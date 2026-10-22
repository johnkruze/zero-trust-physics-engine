#![no_implicit_prelude]

//! Single-crystal CMSX-6 at 980 °C, creeping in tension under a held load.
//!
//! Measured by the Bundesanstalt für Materialforschung und -prüfung (BAM),
//! division 5.2, Berlin, in 2023: twelve test pieces of series Vh5205, cast
//! by PCC Airfoils as [001] single-crystal bars, solution annealed and aged,
//! tested to DIN EN ISO 204:2019-4 in a laboratory accredited to ISO 17025.
//! Ávila Calderón, Schriever, Han, Olbricht, Dolabella Portella and
//! Skrotzki, "BAM Reference Data: Creep of Single-Crystal Ni-Based
//! Superalloy CMSX-6", version 2.1, Zenodo record 20132712,
//! DOI 10.5281/zenodo.20132712, CC BY 4.0. No measured value of the record
//! is stored in the card.
//!
//! The test pieces are round, 6 mm in diameter, loaded along the bar at an
//! initial stress of 140, 170, 200 or 230 MPa, three at each stress. The
//! record gives the percentage creep extension against the time since the
//! full load was applied. The card steps that one number.
//!
//! Six constants are fitted, on six of the twelve tests (two at each of 140,
//! 170 and 230 MPa), by least squares on the logarithm of the time to reach
//! 0.1, 0.2, 0.5, 1, 2 and 5 percent creep extension. The values are those of
//! `analysis/out/03_fit_and_score_Q.json`, key `constants`, of the research
//! record `evidence/research/materials_cmsx6_creep/`, rounded by that script
//! to six significant figures. The other six tests, the three at 200 MPa
//! among them, were not read when the constants were fixed.
//!
//! ```text
//! x                 = s / s_ref                                   s the initial stress, MPa
//! creep rate        = A x^n (1 + C0 x^m e) (1 + K exp(−e / e_p))  percent per hour
//! de/dt             = creep rate / 3600                           percent per second
//! ```
//!
//! `e` is the percentage creep extension, `s_ref` is
//! [`REFERENCE_STRESS_MPA`], `A` [`BASE_RATE_PERCENT_PER_H`], `n`
//! [`BASE_STRESS_EXPONENT`], `C0` [`SOFTENING_PER_PERCENT`], `m`
//! [`SOFTENING_STRESS_EXPONENT`], `K` [`PRIMARY_MULTIPLIER`], `e_p`
//! [`PRIMARY_DECAY_EXTENSION_PERCENT`]. `C0 x^m e` is far above 1 at every
//! extension the fit scored, so the law is in effect a rate proportional to
//! the extension already reached, with a stress exponent `n + m` of 6.25, a
//! multiplier that decays over the first 0.04 percent, and a small starting
//! rate `A x^n` that sets how long the extension takes to get going. The
//! exponents 33.4 and −27.2 are fitted numbers of this law and are not
//! physical constants of the alloy.
//!
//! The range the card answers in, how the law did there, and what each end
//! rests on. Every number here is from the measured curves of all twelve
//! tests, read after the held-back six were scored
//! (`analysis/15_declared_range.py` and `analysis/amendment_4.json` of the
//! research record; an amendment made after the scoring, not a blind test).
//! The quantity is the one the protocol registered, the logarithm of the
//! law's time since the full load over the measured time to a level, and the
//! line is the registered worst line, 0.30, looked at on 171 levels from 0.10
//! to 5.00 percent in place of the six registered.
//!
//! - Initial stress 140 to 230 MPa, the span tested.
//! - Creep extension from zero, the moment the full load is applied, to
//!   [`CREEP_EXTENSION_MAX_PERCENT`], 5 percent, the largest registered
//!   level, at every stress. Where the law met the line and where it missed:
//! - From 170 MPa up, from [`CREEP_EXTENSION_MIN_PERCENT`], 0.1 percent, to 5
//!   percent, all nine tests are inside the line at all 171 levels, the worst
//!   0.289 (170 MPa, 0.33 percent).
//! - Below 0.1 percent, at every stress, the law was compared and missed: it
//!   is slow to start against every measured curve. Hours to 0.05 percent,
//!   the law against the three test pieces: 38.9 against 15.4 to 26.7 at
//!   140 MPa, 7.8 against 2.7 to 3.5 at 170 MPa, 1.69 against 0.61 to 0.66 at
//!   200 MPa and 0.245 against 0.06 to 0.08 at 230 MPa, a factor of 1.5 to 4.
//! - Below [`FULL_REACH_STRESS_MIN_MPA`], 170 MPa, past
//!   [`CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT`], 0.2 percent, the law
//!   was compared and missed. At 140 MPa the three tests are inside the line
//!   at every level from 0.10 to 0.25 percent, where the worst is 0.296; one
//!   test is outside it from 0.26 percent, a second from 0.31, all three from
//!   0.66 to where the tests were stopped near 0.8 percent, and at the hour
//!   each test was stopped, 1010 to 1030 h, the law gives 1.70 to 1.78
//!   percent against 0.79 to 0.83 measured: about a factor of 2. No 140 MPa
//!   test went past 0.83 percent, and no test lies between 140 and 170 MPa.
//! - On every frame in those two regions the card sets
//!   [`FLAG_OUTSIDE_MET_RANGE`]. Through engine 0.9 the card refused them.
//! - The clock. `t_s` is the time since the full load was applied, as the law
//!   gives it: a state made at a creep extension starts at the hour the law
//!   itself reaches that extension from zero at the held stress,
//!   [`CreepCmsx6::law_time_since_load_s`] (0.1 percent at 50.79 h at
//!   140 MPa, 11.36 h at 170, 2.96 h at 200 and 0.668 h at 230). That hour is
//!   what was compared with measurement. The time between two extensions is
//!   not: against the measured curves the law's time from one level to the
//!   next is off by up to 1.06 in the logarithm (140 MPa, 0.6 to 0.7
//!   percent), and a clock started at 0.1 percent is outside the line at 5 to
//!   16 of 170 levels at 170, 200 and 230 MPa, all under 0.4 percent. The
//!   law's errors cancel in the time since load and do not cancel in a
//!   difference of two of its times. A caller who subtracts two frames' times
//!   has a number the comparison does not cover.
//! - The stress is held. A state is made at one stress, the clock is the
//!   law's at that stress, and a step under another stress is refused as
//!   [`StepReject::StressNotHeld`]. No test with a changed stress is in the
//!   record.
//!
//! The card has no limit in time: it ends in extension at every stress, the
//! latest hour it can reach is 1477 h (140 MPa, 5 percent), and the clock
//! starts at a time the law gives, which is no decimal, so no run ends on a
//! time the caller wrote.
//!
//! No end of the range is a number the caller computes, so no allowance in
//! the last place is carried. 0, 0.1 percent, 0.2 percent, 5 percent, 140,
//! 170 and 230 MPa are accepted as written: each comparison is with the
//! 64-bit number nearest that decimal, which is the number the caller's
//! decimal becomes. The upper limit is met by the step: a step that lands
//! exactly on it lands, one that would pass it by one spacing of 64-bit
//! numbers is refused as [`StepReject::CreepExtensionBeyondCard`], and
//! nothing is pulled back. A state below zero extension is
//! [`StepReject::CreepExtensionBelowZero`]: a piece in tension does not
//! shorten by creep.
//!
//! The card leaves out the elastic and the initial plastic extension of
//! loading, rupture, necking,
//! any other temperature, a load that is changed, removed or reversed, any
//! crystal direction other than within 8 degrees of [001], and any other
//! cast.
//!
//! The pass line and where the result is. The protocol
//! (`analysis/prereg.json` with two amendments before the held-back tests
//! were read, a correction, `analysis/amendment_3.json`, after they were
//! scored, and `analysis/amendment_4.json`, which narrows the range the card
//! answers in) fixed, before any curve was fitted: the logarithm of model
//! time over measured time to each level, with a root mean square of at most
//! 0.15, a worst of at most 0.30 and a mean of magnitude at most 0.10. This
//! law met the three in leave-one-test-out prediction on the six opened tests
//! (0.112, 0.279, −0.001), and it met them when the 170 MPa tests were
//! predicted from 140 and 230 MPa alone (0.118, 0.229, −0.039). The first
//! record gave the first forecast as 0.111, 0.269, −0.001 and the second as a
//! miss at 0.229, 0.538, −0.113: the forecast's integral was wrong at the
//! constants those fits reached, and the correction says so.
//! Nothing here is a result of this card.
//!
//! The map is fixed-step fourth-order Runge–Kutta on the creep extension.
//!
//! The ledger is the work of the load. Per unit volume of the gauge length
//! the load does `s Δe / 100` MJ/m³ over a step, and creep stores none of it:
//! it leaves as heat at the power `s · rate / 100`. The heat of a step is the
//! mean power the law gives over the extension the step covered, times the
//! step: `s Δe / 100 · dt / τ`, where `τ = ∫ 3600 de / rate` from the old to
//! the new extension, by five-point Gauss–Legendre quadrature along the
//! extension. The Runge–Kutta step never evaluates `τ`. An exact step has
//! `τ = dt`. The books are empty when a state is made: they count from the
//! start extension, not from the loading.
//!
//! ```text
//! work_by_load_mj_per_m3      = Σ s Δe / 100
//! heat_dissipated_mj_per_m3   = Σ s Δe / 100 · dt / τ
//! work_defect_mj_per_m3       = work − heat
//! residual_conservation_error = |work − heat| / work
//! ```
//!
//! A wrong stage weight, an Euler step, or an extension moved after the step
//! shows in the defect. The ledger shows the arithmetic and the integrator
//! are right. It cannot show the law matches the alloy.
//!
//! The starting clock is `3600 ∫ de / rate` from zero to the start extension.
//! With `C = C0 x^m` and `u = ln(1 + C e)` it is
//! `3600 / (A x^n C) ∫ du / (1 + K exp(−(exp(u) − 1) / (C e_p)))` from 0 to
//! `ln(1 + C e)`, an integrand that is smooth in `u`, taken on
//! [`START_CLOCK_PANELS`] equal panels of five-point Gauss–Legendre. Against
//! integration at 60 digits it is right to 5 parts in 10^14 at 96 starts
//! over the card (`evidence/verification/crosscheck/creep_cmsx6/check_enforced.py`).
//!
//! [`FLAG_CREEP_RATE_RISING`] is set when the law's rate at the end of a step
//! is above its rate at the start. The law's rate falls from 0.1 percent to a
//! minimum near 0.26 percent and rises from there, as a creep curve's rate
//! does after its minimum. It says where on its own curve the law is, and it
//! marks no failure of the law.
//!
//! [`CreepCmsx6::gate_dt_s`] is the step ceiling: `GATE_PHASE_STEP` over the
//! largest value `|d(de/dt)/de|` can take from zero to 5 percent, bounded by
//! `A C0 x^(n+m) (1 + K) + A x^n K / e_p` at 230 MPa, about 50 s: a stability
//! bound of the Runge–Kutta step taken from zero extension. Its accuracy is
//! held by the ledger. From 0.1 percent up one honest step leaves at most
//! 1.7e-10, a twelfth of the limit. Below 0.1 percent the primary multiplier
//! falls over the first 0.04 percent and one step of the ceiling there
//! leaves more than the limit: such a step is refused by the ledger, by name,
//! with the state kept, and a shorter step lands (the largest that lands from
//! zero extension is stated in the engine's description of the card).
//! [`CreepCmsx6::floor_dt_s`] is [`FLOOR_DT_S`], 1 s: the rate is at least
//! `6e-7 e` percent per second from 0.1 percent and `A x^n (1 + K)` at zero,
//! so a step of 1 s moves the extension by more than 1e9 spacings of 64-bit
//! numbers and the quadrature resolves `τ` to nine figures. The public step
//! applies neither. The gateway refuses a step outside them.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. The
//! 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;
use crate::math::{exp, exp_m1, log};

