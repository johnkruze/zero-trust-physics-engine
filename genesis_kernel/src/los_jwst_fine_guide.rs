#![no_implicit_prelude]

//! The line of sight of the James Webb Space Telescope in fine guide.
//!
//! The telescope holds a guide star on its Fine Guidance Sensor. Every 64 ms
//! the flight software measures where the star's image is, the centroid, and
//! a fine steering mirror corrects the line of sight. The centroid is stored
//! by the Space Telescope Science Institute in the table `FGS CENTROID PACKET`
//! of each calibrated fine-guide file in the Mikulski Archive for Space
//! Telescopes (MAST), open to all: arcsec in the sensor's ideal frame, with
//! the guide star's count rate.
//!
//! The card steps a Kalman filter through that table, one frame at a time. It
//! takes the lock point once and then, for each frame, the table's own three
//! numbers. Before it uses a frame's centroid it states where it expects the
//! centroid and with what variance; then it uses the centroid and states where
//! the line of sight is with the sensor's noise taken out.
//!
//! Where the numbers come from, and what they rest on:
//!
//! - The eighteen constants, nine for each axis, are fitted to this
//!   observatory's own centroids: the 127 fine-guide files of programme 1163,
//!   "OTE-26 Routine WFSC/CC Jitter Maintenance", of the visits that started
//!   in April 2022; 936,273 frames, six guide stars, both guiders. The fit is
//!   maximum likelihood on the spectra of the quiet stretches. They are
//!   `analysis/out/card_constants.json` of the research record at
//!   `evidence/research/pointing_jwst_fine_guidance/`, six figures, copied.
//! - The centroid is the flight software's. Nothing here measures a star
//!   image.
//! - The lock point is not in the file. The caller supplies it. The record's
//!   protocol takes the median of the run's good centroids.
//! - The noise law was fitted on count rates from 22,910 to 2,813,637
//!   counts per second, and every held-back file lay inside them (their
//!   medians 62,000 to 816,000). The card takes any count rate above zero:
//!   outside 22,900 to 2,820,000 a frame carries
//!   [`FLAG_COUNT_RATE_BEYOND_COMPARED`], and the noise law is carried on
//!   past where it was fitted. The step is one sensor frame.
//! - The lock point is held to the sensor's field, 70 arcsec from its centre
//!   on either axis; the comparison answered lock points from -61.1512947 to
//!   60.4564934 arcsec on x and from -64.2767 to 50.1812668 arcsec on y, and
//!   at most 10 frames in a row with no measurement. Past those the card
//!   computes; see "The lock point and gaps" below.
//! - Compared on 2026-10-09 with the 134 files of the 24 visits of May and
//!   June 2022, which no fit saw, by a rule fixed beforehand: all five marks
//!   were met on both axes. On their 795,021 quiet frames, one frame ahead,
//!   the card's error was 0.510 mas on x and 0.472 mas on y against 0.644 and
//!   0.584 for taking the last centroid, and the mean of squared error over
//!   stated variance was 1.004 and 0.986. Inside disturbances it was worse
//!   than taking the last centroid, 2.03 mas against 1.00 on x and 1.57
//!   against 0.78 on y. `RESULT.md` of the research record. The card as it
//!   was then answered those frames with a flag; engines 0.7 to 0.9 refused
//!   them; it answers them again, with the flags and the measured shortfall
//!   under "Where the card was measured worse" below.
//!
//! The model. Each axis of the offset from the lock point is the sum of three
//! parts, and the sensor adds noise:
//!
//! ```text
//! slow    first order: standard deviation s_slow, memory time tau_slow
//! fast    first order: standard deviation s_fast, memory time tau_fast
//! mode    one oscillation at f_mode, driven at random, memory time tau_mode,
//!         standard deviation s_mode; two numbers, in phase and in quadrature
//! noise   white, variance R = r_ref · (counts / 1e6)^(−r_power)        mas²
//! ```
//!
//! On x the mode is at 0.544 Hz and fades in 5.9 s; on y it is at 0.303 Hz
//! and fades in 14.6 s. The card does not say what they are. Their numbers are
//! what the centroids show.
//!
//! One step of `dt`, for each axis, with `m = [slow, fast, cos, sin]` and its
//! covariance `P`:
//!
//! ```text
//! fading      rho_i = exp(−dt / tau_i)        turn  theta = 2 pi f_mode dt
//! drive       q_i = s_i² (1 − rho_i²)
//! predict     m ← A m,  P ← A P Aᵀ + Q        A = diag(rho_s, rho_f, rho_m Rot(theta))
//! expected    z_pred = slow + fast + cos                               mas
//! if measured:
//!   offset    z = (centroid − lock) · 1000                             mas
//!   variance  S = H P Hᵀ + R,  H = [1, 1, 1, 0]                        mas²
//!   gain      K = P Hᵀ / S
//!   update    m ← m + K (z − z_pred)
//!             P ← P − K hᵀ − h Kᵀ + K S Kᵀ,  h = P Hᵀ
//! estimate    slow + fast + cos                                        mas
//! ```
//!
//! The update of `P` is the long form, which holds for any gain, written out.
//! A frame with no measurement is predicted and not updated.
//!
//! The ledger is an account of variance, in mas². It is not a conservation
//! law of physics: it is bookkeeping that follows from the model. Fading
//! takes the share `1 − rho_i²` of each part's variance, the drive puts back
//! `q_i`, a turn changes no total, and a measurement takes `|h|² / S`:
//!
//! ```text
//! account   T = rho_s² P_ss + rho_f² P_ff + rho_m² (P_cc + P_qq)
//!               + q_s + q_f + 2 q_m − [ |h|² / S  if measured ]
//! defect    trace(P after the step) − T, for each axis
//! residual  ( |defect_x| + |defect_y| ) / scale
//! ```
//!
//! `scale` is the variance of both axes with nothing measured, 1.52574 mas².
//! The account is taken from the variances before the matrices are touched
//! and never uses the sine or the cosine; the trace is taken from the matrices
//! after. The ledger names a gain that is not the best one (the long form then
//! leaves more variance than the account: `SpontaneousGeneration`; the defect
//! goes as the square of the gain's error, so on a settled filter a gain one
//! part in 10,000 off is named and one part in 100,000 off is not), a short
//! form of the update with a gain too large (less variance than the account,
//! a filter that claims knowledge it was not given: `FirstLawBreach`), a
//! drive left out, and a turn that does not preserve the total. It cannot
//! detect a wrong constant: a card with the wrong frequency or the wrong
//! noise balances its books exactly. It says nothing about the estimate
//! itself. It shows the arithmetic is the filter's. Whether the filter matches
//! the telescope is shown only by the comparison with centroids it has not
//! seen: the mean of `(z − z_pred)² / S`, which is 1 if the stated variance is
//! right. The state carries that sum for each axis.
//!
//! What the card leaves out: everything above 7.8 Hz, which a 64 ms frame
//! does not resolve; lines near 0.51, 1.02 and 1.53 Hz that the y axis shows
//! and the model has no part for; the roll about the line of sight; the
//! reason for any motion. It describes quiet fine guide.
//!
//! Where the card was measured worse. When a mechanism moves or the
//! pointing is disturbed the offset jumps by many mas and rings down; the
//! filter's gain is set for quiet and follows such a ring-down slowly, so its
//! prediction is then worse than the last centroid and its stated variance
//! too small, by 12 to 17 times on the held-back files. Inside disturbances
//! its error one frame ahead was 2.03 mas against 1.00 for the last centroid
//! on x, and 1.57 against 0.78 on y. Over all 973,388 measured held-back
//! frames, quiet and disturbed together, it was 1.35 on x and 1.26 on y of the
//! last centroid's error, with a squared error over the stated variance of
//! 3.9 and 3.0. The card answers those frames and says so with three flags
//! (`analysis/amendment_1.json` of the research record holds the three
//! numbers, the protocol's own; none was fitted to a held-back file):
//!
//! - [`FLAG_POINTING_DISTURBED`]: a measured centroid more than
//!   [`QUIET_OFFSET_MAS`], 5 mas, from the lock point on either axis, the
//!   protocol's event sample.
//! - [`FLAG_BEYOND_FOUR_SIGMA`]: a measured centroid more than
//!   [`INNOVATION_LIMIT_SIGMA`], four, stated standard deviations from where
//!   it was expected, on either axis. On the fitted files of April 2022, in
//!   the 30 s after such a centroid the x axis misses four of the protocol's
//!   five marks (error over the last centroid's 0.92 against a line of 0.85,
//!   squared error over stated variance 1.64 against 0.67 to 1.50, lag-one
//!   correlation 0.27 against 0.15, 74 percent of files against 80), and on
//!   the held-back files three. Such a centroid came 31 times as often on x
//!   as a right variance gives on the fitted files, and 21 times on the
//!   held-back ones. It also comes by chance: with the stated variance right,
//!   once in about 7,900 measured frames, 8.4 minutes.
//! - [`FLAG_RECOVERING`]: the frame ends less than [`RECOVERY_S`], 30 s, after
//!   the last centroid that carried either flag, the protocol's 30 s after an
//!   event sample. The state keeps that time, `since_disturbance_s`, from the
//!   caller's `quiet_before_start_s` at the start of the run; the card cannot
//!   see a centroid that came before its run and cannot check that number.
//!
//! Until engine 0.9 each of the three was a refusal (`PointingDisturbance`,
//! `InnovationBeyondFourSigma`, `RecoveryNotElapsed`) and a run ended at its
//! last quiet frame. The filter's equations and its ledger hold in those
//! frames; what fails there is the model's assumption of a quiet, stationary
//! sum of three processes, and the measured shortfall above is what that
//! costs. A `quiet_before_start_s` below zero is malformed and is
//! [`StepReject::QuietBeforeStartBelowZero`]. The comparison of a time with
//! 30 s carries [`RECOVERY_ALLOWANCE_S`], 2^-36 s, for a time worked out as
//! the difference of two times of the table.
//!
//! The lock point and gaps:
//!
//! - [`StepReject::LockOutsideSensorField`]: a lock point more than
//!   [`LOCK_FIELD_HALF_WIDTH_ARCSEC`], 70 arcsec, from the centre of the
//!   sensor's ideal frame on either axis. Each guider's field is 2.3 by 2.3
//!   arcmin, 69 arcsec either side of its centre; no star can be held beyond
//!   it. The lock points of the files compared span [`LOCK_X_MIN_ARCSEC`] to
//!   [`LOCK_X_MAX_ARCSEC`] on x and [`LOCK_Y_MIN_ARCSEC`] to
//!   [`LOCK_Y_MAX_ARCSEC`] on y, fitted and held-back files together, the
//!   held-back files alone x -61.1512947 to 49.05617 and y -62.0922546 to
//!   50.1812668; on the held-back files all five marks were met on both axes.
//!   The arithmetic only takes the lock point away from the centroid.
//! - A frame with no measurement is predicted, and its stated variance grows
//!   toward its unmeasured value, 1.52574 mas² over both axes, for any number
//!   of such frames in a row. The longest run of them a compared run crossed
//!   was [`MEASUREMENT_GAP_MAX_FRAMES`], 10, on the fitted files, and 2 on the
//!   held-back files. The state counts them, `unmeasured_in_row`; until
//!   engine 0.9 an eleventh was refused as `MeasurementGapOutsideCard`.
//!
//! The run has no limit in time of its own.
//!
//! The power. The noise needs `(counts / 1e6)^(−r_power)`. The crate carries
//! its own exponential and natural logarithm and no power function, so it is
//! `exp(−r_power · log(counts / 1e6))`. Over the fitted count rates the
//! argument of the exponential is within ±5, and the result is within a few
//! units in the last place of the correctly rounded value; farther out the
//! error grows with the size of the argument. The law has no value at a count
//! rate of zero, and under about 7e-237 counts per second the y axis's
//! variance passes the largest 64-bit number: such a frame is
//! [`StepReject::CountRateNotPositive`]. As the count rate grows the variance
//! falls toward zero, and the filter's update holds there, its predicted
//! variance being above zero after every drive.
//!
//! [`FLAG_NO_MEASUREMENT`]: the table had no good centroid for the frame. It
//! marks no failure of the model.
//!
//! [`LosJwstFineGuide::gate_dt_s`] and [`LosJwstFineGuide::floor_dt_s`] are
//! 0.0645 s and 0.0635 s: one sensor frame, 0.064 s, within half a
//! millisecond. The centroid is an average over one frame and the noise law is
//! that of one frame, so a step of another length has no measurement to go
//! with it. The public step applies neither; the gateway refuses a step
//! outside them.
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes into [`LastStateFrame64`] the
//! time, the expected offset on x and y, the variance stated for x, the
//! estimate on x and y, the variance stated for y, the normalised innovation
//! squared of the frame and the ledger residual. The 16-byte frame proof is
//! not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

/// The nine fitted constants of one axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisConstants {
    /// Standard deviation of the slow part, mas.
    pub slow_mas: f64,
    /// Memory time of the slow part, s.
    pub slow_memory_s: f64,
    /// Standard deviation of the fast part, mas.
    pub fast_mas: f64,
    /// Memory time of the fast part, s.
    pub fast_memory_s: f64,
    /// Standard deviation of the mode, mas.
    pub mode_mas: f64,
    /// Memory time of the mode, s.
    pub mode_memory_s: f64,
    /// Frequency of the mode, Hz.
    pub mode_hz: f64,
    /// Noise variance of one centroid at [`COUNTS_REFERENCE_PER_S`], mas².
    pub noise_reference_mas2: f64,
    /// The power of the count rate in the noise variance.
    pub noise_power: f64,
}

