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
//! caller's input so that a frame outside the measured pressures carries
//! [`FLAG_BEYOND_COMPARED`], and so that a flat tyre is named.
//! Also left out: the rise of the force from rest, which the first stage of
//! every run shows and which lasts most of a stage, and the rise over the
//! first seconds after a change of slip (below); slips under 5 percent,
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
//! held to [`RESIDUAL_CONSERVATION_LIMIT`] of the ledger scale. The scale is
//! the larger of a fixed energy and the largest of the three works of the
//! run. The fixed energy is the heaviest load measured at the highest friction
//! ratio of the card over the 6 m a measured run rolled, about 10.3 kJ, the
//! scale through engine 0.9: on every run of 3.5 to 6.5 kN along 6 m it is
//! the larger, so the residual of such a frame reads as it did. A heavier
//! load or a longer run does more work, and the scale then follows the work,
//! so the ledger holds the same relative closure at every load and every
//! distance. A positive defect is work that came from nowhere and is
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
//! 6.5 kN (0.0477) and the comparator (0.0449 against 0.0441), and the
//! seconds from 1 s to 5 s did no better than the constant comparator (0.0430
//! against 0.0424 from 1 s to 2 s, 0.0399 against 0.0395 from 2 s to 5 s).
//! With the protocol's own cuts the window from 5 s to 11 s met all five
//! lines as well. No scored slip was held longer than one stage, about 20 s,
//! except the last slip of a run, 7 or 20 percent, which the rig held for 21
//! to 33 s. The state carries `slip_held_s`, the time the present slip has
//! been held; a start states it, and a run carries it from step to step while
//! the slip is held and starts it again at zero when the slip changes. The
//! measured force rises over the first seconds after a slip is set, and the
//! card's force law has no rise: it gives the steady force from the instant
//! the slip is set, and there it was compared and missed. A step that starts
//! before [`SLIP_SETTLE_S`], 5 s, which every step after a change of slip
//! does, lands with the same force law and the same ledger and carries
//! [`FLAG_SLIP_SETTLING`]; through engine 0.11 it was refused as
//! `SlipNotSettled`. A time held below zero is no time a slip can have been
//! held, [`StepReject::SlipHeldBelowZero`]. From 5 s on the card holds the
//! force for as long as the slip is held; past [`COMPARED_HOLD_END_S`], 21 s,
//! a frame carries [`FLAG_BEYOND_COMPARED`].
//!
//! The force does not depend on where the wheel is along the track. The
//! rig's runs rolled 90 to 97 s, about 5.7 m of ice, and the distance goes on
//! past the end of that track for as long as the run goes on.
//!
//! What stops the card besides. A normal load of zero or less is no contact:
//! the tyre does not press on the ice, and the step is
//! [`StepReject::NoContact`]. A gauge pressure of zero or less is a flat
//! tyre, [`StepReject::TyreFlat`]. A distance below zero is
//! [`StepReject::DistanceBelowZero`]. A step that does not move the time held
//! at the slip or the distance in 64-bit arithmetic is
//! [`StepReject::StepTooSmallToResolve`].
//!
//! [`TireEveR19Ice::gate_dt_s`] is the step ceiling, 2^22 s, about 48.5
//! days, from the step's arithmetic. The force law has no dynamics, so every
//! work of a step is exact at any size: no step error grows with the step.
//! What a step must still do is move the sums of the clock, the time held and
//! the distance. The step floor, [`TireEveR19Ice::floor_dt_s`], is 2^-10 s,
//! about a sixth of the sampling interval of the measurements. Below 2^42 s
//! the spacing of 64-bit numbers is at most 2^-11 s, half the floor, so a
//! floor step still moves a time there; a run of 2^20 steps, more than the
//! million one request takes, at the ceiling ends at 2^42 s. The ceiling is
//! therefore 2^42 s over 2^20, and the floor is the ceiling divided by
//! 2^32. A sum that a step would not move, past those, is refused by name
//! and not left standing. The public step applies neither. The gateway
//! refuses a step outside them.
//!
//! `core` only: no heap and no standard prelude. The card, the state and the
//! load are `Copy`. An accepted step writes time, the distance along the
//! track, the rim travel, the slip, the three works, the longitudinal force
//! and the conservation residual into [`LastStateFrame64`]; the time held at
//! the slip is in the state and has no frame slot. The 16-byte frame proof is
//! not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

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
/// Smallest normal load measured, N. Below it a frame carries
/// [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_LOAD_FLOOR_N: f64 = 3_500.0;
/// Largest normal load measured, N, which the constants were not fitted on:
/// held back and compared. Above it a frame carries [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_LOAD_CEILING_N: f64 = 6_500.0;
/// Largest normal load the constants were fitted on, N. Advisory only; above
/// it the frame carries [`FLAG_LOAD_ABOVE_FITTED`].
pub const NORMAL_LOAD_FITTED_CEILING_N: f64 = 5_000.0;
/// Smallest inflation pressure measured, Pa: 1.5 bar. Below it a frame
/// carries [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_PRESSURE_FLOOR_PA: f64 = 150_000.0;
/// Largest inflation pressure measured, Pa: 3.5 bar. Above it a frame
/// carries [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_PRESSURE_CEILING_PA: f64 = 350_000.0;
/// Smallest carriage speed the card accepts, m/s.
pub const CARRIAGE_SPEED_FLOOR_M_S: f64 = 0.05;
/// Largest carriage speed the card accepts, m/s. The rig moved at about 0.06.
pub const CARRIAGE_SPEED_CEILING_M_S: f64 = 0.07;
/// Length of track a measured run rolled, m: the opened runs roll for 90 to
/// 97 s, about 5.7 m. It sets the fixed part of the ledger scale.
pub const TRACK_LENGTH_M: f64 = 6.0;
/// Time at one slip before which a frame carries [`FLAG_SLIP_SETTLING`], s.
/// Every one-second window of the measurements before it failed a line of
/// the comparison; every one from it to 20 s met all five. Not a limit.
pub const SLIP_SETTLE_S: f64 = 5.0;
/// Longest time at one slip compared with the measurements, s: one slip
/// stage. Past it a frame carries [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_HOLD_END_S: f64 = 21.0;
/// How far past [`COMPARED_HOLD_END_S`] the running sum of the time held may
/// read and still be within the compared hold, s: half the spacing of 64-bit
/// numbers between 16 s and 32 s, 2^-49 s, for each of the 16,384 steps of
/// 2^-10 s in 16 s and six more, 2.9e-11 s. It moves only
/// [`FLAG_BEYOND_COMPARED`].
pub const COMPARED_HOLD_END_ALLOWANCE_S: f64 = 2.911_448_859_777_010_5e-11;
/// Step ceiling, s: 2^22 s. See the header.
pub const GATE_DT_S: f64 = 4_194_304.0;
/// Step ceiling divided by step floor: 2^32.
pub const GATE_OVER_FLOOR: f64 = 4_294_967_296.0;
/// `|D + H − W|` over the ledger scale of the run must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-9;

