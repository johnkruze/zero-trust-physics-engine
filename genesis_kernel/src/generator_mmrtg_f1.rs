#![no_implicit_prelude]

//! One Multi-Mission Radioisotope Thermoelectric Generator, flight unit F1,
//! on NASA's Curiosity rover, from its landing on 2012-08-06 to mission day
//! 4,087, the last day of the published record.
//!
//! Measured by Whiting and Woerner, Historical RTG Performance Data - 2023
//! Update, Dryad, doi: 10.5061/dryad.1zcrjdfw2, CC0 1.0. The authors ask that
//! any use also cite Whiting and Woerner, Chapter 8 of The Technology of
//! Discovery (Wiley, 2023, ISBN 9781119811367).
//!
//! The card gives the daily-average electrical output. It has no seasons, no
//! load and no events. Three constants are fitted to mission days 0 to 2,060
//! of that record. Past day 2,060 the card carries them forward and the frame
//! sets [`FLAG_PAST_FITTING_SPAN`]. A mission time before landing or after
//! day 4,087 is [`StepReject::MissionTimeOutsideCard`]. Nothing is
//! extrapolated past the record.
//!
//! Days 2,061 to 2,808 were used to choose the card's structure and are not
//! an independent test of it. Days 2,809 to 4,087, 1,279 days, were held
//! back, and the card was compared with them once, in a sealed run, on
//! 2026-10-06, against two lines fixed in advance, each 0.010 of 115.4 W.
//! Its root mean square error over those days was 0.008129, inside the line.
//! Its mean over the final 365 days was +0.010094, outside the line by
//! 0.0000936, so by the rule fixed in advance the card failed. It reads high
//! and the gap grows: the mean was +0.0031, +0.0045 and +0.0096 over three
//! successive spans of 365 days and +0.0107 over the last 184. The largest
//! difference on one day was +0.02251, 2.60 W, on day 3,546. The card serves
//! days 2,809 to 4,087 as the model that was scored on them, with no
//! constant changed since: a card that failed its comparison, served with
//! the failure stated. The record is the sealed run of 2026-10-06, 4,087
//! daily steps from landing scored against the dataset above, and row V-11
//! of the company's statement of results.
//!
//! The record holds no heat measurement. The thermal power at landing is a
//! pre-flight estimate. Thermal power, decay heat still held and heat
//! rejected are the card's arithmetic and have not been compared with the
//! generator. The record cannot separate the fuel's share of the loss from
//! the converter's. The stable fraction is the constant that fits days 0 to
//! 2,060. On days 2,809 to 4,087 the measured output fell faster than the
//! card's law with that constant, and it is not a statement about any day
//! after 4,087.
//!
//! The end of the card and rounding. The mission clock is a compensated
//! sum: each step adds `dt` to `t_s` in 64-bit arithmetic, keeps what that
//! addition rounded off in `clock_carry_s`, and folds the carry back in, so
//! `t_s` stays within half a spacing of 64-bit numbers of the exact sum of
//! the start and the steps however many steps a run takes. A start, and a
//! restart from a returned `t_s`, begins with no carry. Every step,
//! the first included, compares its landing with
//! [`MISSION_TIME_MAX_S`] plus one fixed tolerance,
//! [`MISSION_END_TOLERANCE_S`], 2^-22 s, about 2.38e-7 s: four spacings of
//! 64-bit numbers between 2^28 s and 2^29 s, where the last day lies. That
//! covers the rounding of a start and a step read from decimals and the
//! clock's own rounding, whether a run starts after 2^28 s, mission day
//! 3,106.9, or before it, where the spacing is finer, so a run or a single
//! step whose end in decimal is exactly mission day 4,087 lands its last
//! step, and so does a restart from any returned state of such a run. A step
//! whose landing, as the clock reads it, is past mission day 4,087 by more
//! than the tolerance is [`StepReject::MissionTimeOutsideCard`], at any
//! length of run; a step whose end in decimal is past the day by more than
//! twice the tolerance is refused by every route. The clock is stepped at
//! its own value and nothing is pulled back: a landed clock can read up to
//! the tolerance past the last day, where `range_margin_s` is zero.
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
//! thermal power           dQ/dt = −λ Q          λ = FUEL_DECAY_PER_S
//! fading fraction         dh/dt = −h / τ        1/τ = CONVERTER_FADING_RATE_PER_S, h = 1 at landing
//! conversion efficiency   η = η_s + η_f h       STABLE_EFFICIENCY, FADING_EFFICIENCY
//! electrical power        P = η Q
//! decay heat still held   U = Q / λ
//!
//! x1 = 1 − exp(−λ dt)   x2 = 1 − exp(−dt/τ)   x3 = 1 − exp(−(λ + 1/τ) dt)
//! Q' = Q − Q x1
//! h' = h − h x2
//! ΔW_electrical = η_s Q x1 / λ + η_f Q h x3 / (λ + 1/τ)
//! ΔW_rejected   = (1 − η_s) Q x1 / λ − η_f Q h x3 / (λ + 1/τ)
//!
//! residual_conservation_error = |U − U_anchor + W_electrical + W_rejected| / card_heat_j
//! ```
//!
//! The ledger is energy: decay heat released is electrical energy delivered
//! plus heat rejected. `card_heat_j` is the decay heat the fuel releases from
//! landing to day 4,087. The ledger fixes the total that leaves the fuel. It
//! does not fix how that total divides between electricity and rejected heat.
//!
//! The card also keeps one account of the fitted model. It is bookkeeping
//! that follows from the model and not a conservation law:
//!
//! ```text
//! fading yield still to come   Y = η_f Q h / (λ + 1/τ)
//! converter_account_error = |Y − Y_anchor + W_fading| / card_fading_yield_j
//! ```
//!
//! `W_fading` is the electrical energy the fading part has delivered. A
//! fading fraction that is not the exact map breaks this account and is
//! [`StepReject::ConverterAccountBreach`]. The account covers the fading
//! part only.
//!
//! [`GeneratorMmrtgF1::gate_dt_s`] is `(λ + 1/τ) Δt = 0.02`, about 4,224,500 s.
//! The public step does not apply that clock. The formula of the map is exact
//! for every finite `dt > 0` that stays on the card. Its arithmetic rounds
//! once a step: under about 2.3e-7 s the thermal power does not move, and at
//! the last day a first step under 2.9e-8 s, half a spacing of the clock
//! there, does not move the clock.
//! [`GeneratorMmrtgF1::floor_dt_s`], about 4.03 s, is the smallest step the
//! card stands behind. The public step does not apply it either. The gateway
//! refuses a step outside the two.
//!
//! The step calls [`crate::math::exp_m1`]. The state constructor calls
//! [`crate::math::exp`].

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Half-life of plutonium-238. NNDC NuDat 3, ENSDF evaluation of Browne and
/// Tuli, Nuclear Data Sheets 108, 681 (2007). Uncertainty 0.1.
pub const PLUTONIUM_238_HALF_LIFE_YEARS: f64 = 87.7;
/// 365.25 days of 86,400 s. A convention. Turns the two year-valued constants into seconds.
pub const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;
/// Thermal power of the fuel at landing. Woerner et al. 2013, Table 1,
/// "Surface Ops Spring, 08/06/2012", Qsource. A pre-flight estimate, not a
/// measurement.
pub const THERMAL_POWER_AT_LANDING_W: f64 = 1953.2;
/// Highest daily-average electrical power of the first 30 days. The dataset,
/// sheet `MMRTG`, cell C14. The dataset's values are divided by this number.
pub const ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W: f64 = 115.4;
/// Electrical power at landing as a fraction of
/// [`ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W`]. Fitted, mission days 0 to 2,060.
pub const OUTPUT_AMPLITUDE: f64 = 0.995738;
/// Fraction of the converter's output at landing that does not fade. Fitted,
/// mission days 0 to 2,060.
pub const CONVERTER_STABLE_FRACTION: f64 = 0.690601;
/// Time constant of the fading part of the converter's output. Fitted,
/// mission days 0 to 2,060.
pub const CONVERTER_FADING_TIME_CONSTANT_YEARS: f64 = 7.06718;
/// Mission day 2,060. Last day of the fitting data.
pub const FITTING_SPAN_END_S: f64 = 2_060.0 * 86_400.0;
/// Mission day 4,087, 353,116,800 s. Last day of the card and of the record.
/// Past this the step is refused. The card failed its sealed comparison with
/// days 2,809 to 4,087 and serves those days with the failure stated.
pub const MISSION_TIME_MAX_S: f64 = 4_087.0 * 86_400.0;
/// Four spacings of 64-bit numbers between 2^28 s and 2^29 s, 2^-22 s: how
/// far past [`MISSION_TIME_MAX_S`] the landing of any step may read. The
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
/// `1 / τ`. 4.483837657174283e-09 per second.
pub const CONVERTER_FADING_RATE_PER_S: f64 =
    1.0 / (CONVERTER_FADING_TIME_CONSTANT_YEARS * SECONDS_PER_YEAR);
