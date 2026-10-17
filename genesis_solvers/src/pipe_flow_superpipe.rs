//! Fully developed turbulent flow of air through the smooth Princeton Superpipe.
//!
//! The pipe is the Superpipe of A. J. Smits' group at Princeton University:
//! compressed air, a test section of 129.36 mm bore, wall roughness 0.15 um
//! rms, bulk Reynolds numbers from 3.16e4 to 3.57e7. The solver settles the
//! steady, axially uniform mean flow at one bulk Reynolds number:
//!
//! ```text
//! (1/r) d/dr [ r (nu + l^2 |dU/dr|) dU/dr ] = (1/rho) dp/dx
//! l = kappa R [0.35 - 0.20 (1 - y/R)^2 - 0.15 (1 - y/R)^4] (1 - exp(-y+/A+))
//! kappa = 0.41, A+ = 26
//! ```
//!
//! a mixing length with van Driest damping, whose outer shape is Nikuradse's
//! pipe mixing length with its wall slope set to kappa. Finite volumes on the
//! radius, in units of the radius and the bulk velocity; Newton's method on
//! every node's momentum balance and the volume flux, with the pressure
//! gradient as the extra unknown, each iteration one tridiagonal solve
//! bordered by one row and column. The damping takes the friction velocity
//! of the previous iterate.
//!
//! **Nothing is fitted to a measurement.** kappa and A+ are the literature's.
//! The outer shape is not a published form: it is Nikuradse's distribution
//! rescaled by 0.41/0.40 to wall slope kappa. It was chosen on the opened
//! set's friction factors (the twelve Zagarola-Smits profiles below Reynolds
//! number 1e6) among five forms, four published and this one (1.3 percent
//! rms against 2.2 for Nikuradse's form as published, which had the better
//! opened centreline error, 1.2 percent against 1.4), and frozen before any
//! held-back file was opened. The grid was
//! fixed against itself: from 1600 to 6400 intervals the friction factor moves
//! by 1.1e-5 and the centreline ratio by 1.3e-5 (relative) at either end of
//! the range.
//!
//! **What the checks are.** The volume flux is a linear constraint of each
//! Newton step and is checked after every iteration. The largest node
//! imbalance, over the pressure force on the section, is computed every
//! iteration; the solver stops by name if it is not finite, grows past a cap,
//! or has not fallen below tolerance, with the friction velocity settled,
//! within the iteration cap. At acceptance the momentum ledger (shear force on
//! the grid face nearest the wall against the pressure force on the section
//! it encloses; it is the sum of the node balances, so it bounds their total
//! to 1e-10 where each node is held to 1e-11 alone) and a velocity rising
//! from wall to axis are checked. These say the
//! equations were solved, not that they describe the air.
//!
//! **The comparison of 2026-10-11**, run once with this file as fingerprinted
//! (protocol SHA-256 704700a9...). P1, friction: on the 19 McKeon et al.
//! (2004) friction factors, 7.4e4 to 3.57e7, rms error 1.38 percent and worst
//! 2.69 percent against lines of 3 and 5, and below the Prandtl-Karman law's
//! 2.17 percent: met. P3, profile: on 827 points of 21 held-back profiles
//! (McKeon above 1e6 and the nine NSTAP profiles of Hultmark et al. 2013),
//! pooled rms of u+ 1.59 percent and worst profile 2.58 percent against 2 and
//! 3, and below the log law's 3.59 percent: met. P2, centreline: on the same
//! 21 profiles the rms is 1.33 percent, below the baseline's 2.10, but one
//! case is 2.99 percent low against a line of 2 percent (NSTAP case 1, Re
//! 8.1e4; the other 20 are within 1.79 percent): missed. By the rule fixed
//! beforehand the comparison failed, on that one point. The friction factor
//! falls below the measured one as the Reynolds number rises: -0.7 percent at
//! 1e6, -1.4 at 1e7, -2.7 at 3.57e7. Not gated: the Zagarola-Smits friction
//! factors above 1e6 rms 2.75 percent, worst 4.52 (Prandtl-Karman 3.62, 5.45).
//!
//! **What is served.** Every fully turbulent bulk Reynolds number from 4000
//! to [`REYNOLDS_MAX`], 1e10, where the first grid node is still inside the
//! viscous sublayer (y+ 0.78) and doubling the grid moves the friction factor
//! by 2.6e-5: the equations depend on the Reynolds number alone. Pressure and
//! temperature do not enter the solve. They give the bulk velocity and its
//! Mach number, for ideal-gas air with Sutherland's viscosity, and a bulk
//! Mach number above [`MACH_MAX`], where a constant density is no longer
//! the flow, is refused by name. Through engine 0.8.0 every question outside
//! the compared Reynolds numbers, pressures, temperatures and bulk
//! velocities was refused by name.
//!
//! **What was compared where.** The held-back comparison starts at Reynolds
//! number 74,345; from 31,577 to there, at the lowest pressure (100,090 Pa)
//! and at the highest temperature (300.13 K) the only comparison is with the
//! opened set that chose the outer shape (friction rms 1.31 percent, where
//! Prandtl-Karman's 0.77 was better). Profile stations are served from 50
//! viscous units out up to Reynolds number 5,975,000, the highest NSTAP
//! profile; above it only Pitot profiles were compared, whose nearest point
//! is at 0.010591 of the radius (y+ 1054 at 6.11e6, 5444 at 3.57e7), and no
//! station nearer the wall is served.
//!
//! The protocol, the data and the comparison are in
//! `evidence/research/fluid_princeton_superpipe/`. The reference of the tests
//! is the opened Zagarola-Smits set, <https://smits.princeton.edu/zagarola/>,
//! Retrieved 2026-10-11.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::fmt;

/// Bore of the Superpipe's test section, metres (Zagarola and Smits 1998; the
/// data files state 1.2936E-01 m).
pub const DIAMETER_M: f64 = 0.12936;
/// Bore radius, metres.
pub const RADIUS_M: f64 = DIAMETER_M / 2.0;

/// Von Karman's constant of the mixing length, from the literature, not fitted.
pub const KAPPA: f64 = 0.41;
/// Van Driest's damping constant `A+`, from the literature, not fitted.
pub const VAN_DRIEST_A_PLUS: f64 = 26.0;
/// Nikuradse's mixing-length distribution for a pipe, as `l / (kappa R)`:
/// `0.35 - 0.20 (1 - y/R)^2 - 0.15 (1 - y/R)^4`. These are Nikuradse's
/// 0.14, 0.08 and 0.06 divided by his wall slope 0.40, so that the slope at
/// the wall is `KAPPA`.
pub const NIKURADSE_SHAPE: [f64; 3] = [0.35, 0.20, 0.15];

