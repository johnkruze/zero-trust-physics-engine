//! Young's modulus and shear modulus of one steel bar, worked back from the
//! pitches at which it rings.
//!
//! The bars are four test pieces cut from one sheet of structural steel
//! S355J2+N at the Bundesanstalt für Materialforschung und -prüfung (BAM),
//! Berlin, order 5.22/430: ERX1 and ERX2 along the rolling direction, ERY1
//! and ERY2 across it. Each is about 100 mm x 9 mm x 3 mm and 21.3 g. Their
//! length, width, thickness and mass are constants here, as the laboratory
//! entered them (Schilling, Glaubitz, Matzak, Rehmer, Skrotzki, Zenodo,
//! <https://doi.org/10.5281/zenodo.6778336>, version 1.0.0, CC BY 4.0).
//!
//! The question is a series of measured resonant frequencies of one bar, each
//! with its mode: flatwise bending (across the thickness), edgewise bending
//! (across the width), torsion. The answer is the Young's modulus `E` and the
//! shear modulus `G` at which a model of the free bar rings at the flatwise
//! and the torsion frequency, the edgewise frequency that model predicts, and
//! how far a supplied edgewise frequency lies from it. The edgewise frequency
//! never enters the recovery.
//!
//! The forward model. Bending is the Timoshenko beam with free ends,
//! deflection `w`, rotation of the cross-section `φ`, moment `M = E I φ'`,
//! shear force `Q = κ G A (w' − φ)`, `ρ A ẅ = Q'`, `ρ I φ̈ = M' + Q`, and the
//! shear coefficient `κ = 5 (1 + ν) / (6 + 5 ν)` with `ν = E / (2 G) − 1`.
//! In the scaled variables `y = [w/L, φ, M L/(E I), Q L²/(E I)]` over
//! `ξ = x/L` a vibration at angular frequency `ω` obeys `y' = A y`,
//!
//! ```text
//!     |  0     1    0   s2 |      lam = ρ A ω² L⁴ / (E I)
//! A = |  0     0    1    0 |      r2  = I / (A L²)
//!     |  0  -lam r2 0   -1 |      s2  = E I / (κ G A L²)
//!     | -lam   0    0    0 |
//! ```
//!
//! and the first `lam` at which a bar free at `ξ = 0` is free at `ξ = 1` is
//! found by bisection on a determinant of `exp(A)`. Torsion is Saint-Venant's
//! for a rectangle, `f = (1 / 2L) (G K / (ρ I_p))^{1/2} (1 + A)^{1/2}`, with
//! the exact series for `K` and the empirical end correction `A` of
//! ASTM E1875. The model holds no density, no modulus and no Poisson's ratio
//! of steel.
//!
//! The recovery is Gauss–Newton on `ln E` and `ln G`. Every iteration checks
//! that the frequency equation is bracketed, that `E` and `G` are inside the
//! search range, and that the flatwise mode at the trial point holds its
//! energy balance and its free end. A run that misses any of them is refused
//! by name.
//!
//! What a result is. The modulus is the fast (adiabatic) one, about 0.24 %
//! above the slow one for steel. It is the value of the laboratory's
//! resonance method. The data set's own summary sheet states its
//! uncertainty for these measurements: ±1 % at 95 % confidence for both `E`
//! and `G` at room temperature, by the GUM. The laboratory's later paper on
//! other alloys states for the method at most 1 % for `E` and at most 3 %
//! for `G`. The result carries the sheet's 1 % for both, and the paper's 3 %
//! for `G` beside it as a ceiling. The tensile tests of the same sheet give a slope 7 % lower;
//! the comparison and why that slope cannot decide are in
//! `evidence/research/inverse_bam_s355_sheet/`. Poisson's ratio is not
//! returned: it is a small difference of two larger numbers, known on these
//! bars to 13 % of its value.

use std::fmt;

const PI: f64 = std::f64::consts::PI;

/// First root of `cos x cosh x = 1`: a free bar with no shear and no rotary inertia.
pub const EULER_BERNOULLI_FREE_FREE_ROOT: f64 = 4.730040744862704;
/// The bisection for `lam` starts between these fractions of the Euler–Bernoulli value.
pub const LAMBDA_BRACKET_LOW: f64 = 0.4;
pub const LAMBDA_BRACKET_HIGH: f64 = 1.000001;
/// Halvings of that bracket.
pub const LAMBDA_BISECTIONS: u32 = 80;
/// Terms of the Saint-Venant series for the torsion constant of a rectangle.
pub const TORSION_SERIES_TERMS: u32 = 60;
/// Intervals along the bar on which a mode is sampled. Even, for Simpson's rule.
pub const MODE_INTERVALS: usize = 400;

/// Where the search starts, Pa.
pub const START_YOUNGS_MODULUS_PA: f64 = 100.0e9;
pub const START_SHEAR_MODULUS_PA: f64 = 40.0e9;
/// Half-width of the central difference in `ln E` and `ln G`.
pub const JACOBIAN_STEP: f64 = 1.0e-6;
/// The larger of the two steps in `ln E` and `ln G` must fall below this.
pub const STEP_TOLERANCE: f64 = 1.0e-12;
/// The public solver stops after this many Gauss–Newton steps.
pub const MAX_ITERATIONS: u32 = 50;
/// A trial `E` outside this range refuses the run. Pa.
pub const YOUNGS_MODULUS_SEARCH_PA: (f64, f64) = (1.0e9, 2.0e12);
/// A trial `G` outside this range refuses the run. Pa.
pub const SHEAR_MODULUS_SEARCH_PA: (f64, f64) = (0.3e9, 1.0e12);

/// `|strain energy − kinetic energy| / strain energy` of a bending mode above this refuses the run.
pub const ENERGY_BALANCE_TOLERANCE: f64 = 1.0e-8;
/// Moment or shear force left at the far end, as a fraction of its largest value in the bar, above this refuses the run.
pub const FREE_END_TOLERANCE: f64 = 1.0e-8;
/// The model at the returned `E` and `G` must give back each frequency it was fitted to within this fraction.
pub const ROUND_TRIP_TOLERANCE: f64 = 1.0e-10;
/// A supplied edgewise frequency further than this fraction from the predicted one refuses the run.
/// It is line P2 of the validation protocol, the line the prediction was compared at.
pub const EDGEWISE_LIMIT_FRACTION: f64 = 0.0025;

/// Declared range of each measured frequency, Hz: the envelope of the eight
/// scans compared with measurement, two of each of ERX1, ERX2, ERY1 and ERY2.
/// Flatwise runs from ERX1's single scan (1587.428 Hz) to ERY1's single scan
/// (1608.28 Hz), and torsion from ERX2's whole-spectrum scan (8899.446 Hz) to
/// ERY2's whole-spectrum scan (8966.189 Hz).
pub const FLATWISE_RANGE_HZ: (f64, f64) = (1587.428, 1608.28);
/// Edgewise runs from the lowest predicted edgewise frequency of those scans,
/// ERX1's single scan (4695.0353 Hz, cut to the readings' 0.001 Hz), to the
/// highest measured one, ERY1's single scan (4755.856 Hz). The predicted
/// edgewise frequency of every answer must lie inside it too, so that a bar
/// whose two frequencies are answered can be asked again with the edgewise
/// frequency it predicts.
pub const EDGEWISE_RANGE_HZ: (f64, f64) = (4695.035, 4755.856);
pub const TORSION_RANGE_HZ: (f64, f64) = (8899.446, 8966.189);

/// Uncertainty the laboratory states for these measurements, as a fraction
/// of the value: the data set's own summary sheet,
/// `02_secondary_data/results_sonic_resonance_tests.xlsx` in
/// `sonic_resonance_test.zip` of Zenodo record 6778336, cell A30, "Der
/// resultierende Gesamtfehler für den E- und G-Modul beträgt ±1 % (95
/// %-Vertrauensbereich) für die Raumtemperaturmessung", worked out by the
/// GUM from the uncertainties of the measured inputs. One figure for both
/// moduli, for the four bars of this sheet at room temperature.
pub const YOUNGS_MODULUS_METHOD_UNCERTAINTY: f64 = 0.01;
pub const SHEAR_MODULUS_METHOD_UNCERTAINTY: f64 = 0.01;
/// The most the laboratory's later paper states for the shear modulus by the
/// same method at room temperature, as a fraction of the value: Rehmer et
/// al., Sci Data 10, 474 (2023), Technical Validation, "max. 3% for room
/// temperature", for its measurements of three other alloys. For Young's
/// modulus the same passage states "max. 1% for room temperature", the
/// figure of the summary sheet.
pub const SHEAR_MODULUS_METHOD_UNCERTAINTY_CEILING: f64 = 0.03;

/// The published values this solver is checked against.
pub const REFERENCE_NAME: &str = "Schilling, Glaubitz, Matzak, Rehmer, Skrotzki, Full dataset of several mechanical tests on an S355 steel sheet as reference data for digital representations, Zenodo, version 1.0.0: sonic resonance tests to ASTM E1875, bars ERX1 and ERX2";
pub const REFERENCE_URL: &str = "https://doi.org/10.5281/zenodo.6778336";
/// Date the dataset was fetched (UTC).
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// The laboratory's own `E` and `G` are met within this fraction.
pub const REFERENCE_TOLERANCE_FRACTION: f64 = 1.0e-3;

