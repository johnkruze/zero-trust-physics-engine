//! The HTTP router seals the LastStateFrame64 the compiled step writes.
//!
//! The 16-byte proof is the input commitment, written before `step`.
//! `sha256_seal` is the frame seal: SHA-256 of all 64 frame bytes.
//! `receipt_sha256` is SHA-256 of the exact JSON of every other field.
//! A yield, a melt, or a First Law failure is 422 and has neither seal.
//! Thermodynamic constants are not request fields.

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::{
    battery_sag, cable_stretch, cantilever_eth, capsule_stardust, cell_18650pf, cell_2170_m80,
    tokamak_jet_dte1,
    cell_2170_model3, crater_ryugu_sci, generator_mmrtg_f1, grid_node,
    creep_cmsx6,
    capsule_insight,
    hydraulic_fluid, reactor_spert3, reactor_spert3_ccore, reactor_spert3_fuel, reactor_triga_jsi, resonance, satellite_lageos1, swing,
    tesseract, thermal,
    link_gothenburg_10010,
    fingerpad_p02_glass, ti64_lattice, tire_eve_r19_ice,
    accumulator_sbo450,
    grid_gb,
    solar_system,
    los_jwst_fine_guide,
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

fn request_body(domain: &str, card: &str, parameters: Value) -> String {
    serde_json::to_string(&json!({
        "domain": domain,
        "card": card,
        "parameters": parameters,
    }))
    .unwrap()
}

async fn post_json(base: &str, body: &str) -> (reqwest::StatusCode, String) {
    let client = Client::new();
    let url = format!("{base}/verify_physics");
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
                let status = response.status();
                let text = response.text().await.unwrap();
                return (status, text);
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("verify_physics never accepted a connection: {last_error:?}");
}

fn scalar(params: &Value, key: &str) -> f64 {
    params[key].as_f64().expect(key)
}

fn vec3(params: &Value, key: &str) -> [f64; 3] {
    let values = params[key].as_array().expect(key);
    [
        values[0].as_f64().unwrap(),
        values[1].as_f64().unwrap(),
        values[2].as_f64().unwrap(),
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

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn materials_parameters() -> Value {
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let force = 0.4 * ti64_lattice::YIELD_STRENGTH_MPA * 1.0e6 * cell.area_m2[0];
    json!({
        "position_m": [0.0, 0.0, 0.0],
        "velocity_m_s": [0.0, 0.0, 0.0],
        "start_t_s": 0.0,
        "start_force_n": [0.0, 0.0, 0.0],
        "start_temperature_k": ti64_lattice::REFERENCE_TEMPERATURE_K,
        "start_energy_anchor_j": 0.0,
        "start_boundary_work_j": 0.0,
        "force_n": [force, 0.0, 0.0],
        "torque_nm": [0.0, 0.0, 0.0],
        "temperature_k": ti64_lattice::REFERENCE_TEMPERATURE_K,
        "dt_s": cell.gate_dt_s().unwrap(),
    })
}

fn yielded_materials_parameters() -> Value {
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    // Three yield positions out: past yield at every temperature of the card.
    let past_yield = 3.0 * ti64_lattice::uniaxial_yield_strain() * cell.length_m[0];
    json!({
        "position_m": [past_yield, 0.0, 0.0],
        "velocity_m_s": [0.0, 0.0, 0.0],
        "start_t_s": 0.0,
        "start_force_n": [0.0, 0.0, 0.0],
        "start_temperature_k": ti64_lattice::REFERENCE_TEMPERATURE_K,
        "start_energy_anchor_j": 0.5 * cell.stiffness_n_m()[0] * past_yield * past_yield,
        "start_boundary_work_j": 0.0,
        "force_n": [0.0, 0.0, 0.0],
        "torque_nm": [0.0, 0.0, 0.0],
        "temperature_k": ti64_lattice::REFERENCE_TEMPERATURE_K,
        "dt_s": cell.gate_dt_s().unwrap(),
    })
}

fn energy_parameters(upstream_voltage_v: f64) -> Value {
    let node = grid_node::GridNode::copper_fuse_link();
    json!({
        "current_a": 0.0,
        "voltage_v": 0.0,
        "temperature_k": grid_node::REFERENCE_TEMPERATURE_K,
        "upstream_voltage_v": upstream_voltage_v,
        "load_current_a": 0.0,
        "ambient_k": grid_node::REFERENCE_TEMPERATURE_K,
        "dt_s": node.gate_dt_s().unwrap(),
    })
}

fn dt_in_window(gate: Option<f64>) -> f64 {
    gate.expect("card gate").min(0.1)
}

fn thermal_parameters() -> Value {
    let node = thermal::ThermalNode::avionics_cold_plate();
    json!({
        "temperature_k": thermal::REFERENCE_TEMPERATURE_K,
        "heat_power_w": 1.0,
        "ambient_k": thermal::REFERENCE_TEMPERATURE_K,
        "dt_s": dt_in_window(node.gate_dt_s()),
    })
}

fn derated_thermal_parameters() -> Value {
    let node = thermal::ThermalNode::avionics_cold_plate();
    json!({
        "temperature_k": thermal::DERATE_TEMPERATURE_K + 2.0,
        "heat_power_w": 0.0,
        "ambient_k": thermal::REFERENCE_TEMPERATURE_K,
        "dt_s": dt_in_window(node.gate_dt_s()),
    })
}

fn resonance_parameters() -> Value {
    let oscillator = resonance::DynamicOscillator::mount_10hz();
    json!({
        "displacement_m": 0.0,
        "velocity_m_s": 0.0,
        "t_s": 0.0,
        "energy_anchor_j": 0.0,
        "boundary_work_j": 0.0,
        "accel_m_s2": 0.0,
        "dt_s": dt_in_window(oscillator.gate_dt_s()),
    })
}

fn swing_parameters() -> Value {
    let machine = swing::SwingMachine::synchronous_60hz();
    let rest = swing::SwingState::try_rest(&machine).unwrap();
    json!({
        "delta_rad": rest.delta_rad,
        "delta_omega_rad_s": rest.delta_omega_rad_s,
        "pll_err_rad": rest.pll_err_rad,
        "p_mech_pu": swing::NOMINAL_P_MECH_PU,
        "dt_s": dt_in_window(machine.gate_dt_s()),
    })
}

/// From rest at 1.0 rad under 0.9 pu, the dispatch held from the first step:
/// the oracle anchors at 0.9 pu, so the sealed state books no change of dispatch.
fn dispatched_swing_parameters() -> Value {
    let machine = swing::SwingMachine::synchronous_60hz();
    json!({
        "delta_rad": 1.0,
        "delta_omega_rad_s": 0.0,
        "pll_err_rad": 0.0,
        "p_mech_pu": 0.9,
        "dt_s": machine.gate_dt_s().unwrap(),
    })
}

fn slipped_swing_parameters() -> Value {
    let machine = swing::SwingMachine::synchronous_60hz();
    let rest = swing::SwingState::try_rest(&machine).unwrap();
    json!({
        "delta_rad": rest.delta_rad,
        "delta_omega_rad_s": 0.0,
        "pll_err_rad": swing::PLL_TRIP_RAD + 0.05,
        "p_mech_pu": swing::NOMINAL_P_MECH_PU,
        "dt_s": dt_in_window(machine.gate_dt_s()),
    })
}

fn loss_of_synchronism_parameters() -> Value {
    let machine = swing::SwingMachine::synchronous_60hz();
    json!({
        "delta_rad": std::f64::consts::FRAC_PI_2 - 1.0e-6,
        "delta_omega_rad_s": 1.0,
        "pll_err_rad": 0.0,
        "p_mech_pu": swing::NOMINAL_P_MECH_PU,
        "dt_s": dt_in_window(machine.gate_dt_s()),
    })
}

fn tesseract_parameters() -> Value {
    let resonator = tesseract::Tesseract::drive_mode();
    json!({
        "displacement_m": 0.0,
        "velocity_m_s": 0.0,
        "energy_defect_j": 0.0,
        "accel_m_s2": 0.0,
        "omega_ext_rad_s": 0.0,
        "sense_velocity_m_s": tesseract::SENSE_PLATEAU_M_S,
        "dt_s": dt_in_window(resonator.gate_dt_s()),
    })
}

fn hydraulic_parameters() -> Value {
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    json!({
        "pressure_pa": hydraulic_fluid::REFERENCE_PRESSURE_PA,
        "flow_m3_s": 0.0,
        "dt_s": dt_in_window(column.gate_dt_s()),
    })
}

fn soft_hydraulic_parameters() -> Value {
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let pressure_pa = hydraulic_fluid::REFERENCE_PRESSURE_PA
        + hydraulic_fluid::BULK_MODULUS_PA * hydraulic_fluid::COMPRESSION_SOFT_M / column.length_m;
    json!({
        "pressure_pa": pressure_pa,
        "flow_m3_s": 0.0,
        "dt_s": dt_in_window(column.gate_dt_s()),
    })
}

fn cavitation_parameters() -> Value {
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let dt_s = dt_in_window(column.gate_dt_s());
    let dp = -1_000.0 - hydraulic_fluid::REFERENCE_PRESSURE_PA;
    let flow_m3_s = dp * column.volume_m3() / column.bulk_modulus_pa / dt_s;
    json!({
        "pressure_pa": hydraulic_fluid::REFERENCE_PRESSURE_PA,
        "flow_m3_s": flow_m3_s,
        "dt_s": dt_s,
    })
}

fn cable_parameters() -> Value {
    let tendon = cable_stretch::Tendon::dyneema();
    json!({
        "elongation_m": 0.0,
        "velocity_m_s": 0.0,
        "force_n": 0.0,
        "dt_s": dt_in_window(tendon.gate_dt_s()),
    })
}

fn slack_cable_parameters() -> Value {
    let tendon = cable_stretch::Tendon::dyneema();
    json!({
        "elongation_m": 0.0,
        "velocity_m_s": 0.0,
        "force_n": -1.0,
        "dt_s": dt_in_window(tendon.gate_dt_s()),
    })
}

fn battery_parameters() -> Value {
    let pack = battery_sag::LiIonPack::pack_12s();
    json!({
        "soc": 1.0,
        "current_a": 1.0,
        "dt_s": dt_in_window(pack.gate_dt_s()),
    })
}

fn cell_parameters() -> Value {
    let specimen = cell_18650pf::Cell18650Pf::cell();
    json!({
        "soc": 1.0,
        "held_current_a": 2.9,
        "current_a": 2.9,
        "dt_s": dt_in_window(specimen.gate_dt_s()),
    })
}

/// One second of a 4.5 A discharge in the 0 degC block, with the case
/// between the temperatures of two blocks.
fn cell_2170_parameters() -> Value {
    json!({
        "block_chamber_c": 0.0,
        "charge_removed_ah": 1.25,
        "current_a": -4.5,
        "case_temperature_c": 6.5,
        "dt_s": cell_2170_m80::Cell2170M80::cell().gate_dt_s().unwrap(),
    })
}

/// One step at the ceiling from the start of the scored ring-down, the
/// compared run's state after 595 steps: both the deflection and the speed
/// are at work.
fn cantilever_parameters() -> Value {
    json!({
        "deflection_m": -0.00104914043915228,
        "velocity_m_s": 0.7408030982848752,
        "dt_s": cantilever_eth::CantileverEth::first_bending_mode().gate_dt_s().unwrap(),
    })
}

/// One second of a 7 A discharge of the second cell, part discharged and
/// aged, with the case below freezing.
fn cell_2170_model3_parameters() -> Value {
    json!({
        "cell_number": 1000.0,
        "state_of_charge": 0.62,
        "capacity_ah": 4.25,
        "case_temperature_least_c": -3.5,
        "case_temperature_most_c": -3.5,
        "current_a": 7.0,
        "case_temperature_c": -3.5,
        "dt_s": cell_2170_model3::Cell2170Model3::cell().gate_dt_s().unwrap(),
    })
}

/// One step at the ceiling from 0.3 percent creep extension at 185 MPa.
fn creep_parameters() -> Value {
    json!({
        "creep_extension_percent": 0.3,
        "stress_mpa": 185.0,
        "dt_s": creep_cmsx6::CreepCmsx6::at_980_c().gate_dt_s().unwrap(),
    })
}

fn creep_probes(state: &creep_cmsx6::CreepState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("creep_extension_percent", state.creep_extension_percent),
        num("creep_rate_percent_per_h", state.creep_rate_percent_per_h),
        num("stress_mpa", state.stress_mpa),
        num("work_by_load_mj_per_m3", state.work_by_load_mj_per_m3),
        num("heat_dissipated_mj_per_m3", state.heat_dissipated_mj_per_m3),
        num("work_defect_mj_per_m3", state.work_defect_mj_per_m3),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("creep_rate_rising", state.creep_rate_rising),
    ]
}

/// One step at the ceiling from the middle of the growth.
fn crater_parameters() -> Value {
    json!({
        "radius_m": 4.0,
        "dt_s": crater_ryugu_sci::CraterRyuguSci::sci_impact().gate_dt_s().unwrap(),
    })
}

/// Half a second of traction at 12 percent slip, part way along the track,
/// at a load above the fitted loads.
/// A push of 0.2 m/s for 1 ms at the 80 bar condition, held back from the fit.
fn accumulator_parameters() -> Value {
    json!({
        "gas_mass_kg": 0.02747,
        "gas_pressure_pa": 8.0e6,
        "gas_temperature_k": 293.25,
        "wall_temperature_k": 293.55,
        "work_moved_j": 0.0,
        "heat_moved_j": 0.0,
        "piston_speed_m_s": 0.2,
        "dt_s": 0.001,
    })
}

fn tire_parameters() -> Value {
    json!({
        "distance_m": 2.25,
        "slip_held_s": 8.5,
        "normal_load_n": 6500.0,
        "inflation_pressure_pa": 250000.0,
        "slip_ratio": 0.12,
        "carriage_speed_m_s": 0.0625,
        "dt_s": 0.5,
    })
}

/// The first frame of the card's reference run: one measured frame of a
/// fitted file, 469 frames after the file's first centroid, from the lock
/// point of that run.
fn los_parameters() -> Value {
    json!({
        "lock_x_arcsec": 17.3039684,
        "lock_y_arcsec": -15.2342768,
        "quiet_before_start_s": 30.016,
        "measured": 1.0,
        "centroid_x_arcsec": 17.3038845,
        "centroid_y_arcsec": -15.2345047,
        "counts_per_s": 553666.4,
        "dt_s": los_jwst_fine_guide::FRAME_S,
    })
}

const DAY_S: f64 = 86_400.0;

/// One day from this mission day. The generator has no load field.
fn generator_parameters(mission_day: f64) -> Value {
    json!({
        "mission_time_s": mission_day * DAY_S,
        "dt_s": DAY_S,
    })
}

/// One step at the card's ceiling from this radius and these two speeds.
fn capsule_parameters(radius_m: f64, radial_speed_m_s: f64, transverse_speed_m_s: f64) -> Value {
    json!({
        "radius_m": radius_m,
        "radial_speed_m_s": radial_speed_m_s,
        "transverse_speed_m_s": transverse_speed_m_s,
        "dt_s": capsule_stardust::GATE_DT_S,
    })
}

/// The locked entry state, as `describe_card` publishes it.
fn capsule_entry_parameters() -> Value {
    capsule_parameters(
        capsule_stardust::ENTRY_RADIUS_M,
        capsule_stardust::ENTRY_RADIAL_SPEED_M_S,
        capsule_stardust::ENTRY_TRANSVERSE_SPEED_M_S,
    )
}

/// One step at the ceiling from the InSight capsule's locked entry state, with
/// the lift over drag measured on the flight.
fn capsule_insight_parameters() -> Value {
    json!({
        "radius_m": capsule_insight::ENTRY_RADIUS_M,
        "radial_speed_m_s": capsule_insight::ENTRY_RADIAL_SPEED_M_S,
        "transverse_speed_m_s": capsule_insight::ENTRY_TRANSVERSE_SPEED_M_S,
        "lift_over_drag": -0.031,
        "dt_s": capsule_insight::GATE_DT_S,
    })
}

/// The capsule on its locked entry path after this many steps at the ceiling.
fn capsule_after(steps: u64) -> capsule_stardust::CapsuleState {
    let capsule = capsule_stardust::CapsuleStardust::sample_return_capsule();
    let mut state = capsule_stardust::CapsuleState::try_entry_interface(&capsule).unwrap();
    let mut frame = blank_frame();
    for _ in 0..steps {
        capsule_stardust::step(
            &capsule,
            &mut state,
            &mut frame,
            capsule_stardust::CapsuleLoad {},
            capsule_stardust::GATE_DT_S,
        )
        .unwrap();
    }
    state
}

fn sag_warning_parameters() -> Value {
    let pack = battery_sag::LiIonPack::pack_12s();
    json!({
        "soc": 1.0,
        "current_a": 150.0,
        "dt_s": dt_in_window(pack.gate_dt_s()),
    })
}

fn melting_energy_parameters() -> Value {
    let node = grid_node::GridNode::copper_fuse_link();
    json!({
        "current_a": 500.0,
        "voltage_v": 0.0,
        "temperature_k": grid_node::COPPER_MELT_K - 0.5,
        "upstream_voltage_v": 0.0,
        "load_current_a": 0.0,
        "ambient_k": grid_node::REFERENCE_TEMPERATURE_K,
        "dt_s": node.gate_dt_s().unwrap(),
    })
}

#[derive(Clone, Copy)]
enum Probe {
    Num(f64),
    Vec3([f64; 3]),
    Flag(bool),
}

struct Oracle {
    frame: LastStateFrame64,
    residual: f64,
    probes: Vec<(&'static str, Probe)>,
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

fn cell_2170_probes(state: &cell_2170_m80::CellState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("block_chamber_c", state.block_chamber_c),
        num("charge_removed_ah", state.charge_removed_ah),
        num("charge_level", state.charge_level),
        num("current_a", state.current_a),
        num("case_temperature_c", state.case_temperature_c),
        num("terminal_v", state.terminal_v),
        num("open_circuit_v", state.open_circuit_v),
        num("branch_v", state.branch_v),
        num("resistance_at_once_ohm", state.resistance_at_once_ohm),
        num("resistance_branch_ohm", state.resistance_branch_ohm),
        num("time_constant_s", state.time_constant_s),
        num("chemical_energy_j", state.chemical_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("electrical_work_j", state.electrical_work_j),
        num("heat_at_once_j", state.heat_at_once_j),
        num("branch_energy_j", state.branch_energy_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("lower_limit_margin_v", state.lower_limit_margin_v),
        num("upper_limit_margin_v", state.upper_limit_margin_v),
    ]
}

/// One step at the ceiling after the loss of 996 MW on the system of the
/// card's worked example: 49.942 Hz, 241,000 MW s, 27,132 MW of demand.
fn grid_gb_parameters() -> Value {
    json!({
        "frequency_hz": 49.942,
        "inertia_mw_s": 241_000.0,
        "demand_mw": 27_132.0,
        "loss_mw": 996.0,
        "dt_s": grid_gb::GridGb::interconnector_loss().gate_dt_s().unwrap(),
    })
}

fn grid_gb_probes(state: &grid_gb::GridState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("frequency_hz", state.frequency_hz),
        num("response_mw", state.response_mw),
        num("relief_mw", state.relief_mw),
        num("net_power_mw", state.net_power_mw),
        num("frequency_rate_hz_per_s", state.frequency_rate_hz_per_s),
        num("lowest_frequency_hz", state.lowest_frequency_hz),
        num("lowest_time_s", state.lowest_time_s),
        num("start_frequency_hz", state.start_frequency_hz),
        num("inertia_mw_s", state.inertia_mw_s),
        num("demand_mw", state.demand_mw),
        num("kinetic_energy_mw_s", state.kinetic_energy_mw_s),
        num("kinetic_energy_anchor_mw_s", state.kinetic_energy_anchor_mw_s),
        num("energy_lost_mw_s", state.energy_lost_mw_s),
        num("energy_response_mw_s", state.energy_response_mw_s),
        num(
            "frequency_deficit_integral_hz_s",
            state.frequency_deficit_integral_hz_s,
        ),
        num("energy_defect_mw_s", state.energy_defect_mw_s),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
    ]
}

/// The phase fields of the solar_system card, a position and a velocity for
/// each of the thirteen bodies, in the card's own order.
const SOLAR_SYSTEM_POSITION_KEYS: [&str; solar_system::BODIES] = [
    "sun_position_m",
    "mercury_position_m",
    "venus_position_m",
    "earth_position_m",
    "moon_position_m",
    "mars_position_m",
    "jupiter_position_m",
    "saturn_position_m",
    "uranus_position_m",
    "neptune_position_m",
    "ceres_position_m",
    "pallas_position_m",
    "vesta_position_m",
];

const SOLAR_SYSTEM_VELOCITY_KEYS: [&str; solar_system::BODIES] = [
    "sun_velocity_m_s",
    "mercury_velocity_m_s",
    "venus_velocity_m_s",
    "earth_velocity_m_s",
    "moon_velocity_m_s",
    "mars_velocity_m_s",
    "jupiter_velocity_m_s",
    "saturn_velocity_m_s",
    "uranus_velocity_m_s",
    "neptune_velocity_m_s",
    "ceres_velocity_m_s",
    "pallas_velocity_m_s",
    "vesta_velocity_m_s",
];

/// One step at the ceiling from the state of the card's worked example:
/// 1999-01-01 00:00 TDB, the twenty-six vectors in the order the card
/// publishes them.
fn solar_system_parameters() -> Value {
    let mut parameters = serde_json::Map::new();
    for body in 0..solar_system::BODIES {
        parameters.insert(
            SOLAR_SYSTEM_POSITION_KEYS[body].to_string(),
            json!(solar_system::START_1999_POSITION_M[body]),
        );
        parameters.insert(
            SOLAR_SYSTEM_VELOCITY_KEYS[body].to_string(),
            json!(solar_system::START_1999_VELOCITY_M_S[body]),
        );
    }
    // The ledger of a new start: its own energy and angular momentum, nothing booked.
    let start = solar_system::SolarSystemState::try_state(
        &solar_system::SolarSystem::de440(),
        solar_system::START_1999_POSITION_M,
        solar_system::START_1999_VELOCITY_M_S,
    )
    .unwrap();
    parameters.insert("energy_anchor".to_string(), json!(start.energy_anchor));
    parameters.insert("booked_work".to_string(), json!(start.booked_work));
    parameters.insert("angular_momentum_anchor".to_string(), json!(start.angular_momentum_anchor));
    parameters.insert("booked_torque".to_string(), json!(start.booked_torque));
    parameters.insert(
        "dt_s".to_string(),
        json!(solar_system::SolarSystem::de440().gate_dt_s().unwrap()),
    );
    Value::Object(parameters)
}

fn solar_system_probes(state: &solar_system::SolarSystemState) -> Vec<(&'static str, Probe)> {
    let mut probes = vec![num("t_s", state.t_s)];
    for body in 0..solar_system::BODIES {
        probes.push(vec3_probe(
            SOLAR_SYSTEM_POSITION_KEYS[body],
            state.position_m[body],
        ));
        probes.push(vec3_probe(
            SOLAR_SYSTEM_VELOCITY_KEYS[body],
            state.velocity_m_s[body],
        ));
    }
    probes.extend([
        vec3_probe("earth_to_mars_m", state.earth_to_mars_m),
        num("earth_mars_distance_m", state.earth_mars_distance_m),
        num("energy", state.energy),
        num("energy_anchor", state.energy_anchor),
        num("booked_work", state.booked_work),
        num("energy_defect", state.energy_defect),
        vec3_probe("angular_momentum", state.angular_momentum),
        vec3_probe("angular_momentum_anchor", state.angular_momentum_anchor),
        vec3_probe("booked_torque", state.booked_torque),
        num("turning_defect", state.turning_defect),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
    ]);
    probes
}

/// One wet minute at 18 mm/h on a link whose antennas are already half wet.
fn link_parameters() -> Value {
    json!({
        "dry_path_loss_db": 52.3,
        "antenna_water_db": 0.5,
        "gauge_minute_elapsed_s": 0.0,
        "gauge_minute_rain_mm_h": 0.0,
        "rain_rate_mm_h": 18.0,
        "dt_s": link_gothenburg_10010::LinkGothenburg10010::sublink_587().gate_dt_s().unwrap(),
    })
}

fn link_probes(state: &link_gothenburg_10010::LinkState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("dry_path_loss_db", state.dry_path_loss_db),
        num("antenna_water_db", state.antenna_water_db),
        num("rain_rate_mm_h", state.rain_rate_mm_h),
        num("rain_attenuation_db", state.rain_attenuation_db),
        num("path_loss_db", state.path_loss_db),
        num("received_level_dbm", state.received_level_dbm),
        num("received_power_w", state.received_power_w),
        num("dry_path_power_w", state.dry_path_power_w),
        num("dry_path_energy_j", state.dry_path_energy_j),
        num("received_energy_j", state.received_energy_j),
        num("rain_energy_j", state.rain_energy_j),
        num("antenna_energy_j", state.antenna_energy_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("antenna_account_anchor_db", state.antenna_account_anchor_db),
        num("antenna_built_db", state.antenna_built_db),
        num("antenna_dried_db", state.antenna_dried_db),
        num("antenna_account_defect_db", state.antenna_account_defect_db),
        num("antenna_account_error", state.antenna_account_error),
        num("rain_rate_margin_mm_h", state.rain_rate_margin_mm_h),
        num("gauge_minute_elapsed_s", state.gauge_minute_elapsed_s),
        num("gauge_minute_rain_mm_h", state.gauge_minute_rain_mm_h),
        flag("raining", state.raining),
    ]
}

fn cell_2170_model3_probes(state: &cell_2170_model3::CellState) -> Vec<(&'static str, Probe)> {
    let lag = &state.diffusion_lag;
    vec![
        num("t_s", state.t_s),
        num("cell_number", state.cell_number),
        num("capacity_ah", state.capacity_ah),
        num("state_of_charge", state.state_of_charge),
        num("surface_state_of_charge", state.surface_state_of_charge),
        num("charge_removed_ah", state.charge_removed_ah),
        num("current_a", state.current_a),
        num("case_temperature_c", state.case_temperature_c),
        num("case_temperature_least_c", state.case_temperature_least_c),
        num("case_temperature_most_c", state.case_temperature_most_c),
        num("terminal_v", state.terminal_v),
        num("rest_v", state.rest_v),
        num("resistance_at_once_ohm", state.resistance_at_once_ohm),
        num("reaction_overpotential_v", state.reaction_overpotential_v),
        num("near_full_overpotential_v", state.near_full_overpotential_v),
        num("hysteresis_v", state.hysteresis_v),
        num("electrolyte_v", state.electrolyte_v),
        num("current_mean_a", state.current_mean_a),
        num("diffusion_lag_1", lag[0]),
        num("diffusion_lag_2", lag[1]),
        num("diffusion_lag_3", lag[2]),
        num("diffusion_lag_4", lag[3]),
        num("diffusion_lag_5", lag[4]),
        num("diffusion_lag_6", lag[5]),
        num("diffusion_lag_7", lag[6]),
        num("diffusion_lag_8", lag[7]),
        num("diffusion_lag_9", lag[8]),
        num("diffusion_lag_10", lag[9]),
        num("diffusion_lag_11", lag[10]),
        num("chemical_energy_j", state.chemical_energy_j),
        num("energy_anchor_j", state.energy_anchor_j),
        num("electrical_work_j", state.electrical_work_j),
        num("heat_j", state.heat_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("sustained_above_1c", state.sustained_above_1c),
    ]
}

fn crater_probes(state: &crater_ryugu_sci::CraterState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("radius_m", state.radius_m),
        num("wall_speed_m_s", state.wall_speed_m_s),
        num("radius_over_final", state.radius_over_final),
        num("radius_to_go_m", state.radius_to_go_m),
        num("kinetic_j_per_kg", state.kinetic_j_per_kg),
        num("given_up_j_per_kg", state.given_up_j_per_kg),
        num("energy_defect_j_per_kg", state.energy_defect_j_per_kg),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("stopped", state.stopped),
    ]
}

/// The first step of a twist at the card's step ceiling, at 2 N and 20
/// degrees a second. Every step starts untwisted.
fn fingerpad_parameters() -> Value {
    json!({
        "twist_deg": 0.0,
        "normal_force_n": 2.0,
        "twist_rate_deg_s": 20.0,
        "dt_s": 0.01,
    })
}

fn fingerpad_probes(state: &fingerpad_p02_glass::FingerPadState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("twist_deg", state.twist_deg),
        num("torque_n_m", state.torque_n_m),
        num("slip_torque_limit_n_m", state.slip_torque_limit_n_m),
        num("stuck_fraction", state.stuck_fraction),
        num("plate_work_j", state.plate_work_j),
        num("stored_energy_j", state.stored_energy_j),
        num("friction_heat_j", state.friction_heat_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("held_normal_force_n", state.held_normal_force_n),
        num("held_twist_rate_deg_s", state.held_twist_rate_deg_s),
        num("stored_energy_at_start_j", state.stored_energy_at_start_j),
        flag("full_slip", state.full_slip),
    ]
}

fn accumulator_probes(state: &accumulator_sbo450::GasState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("gas_mass_kg", state.gas_mass_kg),
        num("wall_temperature_k", state.wall_temperature_k),
        num("gas_temperature_k", state.gas_temperature_k),
        num("gas_volume_m3", state.gas_volume_m3),
        num("gas_pressure_pa", state.gas_pressure_pa),
        num("gas_force_n", state.gas_force_n),
        num("work_on_gas_j", state.work_on_gas_j),
        num("heat_to_wall_j", state.heat_to_wall_j),
        num("start_energy_j", state.start_energy_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("work_moved_j", state.work_moved_j),
        num("heat_moved_j", state.heat_moved_j),
        num("heat_share", state.heat_share),
    ]
}

fn tire_probes(state: &tire_eve_r19_ice::TireState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("distance_m", state.distance_m),
        num("rim_travel_m", state.rim_travel_m),
        num("slip_ratio", state.slip_ratio),
        num("slip_held_s", state.slip_held_s),
        num("longitudinal_force_n", state.longitudinal_force_n),
        num("wheel_work_j", state.wheel_work_j),
        num("drawbar_work_j", state.drawbar_work_j),
        num("slip_heat_j", state.slip_heat_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("load_above_fitted", state.load_above_fitted),
    ]
}

fn los_probes(state: &los_jwst_fine_guide::LosState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("lock_x_arcsec", state.lock_x_arcsec),
        num("lock_y_arcsec", state.lock_y_arcsec),
        num("quiet_before_start_s", state.quiet_before_start_s),
        num("slow_x_mas", state.x.slow_mas),
        num("fast_x_mas", state.x.fast_mas),
        num("mode_cos_x_mas", state.x.mode_cos_mas),
        num("mode_sin_x_mas", state.x.mode_sin_mas),
        num("slow_y_mas", state.y.slow_mas),
        num("fast_y_mas", state.y.fast_mas),
        num("mode_cos_y_mas", state.y.mode_cos_mas),
        num("mode_sin_y_mas", state.y.mode_sin_mas),
        num("cov_x_slow_slow_mas2", state.x.cov_mas2[0]),
        num("cov_x_slow_fast_mas2", state.x.cov_mas2[1]),
        num("cov_x_slow_cos_mas2", state.x.cov_mas2[2]),
        num("cov_x_slow_sin_mas2", state.x.cov_mas2[3]),
        num("cov_x_fast_fast_mas2", state.x.cov_mas2[4]),
        num("cov_x_fast_cos_mas2", state.x.cov_mas2[5]),
        num("cov_x_fast_sin_mas2", state.x.cov_mas2[6]),
        num("cov_x_cos_cos_mas2", state.x.cov_mas2[7]),
        num("cov_x_cos_sin_mas2", state.x.cov_mas2[8]),
        num("cov_x_sin_sin_mas2", state.x.cov_mas2[9]),
        num("cov_y_slow_slow_mas2", state.y.cov_mas2[0]),
        num("cov_y_slow_fast_mas2", state.y.cov_mas2[1]),
        num("cov_y_slow_cos_mas2", state.y.cov_mas2[2]),
        num("cov_y_slow_sin_mas2", state.y.cov_mas2[3]),
        num("cov_y_fast_fast_mas2", state.y.cov_mas2[4]),
        num("cov_y_fast_cos_mas2", state.y.cov_mas2[5]),
        num("cov_y_fast_sin_mas2", state.y.cov_mas2[6]),
        num("cov_y_cos_cos_mas2", state.y.cov_mas2[7]),
        num("cov_y_cos_sin_mas2", state.y.cov_mas2[8]),
        num("cov_y_sin_sin_mas2", state.y.cov_mas2[9]),
        num("predicted_x_mas", state.predicted_x_mas),
        num("predicted_y_mas", state.predicted_y_mas),
        num("innovation_x_mas", state.innovation_x_mas),
        num("innovation_y_mas", state.innovation_y_mas),
        num("innovation_variance_x_mas2", state.innovation_variance_x_mas2),
        num("innovation_variance_y_mas2", state.innovation_variance_y_mas2),
        num("estimate_x_mas", state.estimate_x_mas),
        num("estimate_y_mas", state.estimate_y_mas),
        num("estimate_variance_x_mas2", state.estimate_variance_x_mas2),
        num("estimate_variance_y_mas2", state.estimate_variance_y_mas2),
        num("nis", state.nis),
        num("measured_frames", state.measured_frames),
        num("nis_sum_x", state.nis_sum_x),
        num("nis_sum_y", state.nis_sum_y),
        num("variance_x_mas2", state.variance_x_mas2),
        num("variance_y_mas2", state.variance_y_mas2),
        num("variance_defect_x_mas2", state.variance_defect_x_mas2),
        num("variance_defect_y_mas2", state.variance_defect_y_mas2),
        num("residual_conservation_error", state.residual_conservation_error),
        num("unmeasured_in_row", state.unmeasured_in_row),
        flag("no_measurement", state.no_measurement),
    ]
}

/// One step at the ceiling of a 4.5 MJ plasma under 20 MW in an even D-T mix,
/// at the start of the heated window (a later start carries a pulse on and
/// arrives with from).
fn tokamak_parameters() -> Value {
    json!({
        "thermal_energy_j": 4.0e6,
        "fast_ion_energy_j": 0.5e6,
        "elapsed_time_s": 0.0,
        "heating_power_w": 20.0e6,
        "ohmic_power_w": 0.5e6,
        "line_density_per_m3": 6.0e19,
        "plasma_current_a": 3.8e6,
        "toroidal_field_t": 4.0,
        "tritium_fraction": 0.5,
        "dt_s": tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode().gate_dt_s().unwrap(),
    })
}

fn tokamak_probes(state: &tokamak_jet_dte1::TokamakState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("thermal_energy_j", state.thermal_energy_j),
        num("fast_ion_energy_j", state.fast_ion_energy_j),
        num("fusion_energy_j", state.fusion_energy_j),
        num("temperature_kev", state.temperature_kev),
        num("loss_power_w", state.loss_power_w),
        num("confinement_time_s", state.confinement_time_s),
        num("slowing_time_s", state.slowing_time_s),
        num("fusion_power_w", state.fusion_power_w),
        num("fusion_power_thermal_w", state.fusion_power_thermal_w),
        num("beta_normalised", state.beta_normalised),
        num("greenwald_fraction", state.greenwald_fraction),
        num("safety_factor", state.safety_factor),
        num("energy_anchor_j", state.energy_anchor_j),
        num("energy_supplied_j", state.energy_supplied_j),
        num("energy_lost_j", state.energy_lost_j),
        num("energy_defect_j", state.energy_defect_j),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("near_density_limit", state.near_density_limit),
    ]
}

fn cantilever_probes(state: &cantilever_eth::CantileverState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("deflection_m", state.deflection_m),
        num("velocity_m_s", state.velocity_m_s),
        num("envelope_m", state.envelope_m),
        num("kinetic_j_per_kg", state.kinetic_j_per_kg),
        num("potential_j_per_kg", state.potential_j_per_kg),
        num("stored_j_per_kg", state.stored_j_per_kg),
        num("energy_anchor_j_per_kg", state.energy_anchor_j_per_kg),
        num("heat_j_per_kg", state.heat_j_per_kg),
        num("energy_defect_j_per_kg", state.energy_defect_j_per_kg),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
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

fn capsule_insight_probes(state: &capsule_insight::CapsuleState) -> Vec<(&'static str, Probe)> {
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
        num("dynamic_pressure_pa", state.dynamic_pressure_pa),
        num("drag_loss_j_kg", state.drag_loss_j_kg),
        num("energy_anchor_j_kg", state.energy_anchor_j_kg),
        num("energy_defect_j_kg", state.energy_defect_j_kg),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("lift_acting", state.lift_acting),
    ]
}

/// 2026-08-23 0 h UTC, in the card's seconds. A date inside the card.
const SATELLITE_EPOCH_UTC_S: f64 = 840_715_200.0;

/// A position on the x axis at this radius, on an orbit of this size,
/// eccentricity and inclination, moving outward, as the card's own tests
/// build their states. The velocity is the one seen from the card's turning
/// frame.
fn satellite_orbit_shaped(
    radius_m: f64,
    semi_major_axis_m: f64,
    eccentricity: f64,
    inclination_deg: f64,
) -> ([f64; 3], [f64; 3]) {
    let gm = satellite_lageos1::GM_EARTH_M3_S2;
    let speed_squared = gm * (2.0 / radius_m - 1.0 / semi_major_axis_m);
    let across = (gm * semi_major_axis_m * (1.0 - eccentricity * eccentricity)).sqrt() / radius_m;
    let out = (speed_squared - across * across).max(0.0).sqrt();
    let tilt = inclination_deg.to_radians();
    (
        [radius_m, 0.0, 0.0],
        [
            out,
            across * tilt.cos() - satellite_lageos1::EARTH_ROTATION_RATE_RAD_S * radius_m,
            across * tilt.sin(),
        ],
    )
}

/// A LAGEOS-1 orbit of this size crossing the x axis at this radius,
/// eccentricity 0.0039 and inclination 109.815 degrees, inside the card's
/// bands when the size is.
fn satellite_orbit(radius_m: f64, semi_major_axis_m: f64) -> ([f64; 3], [f64; 3]) {
    satellite_orbit_shaped(radius_m, semi_major_axis_m, 0.0039, 109.815)
}

/// A LAGEOS-2 orbit of 12 162 km at an apsis of this radius, inclined 52.645
/// degrees: at 11 997 km and 12 327 km its eccentricity is 0.01357, inside
/// the band, so the radius alone decides.
fn satellite_lageos2_apsis(radius_m: f64) -> ([f64; 3], [f64; 3]) {
    let semi_major_axis_m = 12_162_000.0;
    satellite_orbit_shaped(
        radius_m,
        semi_major_axis_m,
        (1.0 - radius_m / semi_major_axis_m).abs(),
        52.645,
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
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        flag("in_earth_shadow", state.in_earth_shadow),
    ]
}

/// One step at the card's ceiling from this epoch, position and velocity,
/// taken from a published orbit.
fn satellite_parameters(epoch_utc_s: f64, position_m: [f64; 3], velocity_m_s: [f64; 3]) -> Value {
    json!({
        "epoch_utc_s": epoch_utc_s,
        "forecast_elapsed_s": 0.0,
        "position_m": position_m,
        "velocity_m_s": velocity_m_s,
        "dt_s": satellite_lageos1::GATE_DT_S,
    })
}

/// The compared start the described examples start from: LAGEOS-1 at
/// 2026-09-06 0 h UTC, the seven-day run of week 260912.
fn satellite_example_parameters() -> Value {
    satellite_parameters(
        841_924_800.0,
        [295558.7678903363, -5108650.370522532, -11100383.23606335],
        [-4821.08544993592, -3331.9602315669836, 1405.5539438152634],
    )
}

/// One step at the card's ceiling from cold start, at the reactivity of
/// opened test 51 (coolant 82 degF).
fn reactor_parameters() -> Value {
    json!({
        "coolant_temperature_k": (82.0 - 32.0) / 1.8 + 273.15,
        "reactivity_inserted_dollar": 0.9982818460448638,
        "dt_s": reactor_spert3::ReactorSpert3::e_core_cold_startup().gate_dt_s().unwrap(),
    })
}

/// One step at the card's ceiling from the start in the hottest band, at the
/// middle of that band's span of reactivity and of flow.
fn reactor_fuel_parameters() -> Value {
    let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
    let middle = |low: f64, high: f64| (0.5 * (low + high)).clamp(low, high);
    json!({
        "inlet_temperature_k": middle(reactor.band_inlet_min_k[2], reactor.band_inlet_max_k[2]),
        "reactivity_inserted_dollar": middle(
            reactor.band_reactivity_min_dollar[2],
            reactor.band_reactivity_max_dollar[2],
        ),
        "coolant_flow_m_per_s": middle(reactor.band_flow_min_m_per_s[2], reactor.band_flow_max_m_per_s[2]),
        "dt_s": reactor.gate_dt_s().unwrap(),
    })
}

/// One step at the card's ceiling from 22 degC with 2.2 printed dollars on the
/// rod-worth table of core 231, inside the starts compared on that table.
fn reactor_triga_parameters() -> Value {
    let reactor = reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode();
    json!({
        "start_temperature_k": 295.15,
        "rod_table_core": 231.0,
        "reactivity_inserted_dollar": 2.2,
        "rods_dropped": 0.0,
        "dt_s": reactor.gate_dt_s().unwrap(),
    })
}

/// One step at the card's ceiling from 28 degC at 230 psig with 5000 gpm and 1.1 dollars.
fn reactor_ccore_parameters() -> Value {
    let reactor = reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients();
    json!({
        "initial_temperature_k": 301.15,
        "system_pressure_psig": 230.0,
        "reactivity_inserted_dollar": 1.1,
        "coolant_flow_m_per_s": 5000.0 * (18.0 / 20000.0) * 0.3048,
        "dt_s": reactor.gate_dt_s().unwrap(),
    })
}

fn reactor_ccore_probes(state: &reactor_spert3_ccore::ReactorState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("power_mw", state.power_mw),
        num("energy_mws", state.energy_mws),
        num("power_rate_mw_per_s", state.power_rate_mw_per_s),
        num("hot_spot_plate_rise_k", state.hot_spot_plate_rise_k),
        num("hot_spot_plate_rate_k_per_s", state.hot_spot_plate_rate_k_per_s),
        num("hot_spot_water_rise_k", state.hot_spot_water_rise_k),
        num("plate_rise_k", state.plate_rise_k),
        num("water_rise_k", state.water_rise_k),
        num("reactivity_net_dollar", state.reactivity_net_dollar),
        num("initial_temperature_k", state.initial_temperature_k),
        num("system_pressure_psig", state.system_pressure_psig),
        num("precursor_1_mw", state.precursor_mw[0]),
        num("precursor_2_mw", state.precursor_mw[1]),
        num("precursor_3_mw", state.precursor_mw[2]),
        num("precursor_4_mw", state.precursor_mw[3]),
        num("precursor_5_mw", state.precursor_mw[4]),
        num("precursor_6_mw", state.precursor_mw[5]),
        num("population_anchor_mw", state.population_anchor_mw),
        num("feedback_integral_dollar_mws", state.feedback_integral_dollar_mws),
        num("swept_energy_mws", state.swept_energy_mws),
        num("hot_spot_swept_energy_mws", state.hot_spot_swept_energy_mws),
        num("population_defect_mw", state.population_defect_mw),
        num("energy_defect_mws", state.energy_defect_mws),
        num("hot_spot_energy_defect_mws", state.hot_spot_energy_defect_mws),
        num("residual_conservation_error", state.residual_conservation_error),
        num("peak_power_mw", state.peak_power_mw),
        num("peak_energy_mws", state.peak_energy_mws),
        num("peak_time_s", state.peak_time_s),
        num("plate_rise_max_k", state.plate_rise_max_k),
        num("plate_rise_max_time_s", state.plate_rise_max_time_s),
        flag("past_power_peak", state.past_power_peak),
        flag("past_plate_maximum", state.past_plate_maximum),
        flag("prompt_supercritical", state.prompt_supercritical),
    ]
}

fn reactor_triga_probes(state: &reactor_triga_jsi::ReactorState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("power_mw", state.power_mw),
        num("energy_mws", state.energy_mws),
        num("power_rate_mw_per_s", state.power_rate_mw_per_s),
        num("fuel_temperature_k", state.fuel_temperature_k),
        num("reactivity_net_dollar", state.reactivity_net_dollar),
        num("reactivity_inserted_now_dollar", state.reactivity_inserted_now_dollar),
        num("start_temperature_k", state.start_temperature_k),
        num("rod_table_core", state.rod_table_core),
        num("reactivity_load_dollar", state.reactivity_load_dollar),
        num("scram_time_s", state.scram_time_s),
        num("energy_at_scram_mws", state.energy_at_scram_mws),
        num("precursor_1_mw", state.precursor_mw[0]),
        num("precursor_2_mw", state.precursor_mw[1]),
        num("precursor_3_mw", state.precursor_mw[2]),
        num("precursor_4_mw", state.precursor_mw[3]),
        num("precursor_5_mw", state.precursor_mw[4]),
        num("precursor_6_mw", state.precursor_mw[5]),
        num("population_anchor_mw", state.population_anchor_mw),
        num("population_supplied_mw", state.population_supplied_mw),
        num("population_defect_mw", state.population_defect_mw),
        num("residual_conservation_error", state.residual_conservation_error),
        num("peak_power_mw", state.peak_power_mw),
        num("peak_energy_mws", state.peak_energy_mws),
        num("peak_time_s", state.peak_time_s),
        num("peak_fuel_temperature_k", state.peak_fuel_temperature_k),
        flag("prompt_supercritical", state.prompt_supercritical),
        flag("rod_moving", state.rod_moving),
        flag("rods_dropped", state.rods_dropped),
    ]
}

