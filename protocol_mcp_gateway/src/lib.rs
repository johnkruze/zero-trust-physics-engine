use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(test)]
use std::time::Duration;

use axum::async_trait;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, FromRequest, State};
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::{
    battery_sag, cable_stretch, cantilever_eth, capsule_stardust, cell_18650pf, cell_2170_m80,
    cell_2170_model3, generator_mmrtg_f1, grid_node, hydraulic_fluid, reactor_spert3,
    reactor_spert3_fuel, resonance, satellite_lageos1, swing, tesseract, thermal, ti64_lattice,
    reactor_triga_jsi,
    reactor_spert3_ccore,
    creep_cmsx6,
    capsule_insight,
    crater_ryugu_sci,
    link_gothenburg_10010,
    grid_gb,
    solar_system,
    tire_eve_r19_ice,
    accumulator_sbo450,
    los_jwst_fine_guide,
    tokamak_jet_dte1,
    fingerpad_p02_glass,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// One in-flight RK4 `spawn_blocking` task per core, hard-capped at 32.
/// A full pool fails closed. Excess calls never queue on the blocking pool.
const PHYSICS_PERMIT_CAP: usize = 32;


pub const ENGINE_VERSION: &str = "genesis_kernel_v0.9.0";

pub mod dataset;
mod descriptor;
pub mod json_text;
mod live;
pub mod framing;
pub mod guard;
pub mod hosted;
pub mod keys;
pub mod mcp_server;
mod check_by_request;
mod cite;
mod find_model;
mod flip;
pub mod identity;
pub mod long_work;
mod open_plan;
mod chain;
mod couplings;
mod fresh_start;
mod receipt_check;
mod solution;
mod solve_plan;
/// What the check of a whole solution says it compared, in every answer.
pub use solution::WHOLE_SOLUTION_CHECKED;
mod solver_ablator_slab;
mod solver_ablator_slab_stardust;
mod solver_argon_pair_gas;
mod solver_step_flow_armaly;
mod solver_pipe_flow_superpipe;
mod solver_culture_trichoderma_ambr250;
mod solver_atom_dft;
mod solver_journal_bearing_sheffield;
mod solver_propeller_apc_10x7e;
mod solver_modulus_bam_s355;
mod solver_seawater_column;
mod solver_fusion_pb11_lhd;
mod solver_tokamak_dt_beam_target;
mod solver_fusion_pb11_shared;
mod solver_fusion_pb11_thermal;
mod solvers;
mod trajectory;
mod readout;
mod ends;
mod validation;
pub mod view;
pub mod open_door;

pub(crate) use live::{DoorError, Live, StepFailure};

/// `π/ω`. The same value `card_clock` compares against `dt_s`.
fn nyquist_limit_s(omega_rad_s: f64) -> f64 {
    std::f64::consts::PI / omega_rad_s
}

pub fn new_gateway_state() -> AppState {
    AppState::new(physics_permit_count())
}

#[derive(Clone)]
pub struct AppState {
    physics_permits: Arc<Semaphore>,
    /// How many permits the physics pool was made with.
    physics_total: usize,
    /// The caller's share of the physics pool: a client of the hosted door
    /// holds at most as many physics permits as this has. The stdio door and
    /// the plain HTTP door have none.
    share: Option<Arc<Semaphore>>,
    /// One permit for each dataset job in flight: half the physics permits,
    /// and at least one. A dataset job also holds one physics permit.
    dataset_permits: Arc<Semaphore>,
    /// The directory dataset files go to, named when the gateway starts.
    /// Without one, `return: "file"` and a stored dataset's check are refused.
    dataset_dir: Option<Arc<PathBuf>>,
    /// A check of a stored dataset reads the frame file as a stream but holds
    /// the rows file whole and its parsed rows with it: about 7 times the
    /// size of the rows file, 1.2 GB at the 100,000-row limit. One runs at a
    /// time.
    stored_checks: Arc<Semaphore>,
    /// What the caller may see. The stdio door and the plain HTTP door hold
    /// the full view.
    view: Arc<view::View>,
    /// Unit tests make one row slow or make it panic, and read how many
    /// finished rows waited for their turn at most.
    #[cfg(test)]
    row_hook: Option<dataset::RowHook>,
    #[cfg(test)]
    window_high_water: Arc<std::sync::atomic::AtomicUsize>,
    /// Unit tests hold a taken permit after the step so a spike can observe
    /// HTTP 429. The production binary does not compile this field or the sleep.
    #[cfg(test)]
    permit_hold: Duration,
}

impl AppState {
    fn new(permits: usize) -> Self {
        Self {
            physics_permits: Arc::new(Semaphore::new(permits.max(1))),
            physics_total: permits.max(1),
            share: None,
            dataset_permits: Arc::new(Semaphore::new(dataset_permit_count(permits))),
            dataset_dir: None,
            stored_checks: Arc::new(Semaphore::new(1)),
            view: Arc::new(view::View::full()),
            #[cfg(test)]
            row_hook: None,
            #[cfg(test)]
            window_high_water: Arc::default(),
            #[cfg(test)]
            permit_hold: Duration::ZERO,
        }
    }

    /// The same state with a directory for dataset files. The directory has
    /// been through [`prepare_dataset_dir`].
    pub fn with_dataset_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.dataset_dir = dir.map(Arc::new);
        self
    }

    /// The same state, seen by one caller. The permit pools are shared with
    /// the state it is made from.
    pub fn with_view(mut self, view: Arc<view::View>) -> Self {
        self.view = view;
        self
    }

    /// The same state, with a share of the physics pool: the caller holds at
    /// most `share.available_permits()` physics permits at once. The pools are
    /// shared with the state it is made from.
    pub fn with_share(mut self, share: Arc<Semaphore>) -> Self {
        self.share = Some(share);
        self
    }

    /// How many permits the physics pool was made with.
    pub fn physics_pool_size(&self) -> usize {
        self.physics_total
    }

    pub fn view(&self) -> &view::View {
        &self.view
    }

    #[cfg(test)]
    fn with_hold(permits: usize, permit_hold: Duration) -> Self {
        Self {
            physics_permits: Arc::new(Semaphore::new(permits.max(1))),
            physics_total: permits.max(1),
            share: None,
            dataset_permits: Arc::new(Semaphore::new(dataset_permit_count(permits))),
            dataset_dir: None,
            stored_checks: Arc::new(Semaphore::new(1)),
            view: Arc::new(view::View::full()),
            row_hook: None,
            window_high_water: Arc::default(),
            permit_hold,
        }
    }
}

/// The directory named by `--dataset-dir`, made ready: it must exist, be a
/// directory, and accept a new file. The answer is its absolute path, or a
/// sentence for why the gateway cannot start with it.
pub fn prepare_dataset_dir(path: &Path) -> Result<PathBuf, String> {
    let named = path.display();
    let absolute = std::fs::canonicalize(path)
        .map_err(|error| format!("the dataset directory {named} cannot be used: {error}."))?;
    if !absolute.is_dir() {
        return Err(format!("the dataset directory {named} is not a directory."));
    }
    let probe = absolute.join(format!(".write_probe_{}", std::process::id()));
    std::fs::write(&probe, b"")
        .and_then(|()| std::fs::remove_file(&probe))
        .map_err(|error| format!("the dataset directory {named} cannot be written: {error}."))?;
    Ok(absolute)
}

/// The directory a binary was started with: `--dataset-dir <path>` or
/// `--dataset-dir=<path>`, given at most once. No other argument is read,
/// and an unknown one is refused, so a misspelled flag does not start a
/// gateway that cannot write files. The directory is made ready.
pub fn dataset_dir_from_args(
    arguments: impl IntoIterator<Item = String>,
) -> Result<Option<PathBuf>, String> {
    let mut named: Option<String> = None;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        let value = if argument == "--dataset-dir" {
            arguments
                .next()
                .ok_or_else(|| "--dataset-dir needs a directory.".to_string())?
        } else if let Some(value) = argument.strip_prefix("--dataset-dir=") {
            value.to_string()
        } else {
            return Err(format!(
                "'{argument}' is not an argument of this program. It reads only --dataset-dir <path>."
            ));
        };
        if named.replace(value).is_some() {
            return Err("--dataset-dir was given more than once.".to_string());
        }
    }
    named.map(|path| prepare_dataset_dir(Path::new(&path))).transpose()
}

/// Dataset jobs together hold at most half the physics permits, and at least one.
fn dataset_permit_count(physics_permits: usize) -> usize {
    (physics_permits.max(1) / 2).max(1)
}

pub fn physics_permit_count() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(16)
        .clamp(1, PHYSICS_PERMIT_CAP)
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VerifyPhysicsRequest {
    pub domain: String,
    pub card: String,
    pub parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyPhysicsResponse {
    pub inputs: serde_json::Value,
    pub engine_version: String,
    /// Post-step card state. Each f64 has a `{field}_hex` sibling: 16 lowercase
    /// hex digits of `f64::to_bits`. That hex is the exact value. The decimal
    /// token is whatever `exact_f64_token` found. A client reconstructs the
    /// f64 from the hex.
    #[serde(serialize_with = "serialize_state_value")]
    pub state: serde_json::Value,
    /// JSON number that parses back to the f64 bits. serde_json's default
    /// float formatter can move the last bit of a residual.
    #[serde(serialize_with = "serialize_exact_f64")]
    pub residual_conservation_error: f64,
    /// Frame seal. SHA-256 of the 64-byte last-state frame. Proves the math
    /// that landed in the frame. It does not cover this JSON object.
    pub sha256_seal: String,
    /// Present only when the request named its upstream run in `from`: the
    /// chain commitment, outside the frame seal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<serde_json::Value>,
    /// Receipt seal. SHA-256 of the exact JSON of every field except this one,
    /// which is the same byte string with this field removed from the wire body.
    /// A hash cannot contain itself. Changing `state`, the residual, the inputs,
    /// or the frame seal breaks this receipt. The frame seal can still match.
    pub receipt_sha256: String,
}

struct IntegratedFrame {
    frame: LastStateFrame64,
    residual_conservation_error: f64,
    state: serde_json::Value,
}

struct Ti64Call {
    cell: ti64_lattice::Ti64LatticeCell,
    state: ti64_lattice::Ti64LatticeState,
    load: ti64_lattice::Ti64Load,
    dt_s: f64,
}

struct GridCall {
    node: grid_node::GridNode,
    state: grid_node::GridNodeState,
    load: grid_node::GridLoad,
    dt_s: f64,
}

struct ThermalCall {
    node: thermal::ThermalNode,
    state: thermal::ThermalNodeState,
    load: thermal::ThermalLoad,
    dt_s: f64,
}

struct ResonanceCall {
    oscillator: resonance::DynamicOscillator,
    state: resonance::OscillatorState,
    load: resonance::OscillatorLoad,
    dt_s: f64,
}

struct SwingCall {
    machine: swing::SwingMachine,
    state: swing::SwingState,
    load: swing::SwingLoad,
    dt_s: f64,
}

struct TesseractCall {
    resonator: tesseract::Tesseract,
    state: tesseract::TesseractState,
    load: tesseract::TesseractLoad,
    dt_s: f64,
}

struct HydraulicCall {
    column: hydraulic_fluid::FluidColumn,
    state: hydraulic_fluid::FluidState,
    load: hydraulic_fluid::FluidLoad,
    dt_s: f64,
}

struct CableCall {
    tendon: cable_stretch::Tendon,
    state: cable_stretch::TendonState,
    load: cable_stretch::TendonLoad,
    dt_s: f64,
}

struct BatteryCall {
    pack: battery_sag::LiIonPack,
    state: battery_sag::PackState,
    load: battery_sag::PackLoad,
    dt_s: f64,
}

struct CellCall {
    cell: cell_18650pf::Cell18650Pf,
    state: cell_18650pf::CellState,
    load: cell_18650pf::CellLoad,
    dt_s: f64,
    /// The one held current of the run: the `held_current_a` the request
    /// states in `initial`, the held current of the 1C discharge the run
    /// belongs to. A segment further from it than the card's band is refused.
    held_current_a: f64,
}

/// The sentence for a segment whose current is not the held current.
fn cell_current_changed(current_a: f64, held: f64) -> String {
    unsealed(&format!(
        "'current_a' {} A is more than {} A from the held current, the {} A of 'held_current_a': a run of the cell_18650pf card is one discharge at one held current, as the tester holds it (CurrentChangedDuringDischarge).",
        message_number(current_a),
        cell_18650pf::HELD_CURRENT_BAND_A,
        message_number(held),
    ))
}

/// The sentence for a current outside the compared span.
fn cell_current_not_compared() -> String {
    unsealed(
        "current is outside [2.89817, 2.90065] A, the 1C currents compared with measurement widened by 0.83 mA; the held 0.87 A was compared and failed (CurrentNotCompared).",
    )
}

/// The sentence for a state below full charge that carries no held current.
fn cell_start_not_at_full_charge() -> String {
    unsealed(
        "state of charge is below 1 and the state carries no held current: every compared discharge started at full charge, and a state below it is taken only as the continuation of a 1C discharge, with that discharge's held current (StartNotAtFullCharge).",
    )
}

/// The 2170 cell holds some thirty kilobytes of tables. One copy serves every
/// call; a call holds a reference to it.
fn cell_2170() -> &'static cell_2170_m80::Cell2170M80 {
    static CELL: std::sync::OnceLock<cell_2170_m80::Cell2170M80> = std::sync::OnceLock::new();
    CELL.get_or_init(cell_2170_m80::Cell2170M80::cell)
}

struct Cell2170Call {
    cell: &'static cell_2170_m80::Cell2170M80,
    state: cell_2170_m80::CellState,
    load: cell_2170_m80::CellLoad,
    dt_s: f64,
}

/// The physical 2170 cell holds its three curves, some five kilobytes. One
/// copy serves every call; a call holds a reference to it.
fn cell_2170_model3_type() -> &'static cell_2170_model3::Cell2170Model3 {
    static CELL: std::sync::OnceLock<cell_2170_model3::Cell2170Model3> =
        std::sync::OnceLock::new();
    CELL.get_or_init(cell_2170_model3::Cell2170Model3::cell)
}

struct Cell2170Model3Call {
    cell: &'static cell_2170_model3::Cell2170Model3,
    state: cell_2170_model3::CellState,
    load: cell_2170_model3::CellLoad,
    dt_s: f64,
    /// The least and the most case temperature of every load read for this
    /// run, so that a span the request shows is refused before its first step.
    planned_least_c: f64,
    planned_most_c: f64,
}

struct GeneratorCall {
    generator: generator_mmrtg_f1::GeneratorMmrtgF1,
    state: generator_mmrtg_f1::GeneratorState,
    load: generator_mmrtg_f1::GeneratorLoad,
    dt_s: f64,
}

struct CapsuleCall {
    capsule: capsule_stardust::CapsuleStardust,
    state: capsule_stardust::CapsuleState,
    load: capsule_stardust::CapsuleLoad,
    dt_s: f64,
}

struct CapsuleInsightCall {
    capsule: capsule_insight::CapsuleInsight,
    state: capsule_insight::CapsuleState,
    load: capsule_insight::CapsuleLoad,
    dt_s: f64,
}

struct CraterCall {
    crater: crater_ryugu_sci::CraterRyuguSci,
    state: crater_ryugu_sci::CraterState,
    load: crater_ryugu_sci::CraterLoad,
    dt_s: f64,
}

struct LosCall {
    card: los_jwst_fine_guide::LosJwstFineGuide,
    state: los_jwst_fine_guide::LosState,
    load: los_jwst_fine_guide::LosLoad,
    dt_s: f64,
}

struct CantileverCall {
    beam: cantilever_eth::CantileverEth,
    state: cantilever_eth::CantileverState,
    load: cantilever_eth::CantileverLoad,
    dt_s: f64,
}

struct CreepCall {
    alloy: creep_cmsx6::CreepCmsx6,
    state: creep_cmsx6::CreepState,
    load: creep_cmsx6::CreepLoad,
    dt_s: f64,
}

struct LinkCall {
    link: link_gothenburg_10010::LinkGothenburg10010,
    state: link_gothenburg_10010::LinkState,
    load: link_gothenburg_10010::LinkLoad,
    dt_s: f64,
}

struct GridGbCall {
    card: grid_gb::GridGb,
    state: grid_gb::GridState,
    load: grid_gb::GridLoad,
    dt_s: f64,
    /// The loss the specimen was opened with: the one loss of the run. A
    /// segment that carries another is refused.
    run_loss_mw: f64,
}

/// The sentence for a segment whose loss is not the first segment's.
fn grid_gb_loss_changed(loss_mw: f64, first: f64) -> String {
    unsealed(&format!(
        "'loss_mw' {} MW is not the {} MW of the first segment: a run of the grid_gb card is one loss at time zero, held for the 60 seconds (LossChangedDuringRun).",
        message_number(loss_mw),
        message_number(first),
    ))
}

struct TireCall {
    tire: tire_eve_r19_ice::TireEveR19Ice,
    state: tire_eve_r19_ice::TireState,
    load: tire_eve_r19_ice::TireLoad,
    dt_s: f64,
}

struct FingerPadCall {
    pad: fingerpad_p02_glass::FingerPadP02Glass,
    state: fingerpad_p02_glass::FingerPadState,
    load: fingerpad_p02_glass::FingerPadLoad,
    dt_s: f64,
}

struct AccumulatorCall {
    accumulator: accumulator_sbo450::AccumulatorSbo450,
    state: accumulator_sbo450::GasState,
    load: accumulator_sbo450::AccumulatorLoad,
    dt_s: f64,
}

struct TokamakCall {
    tokamak: tokamak_jet_dte1::TokamakJetDte1,
    state: tokamak_jet_dte1::TokamakState,
    load: tokamak_jet_dte1::TokamakLoad,
    dt_s: f64,
}

struct ReactorCall {
    reactor: reactor_spert3::ReactorSpert3,
    state: reactor_spert3::ReactorState,
    load: reactor_spert3::ReactorLoad,
    /// The reactivity the specimen was opened with: the one reactivity of the
    /// burst. A segment that carries another is refused.
    burst_reactivity_dollar: f64,
    dt_s: f64,
}

struct ReactorTrigaCall {
    reactor: reactor_triga_jsi::ReactorTrigaJsi,
    state: reactor_triga_jsi::ReactorState,
    load: reactor_triga_jsi::ReactorLoad,
    /// The reactivity the run was opened with. A segment that carries another
    /// is refused before the first step.
    run_reactivity_dollar: f64,
    dt_s: f64,
}

struct ReactorCcoreCall {
    reactor: reactor_spert3_ccore::ReactorSpert3Ccore,
    state: reactor_spert3_ccore::ReactorState,
    load: reactor_spert3_ccore::ReactorLoad,
    /// The load the run was opened with: the one reactivity and the one flow
    /// of the transient. A segment that carries another is refused.
    run_load: reactor_spert3_ccore::ReactorLoad,
    dt_s: f64,
}

struct ReactorFuelCall {
    reactor: reactor_spert3_fuel::ReactorSpert3Fuel,
    state: reactor_spert3_fuel::ReactorState,
    load: reactor_spert3_fuel::ReactorLoad,
    /// The load the specimen was opened with: the one reactivity and the one
    /// flow rate of the burst. A segment that carries another is refused.
    burst_load: reactor_spert3_fuel::ReactorLoad,
    dt_s: f64,
}

struct SolarSystemCall {
    system: solar_system::SolarSystem,
    state: solar_system::SolarSystemState,
    load: solar_system::SolarSystemLoad,
    dt_s: f64,
}

struct SatelliteCall {
    satellite: satellite_lageos1::SatelliteLageos1,
    state: satellite_lageos1::SatelliteState,
    load: satellite_lageos1::SatelliteLoad,
    dt_s: f64,
}

/// What a JSON value is, in the words a refusal uses.
pub(crate) fn kind_of_value(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "a list",
        serde_json::Value::Object(_) => "an object",
    }
}

fn required_f64(params: &serde_json::Value, key: &str) -> Result<f64, String> {
    let value = match params.get(key) {
        None => return Err(format!("Missing required parameter: '{key}'")),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| format!("Physical Violation: '{key}' must be a number; this request has {}.", kind_of_value(value)))?,
    };
    accept_f64(key, value)?;
    Ok(value)
}

/// Finite, normal, and representable in an `f32` frame slot.
///
/// `0.0` is a legal rest component. Subnormals, NaN, infinities, and values
/// that narrow to a non-finite `f32` are rejected before a worker starts.
fn accept_f64(key: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value.is_subnormal() {
        return Err(format!(
            "Physical Violation: '{key}' is not a finite normal number."
        ));
    }
    if value != 0.0 && !(value as f32).is_finite() {
        return Err(format!(
            "Physical Violation: '{key}' does not fit in a finite frame slot."
        ));
    }
    Ok(())
}

fn required_vec3(params: &serde_json::Value, key: &str) -> Result<[f64; 3], String> {
    let values = match params.get(key) {
        None => return Err(format!("Missing required parameter: '{key}'")),
        Some(serde_json::Value::Array(values)) => values,
        Some(other) => {
            return Err(format!(
                "Physical Violation: '{key}' must be a list of three numbers; this request has {}.",
                kind_of_value(other)
            ));
        }
    };
    if values.len() != 3 {
        return Err(format!(
            "Physical Violation: '{key}' must contain three components."
        ));
    }
    let mut out = [0.0; 3];
    for (index, value) in values.iter().enumerate() {
        let label = format!("{key}[{index}]");
        let component = value.as_f64().ok_or_else(|| {
            format!(
                "Physical Violation: '{label}' must be a number; this request has {}.",
                kind_of_value(value)
            )
        })?;
        accept_f64(&label, component)?;
        out[index] = component;
    }
    Ok(out)
}

fn validate_physics(params: &serde_json::Value) -> Result<(), String> {
    walk_physics(params, None)
}

