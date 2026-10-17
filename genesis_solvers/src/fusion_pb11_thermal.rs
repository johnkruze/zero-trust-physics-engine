//! Power balance of a uniform thermal proton-boron-11 plasma.
//!
//! The steady energy balance of protons, boron-11, alpha ash and electrons
//! at one point, as Putvinski, Ryutov and Yushmanov (Nucl. Fusion 59 (2019)
//! 076018) and Ochs, Kolmes, Mlodik, Rubin and Fisch (Phys. Rev. E 106 (2022)
//! 055215) set it up, compared with their published numbers, which are
//! calculations: of eight gated lines, seven were met.
//!
//! The caller gives the ion temperature, the electron density, the fuel mix,
//! the ash, the magnetic field, the minor radius, the wall reflectivity and
//! the energy confinement time of the ions. The solver finds the electron
//! temperature at which the electrons' energy balances, and returns every
//! term of the balance, or a named limit.
//!
//! The declared range is where the balance's own forms hold, each end with
//! its physical reason at its constant: ion temperature 1 to 1000 keV (from
//! the 1 keV floor the electrons are sought from, to the top of the
//! reactivity, `pb11_reaction`), electron density 1e18 to 1e22 per
//! cubic metre, boron 0.01 to 0.9 of the fuel ions, ash 0 to 1 per fuel ion,
//! an applied field above zero to 1e6 T, minor radius 0.1 to 10 m, wall
//! reflectivity 0 to 1, any confinement time above zero. Inside it the
//! solver stops by name where the physics does: the field cannot hold the
//! plasma (`PressureExceedsField`), the electrons do not balance below
//! [`ELECTRON_TEMPERATURE_MAX_KEV`] (`ElectronTemperatureOutOfRange`), or the
//! plasma is not opaque enough for Trubnikov's synchrotron formula
//! (`SynchrotronAboveUnabsorbedEmission`).
//!
//! The comparison with the published calculations is the 29 converged runs
//! of `evidence/research/fusion_pb11/solver/runs_v060`: ion temperatures of
//! 200 to 450 keV, electron densities of 1.52e20 and 1.6e20 per cubic metre,
//! 13 and 15 % boron, no ash, applied fields of 6.661 to 20 T, a minor
//! radius of 2 m, wall reflectivities of 0.95 and 1, and a confinement time
//! of 100 s. From engine 0.7.0 to this version the solver refused every
//! question outside that envelope; the 0.6.0 range, and the audit's runs at
//! 10 keV and at the far corner, answered with every ledger closed.
//!
//! The electron balance need not fall with temperature: where the field
//! holds the plasma near its limit, the field inside, and with it the
//! synchrotron loss, goes to zero, and the balance can turn from heating to
//! loss and back more than once. The solver samples it on a fixed grid,
//! every [`ELECTRON_GRID_STEP_KEV`] from [`ELECTRON_TEMPERATURE_MIN_KEV`] up
//! to the top of the search, and settles the first interval in which it
//! turns from heating to loss: the temperature electrons heated from cold
//! reach. A higher balance is not reported.
//!
//! ```text
//! n_e = n_p + 5 n_B + 2 n_α      n_p + n_B = n_e / (1 + 4 x_B + 2 x_ash)
//! S      = n_p n_B ⟨σv⟩(T_i)                       reactions per m³ per second   (pb11_reaction)
//! P_fus  = S · Y                                   Y = 8.68 MeV
//! P_brem = A0 n_e² √t [Z_eff (1 + 1.78 t^1.34) + 2.12 t (1 + 1.1 t + t² − 1.25 t^2.5)]     t = T_e / m_e c²
//! P_sync = 4.14e-7 √n₂₀ T_e^2.5 B_in^2.5 √(1 − R_w) (1 + 2.5 T_e/511) / √a   MW/m³
//! P_ie   = (3/2) n_e Σ_i ν_i (T_i − T_e) · R(t)
//! ν_i    = (8√(2π)/3) n_i Z_i² e⁴ lnΛ √(m_e m_i) / [(4π ε0)² (m_i T_e + m_e T_i)^1.5]
//! B_in   = √(B² − 2 μ0 p)                          p = (n_p + n_B + n_α) T_i + n_e T_e
//!
//! electrons:  S f_e (Y − 1.5 T_i) + P_ie − P_brem − P_sync = 0          fixes T_e
//! ions:       P_aux + S [1.5 T_i + f_i (Y − 1.5 T_i)] − P_ie − W_i/τ_E = 0   gives P_aux
//! ```
//!
//! The three alphas of a reaction carry `Y + 3 T_i` and come to rest at
//! `1.5 T_i` each; `f_i` and `f_e` are the shares of what they give up that
//! go to ions and to electrons, each the integral of its own channel's rate
//! over the total from the birth energy `(Y + 3 T_i)/3` down (the rates of
//! `pb11_reaction`). Heat is conducted away through the ions only, as in the
//! two papers. `A0 = (32/3) √(2/π) α r_e² m_e c³`.
//!
//! Sources. Bremsstrahlung: Svensson, Astrophys. J. 258 (1982) 335,
//! equations 17 and 19, each within 2 % of the exact rate for `t < 1`, the
//! form both papers use. A second published fit, Xie (2024), is evaluated at
//! the settled temperature and returned beside it, outside the balance: the
//! two differ by about 2 %, which is as large as the whole ignition margin.
//! Synchrotron: Trubnikov's formula in the form of Cai et al. (2022),
//! equation 30. It is the loss of a plasma opaque to its own low harmonics:
//! the unabsorbed emission of the electrons times a transparency factor
//! `60 t^1.5 √((1 − R_w)/p_a)`, `p_a = a ω_pe² / (c ω_ce)`. Where that factor
//! passes one the formula gives more than the electrons emit at all, which no
//! plasma can lose, and the solver stops by name: the loss is held at every
//! trial temperature under the unabsorbed emission
//! `e⁴ B_in² n_e T_e (1 + T_e/146 keV) / (3π ε0 m_e³ c³)` (Xie 2024, book
//! equation 3.25, after Dolan). Exchange: Spitzer's rate with the Formulary's Coulomb
//! logarithm and the relativistic factor of Putvinski (2019), equation 10.
//! The research record is
//! `evidence/research/fusion_pb11/power_balance/FINDINGS.md`.
//!
//! What must hold. Charge: `n_e − n_p − 5 n_B − 2 n_α = 0`. Energy, at every
//! trial temperature: the two alpha shares add to one, and the synchrotron
//! loss is at most the unabsorbed emission. Energy, at
//! acceptance: the electron balance closes, and the sum of the two balances
//! equals `P_aux + P_fus − P_brem − P_sync − W_i/τ_E`, which holds only if
//! the exchange and the alpha shares are each worked out once and used in
//! both.
//!
//! One switch, off unless asked: `kinetic_enhancement` multiplies the
//! Maxwellian reactivity by the factor Ochs et al. (2022, Appendix B) use to
//! stand in for Putvinski's kinetic calculation of the proton tail, linear
//! from 1.16 at 0 keV to 1 at 700 keV. It is a fit to another model's
//! output. It is theory; nothing has measured it.
//!
//! Left out: profiles, particle balance, any loss through the electrons but
//! radiation, alpha channeling, avalanche, spin polarisation, ¹¹B(p,n)¹¹C
//! and every other side reaction, reabsorption of bremsstrahlung, the
//! superthermal suppression of synchrotron loss, and every limit that
//! belongs to a device (density limits, the beta a configuration can hold,
//! the field a magnet can make).

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::f64::consts::PI;
use std::fmt;

use crate::pb11_reaction::{
    self as reaction, ALPHA_MASS_KG, BORON11_MASS_KG, Background, ELECTRON_MASS_KG,
    ELECTRON_REST_KEV, ELEMENTARY_CHARGE_C, JOULE_PER_KEV, PROTON_MASS_KG, REACTION_ENERGY_KEV,
    Species, VACUUM_PERMEABILITY_H_PER_M, VACUUM_PERMITTIVITY_F_PER_M,
};

/// Ion temperature, keV. Low end: [`ELECTRON_TEMPERATURE_MIN_KEV`], the
/// floor of the bremsstrahlung fit; the electrons are sought from it up to
/// the ions' temperature, so below it no electron temperature can be sought,
/// and at it the search is empty and stops by name
/// (`ElectronTemperatureOutOfRange`). The reactivity holds from
/// [`reaction::REACTIVITY_TEMPERATURE_MIN_KEV`], the same 1 keV. High end:
/// [`reaction::REACTIVITY_TEMPERATURE_MAX_KEV`], where the part of the rate
/// above 9.76 MeV, the top of the cross-section fit, at which the integral
/// stops, is 1.7e-5 of it. Below about 3 keV the energies that carry the
/// rate fall under 22 keV, the lowest energy at which the cross-section was
/// measured, and the rate rests on the fit's S-factor near zero energy.
/// Compared with published calculations from 200 to 450 keV. The low end was
/// 10 keV until this version.
pub const ION_TEMPERATURE_MIN_KEV: f64 = ELECTRON_TEMPERATURE_MIN_KEV;
pub const ION_TEMPERATURE_MAX_KEV: f64 = reaction::REACTIVITY_TEMPERATURE_MAX_KEV;
/// Electron density, per cubic metre. Low end: the balance is the steady
/// state the alphas reach as they slow on the electrons, and that takes a
/// time that grows as one over the density, about 1700 s at 1e18 per cubic
/// metre with electrons at 150 keV; below it, longer than half an hour.
/// High end: at 1e22 the pressure of ions at 300 keV needs an applied field
/// of about 37 T, and denser plasmas begin to reabsorb their own
/// bremsstrahlung, which the balance takes as escaping (it is optically
/// thin). Compared at 1.52e20 and 1.6e20.
pub const ELECTRON_DENSITY_MIN_PER_M3: f64 = 1.0e18;
pub const ELECTRON_DENSITY_MAX_PER_M3: f64 = 1.0e22;
/// Boron share of the fuel ions, `n_B / (n_p + n_B)`. Any share strictly
/// between 0 and 1 is a plasma of both fuels; at either end there is one
/// fuel, no reaction, and the ignition margin is 0/0. The range runs from 99
/// protons to each boron nucleus to nine boron nuclei to each proton, and
/// boron's charge, 5, is inside the charges the published bremsstrahlung
/// fits are stated for. Compared at 13 and 15 %.
pub const BORON_FRACTION_MIN: f64 = 0.01;
pub const BORON_FRACTION_MAX: f64 = 0.9;
/// Thermal alpha ash per fuel ion, at most one: as many alphas as fuel ions.
/// No equation fails with ash, which dilutes the fuel through quasi-neutrality
/// and enters the effective charge, the exchange and the alpha drag; past one
/// the plasma is mostly ash. Below zero is no density. No comparison run had
/// ash.
pub const ASH_FRACTION_MAX: f64 = 1.0;
/// Magnetic field applied from outside, tesla: above zero, to 1e6 T. The low
/// end is the pressure balance's, held by name (`PressureExceedsField`).
/// High end: the electron's cyclotron quantum, ħω_ce = 1.158e-4 eV per
/// tesla, is 0.116 keV at 1e6 T, a tenth of the 1 keV floor of the electron
/// temperature; above it the electrons' motion across the field is quantised
/// on the scale of their own energy, and classical synchrotron emission, on
/// which Trubnikov's formula rests, does not hold. Where a field makes the
/// plasma too thin for that formula the solver stops by name
/// (`SynchrotronAboveUnabsorbedEmission`). What field a magnet can make is a
/// property of a device. Compared from 6.661 to 20 T.
pub const MAGNETIC_FIELD_MIN_T: f64 = 0.0;
pub const MAGNETIC_FIELD_MAX_T: f64 = 1.0e6;
/// Minor radius of the plasma, metres: 0.1 m, a laboratory plasma, to 10 m,
/// larger than any reactor design. The radius enters only Trubnikov's
/// synchrotron loss, as one over its root, and that formula's own condition,
/// an opaque plasma, is held by name. Compared at 2 m.
pub const MINOR_RADIUS_MIN_M: f64 = 0.1;
pub const MINOR_RADIUS_MAX_M: f64 = 10.0;
/// Wall reflectivity, 0 to 1: the share of the synchrotron radiation sent
/// back cannot be below none or above all, and `√(1 − R_w)` is not real
/// above 1. Compared at 0.95 and 1.
pub const WALL_REFLECTIVITY_MIN: f64 = 0.0;
pub const WALL_REFLECTIVITY_MAX: f64 = 1.0;
/// Energy confinement time of the ions, seconds: any time above zero. It
/// enters only the auxiliary power, `W_i/τ_E`, which is infinite at zero.
/// The top, 1e300 s, is a capacity and no physics: a stated number past every
/// time a plasma could be held, so the input has a top a request can be told.
/// Compared at 100 s, where no pass line compares the auxiliary power.
pub const CONFINEMENT_TIME_MIN_S: f64 = 0.0;
pub const CONFINEMENT_TIME_MAX_S: f64 = 1.0e300;
/// The electron temperature is sought from here up, keV: the floor of the bremsstrahlung fit.
pub const ELECTRON_TEMPERATURE_MIN_KEV: f64 = 1.0;
/// The bremsstrahlung, exchange and synchrotron forms hold below the electron rest energy; the search stops here, keV.
pub const ELECTRON_TEMPERATURE_MAX_KEV: f64 = 500.0;
/// Step of the fixed grid on which the electron balance is sampled from [`ELECTRON_TEMPERATURE_MIN_KEV`] up, keV.
pub const ELECTRON_GRID_STEP_KEV: f64 = 0.25;

