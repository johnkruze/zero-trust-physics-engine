#![no_implicit_prelude]

//! The crater made on asteroid Ryugu by Hayabusa2's Small Carry-on Impactor.
//!
//! On 5 April 2019 the Small Carry-on Impactor (SCI) of JAXA's Hayabusa2 fired
//! a 2 kg copper projectile at about 2 km/s into the surface of asteroid
//! 162173 Ryugu near 6 degrees north, 303 degrees east. Ryugu is a pile of
//! rubble held together by its own gravity, about 9.3e-5 m/s² at that place.
//! The card steps the radius of the growing crater, measured at the level of
//! the undisturbed surface, from 0.1622 m until it stops.
//!
//! The model is the law of crater size for a dry granular bed with no
//! cohesion, held by gravity alone, fitted on Earth in sand. NOTHING IN THIS
//! CARD IS FITTED TO RYUGU'S CRATER. The card stops at 8.571 m. The measured
//! crater has an apparent radius of 7.25 ± 0.4 m (diameter 14.5 ± 0.8 m,
//! Arakawa et al. 2020, Science 368, 67, as given in JAXA's summary of
//! 2020-03-20). Run through the gateway on 2026-10-09, the card is 1.32 m, or
//! 18.2 percent, above the measurement, against an allowance of 0.97 m fixed
//! by rule beforehand: a miss. In the laboratory the same law, fitted at 1 g,
//! met both of its lines on four centrifuge craters it had not seen, 7.9
//! percent low on average. The record is `research/crater_ryugu_sci/` in the
//! evidence folder.
//!
//! Where the numbers come from:
//!
//! - The projectile: pure copper, 2 kg, 2 km/s (JAXA press briefing of
//!   2019-03-18, before the impact). Copper 8960 kg/m³. Its equivalent solid
//!   sphere has a radius `a` of 0.037631 m.
//! - Ryugu: GM 30.0 m³/s², bulk density 1190 kg/m³, rotation period
//!   7.63262 h, equatorial diameter 1.004 km (JPL Small-Body Database, citing
//!   Watanabe et al. 2019). Gravity at the place is
//!   `GM/r² − ω² r cos²(lat)` at `r` = 502 m and 6 degrees: 9.3084e-5 m/s².
//! - The law of size, `r (ρ/W)^(1/3) = A · (3.22 g a / U²)^(−α)`, with
//!   `A` = 0.8708204315358979 and `α` = 0.1660599800702854 fitted by least
//!   squares to thirteen impact craters in dense dry Ottawa Flintshot sand at 1, 111 and 464 times
//!   Earth's gravity (Schmidt 1980, Proc. Lunar Planet. Sci. Conf. 11, 2099,
//!   Table 1), typed as the fit gave them in 64 bits. Ryugu's `3.22 g a / U²`
//!   is 2.82e-12, 2.5 decades below the smallest in that table: an
//!   extrapolation.
//! - The clock: the time to form a crater is `1.318295773573555 · sqrt(r/g)`, the
//!   least-squares constant over seven points read by eye from Fig. 4 of Gault
//!   and Wedekind (1977), craters in quartz sand at 1 g. Their time runs until
//!   the base of the ejecta plume is 1.25 rim diameters wide, so it is later
//!   than the moment the crater stops growing. The card's time is that clock.
//!   On Ryugu the crater was seen to grow for about 200 s and the whole event
//!   lasted about 500 s; the card stops 400 s after the impact. No pass mark
//!   was set on time.
//!
//! What is left out: cohesion and strength, the boulders (the measured crater
//! is a half circle, stopped on its south side by a block), the 60 degree
//! angle of the impact, the projectile's hollow shape, the rim, the depth,
//! the ejecta. Only the final radius was compared with Ryugu; the growth and
//! its clock were not compared with anything.
//!
//! The smallest radius is set by the ledger's arithmetic. Near the impact the
//! wall is fast and the energy it has yet to give up, `−W(R)`, is large, while
//! the ledger's limit is a fixed 5.064e-11 J/kg. At [`RADIUS_FLOOR_M`],
//! 0.1622 m, `−W` is 22 800 J/kg, the wall moves at 214 m/s, and one rounding
//! of `W` in 64 bits, one part in 2^52, is a tenth of the ledger's limit:
//! closer to the impact the ledger can no longer tell a right step from a
//! wrong one, and the start is [`StepReject::RadiusBelowLedgerResolution`].
//! That radius is 4.3 radii of the projectile's equivalent sphere, outside the
//! 1.2 radii from which point-source laws of cratering are used (Housen and
//! Holsapple 2011, Icarus 211, 856), and the wall there is a tenth of the
//! projectile's 2 km/s. From the floor the wall runs to its stop in steps that
//! grow with the radius. At the ceiling of 0.25 s a step below about 1.39 m
//! is past the ledger's limit when taken whole; since engine 0.15.2 such a
//! step is taken again in 2, 4, 8, ... equal Runge–Kutta pieces, each closed
//! on the same ledger, up to 2^20 of them, and lands (through 0.15.1 the
//! ledger refused it, and a smaller step landed). A step that lands whole
//! lands as it always did. From the floor at 0.25 s a run lands 1,601 steps
//! to the stop, at 400.25 s. The card served from 1.75 m
//! until engine 0.9; that floor had no recorded reason.
//!
//! The equations. `R` is the crater radius, `V` the speed of its wall,
//! `Rf` = 8.571 m the radius the law of size gives, `p = (1 − α)/α` = 5.0219,
//! and `kg` = 5.908e-6 m/s² the clock's share of gravity. Per unit mass,
//!
//! ```text
//!   W(R) = kg · Rf · [ R/Rf − (R/Rf)^(−p) ]          energy the wall has given up
//!   ½ V² + W(R) = 0                                   held at every step
//!   dV/dt = −kg · [ 1 + p · (R/Rf)^(−p−1) ],  dR/dt = V
//! ```
//!
//! Far below `Rf` the wall follows the point-source power law
//! `V ∝ R^(−p/2)`; gravity's term stops it at `Rf`, where `W` = 0. The step is
//! classical fourth-order Runge–Kutta on `q = (R/Rf)^m`, `m = p/2 + 1`, in
//! which the same equations read `d²q/dt² = −(m (p+1) kg / Rf) · q^((m−1)/m)`
//! and the early growth is a straight line. Powers are `exp(e · log(x))` with
//! the crate's own functions. When the trial step turns the wall back, the
//! ledger is closed on the trial point and the crater is set at `Rf` with the
//! wall at rest; from then on a step moves only the time and the frame
//! carries [`FLAG_STOPPED`].
//!
//! The ledger: `|½ V² + W(R)|` divided by `kg · Rf` = 5.064e-5 J/kg must stay
//! at or under [`RESIDUAL_CONSERVATION_LIMIT`]. Over the whole growth at the
//! step ceiling of 0.25 s it reaches 5.8e-8.
//!
//! A step writes the time, the state and the flag into a
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Mass of the copper projectile, kg. JAXA briefing of 2019-03-18.
pub const PROJECTILE_MASS_KG: f64 = 2.0;
/// Speed of the projectile, m/s. JAXA briefing of 2019-03-18.
pub const PROJECTILE_SPEED_M_S: f64 = 2000.0;
/// Density of copper, kg/m³.
pub const PROJECTILE_DENSITY_KG_M3: f64 = 8960.0;
/// Ryugu's bulk density, kg/m³. JPL Small-Body Database.
pub const RYUGU_BULK_DENSITY_KG_M3: f64 = 1190.0;
/// Gravity at the place of the impact, with Ryugu's rotation, m/s².
pub const SITE_GRAVITY_M_S2: f64 = 9.30836264495937e-5;
/// `A` of the law of size. Fitted to thirteen sand craters of Schmidt (1980),
/// as the fit gave it in 64 bits (`analysis/out/04_forecast.json`).
pub const SCALING_PREFACTOR: f64 = 0.8708204315358979;
/// `α` of the law of size. Fitted to the same thirteen craters, as the fit
/// gave it in 64 bits.
pub const SCALING_EXPONENT: f64 = 0.1660599800702854;
/// Formation time over `sqrt(radius / gravity)`. The least-squares constant
/// over seven points read by eye from Gault and Wedekind (1977), Fig. 4, as
/// the fit gave it in 64 bits.
pub const CLOCK_CONSTANT: f64 = 1.318295773573555;
/// `Rf`: the radius the law of size gives for this impact, m. Worked out from
/// the constants above in 50-digit arithmetic and copied. It agrees with the
/// forecast of the research record, 8.571304329477925 m, to 4e-16.
pub const FINAL_RADIUS_M: f64 = 8.571304329477922;
/// `p = (1 − α)/α`. Copied from the same working.
pub const GROWTH_POWER: f64 = 5.021920510750073;
/// `kg`: the clock's share of gravity, m/s². `kg = g · J² / (2 · cT²)`, with
/// `J = B((p/2+1)/(p+1), ½)/(p+1)` = 0.469702. Copied from the same working.
pub const GRAVITY_WORK_M_S2: f64 = 5.9082861135460726e-6;
/// Smallest radius the card accepts, m: where one rounding of `W(R)` in 64
/// bits, one part in 2^52 of 22 807 J/kg, is a tenth of the ledger's limit,
/// 0.16213973 m, rounded up at the fourth decimal. The wall's speed there is
/// 213.6 m/s.
pub const RADIUS_FLOOR_M: f64 = 0.1622;
/// Step ceiling, s. About 1600 steps from the floor to the stop.
pub const GATE_DT_S: f64 = 0.25;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;
/// `|½ V² + W(R)| / (kg · Rf)` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;

