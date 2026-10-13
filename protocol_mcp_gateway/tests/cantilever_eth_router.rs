//! The ETH Zürich aluminium cantilever card on the three connections.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes, and its receipt verifies. The first second from the start of the
//! scored ring-down, 4,000 steps, ends on the kernel loop's state to the bit,
//! and its first 3,405 steps end on the reference numbers of the research
//! record. The compared run's own start is the top of the amplitude range
//! and is accepted: the run lands 94,731 steps and is refused below the
//! floor; from the start of the scored ring-down the run lands 94,136 steps
//! and is refused below the floor, and its tape verifies. Each refusal a
//! request can reach is named and carries no proof: a deflection or a speed
//! past its own bound, the two together past the amplitude ceiling or under
//! its floor, a step of zero or less, a step outside the card's floor and
//! ceiling, a missing input, a load the card does not have. A start near the
//! ceiling that the card refuses part way, and a ring-down refused at the
//! floor, are runs that keep their proof up to the last step that landed. A
//! dataset's rows carry the run seals of the runs alone, and `validate_series`
//! compares the deflection. The local, the web and the hosted connection
//! return the same bytes.
//!
//! No measurement of the beam is read here. The start of the compared run is
//! one number of the opened half of the held-back recording, taken from the
//! research record, and the start of the scored ring-down is the card's own
//! state 595 steps later; every other start is a round number chosen for the
//! test.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::cantilever_eth::{
    self, CantileverEth, CantileverLoad, CantileverState, StepReject,
};
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
                return (status, response.text().await.unwrap());
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
    post_text(base, path, &serde_json::to_string(body).unwrap()).await
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The next 64-bit number toward zero from a positive `x`.
fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

/// The next 64-bit number away from zero from a positive `x`.
fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

fn beam() -> CantileverEth {
    CantileverEth::first_bending_mode()
}

fn ceiling() -> f64 {
    beam().gate_dt_s().unwrap()
}

fn floor() -> f64 {
    beam().floor_dt_s().unwrap()
}

const NO_LOAD: CantileverLoad = CantileverLoad {};

/// The start of the compared run: 0.150 s before the start of the held-back
/// ring-down, on the model's own trajectory. `validation_protocol.md`,
/// section 9, and `card_spec_draft.md`, section 8, test 2. Its envelope,
/// 15.786 mm, is the largest the compared run held, under the ceiling.
const COMPARED_START_DEFLECTION_M: f64 = -1.526315579382e-2;
const COMPARED_START_VELOCITY_M_S: f64 = 2.099361936006e-1;

/// The start of the scored ring-down: the compared run's state after 595
/// steps of 0.25 ms, by the card's own map. The description's worked examples
/// start here.
const START_DEFLECTION_M: f64 = -0.00104914043915228;
const START_VELOCITY_M_S: f64 = 0.7408030982848752;

/// The amplitude ceiling of the card, 15.79 mm.
const CEILING_M: f64 = cantilever_eth::AMPLITUDE_CEILING_M;

/// The amplitude floor of the card, 0.058 mm.
const FLOOR_M: f64 = cantilever_eth::AMPLITUDE_FLOOR_M;

/// The largest speed at zero deflection whose envelope, by the card's own
/// function, is at or under the amplitude ceiling.
fn largest_speed_m_s() -> f64 {
    let beam = beam();
    let inside = |speed_m_s: f64| beam.envelope_m(0.0, speed_m_s) <= CEILING_M;
    let mut speed_m_s = CEILING_M * beam.omega0_rad_s();
    while !inside(speed_m_s) {
        speed_m_s = below(speed_m_s);
    }
    while inside(above(speed_m_s)) {
        speed_m_s = above(speed_m_s);
    }
    speed_m_s
}

fn step_body(deflection_m: f64, velocity_m_s: f64, dt_s: f64) -> Value {
    json!({
        "domain": "signal",
        "card": "cantilever_eth",
        "parameters": {
            "deflection_m": deflection_m,
            "velocity_m_s": velocity_m_s,
            "dt_s": dt_s
        }
    })
}

