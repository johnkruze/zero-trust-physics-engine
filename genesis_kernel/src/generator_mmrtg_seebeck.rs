#![no_implicit_prelude]

//! One Multi-Mission Radioisotope Thermoelectric Generator, flight unit F1,
//! on NASA's Curiosity rover, from its landing on 2012-08-06, with the
//! electrical output in the Seebeck matched-load form: the companion of
//! [`crate::generator_mmrtg_f1`], served beside it and never in its place.
//!
//! Calibrated to the record, and not a blind test. The structure was
//! preferred, and its three constants fitted, with every published day of
//! the record open, so no comparison with those days is a test of it. The
//! model compared blind is `generator_mmrtg_f1`, served beside it under its
//! own word.
//!
//! Measured by Whiting and Woerner, Historical RTG Performance Data - 2023
//! Update, Dryad, doi: 10.5061/dryad.1zcrjdfw2, CC0 1.0, version 6 of
//! 2024-02-13. The authors ask that any use also cite Whiting and Woerner,
//! Chapter 8 of The Technology of Discovery (Wiley, 2023, ISBN
//! 9781119811367).
//!
//! The structure, named D in the record's protocol of 2026-10-06 before any
//! held-back day was opened. The temperature drop across the thermocouples
//! follows the heat, and their output into a matched load follows the
//! square of that drop, so the fuel's decay enters twice; a share `c` of
//! the converter's output wears away at `m` times the fuel's rate:
//!
//! ```text
//! P / 115.4 W = a exp(−2λt) [(1 − c) + c exp(−m λ t)]
//! a = 0.99401   c = 0.230509   m = 18.777   λ = ln 2 / 87.7 years
//! ```
//!
//! The three constants are those of the forecast registered for days after
//! 4,087 (gateway commit a0aff5e, protocol SHA-256 0afc18e0...86d1): fitted
//! by ordinary least squares to every published day, mission days 0 to
//! 4,087, each weighted equally, and rounded to six significant figures,
//! which are the constants. The card's output on every day after 4,087 is
//! that forecast, waiting for the dataset's next release.
//!
//! What was seen before the structure was preferred, after the fact. The
//! served card, `generator_mmrtg_f1` (structure C), was compared with the
//! 1,279 held-back days 2,809 to 4,087 once, on 2026-10-06: its mean error
//! over the final 365 days was +0.0100936 of 115.4 W against a line of
//! 0.010, outside it by 0.0000936. Then, with those days open, structure D
//! with the constants fixed on days 0 to 2,060 on 2026-10-06 (a = 0.995915,
//! c = 0.217256, m = 21.1586) gave a root mean square error of 0.00507 and a
//! final-365 mean of +0.00468 on days 2,809 to 4,087, and 0.00572 and
//! +0.00187 on Perseverance's generator F2, its days 0 to 985. Those are
//! after the fact. This card carries the constants fitted on every day, and
//! its root mean square error on days 0 to 4,087, in sample, is 0.00319.
//!
//! The card gives the daily-average electrical output. It has no seasons,
//! no load and no events. Past day 4,087, the last day of the published
//! record and of the days the constants were fitted on, the card carries
//! the same law forward and the frame sets [`FLAG_PAST_FITTING_SPAN`] and
//! [`FLAG_PAST_RECORD`]: the generator did not stop that day, the record
//! did. A mission time before landing is
//! [`StepReject::MissionTimeBeforeLanding`]. The card stops where the decay
//! heat the fuel still holds is under the ledger's resolution,
//! [`StepReject::DecayHeatBelowLedgerResolution`], about 2,070 years after
//! landing: past it a breach of the ledger the size of its own limit could
//! not be seen.
//!
//! The fuel, the heat and the clock are `generator_mmrtg_f1`'s, number for
//! number. The record holds no heat measurement: the thermal power at
//! landing is a pre-flight estimate, and thermal power, decay heat still
//! held and heat rejected are the card's arithmetic and have not been
//! compared with the generator. The record cannot separate the fuel's share
//! of the loss from the converter's; this structure gives the fuel's decay
//! twice the weight structure C gives it, and the fit decides the rest.
//!
//! The end of the record and rounding. The mission clock is a compensated
//! sum: each step adds `dt` to `t_s` in 64-bit arithmetic, keeps what that
//! addition rounded off in `clock_carry_s`, and folds the carry back in, so
//! `t_s` stays within half a spacing of 64-bit numbers of the exact sum of
//! the start and the steps however many steps a run takes. A start, and a
//! restart from a returned `t_s`, begins with no carry. Every step, the
//! first included, compares its landing with [`MISSION_TIME_MAX_S`], the
//! last day of the record, plus one fixed tolerance,
//! [`MISSION_END_TOLERANCE_S`], 2^-22 s, about 2.38e-7 s: four spacings of
//! 64-bit numbers between 2^28 s and 2^29 s, where the last day lies. That
//! covers the rounding of a start and a step read from decimals and the
//! clock's own rounding, so a run or a single step whose end in decimal is
//! exactly mission day 4,087 lands its last step with neither flag, and so
//! does a restart from any returned state of such a run. A step whose
//! landing, as the clock reads it, is past mission day 4,087 by more than
//! the tolerance lands with both flags set, at any length of run. The clock
//! is stepped at its own value and nothing is pulled back: `range_margin_s`
//! is the time left to the last day of the record, and zero on and past it.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, electrical power, thermal power, the converter
//! fading fraction, electrical energy, heat rejected, decay heat still held,
//! the range margin, and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.
//!
//! The public step is the exact map, not a Runge–Kutta step:
//!
//! ```text
//! thermal power           dQ/dt = −λ Q            λ = FUEL_DECAY_PER_S
//! fading fraction         dg/dt = −m λ g          m λ = CONVERTER_FADING_RATE_PER_S, g = 1 at landing
//! conversion efficiency   η = (Q / Q0) (η_s + η_f g)   STABLE_EFFICIENCY, FADING_EFFICIENCY, Q0 at landing
//! electrical power        P = η Q = (Q² / Q0) (η_s + η_f g)
//! decay heat still held   U = Q / λ
//!
//! x1 = 1 − exp(−λ dt)   x2 = 1 − exp(−m λ dt)
//! xs = 1 − exp(−2λ dt)  xf = 1 − exp(−(2λ + m λ) dt)
//! Q' = Q − Q x1
//! g' = g − g x2
//! ΔW_stable     = η_s Q (Q / Q0) xs / (2λ)
//! ΔW_fading     = η_f Q (Q / Q0) g xf / (2λ + m λ)
//! ΔW_rejected   = Q x1 / λ − ΔW_stable − ΔW_fading
//!
//! residual_conservation_error = |U − U_anchor + W_electrical + W_rejected| / card_heat_j
//! ```
//!
//! The ledger is energy: decay heat released is electrical energy delivered
//! plus heat rejected. `card_heat_j` is the decay heat the fuel releases from
//! landing to day 4,087, the ledger's fixed scale at every mission time, the
//! same number as `generator_mmrtg_f1`'s. The ledger fixes the total that
//! leaves the fuel. It does not fix how that total divides between
//! electricity and rejected heat. Its resolution is
//! [`RESIDUAL_CONSERVATION_LIMIT`] of that scale, about 6.6e5 J: when the
//! decay heat still held falls under it, about 2,070 years after landing, a
//! breach the size of the limit is larger than all the heat left to break,
//! and the step is [`StepReject::DecayHeatBelowLedgerResolution`].
//!
//! The card also keeps one account of the fitted model. It is bookkeeping
//! that follows from the model and not a conservation law:
//!
//! ```text
//! fading yield still to come   Y = η_f Q (Q / Q0) g / (2λ + m λ)
//! converter_account_error = |Y − Y_anchor + W_fading| / card_fading_yield_j
//! ```
//!
//! `W_fading` is the electrical energy the fading part has delivered. A
//! fading fraction that is not the exact map breaks this account and is
//! [`StepReject::ConverterAccountBreach`]. The account covers the fading
//! part only.
//!
//! [`GeneratorMmrtgSeebeck::gate_dt_s`] is `(2λ + m λ) Δt = 0.02`, about
//! 3,843,486 s, on the card's fastest rate, the fading part's output. The
//! public step does not apply that clock. The formula of the map is exact
//! for every finite `dt > 0` that stays on the card. Its arithmetic rounds
//! once a step: under about 2.3e-7 s the thermal power does not move, and at
//! the last day a first step under 2.9e-8 s, half a spacing of the clock
//! there, does not move the clock.
//! [`GeneratorMmrtgSeebeck::floor_dt_s`], about 3.67 s, is the smallest step
//! the card stands behind. The public step does not apply it either. The
//! gateway refuses a step outside the two.
//!
//! The step calls [`crate::math::exp_m1`]. The state constructor calls
//! [`crate::math::exp`].

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Half-life of plutonium-238. NNDC NuDat 3, ENSDF evaluation of Browne and
/// Tuli, Nuclear Data Sheets 108, 681 (2007). Uncertainty 0.1. The record's
/// fit and its forecast use the same number.
pub const PLUTONIUM_238_HALF_LIFE_YEARS: f64 = 87.7;
/// 365.25 days of 86,400 s. A convention, the record's own: its fit takes a
/// mission day over 365.25 as the time in years.
pub const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;
/// Thermal power of the fuel at landing. Woerner et al. 2013, Table 1,
/// "Surface Ops Spring, 08/06/2012", Qsource. A pre-flight estimate, not a
/// measurement; `generator_mmrtg_f1`'s number.
pub const THERMAL_POWER_AT_LANDING_W: f64 = 1953.2;
/// Highest daily-average electrical power of the first 30 days. The dataset,
/// sheet `MMRTG`, cell C14. The dataset's values are divided by this number.
pub const ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W: f64 = 115.4;
/// `a`: electrical power at landing as a fraction of
/// [`ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W`]. Fitted, mission days 0 to
/// 4,087, the registered forecast's constant.
pub const OUTPUT_AMPLITUDE: f64 = 0.99401;
/// `c`: share of the converter's output at landing that wears away. Fitted,
/// mission days 0 to 4,087, the registered forecast's constant.
pub const CONVERTER_FADING_SHARE: f64 = 0.230509;
/// `m`: how many times the fuel's decay rate the wearing share of the
/// converter's output falls at. Fitted, mission days 0 to 4,087, the
/// registered forecast's constant.
pub const CONVERTER_FADING_RATE_OVER_FUEL_DECAY: f64 = 18.777;
/// Mission day 4,087. The last day the constants were fitted on: every
/// published day. The same day as [`MISSION_TIME_MAX_S`].
pub const FITTING_SPAN_END_S: f64 = 4_087.0 * 86_400.0;
/// Mission day 4,087, 353,116,800 s. Last day of the published record. Past
/// this a step lands with [`FLAG_PAST_FITTING_SPAN`] and
/// [`FLAG_PAST_RECORD`], on the days of the registered blind forecast.
pub const MISSION_TIME_MAX_S: f64 = 4_087.0 * 86_400.0;
/// Four spacings of 64-bit numbers between 2^28 s and 2^29 s, 2^-22 s: how
/// far past [`MISSION_TIME_MAX_S`] the landing of any step may read before
/// the frame sets [`FLAG_PAST_FITTING_SPAN`] and [`FLAG_PAST_RECORD`]. The
/// last day lies between those two powers of two, where the spacing is
/// 2^-24 s; 2^28 s is mission day 3,106.9, and before it the spacing is half
/// that or less, so no rounding made earlier in a run is larger. A start
/// read from a decimal rounds by at most half a spacing, the sum of the
/// steps read from a decimal by at most one, and the compensated clock by at
/// most half; a restart from a returned clock leaves out a carry of at most
/// half more. Four spacings hold all of them. The same for every step.
pub const MISSION_END_TOLERANCE_S: f64 = 2.384_185_791_015_625e-7;

/// `ln 2 / half-life`. 2.504504985818715e-10 per second.
pub const FUEL_DECAY_PER_S: f64 =
    core::f64::consts::LN_2 / (PLUTONIUM_238_HALF_LIFE_YEARS * SECONDS_PER_YEAR);
/// `m λ`. 4.702709011871801e-09 per second.
pub const CONVERTER_FADING_RATE_PER_S: f64 =
    CONVERTER_FADING_RATE_OVER_FUEL_DECAY * FUEL_DECAY_PER_S;
/// `2λ`: the stable part's output falls as the square of the heat, at twice
/// the fuel's rate. 5.00900997163743e-10 per second.
pub const STABLE_OUTPUT_RATE_PER_S: f64 = 2.0 * FUEL_DECAY_PER_S;
/// `2λ + m λ`. 5.203610009035544e-09 per second. The fading part's output,
/// the card's fastest rate.
pub const COMBINED_RATE_PER_S: f64 = STABLE_OUTPUT_RATE_PER_S + CONVERTER_FADING_RATE_PER_S;
/// Share of the thermal power the stable part of the converter turns into
/// electricity at landing. 0.04519114981784456.
pub const STABLE_EFFICIENCY: f64 = ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
    * OUTPUT_AMPLITUDE
    * (1.0 - CONVERTER_FADING_SHARE)
    / THERMAL_POWER_AT_LANDING_W;
/// Share of the thermal power the fading part of the converter turns into
/// electricity at landing. 0.013537477050883677.
pub const FADING_EFFICIENCY: f64 =
    ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W * OUTPUT_AMPLITUDE * CONVERTER_FADING_SHARE
        / THERMAL_POWER_AT_LANDING_W;
/// Decay heat the fuel still holds at landing, `Q / λ`. 7.798746702680e12 J.
pub const FUEL_HEAT_AT_LANDING_J: f64 = THERMAL_POWER_AT_LANDING_W / FUEL_DECAY_PER_S;
/// Electrical energy the fading part has still to deliver at landing,
/// `η_f Q0 / (2λ + m λ)`. 5.081357005977e9 J.
pub const FADING_YIELD_AT_LANDING_J: f64 =
    FADING_EFFICIENCY * THERMAL_POWER_AT_LANDING_W / COMBINED_RATE_PER_S;

/// `|U − U_anchor + W_electrical + W_rejected| / card_heat_j` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// `|Y − Y_anchor + W_fading| / card_fading_yield_j` must stay at or under this.
pub const CONVERTER_ACCOUNT_LIMIT: f64 = RESIDUAL_CONSERVATION_LIMIT;
/// Phase advance `(2λ + m λ) Δt` used by [`GeneratorMmrtgSeebeck::gate_dt_s`].
pub const GATE_PHASE_STEP: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Mission time is past mission day 4,087, the last day the constants were
/// fitted on, by more than [`MISSION_END_TOLERANCE_S`]. The step still lands.
pub const FLAG_PAST_FITTING_SPAN: u64 = 1 << 0;
/// Mission time is past mission day 4,087, the last day of the published
/// record, by more than [`MISSION_END_TOLERANCE_S`]: the days of the
/// registered blind forecast. The step still lands.
pub const FLAG_PAST_RECORD: u64 = 1 << 1;

