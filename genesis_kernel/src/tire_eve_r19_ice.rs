#![no_implicit_prelude]
//! One 235/55 R19 all-season sport-utility tyre rolling on ice at a held slip:
//! the tyre of the European project EVE on the Terramechanics Rig of the
//! Advanced Vehicle Dynamics Laboratory at Virginia Polytechnic Institute and
//! State University, April 2016.
//!
//! The specimen and its data. Schleinin, Savitski, Jimenez, Sandu and Ivanov,
//! "Dataset of tyre tests on ice", parts 1 to 3, Zenodo records 1009017,
//! 1009023 and 1009025, CC BY 4.0: 252 runs. In each run the rig carries the
//! wheel along a prepared ice track at one inflation pressure and one normal
//! load, and holds four slip ratios in turn for about 21 s each, driving
//! (traction) or retarding (braking) the wheel. A file gives time,
//! longitudinal force, vertical force and wheel angle at 160 samples a
//! second. The tyre size is from Agliullin et al., "Torque Vectoring Control
//! on Ice for Electric Vehicles with Individually Actuated Wheels", Zenodo
//! record 3515087. The record in the research folder is
//! `evidence/research/tire_eve_235_55r19_ice/`.
//!
//! What the files do not give: the carriage speed, the ice temperature, the
//! rolling radius, and the tyre's make. At zero slip the wheel turns at about
//! 9.45 degrees a second, which with the tyre's nominal unloaded radius,
//! 0.371 m, is about 0.06 m/s.
//!
//! The three fitted constants. [`MU_TRACTION`], [`MU_BRAKING`] and
//! [`SLIP_EXPONENT`] are fitted to this tyre's own measurements: least squares
//! on the logarithm of 48 cell means from 72 runs at 3.5 and 5 kN, 1.5 and
//! 3.5 bar, at slips of 5, 6, 7, 12, 15 and 20 percent, each mean over three
//! repeats of the last nine seconds of a slip stage. No constant here is
//! taken from a handbook or derived from the rubber or the ice.
//!
//! The comparison with runs the fit never saw, made once on 2026-10-09. Held
//! back: all 54 runs at 6.5 kN, a load above every fitted load, and all 36
//! runs at 2.5 bar at the two fitted loads. The lines were fixed beforehand
//! (pre-registration SHA-256
//! 84a380a5c948cd741ac91845fbfeac7ed1b60ed3c6a01855bcb99c996e7782c1) and
//! all five were met: the root-mean-square error of the friction ratio was
//! 0.034 over the 60 held-back cells, 0.040 at 2.5 bar and 0.029 at 6.5 kN,
//! against a line of 0.045; a ratio of 0.1 gave 0.122 and the opened runs'
//! geometric mean with no slip term (least squares on the logarithm) 0.036.
//! The card read high by 0.019 on average, and
//! two conditions taken alone were above 0.045. The same campaign, rig and
//! ice as the fitted runs: nothing was tested at another ice temperature,
//! speed, tyre or laboratory.
//!
//! What is left out. The normal load and the inflation pressure do not enter
//! the friction ratio: with one of the four fitted load-pressure conditions
//! left out, a ratio that varied with load and pressure predicted the
//! left-out condition worse than one that did not, and the traction and the
//! braking runs gave load exponents of opposite sign. The pressure is a
//! caller's input only so that a run outside the tested pressures is named.
//! Also left out: the rise of the force from rest, which the first stage of
//! every run shows and which lasts most of a stage; slips under 5 percent,
//! where the measured force had not settled inside a stage; the ice
//! temperature, which the files do not record; any speed but the rig's
//! creep; a steered or cambered wheel; water on the ice; rolling resistance;
//! and the wheel's own inertia.
//!
//! The equations. With normal load `F_z`, slip `s` (positive in traction,
//! negative in braking) and carriage speed `v`:
//!
//! ```text
//! mu      = MU_mode · (|s| / 0.10)^SLIP_EXPONENT
//! F_x     = sign(s) · mu · F_z                 force of the ice on the tyre
//! v_rim   = v / (1 − s)                        s = 1 − v / v_rim
//! v_slide = v · s / (1 − s)                    = v_rim − v
//! ```
//!
//! The slip is the rig's: the wheel speeds of the opened runs follow
//! `v / (1 − s)` in both modes, 11.83 degrees a second at 20 percent traction
//! and 7.93 at 20 percent braking from 9.45 at zero.
//!
//! The ledger is the work of the three speeds, each summed on its own:
//!
//! ```text
//! wheel work   W += F_x · v_rim · dt       positive when the hub drives the tyre
//! drawbar work D += F_x · v · dt           positive when the tyre pulls the carriage
//! slip heat    H += |F_x| · |v_slide| · dt never falls
//! defect       = D + H − W
//! ```
//!
//! The wheel work is the drawbar work plus the heat of sliding. The defect is
//! held to [`RESIDUAL_CONSERVATION_LIMIT`] of the ledger scale, the heaviest
//! load at the highest friction ratio over the whole track, about 10.3 kJ. A
//! positive defect is work that came from nowhere and is
//! [`StepReject::SpontaneousGeneration`]; a negative one is
//! [`StepReject::FirstLawBreach`].
//!
//! The power is carried as `exp(q · log(x))` with the crate's own exponential
//! and logarithm, so the card takes no function from the platform but
//! `sqrt`, which it does not use.
//!
//! The time at one slip. The comparison scored each slip stage on the window
//! from 10 s to 1 s before it ended. Run again by the same cells and lines on
//! the time after each slip change (2026-10-10), with the change placed where
//! the wheel speed steps, the force ratio met all five lines in every
//! one-second window from 5 s to 20 s after the change, and every one-second
//! window before 5 s failed a line: the first second failed the 0.045 line at
//! 6.5 kN (0.048) and the comparator, and the seconds from 1 s to 5 s did no
//! better than the constant comparator. With the protocol's own cuts the
//! window from 5 s to 11 s met all five lines as well. No scored slip was held longer than one
//! stage, about 20 s, except the last slip of a run, 7 or 20 percent, which
//! the rig held for 21 to 33 s. The card therefore stands behind the force
//! from [`SLIP_SETTLE_S`], 5 s, to [`HOLD_CEILING_S`], 21 s, after a slip was
//! set. The state carries `slip_held_s`, the time the present slip has been
//! held; a start states it, and a run carries it from step to step while the
//! slip is held. A step that starts before 5 s, which every step after a
//! change of slip does, is [`StepReject::SlipNotSettled`]; a step that ends
//! beyond 21 s is [`StepReject::HoldBeyondMeasured`]. The time held is a
//! running sum like the distance, and the comparison with 21 s carries
//! [`HOLD_END_ALLOWANCE_PER_STEP_S`], 2^-49 s, half the spacing of 64-bit
//! numbers between 16 s and 32 s, for each of the 16,384 steps of the
//! floor in 16 s and six more.
//!
//! [`TireEveR19Ice::gate_dt_s`] is the step ceiling, 16 s: the time from 5 s
//! to 21 s at one slip. The force law has no dynamics, so the step is exact
//! at any size. [`TireEveR19Ice::floor_dt_s`] is the ceiling divided by 2^14,
//! 2^-10 s, about a sixth of the sampling interval of the measurements. A
//! run is then at most 16,384 steps, and the residual of such a run at the
//! floor stays under 1e-11 of the ledger scale, a hundredth of its limit. The
//! public step applies neither. The gateway refuses a step outside them.
//!
//! The end of the track and rounding. The distance is a running sum,
//! `distance += v · dt` in 64-bit arithmetic, and each addition rounds by at
//! most half the spacing of 64-bit numbers between 4 m and 8 m, 2^-51 m. A
//! run whose end in decimal is exactly 6 m can therefore reach the last step
//! with a sum a few spacings past 6 m: 1,000 steps of 0.1 s at 0.06 m/s read
//! 5.994000000000005 m before the last. A distance handed back as a new
//! start carries the rounding of the steps before it. The comparison with
//! the end of the track carries [`TireEveR19Ice::track_end_allowance_m`]:
//! [`TRACK_END_ALLOWANCE_PER_STEP_M`], 2^-51 m, for each step the whole track
//! can hold at the step floor and the slowest speed, 6 m over 0.05 m/s times
//! the floor, 122,880 steps, and [`TRACK_END_ALLOWANCE_EXTRA_STEPS`], six,
//! more: one for the start read from a decimal, and five for the speed and
//! the step read from decimals and their product, each within one part in
//! 2^53 of itself, over 6 m. A returned distance reads back exactly, so no
//! mix of steps and restarts along the track can round by more. The
//! allowance is the same at every step, 5.5e-11 m, and is never more than
//! half the step; the comparison is with the 64-bit number nearest 6 m plus
//! the allowance. The distance is stepped at its own value and nothing is
//! pulled back: a run that ends on the track's end in decimal can read up to
//! the allowance past 6 m, and a step that would end farther is
//! [`StepReject::TravelBeyondTrack`].
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes time, the distance along the
//! track, the rim travel, the slip, the three works, the longitudinal force
//! and the conservation residual into [`LastStateFrame64`]; the time held at
//! the slip is in the state and has no frame slot. The 16-byte frame proof is
//! not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Longitudinal force over normal load at 10 percent slip, driven wheel.
/// Fitted to this tyre's opened traction runs.
pub const MU_TRACTION: f64 = 0.228109;
/// Longitudinal force over normal load at 10 percent slip, retarded wheel.
/// Fitted to this tyre's opened braking runs.
pub const MU_BRAKING: f64 = 0.24162;
/// Power of the slip in the friction ratio. Fitted to both modes together.
pub const SLIP_EXPONENT: f64 = -0.132823;
/// The slip at which the two ratios above are stated.
pub const SLIP_REFERENCE: f64 = 0.10;
/// Smallest slip magnitude the card accepts: the smallest scored slip.
pub const SLIP_FLOOR: f64 = 0.05;
/// Largest slip magnitude the card accepts: the largest slip measured.
pub const SLIP_CEILING: f64 = 0.20;
/// Smallest normal load the card accepts, N: the smallest load measured.
pub const NORMAL_LOAD_FLOOR_N: f64 = 3_500.0;
/// Largest normal load the card accepts, N: the largest load measured, which
/// the constants were not fitted on.
pub const NORMAL_LOAD_CEILING_N: f64 = 6_500.0;
/// Largest normal load the constants were fitted on, N. Advisory only; above
/// it the frame carries [`FLAG_LOAD_ABOVE_FITTED`].
pub const NORMAL_LOAD_FITTED_CEILING_N: f64 = 5_000.0;
/// Smallest inflation pressure the card accepts, Pa: 1.5 bar.
pub const INFLATION_PRESSURE_FLOOR_PA: f64 = 150_000.0;
/// Largest inflation pressure the card accepts, Pa: 3.5 bar.
pub const INFLATION_PRESSURE_CEILING_PA: f64 = 350_000.0;
/// Smallest carriage speed the card accepts, m/s.
pub const CARRIAGE_SPEED_FLOOR_M_S: f64 = 0.05;
/// Largest carriage speed the card accepts, m/s. The rig moved at about 0.06.
pub const CARRIAGE_SPEED_CEILING_M_S: f64 = 0.07;
/// Length of track the card accepts, m. The opened runs roll for 90 to 97 s,
/// about 5.7 m.
pub const TRACK_LENGTH_M: f64 = 6.0;
/// Half the spacing of 64-bit numbers between 4 m and 8 m, 2^-51 m: the most
/// one addition to the distance can round by. The comparison with the end of
/// the track allows this much for each step the whole track can hold at the
/// step floor and the slowest speed.
pub const TRACK_END_ALLOWANCE_PER_STEP_M: f64 = 4.440_892_098_500_626e-16;
/// Steps added to the count in [`TireEveR19Ice::track_end_allowance_m`]: one
/// for the start read from a decimal, five for the speed and the step read
/// from decimals and for their product, over 6 m. The same six are added to
/// the count in [`TireEveR19Ice::hold_end_allowance_s`].
pub const TRACK_END_ALLOWANCE_EXTRA_STEPS: f64 = 6.0;
/// Time at one slip before which the card does not stand behind the force,
/// s. Every one-second window of the measurements before it failed a line of
/// the comparison; every one from it to 20 s met all five.
pub const SLIP_SETTLE_S: f64 = 5.0;
/// Longest time at one slip the card stands behind, s: one slip stage of the
/// measurements.
pub const HOLD_CEILING_S: f64 = 21.0;
/// Half the spacing of 64-bit numbers between 16 s and 32 s, 2^-49 s: the
/// most one addition to the time held can round by.
pub const HOLD_END_ALLOWANCE_PER_STEP_S: f64 = 1.776_356_839_400_250_5e-15;
/// Step ceiling, s: the time from [`SLIP_SETTLE_S`] to [`HOLD_CEILING_S`].
pub const GATE_DT_S: f64 = 16.0;
/// Step ceiling divided by step floor: 2^14.
pub const GATE_OVER_FLOOR: f64 = 16_384.0;
/// `|D + H − W| / W_scale` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;