fn reactor_fuel_probes(state: &reactor_spert3_fuel::ReactorState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("power_mw", state.power_mw),
        num("energy_mws", state.energy_mws),
        num("power_rate_mw_per_s", state.power_rate_mw_per_s),
        num("fuel_temperature_rise_k", state.fuel_temperature_rise_k),
        num("water_temperature_rise_k", state.water_temperature_rise_k),
        num("reactivity_net_dollar", state.reactivity_net_dollar),
        num(
            "reactivity_inserted_now_dollar",
            state.reactivity_inserted_now_dollar,
        ),
        num("inlet_temperature_k", state.inlet_temperature_k),
        num("precursor_1_mw", state.precursor_mw[0]),
        num("precursor_2_mw", state.precursor_mw[1]),
        num("precursor_3_mw", state.precursor_mw[2]),
        num("precursor_4_mw", state.precursor_mw[3]),
        num("precursor_5_mw", state.precursor_mw[4]),
        num("precursor_6_mw", state.precursor_mw[5]),
        num("population_anchor_mw", state.population_anchor_mw),
        num(
            "insertion_integral_dollar_mws",
            state.insertion_integral_dollar_mws,
        ),
        num("sweep_integral_k2", state.sweep_integral_k2),
        num("population_defect_mw", state.population_defect_mw),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("peak_power_mw", state.peak_power_mw),
        num("peak_energy_mws", state.peak_energy_mws),
        num("peak_time_s", state.peak_time_s),
        flag("past_first_maximum", state.past_first_maximum),
        flag("prompt_supercritical", state.prompt_supercritical),
        flag("rod_moving", state.rod_moving),
    ]
}

