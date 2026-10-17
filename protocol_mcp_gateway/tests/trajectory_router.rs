//! `POST /run_trajectory` and `POST /verify_tape`.
//!
//! A one-step run matches `POST /verify_physics` on the post-step state.
//! Longer runs match an in-test loop that calls the kernel step on one state.
//! A refusal is HTTP 200. The tape check does not run physics.

mod answer_text;

use std::collections::BTreeSet;
use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::{
    battery_sag, cable_stretch, capsule_stardust, cell_18650pf, generator_mmrtg_f1, grid_node,
    hydraulic_fluid, resonance, satellite_lageos1, swing, tesseract, thermal, ti64_lattice,
};
use protocol_mcp_gateway::app;
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

async fn post(base: &str, path: &str, body: &str) -> (u16, String) {
    let client = Client::new();
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

/// A value as the text it is sent as: its JSON text, with every answer the
/// engine gave inside it as it came (the long form holds the answer sent
/// byte for byte).
fn wire(value: &Value) -> (String, Value) {
    let text = answer_text::as_sent(value);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    (text, parsed)
}

fn scalar(value: &Value, key: &str) -> f64 {
    value[key]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} in {value}"))
}

fn vec3(value: &Value, key: &str) -> [f64; 3] {
    let items = value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} in {value}"));
    assert_eq!(items.len(), 3, "{key} in {value}");
    [
        items[0].as_f64().unwrap(),
        items[1].as_f64().unwrap(),
        items[2].as_f64().unwrap(),
    ]
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

fn dt_in_window(gate: Option<f64>) -> f64 {
    gate.expect("gate").min(0.1)
}

#[derive(Clone, Copy)]
enum Probe {
    Num(f64),
    Vec3([f64; 3]),
    Flag(bool),
}

fn num(name: &'static str, value: f64) -> (&'static str, Probe) {
    (name, Probe::Num(value))
}

fn vec3_probe(name: &'static str, value: [f64; 3]) -> (&'static str, Probe) {
    (name, Probe::Vec3(value))
}

fn flag(name: &'static str, value: bool) -> (&'static str, Probe) {
    (name, Probe::Flag(value))
}

fn hex_digits(hex: &str, label: &str) {
    assert_eq!(hex.len(), 16, "{label}");
    assert!(
        hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "{label} {hex}"
    );
}

fn check_state(state: &Value, probes: &[(&str, Probe)]) {
    let object = state.as_object().unwrap_or_else(|| panic!("{state}"));
    let mut expected_keys = BTreeSet::new();
    for (key, probe) in probes {
        expected_keys.insert((*key).to_string());
        match probe {
            Probe::Num(expected) => {
                let got = object
                    .get(*key)
                    .and_then(Value::as_f64)
                    .unwrap_or_else(|| panic!("state.{key} missing from {state}"));
                let hex_key = format!("{key}_hex");
                let hex = object
                    .get(&hex_key)
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| panic!("state.{hex_key} missing from {state}"));
                assert_eq!(
                    hex,
                    format!("{:016x}", expected.to_bits()),
                    "state.{hex_key}"
                );
                // serde_json is not correctly rounded. Some finite decimals are
                // outside the image of its parser, so the number token can land
                // one ULP away. The hex sibling is the bit check.
                let gap = got.to_bits().abs_diff(expected.to_bits());
                assert!(
                    gap <= 1,
                    "state.{key} number is {gap} bit-steps from {}",
                    expected.to_bits()
                );
                hex_digits(hex, &hex_key);
                expected_keys.insert(hex_key);
            }
            Probe::Vec3(expected) => {
                let got = object
                    .get(*key)
                    .and_then(Value::as_array)
                    .unwrap_or_else(|| panic!("state.{key} missing from {state}"));
                assert_eq!(got.len(), 3, "state.{key}");
                let hex_key = format!("{key}_hex");
                let hexes = object
                    .get(&hex_key)
                    .and_then(Value::as_array)
                    .unwrap_or_else(|| panic!("state.{hex_key} missing from {state}"));
                assert_eq!(hexes.len(), 3, "state.{hex_key}");
                for (index, component) in expected.iter().enumerate() {
                    let hex = hexes[index]
                        .as_str()
                        .unwrap_or_else(|| panic!("state.{hex_key}[{index}] is not a string"));
                    assert_eq!(
                        hex,
                        format!("{:016x}", component.to_bits()),
                        "state.{hex_key}[{index}]"
                    );
                    let number = got[index]
                        .as_f64()
                        .unwrap_or_else(|| panic!("state.{key}[{index}] is not a number"));
                    let gap = number.to_bits().abs_diff(component.to_bits());
                    assert!(
                        gap <= 1,
                        "state.{key}[{index}] number is {gap} bit-steps from {}",
                        component.to_bits()
                    );
                    hex_digits(hex, &format!("{hex_key}[{index}]"));
                }
                expected_keys.insert(hex_key);
            }
            Probe::Flag(expected) => {
                let got = object
                    .get(*key)
                    .and_then(Value::as_bool)
                    .unwrap_or_else(|| panic!("state.{key} missing from {state}"));
                assert_eq!(got, *expected, "state.{key}");
            }
        }
    }
    let actual: BTreeSet<_> = object.keys().cloned().collect();
    assert_eq!(actual, expected_keys, "state keys drifted");
}