/// Below this bulk Reynolds number pipe flow is laminar.
pub const LAMINAR_BELOW_REYNOLDS: f64 = 2300.0;
/// From [`LAMINAR_BELOW_REYNOLDS`] to this bulk Reynolds number pipe flow is
/// transitional.
pub const TRANSITIONAL_BELOW_REYNOLDS: f64 = 4000.0;
/// Lowest bulk Reynolds number compared with measurement: the lowest
/// Zagarola-Smits profile. Stated, not enforced.
pub const COMPARED_REYNOLDS_MIN: f64 = 31_577.0;
/// Highest bulk Reynolds number compared with measurement: the highest
/// McKeon et al. profile. Stated, not enforced.
pub const COMPARED_REYNOLDS_MAX: f64 = 35_724_000.0;
/// Highest bulk Reynolds number served. The grid's first node sits at
/// 7.3e-9 of the radius; at 1e10 that is y+ 0.78, inside the viscous
/// sublayer, and doubling the grid moves the friction factor by 2.6e-5
/// (relative), second order. Above it the first node leaves the sublayer.
pub const REYNOLDS_MAX: f64 = 1.0e10;
/// Lowest static pressure among the compared files, pascals. Stated, not enforced.
pub const COMPARED_PRESSURE_MIN_PA: f64 = 100_090.0;
/// Highest static pressure among the compared files, pascals (Zagarola-Smits
/// at 3.5259e7). The NSTAP files give a gauge pressure; it is taken over one
/// standard atmosphere. Stated, not enforced.
pub const COMPARED_PRESSURE_MAX_PA: f64 = 18_928_000.0;
/// Lowest temperature among the compared files, kelvin: NSTAP case 3 at
/// 18.64 C. Stated, not enforced.
pub const COMPARED_TEMPERATURE_MIN_K: f64 = 291.79;
/// Highest temperature among the compared files, kelvin. Stated, not enforced.
pub const COMPARED_TEMPERATURE_MAX_K: f64 = 300.13;
/// Lowest and highest bulk velocity of the compared runs, metres per
/// second. Stated, not enforced.
pub const COMPARED_BULK_VELOCITY_M_PER_S: (f64, f64) = (3.9, 30.8);
/// Ratio of the heat capacities of air, for its speed of sound.
pub const AIR_HEAT_CAPACITY_RATIO: f64 = 1.4;
/// Specific gas constant of dry air, J/(kg K): 8.314462618 over 0.0289647 kg/mol.
pub const AIR_GAS_CONSTANT_J_PER_KG_K: f64 = 287.05;
/// Sutherland's law for the viscosity of air, as F. M. White, Viscous Fluid
/// Flow, Table 1-2, gives it: `mu0 (T/T0)^1.5 (T0 + S)/(T + S)`, with
/// `mu0` 1.716e-5 Pa s at `T0` 273 K and `S` 111 K, within 2 percent from
/// [`AIR_VISCOSITY_LAW_K`]`.0` to [`AIR_VISCOSITY_LAW_K`]`.1`.
pub const SUTHERLAND_MU0_PA_S: f64 = 1.716e-5;
pub const SUTHERLAND_T0_K: f64 = 273.0;
pub const SUTHERLAND_S_K: f64 = 111.0;
/// The temperatures over which Sutherland's law holds for air, kelvin.
pub const AIR_VISCOSITY_LAW_K: (f64, f64) = (170.0, 1900.0);
/// Highest bulk Mach number served: the solver's density is constant, and at
/// Mach 0.3 the density of air brought to rest changes by about 4.5 percent.
pub const MACH_MAX: f64 = 0.3;
/// The Superpipe's wall roughness, root mean square, micrometres (Zagarola
/// and Smits 1998: 0.15 um rms). A rougher wall is refused.
pub const WALL_ROUGHNESS_RMS_MAX_UM: f64 = 0.15;
/// Lowest wall distance in viscous units at which a profile station is
/// served: the profile line compared measured points from here out, and the
/// NSTAP profiles reach it at every Reynolds number up to
/// [`PROFILE_NSTAP_REYNOLDS_MAX`].
pub const PROFILE_Y_PLUS_MIN: f64 = 50.0;
/// Highest bulk Reynolds number of a compared NSTAP profile (case 9). Above
/// it only Pitot profiles were compared, and their nearest compared point is
/// [`PROFILE_Y_OVER_RADIUS_MIN_PITOT`].
pub const PROFILE_NSTAP_REYNOLDS_MAX: f64 = 5_975_000.0;
/// Wall distance over the radius of the nearest compared point of every
/// McKeon et al. profile above Reynolds number 1.3e6 (0.3 mm probe, kept from
/// two probe diameters): y+ 1054 at 6.11e6, 5444 at 3.57e7. Above
/// [`PROFILE_NSTAP_REYNOLDS_MAX`] no station nearer the wall is served.
pub const PROFILE_Y_OVER_RADIUS_MIN_PITOT: f64 = 0.010591;
/// The profile stations offered, as wall distance over the radius; those
/// below [`PROFILE_Y_PLUS_MIN`] at the asked Reynolds number, and above
/// [`PROFILE_NSTAP_REYNOLDS_MAX`] those below [`PROFILE_Y_OVER_RADIUS_MIN_PITOT`],
/// are left out: nothing nearer the wall was compared there.
pub const PROFILE_Y_OVER_RADIUS: [f64; 25] = [
    0.0002, 0.0003, 0.0005, 0.0007, 0.001, 0.0015, 0.002, 0.003, 0.005, 0.007, 0.01, 0.015, 0.02,
    0.03, 0.05, 0.07, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5, 0.7, 0.85, 1.0,
];

/// Grid intervals from the wall to the axis.
pub const GRID_INTERVALS: usize = 1600;
/// Stretching of the grid: wall distance over the radius at node `j` is
/// `(exp(GRID_STRETCH j/N) - 1) / (exp(GRID_STRETCH) - 1)`.
pub const GRID_STRETCH: f64 = 14.0;
/// Largest momentum imbalance of a node, over the pressure force on the
/// whole section, for an accepted flow.
pub const IMBALANCE_TOLERANCE: f64 = 1.0e-11;
/// Largest change of the friction velocity between the last two Newton
/// iterations, as a fraction, for an accepted flow.
pub const FRICTION_VELOCITY_TOLERANCE: f64 = 1.0e-12;
/// An imbalance above this stops the solver. The first iterate, the
/// one-seventh power law, starts at 1.03e6 at Reynolds number 4000.
pub const IMBALANCE_CAP: f64 = 1.0e8;
/// Newton iterations allowed.
pub const MAX_NEWTON_ITERATIONS: u32 = 60;
/// Largest departure of the volume flux from the asked bulk velocity, as a
/// fraction, after every Newton iteration.
pub const FLUX_TOLERANCE: f64 = 1.0e-12;
/// Largest departure of the shear force on the grid face nearest the wall
/// from the pressure force on the section it encloses, as a fraction of the
/// pressure force on the whole section, for an accepted flow.
pub const MOMENTUM_LEDGER_TOLERANCE: f64 = 1.0e-10;

/// The public measurement the tests compare with.
pub const REFERENCE_NAME: &str = "M. V. Zagarola and A. J. Smits, Mean-flow scaling of turbulent pipe flow, J. Fluid Mech. 373 (1998) 33-79: the twelve Superpipe data files below bulk Reynolds number 1e6, friction factor stated in each file's header";
pub const REFERENCE_URL: &str = "https://smits.princeton.edu/zagarola/";
pub const REFERENCE_RETRIEVED: &str = "2026-10-11";

/// One question.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PipeQuestion {
    /// Bulk velocity times the bore over the kinematic viscosity.
    pub bulk_reynolds_number: f64,
    /// Static pressure of the air in the pipe, pascals.
    pub static_pressure_pa: f64,
    /// Temperature of the air, kelvin.
    pub temperature_k: f64,
    /// Root-mean-square roughness of the wall, micrometres.
    pub wall_roughness_rms_um: f64,
}

/// A settled flow.
#[derive(Clone, Debug, PartialEq)]
pub struct PipeFlow {
    /// Darcy friction factor: pressure drop per length times the bore over
    /// the bulk dynamic pressure.
    pub darcy_friction_factor: f64,
    /// Velocity on the axis over the bulk velocity.
    pub centreline_to_bulk_velocity_ratio: f64,
    /// Friction velocity over the bulk velocity, `sqrt(f / 8)`.
    pub friction_to_bulk_velocity_ratio: f64,
    /// Radius in viscous units, `u_tau R / nu`.
    pub karman_number: f64,
    /// Pressure drop per metre over the bulk dynamic pressure, per metre,
    /// `f / D`.
    pub pressure_drop_per_m_over_bulk_dynamic_pressure: f64,
    /// Bulk velocity over the speed of sound, for ideal-gas air with
    /// Sutherland's viscosity at the asked pressure and temperature. Zero
    /// when settled at a Reynolds number alone.
    pub bulk_mach_number: f64,
    /// Wall distance over the radius of each served profile station.
    pub profile_y_over_radius: Vec<f64>,
    /// The same distances in viscous units.
    pub profile_y_plus: Vec<f64>,
    /// Mean velocity over the friction velocity at each station.
    pub profile_u_plus: Vec<f64>,
    /// Mean velocity over the bulk velocity at each station.
    pub profile_u_over_bulk: Vec<f64>,
    /// Largest momentum imbalance of a node over the pressure force on the section.
    pub largest_imbalance: f64,
    /// Departure of the volume flux from the asked one, as a fraction.
    pub flux_residual: f64,
    /// Shear force on the grid face nearest the wall less the pressure force it
    /// encloses, over the pressure force on the section.
    pub momentum_ledger_residual: f64,
    /// Newton iterations taken.
    pub newton_iterations: u32,
    /// Wall distance over the radius of every grid node, wall to axis.
    pub field_y_over_radius: Vec<f64>,
    /// Mean velocity over the bulk velocity at every grid node.
    pub field_u_over_bulk: Vec<f64>,
}