/// The crater has stopped growing: its radius is [`FINAL_RADIUS_M`] and its
/// wall is at rest.
pub const FLAG_STOPPED: u64 = 1 << 0;

/// Why a step did not advance the crater or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or a stage of the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this crater's.
    InvalidCrater,
    /// The radius is under 0.1622 m: there the energy the wall has yet to give up is so large that one rounding of it in 64 bits passes a tenth of the ledger's limit, and the ledger cannot tell a right step from a wrong one.
    RadiusBelowLedgerResolution,
    /// The radius is above 8.571304329477922 m, where this crater stops.
    RadiusAboveFinal,
    /// The wall's speed is below zero. A crater does not shrink in this model.
    WallMovingInward,
    /// The trial step created energy: the wall's kinetic energy plus the
    /// energy it has given up rose above zero.
    SpontaneousGeneration,
    /// The trial step destroyed energy: the wall's kinetic energy plus the
    /// energy it has given up fell below zero.
    FirstLawBreach,
}

/// Nothing acts on the crater but the impact and Ryugu. The load is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CraterLoad {}

/// The locked crater. Every field is a recorded copy of a crate constant and
/// none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraterRyuguSci {
    /// Recorded copy of [`FINAL_RADIUS_M`].
    pub final_radius_m: f64,
    /// Recorded copy of [`GROWTH_POWER`].
    pub growth_power: f64,
    /// Recorded copy of [`GRAVITY_WORK_M_S2`].
    pub gravity_work_m_s2: f64,
    /// Recorded copy of [`RADIUS_FLOOR_M`].
    pub radius_floor_m: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`GATE_OVER_FLOOR`].
    pub gate_over_floor: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
}

impl CraterRyuguSci {
    /// Card: the crater of the Small Carry-on Impactor on Ryugu. The
    /// constructor takes no arguments.
    #[must_use]
    pub fn sci_impact() -> Self {
        let crater = Self {
            final_radius_m: FINAL_RADIUS_M,
            growth_power: GROWTH_POWER,
            gravity_work_m_s2: GRAVITY_WORK_M_S2,
            radius_floor_m: RADIUS_FLOOR_M,
            gate_dt_s: GATE_DT_S,
            gate_over_floor: GATE_OVER_FLOOR,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
        };
        if crater.is_valid() {
            crater
        } else {
            ::core::panic!("crater failed is_valid");
        }
    }

    /// Every recorded copy equals its crate constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.final_radius_m.to_bits() == FINAL_RADIUS_M.to_bits()
            && self.growth_power.to_bits() == GROWTH_POWER.to_bits()
            && self.gravity_work_m_s2.to_bits() == GRAVITY_WORK_M_S2.to_bits()
            && self.radius_floor_m.to_bits() == RADIUS_FLOOR_M.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.final_radius_m > self.radius_floor_m
            && self.radius_floor_m > 0.0
            && self.growth_power > 0.0
            && self.gravity_work_m_s2 > 0.0
            && self.gate_dt_s > 0.0
            && self.gate_over_floor > 1.0
            && self.residual_conservation_limit > 0.0
    }

    /// `m = p/2 + 1`, the power of the stepping variable.
    #[must_use]
    pub fn stepping_power(&self) -> f64 {
        0.5 * self.growth_power + 1.0
    }

    /// `W(R) = kg · Rf · [ R/Rf − (R/Rf)^(−p) ]`, J/kg: the energy per unit
    /// mass the wall has given up by radius `R`. Negative below `Rf`, exactly
    /// zero at `Rf`.
    #[must_use]
    pub fn given_up_j_per_kg(&self, radius_m: f64) -> f64 {
        let x = radius_m / self.final_radius_m;
        self.gravity_work_m_s2 * self.final_radius_m * (x - raised(x, -self.growth_power))
    }

    /// The wall's speed at radius `R` in this impact, `sqrt(−2 W(R))`, m/s.
    /// Zero at `Rf` and above it.
    #[must_use]
    pub fn wall_speed_m_s(&self, radius_m: f64) -> f64 {
        let twice = -2.0 * self.given_up_j_per_kg(radius_m);
        if twice > 0.0 { f64::sqrt(twice) } else { 0.0 }
    }

    /// The ledger's scale, `kg · Rf`, J/kg. 5.0641718e-5 for this crater.
    #[must_use]
    pub fn ledger_scale_j_per_kg(&self) -> f64 {
        self.gravity_work_m_s2 * self.final_radius_m
    }

    /// Largest step the card stands behind: [`GATE_DT_S`], 0.25 s. Not applied
    /// by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.gate_dt_s.is_finite() && self.gate_dt_s > 0.0 {
            Some(self.gate_dt_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20, 2.384186e-7 s. Not applied by [`step`].
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
/// the crate's own exponential and logarithm. `size` must be positive; zero
/// or less gives NaN, which the step names.
fn raised(size: f64, exponent: f64) -> f64 {
    if !(size > 0.0) {
        return f64::NAN;
    }
    crate::math::exp(exponent * crate::math::log(size))
}

/// The growing crater. Write it only through [`CraterState::try_radius`] and
/// [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CraterState {
    /// Time since the start of the run.
    pub t_s: f64,
    /// Crater radius at the level of the undisturbed surface, `R`.
    pub radius_m: f64,
    /// Speed of the crater wall, `V`.
    pub wall_speed_m_s: f64,
    /// `R / Rf`.
    pub radius_over_final: f64,
    /// `Rf − R`: how far the radius still has to grow.
    pub radius_to_go_m: f64,
    /// `½ V²`.
    pub kinetic_j_per_kg: f64,
    /// `W(R)`.
    pub given_up_j_per_kg: f64,
    /// Signed `½ V² + W(R)`. Positive is created energy.
    pub energy_defect_j_per_kg: f64,
    /// `|energy_defect_j_per_kg| / (kg · Rf)`.
    pub residual_conservation_error: f64,
    /// The crater has stopped at `Rf`.
    pub stopped: bool,
}

