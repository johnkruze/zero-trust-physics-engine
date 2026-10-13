#![no_implicit_prelude]

//! One synchronous machine on a 60 Hz bus.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, rotor angle, speed deviation, PLL error,
//! electrical power, mechanical power, transient energy, the synchronism
//! margin, and the conservation residual into [`LastStateFrame64`]. The
//! 16-byte frame proof is not touched.
//!
//! The card is a round rotor with inertia constant 5 s, damping 0.05 pu,
//! electrical ceiling 1.04 pu, and a PLL bandwidth of 20 s⁻¹. Those
//! constants, the 60 Hz electrical frequency, and the π/2 synchronism
//! ceiling are crate constants. A caller does not pass them. The dispatch
//! command `p_mech` is the zero-order hold.
//!
//! ```text
//! M = 2 H / ω_s
//! M δω̇ = P_m − P_max sin δ − D δω
//! δ̇ = δω
//! φ̇ = δω − K_pll φ
//! U = ½ M δω² − P_m δ − P_max cos δ
//! dW/dt = −D δω²
//! residual_conservation_error = |U − U_anchor − W| / U_barrier
//! ```
//!
//! `φ` is the PLL error. It is a component of the RK4 state, advanced in
//! every stage with the rotor. It does not feed back into `U`. A change of
//! `P_m` is booked at the pre-step angle before the rotor moves. A state
//! made by [`SwingState::try_dispatched`] is anchored under the dispatch it
//! starts with, so a first step at that dispatch books no change. `U_barrier`
//! is the transient-energy gap from the nominal stable equilibrium to the
//! nominal unstable equilibrium.
//!
//! [`SwingMachine::gate_dt_s`] is `ω_s Δt = 0.02` on the electrical
//! frequency, not the swing mode. The public step does not apply it. A
//! landed angle at or beyond ±π/2 is [`StepReject::LossOfSynchronism`]. The
//! state and the frame stay as they were. `|φ|` at or beyond 20° sets
//! [`FLAG_PLL_SLIP`] and does not reject the step.
//!
//! The card ends at a speed deviation of ±6π rad/s, 3 Hz off the bus, and at
//! a dispatch of ±2 pu. A speed beyond the first, on the incoming state or on
//! the landing, is [`StepReject::SpeedDeviationOutsideCard`]. A dispatch
//! beyond the second is [`StepReject::DispatchOutsideCard`].

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Bus frequency.
pub const NOMINAL_FREQUENCY_HZ: f64 = 60.0;
/// Inertia constant, MW·s/MVA.
pub const INERTIA_CONSTANT_S: f64 = 5.0;
/// Speed damping, pu per (rad/s).
pub const DAMPING_PU: f64 = 0.05;
/// Nominal mechanical dispatch, pu.
pub const NOMINAL_P_MECH_PU: f64 = 1.0;
/// Maximum transmissible electrical power, pu.
pub const P_MAX_PU: f64 = 1.04;
/// PLL tracking bandwidth.
pub const K_PLL_PER_S: f64 = 20.0;
/// 20°. Past this the frame is flagged.
pub const PLL_TRIP_RAD: f64 = 20.0 * core::f64::consts::PI / 180.0;
/// Largest speed deviation the card accepts, either way: 3 Hz off the 60 Hz
/// bus, 5 percent of synchronous speed. A rotor this fast crosses the whole
/// ±π/2 window in a sixth of a second and carries a kinetic energy `½ M δω²`
/// of 4.7 pu, 318 times the card's synchronism barrier: it has lost
/// synchronism. A rotor released at rest inside the window does not reach
/// this speed under any held dispatch inside ±[`DISPATCH_LIMIT_PU`]: the
/// most, 17.5 rad/s, is reached as it leaves the window. With both limits,
/// from the step floor to the step ceiling, the largest rounding found in the
/// ledger over a million steps is 8.9e-9 of the barrier, 110 times under the
/// limit. Without them, rounding first refused a step at 150 rad/s.
pub const SPEED_DEVIATION_LIMIT_RAD_S: f64 = 6.0 * core::f64::consts::PI;
/// Largest mechanical dispatch the card accepts, either way: twice the
/// machine's rating. The electrical ceiling is 1.04 pu, so no rotor angle
/// balances a dispatch this large. The transient energy carries the term
/// `P_m δ`, so the size of the dispatch sets the size of the numbers the
/// ledger subtracts; at 10,000 pu rounding alone refused a step.
pub const DISPATCH_LIMIT_PU: f64 = 2.0;

/// `|U − U_anchor − W| / U_barrier` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Electrical phase advance `ω_s Δt` used by [`SwingMachine::gate_dt_s`].
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// `|pll error|` is at or beyond [`PLL_TRIP_RAD`] on this frame.
pub const FLAG_PLL_SLIP: u64 = 1 << 0;

/// Why a step did not advance the machine or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Mechanical power or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// Inertia, damping, ceiling, or the PLL gain is not this machine.
    InvalidMachine,
    /// The landed rotor angle is at or beyond ±π/2.
    LossOfSynchronism,
    /// Rotor speed deviation, incoming or landing, is beyond ±6π rad/s, 3 Hz off the bus.
    SpeedDeviationOutsideCard,
    /// Mechanical dispatch, held or commanded, is beyond ±2 pu, twice the machine's rating.
    DispatchOutsideCard,
    /// The trial step created transient energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed transient energy.
    FirstLawBreach,
}

/// Zero-order hold of the mechanical dispatch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwingLoad {
    pub p_mech_pu: f64,
}

impl Default for SwingLoad {
    fn default() -> Self {
        Self {
            p_mech_pu: NOMINAL_P_MECH_PU,
        }
    }
}