/// Why a step did not advance the generator or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this generator's.
    InvalidGenerator,
    /// Mission time, incoming or landing, is before landing on Mars, where the card's thermal power and converter are anchored.
    MissionTimeBeforeLanding,
    /// The decay heat the fuel still holds, incoming or landing, is under the ledger's resolution, 1e-6 of the decay heat released from landing to mission day 4,087 (about 6.6e5 J, some 2,070 years after landing): a breach of that size could no longer be seen.
    DecayHeatBelowLedgerResolution,
    /// The trial step created energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed energy.
    FirstLawBreach,
    /// The trial step broke the fitted model's account of the fading part of the converter, which is bookkeeping that follows from the model and not a conservation law.
    ConverterAccountBreach,
}

/// The card has no load. Nothing a caller can set changes what it computes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeneratorMmrtgSeebeckLoad {}

/// The locked generator. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratorMmrtgSeebeck {
    /// Recorded copy of [`PLUTONIUM_238_HALF_LIFE_YEARS`].
    pub plutonium_238_half_life_years: f64,
    /// Recorded copy of [`SECONDS_PER_YEAR`].
    pub seconds_per_year: f64,
    /// Recorded copy of [`THERMAL_POWER_AT_LANDING_W`].
    pub thermal_power_at_landing_w: f64,
    /// Recorded copy of [`ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W`].
    pub electrical_power_at_beginning_of_mission_w: f64,
    /// Recorded copy of [`OUTPUT_AMPLITUDE`].
    pub output_amplitude: f64,
    /// Recorded copy of [`CONVERTER_FADING_SHARE`].
    pub converter_fading_share: f64,
    /// Recorded copy of [`CONVERTER_FADING_RATE_OVER_FUEL_DECAY`].
    pub converter_fading_rate_over_fuel_decay: f64,
    /// Recorded copy of [`FITTING_SPAN_END_S`].
    pub fitting_span_end_s: f64,
    /// Recorded copy of [`MISSION_TIME_MAX_S`].
    pub mission_time_max_s: f64,
    /// Recorded copy of [`MISSION_END_TOLERANCE_S`].
    pub mission_end_tolerance_s: f64,
    /// Recorded copy of [`FUEL_DECAY_PER_S`].
    pub fuel_decay_per_s: f64,
    /// Recorded copy of [`CONVERTER_FADING_RATE_PER_S`].
    pub converter_fading_rate_per_s: f64,
    /// Recorded copy of [`STABLE_OUTPUT_RATE_PER_S`].
    pub stable_output_rate_per_s: f64,
    /// Recorded copy of [`COMBINED_RATE_PER_S`].
    pub combined_rate_per_s: f64,
    /// Recorded copy of [`STABLE_EFFICIENCY`].
    pub stable_efficiency: f64,
    /// Recorded copy of [`FADING_EFFICIENCY`].
    pub fading_efficiency: f64,
    /// Recorded copy of [`FUEL_HEAT_AT_LANDING_J`].
    pub fuel_heat_at_landing_j: f64,
    /// Recorded copy of [`FADING_YIELD_AT_LANDING_J`].
    pub fading_yield_at_landing_j: f64,
    /// Decay heat the fuel releases from landing to [`MISSION_TIME_MAX_S`].
    /// 6.600889916499e11 J, `generator_mmrtg_f1`'s scale. Scale of the
    /// residual. Computed once by the constructor from the constants.
    pub card_heat_j: f64,
    /// Electrical energy the fading part delivers from landing to
    /// [`MISSION_TIME_MAX_S`]. 4.272314451325e9 J. Scale of the converter
    /// account. Computed once by the constructor from the constants.
    pub card_fading_yield_j: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`CONVERTER_ACCOUNT_LIMIT`].
    pub converter_account_limit: f64,
    /// Recorded copy of [`GATE_PHASE_STEP`].
    pub gate_phase_step: f64,
}

impl GeneratorMmrtgSeebeck {
    /// Card: flight unit F1 on the Curiosity rover, in the Seebeck
    /// matched-load form. The constructor takes no arguments.
    #[must_use]
    pub fn curiosity() -> Self {
        let generator = Self {
            plutonium_238_half_life_years: PLUTONIUM_238_HALF_LIFE_YEARS,
            seconds_per_year: SECONDS_PER_YEAR,
            thermal_power_at_landing_w: THERMAL_POWER_AT_LANDING_W,
            electrical_power_at_beginning_of_mission_w: ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W,
            output_amplitude: OUTPUT_AMPLITUDE,
            converter_fading_share: CONVERTER_FADING_SHARE,
            converter_fading_rate_over_fuel_decay: CONVERTER_FADING_RATE_OVER_FUEL_DECAY,
            fitting_span_end_s: FITTING_SPAN_END_S,
            mission_time_max_s: MISSION_TIME_MAX_S,
            mission_end_tolerance_s: MISSION_END_TOLERANCE_S,
            fuel_decay_per_s: FUEL_DECAY_PER_S,
            converter_fading_rate_per_s: CONVERTER_FADING_RATE_PER_S,
            stable_output_rate_per_s: STABLE_OUTPUT_RATE_PER_S,
            combined_rate_per_s: COMBINED_RATE_PER_S,
            stable_efficiency: STABLE_EFFICIENCY,
            fading_efficiency: FADING_EFFICIENCY,
            fuel_heat_at_landing_j: FUEL_HEAT_AT_LANDING_J,
            fading_yield_at_landing_j: FADING_YIELD_AT_LANDING_J,
            card_heat_j: card_heat_from_constants_j(),
            card_fading_yield_j: card_fading_yield_from_constants_j(),
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            converter_account_limit: CONVERTER_ACCOUNT_LIMIT,
            gate_phase_step: GATE_PHASE_STEP,
        };
        if generator.is_valid() {
            generator
        } else {
            ::core::panic!("Curiosity generator in the Seebeck form failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.plutonium_238_half_life_years.to_bits() == PLUTONIUM_238_HALF_LIFE_YEARS.to_bits()
            && self.seconds_per_year.to_bits() == SECONDS_PER_YEAR.to_bits()
            && self.thermal_power_at_landing_w.to_bits() == THERMAL_POWER_AT_LANDING_W.to_bits()
            && self.electrical_power_at_beginning_of_mission_w.to_bits()
                == ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W.to_bits()
            && self.output_amplitude.to_bits() == OUTPUT_AMPLITUDE.to_bits()
            && self.converter_fading_share.to_bits() == CONVERTER_FADING_SHARE.to_bits()
            && self.converter_fading_rate_over_fuel_decay.to_bits()
                == CONVERTER_FADING_RATE_OVER_FUEL_DECAY.to_bits()
            && self.fitting_span_end_s.to_bits() == FITTING_SPAN_END_S.to_bits()
            && self.mission_time_max_s.to_bits() == MISSION_TIME_MAX_S.to_bits()
            && self.mission_end_tolerance_s.to_bits() == MISSION_END_TOLERANCE_S.to_bits()
            && self.fuel_decay_per_s.to_bits() == FUEL_DECAY_PER_S.to_bits()
            && self.converter_fading_rate_per_s.to_bits() == CONVERTER_FADING_RATE_PER_S.to_bits()
            && self.stable_output_rate_per_s.to_bits() == STABLE_OUTPUT_RATE_PER_S.to_bits()
            && self.combined_rate_per_s.to_bits() == COMBINED_RATE_PER_S.to_bits()
            && self.stable_efficiency.to_bits() == STABLE_EFFICIENCY.to_bits()
            && self.fading_efficiency.to_bits() == FADING_EFFICIENCY.to_bits()
            && self.fuel_heat_at_landing_j.to_bits() == FUEL_HEAT_AT_LANDING_J.to_bits()
            && self.fading_yield_at_landing_j.to_bits() == FADING_YIELD_AT_LANDING_J.to_bits()
            && self.card_heat_j.to_bits() == card_heat_from_constants_j().to_bits()
            && self.card_fading_yield_j.to_bits() == card_fading_yield_from_constants_j().to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.converter_account_limit.to_bits() == CONVERTER_ACCOUNT_LIMIT.to_bits()
            && self.gate_phase_step.to_bits() == GATE_PHASE_STEP.to_bits()
            && self.fuel_decay_per_s > 0.0
            && self.converter_fading_rate_per_s > 0.0
            && self.stable_output_rate_per_s > 0.0
            && self.combined_rate_per_s.is_finite()
            && self.combined_rate_per_s > 0.0
            && self.thermal_power_at_landing_w > 0.0
            && self.stable_efficiency > 0.0
            && self.fading_efficiency > 0.0
            && self.stable_efficiency + self.fading_efficiency < 1.0
            && self.fitting_span_end_s > 0.0
            && self.mission_time_max_s >= self.fitting_span_end_s
            && self.card_heat_j.is_finite()
            && self.card_heat_j > 0.0
            && self.card_fading_yield_j.is_finite()
            && self.card_fading_yield_j > 0.0
    }

    /// Largest reporting step of the card's fastest rate, `2λ + m λ`. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let rate = self.combined_rate_per_s;
        if rate.is_finite() && rate > 0.0 {
            Some(GATE_PHASE_STEP / rate)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20, about 3.67 s. At the ceiling a step advances the card's fastest
    /// rate by two hundredths. A million times smaller, a step lowers the
    /// thermal power by about one part in a thousand million, which 64-bit
    /// arithmetic still resolves to seven figures, and at the last day the
    /// clock moves by the step to eight. Under that a step is mostly
    /// rounding. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) => Some(gate_dt_s / GATE_OVER_FLOOR),
            None => None,
        }
    }

    /// The latest mission time, in seconds, a start is held to: where the
    /// decay heat still held, `Q / λ`, falls to the ledger's resolution,
    /// [`RESIDUAL_CONSERVATION_LIMIT`] times `card_heat_j`, about 6.5e10 s,
    /// some 2,070 years after landing, less whole floor steps until a step
    /// of [`Self::floor_dt_s`] from it lands. A start is held to it.
    #[must_use]
    pub fn ledger_end_mission_time_s(&self) -> Option<f64> {
        let resolution_j = self.residual_conservation_limit * self.card_heat_j;
        let floor_dt_s = self.floor_dt_s()?;
        if !(resolution_j > 0.0 && self.fuel_heat_at_landing_j > resolution_j) {
            return None;
        }
        let end_s = crate::math::log(self.fuel_heat_at_landing_j / resolution_j) / self.fuel_decay_per_s;
        if !(end_s.is_finite() && end_s > floor_dt_s) {
            return None;
        }
        let lands = |t_s: f64| -> bool {
            match GeneratorMmrtgSeebeckState::try_mission_time(self, t_s) {
                Some(mut state) => {
                    let mut frame = LastStateFrame64 {
                        t: 0.0,
                        pos: [0.0; 3],
                        vel: [0.0; 3],
                        force_torque: 0.0,
                        residual: 0.0,
                        flags: 0,
                        proof: [0; 16],
                    };
                    step(self, &mut state, &mut frame, GeneratorMmrtgSeebeckLoad {}, floor_dt_s).is_ok()
                }
                None => false,
            }
        };
        let mut t_s = end_s - floor_dt_s;
        let mut guard = 0;
        while !lands(t_s) {
            t_s -= floor_dt_s;
            guard += 1;
            if guard > 64 {
                return None;
            }
        }
        Some(t_s)
    }

    /// Decay heat the fuel still holds at this thermal power, `Q / λ`.
    #[must_use]
    pub fn fuel_heat_j(&self, thermal_power_w: f64) -> f64 {
        thermal_power_w / self.fuel_decay_per_s
    }

    /// The thermocouples' temperature drop as a fraction of its value at
    /// landing: it follows the heat, `Q / Q0`.
    #[must_use]
    pub fn temperature_drop_fraction(&self, thermal_power_w: f64) -> f64 {
        thermal_power_w / self.thermal_power_at_landing_w
    }

    /// Electrical energy the fading part has still to deliver,
    /// `η_f Q (Q / Q0) g / (2λ + m λ)`.
    #[must_use]
    pub fn converter_fading_yield_j(
        &self,
        thermal_power_w: f64,
        converter_fading_fraction: f64,
    ) -> f64 {
        self.fading_efficiency
            * thermal_power_w
            * self.temperature_drop_fraction(thermal_power_w)
            * converter_fading_fraction
            / self.combined_rate_per_s
    }
}

/// `FUEL_HEAT_AT_LANDING_J (1 − exp(−λ MISSION_TIME_MAX_S))`.
fn card_heat_from_constants_j() -> f64 {
    FUEL_HEAT_AT_LANDING_J * -crate::math::exp_m1(-FUEL_DECAY_PER_S * MISSION_TIME_MAX_S)
}

/// `FADING_YIELD_AT_LANDING_J (1 − exp(−(2λ + m λ) MISSION_TIME_MAX_S))`.
fn card_fading_yield_from_constants_j() -> f64 {
    FADING_YIELD_AT_LANDING_J * -crate::math::exp_m1(-COMBINED_RATE_PER_S * MISSION_TIME_MAX_S)
}

/// Integrated generator. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratorMmrtgSeebeckState {
    /// Mission time since landing.
    pub t_s: f64,
    /// What the rounding of the mission clock has left out of `t_s`,
    /// seconds: `t_s + clock_carry_s` is the sum of the start and the steps
    /// to within the rounding of the carry itself. Under half a spacing of
    /// `t_s`. Zero at a start, and so zero at a restart from a returned `t_s`.
    pub clock_carry_s: f64,
    /// Thermal power of the fuel, `Q`.
    pub thermal_power_w: f64,
    /// Fading fraction of the converter's wearing share, `g`. 1 at landing.
    pub converter_fading_fraction: f64,
    /// `Q (Q / Q0) (η_s + η_f g)`.
    pub electrical_power_w: f64,
    /// `(Q / Q0) (η_s + η_f g)`.
    pub conversion_efficiency: f64,
    /// Decay heat the fuel still holds, `U = Q / λ`.
    pub fuel_heat_j: f64,
    /// `U` at the state the ledger started from: the start of this run, not
    /// landing.
    pub energy_anchor_j: f64,
    /// Electrical energy delivered since the anchor: from the start of this
    /// run, not from landing. A restart from a returned state starts it
    /// again at zero. No refusal depends on it.
    pub electrical_energy_j: f64,
    /// Heat rejected since the anchor: from the start of this run, not from
    /// landing. A restart starts it again at zero. No refusal depends on it.
    pub heat_rejected_j: f64,
    /// Signed `U − U_anchor + W_electrical + W_rejected`. Positive is created energy.
    pub energy_defect_j: f64,
    /// `|energy_defect_j| / card_heat_j`. The energy ledger alone.
    pub residual_conservation_error: f64,
    /// Electrical energy the fading part has still to deliver,
    /// `Y = η_f Q (Q / Q0) g / (2λ + m λ)`.
    pub converter_fading_yield_j: f64,
    /// `Y` at the state the account started from.
    pub converter_account_anchor_j: f64,
    /// Electrical energy delivered by the fading part since the anchor: from
    /// the start of this run, not from landing. A restart starts it again at
    /// zero. No refusal depends on it.
    pub fading_electrical_energy_j: f64,
    /// Signed `Y − Y_anchor + W_fading`.
    pub converter_account_defect_j: f64,
    /// `|converter_account_defect_j| / card_fading_yield_j`. An account of
    /// the fitted model: bookkeeping that follows from the model and not a
    /// conservation law.
    pub converter_account_error: f64,
    /// `MISSION_TIME_MAX_S − t_s`, never below zero.
    pub range_margin_s: f64,
    /// `t_s > FITTING_SPAN_END_S + MISSION_END_TOLERANCE_S`.
    pub past_fitting_span: bool,
}