/// The normal load is above 5 000 N, the largest load the constants were
/// fitted on, on this frame.
pub const FLAG_LOAD_ABOVE_FITTED: u64 = 1 << 0;

/// Why a step did not advance the tyre or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The load, the pressure, the slip, the speed or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this tyre's.
    InvalidTire,
    /// The normal load is below 3 500 N or above 6 500 N, the loads measured.
    NormalLoadOutsideCard,
    /// The inflation pressure is below 150 kPa or above 350 kPa, the pressures measured.
    InflationPressureOutsideCard,
    /// The slip magnitude is below 0.05 or above 0.20, the slips the constants cover.
    SlipOutsideCard,
    /// The carriage speed is below 0.05 m/s or above 0.07 m/s, the creep of the rig.
    CarriageSpeedOutsideCard,
    /// The distance along the track is below zero, or the step ends beyond 6 m, the length rolled in a run, by more than the rounding allowance of the running sum: 4.44e-16 m for each of the 122,880 steps of the step floor at 0.05 m/s along 6 m, and six more, 5.5e-11 m at every step.
    TravelBeyondTrack,
    /// The slip has been held less than 5 s at the start of the step: a start that states less, or the first step after a change of slip. The force in the first 5 s after a slip is set failed the comparison with the measurements.
    SlipNotSettled,
    /// The step ends more than 21 s after the slip was set, beyond the rounding allowance of the running sum: no scored slip was compared beyond one 21 s stage.
    HoldBeyondMeasured,
    /// The trial step created work: the drawbar work plus the slip heat rose above the wheel work.
    SpontaneousGeneration,
    /// The trial step destroyed work: the drawbar work plus the slip heat fell below the wheel work.
    FirstLawBreach,
}

/// What the rig holds over a step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TireLoad {
    /// Normal load on the wheel, `F_z`.
    pub normal_load_n: f64,
    /// Inflation pressure, gauge. Checked against the tested range; it does
    /// not enter the force.
    pub inflation_pressure_pa: f64,
    /// Slip `s = 1 − v / v_rim`: positive drives the wheel, negative retards it.
    pub slip_ratio: f64,
    /// Speed of the carriage along the track, `v`.
    pub carriage_speed_m_s: f64,
}

/// The locked tyre. Every field is a recorded copy of a crate constant and
/// none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TireEveR19Ice {
    /// Recorded copy of [`MU_TRACTION`].
    pub mu_traction: f64,
    /// Recorded copy of [`MU_BRAKING`].
    pub mu_braking: f64,
    /// Recorded copy of [`SLIP_EXPONENT`].
    pub slip_exponent: f64,
    /// Recorded copy of [`SLIP_REFERENCE`].
    pub slip_reference: f64,
    /// Recorded copy of [`SLIP_FLOOR`].
    pub slip_floor: f64,
    /// Recorded copy of [`SLIP_CEILING`].
    pub slip_ceiling: f64,
    /// Recorded copy of [`NORMAL_LOAD_FLOOR_N`].
    pub normal_load_floor_n: f64,
    /// Recorded copy of [`NORMAL_LOAD_CEILING_N`].
    pub normal_load_ceiling_n: f64,
    /// Recorded copy of [`NORMAL_LOAD_FITTED_CEILING_N`].
    pub normal_load_fitted_ceiling_n: f64,
    /// Recorded copy of [`INFLATION_PRESSURE_FLOOR_PA`].
    pub inflation_pressure_floor_pa: f64,
    /// Recorded copy of [`INFLATION_PRESSURE_CEILING_PA`].
    pub inflation_pressure_ceiling_pa: f64,
    /// Recorded copy of [`CARRIAGE_SPEED_FLOOR_M_S`].
    pub carriage_speed_floor_m_s: f64,
    /// Recorded copy of [`CARRIAGE_SPEED_CEILING_M_S`].
    pub carriage_speed_ceiling_m_s: f64,
    /// Recorded copy of [`TRACK_LENGTH_M`].
    pub track_length_m: f64,
    /// Recorded copy of [`TRACK_END_ALLOWANCE_PER_STEP_M`].
    pub track_end_allowance_per_step_m: f64,
    /// Recorded copy of [`TRACK_END_ALLOWANCE_EXTRA_STEPS`].
    pub track_end_allowance_extra_steps: f64,
    /// Recorded copy of [`SLIP_SETTLE_S`].
    pub slip_settle_s: f64,
    /// Recorded copy of [`HOLD_CEILING_S`].
    pub hold_ceiling_s: f64,
    /// Recorded copy of [`HOLD_END_ALLOWANCE_PER_STEP_S`].
    pub hold_end_allowance_per_step_s: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`GATE_OVER_FLOOR`].
    pub gate_over_floor: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
}

