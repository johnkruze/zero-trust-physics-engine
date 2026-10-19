#!/usr/bin/env python3
"""Rebuild the published sealing rules with no code of the sealing crate: seal rule v2 (SHA-384).

Ported on 18 October 2026 by the squad seal_v2 from evidence/verification/genesis_seal/outside_check.py
(seal rule v1): every SHA-256 is SHA-384, the frame header is 80 bytes (GKF3, layout 2), the head names
seal_rule, and the saved runs are read from ZERO_TRUST_PHYSICS_RUNS (the runs converted to v2).

Written from two documents: the generation design
(genesis_workspace/catalog/GENERATION_DESIGN.md) and the gateway's map
(protocol_mcp_gateway/AGENTS.md), with the file layouts from the sealing
crate's own map. Standard library only.

    python3 outside_check.py <path to the genesis_seal program> [<the crate's vectors directory>] [--four-key-leaf]

It works out, on its own:
  1. the sealed form and hash of the gateway's worked request;
  2. the sampler vectors: x, k and the sampled value, bit for bit;
  3. the roots of small RFC 6962 trees;
  4. the check of saved engine runs, with no engine;
  5. whole datasets made from saved runs: rows file, frame file, manifest
     head, root and dataset seal.
Then it runs the program on the same inputs and compares what it prints.
With the vectors directory it also compares its vectors with the crate's
published files. The exit code is 0 only when every line passes.

A dataset's leaf is built as genesis_seal 0.4.0 (engine 0.11.0) seals every
dataset, whatever kernel made its runs: the compact JSON of row,
request_sha384, outcome, run_seal and reject_detail_sha384, the last the
SHA-384 of the UTF-8 bytes of the row's reject_detail, or null when it has
none. With --four-key-leaf it is built as genesis_seal before 0.4.0 sealed it,
without reject_detail_sha384. Either way the counts carry "ended" between
"completed" and "refused", as they have since engine 0.7.0.
Updated 2026-10-14 for those two; written 2026-10-06 with neither.

    python3 outside_check.py --print-sampler-vectors
    python3 outside_check.py --print-tree-vectors

print the vector files in the form the crate publishes them.
"""

import hashlib
import json
import os
import struct
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
RUNS = os.path.join(os.environ.get("ZERO_TRUST_PHYSICS_RUNS", os.path.join(HERE, "..", "..", "..", ".runs_v2")), "satellite_lageos1_validation")
SAVED_RUNS = [
    "sealed_lageos1_260912_d6_1356_steps.json",
    "sealed_lageos1_260919_d6_1356_steps.json",
    "sealed_lageos2_260829_d6_1356_steps.json",
    "sealed_lageos2_260905_d6_1356_steps.json",
    "sealed_lageos2_260912_d6_1356_steps.json",
    "sealed_lageos2_260919_d6_1356_steps.json",
    "rehearsal_lageos1_260829_d6_1356_steps.json",
    "rehearsal_lageos1_260905_d6_1356_steps.json",
]
TINY = os.path.join(tempfile.gettempdir(), "outside_check_v2_tiny_dataset")

MASK = (1 << 64) - 1
GOLDEN = 0x9E3779B97F4A7C15


def sha384_hex(data):
    return hashlib.sha384(data).hexdigest()


# ---- The sealed form of a request (gateway map, "The sealed form of a request") ----

def json_string(text):
    return json.dumps(text, ensure_ascii=False)


def sealed_form(value):
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, (int, float)):
        return "x" + struct.pack(">d", float(value)).hex()
    if isinstance(value, str):
        return json_string(value)
    if isinstance(value, list):
        return "[" + ",".join(sealed_form(item) for item in value) + "]"
    keys = sorted(value, key=lambda key: key.encode("utf-8"))
    return "{" + ",".join(json_string(key) + ":" + sealed_form(value[key]) for key in keys) + "}"


def request_sha384(request):
    return sha384_hex(sealed_form(request).encode("utf-8"))


# ---- The sampler (design, "The sampler, exactly") ----

def mix(z):
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
    return z ^ (z >> 31)


def draw_bits(seed, row, j):
    row_key = mix((seed + GOLDEN * (row + 1)) & MASK)
    return mix((row_key + GOLDEN * (j + 1)) & MASK)