/// Why no flow was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum PipeReject {
    /// An input was NaN or infinite.
    InputNotFinite { field: &'static str },
    /// The bulk Reynolds number is below [`LAMINAR_BELOW_REYNOLDS`]: the flow is laminar, and this solver is of turbulent flow.
    LaminarFlow { bulk_reynolds_number: f64 },
    /// The bulk Reynolds number is from [`LAMINAR_BELOW_REYNOLDS`] to below [`TRANSITIONAL_BELOW_REYNOLDS`]: the flow is transitional.
    TransitionalFlow { bulk_reynolds_number: f64 },
    /// The bulk Reynolds number is above [`REYNOLDS_MAX`], where the grid's first node leaves the viscous sublayer.
    ReynoldsAboveGrid { bulk_reynolds_number: f64 },
    /// The wall roughness is negative.
    RoughnessNegative { wall_roughness_rms_um: f64 },
    /// The wall is rougher than the Superpipe's [`WALL_ROUGHNESS_RMS_MAX_UM`]: a rough-wall flow, which this solver is not of.
    RoughWall { wall_roughness_rms_um: f64 },
    /// The static pressure is not above zero: no gas.
    PressureNotPositive { static_pressure_pa: f64 },
    /// The temperature is outside 170 to 1900 K ([`AIR_VISCOSITY_LAW_K`]), where Sutherland's law gives the viscosity of air, so the bulk velocity cannot be known.
    TemperatureOutsideViscosityLaw { temperature_k: f64 },
    /// The bulk Mach number is above [`MACH_MAX`]: the flow is compressible, and this solver's density is constant.
    MachAboveIncompressible { bulk_mach_number: f64 },
    /// A value of the field or its imbalance was NaN or infinite.
    NonFiniteValue,
    /// The linear system of a Newton iteration had no usable pivot.
    SingularSystem,
    /// The largest imbalance rose above [`IMBALANCE_CAP`].
    ImbalanceGrew { imbalance: f64 },
    /// The flow had not settled after [`MAX_NEWTON_ITERATIONS`] iterations.
    NotSettled { imbalance: f64, friction_velocity_change: f64, iterations: u32 },
    /// The volume flux departed from the asked one by more than [`FLUX_TOLERANCE`] after a Newton iteration.
    FluxBroken { residual: f64, iteration: u32 },
    /// The shear force on the grid face nearest the wall departs from the pressure force it encloses by more than [`MOMENTUM_LEDGER_TOLERANCE`].
    MomentumLedgerBroken { residual: f64 },
    /// The settled velocity does not rise from the wall to the axis.
    VelocityNotMonotone { node: usize },
}

impl fmt::Display for PipeReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotFinite { field } => write!(f, "{field} is not a finite number"),
            Self::LaminarFlow { bulk_reynolds_number } => write!(
                f,
                "bulk Reynolds number {bulk_reynolds_number} is below {LAMINAR_BELOW_REYNOLDS}: the flow is laminar, and this solver is of turbulent flow"
            ),
            Self::TransitionalFlow { bulk_reynolds_number } => write!(
                f,
                "bulk Reynolds number {bulk_reynolds_number} is from {LAMINAR_BELOW_REYNOLDS} to below {TRANSITIONAL_BELOW_REYNOLDS}: the flow is transitional, and this solver is of fully turbulent flow"
            ),
            Self::ReynoldsAboveGrid { bulk_reynolds_number } => write!(
                f,
                "bulk Reynolds number {bulk_reynolds_number} is above {REYNOLDS_MAX}, where the grid's first node leaves the viscous sublayer"
            ),
            Self::RoughnessNegative { wall_roughness_rms_um } => {
                write!(f, "wall roughness {wall_roughness_rms_um} um is negative")
            }
            Self::RoughWall { wall_roughness_rms_um } => write!(
                f,
                "wall roughness {wall_roughness_rms_um} um rms is above the Superpipe's smooth wall, {WALL_ROUGHNESS_RMS_MAX_UM} um: a rough-wall flow, which this solver is not of"
            ),
            Self::PressureNotPositive { static_pressure_pa } => {
                write!(f, "static pressure {static_pressure_pa} Pa is not above zero")
            }
            Self::TemperatureOutsideViscosityLaw { temperature_k } => write!(
                f,
                "temperature {temperature_k} K is outside {} to {} K, where Sutherland's law gives the viscosity of air, so the bulk velocity cannot be known",
                AIR_VISCOSITY_LAW_K.0, AIR_VISCOSITY_LAW_K.1
            ),
            Self::MachAboveIncompressible { bulk_mach_number } => write!(
                f,
                "bulk Mach number {bulk_mach_number} is above {MACH_MAX}: the flow is compressible, and this solver's density is constant"
            ),
            Self::NonFiniteValue => write!(f, "a value of the field was not finite"),
            Self::SingularSystem => write!(f, "a Newton iteration had no usable pivot"),
            Self::ImbalanceGrew { imbalance } => {
                write!(f, "the largest imbalance rose to {imbalance}, above {IMBALANCE_CAP}")
            }
            Self::NotSettled {
                imbalance,
                friction_velocity_change,
                iterations,
            } => write!(
                f,
                "after {iterations} iterations the largest imbalance was {imbalance} (tolerance {IMBALANCE_TOLERANCE}) and the friction velocity still moved by {friction_velocity_change} (tolerance {FRICTION_VELOCITY_TOLERANCE})"
            ),
            Self::FluxBroken { residual, iteration } => write!(
                f,
                "after Newton iteration {iteration} the volume flux departed from the asked one by {residual}, above {FLUX_TOLERANCE}"
            ),
            Self::MomentumLedgerBroken { residual } => write!(
                f,
                "the shear force on the grid face nearest the wall departs from the pressure force it encloses by {residual} of the section's, above {MOMENTUM_LEDGER_TOLERANCE}"
            ),
            Self::VelocityNotMonotone { node } => write!(
                f,
                "the settled velocity falls toward the axis at grid node {node}"
            ),
        }
    }
}

impl std::error::Error for PipeReject {}

/// The numerical choices. The served solver uses [`SERVED`]; the tests break
/// one at a time.
#[derive(Clone, Copy, Debug)]
struct Numerics {
    intervals: usize,
    stretch: f64,
    max_newton_iterations: u32,
    imbalance_tolerance: f64,
    imbalance_cap: f64,
    flux_tolerance: f64,
    ledger_tolerance: f64,
    /// For the tests: added to the velocity of one node after each Newton
    /// update, to break the flux on purpose.
    flux_kick: f64,
}

const SERVED: Numerics = Numerics {
    intervals: GRID_INTERVALS,
    stretch: GRID_STRETCH,
    max_newton_iterations: MAX_NEWTON_ITERATIONS,
    imbalance_tolerance: IMBALANCE_TOLERANCE,
    imbalance_cap: IMBALANCE_CAP,
    flux_tolerance: FLUX_TOLERANCE,
    ledger_tolerance: MOMENTUM_LEDGER_TOLERANCE,
    flux_kick: 0.0,
};

/// The limits the question itself shows, in the order the solver asks them.
/// No flow is settled.
pub fn admit(question: &PipeQuestion) -> Result<(), PipeReject> {
    let fields = [
        ("bulk_reynolds_number", question.bulk_reynolds_number),
        ("static_pressure_pa", question.static_pressure_pa),
        ("temperature_k", question.temperature_k),
        ("wall_roughness_rms_um", question.wall_roughness_rms_um),
    ];
    for (field, value) in fields {
        if !value.is_finite() {
            return Err(PipeReject::InputNotFinite { field });
        }
    }
    let re = question.bulk_reynolds_number;
    if re < LAMINAR_BELOW_REYNOLDS {
        return Err(PipeReject::LaminarFlow { bulk_reynolds_number: re });
    }
    if re < TRANSITIONAL_BELOW_REYNOLDS {
        return Err(PipeReject::TransitionalFlow { bulk_reynolds_number: re });
    }
    if re > REYNOLDS_MAX {
        return Err(PipeReject::ReynoldsAboveGrid { bulk_reynolds_number: re });
    }
    let roughness = question.wall_roughness_rms_um;
    if roughness < 0.0 {
        return Err(PipeReject::RoughnessNegative { wall_roughness_rms_um: roughness });
    }
    if roughness > WALL_ROUGHNESS_RMS_MAX_UM {
        return Err(PipeReject::RoughWall { wall_roughness_rms_um: roughness });
    }
    let p = question.static_pressure_pa;
    if !(p > 0.0) {
        return Err(PipeReject::PressureNotPositive { static_pressure_pa: p });
    }
    let t = question.temperature_k;
    if !(t >= AIR_VISCOSITY_LAW_K.0 && t <= AIR_VISCOSITY_LAW_K.1) {
        return Err(PipeReject::TemperatureOutsideViscosityLaw { temperature_k: t });
    }
    let mach = bulk_mach_number(re, p, t);
    if !mach.is_finite() {
        return Err(PipeReject::NonFiniteValue);
    }
    if mach > MACH_MAX {
        return Err(PipeReject::MachAboveIncompressible { bulk_mach_number: mach });
    }
    Ok(())
}

