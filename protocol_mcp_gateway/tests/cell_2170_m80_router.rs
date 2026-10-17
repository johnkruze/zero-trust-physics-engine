//! The Tesla Model 3 2170 cell card, cell m80, on the doors.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes. A run ends in each of its three sealed refusals (the lower voltage
//! limit, the upper voltage limit, the end of the tables) on the step the
//! kernel loop stops at, with the kernel loop's last state to the bit. A run
//! whose current changes and reverses, with rests and a drifting case
//! temperature, is the kernel loop's too. Every published bound is refused by
//! name at the last representable number on each side, and nothing is pulled
//! back to a bound. A case temperature or a current outside the tests of the
//! run's block, and the -10 and -20 degC blocks that met no mark, are
//! answered with their flags. A run that ends exactly on either end of the tables lands
//! its last step. A missing input, a card constant sent as a field, a step
//! that is not one second and a chamber setting that names no block are
//! refused. A dataset's rows carry the run seals of the runs alone, and
//! `validate_series` carries the run seal of the run. The agent door returns
//! the bytes of the HTTP door for all four tools.
//!
//! No measurement of the cell is read here. The currents and temperatures are
//! round numbers chosen for the test.

mod answer_text;

use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::cell_2170_m80::{self, Cell2170M80, CellLoad, CellState, StepReject};
use genesis_kernel::frame::LastStateFrame64;
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

// ------------------------------------------------------------------ helpers

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post_text(base: &str, path: &str, body: &str) -> (u16, String) {
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                let text = response.text().await.unwrap();
                answer_text::remember(&text);
                return (status, text);
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    post_text(base, path, &answer_text::as_sent(body)).await
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The next 64-bit number under `x`, for an `x` that is not zero.
fn below(x: f64) -> f64 {
    if x > 0.0 {
        f64::from_bits(x.to_bits() - 1)
    } else {
        f64::from_bits(x.to_bits() + 1)
    }
}

/// The next 64-bit number over `x`, for an `x` that is not zero.
fn above(x: f64) -> f64 {
    if x > 0.0 {
        f64::from_bits(x.to_bits() + 1)
    } else {
        f64::from_bits(x.to_bits() - 1)
    }
}

fn cell() -> Cell2170M80 {
    Cell2170M80::cell()
}

fn step_body(block_c: f64, charge_ah: f64, current_a: f64, case_c: f64, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": "cell_2170_m80",
        "parameters": {
            "block_chamber_c": block_c,
            "charge_removed_ah": charge_ah,
            "current_a": current_a,
            "case_temperature_c": case_c,
            "dt_s": dt_s
        }
    })
}

/// A run of segments, each `(steps, current, case temperature)`.
fn run_body(block_c: f64, charge_ah: f64, segments: &[(u64, f64, f64)], record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "cell_2170_m80",
        "initial": { "block_chamber_c": block_c, "charge_removed_ah": charge_ah },
        "dt_s": 1.0,
        "segments": segments.iter().map(|(steps, current_a, case_c)| json!({
            "steps": steps,
            "load": { "current_a": current_a, "case_temperature_c": case_c }
        })).collect::<Vec<_>>(),
        "record_every": record_every
    })
}

fn blank_frame(proof: [u8; 16]) -> LastStateFrame64 {
    LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof,
    }
}

/// The kernel loop over the same segments, until the kernel refuses.
fn direct_loop(
    block_c: f64,
    charge_ah: f64,
    segments: &[(u64, f64, f64)],
) -> (CellState, u64, Option<StepReject>, f64) {
    let cell = cell();
    let mut state = CellState::try_start(&cell, block_c, charge_ah).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut landed = 0;
    let mut worst = 0.0_f64;
    for (steps, current_a, case_c) in segments {
        let load = CellLoad {
            current_a: *current_a,
            case_temperature_c: *case_c,
        };
        for _ in 0..*steps {
            match cell_2170_m80::step(&cell, &mut state, &mut frame, load, 1.0) {
                Ok(()) => {
                    landed += 1;
                    worst = worst.max(state.residual_conservation_error);
                }
                Err(reject) => return (state, landed, Some(reject), worst),
            }
        }
    }
    (state, landed, None, worst)
}

