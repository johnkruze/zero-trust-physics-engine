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
    runaway_18650_lco,
    capsule_insight,
    crater_ryugu_sci,
    link_gothenburg_10010,
    grid_gb,
    solar_system,
    tire_eve_r19_ice,
    accumulator_sbo450,
    los_jwst_fine_guide,
    tokamak_jet_dte1,
    tokamak_vde_mast,
    fingerpad_p02_glass,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// One in-flight RK4 `spawn_blocking` task per core, hard-capped at 32.
/// A full pool fails closed. Excess calls never queue on the blocking pool.
const PHYSICS_PERMIT_CAP: usize = 32;


pub const ENGINE_VERSION: &str = "genesis_kernel_v0.12.0";

pub mod dataset;
mod descriptor;
pub mod json_text;
mod live;
mod card_reactor_msre_pump;
mod card_reactor_spert3_twin;
mod card_reactor_spert3_ccore_twin;
mod card_generator_mmrtg_seebeck;
mod card_momentum_jwst_dihedral;
mod card_grasp_lift_glass_4tu;
mod card_momentum_jwst_srp;
pub mod framing;
pub mod guard;
pub mod hosted;
pub mod keys;
pub mod mcp_server;
mod verify_by_request;
mod rebuild;
mod cite;
mod find_model;
mod spec_sheet;
mod succession;
mod flip;
mod design;
pub mod identity;
pub mod long_work;
mod open_plan;
mod chain;
mod couplings;
mod fresh_start;
mod verify_receipt;
mod solution;
mod solve_plan;
/// What the check of a whole solution says it compared, in every answer.
pub use solution::{WHOLE_SOLUTION_COMPARED, WHOLE_SOLUTION_COMPUTED};
pub use trajectory::{LONG_FORM_COMPARED, LONG_FORM_COMPUTED};
mod solver_ablator_slab;
mod solver_ablator_slab_stardust;
mod solver_argon_pair_gas;
mod solver_step_flow_armaly;
mod solver_pipe_flow_superpipe;
mod solver_fuel_rod_triga_jsi;
mod solver_culture_trichoderma_atp;
mod solver_junction_nist_b7_s2;
mod solver_water_hammer_oil_line;
mod solver_orifice_air_chamber;
mod solver_pneumatic_hose_resonance;
mod solver_blast_wave_tnt_air;
mod solver_stress_plate_hole_s235;
mod solver_stress_plate_hole_s235_elastic;
mod solver_icf_hotspot_burn;
mod solver_ln2_line_dlr_fttf;
mod solver_plasma_sheath_ram_c2;
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
mod signed_load;
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
    stored_verifications: Arc<Semaphore>,
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
            stored_verifications: Arc::new(Semaphore::new(1)),
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
            stored_verifications: Arc::new(Semaphore::new(1)),
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

