#![no_implicit_prelude]
//! One human finger pad pressed on flat glass and twisted until it slips: the
//! right index finger of participant P02 of the torsion experiment at the
//! Université catholique de Louvain.
//!
//! The specimen and its data. du Bois de Dunilac, Córdova Bulens, Lefèvre,
//! Redmond and Delhaye, "Biomechanics of the finger pad in response to
//! torsion", J. R. Soc. Interface 20 (2023) 20220809, data on Zenodo, record
//! 7646119, both CC BY 4.0. The finger is held still with the nail up. A
//! robot presses a flat glass plate on the pad at a held normal force and
//! turns the plate about the normal through 80 degrees, at a held rate to
//! between 77 and 79.7 degrees and then slowing to a stop: 0.5,
//! 1, 2, 5 and 10 N at 20 degrees a second, and 5, 10, 20, 50 and 100 degrees
//! a second at 2 N, clockwise and counter-clockwise, five repeats. The stuck
//! fraction of the contact was filmed in a first experiment and the torque
//! measured in a second, on another day. The record in the research folder
//! is `evidence/research/tactile_slip_fingerpad_p02_glass/`.
//!
//! The eight fitted constants. [`SLIP_TORQUE_REFERENCE_N_M`],
//! [`LOAD_EXPONENT`], [`RATE_EXPONENT`], [`TORQUE_TWIST_REFERENCE_DEG`],
//! [`TORQUE_TWIST_LOAD_EXPONENT`], [`TORQUE_TWIST_RATE_EXPONENT`],
//! [`STUCK_TWIST_REFERENCE_DEG`] and [`STUCK_TWIST_RATE_EXPONENT`] are fitted
//! to this finger's own measurements: least squares in logarithms on 80
//! trials at 0.5, 2 and 5 N at 20 degrees a second and at 2 N at 5, 10 and 50
//! degrees a second. No constant here is taken from a handbook or derived
//! from skin or glass. The form was chosen on another finger of the same
//! experiment, P01.
//!
//! The comparison, and how the card did. Held back from the fitting: this
//! finger at 1 N, at 10 N and at 100 degrees a second. On 2026-10-09 the
//! slip torque missed its mark of 5.62 percent at all three: +9.4 percent at
//! 1 N, +29.3 percent at 10 N and −13.0 percent at 100 degrees a second
//! (10.15 against 11.66 N·mm). On the card's own fitted trials it is off by
//! +13.6 percent at 0.5 N and +21.7 percent at 5 N, and by +0.7, −6.8, −5.0
//! and −4.2 percent at 2 N and 5, 10, 20 and 50 degrees a second: 70 of the
//! 80 fitted trials are the 2 N block, the level of the law is that block's,
//! and that block stands 24 percent above the law this finger's four other
//! loads follow. The card answers at every positive force and every positive
//! rate with those misses stated. The forces measured on this finger were
//! 0.5 to 10 N and the rates 5 to 100 degrees a second; outside them, and
//! past 75 degrees of twist, a frame carries [`FLAG_BEYOND_COMPARED`]. The
//! record is in the engine's description of the card, in `RESULTS.md` of the
//! research folder and in `evidence/verification/second_party/V-40/`.
//!
//! What is left out. The direction of the twist: clockwise and
//! counter-clockwise trials are pooled. The state of the skin on the day,
//! moisture above all, which moves the friction of a finger from block to
//! block of trials by more than the trials of a block scatter. The small
//! overshoot of the torque before it settles, a few percent. The shrinking of
//! the contact area as the pad twists. The comparison took the torque and
//! the stuck fraction from 1 to 75 degrees, and the measured plate slowed to
//! a stop between 77 and 80 degrees; past 75 degrees the card goes on with
//! the same bed of springs under the held rate, the torque rising toward its
//! slip limit and the heat growing with the twist. The second of rest at 80
//! degrees and the release. Any motion but a twist about the normal: nothing
//! here is a finger sliding along the glass.
//!
//! One twist from zero under one load. The torque, the stuck fraction and the
//! stored energy below are closed forms of the twist for one load held from
//! the untwisted pad: the springs of the bed that have slid and those that
//! have not are where that one path puts them. Under a load changed part way
//! the springs hold a state set by the path, a function of the radius that
//! the card's state, the twist and the works, does not carry, and the closed
//! form would move the stored energy with no work done. So a step that
//! changes the force or the rate is [`StepReject::LoadChangedDuringTwist`],
//! and a start at any twist but zero, whose springs were set by a load and a
//! path a start does not carry, is [`StepReject::TwistNotStartedFromZero`].
//! Every trial began untwisted and held its load.
//!
//! The equations. A bed of independent springs under a friction limit that
//! falls as a parabola from the centre of a round contact to its edge,
//! twisted about the centre. With normal force `F`, twist rate `ω`, twist
//! `θ`, and `c` the stuck radius over the contact radius:
//!
//! ```text
//! T∞ = SLIP_TORQUE_REFERENCE · (F / 2 N)^LOAD_EXPONENT · (ω / 20 °/s)^RATE_EXPONENT
//! θT = TORQUE_TWIST_REFERENCE · (F / 2 N)^p · (ω / 20 °/s)^q
//! θS = STUCK_TWIST_REFERENCE · (ω / 20 °/s)^qs
//! c(u) = 2 / (u + sqrt(u² + 4))            the root of c·u = 1 − c²
//! torque         T = T∞ · [1 − (5c³ + 3c⁵)/8]      at u = θ / θT
//! stuck fraction   = c²                            at u = θ / θS
//! ```
//!
//! A bed of springs on a rigid finger has one characteristic twist. On the
//! measured fingers the torque and the stuck area did not share one, so the
//! card carries two, `θT` and `θS`, and the ledger is that of the bed that
//! carries the torque.
//!
//! The ledger, with `θ` in radians and everything at `u = θ / θT`:
//!
//! ```text
//! plate work     W += ∫ T dθ                              four-point Gauss rule over the step
//! friction heat  H += ∫ T∞ · [1 − (5c³ − 3c⁵)/2] dθ       the same rule
//! stored energy  E  = (15/4) · T∞ · θT · [c²(1 − c²)²/4 + (1 − c²)³/6]
//! defect         = (E − E at the start) + H − W
//! ```
//!
//! The work of the plate is the energy stored in the springs plus the heat of
//! the ring that slides. The defect is held to
//! [`RESIDUAL_CONSERVATION_LIMIT`] of the ledger scale. The scale is the
//! larger of a fixed energy and the plate work of the run. The fixed energy
//! is the slip-torque limit of the law at [`LEDGER_SCALE_NORMAL_FORCE_N`]
//! and [`LEDGER_SCALE_TWIST_RATE_DEG_S`] over [`LEDGER_SCALE_TWIST_DEG`], the
//! whole turn of the plate, about 0.05 J: the largest the law gives over the
//! forces measured on this finger, the scale of the build that was scored.
//! A twist to 75 degrees at 2.07 N and 50 degrees a second does 0.0126 J of
//! plate work, so on every twist of the band at 2 N the fixed energy is the
//! scale and the residual of a frame reads as it did through engine 0.9. A
//! heavier load or a longer twist does more work than the fixed energy, and
//! the scale then follows the work, so the ledger holds the same relative
//! closure at every force and twist. A positive defect is energy from nowhere
//! and is [`StepReject::SpontaneousGeneration`]; a negative one is
//! [`StepReject::FirstLawBreach`]. The three terms are of one bed of springs,
//! so the ledger closes for any value of the eight constants: it catches a
//! wrong stepping rule or a corrupted step, and it cannot catch a wrong
//! constant. No heat and no stored energy was measured.
//!
//! Near zero twist `c` is close to one, and `1 − c` is formed without a
//! subtraction of near-equal numbers; the torque and the heat rate are then
//! polynomials in `1 − c`.
//!
//! Twist and rate are in degrees, the unit of the measurements.
//!
//! What stops the card. A normal force of zero or less is no contact: the
//! slip-torque limit and the characteristic twist of the torque are then
//! zero, the twist ratio has no value, and the step is
//! [`StepReject::NoContact`]; so is a force under the smallest normal 64-bit
//! number, whose powers round to zero. A twist rate of zero or less is
//! [`StepReject::TwistRateNotPositive`]: the plate does not turn forward, and
//! the characteristic twists, which go as the rate to the powers −0.341 and
//! −0.015, have no value at a rate of zero. They grow without limit as the
//! rate falls: at 0.001 degrees a second the characteristic twist of the
//! torque is 430 degrees. The four-point rule sums the plate work and the
//! heat over a step, and the ledger sees only their difference, so the card
//! holds the step itself: a step that turns the plate by more than
//! [`STEP_TWIST_OVER_TORQUE_TWIST`], 0.05, of the characteristic twist of the
//! torque is [`StepReject::TwistStepAboveQuadrature`]. At 2 N that is
//! 0.73 degrees at 20 degrees a second, 0.42 at 100 and 0.088 at 10,000, so
//! a step of 0.01 s holds to 52.6 degrees a second at 2 N, and a faster
//! twist takes a shorter step. Every twist of 1.93 to 2.07 N and 5 to 50
//! degrees a second that engine 0.9 answered is inside it: its largest step
//! is 0.047 of the characteristic twist.
//!
//! [`FingerPadP02Glass::gate_dt_s`] is the step ceiling, 0.01 s: one frame of
//! the camera at 100 degrees a second, the fastest twist measured.
//! [`FingerPadP02Glass::floor_dt_s`] is the ceiling divided by 2^8,
//! 3.90625e-5 s, a twenty-fifth of the sampling interval of the force
//! sensors. The public step applies neither. The gateway refuses a step
//! outside them.
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes time, the twist, the stuck
//! fraction, the slip-torque limit, the plate work, the stored energy, the
//! friction heat, the torque and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Slip-torque limit at 2 N and 20 degrees a second, N·m. Fitted to this
/// finger's opened trials (10.3527 N·mm).
pub const SLIP_TORQUE_REFERENCE_N_M: f64 = 0.0103527;
/// Power of the normal force in the slip-torque limit. Fitted.
pub const LOAD_EXPONENT: f64 = 0.77321;
/// Power of the twist rate in the slip-torque limit. Fitted.
pub const RATE_EXPONENT: f64 = -0.00944233;
/// Characteristic twist of the torque at 2 N and 20 degrees a second,
/// degrees. Fitted.
pub const TORQUE_TWIST_REFERENCE_DEG: f64 = 14.6438;
/// Power of the normal force in the characteristic twist of the torque. Fitted.
pub const TORQUE_TWIST_LOAD_EXPONENT: f64 = 0.110158;
/// Power of the twist rate in the characteristic twist of the torque. Fitted.
pub const TORQUE_TWIST_RATE_EXPONENT: f64 = -0.341316;
/// Characteristic twist of the stuck area at 20 degrees a second, degrees.
/// Fitted to the twists at which the measured stuck fraction fell below 0.03.
pub const STUCK_TWIST_REFERENCE_DEG: f64 = 7.05215;
/// Power of the twist rate in the characteristic twist of the stuck area. Fitted.
pub const STUCK_TWIST_RATE_EXPONENT: f64 = -0.0152421;
/// The normal force at which the reference values are stated, N.
pub const NORMAL_FORCE_REFERENCE_N: f64 = 2.0;
/// The twist rate at which the reference values are stated, degrees a second.
pub const TWIST_RATE_REFERENCE_DEG_S: f64 = 20.0;
/// Smallest normal force measured on this finger, N. Below it a frame
/// carries [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_FORCE_FLOOR_N: f64 = 0.5;
/// Largest normal force measured on this finger, N: held back, where the
/// slip torque was off by +29.3 percent. Above it a frame carries
/// [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_FORCE_CEILING_N: f64 = 10.0;
/// Slowest twist rate measured, degrees a second. Below it a frame carries
/// [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_RATE_FLOOR_DEG_S: f64 = 5.0;
/// Fastest twist rate measured, degrees a second: held back, where the slip
/// torque was off by −13.0 percent. Above it a frame carries
/// [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_RATE_CEILING_DEG_S: f64 = 100.0;
/// Largest share of the characteristic twist of the torque one step may turn
/// the plate through: 0.05, where the four-point rule that sums the plate
/// work and the heat is exact to rounding.
pub const STEP_TWIST_OVER_TORQUE_TWIST: f64 = 0.05;
/// The normal force of the ledger scale, N: the largest measured on this
/// finger. Not a force the card accepts.
pub const LEDGER_SCALE_NORMAL_FORCE_N: f64 = 10.0;
/// The twist rate of the ledger scale, degrees a second: the slowest
/// measured, where the law's slip-torque limit is largest.
pub const LEDGER_SCALE_TWIST_RATE_DEG_S: f64 = 5.0;
/// The end of the twist compared with measurement, degrees. The comparison
/// took the torque and the stuck fraction from 1 to 75 degrees and the slip
/// torque from the 0.5 s before 75 degrees. The plate turned 80 degrees in
/// every trial and slowed to a stop at 80: at 2 N it held its rate to 77.0
/// degrees at 50 degrees a second (trials 41 and 43: 51.5 degrees a second
/// to 77.0 degrees, 21 at 79.0, 0 at 80), and to 78.9, 79.4 and 79.7 degrees
/// at 20, 10 and 5. Past it a frame carries [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_TWIST_END_DEG: f64 = 75.0;
/// The twist of the ledger scale, degrees: the rotation of the plate in every
/// trial.
pub const LEDGER_SCALE_TWIST_DEG: f64 = 80.0;
/// How far past [`COMPARED_TWIST_END_DEG`] a twist may read and still be
/// within the compared twist, degrees. The twist is a sum of steps, each
/// rounded to half a unit in the last place of 75, 7.1e-15: a run of 384,000
/// steps that ends on 75 degrees in decimal reads at most 2.7e-9 past it if
/// every one rounded the same way. It moves only [`FLAG_BEYOND_COMPARED`].
pub const TWIST_END_ALLOWANCE_DEG: f64 = 4.0e-9;
/// Stuck fraction under which the contact is in full slip: the threshold of
/// the published measurement.
pub const FULL_SLIP_STUCK_FRACTION: f64 = 0.03;
/// Radians in one degree.
pub const RAD_PER_DEG: f64 = 0.017_453_292_519_943_295;
/// Step ceiling, s: one camera frame at 100 degrees a second, the fastest
/// twist measured.
pub const GATE_DT_S: f64 = 0.01;
/// Step ceiling divided by step floor: 2^8.
pub const GATE_OVER_FLOOR: f64 = 256.0;
/// `|(E − E_start) + H − W|` over the larger of `W_scale` and the plate work
/// must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;

