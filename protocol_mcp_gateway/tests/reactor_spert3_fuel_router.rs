//! The SPERT III E-core card with fuel temperature on every door.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes, in each of the three inlet-temperature bands. A run from the start
//! ends in the sealed refusal at the first maximum of power, its last
//! recorded frame is the kernel's frame of the last landed step, and every
//! number of its last landed state is the kernel loop's, to the bit. The tape
//! of that run verifies and its receipt matches. Each band's own span of the
//! reactivity and of the flow is refused by name at the last representable
//! number on each side, and an inlet temperature between two bands is refused
//! by name. The hosted door hides the card from a key that does not list it,
//! byte for byte as a card that does not exist.
//!
//! No number a run produces is written here: every expected value comes from
//! a direct call into `genesis_kernel`, so the tests hold when the card's
//! fitted constants change. The loads are the ends and the middle of each
//! band's published span, not the reactivity of any test of the report.

mod answer_text;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::reactor_spert3_fuel::{
    self, ReactorLoad, ReactorSpert3Fuel, ReactorState, StepReject,
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

fn reactor() -> ReactorSpert3Fuel {
    ReactorSpert3Fuel::e_core_low_initial_power()
}

fn ceiling() -> f64 {
    reactor().gate_dt_s().unwrap()
}

fn floor() -> f64 {
    reactor().floor_dt_s().unwrap()
}

const CARD: &str = "reactor_spert3_fuel";
const STEPS_ASKED: u64 = 60_000;
const RECORD_EVERY: u64 = 1_000;

/// One start and one load.
#[derive(Clone, Copy, Debug)]
struct Case {
    inlet_k: f64,
    reactivity: f64,
    flow: f64,
}

/// The middle of the inlet band, with the reactivity and the flow at the
/// given fraction of the band's own spans.
fn case_in(band: usize, reactivity_fraction: f64, flow_fraction: f64) -> Case {
    let reactor = reactor();
    let at = |low: f64, high: f64, fraction: f64| {
        if fraction <= 0.0 {
            low
        } else if fraction >= 1.0 {
            high
        } else {
            (low + (high - low) * fraction).clamp(low, high)
        }
    };
    Case {
        inlet_k: at(reactor.band_inlet_min_k[band], reactor.band_inlet_max_k[band], 0.5),
        reactivity: at(
            reactor.band_reactivity_min_dollar[band],
            reactor.band_reactivity_max_dollar[band],
            reactivity_fraction,
        ),
        flow: at(
            reactor.band_flow_min_m_per_s[band],
            reactor.band_flow_max_m_per_s[band],
            flow_fraction,
        ),
    }
}

fn load_json(case: Case) -> Value {
    json!({ "reactivity_inserted_dollar": case.reactivity, "coolant_flow_m_per_s": case.flow })
}

fn load_of(case: Case) -> ReactorLoad {
    ReactorLoad { reactivity_inserted_dollar: case.reactivity, coolant_flow_m_per_s: case.flow }
}

fn step_body(case: Case, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": CARD,
        "parameters": {
            "inlet_temperature_k": case.inlet_k,
            "reactivity_inserted_dollar": case.reactivity,
            "coolant_flow_m_per_s": case.flow,
            "dt_s": dt_s
        }
    })
}

fn run_body(case: Case, dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": CARD,
        "initial": { "inlet_temperature_k": case.inlet_k },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": load_json(case) }],
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

/// What the kernel loop gives: the last landed state and frame, the steps
/// landed, the refusal that ended it, and the frame of every landed step that
/// `keep` asks for.
struct Direct {
    state: ReactorState,
    frame: LastStateFrame64,
    landed: u64,
    reject: Option<StepReject>,
    kept: Vec<(u64, LastStateFrame64)>,
    flags_seen: u64,
    largest_residual: f64,
}

/// The kernel loop: steps of `dt_s` from the start until the kernel refuses.
fn direct_loop(case: Case, dt_s: f64, limit: u64, keep: impl Fn(u64) -> bool) -> Direct {
    let reactor = reactor();
    let mut state = ReactorState::try_start(&reactor, case.inlet_k).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut out = Direct {
        state,
        frame,
        landed: 0,
        reject: None,
        kept: Vec::new(),
        flags_seen: 0,
        largest_residual: 0.0,
    };
    while out.landed < limit {
        match reactor_spert3_fuel::step(&reactor, &mut state, &mut frame, load_of(case), dt_s) {
            Ok(()) => {
                out.landed += 1;
                out.flags_seen |= frame.flags;
                out.largest_residual = out.largest_residual.max(state.residual_conservation_error);
                if keep(out.landed) {
                    out.kept.push((out.landed, frame));
                }
            }
            Err(reject) => {
                out.reject = Some(reject);
                break;
            }
        }
    }
    out.state = state;
    out.frame = frame;
    out
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
    // here until it is published. The two fields of the burst's load are the
    // exception: they are the load of the request, which every answer carries
    // in `inputs`, and they are held to it in the test of a run in segments.
    let ReactorState {
        t_s,
        power_mw,
        precursor_mw,
        fuel_temperature_rise_k,
        water_temperature_rise_k,
        energy_mws,
        inlet_temperature_k,
        power_rate_mw_per_s,
        reactivity_net_dollar,
        reactivity_inserted_now_dollar,
        population_anchor_mw,
        insertion_integral_dollar_mws,
        sweep_integral_k2,
        population_defect_mw,
        residual_conservation_error,
        peak_power_mw,
        peak_energy_mws,
        peak_time_s,
        burst_reactivity_dollar: _,
        coolant_flow_held_m_per_s: _,
        past_first_maximum: _,
        prompt_supercritical: _,
        rod_moving: _,
    } = *state;
    vec![
        ("t_s", t_s),
        ("power_mw", power_mw),
        ("energy_mws", energy_mws),
        ("power_rate_mw_per_s", power_rate_mw_per_s),
        ("fuel_temperature_rise_k", fuel_temperature_rise_k),
        ("water_temperature_rise_k", water_temperature_rise_k),
        ("reactivity_net_dollar", reactivity_net_dollar),
        ("reactivity_inserted_now_dollar", reactivity_inserted_now_dollar),
        ("inlet_temperature_k", inlet_temperature_k),
        ("precursor_1_mw", precursor_mw[0]),
        ("precursor_2_mw", precursor_mw[1]),
        ("precursor_3_mw", precursor_mw[2]),
        ("precursor_4_mw", precursor_mw[3]),
        ("precursor_5_mw", precursor_mw[4]),
        ("precursor_6_mw", precursor_mw[5]),
        ("population_anchor_mw", population_anchor_mw),
        ("insertion_integral_dollar_mws", insertion_integral_dollar_mws),
        ("sweep_integral_k2", sweep_integral_k2),
        ("population_defect_mw", population_defect_mw),
        ("residual_conservation_error", residual_conservation_error),
        ("peak_power_mw", peak_power_mw),
        ("peak_energy_mws", peak_energy_mws),
        ("peak_time_s", peak_time_s),
    ]
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// both as the decimal and as the hex beside it, the three booleans are the
/// kernel's, and nothing else is published.
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
    for (name, value) in [
        ("past_first_maximum", state.past_first_maximum),
        ("prompt_supercritical", state.prompt_supercritical),
        ("rod_moving", state.rod_moving),
    ] {
        assert_eq!(object[name], value, "{label} {name}");
        keys.insert(name.to_string());
    }
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
}

/// The 48 bytes of a frame that the step writes: everything before the proof.
fn written_bytes(frame: &LastStateFrame64) -> Vec<u8> {
    frame.to_bytes()[..48].to_vec()
}

/// A recorded frame of a tape: its step, its 64 bytes, and its link.
fn recorded(frame: &Value) -> (u64, Vec<u8>, Vec<u8>) {
    let bytes = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap();
    assert_eq!(bytes.len(), 64);
    let link = hex::decode(frame["link"].as_str().unwrap()).unwrap();
    (frame["step"].as_u64().unwrap(), bytes, link)
}

fn flags_of(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[40..48].try_into().unwrap())
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha256\":\"";
    let at = text.rfind(MARKER).expect("a receipt");
    (format!("{}}}", &text[..at]), text[at + MARKER.len()..text.len() - 2].to_string())
}

