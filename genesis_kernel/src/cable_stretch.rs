#![no_implicit_prelude]

//! Dyneema tendon under classical RK4.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, elongation, velocity, tension, kinetic energy,
//! potential energy, mechanical energy, the margin to ultimate tension, and
//! the conservation residual into [`LastStateFrame64`]. The 16-byte frame
//! proof is not touched.
//!
//! The campaign map is `ΔL = F L / (A E)` for a 1.20 m tendon, 1.50 mm
//! diameter, modulus 120 GPa. This card integrates that spring. The mass is
//! the tendon at 970 kg/m³, so `ω = √(E / ρ) / L`. The locked damper is the
//! braid loss `tan δ = 0.02`, hence `ζ = 0.01`. Ultimate tension is 2800 MPa
//! on this cross-section. Tracking degrades at 2 mm. The control limit is
//! 5 mm.
//!
//! ```text
//! ẍ + 2 ζ ω ẋ + ω² x = F / m
//! T = k x + c ẋ
//! k = m ω²
//! E = ½ m v² + ½ k x²
//! dW/dt = F v − c v²
//! residual_conservation_error = |E − E_anchor − W| / E_ultimate
//! ```
//!
//! `E_ultimate` is the elastic energy at the 2800 MPa elongation. A rope
//! has no compressive branch: a start, a stage or a landing with `x < 0` or
//! `T < 0` is [`StepReject::Slack`]. A rope shorter than its free length
//! carries no spring tension, whatever the damper term adds. `x` at or above
//! the ultimate elongation `x_ult = T_ult / k`, or `T` at or above ultimate,
//! is [`StepReject::TensileFailure`]: past `x_ult` the elastic stress is past
//! 2800 MPa and the braid has failed, whatever the damper term takes away.
//! Together the two give the velocity its range at each elongation:
//! `v` in `[-k x / c, (T_ult - k x) / c)`.
//!
//! A landing whose elongation or velocity is a subnormal number is
//! [`StepReject::SubnormalState`]. The gateway does not accept a subnormal
//! number as a start, so a sealed state can always be sent back as one.
//! Explicit Euler is not the public step. From a taut hold it creates
//! kinetic energy on the first gate step.
//!
//! [`Tendon::gate_dt_s`] is `ω Δt = 0.02`. The public step does not apply
//! it. The force Nyquist limit is applied: `|F| dt / m ≤ ω x_ult`. Past it
//! the step is [`StepReject::ForceAboveNyquist`]. Elongation at or above
//! 2 mm sets [`FLAG_TRACKING_DEGRADE`]. At or above 5 mm the frame also
//! sets [`FLAG_CONTROL_LIMIT`]. Those flags do not reject the step.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Campaign free length.
pub const LENGTH_M: f64 = 1.20;
/// Campaign braid diameter.
pub const DIAMETER_MM: f64 = 1.50;
/// Campaign Young's modulus.
pub const YOUNGS_MODULUS_GPA: f64 = 120.0;
/// HMPE density of the braid.
pub const DENSITY_KG_M3: f64 = 970.0;
/// `tan δ / 2` with braid loss `tan δ = 0.02`.
pub const DAMPING_RATIO: f64 = 0.01;
/// Ultimate tensile stress. Landing at or above this tension snaps the braid.
pub const ULTIMATE_STRESS_MPA: f64 = 2_800.0;
/// Campaign tracking degrade.
pub const DEGRADE_ELONGATION_M: f64 = 0.002;
/// Campaign control limit.
pub const CONTROL_ELONGATION_M: f64 = 0.005;

/// `|E − E_anchor − W| / E_ultimate` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Phase advance `ω Δt` used by [`Tendon::gate_dt_s`].
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Elongation is at or above [`DEGRADE_ELONGATION_M`] on this frame.
pub const FLAG_TRACKING_DEGRADE: u64 = 1 << 0;
/// Elongation is at or above [`CONTROL_ELONGATION_M`] on this frame.
pub const FLAG_CONTROL_LIMIT: u64 = 1 << 1;

