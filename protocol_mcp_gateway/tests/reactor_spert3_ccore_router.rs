//! The SPERT III C-core card on the web connection.
//!
//! A run of one step transient through the power peak to the first maximum of
//! the hot-spot plate temperature is the kernel loop, every published number
//! of its last state to the bit and every recorded frame byte for byte; it
//! ends with the outcome `ended`, its tape verifies and its receipt matches.
//! One step is the compiled step with the same frame bytes. Past every edge of
//! the compared range a request is answered as the kernel loop answers it: a
//! start outside 21 to 36 degC, a pressure outside 0 to 2500 psig, a flow at
//! any pressure and far above 20,000 gpm, a flow changed from segment to
//! segment, a reactivity far below and above the compared span, a run past
//! 200 s, 1e5 MW s and 2000 MW. Each physical limit is refused by name with no
//! proof: a pressure at or below absolute zero, a start that is not liquid
//! water, a flow below zero, a reactivity of zero or less, a reactivity changed
//! in a later segment, a step longer than the state lets it be. The card is
//! listed and described with its failed comparison stated and its end declared.
//!
//! No number a run produces is written here: every expected value comes from a
//! direct call into `genesis_kernel`.

mod answer_text;

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
const CARDS: usize = 37;
const PLACE: usize = 33;

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
            .body(answer_text::as_sent(body))
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
    run_body_at(case, segments, record_every, ceiling())
}

fn run_body_at(case: Case, segments: &[(u64, Case)], record_every: u64, dt_s: f64) -> Value {
    let segments: Vec<Value> = segments
        .iter()
        .map(|(steps, c)| json!({ "steps": steps, "load": load_json(*c) }))
        .collect();
    json!({
        "domain": "energy",
        "card": CARD,
        "initial": { "initial_temperature_k": case.initial_k, "system_pressure_psig": case.psig },
        "dt_s": dt_s,
        "segments": segments,
        "record_every": record_every
    })
}

fn step_body(case: Case) -> Value {
    step_body_at(case, ceiling())
}

