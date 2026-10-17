#![no_implicit_prelude]

//! The Great Britain synchronous area after an importing interconnector
//! trips, compared over the first 60 seconds.
//!
//! The specimen is the 50 Hz transmission system of England, Scotland and
//! Wales as one rotating mass. Measured by the system operator (National Grid
//! ESO, since 2024 the National Energy System Operator, NESO): the system
//! frequency once a second ("System Frequency", NESO Open Data Licence) and
//! the half-hourly estimate of the stored kinetic energy ("System Inertia",
//! column "Outturn Inertia", same licence). The size of each loss is the
//! interconnector's flow before and after the trip in Elexon's five-minute
//! generation by fuel type (BMRS dataset FUELINST). 41 trips of 693 to
//! 1402 MW between July 2017 and May 2026 were found by a rule; every third
//! in time order was held back. No measured value is stored in the card's
//! constants or its step. The tests of this file hold five measured
//! frequency samples of the opened trip of 18 April 2019, for the report
//! only, and that trip's inputs.
//!
//! The card is the aggregate swing equation with the kinetic energy written
//! exactly, demand's own relief with frequency, and one response channel
//! whose target is a function of the frequency itself, steeper below a knee:
//!
//! ```text
//! kinetic energy      E_k = H (f / 50)²                                   MW s
//! frequency           df/dt = (50² / (2 H f)) (−L + r + c_L D (f_s − f))  Hz per s
//! response target     g(f) = K_1 (50 − f) + K_2 max(0, f_knee − f)        MW
//! response            dr/dt = (g(f) − g(f_s) − r) / T                     MW per s
//! ```
//!
//! `H` is the kinetic energy stored at 50 Hz (the published inertia, MW s),
//! `D` the demand (MW), `f_s` the frequency at the start of the run and `L`
//! the power lost, held through the step. `H`, `D` and `f_s` are the area's
//! condition on the day: the caller supplies them at the start and they
//! travel in the state. Three constants are fitted,
//! [`RESPONSE_GAIN_MW_PER_HZ`] (`K_1`), [`RESPONSE_GAIN_BELOW_KNEE_MW_PER_HZ`]
//! (`K_2`) and [`RESPONSE_TIME_S`] (`T`): the least-squares fit of the
//! frequency itself, sample by sample over 60 s, to the 28 opened trips
//! (structure F of the protocol, `analysis/out/05_forecast_F.json`, key
//! `constants_all_opened`, rounded there to four significant figures). The
//! knee, 49.8 Hz, is taken as the knee of the Dynamic Containment service
//! and [`LOAD_RELIEF_PER_HZ`], 0.025 per Hz of demand, as demand's own
//! sensitivity; neither is fitted. The knee is from NESO's Response Services
//! Service Terms v6 and its Dynamic Containment testing guidelines of August
//! 2020; the sensitivity from National Grid's System Operability Framework
//! 2016, p. 70, and the NESO webinar of 12 March 2025. The second scoring
//! read them (`evidence/verification/second_party/V-32/NOTES.md`).
//!
//! The response the operator held on each day is not in the open data. The
//! card carries one response for nine years, and the scatter of the days and
//! the change of the system over those years are the error it shows.
//!
//! The pass line and where the result is. The protocol
//! (`evidence/research/grid_gb_interconnector_loss/validation_protocol.md`,
//! fixed before any model ran) asked of the depth of the fall and of the
//! steepest second a root-mean-square relative error of at most 0.25 and a
//! mean within 0.10, of the frequency near 60 s an rms error of at most
//! 0.06 Hz, a median rms over the trace of at most 0.05 Hz, and a depth error
//! below that of "depth is proportional to loss". In leave-one-out prediction
//! on the opened trips structure F met the steepest second (rms 0.154), the
//! trace (median 0.050) and beat the proportional rule (0.302 against 0.350),
//! and missed the depth line (0.302 against 0.25) and the 60 s line (0.097 Hz
//! against 0.06). No structure of the protocol met every line. This card is
//! the structure with the smallest depth error, built as the protocol says,
//! and on the opened trips it did not meet the mark.
//!
//! The 13 held-back trips were then run once through the engine, on
//! 2026-10-09. One was dropped before scoring by the protocol's quality rule
//! (2 September 2021: the recorded frequency moves 0.275 Hz in the ten
//! seconds before the fall). On the other 12 all seven lines were met: depth
//! rms 0.166 and mean −0.018, against 0.182 for the proportional rule on the
//! same trips; steepest second rms 0.103 and mean +0.043; frequency near 60 s
//! 0.046 Hz; median trace error 0.036 Hz. The twelve are an easier set than
//! the opened 28 (the proportional rule scores 0.18 on them and 0.35 on the
//! opened), the margin over that rule is small, and the errors follow the
//! calendar: too shallow before 2021, too deep after 2023. The held-back
//! trips are spent. `README.md` of the same folder has every trip.
//!
//! The compared range: starting frequency 49.85 to 50.15 Hz, inertia
//! 111,000 to 337,000 MW s, demand 18,163 to 40,829 MW, loss 693 to 1402 MW
//! (each the span of the opened trips, the starting frequency rounded outward
//! to 0.05 Hz), the first 60 s, one loss held from time zero. Every scored
//! trip lost 693 MW or more. The six trips of 500 to 692 MW inside the other
//! spans, among those the second scoring found that the scan had lost, were
//! run through the engine on 2026-10-10 by the protocol's lines and missed
//! three of the seven (depth rms 0.2512 against 0.25, and against 0.2509 for
//! the proportional rule; steepest second rms 0.273 against 0.25). No trip
//! under 500 MW or above 1402 MW was compared. Through engine 0.9.0 each
//! edge of that range was a refusal; it is now the range the card was
//! compared on, and a step outside it lands with a flag that says so
//! ([`FLAG_OUTSIDE_COMPARED_TRIPS`], [`FLAG_PAST_COMPARED_MINUTE`],
//! [`FLAG_BELOW_COMPARED_FREQUENCY`]). The equations and the ledger are the
//! same there.
//!
//! What stops the card is physical. The frequency stops it at 48.8 Hz, where
//! low-frequency demand disconnection begins and the area is no longer the
//! same system ([`StepReject::LowFrequencyDemandDisconnection`]), and above
//! 50.5 Hz, the statutory limit, where the card holds no over-frequency
//! response ([`StepReject::FrequencyAboveCard`]). The stored kinetic energy
//! must be above zero, since the rate of change of frequency divides by it
//! ([`StepReject::InertiaNotPositive`]); the demand cannot be negative
//! ([`StepReject::DemandNegative`]); and the power lost, a loss of infeed
//! when positive and a loss of demand when negative, cannot be larger in
//! size than the demand the area carries ([`StepReject::LossLargerThanDemand`]).
//! The lowest frequency compared with measurement is 49.55 Hz, the lowest
//! measured single-loss frequency among the 41 trips, 49.553 Hz, rounded
//! outward; the lowest among the trips the scan lost is 49.596 Hz, and the
//! lowest the card reached in any compared run is 49.563 Hz. Below the 49.8
//! Hz knee the response the grid held changed over the years. Past the first
//! minute the card holds the primary response only: the operator's
//! instructed actions and the slower reserves are not in it.
//!
//! The card leaves out the network (one frequency for the whole area), the
//! separate response services and their volumes on the day, generators that
//! trip as a consequence, the operator's instructed actions after the first
//! minute, an over-frequency response, and every other synchronous area.
//!
//! The ledger is the kinetic energy of the rotating mass against the power
//! imbalance integrated in time:
//!
//! ```text
//! E_k − E_k,anchor = −∫ L dt + ∫ r dt + c_L D ∫ (f_s − f) dt
//! energy_defect_mw_s          = E_k − E_k,anchor + W_lost − W_response − W_relief
//! residual_conservation_error = |energy_defect_mw_s| / max(W_lost + |W_response| + |W_relief|, 1 MW s)
//! ```
//!
//! The step is fixed-step fourth-order Runge–Kutta on the frequency, the
//! response and the two integrals. It advances the frequency by its own
//! differential equation and never evaluates `E_k`; the ledger evaluates
//! `E_k = H (f/50)²` from the landed frequency. That quantity is quadratic
//! in the frequency, so the step does not conserve it by construction: a
//! wrong coefficient, a wrong stage weight or an Euler step in its place
//! shows in the defect. The ledger shows the arithmetic and the integrator
//! are right. It cannot show the physics matches the grid.
//!
//! [`GridGb::gate_dt_s_at`] is the step ceiling of a run: [`GATE_PHASE_STEP`]
//! over the fastest rate the card can reach with that run's inertia and
//! demand, the largest of `1/T`, the relief pole `c c_L D` and the
//! swing-response frequency `sqrt(c (K_1 + K_2) / T)`, with `c = 50² / (2 H
//! f)` at 48.8 Hz, under the lowest frequency of the card. It is the step's
//! own arithmetic and holds for any positive inertia and any demand.
//! [`GridGb::gate_dt_s`] is the same rule at the corner of the compared
//! spans, the smallest inertia and the largest demand, 0.0208 s.
//! [`GridGb::floor_dt_s`] is [`FLOOR_DT_S`], 1e-6 s: at 60 s a step of that
//! size still moves the clock, and a loss of 1 MW on the largest compared
//! inertia moves the frequency by 3.7e-12 Hz, 500 times the spacing of
//! 64-bit numbers at 50. Where a run's ceiling falls under the floor, at an
//! inertia of about 1.3 MW s with the largest compared demand, no step is
//! one the card stands behind. The public step applies neither. The gateway
//! refuses a step outside them.
//!
//! The end of the compared minute and rounding. The clock is a running sum,
//! `t += dt` in 64-bit arithmetic, and each addition rounds by at most half
//! the spacing of 64-bit numbers between 32 s and 64 s, 2^-48 s. A run of
//! exactly 60 s in decimal can therefore reach its last step with a clock a
//! few hundred spacings late: 2,999 steps of 0.02 s read 59.98000000000378 s.
//! The comparison with the end of the minute carries [`GridGb::time_end_allowance_s`]:
//! [`TIME_END_ALLOWANCE_PER_STEP_S`], 2^-48 s, for each step a run of this
//! step takes to reach 60 s, and [`TIME_END_ALLOWANCE_EXTRA_STEPS`], two,
//! more for the step itself read from a decimal, which is within one part in
//! 2^53 of itself, over 60 s. It is never more than half the step, and the
//! comparison is with the 64-bit number nearest 60 s plus the allowance. At
//! the step ceiling it is 1.0e-11 s; at 1e-3 s, 2.1e-10 s; at the step
//! floor, 2.1e-7 s. The clock is stepped at its own value and nothing is
//! pulled back: a run that ends on 60 s in decimal can read up to the
//! allowance past 60 s without a flag, and a step that ends later lands
//! with [`FLAG_PAST_COMPARED_MINUTE`]. The run goes on until its steps run
//! out or a limit above stops it.
//!
//! The loss. Every trip the card was fitted to or compared with is one loss
//! stepped in at time zero and held, and the one measured trip that was
//! followed by a second loss, 22 December 2023, is the card's worst. The
//! loss is the load of each step, and the ledger books each step's own loss,
//! so a second loss, a link that comes back and a quiet period before the
//! trip are stepped by the same equations. The first loss is carried in the
//! state; a step whose loss is not it lands with
//! [`FLAG_OUTSIDE_COMPARED_TRIPS`]. It is not written to the frame.
//!
//! `core` only: no heap. The state is `Copy`. An accepted step writes time,
//! frequency, response, relief, rate of change of frequency, the lowest
//! landed frequency and its time, the net power and the conservation
//! residual into [`LastStateFrame64`], with the card's three flags. The
//! 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Nominal frequency of the area.
pub const NOMINAL_FREQUENCY_HZ: f64 = 50.0;
/// `K_1`, response per hertz below 50 Hz. Fitted, structure F.
pub const RESPONSE_GAIN_MW_PER_HZ: f64 = 2074.0;
/// `K_2`, further response per hertz below the knee. Fitted, structure F.
pub const RESPONSE_GAIN_BELOW_KNEE_MW_PER_HZ: f64 = 3136.0;
/// `T`, the lag of the response. Fitted, structure F.
pub const RESPONSE_TIME_S: f64 = 1.296;
/// The knee: taken as the frequency at which the operator's Dynamic
/// Containment service passes from 5 percent of its volume towards all of
/// it. Not fitted. Source: NESO, Response Services Service Terms v6, table
/// on p. 34, "Knee-point Frequency fa: fn +/- 0.2 Hz" at 5 percent of the
/// quantity, and the Dynamic Containment testing guidelines of August 2020.
/// It is that one service's knee, in force from October 2020; before it
/// there were static blocks at 49.7 and 49.6 Hz. With the knee left free the
/// fit put it at 49.978 Hz and predicted the opened trips no better.
pub const RESPONSE_KNEE_HZ: f64 = 49.8;
/// Demand's own fall with frequency, per hertz, as a fraction of demand:
/// taken as 2.5 percent per hertz. Not fitted. Source: National Grid,
/// System Operability Framework 2016, p. 70, "approximately 2.5% per Hertz",
/// and the NESO webinar on frequency risk modelling of 12 March 2025, "2.5%
/// /Hz as an empirical number". The documented range is 2.0 to 2.5 percent
/// per hertz of national demand.
pub const LOAD_RELIEF_PER_HZ: f64 = 0.025;
/// Smallest and largest starting frequency compared: the opened trips start
/// between 49.853 and 50.126 Hz; rounded outward to 0.05 Hz.
pub const START_FREQUENCY_MIN_HZ: f64 = 49.85;
pub const START_FREQUENCY_MAX_HZ: f64 = 50.15;
/// Smallest and largest stored kinetic energy at 50 Hz among the opened
/// trips, the inertia compared: 111 and 337 GVA s.
pub const INERTIA_MIN_MW_S: f64 = 111_000.0;
pub const INERTIA_MAX_MW_S: f64 = 337_000.0;
/// Smallest and largest demand among the opened trips, the demand compared.
pub const DEMAND_MIN_MW: f64 = 18_163.0;
pub const DEMAND_MAX_MW: f64 = 40_829.0;
/// Smallest loss among the scored trips, opened and held back alike. The
/// six trips of 500 to 692 MW that the scan lost missed three of the
/// protocol's seven lines when run through the engine on 2026-10-10.
pub const LOSS_MIN_MW: f64 = 693.0;
/// Largest loss among the opened trips (North Sea Link, 22 March 2022).
pub const LOSS_MAX_MW: f64 = 1402.0;
/// Low-frequency demand disconnection begins here.
pub const FREQUENCY_MIN_HZ: f64 = 48.8;
/// Upper statutory limit.
pub const FREQUENCY_MAX_HZ: f64 = 50.5;
/// The lowest frequency compared with measurement: the lowest measured
/// single-loss frequency among the 41 trips, 49.553 Hz, rounded outward. A
/// landing under it sets [`FLAG_BELOW_COMPARED_FREQUENCY`].
pub const FREQUENCY_COMPARED_MIN_HZ: f64 = 49.55;
/// The minute after the start the card was compared over. A landing past it
/// sets [`FLAG_PAST_COMPARED_MINUTE`].
pub const TIME_MAX_S: f64 = 60.0;
/// Half the spacing of 64-bit numbers between 32 s and 64 s, 2^-48 s: the
/// most one addition to the clock can round by. The comparison with
/// [`TIME_MAX_S`] allows this much for each step a run takes to reach it.
pub const TIME_END_ALLOWANCE_PER_STEP_S: f64 = 3.552_713_678_800_501e-15;
/// Steps added to the count in [`GridGb::time_end_allowance_s`], for the step
/// read from a decimal.
pub const TIME_END_ALLOWANCE_EXTRA_STEPS: f64 = 2.0;
/// The ledger's scale is never under this: one megawatt for one second.
pub const ENERGY_SCALE_FLOOR_MW_S: f64 = 1.0;
/// `residual_conservation_error` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-7;
/// Fraction of the fastest time scale used by [`GridGb::gate_dt_s`].
pub const GATE_PHASE_STEP: f64 = 0.02;
/// Smallest step the card stands behind.
pub const FLOOR_DT_S: f64 = 1.0e-6;

