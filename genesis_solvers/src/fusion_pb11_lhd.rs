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
//! `evidence/research/fusion_pb11/solver/`: it met one of its four lines.
//!
//! The declared range is where these equations hold, each end with its
//! physical reason at its constant: a beam above the stopping energy to the
//! 190 keV of the device's negative-ion beams, any deposited power above
//! zero, line-averaged electron density 1e18 to 3e19 per cubic metre,
//! central temperature 0.5 keV to two thirds of the stopping energy, boron
//! from zero to what quasi-neutrality allows. From engine 0.7.0 to this
//! version it answered only inside the envelope of the inputs compared with
//! measurement, each beam family its own island and the 45 keV beam in one
//! plasma only. A boron density that is not zero and is below one boron
//! nucleus in the whole plasma ([`BORON_DENSITY_ONE_NUCLEUS_PER_M3`]) is
//! refused by its own name: no boron is asked as zero, which is answered.
//!
//! *Fast protons.* The beam protons slowing in a shell are, in steady state,
//! the protons born there per second times the time each takes to slow:
//! `n_fast = (born per m³ per s) · τ_slow`. They are positive charge beside
//! the thermal protons and the boron, and the electrons must carry it all.
//! The solver takes the thermal protons as `n_e − 5 n_B` and slows each beam
//! proton on them alone (the test-particle model: no beam proton slows on
//! another). Where in any shell `n_fast + 5 n_B` is more than the electrons
//! there, no thermal proton could be left: quasi-neutrality cannot hold
//! with the fast protons this solve finds, and the answer is refused,
//! [`LhdReject::FastProtonChargeExceedsElectrons`]. Below that end the
//! thermal protons the drag uses are `n_fast` more than quasi-neutrality
//! leaves; the largest share is returned, `fast_proton_fraction_max`.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
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

/// Energy of the beam protons, keV: above [`STOPPING_ENERGY_KEV`], where a
/// proton has nothing left to follow, to 190 keV, the rating of the
/// device's negative-ion beams, the most energetic beams the object has.
/// The equations go further (the cross-section's fit reaches 10.6 MeV in the
/// laboratory, and a 190 keV proton's gyroradius at 2.75 T is 2.3 cm against
/// the 60 cm minor radius); the end is the object's. Compared with
/// measurement at 45 keV and from 127.45 to 160.25 keV.
pub const BEAM_ENERGY_MIN_KEV: f64 = STOPPING_ENERGY_KEV;
pub const BEAM_ENERGY_MAX_KEV: f64 = 190.0;
/// Line-averaged electron density, per cubic metre: 1e18 to 3e19, the span
/// of the device's proton-boron discharges. Up to the top the ionisation
/// length of a 45 to 190 keV hydrogen beam stays above the minor radius (an
/// estimate), so the core-peaked deposition the solver takes is not
/// contradicted. Toward the bottom a beam's fast protons grow as one over
/// the density squared against the electrons, and where they would
/// outnumber them the solver stops by name. Compared with measurement from
/// 2.261e18 to 6.872e18.
pub const ELECTRON_DENSITY_MIN_PER_M3: f64 = 1.0e18;
pub const ELECTRON_DENSITY_MAX_PER_M3: f64 = 3.0e19;
/// Central electron temperature, keV. Low end: below 0.5 keV boron at the
/// axis is not fully stripped (the last electron of B⁴⁺ is bound by 0.340
/// keV), and the solver takes boron as a bare nucleus of charge 5. High end:
/// a proton is followed down to [`STOPPING_ENERGY_KEV`] and joins no thermal
/// plasma, which holds only while that energy is at least 1.5 times the ion
/// temperature, here the electron temperature: two thirds of 10 keV.
/// Compared with measurement at 2.1252 keV and from 2.9929 to 3.7872 keV.
pub const ELECTRON_TEMPERATURE_MIN_KEV: f64 = 0.5;
pub const ELECTRON_TEMPERATURE_MAX_KEV: f64 = STOPPING_ENERGY_KEV / 1.5;
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
/// Beam power deposited, watts: any power above zero. The top, 1e300 W, is a
/// capacity and no physics, so the input has a top a request can be told:
/// the fast protons of a beam stop the solve by name far below it.
pub const BEAM_POWER_MAX_W: f64 = 1.0e300;
/// The fast protons and the boron of a shell may carry at most the charge
/// of its electrons: `(n_fast + 5 n_B) / n_e` at most this.
pub const FAST_PROTON_CHARGE_OVER_ELECTRONS_MAX: f64 = 1.0;

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
    /// `beam_energy_kev` is not above the 10 keV a proton is followed down to, where there is nothing to follow, or is above 190 keV, the rating of the device's negative-ion beams.
    BeamEnergyOutOfRange { beam_energy_kev: f64 },
    /// `beam_power_w` is not above zero, which is no beam, or is above the 1e300 W capacity of the input.
    BeamPowerOutOfRange { beam_power_w: f64 },
    /// `electron_density_per_m3` is outside 1e18 to 3e19 per cubic metre, the device's proton-boron discharges: above it the beam's deposition is no longer core-peaked.
    ElectronDensityOutOfRange { electron_density_per_m3: f64 },
    /// `electron_temperature_kev` is outside 0.5 to 6.67 keV: below it boron at the axis is not fully stripped, above it the 10 keV a proton is followed to is less than 1.5 ion temperatures and the proton would be thermal.
    ElectronTemperatureOutOfRange { electron_temperature_kev: f64 },
    /// `boron_density_per_m3` is below zero: no density.
    BoronDensityOutOfRange { boron_density_per_m3: f64 },
    /// `boron_density_per_m3` is not zero and is below [`BORON_DENSITY_ONE_NUCLEUS_PER_M3`], the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered.
    BoronBelowOneNucleus { boron_density_per_m3: f64 },
    /// Quasi-neutrality: in one shell the boron alone carries more charge than the electrons there.
    BoronChargeExceedsElectrons { shell: usize },
    /// An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.
    NonFiniteValue,
    /// At some energy between the beam energy and the stopping energy a beam proton gains energy from the plasma of a shell: it is no longer slowing.
    SlowingRateNotPositive { shell: usize },
    /// The refinement cap was reached with the fused share of a shell still moving.
    NotSettled { refinements: u32 },
    /// In one shell the slowed and the fused shares left [`PROTON_LEDGER_TOLERANCE`] of one.
    ProtonLedgerBroken { residual: f64 },
    /// In one shell the energy ledger left [`ENERGY_LEDGER_TOLERANCE`] of the beam energy.
    EnergyLedgerBroken { residual: f64 },
    /// Quasi-neutrality with the fast protons counted: in one shell the beam protons still slowing, born per second times the time each takes to slow, and the boron carry more charge than the electrons there, so no thermal proton could be left.
    FastProtonChargeExceedsElectrons { shell: usize, fast_proton_fraction: f64, charge_over_electrons: f64 },
}