/// The published number of a state, by its name, from the hex beside it.
fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn state_numbers(state: &CellState) -> Vec<(&'static str, f64)> {
    vec![
        ("t_s", state.t_s),
        ("block_chamber_c", state.block_chamber_c),
        ("charge_removed_ah", state.charge_removed_ah),
        ("charge_level", state.charge_level),
        ("current_a", state.current_a),
        ("case_temperature_c", state.case_temperature_c),
        ("terminal_v", state.terminal_v),
        ("open_circuit_v", state.open_circuit_v),
        ("branch_v", state.branch_v),
        ("resistance_at_once_ohm", state.resistance_at_once_ohm),
        ("resistance_branch_ohm", state.resistance_branch_ohm),
        ("time_constant_s", state.time_constant_s),
        ("chemical_energy_j", state.chemical_energy_j),
        ("energy_anchor_j", state.energy_anchor_j),
        ("electrical_work_j", state.electrical_work_j),
        ("heat_at_once_j", state.heat_at_once_j),
        ("branch_energy_j", state.branch_energy_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
        ("lower_limit_margin_v", state.lower_limit_margin_v),
        ("upper_limit_margin_v", state.upper_limit_margin_v),
    ]
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// both as the decimal and as the hex beside it, and nothing else is published.
fn assert_state_is(published_state: &Value, state: &CellState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in state_numbers(state) {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha256\":\"";
    let at = text.rfind(MARKER).expect("a receipt");
    (format!("{}}}", &text[..at]), text[at + MARKER.len()..text.len() - 2].to_string())
}

fn names(text: &str, name: &str) -> bool {
    text.contains(&format!("({name}). That state is not sealed."))
        && !text.contains("sha256_seal")
        && !text.contains("run_seal")
}

// -------------------------------------------------------------- the step

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_cell_matches_the_compiled_step_with_the_same_proof_bytes() {
    let base = spawn_app().await;
    let cases = [
        step_body(25.0, 0.0, -2.25, 25.0, 1.0),
        step_body(40.0, 4.6, -0.225, 41.5, 1.0),
        step_body(25.0, 2.0, 9.484, 30.993, 1.0),
        step_body(0.0, 2.0, 2.25, 6.0, 1.0),
        step_body(0.0, 0.5, 0.0, 0.808, 1.0),
        step_body(10.0, 3.0, -10.079, 21.0, 1.0),
    ];
    let mut seals = Vec::new();
    for request in cases {
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);

        // The compiled step, with the proof the door writes.
        let parameters = &request["parameters"];
        let number = |key: &str| parameters[key].as_f64().unwrap();
        let cell = cell();
        let mut state =
            CellState::try_start(&cell, number("block_chamber_c"), number("charge_removed_ah")).unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        cell_2170_m80::step(
            &cell,
            &mut state,
            &mut frame,
            CellLoad {
                current_a: number("current_a"),
                case_temperature_c: number("case_temperature_c"),
            },
            number("dt_s"),
        )
        .unwrap();
        assert_eq!(frame.flags, 0);
        assert_state_is(&response.state, &state, "step");
        assert_eq!(
            response.residual_conservation_error.to_bits(),
            state.residual_conservation_error.to_bits()
        );
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        // The seal covers the proof: the same frame with a zero proof seals apart.
        let mut naked = frame;
        naked.proof = [0; 16];
        assert_ne!(response.sha256_seal, sha256_hex(&naked.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha256);
        assert_eq!(sha256_hex(document.as_bytes()), receipt);
        // The same request again gives the same bytes.
        let (_, again) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(again, answer);
        seals.push(response.sha256_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 6, "six different steps, six different seals");
}

// ---------------------------------------------------------------- the run

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_ends_in_each_sealed_refusal_on_the_step_the_kernel_loop_stops_at() {
    let base = spawn_app().await;
    // (block, start, current, case temperature, steps the card lands, the refusal, its sentence)
    let runs = [
        (
            40.0,
            0.0,
            -2.25,
            40.25,
            7_269_u64,
            StepReject::LowerVoltageLimit,
            "Physical Failure: the cell's voltage would fall below the 2.8 V lower limit (LowerVoltageLimit). That state is not sealed.",
        ),
        (
            25.0,
            0.3,
            2.25,
            26.0,
            189,
            StepReject::UpperVoltageLimit,
            "Physical Failure: the cell's voltage would rise above the 4.242 V upper limit (UpperVoltageLimit). That state is not sealed.",
        ),
        (
            40.0,
            4.6,
            -0.225,
            41.5,
            1_229,
            StepReject::ChargeOutsideTables,
            "Physical Failure: the charge removed would leave the tables of the block (ChargeOutsideTables). That state is not sealed.",
        ),
    ];
    for (block_c, charge_ah, current_a, case_c, expect_landed, expect, sentence) in runs {
        let asked = 9_000_u64;
        let segments = [(asked, current_a, case_c)];
        let request = run_body(block_c, charge_ah, &segments, 500);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();

        let (state, landed, reject, worst) = direct_loop(block_c, charge_ah, &segments);
        assert_eq!(reject, Some(expect));
        assert_eq!(landed, expect_landed);
        assert!(worst < 1.0e-13, "residual {worst}");
        assert_eq!(tape["outcome"], "refused");
        assert_eq!(tape["reject"], format!("{expect:?}"));
        assert_eq!(tape["reject_detail"], sentence.replace(" That state is not sealed.", &format!(" The run is sealed to step {}. The state at step {} was refused and is not sealed.", landed, landed + 1)));
        assert_eq!(tape["steps_landed"], landed);
        assert_eq!(tape["steps_requested"], asked);
        // The last landed state is the kernel loop's, to the bit, and it is
        // inside both voltage limits: the refused state is not in it.
        let last = &tape["final_state"];
        assert_state_is(last, &state, &format!("{expect:?}"));
        let terminal_v = published(last, "terminal_v");
        assert!(terminal_v >= cell_2170_m80::LOWER_VOLTAGE_LIMIT_V);
        assert!(terminal_v <= cell_2170_m80::UPPER_VOLTAGE_LIMIT_V);
        assert_eq!(published(last, "t_s"), landed as f64);
        assert_eq!(
            tape["max_residual_conservation_error"].as_f64().unwrap().to_bits(),
            worst.to_bits()
        );

        // The tape verifies, and the receipt of the response matches.
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{report}");
        let (status, receipt) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{receipt}");
        let receipt: Value = serde_json::from_str(&receipt).unwrap();
        assert_eq!(receipt["matches"], true, "{receipt}");

        // The same request gives the same bytes.
        let (_, again) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(again, answer);

        // One step fewer than the card lands completes, with the same state
        // one second earlier than the refusal.
        let short = run_body(block_c, charge_ah, &[(landed, current_a, case_c)], landed);
        let (status, text) = post(&base, "/run_trajectory", &short).await;
        assert_eq!(status, 200, "{text}");
        let short: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(short["outcome"], "completed");
        assert_eq!(short["final_state"], tape["final_state"]);
    }
}

/// A current that changes every few seconds and reverses, with rests, and a
/// case temperature that drifts by a tenth of a kelvin a segment.
fn drive_segments(start_case_c: f64) -> Vec<(u64, f64, f64)> {
    let pattern: [(u64, f64); 12] = [
        (20, 0.0),
        (15, -4.5),
        (6, -9.0),
        (10, 3.0),
        (30, -1.35),
        (4, -9.2),
        (8, 6.75),
        (60, 0.0),
        (25, -2.25),
        (5, 9.0),
        (12, -6.2),
        (40, -0.4),
    ];
    let mut segments = Vec::new();
    for round in 0..5 {
        for (index, (steps, current_a)) in pattern.iter().enumerate() {
            let case_c = start_case_c + 0.1 * (round * pattern.len() + index) as f64;
            segments.push((*steps, *current_a, case_c));
        }
    }
    segments
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_with_a_changing_current_and_rests_is_the_kernel_loop_and_a_changed_tape_breaks() {
    let base = spawn_app().await;
    let segments = drive_segments(24.5);
    let total: u64 = segments.iter().map(|segment| segment.0).sum();
    assert_eq!(total, 5 * 235);
    let request = run_body(25.0, 1.5, &segments, 1);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    let (state, landed, reject, worst) = direct_loop(25.0, 1.5, &segments);
    assert_eq!(reject, None);
    assert_eq!(landed, total);
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], total);
    assert_eq!(tape["frames"].as_array().unwrap().len() as u64, total);
    assert_state_is(&tape["final_state"], &state, "drive");
    assert!(worst < 1.0e-13, "residual {worst}");
    // Charge went out and came back: the cell ends with more removed than at the start.
    assert!(state.charge_removed_ah > 1.5);

    // Every frame carries the terminal voltage of its second, rounded to 32
    // bits, and the kernel loop gives the same value at that second.
    let cell = cell();
    let mut check = CellState::try_start(&cell, 25.0, 1.5).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut second = 0;
    let frames = tape["frames"].as_array().unwrap();
    for (steps, current_a, case_c) in &segments {
        for _ in 0..*steps {
            cell_2170_m80::step(
                &cell,
                &mut check,
                &mut frame,
                CellLoad { current_a: *current_a, case_temperature_c: *case_c },
                1.0,
            )
            .unwrap();
            let bytes = hex::decode(frames[second]["frame_hex"].as_str().unwrap()).unwrap();
            let terminal = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
            assert_eq!(terminal.to_bits(), (check.terminal_v as f32).to_bits(), "second {second}");
            let charge = f32::from_le_bytes(bytes[12..16].try_into().unwrap());
            assert_eq!(charge.to_bits(), (check.charge_removed_ah as f32).to_bits());
            assert_eq!(u64::from_le_bytes(bytes[40..48].try_into().unwrap()), 0, "flags");
            second += 1;
        }
    }

    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200);
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true);
    // A changed state value breaks the tape.
    let mut forged = tape.clone();
    let terminal_hex = forged["final_state"]["terminal_v_hex"].as_str().unwrap().to_string();
    let bumped = format!("{:016x}", u64::from_str_radix(&terminal_hex, 16).unwrap() + 1);
    forged["final_state"]["terminal_v_hex"] = json!(bumped);
    let (_, report) = post(&base, "/verify_tape", &forged).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false);
    assert_eq!(report["cause"], "state");
}