/// One of the four bars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bar {
    Erx1,
    Erx2,
    Ery1,
    Ery2,
}

/// The bars in the order they are listed.
pub const BARS: [Bar; 4] = [Bar::Erx1, Bar::Erx2, Bar::Ery1, Bar::Ery2];

/// Size and mass of one bar as the laboratory entered them in the sheet
/// `E-Modul_rechteck` of its workbook `5.22_430_<bar>_ASTM_E1875.xlsx`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarConstants {
    pub mass_kg: f64,
    pub length_m: f64,
    pub width_m: f64,
    pub thickness_m: f64,
}

impl Bar {
    /// The laboratory's name of the bar.
    pub fn name(self) -> &'static str {
        match self {
            Self::Erx1 => "ERX1",
            Self::Erx2 => "ERX2",
            Self::Ery1 => "ERY1",
            Self::Ery2 => "ERY2",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        BARS.into_iter().find(|bar| bar.name() == name)
    }

    pub fn constants(self) -> BarConstants {
        match self {
            Self::Erx1 => BarConstants {
                mass_kg: 21.348e-3,
                length_m: 100.08e-3,
                width_m: 9.082e-3,
                thickness_m: 2.994e-3,
            },
            Self::Erx2 => BarConstants {
                mass_kg: 21.362e-3,
                length_m: 100.05e-3,
                width_m: 9.067e-3,
                thickness_m: 3.002e-3,
            },
            Self::Ery1 => BarConstants {
                mass_kg: 21.361e-3,
                length_m: 100.08e-3,
                width_m: 9.083e-3,
                thickness_m: 2.995e-3,
            },
            Self::Ery2 => BarConstants {
                mass_kg: 21.367e-3,
                length_m: 100.05e-3,
                width_m: 9.087e-3,
                thickness_m: 2.996e-3,
            },
        }
    }
}

/// How the bar was ringing when a frequency was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// First bending mode, bending across the thickness.
    Flatwise,
    /// First bending mode, bending across the width.
    Edgewise,
    /// First twisting mode.
    Torsion,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Flatwise => "flatwise",
            Self::Edgewise => "edgewise",
            Self::Torsion => "torsion",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Flatwise, Self::Edgewise, Self::Torsion]
            .into_iter()
            .find(|mode| mode.name() == name)
    }

    /// Declared range of a measured frequency of this mode, Hz.
    pub fn range_hz(self) -> (f64, f64) {
        match self {
            Self::Flatwise => FLATWISE_RANGE_HZ,
            Self::Edgewise => EDGEWISE_RANGE_HZ,
            Self::Torsion => TORSION_RANGE_HZ,
        }
    }
}

/// One measured resonant frequency.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub mode: Mode,
    pub frequency_hz: f64,
}

/// Why the moduli were not returned.
#[derive(Clone, Debug, PartialEq)]
pub enum ModulusReject {
    /// The series names one mode twice.
    RepeatedMode { mode: Mode },
    /// A measured frequency is not a finite number inside the declared range of its mode.
    FrequencyOutOfRange { mode: Mode, frequency_hz: f64 },
    /// Not identifiable: the series holds no flatwise frequency, and Young's modulus is recovered from that mode.
    YoungsModulusNotIdentifiable,
    /// Not identifiable: the series holds no torsion frequency, and bending alone fixes the shear modulus only to 19 to 27 % of its value.
    ShearModulusNotIdentifiable,
    /// A trial `E` or `G` left [`YOUNGS_MODULUS_SEARCH_PA`] or [`SHEAR_MODULUS_SEARCH_PA`].
    OutsideSearchRange {
        youngs_modulus_pa: f64,
        shear_modulus_pa: f64,
    },
    /// Outside the forward model's range: at a trial `E` and `G` the frequency equation of a bending mode has no root between [`LAMBDA_BRACKET_LOW`] and [`LAMBDA_BRACKET_HIGH`] of the Euler–Bernoulli value.
    FrequencyEquationNotBracketed {
        mode: Mode,
        youngs_modulus_pa: f64,
        shear_modulus_pa: f64,
    },
    /// An intermediate value was NaN or infinite.
    NonFiniteValue,
    /// Peak strain energy and peak kinetic energy of a computed bending mode differ by more than [`ENERGY_BALANCE_TOLERANCE`].
    EnergyBalanceBroken { mode: Mode, residual: f64 },
    /// A computed bending mode leaves more than [`FREE_END_TOLERANCE`] of moment or shear force at the far end.
    FreeEndBroken { mode: Mode, residual: f64 },
    /// The iteration cap was reached with the step tolerance still open.
    NotConverged { iterations: u32 },
    /// The model at the recovered `E` and `G` does not give back a frequency it was fitted to within [`ROUND_TRIP_TOLERANCE`].
    RoundTripBroken { mode: Mode, residual: f64 },
    /// `E / (2 G) − 1` is not strictly between 0 and 0.5: the frequencies are not those of a uniform bar that is the same in every direction.
    PoissonRatioOutOfRange { poisson_ratio: f64 },
    /// The supplied edgewise frequency is further than [`EDGEWISE_LIMIT_FRACTION`] from the one the recovered `E` and `G` predict.
    ModesInconsistent { residual_fraction: f64 },
    /// The edgewise frequency the recovered `E` and `G` predict is outside [`EDGEWISE_RANGE_HZ`], the declared range of a measured edgewise frequency.
    PredictedEdgewiseOutOfRange { predicted_hz: f64 },
}

impl fmt::Display for ModulusReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepeatedMode { mode } => {
                write!(f, "the series names the {} mode twice", mode.name())
            }
            Self::FrequencyOutOfRange { mode, frequency_hz } => {
                let (low, high) = mode.range_hz();
                write!(
                    f,
                    "{} frequency {frequency_hz} Hz is outside {low} to {high} Hz",
                    mode.name()
                )
            }
            Self::YoungsModulusNotIdentifiable => write!(
                f,
                "Young's modulus is not identifiable: the series holds no flatwise frequency"
            ),
            Self::ShearModulusNotIdentifiable => write!(
                f,
                "the shear modulus is not identifiable: the series holds no torsion frequency"
            ),
            Self::OutsideSearchRange {
                youngs_modulus_pa,
                shear_modulus_pa,
            } => write!(
                f,
                "trial E = {youngs_modulus_pa} Pa, G = {shear_modulus_pa} Pa left the search range"
            ),
            Self::FrequencyEquationNotBracketed {
                mode,
                youngs_modulus_pa,
                shear_modulus_pa,
            } => write!(
                f,
                "outside the forward model's range: the {} frequency equation is not bracketed at E = {youngs_modulus_pa} Pa, G = {shear_modulus_pa} Pa",
                mode.name()
            ),
            Self::NonFiniteValue => write!(f, "a frequency or a modulus was not finite"),
            Self::EnergyBalanceBroken { mode, residual } => write!(
                f,
                "energy balance of the {} mode is off by {residual}",
                mode.name()
            ),
            Self::FreeEndBroken { mode, residual } => write!(
                f,
                "the far end of the {} mode carries {residual} of its largest moment or shear force",
                mode.name()
            ),
            Self::NotConverged { iterations } => {
                write!(f, "not converged after {iterations} iterations")
            }
            Self::RoundTripBroken { mode, residual } => write!(
                f,
                "the recovered moduli give back the {} frequency only within {residual}",
                mode.name()
            ),
            Self::PoissonRatioOutOfRange { poisson_ratio } => write!(
                f,
                "E / (2 G) - 1 = {poisson_ratio} is not between 0 and 0.5"
            ),
            Self::ModesInconsistent { residual_fraction } => write!(
                f,
                "the supplied edgewise frequency is {residual_fraction} of itself away from the predicted one, past {EDGEWISE_LIMIT_FRACTION}"
            ),
            Self::PredictedEdgewiseOutOfRange { predicted_hz } => {
                let (low, high) = EDGEWISE_RANGE_HZ;
                write!(
                    f,
                    "the predicted edgewise frequency {predicted_hz} Hz is outside the declared edgewise range {low} to {high} Hz"
                )
            }
        }
    }
}

impl std::error::Error for ModulusReject {}

/// The recovered moduli of one bar.
#[derive(Clone, Debug, PartialEq)]
pub struct BarModuli {
    pub bar: Bar,
    /// Gauss–Newton steps taken.
    pub iterations: u32,
    pub youngs_modulus_pa: f64,
    pub shear_modulus_pa: f64,
    /// The uncertainty the data set's own summary sheet states for these measurements, ±1 % at 95 % confidence, applied to the value. Pa.
    pub youngs_modulus_method_uncertainty_pa: f64,
    pub shear_modulus_method_uncertainty_pa: f64,
    /// The most the laboratory's later paper states for the shear modulus by this method at room temperature, 3 %, applied to the value. Pa.
    pub shear_modulus_method_uncertainty_ceiling_pa: f64,
    /// The edgewise frequency the recovered `E` and `G` predict. Hz.
    pub edgewise_predicted_hz: f64,
    /// Predicted over supplied edgewise frequency, less one. `None` when the series holds no edgewise frequency.
    pub edgewise_residual_fraction: Option<f64>,
    /// Model over supplied frequency, less one, for the two modes the moduli were fitted to.
    pub flatwise_residual_fraction: f64,
    pub torsion_residual_fraction: f64,
    /// The larger step in `ln E` and `ln G` at the last iteration.
    pub last_step: f64,
    /// `(strain − kinetic) / strain` of each bending mode at the recovered moduli.
    pub flatwise_energy_balance: f64,
    pub edgewise_energy_balance: f64,
    /// Moment or shear force left at the far end, as a fraction of its largest value in the bar.
    pub flatwise_free_end: f64,
    pub edgewise_free_end: f64,
    /// The two bending modes at the recovered moduli: `[w/L, φ, M L/(E I), Q L²/(E I)]`
    /// at each of `MODE_INTERVALS + 1` stations along the bar, flatwise then
    /// edgewise. The energy balances above were computed from these samples.
    pub mode_shapes: Vec<f64>,
}

