#!/usr/bin/env python3
"""Tests of the kit: python3 -I kit/test_kit.py

- the stamp finds every place it edits in this workspace (a dry run), fills
  every placeholder, and refuses to overwrite a file;
- the merge tool merges a clean branch, reports and undoes a conflicting one,
  says what a merge would do with --dry-run and touches nothing then,
  reports a branch with nothing new, and stops a merge where a line changed
  the same way in both branches would land once (a count one short);
- the measure tool tells a moved seal from bytes moved under kept seals;
- the stamp writes a compared model's spec sheet (its SHEETS row: fitted on,
  compared with ... within, the gap), Marks::Measured, and with --supersedes
  the succession.rs row;
- forecasts: a forecast is committed once, its data opens only after that
  commit, opening and scoring commit nothing, and a forecast changed or a
  scoring before it is named;
- check holds a model by the sweep's rule: a forecast its served description
  states is held to its registration, and a model that states none to
  nothing, a calibration with no companion among them; calibrate is retired.
"""

import importlib.util
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
WORKSPACE = os.path.dirname(os.path.dirname(HERE))


def load(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PLACEHOLDER = re.compile(r"\{\{(name|Name|NAME|domain|Title)\}\}")

new_model = load("new_model")
merge_branches = load("merge_branches")
before_after = load("before_after")
forecast = load("forecast")


def git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True, check=True).stdout


def repo_with(path, files, branch="main"):
    os.makedirs(path)
    git(path, "init", "-q", "-b", branch)
    git(path, "config", "user.email", "kit@example.com")
    git(path, "config", "user.name", "kit")
    for name, text in files.items():
        with open(os.path.join(path, name), "w") as f:
            f.write(text)
    git(path, "add", "-A")
    git(path, "commit", "-q", "-m", "start")


