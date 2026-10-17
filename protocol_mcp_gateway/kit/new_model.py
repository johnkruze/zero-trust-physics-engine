#!/usr/bin/env python3
"""Stamp out a new card or solver that starts conformant.

    python3 kit/new_model.py card   <name> --domain energy --title "A heated body" [--root DIR] [--dry-run]
    python3 kit/new_model.py solver <name> --title "A heated rod" [--root DIR] [--dry-run]
    python3 kit/new_model.py rows   <name> [--root DIR] [--dry-run]
    ... card|solver <name> --compared [--protocol FILE [--held-back FILE ...]]
    ... card|solver <name> --calibrated --companion-to <model> [--protocol FILE --output FILE --program FILE --not-blind "<sentence>"]

--compared stamps a model of a real thing compared with measurements of it:
its citation and find_model entry name it so (never "the company's own
specimen"), and forecast first is set up: with --protocol, the protocol's
SHA-256 and each held-back file's are recorded and committed by
kit/forecast.py register before any held-back data is read; without it, the
stamp says the command to run first.

--calibrated --companion-to <model> stamps a model calibrated to the record:
its structure or constants are chosen with the record of a real object open,
so its comparison with that record is not a blind test, and it is served
beside <model>, the served model of the same object, the scored original. It is stamped with the sixth word (find_model's Marks::Calibrated and
its row of CALIBRATIONS, naming the companion), a citation that names the
companion's real object, its sources and the companion, a validation result
that opens by saying it is calibrated to the record and not a blind test and
names the companion, and no blind registration. With --protocol (the
comparison's own record), --output, --program and --not-blind the
calibration is recorded and committed at once by kit/forecast.py calibrate;
without them the stamp says the command to run.

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


def citation(key, kind, title, compared=False):
    """The model's citation. A model compared with measurements of a real
    thing is not the company's own specimen: its citation names the real
    thing and, until it is scored, says that its sources are named when it
    is; its owner writes them in."""
    if compared:
        real_thing = f"{title}, compared with measurements of it"
        how_used = "The measurements it is compared with are named here, with their people and licence, when it is scored against them; the protocol is fixed first (kit/forecasts/" + key.split("/")[-1] + ".json)."
    else:
        real_thing = f"{title}, the company\'s own specimen."
        how_used = "The company\'s own derivation: its constants are its own declared values. Cite the paper."
    return (
        f'    "{key}": {{\n'
        f'      "kind": "{kind}",\n'
        f'      "real_thing": "{real_thing}",\n'
        f'      "sources": [],\n'
        f'      "how_used": "{how_used}",\n'
        f'      "not_held_to_a_measurement": true,\n'
        f'      "cite_also": "paper"\n'
        f'    }},'
    )


# What find_model says of a compared model stamped before its scoring: what
# is so until then, and not "the company's own specimen".
NOT_YET_SCORED = (
    '"not yet scored against the measurements it is compared with: its protocol is fixed and fingerprinted '
    'first (kit/forecasts/{name}.json), and its verdict is written here when it is scored"'
)


# What find_model says after the sixth word of a model stamped calibrated,
# until its own numbers against the record are written.
CALIBRATED_WORDS = (
    '"its numbers against the record, after the fact, are those of its calibration (kit/forecasts/{name}.json)"'
)

# What a stamped calibrated model's validation result opens with.
CALIBRATED_RESULT = (
    "Calibrated to the record, not a blind test: the companion of {companion}, the scored original, which is served beside it under its own word. "
    "Its structure or constants were chosen with the record it is compared with open, and its numbers against that record are after the fact."
)

# What the row of CALIBRATIONS says until the calibration is recorded.
NOT_BLIND_UNTIL_RECORDED = "What of the record was seen before its structure or constants were chosen is written here as its calibration states it (kit/forecasts/{name}.json)."


def rust_text(text):
    return text.replace("\\", "\\\\").replace('"', '\\"')


def companion_citation(stamp, companion):
    """The citation key and entry of the served model `companion`."""
    table = json.loads(stamp.text("protocol_mcp_gateway/data/citations.json"))
    found = [key for key in table["models"] if key.split("/")[-1] == companion]
    if len(found) != 1:
        sys.exit(f"--companion-to {companion}: no served model of that name in protocol_mcp_gateway/data/citations.json")
    return found[0], table["models"][found[0]]


def calibrated_citation(stamp, key, kind, companion):
    """The citation of a model calibrated to the record: the real object and
    the sources are its companion's, and how they are used says so."""
    _, own = companion_citation(stamp, companion)
    entry = {
        "kind": kind,
        "real_thing": own["real_thing"],
        "sources": own["sources"],
        "how_used": (
            f"Calibrated to the record: its structure or constants were chosen with the record of {companion}'s real object open, so its comparison "
            f"with that record is not a blind test; {companion}, the scored original, is served beside it, and the sources are {companion}'s."
        ),
        "not_held_to_a_measurement": False,
        "cite_also": "paper",
    }
    lines = json.dumps({key: entry}, indent=2, ensure_ascii=False).split("\n")[1:-1]
    return "\n".join("  " + line for line in lines) + ","