impl TireEveR19Ice {
    /// Card: the EVE 235/55 R19 tyre on the rig's ice. The constructor takes
    /// no arguments.
    #[must_use]
    pub fn on_rig_ice() -> Self {
        let tire = Self {
            mu_traction: MU_TRACTION,
            mu_braking: MU_BRAKING,
            slip_exponent: SLIP_EXPONENT,
            slip_reference: SLIP_REFERENCE,
            slip_floor: SLIP_FLOOR,
            slip_ceiling: SLIP_CEILING,
            normal_load_floor_n: NORMAL_LOAD_FLOOR_N,
            normal_load_ceiling_n: NORMAL_LOAD_CEILING_N,
            normal_load_fitted_ceiling_n: NORMAL_LOAD_FITTED_CEILING_N,
            inflation_pressure_floor_pa: INFLATION_PRESSURE_FLOOR_PA,
            inflation_pressure_ceiling_pa: INFLATION_PRESSURE_CEILING_PA,
            carriage_speed_floor_m_s: CARRIAGE_SPEED_FLOOR_M_S,
            carriage_speed_ceiling_m_s: CARRIAGE_SPEED_CEILING_M_S,
            track_length_m: TRACK_LENGTH_M,
            track_end_allowance_per_step_m: TRACK_END_ALLOWANCE_PER_STEP_M,
            track_end_allowance_extra_steps: TRACK_END_ALLOWANCE_EXTRA_STEPS,
            slip_settle_s: SLIP_SETTLE_S,
            hold_ceiling_s: HOLD_CEILING_S,
            hold_end_allowance_per_step_s: HOLD_END_ALLOWANCE_PER_STEP_S,
            gate_dt_s: GATE_DT_S,
            gate_over_floor: GATE_OVER_FLOOR,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if tire.is_valid() {
            tire
        } else {
            ::core::panic!("tyre failed is_valid");
        }
    }

    /// Every recorded copy equals its crate constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.mu_traction.to_bits() == MU_TRACTION.to_bits()
            && self.mu_braking.to_bits() == MU_BRAKING.to_bits()
            && self.slip_exponent.to_bits() == SLIP_EXPONENT.to_bits()
            && self.slip_reference.to_bits() == SLIP_REFERENCE.to_bits()
            && self.slip_floor.to_bits() == SLIP_FLOOR.to_bits()
            && self.slip_ceiling.to_bits() == SLIP_CEILING.to_bits()
            && self.normal_load_floor_n.to_bits() == NORMAL_LOAD_FLOOR_N.to_bits()
            && self.normal_load_ceiling_n.to_bits() == NORMAL_LOAD_CEILING_N.to_bits()
            && self.normal_load_fitted_ceiling_n.to_bits() == NORMAL_LOAD_FITTED_CEILING_N.to_bits()
            && self.inflation_pressure_floor_pa.to_bits() == INFLATION_PRESSURE_FLOOR_PA.to_bits()
            && self.inflation_pressure_ceiling_pa.to_bits()
                == INFLATION_PRESSURE_CEILING_PA.to_bits()
            && self.carriage_speed_floor_m_s.to_bits() == CARRIAGE_SPEED_FLOOR_M_S.to_bits()
            && self.carriage_speed_ceiling_m_s.to_bits() == CARRIAGE_SPEED_CEILING_M_S.to_bits()
            && self.track_length_m.to_bits() == TRACK_LENGTH_M.to_bits()
            && self.track_end_allowance_per_step_m.to_bits()
                == TRACK_END_ALLOWANCE_PER_STEP_M.to_bits()
            && self.track_end_allowance_extra_steps.to_bits()
                == TRACK_END_ALLOWANCE_EXTRA_STEPS.to_bits()
            && self.slip_settle_s.to_bits() == SLIP_SETTLE_S.to_bits()
            && self.hold_ceiling_s.to_bits() == HOLD_CEILING_S.to_bits()
            && self.hold_end_allowance_per_step_s.to_bits()
                == HOLD_END_ALLOWANCE_PER_STEP_S.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
    }

    /// The friction ratio `|F_x| / F_z` at a slip inside the card:
    /// `MU_mode · (|s| / 0.10)^SLIP_EXPONENT`, the traction constant for a
    /// positive slip and the braking constant for a negative one.
    #[must_use]
    pub fn friction_ratio(&self, slip_ratio: f64) -> f64 {
        let level = if slip_ratio > 0.0 {
            self.mu_traction
        } else {
            self.mu_braking
        };
        level * raised(slip_ratio.abs() / self.slip_reference, self.slip_exponent)
    }

    /// Longitudinal force of the ice on the tyre, N: positive pulls the
    /// carriage forward.
    #[must_use]
    pub fn longitudinal_force_n(&self, normal_load_n: f64, slip_ratio: f64) -> f64 {
        let size = self.friction_ratio(slip_ratio) * normal_load_n;
        if slip_ratio > 0.0 { size } else { -size }
    }

    /// `W_scale`: the heaviest load at the highest friction ratio of the card
    /// over the whole track, J. About 10 332 J.
    #[must_use]
    pub fn ledger_scale_j(&self) -> f64 {
        self.normal_load_ceiling_n * self.friction_ratio(-self.slip_floor) * self.track_length_m
    }

    /// How far past [`TRACK_LENGTH_M`] the running sum of the distance may
    /// read at the end of a step of `dt_s` at this carriage speed, m:
    /// [`TRACK_END_ALLOWANCE_PER_STEP_M`] for each step the whole track can
    /// hold at the step floor and the slowest speed, and
    /// [`TRACK_END_ALLOWANCE_EXTRA_STEPS`] more. It does not depend on the
    /// step, so every route to the same end, restarts included, meets the
    /// same allowance, about 5.5e-11 m. Never more than half the step; zero
    /// for a step or a speed that is not positive.
    #[must_use]
    pub fn track_end_allowance_m(&self, carriage_speed_m_s: f64, dt_s: f64) -> f64 {
        let half_step_m = 0.5 * carriage_speed_m_s * dt_s;
        let shortest_step_m = match self.floor_dt_s() {
            Some(floor_dt_s) => self.carriage_speed_floor_m_s * floor_dt_s,
            None => return 0.0,
        };
        if !(shortest_step_m > 0.0 && half_step_m > 0.0) {
            return 0.0;
        }
        let steps = self.track_length_m / shortest_step_m + self.track_end_allowance_extra_steps;
        let allowance_m = steps * self.track_end_allowance_per_step_m;
        if allowance_m < half_step_m { allowance_m } else { half_step_m }
    }

    /// How far past [`HOLD_CEILING_S`] the running sum of the time held at
    /// one slip may read at the end of a step, s:
    /// [`HOLD_END_ALLOWANCE_PER_STEP_S`] for each of the steps of the floor
    /// that fit in the ceiling, 2^14, and [`TRACK_END_ALLOWANCE_EXTRA_STEPS`]
    /// more, about 2.9e-11 s at every step. Never more than half the step;
    /// zero for a step that is not positive.
    #[must_use]
    pub fn hold_end_allowance_s(&self, dt_s: f64) -> f64 {
        let half_step_s = 0.5 * dt_s;
        if !(half_step_s > 0.0) {
            return 0.0;
        }
        let steps = self.gate_over_floor + self.track_end_allowance_extra_steps;
        let allowance_s = steps * self.hold_end_allowance_per_step_s;
        if allowance_s < half_step_s { allowance_s } else { half_step_s }
    }

    /// Largest step the card stands behind: [`GATE_DT_S`], 16 s, the time
    /// from 5 s to 21 s at one slip. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.gate_dt_s.is_finite() && self.gate_dt_s > 0.0 {
            Some(self.gate_dt_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^14, 2^-10 s. Not applied by [`step`].
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

/// The rolling tyre. The ledger fields are part of the state; write them only
/// through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TireState {
    /// Time since the start of the run.
    pub t_s: f64,
    /// Distance of the wheel centre along the track.
    pub distance_m: f64,
    /// Length of tread that has passed the contact since the start of the run.
    pub rim_travel_m: f64,
    /// Slip held over the last step. Zero before the first step of a run.
    pub slip_ratio: f64,
    /// Time the present slip has been held, s. At the start it is the time
    /// stated for the slip of the first step; it grows by `dt` while the slip
    /// is held and starts again at zero when the slip changes.
    pub slip_held_s: f64,
    /// Longitudinal force of the ice on the tyre over the last step, `F_x`.
    pub longitudinal_force_n: f64,
    /// `W`, the work of the hub on the tyre since the start of the run.
    pub wheel_work_j: f64,
    /// `D`, the work of the tyre on the carriage since the start of the run.
    pub drawbar_work_j: f64,
    /// `H`, the heat of sliding since the start of the run. Never falls.
    pub slip_heat_j: f64,
    /// Signed `D + H − W`. Positive is created work.
    pub energy_defect_j: f64,
    /// `|energy_defect_j| / W_scale`.
    pub residual_conservation_error: f64,
    /// The normal load of the last step is above [`NORMAL_LOAD_FITTED_CEILING_N`].
    pub load_above_fitted: bool,
}

impl TireState {
    /// The wheel at this distance along the track, its first slip already
    /// held for `slip_held_s`, time 0, with the ledger empty.
    /// [`StepReject::InvalidTire`] for a changed tyre,
    /// [`StepReject::NonFiniteState`] for a value that is not finite,
    /// [`StepReject::TravelBeyondTrack`] for a distance below zero or beyond
    /// [`TRACK_LENGTH_M`], [`StepReject::SlipNotSettled`] for a time held
    /// below [`SLIP_SETTLE_S`] and [`StepReject::HoldBeyondMeasured`] for one
    /// beyond [`HOLD_CEILING_S`].
    pub fn try_start(
        tire: &TireEveR19Ice,
        distance_m: f64,
        slip_held_s: f64,
    ) -> Result<Self, StepReject> {
        if !tire.is_valid() {
            return Err(StepReject::InvalidTire);
        }
        if !(distance_m.is_finite() && slip_held_s.is_finite()) {
            return Err(StepReject::NonFiniteState);
        }
        if !(distance_m >= 0.0 && distance_m <= tire.track_length_m) {
            return Err(StepReject::TravelBeyondTrack);
        }
        if !(slip_held_s >= tire.slip_settle_s) {
            return Err(StepReject::SlipNotSettled);
        }
        if !(slip_held_s <= tire.hold_ceiling_s) {
            return Err(StepReject::HoldBeyondMeasured);
        }
        Ok(Self {
            t_s: 0.0,
            distance_m,
            rim_travel_m: 0.0,
            slip_ratio: 0.0,
            slip_held_s,
            longitudinal_force_n: 0.0,
            wheel_work_j: 0.0,
            drawbar_work_j: 0.0,
            slip_heat_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            load_above_fitted: false,
        })
    }
}

/// What one step adds to the state: the force, the two lengths and the three
/// works, each worked out on its own.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Advance {
    longitudinal_force_n: f64,
    distance_m: f64,
    rim_travel_m: f64,
    wheel_work_j: f64,
    drawbar_work_j: f64,
    slip_heat_j: f64,
}

