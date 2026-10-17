#!/usr/bin/env python3
"""Forecast first: a model's protocol fixed, fingerprinted and committed
before any of its held-back data is read, and its scoring recorded after.

    python3 kit/forecast.py register  <model> --protocol FILE [--held-back FILE ...] [--root DIR]
    python3 kit/forecast.py open      <model> <held-back FILE> [--root DIR]
    python3 kit/forecast.py score     <model> --output FILE [--program FILE] [--verdict WORD] [--root DIR]
    python3 kit/forecast.py calibrate <model> --companion-to <model> --protocol FILE --output FILE --program FILE --not-blind "<sentence>" [--root DIR]
    python3 kit/forecast.py check     [<model> ...] [--binary MCP_STDIO] [--root DIR]

<model> is the card's or the solver's name as it is served. --root is the
workspace (the folder whose Cargo.toml lists the crates); the default is the
workspace this script sits in. The registry is one JSON file per model,
protocol_mcp_gateway/kit/forecasts/<model>.json, and each step is a commit
of that one file in the gateway crate's Git, so Git's own record orders the
steps:

  register  takes the SHA-256 of the protocol (the pass marks and the
            forecast) and of each held-back file as it is held, writes the
            registration and commits it. A registration is made once.
  open      is the gate to the held-back data: it reads nothing until the
            registration is committed, the protocol on disk still has its
            fingerprint and the file asked for is a registered held-back
            file with its fingerprint; then it records the time the file is
            opened, commits that, and prints the file's path.
  score     records the scoring: its time, the output's SHA-256 and the
            program's; a second scoring is recorded beside the first.
  calibrate records that the model was calibrated to the record: its
            structure or constants were chosen with the record it is
            compared with open, so the comparison is not a blind test, and
            it is served beside the model compared blind, --companion-to.
            It writes the record's "calibration" (the companion, the SHA-256
            of the comparison's own record, the time of the comparison with
            the output's and the program's SHA-256, and one sentence of what
            of the record was seen before the structure or constants were
            chosen), sets "verdict" to "calibrated to the record" and
            commits it. A calibration is recorded once.
  check     says, for each registration, whether forecast first holds: the
            protocol was committed before every opening and before the
            scoring, and is unchanged on disk. A record that carries a
            calibration is held to its calibration instead (`calibration_holds`,
            the five rules below), reading what the built mcp_stdio serves
            (--binary, by default the workspace's target/release/mcp_stdio).
            Exit 1 if one does not hold.

A model may carry a calibration and a blind registration in the same record:
the blind registration is the record's top level, as register writes it, and
keeps every rule of forecast first; register adds one to a record that holds
only a calibration. Its verdict is "calibrated to the record" until that blind
registration is scored; score then takes the scoring's word, --verdict met,
"met in part" or missed, and the word served changes with it. A calibrated
record is held to:

  1. calibration.companion_to names a served model, not itself, whose own
     record has been scored (scored.utc);
  2. calibration.protocol.sha256 is 64 hex characters and is the SHA-256 of
     the file on disk;
  3. calibration.scored gives the time of the comparison, with the output's
     and the program's SHA-256;
  4. the model's served word is its verdict (calibrated to the record until a
     blind registration is scored), and while that is calibrated to the
     record its served description's validation result says it is not a
     blind test;
  5. a blind registration in the same record holds forecast first.

The conformance suite (protocol_mcp_gateway/tests/conformance_support,
`forecast_findings`) holds every compared model to the same rule: a model
whose list row says met, missed or met in part fails if it has no
registration, or if its registration does not show a committed,
fingerprinted protocol older than its scoring. A model whose row says
calibrated to the record, or whose registration carries a calibration, is
held to the five rules above (`calibration_findings`), and to nothing of
forecast first unless its record holds a blind registration.

A registration made by hand from an older record ("registered_by" names the
backfill) gives under "protocol.committed" the commit, in a crate's Git,
that carried the protocol's SHA-256 (or the protocol itself) before the
scoring; the rule then reads that commit's time from that crate's Git.
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
# The sixth word of against_measurement, and the three of a blind comparison.
CALIBRATED = "calibrated to the record"
BLIND_WORDS = ("met", "missed", "met in part")
# What the served description of a calibrated model says of its comparison.
NOT_A_BLIND_TEST = "not a blind test"


def is_sha256(text):
    return isinstance(text, str) and len(text) == 64 and all(c in "0123456789abcdefABCDEF" for c in text)


def blind(record):
    """Whether the record carries a blind registration: its top level."""
    return isinstance(record.get("protocol"), dict)


def blind_scored(record):
    return blind(record) and bool(record.get("scored"))


def verdict(record):
    """The word a record gives its model, where it carries a calibration:
    calibrated to the record until a blind registration in the same record is
    scored, then that scoring's, as score wrote it. None for a record with no
    calibration, whose word is find_model's."""
    if not record.get("calibration"):
        return None
    if not blind_scored(record):
        return CALIBRATED
    return record.get("verdict")


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
    if len(text) == 10:
        day = datetime.datetime.strptime(text, "%Y-%m-%d").replace(tzinfo=datetime.timezone.utc)
        return day, True
    value = datetime.datetime.fromisoformat(text.replace("Z", "+00:00"))
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
            sys.exit(f"{model}: no registration ({self.rel(model)}); register its protocol first")
        with open(path) as f:
            return json.load(f)

    def load(self, model):
        """The registration of `model`, or None where there is none."""
        try:
            with open(self.file(model)) as f:
                return json.load(f)
        except (OSError, ValueError):
            return None

    def write_and_commit(self, model, record, message):
        path = self.file(model)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as f:
            json.dump(record, f, indent=1)
            f.write("\n")
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
        crate's Git, or the registration's own first commit. With the reason
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
        if record.get("calibration"):
            # Beside a calibration recorded first, the protocol was fixed by
            # the first commit whose record carries its SHA-256.
            for commit, when in commits:
                shown = git(self.gateway, "show", f"{commit}:{self.rel(model)}", check=False)
                try:
                    if (json.loads(shown).get("protocol") or {}).get("sha256") == record["protocol"]["sha256"]:
                        return when, None
                except ValueError:
                    continue
            return None, "no commit of the registration carries the protocol's SHA-256"
        return commits[0][1], None

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
    """The reasons forecast first does not hold for `record`; empty when it does."""
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
    return wrong


