#!/usr/bin/env python3
"""Every worked example and every printed proof on two engine binaries, and
which bytes and which seals changed between them.

    python3 kit/before_after.py <binary A> <binary B> [--out DIR] [--spectrum DIR] [--skip-replay]
    python3 kit/before_after.py --compare <folder A> <folder B>

<binary A> and <binary B> are two mcp_stdio binaries (or the addresses of two
hosted connections), for example the served build and the integration's.
This wraps the company's two existing programs and adds the account of the
seals:

  evidence/verification/gateway_0.9.1/before_after.py save   every worked step,
      run, solve, verification by request, stop and description, per binary;
  evidence/verification/printed_proofs/replay.py --save      every request
      whose proof is printed in the company's collateral, per binary.

Both are run on each binary into DIR (default: a new folder under /tmp), then
every pair of answers is compared. For each answer that differs it prints the
seals that moved (run_seal, solution_seal, receipt_sha256, state_digest, every
seal_parts entry, every frame link, field_sha256, chain and dataset seals) and
how many other values moved, with the first few paths. An answer whose seals
all stayed and whose other bytes moved is called out: those are the changes
the proof does not see (words, descriptions, keys added). The whole account
is written to DIR/REPORT.txt as well.

--compare reads two folders of answers saved earlier, by either program, and
gives the same account. Exit code 0 when nothing differs, 1 when anything does.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile

SEAL_KEYS = {
    "run_seal", "solution_seal", "receipt_sha256", "state_digest", "field_sha256", "link",
    "chain_seal", "dataset_seal", "plan_seal", "tape_seal", "series_seal", "seal", "proof",
}


def find_spectrum(start):
    here = os.path.abspath(start)
    while here != os.path.dirname(here):
        if os.path.isdir(os.path.join(here, "evidence", "verification")):
            return here
        here = os.path.dirname(here)
    return None


def answer_value(raw):
    """An answer file as a value: the JSON of its text block where it holds
    one (an MCP result), else the file's own JSON, else its text."""
    try:
        value = json.loads(raw)
    except ValueError:
        return raw
    if isinstance(value, dict) and isinstance(value.get("content"), list) and value["content"]:
        text = value["content"][0].get("text", "")
        try:
            return {"isError": value.get("isError", False), "answer": json.loads(text)}
        except ValueError:
            return {"isError": value.get("isError", False), "answer": text}
    return value


def leaves(value, path=""):
    if isinstance(value, dict):
        for key, item in value.items():
            yield from leaves(item, f"{path}.{key}" if path else key)
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from leaves(item, f"{path}[{index}]")
    else:
        yield path, value


def is_seal(path):
    last = path.split(".")[-1].split("[")[0]
    return last in SEAL_KEYS or ".seal_parts." in "." + path or path.startswith("seal_parts.") or last.endswith("_seal")


def differences(a, b):
    """(seals that moved, other paths that moved), as paths."""
    left, right = dict(leaves(a)), dict(leaves(b))
    moved = sorted(path for path in set(left) | set(right) if left.get(path, "<absent>") != right.get(path, "<absent>"))
    seals = [path for path in moved if is_seal(path)]
    others = [path for path in moved if not is_seal(path)]
    return seals, others


def seal_names(paths):
    """The kinds of seal among paths: frames[3].link counts once, as frames[].link."""
    names = []
    for path in paths:
        kind = re.sub(r"\[\d+\]", "[]", path)
        if kind not in names:
            names.append(kind)
    return names


