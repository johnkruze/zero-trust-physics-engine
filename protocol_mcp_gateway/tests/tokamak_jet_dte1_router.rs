//! The JET tokamak card on the HTTP door.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes. A run of three segments under three loads carries the kernel
//! loop's last state to the bit, and its tape verifies. Each limit of physics
//! and of a field is refused by name at the door: the published bounds at
//! zero, a tritium fraction above 1, the Greenwald density of the segment's
//! current, a safety factor under 2, and at the start the stores, the span of
//! the reactivity fit, the Troyon limit and the ceiling of the start's own
//! state. Past each old wall of the compared envelope a request is answered
//! and carries the kernel loop to the bit. A run that reaches a limit of its
//! state part way is sealed with that refusal on its last landed step. The
//! clock is the caller's and has no end: a run continued at the returned t_s
//! is the straight run past 5.475 s, and a start without it is refused. A
//! series of the run's own values compares to exactly zero.
//!
//! The loads here are round numbers. No pulse of the research record is run.

mod answer_text;

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::tokamak_jet_dte1::{
    self, StepReject, TokamakJetDte1, TokamakLoad, TokamakState,
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

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
    let url = format!("{base}{path}");
    let text = answer_text::as_sent(body);
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(text.clone())
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

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
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

fn tokamak() -> TokamakJetDte1 {
    TokamakJetDte1::elmy_h_mode()
}

fn load(heating_w: f64, ohmic_w: f64, density: f64, current_a: f64, field_t: f64, tritium: f64) -> TokamakLoad {
    TokamakLoad {
        heating_power_w: heating_w,
        ohmic_power_w: ohmic_w,
        line_density_per_m3: density,
        plasma_current_a: current_a,
        toroidal_field_t: field_t,
        tritium_fraction: tritium,
    }
}

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn load_json(load: &TokamakLoad) -> Value {
    json!({
        "heating_power_w": load.heating_power_w,
        "ohmic_power_w": load.ohmic_power_w,
        "line_density_per_m3": load.line_density_per_m3,
        "plasma_current_a": load.plasma_current_a,
        "toroidal_field_t": load.toroidal_field_t,
        "tritium_fraction": load.tritium_fraction,
    })
}

fn even_mix() -> TokamakLoad {
    TokamakLoad {
        heating_power_w: 20.0e6,
        ohmic_power_w: 0.5e6,
        line_density_per_m3: 6.0e19,
        plasma_current_a: 3.8e6,
        toroidal_field_t: 4.0,
        tritium_fraction: 0.5,
    }
}

fn step_body(thermal_j: f64, fast_j: f64, load: &TokamakLoad, dt_s: f64) -> Value {
    step_body_at(0.0, thermal_j, fast_j, load, dt_s)
}

fn step_body_at(elapsed_s: f64, thermal_j: f64, fast_j: f64, load: &TokamakLoad, dt_s: f64) -> Value {
    let mut parameters = json!({ "thermal_energy_j": thermal_j, "fast_ion_energy_j": fast_j, "elapsed_time_s": elapsed_s });
    for (key, value) in load_json(load).as_object().unwrap() {
        parameters[key] = value.clone();
    }
    parameters["dt_s"] = json!(dt_s);
    json!({ "domain": "energy", "card": "tokamak_jet_dte1", "parameters": parameters })
}

fn run_body(thermal_j: f64, fast_j: f64, segments: &[(u64, TokamakLoad)], dt_s: f64, record_every: u64) -> Value {
    run_body_at(0.0, thermal_j, fast_j, segments, dt_s, record_every)
}

fn run_body_at(elapsed_s: f64, thermal_j: f64, fast_j: f64, segments: &[(u64, TokamakLoad)], dt_s: f64, record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "tokamak_jet_dte1",
        "initial": { "thermal_energy_j": thermal_j, "fast_ion_energy_j": fast_j, "elapsed_time_s": elapsed_s },
        "dt_s": dt_s,
        "segments": segments.iter().map(|(steps, load)| json!({ "steps": steps, "load": load_json(load) })).collect::<Vec<_>>(),
        "record_every": record_every,
    })
}

