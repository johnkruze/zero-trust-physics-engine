//! The gateway's call of the reactor_msre_pump card: it reads a request, holds it to
//! the card's published bounds, steps the card and publishes its state.
//! Stamped from the kit. Its description is `card_reactor_msre_pump_descriptor.rs`,
//! a child of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::reactor_msre_pump;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    phase_not_valid, refused, reject_unknown_load, required_f64, state_object, unsealed,
};

pub(crate) const DOMAIN: &str = "energy";
pub(crate) const CARD: &str = "reactor_msre_pump";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 =
    reactor_msre_pump::FLAG_FLOW_CHANGING | reactor_msre_pump::FLAG_FUEL_CIRCULATING | reactor_msre_pump::FLAG_OUTSIDE_COMPARED_RUNS;

pub(crate) struct ReactorMsrePumpCall {
    card: reactor_msre_pump::ReactorMsrePump,
    pub(crate) state: reactor_msre_pump::ReactorMsrePumpState,
    load: reactor_msre_pump::ReactorMsrePumpLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way, in plain words and
/// with its name in round brackets at the end.
fn reject_sentence(reject: reactor_msre_pump::StepReject) -> String {
    match reject {
        reactor_msre_pump::StepReject::FlowOutsideCard => {
            unsealed("the pump speed or the fuel flow is below zero or above the pump's full speed (FlowOutsideCard).")
        }
        reactor_msre_pump::StepReject::SpontaneousGeneration => unsealed("the step created precursors beyond what the ledger supplies (SpontaneousGeneration)."),
        reactor_msre_pump::StepReject::FirstLawBreach => unsealed("the step destroyed precursors the ledger supplies (FirstLawBreach)."),
        other => refused(CARD, other),
    }
}

fn read_load(params: &serde_json::Value) -> Result<reactor_msre_pump::ReactorMsrePumpLoad, String> {
    Ok(reactor_msre_pump::ReactorMsrePumpLoad { pump_speed_fraction: required_f64(params, "pump_speed_fraction")? })
}

impl ReactorMsrePumpCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let pump_speed_at_start = required_f64(params, "pump_speed_at_start")?;
        let load = read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = reactor_msre_pump::ReactorMsrePump::specimen();
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        let state = reactor_msre_pump::ReactorMsrePumpState::try_start(&card, pump_speed_at_start).ok_or_else(|| phase_not_valid(CARD))?;
        Ok(Self { card, state, load, dt_s })
    }

    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = read_load(params)?;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        reactor_msre_pump::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
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

pub(crate) fn state(state: &reactor_msre_pump::ReactorMsrePumpState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &reactor_msre_pump::ReactorMsrePumpState) -> Result<(), String> {
    with(state, fields_are_finite)
}

const VESSEL: [&str; 6] = [
    "precursors_in_vessel_1_ws",
    "precursors_in_vessel_2_ws",
    "precursors_in_vessel_3_ws",
    "precursors_in_vessel_4_ws",
    "precursors_in_vessel_5_ws",
    "precursors_in_vessel_6_ws",
];
const OUTSIDE: [&str; 6] = [
    "precursors_outside_1_ws",
    "precursors_outside_2_ws",
    "precursors_outside_3_ws",
    "precursors_outside_4_ws",
    "precursors_outside_5_ws",
    "precursors_outside_6_ws",
];

/// The published fields of the state, in order: the clock, the pump and the
/// fuel flow, the answer, the ledger's residual, each group's precursors in
/// the vessel and outside it, and the flags' booleans. The 252 parcels' own
/// contents are the card's and are not published.
pub(crate) fn with<T>(state: &reactor_msre_pump::ReactorMsrePumpState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = reactor_msre_pump::ReactorMsrePump::specimen();
    let (vessel, outside) = reactor_msre_pump::inventories(&card, state);
    let mut fields = vec![
        ("t_s", StateField::F64(state.t_s)),
        ("pump_speed_at_start", StateField::F64(state.pump_speed_at_start)),
        ("pump_speed_fraction", StateField::F64(state.pump_speed_fraction)),
        ("fuel_flow_fraction", StateField::F64(state.fuel_flow_fraction)),
        ("fuel_flow_rate_per_s", StateField::F64(state.fuel_flow_rate_per_s)),
        ("phase_s", StateField::F64(state.phase_s)),
        ("rod_compensation_percent_dk_k", StateField::F64(state.rod_compensation_percent_dk_k)),
        ("circulation_loss_percent_dk_k", StateField::F64(state.circulation_loss_percent_dk_k)),
        ("loss_at_start_percent_dk_k", StateField::F64(state.loss_at_start_percent_dk_k)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
    ];
    for group in 0..6 {
        fields.push((VESSEL[group], StateField::F64(vessel[group])));
    }
    for group in 0..6 {
        fields.push((OUTSIDE[group], StateField::F64(outside[group])));
    }
    fields.push(("flow_changing", StateField::Bool(state.flow_changing)));
    fields.push(("fuel_circulating", StateField::Bool(state.fuel_circulating)));
    finish(&fields)
}

/// The card at rest: the published fields of the static start, the pump stopped.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = reactor_msre_pump::ReactorMsrePump::specimen();
    let state = reactor_msre_pump::ReactorMsrePumpState::try_start(&card, 0.0).expect("the reactor rests");
    with(&state, finish)
}
