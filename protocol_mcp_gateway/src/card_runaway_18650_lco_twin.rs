//! The gateway's call of the runaway_18650_lco_twin card: it reads a request, holds it to
//! the card's published bounds, steps the card and publishes its state.
//! Stamped from the kit. Its description is `card_runaway_18650_lco_twin_descriptor.rs`,
//! a child of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::runaway_18650_lco_twin;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    message_number, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "energy";
pub(crate) const CARD: &str = "runaway_18650_lco";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = runaway_18650_lco_twin::FLAG_SELF_HEATING
    | runaway_18650_lco_twin::FLAG_RUNAWAY
    | runaway_18650_lco_twin::FLAG_SEPARATOR_MELTED
    | runaway_18650_lco_twin::FLAG_VENTED
    | runaway_18650_lco_twin::FLAG_OUTSIDE_COMPARED;

/// The start fields, in stage order after the temperature.
pub(crate) const AMOUNT_FIELDS: [&str; runaway_18650_lco_twin::STAGES] = [
    "sei_fraction",
    "anode_lithium_fraction",
    "cathode_unreacted_fraction",
    "electrolyte_fraction",
    "separator_unmelted_fraction",
    "vent_gas_held_fraction",
];

pub(crate) struct Runaway18650LcoTwinCall {
    card: runaway_18650_lco_twin::Runaway18650LcoTwin,
    pub(crate) state: runaway_18650_lco_twin::RunawayState,
    load: runaway_18650_lco_twin::RunawayLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way, in plain words and
/// with its name in round brackets at the end.
pub(crate) fn reject_sentence(reject: runaway_18650_lco_twin::StepReject) -> String {
    match reject {
        runaway_18650_lco_twin::StepReject::EveryReactantConsumed => unsealed(
            "the step would carry the cell to every reactant consumed: the heat the six stages still hold or take falls to 1e-6 of the cell's chemical heat or under, 0.0205 J, and the runaway has finished (EveryReactantConsumed).",
        ),
        runaway_18650_lco_twin::StepReject::CanMelting => unsealed(
            "the cell temperature, incoming or in a substep, is at or above 1811 K, the melting point of iron: the steel can melts (CanMelting).",
        ),
        runaway_18650_lco_twin::StepReject::SubstepsAboveBudget => {
            unsealed("the step would need more than 1,048,576 substeps to follow its fastest stage (SubstepsAboveBudget).")
        }
        runaway_18650_lco_twin::StepReject::SpontaneousGeneration => unsealed(
            "the heat stored in the cell rose above the heat released, plus the heater's, less the heat lost (SpontaneousGeneration).",
        ),
        runaway_18650_lco_twin::StepReject::FirstLawBreach => unsealed(
            "the heat stored in the cell fell below the heat released, plus the heater's, less the heat lost (FirstLawBreach).",
        ),
        other => refused(CARD, other),
    }
}

/// The load of a step or a segment. `run_trajectory` reads every segment
/// through here before the first step, so an adiabatic switch between 0 and
/// 1 in any segment refuses the whole run.
fn read_load(params: &serde_json::Value) -> Result<runaway_18650_lco_twin::RunawayLoad, String> {
    let adiabatic = required_f64(params, "adiabatic")?;
    if adiabatic.is_finite() && adiabatic != 0.0 && adiabatic != 1.0 {
        return Err(reject_sentence(runaway_18650_lco_twin::StepReject::AdiabaticNotZeroOrOne));
    }
    let oven_temperature_k = required_f64(params, "oven_temperature_k")?;
    // The oven temperature enters only where heat crosses the can.
    if adiabatic == 0.0 && oven_temperature_k.is_finite() && !(oven_temperature_k > 0.0) {
        return Err(unsealed(&format!(
            "'oven_temperature_k' {} K is at or below 0 K, the exclusive minimum of the runaway_18650_lco card where the cell loses heat to the oven, adiabatic 0 (OvenTemperatureNotPositive).",
            message_number(oven_temperature_k)
        )));
    }
    Ok(runaway_18650_lco_twin::RunawayLoad {
        oven_temperature_k,
        heater_power_w: required_f64(params, "heater_power_w")?,
        adiabatic,
    })
}

impl Runaway18650LcoTwinCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let temperature_k = required_f64(params, "temperature_k")?;
        let mut amounts = [0.0; runaway_18650_lco_twin::STAGES];
        for (amount, name) in amounts.iter_mut().zip(AMOUNT_FIELDS) {
            *amount = required_f64(params, name)?;
        }
        let load = read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = runaway_18650_lco_twin::Runaway18650LcoTwin::fitted();
        // The ceiling comes from the cell's convective time constant; the
        // substeps inside a step follow the stages, so no `π/ω` applies.
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        let state = runaway_18650_lco_twin::RunawayState::try_start(&card, temperature_k, amounts).map_err(reject_sentence)?;
        Ok(Self { card, state, load, dt_s })
    }

    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = read_load(params)?;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        runaway_18650_lco_twin::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| StepFailure::from_reject(reject, reject_sentence(reject)))
    }

    /// The limits the step from `before` to the state now held is past
    /// (`crate::exceedance`).
    pub(crate) fn exceedances(&self, before: &runaway_18650_lco_twin::RunawayState) -> genesis_kernel::exceedance::Exceedances {
        genesis_kernel::runaway_18650_lco_twin::exceedances(&self.card, before, &self.state, self.load, self.dt_s)
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

pub(crate) fn state(state: &runaway_18650_lco_twin::RunawayState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &runaway_18650_lco_twin::RunawayState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order.
pub(crate) fn with<T>(state: &runaway_18650_lco_twin::RunawayState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("temperature_k", StateField::F64(state.temperature_k)),
        ("sei_fraction", StateField::F64(state.sei_fraction)),
        ("anode_lithium_fraction", StateField::F64(state.anode_lithium_fraction)),
        ("cathode_unreacted_fraction", StateField::F64(state.cathode_unreacted_fraction)),
        ("electrolyte_fraction", StateField::F64(state.electrolyte_fraction)),
        ("separator_unmelted_fraction", StateField::F64(state.separator_unmelted_fraction)),
        ("vent_gas_held_fraction", StateField::F64(state.vent_gas_held_fraction)),
        ("sei_thickness", StateField::F64(state.sei_thickness)),
        ("start_temperature_k", StateField::F64(state.start_temperature_k)),
        ("heat_sei_j", StateField::F64(state.heat_sei_j)),
        ("heat_anode_j", StateField::F64(state.heat_anode_j)),
        ("heat_cathode_j", StateField::F64(state.heat_cathode_j)),
        ("heat_electrolyte_j", StateField::F64(state.heat_electrolyte_j)),
        ("heat_separator_j", StateField::F64(state.heat_separator_j)),
        ("heat_vent_j", StateField::F64(state.heat_vent_j)),
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
        ("separator_melted", StateField::Bool(state.separator_melted)),
        ("vented", StateField::Bool(state.vented)),
        ("outside_compared", StateField::Bool(state.outside_compared)),
    ])
}

/// The card at rest: the published fields of a fresh cell at 298.15 K.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = runaway_18650_lco_twin::Runaway18650LcoTwin::fitted();
    let state = runaway_18650_lco_twin::RunawayState::try_fresh(&card, 298.15).expect("a fresh cell rests");
    with(&state, finish)
}
