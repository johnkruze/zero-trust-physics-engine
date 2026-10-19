#![no_implicit_prelude]

//! 12S lithium-ion pack under a held discharge current.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, terminal voltage, state of charge, current,
//! open-circuit voltage, chemical energy, Joule heat, the margin to cutoff,
//! and the conservation residual into [`LastStateFrame64`]. The 16-byte
//! frame proof is not touched.
//!
//! The campaign pack is 12 series cells, 22 Ah, 50.4 V full and 36.0 V
//! empty. Warning is 42.0 V. Cutoff is 36.0 V. Internal resistance is the
//! 0.060 Ω organ point. State of charge is a fraction in `[0, 1]`. The
//! campaign clamp that pinned SoC inside 1%…100% before evaluating `R` is
//! not applied.
//!
//! ```text
//! d s/dt = −I / Q
//! V_oc = V_empty + (V_full − V_empty) s
//! R = R0 (2 − s)
//! V = V_oc − I R
//! E = Q (V_empty s + (V_full − V_empty) s² / 2)
//! dW_elec/dt = V I
//! dW_joule/dt = I² R
//! residual_conservation_error = |E − E_anchor + W_elec + W_joule| / E_pack
//! ```
//!
//! `I > 0` discharges. `E_pack` is the chemical energy from empty to full
//! on the linear open-circuit line. Under a held current the rates are
//! polynomials of degree 1, so classical RK4 lands on the integral.
//!
//! [`LiIonPack::gate_dt_s`] is `ω Δt = 0.02` with `ω = 1 / (R0 C)` and
//! `C = Q / (V_full − V_empty)`. The public step does not apply that clock.
//! The current limit is applied: `|I| Δt ≤ (V_full − V_empty) / R0`. Past
//! it the step is [`StepReject::CurrentAboveNyquist`]. A landing at or
//! below 36 V is [`StepReject::UndervoltageCutoff`]. A landing above 50.4 V,
//! which only a charging current can produce, is
//! [`StepReject::OvervoltageCutoff`]. A charging current from a full pack is
//! [`StepReject::OvervoltageCutoff`] at any size, before the step is taken.
//! A landing outside
//! `[0, 1]` is [`StepReject::StateOfChargeOutsideCard`]. Terminal voltage
//! below 42 V and above cutoff sets [`FLAG_SAG_WARNING`] and the step
//! still lands.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Series count named by the campaign. The pack voltages below are the locked literals.
pub const SERIES_CELLS: f64 = 12.0;
/// Campaign capacity.
pub const CAPACITY_AH: f64 = 22.0;
/// `22 Ah` in coulombs.
pub const CAPACITY_C: f64 = CAPACITY_AH * 3_600.0;
/// Locked series resistance. Organ test point.
pub const INTERNAL_RESISTANCE_OHM: f64 = 0.06;
/// 12S × 4.2 V.
pub const PACK_FULL_V: f64 = 50.4;
/// 12S × 3.0 V.
pub const PACK_EMPTY_V: f64 = 36.0;
/// 12S × 3.5 V. Below this the frame is flagged.
pub const PACK_WARN_V: f64 = 42.0;
/// BMS cutoff. At or below this the step is refused.
pub const PACK_CUTOFF_V: f64 = 36.0;
/// Charging limit: 12S × 4.2 V, the full voltage of the pack. A lithium-ion
/// cell rated 4.2 V full is charged until its terminals reach 4.2 V and is
/// not driven above it. A landing with the terminals above this is refused.
/// On the card's two lines that holds the charging current to
/// `(50.4 V − V_oc) / R`: 80 A at half charge, 21.8 A at 90 percent, and
/// under 120 A everywhere. Between 36 V and 50.4 V the largest rounding found
/// in the ledger over a million steps is 1.3e-10 of the pack energy, 7,700
/// times under the limit. Without the limit the card landed any charging
/// current; rounding first refused a step near 1e11 A, and at 1e19 A a step
/// landed with a reported residual of exactly zero.
pub const PACK_CHARGE_LIMIT_V: f64 = 50.4;

/// `|E − E_anchor + W_elec + W_joule| / E_pack` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Electrical phase advance `ω Δt` used by [`LiIonPack::gate_dt_s`].
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Terminal voltage is below [`PACK_WARN_V`] and above cutoff on this frame.
pub const FLAG_SAG_WARNING: u64 = 1 << 0;

/// Why a step did not advance the pack or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Current or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// Resistance, capacity, or the voltage anchors are not this pack.
    InvalidPack,
    /// `|I| dt` is past `(V_full − V_empty) / R0`.
    CurrentAboveNyquist,
    /// Terminal voltage is at or below the cutoff.
    UndervoltageCutoff,
    /// Terminal voltage is above the 50.4 V charging limit, 12S × 4.2 V. A full pack (soc 1) refuses every charging current, however small, before the step is taken.
    OvervoltageCutoff,
    /// State of charge left `[0, 1]`.
    StateOfChargeOutsideCard,
    /// The trial step created chemical energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed chemical energy.
    FirstLawBreach,
}

/// Zero-order hold. Positive current discharges the pack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackLoad {
    pub current_a: f64,
}

impl Default for PackLoad {
    fn default() -> Self {
        Self { current_a: 0.0 }
    }
}

/// Locked 12S pack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiIonPack {
    pub series_resistance_ohm: f64,
    pub capacity_c: f64,
    pub full_v: f64,
    pub empty_v: f64,
    pub warn_v: f64,
    pub cutoff_v: f64,
    /// Recorded copy of [`PACK_CHARGE_LIMIT_V`]. Not a caller input.
    pub charge_limit_v: f64,
}

