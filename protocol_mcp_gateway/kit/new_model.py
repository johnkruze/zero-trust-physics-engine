#!/usr/bin/env python3
"""Stamp out a new card or solver that starts conformant.

    python3 kit/new_model.py card   <name> --domain energy --title "A heated body" [--root DIR] [--dry-run]
    python3 kit/new_model.py solver <name> --title "A heated rod" [--root DIR] [--dry-run]
    python3 kit/new_model.py rows   <name> [--root DIR] [--dry-run]
    ... card|solver <name> --fitted-on N --compared-with M --records runs --within "X" --gap "G" [--supersedes <model>]
    ... card|solver <name> --reference-calculation --compared-with M --within "X" [--gap "G"]

The evidence flags stamp the model's spec sheet from its numbers:

  (none)                    a company specimen: find_model's Marks::NotCompared
                            and no spec sheet.
  --compared-with M         compared with measurement: M records not in the
  --within X                fit, within X (the accuracy, in numbers);
  --fitted-on N             N records its constants were fitted on (0, or
                            left out, when nothing is fitted);
  --records WORD            what a record is: runs, profiles, cultures;
  --gap G                   the largest disagreement, one line with its
                            number; a model compared with measurement gives it.
  --reference-calculation   held to a reference calculation on M records:
                            Marks::Reference, its numbers in the entry's
                            marks_words, and no spec sheet.
  --supersedes <model>      the served model of the same object this one
                            replaces: the citation takes its real object and
                            sources, and the row { id: <model>, served_as:
                            <name> } goes into src/retired.rs, so <model> is
                            refused by name (RetiredModel) once <name> serves
                            its object: one model for each real object.

A model compared with measurement gets Marks::Measured in find_model.rs and
its row of spec_sheet::SHEETS in src/spec_sheet.rs, the six parts: what it
computes; Range; Fitted on N (or Nothing fitted); Compared with M not in the
fit, within X; Gap; Refuses. The stamp writes the three in numbers from the
flags, and the template's own computes, Range and Refuses, which its owner
rewrites as the real object. The spec line (fitted on N; compared with M not
in the fit, within X; the largest gap) is also its record's result and its
citation's how_used. Nothing here registers a forecast: forecast.py register
is for data that does not exist yet.

<name> is the model's snake_case name, as it is served. --root is the
workspace (the folder whose Cargo.toml lists the crates); the default is the
workspace this script sits in. The script writes the template files under
their new names, makes each registration edit in the shared files at a
marked place (beside the entries of crater_ryugu_sci for a card and of
pipe_flow_superpipe for a solver, the newest of each kind), and prints what
is left to do by hand. It overwrites nothing: a file that exists stops it,
unless --keep-existing is given, when that file is kept as it is (a model
written first, say to settle its numerics before its protocol) and the
registration edits are made around it.

`rows`, run on a card once its own physics is written: every flag its kernel
source declares (`pub const FLAG_...`) gets its row in descriptor.rs
`flag_state` (the state boolean of the same name, which the source must
declare), and every quantity its frame packs (the stamped descriptor's
`("pos[i]", "quantity")` rows) its unit in `unit_of`, from the quantity's
own suffix (`_k` K, `_j` J, `_m_s` m/s, ...). What it cannot derive it names,
and it exits 1: a flag with no boolean, a quantity whose suffix names no
unit, a card with no force_torque row. A card with a flag or a quantity that
has no row would make describe_card panic (answered InternalFault since
0.12.0); the gateway test every_card_has_the_rows_its_flags_and_its_frame_need
names the same rows.

What it writes, for a card:
  genesis_kernel/src/<name>.rs                         the card (template physics: a heated body)
  genesis_kernel/tests/<name>_conformance.rs           the card contract, attacked by the shared harness
  protocol_mcp_gateway/src/card_<name>.rs              the gateway's call of the card
  protocol_mcp_gateway/src/card_<name>_descriptor.rs   its description: the skeleton every sentence keeps to
  protocol_mcp_gateway/tests/<name>_router.rs          the served contract, attacked from the description
and for a solver:
  genesis_solvers/src/<name>.rs                        the solver (template physics: a heated rod)
  genesis_solvers/tests/<name>_conformance.rs
  protocol_mcp_gateway/src/solver_<name>.rs            its entry and description skeleton
  protocol_mcp_gateway/tests/<name>_router.rs
"""

import argparse
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TEMPLATES = os.path.join(HERE, "templates")


def camel(name):
    return "".join(part[:1].upper() + part[1:] for part in name.split("_"))


