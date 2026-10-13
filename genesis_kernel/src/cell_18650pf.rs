#![no_implicit_prelude]

//! One Panasonic NCR18650PF cell, serial NCR18650PF_SN002, new, at 25 °C.
//!
//! Measured by Kollmeyer (2018), Mendeley Data, doi: 10.17632/wykht8y7tg.1,
//! CC BY 4.0. The license requires that credit.
//!
//! The card is a held 1C discharge from full down to 3 percent state of
//! charge or 2.5 V, whichever comes first. The card has no temperature and
//! no memory.
//!
//! Held means what the tester held. The card passed its comparison with
//! measurement only on 1C discharges from full charge, and each run of that comparison
//! fed the tester's own logged current, which moves between 2.899 and
//! 2.89982 A over a run: 0.82 mA, one step of the 0.81 mA in which the tester
//! logs its current. The dataset states no accuracy for the tester. The card
//! takes a current from [`CURRENT_COMPARED_MIN_A`] to
//! [`CURRENT_COMPARED_MAX_A`], the compared span widened on each side by
//! [`HELD_CURRENT_BAND_A`], and refuses any other as
//! [`StepReject::CurrentNotCompared`]. A run is one current: every step's
//! current stays within 0.83 mA of the held current, the current of the
//! run's first step, and a step outside it is
//! [`StepReject::CurrentChangedDuringDischarge`]. A rest, a pulse and a
//! drive cycle were never compared with a measurement of this cell: the card
//! has no memory, and the real cell's voltage relaxes for minutes after a
//! change of load. One other held current was compared and failed: the held
//! 0.86888 to 0.86970 A discharge 04-27-17 3541_dis5_10p.mat, 56.7 mV rms
//! during its holds against the line of 50 mV, and the card left its table
//! instead of stopping at its own cutoff
//! (evidence/research/panasonic_18650pf/held_0p87A_comparison). Those
//! currents are refused as [`StepReject::CurrentNotCompared`].
//!
//! Every discharge that passed the comparison started at full charge. A state below full
//! charge is taken only as the continuation of a 1C discharge, from
//! [`CellState::try_continued`], which carries that discharge's held
//! current; a state below full charge with no held current is
//! [`StepReject::StartNotAtFullCharge`].
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, terminal voltage, state of charge, current,
//! open-circuit voltage, chemical energy, Joule heat, the margin to cutoff,
//! and the conservation residual into [`LastStateFrame64`]. The 16-byte
//! frame proof is not touched.
//!
//! Open-circuit voltage and resistance are straight lines between the locked
//! knots. A step splits at those knots and holds each interval's line through
//! all four RK4 stages. The arithmetic is `+ − × ÷` only.
//!
//! ```text
//! d s/dt = −I / Q
//! V_oc, R   straight lines between the locked knots
//! V = V_oc − I R
//! E(s) = Q ∫ V_oc ds from the bottom knot
//! dW_elec/dt = V I
//! dW_joule/dt = I² R
//! residual = |E − E_anchor + W_elec + W_joule| / CELL_ENERGY_J
//! ```
//!
//! `I > 0` discharges. [`Cell18650Pf::gate_dt_s`] is the time to cross one
//! table interval at 3.0 A. The public step does not apply that clock.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Fitted capacity. Not the C/20 counter.
pub const CAPACITY_AH: f64 = 2.906;
/// `2.906 Ah` in coulombs.
pub const CAPACITY_C: f64 = CAPACITY_AH * 3_600.0;
/// Laboratory end of every 25 °C discharge.
pub const CUTOFF_V: f64 = 2.5;
/// Knots per unit state of charge. Spacing is `1/200`.
pub const TABLE_INTERVALS_PER_UNIT: f64 = 200.0;
/// First knot. State of charge `6/200`.
pub const KNOT_FIRST: usize = 6;
/// Last knot. State of charge 1.
pub const KNOT_LAST: usize = 200;
/// Knots in the two locked tables.
pub const KNOTS: usize = (KNOT_LAST - KNOT_FIRST) + 1;
/// Bottom of the card. `KNOT_FIRST / 200`.
pub const SOC_MIN: f64 = (KNOT_FIRST as f64) / TABLE_INTERVALS_PER_UNIT;
/// The current the step ceiling is set at, A: the cap the card was built
/// with, above the 1C it was to be tested at. No table was fitted at it: the
/// rest-voltage curve came from 0.145 A and the resistance from 0.87 A holds.
/// The card does not take it; it takes only the compared span,
/// [`CURRENT_COMPARED_MIN_A`] to [`CURRENT_COMPARED_MAX_A`].
pub const CURRENT_MAX_A: f64 = 3.0;
/// How far a step's current may lie from the held current, A. The dataset
/// states no accuracy for the tester; it logs current in steps of 0.81 mA,
/// and the measured current fed to the 43 saved validation runs spans
/// 0.82 mA within a run, 2.899 to 2.89982 A. The band is the larger, 0.82 mA,
/// written as 0.83 mA. [`held_current_within_band`] tests it with an
/// allowance of four units in the last place at the size of the currents, so
/// that every pair of decimal currents exactly 0.83 mA apart is inside it.
pub const HELD_CURRENT_BAND_A: f64 = 0.00083;
/// Smallest current the card takes, A: 2.899 A, the lowest current fed to a
/// compared discharge, less [`HELD_CURRENT_BAND_A`].
pub const CURRENT_COMPARED_MIN_A: f64 = 2.89817;
/// Largest current the card takes, A: 2.89982 A, the highest current fed to
/// a compared discharge, plus [`HELD_CURRENT_BAND_A`].
pub const CURRENT_COMPARED_MAX_A: f64 = 2.90065;
/// Largest allowance, in state of charge, by which a discharge that ends in
/// decimal exactly on the bottom knot may land below it in 64-bit arithmetic
/// and still land on it. One unit in the last place at 1 for each step, up to
/// this cap: four million steps.
pub const SOC_SUM_ALLOWANCE_MAX: f64 = 1.0e-9;
/// Advisory only. Not a measurement.
pub const WARN_V: f64 = 3.0;
/// `|E − E_anchor + W_elec + W_joule| / CELL_ENERGY_J` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Terminal voltage is below [`WARN_V`] and above cutoff on this frame.
pub const FLAG_SAG_WARNING: u64 = 1 << 0;

/// Open-circuit voltage at knots 6 through 200, as decimal literals.
pub const OPEN_CIRCUIT_V: [f64; KNOTS] = [
    3.2654,
    3.2773,
    3.2863,
    3.2924,
    3.2958,
    3.3024,
    3.3091,
    3.3151,
    3.3200,
    3.3249,
    3.3292,
    3.3335,
    3.3378,
    3.3421,
    3.3466,
    3.3517,
    3.3583,
    3.3652,
    3.3721,
    3.3796,
    3.3868,
    3.3934,
    3.4004,
    3.4072,
    3.4137,
    3.4204,
    3.4270,
    3.4328,
    3.4394,
    3.4449,
    3.4505,
    3.4557,
    3.4609,
    3.4662,
    3.4717,
    3.4772,
    3.4821,
    3.4875,
    3.4929,
    3.4983,
    3.5030,
    3.5078,
    3.5125,
    3.5166,
    3.5207,
    3.5247,
    3.5287,
    3.5327,
    3.5360,
    3.5394,
    3.5433,
    3.5461,
    3.5494,
    3.5527,
    3.5560,
    3.5593,
    3.5619,
    3.5650,
    3.5677,
    3.5703,
    3.5731,
    3.5756,
    3.5788,
    3.5814,
    3.5840,
    3.5866,
    3.5892,
    3.5918,
    3.5948,
    3.5973,
    3.6002,
    3.6028,
    3.6055,
    3.6081,
    3.6109,
    3.6134,
    3.6160,
    3.6192,
    3.6219,
    3.6248,
    3.6278,
    3.6305,
    3.6337,
    3.6368,
    3.6397,
    3.6430,
    3.6462,
    3.6495,
    3.6528,
    3.6561,
    3.6594,
    3.6627,
    3.6660,
    3.6699,
    3.6739,
    3.6780,
    3.6821,
    3.6863,
    3.6911,
    3.6959,
    3.7006,
    3.7055,
    3.7110,
    3.7164,
    3.7224,
    3.7286,
    3.7348,
    3.7411,
    3.7474,
    3.7536,
    3.7599,
    3.7655,
    3.7709,
    3.7768,
    3.7819,
    3.7872,
    3.7918,
    3.7971,
    3.8017,
    3.8063,
    3.8109,
    3.8155,
    3.8200,
    3.8246,
    3.8292,
    3.8332,
    3.8378,
    3.8424,
    3.8463,
    3.8509,
    3.8549,
    3.8595,
    3.8635,
    3.8674,
    3.8714,
    3.8753,
    3.8792,
    3.8831,
    3.8870,
    3.8915,
    3.8954,
    3.8993,
    3.9032,
    3.9071,
    3.9110,
    3.9149,
    3.9195,
    3.9240,
    3.9279,
    3.9325,
    3.9370,
    3.9416,
    3.9461,
    3.9513,
    3.9559,
    3.9611,
    3.9659,
    3.9716,
    3.9766,
    3.9820,
    3.9873,
    3.9931,
    3.9986,
    4.0042,
    4.0100,
    4.0153,
    4.0211,
    4.0264,
    4.0321,
    4.0368,
    4.0427,
    4.0475,
    4.0531,
    4.0577,
    4.0623,
    4.0669,
    4.0716,
    4.0756,
    4.0796,
    4.0836,
    4.0875,
    4.0915,
    4.0949,
    4.0989,
    4.1029,
    4.1075,
    4.1121,
    4.1174,
    4.1232,
    4.1292,
    4.1366,
    4.1446,
    4.1536,
    4.1649,
    4.1827
];