// -------------------------------------------------------------- the step

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_fuel_reactor_matches_the_compiled_step_with_the_same_proof_bytes() {
    let base = spawn_app().await;
    let reactor = reactor();
    let mut cases = Vec::new();
    for band in 0..3 {
        cases.push(step_body(case_in(band, 0.5, 0.5), ceiling()));
        cases.push(step_body(case_in(band, 0.0, 0.0), floor()));
        // The four corners of the band, at the ceiling and at a step between.
        cases.push(step_body(
            Case {
                inlet_k: reactor.band_inlet_max_k[band],
                reactivity: reactor.band_reactivity_max_dollar[band],
                flow: reactor.band_flow_max_m_per_s[band],
            },
            ceiling(),
        ));
        cases.push(step_body(
            Case {
                inlet_k: reactor.band_inlet_min_k[band],
                reactivity: reactor.band_reactivity_min_dollar[band],
                flow: reactor.band_flow_min_m_per_s[band],
            },
            0.7 * ceiling(),
        ));
    }
    let count = cases.len();
    let mut seals = Vec::new();
    for request in cases {
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);

        // The compiled step, with the proof the door writes.
        let parameters = &request["parameters"];
        let number = |name: &str| parameters[name].as_f64().unwrap();
        let mut state = ReactorState::try_start(&reactor, number("inlet_temperature_k")).unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        reactor_spert3_fuel::step(
            &reactor,
            &mut state,
            &mut frame,
            ReactorLoad {
                reactivity_inserted_dollar: number("reactivity_inserted_dollar"),
                coolant_flow_m_per_s: number("coolant_flow_m_per_s"),
            },
            number("dt_s"),
        )
        .unwrap();
        assert_eq!(
            frame.flags
                & !(reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL | reactor_spert3_fuel::FLAG_ROD_MOVING),
            0
        );
        // One step from the start: the rod has only begun to move.
        assert_eq!(frame.flags, reactor_spert3_fuel::FLAG_ROD_MOVING);
        assert_eq!(response.state["rod_moving"], true);
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
    assert_eq!(seals.len(), count, "each step seals apart");
}

// ---------------------------------------------------------------- the run

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_in_each_band_ends_in_the_sealed_refusal_and_keeps_its_proof_to_the_last_landed_step() {
    let base = spawn_app().await;
    for band in 0..3 {
        for (reactivity_fraction, flow_fraction) in [(0.5, 0.0), (1.0, 1.0)] {
            let case = case_in(band, reactivity_fraction, flow_fraction);
            let label = format!("band {band} {case:?}");
            let request = run_body(case, ceiling(), STEPS_ASKED, RECORD_EVERY);
            let (status, answer) = post(&base, "/run_trajectory", &request).await;
            assert_eq!(status, 200, "{label}: {answer}");
            let tape: Value = serde_json::from_str(&answer).unwrap();

            let direct = direct_loop(case, ceiling(), STEPS_ASKED, |step| step % RECORD_EVERY == 0);
            assert_eq!(direct.reject, Some(StepReject::PastFirstMaximum), "{label}");
            assert!(direct.state.past_first_maximum && !direct.state.rod_moving, "{label}");
            // The first maximum of power is the end the card declares.
            assert_eq!(tape["outcome"], "ended", "{label}");
            assert_eq!(tape["reject"], "PastFirstMaximum", "{label}");
            assert_eq!(
                tape["reject_detail"],
                format!("Ended: the run reached the end the reactor_spert3_fuel card declares (PastFirstMaximum): the first maximum of power, where the card's burst ends. The step past it is not taken, and the run is sealed to step {}.", direct.landed),
                "{label}"
            );
            assert_eq!(tape["steps_landed"], direct.landed, "{label}");
            assert_eq!(tape["steps_requested"], STEPS_ASKED);
            assert!(direct.landed > 1 && direct.landed < STEPS_ASKED, "{label}: {}", direct.landed);

            // The last landed state is the kernel's, every number of it.
            let last = &tape["final_state"];
            assert_state_is(last, &direct.state, &label);
            assert_eq!(last["past_first_maximum"], true);
            assert_eq!(
                published(&tape, "final_residual_conservation_error").to_bits(),
                direct.state.residual_conservation_error.to_bits(),
                "{label}"
            );
            assert_eq!(
                published(&tape, "max_residual_conservation_error").to_bits(),
                direct.largest_residual.to_bits(),
                "{label}"
            );

            // The recorded frames are the kernel's frames of those steps, each
            // with the first 16 bytes of the link before it as its proof, and
            // the last of them is the frame of the last landed step.
            let frames = tape["frames"].as_array().unwrap();
            let mut expected = direct.kept.clone();
            if direct.landed % RECORD_EVERY != 0 {
                expected.push((direct.landed, direct.frame));
            }
            assert_eq!(frames.len(), expected.len(), "{label}");
            let mut link_before: Option<Vec<u8>> = None;
            for (frame, (step, kernel_frame)) in frames.iter().zip(&expected) {
                let (at, bytes, link) = recorded(frame);
                assert_eq!(at, *step, "{label}");
                assert_eq!(bytes[..48], written_bytes(kernel_frame)[..], "{label} step {step}");
                if let Some(before) = &link_before {
                    assert_eq!(bytes[48..], before[..16], "{label} step {step}: the proof");
                }
                assert_eq!(link, Sha256::digest(&bytes).to_vec(), "{label} step {step}: the link");
                link_before = Some(link);
            }
            let (last_step, last_bytes, _) = recorded(frames.last().unwrap());
            assert_eq!(last_step, direct.landed, "{label}");
            assert_eq!(
                f64::from_le_bytes(last_bytes[..8].try_into().unwrap()).to_bits(),
                direct.state.t_s.to_bits(),
                "{label}"
            );
            assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 64), "{label}");

            // The tape verifies, and the receipt of the response matches.
            let (status, report) = post(&base, "/verify_tape", &tape).await;
            assert_eq!(status, 200, "{report}");
            let report: Value = serde_json::from_str(&report).unwrap();
            assert_eq!(report["holds"], true, "{label}: {report}");
            let (status, receipt) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
            assert_eq!(status, 200, "{receipt}");
            let receipt: Value = serde_json::from_str(&receipt).unwrap();
            assert_eq!(receipt["matches"], true, "{label}: {receipt}");
            assert_eq!(receipt["receipt_sha256"], tape["receipt_sha256"]);

            // A changed byte in the last landed frame breaks the tape.
            let mut forged = tape.clone();
            let at = frames.len() - 1;
            let mut bytes = last_bytes.clone();
            bytes[8] ^= 1;
            forged["frames"][at]["frame_hex"] = json!(hex::encode(bytes));
            let (_, report) = post(&base, "/verify_tape", &forged).await;
            let report: Value = serde_json::from_str(&report).unwrap();
            assert_eq!(report["holds"], false, "{label}: {report}");

            // The same request seals the same.
            let (_, again) = post(&base, "/run_trajectory", &request).await;
            let again: Value = serde_json::from_str(&again).unwrap();
            assert_eq!(again["run_seal"], tape["run_seal"]);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn both_flags_are_in_the_frames_as_the_kernel_sets_them() {
    let base = spawn_app().await;
    for band in 0..3 {
        // The largest reactivity of the band, every step recorded: the most
        // frames one run may record.
        const EVERY_STEP: u64 = 10_000;
        let case = case_in(band, 1.0, 0.5);
        let direct = direct_loop(case, ceiling(), EVERY_STEP, |_| true);
        assert_eq!(direct.reject, Some(StepReject::PastFirstMaximum), "band {band}");
        // More than one dollar goes in while the power is still small: the
        // kernel sets both flags on the way.
        assert_eq!(
            direct.flags_seen,
            reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL | reactor_spert3_fuel::FLAG_ROD_MOVING,
            "band {band}"
        );
        let request = run_body(case, ceiling(), EVERY_STEP, 1);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["steps_landed"], direct.landed);
        let frames = tape["frames"].as_array().unwrap();
        assert_eq!(frames.len() as u64, direct.landed);
        let (mut moving, mut prompt, mut both, mut neither) = (0, 0, 0, 0);
        for (frame, (step, kernel_frame)) in frames.iter().zip(&direct.kept) {
            let (at, bytes, _) = recorded(frame);
            assert_eq!(at, *step);
            let flags = flags_of(&bytes);
            assert_eq!(flags, kernel_frame.flags, "band {band} step {step}");
            let is_moving = flags & reactor_spert3_fuel::FLAG_ROD_MOVING != 0;
            let is_prompt = flags & reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL != 0;
            match (is_moving, is_prompt) {
                (true, true) => both += 1,
                (true, false) => moving += 1,
                (false, true) => prompt += 1,
                (false, false) => neither += 1,
            }
        }
        // The rod moves first; the run is prompt supercritical after it stops.
        assert!(moving > 0 && prompt > 0, "band {band}: {moving} {prompt} {both} {neither}");
        assert_eq!(flags_of(&recorded(&frames[0]).1), reactor_spert3_fuel::FLAG_ROD_MOVING);
        // The final state's booleans are the last frame's bits.
        let last = flags_of(&recorded(frames.last().unwrap()).1);
        assert_eq!(
            tape["final_state"]["rod_moving"],
            last & reactor_spert3_fuel::FLAG_ROD_MOVING != 0
        );
        assert_eq!(
            tape["final_state"]["prompt_supercritical"],
            last & reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL != 0
        );
        let (_, report) = post(&base, "/verify_tape", &tape).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "band {band}: {report}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_in_segments_under_one_load_is_the_run_alone_and_a_changed_tape_breaks() {
    let base = spawn_app().await;
    let reactor = reactor();
    // A hot band, where the flow is a load too.
    let case = case_in(2, 0.75, 1.0);
    let request = json!({
        "domain": "energy",
        "card": CARD,
        "initial": { "inlet_temperature_k": case.inlet_k },
        "dt_s": ceiling(),
        "segments": [
            { "steps": 100, "load": load_json(case) },
            { "steps": 100, "load": load_json(case) }
        ],
        "record_every": 50
    });
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let mut tape: Value = serde_json::from_str(&answer).unwrap();
    // The kernel loop under the one load.
    let direct = direct_loop(case, ceiling(), 200, |_| false);
    assert_eq!(direct.reject, None);
    // 200 steps at the ceiling end after the rod has stopped, on the rise.
    assert_eq!(tape["steps_landed"], 200);
    assert_eq!(tape["outcome"], "completed");
    assert!(!direct.state.rod_moving && !direct.state.past_first_maximum);
    assert_state_is(&tape["final_state"], &direct.state, "two segments");
    // The state holds the load of the burst, and it is the request's.
    assert_eq!(direct.state.burst_reactivity_dollar.to_bits(), case.reactivity.to_bits());
    assert_eq!(direct.state.coolant_flow_held_m_per_s.to_bits(), case.flow.to_bits());
    assert_eq!(
        tape["inputs"]["segments"][1]["load"]["reactivity_inserted_dollar"].as_f64().unwrap().to_bits(),
        direct.state.burst_reactivity_dollar.to_bits()
    );
    // The same 200 steps asked as one segment: the same final state and the
    // same 48 written bytes in every frame. The request differs, so the proof
    // bytes and the seals do.
    let (status, alone) = post(&base, "/run_trajectory", &run_body(case, ceiling(), 200, 50)).await;
    assert_eq!(status, 200, "{alone}");
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["final_state"], tape["final_state"]);
    assert_eq!(alone["state_digest"], tape["state_digest"]);
    let (in_segments, in_one) = (tape["frames"].as_array().unwrap(), alone["frames"].as_array().unwrap());
    assert_eq!(in_segments.len(), 4);
    assert_eq!(in_one.len(), 4);
    for (a, b) in in_segments.iter().zip(in_one) {
        assert_eq!(recorded(a).0, recorded(b).0);
        assert_eq!(recorded(a).1[..48], recorded(b).1[..48]);
    }
    assert_ne!(alone["run_seal"], tape["run_seal"]);
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
    // A changed number in the state breaks the tape.
    tape["final_state"]["fuel_temperature_rise_k_hex"] = json!("3ff0000000000000");
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");
    let _ = reactor;
}

