//! Thrust and shaft power of one propeller, the APC Thin Electric 10x7, by
//! blade-element momentum theory.
//!
//! The blade is cut into [`ANNULI`] rings between the hub station and the
//! tip. Chord and blade angle are those the University of Illinois measured
//! on this propeller (UIUC Propeller Data Site, volume 1,
//! `apce_10x7_geom.txt`), linear between the 18 rows. In each ring one
//! unknown, the inflow angle `phi`, is refined by bisection until the axial
//! momentum the ring gives the air equals the axial force of the two blade
//! elements in it:
//!
//! ```text
//! 4 F Vx (Vx - V) = sigma cn W^2            axial momentum of the ring
//! W cos(phi) (1 + kq) = Omega r             angular momentum of the ring
//! Vx = W sin(phi)
//! sigma = B c / (2 pi r)
//! kq    = sigma ct / (4 F sin(phi) cos(phi))
//! cn    = cl cos(phi) - cd sin(phi)
//! ct    = cl sin(phi) + cd cos(phi)
//! F     = (2/pi) acos(exp(-(B/2)(R - r)/(r sin(phi))))     Prandtl tip loss
//! ```
//!
//! The blade section is one for the whole blade, at angle of attack
//! `alpha = beta - phi`: lift coefficient `LIFT_SLOPE_PER_RAD (alpha -
//! alpha_zero_lift)`, held at `CL_MAX` beyond stall; drag coefficient
//! `CD0 + DRAG_DUE_TO_LIFT cl^2`, and never less than
//! `CD_FLAT_PLATE sin^2(alpha)`.
//!
//! **Two constants are fitted to this propeller's own measurements:**
//! [`ALPHA_ZERO_LIFT_DEG`] and [`CD0`], by least squares on the thrust and
//! power coefficients of the UIUC sweeps at 4007, 5001 and 5018 rev/min,
//! 43 points with advance ratio 0.112 to 0.726. Everything else was set
//! before any comparison.
//!
//! The model has no Reynolds-number and no Mach-number dependence: its
//! thrust and power coefficients depend on the advance ratio alone. The
//! range of rotational speed is therefore a limit, and it is enforced.
//!
//! **What is answered is what was compared.** With the air moving, the
//! measured points span 4007 to 6531 rev/min and advance ratio 0.111 to
//! 0.748; at zero airspeed, 4144 to 6542 rev/min. Those are the ranges, and
//! outside them a question is refused by name. The measurements give thrust
//! and power coefficients only: the density of the tunnel air was not
//! recorded, and every comparison was sent at 1.225 kg/m³. Density enters
//! this model only by scaling thrust and power, but it moves the chord
//! Reynolds number the measured coefficients depend on. That Reynolds number
//! at three quarters of the radius, formed at the question's own density, is
//! therefore held inside [`REYNOLDS_NUMBER_MIN`]..=[`REYNOLDS_NUMBER_MAX`],
//! the band the compared points span at 1.225 kg/m³. The comparison that
//! fixed these numbers is `fix_0.7.0/compared_envelope.py` in the evidence
//! folder.
//!
//! **Compared once with the held-back measurements on 2026-10-09, it missed
//! both lines fixed beforehand** (mean error at most 4.0 percent of full
//! scale and no value beyond 10.0). At the speeds never fitted, 6015 to 6531
//! rev/min, 64 points: mean 4.85, worst 10.16, low at every point. At zero
//! airspeed, 9 rows: mean 6.02, worst 11.76, the power 9 low. With the two
//! constants set by hand instead of fitted the error at the held-back speeds
//! is smaller (mean 3.77), but that model errs high where this one errs low:
//! the fit did not carry, because the measured coefficients rise with speed
//! and this model has no term for that. The miss at rest has another cause:
//! the model's static shaft power is about 9 percent of full scale low at
//! every speed. A second party scored the same answers on 2026-10-09 and
//! found the same numbers (`evidence/checks/second_party/V-27/`).
//!
//! **The ends of the advance-ratio range.** The advance ratio is computed
//! from the airspeed and the rotational speed and used as computed; it is
//! never moved to an end. The comparison with each end carries a named
//! allowance, [`ADVANCE_RATIO_MIN_ALLOWANCE_ULP`] and
//! [`ADVANCE_RATIO_MAX_ALLOWANCE_ULP`], for the rounding of that arithmetic
//! alone, so that an advance ratio of exactly 0.111 or 0.748 is answered at
//! every rotational speed.
//!
//! The work that fixed the constants, the protocol and its fingerprint, and
//! every result are in `evidence/research/propeller_apc_thin_electric_10x7/`.

use std::fmt;

const PI: f64 = std::f64::consts::PI;

/// Blades.
pub const BLADES: u32 = 2;
/// Diameter, m: 10 inches.
pub const DIAMETER_M: f64 = 0.254;
/// Rings the blade is cut into, at the mid-points of equal steps of r/R.
pub const ANNULI: usize = 34;
/// r/R of the innermost row of the measured geometry. Nothing inside it
/// carries load in the model.
pub const HUB_STATION: f64 = 0.15;

/// r/R, chord/R and blade angle in degrees: `apce_10x7_geom.txt`.
pub const GEOMETRY: [(f64, f64, f64); 18] = [
    (0.15, 0.138, 37.86),
    (0.20, 0.154, 45.82),
    (0.25, 0.175, 44.19),
    (0.30, 0.190, 38.35),
    (0.35, 0.198, 33.64),
    (0.40, 0.202, 29.90),
    (0.45, 0.200, 27.02),
    (0.50, 0.195, 24.67),
    (0.55, 0.186, 22.62),
    (0.60, 0.174, 20.88),
    (0.65, 0.161, 19.36),
    (0.70, 0.145, 17.98),
    (0.75, 0.129, 16.74),
    (0.80, 0.112, 15.79),
    (0.85, 0.096, 14.64),
    (0.90, 0.081, 13.86),
    (0.95, 0.061, 12.72),
    (1.00, 0.040, 11.53),
];

/// Lift slope of the section, per radian: 2 pi.
pub const LIFT_SLOPE_PER_RAD: f64 = 2.0 * PI;
/// Zero-lift angle of the section, degrees. Fitted to this propeller's
/// measured thrust and power at 4007, 5001 and 5018 rev/min.
pub const ALPHA_ZERO_LIFT_DEG: f64 = -2.77;
/// Minimum drag coefficient of the section. Fitted with
/// [`ALPHA_ZERO_LIFT_DEG`] to the same measurements.
pub const CD0: f64 = 0.0270;
/// The lift coefficient is held at this value beyond stall, either sign.
pub const CL_MAX: f64 = 1.2;
/// Drag rises by this times the square of the lift coefficient.
pub const DRAG_DUE_TO_LIFT: f64 = 0.02;
/// Drag coefficient of the section broadside to the flow.
pub const CD_FLAT_PLATE: f64 = 2.0;

