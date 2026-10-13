//! Argon gas at low density, from one pair potential.
//!
//! Two argon atoms are given the Lennard-Jones 12-6 potential
//! `u(r) = 4 ε [(σ/r)^12 − (σ/r)^6]` with two fixed constants. From it the
//! solver computes, at one temperature and one pressure:
//!
//! - the second virial coefficient `B(T)` and its two temperature
//!   derivatives, by quadrature of the classical Mayer integral;
//! - the second acoustic virial coefficient and the speed of sound, the
//!   pressure series cut after its first term;
//! - the viscosity in the limit of zero density by the kinetic theory of
//!   Chapman and Enskog, with Kihara's second approximation. The collision
//!   integrals are computed here: the classical deflection angle of every
//!   collision, the cross-section at every collision energy, and the thermal
//!   average, each quadrature refined until it settles.
//!
//! The mechanics and the statistics are classical. There is no quantum
//! correction and no three-body force.
//!
//! **The two constants are fitted** to the speed of sound that Goodwin and
//! Hill measured in argon (J. Chem. Eng. Data 54 (2009) 2758), on the 42
//! points at or below 500 kPa of the eleven isotherms from 250 K to
//! 300.117 K. The protocol, the fit and the comparison with the held-back
//! measurements are in `evidence/research/molecular_argon/`.
//!
//! **The comparison of 2026-10-09**, made once through the gateway with the
//! build of this file's first commit. All five lines fixed beforehand were
//! met. Held-back speed of sound (24 points at 310 K to 349.868 K): root mean
//! square error 0.0074 m/s and largest 0.0146 m/s against lines of 0.030 and
//! 0.060, and 0.023 of the ideal gas's error against a line of 0.25.
//! Viscosity (Vogel 2010, 49 points, 291.09 K to 681.96 K, never fitted):
//! mean error 1.47 percent and largest 2.97 percent against lines of 2.0 and
//! 4.0. The viscosity is high at every point, by 3.0 percent at 291 K falling
//! to 0.3 percent at 682 K: 1.4 to 20 times the measurement's uncertainty,
//! 9 times on average. The textbook constants of argon (119.8 K, 3.405 Å), which
//! nobody here fitted, do as well on the held-back speed of sound (0.0068 and
//! 0.0136 m/s) and better on the viscosity (0.80 and 2.05 percent).
//!
//! **The range served** is the range compared, each property in its own.
//! Temperature 250 K to 681.96 K, the union of the two compared spans: the
//! speed of sound was compared from 250 K to 349.868 K, at 30.5 kPa to
//! 467.2 kPa, the lowest and highest pressures of the 66 measured speeds of
//! sound at or below 500 kPa, fitted and held back together; the viscosity
//! was compared from 291.09 K to 681.96 K, asked at zero pressure, the limit
//! of zero density. The pressure says which property a question asks. At
//! 30.5 kPa to 467.2 kPa it asks the speed of sound, and above 349.868 K the
//! solver refuses with `SoundSpeedNotCompared`. At zero it asks the gas at
//! zero density, the viscosity and the second virial coefficient, and below
//! 291.09 K the solver refuses with `ViscosityNotCompared`; the speed of
//! sound it returns there is the ideal gas's. Between zero and 30.5 kPa and
//! from 467.2 kPa to 500 kPa, where neither property was compared, it refuses
//! with `PressureNotCompared`; below 0 kPa and above 500 kPa, where the
//! pressure series is not written, with `PressureOutOfRange`. A question of
//! the speed of sound below 291.09 K also returns the viscosity there, which
//! no measurement here has checked.
//!
//! Everything below is in reduced units unless a name carries a unit: lengths
//! in σ, energies in ε, so the potential is `4 (r^-12 − r^-6)`.

use std::fmt;

const PI: f64 = std::f64::consts::PI;

/// Depth of the pair potential over the Boltzmann constant, kelvin. Fitted.
pub const WELL_DEPTH_K: f64 = 119.33;
/// Distance at which the pair potential is zero, angstrom. Fitted.
pub const ZERO_DISTANCE_ANGSTROM: f64 = 3.3918;
/// Molar mass of argon separated from air, kg/mol.
pub const MOLAR_MASS_KG_PER_MOL: f64 = 39.948e-3;
/// Molar gas constant, J/(mol K). The SI value of 2019.
pub const GAS_CONSTANT_J_PER_MOL_K: f64 = 8.314462618;
/// Boltzmann constant, J/K. Exact in the SI of 2019.
pub const BOLTZMANN_J_PER_K: f64 = 1.380649e-23;
/// Avogadro constant, 1/mol. Exact in the SI of 2019.
pub const AVOGADRO_PER_MOL: f64 = 6.02214076e23;
/// Ratio of the heat capacities of an ideal monatomic gas.
pub const HEAT_CAPACITY_RATIO: f64 = 5.0 / 3.0;

/// Lowest temperature of a question, kelvin: the lowest isotherm the solver was compared with.
pub const TEMPERATURE_MIN_K: f64 = 250.0;
/// Highest temperature of a question, kelvin: the highest viscosity point the solver was compared with.
pub const TEMPERATURE_MAX_K: f64 = 681.96;
/// Lowest pressure the pressure series is written for, kPa.
pub const PRESSURE_MIN_KPA: f64 = 0.0;
/// Highest pressure the pressure series is written for, kPa. Above it the term the pressure series leaves out is larger than the measurement uncertainty.
pub const PRESSURE_MAX_KPA: f64 = 500.0;
/// Lowest pressure of a question, kPa: the lowest pressure at which the speed of sound was compared with measurement.
pub const PRESSURE_COMPARED_MIN_KPA: f64 = 30.5;
/// Highest pressure of a question, kPa: the highest pressure at which the speed of sound was compared with measurement.
pub const PRESSURE_COMPARED_MAX_KPA: f64 = 467.2;
/// Highest temperature at which the speed of sound was compared with measurement, kelvin.
pub const SOUND_SPEED_COMPARED_MAX_K: f64 = 349.868;
/// Lowest temperature at which the viscosity was compared with measurement, kelvin. It was asked at zero pressure.
pub const VISCOSITY_COMPARED_MIN_K: f64 = 291.09;

/// A second virial integral is settled when its two quadrature rules agree to this over the whole range, in reduced units.
pub const VIRIAL_SETTLE_TOLERANCE: f64 = 1.0e-12;
/// The Mayer form and the force form of the second virial coefficient must agree to this, cm³/mol.
pub const VIRIAL_FORMS_TOLERANCE_CM3_PER_MOL: f64 = 1.0e-7;
/// A deflection angle is settled when its two quadrature rules agree to this fraction.
pub const DEFLECTION_SETTLE_TOLERANCE: f64 = 1.0e-9;
/// A cross-section is settled when its two quadrature rules agree to this fraction.
pub const CROSS_SECTION_SETTLE_TOLERANCE: f64 = 1.0e-7;
/// Each collision integral is settled when its two quadrature rules agree to this fraction.
pub const COLLISION_INTEGRAL_SETTLE_TOLERANCE: f64 = 1.0e-7;
/// The most times one panel of a quadrature is halved, and halved again, before the solver refuses.
pub const MAX_REFINEMENTS: u32 = 7;
/// The most times one panel of the thermal average is halved, and halved again, before the solver refuses.
pub const MAX_ENERGY_REFINEMENTS: u32 = 4;

/// Collision energy below which two atoms can orbit, in units of the well depth.
const ORBITING_ENERGY: f64 = 0.8;
/// Closest approach of the critical orbit: `5^(1/6)`.
const CRITICAL_APPROACH: f64 = 1.307_660_486_011_830_6;
/// Just above the orbiting energy the cross-section carries an undulation
/// that repeats in the logarithm of the gap to it, so the panels there are
/// graded: the gap doubles from one edge to the next, this many times, up
/// to twice the orbiting energy.
const ENERGY_GRADED_PANELS: i32 = 6;
/// Panel edges above twice the orbiting energy, in units of the reduced temperature.
const ENERGY_TAIL_EDGES: [f64; 11] = [1.0, 2.0, 3.0, 4.5, 6.0, 8.0, 11.0, 15.0, 20.0, 28.0, 40.0];
/// Below the orbiting energy the cross-section integral stops this many
/// e-folds short of the orbiting impact parameter on each side, and the
/// strip left over is given the mean of `sin²`, one half.
const ORBIT_FOLDS_INNER: f64 = 16.0;
const ORBIT_FOLDS_OUTER: f64 = 9.0;

