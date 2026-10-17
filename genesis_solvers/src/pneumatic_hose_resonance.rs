//! The resonance of a pneumatic hose feeding a closed volume: the company's
//! own specimen, 2.0 m of polyamide 12 tube of 6 mm by 4 mm with a 20 cm^3
//! closed volume at its end, air at 293.15 K about a mean pressure the
//! caller asks, driven by a small pressure at the hose's inlet.
//!
//! The line. Small oscillations of air at rest in a round tube, laminar,
//! with the full radial profile of velocity and temperature: the
//! dissipative model of Nichols (1962) and Brown (1962). Per length, the
//! series impedance (pressure gradient over volume flow) and the shunt
//! admittance (volume flow lost per length over pressure) are
//!
//! ```text
//! Z(s) = rho s / (A (1 - F(r sqrt(-s/nu))))
//! Y(s) = A s / (gamma p) [1 + (gamma - 1) F(r sqrt(-s Pr/nu))] + A s D psi / (E e)
//! F(z) = 2 J1(z) / (z J0(z))
//! ```
//!
//! so that `Z` tends to Poiseuille's `8 mu / (pi r^4)` and `Y` to the
//! isothermal `A s / p` at low frequency, and to the inviscid, adiabatic
//! line at high. The last term is the tube's stretch (thick-walled, `psi =
//! 2 (e/D)(1 + nu) + D (1 - nu^2)/(D + e)`, polyamide 12 at 1.4 GPa and
//! Poisson 0.4), 0.5 percent of the air's compliance at 1.1 MPa. The ratio
//! `J1/J0` is a continued fraction, run down from well above `|z|`, for any
//! complex `z`, with `J2/J1` beside it, so that `1 - F = -J2/J0` is taken
//! with no difference of near-equal numbers and `Z` is Poiseuille's at low
//! frequency to the last bit. Then `Gamma = sqrt(Z Y)`, and with the end
//! volume's admittance `Y_v` the pressure in the volume over the pressure at
//! the inlet is
//!
//! ```text
//! G(s) = 1 / (cosh(Gamma L) + Z Y_v L sinh(Gamma L) / (Gamma L))
//! Y_v(s) = V s / (gamma p) [1 + (gamma - 1) F_s(a sqrt(s Pr/nu))],  F_s(x) = 3 (x coth x - 1) / x^2
//! ```
//!
//! the volume taken as a sphere of the same volume, radius 16.84 mm, whose
//! wall stays at the mean temperature: at low frequency the air in it is
//! isothermal, at high adiabatic, with the heat exchanged in between. `G`
//! is `1/(cosh(Gamma L) + Z_c Y_v sinh(Gamma L))` with `Z_c = Z/Gamma`,
//! written so that it is even in `Gamma` and needs no division by it. On the
//! frequency axis the flows and the powers are taken over `omega`, so that
//! every term keeps its own size at any frequency asked, down to the
//! smallest 64-bit number.
//!
//! What it returns. The line's natural modes: the poles of `G`, each found
//! by Newton's method in the complex plane and followed from the tube's
//! rating down to the asked pressure, starting from the lossless line's
//! frequency (`k L tan(k L) = V_line / V`), each with its frequency and
//! damping ratio. The following steps in the logarithm of the pressure, at
//! most ln 1.25 at a time; each step foresees the pole from the last two and
//! is taken when the settled pole lies within 2 percent of its modulus of
//! the foreseen, and is otherwise halved, so that a heavily damped mode does
//! not jump to another mode's pole; two ringing modes on one pole are
//! refused (`ModesOnOnePole`). A mode overdamped at the asked pressure does
//! not ring and is left out: the first stops ringing at 8,625 Pa, the sixth
//! at 604.7 Pa, and below that none rings. The peaks of `|G(j omega)|`,
//! found on a scan and refined, with their gain; and `G` at the frequencies
//! the caller asks.
//!
//! The ledgers, at every asked frequency and every peak. For an inlet
//! pressure of 1 Pa, the mean power that goes in at the inlet equals the
//! power the line dissipates, `(1/2) integral of Re(Z)|Q|^2 + Re(Y)|P|^2 dx`
//! by Gauss-Legendre quadrature along the standing wave, plus the power
//! the volume takes, `(1/2) Re(Y_v)|P_v|^2`; and the volume flow in at the
//! inlet less the flow into the volume equals the flow the line's air takes
//! up, `integral of Y P dx`. Each is held to 1e-9. The second law is held
//! at the same frequencies: `Re Z`, `Re Y` and `Re Y_v` are never negative
//! (`PassivityBroken`).
//!
//! The real limits. The model is of plane waves: above the cut-on of the
//! first mode across the bore, `1.8412 c / (pi D)`, 50,289 Hz here, sound
//! also travels across the tube, and a frequency at or above it is refused
//! (`AboveFirstCrossMode`). The model's wall holds the air with no slip:
//! slip flow begins where the mean free path reaches a hundredth (Knudsen
//! number 0.01) of the length the flow is sheared over. Over the bore that
//! is at 170.01 Pa, and a mean pressure below it is refused
//! (`RarefiedFlow`); over the oscillating viscous layer at the wall, the
//! Stokes layer `sqrt(2 nu / omega)`, thinner than the bore at all but the
//! lowest frequencies, it is at `omega = 2 nu Kn^2 / lambda^2`, that is `f =
//! 2 pi mu R Kn^2 d^4 p / (k_B^2 T)`, 1.0544 Hz for each pascal of mean
//! pressure, and an asked frequency above it is refused
//! (`StokesLayerSlip`); below 47,696 Pa it is the lower of the two limits,
//! and the scan for the peaks stops there too. The tube is rated to 1.0 MPa
//! over the atmosphere (`PressureAboveTubeRating`).
//! The oscillation is small: the answer is per pascal of inlet pressure,
//! and the model holds while the air's own velocity is a small part of the
//! speed of sound and its oscillating boundary layer stays laminar.
//!
//! What is left out. A mean flow: the air is at rest about the mean
//! pressure. The tube's viscoelastic loss. The fitting at the volume.
//!
//! What was compared. This specimen's own comparison comes later. A test
//! holds the model to Kirchhoff's (1868) closed form for the attenuation of
//! sound in a wide tube, `alpha = (1/(r c)) sqrt(nu omega / 2) (1 + (gamma -
//! 1)/sqrt(Pr))`, and to Poiseuille's resistance at low frequency:
//! <https://doi.org/10.1002/andp.18682100602>, Retrieved 2026-10-14.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::fmt;

/// The published closed form the tests hold the line to.
pub const REFERENCE_NAME: &str = "G. Kirchhoff, Ueber den Einfluss der Wärmeleitung in einem Gase auf die Schallbewegung, Annalen der Physik 210 (1868) 177-193: the attenuation of sound in a wide tube by viscosity and heat conduction";
pub const REFERENCE_URL: &str = "https://doi.org/10.1002/andp.18682100602";
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";