/// Locked synchronous machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwingMachine {
    pub inertia_constant_s: f64,
    pub damping_pu: f64,
    pub p_max_pu: f64,
    pub nominal_p_mech_pu: f64,
    pub omega_electrical_rad_s: f64,
    pub k_pll_per_s: f64,
    /// Recorded copy of the ±π/2 ceiling. Not a caller input.
    pub synchronism_rad: f64,
    /// Recorded copy of [`PLL_TRIP_RAD`]. Not a caller input.
    pub pll_trip_rad: f64,
    /// Recorded copy of [`SPEED_DEVIATION_LIMIT_RAD_S`]. Not a caller input.
    pub speed_deviation_limit_rad_s: f64,
    /// Recorded copy of [`DISPATCH_LIMIT_PU`]. Not a caller input.
    pub dispatch_limit_pu: f64,
}

impl SwingMachine {
    /// Card: 60 Hz machine, H = 5 s, P_max = 1.04 pu.
    #[must_use]
    pub fn synchronous_60hz() -> Self {
        let machine = Self {
            inertia_constant_s: INERTIA_CONSTANT_S,
            damping_pu: DAMPING_PU,
            p_max_pu: P_MAX_PU,
            nominal_p_mech_pu: NOMINAL_P_MECH_PU,
            omega_electrical_rad_s: electrical_omega_rad_s(),
            k_pll_per_s: K_PLL_PER_S,
            synchronism_rad: core::f64::consts::FRAC_PI_2,
            pll_trip_rad: PLL_TRIP_RAD,
            speed_deviation_limit_rad_s: SPEED_DEVIATION_LIMIT_RAD_S,
            dispatch_limit_pu: DISPATCH_LIMIT_PU,
        };
        if machine.is_valid() {
            machine
        } else {
            ::core::panic!("60 Hz machine failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.inertia_constant_s.to_bits() == INERTIA_CONSTANT_S.to_bits()
            && self.damping_pu.to_bits() == DAMPING_PU.to_bits()
            && self.p_max_pu.to_bits() == P_MAX_PU.to_bits()
            && self.nominal_p_mech_pu.to_bits() == NOMINAL_P_MECH_PU.to_bits()
            && self.omega_electrical_rad_s.to_bits() == electrical_omega_rad_s().to_bits()
            && self.k_pll_per_s.to_bits() == K_PLL_PER_S.to_bits()
            && self.synchronism_rad.to_bits() == core::f64::consts::FRAC_PI_2.to_bits()
            && self.pll_trip_rad.to_bits() == PLL_TRIP_RAD.to_bits()
            && self.speed_deviation_limit_rad_s.to_bits() == SPEED_DEVIATION_LIMIT_RAD_S.to_bits()
            && self.dispatch_limit_pu.to_bits() == DISPATCH_LIMIT_PU.to_bits()
            && self.speed_deviation_limit_rad_s > 0.0
            && self.dispatch_limit_pu > self.p_max_pu
            && self.inertia_constant_s > 0.0
            && self.damping_pu > 0.0
            && self.p_max_pu > self.nominal_p_mech_pu
            && self.nominal_p_mech_pu > 0.0
            && self.omega_electrical_rad_s > 0.0
            && self.k_pll_per_s > 0.0
            && self.inertia_m().is_finite()
            && self.synchronism_barrier_pu().is_finite()
            && self.synchronism_barrier_pu() > 0.0
    }

    /// `M = 2 H / ω_s`.
    #[must_use]
    pub fn inertia_m(&self) -> f64 {
        2.0 * self.inertia_constant_s / self.omega_electrical_rad_s
    }

    /// Largest reporting step of the electrical frequency. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let omega = self.omega_electrical_rad_s;
        if omega.is_finite() && omega > 0.0 {
            Some(GATE_PHASE_STEP_RAD / omega)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step turns the electrical angle by two hundredths of a radian. A million times smaller, a step
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

    /// `π / ω_s`.
    #[must_use]
    pub fn nyquist_dt_s(&self) -> Option<f64> {
        let omega = self.omega_electrical_rad_s;
        if omega.is_finite() && omega > 0.0 {
            Some(core::f64::consts::PI / omega)
        } else {
            None
        }
    }

    /// Stable equilibrium angle of a dispatch that the ceiling can still hold.
    #[must_use]
    pub fn equilibrium_delta_rad(&self, p_mech_pu: f64) -> Option<f64> {
        if !p_mech_pu.is_finite() || !(self.p_max_pu > 0.0) {
            return None;
        }
        let ratio = p_mech_pu / self.p_max_pu;
        if !ratio.is_finite() || ratio.abs() >= 1.0 {
            return None;
        }
        let angle = crate::math::asin(ratio);
        if angle.is_finite() && angle.abs() < core::f64::consts::FRAC_PI_2 {
            Some(angle)
        } else {
            None
        }
    }

    /// Gap from the nominal stable equilibrium to `π − δ_s`, at zero speed.
    #[must_use]
    pub fn synchronism_barrier_pu(&self) -> f64 {
        let Some(delta_s) = self.equilibrium_delta_rad(self.nominal_p_mech_pu) else {
            return f64::NAN;
        };
        let delta_u = core::f64::consts::PI - delta_s;
        transient_energy_pu(self, self.nominal_p_mech_pu, delta_u, 0.0)
            - transient_energy_pu(self, self.nominal_p_mech_pu, delta_s, 0.0)
    }
}

fn electrical_omega_rad_s() -> f64 {
    2.0 * core::f64::consts::PI * NOMINAL_FREQUENCY_HZ
}

/// `½ M δω² − P_m δ − P_max cos δ`.
fn transient_energy_pu(
    machine: &SwingMachine,
    p_mech_pu: f64,
    delta_rad: f64,
    delta_omega_rad_s: f64,
) -> f64 {
    0.5 * machine.inertia_m() * delta_omega_rad_s * delta_omega_rad_s
        - p_mech_pu * delta_rad
        - machine.p_max_pu * crate::math::cos(delta_rad)
}

/// Integrated machine. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwingState {
    pub t_s: f64,
    pub delta_rad: f64,
    pub delta_omega_rad_s: f64,
    pub pll_err_rad: f64,
    pub p_mech_pu: f64,
    pub p_elec_pu: f64,
    pub transient_energy_pu: f64,
    pub energy_anchor_pu: f64,
    pub boundary_work_pu: f64,
    /// Signed `U − U_anchor − W`. Positive is created energy.
    pub energy_defect_pu: f64,
    pub residual_conservation_error: f64,
    pub synchronism_margin_rad: f64,
    pub pll_slipped: bool,
}

impl SwingState {
    /// Nominal dispatch, stable equilibrium, PLL locked.
    #[must_use]
    pub fn try_rest(machine: &SwingMachine) -> Option<Self> {
        let delta = machine.equilibrium_delta_rad(NOMINAL_P_MECH_PU)?;
        Self::try_phase(machine, delta, 0.0, 0.0)
    }

    /// Anchor the ledger at this angle, speed, and PLL error. Dispatch starts at nominal.
    #[must_use]
    pub fn try_phase(
        machine: &SwingMachine,
        delta_rad: f64,
        delta_omega_rad_s: f64,
        pll_err_rad: f64,
    ) -> Option<Self> {
        Self::try_dispatched(
            machine,
            delta_rad,
            delta_omega_rad_s,
            pll_err_rad,
            NOMINAL_P_MECH_PU,
        )
    }

    /// Anchor the ledger at this angle, speed, PLL error, and dispatch.
    ///
    /// A first step held at `p_mech_pu` then books no change of dispatch:
    /// the anchor is the transient energy under the dispatch the run starts
    /// with, and the boundary work starts at zero. A dispatch beyond the
    /// card's limit, or not finite, has no state.
    #[must_use]
    pub fn try_dispatched(
        machine: &SwingMachine,
        delta_rad: f64,
        delta_omega_rad_s: f64,
        pll_err_rad: f64,
        p_mech_pu: f64,
    ) -> Option<Self> {
        if !machine.is_valid()
            || !delta_rad.is_finite()
            || !delta_omega_rad_s.is_finite()
            || !pll_err_rad.is_finite()
            || !p_mech_pu.is_finite()
            || p_mech_pu.abs() > machine.dispatch_limit_pu
            || delta_rad.abs() >= core::f64::consts::FRAC_PI_2
        {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            delta_rad,
            delta_omega_rad_s,
            pll_err_rad,
            p_mech_pu,
            p_elec_pu: 0.0,
            transient_energy_pu: 0.0,
            energy_anchor_pu: 0.0,
            boundary_work_pu: 0.0,
            energy_defect_pu: 0.0,
            residual_conservation_error: 0.0,
            synchronism_margin_rad: 0.0,
            pll_slipped: false,
        };
        refresh(machine, &mut state);
        if !state_is_finite(&state) {
            return None;
        }
        state.energy_anchor_pu = state.transient_energy_pu;
        state.boundary_work_pu = 0.0;
        state.energy_defect_pu = 0.0;
        state.residual_conservation_error = 0.0;
        Some(state)
    }
}

/// Advance one RK4 step and write the frame.
///
/// [`StepReject::LossOfSynchronism`] refuses a landed angle at or beyond ±π/2.
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    machine: &SwingMachine,
    state: &mut SwingState,
    frame: &mut LastStateFrame64,
    load: SwingLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(machine, state, load, dt_s)?;
    let predicted = match rk4(machine, pack(state), dt_s, load.p_mech_pu) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(machine, state, frame, load, dt_s, predicted)
}

fn validate(
    machine: &SwingMachine,
    state: &SwingState,
    load: SwingLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.p_mech_pu.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !machine.is_valid() {
        return Err(StepReject::InvalidMachine);
    }
    if state.delta_rad.abs() >= core::f64::consts::FRAC_PI_2 {
        return Err(StepReject::LossOfSynchronism);
    }
    if state.delta_omega_rad_s.abs() > machine.speed_deviation_limit_rad_s {
        return Err(StepReject::SpeedDeviationOutsideCard);
    }
    if state.p_mech_pu.abs() > machine.dispatch_limit_pu
        || load.p_mech_pu.abs() > machine.dispatch_limit_pu
    {
        return Err(StepReject::DispatchOutsideCard);
    }
    Ok(())
}

fn commit(
    machine: &SwingMachine,
    state: &mut SwingState,
    frame: &mut LastStateFrame64,
    load: SwingLoad,
    dt_s: f64,
    predicted: [f64; 4],
) -> Result<(), StepReject> {
    let saved = *state;
    let energy_old = transient_energy_pu(
        machine,
        state.p_mech_pu,
        state.delta_rad,
        state.delta_omega_rad_s,
    );
    let energy_new_dispatch = transient_energy_pu(
        machine,
        load.p_mech_pu,
        state.delta_rad,
        state.delta_omega_rad_s,
    );
    let jump = energy_new_dispatch - energy_old;
    if !jump.is_finite() || !finite4(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    if predicted[0].abs() >= core::f64::consts::FRAC_PI_2 {
        return Err(StepReject::LossOfSynchronism);
    }
    if predicted[1].abs() > machine.speed_deviation_limit_rad_s {
        return Err(StepReject::SpeedDeviationOutsideCard);
    }
    state.delta_rad = predicted[0];
    state.delta_omega_rad_s = predicted[1];
    state.pll_err_rad = predicted[2];
    state.boundary_work_pu = predicted[3] + jump;
    state.p_mech_pu = load.p_mech_pu;
    state.t_s += dt_s;
    refresh(machine, state);
    let scale = machine.synchronism_barrier_pu();
    if !(scale > 0.0 && scale.is_finite()) {
        *state = saved;
        return Err(StepReject::InvalidMachine);
    }
    state.energy_defect_pu =
        state.transient_energy_pu - state.energy_anchor_pu - state.boundary_work_pu;
    state.residual_conservation_error = state.energy_defect_pu.abs() / scale;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    let gate = RESIDUAL_CONSERVATION_LIMIT * scale;
    if state.energy_defect_pu > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_pu < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    publish(state, frame);
    Ok(())
}

fn refresh(machine: &SwingMachine, state: &mut SwingState) {
    state.p_elec_pu = machine.p_max_pu * crate::math::sin(state.delta_rad);
    state.transient_energy_pu = transient_energy_pu(
        machine,
        state.p_mech_pu,
        state.delta_rad,
        state.delta_omega_rad_s,
    );
    state.synchronism_margin_rad = core::f64::consts::FRAC_PI_2 - state.delta_rad.abs();
    state.pll_slipped = state.pll_err_rad.abs() >= machine.pll_trip_rad;
}

fn publish(state: &SwingState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.delta_rad as f32;
    frame.pos[1] = state.delta_omega_rad_s as f32;
    frame.pos[2] = state.pll_err_rad as f32;
    frame.vel[0] = state.p_elec_pu as f32;
    frame.vel[1] = state.p_mech_pu as f32;
    frame.vel[2] = state.transient_energy_pu as f32;
    frame.force_torque = state.synchronism_margin_rad as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.pll_slipped { FLAG_PLL_SLIP } else { 0 };
}

fn derivatives(machine: &SwingMachine, y: [f64; 4], p_mech_pu: f64) -> Option<[f64; 4]> {
    let delta_rad = y[0];
    let delta_omega = y[1];
    let pll_err = y[2];
    let inertia = machine.inertia_m();
    if !(inertia.is_finite() && inertia > 0.0) {
        return None;
    }
    let p_elec = machine.p_max_pu * crate::math::sin(delta_rad);
    let acceleration = (p_mech_pu - p_elec - machine.damping_pu * delta_omega) / inertia;
    // PLL error rides in this stage. It is not a split update after the rotor.
    let d_pll = delta_omega - machine.k_pll_per_s * pll_err;
    let d_work = -machine.damping_pu * delta_omega * delta_omega;
    let out = [delta_omega, acceleration, d_pll, d_work];
    if finite4(out) { Some(out) } else { None }
}

fn rk4(machine: &SwingMachine, y: [f64; 4], dt: f64, p_mech_pu: f64) -> Option<[f64; 4]> {
    let k1 = derivatives(machine, y, p_mech_pu)?;
    let k2 = derivatives(machine, axpy(y, dt * 0.5, k1), p_mech_pu)?;
    let k3 = derivatives(machine, axpy(y, dt * 0.5, k2), p_mech_pu)?;
    let k4 = derivatives(machine, axpy(y, dt, k3), p_mech_pu)?;
    let mut next = y;
    let mut i = 0;
    while i < 4 {
        let increment = k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i];
        // A zero rate times an infinite `dt` is 0. IEEE 754 would make that a NaN.
        if increment != 0.0 {
            next[i] += (dt / 6.0) * increment;
        }
        i += 1;
    }
    if finite4(next) { Some(next) } else { None }
}

fn axpy(y: [f64; 4], scale: f64, k: [f64; 4]) -> [f64; 4] {
    [
        mul_dt(y[0], scale, k[0]),
        mul_dt(y[1], scale, k[1]),
        mul_dt(y[2], scale, k[2]),
        mul_dt(y[3], scale, k[3]),
    ]
}

fn mul_dt(y: f64, scale: f64, k: f64) -> f64 {
    if k == 0.0 { y } else { y + scale * k }
}

fn pack(state: &SwingState) -> [f64; 4] {
    [
        state.delta_rad,
        state.delta_omega_rad_s,
        state.pll_err_rad,
        state.boundary_work_pu,
    ]
}

fn finite4(x: [f64; 4]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite() && x[3].is_finite()
}

fn state_is_finite(state: &SwingState) -> bool {
    state.t_s.is_finite()
        && state.delta_rad.is_finite()
        && state.delta_omega_rad_s.is_finite()
        && state.pll_err_rad.is_finite()
        && state.p_mech_pu.is_finite()
        && state.p_elec_pu.is_finite()
        && state.transient_energy_pu.is_finite()
        && state.energy_anchor_pu.is_finite()
        && state.boundary_work_pu.is_finite()
        && state.energy_defect_pu.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.synchronism_margin_rad.is_finite()
}

/// Explicit Euler on `(δ, δω, φ, W)`.
#[cfg(test)]
fn step_euler(
    machine: &SwingMachine,
    state: &mut SwingState,
    frame: &mut LastStateFrame64,
    load: SwingLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(machine, state, load, dt_s)?;
    let predicted = match explicit_euler(machine, pack(state), dt_s, load.p_mech_pu) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(machine, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
fn explicit_euler(
    machine: &SwingMachine,
    y: [f64; 4],
    dt: f64,
    p_mech_pu: f64,
) -> Option<[f64; 4]> {
    let k1 = derivatives(machine, y, p_mech_pu)?;
    let next = axpy(y, dt, k1);
    if finite4(next) { Some(next) } else { None }
}

/// Add `extra_pu` to the integrated work after a faithful RK4 step.
#[cfg(test)]
fn step_bias_work(
    machine: &SwingMachine,
    state: &mut SwingState,
    frame: &mut LastStateFrame64,
    load: SwingLoad,
    dt_s: f64,
    extra_pu: f64,
) -> Result<(), StepReject> {
    validate(machine, state, load, dt_s)?;
    let mut predicted = match rk4(machine, pack(state), dt_s, load.p_mech_pu) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    predicted[3] += extra_pu;
    commit(machine, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        FLAG_PLL_SLIP, K_PLL_PER_S, NOMINAL_FREQUENCY_HZ, NOMINAL_P_MECH_PU, PLL_TRIP_RAD,
        RESIDUAL_CONSERVATION_LIMIT, StepReject, SwingLoad, SwingMachine, SwingState, step,
        step_bias_work, step_euler,
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

    #[test]
    fn gate_comes_from_the_electrical_frequency() {
        assert_copy::<SwingMachine>();
        assert_copy::<SwingState>();
        assert_copy::<SwingLoad>();
        assert_copy::<StepReject>();
        let machine = SwingMachine::synchronous_60hz();
        let omega = 2.0 * core::f64::consts::PI * NOMINAL_FREQUENCY_HZ;
        let gate = machine.gate_dt_s().unwrap();
        assert_eq!(gate.to_bits(), (0.02 / omega).to_bits());
        assert!(machine.nyquist_dt_s().unwrap() > gate);
        assert!(machine.synchronism_barrier_pu() > 0.0);
        let delta = machine.equilibrium_delta_rad(NOMINAL_P_MECH_PU).unwrap();
        assert!(delta > 1.0 && delta < core::f64::consts::FRAC_PI_2);
        let mut tampered = machine;
        tampered.inertia_constant_s = 2.0;
        assert!(!tampered.is_valid());
    }

    #[test]
    fn pll_inside_rk4_matches_the_exponential_not_euler() {
        let machine = SwingMachine::synchronous_60hz();
        let mut state = SwingState::try_rest(&machine).unwrap();
        state.pll_err_rad = 0.2;
        let anchor_delta = state.delta_rad;
        let mut frame = blank_frame();
        let dt = 0.05;
        assert!(dt > machine.nyquist_dt_s().unwrap());
        step(&machine, &mut state, &mut frame, SwingLoad::default(), dt).unwrap();
        let exact = 0.2 * (-K_PLL_PER_S * dt).exp();
        let euler = 0.2 * (1.0 - K_PLL_PER_S * dt);
        assert!((state.pll_err_rad - exact).abs() < 2.0e-3);
        assert!((state.pll_err_rad - exact).abs() < (euler - exact).abs());
        assert!((state.delta_rad - anchor_delta).abs() < 1.0e-12);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);

        let mut fine = SwingState::try_rest(&machine).unwrap();
        fine.pll_err_rad = 0.2;
        let gate = machine.gate_dt_s().unwrap();
        step(&machine, &mut fine, &mut frame, SwingLoad::default(), gate).unwrap();
        let expected = 0.2 * (-K_PLL_PER_S * gate).exp();
        assert!((fine.pll_err_rad - expected).abs() < 1.0e-12);
    }

    #[test]
    fn dispatch_step_stays_inside_the_barrier() {
        let machine = SwingMachine::synchronous_60hz();
        let mut state = SwingState::try_rest(&machine).unwrap();
        let mut frame = blank_frame();
        let load = SwingLoad { p_mech_pu: 0.7 };
        let dt = machine.gate_dt_s().unwrap();
        let mut max_residual = 0.0;
        let mut n = 0u32;
        while n < 2_000 {
            step(&machine, &mut state, &mut frame, load, dt).unwrap();
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            n += 1;
        }
        assert!(
            max_residual <= RESIDUAL_CONSERVATION_LIMIT,
            "residual {max_residual}"
        );
        assert!(state.delta_rad.abs() < core::f64::consts::FRAC_PI_2);
        assert_eq!(state.p_mech_pu.to_bits(), 0.7_f64.to_bits());
        proof_intact(&frame);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
    }

    #[test]
    fn loss_of_synchronism_rolls_back() {
        let machine = SwingMachine::synchronous_60hz();
        let dt = machine.gate_dt_s().unwrap();
        let mut state =
            SwingState::try_phase(&machine, core::f64::consts::FRAC_PI_2 - 1.0e-6, 1.0, 0.0)
                .unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(&machine, &mut state, &mut frame, SwingLoad::default(), dt);
        assert_eq!(got, Err(StepReject::LossOfSynchronism));
        assert_eq!(state, saved);
        assert_eq!(frame.t, -1.0);
        proof_intact(&frame);
        assert!(SwingState::try_phase(&machine, core::f64::consts::FRAC_PI_2, 0.0, 0.0).is_none());
    }

    #[test]
    fn pll_slip_flags_without_refusing() {
        let machine = SwingMachine::synchronous_60hz();
        let mut state = SwingState::try_rest(&machine).unwrap();
        state.pll_err_rad = PLL_TRIP_RAD + 0.05;
        let mut frame = blank_frame();
        step(
            &machine,
            &mut state,
            &mut frame,
            SwingLoad::default(),
            machine.gate_dt_s().unwrap(),
        )
        .unwrap();
        assert!(state.pll_slipped);
        assert_eq!(frame.flags, FLAG_PLL_SLIP);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    #[test]
    fn euler_from_a_displaced_rotor_creates_energy() {
        let machine = SwingMachine::synchronous_60hz();
        let delta = machine.equilibrium_delta_rad(NOMINAL_P_MECH_PU).unwrap() + 0.15;
        let mut state = SwingState::try_phase(&machine, delta, 0.0, 0.0).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step_euler(&machine, &mut state, &mut frame, SwingLoad::default(), 0.01);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        proof_intact(&frame);

        let mut rk = SwingState::try_phase(&machine, delta, 0.0, 0.0).unwrap();
        step(&machine, &mut rk, &mut frame, SwingLoad::default(), 0.01).unwrap();
        assert!(rk.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    #[test]
    fn biased_work_and_non_finite_inputs_roll_back() {
        let machine = SwingMachine::synchronous_60hz();
        let mut state = SwingState::try_rest(&machine).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let dt = machine.gate_dt_s().unwrap();
        let extra = machine.synchronism_barrier_pu() * 1.0e-3;
        let got = step_bias_work(
            &machine,
            &mut state,
            &mut frame,
            SwingLoad::default(),
            dt,
            extra,
        );
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state, saved);
        let got = step_bias_work(
            &machine,
            &mut state,
            &mut frame,
            SwingLoad::default(),
            dt,
            -extra,
        );
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        let mut refuse = |load: SwingLoad, dt_s: f64, expect: StepReject| {
            let got = step(&machine, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state, saved);
            proof_intact(&frame);
        };
        refuse(SwingLoad::default(), 0.0, StepReject::NonPositiveDt);
        refuse(SwingLoad::default(), -dt, StepReject::NonPositiveDt);
        refuse(SwingLoad::default(), f64::NAN, StepReject::NonFiniteInput);
        refuse(
            SwingLoad {
                p_mech_pu: f64::NAN,
            },
            dt,
            StepReject::NonFiniteInput,
        );
        refuse(
            SwingLoad {
                p_mech_pu: f64::MAX,
            },
            dt,
            StepReject::DispatchOutsideCard,
        );
        state.delta_omega_rad_s = f64::NAN;
        let got = step(&machine, &mut state, &mut frame, SwingLoad::default(), dt);
        assert_eq!(got, Err(StepReject::NonFiniteState));
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::SwingMachine::synchronous_60hz();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The rotor 0.2 rad behind its rest angle, moving at 0.5 rad/s, at nominal power.
        let mut state = {
            let rest = super::SwingState::try_rest(&card).unwrap();
            super::SwingState::try_phase(&card, rest.delta_rad - 0.2, 0.5, 0.05).unwrap()
        };
        let load = super::SwingLoad {
            p_mech_pu: super::NOMINAL_P_MECH_PU,
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
        assert!(state.delta_rad != before.delta_rad);
        assert!(state.delta_omega_rad_s != before.delta_omega_rad_s);
        assert!(state.pll_err_rad != before.pll_err_rad);
        assert!(state.boundary_work_pu != before.boundary_work_pu);
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

    fn state_bits(state: &SwingState) -> [u64; 13] {
        [
            state.t_s.to_bits(),
            state.delta_rad.to_bits(),
            state.delta_omega_rad_s.to_bits(),
            state.pll_err_rad.to_bits(),
            state.p_mech_pu.to_bits(),
            state.p_elec_pu.to_bits(),
            state.transient_energy_pu.to_bits(),
            state.energy_anchor_pu.to_bits(),
            state.boundary_work_pu.to_bits(),
            state.energy_defect_pu.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.synchronism_margin_rad.to_bits(),
            u64::from(state.pll_slipped),
        ]
    }

    /// Step, and require this refusal with the state and the frame bytes as they were.
    fn refused_whole(
        machine: &SwingMachine,
        state: &mut SwingState,
        frame: &mut LastStateFrame64,
        load: SwingLoad,
        dt_s: f64,
        expect: StepReject,
    ) {
        let held = state_bits(state);
        let held_frame = frame.to_bytes();
        let got = step(machine, state, frame, load, dt_s);
        assert_eq!(got, Err(expect));
        assert_eq!(state_bits(state), held);
        assert_eq!(frame.to_bytes(), held_frame);
    }

    #[test]
    fn the_speed_limit_lands_and_the_next_number_past_it_is_refused() {
        let machine = SwingMachine::synchronous_60hz();
        let dt = machine.gate_dt_s().unwrap();
        let limit = super::SPEED_DEVIATION_LIMIT_RAD_S;
        assert_eq!(
            machine.speed_deviation_limit_rad_s.to_bits(),
            limit.to_bits()
        );
        // 3 Hz, a twentieth of synchronous speed.
        assert!((limit - 0.05 * machine.omega_electrical_rad_s).abs() < 1.0e-12);
        let coasting = SwingLoad { p_mech_pu: 0.0 };

        // A rotor on the limit itself, with no dispatch, slows under damping: the step lands.
        let mut sign = 1.0;
        while sign >= -1.0 {
            let mut state = SwingState::try_phase(&machine, 0.0, sign * limit, 0.0).unwrap();
            let mut frame = blank_frame();
            step(&machine, &mut state, &mut frame, coasting, dt).unwrap();
            assert!(state.delta_omega_rad_s.abs() < limit);
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            proof_intact(&frame);

            // The next representable speed is outside the card.
            let mut over =
                SwingState::try_phase(&machine, 0.0, sign * limit.next_up(), 0.0).unwrap();
            let mut frame = blank_frame();
            refused_whole(
                &machine,
                &mut over,
                &mut frame,
                coasting,
                dt,
                StepReject::SpeedDeviationOutsideCard,
            );
            proof_intact(&frame);
            sign -= 2.0;
        }

        // A landing: under a dispatch of 2 pu the rotor gains speed. Find the
        // two neighbouring starting speeds of which one lands and the next is refused.
        let driven = SwingLoad { p_mech_pu: 2.0 };
        let lands = |speed_rad_s: f64| {
            let mut state = SwingState::try_phase(&machine, 0.0, speed_rad_s, 0.0).unwrap();
            let mut frame = blank_frame();
            match step(&machine, &mut state, &mut frame, driven, dt) {
                Ok(()) => Some(state.delta_omega_rad_s),
                Err(StepReject::SpeedDeviationOutsideCard) => None,
                Err(other) => std::panic!("speed {speed_rad_s} refused with {other:?}"),
            }
        };
        let mut inside = limit - 0.01;
        let mut outside = limit;
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
        let landed = lands(inside).unwrap();
        assert!(landed > inside && landed <= limit);
        assert!(limit - landed <= 8.0 * (limit.next_up() - limit));
        let mut state = SwingState::try_phase(&machine, 0.0, outside, 0.0).unwrap();
        let mut frame = blank_frame();
        refused_whole(
            &machine,
            &mut state,
            &mut frame,
            driven,
            dt,
            StepReject::SpeedDeviationOutsideCard,
        );
        proof_intact(&frame);

        // The recorded copy is locked like the other constants.
        let mut tampered = machine;
        tampered.speed_deviation_limit_rad_s = 1.0e6;
        assert!(!tampered.is_valid());
        refused_whole(
            &tampered,
            &mut state,
            &mut frame,
            coasting,
            dt,
            StepReject::InvalidMachine,
        );
    }

    #[test]
    fn the_dispatch_limit_lands_and_the_next_number_past_it_is_refused() {
        let machine = SwingMachine::synchronous_60hz();
        let dt = machine.gate_dt_s().unwrap();
        let limit = super::DISPATCH_LIMIT_PU;
        assert_eq!(machine.dispatch_limit_pu.to_bits(), limit.to_bits());
        assert_eq!(limit, 2.0 * NOMINAL_P_MECH_PU);
        let mut sign = 1.0;
        while sign >= -1.0 {
            let mut state = SwingState::try_rest(&machine).unwrap();
            let mut frame = blank_frame();
            let on_limit = SwingLoad {
                p_mech_pu: sign * limit,
            };
            step(&machine, &mut state, &mut frame, on_limit, dt).unwrap();
            assert_eq!(state.p_mech_pu.to_bits(), (sign * limit).to_bits());
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            let past = SwingLoad {
                p_mech_pu: sign * limit.next_up(),
            };
            refused_whole(
                &machine,
                &mut state,
                &mut frame,
                past,
                dt,
                StepReject::DispatchOutsideCard,
            );
            proof_intact(&frame);
            sign -= 2.0;
        }

        // A dispatch already held past the limit is refused the same way.
        let mut state = SwingState::try_rest(&machine).unwrap();
        state.p_mech_pu = limit.next_up();
        let mut frame = blank_frame();
        refused_whole(
            &machine,
            &mut state,
            &mut frame,
            SwingLoad::default(),
            dt,
            StepReject::DispatchOutsideCard,
        );

        let mut tampered = machine;
        tampered.dispatch_limit_pu = 1.0e7;
        assert!(!tampered.is_valid());
        let mut state = SwingState::try_rest(&machine).unwrap();
        refused_whole(
            &tampered,
            &mut state,
            &mut frame,
            SwingLoad::default(),
            dt,
            StepReject::InvalidMachine,
        );
        proof_intact(&frame);
    }

    /// Runs that used to end in a conservation refusal. At 150, 1,000 and
    /// 3,000 rad/s with steps of 1e-12 s the card landed 611,014, 13,727 and
    /// 1,450 steps and then refused as destroyed or created energy, through
    /// rounding. 100,000 rad/s was refused that way on the first step, and a
    /// dispatch of 1e7 pu after 531 steps.
    #[test]
    fn speeds_and_dispatches_that_ended_in_a_conservation_refusal_are_refused_by_name_before_the_ledger() {
        let machine = SwingMachine::synchronous_60hz();
        let gate = machine.gate_dt_s().unwrap();
        let mut frame = blank_frame();
        let speeds = [
            (150.0, 1.0e-12),
            (1_000.0, 1.0e-12),
            (3_000.0, 1.0e-12),
            (1.0e5, 1.0e-9),
            (3_000.0, gate),
            (1.0e4, gate),
            (-150.0, machine.floor_dt_s().unwrap()),
        ];
        let mut i = 0;
        while i < speeds.len() {
            let (speed_rad_s, dt_s) = speeds[i];
            let mut state = SwingState::try_phase(&machine, 0.0, speed_rad_s, 0.0).unwrap();
            refused_whole(
                &machine,
                &mut state,
                &mut frame,
                SwingLoad::default(),
                dt_s,
                StepReject::SpeedDeviationOutsideCard,
            );
            i += 1;
        }
        let dispatches = [
            (1.0e7, 1.0e-9),
            (3.0e4, gate),
            (1.0e4, 1.0e-9),
            (-1.0e4, gate),
        ];
        let mut i = 0;
        while i < dispatches.len() {
            let (p_mech_pu, dt_s) = dispatches[i];
            let mut state = SwingState::try_rest(&machine).unwrap();
            refused_whole(
                &machine,
                &mut state,
                &mut frame,
                SwingLoad { p_mech_pu },
                dt_s,
                StepReject::DispatchOutsideCard,
            );
            i += 1;
        }
        proof_intact(&frame);
    }

    /// At the dispatch limit the speed limit is not what stops a rotor
    /// released at rest: it leaves the ±π/2 window first.
    #[test]
    fn from_rest_the_rotor_loses_synchronism_before_it_reaches_the_speed_limit() {
        let machine = SwingMachine::synchronous_60hz();
        let dt = machine.gate_dt_s().unwrap();
        let limit = super::SPEED_DEVIATION_LIMIT_RAD_S;
        let edge = core::f64::consts::FRAC_PI_2 - 1.0e-4;
        let rest = SwingState::try_rest(&machine).unwrap().delta_rad;
        let starts = [-edge, -1.0, 0.0, rest, edge];
        let mut fastest: f64 = 0.0;
        let mut sign = 1.0;
        while sign >= -1.0 {
            let load = SwingLoad {
                p_mech_pu: sign * super::DISPATCH_LIMIT_PU,
            };
            let mut i = 0;
            while i < starts.len() {
                let mut state = SwingState::try_phase(&machine, starts[i], 0.0, 0.0).unwrap();
                let mut frame = blank_frame();
                let mut landed = 0u32;
                loop {
                    match step(&machine, &mut state, &mut frame, load, dt) {
                        Ok(()) => {
                            landed += 1;
                            assert!(landed < 100_000, "the rotor stayed in the window");
                            fastest = fastest.max(state.delta_omega_rad_s.abs());
                            assert!(
                                state.residual_conservation_error
                                    <= RESIDUAL_CONSERVATION_LIMIT / 1_000.0
                            );
                        }
                        Err(StepReject::LossOfSynchronism) => break,
                        Err(other) => std::panic!("refused with {other:?} after {landed} steps"),
                    }
                }
                i += 1;
            }
            sign -= 2.0;
        }
        // Measured 17.5 rad/s, from the far edge of the window.
        assert!(fastest > 16.0 && fastest < limit, "fastest {fastest}");
    }

    /// The hardest case found for the ledger's rounding inside the card: a
    /// rotor just under the speed limit, at the dispatch limit, stepped at the
    /// step floor until it leaves the window, about 836,000 steps.
    #[test]
    fn steps_at_the_floor_at_both_limits_stay_far_inside_the_conservation_limit() {
        let machine = SwingMachine::synchronous_60hz();
        let dt = machine.floor_dt_s().unwrap();
        let load = SwingLoad {
            p_mech_pu: super::DISPATCH_LIMIT_PU,
        };
        let speed = 0.999 * super::SPEED_DEVIATION_LIMIT_RAD_S;
        let mut state = SwingState::try_phase(&machine, 1.57, speed, 0.0).unwrap();
        let mut frame = blank_frame();
        let mut max_residual = 0.0;
        let mut landed = 0u32;
        loop {
            match step(&machine, &mut state, &mut frame, load, dt) {
                Ok(()) => {
                    landed += 1;
                    assert!(landed <= 1_000_000);
                    if state.residual_conservation_error > max_residual {
                        max_residual = state.residual_conservation_error;
                    }
                }
                Err(StepReject::LossOfSynchronism) => break,
                Err(other) => std::panic!("refused with {other:?} after {landed} steps"),
            }
        }
        assert!(landed > 800_000, "landed {landed}");
        // Measured 8.9e-9. The limit is 1.0e-6.
        assert!(max_residual < 1.0e-7, "residual {max_residual}");
        proof_intact(&frame);
    }

    /// A start held at its own dispatch books no work on its first step: the
    /// anchor is the transient energy at that dispatch, and the defect and
    /// the rotor motion are those of a start at nominal.
    #[test]
    fn a_start_at_its_own_dispatch_books_no_change_of_dispatch() {
        let machine = SwingMachine::synchronous_60hz();
        let dt = machine.gate_dt_s().unwrap();
        let load = SwingLoad { p_mech_pu: 0.9 };
        let mut own = SwingState::try_dispatched(&machine, 1.0, 0.0, 0.0, 0.9).unwrap();
        assert_eq!(own.p_mech_pu.to_bits(), 0.9_f64.to_bits());
        assert_eq!(
            own.energy_anchor_pu.to_bits(),
            super::transient_energy_pu(&machine, 0.9, 1.0, 0.0).to_bits()
        );
        assert_eq!(own.boundary_work_pu.to_bits(), 0.0_f64.to_bits());
        let mut nominal = SwingState::try_phase(&machine, 1.0, 0.0, 0.0).unwrap();
        let mut frame = blank_frame();
        step(&machine, &mut own, &mut frame, load, dt).unwrap();
        let mut other = blank_frame();
        step(&machine, &mut nominal, &mut other, load, dt).unwrap();
        // The 0.9 pu run books only the damping of its first step from rest;
        // the run anchored at nominal books the 0.1 pu change of dispatch too.
        assert!(own.boundary_work_pu.abs() < 1.0e-5, "work {}", own.boundary_work_pu);
        assert!((nominal.boundary_work_pu - 0.1).abs() < 1.0e-5);
        assert_eq!(own.delta_rad.to_bits(), nominal.delta_rad.to_bits());
        assert_eq!(own.delta_omega_rad_s.to_bits(), nominal.delta_omega_rad_s.to_bits());
        assert!(own.residual_conservation_error < 1.0e-12);
        proof_intact(&frame);
    }

    #[test]
    fn a_start_beyond_the_dispatch_limit_has_no_state() {
        let machine = SwingMachine::synchronous_60hz();
        let limit = super::DISPATCH_LIMIT_PU;
        assert!(SwingState::try_dispatched(&machine, 0.0, 0.0, 0.0, limit).is_some());
        assert!(SwingState::try_dispatched(&machine, 0.0, 0.0, 0.0, -limit).is_some());
        assert!(SwingState::try_dispatched(&machine, 0.0, 0.0, 0.0, limit.next_up()).is_none());
        assert!(SwingState::try_dispatched(&machine, 0.0, 0.0, 0.0, f64::NAN).is_none());
    }

    /// The hardest case at its own dispatch: anchored at the dispatch limit,
    /// just under the speed limit, stepped at the floor until it leaves the window.
    #[test]
    fn steps_at_the_floor_anchored_at_the_dispatch_limit_stay_inside_the_conservation_limit() {
        let machine = SwingMachine::synchronous_60hz();
        let dt = machine.floor_dt_s().unwrap();
        let load = SwingLoad {
            p_mech_pu: super::DISPATCH_LIMIT_PU,
        };
        let speed = 0.999 * super::SPEED_DEVIATION_LIMIT_RAD_S;
        let mut state =
            SwingState::try_dispatched(&machine, 1.57, speed, 0.0, super::DISPATCH_LIMIT_PU)
                .unwrap();
        let mut frame = blank_frame();
        let mut max_residual = 0.0;
        let mut landed = 0u32;
        loop {
            match step(&machine, &mut state, &mut frame, load, dt) {
                Ok(()) => {
                    landed += 1;
                    assert!(landed <= 1_000_000);
                    if state.residual_conservation_error > max_residual {
                        max_residual = state.residual_conservation_error;
                    }
                }
                Err(StepReject::LossOfSynchronism) => break,
                Err(other) => std::panic!("refused with {other:?} after {landed} steps"),
            }
        }
        assert!(landed > 800_000, "landed {landed}");
        assert!(max_residual < 1.0e-7, "residual {max_residual}");
        proof_intact(&frame);
    }
}