fn walk_physics(value: &serde_json::Value, key: Option<&str>) -> Result<(), String> {
    match value {
        serde_json::Value::Number(number) => {
            let label = key.unwrap_or("value");
            let parsed = number.as_f64().ok_or_else(|| {
                format!("Physical Violation: '{label}' is not a finite normal number.")
            })?;
            accept_f64(label, parsed)?;
            if let Some(name) = key {
                apply_sign_rules(name, parsed)?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let path = match key {
                    Some(name) => format!("{name}[{index}]"),
                    None => format!("[{index}]"),
                };
                walk_physics(item, Some(&path))?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for (child, child_value) in map {
                walk_physics(child_value, Some(child))?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn apply_sign_rules(key: &str, val: f64) -> Result<(), String> {
    let lower = key.to_lowercase();
    // A key that ends in `_c` is a temperature in degrees Celsius, which is
    // below zero for a cold cell. Its card bounds it.
    let celsius = lower.ends_with("_c");
    if (lower.contains("kelvin") || lower.contains("temp")) && !celsius && val < 0.0 {
        return Err(format!(
            "Physical Violation: Temperature '{key}' cannot be below absolute zero (negative Kelvin)."
        ));
    }
    if lower.contains("mass") && val < 0.0 {
        return Err(format!(
            "Physical Violation: Mass '{key}' cannot be negative."
        ));
    }
    if lower.contains("density") && val < 0.0 {
        return Err(format!(
            "Physical Violation: Density '{key}' cannot be negative."
        ));
    }
    if lower.contains("volume") && val < 0.0 {
        return Err(format!(
            "Physical Violation: Volume '{key}' cannot be negative."
        ));
    }
    Ok(())
}

/// A number as a refusal writes it: plain when it is of ordinary size, and
/// in exponent form when plain digits would run to dozens of zeros.
pub fn message_number(value: f64) -> String {
    let size = value.abs();
    if value == 0.0 || (1.0e-4..1.0e15).contains(&size) {
        format!("{value}")
    } else {
        format!("{value:e}")
    }
}

/// The name of the refusal a request value breaks, by card and field, as the
/// card lists it in `describe_card`. `upper` is true for the maximum.
fn published_reject(card: &str, field: &str, upper: bool) -> Option<&'static str> {
    Some(match (card, field, upper) {
        ("ti64_lattice" | "grid_node" | "thermal", "temperature_k" | "ambient_k", _) => {
            "TemperatureOutsideCard"
        }
        ("ti64_lattice", "position_m" | "velocity_m_s", _) => "YieldReached",
        ("ti64_lattice", "start_temperature_k", _) => "TemperatureOutsideCard",
        ("ti64_lattice", "start_t_s", _) => "StartNotWritten",
        ("swing", "delta_rad", _) => "LossOfSynchronism",
        ("cable_stretch", "elongation_m", false) => "Slack",
        ("cable_stretch", "elongation_m", true) => "TensileFailure",
        ("swing", "delta_omega_rad_s", _) => "SpeedDeviationOutsideCard",
        ("swing", "p_mech_pu", _) => "DispatchOutsideCard",
        ("battery_sag", "soc", false) => "UndervoltageCutoff",
        ("battery_sag", "current_a", false) => "OvervoltageCutoff",
        ("battery_sag" | "cell_18650pf", "soc", true) | ("cell_18650pf", "soc", false) => {
            "StateOfChargeOutsideCard"
        }
        ("cell_18650pf", "current_a" | "held_current_a", _) => "CurrentNotCompared",
        ("cell_2170_m80", "block_chamber_c", _) => "BlockNotRecognized",
        ("cell_2170_m80", "charge_removed_ah", _) => "ChargeOutsideTables",
        ("cell_2170_m80", "current_a", _) => "CurrentOutsideCard",
        ("cell_2170_m80", "case_temperature_c", _) => "CaseTemperatureOutsideCard",
        ("crater_ryugu_sci", "radius_m", false) => "RadiusBelowCard",
        ("crater_ryugu_sci", "radius_m", true) => "RadiusAboveFinal",
        ("cell_2170_model3", "cell_number", _) => "CellNotRecognized",
        ("cell_2170_model3", "state_of_charge", _) => "StateOfChargeOutsideCard",
        ("cell_2170_model3", "capacity_ah", _) => "CapacityOutsideCard",
        ("cell_2170_model3", "current_a", _) => "CurrentOutsideCard",
        ("cell_2170_model3", "case_temperature_c", _) => "CaseTemperatureOutsideCard",
        ("cell_2170_model3", "case_temperature_least_c" | "case_temperature_most_c", _) => {
            "CaseTemperatureOutsideCard"
        }
        ("resonance", "displacement_m" | "velocity_m_s", _) => "AmplitudeAboveCard",
        ("resonance", "t_s", _) => "StartTimeOutsideCard",
        ("resonance", "energy_anchor_j", _) => "LedgerNotOfThisState",
        ("cantilever_eth", "deflection_m" | "velocity_m_s", _) => "DeflectionAboveCard",
        ("creep_cmsx6", "creep_extension_percent", false) => "CreepExtensionBelowCard",
        ("creep_cmsx6", "creep_extension_percent", true) => "CreepExtensionOutsideCard",
        ("creep_cmsx6", "stress_mpa", _) => "StressOutsideCard",
        ("capsule_insight", "lift_over_drag", _) => "LiftOutsideCard",
        ("link_gothenburg_10010", "dry_path_loss_db", _) => "DryPathLossOutsideCard",
        ("link_gothenburg_10010", "antenna_water_db", _) => "AntennaWaterOutsideCard",
        ("link_gothenburg_10010", "rain_rate_mm_h" | "gauge_minute_rain_mm_h", _) => {
            "RainRateOutsideCard"
        }
        ("link_gothenburg_10010", "gauge_minute_elapsed_s", _) => "GaugeMinuteOutsideCard",
        ("tire_eve_r19_ice", "distance_m", _) => "TravelBeyondTrack",
        ("tire_eve_r19_ice", "slip_held_s", false) => "SlipNotSettled",
        ("tire_eve_r19_ice", "slip_held_s", true) => "HoldBeyondMeasured",
        ("tire_eve_r19_ice", "normal_load_n", _) => "NormalLoadOutsideCard",
        ("tire_eve_r19_ice", "inflation_pressure_pa", _) => "InflationPressureOutsideCard",
        ("tire_eve_r19_ice", "slip_ratio", _) => "SlipOutsideCard",
        ("tire_eve_r19_ice", "carriage_speed_m_s", _) => "CarriageSpeedOutsideCard",
        ("accumulator_sbo450", "gas_mass_kg", _) => "GasMassOutsideCard",
        ("accumulator_sbo450", "gas_pressure_pa", _) => "PressureOutsideCard",
        ("accumulator_sbo450", "gas_temperature_k", _) => "GasTemperatureOutsideCard",
        ("accumulator_sbo450", "wall_temperature_k", _) => "WallTemperatureOutsideCard",
        ("accumulator_sbo450", "piston_speed_m_s", _) => "PistonSpeedOutsideCard",
        ("accumulator_sbo450", "work_moved_j" | "heat_moved_j", _) => "NegativeMovedSum",
        ("los_jwst_fine_guide", "lock_x_arcsec" | "lock_y_arcsec", _) => "LockOutsideCard",
        ("los_jwst_fine_guide", "quiet_before_start_s", _) => "RecoveryNotElapsed",
        ("los_jwst_fine_guide", "measured", _) => "MeasuredNotZeroOrOne",
        ("los_jwst_fine_guide", "counts_per_s", _) => "CountRateOutsideCard",
        ("hydraulic_fluid", "pressure_pa", false) => "Cavitation",
        ("hydraulic_fluid", "pressure_pa", true) => "PressureAboveCard",
        ("hydraulic_fluid", "flow_m3_s", _) => "FlowAboveAcoustic",
        ("generator_mmrtg_f1", "mission_time_s", _) => "MissionTimeOutsideCard",
        ("satellite_lageos1", "epoch_utc_s", _) => "EpochOutsideCard",
        ("satellite_lageos1", "forecast_elapsed_s", _) => "HorizonOutsideCard",
        ("grid_gb", "frequency_hz", _) => "StartFrequencyOutsideCard",
        ("grid_gb", "inertia_mw_s", _) => "InertiaOutsideCard",
        ("grid_gb", "demand_mw", _) => "DemandOutsideCard",
        ("grid_gb", "loss_mw", _) => "LossOutsideCard",
        ("reactor_spert3", "coolant_temperature_k", _) => "CoolantTemperatureOutsideCard",
        ("reactor_spert3", "reactivity_inserted_dollar", _) => "ReactivityOutsideCard",
        ("reactor_triga_jsi", "start_temperature_k", _) => "StartTemperatureOutsideCard",
        ("reactor_triga_jsi", "rod_table_core", _) => "RodTableOutsideCard",
        ("reactor_triga_jsi", "reactivity_inserted_dollar", _) => "ReactivityOutsideCard",
        ("reactor_triga_jsi", "rods_dropped", _) => "ScramValueOutsideCard",
        ("reactor_spert3_ccore", "initial_temperature_k", _) => "InitialTemperatureOutsideCard",
        ("reactor_spert3_ccore", "system_pressure_psig", _) => "PressureOutsideCard",
        ("reactor_spert3_ccore", "reactivity_inserted_dollar", _) => "ReactivityOutsideCard",
        ("reactor_spert3_ccore", "coolant_flow_m_per_s", _) => "FlowOutsideCard",
        ("reactor_spert3_fuel", "inlet_temperature_k", _) => "InletTemperatureOutsideCard",
        ("reactor_spert3_fuel", "reactivity_inserted_dollar", _) => "ReactivityOutsideCard",
        ("reactor_spert3_fuel", "coolant_flow_m_per_s", _) => "FlowOutsideCard",
        ("fingerpad_p02_glass", "twist_deg", _) => "TwistNotStartedFromZero",
        ("fingerpad_p02_glass", "normal_force_n", _) => "NormalForceOutsideCard",
        ("fingerpad_p02_glass", "twist_rate_deg_s", _) => "TwistRateOutsideCard",
        ("tokamak_jet_dte1", "thermal_energy_j" | "fast_ion_energy_j", _) => "EnergyOutsideComparison",
        ("tokamak_jet_dte1", "elapsed_time_s", _) => "TimeBeyondCard",
        ("tokamak_jet_dte1", "heating_power_w", _) => "HeatingPowerOutsideCard",
        ("tokamak_jet_dte1", "ohmic_power_w", _) => "OhmicPowerOutsideCard",
        ("tokamak_jet_dte1", "plasma_current_a", _) => "PlasmaCurrentOutsideCard",
        ("tokamak_jet_dte1", "toroidal_field_t", _) => "ToroidalFieldOutsideCard",
        ("tokamak_jet_dte1", "tritium_fraction", false) => "FuelMixOutsideCard",
        ("tokamak_jet_dte1", "tritium_fraction", true) => "FuelMixOutsideComparison",
        ("tokamak_jet_dte1", "line_density_per_m3", false) => "DensityBelowCard",
        ("tokamak_jet_dte1", "line_density_per_m3", true) => "DensityAboveComparison",
        _ => return None,
    })
}

/// Which fields of a card a request value is checked against.
#[derive(Clone, Copy)]
enum Published {
    /// A single step: the phase fields and the load fields.
    Step,
    /// A load handed over at a segment: the load fields.
    Load,
}

/// The sentence for a value outside a bound the card publishes for a field,
/// or nothing when the value is inside. It names the field, the value, the
/// bound with its side, the card, and the card's refusal for it.
fn published_outside(card: &str, field: &descriptor::FieldSpec, value: f64) -> Option<String> {
    published_outside_as(card, field, field.name, value)
}

/// The same, with the name the sentence gives the value: the field's own name
/// for a single number, and `name[i]` for one component of a 3-vector.
fn published_outside_as(
    card: &str,
    field: &descriptor::FieldSpec,
    label: &str,
    value: f64,
) -> Option<String> {
    let unit = if field.unit == "1" {
        String::new()
    } else {
        format!(" {}", field.unit)
    };
    let outside = |upper: bool, exclusive: bool, bound: f64| {
        let side = match (upper, exclusive) {
            (false, true) => "at or below",
            (false, false) => "below",
            (true, true) => "at or above",
            (true, false) => "above",
        };
        let how = if exclusive { "exclusive" } else { "inclusive" };
        let kind = if upper { "maximum" } else { "minimum" };
        let name = match published_reject(card, field.name, upper) {
            Some(name) => format!(" ({name})"),
            None => String::new(),
        };
        unsealed(&format!(
            "'{label}' {}{unit} is {side} {}{unit}, the {how} {kind} of the {card} card{name}.",
            message_number(value),
            message_number(bound),
        ))
    };
    if let Some(minimum) = field.minimum {
        if value < minimum || (field.minimum_exclusive && value == minimum) {
            return Some(outside(false, field.minimum_exclusive, minimum));
        }
    }
    // The time axes of the generator and the satellite publish the last start
    // from which a step can land. The last day or date of the card itself is
    // still a legal start; the step from it leaves the card and the run is
    // sealed as refused. A value is held here to the end of the card.
    let end_of_card = match (card, field.name) {
        ("generator_mmrtg_f1", "mission_time_s") => Some(generator_mmrtg_f1::MISSION_TIME_MAX_S),
        ("satellite_lageos1", "epoch_utc_s") => Some(satellite_lageos1::EPOCH_MAX_S),
        _ => None,
    };
    if let Some(end) = end_of_card {
        if value > end {
            let name = published_reject(card, field.name, true).unwrap_or("OutsideCard");
            return Some(unsealed(&format!(
                "'{}' {} {} is above {} {}, the end of the {card} card ({name}).",
                field.name,
                message_number(value),
                field.unit,
                message_number(end),
                field.unit,
            )));
        }
    } else if let Some(maximum) = field.maximum {
        if value > maximum || (field.maximum_exclusive && value == maximum) {
            return Some(outside(true, field.maximum_exclusive, maximum));
        }
    }
    None
}

/// A value outside a bound the card publishes is refused here, before any
/// permit is taken, on every card and in every segment of a run. The bounds
/// are the ones `describe_card` lists, read from the same list. A value that
/// is missing or not a number is left to the reader of that field. A state that
/// leaves the card during a run is not this rule: the step refuses it and the
/// run is sealed as refused.
fn enforce_published(
    domain: &str,
    card: &str,
    scope: Published,
    params: &serde_json::Value,
) -> Result<(), String> {
    enforce_published_fields(domain, card, scope, params, |_| true)
}

/// The rule of [`enforce_published`] for the fields `wanted` picks. A 3-vector
/// that publishes a bound is held to it component by component, and the
/// sentence names the component, `force_n[1]`. A component that is missing or
/// not a number is left to the reader of that field.
fn enforce_published_fields(
    domain: &str,
    card: &str,
    scope: Published,
    params: &serde_json::Value,
    wanted: impl Fn(&str) -> bool,
) -> Result<(), String> {
    let spec = descriptor::find(domain, card).expect("descriptor");
    for field in &spec.fields {
        let in_scope = match (scope, field.role) {
            (Published::Step, _) => true,
            (Published::Load, descriptor::Role::Load) => true,
            (Published::Load, descriptor::Role::Phase) => false,
        };
        if !in_scope || !wanted(field.name) {
            continue;
        }
        if field.components == 1 {
            let Some(value) = params.get(field.name).and_then(serde_json::Value::as_f64) else {
                continue;
            };
            if !value.is_finite() {
                continue;
            }
            if let Some(sentence) = published_outside(card, field, value) {
                return Err(sentence);
            }
            continue;
        }
        if field.minimum.is_none() && field.maximum.is_none() {
            continue;
        }
        let Some(items) = params.get(field.name).and_then(serde_json::Value::as_array) else {
            continue;
        };
        for (index, item) in items.iter().enumerate() {
            let Some(value) = item.as_f64() else {
                continue;
            };
            if !value.is_finite() {
                continue;
            }
            let label = format!("{}[{index}]", field.name);
            if let Some(sentence) = published_outside_as(card, field, &label, value) {
                return Err(sentence);
            }
        }
    }
    Ok(())
}

/// `dt_s` must sit at or above the card's own step floor and at or under its
/// own step ceiling, `floor_dt_s` and `gate_dt_s` in `genesis_kernel`. The
/// gateway has no limit of its own. A card that states no floor or no ceiling
/// is refused. A card with a natural frequency or a pole also keeps `dt_s` at
/// or under `π/ω`. `omega_rad_s` is `None` for a card that has neither.
fn card_clock(
    dt_s: f64,
    omega_rad_s: Option<f64>,
    floor_dt_s: Option<f64>,
    ceiling_dt_s: Option<f64>,
    card: &str,
) -> Result<(), String> {
    accept_f64("dt_s", dt_s)?;
    let floor_dt_s = match floor_dt_s {
        Some(floor_dt_s) if floor_dt_s.is_finite() && floor_dt_s > 0.0 => floor_dt_s,
        _ => {
            return Err(format!(
                "Physical Violation: the {card} card states no step floor."
            ));
        }
    };
    let ceiling_dt_s = match ceiling_dt_s {
        Some(ceiling_dt_s) if ceiling_dt_s.is_finite() && ceiling_dt_s > 0.0 => ceiling_dt_s,
        _ => {
            return Err(format!(
                "Physical Violation: the {card} card states no step ceiling."
            ));
        }
    };
    if dt_s <= 0.0 {
        return Err(format!(
            "Physical Violation: 'dt_s' {} is zero or negative (NonPositiveDt) on the {card} card.",
            message_number(dt_s)
        ));
    }
    if dt_s < floor_dt_s {
        return Err(format!(
            "Physical Violation: 'dt_s' {} is below {} s, the step floor of the {card} card.",
            message_number(dt_s),
            message_number(floor_dt_s)
        ));
    }
    if dt_s > ceiling_dt_s {
        return Err(format!(
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the {card} card.",
            message_number(dt_s),
            message_number(ceiling_dt_s)
        ));
    }
    if let Some(omega_rad_s) = omega_rad_s {
        if !(omega_rad_s.is_finite() && omega_rad_s > 0.0) {
            return Err(format!(
                "Physical Violation: the {card} card has no finite frequency."
            ));
        }
        let nyquist_dt_s = nyquist_limit_s(omega_rad_s);
        if dt_s > nyquist_dt_s {
            return Err(format!(
                "Physical Violation: 'dt_s' {} exceeds the Nyquist limit {} s of the {card} card.",
                message_number(dt_s),
                message_number(nyquist_dt_s)
            ));
        }
    }
    Ok(())
}

/// The constant `C` of the titanium card's load rule `|F| * dt_s <= C`:
/// `omega * shortest ligament * mass`, in newton seconds. `None` when the
/// card has no finite ligament length or mass.
pub(crate) fn ti64_impulse_limit_n_s(cell: &ti64_lattice::Ti64LatticeCell) -> Option<f64> {
    let length_m = min_positive(cell.length_m)?;
    let mass_kg = cell.mass_kg();
    let omega_rad_s = cell.max_natural_frequency_rad_s();
    if !(mass_kg.is_finite() && mass_kg > 0.0 && length_m.is_finite() && length_m > 0.0) {
        return None;
    }
    Some(omega_rad_s * length_m * mass_kg)
}

/// A zero-order-hold force is inside the step when its impulse in the step
/// stays within the card's constant: `|F| * dt_s <= C`, each component, the
/// product rounded once to 64 bits and compared with `C`.
fn enforce_force_nyquist(
    force_n: [f64; 3],
    dt_s: f64,
    cell: &ti64_lattice::Ti64LatticeCell,
) -> Result<(), String> {
    let Some(impulse) = ti64_impulse_limit_n_s(cell) else {
        return Err("Physical Violation: the card has no finite mass or length.".to_string());
    };
    let limit = genesis_kernel::impulse::largest(impulse, dt_s);
    let Some(limit) = limit.filter(|limit| !limit.is_subnormal()) else {
        return Err(
            "Physical Violation: the force Nyquist limit of the RK4 step is not finite."
                .to_string(),
        );
    };
    for (index, component) in force_n.iter().enumerate() {
        if !genesis_kernel::impulse::within(*component, dt_s, impulse) {
            return Err(format!(
                "Physical Violation: 'force_n[{index}]' {} N is larger in size than {} N, the largest the RK4 step accepts at this dt_s (the force Nyquist limit: |F| * dt_s <= C, C = omega * shortest ligament * mass = {} N*s).",
                message_number(*component),
                message_number(limit),
                message_number(impulse)
            ));
        }
    }
    Ok(())
}

/// `|value| * dt_s` stays at or under the card's constant `C`: one product,
/// rounded once to 64 bits, compared with `C`. `limit` is the card's own
/// largest load, named in the sentence. Zero is a hold.
fn enforce_abs_nyquist(
    key: &str,
    value: f64,
    dt_s: f64,
    impulse: f64,
    limit: Option<f64>,
) -> Result<(), String> {
    if value == 0.0 {
        return Ok(());
    }
    match limit {
        Some(limit)
            if limit.is_finite()
                && !limit.is_subnormal()
                && genesis_kernel::impulse::within(value, dt_s, impulse) =>
        {
            Ok(())
        }
        Some(limit) => Err(format!(
            "Physical Violation: '{key}' {} is larger in size than {}, the largest the card accepts at this dt_s (the Nyquist limit of the RK4 step: |{key}| * dt_s may not exceed the card's impulse limit {}).",
            message_number(value),
            message_number(limit),
            message_number(impulse)
        )),
        None => {
            Err("Physical Violation: the Nyquist limit of the RK4 step is not finite.".to_string())
        }
    }
}

fn thermal_on_card(temperature_k: f64) -> bool {
    temperature_k >= thermal::CARD_TEMPERATURE_MIN_K
        && temperature_k < thermal::CARD_TEMPERATURE_MAX_K
}

fn refused(card: &str, reject: impl std::fmt::Debug) -> String {
    let name = format!("{reject:?}");
    match descriptor::reject_meaning(card, &name) {
        Some(meaning) => {
            let meaning = meaning.split_whitespace().collect::<Vec<_>>().join(" ");
            format!("Physical Violation: {card} step refused ({name}): {meaning}")
        }
        None => format!("Physical Violation: {card} step refused ({name})."),
    }
}

/// The card built no starting state from phase values that are inside every
/// bound it publishes. The cause is not a bound: a quantity derived from the
/// values (an energy, a stress, a voltage) is not a finite number.
fn phase_not_valid(card: &str) -> String {
    format!(
        "Physical Violation: the {card} card builds no starting state from these phase values: a quantity derived from them (an energy, a stress or a voltage) is not a finite number. No published bound of the card is the cause."
    )
}

fn unsealed(detail: &str) -> String {
    format!("Physical Failure: {detail} That state is not sealed.")
}

fn ti64_reject(reject: ti64_lattice::StepReject) -> String {
    match reject {
        ti64_lattice::StepReject::YieldReached => unsealed(
            "a Ti-6Al-4V ligament yielded: stress reached the yield strength of 880 MPa (YieldReached).",
        ),
        ti64_lattice::StepReject::SpontaneousGeneration => {
            unsealed("the cell created mechanical energy (SpontaneousGeneration).")
        }
        ti64_lattice::StepReject::FirstLawBreach => {
            unsealed("the cell destroyed mechanical energy (FirstLawBreach).")
        }
        other => refused("ti64_lattice", other),
    }
}

fn thermal_reject(reject: thermal::StepReject) -> String {
    match reject {
        thermal::StepReject::TemperatureOutsideCard => {
            unsealed("the plate left 233.15 K to 398.15 K (TemperatureOutsideCard).")
        }
        thermal::StepReject::NegativeTemperature => {
            unsealed("the plate temperature is not positive (NegativeTemperature).")
        }
        thermal::StepReject::SpontaneousGeneration => {
            unsealed("the plate created heat (SpontaneousGeneration).")
        }
        thermal::StepReject::FirstLawBreach => {
            unsealed("the plate destroyed heat (FirstLawBreach).")
        }
        other => refused("thermal", other),
    }
}

fn resonance_reject(reject: resonance::StepReject) -> String {
    match reject {
        resonance::StepReject::SpontaneousGeneration => unsealed(
            "the mechanical energy is above the anchor plus the booked work by more than the conservation limit: a wrong step, or the step method's own error summed over the run since its anchor (SpontaneousGeneration).",
        ),
        resonance::StepReject::FirstLawBreach => unsealed(
            "the mechanical energy is below the anchor plus the booked work by more than the conservation limit: a wrong step, or the step method's own error summed over the run since its anchor (FirstLawBreach).",
        ),
        resonance::StepReject::AmplitudeAboveCard => unsealed(
            "the mount's mechanical energy is above that of a free ring of twice the stroke (AmplitudeAboveCard).",
        ),
        other => refused("resonance", other),
    }
}

const SWING_SPEED_OUTSIDE_CARD: &str = "the rotor speed deviation is beyond +/- 6 pi rad/s, 3 Hz off the bus (SpeedDeviationOutsideCard).";
const SWING_DISPATCH_OUTSIDE_CARD: &str =
    "the mechanical dispatch is beyond +/- 2 pu, twice the machine's rating (DispatchOutsideCard).";

fn swing_reject(reject: swing::StepReject) -> String {
    match reject {
        swing::StepReject::LossOfSynchronism => {
            unsealed("the rotor landed at or beyond +/- pi/2 (LossOfSynchronism).")
        }
        swing::StepReject::SpontaneousGeneration => {
            unsealed("the machine created transient energy (SpontaneousGeneration).")
        }
        swing::StepReject::FirstLawBreach => {
            unsealed("the machine destroyed transient energy (FirstLawBreach).")
        }
        swing::StepReject::SpeedDeviationOutsideCard => unsealed(SWING_SPEED_OUTSIDE_CARD),
        swing::StepReject::DispatchOutsideCard => unsealed(SWING_DISPATCH_OUTSIDE_CARD),
        other => refused("swing", other),
    }
}

fn tesseract_reject(reject: tesseract::StepReject) -> String {
    match reject {
        tesseract::StepReject::SpontaneousGeneration => {
            unsealed("the drive mode created mechanical energy (SpontaneousGeneration).")
        }
        tesseract::StepReject::FirstLawBreach => {
            unsealed("the drive mode destroyed mechanical energy (FirstLawBreach).")
        }
        tesseract::StepReject::AmplitudeOutsideCard => {
            let resonator = tesseract::Tesseract::drive_mode();
            unsealed(&format!(
                "the step would land the displacement outside [-{0}, {0}] m or the velocity outside [-{1}, {1}] m/s, the inclusive limits of the tesseract card (AmplitudeOutsideCard).",
                resonator.amplitude_limit_m,
                resonator.speed_limit_m_s()
            ))
        }
        other => refused("tesseract", other),
    }
}

fn hydraulic_reject(reject: hydraulic_fluid::StepReject) -> String {
    match reject {
        hydraulic_fluid::StepReject::Cavitation => {
            unsealed("the column fell below vapor pressure (Cavitation).")
        }
        hydraulic_fluid::StepReject::SpontaneousGeneration => {
            unsealed("the column created mass or elastic energy (SpontaneousGeneration).")
        }
        hydraulic_fluid::StepReject::FirstLawBreach => {
            unsealed("the column destroyed mass or elastic energy (FirstLawBreach).")
        }
        hydraulic_fluid::StepReject::PressureAboveCard => unsealed(
            "the column pressure is above the card's ceiling of 20.64 MPa over the charge, 6.0 mm of compression (PressureAboveCard).",
        ),
        other => refused("hydraulic_fluid", other),
    }
}

fn cable_reject(reject: cable_stretch::StepReject) -> String {
    match reject {
        cable_stretch::StepReject::Slack => unsealed("the braid went slack (Slack)."),
        cable_stretch::StepReject::TensileFailure => {
            unsealed("the braid reached ultimate tension (TensileFailure).")
        }
        cable_stretch::StepReject::SpontaneousGeneration => {
            unsealed("the tendon created mechanical energy (SpontaneousGeneration).")
        }
        cable_stretch::StepReject::FirstLawBreach => {
            unsealed("the tendon destroyed mechanical energy (FirstLawBreach).")
        }
        other => refused("cable_stretch", other),
    }
}

/// The current Nyquist limit of the pack at the door, refused under the
/// name the card lists for it.
fn battery_nyquist(pack: &battery_sag::LiIonPack, current_a: f64, dt_s: f64) -> Result<(), String> {
    enforce_abs_nyquist(
        "current_a",
        current_a,
        dt_s,
        pack.charge_impulse_c(),
        pack.current_nyquist_a(dt_s),
    )
    .map_err(|text| format!("{} (CurrentAboveNyquist).", text.trim_end_matches('.')))
}

fn battery_reject(reject: battery_sag::StepReject) -> String {
    match reject {
        battery_sag::StepReject::UndervoltageCutoff => {
            unsealed("the pack reached the 36.0 V cutoff (UndervoltageCutoff).")
        }
        battery_sag::StepReject::StateOfChargeOutsideCard => {
            unsealed("state of charge left [0, 1] (StateOfChargeOutsideCard).")
        }
        battery_sag::StepReject::SpontaneousGeneration => {
            unsealed("the pack created chemical energy (SpontaneousGeneration).")
        }
        battery_sag::StepReject::FirstLawBreach => {
            unsealed("the pack destroyed chemical energy (FirstLawBreach).")
        }
        battery_sag::StepReject::OvervoltageCutoff => {
            unsealed("the pack is above the 50.4 V charging limit (OvervoltageCutoff).")
        }
        other => refused("battery_sag", other),
    }
}

fn cell_reject(reject: cell_18650pf::StepReject) -> String {
    match reject {
        cell_18650pf::StepReject::UndervoltageCutoff => {
            unsealed("the cell reached the 2.5 V cutoff (UndervoltageCutoff).")
        }
        cell_18650pf::StepReject::StateOfChargeOutsideCard => {
            unsealed("state of charge left [0.03, 1] (StateOfChargeOutsideCard).")
        }
        cell_18650pf::StepReject::CurrentNotCompared => cell_current_not_compared(),
        cell_18650pf::StepReject::CurrentChangedDuringDischarge => unsealed(
            "the current is more than 0.83 mA from the held current of this run (CurrentChangedDuringDischarge).",
        ),
        cell_18650pf::StepReject::StartNotAtFullCharge => cell_start_not_at_full_charge(),
        cell_18650pf::StepReject::SpontaneousGeneration => {
            unsealed("the cell created chemical energy (SpontaneousGeneration).")
        }
        cell_18650pf::StepReject::FirstLawBreach => {
            unsealed("the cell destroyed chemical energy (FirstLawBreach).")
        }
        other => refused("cell_18650pf", other),
    }
}

/// The sentence for a chamber setting that names none of the six blocks.
fn cell_2170_block_not_recognized(block_chamber_c: f64) -> String {
    unsealed(&format!(
        "'block_chamber_c' {} degC is not one of the six chamber settings 40, 25, 10, 0, -10 and -20 degC of the cell_2170_m80 card (BlockNotRecognized).",
        message_number(block_chamber_c)
    ))
}

/// Whether a start lies on the tables of its block, or past an end of them
/// by no more than the rounding allowance of a start.
fn cell_2170_start_on_tables(charge_removed_ah: f64, block: usize) -> bool {
    let cell = cell_2170();
    let allowance_ah = cell.charge_end_allowance_ah(0.0);
    charge_removed_ah >= cell.charge_top_ah[block] - allowance_ah
        && charge_removed_ah <= cell.charge_deep_ah[block] + allowance_ah
}

/// The sentence for a start outside the tables of its own block.
fn cell_2170_charge_outside_block(charge_removed_ah: f64, block: usize) -> String {
    let cell = cell_2170();
    unsealed(&format!(
        "'charge_removed_ah' {} Ah is outside [{}, {}] Ah, the tables of the {} degC block of the cell_2170_m80 card, by more than the rounding allowance of a start, 2^-49 Ah (ChargeOutsideTables).",
        message_number(charge_removed_ah),
        message_number(cell.charge_top_ah[block]),
        message_number(cell.charge_deep_ah[block]),
        message_number(cell.block_chamber_c[block]),
    ))
}

/// The sentence for a chamber setting that names a block the comparison
/// with measurement showed wrong.
fn cell_2170_block_shown_wrong(block_chamber_c: f64) -> String {
    unsealed(&format!(
        "'block_chamber_c' {} degC names a block of the cell_2170_m80 card that the comparison with measurement showed wrong: no held-back test of the -10 or the -20 degC block met its marks (BlockShownWrong).",
        message_number(block_chamber_c)
    ))
}

/// The sentence for a case temperature outside the band at which the run's
/// block was compared with measurement.
fn cell_2170_case_outside_block(case_temperature_c: f64, block: usize) -> String {
    let cell = cell_2170();
    unsealed(&format!(
        "'case_temperature_c' {} degC is outside [{}, {}] degC, the case temperatures at which the {} degC block of the cell_2170_m80 card was compared with measurement (CaseTemperatureOutsideBlock).",
        message_number(case_temperature_c),
        message_number(cell.block_case_min_c[block]),
        message_number(cell.block_case_max_c[block]),
        message_number(cell.block_chamber_c[block]),
    ))
}

/// The sentence for a current outside the currents at which the run's block
/// was compared with measurement.
fn cell_2170_current_outside_block(current_a: f64, block: usize) -> String {
    let cell = cell_2170();
    unsealed(&format!(
        "'current_a' {} A is outside [{}, {}] A, the currents at which the {} degC block of the cell_2170_m80 card was compared with measurement (CurrentOutsideBlock).",
        message_number(current_a),
        message_number(-cell.block_current_discharge_max_a[block]),
        message_number(cell.block_current_charge_max_a[block]),
        message_number(cell.block_chamber_c[block]),
    ))
}

const CELL_2170_CURRENT_OUTSIDE_CARD: &str =
    "the current is above 9.737 A charging or above 11.375 A discharging, the largest currents compared with measurement (CurrentOutsideCard).";
const CELL_2170_CASE_OUTSIDE_CARD: &str =
    "case temperature is outside [0.808, 45.535] degC (CaseTemperatureOutsideCard).";

fn cell_2170_reject(reject: cell_2170_m80::StepReject) -> String {
    match reject {
        cell_2170_m80::StepReject::LowerVoltageLimit => unsealed(
            "the cell's voltage would fall below the 2.8 V lower limit (LowerVoltageLimit).",
        ),
        cell_2170_m80::StepReject::UpperVoltageLimit => unsealed(
            "the cell's voltage would rise above the 4.242 V upper limit (UpperVoltageLimit).",
        ),
        cell_2170_m80::StepReject::ChargeOutsideTables => unsealed(
            "the charge removed would leave the tables of the block (ChargeOutsideTables).",
        ),
        cell_2170_m80::StepReject::CaseTemperatureOutsideCard => {
            unsealed(CELL_2170_CASE_OUTSIDE_CARD)
        }
        cell_2170_m80::StepReject::CurrentOutsideCard => unsealed(CELL_2170_CURRENT_OUTSIDE_CARD),
        cell_2170_m80::StepReject::StepNotOneSecond => {
            unsealed("the step is not exactly one second (StepNotOneSecond).")
        }
        cell_2170_m80::StepReject::BlockNotRecognized => unsealed(
            "the block is not one of the six chamber settings 40, 25, 10, 0, -10 and -20 degC (BlockNotRecognized).",
        ),
        cell_2170_m80::StepReject::BlockShownWrong => unsealed(
            "the block is the -10 or the -20 degC block, which the comparison with measurement showed wrong (BlockShownWrong).",
        ),
        cell_2170_m80::StepReject::CaseTemperatureOutsideBlock => unsealed(
            "case temperature is outside the band at which the block was compared with measurement (CaseTemperatureOutsideBlock).",
        ),
        cell_2170_m80::StepReject::CurrentOutsideBlock => unsealed(
            "the current is outside the currents at which the block was compared with measurement (CurrentOutsideBlock).",
        ),
        cell_2170_m80::StepReject::SpontaneousGeneration => {
            unsealed("the cell created energy (SpontaneousGeneration).")
        }
        cell_2170_m80::StepReject::FirstLawBreach => {
            unsealed("the cell destroyed energy (FirstLawBreach).")
        }
        other => refused("cell_2170_m80", other),
    }
}

/// The cells the physical 2170 card holds, in words: `80 (cell m80), 1000
/// (cell m1000) and 448 (cell m448-N)`. Read from the card's own rows.
pub(crate) fn cell_2170_model3_cells_in_words() -> String {
    let names: Vec<String> = cell_2170_model3::CELLS
        .iter()
        .map(|row| format!("{} (cell {})", message_number(row.cell_number), row.name))
        .collect();
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => names.join(""),
    }
}

/// The sentence for a cell number that names none of the cells the card holds.
fn cell_2170_model3_cell_not_recognized(cell_number: f64) -> String {
    unsealed(&format!(
        "'cell_number' {} is not one of the cells of the cell_2170_model3 card, {} (CellNotRecognized).",
        message_number(cell_number),
        cell_2170_model3_cells_in_words()
    ))
}

const CELL_2170_MODEL3_CURRENT_OUTSIDE_CARD: &str =
    "the current is above 13.524 A discharging or above 13.984 A charging (CurrentOutsideCard).";
const CELL_2170_MODEL3_CASE_OUTSIDE_CARD: &str =
    "case temperature is outside [-12.9, 45.9] degC (CaseTemperatureOutsideCard).";

/// The sentence for a capacity that no block of tests measured at the case
/// temperature of a step or a segment.
fn cell_2170_model3_capacity_not_compared(capacity_ah: f64, case_temperature_c: f64) -> String {
    unsealed(&format!(
        "capacity_ah {} Ah was not compared at a case temperature of {} degC: no block of tests of the cell_2170_model3 card holds both (CapacityOutsideCardAtTemperature).",
        message_number(capacity_ah),
        message_number(case_temperature_c)
    ))
}

/// The sentence for a span of case temperature past the card's, found before the first step.
fn cell_2170_model3_span_above_card(least_c: f64, most_c: f64, what: &str) -> String {
    unsealed(&format!(
        "{what} {} to {} degC of case temperature, which is not a span of at most 19.2 K from the least to the most (CaseTemperatureSpanAboveCard).",
        message_number(least_c),
        message_number(most_c)
    ))
}

fn cell_2170_model3_reject(reject: cell_2170_model3::StepReject) -> String {
    use cell_2170_model3::StepReject;
    match reject {
        StepReject::UndervoltageLimit => unsealed(
            "the cell's voltage would fall below the 2.8 V lower limit (UndervoltageLimit).",
        ),
        StepReject::OvervoltageLimit => unsealed(
            "the cell's voltage would rise above the 4.242 V upper limit (OvervoltageLimit).",
        ),
        StepReject::StateOfChargeOutsideCard => unsealed(
            "the state of charge would leave [0, 1.02] by more than the rounding allowance of its running sum (StateOfChargeOutsideCard).",
        ),
        StepReject::CapacityOutsideCardAtTemperature => unsealed(
            "the capacity was not compared at this case temperature: no block of tests holds both (CapacityOutsideCardAtTemperature).",
        ),
        StepReject::CaseTemperatureChangeAboveCard => unsealed(
            "the case temperature would differ from the last step's by more than 5.4 K (CaseTemperatureChangeAboveCard).",
        ),
        StepReject::CaseTemperatureSpanAboveCard => unsealed(
            "the case temperature would span more than 19.2 K since the rested start, counting the least and most case temperature the start carried (CaseTemperatureSpanAboveCard).",
        ),
        StepReject::SurfaceOutsideCurve => unsealed(
            "the state of charge at the particle surface would leave [-0.05, 1.05], the ends of the measured curves (SurfaceOutsideCurve).",
        ),
        StepReject::SustainedCurrentOutsideCard => unsealed(
            "the running mean of the current, with its 60 s time constant, would pass 8.722 A discharging or 2.993 A charging (SustainedCurrentOutsideCard).",
        ),
        StepReject::CaseTemperatureOutsideCard => unsealed(CELL_2170_MODEL3_CASE_OUTSIDE_CARD),
        StepReject::CurrentOutsideCard => unsealed(CELL_2170_MODEL3_CURRENT_OUTSIDE_CARD),
        StepReject::CapacityOutsideCard => {
            unsealed("the capacity is outside [4.16, 4.7] Ah (CapacityOutsideCard).")
        }
        StepReject::CellNotRecognized => unsealed(&format!(
            "the cell number is not one of {} (CellNotRecognized).",
            cell_2170_model3_cells_in_words()
        )),
        StepReject::SpontaneousGeneration => {
            unsealed("the cell created energy (SpontaneousGeneration).")
        }
        StepReject::FirstLawBreach => unsealed("the cell destroyed energy (FirstLawBreach)."),
        other => refused("cell_2170_model3", other),
    }
}

fn crater_reject(reject: crater_ryugu_sci::StepReject) -> String {
    match reject {
        crater_ryugu_sci::StepReject::RadiusBelowCard => {
            unsealed("the crater's radius is under 1.75 m (RadiusBelowCard).")
        }
        crater_ryugu_sci::StepReject::RadiusAboveFinal => unsealed(
            "the crater's radius is above 8.571304329477922 m, where this crater stops (RadiusAboveFinal).",
        ),
        crater_ryugu_sci::StepReject::WallMovingInward => {
            unsealed("the crater's wall is moving inward (WallMovingInward).")
        }
        crater_ryugu_sci::StepReject::SpontaneousGeneration => {
            unsealed("the crater created energy (SpontaneousGeneration).")
        }
        crater_ryugu_sci::StepReject::FirstLawBreach => {
            unsealed("the crater destroyed energy (FirstLawBreach).")
        }
        other => refused("crater_ryugu_sci", other),
    }
}

fn generator_reject(reject: generator_mmrtg_f1::StepReject) -> String {
    match reject {
        generator_mmrtg_f1::StepReject::MissionTimeOutsideCard => unsealed(
            "mission time is before landing or after mission day 4,087 (MissionTimeOutsideCard).",
        ),
        generator_mmrtg_f1::StepReject::SpontaneousGeneration => {
            unsealed("the generator created energy (SpontaneousGeneration).")
        }
        generator_mmrtg_f1::StepReject::FirstLawBreach => {
            unsealed("the generator destroyed energy (FirstLawBreach).")
        }
        generator_mmrtg_f1::StepReject::ConverterAccountBreach => unsealed(
            "the fitted model's account of the fading part of the converter broke (ConverterAccountBreach).",
        ),
        other => refused("generator_mmrtg_f1", other),
    }
}

/// The sentence for a reactivity outside the card, from the card's own bounds.
fn reactor_reactivity_outside_card() -> String {
    unsealed(&format!(
        "the reactivity inserted is below {} dollar or above {} dollars (ReactivityOutsideCard).",
        reactor_spert3::REACTIVITY_MIN_DOLLAR,
        reactor_spert3::REACTIVITY_MAX_DOLLAR
    ))
}

/// The sentence for a coolant temperature outside the card, from the card's own bounds.
fn reactor_coolant_outside_card() -> String {
    unsealed(&format!(
        "the primary coolant temperature is outside [{}, {}] K, 56 to 92 degF (CoolantTemperatureOutsideCard).",
        reactor_spert3::COOLANT_TEMPERATURE_MIN_K,
        reactor_spert3::COOLANT_TEMPERATURE_MAX_K
    ))
}

/// The sentence for a stress outside the creep card, from the card's own bounds.
fn creep_stress_outside_card() -> String {
    unsealed(&format!(
        "the initial stress is below {} MPa or above {} MPa (StressOutsideCard).",
        creep_cmsx6::STRESS_MIN_MPA,
        creep_cmsx6::STRESS_MAX_MPA
    ))
}

/// The sentence for a creep extension below the creep card.
fn creep_extension_below_card() -> String {
    unsealed(&format!(
        "the creep extension is below {} percent, where the card's law is slow to start against every measured curve (CreepExtensionBelowCard).",
        creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT
    ))
}

/// The limits of creep extension by stress, as every sentence about them says it.
fn creep_extension_limits() -> String {
    format!(
        "{} percent below {} MPa, {} percent from {} MPa",
        creep_cmsx6::CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT,
        creep_cmsx6::FULL_REACH_STRESS_MIN_MPA,
        creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT,
        creep_cmsx6::FULL_REACH_STRESS_MIN_MPA
    )
}

/// The sentence for a starting creep extension above the limit for its stress.
fn creep_extension_outside_card() -> String {
    unsealed(&format!(
        "the creep extension is above the limit for the held stress, {} (CreepExtensionOutsideCard).",
        creep_extension_limits()
    ))
}

/// The sentence for a segment whose stress is not the stress the run started at.
fn creep_stress_not_held() -> String {
    unsealed(
        "the stress of the step is not the stress the test piece was started at: the card holds one stress through a run (StressNotHeld).",
    )
}

fn creep_reject(reject: creep_cmsx6::StepReject) -> String {
    match reject {
        creep_cmsx6::StepReject::StressOutsideCard => creep_stress_outside_card(),
        creep_cmsx6::StepReject::CreepExtensionOutsideCard => creep_extension_outside_card(),
        creep_cmsx6::StepReject::CreepExtensionBelowCard => creep_extension_below_card(),
        creep_cmsx6::StepReject::StressNotHeld => creep_stress_not_held(),
        creep_cmsx6::StepReject::CreepExtensionBeyondCard => unsealed(&format!(
            "the step would pass the limit of creep extension for the held stress, {} (CreepExtensionBeyondCard).",
            creep_extension_limits()
        )),
        creep_cmsx6::StepReject::StepTooSmallToResolve => unsealed(
            "the step did not move the creep extension in 64-bit arithmetic (StepTooSmallToResolve).",
        ),
        creep_cmsx6::StepReject::SpontaneousGeneration => unsealed(
            "the load did more work than the creep law dissipates in the time (SpontaneousGeneration).",
        ),
        creep_cmsx6::StepReject::FirstLawBreach => unsealed(
            "the load did less work than the creep law dissipates in the time (FirstLawBreach).",
        ),
        other => refused("creep_cmsx6", other),
    }
}

/// The sentence for a density above the Greenwald density of the plasma current.
fn tokamak_density_above_greenwald(density_per_m3: f64, current_a: f64, greenwald_per_m3: f64) -> String {
    unsealed(&format!(
        "the line-averaged density {} per cubic metre is above {} per cubic metre, the Greenwald density of a plasma current of {} A (DensityAboveGreenwald).",
        message_number(density_per_m3),
        message_number(greenwald_per_m3),
        message_number(current_a),
    ))
}

/// The sentence for a current and a field whose edge safety factor is under the limit.
fn tokamak_safety_factor_below_limit(safety_factor: f64) -> String {
    unsealed(&format!(
        "the edge safety factor of the plasma current and the toroidal field is {}, below {} (SafetyFactorBelowLimit).",
        message_number(safety_factor),
        tokamak_jet_dte1::SAFETY_FACTOR_MIN,
    ))
}

const TOKAMAK_TEMPERATURE_BELOW_FIT: &str = "the plasma temperature is below 0.2 keV, the floor of the reactivity fit (TemperatureBelowReactivityFit).";
const TOKAMAK_TEMPERATURE_ABOVE_FIT: &str = "the temperature at the centre of the profile is above 100 keV, the ceiling of the reactivity fit (TemperatureAboveReactivityFit).";
const TOKAMAK_BETA_ABOVE_TROYON: &str = "the normalised beta of the thermal and the fast-ion energy is above the Troyon limit of 2.8 (BetaAboveTroyon).";

fn tokamak_reject(
    reject: tokamak_jet_dte1::StepReject,
    tokamak: &tokamak_jet_dte1::TokamakJetDte1,
    load: tokamak_jet_dte1::TokamakLoad,
    thermal_energy_j: f64,
) -> String {
    let confinement_time_s = tokamak.confinement_time_s(thermal_energy_j, load);
    match reject {
        tokamak_jet_dte1::StepReject::DensityAboveGreenwald => tokamak_density_above_greenwald(
            load.line_density_per_m3,
            load.plasma_current_a,
            tokamak.greenwald_density_per_m3(load.plasma_current_a),
        ),
        tokamak_jet_dte1::StepReject::SafetyFactorBelowLimit => tokamak_safety_factor_below_limit(
            tokamak.safety_factor(load.plasma_current_a, load.toroidal_field_t),
        ),
        tokamak_jet_dte1::StepReject::TemperatureBelowReactivityFit => {
            unsealed(TOKAMAK_TEMPERATURE_BELOW_FIT)
        }
        tokamak_jet_dte1::StepReject::TemperatureAboveReactivityFit => {
            unsealed(TOKAMAK_TEMPERATURE_ABOVE_FIT)
        }
        tokamak_jet_dte1::StepReject::BetaAboveTroyon => unsealed(TOKAMAK_BETA_ABOVE_TROYON),
        tokamak_jet_dte1::StepReject::EnergyOutsideCard => unsealed(
            "the thermal energy is zero or negative, or the fast-ion energy is negative (EnergyOutsideCard).",
        ),
        tokamak_jet_dte1::StepReject::TimeBeyondCard => {
            unsealed("the step would end after 5.475 s from the start of the heated window, the longest compared window (TimeBeyondCard).")
        }
        tokamak_jet_dte1::StepReject::ConfinementTimeOutsideComparison => unsealed(&format!(
            "the confinement time of the thermal energy under this load is {} s, outside 0.150 to 5.72 s, the span of the nine compared runs of the card's regime (ConfinementTimeOutsideComparison).",
            message_number(confinement_time_s),
        )),
        tokamak_jet_dte1::StepReject::TemperatureOutsideComparison => unsealed(&format!(
            "the plasma temperature under this load is {} keV, outside 0.777 to 4.30 keV, the span of the nine compared runs of the card's regime (TemperatureOutsideComparison).",
            message_number(tokamak.temperature_kev(thermal_energy_j, load.line_density_per_m3)),
        )),
        tokamak_jet_dte1::StepReject::SafetyFactorOutsideComparison => unsealed(&format!(
            "the edge safety factor of the plasma current and the toroidal field is {}, outside 3.66 to 4.14, the span of the nine compared pulses of the card's regime (SafetyFactorOutsideComparison).",
            message_number(tokamak.safety_factor(load.plasma_current_a, load.toroidal_field_t)),
        )),
        tokamak_jet_dte1::StepReject::SpontaneousGeneration => {
            unsealed("the plasma created energy (SpontaneousGeneration).")
        }
        tokamak_jet_dte1::StepReject::FirstLawBreach => {
            unsealed("the plasma destroyed energy (FirstLawBreach).")
        }
        other => refused("tokamak_jet_dte1", other),
    }
}

/// The sentence for a segment whose reactivity is not the first segment's.
fn reactor_reactivity_changed(reactivity_inserted_dollar: f64, first: f64) -> String {
    unsealed(&format!(
        "'reactivity_inserted_dollar' {} dollar is not the {} dollar of the first segment: a run of the reactor_spert3 card is one burst with one reactivity, stepped in at time zero and then held (ReactivityChangedDuringBurst).",
        message_number(reactivity_inserted_dollar),
        message_number(first),
    ))
}

fn reactor_reject(reject: reactor_spert3::StepReject) -> String {
    match reject {
        reactor_spert3::StepReject::ReactivityChangedDuringBurst => unsealed(
            "the reactivity of the load is not the one of the first step of this burst (ReactivityChangedDuringBurst).",
        ),
        reactor_spert3::StepReject::ReactivityOutsideCard => reactor_reactivity_outside_card(),
        reactor_spert3::StepReject::CoolantTemperatureOutsideCard => {
            reactor_coolant_outside_card()
        }
        reactor_spert3::StepReject::PastFirstMaximum => unsealed(
            "the reactor is past the first maximum of power; nothing after it is on the card (PastFirstMaximum).",
        ),
        reactor_spert3::StepReject::TimeBeyondCard => {
            unsealed("the step would end after 20 s (TimeBeyondCard).")
        }
        reactor_spert3::StepReject::EnergyBeyondCard => {
            unsealed("the step released more than 10 MW s (EnergyBeyondCard).")
        }
        reactor_spert3::StepReject::PowerBeyondCard => {
            unsealed("the step reached more than 1000 MW (PowerBeyondCard).")
        }
        reactor_spert3::StepReject::SpontaneousGeneration => {
            unsealed("the reactor created neutron population (SpontaneousGeneration).")
        }
        reactor_spert3::StepReject::FirstLawBreach => {
            unsealed("the reactor destroyed neutron population (FirstLawBreach).")
        }
        other => refused("reactor_spert3", other),
    }
}

/// The sentence for an inlet temperature in none of the three bands of the
/// fuel-temperature reactor, from the card's own bounds.
fn reactor_fuel_inlet_outside_card(inlet_temperature_k: f64) -> String {
    let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
    let bands = (0..3)
        .map(|band| {
            format!(
                "[{}, {}] K ({} degF)",
                message_number(reactor.band_inlet_min_k[band]),
                message_number(reactor.band_inlet_max_k[band]),
                descriptor::REACTOR_FUEL_BAND_DEGF[band],
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    unsealed(&format!(
        "'inlet_temperature_k' {} K is in none of the three tested bands of the reactor_spert3_fuel card: {bands} (InletTemperatureOutsideCard).",
        message_number(inlet_temperature_k),
    ))
}

/// The rod-worth tables the JSI TRIGA card holds, in words: "215, 217 and 231".
pub(crate) fn reactor_triga_tables_in_words() -> String {
    let names: Vec<String> = reactor_triga_jsi::ROD_TABLE_CORE
        .iter()
        .map(|core| message_number(*core))
        .collect();
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => names.join(""),
    }
}

/// The sentence for a core number whose rod-worth table the card does not hold.
fn reactor_triga_table_outside_card(rod_table_core: f64) -> String {
    unsealed(&format!(
        "'rod_table_core' {} is not one of the cores whose rod-worth table the reactor_triga_jsi card holds, {} (RodTableOutsideCard).",
        message_number(rod_table_core),
        reactor_triga_tables_in_words()
    ))
}

/// The starts compared on each rod table, in words: "24.00 to 27.94 degC on core 215, ...".
pub(crate) fn reactor_triga_start_spans_in_words() -> String {
    reactor_triga_jsi::ROD_TABLE_CORE
        .iter()
        .zip(reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE.iter())
        .map(|(core, (low, high))| {
            format!(
                "{:.2} to {:.2} K ({:.2} to {:.2} degC) on core {}",
                low,
                high,
                low - 273.15,
                high - 273.15,
                message_number(*core)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The sentence for a start inside the published span but outside the starts compared on its
/// own rod table.
fn reactor_triga_start_outside_table(start_temperature_k: f64, rod_table_core: f64) -> String {
    unsealed(&format!(
        "'start_temperature_k' {} K was never compared on the rod table of core {}: the reactor_triga_jsi card accepts only the starts compared on each table at 2.0 printed dollars and above, {} (StartTemperatureOutsideCard).",
        message_number(start_temperature_k),
        message_number(rod_table_core),
        reactor_triga_start_spans_in_words()
    ))
}

/// The sentence for a segment whose reactivity is not the first segment's.
fn reactor_triga_reactivity_changed(reactivity_inserted_dollar: f64, first: f64) -> String {
    unsealed(&format!(
        "'reactivity_inserted_dollar' {} dollar is not the {} dollar of the first segment: a run of the reactor_triga_jsi card is one pulse with one inserted reactivity, and the model defines no change of it but the scram (ReactivityChangedDuringRun).",
        message_number(reactivity_inserted_dollar),
        message_number(first),
    ))
}

fn reactor_triga_reject(reject: reactor_triga_jsi::StepReject) -> String {
    match reject {
        reactor_triga_jsi::StepReject::StartTemperatureOutsideCard => unsealed(&format!(
            "the starting fuel temperature is outside the starts compared on its rod table, {} (StartTemperatureOutsideCard).",
            reactor_triga_start_spans_in_words()
        )),
        reactor_triga_jsi::StepReject::RodTableOutsideCard => unsealed(&format!(
            "the rod-worth table is not that of core {} (RodTableOutsideCard).",
            reactor_triga_tables_in_words().replace(" and ", " or ")
        )),
        reactor_triga_jsi::StepReject::ReactivityOutsideCard => unsealed(&format!(
            "the reactivity inserted is below {} or above {} printed dollars (ReactivityOutsideCard).",
            reactor_triga_jsi::REACTIVITY_MIN_DOLLAR,
            reactor_triga_jsi::REACTIVITY_MAX_DOLLAR
        )),
        reactor_triga_jsi::StepReject::ScramValueOutsideCard => unsealed(
            "rods_dropped is neither 0 (rods held) nor 1 (rods dropped) (ScramValueOutsideCard).",
        ),
        reactor_triga_jsi::StepReject::ReactivityChangedDuringRun => unsealed(
            "the reactivity inserted differs from the one this run started with; the model defines no change of it but the scram (ReactivityChangedDuringRun).",
        ),
        reactor_triga_jsi::StepReject::ScramBeforeRodStopped => unsealed(
            "the rods were dropped while the transient rod was still moving (ScramBeforeRodStopped).",
        ),
        reactor_triga_jsi::StepReject::ScramBeforeCompared => unsealed(&format!(
            "the rods were dropped earlier than {} s after the peak power, earlier than any scram compared with a measurement (ScramBeforeCompared).",
            reactor_triga_jsi::SCRAM_AFTER_PEAK_MIN_S
        )),
        reactor_triga_jsi::StepReject::RodsHeldBeyondCompared => unsealed(&format!(
            "the step would end with the rods still held more than {} s after the peak power, beyond what was compared with a measurement (RodsHeldBeyondCompared).",
            reactor_triga_jsi::SCRAM_AFTER_PEAK_MAX_S
        )),
        reactor_triga_jsi::StepReject::RodsRaisedAfterScram => {
            unsealed("the rods were raised after the scram (RodsRaisedAfterScram).")
        }
        reactor_triga_jsi::StepReject::TimeBeyondCard => unsealed(&format!(
            "the step would end after {} s (TimeBeyondCard).",
            reactor_triga_jsi::TIME_MAX_S
        )),
        reactor_triga_jsi::StepReject::EnergyBeyondCard => unsealed(&format!(
            "the step released more than {} MW s (EnergyBeyondCard).",
            reactor_triga_jsi::ENERGY_MAX_MWS
        )),
        reactor_triga_jsi::StepReject::PowerBeyondCard => unsealed(&format!(
            "the step reached more than {} MW (PowerBeyondCard).",
            reactor_triga_jsi::POWER_MAX_MW
        )),
        reactor_triga_jsi::StepReject::FuelTemperatureBeyondCard => unsealed(&format!(
            "the step took the core-average fuel temperature above {} K, beyond what was compared with a measurement (FuelTemperatureBeyondCard).",
            reactor_triga_jsi::FUEL_TEMPERATURE_MAX_K
        )),
        reactor_triga_jsi::StepReject::SpontaneousGeneration => {
            unsealed("the reactor created neutron population (SpontaneousGeneration).")
        }
        reactor_triga_jsi::StepReject::FirstLawBreach => {
            unsealed("the reactor destroyed neutron population (FirstLawBreach).")
        }
        other => refused("reactor_triga_jsi", other),
    }
}

/// The sentence for a reactivity outside the span of the run's own band.
fn reactor_fuel_reactivity_outside_band(reactivity_inserted_dollar: f64, band: usize) -> String {
    let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
    unsealed(&format!(
        "'reactivity_inserted_dollar' {} dollar is outside [{}, {}] dollars, the span tested in the {} degF band of the reactor_spert3_fuel card (ReactivityOutsideCard).",
        message_number(reactivity_inserted_dollar),
        message_number(reactor.band_reactivity_min_dollar[band]),
        message_number(reactor.band_reactivity_max_dollar[band]),
        descriptor::REACTOR_FUEL_BAND_DEGF[band],
    ))
}

/// The sentence for a coolant flow outside the span of the run's own band.
fn reactor_fuel_flow_outside_band(coolant_flow_m_per_s: f64, band: usize) -> String {
    let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
    unsealed(&format!(
        "'coolant_flow_m_per_s' {} m/s is outside [{}, {}] m/s, the span tested in the {} degF band of the reactor_spert3_fuel card (FlowOutsideCard).",
        message_number(coolant_flow_m_per_s),
        message_number(reactor.band_flow_min_m_per_s[band]),
        message_number(reactor.band_flow_max_m_per_s[band]),
        descriptor::REACTOR_FUEL_BAND_DEGF[band],
    ))
}

/// The sentence for a segment whose reactivity is not the first segment's.
fn reactor_fuel_reactivity_changed(reactivity_inserted_dollar: f64, first: f64) -> String {
    unsealed(&format!(
        "'reactivity_inserted_dollar' {} dollar is not the {} dollar of the first segment: a run of the reactor_spert3_fuel card is one burst with one reactivity, ramped in and then held (ReactivityChangedDuringBurst).",
        message_number(reactivity_inserted_dollar),
        message_number(first),
    ))
}

/// The sentence for a segment whose coolant flow is not the first segment's.
fn reactor_fuel_flow_changed(coolant_flow_m_per_s: f64, first: f64) -> String {
    unsealed(&format!(
        "'coolant_flow_m_per_s' {} m/s is not the {} m/s of the first segment: a run of the reactor_spert3_fuel card is one burst with one flow rate, and no tested burst changed its flow (FlowChangedDuringBurst).",
        message_number(coolant_flow_m_per_s),
        message_number(first),
    ))
}

/// The sentence for a coolant flow the C-core card did not compare at this pressure.
fn reactor_ccore_flow_outside_card(coolant_flow_m_per_s: f64, system_pressure_psig: f64) -> String {
    unsealed(&format!(
        "'coolant_flow_m_per_s' {} m/s at {} psig was not compared on the reactor_spert3_ccore card: it takes no flow at 0 to {} psig, or [{}, {}] m/s (200 to 20,000 gpm) at {} psig only (FlowOutsideCard).",
        message_number(coolant_flow_m_per_s),
        message_number(system_pressure_psig),
        message_number(reactor_spert3_ccore::PRESSURE_MAX_PSIG),
        message_number(reactor_spert3_ccore::FLOW_MIN_M_PER_S),
        message_number(reactor_spert3_ccore::FLOW_MAX_M_PER_S),
        message_number(reactor_spert3_ccore::FLOW_PRESSURE_MIN_PSIG),
    ))
}

/// The sentence for a reactivity outside the span compared with the run's flow.
fn reactor_ccore_reactivity_outside_card(reactivity_inserted_dollar: f64, coolant_flow_m_per_s: f64) -> String {
    let (least, with) = if coolant_flow_m_per_s == 0.0 {
        (reactor_spert3_ccore::REACTIVITY_MIN_NO_FLOW_DOLLAR, "with no flow")
    } else {
        (reactor_spert3_ccore::REACTIVITY_MIN_FLOW_DOLLAR, "with flow")
    };
    unsealed(&format!(
        "'reactivity_inserted_dollar' {} dollar is outside [{}, {}] dollars, the span compared {with} on the reactor_spert3_ccore card (ReactivityOutsideCard).",
        message_number(reactivity_inserted_dollar),
        message_number(least),
        message_number(reactor_spert3_ccore::REACTIVITY_MAX_DOLLAR),
    ))
}

/// The sentence for a start outside the compared temperatures or pressures.
fn reactor_ccore_start_outside_card(initial_temperature_k: f64, system_pressure_psig: f64) -> String {
    let reactor = reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients();
    if !reactor.initial_temperature_on_card(initial_temperature_k) {
        unsealed(&format!(
            "'initial_temperature_k' {} K is outside [{}, {}] K, the 21 to 36 degC compared on the reactor_spert3_ccore card (InitialTemperatureOutsideCard).",
            message_number(initial_temperature_k),
            message_number(reactor_spert3_ccore::INITIAL_TEMPERATURE_MIN_K),
            message_number(reactor_spert3_ccore::INITIAL_TEMPERATURE_MAX_K),
        ))
    } else {
        unsealed(&format!(
            "'system_pressure_psig' {} psig is outside [0, {}] psig, the span compared on the reactor_spert3_ccore card (PressureOutsideCard).",
            message_number(system_pressure_psig),
            message_number(reactor_spert3_ccore::PRESSURE_MAX_PSIG),
        ))
    }
}

/// The sentence for a segment whose reactivity is not the first segment's.
fn reactor_ccore_reactivity_changed(reactivity_inserted_dollar: f64, first: f64) -> String {
    unsealed(&format!(
        "'reactivity_inserted_dollar' {} dollar is not the {} dollar of the first segment: a run of the reactor_spert3_ccore card is one step of reactivity (ReactivityChangedDuringRun).",
        message_number(reactivity_inserted_dollar),
        message_number(first),
    ))
}

/// The sentence for a segment whose coolant flow is not the first segment's.
fn reactor_ccore_flow_changed(coolant_flow_m_per_s: f64, first: f64) -> String {
    unsealed(&format!(
        "'coolant_flow_m_per_s' {} m/s is not the {} m/s of the first segment: a run of the reactor_spert3_ccore card carries one flow, and no compared test changed its flow (FlowChangedDuringRun).",
        message_number(coolant_flow_m_per_s),
        message_number(first),
    ))
}

fn reactor_ccore_reject(reject: reactor_spert3_ccore::StepReject) -> String {
    match reject {
        reactor_spert3_ccore::StepReject::PastPlateTemperatureMaximum => unsealed(
            "the reactor is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card (PastPlateTemperatureMaximum).",
        ),
        reactor_spert3_ccore::StepReject::TimeBeyondCard => unsealed(&format!(
            "the step would end after {} s (TimeBeyondCard).",
            reactor_spert3_ccore::TIME_MAX_S
        )),
        reactor_spert3_ccore::StepReject::EnergyBeyondCard => unsealed(&format!(
            "the step released more than {} MW s (EnergyBeyondCard).",
            reactor_spert3_ccore::ENERGY_MAX_MWS
        )),
        reactor_spert3_ccore::StepReject::PowerBeyondCard => unsealed(&format!(
            "the step reached more than {} MW (PowerBeyondCard).",
            reactor_spert3_ccore::POWER_MAX_MW
        )),
        reactor_spert3_ccore::StepReject::SpontaneousGeneration => {
            unsealed("the reactor created neutrons or energy beyond what its ledger supplies (SpontaneousGeneration).")
        }
        reactor_spert3_ccore::StepReject::FirstLawBreach => {
            unsealed("the reactor destroyed neutrons or energy its ledger supplies (FirstLawBreach).")
        }
        other => refused("reactor_spert3_ccore", other),
    }
}

fn reactor_fuel_reject(reject: reactor_spert3_fuel::StepReject) -> String {
    match reject {
        reactor_spert3_fuel::StepReject::InletTemperatureOutsideCard => unsealed(
            "the inlet temperature is in none of the three tested bands: 56 to 92, 251 to 259, 498 to 501 degF (InletTemperatureOutsideCard).",
        ),
        reactor_spert3_fuel::StepReject::ReactivityOutsideCard => unsealed(
            "the reactivity inserted is outside the span tested in this band (ReactivityOutsideCard).",
        ),
        reactor_spert3_fuel::StepReject::FlowOutsideCard => unsealed(
            "the coolant flow rate is outside the span tested in this band: none in the cold band, 2.4 to 22 ft/s at 260 degF and 4.8 to 24 ft/s at 500 degF (FlowOutsideCard).",
        ),
        reactor_spert3_fuel::StepReject::PastFirstMaximum => unsealed(
            "the reactor is past the first maximum of power; nothing after it is on the card (PastFirstMaximum).",
        ),
        reactor_spert3_fuel::StepReject::TimeBeyondCard => unsealed(&format!(
            "the step would end after {} s (TimeBeyondCard).",
            reactor_spert3_fuel::TIME_MAX_S
        )),
        reactor_spert3_fuel::StepReject::EnergyBeyondCard => unsealed(&format!(
            "the step released more than {} MW s (EnergyBeyondCard).",
            reactor_spert3_fuel::ENERGY_MAX_MWS
        )),
        reactor_spert3_fuel::StepReject::PowerBeyondCard => unsealed(&format!(
            "the step reached more than {} MW (PowerBeyondCard).",
            reactor_spert3_fuel::POWER_MAX_MW
        )),
        reactor_spert3_fuel::StepReject::SpontaneousGeneration => {
            unsealed("the reactor created neutron population (SpontaneousGeneration).")
        }
        reactor_spert3_fuel::StepReject::FirstLawBreach => {
            unsealed("the reactor destroyed neutron population (FirstLawBreach).")
        }
        other => refused("reactor_spert3_fuel", other),
    }
}

/// The sentence for a loss outside the card, from the card's own bound.
fn grid_gb_loss_outside_card() -> String {
    unsealed(&format!(
        "the power lost is outside {} to {} MW, the losses compared with measurement (LossOutsideCard).",
        grid_gb::LOSS_MIN_MW,
        grid_gb::LOSS_MAX_MW
    ))
}

fn grid_gb_reject(reject: grid_gb::StepReject) -> String {
    match reject {
        grid_gb::StepReject::LossOutsideCard => grid_gb_loss_outside_card(),
        grid_gb::StepReject::LossChangedDuringRun => unsealed(
            "the power lost is not the loss of the first step of this run (LossChangedDuringRun).",
        ),
        grid_gb::StepReject::StartFrequencyOutsideCard => unsealed(
            "the starting frequency is outside 49.85 to 50.15 Hz (StartFrequencyOutsideCard).",
        ),
        grid_gb::StepReject::InertiaOutsideCard => unsealed(
            "the stored kinetic energy at 50 Hz is outside 111,000 to 337,000 MW s (InertiaOutsideCard).",
        ),
        grid_gb::StepReject::DemandOutsideCard => {
            unsealed("the demand is outside 18,163 to 40,829 MW (DemandOutsideCard).")
        }
        grid_gb::StepReject::TimeBeyondCard => {
            unsealed("the step would end later than 60 s after the start (TimeBeyondCard).")
        }
        grid_gb::StepReject::LowFrequencyDemandDisconnection => unsealed(
            "the frequency is at or below 48.8 Hz, where low-frequency demand disconnection begins (LowFrequencyDemandDisconnection).",
        ),
        grid_gb::StepReject::FrequencyBelowCompared => unsealed(
            "the frequency is below 49.55 Hz, the lowest frequency compared with measurement (FrequencyBelowCompared).",
        ),
        grid_gb::StepReject::FrequencyAboveCard => unsealed(
            "the frequency is above 50.5 Hz, the upper statutory limit (FrequencyAboveCard).",
        ),
        grid_gb::StepReject::SpontaneousGeneration => {
            unsealed("the area created kinetic energy (SpontaneousGeneration).")
        }
        grid_gb::StepReject::FirstLawBreach => {
            unsealed("the area destroyed kinetic energy (FirstLawBreach).")
        }
        other => refused("grid_gb", other),
    }
}

const CAPSULE_ALTITUDE_OUTSIDE_CARD: &str =
    "altitude is above 134.7 km or below 30 km (AltitudeOutsideCard).";
const CAPSULE_MACH_OUTSIDE_CARD: &str = "the Mach number is below 1.0, or is above 40 below 68.96 km, where the drag table by Mach number ends (MachOutsideCard).";
const CAPSULE_AIR_SPEED_OUTSIDE_CARD: &str = "above 68.96 km the speed through the air is below 11900 m/s or above 12575 m/s, outside the speeds the entry compared with the flight passed through there (AirSpeedOutsideCard).";
const CAPSULE_PAST_FLIGHT_DEPLOYMENT: &str = "the state is under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, where the capsule flew on a parachute (PastFlightDeployment).";
const CAPSULE_OFF_COMPARED_CORRIDOR: &str = "the state is off the corridor the entry compared with the flight flew from the locked entry state: below 68.96 km, or moving upward above it, it is under 31 km or above 69 km, or its speed through the air or its path angle is outside the corridor at its altitude (OffComparedCorridor).";

fn capsule_reject(reject: capsule_stardust::StepReject) -> String {
    match reject {
        capsule_stardust::StepReject::AltitudeOutsideCard => {
            unsealed(CAPSULE_ALTITUDE_OUTSIDE_CARD)
        }
        capsule_stardust::StepReject::MachOutsideCard => unsealed(CAPSULE_MACH_OUTSIDE_CARD),
        capsule_stardust::StepReject::AirSpeedOutsideCard => {
            unsealed(CAPSULE_AIR_SPEED_OUTSIDE_CARD)
        }
        capsule_stardust::StepReject::PastFlightDeployment => {
            unsealed(CAPSULE_PAST_FLIGHT_DEPLOYMENT)
        }
        capsule_stardust::StepReject::OffComparedCorridor => {
            unsealed(CAPSULE_OFF_COMPARED_CORRIDOR)
        }
        capsule_stardust::StepReject::SpontaneousGeneration => unsealed(
            "the capsule gained energy that drag did not account for (SpontaneousGeneration).",
        ),
        capsule_stardust::StepReject::FirstLawBreach => unsealed(
            "the capsule lost more energy than drag accounts for (FirstLawBreach).",
        ),
        other => refused("capsule_stardust", other),
    }
}

const CAPSULE_INSIGHT_ALTITUDE_OUTSIDE_CARD: &str =
    "altitude is above 128 km or below 4 km (AltitudeOutsideCard).";
const CAPSULE_INSIGHT_MACH_OUTSIDE_CARD: &str = "the Mach number is below 1.0 (MachOutsideCard).";
const CAPSULE_INSIGHT_AIR_SPEED_OUTSIDE_CARD: &str =
    "the speed through the air is above 5850 m/s (AirSpeedOutsideCard).";
const CAPSULE_INSIGHT_LIFT_OUTSIDE_CARD: &str =
    "the lift over drag is above 0.08 or below -0.08 (LiftOutsideCard).";
const CAPSULE_INSIGHT_PAST_COMPARED_FLIGHT: &str = "altitude is below 5700 m or the Mach number is below 1.46, past the lowest state the run compared with the flight reached; the flight record ends at parachute deployment, 6164.6 m above the datum (PastComparedFlight).";
const CAPSULE_INSIGHT_OFF_COMPARED_CORRIDOR: &str = "the state is off the corridor the runs compared with the flight flew from the locked entry state: above 126.5 km, or its speed through the air or its path angle is outside the corridor at its altitude (OffComparedCorridor).";

fn capsule_insight_reject(reject: capsule_insight::StepReject) -> String {
    match reject {
        capsule_insight::StepReject::LiftOutsideCard => {
            unsealed(CAPSULE_INSIGHT_LIFT_OUTSIDE_CARD)
        }
        capsule_insight::StepReject::AltitudeOutsideCard => {
            unsealed(CAPSULE_INSIGHT_ALTITUDE_OUTSIDE_CARD)
        }
        capsule_insight::StepReject::MachOutsideCard => {
            unsealed(CAPSULE_INSIGHT_MACH_OUTSIDE_CARD)
        }
        capsule_insight::StepReject::AirSpeedOutsideCard => {
            unsealed(CAPSULE_INSIGHT_AIR_SPEED_OUTSIDE_CARD)
        }
        capsule_insight::StepReject::PastComparedFlight => {
            unsealed(CAPSULE_INSIGHT_PAST_COMPARED_FLIGHT)
        }
        capsule_insight::StepReject::OffComparedCorridor => {
            unsealed(CAPSULE_INSIGHT_OFF_COMPARED_CORRIDOR)
        }
        capsule_insight::StepReject::SpontaneousGeneration => unsealed(
            "the capsule gained energy that drag did not account for (SpontaneousGeneration).",
        ),
        capsule_insight::StepReject::FirstLawBreach => unsealed(
            "the capsule lost more energy than drag accounts for (FirstLawBreach).",
        ),
        other => refused("capsule_insight", other),
    }
}

const CANTILEVER_DEFLECTION_ABOVE_CARD: &str = "the deflection envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2), is above 0.01579 m, the largest the compared run held (DeflectionAboveCard).";
const CANTILEVER_DEFLECTION_BELOW_CARD: &str = "the deflection envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2), is below 5.8e-5 m, the smallest the card was compared at (DeflectionBelowCard).";

const LOS_COUNT_RATE_OUTSIDE_CARD: &str = "the guide star's count rate is below 22900 or above 2820000 counts per second, the range the constants were fitted on (CountRateOutsideCard).";
const LOS_OFFSET_OUTSIDE_CARD: &str = "the centroid is more than 250 mas from the lock point on x or on y (OffsetOutsideCard).";
const LOS_RECOVERY_NOT_ELAPSED: &str = "the run starts less than 30 s after the last centroid the caller states was refused as a disturbance or was more than 5 mas from the lock point (RecoveryNotElapsed).";
const LOS_POINTING_DISTURBANCE: &str = "a measured centroid is more than 5 mas from the lock point on x or on y: the pointing is disturbed, and there the card is worse than taking the last centroid. A new run may start no earlier than 30 s after this centroid and after any later one more than 5 mas from the lock point (PointingDisturbance).";
const LOS_INNOVATION_BEYOND_FOUR_SIGMA: &str = "a measured centroid is more than four stated standard deviations from where it was expected, on x or on y. A new run may start no earlier than 30 s after this centroid (InnovationBeyondFourSigma).";
const LOS_MEASUREMENT_GAP_OUTSIDE_CARD: &str = "the frame has no measurement and would make more than 10 frames in a row with none, the longest run of them the comparison crossed. A new run may start at the next measured frame (MeasurementGapOutsideCard).";
const LOS_MEASURED_NOT_ZERO_OR_ONE: &str = "Physical Violation: 'measured' is neither 0 nor 1 (MeasuredNotZeroOrOne) on the los_jwst_fine_guide card.";
const LOS_UNMEASURED_CARRIES_NUMBERS: &str = "Physical Violation: a frame with 'measured' 0 carries a centroid or a count rate that is not zero (UnmeasuredFrameCarriesNumbers) on the los_jwst_fine_guide card.";

fn los_reject(reject: los_jwst_fine_guide::StepReject) -> String {
    match reject {
        los_jwst_fine_guide::StepReject::CountRateOutsideCard => {
            unsealed(LOS_COUNT_RATE_OUTSIDE_CARD)
        }
        los_jwst_fine_guide::StepReject::OffsetOutsideCard => unsealed(LOS_OFFSET_OUTSIDE_CARD),
        los_jwst_fine_guide::StepReject::RecoveryNotElapsed => unsealed(LOS_RECOVERY_NOT_ELAPSED),
        los_jwst_fine_guide::StepReject::PointingDisturbance => unsealed(LOS_POINTING_DISTURBANCE),
        los_jwst_fine_guide::StepReject::InnovationBeyondFourSigma => {
            unsealed(LOS_INNOVATION_BEYOND_FOUR_SIGMA)
        }
        los_jwst_fine_guide::StepReject::MeasurementGapOutsideCard => {
            unsealed(LOS_MEASUREMENT_GAP_OUTSIDE_CARD)
        }
        los_jwst_fine_guide::StepReject::MeasuredNotZeroOrOne => {
            LOS_MEASURED_NOT_ZERO_OR_ONE.to_string()
        }
        los_jwst_fine_guide::StepReject::UnmeasuredFrameCarriesNumbers => {
            LOS_UNMEASURED_CARRIES_NUMBERS.to_string()
        }
        los_jwst_fine_guide::StepReject::SpontaneousGeneration => unsealed(
            "the filter was left with more variance than the fading, the drive and the measurement account for (SpontaneousGeneration).",
        ),
        los_jwst_fine_guide::StepReject::FirstLawBreach => unsealed(
            "the filter was left with less variance than the fading, the drive and the measurement account for (FirstLawBreach).",
        ),
        other => refused("los_jwst_fine_guide", other),
    }
}

/// One frame of the table, read from a request and held to what the request
/// itself shows: `measured` is 0 or 1, a frame with no measurement carries
/// zeros, a measured frame has its count rate on the card and its centroid
/// within the card's reach of the lock point.
fn los_load(
    card: &los_jwst_fine_guide::LosJwstFineGuide,
    lock_x_arcsec: f64,
    lock_y_arcsec: f64,
    params: &serde_json::Value,
) -> Result<los_jwst_fine_guide::LosLoad, String> {
    let measured = required_f64(params, "measured")?;
    let centroid_x_arcsec = required_f64(params, "centroid_x_arcsec")?;
    let centroid_y_arcsec = required_f64(params, "centroid_y_arcsec")?;
    let counts_per_s = required_f64(params, "counts_per_s")?;
    if measured == 0.0 {
        if centroid_x_arcsec != 0.0 || centroid_y_arcsec != 0.0 || counts_per_s != 0.0 {
            return Err(LOS_UNMEASURED_CARRIES_NUMBERS.to_string());
        }
    } else if measured == 1.0 {
        if !(counts_per_s >= card.counts_min_per_s && counts_per_s <= card.counts_max_per_s) {
            return Err(unsealed(LOS_COUNT_RATE_OUTSIDE_CARD));
        }
        let offset_x_mas =
            (centroid_x_arcsec - lock_x_arcsec) * los_jwst_fine_guide::MAS_PER_ARCSEC;
        let offset_y_mas =
            (centroid_y_arcsec - lock_y_arcsec) * los_jwst_fine_guide::MAS_PER_ARCSEC;
        if !(offset_x_mas.abs() <= card.offset_max_mas && offset_y_mas.abs() <= card.offset_max_mas)
        {
            return Err(unsealed(LOS_OFFSET_OUTSIDE_CARD));
        }
    } else {
        return Err(LOS_MEASURED_NOT_ZERO_OR_ONE.to_string());
    }
    Ok(los_jwst_fine_guide::LosLoad {
        measured,
        centroid_x_arcsec,
        centroid_y_arcsec,
        counts_per_s,
    })
}

fn cantilever_reject(reject: cantilever_eth::StepReject) -> String {
    match reject {
        cantilever_eth::StepReject::DeflectionAboveCard => {
            unsealed(CANTILEVER_DEFLECTION_ABOVE_CARD)
        }
        cantilever_eth::StepReject::DeflectionBelowCard => {
            unsealed(CANTILEVER_DEFLECTION_BELOW_CARD)
        }
        cantilever_eth::StepReject::SpontaneousGeneration => {
            unsealed("the beam created mechanical energy (SpontaneousGeneration).")
        }
        cantilever_eth::StepReject::FirstLawBreach => {
            unsealed("the beam destroyed mechanical energy (FirstLawBreach).")
        }
        other => refused("cantilever_eth", other),
    }
}

fn link_reject(reject: link_gothenburg_10010::StepReject) -> String {
    match reject {
        link_gothenburg_10010::StepReject::RainRateOutsideCard => {
            unsealed("the rain rate is outside 0 to 84 mm/h (RainRateOutsideCard).")
        }
        link_gothenburg_10010::StepReject::RainRateBelowGaugeCount => unsealed(
            "the rain rate is above zero and below 6 mm/h, one count of the gauge in a minute: the gauge gives no such one-minute rate, and none was compared with the link (RainRateBelowGaugeCount).",
        ),
        link_gothenburg_10010::StepReject::RainNotHeldForGaugeMinute => unsealed(
            "the rain rate changed inside a gauge minute: it is not the rate held since the minute began, and only rain held through whole gauge minutes was compared with the link (RainNotHeldForGaugeMinute).",
        ),
        link_gothenburg_10010::StepReject::GaugeMinuteOutsideCard => unsealed(
            "the seconds already spent in the current gauge minute are below zero or not below 60 s (GaugeMinuteOutsideCard).",
        ),
        link_gothenburg_10010::StepReject::DryPathLossOutsideCard => {
            unsealed("the dry level is outside 50 to 54 dB (DryPathLossOutsideCard).")
        }
        link_gothenburg_10010::StepReject::AntennaWaterOutsideCard => unsealed(
            "the attenuation of the water on the antennas is outside 0 to 2.13 dB (AntennaWaterOutsideCard).",
        ),
        link_gothenburg_10010::StepReject::SpontaneousGeneration => unsealed(
            "the link delivered more power than the dry path carries (SpontaneousGeneration).",
        ),
        link_gothenburg_10010::StepReject::FirstLawBreach => unsealed(
            "the link lost power that neither the rain nor the water on the antennas took (FirstLawBreach).",
        ),
        link_gothenburg_10010::StepReject::AntennaAccountBreach => unsealed(
            "the step broke the fitted model's account of the water on the antennas (AntennaAccountBreach).",
        ),
        other => refused("link_gothenburg_10010", other),
    }
}

const TIRE_SLIP_OUTSIDE_CARD: &str =
    "the slip magnitude is below 0.05 or above 0.20, the slips the constants cover (SlipOutsideCard).";
const TIRE_TRAVEL_BEYOND_TRACK: &str = "the distance along the track is below zero, or the step ends beyond 6 m, the length rolled in a run (TravelBeyondTrack).";
const TIRE_SLIP_NOT_SETTLED: &str = "the slip has been held less than 5 s at the start of the step: a start stated less, or the slip changed; the force in the first 5 s after a slip is set failed the comparison with the measurements (SlipNotSettled).";
const TIRE_HOLD_BEYOND_MEASURED: &str = "the step ends more than 21 s after the slip was set; no scored slip was compared beyond one 21 s stage (HoldBeyondMeasured).";

const FINGERPAD_TWIST_BEYOND_MEASURED: &str = "the twist is below zero, or the step ends beyond 75 degrees, the end of the twist compared with measurement, past which the measured plate slowed to a stop at 80 (TwistBeyondMeasured).";
const FINGERPAD_TWIST_NOT_FROM_ZERO: &str = "the twist at the start is not zero; every measured twist began with the pad untwisted, and the card cannot see the load a pad already twisted was brought there under (TwistNotStartedFromZero).";
const FINGERPAD_LOAD_CHANGED: &str = "the normal force or the twist rate differs from the one this twist began with; no measured twist changed either (LoadChangedDuringTwist).";

fn fingerpad_reject(reject: fingerpad_p02_glass::StepReject) -> String {
    use fingerpad_p02_glass::StepReject;
    match reject {
        StepReject::NormalForceOutsideCard => unsealed(
            "the normal force is below 1.93 N or above 2.07 N: the card holds its mark only in the block of trials at 2 N, within the spread of the robot's force controller (NormalForceOutsideCard).",
        ),
        StepReject::TwistRateOutsideCard => unsealed(
            "the twist rate is below 5 or above 50 degrees a second: the card holds its mark at 2 N only at those rates (TwistRateOutsideCard).",
        ),
        StepReject::LoadChangedDuringTwist => unsealed(FINGERPAD_LOAD_CHANGED),
        StepReject::TwistNotStartedFromZero => unsealed(FINGERPAD_TWIST_NOT_FROM_ZERO),
        StepReject::TwistBeyondMeasured => unsealed(FINGERPAD_TWIST_BEYOND_MEASURED),
        StepReject::SpontaneousGeneration => unsealed(
            "the stored energy plus the friction heat rose above the plate work (SpontaneousGeneration).",
        ),
        StepReject::FirstLawBreach => unsealed(
            "the stored energy plus the friction heat fell below the plate work (FirstLawBreach).",
        ),
        other => refused("fingerpad_p02_glass", other),
    }
}

fn tire_reject(reject: tire_eve_r19_ice::StepReject) -> String {
    use tire_eve_r19_ice::StepReject;
    match reject {
        StepReject::NormalLoadOutsideCard => unsealed(
            "the normal load is below 3 500 N or above 6 500 N, the loads measured (NormalLoadOutsideCard).",
        ),
        StepReject::InflationPressureOutsideCard => unsealed(
            "the inflation pressure is below 150 kPa or above 350 kPa, the pressures measured (InflationPressureOutsideCard).",
        ),
        StepReject::SlipOutsideCard => unsealed(TIRE_SLIP_OUTSIDE_CARD),
        StepReject::CarriageSpeedOutsideCard => unsealed(
            "the carriage speed is below 0.05 m/s or above 0.07 m/s, the creep of the rig (CarriageSpeedOutsideCard).",
        ),
        StepReject::TravelBeyondTrack => unsealed(TIRE_TRAVEL_BEYOND_TRACK),
        StepReject::SlipNotSettled => unsealed(TIRE_SLIP_NOT_SETTLED),
        StepReject::HoldBeyondMeasured => unsealed(TIRE_HOLD_BEYOND_MEASURED),
        StepReject::SpontaneousGeneration => unsealed(
            "the drawbar work plus the slip heat rose above the wheel work (SpontaneousGeneration).",
        ),
        StepReject::FirstLawBreach => unsealed(
            "the drawbar work plus the slip heat fell below the wheel work (FirstLawBreach).",
        ),
        other => refused("tire_eve_r19_ice", other),
    }
}

fn accumulator_reject(reject: accumulator_sbo450::StepReject) -> String {
    use accumulator_sbo450::StepReject;
    match reject {
        StepReject::GasMassOutsideCard => unsealed(
            "the gas mass is neither of the two charges measured: 0.013639 to 0.013667 kg, a precharge of 20 bar, or 0.02744 to 0.027495 kg, a precharge of 40 bar (GasMassOutsideCard).",
        ),
        StepReject::WallTemperatureOutsideCard => unsealed(
            "the wall temperature is below 293.4 K or above 293.7 K: no other wall temperature was compared with measurement (WallTemperatureOutsideCard).",
        ),
        StepReject::PistonSpeedOutsideCard => unsealed(
            "the piston speed is above 0.7 m/s, either way: no faster motion was compared with measurement (PistonSpeedOutsideCard).",
        ),
        StepReject::PistonTravelAboveStep => unsealed(
            "the piston would travel more than 1 mm in the step (PistonTravelAboveStep).",
        ),
        StepReject::GasTemperatureOutsideCard => unsealed(
            "the gas temperature, incoming or landing, is below 285.9 K or above 301.8 K: no other gas temperature was compared with measurement (GasTemperatureOutsideCard).",
        ),
        StepReject::PressureOutsideCard => unsealed(
            "the gas pressure, incoming or landing, is below 37 bar or above 86 bar (PressureOutsideCard).",
        ),
        StepReject::PressureOutsideChargeBands => unsealed(
            "the gas pressure, incoming or landing, is inside 37 to 86 bar and outside the bands compared with measurement for the charge given: 37 to 43 or 55 to 67 bar for the 20 bar charge, 57 to 64 or 75 to 86 bar for the 40 bar charge (PressureOutsideChargeBands).",
        ),
        StepReject::NegativeMovedSum => unsealed(
            "the work moved or the heat moved carried into a start is below zero (NegativeMovedSum).",
        ),
        StepReject::OilSideEmpty => unsealed(
            "the gas volume, incoming or landing, is above the vessel's 0.58879 l: there is no oil left to draw out (OilSideEmpty).",
        ),
        StepReject::HeatShareAboveCard => unsealed(
            "the heat exchanged with the wall over the motion so far, with the sums carried into the start, incoming or landing, is more than 0.065 of the work done either way, with the work counted as 100 J until it is more: the motion is too slow, or the rest too long, for the card's one time constant (HeatShareAboveCard).",
        ),
        StepReject::SpontaneousGeneration => unsealed(
            "the gas gained more energy than the work on it less the heat it gave (SpontaneousGeneration).",
        ),
        StepReject::FirstLawBreach => unsealed(
            "the gas gained less energy than the work on it less the heat it gave (FirstLawBreach).",
        ),
        other => refused("accumulator_sbo450", other),
    }
}

const SATELLITE_RADIUS_OUTSIDE_CARD: &str =
    "distance from the Earth's centre is below 11 997 km or above 12 327 km, the radii the compared runs reached (RadiusOutsideCard).";
const SATELLITE_ORBIT_OUTSIDE_CARD: &str = "the orbit is not closed, or is in neither of the two compared orbits: LAGEOS-1, semi-major axis 12 265 to 12 275 km, eccentricity 0.0034 to 0.0044 and inclination 109.80 to 109.83 degrees, and LAGEOS-2, 12 158 to 12 166 km, 0.0132 to 0.0138 and 52.63 to 52.66 degrees; no other orbit was compared (OrbitOutsideCard).";
const SATELLITE_EPOCH_OUTSIDE_CARD: &str =
    "the time is before 2026-08-23 00:00 or after 2026-09-20 00:00 UTC, the dates of the compared runs (EpochOutsideCard).";
const SATELLITE_HORIZON_OUTSIDE_CARD: &str = "the forecast would end more than seven days, 604800 s, from the state taken from a published orbit, counted as forecast_elapsed_s at the start plus t_s, or the stated forecast_elapsed_s is below zero; the compared runs were seven days long (HorizonOutsideCard).";

const SOLAR_SYSTEM_DISTANCE_OUTSIDE_CARD: &str =
    "a body's distance from its primary is outside that body's band (DistanceOutsideCard).";
const SOLAR_SYSTEM_ORBIT_OUTSIDE_CARD: &str = "the semi-major axis of a body's orbit about its primary is outside that body's band, or the orbit is not closed (OrbitOutsideCard).";
const SOLAR_SYSTEM_FRAME_OUTSIDE_CARD: &str =
    "the centre of mass of the thirteen bodies moves faster than 0.01 m/s (FrameOutsideCard).";
const SOLAR_SYSTEM_BODIES_COINCIDE: &str = "two bodies are at exactly one point, where the attraction between them has no value (BodiesCoincide).";
const SOLAR_SYSTEM_DISTANCE_OUTSIDE_COMPARED_RUN: &str = "a body's distance from its primary is outside the distances the compared run went through up to the last range compared with measurement, 16.2 years after the 1999 state: Mercury 0.30 to 0.47 au, Venus 0.71 to 0.73, the Earth 0.98 to 1.02, the Moon 356 000 to 407 000 km, Mars 1.38 to 1.67, Jupiter 4.94 to 5.46, Saturn 9.03 to 9.97, Uranus 19.88 to 20.10, Neptune 29.96 to 30.14, Ceres 2.54 to 2.99, Pallas 2.13 to 3.42, Vesta 2.14 to 2.58 au (DistanceOutsideComparedRun).";
const SOLAR_SYSTEM_BODIES_TOO_CLOSE: &str = "two bodies other than the Earth and the Moon are closer than 0.26 au, or two of the three asteroids closer than 0.38 au: closer than any pair in the compared run (BodiesTooClose).";
const SOLAR_SYSTEM_ORIGIN_OUTSIDE_CARD: &str = "the centre of mass of the thirteen bodies is more than 120000 m from the origin; the positions are from the barycentre of the Solar System (OriginOutsideCard).";
const SOLAR_SYSTEM_AXES_OUTSIDE_CARD: &str = "the orbit of the Earth-Moon barycentre about the Sun is not inclined 21 to 27 degrees to the z axis, as the ecliptic is to the ICRF equator: the axes are not those of the ICRF (AxesOutsideCard).";

fn solar_system_reject(reject: solar_system::StepReject) -> String {
    match reject {
        solar_system::StepReject::DistanceOutsideCard => {
            unsealed(SOLAR_SYSTEM_DISTANCE_OUTSIDE_CARD)
        }
        solar_system::StepReject::OrbitOutsideCard => unsealed(SOLAR_SYSTEM_ORBIT_OUTSIDE_CARD),
        solar_system::StepReject::FrameOutsideCard => unsealed(SOLAR_SYSTEM_FRAME_OUTSIDE_CARD),
        solar_system::StepReject::BodiesCoincide => unsealed(SOLAR_SYSTEM_BODIES_COINCIDE),
        solar_system::StepReject::DistanceOutsideComparedRun => {
            unsealed(SOLAR_SYSTEM_DISTANCE_OUTSIDE_COMPARED_RUN)
        }
        solar_system::StepReject::BodiesTooClose => unsealed(SOLAR_SYSTEM_BODIES_TOO_CLOSE),
        solar_system::StepReject::OriginOutsideCard => unsealed(SOLAR_SYSTEM_ORIGIN_OUTSIDE_CARD),
        solar_system::StepReject::AxesOutsideCard => unsealed(SOLAR_SYSTEM_AXES_OUTSIDE_CARD),
        solar_system::StepReject::SpontaneousGeneration => unsealed(
            "the carried ledger, or the step, gained energy that the work of relativity and of the Earth's oblateness does not account for (SpontaneousGeneration).",
        ),
        solar_system::StepReject::FirstLawBreach => unsealed(
            "the carried ledger, or the step, lost energy that the work of relativity and of the Earth's oblateness does not account for (FirstLawBreach).",
        ),
        solar_system::StepReject::AngularMomentumBreach => unsealed(
            "the carried ledger, or the step, changed the angular momentum by more than the torque of relativity and of the Earth's oblateness accounts for (AngularMomentumBreach).",
        ),
        other => refused("solar_system", other),
    }
}

fn satellite_reject(reject: satellite_lageos1::StepReject) -> String {
    match reject {
        satellite_lageos1::StepReject::RadiusOutsideCard => {
            unsealed(SATELLITE_RADIUS_OUTSIDE_CARD)
        }
        satellite_lageos1::StepReject::OrbitOutsideCard => unsealed(SATELLITE_ORBIT_OUTSIDE_CARD),
        satellite_lageos1::StepReject::EpochOutsideCard => unsealed(SATELLITE_EPOCH_OUTSIDE_CARD),
        satellite_lageos1::StepReject::HorizonOutsideCard => {
            unsealed(SATELLITE_HORIZON_OUTSIDE_CARD)
        }
        satellite_lageos1::StepReject::SpontaneousGeneration => unsealed(
            "the satellite gained energy that the work of the other forces does not account for (SpontaneousGeneration).",
        ),
        satellite_lageos1::StepReject::FirstLawBreach => unsealed(
            "the satellite lost energy that the work of the other forces does not account for (FirstLawBreach).",
        ),
        other => refused("satellite_lageos1", other),
    }
}

fn min_positive(values: [f64; 3]) -> Option<f64> {
    let mut best = f64::INFINITY;
    for value in values {
        if value.is_finite() && value > 0.0 && value < best {
            best = value;
        }
    }
    if best.is_finite() { Some(best) } else { None }
}

/// The caller names a load. Anything else is either a card constant or not a load.
fn reject_unknown_load(params: &serde_json::Value, allowed: &[&str]) -> Result<(), String> {
    let object = params
        .as_object()
        .ok_or_else(|| "Physical Violation: parameters must be a JSON object.".to_string())?;
    for key in object.keys() {
        if !allowed.iter().any(|allowed_key| *allowed_key == key) {
            return Err(locked_or_unknown(key));
        }
    }
    Ok(())
}

fn locked_or_unknown(key: &str) -> String {
    match key {
        "melt_k" => {
            "Physical Violation: 'melt_k' is fixed at 1357.77 K. The caller does not set the melting point of copper.".to_string()
        }
        "beta_transus_k" => {
            "Physical Violation: 'beta_transus_k' is fixed at 1268.15 K. The caller does not set the beta transus of Ti-6Al-4V.".to_string()
        }
        "yield_strength_mpa" => {
            "Physical Violation: 'yield_strength_mpa' is fixed at 880 MPa. The caller does not set the yield strength of Ti-6Al-4V.".to_string()
        }
        _ => format!("Physical Violation: '{key}' is not a field of this request."),
    }
}

/// The text a request is hashed as. It is written for hashing only and is not
/// JSON.
///
/// Object keys are sorted at every depth. Arrays keep their order. A string is
/// its JSON encoding. `true`, `false` and `null` are themselves. Every number
/// is `x` followed by the 16 lowercase hex digits of `f64::to_bits` of the
/// nearest f64, so `10`, `10.0` and `1e1` are one text, and `0.0` and `-0.0`
/// are two. There are no spaces.
///
/// No JSON value starts with `x`, so a number is never the text of a string,
/// an object or an array.
pub(crate) fn sealed_form(value: &serde_json::Value) -> String {
    let mut out = String::new();
    write_sealed_form(&mut out, value);
    out
}

fn write_sealed_form(out: &mut String, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => out.push_str("null"),
        serde_json::Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        serde_json::Value::Number(number) => {
            // `as_f64` is the reader every request number goes through, so
            // these are the bits the step was given.
            let nearest = number.as_f64().expect("a JSON number has a nearest f64");
            out.push('x');
            out.push_str(&format!("{:016x}", nearest.to_bits()));
        }
        serde_json::Value::String(text) => {
            out.push_str(&serde_json::to_string(text).expect("request string encodes"));
        }
        serde_json::Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_sealed_form(out, item);
            }
            out.push(']');
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).expect("request key encodes"));
                out.push(':');
                write_sealed_form(out, &map[key]);
            }
            out.push('}');
        }
    }
}

/// SHA-256 of the sealed form of the whole step request: `domain`, `card` and
/// `parameters`. The first 16 bytes are the frame proof.
///
/// The compiled step does not write this slot. The seal is the SHA-256 of the
/// whole 64-byte frame, so it binds the pair and the parameters to that step.
/// The response still echoes the request in the order it was received.
pub fn commit_request_proof(request: &serde_json::Value) -> [u8; 16] {
    let digest = Sha256::digest(sealed_form(request).as_bytes());
    let mut proof = [0_u8; 16];
    proof.copy_from_slice(&digest[..16]);
    proof
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

/// `serde_json` is built with `float_roundtrip`, so a decimal is read as the
/// nearest f64. The search looks for a token that parser reads back to `bits`.
fn token_keeps_bits(token: &str, bits: u64) -> bool {
    let wrapped = format!("{{\"n\":{token}}}");
    serde_json::from_str::<serde_json::Value>(&wrapped)
        .ok()
        .and_then(|value| value.get("n").and_then(serde_json::Value::as_f64))
        .is_some_and(|parsed| parsed.to_bits() == bits)
}

fn exact_f64_token(value: f64) -> String {
    let bits = value.to_bits();
    for digits in 1..=20 {
        let token = format!("{value:.digits$e}");
        if token_keeps_bits(&token, bits) {
            return token;
        }
    }
    let debug = format!("{value:?}");
    if token_keeps_bits(&debug, bits) {
        return debug;
    }
    for digits in 1..=20 {
        let token = format!("{value:.digits$}");
        if token_keeps_bits(&token, bits) {
            return token;
        }
    }
    debug
}

fn serialize_exact_f64<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let token = exact_f64_token(*value);
    let raw = serde_json::value::RawValue::from_string(token).map_err(serde::ser::Error::custom)?;
    raw.serialize(serializer)
}

#[derive(Clone, Copy)]
enum StateField {
    F64(f64),
    F64x3([f64; 3]),
    Bool(bool),
}

const NON_FINITE_STATE: &str =
    "Physical Violation: a post-step state field is not finite. That state is not sealed.";

fn finite_number(value: f64) -> Result<serde_json::Value, String> {
    let number = serde_json::Number::from_f64(value).ok_or_else(|| NON_FINITE_STATE.to_string())?;
    Ok(serde_json::Value::Number(number))
}

/// The same fields [`state_object`] publishes, checked without building JSON.
fn fields_are_finite(entries: &[(&str, StateField)]) -> Result<(), String> {
    for (_, field) in entries {
        let finite = match *field {
            StateField::F64(value) => value.is_finite(),
            StateField::F64x3(values) => values.iter().all(|value| value.is_finite()),
            StateField::Bool(_) => true,
        };
        if !finite {
            return Err(NON_FINITE_STATE.to_string());
        }
    }
    Ok(())
}

/// Raw IEEE-754 bits, 16 lowercase hex digits. `293.15` is `4072526666666666`.
fn f64_hex(value: f64) -> serde_json::Value {
    serde_json::Value::String(format!("{:016x}", value.to_bits()))
}

fn state_object(entries: &[(&str, StateField)]) -> Result<serde_json::Value, String> {
    let mut object = serde_json::Map::new();
    for (key, field) in entries {
        match *field {
            StateField::F64(value) => {
                object.insert((*key).to_string(), finite_number(value)?);
                object.insert(format!("{key}_hex"), f64_hex(value));
            }
            StateField::F64x3(values) => {
                let mut items = Vec::with_capacity(3);
                let mut hexes = Vec::with_capacity(3);
                for component in values {
                    items.push(finite_number(component)?);
                    hexes.push(f64_hex(component));
                }
                object.insert((*key).to_string(), serde_json::Value::Array(items));
                object.insert(format!("{key}_hex"), serde_json::Value::Array(hexes));
            }
            StateField::Bool(value) => {
                object.insert((*key).to_string(), serde_json::Value::Bool(value));
            }
        }
    }
    Ok(serde_json::Value::Object(object))
}

fn write_exact_value(out: &mut String, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => out.push_str("null"),
        serde_json::Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        serde_json::Value::Number(number) => {
            let token = match number.as_f64() {
                Some(value) => exact_f64_token(value),
                None => number.to_string(),
            };
            out.push_str(&token);
        }
        serde_json::Value::String(text) => {
            out.push_str(&serde_json::to_string(text).expect("state string encodes"));
        }
        serde_json::Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_exact_value(out, item);
            }
            out.push(']');
        }
        serde_json::Value::Object(map) => {
            out.push('{');
            for (index, (key, child)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).expect("state key encodes"));
                out.push(':');
                write_exact_value(out, child);
            }
            out.push('}');
        }
    }
}

