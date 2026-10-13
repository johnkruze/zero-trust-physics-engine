#![no_implicit_prelude]

//! Aluminum 6061 avionics cold plate.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, plate temperature, bath temperature, heater
//! power, internal energy, boundary heat, the heat leaving to the bath, the
//! ceiling margin, and the conservation residual into [`LastStateFrame64`].
//! The 16-byte frame proof is not touched.
//!
//! The card is one plate, 250 mm by 125 mm by 1/128 m, density 2700 kg/m³,
//! specific heat 896 J/(kg·K). One face is washed by cabinet air at
//! 16 W/(m²·K). That fixes the capacitance at 590.625 J/K and the resistance
//! at 2 K/W. Neither value is a step argument, and neither is raised to a
//! floor.
//!
//! The public step is the exact RC map, not a Runge–Kutta step:
//!
//! ```text
//! C dT/dt = Q̇ − (T − T_amb) / R
//! T_eq = T_amb + Q̇ R
//! T(t+dt) = T + (T_eq − T) (1 − exp(−dt / RC))
//! U = C (T − T_ref)
//! Q_boundary = C (T_eq − T) (1 − exp(−dt / RC))
//! residual_conservation_error = |U − U_anchor − Q_boundary| / U_card
//! ```
//!
//! The landed temperature is a 64-bit number, so the rise that lands is the
//! computed rise rounded to the spacing of 64-bit numbers near the plate
//! temperature, 5.7e-14 K near 293 K. The boundary heat booked for a step is
//! the capacitance times the rise that landed, so the ledger holds the heat of
//! the temperature change the state carries. Each step lands within
//! [`STEP_ERROR_K`] of the exact map of the temperature it starts from, so
//! `N` steps and one step over the same span agree within `N + 1` times
//! [`STEP_ERROR_K`]. Near equilibrium a small step's rise can round to zero:
//! the plate then does not move and no heat is booked.
//!
//! `U_card` is the sensible heat from 20 °C to the 125 °C silicon ceiling.
//! The map is the integral of the heat equation, so a landed temperature
//! that is not this map does not match the heat that crossed the boundary.
//! The pole is `ω = 1/(RC)`. [`ThermalNode::gate_dt_s`] is `ω Δt = 0.02`.
//! The public step does not apply that clock: the exponential is exact for
//! every finite `dt > 0`.
//!
//! A trial temperature outside 233.15 K…398.15 K is
//! [`StepReject::TemperatureOutsideCard`]. At or above 85 °C the frame sets
//! [`FLAG_THERMAL_DERATE`] and the step still lands. Creation of heat is
//! [`StepReject::SpontaneousGeneration`].

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// 6061 plate length.
pub const PLATE_LENGTH_M: f64 = 0.25;
/// 6061 plate width.
pub const PLATE_WIDTH_M: f64 = 0.125;
/// 6061 plate thickness, 1/128 m.
pub const PLATE_THICKNESS_M: f64 = 1.0 / 128.0;
/// Solid density of aluminum 6061.
pub const ALUMINUM_6061_DENSITY_KG_M3: f64 = 2_700.0;
/// Solid specific heat of aluminum 6061.
pub const ALUMINUM_6061_CP_J_PER_KG_K: f64 = 896.0;
/// Forced cabinet air on the wetted face.
pub const WETTED_CONVECTION_W_PER_M2_K: f64 = 16.0;

/// `ρ V c`. 590.625 J/K. Not a caller input and not floored.
pub const THERMAL_CAPACITANCE_J_PER_K: f64 = ALUMINUM_6061_DENSITY_KG_M3
    * PLATE_LENGTH_M
    * PLATE_WIDTH_M
    * PLATE_THICKNESS_M
    * ALUMINUM_6061_CP_J_PER_KG_K;
/// `1 / (h A)`. 2 K/W. Not a caller input and not floored.
pub const THERMAL_RESISTANCE_K_PER_W: f64 =
    1.0 / (WETTED_CONVECTION_W_PER_M2_K * PLATE_LENGTH_M * PLATE_WIDTH_M);

/// 20 °C. Zero of the sensible-heat ledger.
pub const REFERENCE_TEMPERATURE_K: f64 = 293.15;
/// −40 °C. Below this the industrial card is not this model.
pub const CARD_TEMPERATURE_MIN_K: f64 = 233.15;
/// 85 °C. The frame is flagged. The step still lands.
pub const DERATE_TEMPERATURE_K: f64 = 358.15;
/// 125 °C silicon ceiling. At or above this the step is refused.
pub const CARD_TEMPERATURE_MAX_K: f64 = 398.15;