// ------------------------------------------------------------------ bounds

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let (_, described) =
        post(&base, "/describe_card", &json!({"domain": "energy", "card": "cell_2170_m80"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    // The description publishes the kernel's numbers for all four inputs.
    let bound = |list: &str, index: usize, key: &str| described[list][index][key].as_f64().unwrap();
    assert_eq!(bound("phase_fields", 0, "minimum"), -20.0);
    assert_eq!(bound("phase_fields", 0, "maximum"), 40.0);
    assert_eq!(bound("phase_fields", 1, "minimum").to_bits(), cell_2170_m80::CHARGE_TOP_AH[0].to_bits());
    assert_eq!(bound("phase_fields", 1, "maximum").to_bits(), cell_2170_m80::CHARGE_DEEP_AH[0].to_bits());
    assert_eq!(bound("load_fields", 0, "minimum"), -cell_2170_m80::CURRENT_DISCHARGE_MAX_A);
    assert_eq!(bound("load_fields", 0, "maximum"), cell_2170_m80::CURRENT_CHARGE_MAX_A);
    assert_eq!(bound("load_fields", 1, "minimum"), cell_2170_m80::CASE_TEMPERATURE_MIN_C);
    assert_eq!(bound("load_fields", 1, "maximum"), cell_2170_m80::CASE_TEMPERATURE_MAX_C);
    // The description names each block's band of compared case temperatures.
    let case_words = described["load_fields"][1]["bound"].as_str().unwrap();
    for words in [
        "40 degC block: [39.063, 45.535] degC",
        "25 degC block: [24.267, 30.993] degC",
        "10 degC block: [10.201, 21.464] degC",
        "0 degC block: [0.808, 16.259] degC",
        "-10 degC block: [-11.995, 8.389] degC",
        "-20 degC block: [-21.898, 2.885] degC",
        "FLAG_OUTSIDE_COMPARED_TESTS",
    ] {
        assert!(case_words.contains(words), "{words}: {case_words}");
    }

    let top = cell_2170_m80::CHARGE_TOP_AH;
    let deep = cell_2170_m80::CHARGE_DEEP_AH;
    let low = cell_2170_m80::BLOCK_CASE_MIN_C;
    let high = cell_2170_m80::BLOCK_CASE_MAX_C;
    let allowance = cell().charge_end_allowance_ah(0.0);
    assert_eq!(allowance.to_bits(), (2.0 * cell_2170_m80::CHARGE_END_ALLOWANCE_PER_STEP_AH).to_bits());
    // (block, charge, current, case temperature, the refusal named, or none)
    let cases: Vec<(f64, f64, f64, f64, Option<&str>)> = vec![
        // The case temperature: the bounds of the card, where its tables end.
        (-20.0, 1.0, -1.0, -21.898, None),
        (-20.0, 1.0, -1.0, below(-21.898), Some("CaseTemperatureOutsideCard")),
        (40.0, 1.0, -1.0, 45.535, None),
        (40.0, 1.0, -1.0, above(45.535), Some("CaseTemperatureOutsideCard")),
        (40.0, 1.0, -1.0, -25.0, Some("CaseTemperatureOutsideCard")),
        (0.0, 1.0, -1.0, 50.0, Some("CaseTemperatureOutsideCard")),
        // Outside the band of the block: answered, flagged.
        (25.0, 1.0, -1.0, below(low[1]), None),
        (25.0, 1.0, -1.0, above(high[1]), None),
        (40.0, 1.0, -4.5, 0.808, None),
        (0.0, 1.0, -4.5, 45.535, None),
        // The current, both ways: to the end of the tables, and the next number past it.
        (0.0, 2.0, -11.375, 10.0, None),
        (0.0, 2.0, below(-11.375), 10.0, None),
        (40.0, 2.0, -13.5, 40.0, None),
        (40.0, 2.0, below(-13.5), 40.0, Some("CurrentOutsideCard")),
        (40.0, 2.0, 13.5, 40.0, None),
        (40.0, 2.0, above(13.5), 40.0, Some("CurrentOutsideCard")),
        (0.0, 3.0, 9.737, 10.0, None),
        // The block: the two outermost settings, and one number beyond each.
        (40.0, 1.0, -1.0, 40.0, None),
        (above(40.0), 1.0, -1.0, 40.0, Some("BlockNotRecognized")),
        (-20.0, 1.0, -1.0, -18.0, None),
        (below(-20.0), 1.0, -1.0, 5.0, Some("BlockNotRecognized")),
        // The two blocks that met no mark, served.
        (-10.0, 1.0, -1.0, -8.0, None),
        (-10.0, 1.0, -1.0, 5.0, None),
        (above(-20.0), 1.0, -1.0, 5.0, Some("BlockNotRecognized")),
        // A setting between two blocks names no block and is not rounded to one.
        (15.0, 1.0, -1.0, 15.0, Some("BlockNotRecognized")),
        (below(25.0), 1.0, -1.0, 25.0, Some("BlockNotRecognized")),
        (above(25.0), 1.0, -1.0, 25.0, Some("BlockNotRecognized")),
        // The charge removed: the widest range the card publishes, both ends
        // taken, and the next 64-bit number past each end refused by name,
        // as is a start within the rounding allowance of a running sum: no
        // start is read at an end point it is not at.
        (40.0, top[0], 0.0, 40.0, None),
        (40.0, below(top[0]), 0.0, 40.0, Some("ChargeOutsideTables")),
        (40.0, top[0] - allowance, 0.0, 40.0, Some("ChargeOutsideTables")),
        (40.0, top[0] - 1.0e-9, 0.0, 40.0, Some("ChargeOutsideTables")),
        (40.0, deep[0], 0.0, 40.0, None),
        (40.0, above(deep[0]), 0.0, 40.0, Some("ChargeOutsideTables")),
        (40.0, deep[0] + allowance, 0.0, 40.0, Some("ChargeOutsideTables")),
        (40.0, deep[0] + 1.0e-9, 0.0, 40.0, Some("ChargeOutsideTables")),
    ];
    // Each block's own range: its deepest charge is a legal start, the next
    // number beyond it is not, though it is inside the widest range.
    let mut own: Vec<(f64, f64, f64, f64, Option<&str>)> = Vec::new();
    for block in 0..cell_2170_m80::BLOCK_COUNT {
        let chamber = cell_2170_m80::BLOCK_CHAMBER_C[block];
        let case_c = 0.5 * (low[block] + high[block]);
        own.push((chamber, deep[block], 0.0, case_c, None));
        own.push((chamber, above(deep[block]), 0.0, case_c, Some("ChargeOutsideTables")));
        own.push((chamber, deep[block] + allowance, 0.0, case_c, Some("ChargeOutsideTables")));
        own.push((chamber, top[block], 0.0, case_c, None));
        // Its own currents and the next number past each: answered, flagged.
        let discharge = cell_2170_m80::BLOCK_CURRENT_DISCHARGE_MAX_A[block];
        own.push((chamber, 2.5, -discharge, case_c, None));
        own.push((chamber, 2.5, below(-discharge), case_c, None));
    }
    for (block_c, charge_ah, current_a, case_c, refusal) in cases.into_iter().chain(own) {
        let label = format!("block {block_c} charge {charge_ah} current {current_a} case {case_c}");
        let step = step_body(block_c, charge_ah, current_a, case_c, 1.0);
        let run = run_body(block_c, charge_ah, &[(3, current_a, case_c)], 1);
        let plan = json!({
            "domain": "energy",
            "card": "cell_2170_m80",
            "plan": { "kind": "list", "runs": [{
                "initial": { "block_chamber_c": block_c, "charge_removed_ah": charge_ah },
                "dt_s": 1.0,
                "segments": [{ "steps": 3, "load": { "current_a": current_a, "case_temperature_c": case_c } }],
                "record_every": 1
            }] },
            "store": "none",
            "return": "inline", "answer": "full"
        });
        let (step_status, step_text) = post(&base, "/verify_physics", &step).await;
        let (run_status, run_text) = post(&base, "/run_trajectory", &run).await;
        let (plan_status, plan_text) = post(&base, "/run_dataset", &plan).await;
        match refusal {
            None => {
                assert_eq!(step_status, 200, "{label}: {step_text}");
                assert_eq!(run_status, 200, "{label}: {run_text}");
                assert_eq!(plan_status, 200, "{label}: {plan_text}");
                // The state records the value that was sent, not a neighbour of it.
                let response: Value = serde_json::from_str(&step_text).unwrap();
                assert_eq!(published(&response["state"], "block_chamber_c").to_bits(), (block_c + 0.0).to_bits());
                assert_eq!(published(&response["state"], "current_a").to_bits(), current_a.to_bits());
                assert_eq!(published(&response["state"], "case_temperature_c").to_bits(), case_c.to_bits());
                let tape: Value = serde_json::from_str(&run_text).unwrap();
                assert_eq!(tape["outcome"], "completed", "{label}: {run_text}");
            }
            Some(name) => {
                for (status, text) in [(step_status, &step_text), (run_status, &run_text)] {
                    assert_eq!(status, 422, "{label}: {text}");
                    assert!(names(text, name), "{label}: {text}");
                }
                // A dataset row outside the card is not admitted: it carries
                // the same sentence and no run seal, and nothing runs.
                assert_eq!(plan_status, 200, "{label}: {plan_text}");
                let answer: Value = serde_json::from_str(&plan_text).unwrap();
                assert_eq!(answer["rows"][0]["outcome"], "unadmitted", "{plan_text}");
                assert!(answer["rows"][0]["run_seal"].is_null(), "{plan_text}");
                assert_eq!(answer["rows"][0]["reject_detail"], step_text, "{label}");
            }
        }
    }

    // A value outside its range in a later segment refuses the whole run
    // before its first step.
    for (current_a, case_c, name) in [
        (above(13.5), 25.0, "CurrentOutsideCard"),
        (below(-13.5), 25.0, "CurrentOutsideCard"),
        (-1.0, above(45.535), "CaseTemperatureOutsideCard"),
        (-1.0, below(-21.898), "CaseTemperatureOutsideCard"),
    ] {
        let run = run_body(25.0, 1.0, &[(5, -1.0, 25.0), (5, 0.0, 25.0), (5, current_a, case_c)], 1);
        let (status, text) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 422, "{text}");
        assert!(names(&text, name), "{text}");
    }

    // The step is one second and no other.
    for (dt_s, words) in [
        (below(1.0), "the step floor of the cell_2170_m80 card"),
        (0.5, "the step floor of the cell_2170_m80 card"),
        (above(1.0), "the step ceiling of the cell_2170_m80 card"),
        (2.0, "the step ceiling of the cell_2170_m80 card"),
        (0.0, "NonPositiveDt"),
        (-1.0, "NonPositiveDt"),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body(25.0, 1.0, -1.0, 25.0, dt_s)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains(words) && !text.contains("sha256_seal"), "{dt_s}: {text}");
        let mut run = run_body(25.0, 1.0, &[(3, -1.0, 25.0)], 1);
        run["dt_s"] = json!(dt_s);
        let (status, text) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains(words) && !text.contains("run_seal"), "{dt_s}: {text}");
    }
}

