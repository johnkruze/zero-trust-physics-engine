//! The line of sight of JWST in fine guide, on the web connection.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes, measured and unmeasured, and its receipt verifies. The card's
//! reference run, 1,252 frames of one fitted file, is sent as one request of
//! 1,252 segments and lands every frame on the kernel loop's state to the
//! bit: the 1,200th, beyond four stated standard deviations, and the first
//! above 5 mas carry their flags, and the frames after them FLAG_RECOVERING.
//! A long gap with no measurement is predicted; a run stated to start soon
//! after a disturbance lands with its first frames flagged. Each fault a
//! request shows is named before the first step and carries no proof: a lock
//! point outside the sensor's field, a quiet time below zero, a `measured` that is
//! neither 0 nor 1, numbers on an unmeasured frame, a count rate off the card,
//! a centroid too far from the lock point, a step outside the card's floor and
//! ceiling, a missing input, a name the card does not have. `validate_series`
//! compares the expected offset with the centroids and returns the error the
//! caller computes from the frames.
//!
//! The centroids read here are frames 469 to 1,720 of
//! `jw01163017001_gs-fg_2022092164131_cal.fits`, a file of the fitted set.
//! No held-back file is read.

mod answer_text;

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::los_jwst_fine_guide::{
    self, LosJwstFineGuide, LosLoad, LosState, StepReject,
};
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

#[allow(dead_code)]
mod reference {
    include!("../../genesis_kernel/src/los_jwst_fine_guide_reference.rs");
    pub const LOCK: [f64; 2] = [LOCK_X_ARCSEC, LOCK_Y_ARCSEC];
    pub const ROWS: &[(f64, f64, f64, f64)] = &FRAMES;
    pub const UNMEASURED: usize = UNMEASURED_FRAMES;
    pub const FIRST_REFUSED: usize = FIRST_REFUSED_FRAME;
    pub const FIRST_ABOVE_5_MAS: usize = FIRST_FRAME_ABOVE_5_MAS;
}

/// The time the runs here state since the last disturbance: the card's 30 s.
const QUIET_S: f64 = los_jwst_fine_guide::RECOVERY_S;

const DT_S: f64 = los_jwst_fine_guide::FRAME_S;

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

fn card() -> LosJwstFineGuide {
    LosJwstFineGuide::observatory()
}

fn load_of(row: (f64, f64, f64, f64)) -> LosLoad {
    LosLoad { measured: row.0, centroid_x_arcsec: row.1, centroid_y_arcsec: row.2, counts_per_s: row.3 }
}

fn load_json(row: (f64, f64, f64, f64)) -> Value {
    json!({ "measured": row.0, "centroid_x_arcsec": row.1, "centroid_y_arcsec": row.2, "counts_per_s": row.3 })
}

fn step_body(lock: [f64; 2], row: (f64, f64, f64, f64), dt_s: f64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "los_jwst_fine_guide",
        "parameters": {
            "lock_x_arcsec": lock[0], "lock_y_arcsec": lock[1], "quiet_before_start_s": QUIET_S,
            "measured": row.0, "centroid_x_arcsec": row.1, "centroid_y_arcsec": row.2, "counts_per_s": row.3,
            "dt_s": dt_s
        }
    })
}

fn run_body(lock: [f64; 2], rows: &[(f64, f64, f64, f64)], record_every: u64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "los_jwst_fine_guide",
        "initial": { "lock_x_arcsec": lock[0], "lock_y_arcsec": lock[1], "quiet_before_start_s": QUIET_S },
        "dt_s": DT_S,
        "segments": rows.iter().map(|row| json!({ "steps": 1, "load": load_json(*row) })).collect::<Vec<_>>(),
        "record_every": record_every
    })
}

fn blank_frame(proof: [u8; 16]) -> LastStateFrame64 {
    LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof }
}

/// The kernel loop over `rows` from this lock point: every landed state and
/// frame, and the refusal that stopped it.
fn direct_loop(lock: [f64; 2], rows: &[(f64, f64, f64, f64)]) -> (Vec<(LosState, LastStateFrame64)>, Option<StepReject>) {
    let card = card();
    let mut state = LosState::try_lock(&card, lock[0], lock[1], QUIET_S).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut kept = Vec::new();
    for row in rows {
        match los_jwst_fine_guide::step(&card, &mut state, &mut frame, load_of(*row), DT_S) {
            Ok(()) => kept.push((state, frame)),
            Err(reject) => return (kept, Some(reject)),
        }
    }
    (kept, None)
}

const COV: [&str; 10] = [
    "slow_slow", "slow_fast", "slow_cos", "slow_sin", "fast_fast", "fast_cos", "fast_sin", "cos_cos", "cos_sin", "sin_sin",
];