fn assert_same_value(left: &Value, right: &Value, label: &str) {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => {
            assert_eq!(
                a.as_f64().unwrap().to_bits(),
                b.as_f64().unwrap().to_bits(),
                "{label}"
            );
        }
        (Value::String(a), Value::String(b)) => assert_eq!(a, b, "{label}"),
        (Value::Bool(a), Value::Bool(b)) => assert_eq!(a, b, "{label}"),
        (Value::Null, Value::Null) => {}
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{label}");
            for (index, (item, other)) in a.iter().zip(b).enumerate() {
                assert_same_value(item, other, &format!("{label}[{index}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let left_keys: BTreeSet<_> = a.keys().collect();
            let right_keys: BTreeSet<_> = b.keys().collect();
            assert_eq!(left_keys, right_keys, "{label}");
            for key in a.keys() {
                assert_same_value(&a[key], &b[key], &format!("{label}.{key}"));
            }
        }
        _ => panic!("{label} kinds differ: {left} vs {right}"),
    }
}

fn thermal_probes(state: &thermal::ThermalNodeState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("temperature_k", state.temperature_k),
        num("heat_power_w", state.heat_power_w),
        num("ambient_k", state.ambient_k),
        num("internal_energy_j", state.internal_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("boundary_work_j", state.boundary_work_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("heat_to_ambient_w", state.heat_to_ambient_w),
        num("ceiling_margin_k", state.ceiling_margin_k),
        num(
            "entropy_production_w_per_k",
            state.entropy_production_w_per_k,
        ),
        flag("derated", state.derated),
    ]
}

fn resonance_probes(state: &resonance::OscillatorState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("displacement_m", state.displacement_m),
        num("velocity_m_s", state.velocity_m_s),
        num("accel_m_s2", state.accel_m_s2),
        num("kinetic_j", state.kinetic_j),
        num("potential_j", state.potential_j),
        num("mechanical_energy_j", state.mechanical_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("boundary_work_j", state.boundary_work_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("stroke_margin_m", state.stroke_margin_m),
        flag("stroke_limited", state.stroke_limited),
    ]
}

fn battery_probes(state: &battery_sag::PackState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("soc", state.soc),
        num("current_a", state.current_a),
        num("terminal_v", state.terminal_v),
        num("open_circuit_v", state.open_circuit_v),
        num("resistance_ohm", state.resistance_ohm),
        num("chemical_energy_j", state.chemical_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("electrical_work_j", state.electrical_work_j),
        num("joule_j", state.joule_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("cutoff_margin_v", state.cutoff_margin_v),
        flag("sag_warning", state.sag_warning),
    ]
}

fn cell_probes(state: &cell_18650pf::CellState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("soc", state.soc),
        num("current_a", state.current_a),
        num("terminal_v", state.terminal_v),
        num("open_circuit_v", state.open_circuit_v),
        num("resistance_ohm", state.resistance_ohm),
        num("chemical_energy_j", state.chemical_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("electrical_work_j", state.electrical_work_j),
        num("joule_j", state.joule_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("cutoff_margin_v", state.cutoff_margin_v),
        flag("sag_warning", state.sag_warning),
    ]
}

fn hydraulic_probes(state: &hydraulic_fluid::FluidState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("pressure_pa", state.pressure_pa),
        num("flow_m3_s", state.flow_m3_s),
        num("fluid_volume_m3", state.fluid_volume_m3),
        num("compression_m", state.compression_m),
        num("mass_kg", state.mass_kg),
        num("elastic_energy_j", state.elastic_energy_j),
        num("mass_anchor_kg", state.mass_anchor_kg),
        num("energy_anchor_j", state.energy_anchor_j),
        num("boundary_mass_kg", state.boundary_mass_kg),
        num("boundary_work_j", state.boundary_work_j),
        num("mass_defect_kg", state.mass_defect_kg),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("vapor_margin_pa", state.vapor_margin_pa),
        flag("column_soft", state.column_soft),
        flag("column_limited", state.column_limited),
    ]
}

fn grid_probes(state: &grid_node::GridNodeState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("current_a", state.current_a),
        num("voltage_v", state.voltage_v),
        num("temperature_k", state.temperature_k),
        num("upstream_voltage_v", state.upstream_voltage_v),
        num("load_current_a", state.load_current_a),
        num("ambient_k", state.ambient_k),
        num("resistance_ohm", state.resistance_ohm),
        num("joule_power_w", state.joule_power_w),
        num("electrical_energy_j", state.electrical_energy_j),
        num("thermal_energy_j", state.thermal_energy_j),
        num("internal_energy_j", state.internal_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("boundary_work_j", state.boundary_work_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("voltage_spike_v", state.voltage_spike_v),
        num("melt_margin_k", state.melt_margin_k),
        num(
            "entropy_production_w_per_k",
            state.entropy_production_w_per_k,
        ),
        flag("surge", state.surge),
    ]
}

fn ti64_probes(state: &ti64_lattice::Ti64LatticeState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        vec3_probe("position_m", state.position_m),
        vec3_probe("velocity_m_s", state.velocity_m_s),
        vec3_probe("tension_n", state.tension_n),
        vec3_probe("torque_nm", state.torque_nm),
        vec3_probe("applied_force_n", state.applied_force_n),
        num("temperature_k", state.temperature_k),
        vec3_probe("ligament_stress_pa", state.ligament_stress_pa),
        num("von_mises_mpa", state.von_mises_mpa),
        num("energy_j", state.energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("boundary_work_j", state.boundary_work_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
    ]
}

fn swing_probes(state: &swing::SwingState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("delta_rad", state.delta_rad),
        num("delta_omega_rad_s", state.delta_omega_rad_s),
        num("pll_err_rad", state.pll_err_rad),
        num("p_mech_pu", state.p_mech_pu),
        num("p_elec_pu", state.p_elec_pu),
        num("transient_energy_pu", state.transient_energy_pu),
        num("energy_anchor_pu", state.energy_anchor_pu),
        num("boundary_work_pu", state.boundary_work_pu),
        num("energy_defect_pu", state.energy_defect_pu),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("synchronism_margin_rad", state.synchronism_margin_rad),
        flag("pll_slipped", state.pll_slipped),
    ]
}

fn tesseract_probes(state: &tesseract::TesseractState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("displacement_m", state.displacement_m),
        num("velocity_m_s", state.velocity_m_s),
        num("accel_m_s2", state.accel_m_s2),
        num("envelope_m", state.envelope_m),
        num("kinetic_j", state.kinetic_j),
        num("potential_j", state.potential_j),
        num("mechanical_energy_j", state.mechanical_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("boundary_work_j", state.boundary_work_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("scale_factor_n", state.scale_factor_n),
    ]
}

fn cable_probes(state: &cable_stretch::TendonState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("elongation_m", state.elongation_m),
        num("velocity_m_s", state.velocity_m_s),
        num("force_n", state.force_n),
        num("tension_n", state.tension_n),
        num("kinetic_j", state.kinetic_j),
        num("potential_j", state.potential_j),
        num("mechanical_energy_j", state.mechanical_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("boundary_work_j", state.boundary_work_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("ultimate_margin_n", state.ultimate_margin_n),
        flag("tracking_degraded", state.tracking_degraded),
        flag("control_limited", state.control_limited),
    ]
}

fn thermal_loop(
    temp: f64,
    segments: &[(u64, f64, f64)],
    dt: f64,
) -> (f64, Vec<(&'static str, Probe)>) {
    let node = thermal::ThermalNode::avionics_cold_plate();
    let mut state = thermal::ThermalNodeState::try_phase(&node, temp).expect("thermal phase");
    for &(steps, heat, ambient) in segments {
        let load = thermal::ThermalLoad {
            heat_power_w: heat,
            ambient_k: ambient,
        };
        for _ in 0..steps {
            let mut frame = blank_frame();
            thermal::step(&node, &mut state, &mut frame, load, dt).expect("thermal step");
        }
    }
    let residual = state.residual_conservation_error;
    (residual, thermal_probes(&state))
}

fn resonance_loop(
    displacement: f64,
    velocity: f64,
    accel: f64,
    dt: f64,
    steps: u64,
) -> (f64, Vec<(&'static str, Probe)>) {
    let oscillator = resonance::DynamicOscillator::mount_10hz();
    let mut state = resonance::OscillatorState::try_phase(&oscillator, displacement, velocity)
        .expect("resonance phase");
    let load = resonance::OscillatorLoad { accel_m_s2: accel };
    for _ in 0..steps {
        let mut frame = blank_frame();
        resonance::step(&oscillator, &mut state, &mut frame, load, dt).expect("resonance step");
    }
    let residual = state.residual_conservation_error;
    (residual, resonance_probes(&state))
}

fn battery_loop(soc: f64, current: f64, dt: f64, steps: u64) -> (f64, Vec<(&'static str, Probe)>) {
    let pack = battery_sag::LiIonPack::pack_12s();
    let mut state = battery_sag::PackState::try_soc(&pack, soc).expect("battery phase");
    let load = battery_sag::PackLoad { current_a: current };
    for _ in 0..steps {
        let mut frame = blank_frame();
        battery_sag::step(&pack, &mut state, &mut frame, load, dt).expect("battery step");
    }
    let residual = state.residual_conservation_error;
    (residual, battery_probes(&state))
}

fn cell_loop(soc: f64, current: f64, dt: f64, steps: u64) -> (f64, Vec<(&'static str, Probe)>) {
    let specimen = cell_18650pf::Cell18650Pf::cell();
    // a 1C discharge held at its own current
    let mut state =
        cell_18650pf::CellState::try_continued(&specimen, soc, current).expect("cell phase");
    let load = cell_18650pf::CellLoad { current_a: current };
    for _ in 0..steps {
        let mut frame = blank_frame();
        cell_18650pf::step(&specimen, &mut state, &mut frame, load, dt).expect("cell step");
    }
    let residual = state.residual_conservation_error;
    (residual, cell_probes(&state))
}

fn hydraulic_loop(
    pressure: f64,
    flow: f64,
    dt: f64,
    steps: u64,
) -> (f64, Vec<(&'static str, Probe)>) {
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let mut state =
        hydraulic_fluid::FluidState::try_pressure(&column, pressure).expect("hydraulic phase");
    let load = hydraulic_fluid::FluidLoad { flow_m3_s: flow };
    for _ in 0..steps {
        let mut frame = blank_frame();
        hydraulic_fluid::step(&column, &mut state, &mut frame, load, dt).expect("hydraulic step");
    }
    let residual = state.residual_conservation_error;
    (residual, hydraulic_probes(&state))
}

fn assert_landed(body: &Value, residual: f64, probes: &[(&str, Probe)], steps: u64) {
    assert_eq!(body["outcome"], "completed", "{body}");
    assert_eq!(body["steps_landed"], steps, "{body}");
    assert_eq!(body["steps_requested"], steps, "{body}");
    assert!(body["reject"].is_null(), "{body}");
    assert_eq!(body["engine_version"], "genesis_kernel_v0.12.0");
    let got = body["final_residual_conservation_error"]
        .as_f64()
        .expect("final residual");
    let gap = got.to_bits().abs_diff(residual.to_bits());
    assert!(gap <= 1, "final residual drifted by {gap} bit-steps");
    check_state(&body["final_state"], probes);
}

fn frames_cover_landed(body: &Value, landed: u64) {
    let frames = body["frames"].as_array().expect("frames");
    assert_eq!(
        frames.len() as u64,
        landed,
        "frames include a step that did not land"
    );
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame["step"], index as u64 + 1, "{frame}");
        assert_eq!(frame["frame_hex"].as_str().unwrap().len(), 128);
        assert_eq!(frame["link"].as_str().unwrap().len(), 64);
    }
}

/// Why the in-test kernel loop stopped: a step rejection.
#[derive(Debug, PartialEq)]
enum Refusal<R> {
    Reject(R),
}

/// Steps one state until the closure refuses. Returns the landed count, the
/// state of the last landed step, and the refusal.
fn land_until_refused<S: Copy, R>(
    start: S,
    limit: u64,
    mut step: impl FnMut(&mut S) -> Result<(), Refusal<R>>,
) -> (u64, S, Refusal<R>) {
    let mut state = start;
    let mut last = start;
    let mut landed = 0_u64;
    for _ in 0..limit {
        match step(&mut state) {
            Ok(()) => {
                landed += 1;
                last = state;
            }
            Err(refusal) => return (landed, last, refusal),
        }
    }
    panic!("no refusal in {limit} steps");
}

async fn run_trajectory(base: &str, text: &str) -> Value {
    let (status, response) = post(base, "/run_trajectory", text).await;
    assert_eq!(status, 200, "{response}");
    serde_json::from_str(&response).unwrap()
}

/// The sentence of a run refused part way: the card's words, with their last
/// sentence, that the state is not sealed, naming the steps.
fn sealed_to(detail: &str, landed: u64) -> String {
    let body = match detail.strip_suffix(" state is not sealed.") {
        Some(head) => match head.rfind(". ") {
            Some(at) => &detail[..at + 1],
            None => detail,
        },
        None => detail,
    };
    if landed == 0 {
        format!("{body} The run is sealed with no step landed. The state at step 1 was refused and is not sealed.")
    } else {
        format!("{body} The run is sealed to step {landed}. The state at step {} was refused and is not sealed.", landed + 1)
    }
}

fn assert_refused(
    body: &Value,
    reject: Option<&str>,
    detail: &str,
    landed: u64,
    residual: f64,
    probes: &[(&str, Probe)],
) {
    // A run that reaches an end its card declares is ended there; any other
    // refusal names the refused step and the step the run is sealed to.
    let declared_end = matches!(
        (body["inputs"]["card"].as_str(), reject),
        (Some("capsule_stardust"), Some("PastFlightDeployment"))
    );
    match reject {
        Some(name) => assert_eq!(body["reject"], name, "{body}"),
        None => assert!(body["reject"].is_null(), "{body}"),
    }
    if declared_end {
        assert_eq!(body["outcome"], "ended", "{body}");
        let sentence = body["reject_detail"].as_str().unwrap();
        assert!(sentence.starts_with("Ended: the run reached the end the"), "{sentence}");
        assert!(sentence.ends_with(&format!("the run is sealed to step {landed}.")), "{sentence}");
    } else {
        assert_eq!(body["outcome"], "refused", "{body}");
        assert_eq!(body["reject_detail"], sealed_to(detail, landed), "{body}");
    }
    assert_eq!(body["steps_landed"], landed, "{body}");
    assert_eq!(
        body["final_residual_conservation_error_hex"],
        format!("{:016x}", residual.to_bits()),
        "{body}"
    );
    frames_cover_landed(body, landed);
    check_state(&body["final_state"], probes);
}

#[tokio::test]
async fn one_step_matches_verify_physics() {
    let base = spawn_app().await;
    let node = thermal::ThermalNode::avionics_cold_plate();
    let parameters = json!({
        "temperature_k": thermal::REFERENCE_TEMPERATURE_K,
        "heat_power_w": 1.0,
        "ambient_k": thermal::REFERENCE_TEMPERATURE_K,
        "dt_s": dt_in_window(node.gate_dt_s()),
    });
    let verify = json!({
        "domain": "energy",
        "card": "thermal",
        "parameters": parameters,
    });
    let trajectory = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": parameters["temperature_k"] },
        "dt_s": parameters["dt_s"],
        "segments": [{
            "steps": 1,
            "load": {
                "heat_power_w": parameters["heat_power_w"],
                "ambient_k": parameters["ambient_k"]
            }
        }],
        "record_every": 1
    });
    let (verify_wire, _) = wire(&verify);
    let (trajectory_wire, _) = wire(&trajectory);
    let (verify_status, verify_text) = post(&base, "/verify_physics", &verify_wire).await;
    let (run_status, run_text) = post(&base, "/run_trajectory", &trajectory_wire).await;
    assert_eq!(verify_status, 200, "{verify_text}");
    assert_eq!(run_status, 200, "{run_text}");
    let verify_body: Value = serde_json::from_str(&verify_text).unwrap();
    let run_body: Value = serde_json::from_str(&run_text).unwrap();
    assert_same_value(&verify_body["state"], &run_body["final_state"], "state");
    assert_eq!(
        verify_body["residual_conservation_error"]
            .as_f64()
            .unwrap()
            .to_bits(),
        run_body["final_residual_conservation_error"]
            .as_f64()
            .unwrap()
            .to_bits()
    );
    assert_eq!(run_body["outcome"], "completed");
    assert_eq!(run_body["steps_landed"], 1);
    assert_eq!(run_body["frames"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn n_step_matches_the_kernel_on_five_cards() {
    let base = spawn_app().await;

    let node = thermal::ThermalNode::avionics_cold_plate();
    let thermal_body = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": dt_in_window(node.gate_dt_s()),
        "segments": [{
            "steps": 24,
            "load": {
                "heat_power_w": 1.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K
            }
        }],
        "record_every": 4
    });
    let (text, parsed) = wire(&thermal_body);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    let (residual, probes) = thermal_loop(
        scalar(&parsed["initial"], "temperature_k"),
        &[(
            24,
            scalar(&parsed["segments"][0]["load"], "heat_power_w"),
            scalar(&parsed["segments"][0]["load"], "ambient_k"),
        )],
        scalar(&parsed, "dt_s"),
    );
    assert_landed(&body, residual, &probes, 24);
    let frames = body["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 6);
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame["step"], (index as u64 + 1) * 4);
    }

    let oscillator = resonance::DynamicOscillator::mount_10hz();
    let resonance_body = json!({
        "domain": "signal",
        "card": "resonance",
        "initial": {
            "displacement_m": 0.0,
            "velocity_m_s": 0.0,
            "t_s": 0.0,
            "energy_anchor_j": 0.0,
            "boundary_work_j": 0.0
        },
        "dt_s": dt_in_window(oscillator.gate_dt_s()),
        "segments": [{ "steps": 24, "load": { "accel_m_s2": 1.0 } }],
        "record_every": 1
    });
    let (text, parsed) = wire(&resonance_body);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    let (residual, probes) = resonance_loop(
        scalar(&parsed["initial"], "displacement_m"),
        scalar(&parsed["initial"], "velocity_m_s"),
        scalar(&parsed["segments"][0]["load"], "accel_m_s2"),
        scalar(&parsed, "dt_s"),
        24,
    );
    assert_landed(&body, residual, &probes, 24);

    let battery_body = json!({
        "domain": "energy",
        "card": "battery_sag",
        "initial": { "soc": 1.0 },
        "dt_s": 0.1,
        "segments": [{ "steps": 24, "load": { "current_a": 1.0 } }],
        "record_every": 1
    });
    let (text, parsed) = wire(&battery_body);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    let (residual, probes) = battery_loop(
        scalar(&parsed["initial"], "soc"),
        scalar(&parsed["segments"][0]["load"], "current_a"),
        scalar(&parsed, "dt_s"),
        24,
    );
    assert_landed(&body, residual, &probes, 24);

    let cell_body = json!({
        "domain": "energy",
        "card": "cell_18650pf",
        "initial": { "soc": 1.0, "held_current_a": 2.9 },
        "dt_s": 0.1,
        "segments": [{ "steps": 24, "load": { "current_a": 2.9 } }],
        "record_every": 1
    });
    let (text, parsed) = wire(&cell_body);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    let (residual, probes) = cell_loop(
        scalar(&parsed["initial"], "soc"),
        scalar(&parsed["segments"][0]["load"], "current_a"),
        scalar(&parsed, "dt_s"),
        24,
    );
    assert_landed(&body, residual, &probes, 24);

    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let hydraulic_body = json!({
        "domain": "materials",
        "card": "hydraulic_fluid",
        "initial": { "pressure_pa": hydraulic_fluid::REFERENCE_PRESSURE_PA },
        "dt_s": dt_in_window(column.gate_dt_s()),
        "segments": [{ "steps": 24, "load": { "flow_m3_s": 0.0 } }],
        "record_every": 1
    });
    let (text, parsed) = wire(&hydraulic_body);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    let (residual, probes) = hydraulic_loop(
        scalar(&parsed["initial"], "pressure_pa"),
        scalar(&parsed["segments"][0]["load"], "flow_m3_s"),
        scalar(&parsed, "dt_s"),
        24,
    );
    assert_landed(&body, residual, &probes, 24);
}

#[tokio::test]
async fn n_step_matches_the_kernel_on_the_other_five_cards() {
    const STEPS: u64 = 600;
    let base = spawn_app().await;

    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let yield_force = ti64_lattice::YIELD_STRENGTH_MPA * 1.0e6 * cell.area_m2[0];
    let (text, parsed) = wire(&json!({
        "domain": "materials",
        "card": "ti64_lattice",
        "initial": {
            "position_m": [0.0, 0.0, 0.0],
            "velocity_m_s": [0.0, 0.0, 0.0],
            "start_t_s": 0.0, "start_force_n": [0.0, 0.0, 0.0], "start_temperature_k": 293.15, "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0
        },
        "dt_s": dt_in_window(cell.gate_dt_s()),
        "segments": [{
            "steps": STEPS,
            "load": {
                "force_n": [0.3 * yield_force, -0.2 * yield_force, 0.1 * yield_force],
                "torque_nm": [1.0, -0.5, 0.25],
                "temperature_k": 320.0
            }
        }],
        "record_every": 100
    }));
    let body = run_trajectory(&base, &text).await;
    let dt = scalar(&parsed, "dt_s");
    let load = &parsed["segments"][0]["load"];
    let load = ti64_lattice::Ti64Load {
        force_n: vec3(load, "force_n"),
        torque_nm: vec3(load, "torque_nm"),
        temperature_k: scalar(load, "temperature_k"),
    };
    let mut state = ti64_lattice::Ti64LatticeState::try_phase(
        &cell,
        vec3(&parsed["initial"], "position_m"),
        vec3(&parsed["initial"], "velocity_m_s"),
    )
    .expect("ti64 phase");
    let rest = state;
    for _ in 0..STEPS {
        let mut frame = blank_frame();
        ti64_lattice::step(&cell, &mut state, &mut frame, load, dt).expect("ti64 step");
        assert_eq!(frame.flags, 0, "ti64 flagged at {} s", state.t_s);
    }
    assert_ne!(state.position_m, rest.position_m, "the node did not move");
    assert_landed(
        &body,
        state.residual_conservation_error,
        &ti64_probes(&state),
        STEPS,
    );
    assert_eq!(body["frames"].as_array().unwrap().len(), 6);

    let node = grid_node::GridNode::copper_fuse_link();
    let (text, parsed) = wire(&json!({
        "domain": "energy",
        "card": "grid_node",
        "initial": {
            "current_a": 0.0,
            "voltage_v": 0.0,
            "temperature_k": grid_node::REFERENCE_TEMPERATURE_K
        },
        "dt_s": dt_in_window(node.gate_dt_s()),
        "segments": [{
            "steps": STEPS,
            "load": {
                "upstream_voltage_v": grid_node::CATALOG_VOLTAGE_V,
                "load_current_a": 5.0,
                "ambient_k": 300.0
            }
        }],
        "record_every": 100
    }));
    let body = run_trajectory(&base, &text).await;
    let dt = scalar(&parsed, "dt_s");
    let load = &parsed["segments"][0]["load"];
    let load = grid_node::GridLoad {
        upstream_voltage_v: scalar(load, "upstream_voltage_v"),
        load_current_a: scalar(load, "load_current_a"),
        ambient_k: scalar(load, "ambient_k"),
    };
    let mut state = grid_node::GridNodeState::try_phase(
        &node,
        scalar(&parsed["initial"], "current_a"),
        scalar(&parsed["initial"], "voltage_v"),
        scalar(&parsed["initial"], "temperature_k"),
    )
    .expect("grid phase");
    let start = state;
    for _ in 0..STEPS {
        let mut frame = blank_frame();
        grid_node::step(&node, &mut state, &mut frame, load, dt).expect("grid step");
    }
    assert!(
        state.temperature_k > start.temperature_k,
        "the fuse did not warm"
    );
    assert_landed(
        &body,
        state.residual_conservation_error,
        &grid_probes(&state),
        STEPS,
    );

    let machine = swing::SwingMachine::synchronous_60hz();
    let (text, parsed) = wire(&json!({
        "domain": "energy",
        "card": "swing",
        "initial": {
            "delta_rad": 0.3,
            "delta_omega_rad_s": 0.0,
            "pll_err_rad": 0.05
        },
        "dt_s": dt_in_window(machine.gate_dt_s()),
        "segments": [{ "steps": STEPS, "load": { "p_mech_pu": 0.8 } }],
        "record_every": 100
    }));
    let body = run_trajectory(&base, &text).await;
    let dt = scalar(&parsed, "dt_s");
    let load = swing::SwingLoad {
        p_mech_pu: scalar(&parsed["segments"][0]["load"], "p_mech_pu"),
    };
    let mut state = swing::SwingState::try_dispatched(
        &machine,
        scalar(&parsed["initial"], "delta_rad"),
        scalar(&parsed["initial"], "delta_omega_rad_s"),
        scalar(&parsed["initial"], "pll_err_rad"),
        load.p_mech_pu,
    )
    .expect("swing phase");
    let start = state;
    for _ in 0..STEPS {
        let mut frame = blank_frame();
        swing::step(&machine, &mut state, &mut frame, load, dt).expect("swing step");
    }
    assert!(
        state.delta_rad > start.delta_rad,
        "the rotor did not advance"
    );
    assert_landed(
        &body,
        state.residual_conservation_error,
        &swing_probes(&state),
        STEPS,
    );

    let resonator = tesseract::Tesseract::drive_mode();
    let (text, parsed) = wire(&json!({
        "domain": "signal",
        "card": "tesseract",
        "initial": { "displacement_m": 0.001, "velocity_m_s": 0.0, "energy_defect_j": 0.0 },
        "dt_s": dt_in_window(resonator.gate_dt_s()),
        "segments": [{
            "steps": STEPS,
            "load": {
                "accel_m_s2": 2.0,
                "omega_ext_rad_s": 0.5,
                "sense_velocity_m_s": tesseract::SENSE_PLATEAU_M_S
            }
        }],
        "record_every": 100
    }));
    let body = run_trajectory(&base, &text).await;
    let dt = scalar(&parsed, "dt_s");
    let load = &parsed["segments"][0]["load"];
    let load = tesseract::TesseractLoad {
        accel_m_s2: scalar(load, "accel_m_s2"),
        omega_ext_rad_s: scalar(load, "omega_ext_rad_s"),
        sense_velocity_m_s: scalar(load, "sense_velocity_m_s"),
    };
    let mut state = tesseract::TesseractState::try_continue(
        &resonator,
        scalar(&parsed["initial"], "displacement_m"),
        scalar(&parsed["initial"], "velocity_m_s"),
        scalar(&parsed["initial"], "energy_defect_j"),
    )
    .expect("tesseract phase");
    let start = state;
    for _ in 0..STEPS {
        let mut frame = blank_frame();
        tesseract::step(&resonator, &mut state, &mut frame, load, dt).expect("tesseract step");
    }
    assert_ne!(
        state.displacement_m, start.displacement_m,
        "the drive mode did not move"
    );
    assert_landed(
        &body,
        state.residual_conservation_error,
        &tesseract_probes(&state),
        STEPS,
    );

    let tendon = cable_stretch::Tendon::dyneema();
    let (text, parsed) = wire(&json!({
        "domain": "materials",
        "card": "cable_stretch",
        "initial": { "elongation_m": 0.004, "velocity_m_s": 0.0 },
        "dt_s": dt_in_window(tendon.gate_dt_s()),
        "segments": [{ "steps": STEPS, "load": { "force_n": 900.0 } }],
        "record_every": 100
    }));
    let body = run_trajectory(&base, &text).await;
    let dt = scalar(&parsed, "dt_s");
    let load = cable_stretch::TendonLoad {
        force_n: scalar(&parsed["segments"][0]["load"], "force_n"),
    };
    let mut state = cable_stretch::TendonState::try_phase(
        &tendon,
        scalar(&parsed["initial"], "elongation_m"),
        scalar(&parsed["initial"], "velocity_m_s"),
    )
    .expect("cable phase");
    let start = state;
    for _ in 0..STEPS {
        let mut frame = blank_frame();
        cable_stretch::step(&tendon, &mut state, &mut frame, load, dt).expect("cable step");
    }
    assert_ne!(
        state.elongation_m, start.elongation_m,
        "the tendon did not move"
    );
    assert_landed(
        &body,
        state.residual_conservation_error,
        &cable_probes(&state),
        STEPS,
    );
}

#[tokio::test]
async fn multi_segment_matches_the_kernel() {
    let base = spawn_app().await;
    let node = thermal::ThermalNode::avionics_cold_plate();
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": dt_in_window(node.gate_dt_s()),
        "segments": [
            {
                "steps": 8,
                "load": { "heat_power_w": 1.0, "ambient_k": thermal::REFERENCE_TEMPERATURE_K }
            },
            {
                "steps": 7,
                "load": { "heat_power_w": 5.0, "ambient_k": thermal::REFERENCE_TEMPERATURE_K }
            }
        ],
        "record_every": 1
    });
    let (text, parsed) = wire(&request);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    let segments = [
        (
            8,
            scalar(&parsed["segments"][0]["load"], "heat_power_w"),
            scalar(&parsed["segments"][0]["load"], "ambient_k"),
        ),
        (
            7,
            scalar(&parsed["segments"][1]["load"], "heat_power_w"),
            scalar(&parsed["segments"][1]["load"], "ambient_k"),
        ),
    ];
    let (residual, probes) = thermal_loop(
        scalar(&parsed["initial"], "temperature_k"),
        &segments,
        scalar(&parsed, "dt_s"),
    );
    assert_landed(&body, residual, &probes, 15);
    frames_cover_landed(&body, 15);
}

#[tokio::test]
async fn battery_discharge_plate_ceiling_and_fuse_melt_refuse() {
    let base = spawn_app().await;

    let pack = battery_sag::LiIonPack::pack_12s();
    let soc = 0.25;
    let current = 30.0;
    let dt = 0.1;
    let mut state = battery_sag::PackState::try_soc(&pack, soc).expect("battery phase");
    let load = battery_sag::PackLoad { current_a: current };
    let mut landed = 0_u64;
    let mut last = state;
    let mut refusal = None;
    for _ in 0..20_000 {
        let mut frame = blank_frame();
        match battery_sag::step(&pack, &mut state, &mut frame, load, dt) {
            Ok(()) => {
                landed += 1;
                last = state;
            }
            Err(reject) => {
                refusal = Some(reject);
                break;
            }
        }
    }
    let reject =
        refusal.unwrap_or_else(|| panic!("battery stayed above cutoff, soc={}", state.soc));
    assert_eq!(reject, battery_sag::StepReject::UndervoltageCutoff);
    assert!(landed > 0, "cutoff on the first step");
    let request = json!({
        "domain": "energy",
        "card": "battery_sag",
        "initial": { "soc": soc },
        "dt_s": dt,
        "segments": [{ "steps": landed + 1, "load": { "current_a": current } }],
        "record_every": 1
    });
    let (text, _) = wire(&request);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    assert_eq!(body["outcome"], "refused", "{body}");
    assert_eq!(body["reject"], "UndervoltageCutoff");
    assert_eq!(body["steps_landed"], landed);
    assert!(
        body["reject_detail"]
            .as_str()
            .unwrap()
            .contains("UndervoltageCutoff")
    );
    frames_cover_landed(&body, landed);
    check_state(&body["final_state"], &battery_probes(&last));

    let plate = thermal::ThermalNode::avionics_cold_plate();
    let temperature = 397.0;
    let heat = 100.0;
    let ambient = thermal::REFERENCE_TEMPERATURE_K;
    let dt = dt_in_window(plate.gate_dt_s());
    let mut state =
        thermal::ThermalNodeState::try_phase(&plate, temperature).expect("thermal phase");
    let load = thermal::ThermalLoad {
        heat_power_w: heat,
        ambient_k: ambient,
    };
    let mut landed = 0_u64;
    let mut last = state;
    let mut refusal = None;
    for _ in 0..5_000 {
        let mut frame = blank_frame();
        match thermal::step(&plate, &mut state, &mut frame, load, dt) {
            Ok(()) => {
                landed += 1;
                last = state;
            }
            Err(reject) => {
                refusal = Some(reject);
                break;
            }
        }
    }
    let reject = refusal.unwrap_or_else(|| {
        panic!(
            "plate stayed inside the window at {} K",
            state.temperature_k
        )
    });
    assert_eq!(reject, thermal::StepReject::TemperatureOutsideCard);
    assert!(landed > 0, "ceiling on the first step");
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": temperature },
        "dt_s": dt,
        "segments": [{
            "steps": landed + 1,
            "load": { "heat_power_w": heat, "ambient_k": ambient }
        }],
        "record_every": 1
    });
    let (text, _) = wire(&request);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    assert_eq!(body["outcome"], "refused", "{body}");
    assert_eq!(body["reject"], "TemperatureOutsideCard");
    assert_eq!(body["steps_landed"], landed);
    assert!(
        body["reject_detail"]
            .as_str()
            .unwrap()
            .contains("TemperatureOutsideCard")
    );
    frames_cover_landed(&body, landed);
    check_state(&body["final_state"], &thermal_probes(&last));

    let node = grid_node::GridNode::copper_fuse_link();
    let temperature = grid_node::COPPER_MELT_K - 0.25;
    let current = 20.0;
    let dt = node.gate_dt_s().unwrap();
    let upstream = current * node.resistance_ohm(temperature);
    let ambient = grid_node::REFERENCE_TEMPERATURE_K;
    let (text, parsed) = wire(&json!({
        "domain": "energy",
        "card": "grid_node",
        "initial": {
            "current_a": current,
            "voltage_v": 0.0,
            "temperature_k": temperature
        },
        "dt_s": dt,
        "segments": [{
            "steps": 5_000,
            "load": {
                "upstream_voltage_v": upstream,
                "load_current_a": current,
                "ambient_k": ambient
            }
        }],
        "record_every": 1
    }));
    let current = scalar(&parsed["initial"], "current_a");
    let voltage = scalar(&parsed["initial"], "voltage_v");
    let temperature = scalar(&parsed["initial"], "temperature_k");
    let dt = scalar(&parsed, "dt_s");
    let upstream = scalar(&parsed["segments"][0]["load"], "upstream_voltage_v");
    let load_current = scalar(&parsed["segments"][0]["load"], "load_current_a");
    let ambient = scalar(&parsed["segments"][0]["load"], "ambient_k");
    let mut state = grid_node::GridNodeState::try_phase(&node, current, voltage, temperature)
        .expect("grid phase");
    let load = grid_node::GridLoad {
        upstream_voltage_v: upstream,
        load_current_a: load_current,
        ambient_k: ambient,
    };
    let mut landed = 0_u64;
    let mut last = state;
    let mut refusal = None;
    for _ in 0..5_000 {
        let mut frame = blank_frame();
        match grid_node::step(&node, &mut state, &mut frame, load, dt) {
            Ok(()) => {
                landed += 1;
                last = state;
            }
            Err(reject) => {
                refusal = Some(reject);
                break;
            }
        }
    }
    let reject = refusal.unwrap_or_else(|| {
        panic!(
            "fuse stayed solid at {} K, residual {}",
            state.temperature_k, state.residual_conservation_error
        )
    });
    assert_eq!(
        reject,
        grid_node::StepReject::CopperMelt,
        "first refusal was {reject:?} after {landed} steps at {} K",
        state.temperature_k
    );
    assert!(landed > 0, "melt on the first step");
    let mut request: Value = serde_json::from_str(&text).unwrap();
    request["segments"][0]["steps"] = json!(landed + 1);
    let (text, _) = wire(&request);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    assert_eq!(body["outcome"], "refused", "{body}");
    assert_eq!(body["reject"], "CopperMelt");
    assert_eq!(body["steps_landed"], landed);
    assert!(
        body["reject_detail"]
            .as_str()
            .unwrap()
            .contains("melt point"),
        "{}",
        body["reject_detail"]
    );
    frames_cover_landed(&body, landed);
    check_state(&body["final_state"], &grid_probes(&last));
}

#[tokio::test]
async fn ti64_yield_and_a_load_past_beta_transus_refuse() {
    let base = spawn_app().await;
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let yield_force = ti64_lattice::YIELD_STRENGTH_MPA * 1.0e6 * cell.area_m2[0];

    // A held force of 0.6 of the yield force swings the undamped node past yield.
    let request = |steps: u64| {
        json!({
            "domain": "materials",
            "card": "ti64_lattice",
            "initial": {
                "position_m": [0.0, 0.0, 0.0],
                "velocity_m_s": [0.0, 0.0, 0.0],
                "start_t_s": 0.0, "start_force_n": [0.0, 0.0, 0.0], "start_temperature_k": 293.15, "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0
            },
            "dt_s": dt_in_window(cell.gate_dt_s()),
            "segments": [{
                "steps": steps,
                "load": {
                    "force_n": [0.6 * yield_force, 0.0, 0.0],
                    "torque_nm": [0.0, 0.0, 0.0],
                    "temperature_k": ti64_lattice::REFERENCE_TEMPERATURE_K
                }
            }],
            "record_every": 1
        })
    };
    let (_, parsed) = wire(&request(1));
    let dt = scalar(&parsed, "dt_s");
    let load = &parsed["segments"][0]["load"];
    let load = ti64_lattice::Ti64Load {
        force_n: vec3(load, "force_n"),
        torque_nm: vec3(load, "torque_nm"),
        temperature_k: scalar(load, "temperature_k"),
    };
    let start = ti64_lattice::Ti64LatticeState::try_phase(
        &cell,
        vec3(&parsed["initial"], "position_m"),
        vec3(&parsed["initial"], "velocity_m_s"),
    )
    .expect("ti64 phase");
    let (landed, last, refusal) = land_until_refused(start, 5_000, |state| {
        let mut frame = blank_frame();
        ti64_lattice::step(&cell, state, &mut frame, load, dt).map_err(Refusal::Reject)
    });
    assert_eq!(
        refusal,
        Refusal::Reject(ti64_lattice::StepReject::YieldReached)
    );
    assert!(landed > 1, "yield on step {}", landed + 1);
    assert!(last.von_mises_mpa <= ti64_lattice::YIELD_STRENGTH_MPA);
    let (text, _) = wire(&request(landed + 1));
    let body = run_trajectory(&base, &text).await;
    assert_refused(
        &body,
        Some("YieldReached"),
        "Physical Failure: a Ti-6Al-4V ligament yielded: stress reached the yield strength of 880 MPa (YieldReached). That state is not sealed.",
        landed,
        last.residual_conservation_error,
        &ti64_probes(&last),
    );

    // The second segment holds the bath one kelvin above the beta transus.
    const WARM_STEPS: u64 = 300;
    let (text, parsed) = wire(&json!({
        "domain": "materials",
        "card": "ti64_lattice",
        "initial": {
            "position_m": [0.0, 0.0, 0.0],
            "velocity_m_s": [0.0, 0.0, 0.0],
            "start_t_s": 0.0, "start_force_n": [0.0, 0.0, 0.0], "start_temperature_k": 293.15, "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0
        },
        "dt_s": dt_in_window(cell.gate_dt_s()),
        "segments": [
            {
                "steps": WARM_STEPS,
                "load": {
                    "force_n": [0.2 * yield_force, 0.0, 0.0],
                    "torque_nm": [0.0, 0.0, 0.0],
                    "temperature_k": 400.0
                }
            },
            {
                "steps": WARM_STEPS,
                "load": {
                    "force_n": [0.2 * yield_force, 0.0, 0.0],
                    "torque_nm": [0.0, 0.0, 0.0],
                    "temperature_k": ti64_lattice::BETA_TRANSUS_K + 1.0
                }
            }
        ],
        "record_every": 1
    }));
    let dt = scalar(&parsed, "dt_s");
    let loads: Vec<ti64_lattice::Ti64Load> = (0..2)
        .map(|index| {
            let load = &parsed["segments"][index]["load"];
            ti64_lattice::Ti64Load {
                force_n: vec3(load, "force_n"),
                torque_nm: vec3(load, "torque_nm"),
                temperature_k: scalar(load, "temperature_k"),
            }
        })
        .collect();
    let start = ti64_lattice::Ti64LatticeState::try_phase(
        &cell,
        vec3(&parsed["initial"], "position_m"),
        vec3(&parsed["initial"], "velocity_m_s"),
    )
    .expect("ti64 phase");
    let mut taken = 0_u64;
    let (landed, last, refusal) = land_until_refused(start, 2 * WARM_STEPS, |state| {
        let load = loads[(taken / WARM_STEPS) as usize];
        taken += 1;
        let mut frame = blank_frame();
        ti64_lattice::step(&cell, state, &mut frame, load, dt).map_err(Refusal::Reject)
    });
    assert_eq!(
        refusal,
        Refusal::Reject(ti64_lattice::StepReject::TemperatureOutsideCard)
    );
    assert_eq!(landed, WARM_STEPS);
    // The kernel still refuses the hot segment when it is reached. The
    // gateway does not run it: a load outside the published span of the card
    // is refused before a permit, in whatever segment it stands, and nothing
    // is sealed.
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 422, "{response}");
    assert_eq!(
        response,
        format!(
            "Physical Failure: 'temperature_k' {} K is above {} K, the inclusive maximum of the ti64_lattice card (TemperatureOutsideCard). That state is not sealed.",
            protocol_mcp_gateway::message_number(ti64_lattice::BETA_TRANSUS_K + 1.0),
            protocol_mcp_gateway::message_number(ti64_lattice::BETA_TRANSUS_K)
        )
    );
    assert!(!response.contains("run_seal"), "{response}");
    let _ = (landed, last);
}

#[tokio::test]
async fn loss_of_synchronism_refuses() {
    let base = spawn_app().await;
    let machine = swing::SwingMachine::synchronous_60hz();
    // A dispatch above P_max has no equilibrium angle.
    let request = |steps: u64| {
        json!({
            "domain": "energy",
            "card": "swing",
            "initial": {
                "delta_rad": 1.0,
                "delta_omega_rad_s": 0.0,
                "pll_err_rad": 0.0
            },
            "dt_s": dt_in_window(machine.gate_dt_s()),
            "segments": [{ "steps": steps, "load": { "p_mech_pu": 1.5 } }],
            "record_every": 1
        })
    };
    let (_, parsed) = wire(&request(1));
    let dt = scalar(&parsed, "dt_s");
    let load = swing::SwingLoad {
        p_mech_pu: scalar(&parsed["segments"][0]["load"], "p_mech_pu"),
    };
    assert!(load.p_mech_pu > swing::P_MAX_PU);
    let start = swing::SwingState::try_dispatched(
        &machine,
        scalar(&parsed["initial"], "delta_rad"),
        scalar(&parsed["initial"], "delta_omega_rad_s"),
        scalar(&parsed["initial"], "pll_err_rad"),
        load.p_mech_pu,
    )
    .expect("swing phase");
    let (landed, last, refusal) = land_until_refused(start, 9_000, |state| {
        let mut frame = blank_frame();
        swing::step(&machine, state, &mut frame, load, dt).map_err(Refusal::Reject)
    });
    assert_eq!(
        refusal,
        Refusal::Reject(swing::StepReject::LossOfSynchronism)
    );
    assert!(landed > 1, "lost synchronism on step {}", landed + 1);
    assert!(last.delta_rad < std::f64::consts::FRAC_PI_2);
    let (text, _) = wire(&request(landed + 1));
    let body = run_trajectory(&base, &text).await;
    assert_refused(
        &body,
        Some("LossOfSynchronism"),
        "Physical Failure: the rotor landed at or beyond +/- pi/2 (LossOfSynchronism). That state is not sealed.",
        landed,
        last.residual_conservation_error,
        &swing_probes(&last),
    );
}

#[tokio::test]
async fn cavitation_refuses() {
    let base = spawn_app().await;
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let dt = dt_in_window(column.gate_dt_s());
    // Each step draws the volume of a 0.1 micrometre slice of the bore.
    let draw = -column.area_m2() * 1.0e-7 / dt;
    let request = |steps: u64| {
        json!({
            "domain": "materials",
            "card": "hydraulic_fluid",
            "initial": { "pressure_pa": hydraulic_fluid::REFERENCE_PRESSURE_PA },
            "dt_s": dt,
            "segments": [{ "steps": steps, "load": { "flow_m3_s": draw } }],
            "record_every": 1
        })
    };
    let (_, parsed) = wire(&request(1));
    let dt = scalar(&parsed, "dt_s");
    let load = hydraulic_fluid::FluidLoad {
        flow_m3_s: scalar(&parsed["segments"][0]["load"], "flow_m3_s"),
    };
    let start = hydraulic_fluid::FluidState::try_pressure(
        &column,
        scalar(&parsed["initial"], "pressure_pa"),
    )
    .expect("hydraulic phase");
    let (landed, last, refusal) = land_until_refused(start, 9_000, |state| {
        let mut frame = blank_frame();
        hydraulic_fluid::step(&column, state, &mut frame, load, dt).map_err(Refusal::Reject)
    });
    assert_eq!(
        refusal,
        Refusal::Reject(hydraulic_fluid::StepReject::Cavitation)
    );
    assert!(landed > 1, "cavitation on step {}", landed + 1);
    assert!(last.pressure_pa >= hydraulic_fluid::VAPOR_PRESSURE_PA);
    let (text, _) = wire(&request(landed + 1));
    let body = run_trajectory(&base, &text).await;
    assert_refused(
        &body,
        Some("Cavitation"),
        "Physical Failure: the column fell below vapor pressure (Cavitation). That state is not sealed.",
        landed,
        last.residual_conservation_error,
        &hydraulic_probes(&last),
    );
}

#[tokio::test]
async fn overvoltage_cutoff_refuses_a_charging_run() {
    let base = spawn_app().await;
    let pack = battery_sag::LiIonPack::pack_12s();
    let dt = 0.1;
    let request = |steps: u64| {
        json!({
            "domain": "energy",
            "card": "battery_sag",
            "initial": { "soc": 0.8 },
            "dt_s": dt,
            "segments": [{ "steps": steps, "load": { "current_a": -30.0 } }],
            "record_every": 1
        })
    };
    let load = battery_sag::PackLoad { current_a: -30.0 };
    let start = battery_sag::PackState::try_soc(&pack, 0.8).expect("battery phase");
    let (landed, last, refusal) = land_until_refused(start, 20_000, |state| {
        let mut frame = blank_frame();
        battery_sag::step(&pack, state, &mut frame, load, dt).map_err(Refusal::Reject)
    });
    assert_eq!(
        refusal,
        Refusal::Reject(battery_sag::StepReject::OvervoltageCutoff)
    );
    assert!(landed > 1, "overvoltage on step {}", landed + 1);
    assert!(last.terminal_v <= battery_sag::PACK_CHARGE_LIMIT_V);
    let (text, _) = wire(&request(landed + 1));
    let body = run_trajectory(&base, &text).await;
    assert_refused(
        &body,
        Some("OvervoltageCutoff"),
        "Physical Failure: the pack is above the 50.4 V charging limit (OvervoltageCutoff). That state is not sealed.",
        landed,
        last.residual_conservation_error,
        &battery_probes(&last),
    );
}

#[tokio::test]
async fn pressure_above_the_card_refuses_a_compressing_run() {
    let base = spawn_app().await;
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let dt = dt_in_window(column.gate_dt_s());
    // Each step pushes in the volume of a 3 micrometre slice of the bore.
    let push = column.area_m2() * 3.0e-6 / dt;
    let request = |steps: u64| {
        json!({
            "domain": "materials",
            "card": "hydraulic_fluid",
            "initial": { "pressure_pa": hydraulic_fluid::REFERENCE_PRESSURE_PA },
            "dt_s": dt,
            "segments": [{ "steps": steps, "load": { "flow_m3_s": push } }],
            "record_every": 1
        })
    };
    let (_, parsed) = wire(&request(1));
    let dt = scalar(&parsed, "dt_s");
    let load = hydraulic_fluid::FluidLoad {
        flow_m3_s: scalar(&parsed["segments"][0]["load"], "flow_m3_s"),
    };
    let start = hydraulic_fluid::FluidState::try_pressure(
        &column,
        scalar(&parsed["initial"], "pressure_pa"),
    )
    .expect("hydraulic phase");
    let (landed, last, refusal) = land_until_refused(start, 9_000, |state| {
        let mut frame = blank_frame();
        hydraulic_fluid::step(&column, state, &mut frame, load, dt).map_err(Refusal::Reject)
    });
    assert_eq!(
        refusal,
        Refusal::Reject(hydraulic_fluid::StepReject::PressureAboveCard)
    );
    assert!(landed > 1, "pressure ceiling on step {}", landed + 1);
    assert!(last.pressure_pa <= hydraulic_fluid::PRESSURE_CEILING_PA);
    let (text, _) = wire(&request(landed + 1));
    let body = run_trajectory(&base, &text).await;
    assert_refused(
        &body,
        Some("PressureAboveCard"),
        "Physical Failure: the column pressure is above the card's ceiling of 20.64 MPa over the charge, 6.0 mm of compression (PressureAboveCard). That state is not sealed.",
        landed,
        last.residual_conservation_error,
        &hydraulic_probes(&last),
    );
}

#[tokio::test]
async fn amplitude_above_the_card_refuses_a_driven_run() {
    let base = spawn_app().await;
    let oscillator = resonance::DynamicOscillator::mount_10hz();
    let dt = dt_in_window(oscillator.gate_dt_s());
    // A held 200 m/s^2 swings the mount about 0.1 m, past the amplitude ceiling.
    let request = |steps: u64| {
        json!({
            "domain": "signal",
            "card": "resonance",
            "initial": {
                "displacement_m": 0.0,
                "velocity_m_s": 0.0,
                "t_s": 0.0,
                "energy_anchor_j": 0.0,
                "boundary_work_j": 0.0
            },
            "dt_s": dt,
            "segments": [{ "steps": steps, "load": { "accel_m_s2": 200.0 } }],
            "record_every": 1
        })
    };
    let (_, parsed) = wire(&request(1));
    let dt = scalar(&parsed, "dt_s");
    let load = resonance::OscillatorLoad {
        accel_m_s2: scalar(&parsed["segments"][0]["load"], "accel_m_s2"),
    };
    let start = resonance::OscillatorState::try_phase(&oscillator, 0.0, 0.0)
        .expect("resonance phase");
    let (landed, last, refusal) = land_until_refused(start, 9_000, |state| {
        let mut frame = blank_frame();
        resonance::step(&oscillator, state, &mut frame, load, dt).map_err(Refusal::Reject)
    });
    assert_eq!(
        refusal,
        Refusal::Reject(resonance::StepReject::EnergyAboveCard)
    );
    assert!(landed > 1, "amplitude ceiling on step {}", landed + 1);
    let (text, _) = wire(&request(landed + 1));
    let body = run_trajectory(&base, &text).await;
    assert_refused(
        &body,
        Some("EnergyAboveCard"),
        "Physical Failure: the mount's mechanical energy is above that of a free ring of twice the stroke, four stroke energies: a displacement or a velocity of either sign larger than the card takes (EnergyAboveCard). That state is not sealed.",
        landed,
        last.residual_conservation_error,
        &resonance_probes(&last),
    );
}

/// The door stops a swing dispatch beyond 2 pu and a start speed deviation
/// beyond 6 pi rad/s before any step: a 422 that names the refusal, with no
/// run and no seal.
#[tokio::test]
async fn swing_dispatch_and_speed_deviation_outside_the_card_are_422_and_not_a_run() {
    let base = spawn_app().await;
    let machine = swing::SwingMachine::synchronous_60hz();
    for (delta_omega, p_mech, name) in [
        (0.0, 2.5, "DispatchOutsideCard"),
        (20.0, 1.0, "SpeedDeviationOutsideCard"),
    ] {
        let (text, _) = wire(&json!({
            "domain": "energy",
            "card": "swing",
            "initial": {
                "delta_rad": 0.5,
                "delta_omega_rad_s": delta_omega,
                "pll_err_rad": 0.0
            },
            "dt_s": dt_in_window(machine.gate_dt_s()),
            "segments": [{ "steps": 10, "load": { "p_mech_pu": p_mech } }],
            "record_every": 1
        }));
        let (status, response) = post(&base, "/run_trajectory", &text).await;
        assert_eq!(status, 422, "{response}");
        assert!(response.contains(name), "{response}");
        assert!(!response.contains("run_seal"), "{response}");
        assert!(!response.contains("receipt_sha256"), "{response}");
    }

    // A later segment is held to the same dispatch limit.
    let (text, _) = wire(&json!({
        "domain": "energy",
        "card": "swing",
        "initial": { "delta_rad": 0.5, "delta_omega_rad_s": 0.0, "pll_err_rad": 0.0 },
        "dt_s": dt_in_window(machine.gate_dt_s()),
        "segments": [
            { "steps": 10, "load": { "p_mech_pu": 1.0 } },
            { "steps": 10, "load": { "p_mech_pu": -2.5 } }
        ],
        "record_every": 1
    }));
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 422, "{response}");
    assert!(response.contains("DispatchOutsideCard"), "{response}");
    assert!(!response.contains("run_seal"), "{response}");
}

#[tokio::test]
async fn cable_slack_and_tensile_failure_refuse() {
    let base = spawn_app().await;
    let tendon = cable_stretch::Tendon::dyneema();
    // Released from 10 mm with no force, the braid swings back through zero
    // tension. Pulled from rest at 0.6 of ultimate, it overshoots ultimate.
    let cases = [
        (
            0.010,
            0.0,
            cable_stretch::StepReject::Slack,
            "Slack",
            "Physical Failure: the braid went slack (Slack). That state is not sealed.",
        ),
        (
            0.0,
            0.6 * tendon.ultimate_tension_n,
            cable_stretch::StepReject::TensileFailure,
            "TensileFailure",
            "Physical Failure: the braid reached ultimate tension (TensileFailure). That state is not sealed.",
        ),
    ];
    for (elongation, force, expected, name, detail) in cases {
        let request = |steps: u64| {
            json!({
                "domain": "materials",
                "card": "cable_stretch",
                "initial": { "elongation_m": elongation, "velocity_m_s": 0.0 },
                "dt_s": dt_in_window(tendon.gate_dt_s()),
                "segments": [{ "steps": steps, "load": { "force_n": force } }],
                "record_every": 1
            })
        };
        let (_, parsed) = wire(&request(1));
        let dt = scalar(&parsed, "dt_s");
        let load = cable_stretch::TendonLoad {
            force_n: scalar(&parsed["segments"][0]["load"], "force_n"),
        };
        let start = cable_stretch::TendonState::try_phase(
            &tendon,
            scalar(&parsed["initial"], "elongation_m"),
            scalar(&parsed["initial"], "velocity_m_s"),
        )
        .expect("cable phase");
        let (landed, last, refusal) = land_until_refused(start, 5_000, |state| {
            let mut frame = blank_frame();
            cable_stretch::step(&tendon, state, &mut frame, load, dt).map_err(Refusal::Reject)
        });
        assert_eq!(refusal, Refusal::Reject(expected));
        assert!(landed > 1, "{name} on step {}", landed + 1);
        assert!(last.tension_n >= 0.0 && last.tension_n < tendon.ultimate_tension_n);
        let (text, _) = wire(&request(landed + 1));
        let body = run_trajectory(&base, &text).await;
        assert_refused(
            &body,
            Some(name),
            detail,
            landed,
            last.residual_conservation_error,
            &cable_probes(&last),
        );
    }
}

#[tokio::test]
async fn the_same_request_seals_the_same_and_a_load_change_does_not() {
    let base = spawn_app().await;
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": 0.1,
        "segments": [{
            "steps": 3,
            "load": {
                "heat_power_w": 1.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K
            }
        }],
        "record_every": 1
    });
    let (text, _) = wire(&request);
    let (status, first_text) = post(&base, "/run_trajectory", &text).await;
    let (again, second_text) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{first_text}");
    assert_eq!(again, 200, "{second_text}");
    let first: Value = serde_json::from_str(&first_text).unwrap();
    let second: Value = serde_json::from_str(&second_text).unwrap();
    assert_eq!(first["run_seal"], second["run_seal"]);
    assert_eq!(first_text, second_text);

    let changed = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": 0.1,
        "segments": [{
            "steps": 3,
            "load": {
                "heat_power_w": 2.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K
            }
        }],
        "record_every": 1
    });
    let (text, _) = wire(&changed);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    assert_ne!(body["run_seal"], first["run_seal"]);
}