/// Why a step did not advance the tendon or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Force or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// Frequency, damping, mass, or ultimate tension is not this braid.
    InvalidCable,
    /// `|F| dt / m` is past `ω x_ult`.
    ForceAboveNyquist,
    /// Elongation reached the ultimate elongation, or tension reached the
    /// ultimate tensile load.
    TensileFailure,
    /// Elongation or tension went negative. The braid is slack.
    Slack,
    /// The incoming state, or the landing, has a subnormal elongation or velocity.
    SubnormalState,
    /// The trial step created mechanical energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed mechanical energy.
    FirstLawBreach,
}

/// Zero-order hold. Positive force stretches the braid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TendonLoad {
    pub force_n: f64,
}

impl Default for TendonLoad {
    fn default() -> Self {
        Self { force_n: 0.0 }
    }
}

/// Locked braid. Frequency, damping, mass, and ultimate tension are recorded copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tendon {
    pub omega_rad_s: f64,
    pub zeta: f64,
    pub mass_kg: f64,
    pub ultimate_tension_n: f64,
}

impl Tendon {
    /// Card: 1.50 mm by 1.20 m Dyneema, ζ = 0.01, snap at 2800 MPa.
    #[must_use]
    pub fn dyneema() -> Self {
        let tendon = Self {
            omega_rad_s: natural_frequency_rad_s(),
            zeta: DAMPING_RATIO,
            mass_kg: tendon_mass_kg(),
            ultimate_tension_n: ultimate_tension_n(),
        };
        if tendon.is_valid() {
            tendon
        } else {
            ::core::panic!("Dyneema tendon failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.omega_rad_s.to_bits() == natural_frequency_rad_s().to_bits()
            && self.zeta.to_bits() == DAMPING_RATIO.to_bits()
            && self.mass_kg.to_bits() == tendon_mass_kg().to_bits()
            && self.ultimate_tension_n.to_bits() == ultimate_tension_n().to_bits()
            && self.omega_rad_s.is_finite()
            && self.omega_rad_s > 0.0
            && self.zeta >= 0.0
            && self.zeta < 1.0
            && self.mass_kg > 0.0
            && self.ultimate_tension_n > 0.0
            && self.stiffness_n_per_m().is_finite()
            && self.damping_n_s_per_m().is_finite()
            && self.ultimate_elongation_m().is_finite()
            && self.ultimate_elongation_m() > CONTROL_ELONGATION_M
            && self.ultimate_energy_j().is_finite()
            && self.ultimate_energy_j() > 0.0
    }

    #[must_use]
    pub fn stiffness_n_per_m(&self) -> f64 {
        self.mass_kg * self.omega_rad_s * self.omega_rad_s
    }

    #[must_use]
    pub fn damping_n_s_per_m(&self) -> f64 {
        2.0 * self.zeta * self.omega_rad_s * self.mass_kg
    }

    #[must_use]
    pub fn ultimate_elongation_m(&self) -> f64 {
        self.ultimate_tension_n / self.stiffness_n_per_m()
    }

    /// Elastic energy at the ultimate elongation.
    #[must_use]
    pub fn ultimate_energy_j(&self) -> f64 {
        0.5 * self.stiffness_n_per_m() * self.ultimate_elongation_m() * self.ultimate_elongation_m()
    }

    /// Largest reporting step of the braid. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.omega_rad_s.is_finite() && self.omega_rad_s > 0.0 {
            Some(GATE_PHASE_STEP_RAD / self.omega_rad_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step turns the braid's vibration by two hundredths of a radian. A million times smaller, a step
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

    /// `m * omega * x_ult`, in newton seconds: the constant of the load rule
    /// `|F| * dt_s <= C`.
    #[must_use]
    pub fn force_impulse_limit_n_s(&self) -> f64 {
        self.mass_kg * self.omega_rad_s * self.ultimate_elongation_m()
    }

    /// Largest `|F|` with `|F| * dt_s <= force_impulse_limit_n_s`, the product
    /// rounded once to 64 bits and compared with the constant, as [`step`]
    /// applies it.
    #[must_use]
    pub fn force_nyquist_n(&self, dt_s: f64) -> Option<f64> {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return None;
        }
        crate::impulse::largest(self.force_impulse_limit_n_s(), dt_s)
    }

    #[must_use]
    pub fn tension_n(&self, elongation_m: f64, velocity_m_s: f64) -> f64 {
        self.stiffness_n_per_m() * elongation_m + self.damping_n_s_per_m() * velocity_m_s
    }

    #[must_use]
    pub fn mechanical_energy_j(&self, elongation_m: f64, velocity_m_s: f64) -> f64 {
        0.5 * self.mass_kg * velocity_m_s * velocity_m_s
            + 0.5 * self.stiffness_n_per_m() * elongation_m * elongation_m
    }
}

fn section_area_m2() -> f64 {
    let radius_m = DIAMETER_MM * 1.0e-3 * 0.5;
    core::f64::consts::PI * radius_m * radius_m
}

fn axial_stiffness_n_per_m() -> f64 {
    section_area_m2() * YOUNGS_MODULUS_GPA * 1.0e9 / LENGTH_M
}

fn tendon_mass_kg() -> f64 {
    DENSITY_KG_M3 * section_area_m2() * LENGTH_M
}

fn natural_frequency_rad_s() -> f64 {
    (axial_stiffness_n_per_m() / tendon_mass_kg()).sqrt()
}

fn ultimate_tension_n() -> f64 {
    ULTIMATE_STRESS_MPA * 1.0e6 * section_area_m2()
}

/// Integrated tendon. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TendonState {
    pub t_s: f64,
    pub elongation_m: f64,
    pub velocity_m_s: f64,
    pub force_n: f64,
    pub tension_n: f64,
    pub kinetic_j: f64,
    pub potential_j: f64,
    pub mechanical_energy_j: f64,
    pub energy_anchor_j: f64,
    pub boundary_work_j: f64,
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    pub ultimate_margin_n: f64,
    pub tracking_degraded: bool,
    pub control_limited: bool,
}

impl TendonState {
    #[must_use]
    pub fn try_rest(tendon: &Tendon) -> Option<Self> {
        Self::try_phase(tendon, 0.0, 0.0)
    }