def uniform(k, low, high):
    # Three floating-point operations after the exact scaling of k:
    # the subtraction, the multiplication, the addition.
    unit = float(k) * 2.0 ** -53
    width = high - low
    offset = unit * width
    return low + offset


def choice_index(k, n):
    return (k * n) >> 53


def bits(value):
    return struct.pack(">d", value).hex()


def sampler_vector_lines():
    seeds = [0x0000000000000000, 0x0123456789ABCDEF, 0xFFFFFFFFFFFFFFFF, 0x9E3779B97F4A7C15]
    intervals = [(0.0, 1.0), (233.15, 398.15), (-1.0, 1.0), (-3.0e-7, 2.5e9), (1.0e-9, 1.1e-9)]
    lines = [
        "# The sampler splitmix64_counter_v1, as GENERATION_DESIGN.md states it.",
        "# uniform <seed> <row> <draw> <x> <k> <low> <high> <value>",
        "#   seed, x and k are 64-bit whole numbers as 16 hex digits.",
        "#   low, high and value are the bits of 64-bit floats as 16 hex digits.",
        "# choice <seed> <row> <draw> <k> <n> <index>",
        "#   n is the count of choices. index is (k * n) >> 53.",
    ]
    for seed in seeds:
        for row, j in [(0, 0), (99999, 3)]:
            x = draw_bits(seed, row, j)
            k = x >> 11
            for low, high in intervals:
                value = uniform(k, low, high)
                lines.append(
                    "uniform %016x %d %d %016x %016x %s %s %s"
                    % (seed, row, j, x, k, bits(low), bits(high), bits(value))
                )
    for seed in seeds:
        for row, j in [(0, 0), (1, 1), (99999, 3)]:
            k = draw_bits(seed, row, j) >> 11
            for n in [3, 1000]:
                lines.append("choice %016x %d %d %016x %d %d" % (seed, row, j, k, n, choice_index(k, n)))
    return lines


# ---- The row tree (design section 3; RFC 6962 section 2.1) ----

def leaf_hash(leaf):
    return hashlib.sha384(b"\x00" + leaf).digest()


def node_hash(left, right):
    return hashlib.sha384(b"\x01" + left + right).digest()


def split(n):
    k = 1
    while k * 2 < n:
        k *= 2
    return k


def tree_root(hashes):
    if len(hashes) == 1:
        return hashes[0]
    k = split(len(hashes))
    return node_hash(tree_root(hashes[:k]), tree_root(hashes[k:]))


def audit_path(hashes, m):
    if len(hashes) == 1:
        return []
    k = split(len(hashes))
    if m < k:
        return audit_path(hashes[:k], m) + [tree_root(hashes[k:])]
    return audit_path(hashes[k:], m - k) + [tree_root(hashes[:k])]


def root_from_path(m, n, leaf, path):
    if n == 1:
        assert not path
        return leaf
    k = split(n)
    if m < k:
        return node_hash(root_from_path(m, k, leaf, path[:-1]), path[-1])
    return node_hash(path[-1], root_from_path(m - k, n - k, leaf, path[:-1]))


def tree_vector_lines():
    lines = [
        "# RFC 6962 trees over the leaves 'leaf 0', 'leaf 1', ... as ASCII text.",
        "# root <count of leaves> <root>",
        "# path_7_4 <one sibling hash of leaf 4 in the tree of 7, lowest first>",
    ]
    for count in [1, 2, 3, 5, 7, 8]:
        hashes = [leaf_hash(b"leaf %d" % index) for index in range(count)]
        lines.append("root %d %s" % (count, tree_root(hashes).hex()))
    hashes = [leaf_hash(b"leaf %d" % index) for index in range(7)]
    for sibling in audit_path(hashes, 4):
        lines.append("path_7_4 %s" % sibling.hex())
    return lines


# ---- One run (gateway map, "run_trajectory" and "verify_tape") ----