/// Deliberate faults, for the tests that attack the checks.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Fault {
    None,
    /// Bending without shear deflection and without rotary inertia.
    NoShear,
    /// The root of the frequency equation is moved by this fraction.
    ShiftedRoot(f64),
    /// The deflection samples of each mode are scaled by one plus this.
    ScaledDeflection(f64),
    /// The torsion frequency of the model is scaled by one plus this after the last step.
    DriftedTorsion(f64),
}

/// `E` and `G` of one bar from a series of its measured resonant frequencies.
/// The bar's size and mass, the start, the tolerances and the iteration cap
/// are the constants in this file.
pub fn solve(bar: Bar, series: &[Reading]) -> Result<BarModuli, ModulusReject> {
    let (flatwise_hz, edgewise_hz, torsion_hz) = read_series(series)?;
    recover(
        bar,
        flatwise_hz,
        torsion_hz,
        edgewise_hz,
        (START_YOUNGS_MODULUS_PA, START_SHEAR_MODULUS_PA),
        MAX_ITERATIONS,
        Fault::None,
    )
}

/// What the series itself shows, before anything is computed: a mode named
/// twice, a frequency outside its declared range, or a modulus the series
/// cannot fix.
pub fn check_series(series: &[Reading]) -> Result<(), ModulusReject> {
    read_series(series).map(|_| ())
}

/// The flatwise, edgewise and torsion frequency of a series, each inside its
/// declared range, or the name of what the series lacks.
fn read_series(series: &[Reading]) -> Result<(f64, Option<f64>, f64), ModulusReject> {
    let mut found: [Option<f64>; 3] = [None; 3];
    for reading in series {
        let slot = match reading.mode {
            Mode::Flatwise => 0,
            Mode::Edgewise => 1,
            Mode::Torsion => 2,
        };
        if found[slot].is_some() {
            return Err(ModulusReject::RepeatedMode { mode: reading.mode });
        }
        let (low, high) = reading.mode.range_hz();
        if !(reading.frequency_hz >= low && reading.frequency_hz <= high) {
            return Err(ModulusReject::FrequencyOutOfRange {
                mode: reading.mode,
                frequency_hz: reading.frequency_hz,
            });
        }
        found[slot] = Some(reading.frequency_hz);
    }
    let flatwise_hz = found[0].ok_or(ModulusReject::YoungsModulusNotIdentifiable)?;
    let torsion_hz = found[2].ok_or(ModulusReject::ShearModulusNotIdentifiable)?;
    Ok((flatwise_hz, found[1], torsion_hz))
}

fn recover(
    bar: Bar,
    flatwise_hz: f64,
    torsion_hz: f64,
    edgewise_hz: Option<f64>,
    start_pa: (f64, f64),
    iteration_cap: u32,
    fault: Fault,
) -> Result<BarModuli, ModulusReject> {
    let constants = bar.constants();
    let targets = [flatwise_hz.ln(), torsion_hz.ln()];
    let mut x = [start_pa.0.ln(), start_pa.1.ln()];
    if !x.iter().chain(&targets).all(|value| value.is_finite()) {
        return Err(ModulusReject::NonFiniteValue);
    }
    let mut iterations = 0_u32;
    let last_step = loop {
        if iterations >= iteration_cap {
            return Err(ModulusReject::NotConverged { iterations });
        }
        iterations += 1;
        // The trial point is checked before it is used: bracket, energy, free end.
        let flatwise = bending_mode(&constants, Mode::Flatwise, x[0].exp(), x[1].exp(), fault)?;
        check_mode(Mode::Flatwise, &flatwise)?;
        let here = [
            flatwise.frequency_hz.ln() - targets[0],
            torsion_frequency_hz(&constants, x[1].exp()).ln() - targets[1],
        ];
        let mut jacobian = [[0.0_f64; 2]; 2];
        for column in 0..2 {
            let mut ahead = x;
            let mut behind = x;
            ahead[column] += JACOBIAN_STEP;
            behind[column] -= JACOBIAN_STEP;
            let up = log_frequencies(&constants, ahead, fault)?;
            let down = log_frequencies(&constants, behind, fault)?;
            for row in 0..2 {
                jacobian[row][column] = (up[row] - down[row]) / (2.0 * JACOBIAN_STEP);
            }
        }
        let determinant = jacobian[0][0] * jacobian[1][1] - jacobian[0][1] * jacobian[1][0];
        let step = [
            (-here[0] * jacobian[1][1] + here[1] * jacobian[0][1]) / determinant,
            (-here[1] * jacobian[0][0] + here[0] * jacobian[1][0]) / determinant,
        ];
        if !(step[0].is_finite() && step[1].is_finite()) {
            return Err(ModulusReject::NonFiniteValue);
        }
        x = [x[0] + step[0], x[1] + step[1]];
        let (youngs_modulus_pa, shear_modulus_pa) = (x[0].exp(), x[1].exp());
        let inside = youngs_modulus_pa >= YOUNGS_MODULUS_SEARCH_PA.0
            && youngs_modulus_pa <= YOUNGS_MODULUS_SEARCH_PA.1
            && shear_modulus_pa >= SHEAR_MODULUS_SEARCH_PA.0
            && shear_modulus_pa <= SHEAR_MODULUS_SEARCH_PA.1;
        if !inside {
            return Err(ModulusReject::OutsideSearchRange {
                youngs_modulus_pa,
                shear_modulus_pa,
            });
        }
        let largest = step[0].abs().max(step[1].abs());
        if largest < STEP_TOLERANCE {
            break largest;
        }
    };

    let (youngs_modulus_pa, shear_modulus_pa) = (x[0].exp(), x[1].exp());
    let flatwise = bending_mode(
        &constants,
        Mode::Flatwise,
        youngs_modulus_pa,
        shear_modulus_pa,
        fault,
    )?;
    check_mode(Mode::Flatwise, &flatwise)?;
    let edgewise = bending_mode(
        &constants,
        Mode::Edgewise,
        youngs_modulus_pa,
        shear_modulus_pa,
        fault,
    )?;
    check_mode(Mode::Edgewise, &edgewise)?;
    let mut torsion_model_hz = torsion_frequency_hz(&constants, shear_modulus_pa);
    if let Fault::DriftedTorsion(by) = fault {
        torsion_model_hz *= 1.0 + by;
    }
    let flatwise_residual_fraction = flatwise.frequency_hz / flatwise_hz - 1.0;
    let torsion_residual_fraction = torsion_model_hz / torsion_hz - 1.0;
    for (mode, residual) in [
        (Mode::Flatwise, flatwise_residual_fraction),
        (Mode::Torsion, torsion_residual_fraction),
    ] {
        if !(residual.abs() <= ROUND_TRIP_TOLERANCE) {
            return Err(ModulusReject::RoundTripBroken { mode, residual });
        }
    }
    let poisson_ratio = youngs_modulus_pa / (2.0 * shear_modulus_pa) - 1.0;
    if !(poisson_ratio > 0.0 && poisson_ratio < 0.5) {
        return Err(ModulusReject::PoissonRatioOutOfRange { poisson_ratio });
    }
    let edgewise_residual_fraction = match edgewise_hz {
        None => None,
        Some(supplied) => {
            let residual_fraction = edgewise.frequency_hz / supplied - 1.0;
            if !(residual_fraction.abs() <= EDGEWISE_LIMIT_FRACTION) {
                return Err(ModulusReject::ModesInconsistent { residual_fraction });
            }
            Some(residual_fraction)
        }
    };
    // The same range a supplied edgewise frequency is held to: an answer whose
    // prediction lies outside it names a bar that could not be asked again with
    // the edgewise frequency it predicts.
    let predicted_hz = edgewise.frequency_hz;
    if !(predicted_hz >= EDGEWISE_RANGE_HZ.0 && predicted_hz <= EDGEWISE_RANGE_HZ.1) {
        return Err(ModulusReject::PredictedEdgewiseOutOfRange { predicted_hz });
    }
    let mut mode_shapes = flatwise.samples.clone();
    mode_shapes.extend_from_slice(&edgewise.samples);
    Ok(BarModuli {
        bar,
        iterations,
        youngs_modulus_pa,
        shear_modulus_pa,
        youngs_modulus_method_uncertainty_pa: YOUNGS_MODULUS_METHOD_UNCERTAINTY * youngs_modulus_pa,
        shear_modulus_method_uncertainty_pa: SHEAR_MODULUS_METHOD_UNCERTAINTY * shear_modulus_pa,
        shear_modulus_method_uncertainty_ceiling_pa: SHEAR_MODULUS_METHOD_UNCERTAINTY_CEILING
            * shear_modulus_pa,
        edgewise_predicted_hz: edgewise.frequency_hz,
        edgewise_residual_fraction,
        flatwise_residual_fraction,
        torsion_residual_fraction,
        last_step,
        flatwise_energy_balance: flatwise.energy_balance,
        edgewise_energy_balance: edgewise.energy_balance,
        flatwise_free_end: flatwise.free_end,
        edgewise_free_end: edgewise.free_end,
        mode_shapes,
    })
}