/// Coefficient of Trubnikov's synchrotron formula, MW/m³ (Cai et al. 2022, equation 30).
pub const SYNCHROTRON_COEFF_MW_PER_M3: f64 = 4.14e-7;
/// The electron temperature of the first relativistic term of the unabsorbed
/// synchrotron emission, `1 + 3.5 t = 1 + T_e / 146 keV` (Xie 2024, book
/// equation 3.25).
pub const UNABSORBED_EMISSION_RELATIVISTIC_KEV: f64 = 146.0;
/// The kinetic factor at zero temperature and the temperature at which it reaches one, keV (Ochs et al. 2022, Appendix B).
pub const KINETIC_FACTOR_AT_ZERO: f64 = 1.16;
pub const KINETIC_FACTOR_ENDS_KEV: f64 = 700.0;
/// At the field's limit the field inside is zero: a squared field inside below zero by no more than this share of the applied one is rounding.
const FIELD_ROUNDING: f64 = 1.0e-12;
/// Panels of the alpha slowing-down integral.
pub const ALPHA_PANELS: usize = 64;
/// The public solver stops after this many halvings of the temperature bracket.
pub const MAX_ITERATIONS: u32 = 200;
/// Charge must close within this fraction of the electron density.
pub const CHARGE_TOLERANCE: f64 = 1.0e-12;
/// The two alpha shares must add to one within this.
pub const ALPHA_SHARE_TOLERANCE: f64 = 1.0e-12;
/// Each energy balance must close within this fraction of its own heating.
/// Where the electrons sit within a part in a million of the ion temperature
/// the exchange is a difference of two nearly equal numbers, and the balance
/// at the nearest representable temperature is no closer than about 1e-11.
pub const ENERGY_TOLERANCE: f64 = 1.0e-9;

/// The published calculation this solver is checked against.
pub const REFERENCE_NAME: &str = "Ochs, Kolmes, Mlodik, Rubin and Fisch (2022), Improving the feasibility of economical proton-boron-11 fusion via alpha channeling with a hybrid fast and thermal proton scheme, Phys. Rev. E 106, 055215, arXiv:2210.08076: electron temperature 163 keV at an ion temperature of 300 keV with 13 % boron, and fusion power 2.8 % above bremsstrahlung at most with 15 % boron";
pub const REFERENCE_URL: &str = "https://arxiv.org/abs/2210.08076";
/// Date the reference was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// The electron temperature agrees with the published one to this fraction.
pub const REFERENCE_TEMPERATURE_TOLERANCE: f64 = 0.05;
/// The largest fusion power over bremsstrahlung agrees with the published one to this fraction.
pub const REFERENCE_RATIO_TOLERANCE: f64 = 0.02;

/// One plasma.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalQuestion {
    /// Temperature of protons, boron and ash, keV.
    pub ion_temperature_kev: f64,
    /// Electron density, per cubic metre.
    pub electron_density_per_m3: f64,
    /// Boron share of the fuel ions, `n_B / (n_p + n_B)`.
    pub boron_fraction: f64,
    /// Thermal alpha ash per fuel ion, `n_α / (n_p + n_B)`.
    pub ash_fraction: f64,
    /// Magnetic field applied from outside the plasma, tesla.
    pub magnetic_field_t: f64,
    /// Minor radius of the plasma, metres.
    pub minor_radius_m: f64,
    /// Share of the synchrotron radiation the wall sends back, 0 to 1.
    pub wall_reflectivity: f64,
    /// Energy confinement time of the ions, seconds.
    pub energy_confinement_time_s: f64,
    /// Multiply the reactivity by the kinetic factor of Ochs et al. (2022). Theory.
    pub kinetic_enhancement: bool,
}

/// Why no balance was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum ThermalReject {
    /// `ion_temperature_kev` is outside 1 to 1000 keV: below 1 keV, the floor of the bremsstrahlung fit from which the electrons are sought up to the ions' temperature, no electron temperature can be sought; above 1000 keV the part of the rate carried above 9.76 MeV, where the cross-section fit ends and the reactivity integral stops, grows past 1.7e-5 of it.
    IonTemperatureOutOfRange { ion_temperature_kev: f64 },
    /// `electron_density_per_m3` is outside 1e18 to 1e22 per cubic metre: below it the alphas take longer than half an hour to slow and reach the steady balance, above it the plasma begins to reabsorb its bremsstrahlung.
    ElectronDensityOutOfRange { electron_density_per_m3: f64 },
    /// `boron_fraction` is outside 0.01 to 0.9 of the fuel ions: at 0 or 1 there is one fuel and no reaction.
    BoronFractionOutOfRange { boron_fraction: f64 },
    /// `ash_fraction` is below zero, which is no density, or above one alpha per fuel ion.
    AshFractionOutOfRange { ash_fraction: f64 },
    /// `magnetic_field_t` is not above zero, where nothing holds the plasma, or is above 1e6 T, where the electron's cyclotron quantum reaches a tenth of the 1 keV floor of the electron temperature and classical synchrotron emission no longer holds.
    MagneticFieldOutOfRange { magnetic_field_t: f64 },
    /// `minor_radius_m` is outside 0.1 to 10 m, a laboratory plasma to one larger than any reactor design.
    MinorRadiusOutOfRange { minor_radius_m: f64 },
    /// `wall_reflectivity` is outside 0 to 1: a wall cannot send back less than none or more than all of the radiation.
    WallReflectivityOutOfRange { wall_reflectivity: f64 },
    /// `energy_confinement_time_s` is not above zero, where the power that holds the ions is infinite, or is above the 1e300 s capacity of the input.
    ConfinementTimeOutOfRange { energy_confinement_time_s: f64 },
    /// An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.
    NonFiniteValue,
    /// Quasi-neutrality: the ion charges left [`CHARGE_TOLERANCE`] of the electron density.
    ChargeNotNeutral { residual: f64 },
    /// Pressure balance: the ions, with the electrons at [`ELECTRON_TEMPERATURE_MIN_KEV`], are more than the applied field can hold, or the electron balance, sampled every [`ELECTRON_GRID_STEP_KEV`] from [`ELECTRON_TEMPERATURE_MIN_KEV`], heats the electrons at every point up to the highest electron temperature the field can hold.
    PressureExceedsField { pressure_pa: f64 },
    /// The electron energy balance, sampled every [`ELECTRON_GRID_STEP_KEV`], does not turn from heating to loss between [`ELECTRON_TEMPERATURE_MIN_KEV`] and the lower of the ion temperature and [`ELECTRON_TEMPERATURE_MAX_KEV`], the range of the radiation and exchange forms.
    ElectronTemperatureOutOfRange { at_or_below_floor: bool },
    /// At a trial electron temperature Trubnikov's synchrotron loss is above the unabsorbed emission of the electrons: the plasma is too thin or the field too strong for the formula of an opaque plasma, which then gives more than the electrons emit at all.
    SynchrotronAboveUnabsorbedEmission { electron_temperature_kev: f64, ratio: f64 },
    /// At a trial temperature the shares of the alpha energy to ions and to electrons left [`ALPHA_SHARE_TOLERANCE`] of one, or a share was not positive.
    AlphaSharesBroken { residual: f64 },
    /// The iteration cap was reached with the electron temperature still open.
    NotSettled { iterations: u32 },
    /// At the settled temperature an energy balance left [`ENERGY_TOLERANCE`].
    EnergyBalanceBroken { residual: f64 },
}