fn state_numbers(state: &LosState) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = vec![
        ("t_s".into(), state.t_s),
        ("lock_x_arcsec".into(), state.lock_x_arcsec),
        ("lock_y_arcsec".into(), state.lock_y_arcsec),
        ("quiet_before_start_s".into(), state.quiet_before_start_s),
        ("since_disturbance_s".into(), state.since_disturbance_s),
    ];
    for (tag, axis) in [("x", &state.x), ("y", &state.y)] {
        out.push((format!("slow_{tag}_mas"), axis.slow_mas));
        out.push((format!("fast_{tag}_mas"), axis.fast_mas));
        out.push((format!("mode_cos_{tag}_mas"), axis.mode_cos_mas));
        out.push((format!("mode_sin_{tag}_mas"), axis.mode_sin_mas));
    }
    for (tag, axis) in [("x", &state.x), ("y", &state.y)] {
        for (name, value) in COV.iter().zip(axis.cov_mas2) {
            out.push((format!("cov_{tag}_{name}_mas2"), value));
        }
    }
    for (name, value) in [
        ("predicted_x_mas", state.predicted_x_mas),
        ("predicted_y_mas", state.predicted_y_mas),
        ("innovation_x_mas", state.innovation_x_mas),
        ("innovation_y_mas", state.innovation_y_mas),
        ("innovation_variance_x_mas2", state.innovation_variance_x_mas2),
        ("innovation_variance_y_mas2", state.innovation_variance_y_mas2),
        ("estimate_x_mas", state.estimate_x_mas),
        ("estimate_y_mas", state.estimate_y_mas),
        ("estimate_variance_x_mas2", state.estimate_variance_x_mas2),
        ("estimate_variance_y_mas2", state.estimate_variance_y_mas2),
        ("nis", state.nis),
        ("measured_frames", state.measured_frames),
        ("nis_sum_x", state.nis_sum_x),
        ("nis_sum_y", state.nis_sum_y),
        ("variance_x_mas2", state.variance_x_mas2),
        ("variance_y_mas2", state.variance_y_mas2),
        ("variance_defect_x_mas2", state.variance_defect_x_mas2),
        ("variance_defect_y_mas2", state.variance_defect_y_mas2),
        ("residual_conservation_error", state.residual_conservation_error),
        ("unmeasured_in_row", state.unmeasured_in_row),
    ] {
        out.push((name.to_string(), value));
    }
    out
}

fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")].as_str().unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

/// Every published number is the kernel's, to the bit, as the decimal and as
/// the hex beside it; the one flag is the kernel's; nothing else is published.
fn assert_state_is(published_state: &Value, state: &LosState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    let numbers = state_numbers(state);
    assert_eq!(numbers.len(), 53);
    for (name, value) in numbers {
        let decimal = object[&name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, &name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(format!("{name}_hex"));
        keys.insert(name);
    }
    for (name, value) in [
        ("no_measurement", state.no_measurement),
        ("pointing_disturbed", state.pointing_disturbed),
        ("beyond_four_sigma", state.beyond_four_sigma),
        ("recovering", state.recovering),
    ] {
        assert_eq!(object[name], value, "{label} {name}");
        keys.insert(name.to_string());
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
    text.contains(&format!("({name})")) && !text.contains("sha256_seal") && !text.contains("run_seal")
}

/// The slots of one recorded frame: time, expected x and y, variance x,
/// estimate x and y, variance y, normalised innovation squared, flags.
fn slots(frame_hex: &str) -> (f64, [f32; 7], u64) {
    let bytes = hex::decode(frame_hex).unwrap();
    assert_eq!(bytes.len(), 64);
    let f32_at = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    (
        f64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        [f32_at(8), f32_at(12), f32_at(16), f32_at(20), f32_at(24), f32_at(28), f32_at(32)],
        u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
    )
}

const NO_MEASUREMENT: (f64, f64, f64, f64) = (0.0, 0.0, 0.0, 0.0);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_matches_the_compiled_step_with_the_same_proof_bytes_and_its_receipt_verifies() {
    let base = spawn_app().await;
    let lock = reference::LOCK;
    let cases = [
        step_body(lock, reference::ROWS[0], DT_S),
        step_body(lock, NO_MEASUREMENT, DT_S),
        // At the card's floor and ceiling, and at the two ends of its count rates.
        step_body(lock, reference::ROWS[1], los_jwst_fine_guide::FLOOR_DT_S),
        step_body(lock, reference::ROWS[2], los_jwst_fine_guide::GATE_DT_S),
        step_body(lock, (1.0, lock[0], lock[1], los_jwst_fine_guide::COUNTS_MIN_PER_S), DT_S),
        step_body(lock, (1.0, lock[0], lock[1], los_jwst_fine_guide::COUNTS_MAX_PER_S), DT_S),
        // Past the count rates the noise law was fitted on, where through
        // engine 0.11 the step was refused: it lands, flagged.
        step_body(lock, (1.0, lock[0], lock[1], 22_899.0), DT_S),
        step_body(lock, (1.0, lock[0], lock[1], 2_820_001.0), DT_S),
        step_body(lock, (1.0, lock[0], lock[1], 1.0), DT_S),
        step_body(lock, (1.0, lock[0], lock[1], 1.0e9), DT_S),
        // A lock point at two corners of the compared span, and a centroid 5 mas
        // from the centre on x.
        step_body(
            [los_jwst_fine_guide::LOCK_X_MAX_ARCSEC, los_jwst_fine_guide::LOCK_Y_MIN_ARCSEC],
            (1.0, los_jwst_fine_guide::LOCK_X_MAX_ARCSEC, los_jwst_fine_guide::LOCK_Y_MIN_ARCSEC, 500_000.0),
            DT_S,
        ),
        step_body(
            [los_jwst_fine_guide::LOCK_X_MIN_ARCSEC, los_jwst_fine_guide::LOCK_Y_MAX_ARCSEC],
            (1.0, los_jwst_fine_guide::LOCK_X_MIN_ARCSEC, los_jwst_fine_guide::LOCK_Y_MAX_ARCSEC, 500_000.0),
            DT_S,
        ),
        step_body([0.0, 0.0], (1.0, 0.005, 0.0, 30_000.0), DT_S),
    ];
    for request in cases {
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);

        let parameters = &request["parameters"];
        let number = |key: &str| parameters[key].as_f64().unwrap();
        let card = card();
        let mut state = LosState::try_lock(
            &card,
            number("lock_x_arcsec"),
            number("lock_y_arcsec"),
            number("quiet_before_start_s"),
        )
        .unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        let load = LosLoad {
            measured: number("measured"),
            centroid_x_arcsec: number("centroid_x_arcsec"),
            centroid_y_arcsec: number("centroid_y_arcsec"),
            counts_per_s: number("counts_per_s"),
        };
        los_jwst_fine_guide::step(&card, &mut state, &mut frame, load, number("dt_s")).unwrap();
        assert_eq!(
            frame.flags & los_jwst_fine_guide::FLAG_COUNT_RATE_BEYOND_COMPARED != 0,
            card.count_rate_beyond_compared(load.counts_per_s) && load.measured == 1.0
        );
        assert_state_is(&response.state, &state, "step");
        assert_eq!(response.residual_conservation_error.to_bits(), state.residual_conservation_error.to_bits());
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        let mut naked = frame;
        naked.proof = [0; 16];
        assert_ne!(response.sha256_seal, sha256_hex(&naked.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha256);
        assert_eq!(sha256_hex(document.as_bytes()), receipt);
        let (status, report) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], true, "{report}");
    }
}

