#![no_implicit_prelude]

//! One Tesla Model 3 2170 lithium-ion cell, cell m80, as it stood in each of
//! six blocks of tests, under a terminal current and a measured case
//! temperature.
//!
//! Measured by Kollmeyer, Khanum, Naguib and Emadi, "Tesla Model 3 2170
//! Li-ion Cell Dataset and Battery SOC Estimation Blind Modeling Tool",
//! Borealis, doi: 10.5683/SP3/ZVTR4B, version 3.0. The authors ask for this
//! reference: P. J. Kollmeyer, M. Naguib, F. Khanum and A. Emadi, "A Blind
//! Modeling Tool for Standardized Evaluation of Battery State of Charge
//! Estimation Algorithms," 2022 IEEE ITEC, pp. 243-248,
//! doi: 10.1109/ITEC53557.2022.9813996.
//!
//! The card returns the voltage at the cell's terminals, given the current
//! and the temperature measured on the cell's case. It has no model of
//! temperature and does not predict it. A block is the set of tests run at
//! one chamber setting: 40, 25, 10, 0, -10 and -20 degC. The block of a run
//! picks the capacity, the rest-voltage table and the range of charge. The
//! resistances and the time constant are read from all six blocks at the
//! measured case temperature. The tables are in `cell_2170_m80_tables.rs`.
//!
//! In the 25, 10, 0, -10 and -20 degC blocks the capacity is the charge of a
//! capacity test that began after a top-up, 33, 22, 48, 52 and 39 mAh above the
//! laboratory's full (0.73, 0.52, 1.11, 1.23 and 0.92 percent of the block's
//! capacity); in the 40 degC block that test began at full. In those five
//! blocks `charge_level` is therefore not the state of charge counted from the
//! laboratory's full: it is 1 at full and reads up to that share high toward
//! empty. The tables were fitted with these capacities, and the comparison is
//! of the card as fitted.
//!
//! The comparison. The card was compared with measurement in the 60
//! held-back tests of the cell's protocol (12 held discharges, 48 drive
//! cycles), on 2026-10-08, and failed: 27 of 60 met their marks. C/3 met at
//! 40, 25, 10 and 0 degC (5.0, 6.4, 15.0 and 29.2 mV rms against 42); 1C
//! missed at every temperature (48.2, 44.1, 51.2 and 88.2 mV from 40 to 0
//! degC); drive cycles from 0 to 40 degC passed 23 of 32. The -10 and -20
//! degC blocks met no mark in their 20 tests: at -10 degC, C/3 55.6 mV rms
//! and +0.68 % capacity, 1C 134.2 mV and +6.92 %, no drive cycle; at -20
//! degC, C/3 139.2 mV and +7.09 %, 1C 207.8 mV and +15.85 %, no drive cycle,
//! reading far too high from the first seconds, four cycles stopping on the
//! first charging pulse at 4.242 V. Voltage errors in those two blocks ran
//! 55.6 to 425 mV. Through engine 0.9.0 those blocks were refused, and each
//! block was held to the case temperatures and currents of its own tests;
//! all six blocks are now served, and a step in either cold block lands
//! with [`FLAG_BLOCK_MET_NO_MARK`], a step outside its block's compared case
//! temperatures, [`BLOCK_CASE_MIN_C`] to [`BLOCK_CASE_MAX_C`], or currents,
//! [`BLOCK_CURRENT_DISCHARGE_MAX_A`] and [`BLOCK_CURRENT_CHARGE_MAX_A`], with
//! [`FLAG_OUTSIDE_COMPARED_TESTS`].
//!
//! Where the tables end, the card stops. A case temperature beyond the
//! coldest or the warmest block's own tests, [`CASE_TEMPERATURE_MIN_C`] to
//! [`CASE_TEMPERATURE_MAX_C`], is [`StepReject::CaseTemperatureOutsideCard`]:
//! past them every resistance and the time constant would hold the end
//! block's value, and the card has no temperature law of its own. A current
//! larger than 13.5 A either way, the top of the pulse test and the end of
//! the ten-second table's current axis, is [`StepReject::CurrentOutsideCard`]:
//! past it the table would hold its end value. A charging current above
//! 2.25 A is read on the discharge side of the tables, which were fitted on
//! pulses to 13.5 A discharging and 2.25 A charging.
//!
//! The same cell type is served as a physical model with memory,
//! `cell_2170_model3`. On the same discharge, cell m80 in its 10 degC block
//! from the recharge at 1.5 A for 600 s with the case at 12 degC, this card
//! gives 3.934 V and that one 3.917 V: 17.4 mV apart.
//!
//! The ends of the tables and rounding. The charge removed is a running sum
//! in 64-bit arithmetic, and each addition rounds by at most half a spacing
//! of 64-bit numbers below 8 Ah, 2^-51 Ah; the current over 3600 rounds by
//! less than 2^-53 of itself. A run whose end in decimal is exactly an end of
//! a table can reach its last step a few spacings past it. The comparison
//! with either end carries [`Cell2170M80::charge_end_allowance_ah`]:
//! [`CHARGE_END_ALLOWANCE_PER_STEP_AH`], 2^-50 Ah, for each step landed and
//! the step being taken, and [`CHARGE_END_ALLOWANCE_EXTRA_STEPS`], two, more:
//! one for the start and one for the end of the table, each read from a
//! decimal. A landing is held to no more than half the charge of its own
//! step past the end; a rest, or a step back toward the tables, from a state
//! past an end lands. Within the allowance the charge keeps its own value
//! and every table is read at the end point. A start carries no landed step,
//! so [`CellState::try_start`] takes a charge within
//! `charge_end_allowance_ah(0)`, 2^-49 Ah, of either end, read at the end
//! point. A run that ended further past an end by rounding returns a charge
//! from which no new run starts; it starts from the end itself.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, terminal voltage, charge removed, current,
//! rest voltage, the voltage of the relaxing branch, the case temperature,
//! the margin to the lower voltage limit, and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.
//!
//! `I > 0` charges and `I < 0` discharges, as in the measurements. A step is
//! exactly one second. Over a step the current and the case temperature are
//! held, and every table is read at the charge the step lands on.
//!
//! ```text
//! q' = q - I dt / 3600                    ampere-hours removed
//! s  = 1 - q' / capacity of the block     charge level
//! V_rest      straight lines between the block's points, read at q'
//! held, first-second, ten-second(|I|), ten-second(1.35 A), ten-second(2.25 A)
//!             read in each block along s with straight lines, end values
//!             held; ten-second also along the size of the current, limited
//!             to [0.5625, 13.5] A; then brought together at the case
//!             temperature: the logarithm in a straight line between the two
//!             blocks whose own temperatures at s lie either side, the
//!             warmest or the coldest block's value beyond them
//! tau         brought together the same way
//! R_once = first-second + ten-second(|I|) - ten-second(2.25 A), not less than first-second / 5
//! R_slow = held - first-second - (ten-second(1.35 A) - ten-second(2.25 A)), not less than 0
//! a  = exp(-dt / tau)
//! u' = a u + (1 - a) R_slow I             voltage of the relaxing branch
//! V  = V_rest + I R_once + u'
//! ```
//!
//! The ledger is an energy ledger. Along a step the voltage is taken as the
//! rest voltage at the charge passing through, plus `I R_once`, plus the
//! branch voltage on its way from `u` to `u'`.
//!
//! ```text
//! E(q) = 3600 * integral of V_rest from q to the block's last point     chemical energy
//! W_elec  += integral of V I dt along the step: the rest voltage piece by
//!            piece between the table's points, I^2 R_once dt, and
//!            I (u_inf dt + (u - u_inf) tau (1 - a)), with u_inf = R_slow I
//! W_once  += I^2 R_once dt
//! W_branch += I^2 R_slow dt - I tau (u' - u)      what the branch took, from the change in its voltage
//! residual = |E - E_anchor + W_once + W_branch - W_elec| / energy of the block's table
//! ```
//!
//! `W_elec` is the work done on the cell at its terminals: negative while
//! the cell discharges. [`Cell2170M80::gate_dt_s`] and
//! [`Cell2170M80::floor_dt_s`] are both one second, and [`step`] refuses any
//! other step by name.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;
use crate::math::{exp, exp_m1, log};

::core::include!("cell_2170_m80_tables.rs");

/// The laboratory ends every discharge here.
pub const LOWER_VOLTAGE_LIMIT_V: f64 = 2.8;
/// The laboratory's charging limit of 4.2 V plus 0.042 V.
pub const UPPER_VOLTAGE_LIMIT_V: f64 = 4.242;
/// The coldest case temperature in a compared test of any block: -21.898 degC, in the -20 degC block. Past it the tables would hold the coldest block's values.
pub const CASE_TEMPERATURE_MIN_C: f64 = -21.898;
/// The warmest case temperature in a compared test of any block: 45.535 degC, in the 40 degC block. Past it the tables would hold the warmest block's values.
pub const CASE_TEMPERATURE_MAX_C: f64 = 45.535;
/// The coldest case temperature in the compared tests of each block, in degC, rounded down to the millikelvin.
pub const BLOCK_CASE_MIN_C: [f64; BLOCK_COUNT] = [39.063, 24.267, 10.201, 0.808, -11.995, -21.898];
/// The warmest case temperature in the compared tests of each block, in degC, rounded up to the millikelvin.
pub const BLOCK_CASE_MAX_C: [f64; BLOCK_COUNT] = [45.535, 30.993, 21.464, 16.259, 8.389, 2.885];
/// The blocks in which no compared test met its marks: the -10 and -20 degC blocks.
pub const BLOCK_MET_NO_MARK: [bool; BLOCK_COUNT] = [false, false, false, false, true, true];
/// The largest discharging current the card takes: 13.5 A, the top of the pulse test and the end of the ten-second table's current axis. The largest in a compared test was 11.374274 A.
pub const CURRENT_DISCHARGE_MAX_A: f64 = 13.5;
/// The largest charging current the card takes: 13.5 A, read on the discharge side of the tables above 2.25 A. The largest in a compared test was 9.73669 A.
pub const CURRENT_CHARGE_MAX_A: f64 = 13.5;
/// The largest discharging current in a compared test: 11.374274 A, in a drive cycle of the 0 degC block, rounded up to the milliampere.
pub const CURRENT_COMPARED_DISCHARGE_MAX_A: f64 = 11.375;
/// The largest charging current in a compared test: 9.73669 A, in a drive cycle of the 40 degC block, rounded up to the milliampere.
pub const CURRENT_COMPARED_CHARGE_MAX_A: f64 = 9.737;
/// The largest discharging current in the compared tests of each block, in A, rounded up to the milliampere.
pub const BLOCK_CURRENT_DISCHARGE_MAX_A: [f64; BLOCK_COUNT] = [9.809, 9.23, 10.079, 11.375, 10.467, 10.761];
/// The largest charging current in the compared tests of each block, in A, rounded up to the milliampere.
pub const BLOCK_CURRENT_CHARGE_MAX_A: [f64; BLOCK_COUNT] = [9.737, 9.484, 8.776, 8.304, 8.006, 7.638];
/// One spacing of 64-bit numbers between 4 and 8, 2^-50 Ah: the allowance at an end of the tables for each step.
pub const CHARGE_END_ALLOWANCE_PER_STEP_AH: f64 = 8.881784197001252e-16;
/// Steps added to the count in [`Cell2170M80::charge_end_allowance_ah`]: the start and the end of the table, each read from a decimal.
pub const CHARGE_END_ALLOWANCE_EXTRA_STEPS: f64 = 2.0;
/// The current of the holds of the pulse test.
pub const HOLD_CURRENT_A: f64 = 1.35;
/// The reference pulse of the pulse test.
pub const PULSE_CURRENT_A: f64 = 2.25;
/// The part that acts at once is not less than this share of the first-second resistance.
pub const AT_ONCE_FLOOR_SHARE: f64 = 0.2;
/// The one step the card takes. The first-second table has no meaning at another.
pub const STEP_S: f64 = 1.0;
/// Seconds in an hour: ampere-hours to ampere-seconds.
pub const SECONDS_PER_HOUR: f64 = 3_600.0;
/// `|E - E_anchor + W_once + W_branch - W_elec| / BLOCK_ENERGY_J` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;

/// The step is in the -10 or the -20 degC block, where no compared test met its marks. The step still lands.
pub const FLAG_BLOCK_MET_NO_MARK: u64 = 1 << 0;
/// The step's case temperature or current lies outside those of its block's own compared tests. The step still lands.
pub const FLAG_OUTSIDE_COMPARED_TESTS: u64 = 1 << 1;

const fn energy_at_rest_points() -> [f64; REST_TOTAL] {
    let mut energy = [0.0; REST_TOTAL];
    let mut block = 0;
    while block < BLOCK_COUNT {
        let first = REST_FIRST[block];
        let mut index = first + REST_POINTS[block] - 1;
        while index > first {
            let low = index - 1;
            energy[low] = energy[index]
                + SECONDS_PER_HOUR
                    * (REST_CHARGE_AH[index] - REST_CHARGE_AH[low])
                    * 0.5
                    * (REST_V[low] + REST_V[index]);
            index -= 1;
        }
        block += 1;
    }
    energy
}

const fn block_energies() -> [f64; BLOCK_COUNT] {
    let mut energy = [0.0; BLOCK_COUNT];
    let mut block = 0;
    while block < BLOCK_COUNT {
        energy[block] = ENERGY_AT_REST_POINT_J[REST_FIRST[block]];
        block += 1;
    }
    energy
}

/// Chemical energy from each rest-voltage point down to its block's last point.
pub const ENERGY_AT_REST_POINT_J: [f64; REST_TOTAL] = energy_at_rest_points();
/// Chemical energy over each block's whole table. Scale of the residual.
pub const BLOCK_ENERGY_J: [f64; BLOCK_COUNT] = block_energies();

/// Why a step did not advance the cell or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// Current, case temperature or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant or table entry is not this cell's.
    InvalidCell,
    /// `dt` is not exactly one second.
    StepNotOneSecond,
    /// The block of the state is not one of the six chamber settings.
    BlockNotRecognized,
    /// Case temperature is below -21.898 degC or above 45.535 degC, beyond the coldest and the warmest block's own tests, where the tables would hold their end values.
    CaseTemperatureOutsideCard,
    /// The current is above 13.5 A charging or discharging, the end of the ten-second table's current axis.
    CurrentOutsideCard,
    /// Charge removed, incoming or landing, is outside the tables of the block by more than the rounding allowance.
    ChargeOutsideTables,
    /// Landing terminal voltage is below 2.8 V.
    LowerVoltageLimit,
    /// Landing terminal voltage is above 4.242 V.
    UpperVoltageLimit,
    /// The trial step created energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed energy.
    FirstLawBreach,
}

/// Held over the step. Positive current charges the cell, negative discharges it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellLoad {
    pub current_a: f64,
    pub case_temperature_c: f64,
}

/// The locked cell. Every table and limit is a recorded copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell2170M80 {
    pub block_chamber_c: [f64; BLOCK_COUNT],
    pub capacity_ah: [f64; BLOCK_COUNT],
    pub charge_top_ah: [f64; BLOCK_COUNT],
    pub charge_deep_ah: [f64; BLOCK_COUNT],
    pub rest_first: [usize; BLOCK_COUNT],
    pub rest_points: [usize; BLOCK_COUNT],
    pub rest_charge_ah: [f64; REST_TOTAL],
    pub rest_v: [f64; REST_TOTAL],
    pub level_charge_level: [[f64; LEVEL_POINTS]; BLOCK_COUNT],
    pub first_second_ohm: [[f64; LEVEL_POINTS]; BLOCK_COUNT],
    pub pulse_case_c: [[f64; LEVEL_POINTS]; BLOCK_COUNT],
    pub current_size_a: [f64; CURRENT_SIZES],
    pub ten_second_ohm: [[[f64; CURRENT_SIZES]; LEVEL_POINTS]; BLOCK_COUNT],
    pub held_charge_level: [[f64; HELD_POINTS]; BLOCK_COUNT],
    pub held_ohm: [[f64; HELD_POINTS]; BLOCK_COUNT],
    pub held_case_c: [[f64; HELD_POINTS]; BLOCK_COUNT],
    pub time_constant_s: [f64; BLOCK_COUNT],
    pub time_constant_case_c: [f64; BLOCK_COUNT],
    pub lower_voltage_limit_v: f64,
    pub upper_voltage_limit_v: f64,
    pub case_temperature_min_c: f64,
    pub case_temperature_max_c: f64,
    pub block_case_min_c: [f64; BLOCK_COUNT],
    pub block_case_max_c: [f64; BLOCK_COUNT],
    pub block_met_no_mark: [bool; BLOCK_COUNT],
    pub current_discharge_max_a: f64,
    pub current_charge_max_a: f64,
    pub block_current_discharge_max_a: [f64; BLOCK_COUNT],
    pub block_current_charge_max_a: [f64; BLOCK_COUNT],
    pub charge_end_allowance_per_step_ah: f64,
    pub charge_end_allowance_extra_steps: f64,
    pub hold_current_a: f64,
    pub pulse_current_a: f64,
    pub at_once_floor_share: f64,
    pub step_s: f64,
    pub residual_conservation_limit: f64,
}

