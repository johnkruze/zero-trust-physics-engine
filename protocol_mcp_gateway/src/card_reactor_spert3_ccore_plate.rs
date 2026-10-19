//! The gateway's call of the reactor_spert3_ccore_plate card: it reads a request, holds it to
//! the card's published bounds, steps the card and publishes its state, as the gateway does for
//! reactor_spert3_ccore_twin, whose successor it is. Its description is
//! `card_reactor_spert3_ccore_plate_descriptor.rs`, a child of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::reactor_spert3_ccore_plate;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    message_number, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "energy";
pub(crate) const CARD: &str = "reactor_spert3_ccore";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = reactor_spert3_ccore_plate::FLAG_PROMPT_SUPERCRITICAL
    | reactor_spert3_ccore_plate::FLAG_PAST_POWER_PEAK
    | reactor_spert3_ccore_plate::FLAG_HOT_SPOT_BOILING
    | reactor_spert3_ccore_plate::FLAG_OUTSIDE_COMPARED_RANGE;

pub(crate) struct ReactorSpert3CcorePlateCall {
    card: reactor_spert3_ccore_plate::ReactorSpert3CcorePlate,
    pub(crate) state: reactor_spert3_ccore_plate::ReactorState,
    load: reactor_spert3_ccore_plate::ReactorLoad,
    /// The load the run was opened with. Its reactivity is the one reactivity of the transient:
    /// a segment that carries another is refused. A segment may carry another flow.
    run_load: reactor_spert3_ccore_plate::ReactorLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way, in plain words and
/// with its name in round brackets at the end.
fn reject_sentence(reject: reactor_spert3_ccore_plate::StepReject) -> String {
    match reject {
        reactor_spert3_ccore_plate::StepReject::PastPlateTemperatureMaximum => unsealed(
            "the reactor is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card (PastPlateTemperatureMaximum).",
        ),
        reactor_spert3_ccore_plate::StepReject::VoidFractionAboveOne => unsealed(
            "the vapour in a node's channels would fill them: a void fraction of one or more, with no water left between the plates (VoidFractionAboveOne).",
        ),
        reactor_spert3_ccore_plate::StepReject::SpontaneousGeneration => {
            unsealed("the reactor created neutrons or energy beyond what its ledger supplies (SpontaneousGeneration).")
        }
        reactor_spert3_ccore_plate::StepReject::FirstLawBreach => {
            unsealed("the reactor destroyed neutrons or energy its ledger supplies (FirstLawBreach).")
        }
        other => refused(CARD, other),
    }
}

/// The sentence for a step longer than the state ceiling, with the longest step the state the
/// step started from allows.
fn state_ceiling_sentence(dt_s: f64, longest_s: Option<f64>) -> String {
    let longest = match longest_s {
        Some(longest) if longest >= dt_s => {
            format!("the state the step started from allows {} s, and its trial end less", message_number(longest))
        }
        Some(longest) => format!("the state the step started from allows {} s", message_number(longest)),
        None => "the state the step started from allows no step".to_string(),
    };
    unsealed(&format!(
        "the step of {} s is longer than the state lets it be: {longest}. The step ceiling follows the state: the growing prompt pole held to {}, the decaying prompt pole to {} and the fastest node's conduction, heat transfer, boiling, condensation, plate-feedback lag and sweep to {} of a step (DtAboveStateCeiling).",
        message_number(dt_s),
        message_number(reactor_spert3_ccore_plate::GROWTH_PHASE_STEP),
        message_number(reactor_spert3_ccore_plate::DECAY_PHASE_STEP),
        message_number(reactor_spert3_ccore_plate::THERMAL_PHASE_STEP),
    ))
}

/// The sentence for a start whose pressure is outside the steam tables, or that is not liquid water.
fn start_sentence(initial_temperature_k: f64, system_pressure_psig: f64) -> String {
    let card = reactor_spert3_ccore_plate::ReactorSpert3CcorePlate::c_core_plate();
    if !card.pressure_in_steam_tables(system_pressure_psig) {
        unsealed(&format!(
            "'system_pressure_psig' {} psig is outside the steam tables on the {CARD} card: from the triple point of water, 611.657 Pa absolute, to below the critical pressure, 22.064 MPa, about -14.6072 to 3185.4 psig (PressureOutsideSteamTables).",
            message_number(system_pressure_psig),
        ))
    } else {
        let boiling = card.saturation_temperature_k(system_pressure_psig);
        unsealed(&format!(
            "'initial_temperature_k' {} K is not liquid water on the {CARD} card: a start is at or above {} K and below {} K, the saturation temperature of water at {} psig (IAPWS-IF97 region 4) (InitialTemperatureNotLiquidWater).",
            message_number(initial_temperature_k),
            message_number(reactor_spert3_ccore_plate::FREEZING_K),
            message_number(boiling),
            message_number(system_pressure_psig),
        ))
    }
}

fn read_load(params: &serde_json::Value) -> Result<reactor_spert3_ccore_plate::ReactorLoad, String> {
    let reactivity_inserted_dollar = required_f64(params, "reactivity_inserted_dollar")?;
    let coolant_flow_m_per_s = required_f64(params, "coolant_flow_m_per_s")?;
    if coolant_flow_m_per_s < 0.0 {
        return Err(unsealed(&format!(
            "'coolant_flow_m_per_s' {} m/s is below zero on the {CARD} card (NegativeFlow).",
            message_number(coolant_flow_m_per_s),
        )));
    }
    if !(reactivity_inserted_dollar > 0.0) {
        return Err(unsealed(&format!(
            "'reactivity_inserted_dollar' {} dollar is zero or less on the {CARD} card: no step in reactivity is inserted, and there is no burst (ReactivityNotPositive).",
            message_number(reactivity_inserted_dollar),
        )));
    }
    Ok(reactor_spert3_ccore_plate::ReactorLoad { reactivity_inserted_dollar, coolant_flow_m_per_s })
}

impl ReactorSpert3CcorePlateCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let initial_temperature_k = required_f64(params, "initial_temperature_k")?;
        let system_pressure_psig = required_f64(params, "system_pressure_psig")?;
        required_f64(params, "reactivity_inserted_dollar")?;
        required_f64(params, "coolant_flow_m_per_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = reactor_spert3_ccore_plate::ReactorSpert3CcorePlate::c_core_plate();
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        let state = reactor_spert3_ccore_plate::ReactorState::try_start(&card, initial_temperature_k, system_pressure_psig)
            .ok_or_else(|| start_sentence(initial_temperature_k, system_pressure_psig))?;
        let load = read_load(params)?;
        Ok(Self { card, state, load, run_load: load, dt_s })
    }