fn reactor_probes(state: &reactor_spert3::ReactorState) -> Vec<(&'static str, Probe)> {
    vec![
        num("t_s", state.t_s),
        num("power_mw", state.power_mw),
        num("energy_mws", state.energy_mws),
        num("power_rate_mw_per_s", state.power_rate_mw_per_s),
        num("reactivity_net_dollar", state.reactivity_net_dollar),
        num("coolant_temperature_k", state.coolant_temperature_k),
        num("precursor_1_mw", state.precursor_mw[0]),
        num("precursor_2_mw", state.precursor_mw[1]),
        num("precursor_3_mw", state.precursor_mw[2]),
        num("precursor_4_mw", state.precursor_mw[3]),
        num("precursor_5_mw", state.precursor_mw[4]),
        num("precursor_6_mw", state.precursor_mw[5]),
        num("population_anchor_mw", state.population_anchor_mw),
        num(
            "reactivity_energy_integral_dollar_mws",
            state.reactivity_energy_integral_dollar_mws,
        ),
        num("population_defect_mw", state.population_defect_mw),
        num(
            "residual_conservation_error",
            state.residual_conservation_error,
        ),
        num("peak_power_mw", state.peak_power_mw),
        num("peak_energy_mws", state.peak_energy_mws),
        num("peak_time_s", state.peak_time_s),
        flag("past_first_maximum", state.past_first_maximum),
        flag("prompt_supercritical", state.prompt_supercritical),
    ]
}

fn hex_digits(hex: &str, label: &str) {
    assert_eq!(hex.len(), 16, "{label}");
    assert!(
        hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "{label} {hex}"
    );
    let bits = u64::from_str_radix(hex, 16).unwrap();
    assert_eq!(format!("{:016x}", bits), hex, "{label}");
}