/// The x axis of the sensor's ideal frame. Fitted to the April 2022 visits.
pub const X_AXIS: AxisConstants = AxisConstants {
    slow_mas: 1.03257,
    slow_memory_s: 66.7485,
    fast_mas: 0.142973,
    fast_memory_s: 0.0980239,
    mode_mas: 0.136597,
    mode_memory_s: 5.88667,
    mode_hz: 0.543703,
    noise_reference_mas2: 0.0360528,
    noise_power: 1.15083,
};
/// The y axis of the sensor's ideal frame. Fitted to the April 2022 visits.
pub const Y_AXIS: AxisConstants = AxisConstants {
    slow_mas: 0.455171,
    slow_memory_s: 12.2188,
    fast_mas: 0.156568,
    fast_memory_s: 0.0943062,
    mode_mas: 0.291622,
    mode_memory_s: 14.6184,
    mode_hz: 0.302547,
    noise_reference_mas2: 0.0253144,
    noise_power: 1.27308,
};
/// The count rate at which the noise variance is `noise_reference_mas2`,
/// instrument counts per second.
pub const COUNTS_REFERENCE_PER_S: f64 = 1.0e6;
/// The fitted set's smallest count rate, 22,910, rounded down. Below it a
/// measured frame carries [`FLAG_COUNT_RATE_BEYOND_COMPARED`]. Not a limit.
pub const COUNTS_MIN_PER_S: f64 = 22_900.0;
/// The fitted set's largest count rate, 2,813,637, rounded up. Above it a
/// measured frame carries [`FLAG_COUNT_RATE_BEYOND_COMPARED`]. Not a limit.
pub const COUNTS_MAX_PER_S: f64 = 2_820_000.0;
/// Largest offset of a centroid from the lock point the card accepts, mas.
/// The sensor's fine-guide box is 8 by 8 pixels of about 69 mas; the largest
/// offset in the fitted set was 191 mas.
pub const OFFSET_MAX_MAS: f64 = 250.0;
/// Largest lock point the card accepts on either axis, in size, arcsec: the
/// Fine Guidance Sensor's field is 2.3 by 2.3 arcmin, 69 arcsec either side
/// of its centre, rounded outward. The card served this bound up to engine
/// 0.6.0.
pub const LOCK_FIELD_HALF_WIDTH_ARCSEC: f64 = 70.0;
/// Smallest lock point on x of the files the comparison answered, arcsec:
/// held-back file jw01163060001_gs-fg_2022149134548. Not a limit.
pub const LOCK_X_MIN_ARCSEC: f64 = -61.1512947;
/// Largest lock point on x compared, arcsec: fitted file
/// jw01163028001_gs-fg_2022104125709. Not a limit.
pub const LOCK_X_MAX_ARCSEC: f64 = 60.4564934;
/// Smallest lock point on y compared, arcsec: fitted file
/// jw01163026001_gs-fg_2022102173705. Not a limit.
pub const LOCK_Y_MIN_ARCSEC: f64 = -64.2767;
/// Largest lock point on y compared, arcsec: held-back file
/// jw01163052001_gs-fg_2022122160545. Not a limit.
pub const LOCK_Y_MAX_ARCSEC: f64 = 50.1812668;
/// The longest run of frames with no measurement a compared run crossed, 10
/// frames on fitted file jw01163051001_gs-fg_2022120171011 (2 on the
/// held-back files). Not a limit.
pub const MEASUREMENT_GAP_MAX_FRAMES: f64 = 10.0;
/// A measured centroid further than this from the lock point on either axis
/// carries [`FLAG_POINTING_DISTURBED`], mas: the event sample of the protocol.
pub const QUIET_OFFSET_MAS: f64 = 5.0;
/// A measured centroid further than this many stated standard deviations
/// from where it was expected, on either axis, carries
/// [`FLAG_BEYOND_FOUR_SIGMA`].
pub const INNOVATION_LIMIT_SIGMA: f64 = 4.0;
/// A frame that ends less than this after the last flagged centroid carries
/// [`FLAG_RECOVERING`], s: the 30 s after an event sample of the protocol.
pub const RECOVERY_S: f64 = 30.0;
/// How far under [`RECOVERY_S`] the stated time may read, s: 2^-36, the most
/// a difference of two times under 2^17 s, each read from a decimal, can
/// differ from the exact difference.
pub const RECOVERY_ALLOWANCE_S: f64 = 1.455_191_522_836_685_2e-11;
/// One sensor frame, s.
pub const FRAME_S: f64 = 0.064;
/// Step ceiling, s: one frame and half a millisecond.
pub const GATE_DT_S: f64 = 0.0645;
/// Step floor, s: one frame less half a millisecond.
pub const FLOOR_DT_S: f64 = 0.0635;
/// `( |defect_x| + |defect_y| ) / scale` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-12;
/// Milliarcseconds in one arcsecond.
pub const MAS_PER_ARCSEC: f64 = 1000.0;

/// The frame has no measurement: it was predicted and not updated.
pub const FLAG_NO_MEASUREMENT: u64 = 1 << 0;
/// The measured centroid is more than 5 mas from the lock point on either
/// axis: the pointing is disturbed, and there the card was measured worse
/// than the last centroid, 2.03 against 1.00 mas on x and 1.57 against 0.78
/// on y, with its stated variance 12 to 17 times too small.
pub const FLAG_POINTING_DISTURBED: u64 = 1 << 1;
/// The measured centroid is more than four stated standard deviations from
/// where it was expected on either axis. With the stated variance right this
/// comes once in about 7,900 measured frames; on the held-back files it came
/// 21 times as often on x, and in the 30 s after it the x axis missed three
/// of the protocol's five marks.
pub const FLAG_BEYOND_FOUR_SIGMA: u64 = 1 << 2;
/// The frame ends less than 30 s after the last centroid that carried
/// [`FLAG_POINTING_DISTURBED`] or [`FLAG_BEYOND_FOUR_SIGMA`], as the state
/// counts it from the caller's `quiet_before_start_s`: the protocol's 30 s
/// after an event, where the filter is still following the ring-down.
pub const FLAG_RECOVERING: u64 = 1 << 3;
/// The guide star's count rate on this measured frame is below 22,900 or
/// above 2,820,000 counts per second, outside the count rates the noise law
/// was fitted on, inside which every held-back file lay: the same filter,
/// with the noise law carried on past them. On the held-back guide stars the
/// y axis's noise exponent came out 1.045 against the card's 1.273, a
/// difference that grows with the distance from 1,000,000 counts per second.
/// The frame carries it; the state holds no boolean of its own for it.
pub const FLAG_COUNT_RATE_BEYOND_COMPARED: u64 = 1 << 4;

/// Why a step did not advance the filter or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` or a number of the load was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or a number of the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this observatory's.
    InvalidObservatory,
    /// The lock point is more than 70 arcsec from the centre of the sensor's ideal frame on either axis: the guider's field is 2.3 by 2.3 arcmin, and no star can be held beyond it.
    LockOutsideSensorField,
    /// The stated time since the last disturbed centroid before the run is below zero.
    QuietBeforeStartBelowZero,
    /// A variance in the incoming state, or the variance stated for a centroid, was zero or negative.
    InvalidCovariance,
    /// `measured` was neither 0 nor 1.
    MeasuredNotZeroOrOne,
    /// A frame whose `measured` is 0 carried a centroid or a count rate that was not zero: such a frame is predicted and not updated, it has no centroid, and a number there would be one the filter does not take.
    UnmeasuredFrameCarriesNumbers,
    /// The guide star's count rate on a frame with a centroid is zero or below, or so near zero, under about 7e-237 counts per second, that the noise variance of the centroid, a power of one over the count rate, passes the largest 64-bit number: the noise law has no value at zero, and a centroid with no light holds no position.
    CountRateNotPositive,
    /// The centroid is more than 250 mas from the lock point on either axis.
    OffsetOutsideCard,
    /// The trial step left more variance than the fading, the drive and the measurement account for.
    SpontaneousGeneration,
    /// The trial step left less variance than the fading, the drive and the measurement account for: the filter would claim knowledge it was not given.
    FirstLawBreach,
}

/// One frame of the table. A frame with no good centroid has `measured` 0 and
/// the other three at zero, as the table writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LosLoad {
    /// 1 when the frame has a good centroid, 0 when it has none.
    pub measured: f64,
    /// `guide_star_position_x` of the table, arcsec in the sensor's ideal frame.
    pub centroid_x_arcsec: f64,
    /// `guide_star_position_y` of the table, arcsec in the sensor's ideal frame.
    pub centroid_y_arcsec: f64,
    /// `guide_star_instrument_counts_per_sec` of the table.
    pub counts_per_s: f64,
}

/// The locked observatory. Every field is a recorded copy of a crate constant
/// and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LosJwstFineGuide {
    /// Recorded copy of [`X_AXIS`].
    pub x: AxisConstants,
    /// Recorded copy of [`Y_AXIS`].
    pub y: AxisConstants,
    /// Recorded copy of [`COUNTS_REFERENCE_PER_S`].
    pub counts_reference_per_s: f64,
    /// Recorded copy of [`COUNTS_MIN_PER_S`].
    pub counts_min_per_s: f64,
    /// Recorded copy of [`COUNTS_MAX_PER_S`].
    pub counts_max_per_s: f64,
    /// Recorded copy of [`OFFSET_MAX_MAS`].
    pub offset_max_mas: f64,
    /// Recorded copy of [`LOCK_FIELD_HALF_WIDTH_ARCSEC`].
    pub lock_field_half_width_arcsec: f64,
    /// Recorded copy of [`QUIET_OFFSET_MAS`].
    pub quiet_offset_mas: f64,
    /// Recorded copy of [`INNOVATION_LIMIT_SIGMA`].
    pub innovation_limit_sigma: f64,
    /// Recorded copy of [`RECOVERY_S`].
    pub recovery_s: f64,
    /// Recorded copy of [`RECOVERY_ALLOWANCE_S`].
    pub recovery_allowance_s: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`FLOOR_DT_S`].
    pub floor_dt_s: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
}

fn axis_bits_equal(a: &AxisConstants, b: &AxisConstants) -> bool {
    a.slow_mas.to_bits() == b.slow_mas.to_bits()
        && a.slow_memory_s.to_bits() == b.slow_memory_s.to_bits()
        && a.fast_mas.to_bits() == b.fast_mas.to_bits()
        && a.fast_memory_s.to_bits() == b.fast_memory_s.to_bits()
        && a.mode_mas.to_bits() == b.mode_mas.to_bits()
        && a.mode_memory_s.to_bits() == b.mode_memory_s.to_bits()
        && a.mode_hz.to_bits() == b.mode_hz.to_bits()
        && a.noise_reference_mas2.to_bits() == b.noise_reference_mas2.to_bits()
        && a.noise_power.to_bits() == b.noise_power.to_bits()
}

fn axis_is_sane(a: &AxisConstants) -> bool {
    a.slow_mas > 0.0
        && a.slow_memory_s > 0.0
        && a.fast_mas > 0.0
        && a.fast_memory_s > 0.0
        && a.mode_mas > 0.0
        && a.mode_memory_s > 0.0
        && a.mode_hz > 0.0
        && a.noise_reference_mas2 > 0.0
        && a.noise_power > 0.0
}

impl AxisConstants {
    /// The variance of the axis with nothing measured,
    /// `s_slow² + s_fast² + 2 s_mode²`, mas².
    #[must_use]
    pub fn unmeasured_variance_mas2(&self) -> f64 {
        self.slow_mas * self.slow_mas
            + self.fast_mas * self.fast_mas
            + 2.0 * self.mode_mas * self.mode_mas
    }

    /// Noise variance of one centroid at this count rate,
    /// `r_ref · exp(−r_power · log(counts / 1e6))`, mas².
    #[must_use]
    pub fn noise_variance_mas2(&self, counts_per_s: f64) -> f64 {
        self.noise_reference_mas2
            * crate::math::exp(
                -self.noise_power * crate::math::log(counts_per_s / COUNTS_REFERENCE_PER_S),
            )
    }
}

impl LosJwstFineGuide {
    /// Card: the line of sight of JWST in fine guide, as its Fine Guidance
    /// Sensor reads it. The constructor takes no arguments.
    #[must_use]
    pub fn observatory() -> Self {
        let card = Self {
            x: X_AXIS,
            y: Y_AXIS,
            counts_reference_per_s: COUNTS_REFERENCE_PER_S,
            counts_min_per_s: COUNTS_MIN_PER_S,
            counts_max_per_s: COUNTS_MAX_PER_S,
            offset_max_mas: OFFSET_MAX_MAS,
            lock_field_half_width_arcsec: LOCK_FIELD_HALF_WIDTH_ARCSEC,
            quiet_offset_mas: QUIET_OFFSET_MAS,
            innovation_limit_sigma: INNOVATION_LIMIT_SIGMA,
            recovery_s: RECOVERY_S,
            recovery_allowance_s: RECOVERY_ALLOWANCE_S,
            gate_dt_s: GATE_DT_S,
            floor_dt_s: FLOOR_DT_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if card.is_valid() {
            card
        } else {
            ::core::panic!("line-of-sight card failed is_valid");
        }
    }

    /// Every recorded copy equals its crate constant bit for bit, and the
    /// constants are positive and in order.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        axis_bits_equal(&self.x, &X_AXIS)
            && axis_bits_equal(&self.y, &Y_AXIS)
            && self.counts_reference_per_s.to_bits() == COUNTS_REFERENCE_PER_S.to_bits()
            && self.counts_min_per_s.to_bits() == COUNTS_MIN_PER_S.to_bits()
            && self.counts_max_per_s.to_bits() == COUNTS_MAX_PER_S.to_bits()
            && self.offset_max_mas.to_bits() == OFFSET_MAX_MAS.to_bits()
            && self.lock_field_half_width_arcsec.to_bits() == LOCK_FIELD_HALF_WIDTH_ARCSEC.to_bits()
            && self.quiet_offset_mas.to_bits() == QUIET_OFFSET_MAS.to_bits()
            && self.innovation_limit_sigma.to_bits() == INNOVATION_LIMIT_SIGMA.to_bits()
            && self.recovery_s.to_bits() == RECOVERY_S.to_bits()
            && self.recovery_allowance_s.to_bits() == RECOVERY_ALLOWANCE_S.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && axis_is_sane(&self.x)
            && axis_is_sane(&self.y)
            && self.counts_min_per_s > 0.0
            && self.counts_max_per_s > self.counts_min_per_s
            && self.offset_max_mas > self.quiet_offset_mas
            && self.quiet_offset_mas > 0.0
            && self.innovation_limit_sigma > 0.0
            && self.recovery_s > 0.0
            && self.recovery_allowance_s > 0.0
            && self.recovery_allowance_s < self.recovery_s
            && self.lock_field_half_width_arcsec > 0.0
            && self.floor_dt_s > 0.0
            && self.gate_dt_s > self.floor_dt_s
            && self.residual_conservation_limit > 0.0
            && self.ledger_scale_mas2().is_finite()
            && self.ledger_scale_mas2() > 0.0
    }

    /// The scale of the ledger: the variance of both axes with nothing
    /// measured, mas². 1.52574 for this observatory.
    #[must_use]
    pub fn ledger_scale_mas2(&self) -> f64 {
        self.x.unmeasured_variance_mas2() + self.y.unmeasured_variance_mas2()
    }

    /// Whether `since_disturbance_s` after the last flagged centroid is at
    /// least [`RECOVERY_S`] less [`RECOVERY_ALLOWANCE_S`]: past the protocol's
    /// 30 s after an event.
    #[must_use]
    pub fn recovery_has_elapsed(&self, since_disturbance_s: f64) -> bool {
        since_disturbance_s >= self.recovery_s - self.recovery_allowance_s
    }