fn run_body(deflection_m: f64, velocity_m_s: f64, dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "signal",
        "card": "cantilever_eth",
        "initial": { "deflection_m": deflection_m, "velocity_m_s": velocity_m_s },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": {} }],
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

/// What the kernel loop gives: the last landed state, the steps that landed,
/// the refusal that stopped it, the largest residual, and the state and the
/// frame at every `keep_every`-th step.
struct Direct {
    state: CantileverState,
    landed: u64,
    reject: Option<StepReject>,
    worst: f64,
    kept: Vec<(CantileverState, LastStateFrame64)>,
}

/// The kernel loop: steps of `dt_s` from this start until the kernel refuses.
fn direct_loop(deflection_m: f64, velocity_m_s: f64, dt_s: f64, steps: u64, keep_every: u64) -> Direct {
    let beam = beam();
    let mut state = CantileverState::try_phase(&beam, deflection_m, velocity_m_s).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut direct = Direct { state, landed: 0, reject: None, worst: 0.0, kept: Vec::new() };
    while direct.landed < steps {
        match cantilever_eth::step(&beam, &mut state, &mut frame, NO_LOAD, dt_s) {
            Ok(()) => {
                direct.landed += 1;
                direct.worst = direct.worst.max(state.residual_conservation_error);
                if direct.landed % keep_every == 0 {
                    direct.kept.push((state, frame));
                }
            }
            Err(reject) => {
                direct.reject = Some(reject);
                break;
            }
        }
    }
    direct.state = state;
    direct
}

/// The published number of a state, by its name, from the hex beside it.
fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn state_numbers(state: &CantileverState) -> Vec<(&'static str, f64)> {
    vec![
        ("t_s", state.t_s),
        ("deflection_m", state.deflection_m),
        ("velocity_m_s", state.velocity_m_s),
        ("envelope_m", state.envelope_m),
        ("kinetic_j_per_kg", state.kinetic_j_per_kg),
        ("potential_j_per_kg", state.potential_j_per_kg),
        ("stored_j_per_kg", state.stored_j_per_kg),
        ("energy_anchor_j_per_kg", state.energy_anchor_j_per_kg),
        ("heat_j_per_kg", state.heat_j_per_kg),
        ("energy_defect_j_per_kg", state.energy_defect_j_per_kg),
        ("residual_conservation_error", state.residual_conservation_error),
    ]
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// both as the decimal and as the hex beside it, and nothing else is
/// published.
fn assert_state_is(published_state: &Value, state: &CantileverState, label: &str) {
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

/// The refusal is named and the answer carries no proof.
fn names(text: &str, name: &str) -> bool {
    text.contains(&format!("({name}). That state is not sealed."))
        && !text.contains("sha256_seal")
        && !text.contains("run_seal")
}

fn relative(got: f64, expected: f64) -> f64 {
    ((got - expected) / expected).abs()
}

/// The slots of one recorded frame: deflection, speed, envelope, the margin
/// to the amplitude ceiling, and the flag word.
fn slots(frame_hex: &str) -> (f64, f32, f32, f32, f32, u64) {
    let bytes = hex::decode(frame_hex).unwrap();
    assert_eq!(bytes.len(), 64);
    let f32_at = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    (
        f64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        f32_at(8),
        f32_at(12),
        f32_at(16),
        f32_at(32),
        u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
    )
}

const ENVELOPE_ABOVE_THE_CEILING: &str = "Physical Failure: the deflection envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2), is above 0.01579 m, the largest the compared run held (DeflectionAboveCard). That state is not sealed.";
const ENVELOPE_BELOW_THE_FLOOR: &str = "Physical Failure: the deflection envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2), is below 5.8e-5 m, the smallest the card was compared at (DeflectionBelowCard). That state is not sealed.";

/// The sentence of a run refused part way at the floor: the card's words,
/// ending with the refused step and the step the run is sealed to.
fn below_the_floor_at(landed: u64) -> String {
    format!(
        "Physical Failure: the deflection envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2), is below 5.8e-5 m, the smallest the card was compared at (DeflectionBelowCard). The run is sealed to step {landed}. The state at step {} was refused and is not sealed.",
        landed + 1
    )
}

// -------------------------------------------------------------- the step

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_beam_matches_the_compiled_step_with_the_same_proof_bytes_and_its_receipt_verifies() {
    let base = spawn_app().await;
    let cases = [
        // The start of the scored ring-down, at the ceiling.
        step_body(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling()),
        // At rest at the amplitude ceiling, on either side.
        step_body(CEILING_M, 0.0, ceiling()),
        step_body(-CEILING_M, 0.0, 1.0e-4),
        // Through the rest position, at the floor.
        step_body(0.0, -0.4, floor()),
        // Just above the amplitude floor.
        step_body(6.5e-5, 0.0, ceiling()),
        // At rest at the amplitude ceiling, at the step floor.
        step_body(CEILING_M, 0.0, floor()),
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
        let beam = beam();
        let mut state =
            CantileverState::try_phase(&beam, number("deflection_m"), number("velocity_m_s")).unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        cantilever_eth::step(&beam, &mut state, &mut frame, NO_LOAD, number("dt_s")).unwrap();
        assert_eq!(frame.flags, 0);
        assert_state_is(&response.state, &state, "step");
        assert_eq!(
            response.residual_conservation_error.to_bits(),
            state.residual_conservation_error.to_bits()
        );
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        // The proof covers the request: the same frame with a zero proof seals apart.
        let mut naked = frame;
        naked.proof = [0; 16];
        assert_ne!(response.sha256_seal, sha256_hex(&naked.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha256);
        assert_eq!(sha256_hex(document.as_bytes()), receipt);
        // The answer verifies as it was returned, and not with one digit changed.
        let (status, report) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], true, "{report}");
        let changed = answer.replacen("\"t_s\":", "\"t_s\":1", 1);
        assert_ne!(changed, answer);
        let (_, report) = post(&base, "/verify_receipt", &json!({ "response_text": changed })).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], false, "{report}");
        // The same request again gives the same bytes.
        let (_, again) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(again, answer);
        seals.push(response.sha256_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 6, "six different steps, six different proofs");
}

/// A single step handed back as a new start continues the same motion: the
/// deflection and the speed after two steps taken one at a time are those of
/// a run of two steps, to the bit. The time and the heat begin again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_handed_back_as_a_new_start_continues_the_same_motion() {
    let base = spawn_app().await;
    let (_, first) = post(&base, "/verify_physics", &step_body(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling())).await;
    let first: Value = serde_json::from_str(&first).unwrap();
    let handed_back = step_body(
        first["state"]["deflection_m"].as_f64().unwrap(),
        first["state"]["velocity_m_s"].as_f64().unwrap(),
        ceiling(),
    );
    let (status, second) = post(&base, "/verify_physics", &handed_back).await;
    assert_eq!(status, 200, "{second}");
    let second: Value = serde_json::from_str(&second).unwrap();
    let two = direct_loop(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling(), 2, 1);
    assert_eq!(published(&second["state"], "deflection_m").to_bits(), two.state.deflection_m.to_bits());
    assert_eq!(published(&second["state"], "velocity_m_s").to_bits(), two.state.velocity_m_s.to_bits());
    assert_eq!(published(&second["state"], "t_s").to_bits(), ceiling().to_bits());
    assert!(published(&second["state"], "heat_j_per_kg") < two.state.heat_j_per_kg);
}

// ---------------------------------------------------------------- the run

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_second_of_the_scored_ring_down_ends_on_the_reference_state_of_the_kernel() {
    let base = spawn_app().await;
    let request = run_body(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling(), 4_000, 10);
    // The request is the description's own worked run.
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "signal", "card": "cantilever_eth"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    assert_eq!(described["example_run_trajectory"], request);

    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 4_000);
    assert_eq!(tape["steps_requested"], 4_000);
    assert!(tape["reject"].is_null());
    let frames = tape["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 400);

    // The kernel loop from the same start, to the bit.
    let direct = direct_loop(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling(), 4_000, 10);
    assert_eq!(direct.reject, None);
    let last = &tape["final_state"];
    assert_state_is(last, &direct.state, "4,000 steps");
    assert_eq!(
        tape["max_residual_conservation_error"].as_f64().unwrap().to_bits(),
        direct.worst.to_bits()
    );
    assert!(direct.worst < cantilever_eth::RESIDUAL_CONSERVATION_LIMIT / 10.0, "{}", direct.worst);

    // The reference numbers of the research record after 4,000 steps of the
    // compared run are reached 3,405 steps from here, the ones the kernel's
    // own test holds: deflection and speed. The heat begins again here.
    let (status, text) = post(&base, "/run_trajectory", &run_body(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling(), 3_405, 3_405)).await;
    assert_eq!(status, 200, "{text}");
    let part: Value = serde_json::from_str(&text).unwrap();
    for (name, reference) in [("deflection_m", -7.904474048414e-3), ("velocity_m_s", 3.130458266039e-1)] {
        let got = published(&part["final_state"], name);
        assert!(relative(got, reference) <= 1.0e-9, "{name}: {got:e} against {reference:e}");
    }
    assert!((published(last, "t_s") - 1.0).abs() < 1.0e-12);
    // Energy left the beam as heat, and the books close.
    assert!(published(last, "stored_j_per_kg") < published(last, "energy_anchor_j_per_kg"));
    assert!(published(last, "heat_j_per_kg") > 0.0);

    // Every recorded frame is the kernel loop's at that step: the time, the
    // deflection, the speed and the envelope rounded to 32 bits, the margin
    // to the amplitude ceiling, and no flag.
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame["step"], 10 * (index as u64 + 1));
        let (t_s, deflection, speed, envelope, margin, flags) = slots(frame["frame_hex"].as_str().unwrap());
        let (state, kernel_frame) = &direct.kept[index];
        assert_eq!(t_s.to_bits(), state.t_s.to_bits(), "frame {index}");
        assert_eq!(deflection.to_bits(), (state.deflection_m as f32).to_bits(), "frame {index}");
        assert_eq!(speed.to_bits(), (state.velocity_m_s as f32).to_bits(), "frame {index}");
        assert_eq!(envelope.to_bits(), (state.envelope_m as f32).to_bits(), "frame {index}");
        assert_eq!(margin.to_bits(), ((CEILING_M - state.envelope_m) as f32).to_bits(), "frame {index}");
        assert_eq!(margin.to_bits(), kernel_frame.force_torque.to_bits(), "frame {index}");
        assert!(margin > 0.0);
        assert_eq!(flags, 0, "frame {index}");
    }

    // The tape verifies, and the receipt of the answer matches.
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
    let (status, receipt) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
    assert_eq!(status, 200, "{receipt}");
    let receipt: Value = serde_json::from_str(&receipt).unwrap();
    assert_eq!(receipt["matches"], true, "{receipt}");
    assert_eq!(receipt["receipt_sha256"], tape["receipt_sha256"]);

    // The same request gives the same bytes.
    let (_, again) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(again, answer);

    // A changed state value breaks the tape, and so does a changed frame.
    let mut forged = tape.clone();
    let hex = forged["final_state"]["deflection_m_hex"].as_str().unwrap().to_string();
    forged["final_state"]["deflection_m_hex"] = json!(format!("{:016x}", u64::from_str_radix(&hex, 16).unwrap() + 1));
    let (_, report) = post(&base, "/verify_tape", &forged).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["cause"], "state");
    let mut forged = tape.clone();
    let frame_hex = forged["frames"][200]["frame_hex"].as_str().unwrap().to_string();
    let flipped = if frame_hex.starts_with('0') { "1" } else { "0" };
    forged["frames"][200]["frame_hex"] = json!(format!("{flipped}{}", &frame_hex[1..]));
    let (_, report) = post(&base, "/verify_tape", &forged).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");
}