def register(registry, args):
    # A record that holds only a calibration takes a blind registration as
    # its top level; any other record is registered once.
    existing = registry.load(args.model) if os.path.exists(registry.file(args.model)) else None
    if os.path.exists(registry.file(args.model)) and (existing is None or blind(existing) or not existing.get("calibration")):
        sys.exit(f"{args.model}: already registered ({registry.rel(args.model)}); a protocol is fixed once")
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
    held = ", ".join(entry["path"] for entry in record["held_back"]) or "none named"
    commit = registry.write_and_commit(
        args.model,
        record,
        f"Forecast first: {args.model}'s protocol {record['protocol']['path']} fixed and fingerprinted, SHA-256 {record['protocol']['sha256']}, before any held-back data is read (held back: {held}).",
    )
    print(f"{args.model}: protocol {record['protocol']['sha256']} committed in protocol_mcp_gateway {commit}")


def open_held_back(registry, args):
    record = registry.read(args.model)
    if not blind(record):
        sys.exit(f"{args.model}: no blind registration ({registry.rel(args.model)}); register its protocol first")
    if not registry.committed(args.model) or not registry.clean(args.model):
        sys.exit(f"{args.model}: the registration is not committed as it stands; commit it before any held-back data is read")
    protocol = record["protocol"]
    on_disk = registry.full(protocol["path"])
    if not os.path.exists(on_disk) or sha256_of(on_disk) != protocol["sha256"]:
        sys.exit(f"{args.model}: the protocol {protocol['path']} is not the one fingerprinted, {protocol['sha256']}; a protocol is fixed once")
    asked = registry.named(args.file)
    entry = next((entry for entry in record.get("held_back", []) if entry["path"] == asked), None)
    if entry is None:
        sys.exit(f"{args.model}: {asked} is not a registered held-back file")
    if sha256_of(registry.full(asked)) != entry["sha256"]:
        sys.exit(f"{args.model}: {asked} is not the file fingerprinted, {entry['sha256']}")
    after(registry.committed(args.model)[-1][1])
    record.setdefault("opened", []).append({"path": asked, "utc": now_utc()})
    registry.write_and_commit(args.model, record, f"Forecast first: {args.model} opens its held-back data {asked}, after its protocol {protocol['sha256']} was committed.")
    print(registry.full(asked))


