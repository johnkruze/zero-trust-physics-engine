//! The gateway's call of the reactor_spert3_twin card: it reads a request, holds it to
//! the card's published bounds, steps the card and publishes its state.
//! Stamped from the kit. Its description is `card_reactor_spert3_twin_descriptor.rs`,
//! a child of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::reactor_spert3_twin;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    message_number, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "energy";
pub(crate) const CARD: &str = "reactor_spert3_twin";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = reactor_spert3_twin::FLAG_ROD_MOVING
    | reactor_spert3_twin::FLAG_PAST_PEAK
    | reactor_spert3_twin::FLAG_HOT_SPOT_BOILING
    | reactor_spert3_twin::FLAG_PROMPT_SUPERCRITICAL
    | reactor_spert3_twin::FLAG_OUTSIDE_COMPARED;

/// The cold-startup start of the worked examples and of the card at rest:
/// 76 degF, 0.0853 MPa, no flow, 11.7719 W.
pub(crate) const REST_INLET_K: f64 = (76.0 + 459.67) / 1.8;
pub(crate) const REST_PRESSURE_MPA: f64 = 0.0853;
pub(crate) const REST_POWER_MW: f64 = 1.17719e-5;

pub(crate) struct ReactorSpert3TwinCall {
    card: reactor_spert3_twin::ReactorSpert3Twin,
    pub(crate) state: reactor_spert3_twin::ReactorState,
    load: reactor_spert3_twin::ReactorLoad,
    run_load: reactor_spert3_twin::ReactorLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way, in plain words and
/// with its name in round brackets at the end.
fn reject_sentence(reject: reactor_spert3_twin::StepReject) -> String {
    use reactor_spert3_twin::StepReject as R;
    match reject {
        R::ReactivityNegative => unsealed("the reactivity is below zero: the transient rod is ejected, never driven in (ReactivityNegative)."),
        R::ReactivityChangedDuringRun => unsealed("the reactivity is not the one the run's first step sent the rod through; a run is one ejection (ReactivityChangedDuringRun)."),
        R::StateOutsideCard => unsealed("the incoming state is not one this card writes (StateOutsideCard)."),
        R::DtAboveStateCeiling => unsealed(&format!(
            "the step is longer than {} of the prompt time constant l* / |1 - rho| ({} while the prompt pole grows) at the state it starts from or lands on (DtAboveStateCeiling).",
            reactor_spert3_twin::DECAY_PHASE_STEP,
            reactor_spert3_twin::GROWTH_PHASE_STEP
        )),
        R::FuelMelting => unsealed(&format!(
            "a ring of the hot rod reached {} K, the melting point of UO2 (FuelMelting).",
            reactor_spert3_twin::UO2_MELTING_POINT_K
        )),
        R::WaterAtSaturation => unsealed("the water in the core reached its saturation temperature: bulk boiling, where the card's single-phase water no longer holds (WaterAtSaturation)."),
        R::SpontaneousGeneration => unsealed("the step created neutrons or heat from nothing (SpontaneousGeneration)."),
        R::FirstLawBreach => unsealed("the step destroyed neutrons or heat (FirstLawBreach)."),
        other => refused(CARD, other),
    }
}

fn read_load(params: &serde_json::Value) -> Result<reactor_spert3_twin::ReactorLoad, String> {
    Ok(reactor_spert3_twin::ReactorLoad { reactivity_inserted_dollar: required_f64(params, "reactivity_inserted_dollar")? })
}

fn start_outside(inlet_k: f64, pressure_mpa: f64, flow: f64, power: f64) -> String {
    format!(
        "Physical Violation: the start of inlet {} K, pressure {} MPa, flow {} m/s and power {} MW is outside the reactor_spert3_twin card: the inlet must be above 273.15 K and below the saturation temperature at the pressure, {} K, and the steady state at that power must hold liquid water and solid fuel (StateOutsideCard).",
        message_number(inlet_k),
        message_number(pressure_mpa),
        message_number(flow),
        message_number(power),
        message_number(reactor_spert3_twin::saturation_temperature_k(pressure_mpa)),
    )
}

impl ReactorSpert3TwinCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let inlet_k = required_f64(params, "inlet_temperature_k")?;
        let pressure_mpa = required_f64(params, "system_pressure_mpa")?;
        let flow = required_f64(params, "coolant_flow_m_per_s")?;
        let power = required_f64(params, "initial_power_mw")?;
        let load = read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = reactor_spert3_twin::ReactorSpert3Twin::e_core();
        // The reactor has no frequency. Its ceiling is a fifth of the reduced
        // generation time at 70 degF.
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        let state = reactor_spert3_twin::ReactorState::try_start(&card, inlet_k, pressure_mpa, flow, power)
            .ok_or_else(|| start_outside(inlet_k, pressure_mpa, flow, power))?;
        Ok(Self { card, state, load, run_load: load, dt_s })
    }

    /// The load of a segment. Its reactivity is held to the one the run was
    /// opened with: a run is one ejection, and the card's step refuses
    /// another reactivity once the first step has landed.
    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        let load = read_load(params)?;
        if load.reactivity_inserted_dollar.to_bits() != self.run_load.reactivity_inserted_dollar.to_bits() {
            return Err(format!(
                "Physical Violation: 'reactivity_inserted_dollar' {} dollar is not the {} dollar of the first segment: a run of the reactor_spert3_twin card is one ejection of the transient rod (ReactivityChangedDuringRun).",
                message_number(load.reactivity_inserted_dollar),
                message_number(self.run_load.reactivity_inserted_dollar)
            ));
        }
        self.load = load;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        reactor_spert3_twin::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| StepFailure::from_reject(reject, reject_sentence(reject)))
    }

    pub(crate) fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        self.step_into(&mut frame).map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            residual_conservation_error: self.state.residual_conservation_error,
            state: state(&self.state)?,
        })
    }
}