/// The compared run as one request is accepted at its own start, envelope
/// 15.786 mm, and lands 94,731 steps before it is refused below the floor,
/// its deflections and speeds after 595 steps the scored ring-down's to the
/// bit. From the start of the scored ring-down,
/// 595 steps later, the run of the remaining 96,205 steps with a frame every
/// tenth step lands 94,136 steps, to 23.534 s, past the end of the last
/// scored block, and is refused below the floor. It ends on the kernel loop's
/// state to the bit, it keeps its proof up to the last step that landed, and
/// its tape verifies.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_compared_run_and_the_scored_ring_down_land_to_the_floor() {
    let base = spawn_app().await;
    let compared = run_body(COMPARED_START_DEFLECTION_M, COMPARED_START_VELOCITY_M_S, 2.5e-4, 96_800, 10);
    assert_eq!(
        serde_json::to_string(&compared).unwrap(),
        r#"{"domain":"signal","card":"cantilever_eth","initial":{"deflection_m":-0.01526315579382,"velocity_m_s":0.2099361936006},"dt_s":0.00025,"segments":[{"steps":96800,"load":{}}],"record_every":10}"#
    );
    let (status, text) = post(&base, "/run_trajectory", &compared).await;
    assert_eq!(status, 200, "{}", &text[..text.len().min(400)]);
    let whole: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(whole["outcome"], "refused");
    assert_eq!(whole["reject"], "DeflectionBelowCard");
    assert_eq!(whole["steps_landed"], 94_731);
    assert!(beam().envelope_m(COMPARED_START_DEFLECTION_M, COMPARED_START_VELOCITY_M_S) <= CEILING_M);
    assert!(CantileverState::try_phase(&beam(), COMPARED_START_DEFLECTION_M, COMPARED_START_VELOCITY_M_S).is_some());
    let compared_direct = direct_loop(COMPARED_START_DEFLECTION_M, COMPARED_START_VELOCITY_M_S, 2.5e-4, 96_800, 10);
    assert_eq!(compared_direct.reject, Some(StepReject::DeflectionBelowCard));
    assert_eq!(compared_direct.landed, 94_731);
    assert_state_is(&whole["final_state"], &compared_direct.state, "94,731 steps");
    let (status, report) = post(&base, "/verify_tape", &whole).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");

    let request = run_body(START_DEFLECTION_M, START_VELOCITY_M_S, 2.5e-4, 96_205, 10);
    let started = std::time::Instant::now();
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    let took = started.elapsed();
    assert_eq!(status, 200, "{}", &answer[..answer.len().min(400)]);
    println!("the scored ring-down: {} bytes in {:.2} s", answer.len(), took.as_secs_f64());
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "refused");
    assert_eq!(tape["reject"], "DeflectionBelowCard");
    assert_eq!(tape["reject_detail"], below_the_floor_at(94_136));
    assert_eq!(tape["steps_landed"], 94_136);
    assert_eq!(tape["steps_requested"], 96_205);
    let frames = tape["frames"].as_array().unwrap();
    // A frame every tenth step, and the last landed step's frame.
    assert_eq!(frames.len(), 9_414);
    assert_eq!(frames[0]["step"], 10);
    assert_eq!(frames[9_412]["step"], 94_130);
    assert_eq!(frames[9_413]["step"], 94_136);

    let direct = direct_loop(START_DEFLECTION_M, START_VELOCITY_M_S, 2.5e-4, 96_205, 10);
    assert_eq!(direct.reject, Some(StepReject::DeflectionBelowCard));
    assert_eq!(direct.landed, 94_136);
    let last = &tape["final_state"];
    assert_state_is(last, &direct.state, "94,136 steps");
    assert!((published(last, "t_s") - 23.534).abs() < 1.0e-9);
    assert!(published(last, "envelope_m") >= FLOOR_M);
    let worst = tape["max_residual_conservation_error"].as_f64().unwrap();
    assert_eq!(worst.to_bits(), direct.worst.to_bits());
    assert!(relative(worst, 7.368e-8) < 1.0e-3, "{worst:e}");

    // Each recorded frame is the kernel loop's deflection at that step, with
    // no flag.
    for (index, frame) in frames.iter().take(9_413).enumerate() {
        let (_, deflection, _, _, _, flags) = slots(frame["frame_hex"].as_str().unwrap());
        let (state, _) = &direct.kept[index];
        assert_eq!(deflection.to_bits(), (state.deflection_m as f32).to_bits(), "frame {index}");
        assert_eq!(flags, 0, "frame {index}");
    }

    assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 64));
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");

    // One frame more than the limit is refused before anything runs: the same
    // steps with a frame every ninth step would record 10,689.
    let too_many = run_body(START_DEFLECTION_M, START_VELOCITY_M_S, 2.5e-4, 96_205, 9);
    let (status, text) = post(&base, "/run_trajectory", &too_many).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("the run would record more than 10000 frames"), "{text}");
    assert!(!text.contains("run_seal"), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_start_near_the_ceiling_is_refused_part_way_and_the_run_keeps_its_proof_up_to_the_last_step() {
    let base = spawn_app().await;
    let speed_at_ceiling_m_s = CEILING_M * beam().omega0_rad_s();

    // The envelope at 0.99 of the ceiling, all of it as speed.
    let start_m_s = 0.99 * speed_at_ceiling_m_s;
    let request = run_body(0.0, start_m_s, ceiling(), 500, 1);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    let direct = direct_loop(0.0, start_m_s, ceiling(), 500, 1);
    assert_eq!(direct.reject, Some(StepReject::DeflectionAboveCard));
    // The description says 104 steps.
    assert_eq!(direct.landed, 104);
    assert_eq!(tape["outcome"], "refused");
    assert_eq!(tape["reject"], "DeflectionAboveCard");
    assert_eq!(
        tape["reject_detail"],
        "Physical Failure: the deflection envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2), is above 0.01579 m, the largest the compared run held (DeflectionAboveCard). The run is sealed to step 104. The state at step 105 was refused and is not sealed."
    );
    assert_eq!(tape["steps_landed"], 104);
    assert_eq!(tape["steps_requested"], 500);
    assert_eq!(tape["frames"].as_array().unwrap().len(), 104);
    // The last landed state is the kernel loop's, and it is inside the
    // ceiling: the refused state is not in it.
    let last = &tape["final_state"];
    assert_state_is(last, &direct.state, "refused part way");
    assert!(published(last, "envelope_m") <= CEILING_M);
    assert!(published(last, "deflection_m") > 0.0138);
    let (_, _, _, _, margin, _) = slots(tape["frames"][102]["frame_hex"].as_str().unwrap());
    assert!(margin >= 0.0);
    // The run has a proof, and the tape verifies.
    assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 64));
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
    // One step fewer than the card lands completes, on the same state.
    let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, start_m_s, ceiling(), 104, 104)).await;
    assert_eq!(status, 200, "{text}");
    let short: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(short["outcome"], "completed");
    assert_eq!(short["final_state"], tape["final_state"]);
    // The single step from that start lands: only the run reaches the ceiling.
    let (status, text) = post(&base, "/verify_physics", &step_body(0.0, start_m_s, ceiling())).await;
    assert_eq!(status, 200, "{text}");

    // At 0.98 of the ceiling the beam rings through.
    let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, 0.98 * speed_at_ceiling_m_s, ceiling(), 2_000, 100)).await;
    assert_eq!(status, 200, "{text}");
    let through: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(through["outcome"], "completed", "{}", through["reject_detail"]);
    assert_eq!(through["steps_landed"], 2_000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_start_under_the_floor_is_refused_by_name_and_a_ring_down_keeps_its_proof_to_the_floor() {
    let base = spawn_app().await;

    // Under 0.058 mm no step and no run starts, rest at zero included, and a
    // deflection whose 32-bit copy would read as zero or as a subnormal.
    for (deflection_m, velocity_m_s) in
        [(5.7e-5, 0.0), (-5.0e-5, 0.0), (0.0, 3.0e-3), (0.0, 0.0), (1.0e-6, 0.0), (1.0e-40, 0.0), (1.0e-200, 0.0)]
    {
        let label = format!("{deflection_m:e} {velocity_m_s:e}");
        assert!(beam().envelope_m(deflection_m, velocity_m_s) < FLOOR_M, "{label}");
        assert!(CantileverState::try_phase(&beam(), deflection_m, velocity_m_s).is_none(), "{label}");
        let (status, text) = post(&base, "/verify_physics", &step_body(deflection_m, velocity_m_s, ceiling())).await;
        assert_eq!(status, 422, "{label}: {text}");
        assert_eq!(text, ENVELOPE_BELOW_THE_FLOOR, "{label}");
        assert!(names(&text, "DeflectionBelowCard"), "{label}: {text}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(deflection_m, velocity_m_s, ceiling(), 10, 1)).await;
        assert_eq!(status, 422, "{label}: {text}");
        assert_eq!(text, ENVELOPE_BELOW_THE_FLOOR, "{label}");
    }

    // At rest at the floor the start is made, and the first step, toward the
    // rest position, lands under the floor: it is refused by name, unsealed.
    assert!(CantileverState::try_phase(&beam(), FLOOR_M, 0.0).is_some());
    let (status, text) = post(&base, "/verify_physics", &step_body(FLOOR_M, 0.0, ceiling())).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, ENVELOPE_BELOW_THE_FLOOR);

    // Above the floor a step lands with no flag, and the proof is over the
    // frame as the kernel wrote it.
    for (deflection_m, velocity_m_s) in [(6.5e-5, 0.0), (0.0, 3.1e-3), (1.0e-3, 0.0)] {
        let request = step_body(deflection_m, velocity_m_s, ceiling());
        let (status, answer) = post(&base, "/verify_physics", &request).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert!(published(&response.state, "envelope_m") >= FLOOR_M);
        let beam = beam();
        let mut state = CantileverState::try_phase(&beam, deflection_m, velocity_m_s).unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        cantilever_eth::step(&beam, &mut state, &mut frame, NO_LOAD, ceiling()).unwrap();
        assert_eq!(frame.flags, 0);
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
    }

    // A ring-down from 0.1 mm at rest lands until its envelope would pass
    // under the floor, is refused there by name, keeps its proof up to the
    // last step that landed, and its tape verifies.
    let request = run_body(1.0e-4, 0.0, ceiling(), 20_000, 1_000);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    let direct = direct_loop(1.0e-4, 0.0, ceiling(), 20_000, 1_000);
    assert_eq!(direct.reject, Some(StepReject::DeflectionBelowCard));
    assert_eq!(direct.landed, 12_024);
    assert_eq!(tape["outcome"], "refused");
    assert_eq!(tape["reject"], "DeflectionBelowCard");
    assert_eq!(tape["reject_detail"], below_the_floor_at(12_024));
    assert_eq!(tape["steps_landed"], 12_024);
    assert_state_is(&tape["final_state"], &direct.state, "rang down to the floor");
    assert!(published(&tape["final_state"], "envelope_m") >= FLOOR_M);
    // A frame every 1,000th step, and the last landed step's frame.
    let frames = tape["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 13);
    assert_eq!(frames[11]["step"], 12_000);
    assert_eq!(frames[12]["step"], 12_024);
    for (index, frame) in frames.iter().enumerate() {
        let (_, _, _, _, _, flags) = slots(frame["frame_hex"].as_str().unwrap());
        assert_eq!(flags, 0, "frame {index}");
    }
    assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 64));
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
}