impl fmt::Display for ThermalReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IonTemperatureOutOfRange { ion_temperature_kev } => write!(
                f,
                "ion temperature {ion_temperature_kev} keV is outside {ION_TEMPERATURE_MIN_KEV} to {ION_TEMPERATURE_MAX_KEV} keV: below {ION_TEMPERATURE_MIN_KEV} keV, the floor of the bremsstrahlung fit the electrons are sought from, no electron temperature can be sought, and above {ION_TEMPERATURE_MAX_KEV} keV the part of the rate above 9.76 MeV, where the cross-section fit ends and the reactivity integral stops, grows past 1.7e-5 of it"
            ),
            Self::ElectronDensityOutOfRange { electron_density_per_m3 } => write!(
                f,
                "electron density {electron_density_per_m3} m^-3 is outside 1e18 to 1e22 m^-3: below it the alphas take longer than half an hour to slow and reach the steady balance, above it the plasma begins to reabsorb its bremsstrahlung"
            ),
            Self::BoronFractionOutOfRange { boron_fraction } => write!(
                f,
                "boron fraction {boron_fraction} is outside {BORON_FRACTION_MIN} to {BORON_FRACTION_MAX} of the fuel ions: at 0 or 1 there is one fuel and no reaction"
            ),
            Self::AshFractionOutOfRange { ash_fraction } => {
                write!(f, "ash fraction {ash_fraction} is below 0, which is no density, or above {ASH_FRACTION_MAX} alpha per fuel ion")
            }
            Self::MagneticFieldOutOfRange { magnetic_field_t } => write!(
                f,
                "magnetic field {magnetic_field_t} T is not above 0 T, where nothing holds the plasma, or is above {MAGNETIC_FIELD_MAX_T} T, where the electron's cyclotron quantum reaches a tenth of the 1 keV floor and classical synchrotron emission no longer holds"
            ),
            Self::MinorRadiusOutOfRange { minor_radius_m } => write!(
                f,
                "minor radius {minor_radius_m} m is outside {MINOR_RADIUS_MIN_M} to {MINOR_RADIUS_MAX_M} m, a laboratory plasma to one larger than any reactor design"
            ),
            Self::WallReflectivityOutOfRange { wall_reflectivity } => {
                write!(f, "wall reflectivity {wall_reflectivity} is outside {WALL_REFLECTIVITY_MIN} to {WALL_REFLECTIVITY_MAX}: a wall cannot send back less than none or more than all")
            }
            Self::ConfinementTimeOutOfRange { energy_confinement_time_s } => write!(
                f,
                "energy confinement time {energy_confinement_time_s} s is not above 0 s, where the power that holds the ions is infinite, or is above {CONFINEMENT_TIME_MAX_S:e} s, the capacity of the input"
            ),
            Self::NonFiniteValue => write!(f, "an input or an intermediate value was not finite"),
            Self::ChargeNotNeutral { residual } => {
                write!(f, "the ion charges missed the electron density by {residual} of it")
            }
            Self::PressureExceedsField { pressure_pa } => write!(
                f,
                "the plasma pressure, {pressure_pa} Pa or more, is above what the applied field can hold"
            ),
            Self::ElectronTemperatureOutOfRange { at_or_below_floor } => {
                if *at_or_below_floor {
                    write!(f, "the electrons balance at or below {ELECTRON_TEMPERATURE_MIN_KEV} keV, under the range of the radiation forms")
                } else {
                    write!(f, "the electrons do not balance below the ion temperature or {ELECTRON_TEMPERATURE_MAX_KEV} keV")
                }
            }
            Self::SynchrotronAboveUnabsorbedEmission { electron_temperature_kev, ratio } => write!(
                f,
                "at an electron temperature of {electron_temperature_kev} keV Trubnikov's synchrotron loss is {ratio} times the unabsorbed emission of the electrons: the plasma is not opaque enough for the formula"
            ),
            Self::AlphaSharesBroken { residual } => {
                write!(f, "the alpha energy shares missed one by {residual}")
            }
            Self::NotSettled { iterations } => write!(
                f,
                "the electron temperature was still open after {iterations} iterations"
            ),
            Self::EnergyBalanceBroken { residual } => {
                write!(f, "an energy balance missed by {residual} of its heating")
            }
        }
    }
}

impl std::error::Error for ThermalReject {}

/// The settled balance.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalBalance {
    /// Electron temperature at which the electron energy balances, keV.
    pub electron_temperature_kev: f64,
    /// Reactivity used, m³/s: the Maxwellian value, times the kinetic factor when asked.
    pub reactivity_m3_per_s: f64,
    /// Fusion power, W/m³.
    pub fusion_power_w_per_m3: f64,
    /// Bremsstrahlung, W/m³, the fit in the balance (Svensson 1982).
    pub bremsstrahlung_power_w_per_m3: f64,
    /// Bremsstrahlung at the same temperature by the fit of Xie (2024), W/m³. Not in the balance.
    pub bremsstrahlung_power_xie_w_per_m3: f64,
    /// Synchrotron loss, W/m³.
    pub synchrotron_power_w_per_m3: f64,
    /// Power flowing from ions to electrons by collisions, W/m³.
    pub ion_electron_power_w_per_m3: f64,
    /// Share of the alphas' slowing-down energy given to electrons.
    pub alpha_energy_fraction_to_electrons: f64,
    /// Power that must be given to the ions to hold their temperature, W/m³. Negative: a surplus.
    pub auxiliary_power_w_per_m3: f64,
    /// Fusion power less both radiation losses, over fusion power. Above zero the plasma can ignite if confined long enough.
    pub ignition_margin: f64,
    /// The ion energy surplus over the stored energy of ions and electrons, per second. Its inverse is the total-energy confinement time at which the plasma holds itself. It is not one over the ion-only `energy_confinement_time_s` at which the auxiliary power is zero: the auxiliary power is the ion stored energy over `energy_confinement_time_s` less the surplus. Zero or below: it cannot.
    pub ignition_loss_rate_per_s: f64,
    /// Plasma pressure, pascals.
    pub pressure_pa: f64,
    /// Plasma pressure over the pressure of the applied field.
    pub beta: f64,
    /// Field inside the plasma, tesla.
    pub internal_field_t: f64,
    /// `Σ n_i Z_i² / n_e`.
    pub effective_charge: f64,
    /// Electron balance over the electron heating.
    pub electron_energy_residual: f64,
    /// Total balance less the electron balance, over the largest term of the balance.
    pub total_energy_residual: f64,
    /// `(n_e − n_p − 5 n_B − 2 n_α) / n_e`.
    pub charge_residual: f64,
    /// Halvings of the temperature bracket.
    pub iterations: u32,
    /// The terms of the settled balance, W/m³: fusion, alpha heating of ions,
    /// alpha heating of electrons, reactant energy returned to the ions,
    /// exchange, bremsstrahlung, synchrotron, conduction, auxiliary.
    pub balance_terms_w_per_m3: Vec<f64>,
}

/// The plasma, worked out once from the question.
struct Plasma {
    question: ThermalQuestion,
    ions: Vec<Species>,
    ion_density_per_m3: f64,
    effective_charge: f64,
    charge_residual: f64,
    reactivity_m3_per_s: f64,
    reactions_per_m3_s: f64,
    /// Energy the three alphas give up while slowing, joules per reaction.
    slowing_energy_j: f64,
    alpha_birth_energy_kev: f64,
}

/// Every term at one electron temperature.
#[derive(Clone, Copy, Debug)]
struct Terms {
    electron_temperature_kev: f64,
    share_ions: f64,
    share_electrons: f64,
    exchange: f64,
    bremsstrahlung: f64,
    synchrotron: f64,
    pressure_pa: f64,
    internal_field_t: f64,
}

/// `A0 = (32/3) √(2/π) α r_e² m_e c³`, W m³.
pub fn bremsstrahlung_coefficient_w_m3() -> f64 {
    let c = reaction::SPEED_OF_LIGHT_M_PER_S;
    (32.0 / 3.0) * (2.0 / PI).sqrt() * reaction::FINE_STRUCTURE * reaction::ELECTRON_RADIUS_M * reaction::ELECTRON_RADIUS_M
        * ELECTRON_MASS_KG * c * c * c
}

/// Bremsstrahlung of Svensson (1982), W/m³.
pub fn bremsstrahlung_svensson_w_per_m3(electron_density_per_m3: f64, effective_charge: f64, electron_temperature_kev: f64) -> f64 {
    let t = electron_temperature_kev / ELECTRON_REST_KEV;
    bremsstrahlung_coefficient_w_m3() * electron_density_per_m3 * electron_density_per_m3 * t.sqrt()
        * (effective_charge * (1.0 + 1.78 * math::powf(t, 1.34)) + 2.12 * t * (1.0 + 1.1 * t + t * t - 1.25 * math::powf(t, 2.5)))
}

/// Bremsstrahlung of Xie (2024, arXiv:2404.11540, equations 63, A.1 and A.2), W/m³: stated for `t ≤ 2`, within 2 % for charge up to 5.
pub fn bremsstrahlung_xie_w_per_m3(electron_density_per_m3: f64, ions: &[Species], electron_temperature_kev: f64) -> f64 {
    let t = electron_temperature_kev / ELECTRON_REST_KEV;
    const C: [f64; 6] = [0.4302, 24.2255e-5, 0.7546e-5, 0.5282, 0.3301, 0.0911];
    let mut ion_part = 0.0;
    for ion in ions {
        let z2 = ion.charge * ion.charge;
        let non_relativistic = C[0] * (1.0 - math::exp(-math::powf(C[1] * z2 / t, C[3])))
            - (C[0] + C[5]) * math::exp(-math::powf(t / (C[2] * z2), C[4]));
        let gaunt = 1.0 + non_relativistic + t * (0.50 + t * (2.9 + t * (-2.6 + t * (1.12 - 0.19 * t))));
        ion_part += z2 * ion.density_per_m3 * gaunt;
    }
    let onset = 0.5 * (math::tanh(0.602 * (math::log10(t) + 5.06)) + 1.0);
    let electron_part = onset * (3.0 / 2.0_f64.sqrt()) * t
        * (1.0 + t * (0.757 + t * (1.81 + t * (-3.355 + t * (1.96 - 0.388 * t)))));
    bremsstrahlung_coefficient_w_m3() * electron_density_per_m3 * electron_density_per_m3 * t.sqrt()
        * (ion_part / electron_density_per_m3 + electron_part)
}

/// Synchrotron loss by Trubnikov's formula, W/m³, with the field inside the plasma.
pub fn synchrotron_w_per_m3(
    electron_density_per_m3: f64,
    electron_temperature_kev: f64,
    internal_field_t: f64,
    minor_radius_m: f64,
    wall_reflectivity: f64,
) -> f64 {
    let te = electron_temperature_kev;
    SYNCHROTRON_COEFF_MW_PER_M3 * 1.0e6 * (electron_density_per_m3 / 1.0e20).sqrt() * te * te * te.sqrt()
        * internal_field_t * internal_field_t * internal_field_t.sqrt()
        * (1.0 - wall_reflectivity).sqrt()
        * (1.0 + 2.5 * te / 511.0)
        / minor_radius_m.sqrt()
}

/// The coefficient of the unabsorbed synchrotron emission,
/// `e⁴ / (3π ε0 m_e³ c³)` times the joules in a keV: W/m³ per T² per m⁻³ per
/// keV, 6.2116e-17.
pub fn unabsorbed_emission_coefficient() -> f64 {
    let e2 = ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C;
    let c = reaction::SPEED_OF_LIGHT_M_PER_S;
    e2 * e2 * JOULE_PER_KEV
        / (3.0 * PI * VACUUM_PERMITTIVITY_F_PER_M * ELECTRON_MASS_KG * ELECTRON_MASS_KG * ELECTRON_MASS_KG * c * c * c)
}

/// The synchrotron emission of the electrons with none of it absorbed, W/m³:
/// what an optically thin plasma radiates, and the most any plasma can.
pub fn unabsorbed_synchrotron_w_per_m3(electron_density_per_m3: f64, electron_temperature_kev: f64, internal_field_t: f64) -> f64 {
    unabsorbed_emission_coefficient() * internal_field_t * internal_field_t * electron_density_per_m3 * electron_temperature_kev
        * (1.0 + electron_temperature_kev / UNABSORBED_EMISSION_RELATIVISTIC_KEV)
}