#[tokio::test]
async fn verify_tape_accepts_a_tape_and_names_the_first_break() {
    let base = spawn_app().await;
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": 0.1,
        "segments": [{
            "steps": 3,
            "load": {
                "heat_power_w": 1.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K
            }
        }],
        "record_every": 1
    });
    let (text, _) = wire(&request);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let tape: Value = serde_json::from_str(&response).unwrap();
    assert_eq!(tape["frames"].as_array().unwrap().len(), 3);

    let (text, _) = wire(&tape);
    let (status, report_text) = post(&base, "/verify_tape", &text).await;
    assert_eq!(status, 200, "{report_text}");
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["holds"], true, "{report}");
    assert!(report["broken_link"].is_null(), "{report}");
    assert!(report["cause"].is_null(), "{report}");

    let mut flipped = tape.clone();
    let hex = flipped["frames"][1]["frame_hex"]
        .as_str()
        .unwrap()
        .to_string();
    let mut chars: Vec<char> = hex.chars().collect();
    if chars[0] == '0' && chars[1] == '0' {
        chars[0] = 'f';
        chars[1] = 'f';
    } else {
        chars[0] = '0';
        chars[1] = '0';
    }
    flipped["frames"][1]["frame_hex"] = json!(chars.into_iter().collect::<String>());
    let (text, _) = wire(&flipped);
    let (status, report_text) = post(&base, "/verify_tape", &text).await;
    assert_eq!(status, 200, "{report_text}");
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["broken_link"], 1, "{report}");
    assert_eq!(report["cause"], "chain", "{report}");

    let mut removed = tape.clone();
    removed["frames"].as_array_mut().unwrap().remove(1);
    let (text, _) = wire(&removed);
    let (status, report_text) = post(&base, "/verify_tape", &text).await;
    assert_eq!(status, 200, "{report_text}");
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["broken_link"], 1, "{report}");
    assert_eq!(report["cause"], "chain", "{report}");

    let mut edited = tape.clone();
    edited["outcome"] = json!("refused");
    let (text, _) = wire(&edited);
    let (status, report_text) = post(&base, "/verify_tape", &text).await;
    assert_eq!(status, 200, "{report_text}");
    let report: Value = serde_json::from_str(&report_text).unwrap();
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["broken_link"], 3, "{report}");
    assert_eq!(report["cause"], "run_seal", "{report}");
}

