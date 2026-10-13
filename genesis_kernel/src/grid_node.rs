#![no_implicit_prelude]

//! Copper fuse link at one power-grid node.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, feeder current, node voltage, copper
//! temperature, Joule power, thermal energy, the voltage-spike peak, the
//! melt margin, and the conservation residual into [`LastStateFrame64`].
//! The 16-byte frame proof is not touched.
//!
//! The card is a 48 V DC-plant node. The feeder is 50 µH into a 50 µF
//! shunt, so the undamped ring is 20 krad/s. The gate step is one
//! microsecond (`ω Δt = 0.02`). The copper that melts is an AWG 34 fuse
//! link, 0.1601 mm by 120 mm. Its heat capacity is the solid value and it
//! is constant. The model stops at the melt point rather than inventing a
//! latent-heat phase change.
//!
//! ```text
//! L dI/dt = V_up − V − I R(T)
//! C dV/dt = I − I_load
//! R(T) = R_ref (1 + α (T − T_ref))
//! C_th dT/dt = I² R(T) − G_th (T − T_amb)
//! U = ½ L I² + ½ C V² + C_th (T − T_ref)
//! dW/dt = V_up I − V I_load − G_th (T − T_amb)
//! residual_conservation_error = |U − U_anchor − W| / U_melt
//! ```
//!
//! `I² R` is an internal conversion. It leaves the inductor and enters the
//! copper, so it is not boundary work. `U_melt` is the sensible heat from
//! the reference temperature to the melt point. Source work, load work, and
//! heat leaving to the ambient bath are the boundary.
//!
//! The public step is classical RK4 on `(I, V, T, W)` together. A split
//! step that advances the circuit and then adds `I_left² R Δt` does not
//! match the ohmic work, and the residual leaves the gate. Explicit Euler
//! is not the public step: on this card it creates energy on the first
//! microsecond.
//!
//! A trial temperature at or above [`COPPER_MELT_K`] (1357.77 K) is
//! [`StepReject::CopperMelt`]. The state and the frame stay as they were.
//! Melt point, resistance temperature coefficient, reference temperature,
//! card floor, and the 120 V surge ceiling are crate constants.
//! [`GridNode::try_new`] does not accept them. A node whose recorded copies
//! of those constants differ is not valid. A node voltage past 120 V DC sets
//! [`FLAG_GRID_SURGE`] and does not reject the step. That is the
//! extra-low-voltage ceiling. The hard stop is the melt.
//!
//! The card's electrical bounds are [`GridNode::melt_current_a`] and
//! [`GridNode::melt_voltage_v`]: the feeder current and the node voltage
//! whose stored energy equals the sensible heat that takes the link from the
//! reference temperature to the melt point. A trial step that lands with
//! `|I|` above the first is [`StepReject::CurrentOutsideCard`], and one that
//! lands with `|V|` above the second is [`StepReject::VoltageOutsideCard`].
//! Both bounds are inclusive and are checked on the exact landed values. A
//! held command well inside the bounds can ring the node past them, and that
//! step is refused.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Annealed copper resistivity at 20 °C, IACS.
pub const COPPER_RESISTIVITY_OHM_M: f64 = 1.724e-8;
/// Linear resistance temperature coefficient of annealed copper.
pub const COPPER_TEMP_COEFF_PER_K: f64 = 3.93e-3;
/// Solid density of copper.
pub const COPPER_DENSITY_KG_M3: f64 = 8_960.0;
/// Solid specific heat used across this card. It is not the liquid value.
pub const COPPER_SPECIFIC_HEAT_J_PER_KG_K: f64 = 385.0;
/// Melting point of copper, 1084.62 °C.
pub const COPPER_MELT_K: f64 = 1_357.77;
/// 20 °C, the resistivity reference.
pub const REFERENCE_TEMPERATURE_K: f64 = 293.15;
/// −40 °C. Below this the outdoor-gear card is not this model.
pub const CARD_TEMPERATURE_MIN_K: f64 = 233.15;
/// Catalog upstream step of the 48 V DC plant.
pub const CATALOG_VOLTAGE_V: f64 = 48.0;
/// Feeder inductance seen by the fuse link.
pub const FEEDER_INDUCTANCE_H: f64 = 50.0e-6;
/// Shunt capacitance at the node.
pub const NODE_CAPACITANCE_F: f64 = 50.0e-6;
/// AWG 34 bare diameter, 0.1601 mm.
pub const FUSE_DIAMETER_M: f64 = 1.601e-4;
/// Length of the fuse link.
pub const FUSE_LENGTH_M: f64 = 0.120;
/// Forced cabinet air on the bare link.
pub const FORCED_CONVECTION_W_PER_M2_K: f64 = 20.0;
/// Extra-low-voltage DC ceiling, IEC 61140. Past this the frame is flagged.
pub const SURGE_WITHSTAND_V: f64 = 120.0;

/// `|U − U_anchor − W| / U_melt` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Electrical phase advance `ω Δt` used by [`GridNode::gate_dt_s`].
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// `|node voltage|` is at or above [`SURGE_WITHSTAND_V`] on this frame.
pub const FLAG_GRID_SURGE: u64 = 1 << 0;

/// Why a step did not advance the node or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Voltage, current, ambient, or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// Temperature or ambient is below zero, or exactly zero.
    NegativeTemperature,
    /// Temperature or ambient is outside the solid copper card.
    TemperatureOutsideCard,
    /// Inductance, capacitance, resistance, or thermal capacity cannot support the card.
    InvalidNode,
    /// The trial copper temperature reached the melt point.
    ///
    /// The solid heat capacity does not cross the phase change. The step is
    /// refused and nothing is written.
    CopperMelt,
    /// The trial step created energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed energy.
    FirstLawBreach,
    /// The trial feeder current is past the card's current bound,
    /// [`GridNode::melt_current_a`].
    CurrentOutsideCard,
    /// The trial node voltage is past the card's voltage bound,
    /// [`GridNode::melt_voltage_v`].
    VoltageOutsideCard,
}

/// Zero-order hold applied across one microsecond.
///
/// `upstream_voltage_v` and `load_current_a` are the load-balancing command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLoad {
    pub upstream_voltage_v: f64,
    pub load_current_a: f64,
    pub ambient_k: f64,
}

impl Default for GridLoad {
    fn default() -> Self {
        Self {
            upstream_voltage_v: 0.0,
            load_current_a: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        }
    }
}

/// Lumped feeder, shunt, and the copper fuse link that carries the current.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridNode {
    pub inductance_h: f64,
    pub capacitance_f: f64,
    pub resistance_ref_ohm: f64,
    /// Recorded copy of [`COPPER_TEMP_COEFF_PER_K`]. Not a caller input.
    pub temp_coeff_per_k: f64,
    /// Recorded copy of [`REFERENCE_TEMPERATURE_K`]. Not a caller input.
    pub reference_k: f64,
    pub thermal_capacity_j_per_k: f64,
    pub thermal_conductance_w_per_k: f64,
    /// Recorded copy of [`CARD_TEMPERATURE_MIN_K`]. Not a caller input.
    pub temperature_min_k: f64,
    /// Recorded copy of [`COPPER_MELT_K`] (1357.77 K). Not a caller input.
    pub melt_k: f64,
    /// Recorded copy of [`SURGE_WITHSTAND_V`]. Not a caller input.
    pub surge_withstand_v: f64,
}

impl GridNode {
    /// Card: AWG 34 copper fuse link on the 48 V node.
    #[must_use]
    pub fn copper_fuse_link() -> Self {
        let area_m2 = circle_area_m2(FUSE_DIAMETER_M);
        let node = Self::try_new(
            FEEDER_INDUCTANCE_H,
            NODE_CAPACITANCE_F,
            COPPER_RESISTIVITY_OHM_M * FUSE_LENGTH_M / area_m2,
            COPPER_DENSITY_KG_M3 * FUSE_LENGTH_M * area_m2 * COPPER_SPECIFIC_HEAT_J_PER_KG_K,
            FORCED_CONVECTION_W_PER_M2_K * core::f64::consts::PI * FUSE_DIAMETER_M * FUSE_LENGTH_M,
        );
        match node {
            Some(node) => node,
            None => ::core::panic!("copper fuse link card failed is_valid"),
        }
    }

