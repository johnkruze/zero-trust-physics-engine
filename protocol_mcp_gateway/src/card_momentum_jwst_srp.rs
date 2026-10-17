//! The gateway's call of the momentum_jwst_srp card: it reads a request,
//! holds it to the card's published bounds, steps the card and publishes its
//! state. Its description is `card_momentum_jwst_srp_descriptor.rs`, a child
//! of `descriptor.rs`.

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::momentum_jwst_srp;

use crate::live::StepFailure;
use crate::{
    IntegratedFrame, Published, StateField, blank_frame, card_clock, descriptor, enforce_published, fields_are_finite,
    phase_not_valid, refused, reject_unknown_load, required_f64, required_vec3, state_object,
};

pub(crate) const DOMAIN: &str = "aerospace";
pub(crate) const CARD: &str = "momentum_jwst_srp";

/// The flags the card declares, ORed.
pub(crate) const FLAGS: u64 = momentum_jwst_srp::FLAG_OUTSIDE_FIELD_OF_REGARD;

pub(crate) struct MomentumJwstSrpCall {
    card: momentum_jwst_srp::MomentumJwstSrp,
    pub(crate) state: momentum_jwst_srp::MomentumState,
    load: momentum_jwst_srp::MomentumLoad,
    dt_s: f64,
}

/// The sentence of each refusal the card gives part way: the card's own
/// sentence, with its name.
fn reject_sentence(reject: momentum_jwst_srp::StepReject) -> String {
    refused(CARD, reject)
}

fn read_load(params: &serde_json::Value) -> Result<momentum_jwst_srp::MomentumLoad, String> {
    Ok(momentum_jwst_srp::MomentumLoad {
        q1: required_f64(params, "q1")?,
        q2: required_f64(params, "q2")?,
        q3: required_f64(params, "q3")?,
        q4: required_f64(params, "q4")?,
        sun_x: required_f64(params, "sun_x")?,
        sun_y: required_f64(params, "sun_y")?,
        sun_z: required_f64(params, "sun_z")?,
        sun_distance_au: required_f64(params, "sun_distance_au")?,
    })
}

impl MomentumJwstSrpCall {
    pub(crate) fn from_parameters(params: &serde_json::Value) -> Result<Self, String> {
        reject_unknown_load(params, descriptor::parameter_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Step, params)?;
        let h = required_vec3(params, "h_eci_nms")?;
        let load = read_load(params)?;
        let dt_s = required_f64(params, "dt_s")?;
        let card = momentum_jwst_srp::MomentumJwstSrp::observatory();
        card_clock(dt_s, None, card.floor_dt_s(), card.gate_dt_s(), CARD)?;
        let state = momentum_jwst_srp::MomentumState::try_start(&card, h).ok_or_else(|| phase_not_valid(CARD))?;
        Ok(Self { card, state, load, dt_s })
    }

    pub(crate) fn replace_load(&mut self, params: &serde_json::Value) -> Result<(), String> {
        reject_unknown_load(params, descriptor::load_names(DOMAIN, CARD))?;
        enforce_published(DOMAIN, CARD, Published::Load, params)?;
        self.load = read_load(params)?;
        Ok(())
    }

    pub(crate) fn step_into(&mut self, frame: &mut LastStateFrame64) -> Result<(), StepFailure> {
        momentum_jwst_srp::step(&self.card, &mut self.state, frame, self.load, self.dt_s)
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

pub(crate) fn state(state: &momentum_jwst_srp::MomentumState) -> Result<serde_json::Value, String> {
    with(state, state_object)
}

pub(crate) fn published_is_finite(state: &momentum_jwst_srp::MomentumState) -> Result<(), String> {
    with(state, fields_are_finite)
}

/// The published fields of the state, in order.
pub(crate) fn with<T>(state: &momentum_jwst_srp::MomentumState, finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    finish(&[
        ("t_s", StateField::F64(state.t_s)),
        ("h_eci_nms", StateField::F64x3(state.h_eci_nms)),
        ("h_magnitude_nms", StateField::F64(state.h_magnitude_nms)),
        ("h_growth_nm", StateField::F64(state.h_growth_nm)),
        ("torque_eci_nm", StateField::F64x3(state.torque_eci_nm)),
        ("torque_body_nm", StateField::F64x3(state.torque_body_nm)),
        ("force_n", StateField::F64(state.force_n)),
        ("sun_pitch_deg", StateField::F64(state.sun_pitch_deg)),
        ("sun_roll_deg", StateField::F64(state.sun_roll_deg)),
        ("incidence_cosine", StateField::F64(state.incidence_cosine)),
        ("irradiance_w_per_m2", StateField::F64(state.irradiance_w_per_m2)),
        ("layer1_temperature_k", StateField::F64(state.layer1_temperature_k)),
        ("absorbed_w", StateField::F64(state.absorbed_w)),
        ("emitted_front_w", StateField::F64(state.emitted_front_w)),
        ("emitted_back_w", StateField::F64(state.emitted_back_w)),
        ("energy_defect_w", StateField::F64(state.energy_defect_w)),
        ("h_anchor_eci_nms", StateField::F64x3(state.h_anchor_eci_nms)),
        ("impulse_eci_nms", StateField::F64x3(state.impulse_eci_nms)),
        ("momentum_defect_nms", StateField::F64(state.momentum_defect_nms)),
        ("quaternion_norm_defect", StateField::F64(state.quaternion_norm_defect)),
        ("residual_conservation_error", StateField::F64(state.residual_conservation_error)),
        ("outside_field_of_regard", StateField::Bool(state.outside_field_of_regard)),
    ])
}

/// The card at rest: the published fields of a fresh start with no momentum.
pub(crate) fn rest<T>(finish: impl FnOnce(&[(&str, StateField)]) -> T) -> T {
    let card = momentum_jwst_srp::MomentumJwstSrp::observatory();
    let state = momentum_jwst_srp::MomentumState::try_start(&card, [0.0; 3]).expect("the observatory rests");
    with(&state, finish)
}