/// The normal load is above 5 000 N, the largest load the constants were
/// fitted on, on this frame.
pub const FLAG_LOAD_ABOVE_FITTED: u64 = 1 << 0;
/// The frame is beyond what was compared with this tyre's measurements: a
/// normal load below 3 500 N or above 6 500 N, an inflation pressure below
/// 150 kPa or above 350 kPa, or a slip held longer than 21 s. The same force
/// law and the same ledger.
pub const FLAG_BEYOND_COMPARED: u64 = 1 << 1;
/// The step starts less than 5 s after its slip was set, where the measured
/// force was still rising and the card's steady force missed it: on the 60
/// held-back cells the root-mean-square error of the friction ratio was
/// 0.0449 in the first second, 0.0477 at 6.5 kN against the line of 0.045,
/// and from 1 s to 5 s no better than a constant ratio's. The same force law
/// and the same ledger. The frame carries it; the state holds no boolean of
/// its own for it.
pub const FLAG_SLIP_SETTLING: u64 = 1 << 2;

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
    /// The normal load is zero or below, or under the smallest normal 64-bit number: the tyre does not press on the ice, and a friction force has nothing to act on.
    NoContact,
    /// The inflation pressure, gauge, is zero or below: the tyre is flat, and the card is of a tyre inflated.
    TyreFlat,
    /// The slip magnitude is below 0.05 or above 0.20, the slips the constants cover.
    SlipOutsideCard,
    /// The carriage speed is below 0.05 m/s or above 0.07 m/s, the creep of the rig.
    CarriageSpeedOutsideCard,
    /// The distance along the track is below zero.
    DistanceBelowZero,
    /// The time the slip has been held, stated at the start or carried in the state, is below zero: a slip is held from the instant it is set.
    SlipHeldBelowZero,
    /// The step does not move the time held at the slip, or the distance, in 64-bit arithmetic.
    StepTooSmallToResolve,
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
    /// Inflation pressure, gauge. It does not enter the force; zero or below
    /// is a flat tyre.
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
    /// Recorded copy of [`COMPARED_LOAD_FLOOR_N`].
    pub compared_load_floor_n: f64,
    /// Recorded copy of [`COMPARED_LOAD_CEILING_N`].
    pub compared_load_ceiling_n: f64,
    /// Recorded copy of [`NORMAL_LOAD_FITTED_CEILING_N`].
    pub normal_load_fitted_ceiling_n: f64,
    /// Recorded copy of [`COMPARED_PRESSURE_FLOOR_PA`].
    pub compared_pressure_floor_pa: f64,
    /// Recorded copy of [`COMPARED_PRESSURE_CEILING_PA`].
    pub compared_pressure_ceiling_pa: f64,
    /// Recorded copy of [`CARRIAGE_SPEED_FLOOR_M_S`].
    pub carriage_speed_floor_m_s: f64,
    /// Recorded copy of [`CARRIAGE_SPEED_CEILING_M_S`].
    pub carriage_speed_ceiling_m_s: f64,
    /// Recorded copy of [`TRACK_LENGTH_M`].
    pub track_length_m: f64,
    /// Recorded copy of [`SLIP_SETTLE_S`].
    pub slip_settle_s: f64,
    /// Recorded copy of [`COMPARED_HOLD_END_S`].
    pub compared_hold_end_s: f64,
    /// Recorded copy of [`COMPARED_HOLD_END_ALLOWANCE_S`].
    pub compared_hold_end_allowance_s: f64,
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
            compared_load_floor_n: COMPARED_LOAD_FLOOR_N,
            compared_load_ceiling_n: COMPARED_LOAD_CEILING_N,
            normal_load_fitted_ceiling_n: NORMAL_LOAD_FITTED_CEILING_N,
            compared_pressure_floor_pa: COMPARED_PRESSURE_FLOOR_PA,
            compared_pressure_ceiling_pa: COMPARED_PRESSURE_CEILING_PA,
            carriage_speed_floor_m_s: CARRIAGE_SPEED_FLOOR_M_S,
            carriage_speed_ceiling_m_s: CARRIAGE_SPEED_CEILING_M_S,
            track_length_m: TRACK_LENGTH_M,
            slip_settle_s: SLIP_SETTLE_S,
            compared_hold_end_s: COMPARED_HOLD_END_S,
            compared_hold_end_allowance_s: COMPARED_HOLD_END_ALLOWANCE_S,
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
            && self.compared_load_floor_n.to_bits() == COMPARED_LOAD_FLOOR_N.to_bits()
            && self.compared_load_ceiling_n.to_bits() == COMPARED_LOAD_CEILING_N.to_bits()
            && self.normal_load_fitted_ceiling_n.to_bits() == NORMAL_LOAD_FITTED_CEILING_N.to_bits()
            && self.compared_pressure_floor_pa.to_bits() == COMPARED_PRESSURE_FLOOR_PA.to_bits()
            && self.compared_pressure_ceiling_pa.to_bits() == COMPARED_PRESSURE_CEILING_PA.to_bits()
            && self.carriage_speed_floor_m_s.to_bits() == CARRIAGE_SPEED_FLOOR_M_S.to_bits()
            && self.carriage_speed_ceiling_m_s.to_bits() == CARRIAGE_SPEED_CEILING_M_S.to_bits()
            && self.track_length_m.to_bits() == TRACK_LENGTH_M.to_bits()
            && self.slip_settle_s.to_bits() == SLIP_SETTLE_S.to_bits()
            && self.compared_hold_end_s.to_bits() == COMPARED_HOLD_END_S.to_bits()
            && self.compared_hold_end_allowance_s.to_bits()
                == COMPARED_HOLD_END_ALLOWANCE_S.to_bits()
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

    /// `W_scale`: the heaviest load measured at the highest friction ratio of
    /// the card over the 6 m a measured run rolled, J. About 10 332 J.
    #[must_use]
    pub fn ledger_scale_j(&self) -> f64 {
        self.compared_load_ceiling_n * self.friction_ratio(-self.slip_floor) * self.track_length_m
    }

    /// The scale the ledger of a run is held to, J: the larger of
    /// [`Self::ledger_scale_j`] and the size of each of the three works of
    /// the run so far.
    #[must_use]
    pub fn run_ledger_scale_j(&self, state: &TireState) -> f64 {
        let mut scale = self.ledger_scale_j();
        for work in [state.wheel_work_j.abs(), state.drawbar_work_j.abs(), state.slip_heat_j] {
            if work > scale {
                scale = work;
            }
        }
        scale
    }

    /// The load and the time held of a frame are beyond what was compared
    /// with the measurements: see [`FLAG_BEYOND_COMPARED`].
    #[must_use]
    pub fn beyond_compared(&self, load: TireLoad, slip_held_s: f64) -> bool {
        !(load.normal_load_n >= self.compared_load_floor_n
            && load.normal_load_n <= self.compared_load_ceiling_n
            && load.inflation_pressure_pa >= self.compared_pressure_floor_pa
            && load.inflation_pressure_pa <= self.compared_pressure_ceiling_pa
            && slip_held_s <= self.compared_hold_end_s + self.compared_hold_end_allowance_s)
    }

    /// Largest step the card stands behind: [`GATE_DT_S`], 2^22 s, from the
    /// step's arithmetic (see the header). Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.gate_dt_s.is_finite() && self.gate_dt_s > 0.0 {
            Some(self.gate_dt_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^32, 2^-10 s. Not applied by [`step`].
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
    /// `|energy_defect_j|` over the ledger scale of the run.
    pub residual_conservation_error: f64,
    /// The normal load of the last step is above [`NORMAL_LOAD_FITTED_CEILING_N`].
    pub load_above_fitted: bool,
    /// The last step is beyond what was compared: see [`FLAG_BEYOND_COMPARED`].
    pub beyond_compared: bool,
}