    /// Whether a centroid at this count rate has a noise variance on both
    /// axes: the count rate is above zero and the variance of each axis,
    /// `r_ref · (counts / 1e6)^(−r_power)`, is a finite 64-bit number. Under
    /// about 7e-237 counts per second the y axis's passes the largest one.
    #[must_use]
    pub fn count_rate_has_noise(&self, counts_per_s: f64) -> bool {
        counts_per_s > 0.0
            && self.x.noise_variance_mas2(counts_per_s).is_finite()
            && self.y.noise_variance_mas2(counts_per_s).is_finite()
    }

    /// Whether a count rate lies outside the count rates the noise law was
    /// fitted on, 22,900 to 2,820,000 counts per second: see
    /// [`FLAG_COUNT_RATE_BEYOND_COMPARED`].
    #[must_use]
    pub fn count_rate_beyond_compared(&self, counts_per_s: f64) -> bool {
        !(counts_per_s >= self.counts_min_per_s && counts_per_s <= self.counts_max_per_s)
    }

    /// Largest step the card stands behind: [`GATE_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.gate_dt_s.is_finite() && self.gate_dt_s > 0.0 {
            Some(self.gate_dt_s)
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

/// One axis of the filter: the four parts and their covariance. The
/// covariance is the upper triangle in the order slow-slow, slow-fast,
/// slow-cos, slow-sin, fast-fast, fast-cos, fast-sin, cos-cos, cos-sin,
/// sin-sin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisState {
    /// The slow part, mas.
    pub slow_mas: f64,
    /// The fast part, mas.
    pub fast_mas: f64,
    /// The mode, in phase, mas. This is the part of the mode the sensor reads.
    pub mode_cos_mas: f64,
    /// The mode, in quadrature, mas.
    pub mode_sin_mas: f64,
    /// Covariance of the four parts, upper triangle, mas².
    pub cov_mas2: [f64; 10],
}

impl AxisState {
    fn unmeasured(c: &AxisConstants) -> Self {
        let mut cov_mas2 = [0.0; 10];
        cov_mas2[0] = c.slow_mas * c.slow_mas;
        cov_mas2[4] = c.fast_mas * c.fast_mas;
        cov_mas2[7] = c.mode_mas * c.mode_mas;
        cov_mas2[9] = c.mode_mas * c.mode_mas;
        Self {
            slow_mas: 0.0,
            fast_mas: 0.0,
            mode_cos_mas: 0.0,
            mode_sin_mas: 0.0,
            cov_mas2,
        }
    }

    fn is_finite(&self) -> bool {
        let mut index = 0;
        while index < 10 {
            if !self.cov_mas2[index].is_finite() {
                return false;
            }
            index += 1;
        }
        self.slow_mas.is_finite()
            && self.fast_mas.is_finite()
            && self.mode_cos_mas.is_finite()
            && self.mode_sin_mas.is_finite()
    }

    fn variances_positive(&self) -> bool {
        self.cov_mas2[0] > 0.0
            && self.cov_mas2[4] > 0.0
            && self.cov_mas2[7] > 0.0
            && self.cov_mas2[9] > 0.0
    }
}

/// The filter. The published fields are part of the state; write them only
/// through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LosState {
    /// Time since the start of the run.
    pub t_s: f64,
    /// The lock point on x, arcsec in the sensor's ideal frame. The caller's.
    pub lock_x_arcsec: f64,
    /// The lock point on y, arcsec in the sensor's ideal frame. The caller's.
    pub lock_y_arcsec: f64,
    /// Time from the last disturbed centroid before the run, or known to be
    /// more than 5 mas from the lock point, to the first frame of the run, s.
    /// The caller's statement; the card cannot check it.
    pub quiet_before_start_s: f64,
    /// Time from the last centroid that carried [`FLAG_POINTING_DISTURBED`] or
    /// [`FLAG_BEYOND_FOUR_SIGMA`] to the end of this frame, s; at the start,
    /// `quiet_before_start_s`.
    pub since_disturbance_s: f64,
    /// The x axis.
    pub x: AxisState,
    /// The y axis.
    pub y: AxisState,
    /// Where the centroid was expected on x before the frame was used, offset from the lock point.
    pub predicted_x_mas: f64,
    /// Where the centroid was expected on y before the frame was used, offset from the lock point.
    pub predicted_y_mas: f64,
    /// Centroid offset less expected, x. Zero on a frame with no measurement.
    pub innovation_x_mas: f64,
    /// Centroid offset less expected, y. Zero on a frame with no measurement.
    pub innovation_y_mas: f64,
    /// The variance the filter states for `innovation_x_mas`. Zero on a frame with no measurement.
    pub innovation_variance_x_mas2: f64,
    /// The variance the filter states for `innovation_y_mas`. Zero on a frame with no measurement.
    pub innovation_variance_y_mas2: f64,
    /// The line of sight on x after the frame, offset from the lock point, noise taken out.
    pub estimate_x_mas: f64,
    /// The line of sight on y after the frame, offset from the lock point, noise taken out.
    pub estimate_y_mas: f64,
    /// The variance the filter states for `estimate_x_mas`.
    pub estimate_variance_x_mas2: f64,
    /// The variance the filter states for `estimate_y_mas`.
    pub estimate_variance_y_mas2: f64,
    /// `innovation_x² / variance_x + innovation_y² / variance_y` of this frame. Zero with no measurement.
    pub nis: f64,
    /// Frames with a measurement since the start of the run.
    pub measured_frames: f64,
    /// Sum over measured frames of `innovation_x² / variance_x`.
    pub nis_sum_x: f64,
    /// Sum over measured frames of `innovation_y² / variance_y`.
    pub nis_sum_y: f64,
    /// Trace of the x covariance after the step, mas².
    pub variance_x_mas2: f64,
    /// Trace of the y covariance after the step, mas².
    pub variance_y_mas2: f64,
    /// Signed trace less account, x. Positive is variance from nowhere.
    pub variance_defect_x_mas2: f64,
    /// Signed trace less account, y. Positive is variance from nowhere.
    pub variance_defect_y_mas2: f64,
    /// `( |defect_x| + |defect_y| ) / scale`.
    pub residual_conservation_error: f64,
    /// Frames with no measurement in a row, up to and including this one.
    /// Zero after a measured frame and at the start of a run.
    pub unmeasured_in_row: f64,
    /// The frame had no measurement.
    pub no_measurement: bool,
    /// The frame's centroid was more than 5 mas from the lock point.
    pub pointing_disturbed: bool,
    /// The frame's centroid was more than four stated standard deviations from where it was expected.
    pub beyond_four_sigma: bool,
    /// The frame ended less than 30 s after the last flagged centroid.
    pub recovering: bool,
}

impl LosState {
    /// Start at this lock point, time 0, with nothing known: every part at
    /// zero, each covariance at its unmeasured value and no frame with no
    /// measurement counted. `None` for a lock point that is not finite or
    /// more than [`LOCK_FIELD_HALF_WIDTH_ARCSEC`] from the centre on either
    /// axis, and for a `quiet_before_start_s` that is not finite or below zero.
    #[must_use]
    pub fn try_lock(
        card: &LosJwstFineGuide,
        lock_x_arcsec: f64,
        lock_y_arcsec: f64,
        quiet_before_start_s: f64,
    ) -> Option<Self> {
        if !card.is_valid()
            || !lock_x_arcsec.is_finite()
            || !lock_y_arcsec.is_finite()
            || !quiet_before_start_s.is_finite()
        {
            return None;
        }
        if !lock_is_inside(card, lock_x_arcsec, lock_y_arcsec) {
            return None;
        }
        if !(quiet_before_start_s >= 0.0) {
            return None;
        }
        let x = AxisState::unmeasured(&card.x);
        let y = AxisState::unmeasured(&card.y);
        Some(Self {
            t_s: 0.0,
            lock_x_arcsec,
            lock_y_arcsec,
            quiet_before_start_s,
            since_disturbance_s: quiet_before_start_s,
            x,
            y,
            predicted_x_mas: 0.0,
            predicted_y_mas: 0.0,
            innovation_x_mas: 0.0,
            innovation_y_mas: 0.0,
            innovation_variance_x_mas2: 0.0,
            innovation_variance_y_mas2: 0.0,
            estimate_x_mas: 0.0,
            estimate_y_mas: 0.0,
            estimate_variance_x_mas2: estimate_variance(&x.cov_mas2),
            estimate_variance_y_mas2: estimate_variance(&y.cov_mas2),
            nis: 0.0,
            measured_frames: 0.0,
            nis_sum_x: 0.0,
            nis_sum_y: 0.0,
            variance_x_mas2: trace(&x.cov_mas2),
            variance_y_mas2: trace(&y.cov_mas2),
            variance_defect_x_mas2: 0.0,
            variance_defect_y_mas2: 0.0,
            residual_conservation_error: 0.0,
            unmeasured_in_row: 0.0,
            no_measurement: false,
            pointing_disturbed: false,
            beyond_four_sigma: false,
            recovering: !card.recovery_has_elapsed(quiet_before_start_s),
        })
    }
}

fn lock_is_inside(card: &LosJwstFineGuide, lock_x_arcsec: f64, lock_y_arcsec: f64) -> bool {
    let half = card.lock_field_half_width_arcsec;
    lock_x_arcsec >= -half && lock_x_arcsec <= half && lock_y_arcsec >= -half && lock_y_arcsec <= half
}

fn trace(p: &[f64; 10]) -> f64 {
    p[0] + p[4] + p[7] + p[9]
}

/// `H P Hᵀ` with `H = [1, 1, 1, 0]`: the variance of slow + fast + cos.
fn estimate_variance(p: &[f64; 10]) -> f64 {
    p[0] + p[4] + p[7] + 2.0 * (p[1] + p[2] + p[5])
}

/// How the update of one axis is carried out. The card's own is the long form
/// with the best gain; the tests run the others and require the ledger to
/// name them.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Method {
    /// The gain is multiplied by this. 1 is the best gain.
    gain_scale: f64,
    /// `P − K hᵀ`, which holds only for the best gain, in place of the long form.
    short_form: bool,
    /// The drive is multiplied by this. 1 is the model's.
    drive_scale: f64,
}

const OWN_METHOD: Method = Method {
    gain_scale: 1.0,
    short_form: false,
    drive_scale: 1.0,
};

/// What one axis gives for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
struct AxisStep {
    axis: AxisState,
    predicted_mas: f64,
    innovation_mas: f64,
    innovation_variance_mas2: f64,
    nis: f64,
    estimate_mas: f64,
    estimate_variance_mas2: f64,
    variance_mas2: f64,
    defect_mas2: f64,
}

/// One frame of one axis. `measurement` is the offset from the lock point in
/// mas and the count rate, or `None`. `None` out when a number is not finite.
/// `Err(())` when the variance stated for the centroid is not positive.
fn advance_axis(
    c: &AxisConstants,
    axis: &AxisState,
    dt_s: f64,
    measurement: Option<(f64, f64)>,
    method: Method,
) -> Option<Result<AxisStep, ()>> {
    let r0 = crate::math::exp(-dt_s / c.slow_memory_s);
    let r1 = crate::math::exp(-dt_s / c.fast_memory_s);
    let r2 = crate::math::exp(-dt_s / c.mode_memory_s);
    let q0 = c.slow_mas * c.slow_mas * -crate::math::exp_m1(-2.0 * dt_s / c.slow_memory_s);
    let q1 = c.fast_mas * c.fast_mas * -crate::math::exp_m1(-2.0 * dt_s / c.fast_memory_s);
    let q2 = c.mode_mas * c.mode_mas * -crate::math::exp_m1(-2.0 * dt_s / c.mode_memory_s);
    let theta = 2.0 * core::f64::consts::PI * c.mode_hz * dt_s;
    let co = crate::math::cos(theta);
    let si = crate::math::sin(theta);
    let [p00, p01, p02, p03, p11, p12, p13, p22, p23, p33] = axis.cov_mas2;
    // The account of the variance, from the variances as they came in.
    let mut account = r0 * r0 * p00 + r1 * r1 * p11 + r2 * r2 * (p22 + p33) + q0 + q1 + 2.0 * q2;
    // Predict.
    let d0 = q0 * method.drive_scale;
    let d1 = q1 * method.drive_scale;
    let d2 = q2 * method.drive_scale;
    let mut n0 = r0 * axis.slow_mas;
    let mut n1 = r1 * axis.fast_mas;
    let mut n2 = r2 * (co * axis.mode_cos_mas + si * axis.mode_sin_mas);
    let mut n3 = r2 * (co * axis.mode_sin_mas - si * axis.mode_cos_mas);
    let a00 = r0 * r0 * p00 + d0;
    let a01 = r0 * r1 * p01;
    let a02 = r0 * r2 * (co * p02 + si * p03);
    let a03 = r0 * r2 * (co * p03 - si * p02);
    let a11 = r1 * r1 * p11 + d1;
    let a12 = r1 * r2 * (co * p12 + si * p13);
    let a13 = r1 * r2 * (co * p13 - si * p12);
    let r22 = r2 * r2;
    let a22 = r22 * (co * co * p22 + 2.0 * co * si * p23 + si * si * p33) + d2;
    let a23 = r22 * (co * si * (p33 - p22) + (co * co - si * si) * p23);
    let a33 = r22 * (si * si * p22 - 2.0 * co * si * p23 + co * co * p33) + d2;
    let predicted_mas = n0 + n1 + n2;
    let mut cov = [a00, a01, a02, a03, a11, a12, a13, a22, a23, a33];
    let mut innovation_mas = 0.0;
    let mut innovation_variance_mas2 = 0.0;
    let mut nis = 0.0;
    if let Some((offset_mas, counts_per_s)) = measurement {
        let noise_mas2 = c.noise_variance_mas2(counts_per_s);
        let h0 = a00 + a01 + a02;
        let h1 = a01 + a11 + a12;
        let h2 = a02 + a12 + a22;
        let h3 = a03 + a13 + a23;
        let s = h0 + h1 + h2 + noise_mas2;
        if !s.is_finite() {
            return None;
        }
        if !(s > 0.0) {
            return Some(Err(()));
        }
        let k0 = method.gain_scale * h0 / s;
        let k1 = method.gain_scale * h1 / s;
        let k2 = method.gain_scale * h2 / s;
        let k3 = method.gain_scale * h3 / s;
        let e = offset_mas - predicted_mas;
        n0 += k0 * e;
        n1 += k1 * e;
        n2 += k2 * e;
        n3 += k3 * e;
        cov = if method.short_form {
            [
                a00 - k0 * h0,
                a01 - k0 * h1,
                a02 - k0 * h2,
                a03 - k0 * h3,
                a11 - k1 * h1,
                a12 - k1 * h2,
                a13 - k1 * h3,
                a22 - k2 * h2,
                a23 - k2 * h3,
                a33 - k3 * h3,
            ]
        } else {
            [
                a00 - 2.0 * k0 * h0 + k0 * k0 * s,
                a01 - k0 * h1 - h0 * k1 + k0 * k1 * s,
                a02 - k0 * h2 - h0 * k2 + k0 * k2 * s,
                a03 - k0 * h3 - h0 * k3 + k0 * k3 * s,
                a11 - 2.0 * k1 * h1 + k1 * k1 * s,
                a12 - k1 * h2 - h1 * k2 + k1 * k2 * s,
                a13 - k1 * h3 - h1 * k3 + k1 * k3 * s,
                a22 - 2.0 * k2 * h2 + k2 * k2 * s,
                a23 - k2 * h3 - h2 * k3 + k2 * k3 * s,
                a33 - 2.0 * k3 * h3 + k3 * k3 * s,
            ]
        };
        account -= (h0 * h0 + h1 * h1 + h2 * h2 + h3 * h3) / s;
        innovation_mas = e;
        innovation_variance_mas2 = s;
        nis = e * e / s;
    }
    let next = AxisState {
        slow_mas: n0,
        fast_mas: n1,
        mode_cos_mas: n2,
        mode_sin_mas: n3,
        cov_mas2: cov,
    };
    let variance_mas2 = trace(&cov);
    let out = AxisStep {
        axis: next,
        predicted_mas,
        innovation_mas,
        innovation_variance_mas2,
        nis,
        estimate_mas: n0 + n1 + n2,
        estimate_variance_mas2: estimate_variance(&cov),
        variance_mas2,
        defect_mas2: variance_mas2 - account,
    };
    if next.is_finite()
        && out.predicted_mas.is_finite()
        && out.innovation_mas.is_finite()
        && out.nis.is_finite()
        && out.estimate_mas.is_finite()
        && out.estimate_variance_mas2.is_finite()
        && out.defect_mas2.is_finite()
    {
        Some(Ok(out))
    } else {
        None
    }
}