fn step_body_at(case: Case, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": CARD,
        "parameters": {
            "initial_temperature_k": case.initial_k,
            "system_pressure_psig": case.psig,
            "reactivity_inserted_dollar": case.rho,
            "coolant_flow_m_per_s": case.flow,
            "dt_s": dt_s
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
    direct_segments(case, &[(limit, case)], record_every, ceiling())
}

/// The kernel loop over segments, each with its own load, at one step.
fn direct_segments(case: Case, segments: &[(u64, Case)], record_every: u64, dt_s: f64) -> Direct {
    let reactor = reactor();
    let mut state = ReactorState::try_start(&reactor, case.initial_k, case.psig).unwrap();
    let mut frame = blank_frame();
    let mut out = Direct { state, frame, landed: 0, reject: None, kept: Vec::new(), largest_residual: 0.0 };
    let loads: Vec<(u64, ReactorLoad)> = segments
        .iter()
        .map(|(steps, c)| (*steps, ReactorLoad { reactivity_inserted_dollar: c.rho, coolant_flow_m_per_s: c.flow }))
        .collect();
    'segments: for (steps, load) in loads {
      for _ in 0..steps {
        match reactor_spert3_ccore::step(&reactor, &mut state, &mut frame, load, dt_s) {
            Ok(()) => {
                out.landed += 1;
                out.largest_residual = out.largest_residual.max(state.residual_conservation_error);
                if out.landed % record_every == 0 {
                    out.kept.push((out.landed, frame));
                }
            }
            Err(reject) => {
                out.reject = Some(reject);
                break 'segments;
            }
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

/// The run answers as the kernel loop does, to the bit, with its proof holding.
async fn answered_as_the_kernel(base: &str, case: Case, segments: &[(u64, Case)], dt_s: f64, label: &str) -> (Value, Direct) {
    let record_every = 100_000;
    let (status, answer) = post(base, "/run_trajectory", &run_body_at(case, segments, record_every, dt_s)).await;
    assert_eq!(status, 200, "{label}: {answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    let direct = direct_segments(case, segments, record_every, dt_s);
    let requested: u64 = segments.iter().map(|(steps, _)| steps).sum();
    match direct.reject {
        Some(StepReject::PastPlateTemperatureMaximum) => {
            assert_eq!(tape["outcome"], "ended", "{label}: {}", tape["reject_detail"]);
            assert_eq!(tape["reject"], "PastPlateTemperatureMaximum", "{label}");
        }
        None => {
            assert_eq!(tape["outcome"], "completed", "{label}: {}", tape["reject_detail"]);
            assert_eq!(direct.landed, requested, "{label}");
        }
        Some(other) => panic!("{label}: the kernel loop stopped at {other:?}"),
    }
    assert_eq!(tape["steps_landed"], direct.landed, "{label}");
    assert_state_is(&tape["final_state"], &direct.state, label);
    assert!(direct.largest_residual <= reactor_spert3_ccore::RESIDUAL_CONSERVATION_LIMIT, "{label}");
    let (status, report) = post(base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{label}: {report}");
    (tape, direct)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn past_every_edge_of_the_compared_range_a_request_is_answered() {
    let base = spawn_app().await;
    let fine = Case { initial_k: 300.0, psig: 0.0, rho: 1.2, flow: 0.0 };
    // starts outside 21 to 36 degC, pressures outside 0 to 2500 psig, one of them above the critical pressure
    for case in [
        Case { initial_k: 273.15, ..fine },
        Case { initial_k: 372.0, ..fine },
        Case { initial_k: 523.15, psig: 2500.0, ..fine },
        Case { initial_k: 643.15, psig: 3500.0, ..fine },
        Case { psig: -14.0, ..fine },
        Case { psig: 5000.0, ..fine },
    ] {
        let label = format!("{} K at {} psig", case.initial_k, case.psig);
        answered_as_the_kernel(&base, case, &[(20_000, case)], ceiling(), &label).await;
    }
    // a flow at a pressure other than 230 psig, and far above 20,000 gpm at the state ceiling
    for (case, dt) in [
        (Case { flow: gpm(5000.0), ..fine }, ceiling()),
        (Case { psig: 2500.0, flow: gpm(20000.0), ..fine }, ceiling()),
        (Case { psig: 225.0, flow: 30.0, ..fine }, 1.3e-4),
    ] {
        let label = format!("{} m/s at {} psig", case.flow, case.psig);
        answered_as_the_kernel(&base, case, &[(20_000, case)], dt, &label).await;
    }
    // reactivities below 0.412 and above 1.305 dollars; 3 dollars peaks above 2000 MW
    let (_, low) = answered_as_the_kernel(&base, Case { rho: 0.1, ..fine }, &[(1_000, Case { rho: 0.1, ..fine })], ceiling(), "0.1 dollar").await;
    assert!(low.state.power_mw > reactor_spert3_ccore::INITIAL_POWER_MW);
    let three = Case { rho: 3.0, ..fine };
    let (tape, high) = answered_as_the_kernel(&base, three, &[(20_000, three)], 9.0e-5, "3 dollars").await;
    assert_eq!(tape["outcome"], "ended");
    assert!(high.state.peak_power_mw > 30_000.0, "{}", high.state.peak_power_mw);
    // past 200 s and 1e5 MW s: the slowest compared flow insertion with the most flow, to 360 s, completed
    let slow = Case { psig: 230.0, rho: reactor_spert3_ccore::REACTIVITY_MIN_FLOW_DOLLAR, flow: reactor_spert3_ccore::FLOW_MAX_M_PER_S, ..fine };
    let (_, long) = answered_as_the_kernel(&base, slow, &[(600_000, slow)], ceiling(), "360 s").await;
    assert!(long.state.t_s > 359.0 && long.state.energy_mws > 1.0e5, "{} MW s", long.state.energy_mws);
    // the flow changed from segment to segment: none, then 20,000 gpm, then 30 m/s
    let changing = [
        (1_000, fine),
        (1_000, Case { flow: gpm(20000.0), ..fine }),
        (20_000, Case { flow: 30.0, ..fine }),
    ];
    let (tape, direct) = answered_as_the_kernel(&base, fine, &changing, 1.0e-4, "flow changed").await;
    assert_eq!(tape["outcome"], "ended");
    assert!(direct.state.swept_energy_mws > 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_physical_limit_is_refused_by_name_with_no_proof() {
    let base = spawn_app().await;
    let fine = Case { initial_k: 300.0, psig: 0.0, rho: 1.0, flow: 0.0 };
    let boiling = reactor().saturation_temperature_k(0.0);
    for (case, name) in [
        (Case { initial_k: f64::from_bits(reactor_spert3_ccore::FREEZING_K.to_bits() - 1), ..fine }, "InitialTemperatureNotLiquidWater"),
        (Case { initial_k: boiling, ..fine }, "InitialTemperatureNotLiquidWater"),
        (Case { initial_k: 400.0, ..fine }, "InitialTemperatureNotLiquidWater"),
        (Case { initial_k: 630.0, psig: 2500.0, ..fine }, "InitialTemperatureNotLiquidWater"),
        (Case { initial_k: reactor_spert3_ccore::CRITICAL_TEMPERATURE_K, psig: 5000.0, ..fine }, "InitialTemperatureNotLiquidWater"),
        (Case { initial_k: 280.0, psig: -14.69, ..fine }, "InitialTemperatureNotLiquidWater"),
        (Case { psig: -reactor_spert3_ccore::ATMOSPHERE_PSI, ..fine }, "PressureNotPositive"),
        (Case { psig: -30.0, ..fine }, "PressureNotPositive"),
        (Case { flow: -1.0e-9, ..fine }, "NegativeFlow"),
        (Case { flow: -gpm(5000.0), psig: 230.0, ..fine }, "NegativeFlow"),
        (Case { rho: 0.0, ..fine }, "ReactivityNotPositive"),
        (Case { rho: -0.5, ..fine }, "ReactivityNotPositive"),
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
        Case { initial_k: reactor_spert3_ccore::FREEZING_K, ..fine },
        Case { initial_k: boiling.next_down(), ..fine },
        Case { psig: -14.0, initial_k: 280.0, ..fine },
        Case { rho: 1.0e-9, ..fine },
        Case { flow: 0.0, ..fine },
    ] {
        let (status, answer) = post(&base, "/verify_physics", &step_body(case)).await;
        assert_eq!(status, 200, "{answer}");
    }
    // a later segment with a flow below zero, or another reactivity, refuses the whole run
    for (second, name) in [
        (Case { flow: -1.0, ..fine }, "NegativeFlow"),
        (Case { rho: 1.05, ..fine }, "ReactivityChangedDuringRun"),
    ] {
        let (status, answer) = post(&base, "/run_trajectory", &run_body(fine, &[(10, fine), (10, second)], 1)).await;
        assert_ne!(status, 200, "{name}: {answer}");
        assert!(names(&answer, name), "{name}: {answer}");
        assert!(!answer.contains("run_seal"), "{name}: {answer}");
    }
    // a step longer than the state lets it be, in a single step and at the first step of a run
    for case in [Case { rho: 5.0, ..fine }, Case { rho: 1.2, flow: 300.0, ..fine }] {
        let (status, answer) = post(&base, "/verify_physics", &step_body(case)).await;
        assert_ne!(status, 200, "{answer}");
        assert!(names(&answer, "DtAboveStateCeiling"), "{answer}");
        assert!(answer.contains("Past the compared range the step ceiling follows the state"), "{answer}");
        let (status, answer) = post(&base, "/run_trajectory", &run_body(case, &[(10, case)], 1)).await;
        if status == 200 {
            let tape: Value = serde_json::from_str(&answer).unwrap();
            assert_eq!(tape["outcome"], "refused", "{answer}");
            assert_eq!(tape["reject"], "DtAboveStateCeiling", "{answer}");
            assert_eq!(tape["steps_landed"], 0, "{answer}");
        } else {
            assert!(names(&answer, "DtAboveStateCeiling"), "{answer}");
        }
    }
    // and part way: a segment that changes the flow to 300 m/s at 0.6 ms is refused at its first
    // step, and the run is sealed to the step before
    let still = Case { rho: 1.2, ..fine };
    let fast = Case { flow: 300.0, ..still };
    let segments = [(100, still), (10, fast)];
    let direct = direct_segments(still, &segments, 10, ceiling());
    assert_eq!(direct.reject, Some(StepReject::DtAboveStateCeiling));
    assert_eq!(direct.landed, 100);
    let (status, answer) = post(&base, "/run_trajectory", &run_body(still, &segments, 10)).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "refused", "{answer}");
    assert_eq!(tape["reject"], "DtAboveStateCeiling", "{answer}");
    assert_eq!(tape["steps_landed"], direct.landed);
    assert_state_is(&tape["final_state"], &direct.state, "300 m/s from step 101");
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
        "InitialTemperatureNotLiquidWater",
        "DtAboveStateCeiling",
        "Past the compared range the step ceiling follows the state",
        "13.5 percent",
        "2.2 times high",
    ] {
        assert!(described.contains(words), "{words}: {described}");
    }
    let described: Value = serde_json::from_str(&described).unwrap();
    assert!(described.to_string().contains("declared_ends"), "{described}");
}