def state_digest(final_state, final_hex, max_hex):
    lines = []
    for key, value in final_state.items():
        if key.endswith("_hex"):
            lines.append((key, value if isinstance(value, str) else ",".join(value)))
        elif isinstance(value, bool):
            lines.append((key, "true" if value else "false"))
    lines.sort(key=lambda line: line[0].encode("utf-8"))
    text = "".join("%s:%s\n" % line for line in lines)
    text += "final_residual_conservation_error_hex:%s\n" % final_hex
    text += "max_residual_conservation_error_hex:%s\n" % max_hex
    return sha384_hex(text.encode("utf-8"))


def run_seal(last_link_hex, outcome, steps_landed, reject, digest):
    tail = json.dumps(
        {"outcome": outcome, "steps_landed": steps_landed, "reject": reject, "state_digest": digest},
        separators=(",", ":"),
    )
    return sha384_hex((last_link_hex + tail).encode("utf-8"))


def check_run(tape):
    """Return (holds, broken_link, cause, last link, frame bytes), as verify_tape answers."""
    link = hashlib.sha384(sealed_form(tape["inputs"]).encode("utf-8")).digest()
    frames = []
    for index, recorded in enumerate(tape["frames"]):
        frame = bytes.fromhex(recorded["frame_hex"])
        if len(frame) != 64 or frame[48:] != link[:16]:
            return False, index, "chain", None, None
        link = hashlib.sha384(frame).digest()
        if link.hex() != recorded["link"]:
            return False, index, "chain", None, None
        frames.append(frame)
    if tape["steps_landed"] == 0:
        digest = None
        if tape["final_state"] is not None:
            return False, None, "state", None, None
    else:
        digest = state_digest(
            tape["final_state"],
            tape["final_residual_conservation_error_hex"],
            tape["max_residual_conservation_error_hex"],
        )
    if digest != tape["state_digest"]:
        return False, None, "state", None, None
    seal = run_seal(link.hex(), tape["outcome"], tape["steps_landed"], tape["reject"], digest)
    if seal != tape["run_seal"]:
        return False, len(tape["frames"]), "run_seal", None, None
    return True, None, None, link.hex(), frames


def receipt_matches(wire):
    mark = b',"receipt_sha384":"'
    at = wire.rfind(mark)
    return at >= 0 and wire[at + len(mark):] == (sha384_hex(wire[:at] + b"}") + '"}').encode()


# ---- A dataset (design sections 3 and 4; layouts from the sealing crate's map) ----

def compact(value):
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False)


def sort_keys(value):
    if isinstance(value, dict):
        return {key: sort_keys(value[key]) for key in sorted(value, key=lambda key: key.encode("utf-8"))}
    if isinstance(value, list):
        return [sort_keys(item) for item in value]
    return value


def leaf_text(row, request_hash, outcome, seal, sentence, five_keys):
    leaf = {"row": row, "request_sha384": request_hash, "outcome": outcome, "run_seal": seal}
    if five_keys:
        leaf["reject_detail_sha384"] = None if sentence is None else sha384_hex(sentence.encode("utf-8"))
    return compact(leaf)