impl fmt::Display for LhdReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeamEnergyOutOfRange { beam_energy_kev } => write!(
                f,
                "beam energy {beam_energy_kev} keV is not above {BEAM_ENERGY_MIN_KEV} keV, the energy a proton is followed down to, or is above {BEAM_ENERGY_MAX_KEV} keV, the rating of the device's negative-ion beams"
            ),
            Self::BeamPowerOutOfRange { beam_power_w } => write!(
                f,
                "beam power {beam_power_w} W is not above 0 W, which is no beam, or is above {BEAM_POWER_MAX_W:e} W, the capacity of the input"
            ),
            Self::ElectronDensityOutOfRange { electron_density_per_m3 } => write!(
                f,
                "electron density {electron_density_per_m3} m^-3 is outside {ELECTRON_DENSITY_MIN_PER_M3:e} to {ELECTRON_DENSITY_MAX_PER_M3:e} m^-3, the device's proton-boron discharges"
            ),
            Self::ElectronTemperatureOutOfRange { electron_temperature_kev } => write!(
                f,
                "electron temperature {electron_temperature_kev} keV is outside {ELECTRON_TEMPERATURE_MIN_KEV} to {ELECTRON_TEMPERATURE_MAX_KEV} keV, two thirds of the {STOPPING_ENERGY_KEV} keV stopping energy: below it boron at the axis is not fully stripped, above it a proton at the stopping energy would be thermal"
            ),
            Self::BoronDensityOutOfRange { boron_density_per_m3 } => {
                write!(f, "boron density {boron_density_per_m3} m^-3 is below 0 m^-3: no density")
            }
            Self::BoronBelowOneNucleus { boron_density_per_m3 } => write!(
                f,
                "boron density {boron_density_per_m3:e} m^-3 is not zero and is below {BORON_DENSITY_ONE_NUCLEUS_PER_M3} m^-3, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered"
            ),
            Self::BoronChargeExceedsElectrons { shell } => write!(
                f,
                "in shell {shell} of {SHELLS} the boron carries more charge than the electrons: quasi-neutrality cannot hold"
            ),
            Self::NonFiniteValue => write!(f, "an input or an intermediate value was not finite"),
            Self::SlowingRateNotPositive { shell } => write!(
                f,
                "in shell {shell} of {SHELLS} a beam proton gains energy from the plasma before it reaches the stopping energy"
            ),
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
            Self::FastProtonChargeExceedsElectrons { shell, fast_proton_fraction, charge_over_electrons } => write!(
                f,
                "in shell {shell} of {SHELLS} the beam protons still slowing are {fast_proton_fraction} of the electrons, and with the boron they carry {charge_over_electrons} of the electrons' charge: no thermal proton could be left, and quasi-neutrality cannot hold"
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
    /// Largest density of beam protons still slowing over the electron
    /// density, of any shell: born per second per cubic metre times the
    /// shell's slowing time.
    pub fast_proton_fraction_max: f64,
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

/// The named limits a question itself shows: the five ranges, each end with
/// its physical reason at its constant, a boron density above zero and
/// below one nucleus in the plasma, and quasi-neutrality. Nothing is
/// integrated here.
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
    if !(beam_energy_kev > BEAM_ENERGY_MIN_KEV && beam_energy_kev <= BEAM_ENERGY_MAX_KEV) {
        return Err(LhdReject::BeamEnergyOutOfRange { beam_energy_kev });
    }
    if !(beam_power_w > 0.0 && beam_power_w <= BEAM_POWER_MAX_W) {
        return Err(LhdReject::BeamPowerOutOfRange { beam_power_w });
    }
    if !(ELECTRON_DENSITY_MIN_PER_M3..=ELECTRON_DENSITY_MAX_PER_M3).contains(&electron_density_per_m3) {
        return Err(LhdReject::ElectronDensityOutOfRange { electron_density_per_m3 });
    }
    if !(ELECTRON_TEMPERATURE_MIN_KEV..=ELECTRON_TEMPERATURE_MAX_KEV).contains(&electron_temperature_kev) {
        return Err(LhdReject::ElectronTemperatureOutOfRange { electron_temperature_kev });
    }
    if !(boron_density_per_m3 >= 0.0) {
        return Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3 });
    }
    // Less than one boron nucleus in the whole plasma is no boron, and no
    // boron is asked as zero, which is answered.
    if boron_density_per_m3 != 0.0 && boron_density_per_m3 < BORON_DENSITY_ONE_NUCLEUS_PER_M3 {
        return Err(LhdReject::BoronBelowOneNucleus { boron_density_per_m3 });
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
    shell: usize,
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
            if !total.is_finite() {
                return Err(LhdReject::NonFiniteValue);
            }
            if !(total > 0.0) {
                return Err(LhdReject::SlowingRateNotPositive { shell });
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
        let midway = surviving * math::exp(-0.5 * chance);
        let fused_here = -surviving * math::exp_m1(-chance);
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
    shell: usize,
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
            shell,
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
        0,
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
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &LhdQuestion) -> Result<LhdBeamFusion, LhdReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &LhdQuestion) -> Result<LhdBeamFusion, LhdReject> {
    solve_with(question, MAX_REFINEMENTS, Fault::None)
}

