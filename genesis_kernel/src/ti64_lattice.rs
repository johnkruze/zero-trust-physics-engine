#![no_implicit_prelude]

//! Three-strut Ti-6Al-4V lattice constraint.
//!
//! `core` only: no heap and no standard prelude. The node is integrated in
//! `f64`. Each accepted step writes time, position, velocity, the dominant
//! ligament tension, and the conservation residual into [`LastStateFrame64`].
//! The 16-byte frame proof is not touched.
//!
//! Annealed card, catalog slot `ti64_annealed_typical`: yield 880 MPa,
//! modulus 113.8 GPa, Poisson 0.342, density 4430 kg/m³, linear CTE
//! 8.6×10⁻⁶ /K. Temperature enters only as thermal extension. The modulus
//! is the room-temperature value. The card is valid from 250 K through the
//! beta transus at 1268.15 K. Yield, modulus, density, Poisson ratio, CTE,
//! and the beta transus are crate constants. They are not step arguments.
//! [`Ti64LatticeCell::annealed_column`] is the verification specimen.
//!
//! The cell is hyperelastic. Force and temperature are constant on a step.
//! A change in either is boundary work, booked at the pre-step state:
//!
//! `residual_conservation_error = |E − E_anchor − W_boundary| / E_yield`
//!
//! `E_yield` is the linear elastic energy of all three ligaments at 880 MPa.
//! Explicit Euler is not the public step. On a harmonic ligament it multiplies
//! energy by `1 + (ω Δt)²` every step and leaves this gate. The public step
//! is classical RK4.
//!
//! The card is elastic and ends at yield. A ligament past 880 MPa is
//! [`StepReject::YieldReached`], tested in four places: on the incoming
//! state, at the instant the step's temperature is applied to the incoming
//! position, at a turning point of the motion inside the step, and on the
//! landing. Each ligament moves alone under a held force and temperature, so
//! the turning point is found on the step's own orbit: a step in which a
//! ligament's speed changes sign, or lands at zero, holds one, and its
//! stretch is where the step's energy on that ligament is all strain energy.
//! For the linear ligament that is the closed form `s_eq ± A`, with
//! `s_eq = F / k` and `A = sqrt((x − α ΔT L − s_eq)² + (v / ω)²)`. A
//! residual past the limit is [`StepReject::SpontaneousGeneration`] or
//! [`StepReject::FirstLawBreach`] by its sign. Each refusal rolls the step
//! back. A landed step sets no flag.
//!
//! A run may carry on from a state the card returned:
//! [`Ti64LatticeState::try_continue`] takes the clock, the held force and
//! temperature and the two ledger entries with the phase, so a run restarted
//! from its own returned state is the same run, bit for bit, and its ledger
//! is weighed over the whole of it.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

/// Typical annealed yield. Catalog slot `ti64_annealed_typical`.
pub const YIELD_STRENGTH_MPA: f64 = 880.0;
/// Typical solid density, 4.43 g/cm³.
pub const DENSITY_KG_M3: f64 = 4430.0;
/// Room-temperature Young's modulus.
pub const YOUNGS_MODULUS_PA: f64 = 113.8e9;
/// Room-temperature Poisson ratio.
pub const POISSON_RATIO: f64 = 0.342;
/// Mean linear CTE over the room-temperature interval.
pub const THERMAL_EXPANSION_PER_K: f64 = 8.6e-6;
/// Reference temperature for zero thermal extension.
pub const REFERENCE_TEMPERATURE_K: f64 = 293.15;
/// Lower edge of the constant-CTE card.
pub const CARD_TEMPERATURE_MIN_K: f64 = 250.0;
/// Beta transus, 995 °C. Above this the annealed card is not this model.
pub const BETA_TRANSUS_K: f64 = 1268.15;

/// Nominal composition of the annealed card. Identifies the alloy.
pub const MASS_FRACTION_TI: f64 = 0.90;
pub const MASS_FRACTION_AL: f64 = 0.06;
pub const MASS_FRACTION_V: f64 = 0.04;

/// Accumulated hallucinated energy, divided by the yield energy, must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Structural phase advance `ω Δt` used by [`Ti64LatticeCell::gate_dt_s`].
pub const GATE_PHASE_STEP_RAD: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

const PA_PER_MPA: f64 = 1.0e6;

/// Why a step did not advance the node or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Force, torque, temperature, or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state was already non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// Temperature is outside the annealed constant-CTE card.
    TemperatureOutsideCard,
    /// Length, area, hardening, or mass cannot support the card.
    InvalidCell,
    /// A ligament is past 880 MPa on the incoming state, at the instant the step's temperature is applied, at a turning point of the motion inside the step, or on the landing.
    YieldReached,
    /// Mechanical energy is above the anchor plus the booked boundary work by more than the limit.
    SpontaneousGeneration,
    /// Mechanical energy is below the anchor plus the booked boundary work by more than the limit.
    FirstLawBreach,
    /// A handed-in start states what no run of the card writes: a clock below zero, boundary work booked at a clock of zero, or a ledger whose residual is already past the limit.
    StartNotWritten,
}

/// A start handed to [`Ti64LatticeState::try_continue`]: the phase, the clock,
/// the force and temperature the state stands under, and the two ledger
/// entries. A new start has `t_s` zero, `boundary_work_j` zero and
/// `energy_anchor_j` equal to its own mechanical energy under its force and
/// temperature. A state the card returned is handed back with its own values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ti64Start {
    pub t_s: f64,
    pub position_m: [f64; 3],
    pub velocity_m_s: [f64; 3],
    pub applied_force_n: [f64; 3],
    pub temperature_k: f64,
    pub energy_anchor_j: f64,
    pub boundary_work_j: f64,
}

/// Zero-order hold applied across one step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ti64Load {
    pub force_n: [f64; 3],
    pub torque_nm: [f64; 3],
    pub temperature_k: f64,
}

impl Default for Ti64Load {
    fn default() -> Self {
        Self {
            force_n: [0.0; 3],
            torque_nm: [0.0; 3],
            temperature_k: REFERENCE_TEMPERATURE_K,
        }
    }
}

/// Cubic cell: three ligaments meeting at one free node, the other ends fixed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ti64LatticeCell {
    pub length_m: [f64; 3],
    pub area_m2: [f64; 3],
    /// Cubic hardening, N/m³. Zero is the linear ligament.
    pub hardening_n_m3: [f64; 3],
    /// Moment arm of the nodal force. It is reported and does not enter the translation.
    pub eccentricity_m: [f64; 3],
}

impl Ti64LatticeCell {
    /// Verification specimen: 20 mm ligaments, 4.0 mm² section, linear, concentric.
    ///
    /// Material limits stay on the annealed card. This constructor does not
    /// take a yield strength or a beta transus.
    #[must_use]
    pub fn annealed_column() -> Self {
        match Self::try_new([0.02; 3], [4.0e-6; 3], [0.0; 3], [0.0; 3]) {
            Some(cell) => cell,
            None => ::core::panic!("annealed column card failed is_valid"),
        }
    }

    pub fn try_new(
        length_m: [f64; 3],
        area_m2: [f64; 3],
        hardening_n_m3: [f64; 3],
        eccentricity_m: [f64; 3],
    ) -> Option<Self> {
        let cell = Self {
            length_m,
            area_m2,
            hardening_n_m3,
            eccentricity_m,
        };
        if cell.is_valid() { Some(cell) } else { None }
    }

    pub fn is_valid(&self) -> bool {
        finite3(self.length_m)
            && finite3(self.area_m2)
            && finite3(self.hardening_n_m3)
            && finite3(self.eccentricity_m)
            && self.length_m[0] > 0.0
            && self.length_m[1] > 0.0
            && self.length_m[2] > 0.0
            && self.area_m2[0] > 0.0
            && self.area_m2[1] > 0.0
            && self.area_m2[2] > 0.0
            && self.hardening_n_m3[0] >= 0.0
            && self.hardening_n_m3[1] >= 0.0
            && self.hardening_n_m3[2] >= 0.0
            && self.mass_kg() > 0.0
            && self.mass_kg().is_finite()
            && self.yield_energy_scale_j() > 0.0
            && self.yield_energy_scale_j().is_finite()
    }

    pub fn mass_kg(&self) -> f64 {
        DENSITY_KG_M3
            * (self.area_m2[0] * self.length_m[0]
                + self.area_m2[1] * self.length_m[1]
                + self.area_m2[2] * self.length_m[2])
    }

    pub fn stiffness_n_m(&self) -> [f64; 3] {
        [
            YOUNGS_MODULUS_PA * self.area_m2[0] / self.length_m[0],
            YOUNGS_MODULUS_PA * self.area_m2[1] / self.length_m[1],
            YOUNGS_MODULUS_PA * self.area_m2[2] / self.length_m[2],
        ]
    }

    /// `sqrt(k_i / m)` of the stiffest ligament. The node mass is shared.
    pub fn max_natural_frequency_rad_s(&self) -> f64 {
        let k = self.stiffness_n_m();
        let mass = self.mass_kg();
        let mut best = 0.0;
        let mut i = 0;
        while i < 3 {
            let w2 = k[i] / mass;
            if w2 > best {
                best = w2;
            }
            i += 1;
        }
        best.sqrt()
    }

    /// Largest step at which the conservation gate was closed.
    pub fn gate_dt_s(&self) -> Option<f64> {
        let w = self.max_natural_frequency_rad_s();
        if w.is_finite() && w > 0.0 {
            Some(GATE_PHASE_STEP_RAD / w)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step turns the stiffest ligament's vibration by two hundredths of a radian. A million times smaller, a step
    /// changes the state by about two parts in a hundred million, which
    /// 64-bit arithmetic still resolves to eight figures. Under that a step
    /// is mostly rounding. Not applied by [`step`].
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }

    /// Linear elastic energy of the three ligaments at uniaxial yield.
    pub fn yield_energy_scale_j(&self) -> f64 {
        let k = self.stiffness_n_m();
        let strain = uniaxial_yield_strain();
        let mut energy = 0.0;
        let mut i = 0;
        while i < 3 {
            let extension = strain * self.length_m[i];
            energy += 0.5 * k[i] * extension * extension;
            i += 1;
        }
        energy
    }
}

/// Integrated node. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ti64LatticeState {
    pub t_s: f64,
    pub position_m: [f64; 3],
    pub velocity_m_s: [f64; 3],
    /// Positive tension in each ligament.
    pub tension_n: [f64; 3],
    pub torque_nm: [f64; 3],
    pub applied_force_n: [f64; 3],
    pub temperature_k: f64,
    /// Uniaxial ligament stress. These three numbers are not one Cauchy tensor.
    pub ligament_stress_pa: [f64; 3],
    /// Worst ligament von Mises stress, which for a uniaxial bar is `|σ|`.
    pub von_mises_mpa: f64,
    pub energy_j: f64,
    pub energy_anchor_j: f64,
    pub boundary_work_j: f64,
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
}

impl Ti64LatticeState {
    #[must_use]
    pub fn try_rest(cell: &Ti64LatticeCell) -> Option<Self> {
        Self::try_phase(cell, [0.0; 3], [0.0; 3])
    }