impl Cell2170M80 {
    /// The fitted cell m80. The constructor takes no arguments.
    #[must_use]
    pub fn cell() -> Self {
        let cell = Self {
            block_chamber_c: BLOCK_CHAMBER_C,
            capacity_ah: CAPACITY_AH,
            charge_top_ah: CHARGE_TOP_AH,
            charge_deep_ah: CHARGE_DEEP_AH,
            rest_first: REST_FIRST,
            rest_points: REST_POINTS,
            rest_charge_ah: REST_CHARGE_AH,
            rest_v: REST_V,
            level_charge_level: LEVEL_CHARGE_LEVEL,
            first_second_ohm: FIRST_SECOND_OHM,
            pulse_case_c: PULSE_CASE_C,
            current_size_a: CURRENT_SIZE_A,
            ten_second_ohm: TEN_SECOND_OHM,
            held_charge_level: HELD_CHARGE_LEVEL,
            held_ohm: HELD_OHM,
            held_case_c: HELD_CASE_C,
            time_constant_s: TIME_CONSTANT_S,
            time_constant_case_c: TIME_CONSTANT_CASE_C,
            lower_voltage_limit_v: LOWER_VOLTAGE_LIMIT_V,
            upper_voltage_limit_v: UPPER_VOLTAGE_LIMIT_V,
            case_temperature_min_c: CASE_TEMPERATURE_MIN_C,
            case_temperature_max_c: CASE_TEMPERATURE_MAX_C,
            block_case_min_c: BLOCK_CASE_MIN_C,
            block_case_max_c: BLOCK_CASE_MAX_C,
            block_met_no_mark: BLOCK_MET_NO_MARK,
            current_discharge_max_a: CURRENT_DISCHARGE_MAX_A,
            current_charge_max_a: CURRENT_CHARGE_MAX_A,
            block_current_discharge_max_a: BLOCK_CURRENT_DISCHARGE_MAX_A,
            block_current_charge_max_a: BLOCK_CURRENT_CHARGE_MAX_A,
            charge_end_allowance_per_step_ah: CHARGE_END_ALLOWANCE_PER_STEP_AH,
            charge_end_allowance_extra_steps: CHARGE_END_ALLOWANCE_EXTRA_STEPS,
            hold_current_a: HOLD_CURRENT_A,
            pulse_current_a: PULSE_CURRENT_A,
            at_once_floor_share: AT_ONCE_FLOOR_SHARE,
            step_s: STEP_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if cell.is_valid() {
            cell
        } else {
            ::core::panic!("2170 cell m80 failed is_valid");
        }
    }

    /// Every recorded copy equals its constant, bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if !same_bits(&self.block_chamber_c, &BLOCK_CHAMBER_C)
            || !same_bits(&self.capacity_ah, &CAPACITY_AH)
            || !same_bits(&self.charge_top_ah, &CHARGE_TOP_AH)
            || !same_bits(&self.charge_deep_ah, &CHARGE_DEEP_AH)
            || !same_bits(&self.rest_charge_ah, &REST_CHARGE_AH)
            || !same_bits(&self.rest_v, &REST_V)
            || !same_bits(&self.current_size_a, &CURRENT_SIZE_A)
            || !same_bits(&self.time_constant_s, &TIME_CONSTANT_S)
            || !same_bits(&self.time_constant_case_c, &TIME_CONSTANT_CASE_C)
            || !same_bits(&self.block_case_min_c, &BLOCK_CASE_MIN_C)
            || !same_bits(&self.block_case_max_c, &BLOCK_CASE_MAX_C)
            || !same_bits(&self.block_current_discharge_max_a, &BLOCK_CURRENT_DISCHARGE_MAX_A)
            || !same_bits(&self.block_current_charge_max_a, &BLOCK_CURRENT_CHARGE_MAX_A)
            || self.block_met_no_mark != BLOCK_MET_NO_MARK
        {
            return false;
        }
        let mut block = 0;
        while block < BLOCK_COUNT {
            if self.rest_first[block] != REST_FIRST[block]
                || self.rest_points[block] != REST_POINTS[block]
                || !same_bits(&self.level_charge_level[block], &LEVEL_CHARGE_LEVEL[block])
                || !same_bits(&self.first_second_ohm[block], &FIRST_SECOND_OHM[block])
                || !same_bits(&self.pulse_case_c[block], &PULSE_CASE_C[block])
                || !same_bits(&self.held_charge_level[block], &HELD_CHARGE_LEVEL[block])
                || !same_bits(&self.held_ohm[block], &HELD_OHM[block])
                || !same_bits(&self.held_case_c[block], &HELD_CASE_C[block])
            {
                return false;
            }
            let mut level = 0;
            while level < LEVEL_POINTS {
                if !same_bits(
                    &self.ten_second_ohm[block][level],
                    &TEN_SECOND_OHM[block][level],
                ) {
                    return false;
                }
                level += 1;
            }
            if !(BLOCK_ENERGY_J[block].is_finite() && BLOCK_ENERGY_J[block] > 0.0) {
                return false;
            }
            block += 1;
        }
        self.lower_voltage_limit_v.to_bits() == LOWER_VOLTAGE_LIMIT_V.to_bits()
            && self.upper_voltage_limit_v.to_bits() == UPPER_VOLTAGE_LIMIT_V.to_bits()
            && self.case_temperature_min_c.to_bits() == CASE_TEMPERATURE_MIN_C.to_bits()
            && self.case_temperature_max_c.to_bits() == CASE_TEMPERATURE_MAX_C.to_bits()
            && self.current_discharge_max_a.to_bits() == CURRENT_DISCHARGE_MAX_A.to_bits()
            && self.current_charge_max_a.to_bits() == CURRENT_CHARGE_MAX_A.to_bits()
            && self.charge_end_allowance_per_step_ah.to_bits()
                == CHARGE_END_ALLOWANCE_PER_STEP_AH.to_bits()
            && self.charge_end_allowance_extra_steps.to_bits()
                == CHARGE_END_ALLOWANCE_EXTRA_STEPS.to_bits()
            && self.hold_current_a.to_bits() == HOLD_CURRENT_A.to_bits()
            && self.pulse_current_a.to_bits() == PULSE_CURRENT_A.to_bits()
            && self.at_once_floor_share.to_bits() == AT_ONCE_FLOOR_SHARE.to_bits()
            && self.step_s.to_bits() == STEP_S.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
    }

    /// The largest step the card stands behind: one second, the sampling of
    /// the measurements its first-second table was read from. [`step`]
    /// refuses any other step as [`StepReject::StepNotOneSecond`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.is_valid() { Some(STEP_S) } else { None }
    }

    /// The smallest step the card stands behind: the same one second.
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        self.gate_dt_s()
    }

    /// Which block a chamber setting names, or `None` when it names none.
    #[must_use]
    pub fn block_of(&self, block_chamber_c: f64) -> Option<usize> {
        let mut block = 0;
        while block < BLOCK_COUNT {
            if block_chamber_c == self.block_chamber_c[block] {
                return Some(block);
            }
            block += 1;
        }
        None
    }

    /// Which block a chamber setting names and the card serves: every one
    /// of the six. `None` when it names none.
    #[must_use]
    pub fn served_block_of(&self, block_chamber_c: f64) -> Option<usize> {
        self.block_of(block_chamber_c)
    }

    /// The flags of a step in `block` under `load`: whether the block met no
    /// mark, and whether the load lies outside the block's compared tests.
    #[must_use]
    pub fn compared_flags(&self, block: usize, load: CellLoad) -> u64 {
        let mut flags = 0;
        if self.block_met_no_mark[block] {
            flags |= FLAG_BLOCK_MET_NO_MARK;
        }
        if load.case_temperature_c < self.block_case_min_c[block]
            || load.case_temperature_c > self.block_case_max_c[block]
            || load.current_a > self.block_current_charge_max_a[block]
            || load.current_a < -self.block_current_discharge_max_a[block]
        {
            flags |= FLAG_OUTSIDE_COMPARED_TESTS;
        }
        flags
    }

    /// How far past an end of the block's tables the running sum of the
    /// charge removed may read after `steps` landed steps, Ah:
    /// [`CHARGE_END_ALLOWANCE_PER_STEP_AH`] for each step and
    /// [`CHARGE_END_ALLOWANCE_EXTRA_STEPS`] more. Zero for a count that is
    /// not finite or is below zero.
    #[must_use]
    pub fn charge_end_allowance_ah(&self, steps: f64) -> f64 {
        if !(steps.is_finite() && steps >= 0.0) {
            return 0.0;
        }
        (steps + self.charge_end_allowance_extra_steps) * self.charge_end_allowance_per_step_ah
    }

    /// The allowance for the landing of a step of `current_a` from a state
    /// that has landed `steps_landed` steps: the allowance after one more
    /// step, never more than half the charge of the step.
    fn landing_allowance_ah(&self, steps_landed: f64, current_a: f64, dt_s: f64) -> f64 {
        let allowance_ah = self.charge_end_allowance_ah(steps_landed + 1.0);
        let half_step_ah = 0.5 * abs_f64(current_a) * dt_s / SECONDS_PER_HOUR;
        if allowance_ah < half_step_ah { allowance_ah } else { half_step_ah }
    }

    /// Whether a landing at `charge_removed_ah` lies on the block's tables:
    /// within the landing allowance of either end, or past an end by no more
    /// than the incoming state, which [`validate`] has held to the allowance.
    /// A rest, or a step back toward the tables, from a state a few spacings
    /// past an end lands; a step further past it is held to the allowance.
    fn lands_on_tables(
        &self,
        block: usize,
        state: &CellState,
        charge_removed_ah: f64,
        current_a: f64,
        dt_s: f64,
    ) -> bool {
        let allowance_ah = self.landing_allowance_ah(state.t_s, current_a, dt_s);
        let past_top_ok = charge_removed_ah >= self.charge_top_ah[block] - allowance_ah
            || charge_removed_ah >= state.charge_removed_ah;
        let past_deep_ok = charge_removed_ah <= self.charge_deep_ah[block] + allowance_ah
            || charge_removed_ah <= state.charge_removed_ah;
        past_top_ok && past_deep_ok
    }

    /// The charge at which the tables are read: the charge itself inside the
    /// block's tables, the end point within the allowance past an end.
    fn table_charge_ah(&self, block: usize, charge_removed_ah: f64) -> f64 {
        if charge_removed_ah < self.charge_top_ah[block] {
            self.charge_top_ah[block]
        } else if charge_removed_ah > self.charge_deep_ah[block] {
            self.charge_deep_ah[block]
        } else {
            charge_removed_ah
        }
    }
}

/// Integrated cell. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellState {
    pub t_s: f64,
    /// Chamber setting of the block the run belongs to, in degC.
    pub block_chamber_c: f64,
    /// Ampere-hours removed since the recharge.
    pub charge_removed_ah: f64,
    /// `1 - charge_removed_ah / capacity of the block`.
    pub charge_level: f64,
    /// Current held over the last step. Zero before the first step.
    pub current_a: f64,
    /// Case temperature held over the last step. Zero before the first step.
    pub case_temperature_c: f64,
    pub terminal_v: f64,
    pub open_circuit_v: f64,
    /// Voltage of the relaxing branch. Negative while discharging.
    pub branch_v: f64,
    /// The part of the resistance that acts at once, as the last step read it. Zero before the first step.
    pub resistance_at_once_ohm: f64,
    /// The part of the resistance that builds, as the last step read it. Zero before the first step.
    pub resistance_branch_ohm: f64,
    /// Time constant of the relaxing branch, as the last step read it. Zero before the first step.
    pub time_constant_s: f64,
    pub chemical_energy_j: f64,
    pub energy_anchor_j: f64,
    /// Work done on the cell at its terminals. Negative while discharging.
    pub electrical_work_j: f64,
    /// Heat in the part of the resistance that acts at once.
    pub heat_at_once_j: f64,
    /// Energy the relaxing branch took: what it holds and what it lost as heat.
    pub branch_energy_j: f64,
    pub energy_defect_j: f64,
    pub residual_conservation_error: f64,
    pub lower_limit_margin_v: f64,
    pub upper_limit_margin_v: f64,
}

impl CellState {
    /// Anchor the ledger at rest: this block, this charge removed, no
    /// current, and nothing held by the relaxing branch.
    #[must_use]
    pub fn try_start(
        cell: &Cell2170M80,
        block_chamber_c: f64,
        charge_removed_ah: f64,
    ) -> Option<Self> {
        if !cell.is_valid() || !block_chamber_c.is_finite() || !charge_removed_ah.is_finite() {
            return None;
        }
        let block = cell.served_block_of(block_chamber_c)?;
        // A start has landed no step: the allowance at the ends is that of
        // the decimal start and the end of the table alone.
        let allowance_ah = cell.charge_end_allowance_ah(0.0);
        if !(charge_removed_ah >= cell.charge_top_ah[block] - allowance_ah
            && charge_removed_ah <= cell.charge_deep_ah[block] + allowance_ah)
        {
            return None;
        }
        let (open_circuit_v, chemical_energy_j) =
            rest_at(cell, block, cell.table_charge_ah(block, charge_removed_ah))?;
        let state = Self {
            t_s: 0.0,
            block_chamber_c: cell.block_chamber_c[block],
            charge_removed_ah,
            charge_level: 1.0 - charge_removed_ah / cell.capacity_ah[block],
            current_a: 0.0,
            case_temperature_c: 0.0,
            terminal_v: open_circuit_v,
            open_circuit_v,
            branch_v: 0.0,
            resistance_at_once_ohm: 0.0,
            resistance_branch_ohm: 0.0,
            time_constant_s: 0.0,
            chemical_energy_j,
            energy_anchor_j: chemical_energy_j,
            electrical_work_j: 0.0,
            heat_at_once_j: 0.0,
            branch_energy_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            lower_limit_margin_v: open_circuit_v - cell.lower_voltage_limit_v,
            upper_limit_margin_v: cell.upper_voltage_limit_v - open_circuit_v,
        };
        if !state_is_finite(&state)
            || open_circuit_v < cell.lower_voltage_limit_v
            || open_circuit_v > cell.upper_voltage_limit_v
        {
            return None;
        }
        Some(state)
    }
}