pub(crate) fn state(state: &reactor_spert3_twin::ReactorState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &reactor_spert3_twin::ReactorState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order.
pub(crate) fn with<T>(state: &reactor_spert3_twin::ReactorState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let s = state;
    finish(&[
        ("t_s", StateField::F64(s.t_s)),
        ("power_mw", StateField::F64(s.power_mw)),
        ("energy_released_mws", StateField::F64(s.energy_released_mws)),
        ("net_reactivity_dollar", StateField::F64(s.net_reactivity_dollar)),
        ("inserted_reactivity_dollar", StateField::F64(s.inserted_reactivity_dollar)),
        ("doppler_dollar", StateField::F64(s.doppler_dollar)),
        ("moderator_dollar", StateField::F64(s.moderator_dollar)),
        ("rod_position_in", StateField::F64(s.rod_position_in)),
        ("peak_power_mw", StateField::F64(s.peak_power_mw)),
        ("time_of_peak_s", StateField::F64(s.time_of_peak_s)),
        ("energy_to_peak_mws", StateField::F64(s.energy_to_peak_mws)),
        ("hot_clad_max_k", StateField::F64(s.hot_clad_max_k)),
        ("time_of_hot_clad_max_s", StateField::F64(s.time_of_hot_clad_max_s)),
        ("fuel_average_k", StateField::F64(s.fuel_average_k)),
        ("hot_fuel_centre_k", StateField::F64(s.hot_fuel_centre_k)),
        ("clad_k", StateField::F64(s.clad_k)),
        ("hot_clad_k", StateField::F64(s.hot_clad_k)),
        ("hot_clad_heat_flux_mw_per_m2", StateField::F64(s.hot_clad_heat_flux_mw_per_m2)),
        ("water_k", StateField::F64(s.water_k)),
        ("saturation_k", StateField::F64(s.saturation_k)),
        ("fuel_melt_margin_k", StateField::F64(s.fuel_melt_margin_k)),
        ("fuel_ring_0_k", StateField::F64(s.fuel_k[0])),
        ("fuel_ring_1_k", StateField::F64(s.fuel_k[1])),
        ("fuel_ring_2_k", StateField::F64(s.fuel_k[2])),
        ("fuel_ring_3_k", StateField::F64(s.fuel_k[3])),
        ("fuel_ring_4_k", StateField::F64(s.fuel_k[4])),
        ("fuel_ring_5_k", StateField::F64(s.fuel_k[5])),
        ("hot_fuel_ring_0_k", StateField::F64(s.hot_fuel_k[0])),
        ("hot_fuel_ring_1_k", StateField::F64(s.hot_fuel_k[1])),
        ("hot_fuel_ring_2_k", StateField::F64(s.hot_fuel_k[2])),
        ("hot_fuel_ring_3_k", StateField::F64(s.hot_fuel_k[3])),
        ("hot_fuel_ring_4_k", StateField::F64(s.hot_fuel_k[4])),
        ("hot_fuel_ring_5_k", StateField::F64(s.hot_fuel_k[5])),
        ("precursor_1_mw", StateField::F64(s.precursor_mw[0])),
        ("precursor_2_mw", StateField::F64(s.precursor_mw[1])),
        ("precursor_3_mw", StateField::F64(s.precursor_mw[2])),
        ("precursor_4_mw", StateField::F64(s.precursor_mw[3])),
        ("precursor_5_mw", StateField::F64(s.precursor_mw[4])),
        ("precursor_6_mw", StateField::F64(s.precursor_mw[5])),
        ("heat_carried_out_mws", StateField::F64(s.heat_carried_out_mws)),
        ("hot_heat_to_water_mws", StateField::F64(s.hot_heat_to_water_mws)),
        ("population_anchor_mw", StateField::F64(s.population_anchor_mw)),
        ("heat_anchor_mws", StateField::F64(s.heat_anchor_mws)),
        ("hot_heat_anchor_mws", StateField::F64(s.hot_heat_anchor_mws)),
        ("reactivity_power_integral", StateField::F64(s.reactivity_power_integral)),
        ("population_defect_mw", StateField::F64(s.population_defect_mw)),
        ("energy_defect_mws", StateField::F64(s.energy_defect_mws)),
        ("hot_energy_defect_mws", StateField::F64(s.hot_energy_defect_mws)),
        ("residual_conservation_error", StateField::F64(s.residual_conservation_error)),
        ("inlet_temperature_k", StateField::F64(s.inlet_temperature_k)),
        ("system_pressure_mpa", StateField::F64(s.system_pressure_mpa)),
        ("coolant_flow_m_per_s", StateField::F64(s.coolant_flow_m_per_s)),
        ("initial_power_mw", StateField::F64(s.initial_power_mw)),
        ("start_fuel_average_k", StateField::F64(s.start_fuel_average_k)),
        ("start_water_k", StateField::F64(s.start_water_k)),
        ("reactivity_inserted_dollar", StateField::F64(s.reactivity_inserted_dollar)),
        ("rod_moving", StateField::Bool(s.rod_moving)),
        ("past_peak", StateField::Bool(s.past_peak)),
        ("hot_spot_boiling", StateField::Bool(s.hot_spot_boiling)),
        ("prompt_supercritical", StateField::Bool(s.prompt_supercritical)),
        ("outside_compared", StateField::Bool(s.outside_compared)),
    ])
}

/// The card at rest: the published fields of the cold-startup start.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = reactor_spert3_twin::ReactorSpert3Twin::e_core();
    let state = reactor_spert3_twin::ReactorState::try_start(&card, REST_INLET_K, REST_PRESSURE_MPA, 0.0, REST_POWER_MW).expect("the reactor rests");
    with(&state, finish)
}