/// The run's start frequency, inertia or demand, or this step's loss, is
/// outside the spans of the compared trips (start 49.85 to 50.15 Hz, inertia
/// 111,000 to 337,000 MW s, demand 18,163 to 40,829 MW, loss 693 to 1402 MW),
/// or this step's loss is not the run's first: every compared trip was one
/// loss held from time zero. The step still lands.
pub const FLAG_OUTSIDE_COMPARED_TRIPS: u64 = 1 << 0;
/// The step lands later than 60 s after the start, by more than the rounding
/// allowance of the clock: past the minute the card was compared over. The
/// step still lands.
pub const FLAG_PAST_COMPARED_MINUTE: u64 = 1 << 1;
/// The landed frequency is under 49.55 Hz, the lowest frequency compared
/// with measurement. The step still lands.
pub const FLAG_BELOW_COMPARED_FREQUENCY: u64 = 1 << 2;

/// Why a step did not advance the area or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The power lost or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A constant is not this card's.
    InvalidCard,
    /// The stored kinetic energy at 50 Hz is zero or less: the rate of change of frequency divides by it.
    InertiaNotPositive,
    /// The demand is below zero.
    DemandNegative,
    /// The power lost is larger in size than the demand: the area cannot lose more infeed, or more demand, than it carries.
    LossLargerThanDemand,
    /// The clock is below zero.
    NegativeTime,
    /// The frequency, incoming or landing, is at or below 48.8 Hz, where low-frequency demand disconnection begins.
    LowFrequencyDemandDisconnection,
    /// The frequency, incoming or landing, is above 50.5 Hz, the upper statutory limit, where the card holds no over-frequency response.
    FrequencyAboveCard,
    /// The trial step created kinetic energy from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed kinetic energy.
    FirstLawBreach,
}

/// The power lost, held through the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLoad {
    pub loss_mw: f64,
}

/// The locked card.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGb {
    pub nominal_frequency_hz: f64,
    pub response_gain_mw_per_hz: f64,
    pub response_gain_below_knee_mw_per_hz: f64,
    pub response_time_s: f64,
    pub response_knee_hz: f64,
    pub load_relief_per_hz: f64,
    pub start_frequency_min_hz: f64,
    pub start_frequency_max_hz: f64,
    pub inertia_min_mw_s: f64,
    pub inertia_max_mw_s: f64,
    pub demand_min_mw: f64,
    pub demand_max_mw: f64,
    pub loss_min_mw: f64,
    pub loss_max_mw: f64,
    pub frequency_min_hz: f64,
    pub frequency_max_hz: f64,
    pub frequency_compared_min_hz: f64,
    pub time_max_s: f64,
    pub time_end_allowance_per_step_s: f64,
    pub time_end_allowance_extra_steps: f64,
    pub energy_scale_floor_mw_s: f64,
    pub residual_conservation_limit: f64,
    pub gate_phase_step: f64,
    pub floor_dt_s: f64,
}

