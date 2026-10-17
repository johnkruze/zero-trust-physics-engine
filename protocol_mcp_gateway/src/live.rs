//! One live specimen. The HTTP door and the MCP door both drive this.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::{
    battery_sag, cable_stretch, cantilever_eth, capsule_stardust, cell_18650pf, cell_2170_m80,
    cell_2170_model3, generator_mmrtg_f1, grid_node, hydraulic_fluid, reactor_spert3,
    reactor_spert3_fuel, reactor_spert3_ccore, reactor_triga_jsi, runaway_18650_lco, resonance, satellite_lageos1, swing, tesseract, thermal, ti64_lattice,
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
    tokamak_vde_mast,
    fingerpad_p02_glass,
};
use serde_json::Value;

use crate::{
    BatteryCall, CableCall, CantileverCall, CapsuleCall, Cell2170Call, Cell2170Model3Call, CellCall,
    CraterCall, crater_published_is_finite, crater_state,
    GeneratorCall, GridCall, GridGbCall,
    CreepCall,
    CapsuleInsightCall,
    HydraulicCall, ReactorCall, ReactorCcoreCall, RunawayCall, runaway_published_is_finite, runaway_state, ReactorFuelCall, ReactorTrigaCall, ResonanceCall, SatelliteCall, SwingCall, TesseractCall, ThermalCall, Ti64Call,
    SolarSystemCall,
    LosCall,
    TokamakCall,
    VdeCall,
    FingerPadCall, fingerpad_published_is_finite, fingerpad_state,
    advisory_flags, battery_published_is_finite, battery_state, cable_published_is_finite,
    cable_state, cantilever_published_is_finite, cantilever_state, capsule_insight_published_is_finite,
    capsule_insight_state, capsule_published_is_finite,
    capsule_state, cell_2170_model3_published_is_finite, cell_2170_model3_state,
    cell_2170_published_is_finite,
    creep_published_is_finite, creep_state,
    cell_2170_state, cell_published_is_finite, cell_state, frame_slots_are_finite, generator_published_is_finite, generator_state,
    grid_gb_published_is_finite, grid_gb_state, grid_published_is_finite, grid_state, hydraulic_published_is_finite, hydraulic_state,
    reactor_ccore_published_is_finite, reactor_ccore_state,
    reactor_fuel_published_is_finite, reactor_fuel_state, reactor_triga_published_is_finite, reactor_triga_state, reactor_published_is_finite, reactor_state, resonance_published_is_finite, resonance_state, satellite_published_is_finite,
    satellite_state, solar_system_published_is_finite, solar_system_state, swing_published_is_finite, swing_state, tesseract_published_is_finite,
    los_published_is_finite, los_state,
    tokamak_published_is_finite, tokamak_state,
    vde_published_is_finite, vde_state,
    tesseract_state, thermal_published_is_finite, thermal_state,
    ti64_published_is_finite, ti64_state, tire_published_is_finite, tire_state, TireCall,
    accumulator_published_is_finite, accumulator_state, AccumulatorCall,
    LinkCall, link_published_is_finite, link_state,
};

pub struct StepFailure {
    pub reject_name: Option<String>,
    pub detail: String,
}

impl StepFailure {
    pub fn from_reject(reject: impl std::fmt::Debug, detail: String) -> Self {
        Self {
            reject_name: Some(format!("{reject:?}")),
            detail,
        }
    }

    pub fn flag(detail: String) -> Self {
        Self {
            reject_name: None,
            detail,
        }
    }
}

#[derive(Debug)]
pub enum DoorError {
    Unprocessable(String),
    TooMany(String),
    NotFound(String),
    Internal(String),
}

impl DoorError {
    pub fn message(&self) -> &str {
        match self {
            DoorError::Unprocessable(message)
            | DoorError::TooMany(message)
            | DoorError::NotFound(message)
            | DoorError::Internal(message) => message,
        }
    }
}