impl TireState {
    /// The wheel at this distance along the track, its first slip already
    /// held for `slip_held_s`, time 0, with the ledger empty.
    /// [`StepReject::InvalidTire`] for a changed tyre,
    /// [`StepReject::NonFiniteState`] for a value that is not finite,
    /// [`StepReject::DistanceBelowZero`] for a distance below zero and
    /// [`StepReject::SlipHeldBelowZero`] for a time held below zero. A time
    /// held under [`SLIP_SETTLE_S`] is a start: its first steps carry
    /// [`FLAG_SLIP_SETTLING`].
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
        if !(distance_m >= 0.0) {
            return Err(StepReject::DistanceBelowZero);
        }
        if !(slip_held_s >= 0.0) {
            return Err(StepReject::SlipHeldBelowZero);
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
            beyond_compared: false,
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
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    tire: &TireEveR19Ice,
    state: &mut TireState,
    frame: &mut LastStateFrame64,
    load: TireLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(tire, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("tire_eve_r19_ice", exceedances(tire, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    tire: &TireEveR19Ice,
    state: &mut TireState,
    frame: &mut LastStateFrame64,
    load: TireLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(tire, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
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

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut TireState) {
    state.t_s = normal_or_zero(state.t_s);
    state.distance_m = normal_or_zero(state.distance_m);
    state.rim_travel_m = normal_or_zero(state.rim_travel_m);
    state.slip_ratio = normal_or_zero(state.slip_ratio);
    state.slip_held_s = normal_or_zero(state.slip_held_s);
    state.longitudinal_force_n = normal_or_zero(state.longitudinal_force_n);
    state.wheel_work_j = normal_or_zero(state.wheel_work_j);
    state.drawbar_work_j = normal_or_zero(state.drawbar_work_j);
    state.slip_heat_j = normal_or_zero(state.slip_heat_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
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
    if !(load.normal_load_n >= f64::MIN_POSITIVE) {
        return Err(StepReject::NoContact);
    }
    if !(load.inflation_pressure_pa > 0.0) {
        return Err(StepReject::TyreFlat);
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
    if !(held_before_s >= 0.0) {
        return Err(StepReject::SlipHeldBelowZero);
    }
    if !(state.distance_m >= 0.0) {
        return Err(StepReject::DistanceBelowZero);
    }
    if !(held_before_s + dt_s > held_before_s
        && state.distance_m + load.carriage_speed_m_s * dt_s > state.distance_m)
    {
        return Err(StepReject::StepTooSmallToResolve);
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
    state.beyond_compared = tire.beyond_compared(load, state.slip_held_s);
    let scale = tire.run_ledger_scale_j(state);
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
    if !(state.distance_m >= 0.0) {
        *state = saved;
        return Err(StepReject::DistanceBelowZero);
    }
    publish(state, frame, held_before_s < tire.slip_settle_s);
    Ok(())
}

fn publish(state: &TireState, frame: &mut LastStateFrame64, settling: bool) {
    frame.t = state.t_s;
    frame.pos[0] = state.distance_m as f32;
    frame.pos[1] = state.rim_travel_m as f32;
    frame.pos[2] = state.slip_ratio as f32;
    frame.vel[0] = state.wheel_work_j as f32;
    frame.vel[1] = state.drawbar_work_j as f32;
    frame.vel[2] = state.slip_heat_j as f32;
    frame.force_torque = state.longitudinal_force_n as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.load_above_fitted {
        flags |= FLAG_LOAD_ABOVE_FITTED;
    }
    if state.beyond_compared {
        flags |= FLAG_BEYOND_COMPARED;
    }
    if settling {
        flags |= FLAG_SLIP_SETTLING;
    }
    frame.flags = flags;
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

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    FITTED_KIND,
    LOAD_LOW,
    LOAD_HIGH,
    PRESSURE_LOW,
    PRESSURE_HIGH,
    HOLD_HIGH,
    SETTLING_KIND,
];

const FITTED_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_LOAD_ABOVE_FITTED",
    FLAG_LOAD_ABOVE_FITTED,
    "normal_load_n",
    "N",
    Side::Above,
    NORMAL_LOAD_FITTED_CEILING_N,
    "the largest normal load the constants were fitted on, 5,000 N",
);
const fn beyond(quantity: &'static str, unit: &'static str, side: Side, limit: f64, words: &'static str) -> ExceedanceKind {
    ExceedanceKind::fixed("FLAG_BEYOND_COMPARED", FLAG_BEYOND_COMPARED, quantity, unit, side, limit, words)
}
const LOAD_WORDS: &str = "the normal loads compared with this tyre's measurements, 3,500 to 6,500 N";
const LOAD_LOW: ExceedanceKind = beyond("normal_load_n", "N", Side::Below, COMPARED_LOAD_FLOOR_N, LOAD_WORDS);
const LOAD_HIGH: ExceedanceKind = beyond("normal_load_n", "N", Side::Above, COMPARED_LOAD_CEILING_N, LOAD_WORDS);
const PRESSURE_WORDS: &str = "the inflation pressures compared, 150 to 350 kPa";
const PRESSURE_LOW: ExceedanceKind = beyond("inflation_pressure_pa", "Pa", Side::Below, COMPARED_PRESSURE_FLOOR_PA, PRESSURE_WORDS);
const PRESSURE_HIGH: ExceedanceKind = beyond("inflation_pressure_pa", "Pa", Side::Above, COMPARED_PRESSURE_CEILING_PA, PRESSURE_WORDS);
const HOLD_HIGH: ExceedanceKind = beyond(
    "slip_held_s",
    "s",
    Side::Above,
    COMPARED_HOLD_END_S,
    "the longest slip held in the compared measurements, 21 s; set past it by more than its rounding allowance",
);
const SETTLING_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_SLIP_SETTLING",
    FLAG_SLIP_SETTLING,
    "slip_held_at_step_start_s",
    "s",
    Side::Below,
    SLIP_SETTLE_S,
    "5 s after the slip was set: before it the measured force was still rising and the card's steady force missed it",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The conditions are those of the step's `load_above_fitted`, of
/// [`TireEveR19Ice::beyond_compared`] and of the settling the step publishes.
pub fn exceedances(
    tire: &TireEveR19Ice,
    before: &TireState,
    after: &TireState,
    load: TireLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = dt_s;
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.load_above_fitted, Exceedance::at(FITTED_KIND, tire.normal_load_fitted_ceiling_n, load.normal_load_n));
    if after.beyond_compared {
        let force = load.normal_load_n;
        let pressure = load.inflation_pressure_pa;
        records.push_if(!(force >= tire.compared_load_floor_n), Exceedance::at(LOAD_LOW, tire.compared_load_floor_n, force));
        records.push_if(!(force <= tire.compared_load_ceiling_n), Exceedance::at(LOAD_HIGH, tire.compared_load_ceiling_n, force));
        records.push_if(
            !(pressure >= tire.compared_pressure_floor_pa),
            Exceedance::at(PRESSURE_LOW, tire.compared_pressure_floor_pa, pressure),
        );
        records.push_if(
            !(pressure <= tire.compared_pressure_ceiling_pa),
            Exceedance::at(PRESSURE_HIGH, tire.compared_pressure_ceiling_pa, pressure),
        );
        records.push_if(
            !(after.slip_held_s <= tire.compared_hold_end_s + tire.compared_hold_end_allowance_s),
            Exceedance::at(HOLD_HIGH, tire.compared_hold_end_s, after.slip_held_s),
        );
    }
    let held_before_s = held_before_step_s(before, load);
    records.push_if(held_before_s < tire.slip_settle_s, Exceedance::at(SETTLING_KIND, tire.slip_settle_s, held_before_s));
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
        Advance, CARRIAGE_SPEED_CEILING_M_S, CARRIAGE_SPEED_FLOOR_M_S, COMPARED_HOLD_END_S,
        COMPARED_LOAD_CEILING_N, COMPARED_LOAD_FLOOR_N, COMPARED_PRESSURE_CEILING_PA,
        COMPARED_PRESSURE_FLOOR_PA, FLAG_BEYOND_COMPARED, FLAG_LOAD_ABOVE_FITTED, FLAG_SLIP_SETTLING, GATE_DT_S,
        GATE_OVER_FLOOR, MU_BRAKING, MU_TRACTION, RESIDUAL_CONSERVATION_LIMIT, SLIP_CEILING,
        SLIP_EXPONENT, SLIP_FLOOR, SLIP_SETTLE_S, StepReject, TRACK_LENGTH_M, TireEveR19Ice,
        TireLoad, TireState, commit, held, step, validate,
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
    /// (`analysis/out/fit.json`), to six significant figures. The step
    /// ceiling is 2^22 s and the floor 2^-10 s.
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
        assert_eq!((SLIP_SETTLE_S, COMPARED_HOLD_END_S), (5.0, 21.0));
        assert_eq!(GATE_DT_S, 4_194_304.0);
        assert_eq!(TRACK_LENGTH_M, 6.0);
        assert_eq!(GATE_DT_S, (2.0_f64).powi(42) / (2.0_f64).powi(20));
        assert_eq!(GATE_OVER_FLOOR, (2.0_f64).powi(32));
        assert_eq!(tire.gate_dt_s(), Some(GATE_DT_S));
        assert_eq!(tire.floor_dt_s(), Some(1.0 / 1_024.0));
        // Below 2^42 s the spacing of 64-bit numbers is at most half the floor.
        let below = f64::from_bits((2.0_f64).powi(42).to_bits() - 1);
        assert!(f64::from_bits(below.to_bits() + 1) - below <= 0.5 * tire.floor_dt_s().unwrap());
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
        step(&tire, &mut state, &mut frame, rig(0.12), 16.0).unwrap();
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
        assert_eq!(state.slip_held_s, COMPARED_HOLD_END_S);
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
        step(&tire, &mut state, &mut frame, rig(-0.20), 16.0).unwrap();
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
    /// the time held. The load measured last, 6.5 kN, is inside the
    /// compared range and carries no second flag.
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
        let cases: [(TireLoad, f64, StepReject); 23] = [
            (TireLoad { normal_load_n: nan, ..base }, 1.0, StepReject::NonFiniteInput),
            (TireLoad { inflation_pressure_pa: f64::INFINITY, ..base }, 1.0, StepReject::NonFiniteInput),
            (TireLoad { slip_ratio: nan, ..base }, 1.0, StepReject::NonFiniteInput),
            (TireLoad { carriage_speed_m_s: f64::NEG_INFINITY, ..base }, 1.0, StepReject::NonFiniteInput),
            (base, nan, StepReject::NonFiniteInput),
            (base, 0.0, StepReject::NonPositiveDt),
            (base, -1.0, StepReject::NonPositiveDt),
            (TireLoad { normal_load_n: 0.0, ..base }, 1.0, StepReject::NoContact),
            (TireLoad { normal_load_n: -0.0, ..base }, 1.0, StepReject::NoContact),
            (TireLoad { normal_load_n: -5_000.0, ..base }, 1.0, StepReject::NoContact),
            (TireLoad { normal_load_n: 1.0e-310, ..base }, 1.0, StepReject::NoContact),
            (TireLoad { inflation_pressure_pa: 0.0, ..base }, 1.0, StepReject::TyreFlat),
            (TireLoad { inflation_pressure_pa: -50_000.0, ..base }, 1.0, StepReject::TyreFlat),
            (TireLoad { slip_ratio: 0.0, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: 0.049_999, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: -0.03, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: 0.200_001, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: -1.0, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { slip_ratio: 1.0, ..base }, 1.0, StepReject::SlipOutsideCard),
            (TireLoad { carriage_speed_m_s: 0.049, ..base }, 1.0, StepReject::CarriageSpeedOutsideCard),
            (TireLoad { carriage_speed_m_s: 0.071, ..base }, 1.0, StepReject::CarriageSpeedOutsideCard),
            (TireLoad { carriage_speed_m_s: 10.0, ..base }, 1.0, StepReject::CarriageSpeedOutsideCard),
            // a step that does not move the time held
            (base, 1.0e-20, StepReject::StepTooSmallToResolve),
        ];
        for (load, dt_s, reject) in cases {
            assert_eq!(step(&tire, &mut state, &mut frame, load, dt_s), Err(reject), "{load:?} {dt_s}");
            assert_eq!(state, kept_state);
            assert_eq!(frame.to_bytes(), kept_frame);
        }
        // The ends of the speed range are inside the card; the compared loads
        // and pressures and everything past them land.
        for load in [
            TireLoad { normal_load_n: COMPARED_LOAD_FLOOR_N, ..base },
            TireLoad { normal_load_n: COMPARED_LOAD_CEILING_N, ..base },
            TireLoad { normal_load_n: 1.0, ..base },
            TireLoad { normal_load_n: 1.0e6, ..base },
            TireLoad { inflation_pressure_pa: COMPARED_PRESSURE_FLOOR_PA, ..base },
            TireLoad { inflation_pressure_pa: COMPARED_PRESSURE_CEILING_PA, ..base },
            TireLoad { inflation_pressure_pa: 1.0, ..base },
            TireLoad { inflation_pressure_pa: 1.0e7, ..base },
            TireLoad { carriage_speed_m_s: CARRIAGE_SPEED_FLOOR_M_S, ..base },
            TireLoad { carriage_speed_m_s: CARRIAGE_SPEED_CEILING_M_S, ..base },
        ] {
            assert_eq!(validate(&tire, &state, load, 1.0), Ok(()));
        }
        for slip in [SLIP_FLOOR, -SLIP_FLOOR, SLIP_CEILING, -SLIP_CEILING] {
            let fresh = start(&tire, 1.0);
            assert_eq!(validate(&tire, &fresh, rig(slip), 1.0), Ok(()));
        }
        // A distance below zero is no start, and a state made so by hand is
        // refused by name.
        assert_eq!(TireState::try_start(&tire, -0.001, 5.0), Err(StepReject::DistanceBelowZero));
        let mut backward = start(&tire, 0.0);
        backward.distance_m = -1.0e-9;
        let kept = backward;
        assert_eq!(step(&tire, &mut backward, &mut frame, base, 1.0), Err(StepReject::DistanceBelowZero));
        assert_eq!(backward, kept);
    }

    /// The compared range is the loads, the pressures and the hold measured:
    /// its ends carry no flag, and beyond each a frame lands with
    /// FLAG_BEYOND_COMPARED, the force the same law.
    #[test]
    fn beyond_the_compared_loads_pressures_and_hold_a_frame_is_flagged() {
        let tire = TireEveR19Ice::on_rig_ice();
        let below = |x: f64| f64::from_bits(x.to_bits() - 1);
        let above = |x: f64| f64::from_bits(x.to_bits() + 1);
        let base = rig(0.10);
        for (load, flagged) in [
            (TireLoad { normal_load_n: COMPARED_LOAD_FLOOR_N, ..base }, false),
            (TireLoad { normal_load_n: COMPARED_LOAD_CEILING_N, ..base }, false),
            (TireLoad { normal_load_n: below(COMPARED_LOAD_FLOOR_N), ..base }, true),
            (TireLoad { normal_load_n: above(COMPARED_LOAD_CEILING_N), ..base }, true),
            (TireLoad { inflation_pressure_pa: COMPARED_PRESSURE_FLOOR_PA, ..base }, false),
            (TireLoad { inflation_pressure_pa: COMPARED_PRESSURE_CEILING_PA, ..base }, false),
            (TireLoad { inflation_pressure_pa: below(COMPARED_PRESSURE_FLOOR_PA), ..base }, true),
            (TireLoad { inflation_pressure_pa: above(COMPARED_PRESSURE_CEILING_PA), ..base }, true),
        ] {
            let mut state = start(&tire, 0.0);
            let mut frame = blank();
            step(&tire, &mut state, &mut frame, load, 1.0).unwrap();
            assert_eq!(frame.flags & FLAG_BEYOND_COMPARED != 0, flagged, "{load:?}");
            assert_eq!(
                state.longitudinal_force_n,
                tire.longitudinal_force_n(load.normal_load_n, load.slip_ratio)
            );
        }
        // The hold: one step of 16 s from 5 s ends on 21 s with no flag, and
        // the next step of the floor is flagged.
        let mut state = start(&tire, 0.0);
        let mut frame = blank();
        step(&tire, &mut state, &mut frame, base, 16.0).unwrap();
        assert_eq!(state.slip_held_s, 21.0);
        assert_eq!(frame.flags & FLAG_BEYOND_COMPARED, 0);
        step(&tire, &mut state, &mut frame, base, tire.floor_dt_s().unwrap()).unwrap();
        assert_eq!(frame.flags & FLAG_BEYOND_COMPARED, FLAG_BEYOND_COMPARED);
        // 1,600 steps of 0.01 s read 21.000000000000547 s, within the
        // rounding of the sum, and carry no flag.
        let mut many = start(&tire, 0.0);
        for _ in 0..1_600 {
            step(&tire, &mut many, &mut frame, base, 0.01).unwrap();
        }
        assert!(many.slip_held_s > 21.0);
        assert_eq!(frame.flags & FLAG_BEYOND_COMPARED, 0);
        // A load above the fitted and above the compared carries both flags.
        let mut state = start(&tire, 0.0);
        step(&tire, &mut state, &mut frame, TireLoad { normal_load_n: 9_000.0, ..base }, 1.0).unwrap();
        assert_eq!(frame.flags, FLAG_LOAD_ABOVE_FITTED | FLAG_BEYOND_COMPARED);
    }

    /// The time at one slip runs from the instant the slip is set for as long
    /// as the slip is held. A start states it, a run carries it, a change of
    /// slip starts it again at zero, and a restart hands it back. A time held
    /// below zero is no time a slip can have been held.
    #[test]
    fn the_time_at_one_slip_runs_from_zero_for_as_long_as_it_is_held() {
        let tire = TireEveR19Ice::on_rig_ice();
        let below = f64::from_bits(SLIP_SETTLE_S.to_bits() - 1);
        assert!(TireState::try_start(&tire, 0.0, 0.0).is_ok());
        assert!(TireState::try_start(&tire, 0.0, below).is_ok());
        assert_eq!(TireState::try_start(&tire, 0.0, -1.0e-300), Err(StepReject::SlipHeldBelowZero));
        assert_eq!(TireState::try_start(&tire, 0.0, -5.0), Err(StepReject::SlipHeldBelowZero));
        assert!(TireState::try_start(&tire, 0.0, -0.0).is_ok());
        assert_eq!(TireState::try_start(&tire, 0.0, f64::NAN), Err(StepReject::NonFiniteState));
        assert!(TireState::try_start(&tire, 0.0, 21.5).is_ok());
        assert!(TireState::try_start(&tire, 0.0, 1.0e6).is_ok());
        // Through engine 0.9 one step of the floor from 21 s was refused as
        // HoldBeyondMeasured; it lands, flagged.
        let mut frame = blank();
        let mut at_end = TireState::try_start(&tire, 0.0, 21.0).unwrap();
        step(&tire, &mut at_end, &mut frame, rig(0.15), tire.floor_dt_s().unwrap()).unwrap();
        assert_eq!(frame.flags & FLAG_BEYOND_COMPARED, FLAG_BEYOND_COMPARED);
        // A restart from the returned state carries the time held on past 21 s.
        let mut first = start(&tire, 0.0);
        step(&tire, &mut first, &mut frame, rig(0.15), 10.0).unwrap();
        let mut again = TireState::try_start(&tire, first.distance_m, first.slip_held_s).unwrap();
        step(&tire, &mut again, &mut frame, rig(0.15), 7.0).unwrap();
        assert_eq!(again.slip_held_s, 22.0);
        // The rig held the last slip of a run 21 to 33 s: a hold of 33 s in
        // one run lands every step, the force steady.
        let mut long = start(&tire, 0.0);
        for _ in 0..2_800 {
            step(&tire, &mut long, &mut frame, rig(0.07), 0.01).unwrap();
        }
        assert!(relative(long.slip_held_s, 33.0) < 1.0e-12);
        assert_eq!(long.longitudinal_force_n, tire.longitudinal_force_n(5_000.0, 0.07));
        // The first step after a change of slip starts at zero and lands,
        // flagged; through engine 0.11 it was refused as SlipNotSettled.
        let floor = tire.floor_dt_s().unwrap();
        let mut changed = start(&tire, 0.0);
        step(&tire, &mut changed, &mut frame, rig(0.15), 1.0).unwrap();
        assert_eq!(frame.flags & FLAG_SLIP_SETTLING, 0);
        step(&tire, &mut changed, &mut frame, rig(0.12), floor).unwrap();
        assert_eq!(frame.flags & FLAG_SLIP_SETTLING, FLAG_SLIP_SETTLING);
        assert_eq!(changed.slip_held_s, floor);
        assert_eq!(changed.longitudinal_force_n, tire.longitudinal_force_n(5_000.0, 0.12));
        // A state that carries a time held below zero is refused, state and frame kept.
        let mut forged = changed;
        forged.slip_held_s = -1.0;
        let kept = forged;
        let held = frame.to_bytes();
        assert_eq!(step(&tire, &mut forged, &mut frame, rig(0.12), 1.0), Err(StepReject::SlipHeldBelowZero));
        assert_eq!(forged, kept);
        assert_eq!(frame.to_bytes(), held);
        // A changed copy of a constant is not this tyre.
        for altered in [
            TireEveR19Ice { slip_settle_s: 0.0, ..tire },
            TireEveR19Ice { compared_hold_end_s: 100.0, ..tire },
            TireEveR19Ice { compared_hold_end_allowance_s: 1.0, ..tire },
            TireEveR19Ice { compared_load_ceiling_n: 1.0e9, ..tire },
        ] {
            assert!(!altered.is_valid());
            assert_eq!(TireState::try_start(&altered, 0.0, 5.0), Err(StepReject::InvalidTire));
        }
    }

    /// The end of the rig's track is not an end of the tyre: a run rolls on
    /// past 6 m. 1,000 steps of 0.1 s at 0.06 m/s from 0 reach 6 m and the
    /// run goes on to 60 m; a start at 100 km is a start; one step of the
    /// ceiling, 2^22 s, rolls 252 km and closes its ledger, the scale
    /// following the work.
    #[test]
    fn the_end_of_the_track_is_not_an_end() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut frame = blank();
        let mut state = start(&tire, 0.0);
        let mut worst = 0.0_f64;
        for _ in 0..10_000 {
            step(&tire, &mut state, &mut frame, rig(0.10), 0.1).unwrap();
            worst = worst.max(state.residual_conservation_error);
        }
        assert!(relative(state.distance_m, 60.0) < 1.0e-12, "{}", state.distance_m);
        assert!(worst < 1.0e-11, "{worst:e}");
        let mut far = TireState::try_start(&tire, 100_000.0, 5.0).unwrap();
        step(&tire, &mut far, &mut frame, rig(-0.20), 10.0).unwrap();
        assert!(relative(far.distance_m, 100_000.6) < 1.0e-15);
        let mut longest = start(&tire, 0.0);
        step(&tire, &mut longest, &mut frame, rig(0.20), GATE_DT_S).unwrap();
        assert!(relative(longest.distance_m, 0.06 * GATE_DT_S) < 1.0e-15);
        assert!(longest.wheel_work_j > tire.ledger_scale_j());
        assert_eq!(tire.run_ledger_scale_j(&longest), longest.wheel_work_j);
        assert!(longest.residual_conservation_error < 1.0e-15);
        assert_eq!(frame.flags & FLAG_BEYOND_COMPARED, FLAG_BEYOND_COMPARED);
    }

    /// Steps the 64-bit sums cannot resolve are refused by name, and the
    /// state and the frame are left as they were: a time held of 1e300 s,
    /// and a distance of 1e300 m.
    #[test]
    fn a_step_the_sums_cannot_resolve_is_refused_by_name() {
        let tire = TireEveR19Ice::on_rig_ice();
        let mut frame = blank();
        for (distance_m, held_s) in [(0.0, 1.0e300), (1.0e300, 5.0), (0.0, (2.0_f64).powi(60))] {
            let mut state = TireState::try_start(&tire, distance_m, held_s).unwrap();
            let kept = state;
            assert_eq!(
                step(&tire, &mut state, &mut frame, rig(0.10), tire.floor_dt_s().unwrap()),
                Err(StepReject::StepTooSmallToResolve)
            );
            assert_eq!(state, kept);
            assert_eq!(frame.to_bytes(), blank().to_bytes());
        }
        // At 2^42 s held, a floor step still moves the time held.
        let mut edge = TireState::try_start(&tire, 0.0, (2.0_f64).powi(42)).unwrap();
        step(&tire, &mut edge, &mut frame, rig(0.10), tire.floor_dt_s().unwrap()).unwrap();
        assert!(edge.slip_held_s > (2.0_f64).powi(42));
    }

    /// Past the old wall at 5 s held: a run from the instant its slip is set
    /// lands every step, the frames of the first 5 s carry
    /// [`FLAG_SLIP_SETTLING`] and none after, the force is the steady force
    /// throughout, the ledger closes at every step, and a falsified heat
    /// inside those 5 s is named both ways with the state and the frame kept.
    /// Changes of slip every 2 s, so that no step is ever settled, land as
    /// well.
    #[test]
    fn in_the_first_5_seconds_after_a_slip_is_set_the_step_lands_flagged_and_the_ledger_closes() {
        let tire = TireEveR19Ice::on_rig_ice();
        for (slip, normal_load_n) in [(0.15, 5_000.0), (-0.05, 6_500.0), (0.20, 3_500.0), (-0.12, 9_000.0)] {
            let load = TireLoad { normal_load_n, ..rig(slip) };
            let mut state = TireState::try_start(&tire, 0.0, 0.0).unwrap();
            let mut frame = blank();
            let dt_s = 1.0 / 64.0;
            for index in 0..640 {
                let held_before = state.slip_held_s;
                step(&tire, &mut state, &mut frame, load, dt_s).unwrap();
                assert_eq!(frame.flags & FLAG_SLIP_SETTLING != 0, held_before < SLIP_SETTLE_S, "{slip}: step {index}");
                assert_eq!(state.longitudinal_force_n, tire.longitudinal_force_n(normal_load_n, slip));
                assert!(state.residual_conservation_error < 1.0e-13, "{slip}: step {index}");
            }
            assert_eq!(state.slip_held_s, 10.0);
            // A forged heat at 1 s held is named both ways.
            let mut early = TireState::try_start(&tire, 0.0, 1.0).unwrap();
            step(&tire, &mut early, &mut frame, load, 0.5).unwrap();
            assert_eq!(frame.flags & FLAG_SLIP_SETTLING, FLAG_SLIP_SETTLING);
            let honest = held(&tire, load, 0.5);
            let scale = tire.run_ledger_scale_j(&early);
            for (bias, want) in [
                (2.0 * RESIDUAL_CONSERVATION_LIMIT * scale, StepReject::SpontaneousGeneration),
                (-2.0 * RESIDUAL_CONSERVATION_LIMIT * scale, StepReject::FirstLawBreach),
            ] {
                let forged = Advance { slip_heat_j: honest.slip_heat_j + bias, ..honest };
                let mut trial = early;
                let mut trial_frame = frame;
                assert_eq!(commit(&tire, &mut trial, &mut trial_frame, load, 0.5, forged), Err(want));
                assert_eq!(trial, early);
                assert_eq!(trial_frame.to_bytes(), frame.to_bytes());
            }
        }
        // A slip changed every 2 s, driving and braking in turn: no step is
        // ever 5 s from its change, every frame carries the flag, and the
        // ledger closes over 1,000 changes.
        let mut state = TireState::try_start(&tire, 0.0, 0.0).unwrap();
        let mut frame = blank();
        for change in 0..1_000 {
            let slip = if change % 2 == 0 { 0.07 } else { -0.15 };
            for _ in 0..4 {
                step(&tire, &mut state, &mut frame, rig(slip), 0.5).unwrap();
                assert_eq!(frame.flags & FLAG_SLIP_SETTLING, FLAG_SLIP_SETTLING);
            }
            assert_eq!(state.slip_held_s, 2.0);
        }
        assert!(state.residual_conservation_error < 1.0e-13, "{}", state.residual_conservation_error);
        assert!(relative(state.t_s, 2_000.0) < 1.0e-12);
    }

    /// The load of the card past the loads measured: at 50 kN and at 1 N the
    /// force is the same law and the ledger closes; at 50 kN over a long run
    /// the scale is the work, and a falsified heat is named both ways.
    #[test]
    fn past_the_measured_loads_the_ledger_closes_and_a_falsified_heat_is_named() {
        let tire = TireEveR19Ice::on_rig_ice();
        for normal_load_n in [1.0, 50_000.0, 1.0e8] {
            let load = TireLoad { normal_load_n, ..rig(-0.07) };
            let mut state = start(&tire, 0.0);
            let mut frame = blank();
            for _ in 0..2_000 {
                step(&tire, &mut state, &mut frame, load, 0.5).unwrap();
            }
            assert!(state.residual_conservation_error < 1.0e-13, "{normal_load_n}");
            assert_eq!(state.longitudinal_force_n, -(tire.friction_ratio(-0.07) * normal_load_n));
            let honest = held(&tire, load, 0.5);
            let scale = tire.run_ledger_scale_j(&state);
            for (bias, want) in [
                (2.0 * RESIDUAL_CONSERVATION_LIMIT * scale, StepReject::SpontaneousGeneration),
                (-2.0 * RESIDUAL_CONSERVATION_LIMIT * scale, StepReject::FirstLawBreach),
            ] {
                let forged = Advance { slip_heat_j: honest.slip_heat_j + bias, ..honest };
                let mut trial = state;
                let mut trial_frame = frame;
                assert_eq!(commit(&tire, &mut trial, &mut trial_frame, load, 0.5, forged), Err(want));
                assert_eq!(trial, state);
                assert_eq!(trial_frame.to_bytes(), frame.to_bytes());
            }
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

    /// One hold at the step floor and the slowest speed, 16,384 steps to
    /// 21 s and 16,384 more past it: the residual stays under a hundredth of
    /// its limit.
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
        let mut worst = 0.0_f64;
        for landed in 1..=32_768 {
            step(&tire, &mut state, &mut frame, load, floor).unwrap();
            worst = worst.max(state.residual_conservation_error);
            if landed == 16_384 {
                assert_eq!(state.slip_held_s, 21.0);
                assert_eq!(frame.flags & FLAG_BEYOND_COMPARED, 0);
            }
        }
        assert_eq!(state.slip_held_s, 37.0);
        assert_eq!(frame.flags & FLAG_BEYOND_COMPARED, FLAG_BEYOND_COMPARED);
        assert!(worst < 1.0e-11, "{worst}");
    }
}
