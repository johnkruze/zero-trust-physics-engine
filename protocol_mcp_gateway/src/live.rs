//! One live specimen. The HTTP door and the MCP door both drive this.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::{
    battery_sag, cable_stretch, cantilever_eth, capsule_stardust, cell_18650pf, generator_mmrtg_f1, grid_node, hydraulic_fluid, reactor_triga_jsi, resonance, satellite_lageos1, swing, tesseract, thermal, ti64_lattice,
    creep_cmsx6,
    capsule_insight,
    crater_ryugu_sci,
    link_gothenburg_10010,
    solar_system,
    tire_eve_r19_ice,
    accumulator_sbo450,
    los_jwst_fine_guide,
    tokamak_vde_mast,
    fingerpad_p02_glass,
};
use serde_json::Value;

use crate::{
    BatteryCall, CableCall, CantileverCall, CapsuleCall, CellCall,
    CraterCall, crater_published_is_finite, crater_state,
    GeneratorCall, GridCall, CreepCall,
    CapsuleInsightCall,
    HydraulicCall, ReactorTrigaCall, ResonanceCall, SatelliteCall, SwingCall, TesseractCall, ThermalCall, Ti64Call,
    SolarSystemCall,
    LosCall,
    VdeCall,
    FingerPadCall, fingerpad_published_is_finite, fingerpad_state,
    advisory_flags, battery_published_is_finite, battery_state, cable_published_is_finite,
    cable_state, cantilever_published_is_finite, cantilever_state, capsule_insight_published_is_finite,
    capsule_insight_state, capsule_published_is_finite,
    capsule_state, creep_published_is_finite, creep_state,
    cell_published_is_finite, cell_state, frame_slots_are_finite, generator_published_is_finite, generator_state,
    grid_published_is_finite, grid_state, hydraulic_published_is_finite, hydraulic_state,
    reactor_triga_published_is_finite, reactor_triga_state, resonance_published_is_finite, resonance_state, satellite_published_is_finite,
    satellite_state, solar_system_published_is_finite, solar_system_state, swing_published_is_finite, swing_state, tesseract_published_is_finite,
    los_published_is_finite, los_state,
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
    Thermal(ThermalCall),
    Resonance(ResonanceCall),
    Tesseract(TesseractCall),
    Hydraulic(HydraulicCall),
    Cable(CableCall),
    Generator(GeneratorCall),
    Capsule(CapsuleCall),
    Satellite(SatelliteCall),
    SolarSystem(SolarSystemCall),
    Cantilever(CantileverCall),
    Link(LinkCall),
    Los(LosCall),
    Creep(CreepCall),
    /// Boxed: at 18,680 bytes it set the size of every `Live`, and each arm of
    /// `open` holds a `Live` of its own on the stack in a debug build.
    CapsuleInsight(Box<CapsuleInsightCall>),
    Crater(CraterCall),
    GridGbTwin(crate::card_grid_gb_twin::GridGbTwinCall),
    TokamakJetDte1Twin(crate::card_tokamak_jet_dte1_twin::TokamakJetDte1TwinCall),
    Runaway18650LcoTwin(crate::card_runaway_18650_lco_twin::Runaway18650LcoTwinCall),
    ReactorMsrePump(crate::card_reactor_msre_pump::ReactorMsrePumpCall),
    ReactorSpert3Twin(crate::card_reactor_spert3_twin::ReactorSpert3TwinCall),
    Cell2170Twin(crate::card_cell_2170_twin::Cell2170TwinCall),
    ReactorSpert3CcorePlate(crate::card_reactor_spert3_ccore_plate::ReactorSpert3CcorePlateCall),
    GeneratorMmrtgSeebeck(crate::card_generator_mmrtg_seebeck::GeneratorMmrtgSeebeckCall),
    MomentumJwstDihedral(crate::card_momentum_jwst_dihedral::MomentumJwstDihedralCall),
    GraspLiftGlass4tu(crate::card_grasp_lift_glass_4tu::GraspLiftGlass4tuCall),
    MomentumJwstSrp(crate::card_momentum_jwst_srp::MomentumJwstSrpCall),
    Tire(TireCall),
    Accumulator(AccumulatorCall),
    Vde(VdeCall),
    FingerPad(FingerPadCall),
    ReactorTriga(ReactorTrigaCall),
}