fn serialize_state_value<S>(value: &serde_json::Value, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut token = String::new();
    write_exact_value(&mut token, value);
    let raw = serde_json::value::RawValue::from_string(token).map_err(serde::ser::Error::custom)?;
    raw.serialize(serializer)
}

fn ti64_state(state: &ti64_lattice::Ti64LatticeState) -> Result<serde_json::Value, String> {
    ti64_with(state, state_object)
}

pub(crate) fn ti64_published_is_finite(
    state: &ti64_lattice::Ti64LatticeState,
) -> Result<(), String> {
    ti64_with(state, fields_are_finite)
}

fn ti64_with<T>(
    state: &ti64_lattice::Ti64LatticeState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("position_m", StateField::F64x3(state.position_m)),
        ("velocity_m_s", StateField::F64x3(state.velocity_m_s)),
        ("tension_n", StateField::F64x3(state.tension_n)),
        ("torque_nm", StateField::F64x3(state.torque_nm)),
        ("applied_force_n", StateField::F64x3(state.applied_force_n)),
        ("temperature_k", StateField::F64(state.temperature_k)),
        (
            "ligament_stress_pa",
            StateField::F64x3(state.ligament_stress_pa),
        ),
        ("von_mises_mpa", StateField::F64(state.von_mises_mpa)),
        ("energy_j", StateField::F64(state.energy_j)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("boundary_work_j", StateField::F64(state.boundary_work_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
    ])
}

fn grid_state(state: &grid_node::GridNodeState) -> Result<serde_json::Value, String> {
    grid_with(state, state_object)
}

pub(crate) fn grid_published_is_finite(state: &grid_node::GridNodeState) -> Result<(), String> {
    grid_with(state, fields_are_finite)
}

fn grid_with<T>(
    state: &grid_node::GridNodeState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("current_a", StateField::F64(state.current_a)),
        ("voltage_v", StateField::F64(state.voltage_v)),
        ("temperature_k", StateField::F64(state.temperature_k)),
        (
            "upstream_voltage_v",
            StateField::F64(state.upstream_voltage_v),
        ),
        ("load_current_a", StateField::F64(state.load_current_a)),
        ("ambient_k", StateField::F64(state.ambient_k)),
        ("resistance_ohm", StateField::F64(state.resistance_ohm)),
        ("joule_power_w", StateField::F64(state.joule_power_w)),
        (
            "electrical_energy_j",
            StateField::F64(state.electrical_energy_j),
        ),
        ("thermal_energy_j", StateField::F64(state.thermal_energy_j)),
        (
            "internal_energy_j",
            StateField::F64(state.internal_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("boundary_work_j", StateField::F64(state.boundary_work_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("voltage_spike_v", StateField::F64(state.voltage_spike_v)),
        ("melt_margin_k", StateField::F64(state.melt_margin_k)),
        (
            "entropy_production_w_per_k",
            StateField::F64(state.entropy_production_w_per_k),
        ),
        ("surge", StateField::Bool(state.surge)),
    ])
}

fn thermal_state(state: &thermal::ThermalNodeState) -> Result<serde_json::Value, String> {
    thermal_with(state, state_object)
}

pub(crate) fn thermal_published_is_finite(state: &thermal::ThermalNodeState) -> Result<(), String> {
    thermal_with(state, fields_are_finite)
}

fn thermal_with<T>(
    state: &thermal::ThermalNodeState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("temperature_k", StateField::F64(state.temperature_k)),
        ("heat_power_w", StateField::F64(state.heat_power_w)),
        ("ambient_k", StateField::F64(state.ambient_k)),
        (
            "internal_energy_j",
            StateField::F64(state.internal_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("boundary_work_j", StateField::F64(state.boundary_work_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "heat_to_ambient_w",
            StateField::F64(state.heat_to_ambient_w),
        ),
        ("ceiling_margin_k", StateField::F64(state.ceiling_margin_k)),
        (
            "entropy_production_w_per_k",
            StateField::F64(state.entropy_production_w_per_k),
        ),
        ("derated", StateField::Bool(state.derated)),
    ])
}

fn resonance_state(state: &resonance::OscillatorState) -> Result<serde_json::Value, String> {
    resonance_with(state, state_object)
}

pub(crate) fn resonance_published_is_finite(
    state: &resonance::OscillatorState,
) -> Result<(), String> {
    resonance_with(state, fields_are_finite)
}

fn resonance_with<T>(
    state: &resonance::OscillatorState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("displacement_m", StateField::F64(state.displacement_m)),
        ("velocity_m_s", StateField::F64(state.velocity_m_s)),
        ("accel_m_s2", StateField::F64(state.accel_m_s2)),
        ("kinetic_j", StateField::F64(state.kinetic_j)),
        ("potential_j", StateField::F64(state.potential_j)),
        (
            "mechanical_energy_j",
            StateField::F64(state.mechanical_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("boundary_work_j", StateField::F64(state.boundary_work_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("stroke_margin_m", StateField::F64(state.stroke_margin_m)),
        ("stroke_limited", StateField::Bool(state.stroke_limited)),
    ])
}

fn swing_state(state: &swing::SwingState) -> Result<serde_json::Value, String> {
    swing_with(state, state_object)
}

pub(crate) fn swing_published_is_finite(state: &swing::SwingState) -> Result<(), String> {
    swing_with(state, fields_are_finite)
}

fn swing_with<T>(state: &swing::SwingState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("delta_rad", StateField::F64(state.delta_rad)),
        (
            "delta_omega_rad_s",
            StateField::F64(state.delta_omega_rad_s),
        ),
        ("pll_err_rad", StateField::F64(state.pll_err_rad)),
        ("p_mech_pu", StateField::F64(state.p_mech_pu)),
        ("p_elec_pu", StateField::F64(state.p_elec_pu)),
        (
            "transient_energy_pu",
            StateField::F64(state.transient_energy_pu),
        ),
        ("energy_anchor_pu", StateField::F64(state.energy_anchor_pu)),
        ("boundary_work_pu", StateField::F64(state.boundary_work_pu)),
        ("energy_defect_pu", StateField::F64(state.energy_defect_pu)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "synchronism_margin_rad",
            StateField::F64(state.synchronism_margin_rad),
        ),
        ("pll_slipped", StateField::Bool(state.pll_slipped)),
    ])
}

fn tesseract_state(state: &tesseract::TesseractState) -> Result<serde_json::Value, String> {
    tesseract_with(state, state_object)
}

pub(crate) fn tesseract_published_is_finite(
    state: &tesseract::TesseractState,
) -> Result<(), String> {
    tesseract_with(state, fields_are_finite)
}

fn tesseract_with<T>(
    state: &tesseract::TesseractState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("displacement_m", StateField::F64(state.displacement_m)),
        ("velocity_m_s", StateField::F64(state.velocity_m_s)),
        ("accel_m_s2", StateField::F64(state.accel_m_s2)),
        ("envelope_m", StateField::F64(state.envelope_m)),
        ("kinetic_j", StateField::F64(state.kinetic_j)),
        ("potential_j", StateField::F64(state.potential_j)),
        (
            "mechanical_energy_j",
            StateField::F64(state.mechanical_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("boundary_work_j", StateField::F64(state.boundary_work_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("scale_factor_n", StateField::F64(state.scale_factor_n)),
    ])
}

fn hydraulic_state(state: &hydraulic_fluid::FluidState) -> Result<serde_json::Value, String> {
    hydraulic_with(state, state_object)
}

pub(crate) fn hydraulic_published_is_finite(
    state: &hydraulic_fluid::FluidState,
) -> Result<(), String> {
    hydraulic_with(state, fields_are_finite)
}

fn hydraulic_with<T>(
    state: &hydraulic_fluid::FluidState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("pressure_pa", StateField::F64(state.pressure_pa)),
        ("flow_m3_s", StateField::F64(state.flow_m3_s)),
        ("fluid_volume_m3", StateField::F64(state.fluid_volume_m3)),
        ("compression_m", StateField::F64(state.compression_m)),
        ("mass_kg", StateField::F64(state.mass_kg)),
        ("elastic_energy_j", StateField::F64(state.elastic_energy_j)),
        ("mass_anchor_kg", StateField::F64(state.mass_anchor_kg)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("boundary_mass_kg", StateField::F64(state.boundary_mass_kg)),
        ("boundary_work_j", StateField::F64(state.boundary_work_j)),
        ("mass_defect_kg", StateField::F64(state.mass_defect_kg)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("vapor_margin_pa", StateField::F64(state.vapor_margin_pa)),
        ("column_soft", StateField::Bool(state.column_soft)),
        ("column_limited", StateField::Bool(state.column_limited)),
    ])
}

fn cable_state(state: &cable_stretch::TendonState) -> Result<serde_json::Value, String> {
    cable_with(state, state_object)
}

pub(crate) fn cable_published_is_finite(state: &cable_stretch::TendonState) -> Result<(), String> {
    cable_with(state, fields_are_finite)
}

fn cable_with<T>(
    state: &cable_stretch::TendonState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("elongation_m", StateField::F64(state.elongation_m)),
        ("velocity_m_s", StateField::F64(state.velocity_m_s)),
        ("force_n", StateField::F64(state.force_n)),
        ("tension_n", StateField::F64(state.tension_n)),
        ("kinetic_j", StateField::F64(state.kinetic_j)),
        ("potential_j", StateField::F64(state.potential_j)),
        (
            "mechanical_energy_j",
            StateField::F64(state.mechanical_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("boundary_work_j", StateField::F64(state.boundary_work_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "ultimate_margin_n",
            StateField::F64(state.ultimate_margin_n),
        ),
        (
            "tracking_degraded",
            StateField::Bool(state.tracking_degraded),
        ),
        ("control_limited", StateField::Bool(state.control_limited)),
    ])
}

fn battery_state(state: &battery_sag::PackState) -> Result<serde_json::Value, String> {
    battery_with(state, state_object)
}

pub(crate) fn battery_published_is_finite(state: &battery_sag::PackState) -> Result<(), String> {
    battery_with(state, fields_are_finite)
}

fn battery_with<T>(
    state: &battery_sag::PackState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("soc", StateField::F64(state.soc)),
        ("current_a", StateField::F64(state.current_a)),
        ("terminal_v", StateField::F64(state.terminal_v)),
        ("open_circuit_v", StateField::F64(state.open_circuit_v)),
        ("resistance_ohm", StateField::F64(state.resistance_ohm)),
        (
            "chemical_energy_j",
            StateField::F64(state.chemical_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        (
            "electrical_work_j",
            StateField::F64(state.electrical_work_j),
        ),
        ("joule_j", StateField::F64(state.joule_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("cutoff_margin_v", StateField::F64(state.cutoff_margin_v)),
        ("sag_warning", StateField::Bool(state.sag_warning)),
    ])
}

fn cell_state(state: &cell_18650pf::CellState) -> Result<serde_json::Value, String> {
    cell_with(state, state_object)
}

pub(crate) fn cell_published_is_finite(state: &cell_18650pf::CellState) -> Result<(), String> {
    cell_with(state, fields_are_finite)
}

fn cell_with<T>(
    state: &cell_18650pf::CellState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("soc", StateField::F64(state.soc)),
        ("current_a", StateField::F64(state.current_a)),
        ("terminal_v", StateField::F64(state.terminal_v)),
        ("open_circuit_v", StateField::F64(state.open_circuit_v)),
        ("resistance_ohm", StateField::F64(state.resistance_ohm)),
        (
            "chemical_energy_j",
            StateField::F64(state.chemical_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        (
            "electrical_work_j",
            StateField::F64(state.electrical_work_j),
        ),
        ("joule_j", StateField::F64(state.joule_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("cutoff_margin_v", StateField::F64(state.cutoff_margin_v)),
        ("sag_warning", StateField::Bool(state.sag_warning)),
    ])
}

fn cell_2170_state(state: &cell_2170_m80::CellState) -> Result<serde_json::Value, String> {
    cell_2170_with(state, state_object)
}

pub(crate) fn cell_2170_published_is_finite(
    state: &cell_2170_m80::CellState,
) -> Result<(), String> {
    cell_2170_with(state, fields_are_finite)
}

fn cell_2170_with<T>(
    state: &cell_2170_m80::CellState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("block_chamber_c", StateField::F64(state.block_chamber_c)),
        (
            "charge_removed_ah",
            StateField::F64(state.charge_removed_ah),
        ),
        ("charge_level", StateField::F64(state.charge_level)),
        ("current_a", StateField::F64(state.current_a)),
        (
            "case_temperature_c",
            StateField::F64(state.case_temperature_c),
        ),
        ("terminal_v", StateField::F64(state.terminal_v)),
        ("open_circuit_v", StateField::F64(state.open_circuit_v)),
        ("branch_v", StateField::F64(state.branch_v)),
        (
            "resistance_at_once_ohm",
            StateField::F64(state.resistance_at_once_ohm),
        ),
        (
            "resistance_branch_ohm",
            StateField::F64(state.resistance_branch_ohm),
        ),
        ("time_constant_s", StateField::F64(state.time_constant_s)),
        (
            "chemical_energy_j",
            StateField::F64(state.chemical_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        (
            "electrical_work_j",
            StateField::F64(state.electrical_work_j),
        ),
        ("heat_at_once_j", StateField::F64(state.heat_at_once_j)),
        ("branch_energy_j", StateField::F64(state.branch_energy_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "lower_limit_margin_v",
            StateField::F64(state.lower_limit_margin_v),
        ),
        (
            "upper_limit_margin_v",
            StateField::F64(state.upper_limit_margin_v),
        ),
    ])
}

fn cell_2170_model3_state(
    state: &cell_2170_model3::CellState,
) -> Result<serde_json::Value, String> {
    cell_2170_model3_with(state, state_object)
}

pub(crate) fn cell_2170_model3_published_is_finite(
    state: &cell_2170_model3::CellState,
) -> Result<(), String> {
    cell_2170_model3_with(state, fields_are_finite)
}

/// Every number of the state: what the caller gave, the eleven diffusion
/// modes and the four other memories, what the last step read and returned,
/// and the ledger. The least and the most case temperature since the rested
/// start are published, so that a run continued from this state carries them as
/// its start fields.
fn cell_2170_model3_with<T>(
    state: &cell_2170_model3::CellState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    let lag = &state.diffusion_lag;
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("cell_number", StateField::F64(state.cell_number)),
        ("capacity_ah", StateField::F64(state.capacity_ah)),
        ("state_of_charge", StateField::F64(state.state_of_charge)),
        (
            "surface_state_of_charge",
            StateField::F64(state.surface_state_of_charge),
        ),
        (
            "charge_removed_ah",
            StateField::F64(state.charge_removed_ah),
        ),
        ("current_a", StateField::F64(state.current_a)),
        (
            "case_temperature_c",
            StateField::F64(state.case_temperature_c),
        ),
        (
            "case_temperature_least_c",
            StateField::F64(state.case_temperature_least_c),
        ),
        (
            "case_temperature_most_c",
            StateField::F64(state.case_temperature_most_c),
        ),
        ("terminal_v", StateField::F64(state.terminal_v)),
        ("rest_v", StateField::F64(state.rest_v)),
        (
            "resistance_at_once_ohm",
            StateField::F64(state.resistance_at_once_ohm),
        ),
        (
            "reaction_overpotential_v",
            StateField::F64(state.reaction_overpotential_v),
        ),
        (
            "near_full_overpotential_v",
            StateField::F64(state.near_full_overpotential_v),
        ),
        ("hysteresis_v", StateField::F64(state.hysteresis_v)),
        ("electrolyte_v", StateField::F64(state.electrolyte_v)),
        ("current_mean_a", StateField::F64(state.current_mean_a)),
        ("diffusion_lag_1", StateField::F64(lag[0])),
        ("diffusion_lag_2", StateField::F64(lag[1])),
        ("diffusion_lag_3", StateField::F64(lag[2])),
        ("diffusion_lag_4", StateField::F64(lag[3])),
        ("diffusion_lag_5", StateField::F64(lag[4])),
        ("diffusion_lag_6", StateField::F64(lag[5])),
        ("diffusion_lag_7", StateField::F64(lag[6])),
        ("diffusion_lag_8", StateField::F64(lag[7])),
        ("diffusion_lag_9", StateField::F64(lag[8])),
        ("diffusion_lag_10", StateField::F64(lag[9])),
        ("diffusion_lag_11", StateField::F64(lag[10])),
        (
            "chemical_energy_j",
            StateField::F64(state.chemical_energy_j),
        ),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        (
            "electrical_work_j",
            StateField::F64(state.electrical_work_j),
        ),
        ("heat_j", StateField::F64(state.heat_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "sustained_above_1c",
            StateField::Bool(state.sustained_above_1c),
        ),
    ])
}

fn generator_state(
    state: &generator_mmrtg_f1::GeneratorState,
) -> Result<serde_json::Value, String> {
    generator_with(state, state_object)
}

pub(crate) fn generator_published_is_finite(
    state: &generator_mmrtg_f1::GeneratorState,
) -> Result<(), String> {
    generator_with(state, fields_are_finite)
}

fn generator_with<T>(
    state: &generator_mmrtg_f1::GeneratorState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("thermal_power_w", StateField::F64(state.thermal_power_w)),
        (
            "converter_fading_fraction",
            StateField::F64(state.converter_fading_fraction),
        ),
        (
            "electrical_power_w",
            StateField::F64(state.electrical_power_w),
        ),
        (
            "conversion_efficiency",
            StateField::F64(state.conversion_efficiency),
        ),
        ("fuel_heat_j", StateField::F64(state.fuel_heat_j)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        (
            "electrical_energy_j",
            StateField::F64(state.electrical_energy_j),
        ),
        ("heat_rejected_j", StateField::F64(state.heat_rejected_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "converter_fading_yield_j",
            StateField::F64(state.converter_fading_yield_j),
        ),
        (
            "converter_account_anchor_j",
            StateField::F64(state.converter_account_anchor_j),
        ),
        (
            "fading_electrical_energy_j",
            StateField::F64(state.fading_electrical_energy_j),
        ),
        (
            "converter_account_defect_j",
            StateField::F64(state.converter_account_defect_j),
        ),
        (
            "converter_account_error",
            StateField::F64(state.converter_account_error),
        ),
        ("range_margin_s", StateField::F64(state.range_margin_s)),
        ("past_fitting_span", StateField::Bool(state.past_fitting_span)),
    ])
}

fn crater_state(state: &crater_ryugu_sci::CraterState) -> Result<serde_json::Value, String> {
    crater_with(state, state_object)
}

pub(crate) fn crater_published_is_finite(
    state: &crater_ryugu_sci::CraterState,
) -> Result<(), String> {
    crater_with(state, fields_are_finite)
}

fn crater_with<T>(
    state: &crater_ryugu_sci::CraterState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("radius_m", StateField::F64(state.radius_m)),
        ("wall_speed_m_s", StateField::F64(state.wall_speed_m_s)),
        ("radius_over_final", StateField::F64(state.radius_over_final)),
        ("radius_to_go_m", StateField::F64(state.radius_to_go_m)),
        ("kinetic_j_per_kg", StateField::F64(state.kinetic_j_per_kg)),
        ("given_up_j_per_kg", StateField::F64(state.given_up_j_per_kg)),
        (
            "energy_defect_j_per_kg",
            StateField::F64(state.energy_defect_j_per_kg),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("stopped", StateField::Bool(state.stopped)),
    ])
}

fn link_state(
    state: &link_gothenburg_10010::LinkState,
) -> Result<serde_json::Value, String> {
    link_with(state, state_object)
}

pub(crate) fn link_published_is_finite(
    state: &link_gothenburg_10010::LinkState,
) -> Result<(), String> {
    link_with(state, fields_are_finite)
}

fn link_with<T>(
    state: &link_gothenburg_10010::LinkState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("dry_path_loss_db", StateField::F64(state.dry_path_loss_db)),
        ("antenna_water_db", StateField::F64(state.antenna_water_db)),
        ("rain_rate_mm_h", StateField::F64(state.rain_rate_mm_h)),
        ("rain_attenuation_db", StateField::F64(state.rain_attenuation_db)),
        ("path_loss_db", StateField::F64(state.path_loss_db)),
        ("received_level_dbm", StateField::F64(state.received_level_dbm)),
        ("received_power_w", StateField::F64(state.received_power_w)),
        ("dry_path_power_w", StateField::F64(state.dry_path_power_w)),
        ("dry_path_energy_j", StateField::F64(state.dry_path_energy_j)),
        ("received_energy_j", StateField::F64(state.received_energy_j)),
        ("rain_energy_j", StateField::F64(state.rain_energy_j)),
        ("antenna_energy_j", StateField::F64(state.antenna_energy_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "antenna_account_anchor_db",
            StateField::F64(state.antenna_account_anchor_db),
        ),
        ("antenna_built_db", StateField::F64(state.antenna_built_db)),
        ("antenna_dried_db", StateField::F64(state.antenna_dried_db)),
        (
            "antenna_account_defect_db",
            StateField::F64(state.antenna_account_defect_db),
        ),
        (
            "antenna_account_error",
            StateField::F64(state.antenna_account_error),
        ),
        (
            "rain_rate_margin_mm_h",
            StateField::F64(state.rain_rate_margin_mm_h),
        ),
        (
            "gauge_minute_elapsed_s",
            StateField::F64(state.gauge_minute_elapsed_s),
        ),
        (
            "gauge_minute_rain_mm_h",
            StateField::F64(state.gauge_minute_rain_mm_h),
        ),
        ("raining", StateField::Bool(state.raining)),
    ])
}

fn accumulator_state(state: &accumulator_sbo450::GasState) -> Result<serde_json::Value, String> {
    accumulator_with(state, state_object)
}

pub(crate) fn accumulator_published_is_finite(
    state: &accumulator_sbo450::GasState,
) -> Result<(), String> {
    accumulator_with(state, fields_are_finite)
}

fn accumulator_with<T>(
    state: &accumulator_sbo450::GasState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("gas_mass_kg", StateField::F64(state.gas_mass_kg)),
        ("wall_temperature_k", StateField::F64(state.wall_temperature_k)),
        ("gas_temperature_k", StateField::F64(state.gas_temperature_k)),
        ("gas_volume_m3", StateField::F64(state.gas_volume_m3)),
        ("gas_pressure_pa", StateField::F64(state.gas_pressure_pa)),
        ("gas_force_n", StateField::F64(state.gas_force_n)),
        ("work_on_gas_j", StateField::F64(state.work_on_gas_j)),
        ("heat_to_wall_j", StateField::F64(state.heat_to_wall_j)),
        ("start_energy_j", StateField::F64(state.start_energy_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("work_moved_j", StateField::F64(state.work_moved_j)),
        ("heat_moved_j", StateField::F64(state.heat_moved_j)),
        ("heat_share", StateField::F64(state.heat_share)),
    ])
}

fn tire_state(state: &tire_eve_r19_ice::TireState) -> Result<serde_json::Value, String> {
    tire_with(state, state_object)
}

pub(crate) fn tire_published_is_finite(
    state: &tire_eve_r19_ice::TireState,
) -> Result<(), String> {
    tire_with(state, fields_are_finite)
}

fn tire_with<T>(
    state: &tire_eve_r19_ice::TireState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("distance_m", StateField::F64(state.distance_m)),
        ("rim_travel_m", StateField::F64(state.rim_travel_m)),
        ("slip_ratio", StateField::F64(state.slip_ratio)),
        ("slip_held_s", StateField::F64(state.slip_held_s)),
        (
            "longitudinal_force_n",
            StateField::F64(state.longitudinal_force_n),
        ),
        ("wheel_work_j", StateField::F64(state.wheel_work_j)),
        ("drawbar_work_j", StateField::F64(state.drawbar_work_j)),
        ("slip_heat_j", StateField::F64(state.slip_heat_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "load_above_fitted",
            StateField::Bool(state.load_above_fitted),
        ),
    ])
}

fn los_state(state: &los_jwst_fine_guide::LosState) -> Result<serde_json::Value, String> {
    los_with(state, state_object)
}

pub(crate) fn los_published_is_finite(
    state: &los_jwst_fine_guide::LosState,
) -> Result<(), String> {
    los_with(state, fields_are_finite)
}

/// The ten covariance entries of one axis, in the kernel's order.
const LOS_COV_X: [&str; 10] = [
    "cov_x_slow_slow_mas2",
    "cov_x_slow_fast_mas2",
    "cov_x_slow_cos_mas2",
    "cov_x_slow_sin_mas2",
    "cov_x_fast_fast_mas2",
    "cov_x_fast_cos_mas2",
    "cov_x_fast_sin_mas2",
    "cov_x_cos_cos_mas2",
    "cov_x_cos_sin_mas2",
    "cov_x_sin_sin_mas2",
];
const LOS_COV_Y: [&str; 10] = [
    "cov_y_slow_slow_mas2",
    "cov_y_slow_fast_mas2",
    "cov_y_slow_cos_mas2",
    "cov_y_slow_sin_mas2",
    "cov_y_fast_fast_mas2",
    "cov_y_fast_cos_mas2",
    "cov_y_fast_sin_mas2",
    "cov_y_cos_cos_mas2",
    "cov_y_cos_sin_mas2",
    "cov_y_sin_sin_mas2",
];

fn los_with<T>(
    state: &los_jwst_fine_guide::LosState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    let mut fields: Vec<(&str, StateField)> = vec![
        ("t_s", StateField::F64(state.t_s)),
        ("lock_x_arcsec", StateField::F64(state.lock_x_arcsec)),
        ("lock_y_arcsec", StateField::F64(state.lock_y_arcsec)),
        (
            "quiet_before_start_s",
            StateField::F64(state.quiet_before_start_s),
        ),
        ("slow_x_mas", StateField::F64(state.x.slow_mas)),
        ("fast_x_mas", StateField::F64(state.x.fast_mas)),
        ("mode_cos_x_mas", StateField::F64(state.x.mode_cos_mas)),
        ("mode_sin_x_mas", StateField::F64(state.x.mode_sin_mas)),
        ("slow_y_mas", StateField::F64(state.y.slow_mas)),
        ("fast_y_mas", StateField::F64(state.y.fast_mas)),
        ("mode_cos_y_mas", StateField::F64(state.y.mode_cos_mas)),
        ("mode_sin_y_mas", StateField::F64(state.y.mode_sin_mas)),
    ];
    for (name, value) in LOS_COV_X.iter().zip(state.x.cov_mas2) {
        fields.push((name, StateField::F64(value)));
    }
    for (name, value) in LOS_COV_Y.iter().zip(state.y.cov_mas2) {
        fields.push((name, StateField::F64(value)));
    }
    fields.extend([
        ("predicted_x_mas", StateField::F64(state.predicted_x_mas)),
        ("predicted_y_mas", StateField::F64(state.predicted_y_mas)),
        ("innovation_x_mas", StateField::F64(state.innovation_x_mas)),
        ("innovation_y_mas", StateField::F64(state.innovation_y_mas)),
        (
            "innovation_variance_x_mas2",
            StateField::F64(state.innovation_variance_x_mas2),
        ),
        (
            "innovation_variance_y_mas2",
            StateField::F64(state.innovation_variance_y_mas2),
        ),
        ("estimate_x_mas", StateField::F64(state.estimate_x_mas)),
        ("estimate_y_mas", StateField::F64(state.estimate_y_mas)),
        (
            "estimate_variance_x_mas2",
            StateField::F64(state.estimate_variance_x_mas2),
        ),
        (
            "estimate_variance_y_mas2",
            StateField::F64(state.estimate_variance_y_mas2),
        ),
        ("nis", StateField::F64(state.nis)),
        ("measured_frames", StateField::F64(state.measured_frames)),
        ("nis_sum_x", StateField::F64(state.nis_sum_x)),
        ("nis_sum_y", StateField::F64(state.nis_sum_y)),
        ("variance_x_mas2", StateField::F64(state.variance_x_mas2)),
        ("variance_y_mas2", StateField::F64(state.variance_y_mas2)),
        (
            "variance_defect_x_mas2",
            StateField::F64(state.variance_defect_x_mas2),
        ),
        (
            "variance_defect_y_mas2",
            StateField::F64(state.variance_defect_y_mas2),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("unmeasured_in_row", StateField::F64(state.unmeasured_in_row)),
        ("no_measurement", StateField::Bool(state.no_measurement)),
    ]);
    finish(&fields)
}

fn cantilever_state(
    state: &cantilever_eth::CantileverState,
) -> Result<serde_json::Value, String> {
    cantilever_with(state, state_object)
}

pub(crate) fn cantilever_published_is_finite(
    state: &cantilever_eth::CantileverState,
) -> Result<(), String> {
    cantilever_with(state, fields_are_finite)
}

fn cantilever_with<T>(
    state: &cantilever_eth::CantileverState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("deflection_m", StateField::F64(state.deflection_m)),
        ("velocity_m_s", StateField::F64(state.velocity_m_s)),
        ("envelope_m", StateField::F64(state.envelope_m)),
        ("kinetic_j_per_kg", StateField::F64(state.kinetic_j_per_kg)),
        (
            "potential_j_per_kg",
            StateField::F64(state.potential_j_per_kg),
        ),
        ("stored_j_per_kg", StateField::F64(state.stored_j_per_kg)),
        (
            "energy_anchor_j_per_kg",
            StateField::F64(state.energy_anchor_j_per_kg),
        ),
        ("heat_j_per_kg", StateField::F64(state.heat_j_per_kg)),
        (
            "energy_defect_j_per_kg",
            StateField::F64(state.energy_defect_j_per_kg),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
    ])
}

fn capsule_insight_state(
    state: &capsule_insight::CapsuleState,
) -> Result<serde_json::Value, String> {
    capsule_insight_with(state, state_object)
}

pub(crate) fn capsule_insight_published_is_finite(
    state: &capsule_insight::CapsuleState,
) -> Result<(), String> {
    capsule_insight_with(state, fields_are_finite)
}

fn capsule_insight_with<T>(
    state: &capsule_insight::CapsuleState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("altitude_m", StateField::F64(state.altitude_m)),
        ("radius_m", StateField::F64(state.radius_m)),
        ("radial_speed_m_s", StateField::F64(state.radial_speed_m_s)),
        (
            "transverse_speed_m_s",
            StateField::F64(state.transverse_speed_m_s),
        ),
        ("angle_rad", StateField::F64(state.angle_rad)),
        (
            "ground_distance_m",
            StateField::F64(state.ground_distance_m),
        ),
        ("air_speed_m_s", StateField::F64(state.air_speed_m_s)),
        ("mach", StateField::F64(state.mach)),
        ("load_g", StateField::F64(state.load_g)),
        (
            "dynamic_pressure_pa",
            StateField::F64(state.dynamic_pressure_pa),
        ),
        ("drag_loss_j_kg", StateField::F64(state.drag_loss_j_kg)),
        (
            "energy_anchor_j_kg",
            StateField::F64(state.energy_anchor_j_kg),
        ),
        (
            "energy_defect_j_kg",
            StateField::F64(state.energy_defect_j_kg),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("lift_acting", StateField::Bool(state.lift_acting)),
    ])
}

fn capsule_state(state: &capsule_stardust::CapsuleState) -> Result<serde_json::Value, String> {
    capsule_with(state, state_object)
}

pub(crate) fn capsule_published_is_finite(
    state: &capsule_stardust::CapsuleState,
) -> Result<(), String> {
    capsule_with(state, fields_are_finite)
}

fn capsule_with<T>(
    state: &capsule_stardust::CapsuleState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("altitude_m", StateField::F64(state.altitude_m)),
        ("radius_m", StateField::F64(state.radius_m)),
        ("radial_speed_m_s", StateField::F64(state.radial_speed_m_s)),
        (
            "transverse_speed_m_s",
            StateField::F64(state.transverse_speed_m_s),
        ),
        ("angle_rad", StateField::F64(state.angle_rad)),
        (
            "ground_distance_m",
            StateField::F64(state.ground_distance_m),
        ),
        ("air_speed_m_s", StateField::F64(state.air_speed_m_s)),
        ("mach", StateField::F64(state.mach)),
        ("load_g", StateField::F64(state.load_g)),
        ("heat_flux_w_m2", StateField::F64(state.heat_flux_w_m2)),
        ("drag_loss_j_kg", StateField::F64(state.drag_loss_j_kg)),
        (
            "energy_anchor_j_kg",
            StateField::F64(state.energy_anchor_j_kg),
        ),
        (
            "energy_defect_j_kg",
            StateField::F64(state.energy_defect_j_kg),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "drogue_mach_window",
            StateField::Bool(state.drogue_mach_window),
        ),
    ])
}

fn satellite_state(
    state: &satellite_lageos1::SatelliteState,
) -> Result<serde_json::Value, String> {
    satellite_with(state, state_object)
}

pub(crate) fn satellite_published_is_finite(
    state: &satellite_lageos1::SatelliteState,
) -> Result<(), String> {
    satellite_with(state, fields_are_finite)
}

fn satellite_with<T>(
    state: &satellite_lageos1::SatelliteState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("epoch_utc_s", StateField::F64(state.epoch_utc_s())),
        ("forecast_elapsed_s", StateField::F64(state.forecast_elapsed_s())),
        ("position_m", StateField::F64x3(state.position_m)),
        ("velocity_m_s", StateField::F64x3(state.velocity_m_s)),
        ("radius_m", StateField::F64(state.radius_m)),
        (
            "semi_major_axis_m",
            StateField::F64(state.semi_major_axis_m),
        ),
        ("jacobi_j_per_kg", StateField::F64(state.jacobi_j_per_kg)),
        (
            "jacobi_anchor_j_per_kg",
            StateField::F64(state.jacobi_anchor_j_per_kg),
        ),
        (
            "external_work_j_per_kg",
            StateField::F64(state.external_work_j_per_kg),
        ),
        (
            "energy_defect_j_per_kg",
            StateField::F64(state.energy_defect_j_per_kg),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("in_earth_shadow", StateField::Bool(state.in_earth_shadow)),
    ])
}

fn solar_system_state(
    state: &solar_system::SolarSystemState,
) -> Result<serde_json::Value, String> {
    solar_system_with(state, state_object)
}

pub(crate) fn solar_system_published_is_finite(
    state: &solar_system::SolarSystemState,
) -> Result<(), String> {
    solar_system_with(state, fields_are_finite)
}

fn solar_system_with<T>(
    state: &solar_system::SolarSystemState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("sun_position_m", StateField::F64x3(state.position_m[0])),
        ("sun_velocity_m_s", StateField::F64x3(state.velocity_m_s[0])),
        ("mercury_position_m", StateField::F64x3(state.position_m[1])),
        ("mercury_velocity_m_s", StateField::F64x3(state.velocity_m_s[1])),
        ("venus_position_m", StateField::F64x3(state.position_m[2])),
        ("venus_velocity_m_s", StateField::F64x3(state.velocity_m_s[2])),
        ("earth_position_m", StateField::F64x3(state.position_m[3])),
        ("earth_velocity_m_s", StateField::F64x3(state.velocity_m_s[3])),
        ("moon_position_m", StateField::F64x3(state.position_m[4])),
        ("moon_velocity_m_s", StateField::F64x3(state.velocity_m_s[4])),
        ("mars_position_m", StateField::F64x3(state.position_m[5])),
        ("mars_velocity_m_s", StateField::F64x3(state.velocity_m_s[5])),
        ("jupiter_position_m", StateField::F64x3(state.position_m[6])),
        ("jupiter_velocity_m_s", StateField::F64x3(state.velocity_m_s[6])),
        ("saturn_position_m", StateField::F64x3(state.position_m[7])),
        ("saturn_velocity_m_s", StateField::F64x3(state.velocity_m_s[7])),
        ("uranus_position_m", StateField::F64x3(state.position_m[8])),
        ("uranus_velocity_m_s", StateField::F64x3(state.velocity_m_s[8])),
        ("neptune_position_m", StateField::F64x3(state.position_m[9])),
        ("neptune_velocity_m_s", StateField::F64x3(state.velocity_m_s[9])),
        ("ceres_position_m", StateField::F64x3(state.position_m[10])),
        ("ceres_velocity_m_s", StateField::F64x3(state.velocity_m_s[10])),
        ("pallas_position_m", StateField::F64x3(state.position_m[11])),
        ("pallas_velocity_m_s", StateField::F64x3(state.velocity_m_s[11])),
        ("vesta_position_m", StateField::F64x3(state.position_m[12])),
        ("vesta_velocity_m_s", StateField::F64x3(state.velocity_m_s[12])),
        ("earth_to_mars_m", StateField::F64x3(state.earth_to_mars_m)),
        ("earth_mars_distance_m", StateField::F64(state.earth_mars_distance_m)),
        ("energy", StateField::F64(state.energy)),
        ("energy_anchor", StateField::F64(state.energy_anchor)),
        ("booked_work", StateField::F64(state.booked_work)),
        ("energy_defect", StateField::F64(state.energy_defect)),
        ("angular_momentum", StateField::F64x3(state.angular_momentum)),
        (
            "angular_momentum_anchor",
            StateField::F64x3(state.angular_momentum_anchor),
        ),
        ("booked_torque", StateField::F64x3(state.booked_torque)),
        ("turning_defect", StateField::F64(state.turning_defect)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
    ])
}

fn creep_state(state: &creep_cmsx6::CreepState) -> Result<serde_json::Value, String> {
    creep_with(state, state_object)
}

pub(crate) fn creep_published_is_finite(state: &creep_cmsx6::CreepState) -> Result<(), String> {
    creep_with(state, fields_are_finite)
}

fn creep_with<T>(
    state: &creep_cmsx6::CreepState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        (
            "creep_extension_percent",
            StateField::F64(state.creep_extension_percent),
        ),
        (
            "creep_rate_percent_per_h",
            StateField::F64(state.creep_rate_percent_per_h),
        ),
        ("stress_mpa", StateField::F64(state.stress_mpa)),
        (
            "work_by_load_mj_per_m3",
            StateField::F64(state.work_by_load_mj_per_m3),
        ),
        (
            "heat_dissipated_mj_per_m3",
            StateField::F64(state.heat_dissipated_mj_per_m3),
        ),
        (
            "work_defect_mj_per_m3",
            StateField::F64(state.work_defect_mj_per_m3),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("creep_rate_rising", StateField::Bool(state.creep_rate_rising)),
    ])
}

fn grid_gb_state(state: &grid_gb::GridState) -> Result<serde_json::Value, String> {
    grid_gb_with(state, state_object)
}

pub(crate) fn grid_gb_published_is_finite(state: &grid_gb::GridState) -> Result<(), String> {
    grid_gb_with(state, fields_are_finite)
}

fn grid_gb_with<T>(
    state: &grid_gb::GridState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("frequency_hz", StateField::F64(state.frequency_hz)),
        ("response_mw", StateField::F64(state.response_mw)),
        ("relief_mw", StateField::F64(state.relief_mw)),
        ("net_power_mw", StateField::F64(state.net_power_mw)),
        (
            "frequency_rate_hz_per_s",
            StateField::F64(state.frequency_rate_hz_per_s),
        ),
        ("lowest_frequency_hz", StateField::F64(state.lowest_frequency_hz)),
        ("lowest_time_s", StateField::F64(state.lowest_time_s)),
        ("start_frequency_hz", StateField::F64(state.start_frequency_hz)),
        ("inertia_mw_s", StateField::F64(state.inertia_mw_s)),
        ("demand_mw", StateField::F64(state.demand_mw)),
        ("kinetic_energy_mw_s", StateField::F64(state.kinetic_energy_mw_s)),
        (
            "kinetic_energy_anchor_mw_s",
            StateField::F64(state.kinetic_energy_anchor_mw_s),
        ),
        ("energy_lost_mw_s", StateField::F64(state.energy_lost_mw_s)),
        ("energy_response_mw_s", StateField::F64(state.energy_response_mw_s)),
        (
            "frequency_deficit_integral_hz_s",
            StateField::F64(state.frequency_deficit_integral_hz_s),
        ),
        ("energy_defect_mw_s", StateField::F64(state.energy_defect_mw_s)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
    ])
}

fn fingerpad_state(
    state: &fingerpad_p02_glass::FingerPadState,
) -> Result<serde_json::Value, String> {
    fingerpad_with(state, state_object)
}

pub(crate) fn fingerpad_published_is_finite(
    state: &fingerpad_p02_glass::FingerPadState,
) -> Result<(), String> {
    fingerpad_with(state, fields_are_finite)
}

fn fingerpad_with<T>(
    state: &fingerpad_p02_glass::FingerPadState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("twist_deg", StateField::F64(state.twist_deg)),
        ("torque_n_m", StateField::F64(state.torque_n_m)),
        (
            "slip_torque_limit_n_m",
            StateField::F64(state.slip_torque_limit_n_m),
        ),
        ("stuck_fraction", StateField::F64(state.stuck_fraction)),
        ("plate_work_j", StateField::F64(state.plate_work_j)),
        ("stored_energy_j", StateField::F64(state.stored_energy_j)),
        ("friction_heat_j", StateField::F64(state.friction_heat_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        (
            "held_normal_force_n",
            StateField::F64(state.held_normal_force_n),
        ),
        (
            "held_twist_rate_deg_s",
            StateField::F64(state.held_twist_rate_deg_s),
        ),
        (
            "stored_energy_at_start_j",
            StateField::F64(state.stored_energy_at_start_j),
        ),
        ("full_slip", StateField::Bool(state.full_slip)),
    ])
}

fn tokamak_state(state: &tokamak_jet_dte1::TokamakState) -> Result<serde_json::Value, String> {
    tokamak_with(state, state_object)
}

pub(crate) fn tokamak_published_is_finite(
    state: &tokamak_jet_dte1::TokamakState,
) -> Result<(), String> {
    tokamak_with(state, fields_are_finite)
}

fn tokamak_with<T>(
    state: &tokamak_jet_dte1::TokamakState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("thermal_energy_j", StateField::F64(state.thermal_energy_j)),
        ("fast_ion_energy_j", StateField::F64(state.fast_ion_energy_j)),
        ("fusion_energy_j", StateField::F64(state.fusion_energy_j)),
        ("temperature_kev", StateField::F64(state.temperature_kev)),
        ("loss_power_w", StateField::F64(state.loss_power_w)),
        ("confinement_time_s", StateField::F64(state.confinement_time_s)),
        ("slowing_time_s", StateField::F64(state.slowing_time_s)),
        ("fusion_power_w", StateField::F64(state.fusion_power_w)),
        (
            "fusion_power_thermal_w",
            StateField::F64(state.fusion_power_thermal_w),
        ),
        ("beta_normalised", StateField::F64(state.beta_normalised)),
        ("greenwald_fraction", StateField::F64(state.greenwald_fraction)),
        ("safety_factor", StateField::F64(state.safety_factor)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("energy_supplied_j", StateField::F64(state.energy_supplied_j)),
        ("energy_lost_j", StateField::F64(state.energy_lost_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("near_density_limit", StateField::Bool(state.near_density_limit)),
    ])
}

fn reactor_state(state: &reactor_spert3::ReactorState) -> Result<serde_json::Value, String> {
    reactor_with(state, state_object)
}

pub(crate) fn reactor_published_is_finite(
    state: &reactor_spert3::ReactorState,
) -> Result<(), String> {
    reactor_with(state, fields_are_finite)
}

fn reactor_with<T>(
    state: &reactor_spert3::ReactorState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("power_mw", StateField::F64(state.power_mw)),
        ("energy_mws", StateField::F64(state.energy_mws)),
        (
            "power_rate_mw_per_s",
            StateField::F64(state.power_rate_mw_per_s),
        ),
        (
            "reactivity_net_dollar",
            StateField::F64(state.reactivity_net_dollar),
        ),
        (
            "coolant_temperature_k",
            StateField::F64(state.coolant_temperature_k),
        ),
        ("precursor_1_mw", StateField::F64(state.precursor_mw[0])),
        ("precursor_2_mw", StateField::F64(state.precursor_mw[1])),
        ("precursor_3_mw", StateField::F64(state.precursor_mw[2])),
        ("precursor_4_mw", StateField::F64(state.precursor_mw[3])),
        ("precursor_5_mw", StateField::F64(state.precursor_mw[4])),
        ("precursor_6_mw", StateField::F64(state.precursor_mw[5])),
        (
            "population_anchor_mw",
            StateField::F64(state.population_anchor_mw),
        ),
        (
            "reactivity_energy_integral_dollar_mws",
            StateField::F64(state.reactivity_energy_integral_dollar_mws),
        ),
        (
            "population_defect_mw",
            StateField::F64(state.population_defect_mw),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("peak_power_mw", StateField::F64(state.peak_power_mw)),
        ("peak_energy_mws", StateField::F64(state.peak_energy_mws)),
        ("peak_time_s", StateField::F64(state.peak_time_s)),
        (
            "past_first_maximum",
            StateField::Bool(state.past_first_maximum),
        ),
        (
            "prompt_supercritical",
            StateField::Bool(state.prompt_supercritical),
        ),
    ])
}

fn reactor_triga_state(
    state: &reactor_triga_jsi::ReactorState,
) -> Result<serde_json::Value, String> {
    reactor_triga_with(state, state_object)
}

pub(crate) fn reactor_triga_published_is_finite(
    state: &reactor_triga_jsi::ReactorState,
) -> Result<(), String> {
    reactor_triga_with(state, fields_are_finite)
}

fn reactor_triga_with<T>(
    state: &reactor_triga_jsi::ReactorState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("power_mw", StateField::F64(state.power_mw)),
        ("energy_mws", StateField::F64(state.energy_mws)),
        ("power_rate_mw_per_s", StateField::F64(state.power_rate_mw_per_s)),
        ("fuel_temperature_k", StateField::F64(state.fuel_temperature_k)),
        ("reactivity_net_dollar", StateField::F64(state.reactivity_net_dollar)),
        (
            "reactivity_inserted_now_dollar",
            StateField::F64(state.reactivity_inserted_now_dollar),
        ),
        ("start_temperature_k", StateField::F64(state.start_temperature_k)),
        ("rod_table_core", StateField::F64(state.rod_table_core)),
        ("reactivity_load_dollar", StateField::F64(state.reactivity_load_dollar)),
        ("scram_time_s", StateField::F64(state.scram_time_s)),
        ("energy_at_scram_mws", StateField::F64(state.energy_at_scram_mws)),
        ("precursor_1_mw", StateField::F64(state.precursor_mw[0])),
        ("precursor_2_mw", StateField::F64(state.precursor_mw[1])),
        ("precursor_3_mw", StateField::F64(state.precursor_mw[2])),
        ("precursor_4_mw", StateField::F64(state.precursor_mw[3])),
        ("precursor_5_mw", StateField::F64(state.precursor_mw[4])),
        ("precursor_6_mw", StateField::F64(state.precursor_mw[5])),
        ("population_anchor_mw", StateField::F64(state.population_anchor_mw)),
        ("population_supplied_mw", StateField::F64(state.population_supplied_mw)),
        ("population_defect_mw", StateField::F64(state.population_defect_mw)),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("peak_power_mw", StateField::F64(state.peak_power_mw)),
        ("peak_energy_mws", StateField::F64(state.peak_energy_mws)),
        ("peak_time_s", StateField::F64(state.peak_time_s)),
        (
            "peak_fuel_temperature_k",
            StateField::F64(state.peak_fuel_temperature_k),
        ),
        ("prompt_supercritical", StateField::Bool(state.prompt_supercritical)),
        ("rod_moving", StateField::Bool(state.rod_moving)),
        ("rods_dropped", StateField::Bool(state.rods_dropped)),
    ])
}