/// The fifteen-point Kronrod rule on `[-1, 1]`: its positive nodes and the
/// centre, with their weights. The nodes at odd places are the seven-point
/// Gauss rule, whose weights follow.
const KRONROD_NODES: [f64; 8] = [
    0.991_455_371_120_812_6,
    0.949_107_912_342_758_5,
    0.864_864_423_359_769_1,
    0.741_531_185_599_394_4,
    0.586_087_235_467_691_1,
    0.405_845_151_377_397_2,
    0.207_784_955_007_898_47,
    0.0,
];
const KRONROD_WEIGHTS: [f64; 8] = [
    0.022_935_322_010_529_225,
    0.063_092_092_629_978_55,
    0.104_790_010_322_250_18,
    0.140_653_259_715_525_92,
    0.169_004_726_639_267_9,
    0.190_350_578_064_785_4,
    0.204_432_940_075_298_9,
    0.209_482_141_084_727_83,
];
const GAUSS_WEIGHTS: [f64; 4] = [
    0.129_484_966_168_869_7,
    0.279_705_391_489_276_7,
    0.381_830_050_505_118_94,
    0.417_959_183_673_469_4,
];

/// The published calculations this solver is checked against.
pub const REFERENCE_NAME: &str = "National Bureau of Standards tables for the 12-6 potential: second virial coefficient, NSRDS-NBS 47 (1974), Table 4, column gamma = 0; collision integrals, J. Res. NBS 72A (1968) 359, Table 4a";
pub const REFERENCE_URL: &str = "https://nvlpubs.nist.gov/nistpubs/Legacy/NSRDS/nbsnsrds47.pdf";
pub const REFERENCE_URL_COLLISION: &str =
    "https://nvlpubs.nist.gov/nistpubs/jres/72A/jresv72An4p359_A1b.pdf";
/// Date the reference tables were read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// The reduced second virial coefficient agrees with the table to this. The table prints five decimals.
pub const REFERENCE_TOLERANCE_VIRIAL: f64 = 6.0e-6;
/// Each collision integral agrees with the table to this fraction. The authors state one part in a thousand.
pub const REFERENCE_TOLERANCE_COLLISION: f64 = 1.0e-3;

/// Why a result was not returned.
#[derive(Clone, Debug, PartialEq)]
pub enum ArgonReject {
    /// `temperature_k` is outside [`TEMPERATURE_MIN_K`]..=[`TEMPERATURE_MAX_K`].
    TemperatureOutOfRange { temperature_k: f64 },
    /// `pressure_kpa` is outside [`PRESSURE_MIN_KPA`]..=[`PRESSURE_MAX_KPA`].
    PressureOutOfRange { pressure_kpa: f64 },
    /// `pressure_kpa` is above zero and outside [`PRESSURE_COMPARED_MIN_KPA`]..=[`PRESSURE_COMPARED_MAX_KPA`], the pressures at which the speed of sound was compared with measurement; the viscosity was compared at zero pressure only.
    PressureNotCompared { pressure_kpa: f64 },
    /// `pressure_kpa` asks the speed of sound and `temperature_k` is above [`SOUND_SPEED_COMPARED_MAX_K`], the highest temperature at which the speed of sound was compared with measurement.
    SoundSpeedNotCompared { temperature_k: f64 },
    /// `pressure_kpa` is zero, which asks the gas at zero density, and `temperature_k` is below [`VISCOSITY_COMPARED_MIN_K`], the lowest temperature at which the viscosity was compared with measurement.
    ViscosityNotCompared { temperature_k: f64 },
    /// An intermediate value was NaN or infinite.
    NonFiniteValue,
    /// A second virial integral was still open after a panel had been halved [`MAX_REFINEMENTS`] times over.
    VirialNotSettled { refinements: u32 },
    /// The Mayer form and the force form of the second virial coefficient differ by more than [`VIRIAL_FORMS_TOLERANCE_CM3_PER_MOL`].
    VirialFormsDisagree { residual_cm3_per_mol: f64 },
    /// A collision was asked at a closest approach that the atoms do not reach first.
    TurningPointNotOutermost { energy: f64, closest_approach: f64 },
    /// A deflection angle was still open after a panel had been halved [`MAX_REFINEMENTS`] times over.
    DeflectionNotSettled { energy: f64, closest_approach: f64 },
    /// A cross-section was still open after a panel had been halved [`MAX_REFINEMENTS`] times over.
    CrossSectionNotSettled { energy: f64 },
    /// A collision integral was still open after a panel of the thermal average had been halved [`MAX_ENERGY_REFINEMENTS`] times over.
    CollisionIntegralNotSettled { refinements: u32 },
}

impl fmt::Display for ArgonReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TemperatureOutOfRange { temperature_k } => write!(
                f,
                "temperature {temperature_k} K is outside {TEMPERATURE_MIN_K}..={TEMPERATURE_MAX_K} K"
            ),
            Self::PressureOutOfRange { pressure_kpa } => write!(
                f,
                "pressure {pressure_kpa} kPa is outside {PRESSURE_MIN_KPA}..={PRESSURE_MAX_KPA} kPa"
            ),
            Self::PressureNotCompared { pressure_kpa } => write!(
                f,
                "pressure {pressure_kpa} kPa is above zero and outside {PRESSURE_COMPARED_MIN_KPA}..={PRESSURE_COMPARED_MAX_KPA} kPa, the pressures at which the speed of sound was compared with measurement; the viscosity was compared at zero pressure only"
            ),
            Self::SoundSpeedNotCompared { temperature_k } => write!(
                f,
                "temperature {temperature_k} K is above {SOUND_SPEED_COMPARED_MAX_K} K, the highest at which the speed of sound was compared with measurement"
            ),
            Self::ViscosityNotCompared { temperature_k } => write!(
                f,
                "temperature {temperature_k} K at zero pressure is below {VISCOSITY_COMPARED_MIN_K} K, the lowest at which the viscosity was compared with measurement"
            ),
            Self::NonFiniteValue => write!(f, "an integrand or a result was not finite"),
            Self::VirialNotSettled { refinements } => write!(
                f,
                "a second virial integral had not settled after {refinements} halvings of one panel"
            ),
            Self::VirialFormsDisagree {
                residual_cm3_per_mol,
            } => write!(
                f,
                "the two forms of the second virial coefficient differ by {residual_cm3_per_mol} cm3/mol"
            ),
            Self::TurningPointNotOutermost {
                energy,
                closest_approach,
            } => write!(
                f,
                "at reduced energy {energy} the atoms do not reach reduced distance {closest_approach} first"
            ),
            Self::DeflectionNotSettled {
                energy,
                closest_approach,
            } => write!(
                f,
                "the deflection at reduced energy {energy} and reduced closest approach {closest_approach} had not settled"
            ),
            Self::CrossSectionNotSettled { energy } => write!(
                f,
                "the cross-section at reduced energy {energy} had not settled"
            ),
            Self::CollisionIntegralNotSettled { refinements } => write!(
                f,
                "a collision integral had not settled after {refinements} halvings of one panel"
            ),
        }
    }
}

impl std::error::Error for ArgonReject {}

/// What is asked: one temperature and one pressure of the gas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Question {
    pub temperature_k: f64,
    pub pressure_kpa: f64,
}

