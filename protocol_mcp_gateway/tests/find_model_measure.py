#!/usr/bin/env python3
"""Measure find_model against sentences whose right answer is known.

    python3 find_model_measure.py <mcp_stdio> [--tasks <environments.jsonl>]
                                  [--fresh <fresh_sentences.json>]
                                  [--out <result.json>] [--against <earlier result.json>]

<mcp_stdio> is the verifier built as a program (target/release/mcp_stdio). It is
started here and asked over MCP stdio; nothing is sent anywhere else.

--tasks   The rows of the published environments, one JSON object a line, each
          with `task` (plain words), `family` and `model`: the model the task
          is about, or null for a real thing no model is of. A task about a
          served thing is right when the first match is the row's model; a
          task about a thing not served is right when nothing is matched.
          A task longer than the 400 characters find_model reads is cut at the
          last whole word inside them. Counts are given by family.

--fresh   Sentences written by an agent that was shown neither find_model nor
          its list of terms: {"find_served": [{"model", "description"}],
          "find_unserved": [{"description", "shares_word_with"}]}. They are
          split in two by their order (even places: first half; odd places:
          second half). A served sentence is right when the first match is its
          model, or the second model its writer noted as acceptable.

--out     Save every row and the counts as JSON.
--against A result saved earlier with --out: every sentence whose answer was
          right in one and wrong in the other is printed, either way.

Python standard library only.
"""
import argparse
import json
import select
import subprocess
import sys

MOST_CHARACTERS = 400
# The second acceptable model of a fresh sentence, as its writer noted it.
SECOND_ACCEPTABLE = {"energy/cell_2170_m80": "energy/cell_2170_model3", "energy/reactor_spert3": "energy/reactor_spert3_fuel"}


class Stdio:
    """One verifier started as a program: a JSON-RPC message a line."""

    def __init__(self, binary):
        self.proc = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.id = 0
        self.ask("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                "clientInfo": {"name": "find_model_measure", "version": "1"}})
        self.proc.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        self.proc.stdin.flush()

    def ask(self, method, params):
        self.id += 1
        message = {"jsonrpc": "2.0", "id": self.id, "method": method, "params": params}
        self.proc.stdin.write((json.dumps(message) + "\n").encode())
        self.proc.stdin.flush()
        while True:
            ready, _, _ = select.select([self.proc.stdout], [], [], 120)
            if not ready:
                raise RuntimeError("no answer in 120 s")
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError("the program closed its output")
            found = json.loads(line)
            if found.get("id") == self.id:
                return found

    def find(self, description):
        result = self.ask("tools/call", {"name": "find_model", "arguments": {"description": description}})["result"]
        text = "".join(part.get("text", "") for part in result.get("content", []))
        if result.get("isError"):
            raise RuntimeError(text)
        return json.loads(text)

    def close(self):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def name_of(match):
    return "%s/%s" % (match["domain"], match["card"]) if "card" in match else match["solver"]


def cut(text):
    return text if len(text) <= MOST_CHARACTERS else text[:MOST_CHARACTERS].rsplit(" ", 1)[0]


def asked(door, sentence):
    body = door.find(sentence)
    matches = [name_of(match) for match in body["matches"]]
    return {
        "matches": matches,
        "fit": body["matches"][0]["fit"] if body["matches"] else None,
        "nearest": [[name_of(row), row["not_a_match"]] for row in body.get("nearest_by_words", [])],
    }


def measure_tasks(door, path):
    rows, counts = [], {}
    for line in open(path, encoding="utf-8"):
        if not line.strip():
            continue
        task = json.loads(line)
        sent = cut(task["task"])
        found = asked(door, sent)
        first = found["matches"][0] if found["matches"] else None
        right = (not found["matches"]) if task["model"] is None else first == task["model"]
        rows.append({"measure": "tasks", "id": task["id"], "group": task["family"], "model": task["model"],
                     "sentence": task["task"], "cut": sent != task["task"], "right": right, **found})
    for row in rows:
        tally = counts.setdefault(row["group"], [0, 0])
        tally[0] += row["right"]
        tally[1] += 1
    served = [row for row in rows if row["model"] is not None]
    counts["served, all families"] = [sum(row["right"] for row in served), len(served)]
    counts["served, nothing matched"] = [sum(not row["matches"] for row in served), len(served)]
    return rows, counts


def measure_fresh(door, path):
    data = json.load(open(path, encoding="utf-8"))
    rows, counts = [], {}
    for place, item in enumerate(data["find_served"]):
        found = asked(door, item["description"])
        first = found["matches"][0] if found["matches"] else None
        right = first is not None and first in (item["model"], SECOND_ACCEPTABLE.get(item["model"]))
        rows.append({"measure": "fresh", "id": "served %d" % place, "group": "%s half, served" % ("first" if place % 2 == 0 else "second"),
                     "model": item["model"], "sentence": item["description"], "cut": False, "right": right, **found})
    for place, item in enumerate(data["find_unserved"]):
        found = asked(door, item["description"])
        rows.append({"measure": "fresh", "id": "not served %d" % place, "group": "%s half, not served" % ("first" if place % 2 == 0 else "second"),
                     "model": None, "sentence": item["description"], "cut": False, "right": not found["matches"], **found})
    for row in rows:
        tally = counts.setdefault(row["group"], [0, 0])
        tally[0] += row["right"]
        tally[1] += 1
    return rows, counts


def show(title, rows, counts):
    print(title)
    for group, (right, of) in counts.items():
        print("  %-28s %3d of %3d" % (group, right, of))
    for row in rows:
        if not row["right"]:
            print("  WRONG %s | %s | wanted %s | matched %s" % (row["group"], row["id"], row["model"] or "nothing", row["matches"] or "nothing"))
            print("        %s" % row["sentence"][:240])
            if row["fit"]:
                print("        %s" % row["fit"])


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("binary")
    parser.add_argument("--tasks")
    parser.add_argument("--fresh")
    parser.add_argument("--out")
    parser.add_argument("--against")
    args = parser.parse_args()
    if not (args.tasks or args.fresh):
        parser.error("give --tasks, --fresh or both")
    door = Stdio(args.binary)
    result = {"rows": [], "counts": {}}
    try:
        if args.fresh:
            rows, counts = measure_fresh(door, args.fresh)
            show("fresh sentences", rows, counts)
            result["rows"] += rows
            result["counts"]["fresh"] = counts
        if args.tasks:
            rows, counts = measure_tasks(door, args.tasks)
            show("tasks of the environments", rows, counts)
            result["rows"] += rows
            result["counts"]["tasks"] = counts
    finally:
        door.close()
    if args.out:
        json.dump(result, open(args.out, "w", encoding="utf-8"), indent=1, ensure_ascii=False)
    if args.against:
        earlier = {(row["measure"], row["id"]): row for row in json.load(open(args.against, encoding="utf-8"))["rows"]}
        changed = 0
        print("against %s" % args.against)
        for row in result["rows"]:
            was = earlier.get((row["measure"], row["id"]))
            if was is not None and was["right"] != row["right"]:
                changed += 1
                print("  %s | %s | %s" % ("NOW RIGHT" if row["right"] else "NOW WRONG", row["group"], row["id"]))
                print("        %s" % row["sentence"][:240])
                print("        wanted %s | was %s | is %s" % (row["model"] or "nothing", was["matches"] or "nothing", row["matches"] or "nothing"))
        print("  %d changed" % changed)
    return 0


if __name__ == "__main__":
    sys.exit(main())