class TheStamp(unittest.TestCase):
    def test_a_dry_run_finds_every_place_it_edits(self):
        for kind, name in (("card", "kit_dry_run_body"), ("solver", "kit_dry_run_rod")):
            stamp = new_model.Stamp(WORKSPACE, dry=True)
            if kind == "card":
                new_model.stamp_card(stamp, name, "energy", "A dry run")
            else:
                new_model.stamp_solver(stamp, name, "A dry run")
            missed = [line for line in stamp.by_hand if "places hold the anchor" in line or line.endswith("+ 1")]
            self.assertEqual(missed, [], f"{kind}: anchors not found")
            self.assertTrue(any(line.startswith("new ") for line in stamp.done))
            for rel, body in stamp.pending.items():
                self.assertIsNone(PLACEHOLDER.search(body), rel)
            self.assertFalse(os.path.exists(os.path.join(WORKSPACE, f"genesis_kernel/src/{name}.rs")))

    def test_every_template_is_filled(self):
        words = {"name": "x_y", "Name": "XY", "NAME": "X_Y", "domain": "energy", "Title": "T"}
        for template in ("card/kernel.rs", "card/gateway.rs", "card/descriptor.rs", "card/router_test.rs", "card/kernel_conformance.rs",
                         "solver/solver.rs", "solver/gateway.rs", "solver/router_test.rs", "solver/solver_conformance.rs"):
            self.assertIsNone(PLACEHOLDER.search(new_model.fill(template, words)), template)

    def test_a_hook_edits_one_place_or_says_so(self):
        with tempfile.TemporaryDirectory() as root:
            with open(os.path.join(root, "f.rs"), "w") as f:
                f.write("a\nanchor\nb\nanchor twice\n")
            stamp = new_model.Stamp(root, dry=False)
            stamp.hook("f.rs", "zzz", "new", what="missing")
            stamp.hook("f.rs", "anchor", "new", what="twice")
            stamp.hook("f.rs", "a", "first", where="before", next_has="anchor", what="once")
            stamp.commit()
            with open(os.path.join(root, "f.rs")) as f:
                self.assertEqual(f.read(), "first\na\nanchor\nb\nanchor twice\n")
            self.assertEqual(len(stamp.by_hand), 2)

    def test_the_stamp_overwrites_nothing(self):
        with tempfile.TemporaryDirectory() as root:
            with open(os.path.join(root, "there.rs"), "w") as f:
                f.write("kept")
            stamp = new_model.Stamp(root, dry=False)
            with self.assertRaises(SystemExit):
                stamp.write_new("there.rs", "new")

    def test_a_kept_file_is_kept_and_the_rest_is_stamped(self):
        with tempfile.TemporaryDirectory() as root:
            with open(os.path.join(root, "there.rs"), "w") as f:
                f.write("kept")
            stamp = new_model.Stamp(root, dry=False, keep_existing=True)
            stamp.write_new("there.rs", "new")
            stamp.write_new("fresh.rs", "new")
            stamp.commit()
            with open(os.path.join(root, "there.rs")) as f:
                self.assertEqual(f.read(), "kept")
            with open(os.path.join(root, "fresh.rs")) as f:
                self.assertEqual(f.read(), "new")
            self.assertIn("kept there.rs, which exists", stamp.done)

    def test_an_entry_goes_after_the_whole_entry_of_its_anchor(self):
        with tempfile.TemporaryDirectory() as root:
            with open(os.path.join(root, "f.rs"), "w") as f:
                f.write('    Entry { key: "a",\n        own: &[] },\n    Entry { key: "pipe",\n        weak: &[],\n        own: &[] },\n    Entry { key: "z",\n        own: &[] },\n')
            stamp = new_model.Stamp(root, dry=False)
            stamp.hook_after_entry("f.rs", '    Entry { key: "pipe",', '    Entry { key: "new",\n        own: &[] },', ends_with="},")
            stamp.commit()
            with open(os.path.join(root, "f.rs")) as f:
                keys = re.findall(r'key: "(\w+)"', f.read())
            self.assertEqual(keys, ["a", "pipe", "new", "z"])

    def test_a_stamped_solver_is_indexed_in_the_order_it_is_served(self):
        stamp = new_model.Stamp(WORKSPACE, dry=True)
        new_model.stamp_solver(stamp, "kit_order_rod", "An order test")
        table = re.findall(r"&crate::solver_(\w+)::", stamp.pending["protocol_mcp_gateway/src/solvers.rs"])
        index = [key for key in re.findall(r'Entry \{ key: "([\w/]+)"', stamp.pending["protocol_mcp_gateway/src/find_model.rs"]) if "/" not in key]
        self.assertEqual(index, [name for name in table if name != "probe"])
        stamp = new_model.Stamp(WORKSPACE, dry=True)
        new_model.stamp_card(stamp, "kit_order_body", "energy", "An order test")
        index = re.findall(r'Entry \{ key: "([\w/]+)"', stamp.pending["protocol_mcp_gateway/src/find_model.rs"])
        self.assertEqual(index[index.index("aerospace/crater_ryugu_sci") + 1], "energy/kit_order_body")

    def test_rows_adds_the_flag_and_unit_rows_a_card_needs_and_names_the_rest(self):
        with tempfile.TemporaryDirectory() as root:
            os.makedirs(os.path.join(root, "genesis_kernel/src"))
            os.makedirs(os.path.join(root, "protocol_mcp_gateway/src"))
            with open(os.path.join(root, "genesis_kernel/src/lift.rs"), "w") as f:
                f.write("pub const FLAG_GROSS_SLIP: u64 = 1;\npub const FLAG_NO_BOOLEAN: u64 = 2;\npub struct S {\n    pub gross_slip: bool,\n}\n")
            with open(os.path.join(root, "protocol_mcp_gateway/src/card_lift_descriptor.rs"), "w") as f:
                f.write('("pos[0]", "object_height_m"),\n("pos[1]", "temperature_k"),\n("vel[0]", "object_velocity_m_s"),\n("vel[1]", "stick_ratio"),\n')
            with open(os.path.join(root, "protocol_mcp_gateway/src/descriptor.rs"), "w") as f:
                f.write('fn unit_of_known(state: &str) -> Option<&\'static str> {\n    Some(match state {\n        "temperature_k" => "K",\n        _ => return None,\n    })\n}\n'
                        'fn force_torque_state_known(card: &str) -> Option<&\'static str> {\n    Some(match card {\n        _ => return None,\n    })\n}\n'
                        'fn flag_state_known(name: &str) -> Option<&\'static str> {\n    Some(match name {\n        "FLAG_OLD" => "old",\n        _ => return None,\n    })\n}\n')
            stamp = new_model.Stamp(root, dry=False)
            failed = new_model.add_rows(stamp, "lift")
            stamp.commit()
            with open(os.path.join(root, "protocol_mcp_gateway/src/descriptor.rs")) as f:
                text = f.read()
            self.assertIn('"FLAG_GROSS_SLIP" => "gross_slip",', text)
            self.assertIn('"object_height_m" => "m",', text)
            self.assertIn('"object_velocity_m_s" => "m/s",', text)
            self.assertNotIn('"temperature_k" => "K",\n        "temperature_k"', text)
            named = "\n".join(failed)
            self.assertIn("FLAG_NO_BOOLEAN", named)
            self.assertIn("stick_ratio", named)
            self.assertIn("lift: no row in force_torque_state", named)

    def test_rows_on_every_served_card_finds_every_row_there(self):
        for name in ("grasp_lift_glass_4tu", "momentum_jwst_srp"):
            stamp = new_model.Stamp(WORKSPACE, dry=True)
            self.assertEqual(new_model.add_rows(stamp, name), [], name)
            self.assertFalse(any(line.startswith("edit ") for line in stamp.done), f"{name}: {stamp.done}")

    def test_a_compared_model_is_stamped_as_its_spec_sheet(self):
        evidence = new_model.Evidence("measurement", 12, 4, "cultures", "6.1 percent rms", "dry biomass 3.64 g/L against 2.15")
        line = "fitted on 12 cultures; compared with 4 cultures not in the fit, within 6.1 percent rms; the largest gap: dry biomass 3.64 g/L against 2.15"
        self.assertEqual(evidence.spec_line(), line)
        self.assertEqual(new_model.Evidence("measurement", 0, 33, "points", "2 percent").spec_line(), "nothing fitted; compared with 33 points not in the fit, within 2 percent")
        for kind, name in (("card", "kit_compared_body"), ("solver", "kit_compared_rod")):
            stamp = new_model.Stamp(WORKSPACE, dry=True)
            if kind == "card":
                new_model.stamp_card(stamp, name, "energy", "A dry run", evidence)
                described, key = stamp.pending[f"protocol_mcp_gateway/src/card_{name}_descriptor.rs"], f"energy/{name}"
            else:
                new_model.stamp_solver(stamp, name, "A dry run", evidence)
                described, key = stamp.pending[f"protocol_mcp_gateway/src/solver_{name}.rs"], name
            new_model.evidence_by_hand(stamp, name, evidence)
            missed = [line for line in stamp.by_hand if "places hold the anchor" in line or line.endswith("+ 1")]
            self.assertEqual(missed, [], f"{kind}: anchors not found")
            # Marks::Measured in its find_model entry; the spec line in its citation and its record's result.
            found = stamp.pending["protocol_mcp_gateway/src/find_model.rs"]
            stamped = found.split(f'Entry {{ key: "{key}",')[1].split("},")[0]
            self.assertIn(f'marks: Marks::Measured, marks_words: "{new_model.MEASURED_WORDS}"', stamped)
            cited = json.loads(stamp.pending["protocol_mcp_gateway/data/citations.json"])["models"][key]
            self.assertIn(evidence.result(), cited["how_used"])
            self.assertFalse(cited["not_held_to_a_measurement"])
            self.assertIn(new_model.rust_text(evidence.result()), described)
            # Its row of spec_sheet::SHEETS, the six parts, the three of numbers from the flags.
            sheets = stamp.pending["protocol_mcp_gateway/src/spec_sheet.rs"]
            row = sheets.split(f'        model: "{name}",')[1].split("    },")[0]
            self.assertEqual(re.findall(r"^        (\w+): ", row, re.M), ["computes", "range", "fitted_on", "compared", "gap", "refuses"])
            self.assertIn('fitted_on: "Fitted on 12 cultures."', row)
            self.assertIn('compared: "Compared with 4 cultures not in the fit: within 6.1 percent rms."', row)
            self.assertIn('gap: "Gap: dry biomass 3.64 g/L against 2.15."', row)
            self.assertIn('range: "Range: ', row)
            self.assertIn('refuses: "Refuses ', row)
            anchor = new_model.SHEET_ANCHORS[kind]
            self.assertEqual(sheets.index(row) + len(row) + len("    },\n"), sheets.index(anchor), kind)
            # No verdict and no ritual: the numbers, and no word of how it did.
            text = stamped + json.dumps(cited) + row + " ".join(stamp.by_hand)
            for word in ("own specimen", "OWN_SPECIMEN", "blind", "pass mark", "pass line", "verdict", "held-back", "missed", " met ", "calibrat"):
                self.assertNotIn(word, text, f"{kind}: {word}")
        # A company specimen is stamped as one, with no spec sheet.
        own = new_model.citation("energy/kit_own_body", "card", "A heated body") + new_model.entry("energy/kit_own_body", "A heated body")
        self.assertIn("a company specimen", own)
        self.assertIn("Company specimen:", own)
        self.assertIn("OWN_SPECIMEN", own)
        stamp = new_model.Stamp(WORKSPACE, dry=True)
        new_model.stamp_solver(stamp, "kit_own_rod", "A dry run")
        self.assertNotIn("protocol_mcp_gateway/src/spec_sheet.rs", stamp.pending)
        self.assertIn("Company specimen: A dry run", stamp.pending["protocol_mcp_gateway/src/solver_kit_own_rod.rs"])
        # A reference calculation: Marks::Reference, its numbers its marks_words, no spec sheet.
        reference = new_model.Evidence("reference", None, 33, "points", "2 percent")
        stamp = new_model.Stamp(WORKSPACE, dry=True)
        new_model.stamp_solver(stamp, "kit_reference_rod", "A dry run", reference)
        self.assertIn('Entry { key: "kit_reference_rod", marks: Marks::Reference, marks_words: "33 points, within 2 percent",', stamp.pending["protocol_mcp_gateway/src/find_model.rs"])
        self.assertNotIn("protocol_mcp_gateway/src/spec_sheet.rs", stamp.pending)
        json.loads("{" + new_model.citation("energy/kit_own_body", "card", "A heated body").rstrip(",") + "}")

    def test_the_marks_are_the_variants_find_model_declares(self):
        stamp = new_model.Stamp(WORKSPACE, dry=True)
        for kind, variant in (("measurement", "Marks::Measured"), ("reference", "Marks::Reference"), ("specimen", "Marks::NotCompared")):
            self.assertEqual(new_model.marks_of(stamp, new_model.Evidence(kind)), variant)
        # On a tree whose find_model declares no Measured, the stamp says so.
        with tempfile.TemporaryDirectory() as root:
            os.makedirs(os.path.join(root, "protocol_mcp_gateway/src"))
            with open(os.path.join(root, "protocol_mcp_gateway/src/find_model.rs"), "w") as f:
                f.write("pub enum Marks {\n    Met,\n    NotCompared,\n}\n")
            stamp = new_model.Stamp(root, dry=True)
            self.assertIsNone(new_model.marks_of(stamp, new_model.Evidence("measurement")))
            self.assertEqual(new_model.marks_of(stamp, new_model.Evidence("specimen")), "Marks::NotCompared")
            self.assertIn("Marks::NotCompared", new_model.evidence_entry(stamp, "kit_word_rod", "A dry run", new_model.Evidence("measurement", 1, 2, "runs", "1 percent", "x 2")))
            self.assertTrue(any("declares no Marks::Measured" in line for line in stamp.by_hand), stamp.by_hand)

    def test_a_model_that_supersedes_another_takes_its_object(self):
        evidence = new_model.Evidence("measurement", 12, 4, "cultures", "6.1 percent rms", "3.64 g/L against 2.15", supersedes="culture_trichoderma_ambr250")
        stamp = new_model.Stamp(WORKSPACE, dry=True)
        new_model.stamp_solver(stamp, "kit_successor_rod", "A dry run", evidence)
        new_model.evidence_by_hand(stamp, "kit_successor_rod", evidence)
        cited = json.loads(stamp.pending["protocol_mcp_gateway/data/citations.json"])["models"]
        own = cited["culture_trichoderma_ambr250"]
        self.assertEqual((cited["kit_successor_rod"]["real_thing"], cited["kit_successor_rod"]["sources"]), (own["real_thing"], own["sources"]))
        self.assertIn("supersedes culture_trichoderma_ambr250", cited["kit_successor_rod"]["how_used"])
        self.assertIn("supersedes culture_trichoderma_ambr250", evidence.spec_line())
        # The successor's row at the end of succession::TABLE, the old ID kept served.
        table = stamp.pending["protocol_mcp_gateway/src/succession.rs"].split("pub const TABLE: &[Succession] = &[")[1].split("\n];")[0]
        self.assertTrue(table.endswith('\n    Succession { model: "culture_trichoderma_ambr250", by: "kit_successor_rod" },'), table)
        self.assertTrue(any('superseded_by "kit_successor_rod"' in line for line in stamp.by_hand), stamp.by_hand)
        # A model superseded already is not given a second successor.
        served = new_model.Stamp(WORKSPACE, dry=True).text("protocol_mcp_gateway/src/succession.rs")
        taken = re.search(r'Succession \{ model: "(\w+)", by: "(\w+)" \}', served.split("pub const TABLE")[1])
        if taken:
            again = new_model.Stamp(WORKSPACE, dry=True)
            new_model.succession_row(again, taken.group(1), "kit_second_rod")
            self.assertEqual(again.pending["protocol_mcp_gateway/src/succession.rs"], served)
            self.assertTrue(any(f"superseded already, by {taken.group(2)}" in line for line in again.by_hand), again.by_hand)

    def test_the_flags_go_together_or_the_stamp_says_why_not(self):
        def run(*extra):
            return subprocess.run([sys.executable, os.path.join(HERE, "new_model.py"), "solver", "kit_flags_rod", "--dry-run", *extra], capture_output=True, text=True)

        def refused(*extra):
            done = run(*extra)
            self.assertNotEqual(done.returncode, 0, extra)
            return done.stderr
        self.assertIn("--within", refused("--compared-with", "4"))
        self.assertIn("--gap", refused("--compared-with", "4", "--within", "2 percent"))
        self.assertIn("its number", refused("--compared-with", "4", "--within", "2 percent", "--gap", "it reads low"))
        self.assertIn("in numbers", refused("--compared-with", "4", "--within", "closely", "--gap", "3 percent"))
        self.assertIn("--compared-with", refused("--within", "2 percent"))
        self.assertIn("--compared-with", refused("--fitted-on", "3"))
        self.assertIn("--compared-with", refused("--gap", "x"))
        self.assertIn("--compared-with", refused("--reference-calculation"))
        self.assertIn("a count", refused("--fitted-on", "-1", "--compared-with", "4", "--within", "2 percent", "--gap", "3 percent"))
        self.assertIn("not itself", refused("--supersedes", "kit_flags_rod"))
        self.assertIn("no served model", refused("--supersedes", "nothing_served"))
        # The old flags are gone.
        self.assertIn("unrecognized arguments", refused("--compared"))
        self.assertIn("unrecognized arguments", refused("--calibrated", "--companion-to", "stress_plate_hole_s235"))
        done = run("--fitted-on", "12", "--compared-with", "4", "--records", "cultures", "--within", "6.1 percent rms", "--gap", "3.64 g/L against 2.15",
                   "--supersedes", "culture_trichoderma_ambr250")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("compared with 4 cultures not in the fit, within 6.1 percent rms", done.stdout)
        self.assertIn("src/spec_sheet.rs: its spec sheet", done.stdout)
        self.assertIn("src/succession.rs: culture_trichoderma_ambr250 superseded_by kit_flags_rod", done.stdout)
        # A reference calculation needs no gap.
        self.assertEqual(run("--reference-calculation", "--compared-with", "4", "--within", "2 percent").returncode, 0)

    def test_camel_case(self):
        self.assertEqual(new_model.camel("cell_2170_m80"), "Cell2170M80")