fn check_state(state: &Value, probes: &[(&str, Probe)]) {
    let object = state
        .as_object()
        .unwrap_or_else(|| panic!("state is not the post-step object: {state}"));
    let mut expected_keys = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    for (key, probe) in probes {
        assert!(seen.insert(*key), "oracle listed {key} twice");
        expected_keys.insert((*key).to_string());
        match probe {
            Probe::Num(expected) => {
                let got = object
                    .get(*key)
                    .and_then(Value::as_f64)
                    .unwrap_or_else(|| panic!("state.{key} missing from {state}"));
                assert_eq!(got.to_bits(), expected.to_bits(), "state.{key}");
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
                hex_digits(hex, &hex_key);
                let rebuilt = f64::from_bits(u64::from_str_radix(hex, 16).unwrap());
                assert_eq!(rebuilt.to_bits(), expected.to_bits(), "state.{hex_key}");
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
                    assert_eq!(
                        got[index].as_f64().unwrap().to_bits(),
                        component.to_bits(),
                        "state.{key}[{index}]"
                    );
                    let hex = hexes[index]
                        .as_str()
                        .unwrap_or_else(|| panic!("state.{hex_key}[{index}] is not a string"));
                    assert_eq!(
                        hex,
                        format!("{:016x}", component.to_bits()),
                        "state.{hex_key}[{index}]"
                    );
                    hex_digits(hex, &format!("{hex_key}[{index}]"));
                    let rebuilt = f64::from_bits(u64::from_str_radix(hex, 16).unwrap());
                    assert_eq!(rebuilt.to_bits(), component.to_bits());
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
    let actual: std::collections::BTreeSet<_> = object.keys().cloned().collect();
    assert_eq!(
        actual, expected_keys,
        "state keys drifted from the card. every energy, margin, and flag must be bit-checked"
    );
}

fn integrate(domain: &str, card: &str, params: &Value, proof: [u8; 16]) -> Oracle {
    let mut frame = blank_frame();
    frame.proof = proof;
    match (domain, card) {
        ("materials", "ti64_lattice") => {
            let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
            let mut state = ti64_lattice::Ti64LatticeState::try_phase(
                &cell,
                vec3(params, "position_m"),
                vec3(params, "velocity_m_s"),
            )
            .unwrap();
            let load = ti64_lattice::Ti64Load {
                force_n: vec3(params, "force_n"),
                torque_nm: vec3(params, "torque_nm"),
                temperature_k: scalar(params, "temperature_k"),
            };
            ti64_lattice::step(&cell, &mut state, &mut frame, load, scalar(params, "dt_s"))
                .unwrap();
            assert_eq!(frame.flags, 0, "oracle sealed a failed Ti-6Al-4V frame");
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: ti64_probes(&state),
            }
        }
        ("energy", "grid_node") => {
            let node = grid_node::GridNode::copper_fuse_link();
            let mut state = grid_node::GridNodeState::try_phase(
                &node,
                scalar(params, "current_a"),
                scalar(params, "voltage_v"),
                scalar(params, "temperature_k"),
            )
            .unwrap();
            let load = grid_node::GridLoad {
                upstream_voltage_v: scalar(params, "upstream_voltage_v"),
                load_current_a: scalar(params, "load_current_a"),
                ambient_k: scalar(params, "ambient_k"),
            };
            grid_node::step(&node, &mut state, &mut frame, load, scalar(params, "dt_s")).unwrap();
            assert_eq!(
                frame.flags & !grid_node::FLAG_GRID_SURGE,
                0,
                "oracle sealed a failed grid frame"
            );
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: grid_probes(&state),
            }
        }
        ("energy", "thermal") => {
            let node = thermal::ThermalNode::avionics_cold_plate();
            let mut state =
                thermal::ThermalNodeState::try_phase(&node, scalar(params, "temperature_k"))
                    .unwrap();
            let load = thermal::ThermalLoad {
                heat_power_w: scalar(params, "heat_power_w"),
                ambient_k: scalar(params, "ambient_k"),
            };
            thermal::step(&node, &mut state, &mut frame, load, scalar(params, "dt_s")).unwrap();
            if scalar(params, "temperature_k") >= thermal::DERATE_TEMPERATURE_K {
                assert_eq!(frame.flags, thermal::FLAG_THERMAL_DERATE);
            } else {
                assert_eq!(frame.flags, 0);
            }
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: thermal_probes(&state),
            }
        }
        ("signal", "resonance") => {
            let oscillator = resonance::DynamicOscillator::mount_10hz();
            let mut state = resonance::OscillatorState::try_continue(
                &oscillator,
                scalar(params, "displacement_m"),
                scalar(params, "velocity_m_s"),
                scalar(params, "t_s"),
                scalar(params, "energy_anchor_j"),
                scalar(params, "boundary_work_j"),
            )
            .unwrap();
            let load = resonance::OscillatorLoad {
                accel_m_s2: scalar(params, "accel_m_s2"),
            };
            resonance::step(
                &oscillator,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !resonance::FLAG_STROKE_LIMIT, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: resonance_probes(&state),
            }
        }
        ("energy", "swing") => {
            let machine = swing::SwingMachine::synchronous_60hz();
            let mut state = swing::SwingState::try_dispatched(
                &machine,
                scalar(params, "delta_rad"),
                scalar(params, "delta_omega_rad_s"),
                scalar(params, "pll_err_rad"),
                scalar(params, "p_mech_pu"),
            )
            .unwrap();
            let load = swing::SwingLoad {
                p_mech_pu: scalar(params, "p_mech_pu"),
            };
            swing::step(
                &machine,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            if scalar(params, "pll_err_rad").abs() >= swing::PLL_TRIP_RAD {
                assert_eq!(frame.flags, swing::FLAG_PLL_SLIP);
            } else {
                assert_eq!(frame.flags & !swing::FLAG_PLL_SLIP, 0);
            }
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: swing_probes(&state),
            }
        }
        ("signal", "tesseract") => {
            let resonator = tesseract::Tesseract::drive_mode();
            let mut state = tesseract::TesseractState::try_continue(
                &resonator,
                scalar(params, "displacement_m"),
                scalar(params, "velocity_m_s"),
                scalar(params, "energy_defect_j"),
            )
            .unwrap();
            let load = tesseract::TesseractLoad {
                accel_m_s2: scalar(params, "accel_m_s2"),
                omega_ext_rad_s: scalar(params, "omega_ext_rad_s"),
                sense_velocity_m_s: scalar(params, "sense_velocity_m_s"),
            };
            tesseract::step(
                &resonator,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: tesseract_probes(&state),
            }
        }
        ("materials", "hydraulic_fluid") => {
            let column = hydraulic_fluid::FluidColumn::mineral_oil();
            let mut state =
                hydraulic_fluid::FluidState::try_pressure(&column, scalar(params, "pressure_pa"))
                    .unwrap();
            let load = hydraulic_fluid::FluidLoad {
                flow_m3_s: scalar(params, "flow_m3_s"),
            };
            hydraulic_fluid::step(
                &column,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(
                frame.flags
                    & !(hydraulic_fluid::FLAG_COLUMN_SOFT | hydraulic_fluid::FLAG_COLUMN_LIMIT),
                0
            );
            if scalar(params, "pressure_pa") >= hydraulic_fluid::REFERENCE_PRESSURE_PA + 1.0e6 {
                assert_ne!(frame.flags & hydraulic_fluid::FLAG_COLUMN_SOFT, 0);
            }
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: hydraulic_probes(&state),
            }
        }
        ("materials", "cable_stretch") => {
            let tendon = cable_stretch::Tendon::dyneema();
            let mut state = cable_stretch::TendonState::try_phase(
                &tendon,
                scalar(params, "elongation_m"),
                scalar(params, "velocity_m_s"),
            )
            .unwrap();
            let load = cable_stretch::TendonLoad {
                force_n: scalar(params, "force_n"),
            };
            cable_stretch::step(
                &tendon,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(
                frame.flags
                    & !(cable_stretch::FLAG_TRACKING_DEGRADE | cable_stretch::FLAG_CONTROL_LIMIT),
                0
            );
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: cable_probes(&state),
            }
        }
        ("energy", "battery_sag") => {
            let pack = battery_sag::LiIonPack::pack_12s();
            let mut state = battery_sag::PackState::try_soc(&pack, scalar(params, "soc")).unwrap();
            let load = battery_sag::PackLoad {
                current_a: scalar(params, "current_a"),
            };
            battery_sag::step(&pack, &mut state, &mut frame, load, scalar(params, "dt_s")).unwrap();
            if scalar(params, "current_a") >= 150.0 {
                assert_eq!(frame.flags, battery_sag::FLAG_SAG_WARNING);
            } else {
                assert_eq!(frame.flags & !battery_sag::FLAG_SAG_WARNING, 0);
            }
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: battery_probes(&state),
            }
        }
        ("energy", "cell_18650pf") => {
            let specimen = cell_18650pf::Cell18650Pf::cell();
            let mut state = cell_18650pf::CellState::try_continued(
                &specimen,
                scalar(params, "soc"),
                scalar(params, "held_current_a"),
            )
            .unwrap();
            let load = cell_18650pf::CellLoad {
                current_a: scalar(params, "current_a"),
            };
            cell_18650pf::step(
                &specimen,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !cell_18650pf::FLAG_SAG_WARNING, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: cell_probes(&state),
            }
        }
        ("energy", "cell_2170_m80") => {
            let specimen = cell_2170_m80::Cell2170M80::cell();
            let mut state = cell_2170_m80::CellState::try_start(
                &specimen,
                scalar(params, "block_chamber_c"),
                scalar(params, "charge_removed_ah"),
            )
            .unwrap();
            let load = cell_2170_m80::CellLoad {
                current_a: scalar(params, "current_a"),
                case_temperature_c: scalar(params, "case_temperature_c"),
            };
            cell_2170_m80::step(
                &specimen,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            // The card sets no flag.
            assert_eq!(frame.flags, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: cell_2170_probes(&state),
            }
        }
        ("energy", "generator_mmrtg_f1") => {
            let generator = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
            let mut state = generator_mmrtg_f1::GeneratorState::try_mission_time(
                &generator,
                scalar(params, "mission_time_s"),
            )
            .unwrap();
            generator_mmrtg_f1::step(
                &generator,
                &mut state,
                &mut frame,
                generator_mmrtg_f1::GeneratorLoad {},
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(
                frame.flags & !generator_mmrtg_f1::FLAG_PAST_FITTING_SPAN,
                0
            );
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: generator_probes(&state),
            }
        }
        ("aerospace", "capsule_stardust") => {
            let capsule = capsule_stardust::CapsuleStardust::sample_return_capsule();
            let mut state = capsule_stardust::CapsuleState::try_state(
                &capsule,
                scalar(params, "radius_m"),
                scalar(params, "radial_speed_m_s"),
                scalar(params, "transverse_speed_m_s"),
            )
            .unwrap();
            capsule_stardust::step(
                &capsule,
                &mut state,
                &mut frame,
                capsule_stardust::CapsuleLoad {},
                scalar(params, "dt_s"),
            )
            .unwrap();
            let advisory = capsule_stardust::FLAG_DROGUE_MACH_WINDOW;
            assert_eq!(frame.flags & !advisory, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: capsule_probes(&state),
            }
        }
        ("aerospace", "capsule_insight") => {
            let capsule = capsule_insight::CapsuleInsight::entry_capsule();
            let mut state = capsule_insight::CapsuleState::try_state(
                &capsule,
                scalar(params, "radius_m"),
                scalar(params, "radial_speed_m_s"),
                scalar(params, "transverse_speed_m_s"),
            )
            .unwrap();
            capsule_insight::step(
                &capsule,
                &mut state,
                &mut frame,
                capsule_insight::CapsuleLoad {
                    lift_over_drag: scalar(params, "lift_over_drag"),
                },
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !capsule_insight::FLAG_LIFT_ACTING, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: capsule_insight_probes(&state),
            }
        }
        ("aerospace", "satellite_lageos1") => {
            let satellite = satellite_lageos1::SatelliteLageos1::lageos1();
            let mut state = satellite_lageos1::SatelliteState::try_state(
                &satellite,
                scalar(params, "epoch_utc_s"),
                scalar(params, "forecast_elapsed_s"),
                vec3(params, "position_m"),
                vec3(params, "velocity_m_s"),
            )
            .unwrap();
            satellite_lageos1::step(
                &satellite,
                &mut state,
                &mut frame,
                satellite_lageos1::SatelliteLoad {},
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !satellite_lageos1::FLAG_EARTH_SHADOW, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: satellite_probes(&state),
            }
        }
        ("energy", "reactor_spert3") => {
            let reactor = reactor_spert3::ReactorSpert3::e_core_cold_startup();
            let mut state = reactor_spert3::ReactorState::try_cold_start(
                &reactor,
                scalar(params, "coolant_temperature_k"),
            )
            .unwrap();
            reactor_spert3::step(
                &reactor,
                &mut state,
                &mut frame,
                reactor_spert3::ReactorLoad {
                    reactivity_inserted_dollar: scalar(params, "reactivity_inserted_dollar"),
                },
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !reactor_spert3::FLAG_PROMPT_SUPERCRITICAL, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: reactor_probes(&state),
            }
        }
        ("signal", "cantilever_eth") => {
            let beam = cantilever_eth::CantileverEth::first_bending_mode();
            let mut state = cantilever_eth::CantileverState::try_phase(
                &beam,
                scalar(params, "deflection_m"),
                scalar(params, "velocity_m_s"),
            )
            .unwrap();
            cantilever_eth::step(
                &beam,
                &mut state,
                &mut frame,
                cantilever_eth::CantileverLoad {},
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: cantilever_probes(&state),
            }
        }
        ("energy", "cell_2170_model3") => {
            let specimen = cell_2170_model3::Cell2170Model3::cell();
            let mut state = cell_2170_model3::CellState::try_rested(
                &specimen,
                scalar(params, "cell_number"),
                scalar(params, "state_of_charge"),
                scalar(params, "capacity_ah"),
            )
            .and_then(|rested| {
                rested.with_case_span(
                    &specimen,
                    scalar(params, "case_temperature_least_c"),
                    scalar(params, "case_temperature_most_c"),
                )
            })
            .unwrap();
            let load = cell_2170_model3::CellLoad {
                current_a: scalar(params, "current_a"),
                case_temperature_c: scalar(params, "case_temperature_c"),
            };
            cell_2170_model3::step(
                &specimen,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: cell_2170_model3_probes(&state),
            }
        }
        ("materials", "creep_cmsx6") => {
            let alloy = creep_cmsx6::CreepCmsx6::at_980_c();
            let mut state = creep_cmsx6::CreepState::try_new(
                &alloy,
                scalar(params, "stress_mpa"),
                scalar(params, "creep_extension_percent"),
            )
            .unwrap();
            creep_cmsx6::step(
                &alloy,
                &mut state,
                &mut frame,
                creep_cmsx6::CreepLoad {
                    stress_mpa: scalar(params, "stress_mpa"),
                },
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !creep_cmsx6::FLAG_CREEP_RATE_RISING, 0);
            assert!(state.creep_extension_percent >= creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: creep_probes(&state),
            }
        }
        ("energy", "reactor_spert3_ccore") => {
            let reactor = reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients();
            let mut state = reactor_spert3_ccore::ReactorState::try_start(
                &reactor,
                scalar(params, "initial_temperature_k"),
                scalar(params, "system_pressure_psig"),
            )
            .unwrap();
            reactor_spert3_ccore::step(
                &reactor,
                &mut state,
                &mut frame,
                reactor_spert3_ccore::ReactorLoad {
                    reactivity_inserted_dollar: scalar(params, "reactivity_inserted_dollar"),
                    coolant_flow_m_per_s: scalar(params, "coolant_flow_m_per_s"),
                },
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(
                frame.flags
                    & !(reactor_spert3_ccore::FLAG_PROMPT_SUPERCRITICAL
                        | reactor_spert3_ccore::FLAG_PAST_POWER_PEAK),
                0
            );
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: reactor_ccore_probes(&state),
            }
        }
        ("energy", "reactor_triga_jsi") => {
            let reactor = reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode();
            let mut state = reactor_triga_jsi::ReactorState::try_start(
                &reactor,
                scalar(params, "start_temperature_k"),
                scalar(params, "rod_table_core"),
            )
            .unwrap();
            reactor_triga_jsi::step(
                &reactor,
                &mut state,
                &mut frame,
                reactor_triga_jsi::ReactorLoad {
                    reactivity_inserted_dollar: scalar(params, "reactivity_inserted_dollar"),
                    rods_dropped: scalar(params, "rods_dropped"),
                },
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(
                frame.flags
                    & !(reactor_triga_jsi::FLAG_PROMPT_SUPERCRITICAL
                        | reactor_triga_jsi::FLAG_ROD_MOVING
                        | reactor_triga_jsi::FLAG_RODS_DROPPED),
                0
            );
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: reactor_triga_probes(&state),
            }
        }
        ("energy", "reactor_spert3_fuel") => {
            let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
            let mut state = reactor_spert3_fuel::ReactorState::try_start(
                &reactor,
                scalar(params, "inlet_temperature_k"),
            )
            .unwrap();
            reactor_spert3_fuel::step(
                &reactor,
                &mut state,
                &mut frame,
                reactor_spert3_fuel::ReactorLoad {
                    reactivity_inserted_dollar: scalar(params, "reactivity_inserted_dollar"),
                    coolant_flow_m_per_s: scalar(params, "coolant_flow_m_per_s"),
                },
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(
                frame.flags
                    & !(reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL
                        | reactor_spert3_fuel::FLAG_ROD_MOVING),
                0
            );
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: reactor_fuel_probes(&state),
            }
        }
        ("aerospace", "crater_ryugu_sci") => {
            let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();
            let mut state =
                crater_ryugu_sci::CraterState::try_radius(&crater, scalar(params, "radius_m"))
                    .unwrap();
            crater_ryugu_sci::step(
                &crater,
                &mut state,
                &mut frame,
                crater_ryugu_sci::CraterLoad {},
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !crater_ryugu_sci::FLAG_STOPPED, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: crater_probes(&state),
            }
        }
        ("signal", "link_gothenburg_10010") => {
            let specimen = link_gothenburg_10010::LinkGothenburg10010::sublink_587();
            let mut state = link_gothenburg_10010::LinkState::try_new(
                &specimen,
                scalar(params, "dry_path_loss_db"),
                scalar(params, "antenna_water_db"),
            )
            .unwrap();
            let load = link_gothenburg_10010::LinkLoad {
                rain_rate_mm_h: scalar(params, "rain_rate_mm_h"),
            };
            link_gothenburg_10010::step(
                &specimen,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !link_gothenburg_10010::FLAG_RAINING, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: link_probes(&state),
            }
        }
        ("energy", "grid_gb") => {
            let specimen = grid_gb::GridGb::interconnector_loss();
            let mut state = grid_gb::GridState::try_start(
                &specimen,
                scalar(params, "frequency_hz"),
                scalar(params, "inertia_mw_s"),
                scalar(params, "demand_mw"),
            )
            .unwrap();
            let load = grid_gb::GridLoad {
                loss_mw: scalar(params, "loss_mw"),
            };
            grid_gb::step(
                &specimen,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: grid_gb_probes(&state),
            }
        }
        ("aerospace", "solar_system") => {
            let system = solar_system::SolarSystem::de440();
            let mut position_m = [[0.0; 3]; solar_system::BODIES];
            let mut velocity_m_s = [[0.0; 3]; solar_system::BODIES];
            for body in 0..solar_system::BODIES {
                position_m[body] = vec3(params, SOLAR_SYSTEM_POSITION_KEYS[body]);
                velocity_m_s[body] = vec3(params, SOLAR_SYSTEM_VELOCITY_KEYS[body]);
            }
            let mut state =
                solar_system::SolarSystemState::try_state(&system, position_m, velocity_m_s)
                    .unwrap();
            solar_system::step(
                &system,
                &mut state,
                &mut frame,
                solar_system::SolarSystemLoad {},
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: solar_system_probes(&state),
            }
        }
        ("energy", "accumulator_sbo450") => {
            let specimen = accumulator_sbo450::AccumulatorSbo450::on_darmstadt_rig();
            let mut state = accumulator_sbo450::GasState::try_start(
                &specimen,
                scalar(params, "gas_mass_kg"),
                scalar(params, "gas_pressure_pa"),
                scalar(params, "gas_temperature_k"),
                scalar(params, "wall_temperature_k"),
                scalar(params, "work_moved_j"),
                scalar(params, "heat_moved_j"),
            )
            .unwrap();
            let load = accumulator_sbo450::AccumulatorLoad {
                piston_speed_m_s: scalar(params, "piston_speed_m_s"),
            };
            accumulator_sbo450::step(&specimen, &mut state, &mut frame, load, scalar(params, "dt_s"))
                .unwrap();
            assert_eq!(frame.flags, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: accumulator_probes(&state),
            }
        }
        ("materials", "tire_eve_r19_ice") => {
            let specimen = tire_eve_r19_ice::TireEveR19Ice::on_rig_ice();
            let mut state =
                tire_eve_r19_ice::TireState::try_start(
                    &specimen,
                    scalar(params, "distance_m"),
                    scalar(params, "slip_held_s"),
                )
                .unwrap();
            let load = tire_eve_r19_ice::TireLoad {
                normal_load_n: scalar(params, "normal_load_n"),
                inflation_pressure_pa: scalar(params, "inflation_pressure_pa"),
                slip_ratio: scalar(params, "slip_ratio"),
                carriage_speed_m_s: scalar(params, "carriage_speed_m_s"),
            };
            tire_eve_r19_ice::step(&specimen, &mut state, &mut frame, load, scalar(params, "dt_s"))
                .unwrap();
            assert_eq!(frame.flags, tire_eve_r19_ice::FLAG_LOAD_ABOVE_FITTED);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: tire_probes(&state),
            }
        }
        ("aerospace", "los_jwst_fine_guide") => {
            let specimen = los_jwst_fine_guide::LosJwstFineGuide::observatory();
            let mut state = los_jwst_fine_guide::LosState::try_lock(
                &specimen,
                scalar(params, "lock_x_arcsec"),
                scalar(params, "lock_y_arcsec"),
                scalar(params, "quiet_before_start_s"),
            )
            .unwrap();
            let load = los_jwst_fine_guide::LosLoad {
                measured: scalar(params, "measured"),
                centroid_x_arcsec: scalar(params, "centroid_x_arcsec"),
                centroid_y_arcsec: scalar(params, "centroid_y_arcsec"),
                counts_per_s: scalar(params, "counts_per_s"),
            };
            los_jwst_fine_guide::step(
                &specimen,
                &mut state,
                &mut frame,
                load,
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !los_jwst_fine_guide::FLAG_NO_MEASUREMENT, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: los_probes(&state),
            }
        }
        ("materials", "fingerpad_p02_glass") => {
            let specimen = fingerpad_p02_glass::FingerPadP02Glass::on_glass();
            let mut state =
                fingerpad_p02_glass::FingerPadState::try_start(&specimen, scalar(params, "twist_deg"))
                    .unwrap();
            let load = fingerpad_p02_glass::FingerPadLoad {
                normal_force_n: scalar(params, "normal_force_n"),
                twist_rate_deg_s: scalar(params, "twist_rate_deg_s"),
            };
            fingerpad_p02_glass::step(&specimen, &mut state, &mut frame, load, scalar(params, "dt_s"))
                .unwrap();
            assert_eq!(frame.flags, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: fingerpad_probes(&state),
            }
        }
        ("energy", "tokamak_jet_dte1") => {
            let tokamak = tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode();
            let mut state = tokamak_jet_dte1::TokamakState::try_continue(
                &tokamak,
                scalar(params, "elapsed_time_s"),
                scalar(params, "thermal_energy_j"),
                scalar(params, "fast_ion_energy_j"),
            )
            .unwrap();
            tokamak_jet_dte1::step(
                &tokamak,
                &mut state,
                &mut frame,
                tokamak_jet_dte1::TokamakLoad {
                    heating_power_w: scalar(params, "heating_power_w"),
                    ohmic_power_w: scalar(params, "ohmic_power_w"),
                    line_density_per_m3: scalar(params, "line_density_per_m3"),
                    plasma_current_a: scalar(params, "plasma_current_a"),
                    toroidal_field_t: scalar(params, "toroidal_field_t"),
                    tritium_fraction: scalar(params, "tritium_fraction"),
                },
                scalar(params, "dt_s"),
            )
            .unwrap();
            assert_eq!(frame.flags & !tokamak_jet_dte1::FLAG_NEAR_DENSITY_LIMIT, 0);
            let residual = state.residual_conservation_error;
            Oracle {
                frame,
                residual,
                probes: tokamak_probes(&state),
            }
        }
        _ => panic!("oracle has no route for {domain}/{card}"),
    }
}