impl CraterState {
    /// The crater of this impact at radius `radius_m`, time 0: the wall's
    /// speed is the one the impact gives it there. `None` for a radius that is
    /// not finite, under [`RADIUS_FLOOR_M`] or above [`FINAL_RADIUS_M`].
    #[must_use]
    pub fn try_radius(crater: &CraterRyuguSci, radius_m: f64) -> Option<Self> {
        if !crater.is_valid() || !radius_m.is_finite() {
            return None;
        }
        if radius_m < crater.radius_floor_m || radius_m > crater.final_radius_m {
            return None;
        }
        let mut state = Self {
            t_s: 0.0,
            radius_m,
            wall_speed_m_s: crater.wall_speed_m_s(radius_m),
            radius_over_final: 0.0,
            radius_to_go_m: 0.0,
            kinetic_j_per_kg: 0.0,
            given_up_j_per_kg: 0.0,
            energy_defect_j_per_kg: 0.0,
            residual_conservation_error: 0.0,
            stopped: false,
        };
        refresh(crater, &mut state);
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }
}

/// Advance one Runge–Kutta step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were. A stage of the
/// step that is not finite is [`StepReject::NonFiniteState`].
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    crater: &CraterRyuguSci,
    state: &mut CraterState,
    frame: &mut LastStateFrame64,
    load: CraterLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(crater, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("crater_ryugu_sci", exceedances(crater, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    crater: &CraterRyuguSci,
    state: &mut CraterState,
    frame: &mut LastStateFrame64,
    load: CraterLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(crater, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    crater: &CraterRyuguSci,
    state: &mut CraterState,
    frame: &mut LastStateFrame64,
    load: CraterLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(crater, state, load, dt_s)?;
    // The step whole, as through engine 0.15.1: a step that lands whole
    // lands as it always did.
    let whole = piece(crater, state, frame, dt_s);
    if !ledger_refused(whole) || !(dt_s <= crater.gate_dt_s) {
        return whole;
    }
    // The ledger did not close on a step the card stands behind, one at or
    // under its ceiling: the Runge-Kutta error of a step this long at this
    // wall speed is past the limit. The step is taken again in 2, 4, 8, ...
    // equal pieces, each a Runge-Kutta step closed on the same ledger, until
    // one count lands, or the pieces would number more than GATE_OVER_FLOOR,
    // 2^20 (a piece of a step at the ceiling no shorter than the floor). The
    // ledger's limit is not changed. A step above the ceiling is not the
    // card's and is named by the ledger whole, as before.
    let mut refused = whole;
    let mut pieces: f64 = 2.0;
    while pieces <= crater.gate_over_floor {
        let mut trial = *state;
        let mut trial_frame = *frame;
        let mut taken: f64 = 0.0;
        let mut landed = Ok(());
        while taken < pieces {
            // Each piece ends on t_start + dt · (k + 1) / n, the last exactly
            // on t_start + dt.
            let end_s = state.t_s + dt_s * ((taken + 1.0) / pieces);
            let width_s = end_s - trial.t_s;
            landed = piece(crater, &mut trial, &mut trial_frame, width_s);
            if landed.is_err() {
                break;
            }
            trial.t_s = end_s;
            trial_frame.t = end_s;
            taken += 1.0;
        }
        match landed {
            Ok(()) => {
                *state = trial;
                *frame = trial_frame;
                return Ok(());
            }
            Err(_) if ledger_refused(landed) => refused = landed,
            Err(reject) => return Err(reject),
        }
        pieces *= 2.0;
    }
    refused
}

/// The ledger's two refusals: the trial step created or destroyed energy.
fn ledger_refused(outcome: Result<(), StepReject>) -> bool {
    core::matches!(outcome, Err(StepReject::SpontaneousGeneration | StepReject::FirstLawBreach))
}

/// One Runge-Kutta step of `dt_s` from `state`, as [`commit`] lands it, or,
/// from the stopped crater, the clock alone.
fn piece(
    crater: &CraterRyuguSci,
    state: &mut CraterState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
) -> Result<(), StepReject> {
    if is_stopped(crater, state) {
        return commit(crater, state, frame, dt_s, [state.radius_m, 0.0]);
    }
    let advanced = match rk4(crater, [state.radius_m, state.wall_speed_m_s], dt_s) {
        Some(y) => y,
        None => return Err(StepReject::NonFiniteState),
    };
    commit(crater, state, frame, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut CraterState) {
    state.t_s = normal_or_zero(state.t_s);
    state.radius_m = normal_or_zero(state.radius_m);
    state.wall_speed_m_s = normal_or_zero(state.wall_speed_m_s);
    state.radius_over_final = normal_or_zero(state.radius_over_final);
    state.radius_to_go_m = normal_or_zero(state.radius_to_go_m);
    state.kinetic_j_per_kg = normal_or_zero(state.kinetic_j_per_kg);
    state.given_up_j_per_kg = normal_or_zero(state.given_up_j_per_kg);
    state.energy_defect_j_per_kg = normal_or_zero(state.energy_defect_j_per_kg);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

fn validate(
    crater: &CraterRyuguSci,
    state: &CraterState,
    _load: CraterLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !crater.is_valid() {
        return Err(StepReject::InvalidCrater);
    }
    if !(state.radius_m >= crater.radius_floor_m) {
        return Err(StepReject::RadiusBelowLedgerResolution);
    }
    if !(state.radius_m <= crater.final_radius_m) {
        return Err(StepReject::RadiusAboveFinal);
    }
    // The books are closed again on the radius and the speed as they stand;
    // the recorded copies in the state are not trusted.
    if state.wall_speed_m_s < 0.0 {
        return Err(StepReject::WallMovingInward);
    }
    let mut held = *state;
    refresh(crater, &mut held);
    close_ledger(crater, &held)
}

/// `Ok` when `½ V² + W(R)` of `state` is inside the limit.
fn close_ledger(crater: &CraterRyuguSci, state: &CraterState) -> Result<(), StepReject> {
    let gate = crater.residual_conservation_limit * crater.ledger_scale_j_per_kg();
    if !state.energy_defect_j_per_kg.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if state.energy_defect_j_per_kg > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j_per_kg < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    Ok(())
}

/// Take the trial radius and speed. A trial speed of zero or less, or a trial
/// radius at `Rf` or past it, means the wall stopped inside the step: the
/// ledger is closed on the trial point and the crater is then set at `Rf`, at
/// rest.
fn commit(
    crater: &CraterRyuguSci,
    state: &mut CraterState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
    advanced: [f64; 2],
) -> Result<(), StepReject> {
    let saved = *state;
    if !(advanced[0].is_finite() && advanced[1].is_finite()) {
        return Err(StepReject::NonFiniteState);
    }
    state.t_s += dt_s;
    state.radius_m = advanced[0];
    state.wall_speed_m_s = advanced[1];
    refresh(crater, state);
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if let Err(reject) = close_ledger(crater, state) {
        *state = saved;
        return Err(reject);
    }
    if state.wall_speed_m_s <= 0.0 || state.radius_m >= crater.final_radius_m {
        state.radius_m = crater.final_radius_m;
        state.wall_speed_m_s = 0.0;
        refresh(crater, state);
    }
    if !(state.radius_m <= crater.final_radius_m) {
        *state = saved;
        return Err(StepReject::RadiusAboveFinal);
    }
    if !(state.radius_m >= crater.radius_floor_m) {
        *state = saved;
        return Err(StepReject::RadiusBelowLedgerResolution);
    }
    publish(state, frame);
    Ok(())
}

/// The energies, the ratio and the stopped mark of the radius and the speed
/// now in the state.
fn refresh(crater: &CraterRyuguSci, state: &mut CraterState) {
    state.radius_over_final = state.radius_m / crater.final_radius_m;
    state.radius_to_go_m = crater.final_radius_m - state.radius_m;
    state.kinetic_j_per_kg = 0.5 * state.wall_speed_m_s * state.wall_speed_m_s;
    state.given_up_j_per_kg = crater.given_up_j_per_kg(state.radius_m);
    state.energy_defect_j_per_kg = state.kinetic_j_per_kg + state.given_up_j_per_kg;
    state.residual_conservation_error =
        state.energy_defect_j_per_kg.abs() / crater.ledger_scale_j_per_kg();
    state.stopped = is_stopped(crater, state);
}

/// The wall is at rest at `Rf`.
fn is_stopped(crater: &CraterRyuguSci, state: &CraterState) -> bool {
    state.wall_speed_m_s == 0.0 && state.radius_m.to_bits() == crater.final_radius_m.to_bits()
}

fn publish(state: &CraterState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.radius_m as f32;
    frame.pos[1] = state.wall_speed_m_s as f32;
    frame.pos[2] = state.radius_over_final as f32;
    frame.vel[0] = state.kinetic_j_per_kg as f32;
    frame.vel[1] = state.given_up_j_per_kg as f32;
    frame.vel[2] = state.energy_defect_j_per_kg as f32;
    frame.force_torque = state.radius_to_go_m as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.stopped { FLAG_STOPPED } else { 0 };
}

/// `d²q/dt²` at `q = (R/Rf)^m`: `−(m (p+1) kg / Rf) · q^((m−1)/m)`. `None`
/// when `q` is not positive or the rate is not finite.
fn pull(crater: &CraterRyuguSci, q: f64) -> Option<f64> {
    if !(q > 0.0) {
        return None;
    }
    let m = crater.stepping_power();
    let out = -(m * (crater.growth_power + 1.0) * crater.gravity_work_m_s2 / crater.final_radius_m)
        * raised(q, (m - 1.0) / m);
    if out.is_finite() { Some(out) } else { None }
}

/// Classical fourth-order Runge–Kutta on `q` and its rate, from radius and
/// wall speed to radius and wall speed. A trial speed of zero or less is
/// returned as it is; [`commit`] reads it as the stop.
fn rk4(crater: &CraterRyuguSci, y: [f64; 2], dt_s: f64) -> Option<[f64; 2]> {
    let m = crater.stepping_power();
    let x = y[0] / crater.final_radius_m;
    let q = raised(x, m);
    let q_rate = m * raised(x, m - 1.0) * y[1] / crater.final_radius_m;
    let k1 = [q_rate, pull(crater, q)?];
    let k2 = [q_rate + 0.5 * dt_s * k1[1], pull(crater, q + 0.5 * dt_s * k1[0])?];
    let k3 = [q_rate + 0.5 * dt_s * k2[1], pull(crater, q + 0.5 * dt_s * k2[0])?];
    let k4 = [q_rate + dt_s * k3[1], pull(crater, q + dt_s * k3[0])?];
    let q_next = q + dt_s / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
    let q_rate_next = q_rate + dt_s / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
    if !(q_next > 0.0) || !q_rate_next.is_finite() {
        return None;
    }
    let x_next = raised(q_next, 1.0 / m);
    let radius_m = crater.final_radius_m * x_next;
    let wall_speed_m_s = q_rate_next * crater.final_radius_m / (m * raised(x_next, m - 1.0));
    if radius_m.is_finite() && wall_speed_m_s.is_finite() {
        Some([radius_m, wall_speed_m_s])
    } else {
        None
    }
}

fn state_is_finite(state: &CraterState) -> bool {
    state.t_s.is_finite()
        && state.radius_m.is_finite()
        && state.wall_speed_m_s.is_finite()
        && state.radius_over_final.is_finite()
        && state.radius_to_go_m.is_finite()
        && state.kinetic_j_per_kg.is_finite()
        && state.given_up_j_per_kg.is_finite()
        && state.energy_defect_j_per_kg.is_finite()
        && state.residual_conservation_error.is_finite()
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    STOPPED_KIND,
];

const STOPPED_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_STOPPED",
    FLAG_STOPPED,
    "radius_m",
    "m",
    Side::AtOrAbove,
    FINAL_RADIUS_M,
    "the crater's final radius, reached with its wall at rest: the event the card ends on; the extent is zero",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
pub fn exceedances(
    crater: &CraterRyuguSci,
    before: &CraterState,
    after: &CraterState,
    load: CraterLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, load, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.stopped, Exceedance::at(STOPPED_KIND, crater.final_radius_m, after.radius_m));
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::*;

    const NO_LOAD: CraterLoad = CraterLoad {};

    fn blank_frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: -1.0,
            pos: [-1.0; 3],
            vel: [-1.0; 3],
            force_torque: -1.0,
            residual: -1.0,
            flags: 0xdead_beef,
            proof: [0xab; 16],
        }
    }

    fn proof_intact(frame: &LastStateFrame64) {
        assert_eq!(frame.proof, [0xab; 16]);
    }

    fn assert_copy<T: Copy>() {}

    fn state_bits(state: &CraterState) -> [u64; 10] {
        [
            state.t_s.to_bits(),
            state.radius_m.to_bits(),
            state.wall_speed_m_s.to_bits(),
            state.radius_over_final.to_bits(),
            state.radius_to_go_m.to_bits(),
            state.kinetic_j_per_kg.to_bits(),
            state.given_up_j_per_kg.to_bits(),
            state.energy_defect_j_per_kg.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.stopped as u64,
        ]
    }

    fn within(got: f64, expected: f64, tolerance: f64) -> bool {
        (got - expected).abs() <= tolerance * expected.abs()
    }

    /// The start the research record's reference run was made from, 1.75 m,
    /// the card's floor until engine 0.9.
    const REFERENCE_START_M: f64 = 1.75;

    fn start() -> (CraterRyuguSci, CraterState) {
        let crater = CraterRyuguSci::sci_impact();
        let state = CraterState::try_radius(&crater, REFERENCE_START_M).unwrap();
        (crater, state)
    }

    /// The real `validate`, a chosen integrator, the real `commit`.
    fn step_with(
        crater: &CraterRyuguSci,
        state: &mut CraterState,
        frame: &mut LastStateFrame64,
        dt_s: f64,
        integrate: impl FnOnce(&CraterRyuguSci, [f64; 2], f64) -> Option<[f64; 2]>,
    ) -> Result<(), StepReject> {
        validate(crater, state, NO_LOAD, dt_s)?;
        let advanced = match integrate(crater, [state.radius_m, state.wall_speed_m_s], dt_s) {
            Some(y) => y,
            None => return Err(StepReject::NonFiniteState),
        };
        commit(crater, state, frame, dt_s, advanced)
    }

    /// Explicit Euler on `q` and its rate.
    fn euler(crater: &CraterRyuguSci, y: [f64; 2], dt_s: f64) -> Option<[f64; 2]> {
        let m = crater.stepping_power();
        let x = y[0] / crater.final_radius_m;
        let q = raised(x, m);
        let q_rate = m * raised(x, m - 1.0) * y[1] / crater.final_radius_m;
        let q_next = q + dt_s * q_rate;
        let q_rate_next = q_rate + dt_s * pull(crater, q)?;
        let x_next = raised(q_next, 1.0 / m);
        Some([
            crater.final_radius_m * x_next,
            q_rate_next * crater.final_radius_m / (m * raised(x_next, m - 1.0)),
        ])
    }

    #[test]
    fn the_card_locks_its_constants() {
        let crater = CraterRyuguSci::sci_impact();
        assert!(crater.is_valid());
        assert_eq!(crater.final_radius_m.to_bits(), 8.571304329477922_f64.to_bits());
        assert_eq!(crater.growth_power.to_bits(), 5.021920510750073_f64.to_bits());
        assert_eq!(crater.gravity_work_m_s2.to_bits(), 5.9082861135460726e-6_f64.to_bits());
        assert_eq!(crater.radius_floor_m, 0.1622);
        assert_eq!(crater.gate_dt_s, 0.25);
        assert_eq!(crater.residual_conservation_limit, 1.0e-6);
        assert!(within(crater.ledger_scale_j_per_kg(), 5.064171834483174e-5, 1.0e-15));
        // One altered copy is not this crater, and the step names it.
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        for index in 0..7 {
            let mut other = crater;
            match index {
                0 => other.final_radius_m = 7.25,
                1 => other.growth_power = 5.0,
                2 => other.gravity_work_m_s2 *= 2.0,
                3 => other.radius_floor_m = 0.16,
                4 => other.gate_dt_s = 1.0,
                5 => other.gate_over_floor = 2.0,
                _ => other.residual_conservation_limit = 1.0,
            }
            assert!(!other.is_valid(), "{index}");
            assert!(CraterState::try_radius(&other, 2.0).is_none());
            let mut state = CraterState::try_radius(&crater, 2.0).unwrap();
            let fresh = state_bits(&state);
            assert_eq!(
                step(&other, &mut state, &mut frame, NO_LOAD, GATE_DT_S),
                Err(StepReject::InvalidCrater)
            );
            assert_eq!(state_bits(&state), fresh);
            assert_eq!(frame.to_bytes(), held);
        }
    }

    /// The three copied constants follow from the sources' numbers.
    #[test]
    fn the_copied_constants_follow_from_the_sources() {
        let pi = core::f64::consts::PI;
        // Gravity at the place: GM/r² − ω² r cos²(lat).
        let omega = 2.0 * pi / (7.63262 * 3600.0);
        let cos_lat = crate::math::cos(6.0 * pi / 180.0);
        let gravity = 30.0 / (502.0 * 502.0) - omega * omega * 502.0 * cos_lat * cos_lat;
        assert!(within(gravity, SITE_GRAVITY_M_S2, 1.0e-13), "{gravity:e}");
        // The law of size.
        let a = raised(
            3.0 * PROJECTILE_MASS_KG / (4.0 * pi * PROJECTILE_DENSITY_KG_M3),
            1.0 / 3.0,
        );
        assert!(within(a, 0.037630887318588255, 1.0e-13));
        let pi2 = 3.22 * SITE_GRAVITY_M_S2 * a / (PROJECTILE_SPEED_M_S * PROJECTILE_SPEED_M_S);
        assert!(within(pi2, 2.8197696637948285e-12, 1.0e-13));
        let radius = SCALING_PREFACTOR
            * raised(pi2, -SCALING_EXPONENT)
            * raised(PROJECTILE_MASS_KG / RYUGU_BULK_DENSITY_KG_M3, 1.0 / 3.0);
        assert!(within(radius, FINAL_RADIUS_M, 1.0e-13), "{radius}");
        assert!(within((1.0 - SCALING_EXPONENT) / SCALING_EXPONENT, GROWTH_POWER, 1.0e-15));
        // The clock: J = 0.46970152405864873 is the 50-digit value of the beta function.
        let j = 0.46970152405864873_f64;
        let kg = SITE_GRAVITY_M_S2 * j * j / (2.0 * CLOCK_CONSTANT * CLOCK_CONSTANT);
        assert!(within(kg, GRAVITY_WORK_M_S2, 1.0e-14), "{kg:e}");
    }

    /// The protocol's line: the radius at which the card stops agrees with the
    /// closed form of the thirteen-shot law to 1 part in 10^9. The closed form
    /// is the forecast of the research record (`analysis/out/04_forecast.json`),
    /// worked out from the fit's own constants, not from the card's. The
    /// six-figure constants of 0.6.0 stopped at 8.571304624905444 m, 3.45e-8
    /// away: a miss of this line.
    #[test]
    fn the_stop_is_the_closed_form_of_the_fitted_law_to_a_part_in_a_billion() {
        let forecast = 8.571304329477925_f64;
        let (crater, mut state) = start();
        let mut frame = blank_frame();
        while !state.stopped {
            step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        }
        let difference = (state.radius_m - forecast).abs() / forecast;
        assert!(difference <= 1.0e-15, "{difference:e}");
        assert!(difference <= 1.0e-9);
        let rounded = 8.571304624905444_f64;
        assert!((rounded - forecast).abs() / forecast > 3.4e-8);
        // The constants are the fit's to the last printed digit.
        assert_eq!(SCALING_PREFACTOR.to_bits(), 0.8708204315358979_f64.to_bits());
        assert_eq!(SCALING_EXPONENT.to_bits(), 0.1660599800702854_f64.to_bits());
        assert_eq!(CLOCK_CONSTANT.to_bits(), 1.318295773573555_f64.to_bits());
    }

    #[test]
    fn types_are_copy_and_the_load_is_empty() {
        assert_copy::<CraterRyuguSci>();
        assert_copy::<CraterState>();
        assert_copy::<CraterLoad>();
        assert_copy::<StepReject>();
        assert_eq!(core::mem::size_of::<CraterLoad>(), 0);
    }

    #[test]
    fn the_start_is_on_the_impacts_own_curve() {
        let (crater, state) = start();
        assert!(within(state.wall_speed_m_s, 0.543674374221208, 1.0e-12));
        assert!(state.residual_conservation_error < 1.0e-9);
        assert!(!state.stopped);
        assert!(within(crater.wall_speed_m_s(4.0), f64::sqrt(-2.0 * crater.given_up_j_per_kg(4.0)), 1.0e-15));
        assert_eq!(crater.given_up_j_per_kg(FINAL_RADIUS_M), 0.0);
        assert_eq!(crater.wall_speed_m_s(FINAL_RADIUS_M), 0.0);
        assert_eq!(crater.wall_speed_m_s(9.0), 0.0);
    }

    /// Against the plain-Python emulation of the research record
    /// (`analysis/05_card_emulation.py`), which uses the platform's exp and log.
    #[test]
    fn the_run_matches_the_reference_after_1_100_and_1000_steps_and_stops_on_step_1597() {
        let (crater, mut state) = start();
        let mut frame = blank_frame();
        let mut worst: f64 = 0.0;
        let mut count = 0;
        while !state.stopped {
            step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            count += 1;
            worst = worst.max(state.residual_conservation_error);
            match count {
                1 => {
                    assert!(within(state.radius_m, 1.8744118211192553, 1.0e-12));
                    assert!(within(state.wall_speed_m_s, 0.4575480996172396, 1.0e-12));
                    assert!(within(state.residual_conservation_error, 5.1510769810070964e-8, 1.0e-3));
                }
                100 => {
                    assert!(within(state.radius_m, 4.528090545804181, 1.0e-11));
                    assert!(within(state.wall_speed_m_s, 0.04942320475191189, 1.0e-10));
                }
                1000 => {
                    assert!(within(state.radius_m, 8.159084357848577, 1.0e-11));
                    assert!(within(state.wall_speed_m_s, 0.005772001747318231, 1.0e-9));
                }
                _ => {}
            }
            assert!(count <= 1597);
            assert_eq!(frame.flags, if state.stopped { FLAG_STOPPED } else { 0 });
        }
        assert_eq!(count, 1597);
        assert_eq!(state.t_s, 399.25);
        assert_eq!(state.radius_m.to_bits(), FINAL_RADIUS_M.to_bits());
        assert_eq!(state.wall_speed_m_s, 0.0);
        assert_eq!(state.energy_defect_j_per_kg, 0.0);
        // The ledger over the whole growth at the step ceiling.
        assert!(within(worst, 5.8204711758746565e-8, 1.0e-3), "{worst:e}");
        assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT);
        // With the 0.917 s before the card's first radius, the stop is at the
        // clock's own time, 1.318295773573555 · sqrt(Rf / g) = 400.04 s, to one step.
        let clock = CLOCK_CONSTANT * f64::sqrt(FINAL_RADIUS_M / SITE_GRAVITY_M_S2);
        assert!(within(clock, 400.0365471280219, 1.0e-12));
        assert!((state.t_s + 0.91677692965436627 - clock).abs() <= GATE_DT_S);
        proof_intact(&frame);

        // Stopped: a step moves the time and nothing else, and the flag stays.
        let before = state;
        step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        assert_eq!(state.t_s, 399.5);
        assert_eq!(state.radius_m.to_bits(), before.radius_m.to_bits());
        assert_eq!(state.wall_speed_m_s, 0.0);
        assert!(state.stopped);
        assert_eq!(frame.flags, FLAG_STOPPED);
        assert_eq!(frame.t, 399.5);
        assert_eq!(frame.force_torque, 0.0);
    }

    /// Rule 10: a wrong stepping method is named.
    #[test]
    fn explicit_euler_is_refused_and_the_cards_own_step_lands() {
        let (crater, mut state) = start();
        let fresh = state;
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let landed = euler(&crater, [state.radius_m, state.wall_speed_m_s], GATE_DT_S).unwrap();
        assert!(within(landed[0], 1.874412816564025, 1.0e-12));
        assert!(within(landed[1], 0.4575482019954355, 1.0e-12));
        let defect = 0.5 * landed[1] * landed[1] + crater.given_up_j_per_kg(landed[0]);
        assert!(within(defect / crater.ledger_scale_j_per_kg(), 0.0064383370169685365, 1.0e-6));
        assert!(defect > 1000.0 * RESIDUAL_CONSERVATION_LIMIT * crater.ledger_scale_j_per_kg());
        let got = step_with(&crater, &mut state, &mut frame, GATE_DT_S, euler);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), state_bits(&fresh));
        assert_eq!(frame.to_bytes(), held);
        // Euler is refused from every part of the growth, not only the start.
        for radius_m in [2.0, 3.0, 5.0, 7.0, 8.0] {
            let mut state = CraterState::try_radius(&crater, radius_m).unwrap();
            let got = step_with(&crater, &mut state, &mut frame, GATE_DT_S, euler);
            assert!(
                got == Err(StepReject::SpontaneousGeneration) || got == Err(StepReject::FirstLawBreach),
                "{radius_m} {got:?}"
            );
            assert_eq!(frame.to_bytes(), held);
        }
        step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        assert!(state.residual_conservation_error < 0.1 * RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    /// Rule 10: a falsified ledger is named, both ways.
    #[test]
    fn an_altered_speed_or_radius_is_refused_both_ways_with_state_and_frame_bit_identical() {
        let (crater, mut state) = start();
        let mut frame = blank_frame();
        let mut count = 0;
        while count < 200 {
            step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
            count += 1;
        }
        let saved = state_bits(&state);
        let held = frame.to_bytes();
        // The trial speed a thousandth larger or smaller.
        let got = step_with(&crater, &mut state, &mut frame, GATE_DT_S, |c, y, dt| {
            let mut out = rk4(c, y, dt)?;
            out[1] *= 1.001;
            Some(out)
        });
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        let got = step_with(&crater, &mut state, &mut frame, GATE_DT_S, |c, y, dt| {
            let mut out = rk4(c, y, dt)?;
            out[1] *= 0.999;
            Some(out)
        });
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        // The trial radius a thousandth larger: the wall is too fast for it.
        let got = step_with(&crater, &mut state, &mut frame, GATE_DT_S, |c, y, dt| {
            let mut out = rk4(c, y, dt)?;
            out[0] *= 1.001;
            Some(out)
        });
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        // A crater declared stopped early: the wall at rest short of Rf.
        let got = step_with(&crater, &mut state, &mut frame, GATE_DT_S, |_, y, _| Some([y[0], 0.0]));
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        // A state altered between steps is named by the public step.
        let mut fast = state;
        fast.wall_speed_m_s *= 1.001;
        let fast_bits = state_bits(&fast);
        assert_eq!(
            step(&crater, &mut fast, &mut frame, NO_LOAD, GATE_DT_S),
            Err(StepReject::SpontaneousGeneration)
        );
        assert_eq!(state_bits(&fast), fast_bits);
        let mut slow = state;
        slow.wall_speed_m_s *= 0.999;
        assert_eq!(
            step(&crater, &mut slow, &mut frame, NO_LOAD, GATE_DT_S),
            Err(StepReject::FirstLawBreach)
        );
        let mut at_rest_early = state;
        at_rest_early.wall_speed_m_s = 0.0;
        assert_eq!(
            step(&crater, &mut at_rest_early, &mut frame, NO_LOAD, GATE_DT_S),
            Err(StepReject::FirstLawBreach)
        );
        // A recorded mark of a stop on a moving crater does not stop it.
        let mut marked = state;
        marked.stopped = true;
        step(&crater, &mut marked, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        assert!(marked.radius_m > state.radius_m);
        assert!(!marked.stopped);
        proof_intact(&frame);
    }

    #[test]
    fn every_refusal_is_named_and_leaves_state_and_frame_bit_identical() {
        let (crater, state) = start();
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let mut attempt = |alter: &dyn Fn(&mut CraterState), dt_s: f64, expected: StepReject| {
            let mut altered = state;
            alter(&mut altered);
            let before = state_bits(&altered);
            assert_eq!(step(&crater, &mut altered, &mut frame, NO_LOAD, dt_s), Err(expected));
            assert_eq!(state_bits(&altered), before);
            assert_eq!(frame.to_bytes(), held);
        };
        attempt(&|_| {}, f64::NAN, StepReject::NonFiniteInput);
        attempt(&|_| {}, f64::INFINITY, StepReject::NonFiniteInput);
        attempt(&|_| {}, 0.0, StepReject::NonPositiveDt);
        attempt(&|_| {}, -0.25, StepReject::NonPositiveDt);
        attempt(&|s| s.radius_m = f64::NAN, GATE_DT_S, StepReject::NonFiniteState);
        attempt(&|s| s.wall_speed_m_s = f64::INFINITY, GATE_DT_S, StepReject::NonFiniteState);
        attempt(&|s| s.t_s = f64::NAN, GATE_DT_S, StepReject::NonFiniteState);
        attempt(&|s| s.radius_m = 0.1621, GATE_DT_S, StepReject::RadiusBelowLedgerResolution);
        attempt(&|s| s.radius_m = 0.0, GATE_DT_S, StepReject::RadiusBelowLedgerResolution);
        attempt(&|s| s.radius_m = -3.0, GATE_DT_S, StepReject::RadiusBelowLedgerResolution);
        attempt(&|s| s.radius_m = 8.5714, GATE_DT_S, StepReject::RadiusAboveFinal);
        attempt(&|s| s.wall_speed_m_s = -s.wall_speed_m_s, GATE_DT_S, StepReject::WallMovingInward);
        // A step far above the ceiling is not clamped: the ledger names it.
        attempt(&|_| {}, 1.0, StepReject::SpontaneousGeneration);
        // A step so long that a stage leaves the positive numbers.
        attempt(&|_| {}, 1.0e6, StepReject::NonFiniteState);
        // The starting state is refused the same way.
        for radius_m in [f64::NAN, f64::INFINITY, 0.1621, 0.0, -1.0, 8.5714, 100.0] {
            assert!(CraterState::try_radius(&crater, radius_m).is_none(), "{radius_m}");
        }
        assert!(CraterState::try_radius(&crater, 1.75).is_some());
        assert!(CraterState::try_radius(&crater, RADIUS_FLOOR_M).is_some());
        assert!(CraterState::try_radius(&crater, FINAL_RADIUS_M).unwrap().stopped);
        proof_intact(&frame);
    }

    #[test]
    fn every_start_on_the_card_reaches_the_same_stop_under_the_ledger_limit() {
        let crater = CraterRyuguSci::sci_impact();
        for (radius_m, steps) in [(2.0, 1595), (4.0, 1534), (7.0, 1094), (8.5, 253), (8.5713, 2)] {
            let mut state = CraterState::try_radius(&crater, radius_m).unwrap();
            let mut frame = blank_frame();
            let mut count = 0;
            while !state.stopped {
                step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
                assert!(state.residual_conservation_error < 0.1 * RESIDUAL_CONSERVATION_LIMIT);
                count += 1;
                assert!(count < 2000);
            }
            assert_eq!(count, steps, "{radius_m}");
            assert_eq!(state.radius_m.to_bits(), FINAL_RADIUS_M.to_bits());
        }
    }

    #[test]
    fn the_flag_is_the_only_bit_and_the_frame_carries_the_declared_slots() {
        let (crater, mut state) = start();
        let mut frame = blank_frame();
        step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        assert_eq!(frame.flags, 0);
        assert_eq!(frame.t, 0.25);
        assert_eq!(frame.pos[0], state.radius_m as f32);
        assert_eq!(frame.pos[1], state.wall_speed_m_s as f32);
        assert_eq!(frame.pos[2], state.radius_over_final as f32);
        assert_eq!(frame.vel[0], state.kinetic_j_per_kg as f32);
        assert_eq!(frame.vel[1], state.given_up_j_per_kg as f32);
        assert_eq!(frame.vel[2], state.energy_defect_j_per_kg as f32);
        assert_eq!(frame.force_torque, (FINAL_RADIUS_M - state.radius_m) as f32);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        proof_intact(&frame);
        let mut stopped = CraterState::try_radius(&crater, FINAL_RADIUS_M).unwrap();
        step(&crater, &mut stopped, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        assert_eq!(frame.flags, FLAG_STOPPED);
        assert_eq!(FLAG_STOPPED, 1);
        proof_intact(&frame);
    }

    /// The floor is where one rounding of `W` is a tenth of the ledger's
    /// limit, and it lies outside the projectile and the wall there is slower
    /// than the projectile.
    #[test]
    fn the_floor_is_where_the_ledger_still_resolves_the_wall() {
        let crater = CraterRyuguSci::sci_impact();
        let gate = RESIDUAL_CONSERVATION_LIMIT * crater.ledger_scale_j_per_kg();
        let rounding = |r: f64| -crater.given_up_j_per_kg(r) * f64::EPSILON;
        assert!(rounding(RADIUS_FLOOR_M) <= 0.1 * gate);
        assert!(rounding(0.1621) > 0.1 * gate);
        let a = raised(3.0 * PROJECTILE_MASS_KG / (4.0 * core::f64::consts::PI * PROJECTILE_DENSITY_KG_M3), 1.0 / 3.0);
        assert!(RADIUS_FLOOR_M > 4.0 * a && RADIUS_FLOOR_M > 1.2 * a);
        assert!(within(crater.wall_speed_m_s(RADIUS_FLOOR_M), 213.5, 2.0e-3));
        assert!(crater.wall_speed_m_s(RADIUS_FLOOR_M) < 0.2 * PROJECTILE_SPEED_M_S);
    }

    /// Where the card stood at 1.75 m, a crater from 0.1622 m, 0.5 m and 1 m
    /// grows to the same stop under the ledger's limit, in steps that grow
    /// with the radius to the ceiling; the run from the floor matches the run
    /// from 1.75 m once it passes 1.75 m in the energy it holds. At the ceiling
    /// the first step from the floor lands in pieces (engine 0.15.2).
    #[test]
    fn a_crater_from_the_floor_grows_to_the_same_stop_under_the_ledger() {
        let crater = CraterRyuguSci::sci_impact();
        for radius_m in [RADIUS_FLOOR_M, 0.5, 1.0] {
            let mut state = CraterState::try_radius(&crater, radius_m).unwrap();
            let mut frame = blank_frame();
            let mut count = 0_u32;
            let mut worst: f64 = 0.0;
            while !state.stopped {
                let dt_s = if state.wall_speed_m_s > 0.0 {
                    f64::min(GATE_DT_S, 2.0e-3 * state.radius_m / state.wall_speed_m_s)
                } else {
                    GATE_DT_S
                };
                step(&crater, &mut state, &mut frame, NO_LOAD, dt_s).unwrap();
                worst = worst.max(state.residual_conservation_error);
                count += 1;
                assert!(count < 20_000, "{radius_m}");
            }
            assert!(worst < 0.6 * RESIDUAL_CONSERVATION_LIMIT, "{radius_m}: {worst:e}");
            assert_eq!(state.radius_m.to_bits(), FINAL_RADIUS_M.to_bits());
            assert_eq!(frame.flags, FLAG_STOPPED);
            // The time to the stop is the clock's time, to within a second: the time
            // before the start is under a millisecond.
            let clock = CLOCK_CONSTANT * f64::sqrt(FINAL_RADIUS_M / SITE_GRAVITY_M_S2);
            assert!((state.t_s - clock).abs() < 1.0, "{radius_m}: {}", state.t_s);
        }
        // From the floor a step of the ceiling is taken in pieces (engine
        // 0.15.2): through 0.15.1 the ledger refused it whole.
        let mut state = CraterState::try_radius(&crater, RADIUS_FLOOR_M).unwrap();
        let mut frame = blank_frame();
        step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        assert_eq!(state.t_s.to_bits(), GATE_DT_S.to_bits());
        assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
        proof_intact(&frame);
    }

    /// Engine 0.15.2, work order of 21 October: a start inside the card's
    /// range, 0.1622 to 8.571 m, at the ceiling of 0.25 s was refused
    /// SpontaneousGeneration at step 1 below about 1.39 m (1.10901, 0.73644,
    /// 1.154 and 1.34206 m among them). The whole step's Runge-Kutta error was
    /// past the ledger's limit. Now the step is taken in equal pieces, each
    /// closed on the same ledger, and lands; a step that landed whole lands
    /// bit for bit as before.
    #[test]
    fn a_step_of_the_ceiling_lands_in_pieces_from_every_start_on_the_card() {
        let crater = CraterRyuguSci::sci_impact();
        for radius_m in [RADIUS_FLOOR_M, 0.2, 0.5, 0.73644, 1.0, 1.10901, 1.154, 1.34206, 1.39, 1.4] {
            let mut state = CraterState::try_radius(&crater, radius_m).unwrap();
            let mut frame = blank_frame();
            step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).unwrap_or_else(|e| std::panic!("{radius_m}: {e:?}"));
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT, "{radius_m}");
            assert_eq!(state.t_s.to_bits(), GATE_DT_S.to_bits(), "{radius_m}");
            // The landing agrees with 4096 steps of the ceiling over 4096.
            let mut fine = CraterState::try_radius(&crater, radius_m).unwrap();
            let mut fine_frame = blank_frame();
            for _ in 0..4096 {
                step(&crater, &mut fine, &mut fine_frame, NO_LOAD, GATE_DT_S / 4096.0).unwrap();
            }
            assert!((state.radius_m - fine.radius_m).abs() < 1.0e-6 * fine.radius_m, "{radius_m}: {} {}", state.radius_m, fine.radius_m);
            proof_intact(&frame);
        }
        // A start that landed whole lands as the whole step, bit for bit: the
        // pieces are never taken for it.
        let mut whole = CraterState::try_radius(&crater, 1.75).unwrap();
        let mut by_hand = whole;
        let mut frame = blank_frame();
        step(&crater, &mut whole, &mut frame, NO_LOAD, GATE_DT_S).unwrap();
        let advanced = rk4(&crater, [by_hand.radius_m, by_hand.wall_speed_m_s], GATE_DT_S).unwrap();
        by_hand.t_s += GATE_DT_S;
        by_hand.radius_m = advanced[0];
        by_hand.wall_speed_m_s = advanced[1];
        assert_eq!(whole.radius_m.to_bits(), by_hand.radius_m.to_bits());
        assert_eq!(whole.wall_speed_m_s.to_bits(), by_hand.wall_speed_m_s.to_bits());
        // Each start along the line from the floor to 1.75 m lands: the
        // outcome no longer turns with the start.
        let mut radius_m = RADIUS_FLOOR_M;
        while radius_m < 1.75 {
            let mut state = CraterState::try_radius(&crater, radius_m).unwrap();
            let mut frame = blank_frame();
            assert!(step(&crater, &mut state, &mut frame, NO_LOAD, GATE_DT_S).is_ok(), "{radius_m}");
            radius_m += 0.0137;
        }
    }

    #[test]
    fn the_step_ceiling_and_floor_are_present_and_a_tiny_step_is_not_clamped() {
        let (crater, mut state) = start();
        assert_eq!(crater.gate_dt_s(), Some(0.25));
        assert_eq!(crater.floor_dt_s(), Some(0.25 / 1_048_576.0));
        let mut broken = crater;
        broken.gate_dt_s = f64::NAN;
        assert_eq!(broken.gate_dt_s(), None);
        assert_eq!(broken.floor_dt_s(), None);
        // A thousand steps at the floor move the wall by the speed times the
        // time and keep the ledger.
        let floor = crater.floor_dt_s().unwrap();
        let mut frame = blank_frame();
        let first = state;
        let mut count = 0;
        while count < 1000 {
            step(&crater, &mut state, &mut frame, NO_LOAD, floor).unwrap();
            count += 1;
        }
        let moved = state.radius_m - first.radius_m;
        assert!(within(moved, first.wall_speed_m_s * 1000.0 * floor, 1.0e-3), "{moved:e}");
        assert!(state.residual_conservation_error < 1.0e-9);
        // Half the ceiling and a quarter of it land closer to the ledger.
        let (_, mut half) = start();
        step(&crater, &mut half, &mut frame, NO_LOAD, 0.125).unwrap();
        assert!(within(half.radius_m, 1.8148822340950197, 1.0e-12));
        assert!(half.residual_conservation_error < 3.0e-9);
    }
}