/// Length of the hose, m.
pub const HOSE_LENGTH_M: f64 = 2.0;
/// Bore of the hose, m.
pub const HOSE_BORE_M: f64 = 0.004;
/// Wall thickness of the hose, m.
pub const HOSE_WALL_M: f64 = 0.001;
/// Young's modulus of the polyamide 12 wall, Pa.
pub const WALL_YOUNGS_MODULUS_PA: f64 = 1.4e9;
/// Poisson's ratio of the wall.
pub const WALL_POISSON_RATIO: f64 = 0.4;
/// Volume closed at the hose's end, m^3.
pub const END_VOLUME_M3: f64 = 20.0e-6;
/// Temperature of the air and the walls, K.
pub const TEMPERATURE_K: f64 = 293.15;
/// Gas constant of dry air, J/(kg K).
pub const AIR_GAS_CONSTANT_J_KG_K: f64 = 287.05;
/// Ratio of the specific heats of air.
pub const AIR_GAMMA: f64 = 1.4;
/// Dynamic viscosity of air at 293.15 K, Pa s.
pub const AIR_VISCOSITY_PA_S: f64 = 1.8205e-5;
/// Prandtl number of air at 293.15 K.
pub const AIR_PRANDTL: f64 = 0.7130;
/// Kinetic diameter of an air molecule, m, for the mean free path.
pub const AIR_MOLECULE_DIAMETER_M: f64 = 3.66e-10;
/// Boltzmann's constant, J/K.
pub const BOLTZMANN_J_K: f64 = 1.380_649e-23;
/// Pressure of the atmosphere outside, Pa.
pub const ATMOSPHERE_PA: f64 = 101_325.0;
/// The tube's rated pressure over the atmosphere, Pa.
pub const TUBE_RATING_GAUGE_PA: f64 = 1.0e6;
/// Knudsen number, the mean free path over a length of the flow, at which
/// slip flow begins at the wall: over the bore, and over the oscillating
/// viscous layer at the wall.
pub const KNUDSEN_LIMIT: f64 = 0.01;
/// The lowest mean pressure taken, Pa: [`rarefied_below_pa`] itself, where
/// the mean free path is a hundredth of the bore. A test holds the two equal
/// to the bit.
pub const PRESSURE_MIN_PA: f64 = 170.014_572_811_190_65;
/// The cut-on of the first mode across the bore, Hz: [`first_cross_mode_hz`]
/// itself. An asked frequency must lie below it; at it and above it is
/// refused. A test holds the two equal to the bit.
pub const FREQUENCY_MAX_HZ: f64 = 50_289.233_855_166_3;
/// The first zero of the derivative of J1: the first mode across a round bore.
pub const FIRST_CROSS_MODE_ZERO: f64 = 1.841_183_781_340_659;
/// The natural modes returned.
pub const MODES: usize = 6;
/// The most frequencies a question may ask.
pub const FREQUENCIES_MAX: usize = 64;
/// Points of the scan for the peaks, logarithmic, from [`SCAN_LOW_HZ`] to the
/// lower of 1.25 times the sixth lossless mode and [`stokes_slip_hz`] at the
/// asked pressure.
pub const SCAN_POINTS: usize = 4000;
/// Lowest frequency of the scan, Hz.
pub const SCAN_LOW_HZ: f64 = 1.0;
/// The ledgers' residual, held at every asked frequency and peak.
pub const LEDGER_RESIDUAL_LIMIT: f64 = 1.0e-9;
/// Newton iterations allowed for one pole.
pub const MAX_POLE_ITERATIONS: u32 = 100;
/// A pole whose imaginary part is under this share of its modulus has
/// stopped ringing: its damping ratio is above 0.999 999 99.
pub const OVERDAMPED_IMAGINARY_SHARE: f64 = 1.0e-4;
/// A pole is settled when a Newton step moves it by this fraction of its modulus.
pub const POLE_SETTLED: f64 = 1.0e-13;
/// The largest step, in the logarithm of the mean pressure, by which a pole
/// is followed from the tube's rating to the asked pressure: ln 1.25.
pub const TRACK_STEP_MAX: f64 = 0.223_143_551_314_209_76;
/// A step of the following is taken when its settled pole lies within this
/// share of its modulus of the pole foreseen from the last two; otherwise
/// the step is halved and taken again.
pub const TRACK_SHARE: f64 = 0.02;
/// The smallest step of the following, in the logarithm of the pressure.
pub const TRACK_STEP_MIN: f64 = 1.0e-9;
/// Two modes whose poles lie within this share of their modulus are one pole.
pub const DISTINCT_POLE_SHARE: f64 = 1.0e-6;

/// A complex number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    pub fn abs(self) -> f64 {
        math::hypot(self.re, self.im)
    }
    pub fn arg(self) -> f64 {
        math::atan2(self.im, self.re)
    }
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
    pub fn scale(self, k: f64) -> Self {
        Self::new(self.re * k, self.im * k)
    }
    pub fn inv(self) -> Self {
        // Smith's division, so that a large or small modulus does not overflow.
        if self.re.abs() >= self.im.abs() {
            let r = self.im / self.re;
            let d = self.re + self.im * r;
            Self::new(1.0 / d, -r / d)
        } else {
            let r = self.re / self.im;
            let d = self.re * r + self.im;
            Self::new(r / d, -1.0 / d)
        }
    }
    pub fn sqrt(self) -> Self {
        // The root with a real part that is not negative.
        let m = self.abs();
        if m == 0.0 {
            return Self::new(0.0, 0.0);
        }
        let re = (0.5 * (m + self.re.abs())).sqrt();
        if self.re >= 0.0 {
            Self::new(re, self.im / (2.0 * re))
        } else {
            Self::new(self.im.abs() / (2.0 * re), if self.im >= 0.0 { re } else { -re })
        }
    }
    pub fn exp(self) -> Self {
        let m = math::exp(self.re);
        Self::new(m * math::cos(self.im), m * math::sin(self.im))
    }
    pub fn cosh(self) -> Self {
        Self::new(math::cosh(self.re) * math::cos(self.im), math::sinh(self.re) * math::sin(self.im))
    }
    pub fn sinh(self) -> Self {
        Self::new(math::sinh(self.re) * math::cos(self.im), math::cosh(self.re) * math::sin(self.im))
    }
}

impl std::ops::Add for Complex {
    type Output = Complex;
    fn add(self, o: Complex) -> Complex {
        Complex::new(self.re + o.re, self.im + o.im)
    }
}
impl std::ops::Sub for Complex {
    type Output = Complex;
    fn sub(self, o: Complex) -> Complex {
        Complex::new(self.re - o.re, self.im - o.im)
    }
}
impl std::ops::Mul for Complex {
    type Output = Complex;
    fn mul(self, o: Complex) -> Complex {
        Complex::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
}
impl std::ops::Div for Complex {
    type Output = Complex;
    fn div(self, o: Complex) -> Complex {
        self * o.inv()
    }
}

const ONE: Complex = Complex::new(1.0, 0.0);

/// A number as a sentence shows it: the shortest decimal that reads back to
/// the same 64-bit number, with an exponent when it is under a millionth or
/// past 1e21 in size, so that no sentence prints hundreds of digits.
pub fn shown(value: f64) -> String {
    let size = value.abs();
    if size == 0.0 || (1.0e-6..1.0e21).contains(&size) {
        format!("{value}")
    } else {
        format!("{value:e}")
    }
}

/// Below this modulus of `z` the Bessel ratios are taken from their series.
const SERIES_BELOW: f64 = 1.0e-4;

/// The ratios `J1(z)/J0(z)` and `J2(z)/J1(z)` for any complex `z` at or
/// above [`SERIES_BELOW`], as the continued fraction `r_n = J_n/J_{n-1} =
/// 1 / (2n/z - r_{n+1})`, run down from `n` well above `|z|`, where it has
/// settled.
fn bessel_ratios(z: Complex) -> (Complex, Complex) {
    let top = (2.0 * z.abs()) as usize + 60;
    let mut r = Complex::new(0.0, 0.0);
    let mut second = r;
    let inv_z = z.inv();
    for n in (1..=top).rev() {
        if n == 1 {
            second = r;
        }
        r = (inv_z.scale(2.0 * n as f64) - r).inv();
    }
    (r, second)
}

/// `F(z) = 2 J1(z) / (z J0(z))` for any complex `z`.
pub fn bessel_ratio_f(z: Complex) -> Complex {
    if z.abs() < SERIES_BELOW {
        // 1 + z^2/8 + z^4/48, the series, where the fraction would divide by
        // a small z.
        let z2 = z * z;
        return ONE + z2.scale(1.0 / 8.0) + (z2 * z2).scale(1.0 / 48.0);
    }
    let (r1, _) = bessel_ratios(z);
    r1.scale(2.0) * z.inv()
}

/// `(1 - F(z)) / (-z^2/8)`: since `1 - F = -J2/J0`, the product of the two
/// ratios over `-z^2/8`, with no difference of near-equal numbers; its
/// series `1 + z^2/6 + 11 z^4/384` below [`SERIES_BELOW`]. It tends to 1 at
/// low frequency, where the viscous impedance is Poiseuille's.
fn viscous_share(z: Complex) -> Complex {
    let z2 = z * z;
    if z.abs() < SERIES_BELOW {
        return ONE + z2.scale(1.0 / 6.0) + (z2 * z2).scale(11.0 / 384.0);
    }
    let (r1, r2) = bessel_ratios(z);
    (r1 * r2).scale(8.0) / z2
}

/// `F_s(x) = 3 (x coth x - 1) / x^2`, the mean of the temperature in a
/// sphere with an isothermal wall, as a share of the adiabatic.
pub fn sphere_f(x: Complex) -> Complex {
    if x.abs() < 1.0e-3 {
        // 1 - x^2/15 + 2 x^4/315.
        let x2 = x * x;
        return ONE - x2.scale(1.0 / 15.0) + (x2 * x2).scale(2.0 / 315.0);
    }
    // coth x = (1 + e^{-2x}) / (1 - e^{-2x}) for Re x >= 0, which every
    // argument here has.
    let e = Complex::exp(x.scale(-2.0));
    let coth = (ONE + e) / (ONE - e);
    (x * coth - ONE).scale(3.0) / (x * x)
}

/// `sinh(u)/u`, its series `1 + u^2/6 + u^4/120` where `u` is small.
fn sinh_over(u: Complex) -> Complex {
    if u.abs() < 1.0e-3 {
        let u2 = u * u;
        return ONE + u2.scale(1.0 / 6.0) + (u2 * u2).scale(1.0 / 120.0);
    }
    Complex::sinh(u) / u
}

fn area() -> f64 {
    0.25 * std::f64::consts::PI * HOSE_BORE_M * HOSE_BORE_M
}

/// Speed of sound of air at the temperature, m/s.
pub fn sound_speed_m_s() -> f64 {
    (AIR_GAMMA * AIR_GAS_CONSTANT_J_KG_K * TEMPERATURE_K).sqrt()
}

/// The cut-on of the first mode across the bore, Hz.
pub fn first_cross_mode_hz() -> f64 {
    FIRST_CROSS_MODE_ZERO * sound_speed_m_s() / (std::f64::consts::PI * HOSE_BORE_M)
}

/// The mean free path of air at a pressure, m.
pub fn mean_free_path_m(pressure_pa: f64) -> f64 {
    BOLTZMANN_J_K * TEMPERATURE_K / (2f64.sqrt() * std::f64::consts::PI * AIR_MOLECULE_DIAMETER_M * AIR_MOLECULE_DIAMETER_M * pressure_pa)
}

/// The mean pressure at which the mean free path is a hundredth of the bore,
/// Knudsen number [`KNUDSEN_LIMIT`]: below it slip flow begins at the wall, Pa.
pub fn rarefied_below_pa() -> f64 {
    BOLTZMANN_J_K * TEMPERATURE_K / (2f64.sqrt() * std::f64::consts::PI * AIR_MOLECULE_DIAMETER_M * AIR_MOLECULE_DIAMETER_M * KNUDSEN_LIMIT * HOSE_BORE_M)
}

/// The frequency at which the oscillating viscous layer at the wall, the
/// Stokes layer `sqrt(2 nu / omega)`, is [`KNUDSEN_LIMIT`] mean free paths'
/// worth over a hundred, so that slip flow begins in it: `omega = 2 nu Kn^2 /
/// lambda^2`, Hz. With `nu = mu R T / p` and `lambda` falling as `1/p` it is
/// `2 pi mu R Kn^2 d^4 p / (k_B^2 T)`: 1.0544 Hz for each pascal of mean
/// pressure, 179 Hz at the lowest pressure taken, and the cut-on of the
/// first mode across the bore at 47,696 Pa.
pub fn stokes_slip_hz(pressure_pa: f64) -> f64 {
    let lambda = mean_free_path_m(pressure_pa);
    let nu = AIR_VISCOSITY_PA_S * AIR_GAS_CONSTANT_J_KG_K * TEMPERATURE_K / pressure_pa;
    2.0 * nu * KNUDSEN_LIMIT * KNUDSEN_LIMIT / (lambda * lambda) / (2.0 * std::f64::consts::PI)
}

/// The highest mean pressure the tube is rated to, absolute, Pa.
pub fn tube_rating_pa() -> f64 {
    ATMOSPHERE_PA + TUBE_RATING_GAUGE_PA
}

/// The wall's compliance per pascal, `D psi / (E e)`.
pub fn wall_compliance_per_pa() -> f64 {
    let (d, e, nu) = (HOSE_BORE_M, HOSE_WALL_M, WALL_POISSON_RATIO);
    let psi = 2.0 * (e / d) * (1.0 + nu) + d * (1.0 - nu * nu) / (d + e);
    d * psi / (WALL_YOUNGS_MODULUS_PA * e)
}

/// Radius of the sphere of the end volume's volume, m.
pub fn end_sphere_radius_m() -> f64 {
    math::cbrt(3.0 * END_VOLUME_M3 / (4.0 * std::f64::consts::PI))
}

/// A wrong line a test may ask for. Every served answer passes `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tamper {
    None,
    /// The heat exchanged with the wall is given the wrong sign.
    ThermalSignFlipped,
    /// The ledger's flow taken up along the line leaves out the wall.
    LedgerWithoutWall,
    /// The energy ledger's dissipation leaves out the line's thermal loss.
    LedgerWithoutHeat,
}