impl GridGb {
    /// The card.
    #[must_use]
    pub fn interconnector_loss() -> Self {
        let card = Self {
            nominal_frequency_hz: NOMINAL_FREQUENCY_HZ,
            response_gain_mw_per_hz: RESPONSE_GAIN_MW_PER_HZ,
            response_gain_below_knee_mw_per_hz: RESPONSE_GAIN_BELOW_KNEE_MW_PER_HZ,
            response_time_s: RESPONSE_TIME_S,
            response_knee_hz: RESPONSE_KNEE_HZ,
            load_relief_per_hz: LOAD_RELIEF_PER_HZ,
            start_frequency_min_hz: START_FREQUENCY_MIN_HZ,
            start_frequency_max_hz: START_FREQUENCY_MAX_HZ,
            inertia_min_mw_s: INERTIA_MIN_MW_S,
            inertia_max_mw_s: INERTIA_MAX_MW_S,
            demand_min_mw: DEMAND_MIN_MW,
            demand_max_mw: DEMAND_MAX_MW,
            loss_min_mw: LOSS_MIN_MW,
            loss_max_mw: LOSS_MAX_MW,
            frequency_min_hz: FREQUENCY_MIN_HZ,
            frequency_max_hz: FREQUENCY_MAX_HZ,
            frequency_compared_min_hz: FREQUENCY_COMPARED_MIN_HZ,
            time_max_s: TIME_MAX_S,
            time_end_allowance_per_step_s: TIME_END_ALLOWANCE_PER_STEP_S,
            time_end_allowance_extra_steps: TIME_END_ALLOWANCE_EXTRA_STEPS,
            energy_scale_floor_mw_s: ENERGY_SCALE_FLOOR_MW_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_phase_step: GATE_PHASE_STEP,
            floor_dt_s: FLOOR_DT_S,
        };
        if card.is_valid() {
            card
        } else {
            ::core::panic!("grid_gb failed is_valid");
        }
    }

    /// Every copy equals its crate constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.nominal_frequency_hz.to_bits() == NOMINAL_FREQUENCY_HZ.to_bits()
            && self.response_gain_mw_per_hz.to_bits() == RESPONSE_GAIN_MW_PER_HZ.to_bits()
            && self.response_gain_below_knee_mw_per_hz.to_bits()
                == RESPONSE_GAIN_BELOW_KNEE_MW_PER_HZ.to_bits()
            && self.response_time_s.to_bits() == RESPONSE_TIME_S.to_bits()
            && self.response_knee_hz.to_bits() == RESPONSE_KNEE_HZ.to_bits()
            && self.load_relief_per_hz.to_bits() == LOAD_RELIEF_PER_HZ.to_bits()
            && self.start_frequency_min_hz.to_bits() == START_FREQUENCY_MIN_HZ.to_bits()
            && self.start_frequency_max_hz.to_bits() == START_FREQUENCY_MAX_HZ.to_bits()
            && self.inertia_min_mw_s.to_bits() == INERTIA_MIN_MW_S.to_bits()
            && self.inertia_max_mw_s.to_bits() == INERTIA_MAX_MW_S.to_bits()
            && self.demand_min_mw.to_bits() == DEMAND_MIN_MW.to_bits()
            && self.demand_max_mw.to_bits() == DEMAND_MAX_MW.to_bits()
            && self.loss_min_mw.to_bits() == LOSS_MIN_MW.to_bits()
            && self.loss_max_mw.to_bits() == LOSS_MAX_MW.to_bits()
            && self.frequency_min_hz.to_bits() == FREQUENCY_MIN_HZ.to_bits()
            && self.frequency_max_hz.to_bits() == FREQUENCY_MAX_HZ.to_bits()
            && self.frequency_compared_min_hz.to_bits() == FREQUENCY_COMPARED_MIN_HZ.to_bits()
            && self.time_max_s.to_bits() == TIME_MAX_S.to_bits()
            && self.time_end_allowance_per_step_s.to_bits()
                == TIME_END_ALLOWANCE_PER_STEP_S.to_bits()
            && self.time_end_allowance_extra_steps.to_bits()
                == TIME_END_ALLOWANCE_EXTRA_STEPS.to_bits()
            && self.energy_scale_floor_mw_s.to_bits() == ENERGY_SCALE_FLOOR_MW_S.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.gate_phase_step.to_bits() == GATE_PHASE_STEP.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
            && self.response_gain_mw_per_hz > 0.0
            && self.response_gain_below_knee_mw_per_hz > 0.0
            && self.response_time_s > 0.0
            && self.load_relief_per_hz > 0.0
            && self.inertia_min_mw_s > 0.0
            && self.frequency_min_hz > 0.0
    }

    /// The step ceiling at the corner of the compared spans, the smallest
    /// inertia and the largest demand, seconds.
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        self.gate_dt_s_at(self.inertia_min_mw_s, self.demand_max_mw)
    }

    /// The step ceiling of a run with this inertia and demand, seconds: the
    /// same rule at that run's own fastest rate. `None` for an inertia that
    /// is not above zero or a demand below zero.
    #[must_use]
    pub fn gate_dt_s_at(&self, inertia_mw_s: f64, demand_mw: f64) -> Option<f64> {
        if !self.is_valid() || !(inertia_mw_s > 0.0) || !(demand_mw >= 0.0) {
            return None;
        }
        let c = self.nominal_frequency_hz * self.nominal_frequency_hz
            / (2.0 * inertia_mw_s * self.frequency_min_hz);
        let lag_rate = 1.0 / self.response_time_s;
        let relief_rate = c * self.load_relief_per_hz * demand_mw;
        let swing_rate = (c
            * (self.response_gain_mw_per_hz + self.response_gain_below_knee_mw_per_hz)
            / self.response_time_s)
            .sqrt();
        let mut fastest = lag_rate;
        if relief_rate > fastest {
            fastest = relief_rate;
        }
        if swing_rate > fastest {
            fastest = swing_rate;
        }
        if fastest.is_finite() && fastest > 0.0 {
            Some(self.gate_phase_step / fastest)
        } else {
            None
        }
    }

    /// How far past [`TIME_MAX_S`] the running sum of the clock may read at
    /// the end of a step of `dt_s`, seconds:
    /// [`TIME_END_ALLOWANCE_PER_STEP_S`] for each step a run of this step
    /// takes to reach the end, and [`TIME_END_ALLOWANCE_EXTRA_STEPS`] more;
    /// never more than half the step. Zero for a step that is not positive.
    #[must_use]
    pub fn time_end_allowance_s(&self, dt_s: f64) -> f64 {
        if !(dt_s > 0.0) {
            return 0.0;
        }
        let steps = self.time_max_s / dt_s + self.time_end_allowance_extra_steps;
        let allowance_s = steps * self.time_end_allowance_per_step_s;
        if allowance_s < 0.5 * dt_s { allowance_s } else { 0.5 * dt_s }
    }

    /// The step floor, seconds.
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        if self.floor_dt_s.is_finite() && self.floor_dt_s > 0.0 {
            Some(self.floor_dt_s)
        } else {
            None
        }
    }
}

/// The area. `Copy`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridState {
    pub t_s: f64,
    pub frequency_hz: f64,
    /// Response delivered since the start, MW.
    pub response_mw: f64,
    /// Frequency at the start of the run, Hz. Held.
    pub start_frequency_hz: f64,
    /// Kinetic energy stored at 50 Hz, MW s. Held.
    pub inertia_mw_s: f64,
    /// Demand, MW. Held.
    pub demand_mw: f64,
    /// Demand's relief at the landed frequency, MW.
    pub relief_mw: f64,
    /// Rate of change of frequency at the landed state under the held loss, Hz per s.
    pub frequency_rate_hz_per_s: f64,
    /// −loss + response + relief at the landed state, MW.
    pub net_power_mw: f64,
    /// Kinetic energy at the landed frequency, MW s.
    pub kinetic_energy_mw_s: f64,
    /// Kinetic energy at the start, MW s.
    pub kinetic_energy_anchor_mw_s: f64,
    /// ∫ loss dt, MW s.
    pub energy_lost_mw_s: f64,
    /// ∫ response dt, MW s.
    pub energy_response_mw_s: f64,
    /// ∫ (start frequency − frequency) dt, Hz s. The relief energy is this times `c_L D`.
    pub frequency_deficit_integral_hz_s: f64,
    pub energy_defect_mw_s: f64,
    pub residual_conservation_error: f64,
    /// Lowest landed frequency, and the time of it.
    pub lowest_frequency_hz: f64,
    pub lowest_time_s: f64,
    /// The loss of the run's first step, MW: a later step under another loss
    /// sets [`FLAG_OUTSIDE_COMPARED_TRIPS`]. Zero before the first step. Not
    /// written to the frame.
    pub run_loss_mw: f64,
}

