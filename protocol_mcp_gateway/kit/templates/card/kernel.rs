#![no_implicit_prelude]

//! {{Title}}: the company's own specimen.
//!
//! STAMPED FROM THE KIT TEMPLATE. The physics below is the template's: a body
//! of one heat capacity, warmed by a heater and cooled through one
//! conductance to a bath. Replace it with the model's own, keeping the shape:
//! constants with the unit in each name, `StepReject` with a sentence above
//! each variant, a load, the specimen, a state and `step`. Then remove this
//! paragraph.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes into [`LastStateFrame64`]; the 16-byte proof is not
//! touched.
//!
//! The source of every number is the company's own derivation. The step is
//! the exact map of the equation, not a Runge-Kutta step:
//!
//! ```text
//! C dT/dt = P - G (T - T_bath)
//! T_eq = T_bath + P / G
//! T(t+dt) = T + (T_eq - T) (1 - exp(-dt G / C))
//! U = C (T - T_ref)
//! Q_boundary = C (T(t+dt) - T)
//! residual_conservation_error = |U - U_anchor - Q_boundary| / U_card
//! ```
//!
//! Its limits are physical: a temperature at or below absolute zero, and a
//! temperature at or above the melting point of the body, where the body is
//! no longer the solid this equation describes.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// The body's heat capacity. The company's own specimen.
pub const HEAT_CAPACITY_J_PER_K: f64 = 500.0;
/// The conductance from the body to the bath.
pub const CONDUCTANCE_W_PER_K: f64 = 0.5;
/// Zero of the ledger's internal energy.
pub const REFERENCE_TEMPERATURE_K: f64 = 293.15;
/// The melting point of the body, aluminium at one atmosphere. At or above
/// it the body is not a solid, and the step is refused.
pub const MELTING_POINT_K: f64 = 933.47;
/// `|U - U_anchor - Q_boundary| / U_card` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// The phase advance `omega dt` at the step ceiling, with `omega = G / C`.
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Why a step did not advance the body or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The heater power, the bath temperature or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this specimen's.
    InvalidSpecimen,
    /// The body or the bath is at or below absolute zero, incoming or landing.
    BelowAbsoluteZero,
    /// The body or the bath is at or above the melting point of the body, incoming or landing.
    AboveMeltingPoint,
    /// The trial step created heat from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed heat.
    FirstLawBreach,
}

/// Held across one step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct {{Name}}Load {
    pub heat_power_w: f64,
    pub ambient_k: f64,
}

/// The specimen. Every field is a recorded copy of a constant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct {{Name}} {
    pub heat_capacity_j_per_k: f64,
    pub conductance_w_per_k: f64,
    pub reference_k: f64,
    pub melting_point_k: f64,
}

impl {{Name}} {
    /// The company's own specimen.
    #[must_use]
    pub fn specimen() -> Self {
        let card = Self {
            heat_capacity_j_per_k: HEAT_CAPACITY_J_PER_K,
            conductance_w_per_k: CONDUCTANCE_W_PER_K,
            reference_k: REFERENCE_TEMPERATURE_K,
            melting_point_k: MELTING_POINT_K,
        };
        if card.is_valid() {
            card
        } else {
            ::core::panic!("the specimen failed is_valid");
        }
    }

    /// Every recorded copy equals its constant, bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.heat_capacity_j_per_k.to_bits() == HEAT_CAPACITY_J_PER_K.to_bits()
            && self.conductance_w_per_k.to_bits() == CONDUCTANCE_W_PER_K.to_bits()
            && self.reference_k.to_bits() == REFERENCE_TEMPERATURE_K.to_bits()
            && self.melting_point_k.to_bits() == MELTING_POINT_K.to_bits()
    }

    /// `G / C`.
    #[must_use]
    pub fn pole_rad_s(&self) -> f64 {
        self.conductance_w_per_k / self.heat_capacity_j_per_k
    }

    /// The largest step the card stands behind: the body moves two hundredths
    /// of its distance to equilibrium. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let omega = self.pole_rad_s();
        if omega.is_finite() && omega > 0.0 { Some(GATE_PHASE_STEP_RAD / omega) } else { None }
    }

    /// The smallest step the card stands behind: the ceiling over 2^20. Not
    /// applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate) => Some(gate / GATE_OVER_FLOOR),
            None => None,
        }
    }

    /// The ledger's scale: the heat from the reference to the melting point.
    #[must_use]
    pub fn card_energy_j(&self) -> f64 {
        self.heat_capacity_j_per_k * (self.melting_point_k - self.reference_k)
    }

    #[must_use]
    pub fn internal_energy_j(&self, temperature_k: f64) -> f64 {
        self.heat_capacity_j_per_k * (temperature_k - self.reference_k)
    }
}