/// The event the card exists to find, and not a warning: the model's stuck
/// fraction is under 0.03 on this frame, so the whole contact slides. The
/// model's torque is then 1.1 to 6.9 percent under its limit across the
/// card's range (2.4 percent at 2 N and 20 degrees a second) and goes on
/// rising toward it. The card puts the event at 39 to 40 degrees of twist;
/// on this finger at 2 N it was measured at 44 to 49 degrees.
pub const FLAG_FULL_SLIP: u64 = 1 << 0;
/// The frame is beyond what was compared with this finger: a normal force
/// below 0.5 N or above 10 N, a twist rate below 5 or above 100 degrees a
/// second, or a twist past 75 degrees by more than
/// [`TWIST_END_ALLOWANCE_DEG`]. The same bed of springs and the same
/// ledger; the fitted law is carried on past the measurements.
pub const FLAG_BEYOND_COMPARED: u64 = 1 << 1;

/// Why a step did not advance the finger pad or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The normal force, the twist rate or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this finger pad's.
    InvalidFingerPad,
    /// The normal force is zero or below, or under the smallest normal 64-bit number: the plate does not press on the pad, and the slip-torque limit and the characteristic twist of the torque are zero.
    NoContact,
    /// The twist rate is zero or below, or under the smallest normal 64-bit number: the plate does not turn forward, and the characteristic twists, powers of the rate, have no value.
    TwistRateNotPositive,
    /// The step turns the plate by more than 0.05 of the characteristic twist of the torque, past which the four-point rule that sums the plate work and the heat is not exact to rounding.
    TwistStepAboveQuadrature,
    /// The normal force or the twist rate differs from the one this twist began with: the torque, the stuck fraction and the stored energy are closed forms of the twist under one load held from the untwisted pad, and under a changed load the bed's springs hold a state set by the path, which the card's state does not carry; the stored energy would jump with no work done.
    LoadChangedDuringTwist,
    /// The twist at the start is not zero: the card's closed forms are of a pad twisted from zero under the load it holds, and a pad already twisted holds springs set by the load and the path it was brought there under, which a start does not carry.
    TwistNotStartedFromZero,
    /// The twist is below zero: the twist is of the plate turning forward from the untwisted pad.
    TwistBelowZero,
    /// The trial step created energy: the stored energy plus the friction heat rose above the plate work.
    SpontaneousGeneration,
    /// The trial step destroyed energy: the stored energy plus the friction heat fell below the plate work.
    FirstLawBreach,
}

/// What the robot holds over a step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FingerPadLoad {
    /// Normal force of the plate on the pad, `F`.
    pub normal_force_n: f64,
    /// Rate at which the plate turns about the normal, `ω`, degrees a second.
    pub twist_rate_deg_s: f64,
}

/// The locked finger pad. Every field is a recorded copy of a crate constant
/// and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FingerPadP02Glass {
    /// Recorded copy of [`SLIP_TORQUE_REFERENCE_N_M`].
    pub slip_torque_reference_n_m: f64,
    /// Recorded copy of [`LOAD_EXPONENT`].
    pub load_exponent: f64,
    /// Recorded copy of [`RATE_EXPONENT`].
    pub rate_exponent: f64,
    /// Recorded copy of [`TORQUE_TWIST_REFERENCE_DEG`].
    pub torque_twist_reference_deg: f64,
    /// Recorded copy of [`TORQUE_TWIST_LOAD_EXPONENT`].
    pub torque_twist_load_exponent: f64,
    /// Recorded copy of [`TORQUE_TWIST_RATE_EXPONENT`].
    pub torque_twist_rate_exponent: f64,
    /// Recorded copy of [`STUCK_TWIST_REFERENCE_DEG`].
    pub stuck_twist_reference_deg: f64,
    /// Recorded copy of [`STUCK_TWIST_RATE_EXPONENT`].
    pub stuck_twist_rate_exponent: f64,
    /// Recorded copy of [`NORMAL_FORCE_REFERENCE_N`].
    pub normal_force_reference_n: f64,
    /// Recorded copy of [`TWIST_RATE_REFERENCE_DEG_S`].
    pub twist_rate_reference_deg_s: f64,
    /// Recorded copy of [`COMPARED_FORCE_FLOOR_N`].
    pub compared_force_floor_n: f64,
    /// Recorded copy of [`COMPARED_FORCE_CEILING_N`].
    pub compared_force_ceiling_n: f64,
    /// Recorded copy of [`COMPARED_RATE_FLOOR_DEG_S`].
    pub compared_rate_floor_deg_s: f64,
    /// Recorded copy of [`COMPARED_RATE_CEILING_DEG_S`].
    pub compared_rate_ceiling_deg_s: f64,
    /// Recorded copy of [`STEP_TWIST_OVER_TORQUE_TWIST`].
    pub step_twist_over_torque_twist: f64,
    /// Recorded copy of [`LEDGER_SCALE_NORMAL_FORCE_N`].
    pub ledger_scale_normal_force_n: f64,
    /// Recorded copy of [`LEDGER_SCALE_TWIST_RATE_DEG_S`].
    pub ledger_scale_twist_rate_deg_s: f64,
    /// Recorded copy of [`COMPARED_TWIST_END_DEG`].
    pub compared_twist_end_deg: f64,
    /// Recorded copy of [`LEDGER_SCALE_TWIST_DEG`].
    pub ledger_scale_twist_deg: f64,
    /// Recorded copy of [`TWIST_END_ALLOWANCE_DEG`].
    pub twist_end_allowance_deg: f64,
    /// Recorded copy of [`FULL_SLIP_STUCK_FRACTION`].
    pub full_slip_stuck_fraction: f64,
    /// Recorded copy of [`RAD_PER_DEG`].
    pub rad_per_deg: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`GATE_OVER_FLOOR`].
    pub gate_over_floor: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
}