/// The line and its load at one mean pressure.
#[derive(Clone, Copy, Debug)]
struct Line {
    pressure: f64,
    nu: f64,
    tamper: Tamper,
}

impl Line {
    fn at(pressure: f64) -> Self {
        let density = pressure / (AIR_GAS_CONSTANT_J_KG_K * TEMPERATURE_K);
        Self { pressure, nu: AIR_VISCOSITY_PA_S / density, tamper: Tamper::None }
    }

    /// Series impedance and shunt admittance per length, and the volume's
    /// admittance, at `s`, the admittances taken with `unit` in place of `s`
    /// as their factor: `s` itself, or `s/|s|` for them over `|s|`, which
    /// stay of their own size at any frequency.
    fn parts_with(&self, s: Complex, unit: Complex) -> (Complex, Complex, Complex) {
        let r = 0.5 * HOSE_BORE_M;
        let viscous = (s.scale(-1.0 / self.nu)).sqrt().scale(r);
        let thermal = (s.scale(-AIR_PRANDTL / self.nu)).sqrt().scale(r);
        // Z = rho s / (A (1 - F)) = (8 mu / (A r^2)) / ((1 - F)/(-z^2/8)).
        let z = viscous_share(viscous).inv().scale(8.0 * AIR_VISCOSITY_PA_S / (area() * r * r));
        let exchange = if self.tamper == Tamper::ThermalSignFlipped { 1.0 - AIR_GAMMA } else { AIR_GAMMA - 1.0 };
        let gas = unit.scale(area() / (AIR_GAMMA * self.pressure)) * (ONE + bessel_ratio_f(thermal).scale(exchange));
        let y = gas + unit.scale(area() * wall_compliance_per_pa());
        let x = (s.scale(AIR_PRANDTL / self.nu)).sqrt().scale(end_sphere_radius_m());
        let yv = unit.scale(END_VOLUME_M3 / (AIR_GAMMA * self.pressure)) * (ONE + sphere_f(x).scale(AIR_GAMMA - 1.0));
        (z, y, yv)
    }

    /// Series impedance and shunt admittance per length, and the volume's admittance.
    fn parts(&self, s: Complex) -> (Complex, Complex, Complex) {
        self.parts_with(s, s)
    }

    /// The denominator of `G`: zero at a pole. `cosh(Gamma L) + Z_c Y_v
    /// sinh(Gamma L)` written as `cosh(Gamma L) + Z Y_v L sinh(Gamma L) /
    /// (Gamma L)`: both are even in `Gamma`, so it is the same whichever root
    /// `Gamma` is, analytic in `s` off the imaginary axis, where the poles
    /// are, and needs no division by `Gamma`.
    fn denominator(&self, s: Complex) -> Complex {
        let (z, y, yv) = self.parts(s);
        let gl = (z * y).sqrt().scale(HOSE_LENGTH_M);
        Complex::cosh(gl) + z * yv * sinh_over(gl).scale(HOSE_LENGTH_M)
    }

    fn transfer(&self, s: Complex) -> Complex {
        self.denominator(s).inv()
    }
}

/// One question.
#[derive(Clone, Debug, PartialEq)]
pub struct HoseQuestion {
    /// Absolute mean pressure of the air in the hose and the volume.
    pub mean_pressure_pa: f64,
    /// Frequencies at which the response is asked, Hz.
    pub frequencies_hz: Vec<f64>,
}

/// What the hose does.
#[derive(Clone, Debug, PartialEq)]
pub struct HoseResponse {
    pub sound_speed_m_s: f64,
    pub first_cross_mode_hz: f64,
    pub lossless_frequencies_hz: Vec<f64>,
    /// The number of each mode that still rings, from 1.
    pub mode_numbers: Vec<f64>,
    pub natural_frequencies_hz: Vec<f64>,
    pub damping_ratios: Vec<f64>,
    pub peak_frequencies_hz: Vec<f64>,
    pub peak_gains: Vec<f64>,
    pub gains: Vec<f64>,
    pub phases_rad: Vec<f64>,
    pub inlet_power_w: Vec<f64>,
    pub line_dissipation_w: Vec<f64>,
    pub volume_power_w: Vec<f64>,
    pub largest_energy_ledger_residual: f64,
    pub largest_volume_ledger_residual: f64,
    pub frequencies_computed: u64,
    /// The steps of pressure taken in following the poles, every mode's together.
    pub pressure_steps: u64,
    /// The gain at every point of the scan.
    pub field: Vec<f64>,
}