/// The factor on the Maxwellian reactivity that stands in for the kinetic proton tail.
pub fn kinetic_factor(ion_temperature_kev: f64) -> f64 {
    let factor = KINETIC_FACTOR_AT_ZERO - (KINETIC_FACTOR_AT_ZERO - 1.0) * ion_temperature_kev / KINETIC_FACTOR_ENDS_KEV;
    if factor > 1.0 { factor } else { 1.0 }
}

/// The named limits a question itself shows: the eight ranges, each end with its physical reason at its constant.
pub fn admit(question: &ThermalQuestion) -> Result<(), ThermalReject> {
    let q = *question;
    for value in [
        q.ion_temperature_kev,
        q.electron_density_per_m3,
        q.boron_fraction,
        q.ash_fraction,
        q.magnetic_field_t,
        q.minor_radius_m,
        q.wall_reflectivity,
        q.energy_confinement_time_s,
    ] {
        if !value.is_finite() {
            return Err(ThermalReject::NonFiniteValue);
        }
    }
    if !(ION_TEMPERATURE_MIN_KEV..=ION_TEMPERATURE_MAX_KEV).contains(&q.ion_temperature_kev) {
        return Err(ThermalReject::IonTemperatureOutOfRange { ion_temperature_kev: q.ion_temperature_kev });
    }
    if !(ELECTRON_DENSITY_MIN_PER_M3..=ELECTRON_DENSITY_MAX_PER_M3).contains(&q.electron_density_per_m3) {
        return Err(ThermalReject::ElectronDensityOutOfRange { electron_density_per_m3: q.electron_density_per_m3 });
    }
    if !(BORON_FRACTION_MIN..=BORON_FRACTION_MAX).contains(&q.boron_fraction) {
        return Err(ThermalReject::BoronFractionOutOfRange { boron_fraction: q.boron_fraction });
    }
    if !(0.0..=ASH_FRACTION_MAX).contains(&q.ash_fraction) {
        return Err(ThermalReject::AshFractionOutOfRange { ash_fraction: q.ash_fraction });
    }
    if !(q.magnetic_field_t > MAGNETIC_FIELD_MIN_T && q.magnetic_field_t <= MAGNETIC_FIELD_MAX_T) {
        return Err(ThermalReject::MagneticFieldOutOfRange { magnetic_field_t: q.magnetic_field_t });
    }
    if !(MINOR_RADIUS_MIN_M..=MINOR_RADIUS_MAX_M).contains(&q.minor_radius_m) {
        return Err(ThermalReject::MinorRadiusOutOfRange { minor_radius_m: q.minor_radius_m });
    }
    if !(WALL_REFLECTIVITY_MIN..=WALL_REFLECTIVITY_MAX).contains(&q.wall_reflectivity) {
        return Err(ThermalReject::WallReflectivityOutOfRange { wall_reflectivity: q.wall_reflectivity });
    }
    if !(q.energy_confinement_time_s > CONFINEMENT_TIME_MIN_S && q.energy_confinement_time_s <= CONFINEMENT_TIME_MAX_S) {
        return Err(ThermalReject::ConfinementTimeOutOfRange { energy_confinement_time_s: q.energy_confinement_time_s });
    }
    Ok(())
}

impl Plasma {
    fn new(question: &ThermalQuestion) -> Result<Self, ThermalReject> {
        let q = *question;
        let fuel = q.electron_density_per_m3 / (1.0 + 4.0 * q.boron_fraction + 2.0 * q.ash_fraction);
        let protons = (1.0 - q.boron_fraction) * fuel;
        let boron = q.boron_fraction * fuel;
        let ash = q.ash_fraction * fuel;
        let charge_residual = (q.electron_density_per_m3 - protons - 5.0 * boron - 2.0 * ash) / q.electron_density_per_m3;
        if !(charge_residual.abs() <= CHARGE_TOLERANCE) {
            return Err(ThermalReject::ChargeNotNeutral { residual: charge_residual });
        }
        let mut ions = vec![
            Species { charge: 1.0, mass_kg: PROTON_MASS_KG, density_per_m3: protons },
            Species { charge: 5.0, mass_kg: BORON11_MASS_KG, density_per_m3: boron },
        ];
        if ash > 0.0 {
            ions.push(Species { charge: 2.0, mass_kg: ALPHA_MASS_KG, density_per_m3: ash });
        }
        let mut effective_charge = 0.0;
        for ion in &ions {
            effective_charge += ion.charge * ion.charge * ion.density_per_m3;
        }
        effective_charge /= q.electron_density_per_m3;
        let maxwellian = reaction::reactivity_pb11_m3_per_s(q.ion_temperature_kev)
            .ok_or(ThermalReject::IonTemperatureOutOfRange { ion_temperature_kev: q.ion_temperature_kev })?;
        let reactivity_m3_per_s = if q.kinetic_enhancement {
            maxwellian * kinetic_factor(q.ion_temperature_kev)
        } else {
            maxwellian
        };
        Ok(Self {
            question: q,
            ions,
            ion_density_per_m3: protons + boron + ash,
            effective_charge,
            charge_residual,
            reactivity_m3_per_s,
            reactions_per_m3_s: protons * boron * reactivity_m3_per_s,
            slowing_energy_j: (REACTION_ENERGY_KEV - 1.5 * q.ion_temperature_kev) * JOULE_PER_KEV,
            alpha_birth_energy_kev: (REACTION_ENERGY_KEV + 3.0 * q.ion_temperature_kev) / 3.0,
        })
    }

    /// Pressure of the applied field, pascals.
    fn field_pressure_pa(&self) -> f64 {
        self.question.magnetic_field_t * self.question.magnetic_field_t / (2.0 * VACUUM_PERMEABILITY_H_PER_M)
    }

    fn ion_pressure_pa(&self) -> f64 {
        self.ion_density_per_m3 * self.question.ion_temperature_kev * JOULE_PER_KEV
    }

    /// The highest electron temperature the applied field can hold, keV.
    fn pressure_ceiling_kev(&self) -> f64 {
        (self.field_pressure_pa() - self.ion_pressure_pa()) / (self.question.electron_density_per_m3 * JOULE_PER_KEV)
    }

    /// Shares of the alphas' slowing-down energy to ions and to electrons,
    /// each by its own integral. `electron_share_scale` is one outside tests.
    fn alpha_shares(&self, plasma: &Background, electron_share_scale: f64) -> Result<(f64, f64), ThermalReject> {
        let alpha = Species { charge: 2.0, mass_kg: ALPHA_MASS_KG, density_per_m3: 0.0 };
        let low = 1.5 * self.question.ion_temperature_kev;
        let span = self.alpha_birth_energy_kev - low;
        let width = span / ALPHA_PANELS as f64;
        let half = 0.5 * width;
        let mut to_ions = 0.0;
        let mut to_electrons = 0.0;
        for panel in 0..ALPHA_PANELS {
            let middle = low + (panel as f64 + 0.5) * width;
            for (node, weight) in reaction::GAUSS_NODES.iter().zip(reaction::GAUSS_WEIGHTS) {
                let energy = middle + half * node;
                let electrons = plasma.loss_to_electrons_kev_per_s(&alpha, energy);
                let mut ions = 0.0;
                for ion in &self.ions {
                    ions += plasma.loss_to_ions_kev_per_s(&alpha, energy, ion);
                }
                if !(electrons > 0.0 && ions > 0.0) {
                    return Err(ThermalReject::AlphaSharesBroken { residual: f64::NAN });
                }
                to_ions += half * weight * ions / (ions + electrons);
                to_electrons += half * weight * electrons / (ions + electrons);
            }
        }
        let share_ions = to_ions / span;
        let share_electrons = electron_share_scale * to_electrons / span;
        let residual = share_ions + share_electrons - 1.0;
        if !(residual.abs() <= ALPHA_SHARE_TOLERANCE) {
            return Err(ThermalReject::AlphaSharesBroken { residual });
        }
        Ok((share_ions, share_electrons))
    }

    /// Power from ions to electrons by collisions, W/m³.
    fn exchange_w_per_m3(&self, electron_temperature_kev: f64) -> f64 {
        let q = &self.question;
        let e2 = ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C;
        let four_pi_eps = 4.0 * PI * VACUUM_PERMITTIVITY_F_PER_M;
        let k = 8.0 * (2.0 * PI).sqrt() / 3.0 * e2 * e2 / (four_pi_eps * four_pi_eps);
        let logarithm = reaction::coulomb_logarithm_electron(q.electron_density_per_m3, electron_temperature_kev);
        let te = electron_temperature_kev * JOULE_PER_KEV;
        let ti = q.ion_temperature_kev * JOULE_PER_KEV;
        let mut rate = 0.0;
        for ion in &self.ions {
            let mixed = ion.mass_kg * te + ELECTRON_MASS_KG * ti;
            rate += k * ion.density_per_m3 * ion.charge * ion.charge * logarithm * (ELECTRON_MASS_KG * ion.mass_kg).sqrt()
                / (mixed * mixed.sqrt());
        }
        1.5 * q.electron_density_per_m3 * rate * (ti - te) * reaction::relativistic_exchange_factor(electron_temperature_kev)
    }

    fn terms(&self, electron_temperature_kev: f64, electron_share_scale: f64) -> Result<Terms, ThermalReject> {
        let q = &self.question;
        let plasma = Background::new(q.electron_density_per_m3, electron_temperature_kev, q.ion_temperature_kev);
        let (share_ions, share_electrons) = self.alpha_shares(&plasma, electron_share_scale)?;
        let pressure_pa = self.ion_pressure_pa() + q.electron_density_per_m3 * electron_temperature_kev * JOULE_PER_KEV;
        let applied = q.magnetic_field_t * q.magnetic_field_t;
        let mut inside = applied - 2.0 * VACUUM_PERMEABILITY_H_PER_M * pressure_pa;
        // At the highest temperature the field holds, the field inside is
        // zero; rounding may leave a few parts in 1e16 of the applied field.
        if inside < 0.0 && inside >= -FIELD_ROUNDING * applied {
            inside = 0.0;
        }
        if !(inside >= 0.0) {
            return Err(ThermalReject::PressureExceedsField { pressure_pa });
        }
        let internal_field_t = inside.sqrt();
        let terms = Terms {
            electron_temperature_kev,
            share_ions,
            share_electrons,
            exchange: self.exchange_w_per_m3(electron_temperature_kev),
            bremsstrahlung: bremsstrahlung_svensson_w_per_m3(q.electron_density_per_m3, self.effective_charge, electron_temperature_kev),
            synchrotron: synchrotron_w_per_m3(
                q.electron_density_per_m3,
                electron_temperature_kev,
                internal_field_t,
                q.minor_radius_m,
                q.wall_reflectivity,
            ),
            pressure_pa,
            internal_field_t,
        };
        for value in [terms.exchange, terms.bremsstrahlung, terms.synchrotron] {
            if !value.is_finite() {
                return Err(ThermalReject::NonFiniteValue);
            }
        }
        // Trubnikov's loss is that of an opaque plasma: it may not be more
        // than the electrons emit with nothing absorbed.
        if terms.synchrotron > 0.0 {
            let unabsorbed = unabsorbed_synchrotron_w_per_m3(q.electron_density_per_m3, electron_temperature_kev, internal_field_t);
            let ratio = terms.synchrotron / unabsorbed;
            if !(ratio <= 1.0) {
                return Err(ThermalReject::SynchrotronAboveUnabsorbedEmission { electron_temperature_kev, ratio });
            }
        }
        Ok(terms)
    }