class Stamp:
    def __init__(self, root, dry, keep_existing=False):
        self.root = root
        self.dry = dry
        self.keep_existing = keep_existing
        self.done = []
        self.by_hand = []
        self.pending = {}

    def path(self, rel):
        return os.path.join(self.root, rel)

    def text(self, rel):
        if rel not in self.pending:
            with open(self.path(rel)) as f:
                self.pending[rel] = f.read()
        return self.pending[rel]

    def write_new(self, rel, body):
        full = self.path(rel)
        if os.path.exists(full):
            if self.keep_existing:
                self.done.append(f"kept {rel}, which exists")
                return
            sys.exit(f"{rel} exists; nothing was written (--keep-existing keeps it and stamps the rest)")
        self.pending[rel] = body
        self.done.append(f"new  {rel}")

    def hook(self, rel, anchor, insert, where="after", next_has=None, skip=0, what=""):
        """Insert `insert` (whole lines) after or before the one line that
        holds `anchor` (and whose next line holds `next_has`, when given)."""
        lines = self.text(rel).split("\n")
        found = [
            i for i, line in enumerate(lines)
            if anchor in line and (next_has is None or (i + 1 < len(lines) and next_has in lines[i + 1]))
        ]
        if len(found) != 1:
            self.by_hand.append(f"{rel}: {what or 'insert'} ({len(found)} places hold the anchor {anchor.strip()!r}):\n{insert}")
            return
        at = found[0] + 1 + skip if where == "after" else found[0]
        lines[at:at] = insert.split("\n")
        self.pending[rel] = "\n".join(lines)
        self.done.append(f"edit {rel}: {what or anchor.strip()}")

    def hook_after_entry(self, rel, anchor, insert, ends_with, what=""):
        """Insert `insert` after the whole entry that begins on the one line
        holding `anchor`: after the first line from there on that ends with
        `ends_with`. A registration list in the engine's order (find_model's
        index, the citations) then holds the new model just after its anchor,
        as the list of cards or solvers does."""
        lines = self.text(rel).split("\n")
        found = [i for i, line in enumerate(lines) if anchor in line]
        if len(found) != 1:
            self.by_hand.append(f"{rel}: {what or 'insert'} ({len(found)} places hold the anchor {anchor.strip()!r}):\n{insert}")
            return
        end = next((i for i in range(found[0], len(lines)) if lines[i].rstrip().endswith(ends_with)), None)
        if end is None:
            self.by_hand.append(f"{rel}: {what or 'insert'} (the entry at {anchor.strip()!r} has no end {ends_with!r}):\n{insert}")
            return
        lines[end + 1:end + 1] = insert.split("\n")
        self.pending[rel] = "\n".join(lines)
        self.done.append(f"edit {rel}: {what or anchor.strip()}")

    def replace_once(self, rel, pattern, make, what):
        body = self.text(rel)
        new, count = re.subn(pattern, make, body, count=1)
        if count != 1:
            self.by_hand.append(f"{rel}: {what}")
            return
        self.pending[rel] = new
        self.done.append(f"edit {rel}: {what}")

    def commit(self):
        if self.dry:
            return
        for rel, body in self.pending.items():
            with open(self.path(rel), "w") as f:
                f.write(body)


def fill(template, words):
    with open(os.path.join(TEMPLATES, template)) as f:
        body = f.read()
    for key, value in words.items():
        body = body.replace("{{" + key + "}}", value)
    left = re.findall(r"\{\{\w+\}\}", body)
    if left:
        sys.exit(f"{template}: unfilled {sorted(set(left))}")
    return body


def module_line(stamp, rel, name):
    """`pub mod <name>;` in the crate's module list, in order of name."""
    lines = stamp.text(rel).split("\n")
    mods = [(i, line) for i, line in enumerate(lines) if re.match(r"pub mod \w+;$", line)]
    if not mods:
        stamp.by_hand.append(f"{rel}: add `pub mod {name};`")
        return
    at = next((i for i, line in mods if line[len("pub mod "):-1] > name), mods[-1][0] + 1)
    lines[at:at] = [f"pub mod {name};"]
    stamp.pending[rel] = "\n".join(lines)
    stamp.done.append(f"edit {rel}: pub mod {name};")


def counts(stamp, kind):
    rel = "protocol_mcp_gateway/data/citations.json"
    key = "cards" if kind == "card" else "solvers"
    stamp.replace_once(rel, rf'"{key}": (\d+)', lambda m: f'"{key}": {int(m.group(1)) + 1}', f"counts.{key} + 1")
    stamp.replace_once(rel, r'"models": (\d+)', lambda m: f'"models": {int(m.group(1)) + 1}', "counts.models + 1")


def served(stamp, key):
    """How many cards or solvers the citations count before the stamp."""
    with open(stamp.path("protocol_mcp_gateway/data/citations.json")) as f:
        found = re.search(rf'"{key}": (\d+)', f.read())
    return found.group(1) if found else "?"


# The three kinds of evidence a served model carries: the word find_model
# serves for it (spec_sheet::WORDS) and the Marks variant of a new entry.
EVIDENCE_WORDS = {
    "measurement": "compared with measurement",
    "reference": "held to a reference calculation",
    "specimen": "company specimen",
}
MARKS = {"measurement": "Measured", "reference": "Reference", "specimen": "NotCompared"}
# What a new entry compared with measurement says as marks_words: its spec
# sheet gives the numbers (find_model serves the sheet's accuracy).
MEASURED_WORDS = "its spec sheet gives the comparison and the gap"


