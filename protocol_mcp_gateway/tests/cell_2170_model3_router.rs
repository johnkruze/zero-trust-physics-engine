//! The physical card of the Tesla Model 3 2170 cell on the connections.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes, for both cells the card holds. A run ends in each of its five
//! sealed refusals (the lower and the upper voltage limit, the sustained
//! current, the state of charge, the end of the curves) on the step the
//! kernel loop stops at, with the kernel loop's last state to the bit. A run
//! shaped like a logged drive cycle, 6,000 segments of one step each with
//! another current and case temperature every second, is accepted as one
//! request and is the kernel loop's too. Every published bound is refused by
//! name at the last representable number on each side, and nothing is pulled
//! back to a bound. A missing input, a card constant sent as a field, a step
//! outside the card's floor and ceiling and a cell number that names no cell
//! are refused. A dataset's rows carry the run seals of the runs alone, and
//! `validate_series` carries the run seal of the run. The local connection
//! returns the bytes of the web connection for all four tools. The earlier
//! card of the same cell is served beside this one.
//!
//! No measurement of the cell is read here. The currents and temperatures are
//! numbers chosen for the test.

mod answer_text;

use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::cell_2170_model3::{self, Cell2170Model3, CellLoad, CellState, StepReject};
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

fn cell() -> Cell2170Model3 {
    Cell2170Model3::cell()
}

/// A start: the cell number, the state of charge and the capacity.
type Start = (f64, f64, f64);

/// Steps a discharge at 4.5 A from full lands at 25 degC, 4.5006 Ah, cell m80,
/// before its voltage would fall below 2.8 V.
const RATED_DISCHARGE_STEPS: u64 = 3_389;

/// The least and most case temperature a fresh start states: its first case temperature, held
/// to the card's case temperatures so that a load outside them is refused by its own name.
fn fresh_span(case_c: f64) -> f64 {
    case_c
}

fn step_body(start: Start, current_a: f64, case_c: f64, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": "cell_2170_model3",
        "parameters": {
            "cell_number": start.0,
            "state_of_charge": start.1,
            "capacity_ah": start.2,
            "case_temperature_least_c": fresh_span(case_c),
            "case_temperature_most_c": fresh_span(case_c),
            "current_a": current_a,
            "case_temperature_c": case_c,
            "dt_s": dt_s
        }
    })
}

