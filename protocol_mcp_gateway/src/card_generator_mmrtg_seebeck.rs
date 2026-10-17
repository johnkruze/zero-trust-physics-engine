//! The gateway's call of the generator_mmrtg_seebeck card, the companion of
//! generator_mmrtg_f1: it reads a request, holds it to the card's published
//! bounds, steps the card and publishes its state, as the gateway does for
//! generator_mmrtg_f1. Stamped from the kit. Its description is
//! `card_generator_mmrtg_seebeck_descriptor.rs`, a child of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::generator_mmrtg_seebeck;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    phase_not_valid, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "energy";
pub(crate) const CARD: &str = "generator_mmrtg_seebeck";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = generator_mmrtg_seebeck::FLAG_PAST_FITTING_SPAN | generator_mmrtg_seebeck::FLAG_PAST_RECORD;

pub(crate) struct GeneratorMmrtgSeebeckCall {
    card: generator_mmrtg_seebeck::GeneratorMmrtgSeebeck,
    pub(crate) state: generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState,
    load: generator_mmrtg_seebeck::GeneratorMmrtgSeebeckLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way, in plain words and
/// with its name in round brackets at the end: generator_mmrtg_f1's.
fn reject_sentence(reject: generator_mmrtg_seebeck::StepReject) -> String {
    match reject {
        generator_mmrtg_seebeck::StepReject::MissionTimeBeforeLanding => {
            unsealed("mission time is before landing on Mars (MissionTimeBeforeLanding).")
        }
        generator_mmrtg_seebeck::StepReject::DecayHeatBelowLedgerResolution => unsealed(
            "the decay heat the fuel still holds is under the ledger's resolution, about 2,070 years after landing (DecayHeatBelowLedgerResolution).",
        ),
        generator_mmrtg_seebeck::StepReject::SpontaneousGeneration => {
            unsealed("the generator created energy (SpontaneousGeneration).")
        }
        generator_mmrtg_seebeck::StepReject::FirstLawBreach => unsealed("the generator destroyed energy (FirstLawBreach)."),
        generator_mmrtg_seebeck::StepReject::ConverterAccountBreach => unsealed(
            "the fitted model's account of the fading part of the converter broke (ConverterAccountBreach).",
        ),
        other => refused(CARD, other),
    }
}

impl GeneratorMmrtgSeebeckCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let mission_time_s = required_f64(params, "mission_time_s")?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = generator_mmrtg_seebeck::GeneratorMmrtgSeebeck::curiosity();
        card_clock(dt_s, Some(card.combined_rate_per_s), card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        if !(mission_time_s >= 0.0) {
            return Err(unsealed("mission time is before landing on Mars (MissionTimeBeforeLanding)."));
        }
        // Thermal power and the fading fraction are the generator's at this
        // mission time. The caller does not set them.
        let state = generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState::try_mission_time(&card, mission_time_s)
            .ok_or_else(|| phase_not_valid(CARD))?;
        Ok(Self { card, state, load: generator_mmrtg_seebeck::GeneratorMmrtgSeebeckLoad {}, dt_s })
    }

    /// The card has no load. The object must be there and must be empty.
    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = generator_mmrtg_seebeck::GeneratorMmrtgSeebeckLoad {};
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        // Every step, the first included, is the card's own comparison with
        // the last day and its one rounding tolerance.
        generator_mmrtg_seebeck::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
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

pub(crate) fn state(state: &generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order: generator_mmrtg_f1's.
pub(crate) fn with<T>(
    state: &generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState,
    finish: impl FnOnce(&[(&str, StateField)]) -> T,
) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("thermal_power_w", StateField::F64(state.thermal_power_w)),
        ("converter_fading_fraction", StateField::F64(state.converter_fading_fraction)),
        ("electrical_power_w", StateField::F64(state.electrical_power_w)),
        ("conversion_efficiency", StateField::F64(state.conversion_efficiency)),
        ("fuel_heat_j", StateField::F64(state.fuel_heat_j)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("electrical_energy_j", StateField::F64(state.electrical_energy_j)),
        ("heat_rejected_j", StateField::F64(state.heat_rejected_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("converter_fading_yield_j", StateField::F64(state.converter_fading_yield_j)),
        ("converter_account_anchor_j", StateField::F64(state.converter_account_anchor_j)),
        ("fading_electrical_energy_j", StateField::F64(state.fading_electrical_energy_j)),
        ("converter_account_defect_j", StateField::F64(state.converter_account_defect_j)),
        ("converter_account_error", StateField::F64(state.converter_account_error)),
        ("range_margin_s", StateField::F64(state.range_margin_s)),
        ("past_fitting_span", StateField::Bool(state.past_fitting_span)),
    ])
}

/// The card at rest: the published fields of the generator at landing.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = generator_mmrtg_seebeck::GeneratorMmrtgSeebeck::curiosity();
    let state = generator_mmrtg_seebeck::GeneratorMmrtgSeebeckState::try_landing(&card).expect("landing");
    with(&state, finish)
}