class TheForecast(unittest.TestCase):
    """forecast.py on a throwaway workspace: a gateway crate with Git, a
    protocol and a held-back file in an evidence/research folder."""

    def setUp(self):
        self.top = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.top, "evidence", "research", "thing"))
        self.protocol = os.path.join(self.top, "evidence", "research", "thing", "validation_protocol.md")
        self.held = os.path.join(self.top, "evidence", "research", "thing", "held_back.csv")
        with open(self.protocol, "w") as f:
            f.write("Pass marks: within 10 percent. Forecast: 3.2.\n")
        with open(self.held, "w") as f:
            f.write("t,x\n0,3.0\n")
        self.workspace = os.path.join(self.top, "genesis_workspace")
        repo_with(os.path.join(self.workspace, "protocol_mcp_gateway"), {"README.md": "gateway\n"})
        self.registry = forecast.Registry(self.workspace)

    def tearDown(self):
        shutil.rmtree(self.top)

    def run_kit(self, *args):
        return subprocess.run([sys.executable, os.path.join(HERE, "forecast.py"), "--root", self.workspace, *args], capture_output=True, text=True)

    def test_held_back_data_opens_only_after_the_protocol_is_committed(self):
        refused = self.run_kit("open", "thing", self.held)
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("no record", refused.stderr)
        done = self.run_kit("register", "thing", "--protocol", self.protocol, "--held-back", self.held)
        self.assertEqual(done.returncode, 0, done.stderr)
        record = self.registry.read("thing")
        self.assertEqual(record["protocol"]["path"], "evidence/research/thing/validation_protocol.md")
        self.assertEqual(record["protocol"]["sha256"], forecast.sha256_of(self.protocol))
        log = git(os.path.join(self.workspace, "protocol_mcp_gateway"), "log", "--format=%s")
        self.assertIn(record["protocol"]["sha256"], log)
        # A protocol is fixed once.
        self.assertNotEqual(self.run_kit("register", "thing", "--protocol", self.protocol).returncode, 0)
        # A file not registered is not opened.
        other = os.path.join(self.top, "evidence", "research", "thing", "other.csv")
        with open(other, "w") as f:
            f.write("x\n")
        self.assertIn("not a data file named in the forecast", self.run_kit("open", "thing", other).stderr)
        opened = self.run_kit("open", "thing", self.held)
        self.assertEqual(opened.returncode, 0, opened.stderr)
        self.assertEqual(opened.stdout.strip(), self.held)
        # Reading the data records the time and commits nothing; a second reading adds nothing.
        self.assertEqual(self.run_kit("open", "thing", self.held).returncode, 0)
        self.assertEqual(len(self.registry.read("thing")["opened"]), 1)
        commits = lambda: git(os.path.join(self.workspace, "protocol_mcp_gateway"), "log", "--format=%s", "--", "kit/forecasts/thing.json").splitlines()
        self.assertEqual(len(commits()), 1)
        output = os.path.join(self.top, "scored.json")
        with open(output, "w") as f:
            f.write("{}\n")
        scored = self.run_kit("score", "thing", "--output", output)
        self.assertEqual(scored.returncode, 0, scored.stdout + scored.stderr)
        self.assertEqual(len(commits()), 1)
        self.assertTrue(commits()[0].startswith("Forecast of thing: "), commits())
        self.assertEqual(self.run_kit("check", "thing").returncode, 0)
        self.assertEqual(forecast.holds(self.registry, "thing", self.registry.read("thing")), [])

    def test_a_protocol_changed_after_it_was_fixed_is_named(self):
        self.assertEqual(self.run_kit("register", "thing", "--protocol", self.protocol, "--held-back", self.held).returncode, 0)
        with open(self.protocol, "a") as f:
            f.write("Pass marks widened after a look.\n")
        refused = self.run_kit("open", "thing", self.held)
        self.assertIn("is not the one fingerprinted", refused.stderr)
        self.assertTrue(any("not the one fingerprinted" in why for why in forecast.holds(self.registry, "thing", self.registry.read("thing"))))
        self.assertEqual(self.run_kit("check", "thing").returncode, 1)

    def test_a_scoring_before_the_protocol_was_fixed_does_not_hold(self):
        record = {
            "model": "late",
            "protocol": {"path": "evidence/research/thing/validation_protocol.md", "sha256": forecast.sha256_of(self.protocol), "committed": None},
            "scored": {"utc": "2020-01-01T00:00:00Z"},
            "registered_by": "forecast.py register",
        }
        self.registry.write_and_commit("late", record, "a registration made after the scoring")
        wrong = forecast.holds(self.registry, "late", self.registry.read("late"))
        self.assertTrue(any("not before the scoring" in why for why in wrong), wrong)
        # A backfill that names no commit of the protocol: its own commit fixes nothing.
        record["registered_by"] = "backfill from the record"
        self.registry.write_and_commit("late", record, "a backfill")
        wrong = forecast.holds(self.registry, "late", self.registry.read("late"))
        self.assertTrue(any("names none" in why for why in wrong), wrong)
        record["registered_by"] = "forecast.py register"
        # The same day with no time cannot be ordered.
        record["scored"] = {"utc": forecast.now_utc()[:10]}
        self.registry.write_and_commit("late", record, "the same day")
        self.assertTrue(any("same day" in why for why in forecast.holds(self.registry, "late", self.registry.read("late"))))


