#!/usr/bin/env python3
"""Forecasts of data that does not exist yet, fixed in Git before the data
arrives, and the scoring once it has.

    python3 kit/forecast.py register  <model> --protocol FILE [--data FILE ...] [--root DIR]
    python3 kit/forecast.py open      <model> <data FILE> [--root DIR]
    python3 kit/forecast.py score     <model> --output FILE [--program FILE] [--root DIR]
    python3 kit/forecast.py check     [<model> ...] [--binary MCP_STDIO] [--root DIR]

A forecast is for data that does not exist yet: the LAGEOS-1 ranges of a
week not yet flown, the MMRTG's power past the last published day. Data
already on disk needs none of this: fit on what helps, compare on the rest,
and give both counts in the model's spec sheet (new_model.py --fitted-on,
--compared-with).

<model> is the card's or the solver's name as it is served. --root is the
workspace (the folder whose Cargo.toml lists the crates); the default is the
workspace this script sits in. The registry is one JSON file per model,
protocol_mcp_gateway/kit/forecasts/<model>.json.

  register  takes the SHA-256 of the forecast (its file, with the numbers it
            predicts and the tolerance) and of each data file named, writes
            the record and commits it: one commit, whose time in Git shows
            the forecast was fixed before the data. A forecast is fixed once.
  open      reads a data file named in the record once the forecast's
            fingerprint is in a commit and the file on disk is the forecast
            fingerprinted; it records when the file was read in the record
            and prints the file's path. It commits nothing: commit the
            record with the work.
  score     records the scoring in the record: its time, the output's
            SHA-256 and the program's; a second scoring is kept beside the
            first. It commits nothing.
  check     holds each model by the rule of the gateway's conformance sweep
            (tests/conformance_support, `forecast_findings`), read from the
            built mcp_stdio (--binary; by default the workspace's
            target/release/mcp_stdio):
              - a served model whose description states a forecast,
                "Forecast committed before the data was opened, as
                <SHA-256>.", holds when its record registers that SHA-256,
                the forecast was committed before every reading of its data
                and before its comparison, and it is unchanged on disk;
              - a served model that states no forecast is described by its
                spec sheet (src/spec_sheet.rs) and held to nothing here,
                whatever its record carries (a calibration, with a companion
                or none, is the record's own account);
              - a record of a model not served, or every record when no
                built engine is found, is held to its forecast's times where
                it carries a forecast.
            Exit 1 if one does not hold.

The record's keys (protocol, held_back, registered, opened, scored,
registered_by) are the ones the sweep reads; the keys earlier tools wrote
(verdict, calibration) are kept as written and read by nothing.

A record made by hand from an older one ("registered_by" names the
backfill) gives under "protocol.committed" the commit, in a crate's Git,
that carried the forecast's SHA-256 (or the forecast itself) before the
scoring; check then reads that commit's time from that crate's Git.
"""

import argparse
import datetime
import hashlib
import json
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REGISTRY = "kit/forecasts"
# How a served description states a forecast of data not yet published
# (FORECAST_SAID in tests/conformance_support).
FORECAST_SAID = "Forecast committed before the data was opened, as "


def is_sha256(text):
    return isinstance(text, str) and len(text) == 64 and all(c in "0123456789abcdefABCDEF" for c in text)


def carries_forecast(record):
    """Whether the record carries a forecast: its top level."""
    return isinstance(record.get("protocol"), dict)


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def now_utc():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def after(moment):
    """Wait until the clock is past the whole second of `moment`: Git and the
    registry keep whole seconds, so a step taken in the second of the step
    before it could not be ordered after it."""
    while moment is not None and datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0) <= moment:
        time.sleep(0.05)


def parse_time(text):
    """An ISO 8601 time as UTC; a date alone is the start of that day, and
    is marked as a date so that two of the same day are not ordered."""
    if text is None:
        return None, False
    if not isinstance(text, str):
        return None, False
    if len(text) == 10:
        try:
            day = datetime.datetime.strptime(text, "%Y-%m-%d").replace(tzinfo=datetime.timezone.utc)
        except ValueError:
            return None, False
        return day, True
    try:
        value = datetime.datetime.fromisoformat(text.replace("Z", "+00:00"))
    except (TypeError, ValueError):
        return None, False
    if value.tzinfo is None:
        value = value.replace(tzinfo=datetime.timezone.utc)
    return value.astimezone(datetime.timezone.utc), False


def spectrum_root(start):
    """The folder above `start` that holds evidence/research, or None."""
    here = os.path.abspath(start)
    while True:
        if os.path.isdir(os.path.join(here, "evidence", "research")):
            return here
        parent = os.path.dirname(here)
        if parent == here:
            return None
        here = parent