/// A settled result.
#[derive(Clone, Debug, PartialEq)]
pub struct ArgonGas {
    pub temperature_k: f64,
    pub pressure_kpa: f64,
    /// Temperature over the well depth.
    pub reduced_temperature: f64,
    /// Second virial coefficient `B`, cm³/mol.
    pub second_virial_cm3_per_mol: f64,
    /// `T dB/dT`, cm³/mol.
    pub second_virial_first_derivative_cm3_per_mol: f64,
    /// `T² d²B/dT²`, cm³/mol.
    pub second_virial_second_derivative_cm3_per_mol: f64,
    /// Second acoustic virial coefficient, cm³/mol.
    pub acoustic_virial_cm3_per_mol: f64,
    /// Speed of sound at the asked pressure, m/s.
    pub sound_speed_m_per_s: f64,
    /// Speed of sound of the ideal gas at the same temperature, m/s.
    pub ideal_gas_sound_speed_m_per_s: f64,
    /// Reduced collision integral `Ω(2,2)*`.
    pub collision_integral_22: f64,
    /// Reduced collision integral `Ω(2,3)*`.
    pub collision_integral_23: f64,
    /// Kihara's second-approximation factor for the viscosity.
    pub viscosity_second_approximation_factor: f64,
    /// Viscosity in the limit of zero density, Pa s.
    pub viscosity_pa_s: f64,
    /// Panels halved in the second virial integrals and in the thermal average, together.
    pub refinements: u32,
    /// Mayer form minus force form of the second virial coefficient, cm³/mol.
    pub virial_forms_residual_cm3_per_mol: f64,
    /// The smallest margin by which any collision's turning point was the outermost. Positive.
    pub least_turning_point_margin: f64,
    /// The summed difference of the two quadrature rules over the settled thermal average, as a fraction of the collision integral. The larger of the two.
    pub collision_integral_rule_gap: f64,
    /// Reduced collision energies of the settled cross-section.
    pub collision_energies: Vec<f64>,
    /// Reduced viscosity cross-section `Q(2)*` at each of those energies.
    pub cross_sections: Vec<f64>,
}

/// The numerical limits of one run. The public solver uses [`Numerics::LOCKED`].
#[derive(Clone, Copy, Debug)]
struct Numerics {
    max_virial_refinements: u32,
    max_deflection_refinements: u32,
    max_cross_section_refinements: u32,
    max_energy_refinements: u32,
    collision_integral_tolerance: f64,
    /// Well depth the force form of the virial integral is given, in units of the true one. One.
    force_form_well_depth: f64,
}

impl Numerics {
    const LOCKED: Numerics = Numerics {
        max_virial_refinements: MAX_REFINEMENTS,
        max_deflection_refinements: MAX_REFINEMENTS,
        max_cross_section_refinements: MAX_REFINEMENTS,
        max_energy_refinements: MAX_ENERGY_REFINEMENTS,
        collision_integral_tolerance: COLLISION_INTEGRAL_SETTLE_TOLERANCE,
        force_form_well_depth: 1.0,
    };
}

/// Argon gas at one temperature and one pressure, or the named limit.
pub fn solve(question: Question) -> Result<ArgonGas, ArgonReject> {
    run(question, Numerics::LOCKED)
}

/// The limits the question itself shows.
pub fn check(question: Question) -> Result<(), ArgonReject> {
    let Question {
        temperature_k,
        pressure_kpa,
    } = question;
    if !(temperature_k >= TEMPERATURE_MIN_K && temperature_k <= TEMPERATURE_MAX_K) {
        return Err(ArgonReject::TemperatureOutOfRange { temperature_k });
    }
    if !(pressure_kpa >= PRESSURE_MIN_KPA && pressure_kpa <= PRESSURE_MAX_KPA) {
        return Err(ArgonReject::PressureOutOfRange { pressure_kpa });
    }
    if pressure_kpa == 0.0 {
        // Zero pressure asks the gas at zero density, where the viscosity
        // was compared.
        if temperature_k < VISCOSITY_COMPARED_MIN_K {
            return Err(ArgonReject::ViscosityNotCompared { temperature_k });
        }
    } else if pressure_kpa >= PRESSURE_COMPARED_MIN_KPA && pressure_kpa <= PRESSURE_COMPARED_MAX_KPA {
        // A pressure where the speed of sound was compared asks it.
        if temperature_k > SOUND_SPEED_COMPARED_MAX_K {
            return Err(ArgonReject::SoundSpeedNotCompared { temperature_k });
        }
    } else {
        return Err(ArgonReject::PressureNotCompared { pressure_kpa });
    }
    Ok(())
}

fn run(question: Question, numerics: Numerics) -> Result<ArgonGas, ArgonReject> {
    check(question)?;
    let Question {
        temperature_k,
        pressure_kpa,
    } = question;
    let reduced_temperature = temperature_k / WELL_DEPTH_K;

    // Equilibrium: the second virial coefficient and its derivatives.
    let virial = second_virial(reduced_temperature, numerics)?;
    let sigma_m = ZERO_DISTANCE_ANGSTROM * 1.0e-10;
    let b0_cm3 = 2.0 / 3.0 * PI * AVOGADRO_PER_MOL * sigma_m * sigma_m * sigma_m * 1.0e6;
    let residual = b0_cm3 * (virial.mayer - virial.force);
    if !residual.is_finite() {
        return Err(ArgonReject::NonFiniteValue);
    }
    if residual.abs() > VIRIAL_FORMS_TOLERANCE_CM3_PER_MOL {
        return Err(ArgonReject::VirialFormsDisagree {
            residual_cm3_per_mol: residual,
        });
    }
    let g = HEAT_CAPACITY_RATIO;
    let b = b0_cm3 * virial.mayer;
    let b1 = b0_cm3 * virial.first;
    let b2 = b0_cm3 * virial.second;
    let acoustic = 2.0 * b + 2.0 * (g - 1.0) * b1 + (g - 1.0) * (g - 1.0) / g * b2;
    let rt = GAS_CONSTANT_J_PER_MOL_K * temperature_k;
    let ideal_squared = g * rt / MOLAR_MASS_KG_PER_MOL;
    let squared = g / MOLAR_MASS_KG_PER_MOL * (rt + acoustic * 1.0e-6 * pressure_kpa * 1.0e3);
    if !(squared.is_finite() && squared > 0.0) {
        return Err(ArgonReject::NonFiniteValue);
    }

    // Transport: the collision integrals.
    let collisions = collision_integrals(reduced_temperature, numerics)?;
    let ratio = collisions.omega_23 / collisions.omega_22;
    let factor = 1.0 + 3.0 / 196.0 * (8.0 * ratio - 7.0) * (8.0 * ratio - 7.0);
    let atom_mass_kg = MOLAR_MASS_KG_PER_MOL / AVOGADRO_PER_MOL;
    let viscosity = 5.0 / 16.0
        * (PI * atom_mass_kg * BOLTZMANN_J_PER_K * temperature_k).sqrt()
        / (PI * sigma_m * sigma_m * collisions.omega_22)
        * factor;
    let gas = ArgonGas {
        temperature_k,
        pressure_kpa,
        reduced_temperature,
        second_virial_cm3_per_mol: b,
        second_virial_first_derivative_cm3_per_mol: b1,
        second_virial_second_derivative_cm3_per_mol: b2,
        acoustic_virial_cm3_per_mol: acoustic,
        sound_speed_m_per_s: squared.sqrt(),
        ideal_gas_sound_speed_m_per_s: ideal_squared.sqrt(),
        collision_integral_22: collisions.omega_22,
        collision_integral_23: collisions.omega_23,
        viscosity_second_approximation_factor: factor,
        viscosity_pa_s: viscosity,
        refinements: virial.refinements + collisions.refinements,
        virial_forms_residual_cm3_per_mol: residual,
        least_turning_point_margin: collisions.least_margin,
        collision_integral_rule_gap: collisions.rule_gap,
        collision_energies: collisions.energies,
        cross_sections: collisions.cross_sections,
    };
    let numbers = [
        gas.second_virial_cm3_per_mol,
        gas.second_virial_first_derivative_cm3_per_mol,
        gas.second_virial_second_derivative_cm3_per_mol,
        gas.acoustic_virial_cm3_per_mol,
        gas.sound_speed_m_per_s,
        gas.collision_integral_22,
        gas.collision_integral_23,
        gas.viscosity_pa_s,
    ];
    if numbers.iter().any(|value| !value.is_finite()) || !(gas.viscosity_pa_s > 0.0) {
        return Err(ArgonReject::NonFiniteValue);
    }
    Ok(gas)
}

/// One panel `[a, b]`: the fifteen-point value, and how far the seven-point
/// value is from it.
fn panel<F>(a: f64, b: f64, f: &mut F) -> Result<(f64, f64), ArgonReject>
where
    F: FnMut(f64) -> Result<f64, ArgonReject>,
{
    let middle = 0.5 * (a + b);
    let half = 0.5 * (b - a);
    let centre = f(middle)?;
    let mut fifteen = KRONROD_WEIGHTS[7] * centre;
    let mut seven = GAUSS_WEIGHTS[3] * centre;
    for index in 0..7 {
        let offset = half * KRONROD_NODES[index];
        let pair = f(middle - offset)? + f(middle + offset)?;
        fifteen += KRONROD_WEIGHTS[index] * pair;
        if index % 2 == 1 {
            seven += GAUSS_WEIGHTS[index / 2] * pair;
        }
    }
    let value = half * fifteen;
    let gap = (half * (fifteen - seven)).abs();
    if !(value.is_finite() && gap.is_finite()) {
        return Err(ArgonReject::NonFiniteValue);
    }
    Ok((value, gap))
}

