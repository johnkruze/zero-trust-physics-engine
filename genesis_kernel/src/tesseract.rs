#![no_implicit_prelude]

//! Drive-mode Duffing oscillator on one locked proof mass.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, displacement, velocity, the envelope, kinetic
//! energy, potential energy, the held acceleration, the Coriolis scale, and
//! the conservation residual into [`LastStateFrame64`]. The 16-byte frame
//! proof is not touched.
//!
//! The card is an 80 Hz drive mode, damping ratio 1/80, cubic coefficient
//! 6×10⁸ m⁻² s⁻², proof mass 10⁻⁷ kg, amplitude limit 1/128 m. `α`, `ζ`, and
//! `ω` are crate constants. A caller does not pass them. `dt` is not raised
//! to a floor.
//!
//! ```text
//! ẍ + 2 ζ ω ẋ + ω² x + α x³ = a
//! E = ½ m v² + ½ m ω² x² + ¼ m α x⁴
//! dW/dt = m a v − c v²
//! c = 2 ζ ω m
//! residual_conservation_error = |E − E_anchor − W| / E_limit
//! ```
//!
//! `E_limit` is that potential at the amplitude limit. The cubic force is
//! conservative, so it stays inside `E`. Damper heat and the drive's work
//! are the boundary. The sense-axis rate `Ω` and the sense speed set the
//! Coriolis scale `2 m |v_sense| |Ω|`. They do not enter the drive ODE.
//!
//! The amplitude limit is a limit of the card, not the amplitude where the
//! response folds. Under one sinusoidal drive the averaged response of this
//! resonator has two coexisting branches from an amplitude of
//! `sqrt(16 ζ ω² / (3 √3 α))` = 4.026 mm, the cusp of its averaged frequency
//! response, so both branches lie inside the limit.
//!
//! The card holds the displacement to `|x| ≤ L` and the velocity to
//! `|v| ≤ V`, both inclusive, with `L` the amplitude limit and `V` the speed
//! at zero displacement whose kinetic energy is `E_limit`. A state outside
//! either bound, incoming or landed by a trial step, is
//! [`StepReject::AmplitudeOutsideCard`].
//!
//! The ledger belongs to the run. [`TesseractState::try_continue`] starts a
//! state with the signed defect `E − E_anchor − W` that a run carried to it,
//! so a run cut in two and started again from its returned state with its
//! `energy_defect_j` is held to the same limit as the run straight through.
//!
//! The public step is classical RK4 on `(x, v, W)`. There is no
//! `dt.max(1e-12)` and no finite-difference residual. A non-finite trial,
//! including an intermediate stage, is [`StepReject::NonFiniteState`]. The
//! state and the frame stay as they were. [`Tesseract::gate_dt_s`] is
//! `ω Δt = 0.02` and is not applied. The force Nyquist limit is applied:
//! `|a| dt ≤ ω L`.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Drive-mode natural frequency.
pub const NATURAL_FREQUENCY_HZ: f64 = 80.0;
/// Damping ratio of the drive mode.
pub const DAMPING_RATIO: f64 = 1.0 / 80.0;
/// Hardening coefficient, m⁻² s⁻². Positive is spring-hardening.
pub const DUFFING_ALPHA_PER_M2_S2: f64 = 6.0e8;
/// Proof mass.
pub const MASS_KG: f64 = 1.0e-7;
/// Largest displacement the card answers, either sign, inclusive. It is not
/// where the two branches of the driven response part: they coexist from
/// 4.026 mm.
pub const AMPLITUDE_LIMIT_M: f64 = 1.0 / 128.0;
/// Linear-plateau sense speed used when the load does not name one.
pub const SENSE_PLATEAU_M_S: f64 = 1.0;

/// `|E − E_anchor − W| / E_limit` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Phase advance `ω Δt` used by [`Tesseract::gate_dt_s`].
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Why a step did not advance the proof mass or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Acceleration, sense speed, rate, or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or a trial stage, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// `α`, `ζ`, `ω`, mass, or the amplitude limit is not this drive mode.
    InvalidResonator,
    /// `|a| dt` is past `ω L`.
    ForceAboveNyquist,
    /// The displacement or the velocity, incoming or after the trial step, is outside the card's inclusive bounds.
    AmplitudeOutsideCard,
    /// The trial step created mechanical energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed mechanical energy.
    FirstLawBreach,
}

/// Zero-order hold. Sense speed and `Ω` set the scale factor only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TesseractLoad {
    pub accel_m_s2: f64,
    pub omega_ext_rad_s: f64,
    pub sense_velocity_m_s: f64,
}

impl Default for TesseractLoad {
    fn default() -> Self {
        Self {
            accel_m_s2: 0.0,
            omega_ext_rad_s: 0.0,
            sense_velocity_m_s: SENSE_PLATEAU_M_S,
        }
    }
}

/// Locked drive mode. `α`, `ζ`, and `ω` are recorded copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tesseract {
    pub omega_rad_s: f64,
    pub zeta: f64,
    pub alpha_per_m2_s2: f64,
    pub mass_kg: f64,
    pub amplitude_limit_m: f64,
}