impl LiIonPack {
    /// Card: 22 Ah, 0.060 Ω, 50.4 V to 36.0 V.
    #[must_use]
    pub fn pack_12s() -> Self {
        let pack = Self {
            series_resistance_ohm: INTERNAL_RESISTANCE_OHM,
            capacity_c: CAPACITY_C,
            full_v: PACK_FULL_V,
            empty_v: PACK_EMPTY_V,
            warn_v: PACK_WARN_V,
            cutoff_v: PACK_CUTOFF_V,
            charge_limit_v: PACK_CHARGE_LIMIT_V,
        };
        if pack.is_valid() {
            pack
        } else {
            ::core::panic!("12S pack failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.series_resistance_ohm.to_bits() == INTERNAL_RESISTANCE_OHM.to_bits()
            && self.capacity_c.to_bits() == CAPACITY_C.to_bits()
            && self.full_v.to_bits() == PACK_FULL_V.to_bits()
            && self.empty_v.to_bits() == PACK_EMPTY_V.to_bits()
            && self.warn_v.to_bits() == PACK_WARN_V.to_bits()
            && self.cutoff_v.to_bits() == PACK_CUTOFF_V.to_bits()
            && self.charge_limit_v.to_bits() == PACK_CHARGE_LIMIT_V.to_bits()
            && self.charge_limit_v >= self.full_v
            && self.series_resistance_ohm > 0.0
            && self.capacity_c > 0.0
            && self.full_v > self.warn_v
            && self.warn_v > self.cutoff_v
            && self.cutoff_v > 0.0
            && self.empty_v.to_bits() == self.cutoff_v.to_bits()
            && self.chemical_capacitance_f().is_finite()
            && self.chemical_capacitance_f() > 0.0
            && self.pack_energy_j().is_finite()
            && self.pack_energy_j() > 0.0
            && self.charge_impulse_c().is_finite()
            && self.charge_impulse_c() > 0.0
    }

    #[must_use]
    pub fn open_circuit_voltage(&self, soc: f64) -> f64 {
        self.empty_v + (self.full_v - self.empty_v) * soc
    }

    /// `R0 (2 − s)`, the campaign line `R0 (1 + (100 − SoC%) / 100)` with the clamp removed.
    #[must_use]
    pub fn resistance_ohm(&self, soc: f64) -> f64 {
        self.series_resistance_ohm * (2.0 - soc)
    }

    #[must_use]
    pub fn terminal_voltage(&self, soc: f64, current_a: f64) -> f64 {
        self.open_circuit_voltage(soc) - current_a * self.resistance_ohm(soc)
    }

    /// `Q / (V_full − V_empty)`.
    #[must_use]
    pub fn chemical_capacitance_f(&self) -> f64 {
        self.capacity_c / (self.full_v - self.empty_v)
    }

    /// `1 / (R0 C)`.
    #[must_use]
    pub fn pole_rad_s(&self) -> f64 {
        1.0 / (self.series_resistance_ohm * self.chemical_capacitance_f())
    }

    /// Chemical energy from empty to full.
    #[must_use]
    pub fn pack_energy_j(&self) -> f64 {
        self.chemical_energy_j(1.0)
    }

    #[must_use]
    pub fn chemical_energy_j(&self, soc: f64) -> f64 {
        let charge_c = soc * self.capacity_c;
        let window_v = self.full_v - self.empty_v;
        self.empty_v * charge_c + window_v * charge_c * soc * 0.5
    }

    /// `(V_full − V_empty) / R0`.
    #[must_use]
    pub fn charge_impulse_c(&self) -> f64 {
        (self.full_v - self.empty_v) / self.series_resistance_ohm
    }

    /// Largest reporting step of the R–C pole. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let omega = self.pole_rad_s();
        if omega.is_finite() && omega > 0.0 {
            Some(GATE_PHASE_STEP_RAD / omega)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step is two hundredths of the R–C time constant. A million times smaller, a step
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

    /// Largest `|I|` with `|I| * dt_s <= charge_impulse_c`, the product rounded
    /// once to 64 bits and compared with the constant, as [`step`] applies it.
    #[must_use]
    pub fn current_nyquist_a(&self, dt_s: f64) -> Option<f64> {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return None;
        }
        crate::impulse::largest(self.charge_impulse_c(), dt_s)
    }
}

/// Integrated pack. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackState {
    pub t_s: f64,
    pub soc: f64,
    pub current_a: f64,
    pub terminal_v: f64,
    pub open_circuit_v: f64,
    pub resistance_ohm: f64,
    pub chemical_energy_j: f64,
    pub energy_anchor_j: f64,
    pub electrical_work_j: f64,
    pub joule_j: f64,
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    pub cutoff_margin_v: f64,
    pub sag_warning: bool,
}

impl PackState {
    /// Full charge, open circuit.
    #[must_use]
    pub fn try_full(pack: &LiIonPack) -> Option<Self> {
        Self::try_soc(pack, 1.0)
    }