    /// The electron balance, W/m³: heating less radiation.
    fn electron_balance(&self, terms: &Terms) -> f64 {
        self.reactions_per_m3_s * terms.share_electrons * self.slowing_energy_j + terms.exchange
            - terms.bremsstrahlung
            - terms.synchrotron
    }

    /// Holds the terms at one temperature to the two balances and writes the answer.
    fn accept(&self, terms: &Terms, iterations: u32) -> Result<ThermalBalance, ThermalReject> {
        let q = &self.question;
        let s = self.reactions_per_m3_s;
        let ti_j = q.ion_temperature_kev * JOULE_PER_KEV;
        let fusion = s * REACTION_ENERGY_KEV * JOULE_PER_KEV;
        let alpha_to_ions = s * terms.share_ions * self.slowing_energy_j;
        let alpha_to_electrons = s * terms.share_electrons * self.slowing_energy_j;
        let returned = s * 1.5 * ti_j;
        let ion_energy = 1.5 * self.ion_density_per_m3 * ti_j;
        let electron_energy = 1.5 * q.electron_density_per_m3 * terms.electron_temperature_kev * JOULE_PER_KEV;
        let conduction = ion_energy / q.energy_confinement_time_s;
        let ion_surplus = returned + alpha_to_ions - terms.exchange;
        let auxiliary = conduction - ion_surplus;

        let electron_heating = alpha_to_electrons + terms.exchange;
        let electron_residual = self.electron_balance(terms) / electron_heating;
        let total = auxiliary + fusion - terms.bremsstrahlung - terms.synchrotron - conduction;
        // The total is a sum of terms that cancel: where the ions are held
        // for a short time the conduction and the auxiliary power are many
        // times the fusion power and cancel to it, so the residual is taken
        // over the largest term of the sum, the scale of its rounding.
        let mut scale: f64 = 0.0;
        for term in [fusion, alpha_to_ions, alpha_to_electrons, returned, terms.exchange, terms.bremsstrahlung, terms.synchrotron, conduction, auxiliary] {
            scale = scale.max(term.abs());
        }
        let total_residual = (total - self.electron_balance(terms)) / scale;
        for value in [electron_residual, total_residual, auxiliary, fusion] {
            if !value.is_finite() {
                return Err(ThermalReject::NonFiniteValue);
            }
        }
        if !(electron_residual.abs() <= ENERGY_TOLERANCE) {
            return Err(ThermalReject::EnergyBalanceBroken { residual: electron_residual });
        }
        if !(total_residual.abs() <= ENERGY_TOLERANCE) {
            return Err(ThermalReject::EnergyBalanceBroken { residual: total_residual });
        }
        Ok(ThermalBalance {
            electron_temperature_kev: terms.electron_temperature_kev,
            reactivity_m3_per_s: self.reactivity_m3_per_s,
            fusion_power_w_per_m3: fusion,
            bremsstrahlung_power_w_per_m3: terms.bremsstrahlung,
            bremsstrahlung_power_xie_w_per_m3: bremsstrahlung_xie_w_per_m3(
                q.electron_density_per_m3,
                &self.ions,
                terms.electron_temperature_kev,
            ),
            synchrotron_power_w_per_m3: terms.synchrotron,
            ion_electron_power_w_per_m3: terms.exchange,
            alpha_energy_fraction_to_electrons: terms.share_electrons,
            auxiliary_power_w_per_m3: auxiliary,
            ignition_margin: ion_surplus / fusion,
            ignition_loss_rate_per_s: ion_surplus / (ion_energy + electron_energy),
            pressure_pa: terms.pressure_pa,
            beta: terms.pressure_pa / self.field_pressure_pa(),
            internal_field_t: terms.internal_field_t,
            effective_charge: self.effective_charge,
            electron_energy_residual: electron_residual,
            total_energy_residual: total_residual,
            charge_residual: self.charge_residual,
            iterations,
            balance_terms_w_per_m3: vec![
                fusion,
                alpha_to_ions,
                alpha_to_electrons,
                returned,
                terms.exchange,
                terms.bremsstrahlung,
                terms.synchrotron,
                conduction,
                auxiliary,
            ],
        })
    }
}

/// The energy balance of a uniform thermal proton-boron-11 plasma.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &ThermalQuestion) -> Result<ThermalBalance, ThermalReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &ThermalQuestion) -> Result<ThermalBalance, ThermalReject> {
    solve_with(question, MAX_ITERATIONS, 1.0)
}

impl NormalOrZero for ThermalBalance {
    fn normal_or_zero(&mut self) {
        self.electron_temperature_kev.normal_or_zero();
        self.reactivity_m3_per_s.normal_or_zero();
        self.fusion_power_w_per_m3.normal_or_zero();
        self.bremsstrahlung_power_w_per_m3.normal_or_zero();
        self.bremsstrahlung_power_xie_w_per_m3.normal_or_zero();
        self.synchrotron_power_w_per_m3.normal_or_zero();
        self.ion_electron_power_w_per_m3.normal_or_zero();
        self.alpha_energy_fraction_to_electrons.normal_or_zero();
        self.auxiliary_power_w_per_m3.normal_or_zero();
        self.ignition_margin.normal_or_zero();
        self.ignition_loss_rate_per_s.normal_or_zero();
        self.pressure_pa.normal_or_zero();
        self.beta.normal_or_zero();
        self.internal_field_t.normal_or_zero();
        self.effective_charge.normal_or_zero();
        self.electron_energy_residual.normal_or_zero();
        self.total_energy_residual.normal_or_zero();
        self.charge_residual.normal_or_zero();
        self.balance_terms_w_per_m3.normal_or_zero();
    }
}

fn solve_with(question: &ThermalQuestion, iteration_cap: u32, electron_share_scale: f64) -> Result<ThermalBalance, ThermalReject> {
    admit(question)?;
    settle(question, iteration_cap, electron_share_scale)
}