#[tokio::test]
async fn verify_tape_refuses_capital_letters_in_a_frame_by_name() {
    let base = spawn_app().await;
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": 0.1,
        "segments": [{
            "steps": 3,
            "load": { "heat_power_w": 5.0, "ambient_k": thermal::REFERENCE_TEMPERATURE_K }
        }],
        "record_every": 1
    });
    let (text, _) = wire(&request);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let tape: Value = serde_json::from_str(&response).unwrap();

    for (frame, key, digits) in [(0, "frame_hex", 128), (1, "link", 64), (2, "frame_hex", 128)] {
        let mut edited = tape.clone();
        let low = edited["frames"][frame][key].as_str().unwrap().to_string();
        assert_ne!(low, low.to_uppercase());
        edited["frames"][frame][key] = json!(low.to_uppercase());
        let (text, _) = wire(&edited);
        let (status, body) = post(&base, "/verify_tape", &text).await;
        assert_eq!(status, 422, "{body}");
        assert_eq!(
            body,
            format!("Physical Violation: 'frames[{frame}].{key}' must be {digits} lowercase hex digits.")
        );
    }

    // Anything that is not hex at all, or is the wrong length in lowercase,
    // is still a chain failure at its index, as before.
    for bad in ["zz", "ab", ""] {
        let mut edited = tape.clone();
        edited["frames"][1]["frame_hex"] = json!(bad);
        let (text, _) = wire(&edited);
        let (status, body) = post(&base, "/verify_tape", &text).await;
        assert_eq!(status, 200, "{body}");
        let report: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(report["holds"], false);
        assert_eq!(report["broken_link"], 1);
        assert_eq!(report["cause"], "chain");
    }
}

fn flip_hex(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    chars[0] = if chars[0] == '0' { '1' } else { '0' };
    chars.into_iter().collect()
}

fn reverse_objects(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort();
            keys.reverse();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(key.clone(), reverse_objects(&map[&key]));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(reverse_objects).collect()),
        other => other.clone(),
    }
}

#[tokio::test]
async fn verify_tape_names_a_state_edit() {
    let base = spawn_app().await;
    let thermal = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": 0.1,
        "segments": [{
            "steps": 2,
            "load": {
                "heat_power_w": 1.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K
            }
        }],
        "record_every": 1
    });
    let (text, _) = wire(&thermal);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let tape: Value = serde_json::from_str(&response).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert!(tape["state_digest"].is_string(), "{tape}");
    assert_eq!(
        tape["final_residual_conservation_error_hex"]
            .as_str()
            .unwrap()
            .len(),
        16
    );

    let mut scalar_hex = tape.clone();
    let hex = scalar_hex["final_state"]["temperature_k_hex"]
        .as_str()
        .unwrap()
        .to_string();
    scalar_hex["final_state"]["temperature_k_hex"] = json!(flip_hex(&hex));
    let report = tape_of(&base, &scalar_hex).await;
    assert_eq!(report["holds"], false, "{report}");
    assert!(report["broken_link"].is_null(), "{report}");
    assert_eq!(report["cause"], "state", "{report}");

    let mut flag = tape.clone();
    let derated = flag["final_state"]["derated"].as_bool().unwrap();
    flag["final_state"]["derated"] = json!(!derated);
    let report = tape_of(&base, &flag).await;
    assert_eq!(report["cause"], "state", "{report}");
    assert!(report["broken_link"].is_null(), "{report}");

    let mut final_hex = tape.clone();
    let hex = final_hex["final_residual_conservation_error_hex"]
        .as_str()
        .unwrap()
        .to_string();
    final_hex["final_residual_conservation_error_hex"] = json!(flip_hex(&hex));
    let report = tape_of(&base, &final_hex).await;
    assert_eq!(report["cause"], "state", "{report}");
    assert!(report["broken_link"].is_null(), "{report}");

    let mut max_hex = tape.clone();
    let hex = max_hex["max_residual_conservation_error_hex"]
        .as_str()
        .unwrap()
        .to_string();
    max_hex["max_residual_conservation_error_hex"] = json!(flip_hex(&hex));
    let report = tape_of(&base, &max_hex).await;
    assert_eq!(report["cause"], "state", "{report}");
    assert!(report["broken_link"].is_null(), "{report}");

    let mut digest = tape.clone();
    let hex = digest["state_digest"].as_str().unwrap().to_string();
    digest["state_digest"] = json!(flip_hex(&hex));
    let report = tape_of(&base, &digest).await;
    assert_eq!(report["cause"], "state", "{report}");
    assert!(report["broken_link"].is_null(), "{report}");

    let cell = genesis_kernel::ti64_lattice::Ti64LatticeCell::annealed_column();
    let rest = genesis_kernel::ti64_lattice::Ti64LatticeState::try_rest(&cell).unwrap();
    let lattice = json!({
        "domain": "materials",
        "card": "ti64_lattice",
        "initial": {
            "position_m": rest.position_m,
            "velocity_m_s": rest.velocity_m_s,
            "start_t_s": rest.t_s,
            "start_force_n": rest.applied_force_n,
            "start_temperature_k": rest.temperature_k,
            "start_energy_anchor_j": rest.energy_anchor_j,
            "start_boundary_work_j": rest.boundary_work_j
        },
        "dt_s": cell.gate_dt_s().unwrap().min(0.1),
        "segments": [{
            "steps": 2,
            "load": {
                "force_n": rest.applied_force_n,
                "torque_nm": rest.torque_nm,
                "temperature_k": rest.temperature_k
            }
        }],
        "record_every": 1
    });
    let (text, _) = wire(&lattice);
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    assert_eq!(status, 200, "{response}");
    let lattice_tape: Value = serde_json::from_str(&response).unwrap();
    let mut array_hex = lattice_tape.clone();
    let hex = array_hex["final_state"]["position_m_hex"][0]
        .as_str()
        .unwrap()
        .to_string();
    array_hex["final_state"]["position_m_hex"][0] = json!(flip_hex(&hex));
    let report = tape_of(&base, &array_hex).await;
    assert_eq!(report["holds"], false, "{report}");
    assert!(report["broken_link"].is_null(), "{report}");
    assert_eq!(report["cause"], "state", "{report}");
}

async fn tape_of(base: &str, body: &Value) -> Value {
    let (text, _) = wire(body);
    let (status, report_text) = post(base, "/verify_tape", &text).await;
    assert_eq!(status, 200, "{report_text}");
    let mut report: Value = serde_json::from_str(&report_text).unwrap();
    // The sentence of what was compared is one of two, by whether the run
    // was computed again; the rest of the report is held to below.
    let compared = report.as_object_mut().unwrap().shift_remove("compared");
    assert!(
        compared == Some(json!(protocol_mcp_gateway::LONG_FORM_COMPARED)) || compared == Some(json!(protocol_mcp_gateway::LONG_FORM_COMPUTED)),
        "{report_text}"
    );
    report
}