class Evidence:
    """What a stamped model was fitted on and compared with, in numbers.

    kind is "measurement", "reference" or "specimen". fitted_on and
    compared_with are counts of records (runs, profiles, cultures, points:
    `records` names them); within is the accuracy reached on the records not
    in the fit, in numbers; gap is the largest disagreement, one line with
    its number; supersedes is the served model this one replaces."""

    def __init__(self, kind="specimen", fitted_on=None, compared_with=None, records="records", within=None, gap=None, supersedes=None):
        self.kind = kind
        self.fitted_on = fitted_on
        self.compared_with = compared_with
        self.records = records or "records"
        self.within = within
        self.gap = gap
        self.supersedes = supersedes

    @classmethod
    def of(cls, args):
        if args is None:
            return cls()
        kind = "reference" if getattr(args, "reference_calculation", False) else ("measurement" if getattr(args, "compared_with", None) is not None else "specimen")
        return cls(kind, getattr(args, "fitted_on", None), getattr(args, "compared_with", None), getattr(args, "records", None),
                   getattr(args, "within", None), getattr(args, "gap", None), getattr(args, "supersedes", None))

    def spec_line(self):
        """The spec sheet in one line: fitted on N; compared with M not in
        the fit, within X; the gap. Empty for a company specimen."""
        if self.kind == "specimen":
            return ""
        parts = []
        if self.kind == "measurement":
            parts.append("nothing fitted" if not self.fitted_on else f"fitted on {self.fitted_on} {self.records}")
            parts.append(f"compared with {self.compared_with} {self.records} not in the fit, within {self.within}")
        else:
            parts.append(f"held to a reference calculation on {self.compared_with} {self.records}, within {self.within}")
        if self.gap:
            parts.append(f"the largest gap: {self.gap}")
        if self.supersedes:
            parts.append(f"supersedes {self.supersedes}")
        return "; ".join(parts)

    def result(self):
        """The record's result sentence, the spec line as a sentence."""
        line = self.spec_line()
        return line[:1].upper() + line[1:] + "."

    def fitted_sentence(self):
        """The spec sheet's third part: "Fitted on N ..." or "Nothing fitted"."""
        return f"Fitted on {self.fitted_on} {self.records}." if self.fitted_on else "Nothing fitted."

    def compared_sentence(self):
        """The fourth part: "Compared with M ... not in the fit: within X"."""
        return f"Compared with {self.compared_with} {self.records} not in the fit: within {self.within.rstrip('.')}."

    def gap_sentence(self):
        """The fifth part: "Gap: ...", one line with its number."""
        return f"Gap: {self.gap.rstrip('.')}."

    def reference_words(self):
        """A reference calculation's numbers, find_model's marks_words, which
        it serves as "Held to a reference calculation: <these>."."""
        words = f"{self.compared_with} {self.records}, within {self.within.rstrip('.')}"
        return words + (f"; the largest gap: {self.gap.rstrip('.')}" if self.gap else "")


def marks_of(stamp, evidence):
    """The Marks variant of a new entry with this evidence, where
    find_model.rs declares it; None where it does not (then the stamp says
    so)."""
    try:
        text = stamp.text("protocol_mcp_gateway/src/find_model.rs")
    except OSError:
        return None
    variant = MARKS[evidence.kind]
    return f"Marks::{variant}" if re.search(rf"^\s*{variant},", text, re.M) else None


def citation(key, kind, title, evidence=None, predecessor=None):
    """The model's citation. A company specimen says so. A model compared
    with measurements, or held to a reference calculation, names the real
    thing and its numbers; its owner writes in the sources (people, data
    set, licence). A model that supersedes another takes that model's real
    thing and sources, the same object."""
    evidence = evidence or Evidence()
    if evidence.kind == "specimen" and predecessor is None:
        entry = {
            "kind": kind,
            "real_thing": f"{title}, a company specimen.",
            "sources": [],
            "how_used": "Company specimen: its constants are its own declared values. Cite the paper.",
            "not_held_to_a_measurement": True,
            "cite_also": "paper",
        }
    else:
        how = evidence.result() if evidence.kind != "specimen" else ""
        if predecessor is not None:
            name, own = predecessor
            how = (how + " " if how else "") + f"It supersedes {name}, a model of the same object, whose sources these are."
        else:
            how += " Its sources, with their people and licence, are written here by its owner."
        entry = {
            "kind": kind,
            "real_thing": predecessor[1]["real_thing"] if predecessor else (f"{title}, compared with measurements of it." if evidence.kind == "measurement" else f"{title}, held to a reference calculation."),
            "sources": predecessor[1]["sources"] if predecessor else [],
            "how_used": how,
            "not_held_to_a_measurement": evidence.kind != "measurement",
            "cite_also": "paper",
        }
    lines = json.dumps({key: entry}, indent=2, ensure_ascii=False).split("\n")[1:-1]
    return "\n".join("  " + line for line in lines) + ","


def rust_text(text):
    return text.replace("\\", "\\\\").replace('"', '\\"')


def served_citation(stamp, model):
    """The citation key and entry of the served model `model`."""
    table = json.loads(stamp.text("protocol_mcp_gateway/data/citations.json"))
    found = [key for key in table["models"] if key.split("/")[-1] == model]
    if len(found) != 1:
        sys.exit(f"--supersedes {model}: no served model of that name in protocol_mcp_gateway/data/citations.json")
    return found[0].split("/")[-1], table["models"][found[0]]


def entry(key, title, marks="Marks::NotCompared", marks_words="OWN_SPECIMEN"):
    words = title.lower()
    return (
        f'    Entry {{ key: "{key}", marks: {marks}, marks_words: {marks_words},\n'
        f'        strong: &["{words}"],\n'
        f'        plain: &[],\n'
        f'        weak: &[],\n'
        f'        other: &[],\n'
        f'        own: &[] }},'
    )


def evidence_entry(stamp, key, title, evidence):
    """find_model's entry for the stamped model, with the Marks of its
    evidence: Measured (its spec sheet gives the numbers) or Reference (its
    numbers in marks_words)."""
    if evidence.kind == "specimen":
        return entry(key, title)
    marks = marks_of(stamp, evidence)
    if marks is None:
        stamp.by_hand.append(
            f"find_model.rs declares no Marks::{MARKS[evidence.kind]}, the variant served as {EVIDENCE_WORDS[evidence.kind]!r}: the entry of {key} is stamped "
            "Marks::NotCompared, and find_model's test of a company specimen names it until its marks are set to that word's variant."
        )
        marks = "Marks::NotCompared"
    words = MEASURED_WORDS if evidence.kind == "measurement" else evidence.reference_words()
    return entry(key, title, marks, '"' + rust_text(words) + '"')