pub enum Live {
    Ti64(Ti64Call),
    Grid(GridCall),
    Swing(SwingCall),
    Battery(BatteryCall),
    Cell(CellCall),
    Cell2170(Cell2170Call),
    Thermal(ThermalCall),
    Resonance(ResonanceCall),
    Tesseract(TesseractCall),
    Hydraulic(HydraulicCall),
    Cable(CableCall),
    Generator(GeneratorCall),
    Capsule(CapsuleCall),
    Satellite(SatelliteCall),
    SolarSystem(SolarSystemCall),
    Reactor(ReactorCall),
    GridGb(GridGbCall),
    Cantilever(CantileverCall),
    Link(LinkCall),
    Los(LosCall),
    Cell2170Model3(Cell2170Model3Call),
    Creep(CreepCall),
    /// Boxed: at 18,680 bytes it set the size of every `Live`, and each arm of
    /// `open` holds a `Live` of its own on the stack in a debug build.
    CapsuleInsight(Box<CapsuleInsightCall>),
    ReactorFuel(ReactorFuelCall),
    Crater(CraterCall),
    GeneratorMmrtgSeebeck(crate::card_generator_mmrtg_seebeck::GeneratorMmrtgSeebeckCall),
    MomentumJwstDihedral(crate::card_momentum_jwst_dihedral::MomentumJwstDihedralCall),
    GraspLiftGlass4tu(crate::card_grasp_lift_glass_4tu::GraspLiftGlass4tuCall),
    MomentumJwstSrp(crate::card_momentum_jwst_srp::MomentumJwstSrpCall),
    Tire(TireCall),
    Accumulator(AccumulatorCall),
    Tokamak(TokamakCall),
    Vde(VdeCall),
    FingerPad(FingerPadCall),
    ReactorTriga(ReactorTrigaCall),
    ReactorCcore(ReactorCcoreCall),
    Runaway(RunawayCall),
}

pub(crate) enum PhaseSnap {
    Ti64(ti64_lattice::Ti64LatticeState),
    Grid(grid_node::GridNodeState),
    Swing(swing::SwingState),
    Battery(battery_sag::PackState),
    Cell(cell_18650pf::CellState),
    Cell2170(cell_2170_m80::CellState),
    Thermal(thermal::ThermalNodeState),
    Resonance(resonance::OscillatorState),
    Tesseract(tesseract::TesseractState),
    Hydraulic(hydraulic_fluid::FluidState),
    Cable(cable_stretch::TendonState),
    Generator(generator_mmrtg_f1::GeneratorState),
    Capsule(capsule_stardust::CapsuleState),
    Satellite(satellite_lageos1::SatelliteState),
    SolarSystem(solar_system::SolarSystemState),
    Reactor(reactor_spert3::ReactorState),
    GridGb(grid_gb::GridState),
    Cantilever(cantilever_eth::CantileverState),
    Link(link_gothenburg_10010::LinkState),
    Los(los_jwst_fine_guide::LosState),
    Cell2170Model3(cell_2170_model3::CellState),
    Creep(creep_cmsx6::CreepState),
    CapsuleInsight(capsule_insight::CapsuleState),
    ReactorFuel(reactor_spert3_fuel::ReactorState),
    Crater(crater_ryugu_sci::CraterState),
    GeneratorMmrtgSeebeck(genesis_kernel::generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState),
    MomentumJwstDihedral(genesis_kernel::momentum_jwst_dihedral::MomentumJwstDihedralState),
    GraspLiftGlass4tu(genesis_kernel::grasp_lift_glass_4tu::GraspLiftGlass4tuState),
    MomentumJwstSrp(genesis_kernel::momentum_jwst_srp::MomentumState),
    Tire(tire_eve_r19_ice::TireState),
    Accumulator(accumulator_sbo450::GasState),
    Tokamak(tokamak_jet_dte1::TokamakState),
    Vde(tokamak_vde_mast::VdeState),
    FingerPad(fingerpad_p02_glass::FingerPadState),
    ReactorTriga(reactor_triga_jsi::ReactorState),
    ReactorCcore(reactor_spert3_ccore::ReactorState),
    Runaway(runaway_18650_lco::RunawayState),
}