/// Why no response was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum HoseReject {
    /// An input was NaN or infinite.
    InputNotFinite { field: &'static str },
    /// The mean pressure is above the tube's rating, [`TUBE_RATING_GAUGE_PA`] over the atmosphere.
    PressureAboveTubeRating { pressure_pa: f64 },
    /// The mean pressure is below [`PRESSURE_MIN_PA`], where the mean free path is a hundredth of the bore, Knudsen number [`KNUDSEN_LIMIT`], and slip flow begins at the wall: the model's wall holds the air with no slip.
    RarefiedFlow { pressure_pa: f64 },
    /// More frequencies were asked than [`FREQUENCIES_MAX`].
    TooManyFrequencies { count: usize },
    /// An asked frequency is not above zero.
    FrequencyNotPositive { frequency_hz: f64 },
    /// An asked frequency is at or above [`FREQUENCY_MAX_HZ`], the cut-on of the first mode across the bore, where the waves are no longer plane.
    AboveFirstCrossMode { frequency_hz: f64 },
    /// An asked frequency is above [`stokes_slip_hz`] at the mean pressure, 1.0544 Hz for each pascal, where the oscillating viscous layer at the wall is a hundred mean free paths thick and slip flow begins in it: the model's wall holds the air with no slip.
    StokesLayerSlip { frequency_hz: f64, limit_hz: f64 },
    /// A natural mode lies at or above the cut-on of the first mode across the bore.
    ModeAboveFirstCrossMode { mode: usize, frequency_hz: f64 },
    /// A ringing mode's pole, by its modulus over 2 pi, lies above [`stokes_slip_hz`] at the mean pressure. No admitted pressure reaches it: no mode rings below 604.7 Pa, and every ringing pole's modulus lies under 0.53 of the limit.
    ModeAboveStokesLayerSlip { mode: usize, frequency_hz: f64 },
    /// A pole of the response did not settle in [`MAX_POLE_ITERATIONS`] Newton iterations.
    PoleNotSettled { mode: usize },
    /// A mode's pole could not be followed from the tube's rating to the mean pressure: at a step of pressure of [`TRACK_STEP_MIN`] in its logarithm it still left the pole foreseen by more than [`TRACK_SHARE`] of its modulus.
    PoleNotFollowed { mode: usize, pressure_pa: f64 },
    /// Two modes, followed from the tube's rating, settled on one pole, within [`DISTINCT_POLE_SHARE`] of its modulus: the following lost one of them.
    ModesOnOnePole { first: usize, second: usize, frequency_hz: f64 },
    /// A value was NaN or infinite.
    NonFiniteValue { frequency_hz: f64 },
    /// The line or the volume would give energy at a frequency: a resistance or a conductance below zero.
    PassivityBroken { frequency_hz: f64 },
    /// The energy ledger did not close to [`LEDGER_RESIDUAL_LIMIT`] at a frequency.
    EnergyLedgerBroken { frequency_hz: f64, residual: f64 },
    /// The volume ledger did not close to [`LEDGER_RESIDUAL_LIMIT`] at a frequency.
    VolumeLedgerBroken { frequency_hz: f64, residual: f64 },
}

impl fmt::Display for HoseReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotFinite { field } => write!(f, "{field} is not a finite number"),
            Self::PressureAboveTubeRating { pressure_pa } => write!(
                f,
                "mean pressure {} Pa is above the tube's rating, {} Pa absolute",
                shown(*pressure_pa),
                shown(tube_rating_pa())
            ),
            Self::RarefiedFlow { pressure_pa } => write!(
                f,
                "mean pressure {} Pa is below {} Pa, where the mean free path is a hundredth of the bore and slip flow begins at the wall",
                shown(*pressure_pa),
                shown(PRESSURE_MIN_PA)
            ),
            Self::TooManyFrequencies { count } => write!(f, "{count} frequencies were asked, more than {FREQUENCIES_MAX}"),
            Self::FrequencyNotPositive { frequency_hz } => write!(f, "frequency {} Hz is not above 0 Hz", shown(*frequency_hz)),
            Self::AboveFirstCrossMode { frequency_hz } => write!(
                f,
                "frequency {} Hz is not below {} Hz, the cut-on of the first mode across the bore, where the waves are no longer plane",
                shown(*frequency_hz),
                shown(FREQUENCY_MAX_HZ)
            ),
            Self::StokesLayerSlip { frequency_hz, limit_hz } => write!(
                f,
                "frequency {} Hz is above {} Hz, where at this mean pressure the oscillating viscous layer at the wall is a hundred mean free paths thick and slip flow begins in it",
                shown(*frequency_hz),
                shown(*limit_hz)
            ),
            Self::ModeAboveFirstCrossMode { mode, frequency_hz } => write!(
                f,
                "mode {mode} lies at {} Hz, at or above the cut-on of the first mode across the bore, {} Hz",
                shown(*frequency_hz),
                shown(first_cross_mode_hz())
            ),
            Self::ModeAboveStokesLayerSlip { mode, frequency_hz } => write!(
                f,
                "the pole of mode {mode} lies at {} Hz by its modulus, above where slip flow begins in the oscillating viscous layer at this mean pressure",
                shown(*frequency_hz)
            ),
            Self::PoleNotSettled { mode } => write!(f, "the pole of mode {mode} did not settle in {MAX_POLE_ITERATIONS} Newton iterations"),
            Self::PoleNotFollowed { mode, pressure_pa } => write!(
                f,
                "the pole of mode {mode} could not be followed to {} Pa: at the smallest step of pressure it still left the pole foreseen",
                shown(*pressure_pa)
            ),
            Self::ModesOnOnePole { first, second, frequency_hz } => write!(
                f,
                "modes {first} and {second} settled on one pole, at {} Hz: the following lost one of them",
                shown(*frequency_hz)
            ),
            Self::NonFiniteValue { frequency_hz } => write!(f, "at {} Hz a value was not finite", shown(*frequency_hz)),
            Self::PassivityBroken { frequency_hz } => write!(f, "at {} Hz the line or the volume would give energy", shown(*frequency_hz)),
            Self::EnergyLedgerBroken { frequency_hz, residual } => write!(
                f,
                "at {} Hz the energy ledger was off by {}, above {}",
                shown(*frequency_hz),
                shown(*residual),
                shown(LEDGER_RESIDUAL_LIMIT)
            ),
            Self::VolumeLedgerBroken { frequency_hz, residual } => write!(
                f,
                "at {} Hz the volume ledger was off by {}, above {}",
                shown(*frequency_hz),
                shown(*residual),
                shown(LEDGER_RESIDUAL_LIMIT)
            ),
        }
    }
}

/// Every limit the question shows.
pub fn admit(question: &HoseQuestion) -> Result<(), HoseReject> {
    if !question.mean_pressure_pa.is_finite() {
        return Err(HoseReject::InputNotFinite { field: "mean_pressure_pa" });
    }
    if question.mean_pressure_pa > tube_rating_pa() {
        return Err(HoseReject::PressureAboveTubeRating { pressure_pa: question.mean_pressure_pa });
    }
    if question.mean_pressure_pa < PRESSURE_MIN_PA {
        return Err(HoseReject::RarefiedFlow { pressure_pa: question.mean_pressure_pa });
    }
    if question.frequencies_hz.len() > FREQUENCIES_MAX {
        return Err(HoseReject::TooManyFrequencies { count: question.frequencies_hz.len() });
    }
    let slip = stokes_slip_hz(question.mean_pressure_pa);
    for &frequency_hz in &question.frequencies_hz {
        if !frequency_hz.is_finite() {
            return Err(HoseReject::InputNotFinite { field: "frequencies_hz" });
        }
        if frequency_hz <= 0.0 {
            return Err(HoseReject::FrequencyNotPositive { frequency_hz });
        }
        if frequency_hz >= FREQUENCY_MAX_HZ {
            return Err(HoseReject::AboveFirstCrossMode { frequency_hz });
        }
        if frequency_hz > slip {
            return Err(HoseReject::StokesLayerSlip { frequency_hz, limit_hz: slip });
        }
    }
    Ok(())
}

/// The lossless line's frequencies, adiabatic, `k L tan(k L) = V_line / V`:
/// one root in each branch of the tangent.
pub fn lossless_frequencies_hz() -> Vec<f64> {
    let ratio = area() * HOSE_LENGTH_M / END_VOLUME_M3;
    let c = sound_speed_m_s();
    (0..MODES)
        .map(|n| {
            let pi = std::f64::consts::PI;
            let (mut lo, mut hi) = (n as f64 * pi + 1e-12, n as f64 * pi + 0.5 * pi - 1e-12);
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if mid * math::tan(mid) < ratio { lo = mid } else { hi = mid }
            }
            0.5 * (lo + hi) * c / (2.0 * pi * HOSE_LENGTH_M)
        })
        .collect()
}