/// The body. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct {{Name}}State {
    pub t_s: f64,
    pub temperature_k: f64,
    pub heat_power_w: f64,
    pub ambient_k: f64,
    pub internal_energy_j: f64,
    pub energy_anchor_j: f64,
    pub boundary_work_j: f64,
    /// Signed `U - U_anchor - Q_boundary`. Positive is created heat.
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    pub heat_to_ambient_w: f64,
    pub melt_margin_k: f64,
}

impl {{Name}}State {
    /// The body at `temperature_k`, the ledger anchored there, the bath at the reference.
    #[must_use]
    pub fn try_start(card: &{{Name}}, temperature_k: f64) -> Option<Self> {
        if !card.is_valid() || !inside(card, temperature_k) {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            temperature_k,
            heat_power_w: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
            internal_energy_j: 0.0,
            energy_anchor_j: 0.0,
            boundary_work_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            heat_to_ambient_w: 0.0,
            melt_margin_k: 0.0,
        };
        refresh(card, &mut state);
        if !finite(&state) {
            return None;
        }
        state.energy_anchor_j = state.internal_energy_j;
        Some(state)
    }
}

fn inside(card: &{{Name}}, temperature_k: f64) -> bool {
    temperature_k > 0.0 && temperature_k < card.melting_point_k
}

