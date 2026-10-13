#![no_implicit_prelude]

//! Mass-spring-damper at one locked natural frequency.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, displacement, velocity, the held acceleration,
//! kinetic energy, potential energy, mechanical energy, the stroke margin,
//! and the conservation residual into [`LastStateFrame64`]. The 16-byte
//! frame proof is not touched.
//!
//! The card is a 1 kg mass on a 10 Hz mount, damping ratio 1/20, stroke
//! 1/64 m. `ω` and `ζ` are crate constants. [`DynamicOscillator::try_new`]
//! does not exist: a caller does not pass a frequency or a damping ratio.
//!
//! ```text
//! ẍ + 2 ζ ω ẋ + ω² x = a
//! E = ½ m v² + ½ m ω² x²
//! dW/dt = m a v − c v²
//! c = 2 ζ ω m
//! residual_conservation_error = |E − E_anchor − W| / E_stroke
//! ```
//!
//! `E_stroke` is the harmonic energy at the card stroke. Damper heat leaves
//! the mechanical ledger, so it is boundary work. The public step is
//! classical RK4 on `(x, v, W)`. Explicit Euler is not the public step: from
//! rest at the stroke it creates kinetic energy on the first gate step.
//!
//! [`DynamicOscillator::gate_dt_s`] is `ω Δt = 0.02`. The public step does
//! not apply it. The force Nyquist limit is applied: `|a| dt ≤ ω L`, with
//! `L` the card stroke. Past it the step is [`StepReject::ForceAboveNyquist`]
//! and nothing is written. A landing at or beyond the stroke sets
//! [`FLAG_STROKE_LIMIT`] and does not reject the step.
//!
//! The card ends at a free ring of twice the stroke. A state whose mechanical
//! energy is above that ring's, four stroke energies, on the incoming state
//! or on the landing, is [`StepReject::AmplitudeAboveCard`].
//!
//! The ledger has a stated reach. RK4 is not exact for this mount, and the
//! residual is the step method's own error, summed since the anchor. At the
//! step ceiling a mount driven at resonance gains 2.92e-9 J of defect for
//! every joule its damper takes, so the limit is spent after 342 stroke
//! energies of damper heat: 447,824 steps under a square wave of ±3 m/s²,
//! which rings the mount at 0.62 of the stroke, and 175,815 under one of
//! ±4.8 m/s², which rings it at 0.99 of the stroke. The step after that is
//! refused as [`StepReject::SpontaneousGeneration`], though the mount is
//! inside its stroke and nothing is wrong with the step. The error is fourth
//! order: at half the step the same drive shows a fifteenth of the defect.
//!
//! The ledger runs across continued requests. A state is started with
//! [`OscillatorState::try_continue`], which takes the time, the anchor and
//! the booked work with the displacement and the velocity: a landed state
//! handed back with its own `t_s`, `energy_anchor_j` and `boundary_work_j`
//! goes on with the same anchor and the defect it had, bit for bit as if the
//! run had not been cut. A new start states `t_s` 0, no booked work and the
//! anchor at its own mechanical energy. A stated ledger the card could not
//! have written is [`StepReject::LedgerNotOfThisState`]: a negative anchor
//! or one above four stroke energies, booked work at `t_s` 0, or a defect
//! already past the limit. A stated `t_s` below zero or above
//! [`START_TIME_MAX_S`] is [`StepReject::StartTimeOutsideCard`]. The card
//! keeps nothing between calls: it holds a state to the ledger it is given.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Mount natural frequency.
pub const NATURAL_FREQUENCY_HZ: f64 = 10.0;
/// Damping ratio `c / (2 √(k m))`.
pub const DAMPING_RATIO: f64 = 0.05;
/// Proof mass of the mount.
pub const MASS_KG: f64 = 1.0;
/// Card stroke. Past this the frame is flagged.
pub const STROKE_M: f64 = 1.0 / 64.0;
/// Largest free-ring amplitude the card accepts: twice the stroke. The mount
/// is one linear spring and one linear damper, stated for its stroke; the
/// frame is flagged from the stroke on, and the card ends here. The test is
/// on mechanical energy, so it holds a state that carries its energy as
/// speed the same as one that carries it as displacement: `½ m v² + ½ k x²`
/// above that of a ring of this amplitude, four stroke energies, is outside
/// the card. A free ring released on the ceiling ends with a residual of
/// 1.2e-8, 86 times under the limit, all of it the step method's own error.
/// Without a ceiling a start at 1 km and beyond, stepped at the step floor,
/// was refused as created or destroyed energy through rounding alone.
pub const AMPLITUDE_CEILING_M: f64 = 1.0 / 32.0;

/// `|E − E_anchor − W| / E_stroke` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Largest `t_s` a started state may state: 1024 s. A run adds at most
/// 1,000,000 ceiling steps, 318.3 s, so its clock stays under 2048 s, where
/// the 64-bit clock still moves by a floor step to within four ten-thousandths
/// of it.
pub const START_TIME_MAX_S: f64 = 1024.0;
/// Phase advance `ω Δt` used by [`DynamicOscillator::gate_dt_s`].
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// `|x|` is at or beyond [`STROKE_M`] on this frame.
pub const FLAG_STROKE_LIMIT: u64 = 1 << 0;

/// Why a step did not advance the mass or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Acceleration or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// Frequency, damping, mass, or stroke is not this mount.
    InvalidOscillator,
    /// `|a| dt` is past `ω L`.
    ForceAboveNyquist,
    /// Mechanical energy, incoming or landing, is above that of a free ring of twice the stroke.
    AmplitudeAboveCard,
    /// The stated `t_s` of a started state is below zero or above 1024 s.
    StartTimeOutsideCard,
    /// The stated ledger is not one the card could have written for this state: an anchor below zero or above four stroke energies, booked work at `t_s` 0, or `E − E_anchor − W` already past the limit.
    LedgerNotOfThisState,
    /// Mechanical energy is above the anchor plus the booked work by more than the limit: a wrong step, or the step method's own error summed over a long driven run.
    SpontaneousGeneration,
    /// Mechanical energy is below the anchor plus the booked work by more than the limit: a wrong step, or the step method's own error summed over a long driven run.
    FirstLawBreach,
}

/// Zero-order hold applied across one step. `accel_m_s2` is the specific force.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OscillatorLoad {
    pub accel_m_s2: f64,
}

impl Default for OscillatorLoad {
    fn default() -> Self {
        Self { accel_m_s2: 0.0 }
    }
}

/// Locked mount. Frequency and damping are recorded copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicOscillator {
    pub omega_rad_s: f64,
    pub zeta: f64,
    pub mass_kg: f64,
    pub stroke_m: f64,
    /// Recorded copy of [`AMPLITUDE_CEILING_M`]. Not a caller input.
    pub amplitude_ceiling_m: f64,
}

