//! The 18650 thermal-runaway card on the web connection.
//!
//! A step is held bit for bit to the compiled kernel step with the same proof
//! bytes, and its receipt verifies. A run of a fresh cell put in an oven at
//! 155 degC ends on the kernel loop's state to the bit, through its runaway:
//! the frames carry the self-heating and runaway flags where the kernel sets
//! them, the peak is in the frames, and the tape verifies. A calorimeter run
//! with the heater on, then off, crosses segments. The end, every reactant
//! consumed (the heat held falling to 0.0205 J), comes back `ended` on the
//! kernel loop's step; a spent cell under a heater is stepped. Each refusal a
//! request can reach is named and carries no proof: a temperature at or
//! below zero or at the can's melting point, an amount outside where its
//! equation holds, an oven at zero kelvin where heat crosses the can, a
//! heater below zero, an adiabatic switch neither 0 nor 1, a step outside
//! the card's floor and ceiling, a missing input, and a constant of the cell
//! sent as a field. Past the fresh cell's amounts, where 0.12.0 as frozen
//! refused, a start lands and its ledger closes.

mod answer_text;

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::runaway_18650_lco::{
    self, CAN_MELTING_K, FLAG_RUNAWAY, FLAG_SELF_HEATING, Runaway18650Lco, RunawayLoad, RunawayState,
};
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

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

fn cell() -> Runaway18650Lco {
    Runaway18650Lco::hatchard_kim()
}

fn ceiling() -> f64 {
    cell().gate_dt_s().unwrap()
}

fn floor() -> f64 {
    cell().floor_dt_s().unwrap()
}

const FRESH: [f64; 4] = [0.15, 0.75, 0.96, 1.0];

fn phase(temperature_k: f64, amounts: [f64; 4]) -> Value {
    json!({
        "temperature_k": temperature_k,
        "sei_fraction": amounts[0],
        "anode_lithium_fraction": amounts[1],
        "cathode_unreacted_fraction": amounts[2],
        "electrolyte_fraction": amounts[3],
    })
}

fn load_value(load: &RunawayLoad) -> Value {
    json!({
        "oven_temperature_k": load.oven_temperature_k,
        "heater_power_w": load.heater_power_w,
        "adiabatic": load.adiabatic,
    })
}

fn oven(oven_temperature_k: f64) -> RunawayLoad {
    RunawayLoad { oven_temperature_k, heater_power_w: 0.0, adiabatic: 0.0 }
}

fn step_body(temperature_k: f64, amounts: [f64; 4], load: &RunawayLoad, dt_s: f64) -> Value {
    let mut parameters = phase(temperature_k, amounts);
    for (key, value) in load_value(load).as_object().unwrap() {
        parameters[key] = value.clone();
    }
    parameters["dt_s"] = json!(dt_s);
    json!({ "domain": "energy", "card": "runaway_18650_lco", "parameters": parameters })
}

fn run_body(temperature_k: f64, amounts: [f64; 4], dt_s: f64, segments: &[(u64, RunawayLoad)], record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "runaway_18650_lco",
        "initial": phase(temperature_k, amounts),
        "dt_s": dt_s,
        "segments": segments.iter().map(|(steps, load)| json!({ "steps": steps, "load": load_value(load) })).collect::<Vec<_>>(),
        "record_every": record_every
    })
}

fn blank_frame(proof: [u8; 16]) -> LastStateFrame64 {
    LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof }
}

/// The kernel loop over the segments: the last state, the largest residual
/// and the state at every step.
fn direct_loop(temperature_k: f64, amounts: [f64; 4], dt_s: f64, segments: &[(u64, RunawayLoad)]) -> (RunawayState, f64, Vec<RunawayState>) {
    let cell = cell();
    let mut state = RunawayState::try_start(&cell, temperature_k, amounts[0], amounts[1], amounts[2], amounts[3]).unwrap();
    let mut frame = blank_frame([0; 16]);
    let (mut worst, mut kept) = (0.0_f64, Vec::new());
    for (steps, load) in segments {
        for _ in 0..*steps {
            runaway_18650_lco::step(&cell, &mut state, &mut frame, *load, dt_s).unwrap();
            worst = worst.max(state.residual_conservation_error);
            kept.push(state);
        }
    }
    (state, worst, kept)
}

fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")].as_str().unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