# The template's own parts of a spec sheet, which its owner rewrites as the
# real object: what it computes, its range and where it refuses, each as the
# template's physics gives it (the refusals are the template's, so the
# stamped tree's tests hold the sheet to them).
TEMPLATE_SHEET = {
    "card": (
        "{title}: the temperature of a body of 500 J/K warmed by a heater and cooled through 0.5 W/K to a bath, its heat ledger held at every step.",
        "Range: the template's, the body from above 0 K to 933.47 K, its melting point; write where it was compared and how far it computes past that.",
        "Refuses a body or a bath at or below absolute zero (BelowAbsoluteZero) or at or above the body's melting point (AboveMeltingPoint).",
    ),
    "solver": (
        "{title}: steady heat conduction along a 0.5 m aluminium rod with heat generated inside it, the peak temperature and the heat leaving each end.",
        "Range: the template's, ends held from above 0 K to 933.47 K, the rod's melting point; write where it was compared and how far it computes past that.",
        "Refuses an end at or below absolute zero or above the melting point (EndOutsideSolid) and a field that does not settle (NotSettled).",
    ),
}
# Where a new row of spec_sheet::SHEETS goes: after the cards described in
# their own files, or after the last solver.
SHEET_ANCHORS = {"card": "    // <<< sheets: card files >>>", "solver": "    // <<< sheets: solvers two >>>"}


def sheet_row(stamp, kind, name, title, evidence):
    """The model's row of spec_sheet::SHEETS, from its numbers: Fitted on,
    Compared with ... within and Gap from the flags, the template's computes,
    Range and Refuses. Only a model compared with measurement has one."""
    if evidence.kind != "measurement":
        return
    computes, range_, refuses = TEMPLATE_SHEET[kind]
    parts = [
        ("model", name),
        ("computes", computes.format(title=title)),
        ("range", range_),
        ("fitted_on", evidence.fitted_sentence()),
        ("compared", evidence.compared_sentence()),
        ("gap", evidence.gap_sentence()),
        ("refuses", refuses),
    ]
    row = "    SpecSheet {\n" + "".join(f'        {field}: "{rust_text(text)}",\n' for field, text in parts) + "    },"
    stamp.hook("protocol_mcp_gateway/src/spec_sheet.rs", SHEET_ANCHORS[kind], row, where="before",
               what="its spec sheet: a row of SHEETS, in numbers")


def retired_row(stamp, old, new):
    """The row { id: old, served_as: new } in retired::TABLE: once new is
    served, a request naming old is refused by name (RetiredModel), naming
    new. One model is served for each real object (engine 0.15.0): the
    table lists the cards first, then the solvers, so a card's row goes
    after the last card's and a solver's at the end."""
    rel = "protocol_mcp_gateway/src/retired.rs"
    row = f'    Retired {{ id: "{old}", served_as: "{new}" }},'
    try:
        text = stamp.text(rel)
    except OSError:
        stamp.by_hand.append(f"{rel}: add {row.strip()} to TABLE")
        return
    start = text.find("pub const TABLE: &[Retired] = &[")
    end = text.find("\n];", start) if start >= 0 else -1
    if end < 0:
        stamp.by_hand.append(f"{rel}: no TABLE to add a row to; add {row.strip()}")
        return
    table = text[start:end]
    already = re.search(rf'Retired \{{ id: "{old}", served_as: "(\w+)" \}}', table)
    if already:
        stamp.by_hand.append(f"{rel}: {old} is retired already, served as {already.group(1)}; an ID is retired once (every_retired_id_names_a_served_model_and_is_not_served)")
        return
    cited = json.loads(stamp.text("protocol_mcp_gateway/data/citations.json"))["models"]
    cards = {key.split("/")[-1] for key in cited if "/" in key}
    old_is_card = old in cards
    rows = list(re.finditer(r'^    Retired \{ id: "(\w+)", served_as: "(\w+)" \},$', text[:end], re.M))
    card_rows = [found for found in rows if found.group(2) in cards]
    if old_is_card and card_rows:
        at = card_rows[-1].end() + 1
    else:
        at = end + 1
    stamp.pending[rel] = text[:at] + row + "\n" + text[at:]
    stamp.done.append(f"edit {rel}: {old} retired, served as {new}")


def evidence_by_hand(stamp, name, evidence):
    """What a stamp of a compared or superseding model leaves to its owner."""
    if evidence.kind != "specimen":
        stamp.done.append(f"spec line of {name}: {evidence.spec_line()}")
        stamp.by_hand.append(
            f"The stamped description is the template's (a heated body or rod): rewrite its specimen, data and limits as the real object of {name}. "
            "Its record's result is stamped as the spec line; keep it in numbers as the comparison is run again."
        )
        if not evidence.supersedes:
            stamp.by_hand.append(f"citations.json: write the sources of {name} (people, data set, licence).")
    if evidence.kind == "measurement":
        stamp.by_hand.append(
            f"src/spec_sheet.rs, the SHEETS row of {name}: Fitted on, Compared with and Gap are stamped from the flags; rewrite computes, Range (where it was "
            "compared, in numbers, and how far it computes past that) and Refuses (by name) as the real object, and name what was fitted. "
            "find_model's every_model_compared_with_measurement_has_its_spec_sheet and the sheet's every_spec_sheet_has_the_shape hold it."
        )
    if evidence.kind != "specimen":
        stamp.by_hand.append(
            f"find_model's every_served_model_carries_its_word_on_its_list_row_and_in_a_match counts the models of each word: "
            f"{EVIDENCE_WORDS[evidence.kind]!r} goes up by one."
        )
        stamp.by_hand.append(
            "A forecast of data that does not exist yet is fixed before the data arrives: python3 protocol_mcp_gateway/kit/forecast.py register "
            f"{name} --protocol <the forecast> (one commit), and its description says \"Forecast committed before the data was opened, as <SHA-256>.\". "
            "Data already on disk needs no registration: fit on what helps, compare on the rest, and give both counts in the spec sheet."
        )
    if evidence.supersedes:
        stamp.by_hand.append(
            f"{evidence.supersedes} is retired by its retired.rs row once {name} is served: take {evidence.supersedes} out of the served lists "
            f"(CARDS or SOLVERS, its find_model entry and its citation; its file may stay in its crate, not compiled), or serve {name} under the "
            f"object's original ID, {evidence.supersedes}, and retire {name}'s in its place (Retired {{ id: \"{name}\", served_as: \"{evidence.supersedes}\" }}). "
            f"retired::tests::every_retired_id_names_a_served_model_and_is_not_served holds it. "
            f"Say in {name}'s spec sheet what it computes that {evidence.supersedes} did not."
        )