impl GridState {
    /// The area in balance at `start_frequency_hz`, with the day's inertia
    /// and demand. `None` past a limit of the card: a start frequency at or
    /// below 48.8 Hz or above 50.5 Hz, an inertia not above zero or a demand
    /// below zero.
    #[must_use]
    pub fn try_start(
        card: &GridGb,
        start_frequency_hz: f64,
        inertia_mw_s: f64,
        demand_mw: f64,
    ) -> Option<Self> {
        if !card.is_valid()
            || !start_frequency_hz.is_finite()
            || !inertia_mw_s.is_finite()
            || !demand_mw.is_finite()
            || start_frequency_hz <= card.frequency_min_hz
            || start_frequency_hz > card.frequency_max_hz
            || !(inertia_mw_s > 0.0)
            || !(demand_mw >= 0.0)
        {
            return None;
        }
        let kinetic = kinetic_energy_mw_s(card, inertia_mw_s, start_frequency_hz);
        Some(Self {
            t_s: 0.0,
            frequency_hz: start_frequency_hz,
            response_mw: 0.0,
            start_frequency_hz,
            inertia_mw_s,
            demand_mw,
            relief_mw: 0.0,
            frequency_rate_hz_per_s: 0.0,
            net_power_mw: 0.0,
            kinetic_energy_mw_s: kinetic,
            kinetic_energy_anchor_mw_s: kinetic,
            energy_lost_mw_s: 0.0,
            energy_response_mw_s: 0.0,
            frequency_deficit_integral_hz_s: 0.0,
            energy_defect_mw_s: 0.0,
            residual_conservation_error: 0.0,
            lowest_frequency_hz: start_frequency_hz,
            lowest_time_s: 0.0,
            run_loss_mw: 0.0,
        })
    }
}