/// Viscosity of air at a temperature, pascal seconds: Sutherland's law.
pub fn air_viscosity_pa_s(temperature_k: f64) -> f64 {
    SUTHERLAND_MU0_PA_S
        * math::powf(temperature_k / SUTHERLAND_T0_K, 1.5)
        * (SUTHERLAND_T0_K + SUTHERLAND_S_K)
        / (temperature_k + SUTHERLAND_S_K)
}

/// Bulk velocity of air in the pipe at a bulk Reynolds number, pressure and
/// temperature, metres per second: `Re mu / (rho D)`, `rho = p / (R T)`.
pub fn bulk_velocity_m_per_s(bulk_reynolds_number: f64, static_pressure_pa: f64, temperature_k: f64) -> f64 {
    let density = static_pressure_pa / (AIR_GAS_CONSTANT_J_PER_KG_K * temperature_k);
    bulk_reynolds_number * air_viscosity_pa_s(temperature_k) / (density * DIAMETER_M)
}

/// Bulk velocity over the speed of sound of ideal-gas air, `sqrt(gamma R T)`.
pub fn bulk_mach_number(bulk_reynolds_number: f64, static_pressure_pa: f64, temperature_k: f64) -> f64 {
    let sound = (AIR_HEAT_CAPACITY_RATIO * AIR_GAS_CONSTANT_J_PER_KG_K * temperature_k).sqrt();
    bulk_velocity_m_per_s(bulk_reynolds_number, static_pressure_pa, temperature_k) / sound
}

/// Settle the flow for one question, or name the limit.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &PipeQuestion) -> Result<PipeFlow, PipeReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &PipeQuestion) -> Result<PipeFlow, PipeReject> {
    admit(question)?;
    let mut flow = settle(question.bulk_reynolds_number, &SERVED)?;
    flow.bulk_mach_number = bulk_mach_number(
        question.bulk_reynolds_number,
        question.static_pressure_pa,
        question.temperature_k,
    );
    Ok(flow)
}

impl NormalOrZero for PipeFlow {
    fn normal_or_zero(&mut self) {
        self.darcy_friction_factor.normal_or_zero();
        self.centreline_to_bulk_velocity_ratio.normal_or_zero();
        self.friction_to_bulk_velocity_ratio.normal_or_zero();
        self.karman_number.normal_or_zero();
        self.pressure_drop_per_m_over_bulk_dynamic_pressure.normal_or_zero();
        self.bulk_mach_number.normal_or_zero();
        self.profile_y_over_radius.normal_or_zero();
        self.profile_y_plus.normal_or_zero();
        self.profile_u_plus.normal_or_zero();
        self.profile_u_over_bulk.normal_or_zero();
        self.largest_imbalance.normal_or_zero();
        self.flux_residual.normal_or_zero();
        self.momentum_ledger_residual.normal_or_zero();
        self.field_y_over_radius.normal_or_zero();
        self.field_u_over_bulk.normal_or_zero();
    }
}

/// Settle the flow at a bulk Reynolds number alone, with no pressure or
/// temperature and so no Mach number: for the comparison scripts and the
/// tests, which ask at the measured conditions. The gateway calls [`solve`].
pub fn settle_at_reynolds_number(bulk_reynolds_number: f64) -> Result<PipeFlow, PipeReject> {
    turbulent(bulk_reynolds_number)?;
    settle(bulk_reynolds_number, &SERVED)
}

/// The Reynolds-number refusals that do not depend on the compared range:
/// not a number, laminar, transitional.
fn turbulent(re: f64) -> Result<(), PipeReject> {
    if !re.is_finite() {
        return Err(PipeReject::InputNotFinite { field: "bulk_reynolds_number" });
    }
    if re < LAMINAR_BELOW_REYNOLDS {
        return Err(PipeReject::LaminarFlow { bulk_reynolds_number: re });
    }
    if re < TRANSITIONAL_BELOW_REYNOLDS {
        return Err(PipeReject::TransitionalFlow { bulk_reynolds_number: re });
    }
    Ok(())
}

/// The grid: wall distance over the radius of each node, wall (0) to axis (N).
fn grid(numerics: &Numerics) -> Vec<f64> {
    let n = numerics.intervals;
    let denominator = math::exp_m1(numerics.stretch);
    let mut y: Vec<f64> = (0..=n)
        .map(|j| math::exp_m1(numerics.stretch * j as f64 / n as f64) / denominator)
        .collect();
    y[0] = 0.0;
    y[n] = 1.0;
    y
}

/// Mixing length over the radius at wall distance `eta = y/R`, without damping.
fn mixing_shape(eta: f64) -> f64 {
    let s = 1.0 - eta;
    let s2 = s * s;
    KAPPA * (NIKURADSE_SHAPE[0] - NIKURADSE_SHAPE[1] * s2 - NIKURADSE_SHAPE[2] * s2 * s2)
}

/// Solve a tridiagonal system in place (Thomas, no pivoting needed for a
/// diagonally dominant matrix; a vanishing pivot is reported).
fn tridiagonal(lower: &[f64], diag: &[f64], upper: &[f64], rhs: &[f64]) -> Option<Vec<f64>> {
    let n = diag.len();
    let mut c = vec![0.0; n];
    let mut d = vec![0.0; n];
    let mut pivot = diag[0];
    if !(pivot.abs() > 1e-300) || !pivot.is_finite() {
        return None;
    }
    c[0] = upper[0] / pivot;
    d[0] = rhs[0] / pivot;
    for i in 1..n {
        pivot = diag[i] - lower[i] * c[i - 1];
        if !(pivot.abs() > 1e-300) || !pivot.is_finite() {
            return None;
        }
        c[i] = if i + 1 < n { upper[i] / pivot } else { 0.0 };
        d[i] = (rhs[i] - lower[i] * d[i - 1]) / pivot;
    }
    let mut x = vec![0.0; n];
    x[n - 1] = d[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = d[i] - c[i] * x[i + 1];
    }
    Some(x)
}

/// The settled field, in units of the radius and the bulk velocity.
struct Field {
    y: Vec<f64>,
    u: Vec<f64>,
    /// Pressure drop per length times the radius over `rho U_b^2`.
    g: f64,
    largest_imbalance: f64,
    flux_residual: f64,
    ledger: f64,
    iterations: u32,
}