    /// Anchor the ledger at this elongation and velocity.
    #[must_use]
    pub fn try_phase(tendon: &Tendon, elongation_m: f64, velocity_m_s: f64) -> Option<Self> {
        if !tendon.is_valid() || !elongation_m.is_finite() || !velocity_m_s.is_finite() {
            return None;
        }
        if phase_is_subnormal(elongation_m, velocity_m_s) {
            return None;
        }
        if tension_reject(tendon, elongation_m, velocity_m_s).is_err() {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            elongation_m,
            velocity_m_s,
            force_n: 0.0,
            tension_n: 0.0,
            kinetic_j: 0.0,
            potential_j: 0.0,
            mechanical_energy_j: 0.0,
            energy_anchor_j: 0.0,
            boundary_work_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            ultimate_margin_n: 0.0,
            tracking_degraded: false,
            control_limited: false,
        };
        refresh(tendon, &mut state);
        if !state_is_finite(&state) {
            return None;
        }
        state.energy_anchor_j = state.mechanical_energy_j;
        state.boundary_work_j = 0.0;
        state.energy_defect_j = 0.0;
        state.residual_conservation_error = 0.0;
        Some(state)
    }
}

/// Advance one RK4 step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    tendon: &Tendon,
    state: &mut TendonState,
    frame: &mut LastStateFrame64,
    load: TendonLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(tendon, state, load, dt_s)?;
    let predicted = match rk4(tendon, pack(state), dt_s, load.force_n) {
        Ok(y) => y,
        Err(reject) => return Err(reject),
    };
    commit(tendon, state, frame, load, dt_s, predicted)
}

fn validate(
    tendon: &Tendon,
    state: &TendonState,
    load: TendonLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.force_n.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !tendon.is_valid() {
        return Err(StepReject::InvalidCable);
    }
    if phase_is_subnormal(state.elongation_m, state.velocity_m_s) {
        return Err(StepReject::SubnormalState);
    }
    if let Err(reject) = tension_reject(tendon, state.elongation_m, state.velocity_m_s) {
        return Err(reject);
    }
    if exceeds_force_nyquist(tendon, load.force_n, dt_s) {
        return Err(StepReject::ForceAboveNyquist);
    }
    Ok(())
}