impl Tesseract {
    /// Card: 80 Hz hardening drive, ζ = 1/80, α = 6×10⁸.
    #[must_use]
    pub fn drive_mode() -> Self {
        let resonator = Self {
            omega_rad_s: natural_frequency_rad_s(),
            zeta: DAMPING_RATIO,
            alpha_per_m2_s2: DUFFING_ALPHA_PER_M2_S2,
            mass_kg: MASS_KG,
            amplitude_limit_m: AMPLITUDE_LIMIT_M,
        };
        if resonator.is_valid() {
            resonator
        } else {
            ::core::panic!("drive mode failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.omega_rad_s.to_bits() == natural_frequency_rad_s().to_bits()
            && self.zeta.to_bits() == DAMPING_RATIO.to_bits()
            && self.alpha_per_m2_s2.to_bits() == DUFFING_ALPHA_PER_M2_S2.to_bits()
            && self.mass_kg.to_bits() == MASS_KG.to_bits()
            && self.amplitude_limit_m.to_bits() == AMPLITUDE_LIMIT_M.to_bits()
            && self.omega_rad_s.is_finite()
            && self.omega_rad_s > 0.0
            && self.zeta >= 0.0
            && self.zeta < 1.0
            && self.alpha_per_m2_s2 > 0.0
            && self.mass_kg > 0.0
            && self.amplitude_limit_m > 0.0
            && self.limit_energy_j().is_finite()
            && self.limit_energy_j() > 0.0
    }

    #[must_use]
    pub fn damping_n_s_per_m(&self) -> f64 {
        2.0 * self.zeta * self.omega_rad_s * self.mass_kg
    }

    /// Harmonic plus quartic energy at [`AMPLITUDE_LIMIT_M`].
    #[must_use]
    pub fn limit_energy_j(&self) -> f64 {
        self.potential_j(self.amplitude_limit_m)
    }

    /// Largest speed the card answers, either sign, inclusive: the speed at
    /// zero displacement whose kinetic energy is [`Self::limit_energy_j`].
    #[must_use]
    pub fn speed_limit_m_s(&self) -> f64 {
        (2.0 * self.limit_energy_j() / self.mass_kg).sqrt()
    }

    /// True when the displacement and the velocity are inside the card's
    /// inclusive bounds.
    #[must_use]
    pub fn holds(&self, displacement_m: f64, velocity_m_s: f64) -> bool {
        let speed_limit_m_s = self.speed_limit_m_s();
        displacement_m >= -self.amplitude_limit_m
            && displacement_m <= self.amplitude_limit_m
            && velocity_m_s >= -speed_limit_m_s
            && velocity_m_s <= speed_limit_m_s
    }

    /// Largest `|E − E_anchor − W|` a step may land with: the residual limit
    /// times [`Self::limit_energy_j`].
    #[must_use]
    pub fn defect_limit_j(&self) -> f64 {
        RESIDUAL_CONSERVATION_LIMIT * self.limit_energy_j()
    }

    #[must_use]
    pub fn potential_j(&self, displacement_m: f64) -> f64 {
        let x2 = displacement_m * displacement_m;
        0.5 * self.mass_kg * self.omega_rad_s * self.omega_rad_s * x2
            + 0.25 * self.mass_kg * self.alpha_per_m2_s2 * x2 * x2
    }

    #[must_use]
    pub fn mechanical_energy_j(&self, displacement_m: f64, velocity_m_s: f64) -> f64 {
        0.5 * self.mass_kg * velocity_m_s * velocity_m_s + self.potential_j(displacement_m)
    }

    /// Largest reporting step of the drive mode. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.omega_rad_s.is_finite() && self.omega_rad_s > 0.0 {
            Some(GATE_PHASE_STEP_RAD / self.omega_rad_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step turns the drive mode by two hundredths of a radian. A million times smaller, a step
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

    /// `π / ω`. Not applied by [`step`].
    #[must_use]
    pub fn nyquist_dt_s(&self) -> Option<f64> {
        if self.omega_rad_s.is_finite() && self.omega_rad_s > 0.0 {
            Some(core::f64::consts::PI / self.omega_rad_s)
        } else {
            None
        }
    }

    /// `omega * amplitude limit`, in metres per second: the constant of the load rule
    /// `|a| * dt_s <= C`.
    #[must_use]
    pub fn accel_impulse_limit_m_s(&self) -> f64 {
        self.omega_rad_s * self.amplitude_limit_m
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

    /// `|F_c| = 2 m |v_sense| |Ω|`.
    #[must_use]
    pub fn coriolis_scale_n(&self, sense_velocity_m_s: f64, omega_ext_rad_s: f64) -> f64 {
        2.0 * self.mass_kg * sense_velocity_m_s.abs() * omega_ext_rad_s.abs()
    }

    #[must_use]
    pub fn envelope_m(&self, displacement_m: f64, velocity_m_s: f64) -> f64 {
        crate::math::hypot(displacement_m, velocity_m_s / self.omega_rad_s)
    }
}

fn natural_frequency_rad_s() -> f64 {
    NATURAL_FREQUENCY_HZ * 2.0 * core::f64::consts::PI
}

/// Integrated proof mass. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TesseractState {
    pub t_s: f64,
    pub displacement_m: f64,
    pub velocity_m_s: f64,
    pub accel_m_s2: f64,
    pub envelope_m: f64,
    pub kinetic_j: f64,
    pub potential_j: f64,
    pub mechanical_energy_j: f64,
    pub energy_anchor_j: f64,
    pub boundary_work_j: f64,
    /// Signed `E − E_anchor − W`. Positive is created energy.
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    pub scale_factor_n: f64,
}

impl TesseractState {
    #[must_use]
    pub fn try_rest(resonator: &Tesseract) -> Option<Self> {
        Self::try_phase(resonator, 0.0, 0.0)
    }

    /// Start a run at this drive-mode displacement and velocity with an
    /// empty ledger. The same as [`Self::try_continue`] with a defect of zero.
    #[must_use]
    pub fn try_phase(
        resonator: &Tesseract,
        displacement_m: f64,
        velocity_m_s: f64,
    ) -> Option<Self> {
        Self::try_continue(resonator, displacement_m, velocity_m_s, 0.0)
    }

    /// Start at this displacement and velocity carrying the signed defect
    /// `E − E_anchor − W` of the run that reached it. The anchor is the
    /// present energy less that defect and the booked work is zero, so the
    /// next step is held to the error summed over the whole run. A state
    /// outside the card's bounds, or a defect whose size is past
    /// [`Tesseract::defect_limit_j`], is not a start.
    #[must_use]
    pub fn try_continue(
        resonator: &Tesseract,
        displacement_m: f64,
        velocity_m_s: f64,
        energy_defect_j: f64,
    ) -> Option<Self> {
        if !resonator.is_valid()
            || !displacement_m.is_finite()
            || !velocity_m_s.is_finite()
            || !energy_defect_j.is_finite()
            || !resonator.holds(displacement_m, velocity_m_s)
            || !(energy_defect_j.abs() <= resonator.defect_limit_j())
        {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            displacement_m,
            velocity_m_s,
            accel_m_s2: 0.0,
            envelope_m: 0.0,
            kinetic_j: 0.0,
            potential_j: 0.0,
            mechanical_energy_j: 0.0,
            energy_anchor_j: 0.0,
            boundary_work_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            scale_factor_n: 0.0,
        };
        refresh(resonator, &mut state, TesseractLoad::default());
        if !state_is_finite(&state) {
            return None;
        }
        state.energy_anchor_j = state.mechanical_energy_j - energy_defect_j;
        state.boundary_work_j = 0.0;
        state.energy_defect_j = state.mechanical_energy_j - state.energy_anchor_j;
        state.residual_conservation_error =
            state.energy_defect_j.abs() / resonator.limit_energy_j();
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }
}

/// Advance one RK4 step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were. A non-finite
/// RK4 stage is [`StepReject::NonFiniteState`].
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    resonator: &Tesseract,
    state: &mut TesseractState,
    frame: &mut LastStateFrame64,
    load: TesseractLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(resonator, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    resonator: &Tesseract,
    state: &mut TesseractState,
    frame: &mut LastStateFrame64,
    load: TesseractLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(resonator, state, load, dt_s)?;
    let predicted = match rk4(resonator, pack(state), dt_s, load.accel_m_s2) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(resonator, state, frame, load, dt_s, predicted)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut TesseractState) {
    state.t_s = normal_or_zero(state.t_s);
    state.displacement_m = normal_or_zero(state.displacement_m);
    state.velocity_m_s = normal_or_zero(state.velocity_m_s);
    state.accel_m_s2 = normal_or_zero(state.accel_m_s2);
    state.envelope_m = normal_or_zero(state.envelope_m);
    state.kinetic_j = normal_or_zero(state.kinetic_j);
    state.potential_j = normal_or_zero(state.potential_j);
    state.mechanical_energy_j = normal_or_zero(state.mechanical_energy_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.boundary_work_j = normal_or_zero(state.boundary_work_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.scale_factor_n = normal_or_zero(state.scale_factor_n);
}

fn validate(
    resonator: &Tesseract,
    state: &TesseractState,
    load: TesseractLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite()
        || !load.accel_m_s2.is_finite()
        || !load.omega_ext_rad_s.is_finite()
        || !load.sense_velocity_m_s.is_finite()
    {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !resonator.is_valid() {
        return Err(StepReject::InvalidResonator);
    }
    if exceeds_force_nyquist(resonator, load.accel_m_s2, dt_s) {
        return Err(StepReject::ForceAboveNyquist);
    }
    if !resonator.holds(state.displacement_m, state.velocity_m_s) {
        return Err(StepReject::AmplitudeOutsideCard);
    }
    Ok(())
}

fn exceeds_force_nyquist(resonator: &Tesseract, accel_m_s2: f64, dt_s: f64) -> bool {
    if accel_m_s2 == 0.0 {
        return false;
    }
    match resonator.force_nyquist_m_s2(dt_s) {
        Some(_) => !crate::impulse::within(accel_m_s2, dt_s, resonator.accel_impulse_limit_m_s()),
        None => true,
    }
}

fn commit(
    resonator: &Tesseract,
    state: &mut TesseractState,
    frame: &mut LastStateFrame64,
    load: TesseractLoad,
    dt_s: f64,
    predicted: [f64; 3],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite3(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    if !resonator.holds(predicted[0], predicted[1]) {
        return Err(StepReject::AmplitudeOutsideCard);
    }
    state.displacement_m = predicted[0];
    state.velocity_m_s = predicted[1];
    state.boundary_work_j = predicted[2];
    state.accel_m_s2 = load.accel_m_s2;
    state.t_s += dt_s;
    refresh(resonator, state, load);
    let scale = resonator.limit_energy_j();
    if !(scale > 0.0 && scale.is_finite()) {
        *state = saved;
        return Err(StepReject::InvalidResonator);
    }
    state.energy_defect_j =
        state.mechanical_energy_j - state.energy_anchor_j - state.boundary_work_j;
    state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
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

fn refresh(resonator: &Tesseract, state: &mut TesseractState, load: TesseractLoad) {
    state.kinetic_j = 0.5 * resonator.mass_kg * state.velocity_m_s * state.velocity_m_s;
    state.potential_j = resonator.potential_j(state.displacement_m);
    state.mechanical_energy_j = state.kinetic_j + state.potential_j;
    state.envelope_m = resonator.envelope_m(state.displacement_m, state.velocity_m_s);
    state.scale_factor_n =
        resonator.coriolis_scale_n(load.sense_velocity_m_s, load.omega_ext_rad_s);
}

fn publish(state: &TesseractState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.displacement_m as f32;
    frame.pos[1] = state.velocity_m_s as f32;
    frame.pos[2] = state.envelope_m as f32;
    frame.vel[0] = state.kinetic_j as f32;
    frame.vel[1] = state.potential_j as f32;
    frame.vel[2] = state.accel_m_s2 as f32;
    frame.force_torque = state.scale_factor_n as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = 0;
}

fn derivatives(resonator: &Tesseract, y: [f64; 3], accel_m_s2: f64) -> Option<[f64; 3]> {
    let displacement_m = y[0];
    let velocity_m_s = y[1];
    let omega = resonator.omega_rad_s;
    let cubic = resonator.alpha_per_m2_s2 * displacement_m * displacement_m * displacement_m;
    let acceleration = accel_m_s2
        - 2.0 * resonator.zeta * omega * velocity_m_s
        - omega * omega * displacement_m
        - cubic;
    let power_w = resonator.mass_kg * accel_m_s2 * velocity_m_s
        - resonator.damping_n_s_per_m() * velocity_m_s * velocity_m_s;
    let out = [velocity_m_s, acceleration, power_w];
    if finite3(out) { Some(out) } else { None }
}

fn rk4(resonator: &Tesseract, y: [f64; 3], dt: f64, accel_m_s2: f64) -> Option<[f64; 3]> {
    let k1 = derivatives(resonator, y, accel_m_s2)?;
    let k2 = derivatives(resonator, axpy(y, dt * 0.5, k1), accel_m_s2)?;
    let k3 = derivatives(resonator, axpy(y, dt * 0.5, k2), accel_m_s2)?;
    let k4 = derivatives(resonator, axpy(y, dt, k3), accel_m_s2)?;
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

fn pack(state: &TesseractState) -> [f64; 3] {
    [
        state.displacement_m,
        state.velocity_m_s,
        state.boundary_work_j,
    ]
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn state_is_finite(state: &TesseractState) -> bool {
    state.t_s.is_finite()
        && state.displacement_m.is_finite()
        && state.velocity_m_s.is_finite()
        && state.accel_m_s2.is_finite()
        && state.envelope_m.is_finite()
        && state.kinetic_j.is_finite()
        && state.potential_j.is_finite()
        && state.mechanical_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.boundary_work_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.scale_factor_n.is_finite()
}

/// Explicit Euler on `(x, v, W)`.
#[cfg(test)]
fn step_euler(
    resonator: &Tesseract,
    state: &mut TesseractState,
    frame: &mut LastStateFrame64,
    load: TesseractLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(resonator, state, load, dt_s)?;
    let predicted = match explicit_euler(resonator, pack(state), dt_s, load.accel_m_s2) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(resonator, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
fn explicit_euler(
    resonator: &Tesseract,
    y: [f64; 3],
    dt: f64,
    accel_m_s2: f64,
) -> Option<[f64; 3]> {
    let k1 = derivatives(resonator, y, accel_m_s2)?;
    let next = axpy(y, dt, k1);
    if finite3(next) { Some(next) } else { None }
}

/// Add `extra_j` to the integrated work after a faithful RK4 step.
#[cfg(test)]
fn step_bias_work(
    resonator: &Tesseract,
    state: &mut TesseractState,
    frame: &mut LastStateFrame64,
    load: TesseractLoad,
    dt_s: f64,
    extra_j: f64,
) -> Result<(), StepReject> {
    validate(resonator, state, load, dt_s)?;
    let mut predicted = match rk4(resonator, pack(state), dt_s, load.accel_m_s2) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    predicted[2] += extra_j;
    commit(resonator, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        AMPLITUDE_LIMIT_M, DAMPING_RATIO, DUFFING_ALPHA_PER_M2_S2, MASS_KG,
        NATURAL_FREQUENCY_HZ, RESIDUAL_CONSERVATION_LIMIT, StepReject, Tesseract, TesseractLoad,
        TesseractState, natural_frequency_rad_s, step, step_bias_work, step_euler,
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

    fn state_bits(state: &TesseractState) -> [u64; 13] {
        [
            state.t_s.to_bits(),
            state.displacement_m.to_bits(),
            state.velocity_m_s.to_bits(),
            state.accel_m_s2.to_bits(),
            state.envelope_m.to_bits(),
            state.kinetic_j.to_bits(),
            state.potential_j.to_bits(),
            state.mechanical_energy_j.to_bits(),
            state.energy_anchor_j.to_bits(),
            state.boundary_work_j.to_bits(),
            state.energy_defect_j.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.scale_factor_n.to_bits(),
        ]
    }

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
    fn card_locks_alpha_zeta_and_omega() {
        assert_copy::<Tesseract>();
        assert_copy::<TesseractState>();
        assert_copy::<TesseractLoad>();
        assert_copy::<StepReject>();
        let resonator = Tesseract::drive_mode();
        assert_eq!(
            resonator.omega_rad_s.to_bits(),
            natural_frequency_rad_s().to_bits()
        );
        assert_eq!(resonator.zeta.to_bits(), DAMPING_RATIO.to_bits());
        assert_eq!(
            resonator.alpha_per_m2_s2.to_bits(),
            DUFFING_ALPHA_PER_M2_S2.to_bits()
        );
        assert_eq!(NATURAL_FREQUENCY_HZ, 80.0);
        assert_eq!(MASS_KG, 1.0e-7);
        let mut tampered = resonator;
        tampered.alpha_per_m2_s2 = -1.0;
        assert!(!tampered.is_valid());
        tampered.alpha_per_m2_s2 = 0.0;
        assert!(!tampered.is_valid());
        let gate = resonator.gate_dt_s().unwrap();
        assert!(resonator.nyquist_dt_s().unwrap() > gate);
    }

    #[test]
    fn small_amplitude_tracks_the_linear_clock_and_keeps_the_ledger() {
        let resonator = Tesseract::drive_mode();
        let x0 = 1.0e-5;
        let mut state = TesseractState::try_phase(&resonator, x0, 0.0).unwrap();
        let mut frame = blank_frame();
        let dt = resonator.gate_dt_s().unwrap();
        let mut max_residual = 0.0;
        let mut n = 0u32;
        while n < 3_141 {
            step(
                &resonator,
                &mut state,
                &mut frame,
                TesseractLoad::default(),
                dt,
            )
            .unwrap();
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            n += 1;
        }
        let (x, v) = underdamped(x0, 0.0, resonator.omega_rad_s, resonator.zeta, state.t_s);
        assert!((state.displacement_m - x).abs() < 1.0e-9);
        assert!((state.velocity_m_s - v).abs() < 1.0e-6);
        assert!(
            max_residual <= RESIDUAL_CONSERVATION_LIMIT,
            "residual {max_residual}"
        );
        assert_eq!(frame.flags, 0);
        proof_intact(&frame);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
    }

    #[test]
    fn tiny_dt_is_not_clamped() {
        let resonator = Tesseract::drive_mode();
        let x0 = 1.0e-3;
        let mut state = TesseractState::try_phase(&resonator, x0, 0.0).unwrap();
        let mut frame = blank_frame();
        let dt = 1.0e-15;
        step(
            &resonator,
            &mut state,
            &mut frame,
            TesseractLoad::default(),
            dt,
        )
        .unwrap();
        let accel = -(resonator.omega_rad_s * resonator.omega_rad_s) * x0
            - resonator.alpha_per_m2_s2 * x0 * x0 * x0;
        let clamped = accel.abs() * 1.0e-12;
        assert!(
            state.velocity_m_s.abs() < clamped * 0.1,
            "v {}",
            state.velocity_m_s
        );
        assert!(state.velocity_m_s.abs() > 0.0);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    #[test]
    fn coriolis_scale_does_not_move_the_well() {
        let resonator = Tesseract::drive_mode();
        let mut quiet = TesseractState::try_phase(&resonator, 1.0e-4, 0.0).unwrap();
        let mut sensed = quiet;
        let mut frame_q = blank_frame();
        let mut frame_s = blank_frame();
        let dt = resonator.gate_dt_s().unwrap();
        step(
            &resonator,
            &mut quiet,
            &mut frame_q,
            TesseractLoad::default(),
            dt,
        )
        .unwrap();
        let load = TesseractLoad {
            accel_m_s2: 0.0,
            omega_ext_rad_s: 0.4,
            sense_velocity_m_s: 70.0,
        };
        step(&resonator, &mut sensed, &mut frame_s, load, dt).unwrap();
        assert_eq!(
            quiet.displacement_m.to_bits(),
            sensed.displacement_m.to_bits()
        );
        assert_eq!(quiet.velocity_m_s.to_bits(), sensed.velocity_m_s.to_bits());
        let expected = resonator.coriolis_scale_n(70.0, 0.4);
        assert_eq!(sensed.scale_factor_n.to_bits(), expected.to_bits());
        assert_eq!(frame_s.force_torque, expected as f32);
        assert!(quiet.scale_factor_n == 0.0);
        proof_intact(&frame_s);
    }

    #[test]
    fn the_amplitude_limit_lands_and_euler_from_it_creates_energy() {
        let resonator = Tesseract::drive_mode();
        let mut state = TesseractState::try_phase(&resonator, AMPLITUDE_LIMIT_M, 0.0).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step_euler(
            &resonator,
            &mut state,
            &mut frame,
            TesseractLoad::default(),
            resonator.gate_dt_s().unwrap(),
        );
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        proof_intact(&frame);

        step(
            &resonator,
            &mut state,
            &mut frame,
            TesseractLoad::default(),
            resonator.gate_dt_s().unwrap(),
        )
        .unwrap();
        assert!(state.displacement_m < AMPLITUDE_LIMIT_M);
        assert_eq!(frame.flags, 0);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    #[test]
    fn non_finite_trials_and_force_nyquist_roll_back() {
        let resonator = Tesseract::drive_mode();
        let mut state = TesseractState::try_rest(&resonator).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let dt = resonator.gate_dt_s().unwrap();
        let extra = resonator.limit_energy_j() * 1.0e-3;
        let got = step_bias_work(
            &resonator,
            &mut state,
            &mut frame,
            TesseractLoad::default(),
            dt,
            extra,
        );
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state, saved);
        let got = step_bias_work(
            &resonator,
            &mut state,
            &mut frame,
            TesseractLoad::default(),
            dt,
            -extra,
        );
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);

        let limit = resonator.force_nyquist_m_s2(dt).unwrap();
        let hot = TesseractLoad {
            accel_m_s2: limit + 1.0,
            omega_ext_rad_s: 0.0,
            sense_velocity_m_s: 1.0,
        };
        let got = step(&resonator, &mut state, &mut frame, hot, dt);
        assert_eq!(got, Err(StepReject::ForceAboveNyquist));
        assert_eq!(state, saved);

        state.displacement_m = 1.0e150;
        let got = step(
            &resonator,
            &mut state,
            &mut frame,
            TesseractLoad::default(),
            dt,
        );
        assert_eq!(got, Err(StepReject::AmplitudeOutsideCard));
        assert_eq!(state.displacement_m, 1.0e150);
        proof_intact(&frame);

        state = saved;
        let mut refuse = |load: TesseractLoad, dt_s: f64, expect: StepReject| {
            let got = step(&resonator, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state, saved);
            proof_intact(&frame);
        };
        refuse(TesseractLoad::default(), 0.0, StepReject::NonPositiveDt);
        refuse(TesseractLoad::default(), -dt, StepReject::NonPositiveDt);
        refuse(
            TesseractLoad::default(),
            f64::NAN,
            StepReject::NonFiniteInput,
        );
        refuse(
            TesseractLoad {
                accel_m_s2: f64::NAN,
                omega_ext_rad_s: 0.0,
                sense_velocity_m_s: 1.0,
            },
            dt,
            StepReject::NonFiniteInput,
        );
        refuse(
            TesseractLoad {
                accel_m_s2: 0.0,
                omega_ext_rad_s: f64::INFINITY,
                sense_velocity_m_s: 1.0,
            },
            dt,
            StepReject::NonFiniteInput,
        );
        state.velocity_m_s = f64::MAX;
        let got = step(
            &resonator,
            &mut state,
            &mut frame,
            TesseractLoad::default(),
            dt,
        );
        assert_eq!(got, Err(StepReject::AmplitudeOutsideCard));
        state.velocity_m_s = f64::INFINITY;
        let got = step(
            &resonator,
            &mut state,
            &mut frame,
            TesseractLoad::default(),
            dt,
        );
        assert_eq!(got, Err(StepReject::NonFiniteState));
        proof_intact(&frame);
    }

    #[test]
    fn biased_work_on_a_driven_mass_rolls_back_bit_for_bit() {
        let resonator = Tesseract::drive_mode();
        let dt = resonator.gate_dt_s().unwrap();
        let load = TesseractLoad {
            accel_m_s2: resonator.force_nyquist_m_s2(dt).unwrap() * 1.0e-3,
            omega_ext_rad_s: 0.4,
            sense_velocity_m_s: 70.0,
        };
        let mut state = TesseractState::try_phase(&resonator, AMPLITUDE_LIMIT_M * 0.25, 0.0).unwrap();
        let mut frame = blank_frame();
        let mut n = 0u32;
        while n < 100 {
            step(&resonator, &mut state, &mut frame, load, dt).unwrap();
            n += 1;
        }
        assert!(state.boundary_work_j != 0.0);
        assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT * 0.01);
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();

        // Twice the gate. The faithful residual above is under a hundredth of it.
        let extra = resonator.limit_energy_j() * RESIDUAL_CONSERVATION_LIMIT * 2.0;
        let got = step_bias_work(&resonator, &mut state, &mut frame, load, dt, extra);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        let got = step_bias_work(&resonator, &mut state, &mut frame, load, dt, -extra);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        step(&resonator, &mut state, &mut frame, load, dt).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::Tesseract::drive_mode();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The drive mode 0.1 mm out, moving at 10 mm/s, under 1 m/s².
        let mut state = super::TesseractState::try_phase(&card, 1.0e-4, 1.0e-2).unwrap();
        let load = super::TesseractLoad {
            accel_m_s2: 1.0,
            omega_ext_rad_s: 0.1,
            sense_velocity_m_s: super::SENSE_PLATEAU_M_S,
        };
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
}

#[cfg(test)]
mod boundary_tests {
    /// The last load accepted and the first refused, at several step sizes,
    /// agree with the published rule `|accel_m_s2| * dt_s <= omega * L`.
    #[test]
    fn the_load_rule_is_the_published_product_compared_with_the_constant() {
        let resonator = super::Tesseract::drive_mode();
        let limit = resonator.omega_rad_s * resonator.amplitude_limit_m;
        crate::impulse::tests::assert_boundary(limit, |load, dt_s| super::exceeds_force_nyquist(&resonator, load, dt_s));
    }
}

#[cfg(test)]
mod ledger_continuation_tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{StepReject, Tesseract, TesseractLoad, TesseractState, step};
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

    /// A sine drive of `peak_m_s2` at `hz`, a fresh drive value at every step
    /// at the middle of the step, on the run's own clock. The sine is the
    /// crate's own, so the step a run is refused on does not move with the
    /// platform's libm.
    fn sine_load(peak_m_s2: f64, hz: f64, step_index: u64, dt_s: f64) -> TesseractLoad {
        let t_mid_s = (step_index as f64 + 0.5) * dt_s;
        TesseractLoad {
            accel_m_s2: peak_m_s2 * crate::math::sin(2.0 * core::f64::consts::PI * hz * t_mid_s),
            omega_ext_rad_s: 0.0,
            sense_velocity_m_s: 1.0,
        }
    }

    /// Steps the high branch at 83.3 Hz, 46 m/s² peak, until `limit` steps
    /// or a refusal, counting the drive's clock from `first_index`. Returns
    /// the steps landed and the refusal.
    fn drive(
        card: &Tesseract,
        state: &mut TesseractState,
        first_index: u64,
        limit: u64,
    ) -> (u64, Option<StepReject>) {
        drive_sine(card, state, 46.0, 83.3, first_index, limit, &mut [0.0, 0.0])
    }

    /// Steps a sine drive until `limit` steps or a refusal, and keeps in
    /// `peak` the largest |displacement| and |velocity| landed.
    fn drive_sine(
        card: &Tesseract,
        state: &mut TesseractState,
        peak_m_s2: f64,
        hz: f64,
        first_index: u64,
        limit: u64,
        peak: &mut [f64; 2],
    ) -> (u64, Option<StepReject>) {
        let dt_s = card.gate_dt_s().unwrap();
        let mut out = frame();
        let mut n = 0u64;
        while n < limit {
            if let Err(reject) = step(
                card,
                state,
                &mut out,
                sine_load(peak_m_s2, hz, first_index + n, dt_s),
                dt_s,
            ) {
                return (n, Some(reject));
            }
            peak[0] = peak[0].max(state.displacement_m.abs());
            peak[1] = peak[1].max(state.velocity_m_s.abs());
            n += 1;
        }
        (n, None)
    }

    /// The drive of 45 m/s² peak at 83.04 Hz from -6.5 mm at rest completes
    /// a million steps with the residual at half its limit, as the step
    /// ceiling sentence says.
    #[test]
    fn a_drive_of_45_at_83_04_hz_completes_a_million_steps_at_half_the_limit() {
        let card = Tesseract::drive_mode();
        let mut state = TesseractState::try_phase(&card, -0.0065, 0.0).unwrap();
        let (landed, reject) =
            drive_sine(&card, &mut state, 45.0, 83.04, 0, 1_000_000, &mut [0.0, 0.0]);
        assert_eq!((landed, reject), (1_000_000, None));
        assert!(state.residual_conservation_error > 0.45e-6, "{}", state.residual_conservation_error);
        assert!(state.residual_conservation_error < 0.55e-6, "{}", state.residual_conservation_error);
    }

    /// The drive of 46 m/s² peak at 83.3 Hz rides the upper branch near
    /// 7 mm and 3.6 m/s, as the step ceiling sentence says.
    #[test]
    fn the_46_drive_rides_the_upper_branch() {
        let card = Tesseract::drive_mode();
        let mut state = TesseractState::try_phase(&card, -0.0066, 0.0).unwrap();
        let mut peak = [0.0, 0.0];
        let (landed, reject) = drive_sine(&card, &mut state, 46.0, 83.3, 0, 1_000_000, &mut peak);
        assert_eq!((landed, reject), (1_000_000, None));
        assert!(peak[0] > 6.5e-3 && peak[0] <= super::AMPLITUDE_LIMIT_M, "{peak:?}");
        assert!(peak[1] > 3.3 && peak[1] < 3.9, "{peak:?}");
    }

    /// One run inside the bounds sums half the ledger's limit in a million
    /// steps. Cut there and started again with its energy_defect_j, it is
    /// refused at the step the run straight through is refused at. Started
    /// again with the defect set to zero, the same steps land: the ledger is
    /// then a new run's.
    #[test]
    fn a_run_cut_and_continued_with_its_defect_is_refused_where_the_straight_run_is() {
        let card = Tesseract::drive_mode();
        let mut straight = TesseractState::try_phase(&card, -0.0066, 0.0).unwrap();
        let (landed, reject) = drive(&card, &mut straight, 0, 1_000_000);
        assert_eq!((landed, reject), (1_000_000, None));
        assert!(straight.residual_conservation_error > 0.4e-6);
        assert!(straight.residual_conservation_error < 0.6e-6);
        assert!(card.holds(straight.displacement_m, straight.velocity_m_s));

        let mut continued = TesseractState::try_continue(
            &card,
            straight.displacement_m,
            straight.velocity_m_s,
            straight.energy_defect_j,
        )
        .unwrap();
        // The anchor is the energy less the defect, one rounding of the
        // energy away from the carried defect.
        assert!(
            (continued.energy_defect_j - straight.energy_defect_j).abs()
                <= 1.0e-6 * card.defect_limit_j()
        );
        let mut fresh =
            TesseractState::try_phase(&card, straight.displacement_m, straight.velocity_m_s)
                .unwrap();

        let (straight_more, straight_reject) =
            drive(&card, &mut straight, 1_000_000, 1_000_000);
        let (continued_more, continued_reject) =
            drive(&card, &mut continued, 1_000_000, 1_000_000);
        let (fresh_more, fresh_reject) = drive(&card, &mut fresh, 1_000_000, 1_000_000);
        assert!(straight_reject.is_some());
        assert_eq!(continued_reject, straight_reject);
        assert_eq!(continued_more, straight_more);
        assert_eq!(straight_more, 914_930);
        assert_eq!(straight_reject, Some(StepReject::SpontaneousGeneration));
        assert_eq!((fresh_more, fresh_reject), (1_000_000, None));
    }

    /// A start must carry a defect inside the ledger's limit, and inside the
    /// card's bounds. Exactly at either limit it is a start.
    #[test]
    fn a_start_past_the_ledger_or_the_bounds_is_not_a_start() {
        let card = Tesseract::drive_mode();
        let limit = card.defect_limit_j();
        assert!(TesseractState::try_continue(&card, 0.0, 0.0, limit).is_some());
        assert!(TesseractState::try_continue(&card, 0.0, 0.0, -limit).is_some());
        assert!(TesseractState::try_continue(&card, 0.0, 0.0, limit.next_up()).is_none());
        assert!(TesseractState::try_continue(&card, 0.0, 0.0, (-limit).next_down()).is_none());
        assert!(TesseractState::try_continue(&card, 0.0, 0.0, f64::NAN).is_none());
        let reach = card.amplitude_limit_m;
        let speed = card.speed_limit_m_s();
        assert!(TesseractState::try_phase(&card, reach, speed).is_some());
        assert!(TesseractState::try_phase(&card, -reach, -speed).is_some());
        assert!(TesseractState::try_phase(&card, reach.next_up(), 0.0).is_none());
        assert!(TesseractState::try_phase(&card, 0.0, (-speed).next_down()).is_none());
        assert_eq!(speed.to_bits(), 4.066797753299356_f64.to_bits());
    }

    /// A step that would land outside the bounds is refused by name, the
    /// state and frame kept, from rest under the largest held load, which
    /// lands its first step and is refused on its second. A start beyond the
    /// bounds handed to `step` is refused the same way.
    #[test]
    fn a_step_out_of_the_bounds_is_refused_and_keeps_the_state() {
        let card = Tesseract::drive_mode();
        let dt_s = card.gate_dt_s().unwrap();
        let load = TesseractLoad {
            accel_m_s2: card.force_nyquist_m_s2(dt_s).unwrap(),
            omega_ext_rad_s: 0.0,
            sense_velocity_m_s: 1.0,
        };
        let mut state = TesseractState::try_rest(&card).unwrap();
        let mut out = frame();
        step(&card, &mut state, &mut out, load, dt_s).unwrap();
        assert!(card.holds(state.displacement_m, state.velocity_m_s));
        let held = state;
        let held_frame = out.to_bytes();
        assert_eq!(
            step(&card, &mut state, &mut out, load, dt_s),
            Err(StepReject::AmplitudeOutsideCard)
        );
        assert_eq!(state, held);
        assert_eq!(out.to_bytes(), held_frame);

        let mut outside = held;
        outside.velocity_m_s = 23.46976127355659;
        let kept = outside;
        assert_eq!(
            step(&card, &mut outside, &mut out, TesseractLoad::default(), dt_s),
            Err(StepReject::AmplitudeOutsideCard)
        );
        assert_eq!(outside, kept);
        assert_eq!(out.to_bytes(), held_frame);
    }
}
