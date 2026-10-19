//! The accumulator card, HYDAC SBO450-0.6 on the Darmstadt rig, on the web and
//! the local connection.
//!
//! A step and a run of a push and a pull end on the compiled kernel's state to
//! the bit. A run past the compared 86 bar goes on, flagged; a run that would
//! land above the vessel's rated 330 bar, and a run whose heat passes its
//! share of the work, are refused part way and keep their proof up to the
//! last step that landed. Each refusal a request can reach is named and
//! carries no proof: a mass of zero or less, a pressure above the rating or
//! below the precharge, a gas at or below nitrogen's critical temperature, a
//! wall at or below absolute zero, a sum of work or heat carried into a
//! start below zero or already over its share, a step with more than 1 mm of
//! piston travel, a step outside the card's floor and ceiling, a missing
//! input, a key the card does not have, and a wall temperature given to a
//! segment. The edges of the compared range land with no flag.
//! `validate_series` compares the gas pressure. The local and the web
//! connection return the same bytes.
//!
//! No measurement of the accumulator is read here.

mod answer_text;

use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::accumulator_sbo450::{
    self, AccumulatorLoad, AccumulatorSbo450, CHARGE_20_BAR_KG, CHARGE_40_BAR_KG, GasState,
    StepReject,
};
use genesis_kernel::frame::LastStateFrame64;
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

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
    let client = Client::builder().timeout(Duration::from_secs(120)).build().unwrap();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client.post(&url).header(reqwest::header::CONTENT_TYPE, "application/json").body(answer_text::as_sent(body)).send().await {
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

fn card() -> AccumulatorSbo450 {
    AccumulatorSbo450::on_darmstadt_rig()
}

/// The charge of a 40 bar precharge, kg, and of a 20 bar one.
const CHARGE_40: f64 = CHARGE_40_BAR_KG;
const CHARGE_20: f64 = CHARGE_20_BAR_KG;
const ROOM_K: f64 = 293.55;

fn start(mass: f64, pressure: f64, temperature: f64, wall: f64) -> Value {
    json!({
        "gas_mass_kg": mass,
        "gas_pressure_pa": pressure,
        "gas_temperature_k": temperature,
        "wall_temperature_k": wall,
        "work_moved_j": 0.0,
        "heat_moved_j": 0.0
    })
}

fn step_body(initial: &Value, speed: f64, dt_s: f64) -> Value {
    let mut parameters = initial.as_object().unwrap().clone();
    parameters.insert("piston_speed_m_s".into(), json!(speed));
    parameters.insert("dt_s".into(), json!(dt_s));
    json!({ "domain": "energy", "card": "accumulator_sbo450", "parameters": parameters })
}

fn run_body(initial: &Value, dt_s: f64, segments: Value) -> Value {
    json!({
        "domain": "energy", "card": "accumulator_sbo450",
        "initial": initial, "dt_s": dt_s,
        "segments": segments, "record_every": 1
    })
}

fn kernel_start(initial: &Value) -> GasState {
    let of = |key: &str| initial[key].as_f64().unwrap();
    GasState::try_start(
        &card(),
        of("gas_mass_kg"),
        of("gas_pressure_pa"),
        of("gas_temperature_k"),
        of("wall_temperature_k"),
        of("work_moved_j"),
        of("heat_moved_j"),
    )
    .unwrap()
}

fn blank_frame() -> LastStateFrame64 {
    LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof: [0; 16],
    }
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// Every number of the published state is the kernel state's, to the bit.
fn assert_state_is(published: &Value, state: &GasState, label: &str) {
    for (name, value) in [
        ("t_s", state.t_s),
        ("gas_mass_kg", state.gas_mass_kg),
        ("wall_temperature_k", state.wall_temperature_k),
        ("gas_temperature_k", state.gas_temperature_k),
        ("gas_volume_m3", state.gas_volume_m3),
        ("gas_pressure_pa", state.gas_pressure_pa),
        ("gas_force_n", state.gas_force_n),
        ("work_on_gas_j", state.work_on_gas_j),
        ("heat_to_wall_j", state.heat_to_wall_j),
        ("start_energy_j", state.start_energy_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
        ("work_moved_j", state.work_moved_j),
        ("heat_moved_j", state.heat_moved_j),
        ("heat_share", state.heat_share),
    ] {
        assert_eq!(published[format!("{name}_hex")], hex_of(value), "{label} {name}");
    }
    assert!(published.get("pressure_outside_fitted").is_none(), "{label}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_accumulator_is_the_compiled_step_to_the_bit() {
    let base = spawn_app().await;
    let initial = start(CHARGE_40, 6.0e6, 293.15, ROOM_K);
    let (status, text) = post(&base, "/verify_physics", &step_body(&initial, 0.03, 0.0025)).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let mut state = kernel_start(&initial);
    let mut frame = blank_frame();
    accumulator_sbo450::step(&card(), &mut state, &mut frame, AccumulatorLoad { piston_speed_m_s: 0.03 }, 0.0025)
        .unwrap();
    assert_state_is(&answer["state"], &state, "one step");
    // 0.075 mm of travel at 60 bar raises the pressure by about 0.034 bar.
    let pressure = answer["state"]["gas_pressure_pa"].as_f64().unwrap();
    assert!(pressure > 6.003e6 && pressure < 6.004e6, "{pressure}");
    assert!(answer["sha256_seal"].as_str().is_some_and(|seal| seal.len() == 64));
    assert!(answer["receipt_sha256"].as_str().is_some_and(|seal| seal.len() == 64));
    assert_eq!(answer["engine_version"], "genesis_kernel_v0.12.0");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_push_and_a_pull_end_on_the_kernel_state_and_a_run_past_the_ceiling_keeps_its_proof() {
    let base = spawn_app().await;
    let initial = start(CHARGE_40, 8.0e6, ROOM_K, ROOM_K);
    let speeds = [(20_usize, 0.2), (40, -0.2), (20, 0.2)];
    let segments: Vec<Value> = speeds
        .iter()
        .map(|(steps, speed)| json!({ "steps": steps, "load": { "piston_speed_m_s": speed } }))
        .collect();
    let (status, text) = post(&base, "/run_trajectory", &run_body(&initial, 0.001, json!(segments))).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 80);
    let mut state = kernel_start(&initial);
    let mut frame = blank_frame();
    for (steps, speed) in speeds {
        for _ in 0..steps {
            accumulator_sbo450::step(&card(), &mut state, &mut frame, AccumulatorLoad { piston_speed_m_s: speed }, 0.001)
                .unwrap();
        }
    }
    assert_state_is(&tape["final_state"], &state, "a push, a pull and a push");
    // The piston is back where it started, and the swing gave heat to the wall.
    assert!(state.heat_to_wall_j != 0.0 && state.work_on_gas_j > 0.0);
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
    // No frame carries a flag: 80 bar is outside the fitted pressures and inside the card.
    for recorded in tape["frames"].as_array().unwrap() {
        let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
        let flags = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
        assert_eq!(flags, 0);
    }

    // From 85.5 bar a steady push passes 86 bar, the compared: the run goes
    // on, each frame past it flagged, and its tape holds.
    let high = start(CHARGE_40, 8.55e6, ROOM_K, ROOM_K);
    let push = json!([{ "steps": 200, "load": { "piston_speed_m_s": 0.1 } }]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(&high, 0.001, push)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{text}");
    assert!(tape["final_state"]["gas_pressure_pa"].as_f64().unwrap() > 8.6e6);
    let last = tape["frames"].as_array().unwrap().last().unwrap();
    let bytes = hex::decode(last["frame_hex"].as_str().unwrap()).unwrap();
    assert_eq!(u64::from_le_bytes(bytes[40..48].try_into().unwrap()), accumulator_sbo450::FLAG_OUTSIDE_COMPARED_RUNS);
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
    // From 325 bar a push passes the vessel's rated 330 bar: the run is
    // refused there and keeps its proof up to the last step that landed.
    let rated = start(CHARGE_40, 3.25e7, ROOM_K, ROOM_K);
    let push = json!([{ "steps": 200, "load": { "piston_speed_m_s": 0.1 } }]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(&rated, 0.001, push)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "refused");
    assert_eq!(tape["reject"], format!("{:?}", StepReject::PressureAboveRated));
    let landed = tape["steps_landed"].as_u64().unwrap();
    assert!(landed > 0 && landed < 200, "{landed}");
    assert!(tape["reject_detail"].as_str().unwrap().contains("PressureAboveRated"));
    assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 64));
    assert!(tape["final_state"]["gas_pressure_pa"].as_f64().unwrap() <= 3.3e7);
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
}

/// The speed the scoring holds over step `k` of a sine from the piston's
/// outermost point, 400 steps to a period.
fn sine_segments(frequency_hz: f64, amplitude_m: f64, steps: usize) -> (f64, Value) {
    let dt = 1.0 / (frequency_hz * 400.0);
    let w = 2.0 * std::f64::consts::PI * frequency_hz;
    let segments: Vec<Value> = (0..steps)
        .map(|k| {
            let speed = -amplitude_m * ((w * (k + 1) as f64 * dt).cos() - (w * k as f64 * dt).cos()) / dt;
            json!({ "steps": 1, "load": { "piston_speed_m_s": speed } })
        })
        .collect();
    (dt, json!(segments))
}

/// Two measured runs of the record at 20 bar precharge and 60 bar, 5 mm, as
/// the scoring asks them: at 0.2 Hz two periods land with the heat under its
/// share of the work; at 0.1 Hz the run is refused by name part way, sealed,
/// and its proof holds up to the last step that landed. A rest with the gas
/// one kelvin over the wall is refused in the same way, and a rest at the
/// wall's temperature lands every step.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_whose_heat_passes_its_share_of_the_work_is_refused_and_keeps_its_proof() {
    let base = spawn_app().await;
    let quick = start(CHARGE_20, 5_562_310.731_761_613, 287.254_261_423_625, 293.552_334_697_530_4);
    let (dt, segments) = sine_segments(0.2, 0.004_989_864_801_862_285, 800);
    let (status, text) = post(&base, "/run_trajectory", &run_body(&quick, dt, segments)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 800);
    let share = tape["final_state"]["heat_share"].as_f64().unwrap();
    assert!(share > 0.045 && share < 0.056, "{share}");

    let slow = start(CHARGE_20, 5_566_029.968_360_627, 287.285_217_526_049_34, 293.551_841_507_571_4);
    let (dt, segments) = sine_segments(0.1, 0.004_991_137_886_577_627, 800);
    let rest_warm = (
        start(CHARGE_40, 6.0e6, ROOM_K + 1.0, ROOM_K),
        0.25,
        json!([{ "steps": 400, "load": { "piston_speed_m_s": 0.0 } }]),
    );
    for (initial, dt, segments) in [(slow, dt, segments), rest_warm] {
        let (status, text) = post(&base, "/run_trajectory", &run_body(&initial, dt, segments.clone())).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "refused");
        assert_eq!(tape["reject"], format!("{:?}", StepReject::HeatShareAboveCard));
        assert!(tape["reject_detail"].as_str().unwrap().contains("HeatShareAboveCard"));
        let landed = tape["steps_landed"].as_u64().unwrap() as usize;
        assert!(landed > 5 && landed < 400, "{landed}");
        assert_eq!(tape["frames"].as_array().unwrap().len(), landed);
        assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 64));
        // The state is the kernel's at the last step that landed, and the next step is the one refused.
        let mut state = kernel_start(&initial);
        let mut frame = blank_frame();
        let speeds: Vec<f64> = segments
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|segment| {
                let steps = segment["steps"].as_u64().unwrap() as usize;
                std::iter::repeat(segment["load"]["piston_speed_m_s"].as_f64().unwrap()).take(steps)
            })
            .collect();
        for &speed in &speeds[..landed] {
            accumulator_sbo450::step(&card(), &mut state, &mut frame, AccumulatorLoad { piston_speed_m_s: speed }, dt)
                .unwrap();
        }
        assert_state_is(&tape["final_state"], &state, "the last step that landed");
        assert!(state.heat_share <= accumulator_sbo450::HEAT_SHARE_LIMIT);
        assert_eq!(
            accumulator_sbo450::step(&card(), &mut state, &mut frame, AccumulatorLoad { piston_speed_m_s: speeds[landed] }, dt),
            Err(StepReject::HeatShareAboveCard)
        );
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_limit_a_request_can_reach_is_named_and_carries_no_proof() {
    let base = spawn_app().await;
    let good = start(CHARGE_40, 6.0e6, ROOM_K, ROOM_K);
    let with = |key: &str, value: f64| {
        let mut changed = good.clone();
        changed[key] = json!(value);
        changed
    };
    let floor = card().floor_dt_s().unwrap();
    let ceiling = card().gate_dt_s().unwrap();
    // Through engine 0.9.0 each of these was a refusal at an edge of the
    // compared range; now each is answered, with the flag.
    for body in [
        step_body(&with("gas_mass_kg", 0.013638), 0.01, 0.01),
        step_body(&with("gas_mass_kg", 0.027496), 0.01, 0.01),
        step_body(&with("gas_mass_kg", 0.02), 0.01, 0.01),
        step_body(&with("gas_mass_kg", 0.02688), 0.01, 0.01),
        step_body(&with("gas_pressure_pa", 8.601e6), 0.01, 0.01),
        step_body(&with("gas_pressure_pa", 1.2e7), 0.01, 0.01),
        step_body(&with("gas_pressure_pa", 4.5e6), 0.01, 0.01),
        step_body(&with("gas_temperature_k", 283.15), 0.0, 0.01),
        step_body(&with("gas_temperature_k", 303.15), 0.0, 0.01),
        step_body(&with("wall_temperature_k", 288.15), 0.0, 0.01),
        step_body(&with("wall_temperature_k", 298.15), 0.0, 0.01),
        step_body(&good, 0.8, 1.0e-4),
        step_body(&good, -0.75, 1.0e-4),
        step_body(&start(CHARGE_20, 8.0e6, ROOM_K, ROOM_K), 0.01, 0.01),
    ] {
        let (status, text) = post(&base, "/verify_physics", &body).await;
        assert_eq!(status, 200, "{body}: {text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        let bytes = hex::decode(answer["frame_hex"].as_str().unwrap_or_default()).unwrap_or_default();
        if bytes.len() == 64 {
            assert_eq!(u64::from_le_bytes(bytes[40..48].try_into().unwrap()), accumulator_sbo450::FLAG_OUTSIDE_COMPARED_RUNS, "{body}");
        }
    }
    let cases: Vec<(Value, &str)> = vec![
        (step_body(&with("gas_mass_kg", 0.0), 0.01, 0.01), "GasMassNotPositive"),
        (step_body(&with("gas_mass_kg", -0.01), 0.01, 0.01), "GasMassNotPositive"),
        (step_body(&with("gas_pressure_pa", 3.4e7), 0.01, 0.01), "PressureAboveRated"),
        (step_body(&with("gas_pressure_pa", 0.0), 0.01, 0.01), "OilSideEmpty"),
        (step_body(&with("gas_pressure_pa", 3.69e6), 0.01, 0.01), "OilSideEmpty"),
        (step_body(&with("gas_temperature_k", 126.0), 0.01, 0.01), "GasBelowCriticalTemperature"),
        (step_body(&with("wall_temperature_k", 0.0), 0.01, 0.01), "WallTemperatureNotPositive"),
        // The sums carried into a start.
        (step_body(&with("work_moved_j", -1.0), 0.01, 0.01), "NegativeMovedSum"),
        (step_body(&with("heat_moved_j", -1.0), 0.01, 0.01), "NegativeMovedSum"),
        (step_body(&with("heat_moved_j", 6.6), 0.01, 0.01), "HeatShareAboveCard"),
        // Inside the speed range and the clock: more than 1 mm in the step.
        (step_body(&good, 0.5, 0.0021), "PistonTravelAboveStep"),
        // A start under the rated pressure whose first step lands above it.
        (step_body(&with("gas_pressure_pa", 3.2999e7), 0.7, 0.00125), "PressureAboveRated"),
        (step_body(&good, 0.0, ceiling * 1.01), "dt_s"),
        (step_body(&good, 0.0, floor / 2.0), "dt_s"),
        (step_body(&good, 0.0, 0.0), "dt_s"),
    ];
    for (body, name) in &cases {
        let (status, text) = post(&base, "/verify_physics", body).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.contains(name), "{name}: {text}");
        assert!(!text.contains("sha256_seal"), "{name}: {text}");
    }
    // The exact ends of each range land, and so does 1 mm of travel stated three ways.
    let low = start(CHARGE_20, 3.7e6, ROOM_K, ROOM_K);
    for body in [
        step_body(&low, 0.0, 0.01),
        step_body(&with("gas_pressure_pa", 8.6e6), 0.0, 0.01),
        step_body(&start(CHARGE_20, 4.3e6, ROOM_K, ROOM_K), 0.0, 0.01),
        step_body(&start(CHARGE_20, 5.5e6, ROOM_K, ROOM_K), 0.0, 0.01),
        step_body(&start(CHARGE_20, 6.7e6, ROOM_K, ROOM_K), 0.0, 0.01),
        step_body(&with("gas_pressure_pa", 5.7e6), 0.0, 0.01),
        step_body(&with("gas_pressure_pa", 6.4e6), 0.0, 0.01),
        step_body(&with("gas_pressure_pa", 7.5e6), 0.0, 0.01),
        step_body(&with("gas_mass_kg", 0.02744), 0.01, 0.01),
        step_body(&with("gas_mass_kg", 0.027495), 0.01, 0.01),
        step_body(&with("gas_mass_kg", 0.02747), 0.01, 0.01),
        step_body(&start(0.013639, 3.7e6, ROOM_K, ROOM_K), 0.01, 0.01),
        step_body(&start(0.013667, 3.7e6, ROOM_K, ROOM_K), 0.01, 0.01),
        step_body(&start(0.01365, 6.0e6, ROOM_K, ROOM_K), 0.01, 0.01),
        step_body(&with("gas_temperature_k", 285.9), 0.0, 0.01),
        step_body(&with("gas_temperature_k", 301.8), 0.0, 0.01),
        step_body(&with("wall_temperature_k", 293.4), 0.0, 0.01),
        step_body(&with("wall_temperature_k", 293.7), 0.0, 0.01),
        step_body(&good, 0.7, 1.0e-4),
        step_body(&good, -0.7, 1.0e-4),
        step_body(&good, 0.5, 0.002),
        step_body(&good, 0.25, 0.004),
        step_body(&good, -0.1, 0.01),
        step_body(&good, 0.0, floor),
        step_body(&good, 0.0, ceiling),
    ] {
        let (status, text) = post(&base, "/verify_physics", &body).await;
        assert_eq!(status, 200, "{body}: {text}");
    }
    // A missing input, a constant of the card and an unknown key are refused.
    // The largest heat share the card takes at a start is a start that carries
    // a run on: the card takes it, and without from the door refuses it by name.
    let (status, text) = post(&base, "/verify_physics", &step_body(&with("heat_moved_j", 6.5), 0.0, 0.01)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    for key in ["wall_temperature_k", "work_moved_j", "heat_moved_j"] {
        let mut missing = step_body(&good, 0.01, 0.01);
        missing["parameters"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/verify_physics", &missing).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(key) && !text.contains("sha256_seal"), "{key}: {text}");
    }
    for key in ["thermal_time_constant_s", "vessel_gas_volume_m3", "piston_area_m2", "gas_volume_m3"] {
        let mut extra = step_body(&good, 0.01, 0.01);
        extra["parameters"][key] = json!(0.9);
        let (status, text) = post(&base, "/verify_physics", &extra).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(key) && !text.contains("sha256_seal"), "{key}: {text}");
    }
    // A speed past the compared in a later segment is stepped, within the millimetre a step.
    let segments = json!([
        { "steps": 1, "load": { "piston_speed_m_s": 0.01 } },
        { "steps": 1, "load": { "piston_speed_m_s": 0.9 } }
    ]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(&good, 1.0e-4, segments)).await;
    assert_eq!(status, 200, "{text}");
    assert!(text.contains("run_seal"), "{text}");
    // The wall temperature is part of the start: a segment that names it is refused by name.
    let segments = json!([
        { "steps": 1, "load": { "piston_speed_m_s": 0.01 } },
        { "steps": 1, "load": { "piston_speed_m_s": 0.01, "wall_temperature_k": 296.0 } }
    ]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(&good, 1.0e-4, segments)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("wall_temperature_k") && !text.contains("run_seal"), "{text}");
}

/// A run that ends exactly on the pressure ceiling keeps its last step: gas
/// still at the wall's temperature at 86 bar stays there for a thousand
/// steps of the largest step, and so does gas at the floor of 37 bar. Neither
/// moves any heat, so neither is refused for its share of heat.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_resting_exactly_on_a_limit_lands_every_step() {
    let base = spawn_app().await;
    for initial in [start(CHARGE_40, 8.6e6, ROOM_K, ROOM_K), start(CHARGE_20, 3.7e6, ROOM_K, ROOM_K)] {
        let rest = json!([{ "steps": 1000, "load": { "piston_speed_m_s": 0.0 } }]);
        let mut body = run_body(&initial, card().gate_dt_s().unwrap(), rest);
        body["record_every"] = json!(1000);
        let (status, text) = post(&base, "/run_trajectory", &body).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
        assert_eq!(tape["steps_landed"], 1000);
        assert_eq!(tape["final_state"]["heat_moved_j"].as_f64(), Some(0.0));
    }
}