pub(crate) enum PhaseSnap {
    Ti64(ti64_lattice::Ti64LatticeState),
    Grid(grid_node::GridNodeState),
    Swing(swing::SwingState),
    Battery(battery_sag::PackState),
    Cell(cell_18650pf::CellState),
    Thermal(thermal::ThermalNodeState),
    Resonance(resonance::OscillatorState),
    Tesseract(tesseract::TesseractState),
    Hydraulic(hydraulic_fluid::FluidState),
    Cable(cable_stretch::TendonState),
    Generator(generator_mmrtg_f1::GeneratorState),
    Capsule(capsule_stardust::CapsuleState),
    Satellite(satellite_lageos1::SatelliteState),
    SolarSystem(solar_system::SolarSystemState),
    Cantilever(cantilever_eth::CantileverState),
    Link(link_gothenburg_10010::LinkState),
    Los(los_jwst_fine_guide::LosState),
    Creep(creep_cmsx6::CreepState),
    CapsuleInsight(capsule_insight::CapsuleState),
    Crater(crater_ryugu_sci::CraterState),
    GridGbTwin(genesis_kernel::grid_gb_twin::GridGbTwinState),
    TokamakJetDte1Twin(genesis_kernel::tokamak_jet_dte1_twin::TwinState),
    Runaway18650LcoTwin(genesis_kernel::runaway_18650_lco_twin::RunawayState),
    ReactorMsrePump(genesis_kernel::reactor_msre_pump::ReactorMsrePumpState),
    ReactorSpert3Twin(genesis_kernel::reactor_spert3_twin::ReactorState),
    Cell2170Twin(genesis_kernel::cell_2170_twin::CellState),
    ReactorSpert3CcorePlate(genesis_kernel::reactor_spert3_ccore_plate::ReactorState),
    GeneratorMmrtgSeebeck(genesis_kernel::generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState),
    MomentumJwstDihedral(genesis_kernel::momentum_jwst_dihedral::MomentumJwstDihedralState),
    GraspLiftGlass4tu(genesis_kernel::grasp_lift_glass_4tu::GraspLiftGlass4tuState),
    MomentumJwstSrp(genesis_kernel::momentum_jwst_srp::MomentumState),
    Tire(tire_eve_r19_ice::TireState),
    Accumulator(accumulator_sbo450::GasState),
    Vde(tokamak_vde_mast::VdeState),
    FingerPad(fingerpad_p02_glass::FingerPadState),
    ReactorTriga(reactor_triga_jsi::ReactorState),
}