/// Two runs that lost their last step on engine 0.6.0: each ends in decimal
/// exactly on an end of the 25 degC block's tables, the top at -0.02 Ah and
/// the bottom at 4.4615 Ah.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_that_ends_exactly_on_either_end_of_the_tables_lands_its_last_step() {
    let base = spawn_app().await;
    for (charge_ah, segments) in [
        (-0.019, [(3_u64, 1.2, 25.0)]),
        (4.4605, [(4_u64, -0.9, 25.0)]),
    ] {
        let request = run_body(25.0, charge_ah, &segments, 1);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        let (state, landed, reject, _) = direct_loop(25.0, charge_ah, &segments);
        assert_eq!(reject, None, "{charge_ah}");
        assert_eq!(landed, segments[0].0);
        assert_eq!(tape["outcome"], "completed", "{answer}");
        assert_eq!(tape["steps_landed"], segments[0].0);
        assert_state_is(&tape["final_state"], &state, "end of the tables");
        // The rest voltage is read at the end of the tables.
        let end_ah = if charge_ah < 0.0 { cell_2170_m80::CHARGE_TOP_AH[1] } else { cell_2170_m80::CHARGE_DEEP_AH[1] };
        let at_end = CellState::try_start(&cell(), 25.0, end_ah).unwrap();
        assert_eq!(published(&tape["final_state"], "open_circuit_v").to_bits(), at_end.open_circuit_v.to_bits());
        // One step more leaves the tables, and the step is refused by name.
        let (more, current_a, case_c) = segments[0];
        let longer = run_body(25.0, charge_ah, &[(more + 1, current_a, case_c)], 1);
        let (status, text) = post(&base, "/run_trajectory", &longer).await;
        assert_eq!(status, 200, "{text}");
        let longer: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(longer["outcome"], "refused");
        assert_eq!(longer["reject"], "ChargeOutsideTables");
        assert_eq!(longer["steps_landed"], more);
        // The charge this run returned is a spacing or two past the end. No
        // start is read at an end point it is not at, so a new run from that
        // charge is refused by name, as a step from it is; a new run starts
        // from the end itself.
        let returned = published(&tape["final_state"], "charge_removed_ah");
        let back = if charge_ah < 0.0 { -1.2 } else { 0.9 };
        let on_tables = returned >= cell_2170_m80::CHARGE_TOP_AH[1] && returned <= cell_2170_m80::CHARGE_DEEP_AH[1];
        let (status, text) = post(&base, "/run_trajectory", &run_body(25.0, returned, &[(2, back, 25.0)], 1)).await;
        let (step_status, step_text) = post(&base, "/verify_physics", &step_body(25.0, returned, back, 25.0, 1.0)).await;
        if on_tables {
            assert_eq!((status, step_status), (200, 200), "{returned}: {text} {step_text}");
        } else {
            assert_eq!((status, step_status), (422, 422), "{returned}: {text} {step_text}");
            assert!(text.contains("(ChargeOutsideTables)") && !text.contains("run_seal"), "{text}");
            assert!(step_text.contains("(ChargeOutsideTables)") && !step_text.contains("sha256_seal"), "{step_text}");
        }
        let again = run_body(25.0, end_ah, &[(2, back, 25.0)], 1);
        let (status, text) = post(&base, "/run_trajectory", &again).await;
        assert_eq!(status, 200, "{end_ah}: {text}");
        let again: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(again["outcome"], "completed", "{text}");
        let (state, _, reject, _) = direct_loop(25.0, end_ah, &[(2, back, 25.0)]);
        assert_eq!(reject, None);
        assert_state_is(&again["final_state"], &state, "a new run from the end of the tables");
    }
    // The deep-end run returns a charge past the end of the tables.
    let (status, answer) = post(&base, "/run_trajectory", &run_body(25.0, 4.4605, &[(4, -0.9, 25.0)], 1)).await;
    assert_eq!(status, 200);
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert!(published(&tape["final_state"], "charge_removed_ah") > cell_2170_m80::CHARGE_DEEP_AH[1]);
    // The same deep-end run at a case temperature of 40 degC in the 25 degC
    // block is outside the band of that block's own tests: answered, flagged.
    let (status, text) = post(&base, "/run_trajectory", &run_body(25.0, 4.4605, &[(4, -0.9, 40.0)], 1)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    let last = tape["frames"].as_array().unwrap().last().unwrap()["frame_hex"].as_str().unwrap().to_string();
    let bytes = hex::decode(last).unwrap();
    assert_eq!(u64::from_le_bytes(bytes[40..48].try_into().unwrap()), cell_2170_m80::FLAG_OUTSIDE_COMPARED_TESTS);
    // The description says how the ends are read, and keeps no known defect.
    let (_, described) =
        post(&base, "/describe_card", &json!({"domain": "energy", "card": "cell_2170_m80"})).await;
    assert!(described.contains("lands its last step"), "{described}");
    assert!(!described.contains("Known defect"), "{described}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_input_a_non_number_and_a_card_constant_are_refused_and_nothing_is_filled_in() {
    let base = spawn_app().await;
    let good = step_body(25.0, 1.0, -1.0, 25.0, 1.0);
    let (status, _) = post(&base, "/verify_physics", &good).await;
    assert_eq!(status, 200);

    // Each of the five inputs, missing.
    for key in ["block_chamber_c", "charge_removed_ah", "current_a", "case_temperature_c", "dt_s"] {
        let mut step = good.clone();
        step["parameters"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}' (InputMissing)."));
    }
    // A run with a load that lacks the case temperature, or a start that lacks the block.
    let mut run = run_body(25.0, 1.0, &[(3, -1.0, 25.0), (3, 0.0, 25.0)], 1);
    run["segments"][1]["load"].as_object_mut().unwrap().remove("case_temperature_c");
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("Missing required parameter: 'case_temperature_c' (InputMissing)."), "{text}");
    let mut run = run_body(25.0, 1.0, &[(3, -1.0, 25.0)], 1);
    run["initial"].as_object_mut().unwrap().remove("block_chamber_c");
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("Missing required parameter: 'block_chamber_c' (InputMissing)."), "{text}");

    // A value that is not a number, and a number that is not finite.
    for (key, raw) in [
        ("current_a", "\"-1.0\""),
        ("case_temperature_c", "null"),
        ("block_chamber_c", "\"25C\""),
        ("charge_removed_ah", "[1.0]"),
    ] {
        let body = serde_json::to_string(&good).unwrap().replace(
            &format!("\"{key}\":{}", serde_json::to_string(&good["parameters"][key]).unwrap()),
            &format!("\"{key}\":{raw}"),
        );
        assert!(body.contains(raw), "{body}");
        let (status, text) = post_text(&base, "/verify_physics", &body).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(&format!("'{key}' must be a number")), "{key}: {text}");
    }
    for raw in ["NaN", "1e309", "-1e309"] {
        let body = serde_json::to_string(&good).unwrap().replace("\"current_a\":-1.0", &format!("\"current_a\":{raw}"));
        assert!(body.contains(raw), "{body}");
        let (status, text) = post_text(&base, "/verify_physics", &body).await;
        assert!(status == 400 || status == 422, "{raw}: {status} {text}");
        assert!(!text.contains("sha256_seal"), "{text}");
    }
    let (status, text) = post_text(
        &base,
        "/verify_physics",
        &serde_json::to_string(&good).unwrap().replace("\"case_temperature_c\":25.0", "\"case_temperature_c\":1e-320"),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("not a finite normal number"), "{text}");

    // A card constant sent as a field is not a field of the request.
    for constant in [
        "capacity_ah",
        "charge_deep_ah",
        "rest_v",
        "held_ohm",
        "first_second_ohm",
        "ten_second_ohm",
        "time_constant_s",
        "lower_voltage_limit_v",
        "upper_voltage_limit_v",
        "current_max_a",
        "current_charge_max_a",
        "current_discharge_max_a",
        "block_case_min_c",
        "block_shown_wrong",
        "step_s",
        "branch_v",
        "residual_conservation_limit",
    ] {
        let mut step = good.clone();
        step["parameters"][constant] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert_eq!(text, format!("Physical Violation: '{constant}' is not a field of this request."));

        let mut in_phase = run_body(25.0, 1.0, &[(3, -1.0, 25.0)], 1);
        in_phase["initial"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_phase).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");

        let mut in_load = run_body(25.0, 1.0, &[(3, -1.0, 25.0)], 1);
        in_load["segments"][0]["load"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");
    }
}