fn state_numbers(state: &TokamakState) -> Vec<(&'static str, f64)> {
    vec![
        ("t_s", state.t_s),
        ("thermal_energy_j", state.thermal_energy_j),
        ("fast_ion_energy_j", state.fast_ion_energy_j),
        ("fusion_energy_j", state.fusion_energy_j),
        ("temperature_kev", state.temperature_kev),
        ("loss_power_w", state.loss_power_w),
        ("confinement_time_s", state.confinement_time_s),
        ("slowing_time_s", state.slowing_time_s),
        ("fusion_power_w", state.fusion_power_w),
        ("fusion_power_thermal_w", state.fusion_power_thermal_w),
        ("beta_normalised", state.beta_normalised),
        ("greenwald_fraction", state.greenwald_fraction),
        ("safety_factor", state.safety_factor),
        ("energy_anchor_j", state.energy_anchor_j),
        ("energy_supplied_j", state.energy_supplied_j),
        ("energy_lost_j", state.energy_lost_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
    ]
}

fn assert_state_is(published: &Value, state: &TokamakState, label: &str) {
    for (name, value) in state_numbers(state) {
        assert_eq!(
            published[format!("{name}_hex")].as_str().unwrap_or_else(|| panic!("{label} {name}")),
            format!("{:016x}", value.to_bits()),
            "{label} {name}"
        );
    }
    assert_eq!(published["near_density_limit"], state.near_density_limit, "{label}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_tokamak_matches_the_compiled_step_with_the_same_proof_bytes() {
    let base = spawn_app().await;
    let card = tokamak();
    let deuterium = load(10.0e6, 1.0e6, 4.0e19, 2.5e6, 2.6, 0.0);
    let dense = TokamakLoad { line_density_per_m3: tokamak_jet_dte1::LINE_DENSITY_COMPARED_MAX_PER_M3, ..even_mix() };
    let cases = [
        (0.0, 4.0e6, 0.5e6, even_mix(), tokamak_jet_dte1::GATE_DT_S),
        (0.0, 3.0e6, 0.5e6, deuterium, tokamak_jet_dte1::FLOOR_DT_S),
        (0.0, 8.0e6, 1.0e6, dense, 2.5e-4),
    ];
    // Stated 2.5 s into the window, the step carries a pulse on: the card
    // takes it, and without from the door refuses it by name.
    let (status, answer) = post(&base, "/verify_physics", &step_body_at(2.5, 4.0e6, 0.5e6, &even_mix(), tokamak_jet_dte1::GATE_DT_S)).await;
    assert_eq!(status, 422, "{answer}");
    assert!(answer.ends_with("(ContinuationWithoutUpstream)."), "{answer}");
    let mut seals = Vec::new();
    for (elapsed, thermal, fast, load, dt_s) in cases {
        let request = step_body_at(elapsed, thermal, fast, &load, dt_s);
        let (status, answer) = post(&base, "/verify_physics", &request).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);
        let mut state = TokamakState::try_continue(&card, elapsed, thermal, fast).unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        tokamak_jet_dte1::step(&card, &mut state, &mut frame, load, dt_s).unwrap();
        assert_state_is(&response.state, &state, "step");
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        assert_eq!(
            response.residual_conservation_error.to_bits(),
            state.residual_conservation_error.to_bits()
        );
        seals.push(response.sha256_seal);
    }
    assert_eq!(seals.len(), 3);
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_of_three_loads_carries_the_kernel_loop_and_its_tape_verifies() {
    let base = spawn_app().await;
    let card = tokamak();
    let segments = [
        (300_u64, load(8.0e6, 1.2e6, 3.0e19, 3.2e6, 3.4, 0.034)),
        (300, load(24.0e6, 0.4e6, 5.0e19, 3.2e6, 3.4, 0.5)),
        (300, load(0.5e6, 2.0e6, 7.5e19, 3.2e6, 3.3, 0.451)),
    ];
    let request = run_body(1.5e6, 0.2e6, &segments, 5.0e-4, 25);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 900);
    assert_eq!(tape["frames"].as_array().unwrap().len(), 36);

    let mut state = TokamakState::try_start(&card, 1.5e6, 0.2e6).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut worst = 0.0_f64;
    for (steps, load) in segments {
        for _ in 0..steps {
            tokamak_jet_dte1::step(&card, &mut state, &mut frame, load, 5.0e-4).unwrap();
            worst = worst.max(state.residual_conservation_error);
        }
    }
    assert_state_is(&tape["final_state"], &state, "run");
    assert_eq!(
        tape["max_residual_conservation_error_hex"],
        format!("{:016x}", worst.to_bits())
    );
    assert!(state.fusion_energy_j > 0.0);
    assert!(worst < tokamak_jet_dte1::RESIDUAL_CONSERVATION_LIMIT);

    let (status, verdict) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{verdict}");
    let verdict: Value = serde_json::from_str(&verdict).unwrap();
    assert_eq!(verdict["holds"], true, "{verdict}");

    // The run's own final value, sent back as a measurement, compares to zero.
    let series = json!({
        "run": request,
        "quantity": "fusion_power_w",
        "measurements": [{ "step": 900, "value": state.fusion_power_w }],
    });
    let (status, answer) = post(&base, "/validate_series", &series).await;
    assert_eq!(status, 200, "{answer}");
    let answer: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(answer["compared"], 1);
    assert_eq!(answer["max_abs_error_hex"], "0000000000000000");
    assert_eq!(answer["run_seal"], tape["run_seal"]);
}


/// The kernel loop of a run of one load at a held step: the state after the
/// last landed step, the steps landed, the worst residual, and the refusal
/// that stopped it, if any.
fn kernel_run(thermal_j: f64, fast_j: f64, load: TokamakLoad, dt_s: f64, steps: u64) -> (TokamakState, u64, f64, Option<StepReject>) {
    let card = tokamak();
    let mut state = TokamakState::try_start(&card, thermal_j, fast_j).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut worst = 0.0_f64;
    for landed in 0..steps {
        match tokamak_jet_dte1::step(&card, &mut state, &mut frame, load, dt_s) {
            Ok(()) => worst = worst.max(state.residual_conservation_error),
            Err(reject) => return (state, landed, worst, Some(reject)),
        }
    }
    (state, steps, worst, None)
}

/// The step a run of this start holds: 1 ms, or the ceiling of the start's
/// state where that is shorter.
fn held_step(thermal_j: f64, fast_j: f64, load: &TokamakLoad) -> f64 {
    let ceiling = tokamak().state_ceiling_dt_s(thermal_j, fast_j, *load);
    if ceiling < 1.0e-3 { ceiling } else { 1.0e-3 }
}

/// Each limit of physics and each limit of a field is refused by name at the
/// door, before a step: the published bounds at zero, the tritium fraction
/// above 1, the Greenwald density, a safety factor under 2, and at the start
/// the stores, the span of the reactivity fit, the Troyon limit and the
/// ceiling of the start's own state.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_limit_is_refused_by_name_before_a_step() {
    let base = spawn_app().await;
    let card = tokamak();
    let low = load(10.0e6, 1.0e6, 2.0e19, 1.65e6, 1.80, 0.5);
    let wide = load(20.0e6, 1.0e6, 1.65e19, 6.0e6, 6.0, 0.5);
    let cases: Vec<(f64, f64, TokamakLoad, &str)> = vec![
        (4.0e6, 0.5e6, TokamakLoad { heating_power_w: -1.0e-9, ..even_mix() }, "HeatingPowerNegative"),
        (4.0e6, 0.5e6, TokamakLoad { ohmic_power_w: -1.0, ..even_mix() }, "OhmicPowerNegative"),
        (4.0e6, 0.5e6, TokamakLoad { plasma_current_a: 0.0, ..even_mix() }, "PlasmaCurrentNotPositive"),
        (4.0e6, 0.5e6, TokamakLoad { plasma_current_a: -3.8e6, ..even_mix() }, "PlasmaCurrentNotPositive"),
        (4.0e6, 0.5e6, TokamakLoad { toroidal_field_t: 0.0, ..even_mix() }, "ToroidalFieldNotPositive"),
        (4.0e6, 0.5e6, TokamakLoad { tritium_fraction: -1.0e-9, ..even_mix() }, "FuelMixOutsideCard"),
        (4.0e6, 0.5e6, TokamakLoad { tritium_fraction: above(1.0), ..even_mix() }, "FuelMixOutsideCard"),
        (4.0e6, 0.5e6, TokamakLoad { line_density_per_m3: 0.0, ..even_mix() }, "DensityNotPositive"),
        (4.0e6, 0.5e6, TokamakLoad { line_density_per_m3: -1.0, ..even_mix() }, "DensityNotPositive"),
        (4.0e6, 0.5e6, TokamakLoad { line_density_per_m3: above(card.greenwald_density_per_m3(1.65e6)), ..low }, "DensityAboveGreenwald"),
        (4.0e6, 0.5e6, TokamakLoad { line_density_per_m3: above(card.greenwald_density_per_m3(3.8e6)), ..even_mix() }, "DensityAboveGreenwald"),
        (4.0e6, 0.5e6, TokamakLoad { plasma_current_a: 3.84e6, toroidal_field_t: 1.80, ..even_mix() }, "SafetyFactorBelowLimit"),
        // The start, under the first load.
        (0.0, 0.5e6, even_mix(), "EnergyOutsideCard"),
        (4.0e6, -1.0, even_mix(), "EnergyOutsideCard"),
        (0.1e6, 0.0, even_mix(), "TemperatureBelowReactivityFit"),
        // 46 MJ at 1.65e19 per cubic metre is 103 keV at the centre of the profile, under the Troyon limit of 6 MA and 6 T.
        (46.0e6, 0.0, wide, "TemperatureAboveReactivityFit"),
        (20.0e6, 3.0e6, even_mix(), "BetaAboveTroyon"),
        // 2.4 MJ at 1.65e19 per cubic metre, 1.65 MA and 1.80 T: 4.1 keV and a
        // confinement time of 63 ms, whose ceiling is 0.42 ms. 0.6.0 named such
        // states FirstLawBreach; 0.9.0 ConfinementTimeOutsideComparison.
        (2.4e6, 0.1e6, load(24.3e6, 2.44e6, 1.65e19, 1.65e6, 1.80, 0.5), "DtAboveStateCeiling"),
        (0.3 / card.temperature_kev(1.0, 6.0e19), 0.1e6, load(5.0e6, 1.0e6, 6.0e19, 3.8e6, 4.0, 0.5), "DtAboveStateCeiling"),
    ];
    for (thermal, fast, load, name) in cases {
        for (path, request) in [
            ("/verify_physics", step_body(thermal, fast, &load, 1.0e-3)),
            ("/run_trajectory", run_body(thermal, fast, &[(1, load)], 1.0e-3, 1)),
        ] {
            let (status, answer) = post(&base, path, &request).await;
            assert_eq!(status, 422, "{name} {path}: {answer}");
            assert!(answer.contains(&format!("({name})")), "{name} {path}: {answer}");
            assert!(!answer.contains("FirstLawBreach"), "{answer}");
            assert!(!answer.contains("sha256_seal") && !answer.contains("run_seal"), "{answer}");
        }
    }
    // A clock below zero.
    let (status, answer) = post(&base, "/verify_physics", &step_body_at(-1.0e-9, 4.0e6, 0.5e6, &even_mix(), 1.0e-3)).await;
    assert_eq!(status, 422, "{answer}");
    assert!(answer.contains("(NegativeTime)"), "{answer}");
    // A later segment past a limit refuses the whole run.
    for (later, name) in [
        // 7.5e19 per cubic metre is above the Greenwald density of 2.0 MA, 7.23e19.
        (TokamakLoad { line_density_per_m3: 7.5e19, plasma_current_a: 2.0e6, toroidal_field_t: 2.1, ..even_mix() }, "DensityAboveGreenwald"),
        (TokamakLoad { tritium_fraction: 1.5, ..even_mix() }, "FuelMixOutsideCard"),
        (TokamakLoad { toroidal_field_t: 2.0, ..even_mix() }, "SafetyFactorBelowLimit"),
        (TokamakLoad { heating_power_w: -5.0e6, ..even_mix() }, "HeatingPowerNegative"),
    ] {
        let (status, answer) = post(&base, "/run_trajectory", &run_body(4.0e6, 0.5e6, &[(5, even_mix()), (5, later)], 1.0e-3, 1)).await;
        assert_eq!(status, 422, "{answer}");
        assert!(answer.contains(name), "{name}: {answer}");
    }
    // The static step clock.
    for (dt_s, words) in [
        (above(1.0e-3), "the step ceiling of the tokamak_jet_dte1 card"),
        (0.999e-6, "the step floor of the tokamak_jet_dte1 card"),
    ] {
        let (status, answer) = post(&base, "/verify_physics", &step_body(4.0e6, 0.5e6, &even_mix(), dt_s)).await;
        assert_eq!(status, 422);
        assert!(answer.contains(words), "{answer}");
    }
    // A constant of the card sent as a field is not a field.
    let mut request = step_body(4.0e6, 0.5e6, &even_mix(), 1.0e-3);
    request["parameters"]["confinement_multiplier"] = json!(1.5);
    let (status, answer) = post(&base, "/verify_physics", &request).await;
    assert_eq!(status, 422);
    assert!(answer.contains("confinement_multiplier"), "{answer}");
}

