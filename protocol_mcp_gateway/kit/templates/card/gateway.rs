//! The gateway's call of the {{name}} card: it reads a request, holds it to
//! the card's published bounds, steps the card and publishes its state.
//! Stamped from the kit. Its description is `card_{{name}}_descriptor.rs`,
//! a child of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::{{name}};

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    phase_not_valid, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "{{domain}}";
pub(crate) const CARD: &str = "{{name}}";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = 0;

pub(crate) struct {{Name}}Call {
    card: {{name}}::{{Name}},
    pub(crate) state: {{name}}::{{Name}}State,
    load: {{name}}::{{Name}}Load,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way, in plain words and
/// with its name in round brackets at the end.
fn reject_sentence(reject: {{name}}::StepReject) -> String {
    match reject {
        {{name}}::StepReject::BelowAbsoluteZero => unsealed("the body or the bath is at or below absolute zero (BelowAbsoluteZero)."),
        {{name}}::StepReject::AboveMeltingPoint => unsealed(&format!(
            "the body or the bath is at or above {} K, the melting point of the body (AboveMeltingPoint).",
            {{name}}::MELTING_POINT_K
        )),
        {{name}}::StepReject::SpontaneousGeneration => unsealed("the body created heat (SpontaneousGeneration)."),
        {{name}}::StepReject::FirstLawBreach => unsealed("the body destroyed heat (FirstLawBreach)."),
        other => refused(CARD, other),
    }
}

fn read_load(params: &serde_json::Value) -> Result<{{name}}::{{Name}}Load, String> {
    Ok({{name}}::{{Name}}Load {
        heat_power_w: required_f64(params, "heat_power_w")?,
        ambient_k: required_f64(params, "ambient_k")?,
    })
}

impl {{Name}}Call {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let temperature_k = required_f64(params, "temperature_k")?;
        let load = read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = {{name}}::{{Name}}::specimen();
        card_clock(dt_s, Some(card.pole_rad_s()), card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        let state = {{name}}::{{Name}}State::try_start(&card, temperature_k).ok_or_else(|| phase_not_valid(CARD))?;
        Ok(Self { card, state, load, dt_s })
    }

    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = read_load(params)?;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        {{name}}::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
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

pub(crate) fn state(state: &{{name}}::{{Name}}State) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &{{name}}::{{Name}}State) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order.
pub(crate) fn with<T>(state: &{{name}}::{{Name}}State, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("temperature_k", StateField::F64(state.temperature_k)),
        ("heat_power_w", StateField::F64(state.heat_power_w)),
        ("ambient_k", StateField::F64(state.ambient_k)),
        ("internal_energy_j", StateField::F64(state.internal_energy_j)),
        ("energy_anchor_j", StateField::F64(state.energy_anchor_j)),
        ("boundary_work_j", StateField::F64(state.boundary_work_j)),
        ("energy_defect_j", StateField::F64(state.energy_defect_j)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("heat_to_ambient_w", StateField::F64(state.heat_to_ambient_w)),
        ("melt_margin_k", StateField::F64(state.melt_margin_k)),
    ])
}

/// The card at rest: the published fields of a fresh start at the reference.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = {{name}}::{{Name}}::specimen();
    let state = {{name}}::{{Name}}State::try_start(&card, {{name}}::REFERENCE_TEMPERATURE_K).expect("the body rests");
    with(&state, finish)
}
