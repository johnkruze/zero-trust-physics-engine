//! The SPERT III C-core card on the web connection.
//!
//! A run of one step transient through the power peak to the first maximum of
//! the hot-spot plate temperature is the kernel loop, every published number
//! of its last state to the bit and every recorded frame byte for byte; it
//! ends with the outcome `ended`, its tape verifies and its receipt matches.
//! One step is the compiled step with the same frame bytes. Each region that
//! was not compared is refused by name with no proof: an initial temperature
//! or a pressure outside the compared span, a flow at a pressure at which no
//! flow was compared, a reactivity outside the span compared with the run's
//! flow, a load changed in a later segment. The card is listed and described
//! with its failed comparison stated and its end declared.
//!
//! No number a run produces is written here: every expected value comes from a
//! direct call into `genesis_kernel`.

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::reactor_spert3_ccore::{
    self, ReactorLoad, ReactorSpert3Ccore, ReactorState, StepReject,
};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const CARD: &str = "reactor_spert3_ccore";
/// The number of cards served, and this card's place among them.
const CARDS: usize = 31;
const PLACE: usize = 30;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(body).unwrap())
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

fn reactor() -> ReactorSpert3Ccore {
    ReactorSpert3Ccore::c_core_step_transients()
}

fn ceiling() -> f64 {
    reactor().gate_dt_s().unwrap()
}

fn gpm(flow_gpm: f64) -> f64 {
    flow_gpm * (18.0 / 20000.0) * 0.3048
}

#[derive(Clone, Copy)]
struct Case {
    initial_k: f64,
    psig: f64,
    rho: f64,
    flow: f64,
}

fn load_json(case: Case) -> Value {
    json!({ "reactivity_inserted_dollar": case.rho, "coolant_flow_m_per_s": case.flow })
}

fn run_body(case: Case, segments: &[(u64, Case)], record_every: u64) -> Value {
    let segments: Vec<Value> = segments
        .iter()
        .map(|(steps, c)| json!({ "steps": steps, "load": load_json(*c) }))
        .collect();
    json!({
        "domain": "energy",
        "card": CARD,
        "initial": { "initial_temperature_k": case.initial_k, "system_pressure_psig": case.psig },
        "dt_s": ceiling(),
        "segments": segments,
        "record_every": record_every
    })
}

fn step_body(case: Case) -> Value {
    json!({
        "domain": "energy",
        "card": CARD,
        "parameters": {
            "initial_temperature_k": case.initial_k,
            "system_pressure_psig": case.psig,
            "reactivity_inserted_dollar": case.rho,
            "coolant_flow_m_per_s": case.flow,
            "dt_s": ceiling()
        }
    })
}

fn blank_frame() -> LastStateFrame64 {
    LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [0; 16] }
}

struct Direct {
    state: ReactorState,
    frame: LastStateFrame64,
    landed: u64,
    reject: Option<StepReject>,
    kept: Vec<(u64, LastStateFrame64)>,
    largest_residual: f64,
}