/// A run of segments, each `(steps, current, case temperature)`.
fn run_body(start: Start, segments: &[(u64, f64, f64)], dt_s: f64, record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "cell_2170_model3",
        "initial": {
            "cell_number": start.0, "state_of_charge": start.1, "capacity_ah": start.2,
            "case_temperature_least_c": fresh_span(segments[0].2), "case_temperature_most_c": fresh_span(segments[0].2)
        },
        "dt_s": dt_s,
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

/// The kernel loop over the same segments, until the kernel refuses: the
/// last state, the steps landed, the refusal, the largest residual, and how
/// many landed steps carried the flag.
fn direct_loop(
    start: Start,
    segments: &[(u64, f64, f64)],
    dt_s: f64,
) -> (CellState, u64, Option<StepReject>, f64, u64) {
    let cell = cell();
    let mut state = CellState::try_rested(&cell, start.0, start.1, start.2).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut landed = 0;
    let mut worst = 0.0_f64;
    let mut flagged = 0;
    for (steps, current_a, case_c) in segments {
        let load = CellLoad {
            current_a: *current_a,
            case_temperature_c: *case_c,
        };
        for _ in 0..*steps {
            match cell_2170_model3::step(&cell, &mut state, &mut frame, load, dt_s) {
                Ok(()) => {
                    landed += 1;
                    worst = worst.max(state.residual_conservation_error);
                    if frame.flags & cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C != 0 {
                        flagged += 1;
                    }
                    assert_eq!(
                        frame.flags
                            & !(cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C
                                | cell_2170_model3::FLAG_OUTSIDE_COMPARED_RUNS),
                        0
                    );
                }
                Err(reject) => return (state, landed, Some(reject), worst, flagged),
            }
        }
    }
    (state, landed, None, worst, flagged)
}

/// The published number of a state, by its name, from the hex beside it.
fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn state_numbers(state: &CellState) -> Vec<(&'static str, f64)> {
    let lag = &state.diffusion_lag;
    vec![
        ("t_s", state.t_s),
        ("cell_number", state.cell_number),
        ("capacity_ah", state.capacity_ah),
        ("state_of_charge", state.state_of_charge),
        ("surface_state_of_charge", state.surface_state_of_charge),
        ("charge_removed_ah", state.charge_removed_ah),
        ("current_a", state.current_a),
        ("case_temperature_c", state.case_temperature_c),
        ("case_temperature_least_c", state.case_temperature_least_c),
        ("case_temperature_most_c", state.case_temperature_most_c),
        ("terminal_v", state.terminal_v),
        ("rest_v", state.rest_v),
        ("resistance_at_once_ohm", state.resistance_at_once_ohm),
        ("reaction_overpotential_v", state.reaction_overpotential_v),
        ("near_full_overpotential_v", state.near_full_overpotential_v),
        ("hysteresis_v", state.hysteresis_v),
        ("electrolyte_v", state.electrolyte_v),
        ("current_mean_a", state.current_mean_a),
        ("diffusion_lag_1", lag[0]),
        ("diffusion_lag_2", lag[1]),
        ("diffusion_lag_3", lag[2]),
        ("diffusion_lag_4", lag[3]),
        ("diffusion_lag_5", lag[4]),
        ("diffusion_lag_6", lag[5]),
        ("diffusion_lag_7", lag[6]),
        ("diffusion_lag_8", lag[7]),
        ("diffusion_lag_9", lag[8]),
        ("diffusion_lag_10", lag[9]),
        ("diffusion_lag_11", lag[10]),
        ("chemical_energy_j", state.chemical_energy_j),
        ("energy_anchor_j", state.energy_anchor_j),
        ("electrical_work_j", state.electrical_work_j),
        ("heat_j", state.heat_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
    ]
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// both as the decimal and as the hex beside it; the flag is the kernel's;
/// and nothing else is published.
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
    assert_eq!(object["sustained_above_1c"], state.sustained_above_1c, "{label}");
    keys.insert("sustained_above_1c".to_string());
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
        step_body((80.0, 1.0, 4.5), 4.5, 25.0, 1.0),
        step_body((1000.0, 1.0, 4.5), 4.5, 25.0, 1.0),
        step_body((448.0, 1.0, 4.5), 4.5, 25.0, 1.0),
        step_body((80.0, 0.6, 4.25), 10.0, 10.0, 1.0),
        step_body((80.0, 0.6, 4.25), -2.0, 10.0, 1.0),
        step_body((1000.0, 0.35, 4.25), 1.5, -12.9, 1.0),
        step_body((80.0, 0.5, 4.70), 0.0, 45.9, 1.0),
        step_body((80.0, 0.8, 4.35), 13.524, 30.0, 1.0),
        step_body((1000.0, 0.4, 4.3), -13.984, 20.0, 1.0),
    ];
    let mut seals = Vec::new();
    for request in cases {
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);

        // The compiled step, with the proof the gateway writes.
        let parameters = &request["parameters"];
        let number = |key: &str| parameters[key].as_f64().unwrap();
        let cell = cell();
        let mut state = CellState::try_rested(
            &cell,
            number("cell_number"),
            number("state_of_charge"),
            number("capacity_ah"),
        )
        .and_then(|rested| {
            rested.with_case_span(&cell, number("case_temperature_least_c"), number("case_temperature_most_c"))
        })
        .unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        cell_2170_model3::step(
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
        // One step from rest never reaches the flag.
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
        seals.push((response.sha256_seal, published(&response.state, "terminal_v")));
    }
    // Under the same step the second cell stands higher than the first and
    // the third higher than the second: each has less resistance.
    assert!(seals[1].1 > seals[0].1 + 0.003, "{} against {}", seals[1].1, seals[0].1);
    assert!(seals[2].1 > seals[1].1 + 0.003, "{} against {}", seals[2].1, seals[1].1);
    let mut only: Vec<&String> = seals.iter().map(|(seal, _)| seal).collect();
    only.sort();
    only.dedup();
    assert_eq!(only.len(), 9, "nine different steps, nine different seals");
}

/// A step at zero current from a rested state returns the rest voltage at
/// that state of charge, case temperature and capacity: what a caller reads
/// the state of charge before a recharge against.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_at_zero_current_returns_the_rest_voltage() {
    let base = spawn_app().await;
    let cell = cell();
    let mut last = 0.0;
    for z in [0.0, 0.02, 0.05, 0.08, 0.3, 0.7, 1.0] {
        let (status, answer) =
            post(&base, "/verify_physics", &step_body((80.0, z, 4.4), 0.0, 24.8, 1.0)).await;
        assert_eq!(status, 200, "{answer}");
        let response: Value = serde_json::from_str(&answer).unwrap();
        let terminal_v = published(&response["state"], "terminal_v");
        assert_eq!(terminal_v.to_bits(), published(&response["state"], "rest_v").to_bits());
        assert_eq!(terminal_v.to_bits(), cell.rest_v(0, 4.4, z, 24.8).unwrap().to_bits());
        for memory in ["reaction_overpotential_v", "hysteresis_v", "electrolyte_v", "diffusion_lag_1"] {
            assert_eq!(published(&response["state"], memory), 0.0, "{memory}");
        }
        assert!(terminal_v > last, "the rest voltage rises with the state of charge");
        last = terminal_v;
    }
}