def score(registry, args):
    record = registry.read(args.model)
    if not blind(record):
        sys.exit(f"{args.model}: no blind registration ({registry.rel(args.model)}); a calibration is recorded by calibrate, and a blind scoring needs its protocol registered first")
    if not registry.committed(args.model):
        sys.exit(f"{args.model}: the registration is not committed; nothing is scored before its protocol is")
    if args.verdict is not None and args.verdict not in BLIND_WORDS:
        sys.exit(f"{args.model}: --verdict is the scoring's word, met, met in part or missed, not {args.verdict!r}")
    if record.get("calibration") and args.verdict is None and not record.get("scored"):
        sys.exit(f"{args.model}: its record carries a calibration, so its word changes with this scoring from calibrated to the record to the scoring's: give --verdict met, \"met in part\" or missed")
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
        if args.verdict is not None:
            record["verdict"] = args.verdict
    wrong = holds(registry, args.model, record)
    registry.write_and_commit(args.model, record, f"Forecast first: {args.model} {what}, output SHA-256 {scoring['output_sha256']}.")
    if wrong:
        print(f"{args.model}: forecast first does not hold: " + "; ".join(wrong))
        sys.exit(1)
    print(f"{args.model}: {what}; forecast first holds")


def calibrate(registry, args):
    model, companion = args.model, args.companion_to
    record = registry.load(model) if os.path.exists(registry.file(model)) else {"model": model}
    if record is None:
        sys.exit(f"{model}: {registry.rel(model)} is not JSON")
    if record.get("calibration"):
        sys.exit(f"{model}: already calibrated ({registry.rel(model)}); a calibration is recorded once")
    if blind_scored(record):
        sys.exit(f"{model}: its blind registration is scored, so its word is that scoring's; a model calibrated to the record after it is a companion of its own")
    if companion == model:
        sys.exit(f"{model}: --companion-to names the served model it stands beside, not itself")
    own = registry.load(companion)
    if not ((own or {}).get("scored") or {}).get("utc"):
        sys.exit(f"{model}: the companion {companion} has no scored record ({registry.rel(companion)}, scored.utc); a calibrated model stands beside a model compared blind and scored")
    not_blind = " ".join(args.not_blind.split())
    if not not_blind:
        sys.exit(f"{model}: --not-blind is one sentence: what of the record was seen before the structure or constants were chosen")
    for path in (args.protocol, args.output, args.program):
        if not os.path.isfile(path):
            sys.exit(f"{model}: {path} is not a file")
    if registry.committed(model):
        after(registry.committed(model)[-1][1])
    named = lambda path: {"path": registry.named(path), "sha256": sha256_of(path)}
    calibration = {
        "companion_to": companion,
        "protocol": named(args.protocol),
        "scored": {"utc": now_utc(), "output": named(args.output), "program": named(args.program)},
        "not_blind": not_blind,
    }
    record["verdict"] = CALIBRATED
    record["calibration"] = calibration
    commit = registry.write_and_commit(
        model,
        record,
        f"Calibrated to the record: {model}, beside {companion}, compared with {calibration['protocol']['path']} (SHA-256 {calibration['protocol']['sha256']}) after its structure or constants were chosen with that record open, so not a blind test; output SHA-256 {calibration['scored']['output']['sha256']}, program SHA-256 {calibration['scored']['program']['sha256']}. Not blind: {not_blind}",
    )
    print(f"{model}: calibrated to the record beside {companion}, committed in protocol_mcp_gateway {commit}")


class Served:
    """What a built mcp_stdio serves, read once over its local connection:
    each model's word (list_cards, list_solvers) and the validation result
    of its full description. `why` says why nothing could be read."""

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

    def result(self, model):
        """The validation result of the model's full description."""
        self.read()
        row = (self.rows or {}).get(model)
        if row is None:
            return None
        try:
            if "domain" in row:
                described = self.tool("describe_card", {"domain": row["domain"], "card": row["card"]})
            else:
                described = self.tool("describe_solver", {"solver": row["solver"]})
        except (OSError, ValueError, KeyError, IndexError, TypeError):
            return None
        return (described.get("validation") or {}).get("result") or ""

    def close(self):
        if self.door is not None:
            self.door.stdin.close()
            self.door.wait()
            self.door = None


