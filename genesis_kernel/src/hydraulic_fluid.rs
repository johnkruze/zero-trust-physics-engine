#![no_implicit_prelude]

//! Mineral-oil hydraulic column.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, pressure, reference-density fluid volume, column
//! compression, contained mass, admitted mass, elastic energy, the margin to
//! vapor pressure, and the conservation residual into [`LastStateFrame64`].
//! The 16-byte frame proof is not touched.
//!
//! The campaign map is `δL = L ΔP / β` with `β = 1.72 GPa` and `L = 0.50 m`.
//! This card integrates that law in a 50 mm bore. Density is the 870 kg/m³
//! oil used with that bulk modulus. The charge is one atmosphere. Vapor
//! pressure is the cavitation floor, 10 Pa absolute.
//!
//! ```text
//! dP/dt = (β / V) Q
//! dm/dt = ρ Q
//! dW/dt = (P − P_ref) Q
//! ρ_eos = ρ (1 + (P − P_ref) / β)
//! E = V (P − P_ref)² / (2 β)
//! residual = max(|m − m_anchor − m_boundary| / m_card,
//!                |E − E_anchor − W| / E_column)
//! ```
//!
//! `Q > 0` admits oil. `m_card` is the atmospheric fill. `E_column` is the
//! elastic energy at the 3 mm campaign overshoot. Under a held flow the
//! right-hand side is a polynomial of degree 1, so classical RK4 lands on
//! the integral. Explicit Euler keeps the rectangle `(P − P_ref) Q Δt` and
//! creates the missing triangle.
//!
//! [`FluidColumn::gate_dt_s`] is `ω Δt = 0.02` on the half-wave `ω = π c / L`,
//! `c = √(β / ρ)`. The public step does not apply that clock. The stroke
//! limit is applied: `|Q| Δt ≤ A · 3 mm`. Past it the step is
//! [`StepReject::FlowAboveNyquist`]. The flow itself is held to
//! [`FluidColumn::flow_acoustic_m3_s`]: a bore speed `|Q| / A` of at most
//! `(P_ceiling − P_ref) / (ρ c)`, 0.012 of the wave speed. Past it the step
//! is [`StepReject::FlowAboveAcoustic`]. A landing below vapor pressure is
//! [`StepReject::Cavitation`]. Compression at or above 1.8 mm sets
//! [`FLAG_COLUMN_SOFT`]. At or above 3.0 mm the frame also sets
//! [`FLAG_COLUMN_LIMIT`]. Those flags are marks, not limits, and do not reject
//! the step. A pressure above [`PRESSURE_CEILING_PA`], on the incoming state or
//! on the landing, is [`StepReject::PressureAboveCard`]: the card ends at
//! 6.0 mm of compression.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Nominal mineral-oil bulk modulus. Campaign constant.
pub const BULK_MODULUS_PA: f64 = 1.72e9;
/// Oil density paired with that bulk modulus.
pub const DENSITY_KG_M3: f64 = 870.0;
/// Campaign column length.
pub const COLUMN_LENGTH_M: f64 = 0.50;
/// Locked bore. Volume is `π d² L / 4`.
pub const BORE_DIAMETER_M: f64 = 0.050;
/// Atmospheric charge.
pub const REFERENCE_PRESSURE_PA: f64 = 101_325.0;
/// Mineral-oil cavitation floor, absolute.
pub const VAPOR_PRESSURE_PA: f64 = 10.0;
/// Campaign soft overshoot.
pub const COMPRESSION_SOFT_M: f64 = 0.0018;
/// Campaign hard overshoot.
pub const COMPRESSION_LIMIT_M: f64 = 0.0030;
/// Top of the card, absolute: 20.64 MPa above the charge, where the column is
/// 6.0 mm shorter. That is twice the campaign hard overshoot and 1.2 percent
/// of the column length. The card's law `δL = L ΔP / β` is the
/// small-compression law with one bulk modulus, and its campaign ends at
/// 3.0 mm; at 6.0 mm the straight-line density law is 7.2e-5 of the density
/// away from the exponential law of the same modulus. The stroke limit lets
/// one step move at most 3.0 mm, so the ceiling is at least one full step past
/// the hard mark. Under it the largest rounding found in the ledger over a million steps is
/// 7.2e-10 of the residual scale, 1,390 times under the limit. Without a
/// ceiling, rounding first refused a step near 0.26 m of compression.
pub const PRESSURE_CEILING_PA: f64 = 20_741_325.0;

/// Normalized mass and energy defect must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Acoustic phase advance `ω Δt` used by [`FluidColumn::gate_dt_s`].
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Column compression is at or above [`COMPRESSION_SOFT_M`] on this frame.
pub const FLAG_COLUMN_SOFT: u64 = 1 << 0;
/// Column compression is at or above [`COMPRESSION_LIMIT_M`] on this frame.
pub const FLAG_COLUMN_LIMIT: u64 = 1 << 1;

/// Why a step did not advance the column or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Flow or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// Bulk modulus, density, bore, or the pressure anchors are not this column.
    InvalidChamber,
    /// `|Q| dt` is past the 3 mm stroke volume.
    FlowAboveNyquist,
    /// `|Q|` is above the acoustic flow limit: the bore speed `|Q| / A` is above `(P_ceiling − P_ref) / (ρ c)`, 0.012 of the oil's wave speed, about 16.87 m/s, so a flow of 0.0331 m³/s. Above it the pressure front that starting the flow sends down the column is larger than the card's whole span, and one pressure for the whole column does not describe it.
    FlowAboveAcoustic,
    /// The trial pressure is below vapor pressure.
    Cavitation,
    /// Pressure, incoming or landing, is above the card's ceiling of 20.64 MPa over the charge, 6.0 mm of compression.
    PressureAboveCard,
    /// The trial step created mass or elastic energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed mass or elastic energy.
    FirstLawBreach,
}

/// Zero-order hold. Positive flow admits oil.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidLoad {
    pub flow_m3_s: f64,
}

impl Default for FluidLoad {
    fn default() -> Self {
        Self { flow_m3_s: 0.0 }
    }
}