def calibration_row(name, companion, not_blind):
    return f'    Calibration {{ model: "{name}", companion_to: "{companion}", not_blind: "{rust_text(not_blind)}" }},'


def entry(key, title, compared=False, calibrated=None):
    words = title.lower()
    if calibrated:
        marks, marks_words = "Marks::Calibrated", CALIBRATED_WORDS.format(name=key.split("/")[-1])
    else:
        marks, marks_words = "Marks::NotCompared", NOT_YET_SCORED.format(name=key.split("/")[-1]) if compared else "OWN_SPECIMEN"
    return (
        f'    Entry {{ key: "{key}", marks: {marks}, marks_words: {marks_words},\n'
        f'        strong: &["{words}"],\n'
        f'        plain: &[],\n'
        f'        weak: &[],\n'
        f'        other: &[],\n'
        f'        own: &[] }},'
    )


def compared_by_hand(stamp, name, args):
    """What a compared model's stamp leaves, forecast first."""
    if args.protocol:
        if stamp.dry:
            stamp.done.append(f"would register the protocol {args.protocol} and {len(args.held_back)} held-back file(s) in kit/forecasts/{name}.json and commit it")
        else:
            done = subprocess.run(
                [sys.executable, os.path.join(HERE, "forecast.py"), "--root", stamp.root, "register", name, "--protocol", args.protocol, "--held-back", *args.held_back],
                capture_output=True, text=True,
            )
            if done.returncode != 0:
                sys.exit(done.stderr.strip() or done.stdout.strip())
            stamp.done.append(done.stdout.strip())
    else:
        stamp.by_hand.append(
            f"Forecast first, before any held-back data is read: python3 protocol_mcp_gateway/kit/forecast.py register {name} --protocol <its validation_protocol.md> --held-back <each held-back file>; "
            f"read each held-back file only through python3 protocol_mcp_gateway/kit/forecast.py open {name} <file>, and record the scoring with forecast.py score {name} --output <the scored output> --program <the scoring program>. "
            "The conformance suite fails a compared model whose registration does not show a committed, fingerprinted protocol older than its scoring."
        )
    stamp.by_hand += [
        f"The stamped citation and find_model entry of {name} name it a model compared with measurements, not yet scored: once scored, write its sources (people, data set, licence) and how they are used in citations.json, set not_held_to_a_measurement to false, and give its find_model entry the verdict (Marks::Met, InPart or Missed) and its words. Until then find_model's test, which holds every NotCompared entry to the company's own specimen, fails on it by name: that is the guard against serving it unscored.",
        "The stamped description is the template's, the company's own specimen: rewrite its specimen, validation record and summary as the real thing it is, with the range it was compared on and how it did there, in numbers.",
    ]


def not_blind_of(args):
    return " ".join((args.not_blind or "").split())


