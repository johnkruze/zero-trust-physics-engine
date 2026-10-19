//! The CMSX-6 creep card on the web connection.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes. A run from 0.1 percent creep extension under a held stress ends in
//! the sealed refusal at 5 percent at every stress, and its last landed state
//! is the kernel loop's, to the bit; below 170 MPa its frames past 0.2 percent
//! carry FLAG_OUTSIDE_MET_RANGE. The tape of that run verifies and its
//! receipt matches. A start below 0.1 percent, zero among them, lands with
//! the same flag, as a step, as a run and as a row of a dataset, and a start
//! below zero is refused by name and carries no proof. A later segment with
//! another stress refuses the whole run before its first step. Every
//! published bound is refused by name at the last representable number on
//! each side. A constant of the card sent as a field is refused.
//!
//! The stresses of the runs are those of the fitted tests (140, 170 and
//! 230 MPa) and values between them. No measured value is in this file.

mod answer_text;

use std::time::Duration;

use genesis_kernel::creep_cmsx6::{self, CreepCmsx6, CreepLoad, CreepState, StepReject};
use genesis_kernel::frame::LastStateFrame64;
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha384};
use tokio::net::TcpListener;

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

fn sha384_hex(bytes: &[u8]) -> String {
    hex::encode(Sha384::digest(bytes))
}

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

fn alloy() -> CreepCmsx6 {
    CreepCmsx6::at_980_c()
}

fn ceiling() -> f64 {
    alloy().gate_dt_s().unwrap()
}

fn floor() -> f64 {
    alloy().floor_dt_s().unwrap()
}

/// The one advisory flag the card declares. No other bit may be set.
const DECLARED_FLAGS: u64 = creep_cmsx6::FLAG_CREEP_RATE_RISING | creep_cmsx6::FLAG_OUTSIDE_MET_RANGE;

/// The lowest creep extension of the card: every run here starts at or above it.
const START: f64 = creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT;

const STEPS_ASKED: u64 = 110_000;
const RECORD_EVERY: u64 = 1_000;

fn step_body(extension_percent: f64, stress_mpa: f64, dt_s: f64) -> Value {
    json!({
        "domain": "materials",
        "card": "creep_cmsx6",
        "parameters": {
            "creep_extension_percent": extension_percent,
            "stress_mpa": stress_mpa,
            "dt_s": dt_s
        }
    })
}