    /// Circuit specimen only. Melt point, α, reference temperature, card floor,
    /// and surge ceiling are written from the copper constants.
    #[must_use]
    pub fn try_new(
        inductance_h: f64,
        capacitance_f: f64,
        resistance_ref_ohm: f64,
        thermal_capacity_j_per_k: f64,
        thermal_conductance_w_per_k: f64,
    ) -> Option<Self> {
        let node = Self {
            inductance_h,
            capacitance_f,
            resistance_ref_ohm,
            temp_coeff_per_k: COPPER_TEMP_COEFF_PER_K,
            reference_k: REFERENCE_TEMPERATURE_K,
            thermal_capacity_j_per_k,
            thermal_conductance_w_per_k,
            temperature_min_k: CARD_TEMPERATURE_MIN_K,
            melt_k: COPPER_MELT_K,
            surge_withstand_v: SURGE_WITHSTAND_V,
        };
        if node.is_valid() { Some(node) } else { None }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.inductance_h.is_finite()
            && self.capacitance_f.is_finite()
            && self.resistance_ref_ohm.is_finite()
            && self.temp_coeff_per_k.is_finite()
            && self.reference_k.to_bits() == REFERENCE_TEMPERATURE_K.to_bits()
            && self.thermal_capacity_j_per_k.is_finite()
            && self.thermal_conductance_w_per_k.is_finite()
            && self.temperature_min_k.to_bits() == CARD_TEMPERATURE_MIN_K.to_bits()
            && self.melt_k.to_bits() == COPPER_MELT_K.to_bits()
            && self.surge_withstand_v.to_bits() == SURGE_WITHSTAND_V.to_bits()
            && self.inductance_h > 0.0
            && self.capacitance_f > 0.0
            && self.resistance_ref_ohm > 0.0
            && self.temp_coeff_per_k >= 0.0
            && self.reference_k > 0.0
            && self.temperature_min_k > 0.0
            && self.thermal_capacity_j_per_k > 0.0
            && self.thermal_conductance_w_per_k >= 0.0
            && self.surge_withstand_v > 0.0
            && self.melt_k > self.reference_k
            && self.reference_k >= self.temperature_min_k
            && self.reference_k < self.melt_k
            && self.resistance_ohm(self.temperature_min_k) > 0.0
            && self.melt_enthalpy_j() > 0.0
            && self.melt_enthalpy_j().is_finite()
            && self.natural_frequency_rad_s().is_finite()
            && self.natural_frequency_rad_s() > 0.0
    }

    /// `R_ref (1 + α (T − T_ref))`. `T_ref` is [`REFERENCE_TEMPERATURE_K`].
    /// The card sets α to [`COPPER_TEMP_COEFF_PER_K`].
    #[must_use]
    pub fn resistance_ohm(&self, temperature_k: f64) -> f64 {
        self.resistance_ref_ohm
            * (1.0 + self.temp_coeff_per_k * (temperature_k - REFERENCE_TEMPERATURE_K))
    }

    /// Sensible heat from [`REFERENCE_TEMPERATURE_K`] to [`COPPER_MELT_K`].
    #[must_use]
    pub fn melt_enthalpy_j(&self) -> f64 {
        self.thermal_capacity_j_per_k * (COPPER_MELT_K - REFERENCE_TEMPERATURE_K)
    }

    /// The feeder current whose energy in the inductance equals
    /// [`Self::melt_enthalpy_j`]. `|I|` past it is outside the card.
    #[must_use]
    pub fn melt_current_a(&self) -> f64 {
        (2.0 * self.melt_enthalpy_j() / self.inductance_h).sqrt()
    }

    /// The node voltage whose energy in the shunt capacitance equals
    /// [`Self::melt_enthalpy_j`]. `|V|` past it is outside the card.
    #[must_use]
    pub fn melt_voltage_v(&self) -> f64 {
        (2.0 * self.melt_enthalpy_j() / self.capacitance_f).sqrt()
    }

    /// Undamped feeder frequency `1 / sqrt(L C)`.
    #[must_use]
    pub fn natural_frequency_rad_s(&self) -> f64 {
        1.0 / (self.inductance_h * self.capacitance_f).sqrt()
    }

    /// Largest step at which the conservation gate was closed. One microsecond on the card.
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let omega = self.natural_frequency_rad_s();
        if omega.is_finite() && omega > 0.0 {
            Some(GATE_PHASE_STEP_RAD / omega)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step turns the ring of the feeder and the shunt by two hundredths of a radian. A million times smaller, a step
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

    #[must_use]
    pub fn electrical_energy_j(&self, current_a: f64, voltage_v: f64) -> f64 {
        0.5 * self.inductance_h * current_a * current_a
            + 0.5 * self.capacitance_f * voltage_v * voltage_v
    }

    #[must_use]
    pub fn thermal_energy_j(&self, temperature_k: f64) -> f64 {
        self.thermal_capacity_j_per_k * (temperature_k - REFERENCE_TEMPERATURE_K)
    }

    /// `U = ½ L I² + ½ C V² + C_th (T − T_ref)`.
    #[must_use]
    pub fn internal_energy_j(&self, current_a: f64, voltage_v: f64, temperature_k: f64) -> f64 {
        self.electrical_energy_j(current_a, voltage_v) + self.thermal_energy_j(temperature_k)
    }

    /// `I²R / T + G_th (T − T_amb)² / (T T_amb)`. Negative temperature returns NaN.
    #[must_use]
    pub fn entropy_production_w_per_k(
        &self,
        current_a: f64,
        temperature_k: f64,
        ambient_k: f64,
    ) -> f64 {
        if !(temperature_k > 0.0) || !(ambient_k > 0.0) {
            return f64::NAN;
        }
        let joule_w = current_a * current_a * self.resistance_ohm(temperature_k);
        let delta_k = temperature_k - ambient_k;
        joule_w / temperature_k
            + self.thermal_conductance_w_per_k * delta_k * delta_k / (temperature_k * ambient_k)
    }
}

/// Integrated node. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridNodeState {
    pub t_s: f64,
    pub current_a: f64,
    pub voltage_v: f64,
    pub temperature_k: f64,
    pub upstream_voltage_v: f64,
    pub load_current_a: f64,
    pub ambient_k: f64,
    pub resistance_ohm: f64,
    pub joule_power_w: f64,
    pub electrical_energy_j: f64,
    pub thermal_energy_j: f64,
    pub internal_energy_j: f64,
    pub energy_anchor_j: f64,
    pub boundary_work_j: f64,
    /// Signed `U − U_anchor − W`. Positive is created energy.
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    /// Largest `|V|` on an accepted step of this state.
    pub voltage_spike_v: f64,
    pub melt_margin_k: f64,
    pub entropy_production_w_per_k: f64,
    pub surge: bool,
}

impl GridNodeState {
    /// Cold feeder: zero current, zero voltage, reference temperature.
    #[must_use]
    pub fn try_rest(node: &GridNode) -> Option<Self> {
        Self::try_phase(node, 0.0, 0.0, REFERENCE_TEMPERATURE_K)
    }

    /// Anchor the ledger at this current, voltage, and copper temperature.
    /// The held command starts at zero upstream voltage, zero load current,
    /// and the reference ambient.
    #[must_use]
    pub fn try_phase(
        node: &GridNode,
        current_a: f64,
        voltage_v: f64,
        temperature_k: f64,
    ) -> Option<Self> {
        if !node.is_valid()
            || !current_a.is_finite()
            || !voltage_v.is_finite()
            || !(temperature_k > 0.0)
            || temperature_k < CARD_TEMPERATURE_MIN_K
            || temperature_k >= COPPER_MELT_K
        {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            current_a,
            voltage_v,
            temperature_k,
            upstream_voltage_v: 0.0,
            load_current_a: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
            resistance_ohm: 0.0,
            joule_power_w: 0.0,
            electrical_energy_j: 0.0,
            thermal_energy_j: 0.0,
            internal_energy_j: 0.0,
            energy_anchor_j: 0.0,
            boundary_work_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            voltage_spike_v: 0.0,
            melt_margin_k: 0.0,
            entropy_production_w_per_k: 0.0,
            surge: false,
        };
        refresh(node, &mut state);
        if !state_is_finite(&state) {
            return None;
        }
        state.energy_anchor_j = state.internal_energy_j;
        state.boundary_work_j = 0.0;
        state.energy_defect_j = 0.0;
        state.residual_conservation_error = 0.0;
        state.voltage_spike_v = voltage_v.abs();
        Some(state)
    }
}