impl DynamicOscillator {
    /// Card: 10 Hz, ζ = 0.05, 1 kg, stroke 1/64 m.
    #[must_use]
    pub fn mount_10hz() -> Self {
        let oscillator = Self {
            omega_rad_s: natural_frequency_rad_s(),
            zeta: DAMPING_RATIO,
            mass_kg: MASS_KG,
            stroke_m: STROKE_M,
            amplitude_ceiling_m: AMPLITUDE_CEILING_M,
        };
        if oscillator.is_valid() {
            oscillator
        } else {
            ::core::panic!("10 Hz mount failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.omega_rad_s.to_bits() == natural_frequency_rad_s().to_bits()
            && self.zeta.to_bits() == DAMPING_RATIO.to_bits()
            && self.mass_kg.to_bits() == MASS_KG.to_bits()
            && self.stroke_m.to_bits() == STROKE_M.to_bits()
            && self.amplitude_ceiling_m.to_bits() == AMPLITUDE_CEILING_M.to_bits()
            && self.amplitude_ceiling_m > self.stroke_m
            && self.omega_rad_s.is_finite()
            && self.omega_rad_s > 0.0
            && self.zeta >= 0.0
            && self.zeta < 1.0
            && self.mass_kg > 0.0
            && self.stroke_m > 0.0
            && self.stiffness_n_per_m().is_finite()
            && self.damping_n_s_per_m().is_finite()
            && self.stroke_energy_j().is_finite()
            && self.stroke_energy_j() > 0.0
    }

    #[must_use]
    pub fn stiffness_n_per_m(&self) -> f64 {
        self.mass_kg * self.omega_rad_s * self.omega_rad_s
    }

    #[must_use]
    pub fn damping_n_s_per_m(&self) -> f64 {
        2.0 * self.zeta * self.omega_rad_s * self.mass_kg
    }

    /// Harmonic energy at [`STROKE_M`].
    #[must_use]
    pub fn stroke_energy_j(&self) -> f64 {
        0.5 * self.stiffness_n_per_m() * self.stroke_m * self.stroke_m
    }

    /// Harmonic energy at [`AMPLITUDE_CEILING_M`]: four stroke energies.
    #[must_use]
    pub fn energy_ceiling_j(&self) -> f64 {
        0.5 * self.stiffness_n_per_m() * self.amplitude_ceiling_m * self.amplitude_ceiling_m
    }

    /// Largest reporting step of the mount. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.omega_rad_s.is_finite() && self.omega_rad_s > 0.0 {
            Some(GATE_PHASE_STEP_RAD / self.omega_rad_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step turns the mount's vibration by two hundredths of a radian. A million times smaller, a step
    /// changes the state by about two parts in a hundred million, which
    /// 64-bit arithmetic still resolves to eight figures. Under that a step
    /// is mostly rounding. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }

    /// `π / ω`. Not applied by [`step`]. The applied limit is [`Self::force_nyquist_m_s2`].
    #[must_use]
    pub fn nyquist_dt_s(&self) -> Option<f64> {
        if self.omega_rad_s.is_finite() && self.omega_rad_s > 0.0 {
            Some(core::f64::consts::PI / self.omega_rad_s)
        } else {
            None
        }
    }

    /// `omega * stroke`, in metres per second: the constant of the load rule
    /// `|a| * dt_s <= C`.
    #[must_use]
    pub fn accel_impulse_limit_m_s(&self) -> f64 {
        self.omega_rad_s * self.stroke_m
    }

    /// Largest `|a|` with `|a| * dt_s <= accel_impulse_limit_m_s`, the product
    /// rounded once to 64 bits and compared with the constant, as [`step`]
    /// applies it.
    #[must_use]
    pub fn force_nyquist_m_s2(&self, dt_s: f64) -> Option<f64> {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return None;
        }
        crate::impulse::largest(self.accel_impulse_limit_m_s(), dt_s)
    }

    #[must_use]
    pub fn mechanical_energy_j(&self, displacement_m: f64, velocity_m_s: f64) -> f64 {
        0.5 * self.mass_kg * velocity_m_s * velocity_m_s
            + 0.5 * self.stiffness_n_per_m() * displacement_m * displacement_m
    }
}

fn natural_frequency_rad_s() -> f64 {
    NATURAL_FREQUENCY_HZ * 2.0 * core::f64::consts::PI
}

/// Integrated mass. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OscillatorState {
    pub t_s: f64,
    pub displacement_m: f64,
    pub velocity_m_s: f64,
    pub accel_m_s2: f64,
    pub kinetic_j: f64,
    pub potential_j: f64,
    pub mechanical_energy_j: f64,
    pub energy_anchor_j: f64,
    pub boundary_work_j: f64,
    /// Signed `E − E_anchor − W`. Positive is created energy.
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    pub stroke_margin_m: f64,
    pub stroke_limited: bool,
}

impl OscillatorState {
    #[must_use]
    pub fn try_rest(oscillator: &DynamicOscillator) -> Option<Self> {
        Self::try_phase(oscillator, 0.0, 0.0)
    }

    /// Anchor the ledger at this displacement and velocity. The held acceleration starts at zero.
    #[must_use]
    pub fn try_phase(
        oscillator: &DynamicOscillator,
        displacement_m: f64,
        velocity_m_s: f64,
    ) -> Option<Self> {
        if !oscillator.is_valid() || !displacement_m.is_finite() || !velocity_m_s.is_finite() {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            displacement_m,
            velocity_m_s,
            accel_m_s2: 0.0,
            kinetic_j: 0.0,
            potential_j: 0.0,
            mechanical_energy_j: 0.0,
            energy_anchor_j: 0.0,
            boundary_work_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            stroke_margin_m: 0.0,
            stroke_limited: false,
        };
        refresh(oscillator, &mut state);
        if !state_is_finite(&state) {
            return None;
        }
        state.energy_anchor_j = state.mechanical_energy_j;
        state.boundary_work_j = 0.0;
        state.energy_defect_j = 0.0;
        state.residual_conservation_error = 0.0;
        Some(state)
    }

    /// Start from a stated time and ledger. A landed state's `t_s`,
    /// `energy_anchor_j` and `boundary_work_j`, handed back with its
    /// displacement and velocity, continue the run with its anchor and its
    /// defect. A new start is `t_s` 0, `boundary_work_j` 0 and
    /// `energy_anchor_j` the mechanical energy of the start. The held
    /// acceleration starts at zero.
    pub fn try_continue(
        oscillator: &DynamicOscillator,
        displacement_m: f64,
        velocity_m_s: f64,
        t_s: f64,
        energy_anchor_j: f64,
        boundary_work_j: f64,
    ) -> Result<Self, StepReject> {
        if !oscillator.is_valid() {
            return Err(StepReject::InvalidOscillator);
        }
        if !(displacement_m.is_finite()
            && velocity_m_s.is_finite()
            && t_s.is_finite()
            && energy_anchor_j.is_finite()
            && boundary_work_j.is_finite())
        {
            return Err(StepReject::NonFiniteState);
        }
        if !(t_s >= 0.0 && t_s <= START_TIME_MAX_S) {
            return Err(StepReject::StartTimeOutsideCard);
        }
        let mut state = match Self::try_phase(oscillator, displacement_m, velocity_m_s) {
            Some(state) => state,
            None => return Err(StepReject::NonFiniteState),
        };
        if state.mechanical_energy_j > oscillator.energy_ceiling_j() {
            return Err(StepReject::AmplitudeAboveCard);
        }
        if !(energy_anchor_j >= 0.0 && energy_anchor_j <= oscillator.energy_ceiling_j()) {
            return Err(StepReject::LedgerNotOfThisState);
        }
        if t_s == 0.0 && boundary_work_j != 0.0 {
            return Err(StepReject::LedgerNotOfThisState);
        }
        state.t_s = t_s;
        state.energy_anchor_j = energy_anchor_j;
        state.boundary_work_j = boundary_work_j;
        state.energy_defect_j = state.mechanical_energy_j - energy_anchor_j - boundary_work_j;
        let scale = oscillator.stroke_energy_j();
        state.residual_conservation_error = state.energy_defect_j.abs() / scale;
        let gate = RESIDUAL_CONSERVATION_LIMIT * scale;
        if !state_is_finite(&state) {
            return Err(StepReject::NonFiniteState);
        }
        if state.energy_defect_j > gate || state.energy_defect_j < -gate {
            return Err(StepReject::LedgerNotOfThisState);
        }
        Ok(state)
    }
}

/// Advance one RK4 step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    oscillator: &DynamicOscillator,
    state: &mut OscillatorState,
    frame: &mut LastStateFrame64,
    load: OscillatorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(oscillator, state, load, dt_s)?;
    let predicted = match rk4(oscillator, pack(state), dt_s, load.accel_m_s2) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(oscillator, state, frame, load, dt_s, predicted)
}

