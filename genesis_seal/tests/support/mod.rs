//! What the tests share: the saved engine runs, a run made with no engine,
//! and small edits to a tape, a rows file and a head.

#![allow(dead_code)]

use std::io::Write;
use std::path::PathBuf;

use genesis_seal::dataset::{Assembled, RowResult, assemble};
use genesis_seal::plan::DatasetRequest;
use genesis_seal::run::{FRAME_LEN, link_0, run_seal, seal_frame, state_digest};
use serde_json::{Value, json};

/// The environment variable that names the folder of saved runs.
pub const RUNS_VARIABLE: &str = "ZERO_TRUST_PHYSICS_RUNS";

/// The folder of the saved runs the engine made, when `ZERO_TRUST_PHYSICS_RUNS`
/// names one. The runs are read where they stand and are never written. They
/// are not part of this crate, so a test that reads them starts with
/// `needs_saved_runs!()`.
pub fn saved_runs() -> Option<PathBuf> {
    let folder = PathBuf::from(std::env::var_os(RUNS_VARIABLE)?);
    folder.is_dir().then_some(folder)
}

/// The folder of saved runs, for a test that has found it present. A test
/// fails by name when a run is missing from it.
pub fn runs() -> PathBuf {
    saved_runs().unwrap_or_else(|| panic!("{RUNS_VARIABLE} names no folder of saved runs"))
}

/// The one line a test prints when the saved runs are absent. It goes
/// straight to standard error in one write, so it is seen whole while test
/// output is captured.
pub fn say_runs_are_needed(test: &str) {
    let test = test.strip_suffix("::here").unwrap_or(test);
    let line = format!("skipped: {test} needs the saved runs; set {RUNS_VARIABLE} to their folder\n");
    let _ = std::io::stderr().write_all(line.as_bytes());
}

/// The first line of a test that reads the saved runs. Where
/// `ZERO_TRUST_PHYSICS_RUNS` is unset or names no folder, the test prints one
/// line and returns.
macro_rules! needs_saved_runs {
    () => {
        if $crate::support::saved_runs().is_none() {
            fn here() {}
            $crate::support::say_runs_are_needed(std::any::type_name_of_val(&here));
            return;
        }
    };
}
pub(crate) use needs_saved_runs;

pub const CAPSULE: &str = "capsule_stardust_validation";
pub const GENERATOR: &str = "generator_mmrtg_f1_validation";
pub const SATELLITE: &str = "satellite_lageos1_validation";

/// The capsule's entry, 2,800 steps, completed.
pub const CAPSULE_2800: &str = "capsule_stardust_validation/sealed_entry_dt0p05_2800_steps.json";
/// A run that ends in a sealed refusal after 2,865 landed steps.
pub const CAPSULE_REFUSED: &str =
    "capsule_stardust_validation/sealed_entry_dt0p05_to_card_floor.json";

pub fn read_bytes(relative: &str) -> Vec<u8> {
    let path = runs().join(relative);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{} was not read: {error}", path.display()))
}

pub fn read_tape(relative: &str) -> Value {
    serde_json::from_slice(&read_bytes(relative))
        .unwrap_or_else(|error| panic!("{relative} is not JSON: {error}"))
}