/// The heat share is weighed over the whole motion. A slow push of the 20 bar
/// charge from 50 bar is now refused at its start: that charge was never
/// measured at 50 bar. The same push from 60 bar is refused part way as one
/// run. Its returned state with the returned sums is a start that carries a
/// run on: without from it is refused as ContinuationWithoutUpstream, run and
/// single step, and with from as UpstreamOutcomeNotCarried, since a refused
/// run is not continued. With the sums at zero it is a fresh start and the
/// step lands, which is why a start must state them and why stated sums must
/// come from a sealed run.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_heat_share_travels_with_a_carried_on_run_and_a_single_step() {
    let base = spawn_app().await;
    // A wall at 293.15 K and 50 bar on the 20 bar charge were never compared:
    // each run is answered, with the flag, and sealed.
    let push = json!([{ "steps": 30, "load": { "piston_speed_m_s": 0.0005 } }]);
    for audit in [start(CHARGE_20, 5.0e6, 293.15, 293.15), start(CHARGE_20, 5.0e6, 293.5, 293.5)] {
        let (status, text) = post(&base, "/run_trajectory", &run_body(&audit, 0.35, push.clone())).await;
        assert_eq!(status, 200, "{text}");
        assert!(text.contains("run_seal"), "{text}");
    }

    let initial = start(CHARGE_20, 6.0e6, 293.5, 293.5);
    let (status, text) = post(&base, "/run_trajectory", &run_body(&initial, 0.35, push)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "refused");
    assert_eq!(tape["reject"], "HeatShareAboveCard");
    let landed = tape["steps_landed"].as_u64().unwrap();
    assert!(landed > 5 && landed < 30, "{landed}");
    let last = &tape["final_state"];
    let carried = json!({
        "gas_mass_kg": last["gas_mass_kg"],
        "gas_pressure_pa": last["gas_pressure_pa"],
        "gas_temperature_k": last["gas_temperature_k"],
        "wall_temperature_k": last["wall_temperature_k"],
        "work_moved_j": last["work_moved_j"],
        "heat_moved_j": last["heat_moved_j"]
    });
    let rest = json!([{ "steps": 8, "load": { "piston_speed_m_s": 0.0005 } }]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(&carried, 0.35, rest.clone())).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream).") && !text.contains("run_seal"), "{text}");
    let upstream = run_body(&initial, 0.35, json!([{ "steps": 30, "load": { "piston_speed_m_s": 0.0005 } }]));
    let mut chained = run_body(&carried, 0.35, rest.clone());
    chained["from"] = json!({ "request": upstream, "run_seal": tape["run_seal"] });
    let (status, text) = post(&base, "/run_trajectory", &chained).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("UpstreamOutcomeNotCarried") && !text.contains("run_seal\":"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(&carried, 0.0005, 0.35)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("ContinuationWithoutUpstream") && !text.contains("sha256_seal"), "{text}");
    // With the sums dropped the refused step would land: the sums are what carry the limit.
    let mut dropped = carried.clone();
    dropped["work_moved_j"] = json!(0.0);
    dropped["heat_moved_j"] = json!(0.0);
    let (status, text) = post(&base, "/verify_physics", &step_body(&dropped, 0.0005, 0.35)).await;
    assert_eq!(status, 200, "{text}");

    // Where the share does not bind, a run cut in two and carried on ends where the run in one piece ends.
    let swing = |speeds: &[(usize, f64)]| -> Value {
        json!(speeds.iter().map(|(steps, speed)| json!({ "steps": steps, "load": { "piston_speed_m_s": speed } })).collect::<Vec<_>>())
    };
    let all = [(25_usize, 0.01), (50, -0.01), (50, 0.01), (25, -0.01)];
    let (_, whole) = post(&base, "/run_trajectory", &run_body(&initial, 0.01, swing(&all))).await;
    let whole: Value = serde_json::from_str(&whole).unwrap();
    let (_, first) = post(&base, "/run_trajectory", &run_body(&initial, 0.01, swing(&all[..2]))).await;
    let first: Value = serde_json::from_str(&first).unwrap();
    let end = &first["final_state"];
    let next = json!({
        "gas_mass_kg": end["gas_mass_kg"], "gas_pressure_pa": end["gas_pressure_pa"],
        "gas_temperature_k": end["gas_temperature_k"], "wall_temperature_k": end["wall_temperature_k"],
        "work_moved_j": end["work_moved_j"], "heat_moved_j": end["heat_moved_j"]
    });
    // The second half carries the sums on, so it names the first half in from.
    let mut carried_on = run_body(&next, 0.01, swing(&all[2..]));
    carried_on["from"] = json!({ "request": run_body(&initial, 0.01, swing(&all[..2])), "run_seal": first["run_seal"] });
    let (_, second) = post(&base, "/run_trajectory", &carried_on).await;
    let second: Value = serde_json::from_str(&second).unwrap_or_else(|_| panic!("{second}"));
    for tape in [&whole, &first, &second] {
        assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    }
    let close = |name: &str, within: f64| {
        let a = second["final_state"][name].as_f64().unwrap();
        let b = whole["final_state"][name].as_f64().unwrap();
        assert!(((a - b) / b).abs() < within, "{name}: {a} {b}");
    };
    close("gas_temperature_k", 1.0e-13);
    close("gas_pressure_pa", 1.0e-12);
    close("work_moved_j", 1.0e-12);
    close("heat_moved_j", 1.0e-9);
    close("heat_share", 1.0e-9);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_the_gas_pressure_and_carries_the_run_seal() {
    let base = spawn_app().await;
    let initial = start(CHARGE_40, 6.0e6, ROOM_K, ROOM_K);
    let segments = json!([{ "steps": 4, "load": { "piston_speed_m_s": 0.05 } }]);
    let run = run_body(&initial, 0.002, segments);
    let (status, alone) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{alone}");
    let alone: Value = serde_json::from_str(&alone).unwrap();
    let pressure = alone["final_state"]["gas_pressure_pa"].as_f64().unwrap();
    let body = json!({
        "run": run, "quantity": "gas_pressure_pa",
        "measurements": [{ "step": 4, "value": pressure + 1000.0 }]
    });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert!((answer["rms_error"].as_f64().unwrap() - 1000.0).abs() < 1.0e-6, "{text}");
    assert_eq!(answer["compared"], 1);
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

    async fn tool(&mut self, name: &str, arguments: &Value) -> (String, bool) {
        let line = answer_text::tool_call(7, name, arguments);
        let answer: Value = serde_json::from_str(&self.raw(&line.to_string()).await).unwrap();
        (
            answer["result"]["content"][0]["text"].as_str().unwrap().to_string(),
            answer["result"]["isError"] == true,
        )
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_local_connection_lists_the_accumulator_and_gives_the_bytes_of_the_web_connection() {
    let web = spawn_app().await;
    let mut local = Local::start().await;
    let (listed, error) = local.tool("list_cards", &json!({})).await;
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 40);
    let entry = cards
        .iter()
        .find(|card| card["card"] == "accumulator_sbo450")
        .expect("the accumulator is listed");
    assert_eq!(entry["domain"], "energy");
    // The line an agent sees in the list carries the standing: what is fitted and what was compared.
    let line = entry["specimen"].as_str().unwrap();
    assert!(line.contains("Fitted on the 100 runs from 0.01 to 10 Hz of the two records at 40 bar precharge and 60 bar load"), "{line}");
    assert!(line.contains(STANDING), "{line}");

    let which = json!({ "domain": "energy", "card": "accumulator_sbo450" });
    let (described, error) = local.tool("describe_card", &which).await;
    assert!(!error);
    let (status, over_web) = post(&web, "/describe_card", &which).await;
    assert_eq!(status, 200);
    assert_eq!(described, over_web);
    let description: Value = serde_json::from_str(&described).unwrap();
    let result = description["validation"]["result"].as_str().unwrap();
    assert!(result.starts_with(RESULT_STARTS), "{result}");
    let limits = description["validation"]["limits"].as_str().unwrap();
    for words in [
        "The card's geometry and range were corrected after the comparison, with every run read",
        "the result states the corrected card's numbers on the measured runs",
        "Two constants are fitted to this accumulator's own runs at one condition",
        "The piston area is that of a 50 mm bore, 19.635 cm2, which no source prints",
        "an offset of -0.202 kN",
        "a vessel of 0.58879 l",
        "a step lands if H is at most 0.065 of the larger of M and 100 J, and is refused otherwise",
        "the record at 109 to 132 bar, 40 bar precharge and 120 bar load, reads 2.72 percent median error of the swing (target 2.0) on the card as first compared",
        "Fitted to each record alone the lag is 1.47 to 1.53 ms in all seven",
        "say nothing of the card: every baseline is within them",
        "extrapolation in pressure and in charge",
        "The precharge is absolute",
        "gives 2.56 percent in the worst record (target 2.0), 6.73 percent at most (5.0) and 4.03 degrees at most (3.0)",
        "falls threefold",
        "Newton's law is incapable of representing this physical phenomenon",
        "preprint 2012.06526",
        "The 2.72 percent at 40 / 120 bar does not come from the equation of state",
        "one series compliance",
        "with that removed through the model it is 0.35 percent",
        "0.72 percent",
        "for piston accumulators",
        "The ledger closes for any time constant and any vessel volume",
        "no orifice, no flow of gas, and choked flow does not arise",
        "the mean pressure, which is taken from each run",
        "A second program, by another party, recomputes it from the protocol and gives the same numbers",
        "no second program has recomputed them",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    for words in [
        "The card answers 282 runs and refuses three",
        "0.93 percent in the median (target 2.0)",
        "2.72 at 40 / 120 (target 2.0)",
        "13.96 percent (target 5.0)",
        "its median lead error, 0.27 degrees, is above three baselines' 0.12, 0.15 and 0.25",
        "so that target says nothing of the card's exchange of heat",
        "a3d268ca094ba37d1edc91eb3e46d9748deb365d48b9295453e1fa45a8235428",
        "These numbers are of the card as it was then",
        "verifies every proof and reproduces every number above",
        "89 runs have no answer",
        "59 are no longer answered",
        "92e33dfeaacb2c2fceb0bed273e24424ed13d16fa68592c107e70c3171fbe56e",
        "ba073e2a1c070aa1282bcea8b330b765a833c53b0ab63367a61fb46722a68bb7",
        AFTER_THE_FACT,
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    for words in [
        "within 0.93 percent median (target 2.0)",
        "37 to 43 and 55 to 67 bar (20 bar) and 57 to 64 and 75 to 86 bar (40 bar), wall 293.4 to 293.7 K, gas 285.9 to 301.8 K, piston to 0.7 m/s",
        "Gap: 2.72 percent median at 109 to 132 bar (target 2.0)",
        "up to the rated 330 bar",
        "0.01 to 20 Hz",
        AFTER_THE_FACT_LINE,
    ] {
        assert!(line.contains(words), "{words}: {line}");
    }
    assert!(description["advisory"].as_array().is_none_or(Vec::is_empty), "{}", description["advisory"]);
    // After the kernel's 14 names the list holds the gateway's own three for this card, DtBelowFloor and DtAboveCeiling and ContinuationWithoutUpstream.
    assert_eq!(description["rejects"].as_array().map(Vec::len), Some(17), "{}", description["rejects"]);

    // The worked examples of the description run as written, the same bytes on both.
    let step = description["example_compute_step"].clone();
    let (over_local, error) = local.tool("verify_step", &step).await;
    assert!(!error, "{over_local}");
    let (status, over_web) = post(&web, "/verify_physics", &step).await;
    assert_eq!(status, 200, "{over_web}");
    assert_eq!(over_local, over_web);
    let run = description["example_run_trajectory"].clone();
    let (over_local, error) = local.tool("run_trajectory", &run).await;
    assert!(!error, "{over_local}");
    let (status, over_web) = post(&web, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{over_web}");
    assert_eq!(over_local, over_web);
    let tape: Value = serde_json::from_str(&over_local).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], 40);

    // A refusal is the same sentence on both, marked as an error on the local connection.
    let mut off = step.clone();
    off["parameters"]["gas_temperature_k"] = json!(100.0);
    let (over_local, error) = local.tool("verify_step", &off).await;
    assert!(error);
    let (status, over_web) = post(&web, "/verify_physics", &off).await;
    assert_eq!(status, 422);
    assert!(over_local.contains("GasBelowCriticalTemperature") && over_web.contains("GasBelowCriticalTemperature"));
}

/// The standing in the one line of `list_cards`.
const STANDING: &str = "within 0.49 percent median, 4.05 at most (target 5.0)";
/// How the result of the validation record begins.
const RESULT_STARTS: &str = "On the 285 runs outside the fit, run once through the engine.";
/// What the result says of the run after the fact.
const AFTER_THE_FACT: &str = "of the 285 runs it answers 196 and refuses 89, the 57 runs at 40 / 120 bar as PressureOutsideCard before the first step and the 32 runs at 0.1 Hz and under of the other four records as HeatShareAboveCard";
/// What the one line of `list_cards` says of it.
const AFTER_THE_FACT_LINE: &str = "on the 196 runs of the served range, drawn afterwards";