def git(crate, *args, check=True):
    done = subprocess.run(["git", "-C", crate, *args], capture_output=True, text=True)
    if check and done.returncode != 0:
        sys.exit(f"git {' '.join(args)} in {crate}: {done.stderr.strip()}")
    return done.stdout.strip()


class Registry:
    def __init__(self, root):
        self.root = os.path.abspath(root)
        self.gateway = os.path.join(self.root, "protocol_mcp_gateway")
        self.spectrum = spectrum_root(self.root)

    def rel(self, model):
        return f"{REGISTRY}/{model}.json"

    def file(self, model):
        return os.path.join(self.gateway, self.rel(model))

    def named(self, path):
        """A path as the registry keeps it: under the Spectrum folder,
        relative to it; elsewhere, absolute."""
        full = os.path.abspath(path)
        if self.spectrum and full.startswith(self.spectrum + os.sep):
            return os.path.relpath(full, self.spectrum)
        return full

    def full(self, named):
        if os.path.isabs(named) or not self.spectrum:
            return named
        return os.path.join(self.spectrum, named)

    def read(self, model):
        path = self.file(model)
        if not os.path.exists(path):
            sys.exit(f"{model}: no record ({self.rel(model)}); register its forecast first")
        with open(path) as f:
            return json.load(f)

    def load(self, model):
        """The registration of `model`, or None where there is none."""
        try:
            with open(self.file(model)) as f:
                return json.load(f)
        except (OSError, ValueError):
            return None

    def write(self, model, record):
        """The record on disk, not committed."""
        path = self.file(model)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as f:
            json.dump(record, f, indent=1)
            f.write("\n")

    def protocol_committed(self, model, sha256):
        """Whether a commit of the record carries the forecast's SHA-256."""
        for commit, _ in self.committed(model):
            shown = git(self.gateway, "show", f"{commit}:{self.rel(model)}", check=False)
            try:
                if (json.loads(shown).get("protocol") or {}).get("sha256") == sha256:
                    return True
            except ValueError:
                continue
        return False

    def write_and_commit(self, model, record, message):
        """The record on disk and in one commit: register's step, the one
        whose time Git keeps."""
        self.write(model, record)
        git(self.gateway, "add", "--", self.rel(model))
        git(self.gateway, "commit", "-q", "-m", message + "\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>", "--", self.rel(model))
        return git(self.gateway, "rev-parse", "HEAD")

    def committed(self, model):
        """The commits of the registration, oldest first: (hash, UTC time)."""
        lines = git(self.gateway, "log", "--format=%H %cI", "--", self.rel(model), check=False).splitlines()
        out = []
        for line in reversed(lines):
            commit, when = line.split(" ", 1)
            out.append((commit, parse_time(when)[0]))
        return out

    def clean(self, model):
        return git(self.gateway, "status", "--porcelain", "--", self.rel(model), check=False) == ""

    def fixed(self, model, record):
        """When the protocol was fixed: the backfilled commit's time in its
        crate's Git, or the first commit of the registration whose record
        carries the protocol's SHA-256, as the sweep reads it. With the reason
        when it cannot be told."""
        committed = record.get("protocol", {}).get("committed")
        if committed:
            crate = os.path.join(self.root, committed["crate"])
            when = git(crate, "show", "-s", "--format=%cI", committed["commit"], check=False)
            if not when:
                return None, f"the commit {committed['commit']} is not in {committed['crate']}'s Git"
            shown = git(crate, "show", "--format=%B", committed["commit"], check=False)
            sha256 = record["protocol"]["sha256"]
            carried = self.manifest_fingerprint(committed, sha256)
            if sha256[:8] not in shown and not (carried and carried[:8] in shown) and not self.adds_file(crate, committed["commit"], sha256):
                return None, f"the commit {committed['commit']} of {committed['crate']} carries neither the protocol's SHA-256 nor the protocol, nor a manifest's fingerprint that holds it"
            return parse_time(when)[0], None
        if "backfill" in (record.get("registered_by") or ""):
            return None, "no commit carries the protocol's SHA-256 or the protocol: the record it was backfilled from names none"
        commits = self.committed(model)
        if not commits:
            return None, "the registration is not committed"
        for commit, when in commits:
            shown = git(self.gateway, "show", f"{commit}:{self.rel(model)}", check=False)
            try:
                if (json.loads(shown).get("protocol") or {}).get("sha256") == record["protocol"]["sha256"]:
                    return when, None
            except ValueError:
                continue
        return None, "no commit of the registration carries the protocol's SHA-256"

    def manifest_fingerprint(self, committed, sha256):
        """A pre-registration that fingerprints several files at once
        ("manifest": {"path", "list", "fingerprint"}): its fingerprint, the
        SHA-256 of its list of file hashes as json.dumps(list, sort_keys=True)
        writes it, when the list holds the protocol's hash and the fingerprint
        it states is that; else None."""
        named = committed.get("manifest")
        if not named:
            return None
        try:
            with open(self.full(named["path"])) as f:
                manifest = json.load(f)
        except (OSError, ValueError, KeyError):
            return None
        listed = manifest.get(named["list"]) or {}
        if sha256 not in listed.values():
            return None
        fingerprint = hashlib.sha256(json.dumps(listed, sort_keys=True).encode()).hexdigest()
        return fingerprint if manifest.get(named["fingerprint"]) == fingerprint else None

    def adds_file(self, crate, commit, sha256):
        for name in git(crate, "diff-tree", "-r", "--no-commit-id", "--name-only", "--root", commit, check=False).splitlines():
            body = subprocess.run(["git", "-C", crate, "show", f"{commit}:{name}"], capture_output=True).stdout
            if hashlib.sha256(body).hexdigest() == sha256:
                return True
        return False