# A stand-in for a built mcp_stdio: it serves the solvers and the
# descriptions named in a JSON file beside it, read again at each start, over
# the local connection's lines.
FAKE_DOOR = """#!{python} -I
import json, sys
with open({served!r}) as f:
    SERVED = json.load(f)
for line in sys.stdin:
    message = json.loads(line)
    if "id" not in message:
        continue
    result = {{"protocolVersion": "2025-06-18", "capabilities": {{}}, "serverInfo": {{"name": "stand-in", "version": "1"}}}}
    if message["method"] == "tools/call":
        name, arguments = message["params"]["name"], message["params"]["arguments"]
        if name == "list_cards":
            body = {{"count": 0, "cards": []}}
        elif name == "list_solvers":
            body = {{"count": len(SERVED), "solvers": [{{"solver": key, "against_measurement": "compared with measurement", "summary": "x"}} for key in SERVED]}}
        else:
            body = dict(SERVED[arguments["solver"]], solver=arguments["solver"])
        result = {{"content": [{{"type": "text", "text": json.dumps(body)}}], "isError": False}}
    print(json.dumps({{"jsonrpc": "2.0", "id": message["id"], "result": result}}), flush=True)
"""


class TheCheck(unittest.TestCase):
    """forecast.py check by the rule of the gateway's conformance sweep, on a
    throwaway workspace with a stand-in for the built engine: a forecast a
    served description states is held to its registration, and a served
    model that states none is held to nothing."""

    def setUp(self):
        self.top = tempfile.mkdtemp()
        self.folder = os.path.join(self.top, "evidence", "research", "thing")
        os.makedirs(self.folder)
        self.files = {}
        for name, text in (("forecast.md", "Forecast of the days not yet published: within 2 percent.\n"), ("fresh.csv", "t,x\n9,3.1\n"),
                           ("scored.json", "{}\n"), ("CALIBRATION.md", "Fitted on all 48 start-up readings.\n")):
            self.files[name] = os.path.join(self.folder, name)
            with open(self.files[name], "w") as f:
                f.write(text)
        self.workspace = os.path.join(self.top, "genesis_workspace")
        self.gateway = os.path.join(self.workspace, "protocol_mcp_gateway")
        repo_with(self.gateway, {"README.md": "gateway\n"})
        self.registry = forecast.Registry(self.workspace)
        self.served = os.path.join(self.top, "served.json")
        self.door = os.path.join(self.top, "mcp_stdio")
        with open(self.door, "w") as f:
            f.write(FAKE_DOOR.format(python=sys.executable, served=self.served))
        os.chmod(self.door, 0o755)
        self.serve({})

    def tearDown(self):
        shutil.rmtree(self.top)

    def serve(self, descriptions):
        with open(self.served, "w") as f:
            json.dump(descriptions, f)

    def run_kit(self, *args, root=None):
        return subprocess.run([sys.executable, os.path.join(HERE, "forecast.py"), "--root", root or self.workspace, *args], capture_output=True, text=True)

    def check(self, *models, root=None):
        return self.run_kit("check", *models, "--binary", self.door, root=root)

    def stating(self, sha256):
        return {"validation": {"tolerance": f"Forecast committed before the data was opened, as {sha256}. Within 2 percent."}}

    def forecast_of(self, model):
        self.assertEqual(self.run_kit("register", model, "--protocol", self.files["forecast.md"], "--data", self.files["fresh.csv"]).returncode, 0)
        self.assertEqual(self.run_kit("open", model, self.files["fresh.csv"]).returncode, 0)
        self.assertEqual(self.run_kit("score", model, "--output", self.files["scored.json"]).returncode, 0)
        return self.registry.read(model)["protocol"]["sha256"]

    def test_a_forecast_stated_is_held_to_its_registration(self):
        sha256 = self.forecast_of("seebeck")
        self.serve({"seebeck": self.stating(sha256)})
        held = self.check("seebeck")
        self.assertEqual(held.returncode, 0, held.stdout + held.stderr)
        self.assertEqual(held.stdout.strip(), f"seebeck: holds, the forecast it states ({sha256}) committed before its data")
        self.assertEqual([said for said in forecast.forecasts_said(self.stating(sha256))], [sha256])
        # The description states another forecast than the one registered.
        self.serve({"seebeck": self.stating("b" * 64)})
        self.assertIn(f"the description states the forecast {'b' * 64}, and kit/forecasts/seebeck.json registers {sha256}", self.check("seebeck").stdout)
        # A forecast stated with no registration.
        self.serve({"seebeck": self.stating(sha256), "unregistered": self.stating(sha256)})
        everything = self.check()
        self.assertEqual(everything.returncode, 1)
        self.assertIn("unregistered: does not hold: the description states a forecast, and kit/forecasts/unregistered.json is missing", everything.stdout)
        # The forecast changed on disk after it was committed.
        self.serve({"seebeck": self.stating(sha256)})
        with open(self.files["forecast.md"], "a") as f:
            f.write("Widened after a look.\n")
        changed = self.check("seebeck")
        self.assertEqual(changed.returncode, 1)
        self.assertIn("the protocol on disk, evidence/research/thing/forecast.md, is not the one fingerprinted", changed.stdout)

    def test_each_rule_a_stated_forecast_breaks_is_named(self):
        """Every rule check holds a forecast its description states to, each
        broken alone and named: the sweep's rules (forecast_findings)."""
        sha256 = self.forecast_of("fc")
        good = self.registry.read("fc")
        fixed = self.registry.fixed("fc", good)[0]

        class Stated:
            """What check reads of a build: the models served and the forecasts each states."""
            def __init__(self, said):
                self.said_by, self.why = said, None

            def models(self):
                return set(self.said_by)

            def said(self, model):
                return self.said_by.get(model)

        def named(words, change=None, said=None, model="fc"):
            record = json.loads(json.dumps(good))
            if change:
                change(record)
            self.registry.write("fc", record)
            try:
                reasons = forecast.model_holds(self.registry, model, Stated(said or {"fc": [sha256]}))[1]
            finally:
                self.registry.write("fc", good)
            self.assertTrue(any(words in reason for reason in reasons), f"{words!r} in {reasons}")
            return reasons

        self.assertEqual(forecast.model_holds(self.registry, "fc", Stated({"fc": [sha256]}))[1], [])
        # The description and the registry.
        named("the description states a forecast, and kit/forecasts/absent.json is missing", said={"absent": [sha256]}, model="absent")
        named('the description states a forecast as "abc", not a SHA-256', said={"fc": ["abc"]})
        named(f"the description states the forecast {'b' * 64}, and kit/forecasts/fc.json registers {sha256}", said={"fc": ["b" * 64]})
        with open(self.registry.file("broken"), "w") as f:
            f.write("{not json")
        named("kit/forecasts/broken.json is not JSON", said={"broken": [sha256]}, model="broken")
        # The forecast fixed in Git.
        named("the registration names no SHA-256 of the protocol", lambda r: r["protocol"].update(sha256=""))
        named("no commit of the registration carries the protocol's SHA-256", lambda r: r["protocol"].update(sha256="c" * 64))
        named("no commit carries the protocol's SHA-256 or the protocol: the record it was backfilled from names none", lambda r: r.update(registered_by="backfill from the record"))
        named("the commit " + "d" * 40 + " is not in protocol_mcp_gateway's Git", lambda r: r["protocol"].update(committed={"crate": "protocol_mcp_gateway", "commit": "d" * 40}))
        start = git(self.gateway, "rev-list", "--max-parents=0", "HEAD").strip()
        named(f"the commit {start} of protocol_mcp_gateway carries neither the protocol's SHA-256 nor the protocol", lambda r: r["protocol"].update(committed={"crate": "protocol_mcp_gateway", "commit": start}))
        self.registry.write("uncommitted", dict(good, model="uncommitted"))
        named("the registration is not committed", said={"uncommitted": [sha256]}, model="uncommitted")
        # The forecast unchanged on disk.
        with open(self.files["forecast.md"], "a") as f:
            f.write("Widened after a look.\n")
        named("the protocol on disk, evidence/research/thing/forecast.md, is not the one fingerprinted")
        with open(self.files["forecast.md"], "w") as f:
            f.write("Forecast of the days not yet published: within 2 percent.\n")
        self.assertEqual(forecast.model_holds(self.registry, "fc", Stated({"fc": [sha256]}))[1], [])
        # The data opened after the forecast was fixed, and before the scoring.
        named("evidence/research/thing/fresh.csv was opened at 2020-01-01T00:00:00Z, not after the protocol was fixed",
              lambda r: r["opened"][0].update(utc="2020-01-01T00:00:00Z"))
        named("evidence/research/thing/fresh.csv was opened at an hour, not after the protocol was fixed", lambda r: r["opened"][0].update(utc="an hour"))
        named(f"evidence/research/thing/fresh.csv was opened at 2999-01-01T00:00:00Z, after the scoring at {good['scored']['utc']}",
              lambda r: r["opened"][0].update(utc="2999-01-01T00:00:00Z"))
        # The scoring after the forecast was fixed, at a time that orders them.
        named("the scoring has no time", lambda r: r["scored"].pop("utc"))
        named("the scoring has no time", lambda r: r["scored"].update(utc="in the autumn"))
        named("the protocol was fixed and the model scored on the same day", lambda r: r["scored"].update(utc=fixed.strftime("%Y-%m-%d")))
        named("not before the scoring at 2020-01-01T00:00:00Z", lambda r: r["scored"].update(utc="2020-01-01T00:00:00Z"))
        # Each named by check too, which exits 1.
        self.serve({"fc": self.stating(sha256)})
        self.registry.write("fc", dict(good, opened=[dict(good["opened"][0], utc="2020-01-01T00:00:00Z")], scored=dict(good["scored"], utc="2020-01-01T00:00:00Z")))
        broken = self.check("fc")
        self.assertEqual(broken.returncode, 1)
        self.assertIn("fc: does not hold: evidence/research/thing/fresh.csv was opened at 2020-01-01T00:00:00Z, not after the protocol was fixed", broken.stdout)
        self.assertIn("not before the scoring at 2020-01-01T00:00:00Z", broken.stdout)

    def test_a_model_that_states_no_forecast_holds_whatever_its_record(self):
        # A calibration with no companion, as reactor_msre_pump's record: no original of it is served.
        record = {"model": "msre", "verdict": "calibrated to the record", "registered_by": "by hand", "calibration": {
            "companion_to": "", "protocol": {"path": "evidence/research/thing/CALIBRATION.md", "sha256": forecast.sha256_of(self.files["CALIBRATION.md"])},
            "scored": {"utc": "2026-10-18T19:27:06Z"}}}
        self.registry.write("msre", record)
        # A forecast whose times do not hold, the model served stating none.
        late = {"model": "late", "protocol": {"path": "evidence/research/thing/forecast.md", "sha256": forecast.sha256_of(self.files["forecast.md"]), "committed": None},
                "scored": {"utc": "2020-01-01T00:00:00Z"}, "registered_by": "forecast.py register"}
        self.registry.write_and_commit("late", late, "a registration made after the scoring")
        self.serve({"msre": {"summary": "Fitted on 48 start-up readings."}, "late": {"summary": "Compared with 4 runs not in the fit: within 2 percent."}})
        held = self.check()
        self.assertEqual(held.returncode, 0, held.stdout + held.stderr)
        self.assertEqual(held.stdout.splitlines(), ["late: holds, states no forecast", "msre: holds, states no forecast"])
        # Not served, a record's forecast is held to its times, and a calibration alone to nothing.
        self.serve({})
        unserved = self.check()
        self.assertEqual(unserved.returncode, 1)
        self.assertIn("late: does not hold: the protocol was fixed at", unserved.stdout)
        self.assertIn("msre: holds, carries no forecast", unserved.stdout)
        # With no build found by default, each record is held to its forecast's times, and the reason is said.
        default = self.run_kit("check", "msre")
        self.assertEqual((default.returncode, default.stdout.strip()), (0, "msre: holds, carries no forecast"))
        self.assertIn("what is served cannot be read: no mcp_stdio at", default.stderr)
        # A build named and not found is not read past.
        unread = self.run_kit("check", "msre", "--binary", os.path.join(self.top, "no_such_mcp_stdio"))
        self.assertEqual(unread.returncode, 1)
        self.assertIn("what is served cannot be read: no mcp_stdio at", unread.stdout)

    def test_the_msre_record_holds_in_this_workspace(self):
        self.serve({"reactor_msre_pump": {"summary": "Fitted on the steady loss and 48 start-up readings."}})
        held = self.check("reactor_msre_pump", root=WORKSPACE)
        self.assertEqual((held.returncode, held.stdout.strip()), (0, "reactor_msre_pump: holds, states no forecast"), held.stderr)
        record = forecast.Registry(WORKSPACE).read("reactor_msre_pump")
        self.assertEqual(record["calibration"]["companion_to"], "")

    def test_calibrate_and_the_scoring_word_are_retired(self):
        retired = self.run_kit("calibrate", "calib", "--companion-to", "orig")
        self.assertNotEqual(retired.returncode, 0)
        self.assertIn("invalid choice: 'calibrate'", retired.stderr)
        self.forecast_of("thing")
        self.assertIn("unrecognized arguments: --verdict", self.run_kit("score", "thing", "--output", self.files["scored.json"], "--verdict", "met").stderr)
        self.assertNotIn("verdict", self.registry.read("thing"))
        for gone in ("calibrate", "calibration_holds", "verdict", "CALIBRATED", "BLIND_WORDS", "NOT_A_BLIND_TEST"):
            self.assertFalse(hasattr(forecast, gone), gone)
        # A forecast registered beside a record that holds only a calibration: fixed by the commit that carries it.
        self.registry.write("both", {"model": "both", "calibration": {"companion_to": ""}})
        self.assertEqual(self.run_kit("register", "both", "--protocol", self.files["forecast.md"]).returncode, 0)
        record = self.registry.read("both")
        self.assertEqual(list(record)[:2], ["model", "calibration"])
        self.assertEqual(self.registry.fixed("both", record), (self.registry.committed("both")[0][1], None))
        self.assertIn("already registered", self.run_kit("register", "both", "--protocol", self.files["forecast.md"]).stderr)


