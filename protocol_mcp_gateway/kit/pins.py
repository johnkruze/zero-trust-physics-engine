"""Reproduce the gateway's description and tool-list pins from an mcp_stdio binary.

FULL_DESCRIPTIONS (tests/tools_093.rs): SHA-256 over the text of describe_card for
every listed card, then describe_solver for every listed solver, each followed by a
newline, from the local stdio door. TOOL_LIST_CHARS (tests/tools_070.rs): the
length of tools/list's `tools` as compact JSON. LIST_CARDS_CHARS and
LIST_SOLVERS_CHARS (tests/tools_093.rs, since 0.14.4): the length of the text
list_cards and list_solvers answer, whose rows give a compared model's spec
sheet as what it computes, the comparison and the gap.

Usage: python3 kit/pins.py <mcp_stdio>. Kept in the kit since 0.14.4; until
then it was evidence/verification/next_engine_version_2026-10-15/pins.py.
"""
import hashlib, json, subprocess, sys

binary = sys.argv[1]
door = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
counter = 0

def call(method, params=None):
    global counter
    counter += 1
    message = {"jsonrpc": "2.0", "id": counter, "method": method}
    if params is not None:
        message["params"] = params
    door.stdin.write(json.dumps(message) + "\n")
    door.stdin.flush()
    while True:
        reply = json.loads(door.stdout.readline())
        if reply.get("id") == counter:
            return reply

call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "pins", "version": "1"}})
door.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
door.stdin.flush()

def text(tool, arguments):
    reply = call("tools/call", {"name": tool, "arguments": arguments})
    return reply["result"]["content"][0]["text"]

list_cards = text("list_cards", {})
list_solvers = text("list_solvers", {})
cards = json.loads(list_cards)["cards"]
solvers = json.loads(list_solvers)["solvers"]
whole = hashlib.sha256()
for row in cards:
    whole.update(text("describe_card", {"domain": row["domain"], "card": row["card"]}).encode() + b"\n")
for row in solvers:
    whole.update(text("describe_solver", {"solver": row["solver"]}).encode() + b"\n")
tools = call("tools/list")["result"]["tools"]
print(f"cards {len(cards)} solvers {len(solvers)} tools {len(tools)}")
print("FULL_DESCRIPTIONS", whole.hexdigest())
print("TOOL_LIST_CHARS", len(json.dumps(tools, separators=(",", ":"), ensure_ascii=False)))
print("LIST_CARDS_CHARS", len(list_cards))
print("LIST_SOLVERS_CHARS", len(list_solvers))
door.stdin.close()
door.wait()