/// `|U − U_anchor − Q_boundary| / U_card` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Largest distance between a landed temperature and the exact map of the
/// temperature the step starts from, for a step of at most
/// [`ThermalNode::gate_dt_s`] that lands on the card: 2^-42 K, about
/// 2.3e-13 K, four units in the last place of a 64-bit number between 256 K
/// and 512 K. An eighth of it is the rounding of the landed temperature. The
/// rest covers the rounding of the equilibrium, the decay factor and the rise
/// of a step that moves the plate at most 165 K and at most two hundredths of
/// its distance to equilibrium. A run of 200,000 steps driven toward the
/// card's edges, compared outside this crate with a 200-bit exact map, found
/// at most 9.05e-14 K; no reference test in this crate holds the step to a
/// tighter bound, so the bound published is 2^-42 K.
pub const STEP_ERROR_K: f64 = 1.0 / 4_398_046_511_104.0;
/// Thermal phase advance `ω Δt` used by [`ThermalNode::gate_dt_s`]. `ω = 1/(RC)`.
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Plate temperature is at or above [`DERATE_TEMPERATURE_K`] on this frame.
pub const FLAG_THERMAL_DERATE: u64 = 1 << 0;

/// Why a step did not advance the plate or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Heater power, ambient, or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A step would land the plate at or below zero kelvin, or the kernel
    /// was handed a plate or bath temperature at or below zero. Through the
    /// gateway an input temperature is held to the card window first, so
    /// only the landing reaches this name.
    NegativeTemperature,
    /// Plate or ambient temperature is outside the industrial card.
    TemperatureOutsideCard,
    /// Capacitance, resistance, or the card temperatures are not this plate.
    InvalidNode,
    /// The trial step created heat from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed heat.
    FirstLawBreach,
}

/// Zero-order hold applied across one step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalLoad {
    pub heat_power_w: f64,
    pub ambient_k: f64,
}

impl Default for ThermalLoad {
    fn default() -> Self {
        Self {
            heat_power_w: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        }
    }
}

/// The locked cold plate. Capacitance and resistance are recorded copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalNode {
    pub capacitance_j_per_k: f64,
    pub resistance_k_per_w: f64,
    /// Recorded copy of [`REFERENCE_TEMPERATURE_K`]. Not a caller input.
    pub reference_k: f64,
    /// Recorded copy of [`CARD_TEMPERATURE_MIN_K`]. Not a caller input.
    pub temperature_min_k: f64,
    /// Recorded copy of [`DERATE_TEMPERATURE_K`]. Not a caller input.
    pub derate_k: f64,
    /// Recorded copy of [`CARD_TEMPERATURE_MAX_K`]. Not a caller input.
    pub temperature_max_k: f64,
}