fn reactor_ccore_state(
    state: &reactor_spert3_ccore::ReactorState,
) -> Result<serde_json::Value, String> {
    reactor_ccore_with(state, state_object)
}

pub(crate) fn reactor_ccore_published_is_finite(
    state: &reactor_spert3_ccore::ReactorState,
) -> Result<(), String> {
    reactor_ccore_with(state, fields_are_finite)
}

fn reactor_ccore_with<T>(
    state: &reactor_spert3_ccore::ReactorState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("power_mw", StateField::F64(state.power_mw)),
        ("energy_mws", StateField::F64(state.energy_mws)),
        ("power_rate_mw_per_s", StateField::F64(state.power_rate_mw_per_s)),
        ("hot_spot_plate_rise_k", StateField::F64(state.hot_spot_plate_rise_k)),
        ("hot_spot_plate_rate_k_per_s", StateField::F64(state.hot_spot_plate_rate_k_per_s)),
        ("hot_spot_water_rise_k", StateField::F64(state.hot_spot_water_rise_k)),
        ("plate_rise_k", StateField::F64(state.plate_rise_k)),
        ("water_rise_k", StateField::F64(state.water_rise_k)),
        ("reactivity_net_dollar", StateField::F64(state.reactivity_net_dollar)),
        ("initial_temperature_k", StateField::F64(state.initial_temperature_k)),
        ("system_pressure_psig", StateField::F64(state.system_pressure_psig)),
        ("precursor_1_mw", StateField::F64(state.precursor_mw[0])),
        ("precursor_2_mw", StateField::F64(state.precursor_mw[1])),
        ("precursor_3_mw", StateField::F64(state.precursor_mw[2])),
        ("precursor_4_mw", StateField::F64(state.precursor_mw[3])),
        ("precursor_5_mw", StateField::F64(state.precursor_mw[4])),
        ("precursor_6_mw", StateField::F64(state.precursor_mw[5])),
        ("population_anchor_mw", StateField::F64(state.population_anchor_mw)),
        ("feedback_integral_dollar_mws", StateField::F64(state.feedback_integral_dollar_mws)),
        ("swept_energy_mws", StateField::F64(state.swept_energy_mws)),
        ("hot_spot_swept_energy_mws", StateField::F64(state.hot_spot_swept_energy_mws)),
        ("population_defect_mw", StateField::F64(state.population_defect_mw)),
        ("energy_defect_mws", StateField::F64(state.energy_defect_mws)),
        ("hot_spot_energy_defect_mws", StateField::F64(state.hot_spot_energy_defect_mws)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("peak_power_mw", StateField::F64(state.peak_power_mw)),
        ("peak_energy_mws", StateField::F64(state.peak_energy_mws)),
        ("peak_time_s", StateField::F64(state.peak_time_s)),
        ("plate_rise_max_k", StateField::F64(state.plate_rise_max_k)),
        ("plate_rise_max_time_s", StateField::F64(state.plate_rise_max_time_s)),
        ("past_power_peak", StateField::Bool(state.past_power_peak)),
        ("past_plate_maximum", StateField::Bool(state.past_plate_maximum)),
        ("prompt_supercritical", StateField::Bool(state.prompt_supercritical)),
    ])
}