/// Locked chamber. Bulk modulus and density are recorded copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidColumn {
    pub bulk_modulus_pa: f64,
    pub density_kg_m3: f64,
    pub length_m: f64,
    pub bore_diameter_m: f64,
    pub reference_pressure_pa: f64,
    pub vapor_pressure_pa: f64,
    /// Recorded copy of [`PRESSURE_CEILING_PA`]. Not a caller input.
    pub pressure_ceiling_pa: f64,
}

impl FluidColumn {
    /// Card: 1.72 GPa mineral oil, 50 mm by 0.50 m, charged to atmosphere.
    #[must_use]
    pub fn mineral_oil() -> Self {
        let column = Self {
            bulk_modulus_pa: BULK_MODULUS_PA,
            density_kg_m3: DENSITY_KG_M3,
            length_m: COLUMN_LENGTH_M,
            bore_diameter_m: BORE_DIAMETER_M,
            reference_pressure_pa: REFERENCE_PRESSURE_PA,
            vapor_pressure_pa: VAPOR_PRESSURE_PA,
            pressure_ceiling_pa: PRESSURE_CEILING_PA,
        };
        if column.is_valid() {
            column
        } else {
            ::core::panic!("mineral-oil column failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.bulk_modulus_pa.to_bits() == BULK_MODULUS_PA.to_bits()
            && self.density_kg_m3.to_bits() == DENSITY_KG_M3.to_bits()
            && self.length_m.to_bits() == COLUMN_LENGTH_M.to_bits()
            && self.bore_diameter_m.to_bits() == BORE_DIAMETER_M.to_bits()
            && self.reference_pressure_pa.to_bits() == REFERENCE_PRESSURE_PA.to_bits()
            && self.vapor_pressure_pa.to_bits() == VAPOR_PRESSURE_PA.to_bits()
            && self.pressure_ceiling_pa.to_bits() == PRESSURE_CEILING_PA.to_bits()
            && self.pressure_ceiling_pa > self.reference_pressure_pa
            && self.bulk_modulus_pa > 0.0
            && self.density_kg_m3 > 0.0
            && self.length_m > 0.0
            && self.bore_diameter_m > 0.0
            && self.vapor_pressure_pa > 0.0
            && self.vapor_pressure_pa < self.reference_pressure_pa
            && self.volume_m3().is_finite()
            && self.volume_m3() > 0.0
            && self.card_mass_kg().is_finite()
            && self.card_mass_kg() > 0.0
            && self.card_energy_j().is_finite()
            && self.card_energy_j() > 0.0
            && self.acoustic_omega_rad_s().is_finite()
            && self.acoustic_omega_rad_s() > 0.0
    }

    #[must_use]
    pub fn area_m2(&self) -> f64 {
        let radius_m = self.bore_diameter_m * 0.5;
        core::f64::consts::PI * radius_m * radius_m
    }

    #[must_use]
    pub fn volume_m3(&self) -> f64 {
        self.area_m2() * self.length_m
    }

    /// `A` times the 3 mm campaign overshoot.
    #[must_use]
    pub fn stroke_volume_m3(&self) -> f64 {
        self.area_m2() * COMPRESSION_LIMIT_M
    }

    /// `√(β / ρ)`.
    #[must_use]
    pub fn wave_speed_m_s(&self) -> f64 {
        (self.bulk_modulus_pa / self.density_kg_m3).sqrt()
    }

    /// Half-wave `π c / L`.
    #[must_use]
    pub fn acoustic_omega_rad_s(&self) -> f64 {
        core::f64::consts::PI * self.wave_speed_m_s() / self.length_m
    }

    /// Largest reporting step of the oil column. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let omega = self.acoustic_omega_rad_s();
        if omega.is_finite() && omega > 0.0 {
            Some(GATE_PHASE_STEP_RAD / omega)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step turns the half-wave of the oil column by two hundredths of a radian. A million times smaller, a step
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

    /// Largest `|Q|` with `|Q| * dt_s <= stroke_volume_m3` (`A * 3 mm`), the
    /// product rounded once to 64 bits and compared with the constant, as
    /// [`step`] applies it.
    #[must_use]
    pub fn flow_nyquist_m3_s(&self, dt_s: f64) -> Option<f64> {
        if !(dt_s.is_finite() && dt_s > 0.0) {
            return None;
        }
        crate::impulse::largest(self.stroke_volume_m3(), dt_s)
    }

    /// Largest `|Q|` the card takes at any step: the bore speed
    /// `(P_ceiling − P_ref) / (ρ c)` times the bore area. A flow that starts
    /// at the end face sends a pressure front of `ρ c |Q| / A` down the column.
    /// At this flow that front equals the card's whole span from the charge
    /// to the ceiling, and the bore speed is 0.012 of the wave speed, the
    /// strain of the column at the ceiling. The bound is inclusive.
    #[must_use]
    pub fn flow_acoustic_m3_s(&self) -> f64 {
        self.area_m2() * (self.pressure_ceiling_pa - self.reference_pressure_pa)
            / (self.density_kg_m3 * self.wave_speed_m_s())
    }

    /// Atmospheric fill.
    #[must_use]
    pub fn card_mass_kg(&self) -> f64 {
        self.density_kg_m3 * self.volume_m3()
    }

    /// Elastic energy at [`COMPRESSION_LIMIT_M`].
    #[must_use]
    pub fn card_energy_j(&self) -> f64 {
        let delta_p = self.bulk_modulus_pa * COMPRESSION_LIMIT_M / self.length_m;
        self.volume_m3() * delta_p * delta_p / (2.0 * self.bulk_modulus_pa)
    }

    #[must_use]
    pub fn eos_mass_kg(&self, pressure_pa: f64) -> f64 {
        let strain = (pressure_pa - self.reference_pressure_pa) / self.bulk_modulus_pa;
        self.card_mass_kg() * (1.0 + strain)
    }

    #[must_use]
    pub fn elastic_energy_j(&self, pressure_pa: f64) -> f64 {
        let delta_p = pressure_pa - self.reference_pressure_pa;
        self.volume_m3() * delta_p * delta_p / (2.0 * self.bulk_modulus_pa)
    }

    #[must_use]
    pub fn compression_m(&self, pressure_pa: f64) -> f64 {
        self.length_m * (pressure_pa - self.reference_pressure_pa) / self.bulk_modulus_pa
    }
}

/// Integrated column. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidState {
    pub t_s: f64,
    pub pressure_pa: f64,
    pub flow_m3_s: f64,
    pub fluid_volume_m3: f64,
    pub compression_m: f64,
    pub mass_kg: f64,
    pub elastic_energy_j: f64,
    pub mass_anchor_kg: f64,
    pub energy_anchor_j: f64,
    pub boundary_mass_kg: f64,
    pub boundary_work_j: f64,
    pub mass_defect_kg: f64,
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    pub vapor_margin_pa: f64,
    pub column_soft: bool,
    pub column_limited: bool,
}

impl FluidState {
    /// Atmospheric charge, valves shut.
    #[must_use]
    pub fn try_rest(column: &FluidColumn) -> Option<Self> {
        Self::try_pressure(column, REFERENCE_PRESSURE_PA)
    }

    /// Anchor the ledger at this absolute pressure.
    #[must_use]
    pub fn try_pressure(column: &FluidColumn, pressure_pa: f64) -> Option<Self> {
        if !column.is_valid() || !pressure_pa.is_finite() || pressure_pa < VAPOR_PRESSURE_PA {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            pressure_pa,
            flow_m3_s: 0.0,
            fluid_volume_m3: 0.0,
            compression_m: 0.0,
            mass_kg: 0.0,
            elastic_energy_j: 0.0,
            mass_anchor_kg: 0.0,
            energy_anchor_j: 0.0,
            boundary_mass_kg: 0.0,
            boundary_work_j: 0.0,
            mass_defect_kg: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            vapor_margin_pa: 0.0,
            column_soft: false,
            column_limited: false,
        };
        refresh(column, &mut state);
        if !state_is_finite(&state) {
            return None;
        }
        state.mass_anchor_kg = state.mass_kg;
        state.energy_anchor_j = state.elastic_energy_j;
        state.boundary_mass_kg = 0.0;
        state.boundary_work_j = 0.0;
        state.mass_defect_kg = 0.0;
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
    column: &FluidColumn,
    state: &mut FluidState,
    frame: &mut LastStateFrame64,
    load: FluidLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(column, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("hydraulic_fluid", exceedances(column, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    column: &FluidColumn,
    state: &mut FluidState,
    frame: &mut LastStateFrame64,
    load: FluidLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(column, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    column: &FluidColumn,
    state: &mut FluidState,
    frame: &mut LastStateFrame64,
    load: FluidLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(column, state, load, dt_s)?;
    let predicted = match rk4(column, pack(state), dt_s, load.flow_m3_s) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(column, state, frame, load, dt_s, predicted)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut FluidState) {
    state.t_s = normal_or_zero(state.t_s);
    state.pressure_pa = normal_or_zero(state.pressure_pa);
    state.flow_m3_s = normal_or_zero(state.flow_m3_s);
    state.fluid_volume_m3 = normal_or_zero(state.fluid_volume_m3);
    state.compression_m = normal_or_zero(state.compression_m);
    state.mass_kg = normal_or_zero(state.mass_kg);
    state.elastic_energy_j = normal_or_zero(state.elastic_energy_j);
    state.mass_anchor_kg = normal_or_zero(state.mass_anchor_kg);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.boundary_mass_kg = normal_or_zero(state.boundary_mass_kg);
    state.boundary_work_j = normal_or_zero(state.boundary_work_j);
    state.mass_defect_kg = normal_or_zero(state.mass_defect_kg);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.vapor_margin_pa = normal_or_zero(state.vapor_margin_pa);
}

fn validate(
    column: &FluidColumn,
    state: &FluidState,
    load: FluidLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.flow_m3_s.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !column.is_valid() {
        return Err(StepReject::InvalidChamber);
    }
    if state.pressure_pa < column.vapor_pressure_pa {
        return Err(StepReject::Cavitation);
    }
    if state.pressure_pa > column.pressure_ceiling_pa {
        return Err(StepReject::PressureAboveCard);
    }
    if exceeds_flow_nyquist(column, load.flow_m3_s, dt_s) {
        return Err(StepReject::FlowAboveNyquist);
    }
    if load.flow_m3_s.abs() > column.flow_acoustic_m3_s() {
        return Err(StepReject::FlowAboveAcoustic);
    }
    Ok(())
}

fn exceeds_flow_nyquist(column: &FluidColumn, flow_m3_s: f64, dt_s: f64) -> bool {
    if flow_m3_s == 0.0 {
        return false;
    }
    match column.flow_nyquist_m3_s(dt_s) {
        Some(_) => !crate::impulse::within(flow_m3_s, dt_s, column.stroke_volume_m3()),
        None => true,
    }
}

fn commit(
    column: &FluidColumn,
    state: &mut FluidState,
    frame: &mut LastStateFrame64,
    load: FluidLoad,
    dt_s: f64,
    predicted: [f64; 3],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite3(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    if predicted[0] < column.vapor_pressure_pa {
        return Err(StepReject::Cavitation);
    }
    if predicted[0] > column.pressure_ceiling_pa {
        return Err(StepReject::PressureAboveCard);
    }
    state.pressure_pa = predicted[0];
    state.boundary_mass_kg = predicted[1];
    state.boundary_work_j = predicted[2];
    state.flow_m3_s = load.flow_m3_s;
    state.t_s += dt_s;
    refresh(column, state);
    let mass_scale = column.card_mass_kg();
    let energy_scale = column.card_energy_j();
    if !(mass_scale > 0.0
        && mass_scale.is_finite()
        && energy_scale > 0.0
        && energy_scale.is_finite())
    {
        *state = saved;
        return Err(StepReject::InvalidChamber);
    }
    state.mass_defect_kg = state.mass_kg - state.mass_anchor_kg - state.boundary_mass_kg;
    state.energy_defect_j = state.elastic_energy_j - state.energy_anchor_j - state.boundary_work_j;
    let mass_residual = state.mass_defect_kg.abs() / mass_scale;
    let energy_residual = state.energy_defect_j.abs() / energy_scale;
    state.residual_conservation_error = if mass_residual > energy_residual {
        mass_residual
    } else {
        energy_residual
    };
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    let mass_gate = RESIDUAL_CONSERVATION_LIMIT * mass_scale;
    let energy_gate = RESIDUAL_CONSERVATION_LIMIT * energy_scale;
    if state.mass_defect_kg > mass_gate || state.energy_defect_j > energy_gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.mass_defect_kg < -mass_gate || state.energy_defect_j < -energy_gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    publish(state, frame);
    Ok(())
}

fn refresh(column: &FluidColumn, state: &mut FluidState) {
    state.mass_kg = column.eos_mass_kg(state.pressure_pa);
    state.fluid_volume_m3 = state.mass_kg / column.density_kg_m3;
    state.compression_m = column.compression_m(state.pressure_pa);
    state.elastic_energy_j = column.elastic_energy_j(state.pressure_pa);
    state.vapor_margin_pa = state.pressure_pa - column.vapor_pressure_pa;
    state.column_soft = state.compression_m >= COMPRESSION_SOFT_M;
    state.column_limited = state.compression_m >= COMPRESSION_LIMIT_M;
}

fn publish(state: &FluidState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.pressure_pa as f32;
    frame.pos[1] = state.fluid_volume_m3 as f32;
    frame.pos[2] = state.compression_m as f32;
    frame.vel[0] = state.mass_kg as f32;
    frame.vel[1] = state.boundary_mass_kg as f32;
    frame.vel[2] = state.elastic_energy_j as f32;
    frame.force_torque = state.vapor_margin_pa as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0u64;
    if state.column_soft {
        flags |= FLAG_COLUMN_SOFT;
    }
    if state.column_limited {
        flags |= FLAG_COLUMN_LIMIT;
    }
    frame.flags = flags;
}

fn derivatives(column: &FluidColumn, y: [f64; 3], flow_m3_s: f64) -> Option<[f64; 3]> {
    let pressure_pa = y[0];
    let volume = column.volume_m3();
    if !(volume > 0.0) || !pressure_pa.is_finite() {
        return None;
    }
    let pressure_rate = column.bulk_modulus_pa / volume * flow_m3_s;
    let mass_rate = column.density_kg_m3 * flow_m3_s;
    let power_w = (pressure_pa - column.reference_pressure_pa) * flow_m3_s;
    let out = [pressure_rate, mass_rate, power_w];
    if finite3(out) { Some(out) } else { None }
}

fn rk4(column: &FluidColumn, y: [f64; 3], dt: f64, flow_m3_s: f64) -> Option<[f64; 3]> {
    let k1 = derivatives(column, y, flow_m3_s)?;
    let k2 = derivatives(column, axpy(y, dt * 0.5, k1), flow_m3_s)?;
    let k3 = derivatives(column, axpy(y, dt * 0.5, k2), flow_m3_s)?;
    let k4 = derivatives(column, axpy(y, dt, k3), flow_m3_s)?;
    combine(y, dt, k1, k2, k3, k4)
}

fn combine(
    y: [f64; 3],
    dt: f64,
    k1: [f64; 3],
    k2: [f64; 3],
    k3: [f64; 3],
    k4: [f64; 3],
) -> Option<[f64; 3]> {
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

fn pack(state: &FluidState) -> [f64; 3] {
    [
        state.pressure_pa,
        state.boundary_mass_kg,
        state.boundary_work_j,
    ]
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn state_is_finite(state: &FluidState) -> bool {
    state.t_s.is_finite()
        && state.pressure_pa.is_finite()
        && state.flow_m3_s.is_finite()
        && state.fluid_volume_m3.is_finite()
        && state.compression_m.is_finite()
        && state.mass_kg.is_finite()
        && state.elastic_energy_j.is_finite()
        && state.mass_anchor_kg.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.boundary_mass_kg.is_finite()
        && state.boundary_work_j.is_finite()
        && state.mass_defect_kg.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.vapor_margin_pa.is_finite()
}

/// Explicit Euler. The work term keeps the left rectangle and drops the pressure triangle.
#[cfg(test)]
fn step_euler(
    column: &FluidColumn,
    state: &mut FluidState,
    frame: &mut LastStateFrame64,
    load: FluidLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(column, state, load, dt_s)?;
    let k1 = match derivatives(column, pack(state), load.flow_m3_s) {
        Some(k) => k,
        None => return Err(StepReject::NonFiniteState),
    };
    let predicted = axpy(pack(state), dt_s, k1);
    if !finite3(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    commit(column, state, frame, load, dt_s, predicted)
}

/// Add `extra_j` to the integrated work after a faithful RK4 step.
#[cfg(test)]
fn step_bias_work(
    column: &FluidColumn,
    state: &mut FluidState,
    frame: &mut LastStateFrame64,
    load: FluidLoad,
    dt_s: f64,
    extra_j: f64,
) -> Result<(), StepReject> {
    validate(column, state, load, dt_s)?;
    let mut predicted = match rk4(column, pack(state), dt_s, load.flow_m3_s) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    predicted[2] += extra_j;
    commit(column, state, frame, load, dt_s, predicted)
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    SOFT_KIND,
    LIMIT_KIND,
];

const SOFT_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_COLUMN_SOFT",
    FLAG_COLUMN_SOFT,
    "compression_m",
    "m",
    Side::AtOrAbove,
    COMPRESSION_SOFT_M,
    "the column compression at which the column is soft, 1.8 mm",
);
const LIMIT_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_COLUMN_LIMIT",
    FLAG_COLUMN_LIMIT,
    "compression_m",
    "m",
    Side::AtOrAbove,
    COMPRESSION_LIMIT_M,
    "the column compression limit, 3.0 mm",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
pub fn exceedances(
    column: &FluidColumn,
    before: &FluidState,
    after: &FluidState,
    load: FluidLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (column, before, load, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.column_soft, Exceedance::fixed(SOFT_KIND, after.compression_m));
    records.push_if(after.column_limited, Exceedance::fixed(LIMIT_KIND, after.compression_m));
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
        BULK_MODULUS_PA, COMPRESSION_LIMIT_M, COMPRESSION_SOFT_M, DENSITY_KG_M3, FLAG_COLUMN_LIMIT,
        FLAG_COLUMN_SOFT, FluidColumn, FluidLoad, FluidState, REFERENCE_PRESSURE_PA,
        RESIDUAL_CONSERVATION_LIMIT, VAPOR_PRESSURE_PA, step, step_bias_work, step_euler,
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

    fn flow_for_compression(column: &FluidColumn, compression_m: f64, dt_s: f64) -> f64 {
        compression_m * column.area_m2() / dt_s
    }

    #[test]
    fn card_locks_bulk_modulus_and_the_campaign_oil() {
        assert_copy::<FluidColumn>();
        assert_copy::<FluidState>();
        assert_copy::<FluidLoad>();
        assert_copy::<super::StepReject>();
        let column = FluidColumn::mineral_oil();
        assert_eq!(column.bulk_modulus_pa.to_bits(), BULK_MODULUS_PA.to_bits());
        assert_eq!(column.density_kg_m3.to_bits(), DENSITY_KG_M3.to_bits());
        assert!(column.wave_speed_m_s() > 1_300.0 && column.wave_speed_m_s() < 1_500.0);
        let gate = column.gate_dt_s().unwrap();
        assert!(gate > 0.0);
        let mut tampered = column;
        tampered.bulk_modulus_pa = 1.0e9;
        assert!(!tampered.is_valid());
        let mut state = FluidState::try_rest(&column).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(
            &tampered,
            &mut state,
            &mut frame,
            FluidLoad::default(),
            gate,
        );
        assert_eq!(got, Err(super::StepReject::InvalidChamber));
        assert_eq!(state, saved);
        proof_intact(&frame);
    }

    #[test]
    fn inflow_matches_the_column_law_and_a_wide_step_is_not_gated() {
        let column = FluidColumn::mineral_oil();
        let gate = column.gate_dt_s().unwrap();
        let dt = gate * 1_000.0;
        let compression = 0.002;
        let flow = flow_for_compression(&column, compression, dt);
        let mut state = FluidState::try_rest(&column).unwrap();
        let mut frame = blank_frame();
        step(
            &column,
            &mut state,
            &mut frame,
            FluidLoad { flow_m3_s: flow },
            dt,
        )
        .unwrap();
        let expected_p =
            REFERENCE_PRESSURE_PA + column.bulk_modulus_pa * compression / column.length_m;
        assert!(
            (state.pressure_pa - expected_p).abs() < 1.0e-2,
            "pressure {}",
            state.pressure_pa
        );
        assert!((state.compression_m - compression).abs() < 1.0e-12);
        assert!(state.column_soft);
        assert!(!state.column_limited);
        assert_eq!(frame.flags, FLAG_COLUMN_SOFT);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(state.vapor_margin_pa > 0.0);
        proof_intact(&frame);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
    }

    #[test]
    fn shut_valves_hold_at_a_huge_dt_and_the_gate_is_not_applied() {
        let column = FluidColumn::mineral_oil();
        let mut state = FluidState::try_rest(&column).unwrap();
        let mut frame = blank_frame();
        let huge = column.gate_dt_s().unwrap() * 1.0e6;
        step(&column, &mut state, &mut frame, FluidLoad::default(), huge).unwrap();
        assert_eq!(state.pressure_pa.to_bits(), REFERENCE_PRESSURE_PA.to_bits());
        assert_eq!(state.residual_conservation_error, 0.0);
        step(
            &column,
            &mut state,
            &mut frame,
            FluidLoad::default(),
            f64::MAX,
        )
        .unwrap();
        assert_eq!(state.pressure_pa.to_bits(), REFERENCE_PRESSURE_PA.to_bits());
        proof_intact(&frame);
    }

    #[test]
    fn stroke_boundary_flags_the_limit_and_the_next_gulp_is_refused() {
        let column = FluidColumn::mineral_oil();
        let dt = 1.0e-3;
        let flow = flow_for_compression(&column, COMPRESSION_LIMIT_M, dt);
        let mut state = FluidState::try_rest(&column).unwrap();
        let mut frame = blank_frame();
        step(
            &column,
            &mut state,
            &mut frame,
            FluidLoad { flow_m3_s: flow },
            dt,
        )
        .unwrap();
        assert!(state.compression_m + 1.0e-12 >= COMPRESSION_LIMIT_M);
        assert!(state.column_soft && state.column_limited);
        assert_eq!(frame.flags, FLAG_COLUMN_SOFT | FLAG_COLUMN_LIMIT);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        let saved = state;
        let hot = FluidLoad {
            flow_m3_s: flow + 1.0,
        };
        let got = step(&column, &mut state, &mut frame, hot, dt);
        assert_eq!(got, Err(super::StepReject::FlowAboveNyquist));
        assert_eq!(state, saved);
        proof_intact(&frame);
    }

    #[test]
    fn suction_below_vapor_pressure_rolls_back() {
        let column = FluidColumn::mineral_oil();
        let dt = 1.0e-4;
        let dp = -1_000.0 - REFERENCE_PRESSURE_PA;
        let flow = dp * column.volume_m3() / column.bulk_modulus_pa / dt;
        assert!(flow.abs() * dt < column.stroke_volume_m3());
        let mut state = FluidState::try_rest(&column).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step(
            &column,
            &mut state,
            &mut frame,
            FluidLoad { flow_m3_s: flow },
            dt,
        );
        assert_eq!(got, Err(super::StepReject::Cavitation));
        assert_eq!(state, saved);
        proof_intact(&frame);

        let mild =
            (50_000.0 - REFERENCE_PRESSURE_PA) * column.volume_m3() / column.bulk_modulus_pa / dt;
        step(
            &column,
            &mut state,
            &mut frame,
            FluidLoad { flow_m3_s: mild },
            dt,
        )
        .unwrap();
        assert!(state.pressure_pa > VAPOR_PRESSURE_PA);
        assert!((state.pressure_pa - 50_000.0).abs() < 1.0e-2);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    #[test]
    fn rectangle_work_creates_energy_and_a_biased_ledger_rolls_back() {
        let column = FluidColumn::mineral_oil();
        let dt = 1.0e-3;
        let flow = flow_for_compression(&column, COMPRESSION_SOFT_M, dt);
        let load = FluidLoad { flow_m3_s: flow };
        let mut state = FluidState::try_rest(&column).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let got = step_euler(&column, &mut state, &mut frame, load, dt);
        assert_eq!(got, Err(super::StepReject::SpontaneousGeneration));
        assert_eq!(state, saved);
        proof_intact(&frame);

        step(&column, &mut state, &mut frame, load, dt).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        let held = state;
        let extra = column.card_energy_j() * 1.0e-3;
        let got = step_bias_work(&column, &mut state, &mut frame, load, dt, extra);
        assert_eq!(got, Err(super::StepReject::FirstLawBreach));
        assert_eq!(state, held);
        let got = step_bias_work(&column, &mut state, &mut frame, load, dt, -extra);
        assert_eq!(got, Err(super::StepReject::SpontaneousGeneration));
        assert_eq!(state, held);
        proof_intact(&frame);
    }

    #[test]
    fn non_finite_inputs_leave_the_frame_alone() {
        let column = FluidColumn::mineral_oil();
        let mut state = FluidState::try_rest(&column).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let dt = 1.0e-3;
        let mut refuse = |load: FluidLoad, dt_s: f64, expect: super::StepReject| {
            let got = step(&column, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state, saved);
            proof_intact(&frame);
        };
        refuse(FluidLoad::default(), 0.0, super::StepReject::NonPositiveDt);
        refuse(FluidLoad::default(), -dt, super::StepReject::NonPositiveDt);
        refuse(
            FluidLoad::default(),
            f64::NAN,
            super::StepReject::NonFiniteInput,
        );
        refuse(
            FluidLoad {
                flow_m3_s: f64::INFINITY,
            },
            dt,
            super::StepReject::NonFiniteInput,
        );
        refuse(
            FluidLoad {
                flow_m3_s: f64::MAX,
            },
            dt,
            super::StepReject::FlowAboveNyquist,
        );
        state.pressure_pa = f64::NAN;
        let got = step(&column, &mut state, &mut frame, FluidLoad::default(), dt);
        assert_eq!(got, Err(super::StepReject::NonFiniteState));
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::FluidColumn::mineral_oil();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The column at 2 MPa with one cubic centimetre a second flowing in.
        let mut state = super::FluidState::try_pressure(&card, 2.0e6).unwrap();
        let load = super::FluidLoad { flow_m3_s: 1.0e-6 };
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
        assert!(state.pressure_pa != before.pressure_pa);
        assert!(state.mass_kg != before.mass_kg);
        assert!(state.boundary_mass_kg != before.boundary_mass_kg);
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

    fn state_bits(state: &FluidState) -> [u64; 17] {
        [
            state.t_s.to_bits(),
            state.pressure_pa.to_bits(),
            state.flow_m3_s.to_bits(),
            state.fluid_volume_m3.to_bits(),
            state.compression_m.to_bits(),
            state.mass_kg.to_bits(),
            state.elastic_energy_j.to_bits(),
            state.mass_anchor_kg.to_bits(),
            state.energy_anchor_j.to_bits(),
            state.boundary_mass_kg.to_bits(),
            state.boundary_work_j.to_bits(),
            state.mass_defect_kg.to_bits(),
            state.energy_defect_j.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.vapor_margin_pa.to_bits(),
            u64::from(state.column_soft),
            u64::from(state.column_limited),
        ]
    }

    /// Step, and require this refusal with the state and the frame bytes as they were.
    fn refused_whole(
        column: &FluidColumn,
        state: &mut FluidState,
        frame: &mut LastStateFrame64,
        load: FluidLoad,
        dt_s: f64,
        expect: super::StepReject,
    ) {
        let held = state_bits(state);
        let held_frame = frame.to_bytes();
        let got = step(column, state, frame, load, dt_s);
        assert_eq!(got, Err(expect));
        assert_eq!(state_bits(state), held);
        assert_eq!(frame.to_bytes(), held_frame);
    }

    #[test]
    fn the_pressure_ceiling_lands_and_the_next_number_above_it_is_refused() {
        let column = FluidColumn::mineral_oil();
        let dt = column.gate_dt_s().unwrap();
        let ceiling = super::PRESSURE_CEILING_PA;
        assert_eq!(column.pressure_ceiling_pa.to_bits(), ceiling.to_bits());
        // The ceiling is 6.0 mm of compression, twice the hard overshoot.
        assert_eq!(
            column.compression_m(ceiling).to_bits(),
            (2.0 * COMPRESSION_LIMIT_M).to_bits()
        );

        // Valves shut on the ceiling itself: the step lands, with both marks flagged.
        let mut state = FluidState::try_pressure(&column, ceiling).unwrap();
        let mut frame = blank_frame();
        step(&column, &mut state, &mut frame, FluidLoad::default(), dt).unwrap();
        assert_eq!(state.pressure_pa.to_bits(), ceiling.to_bits());
        assert_eq!(frame.flags, FLAG_COLUMN_SOFT | FLAG_COLUMN_LIMIT);
        assert_eq!(state.residual_conservation_error, 0.0);
        proof_intact(&frame);

        // The next representable pressure is outside the card, valves shut or open.
        let above = ceiling.next_up();
        assert!(above > ceiling);
        let mut over = FluidState::try_pressure(&column, above).unwrap();
        let mut frame = blank_frame();
        refused_whole(
            &column,
            &mut over,
            &mut frame,
            FluidLoad::default(),
            dt,
            super::StepReject::PressureAboveCard,
        );
        refused_whole(
            &column,
            &mut over,
            &mut frame,
            FluidLoad { flow_m3_s: -1.0e-6 },
            dt,
            super::StepReject::PressureAboveCard,
        );
        proof_intact(&frame);

        // A landing: from 5.0 mm, find the two neighbouring flows of which
        // one lands and the next is refused. The step is wider than the
        // gate, which the kernel does not apply, so a 1 mm step stays under
        // the acoustic flow limit.
        let dt = 1.0e-4;
        let start_pa = REFERENCE_PRESSURE_PA + BULK_MODULUS_PA * 0.005 / column.length_m;
        let lands = |flow_m3_s: f64| {
            let mut state = FluidState::try_pressure(&column, start_pa).unwrap();
            let mut frame = blank_frame();
            match step(&column, &mut state, &mut frame, FluidLoad { flow_m3_s }, dt) {
                Ok(()) => Some(state.pressure_pa),
                Err(super::StepReject::PressureAboveCard) => None,
                Err(other) => std::panic!("flow {flow_m3_s} refused with {other:?}"),
            }
        };
        let mut inside = flow_for_compression(&column, 0.0009, dt);
        let mut outside = flow_for_compression(&column, 0.0011, dt);
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
        let landed_pa = lands(inside).unwrap();
        assert!(landed_pa <= ceiling);
        assert!(ceiling - landed_pa <= 8.0 * (ceiling.next_up() - ceiling));
        let mut state = FluidState::try_pressure(&column, start_pa).unwrap();
        let mut frame = blank_frame();
        refused_whole(
            &column,
            &mut state,
            &mut frame,
            FluidLoad { flow_m3_s: outside },
            dt,
            super::StepReject::PressureAboveCard,
        );
        proof_intact(&frame);

        // The recorded copy is locked like the other constants.
        let mut tampered = column;
        tampered.pressure_ceiling_pa = 1.0e9;
        assert!(!tampered.is_valid());
        refused_whole(
            &tampered,
            &mut state,
            &mut frame,
            FluidLoad::default(),
            dt,
            super::StepReject::InvalidChamber,
        );
    }

    /// Steady inflow from rest at the ceiling step. Without a ceiling the run
    /// at 0.001 m³/s landed 642,874 steps, to 0.74 m of compression, and was
    /// then refused as created energy through rounding alone.
    #[test]
    fn steady_inflow_from_rest_stops_at_the_ceiling_by_name() {
        let column = FluidColumn::mineral_oil();
        let dt = column.gate_dt_s().unwrap();
        let ceiling_m = column.compression_m(super::PRESSURE_CEILING_PA);
        // Flow, and the largest residual the run may show before it stops.
        let cases = [
            (1.0e-3, 1.0e-11),
            (3.0e-4, 1.0e-10),
            (0.01, 1.0e-11),
            (column.flow_acoustic_m3_s(), 1.0e-11),
        ];
        let mut case = 0;
        while case < cases.len() {
            let (flow_m3_s, residual_allowed) = cases[case];
            let load = FluidLoad { flow_m3_s };
            let per_step_m = flow_m3_s * dt / column.area_m2();
            let expected = (ceiling_m / per_step_m) as u32;
            let mut state = FluidState::try_rest(&column).unwrap();
            let mut frame = blank_frame();
            let mut landed = 0u32;
            let mut max_residual = 0.0;
            loop {
                let held = state_bits(&state);
                let held_frame = frame.to_bytes();
                match step(&column, &mut state, &mut frame, load, dt) {
                    Ok(()) => {
                        landed += 1;
                        assert!(
                            landed <= expected + 1,
                            "flow {flow_m3_s} ran past the ceiling"
                        );
                        assert!(state.pressure_pa <= super::PRESSURE_CEILING_PA);
                        if state.residual_conservation_error > max_residual {
                            max_residual = state.residual_conservation_error;
                        }
                    }
                    Err(super::StepReject::PressureAboveCard) => {
                        assert_eq!(state_bits(&state), held);
                        assert_eq!(frame.to_bytes(), held_frame);
                        break;
                    }
                    Err(other) => {
                        std::panic!("flow {flow_m3_s} refused with {other:?} after {landed} steps")
                    }
                }
            }
            assert!(
                landed + 1 >= expected && landed <= expected + 1,
                "flow {flow_m3_s} landed {landed}, expected {expected}"
            );
            assert!(ceiling_m - state.compression_m < 1.0001 * per_step_m);
            assert!(
                max_residual <= residual_allowed,
                "flow {flow_m3_s} residual {max_residual}"
            );
            assert!(state.column_soft && state.column_limited);
            // The refusal repeats, and drawing oil out is still a step the card takes.
            refused_whole(
                &column,
                &mut state,
                &mut frame,
                load,
                dt,
                super::StepReject::PressureAboveCard,
            );
            step(
                &column,
                &mut state,
                &mut frame,
                FluidLoad {
                    flow_m3_s: -flow_m3_s,
                },
                dt,
            )
            .unwrap();
            proof_intact(&frame);
            case += 1;
        }
    }

    /// The starting pressures at which the ledger used to answer with
    /// rounding: 0.23 m, 0.26 m and 0.29 m of compression, and far beyond.
    #[test]
    fn a_start_above_the_ceiling_is_refused_by_name_before_the_ledger() {
        let column = FluidColumn::mineral_oil();
        let dt = column.gate_dt_s().unwrap();
        let starts = [8.0e8, 9.0e8, 1.0e9, 1.0e12, 1.0e14, 1.0e25];
        let mut i = 0;
        while i < starts.len() {
            let mut state = FluidState::try_pressure(&column, starts[i]).unwrap();
            let mut frame = blank_frame();
            let spacing_pa = starts[i].next_up() - starts[i];
            let stall = FluidLoad {
                flow_m3_s: 0.4999 * spacing_pa * column.volume_m3() / (column.bulk_modulus_pa * dt),
            };
            refused_whole(
                &column,
                &mut state,
                &mut frame,
                stall,
                dt,
                super::StepReject::PressureAboveCard,
            );
            refused_whole(
                &column,
                &mut state,
                &mut frame,
                FluidLoad::default(),
                dt,
                super::StepReject::PressureAboveCard,
            );
            proof_intact(&frame);
            i += 1;
        }
    }

    /// The hardest case for the ledger's rounding under the ceiling: an inflow
    /// so small that each step's pressure rise is just under half the spacing
    /// of 64-bit numbers at that pressure. The pressure stands still while
    /// the admitted mass and the work keep counting, for a million steps.
    #[test]
    fn a_million_stalled_steps_under_the_ceiling_stay_far_inside_the_limit() {
        let column = FluidColumn::mineral_oil();
        let dt = column.gate_dt_s().unwrap();
        let start_pa = REFERENCE_PRESSURE_PA + BULK_MODULUS_PA * 0.00599 / column.length_m;
        assert!(start_pa < super::PRESSURE_CEILING_PA);
        let spacing_pa = start_pa.next_up() - start_pa;
        let load = FluidLoad {
            flow_m3_s: 0.4999 * spacing_pa * column.volume_m3() / (column.bulk_modulus_pa * dt),
        };
        let mut state = FluidState::try_pressure(&column, start_pa).unwrap();
        let mut frame = blank_frame();
        let mut max_residual = 0.0;
        let mut n = 0u32;
        while n < 1_000_000 {
            step(&column, &mut state, &mut frame, load, dt).unwrap();
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            n += 1;
        }
        assert_eq!(state.pressure_pa.to_bits(), start_pa.to_bits());
        assert!(state.boundary_work_j > 0.0);
        // Measured 7.2e-10. The limit is 1.0e-6.
        assert!(
            max_residual > 1.0e-10 && max_residual < 1.0e-9,
            "residual {max_residual}"
        );
        assert!(max_residual < RESIDUAL_CONSERVATION_LIMIT / 1_000.0);
        proof_intact(&frame);
    }
}

#[cfg(test)]
mod acoustic_tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{FluidColumn, FluidLoad, FluidState, StepReject, step};
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

    fn take(column: &FluidColumn, flow_m3_s: f64, dt_s: f64) -> Result<FluidState, StepReject> {
        let mut state = FluidState::try_rest(column).unwrap();
        let held = state;
        let mut frame = frame();
        match step(column, &mut state, &mut frame, FluidLoad { flow_m3_s }, dt_s) {
            Ok(()) => Ok(state),
            Err(reject) => {
                assert_eq!(state, held);
                assert_eq!(frame.to_bytes(), self::frame().to_bytes());
                Err(reject)
            }
        }
    }

    /// The bound is 0.012 of the wave speed through the bore, the strain of
    /// the column at the ceiling, and a pressure front of that bore speed is
    /// the card's span from the charge to the ceiling.
    #[test]
    fn the_acoustic_flow_limit_is_the_card_span_as_a_pressure_front() {
        let column = FluidColumn::mineral_oil();
        let flow = column.flow_acoustic_m3_s();
        let speed = flow / column.area_m2();
        let c = column.wave_speed_m_s();
        assert!((speed / c - 0.012).abs() < 1.0e-15, "{}", speed / c);
        assert!((speed - 16.87).abs() < 0.01, "{speed}");
        assert!((flow - 0.0331).abs() < 1.0e-4, "{flow}");
        let front_pa = column.density_kg_m3 * c * speed;
        let span_pa = super::PRESSURE_CEILING_PA - super::REFERENCE_PRESSURE_PA;
        assert!((front_pa - span_pa).abs() <= 1.0e-9 * span_pa, "{front_pa}");
    }

    /// The largest flow lands in either direction at every step size of the
    /// window, and the next number above it is refused by name, before the
    /// ledger, with the state and the frame as they were.
    #[test]
    fn the_largest_flow_lands_and_the_next_number_above_it_is_refused() {
        let column = FluidColumn::mineral_oil();
        let flow = column.flow_acoustic_m3_s();
        let floor = column.floor_dt_s().unwrap();
        let gate = column.gate_dt_s().unwrap();
        for dt in [floor, floor * 1024.0, gate] {
            assert!(take(&column, flow, dt).is_ok());
            // Outflow at the limit from a start high enough not to cavitate.
            let mut state = FluidState::try_pressure(&column, 1.0e7).unwrap();
            let mut f = frame();
            step(&column, &mut state, &mut f, FluidLoad { flow_m3_s: -flow }, dt).unwrap();
            assert_eq!(take(&column, flow.next_up(), dt), Err(StepReject::FlowAboveAcoustic));
            assert_eq!(take(&column, -flow.next_up(), dt), Err(StepReject::FlowAboveAcoustic));
        }
    }

    /// Two strokes sealed on 0.6.0: a 3.0 mm stroke at the step floor,
    /// 1.39e9 m/s through the bore, and the largest stroke flow at the
    /// step ceiling, 0.94 of the wave speed. Both are refused now.
    #[test]
    fn the_strokes_sealed_on_0_6_0_are_refused_as_above_acoustic() {
        let column = FluidColumn::mineral_oil();
        let floor = column.floor_dt_s().unwrap();
        let gate = column.gate_dt_s().unwrap();
        assert_eq!(
            take(&column, 2728384.2125486047, floor),
            Err(StepReject::FlowAboveAcoustic)
        );
        assert_eq!(
            take(&column, 2.6019899487958953, gate),
            Err(StepReject::FlowAboveAcoustic)
        );
        // Past the stroke as well: the stroke rule is applied first.
        assert_eq!(
            take(&column, 2728384.2125486047_f64.next_up(), floor),
            Err(StepReject::FlowAboveNyquist)
        );
    }
}

#[cfg(test)]
mod boundary_tests {
    /// The last load accepted and the first refused, at several step sizes,
    /// agree with the published rule `|flow_m3_s| * dt_s <= stroke volume`.
    #[test]
    fn the_load_rule_is_the_published_product_compared_with_the_constant() {
        let column = super::FluidColumn::mineral_oil();
        let limit = column.stroke_volume_m3();
        crate::impulse::tests::assert_boundary(limit, |load, dt_s| super::exceeds_flow_nyquist(&column, load, dt_s));
    }
}