/// `λ + 1/τ`. 4.734288155756154e-09 per second. The card's fastest rate.
pub const COMBINED_RATE_PER_S: f64 = FUEL_DECAY_PER_S + CONVERTER_FADING_RATE_PER_S;
/// Share of the thermal power the stable part of the converter turns into
/// electricity. 0.040628555086670695.
pub const STABLE_EFFICIENCY: f64 =
    ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W * OUTPUT_AMPLITUDE * CONVERTER_STABLE_FRACTION
        / THERMAL_POWER_AT_LANDING_W;
/// Share of the thermal power the fading part of the converter turns into
/// electricity at landing. 0.018202166396024372.
pub const FADING_EFFICIENCY: f64 = ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
    * OUTPUT_AMPLITUDE
    * (1.0 - CONVERTER_STABLE_FRACTION)
    / THERMAL_POWER_AT_LANDING_W;
/// Decay heat the fuel still holds at landing, `Q / λ`. 7.798746702680e12 J.
pub const FUEL_HEAT_AT_LANDING_J: f64 = THERMAL_POWER_AT_LANDING_W / FUEL_DECAY_PER_S;
/// Electrical energy the fading part has still to deliver at landing,
/// `η_f Q / (λ + 1/τ)`. 7.509570654564e9 J.
pub const FADING_YIELD_AT_LANDING_J: f64 =
    FADING_EFFICIENCY * THERMAL_POWER_AT_LANDING_W / COMBINED_RATE_PER_S;

/// `|U − U_anchor + W_electrical + W_rejected| / card_heat_j` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// `|Y − Y_anchor + W_fading| / card_fading_yield_j` must stay at or under this.
pub const CONVERTER_ACCOUNT_LIMIT: f64 = RESIDUAL_CONSERVATION_LIMIT;
/// Phase advance `(λ + 1/τ) Δt` used by [`GeneratorMmrtgF1::gate_dt_s`].
pub const GATE_PHASE_STEP: f64 = 0.02;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;

/// Mission time is past [`FITTING_SPAN_END_S`] on this frame. The step still lands.
pub const FLAG_PAST_FITTING_SPAN: u64 = 1 << 0;

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
    /// Mission time, incoming or landing, is before landing or after mission day 4,087; the landing by more than the rounding tolerance of the clock, 2.38e-7 s, the same for every step.
    MissionTimeOutsideCard,
    /// The trial step created energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed energy.
    FirstLawBreach,
    /// The trial step broke the fitted model's account of the fading part of the converter, which is bookkeeping that follows from the model and not a conservation law.
    ConverterAccountBreach,
}

/// The card has no load. Nothing a caller can set changes what it computes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeneratorLoad {}

/// The locked generator. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratorMmrtgF1 {
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
    /// Recorded copy of [`CONVERTER_STABLE_FRACTION`].
    pub converter_stable_fraction: f64,
    /// Recorded copy of [`CONVERTER_FADING_TIME_CONSTANT_YEARS`].
    pub converter_fading_time_constant_years: f64,
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
    /// 6.600889916499e11 J. Scale of the residual. Computed once by the
    /// constructor from the constants.
    pub card_heat_j: f64,
    /// Electrical energy the fading part delivers from landing to
    /// [`MISSION_TIME_MAX_S`]. 6.098397178946e9 J. Scale of the converter
    /// account. Computed once by the constructor from the constants.
    pub card_fading_yield_j: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`CONVERTER_ACCOUNT_LIMIT`].
    pub converter_account_limit: f64,
    /// Recorded copy of [`GATE_PHASE_STEP`].
    pub gate_phase_step: f64,
}

impl GeneratorMmrtgF1 {
    /// Card: flight unit F1 on the Curiosity rover. The constructor takes no arguments.
    #[must_use]
    pub fn curiosity() -> Self {
        let generator = Self {
            plutonium_238_half_life_years: PLUTONIUM_238_HALF_LIFE_YEARS,
            seconds_per_year: SECONDS_PER_YEAR,
            thermal_power_at_landing_w: THERMAL_POWER_AT_LANDING_W,
            electrical_power_at_beginning_of_mission_w: ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W,
            output_amplitude: OUTPUT_AMPLITUDE,
            converter_stable_fraction: CONVERTER_STABLE_FRACTION,
            converter_fading_time_constant_years: CONVERTER_FADING_TIME_CONSTANT_YEARS,
            fitting_span_end_s: FITTING_SPAN_END_S,
            mission_time_max_s: MISSION_TIME_MAX_S,
            mission_end_tolerance_s: MISSION_END_TOLERANCE_S,
            fuel_decay_per_s: FUEL_DECAY_PER_S,
            converter_fading_rate_per_s: CONVERTER_FADING_RATE_PER_S,
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
            ::core::panic!("Curiosity generator failed is_valid");
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
            && self.converter_stable_fraction.to_bits() == CONVERTER_STABLE_FRACTION.to_bits()
            && self.converter_fading_time_constant_years.to_bits()
                == CONVERTER_FADING_TIME_CONSTANT_YEARS.to_bits()
            && self.fitting_span_end_s.to_bits() == FITTING_SPAN_END_S.to_bits()
            && self.mission_time_max_s.to_bits() == MISSION_TIME_MAX_S.to_bits()
            && self.mission_end_tolerance_s.to_bits() == MISSION_END_TOLERANCE_S.to_bits()
            && self.fuel_decay_per_s.to_bits() == FUEL_DECAY_PER_S.to_bits()
            && self.converter_fading_rate_per_s.to_bits() == CONVERTER_FADING_RATE_PER_S.to_bits()
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
            && self.combined_rate_per_s.is_finite()
            && self.combined_rate_per_s > 0.0
            && self.stable_efficiency > 0.0
            && self.fading_efficiency > 0.0
            && self.stable_efficiency + self.fading_efficiency < 1.0
            && self.fitting_span_end_s > 0.0
            && self.mission_time_max_s > self.fitting_span_end_s
            && self.card_heat_j.is_finite()
            && self.card_heat_j > 0.0
            && self.card_fading_yield_j.is_finite()
            && self.card_fading_yield_j > 0.0
    }

    /// Largest reporting step of the card's fastest rate, `λ + 1/τ`. Not applied by [`step`].
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
    /// 2^20, about 4.03 s. At the ceiling a step advances the card's fastest
    /// rate by two hundredths. A million times smaller, a step lowers the
    /// thermal power by one part in a thousand million, which 64-bit
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

    /// The latest mission time, in seconds, a step can start from: the
    /// largest 64-bit number `t` from which a step of `floor_dt_s`, started
    /// with no clock carry, lands by the comparison every step makes, on or
    /// before [`MISSION_TIME_MAX_S`] plus [`MISSION_END_TOLERANCE_S`]. A
    /// start later than this is refused at every step the card allows,
    /// because the smallest step already lands past the end.
    #[must_use]
    pub fn last_start_mission_time_s(&self) -> Option<f64> {
        let floor_dt_s = self.floor_dt_s()?;
        let lands = |t_s: f64| -> bool {
            let (landing_s, _) = advance_clock(t_s, 0.0, floor_dt_s);
            landing_on_card(self, landing_s)
        };
        let mut t_s = MISSION_TIME_MAX_S - floor_dt_s;
        if !on_card(t_s) {
            return None;
        }
        // Walk down until the landing is on the card, then up while it stays.
        while !lands(t_s) {
            t_s = f64::from_bits(t_s.to_bits() - 1);
        }
        loop {
            let above_s = f64::from_bits(t_s.to_bits() + 1);
            if on_card(above_s) && lands(above_s) {
                t_s = above_s;
            } else {
                break;
            }
        }
        Some(t_s)
    }

    /// Decay heat the fuel still holds at this thermal power, `Q / λ`.
    #[must_use]
    pub fn fuel_heat_j(&self, thermal_power_w: f64) -> f64 {
        thermal_power_w / self.fuel_decay_per_s
    }

    /// Electrical energy the fading part has still to deliver, `η_f Q h / (λ + 1/τ)`.
    #[must_use]
    pub fn converter_fading_yield_j(
        &self,
        thermal_power_w: f64,
        converter_fading_fraction: f64,
    ) -> f64 {
        self.fading_efficiency * thermal_power_w * converter_fading_fraction
            / self.combined_rate_per_s
    }
}

/// `FUEL_HEAT_AT_LANDING_J (1 − exp(−λ MISSION_TIME_MAX_S))`.
fn card_heat_from_constants_j() -> f64 {
    FUEL_HEAT_AT_LANDING_J * -crate::math::exp_m1(-FUEL_DECAY_PER_S * MISSION_TIME_MAX_S)
}

/// `FADING_YIELD_AT_LANDING_J (1 − exp(−(λ + 1/τ) MISSION_TIME_MAX_S))`.
fn card_fading_yield_from_constants_j() -> f64 {
    FADING_YIELD_AT_LANDING_J * -crate::math::exp_m1(-COMBINED_RATE_PER_S * MISSION_TIME_MAX_S)
}

/// Integrated generator. The ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeneratorState {
    /// Mission time since landing.
    pub t_s: f64,
    /// What the rounding of the mission clock has left out of `t_s`,
    /// seconds: `t_s + clock_carry_s` is the sum of the start and the steps
    /// to within the rounding of the carry itself. Under half a spacing of
    /// `t_s`. Zero at a start, and so zero at a restart from a returned `t_s`.
    pub clock_carry_s: f64,
    /// Thermal power of the fuel, `Q`.
    pub thermal_power_w: f64,
    /// Fading fraction of the converter, `h`. 1 at landing.
    pub converter_fading_fraction: f64,
    /// `Q (η_s + η_f h)`.
    pub electrical_power_w: f64,
    /// `η_s + η_f h`.
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
    /// Electrical energy the fading part has still to deliver, `Y = η_f Q h / (λ + 1/τ)`.
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
    /// `MISSION_TIME_MAX_S − t_s`.
    pub range_margin_s: f64,
    /// `t_s > FITTING_SPAN_END_S`.
    pub past_fitting_span: bool,
}