fn exceeds_force_nyquist(tendon: &Tendon, force_n: f64, dt_s: f64) -> bool {
    if force_n == 0.0 {
        return false;
    }
    match tendon.force_nyquist_n(dt_s) {
        Some(_) => !crate::impulse::within(force_n, dt_s, tendon.force_impulse_limit_n_s()),
        None => true,
    }
}

/// Elongation first, then tension. `x < 0` is a rope shorter than its free
/// length; `x >= x_ult` is a braid past its ultimate stress. Neither is held
/// by the damper term. `k x >= T_ult` exactly when `x >= x_ult` on this braid,
/// as the kernel tests pin.
fn tension_reject(tendon: &Tendon, elongation_m: f64, velocity_m_s: f64) -> Result<(), StepReject> {
    let tension_n = tendon.tension_n(elongation_m, velocity_m_s);
    if !tension_n.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if elongation_m < 0.0 {
        return Err(StepReject::Slack);
    }
    if elongation_m >= tendon.ultimate_elongation_m() {
        return Err(StepReject::TensileFailure);
    }
    if tension_n < 0.0 {
        return Err(StepReject::Slack);
    }
    if tension_n >= tendon.ultimate_tension_n {
        return Err(StepReject::TensileFailure);
    }
    Ok(())
}

fn commit(
    tendon: &Tendon,
    state: &mut TendonState,
    frame: &mut LastStateFrame64,
    load: TendonLoad,
    dt_s: f64,
    predicted: [f64; 3],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite3(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    if let Err(reject) = tension_reject(tendon, predicted[0], predicted[1]) {
        return Err(reject);
    }
    if phase_is_subnormal(predicted[0], predicted[1]) {
        return Err(StepReject::SubnormalState);
    }
    state.elongation_m = predicted[0];
    state.velocity_m_s = predicted[1];
    state.boundary_work_j = predicted[2];
    state.force_n = load.force_n;
    state.t_s += dt_s;
    refresh(tendon, state);
    let scale = tendon.ultimate_energy_j();
    if !(scale > 0.0 && scale.is_finite()) {
        *state = saved;
        return Err(StepReject::InvalidCable);
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

fn refresh(tendon: &Tendon, state: &mut TendonState) {
    state.tension_n = tendon.tension_n(state.elongation_m, state.velocity_m_s);
    state.kinetic_j = 0.5 * tendon.mass_kg * state.velocity_m_s * state.velocity_m_s;
    state.potential_j = 0.5 * tendon.stiffness_n_per_m() * state.elongation_m * state.elongation_m;
    state.mechanical_energy_j = state.kinetic_j + state.potential_j;
    state.ultimate_margin_n = tendon.ultimate_tension_n - state.tension_n;
    state.tracking_degraded = state.elongation_m >= DEGRADE_ELONGATION_M;
    state.control_limited = state.elongation_m >= CONTROL_ELONGATION_M;
}

fn publish(state: &TendonState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.elongation_m as f32;
    frame.pos[1] = state.velocity_m_s as f32;
    frame.pos[2] = state.tension_n as f32;
    frame.vel[0] = state.kinetic_j as f32;
    frame.vel[1] = state.potential_j as f32;
    frame.vel[2] = state.mechanical_energy_j as f32;
    frame.force_torque = state.ultimate_margin_n as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0u64;
    if state.tracking_degraded {
        flags |= FLAG_TRACKING_DEGRADE;
    }
    if state.control_limited {
        flags |= FLAG_CONTROL_LIMIT;
    }
    frame.flags = flags;
}

fn derivatives(tendon: &Tendon, y: [f64; 3], force_n: f64) -> Result<[f64; 3], StepReject> {
    let elongation_m = y[0];
    let velocity_m_s = y[1];
    tension_reject(tendon, elongation_m, velocity_m_s)?;
    let omega = tendon.omega_rad_s;
    let acceleration = force_n / tendon.mass_kg
        - 2.0 * tendon.zeta * omega * velocity_m_s
        - omega * omega * elongation_m;
    let power_w = force_n * velocity_m_s - tendon.damping_n_s_per_m() * velocity_m_s * velocity_m_s;
    let out = [velocity_m_s, acceleration, power_w];
    if finite3(out) {
        Ok(out)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

fn rk4(tendon: &Tendon, y: [f64; 3], dt: f64, force_n: f64) -> Result<[f64; 3], StepReject> {
    let k1 = match derivatives(tendon, y, force_n) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k2 = match derivatives(tendon, axpy(y, dt * 0.5, k1), force_n) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k3 = match derivatives(tendon, axpy(y, dt * 0.5, k2), force_n) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k4 = match derivatives(tendon, axpy(y, dt, k3), force_n) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let mut next = y;
    let mut i = 0;
    while i < 3 {
        let increment = k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i];
        if increment != 0.0 {
            next[i] += (dt / 6.0) * increment;
        }
        i += 1;
    }
    if finite3(next) {
        Ok(next)
    } else {
        Err(StepReject::NonFiniteState)
    }
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

fn pack(state: &TendonState) -> [f64; 3] {
    [
        state.elongation_m,
        state.velocity_m_s,
        state.boundary_work_j,
    ]
}

/// The gateway's number rule on the two fields a start is sent back with.
fn phase_is_subnormal(elongation_m: f64, velocity_m_s: f64) -> bool {
    elongation_m.is_subnormal() || velocity_m_s.is_subnormal()
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn state_is_finite(state: &TendonState) -> bool {
    state.t_s.is_finite()
        && state.elongation_m.is_finite()
        && state.velocity_m_s.is_finite()
        && state.force_n.is_finite()
        && state.tension_n.is_finite()
        && state.kinetic_j.is_finite()
        && state.potential_j.is_finite()
        && state.mechanical_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.boundary_work_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.ultimate_margin_n.is_finite()
}

/// Explicit Euler on `(x, v, W)`. One gate step from a taut hold creates kinetic energy.
#[cfg(test)]
fn step_euler(
    tendon: &Tendon,
    state: &mut TendonState,
    frame: &mut LastStateFrame64,
    load: TendonLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(tendon, state, load, dt_s)?;
    let k1 = match derivatives(tendon, pack(state), load.force_n) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let predicted = axpy(pack(state), dt_s, k1);
    if !finite3(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    commit(tendon, state, frame, load, dt_s, predicted)
}

/// Add `extra_j` to the integrated work after a faithful RK4 step.
#[cfg(test)]
fn step_bias_work(
    tendon: &Tendon,
    state: &mut TendonState,
    frame: &mut LastStateFrame64,
    load: TendonLoad,
    dt_s: f64,
    extra_j: f64,
) -> Result<(), StepReject> {
    validate(tendon, state, load, dt_s)?;
    let mut predicted = match rk4(tendon, pack(state), dt_s, load.force_n) {
        Ok(y) => y,
        Err(reject) => return Err(reject),
    };
    predicted[2] += extra_j;
    commit(tendon, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        CONTROL_ELONGATION_M, DAMPING_RATIO, DEGRADE_ELONGATION_M, DIAMETER_MM, FLAG_CONTROL_LIMIT,
        FLAG_TRACKING_DEGRADE, LENGTH_M, StepReject, Tendon, TendonLoad, TendonState,
        ULTIMATE_STRESS_MPA, YOUNGS_MODULUS_GPA, axial_stiffness_n_per_m, natural_frequency_rad_s,
        step, step_bias_work, step_euler,
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
    fn card_locks_stiffness_and_damping() {
        assert_copy::<Tendon>();
        assert_copy::<TendonState>();
        assert_copy::<TendonLoad>();
        assert_copy::<StepReject>();
        let tendon = Tendon::dyneema();
        assert_eq!(
            tendon.omega_rad_s.to_bits(),
            natural_frequency_rad_s().to_bits()
        );
        assert_eq!(tendon.zeta.to_bits(), DAMPING_RATIO.to_bits());
        assert_eq!(LENGTH_M, 1.20);
        assert_eq!(DIAMETER_MM, 1.50);
        assert_eq!(YOUNGS_MODULUS_GPA, 120.0);
        assert_eq!(ULTIMATE_STRESS_MPA, 2_800.0);
        let relative = (tendon.stiffness_n_per_m() - axial_stiffness_n_per_m()).abs()
            / axial_stiffness_n_per_m();
        assert!(relative < 1.0e-12, "stiffness drift {relative}");
        assert!(tendon.ultimate_elongation_m() > CONTROL_ELONGATION_M);
        let gate = tendon.gate_dt_s().unwrap();
        assert!(gate > 0.0);
        let mut tampered = tendon;
        tampered.zeta = 0.2;
        assert!(!tampered.is_valid());
        let mut state = TendonState::try_rest(&tendon).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(
            &tampered,
            &mut state,
            &mut frame,
            TendonLoad::default(),
            gate,
        );
        assert_eq!(got, Err(StepReject::InvalidCable));
        assert_eq!(state, saved);
        proof_intact(&frame);
    }

    #[test]
    fn free_ring_tracks_the_underdamped_solution_while_taut() {
        let tendon = Tendon::dyneema();
        let x0 = 1.0e-3;
        let mut state = TendonState::try_phase(&tendon, x0, 0.0).unwrap();
        let mut frame = blank_frame();
        let dt = tendon.gate_dt_s().unwrap();
        let steps = 40u32;
        let mut max_residual = 0.0;
        let mut n = 0u32;
        while n < steps {
            step(&tendon, &mut state, &mut frame, TendonLoad::default(), dt).unwrap();
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            assert!(state.tension_n > 0.0);
            n += 1;
        }
        let (x, v) = underdamped(x0, 0.0, tendon.omega_rad_s, tendon.zeta, state.t_s);
        assert!(
            (state.elongation_m - x).abs() < 1.0e-9,
            "x {} analytic {x}",
            state.elongation_m
        );
        assert!(
            (state.velocity_m_s - v).abs() < 1.0e-6,
            "v {} analytic {v}",
            state.velocity_m_s
        );
        assert!(
            max_residual <= super::RESIDUAL_CONSERVATION_LIMIT,
            "residual {max_residual}"
        );
        assert!(!state.tracking_degraded);
        proof_intact(&frame);
    }

    #[test]
    fn campaign_elongation_flags_and_a_shut_end_holds() {
        let tendon = Tendon::dyneema();
        let dt = tendon.gate_dt_s().unwrap();
        let mut soft = TendonState::try_phase(&tendon, 0.0025, 0.0).unwrap();
        let mut frame = blank_frame();
        step(&tendon, &mut soft, &mut frame, TendonLoad::default(), dt).unwrap();
        assert!(soft.elongation_m >= DEGRADE_ELONGATION_M);
        assert!(soft.elongation_m < CONTROL_ELONGATION_M);
        assert_eq!(frame.flags, FLAG_TRACKING_DEGRADE);
        assert!(soft.tension_n < tendon.ultimate_tension_n);

        let mut hard = TendonState::try_phase(&tendon, 0.0055, 0.0).unwrap();
        step(&tendon, &mut hard, &mut frame, TendonLoad::default(), dt).unwrap();
        assert!(hard.elongation_m >= CONTROL_ELONGATION_M);
        assert_eq!(frame.flags, FLAG_TRACKING_DEGRADE | FLAG_CONTROL_LIMIT);
        proof_intact(&frame);

        let mut rest = TendonState::try_rest(&tendon).unwrap();
        step(
            &tendon,
            &mut rest,
            &mut frame,
            TendonLoad::default(),
            f64::MAX,
        )
        .unwrap();
        assert_eq!(rest.elongation_m, 0.0);
        assert_eq!(rest.tension_n, 0.0);
        assert_eq!(rest.residual_conservation_error, 0.0);
    }

    #[test]
    fn compression_goes_slack_and_a_near_snap_does_not_land() {
        let tendon = Tendon::dyneema();
        let dt = tendon.gate_dt_s().unwrap();
        let mut state = TendonState::try_rest(&tendon).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(
            &tendon,
            &mut state,
            &mut frame,
            TendonLoad { force_n: -1.0 },
            dt,
        );
        assert_eq!(got, Err(StepReject::Slack));
        assert_eq!(state, saved);
        proof_intact(&frame);

        let x = 0.99 * tendon.ultimate_elongation_m();
        let tension = tendon.tension_n(x, 0.0);
        let velocity = (0.999 * tendon.ultimate_tension_n - tension) / tendon.damping_n_s_per_m();
        let mut near = TendonState::try_phase(&tendon, x, velocity).unwrap();
        assert!(near.tension_n < tendon.ultimate_tension_n);
        let held = near;
        let got = step(&tendon, &mut near, &mut frame, TendonLoad::default(), dt);
        assert_eq!(got, Err(StepReject::TensileFailure));
        assert_eq!(near, held);
        proof_intact(&frame);
    }

    #[test]
    fn force_nyquist_and_explicit_euler_from_a_taut_hold() {
        let tendon = Tendon::dyneema();
        let dt = tendon.gate_dt_s().unwrap();
        let limit = tendon.force_nyquist_n(dt).unwrap();
        let mut state = TendonState::try_rest(&tendon).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(
            &tendon,
            &mut state,
            &mut frame,
            TendonLoad {
                force_n: limit + 1.0,
            },
            dt,
        );
        assert_eq!(got, Err(StepReject::ForceAboveNyquist));
        assert_eq!(state, saved);
        proof_intact(&frame);
        step(
            &tendon,
            &mut state,
            &mut frame,
            TendonLoad { force_n: limit },
            dt,
        )
        .unwrap();
        assert!(state.residual_conservation_error <= super::RESIDUAL_CONSERVATION_LIMIT);
        assert!(state.tension_n > 0.0);
        assert!(state.tension_n < tendon.ultimate_tension_n);

        let x = 0.5 * tendon.ultimate_elongation_m();
        let mut held = TendonState::try_phase(&tendon, x, 0.0).unwrap();
        let before = held;
        let got = step_euler(&tendon, &mut held, &mut frame, TendonLoad::default(), dt);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(held, before);
        proof_intact(&frame);
    }

    #[test]
    fn biased_work_and_non_finite_inputs_roll_back() {
        let tendon = Tendon::dyneema();
        let dt = tendon.gate_dt_s().unwrap();
        let mut state = TendonState::try_phase(&tendon, 1.0e-3, 0.0).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let extra = tendon.ultimate_energy_j() * 1.0e-3;
        let got = step_bias_work(
            &tendon,
            &mut state,
            &mut frame,
            TendonLoad::default(),
            dt,
            extra,
        );
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state, saved);
        let got = step_bias_work(
            &tendon,
            &mut state,
            &mut frame,
            TendonLoad::default(),
            dt,
            -extra,
        );
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        let mut refuse = |load: TendonLoad, dt_s: f64, expect: StepReject| {
            let got = step(&tendon, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state, saved);
            proof_intact(&frame);
        };
        refuse(TendonLoad::default(), 0.0, StepReject::NonPositiveDt);
        refuse(TendonLoad::default(), -dt, StepReject::NonPositiveDt);
        refuse(TendonLoad::default(), f64::NAN, StepReject::NonFiniteInput);
        refuse(
            TendonLoad { force_n: f64::MAX },
            dt,
            StepReject::ForceAboveNyquist,
        );
        state.velocity_m_s = f64::NAN;
        let got = step(&tendon, &mut state, &mut frame, TendonLoad::default(), dt);
        assert_eq!(got, Err(StepReject::NonFiniteState));
        proof_intact(&frame);
    }

    /// The ultimate elongation is the published bound, half open, and on this
    /// braid it is exactly where the elastic tension `k x` reaches ultimate.
    #[test]
    fn the_elongation_has_its_own_range_and_the_damper_term_does_not_widen_it() {
        let tendon = Tendon::dyneema();
        let dt = tendon.gate_dt_s().unwrap();
        let x_ult = tendon.ultimate_elongation_m();
        let below = f64::from_bits(x_ult.to_bits() - 1);
        assert!(tendon.stiffness_n_per_m() * x_ult >= tendon.ultimate_tension_n);
        assert!(tendon.stiffness_n_per_m() * below < tendon.ultimate_tension_n);
        assert!(TendonState::try_phase(&tendon, below, 0.0).is_some());
        assert!(TendonState::try_phase(&tendon, x_ult, 0.0).is_none());
        assert!(TendonState::try_phase(&tendon, 0.0, 0.0).is_some());
        assert!(TendonState::try_phase(&tendon, -0.0, 0.0).is_some());

        // Four starts whose tension is in range and whose elongation is
        // not. Each is refused by name, at the start.
        let floor = tendon.floor_dt_s().unwrap();
        let cases = [
            (-0.001, 725.6933909687426, StepReject::Slack),
            (-1.0, 469928.1475272422, StepReject::Slack),
            (-1.0e-30, 1.0, StepReject::Slack),
            (0.03, -7415.051030799522, StepReject::TensileFailure),
            (1.0, -456951.8496728388, StepReject::TensileFailure),
            (x_ult, -1.0, StepReject::TensileFailure),
        ];
        for (x, v, expect) in cases {
            let tension = tendon.tension_n(x, v);
            assert!(
                tension >= 0.0 && tension < tendon.ultimate_tension_n,
                "x {x} v {v} tension {tension}"
            );
            assert!(TendonState::try_phase(&tendon, x, v).is_none(), "x {x}");
            // A forged state reaches the step and is refused there too.
            let mut state = TendonState::try_phase(&tendon, 1.0e-3, 0.0).unwrap();
            state.elongation_m = x;
            state.velocity_m_s = v;
            let saved = state;
            let mut frame = blank_frame();
            let got = step(&tendon, &mut state, &mut frame, TendonLoad::default(), floor);
            assert_eq!(got, Err(expect), "x {x} v {v}");
            assert_eq!(state, saved);
            proof_intact(&frame);
        }

        // The velocity range that follows, at 1 mm: both ends.
        let x = 1.0e-3;
        let k = tendon.stiffness_n_per_m();
        let c = tendon.damping_n_s_per_m();
        let slowest = -k * x / c;
        let mut v = slowest;
        while tendon.tension_n(x, v) < 0.0 {
            v = f64::from_bits(v.to_bits() - 1);
        }
        assert!(TendonState::try_phase(&tendon, x, v).is_some());
        let mut faster = (tendon.ultimate_tension_n - k * x) / c;
        while tendon.tension_n(x, faster) < tendon.ultimate_tension_n {
            faster = f64::from_bits(faster.to_bits() + 1);
        }
        assert!(TendonState::try_phase(&tendon, x, faster).is_none());
        let mut state = TendonState::try_phase(&tendon, 1.0e-3, 0.0).unwrap();
        step(&tendon, &mut state, &mut blank_frame(), TendonLoad::default(), dt).unwrap();
    }

    /// A landing on a subnormal elongation is refused and not committed; a
    /// subnormal start is not built.
    #[test]
    fn a_landing_on_a_subnormal_number_is_refused_by_name() {
        let tendon = Tendon::dyneema();
        let dt = tendon.gate_dt_s().unwrap();
        let mut state = TendonState::try_phase(&tendon, 2.2251e-308, 0.0).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(&tendon, &mut state, &mut frame, TendonLoad::default(), dt);
        assert_eq!(got, Err(StepReject::SubnormalState));
        assert_eq!(state, saved);
        proof_intact(&frame);

        assert!(TendonState::try_phase(&tendon, 2.2246550541640664e-308, 0.0).is_none());
        assert!(TendonState::try_phase(&tendon, 1.0e-3, 1.0e-310).is_none());
        let mut forged = TendonState::try_phase(&tendon, 1.0e-3, 0.0).unwrap();
        forged.velocity_m_s = 1.0e-310;
        let held = forged;
        let got = step(&tendon, &mut forged, &mut frame, TendonLoad::default(), dt);
        assert_eq!(got, Err(StepReject::SubnormalState));
        assert_eq!(forged, held);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::Tendon::dyneema();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The tendon stretched 1 mm, moving at 1 mm/s, under 100 N.
        let mut state = super::TendonState::try_phase(&card, 1.0e-3, 1.0e-3).unwrap();
        let load = super::TendonLoad { force_n: 100.0 };
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
        assert!(state.elongation_m != before.elongation_m);
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
    /// agree with the published rule `|force_n| * dt_s <= mass * omega * x_ult`.
    #[test]
    fn the_load_rule_is_the_published_product_compared_with_the_constant() {
        let tendon = super::Tendon::dyneema();
        let limit = tendon.mass_kg * tendon.omega_rad_s * tendon.ultimate_elongation_m();
        crate::impulse::tests::assert_boundary(limit, |load, dt_s| super::exceeds_force_nyquist(&tendon, load, dt_s));
    }
}