fn forged_frame_seal(residual: f64) -> String {
    let forged = LastStateFrame64 {
        t: 1.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: residual as f32,
        flags: 0,
        proof: [0; 16],
    };
    sha256_hex(&forged.to_bytes())
}

async fn assert_real_seal(
    base: &str,
    domain: &str,
    card: &str,
    parameters: Value,
) -> String {
    let body = request_body(domain, card, parameters);
    let parsed: Value = serde_json::from_str(&body).unwrap();
    let proof = commit_request_proof(&parsed);
    let oracle = integrate(domain, card, &parsed["parameters"], proof);
    let frame = oracle.frame;
    let residual = oracle.residual;
    let (status, text) = post_json(base, &body).await;
    assert_eq!(status, 200, "{text}");
    let response: VerifyPhysicsResponse = serde_json::from_str(&text).unwrap();
    assert_eq!(
        response.engine_version,
        "genesis_kernel_v0.9.0"
    );
    assert_eq!(response.inputs, parsed);
    assert_eq!(
        serde_json::to_string(&response.inputs).unwrap(),
        body,
        "the request is echoed in the order received"
    );
    check_state(&response.state, &oracle.probes);
    assert!(frame.t > 0.0);
    assert_eq!(frame.to_bytes().len(), 64);
    assert_eq!(
        response.residual_conservation_error.to_bits(),
        residual.to_bits(),
        "response residual is not the f64 thermodynamic residual"
    );
    let seal = sha256_hex(&frame.to_bytes());
    // The proof covers the pair with the parameters, not the parameters alone.
    let mut unnamed = frame;
    unnamed.proof = commit_request_proof(&parsed["parameters"]);
    assert_ne!(
        seal,
        sha256_hex(&unnamed.to_bytes()),
        "seal hashed the parameters without the domain and the card"
    );
    let mut naked = frame;
    naked.proof = [0; 16];
    assert_ne!(
        seal,
        sha256_hex(&naked.to_bytes()),
        "seal hashed the frame with a zero proof slot"
    );
    assert_eq!(response.sha256_seal, seal);
    assert_eq!(response.sha256_seal.len(), 64);
    assert!(
        response
            .sha256_seal
            .chars()
            .all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    );
    assert_ne!(response.sha256_seal, forged_frame_seal(residual));
    let (document, receipt) = split_receipt(&text);
    assert_eq!(receipt, response.receipt_sha256);
    assert_eq!(sha256_hex(document.as_bytes()), receipt);
    assert_ne!(receipt, response.sha256_seal);
    assert!(document.contains(&format!("\"sha256_seal\":\"{seal}\"")));
    assert!(!document.contains("receipt_sha256"));
    let needle = "\"t_s_hex\":\"";
    let at = document.find(needle).expect("t_s hex") + needle.len();
    let mut forged = document.into_bytes();
    forged[at] = if forged[at] == b'0' { b'1' } else { b'0' };
    let forged = String::from_utf8(forged).unwrap();
    assert!(forged.contains(&format!("\"sha256_seal\":\"{seal}\"")));
    assert_ne!(
        sha256_hex(forged.as_bytes()),
        receipt,
        "a state-bit change left the receipt intact while the frame seal still matched"
    );
    response.sha256_seal
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha256\":\"";
    let pos = text
        .rfind(MARKER)
        .unwrap_or_else(|| panic!("receipt missing from {text}"));
    assert!(text.ends_with("\"}"), "{text}");
    let receipt = text[pos + MARKER.len()..text.len() - 2].to_string();
    assert_eq!(receipt.len(), 64, "{receipt}");
    assert!(
        receipt.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "{receipt}"
    );
    (format!("{}}}", &text[..pos]), receipt)
}