impl GeneratorState {
    /// The generator at landing, mission time 0.
    #[must_use]
    pub fn try_landing(generator: &GeneratorMmrtgF1) -> Option<Self> {
        Self::try_mission_time(generator, 0.0)
    }

    /// Anchor both ledgers at this mission time. Thermal power and the fading
    /// fraction are the generator's at that time and are not caller inputs.
    #[must_use]
    pub fn try_mission_time(generator: &GeneratorMmrtgF1, t_s: f64) -> Option<Self> {
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
        if !state_is_finite(&state) {
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
#[must_use]
pub fn step(
    generator: &GeneratorMmrtgF1,
    state: &mut GeneratorState,
    frame: &mut LastStateFrame64,
    load: GeneratorLoad,
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

fn on_card(t_s: f64) -> bool {
    t_s.is_finite() && t_s >= 0.0 && t_s <= MISSION_TIME_MAX_S
}

/// The landing of a step is on the card: at or after landing on Mars, and
/// no later than the last day by more than [`MISSION_END_TOLERANCE_S`]. The
/// same comparison for every step, the first included.
fn landing_on_card(generator: &GeneratorMmrtgF1, t_s: f64) -> bool {
    t_s.is_finite()
        && t_s >= 0.0
        && t_s <= generator.mission_time_max_s + generator.mission_end_tolerance_s
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
    generator: &GeneratorMmrtgF1,
    state: &GeneratorState,
    load: GeneratorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    // The load has no field, so none can be missing or non-finite.
    let GeneratorLoad {} = load;
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
        return Err(StepReject::MissionTimeOutsideCard);
    }
    let (landing_s, _) = advance_clock(state.t_s, state.clock_carry_s, dt_s);
    if !landing_on_card(generator, landing_s) {
        return Err(StepReject::MissionTimeOutsideCard);
    }
    Ok(())
}

fn commit(
    generator: &GeneratorMmrtgF1,
    state: &mut GeneratorState,
    frame: &mut LastStateFrame64,
    load: GeneratorLoad,
    dt_s: f64,
    advanced: ExactStep,
) -> Result<(), StepReject> {
    let GeneratorLoad {} = load;
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
    if !landing_on_card(generator, landing_s) {
        return Err(StepReject::MissionTimeOutsideCard);
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
    let gate = RESIDUAL_CONSERVATION_LIMIT * scale;
    if state.energy_defect_j > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    if state.converter_account_error > CONVERTER_ACCOUNT_LIMIT {
        *state = saved;
        return Err(StepReject::ConverterAccountBreach);
    }
    publish(state, frame);
    Ok(())
}

fn refresh(generator: &GeneratorMmrtgF1, state: &mut GeneratorState) {
    state.conversion_efficiency =
        generator.stable_efficiency + generator.fading_efficiency * state.converter_fading_fraction;
    state.electrical_power_w = state.thermal_power_w * state.conversion_efficiency;
    state.fuel_heat_j = generator.fuel_heat_j(state.thermal_power_w);
    state.converter_fading_yield_j =
        generator.converter_fading_yield_j(state.thermal_power_w, state.converter_fading_fraction);
    // Never below zero: a clock inside the rounding tolerance past the last
    // day has no margin left.
    let margin_s = MISSION_TIME_MAX_S - state.t_s;
    state.range_margin_s = if margin_s > 0.0 { margin_s } else { 0.0 };
    state.past_fitting_span = state.t_s > FITTING_SPAN_END_S;
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

/// `Q − Q x1`, `h − h x2`, and the electrical energy and rejected heat each
/// from its own integral. `Q` and `h` are the values at the start of the step.
fn exact_map(
    generator: &GeneratorMmrtgF1,
    thermal_power_w: f64,
    converter_fading_fraction: f64,
    dt_s: f64,
) -> Option<ExactStep> {
    let fuel_decay_per_s = generator.fuel_decay_per_s;
    let fading_rate_per_s = generator.converter_fading_rate_per_s;
    let combined_rate_per_s = generator.combined_rate_per_s;
    if !(fuel_decay_per_s > 0.0) || !(fading_rate_per_s > 0.0) || !(combined_rate_per_s > 0.0) {
        return None;
    }
    let fuel_decrement = -crate::math::exp_m1(-fuel_decay_per_s * dt_s);
    let fading_decrement = -crate::math::exp_m1(-fading_rate_per_s * dt_s);
    let combined_decrement = -crate::math::exp_m1(-combined_rate_per_s * dt_s);
    if !fuel_decrement.is_finite()
        || !fading_decrement.is_finite()
        || !combined_decrement.is_finite()
    {
        return None;
    }
    let next_thermal_power_w = thermal_power_w - thermal_power_w * fuel_decrement;
    let next_fading_fraction =
        converter_fading_fraction - converter_fading_fraction * fading_decrement;
    // `x1 / λ` and `x3 / (λ + 1/τ)`: the time integrals of the two decays over the step.
    let fuel_integral_s = fuel_decrement / fuel_decay_per_s;
    let combined_integral_s = combined_decrement / combined_rate_per_s;
    let stable_electrical_energy_j =
        generator.stable_efficiency * thermal_power_w * fuel_integral_s;
    let fading_electrical_energy_j = generator.fading_efficiency
        * thermal_power_w
        * converter_fading_fraction
        * combined_integral_s;
    let heat_rejected_j = (1.0 - generator.stable_efficiency) * thermal_power_w * fuel_integral_s
        - fading_electrical_energy_j;
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

fn publish(state: &GeneratorState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.electrical_power_w as f32;
    frame.pos[1] = state.thermal_power_w as f32;
    frame.pos[2] = state.converter_fading_fraction as f32;
    frame.vel[0] = state.electrical_energy_j as f32;
    frame.vel[1] = state.heat_rejected_j as f32;
    frame.vel[2] = state.fuel_heat_j as f32;
    frame.force_torque = state.range_margin_s as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.past_fitting_span {
        FLAG_PAST_FITTING_SPAN
    } else {
        0
    };
}

fn state_is_finite(state: &GeneratorState) -> bool {
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

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::panic;

    use super::{
        COMBINED_RATE_PER_S, CONVERTER_ACCOUNT_LIMIT, CONVERTER_FADING_RATE_PER_S,
        CONVERTER_STABLE_FRACTION, ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W, ExactStep,
        FADING_EFFICIENCY, FADING_YIELD_AT_LANDING_J, FITTING_SPAN_END_S, FLAG_PAST_FITTING_SPAN,
        FUEL_DECAY_PER_S, FUEL_HEAT_AT_LANDING_J, GATE_PHASE_STEP, GeneratorLoad, GeneratorMmrtgF1,
        GeneratorState, MISSION_END_TOLERANCE_S, MISSION_TIME_MAX_S, OUTPUT_AMPLITUDE, RESIDUAL_CONSERVATION_LIMIT,
        SECONDS_PER_YEAR, STABLE_EFFICIENCY, StepReject, advance_clock, commit, exact_map, step,
        validate,
    };
    use crate::frame::LastStateFrame64;

    const DAY_S: f64 = 86_400.0;
    const NO_LOAD: GeneratorLoad = GeneratorLoad {};

    /// One row of `analysis/out/card_reference.json`, `daily_steps`, in the
    /// card's research record: every row of the file after landing, to the
    /// card's last day.
    struct ReferenceDay {
        day: u32,
        thermal_power_w: f64,
        electrical_power_w: f64,
        ratio: f64,
        fading_fraction: f64,
        electrical_energy_j: f64,
        heat_rejected_j: f64,
    }

    const REFERENCE_DAYS: [ReferenceDay; 11] = [
        ReferenceDay {
            day: 1,
            thermal_power_w: 1953.157735312728,
            electrical_power_w: 114.89190854899819,
            ratio: 0.9955971278076099,
            fading_fraction: 0.9996126714574951,
            electrical_energy_j: 9927363.142868828,
            heat_rejected_j: 158827291.01605615,
        },
        ReferenceDay {
            day: 17,
            thermal_power_w: 1952.4816246820474,
            electrical_power_w: 114.63261419823897,
            ratio: 0.9933502096901123,
            fading_fraction: 0.9934357784996335,
            electrical_energy_j: 168574537.38651296,
            heat_rejected_j: 2699758015.4342203,
        },
        ReferenceDay {
            day: 365,
            thermal_power_w: 1937.8339841037518,
            electrical_power_w: 109.35310254120095,
            ratio: 0.9476005419514814,
            fading_fraction: 0.8681399849364194,
            electrical_energy_j: 3534204786.843512,
            heat_rejected_j: 57819300130.314606,
        },
        ReferenceDay {
            day: 687,
            thermal_power_w: 1924.3786050399708,
            electrical_power_w: 105.02751811402887,
            ratio: 0.9101171413694009,
            fading_fraction: 0.766327040105466,
            electrical_energy_j: 6515162545.065727,
            heat_rejected_j: 108563047014.19756,
        },
        ReferenceDay {
            day: 1374,
            thermal_power_w: 1895.982498226288,
            electrical_power_w: 97.2978537599288,
            ratio: 0.8431356478330051,
            fading_fraction: 0.587257132396805,
            electrical_energy_j: 12510642443.210155,
            heat_rejected_j: 215947684043.22534,
        },
        ReferenceDay {
            day: 2060,
            thermal_power_w: 1868.0458260749324,
            electrical_power_w: 91.20410320475075,
            ratio: 0.7903301837500065,
            fading_fraction: 0.4502053974508907,
            electrical_energy_j: 18089988852.899303,
            heat_rejected_j: 321914021549.2889,
        },
        ReferenceDay {
            day: 2061,
            thermal_power_w: 1868.0054040123432,
            electrical_power_w: 91.19620053159335,
            ratio: 0.790261703046736,
            fading_fraction: 0.45003102005046824,
            electrical_energy_j: 18097868546.002018,
            heat_rejected_j: 322067539269.31964,
        },
        ReferenceDay {
            day: 2808,
            thermal_power_w: 1838.0531801201873,
            electrical_power_w: 85.9505334448983,
            ratio: 0.7448053158136767,
            fading_fraction: 0.3369471335710184,
            electrical_energy_j: 23807748161.35497,
            heat_rejected_j: 435951048614.0663,
        },
        ReferenceDay {
            day: 2809,
            thermal_power_w: 1838.013407059135,
            electrical_power_w: 85.94430729355918,
            ratio: 0.7447513630291089,
            fading_fraction: 0.33681662432887116,
            electrical_energy_j: 23815174018.46104,
            heat_rejected_j: 436102428833.52014,
        },
        ReferenceDay {
            day: 3448,
            thermal_power_w: 1812.7735970188385,
            electrical_power_w: 82.32698407266847,
            ratio: 0.7134054079087389,
            fading_fraction: 0.2629562733369934,
            electrical_energy_j: 28457090824.660713,
            heat_rejected_j: 532238151453.92267,
        },
        ReferenceDay {
            day: 4087,
            thermal_power_w: 1787.8803829328635,
            electrical_power_w: 79.31989849770096,
            ratio: 0.6873474739835438,
            fading_fraction: 0.20529272218987796,
            electrical_energy_j: 32916859138.300224,
            heat_rejected_j: 627172132511.6387,
        },
    ];

    /// The five rows of the card spec's table, section 8, test 3, to the
    /// digits printed there, and one row the spec does not print: day 2,808,
    /// the last day of the days that chose the structure, from the closed
    /// form of the header worked in 50-digit decimal arithmetic and cut to
    /// the same digits.
    const SPEC_TABLE: [(u32, f64, f64, f64); 6] = [
        (1, 1953.157735313, 114.891908549, 0.995597128),
        (365, 1937.833984104, 109.353102541, 0.947600542),
        (2060, 1868.045826075, 91.204103205, 0.790330184),
        (2808, 1838.053180120, 85.950533445, 0.744805316),
        (2809, 1838.013407059, 85.944307294, 0.744751363),
        (4087, 1787.880382933, 79.319898498, 0.687347474),
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

    fn state_bits(state: &GeneratorState) -> [u64; 19] {
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

    /// `115.4 W × 0.995738 × exp(−λ t) × [0.690601 + 0.309399 exp(−t/τ)]`.
    fn closed_form_electrical_power_w(t_s: f64) -> f64 {
        ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
            * OUTPUT_AMPLITUDE
            * crate::math::exp(-FUEL_DECAY_PER_S * t_s)
            * (CONVERTER_STABLE_FRACTION
                + (1.0 - CONVERTER_STABLE_FRACTION)
                    * crate::math::exp(-CONVERTER_FADING_RATE_PER_S * t_s))
    }

    fn landing() -> (GeneratorMmrtgF1, GeneratorState) {
        let generator = GeneratorMmrtgF1::curiosity();
        let state = GeneratorState::try_landing(&generator).unwrap();
        (generator, state)
    }

    /// The real `validate`, the real exact map with one alteration, the real `commit`.
    fn step_altered(
        generator: &GeneratorMmrtgF1,
        state: &mut GeneratorState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        alter: impl FnOnce(&GeneratorState, &mut ExactStep),
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
        generator: &GeneratorMmrtgF1,
        state: &mut GeneratorState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |start, advanced| {
            advanced.thermal_power_w =
                start.thermal_power_w - start.thermal_power_w * FUEL_DECAY_PER_S * dt_s;
        })
    }

    /// Fading fraction on a straight line, `h (1 − dt/τ)`, against the exact energies.
    fn step_straight_line_fading(
        generator: &GeneratorMmrtgF1,
        state: &mut GeneratorState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        step_altered(generator, state, frame, dt_s, |start, advanced| {
            advanced.converter_fading_fraction = start.converter_fading_fraction
                - start.converter_fading_fraction * CONVERTER_FADING_RATE_PER_S * dt_s;
        })
    }

    /// Add `extra_j` to the electrical energy without taking it from the fuel.
    fn step_bias_electrical_energy(
        generator: &GeneratorMmrtgF1,
        state: &mut GeneratorState,
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
        generator: &GeneratorMmrtgF1,
        state: &mut GeneratorState,
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
        generator: &GeneratorMmrtgF1,
        state: &mut GeneratorState,
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
        generator: &GeneratorMmrtgF1,
        state: &mut GeneratorState,
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
        generator: &GeneratorMmrtgF1,
        state: &mut GeneratorState,
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
        assert_copy::<GeneratorMmrtgF1>();
        assert_copy::<GeneratorState>();
        assert_copy::<GeneratorLoad>();
        assert_copy::<StepReject>();
        let (generator, mut state) = landing();
        let saved = state_bits(&state);
        let mut frame = blank_frame();
        let held = frame.to_bytes();

        // analysis/out/card_reference.json, "constants".
        assert_eq!(
            FUEL_DECAY_PER_S.to_bits(),
            2.504504985818715e-10_f64.to_bits()
        );
        assert_eq!(
            CONVERTER_FADING_RATE_PER_S.to_bits(),
            4.483837657174283e-09_f64.to_bits()
        );
        assert_eq!(
            COMBINED_RATE_PER_S.to_bits(),
            4.734288155756154e-09_f64.to_bits()
        );
        assert_eq!(
            STABLE_EFFICIENCY.to_bits(),
            0.040628555086670695_f64.to_bits()
        );
        assert_eq!(
            FADING_EFFICIENCY.to_bits(),
            0.018202166396024372_f64.to_bits()
        );
        assert_eq!(MISSION_TIME_MAX_S.to_bits(), 353_116_800.0_f64.to_bits());
        assert_eq!(FITTING_SPAN_END_S.to_bits(), 177_984_000.0_f64.to_bits());
        assert_eq!(SECONDS_PER_YEAR.to_bits(), 31_557_600.0_f64.to_bits());
        assert_eq!(
            FUEL_HEAT_AT_LANDING_J.to_bits(),
            7798746702680.2705_f64.to_bits()
        );
        assert_eq!(
            FADING_YIELD_AT_LANDING_J.to_bits(),
            7509570654.563676_f64.to_bits()
        );
        // The two scales to day 4,087, from the closed form of the header
        // worked in 50-digit decimal arithmetic.
        assert!(within(generator.card_heat_j, 660088991649.9416, 1.0e-12));
        assert!(within(
            generator.card_fading_yield_j,
            6098397178.945743,
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
            (ceiling_s - 4_224_500.0).abs() < 50.0,
            "ceiling {ceiling_s}"
        );

        let mut refuse = |tampered: GeneratorMmrtgF1| {
            assert!(!tampered.is_valid());
            let got = step(&tampered, &mut state, &mut frame, NO_LOAD, DAY_S);
            assert_eq!(got, Err(StepReject::InvalidGenerator));
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            proof_intact(&frame);
            assert!(GeneratorState::try_landing(&tampered).is_none());
        };
        let mut tampered = generator;
        tampered.plutonium_238_half_life_years = 87.74;
        refuse(tampered);
        let mut tampered = generator;
        tampered.converter_stable_fraction = 0.7;
        refuse(tampered);
        let mut tampered = generator;
        tampered.thermal_power_at_landing_w = 2_000.0;
        refuse(tampered);
        let mut tampered = generator;
        tampered.fuel_decay_per_s = f64::from_bits(FUEL_DECAY_PER_S.to_bits() + 1);
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
    }

    #[test]
    fn landing_state_is_the_locked_power_and_a_zero_ledger() {
        let (generator, state) = landing();
        assert_eq!(state.t_s, 0.0);
        assert_eq!(state.thermal_power_w.to_bits(), 1953.2_f64.to_bits());
        assert_eq!(state.converter_fading_fraction.to_bits(), 1.0_f64.to_bits());
        assert!((state.electrical_power_w - 114.908165200).abs() < 5.0e-10);
        assert!(within(
            state.electrical_power_w,
            114.90816520000001,
            1.0e-12
        ));
        assert!((100.0 * state.conversion_efficiency - 5.8831).abs() < 5.0e-5);
        assert!(within(
            state.electrical_power_w / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W,
            0.995738,
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
            GeneratorState::try_mission_time(&generator, 0.0).unwrap(),
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
        let mut spec_rows = 0;
        let mut day = 0u32;
        while day < 4_087 {
            step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
            day += 1;
            proof_intact(&frame);
            assert_eq!(state.t_s, f64::from(day) * DAY_S);
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
                    assert!(within(
                        state.electrical_energy_j,
                        row.electrical_energy_j,
                        1.0e-12
                    ));
                    assert!(within(state.heat_rejected_j, row.heat_rejected_j, 1.0e-12));
                }
                index += 1;
            }
            let mut index = 0;
            while index < SPEC_TABLE.len() {
                let (spec_day, thermal_w, electrical_w, spec_ratio) = SPEC_TABLE[index];
                if spec_day == day {
                    spec_rows += 1;
                    assert!((state.thermal_power_w - thermal_w).abs() <= 5.0e-10);
                    assert!((state.electrical_power_w - electrical_w).abs() <= 5.0e-10);
                    assert!((ratio - spec_ratio).abs() <= 5.0e-10);
                }
                index += 1;
            }
        }
        assert_eq!(reference_rows, 11);
        assert_eq!(spec_rows, 6);
        assert_eq!(state.t_s.to_bits(), MISSION_TIME_MAX_S.to_bits());
        assert_eq!(state.range_margin_s, 0.0);
        // 3.2916859138e10 J and 6.27172132512e11 J at day 4,087, the totals
        // the spec prints, which the closed form gives to the same digits.
        assert!((state.electrical_energy_j - 3.2916859138e10).abs() <= 0.5);
        assert!((state.heat_rejected_j - 6.27172132512e11).abs() <= 0.5);
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
        assert_eq!(frame.flags, FLAG_PAST_FITTING_SPAN);
    }

    /// One held-back day of the record. `measured` is the workbook's value
    /// for the day, the daily-average electrical power over 115.4 W: Whiting
    /// and Woerner, Historical RTG Performance Data - 2023 Update, Dryad,
    /// doi: 10.5061/dryad.1zcrjdfw2, CC0 1.0, workbook
    /// `All_RTGs_P_Over_P0_-_2023_Update_for_Release.xlsx`, sheet `MMRTG`,
    /// column C, row 18 plus the mission day. `scored` is the difference the
    /// sealed comparison of 2026-10-06 scored on that day, card minus
    /// measured as a fraction of 115.4 W, from the 32-bit electrical power
    /// in the frame of its run of 4,087 daily steps from landing.
    struct HeldBackDay {
        day: u32,
        measured: f64,
        scored: f64,
    }

    /// Nine of the 1,279 held-back days, taken by rule and not by value: the
    /// first and the last day of each of the four spans the record reports
    /// a mean over (2,809 to 3,173, 3,174 to 3,538, 3,539 to 3,903 and 3,904
    /// to 4,087), and day 3,546, the day of the record's largest difference.
    const HELD_BACK_DAYS: [HeldBackDay; 9] = [
        HeldBackDay {
            day: 2809,
            measured: 0.742320885457405,
            scored: 0.002430461335678813,
        },
        HeldBackDay {
            day: 3173,
            measured: 0.7247239251739339,
            scored: 0.0014422429860665442,
        },
        HeldBackDay {
            day: 3174,
            measured: 0.7256935758675961,
            scored: 0.00042433009129683175,
        },
        HeldBackDay {
            day: 3538,
            measured: 0.7020701369002196,
            scored: 0.007375955305389925,
        },
        HeldBackDay {
            day: 3539,
            measured: 0.7019180915814247,
            scored: 0.00748456464318803,
        },
        HeldBackDay {
            day: 3546,
            measured: 0.6865866381978059,
            scored: 0.02251242894805905,
        },
        HeldBackDay {
            day: 3903,
            measured: 0.6892134386613181,
            scored: 0.005170681704664282,
        },
        HeldBackDay {
            day: 3904,
            measured: 0.6913202292998185,
            scored: 0.0030246862918394957,
        },
        HeldBackDay {
            day: 4087,
            measured: 0.6820414695233954,
            scored: 0.005306021921104631,
        },
    ];

    /// The days past 2,808 are served as the record scored them. On each of
    /// the nine days a run of daily steps from landing writes a frame whose
    /// electrical power, against the measured value, gives the difference
    /// the sealed comparison scored, bit for bit; the 64-bit power gives it
    /// within the frame's single precision. These are the errors of a card
    /// that failed its comparison, and the test holds the card to them: the
    /// largest is the record's largest, +0.02251 of 115.4 W, 2.60 W, on day
    /// 3,546.
    #[test]
    fn the_held_back_days_read_as_the_sealed_comparison_scored_them() {
        let (generator, mut state) = landing();
        let mut frame = blank_frame();
        // Half a unit in the last place of a 32-bit number between 64 W and
        // 128 W, 2^-18 W, as a fraction of 115.4 W.
        let frame_rounding = 2.0_f64.powi(-18) / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W;
        assert!(frame_rounding < 3.4e-8);
        let mut rows = 0;
        let mut largest = 0.0_f64;
        let mut largest_day = 0u32;
        let mut day = 0u32;
        while day < 4_087 {
            step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
            day += 1;
            let mut index = 0;
            while index < HELD_BACK_DAYS.len() {
                let row = &HELD_BACK_DAYS[index];
                if row.day == day {
                    rows += 1;
                    assert!(day > 2_808);
                    assert_eq!(frame.flags, FLAG_PAST_FITTING_SPAN);
                    let from_frame = f64::from(frame.pos[0])
                        / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
                        - row.measured;
                    assert_eq!(
                        from_frame.to_bits(),
                        row.scored.to_bits(),
                        "day {day}: {from_frame} against the scored {}",
                        row.scored
                    );
                    let difference = state.electrical_power_w
                        / ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W
                        - row.measured;
                    assert!(
                        (difference - row.scored).abs() <= frame_rounding,
                        "day {day}: {difference} against the scored {}",
                        row.scored
                    );
                    if difference.abs() > largest.abs() {
                        largest = difference;
                        largest_day = day;
                    }
                }
                index += 1;
            }
        }
        assert_eq!(rows, 9);
        assert_eq!(largest_day, 3_546);
        assert!((largest - 0.02251).abs() < 5.0e-6, "largest {largest}");
        let largest_w = largest * ELECTRICAL_POWER_AT_BEGINNING_OF_MISSION_W;
        assert!((largest_w - 2.60).abs() < 5.0e-3, "largest {largest_w} W");
    }

    /// The spacing of 64-bit numbers at the last day, 2^-24 s.
    fn spacing_at_the_end_s() -> f64 {
        f64::from_bits(MISSION_TIME_MAX_S.to_bits() + 1) - MISSION_TIME_MAX_S
    }

    /// A decimal with three places, from thousandths.
    fn milli(thousandths: u64) -> f64 {
        std::format!("{}.{:03}", thousandths / 1_000, thousandths % 1_000)
            .parse()
            .unwrap()
    }

    /// The tolerance is four spacings of the clock at the last day, one
    /// number for every step, and a recorded constant.
    #[test]
    fn the_mission_end_tolerance_is_four_spacings_at_the_last_day() {
        let generator = GeneratorMmrtgF1::curiosity();
        let spacing_s = spacing_at_the_end_s();
        // The last day lies between 2^28 s and 2^29 s, where the spacing is
        // 2^-24 s, twice the spacing of the days before 2^28 s.
        assert!(MISSION_TIME_MAX_S > 2.0_f64.powi(28) && MISSION_TIME_MAX_S < 2.0_f64.powi(29));
        assert_eq!(spacing_s, 2.0_f64.powi(-24));
        let before_s = 2.0_f64.powi(28);
        assert_eq!(before_s - f64::from_bits(before_s.to_bits() - 1), 2.0_f64.powi(-25));
        assert_eq!(MISSION_END_TOLERANCE_S, 4.0 * spacing_s);
        assert_eq!(MISSION_END_TOLERANCE_S, 2.0_f64.powi(-22));
        assert_eq!(generator.mission_end_tolerance_s, MISSION_END_TOLERANCE_S);
        // The end plus the tolerance is a 64-bit number, so the comparison is exact.
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
        let generator = GeneratorMmrtgF1::curiosity();
        let floor_s = generator.floor_dt_s().unwrap();
        let start_s = 4_085.0 * DAY_S;
        let mut state = GeneratorState::try_mission_time(&generator, start_s).unwrap();
        let mut frame = blank_frame();
        let mut plain_s = start_s;
        let count = 40_000u32;
        for _ in 0..count {
            step(&generator, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
            plain_s += floor_s;
            assert!(state.clock_carry_s.abs() <= 0.5 * spacing_at_the_end_s());
        }
        // The exact sum, as a whole number of steps of the 64-bit floor:
        // the product rounds once.
        let exact_s = start_s + f64::from(count) * floor_s;
        let spacing_s = f64::from_bits(exact_s.to_bits() + 1) - exact_s;
        assert!((state.t_s - exact_s).abs() <= spacing_s, "{} against {exact_s}", state.t_s);
        assert!((plain_s - exact_s).abs() > 16.0 * spacing_s, "the plain sum {plain_s}");
        assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
    }

    /// Runs that end on mission day 4,087 in decimal land every step at
    /// twelve step sizes and three lengths, and the step after is refused
    /// by name. A restart from the state before the last step lands that
    /// step too, and a single step from the decimal start one step before
    /// the end lands. Three of the runs start before 2^28 s, where the clock's
    /// spacing is half the spacing at the last day.
    #[test]
    fn a_decimal_run_to_the_last_day_lands_every_step() {
        let generator = GeneratorMmrtgF1::curiosity();
        let mut started_before_2_28_s = 0;
        let decimal = |tenths: u64| -> f64 {
            std::format!("{}.{}", tenths / 10, tenths % 10).parse().unwrap()
        };
        // step sizes in units of 0.1 s
        let steps_e1: [u64; 12] = [
            42_244_999, 10_000_001, 864_000, 864_005, 36_001, 10_003, 6_007, 1_001, 606, 101, 83,
            41,
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
                    GeneratorState::try_mission_time(&generator, decimal(start_e1)).unwrap();
                let mut frame = blank_frame();
                let mut before_last = state;
                for landed in 0..count {
                    before_last = state;
                    assert_eq!(
                        step(&generator, &mut state, &mut frame, NO_LOAD, dt_s),
                        Ok(()),
                        "dt {dt_s}: step {} of {count}",
                        landed + 1
                    );
                }
                assert!((state.t_s - MISSION_TIME_MAX_S).abs() <= MISSION_END_TOLERANCE_S);
                assert!(state.range_margin_s >= 0.0);
                if state.t_s >= MISSION_TIME_MAX_S {
                    assert_eq!(state.range_margin_s, 0.0);
                } else {
                    assert_eq!(state.range_margin_s, MISSION_TIME_MAX_S - state.t_s);
                }
                let before = state_bits(&state);
                let held = frame.to_bytes();
                assert_eq!(
                    step(&generator, &mut state, &mut frame, NO_LOAD, dt_s),
                    Err(StepReject::MissionTimeOutsideCard)
                );
                assert_eq!(state_bits(&state), before);
                assert_eq!(frame.to_bytes(), held);

                // A restart from the returned clock before the last step.
                let mut restart =
                    GeneratorState::try_mission_time(&generator, before_last.t_s).unwrap();
                assert_eq!(
                    step(&generator, &mut restart, &mut frame, NO_LOAD, dt_s),
                    Ok(()),
                    "restart from {} by {dt_s}",
                    before_last.t_s
                );
                assert!((restart.t_s - state.t_s).abs() <= MISSION_END_TOLERANCE_S);
            }
            // One first step from the decimal start one step before the end.
            if dt_e1 <= 3_531_168_000 {
                let start_s = decimal(3_531_168_000 - dt_e1);
                let mut single = GeneratorState::try_mission_time(&generator, start_s).unwrap();
                let mut frame = blank_frame();
                assert_eq!(
                    step(&generator, &mut single, &mut frame, NO_LOAD, dt_s),
                    Ok(()),
                    "single step from {start_s} by {dt_s}"
                );
            }
        }
        assert_eq!(started_before_2_28_s, 3);
    }

    /// The two cases that engine 0.6.0 answered differently by route, moved
    /// with the last day and held by the same comparison. The run of ten
    /// steps of 4,224,499.9 s from 310,871,801 s and the restart from its
    /// ninth state both land on the last day. Two steps at the floor whose
    /// end in decimal is 0.89 s past the last day, and one step to the same
    /// overshoot, are both refused by name.
    #[test]
    fn a_restart_and_the_run_straight_through_get_the_same_verdict() {
        let generator = GeneratorMmrtgF1::curiosity();
        let mut frame = blank_frame();
        let dt_s = 4_224_499.9;
        let mut straight = GeneratorState::try_mission_time(&generator, 310_871_801.0).unwrap();
        for _ in 0..9 {
            step(&generator, &mut straight, &mut frame, NO_LOAD, dt_s).unwrap();
        }
        let mut restart = GeneratorState::try_mission_time(&generator, straight.t_s).unwrap();
        step(&generator, &mut straight, &mut frame, NO_LOAD, dt_s).unwrap();
        step(&generator, &mut restart, &mut frame, NO_LOAD, dt_s).unwrap();
        assert!((straight.t_s - MISSION_TIME_MAX_S).abs() <= MISSION_END_TOLERANCE_S);
        assert!((restart.t_s - MISSION_TIME_MAX_S).abs() <= MISSION_END_TOLERANCE_S);
        // The decimal start and step of the last step alone land too.
        let mut single = GeneratorState::try_mission_time(&generator, 348_892_300.1).unwrap();
        step(&generator, &mut single, &mut frame, NO_LOAD, dt_s).unwrap();
        assert_eq!(single.t_s.to_bits(), MISSION_TIME_MAX_S.to_bits());

        let floor_s = generator.floor_dt_s().unwrap();
        assert_eq!(floor_s, 4.028797086407726);
        let mut two = GeneratorState::try_mission_time(&generator, 353_116_792.832_405_8).unwrap();
        step(&generator, &mut two, &mut frame, NO_LOAD, floor_s).unwrap();
        let held = state_bits(&two);
        assert_eq!(
            step(&generator, &mut two, &mut frame, NO_LOAD, floor_s),
            Err(StepReject::MissionTimeOutsideCard)
        );
        assert_eq!(state_bits(&two), held);
        let mut one = GeneratorState::try_mission_time(&generator, 353_116_796.861_202_9).unwrap();
        let held = state_bits(&one);
        assert_eq!(
            step(&generator, &mut one, &mut frame, NO_LOAD, floor_s),
            Err(StepReject::MissionTimeOutsideCard)
        );
        assert_eq!(state_bits(&one), held);
    }

    /// Requests whose end in decimal is exactly the last day land whether
    /// they are one step or five, and a restart from each returned state
    /// lands the rest; the same requests a microsecond later are refused at
    /// the last step by every route. Starts and steps to the thousandth of a
    /// second from a fixed generator of numbers.
    #[test]
    fn one_step_and_many_to_the_same_end_agree() {
        let generator = GeneratorMmrtgF1::curiosity();
        let end_e3: u64 = 353_116_800_000;
        let mut seed: u64 = 0x4087_0806_2012_0001;
        let mut next = || {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            seed >> 11
        };
        let floor_e3 = 4_029u64;
        let ceiling_e3 = 4_224_499_000u64;
        let mut cases = 0;
        for _ in 0..4_000 {
            let dt_e3 = floor_e3 + next() % (ceiling_e3 - floor_e3);
            let count = 1 + next() % 5;
            for (late_e3, lands) in [(0u64, true), (1u64, false)] {
                // late_e3 thousandths of a millisecond: one microsecond
                let start_e3 = end_e3 - count * dt_e3;
                let start_s = milli(start_e3) + if late_e3 == 1 { 1.0e-6 } else { 0.0 };
                let dt_s = milli(dt_e3);
                let mut straight = GeneratorState::try_mission_time(&generator, start_s).unwrap();
                let mut frame = blank_frame();
                for taken in 0..count {
                    let mut restart =
                        GeneratorState::try_mission_time(&generator, straight.t_s).unwrap();
                    let mut restart_result = Ok(());
                    for _ in taken..count {
                        restart_result =
                            step(&generator, &mut restart, &mut frame, NO_LOAD, dt_s);
                        if restart_result.is_err() {
                            break;
                        }
                    }
                    let got = step(&generator, &mut straight, &mut frame, NO_LOAD, dt_s);
                    if taken + 1 < count {
                        assert_eq!(got, Ok(()));
                    } else {
                        let expect = if lands { Ok(()) } else { Err(StepReject::MissionTimeOutsideCard) };
                        assert_eq!(got, expect, "{count} of {dt_s} from {start_s}");
                    }
                    let expect = if lands { Ok(()) } else { Err(StepReject::MissionTimeOutsideCard) };
                    assert_eq!(
                        restart_result, expect,
                        "restart after {taken} of {count} of {dt_s} from {start_s}"
                    );
                }
                cases += 1;
            }
        }
        assert_eq!(cases, 8_000);
    }

    /// A step that would end later than the last day by more than the
    /// tolerance is refused by name and nothing moves; every end inside it
    /// lands at its own value. The same for a step of 4.2e6 s, a day, an
    /// hour and the floor.
    #[test]
    fn a_step_beyond_the_mission_end_tolerance_is_refused_by_name() {
        let generator = GeneratorMmrtgF1::curiosity();
        let spacing_s = spacing_at_the_end_s();
        let floor_s = generator.floor_dt_s().unwrap();
        for dt_s in [4_224_499.9, DAY_S, 3_600.1, floor_s] {
            let mut landed_past_the_end = 0;
            let mut refused = 0;
            for spacings in 0..9u64 {
                let end_s = f64::from_bits(MISSION_TIME_MAX_S.to_bits() + spacings);
                // the clock whose sum with the step is this end
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
                let mut state = GeneratorState::try_mission_time(&generator, t_s).unwrap();
                let before = state_bits(&state);
                let mut frame = blank_frame();
                let result = step(&generator, &mut state, &mut frame, NO_LOAD, dt_s);
                if spacings <= 4 {
                    assert_eq!(result, Ok(()), "{spacings} spacings past by {dt_s}");
                    assert_eq!(state.t_s, end_s);
                    if spacings > 0 {
                        landed_past_the_end += 1;
                        assert_eq!(state.range_margin_s, 0.0);
                    }
                } else {
                    assert_eq!(result, Err(StepReject::MissionTimeOutsideCard));
                    assert_eq!(state_bits(&state), before);
                    assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
                    refused += 1;
                }
            }
            assert!(landed_past_the_end >= 1, "dt {dt_s}");
            assert!(refused >= 1, "dt {dt_s}");
            assert_eq!(MISSION_END_TOLERANCE_S, 4.0 * spacing_s);
            // A tenth of a second past is refused at every step size.
            let start_s = MISSION_TIME_MAX_S + 0.1 - dt_s;
            let mut state = GeneratorState::try_mission_time(&generator, start_s).unwrap();
            let mut frame = blank_frame();
            assert_eq!(
                step(&generator, &mut state, &mut frame, NO_LOAD, dt_s),
                Err(StepReject::MissionTimeOutsideCard)
            );
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
        let mut once = GeneratorState::try_landing(&generator).unwrap();
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
        assert!(within(once.thermal_power_w, 1787.8803829328674, 1.0e-12));
        assert!(within(once.electrical_power_w, 79.31989849770113, 1.0e-12));
        assert!(within(
            once.electrical_energy_j,
            32916859138.30031,
            1.0e-12
        ));
        assert!(within(once.heat_rejected_j, 627172132511.6413, 1.0e-12));

        // About one day from mission day 4,085, as steps at the card's floor
        // against one step of the same length. This is the finest comparison
        // the floor allows: 21,445 steps of 4.03 s.
        let floor_s = generator.floor_dt_s().unwrap();
        let count = 21_445u32;
        let span_s = f64::from(count) * floor_s;
        assert!(span_s > 86_390.0 && span_s < DAY_S, "span {span_s}");
        let start_s = 4_085.0 * DAY_S;
        let mut small = GeneratorState::try_mission_time(&generator, start_s).unwrap();
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
        // 0.34 W under the closed form.
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
            account_error > 1.1e-2 && account_error < 1.3e-2,
            "account {account_error}"
        );
        assert!(account_error > 10_000.0 * CONVERTER_ACCOUNT_LIMIT);
        let wrong_power_w =
            exact.thermal_power_w * (STABLE_EFFICIENCY + FADING_EFFICIENCY * wrong_fraction);
        let shortfall_w = closed_form_electrical_power_w(year_s) - wrong_power_w;
        assert!(
            shortfall_w > 0.33 && shortfall_w < 0.35,
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
        let generator = GeneratorMmrtgF1::curiosity();
        let mut frame = blank_frame();
        let held = frame.to_bytes();

        let mut refuse = |start_s: f64, dt_s: f64, expect: StepReject| {
            let mut state = GeneratorState::try_mission_time(&generator, start_s).unwrap();
            let saved = state_bits(&state);
            let got = step(&generator, &mut state, &mut frame, NO_LOAD, dt_s);
            assert_eq!(got, Err(expect), "from {start_s} s by {dt_s} s");
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            proof_intact(&frame);
        };
        refuse(MISSION_TIME_MAX_S, 0.1, StepReject::MissionTimeOutsideCard);
        refuse(
            4_086.0 * DAY_S,
            86_400.1,
            StepReject::MissionTimeOutsideCard,
        );
        refuse(
            0.0,
            MISSION_TIME_MAX_S + 0.1,
            StepReject::MissionTimeOutsideCard,
        );
        refuse(0.0, f64::MAX, StepReject::MissionTimeOutsideCard);
        refuse(0.0, 0.0, StepReject::NonPositiveDt);
        refuse(0.0, -1.0, StepReject::NonPositiveDt);
        refuse(0.0, f64::NAN, StepReject::NonFiniteInput);
        refuse(0.0, f64::INFINITY, StepReject::NonFiniteInput);
        refuse(0.0, f64::NEG_INFINITY, StepReject::NonFiniteInput);

        let mut state = GeneratorState::try_landing(&generator).unwrap();
        state.thermal_power_w = f64::NAN;
        let nan_bits = state_bits(&state);
        let got = step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S);
        assert_eq!(got, Err(StepReject::NonFiniteState));
        assert_eq!(state_bits(&state), nan_bits);
        assert_eq!(frame.to_bytes(), held);

        let mut state = GeneratorState::try_landing(&generator).unwrap();
        state.t_s = -1.0;
        let early_bits = state_bits(&state);
        let got = step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S);
        assert_eq!(got, Err(StepReject::MissionTimeOutsideCard));
        assert_eq!(state_bits(&state), early_bits);
        assert_eq!(frame.to_bytes(), held);

        let mut state = GeneratorState::try_mission_time(&generator, MISSION_TIME_MAX_S).unwrap();
        state.t_s = MISSION_TIME_MAX_S + 1.0;
        let late_bits = state_bits(&state);
        let got = step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S);
        assert_eq!(got, Err(StepReject::MissionTimeOutsideCard));
        assert_eq!(state_bits(&state), late_bits);
        assert_eq!(frame.to_bytes(), held);
        proof_intact(&frame);

        assert!(GeneratorState::try_mission_time(&generator, -1.0).is_none());
        assert!(GeneratorState::try_mission_time(&generator, 353_116_800.1).is_none());
        assert!(GeneratorState::try_mission_time(&generator, f64::NAN).is_none());
        assert!(GeneratorState::try_mission_time(&generator, f64::INFINITY).is_none());
        assert!(GeneratorState::try_mission_time(&generator, MISSION_TIME_MAX_S).is_some());

        let mut last = GeneratorState::try_mission_time(&generator, 4_086.0 * DAY_S).unwrap();
        step(&generator, &mut last, &mut frame, NO_LOAD, DAY_S).unwrap();
        assert_eq!(last.t_s.to_bits(), 353_116_800.0_f64.to_bits());
        assert_eq!(last.range_margin_s, 0.0);
        assert_eq!(frame.t.to_bits(), 353_116_800.0_f64.to_bits());
        proof_intact(&frame);
    }

    #[test]
    fn the_flag_is_only_a_flag() {
        let (generator, mut state) = landing();
        let mut frame = blank_frame();
        let mut day = 0;
        while day < 2_060 {
            step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
            day += 1;
        }
        assert_eq!(state.t_s.to_bits(), FITTING_SPAN_END_S.to_bits());
        assert!(!state.past_fitting_span);
        assert_eq!(frame.flags, 0);
        let at_the_end = GeneratorState::try_mission_time(&generator, FITTING_SPAN_END_S).unwrap();
        assert!(!at_the_end.past_fitting_span);

        step(&generator, &mut state, &mut frame, NO_LOAD, DAY_S).unwrap();
        assert!(state.past_fitting_span);
        assert_eq!(frame.flags, FLAG_PAST_FITTING_SPAN);
        assert!(state.residual_conservation_error < 1.0e-12);
        proof_intact(&frame);

        let mut just_past = at_the_end;
        let floor_s = generator.floor_dt_s().unwrap();
        step(&generator, &mut just_past, &mut frame, NO_LOAD, floor_s).unwrap();
        assert!(just_past.past_fitting_span);
        assert_eq!(frame.flags, FLAG_PAST_FITTING_SPAN);
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
        let mut partway = GeneratorState::try_mission_time(&generator, 2_061.0 * DAY_S).unwrap();
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
        assert!(partway.past_fitting_span);
        // Day 2,061, the reference row above.
        assert!(within(partway.thermal_power_w, 1868.0054040123432, 1.0e-12));
        assert!(within(
            partway.electrical_power_w,
            91.19620053159335,
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
        let lengths = [0.1, 1.0, DAY_S, SECONDS_PER_YEAR, 4_224_500.0];
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
            MISSION_TIME_MAX_S,
        );
        assert_eq!(got, Err(StepReject::MissionTimeOutsideCard));
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
        let card = GeneratorMmrtgF1::curiosity();
        let ceiling_s = card.gate_dt_s().unwrap();
        let floor_s = card.floor_dt_s().unwrap();
        assert_eq!(super::GATE_OVER_FLOOR.to_bits(), 1_048_576.0_f64.to_bits());
        assert_eq!(
            floor_s.to_bits(),
            (ceiling_s / super::GATE_OVER_FLOOR).to_bits()
        );
        assert!(floor_s > 4.02 && floor_s < 4.04, "floor {floor_s}");

        // Landing, and one day before the card's last day.
        let starts = [0.0, MISSION_TIME_MAX_S - DAY_S];
        let mut index = 0;
        while index < starts.len() {
            let mut state = GeneratorState::try_mission_time(&card, starts[index]).unwrap();
            let before = state;
            let mut frame = blank_frame();
            step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
            // The clock moves by the step: exactly from landing, and to
            // eight figures a day before the last.
            let moved_s = state.t_s - before.t_s;
            assert!(
                (moved_s - floor_s).abs() <= 1.0e-8 * floor_s,
                "the clock moved {moved_s} for a step of {floor_s}"
            );
            assert!(state.thermal_power_w != before.thermal_power_w);
            assert!(state.converter_fading_fraction != before.converter_fading_fraction);
            assert!(state.electrical_energy_j != before.electrical_energy_j);
            assert!(state.heat_rejected_j != before.heat_rejected_j);
            assert!(state.fading_electrical_energy_j != before.fading_electrical_energy_j);
            assert!(state.residual_conservation_error < 1.0e-12);
            assert!(state.converter_account_error < 1.0e-12);

            // A thousand more steps at the floor land: both ledgers hold an
            // honest step this small.
            let mut taken = 0;
            while taken < 1_000 {
                step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
                taken += 1;
            }
            assert!(state.residual_conservation_error < 1.0e-12);
            assert!(state.converter_account_error < 1.0e-12);
            index += 1;
        }
        let landed = GeneratorState::try_landing(&card).unwrap();
        let mut from_landing = landed;
        let mut frame = blank_frame();
        step(&card, &mut from_landing, &mut frame, NO_LOAD, floor_s).unwrap();
        assert_eq!(from_landing.t_s.to_bits(), floor_s.to_bits());

        // Under the floor the arithmetic the header describes takes over: at
        // the last day a step of 1e-8 s does not move the clock, and from
        // landing a step of 1e-7 s does not move the thermal power. The step
        // itself applies no floor, so both land.
        let stuck_s = MISSION_TIME_MAX_S + 1.0e-8;
        assert_eq!(stuck_s.to_bits(), MISSION_TIME_MAX_S.to_bits());
        let mut still = landed;
        step(&card, &mut still, &mut frame, NO_LOAD, 1.0e-7).unwrap();
        assert_eq!(still.thermal_power_w.to_bits(), landed.thermal_power_w.to_bits());
    }

    #[test]
    fn the_last_start_is_the_latest_the_smallest_step_can_use() {
        let card = GeneratorMmrtgF1::curiosity();
        let floor_s = card.floor_dt_s().unwrap();
        let last_s = card.last_start_mission_time_s().unwrap();
        assert!(last_s < MISSION_TIME_MAX_S);
        assert!(MISSION_TIME_MAX_S - last_s < 2.0 * floor_s);
        let mut frame = blank_frame();
        // The published top lands at the smallest step, by the comparison every step makes.
        let mut state = GeneratorState::try_mission_time(&card, last_s).unwrap();
        step(&card, &mut state, &mut frame, NO_LOAD, floor_s).unwrap();
        assert!(state.t_s <= MISSION_TIME_MAX_S + MISSION_END_TOLERANCE_S);
        // To every digit: the latest start is 353,116,795.971203148365020751953125 s,
        // and the floor step from it lands on the last day plus the tolerance.
        assert_eq!(last_s.to_bits(), 353_116_795.971_203_15_f64.to_bits());
        assert_eq!(
            state.t_s.to_bits(),
            (MISSION_TIME_MAX_S + MISSION_END_TOLERANCE_S).to_bits()
        );
        // The bound holds the tolerance: a start whose smallest step lands
        // within it is a start the bound includes.
        assert!(last_s >= 353_116_795.971_202_9);
        // The next number above is refused at the smallest step, and nothing moves.
        let above_s = f64::from_bits(last_s.to_bits() + 1);
        let mut state = GeneratorState::try_mission_time(&card, above_s).unwrap();
        let before = state;
        assert_eq!(
            step(&card, &mut state, &mut frame, NO_LOAD, floor_s),
            Err(StepReject::MissionTimeOutsideCard)
        );
        assert_eq!(state, before);
        // A start a second past the bound is refused at every step.
        let beyond_s = last_s + 1.0;
        for dt_s in [floor_s, 2.0 * floor_s, 100.0] {
            let mut state = GeneratorState::try_mission_time(&card, beyond_s).unwrap();
            let before = state;
            assert_eq!(
                step(&card, &mut state, &mut frame, NO_LOAD, dt_s),
                Err(StepReject::MissionTimeOutsideCard)
            );
            assert_eq!(state, before);
        }
        // The card's last day itself is still a state it can hold.
        assert!(GeneratorState::try_mission_time(&card, MISSION_TIME_MAX_S).is_some());
    }
}
