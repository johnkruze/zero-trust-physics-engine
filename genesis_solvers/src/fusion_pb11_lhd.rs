//! Proton-boron-11 fusion in the hydrogen-beam plasma of the Large Helical Device.
//!
//! The specimen is the boron-seeded hydrogen plasma of the Large Helical
//! Device (National Institute for Fusion Science, Toki), toroidal field
//! 2.75 T, magnetic axis 3.60 m, heated by tangential hydrogen neutral beams:
//! the only magnetically confined plasma in which p + ¹¹B → 3 α has been
//! counted. Calculated before the experiment by Ogawa, Isobe, Nuga, Seki,
//! Ohdachi and Osakabe, Fusion Sci. Technol. 78 (2022) 175 (discharges 168156
//! and 169626); measured by Magee et al., Nat. Commun. 14 (2023) 955, and
//! Ogawa et al., Nucl. Fusion 64 (2024) 096028 (discharges 178062 to 178072).
//!
//! The caller gives one beam and the plasma: beam energy, deposited beam
//! power, line-averaged electron density, central electron temperature, peak
//! boron density. The solver returns the alpha particles made per second and
//! where, or a named limit.
//!
//! A beam proton is born at the beam energy and slows on the electrons, the
//! protons and the boron of the shell it was born in (it does not move
//! radially: the classical confinement of the published calculation). At each
//! energy it may fuse with a boron-11 nucleus:
//!
//! ```text
//! dE/dt   = − Σ_s L_s(E)                        s = electrons, protons, boron   (pb11_reaction)
//! λ(E)    = f₁₁ n_B ⟨σv⟩(E, T_i) / Σ_s L_s(E)   chance of fusing per keV lost
//! S(E)    = exp(−∫_E^E0 λ dE′)                  share still slowing at energy E
//! fused   = 1 − S(E_stop)
//! ```
//!
//! `⟨σv⟩(E, T_i)` is the cross-section of `pb11_reaction` averaged over the
//! thermal motion of the boron: the 148 keV term of the cross-section is
//! 5 keV wide in beam energy, and boron at a few keV smears a 160 keV proton's
//! collision energy by more than that. The plasma is [`SHELLS`] radial shells
//! of equal width in `ρ = r/a₉₉` with fixed profile shapes:
//!
//! ```text
//! T_e(ρ) = T_e0 (1 − ρ²)          T_i = T_e
//! n_e(ρ) = (9/8) n̄_e (1 − ρ⁸)     n_p = n_e − 5 n_B
//! n_B(ρ) = n_B,peak · b(ρ)        b: the polynomial BORON_PROFILE, hollow, highest at ρ = 0.655
//! beam ions born per unit volume ∝ (1 − ρ²);  shell volume ∝ ρ dρ
//! ```
//!
//! In each shell the energy integral is taken in panels and the panels are
//! halved until the fused share settles.
//!
//! What must hold in every shell at every refinement. Protons: the share
//! that slowed to [`STOPPING_ENERGY_KEV`] and the share that fused add to
//! one. Energy: the beam energy equals the energy given to electrons, to
//! protons and to boron, the energy the fused protons carried into fusion,
//! and what the slowed protons still hold. The three channel energies are
//! integrals of each channel's own rate over the total, so a channel counted
//! in one place and not the other shows.
//!
//! Source of each number. Field and axis: Ogawa 2022 and 2024. Minor radius
//! 0.6 m: Ogawa 2024, section 4. The boron profile is DIGITISED from the
//! fitted curve of Ogawa 2022, Figure 3 (discharge 169626), and fitted here
//! by a polynomial of degree six; the page image and the points are in
//! `evidence/research/fusion_pb11/solver/raw/digitised/`. The boron is
//! natural boron, 80 % boron-11 (Ogawa 2024, section 2.1). The shapes of the
//! temperature, the density and the beam deposition are assumptions: the
//! papers print only the central temperature and the line-averaged density.
//! Nothing is fitted.
//!
//! Left out: charge-exchange and orbit loss of beam ions, any radial motion
//! while slowing, the time before steady state, reactions between beam ions
//! and of the thermal plasma (nil at these temperatures), ¹⁰B(p,α)⁷Be, the
//! energy spread of the beam, and how many alphas reach a detector.
//!
//! How the solver compared with the published calculation and with the
//! measured count rates is in the research record,
//! `evidence/research/fusion_pb11/solver/`. The solver answers only inside
//! the envelope of the inputs compared with measurement, and refuses outside
//! it by name: the negative-ion beams inside the envelope of their own runs,
//! each input on its own, and the 45 keV beam only in the one plasma it was
//! compared in. The plasma of the published
//! calculation, hotter and denser than every measured one, is outside it.
//! A boron density that is not zero and is below one boron nucleus in the
//! whole plasma ([`BORON_DENSITY_ONE_NUCLEUS_PER_M3`]) is refused by its own
//! name: no boron is asked as zero, which was compared and is answered.

use std::f64::consts::PI;
use std::fmt;

use crate::pb11_reaction::{
    self as reaction, BORON11_MASS_KG, Background, JOULE_PER_KEV, PROTON_MASS_KG,
    REACTION_ENERGY_KEV, Species,
};

/// Magnetic axis of the specimen's configuration, metres.
pub const MAGNETIC_AXIS_M: f64 = 3.60;
/// Minor radius of the plasma, metres.
pub const MINOR_RADIUS_M: f64 = 0.60;
/// Toroidal field of the specimen's configuration, tesla. It does not enter the arithmetic.
pub const TOROIDAL_FIELD_T: f64 = 2.75;
/// Share of boron-11 in the natural boron of the powder dropper.
pub const BORON11_FRACTION: f64 = 0.80;
/// The boron profile over `ρ`, highest power first, one at its peak.
/// Digitised from Ogawa 2022, Figure 3, and fitted; see the file comment.
pub const BORON_PROFILE: [f64; 7] = [
    -7.14483584,
    19.2325532,
    -19.1168166,
    3.96902043,
    2.99605073,
    -0.0760691253,
    0.413307879,
];
/// Central electron density over the line-averaged density, for the shape `1 − ρ⁸`.
pub const CENTRAL_OVER_LINE_AVERAGED_DENSITY: f64 = 9.0 / 8.0;
/// Radial shells of equal width.
pub const SHELLS: usize = 20;
/// A proton is followed down to this energy, keV. The cross-section there is
/// under 1e-17 of its value at 160 keV.
pub const STOPPING_ENERGY_KEV: f64 = 10.0;

/// The declared range is the envelope of the inputs compared with
/// measurement: the runs A3, A4, A4b, A5, A5b and A6 of the research record,
/// with the inputs digitised from Ogawa et al. 2024, Figures 6 and 10. The
/// two beam families are two islands, each held to its own compared inputs:
/// the negative-ion beams to the envelope of their own runs, input by input,
/// and the 45 keV beam to the one plasma it was compared in.
///
/// The negative-ion beam energies compared: 127.45 keV to 160.25 keV, the
/// plotted beam voltages of Figure 10 and the 160 keV beams of Figure 6.
pub const BEAM_ENERGY_MIN_KEV: f64 = 127.45;
pub const BEAM_ENERGY_MAX_KEV: f64 = 160.25;
/// Power of a negative-ion beam, watts: 0.9389 MW, the smallest deposited
/// power of Figure 10, to 3.97 MW, the injected power of Figure 6 at 4.5
/// and 5.0 s.
pub const BEAM_POWER_MIN_W: f64 = 938_900.0;
pub const BEAM_POWER_MAX_W: f64 = 3_970_000.0;
/// Line-averaged electron density under a negative-ion beam, per cubic
/// metre: Figure 6 at 4.5 s to Figure 10, discharge 178065 at 6.75 s.
pub const ELECTRON_DENSITY_MIN_PER_M3: f64 = 2.261e18;
pub const ELECTRON_DENSITY_MAX_PER_M3: f64 = 6.872e18;
/// Central electron temperature under a negative-ion beam, keV: Figure 10,
/// discharge 178067 at 5.0 s, to Figure 10, discharge 178062 at 5.0 s.
pub const ELECTRON_TEMPERATURE_MIN_KEV: f64 = 2.9929;
pub const ELECTRON_TEMPERATURE_MAX_KEV: f64 = 3.7872;
/// Ceiling on the peak boron density under a negative-ion beam, per cubic
/// metre: the largest compared, Figure 10, discharge 178065 at 6.75 s, the
/// boron line times 6.0e13. Zero was compared too (run A5b).
pub const BORON_DENSITY_MAX_PER_M3: f64 = 2.84682e17;
/// Floor on a peak boron density that is not zero, per cubic metre: the
/// peak density at which the plasma holds one boron nucleus, of either
/// isotope. The solver holds the boron in [`SHELLS`] shells, shell `s` with
/// the share `shell_volume_fraction(s)` of the plasma volume `2π² R a²`,
/// 25.582 m³ ([`plasma_volume_m3`]), at the density `peak · b(ρ_s)` of its
/// middle ([`boron_profile`]). The shares times the profile add to
/// 0.785903, the volume mean of the hollow profile over its peak, so the
/// plasma holds `peak × 25.582 × 0.785903 = peak × 20.105` boron nuclei:
/// one at a peak of 0.04973892441153873 per m³. A test holds the constant
/// to that sum bit for bit, and the count to one at it and under one at the
/// number next below.
///
/// The nearby candidates it is not: 0.0391, one nucleus in the volume at
/// the peak density throughout (`1 / V`); 0.0498, the polynomial's exact
/// volume mean, 0.784763, in place of the twenty shells the solver sums;
/// 0.0622, one nucleus of boron-11 alone, [`BORON11_FRACTION`] of the boron.
///
/// Below it the plasma of the question holds less than one boron nucleus:
/// no boron, which is asked as zero. Zero is a compared value (run A5b) and
/// is answered. The smallest density above zero that was compared is
/// 5.1354e16, eighteen powers of ten above this floor. Before version 0.8.0
/// of this crate every density from zero up was admitted, and near the
/// smallest numbers a 64-bit value holds the fused share is itself one of
/// those numbers, with too few digits to settle to [`SETTLE_TOLERANCE`]:
/// in the plasma of run A4 a solve at 1e-290 per m³ ended on `NotSettled`
/// after [`MAX_REFINEMENTS`] refinements, and at 1e-297 it returned a
/// fused share of zero after six refinements and many seconds, where a
/// solve takes two or three refinements and hundredths of a second.
pub const BORON_DENSITY_ONE_NUCLEUS_PER_M3: f64 = 0.04973892441153873;
/// The one positive-ion beam energy compared: the 45 keV beam of Figure 6
/// at 7.8 s. No other energy below the negative-ion band is accepted.
pub const POSITIVE_ION_BEAM_ENERGY_KEV: f64 = 45.0;
/// The one plasma the 45 keV beam was compared in, Figure 6 at 7.8 s (run
/// A5): the injected power, watts. A 45 keV question is answered only with
/// these four values exactly.
pub const POSITIVE_ION_BEAM_POWER_W: f64 = 4_515_000.0;
/// The same plasma's line-averaged electron density, per cubic metre.
pub const POSITIVE_ION_ELECTRON_DENSITY_PER_M3: f64 = 4.502e18;
/// The same plasma's central electron temperature, keV.
pub const POSITIVE_ION_ELECTRON_TEMPERATURE_KEV: f64 = 2.1252;
/// The same plasma's peak boron density, per cubic metre.
pub const POSITIVE_ION_BORON_DENSITY_PER_M3: f64 = 8.1234e16;
/// Ceiling on the peak boron density over the line-averaged electron
/// density: the largest compared, at the same point.
pub const BORON_OVER_ELECTRON_DENSITY_MAX: f64 = BORON_DENSITY_MAX_PER_M3 / ELECTRON_DENSITY_MAX_PER_M3;

