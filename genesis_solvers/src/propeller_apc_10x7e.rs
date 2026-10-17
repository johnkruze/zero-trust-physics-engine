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
//! thrust and power coefficients depend on the advance ratio alone. A
//! rotational speed acts on a real section through those two numbers, so
//! the rotational speed is held by them: the chord Reynolds number inside
//! the band the section's two constants are taken over, and the tip Mach
//! number at or below [`TIP_MACH_MAX`], because the section law is
//! incompressible. Any speed above zero is otherwise answered, up to
//! [`ROTATIONAL_SPEED_MAX_REV_PER_MIN`], where the tip meets Mach 0.3 with
//! the air still.
//!
//! **What was compared, and what is answered.** With the air moving, the
//! measured points span 4007 to 6531 rev/min and advance ratio 0.111 to
//! 0.748; at zero airspeed, 4144 to 6542 rev/min. Until this version the
//! rotational speed was held to those spans. It is now answered past them
//! wherever the Reynolds band and the tip Mach number hold: at 1.225 kg/m³
//! and zero airspeed that is about 4014 to 6828 rev/min, and in thinner or
//! denser air, or with the air moving, other speeds. Nothing outside the
//! compared speeds was compared. The advance ratio is answered across its
//! range too: blade-element momentum theory and the section hold across it,
//! from zero airspeed up, and a ring whose equations have no root stops by
//! name (`NoBalance`). The
//! measurements give thrust and power coefficients only: the density of the
//! tunnel air was not recorded, and every comparison was sent at
//! 1.225 kg/m³. Density enters this model only by scaling thrust and power,
//! and any density above zero is answered, but it moves the chord Reynolds
//! number the section's coefficients depend on. The section has one
//! zero-lift angle and one minimum drag and no law for how they move with
//! the Reynolds number, so that Reynolds number at three quarters of the
//! radius, formed at the question's own density, is held inside
//! [`REYNOLDS_NUMBER_MIN`] to [`REYNOLDS_NUMBER_MAX`], the band its two
//! constants are taken over: the band the compared points span at
//! 1.225 kg/m³, over which the zero-lift angle that fits each sweep alone
//! already drifts from about −2.6 to −3.8 degrees. The section is
//! incompressible, so the tip Mach number is held to [`TIP_MACH_MAX`].
//! Through engine 0.8.0 an advance ratio outside 0.111 to 0.748 and a
//! density outside 1.0 to 1.3 kg/m³ were refused by name. The comparison
//! that fixed the compared numbers is `fix_0.7.0/compared_envelope.py` in
//! the evidence folder.
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
//! found the same numbers (`evidence/verification/second_party/V-27/`).
//!
//! The work that fixed the constants, the protocol and its fingerprint, and
//! every result are in `evidence/research/propeller_apc_thin_electric_10x7/`.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
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