impl Live {
    pub fn open(domain: &str, card: &str, params: &Value) -> Result<Self, String> {
        match (domain, card) {
            ("materials", "ti64_lattice") => Ok(Live::Ti64(Ti64Call::from_parameters(params)?)),
            ("energy", "grid_node") => Ok(Live::Grid(GridCall::from_parameters(params)?)),
            ("energy", "swing") => Ok(Live::Swing(SwingCall::from_parameters(params)?)),
            ("energy", "battery_sag") => Ok(Live::Battery(BatteryCall::from_parameters(params)?)),
            ("energy", "cell_18650pf") => Ok(Live::Cell(CellCall::from_parameters(params)?)),
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
            ("signal", "cantilever_eth") => {
                Ok(Live::Cantilever(CantileverCall::from_parameters(params)?))
            }
            ("signal", "link_gothenburg_10010") => {
                Ok(Live::Link(LinkCall::from_parameters(params)?))
            }
            ("aerospace", "los_jwst_fine_guide") => {
                Ok(Live::Los(LosCall::from_parameters(params)?))
            }
            ("materials", "creep_cmsx6") => Ok(Live::Creep(CreepCall::from_parameters(params)?)),
            ("aerospace", "capsule_insight") => Ok(Live::CapsuleInsight(Box::new(
                CapsuleInsightCall::from_parameters(params)?,
            ))),
            ("energy", "reactor_triga_jsi") => {
                Ok(Live::ReactorTriga(ReactorTrigaCall::from_parameters(params)?))
            }
            ("materials", "grasp_lift_glass_4tu") => Ok(Live::GraspLiftGlass4tu(crate::card_grasp_lift_glass_4tu::GraspLiftGlass4tuCall::from_parameters(params)?)),
            ("aerospace", "momentum_jwst_srp") => Ok(Live::MomentumJwstSrp(crate::card_momentum_jwst_srp::MomentumJwstSrpCall::from_parameters(params)?)),
            ("aerospace", "momentum_jwst_dihedral") => Ok(Live::MomentumJwstDihedral(crate::card_momentum_jwst_dihedral::MomentumJwstDihedralCall::from_parameters(params)?)),
            ("energy", "generator_mmrtg_seebeck") => Ok(Live::GeneratorMmrtgSeebeck(crate::card_generator_mmrtg_seebeck::GeneratorMmrtgSeebeckCall::from_parameters(params)?)),
            ("energy", "reactor_msre_pump") => Ok(Live::ReactorMsrePump(crate::card_reactor_msre_pump::ReactorMsrePumpCall::from_parameters(params)?)),
            ("energy", "reactor_spert3") => Ok(Live::ReactorSpert3Twin(crate::card_reactor_spert3_twin::ReactorSpert3TwinCall::from_parameters(params)?)),
            ("energy", "grid_gb") => Ok(Live::GridGbTwin(crate::card_grid_gb_twin::GridGbTwinCall::from_parameters(params)?)),
            ("energy", "cell_2170_model3") => Ok(Live::Cell2170Twin(crate::card_cell_2170_twin::Cell2170TwinCall::from_parameters(params)?)),
            ("energy", "tokamak_jet_dte1") => Ok(Live::TokamakJetDte1Twin(crate::card_tokamak_jet_dte1_twin::TokamakJetDte1TwinCall::from_parameters(params)?)),
            ("energy", "runaway_18650_lco") => Ok(Live::Runaway18650LcoTwin(crate::card_runaway_18650_lco_twin::Runaway18650LcoTwinCall::from_parameters(params)?)),
            ("energy", "reactor_spert3_ccore") => Ok(Live::ReactorSpert3CcorePlate(crate::card_reactor_spert3_ccore_plate::ReactorSpert3CcorePlateCall::from_parameters(params)?)),
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
            Live::Thermal(call) => call.replace_load(params),
            Live::Resonance(call) => call.replace_load(params),
            Live::Tesseract(call) => call.replace_load(params),
            Live::Hydraulic(call) => call.replace_load(params),
            Live::Cable(call) => call.replace_load(params),
            Live::Generator(call) => call.replace_load(params),
            Live::Capsule(call) => call.replace_load(params),
            Live::Satellite(call) => call.replace_load(params),
            Live::SolarSystem(call) => call.replace_load(params),
            Live::Cantilever(call) => call.replace_load(params),
            Live::Link(call) => call.replace_load(params),
            Live::Los(call) => call.replace_load(params),
            Live::Creep(call) => call.replace_load(params),
            Live::CapsuleInsight(call) => call.replace_load(params),
            Live::ReactorTriga(call) => call.replace_load(params),
            Live::Crater(call) => call.replace_load(params),
            Live::GridGbTwin(call) => call.replace_load(params),
            Live::TokamakJetDte1Twin(call) => call.replace_load(params),
            Live::Runaway18650LcoTwin(call) => call.replace_load(params),
            Live::ReactorMsrePump(call) => call.replace_load(params),
            Live::ReactorSpert3Twin(call) => call.replace_load(params),
            Live::Cell2170Twin(call) => call.replace_load(params),
            Live::ReactorSpert3CcorePlate(call) => call.replace_load(params),
            Live::GeneratorMmrtgSeebeck(call) => call.replace_load(params),
            Live::MomentumJwstDihedral(call) => call.replace_load(params),
            Live::GraspLiftGlass4tu(call) => call.replace_load(params),
            Live::MomentumJwstSrp(call) => call.replace_load(params),
            Live::Tire(call) => call.replace_load(params),
            Live::Accumulator(call) => call.replace_load(params),
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
            Live::Thermal(call) => PhaseSnap::Thermal(call.state),
            Live::Resonance(call) => PhaseSnap::Resonance(call.state),
            Live::Tesseract(call) => PhaseSnap::Tesseract(call.state),
            Live::Hydraulic(call) => PhaseSnap::Hydraulic(call.state),
            Live::Cable(call) => PhaseSnap::Cable(call.state),
            Live::Generator(call) => PhaseSnap::Generator(call.state),
            Live::Capsule(call) => PhaseSnap::Capsule(call.state),
            Live::Satellite(call) => PhaseSnap::Satellite(call.state),
            Live::SolarSystem(call) => PhaseSnap::SolarSystem(call.state),
            Live::Cantilever(call) => PhaseSnap::Cantilever(call.state),
            Live::Link(call) => PhaseSnap::Link(call.state),
            Live::Los(call) => PhaseSnap::Los(call.state),
            Live::Creep(call) => PhaseSnap::Creep(call.state),
            Live::CapsuleInsight(call) => PhaseSnap::CapsuleInsight(call.state),
            Live::ReactorTriga(call) => PhaseSnap::ReactorTriga(call.state),
            Live::Crater(call) => PhaseSnap::Crater(call.state),
            Live::GridGbTwin(call) => PhaseSnap::GridGbTwin(call.state),
            Live::TokamakJetDte1Twin(call) => PhaseSnap::TokamakJetDte1Twin(call.state),
            Live::Runaway18650LcoTwin(call) => PhaseSnap::Runaway18650LcoTwin(call.state),
            Live::ReactorMsrePump(call) => PhaseSnap::ReactorMsrePump(call.state),
            Live::ReactorSpert3Twin(call) => PhaseSnap::ReactorSpert3Twin(call.state),
            Live::Cell2170Twin(call) => PhaseSnap::Cell2170Twin(call.state),
            Live::ReactorSpert3CcorePlate(call) => PhaseSnap::ReactorSpert3CcorePlate(call.state),
            Live::GeneratorMmrtgSeebeck(call) => PhaseSnap::GeneratorMmrtgSeebeck(call.state),
            Live::MomentumJwstDihedral(call) => PhaseSnap::MomentumJwstDihedral(call.state),
            Live::GraspLiftGlass4tu(call) => PhaseSnap::GraspLiftGlass4tu(call.state),
            Live::MomentumJwstSrp(call) => PhaseSnap::MomentumJwstSrp(call.state),
            Live::Tire(call) => PhaseSnap::Tire(call.state),
            Live::Accumulator(call) => PhaseSnap::Accumulator(call.state),
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
            (Live::Thermal(call), PhaseSnap::Thermal(state)) => call.state = state,
            (Live::Resonance(call), PhaseSnap::Resonance(state)) => call.state = state,
            (Live::Tesseract(call), PhaseSnap::Tesseract(state)) => call.state = state,
            (Live::Hydraulic(call), PhaseSnap::Hydraulic(state)) => call.state = state,
            (Live::Cable(call), PhaseSnap::Cable(state)) => call.state = state,
            (Live::Generator(call), PhaseSnap::Generator(state)) => call.state = state,
            (Live::Capsule(call), PhaseSnap::Capsule(state)) => call.state = state,
            (Live::Satellite(call), PhaseSnap::Satellite(state)) => call.state = state,
            (Live::SolarSystem(call), PhaseSnap::SolarSystem(state)) => call.state = state,
            (Live::Cantilever(call), PhaseSnap::Cantilever(state)) => call.state = state,
            (Live::Link(call), PhaseSnap::Link(state)) => call.state = state,
            (Live::Los(call), PhaseSnap::Los(state)) => call.state = state,
            (Live::Creep(call), PhaseSnap::Creep(state)) => call.state = state,
            (Live::CapsuleInsight(call), PhaseSnap::CapsuleInsight(state)) => call.state = state,
            (Live::ReactorTriga(call), PhaseSnap::ReactorTriga(state)) => call.state = state,
            (Live::Crater(call), PhaseSnap::Crater(state)) => call.state = state,
            (Live::GridGbTwin(call), PhaseSnap::GridGbTwin(state)) => call.state = state,
            (Live::TokamakJetDte1Twin(call), PhaseSnap::TokamakJetDte1Twin(state)) => call.state = state,
            (Live::Runaway18650LcoTwin(call), PhaseSnap::Runaway18650LcoTwin(state)) => call.state = state,
            (Live::ReactorMsrePump(call), PhaseSnap::ReactorMsrePump(state)) => call.state = state,
            (Live::ReactorSpert3Twin(call), PhaseSnap::ReactorSpert3Twin(state)) => call.state = state,
            (Live::Cell2170Twin(call), PhaseSnap::Cell2170Twin(state)) => call.state = state,
            (Live::ReactorSpert3CcorePlate(call), PhaseSnap::ReactorSpert3CcorePlate(state)) => call.state = state,
            (Live::GeneratorMmrtgSeebeck(call), PhaseSnap::GeneratorMmrtgSeebeck(state)) => call.state = state,
            (Live::MomentumJwstDihedral(call), PhaseSnap::MomentumJwstDihedral(state)) => call.state = state,
            (Live::GraspLiftGlass4tu(call), PhaseSnap::GraspLiftGlass4tu(state)) => call.state = state,
            (Live::MomentumJwstSrp(call), PhaseSnap::MomentumJwstSrp(state)) => call.state = state,
            (Live::Tire(call), PhaseSnap::Tire(state)) => call.state = state,
            (Live::Accumulator(call), PhaseSnap::Accumulator(state)) => call.state = state,
            (Live::Vde(call), PhaseSnap::Vde(state)) => call.state = state,
            (Live::FingerPad(call), PhaseSnap::FingerPad(state)) => call.state = state,
            _ => unreachable!("snapshot does not belong to this specimen"),
        }
    }