/// Lowest and highest rotational speed with the air moving, rev/min: the
/// slowest and the fastest measured sweep compared. The fitted sweeps are at
/// 4007 to 5018 and the held-back sweeps at 6015 to 6531.
pub const ROTATIONAL_SPEED_MIN_REV_PER_MIN: f64 = 4007.0;
pub const ROTATIONAL_SPEED_MAX_REV_PER_MIN: f64 = 6531.0;
/// Lowest and highest rotational speed at zero airspeed, rev/min: the
/// slowest and the fastest static row compared.
pub const STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN: f64 = 4144.0;
pub const STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN: f64 = 6542.0;
/// Lowest and highest air density, kg/m³. No density was measured: the
/// measurements give coefficients only, and every comparison was sent at
/// 1.225. Inside this range the chord Reynolds number is the limit.
pub const AIR_DENSITY_MIN_KG_M3: f64 = 1.0;
pub const AIR_DENSITY_MAX_KG_M3: f64 = 1.3;
/// Viscosity of the air in the chord Reynolds number, Pa·s.
pub const AIR_VISCOSITY_PA_S: f64 = 1.81e-5;
/// Chord at three quarters of the radius, m: 0.129 R from the geometry.
pub const CHORD_AT_THREE_QUARTERS_M: f64 = GEOMETRY[12].1 * DIAMETER_M / 2.0;
/// The density every comparison was sent at, kg/m³.
pub const COMPARED_AIR_DENSITY_KG_M3: f64 = 1.225;
/// Lowest and highest chord Reynolds number at three quarters of the radius
/// ([`chord_reynolds_number`]) answered: the lowest and highest of the 116
/// compared points at 1.225 kg/m³, 44399.03 (4007 rev/min, advance ratio
/// 0.144) and 75515.76 (6519 rev/min, advance ratio 0.734), each rounded
/// outward to a whole number.
pub const REYNOLDS_NUMBER_MIN: f64 = 44_399.0;
pub const REYNOLDS_NUMBER_MAX: f64 = 75_516.0;
/// With the air moving, the advance ratio J = V / (n D) must lie in this
/// range: the lowest and highest measured advance ratio compared, 0.111 at
/// 6531 rev/min and 0.748 at 6015 rev/min. An airspeed of exactly zero is
/// also a question.
pub const ADVANCE_RATIO_MIN: f64 = 0.111;
pub const ADVANCE_RATIO_MAX: f64 = 0.748;
/// A question whose advance ratio, as computed here, lies below
/// [`ADVANCE_RATIO_MIN`] by no more than this many units in the last place
/// of that end is answered, at its own computed advance ratio; nothing is
/// moved to the end. A unit in the last place is the gap between two
/// neighbouring 64-bit numbers: 2^-56 at 0.111 and 2^-53 at 0.748.
pub const ADVANCE_RATIO_MIN_ALLOWANCE_ULP: u32 = 5;
/// The same allowance above [`ADVANCE_RATIO_MAX`].
///
/// The allowance is the rounding of the arithmetic between a caller's
/// advance ratio and the one computed here, and nothing more. A caller who
/// asks an advance ratio J forms the airspeed V = J (N / 60) D in three
/// rounded operations, in whatever order; [`advance_ratio`] forms N / 60,
/// times D, and V divided by that: three more. Each is rounded to the
/// nearest 64-bit number, so each result is its exact value times a factor
/// between 1 - u and 1 + u with u = 2^-53. The computed advance ratio is
/// therefore J times at most six such factors, and differs from J by less
/// than ((1 - u)^-6 - 1) J = (6u + 21u² + ...) J. In units in the last place
/// that is 0.111 × 6 × 2^-53 / 2^-56 = 5.3 at the lower end and
/// 0.748 × 6 × 2^-53 / 2^-53 = 4.5 at the upper. The computed value is itself
/// a 64-bit number in the same binade as the end, a whole number of units
/// from it, so it is at most 5 units away at the lower end and 4 at the
/// upper. When the caller's N / 60 is the same 64-bit number as the one
/// formed here it cancels exactly, four factors remain, the bounds are 3.6
/// and 3.0, and the computed value is at most 3 and 2 units away. The test
/// `both_ends_are_answered_at_every_whole_rotational_speed` prints the
/// largest distance met over every whole rotational speed of the range.
pub const ADVANCE_RATIO_MAX_ALLOWANCE_ULP: u32 = 4;
/// The lowest and the highest computed advance ratio that is answered:
/// [`ADVANCE_RATIO_MIN`] less [`ADVANCE_RATIO_MIN_ALLOWANCE_ULP`] units in
/// the last place, and [`ADVANCE_RATIO_MAX`] plus
/// [`ADVANCE_RATIO_MAX_ALLOWANCE_ULP`].
pub const ADVANCE_RATIO_LOWEST_ANSWERED: f64 =
    f64::from_bits(ADVANCE_RATIO_MIN.to_bits() - ADVANCE_RATIO_MIN_ALLOWANCE_ULP as u64);
pub const ADVANCE_RATIO_HIGHEST_ANSWERED: f64 =
    f64::from_bits(ADVANCE_RATIO_MAX.to_bits() + ADVANCE_RATIO_MAX_ALLOWANCE_ULP as u64);
/// The highest airspeed answered, m/s: the largest 64-bit airspeed whose
/// advance ratio at [`ROTATIONAL_SPEED_MAX_REV_PER_MIN`], as computed here,
/// is not above [`ADVANCE_RATIO_HIGHEST_ANSWERED`]. The next 64-bit number up
/// is refused at every rotational speed.
pub const AIRSPEED_MAX_M_S: f64 = 20.680629200000013;

/// The inflow angle is first looked for at this many equal steps between
/// [`ANGLE_LOW_RAD`] and a quarter turn less [`ANGLE_LOW_RAD`].
pub const SCAN_STEPS: u32 = 400;
pub const ANGLE_LOW_RAD: f64 = 1.0e-6;
/// A ring has settled when its inflow angle is bracketed this closely, rad.
pub const ANGLE_TOLERANCE_RAD: f64 = 1.0e-14;
/// Halvings allowed in one ring.
pub const MAX_BISECTIONS: u32 = 64;
/// At acceptance, the gap between a ring's axial momentum and its blade
/// force, as a fraction of the larger of the two, must be within this; and so
/// must the gap over the whole disc, as a fraction of the sum of the rings'
/// larger sides.
pub const BALANCE_TOLERANCE: f64 = 1.0e-9;

/// The measurements this solver is compared with.
pub const REFERENCE_NAME: &str = "UIUC Propeller Data Site, volume 1: APC Thin Electric 10x7, measured thrust and power coefficients at 5018 rev/min (apce_10x7_pg0813_5018.txt)";
pub const REFERENCE_URL: &str =
    "https://m-selig.ae.illinois.edu/props/volume-1/data/apce_10x7_pg0813_5018.txt";
/// Date the file was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// The largest thrust coefficient and power coefficient in the three fitted
/// sweeps. An error is stated as a fraction of these.
pub const THRUST_COEFFICIENT_FULL_SCALE: f64 = 0.1071;
pub const POWER_COEFFICIENT_FULL_SCALE: f64 = 0.0528;
/// The solver stays within this fraction of full scale of every point of
/// the reference sweep. The sweep is one of the three it was fitted to.
pub const REFERENCE_TOLERANCE_OF_FULL_SCALE: f64 = 0.06;

/// What is asked: how fast the propeller turns, how fast the air meets it
/// along its axis, and how dense the air is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropellerQuestion {
    pub rotational_speed_rev_per_min: f64,
    pub airspeed_m_s: f64,
    pub air_density_kg_m3: f64,
}

/// The settled propeller.
#[derive(Clone, Debug, PartialEq)]
pub struct PropellerSolution {
    /// J = V / (n D).
    pub advance_ratio: f64,
    /// CT = T / (rho n² D⁴).
    pub thrust_coefficient: f64,
    /// CP = P / (rho n³ D⁵).
    pub power_coefficient: f64,
    pub thrust_n: f64,
    pub torque_n_m: f64,
    pub shaft_power_w: f64,
    /// Thrust times airspeed over shaft power. Zero at zero airspeed.
    pub efficiency: f64,
    /// |blade thrust − momentum thrust| over the whole disc, as a fraction
    /// of the sum over the rings of the larger of each ring's two sides.
    pub thrust_balance_gap: f64,
    /// The largest gap of any one ring, as a fraction of that ring's larger side.
    pub worst_ring_balance_gap: f64,
    /// Halvings taken, all rings together.
    pub iterations: u32,
    /// Inflow angle of each ring, rad, from the hub outward.
    pub inflow_angle_rad: Vec<f64>,
    /// Thrust of each ring, N.
    pub ring_thrust_n: Vec<f64>,
    /// Torque of each ring, N·m.
    pub ring_torque_n_m: Vec<f64>,
}

/// Why no result is returned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropellerReject {
    /// A number of the question is NaN or infinite.
    NonFiniteInput,
    /// The rotational speed is outside [`ROTATIONAL_SPEED_MIN_REV_PER_MIN`]..=[`ROTATIONAL_SPEED_MAX_REV_PER_MIN`] with the air moving, or outside [`STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN`]..=[`STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN`] at zero airspeed.
    RotationalSpeedOutOfRange { rotational_speed_rev_per_min: f64, at_rest: bool },
    /// The air density is outside [`AIR_DENSITY_MIN_KG_M3`]..=[`AIR_DENSITY_MAX_KG_M3`].
    AirDensityOutOfRange { air_density_kg_m3: f64 },
    /// The airspeed is negative.
    AirspeedNegative { airspeed_m_s: f64 },
    /// The air is moving and the advance ratio is below [`ADVANCE_RATIO_MIN`] by more than [`ADVANCE_RATIO_MIN_ALLOWANCE_ULP`] or above [`ADVANCE_RATIO_MAX`] by more than [`ADVANCE_RATIO_MAX_ALLOWANCE_ULP`] units in the last place of the end.
    AdvanceRatioOutOfRange { advance_ratio: f64 },
    /// The chord Reynolds number at three quarters of the radius, at the question's density, is outside [`REYNOLDS_NUMBER_MIN`]..=[`REYNOLDS_NUMBER_MAX`], the band the compared measurements span at [`COMPARED_AIR_DENSITY_KG_M3`].
    ReynoldsNumberOutOfRange { reynolds_number: f64 },
    /// An intermediate value in a ring was NaN or infinite.
    NonFiniteValue { ring: u32 },
    /// No inflow angle balances a ring's momentum and blade force.
    NoBalance { ring: u32 },
    /// A ring's inflow angle was still not bracketed within [`ANGLE_TOLERANCE_RAD`] after the halvings allowed.
    NotSettled { ring: u32, iterations: u32 },
    /// At the settled angle a ring's axial momentum and blade force differ by more than [`BALANCE_TOLERANCE`].
    MomentumBalanceBroken { ring: u32, gap: f64 },
    /// At acceptance the blade thrust and the axial momentum given to the air over the whole disc differ by more than [`BALANCE_TOLERANCE`] of the sum of the rings' larger sides.
    DiscBalanceBroken { gap: f64 },
}