/// Advance one frame and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    card: &LosJwstFineGuide,
    state: &mut LosState,
    frame: &mut LastStateFrame64,
    load: LosLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(card, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("los_jwst_fine_guide", exceedances(card, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    card: &LosJwstFineGuide,
    state: &mut LosState,
    frame: &mut LastStateFrame64,
    load: LosLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(card, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    card: &LosJwstFineGuide,
    state: &mut LosState,
    frame: &mut LastStateFrame64,
    load: LosLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_by(card, state, frame, load, dt_s, OWN_METHOD, 0.0)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut LosState) {
    state.t_s = normal_or_zero(state.t_s);
    state.lock_x_arcsec = normal_or_zero(state.lock_x_arcsec);
    state.lock_y_arcsec = normal_or_zero(state.lock_y_arcsec);
    state.quiet_before_start_s = normal_or_zero(state.quiet_before_start_s);
    state.since_disturbance_s = normal_or_zero(state.since_disturbance_s);
    normal_or_zero_axis_state(&mut state.x);
    normal_or_zero_axis_state(&mut state.y);
    state.predicted_x_mas = normal_or_zero(state.predicted_x_mas);
    state.predicted_y_mas = normal_or_zero(state.predicted_y_mas);
    state.innovation_x_mas = normal_or_zero(state.innovation_x_mas);
    state.innovation_y_mas = normal_or_zero(state.innovation_y_mas);
    state.innovation_variance_x_mas2 = normal_or_zero(state.innovation_variance_x_mas2);
    state.innovation_variance_y_mas2 = normal_or_zero(state.innovation_variance_y_mas2);
    state.estimate_x_mas = normal_or_zero(state.estimate_x_mas);
    state.estimate_y_mas = normal_or_zero(state.estimate_y_mas);
    state.estimate_variance_x_mas2 = normal_or_zero(state.estimate_variance_x_mas2);
    state.estimate_variance_y_mas2 = normal_or_zero(state.estimate_variance_y_mas2);
    state.nis = normal_or_zero(state.nis);
    state.measured_frames = normal_or_zero(state.measured_frames);
    state.nis_sum_x = normal_or_zero(state.nis_sum_x);
    state.nis_sum_y = normal_or_zero(state.nis_sum_y);
    state.variance_x_mas2 = normal_or_zero(state.variance_x_mas2);
    state.variance_y_mas2 = normal_or_zero(state.variance_y_mas2);
    state.variance_defect_x_mas2 = normal_or_zero(state.variance_defect_x_mas2);
    state.variance_defect_y_mas2 = normal_or_zero(state.variance_defect_y_mas2);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.unmeasured_in_row = normal_or_zero(state.unmeasured_in_row);
}

/// Each value of `values` below the normal range written as zero with its
/// sign ([`crate::normal`]).
fn normal_or_zero_axis_state(values: &mut AxisState) {
    values.slow_mas = normal_or_zero(values.slow_mas);
    values.fast_mas = normal_or_zero(values.fast_mas);
    values.mode_cos_mas = normal_or_zero(values.mode_cos_mas);
    values.mode_sin_mas = normal_or_zero(values.mode_sin_mas);
    normal_or_zero_each(&mut values.cov_mas2);
}

/// The step with a chosen method and a number added to the x defect. The
/// card's own step is `OWN_METHOD` and zero.
fn step_by(
    card: &LosJwstFineGuide,
    state: &mut LosState,
    frame: &mut LastStateFrame64,
    load: LosLoad,
    dt_s: f64,
    method: Method,
    defect_bias_mas2: f64,
) -> Result<(), StepReject> {
    let measurement = validate(card, state, load, dt_s)?;
    let (mx, my) = match measurement {
        Some((ox, oy)) => (Some((ox, load.counts_per_s)), Some((oy, load.counts_per_s))),
        None => (None, None),
    };
    let x = match advance_axis(&card.x, &state.x, dt_s, mx, method) {
        Some(Ok(out)) => out,
        Some(Err(())) => return Err(StepReject::InvalidCovariance),
        None => return Err(StepReject::NonFiniteState),
    };
    let y = match advance_axis(&card.y, &state.y, dt_s, my, method) {
        Some(Ok(out)) => out,
        Some(Err(())) => return Err(StepReject::InvalidCovariance),
        None => return Err(StepReject::NonFiniteState),
    };
    let count_rate_beyond_compared =
        measurement.is_some() && card.count_rate_beyond_compared(load.counts_per_s);
    commit(card, state, frame, dt_s, measurement, count_rate_beyond_compared, x, y, defect_bias_mas2)
}

/// The checks before a step. Gives the offset of the centroid from the lock
/// point on x and y in mas when the frame is measured.
fn validate(
    card: &LosJwstFineGuide,
    state: &LosState,
    load: LosLoad,
    dt_s: f64,
) -> Result<Option<(f64, f64)>, StepReject> {
    if !(dt_s.is_finite()
        && load.measured.is_finite()
        && load.centroid_x_arcsec.is_finite()
        && load.centroid_y_arcsec.is_finite()
        && load.counts_per_s.is_finite())
    {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !card.is_valid() {
        return Err(StepReject::InvalidObservatory);
    }
    if !lock_is_inside(card, state.lock_x_arcsec, state.lock_y_arcsec) {
        return Err(StepReject::LockOutsideSensorField);
    }
    if !(state.quiet_before_start_s >= 0.0 && state.since_disturbance_s >= 0.0) {
        return Err(StepReject::QuietBeforeStartBelowZero);
    }
    if !(state.x.variances_positive() && state.y.variances_positive()) {
        return Err(StepReject::InvalidCovariance);
    }
    if load.measured == 0.0 {
        if load.centroid_x_arcsec != 0.0 || load.centroid_y_arcsec != 0.0 || load.counts_per_s != 0.0
        {
            return Err(StepReject::UnmeasuredFrameCarriesNumbers);
        }
        return Ok(None);
    }
    if load.measured != 1.0 {
        return Err(StepReject::MeasuredNotZeroOrOne);
    }
    if !card.count_rate_has_noise(load.counts_per_s) {
        return Err(StepReject::CountRateNotPositive);
    }
    let offset_x_mas = (load.centroid_x_arcsec - state.lock_x_arcsec) * MAS_PER_ARCSEC;
    let offset_y_mas = (load.centroid_y_arcsec - state.lock_y_arcsec) * MAS_PER_ARCSEC;
    if !(offset_x_mas.abs() <= card.offset_max_mas && offset_y_mas.abs() <= card.offset_max_mas) {
        return Err(StepReject::OffsetOutsideCard);
    }
    Ok(Some((offset_x_mas, offset_y_mas)))
}

#[allow(clippy::too_many_arguments)]
fn commit(
    card: &LosJwstFineGuide,
    state: &mut LosState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
    measurement: Option<(f64, f64)>,
    count_rate_beyond_compared: bool,
    x: AxisStep,
    y: AxisStep,
    defect_bias_mas2: f64,
) -> Result<(), StepReject> {
    let saved = *state;
    let scale = card.ledger_scale_mas2();
    if !(scale.is_finite() && scale > 0.0) {
        return Err(StepReject::InvalidObservatory);
    }
    state.t_s += dt_s;
    state.x = x.axis;
    state.y = y.axis;
    state.predicted_x_mas = x.predicted_mas;
    state.predicted_y_mas = y.predicted_mas;
    state.innovation_x_mas = x.innovation_mas;
    state.innovation_y_mas = y.innovation_mas;
    state.innovation_variance_x_mas2 = x.innovation_variance_mas2;
    state.innovation_variance_y_mas2 = y.innovation_variance_mas2;
    state.estimate_x_mas = x.estimate_mas;
    state.estimate_y_mas = y.estimate_mas;
    state.estimate_variance_x_mas2 = x.estimate_variance_mas2;
    state.estimate_variance_y_mas2 = y.estimate_variance_mas2;
    state.nis = x.nis + y.nis;
    state.variance_x_mas2 = x.variance_mas2;
    state.variance_y_mas2 = y.variance_mas2;
    state.variance_defect_x_mas2 = x.defect_mas2 + defect_bias_mas2;
    state.variance_defect_y_mas2 = y.defect_mas2;
    state.residual_conservation_error =
        (state.variance_defect_x_mas2.abs() + state.variance_defect_y_mas2.abs()) / scale;
    state.no_measurement = measurement.is_none();
    state.unmeasured_in_row = if measurement.is_none() {
        saved.unmeasured_in_row + 1.0
    } else {
        0.0
    };
    let mut innovation_beyond_limit = false;
    let mut disturbed = false;
    if let Some((offset_x_mas, offset_y_mas)) = measurement {
        disturbed = offset_x_mas.abs() > card.quiet_offset_mas
            || offset_y_mas.abs() > card.quiet_offset_mas;
    }
    if measurement.is_some() {
        state.measured_frames += 1.0;
        state.nis_sum_x += x.nis;
        state.nis_sum_y += y.nis;
        let limit = card.innovation_limit_sigma * card.innovation_limit_sigma;
        innovation_beyond_limit = x.innovation_mas * x.innovation_mas
            > limit * x.innovation_variance_mas2
            || y.innovation_mas * y.innovation_mas > limit * y.innovation_variance_mas2;
    }
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > card.residual_conservation_limit {
        let created = defect_is_positive(state.variance_defect_x_mas2, state.variance_defect_y_mas2);
        *state = saved;
        return if created {
            Err(StepReject::SpontaneousGeneration)
        } else {
            Err(StepReject::FirstLawBreach)
        };
    }
    if !(state.x.variances_positive() && state.y.variances_positive()) {
        *state = saved;
        return Err(StepReject::InvalidCovariance);
    }
    // The conditions of the frame, reported after the ledger closed.
    state.pointing_disturbed = disturbed;
    state.beyond_four_sigma = innovation_beyond_limit;
    state.since_disturbance_s = if disturbed || innovation_beyond_limit {
        0.0
    } else {
        saved.since_disturbance_s + dt_s
    };
    state.recovering =
        !(disturbed || innovation_beyond_limit) && !card.recovery_has_elapsed(state.since_disturbance_s);
    if !state.since_disturbance_s.is_finite() {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    publish(state, frame, count_rate_beyond_compared);
    Ok(())
}

/// The sign of the two defects together. When they cancel exactly, the
/// larger one in size names the step.
fn defect_is_positive(defect_x_mas2: f64, defect_y_mas2: f64) -> bool {
    let total = defect_x_mas2 + defect_y_mas2;
    if total != 0.0 {
        total > 0.0
    } else if defect_x_mas2.abs() >= defect_y_mas2.abs() {
        defect_x_mas2 > 0.0
    } else {
        defect_y_mas2 > 0.0
    }
}

fn publish(state: &LosState, frame: &mut LastStateFrame64, count_rate_beyond_compared: bool) {
    frame.t = state.t_s;
    frame.pos[0] = state.predicted_x_mas as f32;
    frame.pos[1] = state.predicted_y_mas as f32;
    frame.pos[2] = state.innovation_variance_x_mas2 as f32;
    frame.vel[0] = state.estimate_x_mas as f32;
    frame.vel[1] = state.estimate_y_mas as f32;
    frame.vel[2] = state.innovation_variance_y_mas2 as f32;
    frame.force_torque = state.nis as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.no_measurement {
        flags |= FLAG_NO_MEASUREMENT;
    }
    if state.pointing_disturbed {
        flags |= FLAG_POINTING_DISTURBED;
    }
    if state.beyond_four_sigma {
        flags |= FLAG_BEYOND_FOUR_SIGMA;
    }
    if state.recovering {
        flags |= FLAG_RECOVERING;
    }
    if count_rate_beyond_compared {
        flags |= FLAG_COUNT_RATE_BEYOND_COMPARED;
    }
    frame.flags = flags;
}

fn state_is_finite(state: &LosState) -> bool {
    state.t_s.is_finite()
        && state.lock_x_arcsec.is_finite()
        && state.lock_y_arcsec.is_finite()
        && state.quiet_before_start_s.is_finite()
        && state.since_disturbance_s.is_finite()
        && state.x.is_finite()
        && state.y.is_finite()
        && state.predicted_x_mas.is_finite()
        && state.predicted_y_mas.is_finite()
        && state.innovation_x_mas.is_finite()
        && state.innovation_y_mas.is_finite()
        && state.innovation_variance_x_mas2.is_finite()
        && state.innovation_variance_y_mas2.is_finite()
        && state.estimate_x_mas.is_finite()
        && state.estimate_y_mas.is_finite()
        && state.estimate_variance_x_mas2.is_finite()
        && state.estimate_variance_y_mas2.is_finite()
        && state.nis.is_finite()
        && state.measured_frames.is_finite()
        && state.nis_sum_x.is_finite()
        && state.nis_sum_y.is_finite()
        && state.variance_x_mas2.is_finite()
        && state.variance_y_mas2.is_finite()
        && state.variance_defect_x_mas2.is_finite()
        && state.variance_defect_y_mas2.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.unmeasured_in_row.is_finite()
}


/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    NO_MEASUREMENT_KIND,
    DISTURBED_X,
    DISTURBED_Y,
    SIGMA_X,
    SIGMA_Y,
    RECOVERING_KIND,
    COUNTS_LOW,
    COUNTS_HIGH,
];

const NO_MEASUREMENT_KIND: ExceedanceKind = ExceedanceKind::state(
    "FLAG_NO_MEASUREMENT",
    FLAG_NO_MEASUREMENT,
    "measurement",
    "the frame has no measured centroid: it was predicted and not updated; no number to compare",
);
const DISTURBED_WORDS: &str = "5 mas from the lock point on the axis, the quiet offset: past it the card was measured worse than the last centroid";
const DISTURBED_X: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_POINTING_DISTURBED",
    FLAG_POINTING_DISTURBED,
    "offset_x_mas_magnitude",
    "mas",
    Side::Above,
    QUIET_OFFSET_MAS,
    DISTURBED_WORDS,
);
const DISTURBED_Y: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_POINTING_DISTURBED",
    FLAG_POINTING_DISTURBED,
    "offset_y_mas_magnitude",
    "mas",
    Side::Above,
    QUIET_OFFSET_MAS,
    DISTURBED_WORDS,
);
const SIGMA_WORDS: &str = "four stated standard deviations of the innovation on the axis";
const SIGMA_X: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_BEYOND_FOUR_SIGMA",
    FLAG_BEYOND_FOUR_SIGMA,
    "innovation_x_sigma",
    "1",
    Side::Above,
    INNOVATION_LIMIT_SIGMA,
    SIGMA_WORDS,
);
const SIGMA_Y: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_BEYOND_FOUR_SIGMA",
    FLAG_BEYOND_FOUR_SIGMA,
    "innovation_y_sigma",
    "1",
    Side::Above,
    INNOVATION_LIMIT_SIGMA,
    SIGMA_WORDS,
);
const RECOVERING_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_RECOVERING",
    FLAG_RECOVERING,
    "since_disturbance_s",
    "s",
    Side::Below,
    RECOVERY_S,
    "30 s after the last centroid that was disturbed or beyond four sigma, less its rounding allowance",
);
const COUNTS_WORDS: &str = "the count rates the noise law was fitted on, 22,900 to 2,820,000 counts per second";
const COUNTS_LOW: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_COUNT_RATE_BEYOND_COMPARED",
    FLAG_COUNT_RATE_BEYOND_COMPARED,
    "counts_per_s",
    "counts/s",
    Side::Below,
    COUNTS_MIN_PER_S,
    COUNTS_WORDS,
);
const COUNTS_HIGH: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_COUNT_RATE_BEYOND_COMPARED",
    FLAG_COUNT_RATE_BEYOND_COMPARED,
    "counts_per_s",
    "counts/s",
    Side::Above,
    COUNTS_MAX_PER_S,
    COUNTS_WORDS,
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The offsets of the centroid from the lock point are read again as the
/// step read them (`validate`); the innovation in standard deviations is
/// its size over the square root of its variance.
pub fn exceedances(
    card: &LosJwstFineGuide,
    before: &LosState,
    after: &LosState,
    load: LosLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.no_measurement, Exceedance::state(NO_MEASUREMENT_KIND));
    if after.pointing_disturbed {
        if let Ok(Some((offset_x_mas, offset_y_mas))) = validate(card, before, load, dt_s) {
            let (x, y) = (offset_x_mas.abs(), offset_y_mas.abs());
            records.push_if(x > card.quiet_offset_mas, Exceedance::at(DISTURBED_X, card.quiet_offset_mas, x));
            records.push_if(y > card.quiet_offset_mas, Exceedance::at(DISTURBED_Y, card.quiet_offset_mas, y));
        }
    }
    if after.beyond_four_sigma {
        let sigma = card.innovation_limit_sigma;
        let limit = sigma * sigma;
        let (ix, vx) = (after.innovation_x_mas, after.innovation_variance_x_mas2);
        let (iy, vy) = (after.innovation_y_mas, after.innovation_variance_y_mas2);
        records.push_if(ix * ix > limit * vx, Exceedance::at(SIGMA_X, sigma, ix.abs() / vx.sqrt()));
        records.push_if(iy * iy > limit * vy, Exceedance::at(SIGMA_Y, sigma, iy.abs() / vy.sqrt()));
    }
    records.push_if(after.recovering, Exceedance::at(RECOVERING_KIND, card.recovery_s, after.since_disturbance_s));
    if !after.no_measurement && card.count_rate_beyond_compared(load.counts_per_s) {
        let counts = load.counts_per_s;
        records.push_if(!(counts >= card.counts_min_per_s), Exceedance::at(COUNTS_LOW, card.counts_min_per_s, counts));
        records.push_if(!(counts <= card.counts_max_per_s), Exceedance::at(COUNTS_HIGH, card.counts_max_per_s, counts));
    }
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::assert_ne;
    use std::println;
    use std::vec::Vec;

    use super::{
        AxisConstants, COUNTS_MAX_PER_S, COUNTS_MIN_PER_S, COUNTS_REFERENCE_PER_S,
        FLAG_BEYOND_FOUR_SIGMA, FLAG_COUNT_RATE_BEYOND_COMPARED, FLAG_NO_MEASUREMENT, FLAG_POINTING_DISTURBED, FLAG_RECOVERING,
        FLOOR_DT_S, FRAME_S, GATE_DT_S, INNOVATION_LIMIT_SIGMA, LOCK_FIELD_HALF_WIDTH_ARCSEC,
        LOCK_X_MAX_ARCSEC, LOCK_X_MIN_ARCSEC, LOCK_Y_MAX_ARCSEC, LOCK_Y_MIN_ARCSEC, LosJwstFineGuide,
        LosLoad, LosState, MEASUREMENT_GAP_MAX_FRAMES, Method, OFFSET_MAX_MAS, OWN_METHOD,
        QUIET_OFFSET_MAS, RECOVERY_ALLOWANCE_S, RECOVERY_S, RESIDUAL_CONSERVATION_LIMIT,
        StepReject, X_AXIS, Y_AXIS, step, step_by,
    };
    use crate::frame::LastStateFrame64;

    ::core::include!("los_jwst_fine_guide_reference.rs");

    const NO_MEASUREMENT: LosLoad = LosLoad {
        measured: 0.0,
        centroid_x_arcsec: 0.0,
        centroid_y_arcsec: 0.0,
        counts_per_s: 0.0,
    };

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

    fn state_bits(state: &LosState) -> Vec<u64> {
        let mut bits = Vec::new();
        bits.push(state.t_s.to_bits());
        bits.push(state.lock_x_arcsec.to_bits());
        bits.push(state.lock_y_arcsec.to_bits());
        bits.push(state.quiet_before_start_s.to_bits());
        for axis in [&state.x, &state.y] {
            bits.push(axis.slow_mas.to_bits());
            bits.push(axis.fast_mas.to_bits());
            bits.push(axis.mode_cos_mas.to_bits());
            bits.push(axis.mode_sin_mas.to_bits());
            for entry in axis.cov_mas2 {
                bits.push(entry.to_bits());
            }
        }
        for value in [
            state.predicted_x_mas,
            state.predicted_y_mas,
            state.innovation_x_mas,
            state.innovation_y_mas,
            state.innovation_variance_x_mas2,
            state.innovation_variance_y_mas2,
            state.estimate_x_mas,
            state.estimate_y_mas,
            state.estimate_variance_x_mas2,
            state.estimate_variance_y_mas2,
            state.nis,
            state.measured_frames,
            state.nis_sum_x,
            state.nis_sum_y,
            state.variance_x_mas2,
            state.variance_y_mas2,
            state.variance_defect_x_mas2,
            state.variance_defect_y_mas2,
            state.residual_conservation_error,
            state.unmeasured_in_row,
        ] {
            bits.push(value.to_bits());
        }
        bits.push(u64::from(state.no_measurement));
        bits
    }

    /// Agreement to `tolerance` of the larger of the expected size and 1e-3.
    fn close(got: f64, expected: f64, tolerance: f64) -> bool {
        let size = if expected.abs() > 1.0e-3 { expected.abs() } else { 1.0e-3 };
        (got - expected).abs() <= tolerance * size
    }

    fn load_of(row: (f64, f64, f64, f64)) -> LosLoad {
        LosLoad {
            measured: row.0,
            centroid_x_arcsec: row.1,
            centroid_y_arcsec: row.2,
            counts_per_s: row.3,
        }
    }

    fn reference_start() -> (LosJwstFineGuide, LosState) {
        let card = LosJwstFineGuide::observatory();
        let state = LosState::try_lock(&card, LOCK_X_ARCSEC, LOCK_Y_ARCSEC, RECOVERY_S).unwrap();
        (card, state)
    }

    /// A measured frame at the lock point plus an offset, at a middling count rate.
    fn frame_at(state: &LosState, offset_x_mas: f64, offset_y_mas: f64) -> LosLoad {
        LosLoad {
            measured: 1.0,
            centroid_x_arcsec: state.lock_x_arcsec + offset_x_mas / 1000.0,
            centroid_y_arcsec: state.lock_y_arcsec + offset_y_mas / 1000.0,
            counts_per_s: 551_754.4,
        }
    }

    /// The state after the first `frames` frames of the reference run.
    fn after_frames(frames: usize) -> (LosJwstFineGuide, LosState, LastStateFrame64) {
        let (card, mut state) = reference_start();
        let mut frame = blank_frame();
        let mut index = 0;
        while index < frames {
            step(&card, &mut state, &mut frame, load_of(FRAMES[index]), FRAME_S).unwrap();
            index += 1;
        }
        (card, state, frame)
    }

    /// One frame with no measurement, with the count of such frames in a row
    /// set back to zero by hand first. It shows the filter's arithmetic over a
    /// gap longer than the card takes; the card itself refuses such a gap.
    fn predict_past_the_card(card: &LosJwstFineGuide, state: &mut LosState, frame: &mut LastStateFrame64) {
        state.unmeasured_in_row = 0.0;
        step(card, state, frame, NO_MEASUREMENT, FRAME_S).unwrap();
    }

    /// A refused step leaves the state and the frame bit for bit as they were.
    fn refused(
        card: &LosJwstFineGuide,
        state: &LosState,
        load: LosLoad,
        dt_s: f64,
        expected: StepReject,
    ) {
        let mut trial = *state;
        let mut frame = blank_frame();
        let before = frame.to_bytes();
        assert_eq!(step(card, &mut trial, &mut frame, load, dt_s), Err(expected));
        assert_eq!(state_bits(&trial), state_bits(state), "{expected:?} changed the state");
        assert_eq!(frame.to_bytes(), before, "{expected:?} changed the frame");
    }

    #[test]
    fn the_card_locks_its_constants() {
        let card = LosJwstFineGuide::observatory();
        assert!(card.is_valid());
        // `analysis/out/card_constants.json` of the research record, figure for figure.
        assert_eq!(
            X_AXIS,
            AxisConstants {
                slow_mas: 1.03257,
                slow_memory_s: 66.7485,
                fast_mas: 0.142973,
                fast_memory_s: 0.0980239,
                mode_mas: 0.136597,
                mode_memory_s: 5.88667,
                mode_hz: 0.543703,
                noise_reference_mas2: 0.0360528,
                noise_power: 1.15083,
            }
        );
        assert_eq!(
            Y_AXIS,
            AxisConstants {
                slow_mas: 0.455171,
                slow_memory_s: 12.2188,
                fast_mas: 0.156568,
                fast_memory_s: 0.0943062,
                mode_mas: 0.291622,
                mode_memory_s: 14.6184,
                mode_hz: 0.302547,
                noise_reference_mas2: 0.0253144,
                noise_power: 1.27308,
            }
        );
        assert_eq!(COUNTS_REFERENCE_PER_S, 1.0e6);
        assert_eq!(COUNTS_MIN_PER_S, 22_900.0);
        assert_eq!(COUNTS_MAX_PER_S, 2_820_000.0);
        assert_eq!(OFFSET_MAX_MAS, 250.0);
        // The medians of the good centroids of the files the comparison
        // answered, by the record's protocol, as `runs.json` of
        // `analysis/out/11_enforced/` holds them.
        assert_eq!(LOCK_X_MIN_ARCSEC, -61.1512947);
        assert_eq!(LOCK_X_MAX_ARCSEC, 60.4564934);
        assert_eq!(LOCK_Y_MIN_ARCSEC, -64.2767);
        assert_eq!(LOCK_Y_MAX_ARCSEC, 50.1812668);
        assert_eq!(MEASUREMENT_GAP_MAX_FRAMES, 10.0);
        assert_eq!(QUIET_OFFSET_MAS, 5.0);
        assert_eq!(INNOVATION_LIMIT_SIGMA, 4.0);
        assert_eq!(RECOVERY_S, 30.0);
        // 2^-36 s: the spacing of 64-bit numbers just under 2^17 s, which is twice
        // the most one time under 2^17 s read from a decimal can be off by.
        assert_eq!(RECOVERY_ALLOWANCE_S, 131_072.0 - f64::from_bits(131_072.0_f64.to_bits() - 1));
        assert_eq!(RECOVERY_ALLOWANCE_S.to_bits(), (1.0_f64 / 68_719_476_736.0).to_bits());
        assert_eq!(GATE_DT_S, 0.0645);
        assert_eq!(FLOOR_DT_S, 0.0635);
        assert_eq!(RESIDUAL_CONSERVATION_LIMIT, 1.0e-12);
        assert!(close(card.ledger_scale_mas2(), 1.525740524, 1.0e-9));

        // A card with one constant moved is not this observatory, and no step is taken with it.
        let (_, state) = reference_start();
        let load = load_of(FRAMES[0]);
        let mut moved = card;
        moved.x.mode_hz = 0.55;
        assert!(!moved.is_valid());
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
        let mut moved = card;
        moved.y.noise_power = f64::from_bits(Y_AXIS.noise_power.to_bits() + 1);
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
        let mut moved = card;
        moved.counts_max_per_s = 1.0e9;
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
        let mut moved = card;
        moved.residual_conservation_limit = 1.0;
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
        // A card whose line for a disturbance, whose line for the innovation or
        // whose recovery time is moved is not this card.
        let mut moved = card;
        moved.quiet_offset_mas = 50.0;
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
        let mut moved = card;
        moved.innovation_limit_sigma = 40.0;
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
        let mut moved = card;
        moved.recovery_s = 0.064;
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
        let mut moved = card;
        moved.recovery_allowance_s = 1.0;
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
        // Nor is one whose sensor field is moved.
        let mut moved = card;
        moved.lock_field_half_width_arcsec = 80.0;
        refused(&moved, &state, load, FRAME_S, StepReject::InvalidObservatory);
    }

    #[test]
    fn types_are_copy() {
        assert_copy::<LosJwstFineGuide>();
        assert_copy::<LosState>();
        assert_copy::<LosLoad>();
        assert_copy::<AxisConstants>();
    }

    #[test]
    fn the_reference_run_is_reproduced_after_1_2_100_and_its_last_landed_frame() {
        let (card, mut state) = reference_start();
        let mut frame = blank_frame();
        let mut snapshot = 0;
        let mut unmeasured = 0;
        let mut worst = 0.0_f64;
        let tolerance = 1.0e-10;
        for (index, row) in FRAMES.iter().take(FIRST_REFUSED_FRAME).enumerate() {
            step(&card, &mut state, &mut frame, load_of(*row), FRAME_S).unwrap();
            if state.no_measurement {
                unmeasured += 1;
            }
            if snapshot < SNAPSHOTS.len() && SNAPSHOTS[snapshot].frames == index + 1 {
                let want = SNAPSHOTS[snapshot];
                let got_m_x = [state.x.slow_mas, state.x.fast_mas, state.x.mode_cos_mas, state.x.mode_sin_mas];
                let got_m_y = [state.y.slow_mas, state.y.fast_mas, state.y.mode_cos_mas, state.y.mode_sin_mas];
                let mut pairs: Vec<(f64, f64)> = Vec::new();
                for part in 0..4 {
                    pairs.push((got_m_x[part], want.m_x[part]));
                    pairs.push((got_m_y[part], want.m_y[part]));
                }
                for entry in 0..10 {
                    pairs.push((state.x.cov_mas2[entry], want.p_x[entry]));
                    pairs.push((state.y.cov_mas2[entry], want.p_y[entry]));
                }
                pairs.push((state.predicted_x_mas, want.predicted[0]));
                pairs.push((state.predicted_y_mas, want.predicted[1]));
                pairs.push((state.innovation_x_mas, want.innovation[0]));
                pairs.push((state.innovation_y_mas, want.innovation[1]));
                pairs.push((state.innovation_variance_x_mas2, want.innovation_variance[0]));
                pairs.push((state.innovation_variance_y_mas2, want.innovation_variance[1]));
                pairs.push((state.estimate_x_mas, want.estimate[0]));
                pairs.push((state.estimate_y_mas, want.estimate[1]));
                pairs.push((state.estimate_variance_x_mas2, want.estimate_variance[0]));
                pairs.push((state.estimate_variance_y_mas2, want.estimate_variance[1]));
                pairs.push((state.variance_x_mas2, want.variance[0]));
                pairs.push((state.variance_y_mas2, want.variance[1]));
                pairs.push((state.nis_sum_x, want.nis_sum[0]));
                pairs.push((state.nis_sum_y, want.nis_sum[1]));
                for (got, expected) in pairs {
                    let size = if expected.abs() > 1.0e-3 { expected.abs() } else { 1.0e-3 };
                    let miss = (got - expected).abs() / size;
                    if miss > worst {
                        worst = miss;
                    }
                    assert!(close(got, expected, tolerance), "frame {}: {got:e} against {expected:e}", index + 1);
                }
                assert_eq!(state.measured_frames, want.measured_frames);
                snapshot += 1;
            }
        }
        assert_eq!(snapshot, SNAPSHOTS.len());
        assert_eq!(unmeasured, UNMEASURED_FRAMES);
        assert_eq!(SNAPSHOTS[SNAPSHOTS.len() - 1].frames, FIRST_REFUSED_FRAME);
        assert!(close(state.t_s, FIRST_REFUSED_FRAME as f64 * FRAME_S, 1.0e-12));
        println!("largest difference from the Python reference, relative: {worst:e}");
    }

    #[test]
    fn the_ledger_residual_stays_under_the_limit_over_the_reference_run() {
        let (card, mut state) = reference_start();
        let mut frame = blank_frame();
        let mut worst = 0.0_f64;
        for row in FRAMES.iter().take(FIRST_REFUSED_FRAME) {
            step(&card, &mut state, &mut frame, load_of(*row), FRAME_S).unwrap();
            if state.residual_conservation_error > worst {
                worst = state.residual_conservation_error;
            }
        }
        println!("largest ledger residual over {FIRST_REFUSED_FRAME} frames: {worst:e}");
        assert!(worst < 1.0e-14, "{worst:e}");
        assert!(worst < RESIDUAL_CONSERVATION_LIMIT / 100.0);
    }

    /// The card's own checks and commit, with a chosen method.
    fn step_method(
        card: &LosJwstFineGuide,
        state: &LosState,
        load: LosLoad,
        method: Method,
    ) -> Result<LosState, StepReject> {
        let mut trial = *state;
        let mut frame = blank_frame();
        let before = frame.to_bytes();
        let result = step_by(card, &mut trial, &mut frame, load, FRAME_S, method, 0.0);
        if result.is_err() {
            assert_eq!(state_bits(&trial), state_bits(state));
            assert_eq!(frame.to_bytes(), before);
        }
        result.map(|()| trial)
    }

    /// Past the old wall at 22,900 and 2,820,000 counts per second the card
    /// answers: a measured frame at any count rate above zero lands with
    /// [`FLAG_COUNT_RATE_BEYOND_COMPARED`], and the ledger closes there frame
    /// after frame. The boundary of the noise law is found where the y axis's
    /// variance passes the largest 64-bit number, and a wrong gain is named
    /// at every count rate.
    #[test]
    fn past_the_fitted_count_rates_the_frame_lands_flagged_and_the_ledger_closes() {
        let (card, settled, _) = after_frames(100);
        // Each count rate, 1,100 frames on from the settled filter: the reference
        // file's centroids, at the chosen count rate.
        for counts_per_s in [1.0e-200, 1.0e-6, 1.0, 100.0, 22_899.999, 2_820_000.5, 1.0e8, 1.0e12, 1.0e300, f64::MAX] {
            let mut state = settled;
            let mut frame = blank_frame();
            let mut largest = 0.0_f64;
            let mut index = 100;
            while index < 1_200 {
                let mut load = load_of(FRAMES[index]);
                if load.measured == 1.0 {
                    load.counts_per_s = counts_per_s;
                }
                step(&card, &mut state, &mut frame, load, FRAME_S)
                    .unwrap_or_else(|reject| ::core::panic!("{counts_per_s}: frame {index}: {reject:?}"));
                if load.measured == 1.0 {
                    assert_ne!(frame.flags & FLAG_COUNT_RATE_BEYOND_COMPARED, 0, "{counts_per_s}: frame {index}");
                } else {
                    assert_eq!(frame.flags & FLAG_COUNT_RATE_BEYOND_COMPARED, 0, "{counts_per_s}: frame {index}");
                }
                if state.residual_conservation_error > largest {
                    largest = state.residual_conservation_error;
                }
                index += 1;
            }
            assert!(largest <= RESIDUAL_CONSERVATION_LIMIT, "{counts_per_s}: {largest}");
            // At the faint end the centroid is all but ignored; at the bright end
            // the estimate follows the centroid.
            let load = load_of(FRAMES[1_200]);
            assert_eq!(load.measured, 1.0);
            let load = LosLoad { counts_per_s, ..load };
            for gain_scale in [1.05, 0.95] {
                let method = Method { gain_scale, ..OWN_METHOD };
                if counts_per_s >= 1.0 {
                    assert_eq!(
                        step_method(&card, &state, load, method),
                        Err(StepReject::SpontaneousGeneration),
                        "{counts_per_s}: gain x {gain_scale}"
                    );
                }
            }
            let short = Method { gain_scale: 1.05, short_form: true, ..OWN_METHOD };
            if counts_per_s >= 1.0 {
                assert_eq!(step_method(&card, &state, load, short), Err(StepReject::FirstLawBreach), "{counts_per_s}");
            }
        }
        // The edge of the noise law: the last count rate at which the y axis's
        // variance is a finite number lands, the next one down is refused.
        // Bisection on the bits of positive numbers, which are in the order of the numbers.
        let (mut low, mut high) = (1.0e-300_f64.to_bits(), 1.0e-200_f64.to_bits());
        assert!(!card.y.noise_variance_mas2(f64::from_bits(low)).is_finite());
        assert!(card.y.noise_variance_mas2(f64::from_bits(high)).is_finite());
        while high - low > 1 {
            let middle = low + (high - low) / 2;
            if card.y.noise_variance_mas2(f64::from_bits(middle)).is_finite() {
                high = middle;
            } else {
                low = middle;
            }
        }
        let finite = f64::from_bits(high);
        assert!(finite > 1.0e-237 && finite < 1.0e-236, "{finite:e}");
        assert!(card.x.noise_variance_mas2(finite).is_finite());
        let below = f64::from_bits(finite.to_bits() - 1);
        let good = load_of(FRAMES[100]);
        refused(&card, &settled, LosLoad { counts_per_s: below, ..good }, FRAME_S, StepReject::CountRateNotPositive);
        let mut trial = settled;
        let mut frame = blank_frame();
        step(&card, &mut trial, &mut frame, LosLoad { counts_per_s: finite, ..good }, FRAME_S).unwrap();
        assert_ne!(frame.flags & FLAG_COUNT_RATE_BEYOND_COMPARED, 0);
    }

    #[test]
    fn a_gain_that_is_not_the_best_is_refused_and_the_cards_own_step_lands() {
        // From the start and from a settled filter.
        for frames in [0, 100] {
            let (card, state, _) = after_frames(frames);
            let load = load_of(FRAMES[frames]);
            assert!(step_method(&card, &state, load, OWN_METHOD).is_ok());
            for gain_scale in [1.05, 0.95, 1.000_1, 0.999_9, 0.0, 2.0] {
                let method = Method { gain_scale, ..OWN_METHOD };
                assert_eq!(
                    step_method(&card, &state, load, method),
                    Err(StepReject::SpontaneousGeneration),
                    "gain x {gain_scale} after {frames} frames"
                );
            }
            // The defect goes as the square of the gain's error: on a settled filter one
            // part in 100,000 is under the limit and lands.
            if frames == 100 {
                let near = Method { gain_scale: 1.000_01, ..OWN_METHOD };
                assert!(step_method(&card, &state, load, near).is_ok());
            }
        }
    }

    #[test]
    fn the_short_update_with_a_gain_too_large_is_refused_as_claimed_knowledge() {
        for frames in [0, 100] {
            let (card, state, _) = after_frames(frames);
            let load = load_of(FRAMES[frames]);
            // With the best gain the short form is the long form to rounding, and it lands.
            let short = Method { short_form: true, ..OWN_METHOD };
            let landed = step_method(&card, &state, load, short).unwrap();
            let own = step_method(&card, &state, load, OWN_METHOD).unwrap();
            for entry in 0..10 {
                assert!(close(landed.x.cov_mas2[entry], own.x.cov_mas2[entry], 1.0e-12));
            }
            // A gain too large in the short form leaves less variance than the measurement gave.
            let greedy = Method { gain_scale: 1.05, short_form: true, ..OWN_METHOD };
            assert_eq!(step_method(&card, &state, load, greedy), Err(StepReject::FirstLawBreach));
            // A gain too small leaves more.
            let timid = Method { gain_scale: 0.95, short_form: true, ..OWN_METHOD };
            assert_eq!(step_method(&card, &state, load, timid), Err(StepReject::SpontaneousGeneration));
        }
    }

    #[test]
    fn a_drive_left_out_or_doubled_is_refused() {
        let (card, state, _) = after_frames(100);
        for load in [load_of(FRAMES[100]), NO_MEASUREMENT] {
            let none = Method { drive_scale: 0.0, ..OWN_METHOD };
            assert_eq!(step_method(&card, &state, load, none), Err(StepReject::FirstLawBreach));
            let twice = Method { drive_scale: 2.0, ..OWN_METHOD };
            assert_eq!(step_method(&card, &state, load, twice), Err(StepReject::SpontaneousGeneration));
        }
    }

    #[test]
    fn a_biased_ledger_is_refused_both_ways_with_state_and_frame_bit_identical() {
        let (card, state, _) = after_frames(100);
        let load = load_of(FRAMES[100]);
        let scale = card.ledger_scale_mas2();
        for (bias, expected) in [
            (10.0 * RESIDUAL_CONSERVATION_LIMIT * scale, StepReject::SpontaneousGeneration),
            (-10.0 * RESIDUAL_CONSERVATION_LIMIT * scale, StepReject::FirstLawBreach),
            (1.0e-3, StepReject::SpontaneousGeneration),
            (-1.0e-3, StepReject::FirstLawBreach),
        ] {
            let mut trial = state;
            let mut frame = blank_frame();
            let before = frame.to_bytes();
            assert_eq!(
                step_by(&card, &mut trial, &mut frame, load, FRAME_S, OWN_METHOD, bias),
                Err(expected)
            );
            assert_eq!(state_bits(&trial), state_bits(&state));
            assert_eq!(frame.to_bytes(), before);
        }
        // A bias a tenth of the limit lands.
        let mut trial = state;
        let mut frame = blank_frame();
        let small = 0.1 * RESIDUAL_CONSERVATION_LIMIT * scale;
        assert!(step_by(&card, &mut trial, &mut frame, load, FRAME_S, OWN_METHOD, small).is_ok());
    }

    #[test]
    fn every_refusal_is_named_and_leaves_state_and_frame_bit_identical() {
        let (card, state, _) = after_frames(100);
        let good = load_of(FRAMES[100]);

        refused(&card, &state, good, f64::NAN, StepReject::NonFiniteInput);
        refused(&card, &state, good, f64::INFINITY, StepReject::NonFiniteInput);
        for field in 0..4 {
            for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let mut load = good;
                match field {
                    0 => load.measured = bad,
                    1 => load.centroid_x_arcsec = bad,
                    2 => load.centroid_y_arcsec = bad,
                    _ => load.counts_per_s = bad,
                }
                refused(&card, &state, load, FRAME_S, StepReject::NonFiniteInput);
            }
        }

        let mut broken = state;
        broken.x.cov_mas2[3] = f64::NAN;
        refused(&card, &broken, good, FRAME_S, StepReject::NonFiniteState);
        let mut broken = state;
        broken.y.mode_sin_mas = f64::INFINITY;
        refused(&card, &broken, good, FRAME_S, StepReject::NonFiniteState);
        let mut broken = state;
        broken.nis_sum_x = f64::NAN;
        refused(&card, &broken, good, FRAME_S, StepReject::NonFiniteState);

        refused(&card, &state, good, 0.0, StepReject::NonPositiveDt);
        refused(&card, &state, good, -FRAME_S, StepReject::NonPositiveDt);

        let past = f64::from_bits(LOCK_FIELD_HALF_WIDTH_ARCSEC.to_bits() + 1);
        for (x, y) in [(past, 0.0), (-past, 0.0), (0.0, past), (0.0, -71.0), (100.0, 100.0)] {
            let mut moved = state;
            moved.lock_x_arcsec = x;
            moved.lock_y_arcsec = y;
            refused(&card, &moved, good, FRAME_S, StepReject::LockOutsideSensorField);
            refused(&card, &moved, NO_MEASUREMENT, FRAME_S, StepReject::LockOutsideSensorField);
        }

        for entry in [0, 4, 7, 9] {
            let mut broken = state;
            broken.x.cov_mas2[entry] = 0.0;
            refused(&card, &broken, good, FRAME_S, StepReject::InvalidCovariance);
            let mut broken = state;
            broken.y.cov_mas2[entry] = -1.0e-6;
            refused(&card, &broken, good, FRAME_S, StepReject::InvalidCovariance);
        }

        for measured in [0.5, 2.0, -1.0, 1.000_000_1] {
            let load = LosLoad { measured, ..good };
            refused(&card, &state, load, FRAME_S, StepReject::MeasuredNotZeroOrOne);
        }

        refused(&card, &state, LosLoad { measured: 0.0, ..good }, FRAME_S, StepReject::UnmeasuredFrameCarriesNumbers);
        refused(
            &card,
            &state,
            LosLoad { counts_per_s: 1.0, ..NO_MEASUREMENT },
            FRAME_S,
            StepReject::UnmeasuredFrameCarriesNumbers,
        );
        refused(
            &card,
            &state,
            LosLoad { centroid_y_arcsec: -1.0e-9, ..NO_MEASUREMENT },
            FRAME_S,
            StepReject::UnmeasuredFrameCarriesNumbers,
        );

        // No light, or so little that the noise law passes the largest 64-bit number.
        for counts_per_s in [0.0, -0.0, -5.0e5, -1.0e-300, 5.0e-324, 1.0e-300, 1.0e-240, 7.0e-237] {
            let load = LosLoad { counts_per_s, ..good };
            refused(&card, &state, load, FRAME_S, StepReject::CountRateNotPositive);
        }
        // Inside the fitted count rates, both ends included, the frame carries no flag of its own.
        for counts_per_s in [COUNTS_MIN_PER_S, COUNTS_MAX_PER_S] {
            let mut trial = state;
            let mut frame = blank_frame();
            let load = LosLoad { counts_per_s, ..good };
            assert!(step(&card, &mut trial, &mut frame, load, FRAME_S).is_ok());
            assert_eq!(frame.flags & FLAG_COUNT_RATE_BEYOND_COMPARED, 0);
        }

        // The offset limit, from a lock point at the centre so that the numbers are exact.
        let centre = LosState::try_lock(&card, 0.0, 0.0, RECOVERY_S).unwrap();
        let at = |x_arcsec: f64, y_arcsec: f64| LosLoad {
            measured: 1.0,
            centroid_x_arcsec: x_arcsec,
            centroid_y_arcsec: y_arcsec,
            counts_per_s: 551_754.4,
        };
        refused(&card, &centre, at(0.2501, 0.0), FRAME_S, StepReject::OffsetOutsideCard);
        refused(&card, &centre, at(0.0, -0.2501), FRAME_S, StepReject::OffsetOutsideCard);
        refused(&card, &centre, at(-3.0, 0.1), FRAME_S, StepReject::OffsetOutsideCard);
        // 250 mas exactly is on the card and is a disturbance; so is everything
        // from just over 5 mas, on either axis. Each lands with its flag and
        // the ledger closed.
        for load in [at(0.25, -0.25), at(0.005_001, 0.0), at(0.0, -0.005_001), at(-0.02, 0.03)] {
            let mut trial = centre;
            let mut frame = blank_frame();
            step(&card, &mut trial, &mut frame, load, FRAME_S).unwrap();
            assert!(frame.flags & FLAG_POINTING_DISTURBED != 0);
            assert!(trial.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            assert_eq!(trial.since_disturbance_s, 0.0);
        }

        // A start the caller states is soon after a disturbance: it lands and
        // its frames say they are recovering. Below zero is refused by name.
        let mut soon = state;
        for quiet in [29.9, 0.0] {
            soon.quiet_before_start_s = quiet;
            soon.since_disturbance_s = quiet;
            let mut trial = soon;
            let mut frame = blank_frame();
            step(&card, &mut trial, &mut frame, good, FRAME_S).unwrap();
            assert!(frame.flags & FLAG_RECOVERING != 0, "{quiet}");
        }
        soon.quiet_before_start_s = -30.0;
        soon.since_disturbance_s = -30.0;
        refused(&card, &soon, NO_MEASUREMENT, FRAME_S, StepReject::QuietBeforeStartBelowZero);
        soon.quiet_before_start_s = f64::NAN;
        refused(&card, &soon, good, FRAME_S, StepReject::NonFiniteState);

        // A centroid more than four stated standard deviations from where it
        // was expected lands with its flag.
        for jump in [
            frame_at(&state, state.estimate_x_mas + 3.0, state.estimate_y_mas),
            frame_at(&state, state.estimate_x_mas, state.estimate_y_mas - 3.0),
        ] {
            let mut trial = state;
            let mut frame = blank_frame();
            step(&card, &mut trial, &mut frame, jump, FRAME_S).unwrap();
            assert!(frame.flags & FLAG_BEYOND_FOUR_SIGMA != 0);
            assert_eq!(frame.flags & FLAG_RECOVERING, 0);
        }
    }

    #[test]
    fn a_lock_point_past_the_compared_span_is_a_start_and_past_the_sensor_field_is_not() {
        let card = LosJwstFineGuide::observatory();
        // The four corners of the compared span are starts, and a step from each lands.
        for (x, y) in [
            (LOCK_X_MIN_ARCSEC, LOCK_Y_MIN_ARCSEC),
            (LOCK_X_MIN_ARCSEC, LOCK_Y_MAX_ARCSEC),
            (LOCK_X_MAX_ARCSEC, LOCK_Y_MIN_ARCSEC),
            (LOCK_X_MAX_ARCSEC, LOCK_Y_MAX_ARCSEC),
        ] {
            let mut start = LosState::try_lock(&card, x, y, RECOVERY_S).unwrap();
            let mut frame = blank_frame();
            let load = frame_at(&start, 0.3, -0.2);
            step(&card, &mut start, &mut frame, load, FRAME_S).unwrap();
        }
        // Where the card stood at the compared span, the next 64-bit number
        // outside each end and the sensor's corners are starts, and a hundred
        // frames from each land with the ledger closed.
        let out = |v: f64| f64::from_bits(v.to_bits() + 1);
        for (x, y) in [
            (out(LOCK_X_MAX_ARCSEC), 0.0),
            (out(LOCK_X_MIN_ARCSEC), 0.0),
            (0.0, out(LOCK_Y_MAX_ARCSEC)),
            (0.0, out(LOCK_Y_MIN_ARCSEC)),
            (70.0, 70.0),
            (70.0, -70.0),
            (-70.0, -70.0),
        ] {
            let mut start = LosState::try_lock(&card, x, y, RECOVERY_S).unwrap();
            let mut frame = blank_frame();
            for _ in 0..100 {
                let load = frame_at(&start, start.predicted_x_mas + 0.3, start.predicted_y_mas - 0.2);
                step(&card, &mut start, &mut frame, load, FRAME_S).unwrap();
                assert!(start.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            }
        }
        // Past the sensor's field, no start.
        assert!(LosState::try_lock(&card, 0.0, -70.000_001, RECOVERY_S).is_none());
        assert!(LosState::try_lock(&card, 70.000_001, 0.0, RECOVERY_S).is_none());
        assert!(LosState::try_lock(&card, 120.0, 0.0, RECOVERY_S).is_none());
        assert!(LosState::try_lock(&card, f64::NAN, 0.0, RECOVERY_S).is_none());
        assert!(LosState::try_lock(&card, 0.0, f64::INFINITY, RECOVERY_S).is_none());
        let mut moved = card;
        moved.x.slow_mas = 2.0;
        assert!(LosState::try_lock(&moved, 0.0, 0.0, RECOVERY_S).is_none());
        let start = LosState::try_lock(&card, 17.0, -15.0, RECOVERY_S).unwrap();
        assert_eq!(start.quiet_before_start_s, 30.0);
        assert_eq!(start.t_s, 0.0);
        assert_eq!(start.x.cov_mas2[0], X_AXIS.slow_mas * X_AXIS.slow_mas);
        assert_eq!(start.y.cov_mas2[9], Y_AXIS.mode_mas * Y_AXIS.mode_mas);
        assert_eq!(start.unmeasured_in_row, 0.0);
        assert!(close(start.variance_x_mas2 + start.variance_y_mas2, card.ledger_scale_mas2(), 1.0e-15));
    }

    #[test]
    fn with_nothing_measured_the_filter_knows_nothing_and_stays_there() {
        let (card, mut state) = reference_start();
        let start = state;
        let mut frame = blank_frame();
        for _ in 0..5000 {
            predict_past_the_card(&card, &mut state, &mut frame);
            assert_eq!(frame.flags, FLAG_NO_MEASUREMENT);
            assert!(state.no_measurement);
        }
        // Every part stays at zero, and the covariance stays at its unmeasured value.
        assert_eq!(state.estimate_x_mas, 0.0);
        assert_eq!(state.estimate_y_mas, 0.0);
        assert_eq!(state.predicted_x_mas, 0.0);
        assert_eq!(state.innovation_variance_x_mas2, 0.0);
        assert_eq!(state.nis, 0.0);
        assert_eq!(state.measured_frames, 0.0);
        for entry in [0, 4, 7, 9] {
            assert!(close(state.x.cov_mas2[entry], start.x.cov_mas2[entry], 1.0e-12));
            assert!(close(state.y.cov_mas2[entry], start.y.cov_mas2[entry], 1.0e-12));
        }
        assert!(close(state.t_s, 320.0, 1.0e-12));
    }

    #[test]
    fn a_measurement_takes_variance_and_frames_without_one_give_it_back() {
        let (card, mut state, _) = after_frames(100);
        let unmeasured = card.ledger_scale_mas2();
        let settled = state.variance_x_mas2 + state.variance_y_mas2;
        assert!(settled < 0.5 * unmeasured, "{settled} against {unmeasured}");
        // The stated variance of the estimate is under the noise of one centroid.
        let noise = X_AXIS.noise_variance_mas2(FRAMES[99].3);
        assert!(state.estimate_variance_x_mas2 < noise);
        assert!(state.estimate_variance_x_mas2 > 0.0);
        let mut frame = blank_frame();
        let mut last = settled;
        let estimate = state.estimate_y_mas.abs();
        for _ in 0..20_000 {
            predict_past_the_card(&card, &mut state, &mut frame);
            let now = state.variance_x_mas2 + state.variance_y_mas2;
            assert!(now >= last * (1.0 - 1.0e-14), "the variance fell with nothing measured");
            last = now;
        }
        // 1,280 s is nineteen memory times of the slowest part.
        assert!(close(last, unmeasured, 1.0e-9), "{last} against {unmeasured}");
        assert!(state.estimate_y_mas.abs() < 1.0e-6 * (estimate + 1.0));
    }

    #[test]
    fn the_mode_turns_at_its_frequency_and_fades_with_its_memory() {
        let card = LosJwstFineGuide::observatory();
        let mut state = LosState::try_lock(&card, 0.0, 0.0, RECOVERY_S).unwrap();
        state.x.mode_cos_mas = 1.0;
        state.y.mode_sin_mas = 1.0;
        let mut frame = blank_frame();
        let frames = 250;
        for _ in 0..frames {
            predict_past_the_card(&card, &mut state, &mut frame);
        }
        let t = f64::from(frames) * FRAME_S;
        let two_pi = 2.0 * core::f64::consts::PI;
        let fade_x = (-t / X_AXIS.mode_memory_s).exp();
        let fade_y = (-t / Y_AXIS.mode_memory_s).exp();
        let turn_x = two_pi * X_AXIS.mode_hz * t;
        let turn_y = two_pi * Y_AXIS.mode_hz * t;
        assert!(close(state.x.mode_cos_mas, fade_x * turn_x.cos(), 1.0e-11));
        assert!(close(state.x.mode_sin_mas, -fade_x * turn_x.sin(), 1.0e-11));
        assert!(close(state.y.mode_cos_mas, fade_y * turn_y.sin(), 1.0e-11));
        assert!(close(state.y.mode_sin_mas, fade_y * turn_y.cos(), 1.0e-11));
        assert!(close(state.estimate_x_mas, fade_x * turn_x.cos(), 1.0e-11));
    }

    #[test]
    fn a_start_soon_after_the_last_disturbance_the_caller_states_is_flagged_recovering() {
        let card = LosJwstFineGuide::observatory();
        // 30 s exactly, 469 frames of 0.064 s, and any longer time: a start.
        for quiet in [30.0, 469.0 * 0.064, 30.016, 60.0, 3600.0, 1.0e9] {
            let start = LosState::try_lock(&card, 0.0, 0.0, quiet).unwrap();
            assert_eq!(start.quiet_before_start_s.to_bits(), f64::to_bits(quiet));
            let mut trial = start;
            let mut frame = blank_frame();
            assert!(step(&card, &mut trial, &mut frame, NO_MEASUREMENT, FRAME_S).is_ok());
            assert_eq!(trial.quiet_before_start_s.to_bits(), f64::to_bits(quiet));
        }
        // 468 frames are 29.952 s, and zero: a start whose first frames are
        // recovering. Below zero, or not finite: no start.
        for quiet in [468.0 * 0.064, 29.9, 29.999_999, 1.0, 0.0] {
            let start = LosState::try_lock(&card, 0.0, 0.0, quiet).unwrap();
            let mut trial = start;
            let mut frame = blank_frame();
            step(&card, &mut trial, &mut frame, NO_MEASUREMENT, FRAME_S).unwrap();
            assert_eq!(frame.flags & FLAG_RECOVERING != 0, quiet + FRAME_S < 30.0, "{quiet}");
        }
        for quiet in [-30.0, -1.0e-300, f64::NAN, f64::INFINITY] {
            assert!(LosState::try_lock(&card, 0.0, 0.0, quiet).is_none(), "{quiet}");
        }
        // Thirty seconds as the difference of two times of a table, each across a
        // power of two, where the two times are held to different spacings. Each
        // is 30 s exactly on paper and reads a little under or over it in 64-bit
        // arithmetic. All are taken.
        let computed = [
            1030.1 - 1000.1,
            2060.3 - 2030.3,
            4110.7 - 4080.7,
            8200.1 - 8170.1,
            16_390.9 - 16_360.9,
            32_780.3 - 32_750.3,
            65_550.7 - 65_520.7,
            520.1 - 490.1,
            270.3 - 240.3,
            140.7 - 110.7,
            70.1 - 40.1,
            40.3 - 10.3,
        ];
        let mut under = 0;
        for quiet in computed {
            assert!((quiet - 30.0_f64).abs() <= RECOVERY_ALLOWANCE_S, "{quiet}");
            if quiet < 30.0 {
                under += 1;
            }
            assert!(card.recovery_has_elapsed(quiet), "{quiet}");
            assert!(LosState::try_lock(&card, 0.0, 0.0, quiet).is_some(), "{quiet}");
        }
        assert!(under >= 3, "{under} of the twelve read under 30 s");
        // The allowance itself is past the 30 s; the allowance and a little
        // more is still recovering.
        let at_the_allowance = RECOVERY_S - RECOVERY_ALLOWANCE_S;
        assert!(card.recovery_has_elapsed(at_the_allowance));
        assert!(!LosState::try_lock(&card, 0.0, 0.0, at_the_allowance).unwrap().recovering);
        let beyond = RECOVERY_S - 1.5 * RECOVERY_ALLOWANCE_S;
        assert!(!card.recovery_has_elapsed(beyond));
        assert!(LosState::try_lock(&card, 0.0, 0.0, beyond).unwrap().recovering);
    }

    /// Where engines 0.7 to 0.9 ended a run at its last quiet frame, the card
    /// answers a disturbance: the frame lands with its flag and the ledger
    /// closed, the clock since the disturbance starts again, and the frames of
    /// the next 30 s carry FLAG_RECOVERING. The reference run goes on through
    /// its first frame beyond four sigma and its first above 5 mas.
    #[test]
    fn a_disturbance_is_answered_with_its_flags_and_the_ledger_closed() {
        let (card, state, _) = after_frames(100);

        // A centroid where it is expected sets nothing.
        let mut trial = state;
        let mut frame = blank_frame();
        let here = frame_at(&state, state.estimate_x_mas, state.estimate_y_mas);
        step(&card, &mut trial, &mut frame, here, FRAME_S).unwrap();
        assert_eq!(frame.flags, 0);
        assert!(!trial.no_measurement);

        // 5 mas exactly is quiet; the next 64-bit offset above it is a disturbance,
        // on each axis and each side.
        let centre = LosState::try_lock(&card, 0.0, 0.0, RECOVERY_S).unwrap();
        let at = |x_arcsec: f64, y_arcsec: f64| LosLoad {
            measured: 1.0,
            centroid_x_arcsec: x_arcsec,
            centroid_y_arcsec: y_arcsec,
            counts_per_s: 30_000.0,
        };
        assert_eq!((0.005_f64 - 0.0) * 1000.0, QUIET_OFFSET_MAS);
        for (x, y) in [(0.005, 0.0), (-0.005, 0.0)] {
            let mut trial = centre;
            step(&card, &mut trial, &mut frame, at(x, y), FRAME_S).unwrap();
            assert_eq!(frame.flags, 0);
        }
        let over = f64::from_bits(0.005_f64.to_bits() + 1);
        assert!(over * 1000.0 > QUIET_OFFSET_MAS);
        for (x, y) in [(over, 0.0), (-over, 0.0), (0.0, over), (0.0, -over), (0.005_001, 0.0)] {
            let mut trial = centre;
            step(&card, &mut trial, &mut frame, at(x, y), FRAME_S).unwrap();
            assert!(frame.flags & FLAG_POINTING_DISTURBED != 0, "{x} {y}");
            assert!(trial.pointing_disturbed);
        }

        // Just inside four standard deviations lands with no flag; just
        // outside lands with FLAG_BEYOND_FOUR_SIGMA. The stated variance of the
        // next frame does not depend on its centroid.
        let mut probe = state;
        step(&card, &mut probe, &mut frame, here, FRAME_S).unwrap();
        let sigma = probe.innovation_variance_x_mas2.sqrt();
        let inside = frame_at(&state, probe.predicted_x_mas + 3.999 * sigma, probe.predicted_y_mas);
        let mut trial = state;
        step(&card, &mut trial, &mut frame, inside, FRAME_S).unwrap();
        assert_eq!(frame.flags, 0);
        let outside = frame_at(&state, probe.predicted_x_mas + 4.001 * sigma, probe.predicted_y_mas);
        let mut trial = state;
        step(&card, &mut trial, &mut frame, outside, FRAME_S).unwrap();
        assert_eq!(frame.flags, FLAG_BEYOND_FOUR_SIGMA);
        // The 30 s after it: 468 frames of 0.064 s are recovering, the 469th not.
        let mut count = 0;
        loop {
            let load = frame_at(&trial, trial.predicted_x_mas, trial.predicted_y_mas);
            step(&card, &mut trial, &mut frame, load, FRAME_S).unwrap();
            count += 1;
            if frame.flags & FLAG_RECOVERING == 0 {
                break;
            }
            assert!(count < 1000);
        }
        assert_eq!(count, 469);

        // The reference run: the frames where 0.10 ended it land with their
        // flags, and the ledger closes on every frame.
        let (card, mut run) = reference_start();
        let mut frame = blank_frame();
        for (index, row) in FRAMES.iter().enumerate().take(FIRST_FRAME_ABOVE_5_MAS + 1) {
            step(&card, &mut run, &mut frame, load_of(*row), FRAME_S).unwrap();
            assert!(run.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
            if index < FIRST_REFUSED_FRAME {
                assert_eq!(frame.flags & !FLAG_NO_MEASUREMENT, 0);
            }
            if index == FIRST_REFUSED_FRAME {
                assert!(frame.flags & FLAG_BEYOND_FOUR_SIGMA != 0);
            }
            if index > FIRST_REFUSED_FRAME && index < FIRST_FRAME_ABOVE_5_MAS && !run.beyond_four_sigma {
                assert!(frame.flags & FLAG_RECOVERING != 0, "{index}");
            }
        }
        assert!(frame.flags & FLAG_POINTING_DISTURBED != 0);
        proof_intact(&frame);
        // A frame with no measurement is no centroid and no disturbance.
        step(&card, &mut run, &mut frame, NO_MEASUREMENT, FRAME_S).unwrap();
        assert_eq!(frame.flags, FLAG_NO_MEASUREMENT | FLAG_RECOVERING);
    }

    /// Where the card stood at ten frames in a row with no measurement, any
    /// run of them is predicted: the stated variance grows toward its
    /// unmeasured value and no further, the ledger closes on every frame, and
    /// a measured frame afterwards sets the count to zero.
    #[test]
    fn a_run_of_any_number_of_frames_with_no_measurement_is_predicted() {
        let (card, settled, _) = after_frames(100);
        for start in [settled, LosState::try_lock(&card, 0.0, 0.0, RECOVERY_S).unwrap()] {
            let mut state = start;
            let mut frame = blank_frame();
            let mut last = state.variance_x_mas2 + state.variance_y_mas2;
            for count in 1..=2000 {
                step(&card, &mut state, &mut frame, NO_MEASUREMENT, FRAME_S).unwrap();
                assert_eq!(state.unmeasured_in_row, f64::from(count));
                assert_eq!(frame.flags, FLAG_NO_MEASUREMENT);
                assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
                let total = state.variance_x_mas2 + state.variance_y_mas2;
                assert!(total <= card.ledger_scale_mas2() * (1.0 + 1.0e-12), "{count}");
                assert!(total >= last * (1.0 - 1.0e-12), "{count}");
                last = total;
            }
            assert!(last > 0.95 * card.ledger_scale_mas2(), "{last}");
            let here = frame_at(&state, state.predicted_x_mas, state.predicted_y_mas);
            step(&card, &mut state, &mut frame, here, FRAME_S).unwrap();
            assert_eq!(state.unmeasured_in_row, 0.0);
            assert!(!state.no_measurement);
        }
        // A count that is not finite is a state the card did not write.
        let mut broken = settled;
        broken.unmeasured_in_row = f64::NAN;
        refused(&card, &broken, NO_MEASUREMENT, FRAME_S, StepReject::NonFiniteState);
        // The reference run, a fitted file's first frames, has no gap longer
        // than the comparison crossed.
        let (card, mut run) = reference_start();
        let mut frame = blank_frame();
        let mut longest = 0.0_f64;
        for row in FRAMES.iter().take(FIRST_REFUSED_FRAME) {
            step(&card, &mut run, &mut frame, load_of(*row), FRAME_S).unwrap();
            longest = longest.max(run.unmeasured_in_row);
        }
        assert!(longest <= MEASUREMENT_GAP_MAX_FRAMES, "{longest}");
    }

    #[test]
    fn the_sums_of_normalised_innovation_are_the_sums_of_the_frames() {
        let (card, mut state) = reference_start();
        let mut frame = blank_frame();
        let mut sum = 0.0;
        let mut measured = 0.0;
        for row in FRAMES.iter().take(500) {
            step(&card, &mut state, &mut frame, load_of(*row), FRAME_S).unwrap();
            sum += state.nis;
            if !state.no_measurement {
                measured += 1.0;
                let x = state.innovation_x_mas * state.innovation_x_mas / state.innovation_variance_x_mas2;
                let y = state.innovation_y_mas * state.innovation_y_mas / state.innovation_variance_y_mas2;
                assert!(close(state.nis, x + y, 1.0e-14));
            }
        }
        assert_eq!(state.measured_frames, measured);
        assert!(close(state.nis_sum_x + state.nis_sum_y, sum, 1.0e-12));
    }

    #[test]
    fn the_frame_carries_the_declared_slots_and_the_proof_is_not_the_cards() {
        let (_, state, frame) = after_frames(100);
        assert_eq!(frame.t, state.t_s);
        assert_eq!(frame.pos[0], state.predicted_x_mas as f32);
        assert_eq!(frame.pos[1], state.predicted_y_mas as f32);
        assert_eq!(frame.pos[2], state.innovation_variance_x_mas2 as f32);
        assert_eq!(frame.vel[0], state.estimate_x_mas as f32);
        assert_eq!(frame.vel[1], state.estimate_y_mas as f32);
        assert_eq!(frame.vel[2], state.innovation_variance_y_mas2 as f32);
        assert_eq!(frame.force_torque, state.nis as f32);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        proof_intact(&frame);
        // A refused step does not touch it either.
        let (card, state, mut frame) = after_frames(3);
        let bad = LosLoad { counts_per_s: 0.0, ..load_of(FRAMES[0]) };
        let mut trial = state;
        assert_eq!(step(&card, &mut trial, &mut frame, bad, FRAME_S), Err(StepReject::CountRateNotPositive));
        proof_intact(&frame);
    }

    #[test]
    fn the_step_ceiling_and_floor_are_present_and_the_step_applies_neither() {
        let card = LosJwstFineGuide::observatory();
        assert_eq!(card.gate_dt_s(), Some(0.0645));
        assert_eq!(card.floor_dt_s(), Some(0.0635));
        assert!(card.floor_dt_s().unwrap() < FRAME_S && FRAME_S < card.gate_dt_s().unwrap());
        let mut broken = card;
        broken.gate_dt_s = f64::NAN;
        assert_eq!(broken.gate_dt_s(), None);
        broken.floor_dt_s = -1.0;
        assert_eq!(broken.floor_dt_s(), None);
        // The public step takes a step outside them and does not move it to a limit.
        for dt_s in [1.0e-3, 0.5] {
            let (card, mut state, mut frame) = after_frames(10);
            let before = state.t_s;
            step(&card, &mut state, &mut frame, load_of(FRAMES[10]), dt_s).unwrap();
            assert_eq!(state.t_s, before + dt_s);
        }
    }

    #[test]
    fn the_noise_law_is_within_four_units_in_the_last_place_of_correctly_rounded_values() {
        // r_ref · (counts / 1e6)^(−r_power), correctly rounded, from the exact 64-bit
        // arguments with Python mpmath 1.3.0 at 60 digits.
        let x: [(f64, u64); 5] = [
            (22_900.0, 0x40064345f715e916),
            (100_000.0, 0x3fe053d54768cdea),
            (551_754.4, 0x3fb24c18129dfa75),
            (1.0e6, 0x3fa2758339dbc3e1),
            (2_820_000.0, 0x3f86648fc9bb1046),
        ];
        let y: [(f64, u64); 5] = [
            (22_900.0, 0x4008cdc604b4d609),
            (100_000.0, 0x3fde61fe57f3dd02),
            (551_754.4, 0x3faba1d977f088f0),
            (1.0e6, 0x3f99ec04a078b0ec),
            (2_820_000.0, 0x3f7bb3edfb363a55),
        ];
        let mut worst = 0_i64;
        for (axis, table) in [(X_AXIS, x), (Y_AXIS, y)] {
            for (counts_per_s, bits) in table {
                let got = axis.noise_variance_mas2(counts_per_s).to_bits() as i64;
                let off = (got - bits as i64).abs();
                if off > worst {
                    worst = off;
                }
                assert!(off <= 4, "{counts_per_s}: {off} units in the last place");
            }
        }
        println!("noise law: largest error {worst} units in the last place");
        // At the reference count rate the noise is the constant itself.
        assert_eq!(X_AXIS.noise_variance_mas2(1.0e6), X_AXIS.noise_reference_mas2);
        // Fainter is noisier.
        assert!(Y_AXIS.noise_variance_mas2(30_000.0) > Y_AXIS.noise_variance_mas2(300_000.0));
    }

    #[test]
    fn the_same_run_twice_is_the_same_bits() {
        let (_, a, fa) = after_frames(300);
        let (_, b, fb) = after_frames(300);
        assert_eq!(state_bits(&a), state_bits(&b));
        assert_eq!(fa.to_bytes(), fb.to_bytes());
    }
}