/// Gauss-Legendre nodes and weights on [-1, 1], eight points.
const GAUSS_8: [(f64, f64); 8] = [
    (-0.960_289_856_497_536_3, 0.101_228_536_290_376_26),
    (-0.796_666_477_413_626_7, 0.222_381_034_453_374_47),
    (-0.525_532_409_916_329_0, 0.313_706_645_877_887_3),
    (-0.183_434_642_495_649_8, 0.362_683_783_378_362_0),
    (0.183_434_642_495_649_8, 0.362_683_783_378_362_0),
    (0.525_532_409_916_329_0, 0.313_706_645_877_887_3),
    (0.796_666_477_413_626_7, 0.222_381_034_453_374_47),
    (0.960_289_856_497_536_3, 0.101_228_536_290_376_26),
];

/// One frequency: gain, phase, the three powers and the two residuals.
struct AtFrequency {
    gain: f64,
    phase: f64,
    inlet: f64,
    line: f64,
    volume: f64,
    energy_residual: f64,
    volume_residual: f64,
}

/// The response and the ledgers at one frequency on the imaginary axis. The
/// flows and the powers are taken over `omega`, the admittances as `Y /
/// omega`, so that every term keeps its own size at any frequency asked; the
/// powers returned are those times `omega`.
fn at_frequency(line: &Line, frequency_hz: f64) -> Result<AtFrequency, HoseReject> {
    let omega = 2.0 * std::f64::consts::PI * frequency_hz;
    let s = Complex::new(0.0, omega);
    let (z, y, yv) = line.parts_with(s, Complex::new(0.0, 1.0));
    if z.re < 0.0 || y.re < 0.0 || yv.re < 0.0 {
        return Err(HoseReject::PassivityBroken { frequency_hz });
    }
    // Gamma^2 = Z Y = omega Z (Y/omega).
    let gamma = (z * y).scale(omega).sqrt();
    // The volume's pressure and flow over omega for 1 Pa at the inlet, and
    // the wave along the line from them: P = P_v cosh + Z Q_v l sinh(Gl)/(Gl),
    // Q = Q_v cosh + Y P_v l sinh(Gl)/(Gl).
    let wave_from = |pv: Complex, qv: Complex, length: f64| -> (Complex, Complex) {
        let arg = gamma.scale(length);
        let (c, shc) = (Complex::cosh(arg), sinh_over(arg).scale(length));
        (pv * c + (z * qv * shc).scale(omega), qv * c + y * pv * shc)
    };
    let (unit_in, _) = wave_from(ONE, yv, HOSE_LENGTH_M);
    let g = unit_in.inv();
    let pv = g;
    let qv = yv * pv;
    let wave_at = |x: f64| wave_from(pv, qv, HOSE_LENGTH_M - x);
    let (p_in, q_in) = wave_at(0.0);
    let inlet = 0.5 * (p_in * q_in.conj()).re;
    let volume = 0.5 * yv.re * pv.abs() * pv.abs();
    let segments = ((4.0 * gamma.abs() * HOSE_LENGTH_M / std::f64::consts::PI).ceil() as usize).max(32);
    let h = HOSE_LENGTH_M / segments as f64;
    let mut dissipated = 0.0;
    let mut taken = Complex::new(0.0, 0.0);
    let y_ledger = if line.tamper == Tamper::LedgerWithoutWall { y - Complex::new(0.0, area() * wall_compliance_per_pa()) } else { y };
    let y_heat = if line.tamper == Tamper::LedgerWithoutHeat { 0.0 } else { y.re };
    for k in 0..segments {
        let mid = (k as f64 + 0.5) * h;
        for (node, weight) in GAUSS_8 {
            let x = mid + 0.5 * h * node;
            let (p, q) = wave_at(x);
            let w = 0.5 * h * weight;
            dissipated += w * 0.5 * (omega * z.re * q.abs() * q.abs() + y_heat * p.abs() * p.abs());
            taken = taken + (y_ledger * p).scale(w);
        }
    }
    let scale = 0.5 * p_in.abs() * q_in.abs();
    let energy_residual = (inlet - dissipated - volume) / scale;
    let volume_residual = (q_in - qv - taken).abs() / q_in.abs();
    if !(g.re.is_finite() && g.im.is_finite() && energy_residual.is_finite() && volume_residual.is_finite()) {
        return Err(HoseReject::NonFiniteValue { frequency_hz });
    }
    Ok(AtFrequency { gain: g.abs(), phase: g.arg(), inlet: omega * inlet, line: omega * dissipated, volume: omega * volume, energy_residual, volume_residual })
}

/// The answer.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &HoseQuestion) -> Result<HoseResponse, HoseReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &HoseQuestion) -> Result<HoseResponse, HoseReject> {
    admit(question)?;
    solve_line(question, Line::at(question.mean_pressure_pa))
}

impl NormalOrZero for HoseResponse {
    fn normal_or_zero(&mut self) {
        self.sound_speed_m_s.normal_or_zero();
        self.first_cross_mode_hz.normal_or_zero();
        self.lossless_frequencies_hz.normal_or_zero();
        self.mode_numbers.normal_or_zero();
        self.natural_frequencies_hz.normal_or_zero();
        self.damping_ratios.normal_or_zero();
        self.peak_frequencies_hz.normal_or_zero();
        self.peak_gains.normal_or_zero();
        self.gains.normal_or_zero();
        self.phases_rad.normal_or_zero();
        self.inlet_power_w.normal_or_zero();
        self.line_dissipation_w.normal_or_zero();
        self.volume_power_w.normal_or_zero();
        self.largest_energy_ledger_residual.normal_or_zero();
        self.largest_volume_ledger_residual.normal_or_zero();
        self.field.normal_or_zero();
    }
}

/// Newton's method for a pole of `G` from `s`: the pole and the evaluations
/// it took, or `None` when it did not settle.
fn settle(here: &Line, mut s: Complex) -> (Option<Complex>, u64) {
    let mut computed = 0;
    for _ in 0..MAX_POLE_ITERATIONS {
        let h = s.abs() * 1.0e-7;
        let d = here.denominator(s);
        let d_plus = here.denominator(s + Complex::new(h, 0.0));
        let d_minus = here.denominator(s - Complex::new(h, 0.0));
        computed += 3;
        let slope = (d_plus - d_minus).scale(0.5 / h);
        let step = d / slope;
        s = s - step;
        if !(s.re.is_finite() && s.im.is_finite()) {
            return (None, computed);
        }
        if step.abs() <= POLE_SETTLED * s.abs() {
            return (Some(s), computed);
        }
    }
    (None, computed)
}