impl FingerPadP02Glass {
    /// Card: the finger pad of P02 on the glass plate. The constructor takes
    /// no arguments.
    #[must_use]
    pub fn on_glass() -> Self {
        let pad = Self {
            slip_torque_reference_n_m: SLIP_TORQUE_REFERENCE_N_M,
            load_exponent: LOAD_EXPONENT,
            rate_exponent: RATE_EXPONENT,
            torque_twist_reference_deg: TORQUE_TWIST_REFERENCE_DEG,
            torque_twist_load_exponent: TORQUE_TWIST_LOAD_EXPONENT,
            torque_twist_rate_exponent: TORQUE_TWIST_RATE_EXPONENT,
            stuck_twist_reference_deg: STUCK_TWIST_REFERENCE_DEG,
            stuck_twist_rate_exponent: STUCK_TWIST_RATE_EXPONENT,
            normal_force_reference_n: NORMAL_FORCE_REFERENCE_N,
            twist_rate_reference_deg_s: TWIST_RATE_REFERENCE_DEG_S,
            compared_force_floor_n: COMPARED_FORCE_FLOOR_N,
            compared_force_ceiling_n: COMPARED_FORCE_CEILING_N,
            compared_rate_floor_deg_s: COMPARED_RATE_FLOOR_DEG_S,
            compared_rate_ceiling_deg_s: COMPARED_RATE_CEILING_DEG_S,
            step_twist_over_torque_twist: STEP_TWIST_OVER_TORQUE_TWIST,
            ledger_scale_normal_force_n: LEDGER_SCALE_NORMAL_FORCE_N,
            ledger_scale_twist_rate_deg_s: LEDGER_SCALE_TWIST_RATE_DEG_S,
            compared_twist_end_deg: COMPARED_TWIST_END_DEG,
            ledger_scale_twist_deg: LEDGER_SCALE_TWIST_DEG,
            twist_end_allowance_deg: TWIST_END_ALLOWANCE_DEG,
            full_slip_stuck_fraction: FULL_SLIP_STUCK_FRACTION,
            rad_per_deg: RAD_PER_DEG,
            gate_dt_s: GATE_DT_S,
            gate_over_floor: GATE_OVER_FLOOR,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if pad.is_valid() {
            pad
        } else {
            ::core::panic!("finger pad failed is_valid");
        }
    }

    /// Every recorded copy equals its crate constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.slip_torque_reference_n_m.to_bits() == SLIP_TORQUE_REFERENCE_N_M.to_bits()
            && self.load_exponent.to_bits() == LOAD_EXPONENT.to_bits()
            && self.rate_exponent.to_bits() == RATE_EXPONENT.to_bits()
            && self.torque_twist_reference_deg.to_bits() == TORQUE_TWIST_REFERENCE_DEG.to_bits()
            && self.torque_twist_load_exponent.to_bits() == TORQUE_TWIST_LOAD_EXPONENT.to_bits()
            && self.torque_twist_rate_exponent.to_bits() == TORQUE_TWIST_RATE_EXPONENT.to_bits()
            && self.stuck_twist_reference_deg.to_bits() == STUCK_TWIST_REFERENCE_DEG.to_bits()
            && self.stuck_twist_rate_exponent.to_bits() == STUCK_TWIST_RATE_EXPONENT.to_bits()
            && self.normal_force_reference_n.to_bits() == NORMAL_FORCE_REFERENCE_N.to_bits()
            && self.twist_rate_reference_deg_s.to_bits() == TWIST_RATE_REFERENCE_DEG_S.to_bits()
            && self.compared_force_floor_n.to_bits() == COMPARED_FORCE_FLOOR_N.to_bits()
            && self.compared_force_ceiling_n.to_bits() == COMPARED_FORCE_CEILING_N.to_bits()
            && self.compared_rate_floor_deg_s.to_bits() == COMPARED_RATE_FLOOR_DEG_S.to_bits()
            && self.compared_rate_ceiling_deg_s.to_bits() == COMPARED_RATE_CEILING_DEG_S.to_bits()
            && self.step_twist_over_torque_twist.to_bits()
                == STEP_TWIST_OVER_TORQUE_TWIST.to_bits()
            && self.ledger_scale_normal_force_n.to_bits() == LEDGER_SCALE_NORMAL_FORCE_N.to_bits()
            && self.ledger_scale_twist_rate_deg_s.to_bits()
                == LEDGER_SCALE_TWIST_RATE_DEG_S.to_bits()
            && self.compared_twist_end_deg.to_bits() == COMPARED_TWIST_END_DEG.to_bits()
            && self.ledger_scale_twist_deg.to_bits() == LEDGER_SCALE_TWIST_DEG.to_bits()
            && self.twist_end_allowance_deg.to_bits() == TWIST_END_ALLOWANCE_DEG.to_bits()
            && self.full_slip_stuck_fraction.to_bits() == FULL_SLIP_STUCK_FRACTION.to_bits()
            && self.rad_per_deg.to_bits() == RAD_PER_DEG.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
    }

    /// `T∞`, the torque the pad carries in full slip at this force and rate,
    /// N·m. This and the four functions after it are the law as written and
    /// apply no range: [`step`] does.
    #[must_use]
    pub fn slip_torque_limit_n_m(&self, load: FingerPadLoad) -> f64 {
        self.slip_torque_reference_n_m
            * raised(load.normal_force_n / self.normal_force_reference_n, self.load_exponent)
            * raised(load.twist_rate_deg_s / self.twist_rate_reference_deg_s, self.rate_exponent)
    }

    /// `θT`, the characteristic twist of the torque at this force and rate, degrees.
    #[must_use]
    pub fn torque_twist_deg(&self, load: FingerPadLoad) -> f64 {
        self.torque_twist_reference_deg
            * raised(
                load.normal_force_n / self.normal_force_reference_n,
                self.torque_twist_load_exponent,
            )
            * raised(
                load.twist_rate_deg_s / self.twist_rate_reference_deg_s,
                self.torque_twist_rate_exponent,
            )
    }

    /// `θS`, the characteristic twist of the stuck area at this rate, degrees.
    #[must_use]
    pub fn stuck_twist_deg(&self, load: FingerPadLoad) -> f64 {
        self.stuck_twist_reference_deg
            * raised(
                load.twist_rate_deg_s / self.twist_rate_reference_deg_s,
                self.stuck_twist_rate_exponent,
            )
    }

    /// Torque of the pad on the plate at this twist, N·m.
    #[must_use]
    pub fn torque_n_m(&self, load: FingerPadLoad, twist_deg: f64) -> f64 {
        self.slip_torque_limit_n_m(load)
            * torque_over_limit(slipped_radius(twist_deg / self.torque_twist_deg(load)))
    }

    /// Stuck area over contact area at this twist.
    #[must_use]
    pub fn stuck_fraction(&self, load: FingerPadLoad, twist_deg: f64) -> f64 {
        let stuck_radius = 1.0 - slipped_radius(twist_deg / self.stuck_twist_deg(load));
        stuck_radius * stuck_radius
    }

    /// Energy stored in the springs at this twist, J.
    #[must_use]
    pub fn stored_energy_j(&self, load: FingerPadLoad, twist_deg: f64) -> f64 {
        let twist_scale_rad = self.torque_twist_deg(load) * self.rad_per_deg;
        3.75 * self.slip_torque_limit_n_m(load)
            * twist_scale_rad
            * stored_shape(slipped_radius(twist_deg / self.torque_twist_deg(load)))
    }

    /// `W_scale`: the slip-torque limit of the law at
    /// [`LEDGER_SCALE_NORMAL_FORCE_N`] and [`LEDGER_SCALE_TWIST_RATE_DEG_S`]
    /// over [`LEDGER_SCALE_TWIST_DEG`], J. About 0.05 J, the scale of the
    /// build that was scored.
    #[must_use]
    pub fn ledger_scale_j(&self) -> f64 {
        let largest = self.slip_torque_limit_n_m(FingerPadLoad {
            normal_force_n: self.ledger_scale_normal_force_n,
            twist_rate_deg_s: self.ledger_scale_twist_rate_deg_s,
        });
        largest * self.ledger_scale_twist_deg * self.rad_per_deg
    }