impl Live {
    pub fn open(domain: &str, card: &str, params: &Value) -> Result<Self, String> {
        match (domain, card) {
            ("materials", "ti64_lattice") => Ok(Live::Ti64(Ti64Call::from_parameters(params)?)),
            ("energy", "grid_node") => Ok(Live::Grid(GridCall::from_parameters(params)?)),
            ("energy", "swing") => Ok(Live::Swing(SwingCall::from_parameters(params)?)),
            ("energy", "battery_sag") => Ok(Live::Battery(BatteryCall::from_parameters(params)?)),
            ("energy", "cell_18650pf") => Ok(Live::Cell(CellCall::from_parameters(params)?)),
            ("energy", "cell_2170_m80") => {
                Ok(Live::Cell2170(Cell2170Call::from_parameters(params)?))
            }
            ("energy", "thermal") => Ok(Live::Thermal(ThermalCall::from_parameters(params)?)),
            ("signal", "resonance") => {
                Ok(Live::Resonance(ResonanceCall::from_parameters(params)?))
            }
            ("signal", "tesseract") => {
                Ok(Live::Tesseract(TesseractCall::from_parameters(params)?))
            }
            ("materials", "hydraulic_fluid") => {
                Ok(Live::Hydraulic(HydraulicCall::from_parameters(params)?))
            }
            ("materials", "cable_stretch") => Ok(Live::Cable(CableCall::from_parameters(params)?)),
            ("energy", "generator_mmrtg_f1") => {
                Ok(Live::Generator(GeneratorCall::from_parameters(params)?))
            }
            ("aerospace", "capsule_stardust") => {
                Ok(Live::Capsule(CapsuleCall::from_parameters(params)?))
            }
            ("aerospace", "satellite_lageos1") => {
                Ok(Live::Satellite(SatelliteCall::from_parameters(params)?))
            }
            ("aerospace", "solar_system") => {
                Ok(Live::SolarSystem(SolarSystemCall::from_parameters(params)?))
            }
            ("energy", "reactor_spert3") => Ok(Live::Reactor(ReactorCall::from_parameters(params)?)),
            ("energy", "grid_gb") => Ok(Live::GridGb(GridGbCall::from_parameters(params)?)),
            ("signal", "cantilever_eth") => {
                Ok(Live::Cantilever(CantileverCall::from_parameters(params)?))
            }
            ("signal", "link_gothenburg_10010") => {
                Ok(Live::Link(LinkCall::from_parameters(params)?))
            }
            ("aerospace", "los_jwst_fine_guide") => {
                Ok(Live::Los(LosCall::from_parameters(params)?))
            }
            ("energy", "cell_2170_model3") => Ok(Live::Cell2170Model3(
                Cell2170Model3Call::from_parameters(params)?,
            )),
            ("materials", "creep_cmsx6") => Ok(Live::Creep(CreepCall::from_parameters(params)?)),
            ("aerospace", "capsule_insight") => Ok(Live::CapsuleInsight(Box::new(
                CapsuleInsightCall::from_parameters(params)?,
            ))),
            ("energy", "reactor_spert3_fuel") => {
                Ok(Live::ReactorFuel(ReactorFuelCall::from_parameters(params)?))
            }
            ("energy", "reactor_triga_jsi") => {
                Ok(Live::ReactorTriga(ReactorTrigaCall::from_parameters(params)?))
            }
            ("energy", "runaway_18650_lco") => Ok(Live::Runaway(RunawayCall::from_parameters(params)?)),
            ("energy", "reactor_spert3_ccore") => {
                Ok(Live::ReactorCcore(ReactorCcoreCall::from_parameters(params)?))
            }
            ("materials", "grasp_lift_glass_4tu") => Ok(Live::GraspLiftGlass4tu(crate::card_grasp_lift_glass_4tu::GraspLiftGlass4tuCall::from_parameters(params)?)),
            ("aerospace", "momentum_jwst_srp") => Ok(Live::MomentumJwstSrp(crate::card_momentum_jwst_srp::MomentumJwstSrpCall::from_parameters(params)?)),
            ("aerospace", "momentum_jwst_dihedral") => Ok(Live::MomentumJwstDihedral(crate::card_momentum_jwst_dihedral::MomentumJwstDihedralCall::from_parameters(params)?)),
            ("energy", "generator_mmrtg_seebeck") => Ok(Live::GeneratorMmrtgSeebeck(crate::card_generator_mmrtg_seebeck::GeneratorMmrtgSeebeckCall::from_parameters(params)?)),
            ("aerospace", "crater_ryugu_sci") => {
                Ok(Live::Crater(CraterCall::from_parameters(params)?))
            }
            ("materials", "tire_eve_r19_ice") => Ok(Live::Tire(TireCall::from_parameters(params)?)),
            ("energy", "accumulator_sbo450") => {
                Ok(Live::Accumulator(AccumulatorCall::from_parameters(params)?))
            }
            ("energy", "tokamak_vde_mast") => {
                Ok(Live::Vde(VdeCall::from_parameters(params)?))
            }
            ("energy", "tokamak_jet_dte1") => {
                Ok(Live::Tokamak(TokamakCall::from_parameters(params)?))
            }
            ("materials", "fingerpad_p02_glass") => {
                Ok(Live::FingerPad(FingerPadCall::from_parameters(params)?))
            }
            _ => Err(crate::CARD_NOT_RECOGNIZED.to_string()),
        }
    }