// ---------------------------------------------------------------- the run

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_ends_in_each_sealed_refusal_on_the_step_the_kernel_loop_stops_at() {
    let base = spawn_app().await;
    // (start, segments, steps the card lands, the refusal, its sentence)
    let runs: Vec<(Start, Vec<(u64, f64, f64)>, u64, StepReject, &str)> = vec![
        (
            (80.0, 1.0, 4.5006),
            vec![(9_000, 4.5, 25.0)],
            RATED_DISCHARGE_STEPS,
            StepReject::UndervoltageLimit,
            "Physical Failure: the cell's voltage would fall below the 2.8 V lower limit (UndervoltageLimit). That state is not sealed.",
        ),
        (
            (80.0, 0.9, 4.5),
            vec![(9_000, -2.0, 25.0)],
            434,
            StepReject::OvervoltageLimit,
            "Physical Failure: the cell's voltage would rise above the 4.242 V upper limit (OvervoltageLimit). That state is not sealed.",
        ),
        (
            (80.0, 1.019, 4.17),
            vec![(5, 0.0, -12.6), (9_000, -9.74, -12.6)],
            5,
            StepReject::SurfaceOutsideCurve,
            "Physical Failure: the state of charge at the particle surface would leave [-0.05, 1.05], the ends of the card's rest-voltage curves, outside which it has no rest voltage (SurfaceOutsideCurve). That state is not sealed.",
        ),
    ];
    for (start, segments, expect_landed, expect, sentence) in runs {
        let asked: u64 = segments.iter().map(|segment| segment.0).sum();
        let request = run_body(start, &segments, 1.0, 500);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();

        let (state, landed, reject, worst, flagged) = direct_loop(start, &segments, 1.0);
        assert_eq!(reject, Some(expect));
        assert_eq!(landed, expect_landed, "{expect:?}");
        assert!(worst < 1.0e-13, "residual {worst}");
        assert_eq!(tape["outcome"], "refused");
        assert_eq!(tape["reject"], format!("{expect:?}"));
        assert_eq!(tape["reject_detail"], sentence.replace(" That state is not sealed.", &format!(" The run is sealed to step {}. The state at step {} was refused and is not sealed.", landed, landed + 1)));
        assert_eq!(tape["steps_landed"], landed);
        assert_eq!(tape["steps_requested"], asked);
        // The last landed state is the kernel loop's, to the bit, and it is
        // on the card: the refused state is not in it.
        let last = &tape["final_state"];
        assert_state_is(last, &state, &format!("{expect:?}"));
        let terminal_v = published(last, "terminal_v");
        assert!(terminal_v >= cell_2170_model3::LOWER_VOLTAGE_LIMIT_V);
        assert!(terminal_v <= cell_2170_model3::UPPER_VOLTAGE_LIMIT_V);
        let mean_a = published(last, "current_mean_a");
        assert!((-2.993..=8.722).contains(&mean_a), "{mean_a}");
        assert!((0.0..=1.02).contains(&published(last, "state_of_charge")));
        assert_eq!(published(last, "t_s"), landed as f64);
        assert_eq!(
            tape["max_residual_conservation_error"].as_f64().unwrap().to_bits(),
            worst.to_bits()
        );
        // None of these is above the rated current long enough to be flagged.
        assert_eq!(flagged, 0);

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

        // The steps the card lands, and no more, complete with the same state.
        let mut left = landed;
        let mut short_segments = Vec::new();
        for (steps, current_a, case_c) in &segments {
            let take = left.min(*steps);
            if take > 0 {
                short_segments.push((take, *current_a, *case_c));
            }
            left -= take;
        }
        let short = run_body(start, &short_segments, 1.0, landed);
        let (status, text) = post(&base, "/run_trajectory", &short).await;
        assert_eq!(status, 200, "{text}");
        let short: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(short["outcome"], "completed");
        assert_eq!(short["final_state"], tape["final_state"]);
    }
}

/// Five runs that engine 0.9.0 refused at an edge of the compared range, the
/// sustained current each way, a state of charge past 1.02 and past 0, and a
/// jump of case temperature past 5.4 K, run on past it with the flag, the
/// kernel loop to the bit, until a limit of the card's own stops each, and
/// their tapes hold.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runs_past_the_compared_range_go_on_flagged_to_a_limit_of_the_card() {
    let base = spawn_app().await;
    let runs: Vec<(Start, Vec<(u64, f64, f64)>, u64)> = vec![
        ((80.0, 1.0, 4.5006), vec![(9_000, 9.0, 25.0)], 208),
        ((80.0, 0.3, 4.5006), vec![(9_000, -3.2, 25.0)], 164),
        ((80.0, 1.0195, 4.5), vec![(9_000, -0.2, 25.0)], 40),
        ((80.0, 0.004, 4.6), vec![(9_000, 0.05, 40.0)], 1_324),
        ((80.0, 0.8, 4.25), vec![(10, 1.0, 10.0), (10, 1.0, 15.5)], 10),
    ];
    for (start, segments, old_landed) in runs {
        let (state, landed, reject, worst, _) = direct_loop(start, &segments, 1.0);
        assert!(landed > old_landed, "{start:?}: {landed}");
        assert!(worst < 1.0e-13, "residual {worst}");
        let request = run_body(start, &segments, 1.0, 1);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["steps_landed"], landed);
        assert_state_is(&tape["final_state"], &state, &format!("{start:?}"));
        match reject {
            Some(reject) => {
                assert_eq!(tape["outcome"], "refused");
                assert_eq!(tape["reject"], format!("{reject:?}"));
                assert!(matches!(
                    reject,
                    StepReject::UndervoltageLimit
                        | StepReject::OvervoltageLimit
                        | StepReject::SurfaceOutsideCurve
                        | StepReject::StateOfChargeOutsideCard
                ), "{reject:?}");
            }
            None => assert_eq!(tape["outcome"], "completed"),
        }
        let frames = tape["frames"].as_array().unwrap();
        let bytes = hex::decode(frames[old_landed as usize]["frame_hex"].as_str().unwrap()).unwrap();
        let flags = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
        assert_eq!(flags & cell_2170_model3::FLAG_OUTSIDE_COMPARED_RUNS, cell_2170_model3::FLAG_OUTSIDE_COMPARED_RUNS, "{start:?}");
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{report}");
    }
}

/// A held 9 A is flagged from the step on which the running mean of the
/// current passes 4.6 A, and the flagged frames seal.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_sustained_current_flag_is_set_from_the_step_the_running_mean_passes_and_it_seals() {
    let base = spawn_app().await;
    let start = (80.0, 1.0, 4.5006);
    let segments = [(100, 9.0, 25.0)];
    let (status, answer) = post(&base, "/run_trajectory", &run_body(start, &segments, 1.0, 1)).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed");
    let frames = tape["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 100);
    for (index, frame) in frames.iter().enumerate() {
        let bytes = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap();
        let flags = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
        let mean_a = f32::from_le_bytes(bytes[32..36].try_into().unwrap());
        // 9 (1 - exp(-t / 60)) passes 4.6 A between 42 s and 43 s.
        let expect = if index + 1 >= 43 { cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C } else { 0 };
        assert_eq!(flags, expect, "second {}", index + 1);
        assert_eq!(flags == 1, mean_a > 4.6, "second {}", index + 1);
    }
    assert_eq!(tape["final_state"]["sustained_above_1c"], true);
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true);
}