fn reactor_fuel_state(
    state: &reactor_spert3_fuel::ReactorState,
) -> Result<serde_json::Value, String> {
    reactor_fuel_with(state, state_object)
}

pub(crate) fn reactor_fuel_published_is_finite(
    state: &reactor_spert3_fuel::ReactorState,
) -> Result<(), String> {
    reactor_fuel_with(state, fields_are_finite)
}

fn reactor_fuel_with<T>(
    state: &reactor_spert3_fuel::ReactorState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("power_mw", StateField::F64(state.power_mw)),
        ("energy_mws", StateField::F64(state.energy_mws)),
        (
            "power_rate_mw_per_s",
            StateField::F64(state.power_rate_mw_per_s),
        ),
        (
            "fuel_temperature_rise_k",
            StateField::F64(state.fuel_temperature_rise_k),
        ),
        (
            "water_temperature_rise_k",
            StateField::F64(state.water_temperature_rise_k),
        ),
        (
            "reactivity_net_dollar",
            StateField::F64(state.reactivity_net_dollar),
        ),
        (
            "reactivity_inserted_now_dollar",
            StateField::F64(state.reactivity_inserted_now_dollar),
        ),
        (
            "inlet_temperature_k",
            StateField::F64(state.inlet_temperature_k),
        ),
        ("precursor_1_mw", StateField::F64(state.precursor_mw[0])),
        ("precursor_2_mw", StateField::F64(state.precursor_mw[1])),
        ("precursor_3_mw", StateField::F64(state.precursor_mw[2])),
        ("precursor_4_mw", StateField::F64(state.precursor_mw[3])),
        ("precursor_5_mw", StateField::F64(state.precursor_mw[4])),
        ("precursor_6_mw", StateField::F64(state.precursor_mw[5])),
        (
            "population_anchor_mw",
            StateField::F64(state.population_anchor_mw),
        ),
        (
            "insertion_integral_dollar_mws",
            StateField::F64(state.insertion_integral_dollar_mws),
        ),
        (
            "sweep_integral_k2",
            StateField::F64(state.sweep_integral_k2),
        ),
        (
            "population_defect_mw",
            StateField::F64(state.population_defect_mw),
        ),
        (
            "residual_conservation_error",
            StateField::F64(state.residual_conservation_error),
        ),
        ("peak_power_mw", StateField::F64(state.peak_power_mw)),
        ("peak_energy_mws", StateField::F64(state.peak_energy_mws)),
        ("peak_time_s", StateField::F64(state.peak_time_s)),
        (
            "past_first_maximum",
            StateField::Bool(state.past_first_maximum),
        ),
        (
            "prompt_supercritical",
            StateField::Bool(state.prompt_supercritical),
        ),
        ("rod_moving", StateField::Bool(state.rod_moving)),
    ])
}

/// The published fields of a card at rest, given to `finish`. `None` means the
/// pair is not a card.
fn with_rest_entries<T>(
    domain: &str,
    card: &str,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> Option<T> {
    match (domain, card) {
        ("materials", "ti64_lattice") => {
            let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
            let state = ti64_lattice::Ti64LatticeState::try_rest(&cell).expect("the column rests");
            Some(ti64_with(&state, finish))
        }
        ("energy", "grid_node") => {
            let node = grid_node::GridNode::copper_fuse_link();
            let state = grid_node::GridNodeState::try_rest(&node).expect("the fuse rests");
            Some(grid_with(&state, finish))
        }
        ("energy", "swing") => {
            let machine = swing::SwingMachine::synchronous_60hz();
            let state = swing::SwingState::try_rest(&machine).expect("the machine rests");
            Some(swing_with(&state, finish))
        }
        ("energy", "battery_sag") => {
            let pack = battery_sag::LiIonPack::pack_12s();
            let state = battery_sag::PackState::try_full(&pack).expect("the pack rests");
            Some(battery_with(&state, finish))
        }
        ("energy", "cell_18650pf") => {
            let cell = cell_18650pf::Cell18650Pf::cell();
            let state = cell_18650pf::CellState::try_full(&cell).expect("the cell rests");
            Some(cell_with(&state, finish))
        }
        ("energy", "cell_2170_m80") => {
            let state = cell_2170_m80::CellState::try_start(cell_2170(), 25.0, 0.0)
                .expect("the 2170 cell rests");
            Some(cell_2170_with(&state, finish))
        }
        ("energy", "thermal") => {
            let node = thermal::ThermalNode::avionics_cold_plate();
            let state = thermal::ThermalNodeState::try_rest(&node).expect("the plate rests");
            Some(thermal_with(&state, finish))
        }
        ("signal", "resonance") => {
            let oscillator = resonance::DynamicOscillator::mount_10hz();
            let state =
                resonance::OscillatorState::try_rest(&oscillator).expect("the mount rests");
            Some(resonance_with(&state, finish))
        }
        ("signal", "tesseract") => {
            let resonator = tesseract::Tesseract::drive_mode();
            let state = tesseract::TesseractState::try_rest(&resonator).expect("the drive rests");
            Some(tesseract_with(&state, finish))
        }
        ("materials", "hydraulic_fluid") => {
            let column = hydraulic_fluid::FluidColumn::mineral_oil();
            let state =
                hydraulic_fluid::FluidState::try_rest(&column).expect("the column rests");
            Some(hydraulic_with(&state, finish))
        }
        ("materials", "cable_stretch") => {
            let tendon = cable_stretch::Tendon::dyneema();
            let state = cable_stretch::TendonState::try_rest(&tendon).expect("the tendon rests");
            Some(cable_with(&state, finish))
        }
        ("energy", "generator_mmrtg_f1") => {
            let generator = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
            let state = generator_mmrtg_f1::GeneratorState::try_landing(&generator)
                .expect("the generator lands");
            Some(generator_with(&state, finish))
        }
        ("aerospace", "capsule_stardust") => {
            let capsule = capsule_stardust::CapsuleStardust::sample_return_capsule();
            let state = capsule_stardust::CapsuleState::try_entry_interface(&capsule)
                .expect("the capsule is at entry interface");
            Some(capsule_with(&state, finish))
        }
        ("aerospace", "capsule_insight") => {
            let capsule = capsule_insight::CapsuleInsight::entry_capsule();
            let state = capsule_insight::CapsuleState::try_entry_interface(&capsule)
                .expect("the capsule is at its entry state");
            Some(capsule_insight_with(&state, finish))
        }
        ("aerospace", "satellite_lageos1") => {
            let state = descriptor::satellite_example_state();
            Some(satellite_with(&state, finish))
        }
        ("aerospace", "solar_system") => {
            let state = descriptor::solar_system_example_state();
            Some(solar_system_with(&state, finish))
        }
        ("energy", "reactor_spert3") => {
            let reactor = reactor_spert3::ReactorSpert3::e_core_cold_startup();
            let state = reactor_spert3::ReactorState::try_cold_start(
                &reactor,
                reactor_spert3::COOLANT_TEMPERATURE_MIN_K,
            )
            .expect("the reactor starts cold");
            Some(reactor_with(&state, finish))
        }
        ("energy", "grid_gb") => {
            let card = grid_gb::GridGb::interconnector_loss();
            let state = grid_gb::GridState::try_start(
                &card,
                grid_gb::NOMINAL_FREQUENCY_HZ,
                grid_gb::INERTIA_MIN_MW_S,
                grid_gb::DEMAND_MIN_MW,
            )
            .expect("the area starts in balance");
            Some(grid_gb_with(&state, finish))
        }
        ("signal", "cantilever_eth") => {
            let beam = cantilever_eth::CantileverEth::first_bending_mode();
            // Rest at zero is below the card's floor: at rest at the ceiling.
            let state =
                cantilever_eth::CantileverState::try_phase(&beam, beam.amplitude_ceiling_m, 0.0)
                    .expect("the beam rests at its ceiling");
            Some(cantilever_with(&state, finish))
        }
        ("signal", "link_gothenburg_10010") => {
            let link = link_gothenburg_10010::LinkGothenburg10010::sublink_587();
            let state = link_gothenburg_10010::LinkState::try_new(
                &link,
                descriptor::LINK_EXAMPLE_DRY_PATH_LOSS_DB,
                0.0,
            )
            .expect("the link is dry");
            Some(link_with(&state, finish))
        }
        ("aerospace", "los_jwst_fine_guide") => {
            let card = los_jwst_fine_guide::LosJwstFineGuide::observatory();
            let state = los_jwst_fine_guide::LosState::try_lock(
                &card,
                0.0,
                0.0,
                los_jwst_fine_guide::RECOVERY_S,
            )
            .expect("the centre of the sensor is a lock point");
            Some(los_with(&state, finish))
        }
        ("energy", "cell_2170_model3") => {
            let cell = cell_2170_model3_type();
            let state = cell_2170_model3::CellState::try_rested(
                cell,
                cell_2170_model3::CELLS[0].cell_number,
                1.0,
                cell_2170_model3::CAPACITY_MAX_AH,
            )
            .expect("the physical 2170 cell rests");
            Some(cell_2170_model3_with(&state, finish))
        }
        ("materials", "creep_cmsx6") => {
            let alloy = creep_cmsx6::CreepCmsx6::at_980_c();
            let state = creep_cmsx6::CreepState::try_new(
                &alloy,
                descriptor::CREEP_EXAMPLE_STRESS_MPA,
                creep_cmsx6::CREEP_EXTENSION_MIN_PERCENT,
            )
            .expect("the test piece starts at the card's lowest extension");
            Some(creep_with(&state, finish))
        }
        ("energy", "reactor_triga_jsi") => {
            let reactor = reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode();
            let state = reactor_triga_jsi::ReactorState::try_start(
                &reactor,
                reactor.start_temperature_k_by_table[0].0,
                reactor_triga_jsi::ROD_TABLE_CORE[0],
            )
            .expect("the reactor starts at the low end of its first table's starting temperatures");
            Some(reactor_triga_with(&state, finish))
        }
        ("energy", "reactor_spert3_ccore") => {
            let reactor = reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients();
            let state = reactor_spert3_ccore::ReactorState::try_start(
                &reactor,
                reactor_spert3_ccore::INITIAL_TEMPERATURE_MIN_K,
                0.0,
            )
            .expect("the reactor starts at the lowest compared temperature");
            Some(reactor_ccore_with(&state, finish))
        }
        ("energy", "reactor_spert3_fuel") => {
            let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
            let state = reactor_spert3_fuel::ReactorState::try_start(
                &reactor,
                reactor.band_inlet_min_k[0],
            )
            .expect("the reactor starts in the cold band");
            Some(reactor_fuel_with(&state, finish))
        }
        ("aerospace", "crater_ryugu_sci") => {
            let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();
            let state = crater_ryugu_sci::CraterState::try_radius(
                &crater,
                crater_ryugu_sci::RADIUS_FLOOR_M,
            )
            .expect("the crater starts at its smallest radius");
            Some(crater_with(&state, finish))
        }
        ("materials", "tire_eve_r19_ice") => {
            let tire = tire_eve_r19_ice::TireEveR19Ice::on_rig_ice();
            let state = tire_eve_r19_ice::TireState::try_start(
                &tire,
                0.0,
                tire_eve_r19_ice::SLIP_SETTLE_S,
            )
            .expect("the wheel stands at the start of the track with its slip settled");
            Some(tire_with(&state, finish))
        }
        ("materials", "fingerpad_p02_glass") => {
            let pad = fingerpad_p02_glass::FingerPadP02Glass::on_glass();
            let state = fingerpad_p02_glass::FingerPadState::try_start(&pad, 0.0)
                .expect("the pad starts untwisted");
            Some(fingerpad_with(&state, finish))
        }
        ("energy", "accumulator_sbo450") => {
            let accumulator = accumulator_sbo450::AccumulatorSbo450::on_darmstadt_rig();
            let state = accumulator_sbo450::GasState::try_start(
                &accumulator,
                accumulator_sbo450::CHARGE_40_BAR_KG,
                6.0e6,
                293.15,
                293.55,
                0.0,
                0.0,
            )
            .expect("the gas starts at the fitted condition");
            Some(accumulator_with(&state, finish))
        }
        ("energy", "tokamak_jet_dte1") => {
            let tokamak = tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode();
            let state = tokamak_jet_dte1::TokamakState::try_start(&tokamak, 1.0e6, 0.0)
                .expect("the plasma starts");
            Some(tokamak_with(&state, finish))
        }
        _ => None,
    }
}

/// `true` when `key` is a name this card publishes, or the `_hex` sibling of
/// a published number. The names are the ones each card's `*_with` list
/// already publishes. `None` means the pair is not a card.
pub(crate) fn state_field_is_published(domain: &str, card: &str, key: &str) -> Option<bool> {
    with_rest_entries(domain, card, |entries| field_is_listed(entries, key))
}

/// What a card's published state holds under a plain name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum QuantityKind {
    Number,
    Vector3,
    Flag,
}

/// The kind of the published field `name`. The outer `None` means the pair is
/// not a card. The inner `None` means the card publishes no field of that name.
pub(crate) fn state_field_kind(
    domain: &str,
    card: &str,
    name: &str,
) -> Option<Option<QuantityKind>> {
    with_rest_entries(domain, card, |entries| {
        entries
            .iter()
            .find(|(entry, _)| *entry == name)
            .map(|(_, field)| match field {
                StateField::F64(_) => QuantityKind::Number,
                StateField::F64x3(_) => QuantityKind::Vector3,
                StateField::Bool(_) => QuantityKind::Flag,
            })
    })
}

/// Every plain name a card's published state holds, with its kind, in the
/// order the card lists them. `None` for a pair that is not a card.
pub(crate) fn state_fields(domain: &str, card: &str) -> Option<Vec<(String, QuantityKind)>> {
    with_rest_entries(domain, card, |entries| {
        entries
            .iter()
            .map(|(name, field)| {
                let kind = match field {
                    StateField::F64(_) => QuantityKind::Number,
                    StateField::F64x3(_) => QuantityKind::Vector3,
                    StateField::Bool(_) => QuantityKind::Flag,
                };
                (name.to_string(), kind)
            })
            .collect()
    })
}

/// A listed name, or the `_hex` sibling of a listed number. A flag has no hex sibling.
fn field_is_listed(entries: &[(&str, StateField)], key: &str) -> bool {
    let hex_of = key.strip_suffix("_hex").filter(|base| {
        !base.is_empty() && entries.iter().any(|(name, _)| *name == *base)
    });
    match hex_of {
        Some(base) => entries
            .iter()
            .any(|(name, field)| *name == base && !matches!(field, StateField::Bool(_))),
        None => entries.iter().any(|(name, _)| *name == key),
    }
}

fn sha256_seal(frame: LastStateFrame64) -> String {
    let frame_bytes: [u8; 64] = frame.to_bytes();
    digest_hex(&frame_bytes)
}

fn digest_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Exact JSON of every response field except `receipt_sha256`.
///
/// The wire body is this document with `,"receipt_sha256":"<hex>"}` in place of
/// the closing brace. The receipt is SHA-256 of this document, not of the frame.
#[derive(Serialize)]
struct ReceiptDocument {
    inputs: serde_json::Value,
    engine_version: String,
    #[serde(serialize_with = "serialize_state_value")]
    state: serde_json::Value,
    #[serde(serialize_with = "serialize_exact_f64")]
    residual_conservation_error: f64,
    sha256_seal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    chain: Option<serde_json::Value>,
}

fn receipt_document(response: &VerifyPhysicsResponse) -> String {
    serde_json::to_string(&ReceiptDocument {
        inputs: response.inputs.clone(),
        engine_version: response.engine_version.clone(),
        state: response.state.clone(),
        residual_conservation_error: response.residual_conservation_error,
        sha256_seal: response.sha256_seal.clone(),
        chain: response.chain.clone(),
    })
    .expect("receipt document encodes")
}

fn append_receipt(document: &str, receipt: &str) -> String {
    debug_assert!(document.ends_with('}'));
    let mut wire = String::with_capacity(document.len() + receipt.len() + 22);
    wire.push_str(&document[..document.len() - 1]);
    wire.push_str(",\"receipt_sha256\":\"");
    wire.push_str(receipt);
    wire.push_str("\"}");
    wire
}

fn seal_response(
    mut response: VerifyPhysicsResponse,
) -> Result<VerifyPhysicsResponse, (StatusCode, String)> {
    let document = receipt_document(&response);
    response.receipt_sha256 = digest_hex(document.as_bytes());
    let wire = serde_json::to_string(&response).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Physical Violation: the response could not be encoded.".to_string(),
        )
    })?;
    if wire != append_receipt(&document, &response.receipt_sha256) {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Physical Violation: the receipt does not cover the response bytes.".to_string(),
        ));
    }
    Ok(response)
}

impl Ti64Call {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("materials", "ti64_lattice"),
        )?;
        // The force is held to its published bound after the load rule, so a
        // force the load rule refuses is refused in the load rule's words.
        enforce_published_fields("materials", "ti64_lattice", Published::Step, params, |name| {
            name != "force_n"
        })?;
        let position_m = required_vec3(params, "position_m")?;
        let velocity_m_s = required_vec3(params, "velocity_m_s")?;
        // The start carries its clock, its held load and its ledger, so a
        // run handed back its own returned state is weighed as one run.
        let start = ti64_lattice::Ti64Start {
            t_s: required_f64(params, "start_t_s")?,
            position_m,
            velocity_m_s,
            applied_force_n: required_vec3(params, "start_force_n")?,
            temperature_k: required_f64(params, "start_temperature_k")?,
            energy_anchor_j: required_f64(params, "start_energy_anchor_j")?,
            boundary_work_j: required_f64(params, "start_boundary_work_j")?,
        };
        let force_n = required_vec3(params, "force_n")?;
        let torque_nm = required_vec3(params, "torque_nm")?;
        let temperature_k = required_f64(params, "temperature_k")?;
        let dt_s = required_f64(params, "dt_s")?;
        let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
        let omega = cell.max_natural_frequency_rad_s();
        card_clock(dt_s, Some(omega), cell.floor_dt_s(), cell.gate_dt_s(), "ti64_lattice")?;
        enforce_force_nyquist(force_n, dt_s, &cell)?;
        enforce_published_fields("materials", "ti64_lattice", Published::Step, params, |name| {
            name == "force_n"
        })?;
        let state =
            ti64_lattice::Ti64LatticeState::try_continue(&cell, start).map_err(|reject| match reject {
                ti64_lattice::StepReject::NonFiniteState | ti64_lattice::StepReject::InvalidCell => {
                    phase_not_valid("ti64_lattice")
                }
                other => ti64_reject(other),
            })?;
        Ok(Self {
            cell,
            state,
            load: ti64_lattice::Ti64Load {
                force_n,
                torque_nm,
                temperature_k,
            },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("materials", "ti64_lattice"))?;
        enforce_published_fields("materials", "ti64_lattice", Published::Load, params, |name| {
            name != "force_n"
        })?;
        let force_n = required_vec3(params, "force_n")?;
        let torque_nm = required_vec3(params, "torque_nm")?;
        let temperature_k = required_f64(params, "temperature_k")?;
        enforce_force_nyquist(force_n, self.dt_s, &self.cell)?;
        enforce_published_fields("materials", "ti64_lattice", Published::Load, params, |name| {
            name == "force_n"
        })?;
        self.load = ti64_lattice::Ti64Load {
            force_n,
            torque_nm,
            temperature_k,
        };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        ti64_lattice::step(&self.cell, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = ti64_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: ti64_state(&self.state)?,
        })
    }
}