fn validate(
    oscillator: &DynamicOscillator,
    state: &OscillatorState,
    load: OscillatorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.accel_m_s2.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !oscillator.is_valid() {
        return Err(StepReject::InvalidOscillator);
    }
    let energy_j = oscillator.mechanical_energy_j(state.displacement_m, state.velocity_m_s);
    if !energy_j.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if energy_j > oscillator.energy_ceiling_j() {
        return Err(StepReject::AmplitudeAboveCard);
    }
    if exceeds_force_nyquist(oscillator, load.accel_m_s2, dt_s) {
        return Err(StepReject::ForceAboveNyquist);
    }
    Ok(())
}

fn exceeds_force_nyquist(oscillator: &DynamicOscillator, accel_m_s2: f64, dt_s: f64) -> bool {
    if accel_m_s2 == 0.0 {
        return false;
    }
    match oscillator.force_nyquist_m_s2(dt_s) {
        Some(_) => !crate::impulse::within(accel_m_s2, dt_s, oscillator.accel_impulse_limit_m_s()),
        None => true,
    }
}

fn commit(
    oscillator: &DynamicOscillator,
    state: &mut OscillatorState,
    frame: &mut LastStateFrame64,
    load: OscillatorLoad,
    dt_s: f64,
    predicted: [f64; 3],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite3(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    state.displacement_m = predicted[0];
    state.velocity_m_s = predicted[1];
    state.boundary_work_j = predicted[2];
    state.accel_m_s2 = load.accel_m_s2;
    state.t_s += dt_s;
    refresh(oscillator, state);
    let scale = oscillator.stroke_energy_j();
    if !(scale > 0.0 && scale.is_finite()) {
        *state = saved;
        return Err(StepReject::InvalidOscillator);
    }
    state.energy_defect_j =
        state.mechanical_energy_j - state.energy_anchor_j - state.boundary_work_j;
    state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.mechanical_energy_j > oscillator.energy_ceiling_j() {
        *state = saved;
        return Err(StepReject::AmplitudeAboveCard);
    }
    let gate = RESIDUAL_CONSERVATION_LIMIT * scale;
    if state.energy_defect_j > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    publish(state, frame);
    Ok(())
}

fn refresh(oscillator: &DynamicOscillator, state: &mut OscillatorState) {
    state.kinetic_j = 0.5 * oscillator.mass_kg * state.velocity_m_s * state.velocity_m_s;
    state.potential_j =
        0.5 * oscillator.stiffness_n_per_m() * state.displacement_m * state.displacement_m;
    state.mechanical_energy_j = state.kinetic_j + state.potential_j;
    state.stroke_margin_m = oscillator.stroke_m - state.displacement_m.abs();
    state.stroke_limited = state.displacement_m.abs() >= oscillator.stroke_m;
}

fn publish(state: &OscillatorState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.displacement_m as f32;
    frame.pos[1] = state.velocity_m_s as f32;
    frame.pos[2] = state.accel_m_s2 as f32;
    frame.vel[0] = state.kinetic_j as f32;
    frame.vel[1] = state.potential_j as f32;
    frame.vel[2] = state.mechanical_energy_j as f32;
    frame.force_torque = state.stroke_margin_m as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.stroke_limited {
        FLAG_STROKE_LIMIT
    } else {
        0
    };
}

fn derivatives(oscillator: &DynamicOscillator, y: [f64; 3], accel_m_s2: f64) -> Option<[f64; 3]> {
    let displacement_m = y[0];
    let velocity_m_s = y[1];
    let omega = oscillator.omega_rad_s;
    let acceleration =
        accel_m_s2 - 2.0 * oscillator.zeta * omega * velocity_m_s - omega * omega * displacement_m;
    let power_w = oscillator.mass_kg * accel_m_s2 * velocity_m_s
        - oscillator.damping_n_s_per_m() * velocity_m_s * velocity_m_s;
    let out = [velocity_m_s, acceleration, power_w];
    if finite3(out) { Some(out) } else { None }
}

fn rk4(oscillator: &DynamicOscillator, y: [f64; 3], dt: f64, accel_m_s2: f64) -> Option<[f64; 3]> {
    let k1 = derivatives(oscillator, y, accel_m_s2)?;
    let k2 = derivatives(oscillator, axpy(y, dt * 0.5, k1), accel_m_s2)?;
    let k3 = derivatives(oscillator, axpy(y, dt * 0.5, k2), accel_m_s2)?;
    let k4 = derivatives(oscillator, axpy(y, dt, k3), accel_m_s2)?;
    let mut next = y;
    let mut i = 0;
    while i < 3 {
        let increment = k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i];
        // A zero rate times an infinite `dt` is 0. IEEE 754 would make that a NaN.
        if increment != 0.0 {
            next[i] += (dt / 6.0) * increment;
        }
        i += 1;
    }
    if finite3(next) { Some(next) } else { None }
}

fn axpy(y: [f64; 3], scale: f64, k: [f64; 3]) -> [f64; 3] {
    [
        mul_dt(y[0], scale, k[0]),
        mul_dt(y[1], scale, k[1]),
        mul_dt(y[2], scale, k[2]),
    ]
}

fn mul_dt(y: f64, scale: f64, k: f64) -> f64 {
    if k == 0.0 { y } else { y + scale * k }
}