/// From rest at the amplitude ceiling, on either side, no step from one to
/// four times the step floor is refused, over four steps each: at a turning
/// point the deflection moves least, and the floor is set so that it moves.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn from_rest_at_the_ceiling_no_step_from_one_to_four_floors_is_refused() {
    let base = spawn_app().await;
    assert_eq!(floor().to_bits(), (2.5e-4_f64 / 262_144.0).to_bits());
    for sign in [1.0, -1.0] {
        for index in 0..=24 {
            let dt_s = floor() * (1.0 + 3.0 * f64::from(index) / 24.0);
            let label = format!("{} m, {dt_s:e} s", sign * CEILING_M);
            let (status, text) = post(&base, "/run_trajectory", &run_body(sign * CEILING_M, 0.0, dt_s, 4, 1)).await;
            assert_eq!(status, 200, "{label}: {text}");
            let tape: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(tape["outcome"], "completed", "{label}: {}", tape["reject_detail"]);
            assert_eq!(tape["steps_landed"], 4, "{label}");
            let direct = direct_loop(sign * CEILING_M, 0.0, dt_s, 4, 1);
            assert_eq!(direct.reject, None, "{label}");
            assert_state_is(&tape["final_state"], &direct.state, &label);
            assert!(published(&tape["final_state"], "deflection_m").abs() < CEILING_M, "{label}");
            let (status, text) = post(&base, "/verify_physics", &step_body(sign * CEILING_M, 0.0, dt_s)).await;
            assert_eq!(status, 200, "{label}: {text}");
        }
    }
    // The earlier floor, a quarter of this one, is below the floor and refused by name.
    let (status, text) = post(&base, "/verify_physics", &step_body(CEILING_M, 0.0, 0.25 * floor())).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("the step floor of the cantilever_eth card"), "{text}");
}