    pub fn replace_load(&mut self, params: &Value) -> Result<(), String> {
        match self {
            Live::Ti64(call) => call.replace_load(params),
            Live::Grid(call) => call.replace_load(params),
            Live::Swing(call) => call.replace_load(params),
            Live::Battery(call) => call.replace_load(params),
            Live::Cell(call) => call.replace_load(params),
            Live::Cell2170(call) => call.replace_load(params),
            Live::Thermal(call) => call.replace_load(params),
            Live::Resonance(call) => call.replace_load(params),
            Live::Tesseract(call) => call.replace_load(params),
            Live::Hydraulic(call) => call.replace_load(params),
            Live::Cable(call) => call.replace_load(params),
            Live::Generator(call) => call.replace_load(params),
            Live::Capsule(call) => call.replace_load(params),
            Live::Satellite(call) => call.replace_load(params),
            Live::SolarSystem(call) => call.replace_load(params),
            Live::Reactor(call) => call.replace_load(params),
            Live::GridGb(call) => call.replace_load(params),
            Live::Cantilever(call) => call.replace_load(params),
            Live::Link(call) => call.replace_load(params),
            Live::Los(call) => call.replace_load(params),
            Live::Cell2170Model3(call) => call.replace_load(params),
            Live::Creep(call) => call.replace_load(params),
            Live::CapsuleInsight(call) => call.replace_load(params),
            Live::ReactorFuel(call) => call.replace_load(params),
            Live::ReactorTriga(call) => call.replace_load(params),
            Live::ReactorCcore(call) => call.replace_load(params),
            Live::Runaway(call) => call.replace_load(params),
            Live::Crater(call) => call.replace_load(params),
            Live::GeneratorMmrtgSeebeck(call) => call.replace_load(params),
            Live::MomentumJwstDihedral(call) => call.replace_load(params),
            Live::GraspLiftGlass4tu(call) => call.replace_load(params),
            Live::MomentumJwstSrp(call) => call.replace_load(params),
            Live::Tire(call) => call.replace_load(params),
            Live::Accumulator(call) => call.replace_load(params),
            Live::Tokamak(call) => call.replace_load(params),
            Live::Vde(call) => call.replace_load(params),
            Live::FingerPad(call) => call.replace_load(params),
        }
    }

    pub(crate) fn snapshot(&self) -> PhaseSnap {
        match self {
            Live::Ti64(call) => PhaseSnap::Ti64(call.state),
            Live::Grid(call) => PhaseSnap::Grid(call.state),
            Live::Swing(call) => PhaseSnap::Swing(call.state),
            Live::Battery(call) => PhaseSnap::Battery(call.state),
            Live::Cell(call) => PhaseSnap::Cell(call.state),
            Live::Cell2170(call) => PhaseSnap::Cell2170(call.state),
            Live::Thermal(call) => PhaseSnap::Thermal(call.state),
            Live::Resonance(call) => PhaseSnap::Resonance(call.state),
            Live::Tesseract(call) => PhaseSnap::Tesseract(call.state),
            Live::Hydraulic(call) => PhaseSnap::Hydraulic(call.state),
            Live::Cable(call) => PhaseSnap::Cable(call.state),
            Live::Generator(call) => PhaseSnap::Generator(call.state),
            Live::Capsule(call) => PhaseSnap::Capsule(call.state),
            Live::Satellite(call) => PhaseSnap::Satellite(call.state),
            Live::SolarSystem(call) => PhaseSnap::SolarSystem(call.state),
            Live::Reactor(call) => PhaseSnap::Reactor(call.state),
            Live::GridGb(call) => PhaseSnap::GridGb(call.state),
            Live::Cantilever(call) => PhaseSnap::Cantilever(call.state),
            Live::Link(call) => PhaseSnap::Link(call.state),
            Live::Los(call) => PhaseSnap::Los(call.state),
            Live::Cell2170Model3(call) => PhaseSnap::Cell2170Model3(call.state),
            Live::Creep(call) => PhaseSnap::Creep(call.state),
            Live::CapsuleInsight(call) => PhaseSnap::CapsuleInsight(call.state),
            Live::ReactorFuel(call) => PhaseSnap::ReactorFuel(call.state),
            Live::ReactorTriga(call) => PhaseSnap::ReactorTriga(call.state),
            Live::ReactorCcore(call) => PhaseSnap::ReactorCcore(call.state),
            Live::Runaway(call) => PhaseSnap::Runaway(call.state),
            Live::Crater(call) => PhaseSnap::Crater(call.state),
            Live::GeneratorMmrtgSeebeck(call) => PhaseSnap::GeneratorMmrtgSeebeck(call.state),
            Live::MomentumJwstDihedral(call) => PhaseSnap::MomentumJwstDihedral(call.state),
            Live::GraspLiftGlass4tu(call) => PhaseSnap::GraspLiftGlass4tu(call.state),
            Live::MomentumJwstSrp(call) => PhaseSnap::MomentumJwstSrp(call.state),
            Live::Tire(call) => PhaseSnap::Tire(call.state),
            Live::Accumulator(call) => PhaseSnap::Accumulator(call.state),
            Live::Tokamak(call) => PhaseSnap::Tokamak(call.state),
            Live::Vde(call) => PhaseSnap::Vde(call.state),
            Live::FingerPad(call) => PhaseSnap::FingerPad(call.state),
        }
    }