/// Width of an energy panel at the first refinement, keV.
pub const FIRST_PANEL_KEV: f64 = 4.0;
/// The fused share has settled when halving the panels moves it by less than this fraction.
pub const SETTLE_TOLERANCE: f64 = 1.0e-9;
/// The public solver stops a shell after this many refinements.
pub const MAX_REFINEMENTS: u32 = 6;
/// Slowed and fused shares must add to one within this.
pub const PROTON_LEDGER_TOLERANCE: f64 = 1.0e-12;
/// The energy ledger must close within this fraction of the beam energy.
pub const ENERGY_LEDGER_TOLERANCE: f64 = 1.0e-9;

/// The published calculation this solver is checked against.
pub const REFERENCE_NAME: &str = "Ogawa, Isobe, Nuga, Seki, Ohdachi and Osakabe (2022), Evaluation of alpha particle emission rate due to the p-11B fusion reaction in the Large Helical Device, Fusion Sci. Technol. 78, 175-185: 1.5e14 alpha particles per second for the three negative-ion beams of discharge 168156 with the boron profile of discharge 169626";
pub const REFERENCE_URL: &str = "https://nifs-repository.repo.nii.ac.jp/record/11123/files/Fusion.Sci%20Tech_78_175.pdf";
/// Date the reference was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";
/// The published alpha emission rate, per second.
pub const REFERENCE_ALPHA_RATE_PER_S: f64 = 1.5e14;

/// One beam into one plasma.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LhdQuestion {
    /// Energy of the beam protons, keV.
    pub beam_energy_kev: f64,
    /// Beam power deposited in the plasma, watts.
    pub beam_power_w: f64,
    /// Line-averaged electron density, per cubic metre.
    pub electron_density_per_m3: f64,
    /// Central electron temperature, keV.
    pub electron_temperature_kev: f64,
    /// Boron density at the peak of its profile, all isotopes, per cubic metre.
    pub boron_density_per_m3: f64,
}

/// Why no answer was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum LhdReject {
    /// `beam_energy_kev` is neither [`POSITIVE_ION_BEAM_ENERGY_KEV`] nor inside [`BEAM_ENERGY_MIN_KEV`]..=[`BEAM_ENERGY_MAX_KEV`]: the beam energies compared with measurement.
    BeamEnergyOutOfRange { beam_energy_kev: f64 },
    /// `beam_power_w` is not a power compared with measurement for the beam asked: outside [`BEAM_POWER_MIN_W`]..=[`BEAM_POWER_MAX_W`] for a negative-ion beam, or not [`POSITIVE_ION_BEAM_POWER_W`] for the 45 keV beam.
    BeamPowerOutOfRange { beam_power_w: f64 },
    /// `electron_density_per_m3` is not a density compared with measurement for the beam asked: outside [`ELECTRON_DENSITY_MIN_PER_M3`]..=[`ELECTRON_DENSITY_MAX_PER_M3`] for a negative-ion beam, or not [`POSITIVE_ION_ELECTRON_DENSITY_PER_M3`] for the 45 keV beam.
    ElectronDensityOutOfRange { electron_density_per_m3: f64 },
    /// `electron_temperature_kev` is not a temperature compared with measurement for the beam asked: outside [`ELECTRON_TEMPERATURE_MIN_KEV`]..=[`ELECTRON_TEMPERATURE_MAX_KEV`] for a negative-ion beam, or not [`POSITIVE_ION_ELECTRON_TEMPERATURE_KEV`] for the 45 keV beam.
    ElectronTemperatureOutOfRange { electron_temperature_kev: f64 },
    /// `boron_density_per_m3` is not a boron density compared with measurement for the beam asked: below zero or above [`BORON_DENSITY_MAX_PER_M3`] for a negative-ion beam, or not [`POSITIVE_ION_BORON_DENSITY_PER_M3`] for the 45 keV beam.
    BoronDensityOutOfRange { boron_density_per_m3: f64 },
    /// `boron_density_per_m3` is not zero and is below [`BORON_DENSITY_ONE_NUCLEUS_PER_M3`], the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered.
    BoronBelowOneNucleus { boron_density_per_m3: f64 },
    /// The peak boron density over the line-averaged electron density is above [`BORON_OVER_ELECTRON_DENSITY_MAX`], the largest compared with measurement.
    BoronShareOutOfRange { boron_over_electron_density: f64 },
    /// Quasi-neutrality: in one shell the boron alone carries more charge than the electrons there.
    BoronChargeExceedsElectrons { shell: usize },
    /// An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.
    NonFiniteValue,
    /// The refinement cap was reached with the fused share of a shell still moving.
    NotSettled { refinements: u32 },
    /// In one shell the slowed and the fused shares left [`PROTON_LEDGER_TOLERANCE`] of one.
    ProtonLedgerBroken { residual: f64 },
    /// In one shell the energy ledger left [`ENERGY_LEDGER_TOLERANCE`] of the beam energy.
    EnergyLedgerBroken { residual: f64 },
}

impl fmt::Display for LhdReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeamEnergyOutOfRange { beam_energy_kev } => write!(
                f,
                "beam energy {beam_energy_kev} keV is neither {POSITIVE_ION_BEAM_ENERGY_KEV} nor inside {BEAM_ENERGY_MIN_KEV}..={BEAM_ENERGY_MAX_KEV}, the beam energies compared with measurement"
            ),
            Self::BeamPowerOutOfRange { beam_power_w } => write!(
                f,
                "beam power {beam_power_w} W is not a power compared with measurement for the beam asked: {BEAM_POWER_MIN_W}..={BEAM_POWER_MAX_W} for a negative-ion beam, {POSITIVE_ION_BEAM_POWER_W} for the 45 keV beam"
            ),
            Self::ElectronDensityOutOfRange { electron_density_per_m3 } => write!(
                f,
                "electron density {electron_density_per_m3} m^-3 is not a density compared with measurement for the beam asked: {ELECTRON_DENSITY_MIN_PER_M3}..={ELECTRON_DENSITY_MAX_PER_M3} for a negative-ion beam, {POSITIVE_ION_ELECTRON_DENSITY_PER_M3} for the 45 keV beam"
            ),
            Self::ElectronTemperatureOutOfRange { electron_temperature_kev } => write!(
                f,
                "electron temperature {electron_temperature_kev} keV is not a temperature compared with measurement for the beam asked: {ELECTRON_TEMPERATURE_MIN_KEV}..={ELECTRON_TEMPERATURE_MAX_KEV} for a negative-ion beam, {POSITIVE_ION_ELECTRON_TEMPERATURE_KEV} for the 45 keV beam"
            ),
            Self::BoronDensityOutOfRange { boron_density_per_m3 } => write!(
                f,
                "boron density {boron_density_per_m3} m^-3 is not a boron density compared with measurement for the beam asked: 0..={BORON_DENSITY_MAX_PER_M3} for a negative-ion beam, {POSITIVE_ION_BORON_DENSITY_PER_M3} for the 45 keV beam"
            ),
            Self::BoronBelowOneNucleus { boron_density_per_m3 } => write!(
                f,
                "boron density {boron_density_per_m3:e} m^-3 is not zero and is below {BORON_DENSITY_ONE_NUCLEUS_PER_M3} m^-3, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered"
            ),
            Self::BoronShareOutOfRange { boron_over_electron_density } => write!(
                f,
                "peak boron density over line-averaged electron density {boron_over_electron_density} is above {BORON_OVER_ELECTRON_DENSITY_MAX}, the largest compared with measurement"
            ),
            Self::BoronChargeExceedsElectrons { shell } => write!(
                f,
                "in shell {shell} of {SHELLS} the boron carries more charge than the electrons: quasi-neutrality cannot hold"
            ),
            Self::NonFiniteValue => write!(f, "an input or an intermediate value was not finite"),
            Self::NotSettled { refinements } => write!(
                f,
                "the fused share of a shell was still moving after {refinements} refinements"
            ),
            Self::ProtonLedgerBroken { residual } => write!(
                f,
                "slowed and fused protons missed one by {residual}"
            ),
            Self::EnergyLedgerBroken { residual } => write!(
                f,
                "the energy ledger missed by {residual} of the beam energy"
            ),
        }
    }
}

impl std::error::Error for LhdReject {}