/// Advance one exact step and write the frame. A refused step leaves `state`
/// and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    card: &{{Name}},
    state: &mut {{Name}}State,
    frame: &mut LastStateFrame64,
    load: {{Name}}Load,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(card, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    card: &{{Name}},
    state: &mut {{Name}}State,
    frame: &mut LastStateFrame64,
    load: {{Name}}Load,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(card, state, load, dt_s)?;
    let landed = exact_map(card, state.temperature_k, load, dt_s).ok_or(StepReject::NonFiniteState)?;
    commit(card, state, frame, load, dt_s, landed)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number. Every 64-bit field of the state
/// is listed; a field added to the state is added here.
fn normal_or_zero_state(state: &mut {{Name}}State) {
    state.t_s = normal_or_zero(state.t_s);
    state.temperature_k = normal_or_zero(state.temperature_k);
    state.heat_power_w = normal_or_zero(state.heat_power_w);
    state.ambient_k = normal_or_zero(state.ambient_k);
    state.internal_energy_j = normal_or_zero(state.internal_energy_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.boundary_work_j = normal_or_zero(state.boundary_work_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.heat_to_ambient_w = normal_or_zero(state.heat_to_ambient_w);
    state.melt_margin_k = normal_or_zero(state.melt_margin_k);
}

fn validate(card: &{{Name}}, state: &{{Name}}State, load: {{Name}}Load, dt_s: f64) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.heat_power_w.is_finite() || !load.ambient_k.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !card.is_valid() {
        return Err(StepReject::InvalidSpecimen);
    }
    if !(state.temperature_k > 0.0) || !(load.ambient_k > 0.0) {
        return Err(StepReject::BelowAbsoluteZero);
    }
    if !(state.temperature_k < card.melting_point_k) || !(load.ambient_k < card.melting_point_k) {
        return Err(StepReject::AboveMeltingPoint);
    }
    Ok(())
}

struct Landed {
    temperature_k: f64,
    heat_j: f64,
}

/// `T + (T_eq - T) (1 - e^{-dt G/C})`, and the heat booked, `C` times the
/// rise that landed.
fn exact_map(card: &{{Name}}, temperature_k: f64, load: {{Name}}Load, dt_s: f64) -> Option<Landed> {
    let equilibrium_k = load.ambient_k + load.heat_power_w / card.conductance_w_per_k;
    let one_minus_decay = -crate::math::exp_m1(-dt_s * card.pole_rad_s());
    let next_k = temperature_k + (equilibrium_k - temperature_k) * one_minus_decay;
    let heat_j = card.heat_capacity_j_per_k * (next_k - temperature_k);
    if next_k.is_finite() && heat_j.is_finite() { Some(Landed { temperature_k: next_k, heat_j }) } else { None }
}

fn commit(
    card: &{{Name}},
    state: &mut {{Name}}State,
    frame: &mut LastStateFrame64,
    load: {{Name}}Load,
    dt_s: f64,
    landed: Landed,
) -> Result<(), StepReject> {
    if !(landed.temperature_k > 0.0) {
        return Err(StepReject::BelowAbsoluteZero);
    }
    if !(landed.temperature_k < card.melting_point_k) {
        return Err(StepReject::AboveMeltingPoint);
    }
    let mut next = *state;
    next.temperature_k = landed.temperature_k;
    next.boundary_work_j += landed.heat_j;
    next.heat_power_w = load.heat_power_w;
    next.ambient_k = load.ambient_k;
    next.t_s += dt_s;
    refresh(card, &mut next);
    next.energy_defect_j = next.internal_energy_j - next.energy_anchor_j - next.boundary_work_j;
    next.residual_conservation_error = next.energy_defect_j.abs() / card.card_energy_j();
    if !finite(&next) {
        return Err(StepReject::NonFiniteState);
    }
    let gate = RESIDUAL_CONSERVATION_LIMIT * card.card_energy_j();
    if next.energy_defect_j > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if next.energy_defect_j < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    *state = next;
    publish(state, frame);
    Ok(())
}

fn refresh(card: &{{Name}}, state: &mut {{Name}}State) {
    state.internal_energy_j = card.internal_energy_j(state.temperature_k);
    state.heat_to_ambient_w = card.conductance_w_per_k * (state.temperature_k - state.ambient_k);
    state.melt_margin_k = card.melting_point_k - state.temperature_k;
}

fn finite(state: &{{Name}}State) -> bool {
    state.t_s.is_finite()
        && state.temperature_k.is_finite()
        && state.heat_power_w.is_finite()
        && state.ambient_k.is_finite()
        && state.internal_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.boundary_work_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.heat_to_ambient_w.is_finite()
        && state.melt_margin_k.is_finite()
}

fn publish(state: &{{Name}}State, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.temperature_k as f32;
    frame.pos[1] = state.ambient_k as f32;
    frame.pos[2] = state.heat_power_w as f32;
    frame.vel[0] = state.internal_energy_j as f32;
    frame.vel[1] = state.boundary_work_j as f32;
    frame.vel[2] = state.heat_to_ambient_w as f32;
    frame.force_torque = state.melt_margin_k as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = 0;
}

/// Forward Euler temperature against the exact boundary heat: a wrong
/// stepping method, for the tests.
#[cfg(test)]
fn step_euler(
    card: &{{Name}},
    state: &mut {{Name}}State,
    frame: &mut LastStateFrame64,
    load: {{Name}}Load,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(card, state, load, dt_s)?;
    let exact = exact_map(card, state.temperature_k, load, dt_s).ok_or(StepReject::NonFiniteState)?;
    let equilibrium_k = load.ambient_k + load.heat_power_w / card.conductance_w_per_k;
    let euler_k = state.temperature_k + (equilibrium_k - state.temperature_k) * dt_s * card.pole_rad_s();
    commit(card, state, frame, load, dt_s, Landed { temperature_k: euler_k, heat_j: exact.heat_j })
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::*;

    fn blank() -> LastStateFrame64 {
        LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [7; 16] }
    }

    fn heated() -> {{Name}}Load {
        {{Name}}Load { heat_power_w: 100.0, ambient_k: REFERENCE_TEMPERATURE_K }
    }

    #[test]
    fn the_specimen_holds_its_constants_and_a_moved_one_is_refused() {
        let card = {{Name}}::specimen();
        assert!(card.is_valid());
        let mut moved = card;
        moved.conductance_w_per_k = f64::from_bits(CONDUCTANCE_W_PER_K.to_bits() + 1);
        assert!(!moved.is_valid());
        let mut state = {{Name}}State::try_start(&card, 300.0).unwrap();
        let before = state;
        let mut frame = blank();
        assert_eq!(step(&moved, &mut state, &mut frame, heated(), 1.0), Err(StepReject::InvalidSpecimen));
        assert_eq!(state, before);
    }

    #[test]
    fn the_ledger_closes_at_every_step_from_the_floor_to_the_ceiling() {
        let card = {{Name}}::specimen();
        for dt_s in [card.floor_dt_s().unwrap(), card.gate_dt_s().unwrap()] {
            let mut state = {{Name}}State::try_start(&card, 300.0).unwrap();
            let mut frame = blank();
            for _ in 0..1000 {
                step(&card, &mut state, &mut frame, heated(), dt_s).unwrap();
                assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                assert_eq!(frame.proof, [7; 16]);
            }
        }
    }

    #[test]
    fn the_melting_point_is_refused_by_name_with_the_state_whole() {
        let card = {{Name}}::specimen();
        let mut state = {{Name}}State::try_start(&card, MELTING_POINT_K - 0.001).unwrap();
        let before = state;
        let mut frame = blank();
        let hot = {{Name}}Load { heat_power_w: 1.0e4, ambient_k: REFERENCE_TEMPERATURE_K };
        assert_eq!(step(&card, &mut state, &mut frame, hot, card.gate_dt_s().unwrap()), Err(StepReject::AboveMeltingPoint));
        assert_eq!(state, before);
        assert_eq!(frame.proof, [7; 16]);
        assert!({{Name}}State::try_start(&card, 0.0).is_none());
        assert!({{Name}}State::try_start(&card, MELTING_POINT_K).is_none());
    }

    #[test]
    fn nothing_is_filled_in() {
        let card = {{Name}}::specimen();
        let mut state = {{Name}}State::try_start(&card, 300.0).unwrap();
        let before = state;
        let mut frame = blank();
        for load in [
            {{Name}}Load { heat_power_w: f64::NAN, ambient_k: 300.0 },
            {{Name}}Load { heat_power_w: 1.0, ambient_k: f64::INFINITY },
        ] {
            assert_eq!(step(&card, &mut state, &mut frame, load, 1.0), Err(StepReject::NonFiniteInput));
            assert_eq!(state, before);
        }
        assert_eq!(step(&card, &mut state, &mut frame, heated(), 0.0), Err(StepReject::NonPositiveDt));
        assert_eq!(state, before);
    }

    /// Rule 10: a wrong stepping method is named by the ledger.
    #[test]
    fn a_forward_euler_step_is_named_by_the_ledger() {
        let card = {{Name}}::specimen();
        let mut state = {{Name}}State::try_start(&card, 300.0).unwrap();
        let before = state;
        let mut frame = blank();
        let hot = {{Name}}Load { heat_power_w: 200.0, ambient_k: REFERENCE_TEMPERATURE_K };
        let named = step_euler(&card, &mut state, &mut frame, hot, card.gate_dt_s().unwrap() * 0.9);
        assert!(named == Err(StepReject::SpontaneousGeneration) || named == Err(StepReject::FirstLawBreach), "{named:?}");
        assert_eq!(state, before);
    }

    /// Rule 10: a falsified ledger is named.
    #[test]
    fn a_falsified_ledger_is_named() {
        let card = {{Name}}::specimen();
        let mut state = {{Name}}State::try_start(&card, 300.0).unwrap();
        state.energy_anchor_j -= 1.0;
        let mut frame = blank();
        assert_eq!(step(&card, &mut state, &mut frame, heated(), 1.0), Err(StepReject::SpontaneousGeneration));
    }

    #[test]
    fn the_same_run_gives_the_same_bits() {
        let card = {{Name}}::specimen();
        let run = || {
            let mut state = {{Name}}State::try_start(&card, 300.0).unwrap();
            let mut frame = blank();
            for _ in 0..100 {
                step(&card, &mut state, &mut frame, heated(), 3.0).unwrap();
            }
            frame.to_bytes()
        };
        assert_eq!(run(), run());
    }
}