def calibrated_by_hand(stamp, name, args):
    """What a calibrated model's stamp leaves: its calibration, recorded now
    when the stamp is given the comparison's files, and its own words."""
    companion = args.companion_to
    given = [args.protocol, args.output, args.program, args.not_blind]
    if all(given):
        if stamp.dry:
            stamp.done.append(f"would record the calibration of {name} beside {companion} in kit/forecasts/{name}.json and commit it")
        else:
            done = subprocess.run(
                [sys.executable, os.path.join(HERE, "forecast.py"), "--root", stamp.root, "calibrate", name, "--companion-to", companion,
                 "--protocol", args.protocol, "--output", args.output, "--program", args.program, "--not-blind", not_blind_of(args)],
                capture_output=True, text=True,
            )
            if done.returncode != 0:
                sys.exit(done.stderr.strip() or done.stdout.strip())
            stamp.done.append(done.stdout.strip())
    else:
        stamp.by_hand.append(
            f"Its calibration: python3 protocol_mcp_gateway/kit/forecast.py calibrate {name} --companion-to {companion} --protocol <the comparison's own record> "
            f"--output <its scored output> --program <the program that scored it> --not-blind \"<what of the record was seen before the structure or constants were chosen>\"; "
            f"then the same sentence as not_blind in its row of CALIBRATIONS (find_model.rs), which find_model's test holds to the registration word for word. "
            "Until it is recorded the served sweep names the calibration missing."
        )
    stamp.by_hand += [
        f"The stamped description is the template's: rewrite its specimen, data and limits as the real object it is calibrated to. Its validation result opens, as stamped, by saying it is calibrated to the record and not a blind test and naming {companion}; give its numbers against the record there, as after the fact, and in its find_model marks_words.",
        f"No blind registration is stamped. A forecast of data not yet read is registered with forecast.py register {name} --protocol <its validation_protocol.md> --held-back <each file>, beside the calibration in the same record; its word stays calibrated to the record until that forecast is scored (forecast.py score {name} ... --verdict met, \"met in part\" or missed), and then is the scoring's.",
    ]


def calibrated_result(stamp, rel, old, companion):
    """The stamped validation result of a calibrated model, in place of the template's."""
    stamp.replace_once(rel, re.escape(old), lambda _: f'"{rust_text(CALIBRATED_RESULT.format(companion=companion))}".to_string(),',
                       what="the validation result: calibrated to the record, not a blind test, beside its companion")


def calibrated_registrations(stamp, name, companion, args):
    stamp.hook("protocol_mcp_gateway/src/find_model.rs", "pub(crate) const CALIBRATIONS: &[Calibration] = &[",
               calibration_row(name, companion, not_blind_of(args) or NOT_BLIND_UNTIL_RECORDED.format(name=name)),
               what="its row of CALIBRATIONS: the companion and what of the record was seen first")


def stamp_card(stamp, name, domain, title, compared=False, calibrated=None, args=None):
    words = {"name": name, "Name": camel(name), "NAME": name.upper(), "domain": domain, "Title": title}
    Name = words["Name"]
    stamp.write_new(f"genesis_kernel/src/{name}.rs", fill("card/kernel.rs", words))
    stamp.write_new(f"genesis_kernel/tests/{name}_conformance.rs", fill("card/kernel_conformance.rs", words))
    stamp.write_new(f"protocol_mcp_gateway/src/card_{name}.rs", fill("card/gateway.rs", words))
    stamp.write_new(f"protocol_mcp_gateway/src/card_{name}_descriptor.rs", fill("card/descriptor.rs", words))
    if calibrated and f"protocol_mcp_gateway/src/card_{name}_descriptor.rs" in stamp.pending:
        calibrated_result(stamp, f"protocol_mcp_gateway/src/card_{name}_descriptor.rs",
                          '"Held: the card\'s tests run it from the floor to the ceiling, and every step holds its conservation ledger to its stated limit.".to_string(),', calibrated)
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
    stamp.hook(desc, '("aerospace", "crater_ryugu_sci", vec!["radius_m", "dt_s"]),',
               f'            ("{domain}", "{name}", vec!["temperature_k", "heat_power_w", "ambient_k", "dt_s"]),', what="the descriptor test's parameter lists")
    stamp.hook(desc, "crater_ryugu_sci::CraterRyuguSci::sci_impact().gate_dt_s(),",
               f'            (\n                "{name}",\n                genesis_kernel::{name}::{Name}::specimen().floor_dt_s(),\n                genesis_kernel::{name}::{Name}::specimen().gate_dt_s(),\n            ),',
               skip=1, what="the descriptor test's floors and ceilings")

    stamp.hook("protocol_mcp_gateway/src/chain.rs", 'Continued { card: "crater_ryugu_sci", differ: &["t_s"], within_one_ulp: &[] },',
               f'    Continued {{ card: "{name}", differ: &["boundary_work_j", "energy_anchor_j", "t_s"], within_one_ulp: &[] }},',
               what="the continued cards: what a run cut in two through from ends with otherwise than uncut (the ledger is anchored again at the cut)")
    stamp.hook_after_entry("protocol_mcp_gateway/src/find_model.rs", '    Entry { key: "aerospace/crater_ryugu_sci",', entry(f"{domain}/{name}", title, compared, calibrated),
                           ends_with="},", what="find_model entry, after the crater's, as the card is after it in the list of cards")
    if calibrated:
        calibrated_registrations(stamp, name, calibrated, args)
    cited = calibrated_citation(stamp, f"{domain}/{name}", "card", calibrated) if calibrated else citation(f"{domain}/{name}", "card", title, compared)
    stamp.hook("protocol_mcp_gateway/data/citations.json", '    "aerospace/crater_ryugu_sci": {', cited, where="before", what="citation")
    counts(stamp, "card")
    stamp.by_hand += [
        f"The tests that count the cards ({served(stamp, 'cards')} before this one): grep -rn '{served(stamp, 'cards')}' protocol_mcp_gateway/tests protocol_mcp_gateway/src | grep -i card",
        "protocol_mcp_gateway/tests/tools_093.rs FULL_DESCRIPTIONS: the pinned hash of the full descriptions moves; the main session recomputes it at the merge.",
        f"A frame quantity the template does not use needs its unit in descriptor.rs unit_of; force_torque_state names melt_margin_k for {name}.",
        "Replace the template physics in the card, then its fields in card_<name>.rs, card_<name>_descriptor.rs and the published_reject lines in lib.rs.",
        "find_model entry: its terms; citations.json: its sources when the card stands on someone's data; ends.rs and fresh_start.rs when the card has an end or continuation fields.",
        f"Once the card's own physics is written: python3 protocol_mcp_gateway/kit/new_model.py rows {name} adds the flag_state and unit_of rows its flags and its frame need, and names any it cannot derive (a card with a flag or a frame quantity that has no row makes describe_card fail).",
        "ends.rs: a refusal that is the event finishing (the card's object reaching the end it exists to show) is declared there and served as ended; the stamp declares none.",
        f"chain.rs: the stamp adds a Continued row for {name} with the template's fields, which assumes the start fields carry the card's whole state; if they do not (say, the state holds elements no start field carries), replace it with a NOT_CONTINUED row and its reason, and measure the differ fields with chain_router if they do.",
        "The maps (genesis_kernel/AGENTS.md, protocol_mcp_gateway/AGENTS.md, REFERENCE.md) are on the disk and out of Git.",
    ]