/// Held resistance at knots 6 through 200, as decimal literals.
pub const HELD_RESISTANCE_OHM: [f64; KNOTS] = [
    0.6950,
    0.5506,
    0.4362,
    0.3456,
    0.2738,
    0.2369,
    0.2171,
    0.1989,
    0.1823,
    0.1671,
    0.1531,
    0.1403,
    0.1286,
    0.1179,
    0.1080,
    0.1024,
    0.0990,
    0.0958,
    0.0927,
    0.0896,
    0.0867,
    0.0838,
    0.0811,
    0.0784,
    0.0758,
    0.0748,
    0.0744,
    0.0741,
    0.0738,
    0.0735,
    0.0732,
    0.0729,
    0.0726,
    0.0723,
    0.0720,
    0.0724,
    0.0731,
    0.0738,
    0.0746,
    0.0753,
    0.0761,
    0.0768,
    0.0776,
    0.0784,
    0.0792,
    0.0793,
    0.0793,
    0.0792,
    0.0791,
    0.0790,
    0.0789,
    0.0789,
    0.0788,
    0.0787,
    0.0786,
    0.0780,
    0.0773,
    0.0765,
    0.0758,
    0.0750,
    0.0743,
    0.0736,
    0.0728,
    0.0721,
    0.0714,
    0.0707,
    0.0700,
    0.0693,
    0.0686,
    0.0680,
    0.0673,
    0.0666,
    0.0660,
    0.0653,
    0.0647,
    0.0642,
    0.0638,
    0.0633,
    0.0629,
    0.0625,
    0.0621,
    0.0617,
    0.0613,
    0.0609,
    0.0604,
    0.0600,
    0.0596,
    0.0592,
    0.0588,
    0.0585,
    0.0581,
    0.0577,
    0.0573,
    0.0569,
    0.0565,
    0.0573,
    0.0584,
    0.0596,
    0.0607,
    0.0619,
    0.0631,
    0.0644,
    0.0656,
    0.0669,
    0.0682,
    0.0695,
    0.0709,
    0.0723,
    0.0737,
    0.0751,
    0.0766,
    0.0781,
    0.0796,
    0.0812,
    0.0828,
    0.0828,
    0.0825,
    0.0823,
    0.0820,
    0.0818,
    0.0815,
    0.0812,
    0.0810,
    0.0807,
    0.0805,
    0.0802,
    0.0800,
    0.0797,
    0.0795,
    0.0792,
    0.0790,
    0.0787,
    0.0785,
    0.0782,
    0.0780,
    0.0774,
    0.0767,
    0.0761,
    0.0755,
    0.0748,
    0.0742,
    0.0736,
    0.0730,
    0.0724,
    0.0718,
    0.0712,
    0.0706,
    0.0700,
    0.0694,
    0.0689,
    0.0683,
    0.0677,
    0.0672,
    0.0666,
    0.0661,
    0.0657,
    0.0653,
    0.0649,
    0.0645,
    0.0641,
    0.0638,
    0.0634,
    0.0630,
    0.0626,
    0.0623,
    0.0619,
    0.0615,
    0.0612,
    0.0608,
    0.0605,
    0.0601,
    0.0598,
    0.0594,
    0.0591,
    0.0587,
    0.0587,
    0.0587,
    0.0587,
    0.0587,
    0.0587,
    0.0587,
    0.0587,
    0.0587,
    0.0586,
    0.0586,
    0.0586,
    0.0586,
    0.0586,
    0.0586,
    0.0586,
    0.0586,
    0.0586,
    0.0586,
    0.0586,
    0.0586
];

const fn energy_at_knots() -> [f64; KNOTS] {
    let mut energy = [0.0; KNOTS];
    let mut index = 0;
    while index < KNOTS - 1 {
        let low = OPEN_CIRCUIT_V[index];
        let high = OPEN_CIRCUIT_V[index + 1];
        energy[index + 1] = energy[index] + CAPACITY_C * 0.005 * 0.5 * (low + high);
        index += 1;
    }
    energy
}

/// Chemical energy from the bottom knot up to each knot.
pub const ENERGY_AT_KNOT_J: [f64; KNOTS] = energy_at_knots();
/// Chemical energy from the bottom knot to full. Scale of the residual.
pub const CELL_ENERGY_J: f64 = ENERGY_AT_KNOT_J[KNOTS - 1];

/// Why a step did not advance the cell or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Current or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant or table entry is not this cell.
    InvalidCell,
    /// Current is outside 2.89817 to 2.90065 A, the 1C currents compared with measurement widened by 0.83 mA.
    /// Any other current is refused by this name, the held 0.87 A among them, which was compared and failed.
    CurrentNotCompared,
    /// The current is more than 0.83 mA from the held current, set by the run's first step or carried in from the discharge it continues: the card is of one held current.
    CurrentChangedDuringDischarge,
    /// State of charge is below 1 with no held current: every compared discharge started at full charge.
    StartNotAtFullCharge,
    /// State of charge, incoming or landing, is outside `[0.03, 1]`.
    StateOfChargeOutsideCard,
    /// Landing terminal voltage is at or below 2.5 V.
    UndervoltageCutoff,
    /// The trial step created chemical energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed chemical energy.
    FirstLawBreach,
}

/// Zero-order hold. Positive current discharges the cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellLoad {
    pub current_a: f64,
}

impl Default for CellLoad {
    fn default() -> Self {
        Self { current_a: 0.0 }
    }
}

/// The locked cell. The two tables are recorded copies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell18650Pf {
    pub capacity_ah: f64,
    pub capacity_c: f64,
    pub cutoff_v: f64,
    pub table_intervals_per_unit: f64,
    pub knot_first: usize,
    pub knot_last: usize,
    pub soc_min: f64,
    pub open_circuit_v: [f64; KNOTS],
    pub held_resistance_ohm: [f64; KNOTS],
    pub current_max_a: f64,
    pub warn_v: f64,
    pub residual_conservation_limit: f64,
}