    /// The scale the ledger of a run is held to, J: the larger of
    /// [`Self::ledger_scale_j`] and the plate work of the run so far.
    #[must_use]
    pub fn run_ledger_scale_j(&self, plate_work_j: f64) -> f64 {
        let fixed = self.ledger_scale_j();
        if plate_work_j > fixed { plate_work_j } else { fixed }
    }

    /// The load and the twist of a frame are beyond what was compared with
    /// this finger: see [`FLAG_BEYOND_COMPARED`].
    #[must_use]
    pub fn beyond_compared(&self, load: FingerPadLoad, twist_deg: f64) -> bool {
        !(load.normal_force_n >= self.compared_force_floor_n
            && load.normal_force_n <= self.compared_force_ceiling_n
            && load.twist_rate_deg_s >= self.compared_rate_floor_deg_s
            && load.twist_rate_deg_s <= self.compared_rate_ceiling_deg_s
            && twist_deg <= self.compared_twist_end_deg + self.twist_end_allowance_deg)
    }

    /// Largest step the card stands behind: [`GATE_DT_S`], 0.01 s. Not
    /// applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.gate_dt_s.is_finite() && self.gate_dt_s > 0.0 {
            Some(self.gate_dt_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^8, 3.90625e-5 s. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate_dt_s) if self.gate_over_floor.is_finite() && self.gate_over_floor > 0.0 => {
                Some(gate_dt_s / self.gate_over_floor)
            }
            _ => None,
        }
    }
}

/// `size` raised to the power `exponent`, as `exp(exponent · log(size))` with
/// the crate's own exponential and logarithm. `size` is a magnitude. Zero,
/// and anything smaller than the smallest normal 64-bit number, gives exactly
/// zero and never a NaN.
fn raised(size: f64, exponent: f64) -> f64 {
    if size < f64::MIN_POSITIVE {
        return 0.0;
    }
    crate::math::exp(exponent * crate::math::log(size))
}

/// `w = 1 − c` at the twist ratio `u`, with `c = 2 / (u + sqrt(u² + 4))`:
/// the width of the sliding ring over the contact radius. Formed as
/// `(u + u² / (sqrt(u² + 4) + 2)) / (u + sqrt(u² + 4))`, which subtracts no
/// near-equal numbers. Zero at zero twist and below one always.
fn slipped_radius(twist_ratio: f64) -> f64 {
    let root = (twist_ratio * twist_ratio + 4.0).sqrt();
    (twist_ratio + twist_ratio * twist_ratio / (root + 2.0)) / (twist_ratio + root)
}

/// Torque over its limit, `1 − (5c³ + 3c⁵)/8`, as a polynomial in `w = 1 − c`.
fn torque_over_limit(w: f64) -> f64 {
    w * (30.0 + w * (-45.0 + w * (35.0 + w * (-15.0 + w * 3.0)))) / 8.0
}

/// Friction heat per radian over the torque limit, `1 − (5c³ − 3c⁵)/2`, as a
/// polynomial in `w = 1 − c`. Never below zero.
fn heat_rate_over_limit(w: f64) -> f64 {
    w * w * (15.0 + w * (-25.0 + w * (15.0 - w * 3.0))) / 2.0
}

/// `c²(1 − c²)²/4 + (1 − c²)³/6` in `w = 1 − c`, with `1 − c² = w(2 − w)`.
fn stored_shape(w: f64) -> f64 {
    let stuck_radius = 1.0 - w;
    let ring = w * (2.0 - w);
    stuck_radius * stuck_radius * ring * ring / 4.0 + ring * ring * ring / 6.0
}

/// Nodes of the four-point Gauss rule on [−1, 1].
const GAUSS_NODES: [f64; 4] = [
    -0.861_136_311_594_052_6,
    -0.339_981_043_584_856_3,
    0.339_981_043_584_856_3,
    0.861_136_311_594_052_6,
];
/// Weights of the four-point Gauss rule on [−1, 1].
const GAUSS_WEIGHTS: [f64; 4] = [
    0.347_854_845_137_453_85,
    0.652_145_154_862_546_1,
    0.652_145_154_862_546_1,
    0.347_854_845_137_453_85,
];

/// The twisted finger pad. The ledger fields are part of the state; write
/// them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FingerPadState {
    /// Time since the start of the run.
    pub t_s: f64,
    /// Twist of the plate against the finger since first contact, degrees.
    pub twist_deg: f64,
    /// Torque of the pad on the plate at the end of the last step. Zero
    /// before the first step.
    pub torque_n_m: f64,
    /// `T∞` of the held load. Zero before the first step.
    pub slip_torque_limit_n_m: f64,
    /// Stuck area over contact area at the end of the last step. One before
    /// the first step.
    pub stuck_fraction: f64,
    /// `W`, the work of the plate on the pad since the start of the run.
    pub plate_work_j: f64,
    /// `E − E_start`, the energy the springs have taken up since the start
    /// of the run.
    pub stored_energy_j: f64,
    /// `H`, the heat of the sliding ring since the start of the run. Never falls.
    pub friction_heat_j: f64,
    /// Signed `(E − E_start) + H − W`. Positive is created energy.
    pub energy_defect_j: f64,
    /// `|energy_defect_j|` over the larger of `W_scale` and the plate work.
    pub residual_conservation_error: f64,
    /// The normal force this twist began with. Zero before the first step.
    pub held_normal_force_n: f64,
    /// The twist rate this twist began with. Zero before the first step.
    pub held_twist_rate_deg_s: f64,
    /// `E_start`: the energy in the springs at the twist the run started
    /// from, under the held load. Zero before the first step.
    pub stored_energy_at_start_j: f64,
    /// The stuck fraction is under [`FULL_SLIP_STUCK_FRACTION`]: the event
    /// of full slip.
    pub full_slip: bool,
    /// The last step is beyond what was compared with this finger: see
    /// [`FLAG_BEYOND_COMPARED`].
    pub beyond_compared: bool,
}

impl FingerPadState {
    /// The untwisted pad at time 0, with the ledger empty and no load yet
    /// held. `twist_deg` must be zero: every measured twist began there, and
    /// a state does not say what load an earlier twist was made under. `None`
    /// for any other twist, however small, and for one that is not finite.
    /// The held load is carried in the state from the first step on, so the
    /// refusal of a changed load holds for as long as the state lives.
    #[must_use]
    pub fn try_start(pad: &FingerPadP02Glass, twist_deg: f64) -> Option<Self> {
        if !pad.is_valid() || !(twist_deg == 0.0) {
            return None;
        }
        Some(Self {
            t_s: 0.0,
            twist_deg: 0.0,
            torque_n_m: 0.0,
            slip_torque_limit_n_m: 0.0,
            stuck_fraction: 1.0,
            plate_work_j: 0.0,
            stored_energy_j: 0.0,
            friction_heat_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            held_normal_force_n: 0.0,
            held_twist_rate_deg_s: 0.0,
            stored_energy_at_start_j: 0.0,
            full_slip: false,
            beyond_compared: false,
        })
    }

    /// No step has been taken: no load is held yet.
    #[must_use]
    pub fn load_is_unset(&self) -> bool {
        self.held_normal_force_n == 0.0 && self.held_twist_rate_deg_s == 0.0
    }
}

/// What one step adds to the state, each term worked out on its own.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Advance {
    twist_deg: f64,
    plate_work_j: f64,
    friction_heat_j: f64,
    /// `E` at the end of the step, absolute.
    stored_energy_end_j: f64,
    /// `E` at the start of the step, absolute.
    stored_energy_begin_j: f64,
    torque_n_m: f64,
    slip_torque_limit_n_m: f64,
    stuck_fraction: f64,
}