/// The rotational speed must be above this, rev/min: the coefficients are
/// per turn of the blades, and the rings' angular momentum is that of blades
/// turning forward.
pub const ROTATIONAL_SPEED_ABOVE_REV_PER_MIN: f64 = 0.0;
/// The highest rotational speed answered, rev/min: at it, with the air
/// still, the tip meets the air at [`TIP_MACH_MAX`] of [`SPEED_OF_SOUND_M_S`],
/// 102.087 m/s, about 7676 rev/min. Every question with the air moving meets
/// the tip Mach limit at a lower speed. Inside it the rotational speed is
/// held by the chord Reynolds number ([`REYNOLDS_NUMBER_MIN`] to
/// [`REYNOLDS_NUMBER_MAX`]) and the tip Mach number, the two things through
/// which a rotational speed acts on a section whose coefficients depend on
/// the advance ratio alone.
pub const ROTATIONAL_SPEED_MAX_REV_PER_MIN: f64 = TIP_MACH_MAX * SPEED_OF_SOUND_M_S / (PI * DIAMETER_M) * 60.0;
/// Lowest and highest rotational speed compared with the air moving, rev/min:
/// the slowest and the fastest measured sweep. The fitted sweeps are at 4007
/// to 5018 and the held-back sweeps at 6015 to 6531. Stated, not enforced:
/// until this version a speed outside them was refused by name.
pub const COMPARED_ROTATIONAL_SPEED_MIN_REV_PER_MIN: f64 = 4007.0;
pub const COMPARED_ROTATIONAL_SPEED_MAX_REV_PER_MIN: f64 = 6531.0;
/// Lowest and highest rotational speed compared at zero airspeed, rev/min:
/// the slowest and the fastest static row. Stated, not enforced.
pub const COMPARED_STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN: f64 = 4144.0;
pub const COMPARED_STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN: f64 = 6542.0;
/// The air density must be above this, kg/m³. No density was measured: the
/// measurements give coefficients only, and every comparison was sent at
/// 1.225. Above zero the chord Reynolds number is the limit.
pub const AIR_DENSITY_ABOVE_KG_M3: f64 = 0.0;
/// Speed of sound in the tip Mach number, m/s: dry air at 15 °C, the
/// standard atmosphere at sea level. The question holds no temperature.
pub const SPEED_OF_SOUND_M_S: f64 = 340.29;
/// Highest tip Mach number answered, the relative speed of the air at the
/// tip over [`SPEED_OF_SOUND_M_S`]: the blade section is incompressible.
/// At 6531 rev/min and zero airspeed the tip Mach number is 0.255.
pub const TIP_MACH_MAX: f64 = 0.3;
/// Viscosity of the air in the chord Reynolds number, Pa·s.
pub const AIR_VISCOSITY_PA_S: f64 = 1.81e-5;
/// Chord at three quarters of the radius, m: 0.129 R from the geometry.
pub const CHORD_AT_THREE_QUARTERS_M: f64 = GEOMETRY[12].1 * DIAMETER_M / 2.0;
/// The density every comparison was sent at, kg/m³.
pub const COMPARED_AIR_DENSITY_KG_M3: f64 = 1.225;
/// Lowest and highest chord Reynolds number at three quarters of the radius
/// ([`chord_reynolds_number`]) answered, the band the section's two
/// constants are taken over: the section has no Reynolds-number law to carry
/// them past it. The band is the lowest and highest of the 116 compared
/// points at 1.225 kg/m³, 44399.03 (4007 rev/min, advance ratio
/// 0.144) and 75515.76 (6519 rev/min, advance ratio 0.734), each rounded
/// outward to a whole number.
pub const REYNOLDS_NUMBER_MIN: f64 = 44_399.0;
pub const REYNOLDS_NUMBER_MAX: f64 = 75_516.0;
/// The lowest and highest measured advance ratio J = V / (n D) compared
/// with the air moving, 0.111 at 6531 rev/min and 0.748 at 6015 rev/min.
/// Stated, not enforced.
pub const ADVANCE_RATIO_MIN: f64 = 0.111;
pub const ADVANCE_RATIO_MAX: f64 = 0.748;
/// The highest airspeed answered, m/s: at a tip Mach number of
/// [`TIP_MACH_MAX`] the airspeed alone, with the blades still, is this.
/// Every turning propeller meets the tip Mach limit at a lower airspeed.
pub const AIRSPEED_MAX_M_S: f64 = TIP_MACH_MAX * SPEED_OF_SOUND_M_S;

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
    /// Thrust times airspeed over shaft power. Zero at zero airspeed. It is
    /// an efficiency only while thrust and shaft power are both positive:
    /// past zero thrust, near advance ratio 0.83, the air drives the blades.
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
    /// The rotational speed is not above [`ROTATIONAL_SPEED_ABOVE_REV_PER_MIN`]: the coefficients are per turn of the blades, and the rings' angular momentum is that of blades turning forward.
    RotationalSpeedNotPositive { rotational_speed_rev_per_min: f64 },
    /// The air density is not above [`AIR_DENSITY_ABOVE_KG_M3`]: no air.
    AirDensityNotPositive { air_density_kg_m3: f64 },
    /// The airspeed is negative.
    AirspeedNegative { airspeed_m_s: f64 },
    /// The relative speed of the air at the tip over [`SPEED_OF_SOUND_M_S`] is above [`TIP_MACH_MAX`]: the flow over the blade is compressible, and the section is not.
    TipMachAboveIncompressible { tip_mach_number: f64 },
    /// The chord Reynolds number at three quarters of the radius, at the question's density, is outside [`REYNOLDS_NUMBER_MIN`] to [`REYNOLDS_NUMBER_MAX`], the band the blade section's two constants are taken over: the section has one zero-lift angle and one minimum drag and no law for how they move with the Reynolds number, and across this band alone the zero-lift angle that fits each sweep alone drifts from about −2.6 to −3.8 degrees.
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
            Self::RotationalSpeedNotPositive { rotational_speed_rev_per_min } => write!(
                f,
                "rotational speed {rotational_speed_rev_per_min} rev/min is not above zero: the coefficients are per turn of the blades, and the rings' angular momentum is that of blades turning forward"
            ),
            Self::AirDensityNotPositive { air_density_kg_m3 } => write!(
                f,
                "air density {air_density_kg_m3} kg/m³ is not above zero"
            ),
            Self::AirspeedNegative { airspeed_m_s } => {
                write!(f, "airspeed {airspeed_m_s} m/s is negative")
            }
            Self::TipMachAboveIncompressible { tip_mach_number } => write!(
                f,
                "tip Mach number {tip_mach_number} is above {TIP_MACH_MAX}, at a speed of sound of {SPEED_OF_SOUND_M_S} m/s: the flow over the blade is compressible, and the section is not"
            ),
            Self::ReynoldsNumberOutOfRange { reynolds_number } => write!(
                f,
                "chord Reynolds number {reynolds_number} at three quarters of the radius is outside {REYNOLDS_NUMBER_MIN} to {REYNOLDS_NUMBER_MAX}, the band the blade section's two constants are taken over: the section has no law for how its zero-lift angle and minimum drag move with the Reynolds number"
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

/// The relative speed of the air at the tip over [`SPEED_OF_SOUND_M_S`].
pub fn tip_mach_number(question: &PropellerQuestion) -> f64 {
    let revolutions_per_second = question.rotational_speed_rev_per_min / 60.0;
    let tangential = 2.0 * PI * revolutions_per_second * (DIAMETER_M / 2.0);
    let airspeed = question.airspeed_m_s;
    (tangential * tangential + airspeed * airspeed).sqrt() / SPEED_OF_SOUND_M_S
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
    if !(rotational_speed_rev_per_min > ROTATIONAL_SPEED_ABOVE_REV_PER_MIN) {
        return Err(PropellerReject::RotationalSpeedNotPositive { rotational_speed_rev_per_min });
    }
    if !(air_density_kg_m3 > AIR_DENSITY_ABOVE_KG_M3) {
        return Err(PropellerReject::AirDensityNotPositive { air_density_kg_m3 });
    }
    let tip_mach_number = tip_mach_number(question);
    if !(tip_mach_number <= TIP_MACH_MAX) {
        return Err(PropellerReject::TipMachAboveIncompressible { tip_mach_number });
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
    let sin_alpha = math::sin(alpha);
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
    let (sin_phi, cos_phi) = (math::sin(phi), math::cos(phi));
    let exponent = (blades / 2.0) * (1.0 - x) / (x * sin_phi);
    let tip_loss = (2.0 / PI) * math::acos(math::exp(-exponent));
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
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &PropellerQuestion) -> Result<PropellerSolution, PropellerReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &PropellerQuestion) -> Result<PropellerSolution, PropellerReject> {
    solve_with(question, &Settings::LOCKED)
}

impl NormalOrZero for PropellerSolution {
    fn normal_or_zero(&mut self) {
        self.advance_ratio.normal_or_zero();
        self.thrust_coefficient.normal_or_zero();
        self.power_coefficient.normal_or_zero();
        self.thrust_n.normal_or_zero();
        self.torque_n_m.normal_or_zero();
        self.shaft_power_w.normal_or_zero();
        self.efficiency.normal_or_zero();
        self.thrust_balance_gap.normal_or_zero();
        self.worst_ring_balance_gap.normal_or_zero();
        self.inflow_angle_rad.normal_or_zero();
        self.ring_thrust_n.normal_or_zero();
        self.ring_torque_n_m.normal_or_zero();
    }
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
            ask(COMPARED_STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN, 0.0),
            ask(5000.0, 0.0),
            ask(COMPARED_STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN, 0.0),
        ];
        for speed in [COMPARED_ROTATIONAL_SPEED_MIN_REV_PER_MIN, 5000.0, COMPARED_ROTATIONAL_SPEED_MAX_REV_PER_MIN] {
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
            (PropellerQuestion { rotational_speed_rev_per_min: 0.0, ..good }, "RotationalSpeedNotPositive"),
            (PropellerQuestion { rotational_speed_rev_per_min: -0.0, ..rest }, "RotationalSpeedNotPositive"),
            (PropellerQuestion { rotational_speed_rev_per_min: -5000.0, ..good }, "RotationalSpeedNotPositive"),
            // the least speed above zero, the air still: the section meets no Reynolds number in the band
            (PropellerQuestion { rotational_speed_rev_per_min: f64::from_bits(1), ..rest }, "ReynoldsNumberOutOfRange"),
            // past the top: the tip above Mach 0.3 with the air still
            (PropellerQuestion { rotational_speed_rev_per_min: ROTATIONAL_SPEED_MAX_REV_PER_MIN.next_up(), air_density_kg_m3: 0.9, ..rest }, "TipMachAboveIncompressible"),
            (PropellerQuestion { rotational_speed_rev_per_min: 8000.0, air_density_kg_m3: 0.9, ..rest }, "TipMachAboveIncompressible"),
            // 4100 rev/min at rest, under the compared 4144, is inside the Reynolds band; 3900 is not
            (PropellerQuestion { rotational_speed_rev_per_min: 3900.0, ..rest }, "ReynoldsNumberOutOfRange"),
            (PropellerQuestion { air_density_kg_m3: 0.0, ..good }, "AirDensityNotPositive"),
            (PropellerQuestion { air_density_kg_m3: -1.225, ..good }, "AirDensityNotPositive"),
            (PropellerQuestion { airspeed_m_s: -0.001, ..good }, "AirspeedNegative"),
            // past the tip Mach number of an incompressible section
            (ask(6531.0, 2.0), "TipMachAboveIncompressible"),
            (PropellerQuestion { airspeed_m_s: AIRSPEED_MAX_M_S.next_up(), air_density_kg_m3: 0.6, ..good }, "TipMachAboveIncompressible"),
            // dense air at a high airspeed, past the Reynolds band
            (ask(5000.0, 2.5), "ReynoldsNumberOutOfRange"),
            // never compared: the slowest sweep at the lowest advance ratio,
            // the fastest at the highest, at 1.225 kg/m³
            (ask(COMPARED_ROTATIONAL_SPEED_MIN_REV_PER_MIN, ADVANCE_RATIO_MIN), "ReynoldsNumberOutOfRange"),
            (ask(COMPARED_ROTATIONAL_SPEED_MAX_REV_PER_MIN, ADVANCE_RATIO_MAX), "ReynoldsNumberOutOfRange"),
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
        // The sentence names the limit and no data.
        let still = check_question(&PropellerQuestion { rotational_speed_rev_per_min: 0.0, ..good }).unwrap_err();
        assert_eq!(
            still.to_string(),
            "rotational speed 0 rev/min is not above zero: the coefficients are per turn of the blades, and the rings' angular momentum is that of blades turning forward"
        );
        // The top is the tip Mach limit with the air still: about 7676 rev/min.
        assert!((ROTATIONAL_SPEED_MAX_REV_PER_MIN - 7676.0).abs() < 0.5, "{ROTATIONAL_SPEED_MAX_REV_PER_MIN}");
        // the ends of each range are inside it
        for question in [
            ask_at(COMPARED_ROTATIONAL_SPEED_MIN_REV_PER_MIN, ADVANCE_RATIO_MIN),
            ask(COMPARED_ROTATIONAL_SPEED_MIN_REV_PER_MIN, ADVANCE_RATIO_MAX),
            ask(COMPARED_ROTATIONAL_SPEED_MAX_REV_PER_MIN, ADVANCE_RATIO_MIN),
            ask_at(COMPARED_ROTATIONAL_SPEED_MAX_REV_PER_MIN, ADVANCE_RATIO_MAX),
            ask(COMPARED_STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN, 0.0),
            ask(COMPARED_STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN, 0.0),
            PropellerQuestion { air_density_kg_m3: 1.0, ..good },
            PropellerQuestion { air_density_kg_m3: 1.3, ..good },
            PropellerQuestion { airspeed_m_s: 0.0, ..good },
            PropellerQuestion { airspeed_m_s: -0.0, ..good },
        ] {
            solve(&question).unwrap_or_else(|reject| panic!("{question:?}: {reject}"));
        }
        // No airspeed above the published highest is answered at any
        // rotational speed or density: its tip Mach number is already above
        // the limit.
        for speed in [COMPARED_ROTATIONAL_SPEED_MIN_REV_PER_MIN, 5000.0, COMPARED_ROTATIONAL_SPEED_MAX_REV_PER_MIN] {
            for density in [0.1, 1.225, 2.0] {
                let above = PropellerQuestion {
                    rotational_speed_rev_per_min: speed,
                    airspeed_m_s: AIRSPEED_MAX_M_S.next_up(),
                    air_density_kg_m3: density,
                };
                assert!(matches!(check_question(&above), Err(PropellerReject::TipMachAboveIncompressible { .. })));
            }
        }
        assert_eq!(
            PropellerReject::TipMachAboveIncompressible { tip_mach_number: 0.31 }.to_string(),
            "tip Mach number 0.31 is above 0.3, at a speed of sound of 340.29 m/s: the flow over the blade is compressible, and the section is not"
        );
    }

    /// Past the advance ratios and densities compared, where engine 0.8.0
    /// refused by name: below 0.111 down to the first airspeed above zero,
    /// above 0.748 into the windmill state where the air drives the blades,
    /// and densities outside 1.0 to 1.3 kg/m³. Every ring balances and the
    /// disc's thrust meets the momentum it gives the air; the coefficients
    /// are the advance ratio's alone and thrust goes with density.
    #[test]
    fn past_the_compared_advance_ratios_and_densities_every_ring_balances() {
        let mut questions = vec![
            PropellerQuestion { airspeed_m_s: 1.0e-9, ..ask(5000.0, 0.0) },
            ask(5000.0, 0.05),
            ask(5000.0, 0.1109),
            ask(5000.0, 0.7481),
            ask(5000.0, 0.83),
            ask(5000.0, 1.5),
            ask(4007.0, 3.0),
        ];
        // At 5000 rev/min and 0.4 the Reynolds band holds 0.97 to 1.65 kg/m³;
        // thin air is answered where the blade meets it faster.
        for density in [0.98, 0.999, 1.301, 1.6, 1.64] {
            questions.push(PropellerQuestion { air_density_kg_m3: density, ..ask(5000.0, 0.4) });
        }
        questions.push(PropellerQuestion { air_density_kg_m3: 0.8, ..ask(6531.0, 1.0) });
        let reference = solve(&ask(5000.0, 0.4)).expect("inside");
        for question in questions {
            let solved = solve(&question).unwrap_or_else(|reject| panic!("{question:?}: {reject}"));
            assert!(solved.worst_ring_balance_gap <= BALANCE_TOLERANCE, "{question:?}");
            assert!(solved.thrust_balance_gap <= BALANCE_TOLERANCE, "{question:?}");
            let sum: f64 = solved.ring_thrust_n.iter().sum();
            assert!((sum - solved.thrust_n).abs() <= 1e-12 * solved.thrust_n.abs().max(1e-6));
            if question.airspeed_m_s == ask(5000.0, 0.4).airspeed_m_s {
                assert_eq!(solved.thrust_coefficient, reference.thrust_coefficient);
                let ratio = solved.thrust_n / reference.thrust_n;
                assert!((ratio - question.air_density_kg_m3 / 1.225).abs() < 1e-12, "{ratio}");
            }
        }
        // Between zero airspeed and 0.111 the thrust coefficient joins the
        // static one; past about 0.83 the thrust is negative, and past about
        // 0.88 so is the shaft power: the air drives the propeller.
        let rest = solve(&ask(5000.0, 0.0)).expect("at rest");
        let creeping = solve(&PropellerQuestion { airspeed_m_s: 1.0e-9, ..ask(5000.0, 0.0) }).expect("creeping");
        assert!((creeping.thrust_coefficient / rest.thrust_coefficient - 1.0).abs() < 1e-6);
        let windmill = solve(&ask(5000.0, 1.0)).expect("windmill");
        assert!(windmill.thrust_n < 0.0 && windmill.shaft_power_w < 0.0);
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

    /// Past the rotational speeds compared, where engine 0.12.0 refused by
    /// name, the speed is held by the chord Reynolds number and the tip Mach
    /// number alone: at rest at 1.225 kg/m³ down to 4100 rev/min and up to
    /// 6800; in denser air down to 3000 rev/min; in thinner air up to 7600,
    /// a tip Mach number of 0.298; and 1000 rev/min with the air at 40 m/s,
    /// the blades nearly windmilling. Each is answered with every ring and
    /// the disc in balance, or stopped by a limit of the ring equations by
    /// name; none is pulled back to a compared speed.
    #[test]
    fn past_the_compared_rotational_speeds_every_ring_balances() {
        let questions = [
            ask(4100.0, 0.0),
            ask(6800.0, 0.0),
            PropellerQuestion { air_density_kg_m3: 1.7, ..ask(3000.0, 0.3) },
            PropellerQuestion { air_density_kg_m3: 1.8, ..ask(3000.0, 0.0) },
            PropellerQuestion { air_density_kg_m3: 1.0, ..ask(7600.0, 0.2) },
            PropellerQuestion { air_density_kg_m3: 1.0, ..ask(7600.0, 0.0) },
            PropellerQuestion { rotational_speed_rev_per_min: 1000.0, airspeed_m_s: 40.0, air_density_kg_m3: 1.2 },
        ];
        let mut answered = 0;
        for question in questions {
            assert_eq!(check_question(&question), Ok(()), "{question:?}");
            let reynolds = chord_reynolds_number(&question);
            assert!((REYNOLDS_NUMBER_MIN..=REYNOLDS_NUMBER_MAX).contains(&reynolds) && tip_mach_number(&question) <= TIP_MACH_MAX);
            match solve(&question) {
                Ok(solved) => {
                    answered += 1;
                    assert!(solved.worst_ring_balance_gap <= BALANCE_TOLERANCE, "{question:?}");
                    assert!(solved.thrust_balance_gap <= BALANCE_TOLERANCE, "{question:?}");
                    assert_eq!(solved.inflow_angle_rad.len(), ANNULI);
                    let sum: f64 = solved.ring_thrust_n.iter().sum();
                    assert!((sum - solved.thrust_n).abs() <= 1e-12 * solved.thrust_n.abs().max(1e-6), "{question:?}");
                    let torque: f64 = solved.ring_torque_n_m.iter().sum();
                    assert!((torque - solved.torque_n_m).abs() <= 1e-12 * solved.torque_n_m.abs().max(1e-6), "{question:?}");
                    // The coefficients are the advance ratio's alone: the same as at a compared speed.
                    if question.airspeed_m_s == 0.0 {
                        let compared = solve(&ask(5000.0, 0.0)).expect("compared");
                        assert_eq!(solved.thrust_coefficient, compared.thrust_coefficient, "{question:?}");
                    }
                }
                Err(PropellerReject::NoBalance { .. } | PropellerReject::NotSettled { .. }) => {}
                Err(other) => panic!("{question:?}: {other}"),
            }
        }
        assert!(answered >= 6, "{answered}");
        // The lowest and highest static speeds the band holds at 1.225 kg/m³, about 4014 and 6828 rev/min.
        let band = |rev_per_min: f64| check_question(&ask(rev_per_min, 0.0)).is_ok();
        assert!(band(4015.0) && !band(4013.0) && band(6827.0) && !band(6830.0));
    }

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
        assert_eq!(speeds(&moving), (COMPARED_ROTATIONAL_SPEED_MIN_REV_PER_MIN, COMPARED_ROTATIONAL_SPEED_MAX_REV_PER_MIN));
        assert_eq!(speeds(&questions[107..]), (COMPARED_STATIC_ROTATIONAL_SPEED_MIN_REV_PER_MIN, COMPARED_STATIC_ROTATIONAL_SPEED_MAX_REV_PER_MIN));
        let ratios: Vec<f64> = COMPARED_SWEEPS.iter().flat_map(|(_, r)| r.iter().copied()).collect();
        assert_eq!(ratios.iter().copied().fold(f64::INFINITY, f64::min), ADVANCE_RATIO_MIN);
        assert_eq!(ratios.iter().copied().fold(0.0, f64::max), ADVANCE_RATIO_MAX);
        assert!((CHORD_AT_THREE_QUARTERS_M - 0.016383).abs() < 1e-15);
    }

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