impl fmt::Display for PropellerReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "a number of the question is not finite"),
            Self::RotationalSpeedOutOfRange {
                rotational_speed_rev_per_min,
                at_rest: false,
            } => write!(
                f,
                "rotational speed {rotational_speed_rev_per_min} rev/min is outside {ROTATIONAL_SPEED_MIN_REV_PER_MIN}..={ROTATIONAL_SPEED_MAX_REV_PER_MIN} rev/min, the speeds compared with measurement with the air moving"
            ),
            Self::RotationalSpeedOutOfRange {
                rotational_speed_rev_per_min,
                at_rest: true,
            } => write!(
                f,
                "rotational speed {rotational_speed_rev_per_min} rev/min is outside {STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN}..={STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN} rev/min, the speeds compared with measurement at zero airspeed"
            ),
            Self::AirDensityOutOfRange { air_density_kg_m3 } => write!(
                f,
                "air density {air_density_kg_m3} kg/m³ is outside {AIR_DENSITY_MIN_KG_M3}..={AIR_DENSITY_MAX_KG_M3} kg/m³"
            ),
            Self::AirspeedNegative { airspeed_m_s } => {
                write!(f, "airspeed {airspeed_m_s} m/s is negative")
            }
            Self::AdvanceRatioOutOfRange { advance_ratio } => write!(
                f,
                "advance ratio {advance_ratio} is outside {ADVANCE_RATIO_MIN}..={ADVANCE_RATIO_MAX}, the advance ratios compared with measurement, by more than the allowance of {ADVANCE_RATIO_MIN_ALLOWANCE_ULP} units in the last place below (ADVANCE_RATIO_MIN_ALLOWANCE_ULP) and {ADVANCE_RATIO_MAX_ALLOWANCE_ULP} above (ADVANCE_RATIO_MAX_ALLOWANCE_ULP), and the airspeed is not zero"
            ),
            Self::ReynoldsNumberOutOfRange { reynolds_number } => write!(
                f,
                "chord Reynolds number {reynolds_number} at three quarters of the radius is outside {REYNOLDS_NUMBER_MIN}..={REYNOLDS_NUMBER_MAX}, the band the compared measurements span at {COMPARED_AIR_DENSITY_KG_M3} kg/m³"
            ),
            Self::NonFiniteValue { ring } => {
                write!(f, "a value in ring {ring} was not finite")
            }
            Self::NoBalance { ring } => {
                write!(f, "no inflow angle balances ring {ring}")
            }
            Self::NotSettled { ring, iterations } => {
                write!(f, "ring {ring} not settled after {iterations} halvings")
            }
            Self::MomentumBalanceBroken { ring, gap } => {
                write!(f, "ring {ring} momentum and blade force differ by {gap} of the larger")
            }
            Self::DiscBalanceBroken { gap } => write!(
                f,
                "over the whole disc the blade thrust and the axial momentum differ by {gap} of the sum of the rings' larger sides"
            ),
        }
    }
}

/// The advance ratio of a question, J = V / (n D), in three rounded
/// operations. The value is used as computed: it is never moved to an end
/// of the range.
pub fn advance_ratio(question: &PropellerQuestion) -> f64 {
    let revolutions_per_second = question.rotational_speed_rev_per_min / 60.0;
    question.airspeed_m_s / (revolutions_per_second * DIAMETER_M)
}

/// The chord Reynolds number at three quarters of the radius: density times
/// the geometric speed of the section there, sqrt((2 pi n 0.75 R)² + V²),
/// times [`CHORD_AT_THREE_QUARTERS_M`], over [`AIR_VISCOSITY_PA_S`].
pub fn chord_reynolds_number(question: &PropellerQuestion) -> f64 {
    let revolutions_per_second = question.rotational_speed_rev_per_min / 60.0;
    let tangential = 2.0 * PI * revolutions_per_second * (0.75 * (DIAMETER_M / 2.0));
    let airspeed = question.airspeed_m_s;
    let speed = (tangential * tangential + airspeed * airspeed).sqrt();
    question.air_density_kg_m3 * speed * CHORD_AT_THREE_QUARTERS_M / AIR_VISCOSITY_PA_S
}

/// Every limit a question itself shows, in the order they are asked.
pub fn check_question(question: &PropellerQuestion) -> Result<(), PropellerReject> {
    let PropellerQuestion {
        rotational_speed_rev_per_min,
        airspeed_m_s,
        air_density_kg_m3,
    } = *question;
    if !(rotational_speed_rev_per_min.is_finite()
        && airspeed_m_s.is_finite()
        && air_density_kg_m3.is_finite())
    {
        return Err(PropellerReject::NonFiniteInput);
    }
    if airspeed_m_s < 0.0 {
        return Err(PropellerReject::AirspeedNegative { airspeed_m_s });
    }
    let at_rest = airspeed_m_s == 0.0;
    let speeds = if at_rest {
        STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN..=STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN
    } else {
        ROTATIONAL_SPEED_MIN_REV_PER_MIN..=ROTATIONAL_SPEED_MAX_REV_PER_MIN
    };
    if !speeds.contains(&rotational_speed_rev_per_min) {
        return Err(PropellerReject::RotationalSpeedOutOfRange {
            rotational_speed_rev_per_min,
            at_rest,
        });
    }
    if !(AIR_DENSITY_MIN_KG_M3..=AIR_DENSITY_MAX_KG_M3).contains(&air_density_kg_m3) {
        return Err(PropellerReject::AirDensityOutOfRange { air_density_kg_m3 });
    }
    if !at_rest {
        let advance_ratio = advance_ratio(question);
        // The question's own J against each end with the named allowance.
        // J itself is not changed, here or in the solve.
        if !(ADVANCE_RATIO_LOWEST_ANSWERED..=ADVANCE_RATIO_HIGHEST_ANSWERED)
            .contains(&advance_ratio)
        {
            return Err(PropellerReject::AdvanceRatioOutOfRange { advance_ratio });
        }
    }
    let reynolds_number = chord_reynolds_number(question);
    if !(REYNOLDS_NUMBER_MIN..=REYNOLDS_NUMBER_MAX).contains(&reynolds_number) {
        return Err(PropellerReject::ReynoldsNumberOutOfRange { reynolds_number });
    }
    Ok(())
}

/// The blade section's constants and the numerical limits, so that a test
/// can break one on purpose. The public solver uses [`Settings::LOCKED`].
#[derive(Clone, Copy, Debug)]
struct Settings {
    alpha_zero_lift_rad: f64,
    cd0: f64,
    max_bisections: u32,
    balance_tolerance: f64,
    disc_balance_tolerance: f64,
}

impl Settings {
    const LOCKED: Settings = Settings {
        alpha_zero_lift_rad: ALPHA_ZERO_LIFT_DEG * PI / 180.0,
        cd0: CD0,
        max_bisections: MAX_BISECTIONS,
        balance_tolerance: BALANCE_TOLERANCE,
        disc_balance_tolerance: BALANCE_TOLERANCE,
    };
}

/// Chord/R and blade angle (rad) at r/R = `x`, linear between the rows.
fn geometry_at(x: f64) -> (f64, f64) {
    let mut row = 0;
    while row + 2 < GEOMETRY.len() && x > GEOMETRY[row + 1].0 {
        row += 1;
    }
    let (x0, c0, b0) = GEOMETRY[row];
    let (x1, c1, b1) = GEOMETRY[row + 1];
    let t = (x - x0) / (x1 - x0);
    (c0 + t * (c1 - c0), (b0 + t * (b1 - b0)) * PI / 180.0)
}