    /// Anchor the ledger at this state of charge, open circuit.
    #[must_use]
    pub fn try_soc(pack: &LiIonPack, soc: f64) -> Option<Self> {
        if !pack.is_valid() || !soc.is_finite() || !(soc > 0.0 && soc <= 1.0) {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            soc,
            current_a: 0.0,
            terminal_v: 0.0,
            open_circuit_v: 0.0,
            resistance_ohm: 0.0,
            chemical_energy_j: 0.0,
            energy_anchor_j: 0.0,
            electrical_work_j: 0.0,
            joule_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            cutoff_margin_v: 0.0,
            sag_warning: false,
        };
        refresh(pack, &mut state);
        if !state_is_finite(&state) || state.terminal_v <= pack.cutoff_v {
            return None;
        }
        state.energy_anchor_j = state.chemical_energy_j;
        state.electrical_work_j = 0.0;
        state.joule_j = 0.0;
        state.energy_defect_j = 0.0;
        state.residual_conservation_error = 0.0;
        Some(state)
    }
}

/// Advance one RK4 step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    pack: &LiIonPack,
    state: &mut PackState,
    frame: &mut LastStateFrame64,
    load: PackLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(pack, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("battery_sag", exceedances(pack, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    pack: &LiIonPack,
    state: &mut PackState,
    frame: &mut LastStateFrame64,
    load: PackLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(pack, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    pack: &LiIonPack,
    state: &mut PackState,
    frame: &mut LastStateFrame64,
    load: PackLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(pack, state, load, dt_s)?;
    let predicted = match rk4(pack, pack_state(state), dt_s, load.current_a) {
        Ok(y) => y,
        Err(reject) => return Err(reject),
    };
    commit(pack, state, frame, load, dt_s, predicted)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut PackState) {
    state.t_s = normal_or_zero(state.t_s);
    state.soc = normal_or_zero(state.soc);
    state.current_a = normal_or_zero(state.current_a);
    state.terminal_v = normal_or_zero(state.terminal_v);
    state.open_circuit_v = normal_or_zero(state.open_circuit_v);
    state.resistance_ohm = normal_or_zero(state.resistance_ohm);
    state.chemical_energy_j = normal_or_zero(state.chemical_energy_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.electrical_work_j = normal_or_zero(state.electrical_work_j);
    state.joule_j = normal_or_zero(state.joule_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.cutoff_margin_v = normal_or_zero(state.cutoff_margin_v);
}

fn validate(
    pack: &LiIonPack,
    state: &PackState,
    load: PackLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.current_a.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !pack.is_valid() {
        return Err(StepReject::InvalidPack);
    }
    if !(state.soc >= 0.0 && state.soc <= 1.0) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }
    if exceeds_current_nyquist(pack, load.current_a, dt_s) {
        return Err(StepReject::CurrentAboveNyquist);
    }
    // A full pack sits on the charging limit. Any charging current raises
    // the terminals above it, however small: `I R` is added to `V_full`.
    // The 64-bit sum can round back to the limit, so the rule is applied
    // here on the sign of the current and not on the rounded voltage.
    if load.current_a < 0.0 && state.soc >= 1.0 {
        return Err(StepReject::OvervoltageCutoff);
    }
    Ok(())
}

fn exceeds_current_nyquist(pack: &LiIonPack, current_a: f64, dt_s: f64) -> bool {
    if current_a == 0.0 {
        return false;
    }
    match pack.current_nyquist_a(dt_s) {
        Some(_) => !crate::impulse::within(current_a, dt_s, pack.charge_impulse_c()),
        None => true,
    }
}

fn commit(
    pack: &LiIonPack,
    state: &mut PackState,
    frame: &mut LastStateFrame64,
    load: PackLoad,
    dt_s: f64,
    predicted: [f64; 3],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite3(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    let soc = predicted[0];
    if !(soc >= 0.0 && soc <= 1.0) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }
    let terminal_v = pack.terminal_voltage(soc, load.current_a);
    if !terminal_v.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if terminal_v <= pack.cutoff_v {
        return Err(StepReject::UndervoltageCutoff);
    }
    if terminal_v > pack.charge_limit_v {
        return Err(StepReject::OvervoltageCutoff);
    }
    state.soc = soc;
    state.electrical_work_j = predicted[1];
    state.joule_j = predicted[2];
    state.current_a = load.current_a;
    state.t_s += dt_s;
    refresh(pack, state);
    let scale = pack.pack_energy_j();
    if !(scale > 0.0 && scale.is_finite()) {
        *state = saved;
        return Err(StepReject::InvalidPack);
    }
    state.energy_defect_j =
        state.chemical_energy_j - state.energy_anchor_j + state.electrical_work_j + state.joule_j;
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

fn refresh(pack: &LiIonPack, state: &mut PackState) {
    state.open_circuit_v = pack.open_circuit_voltage(state.soc);
    state.resistance_ohm = pack.resistance_ohm(state.soc);
    state.terminal_v = pack.terminal_voltage(state.soc, state.current_a);
    state.chemical_energy_j = pack.chemical_energy_j(state.soc);
    state.cutoff_margin_v = state.terminal_v - pack.cutoff_v;
    state.sag_warning = state.terminal_v < pack.warn_v;
}

fn publish(state: &PackState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.terminal_v as f32;
    frame.pos[1] = state.soc as f32;
    frame.pos[2] = state.current_a as f32;
    frame.vel[0] = state.open_circuit_v as f32;
    frame.vel[1] = state.chemical_energy_j as f32;
    frame.vel[2] = state.joule_j as f32;
    frame.force_torque = state.cutoff_margin_v as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.sag_warning {
        FLAG_SAG_WARNING
    } else {
        0
    };
}

fn derivatives(pack: &LiIonPack, y: [f64; 3], current_a: f64) -> Result<[f64; 3], StepReject> {
    let soc = y[0];
    if !soc.is_finite() || !current_a.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !(soc >= 0.0 && soc <= 1.0) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }
    let terminal_v = pack.terminal_voltage(soc, current_a);
    let resistance = pack.resistance_ohm(soc);
    let soc_rate = -current_a / pack.capacity_c;
    let electrical_w = terminal_v * current_a;
    let joule_w = current_a * current_a * resistance;
    let out = [soc_rate, electrical_w, joule_w];
    if finite3(out) {
        Ok(out)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

fn rk4(pack: &LiIonPack, y: [f64; 3], dt: f64, current_a: f64) -> Result<[f64; 3], StepReject> {
    let k1 = match derivatives(pack, y, current_a) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k2 = match derivatives(pack, axpy(y, dt * 0.5, k1), current_a) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k3 = match derivatives(pack, axpy(y, dt * 0.5, k2), current_a) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k4 = match derivatives(pack, axpy(y, dt, k3), current_a) {
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

fn pack_state(state: &PackState) -> [f64; 3] {
    [state.soc, state.electrical_work_j, state.joule_j]
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn state_is_finite(state: &PackState) -> bool {
    state.t_s.is_finite()
        && state.soc.is_finite()
        && state.current_a.is_finite()
        && state.terminal_v.is_finite()
        && state.open_circuit_v.is_finite()
        && state.resistance_ohm.is_finite()
        && state.chemical_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.electrical_work_j.is_finite()
        && state.joule_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.cutoff_margin_v.is_finite()
}

/// Add `extra_j` to the electrical work after a faithful RK4 step.
#[cfg(test)]
fn step_bias_work(
    pack: &LiIonPack,
    state: &mut PackState,
    frame: &mut LastStateFrame64,
    load: PackLoad,
    dt_s: f64,
    extra_j: f64,
) -> Result<(), StepReject> {
    validate(pack, state, load, dt_s)?;
    let mut predicted = match rk4(pack, pack_state(state), dt_s, load.current_a) {
        Ok(y) => y,
        Err(reject) => return Err(reject),
    };
    predicted[1] += extra_j;
    commit(pack, state, frame, load, dt_s, predicted)
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    SAG_KIND,
];

const SAG_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_SAG_WARNING",
    FLAG_SAG_WARNING,
    "terminal_v",
    "V",
    Side::Below,
    PACK_WARN_V,
    "the pack's sag warning voltage, 42 V; below cutoff, 36 V, the step stops",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
pub fn exceedances(
    pack: &LiIonPack,
    before: &PackState,
    after: &PackState,
    load: PackLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, load, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.sag_warning, Exceedance::at(SAG_KIND, pack.warn_v, after.terminal_v));
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        CAPACITY_AH, CAPACITY_C, FLAG_SAG_WARNING, INTERNAL_RESISTANCE_OHM, LiIonPack,
        PACK_CUTOFF_V, PACK_EMPTY_V, PACK_FULL_V, PACK_WARN_V, PackLoad, PackState,
        RESIDUAL_CONSERVATION_LIMIT, SERIES_CELLS, StepReject, axpy, commit, derivatives,
        pack_state, step, step_bias_work, validate,
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

    fn state_bits(state: &PackState) -> [u64; 14] {
        [
            state.t_s.to_bits(),
            state.soc.to_bits(),
            state.current_a.to_bits(),
            state.terminal_v.to_bits(),
            state.open_circuit_v.to_bits(),
            state.resistance_ohm.to_bits(),
            state.chemical_energy_j.to_bits(),
            state.energy_anchor_j.to_bits(),
            state.electrical_work_j.to_bits(),
            state.joule_j.to_bits(),
            state.energy_defect_j.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.cutoff_margin_v.to_bits(),
            u64::from(state.sag_warning),
        ]
    }

    /// Rectangle rule on `(s, W_elec, W_joule)`. The two powers are read at the
    /// start of the step, which is explicit Euler, or at the landing.
    fn step_rectangle(
        pack: &LiIonPack,
        state: &mut PackState,
        frame: &mut LastStateFrame64,
        load: PackLoad,
        dt_s: f64,
        at_landing: bool,
    ) -> Result<(), StepReject> {
        validate(pack, state, load, dt_s)?;
        let y = pack_state(state);
        let start = derivatives(pack, y, load.current_a)?;
        let mut predicted = axpy(y, dt_s, start);
        if at_landing {
            let landing = derivatives(pack, predicted, load.current_a)?;
            predicted[1] = y[1] + dt_s * landing[1];
            predicted[2] = y[2] + dt_s * landing[2];
        }
        commit(pack, state, frame, load, dt_s, predicted)
    }

    #[test]
    fn card_locks_resistance_and_matches_the_organ_sag() {
        assert_copy::<LiIonPack>();
        assert_copy::<PackState>();
        assert_copy::<PackLoad>();
        assert_copy::<StepReject>();
        let pack = LiIonPack::pack_12s();
        assert_eq!(SERIES_CELLS, 12.0);
        assert_eq!(CAPACITY_AH, 22.0);
        assert_eq!(pack.capacity_c.to_bits(), CAPACITY_C.to_bits());
        assert_eq!(
            pack.series_resistance_ohm.to_bits(),
            INTERNAL_RESISTANCE_OHM.to_bits()
        );
        assert_eq!(pack.full_v.to_bits(), PACK_FULL_V.to_bits());
        assert_eq!(pack.empty_v.to_bits(), PACK_EMPTY_V.to_bits());
        assert_eq!(pack.warn_v.to_bits(), PACK_WARN_V.to_bits());
        assert_eq!(pack.cutoff_v.to_bits(), PACK_CUTOFF_V.to_bits());
        let gate = pack.gate_dt_s().unwrap();
        // R0 C ≈ 330 s, so ω Δt = 0.02 is a 6.6 s reporting step.
        assert!(gate > 1.0 && gate < 10.0, "gate {gate}");
        assert!((pack.resistance_ohm(0.005) - 0.06 * (2.0 - 0.005)).abs() < 1.0e-15);
        let mut tampered = pack;
        tampered.series_resistance_ohm = 0.04;
        assert!(!tampered.is_valid());
        let mut state = PackState::try_full(&pack).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(&tampered, &mut state, &mut frame, PackLoad::default(), gate);
        assert_eq!(got, Err(StepReject::InvalidPack));
        assert_eq!(state, saved);
        proof_intact(&frame);
    }

    #[test]
    fn discharge_conserves_the_ocv_ledger_past_the_gate() {
        let pack = LiIonPack::pack_12s();
        let gate = pack.gate_dt_s().unwrap();
        let dt = gate * 10.0;
        let current = 0.1;
        assert!(current * dt < pack.charge_impulse_c());
        let mut state = PackState::try_soc(&pack, 0.8).unwrap();
        let mut frame = blank_frame();
        let mut n = 0u32;
        while n < 4 {
            step(
                &pack,
                &mut state,
                &mut frame,
                PackLoad { current_a: current },
                dt,
            )
            .unwrap();
            assert!(
                state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT,
                "residual {}",
                state.residual_conservation_error
            );
            n += 1;
        }
        let removed = current * state.t_s;
        let expected = 0.8 - removed / pack.capacity_c;
        assert!((state.soc - expected).abs() < 1.0e-12, "soc {}", state.soc);
        assert!(state.terminal_v > pack.cutoff_v);
        assert!(!state.sag_warning);
        proof_intact(&frame);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
    }

    #[test]
    fn open_circuit_holds_and_a_sag_below_42_v_is_only_a_flag() {
        let pack = LiIonPack::pack_12s();
        let mut state = PackState::try_full(&pack).unwrap();
        let mut frame = blank_frame();
        step(&pack, &mut state, &mut frame, PackLoad::default(), f64::MAX).unwrap();
        assert_eq!(state.soc, 1.0);
        assert_eq!(state.terminal_v.to_bits(), PACK_FULL_V.to_bits());
        assert_eq!(state.residual_conservation_error, 0.0);
        proof_intact(&frame);

        let warn = PackLoad { current_a: 150.0 };
        step(&pack, &mut state, &mut frame, warn, 1.0).unwrap();
        assert!(state.terminal_v < PACK_WARN_V);
        assert!(state.terminal_v > PACK_CUTOFF_V);
        assert!(state.sag_warning);
        assert_eq!(frame.flags, FLAG_SAG_WARNING);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    #[test]
    fn cutoff_and_an_empty_soc_roll_back() {
        let pack = LiIonPack::pack_12s();
        let mut state = PackState::try_soc(&pack, 0.5).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(
            &pack,
            &mut state,
            &mut frame,
            PackLoad { current_a: 80.0 },
            1.0,
        );
        assert_eq!(got, Err(StepReject::UndervoltageCutoff));
        assert_eq!(state, saved);
        proof_intact(&frame);

        let mut low = PackState::try_soc(&pack, 0.001).unwrap();
        let held = low;
        let got = step(
            &pack,
            &mut low,
            &mut frame,
            PackLoad { current_a: 200.0 },
            1.0,
        );
        assert_eq!(got, Err(StepReject::StateOfChargeOutsideCard));
        assert_eq!(low, held);
        proof_intact(&frame);
    }

    #[test]
    fn current_nyquist_and_a_biased_ledger_roll_back() {
        let pack = LiIonPack::pack_12s();
        let dt = 1.0;
        let limit = pack.current_nyquist_a(dt).unwrap();
        let mut state = PackState::try_full(&pack).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(
            &pack,
            &mut state,
            &mut frame,
            PackLoad {
                current_a: limit + 1.0,
            },
            dt,
        );
        assert_eq!(got, Err(StepReject::CurrentAboveNyquist));
        assert_eq!(state, saved);
        proof_intact(&frame);

        let load = PackLoad { current_a: 10.0 };
        step(&pack, &mut state, &mut frame, load, dt).unwrap();
        let held = state;
        let extra = pack.pack_energy_j() * 1.0e-3;
        let got = step_bias_work(&pack, &mut state, &mut frame, load, dt, extra);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, held);
        let got = step_bias_work(&pack, &mut state, &mut frame, load, dt, -extra);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state, held);
        proof_intact(&frame);
    }

    #[test]
    fn non_finite_inputs_leave_the_frame_alone() {
        let pack = LiIonPack::pack_12s();
        let mut state = PackState::try_full(&pack).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let dt = 1.0;
        let mut refuse = |load: PackLoad, dt_s: f64, expect: StepReject| {
            let got = step(&pack, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state, saved);
            proof_intact(&frame);
        };
        refuse(PackLoad::default(), 0.0, StepReject::NonPositiveDt);
        refuse(PackLoad::default(), -dt, StepReject::NonPositiveDt);
        refuse(PackLoad::default(), f64::NAN, StepReject::NonFiniteInput);
        refuse(
            PackLoad {
                current_a: f64::INFINITY,
            },
            dt,
            StepReject::NonFiniteInput,
        );
        refuse(
            PackLoad {
                current_a: f64::MAX,
            },
            dt,
            StepReject::CurrentAboveNyquist,
        );
        state.soc = f64::NAN;
        let got = step(&pack, &mut state, &mut frame, PackLoad::default(), dt);
        assert_eq!(got, Err(StepReject::NonFiniteState));
        proof_intact(&frame);
    }

    #[test]
    fn rectangle_work_breaches_the_ledger_and_rolls_back_bit_for_bit() {
        let pack = LiIonPack::pack_12s();
        let dt = 10.0;
        let load = PackLoad { current_a: 22.0 };
        let charge_c = load.current_a * dt;
        assert!(charge_c < pack.charge_impulse_c());
        // A rectangle misses the triangle (V_full − V_empty) (I Δt)² / (2 Q).
        let triangle_j =
            (pack.full_v - pack.empty_v) * charge_c * charge_c / (2.0 * pack.capacity_c);
        let gate_j = RESIDUAL_CONSERVATION_LIMIT * pack.pack_energy_j();
        assert!(triangle_j > gate_j, "triangle {triangle_j} gate {gate_j}");

        let mut state = PackState::try_full(&pack).unwrap();
        let mut frame = blank_frame();
        step(&pack, &mut state, &mut frame, load, dt).unwrap();
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();

        let got = step_rectangle(&pack, &mut state, &mut frame, load, dt, false);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        let got = step_rectangle(&pack, &mut state, &mut frame, load, dt, true);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        step(&pack, &mut state, &mut frame, load, dt).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    #[test]
    fn explicit_euler_work_error_adds_up_to_a_refusal_on_the_predicted_step() {
        let pack = LiIonPack::pack_12s();
        let dt = 1.0;
        let load = PackLoad { current_a: 22.0 };
        let charge_c = load.current_a * dt;
        let triangle_j =
            (pack.full_v - pack.empty_v) * charge_c * charge_c / (2.0 * pack.capacity_c);
        let gate_j = RESIDUAL_CONSERVATION_LIMIT * pack.pack_energy_j();
        let landed_steps = (gate_j / triangle_j) as u32;
        assert!(landed_steps > 10, "landed {landed_steps}");
        let u = f64::EPSILON * 0.5;
        let soc_to_j = pack.capacity_c * pack.full_v;

        let mut state = PackState::try_full(&pack).unwrap();
        let mut frame = blank_frame();
        let mut n = 0u32;
        while n < landed_steps {
            step_rectangle(&pack, &mut state, &mut frame, load, dt, false).unwrap();
            n += 1;
            let expected_j = f64::from(n) * triangle_j;
            // One rounding of s per step, carried into E, and the evaluation of E.
            let tol_j = (f64::from(n) + 8.0) * u * soc_to_j + 8.0 * u * pack.pack_energy_j();
            assert!(
                (state.energy_defect_j - expected_j).abs() <= tol_j,
                "step {n} defect {}",
                state.energy_defect_j
            );
        }
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();
        let got = step_rectangle(&pack, &mut state, &mut frame, load, dt, false);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        proof_intact(&frame);
    }

    #[test]
    fn held_current_discharge_matches_the_closed_form() {
        let pack = LiIonPack::pack_12s();
        let dt = 1.0;
        let current = 22.0;
        let steps = 2_880u32;
        let soc0 = 1.0;
        let window_v = pack.full_v - pack.empty_v;
        let r0 = pack.series_resistance_ohm;
        // dV/ds under a held current.
        let slope_v = window_v + current * r0;
        let v0 = pack.terminal_voltage(soc0, current);
        let r_start = pack.resistance_ohm(soc0);
        let e0 = pack.chemical_energy_j(soc0);
        // One rounding of a value near 1 is at most this fraction of it.
        let u = f64::EPSILON * 0.5;

        let mut state = PackState::try_soc(&pack, soc0).unwrap();
        let mut frame = blank_frame();
        let load = PackLoad { current_a: current };
        let mut work_tol_j = 0.0;
        let mut joule_tol_j = 0.0;
        let mut n = 0u32;
        while n < steps {
            step(&pack, &mut state, &mut frame, load, dt).unwrap();
            n += 1;
            // s and V are linear in t, so each integral is charge times a mean.
            let t = state.t_s;
            assert_eq!(t, f64::from(n) * dt);
            let charge_c = current * t;
            let soc = soc0 - charge_c / pack.capacity_c;
            let terminal_v = pack.empty_v + window_v * soc - current * r0 * (2.0 - soc);
            let work_j = charge_c * 0.5 * (v0 + terminal_v);
            let joule_j = current * charge_c * 0.5 * (r_start + r0 * (2.0 - soc));
            let energy_j = pack.capacity_c * (pack.empty_v * soc + 0.5 * window_v * soc * soc);

            // One rounding per accumulated step, eight in the two evaluations.
            let soc_tol = (f64::from(n) + 8.0) * u;
            let v_tol = slope_v * soc_tol + 8.0 * u * pack.full_v;
            // Each step adds one rounding of the sum and the stage error in s.
            work_tol_j += u * work_j + current * slope_v * dt * soc_tol;
            joule_tol_j += u * joule_j + current * current * r0 * dt * soc_tol;
            let work_tol = work_tol_j + 16.0 * u * work_j;
            let joule_tol = joule_tol_j + 16.0 * u * joule_j;
            let energy_tol = pack.capacity_c * pack.full_v * soc_tol + 8.0 * u * e0;

            assert!((state.soc - soc).abs() <= soc_tol, "step {n} soc");
            assert!(
                (state.terminal_v - terminal_v).abs() <= v_tol,
                "step {n} terminal {}",
                state.terminal_v
            );
            assert!(
                (state.electrical_work_j - work_j).abs() <= work_tol,
                "step {n} work {}",
                state.electrical_work_j
            );
            assert!(
                (state.joule_j - joule_j).abs() <= joule_tol,
                "step {n} joule {}",
                state.joule_j
            );
            assert!(
                (state.chemical_energy_j - energy_j).abs() <= energy_tol,
                "step {n} energy {}",
                state.chemical_energy_j
            );
            assert!(
                state.energy_defect_j.abs() <= energy_tol + work_tol + joule_tol,
                "step {n} defect {}",
                state.energy_defect_j
            );
        }
        // 80% of the capacity has left and the pack is still above cutoff.
        assert!((state.soc - 0.2).abs() < 1.0e-12, "soc {}", state.soc);
        assert!(state.terminal_v > pack.cutoff_v);
        assert!(state.terminal_v < pack.cutoff_v + 1.0);
        assert!(state.sag_warning);
        assert_eq!(frame.flags, FLAG_SAG_WARNING);
        assert_eq!(frame.pos[1], state.soc as f32);
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::LiIonPack::pack_12s();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The pack at 80 percent, discharging at 22 A.
        let mut state = super::PackState::try_soc(&card, 0.8).unwrap();
        let load = super::PackLoad { current_a: 22.0 };
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
        assert!(state.soc != before.soc);
        assert!(state.chemical_energy_j != before.chemical_energy_j);
        assert!(state.electrical_work_j != before.electrical_work_j);
        assert!(state.joule_j != before.joule_j);
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

    /// Step, and require this refusal with the state and the frame bytes as they were.
    fn refused_whole(
        pack: &LiIonPack,
        state: &mut PackState,
        frame: &mut LastStateFrame64,
        load: PackLoad,
        dt_s: f64,
        expect: StepReject,
    ) {
        let held = state_bits(state);
        let held_frame = frame.to_bytes();
        let got = step(pack, state, frame, load, dt_s);
        assert_eq!(got, Err(expect), "current {} A", load.current_a);
        assert_eq!(state_bits(state), held);
        assert_eq!(frame.to_bytes(), held_frame);
    }

    #[test]
    fn the_charge_limit_lands_and_the_next_current_past_it_is_refused() {
        let pack = LiIonPack::pack_12s();
        let limit_v = super::PACK_CHARGE_LIMIT_V;
        assert_eq!(pack.charge_limit_v.to_bits(), limit_v.to_bits());
        // 12S × 4.2 V, the pack's full voltage.
        assert_eq!(limit_v.to_bits(), PACK_FULL_V.to_bits());
        assert!((limit_v - SERIES_CELLS * 4.2).abs() < 1.0e-12);
        let dt = 1.0;

        // A full pack at rest sits on the limit itself and lands.
        let mut full = PackState::try_full(&pack).unwrap();
        let mut frame = blank_frame();
        step(&pack, &mut full, &mut frame, PackLoad::default(), dt).unwrap();
        assert_eq!(full.terminal_v.to_bits(), limit_v.to_bits());

        // From half charge, find the two neighbouring charging currents of
        // which one lands and the next is refused. On the card's lines the
        // edge is near 80 A.
        let lands = |current_a: f64| {
            let mut state = PackState::try_soc(&pack, 0.5).unwrap();
            let mut frame = blank_frame();
            match step(&pack, &mut state, &mut frame, PackLoad { current_a }, dt) {
                Ok(()) => Some(state.terminal_v),
                Err(StepReject::OvervoltageCutoff) => None,
                Err(other) => std::panic!("current {current_a} refused with {other:?}"),
            }
        };
        let mut inside = -79.0;
        let mut outside = -81.0;
        assert!(lands(inside).is_some() && lands(outside).is_none());
        while inside.next_down() > outside {
            let middle = inside + (outside - inside) * 0.5;
            if lands(middle).is_some() {
                inside = middle;
            } else {
                outside = middle;
            }
        }
        assert_eq!(inside.next_down().to_bits(), outside.to_bits());
        let landed_v = lands(inside).unwrap();
        assert!(landed_v <= limit_v);
        assert!(limit_v - landed_v <= 8.0 * (limit_v.next_up() - limit_v));
        let mut state = PackState::try_soc(&pack, 0.5).unwrap();
        let mut frame = blank_frame();
        refused_whole(
            &pack,
            &mut state,
            &mut frame,
            PackLoad { current_a: outside },
            dt,
            StepReject::OvervoltageCutoff,
        );
        proof_intact(&frame);

        // The landed charging step raised the charge, balanced the ledger and set no flag.
        step(
            &pack,
            &mut state,
            &mut frame,
            PackLoad { current_a: inside },
            dt,
        )
        .unwrap();
        assert!(state.soc > 0.5);
        assert!(state.electrical_work_j < 0.0 && state.joule_j > 0.0);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert_eq!(frame.flags, 0);

        // Charging current is held under 120 A at every state of charge.
        let mut low = PackState::try_soc(&pack, 1.0e-3).unwrap();
        refused_whole(
            &pack,
            &mut low,
            &mut frame,
            PackLoad { current_a: -120.0 },
            dt,
            StepReject::OvervoltageCutoff,
        );
        step(
            &pack,
            &mut low,
            &mut frame,
            PackLoad { current_a: -119.0 },
            dt,
        )
        .unwrap();
        assert!(low.terminal_v <= limit_v);
        // A discharge never meets the limit: its terminals sit below the open-circuit line.
        let mut full = PackState::try_full(&pack).unwrap();
        step(
            &pack,
            &mut full,
            &mut frame,
            PackLoad { current_a: 22.0 },
            dt,
        )
        .unwrap();

        // The recorded copy is locked like the other constants.
        let mut tampered = pack;
        tampered.charge_limit_v = 60.0;
        assert!(!tampered.is_valid());
        refused_whole(
            &tampered,
            &mut full,
            &mut frame,
            PackLoad::default(),
            dt,
            StepReject::InvalidPack,
        );
        proof_intact(&frame);
    }

    /// A full pack accepts no charging current. `50.4 + |I| * 0.06` rounds
    /// back to 50.4 V for a current of 5e-14 A, and the step used to land
    /// and seal it. The smallest normal current, the subnormal ones, and a
    /// current of ordinary size are refused, and the state and frame stay.
    #[test]
    fn a_full_pack_refuses_every_charging_current_by_name() {
        let pack = LiIonPack::pack_12s();
        let mut frame = blank_frame();
        for dt_s in [0.1, 1.0, pack.floor_dt_s().unwrap()] {
            for current_a in [
                -5.0e-14,
                -5.921189464667501e-14,
                -f64::MIN_POSITIVE,
                -f64::from_bits(1),
                -1.0,
                -22.0,
            ] {
                let mut full = PackState::try_full(&pack).unwrap();
                refused_whole(
                    &pack,
                    &mut full,
                    &mut frame,
                    PackLoad { current_a },
                    dt_s,
                    StepReject::OvervoltageCutoff,
                );
            }
        }
        proof_intact(&frame);
        // A full pack at rest and under discharge still lands; negative zero is a rest.
        for current_a in [0.0, -0.0, 5.0e-14, 22.0] {
            let mut full = PackState::try_full(&pack).unwrap();
            step(&pack, &mut full, &mut frame, PackLoad { current_a }, 0.1).unwrap();
            assert!(full.terminal_v <= pack.charge_limit_v);
        }
        // Just below full, a small charging current still lands.
        let mut nearly = PackState::try_soc(&pack, 0.99).unwrap();
        step(&pack, &mut nearly, &mut frame, PackLoad { current_a: -1.0 }, 0.1).unwrap();
        assert!(nearly.terminal_v <= pack.charge_limit_v);
    }

    /// Charging steps that each move 100 C in one step, at currents from
    /// 1e3 A to 1e20 A. Without a limit the card landed 133 V to 9e14 V on
    /// the terminals, refused 1e17 A and 1e18 A as created energy, and
    /// landed 1e19 A and 1e20 A with a residual of zero.
    #[test]
    fn extreme_charging_currents_are_refused_by_name_before_the_ledger() {
        let pack = LiIonPack::pack_12s();
        let mut frame = blank_frame();
        let mut current_a = -1.0e3;
        let mut decade = 3;
        while decade <= 20 {
            let mut state = PackState::try_soc(&pack, 0.5).unwrap();
            refused_whole(
                &pack,
                &mut state,
                &mut frame,
                PackLoad { current_a },
                100.0 / -current_a,
                StepReject::OvervoltageCutoff,
            );
            current_a *= 10.0;
            decade += 1;
        }
        // Three runs of 1,000 steps from 20 percent. The third used to be
        // refused on step 912 as destroyed energy.
        let runs = [(-1.0e9, 1.0e-9), (-1.0e13, 1.0e-13), (-1.0e14, 1.0e-14)];
        let mut i = 0;
        while i < runs.len() {
            let (current_a, dt_s) = runs[i];
            let mut state = PackState::try_soc(&pack, 0.2).unwrap();
            refused_whole(
                &pack,
                &mut state,
                &mut frame,
                PackLoad { current_a },
                dt_s,
                StepReject::OvervoltageCutoff,
            );
            i += 1;
        }
        // 2,399 A for 0.1 s from 20 percent used to land near 298 V with no flag.
        let mut state = PackState::try_soc(&pack, 0.2).unwrap();
        assert!(pack.terminal_voltage(0.2, -2_399.0) > 297.0);
        refused_whole(
            &pack,
            &mut state,
            &mut frame,
            PackLoad {
                current_a: -2_399.0,
            },
            0.1,
            StepReject::OvervoltageCutoff,
        );
        proof_intact(&frame);
    }

    /// A million steps at the card's ceiling step, charging and discharging
    /// in turn between 10 and 90 percent at half the current each limit
    /// allows. The heat total grows through about 2,500 turns.
    #[test]
    fn a_million_steps_of_cycling_inside_the_voltage_window_stay_far_inside_the_limit() {
        let pack = LiIonPack::pack_12s();
        let dt = pack.gate_dt_s().unwrap();
        let most_a = 0.9 * pack.current_nyquist_a(dt).unwrap();
        let mut state = PackState::try_soc(&pack, 0.5).unwrap();
        let mut frame = blank_frame();
        let mut charging = true;
        let mut turns = 0u32;
        let mut max_residual = 0.0;
        let mut n = 0u32;
        while n < 1_000_000 {
            let open_circuit_v = pack.open_circuit_voltage(state.soc);
            let resistance_ohm = pack.resistance_ohm(state.soc);
            let mut current_a = if charging {
                -0.5 * (pack.charge_limit_v - open_circuit_v) / resistance_ohm
            } else {
                0.5 * (open_circuit_v - pack.cutoff_v) / resistance_ohm
            };
            if current_a > most_a {
                current_a = most_a;
            }
            if current_a < -most_a {
                current_a = -most_a;
            }
            step(&pack, &mut state, &mut frame, PackLoad { current_a }, dt).unwrap();
            assert!(state.terminal_v > pack.cutoff_v && state.terminal_v <= pack.charge_limit_v);
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            if charging && state.soc > 0.9 {
                charging = false;
                turns += 1;
            }
            if !charging && state.soc < 0.1 {
                charging = true;
                turns += 1;
            }
            n += 1;
        }
        assert!(turns > 2_000, "turns {turns}");
        assert!(state.joule_j > 1.0e8, "heat {}", state.joule_j);
        // Measured 1.3e-10. The limit is 1.0e-6.
        assert!(max_residual < 1.0e-9, "residual {max_residual}");
        assert!(max_residual < RESIDUAL_CONSERVATION_LIMIT / 1_000.0);
        proof_intact(&frame);
    }
}

#[cfg(test)]
mod boundary_tests {
    /// The last load accepted and the first refused, at several step sizes,
    /// agree with the published rule `|current_a| * dt_s <= charge_impulse_c`.
    #[test]
    fn the_load_rule_is_the_published_product_compared_with_the_constant() {
        let pack = super::LiIonPack::pack_12s();
        let limit = pack.charge_impulse_c();
        crate::impulse::tests::assert_boundary(limit, |load, dt_s| super::exceeds_current_nyquist(&pack, load, dt_s));
    }
}