impl GridCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names("energy", "grid_node"))?;
        enforce_published("energy", "grid_node", Published::Step, params)?;
        let current_a = required_f64(params, "current_a")?;
        let voltage_v = required_f64(params, "voltage_v")?;
        let temperature_k = required_f64(params, "temperature_k")?;
        let upstream_voltage_v = required_f64(params, "upstream_voltage_v")?;
        let load_current_a = required_f64(params, "load_current_a")?;
        let ambient_k = required_f64(params, "ambient_k")?;
        let dt_s = required_f64(params, "dt_s")?;
        let node = grid_node::GridNode::copper_fuse_link();
        let omega = node.natural_frequency_rad_s();
        card_clock(dt_s, Some(omega), node.floor_dt_s(), node.gate_dt_s(), "grid_node")?;
        if temperature_k >= grid_node::COPPER_MELT_K {
            return Err(
                "Physical Failure: copper is at or above the melt point (1357.77 K). A molten state is not sealed.".to_string(),
            );
        }
        let state = grid_node::GridNodeState::try_phase(&node, current_a, voltage_v, temperature_k)
            .ok_or_else(|| phase_not_valid("grid_node"))?;
        Ok(Self {
            node,
            state,
            load: grid_node::GridLoad {
                upstream_voltage_v,
                load_current_a,
                ambient_k,
            },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "grid_node"))?;
        enforce_published("energy", "grid_node", Published::Load, params)?;
        let upstream_voltage_v = required_f64(params, "upstream_voltage_v")?;
        let load_current_a = required_f64(params, "load_current_a")?;
        let ambient_k = required_f64(params, "ambient_k")?;
        self.load = grid_node::GridLoad {
            upstream_voltage_v,
            load_current_a,
            ambient_k,
        };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        grid_node::step(&self.node, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = match reject {
                    grid_node::StepReject::CopperMelt => {
                        "Physical Failure: copper reached the melt point (1357.77 K). A molten state is not sealed.".to_string()
                    }
                    grid_node::StepReject::SpontaneousGeneration => {
                        "Physical Failure: the grid step created energy (SpontaneousGeneration). That state is not sealed.".to_string()
                    }
                    grid_node::StepReject::FirstLawBreach => {
                        "Physical Failure: the grid step destroyed energy (FirstLawBreach). That state is not sealed.".to_string()
                    }
                    grid_node::StepReject::CurrentOutsideCard => format!(
                        "Physical Failure: the grid step landed with |current_a| above {} A, the inclusive maximum of the grid_node card (CurrentOutsideCard). That state is not sealed.",
                        descriptor::grid_melt_current_a(&self.node)
                    ),
                    grid_node::StepReject::VoltageOutsideCard => format!(
                        "Physical Failure: the grid step landed with |voltage_v| above {} V, the inclusive maximum of the grid_node card (VoltageOutsideCard). That state is not sealed.",
                        descriptor::grid_melt_voltage_v(&self.node)
                    ),
                    other => refused("grid_node", other),
                };
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: grid_state(&self.state)?,
        })
    }
}

impl ThermalCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names("energy", "thermal"))?;
        enforce_published("energy", "thermal", Published::Step, params)?;
        let temperature_k = required_f64(params, "temperature_k")?;
        let heat_power_w = required_f64(params, "heat_power_w")?;
        let ambient_k = required_f64(params, "ambient_k")?;
        let dt_s = required_f64(params, "dt_s")?;
        let node = thermal::ThermalNode::avionics_cold_plate();
        card_clock(dt_s, Some(node.pole_rad_s()), node.floor_dt_s(), node.gate_dt_s(), "thermal")?;
        if !thermal_on_card(temperature_k) || !thermal_on_card(ambient_k) {
            return Err(unsealed(
                "plate or bath temperature is outside 233.15 K to 398.15 K (TemperatureOutsideCard).",
            ));
        }
        let state = thermal::ThermalNodeState::try_phase(&node, temperature_k)
            .ok_or_else(|| phase_not_valid("thermal"))?;
        Ok(Self {
            node,
            state,
            load: thermal::ThermalLoad {
                heat_power_w,
                ambient_k,
            },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "thermal"))?;
        enforce_published("energy", "thermal", Published::Load, params)?;
        let heat_power_w = required_f64(params, "heat_power_w")?;
        let ambient_k = required_f64(params, "ambient_k")?;
        if !thermal_on_card(ambient_k) {
            return Err(unsealed(
                "plate or bath temperature is outside 233.15 K to 398.15 K (TemperatureOutsideCard).",
            ));
        }
        self.load = thermal::ThermalLoad {
            heat_power_w,
            ambient_k,
        };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        thermal::step(&self.node, &mut self.state, frame, self.load, self.dt_s).map_err(|reject| {
            let detail = thermal_reject(reject);
            StepFailure::from_reject(reject, detail)
        })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: thermal_state(&self.state)?,
        })
    }
}

impl ResonanceCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("signal", "resonance"),
        )?;
        enforce_published("signal", "resonance", Published::Step, params)?;
        let displacement_m = required_f64(params, "displacement_m")?;
        let velocity_m_s = required_f64(params, "velocity_m_s")?;
        let t_s = required_f64(params, "t_s")?;
        let energy_anchor_j = required_f64(params, "energy_anchor_j")?;
        let boundary_work_j = required_f64(params, "boundary_work_j")?;
        let accel_m_s2 = required_f64(params, "accel_m_s2")?;
        let dt_s = required_f64(params, "dt_s")?;
        let oscillator = resonance::DynamicOscillator::mount_10hz();
        card_clock(
            dt_s,
            Some(oscillator.omega_rad_s),
            oscillator.floor_dt_s(),
            oscillator.gate_dt_s(),
            "resonance",
        )?;
        enforce_abs_nyquist(
            "accel_m_s2",
            accel_m_s2,
            dt_s,
            oscillator.accel_impulse_limit_m_s(),
            oscillator.force_nyquist_m_s2(dt_s),
        )?;
        // The ledger runs across continued requests: a landed state handed
        // back with its own time, anchor and booked work goes on with the
        // defect it had. A stated ledger the card could not have written is
        // refused by name.
        let state = resonance::OscillatorState::try_continue(
            &oscillator,
            displacement_m,
            velocity_m_s,
            t_s,
            energy_anchor_j,
            boundary_work_j,
        )
        .map_err(|reject| match reject {
            resonance::StepReject::NonFiniteState => phase_not_valid("resonance"),
            other => resonance_reject(other),
        })?;
        Ok(Self {
            oscillator,
            state,
            load: resonance::OscillatorLoad { accel_m_s2 },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("signal", "resonance"))?;
        enforce_published("signal", "resonance", Published::Load, params)?;
        let accel_m_s2 = required_f64(params, "accel_m_s2")?;
        enforce_abs_nyquist(
            "accel_m_s2",
            accel_m_s2,
            self.dt_s,
            self.oscillator.accel_impulse_limit_m_s(),
            self.oscillator.force_nyquist_m_s2(self.dt_s),
        )?;
        self.load = resonance::OscillatorLoad { accel_m_s2 };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        resonance::step(
            &self.oscillator,
            &mut self.state,
            frame,
            self.load,
            self.dt_s,
        )
        .map_err(|reject| {
            let detail = resonance_reject(reject);
            StepFailure::from_reject(reject, detail)
        })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: resonance_state(&self.state)?,
        })
    }
}

fn swing_dispatch_on_card(p_mech_pu: f64) -> Result<(), String> {
    if p_mech_pu.abs() > swing::DISPATCH_LIMIT_PU {
        return Err(unsealed(SWING_DISPATCH_OUTSIDE_CARD));
    }
    Ok(())
}

impl SwingCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names("energy", "swing"))?;
        enforce_published("energy", "swing", Published::Step, params)?;
        let delta_rad = required_f64(params, "delta_rad")?;
        let delta_omega_rad_s = required_f64(params, "delta_omega_rad_s")?;
        let pll_err_rad = required_f64(params, "pll_err_rad")?;
        let p_mech_pu = required_f64(params, "p_mech_pu")?;
        let dt_s = required_f64(params, "dt_s")?;
        let machine = swing::SwingMachine::synchronous_60hz();
        card_clock(
            dt_s,
            Some(machine.omega_electrical_rad_s),
            machine.floor_dt_s(),
            machine.gate_dt_s(),
            "swing",
        )?;
        if delta_rad.abs() >= std::f64::consts::FRAC_PI_2 {
            return Err(unsealed(
                "the rotor angle is at or beyond +/- pi/2 (LossOfSynchronism).",
            ));
        }
        if delta_omega_rad_s.abs() > swing::SPEED_DEVIATION_LIMIT_RAD_S {
            return Err(unsealed(SWING_SPEED_OUTSIDE_CARD));
        }
        swing_dispatch_on_card(p_mech_pu)?;
        let state =
            swing::SwingState::try_dispatched(
                &machine,
                delta_rad,
                delta_omega_rad_s,
                pll_err_rad,
                p_mech_pu,
            )
            .ok_or_else(|| phase_not_valid("swing"))?;
        Ok(Self {
            machine,
            state,
            load: swing::SwingLoad { p_mech_pu },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "swing"))?;
        enforce_published("energy", "swing", Published::Load, params)?;
        let p_mech_pu = required_f64(params, "p_mech_pu")?;
        swing_dispatch_on_card(p_mech_pu)?;
        self.load = swing::SwingLoad { p_mech_pu };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        swing::step(&self.machine, &mut self.state, frame, self.load, self.dt_s).map_err(|reject| {
            let detail = swing_reject(reject);
            StepFailure::from_reject(reject, detail)
        })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: swing_state(&self.state)?,
        })
    }
}

impl TesseractCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names("signal", "tesseract"))?;
        enforce_published("signal", "tesseract", Published::Step, params)?;
        let displacement_m = required_f64(params, "displacement_m")?;
        let velocity_m_s = required_f64(params, "velocity_m_s")?;
        // The ledger the run carried to this state. A start without it is
        // refused: a state handed back must not begin its ledger again.
        let energy_defect_j = required_f64(params, "energy_defect_j")?;
        let accel_m_s2 = required_f64(params, "accel_m_s2")?;
        let omega_ext_rad_s = required_f64(params, "omega_ext_rad_s")?;
        let sense_velocity_m_s = required_f64(params, "sense_velocity_m_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let resonator = tesseract::Tesseract::drive_mode();
        card_clock(
            dt_s,
            Some(resonator.omega_rad_s),
            resonator.floor_dt_s(),
            resonator.gate_dt_s(),
            "tesseract",
        )?;
        enforce_abs_nyquist(
            "accel_m_s2",
            accel_m_s2,
            dt_s,
            resonator.accel_impulse_limit_m_s(),
            resonator.force_nyquist_m_s2(dt_s),
        )?;
        let state = tesseract::TesseractState::try_continue(
            &resonator,
            displacement_m,
            velocity_m_s,
            energy_defect_j,
        )
        .ok_or_else(|| phase_not_valid("tesseract"))?;
        Ok(Self {
            resonator,
            state,
            load: tesseract::TesseractLoad {
                accel_m_s2,
                omega_ext_rad_s,
                sense_velocity_m_s,
            },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("signal", "tesseract"))?;
        enforce_published("signal", "tesseract", Published::Load, params)?;
        let accel_m_s2 = required_f64(params, "accel_m_s2")?;
        let omega_ext_rad_s = required_f64(params, "omega_ext_rad_s")?;
        let sense_velocity_m_s = required_f64(params, "sense_velocity_m_s")?;
        enforce_abs_nyquist(
            "accel_m_s2",
            accel_m_s2,
            self.dt_s,
            self.resonator.accel_impulse_limit_m_s(),
            self.resonator.force_nyquist_m_s2(self.dt_s),
        )?;
        self.load = tesseract::TesseractLoad {
            accel_m_s2,
            omega_ext_rad_s,
            sense_velocity_m_s,
        };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        tesseract::step(
            &self.resonator,
            &mut self.state,
            frame,
            self.load,
            self.dt_s,
        )
        .map_err(|reject| {
            let detail = tesseract_reject(reject);
            StepFailure::from_reject(reject, detail)
        })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: tesseract_state(&self.state)?,
        })
    }
}

impl HydraulicCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("materials", "hydraulic_fluid"),
        )?;
        // The flow's own bound is applied after the stroke rule, in the
        // kernel's order: FlowAboveNyquist, then FlowAboveAcoustic.
        enforce_published_fields("materials", "hydraulic_fluid", Published::Step, params, |name| {
            name != "flow_m3_s"
        })?;
        let pressure_pa = required_f64(params, "pressure_pa")?;
        let flow_m3_s = required_f64(params, "flow_m3_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let column = hydraulic_fluid::FluidColumn::mineral_oil();
        card_clock(
            dt_s,
            Some(column.acoustic_omega_rad_s()),
            column.floor_dt_s(),
            column.gate_dt_s(),
            "hydraulic_fluid",
        )?;
        enforce_abs_nyquist(
            "flow_m3_s",
            flow_m3_s,
            dt_s,
            column.stroke_volume_m3(),
            column.flow_nyquist_m3_s(dt_s),
        )?;
        enforce_published_fields("materials", "hydraulic_fluid", Published::Step, params, |name| {
            name == "flow_m3_s"
        })?;
        if pressure_pa < hydraulic_fluid::VAPOR_PRESSURE_PA {
            return Err(unsealed("pressure is below the vapor floor (Cavitation)."));
        }
        let state = hydraulic_fluid::FluidState::try_pressure(&column, pressure_pa)
            .ok_or_else(|| phase_not_valid("hydraulic_fluid"))?;
        Ok(Self {
            column,
            state,
            load: hydraulic_fluid::FluidLoad { flow_m3_s },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("materials", "hydraulic_fluid"),
        )?;
        let flow_m3_s = required_f64(params, "flow_m3_s")?;
        enforce_abs_nyquist(
            "flow_m3_s",
            flow_m3_s,
            self.dt_s,
            self.column.stroke_volume_m3(),
            self.column.flow_nyquist_m3_s(self.dt_s),
        )?;
        enforce_published("materials", "hydraulic_fluid", Published::Load, params)?;
        self.load = hydraulic_fluid::FluidLoad { flow_m3_s };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        hydraulic_fluid::step(&self.column, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = hydraulic_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: hydraulic_state(&self.state)?,
        })
    }
}

impl CableCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("materials", "cable_stretch"),
        )?;
        enforce_published("materials", "cable_stretch", Published::Step, params)?;
        let elongation_m = required_f64(params, "elongation_m")?;
        let velocity_m_s = required_f64(params, "velocity_m_s")?;
        let force_n = required_f64(params, "force_n")?;
        let dt_s = required_f64(params, "dt_s")?;
        let tendon = cable_stretch::Tendon::dyneema();
        card_clock(
            dt_s,
            Some(tendon.omega_rad_s),
            tendon.floor_dt_s(),
            tendon.gate_dt_s(),
            "cable_stretch",
        )?;
        enforce_abs_nyquist(
            "force_n",
            force_n,
            dt_s,
            tendon.force_impulse_limit_n_s(),
            tendon.force_nyquist_n(dt_s),
        )?;
        let tension_n = tendon.tension_n(elongation_m, velocity_m_s);
        if !tension_n.is_finite() {
            return Err("Physical Violation: cable tension is not finite.".to_string());
        }
        if tension_n < 0.0 {
            return Err(unsealed("the braid is slack (Slack)."));
        }
        if tension_n >= tendon.ultimate_tension_n {
            return Err(unsealed(
                "the braid reached ultimate tension (TensileFailure).",
            ));
        }
        let state = cable_stretch::TendonState::try_phase(&tendon, elongation_m, velocity_m_s)
            .ok_or_else(|| phase_not_valid("cable_stretch"))?;
        Ok(Self {
            tendon,
            state,
            load: cable_stretch::TendonLoad { force_n },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("materials", "cable_stretch"),
        )?;
        enforce_published("materials", "cable_stretch", Published::Load, params)?;
        let force_n = required_f64(params, "force_n")?;
        enforce_abs_nyquist(
            "force_n",
            force_n,
            self.dt_s,
            self.tendon.force_impulse_limit_n_s(),
            self.tendon.force_nyquist_n(self.dt_s),
        )?;
        self.load = cable_stretch::TendonLoad { force_n };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        cable_stretch::step(&self.tendon, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = cable_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: cable_state(&self.state)?,
        })
    }
}

impl BatteryCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names("energy", "battery_sag"))?;
        enforce_published("energy", "battery_sag", Published::Step, params)?;
        let soc = required_f64(params, "soc")?;
        let current_a = required_f64(params, "current_a")?;
        let dt_s = required_f64(params, "dt_s")?;
        let pack = battery_sag::LiIonPack::pack_12s();
        card_clock(dt_s, Some(pack.pole_rad_s()), pack.floor_dt_s(), pack.gate_dt_s(), "battery_sag")?;
        battery_nyquist(&pack, current_a, dt_s)?;
        if !(soc > 0.0 && soc <= 1.0) {
            return Err(unsealed(
                "state of charge is outside (0, 1] (StateOfChargeOutsideCard).",
            ));
        }
        let state = battery_sag::PackState::try_soc(&pack, soc)
            .ok_or_else(|| phase_not_valid("battery_sag"))?;
        Ok(Self {
            pack,
            state,
            load: battery_sag::PackLoad { current_a },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "battery_sag"))?;
        enforce_published("energy", "battery_sag", Published::Load, params)?;
        let current_a = required_f64(params, "current_a")?;
        battery_nyquist(&self.pack, current_a, self.dt_s)?;
        self.load = battery_sag::PackLoad { current_a };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        battery_sag::step(&self.pack, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = battery_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: battery_state(&self.state)?,
        })
    }
}

impl CellCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "cell_18650pf"),
        )?;
        enforce_published("energy", "cell_18650pf", Published::Step, params)?;
        let soc = required_f64(params, "soc")?;
        // The held current of the 1C discharge this run belongs to: for a run
        // from full charge its own current, for a continuation the earlier
        // run's. A state below full charge is taken only with it.
        let held_current_a = required_f64(params, "held_current_a")?;
        let current_a = required_f64(params, "current_a")?;
        let dt_s = required_f64(params, "dt_s")?;
        let cell = cell_18650pf::Cell18650Pf::cell();
        // The cell has no pole. Its ceiling is the time to cross one table
        // interval at 3.0 A.
        card_clock(dt_s, None, cell.floor_dt_s(), cell.gate_dt_s(), "cell_18650pf")?;
        if !(soc >= cell_18650pf::SOC_MIN && soc <= 1.0) {
            return Err(unsealed(
                "state of charge left [0.03, 1] (StateOfChargeOutsideCard).",
            ));
        }
        if !cell_18650pf::current_was_compared(current_a) {
            return Err(cell_current_not_compared());
        }
        if !cell_18650pf::current_was_compared(held_current_a) {
            return Err(cell_current_not_compared());
        }
        if !cell_18650pf::held_current_within_band(current_a, held_current_a) {
            return Err(cell_current_changed(current_a, held_current_a));
        }
        let state = cell_18650pf::CellState::try_continued(&cell, soc, held_current_a)
            .ok_or_else(|| phase_not_valid("cell_18650pf"))?;
        Ok(Self {
            cell,
            state,
            load: cell_18650pf::CellLoad { current_a },
            dt_s,
            held_current_a,
        })
    }

    /// The load of a segment. It is held to the card's range, and then to
    /// the current the run was opened with, within the card's band: a run is
    /// one held current, and the card's step refuses another once the first
    /// step has landed. `run_trajectory` reads every segment through here
    /// before the first step, so a run that would change its current is
    /// refused whole and nothing of it is sealed.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "cell_18650pf"))?;
        enforce_published("energy", "cell_18650pf", Published::Load, params)?;
        let current_a = required_f64(params, "current_a")?;
        if !cell_18650pf::current_was_compared(current_a) {
            return Err(cell_current_not_compared());
        }
        if !cell_18650pf::held_current_within_band(current_a, self.held_current_a) {
            return Err(cell_current_changed(current_a, self.held_current_a));
        }
        self.load = cell_18650pf::CellLoad { current_a };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        cell_18650pf::step(&self.cell, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = cell_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: cell_state(&self.state)?,
        })
    }
}

impl Cell2170Call {
    /// The current and the case temperature of a step or of a segment, held
    /// to the card's own bounds and to the bands of the run's block.
    fn read_load(
        params: &serde_json::Value,
        block: usize,
    ) -> Result<cell_2170_m80::CellLoad, String> {
        let current_a = required_f64(params, "current_a")?;
        let case_temperature_c = required_f64(params, "case_temperature_c")?;
        let cell = cell_2170();
        if !(current_a >= -cell.current_discharge_max_a && current_a <= cell.current_charge_max_a)
        {
            return Err(unsealed(CELL_2170_CURRENT_OUTSIDE_CARD));
        }
        if !(case_temperature_c >= cell.case_temperature_min_c
            && case_temperature_c <= cell.case_temperature_max_c)
        {
            return Err(unsealed(CELL_2170_CASE_OUTSIDE_CARD));
        }
        if !(case_temperature_c >= cell.block_case_min_c[block]
            && case_temperature_c <= cell.block_case_max_c[block])
        {
            return Err(cell_2170_case_outside_block(case_temperature_c, block));
        }
        if !(current_a >= -cell.block_current_discharge_max_a[block]
            && current_a <= cell.block_current_charge_max_a[block])
        {
            return Err(cell_2170_current_outside_block(current_a, block));
        }
        Ok(cell_2170_m80::CellLoad {
            current_a,
            case_temperature_c,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "cell_2170_m80"),
        )?;
        let cell = cell_2170();
        // A block the comparison showed wrong is named before the published
        // bounds, which no longer reach it.
        let named = params.get("block_chamber_c").and_then(serde_json::Value::as_f64);
        if let Some(block_chamber_c) = named {
            if cell.block_of(block_chamber_c).is_some()
                && cell.served_block_of(block_chamber_c).is_none()
            {
                return Err(cell_2170_block_shown_wrong(block_chamber_c));
            }
        }
        // A start past an end of its block's tables by no more than the
        // rounding allowance of a start, as a run that ended on the end can
        // return it, is taken; the published bounds of the charge are the
        // ends themselves, so that field is held to the block's tables below.
        let charge = params.get("charge_removed_ah").and_then(serde_json::Value::as_f64);
        let on_tables = match (named.and_then(|chamber| cell.served_block_of(chamber)), charge) {
            (Some(block), Some(charge)) => cell_2170_start_on_tables(charge, block),
            _ => false,
        };
        enforce_published_fields("energy", "cell_2170_m80", Published::Step, params, |name| {
            !(on_tables && name == "charge_removed_ah")
        })?;
        let block_chamber_c = required_f64(params, "block_chamber_c")?;
        let charge_removed_ah = required_f64(params, "charge_removed_ah")?;
        let block = cell
            .served_block_of(block_chamber_c)
            .ok_or_else(|| cell_2170_block_not_recognized(block_chamber_c))?;
        let load = Self::read_load(params, block)?;
        let dt_s = required_f64(params, "dt_s")?;
        // The cell has no pole. Its floor and its ceiling are the same one
        // second, so the clock accepts one step and no other.
        card_clock(dt_s, None, cell.floor_dt_s(), cell.gate_dt_s(), "cell_2170_m80")?;
        if !cell_2170_start_on_tables(charge_removed_ah, block) {
            return Err(cell_2170_charge_outside_block(charge_removed_ah, block));
        }
        let state = cell_2170_m80::CellState::try_start(cell, block_chamber_c, charge_removed_ah)
            .ok_or_else(|| phase_not_valid("cell_2170_m80"))?;
        Ok(Self {
            cell,
            state,
            load,
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "cell_2170_m80"))?;
        enforce_published("energy", "cell_2170_m80", Published::Load, params)?;
        let block = self
            .cell
            .served_block_of(self.state.block_chamber_c)
            .ok_or_else(|| cell_2170_block_not_recognized(self.state.block_chamber_c))?;
        self.load = Self::read_load(params, block)?;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        cell_2170_m80::step(self.cell, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = cell_2170_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: cell_2170_state(&self.state)?,
        })
    }
}

impl CraterCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("aerospace", "crater_ryugu_sci"),
        )?;
        enforce_published("aerospace", "crater_ryugu_sci", Published::Step, params)?;
        let radius_m = required_f64(params, "radius_m")?;
        let dt_s = required_f64(params, "dt_s")?;
        let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();
        // The step ceiling is a locked constant of the card, so the clock
        // applies no `π/ω` of its own.
        card_clock(dt_s, None, crater.floor_dt_s(), crater.gate_dt_s(), "crater_ryugu_sci")?;
        // The speed of the wall is the one this impact gives it at this
        // radius. The caller does not set it.
        let state = crater_ryugu_sci::CraterState::try_radius(&crater, radius_m)
            .ok_or_else(|| phase_not_valid("crater_ryugu_sci"))?;
        Ok(Self {
            crater,
            state,
            load: crater_ryugu_sci::CraterLoad {},
            dt_s,
        })
    }

    /// The card has no load. The object must be there and must be empty.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("aerospace", "crater_ryugu_sci"),
        )?;
        enforce_published("aerospace", "crater_ryugu_sci", Published::Load, params)?;
        self.load = crater_ryugu_sci::CraterLoad {};
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        crater_ryugu_sci::step(&self.crater, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = crater_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: crater_state(&self.state)?,
        })
    }
}

impl Cell2170Model3Call {
    /// The current and the case temperature of a step or of a segment, held
    /// to the card's own bounds.
    fn read_load(params: &serde_json::Value) -> Result<cell_2170_model3::CellLoad, String> {
        let current_a = required_f64(params, "current_a")?;
        let case_temperature_c = required_f64(params, "case_temperature_c")?;
        let cell = cell_2170_model3_type();
        if !(current_a >= -cell.charge_current_max_a && current_a <= cell.discharge_current_max_a)
        {
            return Err(unsealed(CELL_2170_MODEL3_CURRENT_OUTSIDE_CARD));
        }
        if !(case_temperature_c >= cell.case_temperature_min_c
            && case_temperature_c <= cell.case_temperature_max_c)
        {
            return Err(unsealed(CELL_2170_MODEL3_CASE_OUTSIDE_CARD));
        }
        Ok(cell_2170_model3::CellLoad {
            current_a,
            case_temperature_c,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "cell_2170_model3"),
        )?;
        enforce_published("energy", "cell_2170_model3", Published::Step, params)?;
        let cell_number = required_f64(params, "cell_number")?;
        let state_of_charge = required_f64(params, "state_of_charge")?;
        let capacity_ah = required_f64(params, "capacity_ah")?;
        // The span of case temperature is accumulated over the run, so a start carries the
        // least and the most of the run so far; nothing defaults.
        let least_c = required_f64(params, "case_temperature_least_c")?;
        let most_c = required_f64(params, "case_temperature_most_c")?;
        let load = Self::read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let cell = cell_2170_model3_type();
        // The cell has no pole. Its floor and its ceiling are locked
        // constants of the card, so the clock applies no `π/ω` of its own.
        card_clock(
            dt_s,
            None,
            cell.floor_dt_s(),
            cell.gate_dt_s(),
            "cell_2170_model3",
        )?;
        // A number between the smallest and the largest cell number that
        // names no cell is inside the published bounds and is refused here.
        if cell.cell_of(cell_number).is_none() {
            return Err(cell_2170_model3_cell_not_recognized(cell_number));
        }
        let state =
            cell_2170_model3::CellState::try_rested(cell, cell_number, state_of_charge, capacity_ah)
                .ok_or_else(|| phase_not_valid("cell_2170_model3"))?;
        if !(least_c <= most_c && most_c - least_c <= cell.case_temperature_span_max_k) {
            return Err(cell_2170_model3_span_above_card(least_c, most_c, "the start carries"));
        }
        let state = state
            .with_case_span(cell, least_c, most_c)
            .ok_or_else(|| phase_not_valid("cell_2170_model3"))?;
        let planned_least_c = least_c.min(load.case_temperature_c);
        let planned_most_c = most_c.max(load.case_temperature_c);
        if !(planned_most_c - planned_least_c <= cell.case_temperature_span_max_k) {
            return Err(cell_2170_model3_span_above_card(
                planned_least_c,
                planned_most_c,
                "the start's carried span and the run's segments span",
            ));
        }
        if !cell.capacity_compared_at(capacity_ah, load.case_temperature_c) {
            return Err(cell_2170_model3_capacity_not_compared(
                capacity_ah,
                load.case_temperature_c,
            ));
        }
        Ok(Self {
            cell,
            state,
            load,
            dt_s,
            planned_least_c,
            planned_most_c,
        })
    }

    /// A later segment's load: its own bounds, the capacity paired with its
    /// case temperature, and the span of case temperature over every load of
    /// the run read so far. The change between two segments depends on their
    /// order and is refused by the card's step where it falls.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "cell_2170_model3"),
        )?;
        enforce_published("energy", "cell_2170_model3", Published::Load, params)?;
        let load = Self::read_load(params)?;
        if !self
            .cell
            .capacity_compared_at(self.state.capacity_ah, load.case_temperature_c)
        {
            return Err(cell_2170_model3_capacity_not_compared(
                self.state.capacity_ah,
                load.case_temperature_c,
            ));
        }
        let least = self.planned_least_c.min(load.case_temperature_c);
        let most = self.planned_most_c.max(load.case_temperature_c);
        if !(most - least <= self.cell.case_temperature_span_max_k) {
            return Err(cell_2170_model3_span_above_card(
                least,
                most,
                "the start's carried span and the run's segments span",
            ));
        }
        self.planned_least_c = least;
        self.planned_most_c = most;
        self.load = load;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        cell_2170_model3::step(self.cell, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = cell_2170_model3_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: cell_2170_model3_state(&self.state)?,
        })
    }
}

impl TireCall {
    /// The four held quantities of a step or of a segment. Each is inside
    /// its published bounds here; the slip must also be outside the band
    /// around zero that the constants do not cover.
    fn read_load(params: &serde_json::Value) -> Result<tire_eve_r19_ice::TireLoad, String> {
        let normal_load_n = required_f64(params, "normal_load_n")?;
        let inflation_pressure_pa = required_f64(params, "inflation_pressure_pa")?;
        let slip_ratio = required_f64(params, "slip_ratio")?;
        let carriage_speed_m_s = required_f64(params, "carriage_speed_m_s")?;
        let size = slip_ratio.abs();
        if !(size >= tire_eve_r19_ice::SLIP_FLOOR && size <= tire_eve_r19_ice::SLIP_CEILING) {
            return Err(unsealed(TIRE_SLIP_OUTSIDE_CARD));
        }
        Ok(tire_eve_r19_ice::TireLoad {
            normal_load_n,
            inflation_pressure_pa,
            slip_ratio,
            carriage_speed_m_s,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("materials", "tire_eve_r19_ice"),
        )?;
        enforce_published("materials", "tire_eve_r19_ice", Published::Step, params)?;
        let distance_m = required_f64(params, "distance_m")?;
        // The time the first step's slip has been held: carried across a
        // restart, never filled in.
        let slip_held_s = required_f64(params, "slip_held_s")?;
        let load = Self::read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let tire = tire_eve_r19_ice::TireEveR19Ice::on_rig_ice();
        // The force law has no pole. The floor and the ceiling are locked
        // constants of the card, so the clock applies no `π/ω` of its own.
        card_clock(dt_s, None, tire.floor_dt_s(), tire.gate_dt_s(), "tire_eve_r19_ice")?;
        let state = tire_eve_r19_ice::TireState::try_start(&tire, distance_m, slip_held_s)
            .map_err(tire_reject)?;
        Ok(Self {
            tire,
            state,
            load,
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("materials", "tire_eve_r19_ice"),
        )?;
        enforce_published("materials", "tire_eve_r19_ice", Published::Load, params)?;
        self.load = Self::read_load(params)?;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        tire_eve_r19_ice::step(&self.tire, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = tire_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: tire_state(&self.state)?,
        })
    }
}

impl AccumulatorCall {
    fn read_load(params: &serde_json::Value) -> Result<accumulator_sbo450::AccumulatorLoad, String> {
        Ok(accumulator_sbo450::AccumulatorLoad {
            piston_speed_m_s: required_f64(params, "piston_speed_m_s")?,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "accumulator_sbo450"),
        )?;
        enforce_published("energy", "accumulator_sbo450", Published::Step, params)?;
        let gas_mass_kg = required_f64(params, "gas_mass_kg")?;
        let gas_pressure_pa = required_f64(params, "gas_pressure_pa")?;
        let gas_temperature_k = required_f64(params, "gas_temperature_k")?;
        let wall_temperature_k = required_f64(params, "wall_temperature_k")?;
        // The sums of the motion before this start: the heat share is weighed
        // over the whole motion, so a carried-on run brings them and a start
        // without them is refused as a missing input.
        let work_moved_j = required_f64(params, "work_moved_j")?;
        let heat_moved_j = required_f64(params, "heat_moved_j")?;
        let load = Self::read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let accumulator = accumulator_sbo450::AccumulatorSbo450::on_darmstadt_rig();
        // The floor and the ceiling come from the thermal time constant, a
        // locked constant of the card, so the clock applies no `π/ω` of its own.
        card_clock(
            dt_s,
            None,
            accumulator.floor_dt_s(),
            accumulator.gate_dt_s(),
            "accumulator_sbo450",
        )?;
        let state = accumulator_sbo450::GasState::try_start(
            &accumulator,
            gas_mass_kg,
            gas_pressure_pa,
            gas_temperature_k,
            wall_temperature_k,
            work_moved_j,
            heat_moved_j,
        )
        .map_err(accumulator_reject)?;
        Ok(Self {
            accumulator,
            state,
            load,
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "accumulator_sbo450"),
        )?;
        enforce_published("energy", "accumulator_sbo450", Published::Load, params)?;
        self.load = Self::read_load(params)?;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        accumulator_sbo450::step(&self.accumulator, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| {
                let detail = accumulator_reject(reject);
                StepFailure::from_reject(reject, detail)
            })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: accumulator_state(&self.state)?,
        })
    }
}

impl GeneratorCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "generator_mmrtg_f1"),
        )?;
        enforce_published("energy", "generator_mmrtg_f1", Published::Step, params)?;
        let mission_time_s = required_f64(params, "mission_time_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let generator = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
        card_clock(
            dt_s,
            Some(generator.combined_rate_per_s),
            generator.floor_dt_s(),
            generator.gate_dt_s(),
            "generator_mmrtg_f1",
        )?;
        if !(mission_time_s >= 0.0 && mission_time_s <= generator_mmrtg_f1::MISSION_TIME_MAX_S) {
            return Err(unsealed(
                "mission time is before landing or after mission day 4,087 (MissionTimeOutsideCard).",
            ));
        }
        // Thermal power and the fading fraction are the generator's at this
        // mission time. The caller does not set them.
        let state =
            generator_mmrtg_f1::GeneratorState::try_mission_time(&generator, mission_time_s)
                .ok_or_else(|| phase_not_valid("generator_mmrtg_f1"))?;
        Ok(Self {
            generator,
            state,
            load: generator_mmrtg_f1::GeneratorLoad {},
            dt_s,
        })
    }

    /// The card has no load. The object must be there and must be empty.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "generator_mmrtg_f1"),
        )?;
        enforce_published("energy", "generator_mmrtg_f1", Published::Load, params)?;
        self.load = generator_mmrtg_f1::GeneratorLoad {};
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        // Every step, the first included, is the card's own comparison with
        // the last day and its one rounding tolerance.
        let stepped = generator_mmrtg_f1::step(
            &self.generator,
            &mut self.state,
            frame,
            self.load,
            self.dt_s,
        );
        stepped.map_err(|reject| {
            let detail = generator_reject(reject);
            StepFailure::from_reject(reject, detail)
        })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: generator_state(&self.state)?,
        })
    }
}

impl CreepCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("materials", "creep_cmsx6"),
        )?;
        enforce_published("materials", "creep_cmsx6", Published::Step, params)?;
        let creep_extension_percent = required_f64(params, "creep_extension_percent")?;
        let stress_mpa = required_f64(params, "stress_mpa")?;
        let dt_s = required_f64(params, "dt_s")?;
        let alloy = creep_cmsx6::CreepCmsx6::at_980_c();
        // The alloy has no frequency. Its ceiling is a fifth of the shortest
        // time constant of its creep law.
        card_clock(dt_s, None, alloy.floor_dt_s(), alloy.gate_dt_s(), "creep_cmsx6")?;
        if !(stress_mpa >= alloy.stress_min_mpa && stress_mpa <= alloy.stress_max_mpa) {
            return Err(creep_stress_outside_card());
        }
        // The lower end is the same at every stress; the upper end is the
        // card's own function of the stress.
        if !(creep_extension_percent >= alloy.creep_extension_min_percent) {
            return Err(creep_extension_below_card());
        }
        if !(creep_extension_percent <= alloy.creep_extension_max_percent(stress_mpa)) {
            return Err(creep_extension_outside_card());
        }
        let state = creep_cmsx6::CreepState::try_new(&alloy, stress_mpa, creep_extension_percent)
            .ok_or_else(|| phase_not_valid("creep_cmsx6"))?;
        Ok(Self {
            alloy,
            state,
            load: creep_cmsx6::CreepLoad { stress_mpa },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("materials", "creep_cmsx6"))?;
        enforce_published("materials", "creep_cmsx6", Published::Load, params)?;
        let stress_mpa = required_f64(params, "stress_mpa")?;
        if !(stress_mpa >= self.alloy.stress_min_mpa && stress_mpa <= self.alloy.stress_max_mpa) {
            return Err(creep_stress_outside_card());
        }
        // The request itself shows a changed stress: refused before a step.
        if stress_mpa != self.state.stress_mpa {
            return Err(creep_stress_not_held());
        }
        self.load = creep_cmsx6::CreepLoad { stress_mpa };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        creep_cmsx6::step(&self.alloy, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = creep_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: creep_state(&self.state)?,
        })
    }
}