fn run_body(extension_percent: f64, stress_mpa: f64, dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "materials",
        "card": "creep_cmsx6",
        "initial": { "creep_extension_percent": extension_percent },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": { "stress_mpa": stress_mpa } }],
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

/// The kernel loop: steps of `dt_s` under a held stress until the kernel refuses.
fn direct_loop(extension_percent: f64, stress_mpa: f64, dt_s: f64, limit: u64) -> (CreepState, u64, Option<StepReject>) {
    let alloy = alloy();
    let mut state = CreepState::try_new(&alloy, stress_mpa, extension_percent).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut landed = 0;
    while landed < limit {
        match creep_cmsx6::step(&alloy, &mut state, &mut frame, CreepLoad { stress_mpa }, dt_s) {
            Ok(()) => landed += 1,
            Err(reject) => return (state, landed, Some(reject)),
        }
    }
    (state, landed, None)
}

/// The published number of a state, by its name, from the hex beside it.
fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn state_numbers(state: &CreepState) -> Vec<(&'static str, f64)> {
    vec![
        ("t_s", state.t_s),
        ("creep_extension_percent", state.creep_extension_percent),
        ("creep_rate_percent_per_h", state.creep_rate_percent_per_h),
        ("stress_mpa", state.stress_mpa),
        ("work_by_load_mj_per_m3", state.work_by_load_mj_per_m3),
        ("heat_dissipated_mj_per_m3", state.heat_dissipated_mj_per_m3),
        ("work_defect_mj_per_m3", state.work_defect_mj_per_m3),
        ("residual_conservation_error", state.residual_conservation_error),
    ]
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// both as the decimal and as the hex beside it, and nothing else is published.
fn assert_state_is(published_state: &Value, state: &CreepState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in state_numbers(state) {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    assert_eq!(object["creep_rate_rising"], state.creep_rate_rising, "{label}");
    keys.insert("creep_rate_rising".to_string());
    assert_eq!(object["outside_met_range"], state.outside_met_range, "{label}");
    keys.insert("outside_met_range".to_string());
    assert!(state.creep_extension_percent >= 0.0, "{label}");
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha384\":\"";
    let at = text.rfind(MARKER).expect("a receipt");
    (format!("{}}}", &text[..at]), text[at + MARKER.len()..text.len() - 2].to_string())
}

// -------------------------------------------------------------- the step

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_creep_card_matches_the_compiled_step_with_the_same_proof_bytes() {
    let base = spawn_app().await;
    let cases = [
        step_body(START, 200.0, ceiling()),
        step_body(START, creep_cmsx6::STRESS_MAX_MPA, ceiling()),
        step_body(0.19, creep_cmsx6::STRESS_MIN_MPA, floor()),
        step_body(0.37, 170.0, 30.0),
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
        let alloy = alloy();
        let mut state = CreepState::try_new(
            &alloy,
            parameters["stress_mpa"].as_f64().unwrap(),
            parameters["creep_extension_percent"].as_f64().unwrap(),
        )
        .unwrap();
        let started_at = state.t_s;
        assert_eq!(
            started_at.to_bits(),
            alloy
                .law_time_since_load_s(
                    parameters["stress_mpa"].as_f64().unwrap(),
                    parameters["creep_extension_percent"].as_f64().unwrap()
                )
                .to_bits()
        );
        let mut frame = blank_frame(commit_request_proof(&request));
        creep_cmsx6::step(
            &alloy,
            &mut state,
            &mut frame,
            CreepLoad { stress_mpa: parameters["stress_mpa"].as_f64().unwrap() },
            parameters["dt_s"].as_f64().unwrap(),
        )
        .unwrap();
        assert_eq!(frame.flags & !DECLARED_FLAGS, 0);
        // The clock is the law's own time to the start extension, and one step more.
        assert_eq!(state.t_s.to_bits(), (started_at + parameters["dt_s"].as_f64().unwrap()).to_bits());
        assert!(started_at > 2_000.0, "{started_at}");
        assert_state_is(&response.state, &state, "step");
        assert_eq!(
            response.residual_conservation_error.to_bits(),
            state.residual_conservation_error.to_bits()
        );
        assert_eq!(response.sha384_seal, sha384_hex(&frame.to_bytes()));
        // The seal covers the proof: the same frame with a zero proof seals apart.
        let mut naked = frame;
        naked.proof = [0; 16];
        assert_ne!(response.sha384_seal, sha384_hex(&naked.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha384);
        assert_eq!(sha384_hex(document.as_bytes()), receipt);
        seals.push(response.sha384_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 4, "four different steps, four different seals");
}

/// A single step handed back as a new start continues the same test: the
/// extension after two single steps is the extension after a run of two steps.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn single_steps_chain_through_the_creep_extension() {
    let base = spawn_app().await;
    let (_, first) = post(&base, "/verify_physics", &step_body(0.25, 215.0, 40.0)).await;
    let first: Value = serde_json::from_str(&first).unwrap();
    let reached = published(&first["state"], "creep_extension_percent");
    let (_, second) = post(&base, "/verify_physics", &step_body(reached, 215.0, 40.0)).await;
    let second: Value = serde_json::from_str(&second).unwrap();
    let (state, landed, reject) = direct_loop(0.25, 215.0, 40.0, 2);
    assert_eq!((landed, reject), (2, None));
    assert_eq!(
        published(&second["state"], "creep_extension_percent").to_bits(),
        state.creep_extension_percent.to_bits()
    );
    // The clock continues: the second single step starts at the law's own time to
    // the extension the first reached, which is the run's clock to the error of
    // the stepping. The books begin again at each single step.
    let second_t = published(&second["state"], "t_s");
    assert_eq!(second_t.to_bits(), (alloy().law_time_since_load_s(215.0, reached) + 40.0).to_bits());
    assert!(((second_t - state.t_s) / state.t_s).abs() < 1.0e-9, "{second_t} {}", state.t_s);
    assert!(published(&second["state"], "work_by_load_mj_per_m3") < state.work_by_load_mj_per_m3);
}

// ---------------------------------------------------------------- the run

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_under_a_held_stress_ends_in_the_sealed_refusal_at_the_limit_for_its_stress_with_the_state_of_the_kernel_loop() {
    let base = spawn_app().await;
    // (stress, limit of creep extension, steps the run lands at the ceiling
    // from 0.1 percent). Through engine 0.9 the limit below 170 MPa was 0.2
    // percent, and the 140 MPa run ended after 5,159 steps.
    for (stress, limit, steps) in [
        (140.0, 5.0, 102_748_u64),
        (169.0, 5.0, 0),
        (170.0, 5.0, 30_506),
        (200.0, 5.0, 11_031),
        (230.0, 5.0, 4_462),
    ] {
        let request = run_body(START, stress, ceiling(), STEPS_ASKED, RECORD_EVERY);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{stress}: {answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();

        let (state, landed, reject) = direct_loop(START, stress, ceiling(), STEPS_ASKED);
        assert_eq!(reject, Some(StepReject::CreepExtensionBeyondCard), "{stress}");
        if steps > 0 {
            assert_eq!(landed, steps, "{stress}");
        }
        assert_eq!(tape["outcome"], "refused", "{stress}");
        assert_eq!(tape["reject"], "CreepExtensionBeyondCard", "{stress}");
        assert_eq!(
            tape["reject_detail"],
            format!("Physical Failure: the step would pass the limit of creep extension, 5 percent, the largest level of the protocol (CreepExtensionBeyondCard). The run is sealed to step {landed}. The state at step {} was refused and is not sealed.", landed + 1),
            "{stress}"
        );
        assert_eq!(tape["steps_landed"], landed, "{stress}");
        assert_eq!(tape["steps_requested"], STEPS_ASKED);
        assert!(landed > 1 && landed < STEPS_ASKED, "{stress}: {landed}");
        let last = &tape["final_state"];
        assert_state_is(last, &state, &format!("{stress} MPa"));
        assert!(state.creep_extension_percent < limit && state.creep_extension_percent > 0.997 * limit, "{stress}");
        // Rule 6: the run passed a limit part way and keeps its proof up to the last landed step.
        assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 96));
        assert!(!tape["frames"].as_array().unwrap().is_empty());
        // The law's rate has passed its minimum by 5 percent; below 170 MPa the
        // frame past 0.2 percent is where the law missed the measured curves.
        assert_eq!(last["creep_rate_rising"], true, "{stress}");
        assert_eq!(last["outside_met_range"], stress < 170.0, "{stress}");
        // The clock is the time since the full load: the run started at the law's
        // own time to 0.1 percent and ends within one step of its time to the limit.
        let started = alloy().law_time_since_load_s(stress, START);
        assert_eq!(state.t_s.to_bits(), {
            let mut t = started;
            for _ in 0..landed {
                t += ceiling();
            }
            t.to_bits()
        });
        let to_limit = alloy().law_time_since_load_s(stress, limit);
        assert!(state.t_s <= to_limit * (1.0 + 1.0e-6) && state.t_s > to_limit - 1.001 * ceiling(), "{stress}");

        // The tape verifies, and the receipt of the response matches.
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{stress}: {report}");
        let (status, receipt) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{receipt}");
        let receipt: Value = serde_json::from_str(&receipt).unwrap();
        assert_eq!(receipt["matches"], true, "{stress}: {receipt}");
        assert_eq!(receipt["receipt_sha384"], tape["receipt_sha384"]);

        // The same request seals the same.
        let (_, again) = post(&base, "/run_trajectory", &request).await;
        let again: Value = serde_json::from_str(&again).unwrap();
        assert_eq!(again["run_seal"], tape["run_seal"]);
    }
    // The latest hour the card reaches is at 170 MPa and 5 percent: 434.84 h.
    let latest_h = alloy().law_time_since_load_s(170.0, 5.0) / 3600.0;
    assert!((latest_h - 434.843).abs() < 1.0e-3, "{latest_h}");
}

/// The saved requests of the first held-back run started at zero extension.
/// Through engine 0.9 each was refused as CreepExtensionBelowCard; sent again,
/// each lands, its frames under 0.1 percent flagged as where the law missed
/// the measured curves, and a start below zero is refused by name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_start_below_a_tenth_of_a_percent_lands_flagged_and_one_below_zero_is_refused_by_name() {
    let base = spawn_app().await;
    for stress in [140.0, 170.0, 200.0, 230.0] {
        for start in [0.0, 1.0e-9, 0.05, 0.0999, below(START)] {
            let (status, text) = post(&base, "/verify_physics", &step_body(start, stress, 5.0)).await;
            assert_eq!(status, 200, "{stress} {start}: {text}");
            let answer: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(answer["state"]["outside_met_range"], true, "{stress} {start}");
            let (status, text) = post(&base, "/run_trajectory", &run_body(start, stress, 5.0, 100, 10)).await;
            assert_eq!(status, 200, "{stress} {start}: {text}");
            let tape: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(tape["outcome"], "completed", "{stress} {start}");
            let (state, landed, reject) = direct_loop(start, stress, 5.0, 100);
            assert_eq!((landed, reject), (100, None));
            assert_state_is(&tape["final_state"], &state, &format!("{stress} {start}"));
            for frame in tape["frames"].as_array().unwrap() {
                let bytes: [u8; 64] = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap().try_into().unwrap();
                let frame = LastStateFrame64::from_bytes(bytes);
                assert_eq!(frame.flags & !DECLARED_FLAGS, 0);
            }
        }
        // A start at zero is at the moment the full load is applied: its clock is zero.
        let (_, text) = post(&base, "/verify_physics", &step_body(0.0, stress, 5.0)).await;
        let answer: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(published(&answer["state"], "t_s"), 5.0, "{stress}");
        // 0.1 percent itself starts with no flag.
        let (status, text) = post(&base, "/run_trajectory", &run_body(START, stress, ceiling(), 100, 10)).await;
        assert_eq!(status, 200, "{stress}: {text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{stress}");
        for frame in tape["frames"].as_array().unwrap() {
            let bytes: [u8; 64] = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap().try_into().unwrap();
            let frame = LastStateFrame64::from_bytes(bytes);
            assert!(frame.pos[0] >= START as f32);
            assert_eq!(frame.flags & creep_cmsx6::FLAG_OUTSIDE_MET_RANGE, 0);
        }
    }
    // A step of the ceiling from zero at 230 MPa leaves more than the ledger's
    // limit: it is refused by the ledger, by name, and carries no proof.
    let (status, text) = post(&base, "/verify_physics", &step_body(0.0, 230.0, ceiling())).await;
    assert_eq!(status, 422, "{text}");
    assert!((text.contains("(FirstLawBreach)") || text.contains("(SpontaneousGeneration)")) && !text.contains("sha384_seal"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(-1.0e-9, 200.0, ceiling())).await;
    assert_eq!(status, 422);
    assert_eq!(
        text,
        "Physical Failure: 'creep_extension_percent' -1e-9 % is below 0 %, the inclusive minimum of the creep_cmsx6 card (CreepExtensionBelowZero). That state is not sealed."
    );
    // A dataset row that starts at zero runs, and one below zero is not admitted.
    let runs: Vec<Value> = [0.0, -1.0e-3]
        .iter()
        .map(|start| {
            json!({
                "initial": { "creep_extension_percent": start },
                "dt_s": 5.0,
                "segments": [{ "steps": 10, "load": { "stress_mpa": 200.0 } }],
                "record_every": 10
            })
        })
        .collect();
    let plan = json!({
        "domain": "materials",
        "card": "creep_cmsx6",
        "plan": { "kind": "list", "runs": runs },
        "store": "none",
        "return": "inline", "answer": "full"
    });
    let (status, text) = post(&base, "/run_dataset", &plan).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed", "{text}");
    assert_eq!(answer["rows"][1]["outcome"], "unadmitted", "{text}");
    assert!(answer["rows"][1]["run_seal"].is_null(), "{text}");
    assert!(answer["rows"][1]["reject_detail"].as_str().unwrap().contains("(CreepExtensionBelowZero)"), "{text}");
    // The description names its two flags.
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "materials", "card": "creep_cmsx6"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    let flags = described["advisory_flags"].as_array().unwrap();
    assert_eq!(flags.len(), 2);
    assert_eq!(flags[0]["name"], "FLAG_CREEP_RATE_RISING");
    assert_eq!(flags[0]["mask"], 1);
    assert_eq!(flags[1]["name"], "FLAG_OUTSIDE_MET_RANGE");
    assert_eq!(flags[1]["mask"], 2);
    assert!(!described.to_string().contains("FLAG_BELOW_COMPARED_EXTENSION"));
    let rejects: Vec<&str> = described["rejects"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    // After the kernel's 12 names the list holds the gateway's own two, DtBelowFloor and DtAboveCeiling.
    assert_eq!(rejects.len(), 14);
    for name in ["CreepExtensionBelowZero", "CreepExtensionOutsideCard", "CreepExtensionBeyondCard", "StressNotHeld"] {
        assert!(rejects.contains(&name), "{name}");
    }
    assert!(!rejects.contains(&"CreepExtensionBelowCard"));
    assert!(!rejects.contains(&"TimeBeyondCard"));
    // The describe text names 5 percent as the card's top, and the bound states the two gaps in numbers.
    let meaning = |name: &str| described["rejects"].as_array().unwrap().iter().find(|r| r["name"] == name).unwrap()["meaning"].as_str().unwrap().to_string();
    assert_eq!(meaning("CreepExtensionBeyondCard"), "The trial step went past 5 percent creep extension, the card's top.");
    assert_eq!(meaning("CreepExtensionOutsideCard"), "The incoming creep extension is above 5 percent, the card's top.");
    let bound = described["phase_fields"][0]["bound"].as_str().unwrap();
    assert!(bound.contains("1.5 to 4 times the hours to 0.05 percent") && bound.contains("about 2 times the extension"), "{bound}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_later_segment_with_another_stress_refuses_the_whole_run_and_a_changed_tape_breaks() {
    let base = spawn_app().await;
    let request = json!({
        "domain": "materials",
        "card": "creep_cmsx6",
        "initial": { "creep_extension_percent": 0.5 },
        "dt_s": ceiling(),
        "segments": [
            { "steps": 200, "load": { "stress_mpa": 170.0 } },
            { "steps": 200, "load": { "stress_mpa": 170.0 } }
        ],
        "record_every": 100
    });
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let mut tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], 400);
    // Two segments at one stress are the kernel loop of 400 steps.
    let (state, landed, reject) = direct_loop(0.5, 170.0, ceiling(), 400);
    assert_eq!((landed, reject), (400, None));
    assert_state_is(&tape["final_state"], &state, "two segments");
    // Another stress in the second segment, inside the span: refused before a step.
    for other in [210.0, 140.0, above(170.0), below(170.0)] {
        let mut changed = request.clone();
        changed["segments"][1]["load"]["stress_mpa"] = json!(other);
        let (status, text) = post(&base, "/run_trajectory", &changed).await;
        assert_eq!(status, 422, "{other}: {text}");
        assert_eq!(
            text,
            "Physical Failure: the stress of the step is not the stress the test piece was started at: the card holds one stress through a run (StressNotHeld). That state is not sealed.",
            "{other}"
        );
    }
    // A stress outside the span in the second segment refuses the whole run before its first step.
    let mut outside = request.clone();
    outside["segments"][1]["load"]["stress_mpa"] = json!(231.0);
    let (status, text) = post(&base, "/run_trajectory", &outside).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("StressOutsideCard") && !text.contains("run_seal"), "{text}");
    // A changed number in the state breaks the tape.
    tape["final_state"]["creep_extension_percent_hex"] = json!("3ff0000000000000");
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");
}

// ------------------------------------------------------------- the bounds

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let (stress_min, stress_max) = (creep_cmsx6::STRESS_MIN_MPA, creep_cmsx6::STRESS_MAX_MPA);
    let extension_max = creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT;
    let extension_max_low = creep_cmsx6::CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT;
    let full_reach = creep_cmsx6::FULL_REACH_STRESS_MIN_MPA;
    // The description publishes exactly these numbers.
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "materials", "card": "creep_cmsx6"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    let published = |list: &str, key: &str| described[list][0][key].as_f64().unwrap();
    assert_eq!(published("phase_fields", "minimum").to_bits(), 0.0_f64.to_bits());
    assert_eq!(published("phase_fields", "maximum").to_bits(), extension_max.to_bits());
    assert_eq!(published("load_fields", "minimum").to_bits(), stress_min.to_bits());
    assert_eq!(published("load_fields", "maximum").to_bits(), stress_max.to_bits());
    assert_eq!(described["phase_fields"][0]["unit"], "%");
    assert_eq!(described["load_fields"][0]["unit"], "MPa");
    assert_eq!(described["specimen"].as_str().unwrap().contains("CMSX-6"), true);

    let names = |text: &str, name: &str| {
        text.contains(&format!("({name}). That state is not sealed.")) && !text.contains("sha384_seal")
    };
    // (extension, stress, the refusal, or none when the start is inside the card)
    let cases: [(f64, f64, Option<&str>); 18] = [
        (START, 185.0, None),
        (below(START), 185.0, None),
        (START, stress_min, None),
        (below(START), stress_min, None),
        (0.0, stress_min, None),
        (-f64::MIN_POSITIVE, 185.0, Some("CreepExtensionBelowZero")),
        // The upper limit at every stress: 5 percent.
        (below(extension_max), full_reach, None),
        (extension_max, full_reach, None),
        (above(extension_max), 185.0, Some("CreepExtensionOutsideCard")),
        (below(extension_max), stress_min, None),
        (extension_max, stress_min, None),
        (above(extension_max), stress_min, Some("CreepExtensionOutsideCard")),
        // Past 0.2 percent below 170 MPa: the law missed there, and the card answers.
        (above(extension_max_low), stress_min, None),
        (above(extension_max_low), below(full_reach), None),
        (0.5, 155.0, None),
        // The stress span.
        (0.15, below(stress_min), Some("StressOutsideCard")),
        (0.3, stress_max, None),
        (0.3, above(stress_max), Some("StressOutsideCard")),
    ];
    for (extension, stress, refusal) in cases {
        let limit = extension_max;
        // One second from just under the limit at the lowest stress of its reach stays under it;
        // from the limit itself, and from the last number under it, no step stays inside.
        let at_the_limit = extension >= below(limit);
        let dt = if at_the_limit { floor() } else { ceiling() };
        let step = step_body(extension, stress, dt);
        let run = run_body(extension, stress, dt, 1, 1);
        let plan = json!({
            "domain": "materials",
            "card": "creep_cmsx6",
            "plan": { "kind": "list", "runs": [{
                "initial": { "creep_extension_percent": extension },
                "dt_s": dt,
                "segments": [{ "steps": 1, "load": { "stress_mpa": stress } }],
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
                if at_the_limit {
                    // The limit, and the last number under it, are inside the card, and no
                    // step from them stays inside: the step is refused by the kernel, by
                    // name, and the run of one step comes back refused with nothing landed.
                    assert_eq!(step_status, 422, "{extension} {stress}: {step_text}");
                    assert!(names(&step_text, "CreepExtensionBeyondCard"), "{extension} {stress}: {step_text}");
                    assert_eq!(run_status, 200, "{extension} {stress}: {run_text}");
                    let tape: Value = serde_json::from_str(&run_text).unwrap();
                    assert_eq!(tape["reject"], "CreepExtensionBeyondCard", "{run_text}");
                    assert_eq!(tape["steps_landed"], 0);
                } else {
                    assert_eq!(step_status, 200, "{extension} {stress}: {step_text}");
                    assert_eq!(run_status, 200, "{extension} {stress}: {run_text}");
                    let tape: Value = serde_json::from_str(&run_text).unwrap();
                    assert_eq!(tape["outcome"], "completed", "{extension} {stress}");
                }
                assert_eq!(plan_status, 200, "{extension} {stress}: {plan_text}");
            }
            Some(name) => {
                for (status, text) in [(step_status, &step_text), (run_status, &run_text)] {
                    assert_eq!(status, 422, "{extension} {stress}: {text}");
                    assert!(names(text, name), "{extension} {stress}: {text}");
                }
                // A dataset row outside the card is not admitted.
                assert_eq!(plan_status, 200, "{extension} {stress}: {plan_text}");
                let answer: Value = serde_json::from_str(&plan_text).unwrap();
                assert_eq!(answer["rows"][0]["outcome"], "unadmitted", "{plan_text}");
                assert!(answer["rows"][0]["run_seal"].is_null(), "{plan_text}");
                assert!(names(answer["rows"][0]["reject_detail"].as_str().unwrap(), name), "{plan_text}");
            }
        }
    }
    let (_, text) = post(&base, "/verify_physics", &step_body(above(extension_max), 155.0, ceiling())).await;
    assert_eq!(
        text,
        format!(
            "Physical Failure: 'creep_extension_percent' {} % is above 5 %, the inclusive maximum of the creep_cmsx6 card (CreepExtensionOutsideCard). That state is not sealed.",
            above(extension_max)
        )
    );
    // A clock under the floor or over the ceiling is named as well.
    let (status, text) = post(&base, "/verify_physics", &step_body(0.3, 185.0, below(floor()))).await;
    assert_eq!(status, 422);
    assert!(text.contains("the step floor of the creep_cmsx6 card"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(0.3, 185.0, above(ceiling()))).await;
    assert_eq!(status, 422);
    assert!(text.contains("the step ceiling of the creep_cmsx6 card"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(0.3, 185.0, 0.0)).await;
    assert_eq!(status, 422);
    assert!(text.contains("NonPositiveDt"), "{text}");
    for dt in [floor(), ceiling()] {
        let (status, text) = post(&base, "/verify_physics", &step_body(0.3, 185.0, dt)).await;
        assert_eq!(status, 200, "{text}");
    }
    // Missing fields are refused, never filled in.
    let (status, text) = post(
        &base,
        "/verify_physics",
        &json!({"domain": "materials", "card": "creep_cmsx6",
                "parameters": {"creep_extension_percent": 0.3, "dt_s": ceiling()}}),
    )
    .await;
    assert_eq!(status, 422);
    assert!(text.contains("Missing required parameter: 'stress_mpa' (InputMissing)."), "{text}");
    let (status, text) = post(
        &base,
        "/verify_physics",
        &json!({"domain": "materials", "card": "creep_cmsx6",
                "parameters": {"stress_mpa": 185.0, "dt_s": ceiling()}}),
    )
    .await;
    assert_eq!(status, 422);
    assert!(text.contains("Missing required parameter: 'creep_extension_percent' (InputMissing)."), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_card_constant_sent_as_a_field_is_refused() {
    let base = spawn_app().await;
    for constant in [
        "temperature_c",
        "reference_stress_mpa",
        "base_rate_percent_per_h",
        "base_stress_exponent",
        "softening_per_percent",
        "softening_stress_exponent",
        "primary_multiplier",
        "primary_decay_extension_percent",
        "stress_max_mpa",
        "creep_extension_max_percent",
        "creep_extension_min_percent",
        "full_reach_stress_min_mpa",
        "creep_extension_max_below_full_reach_percent",
        "residual_conservation_limit",
    ] {
        let mut step = step_body(0.3, 185.0, ceiling());
        step["parameters"][constant] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert_eq!(text, format!("Physical Violation: '{constant}' is not a field of this request."));

        let mut in_phase = run_body(0.3, 185.0, ceiling(), 3, 1);
        in_phase["initial"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_phase).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");

        let mut in_load = run_body(0.3, 185.0, ceiling(), 3, 1);
        in_load["segments"][0]["load"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");
    }
}

// ---------------------------------------------------------------- dataset

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_grid_of_four_stresses_gives_rows_whose_run_seals_are_the_runs_alone() {
    let base = spawn_app().await;
    let stresses = [185.0, 200.0, 215.0, 230.0];
    for store in ["frames", "none"] {
        let request = json!({
            "domain": "materials",
            "card": "creep_cmsx6",
            "plan": {
                "kind": "grid",
                "base": {
                    "initial": { "creep_extension_percent": START },
                    "dt_s": ceiling(),
                    "segments": [{ "steps": 40_000, "load": {} }],
                    "record_every": RECORD_EVERY
                },
                "axes": [{ "path": "segments[0].load.stress_mpa", "values": stresses }]
            },
            "store": store,
            "return": "inline", "answer": "full"
        });
        let (status, text) = post(&base, "/run_dataset", &request).await;
        assert_eq!(status, 200, "{text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        let rows = answer["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 4);
        let mut landed_before = u64::MAX;
        for (row, stress) in rows.iter().zip(stresses) {
            assert_eq!(row["outcome"], "refused", "{row}");
            assert_eq!(row["reject"], "CreepExtensionBeyondCard", "{row}");
            let alone = run_body(START, stress, ceiling(), 40_000, RECORD_EVERY);
            let (status, alone) = post(&base, "/run_trajectory", &alone).await;
            assert_eq!(status, 200, "{alone}");
            let alone: Value = serde_json::from_str(&alone).unwrap();
            assert_eq!(row["run_seal"], alone["run_seal"], "{store} {stress}");
            assert_eq!(row["state_digest"], alone["state_digest"], "{store} {stress}");
            assert_eq!(row["steps_landed"], alone["steps_landed"], "{store} {stress}");
            // A higher stress reaches 5 percent in fewer steps.
            let landed = row["steps_landed"].as_u64().unwrap();
            assert!(landed < landed_before, "{stress}");
            landed_before = landed;
        }
    }
}

// ---------------------------------------------------------- validate_series

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_the_creep_extension_of_a_run_and_carries_its_run_seal() {
    let base = spawn_app().await;
    let run = run_body(START, 200.0, ceiling(), 2_000, 100);
    let (_, alone) = post(&base, "/run_trajectory", &run).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["outcome"], "completed");
    let steps = [100_u64, 500, 1_000, 2_000];
    let probe = json!({
        "run": run, "quantity": "creep_extension_percent",
        "measurements": steps.iter().map(|step| json!({ "step": step, "value": 1.0 })).collect::<Vec<_>>()
    });
    let (status, text) = post(&base, "/validate_series", &probe).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["compared"], 4);
    let model: Vec<f64> = answer["model_values_hex"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hex| f64::from_bits(u64::from_str_radix(hex.as_str().unwrap(), 16).unwrap()))
        .collect();
    assert_eq!(model[3].to_bits(), published(&alone["final_state"], "creep_extension_percent").to_bits());
    assert!(model.windows(2).all(|pair| pair[1] > pair[0]));
    // Measurements equal to the model give an error of exactly zero.
    let exact = json!({
        "run": run, "quantity": "creep_extension_percent",
        "measurements": steps.iter().zip(&model).map(|(step, value)| json!({ "step": step, "value": value })).collect::<Vec<_>>()
    });
    let (status, text) = post(&base, "/validate_series", &exact).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rms_error"], 0.0);
    assert_eq!(answer["max_abs_error"], 0.0);
    for quantity in ["creep_rate_percent_per_h", "work_by_load_mj_per_m3", "heat_dissipated_mj_per_m3"] {
        let body = json!({ "run": run, "quantity": quantity, "measurements": [{ "step": 2_000, "value": 1.0 }] });
        let (status, text) = post(&base, "/validate_series", &body).await;
        assert_eq!(status, 200, "{quantity}: {text}");
    }
    // A flag is not a quantity.
    for flag in ["creep_rate_rising"] {
        let body = json!({ "run": run, "quantity": flag, "measurements": [{ "step": 2_000, "value": 1.0 }] });
        let (status, _) = post(&base, "/validate_series", &body).await;
        assert_eq!(status, 422, "{flag}");
    }
}

// ------------------------------------------------------------ the listing

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_creep_card_is_listed_in_the_materials_domain_as_the_eighteenth_card() {
    let base = spawn_app().await;
    let client = Client::new();
    let listed: Value = client.get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let all_pairs = listed["cards"].as_array().unwrap();
    assert_eq!(all_pairs.len(), protocol_mcp_gateway::CARDS);
    // The seventeenth since the fold of 0.15.0 (the eighteenth until 0.14.3).
    let listed = &all_pairs[16];
    assert_eq!(listed["domain"], "materials");
    assert_eq!(listed["card"], "creep_cmsx6");
}