// ------------------------------------------------- the sign of a temperature

/// A request key that ends in `_c` is a temperature in degrees Celsius and may
/// be below zero; any other temperature may not. This walks every served card.
/// The only fields that end in `_c`, and the only ones in degrees Celsius, are
/// the two of this card and the case temperature of the physical card of the
/// same cell. Every field in kelvin on every card still refuses a
/// negative value, on a step, at the start of a run and in each segment, with
/// no proof. A Celsius key sent to another card is not a field of it. The two
/// Celsius fields of this card take no negative value: their bounds are the
/// range compared with measurement, which starts above zero. A negative value
/// is refused by the card's own named refusal, with no proof.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_negative_kelvin_is_refused_on_every_card_and_a_negative_celsius_is_held_to_its_bounds() {
    let base = spawn_app().await;
    let list: Value = Client::new().get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let cards = list["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 37);

    let mut ends_in_c: Vec<(String, String)> = Vec::new();
    let mut in_celsius: Vec<(String, String)> = Vec::new();
    let mut in_kelvin: Vec<(String, String)> = Vec::new();
    for item in cards {
        let (domain, card) = (item["domain"].as_str().unwrap(), item["card"].as_str().unwrap());
        let (status, text) = post(&base, "/describe_card", &json!({ "domain": domain, "card": card })).await;
        assert_eq!(status, 200, "{text}");
        let described: Value = serde_json::from_str(&text).unwrap();
        let step = &described["example_compute_step"];
        let mut run = described["example_run_trajectory"].clone();
        let segment = run["segments"][0].clone();
        run["segments"] = json!([segment.clone(), segment]);

        for (fields, is_load) in [("phase_fields", false), ("load_fields", true)] {
            for field in described[fields].as_array().unwrap() {
                let name = field["name"].as_str().unwrap();
                let unit = field["unit"].as_str().unwrap();
                let lower = name.to_lowercase();
                if lower.ends_with("_c") {
                    ends_in_c.push((card.to_string(), name.to_string()));
                }
                if unit == "degC" {
                    in_celsius.push((card.to_string(), name.to_string()));
                }
                // A temperature by its unit or by its name.
                let temperature =
                    unit == "K" || unit == "degC" || lower.contains("temp") || lower.contains("kelvin");
                if !temperature || unit == "degC" {
                    continue;
                }
                assert_eq!(unit, "K", "{card} {name}");
                assert_eq!(field["components"], 1, "{card} {name}");
                in_kelvin.push((card.to_string(), name.to_string()));
                // The door's sign rule reads the key's name; a kelvin field it
                // does not name is held by the card's published minimum.
                // Re-pinned for 0.10.1: the sign rule's sentence carries the
                // card's own name for a value of that field below what it
                // takes, one of the names the card's description lists.
                let by_sign_rule = format!(
                    "Physical Violation: Temperature '{name}' cannot be below absolute zero, a negative number of kelvin ("
                );
                let listed: Vec<&str> =
                    described["rejects"].as_array().unwrap().iter().map(|reject| reject["name"].as_str().unwrap()).collect();
                let by_minimum = format!("Physical Failure: '{name}' ");
                for value in [-1.0e-3, -1.0, -293.15] {
                    let refused = |text: &str| {
                        if lower.contains("temp") {
                            text.strip_prefix(by_sign_rule.as_str())
                                .and_then(|rest| rest.strip_suffix(")."))
                                .is_some_and(|reject| listed.contains(&reject))
                        } else {
                            text.starts_with(&by_minimum)
                                && text.contains(" K is below ")
                                && text.contains(&format!("minimum of the {card} card"))
                        }
                    };
                    let mut one = step.clone();
                    one["parameters"][name] = json!(value);
                    let (status, text) = post(&base, "/verify_physics", &one).await;
                    assert_eq!(status, 422, "{card} {name} {value} step: {text}");
                    assert!(refused(&text), "{card} {name} {value} step: {text}");
                    assert!(!text.contains("sha256_seal"), "{text}");
                    // The run: at the start, or in the first segment and in a later one.
                    let positions: &[usize] = if is_load { &[0, 1] } else { &[usize::MAX] };
                    for &position in positions {
                        let mut many = run.clone();
                        if is_load {
                            many["segments"][position]["load"][name] = json!(value);
                        } else {
                            many["initial"][name] = json!(value);
                        }
                        let (status, text) = post(&base, "/run_trajectory", &many).await;
                        assert_eq!(status, 422, "{card} {name} {value} run {position}: {text}");
                        assert!(refused(&text), "{card} {name} {value} run {position}: {text}");
                        assert!(!text.contains("run_seal"), "{text}");
                    }
                }
            }
        }

        // A Celsius key sent to a card that has no such field is not a field
        // of the request: the rule for `_c` lets nothing in on another card.
        if card != "cell_2170_m80" {
            // The physical card of the same cell has a case temperature of
            // its own; the other two keys are no field of it.
            let keys: &[&str] = if card == "cell_2170_model3" {
                &["block_chamber_c", "temperature_c"]
            } else {
                &["case_temperature_c", "block_chamber_c", "temperature_c"]
            };
            for key in keys.iter().copied() {
                let sentence = format!("Physical Violation: '{key}' is not a field of this request.");
                let mut one = step.clone();
                one["parameters"][key] = json!(-5.0);
                let (status, text) = post(&base, "/verify_physics", &one).await;
                assert_eq!(status, 422, "{card} {key}: {text}");
                assert_eq!(text, sentence, "{card}");
                for place in ["initial", "load"] {
                    let mut many = run.clone();
                    match place {
                        "initial" => many["initial"][key] = json!(-5.0),
                        _ => many["segments"][1]["load"][key] = json!(-5.0),
                    }
                    let (status, text) = post(&base, "/run_trajectory", &many).await;
                    assert_eq!(status, 422, "{card} {key} {place}: {text}");
                    assert_eq!(text, sentence, "{card} {place}");
                }
            }
        }
    }
    let pair = |card: &str, name: &str| (card.to_string(), name.to_string());
    let the_three = vec![
        pair("cell_2170_m80", "block_chamber_c"),
        pair("cell_2170_m80", "case_temperature_c"),
        pair("cell_2170_model3", "case_temperature_least_c"),
        pair("cell_2170_model3", "case_temperature_most_c"),
        pair("cell_2170_model3", "case_temperature_c"),
    ];
    assert_eq!(ends_in_c, the_three);
    assert_eq!(in_celsius, the_three);
    assert_eq!(
        in_kelvin,
        vec![
            pair("ti64_lattice", "start_temperature_k"),
            pair("ti64_lattice", "temperature_k"),
            pair("grid_node", "temperature_k"),
            pair("grid_node", "ambient_k"),
            pair("thermal", "temperature_k"),
            pair("thermal", "ambient_k"),
            pair("reactor_spert3", "coolant_temperature_k"),
            pair("reactor_spert3_fuel", "inlet_temperature_k"),
            pair("accumulator_sbo450", "gas_temperature_k"),
            pair("accumulator_sbo450", "wall_temperature_k"),
            pair("reactor_triga_jsi", "start_temperature_k"),
            pair("reactor_spert3_ccore", "initial_temperature_k"),
            pair("runaway_18650_lco", "temperature_k"),
            pair("runaway_18650_lco", "oven_temperature_k"),
        ]
    );

    // A case temperature below the coldest of any block's tests is refused by
    // the card's own name, -300 degC as well, in a step and in the first and
    // a later segment of a run; one inside it is answered.
    for case_c in [-22.0, -25.0, -273.15, -300.0] {
        let (status, text) = post(&base, "/verify_physics", &step_body(0.0, 1.0, -1.0, case_c, 1.0)).await;
        assert_eq!(status, 422, "{case_c}: {text}");
        assert!(names(&text, "CaseTemperatureOutsideCard"), "{case_c}: {text}");
        assert!(text.contains("'case_temperature_c'") && text.contains("is below -21.898 degC"), "{case_c}: {text}");
        for segments in [[(3, -1.0, case_c), (3, -1.0, 5.0)], [(3, -1.0, 5.0), (3, -1.0, case_c)]] {
            let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, 1.0, &segments, 1)).await;
            assert_eq!(status, 422, "{case_c}: {text}");
            assert!(names(&text, "CaseTemperatureOutsideCard"), "{case_c}: {text}");
        }
    }
    for case_c in [-1.0e-3, -7.5, -20.0] {
        let (status, text) = post(&base, "/verify_physics", &step_body(0.0, 1.0, -1.0, case_c, 1.0)).await;
        assert_eq!(status, 200, "{case_c}: {text}");
    }

    // The chamber setting below zero: the two cold blocks are served, and a
    // negative setting that names no block is refused as not a block.
    for block_c in [-10.0, -20.0] {
        for case_c in [block_c + 2.0, 5.0] {
            let (status, text) = post(&base, "/verify_physics", &step_body(block_c, 1.0, -1.0, case_c, 1.0)).await;
            assert_eq!(status, 200, "{block_c}: {text}");
            let (status, text) = post(&base, "/run_trajectory", &run_body(block_c, 1.0, &[(3, -1.0, case_c)], 1)).await;
            assert_eq!(status, 200, "{block_c}: {text}");
        }
    }
    for block_c in [-15.0, -1.0e-3] {
        let (status, text) = post(&base, "/verify_physics", &step_body(block_c, 1.0, -1.0, 5.0, 1.0)).await;
        assert_eq!(status, 422, "{block_c}: {text}");
        assert!(names(&text, "BlockNotRecognized"), "{block_c}: {text}");
    }
    for block_c in [below(-20.0), -30.0, -273.15] {
        let (status, text) = post(&base, "/verify_physics", &step_body(block_c, 1.0, -1.0, 5.0, 1.0)).await;
        assert_eq!(status, 422, "{block_c}: {text}");
        assert!(names(&text, "BlockNotRecognized"), "{block_c}: {text}");
        assert!(
            text.contains("'block_chamber_c'") && text.contains("is below -20 degC, the inclusive minimum of the cell_2170_m80 card"),
            "{block_c}: {text}"
        );
        let (status, text) = post(&base, "/run_trajectory", &run_body(block_c, 1.0, &[(3, -1.0, 5.0)], 1)).await;
        assert_eq!(status, 422, "{block_c}: {text}");
        assert!(names(&text, "BlockNotRecognized"), "{block_c}: {text}");
    }
}