def evidence_result(stamp, rel, old, evidence):
    """The stamped record's result, the spec line, in place of the template's."""
    stamp.replace_once(rel, re.escape(old), lambda _: f'"{rust_text(evidence.result())}".to_string(),',
                       what="the record's result: the spec line, in numbers")


def stamp_card(stamp, name, domain, title, evidence=None):
    evidence = evidence or Evidence()
    words = {"name": name, "Name": camel(name), "NAME": name.upper(), "domain": domain, "Title": title}
    Name = words["Name"]
    stamp.write_new(f"genesis_kernel/src/{name}.rs", fill("card/kernel.rs", words))
    stamp.write_new(f"genesis_kernel/tests/{name}_conformance.rs", fill("card/kernel_conformance.rs", words))
    stamp.write_new(f"protocol_mcp_gateway/src/card_{name}.rs", fill("card/gateway.rs", words))
    stamp.write_new(f"protocol_mcp_gateway/src/card_{name}_descriptor.rs", fill("card/descriptor.rs", words))
    if evidence.kind != "specimen" and f"protocol_mcp_gateway/src/card_{name}_descriptor.rs" in stamp.pending:
        evidence_result(stamp, f"protocol_mcp_gateway/src/card_{name}_descriptor.rs",
                        '"Held: the card\'s tests run it from the floor to the ceiling, and every step holds its conservation ledger to its stated limit.".to_string(),', evidence)
    stamp.write_new(f"protocol_mcp_gateway/tests/{name}_router.rs", fill("card/router_test.rs", words))
    module_line(stamp, "genesis_kernel/src/lib.rs", name)

    lib = "protocol_mcp_gateway/src/lib.rs"
    stamp.hook(lib, "mod live;", f"mod card_{name};", what="the module")
    stamp.hook(lib, '("crater_ryugu_sci", "radius_m", true) => "RadiusAboveFinal",',
               f'        ("{name}", "temperature_k" | "ambient_k", false) => "BelowAbsoluteZero",\n'
               f'        ("{name}", "temperature_k" | "ambient_k", true) => "AboveMeltingPoint",',
               what="published_reject: the name of each field's bound")
    stamp.hook(lib, '("aerospace", "crater_ryugu_sci") => {', f'        ("{domain}", "{name}") => Some(card_{name}::rest(finish)),',
               where="before", next_has="let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();", what="with_rest_entries")
    stamp.hook(lib, '("aerospace", "crater_ryugu_sci") => crater_ryugu_sci::FLAG_STOPPED,', f'        ("{domain}", "{name}") => card_{name}::FLAGS,',
               what="advisory_flags")
    stamp.hook(lib, '("aerospace", "crater_ryugu_sci") => {',
               f'        ("{domain}", "{name}") => {{\n'
               f'            let call = card_{name}::{Name}Call::from_parameters(&payload.parameters)\n'
               f'                .map_err(DoorError::Unprocessable)?;\n'
               f'            on_pool(state, move || call.run(proof)).await?\n'
               f'        }}',
               where="before", next_has="let call = CraterCall::from_parameters", what="compute_step")

    live = "protocol_mcp_gateway/src/live.rs"
    stamp.hook(live, "    Crater(CraterCall),", f"    {Name}(crate::card_{name}::{Name}Call),", what="Live")
    stamp.hook(live, "    Crater(crater_ryugu_sci::CraterState),", f"    {Name}(genesis_kernel::{name}::{Name}State),", what="PhaseSnap")
    stamp.hook(live, '("aerospace", "crater_ryugu_sci") => {',
               f'            ("{domain}", "{name}") => Ok(Live::{Name}(crate::card_{name}::{Name}Call::from_parameters(params)?)),',
               where="before", what="Live::open")
    stamp.hook(live, "Live::Crater(call) => call.replace_load(params),", f"            Live::{Name}(call) => call.replace_load(params),", what="replace_load")
    stamp.hook(live, "Live::Crater(call) => PhaseSnap::Crater(call.state),", f"            Live::{Name}(call) => PhaseSnap::{Name}(call.state),", what="snapshot")
    stamp.hook(live, "(Live::Crater(call), PhaseSnap::Crater(state)) => call.state = state,",
               f"            (Live::{Name}(call), PhaseSnap::{Name}(state)) => call.state = state,", what="restore")
    stamp.hook(live, "Live::Crater(call) => call.step_into(frame),", f"            Live::{Name}(call) => call.step_into(frame),", what="step_into")
    stamp.hook(live, "            Live::Crater(call) => {",
               f"            Live::{Name}(call) => {{\n"
               f"                crate::card_{name}::published_is_finite(&call.state).map_err(StepFailure::flag)?;\n"
               f'                finish_audit(frame, "{domain}", "{name}", call.state.residual_conservation_error)\n'
               f"            }}",
               where="before", next_has="crater_published_is_finite", what="audit")
    stamp.hook(live, "Live::Crater(call) => crater_state(&call.state).map_err(StepFailure::flag),",
               f"            Live::{Name}(call) => crate::card_{name}::state(&call.state).map_err(StepFailure::flag),", what="publish")

    desc = "protocol_mcp_gateway/src/descriptor.rs"
    stamp.hook(desc, "fn build() -> Vec<Card> {", f'#[path = "card_{name}_descriptor.rs"]\nmod card_{name}_descriptor;\n', where="before", what="the description's module")
    stamp.hook(desc, "        crater_ryugu_sci_card(),", f"        card_{name}_descriptor::card(),", what="build")
    stamp.hook(desc, '"crater_ryugu_sci" => "radius_to_go_m",', f'        "{name}" => "melt_margin_k",', what="force_torque_state")
    stamp.hook(desc, '        "crater_ryugu_sci" => [', f'        "{name}" => card_{name}_descriptor::validation(specimen),', where="before", what="validation")
    stamp.hook(desc, '("aerospace", "crater_ryugu_sci") => crater_examples(),', f'        ("{domain}", "{name}") => card_{name}_descriptor::examples(),', what="worked_examples")
    stamp.hook(desc, "    Some(match (card, name) {",
               f'        ("{name}", which) if card_{name}_descriptor::meaning(which).is_some() => return card_{name}_descriptor::meaning(which),',
               what="reject_meaning (the card's own names; the gateway's own, as DtBelowFloor, fall through to their shared arms)")
    stamp.hook(desc, '"crater_ryugu_sci",', next_has='vec!["radius_m", "dt_s"],', skip=2, insert=
               f'            (\n                "{domain}",\n                "{name}",\n                vec!["temperature_k", "heat_power_w", "ambient_k", "dt_s"],\n            ),',
               what="the descriptor test's parameter lists")
    stamp.hook(desc, "crater_ryugu_sci::CraterRyuguSci::sci_impact().gate_dt_s(),",
               f'            (\n                "{name}",\n                genesis_kernel::{name}::{Name}::specimen().floor_dt_s(),\n                genesis_kernel::{name}::{Name}::specimen().gate_dt_s(),\n            ),',
               skip=1, what="the descriptor test's floors and ceilings")

    stamp.hook("protocol_mcp_gateway/src/chain.rs", 'Continued { card: "crater_ryugu_sci", differ: &["t_s"], within_one_ulp: &[] },',
               f'    Continued {{ card: "{name}", differ: &["boundary_work_j", "energy_anchor_j", "t_s"], within_one_ulp: &[] }},',
               what="the continued cards: what a run cut in two through from ends with otherwise than uncut (the ledger is anchored again at the cut)")
    stamp.hook_after_entry("protocol_mcp_gateway/src/find_model.rs", '    Entry { key: "aerospace/crater_ryugu_sci",', evidence_entry(stamp, f"{domain}/{name}", title, evidence),
                           ends_with="},", what="find_model entry, after the crater's, as the card is after it in the list of cards")
    predecessor = served_citation(stamp, evidence.supersedes) if evidence.supersedes else None
    cited = citation(f"{domain}/{name}", "card", title, evidence, predecessor)
    stamp.hook("protocol_mcp_gateway/data/citations.json", '    "aerospace/crater_ryugu_sci": {', cited, where="before", what="citation")
    counts(stamp, "card")
    sheet_row(stamp, "card", name, title, evidence)
    if evidence.supersedes:
        retired_row(stamp, evidence.supersedes, name)
    stamp.by_hand += [
        f"The tests that count the cards ({served(stamp, 'cards')} before this one): grep -rn '{served(stamp, 'cards')}' protocol_mcp_gateway/tests protocol_mcp_gateway/src | grep -i card",
        f"A frame quantity the template does not use needs its unit in descriptor.rs unit_of; force_torque_state names melt_margin_k for {name}.",
        "Replace the template physics in the card, then its fields in card_<name>.rs, card_<name>_descriptor.rs and the published_reject lines in lib.rs.",
        "find_model entry: its terms; citations.json: its sources when the card stands on someone's data; ends.rs and fresh_start.rs when the card has an end or continuation fields.",
        f"Once the card's own physics is written: python3 protocol_mcp_gateway/kit/new_model.py rows {name} adds the flag_state and unit_of rows its flags and its frame need, and names any it cannot derive (a card with a flag or a frame quantity that has no row makes describe_card fail).",
        "ends.rs: a refusal that is the event finishing (the card's object reaching the end it exists to show) is declared there and served as ended; the stamp declares none.",
        f"chain.rs: the stamp adds a Continued row for {name} with the template's fields, which assumes the start fields carry the card's whole state; if they do not (say, the state holds elements no start field carries), replace it with a NOT_CONTINUED row and its reason, and measure the differ fields with chain_router if they do.",
        "The maps (genesis_kernel/AGENTS.md, protocol_mcp_gateway/AGENTS.md, REFERENCE.md) are on the disk and out of Git.",
    ]