    /// The load of a segment, held to its limits and to the reactivity the run was opened with.
    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        let load = read_load(params)?;
        if load.reactivity_inserted_dollar != self.run_load.reactivity_inserted_dollar {
            return Err(unsealed(&format!(
                "'reactivity_inserted_dollar' {} dollar is not the {} dollar of the first segment: a run of the {CARD} card is one step of reactivity (ReactivityChangedDuringRun).",
                message_number(load.reactivity_inserted_dollar),
                message_number(self.run_load.reactivity_inserted_dollar),
            )));
        }
        self.load = load;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        let before = self.state;
        reactor_spert3_ccore_plate::step(&self.card, &mut self.state, frame, self.load, self.dt_s).map_err(|reject| {
            let detail = if reject == reactor_spert3_ccore_plate::StepReject::DtAboveStateCeiling {
                state_ceiling_sentence(self.dt_s, reactor_spert3_ccore_plate::state_dt_ceiling_s(&self.card, &before, self.load))
            } else {
                reject_sentence(reject)
            };
            StepFailure::from_reject(reject, detail)
        })
    }

    /// The limits the step from `before` to the state now held is past
    /// (`crate::exceedance`).
    pub(crate) fn exceedances(&self, before: &reactor_spert3_ccore_plate::ReactorState) -> genesis_kernel::exceedance::Exceedances {
        genesis_kernel::reactor_spert3_ccore_plate::exceedances(&self.card, before, &self.state, self.load, self.dt_s)
    }

    pub(crate) fn run(mut self, proof: [u8; 16]) -> Result<IntegratedFrame, String> {
        let mut frame = blank_frame(proof);
        let before = self.state;
        self.step_into(&mut frame).map_err(|failure| failure.detail)?;
        Ok(IntegratedFrame {
            frame,
            exceedances: self.exceedances(&before),
            residual_conservation_error: self.state.residual_conservation_error,
            state: state(&self.state)?,
        })
    }
}