#[tokio::test]
async fn object_key_order_does_not_change_the_seals() {
    let base = spawn_app().await;
    let step = json!({
        "domain": "energy",
        "card": "thermal",
        "parameters": {
            "temperature_k": thermal::REFERENCE_TEMPERATURE_K,
            "heat_power_w": 1.0,
            "ambient_k": thermal::REFERENCE_TEMPERATURE_K,
            "dt_s": 0.1
        }
    });
    let reordered_step = reverse_objects(&step);
    assert_ne!(
        serde_json::to_string(&step).unwrap(),
        serde_json::to_string(&reordered_step).unwrap()
    );
    let (left, _) = wire(&step);
    let (right, _) = wire(&reordered_step);
    let (status, left_text) = post(&base, "/verify_physics", &left).await;
    let (again, right_text) = post(&base, "/verify_physics", &right).await;
    assert_eq!(status, 200, "{left_text}");
    assert_eq!(again, 200, "{right_text}");
    let left_body: Value = serde_json::from_str(&left_text).unwrap();
    let right_body: Value = serde_json::from_str(&right_text).unwrap();
    assert_eq!(left_body["sha256_seal"], right_body["sha256_seal"]);
    assert_ne!(
        serde_json::to_string(&left_body["inputs"]).unwrap(),
        serde_json::to_string(&right_body["inputs"]).unwrap()
    );

    let node = grid_node::GridNode::copper_fuse_link();
    let run = json!({
        "domain": "energy",
        "card": "grid_node",
        "initial": {
            "current_a": 0.0,
            "voltage_v": 0.0,
            "temperature_k": grid_node::REFERENCE_TEMPERATURE_K
        },
        "dt_s": node.gate_dt_s().unwrap().min(0.1),
        "segments": [{
            "steps": 2,
            "load": {
                "upstream_voltage_v": 0.0,
                "load_current_a": 0.0,
                "ambient_k": grid_node::REFERENCE_TEMPERATURE_K
            }
        }],
        "record_every": 1
    });
    let reordered_run = reverse_objects(&run);
    assert_ne!(
        serde_json::to_string(&run).unwrap(),
        serde_json::to_string(&reordered_run).unwrap()
    );
    let (left, _) = wire(&run);
    let (right, _) = wire(&reordered_run);
    let (status, left_text) = post(&base, "/run_trajectory", &left).await;
    let (again, right_text) = post(&base, "/run_trajectory", &right).await;
    assert_eq!(status, 200, "{left_text}");
    assert_eq!(again, 200, "{right_text}");
    let left_body: Value = serde_json::from_str(&left_text).unwrap();
    let right_body: Value = serde_json::from_str(&right_text).unwrap();
    assert_eq!(left_body["outcome"], "completed", "{left_body}");
    assert_eq!(left_body["run_seal"], right_body["run_seal"]);
    assert_eq!(left_body["frames"], right_body["frames"]);
    assert_ne!(
        serde_json::to_string(&left_body["inputs"]).unwrap(),
        serde_json::to_string(&right_body["inputs"]).unwrap()
    );
}

#[tokio::test]
async fn a_repeated_key_in_a_tape_is_refused_and_not_sealed() {
    let base = spawn_app().await;
    let body = r#"{"inputs":{"domain":"energy","domain":"energy"},"frames":[],"outcome":"completed","steps_landed":0,"reject":null,"run_seal":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","final_state":null,"final_residual_conservation_error_hex":null,"max_residual_conservation_error_hex":null,"state_digest":null}"#;
    let (status, text) = post(&base, "/verify_tape", body).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, "Physical Violation: the key 'domain' is repeated.");
    assert!(!text.contains("holds"), "{text}");
}

/// The sealed form of a request, written here from the map alone.
fn sealed_text(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => format!("x{:016x}", number.as_f64().unwrap().to_bits()),
        Value::String(text) => serde_json::to_string(text).unwrap(),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(sealed_text).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        sealed_text(&map[key])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

/// The 16 proof bytes of a recorded frame, as 32 hex characters.
fn proof_hex(frame: &Value) -> String {
    let frame_hex = frame["frame_hex"].as_str().expect("frame_hex");
    assert_eq!(frame_hex.len(), 128, "{frame_hex}");
    frame_hex[96..].to_string()
}

fn thermal_step_text(heat_power_w: &str) -> String {
    format!(
        r#"{{"domain":"energy","card":"thermal","parameters":{{"temperature_k":293.15,"heat_power_w":{heat_power_w},"ambient_k":293.15,"dt_s":0.1}}}}"#
    )
}

fn thermal_run_text(heat_power_w: &str) -> String {
    format!(
        r#"{{"domain":"energy","card":"thermal","initial":{{"temperature_k":293.15}},"dt_s":0.1,"segments":[{{"steps":3,"load":{{"heat_power_w":{heat_power_w},"ambient_k":293.15}}}}],"record_every":1}}"#
    )
}

/// The worked example in the map: the request, its sealed text, its SHA-256,
/// and every link after it, with no call to the gateway's own sealing code.
#[tokio::test]
async fn the_run_seal_is_rebuilt_from_the_response_alone() {
    let base = spawn_app().await;
    let request = r#"{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":10,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}"#;
    let tape = run_trajectory(&base, request).await;
    assert_eq!(tape["outcome"], "completed", "{tape}");

    let sealed = sealed_text(&tape["inputs"]);
    assert_eq!(
        sealed,
        r#"{"card":"thermal","domain":"energy","dt_s":x3fb999999999999a,"initial":{"temperature_k":x4072526666666666},"record_every":x3ff0000000000000,"segments":[{"load":{"ambient_k":x4072526666666666,"heat_power_w":x3ff0000000000000},"steps":x4024000000000000}]}"#
    );
    assert_eq!(sealed.len(), 253);
    let mut link = Sha256::digest(sealed.as_bytes()).to_vec();
    assert_eq!(
        hex::encode(&link),
        "dced576688495b353ee304b7b8400f3fab60536a2deeb86932cb744039703da0"
    );

    let frames = tape["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 10);
    for frame in frames {
        assert_eq!(proof_hex(frame), hex::encode(&link[..16]), "{frame}");
        let bytes = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap();
        link = Sha256::digest(&bytes).to_vec();
        assert_eq!(frame["link"], hex::encode(&link), "{frame}");
    }
    let tail = format!(
        r#"{{"outcome":"completed","steps_landed":10,"reject":null,"state_digest":"{}"}}"#,
        tape["state_digest"].as_str().unwrap()
    );
    let mut bytes = hex::encode(&link).into_bytes();
    bytes.extend_from_slice(tail.as_bytes());
    assert_eq!(tape["run_seal"], hex::encode(Sha256::digest(&bytes)));
}

#[tokio::test]
async fn zero_and_zero_point_zero_seal_the_same() {
    let base = spawn_app().await;
    for spellings in [["0", "0.0", "0e0"], ["10", "10.0", "1e1"]] {
        let mut frame_seals = Vec::new();
        let mut step_echoes = Vec::new();
        let mut run_seals = Vec::new();
        let mut run_frames = Vec::new();
        let mut run_echoes = Vec::new();
        for spelling in spellings {
            let (status, text) =
                post(&base, "/verify_physics", &thermal_step_text(spelling)).await;
            assert_eq!(status, 200, "{spelling}: {text}");
            let body: Value = serde_json::from_str(&text).unwrap();
            frame_seals.push(body["sha256_seal"].as_str().unwrap().to_string());
            step_echoes.push(serde_json::to_string(&body["inputs"]).unwrap());

            let body = run_trajectory(&base, &thermal_run_text(spelling)).await;
            assert_eq!(body["outcome"], "completed", "{spelling}: {body}");
            run_seals.push(body["run_seal"].as_str().unwrap().to_string());
            run_frames.push(body["frames"].clone());
            run_echoes.push(serde_json::to_string(&body["inputs"]).unwrap());
        }
        for index in 1..spellings.len() {
            let label = format!("{} against {}", spellings[index], spellings[0]);
            assert_eq!(frame_seals[index], frame_seals[0], "{label}");
            assert_eq!(run_seals[index], run_seals[0], "{label}");
            assert_eq!(run_frames[index], run_frames[0], "{label}");
        }
        // The whole number is still echoed as it was written.
        assert_ne!(step_echoes[0], step_echoes[1], "{spellings:?}");
        assert_ne!(run_echoes[0], run_echoes[1], "{spellings:?}");
        assert!(
            step_echoes[0].contains(&format!("\"heat_power_w\":{},", spellings[0])),
            "{}",
            step_echoes[0]
        );
    }
}

#[tokio::test]
async fn negative_zero_seals_apart_from_zero() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/verify_physics", &thermal_step_text("0.0")).await;
    assert_eq!(status, 200, "{text}");
    let zero_step: Value = serde_json::from_str(&text).unwrap();
    let zero_run = run_trajectory(&base, &thermal_run_text("0.0")).await;
    assert_eq!(zero_run["outcome"], "completed", "{zero_run}");

    let mut negative_seals = Vec::new();
    for spelling in ["-0.0", "-0", "-0e0"] {
        let (status, text) = post(&base, "/verify_physics", &thermal_step_text(spelling)).await;
        assert_eq!(status, 200, "{spelling}: {text}");
        let step: Value = serde_json::from_str(&text).unwrap();
        assert_ne!(step["sha256_seal"], zero_step["sha256_seal"], "{spelling}");

        let run = run_trajectory(&base, &thermal_run_text(spelling)).await;
        assert_eq!(run["outcome"], "completed", "{spelling}: {run}");
        // The proof of the first frame is the head of `link_0`, the hash of
        // the request alone.
        assert_ne!(
            proof_hex(&run["frames"][0]),
            proof_hex(&zero_run["frames"][0]),
            "{spelling}"
        );
        assert_ne!(run["run_seal"], zero_run["run_seal"], "{spelling}");
        negative_seals.push((
            step["sha256_seal"].as_str().unwrap().to_string(),
            run["run_seal"].as_str().unwrap().to_string(),
        ));
    }
    assert_eq!(negative_seals[1], negative_seals[0]);
    assert_eq!(negative_seals[2], negative_seals[0]);
}

/// A number token as another JSON writer prints it: `0.0` as `0`, `1.0` as
/// `1`, and `0.1` as `1e-1`. Text inside a string is left as it is.
fn respell_numbers(text: &str) -> String {
    fn flush(out: &mut String, token: &mut String) {
        out.push_str(match token.as_str() {
            "0.0" => "0",
            "1.0" => "1",
            "0.1" => "1e-1",
            other => other,
        });
        token.clear();
    }
    let mut out = String::new();
    let mut token = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for letter in text.chars() {
        if in_string {
            out.push(letter);
            if escaped {
                escaped = false;
            } else if letter == '\\' {
                escaped = true;
            } else if letter == '"' {
                in_string = false;
            }
        } else if letter.is_ascii_digit() || matches!(letter, '-' | '+' | '.' | 'e' | 'E') {
            token.push(letter);
        } else {
            flush(&mut out, &mut token);
            in_string = letter == '"';
            out.push(letter);
        }
    }
    flush(&mut out, &mut token);
    out
}

#[tokio::test]
async fn a_tape_survives_a_respelling_of_its_numbers() {
    let base = spawn_app().await;
    let request = r#"{"domain":"energy","card":"battery_sag","initial":{"soc":1.0},"dt_s":0.1,"segments":[{"steps":2,"load":{"current_a":0.0}},{"steps":2,"load":{"current_a":1.0}}],"record_every":1}"#;
    let (status, response) = post(&base, "/run_trajectory", request).await;
    assert_eq!(status, 200, "{response}");
    let tape: Value = serde_json::from_str(&response).unwrap();
    assert_eq!(tape["outcome"], "completed", "{tape}");
    assert_eq!(tape["frames"].as_array().unwrap().len(), 4);
    let (status, report) = post(&base, "/verify_tape", &response).await;
    assert_eq!(status, 200, "{report}");
    let named = |report: &str| {
        let report: Value = serde_json::from_str(report).unwrap();
        (report["holds"].clone(), report["broken_link"].clone(), report["cause"].clone(), report["differs_at"].clone())
    };
    assert_eq!(named(&report), (json!(true), Value::Null, Value::Null, Value::Null), "{report}");

    // The echoed request is the first field of the response.
    assert!(
        response.starts_with(&format!("{{\"inputs\":{request},")),
        "{response}"
    );
    let respelled = respell_numbers(request);
    assert_eq!(
        respelled,
        r#"{"domain":"energy","card":"battery_sag","initial":{"soc":1},"dt_s":1e-1,"segments":[{"steps":2,"load":{"current_a":0}},{"steps":2,"load":{"current_a":1}}],"record_every":1}"#
    );
    let posted = response.replacen(request, &respelled, 1);
    assert_ne!(posted, response);
    let (status, report) = post(&base, "/verify_tape", &posted).await;
    assert_eq!(status, 200, "{report}");
    // Every seal holds over the values. Since 0.11.0 the long form writes the
    // answer again, and a request spelled another way has another receipt:
    // the receipt is the first place named, not a seal.
    assert_eq!(named(&report), (json!(false), Value::Null, json!("answer_text"), json!("receipt_sha256")), "{report}");

    // A count written with a decimal point is the same number to a seal,
    // and a request the engine turns away: the inputs are named.
    let decimal_counts = request
        .replace("\"steps\":2,", "\"steps\":2.0,")
        .replace("\"record_every\":1}", "\"record_every\":1e0}");
    assert_ne!(decimal_counts, request);
    let posted = response.replacen(request, &decimal_counts, 1);
    let (status, report) = post(&base, "/verify_tape", &posted).await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(named(&report), (json!(false), Value::Null, json!("answer_text"), json!("inputs")), "{report}");

    // A changed number is still a broken tape.
    let changed = response.replacen("\"soc\":1.0", "\"soc\":0.5", 1);
    assert_ne!(changed, response);
    let (status, report) = post(&base, "/verify_tape", &changed).await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(named(&report), (json!(false), json!(0), json!("chain"), Value::Null), "{report}");
}

#[tokio::test]
async fn a_number_never_seals_the_same_as_a_string() {
    let base = spawn_app().await;
    let tape = run_trajectory(&base, &thermal_run_text("1.0")).await;
    assert_eq!(tape["outcome"], "completed", "{tape}");
    assert_eq!(tape["inputs"]["initial"]["temperature_k"], 293.15);
    assert_eq!(tape_of(&base, &tape).await["holds"], true);

    for forged in [
        json!("x4072526666666666"),
        json!("4072526666666666"),
        json!("293.15"),
        json!([293.15]),
        json!({ "x4072526666666666": 293.15 }),
    ] {
        let mut edited = tape.clone();
        edited["inputs"]["initial"]["temperature_k"] = forged.clone();
        let (text, _) = wire(&edited);
        let (status, report_text) = post(&base, "/verify_tape", &text).await;
        if status == 422 {
            continue;
        }
        assert_eq!(status, 200, "{forged}: {report_text}");
        let report: Value = serde_json::from_str(&report_text).unwrap();
        assert_eq!(report["holds"], false, "{forged}: {report}");
        assert_eq!(report["broken_link"], 0, "{forged}: {report}");
        assert_eq!(report["cause"], "chain", "{forged}: {report}");
    }

    // The same strings in a request never reach a seal.
    for forged in ["\"x4072526666666666\"", "\"293.15\""] {
        let run = thermal_run_text("1.0").replacen(
            "\"temperature_k\":293.15",
            &format!("\"temperature_k\":{forged}"),
            1,
        );
        let (status, text) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 422, "{forged}: {text}");
        assert!(!text.contains("run_seal"), "{text}");
        let step = thermal_step_text("1.0").replacen(
            "\"temperature_k\":293.15",
            &format!("\"temperature_k\":{forged}"),
            1,
        );
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{forged}: {text}");
        assert!(!text.contains("sha256_seal"), "{text}");
    }
}

#[tokio::test]
async fn a_count_that_is_not_a_json_integer_is_still_refused() {
    let base = spawn_app().await;
    let whole = thermal_run_text("1.0");
    for (field, written) in [
        ("steps", "3.0"),
        ("steps", "3e0"),
        ("steps", "0"),
        ("steps", "-3"),
        ("steps", "18446744073709551616"),
        ("record_every", "1.0"),
        ("record_every", "1e0"),
        ("record_every", "0"),
    ] {
        let was = if field == "steps" {
            "\"steps\":3,"
        } else {
            "\"record_every\":1}"
        };
        let now = was.replacen(|letter: char| letter.is_ascii_digit(), written, 1);
        let body = whole.replacen(was, &now, 1);
        assert_ne!(body, whole, "{field} {written}");
        let (status, text) = post(&base, "/run_trajectory", &body).await;
        assert_eq!(status, 422, "{field} {written}: {text}");
        assert_eq!(
            text,
            format!("Physical Violation: '{field}' must be an integer >= 1."),
            "{field} {written}"
        );
    }
}

#[tokio::test]
async fn the_old_key_orchestrator_is_refused_by_name() {
    let base = spawn_app().await;
    let run = thermal_run_text("1.0");
    let old = run.replacen("\"card\":", "\"orchestrator\":", 1);
    assert_ne!(old, run);
    let beside = run.replacen(
        "\"card\":\"thermal\",",
        "\"card\":\"thermal\",\"orchestrator\":\"thermal\",",
        1,
    );
    assert_ne!(beside, run);
    for body in [old, beside] {
        let (status, text) = post(&base, "/run_trajectory", &body).await;
        assert_eq!(status, 422, "{body}: {text}");
        assert_eq!(
            text,
            "Physical Violation: 'orchestrator' is not a field of this request."
        );
    }
}

#[tokio::test]
async fn an_old_domain_pair_is_not_recognized() {
    let base = spawn_app().await;
    for (domain, card) in [
        ("thermal", "thermal"),
        ("mechanical", "resonance"),
        ("inertial", "tesseract"),
        ("hydraulics", "hydraulic_fluid"),
        ("structures", "cable_stretch"),
    ] {
        let body = json!({
            "domain": domain,
            "card": card,
            "initial": {},
            "dt_s": 0.1,
            "segments": [{ "steps": 1, "load": {} }],
            "record_every": 1
        });
        let (text, _) = wire(&body);
        let (status, response) = post(&base, "/run_trajectory", &text).await;
        assert_eq!(status, 404, "{domain}/{card}: {response}");
        assert_eq!(response, "Card not recognized (UnknownCard).", "{domain}/{card}");
    }
}