    /// The limits the step just landed is past, from the state `before` it
    /// (a [`Live::snapshot`] taken before the step) to the state now held
    /// (`crate::exceedance`).
    pub(crate) fn exceedances(&self, before: &PhaseSnap) -> genesis_kernel::exceedance::Exceedances {
        match (self, before) {
            (Live::Ti64(call), PhaseSnap::Ti64(before)) => call.exceedances(before),
            (Live::Grid(call), PhaseSnap::Grid(before)) => call.exceedances(before),
            (Live::Swing(call), PhaseSnap::Swing(before)) => call.exceedances(before),
            (Live::Battery(call), PhaseSnap::Battery(before)) => call.exceedances(before),
            (Live::Cell(call), PhaseSnap::Cell(before)) => call.exceedances(before),
            (Live::Thermal(call), PhaseSnap::Thermal(before)) => call.exceedances(before),
            (Live::Resonance(call), PhaseSnap::Resonance(before)) => call.exceedances(before),
            (Live::Tesseract(call), PhaseSnap::Tesseract(before)) => call.exceedances(before),
            (Live::Hydraulic(call), PhaseSnap::Hydraulic(before)) => call.exceedances(before),
            (Live::Cable(call), PhaseSnap::Cable(before)) => call.exceedances(before),
            (Live::Generator(call), PhaseSnap::Generator(before)) => call.exceedances(before),
            (Live::Capsule(call), PhaseSnap::Capsule(before)) => call.exceedances(before),
            (Live::Satellite(call), PhaseSnap::Satellite(before)) => call.exceedances(before),
            (Live::SolarSystem(call), PhaseSnap::SolarSystem(before)) => call.exceedances(before),
            (Live::Cantilever(call), PhaseSnap::Cantilever(before)) => call.exceedances(before),
            (Live::Link(call), PhaseSnap::Link(before)) => call.exceedances(before),
            (Live::Los(call), PhaseSnap::Los(before)) => call.exceedances(before),
            (Live::Creep(call), PhaseSnap::Creep(before)) => call.exceedances(before),
            (Live::CapsuleInsight(call), PhaseSnap::CapsuleInsight(before)) => call.exceedances(before),
            (Live::ReactorTriga(call), PhaseSnap::ReactorTriga(before)) => call.exceedances(before),
            (Live::Crater(call), PhaseSnap::Crater(before)) => call.exceedances(before),
            (Live::ReactorMsrePump(call), PhaseSnap::ReactorMsrePump(before)) => call.exceedances(before),
            (Live::ReactorSpert3Twin(call), PhaseSnap::ReactorSpert3Twin(before)) => call.exceedances(before),
            (Live::GeneratorMmrtgSeebeck(call), PhaseSnap::GeneratorMmrtgSeebeck(before)) => call.exceedances(before),
            (Live::MomentumJwstDihedral(call), PhaseSnap::MomentumJwstDihedral(before)) => call.exceedances(before),
            (Live::GraspLiftGlass4tu(call), PhaseSnap::GraspLiftGlass4tu(before)) => call.exceedances(before),
            (Live::MomentumJwstSrp(call), PhaseSnap::MomentumJwstSrp(before)) => call.exceedances(before),
            (Live::Tire(call), PhaseSnap::Tire(before)) => call.exceedances(before),
            (Live::Accumulator(call), PhaseSnap::Accumulator(before)) => call.exceedances(before),
            (Live::Vde(call), PhaseSnap::Vde(before)) => call.exceedances(before),
            (Live::FingerPad(call), PhaseSnap::FingerPad(before)) => call.exceedances(before),
            (Live::GridGbTwin(call), PhaseSnap::GridGbTwin(before)) => call.exceedances(before),
            (Live::TokamakJetDte1Twin(call), PhaseSnap::TokamakJetDte1Twin(before)) => call.exceedances(before),
            (Live::Runaway18650LcoTwin(call), PhaseSnap::Runaway18650LcoTwin(before)) => call.exceedances(before),
            (Live::Cell2170Twin(call), PhaseSnap::Cell2170Twin(before)) => call.exceedances(before),
            (Live::ReactorSpert3CcorePlate(call), PhaseSnap::ReactorSpert3CcorePlate(before)) => call.exceedances(before),
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
            Live::Thermal(call) => call.step_into(frame),
            Live::Resonance(call) => call.step_into(frame),
            Live::Tesseract(call) => call.step_into(frame),
            Live::Hydraulic(call) => call.step_into(frame),
            Live::Cable(call) => call.step_into(frame),
            Live::Generator(call) => call.step_into(frame),
            Live::Capsule(call) => call.step_into(frame),
            Live::Satellite(call) => call.step_into(frame),
            Live::SolarSystem(call) => call.step_into(frame),
            Live::Cantilever(call) => call.step_into(frame),
            Live::Link(call) => call.step_into(frame),
            Live::Los(call) => call.step_into(frame),
            Live::Creep(call) => call.step_into(frame),
            Live::CapsuleInsight(call) => call.step_into(frame),
            Live::ReactorTriga(call) => call.step_into(frame),
            Live::Crater(call) => call.step_into(frame),
            Live::GridGbTwin(call) => call.step_into(frame),
            Live::TokamakJetDte1Twin(call) => call.step_into(frame),
            Live::Runaway18650LcoTwin(call) => call.step_into(frame),
            Live::ReactorMsrePump(call) => call.step_into(frame),
            Live::ReactorSpert3Twin(call) => call.step_into(frame),
            Live::Cell2170Twin(call) => call.step_into(frame),
            Live::ReactorSpert3CcorePlate(call) => call.step_into(frame),
            Live::GeneratorMmrtgSeebeck(call) => call.step_into(frame),
            Live::MomentumJwstDihedral(call) => call.step_into(frame),
            Live::GraspLiftGlass4tu(call) => call.step_into(frame),
            Live::MomentumJwstSrp(call) => call.step_into(frame),
            Live::Tire(call) => call.step_into(frame),
            Live::Accumulator(call) => call.step_into(frame),
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
            Live::ReactorMsrePump(call) => {
                crate::card_reactor_msre_pump::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "reactor_msre_pump", call.state.residual_conservation_error)
            }
            Live::ReactorSpert3Twin(call) => {
                crate::card_reactor_spert3_twin::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "reactor_spert3", call.state.residual_conservation_error)
            }
            Live::GridGbTwin(call) => {
                crate::card_grid_gb_twin::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "grid_gb", call.state.residual_conservation_error)
            }
            Live::Cell2170Twin(call) => {
                crate::card_cell_2170_twin::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "cell_2170_model3", call.state.residual_conservation_error)
            }
            Live::TokamakJetDte1Twin(call) => {
                crate::card_tokamak_jet_dte1_twin::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "tokamak_jet_dte1", call.state.residual_conservation_error)
            }
            Live::Runaway18650LcoTwin(call) => {
                crate::card_runaway_18650_lco_twin::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "runaway_18650_lco", call.state.residual_conservation_error)
            }
            Live::ReactorSpert3CcorePlate(call) => {
                crate::card_reactor_spert3_ccore_plate::published_is_finite(&call.state).map_err(StepFailure::flag)?;
                finish_audit(frame, "energy", "reactor_spert3_ccore", call.state.residual_conservation_error)
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
    /// step that first crosses the flown deployment lands, and the SPERT III
    /// C-core card, whose step that passes its plate maximum lands
    /// (`src/ends.rs`).
    /// A run whose last step lands there has reached that end (rule 13).
    /// Every other card's end is a step that is not taken, so `None`.
    pub(crate) fn landed_at_end(&self) -> Option<&'static str> {
        match self {
            Live::Capsule(call) => call.past_flight_deployment().then_some("PastFlightDeployment"),
            Live::CapsuleInsight(call) => call.past_flight_deployment().then_some("PastFlightDeployment"),
            // The SPERT III C-core card sets the flag on the step that passes
            // its plate maximum, and refuses any step from a state that
            // carries it. The E-core card (reactor_spert3 since the fold) has
            // no declared end: past its peak is a flag, and it steps on.
            Live::ReactorSpert3CcorePlate(call) => call.state.past_plate_maximum.then_some("PastPlateTemperatureMaximum"),
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
            Live::Thermal(call) => thermal_state(&call.state).map_err(StepFailure::flag),
            Live::Resonance(call) => resonance_state(&call.state).map_err(StepFailure::flag),
            Live::Tesseract(call) => tesseract_state(&call.state).map_err(StepFailure::flag),
            Live::Hydraulic(call) => hydraulic_state(&call.state).map_err(StepFailure::flag),
            Live::Cable(call) => cable_state(&call.state).map_err(StepFailure::flag),
            Live::Generator(call) => generator_state(&call.state).map_err(StepFailure::flag),
            Live::Capsule(call) => capsule_state(&call.state).map_err(StepFailure::flag),
            Live::Satellite(call) => satellite_state(&call.state).map_err(StepFailure::flag),
            Live::SolarSystem(call) => solar_system_state(&call.state).map_err(StepFailure::flag),
            Live::Cantilever(call) => cantilever_state(&call.state).map_err(StepFailure::flag),
            Live::Link(call) => link_state(&call.state).map_err(StepFailure::flag),
            Live::Los(call) => los_state(&call.state).map_err(StepFailure::flag),
            Live::Creep(call) => creep_state(&call.state).map_err(StepFailure::flag),
            Live::CapsuleInsight(call) => {
                capsule_insight_state(&call.state).map_err(StepFailure::flag)
            }
            Live::ReactorTriga(call) => reactor_triga_state(&call.state).map_err(StepFailure::flag),
            Live::Crater(call) => crater_state(&call.state).map_err(StepFailure::flag),
            Live::GridGbTwin(call) => crate::card_grid_gb_twin::state(&call.state).map_err(StepFailure::flag),
            Live::TokamakJetDte1Twin(call) => crate::card_tokamak_jet_dte1_twin::state(&call.state).map_err(StepFailure::flag),
            Live::Runaway18650LcoTwin(call) => crate::card_runaway_18650_lco_twin::state(&call.state).map_err(StepFailure::flag),
            Live::ReactorMsrePump(call) => crate::card_reactor_msre_pump::state(&call.state).map_err(StepFailure::flag),
            Live::ReactorSpert3Twin(call) => crate::card_reactor_spert3_twin::state(&call.state).map_err(StepFailure::flag),
            Live::Cell2170Twin(call) => crate::card_cell_2170_twin::state(&call.state).map_err(StepFailure::flag),
            Live::ReactorSpert3CcorePlate(call) => crate::card_reactor_spert3_ccore_plate::state(&call.state).map_err(StepFailure::flag),
            Live::GeneratorMmrtgSeebeck(call) => crate::card_generator_mmrtg_seebeck::state(&call.state).map_err(StepFailure::flag),
            Live::MomentumJwstDihedral(call) => crate::card_momentum_jwst_dihedral::state(&call.state).map_err(StepFailure::flag),
            Live::GraspLiftGlass4tu(call) => crate::card_grasp_lift_glass_4tu::state(&call.state).map_err(StepFailure::flag),
            Live::MomentumJwstSrp(call) => crate::card_momentum_jwst_srp::state(&call.state).map_err(StepFailure::flag),
            Live::Tire(call) => tire_state(&call.state).map_err(StepFailure::flag),
            Live::Accumulator(call) => accumulator_state(&call.state).map_err(StepFailure::flag),
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