impl NormalOrZero for LhdBeamFusion {
    fn normal_or_zero(&mut self) {
        self.alpha_emission_rate_per_s.normal_or_zero();
        self.fusion_power_w.normal_or_zero();
        self.fusion_gain.normal_or_zero();
        self.fused_fraction.normal_or_zero();
        self.peak_alpha_emissivity_per_m3_s.normal_or_zero();
        self.peak_emissivity_radius.normal_or_zero();
        self.slowing_time_s.normal_or_zero();
        self.fast_proton_fraction_max.normal_or_zero();
        self.energy_fraction_to_electrons.normal_or_zero();
        self.energy_fraction_to_ions.normal_or_zero();
        self.proton_ledger_residual.normal_or_zero();
        self.energy_ledger_residual.normal_or_zero();
        self.alpha_emissivity_per_m3_s.normal_or_zero();
    }
}

fn solve_with(question: &LhdQuestion, refinement_cap: u32, fault: Fault) -> Result<LhdBeamFusion, LhdReject> {
    admit(question)?;
    answer(question, refinement_cap, fault)
}

/// The physics of one question, with no range asked: quasi-neutrality, then
/// each shell settled and held to its ledgers and its charge.
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
    let mut fast_proton_fraction_max: f64 = 0.0;
    for (shell, share) in deposited.iter().enumerate() {
        let share = share / deposited_sum;
        let (temperature, electrons, boron) = plasma_at(question, shell_middle(shell));
        let (answer, taken) = settle_shell(shell, question.beam_energy_kev, electrons, temperature, boron, refinement_cap, fault)?;
        // Quasi-neutrality with the fast protons this shell holds in steady state.
        let fast_protons = protons_per_s * share / (volume_m3 * shell_volume_fraction(shell)) * answer.slowing_time_s;
        let fast_proton_fraction = fast_protons / electrons;
        let charge_over_electrons = (fast_protons + 5.0 * boron) / electrons;
        if !fast_proton_fraction.is_finite() {
            return Err(LhdReject::NonFiniteValue);
        }
        if !(charge_over_electrons <= FAST_PROTON_CHARGE_OVER_ELECTRONS_MAX) {
            return Err(LhdReject::FastProtonChargeExceedsElectrons { shell, fast_proton_fraction, charge_over_electrons });
        }
        fast_proton_fraction_max = fast_proton_fraction_max.max(fast_proton_fraction);
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
        fast_proton_fraction_max,
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

    /// A measured point that is answered: Ogawa et al. 2024, Figure 10, the
    /// first point of run A3, the deposited power as given. Its fast protons
    /// are 0.78 of the electrons at the axis.
    const ANSWERED: LhdQuestion = LhdQuestion {
        beam_energy_kev: 159.56,
        beam_power_w: 1_311_100.0,
        electron_density_per_m3: 4.268e18,
        electron_temperature_kev: 3.7872,
        boron_density_per_m3: 1.4298e17,
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
    /// both ratios: it does not hold either to the published one. From
    /// engine 0.7.0 to this version the served solver refused this plasma,
    /// outside the envelope compared with measurement; it is answered now,
    /// with the same numbers the physics gave then.
    #[test]
    fn ogawa_2022_three_beams_of_discharge_168156() {
        println!("Ogawa et al. 2022 read {REFERENCE_RETRIEVED} from {REFERENCE_URL}");
        for (boron, low, high) in [
            (OGAWA_2022_BORON_PER_M3, 1.8e13, 2.1e13),
            (10.0 * OGAWA_2022_BORON_PER_M3, 1.8e14, 2.1e14),
        ] {
            let mut rate = 0.0;
            for beam in 0..3 {
                let served = solve(&ogawa_2022(beam, boron)).expect("answered");
                let direct = answer(&ogawa_2022(beam, boron), MAX_REFINEMENTS, Fault::None).expect("solves");
                assert_eq!(served, direct);
                assert!(served.fast_proton_fraction_max < 0.2, "{}", served.fast_proton_fraction_max);
                rate += served.alpha_emission_rate_per_s;
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
        let answer = solve(&ANSWERED).expect("solves");
        assert!(answer.proton_ledger_residual <= PROTON_LEDGER_TOLERANCE);
        assert!(answer.energy_ledger_residual <= ENERGY_LEDGER_TOLERANCE);
        // Almost all of the beam energy goes to the plasma; the rest is what
        // a proton still holds at the stopping energy.
        let handed = answer.energy_fraction_to_electrons + answer.energy_fraction_to_ions;
        let kept = STOPPING_ENERGY_KEV / ANSWERED.beam_energy_kev;
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
        assert!((answer.fusion_gain * ANSWERED.beam_power_w / answer.fusion_power_w - 1.0).abs() < 1.0e-12);
    }

    /// The gain is the fused share times 8.68 MeV over the beam energy: it
    /// does not pass through the power, and it is the same from the smallest
    /// power above zero to the measured one.
    #[test]
    fn the_gain_does_not_pass_through_the_power() {
        let mut gains = Vec::new();
        for beam_power_w in [5.0e-324, 1.0, 938_900.0, ANSWERED.beam_power_w] {
            let answer = solve(&LhdQuestion { beam_power_w, ..ANSWERED }).expect("solves");
            assert_eq!(
                answer.fusion_gain.to_bits(),
                (answer.fused_fraction * REACTION_ENERGY_KEV / ANSWERED.beam_energy_kev).to_bits()
            );
            assert!(answer.fusion_gain > 1.0e-7, "{}", answer.fusion_gain);
            gains.push(answer.fusion_gain.to_bits());
        }
        gains.dedup();
        assert_eq!(gains.len(), 1);
    }

    /// Any power above zero is a beam. The 0.9389 MW floor of 0.7.0 to this
    /// version guarded the gain, which no longer passes through the power:
    /// the smallest number above zero is answered with the same gain and an
    /// alpha rate in proportion. Zero and below are no beam, by name. A power
    /// too great for the plasma is stopped by its fast protons.
    #[test]
    fn any_power_above_zero_is_answered_and_none_is_named() {
        for beam_power_w in [0.0, -0.0, -1.0] {
            assert_eq!(
                solve(&LhdQuestion { beam_power_w, ..ANSWERED }),
                Err(LhdReject::BeamPowerOutOfRange { beam_power_w })
            );
        }
        let base = solve(&ANSWERED).expect("solves");
        let tiny = solve(&LhdQuestion { beam_power_w: 1.0, ..ANSWERED }).expect("solves");
        assert!((tiny.alpha_emission_rate_per_s * ANSWERED.beam_power_w / base.alpha_emission_rate_per_s - 1.0).abs() < 1.0e-12);
        assert!(tiny.fast_proton_fraction_max < 1.0e-6);
        // Six megawatts, one beam's stated maximum, into this plasma: more fast protons than electrons.
        assert!(matches!(
            solve(&LhdQuestion { beam_power_w: 6.0e6, ..ANSWERED }),
            Err(LhdReject::FastProtonChargeExceedsElectrons { shell: 0, .. })
        ));
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let first = solve(&MOST_BORON).expect("solves");
        let second = solve(&MOST_BORON).expect("solves");
        assert_eq!(first.alpha_emission_rate_per_s.to_bits(), second.alpha_emission_rate_per_s.to_bits());
        assert_eq!(first.fast_proton_fraction_max.to_bits(), second.fast_proton_fraction_max.to_bits());
        for (left, right) in first.alpha_emissivity_per_m3_s.iter().zip(&second.alpha_emissivity_per_m3_s) {
            assert_eq!(left.to_bits(), right.to_bits());
        }
    }

    #[test]
    fn the_rate_follows_power_boron_and_beam_energy() {
        let base = solve(&ANSWERED).expect("solves");
        let halved = solve(&LhdQuestion { beam_power_w: 0.5 * ANSWERED.beam_power_w, ..ANSWERED }).expect("solves");
        assert!((halved.alpha_emission_rate_per_s / base.alpha_emission_rate_per_s - 0.5).abs() < 1.0e-12);
        assert_eq!(halved.fused_fraction.to_bits(), base.fused_fraction.to_bits());
        assert!((halved.fast_proton_fraction_max / base.fast_proton_fraction_max - 0.5).abs() < 1.0e-12);
        // No boron, no alphas.
        let none = solve(&LhdQuestion { boron_density_per_m3: 0.0, ..ANSWERED }).expect("solves");
        assert_eq!(none.alpha_emission_rate_per_s, 0.0);
        assert_eq!(none.fused_fraction, 0.0);
        // A 45 keV beam never reaches the reactive energies.
        let slow = solve(&LhdQuestion { beam_energy_kev: 45.0, ..ANSWERED }).expect("solves");
        assert!(slow.fused_fraction < 1.0e-4 * base.fused_fraction, "{}", slow.fused_fraction);
        // A hotter plasma slows the proton less, so more of them fuse.
        let hot = LhdQuestion { electron_temperature_kev: 4.5, beam_power_w: 0.5 * ANSWERED.beam_power_w, ..ANSWERED };
        assert!(solve(&hot).expect("solves").fused_fraction > base.fused_fraction);
    }

    /// Each end of the declared ranges is answered or stopped by a physical
    /// limit of the solve, and the next number beyond it is refused at the
    /// door by name. The requests 0.6.0 answered with a proof, refused by
    /// name from 0.7.0 to this version, are admitted again.
    #[test]
    fn each_end_of_the_ranges_is_answered_and_the_next_number_is_named() {
        let in_the_solve = |question: LhdQuestion| match solve(&question) {
            Ok(answer) => answer.proton_ledger_residual <= PROTON_LEDGER_TOLERANCE && answer.energy_ledger_residual <= ENERGY_LEDGER_TOLERANCE,
            Err(LhdReject::FastProtonChargeExceedsElectrons { .. } | LhdReject::BoronChargeExceedsElectrons { .. }) => true,
            Err(_) => false,
        };
        let at = |change: fn(&mut LhdQuestion, f64), value: f64| {
            let mut question = ANSWERED;
            question.boron_density_per_m3 = 1.0e16;
            change(&mut question, value);
            question
        };
        type Change = fn(&mut LhdQuestion, f64);
        let ranges: [(Change, f64, f64, fn(f64) -> LhdReject); 3] = [
            (|q, v| q.beam_energy_kev = v, BEAM_ENERGY_MIN_KEV.next_up(), BEAM_ENERGY_MAX_KEV, |v| LhdReject::BeamEnergyOutOfRange { beam_energy_kev: v }),
            (|q, v| q.electron_density_per_m3 = v, ELECTRON_DENSITY_MIN_PER_M3, ELECTRON_DENSITY_MAX_PER_M3, |v| LhdReject::ElectronDensityOutOfRange { electron_density_per_m3: v }),
            (|q, v| q.electron_temperature_kev = v, ELECTRON_TEMPERATURE_MIN_KEV, ELECTRON_TEMPERATURE_MAX_KEV, |v| LhdReject::ElectronTemperatureOutOfRange { electron_temperature_kev: v }),
        ];
        for (change, low, high, reject) in ranges {
            assert!(in_the_solve(at(change, low)), "{low}");
            assert!(in_the_solve(at(change, high)), "{high}");
            assert_eq!(solve(&at(change, low.next_down())), Err(reject(low.next_down())));
            assert_eq!(solve(&at(change, high.next_up())), Err(reject(high.next_up())));
        }
        // The beam at exactly the stopping energy has nothing to follow.
        assert_eq!(solve(&at(|q, v| q.beam_energy_kev = v, STOPPING_ENERGY_KEV)), Err(LhdReject::BeamEnergyOutOfRange { beam_energy_kev: STOPPING_ENERGY_KEV }));
        assert_eq!(ELECTRON_TEMPERATURE_MAX_KEV * 1.5, STOPPING_ENERGY_KEV);
        assert_eq!(
            solve(&LhdQuestion { boron_density_per_m3: -1.0, ..ANSWERED }),
            Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3: -1.0 })
        );
        assert_eq!(solve(&LhdQuestion { beam_energy_kev: f64::NAN, ..ANSWERED }), Err(LhdReject::NonFiniteValue));
        assert_eq!(solve(&LhdQuestion { boron_density_per_m3: f64::INFINITY, ..ANSWERED }), Err(LhdReject::NonFiniteValue));
        // More boron for its electrons than any compared, 0.05 of them: answered.
        let thinner = LhdQuestion { electron_density_per_m3: 4.0e18, boron_density_per_m3: 2.0e17, beam_power_w: 1.0e6, ..ANSWERED };
        assert!(solve(&thinner).is_ok());
        // Requests 0.6.0 answered with a proof outside the envelope.
        for question in [
            LhdQuestion {
                beam_energy_kev: 100.0,
                beam_power_w: 1.0e6,
                electron_density_per_m3: 3.0e19,
                electron_temperature_kev: 6.0,
                boron_density_per_m3: 7.0e17,
            },
            LhdQuestion {
                beam_energy_kev: 190.0,
                beam_power_w: 1.0e6,
                electron_density_per_m3: 1.0e18,
                electron_temperature_kev: 6.0,
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
            assert!(admit(&question).is_ok(), "{question:?}");
            assert!(in_the_solve(question), "{question:?}");
        }
    }

    /// The measured plasmas of Figure 6, runs A4 to A6 and the 45 keV beam of
    /// A5, answered from engine 0.7.0 to this version, hold more beam
    /// protons than electrons at the axis when the beam power given is
    /// slowed classically there with no loss: 1.25 to 7.25 times the
    /// electrons. No plasma can: they are refused by name. The runs of
    /// Figure 10, A3, given the deposited power, are answered: at most 0.78.
    #[test]
    fn the_plasmas_of_figure_6_hold_more_fast_protons_than_electrons_and_are_named() {
        for (question, low, high) in [(MEASURED, 2.3, 2.4), (POSITIVE_ION, 1.7, 1.8)] {
            match solve(&question) {
                Err(LhdReject::FastProtonChargeExceedsElectrons { shell, fast_proton_fraction, charge_over_electrons }) => {
                    assert_eq!(shell, 0);
                    assert!(fast_proton_fraction > low && fast_proton_fraction < high, "{fast_proton_fraction}");
                    assert!(charge_over_electrons > fast_proton_fraction);
                }
                other => panic!("{question:?}: {:?}", other.map(|answer| answer.fast_proton_fraction_max)),
            }
        }
        // The same plasma with a quarter of the power: answered, and its fast protons a quarter as many.
        let quarter = solve(&LhdQuestion { beam_power_w: 0.25 * MEASURED.beam_power_w, ..MEASURED }).expect("answered");
        assert!((quarter.fast_proton_fraction_max - 0.25 * 2.3255).abs() < 1.0e-3, "{}", quarter.fast_proton_fraction_max);
        let answered = solve(&ANSWERED).expect("answered");
        assert!((answered.fast_proton_fraction_max - 0.7755).abs() < 1.0e-3, "{}", answered.fast_proton_fraction_max);
    }

    /// The fast protons of a shell are those born there per second, per
    /// cubic metre, times the shell's slowing time, recomputed here for the
    /// axis shell of the answered point.
    #[test]
    fn the_fast_protons_are_the_born_rate_times_the_slowing_time() {
        let answer = solve(&ANSWERED).expect("solves");
        let mut deposited_sum = 0.0;
        for shell in 0..SHELLS {
            let rho = shell_middle(shell);
            deposited_sum += (1.0 - rho * rho) * shell_volume_fraction(shell);
        }
        let mut largest: f64 = 0.0;
        for shell in 0..SHELLS {
            let rho = shell_middle(shell);
            let (temperature, electrons, boron) = plasma_at(&ANSWERED, rho);
            let (slowed, _) = settle_shell(shell, ANSWERED.beam_energy_kev, electrons, temperature, boron, MAX_REFINEMENTS, Fault::None).expect("settles");
            let born = ANSWERED.beam_power_w / (ANSWERED.beam_energy_kev * JOULE_PER_KEV) * (1.0 - rho * rho) * shell_volume_fraction(shell)
                / deposited_sum
                / (plasma_volume_m3() * shell_volume_fraction(shell));
            largest = largest.max(born * slowed.slowing_time_s / electrons);
        }
        assert!((largest / answer.fast_proton_fraction_max - 1.0).abs() < 1.0e-12, "{largest}");
    }

    /// Attack on the reopened range: every corner of it (beam just above the
    /// stopping energy and 190 keV, 1 W and 16 MW, 1e18 and 3e19 per cubic
    /// metre, 0.5 and 6.67 keV, no boron and boron at 5 and at 19 percent of
    /// the electrons) closes both ledgers or is stopped by a physical limit by
    /// name: the fast protons or the boron outweighing the electrons. When
    /// written: 42 of 64 answered, 6 stopped by their fast protons, 16 by
    /// their boron.
    #[test]
    fn every_corner_of_the_reopened_range_closes_or_stops_by_name() {
        let mut answered = 0;
        for corner in 0..64_u32 {
            let pick = |bit: u32, low: f64, high: f64| if corner & (1 << bit) == 0 { low } else { high };
            let electron_density_per_m3 = pick(2, ELECTRON_DENSITY_MIN_PER_M3, ELECTRON_DENSITY_MAX_PER_M3);
            let question = LhdQuestion {
                beam_energy_kev: pick(0, BEAM_ENERGY_MIN_KEV.next_up(), BEAM_ENERGY_MAX_KEV),
                beam_power_w: pick(1, 1.0, 1.6e7),
                electron_density_per_m3,
                electron_temperature_kev: pick(3, ELECTRON_TEMPERATURE_MIN_KEV, ELECTRON_TEMPERATURE_MAX_KEV),
                boron_density_per_m3: pick(4, 0.0, 1.0) * pick(5, 0.05, 0.19) * electron_density_per_m3,
            };
            match solve(&question) {
                Ok(answer) => {
                    answered += 1;
                    assert!(answer.proton_ledger_residual <= PROTON_LEDGER_TOLERANCE, "{corner}");
                    assert!(answer.energy_ledger_residual <= ENERGY_LEDGER_TOLERANCE, "{corner}");
                    assert!(answer.fast_proton_fraction_max <= 1.0, "{corner}");
                }
                Err(LhdReject::FastProtonChargeExceedsElectrons { .. } | LhdReject::BoronChargeExceedsElectrons { .. }) => {}
                Err(other) => panic!("{corner}: {question:?}: {other:?}"),
            }
        }
        assert_eq!(answered, 42);
    }

    /// Quasi-neutrality: at 1e18 electrons per cubic metre, boron at 3e17
    /// would carry more charge than the electrons at the edge, named from the
    /// question alone and by the physics.
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
        assert_eq!(admit(&question), Err(LhdReject::BoronChargeExceedsElectrons { shell: 7 }));
    }

    /// Attack: a plasma that heats the beam proton. In a shell at 30 keV,
    /// far above the declared top, a 20 keV proton is slower than the thermal
    /// protons and gains energy from them; past `admit` the solve names it.
    /// At the declared top the same proton slows all the way down.
    #[test]
    fn a_plasma_that_heats_the_proton_is_named() {
        assert_eq!(
            settle_shell(3, 20.0, 1.0e19, 30.0, 0.0, MAX_REFINEMENTS, Fault::None).err(),
            Some(LhdReject::SlowingRateNotPositive { shell: 3 })
        );
        assert!(settle_shell(3, 20.0, 1.0e19, ELECTRON_TEMPERATURE_MAX_KEV, 0.0, MAX_REFINEMENTS, Fault::None).is_ok());
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
        let (good, _) = settle_shell(0, 180.0, 2.2e19, 5.4, 6.3e16, MAX_REFINEMENTS, Fault::None).expect("settles");
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
    /// A4b, A5, A5b and A6 of `runs_2026-10-09`: the seven of Figure 10 (A3),
    /// given the deposited power, are answered; the fourteen of Figure 6 (A4
    /// to A6, the 45 keV beam among them), given the injected power or an
    /// estimate of the deposited one, leave more beam protons than electrons
    /// at the axis and are refused by name. The runs against the published
    /// calculation, A1 and A1x, are answered.
    #[test]
    fn every_compared_request_is_answered_or_named_by_its_fast_protons() {
        const ANSWERED_RUNS: [[f64; 5]; 7] = [
            [159.56, 1311100.0, 4.268e18, 3.7872, 1.4298e17],
            [160.25, 1233300.0, 5.154e18, 3.7589, 2.04258e17],
            [148.46, 1366700.0, 4.725e18, 3.5319, 1.90212e17],
            [127.45, 1405600.0, 6.872e18, 3.3333, 2.84682e17],
            [129.35, 938900.0, 4.617e18, 2.9929, 1.64682e17],
            [128.59, 1322200.0, 6.067e18, 3.0213, 2.2596e17],
            // A3 with the top panel's voltages, the protocol's reading.
            [128.8889, 938900.0, 4.617e18, 2.9929, 1.64682e17],
        ];
        const NAMED_RUNS: [[f64; 5]; 14] = [
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
        ];
        let question = |[beam_energy_kev, beam_power_w, electron_density_per_m3, electron_temperature_kev, boron_density_per_m3]: [f64; 5]| LhdQuestion {
            beam_energy_kev,
            beam_power_w,
            electron_density_per_m3,
            electron_temperature_kev,
            boron_density_per_m3,
        };
        let mut most: f64 = 0.0;
        for run in ANSWERED_RUNS {
            let answer = solve(&question(run)).unwrap_or_else(|reject| panic!("{run:?}: {reject}"));
            most = most.max(answer.fast_proton_fraction_max);
        }
        assert!(most > 0.77 && most < 0.78, "{most}");
        let (mut least, mut largest) = (f64::INFINITY, 0.0_f64);
        for run in NAMED_RUNS {
            match solve(&question(run)) {
                Err(LhdReject::FastProtonChargeExceedsElectrons { shell: 0, fast_proton_fraction, .. }) => {
                    least = least.min(fast_proton_fraction);
                    largest = largest.max(fast_proton_fraction);
                }
                other => panic!("{run:?}: {:?}", other.map(|answer| answer.fast_proton_fraction_max)),
            }
        }
        println!("Figure 6: fast protons {least} to {largest} of the electrons at the axis");
        assert!(least > 1.24 && largest < 7.25, "{least} {largest}");
        for boron in [OGAWA_2022_BORON_PER_M3, 10.0 * OGAWA_2022_BORON_PER_M3] {
            for beam in 0..3 {
                assert!(solve(&ogawa_2022(beam, boron)).is_ok());
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
        let (settled, taken) = settle_shell(0, 160.0, 5.0e18, 2.0, 1.0e17, MAX_REFINEMENTS, Fault::None).expect("settles");
        let finer = slow_and_fuse(0, 160.0, 5.0e18, 2.0, 1.0e17, FIRST_PANEL_KEV / 2.0_f64.powi(taken as i32), Fault::None)
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
    /// any refinement. Zero, the floor itself and two measured points of
    /// Figure 10 are answered as engine 0.8.0 answered them, with the bits
    /// genesis_kernel::math gives them since genesis_solvers 0.12.0: the
    /// reopened range moved no answer it already gave.
    #[test]
    fn boron_below_one_nucleus_is_named_and_zero_is_answered_as_before() {
        let ask = |boron_density_per_m3: f64| LhdQuestion { boron_density_per_m3, ..ANSWERED };
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
        for beam_energy_kev in [BEAM_ENERGY_MIN_KEV.next_up(), 45.0, BEAM_ENERGY_MAX_KEV] {
            assert_eq!(
                solve(&LhdQuestion { beam_energy_kev, boron_density_per_m3: 1.0e-297, ..ANSWERED }),
                Err(LhdReject::BoronBelowOneNucleus { boron_density_per_m3: 1.0e-297 })
            );
        }
        // A density below zero keeps its name.
        assert_eq!(solve(&ask(-1.0e-297)), Err(LhdReject::BoronDensityOutOfRange { boron_density_per_m3: -1.0e-297 }));

        // No boron: one refinement, no alphas, the bits of engine 0.8.0.
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
                    0x3fe55cb3cac9fde6,
                    0x3fe341822c77b5f5,
                    0x3fd57a28c4c749fe,
                    0,
                    0x3cc1a6064a0f4625,
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
                    0x3ecdd684fd8fda0d,
                    0x3c3e6a99a80d6f7b,
                    0x3af85379879f485c,
                    0x3a9c9e7eb008fb16,
                    0x3e94842b88654d55,
                    0x3fd8000000000000,
                    0x3fe55cb3cac9fde6,
                    0x3fe341822c77b5f3,
                    0x3fd57a28c4c749fe,
                    0x3aa2d08ec463f03c,
                    0x3cbf4939b1d5422a,
                ],
                0x2a430c6b1e61f620,
                3
            )
        );
        assert_eq!(one.fused_fraction, 2.311834151228454e-26);
        // Two measured points of Figure 10.
        let answered = solve(&ANSWERED).expect("solves");
        assert_eq!(
            bits(&answered),
            (
                [
                    0x42a2d23760713f68,
                    0x40132f9f5c3ed0e2,
                    0x3eceb0517d8b6345,
                    0x3e720d65d5c8c96f,
                    0x4269ed7a6874b506,
                    0x3fd8000000000000,
                    0x3fe6037a511cbb01,
                    0x3fe3a5e2d9357b68,
                    0x3fd4b1672d326693,
                    0x3cc7ba23e8000000,
                    0x3dacbb307407f538,
                ],
                0x9b62415714ef86dd,
                3
            )
        );
        let most_boron = solve(&MOST_BORON).expect("solves");
        assert_eq!(
            bits(&most_boron),
            (
                [
                    0x4272288cd00d67aa,
                    0x3fe282aac58ab9f7,
                    0x3e9b9e149b053d41,
                    0x3e39f3ebb98d1046,
                    0x4238da73771fcabb,
                    0x3fd8000000000000,
                    0x3fd572b882f88100,
                    0x3fe2e603e4cc9c77,
                    0x3fd52e721dae9fdf,
                    0x3cc6c40fe3000000,
                    0x3d75ae899759af74,
                ],
                0xb95e3f1cc9068776,
                2
            )
        );
        // The check of the question adds nothing to an answer: each is the
        // physics of the question alone, which this floor does not enter.
        for question in [ask(0.0), ask(floor), ANSWERED, MOST_BORON] {
            let direct = answer(&question, MAX_REFINEMENTS, Fault::None).expect("solves");
            assert_eq!(bits(&solve(&question).expect("solves")), bits(&direct));
        }
    }

    /// No admitted boron density above zero takes more than three
    /// refinements, and none ends on `NotSettled`: a ladder from the floor to
    /// the most boron for its electrons compared, in the plasma of a measured
    /// point and in the plasma of the largest density compared.
    #[test]
    fn every_admitted_boron_density_settles_within_three_refinements() {
        assert_eq!(solve(&ANSWERED).expect("solves").refinements, 3);
        let floor = BORON_DENSITY_ONE_NUCLEUS_PER_M3;
        let largest_share = MOST_BORON.boron_density_per_m3 / MOST_BORON.electron_density_per_m3;
        for base in [ANSWERED, MOST_BORON] {
            let most = largest_share * base.electron_density_per_m3;
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
    }
}