/// Past each old wall of the compared envelope a request is answered: a run
/// of 300 steps at 1 ms, or at the ceiling of the start's state where that
/// is shorter, carries the kernel loop to the bit and its tape verifies; and
/// where the ceiling is shorter than 1 ms the step at 1 ms is refused by name
/// at the door.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn past_each_old_wall_a_request_is_answered() {
    let base = spawn_app().await;
    let card = tokamak();
    let field_for = |q: f64, current_a: f64| q * (current_a / 1.0e6) / card.safety_factor(1.0e6, 1.0);
    let at_kev = |kev: f64, density: f64| kev / card.temperature_kev(1.0, density);
    let points: Vec<(&str, f64, f64, TokamakLoad)> = vec![
        ("heating 0 MW", 2.0e6, 0.0, load(0.0, 0.5e6, 3.0e19, 2.5e6, 2.6, 0.5)),
        ("heating 30 MW", 4.0e6, 0.5e6, load(30.0e6, 1.0e6, 6.0e19, 3.8e6, 4.0, 0.5)),
        ("ohmic 0 MW", 4.0e6, 0.5e6, load(20.0e6, 0.0, 6.0e19, 3.8e6, 4.0, 0.5)),
        ("ohmic 5 MW", 2.0e6, 0.1e6, load(5.0e6, 5.0e6, 4.0e19, 3.8e6, 4.0, 0.5)),
        ("current 1.0 MA at 1.0 T", 0.5e6, 0.05e6, load(2.0e6, 0.5e6, 2.0e19, 1.0e6, 1.0, 0.5)),
        ("42976's first load", 4.0e6, 0.5e6, load(5.6e6, 3.115e6, 1.5025e19, 4.128e6, 3.827, 0.501)),
        ("current 5 MA at 5 T", 6.0e6, 0.5e6, load(30.0e6, 1.0e6, 6.0e19, 5.0e6, 5.0, 0.5)),
        ("field 8 T", 6.0e6, 0.5e6, load(20.0e6, 1.0e6, 6.0e19, 5.0e6, 8.0, 0.5)),
        ("tritium 0.25", 4.0e6, 0.5e6, TokamakLoad { tritium_fraction: 0.25, ..even_mix() }),
        ("tritium 0.93", 4.0e6, 0.5e6, TokamakLoad { tritium_fraction: 0.93, ..even_mix() }),
        ("tritium 1.0", 4.0e6, 0.5e6, TokamakLoad { tritium_fraction: 1.0, ..even_mix() }),
        ("density 0.5e19", 0.5e6, 0.1e6, load(10.0e6, 1.0e6, 0.5e19, 2.5e6, 2.6, 0.5)),
        ("density 1e20 at 5 MA", 8.0e6, 1.0e6, load(20.0e6, 1.0e6, 1.0e20, 5.0e6, 5.0, 0.5)),
        ("Greenwald 0.95 at 5 MA", 8.0e6, 1.0e6, load(20.0e6, 1.0e6, 0.95 * card.greenwald_density_per_m3(5.0e6), 5.0e6, 5.0, 0.5)),
        ("safety factor 2.05", 4.0e6, 0.5e6, load(20.0e6, 0.5e6, 6.0e19, 3.84e6, field_for(2.05, 3.84e6), 0.5)),
        ("safety factor 8", 2.0e6, 0.3e6, load(10.0e6, 0.5e6, 3.0e19, 2.0e6, field_for(8.0, 2.0e6), 0.5)),
        ("thermal 20 MJ", 20.0e6, 1.0e6, load(30.0e6, 1.0e6, 1.0e20, 5.0e6, 6.0, 0.5)),
        ("fast ions 0", 4.0e6, 0.0, even_mix()),
        ("fast ions 3.5 MJ", 4.0e6, 3.5e6, even_mix()),
        ("temperature 0.3 keV", at_kev(0.3, 6.0e19), 0.1e6, load(5.0e6, 1.0e6, 6.0e19, 3.8e6, 4.0, 0.5)),
        ("temperature 15 keV", at_kev(15.0, 2.0e19), 0.5e6, load(20.0e6, 1.0e6, 2.0e19, 5.0e6, 5.0, 0.5)),
        ("confinement 0.03 s", 2.4e6, 0.1e6, load(24.3e6, 2.44e6, 1.0e19, 1.65e6, 1.80, 0.5)),
        ("confinement 9 s", 2.0e6, 0.5e6, load(1.0e6, 0.2e6, 6.0e19, 3.8e6, 4.0, 0.5)),
    ];
    let mut shorter = 0;
    for (label, thermal, fast, load) in points {
        let dt_s = held_step(thermal, fast, &load);
        if dt_s < 1.0e-3 {
            shorter += 1;
            let (status, answer) = post(&base, "/verify_physics", &step_body(thermal, fast, &load, 1.0e-3)).await;
            assert_eq!(status, 422, "{label}: {answer}");
            assert!(answer.contains("(DtAboveStateCeiling)"), "{label}: {answer}");
        }
        let request = run_body(thermal, fast, &[(300, load)], dt_s, 300);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{label}: {answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        let (state, landed, worst, end) = kernel_run(thermal, fast, load, dt_s, 300);
        assert_eq!(tape["steps_landed"], landed, "{label}");
        assert_state_is(&tape["final_state"], &state, label);
        assert_eq!(tape["max_residual_conservation_error_hex"], format!("{:016x}", worst.to_bits()), "{label}");
        match end {
            None => assert_eq!(tape["outcome"], "completed", "{label}"),
            Some(reject) => {
                // A run stopped part way is stopped by a limit of physics or by
                // the ceiling of a later state, never by the ledger.
                assert!(matches!(reject, StepReject::BetaAboveTroyon | StepReject::DtAboveStateCeiling), "{label}: {reject:?}");
                assert_eq!(tape["outcome"], "refused", "{label}");
                assert_eq!(tape["reject"], format!("{reject:?}"), "{label}");
            }
        }
        assert!(worst < 1.5e-9, "{label}: {worst}");
        if label.starts_with("Greenwald") {
            assert_eq!(tape["final_state"]["near_density_limit"], true, "{label}");
        }
        let (status, verdict) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{verdict}");
        assert_eq!(serde_json::from_str::<Value>(&verdict).unwrap()["holds"], true, "{label}");
    }
    assert!(shorter >= 4, "{shorter}");
}