class TheMerge(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp()
        self.integration = os.path.join(self.root, "integration")
        crate = os.path.join(self.integration, "genesis_kernel")
        repo_with(crate, {"lib.rs": "pub mod a;\n", "shared.rs": "one\n"}, branch="next-integration")
        self.copies = []
        for name, files in (("clean", {"new_card.rs": "card\n"}), ("clash", {"shared.rs": "two\n"}), ("idle", {})):
            copy = os.path.join(self.root, name)
            os.makedirs(copy)
            shutil.copytree(crate, os.path.join(copy, "genesis_kernel"))
            work = os.path.join(copy, "genesis_kernel")
            git(work, "checkout", "-q", "-b", f"wave1-{name}")
            if files:
                for file, text in files.items():
                    with open(os.path.join(work, file), "w") as f:
                        f.write(text)
                git(work, "add", "-A")
                git(work, "commit", "-q", "-m", name)
            self.copies.append(copy)
        # The integration moves on after the copies were taken.
        with open(os.path.join(crate, "shared.rs"), "w") as f:
            f.write("three\n")
        git(crate, "commit", "-q", "-am", "integration moves")

    def tearDown(self):
        shutil.rmtree(self.root)

    def run_tool(self, *extra):
        sys_argv = sys.argv
        sys.argv = ["merge_branches.py", self.integration, *self.copies, "--crates", "genesis_kernel", *extra]
        try:
            buffer = io.StringIO()
            with redirect_stdout(buffer):
                code = merge_branches.main()
        finally:
            sys.argv = sys_argv
        return code, buffer.getvalue()

    def test_check_says_what_a_merge_would_do_and_touches_nothing(self):
        crate = os.path.join(self.integration, "genesis_kernel")
        before = git(crate, "rev-parse", "HEAD")
        code, text = self.run_tool("--dry-run")
        self.assertEqual(code, 1)
        self.assertIn("would merge", text)
        self.assertIn("conflicts: shared.rs", text)
        self.assertIn("nothing new", text)
        self.assertEqual(git(crate, "rev-parse", "HEAD"), before)

    def test_a_clean_branch_merges_and_a_conflict_is_reported_and_undone(self):
        crate = os.path.join(self.integration, "genesis_kernel")
        code, text = self.run_tool()
        self.assertEqual(code, 1)
        self.assertIn("merged", text)
        self.assertIn("CONFLICT", text)
        self.assertTrue(os.path.exists(os.path.join(crate, "new_card.rs")))
        with open(os.path.join(crate, "shared.rs")) as f:
            self.assertEqual(f.read(), "three\n")
        self.assertEqual(git(crate, "status", "--porcelain", "--untracked-files=no"), "")

    def test_a_line_changed_the_same_way_in_both_is_stopped_one_short(self):
        # Two squads each add a card and raise the count from 40 to 41: Git
        # takes the identical change once, and the merge is clean at 41.
        root = os.path.join(self.root, "counts")
        crate = os.path.join(root, "integration", "genesis_kernel")
        os.makedirs(os.path.dirname(crate))
        repo_with(crate, {"citations.json": '{\n  "cards": 40,\n  "solvers": 25\n}\n', "lib.rs": "pub mod a;\n"}, branch="next-integration")
        copy = os.path.join(root, "squad")
        os.makedirs(copy)
        shutil.copytree(crate, os.path.join(copy, "genesis_kernel"))
        work = os.path.join(copy, "genesis_kernel")
        git(work, "checkout", "-q", "-b", "wave1-squad")
        for repo, card in ((crate, "b"), (work, "c")):
            with open(os.path.join(repo, "citations.json"), "w") as f:
                f.write('{\n  "cards": 41,\n  "solvers": 25\n}\n')
            with open(os.path.join(repo, f"{card}.rs"), "w") as f:
                f.write("card\n")
            git(repo, "add", "-A")
            git(repo, "commit", "-q", "-m", f"card {card}")
        before = git(crate, "rev-parse", "HEAD")
        self.assertEqual(merge_branches.one_short(crate, "HEAD", "HEAD", "HEAD^{tree}"), [])

        def run(*extra):
            sys_argv = sys.argv
            sys.argv = ["merge_branches.py", os.path.join(root, "integration"), copy, "--crates", "genesis_kernel", *extra]
            try:
                buffer = io.StringIO()
                with redirect_stdout(buffer):
                    code = merge_branches.main()
            finally:
                sys.argv = sys_argv
            return code, buffer.getvalue()
        for extra in (("--dry-run",), ()):
            code, text = run(*extra)
            self.assertEqual(code, 1, text)
            self.assertIn("ONE SHORT", text)
            self.assertIn('one short: citations.json: "cards": 41, (short by 1)', text)
            self.assertNotIn("lib.rs", text)
            self.assertEqual(git(crate, "rev-parse", "HEAD"), before)
        # Each found right once: merged on the word.
        code, text = run("--allow-identical")
        self.assertEqual(code, 0, text)
        self.assertIn("merged", text)
        self.assertNotEqual(git(crate, "rev-parse", "HEAD"), before)


class TheMeasure(unittest.TestCase):
    def answer(self, body):
        return json.dumps({"content": [{"type": "text", "text": json.dumps(body)}], "isError": False}).encode()

    def test_a_moved_seal_is_told_from_words_under_kept_seals(self):
        a = {"run_seal": "aa", "frames": [{"link": "1"}, {"link": "2"}], "seal_parts": {"result": "r"}, "final_state": {"x": 1.0}, "note": "old"}
        b = json.loads(json.dumps(a))
        b["frames"][1]["link"] = "3"
        b["run_seal"] = "bb"
        seals, others = before_after.differences(before_after.answer_value(self.answer(a)), before_after.answer_value(self.answer(b)))
        self.assertEqual(before_after.seal_names(seals), ["answer.frames[].link", "answer.run_seal"])
        self.assertEqual(others, [])
        c = json.loads(json.dumps(a))
        c["note"] = "new words"
        seals, others = before_after.differences(before_after.answer_value(self.answer(a)), before_after.answer_value(self.answer(c)))
        self.assertEqual((seals, others), ([], ["answer.note"]))

    def test_compare_reports_each_kind(self):
        with tempfile.TemporaryDirectory() as root:
            for side in ("A", "B"):
                os.makedirs(os.path.join(root, side))
            base = {"solution_seal": "s", "summary": "x"}
            files = {
                "same": (base, base),
                "sealed": (base, {"solution_seal": "t", "summary": "x"}),
                "words": (base, {"solution_seal": "s", "summary": "y"}),
            }
            for name, (left, right) in files.items():
                for side, body in (("A", left), ("B", right)):
                    with open(os.path.join(root, side, name + ".json"), "wb") as f:
                        f.write(self.answer(body))
            lines = []
            code = before_after.compare(os.path.join(root, "A"), os.path.join(root, "B"), lines.append)
            text = "\n".join(lines)
            self.assertEqual(code, 1)
            self.assertIn("1 identical byte for byte; 1 with a seal moved; 1 with bytes moved and every seal kept", text)
            self.assertIn("sealed: answer.solution_seal", text)
            self.assertIn("words: 1 values (first: answer.summary)", text)


if __name__ == "__main__":
    unittest.main()