/// Hold the load for one step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    pad: &FingerPadP02Glass,
    state: &mut FingerPadState,
    frame: &mut LastStateFrame64,
    load: FingerPadLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(pad, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("fingerpad_p02_glass", exceedances(pad, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    pad: &FingerPadP02Glass,
    state: &mut FingerPadState,
    frame: &mut LastStateFrame64,
    load: FingerPadLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(pad, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    pad: &FingerPadP02Glass,
    state: &mut FingerPadState,
    frame: &mut LastStateFrame64,
    load: FingerPadLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(pad, state, load, dt_s)?;
    let advance = held(pad, state.twist_deg, load, dt_s);
    commit(pad, state, frame, load, dt_s, advance)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut FingerPadState) {
    state.t_s = normal_or_zero(state.t_s);
    state.twist_deg = normal_or_zero(state.twist_deg);
    state.torque_n_m = normal_or_zero(state.torque_n_m);
    state.slip_torque_limit_n_m = normal_or_zero(state.slip_torque_limit_n_m);
    state.stuck_fraction = normal_or_zero(state.stuck_fraction);
    state.plate_work_j = normal_or_zero(state.plate_work_j);
    state.stored_energy_j = normal_or_zero(state.stored_energy_j);
    state.friction_heat_j = normal_or_zero(state.friction_heat_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.held_normal_force_n = normal_or_zero(state.held_normal_force_n);
    state.held_twist_rate_deg_s = normal_or_zero(state.held_twist_rate_deg_s);
    state.stored_energy_at_start_j = normal_or_zero(state.stored_energy_at_start_j);
}

fn validate(
    pad: &FingerPadP02Glass,
    state: &FingerPadState,
    load: FingerPadLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !(dt_s.is_finite() && load.normal_force_n.is_finite() && load.twist_rate_deg_s.is_finite())
    {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !pad.is_valid() {
        return Err(StepReject::InvalidFingerPad);
    }
    if !(load.normal_force_n >= f64::MIN_POSITIVE) {
        return Err(StepReject::NoContact);
    }
    if !(load.twist_rate_deg_s >= f64::MIN_POSITIVE) {
        return Err(StepReject::TwistRateNotPositive);
    }
    if state.load_is_unset() && !(state.twist_deg == 0.0) {
        return Err(StepReject::TwistNotStartedFromZero);
    }
    if !state.load_is_unset()
        && (state.held_normal_force_n.to_bits() != load.normal_force_n.to_bits()
            || state.held_twist_rate_deg_s.to_bits() != load.twist_rate_deg_s.to_bits())
    {
        return Err(StepReject::LoadChangedDuringTwist);
    }
    if !(state.twist_deg >= 0.0) {
        return Err(StepReject::TwistBelowZero);
    }
    if !(load.twist_rate_deg_s * dt_s
        <= pad.step_twist_over_torque_twist * pad.torque_twist_deg(load))
    {
        return Err(StepReject::TwistStepAboveQuadrature);
    }
    Ok(())
}

/// The increments of one step with the load held. The plate work and the
/// friction heat are each a four-point Gauss sum of their own integrand over
/// the step; the stored energy is the closed form at each end.
fn held(pad: &FingerPadP02Glass, twist_begin_deg: f64, load: FingerPadLoad, dt_s: f64) -> Advance {
    let limit_n_m = pad.slip_torque_limit_n_m(load);
    let torque_twist_deg = pad.torque_twist_deg(load);
    let step_deg = load.twist_rate_deg_s * dt_s;
    let twist_end_deg = twist_begin_deg + step_deg;
    let middle_deg = twist_begin_deg + 0.5 * step_deg;
    let half_deg = 0.5 * step_deg;
    let mut torque_sum = 0.0;
    let mut heat_sum = 0.0;
    let mut node = 0;
    while node < 4 {
        let w = slipped_radius((middle_deg + half_deg * GAUSS_NODES[node]) / torque_twist_deg);
        torque_sum += GAUSS_WEIGHTS[node] * torque_over_limit(w);
        heat_sum += GAUSS_WEIGHTS[node] * heat_rate_over_limit(w);
        node += 1;
    }
    let half_rad = half_deg * pad.rad_per_deg;
    Advance {
        twist_deg: step_deg,
        plate_work_j: limit_n_m * half_rad * torque_sum,
        friction_heat_j: limit_n_m * half_rad * heat_sum,
        stored_energy_end_j: pad.stored_energy_j(load, twist_end_deg),
        stored_energy_begin_j: pad.stored_energy_j(load, twist_begin_deg),
        torque_n_m: pad.torque_n_m(load, twist_end_deg),
        slip_torque_limit_n_m: limit_n_m,
        stuck_fraction: pad.stuck_fraction(load, twist_end_deg),
    }
}

fn commit(
    pad: &FingerPadP02Glass,
    state: &mut FingerPadState,
    frame: &mut LastStateFrame64,
    load: FingerPadLoad,
    dt_s: f64,
    advance: Advance,
) -> Result<(), StepReject> {
    let saved = *state;
    if state.load_is_unset() {
        state.held_normal_force_n = load.normal_force_n;
        state.held_twist_rate_deg_s = load.twist_rate_deg_s;
        state.stored_energy_at_start_j = advance.stored_energy_begin_j;
    }
    state.t_s += dt_s;
    state.twist_deg += advance.twist_deg;
    state.torque_n_m = advance.torque_n_m;
    state.slip_torque_limit_n_m = advance.slip_torque_limit_n_m;
    state.stuck_fraction = advance.stuck_fraction;
    state.plate_work_j += advance.plate_work_j;
    state.friction_heat_j += advance.friction_heat_j;
    state.stored_energy_j = advance.stored_energy_end_j - state.stored_energy_at_start_j;
    state.full_slip = advance.stuck_fraction < pad.full_slip_stuck_fraction;
    state.beyond_compared = pad.beyond_compared(load, state.twist_deg);
    let scale = pad.run_ledger_scale_j(state.plate_work_j);
    if !(scale.is_finite() && scale > 0.0) {
        *state = saved;
        return Err(StepReject::InvalidFingerPad);
    }
    state.energy_defect_j = state.stored_energy_j + state.friction_heat_j - state.plate_work_j;
    state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    let gate = pad.residual_conservation_limit * scale;
    if state.energy_defect_j > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    if !(state.twist_deg >= 0.0) {
        *state = saved;
        return Err(StepReject::TwistBelowZero);
    }
    publish(state, frame);
    Ok(())
}

fn publish(state: &FingerPadState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.twist_deg as f32;
    frame.pos[1] = state.stuck_fraction as f32;
    frame.pos[2] = state.slip_torque_limit_n_m as f32;
    frame.vel[0] = state.plate_work_j as f32;
    frame.vel[1] = state.stored_energy_j as f32;
    frame.vel[2] = state.friction_heat_j as f32;
    frame.force_torque = state.torque_n_m as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.full_slip {
        flags |= FLAG_FULL_SLIP;
    }
    if state.beyond_compared {
        flags |= FLAG_BEYOND_COMPARED;
    }
    frame.flags = flags;
}

fn state_is_finite(state: &FingerPadState) -> bool {
    state.t_s.is_finite()
        && state.twist_deg.is_finite()
        && state.torque_n_m.is_finite()
        && state.slip_torque_limit_n_m.is_finite()
        && state.stuck_fraction.is_finite()
        && state.plate_work_j.is_finite()
        && state.stored_energy_j.is_finite()
        && state.friction_heat_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.held_normal_force_n.is_finite()
        && state.held_twist_rate_deg_s.is_finite()
        && state.stored_energy_at_start_j.is_finite()
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    FULL_SLIP_KIND,
    FORCE_LOW,
    FORCE_HIGH,
    RATE_LOW,
    RATE_HIGH,
    TWIST_HIGH,
];

const FULL_SLIP_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_FULL_SLIP",
    FLAG_FULL_SLIP,
    "stuck_fraction",
    "1",
    Side::Below,
    FULL_SLIP_STUCK_FRACTION,
    "a stuck fraction of 0.03: under it the whole contact slides, the event the card exists to find",
);
const fn beyond(quantity: &'static str, unit: &'static str, side: Side, limit: f64, words: &'static str) -> ExceedanceKind {
    ExceedanceKind::fixed("FLAG_BEYOND_COMPARED", FLAG_BEYOND_COMPARED, quantity, unit, side, limit, words)
}
const FORCE_WORDS: &str = "the normal forces compared with this finger, 0.5 to 10 N";
const FORCE_LOW: ExceedanceKind = beyond("normal_force_n", "N", Side::Below, COMPARED_FORCE_FLOOR_N, FORCE_WORDS);
const FORCE_HIGH: ExceedanceKind = beyond("normal_force_n", "N", Side::Above, COMPARED_FORCE_CEILING_N, FORCE_WORDS);
const RATE_WORDS: &str = "the twist rates compared with this finger, 5 to 100 degrees a second";
const RATE_LOW: ExceedanceKind = beyond("twist_rate_deg_s", "deg/s", Side::Below, COMPARED_RATE_FLOOR_DEG_S, RATE_WORDS);
const RATE_HIGH: ExceedanceKind = beyond("twist_rate_deg_s", "deg/s", Side::Above, COMPARED_RATE_CEILING_DEG_S, RATE_WORDS);
const TWIST_HIGH: ExceedanceKind = beyond(
    "twist_deg",
    "deg",
    Side::Above,
    COMPARED_TWIST_END_DEG,
    "the largest twist compared, 75 degrees; set past it by more than 4e-9 degrees",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The conditions are those of the step's `full_slip` and of
/// [`FingerPadP02Glass::beyond_compared`].
pub fn exceedances(
    pad: &FingerPadP02Glass,
    before: &FingerPadState,
    after: &FingerPadState,
    load: FingerPadLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.full_slip, Exceedance::at(FULL_SLIP_KIND, pad.full_slip_stuck_fraction, after.stuck_fraction));
    if after.beyond_compared {
        let force = load.normal_force_n;
        let rate = load.twist_rate_deg_s;
        records.push_if(!(force >= pad.compared_force_floor_n), Exceedance::at(FORCE_LOW, pad.compared_force_floor_n, force));
        records.push_if(!(force <= pad.compared_force_ceiling_n), Exceedance::at(FORCE_HIGH, pad.compared_force_ceiling_n, force));
        records.push_if(!(rate >= pad.compared_rate_floor_deg_s), Exceedance::at(RATE_LOW, pad.compared_rate_floor_deg_s, rate));
        records.push_if(!(rate <= pad.compared_rate_ceiling_deg_s), Exceedance::at(RATE_HIGH, pad.compared_rate_ceiling_deg_s, rate));
        records.push_if(
            !(after.twist_deg <= pad.compared_twist_end_deg + pad.twist_end_allowance_deg),
            Exceedance::at(TWIST_HIGH, pad.compared_twist_end_deg, after.twist_deg),
        );
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

    use super::{
        Advance, COMPARED_FORCE_CEILING_N, COMPARED_FORCE_FLOOR_N, COMPARED_RATE_CEILING_DEG_S,
        COMPARED_RATE_FLOOR_DEG_S, COMPARED_TWIST_END_DEG, FLAG_BEYOND_COMPARED, FLAG_FULL_SLIP,
        FingerPadLoad, FingerPadP02Glass, FingerPadState, GATE_DT_S, LEDGER_SCALE_TWIST_DEG,
        LOAD_EXPONENT, RAD_PER_DEG, RESIDUAL_CONSERVATION_LIMIT, SLIP_TORQUE_REFERENCE_N_M,
        STEP_TWIST_OVER_TORQUE_TWIST, STUCK_TWIST_REFERENCE_DEG, StepReject,
        TORQUE_TWIST_REFERENCE_DEG, commit, held, step, validate,
    };
    use crate::frame::LastStateFrame64;

    const PROOF: [u8; 16] = [0xA5; 16];

    fn blank() -> LastStateFrame64 {
        LastStateFrame64 {
            t: 0.0,
            pos: [0.0; 3],
            vel: [0.0; 3],
            force_torque: 0.0,
            residual: 0.0,
            flags: 0,
            proof: PROOF,
        }
    }

    fn pad() -> FingerPadP02Glass {
        FingerPadP02Glass::on_glass()
    }

    fn load(normal_force_n: f64, twist_rate_deg_s: f64) -> FingerPadLoad {
        FingerPadLoad {
            normal_force_n,
            twist_rate_deg_s,
        }
    }

    fn relative(a: f64, b: f64) -> f64 {
        ((a - b) / b).abs()
    }

    /// Twist from zero in `steps` steps of `dt_s`.
    fn run(load: FingerPadLoad, dt_s: f64, steps: usize) -> (FingerPadState, LastStateFrame64) {
        let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
        let mut frame = blank();
        for _ in 0..steps {
            step(&pad(), &mut state, &mut frame, load, dt_s).unwrap();
        }
        (state, frame)
    }

    /// The constants are the frozen fit of the research record
    /// (`analysis/constants_frozen.json`), to six significant figures.
    #[test]
    fn the_constants_are_the_frozen_fit() {
        assert_eq!(SLIP_TORQUE_REFERENCE_N_M, 0.0103527);
        assert_eq!(LOAD_EXPONENT, 0.77321);
        assert_eq!(TORQUE_TWIST_REFERENCE_DEG, 14.6438);
        assert_eq!(STUCK_TWIST_REFERENCE_DEG, 7.05215);
        assert!(relative(SLIP_TORQUE_REFERENCE_N_M, 0.010352749) < 5.0e-6);
        assert!(relative(RAD_PER_DEG, core::f64::consts::PI / 180.0) < 1.0e-16);
        assert!(pad().is_valid());
    }

    /// The closed forms against 50-digit arithmetic (mpmath): slip-torque
    /// limit, torque, stuck fraction, and the stored energy. The first five
    /// points are in the block at 2 N where the level was taken. The last
    /// five are at the other forces and rates measured on this finger, where
    /// the card answers with its misses stated.
    #[test]
    fn the_closed_forms_match_fifty_digit_arithmetic() {
        let cases: [(f64, f64, f64, [f64; 4]); 10] = [
            (2.0, 20.0, 75.0, [0.010352699999999999403, 0.01030856248856961514, 0.0086884221203768316552, 0.0015657893253009317]),
            (1.93, 5.0, 0.05, [0.010204103201592734625, 0.000040773397004671194195, 0.99308225712297768162, 1.7778085753199060458e-8]),
            (2.07, 50.0, 80.0, [0.010540183606593078003, 0.010524855535257525125, 0.0074446062594114563332, 0.001203849213319626084]),
            (2.0, 10.0, 30.0, [0.010420679857866548535, 0.0096148983929226358642, 0.050845344696636060205, 0.0014002871633480038372]),
            (2.07, 5.0, 5.0, [0.010771855185368472906, 0.0034894238783367518914, 0.50613749098452381967, 0.00014154852952654886795]),
            (10.0, 20.0, 75.0, [0.035933889360061229876, 0.035682049096026853979, 0.0086884221203768313145, 0.0063487607385529521213]),
            (1.0, 20.0, 30.0, [0.0060575112067896918969, 0.0058219456769322898306, 0.049883227244842311304, 0.00069846784518954182313]),
            (2.0, 100.0, 5.0, [0.010196561048892575783, 0.0066490884238416956103, 0.49090335582242270322, 0.00023481360930805232962]),
            (0.5, 5.0, 0.05, [0.0035910352849279090371, 0.000016645250319374597766, 0.99308225712297768149, 7.2568608464404776187e-9]),
            (5.0, 50.0, 80.0, [0.020844301313353461542, 0.020804089021021637412, 0.0074446062594114560371, 0.002609141405822190058]),
        ];
        for (force, rate, twist, want) in cases {
            let held = load(force, rate);
            assert!(relative(pad().slip_torque_limit_n_m(held), want[0]) < 1.0e-13);
            assert!(relative(pad().torque_n_m(held, twist), want[1]) < 1.0e-13);
            assert!(relative(pad().stuck_fraction(held, twist), want[2]) < 1.0e-13);
            assert!(relative(pad().stored_energy_j(held, twist), want[3]) < 1.0e-12);
        }
    }

    /// A whole twist to 75 degrees at 2 N and 20 degrees a second: the plate
    /// work and the heat against 50-digit closed forms, and the ledger at
    /// rounding. The only flag is the event of full slip.
    #[test]
    fn a_twist_to_75_degrees_closes_its_ledger() {
        let (state, frame) = run(load(2.0, 20.0), GATE_DT_S, 375);
        assert!((state.twist_deg - 75.0).abs() < 1.0e-9);
        assert!(relative(state.plate_work_j, 0.011928087417683076574) < 1.0e-10);
        assert!(relative(state.friction_heat_j, 0.010362298092382144874) < 1.0e-10);
        assert!(relative(state.stored_energy_j, 0.0015657893253009317) < 1.0e-10);
        assert!(state.residual_conservation_error < 1.0e-12);
        assert!(state.full_slip);
        assert_eq!(frame.flags, FLAG_FULL_SLIP);
        assert_eq!(frame.proof, PROOF);
        assert_eq!(frame.force_torque, state.torque_n_m as f32);
    }

    /// The heat never falls, the torque never falls, the stuck fraction
    /// never rises, and the torque stays under its limit. The frame carries
    /// no flag before full slip and the event's flag, alone, from it on. At
    /// the model's full slip the torque is 1.1 to 6.9 percent under its
    /// limit over the card's range.
    #[test]
    fn the_twist_goes_one_way_and_full_slip_is_the_only_flag() {
        let mut least: f64 = 1.0;
        let mut most: f64 = 0.0;
        for held in [load(1.93, 5.0), load(1.93, 50.0), load(2.0, 20.0), load(2.07, 5.0), load(2.07, 50.0)] {
            let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
            let mut frame = blank();
            let mut before = state;
            let steps = (75.0 / (held.twist_rate_deg_s * 0.002_f64)).round() as usize;
            for _ in 0..steps {
                step(&pad(), &mut state, &mut frame, held, 0.002).unwrap();
                assert!(state.friction_heat_j >= before.friction_heat_j);
                assert!(state.torque_n_m >= before.torque_n_m);
                assert!(state.stuck_fraction <= before.stuck_fraction);
                assert!(state.torque_n_m < state.slip_torque_limit_n_m);
                assert_eq!(frame.flags, if state.full_slip { FLAG_FULL_SLIP } else { 0 });
                if state.full_slip && !before.full_slip {
                    let under = 1.0 - state.torque_n_m / state.slip_torque_limit_n_m;
                    least = least.min(under);
                    most = most.max(under);
                    assert!(state.twist_deg > 38.9 && state.twist_deg < 40.4, "{}", state.twist_deg);
                }
                before = state;
            }
            assert_eq!(frame.flags, FLAG_FULL_SLIP);
        }
        assert!(least > 0.0105 && least < 0.0115, "{least}");
        assert!(most > 0.0680 && most < 0.0695, "{most}");
    }

    /// The ceiling step and the floor step reach the same state.
    #[test]
    fn small_steps_and_large_steps_agree() {
        let held = load(1.93, 50.0);
        let (coarse, _) = run(held, GATE_DT_S, 80);
        let (fine, _) = run(held, GATE_DT_S / 256.0, 80 * 256);
        assert!(relative(coarse.twist_deg, fine.twist_deg) < 1.0e-12);
        assert!(relative(coarse.torque_n_m, fine.torque_n_m) < 1.0e-12);
        assert!(relative(coarse.plate_work_j, fine.plate_work_j) < 1.0e-10);
        assert!(relative(coarse.friction_heat_j, fine.friction_heat_j) < 1.0e-10);
        assert!(coarse.residual_conservation_error < 1.0e-11);
        assert!(fine.residual_conservation_error < 1.0e-11);
    }

    /// The compared range is the forces and rates measured on this finger;
    /// its ends carry no flag, and the next 64-bit number outside each end
    /// lands with FLAG_BEYOND_COMPARED.
    #[test]
    fn the_compared_range_is_the_measured_forces_and_rates_and_beyond_it_a_frame_is_flagged() {
        assert_eq!((COMPARED_FORCE_FLOOR_N, COMPARED_FORCE_CEILING_N), (0.5, 10.0));
        assert_eq!((COMPARED_RATE_FLOOR_DEG_S, COMPARED_RATE_CEILING_DEG_S), (5.0, 100.0));
        assert_eq!(COMPARED_TWIST_END_DEG, 75.0);
        assert_eq!(FLAG_BEYOND_COMPARED & FLAG_FULL_SLIP, 0);
        let below = |x: f64| f64::from_bits(x.to_bits() - 1);
        let above = |x: f64| f64::from_bits(x.to_bits() + 1);
        for (held, flagged) in [
            (load(0.5, 20.0), false),
            (load(10.0, 20.0), false),
            (load(2.0, 5.0), false),
            (load(2.0, 100.0), false),
            (load(below(0.5), 20.0), true),
            (load(above(10.0), 20.0), true),
            (load(2.0, below(5.0)), true),
            (load(2.0, above(100.0)), true),
            (load(1.0e-3, 20.0), true),
            (load(1.0e4, 20.0), true),
            (load(2.0, 1.0e-3), true),
        ] {
            let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
            let mut frame = blank();
            let dt_s = 0.004;
            assert_eq!(step(&pad(), &mut state, &mut frame, held, dt_s), Ok(()), "{held:?}");
            assert_eq!(frame.flags & FLAG_BEYOND_COMPARED != 0, flagged, "{held:?}");
            assert_eq!(frame.flags & !(FLAG_BEYOND_COMPARED | FLAG_FULL_SLIP), 0);
            assert!(state.residual_conservation_error < 1.0e-12);
        }
    }

    /// Every twist engine 0.9 answered, 1.93 to 2.07 N and 5 to 50 degrees a
    /// second to 75 degrees at steps up to 0.01 s, lands as it did: it is
    /// inside the quadrature step, the fixed scale is the larger, and no
    /// frame of it is flagged beyond the compared range.
    #[test]
    fn every_twist_of_the_band_answers_as_it_did() {
        for force in [1.93, 2.0, 2.07] {
            for rate in [5.0, 50.0] {
                let held = load(force, rate);
                assert!(rate * GATE_DT_S <= STEP_TWIST_OVER_TORQUE_TWIST * pad().torque_twist_deg(held));
                let steps = (75.0 / (rate * GATE_DT_S)).round() as usize;
                let (state, frame) = run(held, GATE_DT_S, steps);
                assert!(state.plate_work_j < pad().ledger_scale_j());
                assert_eq!(frame.flags & FLAG_BEYOND_COMPARED, 0);
            }
        }
    }

    /// Every condition the comparison of 2026-10-09 missed its mark at is
    /// answered, with the law as written: the other loads of the finger,
    /// fitted or held back, and the fastest rate. The slip torque of each is
    /// the closed form, and the ledger closes over a twist to 75 degrees.
    #[test]
    fn the_conditions_the_card_missed_are_answered_with_the_law_as_written() {
        for (force, rate) in [(0.5, 20.0), (1.0, 20.0), (1.8, 20.0), (5.0, 20.0), (10.0, 20.0), (2.0, 100.0)] {
            let held = load(force, rate);
            let dt_s = 0.0025;
            let steps = (75.0 / (rate * dt_s)).round() as usize;
            let (state, frame) = run(held, dt_s, steps);
            assert!((state.twist_deg - 75.0).abs() < 1.0e-9, "{force} {rate}");
            assert_eq!(state.slip_torque_limit_n_m, pad().slip_torque_limit_n_m(held));
            assert!(relative(state.torque_n_m, pad().torque_n_m(held, state.twist_deg)) < 1.0e-15);
            assert!(state.residual_conservation_error < 1.0e-12, "{force} {rate}");
            assert_eq!(frame.flags, FLAG_FULL_SLIP, "{force} {rate}");
        }
        // The slip torque at 10 N and at 100 degrees a second, the two held-back
        // conditions the card missed by the most: 29.3 percent high, 13.0 low.
        let at_ten = pad().slip_torque_limit_n_m(load(10.0, 20.0));
        let at_hundred = pad().slip_torque_limit_n_m(load(2.0, 100.0));
        assert!((at_ten - 0.0359339).abs() < 1.0e-7, "{at_ten}");
        assert!((at_hundred - 0.0101966).abs() < 1.0e-7, "{at_hundred}");
    }

    #[test]
    fn each_limit_is_named_and_nothing_moves() {
        // Ten degrees into a twist at 2 N and 20 degrees a second.
        let (start, start_frame) = run(load(2.0, 20.0), GATE_DT_S, 50);
        let good = load(2.0, 20.0);
        let cases = [
            (load(f64::NAN, 20.0), 0.01, StepReject::NonFiniteInput),
            (load(2.0, f64::INFINITY), 0.01, StepReject::NonFiniteInput),
            (good, f64::NAN, StepReject::NonFiniteInput),
            (good, 0.0, StepReject::NonPositiveDt),
            (good, -0.01, StepReject::NonPositiveDt),
            (load(0.0, 20.0), 0.01, StepReject::NoContact),
            (load(-0.0, 20.0), 0.01, StepReject::NoContact),
            (load(-2.0, 20.0), 0.01, StepReject::NoContact),
            (load(1.0e-310, 20.0), 0.01, StepReject::NoContact),
            (load(2.0, 0.0), 0.01, StepReject::TwistRateNotPositive),
            (load(2.0, -20.0), 0.01, StepReject::TwistRateNotPositive),
            (load(2.0, 5.0e-324), 0.01, StepReject::TwistRateNotPositive),
            (load(0.0, 0.0), 0.01, StepReject::NoContact),
            (good, 3.6, StepReject::TwistStepAboveQuadrature),
            (good, 0.0367, StepReject::TwistStepAboveQuadrature),
        ];
        for (held, dt_s, want) in cases {
            let mut state = start;
            let mut frame = start_frame;
            assert_eq!(step(&pad(), &mut state, &mut frame, held, dt_s), Err(want));
            assert_eq!(state, start);
            assert_eq!(frame.to_bytes(), start_frame.to_bytes());
        }
    }

    /// A run starts with the pad untwisted. A start at any other twist is
    /// refused, a twist that reads as zero in a 32-bit slot included, and so
    /// is the first step of a state that claims one.
    #[test]
    fn a_start_at_a_twist_other_than_zero_is_refused_by_name() {
        for twist in [1.0e-46, 1.0e-300, 5.0e-324, 0.5, 30.0, 80.0, 80.1, -0.1, f64::NAN, f64::INFINITY] {
            assert!(FingerPadState::try_start(&pad(), twist).is_none(), "{twist}");
        }
        let from_minus_zero = FingerPadState::try_start(&pad(), -0.0).unwrap();
        assert_eq!(from_minus_zero.twist_deg.to_bits(), 0.0_f64.to_bits());
        for twist in [1.0e-46, 30.0, -0.1] {
            let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
            state.twist_deg = twist;
            let saved = state;
            let mut frame = blank();
            assert_eq!(
                step(&pad(), &mut state, &mut frame, load(2.0, 20.0), GATE_DT_S),
                Err(StepReject::TwistNotStartedFromZero)
            );
            assert_eq!(state, saved);
            assert_eq!(frame.to_bytes(), blank().to_bytes());
        }
    }

    /// No trial changed the force or the rate during a twist, and the card
    /// refuses a step that does.
    #[test]
    fn a_load_changed_during_the_twist_is_named() {
        let (mut state, mut frame) = run(load(2.0, 20.0), GATE_DT_S, 50);
        let saved = state;
        let bytes = frame.to_bytes();
        for changed in [load(2.05, 20.0), load(2.0, 50.0), load(2.000_000_000_000_000_4, 20.0)] {
            assert_eq!(
                step(&pad(), &mut state, &mut frame, changed, GATE_DT_S),
                Err(StepReject::LoadChangedDuringTwist)
            );
            assert_eq!(state, saved);
            assert_eq!(frame.to_bytes(), bytes);
        }
        assert_eq!(step(&pad(), &mut state, &mut frame, load(2.0, 20.0), GATE_DT_S), Ok(()));
        // The state carries the load it was twisted under, so the refusal
        // holds for a state taken up again later.
        let mut later = state;
        let mut later_frame = blank();
        assert_eq!(
            step(&pad(), &mut later, &mut later_frame, load(2.07, 20.0), GATE_DT_S),
            Err(StepReject::LoadChangedDuringTwist)
        );
        assert_eq!((later.held_normal_force_n, later.held_twist_rate_deg_s), (2.0, 20.0));
    }

    #[test]
    fn a_broken_state_and_a_changed_pad_are_named() {
        let good = load(2.0, 20.0);
        let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
        state.plate_work_j = f64::NAN;
        let saved_bits = state.plate_work_j.to_bits();
        let mut frame = blank();
        assert_eq!(
            step(&pad(), &mut state, &mut frame, good, GATE_DT_S),
            Err(StepReject::NonFiniteState)
        );
        assert_eq!(state.plate_work_j.to_bits(), saved_bits);
        let mut softer = pad();
        softer.load_exponent = 1.0;
        let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
        assert_eq!(
            step(&softer, &mut state, &mut frame, good, GATE_DT_S),
            Err(StepReject::InvalidFingerPad)
        );
        assert!(FingerPadState::try_start(&softer, 0.0).is_none());
        assert_eq!(frame.to_bytes(), blank().to_bytes());
    }

    /// Rule 10. A wrong stepping rule (the torque at the start of the step
    /// held over the whole step, and the heat likewise) and a falsified
    /// ledger are each named, and nothing moves.
    #[test]
    fn a_wrong_stepping_rule_and_a_falsified_ledger_are_named() {
        let held_load = load(2.07, 50.0);
        let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
        let mut frame = blank();
        let mut named = None;
        for _ in 0..160 {
            let begin = state.twist_deg;
            let mut advance = held(&pad(), begin, held_load, GATE_DT_S);
            let step_rad = advance.twist_deg * RAD_PER_DEG;
            advance.plate_work_j = pad().torque_n_m(held_load, begin) * step_rad;
            let saved = state;
            let bytes = frame.to_bytes();
            validate(&pad(), &state, held_load, GATE_DT_S).unwrap();
            if let Err(reject) = commit(&pad(), &mut state, &mut frame, held_load, GATE_DT_S, advance) {
                assert_eq!(state, saved);
                assert_eq!(frame.to_bytes(), bytes);
                named = Some(reject);
                break;
            }
        }
        // The rectangle rule under-counts the work of a rising torque.
        assert_eq!(named, Some(StepReject::SpontaneousGeneration));

        let (state, frame) = run(held_load, GATE_DT_S, 60);
        let honest = held(&pad(), state.twist_deg, held_load, GATE_DT_S);
        let scale = pad().ledger_scale_j();
        for (factor, want) in [
            (1.0 + 1.0e-4, StepReject::SpontaneousGeneration),
            (1.0 - 1.0e-4, StepReject::FirstLawBreach),
        ] {
            let mut forged: Advance = honest;
            forged.friction_heat_j = honest.friction_heat_j * factor;
            assert!((honest.friction_heat_j * 1.0e-4) > RESIDUAL_CONSERVATION_LIMIT * scale);
            let mut trial = state;
            let mut trial_frame = frame;
            assert_eq!(
                commit(&pad(), &mut trial, &mut trial_frame, held_load, GATE_DT_S, forged),
                Err(want)
            );
            assert_eq!(trial, state);
            assert_eq!(trial_frame.to_bytes(), frame.to_bytes());
        }
    }

    /// The slowest whole twist at the step floor: 384,000 steps, and the
    /// ledger stays inside its limit with room.
    #[test]
    fn the_slowest_twist_at_the_step_floor_stays_inside_the_ledger() {
        let floor = pad().floor_dt_s().unwrap();
        assert_eq!(floor, 3.90625e-5);
        let (state, _) = run(load(2.07, 5.0), floor, 384_000);
        assert!((state.twist_deg - 75.0).abs() < 4.0e-9);
        assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT / 100.0);
    }

    /// The ledger scale is the scale of the build that was scored: the law's
    /// slip-torque limit at 10 N and 5 degrees a second over 80 degrees,
    /// 0.0508 J (50-digit value 0.050834251118021514563).
    #[test]
    fn the_ledger_scale_is_that_of_the_scored_build() {
        let scale = pad().ledger_scale_j();
        assert!(relative(scale, 0.050834251118021514563) < 1.0e-14, "{scale}");
        assert_eq!(pad().gate_dt_s(), Some(0.01));
        // The scale keeps the whole turn of the plate, 80 degrees; the twist
        // was compared to 75.
        assert_eq!((LEDGER_SCALE_TWIST_DEG, COMPARED_TWIST_END_DEG), (80.0, 75.0));
        // A run whose plate work passes the fixed energy is held to its work.
        assert_eq!(pad().run_ledger_scale_j(0.01), scale);
        assert_eq!(pad().run_ledger_scale_j(1.0), 1.0);
    }

    /// The twist from 75 to 80 degrees was never compared with measurement,
    /// and the measured plate slowed to a stop there. The card goes on: a
    /// request of 160 steps of 0.01 s at 2.07 N and 50 degrees a second, which
    /// engine 0.9 refused at the 151st, lands all 160, the frames past 75
    /// degrees flagged. A twist on to ten turns at 2 N and 20 degrees a second
    /// carries the torque up toward its slip limit, the heat up with the
    /// twist, the stored energy toward its full-slip value, and closes its
    /// ledger at every step, with the scale following the work.
    #[test]
    fn a_twist_past_75_degrees_goes_on_with_its_ledger_closed() {
        let held_load = load(2.07, 50.0);
        let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
        let mut frame = blank();
        for count in 1..=160 {
            step(&pad(), &mut state, &mut frame, held_load, GATE_DT_S).unwrap();
            let past = state.twist_deg > COMPARED_TWIST_END_DEG;
            assert_eq!(frame.flags & FLAG_BEYOND_COMPARED != 0, past, "{count}");
            assert_eq!(past, count > 150, "{count}");
        }
        assert!((state.twist_deg - 80.0).abs() < 1.0e-9);

        let held_load = load(2.0, 20.0);
        let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
        let mut frame = blank();
        let mut before = state;
        let mut worst = 0.0_f64;
        let mut followed = false;
        for _ in 0..18_000 {
            step(&pad(), &mut state, &mut frame, held_load, GATE_DT_S).unwrap();
            assert!(state.torque_n_m >= before.torque_n_m);
            assert!(state.torque_n_m <= state.slip_torque_limit_n_m);
            assert!(state.friction_heat_j > before.friction_heat_j);
            worst = worst.max(state.residual_conservation_error);
            if state.plate_work_j > pad().ledger_scale_j() {
                followed = true;
                assert_eq!(
                    state.residual_conservation_error,
                    state.energy_defect_j.abs() / state.plate_work_j
                );
            }
            before = state;
        }
        assert!((state.twist_deg - 3600.0).abs() < 1.0e-6, "{}", state.twist_deg);
        assert!(followed);
        assert!(worst < 1.0e-12, "{worst:e}");
        assert!(relative(state.torque_n_m, state.slip_torque_limit_n_m) < 1.0e-4);
        assert_eq!(frame.flags, FLAG_FULL_SLIP | FLAG_BEYOND_COMPARED);
        // The stored energy tends to (15/4) T_limit theta_T / 6 in full slip.
        let full = 3.75 * state.slip_torque_limit_n_m * pad().torque_twist_deg(held_load) * RAD_PER_DEG / 6.0;
        assert!(relative(state.stored_energy_j + state.stored_energy_at_start_j, full) < 1.0e-3);
    }

    /// Rule 10 past every lifted wall: at a force and a rate far past the
    /// measured ones and past 75 degrees, a wrong stepping rule and a
    /// falsified heat are named, and nothing moves; a step that would turn
    /// the plate past the quadrature step is named.
    #[test]
    fn past_the_lifted_walls_a_wrong_step_and_a_falsified_ledger_are_named() {
        for held_load in [load(0.05, 20.0), load(200.0, 20.0), load(2.0, 1.0), load(2.0, 1000.0)] {
            let dt_s = 0.5 * STEP_TWIST_OVER_TORQUE_TWIST * pad().torque_twist_deg(held_load)
                / held_load.twist_rate_deg_s;
            // The rectangle rule under-counts the work of a rising torque.
            let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
            let mut frame = blank();
            let mut named = None;
            for _ in 0..4000 {
                let begin = state.twist_deg;
                let mut advance = held(&pad(), begin, held_load, dt_s);
                let step_rad = advance.twist_deg * RAD_PER_DEG;
                advance.plate_work_j = pad().torque_n_m(held_load, begin) * step_rad;
                let saved = state;
                let bytes = frame.to_bytes();
                validate(&pad(), &state, held_load, dt_s).unwrap();
                if let Err(reject) = commit(&pad(), &mut state, &mut frame, held_load, dt_s, advance) {
                    assert_eq!(state, saved);
                    assert_eq!(frame.to_bytes(), bytes);
                    named = Some(reject);
                    break;
                }
            }
            assert_eq!(named, Some(StepReject::SpontaneousGeneration), "{held_load:?}");
            // A heat forged by one part in 10^4, deep in full slip past 75
            // degrees, both ways.
            let steps = (120.0 / (held_load.twist_rate_deg_s * dt_s)).round() as usize;
            let (state, frame) = run(held_load, dt_s, steps);
            assert!(state.twist_deg > COMPARED_TWIST_END_DEG);
            let honest = held(&pad(), state.twist_deg, held_load, dt_s);
            for (factor, want) in [
                (1.0 + 1.0e-4, StepReject::SpontaneousGeneration),
                (1.0 - 1.0e-4, StepReject::FirstLawBreach),
            ] {
                let mut forged: Advance = honest;
                forged.friction_heat_j = honest.friction_heat_j * factor;
                let mut trial = state;
                let mut trial_frame = frame;
                assert_eq!(
                    commit(&pad(), &mut trial, &mut trial_frame, held_load, dt_s, forged),
                    Err(want),
                    "{held_load:?}"
                );
                assert_eq!(trial, state);
                assert_eq!(trial_frame.to_bytes(), frame.to_bytes());
            }
            // Four times the half quadrature step is refused by name, state
            // and frame kept.
            let mut trial = state;
            let mut trial_frame = frame;
            assert_eq!(
                step(&pad(), &mut trial, &mut trial_frame, held_load, 4.0 * dt_s),
                Err(StepReject::TwistStepAboveQuadrature)
            );
            assert_eq!(trial, state);
            assert_eq!(trial_frame.to_bytes(), frame.to_bytes());
        }
    }

    /// The largest plate work inside the range is that of a twist to 75
    /// degrees at 2.07 N and 50 degrees a second, 0.0126 J, and the gate of
    /// the ledger is 4.0e-9 of it. The slip-torque limit times the twist is
    /// an upper bound the torque never reaches.
    #[test]
    fn the_largest_work_inside_the_range_is_as_described() {
        let (largest, _) = run(load(2.07, 50.0), GATE_DT_S, 150);
        assert!((largest.plate_work_j - 0.0126).abs() < 5.0e-5, "{}", largest.plate_work_j);
        let ratio = RESIDUAL_CONSERVATION_LIMIT * pad().ledger_scale_j() / largest.plate_work_j;
        assert!((ratio - 4.0e-9).abs() < 5.0e-11, "{ratio}");
        for force in [1.93, 2.0, 2.07] {
            for rate in [5.0, 7.0, 10.0, 20.0, 30.0, 40.0, 50.0] {
                let steps = (75.0 / (rate * 0.01_f64)).floor() as usize;
                let (state, _) = run(load(force, rate), 0.01, steps);
                assert!(state.plate_work_j <= largest.plate_work_j, "{force} {rate}");
            }
        }
    }
}