impl GridGbCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names("energy", "grid_gb"))?;
        enforce_published("energy", "grid_gb", Published::Step, params)?;
        let frequency_hz = required_f64(params, "frequency_hz")?;
        let inertia_mw_s = required_f64(params, "inertia_mw_s")?;
        let demand_mw = required_f64(params, "demand_mw")?;
        let loss_mw = required_f64(params, "loss_mw")?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = grid_gb::GridGb::interconnector_loss();
        // The ceiling is a fiftieth of the fastest time scale the card can
        // reach, so the clock applies no `π/ω` of its own.
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), "grid_gb")?;
        if !(loss_mw >= card.loss_min_mw && loss_mw <= card.loss_max_mw) {
            return Err(grid_gb_loss_outside_card());
        }
        let state = grid_gb::GridState::try_start(&card, frequency_hz, inertia_mw_s, demand_mw)
            .ok_or_else(|| phase_not_valid("grid_gb"))?;
        Ok(Self {
            card,
            state,
            load: grid_gb::GridLoad { loss_mw },
            dt_s,
            run_loss_mw: loss_mw,
        })
    }

    /// The load of a segment. It is held to the card's span, and then to the
    /// loss the run was opened with: a run is one loss, and the card's step
    /// refuses another once the first step has landed. `run_trajectory` reads
    /// every segment through here before the first step, so a run that would
    /// change its loss is refused whole and nothing of it is sealed.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "grid_gb"))?;
        enforce_published("energy", "grid_gb", Published::Load, params)?;
        let loss_mw = required_f64(params, "loss_mw")?;
        if !(loss_mw >= self.card.loss_min_mw && loss_mw <= self.card.loss_max_mw) {
            return Err(grid_gb_loss_outside_card());
        }
        if loss_mw != self.run_loss_mw {
            return Err(grid_gb_loss_changed(loss_mw, self.run_loss_mw));
        }
        self.load = grid_gb::GridLoad { loss_mw };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        grid_gb::step(&self.card, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = grid_gb_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: grid_gb_state(&self.state)?,
        })
    }
}

impl ReactorCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "reactor_spert3"),
        )?;
        enforce_published("energy", "reactor_spert3", Published::Step, params)?;
        let coolant_temperature_k = required_f64(params, "coolant_temperature_k")?;
        let reactivity_inserted_dollar = required_f64(params, "reactivity_inserted_dollar")?;
        let dt_s = required_f64(params, "dt_s")?;
        let reactor = reactor_spert3::ReactorSpert3::e_core_cold_startup();
        // The reactor has no frequency. Its ceiling is a fifth of the time
        // constant of its fastest pole.
        card_clock(
            dt_s,
            None,
            reactor.floor_dt_s(),
            reactor.gate_dt_s(),
            "reactor_spert3",
        )?;
        if !(coolant_temperature_k >= reactor.coolant_temperature_min_k
            && coolant_temperature_k <= reactor.coolant_temperature_max_k)
        {
            return Err(reactor_coolant_outside_card());
        }
        if !(reactivity_inserted_dollar >= reactor.reactivity_min_dollar
            && reactivity_inserted_dollar <= reactor.reactivity_max_dollar)
        {
            return Err(reactor_reactivity_outside_card());
        }
        let state = reactor_spert3::ReactorState::try_cold_start(&reactor, coolant_temperature_k)
            .ok_or_else(|| phase_not_valid("reactor_spert3"))?;
        Ok(Self {
            reactor,
            state,
            load: reactor_spert3::ReactorLoad {
                reactivity_inserted_dollar,
            },
            burst_reactivity_dollar: reactivity_inserted_dollar,
            dt_s,
        })
    }

    /// The load of a segment. It is held to the card's span, and then to the
    /// reactivity the run was opened with: a run is one burst, and the card's
    /// step refuses another reactivity once the first step has landed.
    /// `run_trajectory` reads every segment through here before the first
    /// step, so a run that would change its reactivity is refused whole and
    /// nothing of it is sealed.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "reactor_spert3"))?;
        enforce_published("energy", "reactor_spert3", Published::Load, params)?;
        let reactivity_inserted_dollar = required_f64(params, "reactivity_inserted_dollar")?;
        if !(reactivity_inserted_dollar >= self.reactor.reactivity_min_dollar
            && reactivity_inserted_dollar <= self.reactor.reactivity_max_dollar)
        {
            return Err(reactor_reactivity_outside_card());
        }
        if reactivity_inserted_dollar != self.burst_reactivity_dollar {
            return Err(reactor_reactivity_changed(
                reactivity_inserted_dollar,
                self.burst_reactivity_dollar,
            ));
        }
        self.load = reactor_spert3::ReactorLoad {
            reactivity_inserted_dollar,
        };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        reactor_spert3::step(&self.reactor, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = reactor_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: reactor_state(&self.state)?,
        })
    }
}

impl ReactorTrigaCall {
    fn read_load(params: &serde_json::Value) -> Result<reactor_triga_jsi::ReactorLoad, String> {
        Ok(reactor_triga_jsi::ReactorLoad {
            reactivity_inserted_dollar: required_f64(params, "reactivity_inserted_dollar")?,
            rods_dropped: required_f64(params, "rods_dropped")?,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "reactor_triga_jsi"),
        )?;
        enforce_published("energy", "reactor_triga_jsi", Published::Step, params)?;
        let start_temperature_k = required_f64(params, "start_temperature_k")?;
        let rod_table_core = required_f64(params, "rod_table_core")?;
        // Read here so that a missing or non-number load is named before the clock.
        let load = Self::read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let reactor = reactor_triga_jsi::ReactorTrigaJsi::core_56_pulse_mode();
        // The reactor has no frequency. Its ceiling is a fifth of the time
        // constant of its prompt pole after the scram.
        card_clock(
            dt_s,
            None,
            reactor.floor_dt_s(),
            reactor.gate_dt_s(),
            "reactor_triga_jsi",
        )?;
        // The published span of rod_table_core holds 215 to 231; only three
        // numbers in it are tables.
        let Some(table) = reactor.table_of(rod_table_core) else {
            return Err(reactor_triga_table_outside_card(rod_table_core));
        };
        // The published span is the union of the three tables' spans; each table accepts only
        // the starts compared on it.
        if start_temperature_k.is_finite() && !reactor.start_compared_on(table, start_temperature_k) {
            return Err(reactor_triga_start_outside_table(start_temperature_k, rod_table_core));
        }
        let state =
            reactor_triga_jsi::ReactorState::try_start(&reactor, start_temperature_k, rod_table_core)
                .ok_or_else(|| phase_not_valid("reactor_triga_jsi"))?;
        Ok(Self {
            reactor,
            state,
            load,
            run_reactivity_dollar: load.reactivity_inserted_dollar,
            dt_s,
        })
    }

    /// The load of a segment. A run is one pulse with one inserted
    /// reactivity: `run_trajectory` reads every segment through here before
    /// the first step, so a run that would change its reactivity is refused
    /// whole and nothing of it is sealed. The scram value may go from 0 to 1;
    /// the card's own step holds the rest of the order.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "reactor_triga_jsi"),
        )?;
        enforce_published("energy", "reactor_triga_jsi", Published::Load, params)?;
        let load = Self::read_load(params)?;
        if load.reactivity_inserted_dollar.to_bits() != self.run_reactivity_dollar.to_bits() {
            return Err(reactor_triga_reactivity_changed(
                load.reactivity_inserted_dollar,
                self.run_reactivity_dollar,
            ));
        }
        self.load = load;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        reactor_triga_jsi::step(&self.reactor, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| {
                let detail = reactor_triga_reject(reject);
                StepFailure::from_reject(reject, detail)
            })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: reactor_triga_state(&self.state)?,
        })
    }
}

impl ReactorCcoreCall {
    /// The reactivity and the flow of a load, held to what was compared at the run's pressure.
    fn read_load(
        reactor: &reactor_spert3_ccore::ReactorSpert3Ccore,
        system_pressure_psig: f64,
        params: &serde_json::Value,
    ) -> Result<reactor_spert3_ccore::ReactorLoad, String> {
        let reactivity_inserted_dollar = required_f64(params, "reactivity_inserted_dollar")?;
        let coolant_flow_m_per_s = required_f64(params, "coolant_flow_m_per_s")?;
        if !reactor.flow_on_card(system_pressure_psig, coolant_flow_m_per_s) {
            return Err(reactor_ccore_flow_outside_card(coolant_flow_m_per_s, system_pressure_psig));
        }
        if !reactor.reactivity_on_card(coolant_flow_m_per_s, reactivity_inserted_dollar) {
            return Err(reactor_ccore_reactivity_outside_card(
                reactivity_inserted_dollar,
                coolant_flow_m_per_s,
            ));
        }
        Ok(reactor_spert3_ccore::ReactorLoad {
            reactivity_inserted_dollar,
            coolant_flow_m_per_s,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "reactor_spert3_ccore"),
        )?;
        enforce_published("energy", "reactor_spert3_ccore", Published::Step, params)?;
        let initial_temperature_k = required_f64(params, "initial_temperature_k")?;
        let system_pressure_psig = required_f64(params, "system_pressure_psig")?;
        required_f64(params, "reactivity_inserted_dollar")?;
        required_f64(params, "coolant_flow_m_per_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let reactor = reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients();
        card_clock(
            dt_s,
            None,
            reactor.floor_dt_s(),
            reactor.gate_dt_s(),
            "reactor_spert3_ccore",
        )?;
        let state = reactor_spert3_ccore::ReactorState::try_start(
            &reactor,
            initial_temperature_k,
            system_pressure_psig,
        )
        .ok_or_else(|| reactor_ccore_start_outside_card(initial_temperature_k, system_pressure_psig))?;
        let load = Self::read_load(&reactor, system_pressure_psig, params)?;
        Ok(Self {
            reactor,
            state,
            load,
            run_load: load,
            dt_s,
        })
    }

    /// The load of a segment, held to what was compared and then to the load the run was
    /// opened with. `run_trajectory` reads every segment through here before the first step,
    /// so a run that would change its load is refused whole.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "reactor_spert3_ccore"),
        )?;
        enforce_published("energy", "reactor_spert3_ccore", Published::Load, params)?;
        let load = Self::read_load(&self.reactor, self.state.system_pressure_psig, params)?;
        if load.reactivity_inserted_dollar != self.run_load.reactivity_inserted_dollar {
            return Err(reactor_ccore_reactivity_changed(
                load.reactivity_inserted_dollar,
                self.run_load.reactivity_inserted_dollar,
            ));
        }
        if load.coolant_flow_m_per_s != self.run_load.coolant_flow_m_per_s {
            return Err(reactor_ccore_flow_changed(
                load.coolant_flow_m_per_s,
                self.run_load.coolant_flow_m_per_s,
            ));
        }
        self.load = load;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        reactor_spert3_ccore::step(&self.reactor, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| {
                let detail = reactor_ccore_reject(reject);
                StepFailure::from_reject(reject, detail)
            })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: reactor_ccore_state(&self.state)?,
        })
    }
}

impl ReactorFuelCall {
    /// The reactivity and the flow of a load, each held to the span of the
    /// band the run started in.
    fn read_load(
        reactor: &reactor_spert3_fuel::ReactorSpert3Fuel,
        band: usize,
        params: &serde_json::Value,
    ) -> Result<reactor_spert3_fuel::ReactorLoad, String> {
        let reactivity_inserted_dollar = required_f64(params, "reactivity_inserted_dollar")?;
        let coolant_flow_m_per_s = required_f64(params, "coolant_flow_m_per_s")?;
        if !(reactivity_inserted_dollar >= reactor.band_reactivity_min_dollar[band]
            && reactivity_inserted_dollar <= reactor.band_reactivity_max_dollar[band])
        {
            return Err(reactor_fuel_reactivity_outside_band(
                reactivity_inserted_dollar,
                band,
            ));
        }
        if !(coolant_flow_m_per_s >= reactor.band_flow_min_m_per_s[band]
            && coolant_flow_m_per_s <= reactor.band_flow_max_m_per_s[band])
        {
            return Err(reactor_fuel_flow_outside_band(coolant_flow_m_per_s, band));
        }
        Ok(reactor_spert3_fuel::ReactorLoad {
            reactivity_inserted_dollar,
            coolant_flow_m_per_s,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "reactor_spert3_fuel"),
        )?;
        enforce_published("energy", "reactor_spert3_fuel", Published::Step, params)?;
        let inlet_temperature_k = required_f64(params, "inlet_temperature_k")?;
        // Read here so that a missing or non-number load is named before the clock.
        required_f64(params, "reactivity_inserted_dollar")?;
        required_f64(params, "coolant_flow_m_per_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
        // The reactor has no frequency. Its ceiling is a fifth of the reduced
        // generation time of the cold band.
        card_clock(
            dt_s,
            None,
            reactor.floor_dt_s(),
            reactor.gate_dt_s(),
            "reactor_spert3_fuel",
        )?;
        // The band of the inlet temperature sets the spans of the load.
        let band = reactor
            .band_of(inlet_temperature_k)
            .ok_or_else(|| reactor_fuel_inlet_outside_card(inlet_temperature_k))?;
        let load = Self::read_load(&reactor, band, params)?;
        let state = reactor_spert3_fuel::ReactorState::try_start(&reactor, inlet_temperature_k)
            .ok_or_else(|| phase_not_valid("reactor_spert3_fuel"))?;
        Ok(Self {
            reactor,
            state,
            load,
            burst_load: load,
            dt_s,
        })
    }

    /// The load of a segment. It is held to the span of the band, and then to
    /// the load the run was opened with: a run is one burst, and the card's
    /// step refuses another reactivity or another flow rate once the first
    /// step has landed. `run_trajectory` reads every segment through here
    /// before the first step, so a run that would change its load is refused
    /// whole and nothing of it is sealed.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "reactor_spert3_fuel"),
        )?;
        enforce_published("energy", "reactor_spert3_fuel", Published::Load, params)?;
        let band = self
            .reactor
            .band_of(self.state.inlet_temperature_k)
            .ok_or_else(|| reactor_fuel_inlet_outside_card(self.state.inlet_temperature_k))?;
        let load = Self::read_load(&self.reactor, band, params)?;
        if load.reactivity_inserted_dollar != self.burst_load.reactivity_inserted_dollar {
            return Err(reactor_fuel_reactivity_changed(
                load.reactivity_inserted_dollar,
                self.burst_load.reactivity_inserted_dollar,
            ));
        }
        if load.coolant_flow_m_per_s != self.burst_load.coolant_flow_m_per_s {
            return Err(reactor_fuel_flow_changed(
                load.coolant_flow_m_per_s,
                self.burst_load.coolant_flow_m_per_s,
            ));
        }
        self.load = load;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        reactor_spert3_fuel::step(&self.reactor, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| {
                let detail = reactor_fuel_reject(reject);
                StepFailure::from_reject(reject, detail)
            })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: reactor_fuel_state(&self.state)?,
        })
    }
}

impl CapsuleCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("aerospace", "capsule_stardust"),
        )?;
        enforce_published("aerospace", "capsule_stardust", Published::Step, params)?;
        let radius_m = required_f64(params, "radius_m")?;
        let radial_speed_m_s = required_f64(params, "radial_speed_m_s")?;
        let transverse_speed_m_s = required_f64(params, "transverse_speed_m_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let capsule = capsule_stardust::CapsuleStardust::sample_return_capsule();
        // The capsule has no pole. Its ceiling is a locked constant of the card.
        card_clock(dt_s, None, capsule.floor_dt_s(), capsule.gate_dt_s(), "capsule_stardust")?;
        let altitude_m = radius_m - capsule.local_earth_radius_m;
        if !(altitude_m >= capsule.altitude_floor_m && altitude_m <= capsule.altitude_ceiling_m) {
            return Err(unsealed(CAPSULE_ALTITUDE_OUTSIDE_CARD));
        }
        // Inside its altitudes the card refuses a start by the name its step
        // would refuse it by.
        let state = capsule_stardust::CapsuleState::anchor(
            &capsule,
            radius_m,
            radial_speed_m_s,
            transverse_speed_m_s,
        )
        .map_err(capsule_reject)?;
        Ok(Self {
            capsule,
            state,
            load: capsule_stardust::CapsuleLoad {},
            dt_s,
        })
    }

    /// The card has no load. The object must be there and must be empty.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("aerospace", "capsule_stardust"),
        )?;
        enforce_published("aerospace", "capsule_stardust", Published::Load, params)?;
        self.load = capsule_stardust::CapsuleLoad {};
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        capsule_stardust::step(&self.capsule, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| {
                let detail = capsule_reject(reject);
                StepFailure::from_reject(reject, detail)
            })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: capsule_state(&self.state)?,
        })
    }
}

impl CapsuleInsightCall {
    fn lift_from(
        capsule: &capsule_insight::CapsuleInsight,
        params: &serde_json::Value,
    ) -> Result<capsule_insight::CapsuleLoad, String> {
        let lift_over_drag = required_f64(params, "lift_over_drag")?;
        if !(lift_over_drag >= -capsule.lift_over_drag_limit
            && lift_over_drag <= capsule.lift_over_drag_limit)
        {
            return Err(unsealed(CAPSULE_INSIGHT_LIFT_OUTSIDE_CARD));
        }
        Ok(capsule_insight::CapsuleLoad { lift_over_drag })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("aerospace", "capsule_insight"),
        )?;
        enforce_published("aerospace", "capsule_insight", Published::Step, params)?;
        let radius_m = required_f64(params, "radius_m")?;
        let radial_speed_m_s = required_f64(params, "radial_speed_m_s")?;
        let transverse_speed_m_s = required_f64(params, "transverse_speed_m_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let capsule = capsule_insight::CapsuleInsight::entry_capsule();
        let load = Self::lift_from(&capsule, params)?;
        // The capsule has no pole. Its ceiling is a locked constant of the card.
        card_clock(dt_s, None, capsule.floor_dt_s(), capsule.gate_dt_s(), "capsule_insight")?;
        // The kernel names why a start is not on the card: outside the air
        // tables, past the compared floor, or off the compared corridor.
        let state = capsule_insight::CapsuleState::start(
            &capsule,
            radius_m,
            radial_speed_m_s,
            transverse_speed_m_s,
        )
        .map_err(capsule_insight_reject)?;
        Ok(Self {
            capsule,
            state,
            load,
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("aerospace", "capsule_insight"),
        )?;
        enforce_published("aerospace", "capsule_insight", Published::Load, params)?;
        self.load = Self::lift_from(&self.capsule, params)?;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        capsule_insight::step(&self.capsule, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| {
                let detail = capsule_insight_reject(reject);
                StepFailure::from_reject(reject, detail)
            })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: capsule_insight_state(&self.state)?,
        })
    }
}

impl FingerPadCall {
    /// The two held quantities of a step or of a segment. Each is inside its
    /// published bounds here.
    fn read_load(params: &serde_json::Value) -> Result<fingerpad_p02_glass::FingerPadLoad, String> {
        Ok(fingerpad_p02_glass::FingerPadLoad {
            normal_force_n: required_f64(params, "normal_force_n")?,
            twist_rate_deg_s: required_f64(params, "twist_rate_deg_s")?,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("materials", "fingerpad_p02_glass"),
        )?;
        enforce_published("materials", "fingerpad_p02_glass", Published::Step, params)?;
        let twist_deg = required_f64(params, "twist_deg")?;
        let load = Self::read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let pad = fingerpad_p02_glass::FingerPadP02Glass::on_glass();
        // The model has no pole. The floor and the ceiling are locked
        // constants of the card, so the clock applies no `π/ω` of its own.
        card_clock(dt_s, None, pad.floor_dt_s(), pad.gate_dt_s(), "fingerpad_p02_glass")?;
        let state = fingerpad_p02_glass::FingerPadState::try_start(&pad, twist_deg)
            .ok_or_else(|| unsealed(FINGERPAD_TWIST_NOT_FROM_ZERO))?;
        Ok(Self {
            pad,
            state,
            load,
            dt_s,
        })
    }

    /// A later segment of a run may only repeat the load the run began with:
    /// no measured twist changed its force or its rate, and a changed one is
    /// refused here by name.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("materials", "fingerpad_p02_glass"),
        )?;
        enforce_published("materials", "fingerpad_p02_glass", Published::Load, params)?;
        let load = Self::read_load(params)?;
        if load.normal_force_n.to_bits() != self.load.normal_force_n.to_bits()
            || load.twist_rate_deg_s.to_bits() != self.load.twist_rate_deg_s.to_bits()
        {
            return Err(unsealed(FINGERPAD_LOAD_CHANGED));
        }
        self.load = load;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        fingerpad_p02_glass::step(&self.pad, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = fingerpad_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: fingerpad_state(&self.state)?,
        })
    }
}

impl TokamakCall {
    fn read_load(
        tokamak: &tokamak_jet_dte1::TokamakJetDte1,
        params: &serde_json::Value,
    ) -> Result<tokamak_jet_dte1::TokamakLoad, String> {
        let load = tokamak_jet_dte1::TokamakLoad {
            heating_power_w: required_f64(params, "heating_power_w")?,
            ohmic_power_w: required_f64(params, "ohmic_power_w")?,
            line_density_per_m3: required_f64(params, "line_density_per_m3")?,
            plasma_current_a: required_f64(params, "plasma_current_a")?,
            toroidal_field_t: required_f64(params, "toroidal_field_t")?,
            tritium_fraction: required_f64(params, "tritium_fraction")?,
        };
        // Each value is inside its own published bound here. The card's own
        // test then holds the density to the Greenwald density of this
        // current and to 0.834 of it, the current and the field to the
        // safety factor and to its compared span, and the tritium fraction to
        // the two spans where the fusion power was compared.
        tokamak_jet_dte1::load_in_card(tokamak, load)
            .map_err(|reject| tokamak_reject(reject, tokamak, load, f64::NAN))?;
        Ok(load)
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("energy", "tokamak_jet_dte1"),
        )?;
        enforce_published("energy", "tokamak_jet_dte1", Published::Step, params)?;
        let thermal_energy_j = required_f64(params, "thermal_energy_j")?;
        let fast_ion_energy_j = required_f64(params, "fast_ion_energy_j")?;
        // The time since the start of the heated window: the end of the card
        // is counted from it, so a run continued from a returned state states
        // that state's t_s, and a start without it is refused.
        let elapsed_time_s = required_f64(params, "elapsed_time_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let tokamak = tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode();
        // The step ceiling is a locked constant of the card.
        card_clock(
            dt_s,
            None,
            tokamak.floor_dt_s(),
            tokamak.gate_dt_s(),
            "tokamak_jet_dte1",
        )?;
        let load = Self::read_load(&tokamak, params)?;
        // The start is held by the card's own test under the first load: the
        // temperature range of the reactivity fit, the Troyon limit, and the
        // envelope of the comparison.
        tokamak_jet_dte1::state_in_card(&tokamak, thermal_energy_j, fast_ion_energy_j, load)
            .map_err(|reject| tokamak_reject(reject, &tokamak, load, thermal_energy_j))?;
        let state = tokamak_jet_dte1::TokamakState::try_continue(
            &tokamak,
            elapsed_time_s,
            thermal_energy_j,
            fast_ion_energy_j,
        )
        .ok_or_else(|| phase_not_valid("tokamak_jet_dte1"))?;
        Ok(Self {
            tokamak,
            state,
            load,
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "tokamak_jet_dte1"))?;
        enforce_published("energy", "tokamak_jet_dte1", Published::Load, params)?;
        self.load = Self::read_load(&self.tokamak, params)?;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        let (tokamak, load) = (self.tokamak, self.load);
        let thermal_energy_j = self.state.thermal_energy_j;
        tokamak_jet_dte1::step(&self.tokamak, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = tokamak_reject(reject, &tokamak, load, thermal_energy_j);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: tokamak_state(&self.state)?,
        })
    }
}

/// A rain rate above zero and under one gauge count a minute is refused at
/// the door, in any segment, before any step. The published bounds have
/// already held it to 0 to 84 mm/h.
fn link_rain_on_card(rain_rate_mm_h: f64) -> Result<(), String> {
    if rain_rate_mm_h > 0.0 && rain_rate_mm_h < link_gothenburg_10010::RAIN_RATE_ONE_COUNT_MM_H {
        return Err(link_reject(
            link_gothenburg_10010::StepReject::RainRateBelowGaugeCount,
        ));
    }
    Ok(())
}

impl LinkCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("signal", "link_gothenburg_10010"),
        )?;
        enforce_published("signal", "link_gothenburg_10010", Published::Step, params)?;
        let dry_path_loss_db = required_f64(params, "dry_path_loss_db")?;
        let antenna_water_db = required_f64(params, "antenna_water_db")?;
        let gauge_minute_elapsed_s = required_f64(params, "gauge_minute_elapsed_s")?;
        let gauge_minute_rain_mm_h = required_f64(params, "gauge_minute_rain_mm_h")?;
        let rain_rate_mm_h = required_f64(params, "rain_rate_mm_h")?;
        let dt_s = required_f64(params, "dt_s")?;
        let link = link_gothenburg_10010::LinkGothenburg10010::sublink_587();
        // The step ceiling is the gauge's minute, a locked constant of the
        // card, so the clock applies no `π/ω` of its own.
        card_clock(dt_s, None, link.floor_dt_s(), link.gate_dt_s(), "link_gothenburg_10010")?;
        let state = link_gothenburg_10010::LinkState::try_continue(
            &link,
            dry_path_loss_db,
            antenna_water_db,
            gauge_minute_elapsed_s,
            gauge_minute_rain_mm_h,
        )
        .map_err(link_reject)?;
        link_rain_on_card(rain_rate_mm_h)?;
        if state.gauge_minute_elapsed_s > 0.0 && rain_rate_mm_h != state.gauge_minute_rain_mm_h {
            return Err(link_reject(
                link_gothenburg_10010::StepReject::RainNotHeldForGaugeMinute,
            ));
        }
        Ok(Self {
            link,
            state,
            load: link_gothenburg_10010::LinkLoad { rain_rate_mm_h },
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("signal", "link_gothenburg_10010"),
        )?;
        enforce_published("signal", "link_gothenburg_10010", Published::Load, params)?;
        let rain_rate_mm_h = required_f64(params, "rain_rate_mm_h")?;
        link_rain_on_card(rain_rate_mm_h)?;
        self.load = link_gothenburg_10010::LinkLoad { rain_rate_mm_h };
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        link_gothenburg_10010::step(&self.link, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| {
                let detail = link_reject(reject);
                StepFailure::from_reject(reject, detail)
            })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: link_state(&self.state)?,
        })
    }
}

impl LosCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("aerospace", "los_jwst_fine_guide"),
        )?;
        enforce_published("aerospace", "los_jwst_fine_guide", Published::Step, params)?;
        let lock_x_arcsec = required_f64(params, "lock_x_arcsec")?;
        let lock_y_arcsec = required_f64(params, "lock_y_arcsec")?;
        let quiet_before_start_s = required_f64(params, "quiet_before_start_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = los_jwst_fine_guide::LosJwstFineGuide::observatory();
        // The step is one frame of the sensor: the card's own floor and
        // ceiling, and no `π/ω` beside them.
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), "los_jwst_fine_guide")?;
        if !card.recovery_has_elapsed(quiet_before_start_s) {
            return Err(unsealed(LOS_RECOVERY_NOT_ELAPSED));
        }
        let load = los_load(&card, lock_x_arcsec, lock_y_arcsec, params)?;
        let state = los_jwst_fine_guide::LosState::try_lock(
            &card,
            lock_x_arcsec,
            lock_y_arcsec,
            quiet_before_start_s,
        )
        .ok_or_else(|| phase_not_valid("los_jwst_fine_guide"))?;
        Ok(Self {
            card,
            state,
            load,
            dt_s,
        })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("aerospace", "los_jwst_fine_guide"),
        )?;
        enforce_published("aerospace", "los_jwst_fine_guide", Published::Load, params)?;
        self.load = los_load(
            &self.card,
            self.state.lock_x_arcsec,
            self.state.lock_y_arcsec,
            params,
        )?;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        los_jwst_fine_guide::step(&self.card, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = los_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: los_state(&self.state)?,
        })
    }
}

impl CantileverCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("signal", "cantilever_eth"),
        )?;
        enforce_published("signal", "cantilever_eth", Published::Step, params)?;
        let deflection_m = required_f64(params, "deflection_m")?;
        let velocity_m_s = required_f64(params, "velocity_m_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let beam = cantilever_eth::CantileverEth::first_bending_mode();
        // The step ceiling is a locked constant of the card, set by
        // measurement, so the clock applies no `π/ω` of its own.
        card_clock(dt_s, None, beam.floor_dt_s(), beam.gate_dt_s(), "cantilever_eth")?;
        // The deflection and the speed are each inside their own bound here.
        // Together they must keep the card's own envelope at or under its
        // ceiling and at or over its floor. The envelope is the card's
        // function.
        let envelope_m = beam.envelope_m(deflection_m, velocity_m_s);
        if !(envelope_m <= beam.amplitude_ceiling_m) {
            return Err(unsealed(CANTILEVER_DEFLECTION_ABOVE_CARD));
        }
        if !(envelope_m >= beam.amplitude_floor_m) {
            return Err(unsealed(CANTILEVER_DEFLECTION_BELOW_CARD));
        }
        let state = cantilever_eth::CantileverState::try_phase(&beam, deflection_m, velocity_m_s)
            .ok_or_else(|| phase_not_valid("cantilever_eth"))?;
        Ok(Self {
            beam,
            state,
            load: cantilever_eth::CantileverLoad {},
            dt_s,
        })
    }

    /// The card has no load. The object must be there and must be empty.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("signal", "cantilever_eth"),
        )?;
        enforce_published("signal", "cantilever_eth", Published::Load, params)?;
        self.load = cantilever_eth::CantileverLoad {};
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        cantilever_eth::step(&self.beam, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = cantilever_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: cantilever_state(&self.state)?,
        })
    }
}

impl SolarSystemCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("aerospace", "solar_system"),
        )?;
        let mut position_m = [[0.0; 3]; solar_system::BODIES];
        let mut velocity_m_s = [[0.0; 3]; solar_system::BODIES];
        for body in 0..solar_system::BODIES {
            position_m[body] = required_vec3(params, descriptor::SOLAR_SYSTEM_POSITION_KEYS[body])?;
            velocity_m_s[body] =
                required_vec3(params, descriptor::SOLAR_SYSTEM_VELOCITY_KEYS[body])?;
        }
        // The ledger is carried from run to run: a start without it is
        // refused as a missing parameter, so a returned state handed back
        // cannot begin its ledger again by leaving it out.
        let ledger = solar_system::SolarSystemLedger {
            energy_anchor: required_f64(params, "energy_anchor")?,
            booked_work: required_f64(params, "booked_work")?,
            angular_momentum_anchor: required_vec3(params, "angular_momentum_anchor")?,
            booked_torque: required_vec3(params, "booked_torque")?,
        };
        let dt_s = required_f64(params, "dt_s")?;
        let system = solar_system::SolarSystem::de440();
        // The ceiling is a locked constant of the card, not a share of an orbit.
        card_clock(dt_s, None, system.floor_dt_s(), system.gate_dt_s(), "solar_system")?;
        enforce_published("aerospace", "solar_system", Published::Step, params)?;
        // The card itself says why it will not start a run, in its own order:
        // a distance, then an orbit, then where the compared run went, then
        // two bodies at one point or too close, then the frame, then the
        // origin, then the axes, then the carried ledger.
        let state = solar_system::SolarSystemState::try_continue(
            &system,
            position_m,
            velocity_m_s,
            ledger,
        )
        .map_err(|reject| match reject {
            solar_system::StepReject::DistanceOutsideCard
            | solar_system::StepReject::OrbitOutsideCard
            | solar_system::StepReject::DistanceOutsideComparedRun
            | solar_system::StepReject::BodiesCoincide
            | solar_system::StepReject::BodiesTooClose
            | solar_system::StepReject::FrameOutsideCard
            | solar_system::StepReject::OriginOutsideCard
            | solar_system::StepReject::AxesOutsideCard
            | solar_system::StepReject::SpontaneousGeneration
            | solar_system::StepReject::FirstLawBreach
            | solar_system::StepReject::AngularMomentumBreach => solar_system_reject(reject),
            _ => phase_not_valid("solar_system"),
        })?;
        Ok(Self {
            system,
            state,
            load: solar_system::SolarSystemLoad {},
            dt_s,
        })
    }

    /// The card has no load. The object must be there and must be empty.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("aerospace", "solar_system"),
        )?;
        enforce_published("aerospace", "solar_system", Published::Load, params)?;
        self.load = solar_system::SolarSystemLoad {};
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        solar_system::step(&self.system, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = solar_system_reject(reject);
                StepFailure::from_reject(reject, detail)
            },
        )
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: solar_system_state(&self.state)?,
        })
    }
}

impl SatelliteCall {
    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(
            params,
            descriptor::parameter_names("aerospace", "satellite_lageos1"),
        )?;
        let epoch_utc_s = required_f64(params, "epoch_utc_s")?;
        // The time since the published state is required: without it a
        // restart from a returned state would begin a new seven days.
        let forecast_elapsed_s = required_f64(params, "forecast_elapsed_s")?;
        let position_m = required_vec3(params, "position_m")?;
        let velocity_m_s = required_vec3(params, "velocity_m_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let satellite = satellite_lageos1::SatelliteLageos1::lageos1();
        // The ceiling is a locked constant of the card, not a share of the orbit.
        card_clock(
            dt_s,
            None,
            satellite.floor_dt_s(),
            satellite.gate_dt_s(),
            "satellite_lageos1",
        )?;
        let state = satellite_lageos1::SatelliteState::try_state(
            &satellite,
            epoch_utc_s,
            forecast_elapsed_s,
            position_m,
            velocity_m_s,
        )
        .ok_or_else(|| {
            satellite_off_card(&satellite, epoch_utc_s, forecast_elapsed_s, position_m, velocity_m_s)
        })?;
        // The card names the radius before the orbit, the orbit before the
        // date and the date before the forecast. The date and the forecast
        // are held to the spans `describe_card` publishes after.
        enforce_published("aerospace", "satellite_lageos1", Published::Step, params)?;
        Ok(Self {
            satellite,
            state,
            load: satellite_lageos1::SatelliteLoad {},
            dt_s,
        })
    }

    /// The card has no load. The object must be there and must be empty.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("aerospace", "satellite_lageos1"),
        )?;
        enforce_published("aerospace", "satellite_lageos1", Published::Load, params)?;
        self.load = satellite_lageos1::SatelliteLoad {};
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        // The first step from a start has no running clock to round: it
        // lands on or before the last date or it is the card's refusal, as
        // the card's step gave it before the allowance existed. This keeps
        // the published latest start a bound that holds no allowance.
        if self.state.t_s == 0.0
            && !(self.state.epoch_utc_s() + self.dt_s <= satellite_lageos1::EPOCH_MAX_S)
        {
            let reject = satellite_lageos1::StepReject::EpochOutsideCard;
            return Err(StepFailure::from_reject(reject, satellite_reject(reject)));
        }
        satellite_lageos1::step(
            &self.satellite,
            &mut self.state,
            frame,
            self.load,
            self.dt_s,
        )
        .map_err(|reject| {
            let detail = satellite_reject(reject);
            StepFailure::from_reject(reject, detail)
        })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame)
            .map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: satellite_state(&self.state)?,
        })
    }
}

/// Why the card will not anchor this state, in the order the card decides:
/// the radius, then the orbit, then the date, then the forecast. The radius
/// is the length of the position. The orbit is asked of the card itself, at a
/// date and a forecast it accepts, so the gateway works out no orbit of its
/// own.
fn satellite_off_card(
    satellite: &satellite_lageos1::SatelliteLageos1,
    epoch_utc_s: f64,
    forecast_elapsed_s: f64,
    position_m: [f64; 3],
    velocity_m_s: [f64; 3],
) -> String {
    let radius_m = (position_m[0] * position_m[0]
        + position_m[1] * position_m[1]
        + position_m[2] * position_m[2])
        .sqrt();
    if !(radius_m >= satellite.radius_min_m && radius_m <= satellite.radius_max_m) {
        return unsealed(SATELLITE_RADIUS_OUTSIDE_CARD);
    }
    let at_an_accepted_date = satellite_lageos1::SatelliteState::try_state(
        satellite,
        satellite.epoch_min_s,
        0.0,
        position_m,
        velocity_m_s,
    );
    if at_an_accepted_date.is_none() {
        return unsealed(SATELLITE_ORBIT_OUTSIDE_CARD);
    }
    if !(epoch_utc_s >= satellite.epoch_min_s && epoch_utc_s <= satellite.epoch_max_s) {
        let published = descriptor::find("aerospace", "satellite_lageos1")
            .and_then(|spec| spec.fields.iter().find(|field| field.name == "epoch_utc_s"))
            .and_then(|field| published_outside("satellite_lageos1", field, epoch_utc_s));
        return published.unwrap_or_else(|| unsealed(SATELLITE_EPOCH_OUTSIDE_CARD));
    }
    if !(forecast_elapsed_s >= 0.0 && forecast_elapsed_s <= satellite.forecast_horizon_s) {
        let published = descriptor::find("aerospace", "satellite_lageos1")
            .and_then(|spec| spec.fields.iter().find(|field| field.name == "forecast_elapsed_s"))
            .and_then(|field| published_outside("satellite_lageos1", field, forecast_elapsed_s));
        return published.unwrap_or_else(|| unsealed(SATELLITE_HORIZON_OUTSIDE_CARD));
    }
    phase_not_valid("satellite_lageos1")
}

/// One physics permit as the caller holds it: a place in the caller's share
/// when it has one, and a place in the shared pool. Both go back together
/// when this is dropped, by error or by panic.
pub(crate) struct PhysicsPermit {
    _share: Option<OwnedSemaphorePermit>,
    _pool: OwnedSemaphorePermit,
}

/// A physics permit, or the busy answer. A caller whose share is used up is
/// answered busy exactly as a caller who finds the pool empty: the same text,
/// the same status, and nothing is taken from the pool for it.
fn acquire_physics_permit(state: &AppState) -> Result<PhysicsPermit, (StatusCode, String)> {
    let busy = || {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many concurrent physics tasks.".to_string(),
        )
    };
    let share = match &state.share {
        Some(share) => Some(Arc::clone(share).try_acquire_owned().map_err(|_| busy())?),
        None => None,
    };
    let pool = Arc::clone(&state.physics_permits)
        .try_acquire_owned()
        .map_err(|_| busy())?;
    Ok(PhysicsPermit {
        _share: share,
        _pool: pool,
    })
}

fn door_status(error: DoorError) -> (StatusCode, String) {
    match error {
        DoorError::Unprocessable(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),
        DoorError::TooMany(message) => (StatusCode::TOO_MANY_REQUESTS, message),
        DoorError::NotFound(message) => (StatusCode::NOT_FOUND, message),
        DoorError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

pub(crate) async fn on_pool<T, F>(state: &AppState, work: F) -> Result<T, DoorError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    let permit = acquire_physics_permit(state)
        .map_err(|(_, message)| DoorError::TooMany(message))?;
    #[cfg(test)]
    let hold = state.permit_hold;
    let task = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let result = work();
        #[cfg(test)]
        if !hold.is_zero() {
            std::thread::sleep(hold);
        }
        result
    });
    task.await
        .expect("physics integrator panicked")
        .map_err(DoorError::Unprocessable)
}