fn solve_line(question: &HoseQuestion, line: Line) -> Result<HoseResponse, HoseReject> {
    let mut computed = 0u64;
    let mut pressure_steps = 0u64;
    let asked = question.mean_pressure_pa;
    // The poles. Each mode is followed from the tube's rating, where the
    // air is densest and the line least damped, down to the asked pressure,
    // starting from the lossless line's frequency. A step in the logarithm
    // of the pressure foresees the pole from the last two, settles it by
    // Newton's method, and is taken when the settled pole lies within
    // TRACK_SHARE of the foreseen; otherwise it is halved, so that the
    // following does not jump to another mode's pole. A mode whose pole
    // reaches the real axis has stopped ringing: it is overdamped there and
    // is left out, and `mode_numbers` says which modes still ring.
    let lossless = lossless_frequencies_hz();
    let (top, bottom) = (math::log(tube_rating_pa()), math::log(asked));
    let mut poles: Vec<(usize, Complex)> = Vec::new();
    for (mode, f0) in lossless.iter().enumerate() {
        let here = |pressure: f64| Line { tamper: line.tamper, ..Line::at(pressure) };
        let start = Complex::new(-1.0e-3 * 2.0 * std::f64::consts::PI * f0, 2.0 * std::f64::consts::PI * f0);
        let (found, used) = settle(&here(tube_rating_pa()), start);
        computed += used;
        let Some(mut s) = found else { return Err(HoseReject::PoleNotSettled { mode: mode + 1 }) };
        let mut at = top;
        let mut last: Option<(f64, Complex)> = None;
        let mut step = TRACK_STEP_MAX;
        let mut ringing = s.im > OVERDAMPED_IMAGINARY_SHARE * s.abs();
        while ringing && at > bottom {
            let next = (at - step).max(bottom);
            let pressure = if next == bottom { asked } else { math::exp(next) };
            let foreseen = match last {
                Some((before, pole)) => s + (s - pole).scale((next - at) / (at - before)),
                None => s,
            };
            let (found, used) = settle(&here(pressure), foreseen);
            computed += used;
            pressure_steps += 1;
            match found {
                // Taken when the pole is the one foreseen, not its mirror
                // below the real axis, which lies near it as the mode
                // approaches overdamping.
                Some(pole) if (pole - foreseen).abs() <= TRACK_SHARE * pole.abs() && pole.im >= -OVERDAMPED_IMAGINARY_SHARE * pole.abs() => {
                    last = Some((at, s));
                    at = next;
                    s = pole;
                    step = (step * 1.5).min(TRACK_STEP_MAX);
                    if s.im <= OVERDAMPED_IMAGINARY_SHARE * s.abs() {
                        ringing = false;
                    } else if s.re >= 0.0 {
                        return Err(HoseReject::PassivityBroken { frequency_hz: s.im / (2.0 * std::f64::consts::PI) });
                    }
                }
                _ => {
                    step *= 0.5;
                    if step < TRACK_STEP_MIN {
                        return Err(match found {
                            None => HoseReject::PoleNotSettled { mode: mode + 1 },
                            Some(_) => HoseReject::PoleNotFollowed { mode: mode + 1, pressure_pa: pressure },
                        });
                    }
                }
            }
        }
        if !ringing {
            continue;
        }
        let frequency_hz = s.im / (2.0 * std::f64::consts::PI);
        if frequency_hz >= first_cross_mode_hz() {
            return Err(HoseReject::ModeAboveFirstCrossMode { mode: mode + 1, frequency_hz });
        }
        let modulus_hz = s.abs() / (2.0 * std::f64::consts::PI);
        if modulus_hz > stokes_slip_hz(asked) {
            return Err(HoseReject::ModeAboveStokesLayerSlip { mode: mode + 1, frequency_hz: modulus_hz });
        }
        poles.push((mode + 1, s));
    }
    for (i, &(first, a)) in poles.iter().enumerate() {
        for &(second, b) in &poles[i + 1..] {
            if (a - b).abs() <= DISTINCT_POLE_SHARE * a.abs() {
                return Err(HoseReject::ModesOnOnePole { first, second, frequency_hz: a.im / (2.0 * std::f64::consts::PI) });
            }
        }
    }
    let natural: Vec<f64> = poles.iter().map(|(_, s)| s.im / (2.0 * std::f64::consts::PI)).collect();
    let damping: Vec<f64> = poles.iter().map(|(_, s)| -s.re / s.abs()).collect();
    let modes: Vec<f64> = poles.iter().map(|(mode, _)| *mode as f64).collect();
    // The scan for the peaks, up to past the last mode, or to where slip
    // flow begins in the viscous layer at this pressure, whichever is lower.
    let scan_top = (1.25 * lossless[MODES - 1]).min(stokes_slip_hz(asked));
    let ratio = math::log(scan_top / SCAN_LOW_HZ);
    let scan_f: Vec<f64> = (0..SCAN_POINTS).map(|k| SCAN_LOW_HZ * math::exp(ratio * k as f64 / (SCAN_POINTS - 1) as f64)).collect();
    let gain_at = |f: f64| line.transfer(Complex::new(0.0, 2.0 * std::f64::consts::PI * f)).abs();
    let scan: Vec<f64> = scan_f.iter().map(|f| gain_at(*f)).collect();
    computed += SCAN_POINTS as u64;
    let mut peak_f = Vec::new();
    let mut peak_g = Vec::new();
    for k in 1..SCAN_POINTS - 1 {
        if peak_f.len() == MODES {
            break;
        }
        if scan[k] > scan[k - 1] && scan[k] >= scan[k + 1] {
            let (mut lo, mut hi) = (math::log(scan_f[k - 1]), math::log(scan_f[k + 1]));
            let golden = 0.5 * (5f64.sqrt() - 1.0);
            for _ in 0..80 {
                let a = hi - golden * (hi - lo);
                let b = lo + golden * (hi - lo);
                if gain_at(math::exp(a)) > gain_at(math::exp(b)) { hi = b } else { lo = a }
                computed += 2;
            }
            let f = math::exp(0.5 * (lo + hi));
            peak_f.push(f);
            peak_g.push(gain_at(f));
        }
    }
    let mut out = HoseResponse {
        sound_speed_m_s: sound_speed_m_s(),
        first_cross_mode_hz: first_cross_mode_hz(),
        lossless_frequencies_hz: lossless,
        mode_numbers: modes,
        natural_frequencies_hz: natural,
        damping_ratios: damping,
        peak_frequencies_hz: peak_f.clone(),
        peak_gains: peak_g,
        gains: Vec::new(),
        phases_rad: Vec::new(),
        inlet_power_w: Vec::new(),
        line_dissipation_w: Vec::new(),
        volume_power_w: Vec::new(),
        largest_energy_ledger_residual: 0.0,
        largest_volume_ledger_residual: 0.0,
        frequencies_computed: 0,
        pressure_steps,
        field: scan,
    };
    // The ledgers, at every peak and every asked frequency.
    let peaks = peak_f.len();
    for (k, &f) in peak_f.iter().chain(question.frequencies_hz.iter()).enumerate() {
        let at = at_frequency(&line, f)?;
        computed += 1;
        out.largest_energy_ledger_residual = out.largest_energy_ledger_residual.max(at.energy_residual.abs());
        out.largest_volume_ledger_residual = out.largest_volume_ledger_residual.max(at.volume_residual);
        if at.energy_residual.abs() > LEDGER_RESIDUAL_LIMIT {
            return Err(HoseReject::EnergyLedgerBroken { frequency_hz: f, residual: at.energy_residual });
        }
        if at.volume_residual > LEDGER_RESIDUAL_LIMIT {
            return Err(HoseReject::VolumeLedgerBroken { frequency_hz: f, residual: at.volume_residual });
        }
        if k >= peaks {
            out.gains.push(at.gain);
            out.phases_rad.push(at.phase);
            out.inlet_power_w.push(at.inlet);
            out.line_dissipation_w.push(at.line);
            out.volume_power_w.push(at.volume);
        }
    }
    out.frequencies_computed = computed;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bessel_ratio_meets_its_series_and_its_asymptote() {
        // Small z: 1 + z^2/8 + z^4/48 + z^6/... ; at z = 0.5 the series to z^6.
        let z = Complex::new(0.5, 0.0);
        let f = bessel_ratio_f(z);
        // J1(0.5) = 0.2422684577, J0(0.5) = 0.9384698072.
        let exact = 2.0 * 0.242_268_457_7 / (0.5 * 0.938_469_807_2);
        assert!((f.re - exact).abs() < 1e-9 && f.im.abs() < 1e-15, "{f:?}");
        // Large z off the real axis: J1/J0 tends to -i for Im z < 0.
        let big = Complex::new(500.0, -500.0);
        let ratio = bessel_ratio_f(big).scale(0.5) * big;
        assert!((ratio.re).abs() < 2e-3 && (ratio.im + 1.0).abs() < 2e-3, "{ratio:?}");
        // The sphere's function: 1 at small x, 3/x at large.
        assert!((sphere_f(Complex::new(1e-4, 0.0)).re - 1.0).abs() < 1e-9);
        let x = Complex::new(400.0, 400.0);
        let big_sphere = sphere_f(x) * x.scale(1.0 / 3.0);
        assert!((big_sphere.re - 1.0).abs() < 3e-3 && big_sphere.im.abs() < 3e-3, "{big_sphere:?}");
    }

    #[test]
    fn the_line_meets_poiseuille_and_kirchhoff() {
        let line = Line::at(ATMOSPHERE_PA);
        // Low frequency: Z tends to 8 mu / (pi r^4), Y to the isothermal A s / p.
        let s = Complex::new(0.0, 2.0 * std::f64::consts::PI * 1.0e-3);
        let (z, y, _) = line.parts(s);
        let r = 0.5 * HOSE_BORE_M;
        let poiseuille = 8.0 * AIR_VISCOSITY_PA_S / (std::f64::consts::PI * r.powi(4));
        assert!((z.re / poiseuille - 1.0).abs() < 1e-6, "{}", z.re / poiseuille);
        let isothermal = area() * s.im / ATMOSPHERE_PA;
        assert!((y.im / (isothermal * (1.0 + ATMOSPHERE_PA * wall_compliance_per_pa())) - 1.0).abs() < 1e-6);
        // High frequency, wide-tube limit: Kirchhoff's attenuation.
        for f in [5.0e3, 2.0e4] {
            let omega = 2.0 * std::f64::consts::PI * f;
            let (z, y, _) = line.parts(Complex::new(0.0, omega));
            let gamma = (z * y).sqrt();
            let c = sound_speed_m_s();
            let kirchhoff = (1.0 / (r * c)) * (line.nu * omega / 2.0).sqrt() * (1.0 + (AIR_GAMMA - 1.0) / AIR_PRANDTL.sqrt());
            let off = gamma.re / kirchhoff - 1.0;
            eprintln!("{f} Hz: attenuation {} against Kirchhoff {}: {off}", gamma.re, kirchhoff);
            assert!(off.abs() < 0.05, "{off}");
        }
    }

    #[test]
    fn the_modes_sit_below_the_lossless_ones_and_the_ledgers_close() {
        let q = HoseQuestion { mean_pressure_pa: 6.0e5, frequencies_hz: vec![1.0, 50.0, 200.0, 1000.0, 10_000.0] };
        let h = solve(&q).unwrap();
        eprintln!("lossless {:?}", h.lossless_frequencies_hz);
        eprintln!("natural {:?}", h.natural_frequencies_hz);
        eprintln!("damping {:?}", h.damping_ratios);
        eprintln!("peaks {:?} gains {:?}", h.peak_frequencies_hz, h.peak_gains);
        eprintln!("gains {:?} residuals {:e} {:e} computed {}", h.gains, h.largest_energy_ledger_residual, h.largest_volume_ledger_residual, h.frequencies_computed);
        for (n, l) in h.natural_frequencies_hz.iter().zip(&h.lossless_frequencies_hz) {
            assert!(n < l && *n > 0.85 * l, "{n} {l}");
        }
        assert!(h.damping_ratios.iter().all(|z| *z > 0.0 && *z < 0.5));
        assert!(h.largest_energy_ledger_residual < 1e-10 && h.largest_volume_ledger_residual < 1e-10);
        // At 1 Hz, far under the first mode, the volume follows the inlet.
        assert!((h.gains[0] - 1.0).abs() < 0.01, "{}", h.gains[0]);
        // Each peak of the response is at its pole's frequency, lightly damped.
        for (peak, natural) in h.peak_frequencies_hz.iter().zip(&h.natural_frequencies_hz) {
            assert!((peak / natural - 1.0).abs() < 0.005, "{peak} {natural}");
        }
    }

    #[test]
    fn lower_pressure_damps_more() {
        let low = solve(&HoseQuestion { mean_pressure_pa: 1.5e5, frequencies_hz: vec![] }).unwrap();
        let high = solve(&HoseQuestion { mean_pressure_pa: 1.0e6, frequencies_hz: vec![] }).unwrap();
        eprintln!("damping at 1.5 bar {:?}, at 10 bar {:?}", low.damping_ratios, high.damping_ratios);
        assert!(low.damping_ratios[0] > 1.5 * high.damping_ratios[0]);
    }

    #[test]
    fn the_question_is_refused_by_name_at_its_limits() {
        let ask = |p: f64, f: Vec<f64>| admit(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: f });
        assert!(matches!(ask(f64::NAN, vec![]), Err(HoseReject::InputNotFinite { .. })));
        assert!(matches!(ask(tube_rating_pa() + 1.0, vec![]), Err(HoseReject::PressureAboveTubeRating { .. })));
        assert!(matches!(ask(100.0, vec![]), Err(HoseReject::RarefiedFlow { .. })));
        assert!(matches!(ask(1e5, vec![1.0; 65]), Err(HoseReject::TooManyFrequencies { .. })));
        assert!(matches!(ask(1e5, vec![0.0]), Err(HoseReject::FrequencyNotPositive { .. })));
        // The cut-on is exclusive: it is itself refused, and the number
        // below it, at the rating, is taken; so are the red team's 50,289.2 Hz
        // and 170.5 Pa, inside the limits.
        assert!(matches!(ask(tube_rating_pa(), vec![FREQUENCY_MAX_HZ]), Err(HoseReject::AboveFirstCrossMode { .. })));
        ask(tube_rating_pa(), vec![FREQUENCY_MAX_HZ.next_down(), 50_289.2]).unwrap();
        assert_eq!(FREQUENCY_MAX_HZ.to_bits(), first_cross_mode_hz().to_bits());
        assert_eq!(PRESSURE_MIN_PA.to_bits(), rarefied_below_pa().to_bits());
        assert!(matches!(ask(PRESSURE_MIN_PA.next_down(), vec![]), Err(HoseReject::RarefiedFlow { .. })));
        ask(PRESSURE_MIN_PA, vec![]).unwrap();
        ask(170.5, vec![100.0]).unwrap();
        assert!((first_cross_mode_hz() - 50_289.23).abs() < 0.01, "{}", first_cross_mode_hz());
        assert!(rarefied_below_pa() > 160.0 && rarefied_below_pa() < 180.0, "{}", rarefied_below_pa());
        // Zero is exclusive; the smallest normal number above it is taken.
        ask(1e5, vec![f64::MIN_POSITIVE]).unwrap();
    }

    /// Slip flow begins in the oscillating viscous layer at the wall, the
    /// Stokes layer `sqrt(2 nu/omega)`, where it is a hundred mean free paths
    /// thick: at `f = 2 pi mu R Kn^2 d^4 p / (k_B^2 T)`, 1.0544 Hz for each
    /// pascal. At the lowest pressure that is 179.26 Hz, where the layer is as
    /// thick as the bore, so that the two limits meet; it meets the cut-on at
    /// 47,696 Pa. An asked frequency above it is refused by name, the limit
    /// itself taken.
    #[test]
    fn slip_in_the_viscous_layer_is_refused_by_name() {
        let closed = 2.0 * std::f64::consts::PI * AIR_VISCOSITY_PA_S * AIR_GAS_CONSTANT_J_KG_K * KNUDSEN_LIMIT * KNUDSEN_LIMIT * AIR_MOLECULE_DIAMETER_M.powi(4) / (BOLTZMANN_J_K * BOLTZMANN_J_K * TEMPERATURE_K);
        assert!((stokes_slip_hz(1.0) / closed - 1.0).abs() < 1e-14, "{} {closed}", stokes_slip_hz(1.0));
        assert!((stokes_slip_hz(1.0) - 1.054_377).abs() < 1e-6);
        let at_floor = stokes_slip_hz(PRESSURE_MIN_PA);
        assert!((at_floor - 179.26).abs() < 0.01, "{at_floor}");
        let nu = AIR_VISCOSITY_PA_S * AIR_GAS_CONSTANT_J_KG_K * TEMPERATURE_K / PRESSURE_MIN_PA;
        let layer = (2.0 * nu / (2.0 * std::f64::consts::PI * at_floor)).sqrt();
        assert!((layer / HOSE_BORE_M - 1.0).abs() < 1e-12, "{layer}");
        assert!((FREQUENCY_MAX_HZ / stokes_slip_hz(1.0) - 47_695.68).abs() < 0.01);
        for p in [PRESSURE_MIN_PA, 1000.0, 1.0e4, 4.0e4] {
            let limit = stokes_slip_hz(p);
            admit(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: vec![limit] }).unwrap();
            let refused = admit(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: vec![limit.next_up()] });
            assert!(matches!(refused, Err(HoseReject::StokesLayerSlip { .. })), "{p}: {refused:?}");
        }
        // Above 47,696 Pa the cut-on is the lower limit.
        let refused = admit(&HoseQuestion { mean_pressure_pa: 6.0e5, frequencies_hz: vec![FREQUENCY_MAX_HZ] });
        assert!(matches!(refused, Err(HoseReject::AboveFirstCrossMode { .. })));
    }

    /// Each mode is followed from the tube's rating with steps of pressure
    /// that halve where the pole would jump: over the range no two ringing
    /// modes share a pole, the modes that ring at a pressure are the highest
    /// ones, from one mode up to the sixth, and the poles below 3 kPa meet the
    /// red team's, followed by hand in steps of 2 percent with library Bessel
    /// functions (mpmath, 20 digits). Before, between about 239 and 749 Pa,
    /// the quarter steps landed modes 5 and 6 on the poles of modes 7 to 10,
    /// at 330.1 Pa both on one pole. Each mode stops ringing, its damping
    /// ratio past 0.999 999 99, at 8,625, 2,589.4, 1,446.0, 991.4, 751.8 and
    /// 604.7 Pa, and no mode rings below.
    #[test]
    fn the_modes_are_followed_without_jumping_and_none_lands_on_another() {
        let n = 240;
        let mut ringing_before = 0;
        for k in 0..=n {
            let p = PRESSURE_MIN_PA * (tube_rating_pa() / PRESSURE_MIN_PA).powf(k as f64 / n as f64);
            let h = solve(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: vec![] }).unwrap_or_else(|e| panic!("{p}: {e}"));
            let count = h.mode_numbers.len();
            let expected: Vec<f64> = ((MODES - count + 1)..=MODES).map(|m| m as f64).collect();
            assert_eq!(h.mode_numbers, expected, "{p}");
            assert!(count >= ringing_before, "{p}: {count} after {ringing_before}");
            ringing_before = count;
            for i in 0..count {
                for j in i + 1..count {
                    let gap = (h.natural_frequencies_hz[i] - h.natural_frequencies_hz[j]).abs() + (h.damping_ratios[i] - h.damping_ratios[j]).abs();
                    assert!(gap > 1e-6, "{p}: modes {} and {}", h.mode_numbers[i], h.mode_numbers[j]);
                }
            }
        }
        for (p, poles) in [
            (3000.0, vec![(2.0, 32.265_850_583_627_795, 0.897_480_430_448_101_9), (3.0, 114.759_538_650_034_94, 0.547_516_722_872_377), (4.0, 189.407_664_454_978_35, 0.405_850_803_926_789_74), (5.0, 263.561_596_764_610_84, 0.330_536_971_068_930_8), (6.0, 338.126_415_162_510_3, 0.284_819_092_780_872_3)]),
            (1531.9, vec![(3.0, 36.973_713_464_379_92, 0.960_528_600_366_057_8), (4.0, 142.260_167_944_842, 0.694_331_983_143_213), (5.0, 222.384_247_610_022_21, 0.553_726_832_166_782_1), (6.0, 299.296_763_291_988_16, 0.465_287_954_009_841_9)]),
            (988.1, vec![(5.0, 157.635_293_581_953_22, 0.793_332_106_094_468_6), (6.0, 245.390_607_491_637_55, 0.661_617_749_946_697_8)]),
            (637.3, vec![(6.0, 91.352_911_540_007_95, 0.958_149_447_128_577_7)]),
            (511.8, vec![]),
            (330.1, vec![]),
            (265.1, vec![]),
        ] {
            let h = solve(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: vec![] }).unwrap();
            assert_eq!(h.mode_numbers.len(), poles.len(), "{p}: {:?}", h.mode_numbers);
            for (k, (mode, natural, damping)) in poles.iter().enumerate() {
                assert_eq!(h.mode_numbers[k], *mode, "{p}");
                assert!((h.natural_frequencies_hz[k] / natural - 1.0).abs() < 1e-9, "{p} mode {mode}: {} against {natural}", h.natural_frequencies_hz[k]);
                assert!((h.damping_ratios[k] / damping - 1.0).abs() < 1e-9, "{p} mode {mode}: {} against {damping}", h.damping_ratios[k]);
            }
        }
        for (mode, below, above) in [(1usize, 8624.0, 8626.0), (2, 2589.0, 2590.0), (3, 1445.5, 1446.5), (4, 991.0, 991.8), (5, 751.5, 752.1), (6, 604.5, 605.0)] {
            let rings = |p: f64| solve(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: vec![] }).unwrap().mode_numbers.contains(&(mode as f64));
            assert!(!rings(below) && rings(above), "mode {mode} between {below} and {above}");
        }
        // The guard on a ringing pole past the viscous layer's slip: under
        // 0.53 of the limit at every pressure where a mode rings.
        for k in 0..=400 {
            let p = 604.0 * (2.0e4f64 / 604.0).powf(k as f64 / 400.0);
            let h = solve(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: vec![] }).unwrap();
            for (f, z) in h.natural_frequencies_hz.iter().zip(&h.damping_ratios) {
                assert!(f / (1.0 - z * z).sqrt() < 0.53 * stokes_slip_hz(p), "{p}");
            }
        }
    }

    /// Frequencies down to the smallest normal 64-bit number are answered:
    /// the volume follows the inlet, gain 1, and both ledgers close. Before,
    /// 2.2250738585072014e-308 Hz was refused as NonFiniteValue at 100 kPa
    /// and as VolumeLedgerBroken at 600 kPa: `Z_c` was divided out of a
    /// vanishing `Gamma`, and the flows were subnormal.
    #[test]
    fn the_smallest_frequencies_are_answered_with_their_ledgers() {
        for p in [PRESSURE_MIN_PA, 1.0e5, 6.0e5, tube_rating_pa()] {
            for f in [f64::MIN_POSITIVE, 1.0e-300, 1.0e-150, 1.0e-20, 1.0e-6] {
                let h = solve(&HoseQuestion { mean_pressure_pa: p, frequencies_hz: vec![f] }).unwrap_or_else(|e| panic!("{p} {f:e}: {e}"));
                assert!((h.gains[0] - 1.0).abs() < 1e-9, "{p} {f:e}: {}", h.gains[0]);
                assert!(h.largest_energy_ledger_residual < 1e-12 && h.largest_volume_ledger_residual < 1e-12, "{p} {f:e}");
            }
        }
        // And the low-frequency impedance is Poiseuille's to the last bits.
        let (z, _, _) = Line::at(ATMOSPHERE_PA).parts(Complex::new(0.0, 2.0 * std::f64::consts::PI * 1.0e-12));
        let r = 0.5 * HOSE_BORE_M;
        assert!((z.re / (8.0 * AIR_VISCOSITY_PA_S / (std::f64::consts::PI * r.powi(4))) - 1.0).abs() < 1e-15);
    }

    /// A refusal's sentence prints each number as the shortest decimal that
    /// reads back to it, with an exponent past a millionth.
    #[test]
    fn a_refusal_prints_short_numbers() {
        let sentence = HoseReject::NonFiniteValue { frequency_hz: f64::MIN_POSITIVE }.to_string();
        assert_eq!(sentence, "at 2.2250738585072014e-308 Hz a value was not finite");
        assert_eq!(HoseReject::RarefiedFlow { pressure_pa: 100.0 }.to_string(), "mean pressure 100 Pa is below 170.01457281119065 Pa, where the mean free path is a hundredth of the bore and slip flow begins at the wall");
    }

    #[test]
    fn a_falsified_line_is_stopped_by_name() {
        let q = HoseQuestion { mean_pressure_pa: 3.0e5, frequencies_hz: vec![123.0] };
        let with = |tamper: Tamper| solve_line(&q, Line { tamper, ..Line::at(q.mean_pressure_pa) });
        assert!(with(Tamper::None).is_ok());
        assert!(matches!(with(Tamper::ThermalSignFlipped), Err(HoseReject::PassivityBroken { .. })));
        assert!(matches!(with(Tamper::LedgerWithoutWall), Err(HoseReject::VolumeLedgerBroken { .. })));
        assert!(matches!(with(Tamper::LedgerWithoutHeat), Err(HoseReject::EnergyLedgerBroken { .. })));
        // And at the smallest frequency, where every flow is taken over omega.
        let tiny = HoseQuestion { mean_pressure_pa: 3.0e5, frequencies_hz: vec![f64::MIN_POSITIVE] };
        let with = |tamper: Tamper| solve_line(&tiny, Line { tamper, ..Line::at(tiny.mean_pressure_pa) });
        assert!(with(Tamper::None).is_ok());
        assert!(matches!(with(Tamper::LedgerWithoutWall), Err(HoseReject::VolumeLedgerBroken { .. })));
    }

    #[test]
    fn the_slowest_question_is_quick() {
        let top = FREQUENCY_MAX_HZ.next_down();
        let q = HoseQuestion { mean_pressure_pa: tube_rating_pa(), frequencies_hz: (0..64).map(|k| top * (k as f64 + 1.0) / 64.0).collect() };
        let started = std::time::Instant::now();
        let h = solve(&q).unwrap();
        eprintln!("64 frequencies to {top} Hz at the rating: {:?}, residuals {:e} {:e}", started.elapsed(), h.largest_energy_ledger_residual, h.largest_volume_ledger_residual);
        let q = HoseQuestion { mean_pressure_pa: PRESSURE_MIN_PA, frequencies_hz: (0..64).map(|k| stokes_slip_hz(PRESSURE_MIN_PA) * (k as f64 + 1.0) / 64.0).collect() };
        let started = std::time::Instant::now();
        let h = solve(&q).unwrap();
        eprintln!("64 frequencies to the slip limit at the floor: {:?}", started.elapsed());
        eprintln!("at the floor: modes {:?} natural {:?} damping {:?} residuals {:e} {:e}", h.mode_numbers, h.natural_frequencies_hz, h.damping_ratios, h.largest_energy_ledger_residual, h.largest_volume_ledger_residual);
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let q = HoseQuestion { mean_pressure_pa: 3.0e5, frequencies_hz: vec![123.0] };
        assert_eq!(solve(&q).unwrap(), solve(&q).unwrap());
    }
}