/// The reference run, which engines 0.7 to 0.9 ended at its first centroid
/// beyond four sigma, frame 1,200: every frame now lands, bit for bit as the
/// kernel's loop, with the ledger closed; the frames that 0.10 refused carry
/// their flags; the tape and the receipt verify.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_reference_run_lands_every_frame_and_flags_its_disturbances() {
    let base = spawn_app().await;
    let lock = reference::LOCK;
    let rows = reference::ROWS;
    let quiet_end = reference::FIRST_REFUSED;
    assert_eq!(rows.len(), 1_252);
    assert_eq!(quiet_end, 1_199);
    let request = run_body(lock, rows, 1);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 1_252);
    let frames = tape["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 1_252);
    let (kept, reject) = direct_loop(lock, rows);
    assert_eq!(reject, None);
    assert_eq!(kept.len(), 1_252);
    assert_state_is(&tape["final_state"], &kept[1_251].0, "the last frame");
    let worst = kept.iter().map(|(state, _)| state.residual_conservation_error).fold(0.0, f64::max);
    assert_eq!(tape["max_residual_conservation_error"].as_f64().unwrap().to_bits(), worst.to_bits());
    assert!(worst <= los_jwst_fine_guide::RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
    let quiet_flags = los_jwst_fine_guide::FLAG_NO_MEASUREMENT;
    for (index, frame) in frames.iter().enumerate() {
        let (t_s, _, flags) = slots(frame["frame_hex"].as_str().unwrap());
        let (state, kernel_frame) = &kept[index];
        assert_eq!(t_s.to_bits(), state.t_s.to_bits());
        assert_eq!(flags, kernel_frame.flags, "frame {index}");
        if index < quiet_end {
            assert_eq!(flags & !quiet_flags, 0, "frame {index}");
        }
    }
    let first = slots(frames[quiet_end]["frame_hex"].as_str().unwrap()).2;
    assert!(first & los_jwst_fine_guide::FLAG_BEYOND_FOUR_SIGMA != 0, "{first}");
    let above = slots(frames[reference::FIRST_ABOVE_5_MAS]["frame_hex"].as_str().unwrap()).2;
    assert!(above & los_jwst_fine_guide::FLAG_POINTING_DISTURBED != 0, "{above}");
    let after = slots(frames[quiet_end + 1]["frame_hex"].as_str().unwrap()).2;
    assert!(after & (los_jwst_fine_guide::FLAG_RECOVERING | los_jwst_fine_guide::FLAG_BEYOND_FOUR_SIGMA) != 0, "{after}");
    assert!(published(&tape["final_state"], "since_disturbance_s") < 30.0);

    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
    let (status, report) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["matches"], true, "{report}");
    let (_, again) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(again, answer);
    let mut forged = tape.clone();
    let hex = forged["final_state"]["nis_sum_x_hex"].as_str().unwrap().to_string();
    forged["final_state"]["nis_sum_x_hex"] = json!(format!("{:016x}", u64::from_str_radix(&hex, 16).unwrap() + 1));
    let (_, report) = post(&base, "/verify_tape", &forged).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");
}