/// An integral over `[a, b]`, refined until it settles: the interval starts
/// as `first_panels` equal panels, and a panel on which the fifteen-point
/// and the seven-point rule differ by more than its share of the allowance
/// is halved, at most `max_refinements` times over. The allowance is
/// `absolute + relative × |first estimate|`, shared by width.
/// `Ok(Err(depth))` when a panel was still open at the cap.
fn settled<F>(
    a: f64,
    b: f64,
    first_panels: usize,
    relative: f64,
    absolute: f64,
    max_refinements: u32,
    mut f: F,
) -> Result<Result<(f64, u32), u32>, ArgonReject>
where
    F: FnMut(f64) -> Result<f64, ArgonReject>,
{
    let n = first_panels.max(1);
    let width = (b - a) / n as f64;
    let mut first = Vec::with_capacity(n);
    let mut estimate = 0.0;
    for index in 0..n {
        let left = a + index as f64 * width;
        let (value, gap) = panel(left, left + width, &mut f)?;
        estimate += value;
        first.push((left, value, gap));
    }
    let allowance_per_width = (absolute + relative * estimate.abs()) / (b - a).abs();
    let mut total = 0.0;
    let mut halvings = 0_u32;
    // Open panels: left edge, width, depth, value, gap. Taken in order.
    let mut open: Vec<(f64, f64, u32, f64, f64)> = first
        .into_iter()
        .rev()
        .map(|(left, value, gap)| (left, width, 0, value, gap))
        .collect();
    while let Some((left, wide, depth, value, gap)) = open.pop() {
        if gap <= allowance_per_width * wide.abs() {
            total += value;
            continue;
        }
        if depth >= max_refinements {
            return Ok(Err(depth));
        }
        halvings += 1;
        let half = 0.5 * wide;
        let (left_value, left_gap) = panel(left, left + half, &mut f)?;
        let (right_value, right_gap) = panel(left + half, left + wide, &mut f)?;
        open.push((left + half, half, depth + 1, right_value, right_gap));
        open.push((left, half, depth + 1, left_value, left_gap));
    }
    Ok(Ok((total, halvings)))
}

/// The reduced pair potential.
fn potential(r: f64) -> f64 {
    let q6 = r.powi(-6);
    4.0 * (q6 * q6 - q6)
}

/// `r` times the derivative of the reduced pair potential.
fn potential_slope_times_r(r: f64) -> f64 {
    let q6 = r.powi(-6);
    4.0 * (6.0 * q6 - 12.0 * q6 * q6)
}

struct Virial {
    /// `B*` from the Mayer form.
    mayer: f64,
    /// `B*` from the force form.
    force: f64,
    /// `T* dB*/dT*`.
    first: f64,
    /// `T*² d²B*/dT*²`.
    second: f64,
    refinements: u32,
}

/// One integral over all distances: `[0, 4]` in four pieces and the rest in
/// the reciprocal of the distance.
fn over_all_distances<F>(numerics: Numerics, mut f: F) -> Result<(f64, u32), ArgonReject>
where
    F: FnMut(f64) -> f64,
{
    const EDGES: [f64; 5] = [0.0, 0.8, 1.2, 2.0, 4.0];
    let mut total = 0.0;
    let mut refinements = 0;
    let mut piece = |a: f64, b: f64, tail: bool, f: &mut F| -> Result<f64, ArgonReject> {
        let outcome = settled(
            a,
            b,
            4,
            0.0,
            VIRIAL_SETTLE_TOLERANCE,
            numerics.max_virial_refinements,
            |x| {
                let value = if tail {
                    if x == 0.0 { 0.0 } else { f(1.0 / x) / (x * x) }
                } else {
                    f(x)
                };
                if value.is_finite() {
                    Ok(value)
                } else {
                    Err(ArgonReject::NonFiniteValue)
                }
            },
        )?;
        match outcome {
            Ok((value, count)) => {
                refinements += count;
                Ok(value)
            }
            Err(count) => Err(ArgonReject::VirialNotSettled { refinements: count }),
        }
    };
    for pair in EDGES.windows(2) {
        total += piece(pair[0], pair[1], false, &mut f)?;
    }
    total += piece(0.0, 1.0 / EDGES[4], true, &mut f)?;
    Ok((total, refinements))
}

fn second_virial(t: f64, numerics: Numerics) -> Result<Virial, ArgonReject> {
    // exp(-u/T) with u = +infinity at r = 0 is 0; the quadrature never lands on r = 0.
    let boltzmann = |r: f64| (-potential(r) / t).exp();
    let (mayer, n0) =
        over_all_distances(numerics, |r| -3.0 * (-potential(r) / t).exp_m1() * r * r)?;
    let depth = numerics.force_form_well_depth;
    let (force, n1) = over_all_distances(numerics, |r| {
        let e = (-depth * potential(r) / t).exp();
        if e == 0.0 {
            0.0
        } else {
            -depth * potential_slope_times_r(r) * e * r * r / t
        }
    })?;
    let (first_moment, n2) = over_all_distances(numerics, |r| {
        let e = boltzmann(r);
        if e == 0.0 { 0.0 } else { potential(r) * e * r * r }
    })?;
    let (second_moment, n3) = over_all_distances(numerics, |r| {
        let e = boltzmann(r);
        if e == 0.0 {
            0.0
        } else {
            let u = potential(r);
            u * u * e * r * r
        }
    })?;
    Ok(Virial {
        mayer,
        force,
        first: -3.0 * first_moment / t,
        second: 6.0 * first_moment / t - 3.0 * second_moment / (t * t),
        refinements: n0 + n1 + n2 + n3,
    })
}

/// The square of the sine of the deflection angle of a collision at reduced
/// energy `energy` whose closest approach is `closest`, and the margin by
/// which that turning point is the outermost.
///
/// With `x = closest/r = cos φ` the angle is
/// `χ = π − 2 (b/closest) ∫_0^{π/2} dφ / sqrt(H(cos²φ))`, where
/// `H(t) = 1 + (4t/E) [q12 (1+t+t²+t³+t⁴) − q6 (1+t)]`, `q6 = closest^-6`.
/// `H` has one minimum on `[0, 1]`. The integral is taken on both sides of
/// it in `φ = φ_min ± δ sinh y` with `δ = sqrt(H_min)`, which keeps the
/// integrand smooth when the atoms nearly orbit.
fn deflection(energy: f64, closest: f64, numerics: Numerics) -> Result<(f64, f64), ArgonReject> {
    let q6 = closest.powi(-6);
    let q12 = q6 * q6;
    let k = 4.0 / energy;
    let impact_over_closest_squared = 1.0 - k * (q12 - q6);
    let not_outermost = ArgonReject::TurningPointNotOutermost {
        energy,
        closest_approach: closest,
    };
    if !(impact_over_closest_squared >= 0.0) {
        return Err(not_outermost);
    }
    // The minimum of H: (1+2t+3t²+4t³+5t⁴)/(1+2t) = closest^6, which rises from 1 to 5.
    let c6 = 1.0 / q6;
    let t_min = if c6 >= 5.0 {
        1.0
    } else if c6 <= 1.0 {
        0.0
    } else {
        let (mut low, mut high) = (0.0_f64, 1.0_f64);
        for _ in 0..80 {
            let t = 0.5 * (low + high);
            let g = (1.0 + t * (2.0 + t * (3.0 + t * (4.0 + t * 5.0)))) / (1.0 + 2.0 * t);
            if g < c6 {
                low = t;
            } else {
                high = t;
            }
        }
        0.5 * (low + high)
    };
    let h_min = 1.0
        + k * (q12 * t_min * (1.0 + t_min * (1.0 + t_min * (1.0 + t_min * (1.0 + t_min))))
            - q6 * t_min * (1.0 + t_min));
    if !(h_min > 0.0) {
        return Err(not_outermost);
    }
    let phi_min = t_min.sqrt().acos();
    let delta = h_min.sqrt().min(1.0);
    let t2 = t_min * t_min;
    let t3 = t2 * t_min;
    let t4 = t3 * t_min;
    let integrand = |offset: f64, y: f64| -> Result<f64, ArgonReject> {
        let e = y.exp();
        let sinh = 0.5 * (e - 1.0 / e);
        let cosh = 0.5 * (e + 1.0 / e);
        let a = offset * delta * sinh;
        // t − t_min = cos²(φ_min + a) − cos²(φ_min) = −sin(a) sin(2 φ_min + a).
        let dt = -a.sin() * (2.0 * phi_min + a).sin();
        let t = t_min + dt;
        let divided = q12
            * (1.0
                + (t + t_min)
                + (t * t + t * t_min + t2)
                + (t * t * t + t * t * t_min + t * t2 + t3)
                + (t * t * t * t + t * t * t * t_min + t * t * t2 + t * t3 + t4))
            - q6 * (1.0 + t + t_min);
        let h = h_min + k * dt * divided;
        if !(h > 0.0) {
            return Err(ArgonReject::TurningPointNotOutermost {
                energy,
                closest_approach: closest,
            });
        }
        Ok(delta * cosh / h.sqrt())
    };
    let mut total = 0.0;
    for (offset, span) in [(-1.0, phi_min), (1.0, 0.5 * PI - phi_min)] {
        if span <= 0.0 {
            continue;
        }
        let y_end = (span / delta).asinh();
        let first = (0.5 * y_end).ceil() as usize;
        let outcome = settled(
            0.0,
            y_end,
            first,
            DEFLECTION_SETTLE_TOLERANCE,
            0.0,
            numerics.max_deflection_refinements,
            |y| integrand(offset, y),
        )?;
        match outcome {
            Ok((value, _)) => total += value,
            Err(_) => {
                return Err(ArgonReject::DeflectionNotSettled {
                    energy,
                    closest_approach: closest,
                });
            }
        }
    }
    let angle = PI - 2.0 * impact_over_closest_squared.sqrt() * total;
    let sine = angle.sin();
    if !sine.is_finite() {
        return Err(ArgonReject::NonFiniteValue);
    }
    Ok((sine * sine, h_min))
}