/// Advance one RK4 step and write the frame.
///
/// [`StepReject::CopperMelt`] refuses a trial temperature at or above the
/// melt point. [`StepReject::CurrentOutsideCard`] and
/// [`StepReject::VoltageOutsideCard`] refuse a trial current or voltage past
/// the card's bounds. A rejected step leaves `state` and `frame` as they were.
/// Creation of energy is [`StepReject::SpontaneousGeneration`].
#[must_use]
pub fn step(
    node: &GridNode,
    state: &mut GridNodeState,
    frame: &mut LastStateFrame64,
    load: GridLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(node, state, load, dt_s)?;
    let predicted = match rk4(node, pack(state), dt_s, load) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(node, state, frame, load, dt_s, predicted)
}

fn validate(
    node: &GridNode,
    state: &GridNodeState,
    load: GridLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite()
        || !load.upstream_voltage_v.is_finite()
        || !load.load_current_a.is_finite()
        || !load.ambient_k.is_finite()
    {
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
    if load.ambient_k < CARD_TEMPERATURE_MIN_K || load.ambient_k >= COPPER_MELT_K {
        return Err(StepReject::TemperatureOutsideCard);
    }
    if state.temperature_k >= COPPER_MELT_K {
        return Err(StepReject::CopperMelt);
    }
    if state.temperature_k < CARD_TEMPERATURE_MIN_K {
        return Err(StepReject::TemperatureOutsideCard);
    }
    Ok(())
}

fn commit(
    node: &GridNode,
    state: &mut GridNodeState,
    frame: &mut LastStateFrame64,
    load: GridLoad,
    dt_s: f64,
    predicted: [f64; 4],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite4(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    if !(predicted[2] > 0.0) {
        return Err(StepReject::NegativeTemperature);
    }
    if predicted[2] >= COPPER_MELT_K {
        return Err(StepReject::CopperMelt);
    }
    if predicted[2] < CARD_TEMPERATURE_MIN_K {
        return Err(StepReject::TemperatureOutsideCard);
    }
    if predicted[0].abs() > node.melt_current_a() {
        return Err(StepReject::CurrentOutsideCard);
    }
    if predicted[1].abs() > node.melt_voltage_v() {
        return Err(StepReject::VoltageOutsideCard);
    }
    let resistance = node.resistance_ohm(predicted[2]);
    if !(resistance.is_finite() && resistance > 0.0) {
        return Err(StepReject::InvalidNode);
    }
    state.current_a = predicted[0];
    state.voltage_v = predicted[1];
    state.temperature_k = predicted[2];
    state.boundary_work_j = predicted[3];
    state.upstream_voltage_v = load.upstream_voltage_v;
    state.load_current_a = load.load_current_a;
    state.ambient_k = load.ambient_k;
    state.t_s += dt_s;
    refresh(node, state);
    let scale = node.melt_enthalpy_j();
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
    let observed = state.voltage_v.abs();
    if observed > saved.voltage_spike_v {
        state.voltage_spike_v = observed;
    } else {
        state.voltage_spike_v = saved.voltage_spike_v;
    }
    publish(state, frame);
    Ok(())
}

fn refresh(node: &GridNode, state: &mut GridNodeState) {
    let resistance = node.resistance_ohm(state.temperature_k);
    state.resistance_ohm = resistance;
    state.joule_power_w = state.current_a * state.current_a * resistance;
    state.electrical_energy_j = node.electrical_energy_j(state.current_a, state.voltage_v);
    state.thermal_energy_j = node.thermal_energy_j(state.temperature_k);
    state.internal_energy_j = state.electrical_energy_j + state.thermal_energy_j;
    state.melt_margin_k = COPPER_MELT_K - state.temperature_k;
    state.entropy_production_w_per_k =
        node.entropy_production_w_per_k(state.current_a, state.temperature_k, state.ambient_k);
    state.surge = state.voltage_v.abs() >= SURGE_WITHSTAND_V;
}

fn publish(state: &GridNodeState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.current_a as f32;
    frame.pos[1] = state.voltage_v as f32;
    frame.pos[2] = state.temperature_k as f32;
    frame.vel[0] = state.joule_power_w as f32;
    frame.vel[1] = state.thermal_energy_j as f32;
    frame.vel[2] = state.voltage_spike_v as f32;
    frame.force_torque = state.melt_margin_k as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.surge { FLAG_GRID_SURGE } else { 0 };
}

fn derivatives(node: &GridNode, y: [f64; 4], load: GridLoad) -> Option<[f64; 4]> {
    let current_a = y[0];
    let voltage_v = y[1];
    let temperature_k = y[2];
    let resistance = node.resistance_ohm(temperature_k);
    if !(resistance.is_finite() && resistance > 0.0) {
        return None;
    }
    if !(node.inductance_h > 0.0)
        || !(node.capacitance_f > 0.0)
        || !(node.thermal_capacity_j_per_k > 0.0)
    {
        return None;
    }
    let joule_w = current_a * current_a * resistance;
    let cooling_w = node.thermal_conductance_w_per_k * (temperature_k - load.ambient_k);
    let d_current =
        (load.upstream_voltage_v - voltage_v - current_a * resistance) / node.inductance_h;
    let d_voltage = (current_a - load.load_current_a) / node.capacitance_f;
    let d_temperature = (joule_w - cooling_w) / node.thermal_capacity_j_per_k;
    // Joule heat is already in `d_temperature` and in the ohmic drop.
    // It is not added here again.
    let d_work = load.upstream_voltage_v * current_a - voltage_v * load.load_current_a - cooling_w;
    let out = [d_current, d_voltage, d_temperature, d_work];
    if finite4(out) { Some(out) } else { None }
}

fn rk4(node: &GridNode, y: [f64; 4], dt: f64, load: GridLoad) -> Option<[f64; 4]> {
    let k1 = derivatives(node, y, load)?;
    let k2 = derivatives(node, axpy(y, dt * 0.5, k1), load)?;
    let k3 = derivatives(node, axpy(y, dt * 0.5, k2), load)?;
    let k4 = derivatives(node, axpy(y, dt, k3), load)?;
    let mut next = y;
    let mut i = 0;
    while i < 4 {
        next[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        i += 1;
    }
    if finite4(next) { Some(next) } else { None }
}

fn axpy(y: [f64; 4], scale: f64, k: [f64; 4]) -> [f64; 4] {
    [
        y[0] + scale * k[0],
        y[1] + scale * k[1],
        y[2] + scale * k[2],
        y[3] + scale * k[3],
    ]
}

fn pack(state: &GridNodeState) -> [f64; 4] {
    [
        state.current_a,
        state.voltage_v,
        state.temperature_k,
        state.boundary_work_j,
    ]
}

fn circle_area_m2(diameter_m: f64) -> f64 {
    0.25 * core::f64::consts::PI * diameter_m * diameter_m
}

fn finite4(x: [f64; 4]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite() && x[3].is_finite()
}

fn state_is_finite(state: &GridNodeState) -> bool {
    state.t_s.is_finite()
        && state.current_a.is_finite()
        && state.voltage_v.is_finite()
        && state.temperature_k.is_finite()
        && state.upstream_voltage_v.is_finite()
        && state.load_current_a.is_finite()
        && state.ambient_k.is_finite()
        && state.resistance_ohm.is_finite()
        && state.joule_power_w.is_finite()
        && state.electrical_energy_j.is_finite()
        && state.thermal_energy_j.is_finite()
        && state.internal_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.boundary_work_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.voltage_spike_v.is_finite()
        && state.melt_margin_k.is_finite()
        && state.entropy_production_w_per_k.is_finite()
}

/// Explicit Euler on `(I, V, T, W)`. One gate step of the catalog voltage creates energy.
#[cfg(test)]
fn step_euler(
    node: &GridNode,
    state: &mut GridNodeState,
    frame: &mut LastStateFrame64,
    load: GridLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(node, state, load, dt_s)?;
    let predicted = match explicit_euler(node, pack(state), dt_s, load) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(node, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
fn explicit_euler(node: &GridNode, y: [f64; 4], dt: f64, load: GridLoad) -> Option<[f64; 4]> {
    let k1 = derivatives(node, y, load)?;
    let next = axpy(y, dt, k1);
    if finite4(next) { Some(next) } else { None }
}

/// Add `delta_k` to the trial temperature after a faithful RK4 step.
/// Positive invents heat. Negative destroys it. Neither books the work.
#[cfg(test)]
fn step_shift_temperature(
    node: &GridNode,
    state: &mut GridNodeState,
    frame: &mut LastStateFrame64,
    load: GridLoad,
    dt_s: f64,
    delta_k: f64,
) -> Result<(), StepReject> {
    validate(node, state, load, dt_s)?;
    let mut predicted = match rk4(node, pack(state), dt_s, load) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    predicted[2] += delta_k;
    commit(node, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        CARD_TEMPERATURE_MIN_K, CATALOG_VOLTAGE_V, COPPER_DENSITY_KG_M3, COPPER_MELT_K,
        COPPER_RESISTIVITY_OHM_M, COPPER_SPECIFIC_HEAT_J_PER_KG_K, COPPER_TEMP_COEFF_PER_K,
        FEEDER_INDUCTANCE_H, FLAG_GRID_SURGE, FORCED_CONVECTION_W_PER_M2_K, FUSE_DIAMETER_M,
        FUSE_LENGTH_M, GATE_PHASE_STEP_RAD, GridLoad, GridNode, GridNodeState, NODE_CAPACITANCE_F,
        REFERENCE_TEMPERATURE_K, RESIDUAL_CONSERVATION_LIMIT, SURGE_WITHSTAND_V, StepReject,
        derivatives, step, step_euler, step_shift_temperature,
    };
    use crate::frame::LastStateFrame64;

    const SPIKE_STEPS: u32 = 5_000;
    /// Resonant square wave that stays solid for [`SPIKE_STEPS`] gate steps.
    const EXTREME_SURVIVING_STEP_V: f64 = 25.0;

    fn blank_frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: -1.0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            force_torque: 42.0,
            residual: 42.0,
            flags: u64::MAX,
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

    fn adiabatic(node: &GridNode) -> GridNode {
        let mut cold = *node;
        cold.thermal_conductance_w_per_k = 0.0;
        assert!(cold.is_valid());
        cold
    }

    fn half_steps(node: &GridNode) -> u32 {
        let dt = node.gate_dt_s().unwrap();
        let half_s = core::f64::consts::PI / node.natural_frequency_rad_s();
        let n = (half_s / dt).round();
        assert!(n >= 1.0);
        n as u32
    }

    fn square_voltage(step_index: u32, amplitude_v: f64, half: u32) -> f64 {
        let block = step_index / half;
        if block % 2 == 0 {
            amplitude_v
        } else {
            -amplitude_v
        }
    }

    struct Report {
        max_residual: f64,
        temperature_k: f64,
        peak_voltage_v: f64,
        thermal_energy_j: f64,
        saw_surge: bool,
        audit_residual: f64,
        work_residual: f64,
    }

    enum Drive {
        Survived(Report),
        Rejected {
            reason: StepReject,
            step: u32,
            temperature_k: f64,
        },
    }

    fn drive(
        node: &GridNode,
        state: &mut GridNodeState,
        frame: &mut LastStateFrame64,
        amplitude_v: f64,
        steps: u32,
        half: u32,
    ) -> Drive {
        let dt = node.gate_dt_s().unwrap();
        let scale = node.melt_enthalpy_j();
        let energy_0 = state.internal_energy_j;
        let mut audit_j = 0.0;
        let mut max_residual = 0.0;
        let mut saw_surge = false;
        let mut n = 0_u32;
        while n < steps {
            let upstream = square_voltage(n, amplitude_v, half);
            let voltage_before = state.voltage_v;
            let saved_state = *state;
            let saved_frame = frame.to_bytes();
            let load = GridLoad {
                upstream_voltage_v: upstream,
                load_current_a: 0.0,
                ambient_k: REFERENCE_TEMPERATURE_K,
            };
            match step(node, state, frame, load, dt) {
                Ok(()) => {}
                Err(reason) => {
                    assert_eq!(*state, saved_state);
                    assert_eq!(frame.to_bytes(), saved_frame);
                    return Drive::Rejected {
                        reason,
                        step: n + 1,
                        temperature_k: state.temperature_k,
                    };
                }
            }
            proof_intact(frame);
            assert!(
                state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT,
                "step {} residual {}",
                n + 1,
                state.residual_conservation_error
            );
            assert!(
                state.entropy_production_w_per_k >= 0.0,
                "step {} entropy production {}",
                n + 1,
                state.entropy_production_w_per_k
            );
            assert!(state.temperature_k < node.melt_k);
            assert!(state.melt_margin_k > 0.0);
            assert_eq!(frame.residual, state.residual_conservation_error as f32);
            assert_eq!(frame.flags & !FLAG_GRID_SURGE, 0);
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            if frame.flags & FLAG_GRID_SURGE != 0 {
                saw_surge = true;
            }
            audit_j += upstream * node.capacitance_f * (state.voltage_v - voltage_before);
            n += 1;
        }
        let audit_gap = (state.internal_energy_j - energy_0 - audit_j).abs();
        let work_gap = (state.boundary_work_j - audit_j).abs();
        Drive::Survived(Report {
            max_residual,
            temperature_k: state.temperature_k,
            peak_voltage_v: state.voltage_spike_v,
            thermal_energy_j: state.thermal_energy_j,
            saw_surge,
            audit_residual: audit_gap / scale,
            work_residual: work_gap / scale,
        })
    }

    fn expect_survived(node: &GridNode, amplitude_v: f64, steps: u32) -> Report {
        let mut state = GridNodeState::try_rest(node).unwrap();
        let mut frame = blank_frame();
        let half = half_steps(node);
        match drive(node, &mut state, &mut frame, amplitude_v, steps, half) {
            Drive::Survived(report) => report,
            Drive::Rejected {
                reason,
                step,
                temperature_k,
            } => {
                ::core::panic!(
                    "rejected {reason:?} on step {step} at {temperature_k} K for {amplitude_v} V"
                );
            }
        }
    }

    fn rlc_step_response(
        t_s: f64,
        voltage_v: f64,
        resistance_ohm: f64,
        inductance_h: f64,
        capacitance_f: f64,
    ) -> (f64, f64) {
        let sigma = resistance_ohm / (2.0 * inductance_h);
        let omega0 = 1.0 / (inductance_h * capacitance_f).sqrt();
        let omega_d = (omega0 * omega0 - sigma * sigma).sqrt();
        let decay = (-sigma * t_s).exp();
        let current_a = (voltage_v / (inductance_h * omega_d)) * decay * (omega_d * t_s).sin();
        let node_v = voltage_v
            * (1.0 - decay * ((omega_d * t_s).cos() + (sigma / omega_d) * (omega_d * t_s).sin()));
        (current_a, node_v)
    }

    #[test]
    fn types_are_copy() {
        assert_copy::<GridNode>();
        assert_copy::<GridNodeState>();
        assert_copy::<GridLoad>();
        assert_copy::<StepReject>();
    }

    #[test]
    fn filament_card_is_an_underdamped_microsecond_node() {
        let node = GridNode::copper_fuse_link();
        let area = circle_area_test(FUSE_DIAMETER_M);
        let resistance = COPPER_RESISTIVITY_OHM_M * FUSE_LENGTH_M / area;
        let capacity =
            COPPER_DENSITY_KG_M3 * FUSE_LENGTH_M * area * COPPER_SPECIFIC_HEAT_J_PER_KG_K;
        assert!((node.resistance_ref_ohm - resistance).abs() <= 1e-15 * resistance);
        assert!((node.thermal_capacity_j_per_k - capacity).abs() <= 1e-15 * capacity);
        assert!((resistance - 0.10276517409864093).abs() <= 1e-12);
        assert_eq!(node.inductance_h, FEEDER_INDUCTANCE_H);
        assert_eq!(node.capacitance_f, NODE_CAPACITANCE_F);
        assert_eq!(node.temp_coeff_per_k, COPPER_TEMP_COEFF_PER_K);
        assert_eq!(node.melt_k, COPPER_MELT_K);
        assert_eq!(node.surge_withstand_v, SURGE_WITHSTAND_V);
        let dt = node.gate_dt_s().unwrap();
        assert!((dt - 1.0e-6).abs() <= 1.0e-15);
        assert!((node.natural_frequency_rad_s() - 20_000.0).abs() <= 1.0e-6);
        assert!((GATE_PHASE_STEP_RAD / node.natural_frequency_rad_s() - dt).abs() <= 1.0e-18);
        let zeta = 0.5 * node.resistance_ref_ohm * (node.capacitance_f / node.inductance_h).sqrt();
        assert!(zeta > 0.02 && zeta < 0.1, "zeta {zeta}");
        let tau_s = node.inductance_h / node.resistance_ref_ohm;
        assert!(tau_s > 200.0e-6 && tau_s < 1.0e-3, "tau {tau_s}");
        let surface = core::f64::consts::PI * FUSE_DIAMETER_M * FUSE_LENGTH_M;
        let conductance = FORCED_CONVECTION_W_PER_M2_K * surface;
        assert!((node.thermal_conductance_w_per_k - conductance).abs() <= 1e-15 * conductance);
        assert!(node.resistance_ohm(CARD_TEMPERATURE_MIN_K) > 0.0);
        assert!(
            GridNode::try_new(
                0.0,
                node.capacitance_f,
                node.resistance_ref_ohm,
                node.thermal_capacity_j_per_k,
                node.thermal_conductance_w_per_k,
            )
            .is_none()
        );
        let mut forged = node;
        forged.melt_k = 300.0;
        assert!(
            !forged.is_valid(),
            "a caller-supplied melt point must not validate"
        );
    }

    fn circle_area_test(diameter_m: f64) -> f64 {
        0.25 * core::f64::consts::PI * diameter_m * diameter_m
    }

    #[test]
    fn resistance_rises_linearly_and_entropy_production_is_non_negative() {
        let node = GridNode::copper_fuse_link();
        let hot = REFERENCE_TEMPERATURE_K + 80.0;
        let expected = node.resistance_ref_ohm * (1.0 + COPPER_TEMP_COEFF_PER_K * 80.0);
        assert!((node.resistance_ohm(hot) - expected).abs() <= 1e-15 * expected);
        assert!(node.resistance_ohm(hot) > node.resistance_ohm(REFERENCE_TEMPERATURE_K));
        let sigma = node.entropy_production_w_per_k(10.0, 300.0, REFERENCE_TEMPERATURE_K);
        let joule = 100.0 * node.resistance_ohm(300.0);
        let delta = 300.0 - REFERENCE_TEMPERATURE_K;
        let hand = joule / 300.0
            + node.thermal_conductance_w_per_k * delta * delta / (300.0 * REFERENCE_TEMPERATURE_K);
        assert!((sigma - hand).abs() <= 1e-12 * hand);
        assert!(sigma > 0.0);
        let state = GridNodeState::try_phase(&node, 10.0, 0.0, 300.0).unwrap();
        assert_eq!(state.entropy_production_w_per_k, sigma);
        assert!(GridNodeState::try_phase(&node, 0.0, 0.0, COPPER_MELT_K).is_none());
        assert!(GridNodeState::try_phase(&node, 0.0, 0.0, CARD_TEMPERATURE_MIN_K - 1.0).is_none());
    }

    #[test]
    fn joule_heat_cancels_inside_the_energy_differential() {
        let node = GridNode::copper_fuse_link();
        let phase = [12.0, -30.0, 360.0, 1.5];
        let load = GridLoad {
            upstream_voltage_v: CATALOG_VOLTAGE_V,
            load_current_a: 4.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let rate = derivatives(&node, phase, load).unwrap();
        let d_energy = node.inductance_h * phase[0] * rate[0]
            + node.capacitance_f * phase[1] * rate[1]
            + node.thermal_capacity_j_per_k * rate[2];
        let scale = 1.0 + d_energy.abs() + rate[3].abs();
        assert!(
            (d_energy - rate[3]).abs() <= 1e-9 * scale,
            "dU {d_energy} dW {}",
            rate[3]
        );
        assert!(phase[0] * phase[0] * node.resistance_ohm(phase[2]) > 1.0);
    }

    #[test]
    fn an_accepted_step_publishes_the_frame_and_leaves_the_proof() {
        let node = GridNode::copper_fuse_link();
        let dt = node.gate_dt_s().unwrap();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        let load = GridLoad {
            upstream_voltage_v: CATALOG_VOLTAGE_V,
            load_current_a: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        step(&node, &mut state, &mut frame, load, dt).unwrap();
        proof_intact(&frame);
        assert_eq!(frame.t, dt);
        assert!(frame.pos[0] > 0.0);
        assert!(frame.pos[1] > 0.0);
        assert!((frame.pos[2] as f64 - state.temperature_k).abs() < 1.0e-3);
        assert!(state.temperature_k > REFERENCE_TEMPERATURE_K);
        assert!(frame.vel[1] > 0.0);
        assert!(frame.vel[2] > 0.0);
        assert!(frame.force_torque > 0.0);
        assert_eq!(frame.flags, 0);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(state.temperature_k > REFERENCE_TEMPERATURE_K);
        assert!(state.temperature_k < REFERENCE_TEMPERATURE_K + 1.0);
        assert_eq!(state.t_s, dt);
    }

    #[test]
    fn catalog_step_matches_the_closed_form_rlc_solution() {
        let mut node = adiabatic(&GridNode::copper_fuse_link());
        node.temp_coeff_per_k = 0.0;
        assert!(node.is_valid());
        let dt = node.gate_dt_s().unwrap();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        let load = GridLoad {
            upstream_voltage_v: CATALOG_VOLTAGE_V,
            load_current_a: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let mut n = 0_u32;
        while n < 400 {
            step(&node, &mut state, &mut frame, load, dt).unwrap();
            n += 1;
        }
        let (current_exact, voltage_exact) = rlc_step_response(
            state.t_s,
            CATALOG_VOLTAGE_V,
            node.resistance_ref_ohm,
            node.inductance_h,
            node.capacitance_f,
        );
        assert!(
            (state.current_a - current_exact).abs() <= 1.0e-5,
            "current {} exact {current_exact}",
            state.current_a
        );
        assert!(
            (state.voltage_v - voltage_exact).abs() <= 1.0e-5,
            "voltage {} exact {voltage_exact}",
            state.voltage_v
        );
        let booked = CATALOG_VOLTAGE_V * node.capacitance_f * state.voltage_v;
        let gap = (state.internal_energy_j - booked).abs();
        assert!(
            gap <= RESIDUAL_CONSERVATION_LIMIT * node.melt_enthalpy_j(),
            "identity gap {gap}"
        );
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(state.thermal_energy_j > 0.0);
        proof_intact(&frame);
    }

    #[test]
    fn a_single_catalog_step_stays_cool_over_5000_microseconds() {
        let node = GridNode::copper_fuse_link();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        let load = GridLoad {
            upstream_voltage_v: CATALOG_VOLTAGE_V,
            load_current_a: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let dt = node.gate_dt_s().unwrap();
        let mut max_residual = 0.0;
        let mut n = 0_u32;
        while n < SPIKE_STEPS {
            step(&node, &mut state, &mut frame, load, dt).unwrap();
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            assert!(state.entropy_production_w_per_k >= 0.0);
            proof_intact(&frame);
            n += 1;
        }
        assert!(
            max_residual <= RESIDUAL_CONSERVATION_LIMIT,
            "residual {max_residual}"
        );
        assert!(state.temperature_k > REFERENCE_TEMPERATURE_K + 1.0);
        assert!(state.temperature_k < REFERENCE_TEMPERATURE_K + 30.0);
        assert!(state.voltage_spike_v < SURGE_WITHSTAND_V);
        assert!(!state.surge);
        assert_eq!(frame.flags, 0);
        assert!(state.melt_margin_k > COPPER_MELT_K - REFERENCE_TEMPERATURE_K - 30.0);
    }

    #[test]
    fn adiabatic_extreme_spike_matches_the_endpoint_identity_over_5000_steps() {
        let node = adiabatic(&GridNode::copper_fuse_link());
        let report = expect_survived(&node, EXTREME_SURVIVING_STEP_V, SPIKE_STEPS);
        assert!(
            report.max_residual <= RESIDUAL_CONSERVATION_LIMIT,
            "residual {}",
            report.max_residual
        );
        assert!(
            report.audit_residual <= RESIDUAL_CONSERVATION_LIMIT,
            "audit {}",
            report.audit_residual
        );
        assert!(
            report.work_residual <= RESIDUAL_CONSERVATION_LIMIT,
            "work {}",
            report.work_residual
        );
        assert!(
            report.temperature_k > 1_200.0,
            "temperature {}",
            report.temperature_k
        );
        assert!(report.temperature_k < COPPER_MELT_K);
        assert!(report.thermal_energy_j > 0.8 * node.melt_enthalpy_j());
        assert!(report.peak_voltage_v > SURGE_WITHSTAND_V);
        assert!(report.saw_surge);
    }

    #[test]
    fn card_extreme_spike_stays_inside_the_gate_over_5000_steps() {
        let node = GridNode::copper_fuse_link();
        let report = expect_survived(&node, EXTREME_SURVIVING_STEP_V, SPIKE_STEPS);
        assert!(
            report.max_residual <= RESIDUAL_CONSERVATION_LIMIT,
            "residual {}",
            report.max_residual
        );
        assert!(
            report.temperature_k > 1_200.0,
            "temperature {}",
            report.temperature_k
        );
        assert!(report.temperature_k < COPPER_MELT_K);
        assert!(report.thermal_energy_j > 0.8 * node.melt_enthalpy_j());
        assert!(report.peak_voltage_v >= SURGE_WITHSTAND_V);
        assert!(report.saw_surge);
        assert!(report.peak_voltage_v > EXTREME_SURVIVING_STEP_V);
    }

    #[test]
    fn a_mild_spike_train_does_not_flag_a_surge() {
        let node = GridNode::copper_fuse_link();
        let report = expect_survived(&node, 5.0, 1_000);
        assert!(report.max_residual <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(report.peak_voltage_v > 20.0);
        assert!(report.peak_voltage_v < SURGE_WITHSTAND_V);
        assert!(!report.saw_surge);
        assert!(report.temperature_k > REFERENCE_TEMPERATURE_K);
        assert!(report.temperature_k < 450.0);
    }

    #[test]
    fn explicit_euler_creates_energy_on_the_first_microsecond() {
        let node = GridNode::copper_fuse_link();
        let dt = node.gate_dt_s().unwrap();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let saved_frame = frame.to_bytes();
        let load = GridLoad {
            upstream_voltage_v: CATALOG_VOLTAGE_V,
            load_current_a: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let got = step_euler(&node, &mut state, &mut frame, load, dt);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        assert_eq!(frame.to_bytes(), saved_frame);
        proof_intact(&frame);
        step(&node, &mut state, &mut frame, load, dt).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(state.t_s == dt);
    }

    #[test]
    fn left_rectangle_joule_update_crosses_the_gate() {
        let node = adiabatic(&GridNode::copper_fuse_link());
        let dt = node.gate_dt_s().unwrap();
        let scale = node.melt_enthalpy_j();
        let mut current_a = 0.0;
        let mut voltage_v = 0.0;
        let mut temperature_k = REFERENCE_TEMPERATURE_K;
        let mut work_j = 0.0;
        let mut crossed = 0_u32;
        let mut n = 0_u32;
        while n < 8 {
            let upstream = CATALOG_VOLTAGE_V;
            let resistance = node.resistance_ohm(temperature_k);
            let electrical = rk4_frozen_rl(&node, current_a, voltage_v, resistance, upstream, dt);
            let joule_w = current_a * current_a * resistance;
            let next_t = temperature_k + dt * joule_w / node.thermal_capacity_j_per_k;
            let next_w = work_j + dt * upstream * current_a;
            let energy = node.internal_energy_j(electrical.0, electrical.1, next_t);
            let residual = (energy - next_w).abs() / scale;
            current_a = electrical.0;
            voltage_v = electrical.1;
            temperature_k = next_t;
            work_j = next_w;
            n += 1;
            if residual > RESIDUAL_CONSERVATION_LIMIT && crossed == 0 {
                crossed = n;
            }
        }
        assert!(
            crossed > 0 && crossed <= 4,
            "lagged residual crossed on step {crossed}"
        );
        let faithful = expect_survived(&node, CATALOG_VOLTAGE_V, 8);
        // Constant hold: half-period longer than the run, so the sign never flips.
        assert!(faithful.max_residual <= RESIDUAL_CONSERVATION_LIMIT);
    }

    fn rk4_frozen_rl(
        node: &GridNode,
        current_a: f64,
        voltage_v: f64,
        resistance_ohm: f64,
        upstream_v: f64,
        dt: f64,
    ) -> (f64, f64) {
        let rate = |current: f64, voltage: f64| -> (f64, f64) {
            (
                (upstream_v - voltage - current * resistance_ohm) / node.inductance_h,
                current / node.capacitance_f,
            )
        };
        let k1 = rate(current_a, voltage_v);
        let k2 = rate(current_a + 0.5 * dt * k1.0, voltage_v + 0.5 * dt * k1.1);
        let k3 = rate(current_a + 0.5 * dt * k2.0, voltage_v + 0.5 * dt * k2.1);
        let k4 = rate(current_a + dt * k3.0, voltage_v + dt * k3.1);
        (
            current_a + (dt / 6.0) * (k1.0 + 2.0 * k2.0 + 2.0 * k3.0 + k4.0),
            voltage_v + (dt / 6.0) * (k1.1 + 2.0 * k2.1 + 2.0 * k3.1 + k4.1),
        )
    }

    #[test]
    fn dispatched_current_heats_the_link_and_stays_inside_the_gate() {
        let node = GridNode::copper_fuse_link();
        let dt = node.gate_dt_s().unwrap();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        let load = GridLoad {
            upstream_voltage_v: 30.0 * node.resistance_ref_ohm,
            load_current_a: 30.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let mut n = 0_u32;
        while n < 1_000 {
            step(&node, &mut state, &mut frame, load, dt).unwrap();
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            assert!(state.entropy_production_w_per_k >= 0.0);
            proof_intact(&frame);
            n += 1;
        }
        assert!(state.temperature_k > REFERENCE_TEMPERATURE_K + 5.0);
        assert!(state.temperature_k < COPPER_MELT_K);
        assert!(state.current_a > 20.0);
    }

    #[test]
    fn cooling_is_booked_as_boundary_work() {
        let mut node = GridNode::copper_fuse_link();
        node.thermal_conductance_w_per_k = 100.0;
        assert!(node.is_valid());
        let dt = node.gate_dt_s().unwrap();
        let mut state = GridNodeState::try_phase(&node, 0.0, 0.0, 400.0).unwrap();
        let mut frame = blank_frame();
        let mut n = 0_u32;
        while n < 100 {
            step(&node, &mut state, &mut frame, GridLoad::default(), dt).unwrap();
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            proof_intact(&frame);
            n += 1;
        }
        assert!(state.temperature_k < 400.0);
        assert!(state.boundary_work_j < 0.0);
        assert!(state.internal_energy_j < state.energy_anchor_j);
        assert!(state.entropy_production_w_per_k > 0.0);
    }

    #[test]
    fn invented_heat_is_spontaneous_generation() {
        let node = GridNode::copper_fuse_link();
        let dt = node.gate_dt_s().unwrap();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let saved_frame = frame.to_bytes();
        let got =
            step_shift_temperature(&node, &mut state, &mut frame, GridLoad::default(), dt, 20.0);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        assert_eq!(frame.to_bytes(), saved_frame);
    }

    #[test]
    fn deleted_heat_is_a_first_law_breach() {
        let node = adiabatic(&GridNode::copper_fuse_link());
        let dt = node.gate_dt_s().unwrap();
        let mut state = GridNodeState::try_phase(&node, 0.0, 0.0, 400.0).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let saved_frame = frame.to_bytes();
        let got = step_shift_temperature(
            &node,
            &mut state,
            &mut frame,
            GridLoad::default(),
            dt,
            -50.0,
        );
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state, saved);
        assert_eq!(frame.to_bytes(), saved_frame);
    }

    #[test]
    fn melt_limit_is_a_strict_failure_and_the_last_solid_state_stands() {
        let node = GridNode::copper_fuse_link();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        let half = half_steps(&node);
        match drive(
            &node,
            &mut state,
            &mut frame,
            CATALOG_VOLTAGE_V,
            SPIKE_STEPS,
            half,
        ) {
            Drive::Rejected {
                reason,
                step,
                temperature_k,
            } => {
                assert_eq!(reason, StepReject::CopperMelt);
                assert!(step > 500 && step <= SPIKE_STEPS, "melt step {step}");
                assert!(
                    temperature_k > COPPER_MELT_K - 1.0,
                    "last solid {temperature_k}"
                );
                assert!(temperature_k < COPPER_MELT_K);
                assert_eq!(state.temperature_k, temperature_k);
                proof_intact(&frame);
                assert!(frame.t > 0.0);
                assert_eq!(frame.flags & !FLAG_GRID_SURGE, 0);
            }
            Drive::Survived(report) => {
                ::core::panic!(
                    "catalog square wave stayed solid at {} K",
                    report.temperature_k
                );
            }
        }
    }

    #[test]
    fn an_already_molten_state_is_refused() {
        let node = GridNode::copper_fuse_link();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        state.temperature_k = COPPER_MELT_K;
        let saved = state;
        let mut frame = blank_frame();
        let saved_frame = frame.to_bytes();
        let got = step(
            &node,
            &mut state,
            &mut frame,
            GridLoad::default(),
            node.gate_dt_s().unwrap(),
        );
        assert_eq!(got, Err(StepReject::CopperMelt));
        assert_eq!(state, saved);
        assert_eq!(frame.to_bytes(), saved_frame);
    }

    #[test]
    fn recursive_adversary_bisects_the_melt_boundary() {
        let node = GridNode::copper_fuse_link();
        let cool = expect_survived(&node, EXTREME_SURVIVING_STEP_V, SPIKE_STEPS);
        assert!(cool.temperature_k < COPPER_MELT_K);
        assert!(cool.max_residual <= RESIDUAL_CONSERVATION_LIMIT);
        let mut hot_state = GridNodeState::try_rest(&node).unwrap();
        let mut hot_frame = blank_frame();
        let half = half_steps(&node);
        match drive(
            &node,
            &mut hot_state,
            &mut hot_frame,
            CATALOG_VOLTAGE_V,
            SPIKE_STEPS,
            half,
        ) {
            Drive::Rejected {
                reason: StepReject::CopperMelt,
                ..
            } => {}
            Drive::Rejected {
                reason,
                step,
                temperature_k,
            } => {
                ::core::panic!("high side {reason:?} step {step} at {temperature_k} K");
            }
            Drive::Survived(report) => {
                ::core::panic!("high side stayed solid at {} K", report.temperature_k);
            }
        }
        let (lo, hi) = bisect(&node, EXTREME_SURVIVING_STEP_V, CATALOG_VOLTAGE_V, 6);
        assert!(hi - lo < 1.0, "bracket {lo}..{hi}");
        assert!(lo >= EXTREME_SURVIVING_STEP_V);
        assert!(hi <= CATALOG_VOLTAGE_V);
        let survived = expect_survived(&node, lo, SPIKE_STEPS);
        assert!(survived.max_residual <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(survived.temperature_k < COPPER_MELT_K);
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        match drive(&node, &mut state, &mut frame, hi, SPIKE_STEPS, half) {
            Drive::Rejected {
                reason: StepReject::CopperMelt,
                temperature_k,
                ..
            } => {
                assert!(temperature_k < COPPER_MELT_K);
                assert!(temperature_k > COPPER_MELT_K - 1.0);
            }
            Drive::Rejected { reason, step, .. } => {
                ::core::panic!("bracket high side {reason:?} on step {step}");
            }
            Drive::Survived(report) => {
                ::core::panic!("bracket high side stayed at {} K", report.temperature_k);
            }
        }
    }

    fn bisect(node: &GridNode, lo: f64, hi: f64, depth: u32) -> (f64, f64) {
        if depth == 0 {
            return (lo, hi);
        }
        let mid = 0.5 * (lo + hi);
        let mut state = GridNodeState::try_rest(node).unwrap();
        let mut frame = blank_frame();
        let half = half_steps(node);
        match drive(node, &mut state, &mut frame, mid, SPIKE_STEPS, half) {
            Drive::Survived(report) => {
                assert!(
                    report.max_residual <= RESIDUAL_CONSERVATION_LIMIT,
                    "mid {mid} residual {}",
                    report.max_residual
                );
                assert!(report.temperature_k < node.melt_k);
                bisect(node, mid, hi, depth - 1)
            }
            Drive::Rejected {
                reason: StepReject::CopperMelt,
                ..
            } => bisect(node, lo, mid, depth - 1),
            Drive::Rejected {
                reason,
                step,
                temperature_k,
            } => {
                ::core::panic!(
                    "adversary at {mid} V failed on step {step} with {reason:?} at {temperature_k} K"
                );
            }
        }
    }

    #[test]
    fn bad_inputs_are_refused_and_the_frame_stays() {
        let node = GridNode::copper_fuse_link();
        let dt = node.gate_dt_s().unwrap();
        let mut state = GridNodeState::try_rest(&node).unwrap();
        let mut frame = blank_frame();
        let saved = state;
        let saved_frame = frame.to_bytes();

        let mut refuse = |load: GridLoad, dt_s: f64, expect: StepReject| {
            let got = step(&node, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state, saved);
            assert_eq!(frame.to_bytes(), saved_frame);
        };
        refuse(GridLoad::default(), 0.0, StepReject::NonPositiveDt);
        refuse(GridLoad::default(), -1.0e-6, StepReject::NonPositiveDt);
        refuse(GridLoad::default(), f64::NAN, StepReject::NonFiniteInput);
        refuse(
            GridLoad {
                upstream_voltage_v: f64::INFINITY,
                load_current_a: 0.0,
                ambient_k: REFERENCE_TEMPERATURE_K,
            },
            dt,
            StepReject::NonFiniteInput,
        );
        refuse(
            GridLoad {
                upstream_voltage_v: 0.0,
                load_current_a: 0.0,
                ambient_k: 0.0,
            },
            dt,
            StepReject::NegativeTemperature,
        );
        refuse(
            GridLoad {
                upstream_voltage_v: 0.0,
                load_current_a: 0.0,
                ambient_k: 200.0,
            },
            dt,
            StepReject::TemperatureOutsideCard,
        );
        refuse(
            GridLoad {
                upstream_voltage_v: 0.0,
                load_current_a: 0.0,
                ambient_k: COPPER_MELT_K,
            },
            dt,
            StepReject::TemperatureOutsideCard,
        );

        state.current_a = f64::NAN;
        let got = step(&node, &mut state, &mut frame, GridLoad::default(), dt);
        assert_eq!(got, Err(StepReject::NonFiniteState));
        state = saved;

        let mut broken = node;
        broken.inductance_h = -1.0;
        let got = step(&broken, &mut state, &mut frame, GridLoad::default(), dt);
        assert_eq!(got, Err(StepReject::InvalidNode));
        assert_eq!(state, saved);
        assert_eq!(frame.to_bytes(), saved_frame);
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::GridNode::copper_fuse_link();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The fuse carrying 2 A at 48 V, with 1 A drawn from the node.
        let mut state = super::GridNodeState::try_phase(&card, 2.0, 48.0, super::REFERENCE_TEMPERATURE_K).unwrap();
        let load = super::GridLoad {
            upstream_voltage_v: super::CATALOG_VOLTAGE_V,
            load_current_a: 1.0,
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
        assert!(state.current_a != before.current_a);
        assert!(state.voltage_v != before.voltage_v);
        assert!(state.temperature_k != before.temperature_k);
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

    /// Hold `load` from `start` at the gate step for up to `steps` steps.
    /// Returns the landed count and the refusal, if any, and checks that
    /// every landed state is inside both electrical bounds.
    fn hold(
        node: &GridNode,
        start: GridNodeState,
        load: GridLoad,
        steps: u32,
    ) -> (u32, Option<StepReject>, GridNodeState) {
        let dt = node.gate_dt_s().unwrap();
        let mut state = start;
        let mut frame = blank_frame();
        let mut landed = 0_u32;
        while landed < steps {
            let saved = state;
            let saved_frame = frame.to_bytes();
            match step(node, &mut state, &mut frame, load, dt) {
                Ok(()) => {
                    assert!(state.current_a.abs() <= node.melt_current_a());
                    assert!(state.voltage_v.abs() <= node.melt_voltage_v());
                    landed += 1;
                }
                Err(reason) => {
                    assert_eq!(state, saved);
                    assert_eq!(frame.to_bytes(), saved_frame);
                    return (landed, Some(reason), state);
                }
            }
        }
        (landed, None, state)
    }

    #[test]
    fn the_electrical_bounds_are_the_melt_energy_in_each_store() {
        let node = GridNode::copper_fuse_link();
        let i = node.melt_current_a();
        let v = node.melt_voltage_v();
        assert_eq!(i.to_bits(), (2.0 * node.melt_enthalpy_j() / node.inductance_h).sqrt().to_bits());
        assert_eq!(v.to_bits(), (2.0 * node.melt_enthalpy_j() / node.capacitance_f).sqrt().to_bits());
        assert_eq!(i.to_bits(), 595.7150486594088_f64.to_bits());
        assert_eq!(v.to_bits(), 595.7150486594088_f64.to_bits());
    }

    #[test]
    fn a_held_voltage_at_the_bound_rings_past_it_and_that_step_is_refused() {
        let node = GridNode::copper_fuse_link();
        let rest = GridNodeState::try_rest(&node).unwrap();
        let load = GridLoad {
            upstream_voltage_v: node.melt_voltage_v(),
            load_current_a: 0.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let (landed, reject, last) = hold(&node, rest, load, 2_000);
        assert_eq!(reject, Some(StepReject::VoltageOutsideCard));
        assert!(landed > 0 && landed < 157, "landed {landed}");
        assert!(last.voltage_v <= node.melt_voltage_v());
    }

    #[test]
    fn a_held_current_rings_the_node_past_a_bound_and_that_step_is_refused() {
        let node = GridNode::copper_fuse_link();
        let ib = node.melt_current_a();
        let run = |fraction: f64| {
            let load = GridLoad {
                upstream_voltage_v: 0.0,
                load_current_a: fraction * ib,
                ambient_k: REFERENCE_TEMPERATURE_K,
            };
            hold(&node, GridNodeState::try_rest(&node).unwrap(), load, 5_000)
        };
        // At the bound the shunt voltage swings past its bound first.
        let (landed, reject, _) = run(1.0);
        assert_eq!(reject, Some(StepReject::VoltageOutsideCard));
        assert!(landed > 0 && landed < 100, "landed {landed}");
        // Above about 0.58 of the bound the feeder current rings past its bound.
        assert_eq!(run(0.59).1, Some(StepReject::CurrentOutsideCard));
        assert_eq!(run(0.95).1, Some(StepReject::CurrentOutsideCard));
        // Under that the link melts first.
        assert_eq!(run(0.58).1, Some(StepReject::CopperMelt));
        // A driven feeder from a hot start crosses the current bound in a few steps.
        let start = GridNodeState::try_phase(&node, 0.9 * ib, 0.0, REFERENCE_TEMPERATURE_K).unwrap();
        let load = GridLoad {
            upstream_voltage_v: 0.5 * node.melt_voltage_v(),
            load_current_a: 0.9 * ib,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let (landed, reject, _) = hold(&node, start, load, 5_000);
        assert_eq!(reject, Some(StepReject::CurrentOutsideCard));
        assert_eq!(landed, 12);
    }

    #[test]
    fn a_single_step_past_the_voltage_bound_is_refused() {
        let node = GridNode::copper_fuse_link();
        let b = node.melt_voltage_v();
        let start =
            GridNodeState::try_phase(&node, node.melt_current_a(), b, CARD_TEMPERATURE_MIN_K)
                .unwrap();
        let load = GridLoad {
            upstream_voltage_v: -b,
            load_current_a: -node.melt_current_a(),
            ambient_k: CARD_TEMPERATURE_MIN_K,
        };
        let (landed, reject, _) = hold(&node, start, load, 1);
        assert_eq!(landed, 0);
        assert_eq!(reject, Some(StepReject::VoltageOutsideCard));
    }

    #[test]
    fn the_bounds_are_checked_on_the_exact_landed_values() {
        let node = GridNode::copper_fuse_link();
        let dt = node.gate_dt_s().unwrap();
        let i = node.melt_current_a();
        let v = node.melt_voltage_v();
        let past_i = f64::from_bits(i.to_bits() + 1);
        let past_v = f64::from_bits(v.to_bits() + 1);
        let t = REFERENCE_TEMPERATURE_K;
        let mut frame = blank_frame();
        let cases = [
            ([past_i, 0.0, t, 0.0], Some(StepReject::CurrentOutsideCard)),
            ([-past_i, 0.0, t, 0.0], Some(StepReject::CurrentOutsideCard)),
            ([0.0, past_v, t, 0.0], Some(StepReject::VoltageOutsideCard)),
            ([0.0, -past_v, t, 0.0], Some(StepReject::VoltageOutsideCard)),
            ([i, 0.0, t, 0.0], None),
            ([-i, 0.0, t, 0.0], None),
            ([0.0, v, t, 0.0], None),
            ([0.0, -v, t, 0.0], None),
        ];
        for (predicted, want) in cases {
            let mut state = GridNodeState::try_rest(&node).unwrap();
            let got = super::commit(&node, &mut state, &mut frame, GridLoad::default(), dt, predicted);
            match want {
                Some(reason) => assert_eq!(got, Err(reason), "{predicted:?}"),
                // On the bound the step is not refused for the bound; the
                // ledger, which these hand-made landings do not balance,
                // refuses it instead.
                None => {
                    assert!(got != Err(StepReject::CurrentOutsideCard), "{predicted:?}");
                    assert!(got != Err(StepReject::VoltageOutsideCard), "{predicted:?}");
                }
            }
        }
    }

    #[test]
    fn from_rest_a_held_voltage_rings_past_the_bound_above_0_55_of_it() {
        let node = GridNode::copper_fuse_link();
        let b = node.melt_voltage_v();
        let run = |fraction: f64| {
            let load = GridLoad {
                upstream_voltage_v: fraction * b,
                load_current_a: 0.0,
                ambient_k: REFERENCE_TEMPERATURE_K,
            };
            hold(&node, GridNodeState::try_rest(&node).unwrap(), load, 2_000).1
        };
        assert_eq!(run(0.548), None);
        assert_eq!(run(0.549), Some(StepReject::VoltageOutsideCard));
    }

    #[test]
    fn a_restart_keeps_the_state_and_starts_the_peak_and_ledger_again() {
        let node = GridNode::copper_fuse_link();
        let charge = GridLoad {
            upstream_voltage_v: CATALOG_VOLTAGE_V,
            load_current_a: 1.0,
            ambient_k: REFERENCE_TEMPERATURE_K,
        };
        let coast = GridLoad {
            upstream_voltage_v: 0.0,
            load_current_a: 0.0,
            ambient_k: 253.15,
        };
        let (_, none, mid) = hold(&node, GridNodeState::try_rest(&node).unwrap(), charge, 200);
        assert_eq!(none, None);
        let (_, none, straight) = hold(&node, mid, coast, 200);
        assert_eq!(none, None);
        let restart =
            GridNodeState::try_phase(&node, mid.current_a, mid.voltage_v, mid.temperature_k)
                .unwrap();
        assert_eq!(restart.voltage_spike_v, mid.voltage_v.abs());
        assert_eq!(restart.boundary_work_j, 0.0);
        let (_, none, again) = hold(&node, restart, coast, 200);
        assert_eq!(none, None);
        assert_eq!(again.current_a.to_bits(), straight.current_a.to_bits());
        assert_eq!(again.voltage_v.to_bits(), straight.voltage_v.to_bits());
        assert_eq!(again.temperature_k.to_bits(), straight.temperature_k.to_bits());
        assert!(again.voltage_spike_v < straight.voltage_spike_v);
    }
}