impl Cell18650Pf {
    /// The fitted NCR18650PF cell. The constructor takes no arguments.
    #[must_use]
    pub fn cell() -> Self {
        let cell = Self {
            capacity_ah: CAPACITY_AH,
            capacity_c: CAPACITY_C,
            cutoff_v: CUTOFF_V,
            table_intervals_per_unit: TABLE_INTERVALS_PER_UNIT,
            knot_first: KNOT_FIRST,
            knot_last: KNOT_LAST,
            soc_min: SOC_MIN,
            open_circuit_v: OPEN_CIRCUIT_V,
            held_resistance_ohm: HELD_RESISTANCE_OHM,
            current_max_a: CURRENT_MAX_A,
            warn_v: WARN_V,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if cell.is_valid() {
            cell
        } else {
            ::core::panic!("NCR18650PF cell failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        if self.capacity_ah.to_bits() != CAPACITY_AH.to_bits()
            || self.capacity_c.to_bits() != CAPACITY_C.to_bits()
            || self.cutoff_v.to_bits() != CUTOFF_V.to_bits()
            || self.table_intervals_per_unit.to_bits() != TABLE_INTERVALS_PER_UNIT.to_bits()
            || self.knot_first != KNOT_FIRST
            || self.knot_last != KNOT_LAST
            || self.soc_min.to_bits() != SOC_MIN.to_bits()
            || self.current_max_a.to_bits() != CURRENT_MAX_A.to_bits()
            || self.warn_v.to_bits() != WARN_V.to_bits()
            || self.residual_conservation_limit.to_bits() != RESIDUAL_CONSERVATION_LIMIT.to_bits()
        {
            return false;
        }
        let mut index = 0;
        while index < KNOTS {
            if self.open_circuit_v[index].to_bits() != OPEN_CIRCUIT_V[index].to_bits()
                || self.held_resistance_ohm[index].to_bits() != HELD_RESISTANCE_OHM[index].to_bits()
            {
                return false;
            }
            if !(self.held_resistance_ohm[index] > 0.0) {
                return false;
            }
            if index > 0 && !(self.open_circuit_v[index] > self.open_circuit_v[index - 1]) {
                return false;
            }
            index += 1;
        }
        self.cutoff_v < self.open_circuit_v[0]
            && self.capacity_c > 0.0
            && CELL_ENERGY_J.is_finite()
            && CELL_ENERGY_J > 0.0
    }

    /// Time to cross one table interval at [`CURRENT_MAX_A`]. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if !self.is_valid() {
            return None;
        }
        let gate = CAPACITY_C * 0.005 / CURRENT_MAX_A;
        if gate.is_finite() && gate > 0.0 {
            Some(gate)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. At the ceiling a step at 3.0 A crosses one table interval. A
    /// million times smaller, it lowers the state of charge by five parts in
    /// a thousand million, which 64-bit arithmetic still resolves to seven
    /// figures. Under that a step is mostly rounding. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }
}

/// Integrated cell. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellState {
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
    /// The held current, A: the current every step is held to. Set by the
    /// run's first step, or carried in by [`CellState::try_continued`]. Zero
    /// before either. Not written to the frame.
    pub held_current_a: f64,
}

impl CellState {
    /// Full charge, open circuit.
    #[must_use]
    pub fn try_full(cell: &Cell18650Pf) -> Option<Self> {
        Self::try_soc(cell, 1.0)
    }

    /// The continuation of a held 1C discharge: the ledger anchored at this
    /// state of charge, carrying the held current of the earlier run. `None`
    /// when the held current is outside the compared span.
    #[must_use]
    pub fn try_continued(cell: &Cell18650Pf, soc: f64, held_current_a: f64) -> Option<Self> {
        if !current_was_compared(held_current_a) {
            return None;
        }
        let mut state = Self::try_soc(cell, soc)?;
        state.held_current_a = held_current_a;
        Some(state)
    }

    /// Anchor the ledger at this state of charge, open circuit, with no held
    /// current. Below full charge the step refuses it as
    /// [`StepReject::StartNotAtFullCharge`].
    #[must_use]
    pub fn try_soc(cell: &Cell18650Pf, soc: f64) -> Option<Self> {
        if !cell.is_valid() || !soc.is_finite() || !(soc >= SOC_MIN && soc <= 1.0) {
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
            held_current_a: 0.0,
        };
        if anchor(cell, &mut state).is_none() {
            return None;
        }
        if !state_is_finite(&state) || state.terminal_v <= cell.cutoff_v {
            return None;
        }
        Some(state)
    }
}

/// Advance one split RK4 step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    cell: &Cell18650Pf,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(cell, state, load, dt_s)?;
    let allowance = soc_sum_allowance(state, dt_s);
    let predicted = match integrate(cell, recorded(state), dt_s, load.current_a, allowance) {
        Ok(y) => y,
        Err(reject) => return Err(reject),
    };
    commit(cell, state, frame, load, dt_s, predicted)
}

fn validate(
    cell: &Cell18650Pf,
    state: &CellState,
    load: CellLoad,
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
    if !cell.is_valid() {
        return Err(StepReject::InvalidCell);
    }
    if !(state.soc >= SOC_MIN && state.soc <= 1.0) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }
    if !current_was_compared(load.current_a) {
        return Err(StepReject::CurrentNotCompared);
    }
    // A held current of zero is a run that has not taken a step and carries
    // no earlier discharge: it must start at full charge, and its first step
    // sets the held current. After that the current stays inside the band.
    if state.held_current_a == 0.0 {
        if state.soc != 1.0 {
            return Err(StepReject::StartNotAtFullCharge);
        }
        return Ok(());
    }
    if !current_was_compared(state.held_current_a) {
        return Err(StepReject::CurrentNotCompared);
    }
    if !held_current_within_band(load.current_a, state.held_current_a) {
        return Err(StepReject::CurrentChangedDuringDischarge);
    }
    Ok(())
}

/// The current is inside the compared span, 2.89817 to 2.90065 A.
#[must_use]
pub fn current_was_compared(current_a: f64) -> bool {
    current_a >= CURRENT_COMPARED_MIN_A && current_a <= CURRENT_COMPARED_MAX_A
}

/// The current lies within [`HELD_CURRENT_BAND_A`] of the held current, read
/// in decimal: the difference may pass 0.83 mA by four units in the last
/// place at the size of the larger current, which covers the rounding of the
/// two decimal currents and of their difference, and nothing more. At 2.9 A
/// that is 2.6e-15 A.
#[must_use]
pub fn held_current_within_band(current_a: f64, held_current_a: f64) -> bool {
    let size = if abs_f64(current_a) > abs_f64(held_current_a) {
        abs_f64(current_a)
    } else {
        abs_f64(held_current_a)
    };
    let band = HELD_CURRENT_BAND_A + 4.0 * f64::EPSILON * size;
    let change_a = current_a - held_current_a;
    change_a <= band && change_a >= -band
}

/// How far below the bottom knot a summed state of charge may land and still
/// land on it: one unit in the last place at 1 for each step taken at this
/// `dt_s`, and four more for the step itself and the start, up to
/// [`SOC_SUM_ALLOWANCE_MAX`].
fn soc_sum_allowance(state: &CellState, dt_s: f64) -> f64 {
    let steps = state.t_s / dt_s;
    let allowance = (steps + 4.0) * f64::EPSILON;
    if allowance.is_finite() && allowance < SOC_SUM_ALLOWANCE_MAX {
        allowance
    } else {
        SOC_SUM_ALLOWANCE_MAX
    }
}