/// The square of the impact parameter whose closest approach is `r`.
fn impact_squared(energy: f64, r: f64) -> f64 {
    r * r * (1.0 - potential(r) / energy)
}

/// The reduced viscosity cross-section `Q(2)*` at one reduced energy, and
/// the least turning-point margin met.
///
/// `Q(2)* = (3/2) ∫ sin²χ d(b²)`, taken over the closest approach, for which
/// `d(b²) = 2 r H(1) dr`. Above the orbiting energy every closest approach
/// from the head-on one outward is reached. Below it there are two branches,
/// one inside the barrier and one outside, and the deflection diverges
/// logarithmically at the orbiting impact parameter between them.
fn cross_section(energy: f64, numerics: Numerics) -> Result<(f64, f64), ArgonReject> {
    let k = 4.0 / energy;
    let head_on = (0.5 * (1.0 + (1.0 + energy).sqrt())).powf(-1.0 / 6.0);
    let mut least_margin = f64::INFINITY;
    let weighted = |r: f64, margin: &mut f64| -> Result<f64, ArgonReject> {
        let q6 = r.powi(-6);
        let weight = 2.0 * r * (1.0 + k * (5.0 * q6 * q6 - 2.0 * q6));
        let (sine_squared, h_min) = deflection(energy, r, numerics)?;
        if h_min < *margin {
            *margin = h_min;
        }
        Ok(sine_squared * weight)
    };
    let not_settled = ArgonReject::CrossSectionNotSettled { energy };
    let mut total = 0.0;
    let piece = |a: f64,
                 b: f64,
                 first: usize,
                     margin: &mut f64,
                     map: &dyn Fn(f64) -> (f64, f64)|
     -> Result<f64, ArgonReject> {
        let outcome = settled(
            a,
            b,
            first,
            CROSS_SECTION_SETTLE_TOLERANCE,
            1.0e-11,
            numerics.max_cross_section_refinements,
            |y| {
                let (r, jacobian) = map(y);
                if jacobian == 0.0 {
                    return Ok(0.0);
                }
                Ok(weighted(r, margin)? * jacobian)
            },
        )?;
        match outcome {
            Ok((value, _)) => Ok(value),
            Err(_) => Err(not_settled.clone()),
        }
    };
    let far;
    if energy >= ORBITING_ENERGY {
        far = 3.0;
        let delta = (1.0 - ORBITING_ENERGY / energy).sqrt().max(1.0e-6);
        let sinh_map = |sign: f64| {
            move |y: f64| {
                let e = y.exp();
                (
                    CRITICAL_APPROACH + sign * delta * 0.5 * (e - 1.0 / e),
                    delta * 0.5 * (e + 1.0 / e),
                )
            }
        };
        let inner_end = ((CRITICAL_APPROACH - head_on) / delta).asinh();
        total += piece(
            0.0,
            inner_end,
            inner_end.ceil() as usize,
            &mut least_margin,
            &sinh_map(-1.0),
        )?;
        let outer_end = ((far - CRITICAL_APPROACH) / delta).asinh();
        total += piece(
            0.0,
            outer_end,
            outer_end.ceil() as usize,
            &mut least_margin,
            &sinh_map(1.0),
        )?;
    } else {
        let root = (64.0 - 80.0 * energy).sqrt();
        let barrier = ((8.0 - root) / 40.0).powf(-1.0 / 6.0);
        let inner_top = ((8.0 + root) / 40.0).powf(-1.0 / 6.0);
        let orbiting = impact_squared(energy, barrier);
        // The closest approach inside the barrier at the orbiting impact parameter.
        let (mut low, mut high) = (head_on, inner_top);
        for _ in 0..200 {
            let middle = 0.5 * (low + high);
            if impact_squared(energy, middle) < orbiting {
                low = middle;
            } else {
                high = middle;
            }
        }
        let inside = low;
        let span = inside - head_on;
        total += piece(
            0.0,
            ORBIT_FOLDS_INNER,
            (0.5 * ORBIT_FOLDS_INNER) as usize,
            &mut least_margin,
            &|y: f64| {
                let s = span * (-y).exp();
                (inside - s, s)
            },
        )?;
        let cut = inside - span * (-ORBIT_FOLDS_INNER).exp();
        total += 0.5 * (orbiting - impact_squared(energy, cut));
        far = 2.5 * barrier;
        let span = far - barrier;
        total += piece(
            0.0,
            ORBIT_FOLDS_OUTER,
            (0.5 * ORBIT_FOLDS_OUTER).ceil() as usize,
            &mut least_margin,
            &|y: f64| {
                let s = span * (-y).exp();
                (barrier + s, s)
            },
        )?;
        let cut = barrier + span * (-ORBIT_FOLDS_OUTER).exp();
        total += 0.5 * (impact_squared(energy, cut) - orbiting);
    }
    // Glancing collisions, in v = far/r.
    total += piece(0.0, 1.0, 2, &mut least_margin, &|v: f64| {
        if v == 0.0 {
            (f64::INFINITY, 0.0)
        } else {
            (far / v, far / (v * v))
        }
    })?;
    let value = 1.5 * total;
    if !(value.is_finite() && value > 0.0 && least_margin > 0.0) {
        return Err(ArgonReject::NonFiniteValue);
    }
    Ok((value, least_margin))
}

struct Collisions {
    omega_22: f64,
    omega_23: f64,
    refinements: u32,
    least_margin: f64,
    rule_gap: f64,
    energies: Vec<f64>,
    cross_sections: Vec<f64>,
}

/// The edges of the panels of the thermal average at reduced temperature `t`:
/// one panel below the orbiting energy, panels graded toward it from above,
/// then panels spaced in units of the temperature.
fn energy_edges(t: f64) -> Vec<f64> {
    let mut edges = vec![0.0, ORBITING_ENERGY];
    for step in (0..=ENERGY_GRADED_PANELS).rev() {
        edges.push(ORBITING_ENERGY * (1.0 + 0.5_f64.powi(step)));
    }
    let base = 2.0 * ORBITING_ENERGY;
    for edge in ENERGY_TAIL_EDGES {
        edges.push(base + edge * t);
    }
    edges
}

/// One panel of the thermal average.
struct EnergyPanel {
    low: f64,
    high: f64,
    depth: u32,
    /// Fifteen-point values of the two moments.
    moments: [f64; 2],
    /// Distance of the seven-point values from them.
    gaps: [f64; 2],
    energies: [f64; 15],
    cross_sections: [f64; 15],
    least_margin: f64,
}