/// Test temperature, °C. Zenodo 20132712, every creep file, "Specified temperature".
pub const TEMPERATURE_C: f64 = 980.0;
/// Stress the fit divides by, MPa: the middle of the tested span. `analysis/common.py`.
pub const REFERENCE_STRESS_MPA: f64 = 185.0;
/// `A`, percent per hour. Fitted; `analysis/out/03_fit_and_score_Q.json`, `constants[0]`.
pub const BASE_RATE_PERCENT_PER_H: f64 = 7.10621e-07;
/// `n`. Fitted; same file, `constants[1]`.
pub const BASE_STRESS_EXPONENT: f64 = 33.4123;
/// `C0`, per percent of creep extension. Fitted; same file, `constants[2]`.
pub const SOFTENING_PER_PERCENT: f64 = 18552.9;
/// `m`. Fitted; same file, `constants[3]`.
pub const SOFTENING_STRESS_EXPONENT: f64 = -27.1579;
/// `K`. Fitted; same file, `constants[4]`.
pub const PRIMARY_MULTIPLIER: f64 = 181.475;
/// `e_p`, percent of creep extension. Fitted; same file, `constants[5]`.
pub const PRIMARY_DECAY_EXTENSION_PERCENT: f64 = 0.037091;
/// Lowest initial stress tested, MPa. Zenodo 20132712, tests 85, 94, 95.
pub const STRESS_MIN_MPA: f64 = 140.0;
/// Highest initial stress tested, MPa. Zenodo 20132712, tests 78, 80, 89.
pub const STRESS_MAX_MPA: f64 = 230.0;
/// The lowest level of the protocol, percent. Below it the law is slow to
/// start against every measured curve, and a frame carries
/// [`FLAG_OUTSIDE_MET_RANGE`].
pub const CREEP_EXTENSION_MIN_PERCENT: f64 = 0.1;
/// Lowest initial stress at which the law met the registered worst line to 5
/// percent, MPa: the lowest stress whose three tests reach 5 percent with the
/// law inside it at every level. Zenodo 20132712, tests 81, 91, 98.
pub const FULL_REACH_STRESS_MIN_MPA: f64 = 170.0;
/// Creep extension beyond which the card refuses, percent: the largest level
/// of the protocol. The measured curves at 170 to 230 MPa go on to rupture,
/// and the law, whose rate grows with the extension, has no rupture in it.
pub const CREEP_EXTENSION_MAX_PERCENT: f64 = 5.0;
/// Creep extension past which, below 170 MPa, the law missed the three
/// 140 MPa tests, percent: the largest level of the protocol at which they
/// are inside the registered worst line with every level under it. Past it
/// below 170 MPa a frame carries [`FLAG_OUTSIDE_MET_RANGE`].
pub const CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT: f64 = 0.2;
/// `|work − heat| / work` must stay at or under this. The largest value one
/// step leaves, over every stress and extension of the card and steps from
/// the floor to the ceiling, is 1.7e-10 (230 MPa, just above 0.1 percent, at
/// the ceiling), and the limit is 12 times it. While the card stepped from
/// zero extension the largest was 8.6e-7 and the limit, by the same rule,
/// 1e-5.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.0e-9;
/// Fraction of the shortest time constant of the law that one step at the ceiling advances.
pub const GATE_PHASE_STEP: f64 = 0.2;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0;
/// Equal panels of five-point Gauss–Legendre in the starting clock.
pub const START_CLOCK_PANELS: u32 = 128;

/// The creep rate at the end of this step is above the rate at its start.
pub const FLAG_CREEP_RATE_RISING: u64 = 1 << 0;
/// The step covers creep extension where the law was compared with the
/// measured curves and missed: below 0.1 percent at any stress, or past 0.2
/// percent below 170 MPa. The same law and the same ledger; the miss is
/// stated in the header and in the engine's description of the card.
pub const FLAG_OUTSIDE_MET_RANGE: u64 = 1 << 1;

const SECONDS_PER_HOUR: f64 = 3600.0;

/// Why a step did not advance the test piece or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` or the held stress was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this alloy's.
    InvalidCard,
    /// The held initial stress is below 140 MPa or above 230 MPa.
    StressOutsideCard,
    /// The stress of the step is not the stress the state was made at.
    StressNotHeld,
    /// The incoming creep extension is below zero: a test piece in tension does not shorten by creep.
    CreepExtensionBelowZero,
    /// The incoming creep extension is above 5 percent, the largest level of the protocol.
    CreepExtensionOutsideCard,
    /// The trial step passed 5 percent creep extension, the largest level of the protocol.
    CreepExtensionBeyondCard,
    /// The trial step did not move the creep extension in 64-bit arithmetic.
    StepTooSmallToResolve,
    /// The trial step did more work by the load than the creep law dissipates in the time.
    SpontaneousGeneration,
    /// The trial step did less work by the load than the creep law dissipates in the time.
    FirstLawBreach,
}

/// The load on the test piece, held through the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreepLoad {
    /// Initial stress `s`: the load over the cross-section before loading, MPa.
    pub stress_mpa: f64,
}

/// The locked alloy. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreepCmsx6 {
    /// Recorded copy of [`TEMPERATURE_C`].
    pub temperature_c: f64,
    /// Recorded copy of [`REFERENCE_STRESS_MPA`].
    pub reference_stress_mpa: f64,
    /// Recorded copy of [`BASE_RATE_PERCENT_PER_H`].
    pub base_rate_percent_per_h: f64,
    /// Recorded copy of [`BASE_STRESS_EXPONENT`].
    pub base_stress_exponent: f64,
    /// Recorded copy of [`SOFTENING_PER_PERCENT`].
    pub softening_per_percent: f64,
    /// Recorded copy of [`SOFTENING_STRESS_EXPONENT`].
    pub softening_stress_exponent: f64,
    /// Recorded copy of [`PRIMARY_MULTIPLIER`].
    pub primary_multiplier: f64,
    /// Recorded copy of [`PRIMARY_DECAY_EXTENSION_PERCENT`].
    pub primary_decay_extension_percent: f64,
    /// Recorded copy of [`STRESS_MIN_MPA`].
    pub stress_min_mpa: f64,
    /// Recorded copy of [`STRESS_MAX_MPA`].
    pub stress_max_mpa: f64,
    /// Recorded copy of [`CREEP_EXTENSION_MIN_PERCENT`].
    pub creep_extension_min_percent: f64,
    /// Recorded copy of [`FULL_REACH_STRESS_MIN_MPA`].
    pub full_reach_stress_min_mpa: f64,
    /// Recorded copy of [`CREEP_EXTENSION_MAX_PERCENT`].
    pub creep_extension_max_percent: f64,
    /// Recorded copy of [`CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT`].
    pub creep_extension_max_below_full_reach_percent: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`GATE_PHASE_STEP`].
    pub gate_phase_step: f64,
    /// Recorded copy of [`FLOOR_DT_S`].
    pub floor_dt_s: f64,
}