/// Four requests engine 0.6.0 answered and 0.7.0 to 0.9.0 refused at the
/// door by a bound of the compared envelope are answered: the run at the
/// largest loads, the fusion power at a tritium fraction of 0.93, and a hot
/// plasma at the smallest density. The dense one runs at its own state's
/// ceiling, since 1 ms is above it, and nothing reaches the ledger.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn requests_engine_0_6_0_answered_are_answered_again() {
    let base = spawn_app().await;
    let card = tokamak();
    for (thermal, fast, load, steps) in [
        (10.5e6, 0.0, load(30.0e6, 6.0e6, 1.0e19, 4.5e6, 4.2, 0.5), 1_u64),
        (2.0e6, 0.5e6, load(30.0e6, 6.0e6, card.greenwald_density_per_m3(4.5e6), 4.5e6, 4.2, 0.5), 1000),
        (3.0e6, 0.5e6, load(11.0e6, 1.0e6, 4.0e19, 2.6e6, 2.85, 0.93), 100),
        // 9.5 MJ at 1.65e19 per cubic metre is 16 keV.
        (9.5e6, 5.59e3, load(24.3e6, 2.44e6, 1.65e19, 3.84e6, 4.04, 0.5), 1),
    ] {
        let dt_s = held_step(thermal, fast, &load);
        let request = run_body(thermal, fast, &[(steps, load)], dt_s, steps);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        let (state, landed, _, end) = kernel_run(thermal, fast, load, dt_s, steps);
        assert_eq!(end, None, "{thermal} {load:?}");
        assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
        assert_eq!(tape["steps_landed"], landed);
        assert_state_is(&tape["final_state"], &state, "0.6.0 request");
        assert!(!answer.contains("FirstLawBreach"), "{answer}");
    }
}