    pub(crate) fn restore(&mut self, snap: PhaseSnap) {
        match (self, snap) {
            (Live::Ti64(call), PhaseSnap::Ti64(state)) => call.state = state,
            (Live::Grid(call), PhaseSnap::Grid(state)) => call.state = state,
            (Live::Swing(call), PhaseSnap::Swing(state)) => call.state = state,
            (Live::Battery(call), PhaseSnap::Battery(state)) => call.state = state,
            (Live::Cell(call), PhaseSnap::Cell(state)) => call.state = state,
            (Live::Cell2170(call), PhaseSnap::Cell2170(state)) => call.state = state,
            (Live::Thermal(call), PhaseSnap::Thermal(state)) => call.state = state,
            (Live::Resonance(call), PhaseSnap::Resonance(state)) => call.state = state,
            (Live::Tesseract(call), PhaseSnap::Tesseract(state)) => call.state = state,
            (Live::Hydraulic(call), PhaseSnap::Hydraulic(state)) => call.state = state,
            (Live::Cable(call), PhaseSnap::Cable(state)) => call.state = state,
            (Live::Generator(call), PhaseSnap::Generator(state)) => call.state = state,
            (Live::Capsule(call), PhaseSnap::Capsule(state)) => call.state = state,
            (Live::Satellite(call), PhaseSnap::Satellite(state)) => call.state = state,
            (Live::SolarSystem(call), PhaseSnap::SolarSystem(state)) => call.state = state,
            (Live::Reactor(call), PhaseSnap::Reactor(state)) => call.state = state,
            (Live::GridGb(call), PhaseSnap::GridGb(state)) => call.state = state,
            (Live::Cantilever(call), PhaseSnap::Cantilever(state)) => call.state = state,
            (Live::Link(call), PhaseSnap::Link(state)) => call.state = state,
            (Live::Los(call), PhaseSnap::Los(state)) => call.state = state,
            (Live::Cell2170Model3(call), PhaseSnap::Cell2170Model3(state)) => call.state = state,
            (Live::Creep(call), PhaseSnap::Creep(state)) => call.state = state,
            (Live::CapsuleInsight(call), PhaseSnap::CapsuleInsight(state)) => call.state = state,
            (Live::ReactorFuel(call), PhaseSnap::ReactorFuel(state)) => call.state = state,
            (Live::ReactorTriga(call), PhaseSnap::ReactorTriga(state)) => call.state = state,
            (Live::ReactorCcore(call), PhaseSnap::ReactorCcore(state)) => call.state = state,
            (Live::Runaway(call), PhaseSnap::Runaway(state)) => call.state = state,
            (Live::Crater(call), PhaseSnap::Crater(state)) => call.state = state,
            (Live::GeneratorMmrtgSeebeck(call), PhaseSnap::GeneratorMmrtgSeebeck(state)) => call.state = state,
            (Live::MomentumJwstDihedral(call), PhaseSnap::MomentumJwstDihedral(state)) => call.state = state,
            (Live::GraspLiftGlass4tu(call), PhaseSnap::GraspLiftGlass4tu(state)) => call.state = state,
            (Live::MomentumJwstSrp(call), PhaseSnap::MomentumJwstSrp(state)) => call.state = state,
            (Live::Tire(call), PhaseSnap::Tire(state)) => call.state = state,
            (Live::Accumulator(call), PhaseSnap::Accumulator(state)) => call.state = state,
            (Live::Tokamak(call), PhaseSnap::Tokamak(state)) => call.state = state,
            (Live::Vde(call), PhaseSnap::Vde(state)) => call.state = state,
            (Live::FingerPad(call), PhaseSnap::FingerPad(state)) => call.state = state,
            _ => unreachable!("snapshot does not belong to this specimen"),
        }
    }