def build_dataset(tapes, five_keys):
    """A list-plan dataset whose rows are the given saved runs, in order, with the leaf of five keys or four."""
    first = tapes[0]["inputs"]
    runs = [
        {key: value for key, value in tape["inputs"].items() if key not in ("domain", "card")}
        for tape in tapes
    ]
    request = {
        "domain": first["domain"],
        "card": first["card"],
        "plan": {"kind": "list", "runs": runs},
        "store": "frames",
        "return": "file",
    }
    plan_sha384 = request_sha384(request)
    rows, frame_bytes, leaves = [], b"", []
    counts = {"completed": 0, "ended": 0, "refused": 0, "unadmitted": 0}
    for index, tape in enumerate(tapes):
        holds, _, _, last_link, frames = check_run(tape)
        assert holds, "a saved run does not hold"
        row_request = sort_keys(dict(domain=request["domain"], card=request["card"], **runs[index]))
        leaf = leaf_text(index, request_sha384(row_request), tape["outcome"], tape["run_seal"], tape["reject_detail"], five_keys)
        rows.append(
            {
                "row": index,
                "request": row_request,
                "outcome": tape["outcome"],
                "steps_landed": tape["steps_landed"],
                "reject": tape["reject"],
                "reject_detail": tape["reject_detail"],
                "final_state": tape["final_state"],
                "final_residual_conservation_error_hex": tape["final_residual_conservation_error_hex"],
                "max_residual_conservation_error_hex": tape["max_residual_conservation_error_hex"],
                "state_digest": tape["state_digest"],
                "frames": len(frames),
                "last_link": last_link,
                "run_seal": tape["run_seal"],
                "leaf": json.loads(leaf),
            }
        )
        counts[tape["outcome"]] += 1
        frame_bytes += b"".join(frames)
        leaves.append(leaf_hash(leaf.encode("utf-8")))
    root = tree_root(leaves).hex()
    frame_count = len(frame_bytes) // 64
    counts_text = compact(
        {
            "engine_version": tapes[0]["engine_version"],
            "rows": len(rows),
            "completed": counts["completed"],
            "ended": counts["ended"],
            "refused": counts["refused"],
            "unadmitted": counts["unadmitted"],
            "frames": frame_count,
        }
    )
    seal = sha384_hex((plan_sha384 + root + counts_text).encode("utf-8"))
    rows_file = "".join(compact(row) + "\n" for row in rows).encode("utf-8")
    header = (
        b"GKF3"
        + struct.pack("<I", 2)
        + struct.pack("<Q", len(rows))
        + struct.pack("<Q", frame_count)
        + bytes.fromhex(seal)
        + bytes(8)
    )
    assert len(header) == 80
    frames_file = header + frame_bytes
    document = compact(
        {
            "request": request,
            "engine_version": tapes[0]["engine_version"],
            "rows": len(rows),
            "completed": counts["completed"],
            "ended": counts["ended"],
            "refused": counts["refused"],
            "unadmitted": counts["unadmitted"],
            "frames": frame_count,
            "plan_sha384": plan_sha384,
            "merkle_root": root,
            "dataset_seal": seal,
            "rows_sha384": sha384_hex(rows_file),
            "frames_sha384": sha384_hex(frames_file),
            "seal_rule": "zero-trust-physics/seal/v2",
        }
    )
    head_file = (document[:-1] + ',"receipt_sha384":"' + sha384_hex(document.encode("utf-8")) + '"}').encode("utf-8")
    return {
        "head": head_file,
        "rows": rows_file,
        "frames": frames_file,
        "root": root,
        "seal": seal,
        "leaves": leaves,
        "plan_sha384": plan_sha384,
        "counts_text": counts_text,
    }


# ---- Running the program ----

class Program:
    def __init__(self, path):
        self.path = path

    def run(self, *arguments):
        done = subprocess.run([self.path, *arguments], capture_output=True, text=True)
        return done.returncode, done.stdout, done.stderr

    def answer(self, *arguments):
        code, out, _ = self.run(*arguments)
        fields = {}
        for line in out.splitlines():
            name, _, value = line.partition(": ")
            fields[name] = value
        return code, fields


class Tally:
    def __init__(self):
        self.passed = 0
        self.failed = 0

    def check(self, what, ok):
        print("%s  %s" % ("PASS" if ok else "FAIL", what))
        if ok:
            self.passed += 1
        else:
            self.failed += 1
        return ok


def write(directory, name, data):
    path = os.path.join(directory, name)
    with open(path, "wb") as handle:
        handle.write(data)
    return path


def compare_vector_file(tally, what, path, lines):
    with open(path, "r", encoding="utf-8") as handle:
        published = [line for line in handle.read().splitlines() if line and not line.startswith("#")]
    mine = [line for line in lines if not line.startswith("#")]
    tally.check("%s: %d lines worked out here are the crate's %d published lines" % (what, len(mine), len(published)), mine == published)