/// `ln` of the flatwise and the torsion frequency of the model at `x = [ln E, ln G]`.
fn log_frequencies(
    constants: &BarConstants,
    x: [f64; 2],
    fault: Fault,
) -> Result<[f64; 2], ModulusReject> {
    let (youngs_modulus_pa, shear_modulus_pa) = (x[0].exp(), x[1].exp());
    let flatwise = bending_root(
        constants,
        Mode::Flatwise,
        youngs_modulus_pa,
        shear_modulus_pa,
        fault,
    )?;
    let out = [
        flatwise.frequency_hz.ln(),
        torsion_frequency_hz(constants, shear_modulus_pa).ln(),
    ];
    if out.iter().all(|value| value.is_finite()) {
        Ok(out)
    } else {
        Err(ModulusReject::NonFiniteValue)
    }
}

fn check_mode(mode: Mode, solved: &BendingMode) -> Result<(), ModulusReject> {
    if !(solved.energy_balance.abs() <= ENERGY_BALANCE_TOLERANCE) {
        return Err(ModulusReject::EnergyBalanceBroken {
            mode,
            residual: solved.energy_balance,
        });
    }
    if !(solved.free_end <= FREE_END_TOLERANCE) {
        return Err(ModulusReject::FreeEndBroken {
            mode,
            residual: solved.free_end,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------- bending

type Matrix4 = [[f64; 4]; 4];

struct BendingRoot {
    lam: f64,
    r2: f64,
    s2: f64,
    frequency_hz: f64,
}

struct BendingMode {
    frequency_hz: f64,
    energy_balance: f64,
    free_end: f64,
    samples: Vec<f64>,
}

/// `exp(fraction · A)` for the matrix `A` of the module note, in closed form.
/// `A` has the eigenvalues `±a` and `±i b`; by the Cayley–Hamilton theorem
/// `exp(A)` is a cubic in `A` whose coefficients are fixed by them. `None`
/// when the eigenvalues do not have that form or a value is not finite.
fn transfer(lam: f64, r2: f64, s2: f64, fraction: f64) -> Option<Matrix4> {
    let sum = lam * (r2 + s2);
    let root = (lam * lam * (r2 - s2) * (r2 - s2) + 4.0 * lam).sqrt();
    let a_squared = 0.5 * (root - sum);
    let b_squared = 0.5 * (root + sum);
    if !(a_squared > 0.0 && b_squared > 0.0 && a_squared.is_finite() && b_squared.is_finite()) {
        return None;
    }
    let a = a_squared.sqrt() * fraction;
    let b = b_squared.sqrt() * fraction;
    let (a2, b2) = (a * a, b * b);
    let (cosh_a, sinh_over_a) = (a.cosh(), a.sinh() / a);
    let (cos_b, sin_over_b) = (b.cos(), b.sin() / b);
    let whole = a2 + b2;
    let c0 = (b2 * cosh_a + a2 * cos_b) / whole;
    let c1 = (b2 * sinh_over_a + a2 * sin_over_b) / whole;
    let c2 = (cosh_a - cos_b) / whole;
    let c3 = (sinh_over_a - sin_over_b) / whole;
    let m: Matrix4 = [
        [0.0, fraction, 0.0, fraction * s2],
        [0.0, 0.0, fraction, 0.0],
        [0.0, -fraction * lam * r2, 0.0, -fraction],
        [-fraction * lam, 0.0, 0.0, 0.0],
    ];
    let m2 = multiply(&m, &m);
    let m3 = multiply(&m2, &m);
    let mut out = [[0.0_f64; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            let identity = if row == column { 1.0 } else { 0.0 };
            let value = c0 * identity
                + c1 * m[row][column]
                + c2 * m2[row][column]
                + c3 * m3[row][column];
            if !value.is_finite() {
                return None;
            }
            out[row][column] = value;
        }
    }
    Some(out)
}

fn multiply(left: &Matrix4, right: &Matrix4) -> Matrix4 {
    let mut out = [[0.0_f64; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            let mut sum = 0.0;
            for inner in 0..4 {
                sum += left[row][inner] * right[inner][column];
            }
            out[row][column] = sum;
        }
    }
    out
}

/// Zero when a bar free at one end is free at the other.
fn free_free_determinant(lam: f64, r2: f64, s2: f64) -> Option<f64> {
    let t = transfer(lam, r2, s2, 1.0)?;
    let value = t[2][0] * t[3][1] - t[2][1] * t[3][0];
    value.is_finite().then_some(value)
}

/// First root `lam` of the free-free frequency equation, by bisection.
/// `None` when the bracket does not hold a sign change.
fn first_lambda(r2: f64, s2: f64) -> Option<f64> {
    let euler_bernoulli = EULER_BERNOULLI_FREE_FREE_ROOT.powi(4);
    let mut low = LAMBDA_BRACKET_LOW * euler_bernoulli;
    let mut high = LAMBDA_BRACKET_HIGH * euler_bernoulli;
    let mut at_low = free_free_determinant(low, r2, s2)?;
    let at_high = free_free_determinant(high, r2, s2)?;
    if at_low == 0.0 {
        return Some(low);
    }
    if at_high == 0.0 {
        return Some(high);
    }
    if (at_low > 0.0) == (at_high > 0.0) {
        return None;
    }
    for _ in 0..LAMBDA_BISECTIONS {
        let middle = 0.5 * (low + high);
        if middle <= low || middle >= high {
            break;
        }
        let at_middle = free_free_determinant(middle, r2, s2)?;
        if at_middle == 0.0 {
            return Some(middle);
        }
        if (at_middle > 0.0) == (at_low > 0.0) {
            low = middle;
            at_low = at_middle;
        } else {
            high = middle;
        }
    }
    Some(0.5 * (low + high))
}

/// Shear coefficient of a rectangle that depends on Poisson's ratio.
fn shear_coefficient(youngs_modulus_pa: f64, shear_modulus_pa: f64) -> f64 {
    let nu = youngs_modulus_pa / (2.0 * shear_modulus_pa) - 1.0;
    5.0 * (1.0 + nu) / (6.0 + 5.0 * nu)
}

/// The side the bar bends across, and the other side of the cross-section.
fn depth_and_breadth(constants: &BarConstants, mode: Mode) -> (f64, f64) {
    match mode {
        Mode::Edgewise => (constants.width_m, constants.thickness_m),
        _ => (constants.thickness_m, constants.width_m),
    }
}

fn bending_root(
    constants: &BarConstants,
    mode: Mode,
    youngs_modulus_pa: f64,
    shear_modulus_pa: f64,
    fault: Fault,
) -> Result<BendingRoot, ModulusReject> {
    let (depth, breadth) = depth_and_breadth(constants, mode);
    let length = constants.length_m;
    let area = depth * breadth;
    let inertia = breadth * depth * depth * depth / 12.0;
    let density = constants.mass_kg / (area * length);
    let (r2, s2) = if fault == Fault::NoShear {
        (0.0, 0.0)
    } else {
        let kappa = shear_coefficient(youngs_modulus_pa, shear_modulus_pa);
        (
            inertia / (area * length * length),
            youngs_modulus_pa * inertia / (kappa * shear_modulus_pa * area * length * length),
        )
    };
    if !(r2.is_finite() && s2.is_finite() && r2 >= 0.0 && s2 >= 0.0) {
        return Err(ModulusReject::NonFiniteValue);
    }
    let mut lam =
        first_lambda(r2, s2).ok_or(ModulusReject::FrequencyEquationNotBracketed {
            mode,
            youngs_modulus_pa,
            shear_modulus_pa,
        })?;
    if let Fault::ShiftedRoot(by) = fault {
        lam *= 1.0 + by;
    }
    let omega = (lam * youngs_modulus_pa * inertia / (density * area * length.powi(4))).sqrt();
    let frequency_hz = omega / (2.0 * PI);
    if !(frequency_hz.is_finite() && frequency_hz > 0.0) {
        return Err(ModulusReject::NonFiniteValue);
    }
    Ok(BendingRoot {
        lam,
        r2,
        s2,
        frequency_hz,
    })
}

/// One bending mode at a trial `E` and `G`: its frequency, its samples along
/// the bar, its energy balance and what is left at its far end.
fn bending_mode(
    constants: &BarConstants,
    mode: Mode,
    youngs_modulus_pa: f64,
    shear_modulus_pa: f64,
    fault: Fault,
) -> Result<BendingMode, ModulusReject> {
    let root = bending_root(constants, mode, youngs_modulus_pa, shear_modulus_pa, fault)?;
    let whole = transfer(root.lam, root.r2, root.s2, 1.0).ok_or(ModulusReject::NonFiniteValue)?;
    // Start free: M = Q = 0, with [w, φ] the direction the frequency equation leaves undetermined.
    let rows = [[whole[2][0], whole[2][1]], [whole[3][0], whole[3][1]]];
    let norms = [
        rows[0][0].hypot(rows[0][1]),
        rows[1][0].hypot(rows[1][1]),
    ];
    let pick = if norms[0] >= norms[1] { 0 } else { 1 };
    if !(norms[pick] > 0.0 && norms[pick].is_finite()) {
        return Err(ModulusReject::NonFiniteValue);
    }
    let mut state = [
        rows[pick][1] / norms[pick],
        -rows[pick][0] / norms[pick],
        0.0,
        0.0,
    ];
    let step = transfer(root.lam, root.r2, root.s2, 1.0 / MODE_INTERVALS as f64)
        .ok_or(ModulusReject::NonFiniteValue)?;
    let mut samples = Vec::with_capacity(4 * (MODE_INTERVALS + 1));
    samples.extend_from_slice(&state);
    for _ in 0..MODE_INTERVALS {
        let mut next = [0.0_f64; 4];
        for row in 0..4 {
            for column in 0..4 {
                next[row] += step[row][column] * state[column];
            }
        }
        state = next;
        samples.extend_from_slice(&state);
    }
    if let Fault::ScaledDeflection(by) = fault {
        for station in 0..=MODE_INTERVALS {
            samples[4 * station] *= 1.0 + by;
        }
    }
    let (energy_balance, free_end) = mode_checks(&samples, root.lam, root.r2, root.s2);
    if !(energy_balance.is_finite() && free_end.is_finite()) {
        return Err(ModulusReject::NonFiniteValue);
    }
    Ok(BendingMode {
        frequency_hz: root.frequency_hz,
        energy_balance,
        free_end,
        samples,
    })
}

/// `(strain − kinetic) / strain` by Simpson's rule over the samples, and the
/// larger of moment and shear force left at the far end as a fraction of its
/// largest value in the bar. In the scaled variables
/// `strain = ∫ (y3² + s2 y4²) dξ` and `kinetic = lam ∫ (y1² + r2 y2²) dξ`.
fn mode_checks(samples: &[f64], lam: f64, r2: f64, s2: f64) -> (f64, f64) {
    let stations = samples.len() / 4;
    let intervals = stations - 1;
    let mut strain = 0.0;
    let mut kinetic = 0.0;
    let mut largest_moment = 0.0_f64;
    let mut largest_shear = 0.0_f64;
    for station in 0..stations {
        let y = &samples[4 * station..4 * station + 4];
        let weight = if station == 0 || station == intervals {
            1.0
        } else if station % 2 == 1 {
            4.0
        } else {
            2.0
        };
        strain += weight * (y[2] * y[2] + s2 * y[3] * y[3]);
        kinetic += weight * (y[0] * y[0] + r2 * y[1] * y[1]);
        largest_moment = largest_moment.max(y[2].abs());
        largest_shear = largest_shear.max(y[3].abs());
    }
    let h = 1.0 / intervals as f64;
    strain *= h / 3.0;
    kinetic *= lam * h / 3.0;
    let end = &samples[4 * intervals..];
    let free_end = (end[2].abs() / largest_moment).max(end[3].abs() / largest_shear);
    ((strain - kinetic) / strain, free_end)
}

// ---------------------------------------------------------------- torsion

/// Saint-Venant torsion constant of a `width` x `thickness` rectangle, `width >= thickness`.
fn torsion_constant(width: f64, thickness: f64) -> f64 {
    let mut total = 0.0;
    for k in 0..TORSION_SERIES_TERMS {
        let n = (2 * k + 1) as f64;
        total += (n * PI * width / (2.0 * thickness)).tanh() / n.powi(5);
    }
    width * thickness.powi(3) / 3.0 * (1.0 - 192.0 * thickness / (PI.powi(5) * width) * total)
}

/// Empirical end correction `A` of ASTM E1875, as printed in Rehmer et al.,
/// Sci Data 10, 474 (2023), equation 5.
fn torsion_end_correction(width: f64, thickness: f64) -> f64 {
    let x = width / thickness;
    (0.5062 - 0.8776 * x + 0.3504 * x * x - 0.0078 * x * x * x) / (12.03 * x + 9.892 * x * x)
}

fn torsion_frequency_hz(constants: &BarConstants, shear_modulus_pa: f64) -> f64 {
    let (width, thickness) = (constants.width_m, constants.thickness_m);
    let area = width * thickness;
    let density = constants.mass_kg / (area * constants.length_m);
    let polar = (width * thickness.powi(3) + thickness * width.powi(3)) / 12.0;
    let plain = (shear_modulus_pa * torsion_constant(width, thickness) / (density * polar)).sqrt()
        / (2.0 * constants.length_m);
    plain * (1.0 + torsion_end_correction(width, thickness)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read on 2026-10-09 ([`REFERENCE_RETRIEVED`]) from
    /// <https://doi.org/10.5281/zenodo.6778336> ([`REFERENCE_URL`]), archive
    /// `sonic_resonance_test.zip`, workbooks `5.22_430_ERX1_ASTM_E1875.xlsx`
    /// and `5.22_430_ERX2_ASTM_E1875.xlsx`, sheets `E-Modul_rechteck` and
    /// `G-Modul_rechteck`. Each row is one scan: the three measured
    /// frequencies in Hz, then the `E` from the flatwise mode and the `G`
    /// the laboratory itself computed with the formulas of ASTM E1875, GPa.
    /// These are measurements and the laboratory's reduction of them.
    struct Scan {
        bar: Bar,
        flatwise_hz: f64,
        edgewise_hz: f64,
        torsion_hz: f64,
        laboratory_e_gpa: f64,
        laboratory_g_gpa: f64,
    }

    fn published_scans() -> [Scan; 4] {
        [
            Scan {
                bar: Bar::Erx1,
                flatwise_hz: 1588.627,
                edgewise_hz: 4700.015,
                torsion_hz: 8950.895,
                laboratory_e_gpa: 210.94957160140265,
                laboratory_g_gpa: 80.49577132572126,
            },
            Scan {
                bar: Bar::Erx1,
                flatwise_hz: 1587.428,
                edgewise_hz: 4698.493,
                torsion_hz: 8944.466,
                laboratory_e_gpa: 210.63126769653948,
                laboratory_g_gpa: 80.38018032315216,
            },
            Scan {
                bar: Bar::Erx2,
                flatwise_hz: 1598.579,
                edgewise_hz: 4704.533,
                torsion_hz: 8899.446,
                laboratory_e_gpa: 212.20393711932454,
                laboratory_g_gpa: 78.9970080756985,
            },
            Scan {
                bar: Bar::Erx2,
                flatwise_hz: 1598.747,
                edgewise_hz: 4707.067,
                torsion_hz: 8901.011,
                laboratory_e_gpa: 212.24854190237068,
                laboratory_g_gpa: 79.02479434195808,
            },
        ]
    }

    /// The eight scans compared with measurement, the four above and the four
    /// of the bars cut across the rolling direction, read from the same
    /// archive (workbooks `5.22_430_ERY1_ASTM_E1875.xlsx` and
    /// `5.22_430_ERY2_ASTM_E1875.xlsx`): bar, flatwise, edgewise, torsion, Hz.
    /// The whole-spectrum scan of each bar comes first.
    const COMPARED_SCANS: [(Bar, f64, f64, f64); 8] = [
        (Bar::Erx1, 1588.627, 4700.015, 8950.895),
        (Bar::Erx1, 1587.428, 4698.493, 8944.466),
        (Bar::Erx2, 1598.579, 4704.533, 8899.446),
        (Bar::Erx2, 1598.747, 4707.067, 8901.011),
        (Bar::Ery1, 1606.61, 4755.13, 8948.668),
        (Bar::Ery1, 1608.28, 4755.856, 8936.544),
        (Bar::Ery2, 1607.396, 4755.13, 8966.189),
        (Bar::Ery2, 1606.268, 4753.821, 8962.771),
    ];

    fn three(flatwise_hz: f64, torsion_hz: f64, edgewise_hz: f64) -> [Reading; 3] {
        [
            Reading {
                mode: Mode::Flatwise,
                frequency_hz: flatwise_hz,
            },
            Reading {
                mode: Mode::Torsion,
                frequency_hz: torsion_hz,
            },
            Reading {
                mode: Mode::Edgewise,
                frequency_hz: edgewise_hz,
            },
        ]
    }

    /// The declared ranges are the envelope of the eight scans compared with
    /// measurement: each end is a reading of one of them, or, at the low end
    /// of edgewise, the lowest prediction cut to the readings' 0.001 Hz.
    #[test]
    fn the_declared_ranges_are_the_envelope_of_the_eight_compared_scans() {
        let fold = |pick: fn(&(Bar, f64, f64, f64)) -> f64| {
            COMPARED_SCANS
                .iter()
                .map(pick)
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| {
                    (low.min(value), high.max(value))
                })
        };
        assert_eq!(fold(|scan| scan.1), FLATWISE_RANGE_HZ);
        assert_eq!(fold(|scan| scan.3), TORSION_RANGE_HZ);
        let measured_edgewise = fold(|scan| scan.2);
        assert_eq!(measured_edgewise.1, EDGEWISE_RANGE_HZ.1);
        let mut lowest_prediction = f64::INFINITY;
        let mut highest_prediction = f64::NEG_INFINITY;
        for (bar, flatwise, _, torsion) in COMPARED_SCANS {
            let solved = solve(bar, &two(flatwise, torsion)).expect("a compared scan is solved");
            lowest_prediction = lowest_prediction.min(solved.edgewise_predicted_hz);
            highest_prediction = highest_prediction.max(solved.edgewise_predicted_hz);
        }
        assert_eq!((lowest_prediction * 1000.0).floor() / 1000.0, EDGEWISE_RANGE_HZ.0);
        assert!(lowest_prediction < measured_edgewise.0);
        assert!(highest_prediction < EDGEWISE_RANGE_HZ.1);
    }

    /// Every scan compared with measurement is answered with its two
    /// frequencies and with all three, with the same moduli, and its edgewise
    /// frequency is predicted inside line P2.
    #[test]
    fn every_compared_scan_is_answered_with_two_frequencies_and_with_three() {
        let mut worst = 0.0_f64;
        for (bar, flatwise, edgewise, torsion) in COMPARED_SCANS {
            let pair = solve(bar, &two(flatwise, torsion)).expect("two frequencies");
            let whole = solve(bar, &three(flatwise, torsion, edgewise)).expect("three frequencies");
            assert_eq!(pair.youngs_modulus_pa.to_bits(), whole.youngs_modulus_pa.to_bits());
            assert_eq!(pair.shear_modulus_pa.to_bits(), whole.shear_modulus_pa.to_bits());
            let residual = whole.edgewise_residual_fraction.expect("edgewise was supplied");
            assert!(residual.abs() <= 0.0025, "{residual}");
            worst = worst.max(residual.abs());
            let predicted = pair.edgewise_predicted_hz;
            assert!(predicted >= EDGEWISE_RANGE_HZ.0 && predicted <= EDGEWISE_RANGE_HZ.1);
        }
        // The worst of the eight, ERY1's whole-spectrum scan, -0.114 percent.
        assert!((worst - 0.00114).abs() < 1.0e-5, "{worst}");
    }

    /// The two routes agree everywhere in the declared ranges: a pair of
    /// frequencies is answered exactly when the same pair with the edgewise
    /// frequency it predicts is answered, with the same bits. A prediction
    /// outside the edgewise range is refused by name on both routes.
    #[test]
    fn two_frequencies_and_the_three_they_imply_are_answered_or_refused_together() {
        let points = |range: (f64, f64)| [range.0, 0.5 * (range.0 + range.1), range.1];
        let mut refused = 0;
        for bar in BARS {
            for flatwise in points(FLATWISE_RANGE_HZ) {
                for torsion in points(TORSION_RANGE_HZ) {
                    match solve(bar, &two(flatwise, torsion)) {
                        Ok(pair) => {
                            let predicted = pair.edgewise_predicted_hz;
                            let whole = solve(bar, &three(flatwise, torsion, predicted))
                                .unwrap_or_else(|reject| {
                                    panic!("{} {flatwise} {torsion} {predicted}: {reject}", bar.name())
                                });
                            assert_eq!(
                                pair.youngs_modulus_pa.to_bits(),
                                whole.youngs_modulus_pa.to_bits()
                            );
                            assert_eq!(whole.edgewise_residual_fraction, Some(0.0));
                        }
                        Err(ModulusReject::PredictedEdgewiseOutOfRange { predicted_hz }) => {
                            refused += 1;
                            assert!(
                                predicted_hz < EDGEWISE_RANGE_HZ.0
                                    || predicted_hz > EDGEWISE_RANGE_HZ.1
                            );
                            match solve(bar, &three(flatwise, torsion, predicted_hz)) {
                                Err(ModulusReject::FrequencyOutOfRange {
                                    mode: Mode::Edgewise,
                                    frequency_hz,
                                }) => assert_eq!(frequency_hz.to_bits(), predicted_hz.to_bits()),
                                other => panic!("{other:?}"),
                            }
                        }
                        Err(other) => panic!("{} {flatwise} {torsion}: {other}", bar.name()),
                    }
                }
            }
        }
        // The corners where one bar's flatwise reading meets the other direction's
        // bars are outside the comparison, and some of them are refused.
        assert!(refused > 0);
        match solve(
            Bar::Erx1,
            &two(FLATWISE_RANGE_HZ.0, TORSION_RANGE_HZ.0),
        ) {
            Err(ModulusReject::PredictedEdgewiseOutOfRange { predicted_hz }) => {
                assert!(predicted_hz < EDGEWISE_RANGE_HZ.0, "{predicted_hz}")
            }
            other => panic!("{other:?}"),
        }
    }

    /// Line P2: a supplied edgewise frequency within 0.25 percent of the
    /// prediction is answered, and one past it is refused by name.
    #[test]
    fn the_edgewise_check_is_held_at_line_p2() {
        assert_eq!(EDGEWISE_LIMIT_FRACTION, 0.0025);
        // ERX2's single scan reads low and ERY1's single scan reads high, so each
        // edgewise frequency that is moved stays inside the declared range.
        for ((bar, flatwise, _, torsion), sign) in
            [(COMPARED_SCANS[3], -1.0), (COMPARED_SCANS[5], 1.0)]
        {
            let predicted = solve(bar, &two(flatwise, torsion))
                .expect("solved")
                .edgewise_predicted_hz;
            let inside = predicted / (1.0 + sign * 0.0024);
            let past = predicted / (1.0 + sign * 0.0026);
            let answered = solve(bar, &three(flatwise, torsion, inside)).expect("inside P2");
            assert!((answered.edgewise_residual_fraction.unwrap() - sign * 0.0024).abs() < 1.0e-9);
            match solve(bar, &three(flatwise, torsion, past)) {
                Err(ModulusReject::ModesInconsistent { residual_fraction }) => {
                    assert!((residual_fraction - sign * 0.0026).abs() < 1.0e-9, "{residual_fraction}")
                }
                other => panic!("{other:?}"),
            }
        }
    }

    fn series(scan: &Scan) -> [Reading; 3] {
        [
            Reading {
                mode: Mode::Flatwise,
                frequency_hz: scan.flatwise_hz,
            },
            Reading {
                mode: Mode::Edgewise,
                frequency_hz: scan.edgewise_hz,
            },
            Reading {
                mode: Mode::Torsion,
                frequency_hz: scan.torsion_hz,
            },
        ]
    }

    fn two(flatwise_hz: f64, torsion_hz: f64) -> [Reading; 2] {
        [
            Reading {
                mode: Mode::Flatwise,
                frequency_hz: flatwise_hz,
            },
            Reading {
                mode: Mode::Torsion,
                frequency_hz: torsion_hz,
            },
        ]
    }

    fn faulted(scan: &Scan, fault: Fault) -> Result<BarModuli, ModulusReject> {
        recover(
            scan.bar,
            scan.flatwise_hz,
            scan.torsion_hz,
            Some(scan.edgewise_hz),
            (START_YOUNGS_MODULUS_PA, START_SHEAR_MODULUS_PA),
            MAX_ITERATIONS,
            fault,
        )
    }

    /// The uncertainty a result carries is the data set's own: one percent of
    /// each modulus, as its summary sheet states for both; and the three
    /// percent of the laboratory's later paper stands beside the shear
    /// modulus as a ceiling.
    #[test]
    fn the_uncertainty_is_the_summary_sheet_s_and_the_paper_s_stands_beside_it() {
        assert_eq!(YOUNGS_MODULUS_METHOD_UNCERTAINTY, 0.01);
        assert_eq!(SHEAR_MODULUS_METHOD_UNCERTAINTY, 0.01);
        assert_eq!(SHEAR_MODULUS_METHOD_UNCERTAINTY_CEILING, 0.03);
        for scan in published_scans() {
            let solved = solve(scan.bar, &series(&scan)).expect("a published scan is solved");
            assert_eq!(
                solved.youngs_modulus_method_uncertainty_pa,
                0.01 * solved.youngs_modulus_pa
            );
            assert_eq!(solved.shear_modulus_method_uncertainty_pa, 0.01 * solved.shear_modulus_pa);
            assert_eq!(
                solved.shear_modulus_method_uncertainty_ceiling_pa,
                0.03 * solved.shear_modulus_pa
            );
        }
    }

    #[test]
    fn bam_published_moduli_of_erx1_and_erx2() {
        println!("BAM S355 sheet, Zenodo 6778336, retrieved {REFERENCE_RETRIEVED}");
        let mut misses = Vec::new();
        for scan in published_scans() {
            let solved = solve(scan.bar, &series(&scan)).expect("a published scan is solved");
            let e = solved.youngs_modulus_pa / 1.0e9 / scan.laboratory_e_gpa - 1.0;
            let g = solved.shear_modulus_pa / 1.0e9 / scan.laboratory_g_gpa - 1.0;
            let edge = solved.edgewise_residual_fraction.expect("edgewise was supplied");
            println!(
                "{} E {:.4} GPa ({:+.3e} of the laboratory's) G {:.4} GPa ({:+.3e}) edgewise predicted {:.3} Hz ({:+.3e} of measured) in {} iterations",
                scan.bar.name(),
                solved.youngs_modulus_pa / 1.0e9,
                e,
                solved.shear_modulus_pa / 1.0e9,
                g,
                solved.edgewise_predicted_hz,
                edge,
                solved.iterations
            );
            if e.abs() > REFERENCE_TOLERANCE_FRACTION {
                misses.push(format!("{} E off by {e}", scan.bar.name()));
            }
            if g.abs() > REFERENCE_TOLERANCE_FRACTION {
                misses.push(format!("{} G off by {g}", scan.bar.name()));
            }
            // Line P2 of the protocol: the resonance the recovery never used.
            if edge.abs() > 0.0025 {
                misses.push(format!("{} edgewise off by {edge}", scan.bar.name()));
            }
        }
        assert!(misses.is_empty(), "{misses:#?}");
    }

    /// The correction factor `T1` and the flexure formula of ASTM E1875 as
    /// printed in Rehmer et al., Sci Data 10, 474 (2023), equations 1 and 2,
    /// <https://doi.org/10.1038/s41597-023-02387-6>, read 2026-10-09. The
    /// standard prints the constant 0.9465 where the beam gives 0.946415, a
    /// part in ten thousand, and its `T1` is a fitted curve. The two agree
    /// within 3 parts in ten thousand for the thin way and, for the wide way,
    /// within [`REFERENCE_TOLERANCE_FRACTION`] (9 parts in ten thousand at most).
    fn astm_youngs_modulus_pa(c: &BarConstants, depth: f64, breadth: f64, f: f64, mu: f64) -> f64 {
        let x2 = (depth / c.length_m).powi(2);
        let x4 = x2 * x2;
        let t1 = 1.0 + 6.585 * (1.0 + 0.0752 * mu + 0.8109 * mu * mu) * x2
            - 0.868 * x4
            - 8.34 * (1.0 + 0.2023 * mu + 2.173 * mu * mu) * x4
                / (1.0 + 6.338 * (1.0 + 0.1408 * mu + 1.536 * mu * mu) * x2);
        0.9465 * c.mass_kg * f * f / breadth * (c.length_m / depth).powi(3) * t1
    }

    #[test]
    fn the_standard_formula_gives_the_same_modulus_from_both_bending_modes() {
        for scan in published_scans() {
            let solved = solve(scan.bar, &series(&scan)).expect("solved");
            let c = scan.bar.constants();
            let mu = solved.youngs_modulus_pa / (2.0 * solved.shear_modulus_pa) - 1.0;
            let flat =
                astm_youngs_modulus_pa(&c, c.thickness_m, c.width_m, scan.flatwise_hz, mu);
            let edge = astm_youngs_modulus_pa(
                &c,
                c.width_m,
                c.thickness_m,
                solved.edgewise_predicted_hz,
                mu,
            );
            assert!(
                (flat / solved.youngs_modulus_pa - 1.0).abs() < 3.0e-4,
                "flatwise {flat}"
            );
            assert!(
                (edge / solved.youngs_modulus_pa - 1.0).abs() < REFERENCE_TOLERANCE_FRACTION,
                "edgewise {edge}"
            );
        }
    }

    #[test]
    fn without_shear_and_rotary_inertia_the_root_is_the_euler_bernoulli_one() {
        let lam = first_lambda(0.0, 0.0).expect("bracketed");
        assert!(
            (lam.powf(0.25) - EULER_BERNOULLI_FREE_FREE_ROOT).abs() < 1.0e-12,
            "{}",
            lam.powf(0.25)
        );
    }

    #[test]
    fn frequencies_made_by_the_model_give_back_the_moduli() {
        for bar in BARS {
            let c = bar.constants();
            // Inside the declared ranges for every bar.
            let (e, g) = (212.0e9, 80.0e9);
            let flat = bending_root(&c, Mode::Flatwise, e, g, Fault::None).unwrap();
            let edge = bending_root(&c, Mode::Edgewise, e, g, Fault::None).unwrap();
            let solved = solve(
                bar,
                &[
                    Reading {
                        mode: Mode::Torsion,
                        frequency_hz: torsion_frequency_hz(&c, g),
                    },
                    Reading {
                        mode: Mode::Edgewise,
                        frequency_hz: edge.frequency_hz,
                    },
                    Reading {
                        mode: Mode::Flatwise,
                        frequency_hz: flat.frequency_hz,
                    },
                ],
            )
            .expect("a made series is solved");
            assert!((solved.youngs_modulus_pa / e - 1.0).abs() < 1.0e-12);
            assert!((solved.shear_modulus_pa / g - 1.0).abs() < 1.0e-12);
            assert!(solved.edgewise_residual_fraction.unwrap().abs() < 1.0e-12);
        }
    }

    #[test]
    fn a_series_that_cannot_fix_a_modulus_is_refused_by_name() {
        let scan = &published_scans()[0];
        let all = series(scan);
        assert_eq!(
            solve(scan.bar, &[all[0], all[1]]),
            Err(ModulusReject::ShearModulusNotIdentifiable)
        );
        assert_eq!(
            solve(scan.bar, &[all[1], all[2]]),
            Err(ModulusReject::YoungsModulusNotIdentifiable)
        );
        assert_eq!(
            solve(scan.bar, &[]),
            Err(ModulusReject::YoungsModulusNotIdentifiable)
        );
        assert_eq!(
            solve(scan.bar, &[all[0], all[2], all[0]]),
            Err(ModulusReject::RepeatedMode {
                mode: Mode::Flatwise
            })
        );
    }

    #[test]
    fn a_frequency_outside_its_declared_range_is_refused_and_not_moved_to_the_limit() {
        let scan = &published_scans()[0];
        for bad in [
            f64::NAN,
            f64::INFINITY,
            -1588.627,
            0.0,
            FLATWISE_RANGE_HZ.0 - 1.0e-9,
            FLATWISE_RANGE_HZ.1 + 1.0e-9,
        ] {
            match solve(scan.bar, &two(bad, scan.torsion_hz)) {
                Err(ModulusReject::FrequencyOutOfRange {
                    mode: Mode::Flatwise,
                    frequency_hz,
                }) => assert!(frequency_hz.to_bits() == bad.to_bits()),
                other => panic!("{bad} gave {other:?}"),
            }
        }
        // The limits themselves are inside, and the answer at a limit is the
        // answer for that frequency: the low end is ERX1's single scan.
        let lowest = &published_scans()[1];
        let at_limit = solve(lowest.bar, &two(FLATWISE_RANGE_HZ.0, lowest.torsion_hz))
            .expect("the low end of the flatwise range is answered");
        let just_inside = solve(lowest.bar, &two(FLATWISE_RANGE_HZ.0 + 1.0e-3, lowest.torsion_hz))
            .expect("just inside is answered");
        assert!(at_limit.youngs_modulus_pa < just_inside.youngs_modulus_pa);
        assert_eq!(
            at_limit.youngs_modulus_pa.to_bits(),
            solve(lowest.bar, &series(lowest)).unwrap().youngs_modulus_pa.to_bits()
        );
        for (mode, range) in [
            (Mode::Edgewise, EDGEWISE_RANGE_HZ),
            (Mode::Torsion, TORSION_RANGE_HZ),
        ] {
            for bad in [range.0 - 1.0e-9, range.1 + 1.0e-9] {
                let mut moved = series(scan);
                for reading in &mut moved {
                    if reading.mode == mode {
                        reading.frequency_hz = bad;
                    }
                }
                match solve(scan.bar, &moved) {
                    Err(ModulusReject::FrequencyOutOfRange {
                        mode: refused,
                        frequency_hz,
                    }) => {
                        assert_eq!(refused, mode);
                        assert_eq!(frequency_hz.to_bits(), bad.to_bits());
                    }
                    other => panic!("{bad} gave {other:?}"),
                }
            }
        }
        match solve(
            scan.bar,
            &[
                Reading {
                    mode: Mode::Torsion,
                    frequency_hz: TORSION_RANGE_HZ.1 * 1.01,
                },
                Reading {
                    mode: Mode::Flatwise,
                    frequency_hz: scan.flatwise_hz,
                },
            ],
        ) {
            Err(ModulusReject::FrequencyOutOfRange {
                mode: Mode::Torsion,
                ..
            }) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn frequencies_no_isotropic_bar_has_are_refused() {
        let scan = &published_scans()[0];
        // Far outside the declared ranges, so asked of the recovery itself: a
        // high flatwise pitch with a low torsion pitch, and the other way round.
        for (flatwise, torsion, above) in [(1990.0, 6800.0, true), (1250.0, 11000.0, false)] {
            let run = recover(
                scan.bar,
                flatwise,
                torsion,
                None,
                (START_YOUNGS_MODULUS_PA, START_SHEAR_MODULUS_PA),
                MAX_ITERATIONS,
                Fault::None,
            );
            match run {
                Err(ModulusReject::PoissonRatioOutOfRange { poisson_ratio }) => {
                    assert_eq!(poisson_ratio > 0.5, above, "{poisson_ratio}");
                    assert_eq!(poisson_ratio < 0.0, !above, "{poisson_ratio}");
                }
                other => panic!("{other:?}"),
            }
            // The public solver refuses the same pair at its declared range.
            assert!(matches!(
                solve(scan.bar, &two(flatwise, torsion)),
                Err(ModulusReject::FrequencyOutOfRange { .. })
            ));
        }
    }

    #[test]
    fn an_edgewise_frequency_that_disagrees_is_refused() {
        let scan = &published_scans()[0];
        let mut moved = series(scan);
        moved[1].frequency_hz *= 1.003;
        match solve(scan.bar, &moved) {
            Err(ModulusReject::ModesInconsistent { residual_fraction }) => {
                assert!(residual_fraction < -EDGEWISE_LIMIT_FRACTION, "{residual_fraction}")
            }
            other => panic!("{other:?}"),
        }
        // Without the edgewise frequency the same two frequencies are answered.
        let solved = solve(scan.bar, &two(scan.flatwise_hz, scan.torsion_hz)).expect("solved");
        assert_eq!(solved.edgewise_residual_fraction, None);
    }

    #[test]
    fn a_model_without_shear_misses_the_edgewise_frequency_and_is_refused() {
        for scan in published_scans() {
            match faulted(&scan, Fault::NoShear) {
                Err(ModulusReject::ModesInconsistent { residual_fraction }) => {
                    assert!(residual_fraction > 0.02, "{residual_fraction}")
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn a_root_that_is_not_a_root_breaks_the_free_end_on_the_first_iteration() {
        let scan = &published_scans()[0];
        match faulted(scan, Fault::ShiftedRoot(1.0e-6)) {
            Err(ModulusReject::FreeEndBroken {
                mode: Mode::Flatwise,
                residual,
            }) => assert!(residual > FREE_END_TOLERANCE, "{residual}"),
            Err(ModulusReject::EnergyBalanceBroken {
                mode: Mode::Flatwise,
                residual,
            }) => assert!(residual.abs() > ENERGY_BALANCE_TOLERANCE, "{residual}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_mode_shape_that_is_not_the_mode_breaks_the_energy_balance() {
        let scan = &published_scans()[0];
        match faulted(scan, Fault::ScaledDeflection(1.0e-6)) {
            Err(ModulusReject::EnergyBalanceBroken {
                mode: Mode::Flatwise,
                residual,
            }) => assert!(residual.abs() > ENERGY_BALANCE_TOLERANCE, "{residual}"),
            other => panic!("{other:?}"),
        }
        // A scaling a hundred times smaller than the tolerance can see is not refused.
        assert!(faulted(scan, Fault::ScaledDeflection(1.0e-11)).is_ok());
    }

    #[test]
    fn a_model_that_does_not_return_the_fitted_frequency_is_refused() {
        let scan = &published_scans()[0];
        match faulted(scan, Fault::DriftedTorsion(1.0e-8)) {
            Err(ModulusReject::RoundTripBroken {
                mode: Mode::Torsion,
                residual,
            }) => assert!(residual > ROUND_TRIP_TOLERANCE, "{residual}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn too_few_iterations_are_refused() {
        let scan = &published_scans()[0];
        for cap in [0, 1, 2] {
            let run = recover(
                scan.bar,
                scan.flatwise_hz,
                scan.torsion_hz,
                None,
                (START_YOUNGS_MODULUS_PA, START_SHEAR_MODULUS_PA),
                cap,
                Fault::None,
            );
            assert_eq!(run, Err(ModulusReject::NotConverged { iterations: cap }));
        }
    }

    #[test]
    fn a_trial_point_outside_the_forward_model_or_the_search_range_is_refused() {
        let scan = &published_scans()[0];
        // A bar that would bend almost wholly in shear: the root leaves the bracket.
        let run = recover(
            scan.bar,
            scan.flatwise_hz,
            scan.torsion_hz,
            None,
            (2.0e12, 0.3e9),
            MAX_ITERATIONS,
            Fault::None,
        );
        match run {
            Err(ModulusReject::FrequencyEquationNotBracketed {
                mode: Mode::Flatwise,
                youngs_modulus_pa,
                shear_modulus_pa,
            }) => {
                assert!((youngs_modulus_pa / 2.0e12 - 1.0).abs() < 1.0e-12);
                assert!((shear_modulus_pa / 0.3e9 - 1.0).abs() < 1.0e-12);
            }
            other => panic!("{other:?}"),
        }
        // A pitch four times too high asks for E past the search range. It is not shortened to the limit.
        let run = recover(
            scan.bar,
            4.0 * scan.flatwise_hz,
            scan.torsion_hz,
            None,
            (START_YOUNGS_MODULUS_PA, START_SHEAR_MODULUS_PA),
            MAX_ITERATIONS,
            Fault::None,
        );
        match run {
            Err(ModulusReject::OutsideSearchRange {
                youngs_modulus_pa, ..
            }) => assert!(youngs_modulus_pa > YOUNGS_MODULUS_SEARCH_PA.1),
            other => panic!("{other:?}"),
        }
    }

    /// Line P4 of the protocol: 25 starts, `E` 20 to 2000 GPa and `G` 5 to
    /// 800 GPa on a logarithmic grid, all to the same moduli within 1e-9.
    #[test]
    fn twenty_five_starts_reach_the_same_moduli() {
        for scan in published_scans() {
            let home = solve(scan.bar, &series(&scan)).expect("solved");
            let mut most = 0;
            for i in 0..5 {
                for j in 0..5 {
                    let e = 20.0e9 * 100.0_f64.powf(i as f64 / 4.0);
                    let g = 5.0e9 * 160.0_f64.powf(j as f64 / 4.0);
                    let run = recover(
                        scan.bar,
                        scan.flatwise_hz,
                        scan.torsion_hz,
                        Some(scan.edgewise_hz),
                        (e, g),
                        MAX_ITERATIONS,
                        Fault::None,
                    )
                    .unwrap_or_else(|reject| panic!("start {e} {g}: {reject}"));
                    assert!((run.youngs_modulus_pa / home.youngs_modulus_pa - 1.0).abs() < 1.0e-9);
                    assert!((run.shear_modulus_pa / home.shear_modulus_pa - 1.0).abs() < 1.0e-9);
                    most = most.max(run.iterations);
                }
            }
            assert!(most <= 8, "{most} iterations");
        }
    }

    #[test]
    fn the_same_series_twice_is_bit_identical() {
        let scan = &published_scans()[2];
        let first = solve(scan.bar, &series(scan)).expect("solved");
        let second = solve(scan.bar, &series(scan)).expect("solved");
        assert_eq!(first.iterations, second.iterations);
        assert_eq!(
            first.youngs_modulus_pa.to_bits(),
            second.youngs_modulus_pa.to_bits()
        );
        assert_eq!(
            first.shear_modulus_pa.to_bits(),
            second.shear_modulus_pa.to_bits()
        );
        assert_eq!(first.mode_shapes.len(), 2 * 4 * (MODE_INTERVALS + 1));
        for (left, right) in first.mode_shapes.iter().zip(&second.mode_shapes) {
            assert_eq!(left.to_bits(), right.to_bits());
        }
        // The order of the series does not change the answer.
        let mut turned = series(scan);
        turned.reverse();
        let third = solve(scan.bar, &turned).expect("solved");
        assert_eq!(
            first.youngs_modulus_pa.to_bits(),
            third.youngs_modulus_pa.to_bits()
        );
    }

    #[test]
    fn the_returned_mode_shapes_hold_the_returned_energy_balance() {
        let scan = &published_scans()[1];
        let solved = solve(scan.bar, &series(scan)).expect("solved");
        let c = scan.bar.constants();
        let half = 4 * (MODE_INTERVALS + 1);
        for (mode, samples, balance, end) in [
            (
                Mode::Flatwise,
                &solved.mode_shapes[..half],
                solved.flatwise_energy_balance,
                solved.flatwise_free_end,
            ),
            (
                Mode::Edgewise,
                &solved.mode_shapes[half..],
                solved.edgewise_energy_balance,
                solved.edgewise_free_end,
            ),
        ] {
            let root = bending_root(
                &c,
                mode,
                solved.youngs_modulus_pa,
                solved.shear_modulus_pa,
                Fault::None,
            )
            .unwrap();
            let (again, end_again) = mode_checks(samples, root.lam, root.r2, root.s2);
            assert_eq!(again.to_bits(), balance.to_bits());
            assert_eq!(end_again.to_bits(), end.to_bits());
            assert!(balance.abs() <= ENERGY_BALANCE_TOLERANCE);
            assert!(end <= FREE_END_TOLERANCE);
            // The mode starts free and is not the bar at rest.
            assert_eq!(samples[2], 0.0);
            assert_eq!(samples[3], 0.0);
            assert!(samples[0].hypot(samples[1]) > 0.5);
        }
    }

    #[test]
    fn every_bar_has_a_name_and_its_own_constants() {
        for bar in BARS {
            assert_eq!(Bar::from_name(bar.name()), Some(bar));
            let c = bar.constants();
            let density = c.mass_kg / (c.length_m * c.width_m * c.thickness_m);
            assert!((7800.0..7900.0).contains(&density), "{density}");
        }
        assert_eq!(Bar::from_name("erx1"), None);
        assert_eq!(Bar::from_name("ERX3"), None);
        assert_eq!(Mode::from_name("flatwise"), Some(Mode::Flatwise));
        assert_eq!(Mode::from_name("Flatwise"), None);
    }
}