/// Every published number is the kernel's, to the bit, as the decimal and as
/// the hex beside it; the two flags are the kernel's; nothing else is published.
fn assert_state_is(published_state: &Value, state: &RunawayState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in [
        ("t_s", state.t_s),
        ("temperature_k", state.temperature_k),
        ("sei_fraction", state.sei_fraction),
        ("anode_lithium_fraction", state.anode_lithium_fraction),
        ("cathode_unreacted_fraction", state.cathode_unreacted_fraction),
        ("electrolyte_fraction", state.electrolyte_fraction),
        ("sei_thickness", state.sei_thickness),
        ("start_temperature_k", state.start_temperature_k),
        ("heat_sei_j", state.heat_sei_j),
        ("heat_anode_j", state.heat_anode_j),
        ("heat_cathode_j", state.heat_cathode_j),
        ("heat_electrolyte_j", state.heat_electrolyte_j),
        ("heat_released_j", state.heat_released_j),
        ("heater_heat_j", state.heater_heat_j),
        ("heat_lost_j", state.heat_lost_j),
        ("heat_stored_j", state.heat_stored_j),
        ("chemical_heat_w", state.chemical_heat_w),
        ("chemical_heating_k_s", state.chemical_heating_k_s),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
        ("substeps", state.substeps),
    ] {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    assert_eq!(object["self_heating"], state.self_heating, "{label}");
    assert_eq!(object["runaway"], state.runaway, "{label}");
    keys.insert("self_heating".to_string());
    keys.insert("runaway".to_string());
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

/// Time, temperature and flag word of one recorded frame.
fn slots(frame_hex: &str) -> (f64, f32, u64) {
    let bytes = hex::decode(frame_hex).unwrap();
    assert_eq!(bytes.len(), 64);
    (
        f64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        f32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_cell_matches_the_compiled_step_with_the_same_proof_bytes_and_its_receipt_verifies() {
    let base = spawn_app().await;
    let calorimeter = RunawayLoad { oven_temperature_k: 300.0, heater_power_w: 2.0, adiabatic: 1.0 };
    let cases = [
        step_body(298.15, FRESH, &oven(428.15), ceiling()),
        step_body(298.15, FRESH, &oven(428.15), floor()),
        step_body(470.0, [0.0, 0.65, 0.75, 0.999], &oven(428.15), 1.0),
        step_body(400.0, FRESH, &calorimeter, ceiling()),
        step_body(900.0, [0.0, 0.25, 0.0, 0.0], &oven(428.15), ceiling()),
    ];
    let mut seals = Vec::new();
    for request in cases {
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);
        let number = |key: &str| request["parameters"][key].as_f64().unwrap();
        let cell = cell();
        let mut state = RunawayState::try_start(
            &cell,
            number("temperature_k"),
            number("sei_fraction"),
            number("anode_lithium_fraction"),
            number("cathode_unreacted_fraction"),
            number("electrolyte_fraction"),
        )
        .unwrap();
        let load = RunawayLoad {
            oven_temperature_k: number("oven_temperature_k"),
            heater_power_w: number("heater_power_w"),
            adiabatic: number("adiabatic"),
        };
        let mut frame = blank_frame(commit_request_proof(&request));
        runaway_18650_lco::step(&cell, &mut state, &mut frame, load, number("dt_s")).unwrap();
        assert_eq!(frame.flags & !(FLAG_SELF_HEATING | FLAG_RUNAWAY), 0);
        assert_state_is(&response.state, &state, "step");
        assert_eq!(response.residual_conservation_error.to_bits(), state.residual_conservation_error.to_bits());
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha256);
        assert_eq!(sha256_hex(document.as_bytes()), receipt);
        let (status, report) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], true, "{report}");
        let changed = answer.replacen("\"t_s\":", "\"t_s\":1", 1);
        let (_, report) = post(&base, "/verify_receipt", &json!({ "response_text": changed })).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], false, "{report}");
        let (_, again) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(again, answer);
        seals.push((response.sha256_seal, frame.flags));
    }
    // The cell inside its runaway carries both flags; the fresh cell none.
    assert_eq!(seals[0].1, 0);
    assert_eq!(seals[2].1, FLAG_SELF_HEATING | FLAG_RUNAWAY);
    let mut distinct: Vec<&String> = seals.iter().map(|(seal, _)| seal).collect();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 5, "five different steps, five different proofs");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_oven_at_155_c_runs_away_through_the_gateway_as_the_kernel_loop_does_and_its_tape_verifies() {
    let base = spawn_app().await;
    let segments = [(200, oven(428.15))];
    let request = run_body(298.15, FRESH, 10.0, &segments, 1);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{}", &answer[..answer.len().min(400)]);
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 200);
    let (state, worst, kept) = direct_loop(298.15, FRESH, 10.0, &segments);
    assert_state_is(&tape["final_state"], &state, "200 steps");
    assert_eq!(tape["max_residual_conservation_error"].as_f64().unwrap().to_bits(), worst.to_bits());
    assert!(worst < 1.0e-7, "{worst:e}");
    let frames = tape["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 200);
    let mut peak = (0.0_f32, 0.0);
    let mut first_runaway = None;
    for (index, frame) in frames.iter().enumerate() {
        let (t_s, temperature, flags) = slots(frame["frame_hex"].as_str().unwrap());
        let kernel = &kept[index];
        assert_eq!(t_s, kernel.t_s);
        assert_eq!(temperature.to_bits(), (kernel.temperature_k as f32).to_bits(), "frame {index}");
        let expected = (if kernel.self_heating { FLAG_SELF_HEATING } else { 0 }) | (if kernel.runaway { FLAG_RUNAWAY } else { 0 });
        assert_eq!(flags, expected, "frame {index}");
        if temperature > peak.0 {
            peak = (temperature, t_s);
        }
        if first_runaway.is_none() && flags & FLAG_RUNAWAY != 0 {
            first_runaway = Some(t_s);
        }
    }
    // The runaway flag first stands at 1,420 s and the peak frame is at
    // 1,520 s, above 790 K; a quarter of the anode's lithium is left.
    assert_eq!(first_runaway, Some(1420.0));
    assert_eq!(peak.1, 1520.0);
    assert!(peak.0 > 790.0, "{peak:?}");
    let anode = published(&tape["final_state"], "anode_lithium_fraction");
    assert!((anode - 0.2518).abs() < 1.0e-3, "{anode}");
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_calorimeter_run_with_the_heater_on_then_off_crosses_its_segments_as_the_kernel_loop_does() {
    let base = spawn_app().await;
    let heating = RunawayLoad { oven_temperature_k: 298.15, heater_power_w: 35.41979526671116 / 60.0 * 2.0, adiabatic: 1.0 };
    let waiting = RunawayLoad { heater_power_w: 0.0, ..heating };
    let segments = [(300, heating), (300, waiting)];
    let request = run_body(353.15, FRESH, 10.0, &segments, 50);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{}", &answer[..answer.len().min(400)]);
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    let (state, _, _) = direct_loop(353.15, FRESH, 10.0, &segments);
    assert_state_is(&tape["final_state"], &state, "600 steps");
    assert_eq!(published(&tape["final_state"], "heat_lost_j"), 0.0);
    assert!(state.runaway || state.temperature_k > 600.0, "{}", state.temperature_k);
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
}

/// A thousandth of the electrolyte at 560 K in a calorimeter (1.04 J held)
/// meets the end, the heat held falling to 0.0205 J, near 2.8 s: at steps of
/// 0.5 s and of 0.1 s the run ends on the kernel loop's step and state, each
/// within its own length of the other. A spent start, every amount zero, is
/// stepped under its heater (the red team's request of 0.12.0, which ended
/// with no step landed) and warms by its power over its heat capacity; one
/// ulp of subnormal anode lithium held at 1,700 K is spent too, and completes
/// at the ceiling as at 1 s.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_reactant_consumed_ends_the_run() {
    let base = spawn_app().await;
    let calorimeter = RunawayLoad { oven_temperature_k: 560.0, heater_power_w: 0.0, adiabatic: 1.0 };
    let mut ends = Vec::new();
    for (dt_s, steps) in [(0.5, 20), (0.1, 100)] {
        let request = run_body(560.0, [0.0, 0.0, 0.0, 1.0e-3], dt_s, &[(steps, calorimeter)], 1);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["outcome"], "ended", "{tape}");
        assert_eq!(tape["reject"], "EveryReactantConsumed");
        assert!(tape["reject_detail"].as_str().unwrap().contains("0.0205 J"), "{}", tape["reject_detail"]);
        let cell = cell();
        let mut state = RunawayState::try_start(&cell, 560.0, 0.0, 0.0, 0.0, 1.0e-3).unwrap();
        let mut frame = blank_frame([0; 16]);
        let mut landed = 0_u64;
        let reject = loop {
            match runaway_18650_lco::step(&cell, &mut state, &mut frame, calorimeter, dt_s) {
                Ok(()) => landed += 1,
                Err(reject) => break reject,
            }
        };
        assert_eq!(reject, runaway_18650_lco::StepReject::EveryReactantConsumed);
        assert_eq!(tape["steps_landed"].as_u64(), Some(landed));
        assert!(landed > 3, "{landed}");
        assert_state_is(&tape["final_state"], &state, "ended");
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{report}");
        ends.push((dt_s, state.t_s));
    }
    assert!(ends[0].1 < ends[1].1 + ends[1].0 && ends[1].1 < ends[0].1 + ends[0].0, "{ends:?}");
    // A spent start under a heater: stepped, 1 K a second.
    let heater = RunawayLoad { oven_temperature_k: 298.15, heater_power_w: 35.41979526671116, adiabatic: 1.0 };
    let request = run_body(300.0, [0.0; 4], 1.0, &[(10, heater)], 10);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{tape}");
    assert_eq!(tape["steps_landed"], 10);
    let (state, _, _) = direct_loop(300.0, [0.0; 4], 1.0, &[(10, heater)]);
    assert_state_is(&tape["final_state"], &state, "spent under a heater");
    assert!((state.temperature_k - 310.0).abs() < 1.0e-9, "{}", state.temperature_k);
    // One ulp of subnormal anode lithium held at 1,700 K: spent, at either step.
    for (dt_s, steps) in [(ceiling(), 3000), (1.0, 3000)] {
        let request = run_body(1700.0, [0.0, f64::MIN_POSITIVE, 0.0, 0.0], dt_s, &[(steps, oven(1700.0))], steps);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["outcome"], "completed", "{dt_s}: {}", tape["reject_detail"]);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let least = 1.0 - runaway_18650_lco::CATHODE_CONVERTED_LEAST;
    let inside = [
        (f64::MIN_POSITIVE, FRESH),
        (below(CAN_MELTING_K), [0.0, 0.0, 0.0, 1.0e-300]),
        (300.0, [0.0, 0.0, 0.0, 1.0e-300]),
        (300.0, [0.0; 4]),
        (300.0, FRESH),
        // past the fresh cell's amounts, where 0.12.0 as frozen refused
        (300.0, [0.15000000000000002, 0.7500000000000001, 0.9600000000000001, 1.0]),
        (300.0, [1.0, 0.783, 1.0, 1.0]),
        (300.0, [0.15, 0.75, least, 1.0]),
    ];
    for (temperature_k, amounts) in inside {
        let (status, text) = post(&base, "/verify_physics", &step_body(temperature_k, amounts, &oven(300.0), 1.0)).await;
        assert_eq!(status, 200, "{temperature_k}: {text}");
    }
    let outside: [(f64, [f64; 4], &str); 14] = [
        (0.0, FRESH, "TemperatureNotPositive"),
        (-1.0, FRESH, "TemperatureNotPositive"),
        (CAN_MELTING_K, FRESH, "CanMelting"),
        (5000.0, FRESH, "CanMelting"),
        (300.0, [1.0000000000000002, 0.75, 0.96, 1.0], "SeiFractionOutsideCell"),
        (300.0, [-1.0e-300, 0.75, 0.96, 1.0], "SeiFractionOutsideCell"),
        (300.0, [0.15, 0.7830000000000001, 0.96, 1.0], "AnodeLithiumOutsideCell"),
        (300.0, [0.15, -0.5, 0.96, 1.0], "AnodeLithiumOutsideCell"),
        (300.0, [0.15, 0.75, 1.0000000000000002, 1.0], "CathodeUnreactedOutsideCell"),
        (300.0, [0.15, 0.75, f64::from_bits(least.to_bits() + 1), 1.0], "CathodeUnreactedOutsideCell"),
        (300.0, [0.15, 0.75, below(1.0), 1.0], "CathodeUnreactedOutsideCell"),
        (300.0, [0.15, 0.75, -0.1, 1.0], "CathodeUnreactedOutsideCell"),
        (300.0, [0.15, 0.75, 0.96, 1.0000000000000002], "ElectrolyteOutsideCell"),
        (300.0, [0.15, 0.75, 0.96, -1.0], "ElectrolyteOutsideCell"),
    ];
    for (temperature_k, amounts, name) in outside {
        let (status, text) = post(&base, "/verify_physics", &step_body(temperature_k, amounts, &oven(300.0), 1.0)).await;
        assert_eq!(status, 422, "{temperature_k} {amounts:?}: {text}");
        assert!(names(&text, name), "{temperature_k} {amounts:?}: {text}");
        let (status, run_text) = post(&base, "/run_trajectory", &run_body(temperature_k, amounts, 1.0, &[(3, oven(300.0))], 1)).await;
        assert_eq!(status, 422, "{run_text}");
        assert!(names(&run_text, name), "{run_text}");
    }
    let loads: [(RunawayLoad, &str); 6] = [
        (oven(0.0), "OvenTemperatureNotPositive"),
        (oven(-300.0), "OvenTemperatureNotPositive"),
        (RunawayLoad { heater_power_w: -1.0e-9, ..oven(300.0) }, "HeaterPowerNegative"),
        (RunawayLoad { adiabatic: 0.5, ..oven(300.0) }, "AdiabaticNotZeroOrOne"),
        (RunawayLoad { adiabatic: 2.0, ..oven(300.0) }, "AdiabaticNotZeroOrOne"),
        (RunawayLoad { adiabatic: -1.0, ..oven(300.0) }, "AdiabaticNotZeroOrOne"),
    ];
    for (load, name) in loads {
        let (status, text) = post(&base, "/verify_physics", &step_body(300.0, FRESH, &load, 1.0)).await;
        assert_eq!(status, 422, "{load:?}: {text}");
        assert!(names(&text, name), "{load:?}: {text}");
        // In a later segment of a run, the whole run is refused before its first step.
        let (status, run_text) = post(&base, "/run_trajectory", &run_body(300.0, FRESH, 1.0, &[(3, oven(300.0)), (3, load)], 1)).await;
        assert_eq!(status, 422, "{run_text}");
        assert!(names(&run_text, name), "{run_text}");
    }
    // With adiabatic 1 the oven temperature does not enter: 0 K is taken and
    // the step is the one at 298.15 K, to the bit (the proof differs with the
    // request); a negative number of kelvin is no temperature at the door.
    let calorimeter = |oven_temperature_k: f64| RunawayLoad { oven_temperature_k, heater_power_w: 0.0, adiabatic: 1.0 };
    let mut states = Vec::new();
    for oven_k in [298.15, 0.0, 1.0e-300] {
        let (status, text) = post(&base, "/verify_physics", &step_body(423.15, FRESH, &calorimeter(oven_k), 10.0)).await;
        assert_eq!(status, 200, "{oven_k}: {text}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&text).unwrap();
        states.push(response.state.to_string());
        let (status, run_text) = post(&base, "/run_trajectory", &run_body(423.15, FRESH, 10.0, &[(3, calorimeter(298.15)), (3, calorimeter(oven_k))], 1)).await;
        assert_eq!(status, 200, "{oven_k}: {run_text}");
    }
    assert_eq!(states[1], states[0]);
    assert_eq!(states[2], states[0]);
    let (status, text) = post(&base, "/verify_physics", &step_body(423.15, FRESH, &calorimeter(-1.0), 10.0)).await;
    assert_eq!(status, 422, "{text}");
    assert!(names(&text, "OvenTemperatureNotPositive"), "{text}");
    // A heater that would carry the cell past the can's melting point inside
    // a step: the step is refused by name and the run ends there, sealed.
    let melting = RunawayLoad { heater_power_w: 1.0e6, ..oven(300.0) };
    let (status, text) = post(&base, "/verify_physics", &step_body(300.0, FRESH, &melting, 1.0)).await;
    assert_eq!(status, 422, "{text}");
    assert!(names(&text, "CanMelting"), "{text}");
    for dt_s in [floor(), ceiling(), 1.0] {
        let (status, text) = post(&base, "/verify_physics", &step_body(300.0, FRESH, &oven(300.0), dt_s)).await;
        assert_eq!(status, 200, "{dt_s}: {text}");
    }
    for (dt_s, words) in [
        (below(floor()), "the step floor of the runaway_18650_lco card"),
        (f64::from_bits(ceiling().to_bits() + 1), "the step ceiling of the runaway_18650_lco card"),
        (60.0, "the step ceiling of the runaway_18650_lco card"),
        (0.0, "(NonPositiveDt) on the runaway_18650_lco card"),
        (-1.0, "(NonPositiveDt) on the runaway_18650_lco card"),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body(300.0, FRESH, &oven(300.0), dt_s)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains(words) && !text.contains("sha256_seal"), "{dt_s}: {text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_input_and_a_constant_of_the_cell_are_refused() {
    let base = spawn_app().await;
    let good = step_body(300.0, FRESH, &oven(400.0), 1.0);
    let (status, _) = post(&base, "/verify_physics", &good).await;
    assert_eq!(status, 200);
    for key in [
        "temperature_k",
        "sei_fraction",
        "anode_lithium_fraction",
        "cathode_unreacted_fraction",
        "electrolyte_fraction",
        "oven_temperature_k",
        "heater_power_w",
        "adiabatic",
        "dt_s",
    ] {
        let mut step = good.clone();
        step["parameters"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}' (InputMissing)."));
    }
    for key in [
        "sei_thickness",
        "heat_capacity_j_per_k",
        "cell_density_kg_per_m3",
        "convection_w_per_m2_k",
        "can_emissivity",
        "activation_j_per_mol",
        "frequency_factor_per_s",
        "start_temperature_k",
        "heat_lost_j",
        "substeps",
        "runaway",
    ] {
        let sentence = format!("Physical Violation: '{key}' is not a field of this request.");
        let mut step = good.clone();
        step["parameters"][key] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, sentence);
        let mut in_load = run_body(300.0, FRESH, 1.0, &[(3, oven(400.0))], 1);
        in_load["segments"][0]["load"][key] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, sentence);
    }
}