/// A centroid more than 5 mas from the lock point, where 0.10 ended the
/// run, lands with FLAG_POINTING_DISTURBED; the 469 frames after it carry
/// FLAG_RECOVERING; 5 mas exactly is quiet.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_centroid_more_than_5_mas_from_the_lock_point_is_answered_and_flagged() {
    let base = spawn_app().await;
    let lock = [0.0, 0.0];
    let quiet = (1.0, 0.0001, -0.0002, 600_000.0);
    let mut rows = vec![quiet; 520];
    rows[10] = NO_MEASUREMENT;
    rows[30] = (1.0, 0.008, -0.0002, 600_000.0);
    let (status, text) = post(&base, "/run_trajectory", &run_body(lock, &rows, 1)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{text}");
    assert_eq!(tape["steps_landed"], 520);
    let flags: Vec<u64> = tape["frames"].as_array().unwrap().iter().map(|f| slots(f["frame_hex"].as_str().unwrap()).2).collect();
    assert_eq!(flags[10], los_jwst_fine_guide::FLAG_NO_MEASUREMENT);
    assert!(flags[30] & los_jwst_fine_guide::FLAG_POINTING_DISTURBED != 0);
    // The filter follows the jump for a few frames, which may themselves be
    // beyond four sigma; the 468 frames after the last flagged one recover.
    let marked = los_jwst_fine_guide::FLAG_POINTING_DISTURBED | los_jwst_fine_guide::FLAG_BEYOND_FOUR_SIGMA;
    let last = (30..520).filter(|&index| flags[index] & marked != 0).max().unwrap();
    assert!(last < 40, "{last}");
    for index in 31..=last + 468 {
        assert!(flags[index] & (marked | los_jwst_fine_guide::FLAG_RECOVERING) != 0, "frame {index}");
    }
    for index in last + 1..=last + 468 {
        assert!(flags[index] & los_jwst_fine_guide::FLAG_RECOVERING != 0, "frame {index}");
    }
    assert_eq!(flags[last + 469] & los_jwst_fine_guide::FLAG_RECOVERING, 0);
    let (kept, reject) = direct_loop(lock, &rows);
    assert_eq!(reject, None);
    for (index, (_, frame)) in kept.iter().enumerate() {
        assert_eq!(flags[index], frame.flags, "frame {index}");
    }
    assert_state_is(&tape["final_state"], &kept[519].0, "after the disturbance");
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");

    let edge = (1.0, 0.005, 0.0, 30_000.0);
    let (status, text) = post(&base, "/run_trajectory", &run_body(lock, &[edge], 1)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{text}");
    assert_eq!(slots(tape["frames"][0]["frame_hex"].as_str().unwrap()).2, 0);
    let over = (1.0, f64::from_bits(0.005_f64.to_bits() + 1), 0.0, 30_000.0);
    for row in [over, (1.0, -over.1, 0.0, 30_000.0), (1.0, 0.0, over.1, 30_000.0), (1.0, 0.0, -over.1, 30_000.0)] {
        let (status, text) = post(&base, "/verify_physics", &step_body(lock, row, DT_S)).await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["state"]["pointing_disturbed"], true, "{text}");
    }
    let (status, text) = post(&base, "/verify_physics", &step_body(lock, (1.0, 0.0, 0.003, 2.0e6), DT_S)).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["state"]["beyond_four_sigma"], true, "{text}");
}

/// Where 0.10 ended a run at ten frames in a row with no measurement, a
/// run of 100,000 such frames between two measured ones completes, with the
/// ledger closed, and its tape verifies.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_long_run_of_frames_with_no_measurement_is_predicted() {
    let base = spawn_app().await;
    let lock = [0.0, 0.0];
    let quiet = (1.0, 0.0003, -0.0002, 500_000.0);
    let after = (1.0, 0.0001, -0.0001, 500_000.0);
    let mut rows = vec![quiet];
    rows.extend(std::iter::repeat_n(NO_MEASUREMENT, 11));
    rows.push(after);
    let (status, text) = post(&base, "/run_trajectory", &run_body(lock, &rows, 1)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{text}");
    let (kept, reject) = direct_loop(lock, &rows);
    assert_eq!(reject, None);
    assert_eq!(kept[11].0.unmeasured_in_row, 11.0);
    assert_state_is(&tape["final_state"], &kept[12].0, "after eleven frames with none");
    let request = json!({
        "domain": "aerospace",
        "card": "los_jwst_fine_guide",
        "initial": { "lock_x_arcsec": 0.0, "lock_y_arcsec": 0.0, "quiet_before_start_s": 30.0 },
        "dt_s": 0.064,
        "record_every": 100_002,
        "segments": [
            { "steps": 1, "load": { "measured": 1.0, "centroid_x_arcsec": 0.0003, "centroid_y_arcsec": -0.0002, "counts_per_s": 500_000.0 } },
            { "steps": 100_000, "load": { "measured": 0.0, "centroid_x_arcsec": 0.0, "centroid_y_arcsec": 0.0, "counts_per_s": 0.0 } },
            { "steps": 1, "load": { "measured": 1.0, "centroid_x_arcsec": 0.0031, "centroid_y_arcsec": -0.0019, "counts_per_s": 500_000.0 } }
        ]
    });
    let (status, text) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{text}");
    assert_eq!(tape["steps_landed"], 100_002);
    assert!(tape["max_residual_conservation_error"].as_f64().unwrap() <= los_jwst_fine_guide::RESIDUAL_CONSERVATION_LIMIT);
    assert_eq!(published(&tape["final_state"], "unmeasured_in_row"), 0.0);
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
}