/// Advance one second and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    cell: &Cell2170M80,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(cell, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    cell: &Cell2170M80,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(cell, state, load, dt_s)?;
    let trial = match integrate(cell, state, load, dt_s) {
        Ok(trial) => trial,
        Err(reject) => return Err(reject),
    };
    commit(cell, state, frame, load, dt_s, trial)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut CellState) {
    state.t_s = normal_or_zero(state.t_s);
    state.block_chamber_c = normal_or_zero(state.block_chamber_c);
    state.charge_removed_ah = normal_or_zero(state.charge_removed_ah);
    state.charge_level = normal_or_zero(state.charge_level);
    state.current_a = normal_or_zero(state.current_a);
    state.case_temperature_c = normal_or_zero(state.case_temperature_c);
    state.terminal_v = normal_or_zero(state.terminal_v);
    state.open_circuit_v = normal_or_zero(state.open_circuit_v);
    state.branch_v = normal_or_zero(state.branch_v);
    state.resistance_at_once_ohm = normal_or_zero(state.resistance_at_once_ohm);
    state.resistance_branch_ohm = normal_or_zero(state.resistance_branch_ohm);
    state.time_constant_s = normal_or_zero(state.time_constant_s);
    state.chemical_energy_j = normal_or_zero(state.chemical_energy_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.electrical_work_j = normal_or_zero(state.electrical_work_j);
    state.heat_at_once_j = normal_or_zero(state.heat_at_once_j);
    state.branch_energy_j = normal_or_zero(state.branch_energy_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.lower_limit_margin_v = normal_or_zero(state.lower_limit_margin_v);
    state.upper_limit_margin_v = normal_or_zero(state.upper_limit_margin_v);
}

/// Every input and the incoming state, before any table is read.
fn validate(
    cell: &Cell2170M80,
    state: &CellState,
    load: CellLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.current_a.is_finite() || !load.case_temperature_c.is_finite() {
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
    if dt_s != cell.step_s {
        return Err(StepReject::StepNotOneSecond);
    }
    let block = match cell.block_of(state.block_chamber_c) {
        Some(block) => block,
        None => return Err(StepReject::BlockNotRecognized),
    };
    if load.case_temperature_c < cell.case_temperature_min_c
        || load.case_temperature_c > cell.case_temperature_max_c
    {
        return Err(StepReject::CaseTemperatureOutsideCard);
    }
    if load.current_a > cell.current_charge_max_a || load.current_a < -cell.current_discharge_max_a
    {
        return Err(StepReject::CurrentOutsideCard);
    }
    let allowance_ah = cell.charge_end_allowance_ah(state.t_s);
    if !(state.charge_removed_ah >= cell.charge_top_ah[block] - allowance_ah
        && state.charge_removed_ah <= cell.charge_deep_ah[block] + allowance_ah)
    {
        return Err(StepReject::ChargeOutsideTables);
    }
    Ok(())
}

/// What a trial step hands to [`commit`]: where the charge and the branch
/// land, and the work done on the cell along the way.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Trial {
    charge_removed_ah: f64,
    branch_v: f64,
    electrical_work_j: f64,
}

fn integrate(
    cell: &Cell2170M80,
    state: &CellState,
    load: CellLoad,
    dt_s: f64,
) -> Result<Trial, StepReject> {
    let block = match cell.block_of(state.block_chamber_c) {
        Some(block) => block,
        None => return Err(StepReject::BlockNotRecognized),
    };
    let current_a = load.current_a;
    let charge_removed_ah = state.charge_removed_ah - current_a * dt_s / SECONDS_PER_HOUR;
    if !charge_removed_ah.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if !cell.lands_on_tables(block, state, charge_removed_ah, current_a, dt_s) {
        return Err(StepReject::ChargeOutsideTables);
    }
    let from_ah = cell.table_charge_ah(block, state.charge_removed_ah);
    let to_ah = cell.table_charge_ah(block, charge_removed_ah);
    let rest_work_j = match rest_work_j(cell, block, from_ah, to_ah) {
        Some(work) => work,
        None => return Err(StepReject::ChargeOutsideTables),
    };
    let readings = match read_tables(
        cell,
        block,
        to_ah,
        load.case_temperature_c,
        current_a,
    ) {
        Some(readings) => readings,
        None => return Err(StepReject::NonFiniteState),
    };
    let exponent = -dt_s / readings.time_constant_s;
    let kept = exp(exponent);
    let gained = -exp_m1(exponent);
    let settled_v = readings.resistance_branch_ohm * current_a;
    let branch_v = kept * state.branch_v + gained * settled_v;
    let branch_work_j = current_a
        * (settled_v * dt_s + (state.branch_v - settled_v) * readings.time_constant_s * gained);
    let at_once_work_j = current_a * current_a * readings.resistance_at_once_ohm * dt_s;
    let electrical_work_j = state.electrical_work_j + (rest_work_j + at_once_work_j + branch_work_j);
    if !(branch_v.is_finite() && electrical_work_j.is_finite()) {
        return Err(StepReject::NonFiniteState);
    }
    Ok(Trial {
        charge_removed_ah,
        branch_v,
        electrical_work_j,
    })
}

fn commit(
    cell: &Cell2170M80,
    state: &mut CellState,
    frame: &mut LastStateFrame64,
    load: CellLoad,
    dt_s: f64,
    trial: Trial,
) -> Result<(), StepReject> {
    let block = match cell.block_of(state.block_chamber_c) {
        Some(block) => block,
        None => return Err(StepReject::BlockNotRecognized),
    };
    if !(trial.charge_removed_ah.is_finite()
        && trial.branch_v.is_finite()
        && trial.electrical_work_j.is_finite())
    {
        return Err(StepReject::NonFiniteState);
    }
    let charge_removed_ah = trial.charge_removed_ah;
    let current_a = load.current_a;
    if !cell.lands_on_tables(block, state, charge_removed_ah, current_a, dt_s) {
        return Err(StepReject::ChargeOutsideTables);
    }
    let table_ah = cell.table_charge_ah(block, charge_removed_ah);
    let readings = match read_tables(
        cell,
        block,
        table_ah,
        load.case_temperature_c,
        current_a,
    ) {
        Some(readings) => readings,
        None => return Err(StepReject::NonFiniteState),
    };
    let (open_circuit_v, chemical_energy_j) = match rest_at(cell, block, table_ah) {
        Some(value) => value,
        None => return Err(StepReject::ChargeOutsideTables),
    };
    let terminal_v = open_circuit_v + current_a * readings.resistance_at_once_ohm + trial.branch_v;
    if !terminal_v.is_finite() || !chemical_energy_j.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if terminal_v < cell.lower_voltage_limit_v {
        return Err(StepReject::LowerVoltageLimit);
    }
    if terminal_v > cell.upper_voltage_limit_v {
        return Err(StepReject::UpperVoltageLimit);
    }
    let heat_at_once_j =
        state.heat_at_once_j + current_a * current_a * readings.resistance_at_once_ohm * dt_s;
    let branch_energy_j = state.branch_energy_j
        + (current_a * current_a * readings.resistance_branch_ohm * dt_s
            - current_a * readings.time_constant_s * (trial.branch_v - state.branch_v));
    let scale = BLOCK_ENERGY_J[block];
    if !(scale > 0.0 && scale.is_finite()) {
        return Err(StepReject::InvalidCell);
    }
    let defect = (chemical_energy_j - state.energy_anchor_j) + heat_at_once_j + branch_energy_j
        - trial.electrical_work_j;
    if !defect.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let gate = cell.residual_conservation_limit * scale;
    if defect > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if defect < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    let landed = CellState {
        t_s: state.t_s + dt_s,
        block_chamber_c: state.block_chamber_c,
        charge_removed_ah,
        charge_level: 1.0 - charge_removed_ah / cell.capacity_ah[block],
        current_a,
        case_temperature_c: load.case_temperature_c,
        terminal_v,
        open_circuit_v,
        branch_v: trial.branch_v,
        resistance_at_once_ohm: readings.resistance_at_once_ohm,
        resistance_branch_ohm: readings.resistance_branch_ohm,
        time_constant_s: readings.time_constant_s,
        chemical_energy_j,
        energy_anchor_j: state.energy_anchor_j,
        electrical_work_j: trial.electrical_work_j,
        heat_at_once_j,
        branch_energy_j,
        energy_defect_j: defect,
        residual_conservation_error: abs_f64(defect) / scale,
        lower_limit_margin_v: terminal_v - cell.lower_voltage_limit_v,
        upper_limit_margin_v: cell.upper_voltage_limit_v - terminal_v,
    };
    if !state_is_finite(&landed) {
        return Err(StepReject::NonFiniteState);
    }
    *state = landed;
    publish(state, frame, cell.compared_flags(block, load));
    Ok(())
}

fn publish(state: &CellState, frame: &mut LastStateFrame64, flags: u64) {
    frame.t = state.t_s;
    frame.pos[0] = state.terminal_v as f32;
    frame.pos[1] = state.charge_removed_ah as f32;
    frame.pos[2] = state.current_a as f32;
    frame.vel[0] = state.open_circuit_v as f32;
    frame.vel[1] = state.branch_v as f32;
    frame.vel[2] = state.case_temperature_c as f32;
    frame.force_torque = state.lower_limit_margin_v as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = flags;
}

/// A place on a table: the point at or before it, and the share of the way
/// to the next point. A share of zero is the point itself.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Place {
    index: usize,
    share: f64,
}

/// A place on a table whose points fall as the index rises. Beyond either
/// end the place is the end point.
fn place_falling<const N: usize>(points: &[f64; N], at: f64) -> Place {
    if at >= points[0] {
        return Place {
            index: 0,
            share: 0.0,
        };
    }
    let mut index = 0;
    while index < N - 1 {
        if at > points[index + 1] {
            return Place {
                index,
                share: (points[index] - at) / (points[index] - points[index + 1]),
            };
        }
        index += 1;
    }
    Place {
        index: N - 1,
        share: 0.0,
    }
}

/// A place on a table whose points rise with the index. Beyond either end
/// the place is the end point.
fn place_rising<const N: usize>(points: &[f64; N], at: f64) -> Place {
    if at <= points[0] {
        return Place {
            index: 0,
            share: 0.0,
        };
    }
    let mut index = 0;
    while index < N - 1 {
        if at < points[index + 1] {
            return Place {
                index,
                share: (at - points[index]) / (points[index + 1] - points[index]),
            };
        }
        index += 1;
    }
    Place {
        index: N - 1,
        share: 0.0,
    }
}

/// The straight line between two points of a table, at a place.
fn read_at<const N: usize>(values: &[f64; N], place: Place) -> f64 {
    if place.share == 0.0 {
        values[place.index]
    } else {
        values[place.index] + (values[place.index + 1] - values[place.index]) * place.share
    }
}

/// Ten-second resistance of one block: straight lines along the size of the
/// current, then along the charge level.
fn ten_second_at(cell: &Cell2170M80, block: usize, level: Place, size: Place) -> f64 {
    let low = read_at(&cell.ten_second_ohm[block][level.index], size);
    if level.share == 0.0 {
        low
    } else {
        let high = read_at(&cell.ten_second_ohm[block][level.index + 1], size);
        low + (high - low) * level.share
    }
}

/// Where a case temperature falls among the six blocks' own temperatures,
/// which fall from the first block to the last.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Between {
    /// At or above the warmest block's temperature.
    Warmest,
    /// Below the coldest block's temperature.
    Coldest,
    /// Between block `warm` and the next block, `share` of the way to the next.
    Pair { warm: usize, share: f64 },
}

fn between(block_case_c: &[f64; BLOCK_COUNT], case_temperature_c: f64) -> Between {
    if case_temperature_c >= block_case_c[0] {
        return Between::Warmest;
    }
    let mut warm = 0;
    while warm < BLOCK_COUNT - 1 {
        if case_temperature_c >= block_case_c[warm + 1] {
            return Between::Pair {
                warm,
                share: (block_case_c[warm] - case_temperature_c)
                    / (block_case_c[warm] - block_case_c[warm + 1]),
            };
        }
        warm += 1;
    }
    Between::Coldest
}

/// One reading from the six blocks at the measured case temperature: the
/// logarithm in a straight line between the two blocks either side.
fn bring_together(values: &[f64; BLOCK_COUNT], place: Between) -> f64 {
    match place {
        Between::Warmest => values[0],
        Between::Coldest => values[BLOCK_COUNT - 1],
        Between::Pair { warm, share } => {
            exp((1.0 - share) * log(values[warm]) + share * log(values[warm + 1]))
        }
    }
}

/// What one step reads from the resistance tables and the time constants.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Readings {
    charge_level: f64,
    resistance_at_once_ohm: f64,
    resistance_branch_ohm: f64,
    time_constant_s: f64,
}

/// The part of the resistance that acts at once.
fn at_once_ohm(
    first_second: f64,
    ten_second_at_current: f64,
    ten_second_at_pulse: f64,
    floor_share: f64,
) -> f64 {
    let at_once = first_second + (ten_second_at_current - ten_second_at_pulse);
    let floor = floor_share * first_second;
    if at_once < floor { floor } else { at_once }
}

/// The part of the resistance that builds up and fades.
fn branch_ohm(
    held: f64,
    first_second: f64,
    ten_second_at_hold: f64,
    ten_second_at_pulse: f64,
) -> f64 {
    let branch = held - first_second - (ten_second_at_hold - ten_second_at_pulse);
    if branch < 0.0 { 0.0 } else { branch }
}

fn read_tables(
    cell: &Cell2170M80,
    block: usize,
    charge_removed_ah: f64,
    case_temperature_c: f64,
    current_a: f64,
) -> Option<Readings> {
    let charge_level = 1.0 - charge_removed_ah / cell.capacity_ah[block];
    if !charge_level.is_finite() {
        return None;
    }
    let size_at_current = place_rising(&cell.current_size_a, abs_f64(current_a));
    let size_at_hold = place_rising(&cell.current_size_a, cell.hold_current_a);
    let size_at_pulse = place_rising(&cell.current_size_a, cell.pulse_current_a);
    let mut held = [0.0; BLOCK_COUNT];
    let mut held_case_c = [0.0; BLOCK_COUNT];
    let mut first_second = [0.0; BLOCK_COUNT];
    let mut pulse_case_c = [0.0; BLOCK_COUNT];
    let mut ten_at_current = [0.0; BLOCK_COUNT];
    let mut ten_at_hold = [0.0; BLOCK_COUNT];
    let mut ten_at_pulse = [0.0; BLOCK_COUNT];
    let mut other = 0;
    while other < BLOCK_COUNT {
        let held_place = place_falling(&cell.held_charge_level[other], charge_level);
        held[other] = read_at(&cell.held_ohm[other], held_place);
        held_case_c[other] = read_at(&cell.held_case_c[other], held_place);
        let level_place = place_falling(&cell.level_charge_level[other], charge_level);
        first_second[other] = read_at(&cell.first_second_ohm[other], level_place);
        pulse_case_c[other] = read_at(&cell.pulse_case_c[other], level_place);
        ten_at_current[other] = ten_second_at(cell, other, level_place, size_at_current);
        ten_at_hold[other] = ten_second_at(cell, other, level_place, size_at_hold);
        ten_at_pulse[other] = ten_second_at(cell, other, level_place, size_at_pulse);
        other += 1;
    }
    let among_held = between(&held_case_c, case_temperature_c);
    let among_pulse = between(&pulse_case_c, case_temperature_c);
    let among_time = between(&cell.time_constant_case_c, case_temperature_c);
    let held_ohm = bring_together(&held, among_held);
    let first_second_ohm = bring_together(&first_second, among_pulse);
    let ten_second_at_current_ohm = bring_together(&ten_at_current, among_pulse);
    let ten_second_at_hold_ohm = bring_together(&ten_at_hold, among_pulse);
    let ten_second_at_pulse_ohm = bring_together(&ten_at_pulse, among_pulse);
    let time_constant_s = bring_together(&cell.time_constant_s, among_time);
    let resistance_at_once_ohm = at_once_ohm(
        first_second_ohm,
        ten_second_at_current_ohm,
        ten_second_at_pulse_ohm,
        cell.at_once_floor_share,
    );
    let resistance_branch_ohm = branch_ohm(
        held_ohm,
        first_second_ohm,
        ten_second_at_hold_ohm,
        ten_second_at_pulse_ohm,
    );
    if !(held_ohm.is_finite()
        && first_second_ohm.is_finite()
        && ten_second_at_current_ohm.is_finite()
        && ten_second_at_hold_ohm.is_finite()
        && ten_second_at_pulse_ohm.is_finite()
        && resistance_at_once_ohm.is_finite()
        && resistance_branch_ohm.is_finite()
        && time_constant_s.is_finite()
        && time_constant_s > 0.0)
    {
        return None;
    }
    Some(Readings {
        charge_level,
        resistance_at_once_ohm,
        resistance_branch_ohm,
        time_constant_s,
    })
}

/// Index, among all rest-voltage points, of the point that begins the
/// interval holding this charge. On the block's last point, the interval
/// before it. `None` outside the block's points.
fn rest_interval(cell: &Cell2170M80, block: usize, charge_removed_ah: f64) -> Option<usize> {
    let first = cell.rest_first[block];
    let last = first + cell.rest_points[block] - 1;
    if !(charge_removed_ah >= cell.rest_charge_ah[first]
        && charge_removed_ah <= cell.rest_charge_ah[last])
    {
        return None;
    }
    let mut low = first;
    let mut high = last;
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if charge_removed_ah >= cell.rest_charge_ah[middle] {
            low = middle;
        } else {
            high = middle;
        }
    }
    Some(low)
}