/// Full heating at 1.65 MA, 1.80 T and 0.83 of the Greenwald density, from
/// 2 MJ at 1 ms: the stores rise past the compared normalised beta of 2.21,
/// where 0.9.0 refused the run, until the ceiling of the state the next step
/// would land on falls under 1 ms. The run is sealed with that refusal on
/// its last landed step. At 2e19 per cubic metre from 1 MJ at 0.1 ms the
/// stores go on to the Troyon limit and the run is sealed there.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_that_reaches_a_limit_of_its_state_is_sealed_with_that_refusal_on_its_last_landed_step() {
    let base = spawn_app().await;
    let card = tokamak();
    let dense = load(24.3e6, 0.173e6, 0.83 * card.greenwald_density_per_m3(1.65e6), 1.65e6, 1.80, 0.0);
    let thin = TokamakLoad { line_density_per_m3: 2.0e19, ..dense };
    for (thermal, at, dt_s, steps, name) in [
        (2.0e6, dense, 1.0e-3, 5_000_u64, "DtAboveStateCeiling"),
        (1.0e6, thin, 1.0e-4, 50_000, "BetaAboveTroyon"),
    ] {
        let request = run_body(thermal, 0.1e6, &[(steps, at)], dt_s, 100);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["outcome"], "refused");
        assert_eq!(tape["reject"], name);
        let (state, landed, _, end) = kernel_run(thermal, 0.1e6, at, dt_s, steps);
        assert_eq!(end.map(|reject| format!("{reject:?}")), Some(name.to_string()));
        assert!(state.beta_normalised > 2.21, "{}", state.beta_normalised);
        assert_eq!(tape["steps_landed"], landed);
        let detail = tape["reject_detail"].as_str().unwrap();
        assert!(detail.contains(&format!("({name})")), "{detail}");
        assert!(
            detail.ends_with(&format!("The run is sealed to step {landed}. The state at step {} was refused and is not sealed.", landed + 1)),
            "{detail}"
        );
        assert_state_is(&tape["final_state"], &state, "refused run");
        let (status, verdict) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{verdict}");
        assert_eq!(serde_json::from_str::<Value>(&verdict).unwrap()["holds"], true);
    }
}