    pub fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        match self {
            Live::Ti64(call) => call.step_into(frame),
            Live::Grid(call) => call.step_into(frame),
            Live::Swing(call) => call.step_into(frame),
            Live::Battery(call) => call.step_into(frame),
            Live::Cell(call) => call.step_into(frame),
            Live::Cell2170(call) => call.step_into(frame),
            Live::Thermal(call) => call.step_into(frame),
            Live::Resonance(call) => call.step_into(frame),
            Live::Tesseract(call) => call.step_into(frame),
            Live::Hydraulic(call) => call.step_into(frame),
            Live::Cable(call) => call.step_into(frame),
            Live::Generator(call) => call.step_into(frame),
            Live::Capsule(call) => call.step_into(frame),
            Live::Satellite(call) => call.step_into(frame),
            Live::SolarSystem(call) => call.step_into(frame),
            Live::Reactor(call) => call.step_into(frame),
            Live::GridGb(call) => call.step_into(frame),
            Live::Cantilever(call) => call.step_into(frame),
            Live::Link(call) => call.step_into(frame),
            Live::Los(call) => call.step_into(frame),
            Live::Cell2170Model3(call) => call.step_into(frame),
            Live::Creep(call) => call.step_into(frame),
            Live::CapsuleInsight(call) => call.step_into(frame),
            Live::ReactorFuel(call) => call.step_into(frame),
            Live::ReactorTriga(call) => call.step_into(frame),
            Live::ReactorCcore(call) => call.step_into(frame),
            Live::Runaway(call) => call.step_into(frame),
            Live::Crater(call) => call.step_into(frame),
            Live::GeneratorMmrtgSeebeck(call) => call.step_into(frame),
            Live::MomentumJwstDihedral(call) => call.step_into(frame),
            Live::GraspLiftGlass4tu(call) => call.step_into(frame),
            Live::MomentumJwstSrp(call) => call.step_into(frame),
            Live::Tire(call) => call.step_into(frame),
            Live::Accumulator(call) => call.step_into(frame),
            Live::Tokamak(call) => call.step_into(frame),
            Live::Vde(call) => call.step_into(frame),
            Live::FingerPad(call) => call.step_into(frame),
        }
    }

    /// Reject, non-advisory flag, non-finite frame slot, or non-finite published field.
    /// Does not build the JSON state.
    pub fn audit(&self, frame: &LastStateFrame64) -> Result<f64, StepFailure> {
        match self {
            Live::Ti64(call) => {
                ti64_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "materials",
                    "ti64_lattice",
                    call.state.residual_conservation_error,
                )
            }
            Live::Grid(call) => {
                grid_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "grid_node",
                    call.state.residual_conservation_error,
                )
            }
            Live::Swing(call) => {
                swing_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "swing",
                    call.state.residual_conservation_error,
                )
            }
            Live::Battery(call) => {
                battery_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "battery_sag",
                    call.state.residual_conservation_error,
                )
            }
            Live::Cell(call) => {
                cell_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "cell_18650pf",
                    call.state.residual_conservation_error,
                )
            }
            Live::Cell2170(call) => {
                cell_2170_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "cell_2170_m80",
                    call.state.residual_conservation_error,
                )
            }
            Live::Thermal(call) => {
                thermal_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "thermal",
                    call.state.residual_conservation_error,
                )
            }
            Live::Resonance(call) => {
                resonance_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "signal",
                    "resonance",
                    call.state.residual_conservation_error,
                )
            }
            Live::Tesseract(call) => {
                tesseract_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "signal",
                    "tesseract",
                    call.state.residual_conservation_error,
                )
            }
            Live::Hydraulic(call) => {
                hydraulic_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "materials",
                    "hydraulic_fluid",
                    call.state.residual_conservation_error,
                )
            }
            Live::Cable(call) => {
                cable_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "materials",
                    "cable_stretch",
                    call.state.residual_conservation_error,
                )
            }
            Live::Generator(call) => {
                generator_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "generator_mmrtg_f1",
                    call.state.residual_conservation_error,
                )
            }
            Live::Capsule(call) => {
                capsule_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "aerospace",
                    "capsule_stardust",
                    call.state.residual_conservation_error,
                )
            }
            Live::Satellite(call) => {
                satellite_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "aerospace",
                    "satellite_lageos1",
                    call.state.residual_conservation_error,
                )
            }
            Live::SolarSystem(call) => {
                solar_system_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "aerospace",
                    "solar_system",
                    call.state.residual_conservation_error,
                )
            }
            Live::Reactor(call) => {
                reactor_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "reactor_spert3",
                    call.state.residual_conservation_error,
                )
            }
            Live::GridGb(call) => {
                grid_gb_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "grid_gb",
                    call.state.residual_conservation_error,
                )
            }
            Live::Cantilever(call) => {
                cantilever_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "signal",
                    "cantilever_eth",
                    call.state.residual_conservation_error,
                )
            }
            Live::Link(call) => {
                link_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "signal",
                    "link_gothenburg_10010",
                    call.state.residual_conservation_error,
                )
            }
            Live::Los(call) => {
                los_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "aerospace",
                    "los_jwst_fine_guide",
                    call.state.residual_conservation_error,
                )
            }
            Live::Cell2170Model3(call) => {
                cell_2170_model3_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "cell_2170_model3",
                    call.state.residual_conservation_error,
                )
            }
            Live::Creep(call) => {
                creep_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "materials",
                    "creep_cmsx6",
                    call.state.residual_conservation_error,
                )
            }
            Live::CapsuleInsight(call) => {
                capsule_insight_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "aerospace",
                    "capsule_insight",
                    call.state.residual_conservation_error,
                )
            }
            Live::ReactorTriga(call) => {
                reactor_triga_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "reactor_triga_jsi",
                    call.state.residual_conservation_error,
                )
            }
            Live::Runaway(call) => {
                runaway_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "runaway_18650_lco", call.state.residual_conservation_error)
            }
            Live::ReactorCcore(call) => {
                reactor_ccore_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "reactor_spert3_ccore",
                    call.state.residual_conservation_error,
                )
            }
            Live::ReactorFuel(call) => {
                reactor_fuel_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "reactor_spert3_fuel",
                    call.state.residual_conservation_error,
                )
            }
            Live::GraspLiftGlass4tu(call) => {
                crate::card_grasp_lift_glass_4tu::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "materials", "grasp_lift_glass_4tu", call.state.residual_conservation_error)
            }
            Live::MomentumJwstSrp(call) => {
                crate::card_momentum_jwst_srp::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "aerospace", "momentum_jwst_srp", call.state.residual_conservation_error)
            }
            Live::MomentumJwstDihedral(call) => {
                crate::card_momentum_jwst_dihedral::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "aerospace", "momentum_jwst_dihedral", call.state.residual_conservation_error)
            }
            Live::GeneratorMmrtgSeebeck(call) => {
                crate::card_generator_mmrtg_seebeck::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "generator_mmrtg_seebeck", call.state.residual_conservation_error)
            }
            Live::Crater(call) => {
                crater_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "aerospace",
                    "crater_ryugu_sci",
                    call.state.residual_conservation_error,
                )
            }
            Live::Tire(call) => {
                tire_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "materials",
                    "tire_eve_r19_ice",
                    call.state.residual_conservation_error,
                )
            }
            Live::Accumulator(call) => {
                accumulator_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "accumulator_sbo450",
                    call.state.residual_conservation_error,
                )
            }
            Live::Tokamak(call) => {
                tokamak_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "tokamak_jet_dte1",
                    call.state.residual_conservation_error,
                )
            }
            Live::Vde(call) => {
                vde_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "energy",
                    "tokamak_vde_mast",
                    call.state.residual_conservation_error,
                )
            }
            Live::FingerPad(call) => {
                fingerpad_published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(
                    frame,
                    "materials",
                    "fingerpad_p02_glass",
                    call.state.residual_conservation_error,
                )
            }
        }
    }

    /// The name of the end its card declares that the landed state stands
    /// at, for a card whose end is a step that lands: the capsules, whose
    /// step that first crosses the flown deployment lands (`src/ends.rs`).
    /// A run whose last step lands there has reached that end (rule 13).
    /// Every other card's end is a step that is not taken, so `None`.
    pub(crate) fn landed_at_end(&self) -> Option<&'static str> {
        match self {
            Live::Capsule(call) => call.past_flight_deployment().then_some("PastFlightDeployment"),
            Live::CapsuleInsight(call) => call.past_flight_deployment().then_some("PastFlightDeployment"),
            _ => None,
        }
    }

    /// JSON state of the specimen as it stands. One call, after the run stops.
    pub fn publish(&self) -> Result<Value, StepFailure> {
        match self {
            Live::Ti64(call) => ti64_state(&call.state).map_err(StepFailure::flag),
            Live::Grid(call) => grid_state(&call.state).map_err(StepFailure::flag),
            Live::Swing(call) => swing_state(&call.state).map_err(StepFailure::flag),
            Live::Battery(call) => battery_state(&call.state).map_err(StepFailure::flag),
            Live::Cell(call) => cell_state(&call.state).map_err(StepFailure::flag),
            Live::Cell2170(call) => cell_2170_state(&call.state).map_err(StepFailure::flag),
            Live::Thermal(call) => thermal_state(&call.state).map_err(StepFailure::flag),
            Live::Resonance(call) => resonance_state(&call.state).map_err(StepFailure::flag),
            Live::Tesseract(call) => tesseract_state(&call.state).map_err(StepFailure::flag),
            Live::Hydraulic(call) => hydraulic_state(&call.state).map_err(StepFailure::flag),
            Live::Cable(call) => cable_state(&call.state).map_err(StepFailure::flag),
            Live::Generator(call) => generator_state(&call.state).map_err(StepFailure::flag),
            Live::Capsule(call) => capsule_state(&call.state).map_err(StepFailure::flag),
            Live::Satellite(call) => satellite_state(&call.state).map_err(StepFailure::flag),
            Live::SolarSystem(call) => solar_system_state(&call.state).map_err(StepFailure::flag),
            Live::Reactor(call) => reactor_state(&call.state).map_err(StepFailure::flag),
            Live::GridGb(call) => grid_gb_state(&call.state).map_err(StepFailure::flag),
            Live::Cantilever(call) => cantilever_state(&call.state).map_err(StepFailure::flag),
            Live::Link(call) => link_state(&call.state).map_err(StepFailure::flag),
            Live::Los(call) => los_state(&call.state).map_err(StepFailure::flag),
            Live::Cell2170Model3(call) => {
                cell_2170_model3_state(&call.state).map_err(StepFailure::flag)
            }
            Live::Creep(call) => creep_state(&call.state).map_err(StepFailure::flag),
            Live::CapsuleInsight(call) => {
                capsule_insight_state(&call.state).map_err(StepFailure::flag)
            }
            Live::ReactorFuel(call) => reactor_fuel_state(&call.state).map_err(StepFailure::flag),
            Live::ReactorTriga(call) => reactor_triga_state(&call.state).map_err(StepFailure::flag),
            Live::ReactorCcore(call) => reactor_ccore_state(&call.state).map_err(StepFailure::flag),
            Live::Runaway(call) => runaway_state(&call.state).map_err(StepFailure::flag),
            Live::Crater(call) => crater_state(&call.state).map_err(StepFailure::flag),
            Live::GeneratorMmrtgSeebeck(call) => crate::card_generator_mmrtg_seebeck::state(&call.state).map_err(StepFailure::flag),
            Live::MomentumJwstDihedral(call) => crate::card_momentum_jwst_dihedral::state(&call.state).map_err(StepFailure::flag),
            Live::GraspLiftGlass4tu(call) => crate::card_grasp_lift_glass_4tu::state(&call.state).map_err(StepFailure::flag),
            Live::MomentumJwstSrp(call) => crate::card_momentum_jwst_srp::state(&call.state).map_err(StepFailure::flag),
            Live::Tire(call) => tire_state(&call.state).map_err(StepFailure::flag),
            Live::Accumulator(call) => accumulator_state(&call.state).map_err(StepFailure::flag),
            Live::Tokamak(call) => tokamak_state(&call.state).map_err(StepFailure::flag),
            Live::Vde(call) => vde_state(&call.state).map_err(StepFailure::flag),
            Live::FingerPad(call) => fingerpad_state(&call.state).map_err(StepFailure::flag),
        }
    }
}

fn finish_audit(
    frame: &LastStateFrame64,
    domain: &str,
    card: &str,
    residual: f64,
) -> Result<f64, StepFailure> {
    if !residual.is_finite() {
        return Err(StepFailure::flag(
            "Physical Violation: residual_conservation_error is not finite.".to_string(),
        ));
    }
    if !frame_slots_are_finite(frame) {
        return Err(StepFailure::flag(
            "Physical Violation: the integrated frame has a non-finite slot. That state is not sealed."
                .to_string(),
        ));
    }
    let allowed = advisory_flags(domain, card);
    if frame.flags & !allowed != 0 {
        if domain == "energy" && card == "grid_node" {
            return Err(StepFailure::flag(
                "Physical Failure: the grid frame carries a failure flag other than surge. That state is not sealed.".to_string(),
            ));
        }
        return Err(StepFailure::flag(
            "Physical Failure: the frame carries a failure flag. That state is not sealed."
                .to_string(),
        ));
    }
    Ok(residual)
}