def holds(registry, model, record):
    """The reasons the record's times do not hold: the forecast committed
    before each reading and before the scoring, and unchanged on disk. Empty
    when they hold."""
    wrong = []
    protocol = record.get("protocol") or {}
    if len(protocol.get("sha256") or "") != 64:
        return ["the registration names no SHA-256 of the protocol"]
    fixed, why = registry.fixed(model, record)
    if fixed is None:
        return [why]
    on_disk = registry.full(protocol.get("path", ""))
    if os.path.exists(on_disk) and sha256_of(on_disk) != protocol["sha256"]:
        wrong.append(f"the protocol on disk, {protocol['path']}, is not the one fingerprinted")
    for opened in record.get("opened") or []:
        when, _ = parse_time(opened.get("utc"))
        if when is None or when <= fixed:
            wrong.append(f"{opened.get('path')} was opened at {opened.get('utc')}, not after the protocol was fixed")
    scored = record.get("scored")
    if not scored:
        return wrong
    when, date_only = parse_time(scored.get("utc"))
    if when is None:
        wrong.append("the scoring has no time")
    elif date_only and when.date() == fixed.date():
        wrong.append(f"the protocol was fixed and the model scored on the same day, {scored['utc']}, and the record gives no time to order them")
    elif when <= fixed:
        wrong.append(f"the protocol was fixed at {fixed.strftime('%Y-%m-%dT%H:%M:%SZ')}, not before the scoring at {scored['utc']}")
    # Each data file was opened before the scoring (the sweep's rule too).
    if when is not None and not date_only:
        for opened in record.get("opened") or []:
            read, _ = parse_time(opened.get("utc"))
            if read is not None and read > when:
                wrong.append(f"{opened.get('path')} was opened at {opened.get('utc')}, after the scoring at {scored['utc']}")
    return wrong


def register(registry, args):
    # A record that holds only a calibration takes a forecast as its top
    # level; any other record is registered once.
    existing = registry.load(args.model) if os.path.exists(registry.file(args.model)) else None
    if os.path.exists(registry.file(args.model)) and (existing is None or carries_forecast(existing) or not existing.get("calibration")):
        sys.exit(f"{args.model}: already registered ({registry.rel(args.model)}); a forecast is fixed once")
    if existing is not None:
        after(registry.committed(args.model)[-1][1] if registry.committed(args.model) else None)
    protocol = os.path.abspath(args.protocol)
    record = dict(existing or {"model": args.model})
    record.update({
        "protocol": {"path": registry.named(protocol), "sha256": sha256_of(protocol), "committed": None},
        "held_back": [{"path": registry.named(path), "sha256": sha256_of(path)} for path in args.held_back],
        "registered": {"utc": now_utc()},
        "opened": [],
        "scored": None,
        "registered_by": "forecast.py register",
    })
    data = ", ".join(entry["path"] for entry in record["held_back"]) or "none named yet"
    commit = registry.write_and_commit(
        args.model,
        record,
        f"Forecast of {args.model}: {record['protocol']['path']}, SHA-256 {record['protocol']['sha256']} (data: {data}).",
    )
    print(f"{args.model}: forecast {record['protocol']['sha256']} committed in protocol_mcp_gateway {commit}")