/// A current and a case temperature for each second of a made-up drive: a
/// slow swell, climbs, descents that charge the cell, and a fine ripple, with
/// the case warming and cooling by a few kelvin. Nothing measured.
fn made_up_drive(seconds: usize) -> Vec<(u64, f64, f64)> {
    let mut segments = Vec::with_capacity(seconds);
    let mut ripple: u64 = 0x2170;
    for k in 0..seconds {
        ripple = ripple.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let fine = ((ripple >> 40) as f64 / (1u64 << 24) as f64 - 0.5) * 0.6;
        let t = k as f64;
        let climb = (t / 211.0).sin().max(0.0);
        let descent = if k > 300 { (t / 53.0 + 1.0).sin().max(0.0) } else { 0.0 };
        let current_a =
            1.2 + 2.0 * (t / 37.0).sin() + 4.5 * climb * climb - 4.0 * descent * descent * descent + fine;
        let case_c = 27.0 + 3.0 * (t / 900.0).sin() + 0.01 * fine;
        // Rounded as a tester logs them: tenths of a milliampere and of a millikelvin.
        segments.push((1, (current_a * 1.0e4).round() / 1.0e4, (case_c * 1.0e4).round() / 1.0e4));
    }
    segments
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_drive_of_six_thousand_one_second_segments_is_one_request_and_is_the_kernel_loop() {
    let base = spawn_app().await;
    let start = (80.0, 1.0, 4.5);
    let segments = made_up_drive(6_000);
    assert!(segments.iter().any(|segment| segment.1 < -1.0), "the drive charges the cell");
    assert!(segments.iter().any(|segment| segment.1 > 7.0), "the drive climbs");
    let request = run_body(start, &segments, 1.0, 1);
    let request_text = serde_json::to_string(&request).unwrap();
    assert!(request_text.len() > 400_000, "{}", request_text.len());
    let (status, answer) = post_text(&base, "/run_trajectory", &request_text).await;
    assert_eq!(status, 200, "{}", &answer[..answer.len().min(400)]);
    let tape: Value = serde_json::from_str(&answer).unwrap();
    let (state, landed, reject, worst, flagged) = direct_loop(start, &segments, 1.0);
    assert_eq!(reject, None);
    assert_eq!(landed, 6_000);
    assert!(flagged > 0, "the climbs carry the flag");
    assert_eq!(tape["outcome"], "completed", "{:?}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 6_000);
    assert_eq!(tape["inputs"], request);
    assert_eq!(tape["frames"].as_array().unwrap().len(), 6_000);
    assert_state_is(&tape["final_state"], &state, "drive");
    assert!(worst < 1.0e-13, "residual {worst}");
    assert_eq!(
        tape["max_residual_conservation_error"].as_f64().unwrap().to_bits(),
        worst.to_bits()
    );
    assert!(state.state_of_charge < 0.7 && state.state_of_charge > 0.3);

    // Every frame carries the terminal voltage, the charge removed and the
    // flag of its second, and the kernel loop gives the same at that second.
    let cell = cell();
    let mut check = CellState::try_rested(&cell, start.0, start.1, start.2).unwrap();
    let mut frame = blank_frame([0; 16]);
    let frames = tape["frames"].as_array().unwrap();
    for (second, (_, current_a, case_c)) in segments.iter().enumerate() {
        cell_2170_model3::step(
            &cell,
            &mut check,
            &mut frame,
            CellLoad { current_a: *current_a, case_temperature_c: *case_c },
            1.0,
        )
        .unwrap();
        let bytes = hex::decode(frames[second]["frame_hex"].as_str().unwrap()).unwrap();
        let slot = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()).to_bits();
        assert_eq!(slot(8), (check.terminal_v as f32).to_bits(), "second {second}");
        assert_eq!(slot(12), (check.charge_removed_ah as f32).to_bits());
        assert_eq!(slot(16), (check.surface_state_of_charge as f32).to_bits());
        assert_eq!(slot(20), (check.reaction_overpotential_v as f32).to_bits());
        assert_eq!(slot(24), (check.hysteresis_v as f32).to_bits());
        assert_eq!(slot(28), (check.electrolyte_v as f32).to_bits());
        assert_eq!(slot(32), (check.current_mean_a as f32).to_bits());
        assert_eq!(slot(36), (check.residual_conservation_error as f32).to_bits());
        assert_eq!(u64::from_le_bytes(bytes[40..48].try_into().unwrap()), frame.flags, "flags");
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

    // Recorded once, the same run is the same run: the same state digest.
    let once = run_body(start, &segments, 1.0, 6_000);
    let (status, text) = post(&base, "/run_trajectory", &once).await;
    assert_eq!(status, 200);
    let once: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(once["final_state"], tape["final_state"]);
    assert_eq!(once["state_digest"], tape["state_digest"]);
    assert_eq!(once["frames"].as_array().unwrap().len(), 1);
}

// ------------------------------------------------------------------ bounds

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let (_, described) =
        post(&base, "/describe_card", &json!({"domain": "energy", "card": "cell_2170_model3"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    // The description publishes the card's own limits, not the compared edges.
    let bound = |list: &str, index: usize, key: &str| described[list][index][key].as_f64();
    assert_eq!(bound("phase_fields", 0, "minimum"), Some(80.0));
    assert_eq!(bound("phase_fields", 0, "maximum"), Some(1000.0));
    assert_eq!(bound("phase_fields", 1, "minimum"), Some(-0.05));
    assert_eq!(bound("phase_fields", 1, "maximum"), Some(1.05));
    assert_eq!(bound("phase_fields", 2, "minimum"), Some(0.0));
    assert_eq!(bound("phase_fields", 2, "maximum"), None);
    assert_eq!(bound("load_fields", 0, "minimum"), None);
    assert_eq!(bound("load_fields", 0, "maximum"), None);
    assert_eq!(bound("load_fields", 1, "minimum"), Some(-273.15));
    assert_eq!(bound("load_fields", 1, "maximum"), None);

    // (cell, state of charge, capacity, current, case temperature, the refusal named, or none)
    let cases: Vec<(f64, f64, f64, f64, f64, Option<&str>)> = vec![
        // The compared edges, and past them: answered.
        (80.0, 0.7, 4.25, 1.0, -12.9, None),
        (80.0, 0.7, 4.25, 1.0, below(-12.9), None),
        (80.0, 0.7, 4.6, 1.0, 45.9, None),
        (80.0, 0.7, 4.6, 1.0, above(45.9), None),
        (80.0, 0.7, 4.5, 1.0, -20.0, None),
        (80.0, 0.7, 4.5, 13.524, 25.0, None),
        (80.0, 0.7, 4.5, above(13.524), 25.0, None),
        (80.0, 0.5, 4.5, -13.984, 25.0, None),
        (80.0, 0.5, 4.5, below(-13.984), 25.0, None),
        (80.0, 0.7, 4.16, 1.0, -10.0, None),
        (80.0, 0.7, below(4.16), 1.0, -10.0, None),
        (80.0, 0.7, above(4.70), 1.0, 40.0, None),
        (80.0, 0.7, 4.16, 1.0, above(12.6), None),
        (80.0, 1.0, 4.70, 4.5, -12.9, None),
        (80.0, 1.0, 4.6768, 4.5, 25.0, None),
        (80.0, -1.0e-9, 4.5, 0.0, 25.0, None),
        (80.0, above(1.02), 4.5, 0.0, 25.0, None),
        // The card's own limits.
        (80.0, 0.7, 0.0, 1.0, 25.0, Some("CapacityNotPositive")),
        (80.0, 0.7, -4.5, 1.0, 25.0, Some("CapacityNotPositive")),
        (80.0, 0.7, 4.5, 1.0, -273.15, Some("CaseTemperatureBelowAbsoluteZero")),
        (80.0, 0.7, 4.5, 1.0, -300.0, Some("CaseTemperatureBelowAbsoluteZero")),
        (80.0, below(-0.05), 4.5, 0.0, 25.0, Some("StateOfChargeOutsideCard")),
        (80.0, above(1.05), 4.5, 0.0, 25.0, Some("StateOfChargeOutsideCard")),
        // The cell: the cells the card holds, one number beyond each, and
        // numbers between them that name no cell and are not rounded to one.
        (80.0, 0.7, 4.5, 1.0, 25.0, None),
        (1000.0, 0.7, 4.5, 1.0, 25.0, None),
        (below(80.0), 0.7, 4.5, 1.0, 25.0, Some("CellNotRecognized")),
        (above(1000.0), 0.7, 4.5, 1.0, 25.0, Some("CellNotRecognized")),
        (above(80.0), 0.7, 4.5, 1.0, 25.0, Some("CellNotRecognized")),
        (448.0, 0.7, 4.5, 1.0, 25.0, None),
        (449.0, 0.7, 4.5, 1.0, 25.0, Some("CellNotRecognized")),
    ];
    for (cell_number, z, capacity_ah, current_a, case_c, refusal) in cases {
        let start = (cell_number, z, capacity_ah);
        let label = format!("cell {cell_number} z {z} capacity {capacity_ah} current {current_a} case {case_c}");
        let step = step_body(start, current_a, case_c, 1.0);
        let run = run_body(start, &[(3, current_a, case_c)], 1.0, 1);
        let (step_status, step_text) = post(&base, "/verify_physics", &step).await;
        let (run_status, run_text) = post(&base, "/run_trajectory", &run).await;
        match refusal {
            None => {
                assert_eq!(step_status, 200, "{label}: {step_text}");
                assert_eq!(run_status, 200, "{label}: {run_text}");
                let response: Value = serde_json::from_str(&step_text).unwrap();
                assert_eq!(published(&response["state"], "capacity_ah").to_bits(), capacity_ah.to_bits());
                assert_eq!(published(&response["state"], "current_a").to_bits(), current_a.to_bits());
                assert_eq!(published(&response["state"], "case_temperature_c").to_bits(), case_c.to_bits());
                let tape: Value = serde_json::from_str(&run_text).unwrap();
                assert!(tape["run_seal"].is_string(), "{label}: {run_text}");
            }
            Some(name) => {
                for (status, text) in [(step_status, &step_text), (run_status, &run_text)] {
                    assert_eq!(status, 422, "{label}: {text}");
                    assert!(names(text, name), "{label}: {text}");
                }
            }
        }
    }

    // The words of a refusal: the field, the value, the bound and the name.
    let (_, text) = post(&base, "/verify_physics", &step_body((449.0, 0.7, 4.5), 1.0, 25.0, 1.0)).await;
    assert_eq!(
        text,
        "Physical Failure: 'cell_number' 449 is not one of the cells of the cell_2170_model3 card, 80 (cell m80), 1000 (cell m1000) and 448 (cell m448-N) (CellNotRecognized). That state is not sealed."
    );
    let (_, text) = post(&base, "/verify_physics", &step_body((80.0, 0.7, 0.0), 1.0, 25.0, 1.0)).await;
    assert_eq!(
        text,
        "Physical Failure: 'capacity_ah' 0 Ah is at or below 0 Ah, the exclusive minimum of the cell_2170_model3 card (CapacityNotPositive). That state is not sealed."
    );

    // A span of case temperature over the segments past 19.2 K is answered,
    // flagged; the run's least and most are published.
    let span_steps = [(3, 1.0, 0.0), (3, 1.0, 5.0), (3, 1.0, 10.0), (3, 1.0, 15.0), (3, 1.0, 19.2), (3, 1.0, 24.0)];
    let run = run_body((80.0, 0.7, 4.25), &span_steps, 1.0, 1);
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{text}");
    assert_eq!(published(&tape["final_state"], "case_temperature_least_c"), 0.0);
    assert_eq!(published(&tape["final_state"], "case_temperature_most_c"), 24.0);
    let frames = tape["frames"].as_array().unwrap();
    let flags_at = |index: usize| {
        let bytes = hex::decode(frames[index]["frame_hex"].as_str().unwrap()).unwrap();
        u64::from_le_bytes(bytes[40..48].try_into().unwrap())
    };
    assert_eq!(flags_at(14), 0);
    assert_eq!(flags_at(15), cell_2170_model3::FLAG_OUTSIDE_COMPARED_RUNS);
    // A carried pair that is not a fresh start's is refused as a continuation
    // without its upstream, and an inverted pair by its own name.
    let returned_soc = published(&tape["final_state"], "state_of_charge");
    let continued = |segments: &[(u64, f64, f64)], least: f64, most: f64| {
        let mut run = run_body((80.0, returned_soc, 4.25), segments, 1.0, 1);
        run["initial"]["case_temperature_least_c"] = json!(least);
        run["initial"]["case_temperature_most_c"] = json!(most);
        run
    };
    let (status, text) = post(&base, "/run_trajectory", &continued(&[(3, 1.0, 19.2)], 0.0, 19.2)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream).") && !text.contains("run_seal"), "{text}");
    let (status, text) = post(&base, "/run_trajectory", &continued(&[(3, 1.0, 9.5)], 10.0, 9.0)).await;
    assert_eq!(status, 422, "{text}");
    assert!(names(&text, "CaseTemperatureSpanInverted"), "{text}");

    // The step: 1 s, and no other.
    let run = run_body((80.0, 0.7, 4.5), &[(3, 1.0, 25.0)], 1.0, 1);
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{text}");
    for (dt_s, words) in [
        (below(1.0), "the step floor of the cell_2170_model3 card"),
        (0.5, "the step floor of the cell_2170_model3 card"),
        (above(1.0), "the step ceiling of the cell_2170_model3 card"),
        (10.0, "the step ceiling of the cell_2170_model3 card"),
        (0.0, "NonPositiveDt"),
        (-1.0, "NonPositiveDt"),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body((80.0, 0.7, 4.5), 1.0, 25.0, dt_s)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains(words) && !text.contains("sha256_seal"), "{dt_s}: {text}");
    }
}

/// Requests engine 0.9.0 refused at an edge of the compared range are
/// answered, flagged: a new cell in the cold, an aged cell when hot and a
/// jump of 58.8 K run until a limit of the card stops them, with their
/// tapes holding. The reversal at 0.1 s is refused below the floor.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn requests_engine_0_9_0_refused_at_the_compared_edges_are_answered() {
    let base = spawn_app().await;
    let reversal = run_body((80.0, 0.5, 4.25), &[(50, 4.5, 0.0), (50, -4.5, 0.0)], 0.1, 10);
    let (status, text) = post(&base, "/run_trajectory", &reversal).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("the step floor of the cell_2170_model3 card"), "{text}");
    for run in [
        run_body((80.0, 1.0, 4.70), &[(600, 4.5, -12.9)], 1.0, 600),
        run_body((80.0, 1.0, 4.16), &[(600, 4.5, 45.9)], 1.0, 600),
        run_body((80.0, 0.6, 4.5), &[(300, 2.0, -12.9), (300, 2.0, 45.9)], 1.0, 600),
        run_body((80.0, 0.6, 4.25), &[(300, 2.0, 5.0), (300, 2.0, 10.5)], 1.0, 600),
    ] {
        let (status, text) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert!(tape["steps_landed"].as_u64().unwrap() > 0, "{text}");
        let (_, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_input_a_non_number_and_a_card_constant_are_refused_and_nothing_is_filled_in() {
    let base = spawn_app().await;
    let good = step_body((80.0, 0.7, 4.5), 1.5, 25.0, 1.0);
    let (status, _) = post(&base, "/verify_physics", &good).await;
    assert_eq!(status, 200);

    // Each of the six inputs, missing. No capacity, no cell and no state of
    // charge is assumed.
    for key in [
        "cell_number",
        "state_of_charge",
        "capacity_ah",
        "case_temperature_least_c",
        "case_temperature_most_c",
        "current_a",
        "case_temperature_c",
        "dt_s",
    ] {
        let mut step = good.clone();
        step["parameters"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}' (InputMissing)."));
    }
    // A run with a load that lacks the case temperature, or a start that lacks a field.
    let mut run = run_body((80.0, 0.7, 4.5), &[(3, 1.0, 25.0), (3, 0.0, 25.0)], 1.0, 1);
    run["segments"][1]["load"].as_object_mut().unwrap().remove("case_temperature_c");
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("Missing required parameter: 'case_temperature_c' (InputMissing)."), "{text}");
    for key in ["cell_number", "state_of_charge", "capacity_ah", "case_temperature_least_c", "case_temperature_most_c"] {
        let mut run = run_body((80.0, 0.7, 4.5), &[(3, 1.0, 25.0)], 1.0, 1);
        run["initial"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains(&format!("Missing required parameter: '{key}' (InputMissing).")), "{text}");
    }

    // A value that is not a number, and a number that is not finite.
    for (key, raw) in [
        ("current_a", "\"1.5\""),
        ("case_temperature_c", "null"),
        ("cell_number", "\"m80\""),
        ("capacity_ah", "[4.5]"),
        ("state_of_charge", "true"),
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
        let body = serde_json::to_string(&good).unwrap().replace("\"current_a\":1.5", &format!("\"current_a\":{raw}"));
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

    // A card constant, a memory or a ledger entry sent as a field is not a
    // field of the request: a run starts rested and anchored, always.
    for constant in [
        "resistance_scale",
        "reference_capacity_ah",
        "resistance_fixed_ohm",
        "exchange_current_a",
        "diffusion_time_s",
        "rest_v",
        "lower_voltage_limit_v",
        "upper_voltage_limit_v",
        "sustained_discharge_max_a",
        "hysteresis_v",
        "electrolyte_v",
        "current_mean_a",
        "diffusion_lag_1",
        "electrical_work_j",
        "block_chamber_c",
        "charge_removed_ah",
        "residual_conservation_limit",
    ] {
        let mut step = good.clone();
        step["parameters"][constant] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert_eq!(text, format!("Physical Violation: '{constant}' is not a field of this request."));

        let mut in_phase = run_body((80.0, 0.7, 4.5), &[(3, 1.0, 25.0)], 1.0, 1);
        in_phase["initial"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_phase).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");

        let mut in_load = run_body((80.0, 0.7, 4.5), &[(3, 1.0, 25.0)], 1.0, 1);
        in_load["segments"][0]["load"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");
    }
}

// ---------------------------------------------------------------- dataset

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_grid_over_cells_and_currents_gives_rows_whose_run_seals_are_the_runs_alone() {
    let base = spawn_app().await;
    let cells = [80.0, 1000.0, 448.0];
    let currents = [4.5, 0.0, -1.5];
    for store in ["frames", "none"] {
        let request = json!({
            "domain": "energy",
            "card": "cell_2170_model3",
            "plan": {
                "kind": "grid",
                "base": {
                    "initial": { "state_of_charge": 0.8, "capacity_ah": 4.25, "case_temperature_least_c": 12.5, "case_temperature_most_c": 12.5 },
                    "dt_s": 1.0,
                    "segments": [{ "steps": 120, "load": { "case_temperature_c": 12.5 } }],
                    "record_every": 30
                },
                "axes": [
                    { "path": "initial.cell_number", "values": cells },
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
        assert_eq!(rows.len(), 9);
        assert_eq!(answer["head"]["completed"], 9);
        let mut index = 0;
        let mut seals = std::collections::BTreeSet::new();
        for cell_number in cells {
            for current_a in currents {
                let row = &rows[index];
                assert_eq!(row["outcome"], "completed", "{row}");
                let alone = run_body((cell_number, 0.8, 4.25), &[(120, current_a, 12.5)], 1.0, 30);
                let (status, alone) = post(&base, "/run_trajectory", &alone).await;
                assert_eq!(status, 200, "{alone}");
                let alone: Value = serde_json::from_str(&alone).unwrap();
                assert_eq!(row["run_seal"], alone["run_seal"], "{store} {cell_number} {current_a}");
                assert_eq!(row["steps_landed"], 120);
                seals.insert(row["run_seal"].as_str().unwrap().to_string());
                index += 1;
            }
        }
        assert_eq!(seals.len(), 9, "nine runs, nine seals");
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
            "domain": "energy", "card": "cell_2170_model3",
            "plan": { "kind": "grid",
                "base": { "initial": { "cell_number": 80.0, "state_of_charge": 0.8, "capacity_ah": 4.4, "case_temperature_least_c": 25.0, "case_temperature_most_c": 25.0 }, "dt_s": 1.0,
                          "segments": [{ "steps": 5, "load": { "current_a": 1.0 } }], "record_every": 1 },
                "axes": [{ "path": "segments[0].load.case_temperature_c", "values": [25.0, -300.0] }] },
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
    let start = (1000.0, 0.8, 4.29);
    let segments = [(40, 2.25, 9.0), (20, 0.0, 9.5), (10, -4.5, 10.0), (30, 6.0, 10.5)];
    let run = run_body(start, &segments, 1.0, 100);
    let (_, alone) = post(&base, "/run_trajectory", &run).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["outcome"], "completed");
    // The kernel loop's value of each quantity at each second.
    let cell = cell();
    let mut state = CellState::try_rested(&cell, start.0, start.1, start.2).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut seconds: Vec<CellState> = Vec::new();
    for (steps, current_a, case_c) in segments {
        for _ in 0..steps {
            cell_2170_model3::step(
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
    assert_eq!(quantities, 35);

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
    let refused = run_body((80.0, 0.9, 4.5), &[(600, -2.0, 25.0)], 1.0, 600);
    let probe = json!({
        "run": refused, "quantity": "terminal_v",
        "measurements": [{ "step": 100, "value": 4.2 }, { "step": 434, "value": 4.2 }, { "step": 435, "value": 4.2 }]
    });
    let (status, text) = post(&base, "/validate_series", &probe).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["outcome"], "refused");
    assert_eq!(answer["reject"], "OvervoltageLimit");
    assert_eq!(answer["steps_landed"], 434);
    assert_eq!(answer["compared"], 2);
    assert_eq!(answer["not_reached"], 1);
}

// ------------------------------------------------------ the local connection

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

    /// The text of a tool's result, and whether it was marked an error.
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
async fn the_local_connection_lists_the_cell_and_gives_the_bytes_of_the_routes_for_the_four_tools() {
    let base = spawn_app().await;
    let mut door = StdioDoor::start().await;

    let (listed, error) = door.tool("list_cards", &json!({})).await;
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 37);
    let entry = &cards[16];
    assert_eq!(entry["card"], "cell_2170_model3");
    assert_eq!(entry["domain"], "energy");
    let specimen = entry["specimen"].as_str().unwrap();
    assert!(specimen.starts_with("One Tesla Model 3 2170 lithium-ion cell, any of the three cells m80, m1000 and m448-N of its dataset, as a physical model with memory"), "{specimen}");
    assert!(specimen.contains("cell_2170_m80 reads 17.4 mV higher"), "{specimen}");
    // The table card of the same cell is served beside it, and names this one.
    let earlier = cards.iter().find(|card| card["card"] == "cell_2170_m80").expect("the earlier card is listed");
    assert!(earlier["specimen"].as_str().unwrap().contains("cell_2170_model3 reads 17.4 mV lower"), "{}", earlier["specimen"]);

    let pair = json!({"domain": "energy", "card": "cell_2170_model3"});
    let (described, error) = door.tool("describe_card", &pair).await;
    assert!(!error);
    let (status, http) = post(&base, "/describe_card", &pair).await;
    assert_eq!(status, 200);
    assert_eq!(described, http);
    let description: Value = serde_json::from_str(&described).unwrap();
    let record = description["validation"].as_object().unwrap();
    let parts: Vec<&str> = record.keys().map(String::as_str).collect();
    assert_eq!(parts, ["object", "data", "pass_line", "result", "limits"]);
    let result = record["result"].as_str().unwrap();
    assert!(result.starts_with("Run once, on 2026-10-09, on runs of two further cells of the same type that no model had seen"), "{result}");
    assert!(result.contains("51 of 60") && result.contains("31 of 45") && result.contains("untested on new cells"), "{result}");
    assert!(record["limits"].as_str().unwrap().contains("are now spent"));
    // After the kernel's 14 names the list holds the gateway's own four for this card, DtBelowFloor and DtAboveCeiling, ContinuationWithoutUpstream and ContinuationNotOnCard.
    assert_eq!(description["rejects"].as_array().unwrap().len(), 18);
    for reject in description["rejects"].as_array().unwrap() {
        assert!(reject["meaning"].as_str().is_some_and(|meaning| !meaning.is_empty()), "{reject}");
    }
    assert_eq!(description["advisory_flags"][0]["name"], "FLAG_SUSTAINED_ABOVE_1C");
    // The worked examples run as written.
    let (text, error) = door.tool("verify_step", &description["example_compute_step"]).await;
    assert!(!error, "{text}");
    let (text, error) = door.tool("run_trajectory", &description["example_run_trajectory"]).await;
    assert!(!error, "{text}");
    let example: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(example["outcome"], "completed");
    assert_eq!(example["steps_landed"], 600);
    assert_eq!(example["frames"].as_array().unwrap().len(), 10);

    let step = step_body((1000.0, 0.62, 4.25), 7.0, -3.5, 1.0);
    let run = run_body((80.0, 0.9, 4.3), &[(60, 7.0, 2.0), (30, 0.0, 2.5), (20, -1.0, 3.0)], 1.0, 10);
    let dataset = json!({
        "domain": "energy", "card": "cell_2170_model3",
        "plan": { "kind": "list", "runs": [
            { "initial": { "cell_number": 80.0, "state_of_charge": 1.0, "capacity_ah": 4.5, "case_temperature_least_c": 25.0, "case_temperature_most_c": 25.0 }, "dt_s": 1.0,
              "segments": [{ "steps": 50, "load": { "current_a": 2.25, "case_temperature_c": 25.0 } }], "record_every": 10 },
            { "initial": { "cell_number": 449.0, "state_of_charge": 1.0, "capacity_ah": 4.5, "case_temperature_least_c": 25.0, "case_temperature_most_c": 25.0 }, "dt_s": 1.0,
              "segments": [{ "steps": 50, "load": { "current_a": 2.25, "case_temperature_c": 25.0 } }], "record_every": 10 }
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
    // The dataset's second row names a cell the card does not hold: it is not
    // admitted, and the first row ran.
    let (text, _) = door.tool("run_dataset", &dataset).await;
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed");
    assert_eq!(answer["rows"][1]["outcome"], "unadmitted");
    assert!(answer["rows"][1]["reject_detail"].as_str().unwrap().contains("CellNotRecognized"));

    // A refusal on the local connection is a tool result marked as an error,
    // with the words of the web connection.
    let bad = step_body((80.0, 0.7, 0.0), 1.0, 25.0, 1.0);
    let (text, error) = door.tool("verify_step", &bad).await;
    assert!(error);
    let (status, http) = post(&base, "/verify_physics", &bad).await;
    assert_eq!(status, 422);
    assert_eq!(text, http);
    assert!(names(&text, "CapacityNotPositive"), "{text}");
}