def stamp_solver(stamp, name, title, evidence=None):
    evidence = evidence or Evidence()
    words = {"name": name, "Name": camel(name), "NAME": name.upper(), "domain": "", "Title": title}
    stamp.write_new(f"genesis_solvers/src/{name}.rs", fill("solver/solver.rs", words))
    stamp.write_new(f"genesis_solvers/tests/{name}_conformance.rs", fill("solver/solver_conformance.rs", words))
    stamp.write_new(f"protocol_mcp_gateway/src/solver_{name}.rs", fill("solver/gateway.rs", words))
    if evidence.kind != "specimen" and f"protocol_mcp_gateway/src/solver_{name}.rs" in stamp.pending:
        evidence_result(stamp, f"protocol_mcp_gateway/src/solver_{name}.rs", "crate::descriptor::OWN_SPECIMEN_SOLVER_RESULT.to_string(),", evidence)
    stamp.write_new(f"protocol_mcp_gateway/tests/{name}_router.rs", fill("solver/router_test.rs", words))
    stamp.hook("genesis_solvers/src/lib.rs", "pub mod pipe_flow_superpipe;", f"pub mod {name};", what="the module")
    stamp.hook("protocol_mcp_gateway/src/lib.rs", "mod solver_pipe_flow_superpipe;", f"mod solver_{name};", what="the entry's module")
    stamp.hook("protocol_mcp_gateway/src/solvers.rs", "&crate::solver_pipe_flow_superpipe::PIPE_FLOW_SUPERPIPE,",
               f"    &crate::solver_{name}::{name.upper()},", what="the table of solvers")
    stamp.hook("protocol_mcp_gateway/src/open_plan.rs", '    ("pipe_flow_superpipe", 0.01),', f'    ("{name}", 0.01),',
               what="OPEN_SOLVE_SECONDS: 0.01 s, the template's; measure the solver's own")
    stamp.hook_after_entry("protocol_mcp_gateway/src/find_model.rs", '    Entry { key: "pipe_flow_superpipe",', evidence_entry(stamp, name, title, evidence),
                           ends_with="},", what="find_model entry, after the Superpipe's, as the solver is after it in the table of solvers")
    predecessor = served_citation(stamp, evidence.supersedes) if evidence.supersedes else None
    cited = citation(name, "solver", title, evidence, predecessor)
    stamp.hook("protocol_mcp_gateway/data/citations.json", '    "pipe_flow_superpipe": {', cited, where="before", what="citation")
    counts(stamp, "solver")
    sheet_row(stamp, "solver", name, title, evidence)
    if evidence.supersedes:
        retired_row(stamp, evidence.supersedes, name)
    stamp.by_hand += [
        f"The tests that count the solvers ({served(stamp, 'solvers')} before this one): grep -rn '{served(stamp, 'solvers')}' protocol_mcp_gateway/tests protocol_mcp_gateway/src | grep -i solver",
        "protocol_mcp_gateway/src/open_plan.rs OPEN_SOLVE_SECONDS: the stamp writes 0.01 s; write the solver's measured time for one solve.",
        "Replace the template physics, then the entry's inputs, outputs, refusals and sections.",
    ]