    /// Anchor the ledger to this phase at the reference temperature and zero applied force.
    #[must_use]
    pub fn try_phase(
        cell: &Ti64LatticeCell,
        position_m: [f64; 3],
        velocity_m_s: [f64; 3],
    ) -> Option<Self> {
        if !cell.is_valid() || !finite3(position_m) || !finite3(velocity_m_s) {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            position_m,
            velocity_m_s,
            tension_n: [0.0; 3],
            torque_nm: [0.0; 3],
            applied_force_n: [0.0; 3],
            temperature_k: REFERENCE_TEMPERATURE_K,
            ligament_stress_pa: [0.0; 3],
            von_mises_mpa: 0.0,
            energy_j: 0.0,
            energy_anchor_j: 0.0,
            boundary_work_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
        };
        refresh(cell, &mut state, [0.0; 3]);
        if !state_is_finite(&state) {
            return None;
        }
        state.energy_anchor_j = state.energy_j;
        state.boundary_work_j = 0.0;
        state.energy_defect_j = 0.0;
        state.residual_conservation_error = 0.0;
        Some(state)
    }

    /// Carry on from a stated start. The state is rebuilt from the start's
    /// phase, force and temperature, and its ledger is the start's own: the
    /// defect is the mechanical energy less the stated anchor and booked
    /// work. A state the card returned, handed back here, steps on exactly
    /// as the run it came from. A start the card could not have written is
    /// [`StepReject::StartNotWritten`]. Yield is not tested here: the first
    /// step tests it on the incoming state.
    pub fn try_continue(cell: &Ti64LatticeCell, start: Ti64Start) -> Result<Self, StepReject> {
        if !start.t_s.is_finite()
            || !finite3(start.position_m)
            || !finite3(start.velocity_m_s)
            || !finite3(start.applied_force_n)
            || !start.temperature_k.is_finite()
            || !start.energy_anchor_j.is_finite()
            || !start.boundary_work_j.is_finite()
        {
            return Err(StepReject::NonFiniteState);
        }
        if !cell.is_valid() {
            return Err(StepReject::InvalidCell);
        }
        if start.temperature_k < CARD_TEMPERATURE_MIN_K || start.temperature_k > BETA_TRANSUS_K {
            return Err(StepReject::TemperatureOutsideCard);
        }
        if start.t_s < 0.0 || (start.t_s == 0.0 && start.boundary_work_j != 0.0) {
            return Err(StepReject::StartNotWritten);
        }
        let mut state = Self {
            t_s: start.t_s,
            position_m: start.position_m,
            velocity_m_s: start.velocity_m_s,
            tension_n: [0.0; 3],
            torque_nm: [0.0; 3],
            applied_force_n: start.applied_force_n,
            temperature_k: start.temperature_k,
            ligament_stress_pa: [0.0; 3],
            von_mises_mpa: 0.0,
            energy_j: 0.0,
            energy_anchor_j: start.energy_anchor_j,
            boundary_work_j: start.boundary_work_j,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
        };
        refresh(cell, &mut state, [0.0; 3]);
        state.energy_defect_j = state.energy_j - state.energy_anchor_j - state.boundary_work_j;
        state.residual_conservation_error =
            state.energy_defect_j.abs() / cell.yield_energy_scale_j();
        if !state_is_finite(&state) {
            return Err(StepReject::NonFiniteState);
        }
        if state.residual_conservation_error > RESIDUAL_CONSERVATION_LIMIT {
            return Err(StepReject::StartNotWritten);
        }
        Ok(state)
    }
}

/// `G = E / (2 (1 + ν))`.
#[must_use]
pub fn shear_modulus_pa() -> f64 {
    YOUNGS_MODULUS_PA / (2.0 * (1.0 + POISSON_RATIO))
}

/// `σ_y / E` at the room-temperature card.
#[must_use]
pub fn uniaxial_yield_strain() -> f64 {
    (YIELD_STRENGTH_MPA * PA_PER_MPA) / YOUNGS_MODULUS_PA
}

/// `α (T − T_ref) L`.
#[must_use]
pub fn thermal_extension_m(length_m: f64, temperature_k: f64) -> f64 {
    THERMAL_EXPANSION_PER_K * (temperature_k - REFERENCE_TEMPERATURE_K) * length_m
}

/// Von Mises invariant of a diagonal Cauchy stress. Hydrostatic stress returns 0.
#[must_use]
pub fn cauchy_von_mises_pa(stress_pa: [f64; 3]) -> f64 {
    let d01 = stress_pa[0] - stress_pa[1];
    let d12 = stress_pa[1] - stress_pa[2];
    let d20 = stress_pa[2] - stress_pa[0];
    (0.5 * (d01 * d01 + d12 * d12 + d20 * d20)).sqrt()
}

/// Worst uniaxial ligament stress. A single bar's von Mises stress is `|σ|`.
#[must_use]
pub fn ligament_von_mises_pa(stress_pa: [f64; 3]) -> f64 {
    let mut worst = stress_pa[0].abs();
    if stress_pa[1].abs() > worst {
        worst = stress_pa[1].abs();
    }
    if stress_pa[2].abs() > worst {
        worst = stress_pa[2].abs();
    }
    worst
}

/// `e × (−tension + applied_force) + applied_torque`.
#[must_use]
pub fn nodal_torque_nm(
    eccentricity_m: [f64; 3],
    tension_n: [f64; 3],
    applied_force_n: [f64; 3],
    applied_torque_nm: [f64; 3],
) -> [f64; 3] {
    let force = [
        -tension_n[0] + applied_force_n[0],
        -tension_n[1] + applied_force_n[1],
        -tension_n[2] + applied_force_n[2],
    ];
    let moment = cross(eccentricity_m, force);
    [
        moment[0] + applied_torque_nm[0],
        moment[1] + applied_torque_nm[1],
        moment[2] + applied_torque_nm[2],
    ]
}

