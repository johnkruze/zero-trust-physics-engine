//! The engine version named outside the long forms (the red team of 0.13.0,
//! D4), against the answers the served engine 0.12.0 gave
//! (`tests/validation_0120/answers.jsonl`): `verify_receipt` says the
//! answer's `engine_version` beside `matches`, and whether it is this
//! engine's; the short form of `verify_dataset`, given the dataset seal of a
//! plan of solves 0.12.0 made, says the seal covers `engine_version` and
//! names both versions. Words only: no seal moves. The plan of solves
//! 0.12.0 solved, neon, moved by canonical math too (genesis_solvers 0.12.0):
//! a plan this engine solves, sealed under 0.12.0's names, stands for it.

mod conformance_support;

use std::path::Path;

use conformance_support::Door;
use serde_json::{Value, json};

fn served_0120_answers() -> Vec<Value> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/validation_0120/answers.jsonl");
    std::fs::read_to_string(file).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect()
}

fn answer(door: &mut Door, tool: &str, arguments: &Value) -> Value {
    let (error, text) = door.call(tool, arguments);
    assert!(!error, "{text}");
    serde_json::from_str(&text).unwrap()
}

#[test]
fn a_receipt_names_the_answers_engine_version_and_whether_it_is_this_engines() {
    let mut door = Door::start();
    for saved in served_0120_answers() {
        let report = answer(&mut door, "verify_receipt", &json!({ "response_text": saved["answer"] }));
        assert_eq!(report["matches"], json!(true), "{report}");
        assert_eq!(report["engine_version"], "genesis_kernel_v0.11.0", "{report}");
        assert_eq!(report["engine_version_is_this_engines"], json!(false), "{report}");
        let keys: Vec<&String> = report.as_object().unwrap().keys().collect();
        assert_eq!(&keys[..3], &["matches", "engine_version", "engine_version_is_this_engines"]);
    }
    for (tool, request) in [
        ("solve", json!({ "solver": "atom_dft", "parameters": { "atomic_number": 3 } })),
        ("run_trajectory", json!({ "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 0.1, "segments": [{ "steps": 10, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }], "record_every": 1 })),
    ] {
        let (_, text) = door.call(tool, &request);
        let report = answer(&mut door, "verify_receipt", &json!({ "response_text": text }));
        assert_eq!(report["engine_version_is_this_engines"], json!(true), "{report}");
    }
    // A text that names no version says nothing of one.
    let report = answer(&mut door, "verify_receipt", &json!({ "response_text": "{\"a\":1,\"receipt_sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\"}" }));
    assert!(report.get("engine_version").is_none() && report.get("engine_version_is_this_engines").is_none(), "{report}");
}

#[test]
fn the_short_form_of_a_dataset_another_version_made_names_both_versions() {
    let mut door = Door::start();
    let saved = served_0120_answers().into_iter().find(|saved| saved["label"] == "a plan of solves").unwrap();
    let answered: Value = serde_json::from_str(saved["answer"].as_str().unwrap()).unwrap();
    let solves = answered["inputs"]["solves"].clone();
    // The plan 0.12.0 solved, neon, moved by canonical math as well as by
    // the version (genesis_solvers 0.12.0 computes with genesis_kernel::math):
    // its seal is not this plan's under another version's name, and the
    // short form names this engine's versions alone.
    let report = answer(&mut door, "verify_dataset", &json!({ "request": solves, "dataset_seal": answered["dataset_seal"] }));
    assert_eq!(report["holds"], json!(false));
    let said = report["not_held"].as_str().unwrap();
    assert!(said.ends_with("The dataset_seal covers engine_version, through its counts (genesis_kernel_v0.12.0 on this engine) and each row's solution_seal (genesis_solvers_v0.12.0): the seal of a plan another engine version solved does not hold here."), "{said}");
    // This engine's plan sealed under 0.12.0's names, genesis_kernel_v0.11.0
    // in its counts and genesis_solvers_v0.10.0 in its row's solution seal
    // (`genesis_seal::dataset::dataset_seal`), is named with both versions.
    let mut full = solves.clone();
    full["answer"] = json!("full");
    let made = answer(&mut door, "run_dataset", &full);
    let head = &made["head"];
    let rows = made["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    let solved = answer(&mut door, "solve", &rows[0]["request"]);
    let mut leaf = rows[0]["leaf"].clone();
    leaf["solution_seal"] = json!(solution_seal_at(&solved, "genesis_solvers_v0.10.0"));
    let root = hex::encode(genesis_seal::tree::leaf_hash(serde_json::to_string(&leaf).unwrap().as_bytes()));
    let counts = genesis_seal::dataset::Counts {
        engine_version: "genesis_kernel_v0.11.0".to_string(),
        rows: 1,
        completed: head["completed"].as_u64().unwrap(),
        ended: 0,
        refused: head["refused"].as_u64().unwrap(),
        unadmitted: head["unadmitted"].as_u64().unwrap(),
        frames: 0,
    };
    let theirs = genesis_seal::dataset::dataset_seal(head["plan_sha256"].as_str().unwrap(), &root, &counts);
    let report = answer(&mut door, "verify_dataset", &json!({ "request": solves, "dataset_seal": theirs }));
    assert_eq!(report["holds"], json!(false));
    let said = report["not_held"].as_str().unwrap();
    assert!(said.contains("The dataset_seal covers engine_version, through its counts and each row's solution_seal: the seal given is this plan's dataset seal with engine_version genesis_kernel_v0.11.0 in its counts and genesis_solvers_v0.10.0 in its solution seals, another engine version's, and this engine writes genesis_kernel_v0.12.0 and genesis_solvers_v0.12.0, so it does not hold here."), "{said}");
    assert!(said.len() < 2_000, "{}", said.len());
    // A seal of no version names only this engine's.
    let report = answer(&mut door, "verify_dataset", &json!({ "request": solves, "dataset_seal": "0".repeat(64) }));
    let said = report["not_held"].as_str().unwrap();
    assert!(said.ends_with("The dataset_seal covers engine_version, through its counts (genesis_kernel_v0.12.0 on this engine) and each row's solution_seal (genesis_solvers_v0.12.0): the seal of a plan another engine version solved does not hold here."), "{said}");
    // The engine's own seal holds, as before.
    let (_, made) = door.call("run_dataset", &solves);
    let made: Value = serde_json::from_str(&made).unwrap();
    let report = answer(&mut door, "verify_dataset", &json!({ "request": solves, "dataset_seal": made["head"]["dataset_seal"] }));
    assert_eq!(report["holds"], json!(true), "{report}");
    assert!(report.get("not_held").is_none());
}

/// A hex value of a solution as its seal's line writes it: 16 digits, or an
/// array in brackets, item by item.
fn hex_line(value: &Value) -> String {
    match value {
        Value::Array(items) => format!("[{}]", items.iter().map(hex_line).collect::<Vec<_>>().join(",")),
        other => other.as_str().unwrap().to_string(),
    }
}

/// The solution seal of a solve's answer at `version`, by the rule of
/// REFERENCE.md, "A solution".
fn solution_seal_at(answer: &Value, version: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut text = genesis_seal::sealed_form::sealed_form(&answer["inputs"]);
    text.push_str(&format!("engine_version:{version}\n"));
    for section in ["invariants", "result"] {
        if let Some(values) = answer[section].as_object() {
            let mut keys: Vec<&String> = values.keys().filter(|key| key.ends_with("_hex")).collect();
            keys.sort();
            for key in keys {
                text.push_str(&format!("{section}.{key}:{}\n", hex_line(&values[key])));
            }
        }
    }
    text.push_str(answer["field_sha256"].as_str().unwrap_or("null"));
    text.push_str(&format!("{{\"outcome\":{},\"iterations\":{},\"reject\":{}}}", answer["outcome"], answer["iterations"], answer["reject"]));
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// A plan of runs: its dataset seal covers the version through its counts
/// alone, so the seal another version gave is the same plan's root and
/// counts under that version's name (`genesis_seal::dataset::dataset_seal`).
#[test]
fn the_short_form_of_a_plan_of_runs_another_version_made_names_both_versions() {
    use sha2::{Digest, Sha256};
    let mut door = Door::start();
    let plan = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "list", "runs": [{ "initial": { "temperature_k": 293.15 }, "dt_s": 0.1, "segments": [{ "steps": 10, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }], "record_every": 1 }] },
        "store": "none", "return": "inline"
    });
    let made = answer(&mut door, "run_dataset", &plan);
    let head = &made["head"];
    let counts = |version: &str| {
        format!(
            "{{\"engine_version\":\"{version}\",\"rows\":{},\"completed\":{},\"ended\":{},\"refused\":{},\"unadmitted\":{},\"frames\":{}}}",
            head["rows"], head["completed"], head["ended"], head["refused"], head["unadmitted"], head["frames"]
        )
    };
    let seal = |version: &str| {
        let mut hasher = Sha256::new();
        hasher.update(head["plan_sha256"].as_str().unwrap());
        hasher.update(head["merkle_root"].as_str().unwrap());
        hasher.update(counts(version));
        format!("{:x}", hasher.finalize())
    };
    assert_eq!(seal("genesis_kernel_v0.12.0"), head["dataset_seal"].as_str().unwrap());
    let report = answer(&mut door, "verify_dataset", &json!({ "request": plan, "dataset_seal": seal("genesis_kernel_v0.11.0") }));
    assert_eq!(report["holds"], json!(false));
    let said = report["not_held"].as_str().unwrap();
    assert!(said.ends_with("The dataset_seal covers engine_version through its counts: the seal given is this plan's dataset seal with engine_version genesis_kernel_v0.11.0, another engine version's, and this engine writes genesis_kernel_v0.12.0, so it does not hold here."), "{said}");
}