/// Hold the load for one step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    tire: &TireEveR19Ice,
    state: &mut TireState,
    frame: &mut LastStateFrame64,
    load: TireLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(tire, state, load, dt_s)?;
    let advance = held(tire, load, dt_s);
    commit(tire, state, frame, load, dt_s, advance)
}

fn validate(
    tire: &TireEveR19Ice,
    state: &TireState,
    load: TireLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !(dt_s.is_finite()
        && load.normal_load_n.is_finite()
        && load.inflation_pressure_pa.is_finite()
        && load.slip_ratio.is_finite()
        && load.carriage_speed_m_s.is_finite())
    {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !tire.is_valid() {
        return Err(StepReject::InvalidTire);
    }
    if !(load.normal_load_n >= tire.normal_load_floor_n
        && load.normal_load_n <= tire.normal_load_ceiling_n)
    {
        return Err(StepReject::NormalLoadOutsideCard);
    }
    if !(load.inflation_pressure_pa >= tire.inflation_pressure_floor_pa
        && load.inflation_pressure_pa <= tire.inflation_pressure_ceiling_pa)
    {
        return Err(StepReject::InflationPressureOutsideCard);
    }
    let slip_size = load.slip_ratio.abs();
    if !(slip_size >= tire.slip_floor && slip_size <= tire.slip_ceiling) {
        return Err(StepReject::SlipOutsideCard);
    }
    if !(load.carriage_speed_m_s >= tire.carriage_speed_floor_m_s
        && load.carriage_speed_m_s <= tire.carriage_speed_ceiling_m_s)
    {
        return Err(StepReject::CarriageSpeedOutsideCard);
    }
    let held_before_s = held_before_step_s(state, load);
    if !(held_before_s >= tire.slip_settle_s) {
        return Err(StepReject::SlipNotSettled);
    }
    if !(held_before_s + dt_s <= tire.hold_ceiling_s + tire.hold_end_allowance_s(dt_s)) {
        return Err(StepReject::HoldBeyondMeasured);
    }
    if !(state.distance_m >= 0.0
        && state.distance_m + load.carriage_speed_m_s * dt_s
            <= tire.track_length_m + tire.track_end_allowance_m(load.carriage_speed_m_s, dt_s))
    {
        return Err(StepReject::TravelBeyondTrack);
    }
    Ok(())
}

/// The time the slip of this step has been held when the step starts: the
/// state's time held if the slip is the one held over the last step, or if no
/// step has been taken (the slip of the state is zero, which no step holds);
/// zero if the slip changes here.
fn held_before_step_s(state: &TireState, load: TireLoad) -> f64 {
    if state.slip_ratio == 0.0 || state.slip_ratio == load.slip_ratio {
        state.slip_held_s
    } else {
        0.0
    }
}

/// The force and the increments of one step with the load held. The three
/// speeds are each formed from the carriage speed and the slip on their own,
/// and none is the sum or the difference of the other two.
fn held(tire: &TireEveR19Ice, load: TireLoad, dt_s: f64) -> Advance {
    let force_n = tire.longitudinal_force_n(load.normal_load_n, load.slip_ratio);
    let carriage_m_s = load.carriage_speed_m_s;
    let rim_m_s = carriage_m_s / (1.0 - load.slip_ratio);
    let slide_m_s = carriage_m_s * load.slip_ratio / (1.0 - load.slip_ratio);
    Advance {
        longitudinal_force_n: force_n,
        distance_m: carriage_m_s * dt_s,
        rim_travel_m: rim_m_s * dt_s,
        wheel_work_j: force_n * rim_m_s * dt_s,
        drawbar_work_j: force_n * carriage_m_s * dt_s,
        slip_heat_j: force_n.abs() * slide_m_s.abs() * dt_s,
    }
}

fn commit(
    tire: &TireEveR19Ice,
    state: &mut TireState,
    frame: &mut LastStateFrame64,
    load: TireLoad,
    dt_s: f64,
    advance: Advance,
) -> Result<(), StepReject> {
    let saved = *state;
    let held_before_s = held_before_step_s(state, load);
    state.t_s += dt_s;
    state.distance_m += advance.distance_m;
    state.rim_travel_m += advance.rim_travel_m;
    state.slip_ratio = load.slip_ratio;
    state.slip_held_s = held_before_s + dt_s;
    state.longitudinal_force_n = advance.longitudinal_force_n;
    state.wheel_work_j += advance.wheel_work_j;
    state.drawbar_work_j += advance.drawbar_work_j;
    state.slip_heat_j += advance.slip_heat_j;
    state.load_above_fitted = load.normal_load_n > tire.normal_load_fitted_ceiling_n;
    let scale = tire.ledger_scale_j();
    if !(scale.is_finite() && scale > 0.0) {
        *state = saved;
        return Err(StepReject::InvalidTire);
    }
    state.energy_defect_j = state.drawbar_work_j + state.slip_heat_j - state.wheel_work_j;
    state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    let gate = tire.residual_conservation_limit * scale;
    if state.energy_defect_j > gate {
        *state = saved;
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j < -gate {
        *state = saved;
        return Err(StepReject::FirstLawBreach);
    }
    if !(state.distance_m >= 0.0
        && state.distance_m
            <= tire.track_length_m + tire.track_end_allowance_m(load.carriage_speed_m_s, dt_s))
    {
        *state = saved;
        return Err(StepReject::TravelBeyondTrack);
    }
    if !(held_before_s >= tire.slip_settle_s
        && state.slip_held_s <= tire.hold_ceiling_s + tire.hold_end_allowance_s(dt_s))
    {
        *state = saved;
        return Err(if held_before_s >= tire.slip_settle_s {
            StepReject::HoldBeyondMeasured
        } else {
            StepReject::SlipNotSettled
        });
    }
    publish(state, frame);
    Ok(())
}

fn publish(state: &TireState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.distance_m as f32;
    frame.pos[1] = state.rim_travel_m as f32;
    frame.pos[2] = state.slip_ratio as f32;
    frame.vel[0] = state.wheel_work_j as f32;
    frame.vel[1] = state.drawbar_work_j as f32;
    frame.vel[2] = state.slip_heat_j as f32;
    frame.force_torque = state.longitudinal_force_n as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.load_above_fitted {
        FLAG_LOAD_ABOVE_FITTED
    } else {
        0
    };
}

fn state_is_finite(state: &TireState) -> bool {
    state.t_s.is_finite()
        && state.distance_m.is_finite()
        && state.rim_travel_m.is_finite()
        && state.slip_ratio.is_finite()
        && state.slip_held_s.is_finite()
        && state.longitudinal_force_n.is_finite()
        && state.wheel_work_j.is_finite()
        && state.drawbar_work_j.is_finite()
        && state.slip_heat_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        Advance, CARRIAGE_SPEED_CEILING_M_S, CARRIAGE_SPEED_FLOOR_M_S, FLAG_LOAD_ABOVE_FITTED,
        GATE_DT_S, HOLD_CEILING_S, HOLD_END_ALLOWANCE_PER_STEP_S, INFLATION_PRESSURE_CEILING_PA,
        INFLATION_PRESSURE_FLOOR_PA, MU_BRAKING, MU_TRACTION, NORMAL_LOAD_CEILING_N,
        NORMAL_LOAD_FLOOR_N, RESIDUAL_CONSERVATION_LIMIT, SLIP_CEILING, SLIP_EXPONENT, SLIP_FLOOR,
        SLIP_SETTLE_S, StepReject, TRACK_END_ALLOWANCE_EXTRA_STEPS,
        TRACK_END_ALLOWANCE_PER_STEP_M, TRACK_LENGTH_M, TireEveR19Ice, TireLoad, TireState,
        commit, held, step, validate,
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

    /// The rig's own condition at the middle of the fitted set.
    fn rig(slip_ratio: f64) -> TireLoad {
        TireLoad {
            normal_load_n: 5_000.0,
            inflation_pressure_pa: 250_000.0,
            slip_ratio,
            carriage_speed_m_s: 0.06,
        }
    }

    /// A start at this distance with the slip settled: held for 5 s.
    fn start(tire: &TireEveR19Ice, distance_m: f64) -> TireState {
        TireState::try_start(tire, distance_m, SLIP_SETTLE_S).unwrap()
    }

    fn relative(a: f64, b: f64) -> f64 {
        ((a - b) / b).abs()
    }

    /// The constants are the frozen fit of the research record
    /// (`analysis/out/fit.json`), to six significant figures.
    #[test]
    fn the_constants_are_the_frozen_fit() {
        assert_eq!(MU_TRACTION, 0.228109);
        assert_eq!(MU_BRAKING, 0.24162);
        assert_eq!(SLIP_EXPONENT, -0.132823);
        assert!(relative(MU_TRACTION, 0.2281086164622459) < 5.0e-6);
        assert!(relative(MU_BRAKING, 0.2416198208301724) < 5.0e-6);
        assert!(relative(SLIP_EXPONENT, -0.1328230763666814) < 5.0e-6);
        let tire = TireEveR19Ice::on_rig_ice();
        assert!(tire.is_valid());
        assert_eq!((SLIP_SETTLE_S, HOLD_CEILING_S), (5.0, 21.0));
        assert_eq!(tire.gate_dt_s(), Some(HOLD_CEILING_S - SLIP_SETTLE_S));
        assert_eq!(tire.floor_dt_s(), Some(16.0 / 16_384.0));
        assert_eq!(tire.floor_dt_s(), Some(1.0 / 1_024.0));
    }

    /// The friction ratio against values worked outside Rust with Python
    /// mpmath at 50 digits from the same three constants as 64-bit numbers.
    /// The crate's exponential and logarithm agree with them to 4e-16.
    #[test]
    fn the_friction_ratio_matches_fifty_digit_arithmetic() {
        let tire = TireEveR19Ice::on_rig_ice();
        for (slip, reference) in [
            (0.05, 0.250_107_161_708_704_59),
            (0.07, 0.239_175_650_414_643_27),
            (0.10, 0.228_109),
            (0.15, 0.216_149_109_836_226_93),
            (0.20, 0.208_045_685_399_455_92),
            (-0.05, 0.264_921_122_849_414_98),
            (-0.12, 0.235_839_089_412_292_54),
            (-0.20, 0.220_368_326_134_508_23),
        ] {
            let got = tire.friction_ratio(slip);
            assert!(relative(got, reference) < 1.0e-14, "{slip}: {got} against {reference}");
        }
        assert_eq!(tire.longitudinal_force_n(5_000.0, 0.10), 0.228109 * 5_000.0);
        assert_eq!(tire.longitudinal_force_n(5_000.0, -0.10), -(0.24162 * 5_000.0));
    }

    /// One hold of traction at 12 percent from 5 s to 21 s: the force pulls
    /// forward, the hub does the work, and the three works close.
    #[test]
    fn a_traction_hold_closes_its_ledger() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut state = start(&tire, 0.0);
        let mut frame = blank();
        step(&tire, &mut state, &mut frame, rig(0.12), GATE_DT_S).unwrap();
        let force = tire.friction_ratio(0.12) * 5_000.0;
        assert_eq!(state.longitudinal_force_n, force);
        assert!(force > 1_100.0 && force < 1_120.0);
        assert!(relative(state.distance_m, 0.96) < 1.0e-15);
        assert!(relative(state.rim_travel_m, 0.96 / 0.88) < 1.0e-15);
        assert!(relative(state.drawbar_work_j, force * 0.96) < 1.0e-15);
        assert!(relative(state.wheel_work_j, force * 0.96 / 0.88) < 1.0e-15);
        assert!(relative(state.slip_heat_j, force * 0.96 * 0.12 / 0.88) < 1.0e-15);
        assert!(state.wheel_work_j > state.drawbar_work_j);
        assert!(state.residual_conservation_error < 1.0e-15);
        assert_eq!(state.slip_held_s, HOLD_CEILING_S);
        assert_eq!(frame.t, 16.0);
        assert_eq!(frame.force_torque, force as f32);
        assert_eq!(frame.pos[2], 0.12_f32);
        assert_eq!(frame.flags, 0);
        assert_eq!(frame.proof, PROOF);
    }

    /// Braking: the force holds the carriage back, the carriage does the
    /// work, the hub takes work out, and the heat is still positive.
    #[test]
    fn a_braking_hold_takes_work_from_the_carriage() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut state = start(&tire, 0.0);
        let mut frame = blank();
        step(&tire, &mut state, &mut frame, rig(-0.20), GATE_DT_S).unwrap();
        assert!(state.longitudinal_force_n < 0.0);
        assert!(state.drawbar_work_j < 0.0);
        assert!(state.wheel_work_j < 0.0);
        assert!(state.wheel_work_j > state.drawbar_work_j);
        assert!(state.slip_heat_j > 0.0);
        assert!(relative(state.rim_travel_m, 0.96 / 1.2) < 1.0e-15);
        assert!(
            relative(state.slip_heat_j, state.wheel_work_j - state.drawbar_work_j) < 1.0e-13
        );
        assert!(state.residual_conservation_error < 1.0e-15);
    }

    /// The force is the same whatever the step, and the works of many small
    /// steps are the works of one large one.
    #[test]
    fn small_steps_and_one_step_agree() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut one = start(&tire, 0.5);
        let mut many = one;
        let mut frame = blank();
        step(&tire, &mut one, &mut frame, rig(0.07), 16.0).unwrap();
        let mut count = 0;
        while count < 8_192 {
            step(&tire, &mut many, &mut frame, rig(0.07), 16.0 / 8_192.0).unwrap();
            count += 1;
        }
        assert_eq!(one.longitudinal_force_n, many.longitudinal_force_n);
        assert!(relative(many.distance_m, one.distance_m) < 1.0e-12);
        assert!(relative(many.wheel_work_j, one.wheel_work_j) < 1.0e-12);
        assert!(relative(many.slip_heat_j, one.slip_heat_j) < 1.0e-12);
        assert_eq!(many.slip_held_s, one.slip_held_s);
        assert!(many.residual_conservation_error < 1.0e-13);
    }

    /// A load above the fitted loads lands and carries the flag; a load at
    /// the fitted ceiling does not. A change of load at the same slip keeps
    /// the time held.
    #[test]
    fn a_load_above_the_fitted_loads_carries_the_flag() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut state = start(&tire, 0.0);
        let mut frame = blank();
        let mut load = rig(0.10);
        step(&tire, &mut state, &mut frame, load, 1.0).unwrap();
        assert_eq!(frame.flags, 0);
        assert!(!state.load_above_fitted);
        load.normal_load_n = 6_500.0;
        step(&tire, &mut state, &mut frame, load, 1.0).unwrap();
        assert_eq!(frame.flags, FLAG_LOAD_ABOVE_FITTED);
        assert!(state.load_above_fitted);
        assert_eq!(state.longitudinal_force_n, 0.228109 * 6_500.0);
        assert_eq!(state.slip_held_s, 7.0);
    }

    /// Every limit is named, and a refused step leaves the state and the
    /// frame, proof included, exactly as they were.
    #[test]
    fn each_limit_is_named_and_nothing_moves() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut state = start(&tire, 1.0);
        let mut frame = blank();
        step(&tire, &mut state, &mut frame, rig(0.10), 2.0).unwrap();
        let kept_state = state;
        let kept_frame = frame.to_bytes();
        let base = rig(0.10);
        let nan = f64::NAN;
        let cases: [(TireLoad, f64, StepReject); 24] = [
            (TireLoad { normal_load_n: nan, ..base }, 1.0, StepReject::NonFiniteInput),
            (TireLoad { inflation_pressure_pa: f64::INFINITY, ..base }, 1.0, StepReject::NonFiniteInput),
            (TireLoad { slip_ratio: nan, ..base }, 1.0, StepReject::NonFiniteInput),
            (TireLoad { carriage_speed_m_s: f64::NEG_INFINITY, ..base }, 1.0, StepReject::NonFiniteInput),
            (base, nan, StepReject::NonFiniteInput),
            (base, 0.0, StepReject::NonPositiveDt),
            (base, -1.0, StepReject::NonPositiveDt),
            (TireLoad { normal_load_n: 3_499.999, ..base }, 1.0, StepReject::NormalLoadOutsideCard),
            (TireLoad { normal_load_n: 6_500.001, ..base }, 1.0, StepReject::NormalLoadOutsideCard),
            (TireLoad { normal_load_n: -5_000.0, ..base }, 1.0, StepReject::NormalLoadOutsideCard),
            (TireLoad { inflation_pressure_pa: 149_999.0, ..base }, 1.0, StepReject::InflationPressureOutsideCard),
            (TireLoad { inflation_pressure_pa: 350_001.0, ..base }, 1.0, StepReject::InflationPressureOutsideCard),
            (TireLoad { slip_ratio: 0.0, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: 0.049_999, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: -0.03, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: 0.200_001, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: -1.0, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: 1.0, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { carriage_speed_m_s: 0.049, ..base }, 1.0, StepReject::CarriageSpeedOutsideCard),
            (TireLoad { carriage_speed_m_s: 0.071, ..base }, 1.0, StepReject::CarriageSpeedOutsideCard),
            (TireLoad { carriage_speed_m_s: 10.0, ..base }, 1.0, StepReject::CarriageSpeedOutsideCard),
            // a change of slip starts the hold again at zero
            (TireLoad { slip_ratio: 0.12, ..base }, 1.0, StepReject::SlipNotSettled),
            (TireLoad { slip_ratio: -0.10, ..base }, 1.0, StepReject::SlipNotSettled),
            // 7 s held and 15 s more is 22 s
            (base, 15.0, StepReject::HoldBeyondMeasured),
        ];
        for (load, dt_s, reject) in cases {
            assert_eq!(step(&tire, &mut state, &mut frame, load, dt_s), Err(reject));
            assert_eq!(state, kept_state);
            assert_eq!(frame.to_bytes(), kept_frame);
        }
        // The ends of every range are inside the card.
        for load in [
            TireLoad { normal_load_n: NORMAL_LOAD_FLOOR_N, ..base },
            TireLoad { normal_load_n: NORMAL_LOAD_CEILING_N, ..base },
            TireLoad { inflation_pressure_pa: INFLATION_PRESSURE_FLOOR_PA, ..base },
            TireLoad { inflation_pressure_pa: INFLATION_PRESSURE_CEILING_PA, ..base },
            TireLoad { carriage_speed_m_s: CARRIAGE_SPEED_FLOOR_M_S, ..base },
            TireLoad { carriage_speed_m_s: CARRIAGE_SPEED_CEILING_M_S, ..base },
        ] {
            assert_eq!(validate(&tire, &state, load, 1.0), Ok(()));
        }
        for slip in [SLIP_FLOOR, -SLIP_FLOOR, SLIP_CEILING, -SLIP_CEILING] {
            let fresh = start(&tire, 1.0);
            assert_eq!(validate(&tire, &fresh, rig(slip), 1.0), Ok(()));
        }
    }

    /// The time at one slip runs from 5 s to 21 s. A start states it, a run
    /// carries it, a change of slip starts it again at zero, and a restart
    /// that hands back the time it was given cannot go past 21 s.
    #[test]
    fn the_time_at_one_slip_is_held_from_5_to_21_seconds() {
        let tire = TireEveR19Ice::on_rig_ice();
        let below = f64::from_bits(SLIP_SETTLE_S.to_bits() - 1);
        let above = f64::from_bits(HOLD_CEILING_S.to_bits() + 1);
        assert_eq!(TireState::try_start(&tire, 0.0, 0.0), Err(StepReject::SlipNotSettled));
        assert_eq!(TireState::try_start(&tire, 0.0, below), Err(StepReject::SlipNotSettled));
        assert_eq!(TireState::try_start(&tire, 0.0, above), Err(StepReject::HoldBeyondMeasured));
        assert_eq!(TireState::try_start(&tire, 0.0, f64::NAN), Err(StepReject::NonFiniteState));
        // one step of 16 s from 5 s ends exactly on 21 s and lands
        let mut one = start(&tire, 0.0);
        let mut frame = blank();
        step(&tire, &mut one, &mut frame, rig(0.15), 16.0).unwrap();
        assert_eq!(one.slip_held_s, 21.0);
        let kept = one;
        assert_eq!(
            step(&tire, &mut one, &mut frame, rig(0.15), tire.floor_dt_s().unwrap()),
            Err(StepReject::HoldBeyondMeasured)
        );
        assert_eq!(one, kept);
        // 1,600 steps of 0.01 s read 21.000000000000547 s and land every step
        let mut many = start(&tire, 0.0);
        for landed in 0..1_600 {
            assert_eq!(step(&tire, &mut many, &mut frame, rig(0.15), 0.01), Ok(()), "{landed}");
        }
        assert!(many.slip_held_s > 21.0);
        assert!(many.slip_held_s - 21.0 <= tire.hold_end_allowance_s(0.01));
        assert_eq!(
            step(&tire, &mut many, &mut frame, rig(0.15), 0.01),
            Err(StepReject::HoldBeyondMeasured)
        );
        // the floor 16,384 times is exactly 16 s
        let mut finest = start(&tire, 0.0);
        let floor = tire.floor_dt_s().unwrap();
        for _ in 0..16_384 {
            step(&tire, &mut finest, &mut frame, rig(0.15), floor).unwrap();
        }
        assert_eq!(finest.slip_held_s, 21.0);
        assert_eq!(finest.longitudinal_force_n, kept.longitudinal_force_n);
        assert!(relative(finest.distance_m, kept.distance_m) < 1.0e-12);
        // a start at 21 s is a start, and no step from it lands
        let mut at_end = TireState::try_start(&tire, 0.0, 21.0).unwrap();
        assert_eq!(
            step(&tire, &mut at_end, &mut frame, rig(0.15), floor),
            Err(StepReject::HoldBeyondMeasured)
        );
        // a restart from the returned state carries the time held
        let mut first = start(&tire, 0.0);
        step(&tire, &mut first, &mut frame, rig(0.15), 10.0).unwrap();
        let mut again = TireState::try_start(&tire, first.distance_m, first.slip_held_s).unwrap();
        assert_eq!(
            step(&tire, &mut again, &mut frame, rig(0.15), 7.0),
            Err(StepReject::HoldBeyondMeasured)
        );
        step(&tire, &mut again, &mut frame, rig(0.15), 6.0).unwrap();
        assert_eq!(again.slip_held_s, 21.0);
        // the first step after a change of slip starts at zero and is refused
        let mut changed = start(&tire, 0.0);
        step(&tire, &mut changed, &mut frame, rig(0.15), 1.0).unwrap();
        let kept = changed;
        let held = frame.to_bytes();
        assert_eq!(
            step(&tire, &mut changed, &mut frame, rig(0.12), floor),
            Err(StepReject::SlipNotSettled)
        );
        assert_eq!(changed, kept);
        assert_eq!(frame.to_bytes(), held);
        // the allowance is its derivation and a changed copy is not this tyre
        let spacing_s = f64::from_bits(16.0_f64.to_bits() + 1) - 16.0;
        assert_eq!(HOLD_END_ALLOWANCE_PER_STEP_S, 0.5 * spacing_s);
        assert_eq!(
            tire.hold_end_allowance_s(1.0),
            (16_384.0 + 6.0) * HOLD_END_ALLOWANCE_PER_STEP_S
        );
        assert_eq!(tire.hold_end_allowance_s(1.0), tire.hold_end_allowance_s(floor));
        assert_eq!(tire.hold_end_allowance_s(1.0e-12), 0.5e-12);
        assert_eq!(tire.hold_end_allowance_s(0.0), 0.0);
        for altered in [
            TireEveR19Ice { slip_settle_s: 0.0, ..tire },
            TireEveR19Ice { hold_ceiling_s: 100.0, ..tire },
            TireEveR19Ice { hold_end_allowance_per_step_s: 1.0, ..tire },
        ] {
            assert!(!altered.is_valid());
            assert_eq!(TireState::try_start(&altered, 0.0, 5.0), Err(StepReject::InvalidTire));
        }
    }

    /// The track ends at six metres: a step that would end beyond it is
    /// refused whole, a start off the track is no start, and restarts from
    /// returned distances roll on to the end.
    #[test]
    fn the_track_has_an_end() {
        let tire = TireEveR19Ice::on_rig_ice();
        assert_eq!(TireState::try_start(&tire, -0.001, 5.0), Err(StepReject::TravelBeyondTrack));
        assert_eq!(TireState::try_start(&tire, 6.001, 5.0), Err(StepReject::TravelBeyondTrack));
        assert_eq!(TireState::try_start(&tire, f64::NAN, 5.0), Err(StepReject::NonFiniteState));
        let mut state = start(&tire, 0.0);
        let mut frame = blank();
        let mut landed = 0;
        loop {
            match step(&tire, &mut state, &mut frame, rig(0.10), GATE_DT_S) {
                Ok(()) => {
                    landed += 1;
                    state = start(&tire, state.distance_m);
                }
                Err(reject) => {
                    assert_eq!(reject, StepReject::TravelBeyondTrack);
                    break;
                }
            }
        }
        assert_eq!(landed, 6);
        assert!(state.distance_m <= TRACK_LENGTH_M);
        assert!(relative(state.distance_m, 5.76) < 1.0e-14);
        let mut off = state;
        off.distance_m = -0.5;
        assert_eq!(
            step(&tire, &mut off, &mut frame, rig(0.10), 1.0),
            Err(StepReject::TravelBeyondTrack)
        );
    }

    /// The allowance is what its derivation says: 2^-51 m a step, the track
    /// over the shortest step at the floor and six more, the same at every
    /// step, and never half a step or more.
    #[test]
    fn the_track_end_allowance_is_its_derivation() {
        let tire = TireEveR19Ice::on_rig_ice();
        // half the spacing of 64-bit numbers at 6 m
        let spacing_m = f64::from_bits(TRACK_LENGTH_M.to_bits() + 1) - TRACK_LENGTH_M;
        assert_eq!(TRACK_END_ALLOWANCE_PER_STEP_M, 0.5 * spacing_m);
        assert_eq!(TRACK_END_ALLOWANCE_EXTRA_STEPS, 6.0);
        let floor_dt_s = tire.floor_dt_s().unwrap();
        let at_floor = tire.track_end_allowance_m(CARRIAGE_SPEED_FLOOR_M_S, floor_dt_s);
        assert_eq!(
            at_floor,
            (6.0 / (0.05 * floor_dt_s) + 6.0) * TRACK_END_ALLOWANCE_PER_STEP_M
        );
        assert!(at_floor > 5.4e-11 && at_floor < 5.5e-11);
        // the same at every step and speed inside the card
        for (speed_m_s, dt_s) in [(0.06, GATE_DT_S), (0.07, 0.1), (0.05, 0.01), (0.06, 1.0)] {
            assert_eq!(tire.track_end_allowance_m(speed_m_s, dt_s), at_floor);
        }
        // a hundred-thousandth of the shortest step the gateway lets through
        assert!(at_floor < 1.0e-5 * CARRIAGE_SPEED_FLOOR_M_S * floor_dt_s);
        // under the floor it is held to half the step
        assert_eq!(tire.track_end_allowance_m(0.06, 1.0e-9), 0.5 * 0.06 * 1.0e-9);
        assert_eq!(tire.track_end_allowance_m(0.06, 0.0), 0.0);
        assert_eq!(tire.track_end_allowance_m(0.0, 1.0), 0.0);
        let mut altered = tire;
        altered.track_end_allowance_per_step_m = 1.0e-9;
        assert!(!altered.is_valid());
        let mut altered = tire;
        altered.track_end_allowance_extra_steps = 7.0;
        assert!(!altered.is_valid());
    }

    /// A distance returned by a run of small steps, handed back as a new
    /// start, carries their rounding. In 0.6.0 one step to the end of the
    /// track from it was refused while small steps over the same span
    /// landed. Both routes now land, at the same end to rounding.
    #[test]
    fn a_restart_from_a_returned_distance_lands_where_small_steps_land() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut frame = blank();
        // 8,400 steps of 0.01 s in runs of 16 s, each run a restart
        let mut state = start(&tire, 0.0);
        for count in 0..8_400 {
            if count % 1_600 == 0 {
                state = start(&tire, state.distance_m);
            }
            step(&tire, &mut state, &mut frame, rig(0.12), 0.01).unwrap();
        }
        let returned_m = state.distance_m;
        assert_eq!(returned_m, 5.040000000000356);
        // a rule that scaled the allowance with this step's count would refuse it
        let per_step_rule_m = (6.0 / (0.05 * 16.0) + 6.0) * TRACK_END_ALLOWANCE_PER_STEP_M;
        assert!(returned_m + 0.06 * 16.0 > TRACK_LENGTH_M + per_step_rule_m);
        let mut one = start(&tire, returned_m);
        step(&tire, &mut one, &mut frame, rig(0.12), 16.0).unwrap();
        assert_eq!(one.distance_m, 6.000000000000356);
        let mut many = start(&tire, returned_m);
        for _ in 0..1_600 {
            step(&tire, &mut many, &mut frame, rig(0.12), 0.01).unwrap();
        }
        assert_eq!(many.distance_m, 6.000000000000961);
        assert!((one.distance_m - many.distance_m).abs() < 1.0e-12);
        assert_eq!(one.longitudinal_force_n, many.longitudinal_force_n);
        assert!(relative(many.wheel_work_j, one.wheel_work_j) < 1.0e-12);
        // a step that truly ends beyond 6 m is still refused by name
        let mut beyond = start(&tire, returned_m + 1.0e-9);
        assert_eq!(
            step(&tire, &mut beyond, &mut frame, rig(0.12), 16.0),
            Err(StepReject::TravelBeyondTrack)
        );
    }

    /// Runs whose end in decimal is exactly the track's 6 m land every step,
    /// at twenty step sizes and three speeds, rolled as runs of at most 16 s
    /// each restarted from the distance the last returned; the step after,
    /// from a fresh start, is refused by name.
    #[test]
    fn a_decimal_run_that_ends_on_the_track_end_lands_every_step() {
        let tire = TireEveR19Ice::on_rig_ice();
        // step sizes in units of 0.0001 s
        let steps_e4: [u64; 20] = [
            160_000, 125_000, 100_000, 80_000, 50_000, 25_000, 20_000, 16_000, 10_000, 5_000,
            4_000, 2_500, 2_000, 1_250, 1_000, 500, 400, 200, 100, 50,
        ];
        let mut past_the_end = 0;
        for speed_e2 in [5_u64, 6, 7] {
            for dt_e4 in steps_e4 {
                // lengths in units of 1e-6 m
                let step_e6 = speed_e2 * dt_e4;
                let count = 6_000_000 / step_e6;
                let per_run = 160_000 / dt_e4;
                let start_e6 = 6_000_000 - count * step_e6;
                let decimal = |whole: u64, digits: usize, scale: u64| -> f64 {
                    std::format!("{}.{:0width$}", whole / scale, whole % scale, width = digits)
                        .parse()
                        .unwrap()
                };
                let start_m = decimal(start_e6, 6, 1_000_000);
                let dt_s = decimal(dt_e4, 4, 10_000);
                let speed_m_s = decimal(speed_e2, 2, 100);
                let load = TireLoad { carriage_speed_m_s: speed_m_s, ..rig(0.10) };
                let mut state = start(&tire, start_m);
                let mut frame = blank();
                for landed in 0..count {
                    if landed % per_run == 0 {
                        state = start(&tire, state.distance_m);
                    }
                    assert_eq!(
                        step(&tire, &mut state, &mut frame, load, dt_s),
                        Ok(()),
                        "speed {speed_m_s} dt {dt_s} start {start_m}: step {} of {count}",
                        landed + 1
                    );
                }
                let allowance_m = tire.track_end_allowance_m(speed_m_s, dt_s);
                assert!((state.distance_m - TRACK_LENGTH_M).abs() <= allowance_m);
                if state.distance_m > TRACK_LENGTH_M {
                    past_the_end += 1;
                }
                // a returned distance past 6 m is no start; one on the track
                // starts, and its first step is refused
                match TireState::try_start(&tire, state.distance_m, SLIP_SETTLE_S) {
                    Err(reject) => {
                        assert!(state.distance_m > TRACK_LENGTH_M);
                        assert_eq!(reject, StepReject::TravelBeyondTrack);
                    }
                    Ok(mut fresh) => {
                        let before = fresh;
                        let held = frame.to_bytes();
                        assert_eq!(
                            step(&tire, &mut fresh, &mut frame, load, dt_s),
                            Err(StepReject::TravelBeyondTrack)
                        );
                        assert_eq!(fresh, before);
                        assert_eq!(frame.to_bytes(), held);
                    }
                }
            }
        }
        // some of the sixty runs do read past 6 m: the allowance is in use
        assert!(past_the_end > 0);
    }

    /// The two runs the second scorer found short by a step (V-33), rolled
    /// now as restarts of at most 16 s.
    #[test]
    fn the_two_runs_found_short_by_a_step_land_whole() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut state = start(&tire, 0.0);
        let mut frame = blank();
        for count in 0..999 {
            if count % 160 == 0 {
                state = start(&tire, state.distance_m);
            }
            step(&tire, &mut state, &mut frame, rig(0.10), 0.1).unwrap();
        }
        assert_eq!(state.distance_m, 5.994000000000005);
        step(&tire, &mut state, &mut frame, rig(0.10), 0.1).unwrap();
        let load = TireLoad { carriage_speed_m_s: 0.07, ..rig(0.10) };
        let mut state = start(&tire, 1.52);
        for _ in 0..4 {
            state = start(&tire, state.distance_m);
            step(&tire, &mut state, &mut frame, load, 16.0).unwrap();
        }
        assert_eq!(state.distance_m, 6.000000000000001);
        // handed back, a distance past 6 m is no start
        assert_eq!(
            TireState::try_start(&tire, state.distance_m, SLIP_SETTLE_S),
            Err(StepReject::TravelBeyondTrack)
        );
    }

    /// A step that would end beyond the track by a spacing more than the
    /// allowance is refused by name and nothing moves; every end inside the
    /// allowance lands at its own value. Checked over the last thousand
    /// spacings inside the allowance and the first four beyond it.
    #[test]
    fn a_step_beyond_the_allowance_is_refused_by_name() {
        let tire = TireEveR19Ice::on_rig_ice();
        let spacing_m = f64::from_bits(TRACK_LENGTH_M.to_bits() + 1) - TRACK_LENGTH_M;
        for (speed_m_s, dt_s) in [(0.06, GATE_DT_S), (0.07, 10.0), (0.05, 1.0), (0.06, 0.1)] {
            let load = TireLoad { carriage_speed_m_s: speed_m_s, ..rig(0.10) };
            let allowance_m = tire.track_end_allowance_m(speed_m_s, dt_s);
            let step_m = speed_m_s * dt_s;
            // the comparison is with the 64-bit number nearest the end plus the allowance
            let whole_spacings =
                (((TRACK_LENGTH_M + allowance_m) - TRACK_LENGTH_M) / spacing_m) as u64;
            assert!(whole_spacings > 60_000);
            let mut landed_past_the_end = 0;
            let mut refused = 0;
            let mut ends: std::vec::Vec<u64> = (0..4).collect();
            ends.extend(whole_spacings - 1_000..whole_spacings + 4);
            for spacings in ends {
                let end_m = f64::from_bits(TRACK_LENGTH_M.to_bits() + spacings);
                // the start whose sum with the step is this end
                let mut start_m = end_m - step_m;
                while start_m + step_m < end_m {
                    start_m = f64::from_bits(start_m.to_bits() + 1);
                }
                assert_eq!(start_m + step_m, end_m);
                let mut state = start(&tire, start_m);
                let before = state;
                let mut frame = blank();
                let result = step(&tire, &mut state, &mut frame, load, dt_s);
                if end_m <= TRACK_LENGTH_M + allowance_m {
                    assert_eq!(result, Ok(()));
                    assert_eq!(state.distance_m, end_m);
                    if spacings > 0 {
                        landed_past_the_end += 1;
                    }
                } else {
                    assert_eq!(result, Err(StepReject::TravelBeyondTrack));
                    assert_eq!(state, before);
                    assert_eq!(frame.to_bytes(), blank().to_bytes());
                    refused += 1;
                }
            }
            assert_eq!(landed_past_the_end, 3 + 1_001);
            assert!(refused >= 3);
            // one allowance and a half beyond is refused
            let mut state = start(&tire, 4.0);
            state.distance_m = TRACK_LENGTH_M + 1.5 * allowance_m - step_m;
            let mut frame = blank();
            assert_eq!(
                step(&tire, &mut state, &mut frame, load, dt_s),
                Err(StepReject::TravelBeyondTrack)
            );
        }
    }

    /// A state that is not finite, and a tyre whose recorded constant has
    /// been changed, are named before anything is computed.
    #[test]
    fn a_broken_state_and_a_changed_tyre_are_named() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut state = start(&tire, 0.0);
        let mut frame = blank();
        state.slip_heat_j = f64::NAN;
        assert_eq!(
            step(&tire, &mut state, &mut frame, rig(0.10), 1.0),
            Err(StepReject::NonFiniteState)
        );
        let mut state = start(&tire, 0.0);
        state.slip_held_s = f64::INFINITY;
        assert_eq!(
            step(&tire, &mut state, &mut frame, rig(0.10), 1.0),
            Err(StepReject::NonFiniteState)
        );
        let mut state = start(&tire, 0.0);
        let mut grippier = tire;
        grippier.mu_traction = 0.9;
        assert!(!grippier.is_valid());
        assert_eq!(
            step(&grippier, &mut state, &mut frame, rig(0.10), 1.0),
            Err(StepReject::InvalidTire)
        );
        assert_eq!(TireState::try_start(&grippier, 0.0, 5.0), Err(StepReject::InvalidTire));
        let mut longer = tire;
        longer.track_length_m = 600.0;
        assert_eq!(
            step(&longer, &mut state, &mut frame, rig(0.10), 1.0),
            Err(StepReject::InvalidTire)
        );
        assert_eq!(frame.to_bytes(), blank().to_bytes());
    }

    /// The attack of rule 10, a falsified ledger. A step whose heat is
    /// counted with the slip of another definition, `v · s` in place of
    /// `v · s / (1 − s)`, loses heat in traction and is named
    /// `FirstLawBreach`; one whose wheel work is counted at the carriage
    /// speed has heat with no work behind it and is named
    /// `SpontaneousGeneration`. Neither moves the state or the frame.
    #[test]
    fn a_falsified_ledger_is_named() {
        let tire = TireEveR19Ice::on_rig_ice();
        let load = rig(0.20);
        let dt_s = 10.0;
        let honest = held(&tire, load, dt_s);
        let mut state = start(&tire, 0.0);
        let kept = state;
        let mut frame = blank();

        let lost_heat = Advance {
            slip_heat_j: honest.longitudinal_force_n * load.carriage_speed_m_s * load.slip_ratio * dt_s,
            ..honest
        };
        assert_eq!(
            commit(&tire, &mut state, &mut frame, load, dt_s, lost_heat),
            Err(StepReject::FirstLawBreach)
        );
        assert_eq!(state, kept);
        assert_eq!(frame.to_bytes(), blank().to_bytes());

        let free_heat = Advance {
            wheel_work_j: honest.drawbar_work_j,
            ..honest
        };
        assert_eq!(
            commit(&tire, &mut state, &mut frame, load, dt_s, free_heat),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(state, kept);
        assert_eq!(frame.to_bytes(), blank().to_bytes());

        // A defect just inside the limit lands, and just outside does not.
        let scale = tire.ledger_scale_j();
        assert!(scale > 10_331.0 && scale < 10_333.0);
        let inside = Advance {
            slip_heat_j: honest.slip_heat_j + 0.5 * RESIDUAL_CONSERVATION_LIMIT * scale,
            ..honest
        };
        assert_eq!(commit(&tire, &mut state, &mut frame, load, dt_s, inside), Ok(()));
        let mut state = kept;
        let outside = Advance {
            slip_heat_j: honest.slip_heat_j + 2.0 * RESIDUAL_CONSERVATION_LIMIT * scale,
            ..honest
        };
        assert_eq!(
            commit(&tire, &mut state, &mut frame, load, dt_s, outside),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(commit(&tire, &mut state, &mut frame, load, dt_s, honest), Ok(()));
        assert_eq!(frame.proof, PROOF);
        // A commit past the hold is refused as well, whatever its ledger.
        let mut late = TireState::try_start(&tire, 0.0, 15.0).unwrap();
        let before = late;
        assert_eq!(
            commit(&tire, &mut late, &mut frame, load, dt_s, honest),
            Err(StepReject::HoldBeyondMeasured)
        );
        assert_eq!(late, before);
    }

    /// A whole run of the rig, four slips, each its own run from 5 s to 21 s
    /// started where the last one ended: the residual of each stays at
    /// rounding, a thousandth of its limit or less.
    #[test]
    fn four_holds_of_the_rig_stay_at_rounding() {
        let tire = TireEveR19Ice::on_rig_ice();
        for sign in [1.0, -1.0] {
            let mut state = start(&tire, 0.0);
            let mut frame = blank();
            for slip in [0.05, 0.06, 0.07, 0.20] {
                state = start(&tire, state.distance_m);
                let mut count = 0;
                while count < 1_600 {
                    step(&tire, &mut state, &mut frame, rig(sign * slip), 0.01).unwrap();
                    count += 1;
                }
                assert!(relative(state.t_s, 16.0) < 1.0e-12);
                assert!(state.residual_conservation_error < 1.0e-12);
                assert!(state.slip_heat_j > 0.0);
                assert_eq!(frame.residual, state.residual_conservation_error as f32);
            }
            assert!(relative(state.distance_m, 3.84) < 1.0e-12);
        }
    }

    /// One hold at the step floor and the slowest speed, 16,384 steps: the
    /// residual stays under a hundredth of its limit. The whole track at the
    /// floor, as eight runs, is 122,880 steps.
    #[test]
    fn a_hold_at_the_step_floor_stays_inside_the_ledger() {
        let tire = TireEveR19Ice::on_rig_ice();
        let floor = tire.floor_dt_s().unwrap();
        let load = TireLoad {
            normal_load_n: 6_500.0,
            inflation_pressure_pa: 150_000.0,
            slip_ratio: -0.05,
            carriage_speed_m_s: 0.05,
        };
        let mut state = start(&tire, 0.0);
        let mut frame = blank();
        let mut landed = 0_u32;
        let mut worst = 0.0_f64;
        let mut ends = 0;
        loop {
            match step(&tire, &mut state, &mut frame, load, floor) {
                Ok(()) => {
                    landed += 1;
                    worst = worst.max(state.residual_conservation_error);
                }
                Err(StepReject::HoldBeyondMeasured) => {
                    if ends == 0 {
                        assert_eq!(landed, 16_384);
                        assert_eq!(state.slip_held_s, 21.0);
                    }
                    ends += 1;
                    state = start(&tire, state.distance_m);
                }
                Err(reject) => {
                    assert_eq!(reject, StepReject::TravelBeyondTrack);
                    break;
                }
            }
        }
        assert_eq!(ends, 7);
        assert!(landed > 122_870 && landed < 122_890, "{landed}");
        assert!(worst < 1.0e-11, "{worst}");
    }
}