# A quantity's unit from its suffix, longest suffix first.
UNIT_SUFFIXES = [
    ("_m_s2", "m/s^2"), ("_rad_s", "rad/s"), ("_m_s", "m/s"), ("_n_m", "N m"), ("_nms", "N m s"), ("_nm", "N m"),
    ("_n_s", "N s"), ("_mpa", "MPa"), ("_kpa", "kPa"), ("_pa", "Pa"), ("_mw", "MW"), ("_kw", "kW"), ("_w", "W"),
    ("_kj", "kJ"), ("_j", "J"), ("_kev", "keV"), ("_ev", "eV"), ("_k", "K"), ("_kg", "kg"), ("_m3", "m^3"),
    ("_m2", "m^2"), ("_mm", "mm"), ("_um", "um"), ("_m", "m"), ("_hz", "Hz"), ("_ah", "Ah"), ("_a", "A"),
    ("_v", "V"), ("_n", "N"), ("_s", "s"), ("_rad", "rad"), ("_deg", "deg"), ("_db", "dB"), ("_fraction", "1"),
]


def unit_from_suffix(quantity):
    for suffix, unit in UNIT_SUFFIXES:
        if quantity.endswith(suffix):
            return unit
    return None


def body_of(text, signature):
    """The text of the function whose line starts with `signature`, to its
    closing brace at column 0."""
    start = text.find(signature)
    if start < 0:
        return None, None, None
    end = text.find("\n}\n", start)
    return start, end, text[start:end]


def add_rows(stamp, name):
    """The flag_state and unit_of rows card `name` needs, added where they
    can be derived; what cannot be is listed and the run fails."""
    kernel = f"genesis_kernel/src/{name}.rs"
    descriptor = "protocol_mcp_gateway/src/descriptor.rs"
    child = f"protocol_mcp_gateway/src/card_{name}_descriptor.rs"
    if not os.path.exists(stamp.path(kernel)):
        sys.exit(f"{kernel} does not exist: rows reads the card's own source")
    source = stamp.text(kernel)
    failed = []
    flags = re.findall(r"pub const (FLAG_[A-Z0-9_]+): u64", source)
    start, end, flag_body = body_of(stamp.text(descriptor), "fn flag_state_known(")
    if flag_body is None:
        failed.append(f"{descriptor}: no fn flag_state_known to add a row to")
    else:
        rows = []
        for flag in flags:
            if f'"{flag}"' in flag_body:
                continue
            boolean = flag[len("FLAG_"):].lower()
            if not re.search(rf"pub {boolean}: bool", source):
                failed.append(f"{flag}: the card's state declares no boolean {boolean}; write its flag_state row by hand, naming the state boolean that sets it")
                continue
            rows.append(f'        "{flag}" => "{boolean}",')
        if rows:
            text = stamp.text(descriptor)
            at = text.find("        _ => return None,", start)
            stamp.pending[descriptor] = text[:at] + "\n".join(rows) + "\n" + text[at:]
            stamp.done.append(f"edit {descriptor}: flag_state rows for " + ", ".join(row.split('"')[1] for row in rows))
    # The quantities the frame packs, from the stamped descriptor.
    if os.path.exists(stamp.path(child)):
        packed = re.findall(r'\("(?:pos|vel)\[\d\]", "([a-z0-9_]+)"\)', stamp.text(child))
        start, end, unit_body = body_of(stamp.text(descriptor), "fn unit_of_known(")
        rows = []
        for quantity in dict.fromkeys(packed):
            if f'"{quantity}"' in unit_body:
                continue
            unit = unit_from_suffix(quantity)
            if unit is None:
                failed.append(f"{quantity}: its name's suffix names no unit; write its unit_of row by hand")
                continue
            rows.append(f'        "{quantity}" => "{unit}",')
        if rows:
            text = stamp.text(descriptor)
            at = text.find("        _ => return None,", start)
            stamp.pending[descriptor] = text[:at] + "\n".join(rows) + "\n" + text[at:]
            stamp.done.append(f"edit {descriptor}: unit_of rows for " + ", ".join(row.split('"')[1] for row in rows))
    else:
        stamp.by_hand.append(f"{child} does not exist: the frame's quantities of a card described in descriptor.rs itself are held by the gateway test every_card_has_the_rows_its_flags_and_its_frame_need")
    start, end, torque_body = body_of(stamp.text(descriptor), "fn force_torque_state_known(")
    if torque_body is not None and f'"{name}" =>' not in torque_body:
        failed.append(f"{name}: no row in force_torque_state; write the state quantity its force_torque slot holds")
    stamp.by_hand += [
        f"ends.rs: a refusal of {name} that is the event finishing is declared there (served as ended); rows declares none.",
        f"chain.rs: a card whose start fields do not carry its whole state is NOT_CONTINUED, not Continued.",
    ]
    return failed


