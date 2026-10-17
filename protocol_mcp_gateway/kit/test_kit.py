#!/usr/bin/env python3
"""Tests of the kit: python3 -I kit/test_kit.py

- the stamp finds every place it edits in this workspace (a dry run), fills
  every placeholder, and refuses to overwrite a file;
- the merge tool merges a clean branch, reports and undoes a conflicting one,
  says what a merge would do with --dry-run and touches nothing then, and
  reports a branch with nothing new;
- the measure tool tells a moved seal from bytes moved under kept seals;
- forecast first: held-back data opens only after the protocol is committed,
  and a protocol changed or a scoring before it is named;
- calibrated to the record: calibrate writes and commits the calibration,
  check holds it to its five rules and names each one broken, a pending blind
  registration beside it keeps forecast first, and the verdict is the blind
  scoring's once it is scored.
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

    def test_a_compared_model_is_not_stamped_the_company_own_specimen(self):
        for kind, key in (("card", "energy/kit_compared_body"), ("solver", "kit_compared_rod")):
            text = new_model.citation(key, kind, "A heated body", compared=True) + new_model.entry(key, "A heated body", compared=True)
            self.assertNotIn("own specimen", text, kind)
            self.assertNotIn("OWN_SPECIMEN", text, kind)
            self.assertIn("compared with measurements of it", text, kind)
            self.assertIn(f"kit/forecasts/{key.split('/')[-1]}.json", text, kind)
            json.loads("{" + new_model.citation(key, kind, "A heated body", compared=True).rstrip(",") + "}")
            own = new_model.citation(key, kind, "A heated body") + new_model.entry(key, "A heated body")
            self.assertIn("own specimen", own, kind)
        # A dry run of a compared stamp edits the same places and says what forecast first needs.
        stamp = new_model.Stamp(WORKSPACE, dry=True)
        new_model.stamp_solver(stamp, "kit_compared_rod", "A dry run", compared=True)
        args = type("Args", (), {"protocol": None, "held_back": []})
        new_model.compared_by_hand(stamp, "kit_compared_rod", args)
        self.assertTrue(any("forecast.py register kit_compared_rod" in line for line in stamp.by_hand), stamp.by_hand)
        self.assertNotIn("own specimen", stamp.pending["protocol_mcp_gateway/data/citations.json"].split('"kit_compared_rod"')[1].split("},")[0])

    def test_a_calibrated_model_is_stamped_with_the_sixth_word_beside_its_companion(self):
        for kind, name, companion in (("card", "kit_calibrated_body", "generator_mmrtg_f1"), ("solver", "kit_calibrated_rod", "stress_plate_hole_s235")):
            stamp = new_model.Stamp(WORKSPACE, dry=True)
            args = type("Args", (), {"companion_to": companion, "protocol": None, "output": None, "program": None, "not_blind": None})
            if kind == "card":
                new_model.stamp_card(stamp, name, "energy", "A dry run", calibrated=companion, args=args)
                described = stamp.pending[f"protocol_mcp_gateway/src/card_{name}_descriptor.rs"]
                key = f"energy/{name}"
            else:
                new_model.stamp_solver(stamp, name, "A dry run", calibrated=companion, args=args)
                described = stamp.pending[f"protocol_mcp_gateway/src/solver_{name}.rs"]
                key = name
            new_model.calibrated_by_hand(stamp, name, args)
            missed = [line for line in stamp.by_hand if "places hold the anchor" in line or line.endswith("+ 1")]
            self.assertEqual(missed, [], f"{kind}: anchors not found")
            self.assertFalse(any(line.startswith("edit") and "validation result" in line for line in stamp.by_hand), stamp.by_hand)
            self.assertTrue(any("the validation result" in line for line in stamp.done), stamp.done)
            # The sixth word and its row, naming the companion.
            found = stamp.pending["protocol_mcp_gateway/src/find_model.rs"]
            entry = found.split(f'Entry {{ key: "{key}",')[1].split("},")[0]
            self.assertIn("marks: Marks::Calibrated", entry)
            self.assertIn(f'Calibration {{ model: "{name}", companion_to: "{companion}", not_blind: "', found)
            # The citation names the companion's real object and its sources, and the companion.
            cited = json.loads(stamp.pending["protocol_mcp_gateway/data/citations.json"])["models"]
            own = cited[[k for k in cited if k.split("/")[-1] == companion][0]]
            self.assertEqual((cited[key]["real_thing"], cited[key]["sources"]), (own["real_thing"], own["sources"]))
            self.assertIn(companion, cited[key]["how_used"])
            self.assertIn("not a blind test", cited[key]["how_used"])
            self.assertFalse(cited[key]["not_held_to_a_measurement"])
            self.assertNotIn("own specimen", json.dumps(cited[key]))
            # The validation result opens by saying so.
            self.assertIn(new_model.rust_text(new_model.CALIBRATED_RESULT.format(companion=companion)), described)
            # No blind registration: the calibration is left to forecast.py calibrate.
            self.assertTrue(any(f"forecast.py calibrate {name} --companion-to {companion}" in line for line in stamp.by_hand), stamp.by_hand)
            self.assertFalse(any("forecast.py register" in line and "--protocol" in line and "would register" in line for line in stamp.done))
            self.assertFalse(os.path.exists(os.path.join(HERE, "forecasts", f"{name}.json")))
        # The words go together, or the stamp says why not.
        def refused(*extra):
            done = subprocess.run([sys.executable, os.path.join(HERE, "new_model.py"), "solver", "kit_calibrated_rod", "--dry-run", *extra], capture_output=True, text=True)
            self.assertNotEqual(done.returncode, 0, extra)
            return done.stderr
        self.assertIn("--companion-to", refused("--calibrated"))
        self.assertIn("--calibrated", refused("--companion-to", "stress_plate_hole_s235"))
        self.assertIn("two words", refused("--calibrated", "--companion-to", "stress_plate_hole_s235", "--compared"))
        self.assertIn("give all four", refused("--calibrated", "--companion-to", "stress_plate_hole_s235", "--protocol", "x"))
        self.assertIn("give --calibrated", refused("--not-blind", "x"))
        self.assertIn("no served model", refused("--calibrated", "--companion-to", "nothing_served"))
        done = subprocess.run([sys.executable, os.path.join(HERE, "new_model.py"), "solver", "kit_calibrated_rod", "--dry-run", "--calibrated", "--companion-to", "stress_plate_hole_s235"], capture_output=True, text=True)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("its row of CALIBRATIONS", done.stdout)

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
        self.assertIn("no registration", refused.stderr)
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
        self.assertIn("not a registered held-back file", self.run_kit("open", "thing", other).stderr)
        opened = self.run_kit("open", "thing", self.held)
        self.assertEqual(opened.returncode, 0, opened.stderr)
        self.assertEqual(opened.stdout.strip(), self.held)
        output = os.path.join(self.top, "scored.json")
        with open(output, "w") as f:
            f.write("{}\n")
        scored = self.run_kit("score", "thing", "--output", output)
        self.assertEqual(scored.returncode, 0, scored.stdout + scored.stderr)
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



# A stand-in for a built mcp_stdio: it serves the solvers and the words named
# in a JSON file beside it, read again at each start, over the local
# connection's lines.
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
            body = {{"count": len(SERVED), "solvers": [{{"solver": key, "against_measurement": row["word"], "summary": "x"}} for key, row in SERVED.items()]}}
        else:
            body = {{"solver": arguments["solver"], "validation": {{"result": SERVED[arguments["solver"]]["result"]}}}}
        result = {{"content": [{{"type": "text", "text": json.dumps(body)}}], "isError": False}}
    print(json.dumps({{"jsonrpc": "2.0", "id": message["id"], "result": result}}), flush=True)
"""