pub(crate) fn state(state: &reactor_spert3_ccore_plate::ReactorState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &reactor_spert3_ccore_plate::ReactorState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order.
pub(crate) fn with<T>(state: &reactor_spert3_ccore_plate::ReactorState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("power_mw", StateField::F64(state.power_mw)),
        ("energy_mws", StateField::F64(state.energy_mws)),
        ("power_rate_mw_per_s", StateField::F64(state.power_rate_mw_per_s)),
        ("hot_spot_plate_temperature_k", StateField::F64(state.hot_spot_plate_temperature_k)),
        ("hot_spot_plate_rise_k", StateField::F64(state.hot_spot_plate_rise_k)),
        ("hot_spot_plate_rate_k_per_s", StateField::F64(state.hot_spot_plate_rate_k_per_s)),
        ("hot_spot_film_rise_k", StateField::F64(state.hot_spot_film_rise_k)),
        ("hot_spot_water_rise_k", StateField::F64(state.hot_spot_water_rise_k)),
        ("hot_spot_void_latent_mws", StateField::F64(state.hot_spot_void_latent_mws)),
        ("saturation_temperature_k", StateField::F64(state.saturation_temperature_k)),
        ("reactivity_net_dollar", StateField::F64(state.reactivity_net_dollar)),
        ("moderator_feedback_dollar", StateField::F64(state.moderator_feedback_dollar)),
        ("plate_feedback_dollar", StateField::F64(state.plate_feedback_dollar)),
        ("void_feedback_dollar", StateField::F64(state.void_feedback_dollar)),
        ("void_fraction", StateField::F64(state.void_fraction)),
        ("plate_rise_k", StateField::F64(state.plate_rise_k)),
        ("film_rise_k", StateField::F64(state.film_rise_k)),
        ("water_rise_k", StateField::F64(state.water_rise_k)),
        ("void_latent_mws", StateField::F64(state.void_latent_mws)),
        ("feedback_plate_rise_k", StateField::F64(state.feedback_plate_rise_k)),
        ("feedback_film_rise_k", StateField::F64(state.feedback_film_rise_k)),
        ("feedback_water_rise_k", StateField::F64(state.feedback_water_rise_k)),
        ("feedback_void_latent_mws", StateField::F64(state.feedback_void_latent_mws)),
        ("meat_rise_k", StateField::F64(state.meat_rise_k)),
        ("feedback_meat_rise_k", StateField::F64(state.feedback_meat_rise_k)),
        ("hot_spot_meat_rise_k", StateField::F64(state.hot_spot_meat_rise_k)),
        ("plate_feedback_lagged_dollar", StateField::F64(state.plate_feedback_lagged_dollar)),
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
        ("feedback_swept_energy_mws", StateField::F64(state.feedback_swept_energy_mws)),
        ("hot_spot_swept_energy_mws", StateField::F64(state.hot_spot_swept_energy_mws)),
        ("population_defect_mw", StateField::F64(state.population_defect_mw)),
        ("energy_defect_mws", StateField::F64(state.energy_defect_mws)),
        ("feedback_energy_defect_mws", StateField::F64(state.feedback_energy_defect_mws)),
        ("hot_spot_energy_defect_mws", StateField::F64(state.hot_spot_energy_defect_mws)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("peak_power_mw", StateField::F64(state.peak_power_mw)),
        ("peak_energy_mws", StateField::F64(state.peak_energy_mws)),
        ("peak_time_s", StateField::F64(state.peak_time_s)),
        ("peak_plate_rise_k", StateField::F64(state.peak_plate_rise_k)),
        ("plate_rise_max_k", StateField::F64(state.plate_rise_max_k)),
        ("plate_rise_max_time_s", StateField::F64(state.plate_rise_max_time_s)),
        ("past_power_peak", StateField::Bool(state.past_power_peak)),
        ("past_plate_maximum", StateField::Bool(state.past_plate_maximum)),
        ("prompt_supercritical", StateField::Bool(state.prompt_supercritical)),
        ("hot_spot_boiling", StateField::Bool(state.hot_spot_boiling)),
        ("outside_compared_range", StateField::Bool(state.outside_compared_range)),
    ])
}

/// The card at rest: the published fields of a start at 25 degC and 0 psig.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = reactor_spert3_ccore_plate::ReactorSpert3CcorePlate::c_core_plate();
    let state = reactor_spert3_ccore_plate::ReactorState::try_start(&card, 298.15, 0.0).expect("the reactor rests");
    with(&state, finish)
}