/// The sentence for a current that is not a discharge.
fn cell_current_not_discharging() -> String {
    unsealed(
        "current is zero or negative, a rest or a charge: the card is a held discharge, with no charging limit at 4.2 V and no relaxation after a change of load (CurrentNotDischarging).",
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
    /// The load the run was opened with. Its reactivity is the one reactivity
    /// of the transient: a segment that carries another is refused. A segment
    /// may carry another flow.
    run_load: reactor_spert3_ccore::ReactorLoad,
    dt_s: f64,
}

struct RunawayCall {
    cell: runaway_18650_lco::Runaway18650Lco,
    state: runaway_18650_lco::RunawayState,
    load: runaway_18650_lco::RunawayLoad,
    dt_s: f64,
}

struct ReactorFuelCall {
    reactor: reactor_spert3_fuel::ReactorSpert3Fuel,
    state: reactor_spert3_fuel::ReactorState,
    load: reactor_spert3_fuel::ReactorLoad,
    /// The load the specimen was opened with: the one reactivity of the burst.
    /// A segment that carries another reactivity is refused; each segment may
    /// carry its own flow rate.
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
        None => return Err(format!("Missing required parameter: '{key}' (InputMissing).")),
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
        None => return Err(format!("Missing required parameter: '{key}' (InputMissing).")),
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

/// The number rule and the sign rules over every number of a request that
/// names no card, or whose card is not known yet.
#[cfg(test)]
fn validate_physics(params: &serde_json::Value) -> Result<(), String> {
    walk_physics(params, None, None)
}

/// The same over a request to `card`: a value a sign rule refuses is a value
/// of one of the card's own fields below what the card takes, and the
/// sentence carries the card's name for it.
pub(crate) fn validate_physics_of(card: Option<&str>, params: &serde_json::Value) -> Result<(), String> {
    walk_physics(params, None, card)
}

fn walk_physics(value: &serde_json::Value, key: Option<&str>, card: Option<&str>) -> Result<(), String> {
    match value {
        serde_json::Value::Number(number) => {
            let label = key.unwrap_or("value");
            let parsed = number.as_f64().ok_or_else(|| {
                format!("Physical Violation: '{label}' is not a finite normal number.")
            })?;
            accept_f64(label, parsed)?;
            if let Some(name) = key {
                apply_sign_rules(name, parsed, card)?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let path = match key {
                    Some(name) => format!("{name}[{index}]"),
                    None => format!("[{index}]"),
                };
                walk_physics(item, Some(&path), card)?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for (child, child_value) in map {
                walk_physics(child_value, Some(child), card)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The name `card` lists for a value of `key` below what it takes, in round
/// brackets with a space before it, or nothing when the card has none: a key
/// of a request that names no card, or a field the card publishes no minimum
/// for. `key` is a field's name, or `name[i]` for one component.
fn sign_rule_name(card: Option<&str>, key: &str) -> String {
    let field = key.split('[').next().unwrap_or(key);
    match card.and_then(|card| published_reject(card, field, false)) {
        Some(name) => format!(" ({name})"),
        None => String::new(),
    }
}

fn apply_sign_rules(key: &str, val: f64, card: Option<&str>) -> Result<(), String> {
    let lower = key.to_lowercase();
    // A key that ends in `_c` is a temperature in degrees Celsius, which is
    // below zero for a cold cell. Its card bounds it.
    let celsius = lower.ends_with("_c");
    if (lower.contains("kelvin") || lower.contains("temp")) && !celsius && val < 0.0 {
        return Err(match sign_rule_name(card, key) {
            name if name.is_empty() => format!(
                "Physical Violation: Temperature '{key}' cannot be below absolute zero (negative Kelvin)."
            ),
            name => format!(
                "Physical Violation: Temperature '{key}' cannot be below absolute zero, a negative number of kelvin{name}."
            ),
        });
    }
    if lower.contains("mass") && val < 0.0 {
        return Err(format!(
            "Physical Violation: Mass '{key}' cannot be negative{}.",
            sign_rule_name(card, key)
        ));
    }
    if lower.contains("density") && val < 0.0 {
        return Err(format!(
            "Physical Violation: Density '{key}' cannot be negative{}.",
            sign_rule_name(card, key)
        ));
    }
    if lower.contains("volume") && val < 0.0 {
        return Err(format!(
            "Physical Violation: Volume '{key}' cannot be negative{}.",
            sign_rule_name(card, key)
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
        // The bound of the force is the force that holds a ligament at yield
        // at rest. A start that says it stood under more is one no run of the
        // card writes, since no step takes such a force.
        ("ti64_lattice", "force_n", _) => "YieldReached",
        ("ti64_lattice", "start_force_n", _) => "StartNotWritten",
        // The bounds a landed step is held to, held at the start and in the
        // load under the same names.
        ("grid_node", "current_a" | "load_current_a", _) => "CurrentOutsideCard",
        ("grid_node", "voltage_v" | "upstream_voltage_v", _) => "VoltageOutsideCard",
        ("tesseract", "displacement_m" | "velocity_m_s", _) => "AmplitudeOutsideCard",
        // A stated ledger defect past the limit every step is held to: energy
        // above the anchor and the booked work, or below them.
        ("tesseract", "energy_defect_j", true) => "SpontaneousGeneration",
        ("tesseract", "energy_defect_j", false) => "FirstLawBreach",
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
        ("cell_18650pf", "current_a" | "held_current_a", _) => "CurrentNotDischarging",
        ("cell_2170_m80", "block_chamber_c", _) => "BlockNotRecognized",
        ("cell_2170_m80", "charge_removed_ah", _) => "ChargeOutsideTables",
        ("cell_2170_m80", "current_a", _) => "CurrentOutsideCard",
        ("cell_2170_m80", "case_temperature_c", _) => "CaseTemperatureOutsideCard",
        ("crater_ryugu_sci", "radius_m", false) => "RadiusBelowLedgerResolution",
        ("crater_ryugu_sci", "radius_m", true) => "RadiusAboveFinal",
        ("reactor_msre_pump", "pump_speed_at_start" | "pump_speed_fraction", _) => "FlowOutsideCard",
        ("reactor_spert3_twin", "inlet_temperature_k" | "system_pressure_mpa" | "coolant_flow_m_per_s" | "initial_power_mw", _) => "StateOutsideCard",
        ("reactor_spert3_twin", "reactivity_inserted_dollar", false) => "ReactivityNegative",
        ("reactor_spert3_ccore_twin", "initial_temperature_k", _) => "InitialTemperatureNotLiquidWater",
        ("reactor_spert3_ccore_twin", "system_pressure_psig", _) => "PressureOutsideSteamTables",
        ("reactor_spert3_ccore_twin", "reactivity_inserted_dollar", _) => "ReactivityNotPositive",
        ("reactor_spert3_ccore_twin", "coolant_flow_m_per_s", _) => "NegativeFlow",
        ("generator_mmrtg_seebeck", "mission_time_s", false) => "MissionTimeBeforeLanding",
        ("generator_mmrtg_seebeck", "mission_time_s", true) => "DecayHeatBelowLedgerResolution",
        ("momentum_jwst_dihedral", "q1" | "q2" | "q3" | "q4", _) => "QuaternionNotUnit",
        ("momentum_jwst_dihedral", "sun_x" | "sun_y" | "sun_z", _) => "SunDirectionNotUnit",
        ("momentum_jwst_dihedral", "sun_distance_au", false) => "SunDistanceNotPositive",
        ("grasp_lift_glass_4tu", "grip_force_n", false) => "NoContact",
        ("momentum_jwst_srp", "q1" | "q2" | "q3" | "q4", _) => "QuaternionNotUnit",
        ("momentum_jwst_srp", "sun_x" | "sun_y" | "sun_z", _) => "SunDirectionNotUnit",
        ("momentum_jwst_srp", "sun_distance_au", false) => "SunDistanceNotPositive",
        ("cell_2170_model3", "cell_number", _) => "CellNotRecognized",
        ("cell_2170_model3", "state_of_charge", _) => "StateOfChargeOutsideCard",
        ("cell_2170_model3", "capacity_ah", _) => "CapacityNotPositive",
        ("cell_2170_model3", "case_temperature_c", _) => "CaseTemperatureBelowAbsoluteZero",
        ("cell_2170_model3", "case_temperature_least_c" | "case_temperature_most_c", _) => {
            "CaseTemperatureBelowAbsoluteZero"
        }
        ("resonance", "displacement_m" | "velocity_m_s", _) => "EnergyAboveCard",
        ("resonance", "t_s", _) => "StartTimeOutsideCard",
        ("resonance", "energy_anchor_j", _) => "LedgerNotOfThisState",
        ("cantilever_eth", "deflection_m" | "velocity_m_s", _) => "EnvelopeAboveCard",
        ("creep_cmsx6", "creep_extension_percent", false) => "CreepExtensionBelowZero",
        ("creep_cmsx6", "creep_extension_percent", true) => "CreepExtensionOutsideCard",
        ("creep_cmsx6", "stress_mpa", _) => "StressOutsideCard",
        ("capsule_insight", "lift_over_drag", _) => "LiftOutsideCard",
        ("link_gothenburg_10010", "dry_path_loss_db", _) => "DryPathLossBelowFreeSpace",
        ("link_gothenburg_10010", "antenna_water_db", _) => "AntennaWaterOutsideCard",
        ("link_gothenburg_10010", "rain_rate_mm_h" | "gauge_minute_rain_mm_h", _) => {
            "NegativeRainRate"
        }
        ("link_gothenburg_10010", "gauge_minute_elapsed_s", _) => "GaugeMinuteOutsideCard",
        ("tire_eve_r19_ice", "distance_m", _) => "DistanceBelowZero",
        ("tire_eve_r19_ice", "slip_held_s", _) => "SlipHeldBelowZero",
        ("tire_eve_r19_ice", "normal_load_n", _) => "NoContact",
        ("tire_eve_r19_ice", "inflation_pressure_pa", _) => "TyreFlat",
        ("tire_eve_r19_ice", "slip_ratio", _) => "SlipOutsideCard",
        ("tire_eve_r19_ice", "carriage_speed_m_s", _) => "CarriageSpeedOutsideCard",
        ("accumulator_sbo450", "gas_mass_kg", _) => "GasMassNotPositive",
        ("accumulator_sbo450", "gas_pressure_pa", false) => "OilSideEmpty",
        ("accumulator_sbo450", "gas_pressure_pa", true) => "PressureAboveRated",
        ("accumulator_sbo450", "gas_temperature_k", _) => "GasBelowCriticalTemperature",
        ("accumulator_sbo450", "wall_temperature_k", _) => "WallTemperatureNotPositive",
        ("accumulator_sbo450", "work_moved_j" | "heat_moved_j", _) => "NegativeMovedSum",
        ("los_jwst_fine_guide", "lock_x_arcsec" | "lock_y_arcsec", _) => "LockOutsideSensorField",
        ("los_jwst_fine_guide", "quiet_before_start_s", _) => "QuietBeforeStartBelowZero",
        ("los_jwst_fine_guide", "measured", _) => "MeasuredNotZeroOrOne",
        ("los_jwst_fine_guide", "counts_per_s", _) => "CountRateNotPositive",
        ("hydraulic_fluid", "pressure_pa", false) => "Cavitation",
        ("hydraulic_fluid", "pressure_pa", true) => "PressureAboveCard",
        ("hydraulic_fluid", "flow_m3_s", _) => "FlowAboveAcoustic",
        ("generator_mmrtg_f1", "mission_time_s", false) => "MissionTimeBeforeLanding",
        ("generator_mmrtg_f1", "mission_time_s", true) => "DecayHeatBelowLedgerResolution",
        ("satellite_lageos1", "epoch_utc_s", _) => "EpochOutsideLeapSecondSpan",
        ("satellite_lageos1", "forecast_elapsed_s", _) => "ForecastElapsedBelowZero",
        ("grid_gb", "frequency_hz", false) => "LowFrequencyDemandDisconnection",
        ("grid_gb", "frequency_hz", true) => "FrequencyAboveCard",
        ("grid_gb", "inertia_mw_s", _) => "InertiaNotPositive",
        ("grid_gb", "demand_mw", _) => "DemandNegative",
        ("reactor_spert3", "coolant_temperature_k", _) => "CoolantTemperatureNotLiquidWater",
        ("reactor_triga_jsi", "start_temperature_k", _) => "StartTemperatureNotLiquidWater",
        ("reactor_triga_jsi", "rod_table_core", _) => "RodTableOutsideCard",
        ("reactor_triga_jsi", "reactivity_inserted_dollar", false) => "ReactivityNotPositive",
        ("reactor_triga_jsi", "reactivity_inserted_dollar", true) => "ReactivityAboveRodWorth",
        ("reactor_triga_jsi", "rods_dropped", _) => "ScramValueOutsideCard",
        ("reactor_spert3_ccore", "initial_temperature_k", _) => "InitialTemperatureNotLiquidWater",
        ("reactor_spert3_ccore", "system_pressure_psig", _) => "PressureNotPositive",
        ("reactor_spert3_ccore", "reactivity_inserted_dollar", _) => "ReactivityNotPositive",
        ("reactor_spert3_ccore", "coolant_flow_m_per_s", _) => "NegativeFlow",
        ("runaway_18650_lco", "temperature_k", false) => "TemperatureNotPositive",
        ("runaway_18650_lco", "temperature_k", true) => "CanMelting",
        ("runaway_18650_lco", "sei_fraction", _) => "SeiFractionOutsideCell",
        ("runaway_18650_lco", "anode_lithium_fraction", _) => "AnodeLithiumOutsideCell",
        ("runaway_18650_lco", "cathode_unreacted_fraction", _) => "CathodeUnreactedOutsideCell",
        ("runaway_18650_lco", "electrolyte_fraction", _) => "ElectrolyteOutsideCell",
        ("runaway_18650_lco", "oven_temperature_k", _) => "OvenTemperatureNotPositive",
        ("runaway_18650_lco", "heater_power_w", _) => "HeaterPowerNegative",
        ("runaway_18650_lco", "adiabatic", _) => "AdiabaticNotZeroOrOne",
        ("reactor_spert3_fuel", "inlet_temperature_k", _) => "InletTemperatureOutsideCard",
        ("reactor_spert3_fuel", "reactivity_inserted_dollar", false) => "ReactivityNotPositive",
        ("reactor_spert3_fuel", "coolant_flow_m_per_s", false) => "NegativeFlow",
        ("fingerpad_p02_glass", "twist_deg", _) => "TwistNotStartedFromZero",
        ("fingerpad_p02_glass", "normal_force_n", _) => "NoContact",
        ("fingerpad_p02_glass", "twist_rate_deg_s", _) => "TwistRateNotPositive",
        ("tokamak_jet_dte1", "thermal_energy_j" | "fast_ion_energy_j", _) => "EnergyOutsideCard",
        ("tokamak_jet_dte1", "elapsed_time_s", _) => "NegativeTime",
        ("tokamak_jet_dte1", "heating_power_w", _) => "HeatingPowerNegative",
        ("tokamak_jet_dte1", "ohmic_power_w", _) => "OhmicPowerNegative",
        ("tokamak_jet_dte1", "plasma_current_a", _) => "PlasmaCurrentNotPositive",
        ("tokamak_jet_dte1", "toroidal_field_t", _) => "ToroidalFieldNotPositive",
        ("tokamak_jet_dte1", "tritium_fraction", _) => "FuelMixOutsideCard",
        ("tokamak_jet_dte1", "line_density_per_m3", _) => "DensityNotPositive",
        ("tokamak_vde_mast", "displacement_m", _) => "StartOutsideVessel",
        ("tokamak_vde_mast", "velocity_m_s", _) => "SpeedAtAlfvenSpeed",
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
        // A range that runs the same distance either side of zero under one
        // name that speaks of size ("...Above..."): below its minimum a value
        // is above the card in size, and the sentence says so.
        let of_size = !upper
            && bound < 0.0
            && published_reject(card, field.name, false) == published_reject(card, field.name, true)
            && published_reject(card, field.name, false).is_some_and(|name| name.contains("Above"));
        let in_size = if of_size { ": larger in size than the card takes" } else { "" };
        unsealed(&format!(
            "'{label}' {}{unit} is {side} {}{unit}, the {how} {kind} of the {card} card{in_size}{name}.",
            message_number(value),
            message_number(bound),
        ))
    };
    if let Some(minimum) = field.minimum {
        if value < minimum || (field.minimum_exclusive && value == minimum) {
            return Some(outside(false, field.minimum_exclusive, minimum));
        }
    }
    // The time axis of the satellite publishes the last start from which a
    // step can land. The last date of the card itself is
    // still a legal start; the step from it leaves the card and the run is
    // sealed as refused. A value is held here to the end of the card.
    let end_of_card = match (card, field.name) {
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

/// The name of a step below the card's own step floor.
/// The name of a required input absent from a request, on every tool: nothing
/// is filled in.
pub const INPUT_MISSING: &str = "InputMissing";

/// The answer to a request that names a card the engine does not serve, and,
/// on the hosted connection, a card outside the key's list.
pub const CARD_NOT_RECOGNIZED: &str = "Card not recognized (UnknownCard).";

/// The answer to a request that names a solver the engine does not serve, and,
/// on the hosted connection, a solver outside the key's list.
pub const SOLVER_NOT_RECOGNIZED: &str = "Solver not recognized (UnknownSolver).";
/// The name of a solver's input given with a choice it does not go with: an
/// input taken only when another input holds one word.
pub const INPUT_NOT_WITH_CHOICE: &str = "InputNotWithChoice";

pub(crate) const DT_BELOW_FLOOR: &str = "DtBelowFloor";
/// The name of a step above the card's own step ceiling.
pub(crate) const DT_ABOVE_CEILING: &str = "DtAboveCeiling";
/// The name of a step above `π/ω` on a card whose ceiling lies above it.
pub(crate) const DT_ABOVE_NYQUIST: &str = "DtAboveNyquist";

/// The name a card gives a step outside its floor (`above` false) or its
/// ceiling (`above` true). The 2170 cell takes one step, 1 s, and its kernel
/// names every other step `StepNotOneSecond`, on either side.
pub(crate) fn step_outside_name(card: &str, above: bool) -> &'static str {
    match (card, above) {
        ("cell_2170_m80", _) => "StepNotOneSecond",
        (_, false) => DT_BELOW_FLOOR,
        (_, true) => DT_ABOVE_CEILING,
    }
}

/// `dt_s` must sit at or above the card's own step floor and at or under its
/// own step ceiling, `floor_dt_s` and `gate_dt_s` in `genesis_kernel`. The
/// gateway has no limit of its own. A card that states no floor or no ceiling
/// is refused. A card with a natural frequency or a pole also keeps `dt_s` at
/// or under `π/ω`. `omega_rad_s` is `None` for a card that has neither. A
/// step outside any of them is refused by name.
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
            "Physical Violation: 'dt_s' {} is below {} s, the step floor of the {card} card ({}).",
            message_number(dt_s),
            message_number(floor_dt_s),
            step_outside_name(card, false)
        ));
    }
    if dt_s > ceiling_dt_s {
        return Err(format!(
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the {card} card ({}).",
            message_number(dt_s),
            message_number(ceiling_dt_s),
            step_outside_name(card, true)
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
                "Physical Violation: 'dt_s' {} exceeds the Nyquist limit {} s of the {card} card ({DT_ABOVE_NYQUIST}).",
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
        resonance::StepReject::EnergyAboveCard => unsealed(
            "the mount's mechanical energy is above that of a free ring of twice the stroke, four stroke energies: a displacement or a velocity of either sign larger than the card takes (EnergyAboveCard).",
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

/// A load rule's sentence with the name its card lists for the rule, in
/// round brackets at the end.
fn named(sentence: String, name: &str) -> String {
    format!("{} ({name}).", sentence.trim_end_matches('.'))
}

/// The load rule of a card with one scalar load, refused under the name the
/// card lists for it: `|value| * dt_s` at or under the card's constant.
fn load_nyquist(
    name: &str,
    key: &str,
    value: f64,
    dt_s: f64,
    impulse: f64,
    limit: Option<f64>,
) -> Result<(), String> {
    enforce_abs_nyquist(key, value, dt_s, impulse, limit).map_err(|text| named(text, name))
}

/// The current Nyquist limit of the pack at the door, refused under the
/// name the card lists for it.
fn battery_nyquist(pack: &battery_sag::LiIonPack, current_a: f64, dt_s: f64) -> Result<(), String> {
    load_nyquist(
        "CurrentAboveNyquist",
        "current_a",
        current_a,
        dt_s,
        pack.charge_impulse_c(),
        pack.current_nyquist_a(dt_s),
    )
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
        cell_18650pf::StepReject::CurrentNotDischarging => cell_current_not_discharging(),
        cell_18650pf::StepReject::CurrentChangedDuringDischarge => unsealed(
            "the current is more than 0.83 mA from the held current of this run (CurrentChangedDuringDischarge).",
        ),
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

/// Whether a start lies on the tables of its block, both ends included. A
/// start one 64-bit number past an end is refused: no start is read at an
/// end point it is not at.
fn cell_2170_start_on_tables(charge_removed_ah: f64, block: usize) -> bool {
    let cell = cell_2170();
    charge_removed_ah >= cell.charge_top_ah[block] && charge_removed_ah <= cell.charge_deep_ah[block]
}

/// The sentence for a start outside the tables of its own block.
fn cell_2170_charge_outside_block(charge_removed_ah: f64, block: usize) -> String {
    let cell = cell_2170();
    unsealed(&format!(
        "'charge_removed_ah' {} Ah is outside [{}, {}] Ah, the tables of the {} degC block of the cell_2170_m80 card, both ends included, and no voltage is read outside them (ChargeOutsideTables).",
        message_number(charge_removed_ah),
        message_number(cell.charge_top_ah[block]),
        message_number(cell.charge_deep_ah[block]),
        message_number(cell.block_chamber_c[block]),
    ))
}

const CELL_2170_CURRENT_OUTSIDE_CARD: &str =
    "the current is above 13.5 A charging or discharging, the end of the ten-second table's current axis (CurrentOutsideCard).";
const CELL_2170_CASE_OUTSIDE_CARD: &str =
    "case temperature is outside [-21.898, 45.535] degC, beyond the coldest and the warmest block's own tests, where the tables would hold their end values (CaseTemperatureOutsideCard).";

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

const CELL_2170_MODEL3_BELOW_ABSOLUTE_ZERO: &str =
    "case temperature is at or below absolute zero, -273.15 degC, where the temperature laws have no value (CaseTemperatureBelowAbsoluteZero).";

/// The sentence for a start whose least case temperature is above its most.
fn cell_2170_model3_span_inverted(least_c: f64, most_c: f64) -> String {
    unsealed(&format!(
        "the start carries {} to {} degC of case temperature, a least above its most (CaseTemperatureSpanInverted).",
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
            "the state of charge would leave [-0.05, 1.05], the ends of the card's rest-voltage curves, outside which it has no rest voltage, by more than the rounding allowance of its running sum (StateOfChargeOutsideCard).",
        ),
        StepReject::CaseTemperatureSpanInverted => unsealed(
            "the least case temperature the state carries is above the most it carries (CaseTemperatureSpanInverted).",
        ),
        StepReject::SurfaceOutsideCurve => unsealed(
            "the state of charge at the particle surface would leave [-0.05, 1.05], the ends of the card's rest-voltage curves, outside which it has no rest voltage (SurfaceOutsideCurve).",
        ),
        StepReject::CaseTemperatureBelowAbsoluteZero => unsealed(CELL_2170_MODEL3_BELOW_ABSOLUTE_ZERO),
        StepReject::CapacityNotPositive => {
            unsealed("the capacity is zero or less (CapacityNotPositive).")
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
        crater_ryugu_sci::StepReject::RadiusBelowLedgerResolution => {
            unsealed("the crater's radius is under 0.1622 m, where one rounding of the energy the wall has yet to give up passes a tenth of the ledger's limit (RadiusBelowLedgerResolution).")
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
        generator_mmrtg_f1::StepReject::MissionTimeBeforeLanding => unsealed(
            "mission time is before landing on Mars (MissionTimeBeforeLanding).",
        ),
        generator_mmrtg_f1::StepReject::DecayHeatBelowLedgerResolution => unsealed(
            "the decay heat the fuel still holds is under the ledger's resolution, about 2,070 years after landing (DecayHeatBelowLedgerResolution).",
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

/// The sentence for a first step in reactivity of zero or less.
fn reactor_reactivity_not_positive() -> String {
    unsealed(
        "the reactivity of the first step is zero or less: no step in reactivity is inserted, and there is no burst (ReactivityNotPositive).",
    )
}

/// The sentence for a coolant that is not liquid water, from the card's own bounds.
fn reactor_coolant_not_liquid_water() -> String {
    unsealed(&format!(
        "the primary coolant temperature is outside [{}, {}] K: below it water freezes, and above it, the critical point of water, there is no liquid at any pressure (CoolantTemperatureNotLiquidWater).",
        reactor_spert3::WATER_FREEZING_K,
        reactor_spert3::WATER_CRITICAL_K
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

/// The sentence for a creep extension below zero.
fn creep_extension_below_zero() -> String {
    unsealed(
        "the creep extension is below zero: a test piece in tension does not shorten by creep (CreepExtensionBelowZero).",
    )
}

/// The upper limit of creep extension, as every sentence about it says it.
fn creep_extension_limits() -> String {
    format!(
        "{} percent, the largest level of the protocol",
        creep_cmsx6::CREEP_EXTENSION_MAX_PERCENT
    )
}

/// The sentence for a starting creep extension above the limit.
fn creep_extension_outside_card() -> String {
    unsealed(&format!(
        "the creep extension is above {} (CreepExtensionOutsideCard).",
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
        creep_cmsx6::StepReject::CreepExtensionBelowZero => creep_extension_below_zero(),
        creep_cmsx6::StepReject::StressNotHeld => creep_stress_not_held(),
        creep_cmsx6::StepReject::CreepExtensionBeyondCard => unsealed(&format!(
            "the step would pass the limit of creep extension, {} (CreepExtensionBeyondCard).",
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
    start: (f64, f64),
    dt_s: f64,
) -> String {
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
        tokamak_jet_dte1::StepReject::DtAboveStateCeiling => {
            let (thermal_energy_j, fast_ion_energy_j) = start;
            let ceiling_s = tokamak.state_ceiling_dt_s(thermal_energy_j, fast_ion_energy_j, load);
            let where_s = if dt_s > ceiling_s {
                format!("the step ceiling of the state it starts from, {} s", message_number(ceiling_s))
            } else {
                format!(
                    "the step ceiling of the state where its trial step lands; at the state it starts from the ceiling is {} s",
                    message_number(ceiling_s)
                )
            };
            unsealed(&format!(
                "the step of {} s is above {where_s}: the smallest of an eighteenth of the slowing-down time, a forty-sixth of 0.31 times the confinement time and a forty-eighth of the coupling time of the fast ions and the thermal plasma (DtAboveStateCeiling).",
                message_number(dt_s),
            ))
        }
        tokamak_jet_dte1::StepReject::SpontaneousGeneration => {
            unsealed("the plasma created energy (SpontaneousGeneration).")
        }
        tokamak_jet_dte1::StepReject::FirstLawBreach => {
            unsealed("the plasma destroyed energy (FirstLawBreach).")
        }
        other => refused("tokamak_jet_dte1", other),
    }
}

fn reactor_reject(reject: reactor_spert3::StepReject) -> String {
    match reject {
        reactor_spert3::StepReject::ReactivityNotPositive => reactor_reactivity_not_positive(),
        reactor_spert3::StepReject::CoolantTemperatureNotLiquidWater => {
            reactor_coolant_not_liquid_water()
        }
        reactor_spert3::StepReject::PastFirstMaximum => unsealed(
            "the reactor is past the first maximum of power; nothing after it is on the card (PastFirstMaximum).",
        ),
        reactor_spert3::StepReject::DtAboveStateCeiling => unsealed(&format!(
            "the step is longer than {} of the time constant of the fastest pole at the state it starts from or would land on, or {} of it for a growing pole: the prompt pole |1 - net reactivity| / l* or the feedback pole, the square root of a P / l* (DtAboveStateCeiling).",
            reactor_spert3::GATE_PHASE_STEP,
            reactor_spert3::GROWTH_PHASE_STEP
        )),
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

/// The starts compared on each rod table, in words: "297.15 to 301.09 K (24.00 to 27.94 degC) on core 215, ...".
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
        reactor_triga_jsi::StepReject::StartTemperatureNotLiquidWater => unsealed(&format!(
            "the starting temperature, which is also the pool water's, is below {} K, where water freezes, or above {} K, where it boils at one standard atmosphere (StartTemperatureNotLiquidWater).",
            message_number(reactor_triga_jsi::WATER_FREEZING_K),
            message_number(reactor_triga_jsi::WATER_BOILING_K)
        )),
        reactor_triga_jsi::StepReject::RodTableOutsideCard => unsealed(&format!(
            "the rod-worth table is not that of core {} (RodTableOutsideCard).",
            reactor_triga_tables_in_words().replace(" and ", " or ")
        )),
        reactor_triga_jsi::StepReject::ReactivityNotPositive => unsealed(
            "the reactivity inserted is zero or less: no step in reactivity is inserted (ReactivityNotPositive).",
        ),
        reactor_triga_jsi::StepReject::ReactivityAboveRodWorth => unsealed(&format!(
            "the reactivity inserted is above {} printed dollars, the transient rod's whole worth (ReactivityAboveRodWorth).",
            message_number(reactor_triga_jsi::ROD_WORTH_DOLLAR)
        )),
        reactor_triga_jsi::StepReject::ScramValueOutsideCard => unsealed(
            "rods_dropped is neither 0 (rods held) nor 1 (rods dropped) (ScramValueOutsideCard).",
        ),
        reactor_triga_jsi::StepReject::ReactivityChangedDuringRun => unsealed(
            "the reactivity inserted differs from the one this run started with; the model defines no change of it but the scram (ReactivityChangedDuringRun).",
        ),
        reactor_triga_jsi::StepReject::RodsRaisedAfterScram => {
            unsealed("the rods were raised after the scram (RodsRaisedAfterScram).")
        }
        reactor_triga_jsi::StepReject::FuelTemperatureAboveFuelLimit => unsealed(&format!(
            "the core-average fuel temperature is above {} K (1150 degC), the temperature limit of U-ZrH1.6 fuel in stainless clad (FuelTemperatureAboveFuelLimit).",
            reactor_triga_jsi::FUEL_TEMPERATURE_LIMIT_K
        )),
        reactor_triga_jsi::StepReject::DtAboveStateCeiling => unsealed(&format!(
            "the step is longer than {} of the time constant of the fastest pole at the state it starts from or where it lands (DtAboveStateCeiling).",
            message_number(reactor_triga_jsi::STATE_PHASE_STEP)
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

/// The sentence for a reactivity of zero or less.
fn reactor_fuel_reactivity_not_positive(reactivity_inserted_dollar: f64) -> String {
    unsealed(&format!(
        "'reactivity_inserted_dollar' {} dollar is zero or less: the transient rod inserts no reactivity, and there is no burst (ReactivityNotPositive).",
        message_number(reactivity_inserted_dollar),
    ))
}

/// The sentence for a coolant flow below zero.
fn reactor_fuel_flow_negative(coolant_flow_m_per_s: f64) -> String {
    unsealed(&format!(
        "'coolant_flow_m_per_s' {} m/s is below zero: the flow runs through the core from the inlet (NegativeFlow).",
        message_number(coolant_flow_m_per_s),
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


/// The sentence for a coolant flow below zero on the C-core card.
fn reactor_ccore_negative_flow(coolant_flow_m_per_s: f64) -> String {
    unsealed(&format!(
        "'coolant_flow_m_per_s' {} m/s is below zero on the reactor_spert3_ccore card (NegativeFlow).",
        message_number(coolant_flow_m_per_s),
    ))
}

/// The sentence for a reactivity of zero or less on the C-core card.
fn reactor_ccore_reactivity_not_positive(reactivity_inserted_dollar: f64) -> String {
    unsealed(&format!(
        "'reactivity_inserted_dollar' {} dollar is zero or less on the reactor_spert3_ccore card: no step in reactivity is inserted, and there is no burst (ReactivityNotPositive).",
        message_number(reactivity_inserted_dollar),
    ))
}

/// The sentence for a start at a pressure at or below absolute zero, or one that is not liquid water.
fn reactor_ccore_start_not_liquid(initial_temperature_k: f64, system_pressure_psig: f64) -> String {
    let reactor = reactor_spert3_ccore::ReactorSpert3Ccore::c_core_step_transients();
    if !reactor.pressure_is_positive(system_pressure_psig) {
        unsealed(&format!(
            "'system_pressure_psig' {} psig is at or below absolute zero, -{} psig, on the reactor_spert3_ccore card (PressureNotPositive).",
            message_number(system_pressure_psig),
            message_number(reactor_spert3_ccore::ATMOSPHERE_PSI),
        ))
    } else {
        let boiling = reactor.saturation_temperature_k(system_pressure_psig);
        let upper = if boiling.is_finite() && boiling > reactor_spert3_ccore::FREEZING_K {
            format!("{} K, the saturation temperature of water at {} psig (IAPWS-IF97 region 4; the critical temperature above the critical pressure)", message_number(boiling), message_number(system_pressure_psig))
        } else {
            format!("the freezing point: at {} psig water boils below 273.15 K, so no start is liquid", message_number(system_pressure_psig))
        };
        unsealed(&format!(
            "'initial_temperature_k' {} K is not liquid water on the reactor_spert3_ccore card: a start is at or above {} K and below {} (InitialTemperatureNotLiquidWater).",
            message_number(initial_temperature_k),
            message_number(reactor_spert3_ccore::FREEZING_K),
            upper,
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

/// The sentence for a step longer than the state ceiling, with the longest step the state the
/// step started from allows.
fn reactor_ccore_state_ceiling(dt_s: f64, longest_s: Option<f64>) -> String {
    let longest = match longest_s {
        Some(longest) if longest >= dt_s => format!(
            "the state the step started from allows {} s, and its trial end less",
            message_number(longest)
        ),
        Some(longest) => format!("the state the step started from allows {} s", message_number(longest)),
        None => "the state the step started from allows no step".to_string(),
    };
    unsealed(&format!(
        "the step of {} s is longer than the state lets it be: {longest}. Past the compared range the step ceiling follows the state, the growing prompt pole held to 0.0611, the decaying prompt pole to 0.5 and the plate and water heat transfer and sweep to 0.0184 of a step (DtAboveStateCeiling).",
        message_number(dt_s),
    ))
}

fn runaway_reject(reject: runaway_18650_lco::StepReject) -> String {
    match reject {
        runaway_18650_lco::StepReject::EveryReactantConsumed => unsealed(
            "the step would carry the cell to every reactant consumed: the heat the lithium in the SEI and in the graphite, the unreacted cathode and the electrolyte still hold falls to 1e-6 of the cell's chemical heat or under, 0.0205 J, and the runaway has finished (EveryReactantConsumed).",
        ),
        runaway_18650_lco::StepReject::CanMelting => unsealed(
            "the cell temperature, incoming or in a substep, is at or above 1811 K, the melting point of iron: the steel can melts (CanMelting).",
        ),
        runaway_18650_lco::StepReject::SubstepsAboveBudget => unsealed(
            "the step would need more than 1,048,576 substeps to follow its fastest reaction (SubstepsAboveBudget).",
        ),
        runaway_18650_lco::StepReject::SpontaneousGeneration => unsealed(
            "the heat stored in the cell rose above the heat released, plus the heater's, less the heat lost (SpontaneousGeneration).",
        ),
        runaway_18650_lco::StepReject::FirstLawBreach => unsealed(
            "the heat stored in the cell fell below the heat released, plus the heater's, less the heat lost (FirstLawBreach).",
        ),
        other => refused("runaway_18650_lco", other),
    }
}

fn reactor_ccore_reject(reject: reactor_spert3_ccore::StepReject) -> String {
    match reject {
        reactor_spert3_ccore::StepReject::PastPlateTemperatureMaximum => unsealed(
            "the reactor is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card (PastPlateTemperatureMaximum).",
        ),
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
        reactor_spert3_fuel::StepReject::ReactivityNotPositive => unsealed(
            "the reactivity inserted is zero or less: the transient rod inserts no reactivity, and there is no burst (ReactivityNotPositive).",
        ),
        reactor_spert3_fuel::StepReject::NegativeFlow => unsealed(
            "the coolant flow rate is below zero: the flow runs through the core from the inlet (NegativeFlow).",
        ),
        reactor_spert3_fuel::StepReject::PastFirstMaximum => unsealed(
            "the reactor is past the first maximum of power; nothing after it is on the card (PastFirstMaximum).",
        ),
        reactor_spert3_fuel::StepReject::DtAboveStateCeiling => unsealed(&format!(
            "the step is longer than {} of the time constant of the fastest pole at the state it starts from or would land on, or {} of it for a growing pole: the prompt pole |1 - net reactivity| / l*, the feedback pole of the fuel and the water temperature, or the sweep 2 v / L of the water (DtAboveStateCeiling).",
            reactor_spert3_fuel::GATE_PHASE_STEP,
            reactor_spert3_fuel::GROWTH_PHASE_STEP
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
fn grid_gb_reject(reject: grid_gb::StepReject) -> String {
    match reject {
        grid_gb::StepReject::InertiaNotPositive => unsealed(
            "the stored kinetic energy at 50 Hz is zero or less (InertiaNotPositive).",
        ),
        grid_gb::StepReject::DemandNegative => unsealed("the demand is below zero (DemandNegative)."),
        grid_gb::StepReject::LossLargerThanDemand => unsealed(
            "the power lost is larger in size than the demand the area carries (LossLargerThanDemand).",
        ),
        grid_gb::StepReject::NegativeTime => unsealed("the clock is below zero (NegativeTime)."),
        grid_gb::StepReject::LowFrequencyDemandDisconnection => unsealed(
            "the frequency is at or below 48.8 Hz, where low-frequency demand disconnection begins (LowFrequencyDemandDisconnection).",
        ),
        grid_gb::StepReject::FrequencyAboveCard => unsealed(
            "the frequency is above 50.5 Hz, the upper statutory limit, where the card holds no over-frequency response (FrequencyAboveCard).",
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
    "altitude is above 134.7 km, the top of the drag table read by altitude (AltitudeOutsideCard).";
const CAPSULE_BELOW_STANDARD_SEA_LEVEL: &str = "altitude is below 0 m, the sea level of the 1976 standard's air and the ellipsoid under the path: the capsule is on or under the ground (BelowStandardSeaLevel).";
const CAPSULE_MACH_OUTSIDE_CARD: &str = "the Mach number is below 0.16, the first entry of the drag table read by Mach number, or is above 40 below 68.96 km, where that table ends (MachOutsideCard).";
const CAPSULE_AIR_SPEED_OUTSIDE_CARD: &str = "above 68.96 km the speed through the air is below 11900 m/s or above 12620 m/s, the highest speed the drag table by altitude was computed at (AirSpeedOutsideCard).";
const CAPSULE_PAST_FLIGHT_DEPLOYMENT: &str = "the state is under 31030 m and under Mach 1.23 at once, past the flown drogue deployment, where the capsule flew on a parachute (PastFlightDeployment).";

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
        capsule_stardust::StepReject::BelowStandardSeaLevel => {
            unsealed(CAPSULE_BELOW_STANDARD_SEA_LEVEL)
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
    "altitude is above 128 km (AltitudeOutsideCard).";
const CAPSULE_INSIGHT_BELOW_LANDING_SITE: &str = "altitude is below -2634.5 m, the landing site's radius of 3393079.5 m: the capsule is at or under the ground there (BelowLandingSite).";
const CAPSULE_INSIGHT_MACH_OUTSIDE_CARD: &str = "the Mach number is below 0.9, the first entry of the drag table read by Mach number (MachOutsideCard).";
const CAPSULE_INSIGHT_AIR_SPEED_OUTSIDE_CARD: &str =
    "the speed through the air is above 5850 m/s (AirSpeedOutsideCard).";
const CAPSULE_INSIGHT_LIFT_OUTSIDE_CARD: &str =
    "the lift over drag is above 0.08 or below -0.08 (LiftOutsideCard).";
const CAPSULE_INSIGHT_PAST_FLIGHT_DEPLOYMENT: &str = "the state is under 6164.62 m and under 352.1169 m/s through the air at once, past the flown parachute deployment, the last row of the flight's record, where the capsule flew on its parachute (PastFlightDeployment).";

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
        capsule_insight::StepReject::PastFlightDeployment => {
            unsealed(CAPSULE_INSIGHT_PAST_FLIGHT_DEPLOYMENT)
        }
        capsule_insight::StepReject::BelowLandingSite => {
            unsealed(CAPSULE_INSIGHT_BELOW_LANDING_SITE)
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

const CANTILEVER_ENVELOPE_ABOVE_CARD: &str = "the deflection envelope, sqrt(deflection_m^2 + (velocity_m_s / omega0)^2), a size, is above 0.01875 m: a deflection or a speed of either sign larger than the card takes. 0.01875 m is 3.119 V x 0.006012 m/V, the largest amplitude the softening and the damping were fitted on; above it those laws were not fitted, and the strip's own limit, its yield at the clamp, needs its alloy, which the dataset does not state (EnvelopeAboveCard).";

const LOS_COUNT_RATE_NOT_POSITIVE: &str = "the guide star's count rate on a measured frame is zero or below, or under about 7e-237 counts per second, where the noise variance of its centroid passes the largest 64-bit number: the noise law has no value at zero, and a centroid with no light holds no position (CountRateNotPositive).";
const LOS_OFFSET_OUTSIDE_CARD: &str = "the centroid is more than 250 mas from the lock point on x or on y (OffsetOutsideCard).";
const LOS_QUIET_BELOW_ZERO: &str = "the stated quiet_before_start_s, the time since the last disturbed centroid before the run, is below zero (QuietBeforeStartBelowZero).";
const LOS_LOCK_OUTSIDE_SENSOR_FIELD: &str = "the lock point is more than 70 arcsec from the centre of the sensor's ideal frame on x or on y: the guider's field is 2.3 by 2.3 arcmin, and no star can be held beyond it (LockOutsideSensorField).";
const LOS_MEASURED_NOT_ZERO_OR_ONE: &str = "Physical Violation: 'measured' is neither 0 nor 1 (MeasuredNotZeroOrOne) on the los_jwst_fine_guide card.";
const LOS_UNMEASURED_CARRIES_NUMBERS: &str = "Physical Violation: a frame with 'measured' 0 carries a centroid or a count rate that is not zero (UnmeasuredFrameCarriesNumbers) on the los_jwst_fine_guide card.";

fn los_reject(reject: los_jwst_fine_guide::StepReject) -> String {
    match reject {
        los_jwst_fine_guide::StepReject::CountRateNotPositive => {
            unsealed(LOS_COUNT_RATE_NOT_POSITIVE)
        }
        los_jwst_fine_guide::StepReject::OffsetOutsideCard => unsealed(LOS_OFFSET_OUTSIDE_CARD),
        los_jwst_fine_guide::StepReject::QuietBeforeStartBelowZero => unsealed(LOS_QUIET_BELOW_ZERO),
        los_jwst_fine_guide::StepReject::LockOutsideSensorField => {
            unsealed(LOS_LOCK_OUTSIDE_SENSOR_FIELD)
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
/// zeros, a measured frame has a count rate above zero whose noise variance
/// is a number and its centroid within the card's reach of the lock point.
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
        if !card.count_rate_has_noise(counts_per_s) {
            return Err(unsealed(LOS_COUNT_RATE_NOT_POSITIVE));
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
        cantilever_eth::StepReject::EnvelopeAboveCard => {
            unsealed(CANTILEVER_ENVELOPE_ABOVE_CARD)
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
        link_gothenburg_10010::StepReject::NegativeRainRate => {
            unsealed("the rain rate is below zero: rain does not fall upward (NegativeRainRate).")
        }
        link_gothenburg_10010::StepReject::GaugeMinuteOutsideCard => unsealed(
            "the seconds already spent in the current gauge minute are below zero or not below 60 s (GaugeMinuteOutsideCard).",
        ),
        link_gothenburg_10010::StepReject::DryPathLossBelowFreeSpace => unsealed(
            "the dry level is below 44.32 dB, the free-space loss of this path less the gain of two ideal 0.3 m apertures: no pair of these antennas has that much gain (DryPathLossBelowFreeSpace).",
        ),
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
const TIRE_DISTANCE_BELOW_ZERO: &str = "the distance along the track is below zero (DistanceBelowZero).";
const TIRE_SLIP_HELD_BELOW_ZERO: &str = "the time the slip has been held, stated at the start or carried in the state, is below zero: a slip is held from the instant it is set (SlipHeldBelowZero).";
const TIRE_STEP_TOO_SMALL: &str = "the step does not move the time held at the slip, or the distance, in 64-bit arithmetic (StepTooSmallToResolve).";

const FINGERPAD_TWIST_BELOW_ZERO: &str = "the twist is below zero: the twist is of the plate turning forward from the untwisted pad (TwistBelowZero).";
const FINGERPAD_TWIST_NOT_FROM_ZERO: &str = "the twist at the start is not zero: the card's closed forms are of a pad twisted from zero under the load it holds, and a pad already twisted holds springs set by the load and the path it was brought there under, which a start does not carry (TwistNotStartedFromZero).";
const FINGERPAD_LOAD_CHANGED: &str = "the normal force or the twist rate differs from the one this twist began with: the torque, the stuck fraction and the stored energy are closed forms of the twist under one load held from the untwisted pad, and under a changed load the bed's springs hold a state set by the path, which the card's state does not carry (LoadChangedDuringTwist).";

fn fingerpad_reject(reject: fingerpad_p02_glass::StepReject) -> String {
    use fingerpad_p02_glass::StepReject;
    match reject {
        StepReject::NoContact => unsealed(
            "the normal force is zero or below, or under the smallest normal 64-bit number: the plate does not press on the pad (NoContact).",
        ),
        StepReject::TwistRateNotPositive => unsealed(
            "the twist rate is zero or below, or under the smallest normal 64-bit number: the plate does not turn forward, and the characteristic twists, powers of the rate, have no value (TwistRateNotPositive).",
        ),
        StepReject::TwistStepAboveQuadrature => unsealed(
            "the step turns the plate by more than 0.05 of the characteristic twist of the torque, past which the four-point rule that sums the plate work and the heat is not exact to rounding: take a shorter step (TwistStepAboveQuadrature).",
        ),
        StepReject::LoadChangedDuringTwist => unsealed(FINGERPAD_LOAD_CHANGED),
        StepReject::TwistNotStartedFromZero => unsealed(FINGERPAD_TWIST_NOT_FROM_ZERO),
        StepReject::TwistBelowZero => unsealed(FINGERPAD_TWIST_BELOW_ZERO),
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
        StepReject::NoContact => unsealed(
            "the normal load is zero or below, or under the smallest normal 64-bit number: the tyre does not press on the ice (NoContact).",
        ),
        StepReject::TyreFlat => unsealed(
            "the inflation pressure, gauge, is zero or below: the tyre is flat (TyreFlat).",
        ),
        StepReject::SlipOutsideCard => unsealed(TIRE_SLIP_OUTSIDE_CARD),
        StepReject::CarriageSpeedOutsideCard => unsealed(
            "the carriage speed is below 0.05 m/s or above 0.07 m/s, the creep of the rig (CarriageSpeedOutsideCard).",
        ),
        StepReject::DistanceBelowZero => unsealed(TIRE_DISTANCE_BELOW_ZERO),
        StepReject::SlipHeldBelowZero => unsealed(TIRE_SLIP_HELD_BELOW_ZERO),
        StepReject::StepTooSmallToResolve => unsealed(TIRE_STEP_TOO_SMALL),
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
        StepReject::GasMassNotPositive => unsealed("the gas mass is zero or less (GasMassNotPositive)."),
        StepReject::WallTemperatureNotPositive => unsealed(
            "the wall temperature is at or below absolute zero (WallTemperatureNotPositive).",
        ),
        StepReject::PistonTravelAboveStep => unsealed(
            "the piston would travel more than 1 mm in the step (PistonTravelAboveStep).",
        ),
        StepReject::GasBelowCriticalTemperature => unsealed(
            "the gas temperature, incoming or landing, is at or below 126.192 K, nitrogen's critical temperature, where it can condense (GasBelowCriticalTemperature).",
        ),
        StepReject::PressureAboveRated => unsealed(
            "the gas pressure, incoming or landing, is above 330 bar, the vessel's rated maximum working pressure (PressureAboveRated).",
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

const SATELLITE_RADIUS_BELOW_EARTH_RADIUS: &str =
    "distance from the Earth's centre is below 6 378.1363 km, the Earth's equatorial radius, to which the field is scaled: the satellite is under the sphere of the Earth's equator, where the card's field, the series of the Earth's field outside the Earth, is not the field (RadiusBelowEarthRadius).";
const SATELLITE_EPOCH_OUTSIDE_SPAN: &str =
    "the time is before 2017-01-01 00:00 UTC, the last leap second, or after 2027-06-28 00:00 UTC, when the IERS leap-second file of Bulletin C 72 expires: outside that span TT - UTC is not the card's 69.184 s, or is not yet known (EpochOutsideLeapSecondSpan).";
const SATELLITE_FORECAST_BELOW_ZERO: &str =
    "the stated forecast_elapsed_s, the time since the state was taken from a published orbit, is below zero (ForecastElapsedBelowZero).";

const SOLAR_SYSTEM_DISTANCE_OUTSIDE_CARD: &str =
    "a body's distance from its primary is outside that body's band (DistanceOutsideCard).";
const SOLAR_SYSTEM_ORBIT_OUTSIDE_CARD: &str = "the semi-major axis of a body's orbit about its primary is outside that body's band, or the orbit is not closed (OrbitOutsideCard).";
const SOLAR_SYSTEM_FRAME_OUTSIDE_CARD: &str =
    "the centre of mass of the thirteen bodies moves faster than 0.01 m/s (FrameOutsideCard).";
const SOLAR_SYSTEM_BODIES_COINCIDE: &str = "two bodies are at exactly one point, where the attraction between them has no value (BodiesCoincide).";
const SOLAR_SYSTEM_AXES_OUTSIDE_CARD: &str = "the orbit of the Earth-Moon barycentre about the Sun is not inclined 21 to 27 degrees to the z axis, as the ecliptic is to the ICRF equator: the axes are not those of the ICRF (AxesOutsideCard).";

fn solar_system_reject(reject: solar_system::StepReject) -> String {
    match reject {
        solar_system::StepReject::DistanceOutsideCard => {
            unsealed(SOLAR_SYSTEM_DISTANCE_OUTSIDE_CARD)
        }
        solar_system::StepReject::OrbitOutsideCard => unsealed(SOLAR_SYSTEM_ORBIT_OUTSIDE_CARD),
        solar_system::StepReject::FrameOutsideCard => unsealed(SOLAR_SYSTEM_FRAME_OUTSIDE_CARD),
        solar_system::StepReject::BodiesCoincide => unsealed(SOLAR_SYSTEM_BODIES_COINCIDE),
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
        satellite_lageos1::StepReject::RadiusBelowEarthRadius => {
            unsealed(SATELLITE_RADIUS_BELOW_EARTH_RADIUS)
        }
        satellite_lageos1::StepReject::EpochOutsideLeapSecondSpan => {
            unsealed(SATELLITE_EPOCH_OUTSIDE_SPAN)
        }
        satellite_lageos1::StepReject::ForecastElapsedBelowZero => {
            unsealed(SATELLITE_FORECAST_BELOW_ZERO)
        }
        satellite_lageos1::StepReject::SpontaneousGeneration => unsealed(
            "the satellite gained energy that the work of the other forces does not account for (SpontaneousGeneration).",
        ),
        satellite_lageos1::StepReject::FirstLawBreach => unsealed(
            "the satellite lost energy that the work of the other forces does not account for (FirstLawBreach).",
        ),
        satellite_lageos1::StepReject::AngularMomentumBreach => unsealed(
            "the satellite's angular momentum about the Earth's axis changed by more than the torque of the field and the other forces accounts for (AngularMomentumBreach).",
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
        ("beyond_compared", StateField::Bool(state.beyond_compared)),
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
        (
            "since_disturbance_s",
            StateField::F64(state.since_disturbance_s),
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
        ("pointing_disturbed", StateField::Bool(state.pointing_disturbed)),
        ("beyond_four_sigma", StateField::Bool(state.beyond_four_sigma)),
        ("recovering", StateField::Bool(state.recovering)),
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
        (
            "envelope_below_compared",
            StateField::Bool(state.envelope_below_compared),
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
            "angular_momentum_m2_s",
            StateField::F64(state.angular_momentum_m2_s),
        ),
        (
            "angular_momentum_anchor_m2_s",
            StateField::F64(state.angular_momentum_anchor_m2_s),
        ),
        (
            "external_torque_m2_s",
            StateField::F64(state.external_torque_m2_s),
        ),
        (
            "angular_momentum_defect_m2_s",
            StateField::F64(state.angular_momentum_defect_m2_s),
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
        ("outside_met_range", StateField::Bool(state.outside_met_range)),
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
        ("beyond_compared", StateField::Bool(state.beyond_compared)),
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

fn runaway_state(state: &runaway_18650_lco::RunawayState) -> Result<serde_json::Value, String> {
    runaway_with(state, state_object)
}

pub(crate) fn runaway_published_is_finite(state: &runaway_18650_lco::RunawayState) -> Result<(), String> {
    runaway_with(state, fields_are_finite)
}

fn runaway_with<T>(
    state: &runaway_18650_lco::RunawayState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("temperature_k", StateField::F64(state.temperature_k)),
        ("sei_fraction", StateField::F64(state.sei_fraction)),
        ("anode_lithium_fraction", StateField::F64(state.anode_lithium_fraction)),
        ("cathode_unreacted_fraction", StateField::F64(state.cathode_unreacted_fraction)),
        ("electrolyte_fraction", StateField::F64(state.electrolyte_fraction)),
        ("sei_thickness", StateField::F64(state.sei_thickness)),
        ("start_temperature_k", StateField::F64(state.start_temperature_k)),
        ("heat_sei_j", StateField::F64(state.heat_sei_j)),
        ("heat_anode_j", StateField::F64(state.heat_anode_j)),
        ("heat_cathode_j", StateField::F64(state.heat_cathode_j)),
        ("heat_electrolyte_j", StateField::F64(state.heat_electrolyte_j)),
        ("heat_released_j", StateField::F64(state.heat_released_j)),
        ("heater_heat_j", StateField::F64(state.heater_heat_j)),
        ("heat_lost_j", StateField::F64(state.heat_lost_j)),
        ("heat_stored_j", StateField::F64(state.heat_stored_j)),
        ("chemical_heat_w", StateField::F64(state.chemical_heat_w)),
        ("chemical_heating_k_s", StateField::F64(state.chemical_heating_k_s)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("substeps", StateField::F64(state.substeps)),
        ("self_heating", StateField::Bool(state.self_heating)),
        ("runaway", StateField::Bool(state.runaway)),
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
                reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE[0].0,
                reactor_triga_jsi::ROD_TABLE_CORE[0],
            )
            .expect("the reactor starts at the low end of the starts compared on its first table");
            Some(reactor_triga_with(&state, finish))
        }
        ("energy", "runaway_18650_lco") => {
            let cell = runaway_18650_lco::Runaway18650Lco::hatchard_kim();
            let state = runaway_18650_lco::RunawayState::try_fresh(&cell, 298.15)
                .expect("a fresh cell starts at 298.15 K");
            Some(runaway_with(&state, finish))
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
        ("materials", "grasp_lift_glass_4tu") => Some(card_grasp_lift_glass_4tu::rest(finish)),
        ("aerospace", "momentum_jwst_srp") => Some(card_momentum_jwst_srp::rest(finish)),
        ("aerospace", "momentum_jwst_dihedral") => Some(card_momentum_jwst_dihedral::rest(finish)),
        ("energy", "generator_mmrtg_seebeck") => Some(card_generator_mmrtg_seebeck::rest(finish)),
        ("energy", "reactor_msre_pump") => Some(card_reactor_msre_pump::rest(finish)),
        ("energy", "reactor_spert3_twin") => Some(card_reactor_spert3_twin::rest(finish)),
        ("energy", "reactor_spert3_ccore_twin") => Some(card_reactor_spert3_ccore_twin::rest(finish)),
        ("aerospace", "crater_ryugu_sci") => {
            let crater = crater_ryugu_sci::CraterRyuguSci::sci_impact();
            let state = crater_ryugu_sci::CraterState::try_radius(
                &crater,
                descriptor::CRATER_EXAMPLE_START_M,
            )
            .expect("the crater starts at its worked example's radius");
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
        ("energy", "tokamak_vde_mast") => {
            let card = tokamak_vde_mast::TokamakVdeMast::mast_15045();
            let state = tokamak_vde_mast::VdeState::try_start(&card, 0.0, 0.0, 0.0, 0.0)
                .expect("the plasma rests at its equilibrium");
            Some(vde_with(&state, finish))
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
        enforce_force_nyquist(force_n, dt_s, &cell).map_err(|text| named(text, "ForceAboveNyquist"))?;
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
        enforce_force_nyquist(force_n, self.dt_s, &self.cell)
            .map_err(|text| named(text, "ForceAboveNyquist"))?;
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
                        "Physical Failure: the grid step landed with |current_a| above {} A, the energy bound of the grid_node card: the feeder holds more energy than the link takes as heat to its melting point (CurrentOutsideCard). That state is not sealed.",
                        descriptor::grid_melt_current_a(&self.node)
                    ),
                    grid_node::StepReject::VoltageOutsideCard => format!(
                        "Physical Failure: the grid step landed with |voltage_v| above {} V, the energy bound of the grid_node card: the shunt holds more energy than the link takes as heat to its melting point (VoltageOutsideCard). That state is not sealed.",
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
        load_nyquist(
            "ForceAboveNyquist",
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
        load_nyquist(
            "ForceAboveNyquist",
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
        load_nyquist(
            "ForceAboveNyquist",
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
        load_nyquist(
            "ForceAboveNyquist",
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
        load_nyquist(
            "FlowAboveNyquist",
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
        load_nyquist(
            "FlowAboveNyquist",
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
        load_nyquist(
            "ForceAboveNyquist",
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
        load_nyquist(
            "ForceAboveNyquist",
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
        // The held current of the discharge this run belongs to: for a run
        // from a rested cell its own current, for a continuation the earlier
        // run's.
        let held_current_a = required_f64(params, "held_current_a")?;
        let current_a = required_f64(params, "current_a")?;
        let dt_s = required_f64(params, "dt_s")?;
        let cell = cell_18650pf::Cell18650Pf::cell();
        if !(current_a > 0.0 && held_current_a > 0.0) {
            return Err(cell_current_not_discharging());
        }
        // The cell has no pole. Its ceiling is the time to cross one table
        // interval at the larger of the held current and 3.0 A.
        card_clock(dt_s, None, cell.floor_dt_s(), cell.gate_dt_s_at(held_current_a), "cell_18650pf")?;
        if !(soc >= cell_18650pf::SOC_MIN && soc <= 1.0) {
            return Err(unsealed(
                "state of charge left [0.03, 1] (StateOfChargeOutsideCard).",
            ));
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
        if !(current_a > 0.0) {
            return Err(cell_current_not_discharging());
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
    /// to the card's own bounds, where its tables end. A load outside the
    /// run's block's own tests is the step's flag, not a refusal.
    fn read_load(
        params: &serde_json::Value,
        _block: usize,
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
        let named = params.get("block_chamber_c").and_then(serde_json::Value::as_f64);
        // The published bounds of the charge are the widest ends of the six
        // blocks' tables: a start inside them is held to its own block's
        // tables below, both ends included.
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
    /// to the card's own limits. A load outside the compared range is the
    /// step's flag, not a refusal.
    fn read_load(params: &serde_json::Value) -> Result<cell_2170_model3::CellLoad, String> {
        let current_a = required_f64(params, "current_a")?;
        let case_temperature_c = required_f64(params, "case_temperature_c")?;
        let cell = cell_2170_model3_type();
        if !(case_temperature_c + cell.kelvin_offset > 0.0) {
            return Err(unsealed(CELL_2170_MODEL3_BELOW_ABSOLUTE_ZERO));
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
        if !(least_c <= most_c) {
            return Err(cell_2170_model3_span_inverted(least_c, most_c));
        }
        if !(least_c + cell.kelvin_offset > 0.0) {
            return Err(unsealed(CELL_2170_MODEL3_BELOW_ABSOLUTE_ZERO));
        }
        let state = state
            .with_case_span(cell, least_c, most_c)
            .ok_or_else(|| phase_not_valid("cell_2170_model3"))?;
        Ok(Self {
            cell,
            state,
            load,
            dt_s,
        })
    }

    /// A later segment's load, held to the card's own limits. Where it lies
    /// against the compared range, the capacity at its case temperature, the
    /// jump and the span of case temperature, is the step's flag.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "cell_2170_model3"),
        )?;
        enforce_published("energy", "cell_2170_model3", Published::Load, params)?;
        let load = Self::read_load(params)?;
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
        if !(mission_time_s >= 0.0) {
            return Err(unsealed("mission time is before landing on Mars (MissionTimeBeforeLanding)."));
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
        // The lower end is zero, the moment the full load is applied; the
        // upper end is the card's own function of the stress.
        if !(creep_extension_percent >= 0.0) {
            return Err(creep_extension_below_zero());
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
        // reach with this run's inertia and demand, so the clock applies no
        // `π/ω` of its own. At an inertia not above zero or a demand below
        // zero the published bounds have refused the request already.
        // The published ceiling, or the run's own where that is smaller.
        let ceiling = match (card.gate_dt_s(), card.gate_dt_s_at(inertia_mw_s, demand_mw)) {
            (Some(published), Some(own)) => Some(published.min(own)),
            (published, _) => published,
        };
        card_clock(dt_s, None, card.floor_dt_s(), ceiling, "grid_gb")?;
        let state = grid_gb::GridState::try_start(&card, frequency_hz, inertia_mw_s, demand_mw)
            .ok_or_else(|| phase_not_valid("grid_gb"))?;
        Ok(Self {
            card,
            state,
            load: grid_gb::GridLoad { loss_mw },
            dt_s,
        })
    }

    /// The load of a segment. A segment may carry another loss than the
    /// first: the card's step books each step's own loss, and flags a loss
    /// that is not the run's first as outside the compared trips.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "grid_gb"))?;
        enforce_published("energy", "grid_gb", Published::Load, params)?;
        let loss_mw = required_f64(params, "loss_mw")?;
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
        if !(coolant_temperature_k >= reactor.water_freezing_k
            && coolant_temperature_k <= reactor.water_critical_k)
        {
            return Err(reactor_coolant_not_liquid_water());
        }
        // The load opened with is the first step's: the step in reactivity, which must insert
        // some.
        if !(reactivity_inserted_dollar > 0.0) {
            return Err(reactor_reactivity_not_positive());
        }
        let state = reactor_spert3::ReactorState::try_cold_start(&reactor, coolant_temperature_k)
            .ok_or_else(|| phase_not_valid("reactor_spert3"))?;
        Ok(Self {
            reactor,
            state,
            load: reactor_spert3::ReactorLoad {
                reactivity_inserted_dollar,
            },
            dt_s,
        })
    }

    /// The load of a segment. A later segment may carry any reactivity: each
    /// step is held at its own, and the card's ledger books each step at the
    /// reactivity it held. One at zero or less turns the power down, and the
    /// run ends there at the first maximum.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "reactor_spert3"))?;
        enforce_published("energy", "reactor_spert3", Published::Load, params)?;
        let reactivity_inserted_dollar = required_f64(params, "reactivity_inserted_dollar")?;
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
        // constant of its prompt pole after the scram; the card's own step
        // holds the ceiling that follows the state.
        card_clock(
            dt_s,
            None,
            reactor.floor_dt_s(),
            reactor.gate_dt_s(),
            "reactor_triga_jsi",
        )?;
        // The published span of rod_table_core holds 215 to 231; only three
        // numbers in it are tables.
        if reactor.table_of(rod_table_core).is_none() {
            return Err(reactor_triga_table_outside_card(rod_table_core));
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

impl RunawayCall {
    /// The load of a step or a segment. `run_trajectory` reads every segment
    /// through here before the first step, so an adiabatic switch between 0
    /// and 1 in any segment refuses the whole run.
    fn read_load(params: &serde_json::Value) -> Result<runaway_18650_lco::RunawayLoad, String> {
        let adiabatic = required_f64(params, "adiabatic")?;
        if adiabatic.is_finite() && adiabatic != 0.0 && adiabatic != 1.0 {
            return Err(runaway_reject(runaway_18650_lco::StepReject::AdiabaticNotZeroOrOne));
        }
        let oven_temperature_k = required_f64(params, "oven_temperature_k")?;
        // The oven temperature enters only where heat crosses the can.
        if adiabatic == 0.0 && oven_temperature_k.is_finite() && !(oven_temperature_k > 0.0) {
            return Err(unsealed(&format!(
                "'oven_temperature_k' {} K is at or below 0 K, the exclusive minimum of the runaway_18650_lco card where the cell loses heat to the oven, adiabatic 0 (OvenTemperatureNotPositive).",
                message_number(oven_temperature_k)
            )));
        }
        Ok(runaway_18650_lco::RunawayLoad {
            oven_temperature_k,
            heater_power_w: required_f64(params, "heater_power_w")?,
            adiabatic,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names("energy", "runaway_18650_lco"))?;
        enforce_published("energy", "runaway_18650_lco", Published::Step, params)?;
        let temperature_k = required_f64(params, "temperature_k")?;
        let sei_fraction = required_f64(params, "sei_fraction")?;
        let anode_lithium_fraction = required_f64(params, "anode_lithium_fraction")?;
        let cathode_unreacted_fraction = required_f64(params, "cathode_unreacted_fraction")?;
        let electrolyte_fraction = required_f64(params, "electrolyte_fraction")?;
        let load = Self::read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let cell = runaway_18650_lco::Runaway18650Lco::hatchard_kim();
        // The ceiling comes from the cell's convective time constant; the
        // substeps inside a step follow the reactions, so no `π/ω` applies.
        card_clock(dt_s, None, cell.floor_dt_s(), cell.gate_dt_s(), "runaway_18650_lco")?;
        let state = runaway_18650_lco::RunawayState::try_start(
            &cell,
            temperature_k,
            sei_fraction,
            anode_lithium_fraction,
            cathode_unreacted_fraction,
            electrolyte_fraction,
        )
        .map_err(runaway_reject)?;
        Ok(Self { cell, state, load, dt_s })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "runaway_18650_lco"))?;
        enforce_published("energy", "runaway_18650_lco", Published::Load, params)?;
        self.load = Self::read_load(params)?;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        runaway_18650_lco::step(&self.cell, &mut self.state, frame, self.load, self.dt_s).map_err(|reject| {
            let detail = runaway_reject(reject);
            StepFailure::from_reject(reject, detail)
        })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame).map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: runaway_state(&self.state)?,
        })
    }
}

impl ReactorCcoreCall {
    /// The reactivity and the flow of a load: a reactivity above zero and a flow of zero or more.
    fn read_load(
        params: &serde_json::Value,
    ) -> Result<reactor_spert3_ccore::ReactorLoad, String> {
        let reactivity_inserted_dollar = required_f64(params, "reactivity_inserted_dollar")?;
        let coolant_flow_m_per_s = required_f64(params, "coolant_flow_m_per_s")?;
        if coolant_flow_m_per_s < 0.0 {
            return Err(reactor_ccore_negative_flow(coolant_flow_m_per_s));
        }
        if !(reactivity_inserted_dollar > 0.0) {
            return Err(reactor_ccore_reactivity_not_positive(reactivity_inserted_dollar));
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
        .ok_or_else(|| reactor_ccore_start_not_liquid(initial_temperature_k, system_pressure_psig))?;
        let load = Self::read_load(params)?;
        Ok(Self {
            reactor,
            state,
            load,
            run_load: load,
            dt_s,
        })
    }

    /// The load of a segment, held to its limits and to the reactivity the run was opened with.
    /// `run_trajectory` reads every segment through here before the first step, so a run that
    /// would change its reactivity is refused whole. A segment may carry another flow: each
    /// step is taken at its own.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "reactor_spert3_ccore"),
        )?;
        enforce_published("energy", "reactor_spert3_ccore", Published::Load, params)?;
        let load = Self::read_load(params)?;
        if load.reactivity_inserted_dollar != self.run_load.reactivity_inserted_dollar {
            return Err(reactor_ccore_reactivity_changed(
                load.reactivity_inserted_dollar,
                self.run_load.reactivity_inserted_dollar,
            ));
        }
        self.load = load;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        let before = self.state;
        reactor_spert3_ccore::step(&self.reactor, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| {
                let detail = if reject == reactor_spert3_ccore::StepReject::DtAboveStateCeiling {
                    reactor_ccore_state_ceiling(
                        self.dt_s,
                        reactor_spert3_ccore::state_dt_ceiling_s(&self.reactor, &before, self.load),
                    )
                } else {
                    reactor_ccore_reject(reject)
                };
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
    /// The reactivity and the flow of a load: a reactivity above zero and a
    /// flow of zero or more, in any band.
    fn read_load(params: &serde_json::Value) -> Result<reactor_spert3_fuel::ReactorLoad, String> {
        let reactivity_inserted_dollar = required_f64(params, "reactivity_inserted_dollar")?;
        let coolant_flow_m_per_s = required_f64(params, "coolant_flow_m_per_s")?;
        if !(reactivity_inserted_dollar > 0.0) {
            return Err(reactor_fuel_reactivity_not_positive(reactivity_inserted_dollar));
        }
        if coolant_flow_m_per_s < 0.0 {
            return Err(reactor_fuel_flow_negative(coolant_flow_m_per_s));
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
        // The band of the inlet temperature sets the card's constants.
        reactor
            .band_of(inlet_temperature_k)
            .ok_or_else(|| reactor_fuel_inlet_outside_card(inlet_temperature_k))?;
        let load = Self::read_load(params)?;
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

    /// The load of a segment. Its reactivity is held to the one the run was
    /// opened with: a run is one burst, and the card's step refuses another
    /// reactivity once the first step has landed. `run_trajectory` reads every
    /// segment through here before the first step, so a run that would change
    /// its reactivity is refused whole and nothing of it is sealed. Each
    /// segment may carry its own flow rate: the card books the sweep of the
    /// water per step at that step's flow.
    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(
            params,
            descriptor::load_names("energy", "reactor_spert3_fuel"),
        )?;
        enforce_published("energy", "reactor_spert3_fuel", Published::Load, params)?;
        self.reactor
            .band_of(self.state.inlet_temperature_k)
            .ok_or_else(|| reactor_fuel_inlet_outside_card(self.state.inlet_temperature_k))?;
        let load = Self::read_load(params)?;
        if load.reactivity_inserted_dollar != self.burst_load.reactivity_inserted_dollar {
            return Err(reactor_fuel_reactivity_changed(
                load.reactivity_inserted_dollar,
                self.burst_load.reactivity_inserted_dollar,
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
    /// The landed state is past the flown drogue deployment: the step that
    /// first crossed both conditions landed, and the card's next step would
    /// be refused there as its declared end. Read by the card's own start
    /// rule, which tests the state as the next step's would, with no step.
    pub(crate) fn past_flight_deployment(&self) -> bool {
        capsule_stardust::CapsuleState::anchor(
            &self.capsule,
            self.state.radius_m,
            self.state.radial_speed_m_s,
            self.state.transverse_speed_m_s,
        ) == Err(capsule_stardust::StepReject::PastFlightDeployment)
    }

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
        // The card refuses a start by the name its step would refuse it by:
        // above the ceiling, under sea level, and every other limit.
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
    /// The landed state is past the flown parachute deployment: the step
    /// that first crossed both conditions landed, and the card's next step
    /// would be refused there as its declared end. Read by the card's own
    /// start rule, which tests the state as the next step's would, with no
    /// step.
    pub(crate) fn past_flight_deployment(&self) -> bool {
        capsule_insight::CapsuleState::start(
            &self.capsule,
            self.state.radius_m,
            self.state.radial_speed_m_s,
            self.state.transverse_speed_m_s,
        )
        .err()
            == Some(capsule_insight::StepReject::PastFlightDeployment)
    }

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
        // current and the current and the field to a safety factor of 2.
        tokamak_jet_dte1::load_in_card(tokamak, load)
            .map_err(|reject| tokamak_reject(reject, tokamak, load, (f64::NAN, f64::NAN), f64::NAN))?;
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
        // The time since the start of the heated window: a run continued from
        // a returned state states that state's t_s, and a start without it is
        // refused.
        let elapsed_time_s = required_f64(params, "elapsed_time_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let tokamak = tokamak_jet_dte1::TokamakJetDte1::elmy_h_mode();
        // The static step ceiling is a locked constant of the card; the
        // card's own step holds dt to the ceiling of each state.
        card_clock(
            dt_s,
            None,
            tokamak.floor_dt_s(),
            tokamak.gate_dt_s(),
            "tokamak_jet_dte1",
        )?;
        let load = Self::read_load(&tokamak, params)?;
        // The start is held by the card's own test under the first load: a
        // thermal energy above zero, the temperature range of the reactivity
        // fit and the Troyon limit.
        tokamak_jet_dte1::state_in_card(&tokamak, thermal_energy_j, fast_ion_energy_j, load)
            .map_err(|reject| tokamak_reject(reject, &tokamak, load, (thermal_energy_j, fast_ion_energy_j), dt_s))?;
        // The ceiling of the start's own state under the first load is a
        // limit the request shows, so it is held here, before any step.
        if !(dt_s <= tokamak.state_ceiling_dt_s(thermal_energy_j, fast_ion_energy_j, load)) {
            return Err(tokamak_reject(
                tokamak_jet_dte1::StepReject::DtAboveStateCeiling,
                &tokamak,
                load,
                (thermal_energy_j, fast_ion_energy_j),
                dt_s,
            ));
        }
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
        let (tokamak, load, dt_s) = (self.tokamak, self.load, self.dt_s);
        let start = (self.state.thermal_energy_j, self.state.fast_ion_energy_j);
        tokamak_jet_dte1::step(&self.tokamak, &mut self.state, frame, self.load, self.dt_s).map_err(
            |reject| {
                let detail = tokamak_reject(reject, &tokamak, load, start, dt_s);
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
        // The step ceiling is a locked constant of the card, set by the
        // rounding of the water built in one step, so the clock applies no
        // `π/ω` of its own.
        card_clock(dt_s, None, link.floor_dt_s(), link.gate_dt_s(), "link_gothenburg_10010")?;
        let state = link_gothenburg_10010::LinkState::try_continue(
            &link,
            dry_path_loss_db,
            antenna_water_db,
            gauge_minute_elapsed_s,
            gauge_minute_rain_mm_h,
        )
        .map_err(link_reject)?;
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
        if !(quiet_before_start_s >= 0.0) {
            return Err(unsealed(LOS_QUIET_BELOW_ZERO));
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
        // ceiling. The envelope is the card's function.
        let envelope_m = beam.envelope_m(deflection_m, velocity_m_s);
        if !(envelope_m <= beam.amplitude_ceiling_m) {
            return Err(unsealed(CANTILEVER_ENVELOPE_ABOVE_CARD));
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
        // a distance, then an orbit, then two bodies at one point, then the
        // frame, then the axes, then the carried ledger.
        let state = solar_system::SolarSystemState::try_continue(
            &system,
            position_m,
            velocity_m_s,
            ledger,
        )
        .map_err(|reject| match reject {
            solar_system::StepReject::DistanceOutsideCard
            | solar_system::StepReject::OrbitOutsideCard
            | solar_system::StepReject::BodiesCoincide
            | solar_system::StepReject::FrameOutsideCard
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
        // The time since the published state is required: it is reported
        // with every state, counted over every run a forecast went through.
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
        // The card names the radius before the date and the date before the
        // forecast. The date and the forecast are held to the spans
        // `describe_card` publishes after.
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
            let reject = satellite_lageos1::StepReject::EpochOutsideLeapSecondSpan;
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
/// the radius, then the date, then the forecast. The radius is the length of
/// the position.
fn satellite_off_card(
    satellite: &satellite_lageos1::SatelliteLageos1,
    epoch_utc_s: f64,
    forecast_elapsed_s: f64,
    position_m: [f64; 3],
    _velocity_m_s: [f64; 3],
) -> String {
    let radius_m = (position_m[0] * position_m[0]
        + position_m[1] * position_m[1]
        + position_m[2] * position_m[2])
        .sqrt();
    if !(radius_m >= satellite.radius_min_m) {
        return unsealed(SATELLITE_RADIUS_BELOW_EARTH_RADIUS);
    }
    if !(epoch_utc_s >= satellite.epoch_min_s && epoch_utc_s <= satellite.epoch_max_s) {
        let published = descriptor::find("aerospace", "satellite_lageos1")
            .and_then(|spec| spec.fields.iter().find(|field| field.name == "epoch_utc_s"))
            .and_then(|field| published_outside("satellite_lageos1", field, epoch_utc_s));
        return published.unwrap_or_else(|| unsealed(SATELLITE_EPOCH_OUTSIDE_SPAN));
    }
    if !(forecast_elapsed_s >= 0.0) {
        let published = descriptor::find("aerospace", "satellite_lageos1")
            .and_then(|spec| spec.fields.iter().find(|field| field.name == "forecast_elapsed_s"))
            .and_then(|field| published_outside("satellite_lageos1", field, forecast_elapsed_s));
        return published.unwrap_or_else(|| unsealed(SATELLITE_FORECAST_BELOW_ZERO));
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
        ("energy", "cell_18650pf") => {
            cell_18650pf::FLAG_SAG_WARNING | cell_18650pf::FLAG_CURRENT_OUTSIDE_PASSED_SPAN
        }
        ("energy", "generator_mmrtg_f1") => {
            generator_mmrtg_f1::FLAG_PAST_FITTING_SPAN | generator_mmrtg_f1::FLAG_PAST_RECORD
        }
        ("energy", "grid_gb") => {
            grid_gb::FLAG_OUTSIDE_COMPARED_TRIPS
                | grid_gb::FLAG_PAST_COMPARED_MINUTE
                | grid_gb::FLAG_BELOW_COMPARED_FREQUENCY
        }
        ("energy", "cell_2170_m80") => {
            cell_2170_m80::FLAG_BLOCK_MET_NO_MARK | cell_2170_m80::FLAG_OUTSIDE_COMPARED_TESTS
        }
        ("energy", "accumulator_sbo450") => accumulator_sbo450::FLAG_OUTSIDE_COMPARED_RUNS,
        ("aerospace", "capsule_stardust") => capsule_stardust::FLAG_DROGUE_MACH_WINDOW,
        ("aerospace", "satellite_lageos1") => {
            satellite_lageos1::FLAG_EARTH_SHADOW | satellite_lageos1::FLAG_BELOW_COMPARED_RADIUS
        }
        ("energy", "reactor_spert3") => reactor_spert3::FLAG_PROMPT_SUPERCRITICAL,
        ("signal", "link_gothenburg_10010") => link_gothenburg_10010::FLAG_RAINING,
        ("aerospace", "los_jwst_fine_guide") => {
            los_jwst_fine_guide::FLAG_NO_MEASUREMENT
                | los_jwst_fine_guide::FLAG_POINTING_DISTURBED
                | los_jwst_fine_guide::FLAG_BEYOND_FOUR_SIGMA
                | los_jwst_fine_guide::FLAG_RECOVERING
                | los_jwst_fine_guide::FLAG_COUNT_RATE_BEYOND_COMPARED
        }
        ("energy", "cell_2170_model3") => {
            cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C | cell_2170_model3::FLAG_OUTSIDE_COMPARED_RUNS
        }
        ("materials", "creep_cmsx6") => {
            creep_cmsx6::FLAG_CREEP_RATE_RISING | creep_cmsx6::FLAG_OUTSIDE_MET_RANGE
        }
        ("signal", "cantilever_eth") => {
            cantilever_eth::FLAG_ENVELOPE_BELOW_COMPARED | cantilever_eth::FLAG_ENVELOPE_ABOVE_COMPARED
        }
        ("aerospace", "capsule_insight") => capsule_insight::FLAG_LIFT_ACTING,
        ("energy", "reactor_spert3_fuel") => {
            reactor_spert3_fuel::FLAG_PROMPT_SUPERCRITICAL | reactor_spert3_fuel::FLAG_ROD_MOVING
        }
        ("energy", "runaway_18650_lco") => runaway_18650_lco::FLAG_SELF_HEATING | runaway_18650_lco::FLAG_RUNAWAY,
        ("energy", "reactor_spert3_ccore") => {
            reactor_spert3_ccore::FLAG_PROMPT_SUPERCRITICAL | reactor_spert3_ccore::FLAG_PAST_POWER_PEAK
        }
        ("energy", "reactor_triga_jsi") => {
            reactor_triga_jsi::FLAG_PROMPT_SUPERCRITICAL
                | reactor_triga_jsi::FLAG_ROD_MOVING
                | reactor_triga_jsi::FLAG_RODS_DROPPED
        }
        ("aerospace", "crater_ryugu_sci") => crater_ryugu_sci::FLAG_STOPPED,
        ("energy", "reactor_msre_pump") => card_reactor_msre_pump::FLAGS,
        ("energy", "reactor_spert3_twin") => card_reactor_spert3_twin::FLAGS,
        ("energy", "reactor_spert3_ccore_twin") => card_reactor_spert3_ccore_twin::FLAGS,
        ("energy", "generator_mmrtg_seebeck") => card_generator_mmrtg_seebeck::FLAGS,
        ("aerospace", "momentum_jwst_dihedral") => card_momentum_jwst_dihedral::FLAGS,
        ("aerospace", "momentum_jwst_srp") => card_momentum_jwst_srp::FLAGS,
        ("materials", "grasp_lift_glass_4tu") => card_grasp_lift_glass_4tu::FLAGS,
        ("materials", "tire_eve_r19_ice") => {
            tire_eve_r19_ice::FLAG_LOAD_ABOVE_FITTED
                | tire_eve_r19_ice::FLAG_BEYOND_COMPARED
                | tire_eve_r19_ice::FLAG_SLIP_SETTLING
        }
        ("energy", "tokamak_jet_dte1") => tokamak_jet_dte1::FLAG_NEAR_DENSITY_LIMIT,
        ("energy", "tokamak_vde_mast") => tokamak_vde_mast::FLAG_PAST_IDEAL_WALL_LIMIT,
        ("materials", "fingerpad_p02_glass") => {
            fingerpad_p02_glass::FLAG_FULL_SLIP | fingerpad_p02_glass::FLAG_BEYOND_COMPARED
        }
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
                "Missing required parameter: '{key}' (InputMissing)."
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
        DoorError::Unprocessable("Missing required parameter: 'parameters' (InputMissing).".to_string())
    })?;
    Ok(VerifyPhysicsRequest { domain, card, parameters })
}

/// The card a step request names, when this connection's view holds it.
fn known_card(state: &AppState, payload: &VerifyPhysicsRequest) -> Option<&'static str> {
    state.view.find(&payload.domain, &payload.card).map(|spec| spec.card)
}

/// One anchored step. `request` is the whole request as received. It is
/// sealed into the frame proof and echoed as `inputs`. A request with `from`
/// starts from a sealed upstream run (`src/chain.rs`): the upstream is
/// computed again and checked, `from` is taken out, and the step is the same
/// step as the request typed by hand, with `chain` beside it.
pub async fn compute_step(
    state: &AppState,
    request: serde_json::Value,
) -> Result<VerifyPhysicsResponse, DoorError> {
    let (request, from) = chain::take(request);
    // The plain step's future is large; it is boxed so that this one, which
    // may hold it twice, is not.
    let Some(from) = from else {
        return Box::pin(compute_step_plain(state, request, fresh_start::Carried::Nothing)).await;
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
    validate_physics_of(known_card(state, &payload), &payload.parameters).map_err(DoorError::Unprocessable)?;
    let plan = chain::read(state.view(), &request, from, chain::Target::Step, 1)?;
    let carried = plan.carried();
    // The fresh-start rule of this start, before the upstream is computed
    // (REFERENCE.md, "A chain"). A start the card itself turns away is named
    // where it was, after the chain is checked.
    if let Err(StartTurned::FreshStart(error)) =
        step_start_rules(state, &payload.domain, &payload.card, &payload.parameters, &carried)
    {
        return Err(error);
    }
    let (settled, request) = on_pool(state, move || plan.settle()).await?;
    let mut response = Box::pin(compute_step_plain(state, request, carried)).await?;
    response.chain = Some(settled.chain_value(&response.sha256_seal));
    seal_response(response).map_err(|(_, message)| DoorError::Internal(message))
}

/// Why a step's start was turned away: by the card's own start rules, or
/// by the fresh-start rule (rule 14), read after them.
enum StartTurned {
    Card(DoorError),
    FreshStart(DoorError),
}

/// The start rules of a step's parameters. The card's own start rules are
/// named first: the start is opened, with no physics, when it is not fresh,
/// and to hold its anchors to the card's own values.
fn step_start_rules(
    state: &AppState,
    domain: &str,
    card: &str,
    parameters: &serde_json::Value,
    carried: &fresh_start::Carried,
) -> Result<(), StartTurned> {
    if state.view.find(domain, card).is_none() {
        return Ok(());
    }
    let Some(object) = parameters.as_object() else {
        return Ok(());
    };
    let (domain, card) = state.view.resolve(domain, card);
    let card_turned = |sentence: String| StartTurned::Card(DoorError::Unprocessable(sentence));
    if let Err(sentence) = fresh_start::check(card, object, object, "parameters", "parameters", carried) {
        Live::open(domain, card, parameters).map_err(card_turned)?;
        return Err(StartTurned::FreshStart(DoorError::Unprocessable(sentence)));
    }
    if fresh_start::has_anchors(card) {
        let live = Live::open(domain, card, parameters).map_err(card_turned)?;
        fresh_start::check_anchors(card, &live, "parameters", carried)
            .map_err(|sentence| StartTurned::FreshStart(DoorError::Unprocessable(sentence)))?;
    }
    Ok(())
}

async fn compute_step_plain(
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
    validate_physics_of(known_card(state, &payload), &payload.parameters).map_err(DoorError::Unprocessable)?;
    // A start that carries a run on arrives with `from`, and the fields
    // `from` carries are the upstream's.
    step_start_rules(state, &payload.domain, &payload.card, &payload.parameters, &carried).map_err(|turned| match turned {
        StartTurned::Card(error) | StartTurned::FreshStart(error) => error,
    })?;
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
        ("energy", "runaway_18650_lco") => {
            let call = RunawayCall::from_parameters(&payload.parameters).map_err(DoorError::Unprocessable)?;
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
        ("materials", "grasp_lift_glass_4tu") => {
            let call = card_grasp_lift_glass_4tu::GraspLiftGlass4tuCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("aerospace", "momentum_jwst_srp") => {
            let call = card_momentum_jwst_srp::MomentumJwstSrpCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("aerospace", "momentum_jwst_dihedral") => {
            let call = card_momentum_jwst_dihedral::MomentumJwstDihedralCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "generator_mmrtg_seebeck") => {
            let call = card_generator_mmrtg_seebeck::GeneratorMmrtgSeebeckCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "reactor_msre_pump") => {
            let call = card_reactor_msre_pump::ReactorMsrePumpCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "reactor_spert3_twin") => {
            let call = card_reactor_spert3_twin::ReactorSpert3TwinCall::from_parameters(&payload.parameters)
                .map_err(DoorError::Unprocessable)?;
            on_pool(state, move || call.run(proof)).await?
        }
        ("energy", "reactor_spert3_ccore_twin") => {
            let call = card_reactor_spert3_ccore_twin::ReactorSpert3CcoreTwinCall::from_parameters(&payload.parameters)
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
        ("energy", "tokamak_vde_mast") => {
            let call = VdeCall::from_parameters(&payload.parameters)
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
                crate::CARD_NOT_RECOGNIZED.to_string(),
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
    JsonBody(payload, _): JsonBody,
) -> Result<Json<VerifyPhysicsResponse>, (StatusCode, String)> {
    compute_step(&state, payload)
        .await
        .map(Json)
        .map_err(door_status)
}

async fn run_trajectory_http(
    State(state): State<AppState>,
    JsonBody(payload, _): JsonBody,
) -> Result<Json<trajectory::TrajectoryResponse>, (StatusCode, String)> {
    trajectory::run(&state, payload)
        .await
        .map(Json)
        .map_err(door_status)
}

async fn run_dataset_http(
    State(state): State<AppState>,
    JsonBody(payload, _): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    dataset::run(&state, payload)
        .await
        .map(json_text_response)
        .map_err(door_status)
}

async fn verify_dataset_http(
    State(state): State<AppState>,
    body: JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    dataset::verify(&state, &body.0, body.text())
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
    JsonBody(payload, _): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    validation::run(&state, payload)
        .await
        .map(json_text_response)
        .map_err(door_status)
}

async fn verify_tape_http(
    State(state): State<AppState>,
    body: JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    trajectory::verify(&state, &body.0, body.text())
        .await
        .map(json_text_response)
        .map_err(door_status)
}

async fn solve_http(
    State(state): State<AppState>,
    JsonBody(payload, _): JsonBody,
) -> Result<Json<solution::SolutionResponse>, (StatusCode, String)> {
    solution::run(&state, payload)
        .await
        .map(Json)
        .map_err(door_status)
}

async fn verify_solution_http(
    State(state): State<AppState>,
    body: JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    solution::verify(&state, &body.0, body.text())
        .await
        .map(json_text_response)
        .map_err(door_status)
}

async fn list_solvers_http() -> Json<serde_json::Value> {
    Json(solvers::list_value())
}

async fn describe_solver_http(
    JsonBody(payload, _): JsonBody,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    solvers::describe_request(&payload)
        .map(Json)
        .map_err(door_status)
}

async fn list_cards_http() -> Json<serde_json::Value> {
    Json(descriptor::list_value())
}

async fn describe_card_http(
    JsonBody(payload, _): JsonBody,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    descriptor::describe_request(&payload)
        .map(Json)
        .map_err(door_status)
}

async fn verify_receipt_http(
    JsonBody(payload, _): JsonBody,
) -> Result<Json<verify_receipt::ReceiptReport>, (StatusCode, String)> {
    verify_receipt::verify(&payload).map(Json).map_err(door_status)
}

async fn cite_http(
    State(state): State<AppState>,
    JsonBody(payload, _): JsonBody,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    cite::cite_in(state.view(), &payload).map(Json).map_err(door_status)
}

async fn find_flip_http(
    State(state): State<AppState>,
    JsonBody(payload, _): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    flip::find(&state, payload).await.map(json_text_response).map_err(door_status)
}

async fn design_run_http(
    State(state): State<AppState>,
    JsonBody(payload, _): JsonBody,
) -> Result<([(axum::http::HeaderName, &'static str); 1], String), (StatusCode, String)> {
    design::design(&state, payload).await.map(json_text_response).map_err(door_status)
}

async fn find_model_http(
    State(state): State<AppState>,
    JsonBody(payload, _): JsonBody,
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

// ---- tokamak_vde_mast: the MAST tokamak's vertical displacement event ----

struct VdeCall {
    card: tokamak_vde_mast::TokamakVdeMast,
    state: tokamak_vde_mast::VdeState,
    load: tokamak_vde_mast::VdeLoad,
    dt_s: f64,
}

/// The sentence for each of the MAST VDE card's own limits, with the number
/// the state reached where there is one.
fn vde_reject(reject: tokamak_vde_mast::StepReject, state: &tokamak_vde_mast::VdeState) -> String {
    match reject {
        tokamak_vde_mast::StepReject::StartOutsideVessel => unsealed(&format!(
            "the displacement {} m is at or beyond a contact height, {} m or {} m, where EFIT's boundary, shifted bodily, meets the limiter: the plasma is not inside the vessel (StartOutsideVessel).",
            message_number(state.displacement_m),
            message_number(tokamak_vde_mast::LOWER_CONTACT_M),
            message_number(tokamak_vde_mast::UPPER_CONTACT_M),
        )),
        tokamak_vde_mast::StepReject::SpeedAtAlfvenSpeed => unsealed(&format!(
            "the plasma's vertical speed is at or above its Alfven speed, {} m/s, at the start or at a stage of a substep, where the wall is tested too: the plasma cannot move as one body faster (SpeedAtAlfvenSpeed).",
            message_number(tokamak_vde_mast::ALFVEN_SPEED_M_S),
        )),
        tokamak_vde_mast::StepReject::PlasmaAtWall => unsealed(&format!(
            "the step carries EFIT's boundary to the limiter, at {} m or {} m: the plasma reaches the wall and the vertical displacement event ends (PlasmaAtWall).",
            message_number(tokamak_vde_mast::UPPER_CONTACT_M),
            message_number(tokamak_vde_mast::LOWER_CONTACT_M),
        )),
        tokamak_vde_mast::StepReject::SubstepsAboveBudget => unsealed(&format!(
            "the step would take more than {} substeps, each 0.02 of the inverse of the bound on its fastest pole (SubstepsAboveBudget).",
            tokamak_vde_mast::MAX_SUBSTEPS,
        )),
        tokamak_vde_mast::StepReject::SpontaneousGeneration => {
            unsealed("the plasma and the two circuits created energy beyond the amplifier's work (SpontaneousGeneration).")
        }
        tokamak_vde_mast::StepReject::FirstLawBreach => {
            unsealed("the plasma and the two circuits lost energy beyond the resistive losses (FirstLawBreach).")
        }
        other => refused("tokamak_vde_mast", other),
    }
}

impl VdeCall {
    fn read_load(params: &serde_json::Value) -> Result<tokamak_vde_mast::VdeLoad, String> {
        Ok(tokamak_vde_mast::VdeLoad {
            p6_voltage_v: required_f64(params, "p6_voltage_v")?,
        })
    }

    fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names("energy", "tokamak_vde_mast"))?;
        enforce_published("energy", "tokamak_vde_mast", Published::Step, params)?;
        let displacement_m = required_f64(params, "displacement_m")?;
        let velocity_m_s = required_f64(params, "velocity_m_s")?;
        let wall_current_a = required_f64(params, "wall_current_a")?;
        let p6_current_a = required_f64(params, "p6_current_a")?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = tokamak_vde_mast::TokamakVdeMast::mast_15045();
        // The step ceiling and floor are locked constants of the card; its
        // fastest pole is resolved inside each step by substeps.
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), "tokamak_vde_mast")?;
        let load = Self::read_load(params)?;
        tokamak_vde_mast::start_in_card(&card, displacement_m, velocity_m_s).map_err(|reject| {
            let mut shown = tokamak_vde_mast::VdeState::try_start(&card, 0.0, 0.0, 0.0, 0.0)
                .expect("the plasma rests at its equilibrium");
            shown.displacement_m = displacement_m;
            vde_reject(reject, &shown)
        })?;
        let state = tokamak_vde_mast::VdeState::try_start(&card, displacement_m, velocity_m_s, wall_current_a, p6_current_a)
            .ok_or_else(|| phase_not_valid("tokamak_vde_mast"))?;
        Ok(Self { card, state, load, dt_s })
    }

    fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names("energy", "tokamak_vde_mast"))?;
        enforce_published("energy", "tokamak_vde_mast", Published::Load, params)?;
        self.load = Self::read_load(params)?;
        Ok(())
    }

    fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        let before = self.state;
        tokamak_vde_mast::step(&self.card, &mut self.state, frame, self.load, self.dt_s).map_err(|reject| {
            let detail = vde_reject(reject, &before);
            StepFailure::from_reject(reject, detail)
        })
    }

    fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame).map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: vde_state(&self.state)?,
        })
    }
}

fn vde_state(state: &tokamak_vde_mast::VdeState) -> Result<serde_json::Value, String> {
    vde_with(state, state_object)
}

pub(crate) fn vde_published_is_finite(state: &tokamak_vde_mast::VdeState) -> Result<(), String> {
    vde_with(state, fields_are_finite)
}

fn vde_with<T>(state: &tokamak_vde_mast::VdeState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("displacement_m", StateField::F64(state.displacement_m)),
        ("velocity_m_s", StateField::F64(state.velocity_m_s)),
        ("wall_current_a", StateField::F64(state.wall_current_a)),
        ("p6_current_a", StateField::F64(state.p6_current_a)),
        ("field_force_n", StateField::F64(state.field_force_n)),
        ("wall_force_n", StateField::F64(state.wall_force_n)),
        ("p6_force_n", StateField::F64(state.p6_force_n)),
        ("wall_power_w", StateField::F64(state.wall_power_w)),
        ("p6_power_w", StateField::F64(state.p6_power_w)),
        ("supply_power_w", StateField::F64(state.supply_power_w)),
        ("kinetic_energy_j", StateField::F64(state.kinetic_energy_j)),
        ("potential_energy_j", StateField::F64(state.potential_energy_j)),
        ("magnetic_energy_j", StateField::F64(state.magnetic_energy_j)),
        ("energy_j", StateField::F64(state.energy_j)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("stored_scale_anchor_j", StateField::F64(state.stored_scale_anchor_j)),
        ("energy_supplied_j", StateField::F64(state.energy_supplied_j)),
        ("energy_dissipated_j", StateField::F64(state.energy_dissipated_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("gap_to_wall_m", StateField::F64(state.gap_to_wall_m)),
        ("substeps", StateField::F64(f64::from(state.substeps))),
        ("past_ideal_wall_limit", StateField::Bool(state.past_ideal_wall_limit)),
    ])
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
/// The bytes of the body are kept beside its value: a verification's long
/// form holds the answer sent byte for byte.
struct JsonBody(serde_json::Value, Bytes);

impl JsonBody {
    /// The body's text as it came, when it is UTF-8 (a JSON body that read
    /// is).
    fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.1).ok()
    }
}

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
            Ok(value) => Ok(JsonBody(value, bytes)),
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
    let routes = Router::new()
        .route("/health", get(health_check))
        .route("/verify_physics", post(verify_physics))
        .route("/compute_step", post(verify_physics))
        .route("/cards", get(list_cards_http))
        .route("/describe_card", post(describe_card_http))
        .route("/solvers", get(list_solvers_http))
        .route("/describe_solver", post(describe_solver_http))
        .route("/solve", post(solve_http))
        .route("/verify_solution", post(verify_solution_http))
        .route("/cite", post(cite_http))
        .route("/find_model", post(find_model_http))
        .route("/find_flip", post(find_flip_http))
        .route("/design_run", post(design_run_http))
        .merge(wide);
    #[cfg(test)]
    let routes = routes.route("/panic_for_test", post(panic_for_test_http));
    routes.layer(axum::middleware::from_fn(answer_a_panic)).with_state(state)
}

/// A test's way to make a handler of the HTTP door panic.
#[cfg(test)]
async fn panic_for_test_http() -> StatusCode {
    panic!("a fault injected by a test inside a handler")
}

/// Every request of the HTTP door runs as a task of its own: a panic in its
/// handler ends that task alone, the request is answered 500 with
/// [`mcp_server::internal_fault`], and the door goes on serving.
async fn answer_a_panic(request: Request<Body>, next: axum::middleware::Next) -> axum::response::Response {
    use axum::response::IntoResponse;
    let path = request.uri().path().trim_start_matches('/').to_string();
    match tokio::spawn(next.run(request)).await {
        Ok(response) => response,
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, mcp_server::internal_fault(&path)).into_response(),
    }
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
                // Re-pinned for 0.10.1: a step above the ceiling carries the name DtAboveCeiling.
                "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the ti64_lattice card (DtAboveCeiling).",
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
                // Re-pinned for 0.10.1: a step below the floor carries the name DtBelowFloor.
                "Physical Violation: 'dt_s' {} is below {} s, the step floor of the ti64_lattice card (DtBelowFloor).",
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
                    // Re-pinned for 0.10.1: a step above the ceiling carries the name DtAboveCeiling.
                    "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the {card} card (DtAboveCeiling).",
                    crate::message_number(above),
                    crate::message_number(ceiling)
                )
            );
        }
    }

    /// A step under a card's floor and one over its ceiling are refused by
    /// name. The 2170 cell, which takes 1 s and nothing else, gives its
    /// kernel's one name on both sides. A step over `π/ω` has a name of its
    /// own, on a card whose ceiling lies above it.
    #[test]
    fn a_step_outside_the_floor_the_ceiling_or_the_nyquist_limit_is_refused_by_name() {
        assert_eq!(
            card_clock(0.5, None, Some(1.0), Some(50.0), "creep_cmsx6").unwrap_err(),
            "Physical Violation: 'dt_s' 0.5 is below 1 s, the step floor of the creep_cmsx6 card (DtBelowFloor)."
        );
        assert_eq!(
            card_clock(60.0, None, Some(1.0), Some(50.0), "creep_cmsx6").unwrap_err(),
            "Physical Violation: 'dt_s' 60 is above 50 s, the step ceiling of the creep_cmsx6 card (DtAboveCeiling)."
        );
        assert_eq!(
            card_clock(0.5, None, Some(1.0), Some(1.0), "cell_2170_m80").unwrap_err(),
            "Physical Violation: 'dt_s' 0.5 is below 1 s, the step floor of the cell_2170_m80 card (StepNotOneSecond)."
        );
        assert_eq!(
            card_clock(2.0, None, Some(1.0), Some(1.0), "cell_2170_m80").unwrap_err(),
            "Physical Violation: 'dt_s' 2 is above 1 s, the step ceiling of the cell_2170_m80 card (StepNotOneSecond)."
        );
        // The other card that takes 1 s alone has no kernel name for it.
        assert_eq!(step_outside_name("cell_2170_model3", false), DT_BELOW_FLOOR);
        assert_eq!(step_outside_name("cell_2170_model3", true), DT_ABOVE_CEILING);
        // Zero and below keep the name every kernel has for them.
        assert_eq!(
            card_clock(0.0, None, Some(1.0), Some(50.0), "creep_cmsx6").unwrap_err(),
            "Physical Violation: 'dt_s' 0 is zero or negative (NonPositiveDt) on the creep_cmsx6 card."
        );
        // A ceiling above pi over omega: omega 1 rad/s puts the limit at pi.
        let nyquist = nyquist_limit_s(1.0);
        assert_eq!(
            card_clock(4.0, Some(1.0), Some(0.5), Some(10.0), "a_card").unwrap_err(),
            format!("Physical Violation: 'dt_s' 4 exceeds the Nyquist limit {} s of the a_card card (DtAboveNyquist).", message_number(nyquist))
        );
        card_clock(3.0, Some(1.0), Some(0.5), Some(10.0), "a_card").unwrap();
        // No served card's ceiling lies above its limit, so none gives that name or lists it.
        for item in descriptor::all() {
            assert!(item.nyquist_s.is_none_or(|nyquist_s| nyquist_s >= item.ceiling_s), "{}", item.card);
            assert!(!descriptor::door_rejects(item).contains(&DT_ABOVE_NYQUIST), "{}", item.card);
        }
    }

    /// Every side of every bound a card publishes has a name, and the card's
    /// description lists it: among its kernel's names or among the gateway's
    /// own for that card.
    #[test]
    fn every_published_bound_of_every_card_has_a_name_the_card_lists() {
        let mut sides = 0;
        for item in descriptor::all() {
            let own = descriptor::door_rejects(item);
            for field in &item.fields {
                for (bound, upper) in [(field.minimum, false), (field.maximum, true)] {
                    if bound.is_none() {
                        continue;
                    }
                    let name = published_reject(item.card, field.name, upper)
                        .unwrap_or_else(|| panic!("{} {} {}: no name", item.card, field.name, if upper { "maximum" } else { "minimum" }));
                    assert!(item.rejects.contains(&name) || own.contains(&name), "{} {}: {name} is not listed", item.card, field.name);
                    assert!(descriptor::reject_meaning(item.card, name).is_some(), "{} {name}", item.card);
                    sides += 1;
                }
            }
            // Each of the gateway's own names has a meaning, and none repeats a kernel name.
            for name in &own {
                assert!(descriptor::reject_meaning(item.card, name).is_some_and(|meaning| meaning.ends_with('.')), "{} {name}", item.card);
                assert!(!item.rejects.contains(name), "{} lists {name} twice", item.card);
            }
        }
        assert!(sides > 150, "{sides}");
        // The names given to the bounds that carried none.
        for (card, field, upper, name) in [
            ("grid_node", "current_a", true, "CurrentOutsideCard"),
            ("grid_node", "load_current_a", false, "CurrentOutsideCard"),
            ("grid_node", "voltage_v", false, "VoltageOutsideCard"),
            ("grid_node", "upstream_voltage_v", true, "VoltageOutsideCard"),
            ("tesseract", "displacement_m", false, "AmplitudeOutsideCard"),
            ("tesseract", "velocity_m_s", true, "AmplitudeOutsideCard"),
            ("tesseract", "energy_defect_j", true, "SpontaneousGeneration"),
            ("tesseract", "energy_defect_j", false, "FirstLawBreach"),
            ("ti64_lattice", "force_n", true, "YieldReached"),
            ("ti64_lattice", "start_force_n", false, "StartNotWritten"),
        ] {
            assert_eq!(published_reject(card, field, upper), Some(name), "{card} {field}");
        }
    }

    /// A negative temperature in kelvin, mass, density or volume of a card's
    /// own field is a value below what the card takes, and the sign rule's
    /// sentence carries the card's own name for it. With no card it carries
    /// none, as before.
    #[test]
    fn a_sign_rule_carries_the_cards_own_name_for_the_field() {
        assert_eq!(
            validate_physics_of(Some("thermal"), &serde_json::json!({ "temperature_k": -50.0 })).unwrap_err(),
            "Physical Violation: Temperature 'temperature_k' cannot be below absolute zero, a negative number of kelvin (TemperatureOutsideCard)."
        );
        assert_eq!(
            validate_physics_of(Some("accumulator_sbo450"), &serde_json::json!({ "initial": { "gas_mass_kg": -1.0 } })).unwrap_err(),
            "Physical Violation: Mass 'gas_mass_kg' cannot be negative (GasMassNotPositive)."
        );
        assert_eq!(
            validate_physics_of(Some("tokamak_jet_dte1"), &serde_json::json!({ "segments": [{ "load": { "line_density_per_m3": -1.0 } }] })).unwrap_err(),
            "Physical Violation: Density 'line_density_per_m3' cannot be negative (DensityNotPositive)."
        );
        assert_eq!(
            validate_physics(&serde_json::json!({ "temperature_k": -50.0 })).unwrap_err(),
            "Physical Violation: Temperature 'temperature_k' cannot be below absolute zero (negative Kelvin)."
        );
        assert_eq!(
            validate_physics_of(None, &serde_json::json!({ "volume_m3": -1.0 })).unwrap_err(),
            "Physical Violation: Volume 'volume_m3' cannot be negative."
        );
        // Every field of every card that a sign rule reads has such a name.
        let mut fields = 0;
        for item in descriptor::all() {
            for field in &item.fields {
                let value = if field.components == 1 { serde_json::json!(-1.0) } else { serde_json::json!([-1.0, 0.0, 0.0]) };
                let request = serde_json::json!({ field.name: value });
                let Err(sentence) = validate_physics_of(Some(item.card), &request) else {
                    continue;
                };
                let name = published_reject(item.card, field.name, false)
                    .unwrap_or_else(|| panic!("{} {}: the sign rule refuses it with no name", item.card, field.name));
                assert!(sentence.ends_with(&format!(" ({name}).")), "{sentence}");
                assert!(item.rejects.contains(&name), "{} {name}", item.card);
                fields += 1;
            }
        }
        assert!(fields >= 12, "{fields}");
    }

    /// A load rule's sentence ends with the name its card lists for the rule.
    #[test]
    fn a_load_rule_is_refused_under_the_name_its_card_lists() {
        let sentence = load_nyquist("FlowAboveNyquist", "flow_m3_s", 2.0, 1.0, 1.0, Some(1.0)).unwrap_err();
        assert_eq!(
            sentence,
            "Physical Violation: 'flow_m3_s' 2 is larger in size than 1, the largest the card accepts at this dt_s (the Nyquist limit of the RK4 step: |flow_m3_s| * dt_s may not exceed the card's impulse limit 1) (FlowAboveNyquist)."
        );
        load_nyquist("FlowAboveNyquist", "flow_m3_s", 1.0, 1.0, 1.0, Some(1.0)).unwrap();
        let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
        let dt_s = cell.gate_dt_s().unwrap();
        let sentence = enforce_force_nyquist([0.0, 1.0e30, 0.0], dt_s, &cell)
            .map_err(|text| named(text, "ForceAboveNyquist"))
            .unwrap_err();
        assert!(sentence.starts_with("Physical Violation: 'force_n[1]' 1e30 N is larger in size than ") && sentence.ends_with(" N*s) (ForceAboveNyquist)."), "{sentence}");
        // The titanium card's kernel has no such name, so the gateway lists it; the four whose kernels do list it themselves.
        let listed = |card: &str, name: &str| {
            let item = descriptor::all().iter().find(|item| item.card == card).unwrap();
            (item.rejects.contains(&name), descriptor::door_rejects(item).contains(&name))
        };
        assert_eq!(listed("ti64_lattice", "ForceAboveNyquist"), (false, true));
        for (card, name) in [
            ("resonance", "ForceAboveNyquist"),
            ("tesseract", "ForceAboveNyquist"),
            ("cable_stretch", "ForceAboveNyquist"),
            ("hydraulic_fluid", "FlowAboveNyquist"),
            ("battery_sag", "CurrentAboveNyquist"),
        ] {
            assert_eq!(listed(card, name), (true, false), "{card}");
        }
    }

    /// Below the negative end of a range that one name of size covers either
    /// way, the sentence says the value is larger in size than the card takes.
    #[test]
    fn below_the_negative_end_of_a_range_named_for_size_the_sentence_says_so() {
        let field = |card: &str, name: &str| {
            let item = descriptor::all().iter().find(|item| item.card == card).unwrap();
            item.fields.iter().find(|field| field.name == name).unwrap()
        };
        assert_eq!(
            published_outside("cantilever_eth", field("cantilever_eth", "deflection_m"), -0.02).unwrap(),
            "Physical Failure: 'deflection_m' -0.02 m is below -0.01875 m, the inclusive minimum of the cantilever_eth card: larger in size than the card takes (EnvelopeAboveCard). That state is not sealed."
        );
        assert_eq!(
            published_outside("cantilever_eth", field("cantilever_eth", "deflection_m"), 0.02).unwrap(),
            "Physical Failure: 'deflection_m' 0.02 m is above 0.01875 m, the inclusive maximum of the cantilever_eth card (EnvelopeAboveCard). That state is not sealed."
        );
        for (card, name) in [("resonance", "displacement_m"), ("resonance", "velocity_m_s"), ("hydraulic_fluid", "flow_m3_s"), ("cantilever_eth", "velocity_m_s")] {
            let sentence = published_outside(card, field(card, name), -1.0e9).unwrap();
            assert!(sentence.contains(&format!("the inclusive minimum of the {card} card: larger in size than the card takes (")), "{sentence}");
        }
        // A name that speaks of no side, and a bound whose two sides have two names, are left as they read.
        for (card, name) in [("tesseract", "displacement_m"), ("grid_node", "current_a"), ("cell_2170_model3", "case_temperature_least_c"), ("cell_18650pf", "held_current_a")] {
            let sentence = published_outside(card, field(card, name), -1.0e9).unwrap();
            assert!(!sentence.contains("larger in size"), "{sentence}");
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
            cell_18650pf::FLAG_SAG_WARNING | cell_18650pf::FLAG_CURRENT_OUTSIDE_PASSED_SPAN
        );
        assert_eq!(
            advisory_flags("energy", "generator_mmrtg_f1"),
            generator_mmrtg_f1::FLAG_PAST_FITTING_SPAN | generator_mmrtg_f1::FLAG_PAST_RECORD
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
            satellite_lageos1::FLAG_EARTH_SHADOW | satellite_lageos1::FLAG_BELOW_COMPARED_RADIUS
        );
        assert_eq!(
            advisory_flags("energy", "reactor_spert3"),
            reactor_spert3::FLAG_PROMPT_SUPERCRITICAL
        );
        assert_eq!(
            advisory_flags("energy", "grid_gb"),
            grid_gb::FLAG_OUTSIDE_COMPARED_TRIPS
                | grid_gb::FLAG_PAST_COMPARED_MINUTE
                | grid_gb::FLAG_BELOW_COMPARED_FREQUENCY
        );
        assert_eq!(
            advisory_flags("signal", "link_gothenburg_10010"),
            link_gothenburg_10010::FLAG_RAINING
        );
        assert_eq!(
            advisory_flags("aerospace", "los_jwst_fine_guide"),
            los_jwst_fine_guide::FLAG_NO_MEASUREMENT
                | los_jwst_fine_guide::FLAG_POINTING_DISTURBED
                | los_jwst_fine_guide::FLAG_BEYOND_FOUR_SIGMA
                | los_jwst_fine_guide::FLAG_RECOVERING
                | los_jwst_fine_guide::FLAG_COUNT_RATE_BEYOND_COMPARED
        );
        assert_eq!(
            advisory_flags("energy", "cell_2170_model3"),
            cell_2170_model3::FLAG_SUSTAINED_ABOVE_1C | cell_2170_model3::FLAG_OUTSIDE_COMPARED_RUNS
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
                | tire_eve_r19_ice::FLAG_BEYOND_COMPARED
                | tire_eve_r19_ice::FLAG_SLIP_SETTLING
        );
        assert_eq!(
            advisory_flags("energy", "cell_2170_m80"),
            cell_2170_m80::FLAG_BLOCK_MET_NO_MARK | cell_2170_m80::FLAG_OUTSIDE_COMPARED_TESTS
        );
        assert_eq!(
            advisory_flags("energy", "accumulator_sbo450"),
            accumulator_sbo450::FLAG_OUTSIDE_COMPARED_RUNS
        );
        assert_eq!(
            advisory_flags("materials", "creep_cmsx6"),
            creep_cmsx6::FLAG_CREEP_RATE_RISING | creep_cmsx6::FLAG_OUTSIDE_MET_RANGE
        );
        assert_eq!(
            advisory_flags("signal", "cantilever_eth"),
            cantilever_eth::FLAG_ENVELOPE_BELOW_COMPARED | cantilever_eth::FLAG_ENVELOPE_ABOVE_COMPARED
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
mod fault_probe;

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

#[cfg(test)]
mod rebuild_probe;
