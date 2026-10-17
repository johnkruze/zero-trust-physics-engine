//! The SPERT III E-core card on every door.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes. A run from cold start ends in the sealed refusal at the first
//! maximum of power, and the three peak values of its last landed state are
//! the kernel loop's, to the bit. The tape of that run verifies and its
//! receipt matches. Every published bound is refused by name at the last
//! representable number on each side. The hosted door hides the card from a
//! key that does not list it, byte for byte as a card that does not exist.
//!
//! The reactivities and coolant temperatures of the runs are those of opened
//! tests only (22, 51 and 43 here). No sealed test is run.

mod answer_text;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::reactor_spert3::{
    self, ReactorLoad, ReactorSpert3, ReactorState, StepReject,
};
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

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

fn kelvin(fahrenheit: f64) -> f64 {
    (fahrenheit - 32.0) / 1.8 + 273.15
}

fn reactor() -> ReactorSpert3 {
    ReactorSpert3::e_core_cold_startup()
}

fn ceiling() -> f64 {
    reactor().gate_dt_s().unwrap()
}

fn floor() -> f64 {
    reactor().floor_dt_s().unwrap()
}

/// The opened tests used here: number, coolant in degF, and the reactivity the
/// inhour equation gives for the test's reactor period with the card's own
/// delayed-neutron set and generation time.
const OPENED: [(u32, f64, f64); 3] = [
    (22, 74.0, 0.7686211839115998),
    (51, 82.0, 0.9982818460448638),
    (43, 78.0, 1.2102315121058187),
];

const STEPS_ASKED: u64 = 30_000;
const RECORD_EVERY: u64 = 1_000;

fn step_body(coolant_k: f64, reactivity: f64, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": "reactor_spert3",
        "parameters": {
            "coolant_temperature_k": coolant_k,
            "reactivity_inserted_dollar": reactivity,
            "dt_s": dt_s
        }
    })
}