/// Lift and drag coefficient of the section at angle of attack `alpha`, rad.
fn section(alpha: f64, settings: &Settings) -> (f64, f64) {
    let cl_linear = LIFT_SLOPE_PER_RAD * (alpha - settings.alpha_zero_lift_rad);
    let sin_alpha = alpha.sin();
    let cd_plate = CD_FLAT_PLATE * sin_alpha * sin_alpha;
    let cl = if cl_linear.abs() <= CL_MAX {
        cl_linear
    } else {
        CL_MAX.copysign(cl_linear)
    };
    let cd = settings.cd0 + DRAG_DUE_TO_LIFT * cl * cl;
    (cl, if cd_plate > cd { cd_plate } else { cd })
}

/// One ring at one trial inflow angle. Lengths are in R and speeds in
/// Omega R.
struct RingState {
    /// 4 F Vx (Vx − V): the axial momentum side.
    momentum_side: f64,
    /// sigma cn W²: the blade side.
    blade_side: f64,
    relative_speed: f64,
    cn: f64,
    ct: f64,
}

impl RingState {
    fn imbalance(&self) -> f64 {
        self.momentum_side - self.blade_side
    }
}

/// `None` when a value is not finite, the tip loss factor is outside
/// (0, 1], or the ring's swirl has overturned the blade's own speed.
fn ring_state(
    x: f64,
    chord: f64,
    beta: f64,
    inflow_ratio: f64,
    phi: f64,
    settings: &Settings,
) -> Option<RingState> {
    let blades = f64::from(BLADES);
    let sigma = blades * chord / (2.0 * PI * x);
    let (sin_phi, cos_phi) = (phi.sin(), phi.cos());
    let exponent = (blades / 2.0) * (1.0 - x) / (x * sin_phi);
    let tip_loss = (2.0 / PI) * (-exponent).exp().acos();
    let (cl, cd) = section(beta - phi, settings);
    let cn = cl * cos_phi - cd * sin_phi;
    let ct = cl * sin_phi + cd * cos_phi;
    let kq = sigma * ct / (4.0 * tip_loss * sin_phi * cos_phi);
    let relative_speed = x / ((1.0 + kq) * cos_phi);
    let axial_speed = relative_speed * sin_phi;
    let momentum_side = 4.0 * tip_loss * axial_speed * (axial_speed - inflow_ratio);
    let blade_side = sigma * cn * relative_speed * relative_speed;
    let sound = momentum_side.is_finite()
        && blade_side.is_finite()
        && tip_loss > 0.0
        && tip_loss <= 1.0
        && 1.0 + kq > 0.0
        && relative_speed > 0.0;
    sound.then_some(RingState {
        momentum_side,
        blade_side,
        relative_speed,
        cn,
        ct,
    })
}

/// Thrust and shaft power of the propeller at one rotational speed, airspeed
/// and air density, or the named reason there is none.
pub fn solve(question: &PropellerQuestion) -> Result<PropellerSolution, PropellerReject> {
    solve_with(question, &Settings::LOCKED)
}