/// One step. On `Err` the state and the frame are exactly as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    card: &GridGb,
    state: &mut GridState,
    frame: &mut LastStateFrame64,
    load: GridLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(card, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    card: &GridGb,
    state: &mut GridState,
    frame: &mut LastStateFrame64,
    load: GridLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(card, state, load, dt_s)?;
    let advanced = rk4(card, state, load.loss_mw, &state_vector(state), dt_s);
    commit(card, state, frame, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut GridState) {
    state.t_s = normal_or_zero(state.t_s);
    state.frequency_hz = normal_or_zero(state.frequency_hz);
    state.response_mw = normal_or_zero(state.response_mw);
    state.start_frequency_hz = normal_or_zero(state.start_frequency_hz);
    state.inertia_mw_s = normal_or_zero(state.inertia_mw_s);
    state.demand_mw = normal_or_zero(state.demand_mw);
    state.relief_mw = normal_or_zero(state.relief_mw);
    state.frequency_rate_hz_per_s = normal_or_zero(state.frequency_rate_hz_per_s);
    state.net_power_mw = normal_or_zero(state.net_power_mw);
    state.kinetic_energy_mw_s = normal_or_zero(state.kinetic_energy_mw_s);
    state.kinetic_energy_anchor_mw_s = normal_or_zero(state.kinetic_energy_anchor_mw_s);
    state.energy_lost_mw_s = normal_or_zero(state.energy_lost_mw_s);
    state.energy_response_mw_s = normal_or_zero(state.energy_response_mw_s);
    state.frequency_deficit_integral_hz_s = normal_or_zero(state.frequency_deficit_integral_hz_s);
    state.energy_defect_mw_s = normal_or_zero(state.energy_defect_mw_s);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.lowest_frequency_hz = normal_or_zero(state.lowest_frequency_hz);
    state.lowest_time_s = normal_or_zero(state.lowest_time_s);
    state.run_loss_mw = normal_or_zero(state.run_loss_mw);
}

fn validate(
    card: &GridGb,
    state: &GridState,
    load: GridLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.loss_mw.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !card.is_valid() {
        return Err(StepReject::InvalidCard);
    }
    if !(state.inertia_mw_s > 0.0) {
        return Err(StepReject::InertiaNotPositive);
    }
    if !(state.demand_mw >= 0.0) {
        return Err(StepReject::DemandNegative);
    }
    if load.loss_mw.abs() > state.demand_mw {
        return Err(StepReject::LossLargerThanDemand);
    }
    if !(state.t_s >= 0.0) {
        return Err(StepReject::NegativeTime);
    }
    if state.start_frequency_hz <= card.frequency_min_hz {
        return Err(StepReject::LowFrequencyDemandDisconnection);
    }
    if state.start_frequency_hz > card.frequency_max_hz {
        return Err(StepReject::FrequencyAboveCard);
    }
    if state.frequency_hz <= card.frequency_min_hz {
        return Err(StepReject::LowFrequencyDemandDisconnection);
    }
    if state.frequency_hz > card.frequency_max_hz {
        return Err(StepReject::FrequencyAboveCard);
    }
    Ok(())
}

fn commit(
    card: &GridGb,
    state: &mut GridState,
    frame: &mut LastStateFrame64,
    load: GridLoad,
    dt_s: f64,
    advanced: [f64; 4],
) -> Result<(), StepReject> {
    let saved = *state;
    let mut index = 0;
    while index < 4 {
        if !advanced[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    if advanced[0] <= card.frequency_min_hz {
        return Err(StepReject::LowFrequencyDemandDisconnection);
    }
    if advanced[0] > card.frequency_max_hz {
        return Err(StepReject::FrequencyAboveCard);
    }
    let flags = compared_flags(card, &saved, load, dt_s, advanced[0]);
    if state.t_s == 0.0 {
        state.run_loss_mw = load.loss_mw;
    }
    state.t_s += dt_s;
    state.frequency_hz = advanced[0];
    state.response_mw = advanced[1];
    state.energy_response_mw_s = advanced[2];
    state.frequency_deficit_integral_hz_s = advanced[3];
    state.energy_lost_mw_s += load.loss_mw * dt_s;
    let relief_gain = card.load_relief_per_hz * state.demand_mw;
    state.relief_mw = relief_gain * (state.start_frequency_hz - state.frequency_hz);
    state.net_power_mw = -load.loss_mw + state.response_mw + state.relief_mw;
    state.frequency_rate_hz_per_s = rates(card, state, load.loss_mw, &state_vector(state))[0];
    state.kinetic_energy_mw_s = kinetic_energy_mw_s(card, state.inertia_mw_s, state.frequency_hz);
    let energy_relief = relief_gain * state.frequency_deficit_integral_hz_s;
    state.energy_defect_mw_s = state.kinetic_energy_mw_s - state.kinetic_energy_anchor_mw_s
        + state.energy_lost_mw_s
        - state.energy_response_mw_s
        - energy_relief;
    let mut scale =
        state.energy_lost_mw_s + state.energy_response_mw_s.abs() + energy_relief.abs();
    if !(scale > card.energy_scale_floor_mw_s) {
        scale = card.energy_scale_floor_mw_s;
    }
    state.residual_conservation_error = state.energy_defect_mw_s.abs() / scale;
    if state.frequency_hz < state.lowest_frequency_hz {
        state.lowest_frequency_hz = state.frequency_hz;
        state.lowest_time_s = state.t_s;
    }
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > card.residual_conservation_limit {
        let created = state.energy_defect_mw_s > 0.0;
        *state = saved;
        return Err(if created {
            StepReject::SpontaneousGeneration
        } else {
            StepReject::FirstLawBreach
        });
    }
    publish(state, frame, flags);
    Ok(())
}

/// The flags of a step from `before` under `load` that lands at
/// `landed_frequency_hz`: what of it lies outside the compared trips.
fn compared_flags(
    card: &GridGb,
    before: &GridState,
    load: GridLoad,
    dt_s: f64,
    landed_frequency_hz: f64,
) -> u64 {
    let loss_changed = before.t_s != 0.0 && load.loss_mw != before.run_loss_mw;
    let outside = before.start_frequency_hz < card.start_frequency_min_hz
        || before.start_frequency_hz > card.start_frequency_max_hz
        || before.inertia_mw_s < card.inertia_min_mw_s
        || before.inertia_mw_s > card.inertia_max_mw_s
        || before.demand_mw < card.demand_min_mw
        || before.demand_mw > card.demand_max_mw
        || load.loss_mw < card.loss_min_mw
        || load.loss_mw > card.loss_max_mw
        || loss_changed;
    let late = !(before.t_s + dt_s <= card.time_max_s + card.time_end_allowance_s(dt_s));
    let low = landed_frequency_hz < card.frequency_compared_min_hz;
    let mut flags = 0;
    if outside {
        flags |= FLAG_OUTSIDE_COMPARED_TRIPS;
    }
    if late {
        flags |= FLAG_PAST_COMPARED_MINUTE;
    }
    if low {
        flags |= FLAG_BELOW_COMPARED_FREQUENCY;
    }
    flags
}

fn kinetic_energy_mw_s(card: &GridGb, inertia_mw_s: f64, frequency_hz: f64) -> f64 {
    let ratio = frequency_hz / card.nominal_frequency_hz;
    inertia_mw_s * ratio * ratio
}

fn response_target_mw(card: &GridGb, frequency_hz: f64) -> f64 {
    let below_knee = card.response_knee_hz - frequency_hz;
    let knee_part = if below_knee > 0.0 { below_knee } else { 0.0 };
    card.response_gain_mw_per_hz * (card.nominal_frequency_hz - frequency_hz)
        + card.response_gain_below_knee_mw_per_hz * knee_part
}

fn state_vector(state: &GridState) -> [f64; 4] {
    [
        state.frequency_hz,
        state.response_mw,
        state.energy_response_mw_s,
        state.frequency_deficit_integral_hz_s,
    ]
}

/// Rates of frequency, response, ∫ response and ∫ (start − frequency).
fn rates(card: &GridGb, state: &GridState, loss_mw: f64, y: &[f64; 4]) -> [f64; 4] {
    let deficit_hz = state.start_frequency_hz - y[0];
    let relief_mw = card.load_relief_per_hz * state.demand_mw * deficit_hz;
    let net_mw = -loss_mw + y[1] + relief_mw;
    let target_mw =
        response_target_mw(card, y[0]) - response_target_mw(card, state.start_frequency_hz);
    [
        card.nominal_frequency_hz * card.nominal_frequency_hz / (2.0 * state.inertia_mw_s * y[0])
            * net_mw,
        (target_mw - y[1]) / card.response_time_s,
        y[1],
        deficit_hz,
    ]
}

fn advance(y: &[f64; 4], k: &[f64; 4], scale: f64) -> [f64; 4] {
    let mut out = [0.0; 4];
    let mut index = 0;
    while index < 4 {
        out[index] = y[index] + scale * k[index];
        index += 1;
    }
    out
}

fn rk4(card: &GridGb, state: &GridState, loss_mw: f64, y: &[f64; 4], dt_s: f64) -> [f64; 4] {
    let k1 = rates(card, state, loss_mw, y);
    let k2 = rates(card, state, loss_mw, &advance(y, &k1, 0.5 * dt_s));
    let k3 = rates(card, state, loss_mw, &advance(y, &k2, 0.5 * dt_s));
    let k4 = rates(card, state, loss_mw, &advance(y, &k3, dt_s));
    let mut out = [0.0; 4];
    let mut index = 0;
    while index < 4 {
        out[index] =
            y[index] + dt_s / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    out
}

fn publish(state: &GridState, frame: &mut LastStateFrame64, flags: u64) {
    frame.t = state.t_s;
    frame.pos[0] = state.frequency_hz as f32;
    frame.pos[1] = state.response_mw as f32;
    frame.pos[2] = state.relief_mw as f32;
    frame.vel[0] = state.frequency_rate_hz_per_s as f32;
    frame.vel[1] = state.lowest_frequency_hz as f32;
    frame.vel[2] = state.lowest_time_s as f32;
    frame.force_torque = state.net_power_mw as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = flags;
}

fn state_is_finite(state: &GridState) -> bool {
    state.t_s.is_finite()
        && state.frequency_hz.is_finite()
        && state.response_mw.is_finite()
        && state.start_frequency_hz.is_finite()
        && state.inertia_mw_s.is_finite()
        && state.demand_mw.is_finite()
        && state.relief_mw.is_finite()
        && state.frequency_rate_hz_per_s.is_finite()
        && state.net_power_mw.is_finite()
        && state.kinetic_energy_mw_s.is_finite()
        && state.kinetic_energy_anchor_mw_s.is_finite()
        && state.energy_lost_mw_s.is_finite()
        && state.energy_response_mw_s.is_finite()
        && state.frequency_deficit_integral_hz_s.is_finite()
        && state.energy_defect_mw_s.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.lowest_frequency_hz.is_finite()
        && state.lowest_time_s.is_finite()
        && state.run_loss_mw.is_finite()
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
        FLAG_BELOW_COMPARED_FREQUENCY, FLAG_OUTSIDE_COMPARED_TRIPS, FLAG_PAST_COMPARED_MINUTE,
        FLOOR_DT_S, FREQUENCY_COMPARED_MIN_HZ, FREQUENCY_MAX_HZ, FREQUENCY_MIN_HZ, GridGb, GridLoad,
        GridState, LOSS_MAX_MW, LOSS_MIN_MW, RESIDUAL_CONSERVATION_LIMIT,
        RESPONSE_GAIN_BELOW_KNEE_MW_PER_HZ, RESPONSE_GAIN_MW_PER_HZ, RESPONSE_KNEE_HZ,
        RESPONSE_TIME_S, StepReject, TIME_END_ALLOWANCE_EXTRA_STEPS, TIME_END_ALLOWANCE_PER_STEP_S,
        TIME_MAX_S, advance, commit, rates, rk4, state_vector, step, validate,
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

    fn start(frequency_hz: f64, inertia_mw_s: f64, demand_mw: f64) -> (GridGb, GridState) {
        let card = GridGb::interconnector_loss();
        let state = GridState::try_start(&card, frequency_hz, inertia_mw_s, demand_mw).unwrap();
        (card, state)
    }

    fn load(loss_mw: f64) -> GridLoad {
        GridLoad { loss_mw }
    }

    fn ceiling() -> f64 {
        GridGb::interconnector_loss().gate_dt_s().unwrap()
    }

    /// The trip of 18 April 2019 (rank 5, opened): 996 MW, 241 GVA s,
    /// 27,132 MW, start 49.942 Hz. Inputs only.
    const RANK_5: (f64, f64, f64, f64) = (49.942, 241_000.0, 27_132.0, 996.0);

    fn run(case: (f64, f64, f64, f64), dt_s: f64, steps: usize) -> (GridState, f64) {
        let (card, mut state) = start(case.0, case.1, case.2);
        let mut frame = blank_frame();
        let mut worst = 0.0;
        let mut index = 0;
        while index < steps {
            step(&card, &mut state, &mut frame, load(case.3), dt_s).unwrap();
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
            index += 1;
        }
        (state, worst)
    }

    #[test]
    fn constants_are_locked() {
        let card = GridGb::interconnector_loss();
        assert!(card.is_valid());
        assert_eq!(RESPONSE_GAIN_MW_PER_HZ, 2074.0);
        assert_eq!(RESPONSE_GAIN_BELOW_KNEE_MW_PER_HZ, 3136.0);
        assert_eq!(RESPONSE_TIME_S, 1.296);
        assert_eq!(RESPONSE_KNEE_HZ, 49.8);
        let mut altered = card;
        altered.response_time_s = 1.2960000000000003;
        assert!(!altered.is_valid());
        let mut altered = card;
        altered.loss_max_mw = 2000.0;
        assert!(!altered.is_valid());
        let (_, mut state) = start(50.0, 200_000.0, 30_000.0);
        let mut frame = blank_frame();
        assert_eq!(
            step(&altered, &mut state, &mut frame, load(1000.0), 0.01),
            Err(StepReject::InvalidCard)
        );
    }

    /// The start is in balance: with no loss the integrator leaves it where
    /// it is, and a step of no loss lands there, flagged as outside the
    /// compared trips.
    #[test]
    fn the_start_is_in_balance_and_a_run_of_no_loss_stays_there() {
        let (card, mut state) = start(50.05, 200_000.0, 30_000.0);
        let mut y = state_vector(&state);
        let mut index = 0;
        while index < 1000 {
            y = rk4(&card, &state, 0.0, &y, 0.02);
            index += 1;
        }
        assert_eq!(y, state_vector(&state));
        let mut frame = blank_frame();
        for _ in 0..100 {
            step(&card, &mut state, &mut frame, load(0.0), 0.02).unwrap();
            assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_TRIPS);
        }
        assert_eq!(state.frequency_hz, 50.05);
        assert_eq!(state.residual_conservation_error, 0.0);
    }

    /// The loss compared was 693 to 1402 MW. Each end lands with no flag and
    /// the next 64-bit number outside lands with the flag; so do the six
    /// losses of 500 to 692 MW that missed three of the lines, and losses down
    /// to zero and up to the demand. A loss larger in size than the demand is
    /// refused by name and nothing moves.
    #[test]
    fn a_loss_outside_the_compared_span_lands_with_the_flag() {
        assert_eq!(LOSS_MIN_MW, 693.0);
        let (card, state0) = start(50.0, 200_000.0, 30_000.0);
        for loss in [LOSS_MIN_MW, LOSS_MAX_MW] {
            let mut s = state0;
            let mut frame = blank_frame();
            step(&card, &mut s, &mut frame, load(loss), 0.01).unwrap();
            assert_eq!(s.run_loss_mw, loss);
            assert_eq!(frame.flags, 0);
        }
        let below = f64::from_bits(LOSS_MIN_MW.to_bits() - 1);
        let above = f64::from_bits(LOSS_MAX_MW.to_bits() + 1);
        for loss in [below, above, 692.0, 634.0, 572.0, 545.0, 529.0, 500.0, 300.0, 1.0, 0.0, -0.0, 1.0e-300, -500.0, 3_000.0, 30_000.0, -30_000.0] {
            let mut s = state0;
            let mut frame = blank_frame();
            step(&card, &mut s, &mut frame, load(loss), 0.01).unwrap();
            assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED_TRIPS, FLAG_OUTSIDE_COMPARED_TRIPS, "{loss}");
            assert!(s.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        }
        for loss in [30_000.000_000_000_004, 31_000.0, -30_001.0, 1.0e300] {
            let mut s = state0;
            let mut frame = blank_frame();
            assert_eq!(
                step(&card, &mut s, &mut frame, load(loss), 0.01),
                Err(StepReject::LossLargerThanDemand),
                "{loss}"
            );
            assert_eq!(s, state0);
            assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
        }
        let mut altered = card;
        altered.loss_min_mw = 0.0;
        assert!(!altered.is_valid());
    }

    #[test]
    fn the_first_instant_falls_at_the_swing_equation_rate() {
        let (card, mut state) = start(50.0, 200_000.0, 30_000.0);
        let mut frame = blank_frame();
        step(&card, &mut state, &mut frame, load(1000.0), FLOOR_DT_S).unwrap();
        let rate = (state.frequency_hz - 50.0) / FLOOR_DT_S;
        // −L f0 / (2 H) = −1000 · 50 / 400000 = −0.125 Hz per s
        assert!((rate + 0.125).abs() < 1.0e-6, "{rate}");
    }

    #[test]
    fn every_refusal_is_named_and_leaves_state_and_frame_as_they_were() {
        let (card, state0) = start(50.0, 200_000.0, 30_000.0);
        let refused = |mut state: GridState, loss: f64, dt: f64, want: StepReject| {
            let before = state;
            let mut frame = blank_frame();
            assert_eq!(step(&card, &mut state, &mut frame, load(loss), dt), Err(want));
            assert_eq!(state, before);
            assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
        };
        refused(state0, f64::NAN, 0.01, StepReject::NonFiniteInput);
        refused(state0, 1000.0, f64::INFINITY, StepReject::NonFiniteInput);
        refused(state0, 1000.0, 0.0, StepReject::NonPositiveDt);
        refused(state0, 1000.0, -0.01, StepReject::NonPositiveDt);
        refused(state0, -30_000.5, 0.01, StepReject::LossLargerThanDemand);
        refused(state0, 30_000.5, 0.01, StepReject::LossLargerThanDemand);
        let mut s = state0;
        s.t_s = -0.5;
        refused(s, 1000.0, 0.01, StepReject::NegativeTime);
        let mut s = state0;
        s.start_frequency_hz = 48.8;
        refused(s, 1000.0, 0.01, StepReject::LowFrequencyDemandDisconnection);
        let mut s = state0;
        s.start_frequency_hz = 50.500_000_000_000_01;
        refused(s, 1000.0, 0.01, StepReject::FrequencyAboveCard);
        for inertia in [0.0, -0.0, -1.0, -111_000.0] {
            let mut s = state0;
            s.inertia_mw_s = inertia;
            refused(s, 1000.0, 0.01, StepReject::InertiaNotPositive);
        }
        for demand in [-1.0e-300, -1.0, -40_830.0] {
            let mut s = state0;
            s.demand_mw = demand;
            refused(s, 0.0, 0.01, StepReject::DemandNegative);
        }
        let mut s = state0;
        s.frequency_hz = 48.8;
        refused(s, 1000.0, 0.01, StepReject::LowFrequencyDemandDisconnection);
        let mut s = state0;
        s.frequency_hz = 50.6;
        refused(s, 1000.0, 0.01, StepReject::FrequencyAboveCard);
        let mut s = state0;
        s.response_mw = f64::NAN;
        let mut frame = blank_frame();
        assert_eq!(
            step(&card, &mut s, &mut frame, load(1000.0), 0.01),
            Err(StepReject::NonFiniteState)
        );
        // the largest loss is accepted
        let mut s = state0;
        let mut frame = blank_frame();
        step(&card, &mut s, &mut frame, load(LOSS_MAX_MW), 0.01).unwrap();
        assert!(GridState::try_start(&card, 49.84, 200_000.0, 30_000.0).is_some());
        assert!(GridState::try_start(&card, 50.0, 338_000.0, 30_000.0).is_some());
        assert!(GridState::try_start(&card, 50.0, 200_000.0, 18_000.0).is_some());
        assert!(GridState::try_start(&card, FREQUENCY_MIN_HZ, 200_000.0, 30_000.0).is_none());
        assert!(GridState::try_start(&card, FREQUENCY_MAX_HZ, 200_000.0, 30_000.0).is_some());
        assert!(GridState::try_start(&card, 50.6, 200_000.0, 30_000.0).is_none());
        assert!(GridState::try_start(&card, 50.0, 0.0, 30_000.0).is_none());
        assert!(GridState::try_start(&card, 50.0, 200_000.0, -1.0).is_none());
        assert!(GridState::try_start(&card, f64::NAN, 200_000.0, 30_000.0).is_none());
    }

    #[test]
    fn a_fall_that_lands_at_the_disconnection_frequency_is_refused_on_that_step() {
        // A landing at or below 48.8 Hz is named as disconnection, and one
        // above 50.5 Hz as above the card; nothing moves.
        let (card, mut state) = start(49.85, 111_000.0, 18_163.0);
        let before = state;
        let mut frame = blank_frame();
        for (landing, want) in [
            (48.8, StepReject::LowFrequencyDemandDisconnection),
            (48.7, StepReject::LowFrequencyDemandDisconnection),
            (50.6, StepReject::FrequencyAboveCard),
        ] {
            validate(&card, &state, load(LOSS_MAX_MW), 0.02).unwrap();
            let advanced = [landing, 0.0, 0.0, 0.0];
            assert_eq!(
                commit(&card, &mut state, &mut frame, load(LOSS_MAX_MW), 0.02, advanced),
                Err(want),
                "{landing}"
            );
            assert_eq!(state, before);
            assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
        }
    }

    /// The deepest corner of the compared spans, the lowest start, the
    /// smallest inertia, the smallest demand and the largest loss, was never
    /// compared. Its fall passes under 49.55 Hz with the flag that says so,
    /// the ledger closing every step, and it settles above 48.8 Hz. A loss of
    /// 4,000 MW on the same day falls on to 48.8 Hz and is refused by name on
    /// the step that would land there; nothing moves.
    #[test]
    fn a_fall_below_the_compared_frequencies_lands_until_disconnection() {
        assert_eq!(FREQUENCY_COMPARED_MIN_HZ, 49.55);
        let (card, mut state) = start(49.85, 111_000.0, 18_163.0);
        let mut frame = blank_frame();
        let mut flagged = 0;
        for _ in 0..3840 {
            step(&card, &mut state, &mut frame, load(LOSS_MAX_MW), 0.015625).unwrap();
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            let low = state.frequency_hz < FREQUENCY_COMPARED_MIN_HZ;
            assert_eq!(frame.flags & FLAG_BELOW_COMPARED_FREQUENCY != 0, low);
            assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED_TRIPS, 0);
            if low {
                flagged += 1;
            }
        }
        assert!(flagged > 100, "{flagged}");
        assert!(state.lowest_frequency_hz > 49.4 && state.lowest_frequency_hz < 49.5, "{}", state.lowest_frequency_hz);
        let (card, mut state) = start(49.85, 111_000.0, 18_163.0);
        let mut frame = blank_frame();
        let mut landed = 0;
        let reject = loop {
            let before = state;
            let held = frame.to_bytes();
            match step(&card, &mut state, &mut frame, load(4_000.0), 0.015625) {
                Ok(()) => {
                    landed += 1;
                    assert!(state.frequency_hz > FREQUENCY_MIN_HZ);
                    assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                }
                Err(reject) => {
                    assert_eq!(state, before);
                    assert_eq!(frame.to_bytes(), held);
                    break reject;
                }
            }
        };
        assert_eq!(reject, StepReject::LowFrequencyDemandDisconnection);
        assert!(landed > 10 && landed < 3840, "{landed}");
        let mut altered = card;
        altered.frequency_compared_min_hz = 49.5;
        assert!(!altered.is_valid());
    }

    /// Compared trips land every step above 49.55 Hz: the opened trip of
    /// 18 April 2019 (rank 5) and that of the largest loss (rank 23, 1402 MW
    /// on 283 GVA s). The lowest the card reached in any compared run is
    /// 49.563 Hz.
    #[test]
    fn a_compared_trip_lands_every_step_above_the_compared_frequencies() {
        for case in [RANK_5, (50.077, 283_000.0, 28_510.0, 1402.0)] {
            let (card, mut state) = start(case.0, case.1, case.2);
            let mut frame = blank_frame();
            for _ in 0..3840 {
                step(&card, &mut state, &mut frame, load(case.3), 0.015625).unwrap();
                assert_eq!(frame.flags, 0);
            }
            assert!(state.lowest_frequency_hz > FREQUENCY_COMPARED_MIN_HZ);
            assert_eq!(state.t_s, 60.0);
        }
    }

    /// Past the compared minute the run goes on: ten minutes of the opened
    /// trip of rank 5 land every step, the steps after 60 s with the flag,
    /// the ledger closing to the end, and the frequency settles where the
    /// response and the relief carry the loss.
    #[test]
    fn the_run_goes_on_past_sixty_seconds_with_the_flag() {
        let (card, mut state) = start(RANK_5.0, RANK_5.1, RANK_5.2);
        let mut frame = blank_frame();
        let mut worst = 0.0_f64;
        for landed in 1..=38_400 {
            step(&card, &mut state, &mut frame, load(RANK_5.3), 0.015625).unwrap();
            let late = landed > 3840;
            assert_eq!(frame.flags == FLAG_PAST_COMPARED_MINUTE, late, "{landed}");
            if !late {
                assert_eq!(frame.flags, 0);
            }
            worst = worst.max(state.residual_conservation_error);
        }
        assert_eq!(state.t_s, 600.0);
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
        assert!(state.frequency_rate_hz_per_s.abs() < 1.0e-9);
        assert!((state.net_power_mw).abs() < 1.0e-6);
    }

    /// The allowance is what its derivation says: 2^-48 s a step, the steps
    /// a run takes to reach 60 s and two more, and never half a step or more.
    #[test]
    fn the_time_end_allowance_is_its_derivation() {
        let card = GridGb::interconnector_loss();
        let spacing_s = f64::from_bits(TIME_MAX_S.to_bits() + 1) - TIME_MAX_S;
        assert_eq!(TIME_END_ALLOWANCE_PER_STEP_S, 0.5 * spacing_s);
        assert_eq!(TIME_END_ALLOWANCE_EXTRA_STEPS, 2.0);
        assert_eq!(
            card.time_end_allowance_s(0.02),
            (60.0 / 0.02 + 2.0) * TIME_END_ALLOWANCE_PER_STEP_S
        );
        let at_ceiling = card.time_end_allowance_s(ceiling());
        assert!(at_ceiling > 1.0e-11 && at_ceiling < 1.1e-11);
        let at_floor = card.time_end_allowance_s(FLOOR_DT_S);
        assert!(at_floor > 2.1e-7 && at_floor < 2.2e-7);
        // inside the step floor it is under a quarter of the step
        assert!(at_floor < 0.25 * FLOOR_DT_S);
        // under the floor it is held to half the step
        assert_eq!(card.time_end_allowance_s(1.0e-9), 0.5e-9);
        assert_eq!(card.time_end_allowance_s(0.0), 0.0);
        assert_eq!(card.time_end_allowance_s(-1.0), 0.0);
        let mut altered = card;
        altered.time_end_allowance_per_step_s = 1.0e-9;
        assert!(!altered.is_valid());
        let mut altered = card;
        altered.time_end_allowance_extra_steps = 3.0;
        assert!(!altered.is_valid());
    }

    /// Runs of exactly 60 s in decimal land every step at twenty step sizes
    /// and three trips, and the step after is refused by name. The five
    /// steps the second scorer found short (V-32) are among them.
    #[test]
    fn a_decimal_run_of_sixty_seconds_lands_every_step() {
        // (the step as a decimal, steps to 60 s)
        let sizes: [(&str, usize); 20] = [
            ("0.02", 3_000),
            ("0.016", 3_750),
            ("0.015625", 3_840),
            ("0.015", 4_000),
            ("0.0125", 4_800),
            ("0.012", 5_000),
            ("0.01", 6_000),
            ("0.008", 7_500),
            ("0.0075", 8_000),
            ("0.006", 10_000),
            ("0.005", 12_000),
            ("0.004", 15_000),
            ("0.003", 20_000),
            ("0.0025", 24_000),
            ("0.002", 30_000),
            ("0.0016", 37_500),
            ("0.0015", 40_000),
            ("0.0012", 50_000),
            ("0.001", 60_000),
            ("0.0008", 75_000),
        ];
        let trips = [
            RANK_5,
            (50.0, 200_000.0, 30_000.0, 1_000.0),
            (49.85, 111_000.0, 18_163.0, LOSS_MIN_MW),
        ];
        let mut past_the_end = 0;
        for (index, (text, count)) in sizes.iter().enumerate() {
            let dt_s: f64 = text.parse().unwrap();
            let trip = trips[index % 3];
            let (card, mut state) = start(trip.0, trip.1, trip.2);
            let mut frame = blank_frame();
            for landed in 0..*count {
                assert_eq!(
                    step(&card, &mut state, &mut frame, load(trip.3), dt_s),
                    Ok(()),
                    "dt {text}: step {} of {count}",
                    landed + 1
                );
            }
            assert!((state.t_s - TIME_MAX_S).abs() <= card.time_end_allowance_s(dt_s));
            assert_eq!(frame.flags & FLAG_PAST_COMPARED_MINUTE, 0);
            if state.t_s > TIME_MAX_S {
                past_the_end += 1;
            }
            step(&card, &mut state, &mut frame, load(trip.3), dt_s).unwrap();
            assert_eq!(frame.flags & FLAG_PAST_COMPARED_MINUTE, FLAG_PAST_COMPARED_MINUTE);
        }
        assert!(past_the_end > 0);
    }

    /// A step that ends later than 60 s by a spacing more than the allowance
    /// lands with the flag; every end inside the allowance lands at its own
    /// value without it.
    #[test]
    fn a_step_beyond_the_time_allowance_lands_with_the_flag() {
        let spacing_s = f64::from_bits(TIME_MAX_S.to_bits() + 1) - TIME_MAX_S;
        for dt_s in [0.02, 0.015625, 0.01, 0.003] {
            let (card, fresh) = start(RANK_5.0, RANK_5.1, RANK_5.2);
            let allowance_s = card.time_end_allowance_s(dt_s);
            // the comparison is with the 64-bit number nearest the end plus the allowance
            let whole_spacings = (((TIME_MAX_S + allowance_s) - TIME_MAX_S) / spacing_s) as u64;
            assert!(whole_spacings >= 1);
            let mut landed_past_the_end = 0;
            let mut refused = 0;
            for spacings in (0..whole_spacings + 4).rev().take(8).chain(0..2) {
                let end_s = f64::from_bits(TIME_MAX_S.to_bits() + spacings);
                // the clock whose sum with the step is this end
                let mut t_s = end_s - dt_s;
                while t_s + dt_s < end_s {
                    t_s = f64::from_bits(t_s.to_bits() + 1);
                }
                assert_eq!(t_s + dt_s, end_s);
                let mut state = fresh;
                state.t_s = t_s;
                state.run_loss_mw = RANK_5.3;
                let mut frame = blank_frame();
                let result = step(&card, &mut state, &mut frame, load(RANK_5.3), dt_s);
                assert_eq!(result, Ok(()));
                assert_eq!(state.t_s, end_s);
                if end_s <= TIME_MAX_S + allowance_s {
                    assert_eq!(frame.flags, 0);
                    if spacings > 0 {
                        landed_past_the_end += 1;
                    }
                } else {
                    assert_eq!(frame.flags, FLAG_PAST_COMPARED_MINUTE);
                    refused += 1;
                }
            }
            assert!(landed_past_the_end >= 4);
            assert_eq!(refused, 3);
            // one allowance and a half beyond lands with the flag
            let mut state = fresh;
            state.t_s = TIME_MAX_S + 1.5 * allowance_s - dt_s;
            state.run_loss_mw = RANK_5.3;
            let mut frame = blank_frame();
            step(&card, &mut state, &mut frame, load(RANK_5.3), dt_s).unwrap();
            assert_eq!(frame.flags, FLAG_PAST_COMPARED_MINUTE);
        }
    }

    /// Every compared trip was one loss. A second loss, the link coming back
    /// and a trip after a quiet period are stepped by the same equations:
    /// each lands with the flag, the ledger books each step's own loss and
    /// closes, and the run recovers to its start when the link comes back.
    #[test]
    fn a_loss_changed_during_the_run_lands_and_the_ledger_books_it() {
        let (card, mut state) = start(50.0, 200_000.0, 30_000.0);
        let mut frame = blank_frame();
        assert_eq!(state.run_loss_mw, 0.0);
        for _ in 0..640 {
            step(&card, &mut state, &mut frame, load(700.0), 0.015625).unwrap();
            assert_eq!(frame.flags, 0);
        }
        assert_eq!(state.run_loss_mw, 700.0);
        let mut worst = 0.0_f64;
        // a second loss, then the link coming back, then nothing lost
        for (loss, steps) in [(1400.0, 640), (0.0, 1920)] {
            for _ in 0..steps {
                step(&card, &mut state, &mut frame, load(loss), 0.015625).unwrap();
                assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED_TRIPS, FLAG_OUTSIDE_COMPARED_TRIPS);
                worst = worst.max(state.residual_conservation_error);
            }
        }
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
        assert_eq!(state.run_loss_mw, 700.0);
        // with the loss gone, the response unwinds and the area returns to its start
        assert!((state.frequency_hz - 50.0).abs() < 1.0e-3, "{}", state.frequency_hz);
        // a quiet period before the trip: steps of zero, then the loss
        let (card, mut quiet) = start(50.0, 200_000.0, 30_000.0);
        for _ in 0..64 {
            step(&card, &mut quiet, &mut frame, load(0.0), 0.015625).unwrap();
        }
        assert_eq!(quiet.run_loss_mw, 0.0);
        step(&card, &mut quiet, &mut frame, load(1000.0), 0.015625).unwrap();
        assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED_TRIPS, FLAG_OUTSIDE_COMPARED_TRIPS);
        assert!(quiet.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
    }

    /// A loss of demand, a negative loss, drives the frequency up: it lands
    /// every step with the flag, the ledger closes, and a large enough one is
    /// refused by name at 50.5 Hz, where the card holds no over-frequency
    /// response.
    #[test]
    fn a_loss_of_demand_rises_until_the_statutory_limit() {
        let (card, mut state) = start(50.0, 200_000.0, 30_000.0);
        let mut frame = blank_frame();
        for _ in 0..3840 {
            step(&card, &mut state, &mut frame, load(-1000.0), 0.015625).unwrap();
            assert_eq!(frame.flags, FLAG_OUTSIDE_COMPARED_TRIPS);
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        }
        assert!(state.frequency_hz > 50.1 && state.frequency_hz < 50.5, "{}", state.frequency_hz);
        let (card, mut state) = start(50.0, 111_000.0, 18_163.0);
        let reject = loop {
            let before = state;
            match step(&card, &mut state, &mut frame, load(-5_000.0), 0.015625) {
                Ok(()) => assert!(state.frequency_hz <= FREQUENCY_MAX_HZ),
                Err(reject) => {
                    assert_eq!(state, before);
                    break reject;
                }
            }
        };
        assert_eq!(reject, StepReject::FrequencyAboveCard);
    }

    /// Outside the compared inertia and demand each run takes its own step
    /// ceiling, from the same rule. A run on a fifth of the smallest compared
    /// inertia and one on six times the largest compared demand land every step
    /// at their own ceiling with the flag, and the ledger closes; the small
    /// inertia falls further and faster, and its ceiling is the smaller.
    #[test]
    fn small_inertia_and_large_demand_run_at_their_own_ceiling() {
        let card = GridGb::interconnector_loss();
        let compared = card.gate_dt_s().unwrap();
        for (inertia, demand, loss) in [(22_200.0, 30_000.0, 1000.0), (111_000.0, 250_000.0, 1402.0), (5_000.0, 81_658.0, 300.0)] {
            let ceiling = card.gate_dt_s_at(inertia, demand).unwrap();
            assert!(ceiling < compared, "{inertia} {demand}");
            let (card, mut state) = start(50.0, inertia, demand);
            let mut frame = blank_frame();
            let steps = (60.0 / ceiling) as usize;
            let mut worst = 0.0_f64;
            let mut refused = None;
            for _ in 0..steps {
                let before = state;
                match step(&card, &mut state, &mut frame, load(loss), ceiling) {
                    Ok(()) => {
                        assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED_TRIPS, FLAG_OUTSIDE_COMPARED_TRIPS);
                        worst = worst.max(state.residual_conservation_error);
                    }
                    Err(reject) => {
                        assert_eq!(state, before);
                        refused = Some(reject);
                        break;
                    }
                }
            }
            assert!(worst <= RESIDUAL_CONSERVATION_LIMIT / 10.0, "{inertia} {demand} {worst:e}");
            assert!(refused.is_none() || refused == Some(StepReject::LowFrequencyDemandDisconnection), "{refused:?}");
        }
        // the ceiling falls under the floor near 1.3 MW s at the largest compared demand
        assert!(card.gate_dt_s_at(1.2, 40_829.0).unwrap() < FLOOR_DT_S);
        assert!(card.gate_dt_s_at(1.4, 40_829.0).unwrap() > FLOOR_DT_S);
        assert_eq!(card.gate_dt_s_at(0.0, 30_000.0), None);
        assert_eq!(card.gate_dt_s_at(200_000.0, -1.0), None);
        assert_eq!(card.gate_dt_s_at(111_000.0, 40_829.0), card.gate_dt_s());
    }

    #[test]
    fn the_proof_is_not_the_cards() {
        let (card, mut state) = start(50.0, 200_000.0, 30_000.0);
        let mut frame = blank_frame();
        step(&card, &mut state, &mut frame, load(1000.0), 0.01).unwrap();
        assert_eq!(frame.proof, [0xA5; 16]);
        assert_eq!(frame.t, 0.01);
        assert_eq!(frame.pos[0], state.frequency_hz as f32);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
    }

    #[test]
    fn types_are_copy() {
        fn is_copy<T: Copy>() {}
        is_copy::<GridGb>();
        is_copy::<GridState>();
        is_copy::<GridLoad>();
        is_copy::<StepReject>();
    }

    #[test]
    fn the_ledger_stays_under_the_limit_over_full_runs_at_the_ceiling_and_the_floor() {
        let dt = ceiling();
        let steps = (60.0 / dt) as usize;
        let mut worst_all = 0.0;
        for case in [
            RANK_5,
            (49.85, 111_000.0, 18_163.0, 693.0),
            (50.15, 337_000.0, 40_829.0, 1402.0),
            (50.15, 111_000.0, 40_829.0, 693.0),
            (49.85, 337_000.0, 18_163.0, 700.0),
        ] {
            let (_, worst) = run(case, dt, steps);
            if worst > worst_all {
                worst_all = worst;
            }
        }
        println!("largest residual at the ceiling {worst_all:e}");
        assert!(worst_all < RESIDUAL_CONSERVATION_LIMIT / 10.0, "{worst_all:e}");
        let (_, worst) = run(RANK_5, FLOOR_DT_S, 200_000);
        println!("largest residual at the floor {worst:e}");
        assert!(worst < RESIDUAL_CONSERVATION_LIMIT / 10.0, "{worst:e}");
    }

    #[test]
    fn explicit_euler_at_the_ceiling_is_refused_by_the_ledger() {
        let (card, mut state) = start(RANK_5.0, RANK_5.1, RANK_5.2);
        let dt = ceiling();
        let mut frame = blank_frame();
        let mut refused = None;
        let mut index = 0;
        while index < 3000 {
            let y = state_vector(&state);
            let k = rates(&card, &state, RANK_5.3, &y);
            let advanced = advance(&y, &k, dt);
            validate(&card, &state, load(RANK_5.3), dt).unwrap();
            let before = state;
            match commit(&card, &mut state, &mut frame, load(RANK_5.3), dt, advanced) {
                Ok(()) => {}
                Err(reject) => {
                    assert_eq!(state, before);
                    refused = Some((index, reject));
                    break;
                }
            }
            index += 1;
        }
        let (at, reject) = refused.unwrap();
        println!("Euler refused at step {at}: {reject:?}");
        assert!(at < 5, "{at}");
        assert!(reject == StepReject::SpontaneousGeneration || reject == StepReject::FirstLawBreach);
    }

    #[test]
    fn a_falsified_ledger_is_refused_both_ways() {
        // The honest step, then the energy lost booked 1 part in 10,000 too
        // small and too large: the kinetic energy no longer closes.
        for (factor, want) in [
            (0.9999, StepReject::FirstLawBreach),
            (1.0001, StepReject::SpontaneousGeneration),
        ] {
            let (card, mut state) = start(RANK_5.0, RANK_5.1, RANK_5.2);
            let dt = ceiling();
            let mut frame = blank_frame();
            let mut index = 0;
            while index < 200 {
                step(&card, &mut state, &mut frame, load(RANK_5.3), dt).unwrap();
                index += 1;
            }
            state.energy_lost_mw_s *= factor;
            let before = state;
            let frame_before = frame.to_bytes();
            assert_eq!(
                step(&card, &mut state, &mut frame, load(RANK_5.3), dt),
                Err(want)
            );
            assert_eq!(state, before);
            assert_eq!(frame.to_bytes(), frame_before);
        }
        // A step whose frequency was advanced with half the inertia.
        let (card, mut state) = start(RANK_5.0, RANK_5.1, RANK_5.2);
        let dt = ceiling();
        let mut frame = blank_frame();
        let mut wrong = state;
        wrong.inertia_mw_s *= 0.5;
        let y = state_vector(&state);
        let mut refused = None;
        let mut index = 0;
        while index < 50 {
            let y_now = if index == 0 { y } else { state_vector(&state) };
            let advanced = rk4(&card, &wrong, RANK_5.3, &y_now, dt);
            if let Err(reject) = commit(&card, &mut state, &mut frame, load(RANK_5.3), dt, advanced)
            {
                refused = Some(reject);
                break;
            }
            index += 1;
        }
        assert_eq!(refused, Some(StepReject::FirstLawBreach));
    }

    #[test]
    fn the_step_ceiling_and_floor_are_present_and_derived() {
        let card = GridGb::interconnector_loss();
        let gate = card.gate_dt_s().unwrap();
        let c = 2500.0 / (2.0 * 111_000.0 * 48.8);
        let swing = (c * (2074.0 + 3136.0) / 1.296_f64).sqrt();
        let relief = c * 0.025 * 40_829.0;
        let lag = 1.0 / 1.296;
        let fastest = if swing > lag { swing } else { lag };
        assert!(relief < fastest);
        assert!((gate - 0.02 / fastest).abs() < 1.0e-15, "{gate}");
        println!("gate_dt_s {gate}");
        assert_eq!(card.floor_dt_s(), Some(1.0e-6));
        assert!(gate / FLOOR_DT_S > 1000.0);
    }

    #[test]
    fn halving_the_step_changes_the_answer_by_the_fourth_order_ratio() {
        // The start frequency is above the knee and the fall crosses it, so
        // the ratio is taken on a run that stays above it (the smallest loss
        // on the largest inertia and demand from the highest start), where
        // the right-hand side is smooth.
        let case = (50.15, 337_000.0, 40_829.0, 693.0);
        let at = |dt: f64| run(case, dt, (8.0 / dt + 0.5) as usize).0.frequency_hz;
        let (a, b, c) = (at(0.32), at(0.16), at(0.08));
        let ratio = (a - b) / (b - c);
        println!("ratio {ratio}");
        assert!(ratio > 12.0 && ratio < 20.0, "{ratio}");
        assert!(run(case, 0.08, 100).0.lowest_frequency_hz > RESPONSE_KNEE_HZ);
    }

    /// The forecast of structure F for the opened trip of rank 5 at samples
    /// 1, 5, 10, 30 and 60 (times 0.5, 4.5, 9.5, 29.5, 59.5 s after the loss
    /// begins), made by `analysis/06_card_reference.py` (RK4, step 0.001 s,
    /// the constants above). Then the measured samples beside them, for the
    /// report only.
    const FORECAST_RANK_5: [(f64, f64, f64); 5] = [
        (0.5, 49.891475012168605, 49.870),
        (4.5, 49.67608544815959, 49.629),
        (9.5, 49.698184270373474, 49.622),
        (29.5, 49.69722349369337, 49.756),
        (59.5, 49.69722442810978, 49.736),
    ];

    #[test]
    fn the_card_at_its_ceiling_agrees_with_the_forecast_of_structure_f() {
        let dt = 0.0125;
        assert!(dt < ceiling());
        let (card, mut state) = start(RANK_5.0, RANK_5.1, RANK_5.2);
        let mut frame = blank_frame();
        let mut next = 0;
        let mut index = 1;
        while index <= 4760 && next < 5 {
            step(&card, &mut state, &mut frame, load(RANK_5.3), dt).unwrap();
            if index == (FORECAST_RANK_5[next].0 / dt + 0.5) as usize {
                let (t, forecast, measured) = FORECAST_RANK_5[next];
                println!(
                    "t {t} card {} forecast {forecast} measured {measured}",
                    state.frequency_hz
                );
                assert!((state.frequency_hz - forecast).abs() < 1.0e-7, "{t}");
                next += 1;
            }
            index += 1;
        }
        assert_eq!(next, 5);
    }
}