// ---------------------------------------------------------------- dataset

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_grid_over_blocks_and_currents_gives_rows_whose_run_seals_are_the_runs_alone() {
    let base = spawn_app().await;
    // Two blocks whose bands both hold a case temperature of 14 degC.
    let blocks = [10.0, 0.0];
    let currents = [-2.25, 0.0, 1.35];
    for store in ["frames", "none"] {
        let request = json!({
            "domain": "energy",
            "card": "cell_2170_m80",
            "plan": {
                "kind": "grid",
                "base": {
                    "initial": { "charge_removed_ah": 1.0 },
                    "dt_s": 1.0,
                    "segments": [{ "steps": 120, "load": { "case_temperature_c": 14.0 } }],
                    "record_every": 30
                },
                "axes": [
                    { "path": "initial.block_chamber_c", "values": blocks },
                    { "path": "segments[0].load.current_a", "values": currents }
                ]
            },
            "store": store,
            "return": "inline"
        });
        let (status, text) = post(&base, "/run_dataset", &request).await;
        assert_eq!(status, 200, "{text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        // The summary carries each row's seal, outcome and steps landed.
        let rows = answer["summary"].as_array().unwrap();
        assert_eq!(rows.len(), 6);
        assert_eq!(answer["head"]["completed"], 6);
        let mut index = 0;
        let mut seals = std::collections::BTreeSet::new();
        for block_c in blocks {
            for current_a in currents {
                let row = &rows[index];
                assert_eq!(row["outcome"], "completed", "{row}");
                let alone = run_body(block_c, 1.0, &[(120, current_a, 14.0)], 30);
                let (status, alone) = post(&base, "/run_trajectory", &alone).await;
                assert_eq!(status, 200, "{alone}");
                let alone: Value = serde_json::from_str(&alone).unwrap();
                assert_eq!(row["run_seal"], alone["run_seal"], "{store} {block_c} {current_a}");
                assert_eq!(row["steps_landed"], 120);
                seals.insert(row["run_seal"].as_str().unwrap().to_string());
                index += 1;
            }
        }
        assert_eq!(seals.len(), 6, "six runs, six seals");
        // The dataset checks.
        let (status, report) = post(
            &base,
            "/verify_dataset",
            &json!({ "request": request, "dataset_seal": answer["head"]["dataset_seal"] }),
        )
        .await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{report}");
    }
    // A grid axis with a value outside a published bound refuses the whole job.
    let (status, text) = post(
        &base,
        "/run_dataset",
        &json!({
            "domain": "energy", "card": "cell_2170_m80",
            "plan": { "kind": "grid",
                "base": { "initial": { "block_chamber_c": 25.0, "charge_removed_ah": 1.0 }, "dt_s": 1.0,
                          "segments": [{ "steps": 5, "load": { "case_temperature_c": 25.0 } }], "record_every": 1 },
                "axes": [{ "path": "segments[0].load.current_a", "values": [-1.0, 14.0] }] },
            "store": "none", "return": "inline", "answer": "full"
        }),
    )
    .await;
    // A grid is admitted row by row: the row past the bound is unadmitted
    // with the door's sentence and carries no run seal.
    assert_eq!(status, 200, "{text}");
    let grid_answer: Value = serde_json::from_str(&text).unwrap();
    let last = grid_answer["rows"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["outcome"], "unadmitted", "{text}");
    assert_eq!(last["run_seal"], Value::Null, "{text}");
    assert!(last["reject_detail"].is_string(), "{text}");
}

// ---------------------------------------------------------- validate_series

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_gives_every_second_of_a_run_and_carries_its_run_seal() {
    let base = spawn_app().await;
    let segments = [(40, -2.25, 10.5), (20, 0.0, 11.0), (10, 4.5, 11.5), (30, -6.0, 12.0)];
    let run = run_body(10.0, 0.8, &segments, 100);
    let (_, alone) = post(&base, "/run_trajectory", &run).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["outcome"], "completed");
    // The kernel loop's value of each quantity at each second.
    let cell = cell();
    let mut state = CellState::try_start(&cell, 10.0, 0.8).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut seconds: Vec<CellState> = Vec::new();
    for (steps, current_a, case_c) in segments {
        for _ in 0..steps {
            cell_2170_m80::step(
                &cell,
                &mut state,
                &mut frame,
                CellLoad { current_a, case_temperature_c: case_c },
                1.0,
            )
            .unwrap();
            seconds.push(state);
        }
    }
    let mut quantities = 0;
    for (index, (quantity, _)) in state_numbers(&state).into_iter().enumerate() {
        let probe = json!({
            "run": run, "quantity": quantity,
            "measurements": (1..=100).map(|step| json!({ "step": step, "value": 0.0 })).collect::<Vec<_>>()
        });
        let (status, text) = post(&base, "/validate_series", &probe).await;
        assert_eq!(status, 200, "{quantity}: {text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(answer["run_seal"], alone["run_seal"], "{quantity}");
        assert_eq!(answer["compared"], 100);
        assert_eq!(answer["not_reached"], 0);
        let model = answer["model_values_hex"].as_array().unwrap();
        for (second, hex) in model.iter().enumerate() {
            let value = f64::from_bits(u64::from_str_radix(hex.as_str().unwrap(), 16).unwrap());
            let kernel = state_numbers(&seconds[second])[index].1;
            assert_eq!(value.to_bits(), kernel.to_bits(), "{quantity} at second {}", second + 1);
        }
        quantities += 1;
    }
    assert_eq!(quantities, 21);

    // Measurements equal to the run give an error of exactly zero, and a
    // known offset is returned as that offset.
    let exact: Vec<Value> = seconds
        .iter()
        .enumerate()
        .map(|(second, state)| json!({ "step": second + 1, "value": state.terminal_v }))
        .collect();
    let (status, text) =
        post(&base, "/validate_series", &json!({ "run": run, "quantity": "terminal_v", "measurements": exact })).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rms_error"], 0.0);
    assert_eq!(answer["max_abs_error"], 0.0);
    let offset: Vec<Value> = seconds
        .iter()
        .enumerate()
        .map(|(second, state)| json!({ "step": second + 1, "value": state.terminal_v - 0.25 }))
        .collect();
    let (status, text) =
        post(&base, "/validate_series", &json!({ "run": run, "quantity": "terminal_v", "measurements": offset })).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert!((answer["mean_signed_error"].as_f64().unwrap() - 0.25).abs() < 1.0e-12);
    assert!(answer.get("pass").is_none() && answer.get("verdict").is_none());

    // A run that refuses part way compares the seconds it reached.
    let refused = run_body(25.0, 0.3, &[(400, 2.25, 26.0)], 400);
    let probe = json!({
        "run": refused, "quantity": "terminal_v",
        "measurements": [{ "step": 100, "value": 4.2 }, { "step": 189, "value": 4.2 }, { "step": 190, "value": 4.2 }]
    });
    let (status, text) = post(&base, "/validate_series", &probe).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["outcome"], "refused");
    assert_eq!(answer["reject"], "UpperVoltageLimit");
    assert_eq!(answer["steps_landed"], 189);
    assert_eq!(answer["compared"], 2);
    assert_eq!(answer["not_reached"], 1);
}