def main(arguments):
    if arguments == ["--print-sampler-vectors"]:
        print("\n".join(sampler_vector_lines()))
        return 0
    if arguments == ["--print-tree-vectors"]:
        print("\n".join(tree_vector_lines()))
        return 0
    five_keys = "--four-key-leaf" not in arguments
    arguments = [argument for argument in arguments if argument != "--four-key-leaf"]
    if len(arguments) not in (1, 2):
        print(__doc__)
        return 2
    program = Program(arguments[0])
    vectors = arguments[1] if len(arguments) == 2 else None
    tally = Tally()

    print("1. The sealed form of the gateway's worked request")
    worked = json.loads(
        '{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,'
        '"segments":[{"steps":10,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}'
    )
    text = sealed_form(worked)
    tally.check("the sealed form is 253 bytes", len(text.encode("utf-8")) == 253)
    tally.check(
        "its SHA-384 is 8615887c...baf9",
        sha384_hex(text.encode("utf-8")) == "8615887c32e2e2edb7d4be74b822e7f64ea384173a9332d97be6278dec255c3d57c9249ca16c5a5d5dfa6c2e43d9baf9",
    )

    print("\n2. The sampler")
    sampler_lines = sampler_vector_lines()
    for line in sampler_lines:
        if not line.startswith("#"):
            print("     " + line)
    if vectors:
        compare_vector_file(tally, "sampler", os.path.join(vectors, "sampler.txt"), sampler_lines)
    seed = 0x0123456789ABCDEF
    choices = [0.05, 0.1, 0.2]
    sample = {
        "domain": "energy",
        "card": "thermal",
        "plan": {
            "kind": "sample",
            "base": {"initial": {}, "segments": [{"steps": 10, "load": {"ambient_k": 293.15}}], "record_every": 1},
            "rows": 100000,
            "generator": "splitmix64_counter_v1",
            "seed_hex": "%016x" % seed,
            "draws": [
                {"path": "initial.temperature_k", "uniform": {"low": 233.15, "high": 398.0}},
                {"path": "segments[0].load.heat_power_w", "uniform": {"low": -5.0, "high": 5.0}},
                {"path": "dt_s", "choice": choices},
            ],
        },
        "store": "none",
        "return": "file",
    }
    with tempfile.TemporaryDirectory() as scratch:
        plan_path = write(scratch, "sample_request.json", compact(sample).encode("utf-8"))
        agree = True
        rows_compared = [0, 1, 2, 3, 500, 65535, 65536, 99999]
        for row in rows_compared:
            code, out, _ = program.run("expand", plan_path, str(row))
            expanded = json.loads(out) if code == 0 else {}
            mine = {
                "temperature_k": uniform(draw_bits(seed, row, 0) >> 11, 233.15, 398.0),
                "heat_power_w": uniform(draw_bits(seed, row, 1) >> 11, -5.0, 5.0),
                "dt_s": choices[choice_index(draw_bits(seed, row, 2) >> 11, 3)],
            }
            theirs = {
                "temperature_k": expanded.get("initial", {}).get("temperature_k"),
                "heat_power_w": expanded.get("segments", [{}])[0].get("load", {}).get("heat_power_w"),
                "dt_s": expanded.get("dt_s"),
            }
            for name in mine:
                same = isinstance(theirs[name], float) and bits(theirs[name]) == bits(mine[name])
                agree = agree and same
                if not same:
                    print("     row %d %s: here %s, program %r" % (row, name, bits(mine[name]), theirs[name]))
        tally.check(
            "the program expands rows %s of a sampled plan to the same bits, three draws each" % rows_compared,
            agree,
        )

    print("\n3. Small RFC 6962 trees")
    tree_lines = tree_vector_lines()
    for line in tree_lines:
        if not line.startswith("#"):
            print("     " + line)
    if vectors:
        compare_vector_file(tally, "tree", os.path.join(vectors, "tree.txt"), tree_lines)
    hashes = [leaf_hash(b"leaf %d" % index) for index in range(7)]
    tally.check(
        "every leaf of the tree of 7 reaches the root along its own path",
        all(root_from_path(m, 7, hashes[m], audit_path(hashes, m)) == tree_root(hashes) for m in range(7)),
    )
    copied = [hashes[index] for index in (0, 1, 2)]
    tally.check(
        "three leaves and the same three with the last repeated have different roots",
        tree_root(copied) != tree_root(copied + [copied[-1]]),
    )

    print("\n4. Saved engine runs, checked with no engine")
    tapes = []
    for name in SAVED_RUNS:
        path = os.path.join(RUNS, name)
        with open(path, "rb") as handle:
            wire = handle.read()
        tape = json.loads(wire)
        tapes.append(tape)
        holds, _, cause, _, _ = check_run(tape)
        code, answer = program.answer("verify-run", path)
        tally.check(
            "%s: holds here, receipt matches here, and the program answers holds with exit 0" % name,
            holds and receipt_matches(wire) and code == 0 and answer.get("holds") == "true" and answer.get("receipt") == "matches",
        )

    print("\n5. Datasets made here from saved runs, checked by the program")
    print("     leaf: %s" % ("five keys, with reject_detail_sha384, as genesis_seal 0.4.0 seals a dataset of any kernel"
                             if five_keys else "four keys, no reject_detail_sha384, as genesis_seal before 0.4.0 sealed one"))
    with tempfile.TemporaryDirectory() as scratch:
        for count in [1, 2, 3, 5, 7, 8]:
            made = build_dataset(tapes[:count], five_keys)
            head = write(scratch, "head_%d.json" % count, made["head"])
            rows = write(scratch, "rows_%d.jsonl" % count, made["rows"])
            frames = write(scratch, "frames_%d.bin" % count, made["frames"])
            code, answer = program.answer("verify-dataset", head, rows, frames)
            print("     %d rows: root %s seal %s" % (count, made["root"], made["seal"]))
            tally.check(
                "%d rows: the program holds it, rebuilds the same root and the same dataset seal, and finds the receipt and both files as named" % count,
                code == 0
                and answer.get("holds") == "true"
                and answer.get("merkle_root") == made["root"]
                and answer.get("dataset_seal") == made["seal"]
                and answer.get("chains") == "rebuilt from the frames"
                and answer.get("receipt") == "matches"
                and answer.get("rows_file") == "matches the hash in the head"
                and answer.get("frames_file") == "matches the hash in the head",
            )
            if count == 7:
                code, out, _ = program.run("row-proof", head, rows, "4", frames)
                proof = json.loads(out) if code == 0 else {}
                mine = [sibling.hex() for sibling in audit_path(made["leaves"], 4)]
                tally.check("7 rows: the program's audit path for row 4 is the one worked out here", proof.get("audit_path") == mine)
                proof_path = write(scratch, "proof_4.json", out.encode("utf-8"))
                code, answer = program.answer("check-proof", proof_path)
                tally.check(
                    "7 rows: the program holds that proof and it leads to the dataset seal worked out here",
                    code == 0 and answer.get("holds") == "true" and answer.get("dataset_seal") == made["seal"],
                )
                leaf = leaf_hash(compact(proof["row"]["leaf"]).encode("utf-8")) if proof else b""
                tally.check(
                    "7 rows: the proof's row reaches the root here along the program's path",
                    bool(proof) and root_from_path(4, 7, leaf, [bytes.fromhex(item) for item in proof["audit_path"]]).hex() == made["root"],
                )
                changed = bytearray(made["frames"])
                changed[80 + 64 * (113 * 2 + 10) + 5] ^= 0x01
                bad = write(scratch, "frames_7_changed.bin", bytes(changed))
                code, answer = program.answer("verify-dataset", head, rows, bad)
                # Frame 10 still carries the link before it. Frame 11 no longer
                # carries the link of frame 10, so the chain stops joining there.
                tally.check(
                    "7 rows: one changed byte in frame 10 of row 2 is answered chain, row 2, link 11, exit 1",
                    code == 1 and answer.get("cause") == "chain" and answer.get("row") == "2" and answer.get("broken_link") == "11",
                )

        os.makedirs(TINY, exist_ok=True)
        tiny = build_dataset(tapes[:2], five_keys)
        head = write(TINY, "head.json", tiny["head"])
        rows = write(TINY, "rows.jsonl", tiny["rows"])
        frames = write(TINY, "frames.bin", tiny["frames"])
        code, answer = program.answer("verify-dataset", head, rows, frames)
        print("     tiny_dataset: plan_sha384 %s" % tiny["plan_sha384"])
        print("     tiny_dataset: merkle_root %s" % tiny["root"])
        print("     tiny_dataset: counts      %s" % tiny["counts_text"])
        print("     tiny_dataset: dataset_seal %s" % tiny["seal"])
        tally.check(
            "tiny_dataset (two saved runs, written here): the program holds it and rebuilds the same dataset seal",
            code == 0 and answer.get("holds") == "true" and answer.get("dataset_seal") == tiny["seal"],
        )

    print("\n%d passed, %d failed" % (tally.passed, tally.failed))
    return 0 if tally.failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