def evidence_of(parser, args):
    """The evidence the flags give, or the parser's error naming what is missing."""
    if args.kind == "rows":
        if any(getattr(args, flag) is not None for flag in ("fitted_on", "compared_with", "within", "gap", "supersedes")) or args.reference_calculation:
            parser.error("rows adds a card's flag and unit rows; the evidence flags go with card or solver")
        return Evidence()
    if args.within is not None and args.compared_with is None:
        parser.error("--within goes with --compared-with: the accuracy is on the records not in the fit")
    if args.compared_with is not None and args.within is None:
        parser.error("--compared-with goes with --within <the accuracy, in numbers>")
    if args.reference_calculation and args.compared_with is None:
        parser.error("--reference-calculation goes with --compared-with M --within X: the records of the calculation it is held to")
    if args.fitted_on is not None and args.compared_with is None:
        parser.error("--fitted-on goes with --compared-with M --within X: the records not in the fit")
    if args.gap is not None and args.compared_with is None:
        parser.error("--gap goes with --compared-with")
    if args.compared_with is not None and not args.reference_calculation and args.gap is None:
        parser.error("--compared-with goes with --gap <the largest disagreement, one line with its number>: a spec sheet gives its gap")
    if args.gap is not None and not re.search(r"\d", args.gap):
        parser.error("--gap is one line with its number")
    if args.within is not None and not re.search(r"\d", args.within):
        parser.error("--within is the accuracy in numbers")
    for flag in ("fitted_on", "compared_with"):
        value = getattr(args, flag)
        if value is not None and value < 0:
            parser.error(f"--{flag.replace('_', '-')} is a count, 0 or more")
    if args.supersedes == args.name:
        parser.error("--supersedes names the served model this one replaces, not itself")
    return Evidence.of(args)


def main():
    # No abbreviations: the retired flags (--compared, --calibrated)
    # are refused by name, not read as the start of another.
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter, allow_abbrev=False)
    parser.add_argument("kind", choices=["card", "solver", "rows"])
    parser.add_argument("name")
    parser.add_argument("--domain", default="energy", help="a card's domain: energy, materials, signal or aerospace")
    parser.add_argument("--title", default=None, help="what the model is, in a few words")
    parser.add_argument("--root", default=os.path.dirname(os.path.dirname(HERE)))
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--keep-existing", action="store_true", help="keep a model file that exists and stamp the rest around it")
    parser.add_argument("--fitted-on", type=int, help="how many records its constants were fitted on (0: nothing fitted)")
    parser.add_argument("--compared-with", type=int, help="how many records not in the fit it was compared with")
    parser.add_argument("--records", default="records", help="what one record is, plural: runs, profiles, cultures, step transients")
    parser.add_argument("--within", help="the accuracy on the records not in the fit, in numbers: \"2.7 percent rms, 6.1 worst\"")
    parser.add_argument("--gap", help="the largest disagreement, one line with its number")
    parser.add_argument("--reference-calculation", action="store_true", help="held to a published calculation, not a measurement")
    parser.add_argument("--supersedes", help="the served model of the same object this one replaces; its retired.rs row added, so it is refused by name once this one is served")
    args = parser.parse_args()
    evidence = evidence_of(parser, args)
    if not re.fullmatch(r"[a-z][a-z0-9_]*", args.name):
        sys.exit("the name is snake_case: letters, digits and underscores, starting with a letter")
    title = args.title or args.name.replace("_", " ").capitalize()
    stamp = Stamp(os.path.abspath(args.root), args.dry_run, args.keep_existing)
    failed = []
    if args.kind == "card":
        stamp_card(stamp, args.name, args.domain, title, evidence)
    elif args.kind == "rows":
        failed = add_rows(stamp, args.name)
    else:
        stamp_solver(stamp, args.name, title, evidence)
    stamp.commit()
    if args.kind != "rows":
        evidence_by_hand(stamp, args.name, evidence)
    print(("Would write" if args.dry_run else "Wrote") + f" {args.kind} {args.name} in {stamp.root}:")
    for line in stamp.done:
        print("  " + line)
    if stamp.by_hand:
        print("\nLeft to do by hand:")
        for line in stamp.by_hand:
            print("  - " + line.replace("\n", "\n    "))
    if failed:
        print("\nNot derived; each needs its row by hand:")
        for line in failed:
            print("  - " + line)
        sys.exit(1)
    print("\nThen: cargo test --release -p genesis_kernel --test %s_conformance (or genesis_solvers), and"
          " cargo test --release -p protocol_mcp_gateway --test %s_router -- --test-threads=2" % (args.name, args.name))


if __name__ == "__main__":
    main()