/// Two requests the door once answered, one with a completed, sealed run
/// whose inserted reactivity jumped by 0.1 dollar in one step and one with a
/// sealed first maximum of 0.049 MW: each is refused by name before its
/// first step, and nothing of it is sealed. A flow changed in a later segment
/// is answered, each step at its own flow.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reactivity_changed_in_a_later_segment_refuses_the_whole_run_by_name_and_a_changed_flow_is_answered() {
    let base = spawn_app().await;
    let reactor = reactor();
    let inlet_k: f64 = 294.26111111111106;
    let dt_s: f64 = 0.00043000000000000004;
    assert_eq!(dt_s.to_bits(), ceiling().to_bits());
    let cold = |rho: f64| json!({ "reactivity_inserted_dollar": rho, "coolant_flow_m_per_s": 0.0 });
    let run = |inlet: f64, segments: Value| {
        json!({
            "domain": "energy",
            "card": CARD,
            "initial": { "inlet_temperature_k": inlet },
            "dt_s": dt_s,
            "segments": segments,
            "record_every": 100000
        })
    };
    let nothing_sealed = |text: &str| {
        for word in ["run_seal", "state_digest", "receipt_sha256", "frame_hex", "final_state", "sha256_seal"] {
            assert!(!text.contains(word), "{word}: {text}");
        }
    };
    let reactivity_changed = |to: &str, from: &str| {
        format!(
            "Physical Failure: 'reactivity_inserted_dollar' {to} dollar is not the {from} dollar of the first segment: a run of the reactor_spert3_fuel card is one burst with one reactivity, ramped in and then held (ReactivityChangedDuringBurst). That state is not sealed."
        )
    };
    // 400 steps at 1.1 dollars, the rod stopped since step 141, then one step at 1.2.
    let jump = run(inlet_k, json!([{ "steps": 400, "load": cold(1.1) }, { "steps": 1, "load": cold(1.2) }]));
    let (status, text) = post(&base, "/run_trajectory", &jump).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, reactivity_changed("1.2", "1.1"));
    nothing_sealed(&text);
    // 400 steps at 1.1 dollars, then 0.9 to the end.
    let fall = run(inlet_k, json!([{ "steps": 400, "load": cold(1.1) }, { "steps": 46000, "load": cold(0.9) }]));
    let (status, text) = post(&base, "/run_trajectory", &fall).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, reactivity_changed("0.9", "1.1"));
    nothing_sealed(&text);
    // Before the rod has stopped, on the second step, and by one bit: the same.
    for (steps, rho) in [(50, 1.2), (1, 1.2), (400, f64::from_bits(1.1_f64.to_bits() + 1))] {
        let early = run(inlet_k, json!([{ "steps": steps, "load": cold(1.1) }, { "steps": 10, "load": cold(rho) }]));
        let (status, text) = post(&base, "/run_trajectory", &early).await;
        assert_eq!(status, 422, "{text}");
        assert!(names(&text, "ReactivityChangedDuringBurst"), "{text}");
        nothing_sealed(&text);
    }
    // A change and a return is a change.
    let saw = run(
        inlet_k,
        json!([{ "steps": 200, "load": cold(1.2) }, { "steps": 1, "load": cold(1.19) }, { "steps": 100, "load": cold(1.2) }]),
    );
    let (status, text) = post(&base, "/run_trajectory", &saw).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, reactivity_changed("1.19", "1.2"));
    // The flow, in a hot band: 24 ft/s, then 4.8 ft/s.
    let hot = (500.0 - 32.0) / 1.8 + 273.15;
    let (fast, slow) = (reactor.band_flow_max_m_per_s[2], reactor.band_flow_min_m_per_s[2]);
    let with_flow = |flow: f64| json!({ "reactivity_inserted_dollar": 1.1, "coolant_flow_m_per_s": flow });
    let flow_changed = run(hot, json!([{ "steps": 400, "load": with_flow(fast) }, { "steps": 46000, "load": with_flow(slow) }]));
    let (status, text) = post(&base, "/run_trajectory", &flow_changed).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "ended", "{text}");
    assert_eq!(tape["reject"], "PastFirstMaximum");
    // The kernel loop that changes its flow at the same step.
    let mut state = ReactorState::try_start(&reactor, hot).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut landed = 0_u64;
    let mut worst = 0.0_f64;
    let end = loop {
        let flow = if landed < 400 { fast } else { slow };
        match reactor_spert3_fuel::step(&reactor, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: flow }, dt_s) {
            Ok(()) => {
                landed += 1;
                worst = worst.max(state.residual_conservation_error);
            }
            Err(reject) => break reject,
        }
    };
    assert_eq!(end, StepReject::PastFirstMaximum);
    assert_eq!(tape["steps_landed"], landed);
    assert_state_is(&tape["final_state"], &state, "a flow changed");
    assert!(worst < reactor.residual_conservation_limit, "{worst:e}");
    // And in the cold band, tested with no flow: a flow from the 400th step on.
    let cold_flow = run(inlet_k, json!([{ "steps": 400, "load": cold(1.1) }, { "steps": 46000, "load": with_flow(5.0) }]));
    let (status, text) = post(&base, "/run_trajectory", &cold_flow).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["reject"], "PastFirstMaximum", "{text}");
    // A dataset row that changes its load is not admitted: the same sentence, no run seal.
    let plan = json!({
        "domain": "energy",
        "card": CARD,
        "plan": { "kind": "list", "runs": [{
            "initial": jump["initial"],
            "dt_s": jump["dt_s"],
            "segments": jump["segments"],
            "record_every": jump["record_every"]
        }] },
        "store": "none",
        "return": "inline", "answer": "full"
    });
    let (status, plan_text) = post(&base, "/run_dataset", &plan).await;
    assert_eq!(status, 200, "{plan_text}");
    let answer: Value = serde_json::from_str(&plan_text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "unadmitted", "{plan_text}");
    assert!(answer["rows"][0]["run_seal"].is_null(), "{plan_text}");
    assert_eq!(answer["rows"][0]["reject_detail"], reactivity_changed("1.2", "1.1"), "{plan_text}");
    // The kernel's step refuses the same change, and keeps the state: the door's
    // refusal is the card's, read from the request before the first step.
    let mut state = ReactorState::try_start(&reactor, inlet_k).unwrap();
    let mut frame = blank_frame([0; 16]);
    let load = |rho: f64| ReactorLoad { reactivity_inserted_dollar: rho, coolant_flow_m_per_s: 0.0 };
    for _ in 0..400 {
        reactor_spert3_fuel::step(&reactor, &mut state, &mut frame, load(1.1), dt_s).unwrap();
    }
    let (before, frame_before) = (state, frame.to_bytes());
    for rho in [1.2, 0.9] {
        assert_eq!(
            reactor_spert3_fuel::step(&reactor, &mut state, &mut frame, load(rho), dt_s),
            Err(StepReject::ReactivityChangedDuringBurst)
        );
        assert_eq!(state, before);
        assert_eq!(frame.to_bytes(), frame_before);
    }
    // Under the one load the run goes on to its first maximum, and it is the run
    // of 1.1 dollars alone, seal for seal.
    let whole = run(inlet_k, json!([{ "steps": 46000, "load": cold(1.1) }]));
    let (status, answer) = post(&base, "/run_trajectory", &whole).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["reject"], "PastFirstMaximum");
    assert_eq!(tape["steps_landed"], 808);
    // In 0.7.0 the run is sealed as ended at its first maximum. The same last
    // link and state digest with the outcome "refused" give the seal recorded
    // for this run before that change.
    assert_eq!(tape["outcome"], "ended");
    let last_link = tape["frames"].as_array().unwrap().last().unwrap()["link"].as_str().unwrap().to_string();
    let as_scored = format!(
        r#"{last_link}{{"outcome":"refused","steps_landed":808,"reject":"PastFirstMaximum","state_digest":"{}"}}"#,
        tape["state_digest"].as_str().unwrap()
    );
    assert_eq!(hex::encode(Sha256::digest(as_scored.as_bytes())), "34b919271d8801dd42932fa9bc2276cc1f909a0e66c6ff30aaf9914403d940d0");
    let as_ended = as_scored.replace(r#""outcome":"refused""#, r#""outcome":"ended""#);
    assert_eq!(tape["run_seal"], hex::encode(Sha256::digest(as_ended.as_bytes())));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_of_floor_steps_only_completes_with_the_rod_still_moving() {
    let base = spawn_app().await;
    // A run that asks for the most the door allows, at the floor, lands every
    // step and ends long before the rod has stopped: completed, not refused.
    let case = case_in(0, 0.0, 0.0);
    let request = run_body(case, floor(), 1_000_000, 100_000);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], 1_000_000);
    assert_eq!(tape["final_state"]["past_first_maximum"], false);
    assert_eq!(tape["final_state"]["rod_moving"], true);
    let one_millisecond = published(&tape["final_state"], "t_s");
    assert!(one_millisecond > 9.99e-4 && one_millisecond < 1.01e-3, "{one_millisecond}");
    let direct = direct_loop(case, floor(), 1_000_000, |_| false);
    assert_eq!(direct.reject, None);
    assert_state_is(&tape["final_state"], &direct.state, "a million floor steps");
    // One step past the limit is refused before anything runs.
    let (status, text) = post(&base, "/run_trajectory", &run_body(case, floor(), 1_000_001, 100_000)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("more than 1000000 steps"), "{text}");
}