/// Advance one RK4 step and write the frame.
///
/// The card is elastic: plastic work is not in its ledger. A step is refused
/// as [`StepReject::YieldReached`] when it starts from a state past yield,
/// when its temperature puts the node past yield where it stands, when its
/// motion turns past yield inside the step, or when it lands past yield. A
/// rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    cell: &Ti64LatticeCell,
    state: &mut Ti64LatticeState,
    frame: &mut LastStateFrame64,
    load: Ti64Load,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(cell, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    cell: &Ti64LatticeCell,
    state: &mut Ti64LatticeState,
    frame: &mut LastStateFrame64,
    load: Ti64Load,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(cell, state, load, dt_s)?;
    let predicted = match rk4(
        cell,
        pack_phase(state),
        dt_s,
        load.force_n,
        load.temperature_k,
    ) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(cell, state, frame, load, dt_s, predicted)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut Ti64LatticeState) {
    state.t_s = normal_or_zero(state.t_s);
    normal_or_zero_each(&mut state.position_m);
    normal_or_zero_each(&mut state.velocity_m_s);
    normal_or_zero_each(&mut state.tension_n);
    normal_or_zero_each(&mut state.torque_nm);
    normal_or_zero_each(&mut state.applied_force_n);
    state.temperature_k = normal_or_zero(state.temperature_k);
    normal_or_zero_each(&mut state.ligament_stress_pa);
    state.von_mises_mpa = normal_or_zero(state.von_mises_mpa);
    state.energy_j = normal_or_zero(state.energy_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.boundary_work_j = normal_or_zero(state.boundary_work_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

fn validate(
    cell: &Ti64LatticeCell,
    state: &Ti64LatticeState,
    load: Ti64Load,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite()
        || !load.temperature_k.is_finite()
        || !finite3(load.force_n)
        || !finite3(load.torque_nm)
    {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if load.temperature_k < CARD_TEMPERATURE_MIN_K || load.temperature_k > BETA_TRANSUS_K {
        return Err(StepReject::TemperatureOutsideCard);
    }
    if !cell.is_valid() {
        return Err(StepReject::InvalidCell);
    }
    if past_yield(cell, state.position_m, state.temperature_k)
        || past_yield(cell, state.position_m, load.temperature_k)
    {
        return Err(StepReject::YieldReached);
    }
    Ok(())
}

/// The worst ligament at this node position and temperature is past 880 MPa.
/// The applied force does not enter a ligament's stress.
#[must_use]
pub fn past_yield(cell: &Ti64LatticeCell, position_m: [f64; 3], temperature_k: f64) -> bool {
    let k = cell.stiffness_n_m();
    let mut stress_pa = [0.0; 3];
    let mut i = 0;
    while i < 3 {
        stress_pa[i] = strut_law(
            k[i],
            cell.hardening_n_m3[i],
            cell.area_m2[i],
            position_m[i],
            thermal_extension_m(cell.length_m[i], temperature_k),
            0.0,
        )
        .stress_pa;
        i += 1;
    }
    ligament_von_mises_pa(stress_pa) / PA_PER_MPA > YIELD_STRENGTH_MPA
}

fn commit(
    cell: &Ti64LatticeCell,
    state: &mut Ti64LatticeState,
    frame: &mut LastStateFrame64,
    load: Ti64Load,
    dt_s: f64,
    predicted: [f64; 6],
) -> Result<(), StepReject> {
    let saved = *state;
    let energy_before = mechanical_energy(cell, state);
    state.temperature_k = load.temperature_k;
    state.applied_force_n = load.force_n;
    let energy_switched = mechanical_energy(cell, state);
    state.boundary_work_j += energy_switched - energy_before;
    state.position_m = [predicted[0], predicted[1], predicted[2]];
    state.velocity_m_s = [predicted[3], predicted[4], predicted[5]];
    state.t_s += dt_s;
    refresh(cell, state, load.torque_nm);
    let scale = cell.yield_energy_scale_j();
    state.energy_defect_j = state.energy_j - state.energy_anchor_j - state.boundary_work_j;
    if scale > 0.0 && scale.is_finite() {
        state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    } else {
        state.residual_conservation_error = f64::NAN;
    }
    let residual_breach = state.residual_conservation_error > RESIDUAL_CONSERVATION_LIMIT;
    let yielded = state.von_mises_mpa > YIELD_STRENGTH_MPA
        || turns_past_yield(cell, &saved, load, [predicted[3], predicted[4], predicted[5]]);
    if !state_is_finite(state) {
        *state = saved;
        return Err(if cell.is_valid() {
            StepReject::NonFiniteState
        } else {
            StepReject::InvalidCell
        });
    }
    if yielded {
        *state = saved;
        return Err(StepReject::YieldReached);
    }
    if residual_breach {
        let created = state.energy_defect_j > 0.0;
        *state = saved;
        return Err(if created {
            StepReject::SpontaneousGeneration
        } else {
            StepReject::FirstLawBreach
        });
    }
    publish(state, frame);
    Ok(())
}

/// A ligament turns past 880 MPa inside the step. Each ligament moves alone
/// under the step's held force and temperature. Its speed changes sign, or
/// lands at zero, in a step that holds a turning point of its motion; the
/// stretch there is where the step's energy on that ligament, taken from the
/// incoming state, is all strain energy. The linear ligament has it in
/// closed form, `s_eq + sign(v) A`; a hardened one is solved by bisection on
/// its strain energy.
fn turns_past_yield(
    cell: &Ti64LatticeCell,
    incoming: &Ti64LatticeState,
    load: Ti64Load,
    landed_velocity_m_s: [f64; 3],
) -> bool {
    let k = cell.stiffness_n_m();
    let mass = cell.mass_kg();
    let mut i = 0;
    while i < 3 {
        let v0 = incoming.velocity_m_s[i];
        let v1 = landed_velocity_m_s[i];
        let turns = (v0 > 0.0 && v1 <= 0.0) || (v0 < 0.0 && v1 >= 0.0);
        if turns {
            let x0 = incoming.position_m[i];
            let extension = thermal_extension_m(cell.length_m[i], load.temperature_k);
            let beta = cell.hardening_n_m3[i];
            let tension_n = if beta == 0.0 {
                let omega = (k[i] / mass).sqrt();
                let s_eq = load.force_n[i] / k[i];
                let offset = (x0 - extension) - s_eq;
                let swing = v0 / omega;
                let amplitude = (offset * offset + swing * swing).sqrt();
                let stretch = if v0 > 0.0 { s_eq + amplitude } else { s_eq - amplitude };
                k[i] * stretch
            } else {
                let x = hardened_turning_m(k[i], beta, mass, x0, v0, extension, load.force_n[i]);
                strut_law(k[i], beta, cell.area_m2[i], x, extension, 0.0).tension_n
            };
            let stress_pa = tension_n / cell.area_m2[i];
            if !(stress_pa.abs() / PA_PER_MPA <= YIELD_STRENGTH_MPA) {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Position where a hardened ligament moving at `v0` from `x0` comes to rest:
/// the root of `U(x) = ½ m v0² + U(x0)` on the side it moves toward, with
/// `U` the strut potential under the held force. The bracket is widened by
/// doubling and then halved to the last representable step; the end past the
/// root is returned.
fn hardened_turning_m(
    k: f64,
    beta: f64,
    mass: f64,
    x0: f64,
    v0: f64,
    extension: f64,
    force: f64,
) -> f64 {
    let potential = |x: f64| strut_law(k, beta, 1.0, x, extension, force).potential_j;
    let energy = 0.5 * mass * v0 * v0 + potential(x0);
    let direction = if v0 > 0.0 { 1.0 } else { -1.0 };
    let mut inside = x0;
    let mut reach = (v0 / (k / mass).sqrt()).abs();
    if !(reach > 0.0) {
        return x0;
    }
    let mut outside = x0 + direction * reach;
    let mut widen = 0;
    while potential(outside) < energy && widen < 200 {
        inside = outside;
        reach *= 2.0;
        outside = x0 + direction * reach;
        widen += 1;
    }
    let mut halve = 0;
    while halve < 200 {
        let middle = inside + 0.5 * (outside - inside);
        if middle == inside || middle == outside {
            break;
        }
        if potential(middle) < energy {
            inside = middle;
        } else {
            outside = middle;
        }
        halve += 1;
    }
    outside
}

fn refresh(cell: &Ti64LatticeCell, state: &mut Ti64LatticeState, applied_torque_nm: [f64; 3]) {
    let k = cell.stiffness_n_m();
    let mut energy = kinetic_j(cell.mass_kg(), state.velocity_m_s);
    let mut i = 0;
    while i < 3 {
        let law = strut_law(
            k[i],
            cell.hardening_n_m3[i],
            cell.area_m2[i],
            state.position_m[i],
            thermal_extension_m(cell.length_m[i], state.temperature_k),
            state.applied_force_n[i],
        );
        state.tension_n[i] = law.tension_n;
        state.ligament_stress_pa[i] = law.stress_pa;
        energy += law.potential_j;
        i += 1;
    }
    state.energy_j = energy;
    state.von_mises_mpa = ligament_von_mises_pa(state.ligament_stress_pa) / PA_PER_MPA;
    state.torque_nm = nodal_torque_nm(
        cell.eccentricity_m,
        state.tension_n,
        state.applied_force_n,
        applied_torque_nm,
    );
}

fn publish(state: &Ti64LatticeState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    let mut i = 0;
    while i < 3 {
        frame.pos[i] = state.position_m[i] as f32;
        frame.vel[i] = state.velocity_m_s[i] as f32;
        i += 1;
    }
    frame.force_torque = dominant_tension_n(state.tension_n) as f32;
    frame.residual = state.residual_conservation_error as f32;
    // The card has no advisory condition. Yield and a residual past the limit are refusals.
    frame.flags = 0;
}

struct StrutLaw {
    tension_n: f64,
    force_on_node_n: f64,
    potential_j: f64,
    stress_pa: f64,
}

/// `U = k/2 (x − α ΔT L)² + β/4 x⁴ − F x`. The nodal force is `−dU/dx`.
fn strut_law(k: f64, beta: f64, area: f64, x: f64, extension_th: f64, applied: f64) -> StrutLaw {
    let stretch = x - extension_th;
    let cubic = beta * x * x * x;
    let tension = k * stretch + cubic;
    let x2 = x * x;
    StrutLaw {
        tension_n: tension,
        force_on_node_n: -tension + applied,
        potential_j: 0.5 * k * stretch * stretch + 0.25 * beta * x2 * x2 - applied * x,
        stress_pa: tension / area,
    }
}

fn mechanical_energy(cell: &Ti64LatticeCell, state: &Ti64LatticeState) -> f64 {
    let k = cell.stiffness_n_m();
    let mut energy = kinetic_j(cell.mass_kg(), state.velocity_m_s);
    let mut i = 0;
    while i < 3 {
        energy += strut_law(
            k[i],
            cell.hardening_n_m3[i],
            cell.area_m2[i],
            state.position_m[i],
            thermal_extension_m(cell.length_m[i], state.temperature_k),
            state.applied_force_n[i],
        )
        .potential_j;
        i += 1;
    }
    energy
}

fn kinetic_j(mass_kg: f64, velocity_m_s: [f64; 3]) -> f64 {
    0.5 * mass_kg
        * (velocity_m_s[0] * velocity_m_s[0]
            + velocity_m_s[1] * velocity_m_s[1]
            + velocity_m_s[2] * velocity_m_s[2])
}

fn rates(
    cell: &Ti64LatticeCell,
    y: [f64; 6],
    applied_force_n: [f64; 3],
    temperature_k: f64,
) -> Option<[f64; 6]> {
    let k = cell.stiffness_n_m();
    let mass = cell.mass_kg();
    if !(mass.is_finite() && mass > 0.0) {
        return None;
    }
    let mut out = [0.0; 6];
    let mut i = 0;
    while i < 3 {
        let law = strut_law(
            k[i],
            cell.hardening_n_m3[i],
            cell.area_m2[i],
            y[i],
            thermal_extension_m(cell.length_m[i], temperature_k),
            applied_force_n[i],
        );
        out[i] = y[i + 3];
        out[i + 3] = law.force_on_node_n / mass;
        i += 1;
    }
    if finite6(out) { Some(out) } else { None }
}

fn rk4(
    cell: &Ti64LatticeCell,
    y: [f64; 6],
    dt: f64,
    applied_force_n: [f64; 3],
    temperature_k: f64,
) -> Option<[f64; 6]> {
    let k1 = rates(cell, y, applied_force_n, temperature_k)?;
    let k2 = rates(cell, axpy(y, dt * 0.5, k1), applied_force_n, temperature_k)?;
    let k3 = rates(cell, axpy(y, dt * 0.5, k2), applied_force_n, temperature_k)?;
    let k4 = rates(cell, axpy(y, dt, k3), applied_force_n, temperature_k)?;
    let mut next = y;
    let mut i = 0;
    while i < 6 {
        next[i] += (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        i += 1;
    }
    if finite6(next) { Some(next) } else { None }
}

fn axpy(y: [f64; 6], scale: f64, k: [f64; 6]) -> [f64; 6] {
    [
        y[0] + scale * k[0],
        y[1] + scale * k[1],
        y[2] + scale * k[2],
        y[3] + scale * k[3],
        y[4] + scale * k[4],
        y[5] + scale * k[5],
    ]
}

fn pack_phase(state: &Ti64LatticeState) -> [f64; 6] {
    [
        state.position_m[0],
        state.position_m[1],
        state.position_m[2],
        state.velocity_m_s[0],
        state.velocity_m_s[1],
        state.velocity_m_s[2],
    ]
}

fn dominant_tension_n(tension_n: [f64; 3]) -> f64 {
    let mut best = tension_n[0];
    let mut mag = tension_n[0].abs();
    if tension_n[1].abs() > mag {
        best = tension_n[1];
        mag = tension_n[1].abs();
    }
    if tension_n[2].abs() > mag {
        best = tension_n[2];
    }
    best
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn finite6(x: [f64; 6]) -> bool {
    let mut i = 0;
    while i < 6 {
        if !x[i].is_finite() {
            return false;
        }
        i += 1;
    }
    true
}

fn state_is_finite(state: &Ti64LatticeState) -> bool {
    state.t_s.is_finite()
        && finite3(state.position_m)
        && finite3(state.velocity_m_s)
        && finite3(state.tension_n)
        && finite3(state.torque_nm)
        && finite3(state.applied_force_n)
        && state.temperature_k.is_finite()
        && finite3(state.ligament_stress_pa)
        && state.von_mises_mpa.is_finite()
        && state.energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.boundary_work_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
}

/// Explicit Euler, `x ← x + Δt v`, `v ← v + Δt a`.
/// Harmonic energy grows by exactly `1 + (ω Δt)²` per step. The gate rejects that drift.
#[cfg(test)]
fn step_euler(
    cell: &Ti64LatticeCell,
    state: &mut Ti64LatticeState,
    frame: &mut LastStateFrame64,
    load: Ti64Load,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(cell, state, load, dt_s)?;
    let predicted = match explicit_euler(
        cell,
        pack_phase(state),
        dt_s,
        load.force_n,
        load.temperature_k,
    ) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(cell, state, frame, load, dt_s, predicted)
}

#[cfg(test)]
fn explicit_euler(
    cell: &Ti64LatticeCell,
    y: [f64; 6],
    dt: f64,
    applied_force_n: [f64; 3],
    temperature_k: f64,
) -> Option<[f64; 6]> {
    let k1 = rates(cell, y, applied_force_n, temperature_k)?;
    let next = axpy(y, dt, k1);
    if finite6(next) { Some(next) } else { None }
}

/// Enter `extra_j` of boundary work that no boundary did, then book a faithful RK4 step.
#[cfg(test)]
fn step_bias_work(
    cell: &Ti64LatticeCell,
    state: &mut Ti64LatticeState,
    frame: &mut LastStateFrame64,
    load: Ti64Load,
    dt_s: f64,
    extra_j: f64,
) -> Result<(), StepReject> {
    validate(cell, state, load, dt_s)?;
    let predicted = match rk4(
        cell,
        pack_phase(state),
        dt_s,
        load.force_n,
        load.temperature_k,
    ) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    let honest = *state;
    state.boundary_work_j += extra_j;
    let falsified = *state;
    let got = commit(cell, state, frame, load, dt_s, predicted);
    if got.is_err() {
        // `commit` rolled back to the falsified entry. Take the false entry out as well.
        ::core::assert!(*state == falsified, "the refusal did not roll back");
        *state = honest;
    }
    got
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::assert_ne;

    use super::{
        BETA_TRANSUS_K, CARD_TEMPERATURE_MIN_K, DENSITY_KG_M3, GATE_PHASE_STEP_RAD,
        MASS_FRACTION_AL, MASS_FRACTION_TI, MASS_FRACTION_V, POISSON_RATIO,
        REFERENCE_TEMPERATURE_K, RESIDUAL_CONSERVATION_LIMIT, StepReject, THERMAL_EXPANSION_PER_K,
        Ti64LatticeCell, Ti64LatticeState, Ti64Load, Ti64Start, YIELD_STRENGTH_MPA,
        YOUNGS_MODULUS_PA,
        cauchy_von_mises_pa, ligament_von_mises_pa, nodal_torque_nm, shear_modulus_pa, step,
        step_bias_work, step_euler, thermal_extension_m, uniaxial_yield_strain,
    };
    use crate::frame::LastStateFrame64;

    const STEPS: u32 = 10_000;
    const EARLY_STEPS: u32 = 1_000;

    fn column(hardening_fraction: f64, eccentricity_m: [f64; 3]) -> Ti64LatticeCell {
        let length = 0.02;
        let area = 4.0e-6;
        let k = YOUNGS_MODULUS_PA * area / length;
        let dy = uniaxial_yield_strain() * length;
        let beta = hardening_fraction * k / (dy * dy);
        Ti64LatticeCell::try_new([length; 3], [area; 3], [beta; 3], eccentricity_m).unwrap()
    }

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

    fn state_bits(state: &Ti64LatticeState) -> [u64; 26] {
        let mut bits = [0_u64; 26];
        let mut i = 0;
        while i < 3 {
            bits[i] = state.position_m[i].to_bits();
            bits[3 + i] = state.velocity_m_s[i].to_bits();
            bits[6 + i] = state.tension_n[i].to_bits();
            bits[9 + i] = state.torque_nm[i].to_bits();
            bits[12 + i] = state.applied_force_n[i].to_bits();
            bits[15 + i] = state.ligament_stress_pa[i].to_bits();
            i += 1;
        }
        bits[18] = state.t_s.to_bits();
        bits[19] = state.temperature_k.to_bits();
        bits[20] = state.von_mises_mpa.to_bits();
        bits[21] = state.energy_j.to_bits();
        bits[22] = state.energy_anchor_j.to_bits();
        bits[23] = state.boundary_work_j.to_bits();
        bits[24] = state.energy_defect_j.to_bits();
        bits[25] = state.residual_conservation_error.to_bits();
        bits
    }

    /// Step, and require this refusal with the state and the frame bytes as they were.
    fn refused_whole(
        cell: &Ti64LatticeCell,
        state: &mut Ti64LatticeState,
        frame: &mut LastStateFrame64,
        load: Ti64Load,
        dt_s: f64,
        expect: StepReject,
    ) {
        let held = state_bits(state);
        let held_frame = frame.to_bytes();
        let got = step(cell, state, frame, load, dt_s);
        assert_eq!(got, Err(expect));
        assert_eq!(state_bits(state), held);
        assert_eq!(frame.to_bytes(), held_frame);
    }

    #[test]
    fn card_constants_match_annealed_ti64() {
        let column = Ti64LatticeCell::annealed_column();
        assert_eq!(column.length_m, [0.02; 3]);
        assert_eq!(column.area_m2, [4.0e-6; 3]);
        assert_eq!(column.hardening_n_m3, [0.0; 3]);
        assert!(column.yield_energy_scale_j() > 0.0);
        assert_eq!(YIELD_STRENGTH_MPA, 880.0);
        assert_eq!(DENSITY_KG_M3, 4430.0);
        assert!(DENSITY_KG_M3 / 1000.0 < 4.50);
        assert_eq!(THERMAL_EXPANSION_PER_K, 8.6e-6);
        assert!((MASS_FRACTION_TI + MASS_FRACTION_AL + MASS_FRACTION_V - 1.0).abs() < 1e-15);
        assert_eq!(MASS_FRACTION_TI, 0.90);
        assert_eq!(MASS_FRACTION_AL, 0.06);
        assert_eq!(MASS_FRACTION_V, 0.04);
        let g = shear_modulus_pa();
        let expected = YOUNGS_MODULUS_PA / (2.0 * (1.0 + POISSON_RATIO));
        assert!((g - expected).abs() <= 1e-12 * expected);
        assert!(g > 40.0e9 && g < 46.0e9);
        let hydrostatic = [-900.0e6, -900.0e6, -900.0e6];
        assert_eq!(cauchy_von_mises_pa(hydrostatic), 0.0);
        assert!((ligament_von_mises_pa(hydrostatic) - 900.0e6).abs() < 1.0);
        let uniaxial = [880.0e6, 0.0, 0.0];
        assert!((cauchy_von_mises_pa(uniaxial) - 880.0e6).abs() < 1e-6);
        assert!((ligament_von_mises_pa(uniaxial) - 880.0e6).abs() < 1e-6);
    }

    #[test]
    fn copy_types_fit_on_the_stack() {
        assert_copy::<Ti64LatticeCell>();
        assert_copy::<Ti64LatticeState>();
        assert_copy::<Ti64Load>();
        assert_copy::<StepReject>();
        assert!(core::mem::size_of::<Ti64LatticeState>() < 512);
        assert!(core::mem::size_of::<Ti64LatticeCell>() < 256);
    }

    #[test]
    fn rejected_steps_do_not_write_state_or_frame() {
        let cell = column(0.0, [0.0; 3]);
        let dt = cell.gate_dt_s().unwrap();
        let rest = Ti64LatticeState::try_rest(&cell).unwrap();

        let cases = [
            (
                Ti64Load {
                    force_n: [f64::NAN, 0.0, 0.0],
                    torque_nm: [0.0; 3],
                    temperature_k: REFERENCE_TEMPERATURE_K,
                },
                dt,
                StepReject::NonFiniteInput,
            ),
            (
                Ti64Load {
                    force_n: [0.0; 3],
                    torque_nm: [0.0, f64::INFINITY, 0.0],
                    temperature_k: REFERENCE_TEMPERATURE_K,
                },
                dt,
                StepReject::NonFiniteInput,
            ),
            (Ti64Load::default(), f64::NAN, StepReject::NonFiniteInput),
            (Ti64Load::default(), 0.0, StepReject::NonPositiveDt),
            (Ti64Load::default(), -1.0e-6, StepReject::NonPositiveDt),
            (
                Ti64Load {
                    force_n: [0.0; 3],
                    torque_nm: [0.0; 3],
                    temperature_k: CARD_TEMPERATURE_MIN_K - 1.0,
                },
                dt,
                StepReject::TemperatureOutsideCard,
            ),
            (
                Ti64Load {
                    force_n: [0.0; 3],
                    torque_nm: [0.0; 3],
                    temperature_k: BETA_TRANSUS_K + 1.0,
                },
                dt,
                StepReject::TemperatureOutsideCard,
            ),
            (
                Ti64Load {
                    force_n: [1.0e308, 0.0, 0.0],
                    torque_nm: [0.0; 3],
                    temperature_k: REFERENCE_TEMPERATURE_K,
                },
                dt,
                StepReject::NonFiniteState,
            ),
        ];

        let mut i = 0;
        while i < cases.len() {
            let (load, step_dt, expect) = cases[i];
            let mut state = rest;
            let mut frame = blank_frame();
            let got = step(&cell, &mut state, &mut frame, load, step_dt);
            assert_eq!(got, Err(expect), "case {i}");
            assert_eq!(state, rest, "case {i} wrote the state");
            assert_eq!(frame.t, -1.0);
            assert_eq!(frame.flags, u64::MAX);
            assert_eq!(frame.force_torque, 42.0);
            proof_intact(&frame);
            i += 1;
        }

        let mut broken = cell;
        broken.area_m2 = [0.0; 3];
        let mut state = rest;
        let mut frame = blank_frame();
        assert_eq!(
            step(&broken, &mut state, &mut frame, Ti64Load::default(), dt),
            Err(StepReject::InvalidCell)
        );
        assert_eq!(state, rest);
        proof_intact(&frame);
    }

    /// Explicit Euler grows a harmonic ligament's energy by `1 + (ω Δt)²` a
    /// step. Its steps land, on the closed form, while the residual is inside
    /// the limit, and the first step that would leave it is refused and
    /// rolled back. From 5 percent of the yield extension that is the third
    /// step. From a tenth of a percent, thousands of steps land first.
    #[test]
    fn explicit_euler_multiplies_energy_by_one_plus_phase_squared() {
        let cell = column(0.0, [0.0; 3]);
        let dt = cell.gate_dt_s().unwrap();
        let k = cell.stiffness_n_m()[0];
        let mass = cell.mass_kg();
        let w2 = k / mass;
        assert!((w2.sqrt() * dt - GATE_PHASE_STEP_RAD).abs() < 1e-12);
        let starts = [(0.05, 3_u32), (0.001, EARLY_STEPS)];
        let mut case = 0;
        while case < starts.len() {
            let (yield_fraction, checkpoint) = starts[case];
            let x0 = yield_fraction * uniaxial_yield_strain() * cell.length_m[0];
            let mut state = Ti64LatticeState::try_phase(&cell, [x0, 0.0, 0.0], [0.0; 3]).unwrap();
            let e0 = state.energy_anchor_j;
            let spring = 0.5 * k * x0 * x0;
            assert!((e0 - spring).abs() <= 1e-12 * spring);

            // The closed form names the first step whose residual is past the limit.
            let factor = 1.0 + w2 * dt * dt;
            let mut growth = factor;
            let mut refused_step = 1_u32;
            while e0 * (growth - 1.0) / cell.yield_energy_scale_j() <= RESIDUAL_CONSERVATION_LIMIT {
                growth *= factor;
                refused_step += 1;
            }
            assert!(refused_step >= checkpoint, "refused step {refused_step}");

            let mut frame = blank_frame();
            let mut landed = 0_u32;
            loop {
                let held = state_bits(&state);
                let held_frame = frame.to_bytes();
                match step_euler(&cell, &mut state, &mut frame, Ti64Load::default(), dt) {
                    Ok(()) => {
                        landed += 1;
                        assert!(landed < refused_step, "euler step {landed} landed");
                        assert!(state.energy_defect_j > 0.0, "explicit Euler pumped energy");
                        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                        assert_eq!(frame.flags, 0);
                        if landed == checkpoint || landed + 1 == refused_step {
                            assert_closed_form(&state, e0, w2, dt, landed);
                        }
                    }
                    Err(reject) => {
                        assert_eq!(reject, StepReject::SpontaneousGeneration);
                        assert_eq!(state_bits(&state), held, "the refusal wrote the state");
                        assert_eq!(frame.to_bytes(), held_frame, "the refusal wrote the frame");
                        break;
                    }
                }
            }
            assert_eq!(landed + 1, refused_step);
            if case == 0 {
                assert_eq!(refused_step, 3);
            } else {
                assert!(
                    refused_step > 3 * EARLY_STEPS,
                    "refused step {refused_step}"
                );
            }
            // The refusal repeats and the state stays where the last landed step left it.
            let got = step_euler(&cell, &mut state, &mut frame, Ti64Load::default(), dt);
            assert_eq!(got, Err(StepReject::SpontaneousGeneration));
            proof_intact(&frame);
            case += 1;
        }
    }

    fn assert_closed_form(state: &Ti64LatticeState, e0: f64, w2: f64, dt: f64, steps: u32) {
        let factor = 1.0 + w2 * dt * dt;
        let mut growth = 1.0;
        let mut i = 0_u32;
        while i < steps {
            growth *= factor;
            i += 1;
        }
        let expected = e0 * (growth - 1.0);
        let rel = (state.energy_defect_j - expected).abs() / expected.abs();
        assert!(
            rel < 1e-8,
            "steps {steps} defect {} expected {expected} rel {rel}",
            state.energy_defect_j
        );
    }

    #[test]
    fn rk4_cold_oscillator_stays_inside_the_gate() {
        let cell = column(0.0, [0.0; 3]);
        let dt = cell.gate_dt_s().unwrap();
        let w = cell.max_natural_frequency_rad_s();
        let k = cell.stiffness_n_m()[0];
        let x0 = 0.05 * uniaxial_yield_strain() * cell.length_m[0];
        let mut state = Ti64LatticeState::try_phase(&cell, [x0, 0.0, 0.0], [0.0; 3]).unwrap();
        let mut frame = blank_frame();
        let mut max_residual = 0.0;
        let mut n = 0_u32;
        while n < STEPS {
            step(&cell, &mut state, &mut frame, Ti64Load::default(), dt).unwrap();
            n += 1;
            let t = state.t_s;
            let x_exact = x0 * (w * t).cos();
            let v_exact = -x0 * w * (w * t).sin();
            assert!(
                (state.position_m[0] - x_exact).abs() <= 1e-5 * x0,
                "step {n} x {} exact {x_exact}",
                state.position_m[0]
            );
            assert!(
                (state.velocity_m_s[0] - v_exact).abs() <= 1e-5 * x0 * w,
                "step {n} v {} exact {v_exact}",
                state.velocity_m_s[0]
            );
            assert_eq!(state.position_m[1].to_bits(), 0);
            assert_eq!(state.position_m[2].to_bits(), 0);
            assert!((state.tension_n[0] - k * state.position_m[0]).abs() <= 1e-8 * k * x0);
            if state.residual_conservation_error > max_residual {
                max_residual = state.residual_conservation_error;
            }
            assert!(
                state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT,
                "step {n} residual {}",
                state.residual_conservation_error
            );
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            if n == EARLY_STEPS || n == STEPS {
                assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
                assert_eq!(
                    frame.pos[0].to_bits(),
                    (state.position_m[0] as f32).to_bits()
                );
                assert_eq!(
                    frame.vel[0].to_bits(),
                    (state.velocity_m_s[0] as f32).to_bits()
                );
                assert_eq!(frame.flags, 0);
                proof_intact(&frame);
            }
        }
        assert!((state.t_s - f64::from(STEPS) * dt).abs() < 1e-9);
        assert!(max_residual <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(
            max_residual < 1.0e-9,
            "cold RK4 residual drifted to {max_residual}"
        );
    }

    #[test]
    fn constant_force_and_uniform_heat_match_the_shifted_oscillator() {
        let cell = column(0.0, [0.0; 3]);
        let dt = cell.gate_dt_s().unwrap();
        let w = cell.max_natural_frequency_rad_s();
        let k = cell.stiffness_n_m()[0];
        let f = 0.4 * YIELD_STRENGTH_MPA * pa_per_mpa() * cell.area_m2[0];
        let xe = f / k;
        let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
        let mut frame = blank_frame();
        let load = Ti64Load {
            force_n: [f, 0.0, 0.0],
            torque_nm: [0.0; 3],
            temperature_k: REFERENCE_TEMPERATURE_K,
        };
        let mut n = 0_u32;
        while n < STEPS {
            step(&cell, &mut state, &mut frame, load, dt).unwrap();
            n += 1;
            assert_shifted(
                state.position_m[0],
                state.velocity_m_s[0],
                xe,
                w,
                state.t_s,
                n,
            );
            assert_eq!(state.position_m[1].to_bits(), 0);
            assert_eq!(state.position_m[2].to_bits(), 0);
            assert!(
                state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT,
                "force step {n}"
            );
            assert_eq!(state.boundary_work_j, 0.0);
        }
        proof_intact(&frame);

        drive_heat(&cell, dt, w, k, 0.50, false);
        drive_heat(&cell, dt, w, k, 1.05, true);
    }

    fn pa_per_mpa() -> f64 {
        1.0e6
    }

    fn drive_heat(
        cell: &Ti64LatticeCell,
        dt: f64,
        w: f64,
        k: f64,
        yield_fraction: f64,
        expects_yield: bool,
    ) {
        let dtemp = yield_fraction * YIELD_STRENGTH_MPA * pa_per_mpa()
            / (YOUNGS_MODULUS_PA * THERMAL_EXPANSION_PER_K);
        let temperature = REFERENCE_TEMPERATURE_K + dtemp;
        assert!(
            temperature < BETA_TRANSUS_K,
            "heat fraction {yield_fraction} left the card at {temperature}"
        );
        assert!(temperature > CARD_TEMPERATURE_MIN_K);
        let dth = thermal_extension_m(cell.length_m[0], temperature);
        let booked = 3.0 * 0.5 * k * dth * dth;
        let mut state = Ti64LatticeState::try_rest(cell).unwrap();
        let mut frame = blank_frame();
        let load = Ti64Load {
            force_n: [0.0; 3],
            torque_nm: [0.0; 3],
            temperature_k: temperature,
        };
        if expects_yield {
            // The node has not moved when the heat is applied, so every
            // ligament is past yield at that instant. The first step is refused.
            assert!(booked > cell.yield_energy_scale_j());
            let held = state_bits(&state);
            let held_frame = frame.to_bytes();
            let got = step(cell, &mut state, &mut frame, load, dt);
            assert_eq!(got, Err(StepReject::YieldReached));
            assert_eq!(state_bits(&state), held);
            assert_eq!(frame.to_bytes(), held_frame);
            proof_intact(&frame);
            return;
        }
        let mut n = 0_u32;
        while n < STEPS {
            step(cell, &mut state, &mut frame, load, dt).unwrap();
            n += 1;
            assert_shifted(
                state.position_m[0],
                state.velocity_m_s[0],
                dth,
                w,
                state.t_s,
                n,
            );
            assert_eq!(state.position_m[0].to_bits(), state.position_m[1].to_bits());
            assert_eq!(state.position_m[0].to_bits(), state.position_m[2].to_bits());
            assert_eq!(cauchy_von_mises_pa(state.ligament_stress_pa), 0.0);
            assert!((state.boundary_work_j - booked).abs() <= 1e-9 * booked.abs());
            assert!(
                state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT,
                "heat {yield_fraction} step {n} residual {}",
                state.residual_conservation_error
            );
            assert!(state.von_mises_mpa <= YIELD_STRENGTH_MPA, "heat {yield_fraction} step {n} yielded");
            assert_eq!(frame.flags, 0);
        }
        proof_intact(&frame);
    }

    fn assert_shifted(x: f64, v: f64, xe: f64, w: f64, t: f64, step_index: u32) {
        let x_exact = xe * (1.0 - (w * t).cos());
        let v_exact = xe * w * (w * t).sin();
        let scale = xe.abs().max(1e-12);
        assert!(
            (x - x_exact).abs() <= 1e-5 * scale,
            "step {step_index} x {x} exact {x_exact}"
        );
        assert!(
            (v - v_exact).abs() <= 1e-5 * scale * w,
            "step {step_index} v {v} exact {v_exact}"
        );
    }

    #[test]
    fn rk4_extreme_nonlinear_loads_stay_inside_the_gate() {
        let cell = column(0.25, [0.0, 1.0e-4, 0.0]);
        let dt = cell.gate_dt_s().unwrap();
        let f_yield = YIELD_STRENGTH_MPA * pa_per_mpa() * cell.area_m2[0];
        let mut kind = 0_u32;
        while kind < 4 {
            let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
            let mut frame = blank_frame();
            let mut max_residual = 0.0;
            let mut peak_mpa = 0.0;
            let mut landed = 0_u32;
            let mut refused_for_yield = 0_u32;
            let mut n = 0_u32;
            while n < STEPS {
                let load = extreme_load(kind, n, f_yield);
                let held = state_bits(&state);
                let held_frame = frame.to_bytes();
                let got = step(&cell, &mut state, &mut frame, load, dt);
                if got == Err(StepReject::YieldReached) {
                    // This load would take a ligament past yield. The step is
                    // refused whole and the run goes on from where it stood.
                    assert_eq!(state_bits(&state), held, "kind {kind} step {n}");
                    assert_eq!(frame.to_bytes(), held_frame, "kind {kind} step {n}");
                    refused_for_yield += 1;
                    n += 1;
                    continue;
                }
                assert!(got.is_ok(), "kind {kind} step {n} rejected {got:?}");
                landed += 1;
                let residual = state.residual_conservation_error;
                if residual > max_residual {
                    max_residual = residual;
                }
                if state.von_mises_mpa > peak_mpa {
                    peak_mpa = state.von_mises_mpa;
                }
                assert!(
                    residual <= RESIDUAL_CONSERVATION_LIMIT,
                    "kind {kind} step {n} residual {residual}"
                );
                assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
                assert_eq!(frame.residual.to_bits(), (residual as f32).to_bits());
                assert!(state.von_mises_mpa <= YIELD_STRENGTH_MPA, "kind {kind} step {n} landed past yield");
                assert_eq!(frame.flags, 0, "kind {kind} step {n}");
                if n == EARLY_STEPS {
                    assert!(residual <= RESIDUAL_CONSERVATION_LIMIT);
                }
                n += 1;
            }
            proof_intact(&frame);
            assert!(state.energy_j.is_finite());
            assert_eq!(landed + refused_for_yield, STEPS);
            assert!((state.t_s - f64::from(landed) * dt).abs() < 1e-9);
            // The first three load sets peak near 820, 800 and 260 MPa and
            // never reach yield. The fourth heats to 1193 K; about two thirds
            // of its steps are refused, and what lands stays elastic.
            if kind < 3 {
                assert_eq!(refused_for_yield, 0, "kind {kind}");
            } else {
                assert!(refused_for_yield > 1_000, "kind {kind}");
                assert!(landed > 1_000, "kind {kind} landed {landed}");
            }
            assert!(
                peak_mpa <= YIELD_STRENGTH_MPA,
                "kind {kind} landed at {peak_mpa} MPa"
            );
            assert!(max_residual < 1.0e-6, "kind {kind} residual {max_residual}");
            assert!(
                max_residual > 0.0,
                "kind {kind} residual was identically zero"
            );
            assert!(
                peak_mpa > 100.0,
                "kind {kind} peak ligament stress {peak_mpa} MPa"
            );
            kind += 1;
        }
    }

    fn extreme_load(kind: u32, step_index: u32, f_yield: f64) -> Ti64Load {
        let n = f64::from(step_index);
        let slow = (n * 0.0007).sin();
        if kind == 0 {
            Ti64Load {
                force_n: [
                    0.35 * f_yield * slow,
                    0.15 * f_yield * (n * 0.0011).sin(),
                    -0.10 * f_yield * (n * 0.0005).sin(),
                ],
                torque_nm: [0.02 * slow, 0.0, -0.01 * slow],
                temperature_k: REFERENCE_TEMPERATURE_K + 350.0 * (1.0 + (n * 0.0004).sin()),
            }
        } else if kind == 1 {
            let sign = if (step_index / 25) % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            Ti64Load {
                force_n: [sign * 0.50 * f_yield, sign * 0.20 * f_yield, 0.0],
                torque_nm: [0.0; 3],
                temperature_k: 900.0,
            }
        } else if kind == 2 {
            let spike = if step_index % 400 == 0 {
                0.75
            } else {
                0.05 * slow
            };
            Ti64Load {
                force_n: [spike * f_yield, 0.0, 0.0],
                torque_nm: [0.0; 3],
                temperature_k: REFERENCE_TEMPERATURE_K + 200.0,
            }
        } else {
            let kick = if step_index % 250 == 0 { 0.55 } else { 0.0 };
            Ti64Load {
                force_n: [
                    (0.25 * slow + kick) * f_yield,
                    0.20 * f_yield * (n * 0.0013).cos(),
                    -0.15 * f_yield * (n * 0.0009).sin(),
                ],
                torque_nm: [0.0, kick, 0.0],
                temperature_k: REFERENCE_TEMPERATURE_K + 450.0 * (1.0 + (n * 0.0003).sin()),
            }
        }
    }

    #[test]
    fn torque_is_reported_and_does_not_enter_the_translation() {
        let eccentricity = [0.0, 1.0e-3, 0.0];
        let cell = column(0.0, eccentricity);
        let dt = cell.gate_dt_s().unwrap();
        let force = 120.0;
        let mut with_torque = Ti64LatticeState::try_rest(&cell).unwrap();
        let mut without = with_torque;
        let mut frame_a = blank_frame();
        let mut frame_b = blank_frame();
        let mut n = 0_u32;
        while n < EARLY_STEPS {
            let phase = f64::from(n) * 0.01;
            let applied = [force * phase.sin(), 0.0, 0.0];
            let torque = [0.0, 0.0, 4.0 * phase.cos()];
            step(
                &cell,
                &mut with_torque,
                &mut frame_a,
                Ti64Load {
                    force_n: applied,
                    torque_nm: torque,
                    temperature_k: REFERENCE_TEMPERATURE_K,
                },
                dt,
            )
            .unwrap();
            step(
                &cell,
                &mut without,
                &mut frame_b,
                Ti64Load {
                    force_n: applied,
                    torque_nm: [0.0; 3],
                    temperature_k: REFERENCE_TEMPERATURE_K,
                },
                dt,
            )
            .unwrap();
            assert_eq!(
                with_torque.position_m[0].to_bits(),
                without.position_m[0].to_bits()
            );
            assert_eq!(
                with_torque.position_m[1].to_bits(),
                without.position_m[1].to_bits()
            );
            assert_eq!(
                with_torque.position_m[2].to_bits(),
                without.position_m[2].to_bits()
            );
            assert_eq!(
                with_torque.velocity_m_s[0].to_bits(),
                without.velocity_m_s[0].to_bits()
            );
            assert_eq!(
                with_torque.energy_defect_j.to_bits(),
                without.energy_defect_j.to_bits()
            );
            let expect = nodal_torque_nm(eccentricity, with_torque.tension_n, applied, torque);
            assert_eq!(with_torque.torque_nm[0].to_bits(), expect[0].to_bits());
            assert_eq!(with_torque.torque_nm[1].to_bits(), expect[1].to_bits());
            assert_eq!(with_torque.torque_nm[2].to_bits(), expect[2].to_bits());
            assert_ne!(
                with_torque.torque_nm[2].to_bits(),
                without.torque_nm[2].to_bits()
            );
            n += 1;
        }
        // e × F with e = (0, 1e-3, 0) and F = (10, 0, 0) is (0, 0, -0.01) before the applied couple.
        let moment = nodal_torque_nm(eccentricity, [0.0; 3], [10.0, 0.0, 0.0], [0.0; 3]);
        assert!((moment[0]).abs() < 1e-15);
        assert!((moment[1]).abs() < 1e-15);
        assert!((moment[2] + 0.01).abs() < 1e-15);
        proof_intact(&frame_a);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::Ti64LatticeCell::annealed_column();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The column at rest, pulled along its first ligament at four tenths of yield.
        let mut state = super::Ti64LatticeState::try_rest(&card).unwrap();
        let load = super::Ti64Load {
            force_n: [
                0.4 * super::YIELD_STRENGTH_MPA * 1.0e6 * card.area_m2[0],
                0.0,
                0.0,
            ],
            torque_nm: [0.0; 3],
            temperature_k: super::REFERENCE_TEMPERATURE_K,
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
        assert!(state.position_m[0] != before.position_m[0]);
        assert!(state.velocity_m_s[0] != before.velocity_m_s[0]);
        assert!(state.energy_j != before.energy_j);
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);

        // A thousand more steps at the floor land: the ledger holds an
        // honest step this small.
        let mut taken = 0;
        while taken < 1_000 {
            super::step(&card, &mut state, &mut frame, load, floor_s).unwrap();
            taken += 1;
        }
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);

        // At the largest time a run reaches, the clock still moves by a
        // floor step, to within a thousandth of it.
        let late_s = 1.0e6 * ceiling_s;
        let moved_s = (late_s + floor_s) - late_s;
        assert!(
            (moved_s - floor_s).abs() <= 1.0e-3 * floor_s,
            "the clock moved {moved_s} for a step of {floor_s}"
        );
    }

    /// The largest displacement of the first ligament that is not past yield,
    /// by the card's own stress.
    fn yield_edge_m(cell: &Ti64LatticeCell) -> f64 {
        let past = |x: f64| {
            Ti64LatticeState::try_phase(cell, [x, 0.0, 0.0], [0.0; 3])
                .unwrap()
                .von_mises_mpa
                > YIELD_STRENGTH_MPA
        };
        let mut edge = uniaxial_yield_strain() * cell.length_m[0];
        while past(edge) {
            edge = edge.next_down();
        }
        while !past(edge.next_up()) {
            edge = edge.next_up();
        }
        edge
    }

    /// Yield is tested on the state a step starts from. A start at twice the
    /// yield stress, moving back at 1,500 m/s, used to land at 593 MPa.
    #[test]
    fn a_start_past_yield_is_refused_and_the_last_number_inside_lands() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = cell.gate_dt_s().unwrap();
        let yield_m = uniaxial_yield_strain() * cell.length_m[0];
        let edge = yield_edge_m(&cell);
        assert!((edge - yield_m).abs() <= 4.0 * (yield_m.next_up() - yield_m));
        let rest = Ti64Load::default();

        let mut sign = 1.0;
        while sign >= -1.0 {
            // At rest on the last number inside: the node swings back and the step lands.
            let mut state =
                Ti64LatticeState::try_phase(&cell, [sign * edge, 0.0, 0.0], [0.0; 3]).unwrap();
            assert!(state.von_mises_mpa <= YIELD_STRENGTH_MPA);
            let mut frame = blank_frame();
            step(&cell, &mut state, &mut frame, rest, dt).unwrap();
            assert!(state.von_mises_mpa < YIELD_STRENGTH_MPA);
            assert!(state.von_mises_mpa <= YIELD_STRENGTH_MPA);
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            assert_eq!(frame.flags, 0);
            proof_intact(&frame);

            // The next representable displacement is past yield: no step is taken from it.
            let mut over =
                Ti64LatticeState::try_phase(&cell, [sign * edge.next_up(), 0.0, 0.0], [0.0; 3])
                    .unwrap();
            assert!(over.von_mises_mpa > YIELD_STRENGTH_MPA);
            let mut frame = blank_frame();
            refused_whole(
                &cell,
                &mut over,
                &mut frame,
                rest,
                dt,
                StepReject::YieldReached,
            );
            assert_eq!(frame.flags, u64::MAX);
            proof_intact(&frame);
            sign -= 2.0;
        }

        // Starts past yield that one step would have carried back inside.
        // Multiple of the yield extension, and speed along the first ligament.
        let starts = [
            (880.09 / 880.0, 0.0),
            (2.0, -1_500.0),
            (10.0, -12_000.0),
            (-3.0, 0.0),
            (-3.0, 2_000.0),
        ];
        let mut i = 0;
        while i < starts.len() {
            let (multiple, speed_m_s) = starts[i];
            let mut state = Ti64LatticeState::try_phase(
                &cell,
                [multiple * yield_m, 0.0, 0.0],
                [speed_m_s, 0.0, 0.0],
            )
            .unwrap();
            let mut frame = blank_frame();
            refused_whole(
                &cell,
                &mut state,
                &mut frame,
                rest,
                dt,
                StepReject::YieldReached,
            );
            i += 1;
        }
        // Any one ligament is enough.
        let mut state =
            Ti64LatticeState::try_phase(&cell, [0.0, 0.0, -2.0 * yield_m], [0.0; 3]).unwrap();
        let mut frame = blank_frame();
        refused_whole(
            &cell,
            &mut state,
            &mut frame,
            rest,
            dt,
            StepReject::YieldReached,
        );
        proof_intact(&frame);
    }

    /// Yield is tested at the instant the step's temperature is applied, when
    /// the node has not yet moved. Heat to 1192.4 K at rest puts every
    /// ligament at 880.08 MPa at that instant; 156 steps used to land before
    /// the far side of the swing was refused.
    #[test]
    fn heat_that_yields_the_resting_node_is_refused_on_the_first_step() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = cell.gate_dt_s().unwrap();
        let heated = |temperature_k: f64| Ti64Load {
            force_n: [0.0; 3],
            torque_nm: [0.0; 3],
            temperature_k,
        };
        let lands = |temperature_k: f64| {
            let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
            let mut frame = blank_frame();
            match step(&cell, &mut state, &mut frame, heated(temperature_k), dt) {
                Ok(()) => true,
                Err(StepReject::YieldReached) => false,
                Err(other) => std::panic!("{temperature_k} K refused with {other:?}"),
            }
        };
        let mut inside = 1192.31;
        let mut outside = 1192.4;
        assert!(lands(inside) && !lands(outside));
        while inside.next_up() < outside {
            let middle = inside + (outside - inside) * 0.5;
            if lands(middle) {
                inside = middle;
            } else {
                outside = middle;
            }
        }
        assert_eq!(inside.next_up().to_bits(), outside.to_bits());
        // The edge is where the thermal stress α ΔT E is the yield strength.
        let edge_k = REFERENCE_TEMPERATURE_K
            + YIELD_STRENGTH_MPA * pa_per_mpa() / (YOUNGS_MODULUS_PA * THERMAL_EXPANSION_PER_K);
        assert!((inside - edge_k).abs() < 1.0e-9, "edge {inside} K");
        assert!(outside < BETA_TRANSUS_K);

        for temperature_k in [outside, 1192.4, 1200.0, BETA_TRANSUS_K] {
            let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
            let mut frame = blank_frame();
            refused_whole(
                &cell,
                &mut state,
                &mut frame,
                heated(temperature_k),
                dt,
                StepReject::YieldReached,
            );
            assert_eq!(state.t_s, 0.0);
            proof_intact(&frame);
        }

        // The last temperature inside: the node swings through twice its
        // thermal growth and back, and never lands past yield.
        let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
        let mut frame = blank_frame();
        let mut peak_mpa = 0.0;
        let mut n = 0_u32;
        while n < 2_000 {
            step(&cell, &mut state, &mut frame, heated(inside), dt).unwrap();
            if state.von_mises_mpa > peak_mpa {
                peak_mpa = state.von_mises_mpa;
            }
            n += 1;
        }
        assert!(
            peak_mpa <= YIELD_STRENGTH_MPA && peak_mpa > 879.9,
            "peak {peak_mpa}"
        );

        // A change of temperature in the middle of a run is tested the same
        // way. Heated to 800 K, the node is at the far point of its swing
        // after half a period, 157 steps, stretched by its thermal growth
        // again. Cooled to the reference temperature there, it is past yield.
        let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
        let hot_k = 800.0;
        let mut n = 0_u32;
        while n < 157 {
            step(&cell, &mut state, &mut frame, heated(hot_k), dt).unwrap();
            n += 1;
        }
        let grown_m = thermal_extension_m(cell.length_m[0], hot_k);
        assert!((state.position_m[0] - 2.0 * grown_m).abs() < 0.001 * grown_m);
        assert!(state.von_mises_mpa < 500.0);
        refused_whole(
            &cell,
            &mut state,
            &mut frame,
            Ti64Load::default(),
            dt,
            StepReject::YieldReached,
        );
        step(&cell, &mut state, &mut frame, heated(hot_k), dt).unwrap();
        proof_intact(&frame);
    }

    /// Yield on the landing is a refusal with rollback, as it is on the way in.
    #[test]
    fn a_landing_past_yield_is_refused_and_the_last_speed_inside_lands() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = cell.gate_dt_s().unwrap();
        let yield_m = uniaxial_yield_strain() * cell.length_m[0];
        let start_m = 0.999 * yield_m;
        let lands = |speed_m_s: f64| {
            let mut state =
                Ti64LatticeState::try_phase(&cell, [start_m, 0.0, 0.0], [speed_m_s, 0.0, 0.0])
                    .unwrap();
            let mut frame = blank_frame();
            match step(&cell, &mut state, &mut frame, Ti64Load::default(), dt) {
                Ok(()) => Some(state.von_mises_mpa),
                Err(StepReject::YieldReached) => None,
                Err(other) => std::panic!("{speed_m_s} m/s refused with {other:?}"),
            }
        };
        let mut inside = 0.0;
        let mut outside = 5.0;
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
        let landed_mpa = lands(inside).unwrap();
        assert!(landed_mpa <= YIELD_STRENGTH_MPA);
        assert!(
            YIELD_STRENGTH_MPA - landed_mpa < 1.0e-9,
            "landed at {landed_mpa} MPa"
        );
        let mut state =
            Ti64LatticeState::try_phase(&cell, [start_m, 0.0, 0.0], [outside, 0.0, 0.0]).unwrap();
        let mut frame = blank_frame();
        refused_whole(
            &cell,
            &mut state,
            &mut frame,
            Ti64Load::default(),
            dt,
            StepReject::YieldReached,
        );
        assert_eq!(frame.flags, u64::MAX);
        proof_intact(&frame);

        // A force switched on at rest swings the node to twice its static
        // stretch. The refusing steps were worked outside this crate.
        let f_yield = YIELD_STRENGTH_MPA * pa_per_mpa() * cell.area_m2[0];
        let cases = [(0.55, 127_u32), (0.75, 96), (1.5, 62)];
        let mut i = 0;
        while i < cases.len() {
            let (yield_fraction, refused_step) = cases[i];
            let load = Ti64Load {
                force_n: [yield_fraction * f_yield, 0.0, 0.0],
                torque_nm: [0.0; 3],
                temperature_k: REFERENCE_TEMPERATURE_K,
            };
            let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
            let mut frame = blank_frame();
            let mut n = 1_u32;
            while n < refused_step {
                step(&cell, &mut state, &mut frame, load, dt).unwrap();
                assert!(state.von_mises_mpa <= YIELD_STRENGTH_MPA);
                n += 1;
            }
            refused_whole(
                &cell,
                &mut state,
                &mut frame,
                load,
                dt,
                StepReject::YieldReached,
            );
            // The last landing is within one step's travel of yield.
            assert!(
                state.von_mises_mpa > 0.95 * YIELD_STRENGTH_MPA,
                "stopped at {} MPa",
                state.von_mises_mpa
            );
            i += 1;
        }
        // Half the yield force peaks on 880 MPa and is never refused.
        let load = Ti64Load {
            force_n: [0.5 * f_yield, 0.0, 0.0],
            torque_nm: [0.0; 3],
            temperature_k: REFERENCE_TEMPERATURE_K,
        };
        let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
        let mut n = 0_u32;
        while n < 2_000 {
            step(&cell, &mut state, &mut frame, load, dt).unwrap();
            n += 1;
        }
    }

    /// A peak that falls between two landings is tested at its turning
    /// point. Swings over yield by 6 parts in 100,000 and by one part in
    /// ten million, the second far inside what a landing alone could miss at
    /// the step ceiling (1 − cos(0.01), 5 parts in 100,000), are each refused
    /// on the step that holds their first peak.
    #[test]
    fn a_swing_just_over_yield_is_caught_on_its_first_peak() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = cell.gate_dt_s().unwrap();
        let w = cell.max_natural_frequency_rad_s();
        let yield_m = uniaxial_yield_strain() * cell.length_m[0];
        // A quarter period is 78.5 steps: the peak falls inside step 79.
        // Six parts in 100,000 over, the landing at step 78 is already past
        // yield; one part in ten million over, step 78 lands under yield and
        // the step that holds the peak, step 79, is refused at its turning point.
        for (over, last_landed) in [(1.000_06 * yield_m, 77_u32), (1.000_000_1 * yield_m, 78)] {
            let mut state =
                Ti64LatticeState::try_phase(&cell, [0.0; 3], [w * over, 0.0, 0.0]).unwrap();
            let mut frame = blank_frame();
            let mut landed = 0_u32;
            loop {
                let held = state_bits(&state);
                let held_frame = frame.to_bytes();
                match step(&cell, &mut state, &mut frame, Ti64Load::default(), dt) {
                    Ok(()) => {
                        landed += 1;
                        assert!(landed < 80, "the first peak was missed");
                        assert!(state.von_mises_mpa <= YIELD_STRENGTH_MPA);
                    }
                    Err(reject) => {
                        assert_eq!(reject, StepReject::YieldReached);
                        assert_eq!(state_bits(&state), held);
                        assert_eq!(frame.to_bytes(), held_frame);
                        break;
                    }
                }
            }
            assert_eq!(landed, last_landed, "refused after {landed} steps");
            // The last landing is short of the peak, and under yield.
            assert!(state.von_mises_mpa > 879.0 && state.von_mises_mpa <= YIELD_STRENGTH_MPA);
            assert!(state.velocity_m_s[0] > 0.0);
        }
        let mut frame = blank_frame();

        // A swing that peaks one part in a million under yield is never refused.
        let under = 0.999_999 * yield_m;
        let mut state =
            Ti64LatticeState::try_phase(&cell, [0.0; 3], [w * under, 0.0, 0.0]).unwrap();
        let mut n = 0_u32;
        while n < 2_000 {
            step(&cell, &mut state, &mut frame, Ti64Load::default(), dt).unwrap();
            n += 1;
        }
        proof_intact(&frame);
    }

    #[test]
    fn a_biased_ledger_is_refused_both_ways_and_rolls_back() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = cell.gate_dt_s().unwrap();
        let x0 = 0.05 * uniaxial_yield_strain() * cell.length_m[0];
        let mut state = Ti64LatticeState::try_phase(&cell, [x0, 0.0, 0.0], [0.0; 3]).unwrap();
        let mut frame = blank_frame();
        let load = Ti64Load::default();
        step(&cell, &mut state, &mut frame, load, dt).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();

        // Work booked that no boundary did: the energy is short of the books.
        let extra = 1.0e-3 * cell.yield_energy_scale_j();
        let got = step_bias_work(&cell, &mut state, &mut frame, load, dt, extra);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        // Work left off the books: the energy is more than they account for.
        let got = step_bias_work(&cell, &mut state, &mut frame, load, dt, -extra);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        // The same step with nothing false in it lands.
        step_bias_work(&cell, &mut state, &mut frame, load, dt, 0.0).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        assert_eq!(frame.flags, 0);
        proof_intact(&frame);
    }

    /// A false entry far too small to see in one step: 1.5e-9 of the yield
    /// energy a step, 1/667 of the limit. The ledger is never re-anchored, so
    /// it adds up, and the 667th step is refused.
    #[test]
    fn a_small_persistent_bias_adds_up_to_a_refusal() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = cell.gate_dt_s().unwrap();
        let x0 = 0.05 * uniaxial_yield_strain() * cell.length_m[0];
        let per_step = 1.5e-9 * cell.yield_energy_scale_j();
        let cases = [
            (per_step, StepReject::FirstLawBreach),
            (-per_step, StepReject::SpontaneousGeneration),
        ];
        let mut i = 0;
        while i < cases.len() {
            let (extra_j, expect) = cases[i];
            let mut state = Ti64LatticeState::try_phase(&cell, [x0, 0.0, 0.0], [0.0; 3]).unwrap();
            let mut frame = blank_frame();
            let mut landed = 0_u32;
            loop {
                let held = state_bits(&state);
                let held_frame = frame.to_bytes();
                match step_bias_work(
                    &cell,
                    &mut state,
                    &mut frame,
                    Ti64Load::default(),
                    dt,
                    extra_j,
                ) {
                    Ok(()) => {
                        landed += 1;
                        assert!(landed < 1_000);
                    }
                    Err(reject) => {
                        assert_eq!(reject, expect);
                        assert_eq!(state_bits(&state), held);
                        assert_eq!(frame.to_bytes(), held_frame);
                        break;
                    }
                }
            }
            assert_eq!(landed, 666);
            proof_intact(&frame);
            i += 1;
        }
    }

    /// An honest swing loses energy to the step method, 8.9e-13 of the swing
    /// a step at the step ceiling. With all three ligaments swinging at 99
    /// percent of yield, a million steps spend 87 percent of the limit and
    /// none is refused.
    #[test]
    fn a_million_honest_steps_near_yield_stay_inside_the_limit() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = cell.gate_dt_s().unwrap();
        let w = cell.max_natural_frequency_rad_s();
        let amplitude_m = 0.99 * uniaxial_yield_strain() * cell.length_m[0];
        let mut state = Ti64LatticeState::try_phase(&cell, [0.0; 3], [w * amplitude_m; 3]).unwrap();
        let mut frame = blank_frame();
        let mut n = 0_u32;
        while n < 1_000_000 {
            step(&cell, &mut state, &mut frame, Ti64Load::default(), dt).unwrap();
            n += 1;
        }
        assert!(state.energy_defect_j < 0.0);
        let spent = state.residual_conservation_error / RESIDUAL_CONSERVATION_LIMIT;
        assert!(spent > 0.85 && spent < 0.89, "spent {spent} of the limit");
        proof_intact(&frame);
    }

    /// A force held from rest swings the node to twice its static stretch.
    /// Just over half the yield force, 1760.04 N, the peak is 880.02 MPa and
    /// falls between two landings, each under 880 MPa. The step that holds
    /// the turning point is refused. At exactly half the yield force the
    /// peak is 880 MPa and every step lands.
    #[test]
    fn a_held_force_just_over_half_yield_is_refused_at_its_turning_point() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = 1.363_297_598_465_000_3e-7;
        let f_yield = YIELD_STRENGTH_MPA * pa_per_mpa() * cell.area_m2[0];
        for force_n in [1_760.04, 1_760.000_5] {
            let load = Ti64Load {
                force_n: [force_n, 0.0, 0.0],
                torque_nm: [0.0; 3],
                temperature_k: REFERENCE_TEMPERATURE_K,
            };
            let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
            let mut frame = blank_frame();
            let mut landed = 0_u32;
            loop {
                let held = state_bits(&state);
                match step(&cell, &mut state, &mut frame, load, dt) {
                    Ok(()) => {
                        landed += 1;
                        assert!(state.von_mises_mpa <= YIELD_STRENGTH_MPA);
                        assert!(landed < 200, "{force_n} N: the first peak was missed");
                    }
                    Err(reject) => {
                        assert_eq!(reject, StepReject::YieldReached);
                        assert_eq!(state_bits(&state), held);
                        break;
                    }
                }
            }
            // Half a period is 157.3 steps of this length.
            assert_eq!(landed, 157, "{force_n} N refused after {landed} steps");
            assert!(state.von_mises_mpa < YIELD_STRENGTH_MPA);
            assert!(state.velocity_m_s[0] > 0.0);
        }
        let load = Ti64Load {
            force_n: [0.5 * f_yield, 0.0, 0.0],
            torque_nm: [0.0; 3],
            temperature_k: REFERENCE_TEMPERATURE_K,
        };
        assert_eq!(0.5 * f_yield, 1_760.0);
        let mut state = Ti64LatticeState::try_rest(&cell).unwrap();
        let mut frame = blank_frame();
        let mut n = 0_u32;
        while n < 20_000 {
            step(&cell, &mut state, &mut frame, load, dt).unwrap();
            n += 1;
        }
        proof_intact(&frame);
    }

    /// The turning point of a hardened ligament is found on its strain
    /// energy. A swing that one landing alone would let through is refused
    /// on the step that holds its peak.
    #[test]
    fn a_hardened_ligament_is_tested_at_its_turning_point() {
        let cell = column(0.25, [0.0; 3]);
        let dt = cell.gate_dt_s().unwrap();
        let k = cell.stiffness_n_m()[0];
        let beta = cell.hardening_n_m3[0];
        let mass = cell.mass_kg();
        // The stretch at which tension k x + beta x^3 is the yield tension.
        let f_yield = YIELD_STRENGTH_MPA * pa_per_mpa() * cell.area_m2[0];
        let mut lo: f64 = 0.0;
        let mut hi = uniaxial_yield_strain() * cell.length_m[0];
        while lo.next_up() < hi {
            let mid = lo + 0.5 * (hi - lo);
            if k * mid + beta * mid * mid * mid <= f_yield {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let peak_energy = |x: f64| 0.5 * k * x * x + 0.25 * beta * x * x * x * x;
        let speed_to = |x: f64| (2.0 * peak_energy(x) / mass).sqrt();
        for (scale, refused) in [(1.000_000_1, true), (0.999_99, false)] {
            let mut state = Ti64LatticeState::try_phase(
                &cell,
                [0.0; 3],
                [speed_to(scale * lo), 0.0, 0.0],
            )
            .unwrap();
            let mut frame = blank_frame();
            let mut landed = 0_u32;
            let mut yielded = false;
            while landed < 400 {
                match step(&cell, &mut state, &mut frame, Ti64Load::default(), dt) {
                    Ok(()) => {
                        assert!(state.von_mises_mpa <= YIELD_STRENGTH_MPA);
                        landed += 1;
                    }
                    Err(reject) => {
                        assert_eq!(reject, StepReject::YieldReached);
                        yielded = true;
                        break;
                    }
                }
            }
            assert_eq!(yielded, refused, "scale {scale} landed {landed}");
            if refused {
                assert!(landed < 100, "refused after {landed} steps");
                assert!(state.velocity_m_s[0] > 0.0);
            }
        }
    }

    /// A run carried on from its own returned state, with the clock, the held
    /// load and the two ledger entries, is the run straight through, bit for
    /// bit, ledger and clock included.
    #[test]
    fn a_run_carried_on_from_its_state_is_the_run_straight_through() {
        let cell = Ti64LatticeCell::annealed_column();
        let dt = cell.gate_dt_s().unwrap();
        let load = Ti64Load {
            force_n: [700.0, -300.0, 50.0],
            torque_nm: [0.0; 3],
            temperature_k: 900.0,
        };
        let begin = Ti64LatticeState::try_phase(&cell, [2.0e-5, -1.0e-5, 0.0], [1.0, 2.0, -3.0])
            .unwrap();
        let mut straight = begin;
        let mut frame = blank_frame();
        for _ in 0..600 {
            step(&cell, &mut straight, &mut frame, load, dt).unwrap();
        }
        let mut first = begin;
        for _ in 0..300 {
            step(&cell, &mut first, &mut frame, load, dt).unwrap();
        }
        let handed = Ti64Start {
            t_s: first.t_s,
            position_m: first.position_m,
            velocity_m_s: first.velocity_m_s,
            applied_force_n: first.applied_force_n,
            temperature_k: first.temperature_k,
            energy_anchor_j: first.energy_anchor_j,
            boundary_work_j: first.boundary_work_j,
        };
        let mut carried = Ti64LatticeState::try_continue(&cell, handed).unwrap();
        assert_eq!(carried.energy_defect_j.to_bits(), first.energy_defect_j.to_bits());
        assert_eq!(
            carried.residual_conservation_error.to_bits(),
            first.residual_conservation_error.to_bits()
        );
        for _ in 0..300 {
            step(&cell, &mut carried, &mut frame, load, dt).unwrap();
        }
        let mut a = state_bits(&straight);
        let mut b = state_bits(&carried);
        // The reported torque is the only field that the start does not carry:
        // it is the applied couple of the last step, and it is not in the motion.
        for i in 9..12 {
            a[i] = 0;
            b[i] = 0;
        }
        assert_eq!(a, b);
        assert!(straight.t_s > first.t_s && straight.boundary_work_j != 0.0);

        // A new start at rest is the card's own rest.
        let rest = Ti64LatticeState::try_rest(&cell).unwrap();
        let fresh = Ti64LatticeState::try_continue(
            &cell,
            Ti64Start {
                t_s: 0.0,
                position_m: [0.0; 3],
                velocity_m_s: [0.0; 3],
                applied_force_n: [0.0; 3],
                temperature_k: REFERENCE_TEMPERATURE_K,
                energy_anchor_j: 0.0,
                boundary_work_j: 0.0,
            },
        )
        .unwrap();
        assert_eq!(state_bits(&fresh), state_bits(&rest));
    }

    /// A ledger cannot be cleared by handing a state back: a start whose
    /// stated anchor and work leave a residual past the limit, a clock below
    /// zero, or work booked at a clock of zero, is refused.
    #[test]
    fn a_start_the_card_could_not_have_written_is_refused() {
        let cell = Ti64LatticeCell::annealed_column();
        let scale = cell.yield_energy_scale_j();
        let good = Ti64Start {
            t_s: 1.0e-3,
            position_m: [1.0e-5, 0.0, 0.0],
            velocity_m_s: [0.5, 0.0, 0.0],
            applied_force_n: [100.0, 0.0, 0.0],
            temperature_k: 400.0,
            energy_anchor_j: 0.0,
            boundary_work_j: 0.0,
        };
        let mut probe =
            Ti64LatticeState::try_phase(&cell, good.position_m, good.velocity_m_s).unwrap();
        probe.applied_force_n = good.applied_force_n;
        probe.temperature_k = good.temperature_k;
        let energy = super::mechanical_energy(&cell, &probe);
        let at = |anchor: f64, work: f64, t_s: f64| {
            Ti64LatticeState::try_continue(
                &cell,
                Ti64Start {
                    t_s,
                    energy_anchor_j: anchor,
                    boundary_work_j: work,
                    ..good
                },
            )
        };
        // The energy split between anchor and work, inside the limit, is accepted.
        let state = at(energy - 0.25, 0.25, 1.0e-3).unwrap();
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        let edge = 0.9 * RESIDUAL_CONSERVATION_LIMIT * scale;
        at(energy - edge, 0.0, 1.0e-3).unwrap();
        at(energy + edge, 0.0, 1.0e-3).unwrap();
        let past = 1.1 * RESIDUAL_CONSERVATION_LIMIT * scale;
        assert_eq!(at(energy - past, 0.0, 1.0e-3), Err(StepReject::StartNotWritten));
        assert_eq!(at(energy + past, 0.0, 1.0e-3), Err(StepReject::StartNotWritten));
        assert_eq!(at(energy, 0.0, -1.0e-9), Err(StepReject::StartNotWritten));
        assert_eq!(at(energy - 0.25, 0.25, 0.0), Err(StepReject::StartNotWritten));
        at(energy, 0.0, 0.0).unwrap();
        assert_eq!(
            Ti64LatticeState::try_continue(&cell, Ti64Start { temperature_k: 1_300.0, ..good }),
            Err(StepReject::TemperatureOutsideCard)
        );
        assert_eq!(
            Ti64LatticeState::try_continue(&cell, Ti64Start { energy_anchor_j: f64::NAN, ..good }),
            Err(StepReject::NonFiniteState)
        );
    }
}