/// Newton's method on the finite-volume momentum balance of every node and
/// the volume flux, with the pressure gradient as the extra unknown. The
/// Jacobian is tridiagonal with one bordering row and column, solved directly
/// by two tridiagonal solves. The van Driest damping takes the friction
/// velocity of the previous iterate; the flow is accepted when the imbalance
/// and the change of the friction velocity are both below tolerance.
fn settle_field(bulk_reynolds_number: f64, numerics: &Numerics) -> Result<Field, PipeReject> {
    let n = numerics.intervals;
    let re_r = bulk_reynolds_number / 2.0;
    let y = grid(numerics);
    // Faces j + 1/2 for j = 0..n-1.
    let dy: Vec<f64> = (0..n).map(|j| y[j + 1] - y[j]).collect();
    let yf: Vec<f64> = (0..n).map(|j| 0.5 * (y[j + 1] + y[j])).collect();
    let rf: Vec<f64> = yf.iter().map(|v| 1.0 - v).collect();
    let shape: Vec<f64> = yf.iter().map(|&v| mixing_shape(v)).collect();
    // Control volume of node j: the integral of r dr between its faces.
    let mut vol = vec![0.0; n + 1];
    for j in 0..=n {
        let lo = if j == 0 { 0.0 } else { yf[j - 1] };
        let hi = if j == n { 1.0 } else { yf[j] };
        let (r_lo, r_hi) = (1.0 - lo, 1.0 - hi);
        vol[j] = 0.5 * (r_lo * r_lo - r_hi * r_hi);
    }
    // Start from the one-seventh power law scaled to the asked flux.
    let mut u: Vec<f64> = y.iter().map(|&v| math::powf(v, 1.0 / 7.0)).collect();
    u[0] = 0.0;
    let flux: f64 = (1..=n).map(|j| vol[j] * u[j]).sum();
    for v in u.iter_mut() {
        *v *= 0.5 / flux;
    }
    let mut g: f64 = 0.01;
    let mut last_friction_velocity = f64::NAN;
    let mut iterations = 0_u32;
    let m = n; // unknowns u[1..=n]
    let mut grad = vec![0.0; n];
    let mut face_flux = vec![0.0; n];
    let mut dflux = vec![0.0; n];
    loop {
        // Gradients and the friction velocity of this iterate.
        for j in 0..n {
            grad[j] = (u[j + 1] - u[j]) / dy[j];
        }
        let wall_shear = grad[0] / re_r;
        if !wall_shear.is_finite() || wall_shear <= 0.0 {
            return Err(PipeReject::NonFiniteValue);
        }
        let friction_velocity = wall_shear.sqrt();
        for j in 0..n {
            let y_plus = yf[j] * re_r * friction_velocity;
            let l = shape[j] * (1.0 - math::exp(-y_plus / VAN_DRIEST_A_PLUS));
            let l2 = l * l;
            face_flux[j] = rf[j] * (1.0 / re_r + l2 * grad[j].abs()) * grad[j];
            dflux[j] = rf[j] * (1.0 / re_r + 2.0 * l2 * grad[j].abs());
        }
        // Residuals of nodes 1..=n: F(j+1/2) - F(j-1/2) + G V_j = 0; the axis
        // node has no outer face (symmetry).
        let scale = 0.5 * g.abs().max(1e-300);
        let mut residual = vec![0.0; m];
        let mut largest = 0.0_f64;
        for j in 1..=n {
            let outer = if j < n { face_flux[j] } else { 0.0 };
            let value = outer - face_flux[j - 1] + g * vol[j];
            residual[j - 1] = value;
            largest = largest.max(value.abs() / scale);
        }
        if !largest.is_finite() {
            return Err(PipeReject::NonFiniteValue);
        }
        if largest > numerics.imbalance_cap {
            return Err(PipeReject::ImbalanceGrew { imbalance: largest });
        }
        let change = ((friction_velocity - last_friction_velocity) / friction_velocity).abs();
        if largest <= numerics.imbalance_tolerance && change <= FRICTION_VELOCITY_TOLERANCE {
            let flux: f64 = (1..=n).map(|j| vol[j] * u[j]).sum();
            let flux_residual = (flux - 0.5) / 0.5;
            // The ledger: shear force on the grid face nearest the wall (per unit length
            // over 2 pi) against the pressure force on the section inside it.
            let enclosed: f64 = (1..=n).map(|j| vol[j]).sum();
            let ledger = (face_flux[0] - g * enclosed) / (0.5 * g);
            if !(ledger.abs() <= numerics.ledger_tolerance) {
                return Err(PipeReject::MomentumLedgerBroken { residual: ledger });
            }
            for j in 0..n {
                if !(grad[j] > 0.0) {
                    return Err(PipeReject::VelocityNotMonotone { node: j + 1 });
                }
            }
            return Ok(Field {
                y,
                u,
                g,
                largest_imbalance: largest,
                flux_residual,
                ledger,
                iterations,
            });
        }
        if iterations >= numerics.max_newton_iterations {
            return Err(PipeReject::NotSettled {
                imbalance: largest,
                friction_velocity_change: change,
                iterations,
            });
        }
        last_friction_velocity = friction_velocity;
        // Jacobian rows for nodes 1..=n (index k = j - 1).
        let mut lower = vec![0.0; m];
        let mut diag = vec![0.0; m];
        let mut upper = vec![0.0; m];
        for j in 1..=n {
            let k = j - 1;
            let inner = dflux[j - 1] / dy[j - 1];
            if j < n {
                let outer = dflux[j] / dy[j];
                diag[k] = -outer - inner;
                upper[k] = outer;
            } else {
                diag[k] = -inner;
            }
            lower[k] = if j > 1 { inner } else { 0.0 };
        }
        let minus_r: Vec<f64> = residual.iter().map(|v| -v).collect();
        let border: Vec<f64> = (1..=n).map(|j| vol[j]).collect();
        let x1 = tridiagonal(&lower, &diag, &upper, &minus_r).ok_or(PipeReject::SingularSystem)?;
        let x2 = tridiagonal(&lower, &diag, &upper, &border).ok_or(PipeReject::SingularSystem)?;
        let flux: f64 = (1..=n).map(|j| vol[j] * u[j]).sum();
        let constraint = flux - 0.5;
        let vx1: f64 = border.iter().zip(&x1).map(|(a, b)| a * b).sum();
        let vx2: f64 = border.iter().zip(&x2).map(|(a, b)| a * b).sum();
        if !(vx2.abs() > 1e-300) {
            return Err(PipeReject::SingularSystem);
        }
        let dg = (vx1 + constraint) / vx2;
        for j in 1..=n {
            u[j] += x1[j - 1] - dg * x2[j - 1];
        }
        g += dg;
        if numerics.flux_kick != 0.0 {
            u[n / 2] += numerics.flux_kick;
        }
        iterations += 1;
        if !g.is_finite() || u.iter().any(|v| !v.is_finite()) {
            return Err(PipeReject::NonFiniteValue);
        }
        // The flux is a linear constraint of the Newton system, so it holds
        // after every update to rounding; it is checked every iteration.
        let flux: f64 = (1..=n).map(|j| vol[j] * u[j]).sum();
        let flux_residual = ((flux - 0.5) / 0.5).abs();
        if !(flux_residual <= numerics.flux_tolerance) {
            return Err(PipeReject::FluxBroken {
                residual: flux_residual,
                iteration: iterations,
            });
        }
    }
}

fn settle(bulk_reynolds_number: f64, numerics: &Numerics) -> Result<PipeFlow, PipeReject> {
    let field = settle_field(bulk_reynolds_number, numerics)?;
    Ok(flow_of(bulk_reynolds_number, numerics, &field))
}

fn flow_of(bulk_reynolds_number: f64, numerics: &Numerics, field: &Field) -> PipeFlow {
    let n = numerics.intervals;
    let re_r = bulk_reynolds_number / 2.0;
    // f = (-dp/dx) D / (rho U_b^2 / 2) = 4 G with G in radius and bulk units.
    let darcy_friction_factor = 4.0 * field.g;
    let friction_to_bulk = (darcy_friction_factor / 8.0).sqrt();
    let karman_number = re_r * friction_to_bulk;
    let mut profile_y_over_radius = Vec::new();
    let mut profile_y_plus = Vec::new();
    let mut profile_u_plus = Vec::new();
    let mut profile_u_over_bulk = Vec::new();
    for &eta in PROFILE_Y_OVER_RADIUS.iter() {
        let y_plus = eta * karman_number;
        if y_plus < PROFILE_Y_PLUS_MIN {
            continue;
        }
        if bulk_reynolds_number > PROFILE_NSTAP_REYNOLDS_MAX && eta < PROFILE_Y_OVER_RADIUS_MIN_PITOT {
            continue;
        }
        let u = interpolate(&field.y, &field.u, eta);
        profile_y_over_radius.push(eta);
        profile_y_plus.push(y_plus);
        profile_u_plus.push(u / friction_to_bulk);
        profile_u_over_bulk.push(u);
    }
    PipeFlow {
        darcy_friction_factor,
        centreline_to_bulk_velocity_ratio: field.u[n],
        friction_to_bulk_velocity_ratio: friction_to_bulk,
        karman_number,
        pressure_drop_per_m_over_bulk_dynamic_pressure: darcy_friction_factor / DIAMETER_M,
        bulk_mach_number: 0.0,
        profile_y_over_radius,
        profile_y_plus,
        profile_u_plus,
        profile_u_over_bulk,
        largest_imbalance: field.largest_imbalance,
        flux_residual: field.flux_residual,
        momentum_ledger_residual: field.ledger,
        newton_iterations: field.iterations,
        field_y_over_radius: field.y.clone(),
        field_u_over_bulk: field.u.clone(),
    }
}

/// Mean velocity over the bulk velocity at wall distance `eta` over the
/// radius, linear between grid nodes. For the comparison scripts.
pub fn velocity_over_bulk_at(bulk_reynolds_number: f64, eta: &[f64]) -> Result<(PipeFlow, Vec<f64>), PipeReject> {
    turbulent(bulk_reynolds_number)?;
    let field = settle_field(bulk_reynolds_number, &SERVED)?;
    let values = eta.iter().map(|&e| interpolate(&field.y, &field.u, e)).collect();
    Ok((flow_of(bulk_reynolds_number, &SERVED, &field), values))
}