/// Over the published span of each band, at the ceiling, every run the door
/// can start ends in a refusal the description names as the end of a run, and
/// never in a breach of the conservation ledger.
#[test]
fn over_the_span_of_each_band_every_run_at_the_ceiling_ends_by_name_and_the_ledger_holds() {
    let reactor = reactor();
    let mut worst_steps = 0;
    let mut worst_residual = 0.0_f64;
    let mut ends = std::collections::BTreeMap::new();
    for band in 0..3 {
        for index in 0..=8 {
            for flow_fraction in [0.0, 1.0] {
                let mut case = case_in(band, index as f64 / 8.0, flow_fraction);
                if index % 2 == 1 {
                    case.inlet_k = reactor.band_inlet_min_k[band];
                }
                let direct = direct_loop(case, ceiling(), 1_000_000, |_| false);
                let reject = direct.reject.unwrap_or_else(|| panic!("{case:?} never ended"));
                assert!(
                    matches!(reject, StepReject::PastFirstMaximum),
                    "{case:?}: {reject:?} after {} steps",
                    direct.landed
                );
                // Inside the compared spans: under 20 s, 30 MW s and 1000 MW.
                assert!(direct.state.t_s <= 20.0);
                assert!(direct.state.energy_mws <= 30.0);
                assert!(direct.state.peak_power_mw <= 1000.0);
                assert!(direct.largest_residual <= reactor.residual_conservation_limit);
                worst_steps = worst_steps.max(direct.landed);
                worst_residual = worst_residual.max(direct.largest_residual);
                *ends.entry(format!("{reject:?}")).or_insert(0_u32) += 1;
            }
        }
    }
    assert!(worst_steps < 1_000_000);
    println!(
        "ends over the span: {ends:?}; most steps {worst_steps}; largest residual {worst_residual:e} against a limit of {:e}",
        reactor.residual_conservation_limit
    );
}

// ------------------------------------------------------------- the bounds

fn names(text: &str, name: &str) -> bool {
    text.contains(&format!("({name}). That state is not sealed.")) && !text.contains("sha256_seal")
}