/// The clock is the caller's: a run continued at the returned t_s lands
/// where the straight run lands, bit for bit, and goes on past 5.475 s, the
/// longest compared window; a request without the clock is refused, and so
/// is a clock below zero.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_continued_run_states_its_clock_and_runs_past_the_compared_window() {
    let base = spawn_app().await;
    let run = |body: Value| {
        let base = base.clone();
        async move {
            let (status, answer) = post(&base, "/run_trajectory", &body).await;
            assert_eq!(status, 200, "{answer}");
            serde_json::from_str::<Value>(&answer).unwrap()
        }
    };
    let straight = run(run_body(4.0e6, 0.5e6, &[(7000, even_mix())], 1.0e-3, 7000)).await;
    assert_eq!(straight["outcome"], "completed");
    let first_request = run_body(4.0e6, 0.5e6, &[(5000, even_mix())], 1.0e-3, 5000);
    let first = run(first_request.clone()).await;
    let end = &first["final_state"];
    let number = |state: &Value, name: &str| f64::from_bits(u64::from_str_radix(state[format!("{name}_hex")].as_str().unwrap(), 16).unwrap());
    let mut second_request = run_body_at(
        number(end, "t_s"),
        number(end, "thermal_energy_j"),
        number(end, "fast_ion_energy_j"),
        &[(2000, even_mix())],
        1.0e-3,
        2000,
    );
    // The clock carries the pulse on: the request names the sealed first
    // part in from, and without it is refused by name.
    let (status, answer) = post(&base, "/run_trajectory", &second_request).await;
    assert_eq!(status, 422, "{answer}");
    assert!(answer.ends_with("(ContinuationWithoutUpstream)."), "{answer}");
    second_request["from"] = json!({ "request": first_request, "run_seal": first["run_seal"] });
    let second = run(second_request).await;
    assert_eq!(second["outcome"], "completed");
    assert!(number(&second["final_state"], "t_s") > 5.475);
    for name in ["t_s", "thermal_energy_j", "fast_ion_energy_j"] {
        assert_eq!(second["final_state"][format!("{name}_hex")], straight["final_state"][format!("{name}_hex")], "{name}");
    }
    // A start stated past 0 on the caller's clock carries a pulse on, and
    // without from is refused by name before any step.
    for (path, request) in [
        ("/run_trajectory", run_body_at(5.0, 4.0e6, 0.5e6, &[(1000, even_mix())], 1.0e-3, 1000)),
        ("/verify_physics", step_body_at(6.0, 4.0e6, 0.5e6, &even_mix(), 1.0e-3)),
    ] {
        let (status, answer) = post(&base, path, &request).await;
        assert_eq!(status, 422, "{answer}");
        assert!(answer.ends_with("(ContinuationWithoutUpstream)."), "{answer}");
    }
    // The clock is required, and held at zero.
    let mut request = run_body(4.0e6, 0.5e6, &[(1, even_mix())], 1.0e-3, 1);
    request["initial"].as_object_mut().unwrap().remove("elapsed_time_s");
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 422, "{answer}");
    assert!(answer.contains("elapsed_time_s"), "{answer}");
    let mut request = step_body(4.0e6, 0.5e6, &even_mix(), 1.0e-3);
    request["parameters"].as_object_mut().unwrap().remove("elapsed_time_s");
    let (status, answer) = post(&base, "/verify_physics", &request).await;
    assert_eq!(status, 422, "{answer}");
    assert!(answer.contains("elapsed_time_s"), "{answer}");
    let (status, answer) = post(&base, "/verify_physics", &step_body_at(-1.0e-9, 4.0e6, 0.5e6, &even_mix(), 1.0e-3)).await;
    assert_eq!(status, 422, "{answer}");
    assert!(answer.contains("NegativeTime"), "{answer}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_description_lists_the_card_its_limits_and_examples_that_run() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_card", &json!({ "domain": "energy", "card": "tokamak_jet_dte1" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    let rejects: Vec<&str> = described["rejects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reject| reject["name"].as_str().unwrap())
        .collect();
    for name in [
        "HeatingPowerNegative",
        "OhmicPowerNegative",
        "PlasmaCurrentNotPositive",
        "ToroidalFieldNotPositive",
        "FuelMixOutsideCard",
        "DensityNotPositive",
        "DensityAboveGreenwald",
        "BetaAboveTroyon",
        "SafetyFactorBelowLimit",
        "TemperatureBelowReactivityFit",
        "TemperatureAboveReactivityFit",
        "NegativeTime",
        "StepBelowClockResolution",
        "DtAboveStateCeiling",
    ] {
        assert!(rejects.contains(&name), "{name}");
    }
    for gone in [
        "HeatingPowerOutsideCard",
        "FuelMixOutsideComparison",
        "DensityBelowCard",
        "DensityAboveComparison",
        "GreenwaldFractionAboveComparison",
        "SafetyFactorOutsideComparison",
        "EnergyOutsideComparison",
        "TemperatureOutsideComparison",
        "BetaOutsideComparison",
        "ConfinementTimeOutsideComparison",
        "TimeBeyondCard",
    ] {
        assert!(!rejects.contains(&gone), "{gone}");
    }
    // After the kernel's 21 names the list holds the gateway's own three for this card, DtBelowFloor and DtAboveCeiling and ContinuationWithoutUpstream.
    assert_eq!(rejects.len(), 24);
    assert!(!text.contains("The card ends 5.475 s"), "{text}");
    let (status, answer) = post(&base, "/verify_physics", &described["example_compute_step"]).await;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = post(&base, "/run_trajectory", &described["example_run_trajectory"]).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], 1000);
    let (state, _, _, _) = kernel_run(4.0e6, 0.5e6, even_mix(), 1.0e-3, 1000);
    assert_state_is(&tape["final_state"], &state, "example");
    // The list line states the failure and the envelope.
    let client = Client::new();
    let list: Value = serde_json::from_str(&client.get(format!("{base}/cards")).send().await.unwrap().text().await.unwrap()).unwrap();
    let cards = list["cards"].as_array().unwrap_or_else(|| list.as_array().unwrap());
    let line = cards.iter().find(|entry| entry["card"] == "tokamak_jet_dte1").unwrap().to_string();
    assert!(line.contains("the comparison failed"), "{line}");
    assert!(line.contains("0.743 of the measured"), "{line}");
}