/// A number in `final_state`, read from its hex.
fn state_number(body: &Value, key: &str) -> f64 {
    let hex = body["final_state"][format!("{key}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{key}_hex in {body}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

#[tokio::test]
async fn a_run_at_the_card_ceiling_lands_and_the_next_number_up_is_refused() {
    const STEPS: u64 = 8;
    let base = spawn_app().await;
    let plate = thermal::ThermalNode::avionics_cold_plate();
    let pack = battery_sag::LiIonPack::pack_12s();
    let specimen = cell_18650pf::Cell18650Pf::cell();
    let rest_k = thermal::REFERENCE_TEMPERATURE_K;
    let cases = [
        (
            "thermal",
            plate.gate_dt_s().unwrap(),
            json!({ "temperature_k": rest_k }),
            json!({ "heat_power_w": 25.0, "ambient_k": rest_k }),
        ),
        (
            "battery_sag",
            pack.gate_dt_s().unwrap(),
            json!({ "soc": 0.8 }),
            json!({ "current_a": 22.0 }),
        ),
        (
            "cell_18650pf",
            specimen.gate_dt_s().unwrap(),
            json!({ "soc": 1.0, "held_current_a": 2.9 }),
            json!({ "current_a": 2.9 }),
        ),
    ];
    for (card, ceiling, initial, load) in cases {
        assert!(ceiling > 0.1, "{card} {ceiling}");
        let request = json!({
            "domain": "energy",
            "card": card,
            "initial": initial,
            "dt_s": ceiling,
            "segments": [{ "steps": STEPS, "load": load }],
            "record_every": 1
        });
        let (text, parsed) = wire(&request);
        let dt = scalar(&parsed, "dt_s");
        assert_eq!(dt.to_bits(), ceiling.to_bits());
        let body = run_trajectory(&base, &text).await;
        let (residual, probes) = match card {
            "thermal" => thermal_loop(rest_k, &[(STEPS, 25.0, rest_k)], dt),
            "battery_sag" => battery_loop(0.8, 22.0, dt, STEPS),
            _ => cell_loop(1.0, 2.9, dt, STEPS),
        };
        assert_landed(&body, residual, &probes, STEPS);
        frames_cover_landed(&body, STEPS);
        assert_eq!(tape_of(&base, &body).await["holds"], true, "{card}");

        let above = f64::from_bits(ceiling.to_bits() + 1);
        let mut past = request.clone();
        past["dt_s"] = json!(above);
        let (text, parsed) = wire(&past);
        assert_eq!(scalar(&parsed, "dt_s").to_bits(), above.to_bits());
        let (status, response) = post(&base, "/run_trajectory", &text).await;
        assert_eq!(status, 422, "{card}: {response}");
        assert_eq!(
            response,
            format!(
                // Re-pinned for 0.10.1: a step above the ceiling carries the name DtAboveCeiling.
                "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the {card} card (DtAboveCeiling).",
                protocol_mcp_gateway::message_number(above),
                protocol_mcp_gateway::message_number(ceiling)
            )
        );
        assert!(!response.contains("run_seal"), "{response}");
    }
}

/// The plate steps by the exact solution of its heat equation, so five steps
/// of 4 s and one step of 20 s are the same exponential. Each step rounds the
/// temperature to one unit in the last place, 5.7e-14 K at 293 K, so the two
/// runs stay inside 1e-12 K. An energy is the 590.625 J/K capacitance times a
/// temperature, so the energies stay inside 1e-9 J.
#[tokio::test]
async fn thermal_steps_inside_the_ceiling_compose() {
    let base = spawn_app().await;
    let ceiling = thermal::ThermalNode::avionics_cold_plate()
        .gate_dt_s()
        .unwrap();
    assert!(20.0 < ceiling);
    let run = |steps: u64, dt_s: f64| {
        json!({
            "domain": "energy",
            "card": "thermal",
            "initial": { "temperature_k": 293.15 },
            "dt_s": dt_s,
            "segments": [{ "steps": steps, "load": { "heat_power_w": 25.0, "ambient_k": 293.15 } }],
            "record_every": steps
        })
    };
    let (text, _) = wire(&run(5, 4.0));
    let split = run_trajectory(&base, &text).await;
    let (text, _) = wire(&run(1, 20.0));
    let whole = run_trajectory(&base, &text).await;
    assert_eq!(split["outcome"], "completed", "{split}");
    assert_eq!(whole["outcome"], "completed", "{whole}");
    assert_eq!(state_number(&split, "t_s"), 20.0);
    assert_eq!(state_number(&whole, "t_s"), 20.0);
    let warmed = state_number(&whole, "temperature_k") - 293.15;
    assert!(warmed > 0.8 && warmed < 0.9, "the plate warmed by {warmed} K");
    let gap_k =
        (state_number(&split, "temperature_k") - state_number(&whole, "temperature_k")).abs();
    assert!(gap_k <= 1.0e-12, "temperature differs by {gap_k} K");
    for key in ["internal_energy_j", "boundary_work_j"] {
        let gap_j = (state_number(&split, key) - state_number(&whole, key)).abs();
        assert!(gap_j <= 1.0e-9, "{key} differs by {gap_j} J");
    }
}

/// Inside one table interval the cell's voltage and resistance are straight
/// lines in state of charge, and state of charge is a straight line in time,
/// so a step lands on the integral whatever its length. One step of 1 s and
/// ten of 0.1 s are the same integral and differ by rounding alone: ten
/// additions against one, each inside 1.1e-16 of the value. 1e-14 of the
/// value leaves room for that.
#[tokio::test]
async fn one_second_and_ten_tenths_agree_on_the_cell() {
    let base = spawn_app().await;
    let run = |steps: u64, dt_s: f64| {
        json!({
            "domain": "energy",
            "card": "cell_18650pf",
            "initial": { "soc": 1.0, "held_current_a": 2.9 },
            "dt_s": dt_s,
            "segments": [{ "steps": steps, "load": { "current_a": 2.9 } }],
            "record_every": steps
        })
    };
    let (text, _) = wire(&run(10, 0.1));
    let tenths = run_trajectory(&base, &text).await;
    let (text, _) = wire(&run(1, 1.0));
    let second = run_trajectory(&base, &text).await;
    assert_eq!(tenths["outcome"], "completed", "{tenths}");
    assert_eq!(second["outcome"], "completed", "{second}");
    assert_eq!(state_number(&second, "t_s"), 1.0);
    let drawn = 1.0 - state_number(&second, "soc");
    assert!(drawn > 2.7e-4 && drawn < 2.8e-4, "one second drew {drawn}");
    for key in [
        "t_s",
        "soc",
        "terminal_v",
        "open_circuit_v",
        "chemical_energy_j",
        "electrical_work_j",
        "joule_j",
        "cutoff_margin_v",
    ] {
        let left = state_number(&tenths, key);
        let right = state_number(&second, key);
        let gap = (left - right).abs();
        assert!(
            gap <= 1.0e-14 * right.abs(),
            "{key}: ten tenths {left}, one second {right}"
        );
    }
}

const DAY_S: f64 = 86_400.0;

fn generator_probes(state: &generator_mmrtg_f1::GeneratorState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("thermal_power_w", state.thermal_power_w),
        num("converter_fading_fraction", state.converter_fading_fraction),
        num("electrical_power_w", state.electrical_power_w),
        num("conversion_efficiency", state.conversion_efficiency),
        num("fuel_heat_j", state.fuel_heat_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("electrical_energy_j", state.electrical_energy_j),
        num("heat_rejected_j", state.heat_rejected_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("converter_fading_yield_j", state.converter_fading_yield_j),
        num("converter_account_anchor_j", state.converter_account_anchor_j),
        num("fading_electrical_energy_j", state.fading_electrical_energy_j),
        num("converter_account_defect_j", state.converter_account_defect_j),
        num("converter_account_error", state.converter_account_error),
        num("range_margin_s", state.range_margin_s),
        flag("past_fitting_span", state.past_fitting_span),
    ]
}

/// Steps of `dt` on the kernel card from this mission time. The last frame is
/// returned beside the state.
fn generator_loop(
    mission_time_s: f64,
    dt: f64,
    steps: u64,
) -> (generator_mmrtg_f1::GeneratorState, LastStateFrame64) {
    let generator = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
    let mut state = generator_mmrtg_f1::GeneratorState::try_mission_time(&generator, mission_time_s)
        .expect("generator phase");
    let mut frame = blank_frame();
    for _ in 0..steps {
        generator_mmrtg_f1::step(
            &generator,
            &mut state,
            &mut frame,
            generator_mmrtg_f1::GeneratorLoad {},
            dt,
        )
        .expect("generator step");
    }
    (state, frame)
}

fn generator_run(mission_time_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "generator_mmrtg_f1",
        "initial": { "mission_time_s": mission_time_s },
        "dt_s": DAY_S,
        "segments": [{ "steps": steps, "load": {} }],
        "record_every": record_every
    })
}

/// Every slot of the last recorded frame, apart from its proof, is the slot
/// the kernel wrote.
fn assert_last_frame(body: &Value, expected: &LastStateFrame64) {
    let frames = body["frames"].as_array().expect("frames");
    let last = frames.last().expect("a recorded frame");
    let bytes = hex::decode(last["frame_hex"].as_str().unwrap()).unwrap();
    let mut wanted = *expected;
    wanted.proof.copy_from_slice(&bytes[48..64]);
    assert_eq!(bytes, wanted.to_bytes().to_vec(), "{last}");
}

#[tokio::test]
async fn thirty_days_of_the_generator_match_the_kernel() {
    let base = spawn_app().await;
    let (text, parsed) = wire(&generator_run(0.0, 30, 1));
    assert_eq!(
        text,
        r#"{"domain":"energy","card":"generator_mmrtg_f1","initial":{"mission_time_s":0.0},"dt_s":86400.0,"segments":[{"steps":30,"load":{}}],"record_every":1}"#
    );
    assert_eq!(
        sealed_text(&parsed),
        r#"{"card":"generator_mmrtg_f1","domain":"energy","dt_s":x40f5180000000000,"initial":{"mission_time_s":x0000000000000000},"record_every":x3ff0000000000000,"segments":[{"load":{},"steps":x403e000000000000}]}"#
    );
    let body = run_trajectory(&base, &text).await;
    let (state, frame) = generator_loop(0.0, scalar(&parsed, "dt_s"), 30);
    assert_eq!(state.t_s, 30.0 * DAY_S);
    assert!(!state.past_fitting_span);
    assert_landed(
        &body,
        state.residual_conservation_error,
        &generator_probes(&state),
        30,
    );
    frames_cover_landed(&body, 30);
    assert_eq!(body["frames"].as_array().unwrap().len(), 30);
    assert_last_frame(&body, &frame);

    // The tape verifies, and a changed frame is named.
    let report = tape_of(&base, &body).await;
    assert_eq!(
        report,
        json!({ "holds": true, "values_hold": true, "broken_link": null, "cause": null })
    );
    let mut changed = body.clone();
    let hex = changed["frames"][17]["frame_hex"]
        .as_str()
        .unwrap()
        .to_string();
    changed["frames"][17]["frame_hex"] = json!(flip_hex(&hex));
    let report = tape_of(&base, &changed).await;
    assert_eq!(
        report,
        json!({ "holds": false, "values_hold": false, "broken_link": 17, "cause": "chain" })
    );
    let mut state_edit = body.clone();
    let hex = state_edit["final_state"]["converter_account_error_hex"]
        .as_str()
        .unwrap()
        .to_string();
    state_edit["final_state"]["converter_account_error_hex"] = json!(flip_hex(&hex));
    let report = tape_of(&base, &state_edit).await;
    assert_eq!(
        report,
        json!({ "holds": false, "values_hold": false, "broken_link": null, "cause": "state" })
    );

    // A run from a later day starts from the generator at that mission time.
    let (text, _) = wire(&generator_run(2_059.0 * DAY_S, 3, 1));
    let body = run_trajectory(&base, &text).await;
    let (state, frame) = generator_loop(2_059.0 * DAY_S, DAY_S, 3);
    assert!(state.past_fitting_span);
    assert_landed(
        &body,
        state.residual_conservation_error,
        &generator_probes(&state),
        3,
    );
    assert_last_frame(&body, &frame);
    let flags: Vec<u64> = body["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|recorded| {
            let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
            LastStateFrame64::from_bytes(bytes.as_slice().try_into().unwrap()).flags
        })
        .collect();
    let past = generator_mmrtg_f1::FLAG_PAST_FITTING_SPAN;
    assert_eq!(
        flags,
        vec![0, past, past],
        "day 2,060 is inside the fitting span and day 2,061 is past it"
    );
}

#[tokio::test]
async fn the_whole_generator_record_is_one_run_and_the_next_day_lands() {
    const DAYS: u64 = 4_087;
    let base = spawn_app().await;
    let (text, _) = wire(&generator_run(0.0, DAYS, 1));
    let started = std::time::Instant::now();
    let body = run_trajectory(&base, &text).await;
    println!(
        "{DAYS} daily generator steps through POST /run_trajectory: {:.1} ms",
        started.elapsed().as_secs_f64() * 1.0e3
    );
    let (state, frame) = generator_loop(0.0, DAY_S, DAYS);
    assert_eq!(state.t_s, generator_mmrtg_f1::MISSION_TIME_MAX_S);
    assert_eq!(state.range_margin_s, 0.0);
    assert_landed(
        &body,
        state.residual_conservation_error,
        &generator_probes(&state),
        DAYS,
    );
    frames_cover_landed(&body, DAYS);
    let frames = body["frames"].as_array().unwrap();
    assert_eq!(frames.len(), DAYS as usize);
    assert_eq!(frames[DAYS as usize - 1]["step"], DAYS);
    assert_last_frame(&body, &frame);
    assert_eq!(frame.t, 4_087.0 * DAY_S);
    assert_eq!(tape_of(&base, &body).await["holds"], true);

    // One more day is past the record, not past the generator: it lands, and
    // the steps before it are the same.
    let (text, _) = wire(&generator_run(0.0, DAYS + 1, 1));
    let past = run_trajectory(&base, &text).await;
    assert_eq!(past["outcome"], "completed", "{past}");
    assert_eq!(past["steps_landed"], DAYS + 1);
    // The frame of the last day is the same, all but the proof bytes.
    let physics = |answer: &Value| answer["frames"].as_array().unwrap()[DAYS as usize - 1]["frame_hex"].as_str().unwrap()[..96].to_string();
    assert_eq!(physics(&past), physics(&body));
    assert_eq!(tape_of(&base, &past).await["holds"], true);
}

#[tokio::test]
async fn a_generator_run_takes_an_empty_load_and_nothing_else() {
    let base = spawn_app().await;
    let whole = generator_run(0.0, 2, 1);

    let mut absent = whole.clone();
    absent["segments"][0]
        .as_object_mut()
        .unwrap()
        .remove("load");
    let mut loaded = whole.clone();
    loaded["segments"][0]["load"] = json!({ "bus_voltage_v": 28.0 });
    let mut powered = whole.clone();
    powered["segments"][0]["load"] = json!({ "thermal_power_w": 1953.2 });
    let mut not_an_object = whole.clone();
    not_an_object["segments"][0]["load"] = json!(null);
    let mut set_power = whole.clone();
    set_power["initial"]["thermal_power_w"] = json!(2_000.0);
    let mut set_fraction = whole.clone();
    set_fraction["initial"]["converter_fading_fraction"] = json!(1.0);
    let mut before_landing = whole.clone();
    before_landing["initial"]["mission_time_s"] = json!(-1.0);
    let mut after_the_ledger_end = whole.clone();
    after_the_ledger_end["initial"]["mission_time_s"] = json!(1.0e11);
    let early_card = "Physical Failure: 'mission_time_s' -1 s is below 0 s, the inclusive minimum of the generator_mmrtg_f1 card (MissionTimeBeforeLanding). That state is not sealed.";
    let top = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity().ledger_end_mission_time_s().unwrap();
    let late_card = format!(
        "Physical Failure: 'mission_time_s' 100000000000 s is above {} s, the inclusive maximum of the generator_mmrtg_f1 card (DecayHeatBelowLedgerResolution). That state is not sealed.",
        protocol_mcp_gateway::message_number(top)
    );
    let cases = [
        (absent, "Missing required parameter: 'load' (InputMissing)."),
        (
            loaded,
            "Physical Violation: 'bus_voltage_v' is not a field of this request.",
        ),
        (
            powered,
            "Physical Violation: 'thermal_power_w' is not a field of this request.",
        ),
        (
            not_an_object,
            "Physical Violation: 'load' must be a JSON object.",
        ),
        (
            set_power,
            "Physical Violation: 'thermal_power_w' is not a field of this request.",
        ),
        (
            set_fraction,
            "Physical Violation: 'converter_fading_fraction' is not a field of this request.",
        ),
        (before_landing, early_card),
        (after_the_ledger_end, late_card.as_str()),
    ];
    for (request, message) in cases {
        let (text, _) = wire(&request);
        let (status, response) = post(&base, "/run_trajectory", &text).await;
        assert_eq!(status, 422, "{text}: {response}");
        assert_eq!(response, message, "{text}");
        assert!(!response.contains("run_seal"), "{response}");
    }

    let (text, _) = wire(&whole);
    let body = run_trajectory(&base, &text).await;
    assert_eq!(body["outcome"], "completed", "{body}");
}

fn capsule_probes(state: &capsule_stardust::CapsuleState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("altitude_m", state.altitude_m),
        num("radius_m", state.radius_m),
        num("radial_speed_m_s", state.radial_speed_m_s),
        num("transverse_speed_m_s", state.transverse_speed_m_s),
        num("angle_rad", state.angle_rad),
        num("ground_distance_m", state.ground_distance_m),
        num("air_speed_m_s", state.air_speed_m_s),
        num("mach", state.mach),
        num("load_g", state.load_g),
        num("heat_flux_w_m2", state.heat_flux_w_m2),
        num("drag_loss_j_kg", state.drag_loss_j_kg),
        num("energy_anchor_j_kg", state.energy_anchor_j_kg),
        num("energy_defect_j_kg", state.energy_defect_j_kg),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("drogue_mach_window", state.drogue_mach_window),
    ]
}

/// What the kernel card does from entry interface at its ceiling step: every
/// landed state, the last frame, and the refusal that ended the run, if one did.
struct CapsuleFlight {
    states: Vec<capsule_stardust::CapsuleState>,
    last_frame: LastStateFrame64,
    reject: Option<capsule_stardust::StepReject>,
}