fn run_body(coolant_k: f64, reactivity: f64, dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "reactor_spert3",
        "initial": { "coolant_temperature_k": coolant_k },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": { "reactivity_inserted_dollar": reactivity } }],
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

/// The kernel loop: steps of `dt_s` from cold start until the kernel refuses.
fn direct_loop(coolant_k: f64, reactivity: f64, dt_s: f64, limit: u64) -> (ReactorState, u64, Option<StepReject>) {
    let reactor = reactor();
    let mut state = ReactorState::try_cold_start(&reactor, coolant_k).unwrap();
    let mut frame = blank_frame([0; 16]);
    let load = ReactorLoad { reactivity_inserted_dollar: reactivity };
    let mut landed = 0;
    while landed < limit {
        match reactor_spert3::step(&reactor, &mut state, &mut frame, load, dt_s) {
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

/// Every number of the kernel's state, under the name the door publishes.
fn state_numbers(state: &ReactorState) -> Vec<(&'static str, f64)> {
    // Every field is named, so a field added to the card does not compile
    // here until it is published or named as not published. The reactivity of
    // the burst is the one that is not: it is the load of the request, which
    // every answer carries in `inputs`, it is held to it in the test of a run
    // in segments, and the answers this card gave before the field existed
    // keep their bytes because `assert_state_is` finds nothing else published.
    let ReactorState {
        t_s,
        power_mw,
        precursor_mw,
        energy_mws,
        power_rate_mw_per_s,
        reactivity_net_dollar,
        coolant_temperature_k,
        population_anchor_mw,
        reactivity_energy_integral_dollar_mws,
        population_defect_mw,
        residual_conservation_error,
        peak_power_mw,
        peak_energy_mws,
        peak_time_s,
        reactivity_held_dollar: _,
        past_first_maximum: _,
        prompt_supercritical: _,
    } = *state;
    vec![
        ("t_s", t_s),
        ("power_mw", power_mw),
        ("energy_mws", energy_mws),
        ("power_rate_mw_per_s", power_rate_mw_per_s),
        ("reactivity_net_dollar", reactivity_net_dollar),
        ("coolant_temperature_k", coolant_temperature_k),
        ("precursor_1_mw", precursor_mw[0]),
        ("precursor_2_mw", precursor_mw[1]),
        ("precursor_3_mw", precursor_mw[2]),
        ("precursor_4_mw", precursor_mw[3]),
        ("precursor_5_mw", precursor_mw[4]),
        ("precursor_6_mw", precursor_mw[5]),
        ("population_anchor_mw", population_anchor_mw),
        (
            "reactivity_energy_integral_dollar_mws",
            reactivity_energy_integral_dollar_mws,
        ),
        ("population_defect_mw", population_defect_mw),
        ("residual_conservation_error", residual_conservation_error),
        ("peak_power_mw", peak_power_mw),
        ("peak_energy_mws", peak_energy_mws),
        ("peak_time_s", peak_time_s),
    ]
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// both as the decimal and as the hex beside it, and nothing else is published.
fn assert_state_is(published_state: &Value, state: &ReactorState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in state_numbers(state) {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    assert_eq!(object["past_first_maximum"], state.past_first_maximum, "{label}");
    assert_eq!(object["prompt_supercritical"], state.prompt_supercritical, "{label}");
    keys.insert("past_first_maximum".to_string());
    keys.insert("prompt_supercritical".to_string());
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha256\":\"";
    let at = text.rfind(MARKER).expect("a receipt");
    (format!("{}}}", &text[..at]), text[at + MARKER.len()..text.len() - 2].to_string())
}

// -------------------------------------------------------------- the step

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_reactor_matches_the_compiled_step_with_the_same_proof_bytes() {
    let base = spawn_app().await;
    let cases = [
        step_body(kelvin(82.0), 0.9982818460448638, ceiling()),
        step_body(reactor_spert3::COOLANT_TEMPERATURE_MAX_K, reactor_spert3::REACTIVITY_MAX_DOLLAR, ceiling()),
        step_body(reactor_spert3::COOLANT_TEMPERATURE_MIN_K, reactor_spert3::REACTIVITY_MIN_DOLLAR, floor()),
        step_body(kelvin(78.0), 1.2102315121058187, 3.0e-4),
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
        let reactor = reactor();
        let mut state = ReactorState::try_cold_start(
            &reactor,
            parameters["coolant_temperature_k"].as_f64().unwrap(),
        )
        .unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        reactor_spert3::step(
            &reactor,
            &mut state,
            &mut frame,
            ReactorLoad {
                reactivity_inserted_dollar: parameters["reactivity_inserted_dollar"].as_f64().unwrap(),
            },
            parameters["dt_s"].as_f64().unwrap(),
        )
        .unwrap();
        assert_eq!(frame.flags & !reactor_spert3::FLAG_PROMPT_SUPERCRITICAL, 0);
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
        seals.push(response.sha256_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 4, "four different steps, four different seals");
}

// ---------------------------------------------------------------- the run

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_from_cold_start_ends_in_the_sealed_refusal_with_the_peaks_of_the_kernel_loop() {
    let base = spawn_app().await;
    for (test, fahrenheit, reactivity) in OPENED {
        let coolant_k = kelvin(fahrenheit);
        let request = run_body(coolant_k, reactivity, ceiling(), STEPS_ASKED, RECORD_EVERY);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "test {test}: {answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();

        let (state, landed, reject) = direct_loop(coolant_k, reactivity, ceiling(), STEPS_ASKED);
        assert_eq!(reject, Some(StepReject::PastFirstMaximum), "test {test}");
        assert!(state.past_first_maximum);
        // The first maximum of power is the end the card declares.
        assert_eq!(tape["outcome"], "ended", "test {test}");
        assert_eq!(tape["reject"], "PastFirstMaximum", "test {test}");
        assert_eq!(
            tape["reject_detail"],
            format!("Ended: the run reached the end the reactor_spert3 card declares (PastFirstMaximum): the first maximum of power, where the card's burst ends. The step past it is not taken, and the run is sealed to step {landed}."),
            "test {test}"
        );
        assert_eq!(tape["steps_landed"], landed, "test {test}");
        assert_eq!(tape["steps_requested"], STEPS_ASKED);
        assert!(landed > 1 && landed < STEPS_ASKED, "test {test}: {landed}");

        // The last landed state is the kernel's, and its three peak values are
        // the kernel loop's to the bit.
        let last = &tape["final_state"];
        assert_state_is(last, &state, &format!("test {test}"));
        for name in ["peak_power_mw", "peak_energy_mws", "peak_time_s"] {
            let expected = match name {
                "peak_power_mw" => state.peak_power_mw,
                "peak_energy_mws" => state.peak_energy_mws,
                _ => state.peak_time_s,
            };
            assert_eq!(published(last, name).to_bits(), expected.to_bits(), "test {test} {name}");
        }
        assert_eq!(last["past_first_maximum"], true);

        // The tape verifies, and the receipt of the response matches.
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "test {test}: {report}");
        let (status, receipt) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{receipt}");
        let receipt: Value = serde_json::from_str(&receipt).unwrap();
        assert_eq!(receipt["matches"], true, "test {test}: {receipt}");
        assert_eq!(receipt["receipt_sha256"], tape["receipt_sha256"]);

        // The same request seals the same.
        let (_, again) = post(&base, "/run_trajectory", &request).await;
        let again: Value = serde_json::from_str(&again).unwrap();
        assert_eq!(again["run_seal"], tape["run_seal"]);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_in_segments_under_one_reactivity_is_the_run_alone_and_a_changed_tape_breaks() {
    let base = spawn_app().await;
    let segments = |list: Value| {
        json!({
            "domain": "energy",
            "card": "reactor_spert3",
            "initial": { "coolant_temperature_k": kelvin(82.0) },
            "dt_s": ceiling(),
            "segments": list,
            "record_every": 100
        })
    };
    let request = segments(json!([
        { "steps": 200, "load": { "reactivity_inserted_dollar": 0.95 } },
        { "steps": 200, "load": { "reactivity_inserted_dollar": 0.95 } }
    ]));
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let mut tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], 400);
    // The kernel loop under the one reactivity.
    let (state, landed, reject) = direct_loop(kelvin(82.0), 0.95, ceiling(), 400);
    assert_eq!((landed, reject), (400, None));
    assert_state_is(&tape["final_state"], &state, "two segments");
    // The state holds the reactivity of its last step, and it is the request's.
    assert_eq!(state.reactivity_held_dollar.to_bits(), 0.95_f64.to_bits());
    assert_eq!(
        tape["inputs"]["segments"][1]["load"]["reactivity_inserted_dollar"].as_f64().unwrap().to_bits(),
        state.reactivity_held_dollar.to_bits()
    );
    // The same 400 steps asked as one segment: the same final state, the same
    // state digest and the same 48 written bytes in every frame. The request
    // differs, so the proof bytes and the seals do.
    let (status, alone) = post(
        &base,
        "/run_trajectory",
        &segments(json!([{ "steps": 400, "load": { "reactivity_inserted_dollar": 0.95 } }])),
    )
    .await;
    assert_eq!(status, 200, "{alone}");
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["final_state"], tape["final_state"]);
    assert_eq!(alone["state_digest"], tape["state_digest"]);
    let (in_segments, in_one) = (tape["frames"].as_array().unwrap(), alone["frames"].as_array().unwrap());
    assert_eq!(in_segments.len(), 4);
    assert_eq!(in_one.len(), 4);
    for (a, b) in in_segments.iter().zip(in_one) {
        assert_eq!(a["step"], b["step"]);
        let (a, b) = (a["frame_hex"].as_str().unwrap(), b["frame_hex"].as_str().unwrap());
        assert_eq!(a.len(), 128);
        assert_eq!(a[..96], b[..96]);
    }
    assert_ne!(alone["run_seal"], tape["run_seal"]);
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
    // A changed number in the state breaks the tape, named as the state.
    tape["final_state"]["peak_power_mw_hex"] = json!("3ff0000000000000");
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");
}

/// The kernel loop with a reactivity per segment: `(steps, reactivity)` in turn, from cold
/// start, until the kernel refuses or the segments are done.
fn direct_segments(coolant_k: f64, segments: &[(u64, f64)], dt_s: f64) -> (ReactorState, u64, Option<StepReject>) {
    let reactor = reactor();
    let mut state = ReactorState::try_cold_start(&reactor, coolant_k).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut landed = 0;
    for (steps, rho) in segments {
        for _ in 0..*steps {
            match reactor_spert3::step(&reactor, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: *rho }, dt_s) {
                Ok(()) => landed += 1,
                Err(reject) => return (state, landed, Some(reject)),
            }
        }
    }
    (state, landed, None)
}

/// The two requests the door at 0.4.0 answered with a sealed run, and from 0.8.0 to 0.10.1
/// refused as ReactivityChangedDuringBurst: 100 steps at the ceiling at 1.1 dollars and then one
/// step at 1.2, and the same 100 steps and then 0.9. Each step is now held at its own
/// reactivity: the run is answered with a proof, and every number is the kernel loop's that
/// changes its load at the same step. A later segment at zero or below turns the power down and
/// the run ends there at the first maximum; one at 1.3 dollars is past the state ceiling at the
/// static step and the run is refused there by name, sealed to the step before.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reactivity_changed_in_a_later_segment_is_computed_at_each_segments_reactivity() {
    let base = spawn_app().await;
    let coolant_k = kelvin(70.0);
    let dt_s = ceiling();
    let at = |rho: f64| json!({ "reactivity_inserted_dollar": rho });
    let run = |segments: Value| {
        json!({
            "domain": "energy",
            "card": "reactor_spert3",
            "initial": { "coolant_temperature_k": coolant_k },
            "dt_s": dt_s,
            "segments": segments,
            "record_every": 100000
        })
    };
    for (segments, outcome, reject) in [
        (vec![(100, 1.1), (1, 1.2)], "completed", None),
        (vec![(100, 1.1), (30000, 1.2)], "ended", Some("PastFirstMaximum")),
        (vec![(100, 1.1), (30000, 0.9)], "ended", Some("PastFirstMaximum")),
        (vec![(100, 1.1), (30000, 0.0)], "refused", Some("DtAboveStateCeiling")),
        (vec![(50, 1.2), (1, 1.19), (30000, 1.2)], "ended", Some("PastFirstMaximum")),
        (vec![(50, reactor_spert3::REACTIVITY_MIN_DOLLAR), (30000, reactor_spert3::REACTIVITY_MAX_DOLLAR)], "ended", Some("PastFirstMaximum")),
        (vec![(100, 1.1), (10, 1.3)], "refused", Some("DtAboveStateCeiling")),
    ] {
        let request = run(Value::Array(segments.iter().map(|(steps, rho)| json!({ "steps": steps, "load": at(*rho) })).collect()));
        let (status, text) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{segments:?}: {text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], outcome, "{segments:?}: {text}");
        let (state, landed, kernel_reject) = direct_segments(coolant_k, &segments, dt_s);
        assert_eq!(tape["steps_landed"], landed, "{segments:?}");
        assert_eq!(kernel_reject.map(|r| format!("{r:?}")).as_deref(), reject, "{segments:?}");
        if let Some(name) = reject {
            assert_eq!(tape["reject"], name, "{segments:?}");
        }
        assert_state_is(&tape["final_state"], &state, &format!("{segments:?}"));
        assert!(state.residual_conservation_error < reactor_spert3::RESIDUAL_CONSERVATION_LIMIT);
        let (_, report) = post(&base, "/verify_tape", &tape).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{segments:?}: {report}");
        // The withdrawal ends at the step it is made.
        if segments[1].1 == 0.9 && segments.len() == 2 {
            assert_eq!(landed, 101, "{segments:?}");
        }
    }
    // The state ceiling is named in the run's sentence, with the step it was refused at.
    let past = run(json!([{ "steps": 100, "load": at(1.1) }, { "steps": 10, "load": at(1.3) }]));
    let (_, text) = post(&base, "/run_trajectory", &past).await;
    let tape: Value = serde_json::from_str(&text).unwrap();
    let detail = tape["reject_detail"].as_str().unwrap();
    assert!(detail.contains("(DtAboveStateCeiling)"), "{detail}");
    assert!(detail.ends_with("The run is sealed to step 100. The state at step 101 was refused and is not sealed."), "{detail}");
    // A dataset row and a grid row that change a later segment's reactivity are answered, each
    // with the seal of the run alone.
    let row = |from: &Value| json!({ "initial": from["initial"], "dt_s": from["dt_s"], "segments": from["segments"], "record_every": from["record_every"] });
    let jump = run(json!([{ "steps": 100, "load": at(1.1) }, { "steps": 1, "load": at(1.2) }]));
    let (_, alone) = post(&base, "/run_trajectory", &jump).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    let plan = json!({
        "domain": "energy",
        "card": "reactor_spert3",
        "plan": { "kind": "list", "runs": [row(&jump)] },
        "store": "none",
        "return": "inline", "answer": "full"
    });
    let (status, plan_text) = post(&base, "/run_dataset", &plan).await;
    assert_eq!(status, 200, "{plan_text}");
    let answer: Value = serde_json::from_str(&plan_text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed", "{plan_text}");
    assert_eq!(answer["rows"][0]["run_seal"], alone["run_seal"], "{plan_text}");
    let grid = json!({
        "domain": "energy",
        "card": "reactor_spert3",
        "plan": {
            "kind": "grid",
            "base": {
                "initial": { "coolant_temperature_k": coolant_k },
                "dt_s": dt_s,
                "segments": [{ "steps": 100, "load": at(1.1) }, { "steps": 1, "load": {} }],
                "record_every": 100000
            },
            "axes": [{ "path": "segments[1].load.reactivity_inserted_dollar", "values": [1.1, 1.2, 0.9] }]
        },
        "store": "none",
        "return": "inline", "answer": "full"
    });
    let (status, grid_text) = post(&base, "/run_dataset", &grid).await;
    assert_eq!(status, 200, "{grid_text}");
    let answer: Value = serde_json::from_str(&grid_text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed", "{grid_text}");
    assert_eq!(answer["rows"][1]["outcome"], "completed", "{grid_text}");
    assert_eq!(answer["rows"][1]["run_seal"], alone["run_seal"], "{grid_text}");
    // The withdrawal to 0.9 lands its one step with the power turned down: the run completes
    // with the first maximum passed, and a step after it would end the run.
    assert_eq!(answer["rows"][2]["outcome"], "completed", "{grid_text}");
    assert!(answer["rows"][2]["run_seal"].is_string(), "{grid_text}");
    // A comparison of a series runs the same run.
    let series = json!({ "run": jump, "quantity": "power_mw", "measurements": [{ "step": 101, "value": 1.0 }] });
    let (status, text) = post(&base, "/validate_series", &series).await;
    assert_eq!(status, 200, "{text}");
    // Under one reactivity the run is the kernel loop's, to the bit, as before.
    let whole = run(json!([{ "steps": 100, "load": at(1.1) }, { "steps": 30000, "load": at(1.1) }]));
    let (status, answer) = post(&base, "/run_trajectory", &whole).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["reject"], "PastFirstMaximum");
    let (direct, landed, reject) = direct_loop(coolant_k, 1.1, dt_s, 30100);
    assert_eq!(reject, Some(StepReject::PastFirstMaximum));
    assert_eq!(tape["steps_landed"], landed);
    assert_state_is(&tape["final_state"], &direct, "the whole burst");
}

/// Past every old wall of the card the door answers: a reactivity below and far above the
/// compared span, a coolant of liquid water far from the tested one, a burst past 20 s, past
/// 10 MW s and past 1000 MW. Each run ends at the first maximum, with the kernel loop's numbers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn past_every_old_wall_the_door_answers_with_the_first_maximum() {
    let base = spawn_app().await;
    let quarter = 0.999 * 0.2 * 0.00215;
    for (coolant_k, rho, dt_s, steps) in [
        (kelvin(70.0), 0.3, quarter, 400_000_u64),
        (kelvin(70.0), 0.768, ceiling(), 30_000),
        (kelvin(70.0), 1.2103, ceiling(), 30_000),
        (kelvin(70.0), 3.0, 7.5e-5, 30_000),
        (kelvin(70.0), 10.0, 1.6e-5, 30_000),
        (reactor_spert3::WATER_FREEZING_K, 1.0, ceiling(), 30_000),
        (kelvin(500.0), 1.0, ceiling(), 30_000),
        (reactor_spert3::WATER_CRITICAL_K, 1.0, ceiling(), 30_000),
    ] {
        let request = run_body(coolant_k, rho, dt_s, steps, steps);
        let (status, text) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{coolant_k} {rho}: {text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "ended", "{coolant_k} {rho}: {text}");
        assert_eq!(tape["reject"], "PastFirstMaximum");
        let (state, landed, reject) = direct_loop(coolant_k, rho, dt_s, steps);
        assert_eq!(reject, Some(StepReject::PastFirstMaximum));
        assert_eq!(tape["steps_landed"], landed);
        assert_state_is(&tape["final_state"], &state, &format!("{coolant_k} {rho}"));
        if rho == 0.3 {
            assert!(state.peak_time_s > 100.0, "{}", state.peak_time_s);
        }
        if rho == 3.0 {
            assert!(state.peak_power_mw > 20_000.0 && state.peak_energy_mws > 50.0, "{} {}", state.peak_power_mw, state.peak_energy_mws);
        }
        if rho == 10.0 {
            assert!(state.peak_power_mw > 500_000.0, "{}", state.peak_power_mw);
        }
    }
    // At the static ceiling a reactivity past 1.2104 dollars is past the state ceiling at the
    // first step: refused by name in the run, nothing landed.
    let (status, text) = post(&base, "/run_trajectory", &run_body(kelvin(70.0), 1.211, ceiling(), 100, 100)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "refused", "{text}");
    assert_eq!(tape["reject"], "DtAboveStateCeiling", "{text}");
    assert_eq!(tape["steps_landed"], 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_slowest_reactivity_fits_in_the_step_limit_and_a_run_of_floor_steps_only_completes() {
    let base = spawn_app().await;
    // The slowest the card allows, at the ceiling.
    let (state, landed, reject) = direct_loop(
        reactor_spert3::COOLANT_TEMPERATURE_MIN_K,
        reactor_spert3::REACTIVITY_MIN_DOLLAR,
        ceiling(),
        1_000_000,
    );
    assert_eq!(reject, Some(StepReject::PastFirstMaximum));
    assert!(landed < 1_000_000 / 20, "{landed} steps");
    assert!(state.t_s < 12.3);
    println!("slowest compared reactivity at the ceiling: {landed} steps, peak at {} s", state.peak_time_s);
    // A run that asks for the most the door allows, at the floor, lands every
    // step and ends before the maximum: completed, not refused.
    let request = run_body(
        reactor_spert3::COOLANT_TEMPERATURE_MIN_K,
        reactor_spert3::REACTIVITY_MIN_DOLLAR,
        floor(),
        1_000_000,
        100_000,
    );
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], 1_000_000);
    assert_eq!(tape["final_state"]["past_first_maximum"], false);
    let one_millisecond = published(&tape["final_state"], "t_s");
    assert!(one_millisecond > 9.99e-4 && one_millisecond < 1.01e-3, "{one_millisecond}");
    // One step past the limit is refused before anything runs.
    let (status, text) = post(
        &base,
        "/run_trajectory",
        &run_body(reactor_spert3::COOLANT_TEMPERATURE_MIN_K, 1.0, floor(), 1_000_001, 100_000),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("more than 1000000 steps"), "{text}");
}

/// Over the whole compared span, at the ceiling and at half of it, every run
/// ends at the first maximum, before 12.3 s, 7.3 MW s and 296 MW. The other
/// refusals of the kernel, the state ceiling among them, are not reached.
#[test]
fn over_the_compared_span_every_run_ends_in_the_first_maximum_and_no_other_refusal_is_reached() {
    let mut worst_steps = 0;
    for coolant_k in [
        reactor_spert3::COOLANT_TEMPERATURE_MIN_K,
        kelvin(74.0),
        reactor_spert3::COOLANT_TEMPERATURE_MAX_K,
    ] {
        for index in 0..=40 {
            let rho = reactor_spert3::REACTIVITY_MIN_DOLLAR
                + (reactor_spert3::REACTIVITY_MAX_DOLLAR - reactor_spert3::REACTIVITY_MIN_DOLLAR)
                    * (index as f64 / 40.0);
            let rho = rho.clamp(reactor_spert3::REACTIVITY_MIN_DOLLAR, reactor_spert3::REACTIVITY_MAX_DOLLAR);
            for dt in [ceiling(), 0.5 * ceiling()] {
                let (state, landed, reject) = direct_loop(coolant_k, rho, dt, 1_000_000);
                assert_eq!(reject, Some(StepReject::PastFirstMaximum), "{coolant_k} {rho} {dt}");
                assert!(state.t_s < 12.3);
                assert!(state.energy_mws < 7.3);
                assert!(state.peak_power_mw < 296.0);
                assert!(state.residual_conservation_error < reactor_spert3::RESIDUAL_CONSERVATION_LIMIT);
                worst_steps = worst_steps.max(landed);
            }
        }
    }
    assert!(worst_steps < 1_000_000);
    println!("largest step count over the span, at half the ceiling: {worst_steps}");
}

/// Two requests at the old outward-rounded ends, 0.768 and 1.211 dollars, were answered by
/// 0.6.0 with a proof, and refused from 0.7.0 to 0.10.1 as ReactivityOutsideCard. The card now
/// computes there: 0.768 and 0.7686 run to the first maximum at the ceiling, and 1.211 and
/// 1.2103 do at a step under the state ceiling; tests 22 and 43 themselves run as before.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_slivers_outside_tests_22_and_43_are_answered() {
    let base = spawn_app().await;
    assert_eq!(reactor_spert3::REACTIVITY_MIN_DOLLAR.to_bits(), 0.7686211839115998_f64.to_bits());
    assert_eq!(reactor_spert3::REACTIVITY_MAX_DOLLAR.to_bits(), 1.2102315121058187_f64.to_bits());
    for rho in [0.768, 1.211, 0.7686, 1.2103] {
        // At the old ceiling, which is under today's, and under the state ceiling.
        let dt = if rho > 1.0 { 0.999 * 0.07 * 0.00215 / (rho - 1.0) } else { ceiling() };
        let dt = dt.min(0.0007144471176212568);
        let (status, text) = post(&base, "/run_trajectory", &run_body(296.4833333333333, rho, dt, 20_000, 20_000)).await;
        assert_eq!(status, 200, "{rho}: {text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["reject"], "PastFirstMaximum", "{rho}: {text}");
        let (status, text) = post(&base, "/verify_physics", &step_body(296.4833333333333, rho, dt)).await;
        assert_eq!(status, 200, "{rho}: {text}");
    }
    for (_, fahrenheit, rho) in [OPENED[0], OPENED[2]] {
        let (status, text) = post(&base, "/run_trajectory", &run_body(kelvin(fahrenheit), rho, ceiling(), 20_000, 20_000)).await;
        assert_eq!(status, 200, "{rho}: {text}");
        let tape: Value = serde_json::from_str(text.split('\n').next().unwrap()).unwrap();
        assert_eq!(tape["reject"], "PastFirstMaximum", "{rho}");
    }
}

/// The description says what was compared: the held-back span, the opened tests outside it,
/// that the rising side was compared only with digitised plots, and where the cause is.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_description_says_what_was_compared_and_where_the_card_runs_early() {
    let base = spawn_app().await;
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "energy", "card": "reactor_spert3"})).await;
    for sentence in [
        "The ten held-back tests span 0.890 to 1.128 dollars, 4.3 to 110 MW at the peak and 56 to 82 degrees Fahrenheit.",
        "test 22 below it, tests 42 and 43 above it.",
        "Only the peak power, the energy released to the peak and the time of the peak were compared with the held-back tests.",
        "The states the card seals before the peak were compared only with digitised plots of 19 opened tests; on 18 of them the card reaches a tenth of its peak about 26 percent early.",
        "held-back tests on 2026-10-08",
        "evidence/research/reactor_spert3/analysis/10_peak_timing_cause.md",
        "energy/reactor_spert3_fuel carries the ramp.",
        "1010 ms (test 22) and 10 ms (test 43), IDO-17281 Table III, not rounded",
        "At the ceiling the slowest compared reactivity takes 17,098 steps to the first maximum",
        "Compared on reactivity 0.769 to 1.210 dollars ([0.7686211839115998, 1.2102315121058187]",
        "on the ten held-back tests, 0.890 to 1.128 dollars, it met five of eight lines; peak power and energy to peak were within the report's standard deviations and the peaks came about 21 percent early. Outside that span the card computes the same equations and nothing was compared",
        "The latest first maximum the card reaches inside its compared span is at 12.2 s; the longest in Table III is 13.7 s.",
        "The largest energy to peak in Table III is 6.9 MW s, and the card reaches 7.2 MW s at the bottom of its compared span.",
        "The largest peak power in Table III is 280 MW, and the card reaches about 296 MW at the top of its compared span.",
        "Every tested burst was one step in reactivity, held to the first maximum; a burst whose reactivity changes was not compared.",
        "Tested at a primary coolant of 56 to 92 degF",
        "the report's hot-startup tests at 260 and 500 degF are in reactor_spert3_fuel",
        "Past the compared span the step ceiling follows the state",
    ] {
        assert!(described.contains(sentence), "missing: {sentence}");
    }
    assert!(!described.contains("17,187"));
    assert!(!described.contains("0.768 dollar or above 1.211"));
    for gone in ["are named for the kernel and were not reached", "TimeBeyondCard", "EnergyBeyondCard", "PowerBeyondCard", "ReactivityOutsideCard", "CoolantTemperatureOutsideCard", "ReactivityChangedDuringBurst"] {
        assert!(!described.contains(gone), "still said: {gone}");
    }
}