/// The settled answer for one beam.
#[derive(Clone, Debug, PartialEq)]
pub struct LhdBeamFusion {
    /// Alpha particles made per second in the whole plasma: three per reaction.
    pub alpha_emission_rate_per_s: f64,
    /// Fusion power, watts: reactions per second times 8.68 MeV.
    pub fusion_power_w: f64,
    /// Fusion power over the deposited beam power: the fused share times
    /// 8.68 MeV over the beam energy. It does not pass through the power.
    pub fusion_gain: f64,
    /// Share of the beam protons that fuse before they stop.
    pub fused_fraction: f64,
    /// Highest alpha emissivity of any shell, per cubic metre per second.
    pub peak_alpha_emissivity_per_m3_s: f64,
    /// `ρ` at the middle of that shell. With no boron it is the first shell.
    pub peak_emissivity_radius: f64,
    /// Mean time a beam proton takes to slow to the stopping energy, seconds.
    pub slowing_time_s: f64,
    /// Share of the beam energy given to electrons.
    pub energy_fraction_to_electrons: f64,
    /// Share of the beam energy given to protons and boron.
    pub energy_fraction_to_ions: f64,
    /// Largest miss of slowed plus fused from one, over the shells.
    pub proton_ledger_residual: f64,
    /// Largest miss of the energy ledger over the shells, as a fraction of the beam energy.
    pub energy_ledger_residual: f64,
    /// Refinements taken by the shell that took the most.
    pub refinements: u32,
    /// Alpha emissivity of each shell from the axis outward, per cubic metre per second.
    pub alpha_emissivity_per_m3_s: Vec<f64>,
}

/// A fault a test puts into one shell to see it named.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Fault {
    None,
    /// The boron's drag slows the proton and is left out of the energy given to boron.
    BoronDragNotCredited,
    /// Fused protons are counted twice.
    FusedCountedTwice,
}

/// One shell at one refinement.
#[derive(Clone, Copy, Debug)]
struct ShellAnswer {
    fused: f64,
    to_electrons_kev: f64,
    to_ions_kev: f64,
    slowing_time_s: f64,
    proton_residual: f64,
    energy_residual: f64,
}

/// The value of the boron profile at `rho`.
pub fn boron_profile(rho: f64) -> f64 {
    let mut value = 0.0;
    for coefficient in BORON_PROFILE {
        value = value * rho + coefficient;
    }
    value
}

/// Electron temperature (keV), electron density and boron density (per cubic metre) at `rho`.
fn plasma_at(question: &LhdQuestion, rho: f64) -> (f64, f64, f64) {
    let rho2 = rho * rho;
    let rho8 = rho2 * rho2 * rho2 * rho2;
    (
        question.electron_temperature_kev * (1.0 - rho2),
        CENTRAL_OVER_LINE_AVERAGED_DENSITY * question.electron_density_per_m3 * (1.0 - rho8),
        question.boron_density_per_m3 * boron_profile(rho),
    )
}

fn shell_middle(shell: usize) -> f64 {
    (shell as f64 + 0.5) / SHELLS as f64
}

/// Share of the plasma volume in one shell.
fn shell_volume_fraction(shell: usize) -> f64 {
    let inner = shell as f64 / SHELLS as f64;
    let outer = (shell as f64 + 1.0) / SHELLS as f64;
    outer * outer - inner * inner
}

/// The plasma volume, cubic metres: `2π² R a²`.
pub fn plasma_volume_m3() -> f64 {
    2.0 * PI * PI * MAGNETIC_AXIS_M * MINOR_RADIUS_M * MINOR_RADIUS_M
}

/// The named limits a question itself shows: the envelope compared with
/// measurement, island by island, a boron density above zero and below one
/// nucleus in the plasma, and quasi-neutrality. Nothing is integrated here.
pub fn admit(question: &LhdQuestion) -> Result<(), LhdReject> {
    let LhdQuestion {
        beam_energy_kev,
        beam_power_w,
        electron_density_per_m3,
        electron_temperature_kev,
        boron_density_per_m3,
    } = *question;
    for value in [
        beam_energy_kev,
        beam_power_w,
        electron_density_per_m3,
        electron_temperature_kev,
        boron_density_per_m3,
    ] {
        if !value.is_finite() {
            return Err(LhdReject::NonFiniteValue);
        }
    }
    let positive_ion = beam_energy_kev == POSITIVE_ION_BEAM_ENERGY_KEV;
    if !(positive_ion || (BEAM_ENERGY_MIN_KEV..=BEAM_ENERGY_MAX_KEV).contains(&beam_energy_kev)) {
        return Err(LhdReject::BeamEnergyOutOfRange { beam_energy_kev });
    }
    // Each beam family is held to the inputs compared under it: a range of
    // its own for the negative-ion beams, the one compared plasma for 45 keV.
    let compared = |value: f64, low: f64, high: f64, point: f64| {
        if positive_ion { value == point } else { (low..=high).contains(&value) }
    };
    if !compared(beam_power_w, BEAM_POWER_MIN_W, BEAM_POWER_MAX_W, POSITIVE_ION_BEAM_POWER_W) {
        return Err(LhdReject::BeamPowerOutOfRange { beam_power_w });
    }
    if !compared(
        electron_density_per_m3,
        ELECTRON_DENSITY_MIN_PER_M3,
        ELECTRON_DENSITY_MAX_PER_M3,
        POSITIVE_ION_ELECTRON_DENSITY_PER_M3,
    ) {
        return Err(LhdReject::ElectronDensityOutOfRange { electron_density_per_m3 });
    }
    if !compared(
        electron_temperature_kev,
        ELECTRON_TEMPERATURE_MIN_KEV,
        ELECTRON_TEMPERATURE_MAX_KEV,
        POSITIVE_ION_ELECTRON_TEMPERATURE_KEV,
    ) {
        return Err(LhdReject::ElectronTemperatureOutOfRange { electron_temperature_kev });
    }
    if !compared(boron_density_per_m3, 0.0, BORON_DENSITY_MAX_PER_M3, POSITIVE_ION_BORON_DENSITY_PER_M3) {
        return Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3 });
    }
    // Less than one boron nucleus in the whole plasma is no boron, and no
    // boron is asked as zero, which is answered.
    if boron_density_per_m3 != 0.0 && boron_density_per_m3 < BORON_DENSITY_ONE_NUCLEUS_PER_M3 {
        return Err(LhdReject::BoronBelowOneNucleus { boron_density_per_m3 });
    }
    let boron_over_electron_density = boron_density_per_m3 / electron_density_per_m3;
    if !(boron_over_electron_density <= BORON_OVER_ELECTRON_DENSITY_MAX) {
        return Err(LhdReject::BoronShareOutOfRange { boron_over_electron_density });
    }
    quasi_neutral(question)
}

/// Quasi-neutrality in every shell: five times the boron density may not
/// exceed the electron density there.
fn quasi_neutral(question: &LhdQuestion) -> Result<(), LhdReject> {
    for shell in 0..SHELLS {
        let (_, electrons, boron) = plasma_at(question, shell_middle(shell));
        if !(5.0 * boron <= electrons) {
            return Err(LhdReject::BoronChargeExceedsElectrons { shell });
        }
    }
    Ok(())
}

/// One shell with energy panels `panel_kev` wide, from the beam energy down.
fn slow_and_fuse(
    beam_energy_kev: f64,
    electron_density_per_m3: f64,
    temperature_kev: f64,
    boron_density_per_m3: f64,
    panel_kev: f64,
    fault: Fault,
) -> Result<ShellAnswer, LhdReject> {
    let plasma = Background::new(electron_density_per_m3, temperature_kev, temperature_kev);
    let proton = Species { charge: 1.0, mass_kg: PROTON_MASS_KG, density_per_m3: 0.0 };
    let protons = Species {
        charge: 1.0,
        mass_kg: PROTON_MASS_KG,
        density_per_m3: electron_density_per_m3 - 5.0 * boron_density_per_m3,
    };
    let boron = Species { charge: 5.0, mass_kg: BORON11_MASS_KG, density_per_m3: boron_density_per_m3 };
    let targets = BORON11_FRACTION * boron_density_per_m3;

    let span = beam_energy_kev - STOPPING_ENERGY_KEV;
    let panels = ((span / panel_kev).ceil() as usize).max(1);
    let width = span / panels as f64;
    let half = 0.5 * width;

    let mut surviving = 1.0;
    let mut fused = 0.0;
    let mut fused_energy_kev = 0.0;
    let mut to_electrons_kev = 0.0;
    let mut to_protons_kev = 0.0;
    let mut to_boron_kev = 0.0;
    let mut slowing_time_s = 0.0;
    for panel in (0..panels).rev() {
        let middle = STOPPING_ENERGY_KEV + (panel as f64 + 0.5) * width;
        let mut chance = 0.0;
        let mut chance_energy = 0.0;
        let mut share_electrons = 0.0;
        let mut share_protons = 0.0;
        let mut share_boron = 0.0;
        let mut time = 0.0;
        for (node, weight) in reaction::GAUSS_NODES.iter().zip(reaction::GAUSS_WEIGHTS) {
            let energy = middle + half * node;
            let weight = half * weight;
            let electrons_rate = plasma.loss_to_electrons_kev_per_s(&proton, energy);
            let protons_rate = plasma.loss_to_ions_kev_per_s(&proton, energy, &protons);
            let boron_rate = plasma.loss_to_ions_kev_per_s(&proton, energy, &boron);
            let total = electrons_rate + protons_rate + boron_rate;
            if !(total.is_finite() && total > 0.0) {
                return Err(LhdReject::NonFiniteValue);
            }
            let per_kev = if targets > 0.0 {
                targets * reaction::beam_target_rate_m3_per_s(energy, temperature_kev, panel_kev) / total
            } else {
                0.0
            };
            chance += weight * per_kev;
            chance_energy += weight * per_kev * energy;
            share_electrons += weight * electrons_rate / total;
            share_protons += weight * protons_rate / total;
            if fault != Fault::BoronDragNotCredited {
                share_boron += weight * boron_rate / total;
            }
            time += weight / total;
        }
        // Through the panel the surviving share falls by exp(−chance). The
        // energy handed to the plasma is counted at the share half way through.
        let midway = surviving * (-0.5 * chance).exp();
        let fused_here = -surviving * (-chance).exp_m1();
        fused += fused_here;
        if chance > 0.0 {
            fused_energy_kev += fused_here * chance_energy / chance;
        }
        to_electrons_kev += midway * share_electrons;
        to_protons_kev += midway * share_protons;
        to_boron_kev += midway * share_boron;
        slowing_time_s += midway * time;
        surviving -= fused_here;
    }
    if fault == Fault::FusedCountedTwice {
        fused *= 2.0;
    }
    let answer = ShellAnswer {
        fused,
        to_electrons_kev,
        to_ions_kev: to_protons_kev + to_boron_kev,
        slowing_time_s,
        proton_residual: 1.0 - surviving - fused,
        energy_residual: (beam_energy_kev
            - to_electrons_kev
            - to_protons_kev
            - to_boron_kev
            - fused_energy_kev
            - surviving * STOPPING_ENERGY_KEV)
            / beam_energy_kev,
    };
    hold(&answer)?;
    Ok(answer)
}