// ------------------------------------------------------------- the bounds

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let largest_m_s = largest_speed_m_s();
    // The description publishes exactly these numbers.
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "signal", "card": "cantilever_eth"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    let bound = |index: usize, key: &str| described["phase_fields"][index][key].as_f64().unwrap();
    assert_eq!(described["phase_fields"][0]["name"], "deflection_m");
    assert_eq!(bound(0, "minimum").to_bits(), (-CEILING_M).to_bits());
    assert_eq!(bound(0, "maximum").to_bits(), CEILING_M.to_bits());
    assert_eq!(described["phase_fields"][1]["name"], "velocity_m_s");
    assert_eq!(bound(1, "minimum").to_bits(), (-largest_m_s).to_bits());
    assert_eq!(bound(1, "maximum").to_bits(), largest_m_s.to_bits());
    assert!(described["load_fields"].as_array().unwrap().is_empty());

    // (deflection, speed, the words and the name of the refusal, or none)
    let own = |field: &str, value: f64, unit: &str, limit: f64| {
        let (side, kind, limit) = if value > 0.0 { ("above", "maximum", limit) } else { ("below", "minimum", -limit) };
        format!(
            "Physical Failure: '{field}' {} {unit} is {side} {} {unit}, the inclusive {kind} of the cantilever_eth card (DeflectionAboveCard). That state is not sealed.",
            protocol_mcp_gateway::message_number(value),
            protocol_mcp_gateway::message_number(limit)
        )
    };
    let above_card = |sentence: String| Some((sentence, "DeflectionAboveCard"));
    let below_card = Some((ENVELOPE_BELOW_THE_FLOOR.to_string(), "DeflectionBelowCard"));
    let cases: Vec<(f64, f64, Option<(String, &str)>)> = vec![
        // The deflection at rest, on either side.
        (CEILING_M, 0.0, None),
        (above(CEILING_M), 0.0, above_card(own("deflection_m", above(CEILING_M), "m", CEILING_M))),
        (-CEILING_M, 0.0, None),
        (-above(CEILING_M), 0.0, above_card(own("deflection_m", -above(CEILING_M), "m", CEILING_M))),
        (0.0189, 0.0, above_card(own("deflection_m", 0.0189, "m", CEILING_M))),
        (0.0158, 0.0, above_card(own("deflection_m", 0.0158, "m", CEILING_M))),
        // The speed at zero deflection, on either side.
        (0.0, largest_m_s, None),
        (0.0, above(largest_m_s), above_card(own("velocity_m_s", above(largest_m_s), "m/s", largest_m_s))),
        (0.0, -largest_m_s, None),
        (0.0, -above(largest_m_s), above_card(own("velocity_m_s", -above(largest_m_s), "m/s", largest_m_s))),
        (0.0, 0.9893, above_card(own("velocity_m_s", 0.9893, "m/s", largest_m_s))),
        // Each inside its own bound, and the two together past the ceiling.
        (0.012, 0.6, above_card(ENVELOPE_ABOVE_THE_CEILING.to_string())),
        (-0.012, -0.6, above_card(ENVELOPE_ABOVE_THE_CEILING.to_string())),
        (CEILING_M, 1.0e-3, above_card(ENVELOPE_ABOVE_THE_CEILING.to_string())),
        (1.0e-4, largest_m_s, above_card(ENVELOPE_ABOVE_THE_CEILING.to_string())),
        // Each inside its own bound, and the two together inside the ceiling.
        (0.01, 0.5, None),
        (-0.0095, -0.5, None),
        // The two together under the floor, rest at zero included, and just above it.
        (0.0, 0.0, below_card.clone()),
        (below(FLOOR_M), 0.0, below_card.clone()),
        (-below(FLOOR_M), 0.0, below_card.clone()),
        (0.0, 3.0e-3, below_card.clone()),
        (0.0, 3.1e-3, None),
        (0.0, -3.1e-3, None),
    ];
    for (deflection_m, velocity_m_s, refusal) in cases {
        let label = format!("deflection {deflection_m:e} speed {velocity_m_s:e}");
        let step = step_body(deflection_m, velocity_m_s, ceiling());
        let run = run_body(deflection_m, velocity_m_s, ceiling(), 3, 1);
        let plan = json!({
            "domain": "signal",
            "card": "cantilever_eth",
            "plan": { "kind": "list", "runs": [{
                "initial": { "deflection_m": deflection_m, "velocity_m_s": velocity_m_s },
                "dt_s": ceiling(),
                "segments": [{ "steps": 3, "load": {} }],
                "record_every": 1
            }] },
            "store": "none",
            "return": "inline", "answer": "full"
        });
        let (step_status, step_text) = post(&base, "/verify_physics", &step).await;
        let (run_status, run_text) = post(&base, "/run_trajectory", &run).await;
        let (plan_status, plan_text) = post(&base, "/run_dataset", &plan).await;
        let beam = beam();
        let started = CantileverState::try_phase(&beam, deflection_m, velocity_m_s);
        match refusal {
            None => {
                // The kernel anchors this start, and the gateway lands it.
                assert!(started.is_some(), "{label}");
                assert_eq!(step_status, 200, "{label}: {step_text}");
                assert_eq!(run_status, 200, "{label}: {run_text}");
                assert_eq!(plan_status, 200, "{label}: {plan_text}");
                // Nothing was moved to the bound: the three steps are the
                // kernel loop's from the values that were sent.
                let direct = direct_loop(deflection_m, velocity_m_s, ceiling(), 3, 1);
                assert_eq!(direct.reject, None, "{label}");
                let tape: Value = serde_json::from_str(&run_text).unwrap();
                assert_eq!(tape["outcome"], "completed", "{label}: {run_text}");
                assert_state_is(&tape["final_state"], &direct.state, &label);
                assert_eq!(tape["inputs"]["initial"]["deflection_m"].as_f64().unwrap().to_bits(), deflection_m.to_bits());
                assert_eq!(tape["inputs"]["initial"]["velocity_m_s"].as_f64().unwrap().to_bits(), velocity_m_s.to_bits());
            }
            Some((sentence, name)) => {
                // The kernel anchors no state here, and the gateway says why.
                assert!(started.is_none(), "{label}");
                for (status, text) in [(step_status, &step_text), (run_status, &run_text)] {
                    assert_eq!(status, 422, "{label}: {text}");
                    assert_eq!(*text, sentence, "{label}");
                    assert!(names(text, name), "{label}: {text}");
                }
                // A dataset row off the card is not admitted: it carries the
                // same sentence and no proof, and nothing runs.
                assert_eq!(plan_status, 200, "{label}: {plan_text}");
                let answer: Value = serde_json::from_str(&plan_text).unwrap();
                assert_eq!(answer["rows"][0]["outcome"], "unadmitted", "{plan_text}");
                assert!(answer["rows"][0]["run_seal"].is_null(), "{plan_text}");
                assert_eq!(answer["rows"][0]["reject_detail"], sentence, "{label}");
            }
        }
    }

    // The step: the floor and the ceiling land, and the next number beyond
    // each is named. A step of zero or less is named before the floor.
    for dt_s in [floor(), ceiling(), 1.0e-6] {
        let (status, text) = post(&base, "/verify_physics", &step_body(1.0e-3, 0.0, dt_s)).await;
        assert_eq!(status, 200, "{dt_s}: {text}");
    }
    for (dt_s, words) in [
        (below(floor()), "the step floor of the cantilever_eth card"),
        (1.0e-15, "the step floor of the cantilever_eth card"),
        (above(ceiling()), "the step ceiling of the cantilever_eth card"),
        (5.0e-4, "the step ceiling of the cantilever_eth card"),
        (0.0, "(NonPositiveDt) on the cantilever_eth card"),
        (-2.5e-4, "(NonPositiveDt) on the cantilever_eth card"),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body(1.0e-3, 0.0, dt_s)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains(words) && !text.contains("sha256_seal"), "{dt_s}: {text}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(1.0e-3, 0.0, dt_s, 3, 1)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains(words) && !text.contains("run_seal"), "{dt_s}: {text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_input_a_load_the_card_does_not_have_and_a_card_constant_are_refused() {
    let base = spawn_app().await;
    let good = step_body(1.0e-3, 0.0, ceiling());
    let (status, _) = post(&base, "/verify_physics", &good).await;
    assert_eq!(status, 200);

    // Each of the three inputs, missing. Nothing is filled in.
    for key in ["deflection_m", "velocity_m_s", "dt_s"] {
        let mut step = good.clone();
        step["parameters"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}'"));
    }
    let mut run = run_body(1.0e-3, 0.0, ceiling(), 3, 1);
    run["initial"].as_object_mut().unwrap().remove("velocity_m_s");
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("Missing required parameter: 'velocity_m_s'"), "{text}");
    // A segment must carry its load object, and the object must be empty.
    let mut run = run_body(1.0e-3, 0.0, ceiling(), 3, 1);
    run["segments"][0].as_object_mut().unwrap().remove("load");
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(!text.contains("run_seal"), "{text}");

    // A value that is not a number, and a number that is not finite.
    for (key, raw) in [("deflection_m", "\"0.001\""), ("velocity_m_s", "null"), ("deflection_m", "[0.001]")] {
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
        let body = serde_json::to_string(&good).unwrap().replace("\"velocity_m_s\":0.0", &format!("\"velocity_m_s\":{raw}"));
        assert!(body.contains(raw), "{body}");
        let (status, text) = post_text(&base, "/verify_physics", &body).await;
        assert!(status == 400 || status == 422, "{raw}: {status} {text}");
        assert!(!text.contains("sha256_seal"), "{text}");
    }
    let (status, text) = post_text(
        &base,
        "/verify_physics",
        &serde_json::to_string(&good).unwrap().replace("\"velocity_m_s\":0.0", "\"velocity_m_s\":1e-320"),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("not a finite normal number"), "{text}");

    // The card has no load. A drive of any name, a card constant and a
    // quantity of the state are not fields of a request: in a step, at the
    // start of a run and in a segment's load.
    for key in [
        "accel_m_s2",
        "force_n",
        "drive_m_s2",
        "small_amplitude_frequency_hz",
        "spring_kappa",
        "spring_power",
        "damping_c",
        "damping_power",
        "drag_per_m",
        "amplitude_ceiling_m",
        "amplitude_floor_m",
        "gate_dt_s",
        "residual_conservation_limit",
        "envelope_m",
        "heat_j_per_kg",
        "mass_kg",
    ] {
        let sentence = format!("Physical Violation: '{key}' is not a field of this request.");
        let mut step = good.clone();
        step["parameters"][key] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, sentence);

        let mut in_phase = run_body(1.0e-3, 0.0, ceiling(), 3, 1);
        in_phase["initial"][key] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_phase).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, sentence);

        let mut in_load = run_body(1.0e-3, 0.0, ceiling(), 3, 1);
        in_load["segments"][0]["load"][key] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, sentence);
    }
}