/// A run stated to start less than 30 s after a disturbance, which 0.10
/// refused, lands and its first frames carry FLAG_RECOVERING; 30 s and the
/// allowance under it do not; below zero is refused by name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_stated_to_start_soon_after_a_disturbance_is_answered_and_flagged() {
    let base = spawn_app().await;
    let lock = reference::LOCK;
    let rows = &reference::ROWS[..20];
    let with_quiet = |quiet: Value| {
        let mut request = run_body(lock, rows, 1);
        request["initial"]["quiet_before_start_s"] = quiet;
        request
    };
    let allowance = los_jwst_fine_guide::RECOVERY_ALLOWANCE_S;
    for (quiet, recovering) in [
        (30.0, false),
        (469.0 * 0.064, false),
        (1030.1 - 1000.1, false),
        (30.0 - allowance, false),
        (3600.0, false),
        (468.0 * 0.064, true),
        (29.9, true),
        (0.064, true),
        (0.0, true),
    ] {
        let (status, text) = post(&base, "/run_trajectory", &with_quiet(json!(quiet))).await;
        assert_eq!(status, 200, "{quiet}: {text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{quiet}");
        assert_eq!(tape["steps_landed"], 20);
        let first = slots(tape["frames"][0]["frame_hex"].as_str().unwrap()).2;
        assert_eq!(first & los_jwst_fine_guide::FLAG_RECOVERING != 0, recovering && quiet + DT_S < 30.0, "{quiet}");
    }
    let (status, text) = post(&base, "/run_trajectory", &with_quiet(json!(-30.0))).await;
    assert_eq!(status, 422, "{text}");
    assert!(names(&text, "QuietBeforeStartBelowZero"), "{text}");
    let mut request = run_body(lock, rows, 1);
    request["initial"].as_object_mut().unwrap().remove("quiet_before_start_s");
    let (status, text) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("quiet_before_start_s") && !text.contains("run_seal"), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_fault_a_request_shows_is_named_and_carries_no_proof() {
    let base = spawn_app().await;
    let lock = reference::LOCK;
    let good = reference::ROWS[0];
    const LOCK_X_MIN: f64 = los_jwst_fine_guide::LOCK_X_MIN_ARCSEC;
    const LOCK_X_MAX: f64 = los_jwst_fine_guide::LOCK_X_MAX_ARCSEC;
    const LOCK_Y_MIN: f64 = los_jwst_fine_guide::LOCK_Y_MIN_ARCSEC;
    const LOCK_Y_MAX: f64 = los_jwst_fine_guide::LOCK_Y_MAX_ARCSEC;
    // The next 64-bit number further from zero.
    let out = |v: f64| f64::from_bits(v.to_bits() + 1);
    let cases: Vec<(Value, &str)> = vec![
        (step_body([70.000_001, 0.0], (1.0, 70.0, 0.0, 5.0e5), DT_S), "LockOutsideSensorField"),
        (step_body([0.0, -70.000_001], (1.0, 0.0, -70.0, 5.0e5), DT_S), "LockOutsideSensorField"),
        (step_body(lock, (0.5, good.1, good.2, good.3), DT_S), "MeasuredNotZeroOrOne"),
        (step_body(lock, (2.0, good.1, good.2, good.3), DT_S), "MeasuredNotZeroOrOne"),
        (step_body(lock, (-1.0, good.1, good.2, good.3), DT_S), "MeasuredNotZeroOrOne"),
        (step_body(lock, (0.0, good.1, good.2, good.3), DT_S), "UnmeasuredFrameCarriesNumbers"),
        (step_body(lock, (0.0, 0.0, 0.0, 1.0), DT_S), "UnmeasuredFrameCarriesNumbers"),
        (step_body(lock, (1.0, good.1, good.2, 0.0), DT_S), "CountRateNotPositive"),
        (step_body(lock, (1.0, good.1, good.2, -1.0), DT_S), "CountRateNotPositive"),
        (step_body(lock, (1.0, good.1, good.2, 1.0e-240), DT_S), "CountRateNotPositive"),
        (step_body(lock, (1.0, lock[0] + 0.2501, good.2, good.3), DT_S), "OffsetOutsideCard"),
        (step_body(lock, (1.0, good.1, good.2 - 0.26, good.3), DT_S), "OffsetOutsideCard"),
        (step_body(lock, good, 0.0), "NonPositiveDt"),
        (step_body(lock, good, -DT_S), "NonPositiveDt"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/verify_physics", &request).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(names(&text, name), "{name}: {text}");
    }
    // Where 0.10 refused the lock point, past the compared span and up to the
    // sensor's field, a step lands and seals.
    for (lock_point, row) in [
        ([70.0, 70.0], (1.0, 70.0003, 69.9998, 5.0e5)),
        ([out(LOCK_X_MAX), 0.0], (1.0, LOCK_X_MAX, 0.0, 5.0e5)),
        ([out(LOCK_X_MIN), 0.0], (1.0, LOCK_X_MIN, 0.0, 5.0e5)),
        ([0.0, out(LOCK_Y_MAX)], (1.0, 0.0, LOCK_Y_MAX, 5.0e5)),
        ([0.0, out(LOCK_Y_MIN)], (1.0, 0.0, LOCK_Y_MIN, 5.0e5)),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body(lock_point, row, DT_S)).await;
        assert_eq!(status, 200, "{lock_point:?}: {text}");
        assert!(text.contains("sha256_seal"), "{text}");
    }
    // A step outside the card's floor and ceiling.
    for dt_s in [0.0634, 0.0646, 0.032, 0.128, 1.0] {
        let (status, text) = post(&base, "/verify_physics", &step_body(lock, good, dt_s)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains("of the los_jwst_fine_guide card") && !text.contains("sha256_seal"), "{text}");
    }
    // A missing input, and a name the card does not have.
    for missing in ["lock_x_arcsec", "quiet_before_start_s", "measured", "centroid_y_arcsec", "counts_per_s", "dt_s"] {
        let mut request = step_body(lock, good, DT_S);
        request["parameters"].as_object_mut().unwrap().remove(missing);
        let (status, text) = post(&base, "/verify_physics", &request).await;
        assert_eq!(status, 422, "{missing}: {text}");
        assert!(text.contains(missing) && !text.contains("sha256_seal"), "{text}");
    }
    let mut request = step_body(lock, good, DT_S);
    request["parameters"]["noise_reference_mas2"] = json!(0.01);
    let (status, text) = post(&base, "/verify_physics", &request).await;
    assert_eq!(status, 422, "{text}");
    assert!(!text.contains("sha256_seal"), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_frame_off_the_card_in_a_later_segment_refuses_the_whole_run_before_its_first_step() {
    let base = spawn_app().await;
    let lock = reference::LOCK;
    let mut rows: Vec<(f64, f64, f64, f64)> = reference::ROWS[..50].to_vec();
    for (bad, name) in [
        ((1.0, rows[0].1, rows[0].2, 0.0), "CountRateNotPositive"),
        ((1.0, rows[0].1 + 1.0, rows[0].2, rows[0].3), "OffsetOutsideCard"),
        ((0.25, rows[0].1, rows[0].2, rows[0].3), "MeasuredNotZeroOrOne"),
        ((0.0, rows[0].1, 0.0, 0.0), "UnmeasuredFrameCarriesNumbers"),
    ] {
        let kept = rows[40];
        rows[40] = bad;
        let (status, text) = post(&base, "/run_trajectory", &run_body(lock, &rows, 1)).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(names(&text, name), "{name}: {text}");
        rows[40] = kept;
    }
    // The same fifty frames as they are land.
    let (status, text) = post(&base, "/run_trajectory", &run_body(lock, &rows, 50)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["steps_landed"], 50);
    // A star brighter and a star fainter than any the noise law was fitted
    // on, where through engine 0.11 the run was refused: every frame lands,
    // flagged, bit for bit as the kernel's loop, with the ledger closed.
    for counts_per_s in [3.0e6, 1.0e9, 5_000.0, 10.0] {
        let bright: Vec<(f64, f64, f64, f64)> =
            rows.iter().map(|row| if row.0 == 1.0 { (row.0, row.1, row.2, counts_per_s) } else { *row }).collect();
        let (status, text) = post(&base, "/run_trajectory", &run_body(lock, &bright, 1)).await;
        assert_eq!(status, 200, "{counts_per_s}: {text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{counts_per_s}");
        assert_eq!(tape["steps_landed"], 50);
        let (kept, reject) = direct_loop(lock, &bright);
        assert_eq!(reject, None);
        assert_state_is(&tape["final_state"], &kept[49].0, "the last frame");
        for (index, frame) in tape["frames"].as_array().unwrap().iter().enumerate() {
            let flags = slots(frame["frame_hex"].as_str().unwrap()).2;
            assert_eq!(flags, kept[index].1.flags, "{counts_per_s}: frame {index}");
            assert_eq!(flags & los_jwst_fine_guide::FLAG_COUNT_RATE_BEYOND_COMPARED != 0, bright[index].0 == 1.0);
        }
        assert!(tape["max_residual_conservation_error"].as_f64().unwrap() <= los_jwst_fine_guide::RESIDUAL_CONSERVATION_LIMIT);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_gives_the_error_of_the_expected_offset_against_the_centroids() {
    let base = spawn_app().await;
    let lock = reference::LOCK;
    let rows = &reference::ROWS[..1_100];
    let run = run_body(lock, rows, 1_100);
    let (_, alone) = post(&base, "/run_trajectory", &run).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["outcome"], "completed");
    let (kept, _) = direct_loop(lock, rows);

    // The measured frames, as the offset of the centroid from the lock point in mas.
    let mut measurements = Vec::new();
    let mut sum = 0.0;
    let mut count = 0.0;
    for (index, row) in rows.iter().enumerate() {
        if row.0 == 1.0 {
            let offset_x_mas = (row.1 - lock[0]) * 1000.0;
            measurements.push(json!({ "step": index as u64 + 1, "value": offset_x_mas }));
            let error = kept[index].0.predicted_x_mas - offset_x_mas;
            sum += error * error;
            count += 1.0;
        }
    }
    let probe = json!({ "run": run, "quantity": "predicted_x_mas", "measurements": measurements });
    let (status, text) = post(&base, "/validate_series", &probe).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["outcome"], "completed");
    assert_eq!(answer["compared"], count as u64);
    let rms: f64 = (sum / count as f64).sqrt();
    assert!((answer["rms_error"].as_f64().unwrap() - rms).abs() < 1.0e-12 * rms, "{text}");
    assert!(answer.get("pass").is_none() && answer.get("verdict").is_none());
    // The model's values are the kernel's expected offsets, to the bit.
    let model = answer["model_values_hex"].as_array().unwrap();
    let mut at = 0;
    for (index, row) in rows.iter().enumerate() {
        if row.0 == 1.0 {
            let value = f64::from_bits(u64::from_str_radix(model[at].as_str().unwrap(), 16).unwrap());
            assert_eq!(value.to_bits(), kept[index].0.predicted_x_mas.to_bits(), "step {}", index + 1);
            at += 1;
        }
    }
    // A flag is not a quantity.
    let body = json!({ "run": run, "quantity": "no_measurement", "measurements": [{ "step": 1, "value": 0.0 }] });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 422, "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_card_is_listed_and_its_description_says_what_is_fitted_and_what_was_held_back() {
    let base = spawn_app().await;
    let client = Client::new();
    let cards: Value = client.get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let cards = cards["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 40);
    let mine = cards.iter().find(|card| card["card"] == "los_jwst_fine_guide").expect("listed");
    assert_eq!(mine["domain"], "aerospace");
    assert!(mine["specimen"].as_str().unwrap().contains("fitted to this observatory's own centroids"));

    let (status, text) = post(&base, "/describe_card", &json!({"domain": "aerospace", "card": "los_jwst_fine_guide"})).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let rejects: Vec<&str> = body["rejects"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    // After the kernel's 13 names the list holds the gateway's own three for this card, DtBelowFloor and DtAboveCeiling and ContinuationNotOnCard.
    assert_eq!(rejects.len(), 16);
    for name in ["RecoveryNotElapsed", "PointingDisturbance", "InnovationBeyondFourSigma", "LockOutsideCard", "MeasurementGapOutsideCard"] {
        assert!(!rejects.contains(&name), "{name}");
    }
    assert!(rejects.contains(&"LockOutsideSensorField") && rejects.contains(&"QuietBeforeStartBelowZero"));
    let flags = body["advisory_flags"].as_array().unwrap();
    let flag_names: Vec<&str> = flags.iter().map(|flag| flag["name"].as_str().unwrap()).collect();
    assert_eq!(
        flag_names,
        ["FLAG_NO_MEASUREMENT", "FLAG_POINTING_DISTURBED", "FLAG_BEYOND_FOUR_SIGMA", "FLAG_RECOVERING", "FLAG_COUNT_RATE_BEYOND_COMPARED"]
    );
    assert!(rejects.contains(&"CountRateNotPositive") && rejects.contains(&"FirstLawBreach"));
    assert!(!rejects.contains(&"CountRateOutsideCard"));
    let validation = &body["validation"];
    assert!(validation["data"].as_str().unwrap().contains("programme 1163"));
    assert!(validation["pass_line"].as_str().unwrap().contains("9e160724092e182f847eb0103e6ca7a54aba0f75c5d1ca14dcd275c9a554c8f5"));
    assert!(validation["limits"].as_str().unwrap().contains("extrapolation in time and to other guide stars"));
    // What was compared of the lock point and of the gaps, and that the card computes past both.
    let limits = validation["limits"].as_str().unwrap();
    assert!(limits.contains("span -61.1512947 to 60.4564934 arcsec on x and -64.2767 to 50.1812668 arcsec on y"), "{limits}");
    assert!(limits.contains("10 frames on the fitted files and 2 on the files outside the fit"), "{limits}");
    let fields = body["phase_fields"].as_array().unwrap();
    let field = |name: &str| fields.iter().find(|f| f["name"] == name).unwrap_or_else(|| panic!("{name}"));
    assert_eq!(field("lock_x_arcsec")["minimum"], json!(-70.0));
    assert_eq!(field("lock_x_arcsec")["maximum"], json!(70.0));
    assert_eq!(field("lock_y_arcsec")["minimum"], json!(-70.0));
    assert_eq!(field("lock_y_arcsec")["maximum"], json!(70.0));
    assert_eq!(field("quiet_before_start_s")["minimum"], json!(0.0));
    // Sentences held in the one line of the list, the data, the result and
    // the limits.
    assert!(mine["specimen"].as_str().unwrap().contains(
        "Gap: inside disturbances it is worse than the last centroid, 2.03 against 1.00 mas on x and 1.57 against 0.78 on y, its stated variance 12 to 17 times too small; it flags those frames."
    ));
    let rule = body["dt_s"]["rule"].as_str().unwrap();
    for words in [
        "FLAG_POINTING_DISTURBED on a measured centroid more than 5 mas from the lock point",
        "FLAG_BEYOND_FOUR_SIGMA on a measured centroid more than four stated standard deviations",
        "FLAG_RECOVERING on a frame that ends less than 30 s after the last centroid that carried either",
        "2.03 mas against 1.00 for the last centroid on x and 1.57 against 0.78 on y",
        "once in about 7,900 measured frames",
        "the longest run of such frames a compared run crossed was 10 frames on the fitted files and 2 on the files outside the fit",
        "one beyond it is refused as LockOutsideSensorField before the first step",
    ] {
        assert!(rule.contains(words), "{words}: {rule}");
    }
    let data = validation["data"].as_str().unwrap();
    for words in [
        "the slow part 1.03257 and 0.455171 mas with a memory of 66.7485 and 12.2188 s",
        "the fast part 0.142973 and 0.156568 mas with a memory of 0.0980239 and 0.0943062 s",
        "the mode 0.136597 and 0.291622 mas at 0.543703 and 0.302547 Hz with a memory of 5.88667 and 14.6184 s",
        "0.0360528 and 0.0253144 mas2 at 1,000,000 counts per second, falling as the count rate to the power 1.15083 and 1.27308",
        "under NASA contract NAS5-03127 for JWST. These observations are associated with program #1163.",
    ] {
        assert!(data.contains(words), "{words}: {data}");
    }
    let result = validation["result"].as_str().unwrap();
    for words in [
        "against 0.644 and 0.584 for taking the last centroid (0.790 and 0.807 of it; target 0.85) and 0.660 and 0.748 for taking the lock point (0.772 and 0.631 of it; target 0.85)",
        "With the disturbed frames taken out, as a chain set with the numbers above read",
        "The three numbers are the record's own and none is fitted to a file outside the fit",
        "Over all 973,388 measured frames outside the fit, quiet and disturbed together, the card's error one frame ahead is 1.35 on x and 1.26 on y of the error of taking the last centroid",
        "the squared error over the variance it states is 3.9 and 3.0",
        "Disturbances are in 121 of the 134 files and make up 18.3 percent of the measured frames",
        "lies in 0.5 to 2.0 in 39 of the 134 files",
        "The card answers those frames and marks them with three flags",
        "0.510 mas on x and 0.472 mas on y",
        "A random walk with the same power-law sensor noise, three constants an axis fitted on the same April data, is inside all five targets, at 0.808 and 0.837",
        "lower the error by 2 and 4 percent: 0.510 and 0.472 mas against 0.521 and 0.489",
        "The targets do not test the mode frequencies: with both moved 20 percent up or down every target still holds",
        "a second-order autoregression with one variance is outside the per-file target",
        "the y noise exponent comes out 1.045 against the card's 1.273, where x gives 1.134 against 1.151",
        "differ by +27 to -21 percent in variance, which the variance target cannot resolve",
        "each number within 0.04 of the one above, and the lock-point target is no longer a test",
        "43 files, which lose 12 percent of the set's frames",
        "A second comparison program, with its own filter on whole files and its own FITS reader, on the same machine, gives the same numbers",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let limits = validation["limits"].as_str().unwrap();
    for words in [
        "a second comparison program, on one machine, gives the same numbers",
        "The card answers there and flags the frames",
        "the visits of March 2022 are unopened",
        "the Nyquist frequency is 7.81 Hz, and the card cannot see or state jitter above it, nor tell a folded line from a true one",
        "a narrow line at 0.290 to 0.294 Hz with a shoulder at 0.32 to 0.34 Hz; the card's 0.3025 Hz is one damped mode laid across both",
        "(arXiv:2207.05632, section 3)",
        "a 0.045 Hz oscillation attributed to fuel slosh; the card has no part for the 0.045 Hz one",
        "a broad hump at 0.50 to 0.60 Hz, for which no published attribution was found",
        "it holds with a wrong mode frequency, wrong noise or the two axes swapped, and it says nothing about the telescope",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    // The worked run is the first eight frames of the reference run, 469 frames
    // after the file's first centroid, and it lands.
    let example = &body["example_run_trajectory"];
    let mut expected = run_body(reference::LOCK, &reference::ROWS[..8], 1);
    expected["initial"]["quiet_before_start_s"] = json!(30.016);
    assert_eq!(*example, expected);
    let (status, text) = post(&base, "/run_trajectory", example).await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = post(&base, "/verify_physics", &body["example_compute_step"]).await;
    assert_eq!(status, 200, "{text}");
}