impl ThermalNode {
    /// Card: 6061 avionics plate, 590.625 J/K into 2 K/W.
    #[must_use]
    pub fn avionics_cold_plate() -> Self {
        let node = Self {
            capacitance_j_per_k: THERMAL_CAPACITANCE_J_PER_K,
            resistance_k_per_w: THERMAL_RESISTANCE_K_PER_W,
            reference_k: REFERENCE_TEMPERATURE_K,
            temperature_min_k: CARD_TEMPERATURE_MIN_K,
            derate_k: DERATE_TEMPERATURE_K,
            temperature_max_k: CARD_TEMPERATURE_MAX_K,
        };
        if node.is_valid() {
            node
        } else {
            ::core::panic!("avionics cold plate failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.capacitance_j_per_k.to_bits() == THERMAL_CAPACITANCE_J_PER_K.to_bits()
            && self.resistance_k_per_w.to_bits() == THERMAL_RESISTANCE_K_PER_W.to_bits()
            && self.reference_k.to_bits() == REFERENCE_TEMPERATURE_K.to_bits()
            && self.temperature_min_k.to_bits() == CARD_TEMPERATURE_MIN_K.to_bits()
            && self.derate_k.to_bits() == DERATE_TEMPERATURE_K.to_bits()
            && self.temperature_max_k.to_bits() == CARD_TEMPERATURE_MAX_K.to_bits()
            && self.capacitance_j_per_k > 0.0
            && self.resistance_k_per_w > 0.0
            && self.temperature_min_k > 0.0
            && self.reference_k >= self.temperature_min_k
            && self.derate_k > self.reference_k
            && self.temperature_max_k > self.derate_k
            && self.time_constant_s().is_finite()
            && self.time_constant_s() > 0.0
            && self.card_energy_j().is_finite()
            && self.card_energy_j() > 0.0
    }

    /// `RC`.
    #[must_use]
    pub fn time_constant_s(&self) -> f64 {
        self.resistance_k_per_w * self.capacitance_j_per_k
    }

    /// `1/(RC)`.
    #[must_use]
    pub fn pole_rad_s(&self) -> f64 {
        1.0 / self.time_constant_s()
    }

    /// Largest reporting step of the thermal pole. Not applied by [`step`].
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
    /// 2^20. At the ceiling a step moves the plate two hundredths of its
    /// distance to equilibrium. At the floor a step moves it about two parts
    /// in a hundred million of that distance: 1.9e-8 K or more when the plate
    /// is a kelvin or more from equilibrium, which 64-bit arithmetic resolves
    /// to about five figures. Closer to equilibrium a floor step's rise is a
    /// few units in the last place of the temperature or rounds to zero, and
    /// the step lands within [`STEP_ERROR_K`] of the exact map. Not applied by
    /// [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }

    /// Sensible heat from [`REFERENCE_TEMPERATURE_K`] to [`CARD_TEMPERATURE_MAX_K`].
    #[must_use]
    pub fn card_energy_j(&self) -> f64 {
        self.capacitance_j_per_k * (CARD_TEMPERATURE_MAX_K - REFERENCE_TEMPERATURE_K)
    }

    #[must_use]
    pub fn internal_energy_j(&self, temperature_k: f64) -> f64 {
        self.capacitance_j_per_k * (temperature_k - REFERENCE_TEMPERATURE_K)
    }
}

/// Integrated plate. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalNodeState {
    pub t_s: f64,
    pub temperature_k: f64,
    pub heat_power_w: f64,
    pub ambient_k: f64,
    pub internal_energy_j: f64,
    pub energy_anchor_j: f64,
    pub boundary_work_j: f64,
    /// Signed `U − U_anchor − Q_boundary`. Positive is created heat.
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    pub heat_to_ambient_w: f64,
    pub ceiling_margin_k: f64,
    pub entropy_production_w_per_k: f64,
    pub derated: bool,
}

impl ThermalNodeState {
    /// Plate and bath at the reference temperature, heater off.
    #[must_use]
    pub fn try_rest(node: &ThermalNode) -> Option<Self> {
        Self::try_phase(node, REFERENCE_TEMPERATURE_K)
    }