fn energy_panel(
    low: f64,
    high: f64,
    depth: u32,
    t: f64,
    numerics: Numerics,
) -> Result<EnergyPanel, ArgonReject> {
    let middle = 0.5 * (low + high);
    let half = 0.5 * (high - low);
    let mut energies = [0.0; 15];
    let mut cross_sections = [0.0; 15];
    let mut fifteen = [0.0; 2];
    let mut seven = [0.0; 2];
    let mut least_margin = f64::INFINITY;
    for place in 0..15 {
        // Places 0..7 are the nodes below the centre, 7 the centre, 8..15 above.
        let (index, sign) = if place < 7 {
            (place, -1.0)
        } else {
            (14 - place, 1.0)
        };
        let energy = middle + sign * half * KRONROD_NODES[index];
        let (q, margin) = cross_section(energy, numerics)?;
        least_margin = least_margin.min(margin);
        energies[place] = energy;
        cross_sections[place] = q;
        let second = q * energy.powi(3) * (-energy / t).exp();
        let third = second * energy;
        fifteen[0] += KRONROD_WEIGHTS[index] * second;
        fifteen[1] += KRONROD_WEIGHTS[index] * third;
        if index % 2 == 1 {
            seven[0] += GAUSS_WEIGHTS[index / 2] * second;
            seven[1] += GAUSS_WEIGHTS[index / 2] * third;
        }
    }
    Ok(EnergyPanel {
        low,
        high,
        depth,
        moments: [half * fifteen[0], half * fifteen[1]],
        gaps: [
            (half * (fifteen[0] - seven[0])).abs(),
            (half * (fifteen[1] - seven[1])).abs(),
        ],
        energies,
        cross_sections,
        least_margin,
    })
}