/// The balance of a question already admitted. Tests call it outside the
/// compared envelope to hold the search itself.
fn settle(question: &ThermalQuestion, iteration_cap: u32, electron_share_scale: f64) -> Result<ThermalBalance, ThermalReject> {
    let plasma = Plasma::new(question)?;
    // Pressure balance: the ions alone must leave room under the applied field.
    let ceiling = plasma.pressure_ceiling_kev();
    if !(ceiling > ELECTRON_TEMPERATURE_MIN_KEV) {
        return Err(ThermalReject::PressureExceedsField { pressure_pa: plasma.ion_pressure_pa() });
    }
    let mut low = ELECTRON_TEMPERATURE_MIN_KEV;
    let mut high = question.ion_temperature_kev.min(ELECTRON_TEMPERATURE_MAX_KEV);
    let held_by_field = ceiling < high;
    if held_by_field {
        high = ceiling;
    }
    // The electrons must gain at the cold end of the range.
    let at_low = plasma.electron_balance(&plasma.terms(low, electron_share_scale)?);
    if !(at_low > 0.0) {
        return Err(ThermalReject::ElectronTemperatureOutOfRange { at_or_below_floor: true });
    }
    // The balance need not fall with temperature: sample it on the fixed
    // grid and keep the first interval where it turns from heating to loss.
    let top = high;
    let mut point = 0_u64;
    let mut found = false;
    loop {
        point += 1;
        let grid = ELECTRON_TEMPERATURE_MIN_KEV + point as f64 * ELECTRON_GRID_STEP_KEV;
        let at = if grid < top { grid } else { top };
        let balance = plasma.electron_balance(&plasma.terms(at, electron_share_scale)?);
        if !balance.is_finite() {
            return Err(ThermalReject::NonFiniteValue);
        }
        if !(balance > 0.0) {
            high = at;
            found = true;
            break;
        }
        low = at;
        if !(grid < top) {
            break;
        }
    }
    if !found {
        if held_by_field {
            return Err(ThermalReject::PressureExceedsField { pressure_pa: plasma.field_pressure_pa() });
        }
        return Err(ThermalReject::ElectronTemperatureOutOfRange { at_or_below_floor: false });
    }
    let mut iterations = 0;
    loop {
        let middle = 0.5 * (low + high);
        if !(middle > low && middle < high) {
            // The bracket is two neighbouring numbers: the temperature is settled.
            let lower = plasma.terms(low, electron_share_scale)?;
            let upper = plasma.terms(high, electron_share_scale)?;
            let closer = if plasma.electron_balance(&lower).abs() <= plasma.electron_balance(&upper).abs() {
                lower
            } else {
                upper
            };
            return plasma.accept(&closer, iterations);
        }
        if iterations >= iteration_cap {
            return Err(ThermalReject::NotSettled { iterations });
        }
        iterations += 1;
        let balance = plasma.electron_balance(&plasma.terms(middle, electron_share_scale)?);
        if !balance.is_finite() {
            return Err(ThermalReject::NonFiniteValue);
        }
        if balance > 0.0 {
            low = middle;
        } else {
            high = middle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ochs et al. 2022 as read 2026-10-09 ([`REFERENCE_RETRIEVED`]) from
    /// <https://arxiv.org/abs/2210.08076>: ion density 1e14 cm⁻³, ion
    /// temperature 300 keV, no synchrotron loss, the kinetic factor on.
    /// Published: electron temperature 163 keV at 13 % boron; fusion power
    /// at most 2.8 % above bremsstrahlung at 15 % boron, near 300 keV. These
    /// are calculations, not measurements.
    fn ochs(ion_temperature_kev: f64, boron_fraction: f64) -> ThermalQuestion {
        ThermalQuestion {
            ion_temperature_kev,
            electron_density_per_m3: 1.0e20 * (1.0 + 4.0 * boron_fraction),
            boron_fraction,
            ash_fraction: 0.0,
            magnetic_field_t: 20.0,
            minor_radius_m: 2.0,
            wall_reflectivity: 1.0,
            energy_confinement_time_s: 100.0,
            kinetic_enhancement: true,
        }
    }

    #[test]
    fn ochs_2022_electron_temperature_and_ignition_margin() {
        println!("Ochs et al. 2022 read {REFERENCE_RETRIEVED} from {REFERENCE_URL}");
        let thirteen = solve(&ochs(300.0, 0.13)).expect("solves");
        let off = thirteen.electron_temperature_kev / 163.0 - 1.0;
        println!("13 % boron, 300 keV: Te {} keV against 163, {off:+.4}", thirteen.electron_temperature_kev);
        assert!(off.abs() <= REFERENCE_TEMPERATURE_TOLERANCE, "{off}");
        let mut best: f64 = 0.0;
        let mut best_at = 0.0;
        for step in 20..=45 {
            let ti = 10.0 * step as f64;
            let balance = solve(&ochs(ti, 0.15)).expect("solves");
            let ratio = balance.fusion_power_w_per_m3 / balance.bremsstrahlung_power_w_per_m3;
            if ratio > best {
                best = ratio;
                best_at = ti;
            }
        }
        let off = best / 1.028 - 1.0;
        println!("15 % boron: fusion over bremsstrahlung at most {best} at {best_at} keV against 1.028 near 300, {off:+.4}");
        assert!(off.abs() <= REFERENCE_RATIO_TOLERANCE, "{off}");
        assert!((best_at / 300.0 - 1.0).abs() <= 0.10, "{best_at}");
        // The total-energy confinement time at which this plasma holds itself: printed, not held.
        let fifteen = solve(&ochs(300.0, 0.15)).expect("solves");
        println!(
            "total-energy ignition confinement time: {} s at 15 % (published 550), {} s at 13 % (published 460)",
            1.0 / fifteen.ignition_loss_rate_per_s,
            1.0 / thirteen.ignition_loss_rate_per_s
        );
    }

    #[test]
    fn energy_and_charge_close() {
        let balance = solve(&ochs(300.0, 0.15)).expect("solves");
        assert!(balance.electron_energy_residual.abs() <= ENERGY_TOLERANCE);
        assert!(balance.total_energy_residual.abs() <= ENERGY_TOLERANCE);
        assert!(balance.charge_residual.abs() <= CHARGE_TOLERANCE);
        let t = &balance.balance_terms_w_per_m3;
        assert_eq!(t.len(), 9);
        // Electrons: alpha heating and exchange against the two radiation losses.
        assert!(((t[2] + t[4] - t[5] - t[6]) / t[0]).abs() < 1.0e-11);
        // Ions: auxiliary, returned reactant energy and alpha heating against exchange and conduction.
        assert!(((t[8] + t[3] + t[1] - t[4] - t[7]) / t[0]).abs() < 1.0e-11);
        // The fusion energy is the alpha heating and the returned reactant energy.
        assert!(((t[1] + t[2] + t[3]) / t[0] - 1.0).abs() < 1.0e-11);
        assert!((balance.effective_charge - (0.85 + 25.0 * 0.15) / 1.6).abs() < 1.0e-12);
        assert!(balance.electron_temperature_kev > 100.0 && balance.electron_temperature_kev < 200.0);
        // About a tenth of the alpha energy goes to electrons (Putvinski 2019).
        assert!((balance.alpha_energy_fraction_to_electrons - 0.10).abs() < 0.05);
        // The two published bremsstrahlung fits differ by about two percent here.
        let spread = balance.bremsstrahlung_power_xie_w_per_m3 / balance.bremsstrahlung_power_w_per_m3 - 1.0;
        assert!(spread > 0.0 && spread < 0.04, "{spread}");
    }

    #[test]
    fn the_bremsstrahlung_coefficient_is_the_formulary_value() {
        // 1.69e-32 W cm³ eV^-½ (NRL Formulary) and 5.34e-37 W m³ keV^-½ (Nevins 1998), within half a percent.
        let a0 = bremsstrahlung_coefficient_w_m3();
        assert!((a0 / 1.21046e-35 - 1.0).abs() < 1.0e-5, "{a0}");
        let per_root_kev = a0 / ELECTRON_REST_KEV.sqrt();
        assert!((per_root_kev / 5.34e-37 - 1.0).abs() < 0.005, "{per_root_kev}");
        let per_root_ev_cm3 = a0 * 1.0e6 / (ELECTRON_REST_KEV * 1.0e3).sqrt();
        assert!((per_root_ev_cm3 / 1.69e-32 - 1.0).abs() < 0.005, "{per_root_ev_cm3}");
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let first = solve(&ochs(250.0, 0.13)).expect("solves");
        let second = solve(&ochs(250.0, 0.13)).expect("solves");
        assert_eq!(first.electron_temperature_kev.to_bits(), second.electron_temperature_kev.to_bits());
        assert_eq!(first.iterations, second.iterations);
        for (left, right) in first.balance_terms_w_per_m3.iter().zip(&second.balance_terms_w_per_m3) {
            assert_eq!(left.to_bits(), right.to_bits());
        }
    }

    #[test]
    fn synchrotron_cools_the_electrons_and_the_wall_decides_how_much() {
        let mut question = ochs(300.0, 0.15);
        question.kinetic_enhancement = false;
        question.electron_density_per_m3 = 1.6e20;
        question.magnetic_field_t = 6.661098700751347;
        let none = solve(&question).expect("solves");
        assert_eq!(none.synchrotron_power_w_per_m3, 0.0);
        question.wall_reflectivity = 0.95;
        let some = solve(&question).expect("solves");
        question.wall_reflectivity = 0.0;
        let bare = solve(&question).expect("a bare wall is answered");
        assert!(some.electron_temperature_kev < none.electron_temperature_kev);
        assert!(bare.electron_temperature_kev < some.electron_temperature_kev);
        // At 5 T inside, 2 m and 95 % reflection the synchrotron loss exceeds the fusion power.
        assert!((some.internal_field_t - 5.0).abs() < 0.05, "{}", some.internal_field_t);
        assert!(some.synchrotron_power_w_per_m3 > some.fusion_power_w_per_m3);
        assert!(some.ignition_margin < 0.0 && some.ignition_loss_rate_per_s < 0.0);
        assert!(some.beta > 0.0 && some.beta < 1.0);
        // Ash and the kinetic switch move the margin the way they must; ash, never compared, is answered.
        let clean = solve(&ochs(300.0, 0.15)).expect("solves");
        let mut ashy = ochs(300.0, 0.15);
        ashy.ash_fraction = 0.02;
        assert!(solve(&ashy).expect("ash is answered").ignition_margin < clean.ignition_margin);
        let mut maxwellian = ochs(300.0, 0.15);
        maxwellian.kinetic_enhancement = false;
        let maxwellian = solve(&maxwellian).expect("solves");
        assert!(maxwellian.ignition_margin < clean.ignition_margin);
        assert!((clean.reactivity_m3_per_s / maxwellian.reactivity_m3_per_s - kinetic_factor(300.0)).abs() < 1.0e-12);
        assert!((kinetic_factor(300.0) - 1.091_428_571_428_571_4).abs() < 1.0e-12);
        assert_eq!(kinetic_factor(800.0), 1.0);
    }

    /// Each end of the declared range is answered or stopped by a physical
    /// limit inside the solve, never at the door; the next number beyond it
    /// is refused at the door by name. The corner of the range 0.6.0 declared
    /// and a 10 keV plasma, both refused by name in 0.7.0 to this version,
    /// are answered with their ledgers closed.
    #[test]
    fn each_end_of_the_ranges_is_answered_and_the_next_number_is_named() {
        let below = |value: f64| if value == 0.0 { -f64::MIN_POSITIVE } else { f64::from_bits(value.to_bits() - 1) };
        let above = |value: f64| f64::from_bits(value.to_bits() + 1);
        let good = ochs(300.0, 0.15);
        let with = |change: fn(&mut ThermalQuestion, f64), value: f64| {
            let mut question = good;
            change(&mut question, value);
            solve(&question)
        };
        let inside_the_solve = |answer: Result<ThermalBalance, ThermalReject>| match answer {
            Ok(balance) => balance.total_energy_residual.abs() <= ENERGY_TOLERANCE && balance.electron_energy_residual.abs() <= ENERGY_TOLERANCE,
            Err(reject) => matches!(
                reject,
                ThermalReject::PressureExceedsField { .. }
                    | ThermalReject::ElectronTemperatureOutOfRange { .. }
                    | ThermalReject::SynchrotronAboveUnabsorbedEmission { .. }
            ),
        };
        type Change = fn(&mut ThermalQuestion, f64);
        let closed: [(Change, f64, f64, fn(f64) -> ThermalReject); 6] = [
            (|q, v| q.ion_temperature_kev = v, ION_TEMPERATURE_MIN_KEV, ION_TEMPERATURE_MAX_KEV, |v| ThermalReject::IonTemperatureOutOfRange { ion_temperature_kev: v }),
            (|q, v| q.electron_density_per_m3 = v, ELECTRON_DENSITY_MIN_PER_M3, ELECTRON_DENSITY_MAX_PER_M3, |v| ThermalReject::ElectronDensityOutOfRange { electron_density_per_m3: v }),
            (|q, v| q.boron_fraction = v, BORON_FRACTION_MIN, BORON_FRACTION_MAX, |v| ThermalReject::BoronFractionOutOfRange { boron_fraction: v }),
            (|q, v| q.ash_fraction = v, 0.0, ASH_FRACTION_MAX, |v| ThermalReject::AshFractionOutOfRange { ash_fraction: v }),
            (|q, v| q.minor_radius_m = v, MINOR_RADIUS_MIN_M, MINOR_RADIUS_MAX_M, |v| ThermalReject::MinorRadiusOutOfRange { minor_radius_m: v }),
            (|q, v| q.wall_reflectivity = v, WALL_REFLECTIVITY_MIN, WALL_REFLECTIVITY_MAX, |v| ThermalReject::WallReflectivityOutOfRange { wall_reflectivity: v }),
        ];
        for (change, low, high, reject) in closed {
            assert!(inside_the_solve(with(change, low)), "{low}");
            assert!(inside_the_solve(with(change, high)), "{high}");
            assert_eq!(with(change, below(low)), Err(reject(below(low))));
            assert_eq!(with(change, above(high)), Err(reject(above(high))));
        }
        // The field and the confinement time: anything above zero, the field to 1e6 T.
        let open: [(Change, f64, fn(f64) -> ThermalReject); 2] = [
            (|q, v| q.magnetic_field_t = v, MAGNETIC_FIELD_MAX_T, |v| ThermalReject::MagneticFieldOutOfRange { magnetic_field_t: v }),
            (|q, v| q.energy_confinement_time_s = v, CONFINEMENT_TIME_MAX_S, |v| ThermalReject::ConfinementTimeOutOfRange { energy_confinement_time_s: v }),
        ];
        for (change, top, reject) in open {
            for value in [1.0e-3, 1.0e6, top] {
                assert!(inside_the_solve(with(change, value)), "{value}");
            }
            for value in [0.0, -0.0, -1.0] {
                assert_eq!(with(change, value), Err(reject(value)));
            }
        }
        assert_eq!(with(|q, v| q.magnetic_field_t = v, above(MAGNETIC_FIELD_MAX_T)), Err(ThermalReject::MagneticFieldOutOfRange { magnetic_field_t: above(MAGNETIC_FIELD_MAX_T) }));
        // The field nearest zero cannot hold the ions, by name; at the
        // confinement time nearest zero the power that holds the ions is not
        // a finite number.
        assert!(matches!(with(|q, v| q.magnetic_field_t = v, f64::from_bits(1)), Err(ThermalReject::PressureExceedsField { .. })));
        assert_eq!(with(|q, v| q.energy_confinement_time_s = v, f64::from_bits(1)), Err(ThermalReject::NonFiniteValue));
        assert_eq!(with(|q, v| q.ion_temperature_kev = v, f64::NAN), Err(ThermalReject::NonFiniteValue));
        assert_eq!(with(|q, v| q.magnetic_field_t = v, f64::INFINITY), Err(ThermalReject::NonFiniteValue));
        // A corner of the range 0.6.0 declared and a 10 keV plasma.
        let corner = ThermalQuestion {
            ion_temperature_kev: 300.0,
            electron_density_per_m3: 1.0e21,
            boron_fraction: 0.5,
            ash_fraction: 0.1,
            magnetic_field_t: 40.0,
            minor_radius_m: 10.0,
            wall_reflectivity: 0.0,
            energy_confinement_time_s: 1.0e5,
            kinetic_enhancement: true,
        };
        for question in [corner, ThermalQuestion { ion_temperature_kev: 10.0, ..good }] {
            let balance = solve(&question).expect("answered");
            assert!(balance.electron_energy_residual.abs() <= ENERGY_TOLERANCE);
            assert!(balance.total_energy_residual.abs() <= ENERGY_TOLERANCE);
            assert!(balance.electron_temperature_kev < question.ion_temperature_kev);
        }
    }

    /// Every run of the comparison is inside the declared range, and the
    /// electron temperatures of five of them are the bits 0.6.0 sealed
    /// (`evidence/research/fusion_pb11/solver/runs_v060`): the grid moved
    /// the iteration count, not the answer.
    #[test]
    fn every_compared_run_is_answered_with_the_temperature_it_had() {
        let b8 = |magnetic_field_t: f64, kinetic_enhancement: bool| ThermalQuestion {
            ion_temperature_kev: 300.0,
            electron_density_per_m3: 1.6e20,
            boron_fraction: 0.15,
            ash_fraction: 0.0,
            magnetic_field_t,
            minor_radius_m: 2.0,
            wall_reflectivity: 0.95,
            energy_confinement_time_s: 100.0,
            kinetic_enhancement,
        };
        for step in 20..=45 {
            assert!(solve(&ochs(10.0 * step as f64, 0.15)).is_ok(), "{step}");
        }
        for (question, bits) in [
            (ochs(300.0, 0.13), 0x4063eaafd5885f7d_u64),
            (ochs(200.0, 0.15), 0x405e24f20696ce30),
            (ochs(450.0, 0.15), 0x40688d8ec40b259e),
            (b8(6.661098700751347, false), 0x405c49c0e7914168),
            (b8(6.66210246751073, true), 0x405c5709c86d0e71),
        ] {
            let balance = solve(&question).expect("a compared run");
            assert_eq!(balance.electron_temperature_kev.to_bits(), bits, "{}", balance.electron_temperature_kev);
        }
    }

    /// Pressure balance: 300 keV ions at 1.6e20 electrons per cubic metre
    /// need 4.7 T at the least; 3 T and 4 T cannot hold the ions alone, and
    /// the solve says so by name (0.7.0 to this version refused both at the
    /// door, below the compared fields).
    #[test]
    fn a_plasma_the_field_cannot_hold_is_named() {
        let mut question = ochs(300.0, 0.15);
        question.electron_density_per_m3 = 1.6e20;
        for field in [3.0, 4.0] {
            question.magnetic_field_t = field;
            assert!(matches!(solve(&question), Err(ThermalReject::PressureExceedsField { .. })));
        }
        question.magnetic_field_t = 20.0;
        assert!(solve(&question).is_ok());
    }

    /// The field holds a plasma near its limit. At 4.7 T, 0.1 m and a bare
    /// wall, 0.6.0 refused with `PressureExceedsField`: the field inside is
    /// zero at the limit, so the synchrotron loss is too, and the balance
    /// there heats the electrons although it turns to loss near 105.6 keV
    /// below it. The balance turns from heating to loss, then back to
    /// heating below the limit; the first turn is returned.
    #[test]
    fn a_balance_below_the_field_limit_is_found_and_the_first_is_returned() {
        let question = |magnetic_field_t: f64| ThermalQuestion {
            ion_temperature_kev: 300.0,
            electron_density_per_m3: 1.6e20,
            boron_fraction: 0.15,
            ash_fraction: 0.0,
            magnetic_field_t,
            minor_radius_m: 0.1,
            wall_reflectivity: 0.0,
            energy_confinement_time_s: 100.0,
            kinetic_enhancement: false,
        };
        let held = solve(&question(4.7)).expect("a balance the field holds");
        assert!((held.electron_temperature_kev - 105.613).abs() < 0.01, "{}", held.electron_temperature_kev);
        assert!((held.beta - 0.8549).abs() < 0.001, "{}", held.beta);
        assert!(held.internal_field_t > 1.7 && held.internal_field_t < 1.9);
        assert!(held.electron_energy_residual.abs() <= ENERGY_TOLERANCE);
        let plasma = Plasma::new(&question(4.7)).expect("built");
        let ceiling = plasma.pressure_ceiling_kev();
        let balance = |t: f64| plasma.electron_balance(&plasma.terms(t, 1.0).expect("terms"));
        // Heating everywhere below the answer on a grid ten times finer than the solver's, loss above it, and heating again at the limit.
        let mut t = ELECTRON_TEMPERATURE_MIN_KEV;
        while t < held.electron_temperature_kev - 0.025 {
            assert!(balance(t) > 0.0, "{t}");
            t += 0.025;
        }
        assert!(balance(120.0) < 0.0);
        assert!(balance(ceiling) > 0.0);
        // One part in ten thousand of the field on either side gives nearly the same answer; at 4.7007 T the limit is reached by rounding and is not refused.
        for field in [4.7007, 4.701] {
            let near = solve(&question(field)).expect("settles");
            assert!((near.electron_temperature_kev - held.electron_temperature_kev).abs() < 0.2, "{field}");
        }
        // The refusal stays where the balance heats at every point up to the limit, on the solver's grid and on one twenty-five times finer.
        assert!(matches!(solve(&question(4.6)), Err(ThermalReject::PressureExceedsField { .. })));
        let plasma = Plasma::new(&question(4.6)).expect("built");
        let ceiling = plasma.pressure_ceiling_kev();
        let mut t = ELECTRON_TEMPERATURE_MIN_KEV;
        while t < ceiling {
            assert!(plasma.electron_balance(&plasma.terms(t, 1.0).expect("terms")) > 0.0, "{t}");
            t += 0.01;
        }
    }

    /// The ignition loss rate is over the stored energy of ions and
    /// electrons. The confinement time of the question is of the ions alone:
    /// the auxiliary power is the ion stored energy over it less the surplus,
    /// so it is zero at the ion stored energy over the surplus, not at one
    /// over the rate.
    #[test]
    fn the_ignition_loss_rate_is_over_the_energy_of_ions_and_electrons() {
        let base = ochs(300.0, 0.15);
        let first = solve(&base).expect("solves");
        let plasma = Plasma::new(&base).expect("built");
        let ion_energy = 1.5 * plasma.ion_density_per_m3 * base.ion_temperature_kev * JOULE_PER_KEV;
        let electron_energy = 1.5 * base.electron_density_per_m3 * first.electron_temperature_kev * JOULE_PER_KEV;
        let surplus = first.ignition_loss_rate_per_s * (ion_energy + electron_energy);
        let expected = ion_energy / base.energy_confinement_time_s - surplus;
        assert!(((first.auxiliary_power_w_per_m3 - expected) / first.fusion_power_w_per_m3).abs() < 1.0e-12);
        assert!((1.0 / first.ignition_loss_rate_per_s - 532.3).abs() < 0.5);
        // Other confinement times, never compared, are answered: the zero falls where the note says.
        let at_ion_time = solve(&ThermalQuestion { energy_confinement_time_s: ion_energy / surplus, ..base }).expect("solves");
        assert!((at_ion_time.auxiliary_power_w_per_m3 / first.fusion_power_w_per_m3).abs() < 1.0e-12, "{}", at_ion_time.auxiliary_power_w_per_m3);
        let at_total_time = solve(&ThermalQuestion { energy_confinement_time_s: 1.0 / first.ignition_loss_rate_per_s, ..base }).expect("solves");
        assert!(at_total_time.auxiliary_power_w_per_m3 < 0.0);
    }

    /// Attack: a search stopped before the temperature has settled is named.
    #[test]
    fn a_search_stopped_early_is_named() {
        assert_eq!(
            solve_with(&ochs(300.0, 0.15), 10, 1.0),
            Err(ThermalReject::NotSettled { iterations: 10 })
        );
    }

    /// Attack: alpha energy that reaches no one. A thousandth of the
    /// electrons' share dropped is named at the first trial temperature.
    #[test]
    fn alpha_energy_given_to_no_one_is_named() {
        match solve_with(&ochs(300.0, 0.15), MAX_ITERATIONS, 0.999) {
            Err(ThermalReject::AlphaSharesBroken { residual }) => assert!(residual < -1.0e-5, "{residual}"),
            other => panic!("accepted lost alpha energy: {:?}", other.is_ok()),
        }
    }

    /// Attack: a falsified temperature. The terms of a temperature one part
    /// in a million off the settled one do not balance, and are named.
    #[test]
    fn a_falsified_temperature_is_named() {
        let question = ochs(300.0, 0.15);
        let plasma = Plasma::new(&question).expect("built");
        let settled = solve(&question).expect("solves");
        let good = plasma.terms(settled.electron_temperature_kev, 1.0).expect("terms");
        assert!(plasma.accept(&good, settled.iterations).is_ok());
        let off = plasma.terms(settled.electron_temperature_kev * 1.000_001, 1.0).expect("terms");
        assert!(matches!(plasma.accept(&off, 0), Err(ThermalReject::EnergyBalanceBroken { .. })));
        // Exchange used once for the electrons and changed for the ions is caught by the electron balance too.
        let mut twice = good;
        twice.exchange *= 1.000_001;
        assert!(matches!(plasma.accept(&twice, 0), Err(ThermalReject::EnergyBalanceBroken { .. })));
        // Shares that no longer add to one break the total.
        let mut lost = good;
        lost.share_ions *= 0.999;
        assert!(matches!(plasma.accept(&lost, 0), Err(ThermalReject::EnergyBalanceBroken { .. })));
    }

    /// Every corner of the compared envelope settles above the floor and
    /// below the ion temperature, with both balances closed.
    #[test]
    fn the_corners_of_the_compared_envelope_settle() {
        for corner in 0..64_u32 {
            let pick = |bit: u32, low: f64, high: f64| if corner & (1 << bit) == 0 { low } else { high };
            let question = ThermalQuestion {
                ion_temperature_kev: pick(0, 200.0, 450.0),
                electron_density_per_m3: pick(1, 1.52e20, 1.6e20),
                boron_fraction: pick(2, 0.13, 0.15),
                ash_fraction: 0.0,
                magnetic_field_t: pick(3, 6.661098700751347, 20.0),
                minor_radius_m: 2.0,
                wall_reflectivity: pick(4, 0.95, 1.0),
                energy_confinement_time_s: 100.0,
                kinetic_enhancement: corner & (1 << 5) != 0,
            };
            let balance = solve(&question).unwrap_or_else(|reject| panic!("{corner}: {reject}"));
            assert!(balance.electron_temperature_kev > ELECTRON_TEMPERATURE_MIN_KEV);
            assert!(balance.electron_temperature_kev < question.ion_temperature_kev);
            assert!(balance.electron_energy_residual.abs() <= ENERGY_TOLERANCE);
            assert!(balance.total_energy_residual.abs() <= ENERGY_TOLERANCE);
        }
    }

    /// Attack on the reopened range: at every corner of it, every input at
    /// one end or the other (ion temperature 1 and 1000 keV, electron
    /// density 1e18 and 1e22, boron 0.01 and 0.9, ash 0 and 1, applied field
    /// 1 and 1000 T, radius 0.1 and 10 m, wall 0 and 1, confinement 1 ms and
    /// 1e6 s, the kinetic factor off and on), the balance closes or a
    /// physical limit stops it by name: never a broken ledger, a search that
    /// did not settle, or a value that is not finite. When written: 160 of
    /// the 512 corners answered, all at 1000 keV; at 1 keV, the electrons'
    /// floor, the search for an electron temperature is empty and every
    /// corner stops by name. With the low end at 10 keV, until this version,
    /// 320 were answered, 160 of them at 10 keV. The same corners at 1.25 keV
    /// are attacked in `ion_temperatures_down_to_the_electron_floor_settle`.
    #[test]
    fn every_corner_of_the_reopened_range_closes_or_stops_by_name() {
        let mut answered = 0;
        let mut worst: f64 = 0.0;
        for corner in 0..512_u32 {
            let pick = |bit: u32, low: f64, high: f64| if corner & (1 << bit) == 0 { low } else { high };
            let question = ThermalQuestion {
                ion_temperature_kev: pick(0, ION_TEMPERATURE_MIN_KEV, ION_TEMPERATURE_MAX_KEV),
                electron_density_per_m3: pick(1, ELECTRON_DENSITY_MIN_PER_M3, ELECTRON_DENSITY_MAX_PER_M3),
                boron_fraction: pick(2, BORON_FRACTION_MIN, BORON_FRACTION_MAX),
                ash_fraction: pick(3, 0.0, ASH_FRACTION_MAX),
                magnetic_field_t: pick(4, 1.0, 1000.0),
                minor_radius_m: pick(5, MINOR_RADIUS_MIN_M, MINOR_RADIUS_MAX_M),
                wall_reflectivity: pick(6, WALL_REFLECTIVITY_MIN, WALL_REFLECTIVITY_MAX),
                energy_confinement_time_s: pick(7, 1.0e-3, 1.0e6),
                kinetic_enhancement: corner & (1 << 8) != 0,
            };
            match solve(&question) {
                Ok(balance) => {
                    answered += 1;
                    assert!(balance.electron_energy_residual.abs() <= ENERGY_TOLERANCE, "{corner}");
                    assert!(balance.total_energy_residual.abs() <= ENERGY_TOLERANCE, "{corner}");
                    assert!(balance.charge_residual.abs() <= CHARGE_TOLERANCE, "{corner}");
                    worst = worst.max(balance.total_energy_residual.abs());
                }
                Err(ThermalReject::PressureExceedsField { .. } | ThermalReject::ElectronTemperatureOutOfRange { .. } | ThermalReject::SynchrotronAboveUnabsorbedEmission { .. }) => {}
                Err(other) => panic!("{corner}: {question:?} {other:?}"),
            }
        }
        println!("{answered} of 512 corners answered; worst total residual {worst:e}");
        assert_eq!(answered, 160);
    }

    /// Above 500 keV, where 0.7.0 to this version refused, the balance at
    /// Ochs et al.'s plasma settles at every 10 keV up to 1000 keV with both
    /// ledgers closed, the electron temperature rising with the ion
    /// temperature and staying below the 500 keV of the radiation forms.
    #[test]
    fn ion_temperatures_to_1000_kev_settle_with_both_ledgers_closed() {
        let mut previous = 0.0;
        for step in 1..=100 {
            let ti = 10.0 * step as f64;
            let balance = solve(&ochs(ti, 0.15)).unwrap_or_else(|reject| panic!("{ti}: {reject}"));
            assert!(balance.electron_energy_residual.abs() <= ENERGY_TOLERANCE, "{ti}");
            assert!(balance.total_energy_residual.abs() <= ENERGY_TOLERANCE, "{ti}");
            assert!(balance.electron_temperature_kev > previous && balance.electron_temperature_kev < ti.min(ELECTRON_TEMPERATURE_MAX_KEV), "{ti}");
            previous = balance.electron_temperature_kev;
        }
        assert!((previous - 293.97).abs() < 0.01, "{previous}");
        assert_eq!(solve(&ochs(1000.000_000_000_1, 0.15)), Err(ThermalReject::IonTemperatureOutOfRange { ion_temperature_kev: 1000.000_000_000_1 }));
    }

    /// Below 10 keV, where engine 0.11.0 refused, down to the 1 keV floor
    /// the electrons are sought from: Ochs et al.'s plasma settles at every
    /// quarter keV from 1.25 to 10 keV with both ledgers closed and the
    /// electrons just below the ions; at 1 keV the search is empty and stops
    /// by name, and below it the door refuses. Every corner of the declared
    /// range at 1.25 keV closes or stops by a physical limit by name.
    #[test]
    fn ion_temperatures_down_to_the_electron_floor_settle() {
        let mut previous: f64 = 0.0;
        for quarter in 5..=40 {
            let ti = 0.25 * f64::from(quarter);
            let balance = solve(&ochs(ti, 0.15)).unwrap_or_else(|reject| panic!("{ti}: {reject}"));
            assert!(balance.electron_energy_residual.abs() <= ENERGY_TOLERANCE, "{ti}");
            assert!(balance.total_energy_residual.abs() <= ENERGY_TOLERANCE, "{ti}");
            assert!(balance.charge_residual.abs() <= CHARGE_TOLERANCE, "{ti}");
            assert!(balance.electron_temperature_kev > previous.max(ELECTRON_TEMPERATURE_MIN_KEV) && balance.electron_temperature_kev < ti, "{ti}");
            previous = balance.electron_temperature_kev;
        }
        assert!(matches!(solve(&ochs(ION_TEMPERATURE_MIN_KEV, 0.15)), Err(ThermalReject::ElectronTemperatureOutOfRange { .. })));
        let below = f64::from_bits(ION_TEMPERATURE_MIN_KEV.to_bits() - 1);
        assert_eq!(solve(&ochs(below, 0.15)), Err(ThermalReject::IonTemperatureOutOfRange { ion_temperature_kev: below }));
        let mut answered = 0;
        for corner in 0..256_u32 {
            let pick = |bit: u32, low: f64, high: f64| if corner & (1 << bit) == 0 { low } else { high };
            let question = ThermalQuestion {
                ion_temperature_kev: 1.25,
                electron_density_per_m3: pick(0, ELECTRON_DENSITY_MIN_PER_M3, ELECTRON_DENSITY_MAX_PER_M3),
                boron_fraction: pick(1, BORON_FRACTION_MIN, BORON_FRACTION_MAX),
                ash_fraction: pick(2, 0.0, ASH_FRACTION_MAX),
                magnetic_field_t: pick(3, 1.0, 1000.0),
                minor_radius_m: pick(4, MINOR_RADIUS_MIN_M, MINOR_RADIUS_MAX_M),
                wall_reflectivity: pick(5, WALL_REFLECTIVITY_MIN, WALL_REFLECTIVITY_MAX),
                energy_confinement_time_s: pick(6, 1.0e-3, 1.0e6),
                kinetic_enhancement: corner & (1 << 7) != 0,
            };
            match solve(&question) {
                Ok(balance) => {
                    answered += 1;
                    assert!(balance.electron_energy_residual.abs() <= ENERGY_TOLERANCE, "{corner}");
                    assert!(balance.total_energy_residual.abs() <= ENERGY_TOLERANCE, "{corner}");
                    assert!(balance.charge_residual.abs() <= CHARGE_TOLERANCE, "{corner}");
                }
                Err(ThermalReject::PressureExceedsField { .. } | ThermalReject::ElectronTemperatureOutOfRange { .. } | ThermalReject::SynchrotronAboveUnabsorbedEmission { .. }) => {}
                Err(other) => panic!("{corner}: {question:?} {other:?}"),
            }
        }
        println!("{answered} of 256 corners at 1.25 keV answered");
        assert!(answered > 0);
    }

    /// The unabsorbed synchrotron emission has the published coefficient,
    /// 6.2116e-17 W/m³ per T² per m⁻³ per keV, and Trubnikov's loss over it
    /// is the transparency factor `60 t^1.5 √((1 − R_w)/p_a)`, with
    /// `p_a = 6036 a n₂₀ / B` and the two relativistic brackets, within half a
    /// percent: the two formulas this limit sets side by side are the ones
    /// the research record re-derived.
    #[test]
    fn the_unabsorbed_emission_is_the_published_one_and_bounds_trubnikov() {
        assert!((unabsorbed_emission_coefficient() / 6.2116e-17 - 1.0).abs() < 1.0e-4, "{}", unabsorbed_emission_coefficient());
        for (n, te, b, a, rw) in [(1.6e20_f64, 113.0_f64, 5.0_f64, 2.0_f64, 0.95_f64), (1.0e18, 200.0, 1000.0, 0.1, 0.9), (1.0e22, 50.0, 40.0, 10.0, 0.0)] {
            let t = te / ELECTRON_REST_KEV;
            let opacity = 6036.0 * a * (n / 1.0e20) / b;
            let factor = 60.0 * t.powf(1.5) * ((1.0 - rw) / opacity).sqrt() * (1.0 + 2.5 * te / 511.0) / (1.0 + te / UNABSORBED_EMISSION_RELATIVISTIC_KEV);
            let ratio = synchrotron_w_per_m3(n, te, b, a, rw) / unabsorbed_synchrotron_w_per_m3(n, te, b);
            assert!((ratio / factor - 1.0).abs() < 5.0e-3, "{n} {te}: {ratio} {factor}");
        }
    }

    /// Attack: a plasma too thin for Trubnikov's formula. At 1e18 electrons
    /// per cubic metre, 0.1 m, 1000 T and a wall that sends back 90 percent,
    /// electrons at 200 keV would lose by the formula 50 times what they
    /// emit at all; the trial temperature is stopped by name. Inside the
    /// declared range the electrons' own balance holds every settled answer
    /// met in the sweeps far inside the formula's domain (the largest ratio
    /// met there was 0.16), so it is the trial temperatures this holds.
    #[test]
    fn synchrotron_above_what_the_electrons_emit_is_named() {
        let question = ThermalQuestion {
            ion_temperature_kev: 300.0,
            electron_density_per_m3: 1.0e18,
            boron_fraction: 0.15,
            ash_fraction: 0.0,
            magnetic_field_t: 1000.0,
            minor_radius_m: 0.1,
            wall_reflectivity: 0.9,
            energy_confinement_time_s: 100.0,
            kinetic_enhancement: false,
        };
        let plasma = Plasma::new(&question).expect("built");
        match plasma.terms(200.0, 1.0) {
            Err(ThermalReject::SynchrotronAboveUnabsorbedEmission { electron_temperature_kev, ratio }) => {
                assert_eq!(electron_temperature_kev, 200.0);
                assert!(ratio > 45.0 && ratio < 55.0, "{ratio}");
            }
            other => panic!("{:?}", other.map(|terms| terms.synchrotron)),
        }
        // The same plasma at 1 keV is inside the formula's domain.
        assert!(plasma.terms(1.0, 1.0).is_ok());
        // With no synchrotron loss there is nothing to hold.
        let mirror = Plasma::new(&ThermalQuestion { wall_reflectivity: 1.0, ..question }).expect("built");
        assert!(mirror.terms(200.0, 1.0).is_ok());
    }
}
