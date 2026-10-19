//! The gateway's call of the tokamak_jet_dte1_twin card: it reads a request, holds it to
//! the card's published bounds, steps the card and publishes its state.
//! Stamped from the kit. Its description is `card_tokamak_jet_dte1_twin_descriptor.rs`,
//! a child of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::tokamak_jet_dte1_twin;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    message_number, phase_not_valid, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "energy";
pub(crate) const CARD: &str = "tokamak_jet_dte1";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = tokamak_jet_dte1_twin::FLAG_NEAR_DENSITY_LIMIT | tokamak_jet_dte1_twin::FLAG_OUTSIDE_COMPARED;

pub(crate) struct TokamakJetDte1TwinCall {
    card: tokamak_jet_dte1_twin::TokamakJetDte1Twin,
    pub(crate) state: tokamak_jet_dte1_twin::TwinState,
    load: tokamak_jet_dte1_twin::TwinLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way or on its load, in
/// plain words and with its name in round brackets at the end.
fn reject_sentence(reject: tokamak_jet_dte1_twin::StepReject, card: &tokamak_jet_dte1_twin::TokamakJetDte1Twin, load: tokamak_jet_dte1_twin::TwinLoad, dt_s: f64) -> String {
    use tokamak_jet_dte1_twin::StepReject as R;
    match reject {
        R::DensityAboveGreenwald => unsealed(&format!(
            "the line-averaged density {} per cubic metre is above {} per cubic metre, the Greenwald density of a plasma current of {} A (DensityAboveGreenwald).",
            message_number(load.line_density_per_m3),
            message_number(card.greenwald_density_per_m3(load.plasma_current_a)),
            message_number(load.plasma_current_a),
        )),
        R::SafetyFactorBelowLimit => unsealed(&format!(
            "the edge safety factor of the plasma current and the toroidal field is {}, below {} (SafetyFactorBelowLimit).",
            message_number(card.safety_factor(load.plasma_current_a, load.toroidal_field_t)),
            tokamak_jet_dte1_twin::SAFETY_FACTOR_MIN,
        )),
        R::NoHeatingPower => unsealed("the beam, ion-cyclotron and ohmic powers are all zero: the confinement time of the scaling, P^-0.69, has no value (NoHeatingPower)."),
        R::ElectronTemperatureOutsideCard => unsealed("the central electron temperature is zero or less, or so low against the density that the Coulomb logarithm of a cell is zero or less, where the classical slowing-down rate has no value (ElectronTemperatureOutsideCard)."),
        R::EnergyOutsideCard => unsealed(&format!(
            "a fast store is negative, or the thermal energy is not above {} J, the electrons' energy at the load's electron temperature, so the ions would hold none (EnergyOutsideCard).",
            card.electron_energy_j(load).map_or_else(|| "the".to_string(), message_number),
        )),
        R::TemperatureAboveReactivityFit => unsealed("the central ion temperature is above 100 keV, the ceiling of the reactivity fit (TemperatureAboveReactivityFit)."),
        R::BetaAboveTroyon => unsealed("the normalised beta of the thermal and the fast-ion energy is above the Troyon limit of 2.8 (BetaAboveTroyon)."),
        R::DtAboveStateCeiling => unsealed(&format!(
            "the step of {} s is above {} s, an eighth of the shortest of the beam ions' and the tail's thermalisation times and the confinement time of this load (DtAboveStateCeiling).",
            message_number(dt_s),
            card.state_ceiling_dt_s(load).map_or_else(|| "the ceiling".to_string(), message_number),
        )),
        R::SpontaneousGeneration => unsealed("the plasma created energy (SpontaneousGeneration)."),
        R::FirstLawBreach => unsealed("the plasma destroyed energy (FirstLawBreach)."),
        other => refused(CARD, other),
    }
}

fn read_load(card: &tokamak_jet_dte1_twin::TokamakJetDte1Twin, params: &serde_json::Value) -> Result<tokamak_jet_dte1_twin::TwinLoad, String> {
    let load = tokamak_jet_dte1_twin::TwinLoad {
        deuterium_beam_power_w: required_f64(params, "deuterium_beam_power_w")?,
        tritium_beam_power_w: required_f64(params, "tritium_beam_power_w")?,
        deuterium_beam_energy_kev: required_f64(params, "deuterium_beam_energy_kev")?,
        tritium_beam_energy_kev: required_f64(params, "tritium_beam_energy_kev")?,
        icrh_power_w: required_f64(params, "icrh_power_w")?,
        ohmic_power_w: required_f64(params, "ohmic_power_w")?,
        line_density_per_m3: required_f64(params, "line_density_per_m3")?,
        plasma_current_a: required_f64(params, "plasma_current_a")?,
        toroidal_field_t: required_f64(params, "toroidal_field_t")?,
        tritium_fraction: required_f64(params, "tritium_fraction")?,
        electron_temperature_kev: required_f64(params, "electron_temperature_kev")?,
        effective_charge: required_f64(params, "effective_charge")?,
        hot_ion_regime: required_f64(params, "hot_ion_regime")?,
    };
    // Each value is inside its own published bound here. The card's own test
    // then holds the powers together, the density to the Greenwald density,
    // the current and the field to a safety factor of 2, the regime to 0 or 1
    // and the electron temperature to a Coulomb logarithm above zero.
    tokamak_jet_dte1_twin::load_in_card(card, load).map_err(|reject| reject_sentence(reject, card, load, f64::NAN))?;
    Ok(load)
}

impl TokamakJetDte1TwinCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let thermal = required_f64(params, "thermal_energy_j")?;
        let deuterium = required_f64(params, "deuterium_beam_ion_energy_j")?;
        let tritium = required_f64(params, "tritium_beam_ion_energy_j")?;
        let tail = required_f64(params, "tail_ion_energy_j")?;
        let elapsed_time_s = required_f64(params, "elapsed_time_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = tokamak_jet_dte1_twin::TokamakJetDte1Twin::dte1();
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        let load = read_load(&card, params)?;
        let state = tokamak_jet_dte1_twin::TwinState::try_continue(&card, elapsed_time_s, thermal, deuterium, tritium, tail)
            .ok_or_else(|| phase_not_valid(CARD))?;
        // The start is held by the card's own test under the first load: the
        // thermal energy above the electrons', the reactivity fit and the
        // Troyon limit; and the step to the ceiling of that load.
        tokamak_jet_dte1_twin::state_in_card(&card, &state, load).map_err(|reject| reject_sentence(reject, &card, load, dt_s))?;
        if !card.state_ceiling_dt_s(load).is_some_and(|ceiling| dt_s <= ceiling) {
            return Err(reject_sentence(tokamak_jet_dte1_twin::StepReject::DtAboveStateCeiling, &card, load, dt_s));
        }
        Ok(Self { card, state, load, dt_s })
    }

    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = read_load(&self.card, params)?;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        let (card, load, dt_s) = (self.card, self.load, self.dt_s);
        tokamak_jet_dte1_twin::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
            .map_err(|reject| StepFailure::from_reject(reject, reject_sentence(reject, &card, load, dt_s)))
    }

    /// The limits the step from `before` to the state now held is past
    /// (`crate::exceedance`).
    pub(crate) fn exceedances(&self, before: &tokamak_jet_dte1_twin::TwinState) -> genesis_kernel::exceedance::Exceedances {
        genesis_kernel::tokamak_jet_dte1_twin::exceedances(&self.card, before, &self.state, self.load, self.dt_s)
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

pub(crate) fn state(state: &tokamak_jet_dte1_twin::TwinState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &tokamak_jet_dte1_twin::TwinState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order.
pub(crate) fn with<T>(state: &tokamak_jet_dte1_twin::TwinState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let s = state;
    finish(&[
        ("t_s", StateField::F64(s.t_s)),
        ("thermal_energy_j", StateField::F64(s.thermal_energy_j)),
        ("deuterium_beam_ion_energy_j", StateField::F64(s.deuterium_beam_ion_energy_j)),
        ("tritium_beam_ion_energy_j", StateField::F64(s.tritium_beam_ion_energy_j)),
        ("tail_ion_energy_j", StateField::F64(s.tail_ion_energy_j)),
        ("fast_ion_energy_j", StateField::F64(s.fast_ion_energy_j)),
        ("fusion_energy_j", StateField::F64(s.fusion_energy_j)),
        ("fusion_power_w", StateField::F64(s.fusion_power_w)),
        ("fusion_power_thermal_w", StateField::F64(s.fusion_power_thermal_w)),
        ("fusion_power_deuterium_beam_w", StateField::F64(s.fusion_power_deuterium_beam_w)),
        ("fusion_power_tritium_beam_w", StateField::F64(s.fusion_power_tritium_beam_w)),
        ("electron_energy_j", StateField::F64(s.electron_energy_j)),
        ("ion_temperature_kev", StateField::F64(s.ion_temperature_kev)),
        ("loss_power_w", StateField::F64(s.loss_power_w)),
        ("confinement_time_s", StateField::F64(s.confinement_time_s)),
        ("beta_normalised", StateField::F64(s.beta_normalised)),
        ("greenwald_fraction", StateField::F64(s.greenwald_fraction)),
        ("safety_factor", StateField::F64(s.safety_factor)),
        ("energy_anchor_j", StateField::F64(s.energy_anchor_j)),
        ("energy_supplied_j", StateField::F64(s.energy_supplied_j)),
        ("energy_lost_j", StateField::F64(s.energy_lost_j)),
        ("energy_defect_j", StateField::F64(s.energy_defect_j)),
        ("residual_conservation_error", StateField::F64(s.residual_conservation_error)),
        ("near_density_limit", StateField::Bool(s.near_density_limit)),
        ("outside_compared", StateField::Bool(s.outside_compared)),
    ])
}

/// The card at rest: the published fields of a start of 1 MJ thermal and no fast ions.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = tokamak_jet_dte1_twin::TokamakJetDte1Twin::dte1();
    let state = tokamak_jet_dte1_twin::TwinState::try_start(&card, 1.0e6, 0.0, 0.0, 0.0).expect("the plasma starts");
    with(&state, finish)
}