fn assert_no_seal(text: &str) {
    assert!(
        !text.contains("sha256_seal"),
        "rejected response wrote a frame seal: {text}"
    );
    assert!(
        !text.contains("receipt_sha256"),
        "rejected response wrote a receipt: {text}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_card_seals_the_integrated_frame() {
    let base = spawn_app().await;
    let materials =
        assert_real_seal(&base, "materials", "ti64_lattice", materials_parameters()).await;
    let energy = assert_real_seal(&base, "energy", "grid_node", energy_parameters(25.0)).await;
    assert_ne!(materials, energy);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_json_seals_the_same_bytes() {
    let base = spawn_app().await;
    let parameters = materials_parameters();
    let first = assert_real_seal(&base, "materials", "ti64_lattice", parameters.clone()).await;
    let second = assert_real_seal(&base, "materials", "ti64_lattice", parameters).await;
    assert_eq!(first, second);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upstream_voltage_changes_the_grid_seal() {
    let base = spawn_app().await;
    let at_25 = assert_real_seal(&base, "energy", "grid_node", energy_parameters(25.0)).await;
    let at_26 = assert_real_seal(&base, "energy", "grid_node", energy_parameters(26.0)).await;
    assert_ne!(at_25, at_26);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_parameter_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    let mut parameters = materials_parameters();
    parameters.as_object_mut().unwrap().remove("dt_s");
    let (status, text) = post_json(
        &base,
        &request_body("materials", "ti64_lattice", parameters),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(
        text.contains("Missing required parameter: 'dt_s'"),
        "{text}"
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn negative_kelvin_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    let parameters = json!({ "temperature_k": -50.0 });
    let (status, text) = post_json(&base, &request_body("energy", "grid_node", parameters)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("absolute zero"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn negative_mass_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    let parameters = json!({ "mass_kg": -10.0 });
    let (status, text) = post_json(&base, &request_body("energy", "grid_node", parameters)).await;
    assert_eq!(status, 422, "{text}");
    // An unknown key is named as unknown before any rule that reads a key's
    // name for its meaning: the sentence does not call it a mass.
    assert_eq!(
        text,
        "Physical Violation: 'mass_kg' is not a field of this request."
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_step_does_not_seal() {
    let base = spawn_app().await;
    let mut parameters = materials_parameters();
    parameters["temperature_k"] = json!(200.0);
    let (status, text) = post_json(
        &base,
        &request_body("materials", "ti64_lattice", parameters),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(
        text.contains(
            "'temperature_k' 200 K is below 250 K, the inclusive minimum of the ti64_lattice card (TemperatureOutsideCard)."
        ),
        "{text}"
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_card_is_404() {
    let base = spawn_app().await;
    let (status, text) = post_json(
        &base,
        &request_body("materials", "warp_drive", json!({ "temperature_k": 300.0 })),
    )
    .await;
    assert_eq!(status, 404, "{text}");
    assert!(text.contains("Card not recognized"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn removed_mock_routes_are_not_found() {
    let base = spawn_app().await;
    let routes = [
        (
            "aerospace",
            "reentry_trajectory",
            json!({ "mach": 20.0, "altitude_m": 85000.0 }),
        ),
        (
            "energy",
            "battery_thermal_runaway",
            json!({ "temperature_k": 300.0, "mass_kg": 2.0 }),
        ),
        ("energy", "grid_simulation", json!({ "base_load": 1.0 })),
        ("biology", "protein_fold", json!({ "temperature_k": 300.0 })),
        ("biology", "bioreactor", json!({ "temperature_k": 300.0 })),
    ];
    for (domain, card, parameters) in routes {
        let (status, text) =
            post_json(&base, &request_body(domain, card, parameters)).await;
        assert_eq!(status, 404, "{domain}/{card}: {text}");
        assert_no_seal(&text);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn yield_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    // A start that stands past yield. In version 0.2.1 the position published no
    // bound and the card refused it; the position now publishes the yield position
    // and the door turns the start away, naming the card's refusal.
    let (status, text) = post_json(
        &base,
        &request_body("materials", "ti64_lattice", yielded_materials_parameters()),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'position_m[0]'"), "{text}");
    assert!(text.contains("the inclusive maximum of the ti64_lattice card (YieldReached)"), "{text}");
    assert_no_seal(&text);
    // A step that starts inside every bound and lands past yield is the card's own refusal.
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let near_yield = 0.999 * ti64_lattice::uniaxial_yield_strain() * cell.length_m[0];
    let mut parameters = yielded_materials_parameters();
    parameters["position_m"] = json!([near_yield, 0.0, 0.0]);
    parameters["velocity_m_s"] = json!([22.0, 0.0, 0.0]);
    parameters["start_energy_anchor_j"] = json!(
        0.5 * cell.stiffness_n_m()[0] * near_yield * near_yield + 0.5 * cell.mass_kg() * 22.0 * 22.0
    );
    let (status, text) = post_json(&base, &request_body("materials", "ti64_lattice", parameters)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("yielded"), "{text}");
    assert!(text.contains("880"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn copper_melt_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    let (status, text) = post_json(
        &base,
        &request_body("energy", "grid_node", melting_energy_parameters()),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("1357.77"), "{text}");
    assert!(text.contains("melt"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn caller_cannot_set_the_copper_melt_point() {
    let base = spawn_app().await;
    let mut parameters = energy_parameters(25.0);
    parameters["melt_k"] = json!(300.0);
    let (status, text) = post_json(&base, &request_body("energy", "grid_node", parameters)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("1357.77"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn caller_cannot_set_the_ti64_beta_transus_or_yield() {
    let base = spawn_app().await;
    let mut beta = materials_parameters();
    beta["beta_transus_k"] = json!(500.0);
    let (status, text) = post_json(&base, &request_body("materials", "ti64_lattice", beta)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("1268.15"), "{text}");
    assert_no_seal(&text);

    let mut yield_strength = materials_parameters();
    yield_strength["yield_strength_mpa"] = json!(1.0);
    let (status, text) = post_json(
        &base,
        &request_body("materials", "ti64_lattice", yield_strength),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("880"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_check_stays_up() {
    let base = spawn_app().await;
    let client = Client::new();
    let mut last_error = None;
    for _ in 0..20 {
        match client.get(format!("{base}/health")).send().await {
            Ok(response) => {
                assert_eq!(response.status(), 200);
                return;
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("health never accepted a connection: {last_error:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subnormal_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    let mut parameters = materials_parameters();
    parameters["dt_s"] = serde_json::from_str("1e-320").unwrap();
    let (status, text) = post_json(
        &base,
        &request_body("materials", "ti64_lattice", parameters),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("not a finite normal number"), "{text}");
    assert_no_seal(&text);

    let mut components = materials_parameters();
    components["position_m"] = serde_json::from_str("[0.0, 5e-324, 0.0]").unwrap();
    let (status, text) = post_json(
        &base,
        &request_body("materials", "ti64_lattice", components),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("position_m[1]"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dt_past_the_card_ceiling_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    let mut parameters = materials_parameters();
    let dt = parameters["dt_s"].as_f64().unwrap() * 10.0;
    parameters["dt_s"] = json!(dt);
    let (status, text) = post_json(
        &base,
        &request_body("materials", "ti64_lattice", parameters),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(
        text.contains("the step ceiling of the ti64_lattice card"),
        "{text}"
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_one_second_step_is_above_the_fuse_ceiling_and_does_not_seal() {
    let base = spawn_app().await;
    let mut parameters = energy_parameters(25.0);
    parameters["dt_s"] = json!(1.0);
    let (status, text) = post_json(&base, &request_body("energy", "grid_node", parameters)).await;
    assert_eq!(status, 422, "{text}");
    assert!(
        text.contains("the step ceiling of the grid_node card"),
        "{text}"
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn force_past_nyquist_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    let mut parameters = materials_parameters();
    parameters["force_n"] = json!([1.0e20, 0.0, 0.0]);
    let (status, text) = post_json(
        &base,
        &request_body("materials", "ti64_lattice", parameters),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("Nyquist"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_cards_seal_the_integrated_frame() {
    let base = spawn_app().await;
    let cases = [
        ("energy", "thermal", thermal_parameters()),
        ("signal", "resonance", resonance_parameters()),
        ("energy", "swing", swing_parameters()),
        ("energy", "swing", dispatched_swing_parameters()),
        ("signal", "tesseract", tesseract_parameters()),
        ("materials", "hydraulic_fluid", hydraulic_parameters()),
        ("materials", "cable_stretch", cable_parameters()),
        ("energy", "battery_sag", battery_parameters()),
        ("energy", "cell_18650pf", cell_parameters()),
        ("energy", "generator_mmrtg_f1", generator_parameters(0.0)),
        ("aerospace", "capsule_stardust", capsule_entry_parameters()),
        (
            "aerospace",
            "satellite_lageos1",
            satellite_example_parameters(),
        ),
        ("energy", "reactor_spert3", reactor_parameters()),
        ("energy", "cell_2170_m80", cell_2170_parameters()),
        ("signal", "cantilever_eth", cantilever_parameters()),
        ("energy", "cell_2170_model3", cell_2170_model3_parameters()),
        ("materials", "creep_cmsx6", creep_parameters()),
        ("aerospace", "capsule_insight", capsule_insight_parameters()),
        ("energy", "reactor_spert3_fuel", reactor_fuel_parameters()),
        ("energy", "reactor_triga_jsi", reactor_triga_parameters()),
        ("energy", "reactor_spert3_ccore", reactor_ccore_parameters()),
        ("aerospace", "crater_ryugu_sci", crater_parameters()),
        ("signal", "link_gothenburg_10010", link_parameters()),
        ("energy", "grid_gb", grid_gb_parameters()),
        ("aerospace", "solar_system", solar_system_parameters()),
        ("materials", "tire_eve_r19_ice", tire_parameters()),
        ("energy", "accumulator_sbo450", accumulator_parameters()),
        ("aerospace", "los_jwst_fine_guide", los_parameters()),
        ("energy", "tokamak_jet_dte1", tokamak_parameters()),
        ("materials", "fingerpad_p02_glass", fingerpad_parameters()),
    ];
    let mut seals = Vec::new();
    for (domain, card, parameters) in cases {
        let seal = assert_real_seal(&base, domain, card, parameters).await;
        assert!(
            !seals.contains(&seal),
            "{domain}/{card} reused a seal"
        );
        seals.push(seal);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn advisory_flags_still_seal() {
    let base = spawn_app().await;
    assert_real_seal(&base, "energy", "thermal", derated_thermal_parameters()).await;
    assert_real_seal(&base, "energy", "swing", slipped_swing_parameters()).await;
    assert_real_seal(
        &base,
        "materials",
        "hydraulic_fluid",
        soft_hydraulic_parameters(),
    )
    .await;
    assert_real_seal(&base, "energy", "battery_sag", sag_warning_parameters()).await;
    assert_real_seal(
        &base,
        "energy",
        "generator_mmrtg_f1",
        generator_parameters(2_060.0),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn named_step_rejects_are_422_and_do_not_seal() {
    let base = spawn_app().await;
    let cases = [
        (
            "materials",
            "cable_stretch",
            slack_cable_parameters(),
            "Slack",
        ),
        (
            "materials",
            "hydraulic_fluid",
            cavitation_parameters(),
            "Cavitation",
        ),
        (
            "energy",
            "swing",
            loss_of_synchronism_parameters(),
            "LossOfSynchronism",
        ),
    ];
    for (domain, card, parameters, needle) in cases {
        let (status, text) =
            post_json(&base, &request_body(domain, card, parameters)).await;
        assert_eq!(status, 422, "{domain}/{card}: {text}");
        assert!(text.contains(needle), "{domain}/{card}: {text}");
        assert_no_seal(&text);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bounded_card_rejects_are_422_name_the_refusal_and_do_not_seal() {
    let base = spawn_app().await;
    let column = hydraulic_fluid::FluidColumn::mineral_oil();
    let oscillator = resonance::DynamicOscillator::mount_10hz();
    let machine = swing::SwingMachine::synchronous_60hz();
    let pack = battery_sag::LiIonPack::pack_12s();
    let cases = [
        // A charging current from half charge lands above 50.4 V.
        (
            "energy",
            "battery_sag",
            json!({ "soc": 0.5, "current_a": -100.0, "dt_s": dt_in_window(pack.gate_dt_s()) }),
            "OvervoltageCutoff",
        ),
        // The start pressure is above the card's ceiling.
        (
            "materials",
            "hydraulic_fluid",
            json!({
                "pressure_pa": hydraulic_fluid::PRESSURE_CEILING_PA + 1.0e5,
                "flow_m3_s": 0.0,
                "dt_s": dt_in_window(column.gate_dt_s()),
            }),
            "PressureAboveCard",
        ),
        // The start displacement carries more energy than a free ring of twice the stroke.
        (
            "signal",
            "resonance",
            json!({
                "displacement_m": 2.0 * resonance::AMPLITUDE_CEILING_M,
                "velocity_m_s": 0.0,
                "t_s": 0.0,
                "energy_anchor_j": 0.0,
                "boundary_work_j": 0.0,
                "accel_m_s2": 0.0,
                "dt_s": dt_in_window(oscillator.gate_dt_s()),
            }),
            "AmplitudeAboveCard",
        ),
        (
            "energy",
            "swing",
            json!({
                "delta_rad": 0.0,
                "delta_omega_rad_s": 1.1 * swing::SPEED_DEVIATION_LIMIT_RAD_S,
                "pll_err_rad": 0.0,
                "p_mech_pu": swing::NOMINAL_P_MECH_PU,
                "dt_s": dt_in_window(machine.gate_dt_s()),
            }),
            "SpeedDeviationOutsideCard",
        ),
        (
            "energy",
            "swing",
            json!({
                "delta_rad": 0.0,
                "delta_omega_rad_s": 0.0,
                "pll_err_rad": 0.0,
                "p_mech_pu": 1.5 * swing::DISPATCH_LIMIT_PU,
                "dt_s": dt_in_window(machine.gate_dt_s()),
            }),
            "DispatchOutsideCard",
        ),
        (
            "materials",
            "ti64_lattice",
            yielded_materials_parameters(),
            "YieldReached",
        ),
    ];
    for (domain, card, parameters, needle) in cases {
        let (status, text) = post_json(&base, &request_body(domain, card, parameters)).await;
        assert_eq!(status, 422, "{domain}/{card}: {text}");
        assert!(text.contains(needle), "{domain}/{card}: {text}");
        assert_no_seal(&text);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_card_constants_are_not_a_load() {
    let base = spawn_app().await;
    let mut parameters = thermal_parameters();
    parameters["capacitance_j_per_k"] = json!(1.0);
    let (status, text) = post_json(&base, &request_body("energy", "thermal", parameters)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("is not a field of this request"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v2_dt_past_the_card_ceiling_is_422() {
    let base = spawn_app().await;
    let mut parameters = resonance_parameters();
    parameters["dt_s"] = json!(parameters["dt_s"].as_f64().unwrap() * 10.0);
    let (status, text) =
        post_json(&base, &request_body("signal", "resonance", parameters)).await;
    assert_eq!(status, 422, "{text}");
    assert!(
        text.contains("the step ceiling of the resonance card"),
        "{text}"
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn frame_overflow_is_422_and_does_not_seal() {
    let base = spawn_app().await;
    let mut parameters = energy_parameters(25.0);
    parameters["current_a"] = json!(1.0e39);
    let (status, text) = post_json(&base, &request_body("energy", "grid_node", parameters)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("frame slot"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_repeated_key_in_a_request_is_refused_and_not_sealed() {
    let base = spawn_app().await;
    let body = r#"{"domain":"energy","card":"thermal","parameters":{"temperature_k":-500.0,"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}"#;
    let (status, text) = post_json(&base, body).await;
    assert_eq!(status.as_u16(), 422, "{text}");
    assert_eq!(
        text,
        "Physical Violation: the key 'temperature_k' is repeated."
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_key_beside_parameters_is_refused_and_not_sealed() {
    let base = spawn_app().await;
    let melt = r#"{"domain":"energy","card":"thermal","parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1},"melt_k":9999.0}"#;
    let (status, text) = post_json(&base, melt).await;
    assert_eq!(status.as_u16(), 422, "{text}");
    assert!(text.contains("'melt_k' is fixed at 1357.77 K"), "{text}");
    assert_no_seal(&text);

    let clock = r#"{"domain":"energy","card":"thermal","dt_s":0.001,"parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}"#;
    let (status, text) = post_json(&base, clock).await;
    assert_eq!(status.as_u16(), 422, "{text}");
    assert!(text.contains("'dt_s' is not a field of this request"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_old_key_orchestrator_is_refused_by_name() {
    let base = spawn_app().await;
    let old = r#"{"domain":"energy","orchestrator":"thermal","parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}"#;
    let beside = r#"{"domain":"energy","card":"thermal","orchestrator":"thermal","parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}"#;
    for body in [old, beside] {
        let (status, text) = post_json(&base, body).await;
        assert_eq!(status.as_u16(), 422, "{body}: {text}");
        assert_eq!(
            text,
            "Physical Violation: 'orchestrator' is not a field of this request."
        );
        assert_no_seal(&text);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_old_domain_pair_is_not_recognized() {
    let base = spawn_app().await;
    let pairs = [
        ("thermal", "energy", "thermal", thermal_parameters()),
        ("mechanical", "signal", "resonance", resonance_parameters()),
        ("inertial", "signal", "tesseract", tesseract_parameters()),
        ("hydraulics", "materials", "hydraulic_fluid", hydraulic_parameters()),
        ("structures", "materials", "cable_stretch", cable_parameters()),
    ];
    for (old_domain, domain, card, parameters) in pairs {
        let (status, text) =
            post_json(&base, &request_body(old_domain, card, parameters.clone())).await;
        assert_eq!(status, 404, "{old_domain}/{card}: {text}");
        assert_eq!(text, "Card not recognized", "{old_domain}/{card}");
        assert_no_seal(&text);
        // The same parameters seal under the pair the card now has.
        assert_real_seal(&base, domain, card, parameters).await;
    }
}

/// The three cards whose own step ceiling is above 0.1 s, each with a state
/// and a load it holds for one step at that ceiling.
fn cards_with_a_ceiling_above_a_tenth_of_a_second() -> [(&'static str, f64, Value); 3] {
    let plate = thermal::ThermalNode::avionics_cold_plate();
    let pack = battery_sag::LiIonPack::pack_12s();
    let specimen = cell_18650pf::Cell18650Pf::cell();
    [
        (
            "thermal",
            plate.gate_dt_s().unwrap(),
            json!({
                "temperature_k": thermal::REFERENCE_TEMPERATURE_K,
                "heat_power_w": 25.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K,
            }),
        ),
        (
            "battery_sag",
            pack.gate_dt_s().unwrap(),
            json!({ "soc": 0.8, "current_a": 22.0 }),
        ),
        (
            "cell_18650pf",
            specimen.gate_dt_s().unwrap(),
            json!({ "soc": 1.0, "held_current_a": 2.9, "current_a": 2.9 }),
        ),
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_at_the_card_ceiling_lands_and_the_next_number_up_is_refused() {
    let base = spawn_app().await;
    for (card, ceiling, mut parameters) in cards_with_a_ceiling_above_a_tenth_of_a_second() {
        assert!(ceiling > 0.1, "{card} {ceiling}");
        parameters["dt_s"] = json!(ceiling);
        assert_real_seal(&base, "energy", card, parameters.clone()).await;

        let above = f64::from_bits(ceiling.to_bits() + 1);
        parameters["dt_s"] = json!(above);
        let body = request_body("energy", card, parameters);
        let sent: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(sent["parameters"]["dt_s"].as_f64().unwrap().to_bits(), above.to_bits());
        let (status, text) = post_json(&base, &body).await;
        assert_eq!(status, 422, "{card}: {text}");
        assert_eq!(
            text,
            format!(
                "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the {card} card.",
                protocol_mcp_gateway::message_number(above),
                protocol_mcp_gateway::message_number(ceiling)
            )
        );
        assert_no_seal(&text);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_tenth_of_a_second_and_one_second_are_steps_like_any_other() {
    let base = spawn_app().await;
    for (card, ceiling, mut parameters) in cards_with_a_ceiling_above_a_tenth_of_a_second() {
        let mut seals = Vec::new();
        for dt_s in [0.05, 0.1, 0.5, 1.0, 2.0] {
            assert!(dt_s < ceiling, "{card} {dt_s}");
            parameters["dt_s"] = json!(dt_s);
            seals.push(assert_real_seal(&base, "energy", card, parameters.clone()).await);
        }
        seals.sort();
        seals.dedup();
        assert_eq!(seals.len(), 5, "{card}: each step has its own seal");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_sealed_step_names_its_card() {
    let base = spawn_app().await;
    // The cell card also takes the held current of its discharge; the three
    // inputs the two cards share are the same.
    let parameters = json!({ "soc": 1.0, "current_a": 2.9, "dt_s": 0.1 });
    let mut cell_parameters = parameters.clone();
    cell_parameters["held_current_a"] = json!(2.9);
    let pack = request_body("energy", "battery_sag", parameters.clone());
    let cell = request_body("energy", "cell_18650pf", cell_parameters.clone());
    let pack_request: Value = serde_json::from_str(&pack).unwrap();
    let cell_request: Value = serde_json::from_str(&cell).unwrap();
    for key in ["soc", "current_a", "dt_s"] {
        assert_eq!(pack_request["parameters"][key], cell_request["parameters"][key]);
    }
    assert_ne!(
        commit_request_proof(&pack_request),
        commit_request_proof(&cell_request)
    );
    // The card name alone is in the proof: the same four parameters under the
    // two names commit to different proofs. The commitment is computed
    // offline, so the pack-named request need not be admitted.
    let same_parameters: Value =
        serde_json::from_str(&request_body("energy", "battery_sag", cell_parameters.clone())).unwrap();
    assert_eq!(same_parameters["parameters"], cell_request["parameters"]);
    assert_ne!(
        commit_request_proof(&same_parameters),
        commit_request_proof(&cell_request)
    );

    let pack_seal = assert_real_seal(&base, "energy", "battery_sag", parameters.clone()).await;
    let cell_seal = assert_real_seal(&base, "energy", "cell_18650pf", cell_parameters).await;
    assert_ne!(pack_seal, cell_seal);

    let (status, text) = post_json(&base, &pack).await;
    assert_eq!(status, 200, "{text}");
    assert!(text.starts_with(&format!("{{\"inputs\":{pack},")), "{text}");
    let (status, text) = post_json(&base, &cell).await;
    assert_eq!(status, 200, "{text}");
    assert!(text.starts_with(&format!("{{\"inputs\":{cell},")), "{text}");
}

/// The worked step in the map: the request, its sealed text, its SHA-256, and
/// the 16 proof bytes at its head.
#[test]
fn the_step_proof_is_the_head_of_the_hash_of_the_whole_request() {
    let request: Value = serde_json::from_str(
        r#"{"domain":"energy","card":"thermal","parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}"#,
    )
    .unwrap();
    let sealed = r#"{"card":"thermal","domain":"energy","parameters":{"ambient_k":x4072526666666666,"dt_s":x3fb999999999999a,"heat_power_w":x3ff0000000000000,"temperature_k":x4072526666666666}}"#;
    let digest = sha256_hex(sealed.as_bytes());
    assert_eq!(digest, "f7a263c5660593ffa2a9cb60eed8b70f108b70485725b694f59321a2a0dc70e1");
    assert_eq!(hex::encode(commit_request_proof(&request)), &digest[..32]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_generator_flag_appears_after_day_2060_and_the_step_still_seals() {
    let base = spawn_app().await;
    let mut seals = Vec::new();
    for (mission_day, past) in [(0.0, false), (2_059.0, false), (2_060.0, true), (4_086.0, true)] {
        let parameters = generator_parameters(mission_day);
        seals.push(
            assert_real_seal(&base, "energy", "generator_mmrtg_f1", parameters.clone()).await,
        );
        let (status, text) = post_json(
            &base,
            &request_body("energy", "generator_mmrtg_f1", parameters),
        )
        .await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["state"]["past_fitting_span"], past, "day {mission_day}");
        assert_eq!(body["state"]["t_s"], (mission_day + 1.0) * DAY_S);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_generator_mission_time_off_the_card_is_refused_and_does_not_seal() {
    let base = spawn_app().await;
    const OFF_CARD: &str = "Physical Failure: mission time is before landing or after mission day 4,087 (MissionTimeOutsideCard). That state is not sealed.";
    let off_card = |mission_time_s: f64| {
        let (side, bound, end) = if mission_time_s < 0.0 {
            ("below", 0.0, false)
        } else {
            ("above", generator_mmrtg_f1::MISSION_TIME_MAX_S, true)
        };
        if end {
            format!(
                "Physical Failure: '{}' {} s is above {} s, the end of the generator_mmrtg_f1 card (MissionTimeOutsideCard). That state is not sealed.",
                "mission_time_s",
                protocol_mcp_gateway::message_number(mission_time_s),
                protocol_mcp_gateway::message_number(bound)
            )
        } else {
            format!(
                "Physical Failure: 'mission_time_s' {} s is {side} {} s, the inclusive minimum of the generator_mmrtg_f1 card (MissionTimeOutsideCard). That state is not sealed.",
                protocol_mcp_gateway::message_number(mission_time_s),
                protocol_mcp_gateway::message_number(bound)
            )
        }
    };
    let last_s = generator_mmrtg_f1::MISSION_TIME_MAX_S;
    assert_eq!(last_s, 353_116_800.0);
    let above = f64::from_bits(last_s.to_bits() + 1);
    for mission_time_s in [-1.0, -DAY_S, above, 2.0 * last_s] {
        let parameters = json!({ "mission_time_s": mission_time_s, "dt_s": DAY_S });
        let (status, text) = post_json(
            &base,
            &request_body("energy", "generator_mmrtg_f1", parameters),
        )
        .await;
        assert_eq!(status, 422, "{mission_time_s}: {text}");
        assert_eq!(text, off_card(mission_time_s), "{mission_time_s}");
        assert_no_seal(&text);
    }
    // The last day is on the card. A step from it would leave, and is refused whole.
    let (status, text) = post_json(
        &base,
        &request_body(
            "energy",
            "generator_mmrtg_f1",
            json!({ "mission_time_s": last_s, "dt_s": DAY_S }),
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, OFF_CARD);
    assert_no_seal(&text);

    // A start time inside the published range is still refused when it is
    // not a normal number. The description states that rule once.
    let subnormal = r#"{"domain":"energy","card":"generator_mmrtg_f1","parameters":{"mission_time_s":1e-310,"dt_s":86400.0}}"#;
    let (status, text) = post_json(&base, subnormal).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        "Physical Violation: 'mission_time_s' is not a finite normal number."
    );
    assert_no_seal(&text);

    // A number that is not finite never reaches the card.
    for written in ["NaN", "Infinity", "1e309"] {
        let body = format!(
            r#"{{"domain":"energy","card":"generator_mmrtg_f1","parameters":{{"mission_time_s":{written},"dt_s":86400.0}}}}"#
        );
        let (status, text) = post_json(&base, &body).await;
        assert_eq!(status, 400, "{written}: {text}");
        assert_no_seal(&text);
    }
    let (status, text) = post_json(
        &base,
        &request_body(
            "energy",
            "generator_mmrtg_f1",
            json!({ "mission_time_s": 1.0e39, "dt_s": DAY_S }),
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("frame slot"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_generator_takes_mission_time_and_dt_and_nothing_else() {
    let base = spawn_app().await;
    for key in [
        "thermal_power_w",
        "converter_fading_fraction",
        "electrical_power_w",
        "load",
    ] {
        let mut parameters = generator_parameters(0.0);
        parameters[key] = json!(1.0);
        let (status, text) = post_json(
            &base,
            &request_body("energy", "generator_mmrtg_f1", parameters),
        )
        .await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(
            text,
            format!("Physical Violation: '{key}' is not a field of this request."),
        );
        assert_no_seal(&text);
    }
    for key in ["mission_time_s", "dt_s"] {
        let mut parameters = generator_parameters(0.0);
        parameters.as_object_mut().unwrap().remove(key);
        let (status, text) = post_json(
            &base,
            &request_body("energy", "generator_mmrtg_f1", parameters),
        )
        .await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}'"));
        assert_no_seal(&text);
    }

    // The ceiling is the card's own, with nothing special for this card.
    let ceiling = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity()
        .gate_dt_s()
        .unwrap();
    assert!(ceiling > 4_224_000.0 && ceiling < 4_225_000.0, "{ceiling}");
    let at = json!({ "mission_time_s": 0.0, "dt_s": ceiling });
    assert_real_seal(&base, "energy", "generator_mmrtg_f1", at).await;
    let above = f64::from_bits(ceiling.to_bits() + 1);
    let (status, text) = post_json(
        &base,
        &request_body(
            "energy",
            "generator_mmrtg_f1",
            json!({ "mission_time_s": 0.0, "dt_s": above }),
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        format!(
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the generator_mmrtg_f1 card.",
            protocol_mcp_gateway::message_number(above),
            protocol_mcp_gateway::message_number(ceiling)
        )
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_capsule_flag_appears_near_the_drogue_condition_and_past_it_a_step_is_refused() {
    const PAST: &str = "Physical Failure: the state is under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, where the capsule flew on a parachute (PastFlightDeployment). That state is not sealed.";
    let base = spawn_app().await;
    let mut seals = Vec::new();
    // On the locked entry path: early, at the peak load, in the drogue
    // window, and the step that first crosses both deployment conditions.
    for (steps, window) in [
        (0, false),
        (1_188, false),
        (2_700, true),
        (2_740, true),
        (2_778, false),
    ] {
        let from = capsule_after(steps);
        let parameters = capsule_parameters(
            from.radius_m,
            from.radial_speed_m_s,
            from.transverse_speed_m_s,
        );
        seals.push(
            assert_real_seal(&base, "aerospace", "capsule_stardust", parameters.clone()).await,
        );
        let (status, text) = post_json(
            &base,
            &request_body("aerospace", "capsule_stardust", parameters),
        )
        .await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["state"]["drogue_mach_window"], window, "step {steps}: {text}");
        assert!(body["state"].get("past_flight_deployment").is_none(), "{text}");
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 5);
    // From the state that step lands on the next step is refused by name.
    // Engine 0.6.0 sealed it, and every step after it down to 30 km.
    for steps in [2_779] {
        let from = capsule_after(steps);
        let parameters = capsule_parameters(
            from.radius_m,
            from.radial_speed_m_s,
            from.transverse_speed_m_s,
        );
        let (status, text) = post_json(
            &base,
            &request_body("aerospace", "capsule_stardust", parameters),
        )
        .await;
        assert_eq!(status, 422, "step {steps}: {text}");
        assert_eq!(text, PAST, "step {steps}");
        assert_no_seal(&text);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_capsule_state_off_the_card_is_refused_and_does_not_seal() {
    const ALTITUDE: &str = "Physical Failure: altitude is above 134.7 km or below 30 km (AltitudeOutsideCard). That state is not sealed.";
    const MACH: &str = "Physical Failure: the Mach number is below 1.0, or is above 40 below 68.96 km, where the drag table by Mach number ends (MachOutsideCard). That state is not sealed.";
    const AIR_SPEED: &str = "Physical Failure: above 68.96 km the speed through the air is below 11900 m/s or above 12575 m/s, outside the speeds the entry compared with the flight passed through there (AirSpeedOutsideCard). That state is not sealed.";
    const CORRIDOR: &str = "Physical Failure: the state is off the corridor the entry compared with the flight flew from the locked entry state: below 68.96 km, or moving upward above it, it is under 31 km or above 69 km, or its speed through the air or its path angle is outside the corridor at its altitude (OffComparedCorridor). That state is not sealed.";
    const PAST: &str = "Physical Failure: the state is under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, where the capsule flew on a parachute (PastFlightDeployment). That state is not sealed.";
    let base = spawn_app().await;
    let earth_m = capsule_stardust::LOCAL_EARTH_RADIUS_M;
    let entry_radial = capsule_stardust::ENTRY_RADIAL_SPEED_M_S;
    let entry_transverse = capsule_stardust::ENTRY_TRANSVERSE_SPEED_M_S;
    // At rest in the air at 40 km, then falling through it at 100 m/s.
    let with_the_air = capsule_stardust::AIR_ROTATION_RATE_RAD_S * (earth_m + 40_000.0);
    let cases = [
        (earth_m + 134_701.0, entry_radial, entry_transverse, ALTITUDE),
        (earth_m + 140_000.0, entry_radial, entry_transverse, ALTITUDE),
        (earth_m + 29_999.0, -300.0, 400.0, ALTITUDE),
        (earth_m - 1.0, -300.0, 400.0, ALTITUDE),
        (-(earth_m + 40_000.0), -300.0, 400.0, ALTITUDE),
        (earth_m + 40_000.0, 0.0, with_the_air, MACH),
        (earth_m + 40_000.0, -100.0, with_the_air, MACH),
        // Single steps that engine 0.6.0 sealed: 68.96 km at 20 km/s, at rest
        // in the fixed frame at 100 km, 300 m/s through the air at 130 km,
        // and 30.5 km at Mach 1.14.
        (6438188.511478454, -2000.0, 20000.0, AIR_SPEED),
        (6469228.511478454, 0.0, 0.0, AIR_SPEED),
        (6499228.511478454, -300.0, 352.2425992140121, AIR_SPEED),
        (6399728.511478454, -200.0, 626.8541052549, PAST),
        // Mach 62 at 60 km, below the switch to the drag table by Mach number.
        (earth_m + 60_000.0, -2000.0, 20000.0, MACH),
        // Retrograde at the entry speed meets the air at about 13250 m/s.
        (earth_m + 100_000.0, entry_radial, -entry_transverse, AIR_SPEED),
        // Off the corridor the locked entry flew: 40 km at the entry speed,
        // and upward at the entry's air speed at 100 km.
        (earth_m + 40_000.0, entry_radial, entry_transverse, CORRIDOR),
        (earth_m + 100_000.0, -entry_radial, entry_transverse, CORRIDOR),
    ];
    for (radius_m, radial, transverse, message) in cases {
        let (status, text) = post_json(
            &base,
            &request_body(
                "aerospace",
                "capsule_stardust",
                capsule_parameters(radius_m, radial, transverse),
            ),
        )
        .await;
        assert_eq!(status, 422, "{radius_m} {radial} {transverse}: {text}");
        assert_eq!(text, message, "{radius_m} {radial} {transverse}");
        assert_no_seal(&text);
    }
    // The same place at 400 m/s straight down through the air is off the
    // corridor the locked entry flew; engine 0.6.0 sealed it.
    let (status, text) = post_json(
        &base,
        &request_body(
            "aerospace",
            "capsule_stardust",
            capsule_parameters(earth_m + 40_000.0, -400.0, with_the_air),
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, CORRIDOR);
    assert_no_seal(&text);
    // At 1250 m/s through the air on a path sine of -0.2 it is inside the
    // corridor, 1147.51 to 1350.11 m/s at 40 km, and on the card.
    let along_m_s = (1250.0_f64 * 1250.0 - 250.0 * 250.0).sqrt();
    assert_real_seal(
        &base,
        "aerospace",
        "capsule_stardust",
        capsule_parameters(earth_m + 40_000.0, -250.0, with_the_air + along_m_s),
    )
    .await;

    // A number that is not finite never reaches the card.
    for written in ["NaN", "Infinity", "1e309"] {
        let body = format!(
            r#"{{"domain":"aerospace","card":"capsule_stardust","parameters":{{"radius_m":{written},"radial_speed_m_s":-1842.1416729435698,"transverse_speed_m_s":12767.792058801891,"dt_s":0.05}}}}"#
        );
        let (status, text) = post_json(&base, &body).await;
        assert_eq!(status, 400, "{written}: {text}");
        assert_no_seal(&text);
    }
    let (status, text) = post_json(
        &base,
        &request_body(
            "aerospace",
            "capsule_stardust",
            capsule_parameters(1.0e39, entry_radial, entry_transverse),
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("frame slot"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_capsule_takes_its_three_phase_numbers_and_dt_and_nothing_else() {
    let base = spawn_app().await;
    for (key, value) in [
        ("mass_kg", json!(45.8)),
        ("capsule_mass_kg", json!(45.8)),
        ("reference_area_m2", json!(0.51887)),
        ("drag_by_mach", json!([1.5, 1.5])),
        ("air_density_kg_m3", json!(0.0184)),
        ("entry_azimuth_deg", json!(102.9)),
        ("altitude_m", json!(120_000.0)),
        ("load", json!({})),
    ] {
        let mut parameters = capsule_entry_parameters();
        parameters[key] = value;
        let (status, text) = post_json(
            &base,
            &request_body("aerospace", "capsule_stardust", parameters),
        )
        .await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(
            text,
            format!("Physical Violation: '{key}' is not a field of this request."),
        );
        assert_no_seal(&text);
    }
    for key in ["radius_m", "radial_speed_m_s", "transverse_speed_m_s", "dt_s"] {
        let mut parameters = capsule_entry_parameters();
        parameters.as_object_mut().unwrap().remove(key);
        let (status, text) = post_json(
            &base,
            &request_body("aerospace", "capsule_stardust", parameters),
        )
        .await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}'"));
        assert_no_seal(&text);
    }

    // One representable number above the card's 0.05 s ceiling.
    let ceiling = capsule_stardust::CapsuleStardust::sample_return_capsule()
        .gate_dt_s()
        .unwrap();
    assert_eq!(ceiling, 0.05);
    let above = f64::from_bits(ceiling.to_bits() + 1);
    let mut parameters = capsule_entry_parameters();
    parameters["dt_s"] = json!(above);
    let (status, text) = post_json(
        &base,
        &request_body("aerospace", "capsule_stardust", parameters),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        format!(
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the capsule_stardust card.",
            protocol_mcp_gateway::message_number(above),
            protocol_mcp_gateway::message_number(ceiling)
        )
    );
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_pair_in_aerospace_is_404() {
    let base = spawn_app().await;
    for (domain, card) in [
        ("aerospace", "reentry_trajectory"),
        ("aerospace", "capsule"),
        ("aerospace", "generator_mmrtg_f1"),
        ("energy", "capsule_stardust"),
    ] {
        let (status, text) =
            post_json(&base, &request_body(domain, card, capsule_entry_parameters())).await;
        assert_eq!(status, 404, "{domain}/{card}: {text}");
        assert_eq!(text, "Card not recognized", "{domain}/{card}");
        assert_no_seal(&text);
    }
    assert_real_seal(
        &base,
        "aerospace",
        "capsule_stardust",
        capsule_entry_parameters(),
    )
    .await;
}

/// Each of the thirty-one cards with a request it lands, and its own step floor.
fn every_card_with_its_floor() -> [(&'static str, &'static str, Value, f64); 31] {
    [
        (
            "materials",
            "ti64_lattice",
            materials_parameters(),
            ti64_lattice::Ti64LatticeCell::annealed_column()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "grid_node",
            energy_parameters(25.0),
            grid_node::GridNode::copper_fuse_link().floor_dt_s().unwrap(),
        ),
        (
            "energy",
            "swing",
            swing_parameters(),
            swing::SwingMachine::synchronous_60hz().floor_dt_s().unwrap(),
        ),
        (
            "energy",
            "battery_sag",
            battery_parameters(),
            battery_sag::LiIonPack::pack_12s().floor_dt_s().unwrap(),
        ),
        (
            "energy",
            "cell_18650pf",
            cell_parameters(),
            cell_18650pf::Cell18650Pf::cell().floor_dt_s().unwrap(),
        ),
        (
            "energy",
            "thermal",
            thermal_parameters(),
            thermal::ThermalNode::avionics_cold_plate()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "signal",
            "resonance",
            resonance_parameters(),
            resonance::DynamicOscillator::mount_10hz()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "signal",
            "tesseract",
            tesseract_parameters(),
            tesseract::Tesseract::drive_mode().floor_dt_s().unwrap(),
        ),
        (
            "materials",
            "hydraulic_fluid",
            hydraulic_parameters(),
            hydraulic_fluid::FluidColumn::mineral_oil()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "materials",
            "cable_stretch",
            cable_parameters(),
            cable_stretch::Tendon::dyneema().floor_dt_s().unwrap(),
        ),
        (
            "energy",
            "generator_mmrtg_f1",
            generator_parameters(0.0),
            generator_mmrtg_f1::GeneratorMmrtgF1::curiosity()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "aerospace",
            "capsule_stardust",
            capsule_entry_parameters(),
            capsule_stardust::CapsuleStardust::sample_return_capsule()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "aerospace",
            "satellite_lageos1",
            satellite_example_parameters(),
            satellite_lageos1::SatelliteLageos1::lageos1()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "reactor_spert3",
            reactor_parameters(),
            reactor_spert3::ReactorSpert3::e_core_cold_startup()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "cell_2170_m80",
            cell_2170_parameters(),
            cell_2170_m80::Cell2170M80::cell().floor_dt_s().unwrap(),
        ),
        (
            "signal",
            "cantilever_eth",
            cantilever_parameters(),
            cantilever_eth::CantileverEth::first_bending_mode()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "cell_2170_model3",
            cell_2170_model3_parameters(),
            cell_2170_model3::Cell2170Model3::cell()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "materials",
            "creep_cmsx6",
            creep_parameters(),
            creep_cmsx6::CreepCmsx6::at_980_c().floor_dt_s().unwrap(),
        ),
        (
            "aerospace",
            "capsule_insight",
            capsule_insight_parameters(),
            capsule_insight::CapsuleInsight::entry_capsule()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "reactor_spert3_fuel",
            reactor_fuel_parameters(),
            reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "reactor_triga_jsi",
            reactor_triga_parameters(),
            reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "reactor_spert3_ccore",
            reactor_ccore_parameters(),
            reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "aerospace",
            "crater_ryugu_sci",
            crater_parameters(),
            crater_ryugu_sci::CraterRyuguSci::sci_impact()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "signal",
            "link_gothenburg_10010",
            link_parameters(),
            link_gothenburg_10010::LinkGothenburg10010::sublink_587()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "grid_gb",
            grid_gb_parameters(),
            grid_gb::GridGb::interconnector_loss().floor_dt_s().unwrap(),
        ),
        (
            "aerospace",
            "solar_system",
            solar_system_parameters(),
            solar_system::SolarSystem::de440().floor_dt_s().unwrap(),
        ),
        (
            "materials",
            "tire_eve_r19_ice",
            tire_parameters(),
            tire_eve_r19_ice::TireEveR19Ice::on_rig_ice()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "accumulator_sbo450",
            accumulator_parameters(),
            accumulator_sbo450::AccumulatorSbo450::on_darmstadt_rig()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "aerospace",
            "los_jwst_fine_guide",
            los_parameters(),
            los_jwst_fine_guide::LosJwstFineGuide::observatory()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "energy",
            "tokamak_jet_dte1",
            tokamak_parameters(),
            tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode()
                .floor_dt_s()
                .unwrap(),
        ),
        (
            "materials",
            "fingerpad_p02_glass",
            fingerpad_parameters(),
            fingerpad_p02_glass::FingerPadP02Glass::on_glass()
                .floor_dt_s()
                .unwrap(),
        ),
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_at_each_card_floor_lands_and_the_next_number_down_is_refused() {
    let base = spawn_app().await;
    for (domain, card, mut parameters, floor) in every_card_with_its_floor() {
        assert!(floor > 0.0, "{card} {floor}");
        parameters["dt_s"] = json!(floor);
        assert_real_seal(&base, domain, card, parameters.clone()).await;

        let below = f64::from_bits(floor.to_bits() - 1);
        parameters["dt_s"] = json!(below);
        let body = request_body(domain, card, parameters);
        let sent: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            sent["parameters"]["dt_s"].as_f64().unwrap().to_bits(),
            below.to_bits()
        );
        let (status, text) = post_json(&base, &body).await;
        assert_eq!(status, 422, "{card}: {text}");
        assert_eq!(
            text,
            format!(
                "Physical Violation: 'dt_s' {} is below {} s, the step floor of the {card} card.",
                protocol_mcp_gateway::message_number(below),
                protocol_mcp_gateway::message_number(floor)
            )
        );
        assert_no_seal(&text);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_old_gateway_floor_is_below_every_card_floor() {
    let base = spawn_app().await;
    for (domain, card, mut parameters, floor) in every_card_with_its_floor() {
        assert!(floor > 1.0e-18, "{card} {floor}");
        parameters["dt_s"] = json!(1.0e-18);
        let (status, text) = post_json(&base, &request_body(domain, card, parameters)).await;
        assert_eq!(status, 422, "{card}: {text}");
        assert!(
            text.contains(&format!("the step floor of the {card} card")),
            "{card}: {text}"
        );
        assert_no_seal(&text);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_satellite_state_off_the_card_is_refused_and_does_not_seal() {
    const RADIUS: &str = "Physical Failure: distance from the Earth's centre is below 11 997 km or above 12 327 km, the radii the compared runs reached (RadiusOutsideCard). That state is not sealed.";
    const ORBIT: &str = "Physical Failure: the orbit is not closed, or is in neither of the two compared orbits: LAGEOS-1, semi-major axis 12 265 to 12 275 km, eccentricity 0.0034 to 0.0044 and inclination 109.80 to 109.83 degrees, and LAGEOS-2, 12 158 to 12 166 km, 0.0132 to 0.0138 and 52.63 to 52.66 degrees; no other orbit was compared (OrbitOutsideCard). That state is not sealed.";
    let base = spawn_app().await;
    let nominal_m = satellite_lageos1::REFERENCE_RADIUS_M;
    let first_s = satellite_lageos1::EPOCH_MIN_S;
    let last_s = satellite_lageos1::EPOCH_MAX_S;
    let described: Value = serde_json::from_str(
        &Client::new()
            .post(format!("{base}/describe_card"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(json!({ "domain": "aerospace", "card": "satellite_lageos1" }).to_string())
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
    )
    .unwrap();
    let published_last_s = described["phase_fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["name"] == "epoch_utc_s")
        .unwrap()["maximum"]
        .as_f64()
        .unwrap();
    let say = |value: f64, side: &str, bound: f64, kind: &str| {
        format!(
            "Physical Failure: 'epoch_utc_s' {} s is {side} {} s, the inclusive {kind} of the satellite_lageos1 card (EpochOutsideCard). That state is not sealed.",
            protocol_mcp_gateway::message_number(value),
            protocol_mcp_gateway::message_number(bound)
        )
    };
    let epoch_early = say(first_s - 1.0, "below", first_s, "minimum");
    let epoch_late = format!(
        "Physical Failure: 'epoch_utc_s' {} s is above {} s, the end of the satellite_lageos1 card (EpochOutsideCard). That state is not sealed.",
        protocol_mcp_gateway::message_number(last_s + 1.0),
        protocol_mcp_gateway::message_number(last_s)
    );
    let _ = published_last_s;
    let escaping = {
        let (position_m, mut velocity_m_s) = satellite_orbit(nominal_m, nominal_m);
        velocity_m_s[2] *= 1.5;
        (position_m, velocity_m_s)
    };
    let cases = [
        // One metre under the radius band and one metre over it.
        (SATELLITE_EPOCH_UTC_S, satellite_lageos2_apsis(11_996_999.0), RADIUS),
        (SATELLITE_EPOCH_UTC_S, satellite_lageos2_apsis(12_327_001.0), RADIUS),
        // Inside the radius band on an orbit too small, too large, and open.
        (SATELLITE_EPOCH_UTC_S, satellite_orbit(nominal_m, 12_099_000.0), ORBIT),
        (SATELLITE_EPOCH_UTC_S, satellite_orbit(nominal_m, 12_331_000.0), ORBIT),
        (SATELLITE_EPOCH_UTC_S, escaping, ORBIT),
        // Inside the radius band on orbits never compared: a circle over the
        // pole, as 0.6.0's examples were, an equatorial orbit, a retrograde
        // one, and the size of LAGEOS-1 at the inclination of LAGEOS-2.
        (SATELLITE_EPOCH_UTC_S, satellite_orbit_shaped(nominal_m, nominal_m, 0.0, 90.0), ORBIT),
        (SATELLITE_EPOCH_UTC_S, satellite_orbit_shaped(nominal_m, nominal_m, 0.0039, 0.0), ORBIT),
        (SATELLITE_EPOCH_UTC_S, satellite_orbit_shaped(nominal_m, nominal_m, 0.0039, 180.0), ORBIT),
        (SATELLITE_EPOCH_UTC_S, satellite_orbit_shaped(nominal_m, nominal_m, 0.0039, 52.645), ORBIT),
        // One second before the first date and one second after the last.
        (first_s - 1.0, satellite_orbit(nominal_m, nominal_m), epoch_early.as_str()),
        (last_s + 1.0, satellite_orbit(nominal_m, nominal_m), epoch_late.as_str()),
        // The card names the radius before the orbit and the orbit before the date.
        (last_s + 1.0, satellite_lageos2_apsis(11_996_999.0), RADIUS),
        (last_s + 1.0, satellite_orbit(nominal_m, 12_099_000.0), ORBIT),
    ];
    for (epoch_utc_s, (position_m, velocity_m_s), message) in cases {
        let (status, text) = post_json(
            &base,
            &request_body(
                "aerospace",
                "satellite_lageos1",
                satellite_parameters(epoch_utc_s, position_m, velocity_m_s),
            ),
        )
        .await;
        assert_eq!(status, 422, "{epoch_utc_s} {position_m:?}: {text}");
        assert_eq!(text, message, "{epoch_utc_s} {position_m:?}");
        assert_no_seal(&text);
    }

    // The first date is on the card. The last date is too, and a step from
    // it would end after it, so that step is refused whole.
    let (position_m, velocity_m_s) = satellite_orbit(nominal_m, nominal_m);
    assert_real_seal(
        &base,
        "aerospace",
        "satellite_lageos1",
        satellite_parameters(first_s, position_m, velocity_m_s),
    )
    .await;
    let (status, text) = post_json(
        &base,
        &request_body(
            "aerospace",
            "satellite_lageos1",
            satellite_parameters(last_s, position_m, velocity_m_s),
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        "Physical Failure: the time is before 2026-08-23 00:00 or after 2026-09-20 00:00 UTC, the dates of the compared runs (EpochOutsideCard). That state is not sealed."
    );
    assert_no_seal(&text);

    // A number that is not finite never reaches the card.
    for written in ["NaN", "Infinity", "1e309"] {
        let body = format!(
            r#"{{"domain":"aerospace","card":"satellite_lageos1","parameters":{{"epoch_utc_s":840715200.0,"position_m":[{written},0.0,0.0],"velocity_m_s":[0.0,-894.7,5699.6],"dt_s":10.0}}}}"#
        );
        let (status, text) = post_json(&base, &body).await;
        assert_eq!(status, 400, "{written}: {text}");
        assert_no_seal(&text);
    }
    let (status, text) = post_json(
        &base,
        &request_body(
            "aerospace",
            "satellite_lageos1",
            satellite_parameters(SATELLITE_EPOCH_UTC_S, [1.0e39, 0.0, 0.0], velocity_m_s),
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("frame slot"), "{text}");
    assert_no_seal(&text);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_satellite_takes_an_epoch_a_position_a_velocity_and_dt_and_nothing_else() {
    let base = spawn_app().await;
    for (key, value) in [
        ("satellite_mass_kg", json!(406.965)),
        ("mass_kg", json!(406.965)),
        ("gm_earth_m3_s2", json!(3.986004415e14)),
        ("earth_rotation_rate_rad_s", json!(7.29211514670698e-5)),
        ("field_c", json!([[0.0, 0.0], [0.0, 0.0]])),
        ("radiation_coefficient", json!(1.13)),
        ("radius_m", json!(12_270_000.0)),
        ("load", json!({})),
    ] {
        let mut parameters = satellite_example_parameters();
        parameters[key] = value;
        let (status, text) = post_json(
            &base,
            &request_body("aerospace", "satellite_lageos1", parameters),
        )
        .await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(
            text,
            format!("Physical Violation: '{key}' is not a field of this request."),
        );
        assert_no_seal(&text);
    }
    for key in ["epoch_utc_s", "forecast_elapsed_s", "position_m", "velocity_m_s", "dt_s"] {
        let mut parameters = satellite_example_parameters();
        parameters.as_object_mut().unwrap().remove(key);
        let (status, text) = post_json(
            &base,
            &request_body("aerospace", "satellite_lageos1", parameters),
        )
        .await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}'"));
        assert_no_seal(&text);
    }
    let mut flat = satellite_example_parameters();
    flat["position_m"] = json!([12_270_000.0, 0.0]);
    let (status, text) =
        post_json(&base, &request_body("aerospace", "satellite_lageos1", flat)).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        "Physical Violation: 'position_m' must contain three components."
    );
    assert_no_seal(&text);

    // One representable number above the card's 10 s ceiling. The floor is
    // covered with the other cards'.
    let ceiling = satellite_lageos1::SatelliteLageos1::lageos1()
        .gate_dt_s()
        .unwrap();
    assert_eq!(ceiling, 10.0);
    let above = f64::from_bits(ceiling.to_bits() + 1);
    let mut parameters = satellite_example_parameters();
    parameters["dt_s"] = json!(above);
    let (status, text) = post_json(
        &base,
        &request_body("aerospace", "satellite_lageos1", parameters),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        format!(
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the satellite_lageos1 card.",
            protocol_mcp_gateway::message_number(above),
            protocol_mcp_gateway::message_number(ceiling)
        )
    );
    assert_no_seal(&text);
}

/// The float armor asks that a request number fit a 32-bit slot without
/// overflow. It does not round the number to 32 bits: an epoch and a
/// position that no 32-bit number holds are read, used and published as the
/// 64-bit numbers that were sent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_satellite_epoch_and_position_are_read_as_64_bit_numbers() {
    let base = spawn_app().await;
    let epoch_utc_s = SATELLITE_EPOCH_UTC_S + 0.125;
    let (mut position_m, velocity_m_s) = satellite_orbit(
        satellite_lageos1::REFERENCE_RADIUS_M,
        satellite_lageos1::REFERENCE_RADIUS_M,
    );
    position_m[0] += 0.123_456_789;
    position_m[1] = 0.000_123_456_789;
    assert_ne!(f64::from(epoch_utc_s as f32), epoch_utc_s);
    assert_ne!(f64::from(position_m[0] as f32), position_m[0]);
    let parameters = satellite_parameters(epoch_utc_s, position_m, velocity_m_s);
    assert_real_seal(&base, "aerospace", "satellite_lageos1", parameters.clone()).await;
    let (status, text) = post_json(
        &base,
        &request_body("aerospace", "satellite_lageos1", parameters),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    // The request's epoch is read as sent. The state's epoch is the epoch of
    // that state: the start epoch plus the time elapsed.
    assert_eq!(
        body["inputs"]["parameters"]["epoch_utc_s"].as_f64().unwrap().to_bits(),
        epoch_utc_s.to_bits()
    );
    let elapsed_s = body["state"]["t_s"].as_f64().unwrap();
    assert!(elapsed_s > 0.0);
    assert_eq!(
        body["state"]["epoch_utc_s_hex"],
        format!("{:016x}", (epoch_utc_s + elapsed_s).to_bits())
    );
    assert_eq!(
        body["inputs"]["parameters"]["position_m"][0]
            .as_f64()
            .unwrap()
            .to_bits(),
        position_m[0].to_bits()
    );
}