const KERNEL_SOURCE: &str = include_str!("../../genesis_kernel/src/runaway_18650_lco.rs");

/// `(variant, doc sentence)` for every variant of the kernel's `StepReject`.
fn documented_rejects(source: &str) -> Vec<(String, String)> {
    let start = source.find("pub enum StepReject {").expect("StepReject is in the card");
    let mut documented = Vec::new();
    let mut doc = String::new();
    for line in source[start..].lines().skip(1) {
        if line == "}" {
            break;
        }
        let text = line.trim_start();
        if let Some(sentence) = text.strip_prefix("/// ") {
            if !doc.is_empty() {
                doc.push(' ');
            }
            doc.push_str(sentence);
        } else if let Some(name) = text.strip_suffix(',') {
            documented.push((name.to_string(), doc.clone()));
            doc.clear();
        }
    }
    documented
}

/// Every refusal's meaning is the kernel's sentence, word for word: the end,
/// the bounds where their equations hold and the oven tested where it enters
/// among them.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_description_carries_the_kernel_sentence_of_every_refusal() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_card", &json!({ "domain": "energy", "card": "runaway_18650_lco" })).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let documented = documented_rejects(KERNEL_SOURCE);
    assert_eq!(documented.len(), 17);
    for (name, sentence) in &documented {
        let row = body["rejects"].as_array().unwrap().iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(row["meaning"].as_str().unwrap(), sentence, "{name}");
    }
    assert!(body["declared_ends"].to_string().contains("0.0205 J"), "{}", body["declared_ends"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_card_is_listed_and_described_with_its_record() {
    let base = spawn_app().await;
    let list: Value = Client::new().get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let cards = list["cards"].as_array().unwrap();
    let row = cards.iter().find(|row| row["card"] == "runaway_18650_lco").expect("the card is listed");
    assert_eq!(row["domain"], "energy");
    assert!(row["specimen"].as_str().unwrap().contains("Hatchard et al. (2001)"));
    let (status, text) = post(&base, "/describe_card", &json!({ "domain": "energy", "card": "runaway_18650_lco" })).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["phase_fields"][0]["name"], "temperature_k");
    // The kernel's 17 names and the gateway's two, DtBelowFloor and DtAboveCeiling.
    assert_eq!(body["rejects"].as_array().map(Vec::len), Some(19), "{}", body["rejects"]);
    for name in ["EveryReactantConsumed", "SubstepsAboveBudget", "CanMelting", "SpontaneousGeneration", "FirstLawBreach"] {
        assert!(body["rejects"].to_string().contains(name), "{name}");
    }
    assert!(body["declared_ends"].to_string().contains("EveryReactantConsumed"), "{}", body["declared_ends"]);
    assert!(body["validation"]["pass_line"].as_str().unwrap().contains("b704cdf7d0d8bdde297c8ffa7e4b157f4edf633f26890c94e6f4fbfe65a60ad4"));
    assert!(!text.to_lowercase().contains("check"));
}
