//! What the tests share: datasets made with the sealing crate from the saved
//! engine runs (read where they stand, never written), the program, and
//! readers for the Parquet files.

#![allow(dead_code)]

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use arrow_array::RecordBatch;
use genesis_seal::dataset::{Assembled, Head, RowResult, assemble};
use genesis_seal::plan::DatasetRequest;
use genesis_seal::run::{FRAME_LEN, link_0, run_seal, seal_frame, state_digest};
use genesis_seal::sealed_form::digest_hex;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
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
pub const SATELLITE: &str = "satellite_lageos1_validation";
pub const VERSION: &str = "genesis_kernel_v0.1.0";

pub fn read_json(relative: &str) -> Value {
    let path = runs().join(relative);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("{} was not read: {error}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{relative} is not JSON: {e}"))
}

pub fn tapes_in(folder: &str) -> Vec<(String, Value)> {
    let directory = runs().join(folder);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("{} was not read: {error}", directory.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let mut tapes = Vec::new();
    for path in paths {
        let name = format!("{folder}/{}", path.file_name().unwrap().to_string_lossy());
        let value = read_json(&name);
        if value.get("frames").is_some() && value.get("run_seal").is_some() {
            tapes.push((name, value));
        }
    }
    tapes
}

/// Eight satellite runs of 113 frames each: 3-vectors and a flag in the state.
pub fn eight_short_runs() -> Vec<Value> {
    let tapes: Vec<Value> = tapes_in(SATELLITE)
        .into_iter()
        .filter(|(name, _)| name.ends_with("_d6_1356_steps.json"))
        .map(|(_, tape)| tape)
        .collect();
    assert_eq!(tapes.len(), 8);
    tapes
}

/// The three capsule runs (two complete, one refused), which have flags.
pub fn capsule_runs() -> Vec<Value> {
    let tapes: Vec<Value> = tapes_in(CAPSULE).into_iter().map(|(_, t)| t).collect();
    assert_eq!(tapes[2]["outcome"], json!("refused"));
    tapes
}

pub fn without_pair(inputs: &Value) -> Value {
    let mut run = inputs.as_object().unwrap().clone();
    run.shift_remove("domain");
    run.shift_remove("card");
    Value::Object(run)
}

pub fn list_request(tapes: &[Value], store: &str) -> DatasetRequest {
    let runs: Vec<Value> = tapes.iter().map(|t| without_pair(&t["inputs"])).collect();
    DatasetRequest::from_value(json!({
        "domain": tapes[0]["inputs"]["domain"],
        "card": tapes[0]["inputs"]["card"],
        "plan": { "kind": "list", "runs": runs },
        "store": store,
        "return": "file"
    }))
    .unwrap_or_else(|r| panic!("the list plan was refused: {r}"))
}

/// A run with no engine behind it: made-up bytes, every seal by the rules.
/// `state` is the final state; its keys decide the schema.
pub fn fabricated_run(request: Value, steps: u64, mark: u8, state: Value) -> Value {
    let mut link = link_0(&request);
    let mut frames = Vec::new();
    for step in 1..=steps {
        let mut frame = [mark; FRAME_LEN];
        frame[0..8].copy_from_slice(&step.to_le_bytes());
        link = seal_frame(&link, &mut frame);
        frames.push(json!({ "step": step, "frame_hex": hex::encode(frame), "link": hex::encode(link) }));
    }
    let residual_hex = "0000000000000000";
    let digest = state_digest(state.as_object().unwrap(), residual_hex, residual_hex).unwrap();
    let seal = run_seal(&link, "completed", steps, None, Some(&digest));
    json!({
        "inputs": request,
        "engine_version": "no_engine",
        "outcome": "completed",
        "steps_requested": steps,
        "steps_landed": steps,
        "reject": null,
        "reject_detail": null,
        "final_state": state,
        "final_residual_conservation_error": 0.0,
        "final_residual_conservation_error_hex": residual_hex,
        "max_residual_conservation_error": 0.0,
        "max_residual_conservation_error_hex": residual_hex,
        "state_digest": digest,
        "frames": frames,
        "run_seal": seal
    })
}

/// A made-up run that stopped at an end its card declares after `landed` of
/// `asked` steps, sealed with the outcome `ended` and the end's name.
pub fn fabricated_ended_run(request: Value, asked: u64, landed: u64, mark: u8, state: Value, end: &str) -> Value {
    let mut run = fabricated_run(request, landed, mark, state);
    let last = run["frames"].as_array().unwrap().last().unwrap()["link"].as_str().unwrap().to_string();
    let mut link = [0_u8; 32];
    link.copy_from_slice(&hex::decode(last).unwrap());
    let digest = run["state_digest"].as_str().unwrap().to_string();
    run["outcome"] = json!("ended");
    run["steps_requested"] = json!(asked);
    run["reject"] = json!(end);
    run["reject_detail"] = json!(format!("Ended: the run reached the end the card declares ({end})."));
    run["run_seal"] = json!(run_seal(&link, "ended", landed, Some(end), Some(&digest)));
    run
}

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

pub fn dataset_of(tapes: &[Value], store: &str) -> Files {
    let request = list_request(tapes, store);
    let results: Vec<RowResult> = tapes.iter().cloned().map(RowResult::Run).collect();
    files_of(assemble(&request, VERSION, &results).unwrap_or_else(|r| panic!("refused: {r}")))
}

/// The capsule's three runs and a fourth request the door refuses: the
/// completed, refused and unadmitted rows in one dataset.
pub fn capsule_with_unadmitted(store: &str) -> Files {
    let mut tapes = capsule_runs();
    let mut below_ground = tapes[0]["inputs"].clone();
    below_ground["initial"]["radius_m"] = json!(6000000.0);
    tapes.push(json!({ "inputs": below_ground, "engine_version": VERSION }));
    let request = list_request(&tapes, store);
    let sentence = "Physical Violation: altitude is above 134.7 km or below 30 km.";
    let results = vec![
        RowResult::Run(tapes[0].clone()),
        RowResult::Run(tapes[1].clone()),
        RowResult::Run(tapes[2].clone()),
        RowResult::Unadmitted(sentence.to_string()),
    ];
    files_of(assemble(&request, VERSION, &results).unwrap())
}

pub fn split_lines(rows: &[u8]) -> Vec<Vec<u8>> {
    rows.strip_suffix(b"\n").unwrap().split(|b| *b == b'\n').map(<[u8]>::to_vec).collect()
}

pub fn join_lines(lines: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in lines {
        bytes.extend_from_slice(line);
        bytes.push(b'\n');
    }
    bytes
}

pub fn edit_row(rows: &[u8], row: usize, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut lines = split_lines(rows);
    let mut value: Value = serde_json::from_slice(&lines[row]).unwrap();
    edit(&mut value);
    lines[row] = serde_json::to_vec(&value).unwrap();
    join_lines(&lines)
}

/// A head that names `rows` (and `frames`) by hash, with a receipt that matches.
pub fn head_naming(head: &[u8], rows: &[u8], frames: Option<&[u8]>) -> Vec<u8> {
    let mut head = Head::read(head).unwrap();
    head.rows_sha256 = Some(digest_hex(rows));
    head.frames_sha256 = frames.map(digest_hex);
    head.wire().into_bytes()
}

/// A folder of the tests' own, empty.
pub fn folder(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

pub fn write(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path.display().to_string()
}

pub struct Ran {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Run the program on the files, into `out`. `frames` may be absent.
pub fn convert(
    dir: &Path,
    head: &[u8],
    rows: &[u8],
    frames: Option<&[u8]>,
    out: &Path,
) -> Ran {
    let head = write(dir, "head.json", head);
    let rows = write(dir, "rows.jsonl", rows);
    let mut arguments = vec![head, rows];
    if let Some(frames) = frames {
        arguments.push(write(dir, "frames.bin", frames));
    }
    arguments.push(out.display().to_string());
    let output = Command::new(env!("CARGO_BIN_EXE_genesis_seal_parquet"))
        .args(&arguments)
        .output()
        .unwrap();
    Ran {
        code: output.status.code().unwrap(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// The names in a folder, sorted. Nothing, not even a temporary file or the folder itself, is `None`.
pub fn contents(dir: &Path) -> Option<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Some(names)
}

pub fn read_parquet(path: &Path) -> (Vec<RecordBatch>, Vec<(String, String)>) {
    let file = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
    let metadata: Vec<(String, String)> = builder
        .metadata()
        .file_metadata()
        .key_value_metadata()
        .map(|pairs| {
            pairs
                .iter()
                .filter(|kv| kv.key != "ARROW:schema")
                .map(|kv| (kv.key.clone(), kv.value.clone().unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default();
    let batches = builder.build().unwrap().map(|b| b.unwrap()).collect();
    (batches, metadata)
}

pub fn meta<'a>(metadata: &'a [(String, String)], key: &str) -> &'a str {
    &metadata.iter().find(|(k, _)| k == key).unwrap_or_else(|| panic!("no metadata {key}")).1
}