def calibration_holds(registry, model, record, served):
    """The reasons a record that carries a calibration does not hold; empty
    when it does. `served` is what the built engine serves (`Served`, or any
    object with models(), word() and result(), and `why`)."""
    wrong = []
    calibration = record.get("calibration") or {}
    # 1. The companion: a served model, not itself, its own record scored.
    companion = calibration.get("companion_to")
    if not companion or companion == model:
        wrong.append("calibration.companion_to names no served model beside it")
    else:
        models = served.models()
        if models is not None and companion not in models:
            wrong.append(f"calibration.companion_to, {companion}, is not a served model")
        if not ((registry.load(companion) or {}).get("scored") or {}).get("utc"):
            wrong.append(f"calibration.companion_to, {companion}: its own record, {registry.rel(companion)}, is not scored (no scored.utc)")
    # 2. The fingerprint of the record it is compared with.
    protocol = calibration.get("protocol") or {}
    if not is_sha256(protocol.get("sha256")):
        wrong.append("calibration.protocol.sha256 is not 64 hex characters")
    elif registry.spectrum or os.path.isabs(protocol.get("path", "")):
        on_disk = registry.full(protocol.get("path", ""))
        if not os.path.isfile(on_disk):
            wrong.append(f"calibration.protocol, {protocol.get('path')}, is not on this disk, so it is not the file fingerprinted")
        elif sha256_of(on_disk) != protocol["sha256"]:
            wrong.append(f"calibration.protocol on disk, {protocol.get('path')}, is not the one fingerprinted")
    # 3. The comparison's scoring.
    scored = calibration.get("scored") or {}
    try:
        when = parse_time(scored.get("utc"))[0]
    except (TypeError, ValueError):
        when = None
    if when is None:
        wrong.append("calibration.scored gives no time of the comparison")
    for part in ("output", "program"):
        if not is_sha256((scored.get(part) or {}).get("sha256")):
            wrong.append(f"calibration.scored.{part} gives no SHA-256")
    # 4, and the word: calibrated to the record until a blind registration
    # in the same record is scored, then that scoring's.
    word = verdict(record)
    if blind_scored(record):
        if word not in BLIND_WORDS:
            wrong.append(f"its blind registration is scored, and {registry.rel(model)} gives no verdict of that scoring (met, met in part or missed)")
    elif record.get("verdict") not in (None, CALIBRATED):
        wrong.append(f"{registry.rel(model)} gives the verdict {record.get('verdict')}, and no blind registration of it is scored: until one is, the verdict is calibrated to the record")
    if word is not None:
        shown = served.word(model)
        if shown is None:
            wrong.append(served.why or f"the served word: {model} is not served")
        elif shown != word:
            wrong.append(f"the served word is {shown!r}, not {word!r}")
        elif word == CALIBRATED and NOT_A_BLIND_TEST not in (served.result(model) or ""):
            wrong.append("the served description's validation result does not say it is not a blind test")
    # 5. A blind registration beside it keeps every rule of forecast first.
    if blind(record):
        wrong += holds(registry, model, record)
    return wrong


def check(registry, args):
    folder = os.path.join(registry.gateway, REGISTRY)
    models = args.models or sorted(name[:-5] for name in os.listdir(folder) if name.endswith(".json")) if os.path.isdir(folder) else args.models
    served = Served(args.binary or os.path.join(registry.root, "target", "release", "mcp_stdio"))
    failed = 0
    for model in models or []:
        record = registry.read(model)
        if record.get("calibration"):
            wrong = calibration_holds(registry, model, record, served)
            said = f"holds, {verdict(record)}"
        else:
            wrong = holds(registry, model, record)
            said = "holds"
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
    one.add_argument("--held-back", nargs="*", default=[])
    one = commands.add_parser("open")
    one.add_argument("model")
    one.add_argument("file")
    one = commands.add_parser("score")
    one.add_argument("model")
    one.add_argument("--output", required=True)
    one.add_argument("--program")
    one.add_argument("--verdict", help="the scoring's word, met, \"met in part\" or missed; required for a record that carries a calibration, whose word changes with it")
    one = commands.add_parser("calibrate")
    one.add_argument("model")
    one.add_argument("--companion-to", required=True, help="the served model it stands beside, compared blind and scored")
    one.add_argument("--protocol", required=True, help="the comparison's own record, a file in evidence/")
    one.add_argument("--output", required=True, help="the comparison's scored output")
    one.add_argument("--program", required=True, help="the program that scored it")
    one.add_argument("--not-blind", required=True, help="one sentence: what of the record was seen before the structure or constants were chosen")
    one = commands.add_parser("check")
    one.add_argument("models", nargs="*")
    one.add_argument("--binary", help="the mcp_stdio whose served words a calibrated record is held to; by default the workspace's target/release/mcp_stdio")
    args = parser.parse_args()
    registry = Registry(args.root)
    {"register": register, "open": open_held_back, "score": score, "calibrate": calibrate, "check": check}[args.command](registry, args)


if __name__ == "__main__":
    main()
