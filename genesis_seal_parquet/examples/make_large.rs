//! Makes a large sealed dataset with no engine, to measure the converter.
//! `cargo run --release --example make_large <rows> <frames per row> <folder>`
//! Every row is a made-up run (see `tests/support`), sealed by the sealing
//! crate's own writer as the rows arrive. 336 rows of 50,000 frames is 1 GiB.
//! The request of every row is taken from one saved engine run, in the folder
//! the environment variable `ZERO_TRUST_PHYSICS_RUNS` names.

use std::fs::File;
use std::io::BufWriter;

use genesis_seal::dataset::{DatasetWriter, RowResult};
use genesis_seal::plan::DatasetRequest;
use genesis_seal::run::{FRAME_LEN, link_0, run_seal, seal_frame, state_digest};
use serde_json::{Value, json};

fn run(request: Value, steps: u64, mark: u8) -> Value {
    let mut link = link_0(&request);
    let mut frames = Vec::with_capacity(steps as usize);
    for step in 1..=steps {
        let mut frame = [mark; FRAME_LEN];
        frame[0..8].copy_from_slice(&step.to_le_bytes());
        link = seal_frame(&link, &mut frame);
        frames.push(json!({ "step": step, "frame_hex": hex::encode(frame), "link": hex::encode(link) }));
    }
    let state = json!({
        "t_s": 1.0, "t_s_hex": "3ff0000000000000",
        "position_m": [1.0, 2.0, 3.0],
        "position_m_hex": ["3ff0000000000000", "4000000000000000", "4008000000000000"],
        "in_shadow": false
    });
    let zero = "0000000000000000";
    let digest = state_digest(state.as_object().unwrap(), zero, zero).unwrap();
    let seal = run_seal(&link, "completed", steps, None, Some(&digest));
    json!({
        "inputs": request, "engine_version": "no_engine", "outcome": "completed",
        "steps_requested": steps, "steps_landed": steps, "reject": null, "reject_detail": null,
        "final_state": state,
        "final_residual_conservation_error": 0.0, "final_residual_conservation_error_hex": zero,
        "max_residual_conservation_error": 0.0, "max_residual_conservation_error_hex": zero,
        "state_digest": digest, "frames": frames, "run_seal": seal
    })
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [rows, steps, folder] = arguments.as_slice() else {
        panic!("usage: make_large <rows> <frames per row> <folder>");
    };
    let (rows, steps): (u64, u64) = (rows.parse().unwrap(), steps.parse().unwrap());
    std::fs::create_dir_all(folder).unwrap();
    // A real run request from the saved runs, read where it stands, with one frame at each step.
    let saved = std::env::var_os("ZERO_TRUST_PHYSICS_RUNS")
        .map(std::path::PathBuf::from)
        .filter(|runs| runs.is_dir())
        .map(|runs| runs.join("satellite_lageos1_validation/rehearsal_lageos1_260905_d6_1356_steps.json"));
    let Some(saved) = saved else {
        eprintln!("make_large needs the saved runs; set ZERO_TRUST_PHYSICS_RUNS to their folder");
        std::process::exit(2);
    };
    let saved: Value = serde_json::from_slice(
        &std::fs::read(&saved).unwrap_or_else(|error| panic!("{} was not read: {error}", saved.display())),
    )
    .unwrap();
    let mut base = saved["inputs"].clone();
    base.as_object_mut().unwrap().shift_remove("domain");
    base.as_object_mut().unwrap().shift_remove("card");
    base["record_every"] = json!(1);
    let runs: Vec<Value> = (0..rows)
        .map(|i| {
            let mut run = base.clone();
            run["dt_s"] = json!(0.01 + i as f64 * 0.0001);
            run
        })
        .collect();
    let request = DatasetRequest::from_value(json!({
        "domain": saved["inputs"]["domain"], "card": saved["inputs"]["card"],
        "plan": { "kind": "list", "runs": runs }, "store": "frames", "return": "file"
    }))
    .unwrap_or_else(|r| panic!("plan refused: {r}"));
    let rows_out = BufWriter::new(File::create(format!("{folder}/rows.jsonl")).unwrap());
    let frames_out = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(format!("{folder}/frames.bin"))
        .unwrap();
    let mut writer = DatasetWriter::new(&request, "no_engine", rows_out, Some(frames_out)).unwrap();
    for i in 0..rows {
        let mut inputs = request.row(i).unwrap();
        inputs["record_every"] = json!(1);
        let tape = run(inputs, steps, (i % 251) as u8);
        writer.push(i, &RowResult::Run(tape)).unwrap_or_else(|r| panic!("row {i}: {r}"));
    }
    let written = writer.finish().unwrap();
    std::fs::write(format!("{folder}/head.json"), written.head.wire()).unwrap();
    println!("{}", written.head.counts.text());
}