/// Rest voltage on the interval that begins at point `index`.
fn rest_v_on(cell: &Cell2170M80, index: usize, charge_removed_ah: f64) -> f64 {
    let low_ah = cell.rest_charge_ah[index];
    let high_ah = cell.rest_charge_ah[index + 1];
    if charge_removed_ah == low_ah {
        cell.rest_v[index]
    } else if charge_removed_ah == high_ah {
        cell.rest_v[index + 1]
    } else {
        cell.rest_v[index]
            + (cell.rest_v[index + 1] - cell.rest_v[index])
                * ((charge_removed_ah - low_ah) / (high_ah - low_ah))
    }
}

/// Rest voltage and chemical energy at a charge removed.
fn rest_at(cell: &Cell2170M80, block: usize, charge_removed_ah: f64) -> Option<(f64, f64)> {
    let index = rest_interval(cell, block, charge_removed_ah)?;
    let open_circuit_v = rest_v_on(cell, index, charge_removed_ah);
    let chemical_energy_j = ENERGY_AT_REST_POINT_J[index + 1]
        + SECONDS_PER_HOUR
            * (cell.rest_charge_ah[index + 1] - charge_removed_ah)
            * 0.5
            * (open_circuit_v + cell.rest_v[index + 1]);
    Some((open_circuit_v, chemical_energy_j))
}

/// Work the rest voltage takes as the charge removed goes from `from_ah` to
/// `to_ah`: `3600 *` the integral of the rest voltage from `to_ah` to
/// `from_ah`, piece by piece between the table's points.
fn rest_work_j(cell: &Cell2170M80, block: usize, from_ah: f64, to_ah: f64) -> Option<f64> {
    let first = cell.rest_first[block];
    let last = first + cell.rest_points[block] - 1;
    let mut index = rest_interval(cell, block, from_ah)?;
    if to_ah == from_ah {
        return Some(0.0);
    }
    let mut at_ah = from_ah;
    let mut at_v = rest_v_on(cell, index, at_ah);
    let mut work_j = 0.0;
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > REST_TOTAL {
            return None;
        }
        if to_ah > at_ah {
            let point_ah = cell.rest_charge_ah[index + 1];
            if to_ah > point_ah {
                let point_v = cell.rest_v[index + 1];
                work_j += SECONDS_PER_HOUR * (at_ah - point_ah) * 0.5 * (at_v + point_v);
                at_ah = point_ah;
                at_v = point_v;
                if index + 1 >= last {
                    return None;
                }
                index += 1;
            } else {
                let to_v = rest_v_on(cell, index, to_ah);
                work_j += SECONDS_PER_HOUR * (at_ah - to_ah) * 0.5 * (at_v + to_v);
                break;
            }
        } else {
            let point_ah = cell.rest_charge_ah[index];
            if at_ah == point_ah {
                // Standing on the point that begins this interval: the way
                // down is the interval before it.
                if index == first {
                    return None;
                }
                index -= 1;
            } else if to_ah < point_ah {
                let point_v = cell.rest_v[index];
                work_j += SECONDS_PER_HOUR * (at_ah - point_ah) * 0.5 * (at_v + point_v);
                at_ah = point_ah;
                at_v = point_v;
            } else {
                let to_v = rest_v_on(cell, index, to_ah);
                work_j += SECONDS_PER_HOUR * (at_ah - to_ah) * 0.5 * (at_v + to_v);
                break;
            }
        }
    }
    if work_j.is_finite() { Some(work_j) } else { None }
}

fn same_bits<const N: usize>(recorded: &[f64; N], locked: &[f64; N]) -> bool {
    let mut index = 0;
    while index < N {
        if recorded[index].to_bits() != locked[index].to_bits() {
            return false;
        }
        index += 1;
    }
    true
}

fn abs_f64(value: f64) -> f64 {
    if value < 0.0 { -value } else { value }
}