    /// Anchor the ledger at this plate temperature. The bath starts at the reference.
    #[must_use]
    pub fn try_phase(node: &ThermalNode, temperature_k: f64) -> Option<Self> {
        if !node.is_valid() || !on_card(temperature_k) {
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
            ceiling_margin_k: 0.0,
            entropy_production_w_per_k: 0.0,
            derated: false,
        };
        refresh(node, &mut state);
        if !state_is_finite(&state) {
            return None;
        }
        state.energy_anchor_j = state.internal_energy_j;
        state.boundary_work_j = 0.0;
        state.energy_defect_j = 0.0;
        state.residual_conservation_error = 0.0;
        Some(state)
    }
}

/// Advance one exact RC step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    node: &ThermalNode,
    state: &mut ThermalNodeState,
    frame: &mut LastStateFrame64,
    load: ThermalLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(node, state, load, dt_s)?;
    let advanced = match exact_map(node, state.temperature_k, load, dt_s) {
        Some(stepped) => stepped,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(node, state, frame, load, dt_s, advanced)
}

fn on_card(temperature_k: f64) -> bool {
    temperature_k.is_finite()
        && temperature_k >= CARD_TEMPERATURE_MIN_K
        && temperature_k < CARD_TEMPERATURE_MAX_K
}

fn validate(
    node: &ThermalNode,
    state: &ThermalNodeState,
    load: ThermalLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.heat_power_w.is_finite() || !load.ambient_k.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !(load.ambient_k > 0.0) || !(state.temperature_k > 0.0) {
        return Err(StepReject::NegativeTemperature);
    }
    if !node.is_valid() {
        return Err(StepReject::InvalidNode);
    }
    if !on_card(load.ambient_k) || !on_card(state.temperature_k) {
        return Err(StepReject::TemperatureOutsideCard);
    }
    Ok(())
}

fn commit(
    node: &ThermalNode,
    state: &mut ThermalNodeState,
    frame: &mut LastStateFrame64,
    load: ThermalLoad,
    dt_s: f64,
    advanced: ExactStep,
) -> Result<(), StepReject> {
    let saved = *state;
    if !advanced.temperature_k.is_finite() || !advanced.heat_j.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !(advanced.temperature_k > 0.0) {
        return Err(StepReject::NegativeTemperature);
    }
    if !on_card(advanced.temperature_k) {
        return Err(StepReject::TemperatureOutsideCard);
    }
    state.temperature_k = advanced.temperature_k;
    state.boundary_work_j += advanced.heat_j;
    state.heat_power_w = load.heat_power_w;
    state.ambient_k = load.ambient_k;
    state.t_s += dt_s;
    refresh(node, state);
    let scale = node.card_energy_j();
    if !(scale > 0.0 && scale.is_finite()) {
        *state = saved;
        return Err(StepReject::InvalidNode);
    }
    state.energy_defect_j = state.internal_energy_j - state.energy_anchor_j - state.boundary_work_j;
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

fn refresh(node: &ThermalNode, state: &mut ThermalNodeState) {
    state.internal_energy_j = node.internal_energy_j(state.temperature_k);
    state.heat_to_ambient_w = (state.temperature_k - state.ambient_k) / node.resistance_k_per_w;
    state.ceiling_margin_k = CARD_TEMPERATURE_MAX_K - state.temperature_k;
    state.derated = state.temperature_k >= DERATE_TEMPERATURE_K;
    state.entropy_production_w_per_k = conduction_entropy_w_per_k(
        state.temperature_k,
        state.ambient_k,
        node.resistance_k_per_w,
    );
}

/// `(T − T_amb)² / (R T T_amb)`. Negative temperature returns NaN.
fn conduction_entropy_w_per_k(temperature_k: f64, ambient_k: f64, resistance_k_per_w: f64) -> f64 {
    if !(temperature_k > 0.0) || !(ambient_k > 0.0) || !(resistance_k_per_w > 0.0) {
        return f64::NAN;
    }
    let delta_k = temperature_k - ambient_k;
    delta_k * delta_k / (resistance_k_per_w * temperature_k * ambient_k)
}

struct ExactStep {
    temperature_k: f64,
    heat_j: f64,
}

/// `T + (T_eq − T) (1 − e^{−dt/RC})`, and the boundary heat `C` times the rise
/// that landed, `C (T_next − T)`. The difference of two landed temperatures is
/// exact in 64-bit arithmetic, so the heat booked is the heat of the
/// temperature change the state carries.
fn exact_map(
    node: &ThermalNode,
    temperature_k: f64,
    load: ThermalLoad,
    dt_s: f64,
) -> Option<ExactStep> {
    if !(node.capacitance_j_per_k > 0.0) || !(node.resistance_k_per_w > 0.0) {
        return None;
    }
    let tau = node.time_constant_s();
    if !(tau.is_finite() && tau > 0.0) {
        return None;
    }
    let equilibrium_k = load.ambient_k + load.heat_power_w * node.resistance_k_per_w;
    let one_minus_decay = -crate::math::exp_m1(-dt_s / tau);
    if !equilibrium_k.is_finite() || !one_minus_decay.is_finite() {
        return None;
    }
    let rise_k = (equilibrium_k - temperature_k) * one_minus_decay;
    let next_k = temperature_k + rise_k;
    let heat_j = node.capacitance_j_per_k * (next_k - temperature_k);
    if next_k.is_finite() && heat_j.is_finite() {
        Some(ExactStep {
            temperature_k: next_k,
            heat_j,
        })
    } else {
        None
    }
}

fn publish(state: &ThermalNodeState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.temperature_k as f32;
    frame.pos[1] = state.ambient_k as f32;
    frame.pos[2] = state.heat_power_w as f32;
    frame.vel[0] = state.internal_energy_j as f32;
    frame.vel[1] = state.boundary_work_j as f32;
    frame.vel[2] = state.heat_to_ambient_w as f32;
    frame.force_torque = state.ceiling_margin_k as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.derated {
        FLAG_THERMAL_DERATE
    } else {
        0
    };
}

fn state_is_finite(state: &ThermalNodeState) -> bool {
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
        && state.ceiling_margin_k.is_finite()
        && state.entropy_production_w_per_k.is_finite()
}

/// Forward Euler temperature against the exact boundary heat. The temperatures disagree.
#[cfg(test)]
fn step_euler(
    node: &ThermalNode,
    state: &mut ThermalNodeState,
    frame: &mut LastStateFrame64,
    load: ThermalLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(node, state, load, dt_s)?;
    let exact = match exact_map(node, state.temperature_k, load, dt_s) {
        Some(stepped) => stepped,
        None => return Err(StepReject::NonFiniteState),
    };
    let tau = node.time_constant_s();
    let equilibrium_k = load.ambient_k + load.heat_power_w * node.resistance_k_per_w;
    let euler = ExactStep {
        temperature_k: state.temperature_k + (equilibrium_k - state.temperature_k) * (dt_s / tau),
        heat_j: exact.heat_j,
    };
    commit(node, state, frame, load, dt_s, euler)
}

/// Add `extra_j` to the exact boundary heat without adding it to the plate.
#[cfg(test)]
fn step_bias_work(
    node: &ThermalNode,
    state: &mut ThermalNodeState,
    frame: &mut LastStateFrame64,
    load: ThermalLoad,
    dt_s: f64,
    extra_j: f64,
) -> Result<(), StepReject> {
    validate(node, state, load, dt_s)?;
    let mut advanced = match exact_map(node, state.temperature_k, load, dt_s) {
        Some(stepped) => stepped,
        None => return Err(StepReject::NonFiniteState),
    };
    advanced.heat_j += extra_j;
    commit(node, state, frame, load, dt_s, advanced)
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        CARD_TEMPERATURE_MAX_K, CARD_TEMPERATURE_MIN_K, DERATE_TEMPERATURE_K, FLAG_THERMAL_DERATE,
        REFERENCE_TEMPERATURE_K, RESIDUAL_CONSERVATION_LIMIT, STEP_ERROR_K, StepReject,
        THERMAL_CAPACITANCE_J_PER_K, THERMAL_RESISTANCE_K_PER_W, ThermalLoad, ThermalNode,
        ThermalNodeState, step, step_bias_work, step_euler,
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

    fn classic_map(temperature_k: f64, load: ThermalLoad, dt_s: f64) -> f64 {
        let tau = THERMAL_RESISTANCE_K_PER_W * THERMAL_CAPACITANCE_J_PER_K;
        let equilibrium_k = load.ambient_k + load.heat_power_w * THERMAL_RESISTANCE_K_PER_W;
        let decay = (-dt_s / tau).exp();
        equilibrium_k + (temperature_k - equilibrium_k) * decay
    }

    #[test]
    fn card_locks_capacitance_and_resistance_with_no_floor() {
        assert_copy::<ThermalNode>();
        assert_copy::<ThermalNodeState>();
        assert_copy::<ThermalLoad>();
        assert_copy::<StepReject>();
        let node = ThermalNode::avionics_cold_plate();
        assert_eq!(node.capacitance_j_per_k.to_bits(), 590.625_f64.to_bits());
        assert_eq!(node.resistance_k_per_w.to_bits(), 2.0_f64.to_bits());
        assert!(node.capacitance_j_per_k > 1.0e-3);
        assert!(node.resistance_k_per_w > 1.0e-6);
        let dt = node.gate_dt_s().unwrap();
        assert_eq!(dt.to_bits(), (0.02 * node.time_constant_s()).to_bits());
        let mut floored = node;
        floored.resistance_k_per_w = 1.0e-6;
        assert!(!floored.is_valid());
        floored.resistance_k_per_w = 1.0e-9;
        assert!(!floored.is_valid());
        let mut state = ThermalNodeState::try_rest(&node).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(&floored, &mut state, &mut frame, ThermalLoad::default(), dt);
        assert_eq!(got, Err(StepReject::InvalidNode));
        assert_eq!(state, saved);
        assert_eq!(frame.t, -1.0);
        proof_intact(&frame);
    }

    #[test]
    fn exact_map_matches_the_closed_form_and_the_semigroup() {
        let node = ThermalNode::avionics_cold_plate();
        let mut state = ThermalNodeState::try_rest(&node).unwrap();
        let load = ThermalLoad {
            heat_power_w: 50.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let tau = node.time_constant_s();
        let mut frame = blank_frame();
        step(&node, &mut state, &mut frame, load, tau).unwrap();
        let expected = classic_map(REFERENCE_TEMPERATURE_K, load, tau);
        assert!((state.temperature_k - expected).abs() < 1.0e-9);
        assert!(state.temperature_k > 340.0 && state.temperature_k < 370.0);
        assert!(state.residual_conservation_error < 1.0e-12);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        assert_eq!(frame.flags, 0);

        let mut split = ThermalNodeState::try_rest(&node).unwrap();
        let mut once = ThermalNodeState::try_rest(&node).unwrap();
        let half = tau * 0.5;
        step(&node, &mut split, &mut frame, load, half).unwrap();
        step(&node, &mut split, &mut frame, load, half).unwrap();
        step(&node, &mut once, &mut frame, load, tau).unwrap();
        assert!((split.temperature_k - once.temperature_k).abs() < 1.0e-9);
        assert!(split.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    #[test]
    fn rest_at_huge_dt_holds_the_anchor() {
        let node = ThermalNode::avionics_cold_plate();
        let mut state = ThermalNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        step(
            &node,
            &mut state,
            &mut frame,
            ThermalLoad::default(),
            f64::MAX,
        )
        .unwrap();
        assert_eq!(
            state.temperature_k.to_bits(),
            REFERENCE_TEMPERATURE_K.to_bits()
        );
        assert_eq!(state.residual_conservation_error, 0.0);
        assert_eq!(state.energy_defect_j, 0.0);
        proof_intact(&frame);
    }

    #[test]
    fn gate_clock_is_not_applied() {
        let node = ThermalNode::avionics_cold_plate();
        let mut state = ThermalNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        let dt = node.gate_dt_s().unwrap() * 10.0;
        step(&node, &mut state, &mut frame, ThermalLoad::default(), dt).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    #[test]
    fn derate_flags_and_the_ceiling_refuses() {
        let node = ThermalNode::avionics_cold_plate();
        let mut state = ThermalNodeState::try_phase(&node, 350.0).unwrap();
        let mut frame = blank_frame();
        let load = ThermalLoad {
            heat_power_w: (370.0 - REFERENCE_TEMPERATURE_K) / THERMAL_RESISTANCE_K_PER_W,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let tau = node.time_constant_s();
        step(
            &node,
            &mut state,
            &mut frame,
            load,
            tau * core::f64::consts::LN_2,
        )
        .unwrap();
        assert!(state.temperature_k > DERATE_TEMPERATURE_K);
        assert!(state.temperature_k < CARD_TEMPERATURE_MAX_K);
        assert!(state.derated);
        assert_eq!(frame.flags, FLAG_THERMAL_DERATE);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);

        let hot = ThermalLoad {
            heat_power_w: 200.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let before = state;
        let sentinel = blank_frame();
        let mut refused = sentinel;
        let got = step(&node, &mut state, &mut refused, hot, tau);
        assert_eq!(got, Err(StepReject::TemperatureOutsideCard));
        assert_eq!(state, before);
        assert_eq!(refused.t, sentinel.t);
        proof_intact(&refused);
    }

    #[test]
    fn euler_temperature_against_exact_heat_is_refused() {
        let node = ThermalNode::avionics_cold_plate();
        let mut state = ThermalNodeState::try_rest(&node).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let load = ThermalLoad {
            heat_power_w: 50.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let got = step_euler(&node, &mut state, &mut frame, load, node.time_constant_s());
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        assert_eq!(frame.t, -1.0);
        proof_intact(&frame);
    }

    #[test]
    fn biased_heat_is_a_first_law_breach_and_rolls_back() {
        let node = ThermalNode::avionics_cold_plate();
        let mut state = ThermalNodeState::try_rest(&node).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let extra = node.card_energy_j() * 1.0e-3;
        let got = step_bias_work(
            &node,
            &mut state,
            &mut frame,
            ThermalLoad::default(),
            node.gate_dt_s().unwrap(),
            extra,
        );
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state, saved);
        proof_intact(&frame);
        let got = step_bias_work(
            &node,
            &mut state,
            &mut frame,
            ThermalLoad::default(),
            node.gate_dt_s().unwrap(),
            -extra,
        );
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        proof_intact(&frame);
    }

    #[test]
    fn boundaries_and_non_finite_inputs_do_not_panic() {
        let node = ThermalNode::avionics_cold_plate();
        let mut state = ThermalNodeState::try_rest(&node).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let dt = node.gate_dt_s().unwrap();
        let mut refuse = |load: ThermalLoad, dt_s: f64, expect: StepReject| {
            let got = step(&node, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state.temperature_k, saved.temperature_k);
            proof_intact(&frame);
        };
        refuse(ThermalLoad::default(), 0.0, StepReject::NonPositiveDt);
        refuse(ThermalLoad::default(), -1.0, StepReject::NonPositiveDt);
        refuse(ThermalLoad::default(), f64::NAN, StepReject::NonFiniteInput);
        refuse(
            ThermalLoad::default(),
            f64::INFINITY,
            StepReject::NonFiniteInput,
        );
        refuse(
            ThermalLoad {
                heat_power_w: f64::NAN,
                ambient_k: REFERENCE_TEMPERATURE_K,
            },
            dt,
            StepReject::NonFiniteInput,
        );
        refuse(
            ThermalLoad {
                heat_power_w: 0.0,
                ambient_k: 0.0,
            },
            dt,
            StepReject::NegativeTemperature,
        );
        refuse(
            ThermalLoad {
                heat_power_w: 0.0,
                ambient_k: -10.0,
            },
            dt,
            StepReject::NegativeTemperature,
        );
        refuse(
            ThermalLoad {
                heat_power_w: 0.0,
                ambient_k: 100.0,
            },
            dt,
            StepReject::TemperatureOutsideCard,
        );
        refuse(
            ThermalLoad {
                heat_power_w: 0.0,
                ambient_k: CARD_TEMPERATURE_MAX_K,
            },
            dt,
            StepReject::TemperatureOutsideCard,
        );
        refuse(
            ThermalLoad {
                heat_power_w: f64::MAX,
                ambient_k: REFERENCE_TEMPERATURE_K,
            },
            dt,
            StepReject::NonFiniteState,
        );
        state.temperature_k = f64::from_bits(1);
        let got = step(&node, &mut state, &mut frame, ThermalLoad::default(), dt);
        assert_eq!(got, Err(StepReject::TemperatureOutsideCard));
        proof_intact(&frame);
        assert!(ThermalNodeState::try_phase(&node, CARD_TEMPERATURE_MIN_K - 1.0).is_none());
        assert!(ThermalNodeState::try_phase(&node, CARD_TEMPERATURE_MAX_K).is_none());
        assert!(ThermalNodeState::try_phase(&node, f64::NAN).is_none());
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::ThermalNode::avionics_cold_plate();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The plate at the bath temperature with a 25 W heater.
        let mut state = super::ThermalNodeState::try_phase(&card, super::REFERENCE_TEMPERATURE_K).unwrap();
        let load = super::ThermalLoad {
            heat_power_w: 25.0,
            ambient_k: super::REFERENCE_TEMPERATURE_K,
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
        assert!(state.temperature_k != before.temperature_k);
        assert!(state.internal_energy_j != before.internal_energy_j);
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

    /// Run `steps` steps of `dt_s` from `temperature_k` under one load.
    fn run(temperature_k: f64, load: ThermalLoad, dt_s: f64, steps: u64) -> ThermalNodeState {
        let node = ThermalNode::avionics_cold_plate();
        let mut state = ThermalNodeState::try_phase(&node, temperature_k).unwrap();
        let mut frame = blank_frame();
        let mut taken = 0;
        while taken < steps {
            step(&node, &mut state, &mut frame, load, dt_s).unwrap();
            taken += 1;
        }
        state
    }

    #[test]
    fn the_step_error_is_two_to_the_minus_forty_two_kelvin() {
        assert_eq!(STEP_ERROR_K.to_bits(), 2.0_f64.powi(-42).to_bits());
        // Four units in the last place of the hottest temperature the card holds.
        let hottest = f64::from_bits(CARD_TEMPERATURE_MAX_K.to_bits() - 1);
        let ulp = f64::from_bits(hottest.to_bits() + 1) - hottest;
        assert_eq!((4.0 * ulp).to_bits(), STEP_ERROR_K.to_bits());
    }

    /// A million floor steps of 1e-6 W from equilibrium land rises of zero or
    /// one unit in the last place. The heat booked is the heat of the rise
    /// that landed, so the ledger is exact, and the run agrees with one step
    /// over the same span within a million and one times the step error.
    #[test]
    fn a_million_floor_steps_near_equilibrium_book_the_heat_that_landed() {
        let node = ThermalNode::avionics_cold_plate();
        let floor_s = node.floor_dt_s().unwrap();
        let steps: u64 = 1_000_000;
        let span_s = floor_s * steps as f64;
        for heat_power_w in [1.0e-6, 7.0e-7] {
            let load = ThermalLoad {
                heat_power_w,
                ambient_k: REFERENCE_TEMPERATURE_K,
            };
            let many = run(REFERENCE_TEMPERATURE_K, load, floor_s, steps);
            let one = run(REFERENCE_TEMPERATURE_K, load, span_s, 1);
            let rise_k = many.temperature_k - REFERENCE_TEMPERATURE_K;
            let landed_j = THERMAL_CAPACITANCE_J_PER_K * rise_k;
            assert!(
                (many.boundary_work_j - landed_j).abs() <= 1.0e-18,
                "{heat_power_w} W: booked {} J for a landed rise worth {landed_j} J",
                many.boundary_work_j
            );
            assert!(many.energy_defect_j.abs() <= 1.0e-18, "{}", many.energy_defect_j);
            assert!(one.energy_defect_j.abs() <= 1.0e-18, "{}", one.energy_defect_j);
            let gap_k = (many.temperature_k - one.temperature_k).abs();
            assert!(
                gap_k <= (steps + 1) as f64 * STEP_ERROR_K,
                "{heat_power_w} W: a million steps and one differ by {gap_k} K"
            );
            assert!(one.temperature_k > REFERENCE_TEMPERATURE_K);
        }
        // At 7e-7 W every floor rise is under half a unit in the last place:
        // the plate does not move and no heat is booked.
        let still = run(
            REFERENCE_TEMPERATURE_K,
            ThermalLoad {
                heat_power_w: 7.0e-7,
                ambient_k: REFERENCE_TEMPERATURE_K,
            },
            floor_s,
            steps,
        );
        assert_eq!(still.temperature_k.to_bits(), REFERENCE_TEMPERATURE_K.to_bits());
        assert_eq!(still.boundary_work_j, 0.0);
        assert_eq!(still.energy_defect_j, 0.0);
    }

    /// Many steps agree with one step over the same span within the step
    /// error times one more than the count, from either edge of the card,
    /// at the floor and at the ceiling, and under a megawatt either way.
    #[test]
    fn many_steps_and_one_agree_within_the_stated_step_error() {
        let node = ThermalNode::avionics_cold_plate();
        let ceiling_s = node.gate_dt_s().unwrap();
        let floor_s = node.floor_dt_s().unwrap();
        let cases: [(f64, f64, f64, f64, u64); 8] = [
            (293.15, 25.0, 293.15, ceiling_s / 7.0, 7),
            (233.15, 0.0, 398.1499999999999, ceiling_s / 3.0, 3),
            (398.1499999999999, -50.0, 233.15, ceiling_s / 5.0, 5),
            (300.0, 1.0e-9, 300.0, floor_s, 4096),
            (350.0, 3.0, 290.0, ceiling_s / 64.0, 64),
            (240.0, 1.0e6, 233.15, floor_s, 2),
            (397.0, -1.0e6, 398.0, floor_s, 2),
            (293.15, 1.0e-6, 293.15, floor_s * 16.0, 1000),
        ];
        for (start_k, heat_power_w, ambient_k, dt_s, steps) in cases {
            let load = ThermalLoad {
                heat_power_w,
                ambient_k,
            };
            let state = run(start_k, load, dt_s, steps);
            let one = run(start_k, load, dt_s * steps as f64, 1);
            let gap_k = (state.temperature_k - one.temperature_k).abs();
            assert!(
                gap_k <= (steps + 1) as f64 * STEP_ERROR_K,
                "{start_k} K, {heat_power_w} W: {steps} steps and one differ by {gap_k} K"
            );
            // The energies are 590.625 J/K times temperatures of a few hundred
            // kelvin; their own rounding is a few units of 7e-12 J.
            assert!(state.energy_defect_j.abs() <= 1.0e-10, "{}", state.energy_defect_j);
        }
    }
}