fn capsule_flight(limit: u64) -> CapsuleFlight {
    let capsule = capsule_stardust::CapsuleStardust::sample_return_capsule();
    let mut state =
        capsule_stardust::CapsuleState::try_entry_interface(&capsule).expect("entry interface");
    let mut frame = blank_frame();
    let mut states = Vec::new();
    let mut reject = None;
    for _ in 0..limit {
        match capsule_stardust::step(
            &capsule,
            &mut state,
            &mut frame,
            capsule_stardust::CapsuleLoad {},
            capsule_stardust::GATE_DT_S,
        ) {
            Ok(()) => states.push(state),
            Err(refusal) => {
                reject = Some(refusal);
                break;
            }
        }
    }
    CapsuleFlight {
        states,
        last_frame: frame,
        reject,
    }
}

/// A run from the entry state `describe_card` publishes, at the ceiling step.
fn capsule_run(steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "capsule_stardust",
        "initial": {
            "radius_m": capsule_stardust::ENTRY_RADIUS_M,
            "radial_speed_m_s": capsule_stardust::ENTRY_RADIAL_SPEED_M_S,
            "transverse_speed_m_s": capsule_stardust::ENTRY_TRANSVERSE_SPEED_M_S
        },
        "dt_s": capsule_stardust::GATE_DT_S,
        "segments": [{ "steps": steps, "load": {} }],
        "record_every": record_every
    })
}

/// Time at which a quantity that falls along the path first passes `level`,
/// by a straight line between the two samples either side of it.
fn crossing_time_s(samples: &[(f64, f64)], level: f64) -> f64 {
    for pair in samples.windows(2) {
        let (t0, y0) = pair[0];
        let (t1, y1) = pair[1];
        if y0 >= level && level > y1 {
            return t0 + (y0 - level) / (y0 - y1) * (t1 - t0);
        }
    }
    panic!("no sample passes {level}");
}

#[tokio::test]
async fn a_hundred_steps_of_the_capsule_match_the_kernel() {
    let base = spawn_app().await;
    let (text, parsed) = wire(&capsule_run(100, 1));
    assert_eq!(
        text,
        r#"{"domain":"aerospace","card":"capsule_stardust","initial":{"radius_m":6503140.0,"radial_speed_m_s":-1842.1416729435698,"transverse_speed_m_s":12767.792058801891},"dt_s":0.05,"segments":[{"steps":100,"load":{}}],"record_every":1}"#
    );
    assert_eq!(
        sealed_text(&parsed),
        r#"{"card":"capsule_stardust","domain":"aerospace","dt_s":x3fa999999999999a,"initial":{"radial_speed_m_s":xc09cc89112b64d71,"radius_m":x4158ceb900000000,"transverse_speed_m_s":x40c8efe5622ecd51},"record_every":x3ff0000000000000,"segments":[{"load":{},"steps":x4059000000000000}]}"#
    );
    let body = run_trajectory(&base, &text).await;
    let flight = capsule_flight(100);
    assert!(flight.reject.is_none());
    let state = flight.states.last().unwrap();
    assert_landed(
        &body,
        state.residual_conservation_error,
        &capsule_probes(state),
        100,
    );
    frames_cover_landed(&body, 100);
    assert_last_frame(&body, &flight.last_frame);
    assert_eq!(tape_of(&base, &body).await["holds"], true);
}

#[tokio::test]
async fn the_full_capsule_entry_ends_in_the_refusal_the_kernel_gives() {
    const ASKED: u64 = 5_000;
    let base = spawn_app().await;
    let flight = capsule_flight(ASKED);
    let landed = flight.states.len() as u64;
    // The last landed step is the first under both 31.03 km and Mach 1.23;
    // the next is refused by name. Through engine 0.6.0 the run went on to
    // 30 km, 2865 steps, with an advisory flag.
    assert_eq!(landed, 2_779);
    assert_eq!(
        flight.reject,
        Some(capsule_stardust::StepReject::PastFlightDeployment)
    );
    let last = flight.states.last().unwrap();
    assert!(last.altitude_m < capsule_stardust::FLIGHT_DROGUE_ALTITUDE_M);
    assert!(last.mach < capsule_stardust::FLIGHT_DROGUE_MACH);
    let before = &flight.states[flight.states.len() - 2];
    assert!(before.altitude_m >= capsule_stardust::FLIGHT_DROGUE_ALTITUDE_M);

    let (text, _) = wire(&capsule_run(ASKED, 1));
    let started = std::time::Instant::now();
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    println!(
        "the full capsule entry through POST /run_trajectory: {landed} steps, {:.1} ms, {} bytes",
        started.elapsed().as_secs_f64() * 1.0e3,
        response.len()
    );
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    assert_refused(
        &body,
        Some("PastFlightDeployment"),
        "Physical Failure: the state is under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, where the capsule flew on a parachute (PastFlightDeployment). That state is not sealed.",
        landed,
        last.residual_conservation_error,
        &capsule_probes(last),
    );
    assert_eq!(body["steps_requested"], ASKED);
    let frames = body["frames"].as_array().unwrap();
    assert_eq!(frames.len(), landed as usize);
    assert_last_frame(&body, &flight.last_frame);

    // The tape verifies, and a changed frame is named.
    assert_eq!(
        tape_of(&base, &body).await,
        json!({ "holds": true, "values_hold": true, "broken_link": null, "cause": null })
    );
    let mut changed = body.clone();
    let hex = changed["frames"][1_188]["frame_hex"]
        .as_str()
        .unwrap()
        .to_string();
    changed["frames"][1_188]["frame_hex"] = json!(flip_hex(&hex));
    assert_eq!(
        tape_of(&base, &changed).await,
        json!({ "holds": false, "values_hold": false, "broken_link": 1_188, "cause": "chain" })
    );

    // The quantities the comparison with the flight reads are in every
    // recorded frame: time as a 64-bit number, and altitude, Mach number and
    // load as 32-bit numbers. Read from the frames alone they agree with the
    // 64-bit states to well inside what the comparison needs.
    let mut altitude = Vec::new();
    let mut mach = Vec::new();
    let mut peak_load_g = 0.0_f64;
    let mut peak_at_s = 0.0;
    for (recorded, state) in frames.iter().zip(&flight.states) {
        let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
        let frame = LastStateFrame64::from_bytes(bytes.as_slice().try_into().unwrap());
        assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
        altitude.push((frame.t, f64::from(frame.pos[0])));
        mach.push((frame.t, f64::from(frame.pos[2])));
        if f64::from(frame.force_torque) > peak_load_g {
            peak_load_g = f64::from(frame.force_torque);
            peak_at_s = frame.t;
        }
    }
    let exact_altitude: Vec<(f64, f64)> = flight
        .states
        .iter()
        .map(|state| (state.t_s, state.altitude_m))
        .collect();
    let exact_mach: Vec<(f64, f64)> = flight
        .states
        .iter()
        .map(|state| (state.t_s, state.mach))
        .collect();
    let exact_peak_g = flight
        .states
        .iter()
        .map(|state| state.load_g)
        .fold(0.0, f64::max);
    assert!(peak_load_g > 32.1 && peak_load_g < 32.2, "{peak_load_g}");
    assert!(peak_at_s > 59.0 && peak_at_s < 60.0, "{peak_at_s}");
    assert!(
        (peak_load_g - exact_peak_g).abs() < 1.0e-5,
        "peak load from frames {peak_load_g}, from states {exact_peak_g}"
    );
    let floor_m = capsule_stardust::FLIGHT_DROGUE_ALTITUDE_M;
    let drogue_mach = capsule_stardust::FLIGHT_DROGUE_MACH;
    let to_altitude_s = crossing_time_s(&altitude, floor_m);
    let to_mach_s = crossing_time_s(&mach, drogue_mach);
    assert!(to_altitude_s > 138.9 && to_altitude_s < 139.0, "{to_altitude_s}");
    assert!(to_mach_s > 136.9 && to_mach_s < 137.0, "{to_mach_s}");
    assert!(
        (to_altitude_s - crossing_time_s(&exact_altitude, floor_m)).abs() < 1.0e-4,
        "time to 31.03 km from frames {to_altitude_s}"
    );
    assert!(
        (to_mach_s - crossing_time_s(&exact_mach, drogue_mach)).abs() < 1.0e-4,
        "time to Mach 1.23 from frames {to_mach_s}"
    );
    println!(
        "from frames: peak load {peak_load_g} g at {peak_at_s} s (states {exact_peak_g} g), 31.03 km at {to_altitude_s} s (states {}), Mach 1.23 at {to_mach_s} s (states {})",
        crossing_time_s(&exact_altitude, floor_m),
        crossing_time_s(&exact_mach, drogue_mach)
    );
}

#[tokio::test]
async fn a_capsule_run_takes_an_empty_load_and_nothing_else() {
    const ALTITUDE: &str = "Physical Failure: altitude is above 134.7 km, the top of the drag table read by altitude (AltitudeOutsideCard). That state is not sealed.";
    const MACH: &str = "Physical Failure: the Mach number is below 0.16, the first entry of the drag table read by Mach number, or is above 40 below 68.96 km, where that table ends (MachOutsideCard). That state is not sealed.";
    const AIR_SPEED: &str = "Physical Failure: above 68.96 km the speed through the air is below 11900 m/s or above 12620 m/s, the highest speed the drag table by altitude was computed at (AirSpeedOutsideCard). That state is not sealed.";
    const PAST: &str = "Physical Failure: the state is under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, where the capsule flew on a parachute (PastFlightDeployment). That state is not sealed.";
    let base = spawn_app().await;
    let whole = capsule_run(2, 1);
    let earth_m = capsule_stardust::LOCAL_EARTH_RADIUS_M;

    let mut absent = whole.clone();
    absent["segments"][0]
        .as_object_mut()
        .unwrap()
        .remove("load");
    let mut loaded = whole.clone();
    loaded["segments"][0]["load"] = json!({ "wind_m_s": 10.0 });
    let mut lifted = whole.clone();
    lifted["segments"][0]["load"] = json!({ "angle_of_attack_rad": 0.1 });
    let mut weighed = whole.clone();
    weighed["initial"]["capsule_mass_kg"] = json!(45.8);
    let mut tabled = whole.clone();
    tabled["initial"]["drag_by_altitude"] = json!([1.5, 1.6]);
    let mut short = whole.clone();
    short["initial"]
        .as_object_mut()
        .unwrap()
        .remove("transverse_speed_m_s");
    let mut above = whole.clone();
    above["initial"]["radius_m"] = json!(earth_m + 134_701.0);
    let mut below = whole.clone();
    below["initial"]["radius_m"] = json!(earth_m + 29_999.0);
    let mut slow = whole.clone();
    slow["initial"] = json!({
        "radius_m": earth_m + 40_000.0,
        "radial_speed_m_s": -20.0,
        "transverse_speed_m_s": capsule_stardust::AIR_ROTATION_RATE_RAD_S * (earth_m + 40_000.0)
    });
    // Three starts that engine 0.6.0 sealed: 60 km at 20 km/s (Mach 62), at
    // rest in the fixed frame at 100 km, and 30.5 km at Mach 1.14.
    let mut fast = whole.clone();
    fast["initial"] = json!({
        "radius_m": earth_m + 60_000.0,
        "radial_speed_m_s": -2000.0,
        "transverse_speed_m_s": 20000.0
    });
    let mut at_rest = whole.clone();
    at_rest["initial"] = json!({
        "radius_m": earth_m + 100_000.0,
        "radial_speed_m_s": 0.0,
        "transverse_speed_m_s": 0.0
    });
    let mut parachute = whole.clone();
    parachute["initial"] = json!({
        "radius_m": 6399728.511478454,
        "radial_speed_m_s": -200.0,
        "transverse_speed_m_s": 626.8541052549
    });
    // Off the corridor the locked entry flew: 40 km at the entry speed,
    // which engine 0.6.0 took and the largest step refused on the ledger,
    // and upward at the entry's air speed above 68.96 km.
    let mut deep = whole.clone();
    deep["initial"] = json!({
        "radius_m": earth_m + 40_000.0,
        "radial_speed_m_s": capsule_stardust::ENTRY_RADIAL_SPEED_M_S,
        "transverse_speed_m_s": capsule_stardust::ENTRY_TRANSVERSE_SPEED_M_S
    });
    let mut climbing = whole.clone();
    climbing["initial"] = json!({
        "radius_m": earth_m + 100_000.0,
        "radial_speed_m_s": -capsule_stardust::ENTRY_RADIAL_SPEED_M_S,
        "transverse_speed_m_s": capsule_stardust::ENTRY_TRANSVERSE_SPEED_M_S
    });
    let mut long_step = whole.clone();
    let past = f64::from_bits(capsule_stardust::GATE_DT_S.to_bits() + 1);
    long_step["dt_s"] = json!(past);
    let ceiling_text = format!(
        // Re-pinned for 0.10.1: a step above the ceiling carries the name DtAboveCeiling.
        "Physical Violation: 'dt_s' {past} is above 0.05 s, the step ceiling of the capsule_stardust card (DtAboveCeiling)."
    );
    let cases = [
        (absent, "Missing required parameter: 'load' (InputMissing)."),
        (
            loaded,
            "Physical Violation: 'wind_m_s' is not a field of this request.",
        ),
        (
            lifted,
            "Physical Violation: 'angle_of_attack_rad' is not a field of this request.",
        ),
        (
            weighed,
            "Physical Violation: 'capsule_mass_kg' is not a field of this request.",
        ),
        (
            tabled,
            "Physical Violation: 'drag_by_altitude' is not a field of this request.",
        ),
        (
            short,
            "Missing required parameter: 'transverse_speed_m_s' (InputMissing).",
        ),
        (above, ALTITUDE),
        // 29 999 m at the entry's speeds: Mach 42, past the Mach table.
        (below, MACH),
        (slow, MACH),
        (fast, MACH),
        (at_rest, AIR_SPEED),
        (parachute, PAST),
        (long_step, ceiling_text.as_str()),
    ];
    // Off the corridor the locked entry flew, where engines 0.7 to 0.9
    // refused, the two runs are computed.
    for request in [&deep, &climbing] {
        let (text, _) = wire(request);
        let (status, response) = post(&base, "/run_trajectory", &text).await;
        assert_eq!(status, 200, "{text}: {response}");
    }
    for (request, message) in cases {
        let (text, _) = wire(&request);
        let (status, response) = post(&base, "/run_trajectory", &text).await;
        assert_eq!(status, 422, "{text}: {response}");
        assert_eq!(response, message, "{text}");
        assert!(!response.contains("run_seal"), "{response}");
    }

    let (text, _) = wire(&whole);
    let body = run_trajectory(&base, &text).await;
    assert_eq!(body["outcome"], "completed", "{body}");
}

#[tokio::test]
async fn a_run_at_the_card_floor_lands_and_the_next_number_down_is_refused() {
    const STEPS: u64 = 8;
    let base = spawn_app().await;
    let rest_k = thermal::REFERENCE_TEMPERATURE_K;
    let cases = [
        (
            "thermal",
            thermal::ThermalNode::avionics_cold_plate()
                .floor_dt_s()
                .unwrap(),
            json!({ "temperature_k": rest_k }),
            json!({ "heat_power_w": 25.0, "ambient_k": rest_k }),
        ),
        (
            "battery_sag",
            battery_sag::LiIonPack::pack_12s().floor_dt_s().unwrap(),
            json!({ "soc": 0.8 }),
            json!({ "current_a": 22.0 }),
        ),
        (
            "cell_18650pf",
            cell_18650pf::Cell18650Pf::cell().floor_dt_s().unwrap(),
            json!({ "soc": 1.0, "held_current_a": 2.9 }),
            json!({ "current_a": 2.9 }),
        ),
        (
            "generator_mmrtg_f1",
            generator_mmrtg_f1::GeneratorMmrtgF1::curiosity()
                .floor_dt_s()
                .unwrap(),
            json!({ "mission_time_s": 4_086.0 * DAY_S }),
            json!({}),
        ),
    ];
    for (card, floor, initial, load) in cases {
        let request = json!({
            "domain": "energy",
            "card": card,
            "initial": initial,
            "dt_s": floor,
            "segments": [{ "steps": STEPS, "load": load }],
            "record_every": 1
        });
        let (text, parsed) = wire(&request);
        let dt = scalar(&parsed, "dt_s");
        assert_eq!(dt.to_bits(), floor.to_bits());
        let body = run_trajectory(&base, &text).await;
        let (residual, probes) = match card {
            "thermal" => thermal_loop(rest_k, &[(STEPS, 25.0, rest_k)], dt),
            "battery_sag" => battery_loop(0.8, 22.0, dt, STEPS),
            "cell_18650pf" => cell_loop(1.0, 2.9, dt, STEPS),
            _ => {
                let (state, _) = generator_loop(4_086.0 * DAY_S, dt, STEPS);
                (state.residual_conservation_error, generator_probes(&state))
            }
        };
        assert_landed(&body, residual, &probes, STEPS);
        frames_cover_landed(&body, STEPS);
        assert_eq!(tape_of(&base, &body).await["holds"], true, "{card}");

        let below = f64::from_bits(floor.to_bits() - 1);
        let mut under = request.clone();
        under["dt_s"] = json!(below);
        let (text, parsed) = wire(&under);
        assert_eq!(scalar(&parsed, "dt_s").to_bits(), below.to_bits());
        let (status, response) = post(&base, "/run_trajectory", &text).await;
        assert_eq!(status, 422, "{card}: {response}");
        assert_eq!(
            response,
            format!(
                // Re-pinned for 0.10.1: a step below the floor carries the name DtBelowFloor.
                "Physical Violation: 'dt_s' {} is below {} s, the step floor of the {card} card (DtBelowFloor).",
                protocol_mcp_gateway::message_number(below),
                protocol_mcp_gateway::message_number(floor)
            )
        );
        assert!(!response.contains("run_seal"), "{response}");
    }
}

/// 2026-09-06 0 h UTC, in the card's seconds: the start of the seven-day
/// run of LAGEOS-1 week 260912, as `describe_card` publishes it.
const SATELLITE_EPOCH_UTC_S: f64 = 841_924_800.0;

/// The published orbit of LAGEOS-1 at that epoch turned into the card's
/// frame, as the saved validation request gives it. The velocity is the one
/// seen from the card's turning frame.
fn satellite_start() -> ([f64; 3], [f64; 3]) {
    (
        [295558.7678903363, -5108650.370522532, -11100383.23606335],
        [-4821.08544993592, -3331.9602315669836, 1405.5539438152634],
    )
}