fn interpolate(y: &[f64], u: &[f64], eta: f64) -> f64 {
    if eta <= 0.0 {
        return u[0];
    }
    if eta >= 1.0 {
        return u[u.len() - 1];
    }
    // Rising grid: find the interval by bisection.
    let (mut lo, mut hi) = (0usize, y.len() - 1);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if y[mid] <= eta {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t = (eta - y[lo]) / (y[hi] - y[lo]);
    u[lo] + t * (u[hi] - u[lo])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A question at one atmosphere and room temperature on the Superpipe's
    /// own wall, inside every bound that does not depend on the Reynolds number.
    fn ask(re: f64, p: f64) -> PipeQuestion {
        PipeQuestion {
            bulk_reynolds_number: re,
            static_pressure_pa: p,
            temperature_k: 296.0,
            wall_roughness_rms_um: 0.15,
        }
    }

    /// The twelve opened Zagarola-Smits files: bulk Reynolds number and the
    /// friction factor stated in each header. Retrieved 2026-10-11 from the
    /// files linked at REFERENCE_URL.
    const ZAGAROLA_SMITS_1998_OPENED: [(f64, f64); 12] = [
        (31_577.0, 2.3239e-2),
        (41_727.0, 2.1858e-2),
        (56_677.0, 2.0379e-2),
        (74_293.0, 1.9120e-2),
        (98_811.0, 1.8018e-2),
        (145_790.0, 1.6613e-2),
        (185_430.0, 1.5835e-2),
        (230_460.0, 1.5185e-2),
        (309_500.0, 1.4509e-2),
        (409_290.0, 1.3758e-2),
        (539_090.0, 1.3176e-2),
        (751_820.0, 1.2455e-2),
    ];

    /// `analysis/model.py` on the served grid.
    const PYTHON_MODEL: [(f64, f64, f64); 3] = [
        (31_577.0, 0.023763975234121628, 1.223106674252446),
        (752_000.0, 0.012348568741189938, 1.156239160005525),
        (35_724_000.0, 0.006854894341658778, 1.1162342809606838),
    ];

    #[test]
    fn agrees_with_the_plain_python_model() {
        for (re, f_py, ucl_py) in PYTHON_MODEL {
            let flow = settle_at_reynolds_number(re).expect("settles");
            assert!((flow.darcy_friction_factor / f_py - 1.0).abs() < 1e-12, "{re} {}", flow.darcy_friction_factor);
            assert!((flow.centreline_to_bulk_velocity_ratio / ucl_py - 1.0).abs() < 1e-12, "{re}");
        }
    }

    /// The published reference: every opened Zagarola-Smits friction factor
    /// within 3 percent (found: rms 1.31, worst 2.26 percent, at 3.16e4).
    #[test]
    fn zagarola_smits_1998_friction_factors() {
        let mut worst = 0.0_f64;
        for (re, measured) in ZAGAROLA_SMITS_1998_OPENED {
            let flow = settle_at_reynolds_number(re).expect("settles");
            let error = flow.darcy_friction_factor / measured - 1.0;
            assert!(error.abs() <= 0.03, "{re} {error}");
            worst = worst.max(error.abs());
        }
        assert!((worst - 0.0226).abs() < 0.0005, "{worst}");
    }

    /// The held-back result as it was scored, at three of its cases: McKeon
    /// et al. 2004 at 1.0309e6 and 3.5724e7, and the centreline ratio of
    /// NSTAP case 1, the one miss.
    #[test]
    fn the_scored_comparison_at_three_cases() {
        let low = settle_at_reynolds_number(1_030_900.0).expect("settles");
        assert!((low.darcy_friction_factor / 0.011777 - 1.0 + 0.0073).abs() < 0.0005);
        let high = settle_at_reynolds_number(35_724_000.0).expect("settles");
        assert!((high.darcy_friction_factor / 0.0070443 - 1.0 + 0.0269).abs() < 0.0005);
        let nstap = settle_at_reynolds_number(81_330.0).expect("settles");
        let miss = nstap.centreline_to_bulk_velocity_ratio / (11.69 / 9.48) - 1.0;
        assert!(miss < -0.02 && (miss + 0.0299).abs() < 0.0005, "{miss}");
    }

    /// The grid was fixed against itself at the two ends of the compared
    /// range, and is held so at the two ends of the served one: 4000, the
    /// start of fully turbulent flow, and 1e10, the top.
    #[test]
    fn the_grid_is_converged() {
        for re in [TRANSITIONAL_BELOW_REYNOLDS, COMPARED_REYNOLDS_MIN, COMPARED_REYNOLDS_MAX, REYNOLDS_MAX] {
            let served = settle_field(re, &SERVED).expect("settles");
            let fine = settle_field(re, &Numerics { intervals: 2 * GRID_INTERVALS, ..SERVED }).expect("settles");
            let coarse = settle_field(re, &Numerics { intervals: GRID_INTERVALS / 2, ..SERVED }).expect("settles");
            let near = (served.g / fine.g - 1.0).abs();
            let far = (coarse.g / served.g - 1.0).abs();
            assert!(near < 5e-5, "{re} {near}");
            // Second order: halving the spacing cuts the change by about four.
            assert!(far / near > 3.5 && far / near < 4.5, "{re} {}", far / near);
        }
    }

    #[test]
    fn the_ledgers_close_and_the_profile_rises() {
        for re in [TRANSITIONAL_BELOW_REYNOLDS, 1.0e4, COMPARED_REYNOLDS_MIN, 1.0e6, COMPARED_REYNOLDS_MAX, 1.0e9, REYNOLDS_MAX] {
            let flow = settle_at_reynolds_number(re).expect("settles");
            assert!(flow.largest_imbalance <= IMBALANCE_TOLERANCE);
            assert!(flow.flux_residual.abs() <= FLUX_TOLERANCE);
            assert!(flow.momentum_ledger_residual.abs() <= MOMENTUM_LEDGER_TOLERANCE);
            assert!(!flow.profile_y_plus.is_empty());
            assert!(flow.profile_y_plus.iter().all(|y| *y >= PROFILE_Y_PLUS_MIN));
            assert!(flow.profile_u_plus.windows(2).all(|w| w[1] > w[0]));
            let last = flow.profile_u_over_bulk.last().copied().expect("the axis");
            assert_eq!(last, flow.centreline_to_bulk_velocity_ratio);
            assert_eq!(*flow.profile_y_over_radius.last().unwrap(), 1.0);
            let ratio = flow.friction_to_bulk_velocity_ratio;
            assert!((ratio * ratio * 8.0 / flow.darcy_friction_factor - 1.0).abs() < 1e-14);
        }
    }

    #[test]
    fn the_friction_factor_falls_and_the_profile_flattens_with_reynolds_number() {
        let mut last = (f64::INFINITY, f64::INFINITY);
        for re in [TRANSITIONAL_BELOW_REYNOLDS, 1.0e4, COMPARED_REYNOLDS_MIN, 1.0e5, 1.0e6, 1.0e7, COMPARED_REYNOLDS_MAX, 1.0e9, REYNOLDS_MAX] {
            let flow = settle_at_reynolds_number(re).expect("settles");
            assert!(flow.darcy_friction_factor < last.0);
            assert!(flow.centreline_to_bulk_velocity_ratio < last.1);
            last = (flow.darcy_friction_factor, flow.centreline_to_bulk_velocity_ratio);
        }
    }

    #[test]
    fn same_question_same_bits() {
        let q = ask(1.0e5, 101_325.0);
        let a = solve(&q).expect("settles");
        let b = solve(&q).expect("settles");
        assert_eq!(a.darcy_friction_factor.to_bits(), b.darcy_friction_factor.to_bits());
        assert_eq!(a, b);
    }

    #[test]
    fn both_ends_of_every_range_are_inside_and_the_next_number_is_not() {
        let base = PipeQuestion {
            bulk_reynolds_number: 1.0e6,
            static_pressure_pa: 2.0e6,
            temperature_k: 296.0,
            wall_roughness_rms_um: 0.0,
        };
        assert!(admit(&base).is_ok());
        let at_p = |re: f64, p: f64| PipeQuestion { bulk_reynolds_number: re, static_pressure_pa: p, ..base };
        // The Reynolds ends: fully turbulent from 4000, and the grid's top.
        assert!(admit(&at_p(TRANSITIONAL_BELOW_REYNOLDS, 101_325.0)).is_ok());
        assert!(admit(&at_p(REYNOLDS_MAX, 2.0e9)).is_ok());
        assert_eq!(
            admit(&at_p(REYNOLDS_MAX.next_up(), 2.0e9)),
            Err(PipeReject::ReynoldsAboveGrid { bulk_reynolds_number: REYNOLDS_MAX.next_up() })
        );
        // Laminar and transitional flow by name.
        assert!(matches!(admit(&at_p(2299.0, 100_300.0)), Err(PipeReject::LaminarFlow { .. })));
        assert!(matches!(admit(&at_p(2300.0, 100_300.0)), Err(PipeReject::TransitionalFlow { .. })));
        assert!(matches!(admit(&at_p(3999.0, 100_300.0)), Err(PipeReject::TransitionalFlow { .. })));
        // Pressure: anything above zero; the Mach number holds the rest.
        assert!(admit(&at_p(4000.0, 1000.0)).is_ok());
        for p in [0.0, -0.0, -1.0, -1.0e9] {
            assert_eq!(admit(&at_p(1.0e5, p)), Err(PipeReject::PressureNotPositive { static_pressure_pa: p }));
        }
        // Temperature: where Sutherland's law holds.
        let at_t = |t: f64| PipeQuestion { temperature_k: t, ..base };
        for t in [AIR_VISCOSITY_LAW_K.0, 291.78999999999996, COMPARED_TEMPERATURE_MIN_K, AIR_VISCOSITY_LAW_K.1] {
            assert!(admit(&at_t(t)).is_ok(), "{t}");
        }
        for t in [AIR_VISCOSITY_LAW_K.0.next_down(), AIR_VISCOSITY_LAW_K.1.next_up(), 0.0, -10.0] {
            assert_eq!(admit(&at_t(t)), Err(PipeReject::TemperatureOutsideViscosityLaw { temperature_k: t }));
        }
        // The Mach number: the same Reynolds number at a lower pressure is a
        // faster flow.
        assert!(admit(&at_p(35_724_000.0, 1.0e6)).is_err());
        match admit(&at_p(35_724_000.0, 1.0e6)) {
            Err(PipeReject::MachAboveIncompressible { bulk_mach_number }) => assert!(bulk_mach_number > MACH_MAX),
            other => panic!("{other:?}"),
        }
        // Roughness: the Superpipe's wall and smoother are inside, rougher is a rough wall.
        let at_k = |k: f64| PipeQuestion { wall_roughness_rms_um: k, ..base };
        assert!(admit(&at_k(WALL_ROUGHNESS_RMS_MAX_UM)).is_ok());
        assert!(matches!(admit(&at_k(WALL_ROUGHNESS_RMS_MAX_UM.next_up())), Err(PipeReject::RoughWall { .. })));
        assert!(matches!(admit(&at_k(2.5)), Err(PipeReject::RoughWall { .. })));
        assert!(matches!(admit(&at_k(-1e-9)), Err(PipeReject::RoughnessNegative { .. })));
        // Not a number.
        assert_eq!(
            admit(&PipeQuestion { temperature_k: f64::NAN, ..base }),
            Err(PipeReject::InputNotFinite { field: "temperature_k" })
        );
        // The sentences say their ranges in words.
        assert_eq!(
            PipeReject::TemperatureOutsideViscosityLaw { temperature_k: 100.0 }.to_string(),
            "temperature 100 K is outside 170 to 1900 K, where Sutherland's law gives the viscosity of air, so the bulk velocity cannot be known"
        );
        assert_eq!(
            PipeReject::ReynoldsAboveGrid { bulk_reynolds_number: 2.0e10 }.to_string(),
            "bulk Reynolds number 20000000000 is above 10000000000, where the grid's first node leaves the viscous sublayer"
        );
    }

    /// The Mach number is the bulk velocity of ideal-gas air over its speed
    /// of sound. At NSTAP case 1 (Reynolds number 81,330, one atmosphere,
    /// 292.4 K) the bulk velocity is about 9.4 m/s; at the highest McKeon
    /// profile (35.7 million, 17.89 MPa, 297.25 K) about 24 m/s; the limit is
    /// a bulk Mach number of 0.3, about 104 m/s at 300 K.
    #[test]
    fn the_mach_number_is_the_bulk_velocity_over_the_speed_of_sound() {
        let low = bulk_velocity_m_per_s(81_330.0, 101_325.0, 292.4);
        assert!((low - 9.48).abs() < 0.2, "{low}");
        let high = bulk_velocity_m_per_s(35_724_000.0, 17_890_000.0, 297.25);
        // Ideal-gas air with the viscosity of air at low pressure: at 17.89 MPa
        // air is denser-than-ideal by about 3 percent and more viscous, so
        // this is below the run's own bulk velocity.
        assert!(high > 20.0 && high < COMPARED_BULK_VELOCITY_M_PER_S.1, "{high}");
        let sound = (AIR_HEAT_CAPACITY_RATIO * AIR_GAS_CONSTANT_J_PER_KG_K * 300.0).sqrt();
        assert!((sound - 347.2).abs() < 0.2, "{sound}");
        assert!((air_viscosity_pa_s(300.0) - 1.846e-5).abs() < 0.01e-5, "{}", air_viscosity_pa_s(300.0));
        // the answer carries its Mach number, and the friction is the
        // Reynolds number's alone
        let flow = solve(&ask(1.0e5, 101_325.0)).expect("settles");
        let mach = bulk_mach_number(1.0e5, 101_325.0, 296.0);
        assert_eq!(flow.bulk_mach_number, mach);
        assert_eq!(flow.darcy_friction_factor, settle_at_reynolds_number(1.0e5).unwrap().darcy_friction_factor);
        // At one atmosphere and 296 K a Mach number of 0.3 is near Reynolds
        // number 870,000: just under it is answered, just over it refused.
        let mut re = 8.0e5;
        while bulk_mach_number(re * 1.001, 101_325.0, 296.0) <= MACH_MAX {
            re *= 1.001;
        }
        assert!(re > 8.0e5 && re < 9.5e5, "{re}");
        assert!(solve(&ask(re, 101_325.0)).is_ok());
        assert!(matches!(solve(&ask(re * 1.001, 101_325.0)), Err(PipeReject::MachAboveIncompressible { .. })));
    }

    /// Past every wall engine 0.8.0 held, the flow is settled with every
    /// ledger closed and the profile rising, and gives the same friction
    /// factor at the same Reynolds number whatever the pressure and
    /// temperature: below the compared Reynolds numbers, above them, at
    /// pressures and temperatures no compared run had, and at bulk velocities
    /// outside 3.9 to 30.8 m/s.
    #[test]
    fn past_the_compared_runs_the_flow_settles_with_its_ledgers_closed() {
        for (re, p, t) in [
            (4000.0, 101_325.0, 296.0),
            (10_000.0, 2_000.0, 200.0),
            (20_000.0, 101_325.0, 1500.0),
            (1.0e7, 2.0e6, 220.0),
            (1.0e8, 5.0e7, 300.0),
            (1.0e9, 5.0e8, 300.0),
            (REYNOLDS_MAX, 2.0e9, 300.0),
        ] {
            let flow = solve(&PipeQuestion { bulk_reynolds_number: re, static_pressure_pa: p, temperature_k: t, wall_roughness_rms_um: 0.15 })
                .unwrap_or_else(|reject| panic!("{re} {p} {t}: {reject}"));
            assert!(flow.largest_imbalance <= IMBALANCE_TOLERANCE);
            assert!(flow.flux_residual.abs() <= FLUX_TOLERANCE);
            assert!(flow.momentum_ledger_residual.abs() <= MOMENTUM_LEDGER_TOLERANCE);
            assert!(flow.bulk_mach_number > 0.0 && flow.bulk_mach_number <= MACH_MAX);
            assert!(flow.profile_u_plus.windows(2).all(|w| w[1] > w[0]));
            assert_eq!(flow.darcy_friction_factor, settle_at_reynolds_number(re).unwrap().darcy_friction_factor);
            // the first node of the grid is inside the viscous sublayer
            assert!(flow.field_y_over_radius[1] * flow.karman_number < 1.0, "{re}");
        }
    }

    /// Every compared run's own condition is answered: all 60 files, (Re, p,
    /// T) as the scoring script read them, and each lies inside the stated
    /// compared envelope.
    #[test]
    fn every_compared_run_is_inside() {
        let runs: [(f64, f64, f64); 60] = [
            (410860.0, 292320.0, 296.76099999999997), // 10f.txt
            (536930.0, 288520.0, 294.748), // 11f.txt
            (753590.0, 1472900.0, 295.443), // 12f.txt
            (1030900.0, 1469600.0, 295.212), // 13f.txt
            (1346200.0, 1470100.0, 295.57899999999995), // 14f.txt
            (1795000.0, 1461800.0, 295.64599999999996), // 15f.txt
            (2362900.0, 1452200.0, 295.464), // 16f.txt
            (3105000.0, 2824900.0, 296.83099999999996), // 17f.txt
            (4462200.0, 11947000.0, 297.04999999999995), // 18f.txt
            (6112700.0, 11933000.0, 297.45), // 19f.txt
            (7806100.0, 18166000.0, 294.04999999999995), // 20f.txt
            (10314000.0, 18166000.0, 294.15), // 21f.txt
            (13719000.0, 18111000.0, 294.25), // 22f.txt
            (18308000.0, 17959000.0, 294.75), // 23f.txt
            (35724000.0, 17890000.0, 297.25), // 26f.txt
            (74345.0, 101330.0, 296.18899999999996), // 4af.txt
            (74345.0, 101330.0, 296.18899999999996), // 4bf.txt
            (74345.0, 101330.0, 296.18899999999996), // 4cf.txt
            (74345.0, 101330.0, 296.18899999999996), // 4df.txt
            (144580.0, 101330.0, 297.04999999999995), // 6af.txt
            (144580.0, 101330.0, 297.04999999999995), // 6bf.txt
            (144580.0, 101330.0, 297.04999999999995), // 6cf.txt
            (144580.0, 101330.0, 297.04999999999995), // 6df.txt
            (233970.0, 291900.0, 295.47499999999997), // 8f.txt
            (309630.0, 292250.0, 296.224), // 9f.txt
            (81330.0, 101325.0, 292.4), // Statistics_Case1
            (145600.0, 169212.75, 291.94), // Statistics_Case2
            (246600.0, 344505.0, 291.78999999999996), // Statistics_Case3
            (512500.0, 651519.75, 294.26), // Statistics_Case4
            (1055000.0, 1194621.75, 294.09999999999997), // Statistics_Case5
            (2083000.0, 2382150.75, 294.47999999999996), // Statistics_Case6
            (3951000.0, 4718705.25, 296.48999999999995), // Statistics_Case7
            (4000000.0, 4748089.5, 294.95), // Statistics_Case8
            (5975000.0, 7168743.75, 295.90999999999997), // Statistics_Case9
            (1023800.0, 1488200.0, 298.4), // 1.0238E+06.txt
            (10249000.0, 18110000.0, 295.59), // 1.0249E+07.txt
            (1340400.0, 1472000.0, 296.94), // 1.3404E+06.txt
            (13598000.0, 18200000.0, 295.46), // 1.3598E+07.txt
            (1787500.0, 1476100.0, 297.45), // 1.7875E+06.txt
            (18196000.0, 18379000.0, 296.01), // 1.8196E+07.txt
            (2345000.0, 1462200.0, 296.77), // 2.3450E+06.txt
            (23977000.0, 18588000.0, 296.7), // 2.3977E+07.txt
            (29927000.0, 18755000.0, 296.94), // 2.9927E+07.txt
            (3098100.0, 2830000.0, 294.95), // 3.0981E+06.txt
            (35259000.0, 18928000.0, 296.69), // 3.5259E+07.txt
            (4420300.0, 11788000.0, 294.7), // 4.4203E+06.txt
            (6072700.0, 11927000.0, 295.04), // 6.0727E+06.txt
            (7714700.0, 18005000.0, 295.75), // 7.7147E+06.txt
            (145790.0, 101260.0, 298.31), // 1.4579E+05.txt
            (185430.0, 100410.0, 299.31), // 1.8543E+05.txt
            (230460.0, 287390.0, 297.94), // 2.3046E+05.txt
            (309500.0, 288760.0, 298.91), // 3.0950E+05.txt
            (31577.0, 100290.0, 300.12), // 3.1577E+04.txt
            (409290.0, 286900.0, 298.02), // 4.0929E+05.txt
            (41727.0, 100090.0, 300.13), // 4.1727E+04.txt
            (539090.0, 287200.0, 298.04), // 5.3909E+05.txt
            (56677.0, 101230.0, 298.97), // 5.6677E+04.txt
            (74293.0, 101320.0, 298.75), // 7.4293E+04.txt
            (751820.0, 1474400.0, 298.85), // 7.5182E+05.txt
            (98811.0, 101350.0, 298.5), // 9.8811E+04.txt
        ];
        for (re, p, t) in runs {
            let q = PipeQuestion { bulk_reynolds_number: re, static_pressure_pa: p, temperature_k: t, wall_roughness_rms_um: 0.15 };
            assert!(admit(&q).is_ok(), "{re} {p} {t}: {:?}", admit(&q));
            assert!(re >= COMPARED_REYNOLDS_MIN && re <= COMPARED_REYNOLDS_MAX);
            assert!(p >= COMPARED_PRESSURE_MIN_PA && p <= COMPARED_PRESSURE_MAX_PA);
            assert!(t >= COMPARED_TEMPERATURE_MIN_K.next_down() && t <= COMPARED_TEMPERATURE_MAX_K);
            let speed = bulk_velocity_m_per_s(re, p, t);
            assert!(speed > 3.5 && speed < 33.0, "{re} {p} {t}: {speed}");
        }
    }

    /// Above the highest NSTAP profile no station nearer the wall than the
    /// Pitot profiles' nearest compared point is served; at and below it the
    /// stations go in to 50 viscous units.
    #[test]
    fn a_station_nearer_the_wall_than_was_compared_is_left_out() {
        let high = settle_at_reynolds_number(COMPARED_REYNOLDS_MAX).expect("settles");
        assert_eq!(high.profile_y_over_radius[0], 0.015);
        assert!(high.profile_y_over_radius.iter().all(|e| *e >= PROFILE_Y_OVER_RADIUS_MIN_PITOT));
        assert_eq!(high.profile_y_over_radius.len(), 14);
        let just_above = settle_at_reynolds_number(PROFILE_NSTAP_REYNOLDS_MAX.next_up()).expect("settles");
        assert_eq!(just_above.profile_y_over_radius[0], 0.015);
        let at = settle_at_reynolds_number(PROFILE_NSTAP_REYNOLDS_MAX).expect("settles");
        assert!(at.profile_y_over_radius[0] < 0.001, "{:?}", at.profile_y_over_radius);
        assert!(at.profile_y_plus[0] >= PROFILE_Y_PLUS_MIN);
    }

    #[test]
    fn the_unchecked_entry_points_name_laminar_and_transitional_flow() {
        assert!(matches!(settle_at_reynolds_number(2000.0), Err(PipeReject::LaminarFlow { .. })));
        assert!(matches!(settle_at_reynolds_number(3000.0), Err(PipeReject::TransitionalFlow { .. })));
        assert!(matches!(velocity_over_bulk_at(3999.0, &[0.5]), Err(PipeReject::TransitionalFlow { .. })));
        assert_eq!(settle_at_reynolds_number(f64::NAN).unwrap_err(), PipeReject::InputNotFinite { field: "bulk_reynolds_number" });
    }

    #[test]
    fn a_broken_check_is_refused_by_name() {
        let re = 1.0e6;
        let capped = Numerics { max_newton_iterations: 2, ..SERVED };
        assert!(matches!(settle(re, &capped), Err(PipeReject::NotSettled { iterations: 2, .. })));
        let unreachable = Numerics { imbalance_tolerance: 0.0, ..SERVED };
        assert!(matches!(settle(re, &unreachable), Err(PipeReject::NotSettled { .. })));
        let low_cap = Numerics { imbalance_cap: 1e-6, ..SERVED };
        assert!(matches!(settle(re, &low_cap), Err(PipeReject::ImbalanceGrew { .. })));
        let kicked = Numerics { flux_kick: 1e-6, ..SERVED };
        assert!(matches!(settle(re, &kicked), Err(PipeReject::FluxBroken { iteration: 1, .. })));
        let no_ledger = Numerics { ledger_tolerance: -1.0, ..SERVED };
        assert!(matches!(settle(re, &no_ledger), Err(PipeReject::MomentumLedgerBroken { .. })));
    }

    #[test]
    fn every_refusal_has_a_sentence() {
        let all = [
            PipeReject::InputNotFinite { field: "temperature_k" },
            PipeReject::LaminarFlow { bulk_reynolds_number: 2000.0 },
            PipeReject::TransitionalFlow { bulk_reynolds_number: 3000.0 },
            PipeReject::ReynoldsAboveGrid { bulk_reynolds_number: 1e11 },
            PipeReject::RoughnessNegative { wall_roughness_rms_um: -1.0 },
            PipeReject::RoughWall { wall_roughness_rms_um: 2.5 },
            PipeReject::PressureNotPositive { static_pressure_pa: 0.0 },
            PipeReject::TemperatureOutsideViscosityLaw { temperature_k: 100.0 },
            PipeReject::MachAboveIncompressible { bulk_mach_number: 0.5 },
            PipeReject::NonFiniteValue,
            PipeReject::SingularSystem,
            PipeReject::ImbalanceGrew { imbalance: 1e9 },
            PipeReject::NotSettled { imbalance: 1.0, friction_velocity_change: 1.0, iterations: 60 },
            PipeReject::FluxBroken { residual: 1.0, iteration: 1 },
            PipeReject::MomentumLedgerBroken { residual: 1.0 },
            PipeReject::VelocityNotMonotone { node: 3 },
        ];
        for reject in all {
            assert!(!reject.to_string().is_empty());
            assert!(!reject.to_string().contains("..="), "{reject}");
        }
    }
}