fn pack(state: &OscillatorState) -> [f64; 3] {
    [
        state.displacement_m,
        state.velocity_m_s,
        state.boundary_work_j,
    ]
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn state_is_finite(state: &OscillatorState) -> bool {
    state.t_s.is_finite()
        && state.displacement_m.is_finite()
        && state.velocity_m_s.is_finite()
        && state.accel_m_s2.is_finite()
        && state.kinetic_j.is_finite()
        && state.potential_j.is_finite()
        && state.mechanical_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.boundary_work_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.stroke_margin_m.is_finite()
}

/// Explicit Euler on `(x, v, W)`. One gate step from the stroke creates kinetic energy.
#[cfg(test)]
fn step_euler(
    oscillator: &DynamicOscillator,
    state: &mut OscillatorState,
    frame: &mut LastStateFrame64,
    load: OscillatorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(oscillator, state, load, dt_s)?;
    let predicted = match explicit_euler(oscillator, pack(state), dt_s, load.accel_m_s2) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(oscillator, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
fn explicit_euler(
    oscillator: &DynamicOscillator,
    y: [f64; 3],
    dt: f64,
    accel_m_s2: f64,
) -> Option<[f64; 3]> {
    let k1 = derivatives(oscillator, y, accel_m_s2)?;
    let next = axpy(y, dt, k1);
    if finite3(next) { Some(next) } else { None }
}

/// Add `extra_j` to the integrated work after a faithful RK4 step.
#[cfg(test)]
fn step_bias_work(
    oscillator: &DynamicOscillator,
    state: &mut OscillatorState,
    frame: &mut LastStateFrame64,
    load: OscillatorLoad,
    dt_s: f64,
    extra_j: f64,
) -> Result<(), StepReject> {
    validate(oscillator, state, load, dt_s)?;
    let mut predicted = match rk4(oscillator, pack(state), dt_s, load.accel_m_s2) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    predicted[2] += extra_j;
    commit(oscillator, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        DAMPING_RATIO, DynamicOscillator, FLAG_STROKE_LIMIT, MASS_KG, NATURAL_FREQUENCY_HZ,
        OscillatorLoad, OscillatorState, RESIDUAL_CONSERVATION_LIMIT, STROKE_M, StepReject,
        natural_frequency_rad_s, step, step_bias_work, step_euler,
    };
    use crate::frame::LastStateFrame64;

    fn blank_frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: -1.0,
            pos: [1.0, 2.0, 3.0],
            vel: [4.0, 5.0, 6.0],
            force_torque: 7.0,
            residual: 8.0,
            flags: 9,
            proof: [0xA5; 16],
        }
    }

    fn proof_intact(frame: &LastStateFrame64) {
        let mut i = 0;
        while i < 16 {
            assert_eq!(frame.proof[i], 0xA5, "proof byte {i} was resealed");
            i += 1;
        }
    }

    fn assert_copy<T: Copy>() {}

    fn underdamped(x0: f64, v0: f64, omega: f64, zeta: f64, t: f64) -> (f64, f64) {
        let wd = omega * (1.0 - zeta * zeta).sqrt();
        let decay = (-zeta * omega * t).exp();
        let angle = wd * t;
        let bump = v0 + zeta * omega * x0;
        let phi = x0 * angle.cos() + (bump / wd) * angle.sin();
        let phi_dot = -x0 * wd * angle.sin() + bump * angle.cos();
        let x = decay * phi;
        let v = -zeta * omega * x + decay * phi_dot;
        (x, v)
    }

    #[test]
    fn card_locks_omega_and_zeta() {
        assert_copy::<DynamicOscillator>();
        assert_copy::<OscillatorState>();
        assert_copy::<OscillatorLoad>();
        assert_copy::<StepReject>();
        let oscillator = DynamicOscillator::mount_10hz();
        assert_eq!(
            oscillator.omega_rad_s.to_bits(),
            natural_frequency_rad_s().to_bits()
        );
        assert_eq!(oscillator.zeta.to_bits(), DAMPING_RATIO.to_bits());
        assert_eq!(NATURAL_FREQUENCY_HZ, 10.0);
        assert_eq!(MASS_KG, 1.0);
        let gate = oscillator.gate_dt_s().unwrap();
        let nyquist = oscillator.nyquist_dt_s().unwrap();
        assert!(nyquist > gate);
        assert!((nyquist - core::f64::consts::PI / oscillator.omega_rad_s).abs() < 1.0e-15);
        let mut tampered = oscillator;
        tampered.zeta = 0.2;
        assert!(!tampered.is_valid());
        let mut state = OscillatorState::try_rest(&oscillator).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(
            &tampered,
            &mut state,
            &mut frame,
            OscillatorLoad::default(),
            gate,
        );
        assert_eq!(got, Err(StepReject::InvalidOscillator));
        assert_eq!(state, saved);
        proof_intact(&frame);
    }

    #[test]
    fn free_ring_tracks_the_underdamped_solution() {
        let oscillator = DynamicOscillator::mount_10hz();
        let x0 = STROKE_M / 5.0;
        let mut state = OscillatorState::try_phase(&oscillator, x0, 0.0).unwrap();
        let mut frame = blank_frame();
        let dt = oscillator.gate_dt_s().unwrap();
        let steps = 2_000u32;
        let mut max_residual = 0.0;
        let mut n = 0u32;
        while n < steps {
            step(
                &oscillator,
                &mut state,
                &mut frame,
                OscillatorLoad::default(),
                dt,
            )
            .unwrap();
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            n += 1;
        }
        let (x, v) = underdamped(x0, 0.0, oscillator.omega_rad_s, oscillator.zeta, state.t_s);
        assert!(
            (state.displacement_m - x).abs() < 1.0e-9,
            "x {} analytic {x}",
            state.displacement_m
        );
        assert!(
            (state.velocity_m_s - v).abs() < 1.0e-7,
            "v {} analytic {v}",
            state.velocity_m_s
        );
        assert!(
            max_residual <= RESIDUAL_CONSERVATION_LIMIT,
            "residual {max_residual}"
        );
        assert!(!state.stroke_limited);
        proof_intact(&frame);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
    }

    #[test]
    fn rest_at_huge_dt_holds_and_the_gate_is_not_applied() {
        let oscillator = DynamicOscillator::mount_10hz();
        let mut state = OscillatorState::try_rest(&oscillator).unwrap();
        let mut frame = blank_frame();
        step(
            &oscillator,
            &mut state,
            &mut frame,
            OscillatorLoad::default(),
            f64::MAX,
        )
        .unwrap();
        assert_eq!(state.displacement_m, 0.0);
        assert_eq!(state.residual_conservation_error, 0.0);
        let wide = oscillator.nyquist_dt_s().unwrap() * 10.0;
        step(
            &oscillator,
            &mut state,
            &mut frame,
            OscillatorLoad::default(),
            wide,
        )
        .unwrap();
        assert_eq!(state.residual_conservation_error, 0.0);
        proof_intact(&frame);
    }

    #[test]
    fn force_nyquist_refuses_and_the_limit_itself_lands() {
        let oscillator = DynamicOscillator::mount_10hz();
        let dt = oscillator.gate_dt_s().unwrap();
        let limit = oscillator.force_nyquist_m_s2(dt).unwrap();
        let mut state = OscillatorState::try_rest(&oscillator).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let hot = OscillatorLoad {
            accel_m_s2: limit + 1.0,
        };
        let got = step(&oscillator, &mut state, &mut frame, hot, dt);
        assert_eq!(got, Err(StepReject::ForceAboveNyquist));
        assert_eq!(state, saved);
        proof_intact(&frame);
        let on_limit = OscillatorLoad { accel_m_s2: limit };
        step(&oscillator, &mut state, &mut frame, on_limit, dt).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert_eq!(state.accel_m_s2.to_bits(), limit.to_bits());
        proof_intact(&frame);
    }

    #[test]
    fn explicit_euler_from_the_stroke_creates_energy() {
        let oscillator = DynamicOscillator::mount_10hz();
        let mut state = OscillatorState::try_phase(&oscillator, STROKE_M, 0.0).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step_euler(
            &oscillator,
            &mut state,
            &mut frame,
            OscillatorLoad::default(),
            oscillator.gate_dt_s().unwrap(),
        );
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        proof_intact(&frame);

        let mut rk = OscillatorState::try_phase(&oscillator, STROKE_M, 0.0).unwrap();
        assert!(rk.stroke_limited);
        step(
            &oscillator,
            &mut rk,
            &mut frame,
            OscillatorLoad::default(),
            oscillator.gate_dt_s().unwrap(),
        )
        .unwrap();
        assert!(rk.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(!rk.stroke_limited);

        let mut held = OscillatorState::try_phase(&oscillator, STROKE_M, 0.1).unwrap();
        step(
            &oscillator,
            &mut held,
            &mut frame,
            OscillatorLoad::default(),
            oscillator.gate_dt_s().unwrap(),
        )
        .unwrap();
        assert!(held.stroke_limited);
        assert_eq!(frame.flags, FLAG_STROKE_LIMIT);
        assert!(held.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    #[test]
    fn biased_work_and_non_finite_inputs_roll_back() {
        let oscillator = DynamicOscillator::mount_10hz();
        let mut state = OscillatorState::try_rest(&oscillator).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let dt = oscillator.gate_dt_s().unwrap();
        let extra = oscillator.stroke_energy_j() * 1.0e-3;
        let got = step_bias_work(
            &oscillator,
            &mut state,
            &mut frame,
            OscillatorLoad::default(),
            dt,
            extra,
        );
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state, saved);
        let got = step_bias_work(
            &oscillator,
            &mut state,
            &mut frame,
            OscillatorLoad::default(),
            dt,
            -extra,
        );
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        let mut refuse = |load: OscillatorLoad, dt_s: f64, expect: StepReject| {
            let got = step(&oscillator, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state, saved);
            proof_intact(&frame);
        };
        refuse(OscillatorLoad::default(), 0.0, StepReject::NonPositiveDt);
        refuse(OscillatorLoad::default(), -dt, StepReject::NonPositiveDt);
        refuse(
            OscillatorLoad::default(),
            f64::NAN,
            StepReject::NonFiniteInput,
        );
        refuse(
            OscillatorLoad {
                accel_m_s2: f64::NAN,
            },
            dt,
            StepReject::NonFiniteInput,
        );
        refuse(
            OscillatorLoad {
                accel_m_s2: f64::INFINITY,
            },
            dt,
            StepReject::NonFiniteInput,
        );
        refuse(
            OscillatorLoad {
                accel_m_s2: f64::MAX,
            },
            dt,
            StepReject::ForceAboveNyquist,
        );
        state.velocity_m_s = f64::MAX;
        let got = step(
            &oscillator,
            &mut state,
            &mut frame,
            OscillatorLoad::default(),
            dt,
        );
        assert_eq!(got, Err(StepReject::NonFiniteState));
        assert_eq!(state.velocity_m_s, f64::MAX);
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::DynamicOscillator::mount_10hz();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The mount 1 mm out, moving at 10 mm/s, under 1 m/s².
        let mut state = super::OscillatorState::try_phase(&card, 1.0e-3, 1.0e-2).unwrap();
        let load = super::OscillatorLoad { accel_m_s2: 1.0 };
        let before = state;
        let mut frame = crate::frame::LastStateFrame64 {
            t: 0.0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            force_torque: 0.0,
            residual: 0.0,
            flags: 0,
            proof: [0; 16],
        };
        super::step(&card, &mut state, &mut frame, load, floor_s).unwrap();
        assert_eq!(state.t_s.to_bits(), floor_s.to_bits());
        assert!(state.displacement_m != before.displacement_m);
        assert!(state.velocity_m_s != before.velocity_m_s);
        assert!(state.boundary_work_j != before.boundary_work_j);
        assert!(state.residual_conservation_error <= super::RESIDUAL_CONSERVATION_LIMIT);

        // A thousand more steps at the floor land: the ledger holds an
        // honest step this small.
        let mut taken = 0;
        while taken < 1_000 {
            super::step(&card, &mut state, &mut frame, load, floor_s).unwrap();
            taken += 1;
        }
        assert!(state.residual_conservation_error <= super::RESIDUAL_CONSERVATION_LIMIT);

        // At the largest time a run reaches, the clock still moves by a
        // floor step, to within a thousandth of it.
        let late_s = 1.0e6 * ceiling_s;
        let moved_s = (late_s + floor_s) - late_s;
        assert!(
            (moved_s - floor_s).abs() <= 1.0e-3 * floor_s,
            "the clock moved {moved_s} for a step of {floor_s}"
        );
    }

    fn state_bits(state: &OscillatorState) -> [u64; 13] {
        [
            state.t_s.to_bits(),
            state.displacement_m.to_bits(),
            state.velocity_m_s.to_bits(),
            state.accel_m_s2.to_bits(),
            state.kinetic_j.to_bits(),
            state.potential_j.to_bits(),
            state.mechanical_energy_j.to_bits(),
            state.energy_anchor_j.to_bits(),
            state.boundary_work_j.to_bits(),
            state.energy_defect_j.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.stroke_margin_m.to_bits(),
            u64::from(state.stroke_limited),
        ]
    }

    /// Step, and require this refusal with the state and the frame bytes as they were.
    fn refused_whole(
        oscillator: &DynamicOscillator,
        state: &mut OscillatorState,
        frame: &mut LastStateFrame64,
        load: OscillatorLoad,
        dt_s: f64,
        expect: StepReject,
    ) {
        let held = state_bits(state);
        let held_frame = frame.to_bytes();
        let got = step(oscillator, state, frame, load, dt_s);
        assert_eq!(got, Err(expect));
        assert_eq!(state_bits(state), held);
        assert_eq!(frame.to_bytes(), held_frame);
    }

    #[test]
    fn the_amplitude_ceiling_lands_and_the_next_number_past_it_is_refused() {
        let oscillator = DynamicOscillator::mount_10hz();
        let dt = oscillator.gate_dt_s().unwrap();
        let ceiling_m = super::AMPLITUDE_CEILING_M;
        assert_eq!(
            oscillator.amplitude_ceiling_m.to_bits(),
            ceiling_m.to_bits()
        );
        assert_eq!(ceiling_m.to_bits(), (2.0 * STROKE_M).to_bits());
        let ceiling_j = oscillator.energy_ceiling_j();
        assert_eq!(
            ceiling_j.to_bits(),
            (4.0 * oscillator.stroke_energy_j()).to_bits()
        );
        let free = OscillatorLoad::default();

        // Held at rest on the ceiling itself, either side: the step lands, flagged.
        let mut sign = 1.0;
        while sign >= -1.0 {
            let mut state = OscillatorState::try_phase(&oscillator, sign * ceiling_m, 0.0).unwrap();
            assert_eq!(state.mechanical_energy_j.to_bits(), ceiling_j.to_bits());
            let mut frame = blank_frame();
            step(&oscillator, &mut state, &mut frame, free, dt).unwrap();
            assert!(state.mechanical_energy_j < ceiling_j);
            assert_eq!(frame.flags, FLAG_STROKE_LIMIT);
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            proof_intact(&frame);

            // The next representable displacement is outside the card.
            let mut over =
                OscillatorState::try_phase(&oscillator, sign * ceiling_m.next_up(), 0.0).unwrap();
            let mut frame = blank_frame();
            refused_whole(
                &oscillator,
                &mut over,
                &mut frame,
                free,
                dt,
                StepReject::AmplitudeAboveCard,
            );
            proof_intact(&frame);
            sign -= 2.0;
        }

        // The same energy carried as speed through the rest point: find the
        // two neighbouring speeds of which one is inside and the next is not.
        let enters = |velocity_m_s: f64| {
            let mut state = OscillatorState::try_phase(&oscillator, 0.0, velocity_m_s).unwrap();
            let mut frame = blank_frame();
            match step(&oscillator, &mut state, &mut frame, free, dt) {
                Ok(()) => true,
                Err(StepReject::AmplitudeAboveCard) => false,
                Err(other) => std::panic!("speed {velocity_m_s} refused with {other:?}"),
            }
        };
        let ring_speed_m_s = oscillator.omega_rad_s * ceiling_m;
        let mut inside = 0.99 * ring_speed_m_s;
        let mut outside = 1.01 * ring_speed_m_s;
        assert!(enters(inside) && !enters(outside));
        while inside.next_up() < outside {
            let middle = inside + (outside - inside) * 0.5;
            if enters(middle) {
                inside = middle;
            } else {
                outside = middle;
            }
        }
        assert_eq!(inside.next_up().to_bits(), outside.to_bits());
        assert!((inside - ring_speed_m_s).abs() <= 4.0 * (inside.next_up() - inside));
        assert!(oscillator.mechanical_energy_j(0.0, inside) <= ceiling_j);
        assert!(oscillator.mechanical_energy_j(0.0, outside) > ceiling_j);

        // A landing: a held force along the motion adds energy. Find the two
        // neighbouring starting speeds of which one lands and the next is refused.
        let pushed = OscillatorLoad { accel_m_s2: 50.0 };
        let lands = |velocity_m_s: f64| {
            let mut state = OscillatorState::try_phase(&oscillator, 0.0, velocity_m_s).unwrap();
            let mut frame = blank_frame();
            match step(&oscillator, &mut state, &mut frame, pushed, dt) {
                Ok(()) => Some(state.mechanical_energy_j),
                Err(StepReject::AmplitudeAboveCard) => None,
                Err(other) => std::panic!("speed {velocity_m_s} refused with {other:?}"),
            }
        };
        let mut inside = 0.99 * ring_speed_m_s;
        let mut outside = ring_speed_m_s;
        assert!(lands(inside).is_some() && lands(outside).is_none());
        while inside.next_up() < outside {
            let middle = inside + (outside - inside) * 0.5;
            if lands(middle).is_some() {
                inside = middle;
            } else {
                outside = middle;
            }
        }
        assert_eq!(inside.next_up().to_bits(), outside.to_bits());
        let landed_j = lands(inside).unwrap();
        assert!(landed_j > oscillator.mechanical_energy_j(0.0, inside));
        assert!(landed_j <= ceiling_j);
        assert!(ceiling_j - landed_j <= 8.0 * (ceiling_j.next_up() - ceiling_j));
        let mut state = OscillatorState::try_phase(&oscillator, 0.0, outside).unwrap();
        let mut frame = blank_frame();
        refused_whole(
            &oscillator,
            &mut state,
            &mut frame,
            pushed,
            dt,
            StepReject::AmplitudeAboveCard,
        );
        proof_intact(&frame);

        // The recorded copy is locked like the other constants.
        let mut tampered = oscillator;
        tampered.amplitude_ceiling_m = 1.0;
        assert!(!tampered.is_valid());
        refused_whole(
            &tampered,
            &mut state,
            &mut frame,
            free,
            dt,
            StepReject::InvalidOscillator,
        );
    }

    /// Starts and loads that used to end in a conservation refusal. A free
    /// ring from 19 strokes landed 1,482 steps and one from 40 strokes 95,
    /// then both were refused as created energy; from 1,000
    /// strokes the second step was refused as destroyed energy. The largest
    /// force the card admits at its ceiling step landed 72 steps from rest,
    /// and half of it 391. At 1 km and beyond, stepped at the step floor, the
    /// first steps were refused through rounding alone.
    #[test]
    fn rings_and_loads_past_the_ceiling_are_refused_by_name_before_the_ledger() {
        let oscillator = DynamicOscillator::mount_10hz();
        let gate = oscillator.gate_dt_s().unwrap();
        let floor = oscillator.floor_dt_s().unwrap();
        let mut frame = blank_frame();
        let starts = [
            (18.0 * STROKE_M, gate),
            (19.0 * STROKE_M, gate),
            (40.0 * STROKE_M, gate),
            (1_000.0 * STROKE_M, gate),
            (1.0e3, floor),
            (1.0e6, floor),
            (1.0e10, floor),
            (3.0e38, floor),
        ];
        let mut i = 0;
        while i < starts.len() {
            let (displacement_m, dt_s) = starts[i];
            let mut state = OscillatorState::try_phase(&oscillator, displacement_m, 0.0).unwrap();
            refused_whole(
                &oscillator,
                &mut state,
                &mut frame,
                OscillatorLoad::default(),
                dt_s,
                StepReject::AmplitudeAboveCard,
            );
            // The same ring a quarter turn on: all of its energy is speed.
            let velocity_m_s = -oscillator.omega_rad_s * displacement_m;
            let mut state = OscillatorState::try_phase(&oscillator, 0.0, velocity_m_s).unwrap();
            refused_whole(
                &oscillator,
                &mut state,
                &mut frame,
                OscillatorLoad::default(),
                dt_s,
                StepReject::AmplitudeAboveCard,
            );
            i += 1;
        }

        // Held forces from rest: the mass is driven through the ceiling in a
        // few steps, and what lands before that is far inside the limit.
        let most = oscillator.force_nyquist_m_s2(gate).unwrap();
        let loads = [(most, 2u32), (0.5 * most, 4), (0.2 * most, 10)];
        let mut i = 0;
        while i < loads.len() {
            let (accel_m_s2, expect_landed) = loads[i];
            let load = OscillatorLoad { accel_m_s2 };
            let mut state = OscillatorState::try_rest(&oscillator).unwrap();
            let mut landed = 0u32;
            let mut max_residual = 0.0;
            loop {
                let held = state_bits(&state);
                let held_frame = frame.to_bytes();
                match step(&oscillator, &mut state, &mut frame, load, gate) {
                    Ok(()) => {
                        landed += 1;
                        assert!(landed <= expect_landed);
                        if state.residual_conservation_error > max_residual {
                            max_residual = state.residual_conservation_error;
                        }
                    }
                    Err(StepReject::AmplitudeAboveCard) => {
                        assert_eq!(state_bits(&state), held);
                        assert_eq!(frame.to_bytes(), held_frame);
                        break;
                    }
                    Err(other) => std::panic!("refused with {other:?} after {landed} steps"),
                }
            }
            assert_eq!(landed, expect_landed, "load {accel_m_s2}");
            // Measured 3.6e-9 at most. The limit is 1.0e-6.
            assert!(
                max_residual < 1.0e-8,
                "load {accel_m_s2} residual {max_residual}"
            );
            i += 1;
        }
        proof_intact(&frame);
    }

    /// A free ring released on the ceiling rings down inside the card. Its
    /// residual is the step method's own error and stays far inside the limit.
    #[test]
    fn a_free_ring_from_the_ceiling_stays_far_inside_the_limit() {
        let oscillator = DynamicOscillator::mount_10hz();
        let dt = oscillator.gate_dt_s().unwrap();
        let mut state =
            OscillatorState::try_phase(&oscillator, super::AMPLITUDE_CEILING_M, 0.0).unwrap();
        let mut frame = blank_frame();
        let mut max_residual = 0.0;
        let mut n = 0u32;
        while n < 20_000 {
            step(
                &oscillator,
                &mut state,
                &mut frame,
                OscillatorLoad::default(),
                dt,
            )
            .unwrap();
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            n += 1;
        }
        assert!(state.mechanical_energy_j < 1.0e-6 * oscillator.stroke_energy_j());
        // Measured 1.2e-8. The limit is 1.0e-6.
        assert!(
            max_residual > 1.0e-8 && max_residual < 1.3e-8,
            "residual {max_residual}"
        );
        proof_intact(&frame);
    }

    /// The stated reach of the ledger. A square-wave force of ±3 m/s² at
    /// 10 Hz rings the mount at 0.62 of its stroke. The ledger's defect grows
    /// in step with the damper heat, 2.92e-9 J per joule, and the limit is
    /// spent after 342 stroke energies of heat. Halving the step divides the
    /// defect by fifteen: the error is the step method's, not rounding.
    #[test]
    fn a_mount_driven_at_resonance_spends_the_ledger_on_step_method_error() {
        let oscillator = DynamicOscillator::mount_10hz();
        let dt = oscillator.gate_dt_s().unwrap();
        let stroke_j = oscillator.stroke_energy_j();
        let half_wave = 157u32;
        let mut state = OscillatorState::try_rest(&oscillator).unwrap();
        let mut frame = blank_frame();
        let mut landed = 0u32;
        let mut heat_j = 0.0;
        let mut peak_m: f64 = 0.0;
        let mut defect_at_100_000 = 0.0;
        let refusal = 'run: loop {
            let accel_m_s2 = if (landed / half_wave) % 2 == 0 {
                3.0
            } else {
                -3.0
            };
            let from_m = state.displacement_m;
            let work_before_j = state.boundary_work_j;
            let held = state_bits(&state);
            let held_frame = frame.to_bytes();
            match step(
                &oscillator,
                &mut state,
                &mut frame,
                OscillatorLoad { accel_m_s2 },
                dt,
            ) {
                Ok(()) => {
                    landed += 1;
                    assert!(landed < 500_000);
                    // Work of the held force less the booked work: the damper's share.
                    heat_j += MASS_KG * accel_m_s2 * (state.displacement_m - from_m)
                        - (state.boundary_work_j - work_before_j);
                    peak_m = peak_m.max(state.displacement_m.abs());
                    if landed == 100_000 || landed == 300_000 {
                        let per_joule = state.energy_defect_j / heat_j;
                        assert!(
                            (per_joule - 2.9214e-9).abs() < 0.01 * 2.9214e-9,
                            "defect per joule of damper heat {per_joule}"
                        );
                    }
                    if landed == 100_000 {
                        defect_at_100_000 = state.energy_defect_j;
                    }
                }
                Err(reject) => {
                    assert_eq!(state_bits(&state), held);
                    assert_eq!(frame.to_bytes(), held_frame);
                    break 'run reject;
                }
            }
        };
        assert_eq!(refusal, StepReject::SpontaneousGeneration);
        assert!(
            landed >= 447_816 && landed <= 447_832,
            "landed {landed} steps"
        );
        // Inside the stroke the whole way, and never flagged.
        assert!(peak_m > 0.61 * STROKE_M && peak_m < 0.63 * STROKE_M);
        assert!(!state.stroke_limited);
        assert!((heat_j / stroke_j - 342.3).abs() < 0.5, "heat {heat_j}");
        proof_intact(&frame);

        // The same drive at half the step, over the same 31.8 s.
        let mut fine = OscillatorState::try_rest(&oscillator).unwrap();
        let mut n = 0u32;
        while n < 200_000 {
            let accel_m_s2 = if (n / (2 * half_wave)) % 2 == 0 {
                3.0
            } else {
                -3.0
            };
            step(
                &oscillator,
                &mut fine,
                &mut frame,
                OscillatorLoad { accel_m_s2 },
                dt * 0.5,
            )
            .unwrap();
            n += 1;
        }
        let ratio = defect_at_100_000 / fine.energy_defect_j;
        assert!(ratio > 14.0 && ratio < 16.0, "ratio {ratio}");
    }

    /// The same drive with the work total kept to twice the precision: each
    /// step's work is taken from the card's own RK4 stages and added with an
    /// error-free sum. The ledger crosses the limit on the same step. The
    /// refusal after 447,824 steps is not rounding in the running sum.
    #[test]
    fn an_error_free_work_sum_does_not_move_the_refusing_step() {
        let oscillator = DynamicOscillator::mount_10hz();
        let dt = oscillator.gate_dt_s().unwrap();
        let gate_j = RESIDUAL_CONSERVATION_LIMIT * oscillator.stroke_energy_j();
        let mut displacement_m = 0.0;
        let mut velocity_m_s = 0.0;
        let mut work_j = 0.0;
        let mut work_lost_j = 0.0;
        let mut landed = 0u32;
        loop {
            let accel_m_s2 = if (landed / 157) % 2 == 0 { 3.0 } else { -3.0 };
            let next = super::rk4(
                &oscillator,
                [displacement_m, velocity_m_s, 0.0],
                dt,
                accel_m_s2,
            )
            .unwrap();
            // Two-sum: `sum` and `lost` together are exactly `work_j + next[2]`.
            let sum = work_j + next[2];
            let part = sum - work_j;
            let lost = (work_j - (sum - part)) + (next[2] - part);
            let energy_j = oscillator.mechanical_energy_j(next[0], next[1]);
            let defect_j = energy_j - sum - (work_lost_j + lost);
            if defect_j.abs() > gate_j {
                assert!(defect_j > 0.0);
                break;
            }
            displacement_m = next[0];
            velocity_m_s = next[1];
            work_j = sum;
            work_lost_j += lost;
            landed += 1;
            assert!(landed < 500_000);
        }
        assert!(
            landed >= 447_816 && landed <= 447_832,
            "landed {landed} steps"
        );
        // What the plain sum had dropped by then is 6.0e-14 J, a ten-millionth of the limit.
        assert!(work_lost_j.abs() < 1.0e-6 * gate_j, "lost {work_lost_j}");
    }
}

#[cfg(test)]
mod continuation_tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        DynamicOscillator, OscillatorLoad, OscillatorState, RESIDUAL_CONSERVATION_LIMIT,
        START_TIME_MAX_S, STROKE_M, StepReject, step,
    };
    use crate::frame::LastStateFrame64;

    fn frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: 0.0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            force_torque: 0.0,
            residual: 0.0,
            flags: 0,
            proof: [0; 16],
        }
    }

    fn bits(state: &OscillatorState) -> [u64; 12] {
        [
            state.t_s.to_bits(),
            state.displacement_m.to_bits(),
            state.velocity_m_s.to_bits(),
            state.kinetic_j.to_bits(),
            state.potential_j.to_bits(),
            state.mechanical_energy_j.to_bits(),
            state.energy_anchor_j.to_bits(),
            state.boundary_work_j.to_bits(),
            state.energy_defect_j.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.stroke_margin_m.to_bits(),
            u64::from(state.stroke_limited),
        ]
    }

    /// The landed state handed back with its own time and ledger.
    fn handed_back(oscillator: &DynamicOscillator, state: &OscillatorState) -> OscillatorState {
        OscillatorState::try_continue(
            oscillator,
            state.displacement_m,
            state.velocity_m_s,
            state.t_s,
            state.energy_anchor_j,
            state.boundary_work_j,
        )
        .unwrap()
    }

    fn square_wave(n: u32) -> f64 {
        if (n / 157) % 2 == 0 { 3.0 } else { -3.0 }
    }

    /// A new start from rest is the rest the card always had, and a new start
    /// off rest is `try_phase`.
    #[test]
    fn a_new_start_is_the_start_the_card_had() {
        let oscillator = DynamicOscillator::mount_10hz();
        let rest = OscillatorState::try_continue(&oscillator, 0.0, 0.0, 0.0, 0.0, 0.0).unwrap();
        assert_eq!(bits(&rest), bits(&OscillatorState::try_rest(&oscillator).unwrap()));
        let phase = OscillatorState::try_phase(&oscillator, 0.005, 0.1).unwrap();
        let start = OscillatorState::try_continue(
            &oscillator,
            0.005,
            0.1,
            0.0,
            phase.mechanical_energy_j,
            0.0,
        )
        .unwrap();
        assert_eq!(bits(&start), bits(&phase));
    }

    /// 1,000 steps from (0.005 m, 0.1 m/s) at +3 m/s², straight and cut
    /// after 400. The cut run ends with the same time, anchor, work,
    /// defect and residual as the straight one, bit for bit.
    #[test]
    fn a_run_cut_and_handed_back_ends_bit_for_bit_as_the_straight_run() {
        let oscillator = DynamicOscillator::mount_10hz();
        let dt = oscillator.gate_dt_s().unwrap();
        let load = OscillatorLoad { accel_m_s2: 3.0 };
        let mut f = frame();
        let mut straight = OscillatorState::try_phase(&oscillator, 0.005, 0.1).unwrap();
        let mut cut = straight;
        for _ in 0..1_000 {
            step(&oscillator, &mut straight, &mut f, load, dt).unwrap();
        }
        for _ in 0..400 {
            step(&oscillator, &mut cut, &mut f, load, dt).unwrap();
        }
        let mut cut = handed_back(&oscillator, &cut);
        for _ in 0..600 {
            step(&oscillator, &mut cut, &mut f, load, dt).unwrap();
        }
        assert_eq!(bits(&cut), bits(&straight));
        assert!(straight.t_s > 0.318 && straight.t_s < 0.319);
    }

    /// The reach of the ledger does not depend on where a run is cut: the
    /// resonant drive cut after 299,870 steps is refused on the same step as
    /// the straight run, 447,824 steps in, with the same defect.
    #[test]
    fn the_reach_of_the_ledger_holds_across_a_cut() {
        let oscillator = DynamicOscillator::mount_10hz();
        let dt = oscillator.gate_dt_s().unwrap();
        let mut f = frame();
        let mut state = OscillatorState::try_rest(&oscillator).unwrap();
        let mut landed = 0u32;
        let refusal = loop {
            if landed == 299_870 {
                state = handed_back(&oscillator, &state);
            }
            let load = OscillatorLoad {
                accel_m_s2: square_wave(landed),
            };
            match step(&oscillator, &mut state, &mut f, load, dt) {
                Ok(()) => landed += 1,
                Err(reject) => break reject,
            }
            assert!(landed < 500_000);
        };
        assert_eq!(refusal, StepReject::SpontaneousGeneration);
        assert!(landed >= 447_816 && landed <= 447_832, "landed {landed}");
        assert!(state.residual_conservation_error > 0.999 * RESIDUAL_CONSERVATION_LIMIT);
        assert!(state.t_s > 142.5 && state.t_s < 142.6, "t_s {}", state.t_s);
    }

    /// The published drive count at the stroke: a square wave of ±4.8 m/s²
    /// rings the mount at 0.99 of the stroke and spends the ledger after
    /// 175,815 steps.
    #[test]
    fn a_drive_ringing_the_mount_at_its_stroke_spends_the_ledger_after_175_815_steps() {
        let oscillator = DynamicOscillator::mount_10hz();
        let dt = oscillator.gate_dt_s().unwrap();
        let mut f = frame();
        let mut state = OscillatorState::try_rest(&oscillator).unwrap();
        let mut landed = 0u32;
        let mut peak_m: f64 = 0.0;
        let refusal = loop {
            let accel_m_s2 = if (landed / 157) % 2 == 0 { 4.8 } else { -4.8 };
            match step(&oscillator, &mut state, &mut f, OscillatorLoad { accel_m_s2 }, dt) {
                Ok(()) => {
                    landed += 1;
                    peak_m = peak_m.max(state.displacement_m.abs());
                }
                Err(reject) => break reject,
            }
            assert!(landed < 300_000);
        };
        assert_eq!(refusal, StepReject::SpontaneousGeneration);
        assert!(landed >= 175_807 && landed <= 175_823, "landed {landed}");
        assert!(peak_m > 0.985 * STROKE_M && peak_m < 0.995 * STROKE_M, "peak {}", peak_m / STROKE_M);
    }

    /// A stated ledger the card could not have written is refused by name.
    #[test]
    fn a_stated_ledger_the_card_could_not_have_written_is_refused_by_name() {
        let oscillator = DynamicOscillator::mount_10hz();
        let gate_j = RESIDUAL_CONSERVATION_LIMIT * oscillator.stroke_energy_j();
        let ceiling_j = oscillator.energy_ceiling_j();
        let at = |x: f64, v: f64, t: f64, anchor: f64, work: f64| {
            OscillatorState::try_continue(&oscillator, x, v, t, anchor, work)
        };
        // The clock: zero and 1024 s are inside, either side is not.
        assert!(at(0.0, 0.0, START_TIME_MAX_S, 0.0, 0.0).is_ok());
        assert_eq!(
            at(0.0, 0.0, START_TIME_MAX_S.next_up(), 0.0, 0.0),
            Err(StepReject::StartTimeOutsideCard)
        );
        assert_eq!(
            at(0.0, 0.0, -5.0e-324, 0.0, 0.0),
            Err(StepReject::StartTimeOutsideCard)
        );
        assert!(at(0.0, 0.0, -0.0, 0.0, 0.0).is_ok());
        // The anchor: an energy of a state on the card.
        assert!(at(0.0, 0.0, 1.0, ceiling_j, -ceiling_j).is_ok());
        assert_eq!(
            at(0.0, 0.0, 1.0, ceiling_j.next_up(), -ceiling_j),
            Err(StepReject::LedgerNotOfThisState)
        );
        assert_eq!(
            at(0.0, 0.0, 1.0, -5.0e-324, 0.0),
            Err(StepReject::LedgerNotOfThisState)
        );
        // No work is booked before the first step.
        assert_eq!(
            at(0.0, 0.0, 0.0, 0.0, 1.0e-300),
            Err(StepReject::LedgerNotOfThisState)
        );
        assert!(at(0.0, 0.0, 1.0e-3, 0.0, 0.5 * gate_j).is_ok());
        // A defect already past the limit, either sign; one inside it is kept.
        let x = 0.005;
        let e = oscillator.mechanical_energy_j(x, 0.0);
        let inside = at(x, 0.0, 1.0, e - 0.999 * gate_j, 0.0).unwrap();
        assert!(inside.energy_defect_j > 0.998 * gate_j && inside.energy_defect_j <= gate_j);
        assert_eq!(
            at(x, 0.0, 1.0, e - 2.0 * gate_j, 0.0),
            Err(StepReject::LedgerNotOfThisState)
        );
        assert_eq!(
            at(x, 0.0, 1.0, e, 2.0 * gate_j),
            Err(StepReject::LedgerNotOfThisState)
        );
        // A new start whose anchor misses its energy by more than the limit.
        assert_eq!(
            at(x, 0.0, 0.0, e + 2.0 * gate_j, 0.0),
            Err(StepReject::LedgerNotOfThisState)
        );
        // A state above the card is refused for its energy before its ledger is read.
        let ceiling_m = super::AMPLITUDE_CEILING_M;
        assert!(at(ceiling_m, 0.0, 0.0, ceiling_j, 0.0).is_ok());
        let over_m = ceiling_m.next_up();
        assert_eq!(
            at(over_m, 0.0, 0.0, oscillator.mechanical_energy_j(over_m, 0.0), 0.0),
            Err(StepReject::AmplitudeAboveCard)
        );
        // Not finite, and not this mount.
        assert_eq!(
            at(0.0, 0.0, f64::NAN, 0.0, 0.0),
            Err(StepReject::NonFiniteState)
        );
        assert_eq!(
            at(0.0, 0.0, 0.0, 0.0, f64::INFINITY),
            Err(StepReject::NonFiniteState)
        );
        let mut tampered = oscillator;
        tampered.zeta = 0.2;
        assert_eq!(
            OscillatorState::try_continue(&tampered, 0.0, 0.0, 0.0, 0.0, 0.0),
            Err(StepReject::InvalidOscillator)
        );
    }

    /// At the latest clock a run can reach from the latest start, a floor
    /// step still moves the clock by the floor step to within 4e-4 of it.
    #[test]
    fn the_clock_moves_by_a_floor_step_at_the_latest_time_a_run_reaches() {
        let oscillator = DynamicOscillator::mount_10hz();
        let floor_s = oscillator.floor_dt_s().unwrap();
        let late_s = START_TIME_MAX_S + 1.0e6 * oscillator.gate_dt_s().unwrap();
        assert!(late_s < 2048.0);
        let moved_s = (late_s + floor_s) - late_s;
        assert!((moved_s - floor_s).abs() <= 4.0e-4 * floor_s, "moved {moved_s}");
    }
}

#[cfg(test)]
mod boundary_tests {
    /// The last load accepted and the first refused, at several step sizes,
    /// agree with the published rule `|accel_m_s2| * dt_s <= omega * stroke`.
    #[test]
    fn the_load_rule_is_the_published_product_compared_with_the_constant() {
        let oscillator = super::DynamicOscillator::mount_10hz();
        let limit = oscillator.omega_rad_s * oscillator.stroke_m;
        crate::impulse::tests::assert_boundary(limit, |load, dt_s| super::exceeds_force_nyquist(&oscillator, load, dt_s));
    }
}