fn satellite_probes(state: &satellite_lageos1::SatelliteState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("epoch_utc_s", state.epoch_utc_s()),
        num("forecast_elapsed_s", state.forecast_elapsed_s()),
        vec3_probe("position_m", state.position_m),
        vec3_probe("velocity_m_s", state.velocity_m_s),
        num("radius_m", state.radius_m),
        num("semi_major_axis_m", state.semi_major_axis_m),
        num("jacobi_j_per_kg", state.jacobi_j_per_kg),
        num("jacobi_anchor_j_per_kg", state.jacobi_anchor_j_per_kg),
        num("external_work_j_per_kg", state.external_work_j_per_kg),
        num("energy_defect_j_per_kg", state.energy_defect_j_per_kg),
        num("angular_momentum_m2_s", state.angular_momentum_m2_s),
        num("angular_momentum_anchor_m2_s", state.angular_momentum_anchor_m2_s),
        num("external_torque_m2_s", state.external_torque_m2_s),
        num("angular_momentum_defect_m2_s", state.angular_momentum_defect_m2_s),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("in_earth_shadow", state.in_earth_shadow),
    ]
}

/// Steps of 10 s on the kernel card from the published start. Every landed
/// state is kept, and the last frame.
fn satellite_flight(steps: u64) -> (Vec<satellite_lageos1::SatelliteState>, LastStateFrame64) {
    let satellite = satellite_lageos1::SatelliteLageos1::lageos1();
    let (position_m, velocity_m_s) = satellite_start();
    let mut state = satellite_lageos1::SatelliteState::try_state(
        &satellite,
        SATELLITE_EPOCH_UTC_S,
        0.0,
        position_m,
        velocity_m_s,
    )
    .expect("the start is on the card");
    let mut frame = blank_frame();
    let mut states = Vec::with_capacity(steps as usize);
    for _ in 0..steps {
        satellite_lageos1::step(
            &satellite,
            &mut state,
            &mut frame,
            satellite_lageos1::SatelliteLoad {},
            satellite_lageos1::GATE_DT_S,
        )
        .expect("satellite step");
        states.push(state);
    }
    (states, frame)
}

fn satellite_run(steps: u64, record_every: u64) -> Value {
    let (position_m, velocity_m_s) = satellite_start();
    json!({
        "domain": "aerospace",
        "card": "satellite_lageos1",
        "initial": {
            "epoch_utc_s": SATELLITE_EPOCH_UTC_S,
            "forecast_elapsed_s": 0.0,
            "position_m": position_m,
            "velocity_m_s": velocity_m_s
        },
        "dt_s": satellite_lageos1::GATE_DT_S,
        "segments": [{ "steps": steps, "load": {} }],
        "record_every": record_every
    })
}

#[tokio::test]
async fn a_hundred_steps_of_the_satellite_match_the_kernel() {
    let base = spawn_app().await;
    let (text, parsed) = wire(&satellite_run(100, 1));
    assert_eq!(
        text,
        r#"{"domain":"aerospace","card":"satellite_lageos1","initial":{"epoch_utc_s":841924800.0,"forecast_elapsed_s":0.0,"position_m":[295558.7678903363,-5108650.370522532,-11100383.23606335],"velocity_m_s":[-4821.08544993592,-3331.9602315669836,1405.5539438152634]},"dt_s":10.0,"segments":[{"steps":100,"load":{}}],"record_every":1}"#
    );
    assert_eq!(
        sealed_text(&parsed),
        r#"{"card":"satellite_lageos1","domain":"aerospace","dt_s":x4024000000000000,"initial":{"epoch_utc_s":x41c9176060000000,"forecast_elapsed_s":x0000000000000000,"position_m":[x41120a1b1251d825,xc1537cea97b6a423,xc1652c1be78dd4ba],"velocity_m_s":[xc0b2d515e00c0839,xc0aa07eba378d190,x4095f6373d0c2983]},"record_every":x3ff0000000000000,"segments":[{"load":{},"steps":x4059000000000000}]}"#
    );
    let body = run_trajectory(&base, &text).await;
    let (states, frame) = satellite_flight(100);
    let last = states.last().unwrap();
    assert_landed(
        &body,
        last.residual_conservation_error,
        &satellite_probes(last),
        100,
    );
    frames_cover_landed(&body, 100);
    assert_last_frame(&body, &frame);
    assert_eq!(tape_of(&base, &body).await["holds"], true);

    // Position is rebuilt from each frame alone, by the rule the description
    // publishes: the 32-bit number in `pos` plus the remainder in `vel`,
    // added as 64-bit numbers. The card states better than 3e-8 m.
    let frames = body["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 100);
    let mut worst_m = 0.0_f64;
    for (recorded, state) in frames.iter().zip(&states) {
        let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
        let time_s = f64::from_le_bytes(bytes[0..8].try_into().unwrap());
        assert_eq!(time_s.to_bits(), state.t_s.to_bits());
        for axis in 0..3 {
            let at = 8 + 4 * axis;
            let rounded = f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            let left_over = f32::from_le_bytes(bytes[at + 12..at + 16].try_into().unwrap());
            assert!(left_over.abs() <= 0.5, "remainder {left_over}");
            let rebuilt_m = f64::from(rounded) + f64::from(left_over);
            worst_m = worst_m.max((rebuilt_m - state.position_m[axis]).abs());
            // The 32-bit number alone is up to half a metre off.
            assert!((f64::from(rounded) - state.position_m[axis]).abs() <= 0.5);
        }
        let above_reference = f32::from_le_bytes(bytes[32..36].try_into().unwrap());
        assert_eq!(
            above_reference,
            (state.radius_m - satellite_lageos1::REFERENCE_RADIUS_M) as f32
        );
        let flags = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
        assert_eq!(flags & !satellite_lageos1::FLAG_EARTH_SHADOW, 0);
    }
    println!("position rebuilt from 100 frames: worst coordinate error {worst_m:e} m");
    assert!(worst_m < 3.0e-8, "rebuilt position is {worst_m} m off");
    assert!(worst_m > 0.0, "the remainder slot is itself rounded");
}

#[tokio::test]
async fn one_revolution_of_the_satellite_lands_every_step_like_the_kernel() {
    const STEPS: u64 = 1_356;
    let base = spawn_app().await;
    let (text, _) = wire(&satellite_run(STEPS, 36));
    let started = std::time::Instant::now();
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    println!(
        "one satellite revolution through POST /run_trajectory: {STEPS} steps, {:.1} ms, {} bytes",
        started.elapsed().as_secs_f64() * 1.0e3,
        response.len()
    );
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    let (states, frame) = satellite_flight(STEPS);
    let last = states.last().unwrap();
    assert_eq!(last.t_s, 13_560.0);
    assert_landed(
        &body,
        last.residual_conservation_error,
        &satellite_probes(last),
        STEPS,
    );
    let frames = body["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 38);
    assert_eq!(frames[0]["step"], 36);
    assert_eq!(frames[37]["step"], STEPS);
    assert_last_frame(&body, &frame);
    // One revolution brings the satellite back near where it began in a
    // frame that does not turn; in the turning frame it is 52 degrees on.
    assert!((last.radius_m - satellite_lageos1::REFERENCE_RADIUS_M).abs() < 50_000.0);
    assert_eq!(tape_of(&base, &body).await["holds"], true);
}

#[tokio::test]
async fn a_week_of_the_satellite_is_one_run_and_its_tape_verifies() {
    const STEPS: u64 = 60_480;
    let base = spawn_app().await;
    let (text, _) = wire(&satellite_run(STEPS, 12));
    let started = std::time::Instant::now();
    let (status, response) = post(&base, "/run_trajectory", &text).await;
    println!(
        "one satellite week through POST /run_trajectory: {STEPS} steps, {:.1} ms, {} bytes",
        started.elapsed().as_secs_f64() * 1.0e3,
        response.len()
    );
    assert_eq!(status, 200, "{response}");
    let body: Value = serde_json::from_str(&response).unwrap();
    assert_eq!(body["outcome"], "completed", "{}", body["reject_detail"]);
    assert_eq!(body["steps_landed"], STEPS);
    let frames = body["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 5_040);
    assert_eq!(frames[5_039]["step"], STEPS);
    assert_eq!(body["final_state"]["t_s"], 604_800.0);
    let (states, frame) = satellite_flight(STEPS);
    let last = states.last().unwrap();
    assert_landed(
        &body,
        last.residual_conservation_error,
        &satellite_probes(last),
        STEPS,
    );
    assert_last_frame(&body, &frame);

    // The tape verifies, and a changed frame is named.
    assert_eq!(
        tape_of(&base, &body).await,
        json!({ "holds": true, "values_hold": true, "broken_link": null, "cause": null })
    );
    let mut changed = body.clone();
    let hex = changed["frames"][2_520]["frame_hex"]
        .as_str()
        .unwrap()
        .to_string();
    changed["frames"][2_520]["frame_hex"] = json!(flip_hex(&hex));
    assert_eq!(
        tape_of(&base, &changed).await,
        json!({ "holds": false, "values_hold": false, "broken_link": 2_520, "cause": "chain" })
    );
}

#[tokio::test]
async fn a_satellite_run_takes_an_empty_load_and_nothing_else() {
    const RADIUS: &str = "Physical Failure: distance from the Earth's centre is below 6 378.1363 km, the Earth's equatorial radius, to which the field is scaled: the satellite is under the sphere of the Earth's equator, where the card's field, the series of the Earth's field outside the Earth, is not the field (RadiusBelowEarthRadius). That state is not sealed.";
    const EPOCH_EARLY: &str = "Physical Failure: 'epoch_utc_s' 536500799 s is below 536500800 s, the inclusive minimum of the satellite_lageos1 card (EpochOutsideLeapSecondSpan). That state is not sealed.";
    const EPOCH: &str = "Physical Failure: the time is before 2017-01-01 00:00 UTC, the last leap second, or after 2027-06-28 00:00 UTC, when the IERS leap-second file of Bulletin C 72 expires: outside that span TT - UTC is not the card's 69.184 s, or is not yet known (EpochOutsideLeapSecondSpan). That state is not sealed.";
    let base = spawn_app().await;
    let epoch_late = format!(
        "Physical Failure: 'epoch_utc_s' {} s is above {} s, the end of the satellite_lageos1 card (EpochOutsideLeapSecondSpan). That state is not sealed.",
        protocol_mcp_gateway::message_number(satellite_lageos1::EPOCH_MAX_S + 1.0),
        protocol_mcp_gateway::message_number(satellite_lageos1::EPOCH_MAX_S)
    );
    let whole = satellite_run(2, 1);

    let mut absent = whole.clone();
    absent["segments"][0]
        .as_object_mut()
        .unwrap()
        .remove("load");
    let mut thrust = whole.clone();
    thrust["segments"][0]["load"] = json!({ "thrust_n": [0.0, 0.0, 1.0] });
    let mut drag = whole.clone();
    drag["segments"][0]["load"] = json!({ "drag_coefficient": 2.2 });
    let mut weighed = whole.clone();
    weighed["initial"]["satellite_mass_kg"] = json!(406.965);
    let mut regraded = whole.clone();
    regraded["initial"]["gm_earth_m3_s2"] = json!(3.986004418e14);
    let mut no_epoch = whole.clone();
    no_epoch["initial"]
        .as_object_mut()
        .unwrap()
        .remove("epoch_utc_s");
    let mut low = whole.clone();
    low["initial"]["position_m"] = json!([6.378e6, 0.0, 0.0]);
    let mut early = whole.clone();
    early["initial"]["epoch_utc_s"] = json!(satellite_lageos1::EPOCH_MIN_S - 1.0);
    let mut late = whole.clone();
    late["initial"]["epoch_utc_s"] = json!(satellite_lageos1::EPOCH_MAX_S + 1.0);
    let cases = [
        (absent, "Missing required parameter: 'load' (InputMissing)."),
        (
            thrust,
            "Physical Violation: 'thrust_n' is not a field of this request.",
        ),
        (
            drag,
            "Physical Violation: 'drag_coefficient' is not a field of this request.",
        ),
        (
            weighed,
            "Physical Violation: 'satellite_mass_kg' is not a field of this request.",
        ),
        (
            regraded,
            "Physical Violation: 'gm_earth_m3_s2' is not a field of this request.",
        ),
        (no_epoch, "Missing required parameter: 'epoch_utc_s' (InputMissing)."),
        (low, RADIUS),
        (early, EPOCH_EARLY),
        (late, epoch_late.as_str()),
    ];
    for (request, message) in cases {
        let (text, _) = wire(&request);
        let (status, response) = post(&base, "/run_trajectory", &text).await;
        assert_eq!(status, 422, "{text}: {response}");
        assert_eq!(response, message, "{text}");
        assert!(!response.contains("run_seal"), "{response}");
    }

    // A run that reaches the last date is refused there, and everything
    // before it is sealed.
    let mut to_the_end = whole.clone();
    to_the_end["initial"]["epoch_utc_s"] = json!(satellite_lageos1::EPOCH_MAX_S - 25.0);
    to_the_end["segments"][0]["steps"] = json!(5);
    let (text, _) = wire(&to_the_end);
    let body = run_trajectory(&base, &text).await;
    assert_eq!(body["outcome"], "refused", "{body}");
    assert_eq!(body["reject"], "EpochOutsideLeapSecondSpan", "{body}");
    assert_eq!(body["reject_detail"], sealed_to(EPOCH, 2));
    assert_eq!(body["steps_landed"], 2);
    assert_eq!(tape_of(&base, &body).await["holds"], true);
}

/// A swing run is anchored at the dispatch it starts with: from rest at
/// 1.0 rad under 0.9 pu, the ledger books no change of dispatch from 1.0 pu,
/// and a restart from the returned state continues the same motion with its
/// clock and ledger begun again, as `describe_card` says.
#[tokio::test]
async fn swing_ledger_is_anchored_at_the_first_dispatch_and_restarts_as_described() {
    let base = spawn_app().await;
    let machine = swing::SwingMachine::synchronous_60hz();
    let dt = machine.gate_dt_s().unwrap();
    let request = |delta: f64, omega: f64, pll: f64, steps: u64| {
        wire(&json!({
            "domain": "energy",
            "card": "swing",
            "initial": {
                "delta_rad": delta,
                "delta_omega_rad_s": omega,
                "pll_err_rad": pll
            },
            "dt_s": dt,
            "segments": [{ "steps": steps, "load": { "p_mech_pu": 0.9 } }],
            "record_every": 1
        }))
        .0
    };
    let load = swing::SwingLoad { p_mech_pu: 0.9 };
    let run_kernel = |mut state: swing::SwingState, steps: u64| {
        for _ in 0..steps {
            let mut frame = blank_frame();
            swing::step(&machine, &mut state, &mut frame, load, dt).expect("swing step");
        }
        state
    };

    // Four steps from rest at 1.0 rad, held at 0.9 pu from the first.
    let first = run_trajectory(&base, &request(1.0, 0.0, 0.0, 4)).await;
    let start = swing::SwingState::try_dispatched(&machine, 1.0, 0.0, 0.0, 0.9).unwrap();
    let four = run_kernel(start, 4);
    check_state(&first["final_state"], &swing_probes(&four));
    let anchor = first["final_state"]["energy_anchor_pu"].as_f64().unwrap();
    let u_at_0_9 = -0.9 * 1.0 - swing::P_MAX_PU * 1.0_f64.cos();
    assert!((anchor - u_at_0_9).abs() < 1.0e-12, "anchor {anchor}");
    let work = first["final_state"]["boundary_work_pu"].as_f64().unwrap();
    // Only the damping of four steps from rest: no 0.1 pu change of dispatch.
    assert!(work.abs() < 1.0e-9, "boundary work {work}");

    // The same start anchored at 1.0 pu would book the 0.1 pu jump.
    let nominal = run_kernel(swing::SwingState::try_phase(&machine, 1.0, 0.0, 0.0).unwrap(), 4);
    assert!((nominal.boundary_work_pu - 0.1).abs() < 1.0e-6);
    assert_eq!(nominal.delta_rad.to_bits(), four.delta_rad.to_bits());

    // Restart from the returned angle, speed and PLL error for four more steps.
    let again = run_trajectory(
        &base,
        &request(four.delta_rad, four.delta_omega_rad_s, four.pll_err_rad, 4),
    )
    .await;
    let eight = run_kernel(start, 8);
    let restarted = run_kernel(
        swing::SwingState::try_dispatched(
            &machine,
            four.delta_rad,
            four.delta_omega_rad_s,
            four.pll_err_rad,
            0.9,
        )
        .unwrap(),
        4,
    );
    check_state(&again["final_state"], &swing_probes(&restarted));
    assert_eq!(restarted.delta_rad.to_bits(), eight.delta_rad.to_bits());
    assert_eq!(
        restarted.delta_omega_rad_s.to_bits(),
        eight.delta_omega_rad_s.to_bits()
    );
    assert_eq!(restarted.pll_err_rad.to_bits(), eight.pll_err_rad.to_bits());
    // The clock and the ledger begin again; no jump is booked again.
    assert_eq!(restarted.t_s.to_bits(), four.t_s.to_bits());
    assert!(restarted.boundary_work_pu.abs() < 1.0e-9);

    // The description says both.
    let (status, described) = post(
        &base,
        "/describe_card",
        &json!({ "domain": "energy", "card": "swing" }).to_string(),
    )
    .await;
    assert_eq!(status, 200, "{described}");
    for sentence in [
        "A run starts at time zero with its ledger anchored at the angle, the speed and the dispatch given",
        "A first step held at that dispatch books no change of dispatch.",
        "energy_defect_pu is transient_energy_pu - energy_anchor_pu - boundary_work_pu",
        "A single step starts in the same way at each call: its delta_rad, delta_omega_rad_s and pll_err_rad handed back as a new start continue the same motion, and t_s, the anchor, the boundary work and the residual begin again.",
    ] {
        assert!(described.contains(sentence), "missing: {sentence}");
    }
}