def open_held_back(registry, args):
    record = registry.read(args.model)
    if not carries_forecast(record):
        sys.exit(f"{args.model}: no forecast in {registry.rel(args.model)}; register it first")
    protocol = record["protocol"]
    if not registry.protocol_committed(args.model, protocol["sha256"]):
        sys.exit(f"{args.model}: the forecast's fingerprint is in no commit of {registry.rel(args.model)}; register commits it")
    on_disk = registry.full(protocol["path"])
    if not os.path.exists(on_disk) or sha256_of(on_disk) != protocol["sha256"]:
        sys.exit(f"{args.model}: the forecast {protocol['path']} is not the one fingerprinted, {protocol['sha256']}; a forecast is fixed once")
    asked = registry.named(args.file)
    entry = next((entry for entry in record.get("held_back", []) if entry["path"] == asked), None)
    if entry is None:
        sys.exit(f"{args.model}: {asked} is not a data file named in the forecast")
    if sha256_of(registry.full(asked)) != entry["sha256"]:
        sys.exit(f"{args.model}: {asked} is not the file fingerprinted, {entry['sha256']}")
    after(registry.committed(args.model)[-1][1])
    if not any(opened.get("path") == asked for opened in record.get("opened") or []):
        record.setdefault("opened", []).append({"path": asked, "utc": now_utc()})
        registry.write(args.model, record)
    print(registry.full(asked))


def score(registry, args):
    record = registry.read(args.model)
    if not carries_forecast(record):
        sys.exit(f"{args.model}: no forecast in {registry.rel(args.model)}; register it first")
    if not registry.protocol_committed(args.model, record["protocol"]["sha256"]):
        sys.exit(f"{args.model}: the forecast's fingerprint is in no commit; nothing is scored before it is")
    after(registry.committed(args.model)[-1][1])
    scoring = {"utc": now_utc(), "output": registry.named(args.output), "output_sha256": sha256_of(args.output)}
    if args.program:
        scoring["program"] = registry.named(args.program)
        scoring["program_sha256"] = sha256_of(args.program)
    if record.get("scored"):
        record.setdefault("scored_again", []).append(scoring)
        what = "scored again"
    else:
        record["scored"] = scoring
        what = "scored"
    wrong = holds(registry, args.model, record)
    registry.write(args.model, record)
    if wrong:
        print(f"{args.model}: {what}; its times do not hold: " + "; ".join(wrong))
        sys.exit(1)
    print(f"{args.model}: {what}, output SHA-256 {scoring['output_sha256']}; commit {registry.rel(args.model)} with the work")


class Served:
    """What a built mcp_stdio serves, read once over its local connection:
    each model's row (list_cards, list_solvers) and its full description.
    `why` says why nothing could be read."""

    def __init__(self, binary):
        self.binary = binary
        self.why = None
        self.rows = None
        self.door = None
        self.count = 0

    def call(self, method, params=None):
        self.count += 1
        message = {"jsonrpc": "2.0", "id": self.count, "method": method}
        if params is not None:
            message["params"] = params
        self.door.stdin.write(json.dumps(message) + "\n")
        self.door.stdin.flush()
        while True:
            line = self.door.stdout.readline()
            if not line:
                raise OSError("the connection closed")
            reply = json.loads(line)
            if reply.get("id") == self.count:
                return reply

    def tool(self, name, arguments):
        reply = self.call("tools/call", {"name": name, "arguments": arguments})
        return json.loads(reply["result"]["content"][0]["text"])

    def read(self):
        if self.rows is not None or self.why is not None:
            return
        if not (self.binary and os.path.isfile(self.binary) and os.access(self.binary, os.X_OK)):
            self.why = f"what is served cannot be read: no mcp_stdio at {self.binary} (cargo build --release, or give --binary)"
            return
        try:
            self.door = subprocess.Popen([self.binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
            self.call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "forecast.py", "version": "1"}})
            self.door.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
            self.door.stdin.flush()
            self.rows = {}
            for row in self.tool("list_cards", {})["cards"]:
                self.rows[row["card"]] = row
            for row in self.tool("list_solvers", {})["solvers"]:
                self.rows[row["solver"]] = row
        except (OSError, ValueError, KeyError, IndexError, TypeError) as error:
            self.rows = None
            self.why = f"what is served cannot be read from {self.binary}: {error}"

    def models(self):
        self.read()
        return None if self.rows is None else set(self.rows)

    def word(self, model):
        self.read()
        row = (self.rows or {}).get(model)
        return None if row is None else row.get("against_measurement")

    def description(self, model):
        """The model's full description, or None where it is not served or
        cannot be read."""
        self.read()
        row = (self.rows or {}).get(model)
        if row is None:
            return None
        try:
            if "domain" in row:
                return self.tool("describe_card", {"domain": row["domain"], "card": row["card"]})
            return self.tool("describe_solver", {"solver": row["solver"]})
        except (OSError, ValueError, KeyError, IndexError, TypeError):
            return None

    def said(self, model):
        """The forecasts the model's full description states, each the text
        after "Forecast committed before the data was opened, as ", as the
        sweep's forecasts_said reads it; None where it is not served."""
        described = self.description(model)
        return None if described is None else forecasts_said(described)

    def close(self):
        if self.door is not None:
            self.door.stdin.close()
            self.door.wait()
            self.door = None