// ------------------------------------------------------------- the bounds

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let (cold_min, cold_max) = (reactor_spert3::WATER_FREEZING_K, reactor_spert3::WATER_CRITICAL_K);
    // The description publishes exactly these numbers, and no bound on the reactivity.
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "energy", "card": "reactor_spert3"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    let published = |list: &str, key: &str| described[list][0][key].as_f64().unwrap();
    assert_eq!(published("phase_fields", "minimum").to_bits(), cold_min.to_bits());
    assert_eq!(published("phase_fields", "maximum").to_bits(), cold_max.to_bits());
    assert!(described["load_fields"][0]["minimum"].is_null());
    assert!(described["load_fields"][0]["maximum"].is_null());
    assert_eq!(described["phase_fields"][0]["unit"], "K");
    assert_eq!(described["load_fields"][0]["unit"], "dollar");

    let names = |text: &str, name: &str| {
        text.contains(&format!("({name}). That state is not sealed.")) && !text.contains("sha256_seal")
    };
    let cases: [(f64, f64, Option<&str>); 8] = [
        (cold_min, 1.0, None),
        (below(cold_min), 1.0, Some("CoolantTemperatureNotLiquidWater")),
        (cold_max, 1.0, None),
        (above(cold_max), 1.0, Some("CoolantTemperatureNotLiquidWater")),
        (kelvin(74.0), f64::MIN_POSITIVE, None),
        (kelvin(74.0), 0.0, Some("ReactivityNotPositive")),
        (kelvin(74.0), -0.0, Some("ReactivityNotPositive")),
        (kelvin(74.0), -1.0, Some("ReactivityNotPositive")),
    ];
    for (coolant_k, rho, refused) in cases {
        let dt = 0.999 * 0.2 * 0.00215;
        let step = step_body(coolant_k, rho, dt);
        let run = run_body(coolant_k, rho, dt, 5, 1);
        let plan = json!({
            "domain": "energy",
            "card": "reactor_spert3",
            "plan": { "kind": "list", "runs": [{
                "initial": { "coolant_temperature_k": coolant_k },
                "dt_s": dt,
                "segments": [{ "steps": 5, "load": { "reactivity_inserted_dollar": rho } }],
                "record_every": 1
            }] },
            "store": "none",
            "return": "inline", "answer": "full"
        });
        let (step_status, step_text) = post(&base, "/verify_physics", &step).await;
        let (run_status, run_text) = post(&base, "/run_trajectory", &run).await;
        let (plan_status, plan_text) = post(&base, "/run_dataset", &plan).await;
        match refused {
            None => {
                assert_eq!(step_status, 200, "{coolant_k} {rho}: {step_text}");
                assert_eq!(run_status, 200, "{coolant_k} {rho}: {run_text}");
                assert_eq!(plan_status, 200, "{coolant_k} {rho}: {plan_text}");
            }
            Some(name) => {
                for (status, text) in [(step_status, &step_text), (run_status, &run_text)] {
                    assert_eq!(status, 422, "{coolant_k} {rho}: {text}");
                    assert!(names(text, name), "{coolant_k} {rho}: {text}");
                }
                // A dataset row outside the published bounds is not admitted: it
                // carries the same sentence and no run seal, and nothing runs.
                assert_eq!(plan_status, 200, "{coolant_k} {rho}: {plan_text}");
                let answer: Value = serde_json::from_str(&plan_text).unwrap();
                assert_eq!(answer["rows"][0]["outcome"], "unadmitted", "{plan_text}");
                assert!(answer["rows"][0]["run_seal"].is_null(), "{plan_text}");
                assert!(names(answer["rows"][0]["reject_detail"].as_str().unwrap(), name), "{plan_text}");
            }
        }
    }
    // A clock under the floor or over the ceiling is named as well.
    let (status, text) = post(&base, "/verify_physics", &step_body(kelvin(74.0), 1.0, below(floor()))).await;
    assert_eq!(status, 422);
    assert!(text.contains("the step floor of the reactor_spert3 card"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(kelvin(74.0), 1.0, above(ceiling()))).await;
    assert_eq!(status, 422);
    assert!(text.contains("the step ceiling of the reactor_spert3 card"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(kelvin(74.0), 1.0, 0.0)).await;
    assert_eq!(status, 422);
    assert!(text.contains("NonPositiveDt"), "{text}");
    // A step at the floor and at the ceiling lands.
    for dt in [floor(), ceiling()] {
        let (status, text) = post(&base, "/verify_physics", &step_body(kelvin(74.0), 1.0, dt)).await;
        assert_eq!(status, 200, "{text}");
    }
    // Missing and non-number fields are refused, never filled in.
    let (status, text) = post(
        &base,
        "/verify_physics",
        &json!({"domain": "energy", "card": "reactor_spert3",
                "parameters": {"coolant_temperature_k": kelvin(74.0), "dt_s": ceiling()}}),
    )
    .await;
    assert_eq!(status, 422);
    assert!(text.contains("Missing required parameter: 'reactivity_inserted_dollar' (InputMissing)."), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_card_constant_sent_as_a_field_is_refused() {
    let base = spawn_app().await;
    for constant in [
        "generation_time_reduced_s",
        "feedback_strength_dollar_per_mws",
        "initial_power_mw",
        "group_fraction",
        "group_decay_per_s",
        "reactivity_max_dollar",
        "ceiling_energy_mws",
        "water_critical_k",
        "residual_conservation_limit",
    ] {
        let mut step = step_body(kelvin(74.0), 1.0, ceiling());
        step["parameters"][constant] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert_eq!(text, format!("Physical Violation: '{constant}' is not a field of this request."));

        let mut in_phase = run_body(kelvin(74.0), 1.0, ceiling(), 3, 1);
        in_phase["initial"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_phase).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");

        let mut in_load = run_body(kelvin(74.0), 1.0, ceiling(), 3, 1);
        in_load["segments"][0]["load"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");
    }
}

// ---------------------------------------------------------------- dataset

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_grid_of_four_reactivities_gives_rows_whose_run_seals_are_the_runs_alone() {
    let base = spawn_app().await;
    let reactivities = [0.8, 0.95, 1.05, 1.2];
    let coolant_k = kelvin(70.0);
    for store in ["frames", "none"] {
        let request = json!({
            "domain": "energy",
            "card": "reactor_spert3",
            "plan": {
                "kind": "grid",
                "base": {
                    "initial": { "coolant_temperature_k": coolant_k },
                    "dt_s": ceiling(),
                    "segments": [{ "steps": STEPS_ASKED, "load": {} }],
                    "record_every": RECORD_EVERY
                },
                "axes": [{ "path": "segments[0].load.reactivity_inserted_dollar", "values": reactivities }]
            },
            "store": store,
            "return": "inline", "answer": "full"
        });
        let (status, text) = post(&base, "/run_dataset", &request).await;
        assert_eq!(status, 200, "{text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        let rows = answer["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 4);
        for (row, rho) in rows.iter().zip(reactivities) {
            assert_eq!(row["outcome"], "ended", "{row}");
            assert_eq!(row["reject"], "PastFirstMaximum", "{row}");
            let alone = run_body(coolant_k, rho, ceiling(), STEPS_ASKED, RECORD_EVERY);
            let (status, alone) = post(&base, "/run_trajectory", &alone).await;
            assert_eq!(status, 200, "{alone}");
            let alone: Value = serde_json::from_str(&alone).unwrap();
            assert_eq!(row["run_seal"], alone["run_seal"], "{store} {rho}");
            assert_eq!(row["state_digest"], alone["state_digest"], "{store} {rho}");
            assert_eq!(row["steps_landed"], alone["steps_landed"], "{store} {rho}");
        }
    }
}

// ---------------------------------------------------------- validate_series

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_the_power_of_a_run_and_carries_its_run_seal() {
    let base = spawn_app().await;
    let (_, _, rho) = OPENED[1];
    let run = run_body(kelvin(82.0), rho, ceiling(), STEPS_ASKED, RECORD_EVERY);
    let (_, alone) = post(&base, "/run_trajectory", &run).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    let landed = alone["steps_landed"].as_u64().unwrap();
    let steps = [100_u64, 400, 800, landed];
    let probe = json!({
        "run": run, "quantity": "power_mw",
        "measurements": steps.iter().map(|step| json!({ "step": step, "value": 1.0 })).collect::<Vec<_>>()
    });
    let (status, text) = post(&base, "/validate_series", &probe).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["outcome"], "ended");
    assert_eq!(answer["compared"], 4);
    let model: Vec<f64> = answer["model_values_hex"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hex| f64::from_bits(u64::from_str_radix(hex.as_str().unwrap(), 16).unwrap()))
        .collect();
    // The last model value is the power of the run's own final state, to the bit.
    assert_eq!(model[3].to_bits(), published(&alone["final_state"], "power_mw").to_bits());
    // Measurements equal to the model give an error of exactly zero.
    let exact = json!({
        "run": run, "quantity": "power_mw",
        "measurements": steps.iter().zip(&model).map(|(step, value)| json!({ "step": step, "value": value })).collect::<Vec<_>>()
    });
    let (status, text) = post(&base, "/validate_series", &exact).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rms_error"], 0.0);
    assert_eq!(answer["max_abs_error"], 0.0);
    // The peak values are quantities too.
    for quantity in ["peak_power_mw", "peak_energy_mws", "peak_time_s", "energy_mws"] {
        let body = json!({ "run": run, "quantity": quantity,
                           "measurements": [{ "step": landed, "value": 1.0 }] });
        let (status, text) = post(&base, "/validate_series", &body).await;
        assert_eq!(status, 200, "{quantity}: {text}");
    }
    // A flag is not a quantity.
    let body = json!({ "run": run, "quantity": "past_first_maximum",
                       "measurements": [{ "step": landed, "value": 1.0 }] });
    let (status, _) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 422);
}

// --------------------------------------------------------------- hosted door

const KEY_ALL: &str = "key-for-the-client-that-sees-everything";
const KEY_REACTOR: &str = "key-for-the-reactor-client";
const KEY_THERMAL: &str = "key-for-the-thermal-client";

fn client_entry(label: &str, key: &str, cards: Value) -> Value {
    json!({ "client": label, "key_sha256": sha256_hex(key.as_bytes()), "cards": cards, "solvers": [], "max_in_flight": 8, "max_permits": 1 })
}

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
}

struct Hosted {
    _child: Child,
    base: String,
    _dir: PathBuf,
}

impl Hosted {
    async fn start(keys: &Value) -> Hosted {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("reactor_hosted_{}", std::process::id()));
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

    async fn tool(&self, key: &str, tool: &str, arguments: &Value) -> String {
        let body = answer_text::tool_call(7, tool, arguments);
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

fn result_text(answer: &str) -> String {
    let value: Value = serde_json::from_str(answer).unwrap();
    let text = value["result"]["content"][0]["text"].as_str().unwrap_or_else(|| panic!("no text in {answer}")).to_string();
    answer_text::remember(&text);
    text
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_hosted_door_hides_the_reactor_from_a_key_without_it_and_shows_it_to_a_key_with_it() {
    let mut stdio = StdioDoor::start().await;
    let listed = result_text(&stdio.raw(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_cards","arguments":{}}}"#).await);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let all_pairs: Vec<Value> = listed["cards"].as_array().unwrap().iter().map(|c| json!([c["domain"], c["card"]])).collect();
    assert_eq!(all_pairs.len(), 37);
    let keys = json!([
        client_entry("all", KEY_ALL, Value::Array(all_pairs)),
        client_entry("reactor", KEY_REACTOR, json!([["energy", "reactor_spert3"]])),
        client_entry("thermal", KEY_THERMAL, json!([["energy", "thermal"]])),
    ]);
    let hosted = Hosted::start(&keys).await;

    let pair = json!({"domain": "energy", "card": "reactor_spert3"});
    let described = result_text(&stdio.raw(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"describe_card","arguments":pair}}).to_string()).await);
    let description: Value = serde_json::from_str(&described).unwrap();
    let step = description["example_compute_step"].clone();
    let run = description["example_run_trajectory"].clone();

    // The full view and the key that lists it get the bytes of the stdio door.
    for key in [KEY_ALL, KEY_REACTOR] {
        // A key that holds this card alone sees no coupling naming the other
        // SPERT III card: the couplings, last in the answer, are left out.
        let expected = if key == KEY_REACTOR {
            format!("{}}}", &described[..described.find(",\"couplings\":").expect("the couplings")])
        } else {
            described.clone()
        };
        assert_eq!(result_text(&hosted.tool(key, "describe_card", &pair).await), expected);
        for (tool, arguments) in [("verify_step", &step), ("run_trajectory", &run)] {
            let on_stdio = stdio.raw(&answer_text::tool_call(7, tool, arguments).to_string()).await;
            assert_eq!(hosted.tool(key, tool, arguments).await, on_stdio, "{tool}");
        }
    }
    // A run that changes its reactivity in a later segment is answered with the
    // same bytes on the tool and on the hosted door.
    let mut changed = run.clone();
    changed["segments"] = json!([
        { "steps": 100, "load": { "reactivity_inserted_dollar": 1.1 } },
        { "steps": 1, "load": { "reactivity_inserted_dollar": 1.2 } }
    ]);
    let on_stdio = stdio.raw(&json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"run_trajectory","arguments":changed}}).to_string()).await;
    assert!(on_stdio.contains("run_seal"), "{on_stdio}");
    for key in [KEY_ALL, KEY_REACTOR] {
        assert_eq!(hosted.tool(key, "run_trajectory", &changed).await, on_stdio);
    }
    // The description says each segment is held at its own reactivity.
    for words in [
        "A later segment may carry another reactivity: each step is computed at the reactivity its segment holds",
        "ReactivityNotPositive",
        "DtAboveStateCeiling",
    ] {
        assert!(described.contains(words), "{words}");
    }
    assert!(!described.contains("ReactivityChangedDuringBurst"));
    // After the kernel's 13 names the list holds the gateway's own three for this card, DtBelowFloor and DtAboveCeiling and ContinuationNotOnCard.
    assert_eq!(description["rejects"].as_array().unwrap().len(), 16);
    let seen = result_text(&hosted.tool(KEY_REACTOR, "list_cards", &json!({})).await);
    let seen: Value = serde_json::from_str(&seen).unwrap();
    assert_eq!(seen["cards"].as_array().unwrap().len(), 1);
    assert_eq!(seen["cards"][0]["card"], "reactor_spert3");

    // A key that does not list it gets the answers of a card that does not exist.
    let invented = "qqqqqqqqqqqqqq";
    assert_eq!(invented.len(), "reactor_spert3".len());
    let rename = |value: &Value| {
        let mut value = value.clone();
        value["card"] = json!(invented);
        value
    };
    let mut renamed_tape = serde_json::from_str::<Value>(&result_text(&hosted.tool(KEY_ALL, "run_trajectory", &run).await)).unwrap();
    let tape = renamed_tape.clone();
    renamed_tape["inputs"]["card"] = json!(invented);
    let mut session = Vec::new();
    for (tool, hidden, fake) in [
        ("describe_card", pair.clone(), rename(&pair)),
        ("verify_step", step.clone(), rename(&step)),
        ("run_trajectory", run.clone(), rename(&run)),
        ("verify_tape", tape, renamed_tape),
    ] {
        let on_hidden = hosted.tool(KEY_THERMAL, tool, &hidden).await;
        let on_fake = hosted.tool(KEY_THERMAL, tool, &fake).await;
        assert_eq!(on_hidden, on_fake, "{tool}");
        session.push(on_hidden);
    }
    let plan = json!({"domain": "energy", "card": "reactor_spert3",
        "plan": {"kind": "list", "runs": [{"initial": run["initial"], "dt_s": run["dt_s"], "segments": run["segments"], "record_every": run["record_every"]}]},
        "store": "none", "return": "inline", "answer": "full"});
    let validation = json!({"run": run, "quantity": "power_mw", "measurements": [{"step": 1, "value": 1.0}]});
    let validation_fake = json!({"run": rename(&run), "quantity": "power_mw", "measurements": [{"step": 1, "value": 1.0}]});
    let on_hidden = hosted.tool(KEY_THERMAL, "run_dataset", &plan).await;
    let on_fake = hosted.tool(KEY_THERMAL, "run_dataset", &rename(&plan)).await;
    assert_eq!(on_hidden, on_fake);
    session.push(on_hidden);
    let on_hidden = hosted.tool(KEY_THERMAL, "validate_series", &validation).await;
    let on_fake = hosted.tool(KEY_THERMAL, "validate_series", &validation_fake).await;
    assert_eq!(on_hidden, on_fake);
    session.push(on_hidden);
    assert_eq!(result_text(&hosted.tool(KEY_THERMAL, "describe_card", &pair).await), "Card not recognized (UnknownCard).");

    // Nothing the thermal key was sent names the reactor or its fields.
    let text = session.join("\n").to_lowercase();
    for word in ["reactor", "spert", "reactivity", "coolant_temperature_k", "peak_power"] {
        assert!(!text.contains(word), "the word '{word}' of the hidden card was sent");
    }
    let tools = hosted.tool(KEY_THERMAL, "list_cards", &json!({})).await;
    assert!(!tools.to_lowercase().contains("reactor"));
}