class Served:
    """What calibration_holds reads of a build, given in place of one."""

    def __init__(self, rows):
        self.rows = rows
        self.why = None

    def models(self):
        return set(self.rows)

    def word(self, model):
        return (self.rows.get(model) or {}).get("word")

    def result(self, model):
        return (self.rows.get(model) or {}).get("result")


class TheCalibration(unittest.TestCase):
    """forecast.py calibrate and check on a throwaway workspace: a model
    calibrated to the record, beside a model registered, opened and scored
    blind, with a stand-in for the built engine."""

    RESULT = "Calibrated to the record, not a blind test: the companion of orig, its constants fitted on the record orig was scored on."

    def setUp(self):
        self.top = tempfile.mkdtemp()
        self.folder = os.path.join(self.top, "evidence", "research", "thing")
        os.makedirs(self.folder)
        self.files = {}
        for name, text in (("validation_protocol.md", "Pass marks: within 10 percent. Forecast: 3.2.\n"), ("held_back.csv", "t,x\n0,3.0\n"),
                           ("comparison.json", '{"rms": 0.4}\n'), ("calibrated_out.json", '{"rms": 0.4, "worst": 0.9}\n'), ("calibrate.py", "print('fit')\n"),
                           ("blind_protocol.md", "Forecast of the days not yet read: within 2 percent.\n"), ("fresh.csv", "t,x\n9,3.1\n"),
                           ("blind_out.json", "{}\n"), ("score.py", "print('score')\n")):
            self.files[name] = os.path.join(self.folder, name)
            with open(self.files[name], "w") as f:
                f.write(text)
        self.workspace = os.path.join(self.top, "genesis_workspace")
        self.gateway = os.path.join(self.workspace, "protocol_mcp_gateway")
        repo_with(self.gateway, {"README.md": "gateway\n"})
        self.registry = forecast.Registry(self.workspace)
        # The model compared blind: registered, opened and scored.
        self.assertEqual(self.run_kit("register", "orig", "--protocol", self.files["validation_protocol.md"], "--held-back", self.files["held_back.csv"]).returncode, 0)
        self.assertEqual(self.run_kit("open", "orig", self.files["held_back.csv"]).returncode, 0)
        self.assertEqual(self.run_kit("score", "orig", "--output", self.files["blind_out.json"]).returncode, 0)
        self.served = os.path.join(self.top, "served.json")
        self.serve({"orig": {"word": "missed", "result": "Scored once: missed."}, "calib": {"word": forecast.CALIBRATED, "result": self.RESULT}})
        self.door = os.path.join(self.top, "mcp_stdio")
        with open(self.door, "w") as f:
            f.write(FAKE_DOOR.format(python=sys.executable, served=self.served))
        os.chmod(self.door, 0o755)

    def tearDown(self):
        shutil.rmtree(self.top)

    def serve(self, rows):
        with open(self.served, "w") as f:
            json.dump(rows, f)

    def rows(self):
        with open(self.served) as f:
            return json.load(f)

    def run_kit(self, *args):
        return subprocess.run([sys.executable, os.path.join(HERE, "forecast.py"), "--root", self.workspace, *args], capture_output=True, text=True)

    def calibrate(self, model="calib", companion="orig"):
        return self.run_kit("calibrate", model, "--companion-to", companion, "--protocol", self.files["comparison.json"],
                            "--output", self.files["calibrated_out.json"], "--program", self.files["calibrate.py"],
                            "--not-blind", "The constants were fitted on the record orig was scored on.")

    def check(self, *models):
        return self.run_kit("check", *models, "--binary", self.door)

    def register_blind(self, model="calib"):
        done = self.run_kit("register", model, "--protocol", self.files["blind_protocol.md"], "--held-back", self.files["fresh.csv"])
        self.assertEqual(done.returncode, 0, done.stderr)
        opened = self.run_kit("open", model, self.files["fresh.csv"])
        self.assertEqual(opened.returncode, 0, opened.stderr)

    def test_a_calibrated_record_holds(self):
        done = self.calibrate()
        self.assertEqual(done.returncode, 0, done.stderr)
        record = self.registry.read("calib")
        self.assertEqual(list(record), ["model", "verdict", "calibration"])
        self.assertEqual(record["verdict"], forecast.CALIBRATED)
        self.assertEqual(forecast.verdict(record), forecast.CALIBRATED)
        calibration = record["calibration"]
        self.assertEqual(list(calibration), ["companion_to", "protocol", "scored", "not_blind"])
        self.assertEqual(calibration["companion_to"], "orig")
        self.assertEqual(calibration["protocol"], {"path": "evidence/research/thing/comparison.json", "sha256": forecast.sha256_of(self.files["comparison.json"])})
        self.assertEqual(list(calibration["scored"]), ["utc", "output", "program"])
        self.assertEqual(calibration["scored"]["program"], {"path": "evidence/research/thing/calibrate.py", "sha256": forecast.sha256_of(self.files["calibrate.py"])})
        self.assertEqual(calibration["not_blind"], "The constants were fitted on the record orig was scored on.")
        # One commit, as every step of the registry is.
        log = git(self.gateway, "log", "--format=%s", "--", "kit/forecasts/calib.json").splitlines()
        self.assertEqual(len(log), 1)
        self.assertTrue(log[0].startswith("Calibrated to the record: calib, beside orig"), log)
        held = self.check("calib", "orig")
        self.assertEqual(held.returncode, 0, held.stdout + held.stderr)
        self.assertEqual(held.stdout.splitlines(), ["calib: holds, calibrated to the record", "orig: holds"])
        self.assertEqual(forecast.calibration_holds(self.registry, "calib", record, Served(self.rows())), [])
        # A calibration is recorded once, and stands beside a model scored blind.
        self.assertIn("already calibrated", self.calibrate().stderr)
        self.assertIn("has no scored record", self.calibrate("other", companion="calib").stderr)
        self.assertIn("not itself", self.calibrate("other", companion="other").stderr)
        # With no build to read, what is served is named as unread.
        unread = self.run_kit("check", "calib", "--binary", os.path.join(self.top, "no_such_mcp_stdio"))
        self.assertEqual(unread.returncode, 1)
        self.assertIn("what is served cannot be read: no mcp_stdio at", unread.stdout)

    def test_each_rule_broken_is_named(self):
        self.assertEqual(self.calibrate().returncode, 0)
        good = self.registry.read("calib")
        rows = self.rows()

        def named(change=None, served=None):
            record = json.loads(json.dumps(good))
            if change:
                change(record)
            return forecast.calibration_holds(self.registry, "calib", record, Served(served or rows))

        def says(words, reasons):
            self.assertTrue(any(words in reason for reason in reasons), f"{words!r} in {reasons}")

        self.assertEqual(named(), [])
        # 1. The companion: served, not itself, its own record scored.
        says("calibration.companion_to, nobody, is not a served model", named(lambda r: r["calibration"].update(companion_to="nobody")))
        says("calibration.companion_to, nobody: its own record, kit/forecasts/nobody.json, is not scored", named(lambda r: r["calibration"].update(companion_to="nobody")))
        says("calibration.companion_to names no served model beside it", named(lambda r: r["calibration"].update(companion_to="calib")))
        self.assertEqual(self.run_kit("register", "pending", "--protocol", self.files["blind_protocol.md"]).returncode, 0)
        says("calibration.companion_to, pending: its own record, kit/forecasts/pending.json, is not scored",
             named(lambda r: r["calibration"].update(companion_to="pending"), dict(rows, pending={"word": "missed", "result": ""})))
        # 2. The fingerprint of the record it is compared with.
        says("calibration.protocol.sha256 is not 64 hex characters", named(lambda r: r["calibration"]["protocol"].update(sha256="abc")))
        says("calibration.protocol, evidence/research/thing/gone.json, is not on this disk", named(lambda r: r["calibration"]["protocol"].update(path="evidence/research/thing/gone.json")))
        with open(self.files["comparison.json"], "a") as f:
            f.write("rms widened after a look\n")
        says("calibration.protocol on disk, evidence/research/thing/comparison.json, is not the one fingerprinted", named())
        broken = self.check("calib")
        self.assertEqual(broken.returncode, 1)
        self.assertIn("is not the one fingerprinted", broken.stdout)
        with open(self.files["comparison.json"], "w") as f:
            f.write('{"rms": 0.4}\n')
        self.assertEqual(named(), [])
        # 3. The comparison's scoring: its time, the output's and the program's SHA-256.
        says("calibration.scored gives no time of the comparison", named(lambda r: r["calibration"]["scored"].pop("utc")))
        says("calibration.scored gives no time of the comparison", named(lambda r: r["calibration"]["scored"].update(utc="in the autumn")))
        says("calibration.scored.output gives no SHA-256", named(lambda r: r["calibration"]["scored"].pop("output")))
        says("calibration.scored.program gives no SHA-256", named(lambda r: r["calibration"]["scored"]["program"].update(sha256="")))
        # 4. The served word, and the served description.
        says("the served word is 'missed', not 'calibrated to the record'", named(served=dict(rows, calib={"word": "missed", "result": self.RESULT})))
        says("the served description's validation result does not say it is not a blind test",
             named(served=dict(rows, calib={"word": forecast.CALIBRATED, "result": "Calibrated to the record."})))
        says("the served word: calib is not served", named(served={"orig": rows["orig"]}))
        says("gives the verdict met, and no blind registration of it is scored", named(lambda r: r.update(verdict="met")))

    def test_a_calibration_and_a_pending_blind_registration_both_hold(self):
        # Registered blind first, a held-back file opened, then calibrated.
        self.register_blind("calib")
        self.assertEqual(self.calibrate().returncode, 0)
        record = self.registry.read("calib")
        self.assertEqual(forecast.verdict(record), forecast.CALIBRATED)
        held = self.check("calib")
        self.assertEqual(held.returncode, 0, held.stdout + held.stderr)
        self.assertEqual(held.stdout.strip(), "calib: holds, calibrated to the record")
        # Calibrated first, then registered blind: the protocol was fixed by the
        # registration's commit, not by the calibration's before it.
        self.assertEqual(self.calibrate("calib2").returncode, 0)
        self.register_blind("calib2")
        commits = self.registry.committed("calib2")
        record = self.registry.read("calib2")
        self.assertEqual(list(record)[:3], ["model", "verdict", "calibration"])
        self.assertEqual(self.registry.fixed("calib2", record), (commits[1][1], None))
        self.assertLess(commits[0][1], commits[1][1])
        self.serve(dict(self.rows(), calib2={"word": forecast.CALIBRATED, "result": self.RESULT}))
        held = self.check("calib", "calib2")
        self.assertEqual(held.returncode, 0, held.stdout + held.stderr)
        # A protocol fixed once, beside a calibration as anywhere (rule 5).
        self.assertIn("already registered", self.run_kit("register", "calib2", "--protocol", self.files["blind_protocol.md"]).stderr)
        with open(self.files["blind_protocol.md"], "a") as f:
            f.write("Marks widened after a look.\n")
        broken = self.check("calib")
        self.assertEqual(broken.returncode, 1)
        self.assertIn("the protocol on disk, evidence/research/thing/blind_protocol.md, is not the one fingerprinted", broken.stdout)
        self.assertEqual(forecast.calibration_holds(self.registry, "calib", self.registry.read("calib"), Served(self.rows())),
                         forecast.holds(self.registry, "calib", self.registry.read("calib")))

    def test_the_verdict_changes_when_the_blind_registration_is_scored(self):
        self.register_blind("calib")
        self.assertEqual(self.calibrate().returncode, 0)
        self.assertEqual(forecast.verdict(self.registry.read("calib")), forecast.CALIBRATED)
        self.assertEqual(self.check("calib").returncode, 0)
        # The scoring names its word, since the word changes with it.
        refused = self.run_kit("score", "calib", "--output", self.files["blind_out.json"], "--program", self.files["score.py"])
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("give --verdict", refused.stderr)
        self.assertIn("not 'calibrated'", self.run_kit("score", "calib", "--output", self.files["blind_out.json"], "--verdict", "calibrated").stderr)
        scored = self.run_kit("score", "calib", "--output", self.files["blind_out.json"], "--program", self.files["score.py"], "--verdict", "met")
        self.assertEqual(scored.returncode, 0, scored.stdout + scored.stderr)
        record = self.registry.read("calib")
        self.assertEqual((record["verdict"], forecast.verdict(record)), ("met", "met"))
        self.assertEqual(record["calibration"]["companion_to"], "orig")
        # Served still under the sixth word, it does not hold; served as the scoring's, it does.
        stale = self.check("calib")
        self.assertEqual(stale.returncode, 1)
        self.assertIn("the served word is 'calibrated to the record', not 'met'", stale.stdout)
        self.serve(dict(self.rows(), calib={"word": "met", "result": "Scored blind once: met."}))
        held = self.check("calib")
        self.assertEqual(held.returncode, 0, held.stdout + held.stderr)
        self.assertEqual(held.stdout.strip(), "calib: holds, met")
        # A model scored blind is not calibrated after it.
        self.assertIn("its blind registration is scored", self.run_kit("calibrate", "orig", "--companion-to", "calib", "--protocol", self.files["comparison.json"],
                                                                         "--output", self.files["calibrated_out.json"], "--program", self.files["calibrate.py"],
                                                                         "--not-blind", "x").stderr)
        # A blind record keeps its scoring as before: no word asked, none written.
        self.assertNotIn("verdict", self.registry.read("orig"))

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