def strings(value):
    """Every string of a JSON value, its keys among them."""
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)
    elif isinstance(value, dict):
        for key, item in value.items():
            yield key
            yield from strings(item)


def forecasts_said(description):
    """The forecasts a description states: the hex after each "Forecast
    committed before the data was opened, as ", once each."""
    said = []
    for text in strings(description):
        at = text.find(FORECAST_SAID)
        while at >= 0:
            start = at + len(FORECAST_SAID)
            end = start
            while end < len(text) and text[end] in "0123456789abcdefABCDEF":
                end += 1
            if text[start:end] not in said:
                said.append(text[start:end])
            at = text.find(FORECAST_SAID, start)
    return said


def model_holds(registry, model, served):
    """(what is said when it holds, the reasons it does not) for one model,
    by the sweep's rule; see check."""
    record = registry.load(model)
    relative = registry.rel(model)
    if os.path.exists(registry.file(model)) and record is None:
        return "", [f"{relative} is not JSON"]
    said = served.said(model) if served.models() is not None else None
    if said:
        if record is None:
            return "", [f"the description states a forecast, and {relative} is missing: no committed, fingerprinted forecast"]
        registered = (record.get("protocol") or {}).get("sha256") or ""
        wrong = []
        for sha256 in said:
            if not is_sha256(sha256):
                wrong.append(f"the description states a forecast as \"{sha256}\", not a SHA-256")
            elif sha256 != registered:
                wrong.append(f"the description states the forecast {sha256}, and {relative} registers {registered or 'none'}")
        return f"holds, the forecast it states ({', '.join(said)}) committed before its data", wrong + holds(registry, model, record)
    if said is not None:
        return "holds, states no forecast", []
    if record is None:
        return "", [f"no record ({relative}), and it is not served"]
    if carries_forecast(record):
        return "holds", holds(registry, model, record)
    return "holds, carries no forecast", []


def check(registry, args):
    folder = os.path.join(registry.gateway, REGISTRY)
    recorded = sorted(name[:-5] for name in os.listdir(folder) if name.endswith(".json")) if os.path.isdir(folder) else []
    served = Served(args.binary or os.path.join(registry.root, "target", "release", "mcp_stdio"))
    if served.models() is None:
        if args.binary:
            print(served.why)
            sys.exit(1)
        print(f"{served.why}; each record is held to its forecast's times", file=sys.stderr)
    models = args.models
    if not models:
        stating = sorted(model for model in (served.models() or ()) if model not in recorded and served.said(model))
        models = recorded + stating
    failed = 0
    for model in models:
        said, wrong = model_holds(registry, model, served)
        print(f"{model}: " + (said if not wrong else "does not hold: " + "; ".join(wrong)))
        failed += bool(wrong)
    served.close()
    sys.exit(1 if failed else 0)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", default=os.path.dirname(os.path.dirname(HERE)))
    commands = parser.add_subparsers(dest="command", required=True)
    one = commands.add_parser("register")
    one.add_argument("model")
    one.add_argument("--protocol", required=True)
    one.add_argument("--data", "--held-back", dest="held_back", nargs="*", default=[], help="each data file the forecast is scored on, fingerprinted as it is")
    one = commands.add_parser("open")
    one.add_argument("model")
    one.add_argument("file")
    one = commands.add_parser("score")
    one.add_argument("model")
    one.add_argument("--output", required=True)
    one.add_argument("--program")
    one = commands.add_parser("check")
    one.add_argument("models", nargs="*")
    one.add_argument("--binary", help="the built mcp_stdio whose descriptions say which models state a forecast; by default the workspace's target/release/mcp_stdio")
    args = parser.parse_args()
    registry = Registry(args.root)
    {"register": register, "open": open_held_back, "score": score, "check": check}[args.command](registry, args)


if __name__ == "__main__":
    main()