fn solve_with(
    question: &PropellerQuestion,
    settings: &Settings,
) -> Result<PropellerSolution, PropellerReject> {
    check_question(question)?;
    let advance_ratio = advance_ratio(question);
    // V / (Omega R)
    let inflow_ratio = advance_ratio / PI;
    let step = (1.0 - HUB_STATION) / ANNULI as f64;
    let blades = f64::from(BLADES);
    let high = PI / 2.0 - ANGLE_LOW_RAD;

    let mut iterations = 0u32;
    let mut inflow_angle_rad = Vec::with_capacity(ANNULI);
    // per ring, in units rho (Omega R)² R² for thrust and ... R³ for torque
    let mut ring_thrust = Vec::with_capacity(ANNULI);
    let mut ring_torque = Vec::with_capacity(ANNULI);
    let (mut blade_thrust, mut momentum_thrust, mut balance_scale) = (0.0, 0.0, 0.0);
    let mut worst_ring_balance_gap: f64 = 0.0;

    for index in 0..ANNULI {
        let ring = index as u32;
        let x = HUB_STATION + (index as f64 + 0.5) * step;
        let (chord, beta) = geometry_at(x);
        let state_at = |phi: f64| {
            ring_state(x, chord, beta, inflow_ratio, phi, settings)
                .ok_or(PropellerReject::NonFiniteValue { ring })
        };

        // The first change of sign of the imbalance, from the lowest angle up.
        let mut low = ANGLE_LOW_RAD;
        let mut low_imbalance = state_at(low)?.imbalance();
        let mut bracket = None;
        for k in 1..=SCAN_STEPS {
            let phi = ANGLE_LOW_RAD + (high - ANGLE_LOW_RAD) * f64::from(k) / f64::from(SCAN_STEPS);
            let Some(state) = ring_state(x, chord, beta, inflow_ratio, phi, settings) else {
                break;
            };
            let imbalance = state.imbalance();
            if low_imbalance == 0.0 || (low_imbalance < 0.0) != (imbalance < 0.0) {
                bracket = Some(phi);
                break;
            }
            low = phi;
            low_imbalance = imbalance;
        }
        let Some(mut upper) = bracket else {
            return Err(PropellerReject::NoBalance { ring });
        };

        let mut halvings = 0u32;
        while upper - low > ANGLE_TOLERANCE_RAD {
            if halvings == settings.max_bisections {
                return Err(PropellerReject::NotSettled {
                    ring,
                    iterations: halvings,
                });
            }
            let middle = 0.5 * (low + upper);
            let imbalance = state_at(middle)?.imbalance();
            if (imbalance < 0.0) == (low_imbalance < 0.0) {
                low = middle;
                low_imbalance = imbalance;
            } else {
                upper = middle;
            }
            halvings += 1;
        }
        iterations += halvings;

        let phi = 0.5 * (low + upper);
        let state = state_at(phi)?;
        let larger = state.momentum_side.abs().max(state.blade_side.abs());
        let gap = if larger > 0.0 {
            state.imbalance().abs() / larger
        } else {
            0.0
        };
        if !(gap <= settings.balance_tolerance) {
            return Err(PropellerReject::MomentumBalanceBroken { ring, gap });
        }
        worst_ring_balance_gap = worst_ring_balance_gap.max(gap);

        let w2 = state.relative_speed * state.relative_speed;
        let thrust = blades * 0.5 * w2 * chord * state.cn * step;
        let torque = blades * 0.5 * w2 * chord * state.ct * x * step;
        // the same thrust from the momentum side: 2 pi x dx (4 F Vx (Vx − V)) / 2
        let from_momentum = PI * x * state.momentum_side * step;
        blade_thrust += thrust;
        momentum_thrust += from_momentum;
        balance_scale += thrust.abs().max(from_momentum.abs());
        inflow_angle_rad.push(phi);
        ring_thrust.push(thrust);
        ring_torque.push(torque);
    }

    let total_torque: f64 = ring_torque.iter().sum();
    // T = CT rho n² D⁴ with Omega R = pi n D and R = D / 2
    let thrust_coefficient = blade_thrust * PI * PI / 4.0;
    let power_coefficient = 2.0 * PI * total_torque * PI * PI / 8.0;
    let thrust_balance_gap = if balance_scale > 0.0 {
        (blade_thrust - momentum_thrust).abs() / balance_scale
    } else {
        0.0
    };

    let n = question.rotational_speed_rev_per_min / 60.0;
    let rho = question.air_density_kg_m3;
    let d = DIAMETER_M;
    let thrust_unit = rho * n * n * d * d * d * d;
    let power_unit = thrust_unit * n * d;
    let thrust_n = thrust_coefficient * thrust_unit;
    let shaft_power_w = power_coefficient * power_unit;
    let torque_n_m = shaft_power_w / (2.0 * PI * n);
    let efficiency = if question.airspeed_m_s == 0.0 {
        0.0
    } else {
        thrust_coefficient * advance_ratio / power_coefficient
    };
    // one ring's thrust in N: CT-share of the whole
    let to_newtons = thrust_unit * PI * PI / 4.0;
    let to_newton_metres = power_unit * PI * PI / 8.0 / n;
    let ring_thrust_n: Vec<f64> = ring_thrust.iter().map(|t| t * to_newtons).collect();
    let ring_torque_n_m: Vec<f64> = ring_torque.iter().map(|q| q * to_newton_metres).collect();

    let all_finite = [
        thrust_coefficient,
        power_coefficient,
        thrust_n,
        shaft_power_w,
        torque_n_m,
        efficiency,
        thrust_balance_gap,
    ]
    .iter()
    .all(|value| value.is_finite());
    if !all_finite {
        return Err(PropellerReject::NonFiniteValue {
            ring: ANNULI as u32,
        });
    }
    if !(thrust_balance_gap <= settings.disc_balance_tolerance) {
        return Err(PropellerReject::DiscBalanceBroken {
            gap: thrust_balance_gap,
        });
    }

    Ok(PropellerSolution {
        advance_ratio,
        thrust_coefficient,
        power_coefficient,
        thrust_n,
        torque_n_m,
        shaft_power_w,
        efficiency,
        thrust_balance_gap,
        worst_ring_balance_gap,
        iterations,
        inflow_angle_rad,
        ring_thrust_n,
        ring_torque_n_m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(rev_per_min: f64, advance_ratio: f64) -> PropellerQuestion {
        PropellerQuestion {
            rotational_speed_rev_per_min: rev_per_min,
            airspeed_m_s: advance_ratio * (rev_per_min / 60.0) * DIAMETER_M,
            air_density_kg_m3: 1.225,
        }
    }

    /// UIUC Propeller Data Site, volume 1, `apce_10x7_pg0813_5018.txt`:
    /// J, CT, CP of the APC Thin Electric 10x7 at 5018 rev/min.
    /// <https://m-selig.ae.illinois.edu/props/volume-1/data/apce_10x7_pg0813_5018.txt>
    /// Retrieved 2026-10-09. SHA-256 of the file is in
    /// `evidence/research/propeller_apc_thin_electric_10x7/raw/SOURCES.md`.
    /// This sweep is one of the three the two fitted constants were fitted
    /// to, so this test shows that the solver is the fitted model. It is not
    /// the validation; the held-back sweeps are scored through the gateway.
    const UIUC_5018: [(f64, f64, f64); 17] = [
        (0.112, 0.1071, 0.0521),
        (0.144, 0.1056, 0.0524),
        (0.173, 0.1038, 0.0524),
        (0.202, 0.1017, 0.0525),
        (0.230, 0.0996, 0.0525),
        (0.260, 0.0972, 0.0526),
        (0.287, 0.0949, 0.0526),
        (0.320, 0.0922, 0.0528),
        (0.344, 0.0898, 0.0528),
        (0.372, 0.0865, 0.0527),
        (0.400, 0.0825, 0.0521),
        (0.431, 0.0766, 0.0507),
        (0.463, 0.0700, 0.0486),
        (0.492, 0.0639, 0.0463),
        (0.517, 0.0576, 0.0437),
        (0.547, 0.0504, 0.0402),
        (0.575, 0.0446, 0.0372),
    ];

    #[test]
    fn uiuc_measured_sweep_at_5018_rev_per_min() {
        let mut worst: f64 = 0.0;
        for (j, ct, cp) in UIUC_5018 {
            let solved = solve(&ask(5018.0, j)).expect("inside the range");
            let ct_error = (solved.thrust_coefficient - ct) / THRUST_COEFFICIENT_FULL_SCALE;
            let cp_error = (solved.power_coefficient - cp) / POWER_COEFFICIENT_FULL_SCALE;
            println!("J {j} CT error {ct_error:+.4} CP error {cp_error:+.4} of full scale");
            worst = worst.max(ct_error.abs()).max(cp_error.abs());
        }
        println!("UIUC retrieved {REFERENCE_RETRIEVED} from {REFERENCE_URL}: worst {worst:.4}");
        assert!(worst <= REFERENCE_TOLERANCE_OF_FULL_SCALE, "{worst}");
        // and it is not a loose line: the worst point is beyond 4 percent
        assert!(worst > 0.04, "{worst}");
    }

    /// A density at which the chord Reynolds number stays inside the band
    /// over the whole advance-ratio range at `rev_per_min`: at 1.225 kg/m³
    /// the slowest speed's lowest advance ratio and the fastest speed's
    /// highest fall just outside it, since neither was measured.
    fn density_for(rev_per_min: f64) -> f64 {
        if rev_per_min < 5000.0 { 1.25 } else { 1.2 }
    }

    fn ask_at(rev_per_min: f64, advance_ratio: f64) -> PropellerQuestion {
        PropellerQuestion {
            air_density_kg_m3: density_for(rev_per_min),
            ..ask(rev_per_min, advance_ratio)
        }
    }

    #[test]
    fn every_ring_balances_over_the_whole_range() {
        let mut questions = vec![
            ask(STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN, 0.0),
            ask(5000.0, 0.0),
            ask(STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN, 0.0),
        ];
        for speed in [ROTATIONAL_SPEED_MIN_REV_PER_MIN, 5000.0, ROTATIONAL_SPEED_MAX_REV_PER_MIN] {
            for k in 0..=64 {
                let j = (ADVANCE_RATIO_MIN + 0.01 * f64::from(k)).min(ADVANCE_RATIO_MAX);
                questions.push(ask_at(speed, j));
            }
        }
        for question in questions {
            let solved = solve(&question).unwrap_or_else(|reject| panic!("{question:?}: {reject}"));
            assert!(solved.worst_ring_balance_gap <= BALANCE_TOLERANCE);
            assert!(solved.thrust_balance_gap <= BALANCE_TOLERANCE);
            assert_eq!(solved.inflow_angle_rad.len(), ANNULI);
            assert!(solved.thrust_n > 0.0 && solved.shaft_power_w > 0.0, "{question:?}");
            assert!(solved.iterations <= MAX_BISECTIONS * ANNULI as u32);
            let sum: f64 = solved.ring_thrust_n.iter().sum();
            assert!((sum - solved.thrust_n).abs() <= 1e-12 * solved.thrust_n);
            let torque: f64 = solved.ring_torque_n_m.iter().sum();
            assert!((torque - solved.torque_n_m).abs() <= 1e-12 * solved.torque_n_m);
        }
    }

    /// No propeller turns shaft power into thrust power better than the
    /// ideal actuator disc of the same thrust: eta <= 2 / (1 + sqrt(1 + 8 CT / (pi J²))).
    #[test]
    fn efficiency_stays_below_the_ideal_disc() {
        for k in 0..=64 {
            let j = (ADVANCE_RATIO_MIN + 0.01 * f64::from(k)).min(ADVANCE_RATIO_MAX);
            let solved = solve(&ask(5000.0, j)).expect("inside the range");
            let ideal = 2.0 / (1.0 + (1.0 + 8.0 * solved.thrust_coefficient / (PI * j * j)).sqrt());
            assert!(solved.efficiency > 0.0 && solved.efficiency < ideal, "{j}");
        }
        // At rest, the power is above the ideal disc's: P >= T^1.5 / sqrt(2 rho A).
        let rest = solve(&ask(5000.0, 0.0)).expect("inside the range");
        let area = PI * DIAMETER_M * DIAMETER_M / 4.0;
        let ideal_power = rest.thrust_n.powf(1.5) / (2.0 * 1.225 * area).sqrt();
        assert!(rest.shaft_power_w > ideal_power);
        assert_eq!(rest.efficiency, 0.0);
    }

    /// The coefficients depend on the advance ratio alone, thrust goes with
    /// density and the square of speed, and power with density and its cube.
    #[test]
    fn thrust_and_power_scale_as_the_coefficients_say() {
        let slow = solve(&ask(4354.0, 0.4)).expect("slow");
        let fast = solve(&ask(6531.0, 0.4)).expect("fast");
        assert!((slow.thrust_coefficient - fast.thrust_coefficient).abs() < 1e-12);
        assert!((fast.thrust_n / slow.thrust_n - 2.25).abs() < 1e-10);
        assert!((fast.shaft_power_w / slow.shaft_power_w - 3.375).abs() < 1e-10);
        let mut thin = ask(5000.0, 0.4);
        thin.air_density_kg_m3 = 1.0;
        let thick = PropellerQuestion { air_density_kg_m3: 1.25, ..thin };
        let (thin, thick) = (solve(&thin).expect("thin"), solve(&thick).expect("thick"));
        assert!((thick.thrust_n / thin.thrust_n - 1.25).abs() < 1e-12);
        assert!((thick.shaft_power_w - thick.torque_n_m * 2.0 * PI * 5000.0 / 60.0).abs() < 1e-9);
    }

    #[test]
    fn same_question_same_bits() {
        let question = ask(5555.0, 0.3333);
        let (first, second) = (solve(&question).expect("first"), solve(&question).expect("second"));
        assert_eq!(first.thrust_n.to_bits(), second.thrust_n.to_bits());
        assert_eq!(first.shaft_power_w.to_bits(), second.shaft_power_w.to_bits());
        assert_eq!(first, second);
    }

    #[test]
    fn a_question_outside_the_range_is_named_and_nothing_is_pulled_back() {
        let good = ask(5000.0, 0.4);
        let rest = ask(5000.0, 0.0);
        let cases: Vec<(PropellerQuestion, &str)> = vec![
            (PropellerQuestion { rotational_speed_rev_per_min: 4006.999, ..good }, "RotationalSpeedOutOfRange"),
            (PropellerQuestion { rotational_speed_rev_per_min: 6531.001, ..good }, "RotationalSpeedOutOfRange"),
            (PropellerQuestion { rotational_speed_rev_per_min: 4143.999, ..rest }, "RotationalSpeedOutOfRange"),
            (PropellerQuestion { rotational_speed_rev_per_min: 6542.001, ..rest }, "RotationalSpeedOutOfRange"),
            (PropellerQuestion { rotational_speed_rev_per_min: 4100.0, ..rest }, "RotationalSpeedOutOfRange"),
            (PropellerQuestion { air_density_kg_m3: 0.999, ..good }, "AirDensityOutOfRange"),
            (PropellerQuestion { air_density_kg_m3: 1.301, ..good }, "AirDensityOutOfRange"),
            (PropellerQuestion { airspeed_m_s: -0.001, ..good }, "AirspeedNegative"),
            (PropellerQuestion { airspeed_m_s: 1.0e-9, ..good }, "AdvanceRatioOutOfRange"),
            (ask(5000.0, 0.10), "AdvanceRatioOutOfRange"),
            (ask(5000.0, 0.1109), "AdvanceRatioOutOfRange"),
            (ask(5000.0, 0.7481), "AdvanceRatioOutOfRange"),
            (ask(5000.0, 0.75), "AdvanceRatioOutOfRange"),
            // never compared: the slowest sweep at the lowest advance ratio,
            // the fastest at the highest, at 1.225 kg/m³
            (ask(ROTATIONAL_SPEED_MIN_REV_PER_MIN, ADVANCE_RATIO_MIN), "ReynoldsNumberOutOfRange"),
            (ask(ROTATIONAL_SPEED_MAX_REV_PER_MIN, ADVANCE_RATIO_MAX), "ReynoldsNumberOutOfRange"),
            // thin air at a low speed, dense air at a high one
            (PropellerQuestion { air_density_kg_m3: 1.0, ..ask(4500.0, 0.3) }, "ReynoldsNumberOutOfRange"),
            (PropellerQuestion { air_density_kg_m3: 1.3, ..ask(6531.0, 0.4) }, "ReynoldsNumberOutOfRange"),
            (PropellerQuestion { air_density_kg_m3: 1.0, ..ask(4144.0, 0.0) }, "ReynoldsNumberOutOfRange"),
            (PropellerQuestion { airspeed_m_s: f64::NAN, ..good }, "NonFiniteInput"),
            (PropellerQuestion { rotational_speed_rev_per_min: f64::INFINITY, ..good }, "NonFiniteInput"),
            (PropellerQuestion { air_density_kg_m3: f64::NEG_INFINITY, ..good }, "NonFiniteInput"),
        ];
        for (question, name) in cases {
            let reject = solve(&question).expect_err(name);
            assert!(format!("{reject:?}").starts_with(name), "{reject:?} is not {name}");
            assert_eq!(check_question(&question), Err(reject));
            assert!(!reject.to_string().ends_with('.'));
        }
        // The sentence names which range of rotational speed was broken.
        let moving = check_question(&PropellerQuestion { rotational_speed_rev_per_min: 4000.0, ..good }).unwrap_err();
        assert_eq!(
            moving.to_string(),
            "rotational speed 4000 rev/min is outside 4007..=6531 rev/min, the speeds compared with measurement with the air moving"
        );
        let resting = check_question(&PropellerQuestion { rotational_speed_rev_per_min: 4100.0, ..rest }).unwrap_err();
        assert_eq!(
            resting.to_string(),
            "rotational speed 4100 rev/min is outside 4144..=6542 rev/min, the speeds compared with measurement at zero airspeed"
        );
        // the ends of each range are inside it
        for question in [
            ask_at(ROTATIONAL_SPEED_MIN_REV_PER_MIN, ADVANCE_RATIO_MIN),
            ask(ROTATIONAL_SPEED_MIN_REV_PER_MIN, ADVANCE_RATIO_MAX),
            ask(ROTATIONAL_SPEED_MAX_REV_PER_MIN, ADVANCE_RATIO_MIN),
            ask_at(ROTATIONAL_SPEED_MAX_REV_PER_MIN, ADVANCE_RATIO_MAX),
            ask(STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN, 0.0),
            ask(STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN, 0.0),
            PropellerQuestion { air_density_kg_m3: 1.0, ..good },
            PropellerQuestion { air_density_kg_m3: 1.3, ..good },
            PropellerQuestion { airspeed_m_s: 0.0, ..good },
            PropellerQuestion { airspeed_m_s: -0.0, ..good },
        ] {
            solve(&question).unwrap_or_else(|reject| panic!("{question:?}: {reject}"));
        }
        // The published highest airspeed is answered at the top rotational
        // speed (at a density that keeps the Reynolds number in its band),
        // the next 64-bit number up is refused at every rotational speed,
        // and it is the largest airspeed whose computed advance ratio there
        // is not above the highest answered.
        let top = PropellerQuestion {
            rotational_speed_rev_per_min: ROTATIONAL_SPEED_MAX_REV_PER_MIN,
            airspeed_m_s: AIRSPEED_MAX_M_S,
            air_density_kg_m3: 1.2,
        };
        assert_eq!(advance_ratio(&top), ADVANCE_RATIO_HIGHEST_ANSWERED);
        solve(&top).unwrap_or_else(|reject| panic!("{reject}"));
        for speed in [ROTATIONAL_SPEED_MIN_REV_PER_MIN, 5000.0, ROTATIONAL_SPEED_MAX_REV_PER_MIN] {
            for density in [1.0, 1.2, 1.3] {
                let above = PropellerQuestion {
                    rotational_speed_rev_per_min: speed,
                    airspeed_m_s: AIRSPEED_MAX_M_S.next_up(),
                    air_density_kg_m3: density,
                };
                assert!(matches!(check_question(&above), Err(PropellerReject::AdvanceRatioOutOfRange { .. })));
            }
        }
        let caller = ADVANCE_RATIO_MAX * (ROTATIONAL_SPEED_MAX_REV_PER_MIN / 60.0) * DIAMETER_M;
        assert!(caller <= AIRSPEED_MAX_M_S && AIRSPEED_MAX_M_S - caller < 1e-13, "{caller}");
    }

    /// Every point the 0.6.0 comparison scored, the 43 fitted, the 64 held
    /// back and the 9 static rows, read from the UIUC files in
    /// `evidence/research/propeller_apc_thin_electric_10x7/raw/` and asked
    /// as the protocol asked it (airspeed J × (rev/min / 60) × 0.254 m,
    /// 1.225 kg/m³). None is refused, and the ranges are their extremes.
    const COMPARED_SWEEPS: [(f64, &[f64]); 7] = [
        (4007.0, &[0.144, 0.178, 0.217, 0.255, 0.29, 0.327, 0.363, 0.394, 0.432, 0.465, 0.503, 0.54, 0.575, 0.613, 0.647, 0.685, 0.721]),
        (5001.0, &[0.487, 0.514, 0.547, 0.575, 0.605, 0.633, 0.663, 0.69, 0.726]),
        (5018.0, &[0.112, 0.144, 0.173, 0.202, 0.23, 0.26, 0.287, 0.32, 0.344, 0.372, 0.4, 0.431, 0.463, 0.492, 0.517, 0.547, 0.575]),
        (6015.0, &[0.408, 0.433, 0.455, 0.477, 0.504, 0.527, 0.548, 0.576, 0.601, 0.627, 0.647, 0.67, 0.697, 0.717, 0.748]),
        (6020.0, &[0.126, 0.145, 0.168, 0.192, 0.218, 0.24, 0.266, 0.29, 0.311, 0.338, 0.358, 0.384, 0.409, 0.428, 0.456, 0.479]),
        (6519.0, &[0.376, 0.397, 0.419, 0.439, 0.465, 0.486, 0.507, 0.53, 0.549, 0.575, 0.595, 0.625, 0.644, 0.669, 0.687, 0.704, 0.734]),
        (6531.0, &[0.111, 0.131, 0.157, 0.176, 0.199, 0.224, 0.245, 0.265, 0.29, 0.311, 0.334, 0.353, 0.374, 0.397, 0.418, 0.44]),
    ];
    const COMPARED_STATIC_REV_PER_MIN: [f64; 9] =
        [4144.0, 4409.0, 4711.0, 5011.0, 5325.0, 5624.0, 5938.0, 6231.0, 6542.0];

    #[test]
    fn every_compared_point_is_answered_and_the_ranges_are_their_extremes() {
        let mut questions = Vec::new();
        for (speed, ratios) in COMPARED_SWEEPS {
            for &j in ratios {
                questions.push(ask(speed, j));
            }
        }
        assert_eq!(questions.len(), 107);
        let moving = questions.clone();
        for speed in COMPARED_STATIC_REV_PER_MIN {
            questions.push(ask(speed, 0.0));
        }
        let (mut low, mut high) = (f64::INFINITY, 0.0f64);
        for question in &questions {
            solve(question).unwrap_or_else(|reject| panic!("{question:?}: {reject}"));
            let reynolds = chord_reynolds_number(question);
            low = low.min(reynolds);
            high = high.max(reynolds);
        }
        println!("compared chord Reynolds numbers {low} to {high}");
        assert_eq!((low.floor(), high.ceil()), (REYNOLDS_NUMBER_MIN, REYNOLDS_NUMBER_MAX));
        let speeds = |list: &[PropellerQuestion]| {
            list.iter().map(|q| q.rotational_speed_rev_per_min).fold((f64::INFINITY, 0.0f64), |(a, b), s| (a.min(s), b.max(s)))
        };
        assert_eq!(speeds(&moving), (ROTATIONAL_SPEED_MIN_REV_PER_MIN, ROTATIONAL_SPEED_MAX_REV_PER_MIN));
        assert_eq!(speeds(&questions[107..]), (STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN, STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN));
        let ratios: Vec<f64> = COMPARED_SWEEPS.iter().flat_map(|(_, r)| r.iter().copied()).collect();
        assert_eq!(ratios.iter().copied().fold(f64::INFINITY, f64::min), ADVANCE_RATIO_MIN);
        assert_eq!(ratios.iter().copied().fold(0.0, f64::max), ADVANCE_RATIO_MAX);
        assert!((CHORD_AT_THREE_QUARTERS_M - 0.016383).abs() < 1e-15);
    }

    /// The airspeed whose computed advance ratio at `rev_per_min` is exactly
    /// the 64-bit number `wanted`, found by stepping the airspeed one 64-bit
    /// number at a time from the caller's product.
    fn airspeed_computing_to(rev_per_min: f64, wanted: f64) -> Option<f64> {
        let mut question = ask(rev_per_min, wanted);
        for _ in 0..64 {
            let computed = advance_ratio(&question);
            if computed == wanted {
                return Some(question.airspeed_m_s);
            }
            question.airspeed_m_s = if computed < wanted {
                question.airspeed_m_s.next_up()
            } else {
                question.airspeed_m_s.next_down()
            };
        }
        None
    }

    fn units_from(end: f64, value: f64) -> i64 {
        value.to_bits() as i64 - end.to_bits() as i64
    }

    /// The allowances are the published numbers of units, and the two
    /// answered ends are that many 64-bit numbers beyond the ends of the range.
    #[test]
    fn the_allowance_is_five_units_below_and_four_above() {
        assert_eq!((ADVANCE_RATIO_MIN_ALLOWANCE_ULP, ADVANCE_RATIO_MAX_ALLOWANCE_ULP), (5, 4));
        let unit_low = ADVANCE_RATIO_MIN.next_up() - ADVANCE_RATIO_MIN;
        let unit_high = ADVANCE_RATIO_MAX.next_up() - ADVANCE_RATIO_MAX;
        assert_eq!(unit_low, 2.0f64.powi(-56));
        assert_eq!(unit_high, 2.0f64.powi(-53));
        assert_eq!(ADVANCE_RATIO_MIN - ADVANCE_RATIO_LOWEST_ANSWERED, 5.0 * unit_low);
        assert_eq!(ADVANCE_RATIO_HIGHEST_ANSWERED - ADVANCE_RATIO_MAX, 4.0 * unit_high);
        assert_eq!(units_from(ADVANCE_RATIO_MIN, ADVANCE_RATIO_LOWEST_ANSWERED), -5);
        assert_eq!(units_from(ADVANCE_RATIO_MAX, ADVANCE_RATIO_HIGHEST_ANSWERED), 4);
        // The bound of the derivation, six roundings of 2^-53 each, in units
        // in the last place of each end: its whole part is each allowance.
        let u = 2.0f64.powi(-53);
        let six = (1.0 - u).powi(-6) - 1.0;
        assert!(six >= 6.0 * u && six < 6.0 * u * (1.0 + 1e-9));
        let low_units = ADVANCE_RATIO_MIN * six / unit_low;
        let high_units = ADVANCE_RATIO_MAX * six / unit_high;
        assert!((low_units - 5.328).abs() < 1e-9 && (high_units - 4.488).abs() < 1e-9);
        assert_eq!(low_units.floor(), f64::from(ADVANCE_RATIO_MIN_ALLOWANCE_ULP));
        assert_eq!(high_units.floor(), f64::from(ADVANCE_RATIO_MAX_ALLOWANCE_ULP));
    }

    fn allowance(end: f64) -> i64 {
        if end == ADVANCE_RATIO_MIN {
            i64::from(ADVANCE_RATIO_MIN_ALLOWANCE_ULP)
        } else {
            i64::from(ADVANCE_RATIO_MAX_ALLOWANCE_ULP)
        }
    }

    /// An advance ratio of exactly 0.111 and of exactly 0.748, asked as a
    /// caller asks it, V = J (N / 60) D in 64-bit arithmetic, is answered at
    /// every whole rotational speed of the range, and at the advance ratio
    /// the solver itself computed from that airspeed.
    #[test]
    fn both_ends_are_answered_at_every_whole_rotational_speed() {
        for end in [ADVANCE_RATIO_MIN, ADVANCE_RATIO_MAX] {
            let (mut off_the_end, mut beyond, mut farthest) = (0, 0, 0i64);
            let mut speeds = 0;
            for whole in 4007..=6531u32 {
                let speed = f64::from(whole);
                let question = ask_at(speed, end);
                let computed = advance_ratio(&question);
                let units = units_from(end, computed);
                if units != 0 {
                    off_the_end += 1;
                }
                let outside = if end == ADVANCE_RATIO_MIN { units < 0 } else { units > 0 };
                if outside {
                    beyond += 1;
                }
                farthest = farthest.max(units.abs());
                check_question(&question).unwrap_or_else(|reject| panic!("{whole} {end}: {reject}"));
                let solved = solve(&question).unwrap_or_else(|reject| panic!("{whole} {end}: {reject}"));
                // answered at its own computed advance ratio, not at the end
                assert_eq!(solved.advance_ratio.to_bits(), computed.to_bits(), "{whole} {end}");
                assert!(solved.thrust_n.is_finite() && solved.shaft_power_w > 0.0, "{whole} {end}");
                assert!(solved.worst_ring_balance_gap <= BALANCE_TOLERANCE, "{whole} {end}");
                speeds += 1;
            }
            println!(
                "J {end}: {speeds} whole speeds, {off_the_end} computed off the end, {beyond} beyond it, farthest {farthest} units in the last place"
            );
            assert_eq!(speeds, 2525);
            // without the allowance these would be refused: 600 at 0.111 and 446 at 0.748
            assert_eq!(beyond, if end == ADVANCE_RATIO_MIN { 600 } else { 446 });
            assert!(farthest <= 3, "{farthest}");
        }
        // Exact decimal questions at both ends.
        for (speed, airspeed) in [(6000.0, 18.9992), (6000.0, 2.8194), (4500.0, 14.2494), (4500.0, 2.11455)] {
            let question = PropellerQuestion {
                rotational_speed_rev_per_min: speed,
                airspeed_m_s: airspeed,
                air_density_kg_m3: 1.225,
            };
            let solved = solve(&question).unwrap_or_else(|reject| panic!("{speed} {airspeed}: {reject}"));
            assert_eq!(solved.advance_ratio.to_bits(), advance_ratio(&question).to_bits());
        }
        // The other orders of the caller's three operations stay inside too.
        for whole in 4007..=6531u32 {
            let speed = f64::from(whole);
            for end in [ADVANCE_RATIO_MIN, ADVANCE_RATIO_MAX] {
                for airspeed in [
                    end * speed * DIAMETER_M / 60.0,
                    end * speed / 60.0 * DIAMETER_M,
                    end * DIAMETER_M * speed / 60.0,
                    end * (speed * DIAMETER_M / 60.0),
                    end * (DIAMETER_M / 60.0) * speed,
                ] {
                    let question = PropellerQuestion {
                        rotational_speed_rev_per_min: speed,
                        airspeed_m_s: airspeed,
                        air_density_kg_m3: density_for(speed),
                    };
                    check_question(&question).unwrap_or_else(|reject| panic!("{whole} {end}: {reject}"));
                    assert!(units_from(end, advance_ratio(&question)).abs() <= allowance(end));
                }
            }
        }
    }

    /// A question inside the allowance is solved at its own advance ratio:
    /// the advance ratio it reports is the one computed, bit for bit, never
    /// the end, and its efficiency is formed with that advance ratio.
    #[test]
    fn inside_the_allowance_the_answer_is_at_the_questions_own_advance_ratio() {
        for (end, outward) in [(ADVANCE_RATIO_MIN, -1i64), (ADVANCE_RATIO_MAX, 1)] {
            let units = allowance(end);
            let mut found = [0u32; 6];
            for speed in [4007.0, 4008.0, 5000.0, 5018.0, 6015.0, 6530.0, 6531.0] {
                for step in 1..=units {
                    let wanted = f64::from_bits((end.to_bits() as i64 + outward * step) as u64);
                    // Not every 64-bit advance ratio is the quotient of some
                    // airspeed at a given speed; those that are, are asked.
                    let Some(airspeed) = airspeed_computing_to(speed, wanted) else {
                        continue;
                    };
                    let question = PropellerQuestion {
                        rotational_speed_rev_per_min: speed,
                        airspeed_m_s: airspeed,
                        air_density_kg_m3: density_for(speed),
                    };
                    let solved = solve(&question).unwrap_or_else(|reject| panic!("{speed} {end} {step}: {reject}"));
                    assert_eq!(solved.advance_ratio.to_bits(), wanted.to_bits());
                    assert_ne!(solved.advance_ratio.to_bits(), end.to_bits());
                    let own = solved.thrust_coefficient * wanted / solved.power_coefficient;
                    assert_eq!(solved.efficiency.to_bits(), own.to_bits());
                    found[step as usize] += 1;
                }
            }
            // every distance up to the allowance itself was met and answered
            for step in 1..=units {
                assert!(found[step as usize] >= 1, "{end}: {step} units never asked");
            }
        }
    }

    /// One unit in the last place beyond the allowance, and a little more,
    /// is refused by name, with both allowances in the sentence.
    #[test]
    fn beyond_the_allowance_is_refused_and_the_sentence_names_the_allowance() {
        let past_low = u64::from(ADVANCE_RATIO_MIN_ALLOWANCE_ULP) + 1;
        let past_high = u64::from(ADVANCE_RATIO_MAX_ALLOWANCE_ULP) + 1;
        let just_below = f64::from_bits(ADVANCE_RATIO_MIN.to_bits() - past_low);
        let just_above = f64::from_bits(ADVANCE_RATIO_MAX.to_bits() + past_high);
        assert!(just_below < ADVANCE_RATIO_LOWEST_ANSWERED && just_above > ADVANCE_RATIO_HIGHEST_ANSWERED);
        assert_eq!(ADVANCE_RATIO_LOWEST_ANSWERED.next_down(), just_below);
        assert_eq!(ADVANCE_RATIO_HIGHEST_ANSWERED.next_up(), just_above);
        let (mut refused, mut one_past_below, mut one_past_above) = (0, 0, 0);
        for speed in [4007.0, 4008.0, 5000.0, 5018.0, 6000.0, 6530.0, 6531.0] {
            for wanted in [
                just_below,
                just_above,
                f64::from_bits(ADVANCE_RATIO_MIN.to_bits() - past_low - 3),
                f64::from_bits(ADVANCE_RATIO_MAX.to_bits() + past_high + 3),
                ADVANCE_RATIO_MIN * (1.0 - 1.0e-12),
                ADVANCE_RATIO_MAX * (1.0 + 1.0e-12),
            ] {
                // Not every 64-bit advance ratio is the quotient of some
                // airspeed at a given speed; those that are, are asked.
                let Some(airspeed) = airspeed_computing_to(speed, wanted) else {
                    continue;
                };
                let question = PropellerQuestion {
                    rotational_speed_rev_per_min: speed,
                    airspeed_m_s: airspeed,
                    air_density_kg_m3: density_for(speed),
                };
                let reject = solve(&question).expect_err("beyond the allowance");
                assert_eq!(reject, PropellerReject::AdvanceRatioOutOfRange { advance_ratio: wanted });
                assert_eq!(check_question(&question), Err(reject));
                let sentence = reject.to_string();
                assert!(
                    sentence.contains(
                        "by more than the allowance of 5 units in the last place below (ADVANCE_RATIO_MIN_ALLOWANCE_ULP) and 4 above (ADVANCE_RATIO_MAX_ALLOWANCE_ULP)"
                    ),
                    "{sentence}"
                );
                assert!(sentence.contains("is outside 0.111..=0.748, the advance ratios compared with measurement"), "{sentence}");
                refused += 1;
                one_past_below += u32::from(wanted == just_below);
                one_past_above += u32::from(wanted == just_above);
            }
        }
        assert!(refused >= 20, "{refused}");
        assert!(one_past_below >= 1 && one_past_above >= 1, "{one_past_below} {one_past_above}");
    }

    /// The balance over the whole disc is checked at acceptance with its own
    /// name: with its tolerance set to zero and the rings' left as they are,
    /// every answer whose disc gap is not exactly zero is refused.
    #[test]
    fn a_disc_balance_that_cannot_be_met_is_broken_by_name() {
        let strict = Settings { disc_balance_tolerance: 0.0, ..Settings::LOCKED };
        let mut broken = 0;
        for k in 0..20 {
            let question = ask(5000.0, 0.2 + 0.02 * f64::from(k));
            let answered = solve(&question).expect("answered as locked");
            match solve_with(&question, &strict) {
                Err(PropellerReject::DiscBalanceBroken { gap }) => {
                    assert_eq!(gap, answered.thrust_balance_gap);
                    assert!(gap > 0.0 && gap < BALANCE_TOLERANCE);
                    broken += 1;
                }
                Ok(solved) => assert_eq!(solved.thrust_balance_gap, 0.0),
                Err(other) => panic!("{other}"),
            }
        }
        assert!(broken >= 15, "{broken}");
        let sentence = PropellerReject::DiscBalanceBroken { gap: 2.0e-9 }.to_string();
        assert_eq!(
            sentence,
            "over the whole disc the blade thrust and the axial momentum differ by 0.000000002 of the sum of the rings' larger sides"
        );
        assert!(!sentence.contains("ring 34"));
    }

    #[test]
    fn too_few_halvings_is_not_settled() {
        let short = Settings { max_bisections: 5, ..Settings::LOCKED };
        assert_eq!(
            solve_with(&ask(5000.0, 0.4), &short),
            Err(PropellerReject::NotSettled { ring: 0, iterations: 5 })
        );
    }

    #[test]
    fn a_balance_that_cannot_be_met_is_broken_by_name() {
        let strict = Settings { balance_tolerance: 0.0, ..Settings::LOCKED };
        let mut broken = 0;
        for k in 0..20 {
            match solve_with(&ask(5000.0, 0.2 + 0.02 * f64::from(k)), &strict) {
                Err(PropellerReject::MomentumBalanceBroken { gap, .. }) => {
                    assert!(gap > 0.0 && gap < BALANCE_TOLERANCE);
                    broken += 1;
                }
                Ok(solved) => assert_eq!(solved.worst_ring_balance_gap, 0.0),
                Err(other) => panic!("{other}"),
            }
        }
        assert!(broken >= 15, "{broken}");
    }

    #[test]
    fn a_section_that_is_not_a_number_is_named() {
        let bad = Settings { cd0: f64::NAN, ..Settings::LOCKED };
        assert_eq!(
            solve_with(&ask(5000.0, 0.4), &bad),
            Err(PropellerReject::NonFiniteValue { ring: 0 })
        );
        // A section that only pushes backward can balance no ring.
        let backward = Settings { alpha_zero_lift_rad: 3.0, ..Settings::LOCKED };
        assert!(matches!(
            solve_with(&ask(5000.0, 0.0), &backward),
            Err(PropellerReject::NoBalance { .. })
        ));
    }

    #[test]
    fn the_geometry_is_the_measured_table_between_its_rows() {
        for (x, chord, beta) in GEOMETRY {
            let (c, b) = geometry_at(x);
            assert!((c - chord).abs() < 1e-15 && (b.to_degrees() - beta).abs() < 1e-12, "{x}");
        }
        let (c, b) = geometry_at(0.7625);
        assert!((c - 0.12475).abs() < 1e-12 && (b.to_degrees() - 16.5025).abs() < 1e-9);
        // 7 inch pitch at three quarters radius: pi * 0.75 * tan(16.74 deg) * 10 in
        let pitch_in = PI * 0.75 * 16.74f64.to_radians().tan() * 10.0;
        assert!((pitch_in - 7.0).abs() < 0.1, "{pitch_in}");
    }
}