impl GeneratorMmrtgSeebeckState {
    /// The generator at landing, mission time 0.
    #[must_use]
    pub fn try_landing(generator: &GeneratorMmrtgSeebeck) -> Option<Self> {
        Self::try_mission_time(generator, 0.0)
    }

    /// Anchor both ledgers at this mission time. Thermal power and the fading
    /// fraction are the generator's at that time and are not caller inputs.
    #[must_use]
    pub fn try_mission_time(generator: &GeneratorMmrtgSeebeck, t_s: f64) -> Option<Self> {
        if !generator.is_valid() || !on_card(t_s) {
            return None;
        }
        let mut state = Self {
            t_s,
            clock_carry_s: 0.0,
            thermal_power_w: generator.thermal_power_at_landing_w
                * crate::math::exp(-generator.fuel_decay_per_s * t_s),
            converter_fading_fraction: crate::math::exp(
                -generator.converter_fading_rate_per_s * t_s,
            ),
            electrical_power_w: 0.0,
            conversion_efficiency: 0.0,
            fuel_heat_j: 0.0,
            energy_anchor_j: 0.0,
            electrical_energy_j: 0.0,
            heat_rejected_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            converter_fading_yield_j: 0.0,
            converter_account_anchor_j: 0.0,
            fading_electrical_energy_j: 0.0,
            converter_account_defect_j: 0.0,
            converter_account_error: 0.0,
            range_margin_s: 0.0,
            past_fitting_span: false,
        };
        refresh(generator, &mut state);
        if !state_is_finite(&state) || !heat_resolved(generator, state.fuel_heat_j) {
            return None;
        }
        state.energy_anchor_j = state.fuel_heat_j;
        state.converter_account_anchor_j = state.converter_fading_yield_j;
        Some(state)
    }
}