/// Holds one shell's answer to the two ledgers.
fn hold(answer: &ShellAnswer) -> Result<(), LhdReject> {
    for value in [
        answer.fused,
        answer.to_electrons_kev,
        answer.to_ions_kev,
        answer.slowing_time_s,
        answer.proton_residual,
        answer.energy_residual,
    ] {
        if !value.is_finite() {
            return Err(LhdReject::NonFiniteValue);
        }
    }
    if !(answer.proton_residual.abs() <= PROTON_LEDGER_TOLERANCE) {
        return Err(LhdReject::ProtonLedgerBroken { residual: answer.proton_residual });
    }
    if !(answer.energy_residual.abs() <= ENERGY_LEDGER_TOLERANCE) {
        return Err(LhdReject::EnergyLedgerBroken { residual: answer.energy_residual });
    }
    Ok(())
}

/// One shell refined until its fused share settles. Returns the answer and the refinements taken.
fn settle_shell(
    beam_energy_kev: f64,
    electron_density_per_m3: f64,
    temperature_kev: f64,
    boron_density_per_m3: f64,
    refinement_cap: u32,
    fault: Fault,
) -> Result<(ShellAnswer, u32), LhdReject> {
    let mut panel_kev = FIRST_PANEL_KEV;
    let mut previous: Option<ShellAnswer> = None;
    for refinement in 1..=refinement_cap {
        let current = slow_and_fuse(
            beam_energy_kev,
            electron_density_per_m3,
            temperature_kev,
            boron_density_per_m3,
            panel_kev,
            fault,
        )?;
        if boron_density_per_m3 == 0.0 {
            return Ok((current, refinement));
        }
        if let Some(previous) = previous {
            if (current.fused - previous.fused).abs() <= SETTLE_TOLERANCE * current.fused {
                return Ok((current, refinement));
            }
        }
        previous = Some(current);
        panel_kev *= 0.5;
    }
    Err(LhdReject::NotSettled { refinements: refinement_cap })
}

/// The share of beam protons that fuse while slowing in one uniform plasma:
/// one shell of the solver, settled and held to both ledgers, with the ions
/// at the electron temperature. For the outside recomputation and for tests;
/// it enforces no range of the specimen.
pub fn fused_fraction_in_uniform_plasma(
    beam_energy_kev: f64,
    electron_density_per_m3: f64,
    temperature_kev: f64,
    boron_density_per_m3: f64,
) -> Result<f64, LhdReject> {
    settle_shell(
        beam_energy_kev,
        electron_density_per_m3,
        temperature_kev,
        boron_density_per_m3,
        MAX_REFINEMENTS,
        Fault::None,
    )
    .map(|(answer, _)| answer.fused)
}

/// The alpha particles one beam makes in the boron-seeded plasma of the Large Helical Device.
pub fn solve(question: &LhdQuestion) -> Result<LhdBeamFusion, LhdReject> {
    solve_with(question, MAX_REFINEMENTS, Fault::None)
}

fn solve_with(question: &LhdQuestion, refinement_cap: u32, fault: Fault) -> Result<LhdBeamFusion, LhdReject> {
    admit(question)?;
    answer(question, refinement_cap, fault)
}