impl CreepCmsx6 {
    /// Card: CMSX-6, [001], at 980 °C. The constructor takes no arguments.
    #[must_use]
    pub fn at_980_c() -> Self {
        let alloy = Self {
            temperature_c: TEMPERATURE_C,
            reference_stress_mpa: REFERENCE_STRESS_MPA,
            base_rate_percent_per_h: BASE_RATE_PERCENT_PER_H,
            base_stress_exponent: BASE_STRESS_EXPONENT,
            softening_per_percent: SOFTENING_PER_PERCENT,
            softening_stress_exponent: SOFTENING_STRESS_EXPONENT,
            primary_multiplier: PRIMARY_MULTIPLIER,
            primary_decay_extension_percent: PRIMARY_DECAY_EXTENSION_PERCENT,
            stress_min_mpa: STRESS_MIN_MPA,
            stress_max_mpa: STRESS_MAX_MPA,
            creep_extension_min_percent: CREEP_EXTENSION_MIN_PERCENT,
            full_reach_stress_min_mpa: FULL_REACH_STRESS_MIN_MPA,
            creep_extension_max_percent: CREEP_EXTENSION_MAX_PERCENT,
            creep_extension_max_below_full_reach_percent:
                CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_phase_step: GATE_PHASE_STEP,
            floor_dt_s: FLOOR_DT_S,
        };
        if alloy.is_valid() {
            alloy
        } else {
            ::core::panic!("CMSX-6 creep card failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.temperature_c.to_bits() == TEMPERATURE_C.to_bits()
            && self.reference_stress_mpa.to_bits() == REFERENCE_STRESS_MPA.to_bits()
            && self.base_rate_percent_per_h.to_bits() == BASE_RATE_PERCENT_PER_H.to_bits()
            && self.base_stress_exponent.to_bits() == BASE_STRESS_EXPONENT.to_bits()
            && self.softening_per_percent.to_bits() == SOFTENING_PER_PERCENT.to_bits()
            && self.softening_stress_exponent.to_bits() == SOFTENING_STRESS_EXPONENT.to_bits()
            && self.primary_multiplier.to_bits() == PRIMARY_MULTIPLIER.to_bits()
            && self.primary_decay_extension_percent.to_bits()
                == PRIMARY_DECAY_EXTENSION_PERCENT.to_bits()
            && self.stress_min_mpa.to_bits() == STRESS_MIN_MPA.to_bits()
            && self.stress_max_mpa.to_bits() == STRESS_MAX_MPA.to_bits()
            && self.creep_extension_min_percent.to_bits() == CREEP_EXTENSION_MIN_PERCENT.to_bits()
            && self.full_reach_stress_min_mpa.to_bits() == FULL_REACH_STRESS_MIN_MPA.to_bits()
            && self.creep_extension_max_percent.to_bits() == CREEP_EXTENSION_MAX_PERCENT.to_bits()
            && self.creep_extension_max_below_full_reach_percent.to_bits()
                == CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.gate_phase_step.to_bits() == GATE_PHASE_STEP.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
            && self.reference_stress_mpa > 0.0
            && self.base_rate_percent_per_h > 0.0
            && self.softening_per_percent > 0.0
            && self.primary_multiplier > 0.0
            && self.primary_decay_extension_percent > 0.0
            && self.stress_min_mpa > 0.0
            && self.stress_max_mpa > self.stress_min_mpa
            && self.full_reach_stress_min_mpa > self.stress_min_mpa
            && self.full_reach_stress_min_mpa < self.stress_max_mpa
            && self.creep_extension_min_percent > 0.0
            && self.creep_extension_max_below_full_reach_percent > self.creep_extension_min_percent
            && self.creep_extension_max_percent > self.creep_extension_max_below_full_reach_percent
            && self.residual_conservation_limit > 0.0
            && self.gate_phase_step > 0.0
            && self.floor_dt_s > 0.0
    }

    /// The creep extension beyond which the card refuses at initial stress
    /// `stress_mpa`, percent: 5 at every stress of the card.
    #[must_use]
    pub fn creep_extension_max_percent(&self, _stress_mpa: f64) -> f64 {
        self.creep_extension_max_percent
    }

    /// A step at `stress_mpa` from `from_percent` to `to_percent` covers
    /// creep extension where the law does not follow the measured curves: below 0.1
    /// percent, or past 0.2 percent below 170 MPa. See
    /// [`FLAG_OUTSIDE_MET_RANGE`].
    #[must_use]
    pub fn outside_met_range(&self, stress_mpa: f64, from_percent: f64, to_percent: f64) -> bool {
        from_percent < self.creep_extension_min_percent
            || (stress_mpa < self.full_reach_stress_min_mpa
                && to_percent > self.creep_extension_max_below_full_reach_percent)
    }

    /// Creep rate in percent per hour at initial stress `stress_mpa` and
    /// creep extension `creep_extension_percent`, by the law of the header.
    #[must_use]
    pub fn creep_rate_percent_per_h(&self, stress_mpa: f64, creep_extension_percent: f64) -> f64 {
        let log_x = log(stress_mpa / self.reference_stress_mpa);
        self.base_rate_percent_per_h
            * exp(self.base_stress_exponent * log_x)
            * (1.0
                + self.softening_per_percent
                    * exp(self.softening_stress_exponent * log_x)
                    * creep_extension_percent)
            * (1.0
                + self.primary_multiplier
                    * exp(-creep_extension_percent / self.primary_decay_extension_percent))
    }

    /// The time the law takes from zero creep extension to
    /// `creep_extension_percent` at initial stress `stress_mpa`, s: the
    /// starting clock of the header, on [`START_CLOCK_PANELS`] panels.
    #[must_use]
    pub fn law_time_since_load_s(&self, stress_mpa: f64, creep_extension_percent: f64) -> f64 {
        let log_x = log(stress_mpa / self.reference_stress_mpa);
        let softening = self.softening_per_percent * exp(self.softening_stress_exponent * log_x);
        let upper = log(1.0 + softening * creep_extension_percent);
        let decay = softening * self.primary_decay_extension_percent;
        let width = upper / (START_CLOCK_PANELS as f64);
        let half = 0.5 * width;
        let mut sum = 0.0;
        let mut panel = 0;
        while panel < START_CLOCK_PANELS {
            let centre = ((panel as f64) + 0.5) * width;
            let mut index = 0;
            while index < 5 {
                let u = centre + half * GAUSS_NODE[index];
                sum += GAUSS_WEIGHT[index]
                    / (1.0 + self.primary_multiplier * exp(-exp_m1(u) / decay));
                index += 1;
            }
            panel += 1;
        }
        SECONDS_PER_HOUR * half * sum
            / (self.base_rate_percent_per_h * exp(self.base_stress_exponent * log_x) * softening)
    }

    /// Largest step the card stands behind, about 50 s: `GATE_PHASE_STEP`
    /// over `(A C0 x^(n+m) (1 + K) + A x^n K / e_p) / 3600` at the highest
    /// stress. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let log_x = log(self.stress_max_mpa / self.reference_stress_mpa);
        let fastest_per_s = (self.base_rate_percent_per_h
            * self.softening_per_percent
            * exp((self.base_stress_exponent + self.softening_stress_exponent) * log_x)
            * (1.0 + self.primary_multiplier)
            + self.base_rate_percent_per_h
                * exp(self.base_stress_exponent * log_x)
                * self.primary_multiplier
                / self.primary_decay_extension_percent)
            / SECONDS_PER_HOUR;
        if fastest_per_s.is_finite() && fastest_per_s > 0.0 {
            Some(self.gate_phase_step / fastest_per_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`FLOOR_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        if self.floor_dt_s.is_finite() && self.floor_dt_s > 0.0 {
            Some(self.floor_dt_s)
        } else {
            None
        }
    }
}

/// The test piece under load. Every field but the two the caller gives to
/// [`CreepState::try_new`] is the card's; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreepState {
    /// Time since the full load was applied, as the law gives it.
    pub t_s: f64,
    /// Percentage creep extension `e`: extension since the full load was applied, over the reference length.
    pub creep_extension_percent: f64,
    /// Creep rate at this state under the held stress.
    pub creep_rate_percent_per_h: f64,
    /// Initial stress the state was made at and is held at.
    pub stress_mpa: f64,
    /// Work done by the load per unit volume of the gauge length since the state was made.
    pub work_by_load_mj_per_m3: f64,
    /// Heat the creep law dissipates per unit volume over the same steps.
    pub heat_dissipated_mj_per_m3: f64,
    /// Signed `work − heat`. Positive is extension the law does not supply.
    pub work_defect_mj_per_m3: f64,
    /// `|work − heat| / work`.
    pub residual_conservation_error: f64,
    /// The creep rate at the end of the last step was above the rate at its start.
    pub creep_rate_rising: bool,
    /// The last step covered creep extension where the law missed the
    /// measured curves: see [`FLAG_OUTSIDE_MET_RANGE`].
    pub outside_met_range: bool,
}

impl CreepState {
    /// A test piece held at `stress_mpa` at the given creep extension, with
    /// empty books and the clock at the time the law takes to reach that
    /// extension from zero at that stress, zero at zero extension. `None`
    /// unless the stress is finite and within 140 to 230 MPa and the
    /// extension is finite, at or above zero and at or below 5 percent.
    #[must_use]
    pub fn try_new(alloy: &CreepCmsx6, stress_mpa: f64, creep_extension_percent: f64) -> Option<Self> {
        if !alloy.is_valid()
            || !stress_mpa.is_finite()
            || !creep_extension_percent.is_finite()
            || stress_mpa < alloy.stress_min_mpa
            || stress_mpa > alloy.stress_max_mpa
            || !(creep_extension_percent >= 0.0)
            || creep_extension_percent > alloy.creep_extension_max_percent(stress_mpa)
        {
            return None;
        }
        let t_s = alloy.law_time_since_load_s(stress_mpa, creep_extension_percent);
        let creep_rate_percent_per_h =
            alloy.creep_rate_percent_per_h(stress_mpa, creep_extension_percent);
        if !(t_s.is_finite() && t_s >= 0.0 && creep_rate_percent_per_h.is_finite()) {
            return None;
        }
        Some(Self {
            t_s,
            creep_extension_percent,
            creep_rate_percent_per_h,
            stress_mpa,
            work_by_load_mj_per_m3: 0.0,
            heat_dissipated_mj_per_m3: 0.0,
            work_defect_mj_per_m3: 0.0,
            residual_conservation_error: 0.0,
            creep_rate_rising: false,
            outside_met_range: false,
        })
    }
}

/// Advance one Runge–Kutta step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    alloy: &CreepCmsx6,
    state: &mut CreepState,
    frame: &mut LastStateFrame64,
    load: CreepLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(alloy, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("creep_cmsx6", exceedances(alloy, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    alloy: &CreepCmsx6,
    state: &mut CreepState,
    frame: &mut LastStateFrame64,
    load: CreepLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(alloy, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    alloy: &CreepCmsx6,
    state: &mut CreepState,
    frame: &mut LastStateFrame64,
    load: CreepLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(alloy, state, load, dt_s)?;
    let advanced = rk4(alloy, load.stress_mpa, state.creep_extension_percent, dt_s);
    commit(alloy, state, frame, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut CreepState) {
    state.t_s = normal_or_zero(state.t_s);
    state.creep_extension_percent = normal_or_zero(state.creep_extension_percent);
    state.creep_rate_percent_per_h = normal_or_zero(state.creep_rate_percent_per_h);
    state.stress_mpa = normal_or_zero(state.stress_mpa);
    state.work_by_load_mj_per_m3 = normal_or_zero(state.work_by_load_mj_per_m3);
    state.heat_dissipated_mj_per_m3 = normal_or_zero(state.heat_dissipated_mj_per_m3);
    state.work_defect_mj_per_m3 = normal_or_zero(state.work_defect_mj_per_m3);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

fn validate(
    alloy: &CreepCmsx6,
    state: &CreepState,
    load: CreepLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.stress_mpa.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !alloy.is_valid() {
        return Err(StepReject::InvalidCard);
    }
    if load.stress_mpa < alloy.stress_min_mpa || load.stress_mpa > alloy.stress_max_mpa {
        return Err(StepReject::StressOutsideCard);
    }
    if load.stress_mpa != state.stress_mpa {
        return Err(StepReject::StressNotHeld);
    }
    if !(state.creep_extension_percent >= 0.0) {
        return Err(StepReject::CreepExtensionBelowZero);
    }
    if state.creep_extension_percent > alloy.creep_extension_max_percent(load.stress_mpa) {
        return Err(StepReject::CreepExtensionOutsideCard);
    }
    Ok(())
}

fn commit(
    alloy: &CreepCmsx6,
    state: &mut CreepState,
    frame: &mut LastStateFrame64,
    load: CreepLoad,
    dt_s: f64,
    advanced_extension_percent: f64,
) -> Result<(), StepReject> {
    let saved = *state;
    if !advanced_extension_percent.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if advanced_extension_percent > alloy.creep_extension_max_percent(load.stress_mpa) {
        return Err(StepReject::CreepExtensionBeyondCard);
    }
    let old_extension = state.creep_extension_percent;
    if !(advanced_extension_percent > old_extension) {
        return Err(StepReject::StepTooSmallToResolve);
    }
    let stress = load.stress_mpa;
    let law_time = law_time_s(alloy, stress, old_extension, advanced_extension_percent);
    if !(law_time > 0.0) || !law_time.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    let work_step = stress * (advanced_extension_percent - old_extension) / 100.0;
    let rate_at_start = alloy.creep_rate_percent_per_h(stress, old_extension);
    state.t_s += dt_s;
    state.creep_extension_percent = advanced_extension_percent;
    state.creep_rate_percent_per_h =
        alloy.creep_rate_percent_per_h(stress, advanced_extension_percent);
    state.stress_mpa = stress;
    state.work_by_load_mj_per_m3 += work_step;
    state.heat_dissipated_mj_per_m3 += work_step * dt_s / law_time;
    state.work_defect_mj_per_m3 = state.work_by_load_mj_per_m3 - state.heat_dissipated_mj_per_m3;
    state.creep_rate_rising = state.creep_rate_percent_per_h > rate_at_start;
    state.outside_met_range =
        alloy.outside_met_range(stress, old_extension, advanced_extension_percent);
    if !state_is_finite(state) || !(state.work_by_load_mj_per_m3 > 0.0) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    state.residual_conservation_error =
        state.work_defect_mj_per_m3.abs() / state.work_by_load_mj_per_m3;
    if !state.residual_conservation_error.is_finite() {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > alloy.residual_conservation_limit {
        let created = state.work_defect_mj_per_m3 > 0.0;
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

/// `de/dt` in percent per second.
fn rate_per_s(alloy: &CreepCmsx6, stress_mpa: f64, extension_percent: f64) -> f64 {
    alloy.creep_rate_percent_per_h(stress_mpa, extension_percent) / SECONDS_PER_HOUR
}

/// Classical fourth-order Runge–Kutta on the creep extension.
fn rk4(alloy: &CreepCmsx6, stress_mpa: f64, extension_percent: f64, dt_s: f64) -> f64 {
    let k1 = rate_per_s(alloy, stress_mpa, extension_percent);
    let k2 = rate_per_s(alloy, stress_mpa, extension_percent + 0.5 * dt_s * k1);
    let k3 = rate_per_s(alloy, stress_mpa, extension_percent + 0.5 * dt_s * k2);
    let k4 = rate_per_s(alloy, stress_mpa, extension_percent + dt_s * k3);
    extension_percent + dt_s / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + k4)
}

/// Five-point Gauss–Legendre nodes on [−1, 1].
const GAUSS_NODE: [f64; 5] = [
    -0.906_179_845_938_664,
    -0.538_469_310_105_683_1,
    0.0,
    0.538_469_310_105_683_1,
    0.906_179_845_938_664,
];
/// Five-point Gauss–Legendre weights.
const GAUSS_WEIGHT: [f64; 5] = [
    0.236_926_885_056_189_08,
    0.478_628_670_499_366_47,
    0.568_888_888_888_888_9,
    0.478_628_670_499_366_47,
    0.236_926_885_056_189_08,
];

/// `τ`: the time the law takes from extension `from` to extension `to`, s.
fn law_time_s(alloy: &CreepCmsx6, stress_mpa: f64, from_percent: f64, to_percent: f64) -> f64 {
    let half = 0.5 * (to_percent - from_percent);
    let centre = 0.5 * (to_percent + from_percent);
    let mut sum = 0.0;
    let mut index = 0;
    while index < 5 {
        sum += GAUSS_WEIGHT[index]
            / rate_per_s(alloy, stress_mpa, centre + half * GAUSS_NODE[index]);
        index += 1;
    }
    half * sum
}

fn publish(state: &CreepState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.creep_extension_percent as f32;
    frame.pos[1] = state.creep_rate_percent_per_h as f32;
    frame.pos[2] = state.stress_mpa as f32;
    frame.vel[0] = state.work_by_load_mj_per_m3 as f32;
    frame.vel[1] = state.heat_dissipated_mj_per_m3 as f32;
    frame.vel[2] = state.work_defect_mj_per_m3 as f32;
    frame.force_torque = state.stress_mpa as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.creep_rate_rising {
        flags |= FLAG_CREEP_RATE_RISING;
    }
    if state.outside_met_range {
        flags |= FLAG_OUTSIDE_MET_RANGE;
    }
    frame.flags = flags;
}

fn state_is_finite(state: &CreepState) -> bool {
    state.t_s.is_finite()
        && state.creep_extension_percent.is_finite()
        && state.creep_rate_percent_per_h.is_finite()
        && state.stress_mpa.is_finite()
        && state.work_by_load_mj_per_m3.is_finite()
        && state.heat_dissipated_mj_per_m3.is_finite()
        && state.work_defect_mj_per_m3.is_finite()
        && state.residual_conservation_error.is_finite()
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    RISING_KIND,
    START_LOW_KIND,
    REACH_HIGH_KIND,
];

const RISING_KIND: ExceedanceKind = ExceedanceKind::varying(
    "FLAG_CREEP_RATE_RISING",
    FLAG_CREEP_RATE_RISING,
    "creep_rate_percent_per_h",
    "percent/h",
    Side::Above,
    "the creep rate at the start of the step",
);
const START_LOW_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_MET_RANGE",
    FLAG_OUTSIDE_MET_RANGE,
    "creep_extension_at_step_start_percent",
    "percent",
    Side::Below,
    CREEP_EXTENSION_MIN_PERCENT,
    "0.1 percent creep extension: below it the law does not follow the measured curves at any stress",
);
const REACH_HIGH_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_MET_RANGE",
    FLAG_OUTSIDE_MET_RANGE,
    "creep_extension_percent",
    "percent",
    Side::Above,
    CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT,
    "0.2 percent creep extension at a stress below 170 MPa: past it the law does not follow the measured curves",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The conditions are those of the step's `creep_rate_rising` and of
/// [`CreepCmsx6::outside_met_range`] over the step.
pub fn exceedances(
    alloy: &CreepCmsx6,
    before: &CreepState,
    after: &CreepState,
    load: CreepLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = dt_s;
    let mut records = crate::exceedance::Exceedances::new();
    let stress = load.stress_mpa;
    let from = before.creep_extension_percent;
    let to = after.creep_extension_percent;
    if after.creep_rate_rising {
        let rate_at_start = alloy.creep_rate_percent_per_h(stress, from);
        records.push(Exceedance::at(RISING_KIND, rate_at_start, after.creep_rate_percent_per_h));
    }
    records.push_if(
        from < alloy.creep_extension_min_percent,
        Exceedance::at(START_LOW_KIND, alloy.creep_extension_min_percent, from),
    );
    records.push_if(
        stress < alloy.full_reach_stress_min_mpa && to > alloy.creep_extension_max_below_full_reach_percent,
        Exceedance::at(REACH_HIGH_KIND, alloy.creep_extension_max_below_full_reach_percent, to),
    );
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::println;

    use super::{
        BASE_RATE_PERCENT_PER_H, BASE_STRESS_EXPONENT, CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT,
        CREEP_EXTENSION_MAX_PERCENT, CREEP_EXTENSION_MIN_PERCENT, CreepCmsx6, CreepLoad, CreepState,
        FLAG_CREEP_RATE_RISING, FLAG_OUTSIDE_MET_RANGE, FLOOR_DT_S, FULL_REACH_STRESS_MIN_MPA,
        GATE_PHASE_STEP,
        PRIMARY_DECAY_EXTENSION_PERCENT, PRIMARY_MULTIPLIER, REFERENCE_STRESS_MPA,
        RESIDUAL_CONSERVATION_LIMIT, SOFTENING_PER_PERCENT, SOFTENING_STRESS_EXPONENT,
        START_CLOCK_PANELS, STRESS_MAX_MPA, STRESS_MIN_MPA, StepReject, TEMPERATURE_C, commit,
        law_time_s, rate_per_s, rk4, step, validate,
    };
    use crate::frame::LastStateFrame64;

    /// The forecast of structure Q at the constants of the card:
    /// `analysis/02_structures.py`, `model_times`, quadrature of `de / rate`
    /// on 6000 points from 1e-9 to 5 percent, printed by
    /// `analysis/05_step_size.py` to `analysis/out/05_step_size.log`
    /// ("quadrature times h"). Columns: initial stress MPa, then hours since
    /// the full load to 0.1, 0.2, 0.5, 1, 2 and 5 percent creep extension. No
    /// measured value is in this table.
    const FORECAST_H: [(f64, [f64; 6]); 4] = [
        (140.0, [50.79, 122.415, 478.907, 779.391, 1079.883, 1477.112]),
        (170.0, [11.359, 32.625, 138.469, 227.685, 316.903, 434.843]),
        (200.0, [2.962, 10.636, 48.884, 81.149, 113.424, 156.098]),
        (230.0, [0.668, 3.515, 18.553, 31.647, 44.927, 62.618]),
    ];
    const LEVEL_PERCENT: [f64; 6] = [0.1, 0.2, 0.5, 1.0, 2.0, 5.0];

    /// The law's time from zero creep extension, s, by Python mpmath 1.3.0 at
    /// 50 digits (`mp.quad` of `3600 / rate` with break points at the powers
    /// of ten from 1e-12), at the 64-bit number nearest each extension.
    /// Columns: initial stress MPa, creep extension percent, seconds.
    /// `evidence/verification/crosscheck/creep_cmsx6/`. No measured value is here.
    const START_CLOCK_S: [(f64, f64, f64); 11] = [
        (140.0, 0.1, 182_843.346_857_141_67),
        (140.0, 0.2, 440_691.719_409_433_5),
        (155.0, 0.15, 130_826.911_629_748_37),
        (170.0, 0.1, 40_893.701_749_044),
        (170.0, 0.2, 117_448.559_679_444_81),
        (170.0, 5.0, 1_565_434.553_508_579_7),
        (185.0, 1.0, 479_495.187_775_656_87),
        (200.0, 0.1, 10_663.590_185_026_2),
        (200.0, 2.5, 445_737.084_373_965_04),
        (230.0, 0.1, 2_404.863_811_532_798_8),
        (230.0, 5.0, 225_426.195_337_823_32),
    ];

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

    fn frame_untouched(frame: &LastStateFrame64) {
        assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
    }

    fn assert_copy<T: Copy>() {}

    fn state_bits(state: &CreepState) -> [u64; 9] {
        [
            state.t_s.to_bits(),
            state.creep_extension_percent.to_bits(),
            state.creep_rate_percent_per_h.to_bits(),
            state.stress_mpa.to_bits(),
            state.work_by_load_mj_per_m3.to_bits(),
            state.heat_dissipated_mj_per_m3.to_bits(),
            state.work_defect_mj_per_m3.to_bits(),
            state.residual_conservation_error.to_bits(),
            u64::from(state.creep_rate_rising),
        ]
    }

    fn load(stress_mpa: f64) -> CreepLoad {
        CreepLoad { stress_mpa }
    }

    fn start(stress_mpa: f64, extension_percent: f64) -> (CreepCmsx6, CreepState) {
        let alloy = CreepCmsx6::at_980_c();
        let state = CreepState::try_new(&alloy, stress_mpa, extension_percent).unwrap();
        (alloy, state)
    }

    fn ceiling() -> f64 {
        CreepCmsx6::at_980_c().gate_dt_s().unwrap()
    }

    fn one_under(value: f64) -> f64 {
        f64::from_bits(value.to_bits() - 1)
    }

    fn one_over(value: f64) -> f64 {
        f64::from_bits(value.to_bits() + 1)
    }

    /// The real `validate`, a chosen integrator, the real `commit`.
    fn step_with(
        alloy: &CreepCmsx6,
        state: &mut CreepState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        integrate: impl FnOnce(&CreepCmsx6, f64, f64, f64) -> f64,
    ) -> Result<(), StepReject> {
        let stress = state.stress_mpa;
        validate(alloy, state, load(stress), dt_s)?;
        let advanced = integrate(alloy, stress, state.creep_extension_percent, dt_s);
        commit(alloy, state, frame, load(stress), dt_s, advanced)
    }

    fn euler(alloy: &CreepCmsx6, stress: f64, e: f64, dt_s: f64) -> f64 {
        e + dt_s * rate_per_s(alloy, stress, e)
    }

    /// Midpoint rule, second order.
    fn midpoint(alloy: &CreepCmsx6, stress: f64, e: f64, dt_s: f64) -> f64 {
        let k1 = rate_per_s(alloy, stress, e);
        e + dt_s * rate_per_s(alloy, stress, e + 0.5 * dt_s * k1)
    }

    /// Fourth-order form with the last stage weighted 2 in place of 1.
    fn wrong_stage_weight(alloy: &CreepCmsx6, stress: f64, e: f64, dt_s: f64) -> f64 {
        let k1 = rate_per_s(alloy, stress, e);
        let k2 = rate_per_s(alloy, stress, e + 0.5 * dt_s * k1);
        let k3 = rate_per_s(alloy, stress, e + 0.5 * dt_s * k2);
        let k4 = rate_per_s(alloy, stress, e + dt_s * k3);
        e + dt_s / 6.0 * (k1 + 2.0 * k2 + 2.0 * k3 + 2.0 * k4)
    }

    struct Run {
        first: CreepState,
        last: CreepState,
        end: StepReject,
        steps: u32,
        worst_residual: f64,
        level_time_h: [f64; 6],
        frame: LastStateFrame64,
    }

    /// Step at `dt_s` under a held stress from 0.1 percent until the card
    /// refuses. The time since load to each level is read as the protocol
    /// reads it: linear interpolation between the two landed samples that
    /// bracket it, and for the limit no sample passed, the last sample plus
    /// the remaining extension over its rate. The 0.1 percent level is the
    /// starting clock itself.
    fn run_to_refusal(
        stress: f64,
        dt_s: f64,
        integrate: fn(&CreepCmsx6, f64, f64, f64) -> f64,
    ) -> Run {
        let (alloy, mut state) = start(stress, CREEP_EXTENSION_MIN_PERCENT);
        let first = state;
        let limit = alloy.creep_extension_max_percent(stress);
        let mut frame = blank_frame();
        let mut steps = 0;
        let mut worst_residual = 0.0_f64;
        let mut level_time_h = [f64::NAN; 6];
        level_time_h[0] = state.t_s / 3600.0;
        loop {
            let before = state;
            match step_with(&alloy, &mut state, &mut frame, dt_s, integrate) {
                Ok(()) => {
                    steps += 1;
                    if state.residual_conservation_error > worst_residual {
                        worst_residual = state.residual_conservation_error;
                    }
                    let mut index = 1;
                    while index < 6 {
                        let level = LEVEL_PERCENT[index];
                        if before.creep_extension_percent < level
                            && state.creep_extension_percent >= level
                        {
                            let fraction = (level - before.creep_extension_percent)
                                / (state.creep_extension_percent - before.creep_extension_percent);
                            level_time_h[index] = (before.t_s + fraction * dt_s) / 3600.0;
                        }
                        index += 1;
                    }
                }
                Err(end) => {
                    if end == StepReject::CreepExtensionBeyondCard {
                        let index = if limit == 5.0 { 5 } else { 1 };
                        level_time_h[index] = (state.t_s
                            + (limit - state.creep_extension_percent)
                                / (state.creep_rate_percent_per_h / 3600.0))
                            / 3600.0;
                    }
                    return Run {
                        first,
                        last: state,
                        end,
                        steps,
                        worst_residual,
                        level_time_h,
                        frame,
                    };
                }
            }
        }
    }

    #[test]
    fn constants_are_locked() {
        assert_eq!(TEMPERATURE_C.to_bits(), 980.0_f64.to_bits());
        assert_eq!(REFERENCE_STRESS_MPA.to_bits(), 185.0_f64.to_bits());
        assert_eq!(BASE_RATE_PERCENT_PER_H.to_bits(), 7.10621e-07_f64.to_bits());
        assert_eq!(BASE_STRESS_EXPONENT.to_bits(), 33.4123_f64.to_bits());
        assert_eq!(SOFTENING_PER_PERCENT.to_bits(), 18552.9_f64.to_bits());
        assert_eq!(SOFTENING_STRESS_EXPONENT.to_bits(), (-27.1579_f64).to_bits());
        assert_eq!(PRIMARY_MULTIPLIER.to_bits(), 181.475_f64.to_bits());
        assert_eq!(PRIMARY_DECAY_EXTENSION_PERCENT.to_bits(), 0.037091_f64.to_bits());
        assert_eq!(STRESS_MIN_MPA.to_bits(), 140.0_f64.to_bits());
        assert_eq!(STRESS_MAX_MPA.to_bits(), 230.0_f64.to_bits());
        assert_eq!(CREEP_EXTENSION_MIN_PERCENT.to_bits(), 0.1_f64.to_bits());
        assert_eq!(FULL_REACH_STRESS_MIN_MPA.to_bits(), 170.0_f64.to_bits());
        assert_eq!(CREEP_EXTENSION_MAX_PERCENT.to_bits(), 5.0_f64.to_bits());
        assert_eq!(CREEP_EXTENSION_MAX_BELOW_FULL_REACH_PERCENT.to_bits(), 0.2_f64.to_bits());
        assert_eq!(RESIDUAL_CONSERVATION_LIMIT.to_bits(), 2.0e-9_f64.to_bits());
        assert_eq!(GATE_PHASE_STEP.to_bits(), 0.2_f64.to_bits());
        assert_eq!(FLOOR_DT_S.to_bits(), 1.0_f64.to_bits());
        assert_eq!(START_CLOCK_PANELS, 128);
        assert_eq!(FLAG_CREEP_RATE_RISING, 1);

        let alloy = CreepCmsx6::at_980_c();
        assert!(alloy.is_valid());
        // Every recorded copy, changed by one bit, is invalid.
        let bump = |value: &mut f64| *value = f64::from_bits(value.to_bits() + 1);
        let mut tampered = std::vec::Vec::new();
        macro_rules! one_bit {
            ($field:ident) => {{
                let mut one = alloy;
                bump(&mut one.$field);
                tampered.push(one);
            }};
        }
        one_bit!(temperature_c);
        one_bit!(reference_stress_mpa);
        one_bit!(base_rate_percent_per_h);
        one_bit!(base_stress_exponent);
        one_bit!(softening_per_percent);
        one_bit!(softening_stress_exponent);
        one_bit!(primary_multiplier);
        one_bit!(primary_decay_extension_percent);
        one_bit!(stress_min_mpa);
        one_bit!(stress_max_mpa);
        one_bit!(creep_extension_min_percent);
        one_bit!(full_reach_stress_min_mpa);
        one_bit!(creep_extension_max_percent);
        one_bit!(creep_extension_max_below_full_reach_percent);
        one_bit!(residual_conservation_limit);
        one_bit!(gate_phase_step);
        one_bit!(floor_dt_s);
        assert_eq!(tampered.len(), 17);
        let (_, good) = start(170.0, 0.3);
        for card in tampered.iter() {
            assert!(!card.is_valid());
            // A step on a card that is not this alloy's is refused by name.
            let mut state = good;
            let mut frame = blank_frame();
            assert_eq!(
                step(card, &mut state, &mut frame, load(170.0), 10.0),
                Err(StepReject::InvalidCard)
            );
            assert_eq!(state_bits(&state), state_bits(&good));
            frame_untouched(&frame);
            assert!(CreepState::try_new(card, 170.0, 0.3).is_none());
        }
        let mut nan = alloy;
        nan.primary_multiplier = f64::NAN;
        assert!(!nan.is_valid());
    }

    #[test]
    fn card_state_and_load_are_copy() {
        assert_copy::<CreepCmsx6>();
        assert_copy::<CreepState>();
        assert_copy::<CreepLoad>();
        assert_copy::<StepReject>();
    }

    #[test]
    fn the_upper_limit_is_five_percent_at_every_stress_and_the_flag_marks_where_the_law_missed() {
        let alloy = CreepCmsx6::at_980_c();
        for stress in [140.0, 155.0, 169.99, one_under(170.0), 170.0, one_over(170.0), 200.0, 230.0] {
            assert_eq!(alloy.creep_extension_max_percent(stress).to_bits(), 5.0_f64.to_bits(), "{stress}");
        }
        // The region where the law was compared and missed: below 0.1 percent
        // at any stress, past 0.2 percent below 170 MPa.
        for (stress, from, to, outside) in [
            (230.0, 0.0, 0.05, true),
            (230.0, one_under(0.1), 0.11, true),
            (230.0, 0.1, 0.2, false),
            (230.0, 0.3, 5.0, false),
            (170.0, 0.1, 5.0, false),
            (one_under(170.0), 0.15, 0.2, false),
            (one_under(170.0), 0.15, one_over(0.2), true),
            (140.0, 0.1, 0.2, false),
            (140.0, 0.25, 0.8, true),
            (140.0, 2.0, 5.0, true),
        ] {
            assert_eq!(alloy.outside_met_range(stress, from, to), outside, "{stress} {from} {to}");
        }
    }

    #[test]
    fn the_start_takes_a_stress_and_an_extension_inside_the_card_and_nothing_else() {
        let alloy = CreepCmsx6::at_980_c();
        for (stress, inside) in [
            (140.0, 0.1),
            (140.0, 0.15),
            (140.0, 0.2),
            (one_under(170.0), 0.2),
            (170.0, 0.1),
            (170.0, 0.2),
            (170.0, one_over(0.2)),
            (170.0, 5.0),
            (200.0, 2.5),
            (230.0, 0.1),
            (230.0, 5.0),
            (140.0, 0.0),
            (140.0, 0.05),
            (140.0, 0.5),
            (140.0, 5.0),
            (one_under(170.0), 0.25),
            (169.999, 5.0),
            (170.0, 0.0),
            (230.0, 1.0e-9),
            (230.0, one_under(0.1)),
        ] {
            let state = CreepState::try_new(&alloy, stress, inside).unwrap();
            assert_eq!(state.creep_extension_percent.to_bits(), inside.to_bits());
            assert_eq!(state.stress_mpa.to_bits(), stress.to_bits());
            // The clock starts at the law's own time to the start extension.
            assert_eq!(
                state.t_s.to_bits(),
                alloy.law_time_since_load_s(stress, inside).to_bits()
            );
            assert!(if inside == 0.0 { state.t_s == 0.0 } else { state.t_s > 0.0 });
            assert_eq!(
                state.creep_rate_percent_per_h.to_bits(),
                alloy.creep_rate_percent_per_h(stress, inside).to_bits()
            );
            assert_eq!(state.work_by_load_mj_per_m3, 0.0);
            assert_eq!(state.heat_dissipated_mj_per_m3, 0.0);
            assert_eq!(state.work_defect_mj_per_m3, 0.0);
            assert_eq!(state.residual_conservation_error, 0.0);
            assert!(!state.creep_rate_rising);
        }
        // Below zero at every stress: no start.
        for stress in [140.0, 170.0, 200.0, 230.0] {
            for below in [-1.0e-300, -1.0e-12, -0.1] {
                assert!(CreepState::try_new(&alloy, stress, below).is_none(), "{stress} {below}");
            }
        }
        // Above 5 percent.
        for (stress, above) in [
            (140.0, one_over(5.0)),
            (one_under(170.0), 5.000_000_1),
            (170.0, one_over(5.0)),
            (230.0, 5.000_000_1),
        ] {
            assert!(CreepState::try_new(&alloy, stress, above).is_none(), "{stress} {above}");
        }
        // A stress off the card, and numbers that are not finite.
        for stress in [one_under(140.0), one_over(230.0), 0.0, -170.0, f64::NAN, f64::INFINITY] {
            assert!(CreepState::try_new(&alloy, stress, 0.15).is_none(), "{stress}");
        }
        for extension in [f64::NAN, f64::INFINITY, -f64::INFINITY] {
            assert!(CreepState::try_new(&alloy, 170.0, extension).is_none());
        }
    }

    #[test]
    fn the_rate_is_the_law_of_the_header_written_out() {
        let alloy = CreepCmsx6::at_980_c();
        // 185 MPa: x = 1, so the rate is A (1 + C0 e) (1 + K exp(−e/e_p)) with no power.
        let e = 0.5_f64;
        let expected = 7.10621e-07
            * (1.0 + 18552.9 * e)
            * (1.0 + 181.475 * crate::math::exp(-e / 0.037091));
        let got = alloy.creep_rate_percent_per_h(185.0, e);
        assert!((got - expected).abs() <= 4.0 * f64::EPSILON * expected, "{got} {expected}");
        // At zero extension the rate is A x^n (1 + K).
        let x = 230.0_f64 / 185.0;
        let expected = 7.10621e-07 * std::primitive::f64::powf(x, 33.4123) * 182.475;
        let got = alloy.creep_rate_percent_per_h(230.0, 0.0);
        assert!((got - expected).abs() <= 1.0e-12 * expected, "{got} {expected}");
        // The rate rises with stress at every extension of the card, and is positive.
        let mut e = 0.1;
        while e <= 5.0 {
            let mut previous = 0.0;
            let mut stress = 140.0;
            while stress <= 230.0 {
                let rate = alloy.creep_rate_percent_per_h(stress, e);
                assert!(rate > previous && rate.is_finite(), "{stress} {e}");
                previous = rate;
                stress += 5.0;
            }
            e += 0.05;
        }
    }

    #[test]
    fn the_starting_clock_is_the_laws_time_from_zero_extension() {
        let alloy = CreepCmsx6::at_980_c();
        // Against integration at 50 digits.
        let mut worst = 0.0_f64;
        for (stress, extension, seconds) in START_CLOCK_S {
            let got = alloy.law_time_since_load_s(stress, extension);
            let off = (got - seconds).abs() / seconds;
            if off > worst {
                worst = off;
            }
            assert!(off <= 1.0e-13, "{stress} MPa {extension} percent: {got} against {seconds}");
        }
        println!("starting clock against 50-digit integration: largest relative difference {worst:e}");
        // Against the forecast's own table, printed to a thousandth of an hour.
        for (stress, forecast) in FORECAST_H {
            let limit = alloy.creep_extension_max_percent(stress);
            let mut index = 0;
            while index < 6 {
                if LEVEL_PERCENT[index] <= limit {
                    let got = alloy.law_time_since_load_s(stress, LEVEL_PERCENT[index]) / 3600.0;
                    assert!(
                        (got - forecast[index]).abs() <= 2.0e-5 * forecast[index] + 6.0e-4,
                        "{stress} MPa level {}: {got} against {}",
                        LEVEL_PERCENT[index],
                        forecast[index]
                    );
                }
                index += 1;
            }
        }
        // An independent sum in the extension itself: 3600 de / rate by the midpoint
        // rule on 200 panels in each of 60 bands, the bands spaced by a factor of
        // two in the extension down from the start.
        for (stress, extension) in [(140.0, 0.1), (170.0, 0.33), (230.0, 5.0)] {
            let mut sum = 0.0;
            let mut top = extension;
            let mut band = 0;
            while band < 60 {
                let bottom = if band == 59 { 0.0 } else { 0.5 * top };
                let width = (top - bottom) / 200.0;
                let mut index = 0;
                while index < 200 {
                    sum += width / rate_per_s(&alloy, stress, bottom + (index as f64 + 0.5) * width);
                    index += 1;
                }
                top = bottom;
                band += 1;
            }
            let got = alloy.law_time_since_load_s(stress, extension);
            assert!((got - sum).abs() <= 2.0e-6 * sum, "{stress} {extension}: {got} against {sum}");
        }
        // The clock grows with the extension and falls with the stress.
        assert!(alloy.law_time_since_load_s(170.0, 0.2) > alloy.law_time_since_load_s(170.0, 0.1));
        assert!(alloy.law_time_since_load_s(200.0, 0.2) < alloy.law_time_since_load_s(170.0, 0.2));
    }

    #[test]
    fn the_ceiling_and_the_floor_are_from_the_constants() {
        let alloy = CreepCmsx6::at_980_c();
        let gate = alloy.gate_dt_s().unwrap();
        // analysis/out/05_step_size.log: "gate s 49.97408036862698".
        assert!((gate - 49.974_080_368_626_98).abs() < 1.0e-9, "{gate}");
        assert_eq!(alloy.floor_dt_s().unwrap().to_bits(), FLOOR_DT_S.to_bits());
        // The bound of the ceiling holds: |d rate / d e| / 3600 is under 0.2 / gate
        // at every stress and extension from zero to 5 percent, the span the bound
        // was taken over, and so on the card, which starts at 0.1 percent.
        let bound = GATE_PHASE_STEP / gate;
        let mut worst = 0.0_f64;
        let mut worst_on_card = 0.0_f64;
        let mut stress = 140.0;
        while stress <= 230.0 {
            let mut e = 0.0;
            while e < 5.0 {
                let h = 1.0e-7;
                let slope = (rate_per_s(&alloy, stress, e + h) - rate_per_s(&alloy, stress, e)) / h;
                if slope.abs() > worst {
                    worst = slope.abs();
                }
                if e >= 0.1 && e <= alloy.creep_extension_max_percent(stress) && slope.abs() > worst_on_card {
                    worst_on_card = slope.abs();
                }
                e += 0.001;
            }
            stress += 2.5;
        }
        println!("largest slope of the rate, per s: {worst:e} from zero, {worst_on_card:e} on the card, bound {bound:e}");
        assert!(worst <= bound, "{worst} {bound}");
        assert!(worst > 0.3 * bound, "{worst} {bound}");
        assert!(worst_on_card <= worst);
        // A step at the floor lands from both ends of the extension span, at both ends of the stress span.
        for (stress, e) in [(140.0, 0.1), (140.0, 0.19), (230.0, 0.1), (230.0, 4.9)] {
            let (alloy, mut state) = start(stress, e);
            let mut frame = blank_frame();
            assert_eq!(step(&alloy, &mut state, &mut frame, load(stress), FLOOR_DT_S), Ok(()));
            assert!(state.creep_extension_percent > e);
            assert!(state.residual_conservation_error < 1.0e-8, "{}", state.residual_conservation_error);
        }
    }

    #[test]
    fn a_held_stress_reaches_each_level_at_the_forecast_time_since_load() {
        let mut worst_residual = 0.0_f64;
        for (stress, forecast) in FORECAST_H {
            let run = run_to_refusal(stress, ceiling(), rk4);
            println!(
                "{stress} MPa: starts at {:.3} h, {} steps, ends {:?} at {:.3} h, {:.4} percent, worst residual {:.3e}, levels {:?}",
                run.first.t_s / 3600.0,
                run.steps,
                run.end,
                run.last.t_s / 3600.0,
                run.last.creep_extension_percent,
                run.worst_residual,
                run.level_time_h
            );
            if run.worst_residual > worst_residual {
                worst_residual = run.worst_residual;
            }
            assert!(run.worst_residual <= RESIDUAL_CONSERVATION_LIMIT / 10.0);
            // Every run ends at 5 percent, the limit of extension at every
            // stress; through engine 0.9 the 140 MPa run ended at 0.2.
            assert_eq!(run.end, StepReject::CreepExtensionBeyondCard);
            let limit = 5.0;
            assert!(run.last.creep_extension_percent <= limit);
            assert!(run.last.creep_extension_percent > 0.998 * limit);
            let mut index = 0;
            while index < 6 {
                let expected = forecast[index];
                let got = run.level_time_h[index];
                if LEVEL_PERCENT[index] > limit {
                    // The card refused before this level.
                    assert!(got.is_nan());
                } else {
                    // The forecast is printed to a thousandth of an hour.
                    assert!(
                        (got - expected).abs() <= 1.0e-4 * expected + 6.0e-4,
                        "{stress} MPa level {}: {got} against {expected}",
                        LEVEL_PERCENT[index]
                    );
                }
                index += 1;
            }
            proof_intact(&run.frame);
        }
        println!("largest ledger residual over the four runs: {worst_residual:e}");
    }

    #[test]
    fn a_run_made_at_a_landed_extension_goes_on_through_the_same_extensions_bit_for_bit() {
        // The extension is the only memory of the test piece. A state made at an
        // extension a run landed on steps through the same 64-bit extensions and
        // rates as that run, and its clock, the law's own time to that extension,
        // agrees with the run's stepped clock to the error of the stepping.
        for stress in [150.0, 170.0, 230.0] {
            let (alloy, mut first) = start(stress, 0.1);
            let mut frame = blank_frame();
            let mut steps = 0;
            while steps < 200 {
                assert_eq!(step(&alloy, &mut first, &mut frame, load(stress), ceiling()), Ok(()));
                steps += 1;
            }
            let mut second =
                CreepState::try_new(&alloy, stress, first.creep_extension_percent).unwrap();
            assert!(((second.t_s - first.t_s) / first.t_s).abs() < 1.0e-9, "{stress}");
            assert_eq!(second.creep_rate_percent_per_h.to_bits(), first.creep_rate_percent_per_h.to_bits());
            let mut second_frame = blank_frame();
            let mut more = 0;
            while more < 100 {
                assert_eq!(step(&alloy, &mut first, &mut frame, load(stress), ceiling()), Ok(()));
                assert_eq!(step(&alloy, &mut second, &mut second_frame, load(stress), ceiling()), Ok(()));
                assert_eq!(
                    second.creep_extension_percent.to_bits(),
                    first.creep_extension_percent.to_bits()
                );
                assert_eq!(
                    second.creep_rate_percent_per_h.to_bits(),
                    first.creep_rate_percent_per_h.to_bits()
                );
                // The extension, the rate and the stress fill the same frame slots with the same bytes.
                assert_eq!(second_frame.pos, frame.pos);
                assert_eq!(second_frame.force_torque, frame.force_torque);
                assert_eq!(second_frame.flags, frame.flags);
                more += 1;
            }
        }
    }

    #[test]
    fn a_step_writes_the_frame_and_the_books_and_keeps_the_proof() {
        let (alloy, mut state) = start(200.0, 0.1);
        let started_at = state.t_s;
        let mut frame = blank_frame();
        assert_eq!(step(&alloy, &mut state, &mut frame, load(200.0), 40.0), Ok(()));
        assert_eq!(state.t_s.to_bits(), (started_at + 40.0).to_bits());
        assert!(state.creep_extension_percent > 0.1);
        assert_eq!(state.stress_mpa, 200.0);
        assert_eq!(
            state.work_by_load_mj_per_m3.to_bits(),
            (200.0 * (state.creep_extension_percent - 0.1) / 100.0).to_bits()
        );
        assert_eq!(
            state.creep_rate_percent_per_h.to_bits(),
            alloy.creep_rate_percent_per_h(200.0, state.creep_extension_percent).to_bits()
        );
        assert!(state.heat_dissipated_mj_per_m3 > 0.0);
        assert_eq!(
            state.work_defect_mj_per_m3.to_bits(),
            (state.work_by_load_mj_per_m3 - state.heat_dissipated_mj_per_m3).to_bits()
        );
        assert_eq!(frame.t, started_at + 40.0);
        assert_eq!(frame.pos[0], state.creep_extension_percent as f32);
        assert_eq!(frame.pos[1], state.creep_rate_percent_per_h as f32);
        assert_eq!(frame.pos[2], 200.0);
        assert_eq!(frame.vel[0], state.work_by_load_mj_per_m3 as f32);
        assert_eq!(frame.vel[1], state.heat_dissipated_mj_per_m3 as f32);
        assert_eq!(frame.vel[2], state.work_defect_mj_per_m3 as f32);
        assert_eq!(frame.force_torque, 200.0);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        // At 0.1 percent the law's rate is still falling.
        assert!(!state.creep_rate_rising);
        assert_eq!(frame.flags, 0);
        proof_intact(&frame);
    }

    #[test]
    fn the_rising_flag_is_the_rate_rising_over_the_step_and_no_undeclared_bit_is_set() {
        // From 170 MPa up the law's rate falls from 0.1 percent to its minimum and
        // rises from there to 5 percent: the flag is clear, then set, and stays set.
        for stress in [170.0, 200.0, 230.0] {
            let (alloy, mut state) = start(stress, 0.1);
            let mut frame = blank_frame();
            let mut falling = 0_u32;
            let mut rising = 0_u32;
            let mut minimum_at = f64::NAN;
            loop {
                let before = alloy.creep_rate_percent_per_h(stress, state.creep_extension_percent);
                if step(&alloy, &mut state, &mut frame, load(stress), ceiling()).is_err() {
                    break;
                }
                let is_rising = state.creep_rate_percent_per_h > before;
                assert_eq!(state.creep_rate_rising, is_rising);
                assert_eq!(frame.flags & FLAG_CREEP_RATE_RISING != 0, is_rising);
                // Rule 7: the one declared bit and no other.
                assert_eq!(frame.flags & !FLAG_CREEP_RATE_RISING, 0);
                if is_rising {
                    if rising == 0 {
                        minimum_at = state.creep_extension_percent;
                    }
                    rising += 1;
                } else {
                    assert_eq!(rising, 0, "the rate fell again at {stress} MPa");
                    falling += 1;
                }
            }
            println!("{stress} MPa: rate falling over {falling} steps, rising over {rising}, from {minimum_at:.4} percent");
            assert!(falling > 0 && rising > 0);
            assert!(minimum_at > 0.2 && minimum_at < 0.3, "{minimum_at}");
        }
        // Below 170 MPa the run goes on past 0.2 percent, through the law's
        // minimum, to 5 percent: the rising flag is set past the minimum, and
        // every frame past 0.2 percent carries FLAG_OUTSIDE_MET_RANGE.
        for stress in [140.0, 155.0, 169.9] {
            let (alloy, mut state) = start(stress, 0.1);
            let mut frame = blank_frame();
            let mut steps = 0_u32;
            let mut rising = 0_u32;
            let mut outside = 0_u32;
            while step(&alloy, &mut state, &mut frame, load(stress), ceiling()).is_ok() {
                assert_eq!(
                    frame.flags & FLAG_OUTSIDE_MET_RANGE != 0,
                    state.creep_extension_percent > 0.2
                );
                assert_eq!(frame.flags & !(FLAG_CREEP_RATE_RISING | FLAG_OUTSIDE_MET_RANGE), 0);
                if state.creep_rate_rising {
                    rising += 1;
                }
                if frame.flags & FLAG_OUTSIDE_MET_RANGE != 0 {
                    outside += 1;
                }
                steps += 1;
            }
            assert!(state.creep_extension_percent > 4.99);
            assert!(rising > 0 && outside > 0 && steps > outside, "{stress}");
        }
    }

    #[test]
    fn each_named_limit_refuses_and_leaves_the_state_and_the_frame_as_they_were() {
        let alloy = CreepCmsx6::at_980_c();
        let fresh = CreepState::try_new(&alloy, 170.0, 0.3).unwrap();
        let try_step = |state: CreepState, stress: f64, dt_s: f64, expected: StepReject| {
            let mut stepped = state;
            let mut frame = blank_frame();
            assert_eq!(
                step(&alloy, &mut stepped, &mut frame, load(stress), dt_s),
                Err(expected),
                "{stress} {dt_s}"
            );
            if state_bits(&state).iter().all(|bits| f64::from_bits(*bits).is_finite()) {
                assert_eq!(state_bits(&stepped), state_bits(&state));
            }
            frame_untouched(&frame);
            proof_intact(&frame);
        };
        // Inputs that are not finite.
        for dt in [f64::NAN, f64::INFINITY, -f64::INFINITY] {
            try_step(fresh, 170.0, dt, StepReject::NonFiniteInput);
        }
        for stress in [f64::NAN, f64::INFINITY, -f64::INFINITY] {
            try_step(fresh, stress, 10.0, StepReject::NonFiniteInput);
        }
        // A state that is not finite.
        let mut broken = fresh;
        broken.creep_extension_percent = f64::NAN;
        try_step(broken, 170.0, 10.0, StepReject::NonFiniteState);
        let mut broken = fresh;
        broken.work_by_load_mj_per_m3 = f64::INFINITY;
        try_step(broken, 170.0, 10.0, StepReject::NonFiniteState);
        let mut broken = fresh;
        broken.t_s = f64::NAN;
        try_step(broken, 170.0, 10.0, StepReject::NonFiniteState);
        // dt zero or negative.
        try_step(fresh, 170.0, 0.0, StepReject::NonPositiveDt);
        try_step(fresh, 170.0, -1.0, StepReject::NonPositiveDt);
        // Stress one spacing outside each end; the ends themselves land.
        try_step(fresh, one_under(STRESS_MIN_MPA), 10.0, StepReject::StressOutsideCard);
        try_step(fresh, one_over(STRESS_MAX_MPA), 10.0, StepReject::StressOutsideCard);
        try_step(fresh, 0.0, 10.0, StepReject::StressOutsideCard);
        try_step(fresh, -170.0, 10.0, StepReject::StressOutsideCard);
        for edge in [STRESS_MIN_MPA, STRESS_MAX_MPA] {
            let (alloy, mut state) = start(edge, 0.15);
            let mut frame = blank_frame();
            assert_eq!(step(&alloy, &mut state, &mut frame, load(edge), 10.0), Ok(()));
        }
        // A stress on the card that is not the stress the state was made at.
        for other in [one_under(170.0), one_over(170.0), 140.0, 200.0, 230.0] {
            try_step(fresh, other, 10.0, StepReject::StressNotHeld);
        }
        // An incoming extension below zero.
        for below in [-1.0e-300, -1.0e-9, -0.1] {
            for stress in [140.0, 170.0, 230.0] {
                let mut state = CreepState::try_new(&alloy, stress, 0.15).unwrap();
                state.creep_extension_percent = below;
                try_step(state, stress, 10.0, StepReject::CreepExtensionBelowZero);
            }
        }
        // Under 0.1 percent, zero among them, a step lands with the flag set:
        // through engine 0.9 each was refused as CreepExtensionBelowCard.
        for below in [one_under(0.1), 0.05, 1.0e-9, 0.0] {
            for stress in [140.0, 170.0, 230.0] {
                let (alloy, mut state) = start(stress, below);
                let mut frame = blank_frame();
                assert_eq!(step(&alloy, &mut state, &mut frame, load(stress), 5.0), Ok(()), "{stress} {below}");
                assert_eq!(frame.flags & FLAG_OUTSIDE_MET_RANGE, FLAG_OUTSIDE_MET_RANGE);
            }
        }
        // 0.1 percent itself lands, at each tested stress.
        for stress in [140.0, 170.0, 200.0, 230.0] {
            let (alloy, mut state) = start(stress, 0.1);
            let mut frame = blank_frame();
            assert_eq!(step(&alloy, &mut state, &mut frame, load(stress), 10.0), Ok(()));
        }
        // An incoming extension above 5 percent.
        for (stress, above) in [
            (140.0, one_over(5.0)),
            (one_under(170.0), 5.000_001),
            (170.0, one_over(5.0)),
            (230.0, 5.000_001),
        ] {
            let mut state = CreepState::try_new(&alloy, stress, 0.15).unwrap();
            state.creep_extension_percent = above;
            try_step(state, stress, 10.0, StepReject::CreepExtensionOutsideCard);
        }
        // A step that would pass 5 percent, at every stress.
        for stress in [140.0, one_under(170.0), 230.0] {
            let near = CreepState::try_new(&alloy, stress, 4.999_99).unwrap();
            try_step(near, stress, ceiling(), StepReject::CreepExtensionBeyondCard);
        }
        // Past 0.2 percent below 170 MPa a step lands, flagged; at 170 MPa
        // itself it lands with no flag.
        for (stress, flagged) in [(140.0, true), (one_under(170.0), true), (170.0, false)] {
            let (alloy, mut state) = start(stress, 0.199_999);
            let mut frame = blank_frame();
            assert_eq!(step(&alloy, &mut state, &mut frame, load(stress), 40.0), Ok(()));
            assert!(state.creep_extension_percent > 0.2);
            assert_eq!(frame.flags & FLAG_OUTSIDE_MET_RANGE != 0, flagged, "{stress}");
        }
        // A step too small to move the extension.
        let high = CreepState::try_new(&alloy, 170.0, 4.0).unwrap();
        try_step(high, 170.0, 1.0e-12, StepReject::StepTooSmallToResolve);
    }

    #[test]
    fn nothing_is_pulled_back_at_the_edge_of_the_extension() {
        // The last landed sample of a run is under the limit and the refused step would have passed it.
        for (stress, limit) in [(200.0, 5.0), (140.0, 5.0), (169.0, 5.0)] {
            let run = run_to_refusal(stress, ceiling(), rk4);
            assert_eq!(run.end, StepReject::CreepExtensionBeyondCard);
            let alloy = CreepCmsx6::at_980_c();
            let would_be = rk4(&alloy, stress, run.last.creep_extension_percent, ceiling());
            assert!(would_be > limit);
            assert!(run.last.creep_extension_percent < limit);
            // A smaller step from the same state still lands.
            let mut state = run.last;
            let mut frame = blank_frame();
            assert_eq!(step(&alloy, &mut state, &mut frame, load(stress), 1.0), Ok(()));
            assert!(state.creep_extension_percent < limit);
        }
    }

    #[test]
    fn a_step_that_lands_exactly_on_the_limit_lands_and_one_spacing_past_it_does_not() {
        // The real validate and commit, with the landing extension chosen, and the
        // step the law takes to reach it, so the ledger balances.
        for (stress, limit) in [(140.0, 5.0), (200.0, 5.0)] {
            for (landing, lands) in [(one_under(limit), true), (limit, true), (one_over(limit), false)] {
                let (alloy, mut state) = start(stress, 0.999 * limit);
                let held = state;
                let mut frame = blank_frame();
                let dt_s = law_time_s(&alloy, stress, state.creep_extension_percent, landing);
                let end = step_with(&alloy, &mut state, &mut frame, dt_s, |_, _, _, _| landing);
                if lands {
                    assert_eq!(end, Ok(()), "{stress} {landing}");
                    assert_eq!(state.creep_extension_percent.to_bits(), landing.to_bits());
                } else {
                    assert_eq!(end, Err(StepReject::CreepExtensionBeyondCard));
                    assert_eq!(state_bits(&state), state_bits(&held));
                    frame_untouched(&frame);
                }
            }
            // From the limit itself the next step is refused, whatever its size.
            let (alloy, state) = start(stress, limit);
            for dt_s in [1.0, ceiling()] {
                let mut stepped = state;
                let mut frame = blank_frame();
                assert_eq!(
                    step(&alloy, &mut stepped, &mut frame, load(stress), dt_s),
                    Err(StepReject::CreepExtensionBeyondCard)
                );
                assert_eq!(state_bits(&stepped), state_bits(&state));
                frame_untouched(&frame);
            }
        }
    }

    #[test]
    fn the_honest_step_is_inside_a_twelfth_of_the_ledgers_limit_everywhere_on_the_card() {
        // One step from a fresh state: its residual is the step's own. Every
        // stress in steps of 1.25 MPa, every extension in steps of one percent
        // of itself, seven step sizes from the floor to the ceiling.
        let alloy = CreepCmsx6::at_980_c();
        let mut worst = 0.0_f64;
        let mut worst_at = (0.0, 0.0, 0.0);
        for dt_s in [FLOOR_DT_S, 1.5, 3.0, 7.0, 20.0, 33.0, ceiling()] {
            let mut stress = 140.0;
            while stress <= 230.0 {
                let limit = alloy.creep_extension_max_percent(stress);
                let mut e = 0.1;
                while e < 0.999 * limit {
                    let mut state = CreepState::try_new(&alloy, stress, e).unwrap();
                    let mut frame = blank_frame();
                    assert_eq!(step(&alloy, &mut state, &mut frame, load(stress), dt_s), Ok(()));
                    if state.residual_conservation_error > worst {
                        worst = state.residual_conservation_error;
                        worst_at = (stress, e, dt_s);
                    }
                    e *= 1.01;
                }
                stress += 1.25;
            }
        }
        println!("largest residual of one honest step: {worst:e} at {worst_at:?}");
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT / 11.0, "{worst:e}");
        assert!(worst >= RESIDUAL_CONSERVATION_LIMIT / 13.0, "{worst:e}");
    }

    /// The wall at 0.1 percent lifted: from zero extension, the moment the
    /// full load is applied, a run at 5 s reaches 5 percent at every stress
    /// with the ledger closed, through the region where the law missed the
    /// measured curves, flagged there. It reaches 0.1 percent at the hour the
    /// starting clock gives. A step of the ceiling from zero leaves more than
    /// the ledger's limit at 200 and 230 MPa and is refused by name, with the
    /// state kept; at 140 and 170 MPa it lands. Explicit Euler from zero is
    /// named.
    #[test]
    fn from_zero_extension_the_law_runs_to_five_percent_and_a_coarse_step_is_named() {
        let alloy = CreepCmsx6::at_980_c();
        for (stress, forecast) in FORECAST_H {
            let (alloy, mut state) = start(stress, 0.0);
            assert_eq!(state.t_s, 0.0);
            let mut frame = blank_frame();
            let mut worst = 0.0_f64;
            let mut at_tenth_h = f64::NAN;
            let mut steps = 0_u64;
            let end = loop {
                let before = state;
                match step(&alloy, &mut state, &mut frame, load(stress), 5.0) {
                    Ok(()) => {
                        steps += 1;
                        worst = worst.max(state.residual_conservation_error);
                        assert_eq!(
                            frame.flags & FLAG_OUTSIDE_MET_RANGE != 0,
                            before.creep_extension_percent < 0.1
                                || (stress < 170.0 && state.creep_extension_percent > 0.2)
                        );
                        if before.creep_extension_percent < 0.1 && state.creep_extension_percent >= 0.1 {
                            let fraction = (0.1 - before.creep_extension_percent)
                                / (state.creep_extension_percent - before.creep_extension_percent);
                            at_tenth_h = (before.t_s + fraction * 5.0) / 3600.0;
                        }
                    }
                    Err(end) => break end,
                }
            };
            println!(
                "{stress} MPa from zero at 5 s: {steps} steps, {end:?} at {:.4} percent, {:.3} h; 0.1 percent at {at_tenth_h:.4} h; worst residual {worst:e}",
                state.creep_extension_percent,
                state.t_s / 3600.0
            );
            assert_eq!(end, StepReject::CreepExtensionBeyondCard);
            assert!(state.creep_extension_percent > 4.999);
            assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT, "{stress}: {worst:e}");
            assert!((at_tenth_h - forecast[0]).abs() <= 1.0e-3 * forecast[0] + 6.0e-4, "{stress}");
            assert!((state.t_s / 3600.0 - forecast[5]).abs() <= 1.0e-3 * forecast[5], "{stress}");
        }
        for (stress, lands) in [(140.0, true), (170.0, true), (200.0, false), (230.0, false)] {
            let fresh = CreepState::try_new(&alloy, stress, 0.0).unwrap();
            let mut state = fresh;
            let mut frame = blank_frame();
            let end = step(&alloy, &mut state, &mut frame, load(stress), ceiling());
            if lands {
                assert_eq!(end, Ok(()), "{stress}");
            } else {
                assert!(
                    end == Err(StepReject::FirstLawBreach) || end == Err(StepReject::SpontaneousGeneration),
                    "{stress}: {end:?}"
                );
                assert_eq!(state_bits(&state), state_bits(&fresh));
                frame_untouched(&frame);
            }
        }
        for stress in [140.0, 230.0] {
            let fresh = CreepState::try_new(&alloy, stress, 0.0).unwrap();
            let mut state = fresh;
            let mut frame = blank_frame();
            let end = step_with(&alloy, &mut state, &mut frame, 5.0, euler);
            assert!(
                end == Err(StepReject::FirstLawBreach) || end == Err(StepReject::SpontaneousGeneration),
                "{stress}: {end:?}"
            );
            assert_eq!(state_bits(&state), state_bits(&fresh));
        }
    }

    #[test]
    fn a_wrong_stepping_method_is_named_by_the_ledger() {
        // Rule 10. Euler, the midpoint rule and a wrong stage weight, each with the
        // real validate and commit, at the ceiling: each is refused on its first
        // step from 0.1 percent at every tested stress.
        let methods: [(&str, fn(&CreepCmsx6, f64, f64, f64) -> f64); 3] = [
            ("euler", euler),
            ("midpoint", midpoint),
            ("wrong stage weight", wrong_stage_weight),
        ];
        for (name, method) in methods {
            for stress in [140.0, 170.0, 200.0, 230.0] {
                let run = run_to_refusal(stress, ceiling(), method);
                println!(
                    "{name} at {stress} MPa: ended {:?} after {} landed steps at {:.4} percent",
                    run.end, run.steps, run.last.creep_extension_percent
                );
                assert!(
                    run.end == StepReject::SpontaneousGeneration
                        || run.end == StepReject::FirstLawBreach,
                    "{name} at {stress} MPa ended {:?} after {} steps",
                    run.end,
                    run.steps
                );
                assert_eq!(run.steps, 0, "{name} {stress}");
                assert_eq!(state_bits(&run.last), state_bits(&run.first));
                frame_untouched(&run.frame);
            }
        }
        // Euler and the wrong weight from states further into the test, the law's
        // minimum rate near 0.26 percent among them: the first step is refused.
        for (name, method) in [methods[0], methods[2]] {
            for (stress, e) in [(140.0, 0.15), (170.0, 0.26), (170.0, 1.0), (230.0, 0.5), (230.0, 4.0)] {
                let (alloy, mut state) = start(stress, e);
                let held = state;
                let mut frame = blank_frame();
                let end = step_with(&alloy, &mut state, &mut frame, ceiling(), method);
                assert!(
                    end == Err(StepReject::SpontaneousGeneration)
                        || end == Err(StepReject::FirstLawBreach),
                    "{name} at {stress} MPa from {e} percent: {end:?}"
                );
                assert_eq!(state_bits(&state), state_bits(&held));
                frame_untouched(&frame);
            }
        }
        // The midpoint rule is second order. Where the law is fast against the
        // ceiling it is refused. Where the law is slow against it, near the
        // minimum rate and at 170 MPa past it, the rule's error is under the
        // ledger's limit and it is a correct step to that limit.
        for (stress, e) in [(140.0, 0.15), (170.0, 0.15), (230.0, 0.5), (230.0, 4.0)] {
            let (alloy, mut state) = start(stress, e);
            let held = state;
            let mut frame = blank_frame();
            let end = step_with(&alloy, &mut state, &mut frame, ceiling(), midpoint);
            assert!(
                end == Err(StepReject::SpontaneousGeneration)
                    || end == Err(StepReject::FirstLawBreach),
                "midpoint at {stress} MPa from {e} percent: {end:?}"
            );
            assert_eq!(state_bits(&state), state_bits(&held));
            frame_untouched(&frame);
        }
        let (alloy, mut state) = start(170.0, 1.0);
        let mut frame = blank_frame();
        assert_eq!(step_with(&alloy, &mut state, &mut frame, ceiling(), midpoint), Ok(()));
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        // Euler falls short of the law where the rate is rising: less work than the law dissipates.
        let (alloy, mut state) = start(230.0, 1.0);
        let mut frame = blank_frame();
        assert_eq!(
            step_with(&alloy, &mut state, &mut frame, ceiling(), euler),
            Err(StepReject::FirstLawBreach)
        );
        // And overshoots it where the rate is falling: more.
        let (alloy, mut state) = start(230.0, 0.1);
        assert_eq!(
            step_with(&alloy, &mut state, &mut frame, ceiling(), euler),
            Err(StepReject::SpontaneousGeneration)
        );
    }

    /// Rule 10 in the regions the card answers where the law missed the
    /// measured curves: below 0.1 percent and past 0.2 percent below
    /// 170 MPa. Euler, a wrong stage weight and an extension moved after the
    /// step are each refused by the ledger, by name, and nothing moves; the
    /// honest step from each state lands with FLAG_OUTSIDE_MET_RANGE.
    #[test]
    fn where_the_law_missed_a_wrong_step_and_a_moved_extension_are_named() {
        let states = [
            (140.0, 0.0, 5.0),
            (140.0, 0.05, 20.0),
            (230.0, 0.02, 5.0),
            (140.0, 0.5, 40.0),
            (140.0, 1.0, 40.0),
            (140.0, 3.0, 40.0),
            (155.0, 2.0, 40.0),
            (169.9, 4.5, 40.0),
        ];
        for (stress, e, dt_s) in states {
            for (name, method) in [
                ("euler", euler as fn(&CreepCmsx6, f64, f64, f64) -> f64),
                ("wrong stage weight", wrong_stage_weight),
            ] {
                let (alloy, mut state) = start(stress, e);
                let held = state;
                let mut frame = blank_frame();
                let end = step_with(&alloy, &mut state, &mut frame, ceiling(), method);
                assert!(
                    end == Err(StepReject::SpontaneousGeneration) || end == Err(StepReject::FirstLawBreach),
                    "{name} at {stress} MPa from {e} percent: {end:?}"
                );
                assert_eq!(state_bits(&state), state_bits(&held));
                frame_untouched(&frame);
            }
            for (factor, expected) in [
                (1.0 + 1.0e-4, StepReject::SpontaneousGeneration),
                (1.0 - 1.0e-4, StepReject::FirstLawBreach),
            ] {
                let (alloy, mut state) = start(stress, e);
                let held = state;
                let mut frame = blank_frame();
                let end = step_with(&alloy, &mut state, &mut frame, dt_s, |a, s, e, dt| {
                    let honest = rk4(a, s, e, dt);
                    e + (honest - e) * factor
                });
                assert_eq!(end, Err(expected), "{stress} MPa from {e} percent");
                assert_eq!(state_bits(&state), state_bits(&held));
                frame_untouched(&frame);
            }
            let (alloy, mut state) = start(stress, e);
            let mut frame = blank_frame();
            assert_eq!(step(&alloy, &mut state, &mut frame, load(stress), dt_s), Ok(()), "{stress} {e}");
            assert_eq!(frame.flags & FLAG_OUTSIDE_MET_RANGE, FLAG_OUTSIDE_MET_RANGE, "{stress} {e}");
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        }
    }

    #[test]
    fn an_extension_moved_after_the_step_is_named_in_its_direction() {
        // Rule 10, the falsified result: the real Runge–Kutta step, then the
        // extension it reached moved by one part in 10^4 of the step's own increment.
        let states = [(140.0, 0.1), (140.0, 0.18), (170.0, 0.3), (230.0, 0.1), (230.0, 3.0)];
        for (factor, expected) in [
            (1.0 + 1.0e-4, StepReject::SpontaneousGeneration),
            (1.0 - 1.0e-4, StepReject::FirstLawBreach),
        ] {
            for (stress, e) in states {
                let (alloy, mut state) = start(stress, e);
                let held = state;
                let mut frame = blank_frame();
                let end = step_with(&alloy, &mut state, &mut frame, 20.0, |a, s, e, dt| {
                    let honest = rk4(a, s, e, dt);
                    e + (honest - e) * factor
                });
                assert_eq!(end, Err(expected), "{stress} MPa from {e} percent");
                assert_eq!(state_bits(&state), state_bits(&held));
                frame_untouched(&frame);
            }
        }
        // The honest step from each of the same states lands.
        for (stress, e) in states {
            let (alloy, mut state) = start(stress, e);
            let mut frame = blank_frame();
            assert_eq!(step(&alloy, &mut state, &mut frame, load(stress), 20.0), Ok(()));
        }
    }

    #[test]
    fn the_law_time_is_the_step_for_an_exact_step_and_the_quadrature_is_independent() {
        let alloy = CreepCmsx6::at_980_c();
        // Against a 2000-panel midpoint sum of 3600 de / rate.
        for (stress, from, to) in [(140.0, 0.1, 0.100_01), (170.0, 0.26, 0.2605), (230.0, 2.0, 2.01)] {
            let panels = 2000;
            let width = (to - from) / panels as f64;
            let mut sum = 0.0;
            let mut index = 0;
            while index < panels {
                sum += width / rate_per_s(&alloy, stress, from + (index as f64 + 0.5) * width);
                index += 1;
            }
            let got = law_time_s(&alloy, stress, from, to);
            assert!((got - sum).abs() <= 1.0e-7 * sum, "{stress}: {got} {sum}");
        }
    }

    #[test]
    fn a_stress_changed_between_steps_is_refused_and_the_held_stress_goes_on() {
        let (alloy, mut state) = start(170.0, 0.5);
        let mut frame = blank_frame();
        assert_eq!(step(&alloy, &mut state, &mut frame, load(170.0), 30.0), Ok(()));
        let first = state;
        let landed = frame.to_bytes();
        for other in [210.0, 140.0, one_over(170.0)] {
            assert_eq!(
                step(&alloy, &mut state, &mut frame, load(other), 30.0),
                Err(StepReject::StressNotHeld)
            );
            assert_eq!(state_bits(&state), state_bits(&first));
            assert_eq!(frame.to_bytes(), landed);
        }
        assert_eq!(step(&alloy, &mut state, &mut frame, load(170.0), 30.0), Ok(()));
        let expected = first.work_by_load_mj_per_m3
            + 170.0 * (state.creep_extension_percent - first.creep_extension_percent) / 100.0;
        assert_eq!(state.work_by_load_mj_per_m3.to_bits(), expected.to_bits());
        assert_eq!(state.stress_mpa, 170.0);
        assert!(state.residual_conservation_error < 1.0e-7);
    }

    #[test]
    fn the_same_steps_give_the_same_bits() {
        let mut ends = [[0_u64; 9]; 2];
        let mut pass = 0;
        while pass < 2 {
            let (alloy, mut state) = start(185.0, 0.1);
            let mut frame = blank_frame();
            let mut steps = 0;
            while steps < 500 {
                assert_eq!(step(&alloy, &mut state, &mut frame, load(185.0), 45.0), Ok(()));
                steps += 1;
            }
            ends[pass] = state_bits(&state);
            pass += 1;
        }
        assert_eq!(ends[0], ends[1]);
    }

}