/// Advance one exact step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    generator: &GeneratorMmrtgSeebeck,
    state: &mut GeneratorMmrtgSeebeckState,
    frame: &mut LastStateFrame64,
    load: GeneratorMmrtgSeebeckLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(generator, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("generator_mmrtg_seebeck", exceedances(generator, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    generator: &GeneratorMmrtgSeebeck,
    state: &mut GeneratorMmrtgSeebeckState,
    frame: &mut LastStateFrame64,
    load: GeneratorMmrtgSeebeckLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(generator, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    generator: &GeneratorMmrtgSeebeck,
    state: &mut GeneratorMmrtgSeebeckState,
    frame: &mut LastStateFrame64,
    load: GeneratorMmrtgSeebeckLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(generator, state, load, dt_s)?;
    let advanced = match exact_map(
        generator,
        state.thermal_power_w,
        state.converter_fading_fraction,
        dt_s,
    ) {
        Some(stepped) => stepped,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(generator, state, frame, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut GeneratorMmrtgSeebeckState) {
    state.t_s = normal_or_zero(state.t_s);
    state.clock_carry_s = normal_or_zero(state.clock_carry_s);
    state.thermal_power_w = normal_or_zero(state.thermal_power_w);
    state.converter_fading_fraction = normal_or_zero(state.converter_fading_fraction);
    state.electrical_power_w = normal_or_zero(state.electrical_power_w);
    state.conversion_efficiency = normal_or_zero(state.conversion_efficiency);
    state.fuel_heat_j = normal_or_zero(state.fuel_heat_j);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.electrical_energy_j = normal_or_zero(state.electrical_energy_j);
    state.heat_rejected_j = normal_or_zero(state.heat_rejected_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.converter_fading_yield_j = normal_or_zero(state.converter_fading_yield_j);
    state.converter_account_anchor_j = normal_or_zero(state.converter_account_anchor_j);
    state.fading_electrical_energy_j = normal_or_zero(state.fading_electrical_energy_j);
    state.converter_account_defect_j = normal_or_zero(state.converter_account_defect_j);
    state.converter_account_error = normal_or_zero(state.converter_account_error);
    state.range_margin_s = normal_or_zero(state.range_margin_s);
}

fn on_card(t_s: f64) -> bool {
    t_s.is_finite() && t_s >= 0.0
}

/// The landing of a step is past the last day of the record by more than
/// [`MISSION_END_TOLERANCE_S`]. The same comparison for every step, the
/// first included.
fn past_record(generator: &GeneratorMmrtgSeebeck, t_s: f64) -> bool {
    t_s > generator.mission_time_max_s + generator.mission_end_tolerance_s
}

/// The clock is past the last day the constants were fitted on by more
/// than [`MISSION_END_TOLERANCE_S`], the same comparison as [`past_record`]
/// on the same day.
fn past_fitting_span(generator: &GeneratorMmrtgSeebeck, t_s: f64) -> bool {
    t_s > generator.fitting_span_end_s + generator.mission_end_tolerance_s
}

/// The decay heat still held is at or above the ledger's resolution.
fn heat_resolved(generator: &GeneratorMmrtgSeebeck, fuel_heat_j: f64) -> bool {
    fuel_heat_j >= generator.residual_conservation_limit * generator.card_heat_j
}

/// The compensated mission clock: `t_s + carry_s` advanced by `dt_s`. The
/// addition's rounding is found exactly (Knuth's two-sum), added to the
/// carry, and folded back into the clock (Dekker's fast two-sum), so the
/// clock returned is within half a spacing of the exact sum and the carry
/// returned is what it leaves out.
fn advance_clock(t_s: f64, carry_s: f64, dt_s: f64) -> (f64, f64) {
    let sum_s = t_s + dt_s;
    let dt_part_s = sum_s - t_s;
    let rounded_off_s = (t_s - (sum_s - dt_part_s)) + (dt_s - dt_part_s);
    let carry_s = carry_s + rounded_off_s;
    let clock_s = sum_s + carry_s;
    let left_s = carry_s - (clock_s - sum_s);
    (clock_s, left_s)
}

fn validate(
    generator: &GeneratorMmrtgSeebeck,
    state: &GeneratorMmrtgSeebeckState,
    load: GeneratorMmrtgSeebeckLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    // The load has no field, so none can be missing or non-finite.
    let GeneratorMmrtgSeebeckLoad {} = load;
    if !dt_s.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !generator.is_valid() {
        return Err(StepReject::InvalidGenerator);
    }
    if !on_card(state.t_s) {
        return Err(StepReject::MissionTimeBeforeLanding);
    }
    if !heat_resolved(generator, state.fuel_heat_j) {
        return Err(StepReject::DecayHeatBelowLedgerResolution);
    }
    let (landing_s, _) = advance_clock(state.t_s, state.clock_carry_s, dt_s);
    if !on_card(landing_s) {
        return Err(StepReject::MissionTimeBeforeLanding);
    }
    Ok(())
}

fn commit(
    generator: &GeneratorMmrtgSeebeck,
    state: &mut GeneratorMmrtgSeebeckState,
    frame: &mut LastStateFrame64,
    load: GeneratorMmrtgSeebeckLoad,
    dt_s: f64,
    advanced: ExactStep,
) -> Result<(), StepReject> {
    let GeneratorMmrtgSeebeckLoad {} = load;
    let saved = *state;
    if !advanced.thermal_power_w.is_finite()
        || !advanced.converter_fading_fraction.is_finite()
        || !advanced.stable_electrical_energy_j.is_finite()
        || !advanced.fading_electrical_energy_j.is_finite()
        || !advanced.heat_rejected_j.is_finite()
    {
        return Err(StepReject::NonFiniteState);
    }
    let (landing_s, landing_carry_s) = advance_clock(state.t_s, state.clock_carry_s, dt_s);
    if !on_card(landing_s) {
        return Err(StepReject::MissionTimeBeforeLanding);
    }
    state.thermal_power_w = advanced.thermal_power_w;
    state.converter_fading_fraction = advanced.converter_fading_fraction;
    state.electrical_energy_j = state.electrical_energy_j
        + advanced.stable_electrical_energy_j
        + advanced.fading_electrical_energy_j;
    state.fading_electrical_energy_j += advanced.fading_electrical_energy_j;
    state.heat_rejected_j += advanced.heat_rejected_j;
    state.t_s = landing_s;
    state.clock_carry_s = landing_carry_s;
    refresh(generator, state);
    let scale = generator.card_heat_j;
    let account_scale = generator.card_fading_yield_j;
    if !(scale > 0.0 && scale.is_finite() && account_scale > 0.0 && account_scale.is_finite()) {
        *state = saved;
        return Err(StepReject::InvalidGenerator);
    }
    state.energy_defect_j = state.fuel_heat_j - state.energy_anchor_j
        + state.electrical_energy_j
        + state.heat_rejected_j;
    state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    state.converter_account_defect_j = state.converter_fading_yield_j
        - state.converter_account_anchor_j
        + state.fading_electrical_energy_j;
    state.converter_account_error = state.converter_account_defect_j.abs() / account_scale;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if !heat_resolved(generator, state.fuel_heat_j) {
        *state = saved;
        return Err(StepReject::DecayHeatBelowLedgerResolution);
    }
    let gate = generator.residual_conservation_limit * scale;
    if state.energy_defect_j > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    if state.converter_account_error > generator.converter_account_limit {
        *state = saved;
        return Err(StepReject::ConverterAccountBreach);
    }
    let flags_past_record = past_record(generator, state.t_s);
    publish(state, frame, flags_past_record);
    Ok(())
}

fn refresh(generator: &GeneratorMmrtgSeebeck, state: &mut GeneratorMmrtgSeebeckState) {
    // The thermocouples' temperature drop follows the heat, so the
    // efficiency falls with it: the fuel's decay enters twice.
    state.conversion_efficiency = generator.temperature_drop_fraction(state.thermal_power_w)
        * (generator.stable_efficiency
            + generator.fading_efficiency * state.converter_fading_fraction);
    state.electrical_power_w = state.thermal_power_w * state.conversion_efficiency;
    state.fuel_heat_j = generator.fuel_heat_j(state.thermal_power_w);
    state.converter_fading_yield_j =
        generator.converter_fading_yield_j(state.thermal_power_w, state.converter_fading_fraction);
    // Never below zero: a clock on or past the last day of the record has
    // no margin left.
    let margin_s = generator.mission_time_max_s - state.t_s;
    state.range_margin_s = if margin_s > 0.0 { margin_s } else { 0.0 };
    state.past_fitting_span = past_fitting_span(generator, state.t_s);
}

struct ExactStep {
    thermal_power_w: f64,
    converter_fading_fraction: f64,
    /// Electrical energy delivered by the stable part across the step.
    stable_electrical_energy_j: f64,
    /// Electrical energy delivered by the fading part across the step.
    fading_electrical_energy_j: f64,
    heat_rejected_j: f64,
}

/// `Q − Q x1`, `g − g x2`, and the electrical energy of each part from its
/// own integral, the heat rejected the rest of the heat released. `Q` and
/// `g` are the values at the start of the step.
fn exact_map(
    generator: &GeneratorMmrtgSeebeck,
    thermal_power_w: f64,
    converter_fading_fraction: f64,
    dt_s: f64,
) -> Option<ExactStep> {
    let fuel_decay_per_s = generator.fuel_decay_per_s;
    let fading_rate_per_s = generator.converter_fading_rate_per_s;
    let stable_rate_per_s = generator.stable_output_rate_per_s;
    let combined_rate_per_s = generator.combined_rate_per_s;
    if !(fuel_decay_per_s > 0.0)
        || !(fading_rate_per_s > 0.0)
        || !(stable_rate_per_s > 0.0)
        || !(combined_rate_per_s > 0.0)
    {
        return None;
    }
    let fuel_decrement = -crate::math::exp_m1(-fuel_decay_per_s * dt_s);
    let fading_decrement = -crate::math::exp_m1(-fading_rate_per_s * dt_s);
    let stable_decrement = -crate::math::exp_m1(-stable_rate_per_s * dt_s);
    let combined_decrement = -crate::math::exp_m1(-combined_rate_per_s * dt_s);
    if !fuel_decrement.is_finite()
        || !fading_decrement.is_finite()
        || !stable_decrement.is_finite()
        || !combined_decrement.is_finite()
    {
        return None;
    }
    let next_thermal_power_w = thermal_power_w - thermal_power_w * fuel_decrement;
    let next_fading_fraction =
        converter_fading_fraction - converter_fading_fraction * fading_decrement;
    // `x1 / λ`, `xs / (2λ)` and `xf / (2λ + m λ)`: the time integrals of the
    // three decays over the step.
    let fuel_integral_s = fuel_decrement / fuel_decay_per_s;
    let stable_integral_s = stable_decrement / stable_rate_per_s;
    let combined_integral_s = combined_decrement / combined_rate_per_s;
    let drop = generator.temperature_drop_fraction(thermal_power_w);
    let stable_electrical_energy_j =
        generator.stable_efficiency * thermal_power_w * drop * stable_integral_s;
    let fading_electrical_energy_j = generator.fading_efficiency
        * thermal_power_w
        * drop
        * converter_fading_fraction
        * combined_integral_s;
    let heat_rejected_j =
        thermal_power_w * fuel_integral_s - stable_electrical_energy_j - fading_electrical_energy_j;
    if next_thermal_power_w.is_finite()
        && next_fading_fraction.is_finite()
        && stable_electrical_energy_j.is_finite()
        && fading_electrical_energy_j.is_finite()
        && heat_rejected_j.is_finite()
    {
        Some(ExactStep {
            thermal_power_w: next_thermal_power_w,
            converter_fading_fraction: next_fading_fraction,
            stable_electrical_energy_j,
            fading_electrical_energy_j,
            heat_rejected_j,
        })
    } else {
        None
    }
}

fn publish(state: &GeneratorMmrtgSeebeckState, frame: &mut LastStateFrame64, past_record: bool) {
    frame.t = state.t_s;
    frame.pos[0] = state.electrical_power_w as f32;
    frame.pos[1] = state.thermal_power_w as f32;
    frame.pos[2] = state.converter_fading_fraction as f32;
    frame.vel[0] = state.electrical_energy_j as f32;
    frame.vel[1] = state.heat_rejected_j as f32;
    frame.vel[2] = state.fuel_heat_j as f32;
    frame.force_torque = state.range_margin_s as f32;
    frame.residual = state.residual_conservation_error as f32;
    let fitting = if state.past_fitting_span { FLAG_PAST_FITTING_SPAN } else { 0 };
    let record = if past_record { FLAG_PAST_RECORD } else { 0 };
    frame.flags = fitting | record;
}

fn state_is_finite(state: &GeneratorMmrtgSeebeckState) -> bool {
    state.t_s.is_finite()
        && state.clock_carry_s.is_finite()
        && state.thermal_power_w.is_finite()
        && state.converter_fading_fraction.is_finite()
        && state.electrical_power_w.is_finite()
        && state.conversion_efficiency.is_finite()
        && state.fuel_heat_j.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.electrical_energy_j.is_finite()
        && state.heat_rejected_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.converter_fading_yield_j.is_finite()
        && state.converter_account_anchor_j.is_finite()
        && state.fading_electrical_energy_j.is_finite()
        && state.converter_account_defect_j.is_finite()
        && state.converter_account_error.is_finite()
        && state.range_margin_s.is_finite()
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    FITTING_KIND,
    RECORD_KIND,
];

const FITTING_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_PAST_FITTING_SPAN",
    FLAG_PAST_FITTING_SPAN,
    "t_s",
    "s",
    Side::Above,
    FITTING_SPAN_END_S,
    "mission day 4,087, the last day the constants were fitted on, in seconds of mission time; set past it by more than 2^-22 s",
);
const RECORD_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_PAST_RECORD",
    FLAG_PAST_RECORD,
    "t_s",
    "s",
    Side::Above,
    MISSION_TIME_MAX_S,
    "mission day 4,087, the last day of the published record, in seconds; set past it by more than 2^-22 s: the forecast's days",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
pub fn exceedances(
    generator: &GeneratorMmrtgSeebeck,
    before: &GeneratorMmrtgSeebeckState,
    after: &GeneratorMmrtgSeebeckState,
    load: GeneratorMmrtgSeebeckLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, load, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.past_fitting_span, Exceedance::at(FITTING_KIND, generator.fitting_span_end_s, after.t_s));
    records.push_if(past_record(generator, after.t_s), Exceedance::at(RECORD_KIND, generator.mission_time_max_s, after.t_s));
    records
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
        COMBINED_RATE_PER_S, CONVERTER_ACCOUNT_LIMIT, CONVERTER_FADING_RATE_OVER_FUEL_DECAY,
        CONVERTER_FADING_RATE_PER_S, CONVERTER_FADING_SHARE, ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W,
        ExactStep, FADING_EFFICIENCY, FADING_YIELD_AT_LANDING_J, FITTING_SPAN_END_S, FLAG_PAST_FITTING_SPAN,
        FLAG_PAST_RECORD, FUEL_DECAY_PER_S, FUEL_HEAT_AT_LANDING_J, GATE_PHASE_STEP, GeneratorMmrtgSeebeck,
        GeneratorMmrtgSeebeckLoad, GeneratorMmrtgSeebeckState, MISSION_END_TOLERANCE_S, MISSION_TIME_MAX_S,
        OUTPUT_AMPLITUDE, RESIDUAL_CONSERVATION_LIMIT, SECONDS_PER_YEAR, STABLE_EFFICIENCY,
        STABLE_OUTPUT_RATE_PER_S, StepReject, THERMAL_POWER_AT_LANDING_W, advance_clock, commit, exact_map,
        step, validate,
    };
    use crate::frame::LastStateFrame64;
    use crate::generator_mmrtg_f1;

    const DAY_S: f64 = 86_400.0;
    const NO_LOAD: GeneratorMmrtgSeebeckLoad = GeneratorMmrtgSeebeckLoad {};

    /// One mission day of the closed form of the header, worked in 50-digit
    /// decimal arithmetic (mpmath) and cut to seventeen figures: the state a
    /// run of daily steps from landing reaches on that day, and the
    /// electrical energy and heat rejected it has booked since landing.
    struct ReferenceDay {
        day: u32,
        thermal_power_w: f64,
        electrical_power_w: f64,
        ratio: f64,
        fading_fraction: f64,
        electrical_energy_j: f64,
        heat_rejected_j: f64,
    }

    const REFERENCE_DAYS: [ReferenceDay; 8] = [
        ReferenceDay {
            day: 1,
            thermal_power_w: 1953.1577353127279,
            electrical_power_w: 114.69304889416829,
            ratio: 0.99387390722849473,
            fading_fraction: 0.99959376847575272,
            electrical_energy_j: 9910157.8453647549,
            heat_rejected_j: 158844496.31356024,
        },
        ReferenceDay {
            day: 365,
            thermal_power_w: 1937.8339841037524,
            electrical_power_w: 109.32366085755966,
            ratio: 0.94734541471022233,
            fading_fraction: 0.86216843135835152,
            electrical_energy_j: 3530760683.9959122,
            heat_rejected_j: 57822744233.162262,
        },
        ReferenceDay {
            day: 2060,
            thermal_power_w: 1868.045826074936,
            electrical_power_w: 91.21139304912776,
            ratio: 0.79039335397857677,
            fading_fraction: 0.43300458558549545,
            electrical_energy_j: 18099006337.464095,
            heat_rejected_j: 321905004064.72479,
        },
        ReferenceDay {
            day: 2061,
            thermal_power_w: 1868.0054040123469,
            electrical_power_w: 91.20319154609599,
            ratio: 0.79032228376166369,
            fading_fraction: 0.432828685472687,
            electrical_energy_j: 18106886647.502282,
            heat_rejected_j: 322058521167.82008,
        },
        ReferenceDay {
            day: 2808,
            thermal_power_w: 1838.0531801201942,
            electrical_power_w: 85.648713976438516,
            ratio: 0.74218989580969252,
            fading_fraction: 0.31952176781114951,
            electrical_energy_j: 23808137995.425227,
            heat_rejected_j: 435950658779.99709,
        },
        ReferenceDay {
            day: 2809,
            thermal_power_w: 1838.0134070591419,
            electrical_power_w: 85.64196814266666,
            ratio: 0.74213143971114956,
            fading_fraction: 0.3193919679963814,
            electrical_energy_j: 23815537752.880832,
            heat_rejected_j: 436102065099.10144,
        },
        ReferenceDay {
            day: 3448,
            thermal_power_w: 1812.7735970188453,
            electrical_power_w: 81.642595207851385,
            ratio: 0.70747482849091322,
            fading_fraction: 0.24635766550907138,
            electrical_energy_j: 28430658532.600454,
            heat_rejected_j: 532264583745.9842,
        },
        ReferenceDay {
            day: 4087,
            thermal_power_w: 1787.8803829328673,
            electrical_power_w: 78.16767558603774,
            ratio: 0.67736287336254541,
            fading_fraction: 0.19002387485137735,
            electrical_energy_j: 32840075853.927123,
            heat_rejected_j: 627248915796.01447,
        },
    ];

    /// The registered forecast's own table: rows of
    /// `companion_seebeck/forecast_table.csv`, column `primary_D_0_4087`, to
    /// its twelve decimals, on the days its protocol names (986, the first
    /// day of Perseverance not yet published; 2,060, 2,808, 2,809 and 4,087,
    /// the ends of the record's spans; 4,088, 4,452, 4,818 and 5,183, the
    /// forecast) and four more by rule, the day after each of the first two
    /// and two days between. The table is the closed form of the protocol
    /// on that day; the protocol's SHA-256 is registered in the gateway at
    /// a0aff5e.
    const FORECAST_TABLE: [(u32, f64); 12] = [
        (986, 0.880010453599),
        (1000, 0.878643650353),
        (2060, 0.790393353979),
        (2061, 0.790322283762),
        (2808, 0.742189895810),
        (2809, 0.742131439711),
        (3448, 0.707474828491),
        (4087, 0.677362873363),
        (4088, 0.677318739978),
        (4452, 0.661797565865),
        (4818, 0.647186590932),
        (5183, 0.633480592952),
    ];

    /// The protocol's printed forecast, to its six decimals.
    const PROTOCOL_TABLE: [(u32, f64); 4] = [
        (4088, 0.677319),
        (4452, 0.661798),
        (4818, 0.647187),
        (5183, 0.633481),
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

    fn assert_copy<T: Copy>() {}

    fn state_bits(state: &GeneratorMmrtgSeebeckState) -> [u64; 19] {
        [
            state.t_s.to_bits(),
            state.clock_carry_s.to_bits(),
            state.thermal_power_w.to_bits(),
            state.converter_fading_fraction.to_bits(),
            state.electrical_power_w.to_bits(),
            state.conversion_efficiency.to_bits(),
            state.fuel_heat_j.to_bits(),
            state.energy_anchor_j.to_bits(),
            state.electrical_energy_j.to_bits(),
            state.heat_rejected_j.to_bits(),
            state.energy_defect_j.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.converter_fading_yield_j.to_bits(),
            state.converter_account_anchor_j.to_bits(),
            state.fading_electrical_energy_j.to_bits(),
            state.converter_account_defect_j.to_bits(),
            state.converter_account_error.to_bits(),
            state.range_margin_s.to_bits(),
            u64::from(state.past_fitting_span),
        ]
    }

    fn within(got: f64, expected: f64, relative: f64) -> bool {
        (got - expected).abs() <= relative * expected.abs()
    }

    /// `115.4 W × 0.99401 × exp(−2λ t) × [0.769491 + 0.230509 exp(−m λ t)]`.
    fn closed_form_electrical_power_w(t_s: f64) -> f64 {
        ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
            * OUTPUT_AMPLITUDE
            * crate::math::exp(-2.0 * FUEL_DECAY_PER_S * t_s)
            * ((1.0 - CONVERTER_FADING_SHARE)
                + CONVERTER_FADING_SHARE * crate::math::exp(-CONVERTER_FADING_RATE_PER_S * t_s))
    }

    fn landing() -> (GeneratorMmrtgSeebeck, GeneratorMmrtgSeebeckState) {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let state = GeneratorMmrtgSeebeckState::try_landing(&generator).unwrap();
        (generator, state)
    }

    /// The real `validate`, the real exact map with one alteration, the real `commit`.
    fn step_altered(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        alter: impl FnOnce(&GeneratorMmrtgSeebeckState, &mut ExactStep),
    ) -> Result<(), StepReject> {
        validate(generator, state, NO_LOAD, dt_s)?;
        let mut advanced = match exact_map(
            generator,
            state.thermal_power_w,
            state.converter_fading_fraction,
            dt_s,
        ) {
            Some(stepped) => stepped,
            None => return Err(StepReject::NonFiniteState),
        };
        alter(state, &mut advanced);
        commit(generator, state, frame, NO_LOAD, dt_s, advanced)
    }

    /// Thermal power on a straight line, `Q (1 − λ dt)`, against the exact heats.
    fn step_straight_line_fuel(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |start, advanced| {
            advanced.thermal_power_w =
                start.thermal_power_w - start.thermal_power_w * FUEL_DECAY_PER_S * dt_s;
        })
    }

    /// Fading fraction on a straight line, `g (1 − m λ dt)`, against the exact energies.
    fn step_straight_line_fading(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |start, advanced| {
            advanced.converter_fading_fraction = start.converter_fading_fraction
                - start.converter_fading_fraction * CONVERTER_FADING_RATE_PER_S * dt_s;
        })
    }

    /// The efficiency held at its value at the start of the step, as a
    /// converter whose temperature drop did not follow the heat would hold
    /// it: the stable part's energy as `η_s Q (Q / Q0) x1 / λ`, the fuel's
    /// rate and not twice it, against the exact heats.
    fn step_single_decay(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |start, advanced| {
            let fuel_integral_s = -crate::math::exp_m1(-FUEL_DECAY_PER_S * dt_s) / FUEL_DECAY_PER_S;
            advanced.stable_electrical_energy_j = STABLE_EFFICIENCY
                * start.thermal_power_w
                * (start.thermal_power_w / THERMAL_POWER_AT_LANDING_W)
                * fuel_integral_s;
        })
    }

    /// Add `extra_j` to the electrical energy without taking it from the fuel.
    fn step_bias_electrical_energy(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        extra_j: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |_, advanced| {
            advanced.stable_electrical_energy_j += extra_j;
        })
    }

    /// Add `extra_j` to the heat rejected without taking it from the fuel.
    fn step_bias_heat_rejected(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        extra_j: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |_, advanced| {
            advanced.heat_rejected_j += extra_j;
        })
    }

    /// Move `moved_j` of electrical energy from the stable part to the fading
    /// part. The total electrical energy and the heat rejected are unchanged.
    fn step_bias_fading_part(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        moved_j: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |_, advanced| {
            advanced.stable_electrical_energy_j -= moved_j;
            advanced.fading_electrical_energy_j += moved_j;
        })
    }

    /// Electrical energy as power at the start times `dt`. Heat rejected exact.
    fn step_rectangle(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |start, advanced| {
            advanced.stable_electrical_energy_j =
                start.electrical_power_w * dt_s - advanced.fading_electrical_energy_j;
        })
    }

    /// Move `share` of the step's electrical energy out of the heat rejected.
    fn step_wrong_split(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        share: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |_, advanced| {
            let moved_j =
                share * (advanced.stable_electrical_energy_j + advanced.fading_electrical_energy_j);
            advanced.stable_electrical_energy_j += moved_j;
            advanced.heat_rejected_j -= moved_j;
        })
    }

    #[test]
    fn constants_are_locked() {
        assert_copy::<GeneratorMmrtgSeebeck>();
        assert_copy::<GeneratorMmrtgSeebeckState>();
        assert_copy::<GeneratorMmrtgSeebeckLoad>();
        assert_copy::<StepReject>();
        let (generator, mut state) = landing();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();

        // The registered forecast's primary constants, as its protocol prints
        // them: six significant figures, and no other digits.
        assert_eq!(OUTPUT_AMPLITUDE.to_bits(), 0.99401_f64.to_bits());
        assert_eq!(CONVERTER_FADING_SHARE.to_bits(), 0.230509_f64.to_bits());
        assert_eq!(CONVERTER_FADING_RATE_OVER_FUEL_DECAY.to_bits(), 18.777_f64.to_bits());
        // The fuel's decay is generator_mmrtg_f1's, and the protocol's ln 2 / 87.7 per year.
        assert_eq!(FUEL_DECAY_PER_S.to_bits(), generator_mmrtg_f1::FUEL_DECAY_PER_S.to_bits());
        assert_eq!(FUEL_DECAY_PER_S.to_bits(), 2.504504985818715e-10_f64.to_bits());
        assert!(within(FUEL_DECAY_PER_S * SECONDS_PER_YEAR, 0.007903616654047266, 1.0e-15));
        assert_eq!(
            CONVERTER_FADING_RATE_PER_S.to_bits(),
            4.702709011871801e-09_f64.to_bits()
        );
        assert_eq!(
            STABLE_OUTPUT_RATE_PER_S.to_bits(),
            5.00900997163743e-10_f64.to_bits()
        );
        assert_eq!(
            COMBINED_RATE_PER_S.to_bits(),
            5.203610009035544e-09_f64.to_bits()
        );
        assert_eq!(
            STABLE_EFFICIENCY.to_bits(),
            0.04519114981784456_f64.to_bits()
        );
        assert_eq!(
            FADING_EFFICIENCY.to_bits(),
            0.013537477050883677_f64.to_bits()
        );
        assert_eq!(MISSION_TIME_MAX_S.to_bits(), 353_116_800.0_f64.to_bits());
        assert_eq!(FITTING_SPAN_END_S.to_bits(), MISSION_TIME_MAX_S.to_bits());
        assert_eq!(SECONDS_PER_YEAR.to_bits(), 31_557_600.0_f64.to_bits());
        assert_eq!(
            THERMAL_POWER_AT_LANDING_W.to_bits(),
            generator_mmrtg_f1::THERMAL_POWER_AT_LANDING_W.to_bits()
        );
        assert_eq!(
            FUEL_HEAT_AT_LANDING_J.to_bits(),
            7798746702680.2705_f64.to_bits()
        );
        assert_eq!(
            FADING_YIELD_AT_LANDING_J.to_bits(),
            5081357005.97723_f64.to_bits()
        );
        // The two scales to day 4,087, from the closed form of the header
        // worked in 50-digit decimal arithmetic. The heat's is
        // generator_mmrtg_f1's, bit for bit.
        assert!(within(generator.card_heat_j, 660088991649.9416, 1.0e-12));
        assert_eq!(
            generator.card_heat_j.to_bits(),
            generator_mmrtg_f1::GeneratorMmrtgF1::curiosity().card_heat_j.to_bits()
        );
        assert!(within(
            generator.card_fading_yield_j,
            4272314451.325213,
            1.0e-12
        ));
        let gate_j = RESIDUAL_CONSERVATION_LIMIT * generator.card_heat_j;
        assert!(gate_j > 660_088.0 && gate_j < 660_090.0, "gate {gate_j}");
        assert_eq!(
            CONVERTER_ACCOUNT_LIMIT.to_bits(),
            RESIDUAL_CONSERVATION_LIMIT.to_bits()
        );

        let ceiling_s = generator.gate_dt_s().unwrap();
        assert_eq!(
            ceiling_s.to_bits(),
            (GATE_PHASE_STEP / COMBINED_RATE_PER_S).to_bits()
        );
        assert!(
            (ceiling_s - 3_843_485.6).abs() < 1.0,
            "ceiling {ceiling_s}"
        );

        let mut refuse = |tampered: GeneratorMmrtgSeebeck| {
            assert!(!tampered.is_valid());
            let got = step(&tampered, &mut state, &mut frame, NO_LOAD, DAY_S);
            assert_eq!(got, Err(StepReject::InvalidGenerator));
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            proof_intact(&frame);
            assert!(GeneratorMmrtgSeebeckState::try_landing(&tampered).is_none());
        };
        let mut tampered = generator;
        tampered.plutonium_238_half_life_years = 87.74;
        refuse(tampered);
        // The constants of the fit on days 0 to 2,060, the secondary of the
        // protocol, are not this card's.
        let mut tampered = generator;
        tampered.output_amplitude = 0.995915;
        refuse(tampered);
        let mut tampered = generator;
        tampered.converter_fading_share = 0.217256;
        refuse(tampered);
        let mut tampered = generator;
        tampered.converter_fading_rate_over_fuel_decay = 21.1586;
        refuse(tampered);
        let mut tampered = generator;
        tampered.thermal_power_at_landing_w = 2_000.0;
        refuse(tampered);
        let mut tampered = generator;
        tampered.fuel_decay_per_s = f64::from_bits(FUEL_DECAY_PER_S.to_bits() + 1);
        refuse(tampered);
        let mut tampered = generator;
        tampered.stable_output_rate_per_s = FUEL_DECAY_PER_S;
        refuse(tampered);
        let mut tampered = generator;
        tampered.card_heat_j *= 2.0;
        refuse(tampered);
        let mut tampered = generator;
        tampered.card_fading_yield_j *= 2.0;
        refuse(tampered);
        let mut tampered = generator;
        tampered.mission_time_max_s += DAY_S;
        refuse(tampered);
        let mut tampered = generator;
        tampered.fitting_span_end_s = generator_mmrtg_f1::FITTING_SPAN_END_S;
        refuse(tampered);
    }

    #[test]
    fn landing_state_is_the_fitted_amplitude_and_a_zero_ledger() {
        let (generator, state) = landing();
        assert_eq!(state.t_s, 0.0);
        assert_eq!(state.thermal_power_w.to_bits(), 1953.2_f64.to_bits());
        assert_eq!(state.converter_fading_fraction.to_bits(), 1.0_f64.to_bits());
        // 115.4 W × 0.99401 = 114.708754 W.
        assert!(within(state.electrical_power_w, 114.708754, 1.0e-12));
        assert!((100.0 * state.conversion_efficiency - 5.87286).abs() < 5.0e-6);
        assert!(within(
            state.electrical_power_w / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W,
            0.99401,
            1.0e-12
        ));
        assert_eq!(
            state.fuel_heat_j.to_bits(),
            FUEL_HEAT_AT_LANDING_J.to_bits()
        );
        assert_eq!(state.energy_anchor_j.to_bits(), state.fuel_heat_j.to_bits());
        assert_eq!(state.electrical_energy_j, 0.0);
        assert_eq!(state.heat_rejected_j, 0.0);
        assert_eq!(state.energy_defect_j, 0.0);
        assert_eq!(state.residual_conservation_error, 0.0);
        assert_eq!(
            state.converter_fading_yield_j.to_bits(),
            FADING_YIELD_AT_LANDING_J.to_bits()
        );
        assert_eq!(
            state.converter_account_anchor_j.to_bits(),
            state.converter_fading_yield_j.to_bits()
        );
        assert_eq!(state.fading_electrical_energy_j, 0.0);
        assert_eq!(state.converter_account_error, 0.0);
        assert_eq!(state.range_margin_s.to_bits(), MISSION_TIME_MAX_S.to_bits());
        assert!(!state.past_fitting_span);
        assert_eq!(
            GeneratorMmrtgSeebeckState::try_mission_time(&generator, 0.0).unwrap(),
            state
        );
    }

    #[test]
    fn daily_steps_match_the_closed_form_and_the_reference_table() {
        let (generator, mut state) = landing();
        let mut frame = blank_frame();
        let mut largest_residual = 0.0_f64;
        let mut largest_account_error = 0.0_f64;
        let mut reference_rows = 0;
        let mut day = 0u32;
        while day < 4_087 {
            step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
            day += 1;
            proof_intact(&frame);
            assert_eq!(state.t_s, f64::from(day) * DAY_S);
            // No day of the record is past the fitting span or the record.
            assert_eq!(frame.flags, 0, "day {day}");
            assert!(!state.past_fitting_span);
            let closed = closed_form_electrical_power_w(state.t_s);
            assert!(
                within(state.electrical_power_w, closed, 1.0e-12),
                "day {day}: {} against the closed form {closed}",
                state.electrical_power_w
            );
            largest_residual = largest_residual.max(state.residual_conservation_error);
            largest_account_error = largest_account_error.max(state.converter_account_error);
            let ratio = state.electrical_power_w / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W;

            let mut index = 0;
            while index < REFERENCE_DAYS.len() {
                let row = &REFERENCE_DAYS[index];
                if row.day == day {
                    reference_rows += 1;
                    assert!(within(state.thermal_power_w, row.thermal_power_w, 1.0e-12));
                    assert!(within(
                        state.electrical_power_w,
                        row.electrical_power_w,
                        1.0e-12
                    ));
                    assert!(within(ratio, row.ratio, 1.0e-12));
                    assert!(within(
                        state.converter_fading_fraction,
                        row.fading_fraction,
                        1.0e-12
                    ));
                    assert!(
                        within(state.electrical_energy_j, row.electrical_energy_j, 1.0e-12),
                        "day {day}: {}",
                        state.electrical_energy_j
                    );
                    assert!(
                        within(state.heat_rejected_j, row.heat_rejected_j, 1.0e-12),
                        "day {day}: {}",
                        state.heat_rejected_j
                    );
                }
                index += 1;
            }
        }
        assert_eq!(reference_rows, 8);
        assert_eq!(state.t_s.to_bits(), MISSION_TIME_MAX_S.to_bits());
        assert_eq!(state.range_margin_s, 0.0);
        assert!(largest_residual < 1.0e-12, "residual {largest_residual}");
        assert!(
            largest_account_error < 1.0e-12,
            "converter account {largest_account_error}"
        );
        assert_eq!(frame.t.to_bits(), MISSION_TIME_MAX_S.to_bits());
        assert_eq!(frame.pos[0], state.electrical_power_w as f32);
        assert_eq!(frame.pos[1], state.thermal_power_w as f32);
        assert_eq!(frame.pos[2], state.converter_fading_fraction as f32);
        assert_eq!(frame.vel[0], state.electrical_energy_j as f32);
        assert_eq!(frame.vel[1], state.heat_rejected_j as f32);
        assert_eq!(frame.vel[2], state.fuel_heat_j as f32);
        assert_eq!(frame.force_torque, 0.0);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        assert_eq!(frame.flags, 0);
    }

    /// The card is the forecast registered for the days after 4,087: a run
    /// of daily steps from landing to mission day 5,183 (2026-10-15) gives,
    /// on each day of the forecast's own table, its number to the table's
    /// twelve decimals, and the protocol's six; every frame after day 4,087
    /// carries both flags and none before it does.
    #[test]
    fn the_card_is_the_registered_forecast() {
        let (generator, mut state) = landing();
        let mut frame = blank_frame();
        let mut table_rows = 0;
        let mut protocol_rows = 0;
        let mut day = 0u32;
        while day < 5_183 {
            step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
            day += 1;
            if day > 4_087 {
                assert_eq!(frame.flags, FLAG_PAST_FITTING_SPAN | FLAG_PAST_RECORD, "day {day}");
                assert!(state.past_fitting_span);
                assert_eq!(state.range_margin_s, 0.0);
            } else {
                assert_eq!(frame.flags, 0, "day {day}");
            }
            assert!(state.residual_conservation_error < 1.0e-12);
            let ratio = state.electrical_power_w / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W;
            for (table_day, value) in FORECAST_TABLE {
                if table_day == day {
                    table_rows += 1;
                    assert!((ratio - value).abs() <= 1.0e-12, "day {day}: {ratio} against the table's {value}");
                }
            }
            for (table_day, value) in PROTOCOL_TABLE {
                if table_day == day {
                    protocol_rows += 1;
                    assert!((ratio - value).abs() <= 5.0e-7, "day {day}: {ratio} against the protocol's {value}");
                }
            }
        }
        assert_eq!(table_rows, 12);
        assert_eq!(protocol_rows, 4);
    }

    /// One day of the record. `measured` is the workbook's value for the
    /// day, the daily-average electrical power over 115.4 W: Whiting and
    /// Woerner, Historical RTG Performance Data - 2023 Update, Dryad, doi:
    /// 10.5061/dryad.1zcrjdfw2, CC0 1.0, workbook
    /// `All_RTGs_P_Over_P0_-_2023_Update_for_Release.xlsx`, sheet `MMRTG`,
    /// column C, row 18 plus the mission day. `difference` is this card's
    /// closed form less the measured value, in 64-bit arithmetic, as a
    /// fraction of 115.4 W.
    struct RecordDay {
        day: u32,
        measured: f64,
        difference: f64,
    }

    /// The nine days of the record generator_mmrtg_f1's test holds, taken by
    /// its rule and not by value: the first and the last day of each of the
    /// four spans the sealed comparison reports a mean over (2,809 to 3,173,
    /// 3,174 to 3,538, 3,539 to 3,903 and 3,904 to 4,087), and day 3,546,
    /// the day of its largest difference.
    const RECORD_DAYS: [RecordDay; 9] = [
        RecordDay { day: 2809, measured: 0.742320885457405, difference: -0.00018944574625545663 },
        RecordDay { day: 3173, measured: 0.7247239251739339, difference: -0.0029619566813932074 },
        RecordDay { day: 3174, measured: 0.7256935758675961, difference: -0.00398519356485072 },
        RecordDay { day: 3538, measured: 0.7020701369002196, difference: 0.0009152917612357259 },
        RecordDay { day: 3539, measured: 0.7019180915814247, difference: 0.0010179460984304223 },
        RecordDay { day: 3546, measured: 0.6865866381978059, difference: 0.016003960945916496 },
        RecordDay { day: 3903, measured: 0.6892134386613181, difference: -0.0035805174371849047 },
        RecordDay { day: 3904, measured: 0.6913202292998185, difference: -0.005733077414790344 },
        RecordDay { day: 4087, measured: 0.6820414695233954, difference: -0.004678596160849868 },
    ];

    /// After the fact, on days the constants were fitted to: on the nine
    /// days, a run of daily steps from landing differs from the record by
    /// the closed form's difference, within 1e-12 in 64 bits and within the
    /// frame's single precision in its frame. These days are not a test of
    /// the card: every one of them was in its fit. The largest is still day
    /// 3,546, +0.0160 of 115.4 W, 1.85 W, where generator_mmrtg_f1, compared
    /// blind, was +0.0225.
    #[test]
    fn the_record_days_read_after_the_fact_as_the_closed_form() {
        let (generator, mut state) = landing();
        let mut frame = blank_frame();
        let mut original = generator_mmrtg_f1::GeneratorState::try_landing(
            &generator_mmrtg_f1::GeneratorMmrtgF1::curiosity(),
        )
        .unwrap();
        let mut original_frame = blank_frame();
        let frame_rounding = 2.0_f64.powi(-18) / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W;
        let mut rows = 0;
        let mut largest = 0.0_f64;
        let mut largest_day = 0u32;
        let mut day = 0u32;
        while day < 4_087 {
            step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
            generator_mmrtg_f1::step(
                &generator_mmrtg_f1::GeneratorMmrtgF1::curiosity(),
                &mut original,
                &mut original_frame,
                generator_mmrtg_f1::GeneratorLoad {},
                DAY_S,
            )
            .unwrap();
            day += 1;
            for row in &RECORD_DAYS {
                if row.day == day {
                    rows += 1;
                    let difference = state.electrical_power_w
                        / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
                        - row.measured;
                    assert!(
                        (difference - row.difference).abs() <= 1.0e-12,
                        "day {day}: {difference} against {}",
                        row.difference
                    );
                    let from_frame = f64::from(frame.pos[0]) / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
                        - row.measured;
                    assert!((from_frame - row.difference).abs() <= frame_rounding);
                    // The original reads higher on every one of the nine days.
                    let blind = original.electrical_power_w / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
                        - row.measured;
                    assert!(blind > difference, "day {day}");
                    if difference.abs() > largest.abs() {
                        largest = difference;
                        largest_day = day;
                    }
                }
            }
        }
        assert_eq!(rows, 9);
        assert_eq!(largest_day, 3_546);
        assert!((largest - 0.016004).abs() < 5.0e-7, "largest {largest}");
        let largest_w = largest * ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W;
        assert!((largest_w - 1.847).abs() < 5.0e-4, "largest {largest_w} W");
    }

    /// The fuel, the heat and the clock are generator_mmrtg_f1's: a run of
    /// both cards in daily steps from landing to day 4,087 holds the same
    /// mission time, thermal power, decay heat still held and range margin,
    /// bit for bit, on every day, and each closes the same energy ledger. Only
    /// the electrical output, and so the split of the heat, differs: 0.20 W
    /// lower at landing and 1.15 W lower on day 4,087.
    #[test]
    fn the_fuel_and_the_clock_are_generator_mmrtg_f1s() {
        let (generator, mut state) = landing();
        let original_card = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
        let mut original = generator_mmrtg_f1::GeneratorState::try_landing(&original_card).unwrap();
        let first_gap_w = original.electrical_power_w - state.electrical_power_w;
        assert!((first_gap_w - 0.1994).abs() < 1.0e-4, "{first_gap_w}");
        let mut frame = blank_frame();
        let mut original_frame = blank_frame();
        for _ in 0..4_087 {
            step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
            generator_mmrtg_f1::step(&original_card, &mut original, &mut original_frame, generator_mmrtg_f1::GeneratorLoad {}, DAY_S)
                .unwrap();
            assert_eq!(state.t_s.to_bits(), original.t_s.to_bits());
            assert_eq!(state.clock_carry_s.to_bits(), original.clock_carry_s.to_bits());
            assert_eq!(state.thermal_power_w.to_bits(), original.thermal_power_w.to_bits());
            assert_eq!(state.fuel_heat_j.to_bits(), original.fuel_heat_j.to_bits());
            assert_eq!(state.range_margin_s.to_bits(), original.range_margin_s.to_bits());
            assert_eq!(frame.vel[2].to_bits(), original_frame.vel[2].to_bits());
            let released = state.electrical_energy_j + state.heat_rejected_j;
            let original_released = original.electrical_energy_j + original.heat_rejected_j;
            assert!(within(released, original_released, 1.0e-12));
        }
        let last_gap_w = original.electrical_power_w - state.electrical_power_w;
        assert!((last_gap_w - 1.1522).abs() < 1.0e-4, "{last_gap_w}");
    }

    /// The spacing of 64-bit numbers at the last day, 2^-24 s.
    fn spacing_at_the_end_s() -> f64 {
        f64::from_bits(MISSION_TIME_MAX_S.to_bits() + 1) - MISSION_TIME_MAX_S
    }

    /// One step, and whether its frame reports [`FLAG_PAST_RECORD`]; when it
    /// does, it reports [`FLAG_PAST_FITTING_SPAN`] too, and when it does
    /// not, neither.
    fn past_record_step(
        generator: &GeneratorMmrtgSeebeck,
        state: &mut GeneratorMmrtgSeebeckState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<bool, StepReject> {
        step(generator, state, frame, NO_LOAD, dt_s)?;
        let past = frame.flags & FLAG_PAST_RECORD != 0;
        assert_eq!(frame.flags & FLAG_PAST_FITTING_SPAN != 0, past, "the two flags part at {}", state.t_s);
        assert_eq!(state.past_fitting_span, past);
        Ok(past)
    }

    /// A decimal with three places, from thousandths.
    fn milli(thousandths: u64) -> f64 {
        std::format!("{}.{:03}", thousandths / 1_000, thousandths % 1_000)
            .parse()
            .unwrap()
    }

    /// The tolerance is four spacings of the clock at the last day, one
    /// number for every step, and a recorded constant: generator_mmrtg_f1's.
    #[test]
    fn the_mission_end_tolerance_is_four_spacings_at_the_last_day() {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let spacing_s = spacing_at_the_end_s();
        assert!(MISSION_TIME_MAX_S > 2.0_f64.powi(28) && MISSION_TIME_MAX_S < 2.0_f64.powi(29));
        assert_eq!(spacing_s, 2.0_f64.powi(-24));
        let before_s = 2.0_f64.powi(28);
        assert_eq!(before_s - f64::from_bits(before_s.to_bits() - 1), 2.0_f64.powi(-25));
        assert_eq!(MISSION_END_TOLERANCE_S, 4.0 * spacing_s);
        assert_eq!(MISSION_END_TOLERANCE_S, 2.0_f64.powi(-22));
        assert_eq!(MISSION_END_TOLERANCE_S, generator_mmrtg_f1::MISSION_END_TOLERANCE_S);
        assert_eq!(MISSION_TIME_MAX_S, generator_mmrtg_f1::MISSION_TIME_MAX_S);
        assert_eq!(generator.mission_end_tolerance_s, MISSION_END_TOLERANCE_S);
        assert_eq!(
            (MISSION_TIME_MAX_S + MISSION_END_TOLERANCE_S).to_bits(),
            MISSION_TIME_MAX_S.to_bits() + 4
        );
        let mut altered = generator;
        altered.mission_end_tolerance_s = 0.9;
        assert!(!altered.is_valid());
    }

    /// The compensated clock keeps within a spacing of the exact sum of a
    /// long run, where a plain running sum drifts by many.
    #[test]
    fn the_compensated_clock_does_not_drift() {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let floor_s = generator.floor_dt_s().unwrap();
        let start_s = 4_085.0 * DAY_S;
        let mut state = GeneratorMmrtgSeebeckState::try_mission_time(&generator, start_s).unwrap();
        let mut frame = blank_frame();
        let mut plain_s = start_s;
        let count = 40_000u32;
        for _ in 0..count {
            step(&generator, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
            plain_s += floor_s;
            assert!(state.clock_carry_s.abs() <= 0.5 * spacing_at_the_end_s());
        }
        let exact_s = start_s + f64::from(count) * floor_s;
        let spacing_s = f64::from_bits(exact_s.to_bits() + 1) - exact_s;
        assert!((state.t_s - exact_s).abs() <= spacing_s, "{} against {exact_s}", state.t_s);
        assert!((plain_s - exact_s).abs() > 16.0 * spacing_s, "the plain sum {plain_s}");
        assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
    }

    /// Runs that end on mission day 4,087 in decimal land every step with
    /// neither flag at twelve step sizes and three lengths, and the step
    /// after lands with both. A restart from the state before the last step
    /// lands that step too, and a single step from the decimal start one
    /// step before the end lands. Three of the runs start before 2^28 s,
    /// where the clock's spacing is half the spacing at the last day.
    #[test]
    fn a_decimal_run_to_the_last_day_lands_every_step() {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let mut started_before_2_28_s = 0;
        let decimal = |tenths: u64| -> f64 {
            std::format!("{}.{}", tenths / 10, tenths % 10).parse().unwrap()
        };
        // step sizes in units of 0.1 s; the first is under the card's ceiling
        let steps_e1: [u64; 12] = [
            38_434_855, 10_000_001, 864_000, 864_005, 36_001, 10_003, 6_007, 1_001, 606, 101, 83,
            37,
        ];
        for dt_e1 in steps_e1 {
            let dt_s = decimal(dt_e1);
            for count in [10_u64, 1_000, 50_000] {
                if count * dt_e1 > 3_531_168_000 {
                    continue;
                }
                let start_e1 = 3_531_168_000 - count * dt_e1;
                if decimal(start_e1) < 2.0_f64.powi(28) {
                    started_before_2_28_s += 1;
                }
                let mut state =
                    GeneratorMmrtgSeebeckState::try_mission_time(&generator, decimal(start_e1)).unwrap();
                let mut frame = blank_frame();
                let mut before_last = state;
                for landed in 0..count {
                    before_last = state;
                    assert_eq!(
                        past_record_step(&generator, &mut state, &mut frame, dt_s),
                        Ok(false),
                        "dt {dt_s}: step {} of {count}",
                        landed + 1
                    );
                }
                assert!((state.t_s - MISSION_TIME_MAX_S).abs() <= MISSION_END_TOLERANCE_S);
                if state.t_s >= MISSION_TIME_MAX_S {
                    assert_eq!(state.range_margin_s, 0.0);
                } else {
                    assert_eq!(state.range_margin_s, MISSION_TIME_MAX_S - state.t_s);
                }
                let mut past = state;
                assert_eq!(past_record_step(&generator, &mut past, &mut frame, dt_s), Ok(true));
                assert_eq!(past.range_margin_s, 0.0);

                let mut restart =
                    GeneratorMmrtgSeebeckState::try_mission_time(&generator, before_last.t_s).unwrap();
                assert_eq!(
                    past_record_step(&generator, &mut restart, &mut frame, dt_s),
                    Ok(false),
                    "restart from {} by {dt_s}",
                    before_last.t_s
                );
                assert!((restart.t_s - state.t_s).abs() <= MISSION_END_TOLERANCE_S);
            }
            if dt_e1 <= 3_531_168_000 {
                let start_s = decimal(3_531_168_000 - dt_e1);
                let mut single = GeneratorMmrtgSeebeckState::try_mission_time(&generator, start_s).unwrap();
                let mut frame = blank_frame();
                assert_eq!(
                    past_record_step(&generator, &mut single, &mut frame, dt_s),
                    Ok(false),
                    "single step from {start_s} by {dt_s}"
                );
            }
        }
        assert_eq!(started_before_2_28_s, 3);
    }

    /// The run of ten steps of 3,843,485.5 s from 314,681,945 s and the
    /// restart from its ninth state both land on the last day with neither
    /// flag. Two steps at the floor whose end in decimal is 0.89 s past the
    /// last day, and one step to the same overshoot, land on the same clock
    /// with both flags.
    #[test]
    fn a_restart_and_the_run_straight_through_get_the_same_verdict() {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let mut frame = blank_frame();
        let dt_s = 3_843_485.5;
        assert!(dt_s < generator.gate_dt_s().unwrap());
        let mut straight = GeneratorMmrtgSeebeckState::try_mission_time(&generator, 314_681_945.0).unwrap();
        for _ in 0..9 {
            assert_eq!(past_record_step(&generator, &mut straight, &mut frame, dt_s), Ok(false));
        }
        let mut restart = GeneratorMmrtgSeebeckState::try_mission_time(&generator, straight.t_s).unwrap();
        assert_eq!(past_record_step(&generator, &mut straight, &mut frame, dt_s), Ok(false));
        assert_eq!(past_record_step(&generator, &mut restart, &mut frame, dt_s), Ok(false));
        assert!((straight.t_s - MISSION_TIME_MAX_S).abs() <= MISSION_END_TOLERANCE_S);
        assert!((restart.t_s - MISSION_TIME_MAX_S).abs() <= MISSION_END_TOLERANCE_S);
        let mut single = GeneratorMmrtgSeebeckState::try_mission_time(&generator, 349_273_314.5).unwrap();
        assert_eq!(past_record_step(&generator, &mut single, &mut frame, dt_s), Ok(false));
        assert_eq!(single.t_s.to_bits(), MISSION_TIME_MAX_S.to_bits());

        let floor_s = generator.floor_dt_s().unwrap();
        assert_eq!(floor_s, 3.6654334769526953);
        let mut two = GeneratorMmrtgSeebeckState::try_mission_time(&generator, 353_116_793.559_133_05).unwrap();
        assert_eq!(past_record_step(&generator, &mut two, &mut frame, floor_s), Ok(false));
        assert_eq!(past_record_step(&generator, &mut two, &mut frame, floor_s), Ok(true));
        let mut one = GeneratorMmrtgSeebeckState::try_mission_time(&generator, 353_116_797.224_566_52).unwrap();
        assert_eq!(past_record_step(&generator, &mut one, &mut frame, floor_s), Ok(true));
        assert_eq!(one.t_s.to_bits(), two.t_s.to_bits());
    }

    /// Requests whose end in decimal is exactly the last day land with
    /// neither flag whether they are one step or five, and a restart from
    /// each returned state lands the rest the same; the same requests a
    /// microsecond later land their last step with both flags by every
    /// route. Starts and steps to the thousandth of a second from a fixed
    /// generator of numbers, between the card's floor and its ceiling.
    #[test]
    fn one_step_and_many_to_the_same_end_agree() {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let end_e3: u64 = 353_116_800_000;
        let mut seed: u64 = 0x4087_0806_2012_0002;
        let mut next = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            seed >> 11
        };
        let floor_e3 = 3_666u64;
        let ceiling_e3 = 3_843_485_000u64;
        assert!(milli(floor_e3) >= generator.floor_dt_s().unwrap());
        assert!(milli(ceiling_e3) <= generator.gate_dt_s().unwrap());
        let mut cases = 0;
        for _ in 0..4_000 {
            let dt_e3 = floor_e3 + next() % (ceiling_e3 - floor_e3);
            let count = 1 + next() % 5;
            for (late_e3, lands) in [(0u64, true), (1u64, false)] {
                let start_e3 = end_e3 - count * dt_e3;
                let start_s = milli(start_e3) + if late_e3 == 1 { 1.0e-6 } else { 0.0 };
                let dt_s = milli(dt_e3);
                let mut straight = GeneratorMmrtgSeebeckState::try_mission_time(&generator, start_s).unwrap();
                let mut frame = blank_frame();
                for taken in 0..count {
                    let mut restart =
                        GeneratorMmrtgSeebeckState::try_mission_time(&generator, straight.t_s).unwrap();
                    let mut restart_result = Ok(false);
                    for _ in taken..count {
                        restart_result = past_record_step(&generator, &mut restart, &mut frame, dt_s);
                        if restart_result.is_err() {
                            break;
                        }
                    }
                    let got = past_record_step(&generator, &mut straight, &mut frame, dt_s);
                    if taken + 1 < count {
                        assert_eq!(got, Ok(false));
                    } else {
                        assert_eq!(got, Ok(!lands), "{count} of {dt_s} from {start_s}");
                    }
                    assert_eq!(
                        restart_result,
                        Ok(!lands),
                        "restart after {taken} of {count} of {dt_s} from {start_s}"
                    );
                }
                cases += 1;
            }
        }
        assert_eq!(cases, 8_000);
    }

    /// A step that would end later than the last day of the record by more
    /// than the tolerance lands with both flags; every end inside it lands
    /// at its own value with neither. The same for a step of 3.8e6 s, a day,
    /// an hour and the floor.
    #[test]
    fn a_step_beyond_the_mission_end_tolerance_lands_past_the_record() {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let spacing_s = spacing_at_the_end_s();
        let floor_s = generator.floor_dt_s().unwrap();
        for dt_s in [3_843_485.5, DAY_S, 3_600.1, floor_s] {
            let mut landed_past_the_end = 0;
            let mut flagged = 0;
            for spacings in 0..9u64 {
                let end_s = f64::from_bits(MISSION_TIME_MAX_S.to_bits() + spacings);
                let mut t_s = end_s - dt_s;
                while t_s + dt_s < end_s {
                    t_s = f64::from_bits(t_s.to_bits() + 1);
                }
                while t_s + dt_s > end_s {
                    t_s = f64::from_bits(t_s.to_bits() - 1);
                }
                if t_s + dt_s != end_s || advance_clock(t_s, 0.0, dt_s).0 != end_s {
                    continue;
                }
                let mut state = GeneratorMmrtgSeebeckState::try_mission_time(&generator, t_s).unwrap();
                let mut frame = blank_frame();
                let result = past_record_step(&generator, &mut state, &mut frame, dt_s);
                assert_eq!(state.t_s, end_s);
                if spacings <= 4 {
                    assert_eq!(result, Ok(false), "{spacings} spacings past by {dt_s}");
                    if spacings > 0 {
                        landed_past_the_end += 1;
                        assert_eq!(state.range_margin_s, 0.0);
                    }
                } else {
                    assert_eq!(result, Ok(true));
                    assert_eq!(state.range_margin_s, 0.0);
                    flagged += 1;
                }
            }
            assert!(landed_past_the_end >= 1, "dt {dt_s}");
            assert!(flagged >= 1, "dt {dt_s}");
            assert_eq!(MISSION_END_TOLERANCE_S, 4.0 * spacing_s);
            let start_s = MISSION_TIME_MAX_S + 0.1 - dt_s;
            let mut state = GeneratorMmrtgSeebeckState::try_mission_time(&generator, start_s).unwrap();
            let mut frame = blank_frame();
            assert_eq!(past_record_step(&generator, &mut state, &mut frame, dt_s), Ok(true));
        }
    }

    #[test]
    fn one_step_equals_many() {
        let (generator, mut daily) = landing();
        let mut frame = blank_frame();
        let mut day = 0;
        while day < 4_087 {
            step(&generator, &mut daily, &mut frame, NO_LOAD, DAY_S).unwrap();
            day += 1;
        }
        let mut once = GeneratorMmrtgSeebeckState::try_landing(&generator).unwrap();
        assert!(MISSION_TIME_MAX_S > generator.gate_dt_s().unwrap());
        step(
            &generator,
            &mut once,
            &mut frame,
            NO_LOAD,
            MISSION_TIME_MAX_S,
        )
        .unwrap();
        proof_intact(&frame);
        assert_eq!(once.t_s.to_bits(), daily.t_s.to_bits());
        assert!((once.thermal_power_w - daily.thermal_power_w).abs() < 1.0e-9);
        assert!((once.electrical_power_w - daily.electrical_power_w).abs() < 1.0e-9);
        assert!(within(
            once.electrical_energy_j,
            daily.electrical_energy_j,
            1.0e-12
        ));
        assert!(within(once.heat_rejected_j, daily.heat_rejected_j, 1.0e-12));
        assert!(once.residual_conservation_error < 1.0e-12);
        assert!(once.converter_account_error < 1.0e-12);
        // The card at day 4,087, from the closed form of the header worked in
        // 50-digit decimal arithmetic.
        assert!(within(once.thermal_power_w, 1787.8803829328673, 1.0e-12));
        assert!(within(once.electrical_power_w, 78.16767558603774, 1.0e-12));
        assert!(within(
            once.electrical_energy_j,
            32840075853.927123,
            1.0e-12
        ));
        assert!(within(once.heat_rejected_j, 627248915796.01447, 1.0e-12));
        assert!(within(once.fading_electrical_energy_j, generator.card_fading_yield_j, 1.0e-12));

        // About one day from mission day 4,085, as steps at the card's floor
        // against one step of the same length. This is the finest comparison
        // the floor allows: 23,571 steps of 3.67 s.
        let floor_s = generator.floor_dt_s().unwrap();
        let count = 23_571u32;
        let span_s = f64::from(count) * floor_s;
        assert!(span_s > 86_390.0 && span_s < DAY_S, "span {span_s}");
        let start_s = 4_085.0 * DAY_S;
        let mut small = GeneratorMmrtgSeebeckState::try_mission_time(&generator, start_s).unwrap();
        let mut big = small;
        let mut largest_residual = 0.0_f64;
        let mut taken = 0u32;
        while taken < count {
            step(&generator, &mut small, &mut frame, NO_LOAD, floor_s).unwrap();
            largest_residual = largest_residual.max(small.residual_conservation_error);
            taken += 1;
        }
        step(&generator, &mut big, &mut frame, NO_LOAD, span_s).unwrap();
        proof_intact(&frame);
        assert!((small.electrical_power_w - big.electrical_power_w).abs() < 1.0e-8);
        assert!((small.thermal_power_w - big.thermal_power_w).abs() < 1.0e-8);
        assert!(within(
            small.electrical_energy_j,
            big.electrical_energy_j,
            1.0e-10
        ));
        assert!(largest_residual < 1.0e-9, "residual {largest_residual}");
        assert!((small.t_s - big.t_s).abs() < 0.01);
        let closed = closed_form_electrical_power_w(big.t_s);
        assert!(within(big.electrical_power_w, closed, 1.0e-12));
        assert!((small.electrical_power_w - closed).abs() < 1.0e-8);
    }

    #[test]
    fn a_straight_line_fuel_step_is_refused() {
        let (generator, mut state) = landing();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let year_s = SECONDS_PER_YEAR;

        // The defect the refused trial carries, worked by hand.
        let exact = exact_map(&generator, state.thermal_power_w, 1.0, year_s).unwrap();
        let wrong_power_w =
            state.thermal_power_w - state.thermal_power_w * FUEL_DECAY_PER_S * year_s;
        let defect_j = wrong_power_w / FUEL_DECAY_PER_S - state.fuel_heat_j
            + exact.stable_electrical_energy_j
            + exact.fading_electrical_energy_j
            + exact.heat_rejected_j;
        assert!(
            defect_j < -2.42e8 && defect_j > -2.44e8,
            "defect {defect_j}"
        );
        assert!(-defect_j > 300.0 * RESIDUAL_CONSERVATION_LIMIT * generator.card_heat_j);

        let got = step_straight_line_fuel(&generator, &mut state, &mut frame, year_s);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        // The same wrong step at one day lands while the defect grows, then is refused.
        let mut previous = 0.0;
        let mut landed = 0u32;
        loop {
            let held_state = state_bits(&state);
            let held_frame = frame.to_bytes();
            match step_straight_line_fuel(&generator, &mut state, &mut frame, DAY_S) {
                Ok(()) => {
                    landed += 1;
                    assert!(
                        state.energy_defect_j < previous,
                        "step {landed} defect {}",
                        state.energy_defect_j
                    );
                    previous = state.energy_defect_j;
                    assert!(landed < 410, "the straight line did not refuse");
                }
                Err(StepReject::FirstLawBreach) => {
                    assert!(landed > 310, "refused after only {landed} steps");
                    assert_eq!(state_bits(&state), held_state);
                    assert_eq!(frame.to_bytes(), held_frame);
                    proof_intact(&frame);
                    break;
                }
                Err(other) => panic!("the straight line refused with {other:?} after {landed}"),
            }
        }
        let growth_j = -previous / f64::from(landed);
        assert!(
            growth_j > 1_700.0 && growth_j < 1_900.0,
            "growth {growth_j}"
        );
    }

    #[test]
    fn a_straight_line_fading_step_is_refused_by_the_converter_account() {
        let (generator, mut state) = landing();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let year_s = SECONDS_PER_YEAR;

        // What the refused trial carries, worked by hand: the energy ledger
        // is inside its gate, the account is not, and the electrical power is
        // 0.27 W under the closed form.
        let exact = exact_map(&generator, state.thermal_power_w, 1.0, year_s).unwrap();
        let wrong_fraction = 1.0 - CONVERTER_FADING_RATE_PER_S * year_s;
        let energy_defect_j = exact.thermal_power_w / FUEL_DECAY_PER_S - state.fuel_heat_j
            + exact.stable_electrical_energy_j
            + exact.fading_electrical_energy_j
            + exact.heat_rejected_j;
        assert!(energy_defect_j.abs() / generator.card_heat_j < 1.0e-12);
        let account_defect_j = generator
            .converter_fading_yield_j(exact.thermal_power_w, wrong_fraction)
            - state.converter_fading_yield_j
            + exact.fading_electrical_energy_j;
        let account_error = account_defect_j.abs() / generator.card_fading_yield_j;
        assert!(
            account_error > 1.2e-2 && account_error < 1.25e-2,
            "account {account_error}"
        );
        assert!(account_error > 10_000.0 * CONVERTER_ACCOUNT_LIMIT);
        let drop = exact.thermal_power_w / THERMAL_POWER_AT_LANDING_W;
        let wrong_power_w =
            exact.thermal_power_w * drop * (STABLE_EFFICIENCY + FADING_EFFICIENCY * wrong_fraction);
        let shortfall_w = closed_form_electrical_power_w(year_s) - wrong_power_w;
        assert!(
            shortfall_w > 0.27 && shortfall_w < 0.28,
            "shortfall {shortfall_w}"
        );

        let got = step_straight_line_fading(&generator, &mut state, &mut frame, year_s);
        assert_eq!(got, Err(StepReject::ConverterAccountBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        // Electrical energy moved from the stable part to the fading part:
        // the energy ledger cannot see it, the account can.
        let moved_j = 1.0e-3 * generator.card_fading_yield_j;
        let got = step_bias_fading_part(&generator, &mut state, &mut frame, year_s, moved_j);
        assert_eq!(got, Err(StepReject::ConverterAccountBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        let got = step_bias_fading_part(&generator, &mut state, &mut frame, year_s, -moved_j);
        assert_eq!(got, Err(StepReject::ConverterAccountBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        // The honest step from the same state lands with both ledgers near zero.
        step(&generator, &mut state, &mut frame, NO_LOAD, year_s).unwrap();
        assert!(state.residual_conservation_error < 1.0e-12);
        assert!(state.converter_account_error < 1.0e-12);
    }

    /// The Seebeck form's own attack: the stable part's energy booked at the
    /// fuel's rate, as a converter whose temperature drop did not follow the
    /// heat, books more electricity than the fuel releases for it and is
    /// refused over a year.
    #[test]
    fn the_fuel_counted_once_in_the_output_is_refused() {
        let (generator, mut state) = landing();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let got = step_single_decay(&generator, &mut state, &mut frame, SECONDS_PER_YEAR);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);
    }

    #[test]
    fn a_biased_ledger_is_refused_and_rolls_back() {
        let (generator, mut state) = landing();
        let mut frame = blank_frame();
        let year_s = SECONDS_PER_YEAR;
        step(&generator, &mut state, &mut frame, NO_LOAD, year_s).unwrap();
        let held = state_bits(&state);
        let held_frame = frame.to_bytes();
        let extra_j = 1.0e-3 * generator.card_heat_j;

        let got = step_bias_electrical_energy(&generator, &mut state, &mut frame, year_s, extra_j);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        let got = step_bias_heat_rejected(&generator, &mut state, &mut frame, year_s, -extra_j);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        let got = step_bias_electrical_energy(&generator, &mut state, &mut frame, year_s, -extra_j);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);

        let got = step_bias_heat_rejected(&generator, &mut state, &mut frame, year_s, extra_j);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), held);
        assert_eq!(frame.to_bytes(), held_frame);
        proof_intact(&frame);
    }

    #[test]
    fn rectangle_rule_electrical_energy_is_refused() {
        let (generator, mut state) = landing();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let got = step_rectangle(&generator, &mut state, &mut frame, SECONDS_PER_YEAR);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);
    }

    #[test]
    fn edges_of_the_card_are_refused_by_name() {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let mut frame = blank_frame();
        let held = frame.to_bytes();

        let mut refuse = |start_s: f64, dt_s: f64, expect: StepReject| {
            let mut state = GeneratorMmrtgSeebeckState::try_mission_time(&generator, start_s).unwrap();
            let saved = state_bits(&state);
            let got = step(&generator, &mut state, &mut frame, NO_LOAD, dt_s);
            assert_eq!(got, Err(expect), "from {start_s} s by {dt_s} s");
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            proof_intact(&frame);
        };
        let ledger_end_s = generator.ledger_end_mission_time_s().unwrap();
        refuse(0.0, 2.0 * ledger_end_s, StepReject::DecayHeatBelowLedgerResolution);
        refuse(0.0, f64::MAX, StepReject::DecayHeatBelowLedgerResolution);
        refuse(0.0, 0.0, StepReject::NonPositiveDt);
        refuse(0.0, -1.0, StepReject::NonPositiveDt);
        refuse(0.0, f64::NAN, StepReject::NonFiniteInput);
        refuse(0.0, f64::INFINITY, StepReject::NonFiniteInput);
        refuse(0.0, f64::NEG_INFINITY, StepReject::NonFiniteInput);

        let mut state = GeneratorMmrtgSeebeckState::try_landing(&generator).unwrap();
        state.thermal_power_w = f64::NAN;
        let nan_bits = state_bits(&state);
        let got = step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S);
        assert_eq!(got, Err(StepReject::NonFiniteState));
        assert_eq!(state_bits(&state), nan_bits);
        assert_eq!(frame.to_bytes(), held);

        let mut state = GeneratorMmrtgSeebeckState::try_landing(&generator).unwrap();
        state.t_s = -1.0;
        let early_bits = state_bits(&state);
        let got = step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S);
        assert_eq!(got, Err(StepReject::MissionTimeBeforeLanding));
        assert_eq!(state_bits(&state), early_bits);
        assert_eq!(frame.to_bytes(), held);

        let mut state = GeneratorMmrtgSeebeckState::try_mission_time(&generator, MISSION_TIME_MAX_S).unwrap();
        state.fuel_heat_j = 1.0;
        let late_bits = state_bits(&state);
        let got = step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S);
        assert_eq!(got, Err(StepReject::DecayHeatBelowLedgerResolution));
        assert_eq!(state_bits(&state), late_bits);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        assert!(GeneratorMmrtgSeebeckState::try_mission_time(&generator, -1.0).is_none());
        assert!(GeneratorMmrtgSeebeckState::try_mission_time(&generator, 353_116_800.1).is_some());
        assert!(GeneratorMmrtgSeebeckState::try_mission_time(&generator, 1.01 * ledger_end_s).is_none());
        assert!(GeneratorMmrtgSeebeckState::try_mission_time(&generator, f64::NAN).is_none());
        assert!(GeneratorMmrtgSeebeckState::try_mission_time(&generator, f64::INFINITY).is_none());
        assert!(GeneratorMmrtgSeebeckState::try_mission_time(&generator, MISSION_TIME_MAX_S).is_some());

        let mut last = GeneratorMmrtgSeebeckState::try_mission_time(&generator, 4_086.0 * DAY_S).unwrap();
        step(&generator, &mut last, &mut frame, NO_LOAD, DAY_S).unwrap();
        assert_eq!(last.t_s.to_bits(), 353_116_800.0_f64.to_bits());
        assert_eq!(last.range_margin_s, 0.0);
        assert_eq!(frame.t.to_bits(), 353_116_800.0_f64.to_bits());
        assert_eq!(frame.flags, 0);
        proof_intact(&frame);
    }

    /// The two flags are set together, on the step past the last day of the
    /// record and of the fit, and are only flags: the step lands and closes
    /// its ledgers.
    #[test]
    fn the_flags_are_only_flags() {
        let (generator, mut state) = landing();
        let mut frame = blank_frame();
        let mut day = 0;
        while day < 4_087 {
            step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
            day += 1;
        }
        assert_eq!(state.t_s.to_bits(), FITTING_SPAN_END_S.to_bits());
        assert!(!state.past_fitting_span);
        assert_eq!(frame.flags, 0);
        let at_the_end = GeneratorMmrtgSeebeckState::try_mission_time(&generator, FITTING_SPAN_END_S).unwrap();
        assert!(!at_the_end.past_fitting_span);

        step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
        assert!(state.past_fitting_span);
        assert_eq!(frame.flags, FLAG_PAST_FITTING_SPAN | FLAG_PAST_RECORD);
        assert!(state.residual_conservation_error < 1.0e-12);
        proof_intact(&frame);

        let mut just_past = at_the_end;
        let floor_s = generator.floor_dt_s().unwrap();
        step(&generator, &mut just_past, &mut frame, NO_LOAD, floor_s).unwrap();
        assert!(just_past.past_fitting_span);
        assert_eq!(frame.flags, FLAG_PAST_FITTING_SPAN | FLAG_PAST_RECORD);
        // generator_mmrtg_f1's own flag of its fitting span, day 2,060, is not
        // this card's: day 2,061 carries no flag.
        let mut day_2061 = GeneratorMmrtgSeebeckState::try_mission_time(&generator, 2_060.0 * DAY_S).unwrap();
        step(&generator, &mut day_2061, &mut frame, NO_LOAD, DAY_S).unwrap();
        assert_eq!(frame.flags, 0);
    }

    #[test]
    fn starting_partway_matches_stepping_from_landing() {
        let (generator, mut from_landing) = landing();
        let mut frame = blank_frame();
        let mut day = 0;
        while day < 2_062 {
            step(&generator, &mut from_landing, &mut frame, NO_LOAD, DAY_S).unwrap();
            day += 1;
        }
        let mut partway = GeneratorMmrtgSeebeckState::try_mission_time(&generator, 2_061.0 * DAY_S).unwrap();
        assert_eq!(partway.electrical_energy_j, 0.0);
        assert_eq!(partway.heat_rejected_j, 0.0);
        assert_eq!(partway.fading_electrical_energy_j, 0.0);
        assert_eq!(partway.energy_defect_j, 0.0);
        assert_eq!(partway.residual_conservation_error, 0.0);
        assert_eq!(partway.converter_account_defect_j, 0.0);
        assert_eq!(partway.converter_account_error, 0.0);
        assert_eq!(
            partway.energy_anchor_j.to_bits(),
            partway.fuel_heat_j.to_bits()
        );
        assert_eq!(
            partway.converter_account_anchor_j.to_bits(),
            partway.converter_fading_yield_j.to_bits()
        );
        assert!(!partway.past_fitting_span);
        // Day 2,061, the reference row above.
        assert!(within(partway.thermal_power_w, 1868.0054040123469, 1.0e-12));
        assert!(within(
            partway.electrical_power_w,
            91.20319154609599,
            1.0e-12
        ));

        step(&generator, &mut partway, &mut frame, NO_LOAD, DAY_S).unwrap();
        proof_intact(&frame);
        assert_eq!(partway.t_s.to_bits(), from_landing.t_s.to_bits());
        assert!(within(
            partway.thermal_power_w,
            from_landing.thermal_power_w,
            1.0e-12
        ));
        assert!(within(
            partway.electrical_power_w,
            from_landing.electrical_power_w,
            1.0e-12
        ));
        assert!(partway.residual_conservation_error < 1.0e-12);
        assert!(partway.converter_account_error < 1.0e-12);
        assert!(partway.electrical_energy_j < from_landing.electrical_energy_j);
    }

    #[test]
    fn a_wrong_split_between_electricity_and_rejected_heat_lands() {
        // Stated limit of the card: the energy ledger fixes the total that
        // leaves the fuel and not its division, and the converter account
        // covers the fading part only. One percent more electricity and that
        // much less rejected heat conserves energy and lands.
        let (generator, mut honest) = landing();
        let mut wrong = honest;
        let mut frame = blank_frame();
        let year_s = SECONDS_PER_YEAR;
        step(&generator, &mut honest, &mut frame, NO_LOAD, year_s).unwrap();
        step_wrong_split(&generator, &mut wrong, &mut frame, year_s, 0.01).unwrap();
        proof_intact(&frame);
        assert!(wrong.residual_conservation_error < 1.0e-12);
        assert!(wrong.converter_account_error < 1.0e-12);
        assert!(within(
            wrong.electrical_energy_j,
            1.01 * honest.electrical_energy_j,
            1.0e-12
        ));
        assert_eq!(
            wrong.electrical_power_w.to_bits(),
            honest.electrical_power_w.to_bits()
        );
    }

    #[test]
    fn the_proof_is_not_the_cards() {
        let (generator, mut state) = landing();
        let mut frame = blank_frame();
        let lengths = [0.1, 1.0, DAY_S, SECONDS_PER_YEAR, 3_843_485.0];
        let mut index = 0;
        while index < lengths.len() {
            step(&generator, &mut state, &mut frame, NO_LOAD, lengths[index]).unwrap();
            proof_intact(&frame);
            index += 1;
        }
        let got = step(&generator, &mut state, &mut frame, NO_LOAD, 0.0);
        assert_eq!(got, Err(StepReject::NonPositiveDt));
        proof_intact(&frame);
        let got = step(&generator, &mut state, &mut frame, NO_LOAD, f64::NAN);
        assert_eq!(got, Err(StepReject::NonFiniteInput));
        proof_intact(&frame);
        let got = step(
            &generator,
            &mut state,
            &mut frame,
            NO_LOAD,
            -MISSION_TIME_MAX_S,
        );
        assert_eq!(got, Err(StepReject::NonPositiveDt));
        proof_intact(&frame);
        let got = step_straight_line_fuel(&generator, &mut state, &mut frame, SECONDS_PER_YEAR);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        proof_intact(&frame);
        let got = step_straight_line_fading(&generator, &mut state, &mut frame, SECONDS_PER_YEAR);
        assert_eq!(got, Err(StepReject::ConverterAccountBreach));
        proof_intact(&frame);
        let mut tampered = generator;
        tampered.output_amplitude = 1.0;
        let got = step(&tampered, &mut state, &mut frame, NO_LOAD, DAY_S);
        assert_eq!(got, Err(StepReject::InvalidGenerator));
        proof_intact(&frame);
    }

    #[test]
    fn a_step_at_the_floor_moves_the_clock_and_every_integrated_quantity() {
        let card = GeneratorMmrtgSeebeck::curiosity();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );
        assert!(floor_s > 3.66 && floor_s < 3.67, "floor {floor_s}");

        let starts = [0.0, MISSION_TIME_MAX_S - DAY_S];
        let mut index = 0;
        while index < starts.len() {
            let mut state = GeneratorMmrtgSeebeckState::try_mission_time(&card, starts[index]).unwrap();
            let before = state;
            let mut frame = blank_frame();
            step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
            let moved_s = state.t_s - before.t_s;
            assert!(
                (moved_s - floor_s).abs() <= 1.0e-8 * floor_s,
                "the clock moved {moved_s} for a step of {floor_s}"
            );
            assert!(state.thermal_power_w != before.thermal_power_w);
            assert!(state.converter_fading_fraction != before.converter_fading_fraction);
            assert!(state.electrical_power_w != before.electrical_power_w);
            assert!(state.electrical_energy_j != before.electrical_energy_j);
            assert!(state.heat_rejected_j != before.heat_rejected_j);
            assert!(state.fading_electrical_energy_j != before.fading_electrical_energy_j);
            assert!(state.residual_conservation_error < 1.0e-12);
            assert!(state.converter_account_error < 1.0e-12);

            let mut taken = 0;
            while taken < 1_000 {
                step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
                taken += 1;
            }
            assert!(state.residual_conservation_error < 1.0e-12);
            assert!(state.converter_account_error < 1.0e-12);
            index += 1;
        }
        let landed = GeneratorMmrtgSeebeckState::try_landing(&card).unwrap();
        let mut from_landing = landed;
        let mut frame = blank_frame();
        step(&card, &mut from_landing, &mut frame, NO_LOAD, floor_s).unwrap();
        assert_eq!(from_landing.t_s.to_bits(), floor_s.to_bits());

        let stuck_s = MISSION_TIME_MAX_S + 1.0e-8;
        assert_eq!(stuck_s.to_bits(), MISSION_TIME_MAX_S.to_bits());
        let mut still = landed;
        step(&card, &mut still, &mut frame, NO_LOAD, 1.0e-7).unwrap();
        assert_eq!(still.thermal_power_w.to_bits(), landed.thermal_power_w.to_bits());
    }

    /// Past the last day of the record the exact map goes on: a run of
    /// 4,000 steps at the ceiling, from day 4,087 to about day 182,000, some
    /// 498 years, lands every step with both flags, matches the closed form
    /// and closes both ledgers. The attacks of the card are refused there by
    /// name as they are inside the record.
    #[test]
    fn past_the_record_every_step_lands_and_the_ledger_closes() {
        let generator = GeneratorMmrtgSeebeck::curiosity();
        let ceiling_s = generator.gate_dt_s().unwrap();
        let mut state = GeneratorMmrtgSeebeckState::try_mission_time(&generator, MISSION_TIME_MAX_S).unwrap();
        let mut frame = blank_frame();
        let mut worst = 0.0_f64;
        for _ in 0..4_000 {
            assert_eq!(past_record_step(&generator, &mut state, &mut frame, ceiling_s), Ok(true));
            assert_eq!(frame.flags, FLAG_PAST_FITTING_SPAN | FLAG_PAST_RECORD);
            assert_eq!(state.range_margin_s, 0.0);
            proof_intact(&frame);
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
            assert!(state.converter_account_error <= CONVERTER_ACCOUNT_LIMIT);
        }
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst}");
        assert!(state.t_s > 498.0 * SECONDS_PER_YEAR);
        assert!(within(state.electrical_power_w, closed_form_electrical_power_w(state.t_s), 1.0e-9));
        let start = GeneratorMmrtgSeebeckState::try_mission_time(&generator, 20_000.0 * DAY_S).unwrap();
        let mut probe = start;
        assert_eq!(
            step_straight_line_fuel(&generator, &mut probe, &mut frame, 10.0 * SECONDS_PER_YEAR),
            Err(StepReject::FirstLawBreach)
        );
        assert_eq!(state_bits(&probe), state_bits(&start));
        let mut probe = start;
        assert_eq!(
            step_straight_line_fading(&generator, &mut probe, &mut frame, 10.0 * SECONDS_PER_YEAR),
            Err(StepReject::ConverterAccountBreach)
        );
        assert_eq!(state_bits(&probe), state_bits(&start));
    }

    /// The card holds mission time up to where the decay heat still held is
    /// the ledger's resolution, about 2,070 years after landing: a start just
    /// inside lands a step at the floor, and a step that would land past it
    /// is refused by name with nothing moved.
    #[test]
    fn the_ledger_end_is_where_the_heat_left_is_the_ledger_resolution() {
        let card = GeneratorMmrtgSeebeck::curiosity();
        let end_s = card.ledger_end_mission_time_s().unwrap();
        let years = end_s / SECONDS_PER_YEAR;
        assert!(years > 2_060.0 && years < 2_080.0, "{years}");
        let resolution_j = RESIDUAL_CONSERVATION_LIMIT * card.card_heat_j;
        assert!(within(FUEL_HEAT_AT_LANDING_J * crate::math::exp(-FUEL_DECAY_PER_S * end_s), resolution_j, 1.0e-6));
        let floor_s = card.floor_dt_s().unwrap();
        let mut frame = blank_frame();
        let mut state = GeneratorMmrtgSeebeckState::try_mission_time(&card, end_s - 10.0 * floor_s).unwrap();
        step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
        // Every published number of the state there is normal or zero.
        for value in [
            state.electrical_power_w,
            state.conversion_efficiency,
            state.converter_fading_fraction,
            state.converter_fading_yield_j,
            state.thermal_power_w,
        ] {
            assert!(value == 0.0 || value.is_normal(), "{value}");
        }
        let before = state_bits(&state);
        let held = frame.to_bytes();
        assert_eq!(
            step(&card, &mut state, &mut frame, NO_LOAD, 1_000.0 * floor_s),
            Err(StepReject::DecayHeatBelowLedgerResolution)
        );
        assert_eq!(state_bits(&state), before);
        assert_eq!(frame.to_bytes(), held);
        assert!(GeneratorMmrtgSeebeckState::try_mission_time(&card, end_s + 1_000.0).is_none());
    }
}