// ------------------------------------------------------------ the agent door

struct StdioDoor {
    _child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl StdioDoor {
    async fn start() -> StdioDoor {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut door = StdioDoor { _child: child, stdin, out };
        let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
        door.raw(&init.to_string()).await;
        door.stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .unwrap();
        door
    }

    async fn raw(&mut self, line: &str) -> String {
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.write_all(b"\n").await.unwrap();
        self.stdin.flush().await.unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).await.unwrap();
        answer.trim_end_matches('\n').to_string()
    }

    /// The text of a tool's result, and whether the door marked it an error.
    async fn tool(&mut self, name: &str, arguments: &Value) -> (String, bool) {
        let line = answer_text::tool_call(7, name, arguments);
        let answer: Value = serde_json::from_str(&self.raw(&line.to_string()).await).unwrap();
        let text = answer["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("no text in {answer}"))
            .to_string();
        (text, answer["result"]["isError"] == true)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_agent_door_lists_the_cell_and_gives_the_bytes_of_the_routes_for_the_four_tools() {
    let base = spawn_app().await;
    let mut door = StdioDoor::start().await;

    let (listed, error) = door.tool("list_cards", &json!({})).await;
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 37);
    let entry = cards.iter().find(|card| card["card"] == "cell_2170_m80").expect("the cell is listed");
    assert_eq!(entry["domain"], "energy");
    assert!(
        entry["specimen"].as_str().unwrap().starts_with("One Tesla Model 3 2170 lithium-ion cell, cell m80 alone, in six blocks of tests from -20 to 40 degC"),
        "{}", entry["specimen"]
    );
    assert!(entry["specimen"].as_str().unwrap().contains("17.4 mV"));

    let pair = json!({"domain": "energy", "card": "cell_2170_m80"});
    let (described, error) = door.tool("describe_card", &pair).await;
    assert!(!error);
    let (status, http) = post(&base, "/describe_card", &pair).await;
    assert_eq!(status, 200);
    assert_eq!(described, http);
    let description: Value = serde_json::from_str(&described).unwrap();
    assert!(description["validation"]["result"].as_str().unwrap().starts_with(
        "Compared with the 60 held-back tests on 2026-10-08, and by the rule fixed beforehand the comparison failed: 27 of 60 runs met their marks"
    ));
    // After the kernel's 13 names the list holds ContinuationNotOnCard; a step other than 1 s is the kernel's own StepNotOneSecond, so the two step names are not listed.
    assert_eq!(description["rejects"].as_array().unwrap().len(), 14);
    for reject in description["rejects"].as_array().unwrap() {
        assert!(reject["meaning"].as_str().is_some_and(|meaning| !meaning.is_empty()), "{reject}");
    }

    let step = step_body(0.0, 2.0, -4.5, 3.25, 1.0);
    let run = run_body(0.0, 0.5, &[(60, -1.35, 3.0), (30, 0.0, 3.5), (20, 0.5625, 4.0)], 10);
    let dataset = json!({
        "domain": "energy", "card": "cell_2170_m80",
        "plan": { "kind": "list", "runs": [
            { "initial": { "block_chamber_c": 25.0, "charge_removed_ah": 0.0 }, "dt_s": 1.0,
              "segments": [{ "steps": 50, "load": { "current_a": -2.25, "case_temperature_c": 25.0 } }], "record_every": 10 },
            { "initial": { "block_chamber_c": 15.0, "charge_removed_ah": 0.0 }, "dt_s": 1.0,
              "segments": [{ "steps": 50, "load": { "current_a": -2.25, "case_temperature_c": 25.0 } }], "record_every": 10 }
        ] },
        "store": "frames", "return": "inline", "answer": "full"
    });
    let series = json!({
        "run": run, "quantity": "terminal_v",
        "measurements": [{ "step": 1, "value": 3.9 }, { "step": 60, "value": 3.8 }, { "step": 110, "value": 4.0 }]
    });
    for (tool, route, arguments) in [
        ("verify_step", "/verify_physics", &step),
        ("run_trajectory", "/run_trajectory", &run),
        ("run_dataset", "/run_dataset", &dataset),
        ("validate_series", "/validate_series", &series),
    ] {
        let (text, error) = door.tool(tool, arguments).await;
        assert!(!error, "{tool}: {text}");
        let (status, http) = post(&base, route, arguments).await;
        assert_eq!(status, 200, "{tool}: {http}");
        assert_eq!(text, http, "{tool}");
    }
    // The dataset's second row names a chamber setting that is no block: it is
    // not admitted, and the first row ran.
    let (text, _) = door.tool("run_dataset", &dataset).await;
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed");
    assert_eq!(answer["rows"][1]["outcome"], "unadmitted");
    assert!(answer["rows"][1]["reject_detail"].as_str().unwrap().contains("BlockNotRecognized"));

    // A refusal on the agent door is a tool result marked as an error, with
    // the words of the HTTP door.
    let bad = step_body(25.0, 1.0, -14.0, 25.0, 1.0);
    let (text, error) = door.tool("verify_step", &bad).await;
    assert!(error);
    let (status, http) = post(&base, "/verify_physics", &bad).await;
    assert_eq!(status, 422);
    assert_eq!(text, http);
    assert!(names(&text, "CurrentOutsideCard"), "{text}");
}