/// The step, the run, a dataset row and a grid axis all treat a request the
/// same way: `expected` is `None` when it lands and the refusal's name when
/// it does not. `axis` names the field the grid varies.
async fn on_every_route(base: &str, case: Case, expected: Option<&str>, axis: &str) {
    let label = format!("{case:?}");
    let step = step_body(case, ceiling());
    let run = run_body(case, ceiling(), 5, 1);
    let plan = json!({
        "domain": "energy",
        "card": CARD,
        "plan": { "kind": "list", "runs": [{
            "initial": { "inlet_temperature_k": case.inlet_k },
            "dt_s": ceiling(),
            "segments": [{ "steps": 5, "load": load_json(case) }],
            "record_every": 1
        }] },
        "store": "none",
        "return": "inline", "answer": "full"
    });
    let (step_status, step_text) = post(base, "/verify_physics", &step).await;
    let (run_status, run_text) = post(base, "/run_trajectory", &run).await;
    let (plan_status, plan_text) = post(base, "/run_dataset", &plan).await;
    let Some(name) = expected else {
        assert_eq!(step_status, 200, "{label}: {step_text}");
        assert_eq!(run_status, 200, "{label}: {run_text}");
        assert_eq!(plan_status, 200, "{label}: {plan_text}");
        let tape: Value = serde_json::from_str(&run_text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{label}");
        let answer: Value = serde_json::from_str(&plan_text).unwrap();
        assert_eq!(answer["rows"][0]["run_seal"], tape["run_seal"], "{label}");
        return;
    };
    for (status, text) in [(step_status, &step_text), (run_status, &run_text)] {
        assert_eq!(status, 422, "{label}: {text}");
        assert!(names(text, name), "{label}: {text}");
        assert!(text.contains(&format!("'{axis}'")), "{label}: {text}");
    }
    assert_eq!(step_text, run_text, "{label}");
    // A dataset row outside the card is not admitted: it carries the same
    // sentence and no run seal, and nothing runs.
    assert_eq!(plan_status, 200, "{label}: {plan_text}");
    let answer: Value = serde_json::from_str(&plan_text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "unadmitted", "{plan_text}");
    assert!(answer["rows"][0]["run_seal"].is_null(), "{plan_text}");
    assert!(names(answer["rows"][0]["reject_detail"].as_str().unwrap(), name), "{plan_text}");
    // A later segment that carries the load refuses the whole run before its first step.
    if axis != "inlet_temperature_k" {
        let inside = case_in(reactor().band_of(case.inlet_k).unwrap(), 0.5, 0.5);
        let mut later = run_body(inside, ceiling(), 5, 1);
        later["segments"] = json!([
            { "steps": 5, "load": load_json(inside) },
            { "steps": 5, "load": load_json(case) }
        ]);
        let (status, text) = post(base, "/run_trajectory", &later).await;
        assert_eq!(status, 422, "{label}: {text}");
        assert_eq!(text, run_text, "{label}");
        assert!(!text.contains("run_seal"), "{text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_end_of_each_band_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let reactor = reactor();
    // The description publishes the widest of the three bands for the inlet, and the
    // physical limits for the load: a reactivity above zero and a flow of zero or more.
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "energy", "card": CARD})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    let least = |values: [f64; 3]| values.into_iter().fold(f64::INFINITY, f64::min);
    let most = |values: [f64; 3]| values.into_iter().fold(f64::NEG_INFINITY, f64::max);
    let field = &described["phase_fields"][0];
    assert_eq!(field["name"], "inlet_temperature_k");
    assert_eq!(field["minimum"].as_f64().unwrap().to_bits(), least(reactor.band_inlet_min_k).to_bits());
    assert_eq!(field["maximum"].as_f64().unwrap().to_bits(), most(reactor.band_inlet_max_k).to_bits());
    for (index, name, unit, exclusive) in [(0, "reactivity_inserted_dollar", "dollar", true), (1, "coolant_flow_m_per_s", "m/s", false)] {
        let field = &described["load_fields"][index];
        assert_eq!(field["name"], name);
        assert_eq!(field["unit"], unit);
        assert_eq!(field["minimum"].as_f64().unwrap().to_bits(), 0.0_f64.to_bits(), "{name}");
        assert_eq!(field["minimum_exclusive"], exclusive, "{name}");
        assert!(field["maximum"].is_null(), "{name}");
    }
    assert_eq!(described["phase_fields"].as_array().unwrap().len(), 1);
    assert_eq!(described["load_fields"].as_array().unwrap().len(), 2);

    for band in 0..3 {
        let middle = case_in(band, 0.5, 0.5);
        let (t_min, t_max) = (reactor.band_inlet_min_k[band], reactor.band_inlet_max_k[band]);
        let (rho_min, rho_max) = (
            reactor.band_reactivity_min_dollar[band],
            reactor.band_reactivity_max_dollar[band],
        );
        let (v_min, v_max) = (reactor.band_flow_min_m_per_s[band], reactor.band_flow_max_m_per_s[band]);
        // The numbers next to zero are refused by the number rule before a
        // bound is read, so a flow of zero is left through a small flow that
        // a 32-bit slot still holds.
        let under_flow = if v_min == 0.0 { -1.0e-30 } else { below(v_min) };
        let over_flow = if v_max == 0.0 { 1.0e-30 } else { above(v_max) };

        on_every_route(&base, middle, None, "").await;
        for inlet_k in [t_min, t_max] {
            on_every_route(&base, Case { inlet_k, ..middle }, None, "").await;
        }
        for inlet_k in [below(t_min), above(t_max)] {
            on_every_route(&base, Case { inlet_k, ..middle }, Some("InletTemperatureOutsideCard"), "inlet_temperature_k").await;
        }
        for reactivity in [rho_min, rho_max] {
            on_every_route(&base, Case { reactivity, ..middle }, None, "").await;
        }
        // The old walls of the band's reactivity and flow, and past them: answered.
        for reactivity in [below(rho_min), above(rho_max), 0.5, 1.3] {
            on_every_route(&base, Case { reactivity, ..middle }, None, "").await;
        }
        for reactivity in [0.0, -1.0] {
            on_every_route(&base, Case { reactivity, ..middle }, Some("ReactivityNotPositive"), "reactivity_inserted_dollar").await;
        }
        for flow in [v_min, v_max, over_flow, 0.0, 30.0] {
            on_every_route(&base, Case { flow, ..middle }, None, "").await;
        }
        if v_min > 0.0 {
            on_every_route(&base, Case { flow: under_flow, ..middle }, None, "").await;
        }
        for flow in [-1.0e-30, -1.0] {
            on_every_route(&base, Case { flow, ..middle }, Some("NegativeFlow"), "coolant_flow_m_per_s").await;
        }
    }

    // 300 degF lies between the second band and the third: nothing was
    // measured there, and the card refuses it by name with any load.
    let between = kelvin(300.0);
    assert!(reactor.band_of(between).is_none());
    assert!(between > reactor.band_inlet_max_k[1] && between < reactor.band_inlet_min_k[2]);
    for band in 0..3 {
        let case = Case { inlet_k: between, ..case_in(band, 0.5, 0.5) };
        on_every_route(&base, case, Some("InletTemperatureOutsideCard"), "inlet_temperature_k").await;
    }
    let (_, text) = post(&base, "/verify_physics", &step_body(Case { inlet_k: between, ..case_in(1, 0.5, 0.5) }, ceiling())).await;
    assert!(
        text.starts_with(&format!(
            "Physical Failure: 'inlet_temperature_k' {between} K is in none of the three tested bands of the {CARD} card: "
        )),
        "{text}"
    );
    for words in ["56 to 92 degF", "251 to 259 degF", "498 to 501 degF"] {
        assert!(text.contains(words), "{text}");
    }
    // The kernel refuses the same start: it builds no state there.
    assert!(ReactorState::try_start(&reactor, between).is_none());
    // The other gap, 170 degF, and the two sides of the whole card.
    for inlet_k in [kelvin(170.0), kelvin(40.0), kelvin(520.0)] {
        on_every_route(&base, Case { inlet_k, ..case_in(0, 0.5, 0.5) }, Some("InletTemperatureOutsideCard"), "inlet_temperature_k").await;
    }

    // A load of one band is answered in another.
    let cold = case_in(0, 0.5, 0.5);
    let hot = case_in(2, 0.5, 0.5);
    on_every_route(&base, Case { flow: hot.flow, ..cold }, None, "").await;
    on_every_route(&base, Case { flow: cold.flow, ..hot }, None, "").await;
    let widest_max = most(reactor.band_reactivity_max_dollar);
    let narrow = (0..3).find(|&band| reactor.band_reactivity_max_dollar[band] < widest_max).expect("the spans differ");
    on_every_route(&base, Case { reactivity: widest_max, ..case_in(narrow, 0.5, 0.5) }, None, "").await;
    // The sentence of a flow below zero names the field, the value and the limit.
    let (_, text) = post(&base, "/verify_physics", &step_body(Case { flow: -1.0, ..cold }, ceiling())).await;
    assert_eq!(
        text,
        format!("Physical Failure: 'coolant_flow_m_per_s' -1 m/s is below 0 m/s, the inclusive minimum of the {CARD} card (NegativeFlow). That state is not sealed.")
    );

    // A grid axis that carries a value outside the published bounds is refused row by row.
    let (grid_status, grid_text) = post(
        &base,
        "/run_dataset",
        &json!({
            "domain": "energy", "card": CARD,
            "plan": { "kind": "grid",
                "base": { "initial": { "inlet_temperature_k": cold.inlet_k }, "dt_s": ceiling(),
                          "segments": [{ "steps": 5, "load": { "coolant_flow_m_per_s": cold.flow } }], "record_every": 1 },
                "axes": [{ "path": "segments[0].load.reactivity_inserted_dollar", "values": [-1.0] }] },
            "store": "none", "return": "inline", "answer": "full"
        }),
    )
    .await;
    // A grid is admitted row by row: the row past the bound is unadmitted
    // with the door's sentence and carries no run seal.
    assert_eq!(grid_status, 200, "{grid_text}");
    let grid_answer: Value = serde_json::from_str(&grid_text).unwrap();
    let last = grid_answer["rows"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["outcome"], "unadmitted", "{grid_text}");
    assert_eq!(last["run_seal"], Value::Null, "{grid_text}");
    assert!(last["reject_detail"].is_string(), "{grid_text}");

    // A clock under the floor or over the ceiling is named as well.
    let (status, text) = post(&base, "/verify_physics", &step_body(cold, below(floor()))).await;
    assert_eq!(status, 422);
    assert!(text.contains(&format!("the step floor of the {CARD} card")), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(cold, above(ceiling()))).await;
    assert_eq!(status, 422);
    assert!(text.contains(&format!("the step ceiling of the {CARD} card")), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(cold, 0.0)).await;
    assert_eq!(status, 422);
    assert!(text.contains("NonPositiveDt"), "{text}");
    // A step at the floor and at the ceiling lands, in every band.
    for band in 0..3 {
        for dt in [floor(), ceiling()] {
            let (status, text) = post(&base, "/verify_physics", &step_body(case_in(band, 0.5, 0.5), dt)).await;
            assert_eq!(status, 200, "{text}");
        }
    }
    // Missing fields are refused, never filled in: the flow has no default,
    // not even in the cold band, where it can only be zero.
    for missing in ["inlet_temperature_k", "reactivity_inserted_dollar", "coolant_flow_m_per_s", "dt_s"] {
        let mut step = step_body(cold, ceiling());
        step["parameters"].as_object_mut().unwrap().remove(missing);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{missing}: {text}");
        assert!(text.contains(&format!("Missing required parameter: '{missing}' (InputMissing).")), "{text}");
    }
    let mut run = run_body(cold, ceiling(), 5, 1);
    run["segments"][0]["load"].as_object_mut().unwrap().remove("coolant_flow_m_per_s");
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("coolant_flow_m_per_s") && !text.contains("run_seal"), "{text}");
    // A kelvin below zero is refused before the card is asked.
    let (status, text) = post(&base, "/verify_physics", &step_body(Case { inlet_k: -1.0, ..cold }, ceiling())).await;
    assert_eq!(status, 422);
    assert!(text.contains("inlet_temperature_k") && !text.contains("sha256_seal"), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_band_answers_any_flow_and_each_tested_edge_lands_by_either_conversion() {
    let base = spawn_app().await;
    let reactor = reactor();
    let feet = |per_second: f64| per_second * 0.3048;
    // IDO-17281 Table V: 2.4, 14, 19 and 22 ft/s at 251 to 259 degF; 4.8, 14 and 24 ft/s at 498 to
    // 501 degF.
    assert_eq!(reactor.band_flow_min_m_per_s, [0.0, 0.73152, 1.46304]);
    assert_eq!(reactor.band_flow_max_m_per_s, [0.0, feet(22.0), feet(24.0)]);
    let warm = case_in(1, 0.5, 0.5);
    let hot = case_in(2, 0.5, 0.5);
    // 24 ft/s at 251 to 259 degF and 2.4 ft/s at 498 to 501 degF were answered by 0.6.0 and
    // refused from 0.7.0 to 0.10.1 as FlowOutsideCard; no test was run at either. Answered.
    for case in [
        Case { flow: feet(24.0), ..warm },
        Case { flow: feet(2.4), ..hot },
        Case { flow: 0.73152, ..hot },
        Case { flow: 7.3152, ..warm },
        Case { flow: 50.0, ..hot },
    ] {
        on_every_route(&base, case, None, "").await;
    }
    // Every tested flow of each hot band lands, as a decimal in m/s and converted from ft/s.
    for (case, flows) in [(warm, vec![2.4, 14.0, 19.0, 22.0]), (hot, vec![4.8, 14.0, 24.0])] {
        for fps in flows {
            on_every_route(&base, Case { flow: feet(fps), ..case }, None, "").await;
        }
    }
    for (case, flow) in [(warm, 0.73152), (warm, 6.7056), (hot, 1.46304), (hot, 7.3152)] {
        on_every_route(&base, Case { flow, ..case }, None, "").await;
    }
    // Each tested inlet edge lands converted either way: (F - 32) / 1.8 + 273.15
    // and (F + 459.67) / 1.8. At 92 and 259 degF the second is the double
    // nearest the exact edge, and 0.6.0 refused it.
    for (band, edges) in [(0, [56.0, 92.0]), (1, [251.0, 259.0]), (2, [498.0, 501.0])] {
        for fahrenheit in edges {
            for inlet_k in [kelvin(fahrenheit), (fahrenheit + 459.67) / 1.8] {
                assert_eq!(reactor.band_of(inlet_k), Some(band), "{fahrenheit} degF as {inlet_k} K");
                on_every_route(&base, Case { inlet_k, ..case_in(band, 0.5, 0.5) }, None, "").await;
            }
        }
    }
    assert_eq!(reactor.band_inlet_max_k[0], 306.48333333333335);
    assert_eq!(reactor.band_inlet_max_k[1], 399.2611111111111);
    assert_eq!(reactor.band_inlet_max_k[2], 533.7055555555556);
    // The description says what each band was compared at.
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "energy", "card": CARD})).await;
    // Re-pinned for 0.10.1: the bands are named by their spans. The middle band ends at 259 degF, and 260 degF
    // itself, the report's round name for the series, is above it and is refused.
    assert!(
        described.contains("The cold band was compared with no flow, 251 to 259 degF at 2.4 to 22 ft/s and 498 to 501 degF at 4.8 to 24 ft/s; the flow enters through the sweep of the water temperature alone."),
        "{described}"
    );
    for gone in ["260 degF band", "the 500 degF band", "at 260 degF and", "FlowOutsideCard", "ReactivityOutsideCard", "FlowChangedDuringBurst", "TimeBeyondCard", "EnergyBeyondCard", "PowerBeyondCard"] {
        assert!(!described.contains(gone), "still said: {gone}");
    }
    assert!(reactor.band_of(kelvin(260.0)).is_none());
    assert!(!described.contains("2.4 to 24 ft/s"), "{described}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_card_constant_sent_as_a_field_is_refused() {
    let base = spawn_app().await;
    let case = case_in(0, 0.5, 0.5);
    for constant in [
        "band_generation_time_reduced_s",
        "band_ramp_dollar_per_s2",
        "band_moderator_coefficient_dollar_per_k",
        "band_water_heat_capacity_mws_per_k",
        "band_reactivity_max_dollar",
        "band_flow_max_m_per_s",
        "initial_power_mw",
        "doppler_strength_dollar_per_sqrt_k",
        "fuel_heat_capacity_mws_per_k",
        "water_feedback_weight",
        "water_prompt_heating_fraction",
        "group_fraction",
        "group_decay_per_s",
        "growth_phase_step",
        "residual_conservation_limit",
        // The state the card builds for itself is not the caller's either.
        "fuel_temperature_rise_k",
        "water_temperature_rise_k",
        "power_mw",
        // The field of the first card of this reactor is not this card's.
        "coolant_temperature_k",
    ] {
        let mut step = step_body(case, ceiling());
        step["parameters"][constant] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert_eq!(text, format!("Physical Violation: '{constant}' is not a field of this request."));

        let mut in_phase = run_body(case, ceiling(), 3, 1);
        in_phase["initial"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_phase).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");

        let mut in_load = run_body(case, ceiling(), 3, 1);
        in_load["segments"][0]["load"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");
    }
}

// ---------------------------------------------------------------- dataset

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_grid_of_reactivities_and_flows_gives_rows_whose_run_seals_are_the_runs_alone() {
    let base = spawn_app().await;
    let reactivities = [case_in(1, 0.5, 0.0).reactivity, case_in(1, 1.0, 0.0).reactivity];
    let flows = [case_in(1, 0.0, 0.0).flow, case_in(1, 0.0, 1.0).flow];
    let inlet_k = case_in(1, 0.5, 0.5).inlet_k;
    for store in ["frames", "none"] {
        let request = json!({
            "domain": "energy",
            "card": CARD,
            "plan": {
                "kind": "grid",
                "base": {
                    "initial": { "inlet_temperature_k": inlet_k },
                    "dt_s": ceiling(),
                    "segments": [{ "steps": STEPS_ASKED, "load": {} }],
                    "record_every": RECORD_EVERY
                },
                "axes": [
                    { "path": "segments[0].load.reactivity_inserted_dollar", "values": reactivities },
                    { "path": "segments[0].load.coolant_flow_m_per_s", "values": flows }
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
        assert_eq!(rows.len(), 4);
        let mut seals = std::collections::BTreeSet::new();
        for row in rows {
            let load = &row["set"];
            let case = Case {
                inlet_k,
                reactivity: load["segments[0].load.reactivity_inserted_dollar"].as_f64().unwrap(),
                flow: load["segments[0].load.coolant_flow_m_per_s"].as_f64().unwrap(),
            };
            let direct = direct_loop(case, ceiling(), STEPS_ASKED, |_| false);
            let ended = direct.reject == Some(StepReject::PastFirstMaximum);
            assert_eq!(row["outcome"], if ended { "ended" } else { "refused" }, "{row}");
            assert_eq!(row["reject"], format!("{:?}", direct.reject.unwrap()), "{row}");
            assert_eq!(row["steps_landed"], direct.landed, "{row}");
            let (status, alone) = post(&base, "/run_trajectory", &run_body(case, ceiling(), STEPS_ASKED, RECORD_EVERY)).await;
            assert_eq!(status, 200, "{alone}");
            let alone: Value = serde_json::from_str(&alone).unwrap();
            assert_eq!(row["run_seal"], alone["run_seal"], "{store} {case:?}");
            assert_eq!(row["steps_landed"], alone["steps_landed"], "{store} {case:?}");
            seals.insert(row["run_seal"].as_str().unwrap().to_string());
        }
        // The flow is in the proof: four loads, four runs.
        assert_eq!(seals.len(), 4);
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
}

// ---------------------------------------------------------- validate_series

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_a_quantity_of_a_run_and_carries_its_run_seal() {
    let base = spawn_app().await;
    let case = case_in(2, 0.5, 0.5);
    let run = run_body(case, ceiling(), STEPS_ASKED, RECORD_EVERY);
    let (_, alone) = post(&base, "/run_trajectory", &run).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    let landed = alone["steps_landed"].as_u64().unwrap();
    assert!(landed > 3);
    let steps = [1, landed / 3, 2 * landed / 3, landed];
    let probe = json!({
        "run": run, "quantity": "power_mw",
        "measurements": steps.iter().map(|step| json!({ "step": step, "value": 1.0 })).collect::<Vec<_>>()
    });
    let (status, text) = post(&base, "/validate_series", &probe).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["outcome"], alone["outcome"]);
    assert_eq!(answer["compared"], 4);
    let model: Vec<f64> = answer["model_values_hex"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hex| f64::from_bits(u64::from_str_radix(hex.as_str().unwrap(), 16).unwrap()))
        .collect();
    // Each model value is the kernel loop's power at that step, to the bit.
    let direct = direct_loop(case, ceiling(), STEPS_ASKED, |_| false);
    assert_eq!(direct.landed, landed);
    for (step, value) in steps.iter().zip(&model) {
        let reactor = reactor();
        let mut state = ReactorState::try_start(&reactor, case.inlet_k).unwrap();
        let mut frame = blank_frame([0; 16]);
        for _ in 0..*step {
            reactor_spert3_fuel::step(&reactor, &mut state, &mut frame, load_of(case), ceiling()).unwrap();
        }
        assert_eq!(value.to_bits(), state.power_mw.to_bits(), "step {step}");
    }
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
    // Every number of the state is a quantity: the two temperatures among them.
    for (quantity, expected) in state_numbers(&direct.state) {
        let body = json!({ "run": run, "quantity": quantity,
                           "measurements": [{ "step": landed, "value": 1.0 }] });
        let (status, text) = post(&base, "/validate_series", &body).await;
        assert_eq!(status, 200, "{quantity}: {text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        let hex = answer["model_values_hex"][0].as_str().unwrap();
        assert_eq!(u64::from_str_radix(hex, 16).unwrap(), expected.to_bits(), "{quantity}");
    }
    // A flag is not a quantity.
    for flag in ["past_first_maximum", "prompt_supercritical", "rod_moving"] {
        let body = json!({ "run": run, "quantity": flag,
                           "measurements": [{ "step": landed, "value": 1.0 }] });
        let (status, _) = post(&base, "/validate_series", &body).await;
        assert_eq!(status, 422, "{flag}");
    }
}

// --------------------------------------------------------------- hosted door

const KEY_ALL: &str = "key-for-the-client-that-sees-everything";
const KEY_FUEL: &str = "key-for-the-fuel-reactor-client";
const KEY_FIRST: &str = "key-for-the-client-of-the-first-reactor-card";
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
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("reactor_fuel_hosted_{}", std::process::id()));
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
async fn the_three_connections_give_the_same_bytes_and_the_hosted_door_hides_the_card_from_a_key_without_it() {
    let base = spawn_app().await;
    let mut stdio = StdioDoor::start().await;
    let listed = result_text(&stdio.raw(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_cards","arguments":{}}}"#).await);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    // Thirty cards, this one the twentieth, in the energy domain, with
    // the first card of the reactor still served beside it.
    assert_eq!(cards.len(), 40);
    assert_eq!(cards[19]["domain"], "energy");
    assert_eq!(cards[19]["card"], CARD);
    assert!(cards.iter().any(|card| card["domain"] == "energy" && card["card"] == "reactor_spert3"));
    let all_pairs: Vec<Value> = cards.iter().map(|c| json!([c["domain"], c["card"]])).collect();
    let keys = json!([
        client_entry("all", KEY_ALL, Value::Array(all_pairs)),
        client_entry("fuel", KEY_FUEL, json!([["energy", CARD]])),
        client_entry("first", KEY_FIRST, json!([["energy", "reactor_spert3"]])),
        client_entry("thermal", KEY_THERMAL, json!([["energy", "thermal"]])),
    ]);
    let hosted = Hosted::start(&keys).await;

    let pair = json!({"domain": "energy", "card": CARD});
    let described = result_text(&stdio.raw(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"describe_card","arguments":pair}}).to_string()).await);
    let description: Value = serde_json::from_str(&described).unwrap();
    let step = description["example_compute_step"].clone();
    let run = description["example_run_trajectory"].clone();

    // The HTTP routes give the bytes of the tools.
    let (status, on_http) = post(&base, "/describe_card", &pair).await;
    assert_eq!(status, 200);
    assert_eq!(on_http, described);
    for (tool, path, arguments) in [("verify_step", "/verify_physics", &step), ("run_trajectory", "/run_trajectory", &run)] {
        let on_stdio = result_text(&stdio.raw(&answer_text::tool_call(7, tool, arguments).to_string()).await);
        let (status, on_http) = post(&base, path, arguments).await;
        assert_eq!(status, 200, "{on_http}");
        assert_eq!(on_http, on_stdio, "{tool}");
    }
    // The worked run is the kernel loop's, ended at the first maximum.
    let (_, tape_text) = post(&base, "/run_trajectory", &run).await;
    let tape: Value = serde_json::from_str(&tape_text).unwrap();
    let case = Case {
        inlet_k: run["initial"]["inlet_temperature_k"].as_f64().unwrap(),
        reactivity: run["segments"][0]["load"]["reactivity_inserted_dollar"].as_f64().unwrap(),
        flow: run["segments"][0]["load"]["coolant_flow_m_per_s"].as_f64().unwrap(),
    };
    let asked = run["segments"][0]["steps"].as_u64().unwrap();
    let direct = direct_loop(case, run["dt_s"].as_f64().unwrap(), asked, |_| false);
    assert_eq!(direct.reject, Some(StepReject::PastFirstMaximum));
    // The worked run asks for at least half as many steps again as it needs.
    assert!(direct.landed * 3 <= asked * 2, "{} of {asked}", direct.landed);
    assert_eq!(tape["steps_landed"], direct.landed);
    assert_state_is(&tape["final_state"], &direct.state, "the worked run");
    // The tape checks on the tool as on the route.
    let on_stdio = result_text(&stdio.raw(&answer_text::tool_call(7, "verify_tape", &tape).to_string()).await);
    let (status, on_http) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200);
    assert_eq!(on_http, on_stdio);
    assert_eq!(serde_json::from_str::<Value>(&on_http).unwrap()["holds"], true);

    // The full view and the key that lists it get the bytes of the stdio door.
    for key in [KEY_ALL, KEY_FUEL] {
        // A key that holds this card alone sees no coupling naming the other
        // SPERT III card: the couplings, last in the answer, are left out; nor
        // does it hold reactor_spert3_twin, so it is told no superseded_by
        // (succession::TABLE says a successor only where it is served).
        let expected = if key == KEY_FUEL {
            format!("{}}}", &described[..described.find(",\"couplings\":").expect("the couplings")]).replace("\"superseded_by\":\"reactor_spert3_twin\",", "")
        } else {
            described.clone()
        };
        assert_eq!(result_text(&hosted.tool(key, "describe_card", &pair).await), expected);
        for (tool, arguments) in [("verify_step", &step), ("run_trajectory", &run), ("verify_tape", &tape)] {
            let on_stdio = stdio.raw(&answer_text::tool_call(7, tool, arguments).to_string()).await;
            assert_eq!(hosted.tool(key, tool, arguments).await, on_stdio, "{tool}");
        }
    }
    // A run that changes its reactivity in a later segment is refused in the same
    // words on the route, on the tool and on the hosted door, with no proof.
    let mut changed = run.clone();
    changed["segments"] = json!([
        { "steps": 400, "load": { "reactivity_inserted_dollar": 1.1, "coolant_flow_m_per_s": 0.0 } },
        { "steps": 1, "load": { "reactivity_inserted_dollar": 1.2, "coolant_flow_m_per_s": 0.0 } }
    ]);
    let (status, on_http) = post(&base, "/run_trajectory", &changed).await;
    assert_eq!(status, 422, "{on_http}");
    assert_eq!(
        on_http,
        "Physical Failure: 'reactivity_inserted_dollar' 1.2 dollar is not the 1.1 dollar of the first segment: a run of the reactor_spert3_fuel card is one burst with one reactivity, ramped in and then held (ReactivityChangedDuringBurst). That state is not sealed."
    );
    let on_stdio = stdio.raw(&json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"run_trajectory","arguments":changed}}).to_string()).await;
    assert_eq!(result_text(&on_stdio), on_http);
    assert!(!on_stdio.contains("run_seal") && !on_stdio.contains("frame_hex"), "{on_stdio}");
    for key in [KEY_ALL, KEY_FUEL] {
        assert_eq!(hosted.tool(key, "run_trajectory", &changed).await, on_stdio);
    }
    // The description names the refusal and says a run carries one reactivity and each
    // segment its own flow.
    let rule = description.to_string();
    for words in [
        "A run is one burst, with one reactivity: every segment carries the reactivity of the first",
        "Each segment may carry its own coolant flow rate.",
        "ReactivityChangedDuringBurst",
        "NegativeState",
        "InconsistentState",
        "DtAboveStateCeiling",
    ] {
        assert!(rule.contains(words), "{words}");
    }
    assert!(!rule.contains("FlowChangedDuringBurst"));
    // After the kernel's 15 names the list holds the gateway's own three for this card, DtBelowFloor and DtAboveCeiling and ContinuationNotOnCard.
    assert_eq!(description["rejects"].as_array().unwrap().len(), 18);
    let seen = result_text(&hosted.tool(KEY_FUEL, "list_cards", &json!({})).await);
    let seen: Value = serde_json::from_str(&seen).unwrap();
    assert_eq!(seen["cards"].as_array().unwrap().len(), 1);
    assert_eq!(seen["cards"][0]["card"], CARD);
    // The key of this card does not open the first card of the reactor, and
    // the key of the first card does not open this one.
    let first = json!({"domain": "energy", "card": "reactor_spert3"});
    assert_eq!(result_text(&hosted.tool(KEY_FUEL, "describe_card", &first).await), "Card not recognized (UnknownCard).");
    assert_eq!(result_text(&hosted.tool(KEY_FIRST, "describe_card", &pair).await), "Card not recognized (UnknownCard).");
    let seen = result_text(&hosted.tool(KEY_FIRST, "list_cards", &json!({})).await);
    let seen: Value = serde_json::from_str(&seen).unwrap();
    assert_eq!(seen["cards"].as_array().unwrap().len(), 1);
    assert_eq!(seen["cards"][0]["card"], "reactor_spert3");

    // A key that does not list it gets the answers of a card that does not exist.
    let invented = "qqqqqqqqqqqqqqqqqqq";
    assert_eq!(invented.len(), CARD.len());
    let rename = |value: &Value| {
        let mut value = value.clone();
        value["card"] = json!(invented);
        value
    };
    let mut renamed_tape = tape.clone();
    renamed_tape["inputs"]["card"] = json!(invented);
    let mut session = Vec::new();
    for (tool, hidden, fake) in [
        ("describe_card", pair.clone(), rename(&pair)),
        ("verify_step", step.clone(), rename(&step)),
        ("run_trajectory", run.clone(), rename(&run)),
        ("verify_tape", tape.clone(), renamed_tape),
    ] {
        let on_hidden = hosted.tool(KEY_THERMAL, tool, &hidden).await;
        let on_fake = hosted.tool(KEY_THERMAL, tool, &fake).await;
        assert_eq!(on_hidden, on_fake, "{tool}");
        session.push(on_hidden);
    }
    let plan = json!({"domain": "energy", "card": CARD,
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
    for word in ["reactor", "spert", "reactivity", "inlet_temperature_k", "coolant_flow", "fuel_temperature", "rod_moving", "peak_power"] {
        assert!(!text.contains(word), "the word '{word}' of the hidden card was sent");
    }
    let tools = hosted.tool(KEY_THERMAL, "list_cards", &json!({})).await;
    assert!(!tools.to_lowercase().contains("reactor"));
}