def stamp_solver(stamp, name, title, compared=False, calibrated=None, args=None):
    words = {"name": name, "Name": camel(name), "NAME": name.upper(), "domain": "", "Title": title}
    stamp.write_new(f"genesis_solvers/src/{name}.rs", fill("solver/solver.rs", words))
    stamp.write_new(f"genesis_solvers/tests/{name}_conformance.rs", fill("solver/solver_conformance.rs", words))
    stamp.write_new(f"protocol_mcp_gateway/src/solver_{name}.rs", fill("solver/gateway.rs", words))
    if calibrated and f"protocol_mcp_gateway/src/solver_{name}.rs" in stamp.pending:
        calibrated_result(stamp, f"protocol_mcp_gateway/src/solver_{name}.rs", "crate::descriptor::OWN_SPECIMEN_SOLVER_RESULT.to_string(),", calibrated)
    stamp.write_new(f"protocol_mcp_gateway/tests/{name}_router.rs", fill("solver/router_test.rs", words))
    stamp.hook("genesis_solvers/src/lib.rs", "pub mod pipe_flow_superpipe;", f"pub mod {name};", what="the module")
    stamp.hook("protocol_mcp_gateway/src/lib.rs", "mod solver_pipe_flow_superpipe;", f"mod solver_{name};", what="the entry's module")
    stamp.hook("protocol_mcp_gateway/src/solvers.rs", "&crate::solver_pipe_flow_superpipe::PIPE_FLOW_SUPERPIPE,",
               f"    &crate::solver_{name}::{name.upper()},", what="the table of solvers")
    stamp.hook("protocol_mcp_gateway/src/open_plan.rs", '    ("pipe_flow_superpipe", 0.01),', f'    ("{name}", 0.01),',
               what="OPEN_SOLVE_SECONDS: 0.01 s, the template's; measure the solver's own")
    stamp.hook_after_entry("protocol_mcp_gateway/src/find_model.rs", '    Entry { key: "pipe_flow_superpipe",', entry(name, title, compared, calibrated),
                           ends_with="},", what="find_model entry, after the Superpipe's, as the solver is after it in the table of solvers")
    if calibrated:
        calibrated_registrations(stamp, name, calibrated, args)
    cited = calibrated_citation(stamp, name, "solver", calibrated) if calibrated else citation(name, "solver", title, compared)
    stamp.hook("protocol_mcp_gateway/data/citations.json", '    "pipe_flow_superpipe": {', cited, where="before", what="citation")
    counts(stamp, "solver")
    stamp.by_hand += [
        f"The tests that count the solvers ({served(stamp, 'solvers')} before this one): grep -rn '{served(stamp, 'solvers')}' protocol_mcp_gateway/tests protocol_mcp_gateway/src | grep -i solver",
        "protocol_mcp_gateway/src/open_plan.rs OPEN_SOLVE_SECONDS: the stamp writes 0.01 s; write the solver's measured time for one solve.",
        "protocol_mcp_gateway/tests/tools_093.rs FULL_DESCRIPTIONS: the pinned hash moves; the main session recomputes it at the merge.",
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


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("kind", choices=["card", "solver", "rows"])
    parser.add_argument("name")
    parser.add_argument("--domain", default="energy", help="a card's domain: energy, materials, signal or aerospace")
    parser.add_argument("--title", default=None, help="what the model is, in a few words")
    parser.add_argument("--root", default=os.path.dirname(os.path.dirname(HERE)))
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--keep-existing", action="store_true", help="keep a model file that exists and stamp the rest around it")
    parser.add_argument("--compared", action="store_true", help="a model of a real thing compared with measurements of it: its citation and find_model entry say so, not the company's own specimen, and forecast first is set up")
    parser.add_argument("--protocol", help="with --compared: its validation protocol, fingerprinted and committed now by kit/forecast.py register, before any held-back data is read; with --calibrated: the comparison's own record, for kit/forecast.py calibrate")
    parser.add_argument("--held-back", nargs="*", default=[], help="with --protocol: each held-back data file, fingerprinted as it is held")
    parser.add_argument("--calibrated", action="store_true", help="a model calibrated to the record: its structure or constants chosen with the record of a real object open, served beside --companion-to under the sixth word, with no blind registration")
    parser.add_argument("--companion-to", help="with --calibrated: the served model of the same real object, the scored original, that it stands beside")
    parser.add_argument("--output", help="with --calibrated --protocol: the comparison's scored output")
    parser.add_argument("--program", help="with --calibrated --protocol: the program that scored it")
    parser.add_argument("--not-blind", help="with --calibrated --protocol: one sentence, what of the record was seen before the structure or constants were chosen")
    args = parser.parse_args()
    if args.calibrated and args.compared:
        sys.exit("--calibrated and --compared are two words: a model calibrated to the record is a companion of a scored original")
    if args.calibrated != bool(args.companion_to):
        sys.exit("--calibrated goes with --companion-to <the served model it stands beside>")
    if (args.output or args.program or args.not_blind) and not args.calibrated:
        sys.exit("--output, --program and --not-blind are for a model calibrated to the record: give --calibrated")
    if args.calibrated:
        given = [args.protocol, args.output, args.program, args.not_blind]
        if any(given) and not all(given):
            sys.exit("--protocol, --output, --program and --not-blind record the calibration together: give all four, or none")
        if args.held_back:
            sys.exit("--held-back is for a blind registration, which a calibrated stamp does not make: register one later with kit/forecast.py register")
        if args.companion_to == args.name:
            sys.exit("--companion-to names the served model it stands beside, not itself")
    elif args.protocol or args.held_back:
        if not args.compared:
            sys.exit("--protocol and --held-back are for a model compared with measurements: give --compared")
    if args.held_back and not args.protocol:
        sys.exit("--held-back needs --protocol: the protocol is fixed before the held-back data is fingerprinted")
    if not re.fullmatch(r"[a-z][a-z0-9_]*", args.name):
        sys.exit("the name is snake_case: letters, digits and underscores, starting with a letter")
    title = args.title or args.name.replace("_", " ").capitalize()
    stamp = Stamp(os.path.abspath(args.root), args.dry_run, args.keep_existing)
    failed = []
    calibrated = args.companion_to if args.calibrated else None
    if args.kind == "card":
        stamp_card(stamp, args.name, args.domain, title, args.compared, calibrated, args)
    elif args.kind == "rows":
        failed = add_rows(stamp, args.name)
    else:
        stamp_solver(stamp, args.name, title, args.compared, calibrated, args)
    stamp.commit()
    if args.compared and args.kind != "rows":
        compared_by_hand(stamp, args.name, args)
    if calibrated and args.kind != "rows":
        calibrated_by_hand(stamp, args.name, args)
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