// ---------------------------------------------------------------- dataset

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_small_grid_gives_rows_whose_run_seals_are_the_runs_alone_and_the_dataset_verifies() {
    let base = spawn_app().await;
    let deflections = [-0.01, 0.005, 0.0125];
    let speeds = [0.0, 0.3];
    for store in ["frames", "none"] {
        let request = json!({
            "domain": "signal",
            "card": "cantilever_eth",
            "plan": {
                "kind": "grid",
                "base": {
                    "initial": {},
                    "dt_s": ceiling(),
                    "segments": [{ "steps": 400, "load": {} }],
                    "record_every": 100
                },
                "axes": [
                    { "path": "initial.deflection_m", "values": deflections },
                    { "path": "initial.velocity_m_s", "values": speeds }
                ]
            },
            "store": store,
            "return": "inline", "answer": "full"
        });
        let (status, text) = post(&base, "/run_dataset", &request).await;
        assert_eq!(status, 200, "{text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        let rows = answer["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 6);
        assert_eq!(answer["head"]["completed"], 6);
        assert_eq!(answer["head"]["unadmitted"], 0);
        let mut index = 0;
        let mut seals = std::collections::BTreeSet::new();
        for deflection_m in deflections {
            for velocity_m_s in speeds {
                let row = &rows[index];
                assert_eq!(row["outcome"], "completed", "{row}");
                let alone = run_body(deflection_m, velocity_m_s, ceiling(), 400, 100);
                let (status, alone) = post(&base, "/run_trajectory", &alone).await;
                assert_eq!(status, 200, "{alone}");
                let alone: Value = serde_json::from_str(&alone).unwrap();
                assert_eq!(row["run_seal"], alone["run_seal"], "{store} {deflection_m} {velocity_m_s}");
                assert_eq!(row["state_digest"], alone["state_digest"], "{store} {deflection_m} {velocity_m_s}");
                assert_eq!(row["steps_landed"], 400);
                seals.insert(row["run_seal"].as_str().unwrap().to_string());
                index += 1;
            }
        }
        assert_eq!(seals.len(), 6, "six runs, six proofs");
        // The dataset verifies, and not with one row's proof changed.
        let parts = json!({ "head": answer["head"], "rows": answer["rows"], "frames_hex": answer["frames_hex"] });
        let (status, report) = post(&base, "/verify_dataset", &parts).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{report}");
        let mut forged = parts.clone();
        forged["rows"][3]["run_seal"] = forged["rows"][2]["run_seal"].clone();
        let (_, report) = post(&base, "/verify_dataset", &forged).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], false, "{report}");
    }

    // An axis value outside a published bound makes its row unadmitted.
    let (status, text) = post(
        &base,
        "/run_dataset",
        &json!({
            "domain": "signal", "card": "cantilever_eth",
            "plan": { "kind": "grid",
                "base": { "initial": { "velocity_m_s": 0.0 }, "dt_s": ceiling(),
                          "segments": [{ "steps": 5, "load": {} }], "record_every": 1 },
                "axes": [{ "path": "initial.deflection_m", "values": [0.01, 0.02] }] },
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

    // A grid whose axes are each inside their bound, with one corner past the
    // ceiling together: that row is not admitted, by name, and the others run.
    let (status, text) = post(
        &base,
        "/run_dataset",
        &json!({
            "domain": "signal", "card": "cantilever_eth",
            "plan": { "kind": "grid",
                "base": { "initial": {}, "dt_s": ceiling(),
                          "segments": [{ "steps": 5, "load": {} }], "record_every": 5 },
                "axes": [{ "path": "initial.deflection_m", "values": [0.005, 0.014] },
                         { "path": "initial.velocity_m_s", "values": [0.1, 0.45] }] },
            "store": "none", "return": "inline", "answer": "full"
        }),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let outcomes: Vec<&str> = answer["rows"].as_array().unwrap().iter().map(|row| row["outcome"].as_str().unwrap()).collect();
    assert_eq!(outcomes, ["completed", "completed", "completed", "unadmitted"]);
    assert_eq!(answer["rows"][3]["reject_detail"], ENVELOPE_ABOVE_THE_CEILING);
    assert!(answer["rows"][3]["run_seal"].is_null());
}

// ---------------------------------------------------------- validate_series

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_the_deflection_of_a_run_and_carries_its_run_seal() {
    let base = spawn_app().await;
    // The first second of the scored ring-down, read every tenth step: 400
    // values a second, as a measurement of this beam is read.
    let run = run_body(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling(), 4_000, 1_000);
    let (_, alone) = post(&base, "/run_trajectory", &run).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["outcome"], "completed");
    let direct = direct_loop(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling(), 4_000, 10);
    let steps: Vec<u64> = (1..=400).map(|index| 10 * index).collect();

    // Every published number is a quantity, and its values are the kernel
    // loop's at each step that was asked for, to the bit.
    let mut quantities = 0;
    for (index, (quantity, _)) in state_numbers(&direct.state).into_iter().enumerate() {
        let probe = json!({
            "run": run, "quantity": quantity,
            "measurements": steps.iter().map(|step| json!({ "step": step, "value": 0.0 })).collect::<Vec<_>>()
        });
        let (status, text) = post(&base, "/validate_series", &probe).await;
        assert_eq!(status, 200, "{quantity}: {text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(answer["run_seal"], alone["run_seal"], "{quantity}");
        assert_eq!(answer["outcome"], "completed", "{quantity}");
        assert_eq!(answer["compared"], 400, "{quantity}");
        assert_eq!(answer["not_reached"], 0, "{quantity}");
        let model = answer["model_values_hex"].as_array().unwrap();
        for (at, hex) in model.iter().enumerate() {
            let value = f64::from_bits(u64::from_str_radix(hex.as_str().unwrap(), 16).unwrap());
            let kernel = state_numbers(&direct.kept[at].0)[index].1;
            assert_eq!(value.to_bits(), kernel.to_bits(), "{quantity} at step {}", steps[at]);
        }
        quantities += 1;
    }
    assert_eq!(quantities, 11);

    // Deflections equal to the run's give an error of exactly zero, and a
    // known offset comes back as that offset. No verdict is given.
    let exact: Vec<Value> = steps
        .iter()
        .zip(&direct.kept)
        .map(|(step, (state, _))| json!({ "step": step, "value": state.deflection_m }))
        .collect();
    let (status, text) =
        post(&base, "/validate_series", &json!({ "run": run, "quantity": "deflection_m", "measurements": exact })).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rms_error"], 0.0);
    assert_eq!(answer["max_abs_error"], 0.0);
    let offset: Vec<Value> = steps
        .iter()
        .zip(&direct.kept)
        .map(|(step, (state, _))| json!({ "step": step, "value": state.deflection_m - 1.0e-3 }))
        .collect();
    let (status, text) =
        post(&base, "/validate_series", &json!({ "run": run, "quantity": "deflection_m", "measurements": offset })).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert!((answer["mean_signed_error"].as_f64().unwrap() - 1.0e-3).abs() < 1.0e-12, "{text}");
    assert!((answer["rms_error"].as_f64().unwrap() - 1.0e-3).abs() < 1.0e-12, "{text}");
    assert!(answer.get("pass").is_none() && answer.get("verdict").is_none());

    // A name the card does not publish is not a quantity: the flag of earlier
    // versions, a margin and a frequency.
    for quantity in ["below_fitted_amplitude", "ceiling_margin_m", "frequency_hz"] {
        let body = json!({ "run": run, "quantity": quantity, "measurements": [{ "step": 10, "value": 0.0 }] });
        let (status, text) = post(&base, "/validate_series", &body).await;
        assert_eq!(status, 422, "{quantity}: {text}");
        assert!(!text.contains("run_seal"), "{text}");
    }

    // A run that is refused part way compares the steps it reached.
    let refused = run_body(0.0, 0.99 * CEILING_M * beam().omega0_rad_s(), ceiling(), 500, 500);
    let probe = json!({
        "run": refused, "quantity": "deflection_m",
        "measurements": [{ "step": 50, "value": 0.01 }, { "step": 104, "value": 0.014 }, { "step": 105, "value": 0.014 }]
    });
    let (status, text) = post(&base, "/validate_series", &probe).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["outcome"], "refused");
    assert_eq!(answer["reject"], "DeflectionAboveCard");
    assert_eq!(answer["steps_landed"], 104);
    assert_eq!(answer["compared"], 2);
    assert_eq!(answer["not_reached"], 1);
}

// ------------------------------------------------------ the three connections

const KEY_ALL: &str = "key-for-the-client-that-sees-everything";
const KEY_BEAM: &str = "key-for-the-cantilever-client";
const KEY_THERMAL: &str = "key-for-the-thermal-client";

fn client_entry(label: &str, key: &str, cards: Value) -> Value {
    json!({ "client": label, "key_sha256": sha256_hex(key.as_bytes()), "cards": cards, "solvers": [], "max_in_flight": 8, "max_permits": 1 })
}

/// The local connection: `mcp_stdio` as a child process.
struct Local {
    _child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Local {
    async fn start() -> Local {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut local = Local { _child: child, stdin, out };
        let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
        local.raw(&init.to_string()).await;
        local
            .stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .unwrap();
        local
    }

    async fn raw(&mut self, line: &str) -> String {
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.write_all(b"\n").await.unwrap();
        self.stdin.flush().await.unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).await.unwrap();
        answer.trim_end_matches('\n').to_string()
    }

    /// The whole answer line of a tool call, as the hosted connection frames it.
    async fn call(&mut self, name: &str, arguments: &Value) -> String {
        let line = json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":name,"arguments":arguments}});
        self.raw(&line.to_string()).await
    }
}

/// The hosted connection: `hosted_door` as a child process, with a keys file.
struct Hosted {
    _child: Child,
    base: String,
    _dir: PathBuf,
}

impl Hosted {
    async fn start(keys: &Value) -> Hosted {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cantilever_hosted_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("keys.json");
        std::fs::write(&file, serde_json::to_string_pretty(keys).unwrap()).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .arg("--port")
            .arg("0")
            .arg("--keys")
            .arg(&file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        out.read_line(&mut line).await.unwrap();
        let address = line
            .split("running on ")
            .nth(1)
            .unwrap_or_else(|| panic!("no address in {line:?}"))
            .split(' ')
            .next()
            .unwrap()
            .to_string();
        tokio::spawn(async move {
            let mut rest = String::new();
            while out.read_line(&mut rest).await.unwrap_or(0) > 0 {
                rest.clear();
            }
        });
        Hosted { _child: child, base: format!("http://{address}"), _dir: dir }
    }

    async fn call(&self, key: &str, tool: &str, arguments: &Value) -> String {
        let body = json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":tool,"arguments":arguments}});
        let response = reqwest::Client::new()
            .post(format!("{}/mcp", self.base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Authorization", format!("Bearer {key}"))
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        response.text().await.unwrap()
    }
}

/// The text of a tool's result, and whether the connection marked it an error.
fn result_text(answer: &str) -> (String, bool) {
    let value: Value = serde_json::from_str(answer).unwrap();
    let text = value["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text in {answer}"))
        .to_string();
    (text, value["result"]["isError"] == true)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_three_connections_list_the_beam_and_give_the_same_bytes_for_its_description_and_its_tools() {
    let web = spawn_app().await;
    let mut local = Local::start().await;

    // Thirty cards, the beam the sixteenth, in the signal domain.
    let (listed, error) = result_text(&local.call("list_cards", &json!({})).await);
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 31);
    assert_eq!(cards[15]["domain"], "signal");
    assert_eq!(cards[15]["card"], "cantilever_eth");
    let on_web: Value = Client::new().get(format!("{web}/cards")).send().await.unwrap().json().await.unwrap();
    assert_eq!(on_web, listed);

    let all_pairs: Vec<Value> = cards.iter().map(|card| json!([card["domain"], card["card"]])).collect();
    let keys = json!([
        client_entry("all", KEY_ALL, Value::Array(all_pairs)),
        client_entry("beam", KEY_BEAM, json!([["signal", "cantilever_eth"]])),
        client_entry("thermal", KEY_THERMAL, json!([["energy", "thermal"]])),
    ]);
    let hosted = Hosted::start(&keys).await;

    // The description: the same bytes on the local, the web and the hosted
    // connection, for the full list and for a key that lists the beam alone.
    let pair = json!({"domain": "signal", "card": "cantilever_eth"});
    let on_local = local.call("describe_card", &pair).await;
    let (described, error) = result_text(&on_local);
    assert!(!error);
    let (status, on_web) = post(&web, "/describe_card", &pair).await;
    assert_eq!(status, 200);
    assert_eq!(described, on_web);
    for key in [KEY_ALL, KEY_BEAM] {
        assert_eq!(hosted.call(key, "describe_card", &pair).await, on_local, "{key}");
    }

    // What the description holds: two inputs with their bounds, no load,
    // eight refusals each with its meaning, no flag, and the five parts of
    // the validation record.
    let description: Value = serde_json::from_str(&described).unwrap();
    let phase: Vec<&str> = description["phase_fields"].as_array().unwrap().iter().map(|field| field["name"].as_str().unwrap()).collect();
    assert_eq!(phase, ["deflection_m", "velocity_m_s"]);
    assert!(description["load_fields"].as_array().unwrap().is_empty());
    let refusals: Vec<&str> = description["rejects"].as_array().unwrap().iter().map(|reject| reject["name"].as_str().unwrap()).collect();
    assert_eq!(
        refusals,
        ["NonFiniteInput", "NonFiniteState", "NonPositiveDt", "InvalidBeam", "DeflectionAboveCard", "DeflectionBelowCard", "SpontaneousGeneration", "FirstLawBreach"]
    );
    for reject in description["rejects"].as_array().unwrap() {
        assert!(reject["meaning"].as_str().is_some_and(|meaning| !meaning.is_empty()), "{reject}");
    }
    assert!(description["advisory_flags"].as_array().unwrap().is_empty());
    let record = description["validation"].as_object().unwrap();
    let parts: Vec<&str> = record.keys().map(String::as_str).collect();
    assert_eq!(parts, ["object", "data", "pass_line", "result", "limits"]);
    assert!(record["data"].as_str().unwrap().contains("doi 10.5281/zenodo.22723484"));
    assert!(record["pass_line"].as_str().unwrap().contains("d081cc5ae51b5de4a028cd17b4e1ae80b402f71fdc786f82876713f3b925d4a6"));
    // The one place the tests hold the words of the result: the comparison
    // with the half that had been kept unread.
    assert!(record["result"].as_str().unwrap().starts_with(
        "Compared with the held-back half on 2026-10-09, and by the rule fixed beforehand the comparison failed: three of the four lines were met and the amplitude line was missed."
    ));

    // The four tools that compute: the same bytes on the three connections.
    let step = description["example_verify_step"].clone();
    let run = run_body(START_DEFLECTION_M, START_VELOCITY_M_S, ceiling(), 800, 100);
    let dataset = json!({
        "domain": "signal", "card": "cantilever_eth",
        "plan": { "kind": "list", "runs": [
            { "initial": { "deflection_m": 0.01, "velocity_m_s": 0.0 }, "dt_s": ceiling(),
              "segments": [{ "steps": 200, "load": {} }], "record_every": 50 },
            { "initial": { "deflection_m": 0.012, "velocity_m_s": 0.6 }, "dt_s": ceiling(),
              "segments": [{ "steps": 200, "load": {} }], "record_every": 50 }
        ] },
        "store": "frames", "return": "inline", "answer": "full"
    });
    let series = json!({
        "run": run, "quantity": "deflection_m",
        "measurements": [{ "step": 100, "value": 0.0 }, { "step": 400, "value": 0.01 }, { "step": 800, "value": -0.01 }]
    });
    let mut tape_text = String::new();
    for (tool, route, arguments) in [
        ("verify_step", "/verify_physics", &step),
        ("run_trajectory", "/run_trajectory", &run),
        ("run_dataset", "/run_dataset", &dataset),
        ("validate_series", "/validate_series", &series),
    ] {
        let on_local = local.call(tool, arguments).await;
        let (text, error) = result_text(&on_local);
        assert!(!error, "{tool}: {text}");
        let (status, on_web) = post(&web, route, arguments).await;
        assert_eq!(status, 200, "{tool}: {on_web}");
        assert_eq!(text, on_web, "{tool}");
        for key in [KEY_ALL, KEY_BEAM] {
            assert_eq!(hosted.call(key, tool, arguments).await, on_local, "{tool} {key}");
        }
        if tool == "run_trajectory" {
            tape_text = text;
        }
    }
    // The dataset's second row is past the ceiling: not admitted, by name.
    let (text, _) = result_text(&local.call("run_dataset", &dataset).await);
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed");
    assert_eq!(answer["rows"][1]["outcome"], "unadmitted");
    assert_eq!(answer["rows"][1]["reject_detail"], ENVELOPE_ABOVE_THE_CEILING);

    // The tape and the dataset check on the three connections, with the same bytes.
    let tape: Value = serde_json::from_str(&tape_text).unwrap();
    let parts = json!({ "head": answer["head"], "rows": answer["rows"], "frames_hex": answer["frames_hex"] });
    for (tool, route, arguments) in [("verify_tape", "/verify_tape", &tape), ("verify_dataset", "/verify_dataset", &parts)] {
        let on_local = local.call(tool, arguments).await;
        let (text, error) = result_text(&on_local);
        assert!(!error, "{tool}: {text}");
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], true, "{tool}: {report}");
        let (status, on_web) = post(&web, route, arguments).await;
        assert_eq!(status, 200, "{tool}: {on_web}");
        assert_eq!(text, on_web, "{tool}");
        for key in [KEY_ALL, KEY_BEAM] {
            assert_eq!(hosted.call(key, tool, arguments).await, on_local, "{tool} {key}");
        }
    }

    // A refusal is a tool result marked as an error, with the words of the
    // web connection, on the local and the hosted connection alike.
    let bad = step_body(0.012, 0.6, ceiling());
    let on_local = local.call("verify_step", &bad).await;
    let (text, error) = result_text(&on_local);
    assert!(error);
    let (status, on_web) = post(&web, "/verify_physics", &bad).await;
    assert_eq!(status, 422);
    assert_eq!(text, on_web);
    assert_eq!(text, ENVELOPE_ABOVE_THE_CEILING);
    assert_eq!(hosted.call(KEY_BEAM, "verify_step", &bad).await, on_local);

    // The key that lists the beam alone sees one card.
    let (seen, _) = result_text(&hosted.call(KEY_BEAM, "list_cards", &json!({})).await);
    let seen: Value = serde_json::from_str(&seen).unwrap();
    assert_eq!(seen["cards"].as_array().unwrap().len(), 1);
    assert_eq!(seen["cards"][0]["card"], "cantilever_eth");

    // A key that does not list the beam gets the answers of a card that does
    // not exist, byte for byte, and no word of the beam.
    let invented = "qqqqqqqqqqqqqq";
    assert_eq!(invented.len(), "cantilever_eth".len());
    let rename = |value: &Value| {
        let mut value = value.clone();
        value["card"] = json!(invented);
        value
    };
    let mut renamed_tape = tape.clone();
    renamed_tape["inputs"]["card"] = json!(invented);
    let renamed_series = json!({ "run": rename(&run), "quantity": "deflection_m", "measurements": series["measurements"] });
    let mut session = Vec::new();
    for (tool, hidden, fake) in [
        ("describe_card", pair.clone(), rename(&pair)),
        ("verify_step", step.clone(), rename(&step)),
        ("run_trajectory", run.clone(), rename(&run)),
        ("verify_tape", tape.clone(), renamed_tape),
        ("run_dataset", dataset.clone(), rename(&dataset)),
        ("validate_series", series.clone(), renamed_series),
    ] {
        let on_hidden = hosted.call(KEY_THERMAL, tool, &hidden).await;
        let on_fake = hosted.call(KEY_THERMAL, tool, &fake).await;
        assert_eq!(on_hidden, on_fake, "{tool}");
        session.push(on_hidden);
    }
    assert_eq!(result_text(&hosted.call(KEY_THERMAL, "describe_card", &pair).await).0, "Card not recognized");
    session.push(hosted.call(KEY_THERMAL, "list_cards", &json!({})).await);
    let text = session.join("\n").to_lowercase();
    for word in ["cantilever", "deflection", "envelope", "zürich", "aluminium", "fitted_amplitude"] {
        assert!(!text.contains(word), "the word '{word}' of the hidden card was sent");
    }
}