fn integrate(
    cell: &Cell18650Pf,
    mut y: [f64; 3],
    dt_s: f64,
    current_a: f64,
    soc_allowance: f64,
) -> Result<[f64; 3], StepReject> {
    let mut s_target = if current_a == 0.0 {
        y[0]
    } else {
        y[0] - current_a * dt_s / cell.capacity_c
    };
    if !s_target.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    // A discharge that ends in decimal on the bottom knot can land a few
    // units in the last place under it, the rounding of a state of charge
    // summed step by step. Within that rounding it lands on the knot.
    if s_target < SOC_MIN && SOC_MIN - s_target <= soc_allowance {
        s_target = SOC_MIN;
    }
    if !(s_target >= SOC_MIN && s_target <= 1.0) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }
    // A target equal to the start, bit for bit, is a step that does not move
    // the state: a current of zero, or one too small to show at this state
    // of charge. It returns the state as it was, also on the bottom knot,
    // which has no interval below it.
    if current_a == 0.0 || s_target.to_bits() == y[0].to_bits() {
        return Ok(y);
    }
    let mut time_left = dt_s;
    let mut guard = 0u32;
    loop {
        guard += 1;
        if guard > 195 {
            return Err(StepReject::StateOfChargeOutsideCard);
        }
        let interval = match interval_index(y[0]) {
            Some(interval) => interval,
            None => return Err(StepReject::StateOfChargeOutsideCard),
        };
        let s_j = knot_soc(interval);
        let time_to_lower = (y[0] - s_j) * cell.capacity_c / current_a;
        // A current so small that the time to the next knot overflows means
        // the knot is never reached in this step: the time is then infinite,
        // and the step stays inside its interval. Only a time that is not a
        // number, or is negative infinity, is refused.
        if time_to_lower.is_nan() || time_to_lower == f64::NEG_INFINITY {
            return Err(StepReject::NonFiniteState);
        }
        let hit_knot = time_to_lower < time_left && s_j > s_target;
        let h = if hit_knot { time_to_lower } else { time_left };
        if !(h.is_finite() && h > 0.0) {
            return Err(StepReject::NonFiniteState);
        }
        let line = line_of(cell, interval, current_a);
        y = match rk4_line(&line, y, h) {
            Ok(next) => next,
            Err(reject) => return Err(reject),
        };
        if hit_knot {
            y[0] = s_j;
            time_left -= h;
        } else {
            y[0] = s_target;
            break;
        }
    }
    if finite3(y) {
        Ok(y)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

fn commit(
    cell: &Cell18650Pf,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
    predicted: [f64; 3],
) -> Result<(), StepReject> {
    let saved = *state;
    if !finite3(predicted) {
        return Err(StepReject::NonFiniteState);
    }
    let soc = predicted[0];
    if !(soc >= SOC_MIN && soc <= 1.0) {
        return Err(StepReject::StateOfChargeOutsideCard);
    }
    let (open_circuit_v, resistance_ohm, chemical_energy_j) = match sample(cell, soc) {
        Some(value) => value,
        None => return Err(StepReject::StateOfChargeOutsideCard),
    };
    let terminal_v = open_circuit_v - load.current_a * resistance_ohm;
    if !terminal_v.is_finite() || !chemical_energy_j.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if terminal_v <= cell.cutoff_v {
        return Err(StepReject::UndervoltageCutoff);
    }
    state.soc = soc;
    state.electrical_work_j = predicted[1];
    state.joule_j = predicted[2];
    state.current_a = load.current_a;
    if saved.held_current_a == 0.0 {
        state.held_current_a = load.current_a;
    }
    state.open_circuit_v = open_circuit_v;
    state.resistance_ohm = resistance_ohm;
    state.terminal_v = terminal_v;
    state.chemical_energy_j = chemical_energy_j;
    state.cutoff_margin_v = terminal_v - cell.cutoff_v;
    state.sag_warning = terminal_v < cell.warn_v;
    let scale = CELL_ENERGY_J;
    if !(scale > 0.0 && scale.is_finite()) {
        *state = saved;
        return Err(StepReject::InvalidCell);
    }
    let defect =
        chemical_energy_j - state.energy_anchor_j + state.electrical_work_j + state.joule_j;
    if !defect.is_finite() {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    let gate = cell.residual_conservation_limit * scale;
    if defect > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if defect < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    state.energy_defect_j = defect;
    state.residual_conservation_error = abs_f64(defect) / scale;
    state.t_s += dt_s;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    publish(state, frame);
    Ok(())
}

fn anchor(cell: &Cell18650Pf, state: &mut CellState) -> Option<()> {
    let (open_circuit_v, resistance_ohm, chemical_energy_j) = sample(cell, state.soc)?;
    state.current_a = 0.0;
    state.open_circuit_v = open_circuit_v;
    state.resistance_ohm = resistance_ohm;
    state.terminal_v = open_circuit_v;
    state.chemical_energy_j = chemical_energy_j;
    state.energy_anchor_j = chemical_energy_j;
    state.electrical_work_j = 0.0;
    state.joule_j = 0.0;
    state.energy_defect_j = 0.0;
    state.residual_conservation_error = 0.0;
    state.cutoff_margin_v = open_circuit_v - cell.cutoff_v;
    state.sag_warning = open_circuit_v < cell.warn_v;
    Some(())
}

fn publish(state: &CellState, frame: &mut LastStateFrame64) {
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

/// State of charge of table index `index`. Knot number is `KNOT_FIRST + index`.
fn knot_soc(index: usize) -> f64 {
    ((KNOT_FIRST + index) as f64) / TABLE_INTERVALS_PER_UNIT
}

fn exact_knot(soc: f64) -> Option<usize> {
    let mut index = 0;
    while index < KNOTS {
        if soc.to_bits() == knot_soc(index).to_bits() {
            return Some(index);
        }
        if soc < knot_soc(index) {
            return None;
        }
        index += 1;
    }
    None
}

/// Interval whose line holds `soc`. On a knot, the interval below it.
fn interval_index(soc: f64) -> Option<usize> {
    if let Some(index) = exact_knot(soc) {
        if index == 0 {
            return None;
        }
        return Some(index - 1);
    }
    let mut index = 0;
    while index < KNOTS - 1 {
        if soc > knot_soc(index) && soc < knot_soc(index + 1) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn sample(cell: &Cell18650Pf, soc: f64) -> Option<(f64, f64, f64)> {
    if let Some(index) = exact_knot(soc) {
        return Some((
            cell.open_circuit_v[index],
            cell.held_resistance_ohm[index],
            ENERGY_AT_KNOT_J[index],
        ));
    }
    let index = interval_index(soc)?;
    let u = soc - knot_soc(index);
    let slope_v = (cell.open_circuit_v[index + 1] - cell.open_circuit_v[index]) * TABLE_INTERVALS_PER_UNIT;
    let open_circuit_v = cell.open_circuit_v[index] + slope_v * u;
    let resistance_ohm = cell.held_resistance_ohm[index]
        + (cell.held_resistance_ohm[index + 1] - cell.held_resistance_ohm[index])
            * TABLE_INTERVALS_PER_UNIT
            * u;
    let energy = ENERGY_AT_KNOT_J[index]
        + cell.capacity_c * (cell.open_circuit_v[index] * u + slope_v * u * u * 0.5);
    Some((open_circuit_v, resistance_ohm, energy))
}

struct Line {
    s_j: f64,
    v_j: f64,
    r_j: f64,
    m: f64,
    n: f64,
    current_a: f64,
    capacity_c: f64,
}

fn line_of(cell: &Cell18650Pf, index: usize, current_a: f64) -> Line {
    let v0 = cell.open_circuit_v[index];
    let v1 = cell.open_circuit_v[index + 1];
    let r0 = cell.held_resistance_ohm[index];
    let r1 = cell.held_resistance_ohm[index + 1];
    Line {
        s_j: knot_soc(index),
        v_j: v0,
        r_j: r0,
        m: (v1 - v0) * TABLE_INTERVALS_PER_UNIT,
        n: (r1 - r0) * TABLE_INTERVALS_PER_UNIT,
        current_a,
        capacity_c: cell.capacity_c,
    }
}

fn rates_on_line(line: &Line, soc: f64) -> Result<[f64; 3], StepReject> {
    if !soc.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let u = soc - line.s_j;
    let open_circuit_v = line.v_j + line.m * u;
    let resistance_ohm = line.r_j + line.n * u;
    let terminal_v = open_circuit_v - line.current_a * resistance_ohm;
    let out = [
        -line.current_a / line.capacity_c,
        terminal_v * line.current_a,
        line.current_a * line.current_a * resistance_ohm,
    ];
    if finite3(out) {
        Ok(out)
    } else {
        Err(StepReject::NonFiniteState)
    }
}

fn rk4_line(line: &Line, y: [f64; 3], dt: f64) -> Result<[f64; 3], StepReject> {
    let k1 = match rates_on_line(line, y[0]) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k2 = match rates_on_line(line, axpy(y, dt * 0.5, k1)[0]) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k3 = match rates_on_line(line, axpy(y, dt * 0.5, k2)[0]) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    let k4 = match rates_on_line(line, axpy(y, dt, k3)[0]) {
        Ok(k) => k,
        Err(reject) => return Err(reject),
    };
    Ok(rk4_combine(y, dt, k1, k2, k3, k4)?)
}

fn rk4_combine(
    y: [f64; 3],
    dt: f64,
    k1: [f64; 3],
    k2: [f64; 3],
    k3: [f64; 3],
    k4: [f64; 3],
) -> Result<[f64; 3], StepReject> {
    let mut next = y;
    let mut index = 0;
    while index < 3 {
        let increment = k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index];
        if increment != 0.0 {
            next[index] += (dt / 6.0) * increment;
        }
        index += 1;
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

fn recorded(state: &CellState) -> [f64; 3] {
    [state.soc, state.electrical_work_j, state.joule_j]
}

fn finite3(x: [f64; 3]) -> bool {
    x[0].is_finite() && x[1].is_finite() && x[2].is_finite()
}

fn abs_f64(value: f64) -> f64 {
    if value < 0.0 { -value } else { value }
}

fn state_is_finite(state: &CellState) -> bool {
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
        && state.held_current_a.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::panic;

    use super::{
        CAPACITY_C, CELL_ENERGY_J, CURRENT_COMPARED_MAX_A, CURRENT_COMPARED_MIN_A, CUTOFF_V,
        Cell18650Pf, CellLoad, CellState, HELD_CURRENT_BAND_A,
        ENERGY_AT_KNOT_J, FLAG_SAG_WARNING, HELD_RESISTANCE_OHM, KNOTS, OPEN_CIRCUIT_V,
        RESIDUAL_CONSERVATION_LIMIT, SOC_MIN, SOC_SUM_ALLOWANCE_MAX, TABLE_INTERVALS_PER_UNIT,
        WARN_V, StepReject, abs_f64, axpy, commit, current_was_compared,
        held_current_within_band, integrate, interval_index, knot_soc, line_of, rates_on_line,
        recorded, rk4_combine, sample, soc_sum_allowance, step, validate,
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
        let mut index = 0;
        while index < 16 {
            assert_eq!(frame.proof[index], 0xA5, "proof byte {index} was resealed");
            index += 1;
        }
    }

    fn assert_copy<T: Copy>() {}

    fn state_bits(state: &CellState) -> [u64; 15] {
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
            state.held_current_a.to_bits(),
        ]
    }

    /// A run is one current. The first step sets it; a later step inside
    /// 0.83 mA of it lands, as the tester's logged current of a 1C discharge
    /// does, 2.899 to 2.89982 A; a current 0.84 mA away is refused by name
    /// and nothing moves. A current outside the compared span, a rest among
    /// them, is refused as not compared.
    #[test]
    fn a_current_changed_during_the_discharge_is_named() {
        let cell = Cell18650Pf::cell();
        assert_eq!(HELD_CURRENT_BAND_A, 0.00083);
        assert_eq!(CURRENT_COMPARED_MIN_A, 2.89817);
        assert_eq!(CURRENT_COMPARED_MAX_A, 2.90065);
        // the largest change in the saved validation runs, as 64-bit arithmetic reads it
        assert_eq!(2.89982_f64 - 2.899, 0.0008200000000000429);
        assert!(held_current_within_band(2.89982, 2.899));
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        assert_eq!(state.held_current_a, 0.0);
        step(&cell, &mut state, &mut frame, CellLoad { current_a: 2.899 }, 10.0).unwrap();
        assert_eq!(state.held_current_a, 2.899);
        for current_a in [2.89982, 2.899, 2.899_41, 2.898_2, 2.899_8, 2.89983, 2.89817] {
            step(&cell, &mut state, &mut frame, CellLoad { current_a }, 10.0).unwrap();
            assert_eq!(state.held_current_a, 2.899);
            assert_eq!(state.current_a, current_a);
        }
        let saved = state_bits(&state);
        let held = frame.to_bytes();
        for current_a in [2.899_84, 2.9, 2.900_65] {
            assert_eq!(
                step(&cell, &mut state, &mut frame, CellLoad { current_a }, 10.0),
                Err(StepReject::CurrentChangedDuringDischarge),
                "{current_a}"
            );
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
        }
        // 0.868_05 to 0.870_53 A: the held discharge 3541_dis5_10p.mat, compared and failed
        for current_a in [0.0, -0.0, 3.0, 2.0, 1.0, 0.145, 0.868_05, 0.868_88, 0.87, 0.869_7, 0.870_53, 2.898_16, 2.900_66, 3.5, -0.1] {
            assert_eq!(
                step(&cell, &mut state, &mut frame, CellLoad { current_a }, 10.0),
                Err(StepReject::CurrentNotCompared),
                "{current_a}"
            );
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
        }
        // a rest from full is a current that was never compared
        let mut rest = CellState::try_full(&cell).unwrap();
        assert_eq!(
            step(&cell, &mut rest, &mut frame, CellLoad { current_a: 0.0 }, 10.0),
            Err(StepReject::CurrentNotCompared)
        );
        // a held current that is not a number is a state that is not finite
        let mut broken = state;
        broken.held_current_a = f64::NAN;
        assert_eq!(
            step(&cell, &mut broken, &mut frame, CellLoad { current_a: 2.899 }, 10.0),
            Err(StepReject::NonFiniteState)
        );
        // a held current outside the compared span is refused as that
        let mut forged = state;
        forged.held_current_a = 1.0;
        assert_eq!(
            step(&cell, &mut forged, &mut frame, CellLoad { current_a: 2.899 }, 10.0),
            Err(StepReject::CurrentNotCompared)
        );
    }

    /// Every pair of decimal currents exactly 0.83 mA apart is inside the
    /// band, whatever the two currents, and every pair 0.84 mA apart is
    /// outside it. In 0.6.0, 2.899 then 2.89983 A was refused with a sentence
    /// that said 0.83 mA was more than 0.83 mA.
    #[test]
    fn a_change_of_exactly_the_band_in_decimal_is_inside_it_at_every_current() {
        for (low, high) in [
            (2.899, 2.89983),
            (2.89817, 2.899),
            (2.89982, 2.90065),
            (0.5, 0.50083),
            (1.0, 1.00083),
            (2.0, 2.00083),
            (2.99917, 3.0),
            (0.0, 0.00083),
        ] {
            assert!(held_current_within_band(high, low), "{low} to {high}");
            assert!(held_current_within_band(low, high), "{high} to {low}");
        }
        assert_eq!(2.89983_f64 - 2.899, 0.0008300000000001084);
        let mut index = 0u32;
        while index <= 24_800 {
            let low: f64 = std::format!("{:.7}", 2.89817 + f64::from(index) * 1.0e-7).parse().unwrap();
            let at_band: f64 = std::format!("{:.7}", low + 0.00083).parse().unwrap();
            let past_band: f64 = std::format!("{:.7}", low + 0.00084).parse().unwrap();
            let just_past: f64 = std::format!("{:.8}", low + 0.000_830_01).parse().unwrap();
            assert!(held_current_within_band(at_band, low), "{low} to {at_band}");
            assert!(held_current_within_band(low, at_band), "{at_band} to {low}");
            assert!(!held_current_within_band(past_band, low), "{low} to {past_band}");
            assert!(!held_current_within_band(low, past_band), "{past_band} to {low}");
            assert!(!held_current_within_band(just_past, low), "{low} to {just_past}");
            index += 1;
        }
    }

    /// Every discharge that passed started at full charge. A state below full
    /// charge with no held current is refused by name and nothing moves; the
    /// same state carrying the held current of the discharge it continues
    /// lands, matches the straight run bit for bit, and is held to that
    /// current's band.
    #[test]
    fn a_start_below_full_charge_needs_the_held_current_of_its_discharge() {
        let cell = Cell18650Pf::cell();
        let mut frame = blank_frame();
        for soc in [0.5, 0.7227957482602845, 0.999_999, SOC_MIN] {
            let mut state = CellState::try_soc(&cell, soc).unwrap();
            let saved = state_bits(&state);
            let held = frame.to_bytes();
            assert_eq!(
                step(&cell, &mut state, &mut frame, CellLoad { current_a: 2.9 }, 1.0),
                Err(StepReject::StartNotAtFullCharge),
                "{soc}"
            );
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
        }
        assert!(CellState::try_continued(&cell, 0.5, 1.0).is_none());
        assert!(CellState::try_continued(&cell, 0.5, 0.0).is_none());
        assert!(CellState::try_continued(&cell, 0.5, 2.900_66).is_none());
        assert!(CellState::try_continued(&cell, 1.0001, 2.9).is_none());

        let load = CellLoad { current_a: 2.9 };
        let mut straight = CellState::try_full(&cell).unwrap();
        for _ in 0..1_000 {
            step(&cell, &mut straight, &mut frame, load, 1.0).unwrap();
        }
        let mut continued = CellState::try_continued(&cell, straight.soc, 2.9).unwrap();
        assert_eq!(continued.held_current_a, 2.9);
        let saved = state_bits(&continued);
        for current_a in [2.89, 1.0, 0.0] {
            assert_eq!(
                step(&cell, &mut continued, &mut frame, CellLoad { current_a }, 1.0),
                Err(StepReject::CurrentNotCompared)
            );
        }
        assert_eq!(
            step(&cell, &mut continued, &mut frame, CellLoad { current_a: 2.89817 }, 1.0),
            Err(StepReject::CurrentChangedDuringDischarge)
        );
        assert_eq!(state_bits(&continued), saved);
        for _ in 0..100 {
            step(&cell, &mut straight, &mut frame, load, 1.0).unwrap();
            step(&cell, &mut continued, &mut frame, load, 1.0).unwrap();
        }
        assert_eq!(continued.soc.to_bits(), straight.soc.to_bits());
        assert_eq!(continued.terminal_v.to_bits(), straight.terminal_v.to_bits());
        assert_eq!(continued.held_current_a, 2.9);
    }

    /// Integrate and commit with no check of the current: the fitted tables
    /// at currents the card refuses, for the tests of the step's arithmetic.
    fn step_any_current(
        cell: &Cell18650Pf,
        state: &mut CellState,
        frame: &mut LastStateFrame64,
        load: CellLoad,
        dt_s: f64,
        soc_allowance: f64,
    ) -> Result<(), StepReject> {
        let predicted = integrate(cell, recorded(state), dt_s, load.current_a, soc_allowance)?;
        commit(cell, state, frame, load, dt_s, predicted)
    }

    /// A step that does not move the state of charge returns it as it was,
    /// on the bottom knot too: in 0.6.0 it was refused there as having left
    /// the card, which it had not.
    #[test]
    fn a_step_too_small_to_move_the_bottom_knot_stays_on_it() {
        let cell = Cell18650Pf::cell();
        let y = [SOC_MIN, 0.0, 0.0];
        for current_a in [1.0e-30, f64::MIN_POSITIVE, 1.0e-20] {
            let landed = integrate(&cell, y, 1.0, current_a, 0.0).unwrap();
            assert_eq!(landed[0].to_bits(), SOC_MIN.to_bits(), "{current_a}");
            let mut state = CellState::try_soc(&cell, SOC_MIN).unwrap();
            let mut frame = blank_frame();
            step_any_current(&cell, &mut state, &mut frame, CellLoad { current_a }, 1.0, 0.0)
                .unwrap();
            assert_eq!(state.soc.to_bits(), SOC_MIN.to_bits());
        }
        // a current that does move it below the knot is still refused
        assert_eq!(
            integrate(&cell, y, 1.0, 1.0, 0.0),
            Err(StepReject::StateOfChargeOutsideCard)
        );
    }

    /// A run whose end in decimal is exactly the bottom knot lands its last
    /// step. 1000 steps of 10.147752 s at 1.0 A remove 0.97 of the charge
    /// from full; summed in binary the last lands 2e-14 under 0.03, and
    /// without the allowance it was refused.
    #[test]
    fn a_run_ending_exactly_on_the_bottom_knot_lands_its_last_step() {
        let cell = Cell18650Pf::cell();
        let load = CellLoad { current_a: 1.0 };
        let dt_s = 10.147752;
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        for _ in 0..999 {
            let allowance = soc_sum_allowance(&state, dt_s);
            step_any_current(&cell, &mut state, &mut frame, load, dt_s, allowance).unwrap();
        }
        let before = state;
        assert_eq!(
            step_any_current(&cell, &mut state, &mut frame, load, dt_s, 0.0),
            Err(StepReject::StateOfChargeOutsideCard)
        );
        assert_eq!(state, before);
        let allowance = soc_sum_allowance(&state, dt_s);
        assert!(allowance > 2.0e-14 && allowance < 3.0e-13, "{allowance}");
        step_any_current(&cell, &mut state, &mut frame, load, dt_s, allowance).unwrap();
        assert_eq!(state.soc.to_bits(), SOC_MIN.to_bits());
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        // the next step has nowhere to go
        let allowance = soc_sum_allowance(&state, dt_s);
        assert_eq!(
            step_any_current(&cell, &mut state, &mut frame, load, dt_s, allowance),
            Err(StepReject::StateOfChargeOutsideCard)
        );
        // the allowance is capped
        let mut late = state;
        late.t_s = 1.0e12;
        assert_eq!(soc_sum_allowance(&late, 1.0e-5), SOC_SUM_ALLOWANCE_MAX);
        assert!(current_was_compared(2.9) && !current_was_compared(1.0));
    }

    fn line_vr(index: usize, soc: f64, current_a: f64) -> (f64, f64) {
        let u = soc - knot_soc(index);
        let slope_v =
            (OPEN_CIRCUIT_V[index + 1] - OPEN_CIRCUIT_V[index]) * TABLE_INTERVALS_PER_UNIT;
        let slope_r = (HELD_RESISTANCE_OHM[index + 1] - HELD_RESISTANCE_OHM[index])
            * TABLE_INTERVALS_PER_UNIT;
        let open_circuit_v = OPEN_CIRCUIT_V[index] + slope_v * u;
        let resistance = HELD_RESISTANCE_OHM[index] + slope_r * u;
        (open_circuit_v - current_a * resistance, resistance)
    }

    fn energy_of(soc: f64) -> f64 {
        if let Some(index) = super::exact_knot(soc) {
            return ENERGY_AT_KNOT_J[index];
        }
        let index = interval_index(soc).unwrap();
        let u = soc - knot_soc(index);
        let slope_v =
            (OPEN_CIRCUIT_V[index + 1] - OPEN_CIRCUIT_V[index]) * TABLE_INTERVALS_PER_UNIT;
        ENERGY_AT_KNOT_J[index]
            + CAPACITY_C * (OPEN_CIRCUIT_V[index] * u + slope_v * u * u * 0.5)
    }

    fn terminal_of(soc: f64, current_a: f64) -> f64 {
        if let Some(index) = super::exact_knot(soc) {
            return OPEN_CIRCUIT_V[index] - current_a * HELD_RESISTANCE_OHM[index];
        }
        let index = interval_index(soc).unwrap();
        line_vr(index, soc, current_a).0
    }

    /// Per-interval trapezoid. Inside one interval the powers are linear in time.
    fn closed_advance(
        soc: f64,
        mut work: f64,
        mut joule: f64,
        current_a: f64,
        dt_s: f64,
    ) -> (f64, f64, f64, f64, f64) {
        let s_target = soc - current_a * dt_s / CAPACITY_C;
        let mut s_now = soc;
        let mut time_left = dt_s;
        let mut guard = 0u32;
        loop {
            guard += 1;
            assert!(guard <= 195, "closed form did not finish");
            let interval = interval_index(s_now).unwrap();
            let s_j = knot_soc(interval);
            let time_to_lower = (s_now - s_j) * CAPACITY_C / current_a;
            let hit_knot = time_to_lower < time_left && s_j > s_target;
            let h = if hit_knot { time_to_lower } else { time_left };
            let s_end = if hit_knot { s_j } else { s_target };
            let (v0, r0) = line_vr(interval, s_now, current_a);
            let (v1, r1) = line_vr(interval, s_end, current_a);
            work += current_a * h * 0.5 * (v0 + v1);
            joule += current_a * current_a * h * 0.5 * (r0 + r1);
            if hit_knot {
                s_now = s_j;
                time_left -= h;
            } else {
                s_now = s_target;
                break;
            }
        }
        (s_now, work, joule, terminal_of(s_now, current_a), energy_of(s_now))
    }

    fn max_terminal_slope(current_a: f64) -> f64 {
        let mut slope = 0.0;
        let mut index = 0;
        while index < KNOTS - 1 {
            let slope_v =
                (OPEN_CIRCUIT_V[index + 1] - OPEN_CIRCUIT_V[index]) * TABLE_INTERVALS_PER_UNIT;
            let slope_r = (HELD_RESISTANCE_OHM[index + 1] - HELD_RESISTANCE_OHM[index])
                * TABLE_INTERVALS_PER_UNIT;
            let here = abs_f64(slope_v - current_a * slope_r);
            if here > slope {
                slope = here;
            }
            index += 1;
        }
        slope
    }

    fn rates_lookup(
        cell: &Cell18650Pf,
        soc: f64,
        current_a: f64,
    ) -> Result<[f64; 3], StepReject> {
        if !soc.is_finite() || !current_a.is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        if !(soc >= SOC_MIN && soc <= 1.0) {
            return Err(StepReject::StateOfChargeOutsideCard);
        }
        let (open_circuit_v, resistance_ohm, _) = match sample(cell, soc) {
            Some(value) => value,
            None => return Err(StepReject::StateOfChargeOutsideCard),
        };
        let terminal_v = open_circuit_v - current_a * resistance_ohm;
        let out = [
            -current_a / cell.capacity_c,
            terminal_v * current_a,
            current_a * current_a * resistance_ohm,
        ];
        if out[0].is_finite() && out[1].is_finite() && out[2].is_finite() {
            Ok(out)
        } else {
            Err(StepReject::NonFiniteState)
        }
    }

    fn rk4_lookup(
        cell: &Cell18650Pf,
        y: [f64; 3],
        dt_s: f64,
        current_a: f64,
    ) -> Result<[f64; 3], StepReject> {
        let k1 = rates_lookup(cell, y[0], current_a)?;
        let k2 = rates_lookup(cell, axpy(y, dt_s * 0.5, k1)[0], current_a)?;
        let k3 = rates_lookup(cell, axpy(y, dt_s * 0.5, k2)[0], current_a)?;
        let k4 = rates_lookup(cell, axpy(y, dt_s, k3)[0], current_a)?;
        rk4_combine(y, dt_s, k1, k2, k3, k4)
    }

    /// Plain RK4 across knots, with a table lookup at each stage.
    fn step_plain(
        cell: &Cell18650Pf,
        state: &mut CellState,
        frame: &mut LastStateFrame64,
        load: CellLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(cell, state, load, dt_s)?;
        let predicted = rk4_lookup(cell, recorded(state), dt_s, load.current_a)?;
        commit(cell, state, frame, load, dt_s, predicted)
    }

    fn step_rectangle(
        cell: &Cell18650Pf,
        state: &mut CellState,
        frame: &mut LastStateFrame64,
        load: CellLoad,
        dt_s: f64,
        at_landing: bool,
    ) -> Result<(), StepReject> {
        validate(cell, state, load, dt_s)?;
        let y = recorded(state);
        let interval = interval_index(y[0]).ok_or(StepReject::StateOfChargeOutsideCard)?;
        let line = line_of(cell, interval, load.current_a);
        let start = rates_on_line(&line, y[0])?;
        let mut predicted = axpy(y, dt_s, start);
        if at_landing {
            let landing = rates_lookup(cell, predicted[0], load.current_a)?;
            predicted[1] = y[1] + dt_s * landing[1];
            predicted[2] = y[2] + dt_s * landing[2];
        }
        commit(cell, state, frame, load, dt_s, predicted)
    }

    fn step_bias_work(
        cell: &Cell18650Pf,
        state: &mut CellState,
        frame: &mut LastStateFrame64,
        load: CellLoad,
        dt_s: f64,
        extra_j: f64,
    ) -> Result<(), StepReject> {
        validate(cell, state, load, dt_s)?;
        let mut predicted = integrate(cell, recorded(state), dt_s, load.current_a, 0.0)?;
        predicted[1] += extra_j;
        commit(cell, state, frame, load, dt_s, predicted)
    }

    #[test]
    fn constants_are_locked() {
        assert_copy::<Cell18650Pf>();
        assert_copy::<CellState>();
        assert_copy::<CellLoad>();
        assert_copy::<StepReject>();
        let cell = Cell18650Pf::cell();
        let mut state = CellState::try_full(&cell).unwrap();
        let saved = state;
        let mut frame = blank_frame();
        let held = frame.to_bytes();

        let mut tampered = cell;
        tampered.open_circuit_v[10] += 0.001;
        assert!(!tampered.is_valid());
        let got = step(&tampered, &mut state, &mut frame, CellLoad { current_a: 1.0 }, 1.0);
        assert_eq!(got, Err(StepReject::InvalidCell));
        assert_eq!(state, saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        let mut tampered = cell;
        tampered.capacity_c += 1.0;
        assert!(!tampered.is_valid());
        let got = step(&tampered, &mut state, &mut frame, CellLoad { current_a: 1.0 }, 1.0);
        assert_eq!(got, Err(StepReject::InvalidCell));
        assert_eq!(state, saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);
    }

    #[test]
    fn tables_are_sane() {
        let cell = Cell18650Pf::cell();
        assert_eq!(KNOTS, 195);
        assert_eq!(OPEN_CIRCUIT_V.len(), 195);
        assert_eq!(HELD_RESISTANCE_OHM.len(), 195);
        assert_eq!(OPEN_CIRCUIT_V[0].to_bits(), 3.2654_f64.to_bits());
        assert_eq!(HELD_RESISTANCE_OHM[0].to_bits(), 0.6950_f64.to_bits());
        assert_eq!(OPEN_CIRCUIT_V[KNOTS - 1].to_bits(), 4.1827_f64.to_bits());
        assert_eq!(HELD_RESISTANCE_OHM[KNOTS - 1].to_bits(), 0.0586_f64.to_bits());
        assert_eq!(CAPACITY_C.to_bits(), 10461.6_f64.to_bits());
        assert_eq!(SOC_MIN.to_bits(), 0.03_f64.to_bits());
        assert_eq!(cell.gate_dt_s().unwrap().to_bits(), 17.436_f64.to_bits());
        assert!(CUTOFF_V < OPEN_CIRCUIT_V[0]);
        let mut index = 0;
        while index < KNOTS - 1 {
            assert!(OPEN_CIRCUIT_V[index + 1] > OPEN_CIRCUIT_V[index]);
            assert!(HELD_RESISTANCE_OHM[index] > 0.0);
            index += 1;
        }
        assert!(HELD_RESISTANCE_OHM[KNOTS - 1] > 0.0);
        let target = 37720.4155758_f64;
        let relative = abs_f64(CELL_ENERGY_J - target) / target;
        assert!(
            relative <= 1.0e-12,
            "CELL_ENERGY_J {CELL_ENERGY_J} relative {relative}"
        );
    }

    #[test]
    fn table_matches_the_c20_discharge() {
        let cell = Cell18650Pf::cell();
        let points = [
            (1.00, 4.1742),
            (0.95, 4.0944),
            (0.90, 4.0538),
            (0.80, 3.9463),
            (0.70, 3.8601),
            (0.60, 3.7699),
            (0.50, 3.6657),
            (0.40, 3.6016),
            (0.30, 3.5446),
            (0.20, 3.4612),
            (0.10, 3.3310),
            (0.05, 3.2561),
            (0.03, 3.1646),
        ];
        let current_a = 0.14496;
        let mut index = 0;
        while index < points.len() {
            let (soc, expect) = points[index];
            assert!(CellState::try_soc(&cell, soc).is_some(), "soc {soc}");
            let terminal = terminal_of(soc, current_a);
            assert!(
                (terminal - expect).abs() <= 0.0002,
                "soc {soc} terminal {terminal} expect {expect}"
            );
            index += 1;
        }
    }

    #[test]
    fn held_current_discharge_matches_the_closed_form() {
        let cell = Cell18650Pf::cell();
        let dt = 1.0;
        let current = 2.9;
        let slope_v = max_terminal_slope(current);
        let u = f64::EPSILON * 0.5;
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        let load = CellLoad { current_a: current };
        let mut cf_soc = 1.0;
        let mut cf_work = 0.0;
        let mut cf_joule = 0.0;
        let mut work_tol_j = 0.0;
        let mut joule_tol_j = 0.0;
        let mut n = 0u32;
        while n < 3_427 {
            step(&cell, &mut state, &mut frame, load, dt).unwrap();
            n += 1;
            let (soc, work, joule, terminal, energy) =
                closed_advance(cf_soc, cf_work, cf_joule, current, dt);
            cf_soc = soc;
            cf_work = work;
            cf_joule = joule;
            let soc_tol = (f64::from(n) + 8.0) * u;
            let v_tol = slope_v * soc_tol + 8.0 * u * OPEN_CIRCUIT_V[KNOTS - 1];
            work_tol_j += u * abs_f64(work) + current * slope_v * dt * soc_tol;
            joule_tol_j += u * abs_f64(joule) + current * current * 1.0 * dt * soc_tol;
            let work_tol = work_tol_j + 16.0 * u * abs_f64(work);
            let joule_tol = joule_tol_j + 16.0 * u * abs_f64(joule);
            let energy_tol = CAPACITY_C * OPEN_CIRCUIT_V[KNOTS - 1] * soc_tol
                + 8.0 * u * CELL_ENERGY_J;
            assert!(
                (state.soc - soc).abs() <= soc_tol,
                "step {n} soc {} closed {soc}",
                state.soc
            );
            assert!(
                (state.terminal_v - terminal).abs() <= v_tol,
                "step {n} terminal {} closed {terminal}",
                state.terminal_v
            );
            assert!(
                (state.electrical_work_j - work).abs() <= work_tol,
                "step {n} work {} closed {work}",
                state.electrical_work_j
            );
            assert!(
                (state.joule_j - joule).abs() <= joule_tol,
                "step {n} heat {} closed {joule}",
                state.joule_j
            );
            assert!(
                (state.chemical_energy_j - energy).abs() <= energy_tol,
                "step {n} energy {} closed {energy}",
                state.chemical_energy_j
            );
        }
        assert_eq!(n, 3_427);
        assert!((state.soc - 0.050021).abs() <= 5.0e-7, "soc {}", state.soc);
        assert!(
            (state.terminal_v - 2.50226).abs() <= 1.0e-5,
            "terminal {}",
            state.terminal_v
        );
        assert!(
            (state.electrical_work_j - 34_849.41).abs() <= 0.01,
            "work {}",
            state.electrical_work_j
        );
        assert!(
            (state.joule_j - 2_183.13).abs() <= 0.01,
            "heat {}",
            state.joule_j
        );
        let ampere_hours = current * state.t_s / 3_600.0;
        assert!(
            (ampere_hours - 2.76064).abs() <= 1.0e-5,
            "Ah {ampere_hours}"
        );
        proof_intact(&frame);
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();
        let got = step(&cell, &mut state, &mut frame, load, dt);
        assert_eq!(got, Err(StepReject::UndervoltageCutoff));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        proof_intact(&frame);
    }

    #[test]
    fn steps_that_cross_knots_conserve() {
        let cell = Cell18650Pf::cell();
        let dt = 60.0;
        let load = CellLoad { current_a: 2.9 };
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        step(&cell, &mut state, &mut frame, load, dt).unwrap();
        assert!(
            1.0 - state.soc > 3.0 * 0.005,
            "a 60 s step crossed soc down to {}",
            state.soc
        );
        assert!(
            state.residual_conservation_error < 1.0e-12,
            "step 1 residual {}",
            state.residual_conservation_error
        );
        let mut n = 1u32;
        while n < 57 {
            step(&cell, &mut state, &mut frame, load, dt).unwrap();
            assert!(
                state.residual_conservation_error < 1.0e-12,
                "step {n} residual {}",
                state.residual_conservation_error
            );
            n += 1;
        }
        assert_eq!(n, 57);
        assert!(state.residual_conservation_error < 1.0e-12);
        proof_intact(&frame);
        let got = step(&cell, &mut state, &mut frame, load, dt);
        assert_eq!(got, Err(StepReject::UndervoltageCutoff));

        let mut attack = CellState::try_full(&cell).unwrap();
        let mut attack_frame = blank_frame();
        let mut landed = 0u32;
        loop {
            assert!(landed < 10, "plain RK4 was not refused within ten steps");
            let held = state_bits(&attack);
            let held_frame = attack_frame.to_bytes();
            match step_plain(&cell, &mut attack, &mut attack_frame, load, dt) {
                Ok(()) => landed += 1,
                Err(StepReject::FirstLawBreach) => {
                    assert_eq!(state_bits(&attack), held);
                    assert_eq!(attack_frame.to_bytes(), held_frame);
                    proof_intact(&attack_frame);
                    break;
                }
                Err(other) => panic!("plain RK4 refused with {other:?} after {landed}"),
            }
        }
    }

    #[test]
    fn rectangle_rule_breaches_the_ledger() {
        let cell = Cell18650Pf::cell();
        let dt = 10.0;
        let load = CellLoad { current_a: 2.9 };
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        step(&cell, &mut state, &mut frame, load, dt).unwrap();
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();

        let got = step_rectangle(&cell, &mut state, &mut frame, load, dt, false);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        let got = step_rectangle(&cell, &mut state, &mut frame, load, dt, true);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        proof_intact(&frame);
        let gate_j = RESIDUAL_CONSERVATION_LIMIT * CELL_ENERGY_J;
        assert!(gate_j > 0.037 && gate_j < 0.038, "gate {gate_j}");
    }

    #[test]
    fn explicit_euler_adds_up_to_a_refusal() {
        let cell = Cell18650Pf::cell();
        let dt = 1.0;
        let load = CellLoad { current_a: 2.9 };
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        let mut previous = 0.0;
        let mut landed = 0u32;
        loop {
            let held = state_bits(&state);
            let held_frame = frame.to_bytes();
            match step_rectangle(&cell, &mut state, &mut frame, load, dt, false) {
                Ok(()) => {
                    landed += 1;
                    assert!(
                        state.energy_defect_j > previous,
                        "step {landed} defect {}",
                        state.energy_defect_j
                    );
                    previous = state.energy_defect_j;
                    assert!(landed < 80, "euler did not refuse");
                }
                Err(StepReject::SpontaneousGeneration) => {
                    assert!(landed > 10, "refused after only {landed} steps");
                    assert_eq!(state_bits(&state), held);
                    assert_eq!(frame.to_bytes(), held_frame);
                    proof_intact(&frame);
                    break;
                }
                Err(other) => panic!("euler refused with {other:?}"),
            }
        }
    }

    #[test]
    fn biased_ledger_rolls_back() {
        let cell = Cell18650Pf::cell();
        let dt = 1.0;
        let load = CellLoad { current_a: 2.9 };
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        step(&cell, &mut state, &mut frame, load, dt).unwrap();
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();
        let extra = CELL_ENERGY_J * 1.0e-3;
        let got = step_bias_work(&cell, &mut state, &mut frame, load, dt, extra);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        let got = step_bias_work(&cell, &mut state, &mut frame, load, dt, -extra);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        proof_intact(&frame);
    }

    #[test]
    fn edges_of_the_card_are_refused_by_name() {
        let cell = Cell18650Pf::cell();
        let mut state = CellState::try_full(&cell).unwrap();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let mut refuse = |load: CellLoad, dt_s: f64, expect: StepReject| {
            let got = step(&cell, &mut state, &mut frame, load, dt_s);
            assert_eq!(got, Err(expect));
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            proof_intact(&frame);
        };
        refuse(
            CellLoad { current_a: -0.1 },
            1.0,
            StepReject::CurrentNotCompared,
        );
        refuse(
            CellLoad { current_a: 3.0001 },
            1.0,
            StepReject::CurrentNotCompared,
        );
        refuse(CellLoad::default(), 1.0, StepReject::CurrentNotCompared);
        refuse(CellLoad::default(), 0.0, StepReject::NonPositiveDt);
        refuse(CellLoad::default(), -1.0, StepReject::NonPositiveDt);
        refuse(CellLoad::default(), f64::NAN, StepReject::NonFiniteInput);
        refuse(
            CellLoad {
                current_a: f64::INFINITY,
            },
            1.0,
            StepReject::NonFiniteInput,
        );
        refuse(
            CellLoad { current_a: 2.9 },
            3_600.0,
            StepReject::StateOfChargeOutsideCard,
        );
        state.soc = f64::NAN;
        let nan_bits = state_bits(&state);
        let got = step(&cell, &mut state, &mut frame, CellLoad::default(), 1.0);
        assert_eq!(got, Err(StepReject::NonFiniteState));
        assert_eq!(state_bits(&state), nan_bits);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        assert!(CellState::try_soc(&cell, 0.0299).is_none());
        assert!(CellState::try_soc(&cell, 1.0001).is_none());

        for current_a in [CURRENT_COMPARED_MIN_A, CURRENT_COMPARED_MAX_A] {
            let mut on_limit = CellState::try_full(&cell).unwrap();
            step(&cell, &mut on_limit, &mut frame, CellLoad { current_a }, 1.0).unwrap();
            proof_intact(&frame);
        }
        for current_a in [
            f64::from_bits(CURRENT_COMPARED_MIN_A.to_bits() - 1),
            f64::from_bits(CURRENT_COMPARED_MAX_A.to_bits() + 1),
        ] {
            let mut past_limit = CellState::try_full(&cell).unwrap();
            assert_eq!(
                step(&cell, &mut past_limit, &mut frame, CellLoad { current_a }, 1.0),
                Err(StepReject::CurrentNotCompared)
            );
        }
    }

    /// The card takes 1C only. The two endings of the fitted tables, the
    /// bottom knot at low current and the cutoff at a higher one, are still
    /// checked here through the arithmetic of the step, with no check of the
    /// current.
    #[test]
    fn two_endings() {
        let cell = Cell18650Pf::cell();
        let mut frame = blank_frame();
        for current_a in [0.869, 1.45] {
            let mut state = CellState::try_full(&cell).unwrap();
            assert_eq!(
                step(&cell, &mut state, &mut frame, CellLoad { current_a }, 1.0),
                Err(StepReject::CurrentNotCompared)
            );
        }
        let mut low = CellState::try_full(&cell).unwrap();
        let load = CellLoad { current_a: 0.869 };
        let mut landed = 0u32;
        loop {
            let allowance = soc_sum_allowance(&low, 1.0);
            match step_any_current(&cell, &mut low, &mut frame, load, 1.0, allowance) {
                Ok(()) => {
                    landed += 1;
                    assert!(landed <= 11_677, "low current did not refuse");
                }
                Err(StepReject::StateOfChargeOutsideCard) => break,
                Err(other) => panic!("0.869 A refused with {other:?} after {landed}"),
            }
        }
        assert_eq!(landed, 11_677);
        assert_eq!(low.t_s, 11_677.0);
        let ampere_hours = 0.869 * low.t_s / 3_600.0;
        assert!(
            (ampere_hours - 2.8187).abs() <= 5.0e-5,
            "Ah {ampere_hours}"
        );
        assert!(
            (low.terminal_v - 2.6626).abs() <= 5.0e-5,
            "terminal {}",
            low.terminal_v
        );
        proof_intact(&frame);

        let mut high = CellState::try_full(&cell).unwrap();
        let load = CellLoad { current_a: 1.45 };
        let mut landed = 0u32;
        loop {
            let allowance = soc_sum_allowance(&high, 1.0);
            match step_any_current(&cell, &mut high, &mut frame, load, 1.0, allowance) {
                Ok(()) => {
                    landed += 1;
                    assert!(landed <= 6_958, "1.45 A did not refuse");
                }
                Err(StepReject::UndervoltageCutoff) => break,
                Err(other) => panic!("1.45 A refused with {other:?} after {landed}"),
            }
        }
        assert_eq!(landed, 6_958);
        assert_eq!(high.t_s, 6_958.0);
        let ampere_hours = 1.45 * high.t_s / 3_600.0;
        assert!(
            (ampere_hours - 2.8025).abs() <= 5.0e-5,
            "Ah {ampere_hours}"
        );
        proof_intact(&frame);
    }

    #[test]
    fn open_circuit_holds_and_the_warning_is_only_a_flag() {
        let cell = Cell18650Pf::cell();
        // the step's arithmetic holds a rest exactly; the card refuses one
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        let mut index = 0;
        let lengths = [1.0, 10.0, 17.436, 1.0e6];
        while index < lengths.len() {
            assert_eq!(
                step(&cell, &mut state, &mut frame, CellLoad::default(), lengths[index]),
                Err(StepReject::CurrentNotCompared)
            );
            step_any_current(&cell, &mut state, &mut frame, CellLoad::default(), lengths[index], 0.0)
                .unwrap();
            assert_eq!(state.soc.to_bits(), 1.0_f64.to_bits());
            assert_eq!(state.terminal_v.to_bits(), 4.1827_f64.to_bits());
            assert!(!state.sag_warning);
            assert_eq!(frame.flags, 0);
            proof_intact(&frame);
            index += 1;
        }

        let mut state = CellState::try_full(&cell).unwrap();
        step(
            &cell,
            &mut state,
            &mut frame,
            CellLoad { current_a: 2.9 },
            1.0,
        )
        .unwrap();
        assert!(state.terminal_v > WARN_V);
        assert!(!state.sag_warning);
        assert_eq!(frame.flags, 0);

        let mut low = CellState::try_continued(&cell, 0.06, 2.9).unwrap();
        step(&cell, &mut low, &mut frame, CellLoad { current_a: 2.9 }, 1.0).unwrap();
        assert!(low.terminal_v > CUTOFF_V && low.terminal_v < WARN_V);
        assert!(low.sag_warning);
        assert_eq!(frame.flags, FLAG_SAG_WARNING);
        proof_intact(&frame);
    }

    #[test]
    fn the_proof_is_not_the_cards() {
        let cell = Cell18650Pf::cell();
        let mut state = CellState::try_full(&cell).unwrap();
        let mut frame = blank_frame();
        let load = CellLoad { current_a: 2.9 };
        let mut n = 0;
        while n < 4 {
            step(&cell, &mut state, &mut frame, load, 1.0).unwrap();
            proof_intact(&frame);
            n += 1;
        }
        let got = step(
            &cell,
            &mut state,
            &mut frame,
            CellLoad { current_a: -0.1 },
            1.0,
        );
        assert_eq!(got, Err(StepReject::CurrentNotCompared));
        proof_intact(&frame);
        let got = step(&cell, &mut state, &mut frame, load, 0.0);
        assert_eq!(got, Err(StepReject::NonPositiveDt));
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = super::Cell18650Pf::cell();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );

        // The cell at 80 percent, in a 1C discharge.
        let mut state = super::CellState::try_continued(&card, 0.8, 2.9).unwrap();
        let load = super::CellLoad { current_a: 2.9 };
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

    /// The card refuses a tiny current. The step's arithmetic still holds
    /// one: at the floor, the charge moves by what it should or stays put.
    #[test]
    fn the_smallest_normal_positive_current_is_refused_and_the_arithmetic_keeps_the_charge() {
        let card = Cell18650Pf::cell();
        let floor_s = card.floor_dt_s().unwrap();
        for current_a in [f64::MIN_POSITIVE, 1.0e-306, 1.0e-304, 1.0e-300, 1.0e-12] {
            let mut full = CellState::try_full(&card).unwrap();
            let mut frame = blank_frame();
            assert_eq!(
                step(&card, &mut full, &mut frame, CellLoad { current_a }, floor_s),
                Err(StepReject::CurrentNotCompared)
            );
            for soc in [1.0, 0.8, 0.5123, SOC_MIN + 0.001, SOC_MIN] {
                let mut state = CellState::try_soc(&card, soc).unwrap();
                let before = state;
                let load = CellLoad { current_a };
                step_any_current(&card, &mut state, &mut frame, load, floor_s, 0.0)
                    .unwrap_or_else(|reject| panic!("{current_a} A at {soc}: {reject:?}"));
                let moved = current_a * floor_s / CAPACITY_C;
                assert!((before.soc - state.soc - moved).abs() <= 1.0e-15 * before.soc);
                assert!(state.electrical_work_j >= 0.0 && state.electrical_work_j <= 4.5 * current_a * floor_s, "work {} at {current_a} A and {soc}", state.electrical_work_j);
                assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                proof_intact(&frame);
            }
        }
        // A 1C step across a knot is split at the knot as before.
        let mut state = CellState::try_continued(&card, 0.700001, 2.9).unwrap();
        let mut frame = blank_frame();
        step(&card, &mut state, &mut frame, CellLoad { current_a: 2.9 }, card.gate_dt_s().unwrap()).unwrap();
        assert!(state.soc < 0.7 && state.soc > 0.695);
    }
}