/// `Ω(2,2)*` and `Ω(2,3)*` at reduced temperature `t`:
/// `Ω(2,s)* = ∫ Q(2)*(E) E^(s+1) exp(−E/t) dE / ((s+1)! t^(s+2))`.
///
/// A panel is halved while, for either moment, its two rules differ by more
/// than its share of [`COLLISION_INTEGRAL_SETTLE_TOLERANCE`] of the first
/// estimate of that moment. A first panel's share is one over their number,
/// and a half takes half of its parent's.
fn collision_integrals(t: f64, numerics: Numerics) -> Result<Collisions, ArgonReject> {
    let edges = energy_edges(t);
    let count = edges.len() - 1;
    let mut open = Vec::with_capacity(count);
    let mut estimate = [0.0; 2];
    for pair in edges.windows(2) {
        let first = energy_panel(pair[0], pair[1], 0, t, numerics)?;
        estimate[0] += first.moments[0];
        estimate[1] += first.moments[1];
        open.push(first);
    }
    if !(estimate[0] > 0.0 && estimate[1] > 0.0) {
        return Err(ArgonReject::NonFiniteValue);
    }
    open.reverse();
    let mut moments = [0.0; 2];
    let mut gaps = [0.0; 2];
    let mut refinements = 0_u32;
    let mut least_margin = f64::INFINITY;
    let mut energies = Vec::new();
    let mut cross_sections = Vec::new();
    while let Some(one) = open.pop() {
        let share = numerics.collision_integral_tolerance
            / (count as f64 * 2.0_f64.powi(one.depth as i32));
        if one.gaps[0] <= share * estimate[0] && one.gaps[1] <= share * estimate[1] {
            moments[0] += one.moments[0];
            moments[1] += one.moments[1];
            gaps[0] += one.gaps[0];
            gaps[1] += one.gaps[1];
            least_margin = least_margin.min(one.least_margin);
            energies.extend_from_slice(&one.energies);
            cross_sections.extend_from_slice(&one.cross_sections);
            continue;
        }
        if one.depth >= numerics.max_energy_refinements {
            return Err(ArgonReject::CollisionIntegralNotSettled {
                refinements: one.depth,
            });
        }
        refinements += 1;
        let middle = 0.5 * (one.low + one.high);
        let upper = energy_panel(middle, one.high, one.depth + 1, t, numerics)?;
        let lower = energy_panel(one.low, middle, one.depth + 1, t, numerics)?;
        open.push(upper);
        open.push(lower);
    }
    let omega_22 = moments[0] / (6.0 * t.powi(4));
    let omega_23 = moments[1] / (24.0 * t.powi(5));
    if !(omega_22.is_finite() && omega_23.is_finite() && omega_22 > 0.0 && omega_23 > 0.0) {
        return Err(ArgonReject::NonFiniteValue);
    }
    Ok(Collisions {
        omega_22,
        omega_23,
        refinements,
        least_margin,
        rule_gap: (gaps[0] / moments[0]).max(gaps[1] / moments[1]),
        energies,
        cross_sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(temperature_k: f64, pressure_kpa: f64) -> Question {
        Question {
            temperature_k,
            pressure_kpa,
        }
    }

    /// `B*`, `T* dB*/dT*` and `T*² d²B*/dT*²` of the 12-6 potential from its
    /// exact series: `B* = Σ_j b_j T*^(−(2j+1)/4)`,
    /// `b_j = −2^(j+1/2) Γ((2j−1)/4) / (4 j!)`.
    fn series(t: f64) -> [f64; 3] {
        // Γ(1/4) and Γ(3/4).
        const GAMMA_QUARTER: f64 = 3.625_609_908_221_908;
        const GAMMA_THREE_QUARTERS: f64 = 1.225_416_702_465_177_6;
        let mut gamma_even = GAMMA_THREE_QUARTERS / -0.25; // Γ(−1/4), for j = 0, 2, 4, ...
        let mut gamma_odd = GAMMA_QUARTER; // Γ(1/4), for j = 1, 3, 5, ...
        let mut sums = [0.0; 3];
        let mut factorial = 1.0;
        for j in 0..120_u32 {
            if j > 0 {
                factorial *= f64::from(j);
            }
            let argument = (2.0 * f64::from(j) - 1.0) / 4.0;
            let gamma = if j % 2 == 0 { gamma_even } else { gamma_odd };
            let power = (2.0 * f64::from(j) + 1.0) / 4.0;
            let term = -(2.0_f64.powf(f64::from(j) + 0.5)) * gamma / (4.0 * factorial) * t.powf(-power);
            sums[0] += term;
            sums[1] += -power * term;
            sums[2] += power * (power + 1.0) * term;
            // Γ(x + 1) = x Γ(x) carries each branch to its next member.
            if j % 2 == 0 {
                gamma_even *= argument;
            } else {
                gamma_odd *= argument;
            }
        }
        sums
    }

    #[test]
    fn the_quadrature_rule_is_exact_for_the_polynomials_it_claims() {
        for degree in (0..=22).step_by(2) {
            let (fifteen, gap) = panel(-1.0, 1.0, &mut |x: f64| Ok(x.powi(degree))).expect("finite");
            let exact = 2.0 / f64::from(degree + 1);
            assert!((fifteen - exact).abs() < 4.0e-16, "degree {degree}: {fifteen}");
            // The seven-point rule is exact to degree 13 and not beyond.
            assert_eq!(gap < 1.0e-15, degree <= 12, "degree {degree}: gap {gap}");
        }
    }

    #[test]
    fn a_question_outside_the_range_is_refused_by_name() {
        for temperature_k in [249.999, 681.97, 0.0, -300.0, f64::NAN, f64::INFINITY] {
            match solve(question(temperature_k, 100.0)) {
                Err(ArgonReject::TemperatureOutOfRange { .. }) => {}
                other => panic!("{temperature_k}: {other:?}"),
            }
        }
        for pressure_kpa in [-0.001, 500.001, 6000.0, f64::NAN, f64::INFINITY] {
            match solve(question(300.0, pressure_kpa)) {
                Err(ArgonReject::PressureOutOfRange { .. }) => {}
                other => panic!("{pressure_kpa}: {other:?}"),
            }
        }
        // Above zero and outside the pressures the speed of sound was
        // compared at, neither property was compared.
        for pressure_kpa in [5.0e-324, 1.0, 30.499, 467.201, 500.0] {
            match solve(question(300.0, pressure_kpa)) {
                Err(ArgonReject::PressureNotCompared { pressure_kpa: refused }) => {
                    assert_eq!(refused.to_bits(), pressure_kpa.to_bits())
                }
                other => panic!("{pressure_kpa}: {other:?}"),
            }
        }
        assert_eq!(
            ArgonReject::PressureNotCompared { pressure_kpa: 500.0 }.to_string(),
            "pressure 500 kPa is above zero and outside 30.5..=467.2 kPa, the pressures at which the speed of sound was compared with measurement; the viscosity was compared at zero pressure only"
        );
        // The speed of sound above its compared temperatures, and the gas at
        // zero density below the viscosity's.
        for temperature_k in [SOUND_SPEED_COMPARED_MAX_K.next_up(), 400.0, TEMPERATURE_MAX_K] {
            for pressure_kpa in [PRESSURE_COMPARED_MIN_KPA, 101.325, PRESSURE_COMPARED_MAX_KPA] {
                assert_eq!(
                    check(question(temperature_k, pressure_kpa)),
                    Err(ArgonReject::SoundSpeedNotCompared { temperature_k })
                );
            }
        }
        for temperature_k in [TEMPERATURE_MIN_K, 291.0, VISCOSITY_COMPARED_MIN_K.next_down()] {
            for pressure_kpa in [0.0, -0.0] {
                assert_eq!(
                    check(question(temperature_k, pressure_kpa)),
                    Err(ArgonReject::ViscosityNotCompared { temperature_k })
                );
            }
        }
        // The compared span sits inside the series' range.
        assert!(PRESSURE_MIN_KPA <= PRESSURE_COMPARED_MIN_KPA);
        assert!(PRESSURE_COMPARED_MAX_KPA <= PRESSURE_MAX_KPA);
        // The ends of each compared region are inside.
        assert!(check(question(TEMPERATURE_MIN_K, PRESSURE_COMPARED_MIN_KPA)).is_ok());
        assert!(check(question(SOUND_SPEED_COMPARED_MAX_K, PRESSURE_COMPARED_MAX_KPA)).is_ok());
        assert!(check(question(VISCOSITY_COMPARED_MIN_K, 0.0)).is_ok());
        assert!(check(question(TEMPERATURE_MAX_K, 0.0)).is_ok());
        assert!(check(question(TEMPERATURE_MIN_K, PRESSURE_COMPARED_MIN_KPA.next_down())).is_err());
        assert!(check(question(TEMPERATURE_MIN_K, PRESSURE_COMPARED_MAX_KPA.next_up())).is_err());
    }

    #[test]
    fn the_second_virial_quadrature_is_the_exact_series() {
        for t in [2.0, 2.095, 2.514, 3.4, 5.715, 9.0] {
            let virial = second_virial(t, Numerics::LOCKED).expect("settles");
            let [b, first, second] = series(t);
            assert!((virial.mayer - b).abs() < 2.0e-12, "{t}: {} {b}", virial.mayer);
            assert!((virial.force - b).abs() < 2.0e-12, "{t}: {} {b}", virial.force);
            assert!((virial.first - first).abs() < 2.0e-11, "{t}: {} {first}", virial.first);
            assert!((virial.second - second).abs() < 2.0e-11, "{t}: {} {second}", virial.second);
        }
    }

    #[test]
    fn a_force_form_that_is_not_the_same_potential_is_refused() {
        let numerics = Numerics {
            force_form_well_depth: 1.0 + 1.0e-8,
            ..Numerics::LOCKED
        };
        match run(question(300.0, 100.0), numerics) {
            Err(ArgonReject::VirialFormsDisagree {
                residual_cm3_per_mol,
            }) => assert!(residual_cm3_per_mol.abs() > VIRIAL_FORMS_TOLERANCE_CM3_PER_MOL),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_quadrature_that_may_not_refine_is_refused_and_no_number_is_returned() {
        let no_virial = Numerics {
            max_virial_refinements: 0,
            ..Numerics::LOCKED
        };
        assert_eq!(
            run(question(300.0, 100.0), no_virial),
            Err(ArgonReject::VirialNotSettled { refinements: 0 })
        );
        let no_deflection = Numerics {
            max_deflection_refinements: 0,
            ..Numerics::LOCKED
        };
        match run(question(300.0, 100.0), no_deflection) {
            Err(ArgonReject::DeflectionNotSettled { .. }) => {}
            other => panic!("{other:?}"),
        }
        let no_cross_section = Numerics {
            max_cross_section_refinements: 0,
            ..Numerics::LOCKED
        };
        match run(question(300.0, 100.0), no_cross_section) {
            Err(ArgonReject::CrossSectionNotSettled { .. }) => {}
            other => panic!("{other:?}"),
        }
        let no_average = Numerics {
            max_energy_refinements: 0,
            collision_integral_tolerance: 1.0e-13,
            ..Numerics::LOCKED
        };
        assert_eq!(
            run(question(300.0, 100.0), no_average),
            Err(ArgonReject::CollisionIntegralNotSettled { refinements: 0 })
        );
    }

    #[test]
    fn a_closest_approach_the_atoms_do_not_reach_first_is_refused() {
        // Above the orbiting energy: inside the head-on closest approach.
        match deflection(0.9, 0.96, Numerics::LOCKED) {
            Err(ArgonReject::TurningPointNotOutermost { .. }) => {}
            other => panic!("{other:?}"),
        }
        // Below it: between the two branches, behind the barrier.
        match deflection(0.5, 1.25, Numerics::LOCKED) {
            Err(ArgonReject::TurningPointNotOutermost { .. }) => {}
            other => panic!("{other:?}"),
        }
        // On each branch the collision is computed.
        assert!(deflection(0.5, 1.05, Numerics::LOCKED).is_ok());
        assert!(deflection(0.5, 1.60, Numerics::LOCKED).is_ok());
    }

    #[test]
    fn head_on_and_glancing_collisions_have_the_angles_they_must() {
        // Nearly head-on: the atoms turn back, χ → π, and sin²χ falls with
        // the square of the impact parameter, so with the gap to the head-on
        // closest approach.
        let head_on = (0.5 * (1.0 + (1.0_f64 + 3.0).sqrt())).powf(-1.0 / 6.0);
        let near = deflection(3.0, head_on * (1.0 + 1.0e-6), Numerics::LOCKED).expect("angle").0;
        let nearer = deflection(3.0, head_on * (1.0 + 1.0e-9), Numerics::LOCKED).expect("angle").0;
        assert!(nearer < 1.0e-6, "{nearer}");
        assert!((near / nearer / 1000.0 - 1.0).abs() < 1.0e-3, "{near} {nearer}");
        // Far apart the deflection is that of the attraction alone:
        // χ = −(15 π / 4) / (E b^6) for the potential −4 / r^6.
        let (sine_squared, margin) = deflection(3.0, 12.0, Numerics::LOCKED).expect("angle");
        let b = impact_squared(3.0, 12.0).sqrt();
        let angle = 15.0 * PI / 4.0 / (3.0 * b.powi(6));
        assert!((sine_squared / (angle * angle) - 1.0).abs() < 1.0e-4, "{sine_squared} {angle}");
        assert!(margin > 0.99);
    }

    #[test]
    fn the_cross_section_is_the_same_from_both_sides_of_the_orbiting_energy() {
        let below = cross_section(ORBITING_ENERGY - 1.0e-7, Numerics::LOCKED).expect("below").0;
        let above = cross_section(ORBITING_ENERGY + 1.0e-7, Numerics::LOCKED).expect("above").0;
        assert!((below - above).abs() < 2.0e-6, "{below} {above}");
        assert!((below - 3.066_47).abs() < 1.0e-4, "{below}");
    }

    #[test]
    fn the_same_question_twice_is_bit_identical() {
        let first = solve(question(325.046, 438.6)).expect("settles");
        let second = solve(question(325.046, 438.6)).expect("settles");
        assert_eq!(first.sound_speed_m_per_s.to_bits(), second.sound_speed_m_per_s.to_bits());
        assert_eq!(first.viscosity_pa_s.to_bits(), second.viscosity_pa_s.to_bits());
        assert_eq!(first.cross_sections.len(), second.cross_sections.len());
        for (a, b) in first.cross_sections.iter().zip(&second.cross_sections) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
        assert_eq!(first, second);
    }

    #[test]
    fn the_result_holds_what_it_states() {
        let gas = solve(question(300.0, PRESSURE_COMPARED_MIN_KPA)).expect("settles");
        // The speed of sound is the pressure series cut after its first term.
        let squared = HEAT_CAPACITY_RATIO / MOLAR_MASS_KG_PER_MOL
            * (GAS_CONSTANT_J_PER_MOL_K * 300.0
                + gas.acoustic_virial_cm3_per_mol * 1.0e-6 * PRESSURE_COMPARED_MIN_KPA * 1.0e3);
        assert_eq!(gas.sound_speed_m_per_s.to_bits(), squared.sqrt().to_bits());
        let ideal = (HEAT_CAPACITY_RATIO * GAS_CONSTANT_J_PER_MOL_K * 300.0 / MOLAR_MASS_KG_PER_MOL).sqrt();
        assert!((gas.ideal_gas_sound_speed_m_per_s - ideal).abs() < 1.0e-12);
        assert!(gas.virial_forms_residual_cm3_per_mol.abs() <= VIRIAL_FORMS_TOLERANCE_CM3_PER_MOL);
        assert!(gas.least_turning_point_margin > 0.0);
        assert!(gas.collision_integral_rule_gap <= COLLISION_INTEGRAL_SETTLE_TOLERANCE);
        assert_eq!(gas.collision_energies.len(), gas.cross_sections.len());
        assert!(gas.collision_energies.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(gas.cross_sections.iter().all(|q| q.is_finite() && *q > 0.0));
        // The acoustic coefficient is its three parts.
        let g = HEAT_CAPACITY_RATIO;
        let acoustic = 2.0 * gas.second_virial_cm3_per_mol
            + 2.0 * (g - 1.0) * gas.second_virial_first_derivative_cm3_per_mol
            + (g - 1.0) * (g - 1.0) / g * gas.second_virial_second_derivative_cm3_per_mol;
        assert!((gas.acoustic_virial_cm3_per_mol - acoustic).abs() < 1.0e-12);
        // Pressure raises the speed of sound at 300 K, where the acoustic coefficient is positive.
        let pressed = solve(question(300.0, PRESSURE_COMPARED_MAX_KPA)).expect("settles");
        assert!(gas.acoustic_virial_cm3_per_mol > 0.0);
        assert!(pressed.sound_speed_m_per_s > gas.sound_speed_m_per_s);
        assert_eq!(pressed.viscosity_pa_s.to_bits(), gas.viscosity_pa_s.to_bits());
    }

    /// NSRDS-NBS 47 (1974), Table 4, second virial coefficient of the
    /// (12, 6, 8) potential, column γ = 0, report page 98:
    /// `(T*, B*)`. Read from
    /// <https://nvlpubs.nist.gov/nistpubs/Legacy/NSRDS/nbsnsrds47.pdf>.
    /// Retrieved 2026-10-09. A calculation, not a measurement.
    const NBS_SECOND_VIRIAL: [(f64, f64); 20] = [
        (0.3, -27.88058),
        (0.5, -8.72021),
        (1.0, -2.53808),
        (1.5, -1.20088),
        (2.0, -0.62763),
        (2.2, -0.48171),
        (2.4, -0.36358),
        (2.6, -0.26613),
        (2.8, -0.18451),
        (3.0, -0.11523),
        (3.2, -0.05579),
        (3.4, -0.00428),
        (3.6, 0.04072),
        (3.8, 0.08033),
        (4.0, 0.11542),
        (4.5, 0.18762),
        (5.0, 0.24334),
        (5.5, 0.28740),
        (6.0, 0.32290),
        (7.5, 0.39631),
    ];

    /// M. Klein and F. J. Smith, J. Res. NBS 72A (1968) 359, Table 4a,
    /// collision integrals of the (12, 6) potential, journal page 365:
    /// `(T*, Ω(2,2)*, Ω(2,3)*)`. Read from
    /// <https://nvlpubs.nist.gov/nistpubs/jres/72A/jresv72An4p359_A1b.pdf>.
    /// Retrieved 2026-10-09. A calculation, not a measurement.
    const NBS_COLLISION: [(f64, f64, f64); 21] = [
        (0.3, 2.8456, 2.5815),
        (0.5, 2.2867, 2.0072),
        (1.0, 1.5933, 1.3890),
        (1.5, 1.3151, 1.1747),
        (2.0, 1.1753, 1.0718),
        (2.2, 1.1379, 1.0443),
        (2.4, 1.1069, 1.0213),
        (2.6, 1.0807, 1.0019),
        (2.8, 1.0581, 0.9851),
        (3.0, 1.0386, 0.9704),
        (3.2, 1.0215, 0.9574),
        (3.4, 1.0064, 0.9458),
        (3.6, 0.9929, 0.9353),
        (3.8, 0.9807, 0.9258),
        (4.0, 0.9697, 0.9171),
        (4.5, 0.9461, 0.8981),
        (5.0, 0.9266, 0.8822),
        (5.5, 0.9102, 0.8685),
        (6.0, 0.8961, 0.8564),
        (7.5, 0.8627, 0.8273),
        (9.5, 0.8308, 0.7985),
    ];

    /// The solver as it is served, at every tabulated reduced temperature
    /// that falls inside its temperature range.
    #[test]
    fn nbs_tables_inside_the_served_range() {
        let sigma_m = ZERO_DISTANCE_ANGSTROM * 1.0e-10;
        let b0_cm3 = 2.0 / 3.0 * PI * AVOGADRO_PER_MOL * sigma_m.powi(3) * 1.0e6;
        let mut misses = Vec::new();
        let mut compared = 0;
        let mut worst = (0.0_f64, 0.0_f64);
        for (t, b) in NBS_SECOND_VIRIAL {
            let temperature_k = t * WELL_DEPTH_K;
            // Each temperature asked where the solver answers it: at zero
            // pressure from the viscosity's lowest compared temperature up,
            // and as a question of the speed of sound below it.
            let pressure_kpa = if temperature_k >= VISCOSITY_COMPARED_MIN_K { 0.0 } else { 100.0 };
            if check(question(temperature_k, pressure_kpa)).is_err() {
                continue;
            }
            let gas = solve(question(temperature_k, pressure_kpa)).expect("settles");
            let got = gas.second_virial_cm3_per_mol / b0_cm3;
            worst.0 = worst.0.max((got - b).abs());
            if (got - b).abs() > REFERENCE_TOLERANCE_VIRIAL {
                misses.push(format!("B* at {t}: {got} against {b}"));
            }
            let (_, omega_22, omega_23) = NBS_COLLISION
                .iter()
                .copied()
                .find(|row| row.0 == t)
                .expect("both tables hold this temperature");
            for (label, got, expect) in [
                ("Ω22", gas.collision_integral_22, omega_22),
                ("Ω23", gas.collision_integral_23, omega_23),
            ] {
                worst.1 = worst.1.max((got / expect - 1.0).abs());
                if (got / expect - 1.0).abs() > REFERENCE_TOLERANCE_COLLISION {
                    misses.push(format!("{label} at {t}: {got} against {expect}"));
                }
            }
            compared += 1;
        }
        assert_eq!(compared, 13);
        assert!(misses.is_empty(), "{misses:#?}");
        // What the comparison came to: the table's own rounding for B*, and
        // three parts in ten thousand at most for the integrals.
        assert!(worst.0 < 5.1e-6 && worst.1 < 3.0e-4, "{worst:?}");
    }

    /// The same quadratures outside the served range, where the orbiting
    /// collisions carry most of the integral.
    #[test]
    fn nbs_tables_over_their_whole_range() {
        for (t, b) in NBS_SECOND_VIRIAL {
            let virial = second_virial(t, Numerics::LOCKED).expect("settles");
            assert!((virial.mayer - b).abs() <= REFERENCE_TOLERANCE_VIRIAL, "{t}: {}", virial.mayer);
        }
        for (t, omega_22, omega_23) in NBS_COLLISION {
            let got = collision_integrals(t, Numerics::LOCKED).expect("settles");
            assert!(
                (got.omega_22 / omega_22 - 1.0).abs() <= REFERENCE_TOLERANCE_COLLISION,
                "Ω22 at {t}: {} against {omega_22}",
                got.omega_22
            );
            assert!(
                (got.omega_23 / omega_23 - 1.0).abs() <= REFERENCE_TOLERANCE_COLLISION,
                "Ω23 at {t}: {} against {omega_23}",
                got.omega_23
            );
        }
    }
}