/// Every trajectory file in one folder of saved runs, in name order. A
/// trajectory file is a JSON object that carries `frames` and `run_seal`.
pub fn tapes_in(folder: &str) -> Vec<(String, Vec<u8>, Value)> {
    let directory = runs().join(folder);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("{} was not read: {error}", directory.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .collect();
    paths.sort();
    let mut tapes = Vec::new();
    for path in paths {
        let name = format!(
            "{folder}/{}",
            path.file_name().expect("a file name").to_string_lossy()
        );
        let bytes = read_bytes(&name);
        let value: Value = serde_json::from_slice(&bytes).expect("a saved file is JSON");
        if value.get("frames").is_some() && value.get("run_seal").is_some() {
            tapes.push((name, bytes, value));
        }
    }
    tapes
}

/// The eight shortest saved runs: one revolution of the satellite each, 113
/// recorded frames, all on one card.
pub fn eight_short_runs() -> Vec<Value> {
    let tapes: Vec<Value> = tapes_in(SATELLITE)
        .into_iter()
        .filter(|(name, _, _)| name.ends_with("_d6_1356_steps.json"))
        .map(|(_, _, tape)| tape)
        .collect();
    assert_eq!(tapes.len(), 8);
    tapes
}

/// A run with no engine behind it. The frames hold made-up bytes and the
/// state holds made-up numbers. Every seal is built by the published rules,
/// so the tape is self-consistent.
pub fn fabricated_run(request: Value, steps: u64, record_every: u64, mark: u8) -> Value {
    let mut link = link_0(&request);
    let mut frames = Vec::new();
    for step in 1..=steps {
        if step % record_every != 0 && step != steps {
            continue;
        }
        let mut frame = [mark; FRAME_LEN];
        frame[0..8].copy_from_slice(&step.to_le_bytes());
        link = seal_frame(&link, &mut frame);
        frames.push(json!({ "step": step, "frame_hex": hex::encode(frame), "link": hex::encode(link) }));
    }
    let final_state = json!({
        "temperature_k": 1000000.0,
        "temperature_k_hex": "412e848000000000",
        "derated": false
    });
    let residual_hex = "0000000000000000";
    let digest = state_digest(final_state.as_object().unwrap(), residual_hex, residual_hex).unwrap();
    let seal = run_seal(&link, "completed", steps, None, Some(&digest));
    json!({
        "inputs": request,
        "engine_version": "no_engine",
        "outcome": "completed",
        "steps_requested": steps,
        "steps_landed": steps,
        "reject": null,
        "reject_detail": null,
        "final_state": final_state,
        "final_residual_conservation_error": 0.0,
        "final_residual_conservation_error_hex": residual_hex,
        "max_residual_conservation_error": 0.0,
        "max_residual_conservation_error_hex": residual_hex,
        "state_digest": digest,
        "frames": frames,
        "run_seal": seal
    })
}

/// A run request without its pair, as a `list` plan carries it.
pub fn without_pair(inputs: &Value) -> Value {
    let mut run = inputs.as_object().expect("a request is an object").clone();
    run.shift_remove("domain");
    run.shift_remove("card");
    Value::Object(run)
}

/// The `list` request whose rows are the requests of `tapes`, in order.
pub fn list_request(tapes: &[Value], store: &str, returns: &str) -> DatasetRequest {
    let runs: Vec<Value> = tapes.iter().map(|tape| without_pair(&tape["inputs"])).collect();
    DatasetRequest::from_value(json!({
        "domain": tapes[0]["inputs"]["domain"],
        "card": tapes[0]["inputs"]["card"],
        "plan": { "kind": "list", "runs": runs },
        "store": store,
        "return": returns
    }))
    .unwrap_or_else(|refusal| panic!("the list plan was refused: {refusal}"))
}

/// The files of a dataset.
pub struct Files {
    pub head: Vec<u8>,
    pub rows: Vec<u8>,
    pub frames: Option<Vec<u8>>,
    pub assembled: Assembled,
}

pub fn files_of(assembled: Assembled) -> Files {
    Files {
        head: assembled.head.wire().into_bytes(),
        rows: assembled.rows_text(),
        frames: assembled.frames_bytes(),
        assembled,
    }
}

/// The dataset whose rows are `tapes`, sealed and written to bytes.
pub fn dataset_of(tapes: &[Value], store: &str) -> Files {
    let request = list_request(tapes, store, "file");
    let results: Vec<RowResult> = tapes.iter().cloned().map(RowResult::Run).collect();
    let engine_version = tapes[0]["engine_version"].as_str().expect("an engine version");
    files_of(
        assemble(&request, engine_version, &results)
            .unwrap_or_else(|refusal| panic!("the dataset was refused: {refusal}")),
    )
}

/// The rows file with one row edited as JSON.
pub fn edit_row(rows: &[u8], row: usize, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut lines: Vec<Vec<u8>> = rows
        .strip_suffix(b"\n")
        .expect("a rows file ends with a line break")
        .split(|byte| *byte == b'\n')
        .map(<[u8]>::to_vec)
        .collect();
    let mut value: Value = serde_json::from_slice(&lines[row]).expect("a row is JSON");
    edit(&mut value);
    lines[row] = serde_json::to_vec(&value).unwrap();
    join_lines(&lines)
}

pub fn split_lines(rows: &[u8]) -> Vec<Vec<u8>> {
    rows.strip_suffix(b"\n")
        .expect("a rows file ends with a line break")
        .split(|byte| *byte == b'\n')
        .map(<[u8]>::to_vec)
        .collect()
}

pub fn join_lines(lines: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in lines {
        bytes.extend_from_slice(line);
        bytes.push(b'\n');
    }
    bytes
}

/// The head with one field edited as JSON. The receipt is left as it was,
/// so it no longer matches. The seals are what these tests look at.
pub fn edit_head(head: &[u8], edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(head).expect("a head is JSON");
    edit(&mut value);
    serde_json::to_vec(&value).unwrap()
}

/// One hex digit changed to another hex digit.
pub fn flip_hex_digit(text: &str, at: usize) -> String {
    let mut bytes = text.as_bytes().to_vec();
    bytes[at] = if bytes[at] == b'0' { b'1' } else { b'0' };
    String::from_utf8(bytes).unwrap()
}