/// Bits a landed step may set and still seal. Every other bit is a failure.
fn advisory_flags(domain: &str, card: &str) -> u64 {
    match (domain, card) {
        ("energy", "grid_node") => grid_node::FLAG_GRID_SURGE,
        ("energy", "thermal") => thermal::FLAG_THERMAL_DERATE,
        ("signal", "resonance") => resonance::FLAG_STROKE_LIMIT,
        ("energy", "swing") => swing::FLAG_PLL_SLIP,
        ("materials", "hydraulic_fluid") => {
            hydraulic_fluid::FLAG_COLUMN_SOFT | hydraulic_fluid::FLAG_COLUMN_LIMIT
        }
        ("materials", "cable_stretch") => {
            cable_stretch::FLAG_TRACKING_DEGRADE | cable_stretch::FLAG_CONTROL_LIMIT
        }
        ("energy", "battery_sag") => battery_sag::FLAG_SAG_WARNING,
        ("energy", "cell_18650pf") => cell_18650pf::FLAG_SAG_WARNING,
        ("energy", "generator_mmrtg_f1") => generator_mmrtg_f1::FLAG_PAST_FITTING_SPAN,
        ("aerospace", "capsule_stardust") => capsule_stardust::FLAG_DROGUE_MACH_WINDOW,
        ("aerospace", "satellite_lageos1") => satellite_lageos1::FLAG_EARTH_SHADOW,
        ("energy", "reactor_spert3") => reactor_spert3::FLAG_PROMPT_SUPERCRITICAL,
        ("signal", "link_gothenburg_10010") => link_gothenburg_10010::FLAG_RAINING,
        ("aerospace", "los_jwst_fine_guide") => los_jwst_fine_guide::FLAG_NO_MEASUREMENT,
        ("energy", "cell_2170_model3") => cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C,
        ("materials", "creep_cmsx6") => creep_cmsx6::FLAG_CREEP_RATE_RISING,
        ("aerospace", "capsule_insight") => capsule_insight::FLAG_LIFT_ACTING,
        ("energy", "reactor_spert3_fuel") => {
            reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL | reactor_spert3_fuel::FLAG_ROD_MOVING
        }
        ("energy", "reactor_spert3_ccore") => {
            reactor_spert3_ccore::FLAG_PROMPT_SUPERCRITICAL | reactor_spert3_ccore::FLAG_PAST_POWER_PEAK
        }
        ("energy", "reactor_triga_jsi") => {
            reactor_triga_jsi::FLAG_PROMPT_SUPERCRITICAL
                | reactor_triga_jsi::FLAG_ROD_MOVING
                | reactor_triga_jsi::FLAG_RODS_DROPPED
        }
        ("aerospace", "crater_ryugu_sci") => crater_ryugu_sci::FLAG_STOPPED,
        ("materials", "tire_eve_r19_ice") => tire_eve_r19_ice::FLAG_LOAD_ABOVE_FITTED,
        ("energy", "tokamak_jet_dte1") => tokamak_jet_dte1::FLAG_NEAR_DENSITY_LIMIT,
        ("materials", "fingerpad_p02_glass") => fingerpad_p02_glass::FLAG_FULL_SLIP,
        _ => 0,
    }
}

fn frame_slots_are_finite(frame: &LastStateFrame64) -> bool {
    frame.t.is_finite()
        && frame.pos.iter().all(|slot| slot.is_finite())
        && frame.vel.iter().all(|slot| slot.is_finite())
        && frame.force_torque.is_finite()
        && frame.residual.is_finite()
}

/// The keys a step request takes. All three are required.
pub(crate) const STEP_KEYS: &[&str] = &["domain", "card", "parameters"];

/// A step request. Keys other than `domain`, `card`, and `parameters`
/// are refused with the same sentence a trajectory uses for that key.
fn parse_step_request(value: serde_json::Value) -> Result<VerifyPhysicsRequest, DoorError> {
    let object = value.as_object().ok_or_else(|| {
        DoorError::Unprocessable(format!(
            "Physical Violation: the request must be a JSON object; this request is {}.",
            kind_of_value(&value)
        ))
    })?;
    for key in object.keys() {
        if !STEP_KEYS.contains(&key.as_str()) {
            return Err(DoorError::Unprocessable(locked_or_unknown(key)));
        }
    }
    let text = |key: &str| -> Result<String, DoorError> {
        match object.get(key) {
            None => Err(DoorError::Unprocessable(format!(
                "Missing required parameter: '{key}'"
            ))),
            Some(serde_json::Value::String(text)) => Ok(text.clone()),
            Some(other) => Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{key}' must be a string; this request has {}.",
                kind_of_value(other)
            ))),
        }
    };
    let domain = text("domain")?;
    let card = text("card")?;
    let parameters = object.get("parameters").cloned().ok_or_else(|| {
        DoorError::Unprocessable("Missing required parameter: 'parameters'".to_string())
    })?;
    Ok(VerifyPhysicsRequest { domain, card, parameters })
}

/// One anchored step. `request` is the whole request as received. It is
/// sealed into the frame proof and echoed as `inputs`. A request with `from`
/// starts from a sealed upstream run (`src/chain.rs`): the upstream is
/// computed again and checked, `from` is taken out, and the step is the same
/// step as the request typed by hand, with `chain` beside it.
pub async fn verify_step(
    state: &AppState,
    request: serde_json::Value,
) -> Result<VerifyPhysicsResponse, DoorError> {
    let (request, from) = chain::take(request);
    // The plain step's future is large; it is boxed so that this one, which
    // may hold it twice, is not.
    let Some(from) = from else {
        return Box::pin(verify_step_plain(state, request, fresh_start::Carried::Nothing)).await;
    };
    // The step's own shape is named first.
    let payload = parse_step_request(request.clone())?;
    if state.view.find(&payload.domain, &payload.card).is_some() && payload.parameters.is_object() {
        reject_unknown_load(
            &payload.parameters,
            descriptor::parameter_names(&payload.domain, &payload.card),
        )
        .map_err(DoorError::Unprocessable)?;
    }
    validate_physics(&payload.parameters).map_err(DoorError::Unprocessable)?;
    let plan = chain::read(state.view(), &request, from, chain::Target::Step, 1)?;
    let carried = plan.carried();
    let (settled, request) = on_pool(state, move || plan.settle()).await?;
    let mut response = Box::pin(verify_step_plain(state, request, carried)).await?;
    response.chain = Some(settled.chain_value(&response.sha256_seal));
    seal_response(response).map_err(|(_, message)| DoorError::Internal(message))
}

async fn verify_step_plain(
    state: &AppState,
    request: serde_json::Value,
    carried: fresh_start::Carried,
) -> Result<VerifyPhysicsResponse, DoorError> {
    let payload = parse_step_request(request.clone())?;
    // An unknown key is named as unknown before any rule that reads a key's
    // name for its meaning.
    if state.view.find(&payload.domain, &payload.card).is_some()
        && payload.parameters.is_object()
    {
        reject_unknown_load(
            &payload.parameters,
            descriptor::parameter_names(&payload.domain, &payload.card),
        )
        .map_err(DoorError::Unprocessable)?;
    }
    validate_physics(&payload.parameters).map_err(DoorError::Unprocessable)?;
    // A start that carries a run on arrives with `from`, and the fields
    // `from` carries are the upstream's. The card's own start rules are
    // named first: the start is opened, with no physics, when it is not
    // fresh, and to hold its anchors to the card's own values.
    if state.view.find(&payload.domain, &payload.card).is_some() {
        if let Some(parameters) = payload.parameters.as_object() {
            let (domain, card) = state.view.resolve(payload.domain.as_str(), payload.card.as_str());
            if let Err(sentence) = fresh_start::check(card, parameters, parameters, "parameters", "parameters", &carried) {
                Live::open(domain, card, &payload.parameters).map_err(DoorError::Unprocessable)?;
                return Err(DoorError::Unprocessable(sentence));
            }
            if fresh_start::has_anchors(card) {
                let live = Live::open(domain, card, &payload.parameters).map_err(DoorError::Unprocessable)?;
                fresh_start::check_anchors(card, &live, "parameters", &carried).map_err(DoorError::Unprocessable)?;
            }
        }
    }
    let proof = commit_request_proof(&request);

    let domain = payload.domain.clone();
    let card = payload.card.clone();
    let integrated = match state.view.resolve(domain.as_str(), card.as_str()) {
        ("materials", "ti64_lattice") => {
            let call =
                Ti64Call::from_parameters(&payload.parameters).map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "grid_node") => {
            let call =
                GridCall::from_parameters(&payload.parameters).map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "thermal") => {
            let call = ThermalCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("signal", "resonance") => {
            let call = ResonanceCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "swing") => {
            let call = SwingCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("signal", "tesseract") => {
            let call = TesseractCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("materials", "hydraulic_fluid") => {
            let call = HydraulicCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("materials", "cable_stretch") => {
            let call = CableCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "battery_sag") => {
            let call = BatteryCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "cell_18650pf") => {
            let call =
                CellCall::from_parameters(&payload.parameters).map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "cell_2170_m80") => {
            let call = Cell2170Call::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "generator_mmrtg_f1") => {
            let call = GeneratorCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("aerospace", "capsule_stardust") => {
            let call = CapsuleCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("aerospace", "capsule_insight") => {
            let call = CapsuleInsightCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("aerospace", "satellite_lageos1") => {
            let call = SatelliteCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("aerospace", "solar_system") => {
            let call = SolarSystemCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "reactor_spert3") => {
            let call = ReactorCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "grid_gb") => {
            let call = GridGbCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("signal", "cantilever_eth") => {
            let call = CantileverCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("signal", "link_gothenburg_10010") => {
            let call = LinkCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("aerospace", "los_jwst_fine_guide") => {
            let call = LosCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "cell_2170_model3") => {
            let call = Cell2170Model3Call::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("materials", "creep_cmsx6") => {
            let call = CreepCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "reactor_spert3_fuel") => {
            let call = ReactorFuelCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "reactor_spert3_ccore") => {
            let call = ReactorCcoreCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "reactor_triga_jsi") => {
            let call = ReactorTrigaCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("aerospace", "crater_ryugu_sci") => {
            let call = CraterCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("materials", "tire_eve_r19_ice") => {
            let call = TireCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "accumulator_sbo450") => {
            let call = AccumulatorCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "tokamak_jet_dte1") => {
            let call = TokamakCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("materials", "fingerpad_p02_glass") => {
            let call = FingerPadCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        _ => {
            return Err(DoorError::NotFound(
                "Card not recognized".to_string(),
            ));
        }
    };

    if !integrated.residual_conservation_error.is_finite() {
        return Err(DoorError::Unprocessable(
            "Physical Violation: residual_conservation_error is not finite.".to_string(),
        ));
    }
    if !frame_slots_are_finite(&integrated.frame) {
        return Err(DoorError::Unprocessable(
            "Physical Violation: the integrated frame has a non-finite slot. That state is not sealed.".to_string(),
        ));
    }
    let allowed = advisory_flags(domain.as_str(), card.as_str());
    if integrated.frame.flags & !allowed != 0 {
        if domain == "energy" && card == "grid_node" {
            return Err(DoorError::Unprocessable(
                "Physical Failure: the grid frame carries a failure flag other than surge. That state is not sealed.".to_string(),
            ));
        }
        return Err(DoorError::Unprocessable(
            "Physical Failure: the frame carries a failure flag. That state is not sealed."
                .to_string(),
        ));
    }

    seal_response(VerifyPhysicsResponse {
        inputs: request,
        engine_version: ENGINE_VERSION.to_string(),
        state: integrated.state,
        residual_conservation_error: integrated.residual_conservation_error,
        sha256_seal: sha256_seal(integrated.frame),
        chain: None,
        receipt_sha256: String::new(),
    })
    .map_err(|(_, message)| DoorError::Internal(message))
}

pub(crate) async fn verify_physics(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<Json<VerifyPhysicsResponse>, (StatusCode, String)> {
    verify_step(&state, payload)
        .await
        .map(Json)
        .map_err(door_status)
}

async fn run_trajectory_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<Json<trajectory::TrajectoryResponse>, (StatusCode, String)> {
    trajectory::run(&state, payload)
        .await
        .map(Json)
        .map_err(door_status)
}

async fn run_dataset_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    dataset::run(&state, payload)
        .await
        .map(json_text_response)
        .map_err(door_status)
}

async fn verify_dataset_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    dataset::check(&state, &payload)
        .await
        .map(json_text_response)
        .map_err(door_status)
}

/// An answer already written as JSON text, sent as it is.
fn json_text_response(text: String) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    ([(axum::http::header::CONTENT_TYPE, "application/json")], text)
}

async fn validate_series_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    validation::run(&state, payload)
        .await
        .map(json_text_response)
        .map_err(door_status)
}

async fn verify_tape_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    trajectory::verify(&state, &payload)
        .await
        .map(json_text_response)
        .map_err(door_status)
}

async fn solve_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<Json<solution::SolutionResponse>, (StatusCode, String)> {
    solution::run(&state, payload)
        .await
        .map(Json)
        .map_err(door_status)
}

async fn verify_solution_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    solution::verify(&state, &payload)
        .await
        .map(json_text_response)
        .map_err(door_status)
}

async fn list_solvers_http() -> Json<serde_json::Value> {
    Json(solvers::list_value())
}

async fn describe_solver_http(
    JsonBody(payload): JsonBody,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    solvers::describe_request(&payload)
        .map(Json)
        .map_err(door_status)
}

async fn list_cards_http() -> Json<serde_json::Value> {
    Json(descriptor::list_value())
}

async fn describe_card_http(
    JsonBody(payload): JsonBody,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    descriptor::describe_request(&payload)
        .map(Json)
        .map_err(door_status)
}

async fn verify_receipt_http(
    JsonBody(payload): JsonBody,
) -> Result<Json<receipt_check::ReceiptReport>, (StatusCode, String)> {
    receipt_check::check(&payload).map(Json).map_err(door_status)
}

async fn cite_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    cite::cite_in(state.view(), &payload).map(Json).map_err(door_status)
}

async fn find_flip_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    flip::find(&state, payload).await.map(json_text_response).map_err(door_status)
}

async fn find_model_http(
    State(state): State<AppState>,
    JsonBody(payload): JsonBody,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    find_model::find_in(state.view(), &payload).map(Json).map_err(door_status)
}

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub fn app() -> Router {
    router(AppState::new(physics_permit_count()))
}

/// The router with a directory for dataset files, or none. The directory
/// has been through [`prepare_dataset_dir`].
pub fn app_with_dataset_dir(dir: Option<PathBuf>) -> Router {
    router(AppState::new(physics_permit_count()).with_dataset_dir(dir))
}

/// Test-only router. The sleep exists solely so a unit test can fill
/// [`physics_permit_count`] and observe HTTP 429. `#[cfg(test)]` keeps the
/// hold out of the production binary. [`app`] does not sleep.
#[cfg(test)]
fn app_with_permit_hold(hold: Duration) -> Router {
    router(AppState::with_hold(physics_permit_count(), hold))
}

/// The router with a chosen number of physics permits, for measuring a
/// pool of one worker against the machine's own. Not part of the door.
#[doc(hidden)]
pub fn app_with_permits(permits: usize, dir: Option<PathBuf>) -> Router {
    router(AppState::new(permits).with_dataset_dir(dir))
}

const TRAJECTORY_BODY_LIMIT: usize = 16 * 1024 * 1024;

/// A JSON body that refuses a repeated object key before `serde_json` drops
/// it, and a non-zero number before `serde_json` reads it as zero.
struct JsonBody(serde_json::Value);

#[async_trait]
impl<S> FromRequest<S> for JsonBody
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        if !json_content_type(req.headers()) {
            return Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected request with `Content-Type: application/json`".to_string(),
            ));
        }
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|error| (error.status(), error.body_text()))?;
        match json_text::scan(&bytes) {
            json_text::Scan::Repeated(key) => {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    json_text::repeated_message(&key),
                ));
            }
            json_text::Scan::ReadsAsZero { field, token } => {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    json_text::reads_as_zero_message(&field, &token),
                ));
            }
            // An unreadable body is answered by the reader below, with its reason.
            json_text::Scan::Clean | json_text::Scan::Unreadable(_) => {}
        }
        match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(value) => Ok(JsonBody(value)),
            Err(error) => {
                let (status, prefix) = match error.classify() {
                    serde_json::error::Category::Data => (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "Failed to deserialize the JSON body into the target type",
                    ),
                    _ => (
                        StatusCode::BAD_REQUEST,
                        "Failed to parse the request body as JSON",
                    ),
                };
                Err((status, format!("{prefix}: {error}")))
            }
        }
    }
}

fn json_content_type(headers: &axum::http::HeaderMap) -> bool {
    let Some(value) = headers.get(axum::http::header::CONTENT_TYPE) else {
        return false;
    };
    let Ok(text) = value.to_str() else {
        return false;
    };
    let mime = text.split(';').next().unwrap_or("").trim();
    let mime = mime.to_ascii_lowercase();
    mime == "application/json" || mime.ends_with("+json")
}

fn router(state: AppState) -> Router {
    let wide = Router::new()
        .route("/run_trajectory", post(run_trajectory_http))
        .route("/verify_tape", post(verify_tape_http))
        .route("/run_dataset", post(run_dataset_http))
        .route("/verify_dataset", post(verify_dataset_http))
        .route("/verify_receipt", post(verify_receipt_http))
        .route("/validate_series", post(validate_series_http))
        .layer(DefaultBodyLimit::max(TRAJECTORY_BODY_LIMIT));
    Router::new()
        .route("/health", get(health_check))
        .route("/verify_physics", post(verify_physics))
        .route("/cards", get(list_cards_http))
        .route("/describe_card", post(describe_card_http))
        .route("/solvers", get(list_solvers_http))
        .route("/describe_solver", post(describe_solver_http))
        .route("/solve", post(solve_http))
        .route("/verify_solution", post(verify_solution_http))
        .route("/cite", post(cite_http))
        .route("/find_model", post(find_model_http))
        .route("/find_flip", post(find_flip_http))
        .merge(wide)
        .with_state(state)
}

#[cfg(test)]
mod armor {
    use super::*;

    /// A refusal the card names, and that is not a physical limit of the
    /// specimen, is given with what the card says it means.
    #[test]
    fn a_named_refusal_that_is_not_a_physical_limit_is_given_with_its_meaning() {
        assert_eq!(
            refused("grid_node", grid_node::StepReject::NonFiniteState),
            "Physical Violation: grid_node step refused (NonFiniteState): The incoming state, or the trial step, was non-finite."
        );
    }

    #[test]
    fn float_armor_rejects_non_finite_and_subnormal() {
        for poison in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from_bits(1),
        ] {
            let error = accept_f64("dt_s", poison).unwrap_err();
            assert!(
                error.contains("not a finite normal number"),
                "{poison:?} -> {error}"
            );
        }
        assert!(accept_f64("dt_s", 0.0).is_ok());
        assert!(accept_f64("dt_s", -0.0).is_ok());
        assert!(accept_f64("dt_s", 1.0e-6).is_ok());
        assert!(
            accept_f64("current_a", 1.0e39)
                .unwrap_err()
                .contains("frame slot")
        );

        let subnormal = serde_json::from_str::<serde_json::Value>(r#"{"dt_s":1e-320}"#).unwrap();
        assert!(
            validate_physics(&subnormal)
                .unwrap_err()
                .contains("not a finite normal number")
        );
        let nested =
            serde_json::from_str::<serde_json::Value>(r#"{"position_m":[0.0,5e-324,0.0]}"#)
                .unwrap();
        assert!(
            validate_physics(&nested)
                .unwrap_err()
                .contains("position_m[1]")
        );
        assert!(
            validate_physics(&serde_json::json!({"temperature_k": -50.0}))
                .unwrap_err()
                .contains("absolute zero")
        );
        // A temperature in degrees Celsius may be below zero; its card bounds
        // it. Any other temperature key still may not be negative.
        validate_physics(&serde_json::json!({"case_temperature_c": -20.0})).unwrap();
        validate_physics(&serde_json::json!({"load": {"case_temperature_c": -25.0}})).unwrap();
        for key in ["case_temperature", "temp", "temperature_c_k", "kelvin"] {
            assert!(
                validate_physics(&serde_json::json!({ key: -1.0 }))
                    .unwrap_err()
                    .contains("absolute zero"),
                "{key}"
            );
        }
    }

    #[test]
    fn a_long_decimal_is_the_nearest_f64() {
        let parsed: f64 = serde_json::from_str("233.14999999999998").unwrap();
        let nearest = f64::from_bits(233.15_f64.to_bits() - 1);
        assert_eq!(parsed.to_bits(), nearest.to_bits());
        assert_ne!(parsed.to_bits(), 233.15_f64.to_bits());

        let dt: f64 = serde_json::from_str("5.73168e-18").unwrap();
        assert_eq!(format!("{:016x}", dt.to_bits()), "3c5a6ec5fc45f08d");
    }

    #[test]
    fn the_sealed_form_sorts_keys_and_writes_each_number_as_its_bits() {
        let request: serde_json::Value = serde_json::from_str(
            r#"{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":10,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}"#,
        )
        .unwrap();
        let text = sealed_form(&request);
        assert_eq!(
            text,
            r#"{"card":"thermal","domain":"energy","dt_s":x3fb999999999999a,"initial":{"temperature_k":x4072526666666666},"record_every":x3ff0000000000000,"segments":[{"load":{"ambient_k":x4072526666666666,"heat_power_w":x3ff0000000000000},"steps":x4024000000000000}]}"#
        );
        assert_eq!(
            digest_hex(text.as_bytes()),
            "dced576688495b353ee304b7b8400f3fab60536a2deeb86932cb744039703da0"
        );
        assert_eq!(
            sealed_form(&serde_json::json!([null, true, false, "a\"b\n", -0.0])),
            r#"[null,true,false,"a\"b\n",x8000000000000000]"#
        );
        // A string is its JSON encoding. Keys are ordered by their UTF-8 bytes.
        assert_eq!(
            sealed_form(&serde_json::json!(
                "\u{0001}\u{001f}\u{0008}\t\n\u{000c}\r\\/\u{007f}\u{00e9}"
            )),
            "\"\\u0001\\u001f\\b\\t\\n\\f\\r\\\\/\u{007f}\u{00e9}\""
        );
        assert_eq!(
            sealed_form(&serde_json::json!({ "b": 1, "\u{00e9}": 2, "aa": 3, "a": 4, "B": 5 })),
            "{\"B\":x4014000000000000,\"a\":x4010000000000000,\"aa\":x4008000000000000,\"b\":x3ff0000000000000,\"\u{00e9}\":x4000000000000000}"
        );
    }

    #[test]
    fn one_number_has_one_sealed_form_however_it_is_written() {
        for (spellings, hex) in [
            (["0", "0.0", "0e0"], "0000000000000000"),
            (["-0", "-0.0", "-0e0"], "8000000000000000"),
            (["10", "10.0", "1e1"], "4024000000000000"),
            (["0.1", "0.10", "1e-1"], "3fb999999999999a"),
        ] {
            for spelling in spellings {
                let value: serde_json::Value = serde_json::from_str(spelling).unwrap();
                assert_eq!(sealed_form(&value), format!("x{hex}"), "{spelling}");
            }
        }
    }

    #[test]
    fn a_number_never_seals_the_same_as_a_string() {
        let number = sealed_form(&serde_json::json!(293.15));
        assert_eq!(number, "x4072526666666666");
        assert!(serde_json::from_str::<serde_json::Value>(&number).is_err());
        for other in [
            serde_json::json!("x4072526666666666"),
            serde_json::json!("293.15"),
            serde_json::json!("4072526666666666"),
            serde_json::json!([293.15]),
            serde_json::json!({ "x4072526666666666": 293.15 }),
            serde_json::json!(null),
            serde_json::json!(true),
        ] {
            let text = sealed_form(&other);
            assert_ne!(text, number, "{other}");
            assert!(!text.starts_with('x'), "{other} is sealed as {text}");
            let held = sealed_form(&serde_json::json!({ "temperature_k": other }));
            assert_ne!(held, r#"{"temperature_k":x4072526666666666}"#, "{other}");
        }
    }

    #[test]
    fn the_sealed_number_is_the_number_the_step_reads() {
        // 2^53 + 1 and 2^64 are whole numbers no f64 holds. The step reads
        // the nearest f64 in each case, and those are the bits that are sealed.
        for (token, hex) in [
            ("9007199254740992", "4340000000000000"),
            ("9007199254740993", "4340000000000000"),
            ("18446744073709551615", "43f0000000000000"),
            ("18446744073709551616", "43f0000000000000"),
            ("-9223372036854775809", "c3e0000000000000"),
        ] {
            let text = format!(r#"{{"heat_power_w":{token}}}"#);
            assert_eq!(json_text::scan(text.as_bytes()), json_text::Scan::Clean);
            let parameters: serde_json::Value =
                serde_json::from_str(&format!(r#"{{"heat_power_w":{token}}}"#)).unwrap();
            let read = required_f64(&parameters, "heat_power_w").unwrap();
            assert_eq!(format!("{:016x}", read.to_bits()), hex, "{token}");
            assert_eq!(
                sealed_form(&parameters),
                format!(r#"{{"heat_power_w":x{hex}}}"#),
                "{token}"
            );
        }
        let subnormal: serde_json::Value =
            serde_json::from_str(r#"{"heat_power_w":1e-320}"#).unwrap();
        assert!(required_f64(&subnormal, "heat_power_w").is_err());
        assert!(serde_json::from_str::<serde_json::Value>(r#"{"heat_power_w":1e400}"#).is_err());
        // A non-zero number under the smallest subnormal would be read as
        // zero. The door finds it in the text and nothing is read or sealed.
        assert_eq!(
            json_text::scan(br#"{"heat_power_w":1e-400}"#),
            json_text::Scan::ReadsAsZero {
                field: "heat_power_w".to_string(),
                token: "1e-400".to_string()
            }
        );
    }

    #[test]
    fn sampled_finite_values_have_a_token_this_parser_reads_back() {
        let mut state = 0x1234_5678_9abc_def0_u64;
        let mut checked = 0_u32;
        for _ in 0..4096 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let value = f64::from_bits(state);
            if !value.is_finite() {
                continue;
            }
            let token = exact_f64_token(value);
            assert!(
                token_keeps_bits(&token, value.to_bits()),
                "{value:?} token {token}"
            );
            checked += 1;
        }
        assert!(checked > 2000, "the sample was almost all non-finite");
        let hard = f64::from_bits(4570536701646012416);
        let token = exact_f64_token(hard);
        assert!(
            token_keeps_bits(&token, hard.to_bits()),
            "hard token {token}"
        );
    }

    #[test]
    fn the_card_floor_and_ceiling_are_the_only_limits_on_dt() {
        let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
        let omega = cell.max_natural_frequency_rad_s();
        let gate = cell.gate_dt_s().unwrap();
        assert!(gate < 1.0e-6, "the Ti-64 ceiling is finer than 1e-6 s");
        let floor = cell.floor_dt_s().unwrap();
        assert_eq!(floor.to_bits(), (gate / 1_048_576.0).to_bits());
        card_clock(gate, Some(omega), Some(floor), Some(gate), "ti64_lattice").unwrap();
        let above = f64::from_bits(gate.to_bits() + 1);
        assert_eq!(
            card_clock(above, Some(omega), Some(floor), Some(gate), "ti64_lattice").unwrap_err(),
            format!(
                "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the ti64_lattice card.",
                crate::message_number(above),
                crate::message_number(gate)
            )
        );
        assert!(
            card_clock(1.0, Some(omega), Some(floor), Some(gate), "ti64_lattice")
                .unwrap_err()
                .contains("the step ceiling of the ti64_lattice card")
        );
        card_clock(floor, Some(omega), Some(floor), Some(gate), "ti64_lattice").unwrap();
        let below = f64::from_bits(floor.to_bits() - 1);
        assert_eq!(
            card_clock(below, Some(omega), Some(floor), Some(gate), "ti64_lattice").unwrap_err(),
            format!(
                "Physical Violation: 'dt_s' {} is below {} s, the step floor of the ti64_lattice card.",
                crate::message_number(below),
                crate::message_number(floor)
            )
        );
        assert!(
            card_clock(1.0e-18, Some(omega), Some(floor), Some(gate), "ti64_lattice")
                .unwrap_err()
                .contains("the step floor of the ti64_lattice card")
        );
        assert!(
            card_clock(1.0e-320, Some(omega), Some(floor), Some(gate), "ti64_lattice").is_err()
        );

        // A card that states no ceiling or no floor is refused. No number
        // stands in for either.
        for missing in [None, Some(f64::NAN), Some(f64::INFINITY), Some(0.0), Some(-1.0)] {
            assert_eq!(
                card_clock(gate, Some(omega), Some(floor), missing, "ti64_lattice").unwrap_err(),
                "Physical Violation: the ti64_lattice card states no step ceiling.",
                "{missing:?}"
            );
            assert_eq!(
                card_clock(gate, Some(omega), missing, Some(gate), "ti64_lattice").unwrap_err(),
                "Physical Violation: the ti64_lattice card states no step floor.",
                "{missing:?}"
            );
        }

        // `π/ω` still holds where a card has a frequency, and only there.
        let past_nyquist = 4.0 / omega;
        assert!(
            card_clock(
                past_nyquist,
                Some(omega),
                Some(floor),
                Some(10.0 / omega),
                "ti64_lattice"
            )
            .unwrap_err()
            .contains("Nyquist limit")
        );
        card_clock(
            past_nyquist,
            None,
            Some(floor),
            Some(10.0 / omega),
            "ti64_lattice",
        )
        .unwrap();

        // The plate, the pack and the cell step up to their own ceilings,
        // all above 0.1 s.
        let plate = thermal::ThermalNode::avionics_cold_plate();
        let pack = battery_sag::LiIonPack::pack_12s();
        let single = cell_18650pf::Cell18650Pf::cell();
        for (card, omega, floor, ceiling) in [
            (
                "thermal",
                Some(plate.pole_rad_s()),
                plate.floor_dt_s().unwrap(),
                plate.gate_dt_s().unwrap(),
            ),
            (
                "battery_sag",
                Some(pack.pole_rad_s()),
                pack.floor_dt_s().unwrap(),
                pack.gate_dt_s().unwrap(),
            ),
            (
                "cell_18650pf",
                None,
                single.floor_dt_s().unwrap(),
                single.gate_dt_s().unwrap(),
            ),
        ] {
            assert!(ceiling > 1.0, "{card} {ceiling}");
            assert!(floor < 1.0e-4, "{card} {floor}");
            for dt_s in [floor, 0.1, 1.0, ceiling] {
                card_clock(dt_s, omega, Some(floor), Some(ceiling), card)
                    .unwrap_or_else(|error| panic!("{card} {dt_s}: {error}"));
            }
            let above = f64::from_bits(ceiling.to_bits() + 1);
            assert_eq!(
                card_clock(above, omega, Some(floor), Some(ceiling), card).unwrap_err(),
                format!(
                    "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the {card} card.",
                    crate::message_number(above),
                    crate::message_number(ceiling)
                )
            );
        }
    }

    #[test]
    fn advisory_flags_are_the_only_bits_that_still_seal() {
        assert_eq!(
            advisory_flags("energy", "grid_node"),
            grid_node::FLAG_GRID_SURGE
        );
        assert_eq!(
            advisory_flags("energy", "thermal"),
            thermal::FLAG_THERMAL_DERATE
        );
        assert_eq!(
            advisory_flags("signal", "resonance"),
            resonance::FLAG_STROKE_LIMIT
        );
        assert_eq!(advisory_flags("energy", "swing"), swing::FLAG_PLL_SLIP);
        assert_eq!(advisory_flags("signal", "tesseract"), 0);
        assert_eq!(
            advisory_flags("materials", "hydraulic_fluid"),
            hydraulic_fluid::FLAG_COLUMN_SOFT | hydraulic_fluid::FLAG_COLUMN_LIMIT
        );
        assert_eq!(
            advisory_flags("materials", "cable_stretch"),
            cable_stretch::FLAG_TRACKING_DEGRADE | cable_stretch::FLAG_CONTROL_LIMIT
        );
        assert_eq!(
            advisory_flags("energy", "battery_sag"),
            battery_sag::FLAG_SAG_WARNING
        );
        assert_eq!(
            advisory_flags("energy", "cell_18650pf"),
            cell_18650pf::FLAG_SAG_WARNING
        );
        assert_eq!(
            advisory_flags("energy", "generator_mmrtg_f1"),
            generator_mmrtg_f1::FLAG_PAST_FITTING_SPAN
        );
        assert_eq!(
            advisory_flags("aerospace", "capsule_stardust"),
            capsule_stardust::FLAG_DROGUE_MACH_WINDOW
        );
        assert_eq!(
            advisory_flags("aerospace", "capsule_insight"),
            capsule_insight::FLAG_LIFT_ACTING
        );
        assert_eq!(
            advisory_flags("aerospace", "satellite_lageos1"),
            satellite_lageos1::FLAG_EARTH_SHADOW
        );
        assert_eq!(
            advisory_flags("energy", "reactor_spert3"),
            reactor_spert3::FLAG_PROMPT_SUPERCRITICAL
        );
        assert_eq!(advisory_flags("energy", "grid_gb"), 0);
        assert_eq!(advisory_flags("signal", "cantilever_eth"), 0);
        assert_eq!(
            advisory_flags("signal", "link_gothenburg_10010"),
            link_gothenburg_10010::FLAG_RAINING
        );
        assert_eq!(
            advisory_flags("aerospace", "los_jwst_fine_guide"),
            los_jwst_fine_guide::FLAG_NO_MEASUREMENT
        );
        assert_eq!(
            advisory_flags("energy", "cell_2170_model3"),
            cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C
        );
        assert_eq!(
            advisory_flags("energy", "reactor_spert3_fuel"),
            reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL | reactor_spert3_fuel::FLAG_ROD_MOVING
        );
        assert_eq!(
            advisory_flags("aerospace", "crater_ryugu_sci"),
            crater_ryugu_sci::FLAG_STOPPED
        );
        assert_eq!(
            advisory_flags("materials", "tire_eve_r19_ice"),
            tire_eve_r19_ice::FLAG_LOAD_ABOVE_FITTED
        );
        assert_eq!(advisory_flags("energy", "cell_2170_m80"), 0);
        assert_eq!(
            advisory_flags("materials", "creep_cmsx6"),
            creep_cmsx6::FLAG_CREEP_RATE_RISING
        );
        assert_eq!(
            advisory_flags("energy", "tokamak_jet_dte1"),
            tokamak_jet_dte1::FLAG_NEAR_DENSITY_LIMIT
        );
        assert_eq!(advisory_flags("aerospace", "reentry_trajectory"), 0);
        assert_eq!(advisory_flags("materials", "ti64_lattice"), 0);
        assert_eq!(advisory_flags("energy", "battery_thermal_runaway"), 0);
    }

    #[tokio::test]
    async fn saturated_pool_is_429_and_releases() {
        let state = AppState::new(1);
        let held = acquire_physics_permit(&state).unwrap();
        let rejected = acquire_physics_permit(&state).err().unwrap();
        assert_eq!(rejected.0, StatusCode::TOO_MANY_REQUESTS);
        assert!(rejected.1.contains("Too many concurrent physics tasks"));
        drop(held);
        assert!(acquire_physics_permit(&state).is_ok());
    }

    #[test]
    fn post_step_state_keeps_f64_bits_and_rejects_non_finite() {
        let awkward = f64::from_bits(0x3ff0_0000_0000_0001);
        // serde_json needs 18 digits for this substrate sample. 17 drifts one ULP.
        let substrate = f64::from_bits(0x3fc6665d1dd9d99f);
        let state = state_object(&[
            ("temperature_k", StateField::F64(awkward)),
            ("pressure_pa", StateField::F64(substrate)),
            ("position_m", StateField::F64x3([awkward, 0.0, -0.0])),
            ("yielded", StateField::Bool(false)),
        ])
        .unwrap();
        let draft = VerifyPhysicsResponse {
            inputs: serde_json::json!({}),
            engine_version: "test".to_string(),
            state,
            residual_conservation_error: awkward,
            sha256_seal: "ab".repeat(32),
            chain: None,
            receipt_sha256: String::new(),
        };
        let response = seal_response(draft).expect("receipt covers the test response");
        let text = serde_json::to_string(&response).unwrap();
        let document = receipt_document(&response);
        assert_eq!(text, append_receipt(&document, &response.receipt_sha256));
        assert_eq!(response.receipt_sha256, digest_hex(document.as_bytes()));
        assert_ne!(response.receipt_sha256, response.sha256_seal);
        let mut forged = document.clone().into_bytes();
        let hex_at =
            document.find("\"temperature_k_hex\":\"").unwrap() + "\"temperature_k_hex\":\"".len();
        forged[hex_at] = if forged[hex_at] == b'0' { b'1' } else { b'0' };
        let forged = String::from_utf8(forged).unwrap();
        assert!(forged.contains(&format!("\"sha256_seal\":\"{}\"", response.sha256_seal)));
        assert_ne!(digest_hex(forged.as_bytes()), response.receipt_sha256);
        let parsed: VerifyPhysicsResponse = serde_json::from_str(&text).unwrap();
        assert_eq!(
            parsed.state["temperature_k"].as_f64().unwrap().to_bits(),
            awkward.to_bits()
        );
        assert_eq!(
            parsed.state["temperature_k_hex"].as_str().unwrap(),
            "3ff0000000000001"
        );
        assert_eq!(
            parsed.state["pressure_pa"].as_f64().unwrap().to_bits(),
            substrate.to_bits()
        );
        assert_eq!(
            parsed.state["pressure_pa_hex"].as_str().unwrap(),
            "3fc6665d1dd9d99f"
        );
        assert_eq!(
            parsed.state["position_m"][0].as_f64().unwrap().to_bits(),
            awkward.to_bits()
        );
        assert_eq!(
            parsed.state["position_m_hex"][2].as_str().unwrap(),
            "8000000000000000"
        );
        assert_eq!(format!("{:016x}", 293.15_f64.to_bits()), "4072526666666666");
        assert_eq!(
            parsed.residual_conservation_error.to_bits(),
            awkward.to_bits()
        );
        assert_eq!(parsed.receipt_sha256, response.receipt_sha256);
        let rejected = state_object(&[("pressure_pa", StateField::F64(f64::NAN))]).unwrap_err();
        assert!(rejected.contains("not finite"));
        assert!(rejected.contains("not sealed"));
    }
}

#[cfg(test)]
mod permit_probe;

#[cfg(test)]
mod trajectory_probe;

#[cfg(test)]
mod reactor_probe;

#[cfg(test)]
mod dataset_probe;

#[cfg(test)]
mod dataset_files_probe;

#[cfg(test)]
mod share_probe;

#[cfg(test)]
mod schema_probe;

#[cfg(test)]
mod solver_probe;

#[cfg(test)]
mod frame_probe;

#[cfg(test)]
mod impulse_probe;

#[cfg(test)]
mod description_probe;