fn direct_loop(case: Case, limit: u64, record_every: u64) -> Direct {
    let reactor = reactor();
    let mut state = ReactorState::try_start(&reactor, case.initial_k, case.psig).unwrap();
    let mut frame = blank_frame();
    let load = ReactorLoad { reactivity_inserted_dollar: case.rho, coolant_flow_m_per_s: case.flow };
    let mut out = Direct { state, frame, landed: 0, reject: None, kept: Vec::new(), largest_residual: 0.0 };
    for _ in 0..limit {
        match reactor_spert3_ccore::step(&reactor, &mut state, &mut frame, load, ceiling()) {
            Ok(()) => {
                out.landed += 1;
                out.largest_residual = out.largest_residual.max(state.residual_conservation_error);
                if out.landed % record_every == 0 {
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

fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")].as_str().unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

/// Every published number is the kernel's, to the bit, and nothing else is published.
fn assert_state_is(published_state: &Value, state: &ReactorState, label: &str) {
    let ReactorState {
        t_s,
        power_mw,
        precursor_mw,
        plate_rise_k,
        water_rise_k,
        hot_spot_plate_rise_k,
        hot_spot_water_rise_k,
        energy_mws,
        initial_temperature_k,
        system_pressure_psig,
        power_rate_mw_per_s,
        hot_spot_plate_rate_k_per_s,
        reactivity_net_dollar,
        population_anchor_mw,
        feedback_integral_dollar_mws,
        swept_energy_mws,
        hot_spot_swept_energy_mws,
        population_defect_mw,
        energy_defect_mws,
        hot_spot_energy_defect_mws,
        residual_conservation_error,
        peak_power_mw,
        peak_energy_mws,
        peak_time_s,
        plate_rise_max_k,
        plate_rise_max_time_s,
        run_reactivity_dollar: _,
        run_coolant_flow_m_per_s: _,
        past_power_peak,
        past_plate_maximum,
        prompt_supercritical,
    } = *state;
    let numbers = [
        ("t_s", t_s),
        ("power_mw", power_mw),
        ("energy_mws", energy_mws),
        ("power_rate_mw_per_s", power_rate_mw_per_s),
        ("hot_spot_plate_rise_k", hot_spot_plate_rise_k),
        ("hot_spot_plate_rate_k_per_s", hot_spot_plate_rate_k_per_s),
        ("hot_spot_water_rise_k", hot_spot_water_rise_k),
        ("plate_rise_k", plate_rise_k),
        ("water_rise_k", water_rise_k),
        ("reactivity_net_dollar", reactivity_net_dollar),
        ("initial_temperature_k", initial_temperature_k),
        ("system_pressure_psig", system_pressure_psig),
        ("precursor_1_mw", precursor_mw[0]),
        ("precursor_2_mw", precursor_mw[1]),
        ("precursor_3_mw", precursor_mw[2]),
        ("precursor_4_mw", precursor_mw[3]),
        ("precursor_5_mw", precursor_mw[4]),
        ("precursor_6_mw", precursor_mw[5]),
        ("population_anchor_mw", population_anchor_mw),
        ("feedback_integral_dollar_mws", feedback_integral_dollar_mws),
        ("swept_energy_mws", swept_energy_mws),
        ("hot_spot_swept_energy_mws", hot_spot_swept_energy_mws),
        ("population_defect_mw", population_defect_mw),
        ("energy_defect_mws", energy_defect_mws),
        ("hot_spot_energy_defect_mws", hot_spot_energy_defect_mws),
        ("residual_conservation_error", residual_conservation_error),
        ("peak_power_mw", peak_power_mw),
        ("peak_energy_mws", peak_energy_mws),
        ("peak_time_s", peak_time_s),
        ("plate_rise_max_k", plate_rise_max_k),
        ("plate_rise_max_time_s", plate_rise_max_time_s),
    ];
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in numbers {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    for (name, value) in [
        ("past_power_peak", past_power_peak),
        ("past_plate_maximum", past_plate_maximum),
        ("prompt_supercritical", prompt_supercritical),
    ] {
        assert_eq!(object[name], value, "{label} {name}");
        keys.insert(name.to_string());
    }
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
    // The temperature published is a plate surface temperature, never a fuel temperature.
    assert!(!found.iter().any(|name| name.contains("fuel")), "{label}");
}

fn recorded(frame: &Value) -> (u64, Vec<u8>, Vec<u8>) {
    let bytes = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap();
    let link = hex::decode(frame["link"].as_str().unwrap()).unwrap();
    (frame["step"].as_u64().unwrap(), bytes, link)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_transient_runs_to_the_plate_maximum_as_the_kernel_loop_and_ends_with_its_proof() {
    let base = spawn_app().await;
    let r = reactor();
    for case in [
        Case { initial_k: 298.15, psig: 0.0, rho: r.inhour_dollar(0.0107), flow: 0.0 },
        Case { initial_k: 305.15, psig: 2500.0, rho: reactor_spert3_ccore::REACTIVITY_MAX_DOLLAR, flow: 0.0 },
        Case { initial_k: 304.15, psig: 230.0, rho: r.inhour_dollar(0.022), flow: gpm(20000.0) },
        Case { initial_k: 304.15, psig: 230.0, rho: r.inhour_dollar(0.04), flow: reactor_spert3_ccore::FLOW_MIN_M_PER_S },
    ] {
        let label = format!("{} K, {} psig, {} dollar, {} m/s", case.initial_k, case.psig, case.rho, case.flow);
        let limit = 20_000;
        let record_every = 50;
        let (status, answer) = post(&base, "/run_trajectory", &run_body(case, &[(limit, case)], record_every)).await;
        assert_eq!(status, 200, "{label}: {answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        let direct = direct_loop(case, limit, record_every);
        assert_eq!(direct.reject, Some(StepReject::PastPlateTemperatureMaximum), "{label}");
        assert_eq!(tape["outcome"], "ended", "{label}: {}", tape["reject_detail"]);
        assert_eq!(tape["reject"], "PastPlateTemperatureMaximum", "{label}");
        assert_eq!(tape["steps_landed"], direct.landed, "{label}");
        assert_state_is(&tape["final_state"], &direct.state, &label);
        assert!(direct.state.past_power_peak && direct.state.past_plate_maximum, "{label}");
        assert!(direct.state.peak_time_s < direct.state.plate_rise_max_time_s, "{label}");
        assert_eq!(
            published(&tape, "max_residual_conservation_error").to_bits(),
            direct.largest_residual.to_bits(),
            "{label}"
        );
        assert!(direct.largest_residual <= reactor_spert3_ccore::RESIDUAL_CONSERVATION_LIMIT, "{label}");
        // The recorded frames are the kernel's, and the last is the frame of the last landed step.
        let frames = tape["frames"].as_array().unwrap();
        let mut expected = direct.kept.clone();
        if direct.landed % record_every != 0 {
            expected.push((direct.landed, direct.frame));
        }
        assert_eq!(frames.len(), expected.len(), "{label}");
        for (frame, (step, kernel_frame)) in frames.iter().zip(&expected) {
            let (at, bytes, link) = recorded(frame);
            assert_eq!(at, *step, "{label}");
            assert_eq!(bytes[..48], kernel_frame.to_bytes()[..48], "{label} step {step}");
            assert_eq!(link, Sha256::digest(&bytes).to_vec(), "{label} step {step}: the link");
        }
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{label}: {report}");
        let (status, receipt) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{receipt}");
        let receipt: Value = serde_json::from_str(&receipt).unwrap();
        assert_eq!(receipt["matches"], true, "{label}: {receipt}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_step_is_the_compiled_step() {
    let base = spawn_app().await;
    let case = Case { initial_k: 300.0, psig: 1000.0, rho: 1.1, flow: 0.0 };
    let (status, answer) = post(&base, "/verify_physics", &step_body(case)).await;
    assert_eq!(status, 200, "{answer}");
    let answer: Value = serde_json::from_str(&answer).unwrap();
    let direct = direct_loop(case, 1, 1);
    assert_eq!(direct.landed, 1);
    assert_state_is(&answer["state"], &direct.state, "one step");
    assert!(answer["sha256_seal"].as_str().is_some_and(|seal| seal.len() == 64), "{answer}");
}

fn names(text: &str, name: &str) -> bool {
    text.contains(&format!("({name})"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_region_not_compared_is_refused_by_name_with_no_proof() {
    let base = spawn_app().await;
    let r = reactor();
    let fine = Case { initial_k: 300.0, psig: 0.0, rho: 1.0, flow: 0.0 };
    let loop230 = Case { psig: 230.0, flow: gpm(5000.0), ..fine };
    for (case, name) in [
        (Case { initial_k: f64::from_bits(reactor_spert3_ccore::INITIAL_TEMPERATURE_MIN_K.to_bits() - 1), ..fine }, "InitialTemperatureOutsideCard"),
        (Case { initial_k: f64::from_bits(reactor_spert3_ccore::INITIAL_TEMPERATURE_MAX_K.to_bits() + 1), ..fine }, "InitialTemperatureOutsideCard"),
        (Case { psig: 2500.5, ..fine }, "PressureOutsideCard"),
        (Case { flow: gpm(5000.0), ..fine }, "FlowOutsideCard"),
        (Case { psig: 2500.0, flow: gpm(20000.0), ..fine }, "FlowOutsideCard"),
        (Case { flow: gpm(150.0), ..loop230 }, "FlowOutsideCard"),
        // flow was compared at 230 psig only
        (Case { psig: 225.0, ..loop230 }, "FlowOutsideCard"),
        (Case { psig: 250.0, ..loop230 }, "FlowOutsideCard"),
        (Case { psig: 225.0, flow: gpm(20000.0), ..loop230 }, "FlowOutsideCard"),
        (Case { psig: 250.0, flow: reactor_spert3_ccore::FLOW_MIN_M_PER_S, ..loop230 }, "FlowOutsideCard"),
        (Case { flow: f64::from_bits(reactor_spert3_ccore::FLOW_MAX_M_PER_S.to_bits() + 1), ..loop230 }, "FlowOutsideCard"),
        (Case { rho: f64::from_bits(reactor_spert3_ccore::REACTIVITY_MIN_NO_FLOW_DOLLAR.to_bits() - 1), ..fine }, "ReactivityOutsideCard"),
        (Case { rho: f64::from_bits(reactor_spert3_ccore::REACTIVITY_MAX_DOLLAR.to_bits() + 1), ..fine }, "ReactivityOutsideCard"),
        (Case { rho: r.inhour_dollar(1.0), ..loop230 }, "ReactivityOutsideCard"),
    ] {
        for path in ["/verify_physics", "/run_trajectory"] {
            let body = if path == "/verify_physics" { step_body(case) } else { run_body(case, &[(10, case)], 1) };
            let (status, answer) = post(&base, path, &body).await;
            assert_ne!(status, 200, "{name} on {path}: {answer}");
            assert!(names(&answer, name), "{name} on {path}: {answer}");
            assert!(!answer.contains("run_seal"), "{name} on {path}: {answer}");
        }
    }
    // the edges themselves land
    for case in [
        Case { initial_k: reactor_spert3_ccore::INITIAL_TEMPERATURE_MIN_K, ..fine },
        Case { initial_k: reactor_spert3_ccore::INITIAL_TEMPERATURE_MAX_K, psig: 2500.0, ..fine },
        Case { rho: reactor_spert3_ccore::REACTIVITY_MIN_NO_FLOW_DOLLAR, ..fine },
        Case { rho: reactor_spert3_ccore::REACTIVITY_MIN_FLOW_DOLLAR, flow: gpm(200.0), ..loop230 },
        Case { rho: reactor_spert3_ccore::REACTIVITY_MAX_DOLLAR, flow: gpm(20000.0), ..loop230 },
    ] {
        let (status, answer) = post(&base, "/verify_physics", &step_body(case)).await;
        assert_eq!(status, 200, "{answer}");
    }
    // a load changed in a later segment refuses the whole run
    for (second, name) in [
        (Case { rho: 1.05, ..fine }, "ReactivityChangedDuringRun"),
        (Case { flow: gpm(6000.0), ..loop230 }, "FlowChangedDuringRun"),
    ] {
        let first = if second.flow == 0.0 { fine } else { loop230 };
        let (status, answer) = post(&base, "/run_trajectory", &run_body(first, &[(10, first), (10, second)], 1)).await;
        assert_ne!(status, 200, "{name}: {answer}");
        assert!(names(&answer, name), "{name}: {answer}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_card_is_listed_and_described_with_its_failed_comparison_and_its_end() {
    let base = spawn_app().await;
    let client = Client::new();
    let listed: Value = client.get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let cards = listed.as_array().or_else(|| listed["cards"].as_array()).unwrap_or_else(|| panic!("{listed}"));
    assert_eq!(cards.len(), CARDS, "{listed}");
    assert_eq!(cards[PLACE]["card"], CARD, "{listed}");
    let (status, described) = post(&base, "/describe_card", &json!({ "domain": "energy", "card": CARD })).await;
    assert_eq!(status, 200, "{described}");
    for words in [
        "plate SURFACE temperature",
        "failed on all three lines",
        "6403.73",
        "0.551915",
        "no pressure term",
        "230 psig only",
        "not the physical share",
        "IDO-16918",
        "PastPlateTemperatureMaximum",
    ] {
        assert!(described.contains(words), "{words}: {described}");
    }
    let described: Value = serde_json::from_str(&described).unwrap();
    assert!(described.to_string().contains("declared_ends"), "{described}");
}