def compare(a, b, out):
    files_a = {n for n in os.listdir(a) if n.endswith(".json") and not n.startswith("_SUMMARY")}
    files_b = {n for n in os.listdir(b) if n.endswith(".json") and not n.startswith("_SUMMARY")}
    same, seal_moved, words_only = [], [], []
    for name in sorted(files_a & files_b):
        with open(os.path.join(a, name), "rb") as f, open(os.path.join(b, name), "rb") as g:
            raw_a, raw_b = f.read(), g.read()
        if raw_a == raw_b:
            same.append(name)
            continue
        seals, others = differences(answer_value(raw_a), answer_value(raw_b))
        (seal_moved if seals else words_only).append((name, seals, others))
    out(f"{a}\n  against {b}")
    out(f"  {len(same)} identical byte for byte; {len(seal_moved)} with a seal moved; {len(words_only)} with bytes moved and every seal kept; "
        f"{len(files_a - files_b)} only in the first; {len(files_b - files_a)} only in the second.")
    kinds = {}
    for name, seals, others in seal_moved + words_only:
        kind = (", ".join(seal_names(seals)) or "no seal") + " | " + (", ".join(others) if len(others) <= 2 else f"{len(others)} other values")
        kinds.setdefault(kind, []).append(name[:-5])
    if kinds:
        out("  Kinds of change (seals that moved | other values that moved): answers")
        for kind, names in sorted(kinds.items(), key=lambda item: -len(item[1])):
            out(f"    {len(names):>4}  {kind}")
    if seal_moved:
        out("  Seals moved:")
        for name, seals, others in seal_moved:
            out(f"    {name[:-5]}: {', '.join(seal_names(seals))}; {len(others)} other values" + (f" (first: {', '.join(others[:3])})" if others else ""))
    if words_only:
        out("  Bytes moved, every seal kept (the proof does not cover these):")
        for name, _, others in words_only:
            out(f"    {name[:-5]}: {len(others)} values" + (f" (first: {', '.join(others[:4])})" if others else " (spelling or order only)"))
    for name in sorted(files_a - files_b):
        out(f"  only in the first: {name[:-5]}")
    for name in sorted(files_b - files_a):
        out(f"  only in the second: {name[:-5]}")
    return 0 if not (seal_moved or words_only or files_a ^ files_b) else 1


def run(command, log):
    done = subprocess.run(command, capture_output=True, text=True)
    with open(log, "w") as f:
        f.write(done.stdout)
        f.write(done.stderr)
    return done


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("first", help="binary A, or with --compare a folder")
    parser.add_argument("second", help="binary B, or with --compare a folder")
    parser.add_argument("--compare", action="store_true", help="compare two folders of saved answers")
    parser.add_argument("--out", default=None)
    parser.add_argument("--spectrum", default=None, help="the Spectrum folder, where evidence/verification is")
    parser.add_argument("--skip-replay", action="store_true", help="leave out the printed proofs")
    args = parser.parse_args()
    lines = []

    def out(line):
        print(line)
        lines.append(line)

    if args.compare:
        return compare(args.first, args.second, out)
    spectrum = args.spectrum or find_spectrum(os.path.dirname(os.path.abspath(__file__)))
    if spectrum is None:
        sys.exit("no evidence/verification above this folder: give --spectrum")
    save_script = os.path.join(spectrum, "evidence", "verification", "gateway_0.9.1", "before_after.py")
    replay_script = os.path.join(spectrum, "evidence", "verification", "printed_proofs", "replay.py")
    folder = args.out or tempfile.mkdtemp(prefix="before_after_")
    os.makedirs(folder, exist_ok=True)
    code = 0
    for label, binary in (("A", args.first), ("B", args.second)):
        out(f"{label}: {binary}")
        done = run([sys.executable, "-I", save_script, "save", binary, os.path.join(folder, f"{label}_worked")], os.path.join(folder, f"{label}_worked.log"))
        if done.returncode != 0:
            out(f"  the worked examples did not finish on {label}: see {label}_worked.log")
            code = 1
        if not args.skip_replay:
            run([sys.executable, "-I", replay_script, binary, "--save", os.path.join(folder, f"{label}_printed")], os.path.join(folder, f"{label}_printed.log"))
    out("\nThe worked examples:")
    code |= compare(os.path.join(folder, "A_worked"), os.path.join(folder, "B_worked"), out)
    if not args.skip_replay:
        out("\nThe printed proofs:")
        code |= compare(os.path.join(folder, "A_printed"), os.path.join(folder, "B_printed"), out)
    with open(os.path.join(folder, "REPORT.txt"), "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"\nEverything is in {folder}; the account is {os.path.join(folder, 'REPORT.txt')}.")
    return code


if __name__ == "__main__":
    sys.exit(main())