/// The physics of one question, with no envelope asked. For the tests that
/// run the published calculation's plasma, which the solver refuses.
fn answer(question: &LhdQuestion, refinement_cap: u32, fault: Fault) -> Result<LhdBeamFusion, LhdReject> {
    quasi_neutral(question)?;
    let mut deposited = [0.0; SHELLS];
    let mut deposited_sum = 0.0;
    for (shell, share) in deposited.iter_mut().enumerate() {
        let rho = shell_middle(shell);
        *share = (1.0 - rho * rho) * shell_volume_fraction(shell);
        deposited_sum += *share;
    }
    let protons_per_s = question.beam_power_w / (question.beam_energy_kev * JOULE_PER_KEV);
    let volume_m3 = plasma_volume_m3();

    let mut emissivity = Vec::with_capacity(SHELLS);
    let mut fused_fraction = 0.0;
    let mut slowing_time_s = 0.0;
    let mut to_electrons = 0.0;
    let mut to_ions = 0.0;
    let mut proton_residual: f64 = 0.0;
    let mut energy_residual: f64 = 0.0;
    let mut refinements = 0;
    for (shell, share) in deposited.iter().enumerate() {
        let share = share / deposited_sum;
        let (temperature, electrons, boron) = plasma_at(question, shell_middle(shell));
        let (answer, taken) = settle_shell(question.beam_energy_kev, electrons, temperature, boron, refinement_cap, fault)?;
        fused_fraction += share * answer.fused;
        slowing_time_s += share * answer.slowing_time_s;
        to_electrons += share * answer.to_electrons_kev / question.beam_energy_kev;
        to_ions += share * answer.to_ions_kev / question.beam_energy_kev;
        proton_residual = proton_residual.max(answer.proton_residual.abs());
        energy_residual = energy_residual.max(answer.energy_residual.abs());
        refinements = refinements.max(taken);
        emissivity.push(3.0 * protons_per_s * share * answer.fused / (volume_m3 * shell_volume_fraction(shell)));
    }
    let reactions_per_s = protons_per_s * fused_fraction;
    let fusion_power_w = reactions_per_s * REACTION_ENERGY_KEV * JOULE_PER_KEV;
    let mut peak_shell = 0;
    for shell in 1..SHELLS {
        if emissivity[shell] > emissivity[peak_shell] {
            peak_shell = shell;
        }
    }
    let result = LhdBeamFusion {
        alpha_emission_rate_per_s: 3.0 * reactions_per_s,
        fusion_power_w,
        fusion_gain: fused_fraction * REACTION_ENERGY_KEV / question.beam_energy_kev,
        fused_fraction,
        peak_alpha_emissivity_per_m3_s: emissivity[peak_shell],
        peak_emissivity_radius: shell_middle(peak_shell),
        slowing_time_s,
        energy_fraction_to_electrons: to_electrons,
        energy_fraction_to_ions: to_ions,
        proton_ledger_residual: proton_residual,
        energy_ledger_residual: energy_residual,
        refinements,
        alpha_emissivity_per_m3_s: emissivity,
    };
    if !(result.alpha_emission_rate_per_s.is_finite() && result.fusion_gain.is_finite()) {
        return Err(LhdReject::NonFiniteValue);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three negative-ion beams of discharge 168156 as Ogawa et al. 2022
    /// give them, read 2026-10-09 ([`REFERENCE_RETRIEVED`]) from the NIFS
    /// repository copy of the paper
    /// (<https://nifs-repository.repo.nii.ac.jp/record/11123/files/Fusion.Sci%20Tech_78_175.pdf>,
    /// CC BY-NC-SA 3.0). Energies are from the text; the deposited powers,
    /// the density and the temperature are DIGITISED from its Figure 4 as
    /// means from 4.7 to 5.2 s; the boron peak is DIGITISED from its Figure 3
    /// with the scale of the text and the colour bar (about 7e16 m^-3). The
    /// axis of that figure's lower panel is labelled 1e17 m^-3, ten times
    /// more. The paper's 1.5e14 alphas per second is a calculation, not a
    /// measurement.
    const OGAWA_2022_BEAMS: [(f64, f64); 3] = [(180.0, 4.33e6), (165.0, 2.39e6), (164.0, 3.37e6)];
    const OGAWA_2022_DENSITY_PER_M3: f64 = 1.96e19;
    const OGAWA_2022_TEMPERATURE_KEV: f64 = 5.4;
    const OGAWA_2022_BORON_PER_M3: f64 = 6.3e16;

    fn ogawa_2022(beam: usize, boron_density_per_m3: f64) -> LhdQuestion {
        LhdQuestion {
            beam_energy_kev: OGAWA_2022_BEAMS[beam].0,
            beam_power_w: OGAWA_2022_BEAMS[beam].1,
            electron_density_per_m3: OGAWA_2022_DENSITY_PER_M3,
            electron_temperature_kev: OGAWA_2022_TEMPERATURE_KEV,
            boron_density_per_m3,
        }
    }

    /// A measured point: Ogawa et al. 2024, Figure 6, discharge 178063 at
    /// 6.0 s (run A4 of the research record), the injected power as given.
    const MEASURED: LhdQuestion = LhdQuestion {
        beam_energy_kev: 160.0,
        beam_power_w: 2.424e6,
        electron_density_per_m3: 3.137e18,
        electron_temperature_kev: 3.2438,
        boron_density_per_m3: 8.6706e16,
    };

    /// The one plasma the 45 keV beam was compared in: Ogawa et al. 2024,
    /// Figure 6, discharge 178063 at 7.8 s (run A5), the injected power.
    const POSITIVE_ION: LhdQuestion = LhdQuestion {
        beam_energy_kev: 45.0,
        beam_power_w: 4.515e6,
        electron_density_per_m3: 4.502e18,
        electron_temperature_kev: 2.1252,
        boron_density_per_m3: 8.1234e16,
    };

    /// The measured point with the most boron for its electrons: Ogawa et al.
    /// 2024, Figure 10, discharge 178065 at 6.75 s (run A3).
    const MOST_BORON: LhdQuestion = LhdQuestion {
        beam_energy_kev: 127.45,
        beam_power_w: 1.4056e6,
        electron_density_per_m3: 6.872e18,
        electron_temperature_kev: 3.3333,
        boron_density_per_m3: 2.84682e17,
    };

    /// The comparison with the published calculation, printed as it comes
    /// out. With the boron of the text the solver gives about an eighth of
    /// the published rate; with the boron of the figure's axis, about 1.3
    /// times. The test holds the solver to its own two numbers and prints
    /// both ratios: it does not hold either to the published one. The
    /// served solver refuses this plasma: it lies outside the envelope
    /// compared with measurement. The test runs the physics directly.
    #[test]
    fn ogawa_2022_three_beams_of_discharge_168156() {
        println!("Ogawa et al. 2022 read {REFERENCE_RETRIEVED} from {REFERENCE_URL}");
        for beam in 0..3 {
            assert!(matches!(
                solve(&ogawa_2022(beam, OGAWA_2022_BORON_PER_M3)),
                Err(LhdReject::BeamEnergyOutOfRange { .. })
            ));
        }
        for (boron, low, high) in [
            (OGAWA_2022_BORON_PER_M3, 1.8e13, 2.1e13),
            (10.0 * OGAWA_2022_BORON_PER_M3, 1.8e14, 2.1e14),
        ] {
            let mut rate = 0.0;
            for beam in 0..3 {
                rate += answer(&ogawa_2022(beam, boron), MAX_REFINEMENTS, Fault::None)
                    .expect("solves")
                    .alpha_emission_rate_per_s;
            }
            println!(
                "boron peak {boron:e} m^-3: {rate:e} alphas per second, {:.3} of the published {REFERENCE_ALPHA_RATE_PER_S:e}",
                rate / REFERENCE_ALPHA_RATE_PER_S
            );
            assert!(rate > low && rate < high, "{rate}");
        }
    }

    #[test]
    fn every_proton_and_every_kev_is_accounted_for() {
        let answer = solve(&MEASURED).expect("solves");
        assert!(answer.proton_ledger_residual <= PROTON_LEDGER_TOLERANCE);
        assert!(answer.energy_ledger_residual <= ENERGY_LEDGER_TOLERANCE);
        // Almost all of the beam energy goes to the plasma; the rest is what
        // a proton still holds at the stopping energy.
        let handed = answer.energy_fraction_to_electrons + answer.energy_fraction_to_ions;
        let kept = STOPPING_ENERGY_KEV / 160.0;
        assert!((handed + kept - 1.0).abs() < 1.0e-6, "{handed}");
        assert!(answer.fused_fraction > 1.0e-9 && answer.fused_fraction < 1.0e-6);
        assert_eq!(answer.alpha_emissivity_per_m3_s.len(), SHELLS);
        // The alphas of the shells add to the total.
        let volume = plasma_volume_m3();
        let mut total = 0.0;
        for (shell, emissivity) in answer.alpha_emissivity_per_m3_s.iter().enumerate() {
            total += emissivity * volume * shell_volume_fraction(shell);
        }
        assert!((total / answer.alpha_emission_rate_per_s - 1.0).abs() < 1.0e-12);
        assert!((answer.fusion_gain * 2.424e6 / answer.fusion_power_w - 1.0).abs() < 1.0e-12);
    }

    /// The gain is the fused share times 8.68 MeV over the beam energy: it
    /// does not pass through the power, and it is the same at both ends of
    /// the power's range.
    #[test]
    fn the_gain_does_not_pass_through_the_power() {
        let mut gains = Vec::new();
        for beam_power_w in [BEAM_POWER_MIN_W, MEASURED.beam_power_w, BEAM_POWER_MAX_W] {
            let answer = solve(&LhdQuestion { beam_power_w, ..MEASURED }).expect("solves");
            assert_eq!(
                answer.fusion_gain.to_bits(),
                (answer.fused_fraction * REACTION_ENERGY_KEV / 160.0).to_bits()
            );
            assert!(answer.fusion_gain > 1.0e-7, "{}", answer.fusion_gain);
            gains.push(answer.fusion_gain.to_bits());
        }
        gains.dedup();
        assert_eq!(gains.len(), 1);
    }

    /// A power too small to have been compared, the smallest number above
    /// zero among them, is refused by name: it no longer returns a gain of
    /// zero with a proof.
    #[test]
    fn a_power_below_the_compared_floor_is_named() {
        for beam_power_w in [5.0e-324, 1.0, BEAM_POWER_MIN_W.next_down(), 0.0, -1.0] {
            assert_eq!(
                solve(&LhdQuestion { beam_power_w, ..MEASURED }),
                Err(LhdReject::BeamPowerOutOfRange { beam_power_w })
            );
        }
        let above = BEAM_POWER_MAX_W.next_up();
        assert_eq!(
            solve(&LhdQuestion { beam_power_w: above, ..MEASURED }),
            Err(LhdReject::BeamPowerOutOfRange { beam_power_w: above })
        );
        // Six megawatts, one beam's stated maximum, was never compared.
        assert!(solve(&LhdQuestion { beam_power_w: 6.0e6, ..MEASURED }).is_err());
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let first = solve(&MOST_BORON).expect("solves");
        let second = solve(&MOST_BORON).expect("solves");
        assert_eq!(first.alpha_emission_rate_per_s.to_bits(), second.alpha_emission_rate_per_s.to_bits());
        for (left, right) in first.alpha_emissivity_per_m3_s.iter().zip(&second.alpha_emissivity_per_m3_s) {
            assert_eq!(left.to_bits(), right.to_bits());
        }
    }

    #[test]
    fn the_rate_follows_power_boron_and_beam_energy() {
        let base = solve(&MEASURED).expect("solves");
        let halved = solve(&LhdQuestion { beam_power_w: 0.5 * MEASURED.beam_power_w, ..MEASURED }).expect("solves");
        assert!((halved.alpha_emission_rate_per_s / base.alpha_emission_rate_per_s - 0.5).abs() < 1.0e-12);
        assert_eq!(halved.fused_fraction.to_bits(), base.fused_fraction.to_bits());
        // No boron, no alphas.
        let none = solve(&LhdQuestion { boron_density_per_m3: 0.0, ..MEASURED }).expect("solves");
        assert_eq!(none.alpha_emission_rate_per_s, 0.0);
        assert_eq!(none.fused_fraction, 0.0);
        // A 45 keV beam never reaches the reactive energies: in the one plasma it was compared in.
        let slow = solve(&POSITIVE_ION).expect("solves");
        assert!(slow.fused_fraction < 1.0e-4 * base.fused_fraction, "{}", slow.fused_fraction);
        // A hotter plasma slows the proton less, so more of them fuse.
        let hot = LhdQuestion { electron_temperature_kev: ELECTRON_TEMPERATURE_MAX_KEV, ..MEASURED };
        assert!(solve(&hot).expect("solves").fused_fraction > base.fused_fraction);
    }

    #[test]
    fn a_question_outside_the_compared_envelope_is_named() {
        let with = |change: fn(&mut LhdQuestion)| {
            let mut question = MEASURED;
            change(&mut question);
            solve(&question)
        };
        // The beam energies compared: 45 keV, and 127.45 to 160.25 keV.
        for beam_energy_kev in [
            39.9,
            44.9,
            POSITIVE_ION_BEAM_ENERGY_KEV.next_down(),
            POSITIVE_ION_BEAM_ENERGY_KEV.next_up(),
            46.0,
            100.0,
            BEAM_ENERGY_MIN_KEV.next_down(),
            BEAM_ENERGY_MAX_KEV.next_up(),
            164.0,
            180.0,
            190.0,
        ] {
            assert_eq!(
                solve(&LhdQuestion { beam_energy_kev, ..MEASURED }),
                Err(LhdReject::BeamEnergyOutOfRange { beam_energy_kev })
            );
        }
        for beam_energy_kev in [BEAM_ENERGY_MIN_KEV, BEAM_ENERGY_MAX_KEV] {
            assert!(solve(&LhdQuestion { beam_energy_kev, ..MEASURED }).is_ok(), "{beam_energy_kev}");
        }
        assert!(solve(&LhdQuestion { beam_energy_kev: POSITIVE_ION_BEAM_ENERGY_KEV, ..POSITIVE_ION }).is_ok());
        let below = ELECTRON_DENSITY_MIN_PER_M3.next_down();
        let above = ELECTRON_DENSITY_MAX_PER_M3.next_up();
        for electron_density_per_m3 in [1.0e18, below, above, 1.96e19, 3.0e19] {
            assert_eq!(
                solve(&LhdQuestion { electron_density_per_m3, boron_density_per_m3: 0.0, ..MEASURED }),
                Err(LhdReject::ElectronDensityOutOfRange { electron_density_per_m3 })
            );
        }
        let below = ELECTRON_TEMPERATURE_MIN_KEV.next_down();
        let above = ELECTRON_TEMPERATURE_MAX_KEV.next_up();
        for electron_temperature_kev in [1.0, below, above, 5.4, 8.0] {
            assert_eq!(
                solve(&LhdQuestion { electron_temperature_kev, ..MEASURED }),
                Err(LhdReject::ElectronTemperatureOutOfRange { electron_temperature_kev })
            );
        }
        assert_eq!(
            with(|q| q.boron_density_per_m3 = -1.0),
            Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3: -1.0 })
        );
        let above = BORON_DENSITY_MAX_PER_M3.next_up();
        assert_eq!(
            solve(&LhdQuestion { boron_density_per_m3: above, ..MOST_BORON }),
            Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3: above })
        );
        // Boron for its electrons: the most compared is accepted, and more is named.
        assert!(solve(&MOST_BORON).is_ok());
        assert_eq!(
            MOST_BORON.boron_density_per_m3 / MOST_BORON.electron_density_per_m3,
            BORON_OVER_ELECTRON_DENSITY_MAX
        );
        let thinner = LhdQuestion { electron_density_per_m3: 4.0e18, boron_density_per_m3: 2.0e17, ..MEASURED };
        assert_eq!(solve(&thinner), Err(LhdReject::BoronShareOutOfRange { boron_over_electron_density: 0.05 }));
        assert_eq!(with(|q| q.beam_energy_kev = f64::NAN), Err(LhdReject::NonFiniteValue));
        assert_eq!(with(|q| q.boron_density_per_m3 = f64::INFINITY), Err(LhdReject::NonFiniteValue));
        // Requests 0.6.0 answered with a proof outside the envelope, each
        // now refused.
        for question in [
            LhdQuestion {
                beam_energy_kev: 100.0,
                beam_power_w: 1.0e6,
                electron_density_per_m3: 3.0e19,
                electron_temperature_kev: 8.0,
                boron_density_per_m3: 7.0e17,
            },
            LhdQuestion {
                beam_energy_kev: 190.0,
                beam_power_w: 1.0e6,
                electron_density_per_m3: 1.0e18,
                electron_temperature_kev: 8.0,
                boron_density_per_m3: 1.0e17,
            },
            LhdQuestion {
                beam_energy_kev: 160.0,
                beam_power_w: 1.0e6,
                electron_density_per_m3: 1.0e18,
                electron_temperature_kev: 3.0,
                boron_density_per_m3: 1.036081e17,
            },
            LhdQuestion {
                beam_energy_kev: 161.5,
                beam_power_w: 1.0e6,
                electron_density_per_m3: 1.0e18,
                electron_temperature_kev: 1.0,
                boron_density_per_m3: 1.0e17,
            },
            LhdQuestion {
                beam_energy_kev: 180.0,
                beam_power_w: 5.0e-324,
                electron_density_per_m3: 1.96e19,
                electron_temperature_kev: 5.4,
                boron_density_per_m3: 6.3e16,
            },
        ] {
            assert!(admit(&question).is_err(), "{question:?}");
        }
    }

    /// The two beam families are two islands, each held to the inputs
    /// compared under it. The 45 keV plasma's temperature of 2.1252 keV and
    /// power of 4.515 MW do not open those values to a negative-ion beam,
    /// whose runs reached 2.9929 to 3.7872 keV and 0.9389 to 3.97 MW; and the
    /// 45 keV beam is answered only in the one plasma it was compared in.
    #[test]
    fn each_beam_family_is_held_to_its_own_compared_plasma() {
        assert_eq!((ELECTRON_TEMPERATURE_MIN_KEV, ELECTRON_TEMPERATURE_MAX_KEV), (2.9929, 3.7872));
        assert_eq!((BEAM_POWER_MIN_W, BEAM_POWER_MAX_W), (938_900.0, 3_970_000.0));
        assert_eq!(
            (
                POSITIVE_ION_BEAM_POWER_W,
                POSITIVE_ION_ELECTRON_DENSITY_PER_M3,
                POSITIVE_ION_ELECTRON_TEMPERATURE_KEV,
                POSITIVE_ION_BORON_DENSITY_PER_M3
            ),
            (4.515e6, 4.502e18, 2.1252, 8.1234e16)
        );
        // Negative-ion beams in the 45 keV plasma's temperature or power: never compared, named.
        for beam_energy_kev in [BEAM_ENERGY_MIN_KEV, 140.0, BEAM_ENERGY_MAX_KEV] {
            for electron_temperature_kev in [2.1252, 2.5, 2.99, ELECTRON_TEMPERATURE_MIN_KEV.next_down()] {
                assert_eq!(
                    solve(&LhdQuestion { beam_energy_kev, electron_temperature_kev, ..MEASURED }),
                    Err(LhdReject::ElectronTemperatureOutOfRange { electron_temperature_kev })
                );
            }
            for beam_power_w in [BEAM_POWER_MAX_W.next_up(), 4.0e6, 4.515e6] {
                assert_eq!(
                    solve(&LhdQuestion { beam_energy_kev, beam_power_w, ..MEASURED }),
                    Err(LhdReject::BeamPowerOutOfRange { beam_power_w })
                );
            }
            // The whole 45 keV plasma under a negative-ion beam: its power is named first.
            assert_eq!(
                solve(&LhdQuestion { beam_energy_kev, ..POSITIVE_ION }),
                Err(LhdReject::BeamPowerOutOfRange { beam_power_w: 4.515e6 })
            );
            assert!(solve(&LhdQuestion { beam_energy_kev, electron_temperature_kev: ELECTRON_TEMPERATURE_MIN_KEV, ..MEASURED }).is_ok());
            assert!(solve(&LhdQuestion { beam_energy_kev, beam_power_w: BEAM_POWER_MAX_W, ..MEASURED }).is_ok());
        }
        // The 45 keV beam: the compared plasma is answered; any other value of any input is named.
        assert!(solve(&POSITIVE_ION).is_ok());
        let with = |change: fn(&mut LhdQuestion)| {
            let mut question = POSITIVE_ION;
            change(&mut question);
            solve(&question)
        };
        let up = 4.515e6_f64.next_up();
        assert_eq!(with(|q| q.beam_power_w = 4.515e6_f64.next_up()), Err(LhdReject::BeamPowerOutOfRange { beam_power_w: up }));
        assert_eq!(with(|q| q.beam_power_w = 2.424e6), Err(LhdReject::BeamPowerOutOfRange { beam_power_w: 2.424e6 }));
        assert_eq!(
            with(|q| q.electron_density_per_m3 = 3.137e18),
            Err(LhdReject::ElectronDensityOutOfRange { electron_density_per_m3: 3.137e18 })
        );
        let down = 2.1252_f64.next_down();
        assert_eq!(
            with(|q| q.electron_temperature_kev = 2.1252_f64.next_down()),
            Err(LhdReject::ElectronTemperatureOutOfRange { electron_temperature_kev: down })
        );
        assert_eq!(
            with(|q| q.electron_temperature_kev = 3.2438),
            Err(LhdReject::ElectronTemperatureOutOfRange { electron_temperature_kev: 3.2438 })
        );
        assert_eq!(with(|q| q.boron_density_per_m3 = 0.0), Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3: 0.0 }));
        assert_eq!(
            with(|q| q.boron_density_per_m3 = 8.6706e16),
            Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3: 8.6706e16 })
        );
        // The measured negative-ion plasma under a 45 keV beam was never compared.
        assert_eq!(
            solve(&LhdQuestion { beam_energy_kev: 45.0, ..MEASURED }),
            Err(LhdReject::BeamPowerOutOfRange { beam_power_w: 2.424e6 })
        );
    }

    /// Every corner of the negative-ion envelope is answered, and so is the
    /// one 45 keV plasma. At the most boron for its electrons compared,
    /// quasi-neutrality holds in every shell.
    #[test]
    fn every_corner_of_the_envelope_is_answered() {
        let answer = solve(&POSITIVE_ION).expect("the 45 keV plasma is answered");
        assert!(answer.alpha_emission_rate_per_s.is_normal());
        for beam_energy_kev in [BEAM_ENERGY_MIN_KEV, BEAM_ENERGY_MAX_KEV] {
            for beam_power_w in [BEAM_POWER_MIN_W, BEAM_POWER_MAX_W] {
                for electron_density_per_m3 in [ELECTRON_DENSITY_MIN_PER_M3, ELECTRON_DENSITY_MAX_PER_M3] {
                    for electron_temperature_kev in [ELECTRON_TEMPERATURE_MIN_KEV, ELECTRON_TEMPERATURE_MAX_KEV] {
                        for share in [0.0, BORON_OVER_ELECTRON_DENSITY_MAX] {
                            // The product may round one bit above the ceiling.
                            let mut boron_density_per_m3 = share * electron_density_per_m3;
                            while boron_density_per_m3 / electron_density_per_m3 > BORON_OVER_ELECTRON_DENSITY_MAX {
                                boron_density_per_m3 = boron_density_per_m3.next_down();
                            }
                            let question = LhdQuestion {
                                beam_energy_kev,
                                beam_power_w,
                                electron_density_per_m3,
                                electron_temperature_kev,
                                boron_density_per_m3,
                            };
                            let answer = solve(&question).unwrap_or_else(|reject| panic!("{question:?}: {reject}"));
                            assert!(answer.fusion_gain.is_normal() || answer.fusion_gain == 0.0);
                            assert!(answer.alpha_emission_rate_per_s.is_normal() || share == 0.0);
                        }
                    }
                }
            }
        }
    }

    /// Quasi-neutrality: at 1e18 electrons per cubic metre, boron at 3e17
    /// would carry more charge than the electrons at the edge. Such a plasma
    /// is outside the compared envelope and is named there first; the
    /// physics names it too.
    #[test]
    fn boron_that_outweighs_the_electrons_is_named() {
        let question = LhdQuestion {
            beam_energy_kev: 160.0,
            beam_power_w: 2.0e6,
            electron_density_per_m3: 1.0e18,
            electron_temperature_kev: 2.0,
            boron_density_per_m3: 3.0e17,
        };
        assert!(matches!(quasi_neutral(&question), Err(LhdReject::BoronChargeExceedsElectrons { shell: 7 })));
        assert!(matches!(
            answer(&question, MAX_REFINEMENTS, Fault::None),
            Err(LhdReject::BoronChargeExceedsElectrons { .. })
        ));
        assert!(matches!(admit(&question), Err(LhdReject::ElectronDensityOutOfRange { .. })));
    }

    /// Attack: a refinement stopped before the fused share has settled is named.
    #[test]
    fn a_refinement_stopped_early_is_named() {
        assert_eq!(
            solve_with(&MEASURED, 1, Fault::None),
            Err(LhdReject::NotSettled { refinements: 1 })
        );
    }

    /// Attack: boron drag that slows the proton and is credited to no one
    /// leaves energy unaccounted for.
    #[test]
    fn energy_given_to_no_one_is_named() {
        // At the most boron compared the boron takes a few percent of the ion drag.
        let question = MOST_BORON;
        match solve_with(&question, MAX_REFINEMENTS, Fault::BoronDragNotCredited) {
            Err(LhdReject::EnergyLedgerBroken { residual }) => assert!(residual > 1.0e-3, "{residual}"),
            other => panic!("accepted energy given to no one: {:?}", other.is_ok()),
        }
        assert!(solve(&question).is_ok());
    }

    /// Attack: fused protons counted twice no longer add to one with the slowed.
    #[test]
    fn protons_counted_twice_are_named() {
        // The fused share is a few parts in ten million, far above the ledger's tolerance.
        match solve_with(&MOST_BORON, MAX_REFINEMENTS, Fault::FusedCountedTwice) {
            Err(LhdReject::ProtonLedgerBroken { residual }) => assert!(residual.abs() > 1.0e-9, "{residual}"),
            other => panic!("accepted protons counted twice: {:?}", other.is_ok()),
        }
    }

    /// Attack: a falsified shell answer is named by the ledger it breaks.
    #[test]
    fn a_falsified_shell_is_named() {
        let (good, _) = settle_shell(180.0, 2.2e19, 5.4, 6.3e16, MAX_REFINEMENTS, Fault::None).expect("settles");
        assert!(hold(&good).is_ok());
        let mut more = good;
        more.proton_residual = 1.0e-9;
        assert!(matches!(hold(&more), Err(LhdReject::ProtonLedgerBroken { .. })));
        let mut lost = good;
        lost.energy_residual = -1.0e-6;
        assert!(matches!(hold(&lost), Err(LhdReject::EnergyLedgerBroken { .. })));
        let mut broken = good;
        broken.fused = f64::NAN;
        assert_eq!(hold(&broken).err(), Some(LhdReject::NonFiniteValue));
    }

    #[test]
    fn the_boron_profile_is_hollow_and_one_at_its_peak() {
        let mut peak: f64 = 0.0;
        let mut at = 0.0;
        for step in 0..=1000 {
            let rho = step as f64 / 1000.0;
            let value = boron_profile(rho);
            assert!(value > 0.0);
            if value > peak {
                peak = value;
                at = rho;
            }
        }
        assert!((peak - 1.0).abs() < 1.0e-6, "{peak}");
        assert!((at - 0.655).abs() < 0.01, "{at}");
        assert!((boron_profile(0.0) - 0.413307879).abs() < 1.0e-12);
        // The deposition shares used by the solver add to one, and so do the shell volumes.
        let mut volume = 0.0;
        for shell in 0..SHELLS {
            volume += shell_volume_fraction(shell);
        }
        assert!((volume - 1.0).abs() < 1.0e-14);
    }

    /// Every request the comparison with measurement sent, runs A3, A4,
    /// A4b, A5, A5b and A6 of `runs_2026-10-09`, is inside the envelope and
    /// answered: no compared region is refused. The runs against the
    /// published calculation, A1 and A1x, are refused by name.
    #[test]
    fn every_compared_request_is_answered() {
        const RUNS: [[f64; 5]; 21] = [
            [159.56, 1311100.0, 4.268e18, 3.7872, 1.4298e17],
            [160.25, 1233300.0, 5.154e18, 3.7589, 2.04258e17],
            [148.46, 1366700.0, 4.725e18, 3.5319, 1.90212e17],
            [127.45, 1405600.0, 6.872e18, 3.3333, 2.84682e17],
            [129.35, 938900.0, 4.617e18, 2.9929, 1.64682e17],
            [128.59, 1322200.0, 6.067e18, 3.0213, 2.2596e17],
            [160.0, 3970000.0, 2.261e18, 3.2401, 5.1354e16],
            [160.0, 3970000.0, 2.964e18, 3.323, 7.8726e16],
            [160.0, 2273000.0, 3.111e18, 3.2159, 8.565e16],
            [160.0, 2424000.0, 3.137e18, 3.2438, 8.6706e16],
            [160.0, 2545000.0, 3.378e18, 3.369, 9.717e16],
            [160.0, 2606000.0, 3.356e18, 3.4745, 9.021e16],
            [160.0, 2861386.6751673575, 2.261e18, 3.2401, 5.1354e16],
            [160.0, 3751061.523748805, 2.964e18, 3.323, 7.8726e16],
            [160.0, 2254160.9818297736, 3.111e18, 3.2159, 8.565e16],
            [160.0, 2740519.6047178833, 3.378e18, 3.369, 9.717e16],
            [160.0, 2787929.8693018807, 3.356e18, 3.4745, 9.021e16],
            [45.0, 4515000.0, 4.502e18, 2.1252, 8.1234e16],
            [160.0, 2424000.0, 3.137e18, 3.2438, 0.0],
            [160.0, 1300000.0, 3.137e18, 3.2438, 8.6706e16],
            // A3 with the top panel's voltages, the protocol's reading.
            [128.8889, 938900.0, 4.617e18, 2.9929, 1.64682e17],
        ];
        for [beam_energy_kev, beam_power_w, electron_density_per_m3, electron_temperature_kev, boron_density_per_m3] in RUNS {
            let question = LhdQuestion {
                beam_energy_kev,
                beam_power_w,
                electron_density_per_m3,
                electron_temperature_kev,
                boron_density_per_m3,
            };
            solve(&question).unwrap_or_else(|reject| panic!("{question:?}: {reject}"));
        }
        for boron in [OGAWA_2022_BORON_PER_M3, 10.0 * OGAWA_2022_BORON_PER_M3] {
            for beam in 0..3 {
                assert!(matches!(solve(&ogawa_2022(beam, boron)), Err(LhdReject::BeamEnergyOutOfRange { .. })));
            }
        }
    }

    /// Run A3 of the research record, both readings of the beam voltage of
    /// Ogawa et al. 2024, Figure 10. The record's runs took the voltage of
    /// each circle of the bottom panel; the protocol names the top panel.
    /// Scored as `analysis/05_score.py` scores A3: the solver's rate over
    /// boron times protons per second times the Spitzer slowing time, one
    /// common scale, the worst factor, and the rise from 128 to 160 kV. The
    /// line is missed with either reading.
    #[test]
    fn run_a3_is_missed_with_either_beam_voltage() {
        // shot time, top-panel kV, bottom-panel kV, deposited MW, Te keV, ne, boron counts, measured
        const POINTS: [(f64, f64, f64, f64, f64, f64, f64); 6] = [
            (159.4444, 159.56, 1.3111, 3.7872, 4.268e18, 2383.0, 0.2931),
            (159.4444, 160.25, 1.2333, 3.7589, 5.154e18, 3404.3, 0.4177),
            (148.3333, 148.46, 1.3667, 3.5319, 4.725e18, 3170.2, 0.2169),
            (128.3333, 127.45, 1.4056, 3.3333, 6.872e18, 4744.7, 0.1597),
            (128.8889, 129.35, 0.9389, 2.9929, 4.617e18, 2744.7, 0.1697),
            (128.3333, 128.59, 1.3222, 3.0213, 6.067e18, 3766.0, 0.1788),
        ];
        let spitzer = |ne: f64, te: f64| {
            let log = 24.0 - ((ne * 1.0e-6).sqrt() / (te * 1.0e3)).ln();
            6.27e8 * (te * 1.0e3).powf(1.5) / (ne * 1.0e-6 * log)
        };
        let score = |top: bool| {
            let mut normalised = Vec::new();
            for (top_kv, plotted_kv, mw, te, ne, counts, measured) in POINTS {
                let kv = if top { top_kv } else { plotted_kv };
                let boron = counts * 6.0e13;
                let question = LhdQuestion {
                    beam_energy_kev: kv,
                    beam_power_w: mw * 1.0e6,
                    electron_density_per_m3: ne,
                    electron_temperature_kev: te,
                    boron_density_per_m3: boron,
                };
                let rate = solve(&question).expect("a compared point is answered").alpha_emission_rate_per_s;
                let protons = mw * 1.0e6 / (kv * JOULE_PER_KEV);
                normalised.push((kv, rate / (boron * protons * spitzer(ne, te)), measured));
            }
            let scale = (normalised.iter().map(|(_, s, m)| (s / m).ln()).sum::<f64>() / 6.0).exp();
            let worst = normalised.iter().map(|(_, s, m)| (s / m / scale).max(scale * m / s)).fold(0.0, f64::max);
            let mean = |high: bool| {
                let picked: Vec<f64> =
                    normalised.iter().filter(|(kv, _, _)| if high { *kv > 155.0 } else { *kv < 135.0 }).map(|p| p.1).collect();
                picked.iter().sum::<f64>() / picked.len() as f64
            };
            (worst, mean(true) / mean(false))
        };
        let (worst, rise) = score(false);
        println!("A3 with the bottom panel's voltages, as run: worst {worst:.4}, rise {rise:.3}");
        assert!((worst - 2.944).abs() < 1.0e-3 && (rise - 11.12).abs() < 1.0e-2, "{worst} {rise}");
        let (worst, rise) = score(true);
        println!("A3 with the top panel's voltages, as the protocol names: worst {worst:.4}, rise {rise:.3}");
        assert!((worst - 2.953).abs() < 1.0e-3 && (rise - 10.72).abs() < 1.0e-2, "{worst} {rise}");
        assert!(worst > 2.0);
    }

    /// Half the first panel width moves the settled answer by less than the tolerance asks.
    #[test]
    fn the_settled_share_does_not_depend_on_the_panels() {
        let (settled, taken) = settle_shell(160.0, 5.0e18, 2.0, 1.0e17, MAX_REFINEMENTS, Fault::None).expect("settles");
        let finer = slow_and_fuse(160.0, 5.0e18, 2.0, 1.0e17, FIRST_PANEL_KEV / 2.0_f64.powi(taken as i32), Fault::None)
            .expect("solves");
        assert!((finer.fused / settled.fused - 1.0).abs() < 10.0 * SETTLE_TOLERANCE);
    }

    /// The floor on a boron density above zero is the peak density at which
    /// the solver's own twenty shells hold one boron nucleus, worked out
    /// here as the constant's comment gives it. The nearby candidates are
    /// other numbers.
    #[test]
    fn the_boron_floor_is_one_nucleus_in_the_solvers_own_shells() {
        let mut profile_mean = 0.0;
        for shell in 0..SHELLS {
            profile_mean += boron_profile(shell_middle(shell)) * shell_volume_fraction(shell);
        }
        let nuclei_per_peak_density = plasma_volume_m3() * profile_mean;
        assert_eq!(BORON_DENSITY_ONE_NUCLEUS_PER_M3.to_bits(), (1.0 / nuclei_per_peak_density).to_bits());
        assert_eq!(BORON_DENSITY_ONE_NUCLEUS_PER_M3, 0.04973892441153873);
        // One nucleus at the floor, and less than one at the number next below it.
        assert_eq!(BORON_DENSITY_ONE_NUCLEUS_PER_M3 * nuclei_per_peak_density, 1.0);
        assert!(BORON_DENSITY_ONE_NUCLEUS_PER_M3.next_down() * nuclei_per_peak_density < 1.0);
        // The numbers of the constant's comment.
        assert!((plasma_volume_m3() - 25.582).abs() < 5.0e-4);
        assert!((profile_mean - 0.785903).abs() < 5.0e-7);
        assert!((nuclei_per_peak_density - 20.105).abs() < 5.0e-4);
        // The candidates it is not: one nucleus in the volume at the peak
        // density throughout; the polynomial's exact volume mean, the
        // integral of b(ρ) 2ρ dρ, in place of the shells; boron-11 alone.
        assert!((1.0 / plasma_volume_m3() - 0.0391).abs() < 5.0e-5);
        let mut exact_mean = 0.0;
        for (place, coefficient) in BORON_PROFILE.iter().enumerate() {
            let power = (BORON_PROFILE.len() - 1 - place) as f64;
            exact_mean += coefficient * 2.0 / (power + 2.0);
        }
        assert!((exact_mean - 0.784763).abs() < 5.0e-7);
        assert!((1.0 / (plasma_volume_m3() * exact_mean) - 0.0498).abs() < 5.0e-5);
        assert!((BORON_DENSITY_ONE_NUCLEUS_PER_M3 / BORON11_FRACTION - 0.0622).abs() < 5.0e-5);
        // Eighteen powers of ten below the smallest density above zero that was compared.
        assert!(5.1354e16 / BORON_DENSITY_ONE_NUCLEUS_PER_M3 > 1.0e18);
    }

    /// Every number of an answer as its bits: the eleven numbers, the shells
    /// folded into one, and the refinements.
    fn bits(answer: &LhdBeamFusion) -> ([u64; 11], u64, u32) {
        let mut shells = 0_u64;
        for emissivity in &answer.alpha_emissivity_per_m3_s {
            shells = shells.rotate_left(7) ^ emissivity.to_bits();
        }
        (
            [
                answer.alpha_emission_rate_per_s.to_bits(),
                answer.fusion_power_w.to_bits(),
                answer.fusion_gain.to_bits(),
                answer.fused_fraction.to_bits(),
                answer.peak_alpha_emissivity_per_m3_s.to_bits(),
                answer.peak_emissivity_radius.to_bits(),
                answer.slowing_time_s.to_bits(),
                answer.energy_fraction_to_electrons.to_bits(),
                answer.energy_fraction_to_ions.to_bits(),
                answer.proton_ledger_residual.to_bits(),
                answer.energy_ledger_residual.to_bits(),
            ],
            shells,
            answer.refinements,
        )
    }

    /// A boron density that is not zero and is below one nucleus in the
    /// plasma is refused by its own name, from the question alone and before
    /// any refinement. Zero, the floor itself and the measured point are
    /// answered with the bits they had before the floor was written.
    #[test]
    fn boron_below_one_nucleus_is_named_and_zero_is_answered_as_before() {
        let ask = |boron_density_per_m3: f64| LhdQuestion { boron_density_per_m3, ..MEASURED };
        let floor = BORON_DENSITY_ONE_NUCLEUS_PER_M3;
        // 1e-297 took many seconds and returned zero; 1.8008101798042446e-289 is among the smallest numbers too.
        for boron_density_per_m3 in [floor.next_down(), 0.04, 1.0e-3, 1.0e-100, 1.8008101798042446e-289, 1.0e-290, 1.0e-297, f64::MIN_POSITIVE, 5.0e-324] {
            let named = LhdReject::BoronBelowOneNucleus { boron_density_per_m3 };
            // `admit` integrates nothing.
            assert_eq!(admit(&ask(boron_density_per_m3)), Err(named.clone()));
            assert_eq!(solve(&ask(boron_density_per_m3)), Err(named.clone()));
            // With no refinement allowed, a question that is admitted ends
            // on `NotSettled`: the name comes before the first refinement.
            assert_eq!(solve_with(&ask(boron_density_per_m3), 0, Fault::None), Err(named));
        }
        assert_eq!(solve_with(&ask(floor), 0, Fault::None), Err(LhdReject::NotSettled { refinements: 0 }));
        assert_eq!(
            LhdReject::BoronBelowOneNucleus { boron_density_per_m3: 1.0e-297 }.to_string(),
            "boron density 1e-297 m^-3 is not zero and is below 0.04973892441153873 m^-3, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered"
        );
        // Under every beam energy compared, with the rest of the measured point.
        for beam_energy_kev in [BEAM_ENERGY_MIN_KEV, BEAM_ENERGY_MAX_KEV] {
            assert_eq!(
                solve(&LhdQuestion { beam_energy_kev, boron_density_per_m3: 1.0e-297, ..MEASURED }),
                Err(LhdReject::BoronBelowOneNucleus { boron_density_per_m3: 1.0e-297 })
            );
        }
        // What was refused before keeps its name: a density below zero, and
        // any density but its own under the 45 keV beam.
        assert_eq!(solve(&ask(-1.0e-297)), Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3: -1.0e-297 }));
        assert_eq!(
            solve(&LhdQuestion { boron_density_per_m3: 1.0e-297, ..POSITIVE_ION }),
            Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3: 1.0e-297 })
        );

        // No boron, the control of the comparison (run A5b): one refinement, no alphas, these bits.
        let none = solve(&ask(0.0)).expect("no boron is answered");
        assert_eq!(
            bits(&none),
            (
                [
                    0,
                    0,
                    0,
                    0,
                    0,
                    0x3f9999999999999a,
                    0x3fe93b7241166a40,
                    0x3fe47f998dfa769d,
                    0x3fd300cce40b12c4,
                    0,
                    0x3cb999999999999a,
                ],
                0,
                1
            )
        );
        assert_eq!(none.alpha_emissivity_per_m3_s, vec![0.0; SHELLS]);
        // The floor itself: one boron nucleus in the plasma.
        let one = solve(&ask(floor)).expect("the floor is answered");
        assert_eq!(
            bits(&one),
            (
                [
                    0x3edf53949bff1e8c,
                    0x3c4fef0c6a7d3ffa,
                    0x3afba0bd2e72223a,
                    0x3aa04bf02e45aebb,
                    0x3ea59e99d9e3debb,
                    0x3fd8000000000000,
                    0x3fe93b7241166a40,
                    0x3fe47f998dfa769e,
                    0x3fd300cce40b12c4,
                    0x3aa57784708308bf,
                    0x3cc6666666666666,
                ],
                0x3d66a9a9fb2a6bad,
                3
            )
        );
        assert_eq!(one.fused_fraction, 2.632863159805441e-26);
        // The measured point.
        let measured = solve(&MEASURED).expect("solves");
        assert_eq!(
            bits(&measured),
            (
                [
                    0x42a7dfb8201506eb,
                    0x401856338af0be71,
                    0x3ec50e21b5efc26a,
                    0x3e68d6dd725bbc11,
                    0x42707f1ab47e949a,
                    0x3fd8000000000000,
                    0x3fe9cf57824999b8,
                    0x3fe4ccddde2f5ec5,
                    0x3fd2664418d3c01b,
                    0x3cc3e8f000000000,
                    0x3da47e5000000000,
                ],
                0x6354c8d735279a36,
                3
            )
        );
        // The check of the question adds nothing to an answer: each is the
        // physics of the question alone, which this floor does not enter.
        for question in [ask(0.0), ask(floor), MEASURED, MOST_BORON] {
            let direct = answer(&question, MAX_REFINEMENTS, Fault::None).expect("solves");
            assert_eq!(bits(&solve(&question).expect("solves")), bits(&direct));
        }
    }

    /// No admitted boron density above zero takes more than the three
    /// refinements the measured point takes, and none ends on `NotSettled`:
    /// a ladder from the floor to the most boron the electrons allow, in the
    /// measured point's plasma and in the plasma of the largest density
    /// compared.
    #[test]
    fn every_admitted_boron_density_settles_within_three_refinements() {
        assert_eq!(solve(&MEASURED).expect("solves").refinements, 3);
        let floor = BORON_DENSITY_ONE_NUCLEUS_PER_M3;
        for base in [MEASURED, MOST_BORON] {
            // The product may round one bit above the ceiling of the share.
            let mut most = (BORON_OVER_ELECTRON_DENSITY_MAX * base.electron_density_per_m3).min(BORON_DENSITY_MAX_PER_M3);
            while most / base.electron_density_per_m3 > BORON_OVER_ELECTRON_DENSITY_MAX {
                most = most.next_down();
            }
            let rungs = 12;
            for rung in 0..=rungs {
                let boron_density_per_m3 = if rung == 0 {
                    floor
                } else if rung == rungs {
                    most
                } else {
                    floor * (most / floor).powf(rung as f64 / rungs as f64)
                };
                let question = LhdQuestion { boron_density_per_m3, ..base };
                let answer = solve(&question).unwrap_or_else(|reject| panic!("{question:?}: {reject}"));
                assert!((2..=3).contains(&answer.refinements), "{question:?}: {}", answer.refinements);
                assert!(answer.fused_fraction.is_normal() && answer.fused_fraction > 0.0, "{question:?}");
            }
        }
        assert_eq!(MOST_BORON.boron_density_per_m3, BORON_DENSITY_MAX_PER_M3);
    }
}