fn state_is_finite(state: &CellState) -> bool {
    state.t_s.is_finite()
        && state.block_chamber_c.is_finite()
        && state.charge_removed_ah.is_finite()
        && state.charge_level.is_finite()
        && state.current_a.is_finite()
        && state.case_temperature_c.is_finite()
        && state.terminal_v.is_finite()
        && state.open_circuit_v.is_finite()
        && state.branch_v.is_finite()
        && state.resistance_at_once_ohm.is_finite()
        && state.resistance_branch_ohm.is_finite()
        && state.time_constant_s.is_finite()
        && state.chemical_energy_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.electrical_work_j.is_finite()
        && state.heat_at_once_j.is_finite()
        && state.branch_energy_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.lower_limit_margin_v.is_finite()
        && state.upper_limit_margin_v.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::panic;
    use std::vec::Vec;

    use super::{
        AT_ONCE_FLOOR_SHARE, BLOCK_CASE_MAX_C, BLOCK_CASE_MIN_C, BLOCK_CHAMBER_C, BLOCK_COUNT,
        BLOCK_CURRENT_CHARGE_MAX_A, BLOCK_CURRENT_DISCHARGE_MAX_A,
        BLOCK_ENERGY_J, BLOCK_MET_NO_MARK, CAPACITY_AH, CASE_TEMPERATURE_MAX_C,
        FLAG_BLOCK_MET_NO_MARK, FLAG_OUTSIDE_COMPARED_TESTS,
        CASE_TEMPERATURE_MIN_C, CHARGE_DEEP_AH, CHARGE_END_ALLOWANCE_EXTRA_STEPS,
        CHARGE_END_ALLOWANCE_PER_STEP_AH, CHARGE_TOP_AH, CURRENT_CHARGE_MAX_A,
        CURRENT_COMPARED_CHARGE_MAX_A, CURRENT_COMPARED_DISCHARGE_MAX_A,
        CURRENT_DISCHARGE_MAX_A, CURRENT_SIZE_A, CURRENT_SIZES, Cell2170M80, CellLoad, CellState,
        ENERGY_AT_REST_POINT_J, FIRST_SECOND_OHM, FITTED_NUMBERS, HELD_CASE_C, HELD_CHARGE_LEVEL,
        HELD_OHM, HELD_POINTS, HOLD_CURRENT_A, LEVEL_CHARGE_LEVEL, LEVEL_POINTS,
        LOWER_VOLTAGE_LIMIT_V, PULSE_CASE_C, PULSE_CURRENT_A, RESIDUAL_CONSERVATION_LIMIT,
        REST_CHARGE_AH, REST_FIRST, REST_POINTS, REST_TOTAL, REST_V, SECONDS_PER_HOUR, STEP_S,
        StepReject, TEN_SECOND_OHM, TIME_CONSTANT_CASE_C, TIME_CONSTANT_S, Trial,
        UPPER_VOLTAGE_LIMIT_V, abs_f64, at_once_ohm, branch_ohm, commit, integrate, read_tables,
        rest_at, rest_work_j, step, validate,
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

    fn state_bits(state: &CellState) -> [u64; 21] {
        [
            state.t_s.to_bits(),
            state.block_chamber_c.to_bits(),
            state.charge_removed_ah.to_bits(),
            state.charge_level.to_bits(),
            state.current_a.to_bits(),
            state.case_temperature_c.to_bits(),
            state.terminal_v.to_bits(),
            state.open_circuit_v.to_bits(),
            state.branch_v.to_bits(),
            state.resistance_at_once_ohm.to_bits(),
            state.resistance_branch_ohm.to_bits(),
            state.time_constant_s.to_bits(),
            state.chemical_energy_j.to_bits(),
            state.energy_anchor_j.to_bits(),
            state.electrical_work_j.to_bits(),
            state.heat_at_once_j.to_bits(),
            state.branch_energy_j.to_bits(),
            state.energy_defect_j.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.lower_limit_margin_v.to_bits(),
            state.upper_limit_margin_v.to_bits(),
        ]
    }

    fn load(current_a: f64, case_temperature_c: f64) -> CellLoad {
        CellLoad {
            current_a,
            case_temperature_c,
        }
    }

    fn next_up(value: f64) -> f64 {
        if value > 0.0 {
            f64::from_bits(value.to_bits() + 1)
        } else if value < 0.0 {
            f64::from_bits(value.to_bits() - 1)
        } else {
            f64::from_bits(1)
        }
    }

    fn next_down(value: f64) -> f64 {
        -next_up(-value)
    }

    fn close(got: f64, want: f64, relative: f64) -> bool {
        abs_f64(got - want) <= relative * abs_f64(want)
    }

    // ---- a plain reading of the tables, written apart from the card's own
    // ---- lookups, with the platform's logarithm and exponential -----------

    fn plain_line(x0: f64, y0: f64, x1: f64, y1: f64, x: f64) -> f64 {
        y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    }

    /// A table along the charge level, which falls along the table. End values held.
    fn plain_level(points: &[f64], values: &[f64], charge_level: f64) -> f64 {
        let last = points.len() - 1;
        if charge_level >= points[0] {
            return values[0];
        }
        if charge_level <= points[last] {
            return values[last];
        }
        let mut j = 0;
        while j < last {
            if charge_level <= points[j] && charge_level >= points[j + 1] {
                return plain_line(points[j], values[j], points[j + 1], values[j + 1], charge_level);
            }
            j += 1;
        }
        panic!("no interval for charge level {charge_level}");
    }

    fn plain_ten_second(block: usize, charge_level: f64, size_a: f64) -> f64 {
        let mut size = size_a;
        if size < CURRENT_SIZE_A[0] {
            size = CURRENT_SIZE_A[0];
        }
        if size > CURRENT_SIZE_A[CURRENT_SIZES - 1] {
            size = CURRENT_SIZE_A[CURRENT_SIZES - 1];
        }
        let mut columns = [0.0; CURRENT_SIZES];
        let mut k = 0;
        while k < CURRENT_SIZES {
            let mut column = [0.0; LEVEL_POINTS];
            let mut row = 0;
            while row < LEVEL_POINTS {
                column[row] = TEN_SECOND_OHM[block][row][k];
                row += 1;
            }
            columns[k] = plain_level(&LEVEL_CHARGE_LEVEL[block], &column, charge_level);
            k += 1;
        }
        let mut j = 0;
        while j < CURRENT_SIZES - 1 {
            if size >= CURRENT_SIZE_A[j] && size <= CURRENT_SIZE_A[j + 1] {
                return plain_line(
                    CURRENT_SIZE_A[j],
                    columns[j],
                    CURRENT_SIZE_A[j + 1],
                    columns[j + 1],
                    size,
                );
            }
            j += 1;
        }
        panic!("no interval for current size {size}");
    }

    fn plain_together(case_c: f64, block_case_c: &[f64; BLOCK_COUNT], values: &[f64; BLOCK_COUNT]) -> f64 {
        if case_c >= block_case_c[0] {
            return values[0];
        }
        if case_c <= block_case_c[BLOCK_COUNT - 1] {
            return values[BLOCK_COUNT - 1];
        }
        let mut i = 0;
        while i < BLOCK_COUNT - 1 {
            if case_c <= block_case_c[i] && case_c >= block_case_c[i + 1] {
                let w = (block_case_c[i] - case_c) / (block_case_c[i] - block_case_c[i + 1]);
                return ((1.0 - w) * values[i].ln() + w * values[i + 1].ln()).exp();
            }
            i += 1;
        }
        panic!("no pair of blocks for {case_c}");
    }

    fn plain_rest_v(block: usize, charge_removed_ah: f64) -> f64 {
        let first = REST_FIRST[block];
        let mut j = first;
        while j < first + REST_POINTS[block] - 1 {
            if charge_removed_ah >= REST_CHARGE_AH[j] && charge_removed_ah <= REST_CHARGE_AH[j + 1] {
                return plain_line(
                    REST_CHARGE_AH[j],
                    REST_V[j],
                    REST_CHARGE_AH[j + 1],
                    REST_V[j + 1],
                    charge_removed_ah,
                );
            }
            j += 1;
        }
        panic!("no interval for charge {charge_removed_ah}");
    }

    struct Plain {
        open_circuit_v: f64,
        held_ohm: f64,
        at_once_ohm: f64,
        branch_ohm: f64,
        time_constant_s: f64,
        at_once_before_floor_ohm: f64,
        branch_before_floor_ohm: f64,
    }

    fn plain_readings(block: usize, charge_removed_ah: f64, case_c: f64, current_a: f64) -> Plain {
        let charge_level = 1.0 - charge_removed_ah / CAPACITY_AH[block];
        let size = abs_f64(current_a);
        let mut held = [0.0; BLOCK_COUNT];
        let mut held_c = [0.0; BLOCK_COUNT];
        let mut first = [0.0; BLOCK_COUNT];
        let mut pulse_c = [0.0; BLOCK_COUNT];
        let mut ten_i = [0.0; BLOCK_COUNT];
        let mut ten_hold = [0.0; BLOCK_COUNT];
        let mut ten_pulse = [0.0; BLOCK_COUNT];
        let mut b = 0;
        while b < BLOCK_COUNT {
            held[b] = plain_level(&HELD_CHARGE_LEVEL[b], &HELD_OHM[b], charge_level);
            held_c[b] = plain_level(&HELD_CHARGE_LEVEL[b], &HELD_CASE_C[b], charge_level);
            first[b] = plain_level(&LEVEL_CHARGE_LEVEL[b], &FIRST_SECOND_OHM[b], charge_level);
            pulse_c[b] = plain_level(&LEVEL_CHARGE_LEVEL[b], &PULSE_CASE_C[b], charge_level);
            ten_i[b] = plain_ten_second(b, charge_level, size);
            ten_hold[b] = plain_ten_second(b, charge_level, HOLD_CURRENT_A);
            ten_pulse[b] = plain_ten_second(b, charge_level, PULSE_CURRENT_A);
            b += 1;
        }
        let held_ohm = plain_together(case_c, &held_c, &held);
        let first_ohm = plain_together(case_c, &pulse_c, &first);
        let ten_i_ohm = plain_together(case_c, &pulse_c, &ten_i);
        let ten_hold_ohm = plain_together(case_c, &pulse_c, &ten_hold);
        let ten_pulse_ohm = plain_together(case_c, &pulse_c, &ten_pulse);
        let at_once_before = first_ohm + ten_i_ohm - ten_pulse_ohm;
        let branch_before = held_ohm - first_ohm - (ten_hold_ohm - ten_pulse_ohm);
        Plain {
            open_circuit_v: plain_rest_v(block, charge_removed_ah),
            held_ohm,
            at_once_ohm: if at_once_before < first_ohm / 5.0 {
                first_ohm / 5.0
            } else {
                at_once_before
            },
            branch_ohm: if branch_before < 0.0 { 0.0 } else { branch_before },
            time_constant_s: plain_together(case_c, &TIME_CONSTANT_CASE_C, &TIME_CONSTANT_S),
            at_once_before_floor_ohm: at_once_before,
            branch_before_floor_ohm: branch_before,
        }
    }

    // ---- deliberately wrong steppers ---------------------------------------

    /// The branch advanced by one explicit Euler step; everything else the card's.
    fn step_euler_branch(
        cell: &Cell2170M80,
        state: &mut CellState,
        frame: &mut LastStateFrame64,
        load: CellLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(cell, state, load, dt_s)?;
        let mut trial = integrate(cell, state, load, dt_s)?;
        let block = cell.block_of(state.block_chamber_c).unwrap();
        let readings = read_tables(
            cell,
            block,
            trial.charge_removed_ah,
            load.case_temperature_c,
            load.current_a,
        )
        .unwrap();
        trial.branch_v = state.branch_v
            + dt_s / readings.time_constant_s
                * (readings.resistance_branch_ohm * load.current_a - state.branch_v);
        commit(cell, state, frame, load, dt_s, trial)
    }

    /// The work taken as landing voltage times current times the step.
    fn step_rectangle_work(
        cell: &Cell2170M80,
        state: &mut CellState,
        frame: &mut LastStateFrame64,
        load: CellLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(cell, state, load, dt_s)?;
        let mut trial = integrate(cell, state, load, dt_s)?;
        let block = cell.block_of(state.block_chamber_c).unwrap();
        let readings = read_tables(
            cell,
            block,
            trial.charge_removed_ah,
            load.case_temperature_c,
            load.current_a,
        )
        .unwrap();
        let (open_circuit_v, _) = rest_at(cell, block, trial.charge_removed_ah).unwrap();
        let landing_v =
            open_circuit_v + load.current_a * readings.resistance_at_once_ohm + trial.branch_v;
        trial.electrical_work_j = state.electrical_work_j + landing_v * load.current_a * dt_s;
        commit(cell, state, frame, load, dt_s, trial)
    }

    fn step_bias_work(
        cell: &Cell2170M80,
        state: &mut CellState,
        frame: &mut LastStateFrame64,
        load: CellLoad,
        dt_s: f64,
        extra_j: f64,
    ) -> Result<(), StepReject> {
        validate(cell, state, load, dt_s)?;
        let mut trial = integrate(cell, state, load, dt_s)?;
        trial.electrical_work_j += extra_j;
        commit(cell, state, frame, load, dt_s, trial)
    }

    fn mix(h: u64, w: u64) -> u64 {
        let mut x = (h ^ w).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        x ^= x >> 29;
        x
    }

    // ---- the tests -----------------------------------------------------------

    #[test]
    fn constants_are_locked_and_a_caller_cannot_set_one() {
        assert_copy::<Cell2170M80>();
        assert_copy::<CellState>();
        assert_copy::<CellLoad>();
        assert_copy::<StepReject>();
        assert_copy::<Trial>();
        let cell = Cell2170M80::cell();
        assert!(cell.is_valid());
        let mut state = CellState::try_start(&cell, 25.0, 1.0).unwrap();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();

        let tamper: [fn(&mut Cell2170M80); 20] = [
            |c| c.rest_v[700] += 1.0e-9,
            |c| c.rest_charge_ah[3] = next_up(c.rest_charge_ah[3]),
            |c| c.capacity_ah[1] = 4.5,
            |c| c.charge_deep_ah[1] = 5.0,
            |c| c.ten_second_ohm[5][13][6] *= 0.5,
            |c| c.first_second_ohm[0][0] = next_down(c.first_second_ohm[0][0]),
            |c| c.held_ohm[3][7] = 0.0,
            |c| c.pulse_case_c[2][4] += 1.0,
            |c| c.time_constant_s[4] = 60.0,
            |c| c.lower_voltage_limit_v = 2.5,
            |c| c.current_discharge_max_a = 11.375,
            |c| c.current_charge_max_a = 9.737,
            |c| c.block_case_min_c[0] = -25.0,
            |c| c.block_case_max_c[3] = 50.0,
            |c| c.block_met_no_mark[5] = false,
            |c| c.block_current_discharge_max_a[0] = 11.375,
            |c| c.block_current_charge_max_a[3] = 9.737,
            |c| c.charge_end_allowance_per_step_ah *= 2.0,
            |c| c.charge_end_allowance_extra_steps = 1.0e9,
            |c| c.step_s = 0.5,
        ];
        for (index, change) in tamper.iter().enumerate() {
            let mut tampered = cell;
            change(&mut tampered);
            assert!(!tampered.is_valid(), "change {index}");
            assert!(tampered.gate_dt_s().is_none(), "change {index}");
            assert!(tampered.floor_dt_s().is_none(), "change {index}");
            assert!(CellState::try_start(&tampered, 25.0, 1.0).is_none(), "change {index}");
            let got = step(&tampered, &mut state, &mut frame, load(-1.0, 25.0), 1.0);
            assert_eq!(got, Err(StepReject::InvalidCell), "change {index}");
            assert_eq!(state_bits(&state), saved, "change {index}");
            assert_eq!(frame.to_bytes(), held, "change {index}");
            proof_intact(&frame);
        }
        // The same step on the untouched card lands.
        step(&cell, &mut state, &mut frame, load(-1.0, 25.0), 1.0).unwrap();
        proof_intact(&frame);
    }

    #[test]
    fn the_tables_are_the_fitted_numbers_in_their_order() {
        assert_eq!(BLOCK_COUNT, 6);
        assert_eq!(REST_TOTAL, 1353);
        assert_eq!(REST_POINTS, [249, 239, 226, 221, 213, 205]);
        let fitted = BLOCK_COUNT
            + REST_TOTAL
            + BLOCK_COUNT * (2 * HELD_POINTS + 2 * LEVEL_POINTS + LEVEL_POINTS * CURRENT_SIZES)
            + 2 * BLOCK_COUNT;
        assert_eq!(fitted, 2307);
        assert_eq!(FITTED_NUMBERS, 2307);
        assert_eq!(BLOCK_CHAMBER_C, [40.0, 25.0, 10.0, 0.0, -10.0, -20.0]);
        assert_eq!(CURRENT_SIZE_A, [0.5625, 1.125, 1.46, 2.25, 4.5, 9.0, 13.5]);
        assert_eq!(STEP_S.to_bits(), 1.0_f64.to_bits());
        assert_eq!(LOWER_VOLTAGE_LIMIT_V.to_bits(), 2.8_f64.to_bits());
        assert_eq!(UPPER_VOLTAGE_LIMIT_V.to_bits(), 4.242_f64.to_bits());
        assert_eq!(CASE_TEMPERATURE_MIN_C.to_bits(), (-21.898_f64).to_bits());
        assert_eq!(CASE_TEMPERATURE_MIN_C.to_bits(), BLOCK_CASE_MIN_C[5].to_bits());
        assert_eq!(CASE_TEMPERATURE_MAX_C.to_bits(), 45.535_f64.to_bits());
        assert_eq!(CURRENT_DISCHARGE_MAX_A.to_bits(), 13.5_f64.to_bits());
        assert_eq!(CURRENT_CHARGE_MAX_A.to_bits(), 13.5_f64.to_bits());
        assert_eq!(CURRENT_DISCHARGE_MAX_A.to_bits(), CURRENT_SIZE_A[CURRENT_SIZES - 1].to_bits());
        assert_eq!(CHARGE_END_ALLOWANCE_PER_STEP_AH.to_bits(), (1.0_f64 / 1_125_899_906_842_624.0).to_bits());
        assert_eq!(CHARGE_END_ALLOWANCE_EXTRA_STEPS.to_bits(), 2.0_f64.to_bits());
        assert_eq!(BLOCK_MET_NO_MARK, [false, false, false, false, true, true]);
        // The bounds of the card are the widest of all six blocks' bands,
        // and each block's band lies around its own chamber setting.
        let mut coldest = f64::INFINITY;
        let mut warmest = f64::NEG_INFINITY;
        for block in 0..BLOCK_COUNT {
            assert!(BLOCK_CASE_MIN_C[block] < BLOCK_CASE_MAX_C[block]);
            assert!(BLOCK_CASE_MIN_C[block] < BLOCK_CHAMBER_C[block] + 1.0);
            coldest = coldest.min(BLOCK_CASE_MIN_C[block]);
            warmest = warmest.max(BLOCK_CASE_MAX_C[block]);
        }
        assert_eq!(coldest.to_bits(), CASE_TEMPERATURE_MIN_C.to_bits());
        assert_eq!(warmest.to_bits(), CASE_TEMPERATURE_MAX_C.to_bits());
        // Each block's currents are the largest of its own compared tests
        // (envelope.json of the 0.7.0 fix), rounded outward to the
        // milliampere; the largest compared are the largest of the served
        // blocks' through engine 0.9.0, and the card's are the pulse test's top.
        assert_eq!(BLOCK_CURRENT_DISCHARGE_MAX_A, [9.809, 9.23, 10.079, 11.375, 10.467, 10.761]);
        assert_eq!(BLOCK_CURRENT_CHARGE_MAX_A, [9.737, 9.484, 8.776, 8.304, 8.006, 7.638]);
        let compared_discharge = [9.808720862587762, 9.22979039039039, 10.078014999999999, 11.374274, 10.466104000000001, 10.760983999999999];
        let compared_charge = [9.73669008016032, 9.483595800000002, 8.775787500000002, 8.30335144855145, 8.005188577154309, 7.63735672];
        let mut most_discharge = 0.0_f64;
        let mut most_charge = 0.0_f64;
        for block in 0..BLOCK_COUNT {
            assert!(BLOCK_CURRENT_DISCHARGE_MAX_A[block] >= compared_discharge[block]);
            assert!(BLOCK_CURRENT_DISCHARGE_MAX_A[block] - compared_discharge[block] < 0.001);
            assert!(BLOCK_CURRENT_CHARGE_MAX_A[block] >= compared_charge[block]);
            assert!(BLOCK_CURRENT_CHARGE_MAX_A[block] - compared_charge[block] < 0.001);
            if !BLOCK_MET_NO_MARK[block] {
                most_discharge = most_discharge.max(BLOCK_CURRENT_DISCHARGE_MAX_A[block]);
                most_charge = most_charge.max(BLOCK_CURRENT_CHARGE_MAX_A[block]);
            }
        }
        assert_eq!(most_discharge.to_bits(), CURRENT_COMPARED_DISCHARGE_MAX_A.to_bits());
        assert_eq!(most_charge.to_bits(), CURRENT_COMPARED_CHARGE_MAX_A.to_bits());
        assert_eq!(HOLD_CURRENT_A.to_bits(), 1.35_f64.to_bits());
        assert_eq!(PULSE_CURRENT_A.to_bits(), 2.25_f64.to_bits());
        assert_eq!(AT_ONCE_FLOOR_SHARE.to_bits(), 0.2_f64.to_bits());
        assert_eq!(CAPACITY_AH[0].to_bits(), 4.67683635335689_f64.to_bits());
        assert_eq!(CAPACITY_AH[5].to_bits(), 4.211231603553062_f64.to_bits());
        assert_eq!(REST_V[0].to_bits(), 4.17879_f64.to_bits());
        assert_eq!(REST_V[REST_TOTAL - 1].to_bits(), 3.21725_f64.to_bits());
        assert_eq!(TIME_CONSTANT_S[0].to_bits(), 94.35132182501626_f64.to_bits());
        assert_eq!(TIME_CONSTANT_S[5].to_bits(), 76.6044853061611_f64.to_bits());

        let mut offset = 0;
        let mut block = 0;
        while block < BLOCK_COUNT {
            assert_eq!(REST_FIRST[block], offset);
            let first = REST_FIRST[block];
            let last = first + REST_POINTS[block] - 1;
            // Charge removed rises along each rest-voltage table, the table
            // starts at the block's top and reaches its deepest charge.
            let mut j = first;
            while j < last {
                assert!(REST_CHARGE_AH[j + 1] > REST_CHARGE_AH[j], "block {block} point {j}");
                j += 1;
            }
            assert_eq!(REST_CHARGE_AH[first].to_bits(), CHARGE_TOP_AH[block].to_bits());
            assert!(REST_CHARGE_AH[last] >= CHARGE_DEEP_AH[block]);
            assert!(REST_CHARGE_AH[last - 1] < CHARGE_DEEP_AH[block]);
            assert!(CHARGE_DEEP_AH[block] <= CAPACITY_AH[block]);
            // Every rest voltage lies between the two voltage limits.
            let mut j = first;
            while j <= last {
                assert!(REST_V[j] > LOWER_VOLTAGE_LIMIT_V && REST_V[j] < UPPER_VOLTAGE_LIMIT_V);
                j += 1;
            }
            // Charge level falls along the resistance tables; every resistance is positive.
            let mut row = 0;
            while row < LEVEL_POINTS {
                if row > 0 {
                    assert!(LEVEL_CHARGE_LEVEL[block][row] < LEVEL_CHARGE_LEVEL[block][row - 1]);
                }
                assert!(FIRST_SECOND_OHM[block][row] > 0.0);
                let mut k = 0;
                while k < CURRENT_SIZES {
                    assert!(TEN_SECOND_OHM[block][row][k] > 0.0);
                    k += 1;
                }
                row += 1;
            }
            let mut row = 0;
            while row < HELD_POINTS {
                if row > 0 {
                    assert!(HELD_CHARGE_LEVEL[block][row] < HELD_CHARGE_LEVEL[block][row - 1]);
                }
                assert!(HELD_OHM[block][row] > 0.0);
                row += 1;
            }
            assert!(TIME_CONSTANT_S[block] > 0.0);
            assert_eq!(BLOCK_ENERGY_J[block].to_bits(), ENERGY_AT_REST_POINT_J[first].to_bits());
            assert_eq!(ENERGY_AT_REST_POINT_J[last].to_bits(), 0.0_f64.to_bits());
            offset += REST_POINTS[block];
            block += 1;
        }
        assert_eq!(offset, REST_TOTAL);
        let mut k = 0;
        while k < CURRENT_SIZES - 1 {
            assert!(CURRENT_SIZE_A[k + 1] > CURRENT_SIZE_A[k]);
            k += 1;
        }

        // The case temperatures of a warmer block all lie above those of the
        // next colder block, at every row, so the two blocks either side of a
        // case temperature are found in block order at every charge level.
        let mut block = 0;
        while block < BLOCK_COUNT - 1 {
            let mut coldest_of_warmer = f64::INFINITY;
            let mut warmest_of_colder = f64::NEG_INFINITY;
            for value in PULSE_CASE_C[block].iter().chain(HELD_CASE_C[block].iter()) {
                if *value < coldest_of_warmer {
                    coldest_of_warmer = *value;
                }
            }
            for value in PULSE_CASE_C[block + 1].iter().chain(HELD_CASE_C[block + 1].iter()) {
                if *value > warmest_of_colder {
                    warmest_of_colder = *value;
                }
            }
            assert!(
                coldest_of_warmer - warmest_of_colder > 3.0,
                "blocks {block} and the next: {coldest_of_warmer} and {warmest_of_colder}"
            );
            assert!(TIME_CONSTANT_CASE_C[block] - TIME_CONSTANT_CASE_C[block + 1] > 3.0);
            block += 1;
        }

        // The energy of each block's table: about capacity times mean voltage.
        let expect_kj = [61.9, 59.9, 56.9, 56.2, 54.4, 52.6];
        let mut block = 0;
        while block < BLOCK_COUNT {
            let got_kj = BLOCK_ENERGY_J[block] / 1000.0;
            assert!(
                abs_f64(got_kj - expect_kj[block]) < 1.5,
                "block {block} energy {got_kj} kJ"
            );
            block += 1;
        }

        // One digest over every table bit, in the order of the tables file.
        let mut digest = 0_u64;
        let mut fold = |values: &[f64]| {
            for value in values {
                digest = mix(digest, value.to_bits());
            }
        };
        fold(&BLOCK_CHAMBER_C);
        fold(&CAPACITY_AH);
        fold(&CHARGE_TOP_AH);
        fold(&CHARGE_DEEP_AH);
        fold(&REST_CHARGE_AH);
        fold(&REST_V);
        for block in 0..BLOCK_COUNT {
            fold(&LEVEL_CHARGE_LEVEL[block]);
            fold(&FIRST_SECOND_OHM[block]);
            fold(&PULSE_CASE_C[block]);
            for row in 0..LEVEL_POINTS {
                fold(&TEN_SECOND_OHM[block][row]);
            }
            fold(&HELD_CHARGE_LEVEL[block]);
            fold(&HELD_OHM[block]);
            fold(&HELD_CASE_C[block]);
        }
        fold(&CURRENT_SIZE_A);
        fold(&TIME_CONSTANT_S);
        fold(&TIME_CONSTANT_CASE_C);
        // The same fold over the numbers of the constants file gives this value.
        assert_eq!(digest, 0x42db0c20f0b6ff9b);
    }

    #[test]
    fn a_start_outside_the_card_builds_no_state() {
        let cell = Cell2170M80::cell();
        for block in 0..BLOCK_COUNT {
            let chamber = BLOCK_CHAMBER_C[block];
            let top = CHARGE_TOP_AH[block];
            let deep = CHARGE_DEEP_AH[block];
            assert_eq!(cell.served_block_of(chamber), Some(block));
            // The two ends of the tables are legal starts; the next number beyond each is not.
            let at_top = CellState::try_start(&cell, chamber, top).unwrap();
            assert_eq!(at_top.charge_removed_ah.to_bits(), top.to_bits());
            assert_eq!(at_top.terminal_v.to_bits(), REST_V[REST_FIRST[block]].to_bits());
            assert_eq!(at_top.energy_anchor_j.to_bits(), BLOCK_ENERGY_J[block].to_bits());
            let at_deep = CellState::try_start(&cell, chamber, deep).unwrap();
            assert_eq!(at_deep.charge_removed_ah.to_bits(), deep.to_bits());
            assert!(at_deep.terminal_v > LOWER_VOLTAGE_LIMIT_V);
            // Past an end, a start is taken within the rounding allowance of
            // a start, 2^-49 Ah, read at the end point, and not beyond it.
            let allowance = cell.charge_end_allowance_ah(0.0);
            let past_top = top - allowance;
            let past_deep = deep + allowance;
            let at_past_top = CellState::try_start(&cell, chamber, past_top).unwrap();
            assert_eq!(at_past_top.charge_removed_ah.to_bits(), past_top.to_bits());
            assert_eq!(at_past_top.open_circuit_v.to_bits(), at_top.open_circuit_v.to_bits());
            assert_eq!(at_past_top.chemical_energy_j.to_bits(), at_top.chemical_energy_j.to_bits());
            let at_past_deep = CellState::try_start(&cell, chamber, past_deep).unwrap();
            assert_eq!(at_past_deep.open_circuit_v.to_bits(), at_deep.open_circuit_v.to_bits());
            assert_eq!(at_past_deep.chemical_energy_j.to_bits(), at_deep.chemical_energy_j.to_bits());
            assert!(CellState::try_start(&cell, chamber, next_down(past_top)).is_none());
            assert!(CellState::try_start(&cell, chamber, next_up(past_deep)).is_none());
            assert!(CellState::try_start(&cell, chamber, top - 1.0e-9).is_none());
            assert!(CellState::try_start(&cell, chamber, deep + 1.0e-9).is_none());
            assert!(CellState::try_start(&cell, chamber, f64::NAN).is_none());
            assert!(CellState::try_start(&cell, chamber, f64::INFINITY).is_none());
            // A start is at rest: nothing in the branch, no work, and the ledger anchored.
            let rest = CellState::try_start(&cell, chamber, 1.0).unwrap();
            assert_eq!(rest.branch_v.to_bits(), 0.0_f64.to_bits());
            assert_eq!(rest.current_a.to_bits(), 0.0_f64.to_bits());
            assert_eq!(rest.electrical_work_j.to_bits(), 0.0_f64.to_bits());
            assert_eq!(rest.chemical_energy_j.to_bits(), rest.energy_anchor_j.to_bits());
            assert_eq!(rest.terminal_v.to_bits(), rest.open_circuit_v.to_bits());
            assert_eq!(rest.block_chamber_c.to_bits(), chamber.to_bits());
            assert!(close(rest.open_circuit_v, plain_rest_v(block, 1.0), 1.0e-15));
        }
        // A chamber setting that names no block builds nothing, and nothing
        // stands in for it.
        for chamber in [15.0, 24.999999999999996, 25.000000000000004, 41.0, -30.0, f64::NAN] {
            assert!(CellState::try_start(&cell, chamber, 1.0).is_none(), "{chamber}");
            assert!(cell.block_of(chamber).is_none(), "{chamber}");
        }
    }

    #[test]
    fn an_input_outside_the_card_is_refused_by_name_and_nothing_moves() {
        let cell = Cell2170M80::cell();
        let mut state = CellState::try_start(&cell, 40.0, 1.0).unwrap();
        let mut frame = blank_frame();
        // Land two steps first, so that the state holds a branch voltage and a ledger.
        step(&cell, &mut state, &mut frame, load(-2.25, 40.0), 1.0).unwrap();
        step(&cell, &mut state, &mut frame, load(-2.25, 40.0), 1.0).unwrap();
        let saved = state_bits(&state);
        let held = frame.to_bytes();
        let mut refuse = |with: CellLoad, dt_s: f64, expect: StepReject| {
            let got = step(&cell, &mut state, &mut frame, with, dt_s);
            assert_eq!(got, Err(expect), "{with:?} {dt_s}");
            assert_eq!(state_bits(&state), saved, "{with:?} {dt_s}");
            assert_eq!(frame.to_bytes(), held, "{with:?} {dt_s}");
            proof_intact(&frame);
        };
        // Not a number, or not finite.
        refuse(load(f64::NAN, 40.0), 1.0, StepReject::NonFiniteInput);
        refuse(load(f64::INFINITY, 40.0), 1.0, StepReject::NonFiniteInput);
        refuse(load(-1.0, f64::NAN), 1.0, StepReject::NonFiniteInput);
        refuse(load(-1.0, f64::NEG_INFINITY), 1.0, StepReject::NonFiniteInput);
        refuse(load(-1.0, 40.0), f64::NAN, StepReject::NonFiniteInput);
        refuse(load(-1.0, 40.0), f64::INFINITY, StepReject::NonFiniteInput);
        // The step.
        refuse(load(-1.0, 40.0), 0.0, StepReject::NonPositiveDt);
        refuse(load(-1.0, 40.0), -1.0, StepReject::NonPositiveDt);
        refuse(load(-1.0, 40.0), 0.5, StepReject::StepNotOneSecond);
        refuse(load(-1.0, 40.0), 2.0, StepReject::StepNotOneSecond);
        refuse(load(-1.0, 40.0), next_up(1.0), StepReject::StepNotOneSecond);
        refuse(load(-1.0, 40.0), next_down(1.0), StepReject::StepNotOneSecond);
        // The case temperature: the next number beyond each bound of the card.
        refuse(
            load(-1.0, next_down(CASE_TEMPERATURE_MIN_C)),
            1.0,
            StepReject::CaseTemperatureOutsideCard,
        );
        refuse(
            load(-1.0, next_up(CASE_TEMPERATURE_MAX_C)),
            1.0,
            StepReject::CaseTemperatureOutsideCard,
        );
        refuse(load(-1.0, 60.0), 1.0, StepReject::CaseTemperatureOutsideCard);
        refuse(load(-1.0, -273.15), 1.0, StepReject::CaseTemperatureOutsideCard);
        refuse(load(-1.0, -25.0), 1.0, StepReject::CaseTemperatureOutsideCard);
        // The current: the next number beyond 13.5 A, either way.
        refuse(load(next_up(CURRENT_CHARGE_MAX_A), 40.0), 1.0, StepReject::CurrentOutsideCard);
        refuse(load(next_down(-CURRENT_DISCHARGE_MAX_A), 40.0), 1.0, StepReject::CurrentOutsideCard);
        refuse(load(-100.0, 40.0), 1.0, StepReject::CurrentOutsideCard);
        // An input outside the card is named before a fault of the step it comes with.
        refuse(load(f64::NAN, 60.0), 0.5, StepReject::NonFiniteInput);
        refuse(load(-100.0, 60.0), 1.0, StepReject::CaseTemperatureOutsideCard);
        refuse(load(-100.0, 25.0), 1.0, StepReject::CurrentOutsideCard);

        // The limits themselves are on the card: each lands, and the state
        // records the value sent, not a neighbour of it.
        for (with, label) in [
            (load(-1.0, BLOCK_CASE_MIN_C[0]), "coldest of the block"),
            (load(-1.0, CASE_TEMPERATURE_MAX_C), "warmest"),
            (load(-1.0, CASE_TEMPERATURE_MIN_C), "coldest of the card"),
            (load(-BLOCK_CURRENT_DISCHARGE_MAX_A[0], 40.0), "largest discharge of the block"),
            (load(-CURRENT_DISCHARGE_MAX_A, 40.0), "largest discharge of the card"),
        ] {
            let mut on_limit = CellState::try_start(&cell, 40.0, 1.0).unwrap();
            let mut fresh = blank_frame();
            step(&cell, &mut on_limit, &mut fresh, with, 1.0)
                .unwrap_or_else(|reject| panic!("{label}: {reject:?}"));
            assert_eq!(on_limit.current_a.to_bits(), with.current_a.to_bits());
            assert_eq!(
                on_limit.case_temperature_c.to_bits(),
                with.case_temperature_c.to_bits()
            );
            proof_intact(&fresh);
        }
        // The largest charging current is on the card too: away from the top
        // of the 40 degC block it lands.
        let mut charging = CellState::try_start(&cell, 40.0, 2.0).unwrap();
        let mut fresh = blank_frame();
        step(&cell, &mut charging, &mut fresh, load(CURRENT_CHARGE_MAX_A, 40.0), 1.0).unwrap();
        assert!(charging.charge_removed_ah < 2.0);
    }

    /// Every block takes every case temperature of the card: inside its own
    /// compared band a step lands with no flag of the tests, and outside it,
    /// at each other block's chamber setting and at the card's two ends, it
    /// lands with FLAG_OUTSIDE_COMPARED_TESTS; the ledger closes on each.
    #[test]
    fn each_block_takes_every_case_temperature_of_the_card_flagged_outside_its_tests() {
        let cell = Cell2170M80::cell();
        let mut flagged = 0;
        for block in 0..BLOCK_COUNT {
            let chamber = BLOCK_CHAMBER_C[block];
            let low = BLOCK_CASE_MIN_C[block];
            let high = BLOCK_CASE_MAX_C[block];
            let no_mark = if BLOCK_MET_NO_MARK[block] { FLAG_BLOCK_MET_NO_MARK } else { 0 };
            for case_c in [low, high, 0.5 * (low + high)] {
                let mut state = CellState::try_start(&cell, chamber, 1.0).unwrap();
                let mut frame = blank_frame();
                step(&cell, &mut state, &mut frame, load(-1.0, case_c), 1.0)
                    .unwrap_or_else(|reject| panic!("{chamber} at {case_c}: {reject:?}"));
                assert_eq!(state.case_temperature_c.to_bits(), case_c.to_bits());
                assert_eq!(frame.flags, no_mark);
            }
            let mut outside = std::vec::Vec::new();
            for other in 0..BLOCK_COUNT {
                outside.push(BLOCK_CHAMBER_C[other]);
            }
            outside.push(CASE_TEMPERATURE_MIN_C);
            outside.push(CASE_TEMPERATURE_MAX_C);
            outside.push(next_down(low));
            outside.push(next_up(high));
            for case_c in outside {
                if case_c >= low && case_c <= high || case_c < CASE_TEMPERATURE_MIN_C || case_c > CASE_TEMPERATURE_MAX_C {
                    continue;
                }
                let mut state = CellState::try_start(&cell, chamber, 1.0).unwrap();
                let mut frame = blank_frame();
                for _ in 0..60 {
                    step(&cell, &mut state, &mut frame, load(-1.0, case_c), 1.0)
                        .unwrap_or_else(|reject| panic!("{chamber} at {case_c}: {reject:?}"));
                    assert_eq!(frame.flags, no_mark | FLAG_OUTSIDE_COMPARED_TESTS);
                    assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                }
                flagged += 1;
            }
        }
        assert!(flagged > 30, "{flagged}");
    }

    /// Every block takes currents to 13.5 A either way: its own largest
    /// compared currents land with no flag of the tests, the next number
    /// beyond each and the card's 13.5 A land with the flag, the ledger
    /// closing, and a step past 13.5 A is refused by name.
    #[test]
    fn each_block_takes_currents_to_the_end_of_the_tables_flagged_outside_its_tests() {
        let cell = Cell2170M80::cell();
        for block in 0..BLOCK_COUNT {
            let chamber = BLOCK_CHAMBER_C[block];
            let case_c = 0.5 * (BLOCK_CASE_MIN_C[block] + BLOCK_CASE_MAX_C[block]);
            let discharge = BLOCK_CURRENT_DISCHARGE_MAX_A[block];
            let charge = BLOCK_CURRENT_CHARGE_MAX_A[block];
            let no_mark = if BLOCK_MET_NO_MARK[block] { FLAG_BLOCK_MET_NO_MARK } else { 0 };
            for current_a in [-discharge, charge, 0.0] {
                let mut state = CellState::try_start(&cell, chamber, 2.0).unwrap();
                let mut frame = blank_frame();
                step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0)
                    .unwrap_or_else(|reject| panic!("{chamber} at {current_a} A: {reject:?}"));
                assert_eq!(state.current_a.to_bits(), current_a.to_bits());
                assert_eq!(frame.flags, no_mark);
            }
            for current_a in [next_down(-discharge), next_up(charge), -CURRENT_DISCHARGE_MAX_A, CURRENT_CHARGE_MAX_A] {
                let mut state = CellState::try_start(&cell, chamber, 2.0).unwrap();
                let mut frame = blank_frame();
                match step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0) {
                    Ok(()) => {
                        assert_eq!(frame.flags, no_mark | FLAG_OUTSIDE_COMPARED_TESTS, "{chamber} at {current_a} A");
                        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                    }
                    // a large charge in the coldest blocks can cross 4.242 V at once
                    Err(reject) => assert_eq!(reject, StepReject::UpperVoltageLimit, "{chamber} at {current_a} A"),
                }
            }
            for current_a in [next_down(-CURRENT_DISCHARGE_MAX_A), next_up(CURRENT_CHARGE_MAX_A)] {
                let mut state = CellState::try_start(&cell, chamber, 2.0).unwrap();
                let saved = state_bits(&state);
                let mut frame = blank_frame();
                let held = frame.to_bytes();
                assert_eq!(step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0), Err(StepReject::CurrentOutsideCard));
                assert_eq!(state_bits(&state), saved);
                assert_eq!(frame.to_bytes(), held);
                proof_intact(&frame);
            }
        }
    }

    /// The -10 and -20 degC blocks, refused through engine 0.9.0, are served
    /// with the flag that no compared test met its marks. A C/3 and a 1C
    /// discharge from the recharge in each, at the block's own case
    /// temperature, land every step with the ledger closing, and stop by name
    /// at 2.8 V or at the deepest point of the block's table.
    #[test]
    fn the_blocks_that_met_no_mark_are_served_with_the_flag() {
        let cell = Cell2170M80::cell();
        assert_eq!(BLOCK_CHAMBER_C[4], -10.0);
        assert_eq!(BLOCK_CHAMBER_C[5], -20.0);
        for block in 0..BLOCK_COUNT {
            if !BLOCK_MET_NO_MARK[block] {
                continue;
            }
            let chamber = BLOCK_CHAMBER_C[block];
            let case_c = 0.5 * (BLOCK_CASE_MIN_C[block] + BLOCK_CASE_MAX_C[block]);
            for current_a in [-1.5, -4.5] {
                let mut state = CellState::try_start(&cell, chamber, 0.0).unwrap();
                let mut frame = blank_frame();
                let mut landed = 0u32;
                let mut worst = 0.0_f64;
                let reject = loop {
                    let before = state_bits(&state);
                    match step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0) {
                        Ok(()) => {
                            landed += 1;
                            assert_eq!(frame.flags & FLAG_BLOCK_MET_NO_MARK, FLAG_BLOCK_MET_NO_MARK);
                            worst = worst.max(state.residual_conservation_error);
                        }
                        Err(reject) => {
                            assert_eq!(state_bits(&state), before);
                            break reject;
                        }
                    }
                };
                assert!(landed > 100, "{chamber} {current_a} {landed}");
                assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
                assert!(
                    reject == StepReject::LowerVoltageLimit || reject == StepReject::ChargeOutsideTables,
                    "{chamber} {current_a} {reject:?}"
                );
            }
        }
    }

    /// A start, in units of 1e-14 Ah, written as a decimal and read the way a
    /// request is read.
    fn decimal_ah(units_1e14: i64) -> f64 {
        std::format!("{units_1e14}e-14").parse::<f64>().unwrap()
    }

    #[test]
    fn a_run_that_ends_exactly_on_an_end_of_the_tables_lands_its_last_step() {
        let cell = Cell2170M80::cell();
        // Each end of each served block's tables in units of 1e-14 Ah.
        let mut ran = 0;
        let mut whole = 0;
        for block in 0..BLOCK_COUNT {
            if BLOCK_MET_NO_MARK[block] {
                continue;
            }
            let chamber = BLOCK_CHAMBER_C[block];
            let case_c = 0.5 * (BLOCK_CASE_MIN_C[block] + BLOCK_CASE_MAX_C[block]);
            let top_units = (CHARGE_TOP_AH[block] * 1.0e14).round() as i64;
            let deep_units = (CHARGE_DEEP_AH[block] * 1.0e14).round() as i64;
            assert_eq!(decimal_ah(top_units).to_bits(), CHARGE_TOP_AH[block].to_bits());
            assert_eq!(decimal_ah(deep_units).to_bits(), CHARGE_DEEP_AH[block].to_bits());
            // (end, current in A, its charge for one second in units of 1e-14 Ah)
            let ends = [
                (top_units, 0.36, -10_000_000_000_i64),
                (top_units, 0.72, -20_000_000_000),
                (top_units, 1.08, -30_000_000_000),
                (deep_units, -0.36, 10_000_000_000),
                (deep_units, -0.9, 25_000_000_000),
                (deep_units, -1.8, 50_000_000_000),
            ];
            for (end_units, current_a, per_step_units) in ends {
                for steps in 1..=60_i64 {
                    let start_units = end_units - steps * per_step_units;
                    let start_ah = decimal_ah(start_units);
                    let Some(mut state) = CellState::try_start(&cell, chamber, start_ah) else {
                        continue;
                    };
                    let mut frame = blank_frame();
                    let with = load(current_a, case_c);
                    let mut landed = 0;
                    let mut stopped = None;
                    while landed < steps {
                        match step(&cell, &mut state, &mut frame, with, 1.0) {
                            Ok(()) => landed += 1,
                            Err(reject) => {
                                stopped = Some(reject);
                                break;
                            }
                        }
                    }
                    ran += 1;
                    match stopped {
                        None => {
                            whole += 1;
                            // The run ends on the end in decimal, within the allowance.
                            let end_ah = decimal_ah(end_units);
                            let off = abs_f64(state.charge_removed_ah - end_ah);
                            assert!(off <= cell.charge_end_allowance_ah(state.t_s), "block {block} {current_a} A {steps} steps: {off}");
                            // Read at the end point, or within rounding of it.
                            let at_end = CellState::try_start(&cell, chamber, end_ah).unwrap();
                            assert!(abs_f64(state.open_circuit_v - at_end.open_circuit_v) < 1.0e-12);
                            assert!(state.residual_conservation_error < 1.0e-13);
                            // One step more leaves the tables and is refused by name.
                            let held = state_bits(&state);
                            assert_eq!(step(&cell, &mut state, &mut frame, with, 1.0), Err(StepReject::ChargeOutsideTables));
                            assert_eq!(state_bits(&state), held);
                        }
                        Some(reject) => {
                            // Only a voltage limit may stop it short of the end.
                            assert!(
                                reject == StepReject::LowerVoltageLimit || reject == StepReject::UpperVoltageLimit,
                                "block {block}: {current_a} A for {steps} steps stopped after {landed} with {reject:?}"
                            );
                        }
                    }
                }
            }
        }
        assert!(ran > 1_000, "{ran}");
        assert!(whole > 900, "{whole} of {ran}");
    }

    #[test]
    fn two_runs_that_end_on_an_end_of_the_charge_tables_land_whole() {
        let cell = Cell2170M80::cell();
        // From -0.019 Ah, 1.2 A charging for three seconds ends on -0.02 Ah.
        let mut state = CellState::try_start(&cell, 25.0, -0.019).unwrap();
        let mut frame = blank_frame();
        for _ in 0..3 {
            step(&cell, &mut state, &mut frame, load(1.2, 25.0), 1.0).unwrap();
        }
        assert!(abs_f64(state.charge_removed_ah - -0.02) <= cell.charge_end_allowance_ah(3.0));
        assert_eq!(state.open_circuit_v.to_bits(), REST_V[REST_FIRST[1]].to_bits());
        assert_eq!(
            step(&cell, &mut state, &mut frame, load(1.2, 25.0), 1.0),
            Err(StepReject::ChargeOutsideTables)
        );
        // From 4.4605 Ah, 0.9 A discharging for four seconds ends on 4.4615 Ah.
        let mut state = CellState::try_start(&cell, 25.0, 4.4605).unwrap();
        for _ in 0..4 {
            step(&cell, &mut state, &mut frame, load(-0.9, 25.0), 1.0).unwrap();
        }
        assert!(state.charge_removed_ah > CHARGE_DEEP_AH[1]);
        assert!(state.charge_removed_ah - CHARGE_DEEP_AH[1] <= cell.charge_end_allowance_ah(4.0));
        let at_end = CellState::try_start(&cell, 25.0, CHARGE_DEEP_AH[1]).unwrap();
        assert_eq!(state.open_circuit_v.to_bits(), at_end.open_circuit_v.to_bits());
        assert_eq!(state.chemical_energy_j.to_bits(), at_end.chemical_energy_j.to_bits());
        assert!(state.residual_conservation_error < 1.0e-14);
        // A new run starts from the charge this run returned: it lies within
        // the rounding allowance of a start, and is read at the end point.
        let restarted = CellState::try_start(&cell, 25.0, state.charge_removed_ah).unwrap();
        assert_eq!(restarted.charge_removed_ah.to_bits(), state.charge_removed_ah.to_bits());
        assert_eq!(restarted.open_circuit_v.to_bits(), at_end.open_circuit_v.to_bits());
        assert_eq!(restarted.chemical_energy_j.to_bits(), at_end.chemical_energy_j.to_bits());
        let mut again = restarted;
        let mut again_frame = blank_frame();
        step(&cell, &mut again, &mut again_frame, load(0.9, 25.0), 1.0).unwrap();
        assert!(again.charge_removed_ah < CHARGE_DEEP_AH[1]);
        // A rest from there lands and keeps the charge.
        let mut resting = restarted;
        step(&cell, &mut resting, &mut again_frame, load(0.0, 25.0), 1.0).unwrap();
        assert_eq!(resting.charge_removed_ah.to_bits(), restarted.charge_removed_ah.to_bits());
        assert!(resting.residual_conservation_error < 1.0e-14);
        let mut deeper = restarted;
        assert_eq!(
            step(&cell, &mut deeper, &mut again_frame, load(-0.9, 25.0), 1.0),
            Err(StepReject::ChargeOutsideTables)
        );
        // A run that comes back from there reads the tables again from the end.
        step(&cell, &mut state, &mut frame, load(0.9, 25.0), 1.0).unwrap();
        assert!(state.charge_removed_ah < CHARGE_DEEP_AH[1]);
        assert!(state.residual_conservation_error < 1.0e-14);
        // A landing more than the allowance past the end is refused: a step
        // whose half is smaller than the allowance is held to that half.
        let mut on_end = CellState::try_start(&cell, 25.0, CHARGE_DEEP_AH[1]).unwrap();
        let held = state_bits(&on_end);
        assert_eq!(
            step(&cell, &mut on_end, &mut frame, load(-1.0e-9, 25.0), 1.0),
            Err(StepReject::ChargeOutsideTables)
        );
        assert_eq!(state_bits(&on_end), held);
        // The allowance itself.
        assert_eq!(cell.charge_end_allowance_ah(0.0).to_bits(), (2.0 * CHARGE_END_ALLOWANCE_PER_STEP_AH).to_bits());
        assert_eq!(cell.charge_end_allowance_ah(f64::NAN).to_bits(), 0.0_f64.to_bits());
        assert_eq!(cell.charge_end_allowance_ah(-1.0).to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn a_state_that_is_not_the_cards_is_refused() {
        let cell = Cell2170M80::cell();
        let good = CellState::try_start(&cell, 10.0, 2.0).unwrap();
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let cases: [(fn(&mut CellState), StepReject); 6] = [
            (|s| s.block_chamber_c = 15.0, StepReject::BlockNotRecognized),
            (|s| s.block_chamber_c = 10.000000000000002, StepReject::BlockNotRecognized),
            (|s| s.charge_removed_ah = 4.3, StepReject::ChargeOutsideTables),
            (|s| s.charge_removed_ah = -0.03, StepReject::ChargeOutsideTables),
            (|s| s.charge_removed_ah = f64::NAN, StepReject::NonFiniteState),
            (|s| s.branch_v = f64::INFINITY, StepReject::NonFiniteState),
        ];
        for (index, (change, expect)) in cases.iter().enumerate() {
            let mut state = good;
            change(&mut state);
            let before = state_bits(&state);
            let got = step(&cell, &mut state, &mut frame, load(-1.0, 11.0), 1.0);
            assert_eq!(got, Err(*expect), "case {index}");
            assert_eq!(state_bits(&state), before, "case {index}");
            assert_eq!(frame.to_bytes(), held, "case {index}");
            proof_intact(&frame);
        }
    }

    #[test]
    fn one_step_from_rest_is_the_plain_reading_of_the_tables() {
        let cell = Cell2170M80::cell();
        let mut checked = 0;
        let mut read_off_card = 0;
        let mut pairs_used = [false; BLOCK_COUNT + 1];
        for block in 0..BLOCK_COUNT {
            let top = CHARGE_TOP_AH[block];
            let deep = CHARGE_DEEP_AH[block];
            for fraction in [0.0, 0.004, 0.11, 0.37, 0.5, 0.73, 0.9, 0.97, 0.999] {
                let start_ah = top + fraction * (deep - top);
                for case_c in [-25.0, -19.0, -16.4, -12.0, -5.0, 0.9, 1.7, 6.0, 11.5, 14.0, 18.0, 25.1, 28.0, 33.0, 40.2, 44.0, 50.0] {
                    for current_a in [-13.5, -6.0, -2.25, -1.35, -0.3, 0.0, 0.7, 2.25, 5.0] {
                        let landing_ah = start_ah - current_a / SECONDS_PER_HOUR;
                        let off_card = if case_c < CASE_TEMPERATURE_MIN_C || case_c > CASE_TEMPERATURE_MAX_C {
                            Some(StepReject::CaseTemperatureOutsideCard)
                        } else if current_a > CURRENT_CHARGE_MAX_A || current_a < -CURRENT_DISCHARGE_MAX_A {
                            Some(StepReject::CurrentOutsideCard)
                        } else {
                            None
                        };
                        if let Some(expect) = off_card {
                            // Off the card the step is refused by name. The
                                // tables are still read the plain way there.
                            {
                                let mut state = CellState::try_start(&cell, BLOCK_CHAMBER_C[block], start_ah).unwrap();
                                let saved = state_bits(&state);
                                let mut frame = blank_frame();
                                assert_eq!(step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0), Err(expect));
                                assert_eq!(state_bits(&state), saved);
                            }
                            if landing_ah < top || landing_ah > deep {
                                continue;
                            }
                            let plain = plain_readings(block, landing_ah, case_c, current_a);
                            let readings = read_tables(&cell, block, landing_ah, case_c, current_a).unwrap();
                            let (open_circuit_v, _) = rest_at(&cell, block, landing_ah).unwrap();
                            let what = std::format!("off the card: block {block} from {start_ah} at {case_c} degC and {current_a} A");
                            assert!(close(open_circuit_v, plain.open_circuit_v, 1.0e-13), "{what}");
                            assert!(close(readings.resistance_at_once_ohm, plain.at_once_ohm, 1.0e-12), "{what}");
                            assert!(
                                abs_f64(readings.resistance_branch_ohm - plain.branch_ohm) <= 1.0e-12 * plain.held_ohm,
                                "{what}"
                            );
                            assert!(close(readings.time_constant_s, plain.time_constant_s, 1.0e-13), "{what}");
                            read_off_card += 1;
                            continue;
                        }
                        let mut state = CellState::try_start(&cell, BLOCK_CHAMBER_C[block], start_ah).unwrap();
                        let mut frame = blank_frame();
                        if landing_ah < top || landing_ah > deep {
                            assert_eq!(
                                step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0),
                                Err(StepReject::ChargeOutsideTables)
                            );
                            continue;
                        }
                        let plain = plain_readings(block, landing_ah, case_c, current_a);
                        let gained = 1.0 - (-1.0 / plain.time_constant_s).exp();
                        let branch_v = gained * plain.branch_ohm * current_a;
                        let terminal_v = plain.open_circuit_v + current_a * plain.at_once_ohm + branch_v;
                        let got = step(&cell, &mut state, &mut frame, load(current_a, case_c), 1.0);
                        if terminal_v < LOWER_VOLTAGE_LIMIT_V - 1.0e-9 {
                            assert_eq!(got, Err(StepReject::LowerVoltageLimit));
                            continue;
                        }
                        if terminal_v > UPPER_VOLTAGE_LIMIT_V + 1.0e-9 {
                            assert_eq!(got, Err(StepReject::UpperVoltageLimit));
                            continue;
                        }
                        if got.is_err() {
                            // Within a nanovolt of a limit either answer is the card's.
                            assert!(
                                abs_f64(terminal_v - LOWER_VOLTAGE_LIMIT_V) <= 1.0e-9
                                    || abs_f64(terminal_v - UPPER_VOLTAGE_LIMIT_V) <= 1.0e-9
                            );
                            continue;
                        }
                        let what = std::format!("block {block} from {start_ah} at {case_c} degC and {current_a} A");
                        assert!(close(state.charge_removed_ah, landing_ah, 4.0e-16) || landing_ah == 0.0, "{what}");
                        assert!(close(state.open_circuit_v, plain.open_circuit_v, 1.0e-13), "{what}");
                        assert!(close(state.resistance_at_once_ohm, plain.at_once_ohm, 1.0e-12), "{what}");
                        assert!(
                            abs_f64(state.resistance_branch_ohm - plain.branch_ohm) <= 1.0e-12 * plain.held_ohm,
                            "{what}"
                        );
                        assert!(close(state.time_constant_s, plain.time_constant_s, 1.0e-13), "{what}");
                        assert!(abs_f64(state.branch_v - branch_v) <= 1.0e-12 * (abs_f64(current_a) * plain.held_ohm), "{what}");
                        assert!(abs_f64(state.terminal_v - terminal_v) <= 1.0e-12 * 4.3, "{what}");
                        assert_eq!(state.t_s.to_bits(), 1.0_f64.to_bits());
                        assert!(state.residual_conservation_error <= 1.0e-15, "{what}");
                        assert_eq!(state.lower_limit_margin_v.to_bits(), (state.terminal_v - 2.8).to_bits());
                        assert_eq!(state.upper_limit_margin_v.to_bits(), (4.242 - state.terminal_v).to_bits());
                        // Neither floor is active at this point of the card: at a
                        // steady 1.35 A the two parts add up to the held resistance.
                        if plain.at_once_before_floor_ohm >= plain.at_once_ohm
                            && plain.branch_before_floor_ohm > 0.0
                            && abs_f64(current_a) == HOLD_CURRENT_A
                        {
                            assert!(
                                close(
                                    state.resistance_at_once_ohm + state.resistance_branch_ohm,
                                    plain.held_ohm,
                                    1.0e-12
                                ),
                                "{what}"
                            );
                            pairs_used[BLOCK_COUNT] = true;
                        }
                        proof_intact(&frame);
                        checked += 1;
                    }
                }
            }
            pairs_used[block] = true;
        }
        assert!(checked > 4_000, "only {checked} steps were compared");
        assert!(read_off_card > 900, "only {read_off_card} readings off the card were compared");
        assert!(pairs_used.iter().all(|used| *used));
    }

    #[test]
    fn the_two_parts_of_the_resistance_keep_their_floors() {
        // The part that acts at once: first-second plus the difference of the
        // two ten-second readings, and never under a fifth of the first-second.
        assert_eq!(at_once_ohm(0.05, 0.08, 0.07, 0.2).to_bits(), (0.05_f64 + (0.08 - 0.07)).to_bits());
        assert_eq!(at_once_ohm(0.05, 0.02, 0.07, 0.2).to_bits(), (0.2 * 0.05_f64).to_bits());
        assert_eq!(at_once_ohm(0.05, 0.03, 0.07, 0.2).to_bits(), (0.2 * 0.05_f64).to_bits());
        assert_eq!(at_once_ohm(0.05, 0.031, 0.07, 0.2).to_bits(), (0.05_f64 + (0.031 - 0.07)).to_bits());
        // The part that builds: never under zero.
        assert_eq!(branch_ohm(0.2, 0.05, 0.09, 0.07).to_bits(), (0.2_f64 - 0.05 - (0.09 - 0.07)).to_bits());
        assert_eq!(branch_ohm(0.06, 0.05, 0.09, 0.07).to_bits(), 0.0_f64.to_bits());
        assert_eq!(branch_ohm(0.05, 0.05, 0.07, 0.07).to_bits(), 0.0_f64.to_bits());
        // With nothing building, the branch only fades, and the ledger holds.
        let cell = Cell2170M80::cell();
        let mut state = CellState::try_start(&cell, 25.0, 1.0).unwrap();
        let mut frame = blank_frame();
        step(&cell, &mut state, &mut frame, load(-4.5, 25.0), 1.0).unwrap();
        assert!(state.branch_v < 0.0);
        let before = state.branch_v;
        step(&cell, &mut state, &mut frame, load(0.0, 25.0), 1.0).unwrap();
        assert!(state.branch_v < 0.0 && state.branch_v > before);
    }

    #[test]
    fn the_work_of_the_rest_voltage_is_the_change_in_chemical_energy() {
        let cell = Cell2170M80::cell();
        for block in 0..BLOCK_COUNT {
            let first = REST_FIRST[block];
            let top = CHARGE_TOP_AH[block];
            let deep = CHARGE_DEEP_AH[block];
            let scale = BLOCK_ENERGY_J[block];
            let mut ends: Vec<f64> = Vec::new();
            ends.push(top);
            ends.push(deep);
            // On a point, one number either side of it, and between points.
            for point in [1, 2, 7, 50, 120, REST_POINTS[block] - 2] {
                let at = REST_CHARGE_AH[first + point];
                ends.push(at);
                ends.push(next_up(at));
                ends.push(next_down(at));
                ends.push(at + 0.0071);
            }
            for from in &ends {
                for to in &ends {
                    if *to < top || *to > deep || *from < top || *from > deep {
                        continue;
                    }
                    let work = rest_work_j(&cell, block, *from, *to).unwrap();
                    let (_, energy_from) = rest_at(&cell, block, *from).unwrap();
                    let (_, energy_to) = rest_at(&cell, block, *to).unwrap();
                    assert!(
                        abs_f64(work - (energy_to - energy_from)) <= 1.0e-13 * scale,
                        "block {block} from {from} to {to}: {work} against {}",
                        energy_to - energy_from
                    );
                    if *to == *from {
                        assert_eq!(work.to_bits(), 0.0_f64.to_bits());
                    }
                }
            }
            // Outside the block's points there is no work to state.
            assert!(rest_work_j(&cell, block, next_down(top), 1.0).is_none());
        }
    }

    #[test]
    fn three_endings() {
        let cell = Cell2170M80::cell();
        // (chamber, start, current, case temperature, landed steps, refusal)
        let runs = [
            (40.0, 0.0, -2.25, 40.25, 7_269_u32, StepReject::LowerVoltageLimit),
            (25.0, 0.3, 2.25, 26.0, 189, StepReject::UpperVoltageLimit),
            (40.0, 4.6, -0.225, 41.5, 1_229, StepReject::ChargeOutsideTables),
        ];
        for (chamber, start_ah, current_a, case_c, expect_landed, expect) in runs {
            let mut state = CellState::try_start(&cell, chamber, start_ah).unwrap();
            let mut frame = blank_frame();
            let with = load(current_a, case_c);
            let mut landed = 0_u32;
            let mut worst = 0.0;
            let reject = loop {
                let held = state_bits(&state);
                let held_frame = frame.to_bytes();
                match step(&cell, &mut state, &mut frame, with, 1.0) {
                    Ok(()) => {
                        landed += 1;
                        assert!(landed <= expect_landed, "{chamber}: did not refuse");
                        assert!(state.terminal_v >= LOWER_VOLTAGE_LIMIT_V);
                        assert!(state.terminal_v <= UPPER_VOLTAGE_LIMIT_V);
                        if state.residual_conservation_error > worst {
                            worst = state.residual_conservation_error;
                        }
                    }
                    Err(reject) => {
                        assert_eq!(state_bits(&state), held);
                        assert_eq!(frame.to_bytes(), held_frame);
                        proof_intact(&frame);
                        break reject;
                    }
                }
            };
            assert_eq!(reject, expect, "{chamber} {start_ah} {current_a}");
            assert_eq!(landed, expect_landed, "{chamber} {start_ah} {current_a}");
            assert_eq!(state.t_s, f64::from(expect_landed));
            assert!(worst < 1.0e-13, "residual {worst}");
            // The charge removed is the current times the time.
            let counted = start_ah - current_a * f64::from(landed) / SECONDS_PER_HOUR;
            assert!(abs_f64(state.charge_removed_ah - counted) <= 1.0e-12 * (abs_f64(counted) + 1.0));
            // The refused step is refused again, and still changes nothing.
            let held = state_bits(&state);
            assert_eq!(step(&cell, &mut state, &mut frame, with, 1.0), Err(expect));
            assert_eq!(state_bits(&state), held);
        }
    }

    #[test]
    fn rest_holds_the_charge_and_the_branch_fades() {
        let cell = Cell2170M80::cell();
        let mut state = CellState::try_start(&cell, 0.0, 1.5).unwrap();
        let mut frame = blank_frame();
        for _ in 0..120 {
            step(&cell, &mut state, &mut frame, load(-4.5, 3.0), 1.0).unwrap();
        }
        assert!(state.branch_v < -0.05, "branch {}", state.branch_v);
        let loaded = state;
        for zero in [0.0, -0.0] {
            let mut resting = loaded;
            let mut previous = resting.branch_v;
            for n in 0..900 {
                step(&cell, &mut resting, &mut frame, load(zero, 3.0), 1.0).unwrap();
                assert_eq!(resting.charge_removed_ah.to_bits(), loaded.charge_removed_ah.to_bits());
                assert_eq!(resting.chemical_energy_j.to_bits(), loaded.chemical_energy_j.to_bits());
                assert_eq!(resting.electrical_work_j.to_bits(), loaded.electrical_work_j.to_bits());
                assert_eq!(resting.heat_at_once_j.to_bits(), loaded.heat_at_once_j.to_bits());
                assert_eq!(resting.branch_energy_j.to_bits(), loaded.branch_energy_j.to_bits());
                assert!(resting.branch_v < 0.0 && resting.branch_v > previous, "step {n}");
                // At rest the terminal voltage is the rest voltage plus what the branch still holds.
                assert_eq!(
                    resting.terminal_v.to_bits(),
                    (resting.open_circuit_v + zero * resting.resistance_at_once_ohm + resting.branch_v).to_bits()
                );
                previous = resting.branch_v;
            }
            let expected = loaded.branch_v * (-900.0 / resting.time_constant_s).exp();
            assert!(close(resting.branch_v, expected, 1.0e-10), "{} against {expected}", resting.branch_v);
            assert_eq!(resting.t_s, 1_020.0);
            proof_intact(&frame);
        }
    }

    /// A current that changes every few seconds and reverses, with rests, and
    /// a case temperature that drifts: (seconds, amperes).
    const DRIVE: [(u32, f64); 14] = [
        (20, 0.0),
        (15, -4.5),
        (6, -9.0),
        (10, 3.0),
        (30, -1.35),
        (4, -13.5),
        (8, 6.75),
        (60, 0.0),
        (25, -2.25),
        (5, 9.0),
        (12, -6.2),
        (3, 1.0),
        (40, -0.4),
        (10, 0.0),
    ];

    /// The share of [`DRIVE`] each block is given: the colder the block, the
    /// larger its resistance and the smaller the current that stays between
    /// the two voltage limits.
    /// The 40 and 25 degC blocks take 0.72 and 0.68 of it, so that its largest
    /// current stays inside the currents their own tests were compared at.
    const DRIVE_SHARE: [f64; BLOCK_COUNT] = [0.72, 0.68, 0.7, 0.5, 0.3, 0.15];

    fn drive(
        cell: &Cell2170M80,
        chamber_c: f64,
        start_ah: f64,
        start_case_c: f64,
        rounds: u32,
    ) -> (CellState, LastStateFrame64, f64) {
        let share = DRIVE_SHARE[cell.block_of(chamber_c).unwrap()];
        let mut state = CellState::try_start(cell, chamber_c, start_ah).unwrap();
        let mut frame = blank_frame();
        let mut worst = 0.0;
        let mut second = 0_u32;
        for _ in 0..rounds {
            for (seconds, current_a) in DRIVE {
                for _ in 0..seconds {
                    second += 1;
                    let case_c = start_case_c + 4.0 * f64::from(second) / 1_000.0;
                    step(cell, &mut state, &mut frame, load(share * current_a, case_c), 1.0)
                        .unwrap_or_else(|reject| panic!("{chamber_c}: second {second}: {reject:?}"));
                    if state.residual_conservation_error > worst {
                        worst = state.residual_conservation_error;
                    }
                }
            }
        }
        (state, frame, worst)
    }

    #[test]
    fn a_changing_current_with_rests_keeps_the_ledger_on_every_block() {
        let cell = Cell2170M80::cell();
        for block in 0..BLOCK_COUNT {
            if BLOCK_MET_NO_MARK[block] {
                continue;
            }
            let chamber = BLOCK_CHAMBER_C[block];
            // The case temperature drifts up by six kelvin from just inside the
            // coldest of the block's own band.
            let (state, frame, worst) = drive(&cell, chamber, 2.0, BLOCK_CASE_MIN_C[block] + 0.2, 6);
            assert_eq!(state.t_s, 6.0 * 248.0);
            assert!(worst < 1.0e-13, "block {block}: residual {worst}");
            assert!(worst < RESIDUAL_CONSERVATION_LIMIT);
            // More went out than came back, and the heat is never negative.
            assert!(state.charge_removed_ah > 2.0);
            assert!(state.electrical_work_j < 0.0);
            assert!(state.heat_at_once_j > 0.0);
            assert_eq!(frame.flags, 0);
            proof_intact(&frame);
            // The frame holds the state, each slot rounded to 32 bits.
            assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
            assert_eq!(frame.pos[0].to_bits(), (state.terminal_v as f32).to_bits());
            assert_eq!(frame.pos[1].to_bits(), (state.charge_removed_ah as f32).to_bits());
            assert_eq!(frame.pos[2].to_bits(), (state.current_a as f32).to_bits());
            assert_eq!(frame.vel[0].to_bits(), (state.open_circuit_v as f32).to_bits());
            assert_eq!(frame.vel[1].to_bits(), (state.branch_v as f32).to_bits());
            assert_eq!(frame.vel[2].to_bits(), (state.case_temperature_c as f32).to_bits());
            assert_eq!(frame.force_torque.to_bits(), (state.lower_limit_margin_v as f32).to_bits());
            assert_eq!(frame.residual.to_bits(), (state.residual_conservation_error as f32).to_bits());
        }
    }

    #[test]
    fn repeated_runs_are_the_same_to_the_bit() {
        let cell = Cell2170M80::cell();
        let (first, first_frame, _) = drive(&cell, 10.0, 2.0, 10.5, 4);
        for _ in 0..3 {
            let (again, again_frame, _) = drive(&cell, 10.0, 2.0, 10.5, 4);
            assert_eq!(state_bits(&again), state_bits(&first));
            assert_eq!(again_frame.to_bytes(), first_frame.to_bytes());
        }
        // A second card built the same way steps the same.
        let other = Cell2170M80::cell();
        let (again, _, _) = drive(&other, 10.0, 2.0, 10.5, 4);
        assert_eq!(state_bits(&again), state_bits(&first));
        // The bits themselves, so that a change in the arithmetic, on this
        // machine or another, shows here.
        let mut digest = 0_u64;
        for word in state_bits(&first) {
            digest = mix(digest, word);
        }
        assert_eq!(digest, 0xaf2e25dfedf7b67c);
    }

    #[test]
    fn a_falsified_ledger_rolls_back() {
        let cell = Cell2170M80::cell();
        let mut state = CellState::try_start(&cell, 25.0, 1.0).unwrap();
        let mut frame = blank_frame();
        let with = load(-2.25, 25.0);
        step(&cell, &mut state, &mut frame, with, 1.0).unwrap();
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();
        let gate_j = RESIDUAL_CONSERVATION_LIMIT * BLOCK_ENERGY_J[1];
        assert!(gate_j > 0.059 && gate_j < 0.061, "gate {gate_j}");
        // Work that never went in: the cell holds more energy than it was given.
        let got = step_bias_work(&cell, &mut state, &mut frame, with, 1.0, -2.0 * gate_j);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        // Work that went in and is nowhere: energy destroyed.
        let got = step_bias_work(&cell, &mut state, &mut frame, with, 1.0, 2.0 * gate_j);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        proof_intact(&frame);
        // Half the limit either way still lands.
        let mut inside = state;
        step_bias_work(&cell, &mut inside, &mut frame, with, 1.0, 0.5 * gate_j).unwrap();
        let mut inside = state;
        step_bias_work(&cell, &mut inside, &mut frame, with, 1.0, -0.5 * gate_j).unwrap();
    }

    #[test]
    fn an_euler_step_of_the_branch_is_refused() {
        let cell = Cell2170M80::cell();
        for (chamber, case_c, current_a) in [(10.0, 11.0, -4.5), (0.0, 1.5, -9.0), (40.0, 40.0, -9.809), (25.0, 25.0, -9.0)] {
            let mut state = CellState::try_start(&cell, chamber, 1.0).unwrap();
            let mut frame = blank_frame();
            let with = load(current_a, case_c);
            let mut landed = 0_u32;
            loop {
                assert!(landed < 40, "{chamber}: Euler was not refused within forty steps");
                let held = state_bits(&state);
                let held_frame = frame.to_bytes();
                match step_euler_branch(&cell, &mut state, &mut frame, with, 1.0) {
                    Ok(()) => {
                        landed += 1;
                        // Each landed step moves the defect the same way.
                        assert!(state.energy_defect_j < 0.0, "step {landed}");
                    }
                    Err(StepReject::FirstLawBreach) => {
                        assert_eq!(state_bits(&state), held);
                        assert_eq!(frame.to_bytes(), held_frame);
                        proof_intact(&frame);
                        break;
                    }
                    Err(other) => panic!("{chamber}: Euler refused with {other:?} after {landed}"),
                }
            }
            // The card's own step lands the same stretch with a residual at rounding.
            let mut honest = CellState::try_start(&cell, chamber, 1.0).unwrap();
            for _ in 0..=landed {
                step(&cell, &mut honest, &mut frame, with, 1.0).unwrap();
                assert!(honest.residual_conservation_error < 1.0e-14);
            }
        }
    }

    #[test]
    fn work_by_the_rectangle_rule_adds_up_to_a_refusal() {
        let cell = Cell2170M80::cell();
        let mut state = CellState::try_start(&cell, 25.0, 0.0).unwrap();
        let mut frame = blank_frame();
        let with = load(-2.25, 25.0);
        let mut previous = 0.0;
        let mut landed = 0_u32;
        loop {
            let held = state_bits(&state);
            let held_frame = frame.to_bytes();
            match step_rectangle_work(&cell, &mut state, &mut frame, with, 1.0) {
                Ok(()) => {
                    landed += 1;
                    assert!(
                        state.energy_defect_j < previous,
                        "step {landed} defect {}",
                        state.energy_defect_j
                    );
                    previous = state.energy_defect_j;
                    assert!(landed < 2_000, "the rectangle rule did not refuse");
                }
                Err(StepReject::FirstLawBreach) => {
                    assert!(landed > 20, "refused after only {landed} steps");
                    assert_eq!(state_bits(&state), held);
                    assert_eq!(frame.to_bytes(), held_frame);
                    proof_intact(&frame);
                    break;
                }
                Err(other) => panic!("the rectangle rule refused with {other:?} after {landed}"),
            }
        }
        // The card's own step runs the same stretch, and far past it, inside the limit.
        let mut honest = CellState::try_start(&cell, 25.0, 0.0).unwrap();
        for _ in 0..(landed + 500) {
            step(&cell, &mut honest, &mut frame, with, 1.0).unwrap();
        }
        assert!(honest.residual_conservation_error < 1.0e-14);
    }

    #[test]
    fn the_proof_is_not_the_cards() {
        let cell = Cell2170M80::cell();
        let mut state = CellState::try_start(&cell, 0.0, 0.5).unwrap();
        let mut frame = blank_frame();
        for _ in 0..4 {
            step(&cell, &mut state, &mut frame, load(-1.0, 2.0), 1.0).unwrap();
            proof_intact(&frame);
        }
        assert_eq!(
            step(&cell, &mut state, &mut frame, load(-14.0, 2.0), 1.0),
            Err(StepReject::CurrentOutsideCard)
        );
        proof_intact(&frame);
        assert_eq!(
            step(&cell, &mut state, &mut frame, load(-1.0, 2.0), 0.0),
            Err(StepReject::NonPositiveDt)
        );
        proof_intact(&frame);
    }

    #[test]
    fn the_step_is_one_second_and_the_card_states_it_as_floor_and_ceiling() {
        let cell = Cell2170M80::cell();
        assert_eq!(cell.gate_dt_s().unwrap().to_bits(), 1.0_f64.to_bits());
        assert_eq!(cell.floor_dt_s().unwrap().to_bits(), 1.0_f64.to_bits());
        let mut state = CellState::try_start(&cell, 25.0, 1.0).unwrap();
        let mut frame = blank_frame();
        step(&cell, &mut state, &mut frame, load(-1.0, 25.0), 1.0).unwrap();
        assert_eq!(state.t_s.to_bits(), 1.0_f64.to_bits());
        // A million seconds on, one more second still moves the clock exactly.
        assert_eq!((1.0e6_f64 + 1.0) - 1.0e6, 1.0);
    }
}
