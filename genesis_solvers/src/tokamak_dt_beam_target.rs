//! The deuterium-tritium fusion rate of one time slice of a tokamak pulse,
//! from its measured profiles and its neutral beams.
//!
//! The specimens are the deuterium-tritium pulses of two tokamaks, JET
//! (Culham; 1997, the first deuterium-tritium experiment) and TFTR
//! (Princeton; 1993 to 1997), as the International Multi-Tokamak Confinement
//! Profile Database, 2008 public release, holds them
//! (`https://tokamak-profiledb.ccfe.ac.uk/PR08/`): for each pulse, radial
//! profiles of temperature and density at each time, the beam heating
//! profile, and the fusion power. The solver stores no value of any pulse.
//!
//! The caller gives one time slice as a set of nested radial shells: each
//! shell's volume, electron temperature and density, ion temperature,
//! thermal deuterium, tritium and impurity density, and the beam power
//! deposited in it; and the beam as components, each with its mass number
//! (2 or 3), its energy and its share of the deposited power. The solver
//! returns the reactions per second of three kinds, or a named limit.
//!
//! ```text
//! thermal       n_D n_T ⟨σv⟩_DT(T_i) V_shell                        Bosch and Hale's reactivity fit (bosch_hale)
//! beam-target   a beam ion is born at E0 in a shell and slows there:
//!                 dE/dt  = − Σ_s L_s(E)             s = electrons, deuterium, tritium, impurity   (pb11_reaction)
//!                 λ_r(E) = n_target ⟨σv⟩_r(E, T_i) / Σ_s L_s(E)     chance of reaction r per keV lost
//!                 S(E)   = exp(−∫_E^E0 Σ_r λ_r dE′)                 share still slowing at energy E
//!               r: a deuteron on thermal tritium and on thermal deuterium (both branches);
//!                  a triton on thermal deuterium
//!               followed down to 1.5 T_i of the shell; a component born at or below that joins
//!               the thermal plasma at once
//! beam-beam     fast deuterons on fast tritons of the same shell, each with its settled
//!               slowing-down spectrum f(E) = (born per second) S(E) / Σ_s L_s(E) and taken as
//!               isotropic in direction:
//!                 rate = (1/V) ∬ f_D f_T K dE_D dE_T,   K = [G(v_D + v_T) − G(|v_D − v_T|)] / (2 v_D v_T)
//!                 G(u) = ∫₀^u σ_DT(½ μ u′²) u′² du′
//! ```
//!
//! `⟨σv⟩_r(E, T_i)` is the cross-section averaged over the thermal motion of
//! the target, [`crate::pb11_reaction::beam_target_rate_for_m3_per_s`], in
//! panels [`THERMAL_AVERAGE_PANEL_KEV`] wide. In each shell the energy
//! integral of each component is taken in panels of the eight-point Gauss
//! rule, and the panels are halved until every fused share and the
//! beam-beam rate of the shell settle.
//!
//! What must hold in every shell for every component at every refinement.
//! Particles: the share that slowed to the stopping energy and the share
//! that fused with the thermal plasma add to one. Energy: the birth energy
//! equals the energy given to electrons and to ions, the energy the fused
//! ions carried into fusion, and what the slowed ions still hold. Each
//! channel's energy is the integral of that channel's own rate over the
//! total, so a channel counted in one place and not the other shows. Both
//! ledgers are accounts of the arithmetic: they hold for any slowing-down
//! rate and any cross-section, and say nothing of whether those are right.
//! Beam-beam reactions are counted outside the ledgers: the share of beam
//! ions they take is returned, and above [`BEAM_BEAM_SHARE_MAX`] the answer
//! is refused.
//!
//! Nothing is fitted. Source of each number: the cross-sections and
//! reactivities, `bosch_hale`; the slowing-down rates, the Coulomb logarithms
//! and the relativistic factor, `pb11_reaction`. The ranges of the physics
//! are the span of the 166 slices compared and answered on engine 0.6.0
//! (90 saved TFTR slices, the 23 held-back TFTR slices answered and 53 of
//! JET), rounded outward in the third figure: electron and ion temperature,
//! their ratio, electron density, the tritium share of the fuel, the
//! impurity, the beam energy, the fast-ion share and the charge balance with
//! the fast ions counted. The shell count, the shell volume, the component
//! count and the whole beam power are capacities: each shell is solved
//! alone.
//!
//! Left out: any motion of a beam ion across shells while it slows, its
//! loss by charge exchange or on an unconfined orbit (the caller gives the
//! power deposited, after those), the rotation of the plasma,
//! ion-cyclotron acceleration of beam ions, the direction of the beams
//! (beam-beam is taken as isotropic), reactions of fast deuterons with each
//! other and of tritons with tritons, fast ions as a background that other
//! beam ions slow on, fusion alphas, and thermal hydrogen, which the
//! question does not take. Greenwald's density limit and
//! Troyon's pressure limit are properties of a machine's operating point:
//! the question holds no current, field or minor radius, and neither is
//! asked.
//!
//! **Limits written after the held-back comparison**, on data that
//! was by then spent (`evidence/research/tokamak_dt_beam_target/protocol_amendment_2.md`;
//! second scoring `evidence/checks/second_party/V-42/`). None is a result
//! of a blind test.
//!
//! *Steady state.* The model is the settled slowing-down spectrum of a
//! constant source. The caller states how long the beam power has been
//! steady, `time_since_beam_change_s`, and the solve finds the fast ions'
//! energy time, their kinetic energy over the deposited beam power. A
//! slice less than [`SETTLED_AFTER_ENERGY_TIMES`] energy times after a
//! change is refused, [`SliceReject::BeamChangeTooRecent`]. The solver
//! cannot check the caller's statement.
//!
//! *Fast ions as a share of the plasma.* A beam ion is slowed on electrons
//! and on thermal ions; the other fast ions are not there for it. Where the
//! fast ions the solve finds are more than [`FAST_ION_SHARE_MAX`] of the
//! electrons of a shell the answer is refused,
//! [`SliceReject::FastIonShareTooLarge`].
//!
//! *Quasi-neutrality.* At the door the thermal ions may carry more charge
//! than the electrons of a shell by [`QUASI_NEUTRALITY_ALLOWANCE`], the
//! closure error of analysed profiles, and no more,
//! [`SliceReject::ThermalChargeExceedsElectrons`]. After the solve (engine
//! 0.7.0) the thermal ions and the fast ions this solve finds are held from
//! both sides, at the extremes the slices compared and answered reach:
//! within [`QUASI_NEUTRALITY_FAST_ION_ALLOWANCE`] over the electrons,
//! [`SliceReject::IonChargeExceedsElectrons`], and within
//! [`QUASI_NEUTRALITY_DEFICIT_ALLOWANCE`] under them,
//! [`SliceReject::IonChargeBelowElectrons`]. The band is wide because two
//! things the model does not hold move the balance on TFTR: the solver's
//! fast ions are 1.1 to 3.1 times the database's in the shell of largest
//! excess (deposition is local, with no orbit spread and no drag on other
//! fast ions), and the question carries no thermal hydrogen, which the
//! database's analysis holds at 4 to 22 percent of the electrons at the
//! edge. Inside the band the solve uses the question's own densities:
//! nothing is trimmed and nothing is filled in. The door's end carries
//! [`QUASI_NEUTRALITY_END_ALLOWANCE_ULP`] and the lower end
//! [`QUASI_NEUTRALITY_DEFICIT_END_ALLOWANCE_ULP`] for the rounding of the
//! sum alone, so that a shell without fast ions at exactly either end is
//! answered. The upper end after the solve needs none: the thermal ions
//! alone stop at the door's end, far below it.
//!
//! *Fuel.* The tritium share of the fuel, n_T / (n_D + n_T), is held in
//! every shell to [`TRITIUM_SHARE_MIN`]..=[`TRITIUM_SHARE_MAX`], the span
//! compared, [`SliceReject::TritiumShareOutOfRange`]: no slice compared is
//! free of tritium or of deuterium.
//!
//! How the solver compared with the fusion power of the database's pulses is
//! in the research record, `evidence/research/tokamak_dt_beam_target/`.

use std::fmt;

use crate::bosch_hale::{self, DEUTERON_MASS_KG, Reaction, TRITON_MASS_KG};
use crate::pb11_reaction::{self as reaction, ATOMIC_MASS_KG, Background, JOULE_PER_KEV, Species};

/// Most shells in one question. A capacity of the solver and no range of the
/// physics: each shell is solved alone, and a shell split in two gives the
/// same rates and the same fast-ion share.
pub const SHELLS_MAX: usize = 128;
/// Most beam components in one question. A capacity, as the shell count is:
/// a component split in two gives the same answer.
pub const BEAM_COMPONENTS_MAX: usize = 12;
/// Volume of one shell, cubic metres. A capacity bound, as the shell count
/// is: the solve is local to each shell, and what the physics sees is the
/// densities and the deposited power over the volume, which the fast-ion
/// share and the charge balance hold. JET's plasma is about 85 cubic metres.
pub const SHELL_VOLUME_MAX_M3: f64 = 200.0;
/// The smallest shell volume answered, cubic metres. Below it the deposited
/// power over the volume may not be finite; the smallest shell compared is
/// 0.0070 cubic metres.
pub const SHELL_VOLUME_MIN_M3: f64 = 1.0e-6;
/// Electron temperature of a shell, keV: the span of the slices compared and
/// answered on engine 0.6.0, 0.1991 to 13.095 keV, rounded outward in the
/// third figure. The Formulary's electron logarithm holds from 0.05 keV,
/// below the floor.
pub const ELECTRON_TEMPERATURE_MIN_KEV: f64 = 0.199;
pub const ELECTRON_TEMPERATURE_MAX_KEV: f64 = 13.1;
/// Ion temperature of a shell, keV: the span of the slices compared and
/// answered on engine 0.6.0, 0.2241 to 41.76 keV, rounded outward in the
/// third figure. The reactivity fit holds from 0.2 keV, below the floor.
pub const ION_TEMPERATURE_MIN_KEV: f64 = 0.224;
pub const ION_TEMPERATURE_MAX_KEV: f64 = 41.8;
/// Electron over ion temperature of a shell: the span of the shells of the
/// slices compared and answered on engine 0.6.0, 0.07308 (the edge of TFTR
/// 79099 at 4.22 s) to 1.9945 (JET 42794 at 21.10 s), rounded outward in the
/// third figure. Outside it the electron drag and the ion drag on a beam ion
/// stand in a ratio no comparison has met.
pub const TEMPERATURE_RATIO_MIN: f64 = 0.0730;
pub const TEMPERATURE_RATIO_MAX: f64 = 2.0;
/// Electron density of a shell, per cubic metre: the span of the slices
/// compared and answered on engine 0.6.0, 4.195e18 to 1.2015e20, rounded
/// outward in the third figure.
pub const ELECTRON_DENSITY_MIN_PER_M3: f64 = 4.19e18;
pub const ELECTRON_DENSITY_MAX_PER_M3: f64 = 1.21e20;
/// Tritium over deuterium plus tritium of a shell: the span of the shells of
/// the slices compared and answered on engine 0.6.0, 0.004675 (TFTR 105324
/// at 3.41 s) to 0.9974 (JET 43002 at 18.05 s), rounded outward in the third
/// figure. No slice compared is free of tritium or of deuterium.
pub const TRITIUM_SHARE_MIN: f64 = 0.00467;
pub const TRITIUM_SHARE_MAX: f64 = 0.998;
/// Charge number of the impurity: the span of the slices compared. No other
/// impurity is in the data on disk.
pub const IMPURITY_CHARGE_MIN: f64 = 5.0;
pub const IMPURITY_CHARGE_MAX: f64 = 6.5;
/// Mass number of the impurity: the span of the slices compared.
pub const IMPURITY_MASS_MIN_U: f64 = 10.0;
pub const IMPURITY_MASS_MAX_U: f64 = 13.0;
/// Energy of a beam component, keV: the span of the slices compared. No
/// saved question holds a component outside it.
pub const BEAM_ENERGY_MIN_KEV: f64 = 26.0;
pub const BEAM_ENERGY_MAX_KEV: f64 = 152.0;
/// Beam power deposited in the whole plasma, watts. A property of the two
/// machines: the most any pulse of the profile database injects is 39.6 MW
/// (TFTR 80539), and in steady state the power deposited is below the power
/// injected (0.87 of it in the median of the database's beam heating
/// profiles while the beams are on). The database's heating profile sums to
/// more than this at eleven profile times of one pulse, TFTR 80539 from
/// 3.81 to 3.91 s, 46 to 57 MW against 39.5 MW injected: the fast ions are
/// giving up stored energy in a collapse, which is no steady state.
pub const BEAM_POWER_MAX_W: f64 = 4.5e7;
/// The shares of the beam power over the components must add to one within
/// this, and so must nothing else: a caller's decimal shares need not add
/// to one in the last place.
pub const SHARE_SUM_ALLOWANCE: f64 = 1.0e-9;
/// The thermal ions of a shell may carry more charge than its electrons by
/// this fraction of the electron density and no more, held at the door. It
/// is the closure error of analysed profiles: over the 4930 shells of the
/// 175 saved slices, the profile database's own deuterium, tritium,
/// hydrogen, impurity and fast ions together stand between 7.53 percent
/// under and 2.40 percent over the electrons (median 0.03 percent under);
/// the largest excess, of JET pulse 43002, rounded up to the half percent.
/// Until 2026-10-09 it was 1e-9, which refused eight slices of that pulse
/// whose thermal ions alone exceed the electrons by 0.05 to 1.37 percent.
/// Set on spent data.
pub const QUASI_NEUTRALITY_ALLOWANCE: f64 = 0.025;
/// The thermal ions and the fast ions this solve finds may carry more charge
/// than the electrons of a shell by this fraction of the electron density
/// and no more, held after the solve (engine 0.7.0). It is the largest
/// excess on a slice compared and answered on engine 0.6.0, 0.1186 (TFTR
/// 83526 at 4.73 s), rounded up to the half percent. It is no closure error:
/// the database's own balance stays within 2.40 percent over. What fills the
/// band is the solver's fast-ion number, 1.1 to 3.1 times the database's in
/// the shell of largest excess on the TFTR slices, since a beam ion is
/// deposited where it is born, with no orbit spread, and is not slowed by
/// the other fast ions.
pub const QUASI_NEUTRALITY_FAST_ION_ALLOWANCE: f64 = 0.12;
/// The thermal ions and the fast ions this solve finds may carry less charge
/// than the electrons of a shell by this fraction of the electron density
/// and no more, held after the solve (engine 0.7.0). It is the largest
/// shortfall on a slice compared and answered on engine 0.6.0, 0.2109 (TFTR
/// 74898 at 3.12 s), rounded up to the half percent. What fills the band is
/// the thermal hydrogen the question cannot carry: the database's analysis
/// holds it at 4 to 22 percent of the electrons in the outer shells of the
/// TFTR slices, and with it counted the lowest balance of every held-back
/// slice lies between 9.0 percent under and 0.9 percent over.
pub const QUASI_NEUTRALITY_DEFICIT_ALLOWANCE: f64 = 0.215;
/// The comparison of a shell's thermal ion charge over its electrons with
/// 1 + [`QUASI_NEUTRALITY_ALLOWANCE`] accepts this many units in the last
/// place beyond it, for rounding alone. The ratio is formed here as
/// `(n_D + n_T + Z n_Z) / n_e`: one product, two sums and one quotient,
/// four roundings, and the four densities a caller writes as decimals are
/// each the nearest 64-bit number, which moves the sum by at most one more
/// rounding and the quotient by one. Six factors between 1 - u and 1 + u,
/// u = 2^-53, are 6u in the ratio; a unit in the last place of 1.025 is
/// 2u / 1.025 of it, so 3.1 units; the 64-bit 1.025 is itself within half a
/// unit of 1.025. The ratio is a whole number of units from the end: at
/// most 3. The test `an_excess_of_exactly_the_allowance_is_answered`
/// prints the largest distance it meets.
pub const QUASI_NEUTRALITY_END_ALLOWANCE_ULP: u32 = 4;
/// The same for 1 - [`QUASI_NEUTRALITY_DEFICIT_ALLOWANCE`]: a unit in the
/// last place of 0.785 is u, so 6u of 0.785 is 4.71 units, and with the half
/// unit of the 64-bit 0.785 the ratio is at most 5 whole units from the end.
/// With fast ions in the shell their density is added to the sum before the
/// quotient; it is this solve's own number, and the end is exact only where
/// a shell holds none. The test
/// `a_shortfall_of_exactly_the_allowance_is_answered` prints the largest
/// distance it meets.
pub const QUASI_NEUTRALITY_DEFICIT_END_ALLOWANCE_ULP: u32 = 6;
/// The largest thermal ion charge over electron density answered at the door.
pub const THERMAL_CHARGE_OVER_ELECTRONS_HIGHEST_ANSWERED: f64 =
    f64::from_bits((1.0 + QUASI_NEUTRALITY_ALLOWANCE).to_bits() + QUASI_NEUTRALITY_END_ALLOWANCE_ULP as u64);
/// The largest charge of the thermal and fast ions over electron density
/// answered after the solve.
pub const ION_CHARGE_OVER_ELECTRONS_HIGHEST_ANSWERED: f64 = 1.0 + QUASI_NEUTRALITY_FAST_ION_ALLOWANCE;
/// The smallest charge of the thermal and fast ions over electron density
/// answered after the solve.
pub const ION_CHARGE_OVER_ELECTRONS_LOWEST_ANSWERED: f64 = f64::from_bits(
    (1.0 - QUASI_NEUTRALITY_DEFICIT_ALLOWANCE).to_bits() - QUASI_NEUTRALITY_DEFICIT_END_ALLOWANCE_ULP as u64,
);
/// Seconds since the beam power last changed, the caller's statement. The
/// ceiling is a bound on the input and no limit of the physics: no pulse of
/// either machine held its beams a tenth as long, and every value above a
/// few seconds says the same thing, steady.
pub const TIME_SINCE_BEAM_CHANGE_MAX_S: f64 = 100.0;
/// What counts as a change of the beam power, for the caller who states
/// `time_since_beam_change_s`: the injected power differing from its
/// present value by more than this fraction of it. The solver cannot see
/// the beam's history and does not use this number; it is published so that
/// two callers mean the same thing. Origin: the protocol's own measure of
/// "beams on", 0.8 of the maximum, fixed before the held-back data; and, on
/// spent data, the time scans of the second scoring, where a change of 10
/// to 20 percent inside five energy times left 17 of 17 TFTR rows inside
/// 0.75 to 1.25 of the reference and a change of 20 to 50 percent 31 of 39.
pub const BEAM_CHANGE_FRACTION: f64 = 0.2;
/// A slice is answered only if the beam power has been steady for at least
/// this many fast-ion energy times (fast-ion kinetic energy over deposited
/// beam power, both of this solve). Origin, on spent data: of the 34 scored
/// TFTR slices the 27 at five energy times or later after the first beam
/// power are all inside 0.75 to 1.25 of the reference (pulse 73268 against
/// five times its database value) and two of the seven earlier ones are
/// outside, at 3.8 and 2.3 (second scoring V-42, section 6d). Within five
/// energy times of a beam change the solver was 0.6 to 1.55 of the
/// reference.
pub const SETTLED_AFTER_ENERGY_TIMES: f64 = 5.0;
/// The fast ions the solve finds may be this share of the electrons of a
/// shell and no more: the largest share on a slice compared and answered on
/// engine 0.6.0, 0.4383 (TFTR 80539 at 3.74 s), rounded up in the third
/// figure. Until engine 0.7.0 it was 0.5, set on spent data: of the 34 scored
/// TFTR slices the 31 at or below one half were inside 0.75 to 1.25 of the
/// reference (73268 against five times its database value), and of the
/// three above, 0.59, 0.63 and 0.91, two were outside (second scoring V-42,
/// section 6e). A share between 0.439 and 0.5 is answered on no slice
/// compared.
pub const FAST_ION_SHARE_MAX: f64 = 0.439;

/// A beam ion is followed down to this many ion temperatures.
pub const STOPPING_ENERGY_OVER_ION_TEMPERATURE: f64 = 1.5;
/// Width of an energy panel at the first refinement, keV.
pub const FIRST_PANEL_KEV: f64 = 16.0;
/// Width of a panel of the average over the target's thermal motion, keV. It is not refined.
pub const THERMAL_AVERAGE_PANEL_KEV: f64 = 16.0;
/// A shell has settled when halving the panels moves each fused share and
/// the beam-beam rate by less than this fraction.
pub const SETTLE_TOLERANCE: f64 = 1.0e-5;
/// The public solver stops a shell after this many refinements.
pub const MAX_REFINEMENTS: u32 = 6;
/// Slowed and fused shares must add to one within this.
pub const PARTICLE_LEDGER_TOLERANCE: f64 = 1.0e-12;
/// The energy ledger must close within this fraction of the birth energy.
pub const ENERGY_LEDGER_TOLERANCE: f64 = 2.0e-5;
/// The power ledger of the whole plasma must close within this fraction of the beam power.
pub const POWER_LEDGER_TOLERANCE: f64 = 2.0e-5;
/// Beam-beam reactions may take this share of the beam ions and no more.
pub const BEAM_BEAM_SHARE_MAX: f64 = 2.0e-3;
/// Cells of the table of `G(u)`.
pub const BEAM_BEAM_CELLS: usize = 4096;
/// Top of the table of `G(u)`, m/s: above the summed speeds of a 160 keV deuteron and a 160 keV triton.
pub const BEAM_BEAM_SPEED_MAX_M_PER_S: f64 = 8.0e6;

/// The published record this solver is checked against.
pub const REFERENCE_NAME: &str = "The International Multi-Tokamak Confinement Profile Database, 2008 public release (Nucl. Fusion 40 (2000) 1955): JET pulse 42976 of 1997, analysed with TRANSP by R. V. Budny, at 13.35 s: D-T fusion power 13.2 MW";
pub const REFERENCE_URL: &str = "https://tokamak-profiledb.ccfe.ac.uk/PR08/itpa_pr08/jet/42976/";
/// Date the reference was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";

/// One time slice.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceQuestion {
    /// Volume of each shell from the axis outward, cubic metres.
    pub shell_volume_m3: Vec<f64>,
    /// Electron temperature of each shell, keV.
    pub electron_temperature_kev: Vec<f64>,
    /// Ion temperature of each shell, keV.
    pub ion_temperature_kev: Vec<f64>,
    /// Electron density of each shell, per cubic metre.
    pub electron_density_per_m3: Vec<f64>,
    /// Thermal deuterium density of each shell, per cubic metre.
    pub deuterium_density_per_m3: Vec<f64>,
    /// Thermal tritium density of each shell, per cubic metre.
    pub tritium_density_per_m3: Vec<f64>,
    /// Impurity density of each shell, per cubic metre.
    pub impurity_density_per_m3: Vec<f64>,
    /// Charge number of the impurity.
    pub impurity_charge: f64,
    /// Mass number of the impurity.
    pub impurity_mass_u: f64,
    /// Beam power deposited in each shell, watts, all components together.
    pub beam_power_w: Vec<f64>,
    /// Mass number of each beam component: 2, deuterium, or 3, tritium.
    pub beam_mass_number: Vec<f64>,
    /// Energy of each beam component, keV.
    pub beam_energy_kev: Vec<f64>,
    /// Share of the deposited beam power in each component. They add to one.
    pub beam_power_share: Vec<f64>,
    /// Seconds since the injected beam power last differed from its present
    /// value by more than [`BEAM_CHANGE_FRACTION`] of it. The caller's
    /// statement: the solver cannot check it.
    pub time_since_beam_change_s: f64,
}

/// Why no answer was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum SliceReject {
    /// There is no shell, or more than [`SHELLS_MAX`].
    ShellCountOutOfRange { shells: usize },
    /// A profile does not hold one value for each shell.
    ProfileLengthsDiffer { name: &'static str, length: usize, shells: usize },
    /// There is no beam component, or more than [`BEAM_COMPONENTS_MAX`], or the three beam lists differ in length.
    BeamComponentCountOutOfRange { components: usize },
    /// A shell volume is outside [`SHELL_VOLUME_MIN_M3`]..=[`SHELL_VOLUME_MAX_M3`].
    ShellVolumeOutOfRange { shell: usize, value: f64 },
    /// An electron temperature is outside [`ELECTRON_TEMPERATURE_MIN_KEV`]..=[`ELECTRON_TEMPERATURE_MAX_KEV`], the span of the slices compared.
    ElectronTemperatureOutOfRange { shell: usize, value: f64 },
    /// An ion temperature is outside [`ION_TEMPERATURE_MIN_KEV`]..=[`ION_TEMPERATURE_MAX_KEV`], the span of the slices compared.
    IonTemperatureOutOfRange { shell: usize, value: f64 },
    /// The electron temperature of a shell over its ion temperature is outside [`TEMPERATURE_RATIO_MIN`]..=[`TEMPERATURE_RATIO_MAX`], the span of the slices compared.
    TemperatureRatioOutOfRange { shell: usize, value: f64 },
    /// An electron density is outside [`ELECTRON_DENSITY_MIN_PER_M3`]..=[`ELECTRON_DENSITY_MAX_PER_M3`], the span of the slices compared.
    ElectronDensityOutOfRange { shell: usize, value: f64 },
    /// A deuterium, tritium or impurity density is below zero.
    IonDensityNegative { shell: usize, value: f64 },
    /// The tritium share of the fuel of a shell, n_T / (n_D + n_T), is outside [`TRITIUM_SHARE_MIN`]..=[`TRITIUM_SHARE_MAX`], the span of the slices compared: no slice compared is free of tritium or of deuterium.
    TritiumShareOutOfRange { shell: usize, value: f64 },
    /// `impurity_charge` is outside [`IMPURITY_CHARGE_MIN`]..=[`IMPURITY_CHARGE_MAX`], the span of the slices compared.
    ImpurityChargeOutOfRange { value: f64 },
    /// `impurity_mass_u` is outside [`IMPURITY_MASS_MIN_U`]..=[`IMPURITY_MASS_MAX_U`], the span of the slices compared.
    ImpurityMassOutOfRange { value: f64 },
    /// Quasi-neutrality, at the door: in one shell the thermal ions alone carry more charge than the electrons there by more than [`QUASI_NEUTRALITY_ALLOWANCE`] of the electron density, with [`QUASI_NEUTRALITY_END_ALLOWANCE_ULP`] units in the last place for rounding. Fast ions only add to that charge.
    ThermalChargeExceedsElectrons { shell: usize, excess: f64 },
    /// A shell's deposited beam power is below zero, or the whole is not above zero or is above [`BEAM_POWER_MAX_W`].
    BeamPowerOutOfRange { value: f64 },
    /// A beam mass number is neither 2 nor 3.
    BeamSpeciesUnknown { component: usize, value: f64 },
    /// A beam energy is outside [`BEAM_ENERGY_MIN_KEV`]..=[`BEAM_ENERGY_MAX_KEV`], the span of the slices compared.
    BeamEnergyOutOfRange { component: usize, value: f64 },
    /// A beam power share is not above zero, or the shares do not add to one within [`SHARE_SUM_ALLOWANCE`].
    BeamPowerShareOutOfRange { value: f64 },
    /// `time_since_beam_change_s` is outside 0..=[`TIME_SINCE_BEAM_CHANGE_MAX_S`].
    TimeSinceBeamChangeOutOfRange { value: f64 },
    /// An intermediate value was NaN or infinite, or an input was when the solver is called directly: through the gateway no input can be, since a number that is not finite is not JSON.
    NonFiniteValue,
    /// At some energy between birth and stopping a beam ion gains energy from the plasma of a shell: the electrons there heat it more than the thermal ions slow it.
    SlowingRateNotPositive { shell: usize },
    /// The refinement cap was reached with a fused share or the beam-beam rate of a shell still moving.
    NotSettled { refinements: u32 },
    /// For one component in one shell the slowed and the fused shares left [`PARTICLE_LEDGER_TOLERANCE`] of one.
    ParticleLedgerBroken { residual: f64 },
    /// For one component in one shell the energy ledger left [`ENERGY_LEDGER_TOLERANCE`] of the birth energy.
    EnergyLedgerBroken { residual: f64 },
    /// The beam power of the whole plasma and where it went differ by more than [`POWER_LEDGER_TOLERANCE`] of it.
    PowerLedgerBroken { residual: f64 },
    /// Beam-beam reactions take more than [`BEAM_BEAM_SHARE_MAX`] of the beam ions, and the ledgers do not count them.
    BeamBeamShareTooLarge { share: f64 },
    /// In one shell the fast ions this solve finds are more than [`FAST_ION_SHARE_MAX`] of the electrons, the largest share on a slice compared: a beam ion is slowed on thermal ions only, and other fast ions are not there for it.
    FastIonShareTooLarge { shell: usize, share: f64 },
    /// The beam power changed less than [`SETTLED_AFTER_ENERGY_TIMES`] fast-ion energy times before this slice, by the caller's `time_since_beam_change_s` and the energy time this solve finds: the fast ions have not settled, and the model is steady state.
    BeamChangeTooRecent { time_since_beam_change_s: f64, fast_ion_energy_time_s: f64 },
    /// Quasi-neutrality, after the solve: in one shell the thermal ions and the fast ions this solve finds carry more charge than the electrons there by more than [`QUASI_NEUTRALITY_FAST_ION_ALLOWANCE`] of the electron density, the largest excess on a slice compared: the fast ions the model finds cannot be there beside the measured electrons and the given thermal ions.
    IonChargeExceedsElectrons { shell: usize, excess: f64 },
    /// Quasi-neutrality, after the solve: in one shell the thermal ions and the fast ions this solve finds carry less charge than the electrons there by more than [`QUASI_NEUTRALITY_DEFICIT_ALLOWANCE`] of the electron density, the largest shortfall on a slice compared: the question leaves out ions, such as hydrogen, that would slow and meet the beam.
    IonChargeBelowElectrons { shell: usize, deficit: f64 },
}

impl fmt::Display for SliceReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShellCountOutOfRange { shells } => write!(f, "{shells} shells is outside 1..={SHELLS_MAX}"),
            Self::ProfileLengthsDiffer { name, length, shells } => {
                write!(f, "{name} holds {length} values for {shells} shells")
            }
            Self::BeamComponentCountOutOfRange { components } => write!(
                f,
                "{components} beam components is outside 1..={BEAM_COMPONENTS_MAX}, or the three beam lists differ in length"
            ),
            Self::ShellVolumeOutOfRange { shell, value } => write!(
                f,
                "the volume of shell {shell}, {value} m^3, is outside {SHELL_VOLUME_MIN_M3}..={SHELL_VOLUME_MAX_M3}"
            ),
            Self::ElectronTemperatureOutOfRange { shell, value } => write!(
                f,
                "the electron temperature of shell {shell}, {value} keV, is outside {ELECTRON_TEMPERATURE_MIN_KEV}..={ELECTRON_TEMPERATURE_MAX_KEV}"
            ),
            Self::IonTemperatureOutOfRange { shell, value } => write!(
                f,
                "the ion temperature of shell {shell}, {value} keV, is outside {ION_TEMPERATURE_MIN_KEV}..={ION_TEMPERATURE_MAX_KEV}"
            ),
            Self::TemperatureRatioOutOfRange { shell, value } => write!(
                f,
                "the electron over ion temperature of shell {shell}, {value}, is outside {TEMPERATURE_RATIO_MIN}..={TEMPERATURE_RATIO_MAX}, the span of the slices compared"
            ),
            Self::ElectronDensityOutOfRange { shell, value } => write!(
                f,
                "the electron density of shell {shell}, {value} m^-3, is outside {ELECTRON_DENSITY_MIN_PER_M3}..={ELECTRON_DENSITY_MAX_PER_M3}"
            ),
            Self::IonDensityNegative { shell, value } => {
                write!(f, "an ion density of shell {shell}, {value} m^-3, is below zero")
            }
            Self::TritiumShareOutOfRange { shell, value } => write!(
                f,
                "the tritium share of the fuel of shell {shell}, {value}, is outside {TRITIUM_SHARE_MIN}..={TRITIUM_SHARE_MAX}, the span of the slices compared"
            ),
            Self::ImpurityChargeOutOfRange { value } => write!(
                f,
                "impurity charge {value} is outside {IMPURITY_CHARGE_MIN}..={IMPURITY_CHARGE_MAX}"
            ),
            Self::ImpurityMassOutOfRange { value } => write!(
                f,
                "impurity mass number {value} is outside {IMPURITY_MASS_MIN_U}..={IMPURITY_MASS_MAX_U}"
            ),
            Self::ThermalChargeExceedsElectrons { shell, excess } => write!(
                f,
                "in shell {shell} the thermal ions alone carry {excess} of the electron density more charge than the electrons, above the allowance of {QUASI_NEUTRALITY_ALLOWANCE} (QUASI_NEUTRALITY_ALLOWANCE) by more than {QUASI_NEUTRALITY_END_ALLOWANCE_ULP} units in the last place: quasi-neutrality cannot hold"
            ),
            Self::BeamPowerOutOfRange { value } => write!(
                f,
                "beam power {value} W: a shell's is below 0, or the whole is not above 0 and at most {BEAM_POWER_MAX_W}"
            ),
            Self::BeamSpeciesUnknown { component, value } => write!(
                f,
                "the mass number of beam component {component}, {value}, is neither 2 (deuterium) nor 3 (tritium)"
            ),
            Self::BeamEnergyOutOfRange { component, value } => write!(
                f,
                "the energy of beam component {component}, {value} keV, is outside {BEAM_ENERGY_MIN_KEV}..={BEAM_ENERGY_MAX_KEV}"
            ),
            Self::BeamPowerShareOutOfRange { value } => write!(
                f,
                "beam power share {value}: a share is not above 0, or the shares do not add to 1 within {SHARE_SUM_ALLOWANCE}"
            ),
            Self::TimeSinceBeamChangeOutOfRange { value } => write!(
                f,
                "time since the beam power last changed, {value} s, is outside 0..={TIME_SINCE_BEAM_CHANGE_MAX_S}"
            ),
            Self::NonFiniteValue => write!(f, "an input or an intermediate value was not finite"),
            Self::SlowingRateNotPositive { shell } => write!(
                f,
                "in shell {shell} a beam ion gains energy from the plasma before it reaches the stopping energy"
            ),
            Self::NotSettled { refinements } => write!(
                f,
                "a fused share or the beam-beam rate of a shell was still moving after {refinements} refinements"
            ),
            Self::ParticleLedgerBroken { residual } => write!(f, "slowed and fused beam ions missed one by {residual}"),
            Self::EnergyLedgerBroken { residual } => {
                write!(f, "the energy ledger missed by {residual} of the birth energy")
            }
            Self::PowerLedgerBroken { residual } => {
                write!(f, "the power ledger missed by {residual} of the beam power")
            }
            Self::BeamBeamShareTooLarge { share } => write!(
                f,
                "beam-beam reactions take {share} of the beam ions, above {BEAM_BEAM_SHARE_MAX}"
            ),
            Self::FastIonShareTooLarge { shell, share } => write!(
                f,
                "in shell {shell} the fast ions this solve finds are {share} of the electrons, above {FAST_ION_SHARE_MAX} (FAST_ION_SHARE_MAX), the largest share on a slice compared: a beam ion is slowed on thermal ions only, and other fast ions are not there for it"
            ),
            Self::BeamChangeTooRecent { time_since_beam_change_s, fast_ion_energy_time_s } => write!(
                f,
                "the beam power changed {time_since_beam_change_s} s before this slice, less than {SETTLED_AFTER_ENERGY_TIMES} (SETTLED_AFTER_ENERGY_TIMES) times the fast-ion energy time of {fast_ion_energy_time_s} s this solve finds: the fast ions have not settled, and the model is steady state"
            ),
            Self::IonChargeExceedsElectrons { shell, excess } => write!(
                f,
                "in shell {shell} the thermal ions and the fast ions this solve finds carry {excess} of the electron density more charge than the electrons, above the allowance of {QUASI_NEUTRALITY_FAST_ION_ALLOWANCE} (QUASI_NEUTRALITY_FAST_ION_ALLOWANCE), the largest excess on a slice compared: quasi-neutrality cannot hold"
            ),
            Self::IonChargeBelowElectrons { shell, deficit } => write!(
                f,
                "in shell {shell} the thermal ions and the fast ions this solve finds carry {deficit} of the electron density less charge than the electrons, above the allowance of {QUASI_NEUTRALITY_DEFICIT_ALLOWANCE} (QUASI_NEUTRALITY_DEFICIT_ALLOWANCE), the largest shortfall on a slice compared: quasi-neutrality cannot hold, and the ions the question leaves out would slow and meet the beam"
            ),
        }
    }
}

impl std::error::Error for SliceReject {}

/// The settled answer for one time slice.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceFusion {
    /// D-T reactions per second between thermal ions.
    pub thermal_dt_rate_per_s: f64,
    /// D-T reactions per second of beam ions with thermal ions.
    pub beam_target_dt_rate_per_s: f64,
    /// D-T reactions per second of fast deuterons with fast tritons.
    pub beam_beam_dt_rate_per_s: f64,
    /// All D-T reactions per second: the 14 MeV neutrons per second.
    pub dt_rate_per_s: f64,
    /// D-T fusion power, watts: reactions per second times 17.589 MeV.
    pub dt_fusion_power_w: f64,
    /// D-D neutrons per second: the neutron branch, thermal and beam-target.
    pub dd_neutron_rate_per_s: f64,
    /// D-D fusion power, watts: both branches, thermal and beam-target.
    pub dd_fusion_power_w: f64,
    /// All neutrons per second.
    pub neutron_rate_per_s: f64,
    /// Fast ions in the plasma.
    pub fast_ion_count: f64,
    /// Kinetic energy of the fast ions, joules.
    pub fast_ion_energy_j: f64,
    /// The fast ions' energy time, seconds: their kinetic energy over the deposited beam power.
    pub fast_ion_energy_time_s: f64,
    /// Beam power given to electrons, watts.
    pub beam_power_to_electrons_w: f64,
    /// Beam power given to thermal ions by drag, watts.
    pub beam_power_to_ions_w: f64,
    /// Beam power the ions still hold when they join the thermal plasma, watts.
    pub beam_power_thermalised_w: f64,
    /// Share of the beam ions that fuse with thermal ions before they stop.
    pub beam_target_fused_fraction: f64,
    /// Beam-beam D-T reactions over beam ions born: outside the ledgers.
    pub beam_beam_fused_fraction: f64,
    /// Largest fast-ion density over electron density of any shell.
    pub fast_ion_fraction_max: f64,
    /// Largest of (thermal ion charge + fast ions − electrons) / electrons over the shells.
    pub charge_excess_max: f64,
    /// Smallest of the same over the shells.
    pub charge_excess_min: f64,
    /// Largest miss of slowed plus fused from one, over shells and components.
    pub particle_ledger_residual: f64,
    /// Largest miss of the energy ledger, as a fraction of the birth energy.
    pub energy_ledger_residual: f64,
    /// Miss of the whole plasma's power ledger, as a fraction of the beam power.
    pub power_ledger_residual: f64,
    /// Refinements taken by the shell that took the most.
    pub refinements: u32,
    /// D-T reactions per cubic metre per second in each shell, all three kinds.
    pub dt_rate_density_per_m3_s: Vec<f64>,
}

/// A fault a test puts into a shell to see it named.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Fault {
    None,
    /// The impurity's drag slows the ion and is left out of the energy given to ions.
    ImpurityDragNotCredited,
    /// Fused ions are counted twice.
    FusedCountedTwice,
}

/// One shell of the question.
#[derive(Clone, Copy, Debug)]
struct Shell {
    electron_temperature_kev: f64,
    ion_temperature_kev: f64,
    electron_density_per_m3: f64,
    deuterium: Species,
    tritium: Species,
    impurity: Species,
}

/// One beam component in one shell at one refinement, per ion born.
#[derive(Clone, Debug)]
struct Slowed {
    /// Share that fuses with thermal ions: D-T, D-D neutron branch, D-D proton branch.
    fused: [f64; 3],
    to_electrons_kev: f64,
    to_ions_kev: f64,
    /// Energy carried into fusion by the share that fused.
    fused_energy_kev: f64,
    /// Energy the ion still holds at the stopping energy, times the share that reaches it.
    thermalised_kev: f64,
    slowing_time_s: f64,
    /// `∫ E S / L dE`: energy held while slowing, keV seconds.
    energy_time_kev_s: f64,
    particle_residual: f64,
    energy_residual: f64,
    /// Energy of each node and the fast ions it stands for per ion born per second.
    spectrum: Vec<(f64, f64)>,
}

fn shell_at(question: &SliceQuestion, index: usize) -> Shell {
    Shell {
        electron_temperature_kev: question.electron_temperature_kev[index],
        ion_temperature_kev: question.ion_temperature_kev[index],
        electron_density_per_m3: question.electron_density_per_m3[index],
        deuterium: Species { charge: 1.0, mass_kg: DEUTERON_MASS_KG, density_per_m3: question.deuterium_density_per_m3[index] },
        tritium: Species { charge: 1.0, mass_kg: TRITON_MASS_KG, density_per_m3: question.tritium_density_per_m3[index] },
        impurity: Species {
            charge: question.impurity_charge,
            mass_kg: question.impurity_mass_u * ATOMIC_MASS_KG,
            density_per_m3: question.impurity_density_per_m3[index],
        },
    }
}

/// The charge density of the thermal ions of one shell, `n_D + n_T + Z n_Z`,
/// formed the same way at the door and after the solve.
fn thermal_charge(question: &SliceQuestion, shell: usize) -> f64 {
    question.deuterium_density_per_m3[shell]
        + question.tritium_density_per_m3[shell]
        + question.impurity_charge * question.impurity_density_per_m3[shell]
}

/// The named limits a question itself shows: the counts, the ranges, the shares and quasi-neutrality.
pub fn admit(question: &SliceQuestion) -> Result<(), SliceReject> {
    let shells = question.shell_volume_m3.len();
    if !(1..=SHELLS_MAX).contains(&shells) {
        return Err(SliceReject::ShellCountOutOfRange { shells });
    }
    for (name, profile) in [
        ("electron_temperature_kev", &question.electron_temperature_kev),
        ("ion_temperature_kev", &question.ion_temperature_kev),
        ("electron_density_per_m3", &question.electron_density_per_m3),
        ("deuterium_density_per_m3", &question.deuterium_density_per_m3),
        ("tritium_density_per_m3", &question.tritium_density_per_m3),
        ("impurity_density_per_m3", &question.impurity_density_per_m3),
        ("beam_power_w", &question.beam_power_w),
    ] {
        if profile.len() != shells {
            return Err(SliceReject::ProfileLengthsDiffer { name, length: profile.len(), shells });
        }
    }
    let components = question.beam_mass_number.len();
    if !(1..=BEAM_COMPONENTS_MAX).contains(&components)
        || question.beam_energy_kev.len() != components
        || question.beam_power_share.len() != components
    {
        return Err(SliceReject::BeamComponentCountOutOfRange { components });
    }
    let every = question
        .shell_volume_m3
        .iter()
        .chain(&question.electron_temperature_kev)
        .chain(&question.ion_temperature_kev)
        .chain(&question.electron_density_per_m3)
        .chain(&question.deuterium_density_per_m3)
        .chain(&question.tritium_density_per_m3)
        .chain(&question.impurity_density_per_m3)
        .chain(&question.beam_power_w)
        .chain(&question.beam_mass_number)
        .chain(&question.beam_energy_kev)
        .chain(&question.beam_power_share)
        .chain([&question.impurity_charge, &question.impurity_mass_u, &question.time_since_beam_change_s]);
    for value in every {
        if !value.is_finite() {
            return Err(SliceReject::NonFiniteValue);
        }
    }
    if !(IMPURITY_CHARGE_MIN..=IMPURITY_CHARGE_MAX).contains(&question.impurity_charge) {
        return Err(SliceReject::ImpurityChargeOutOfRange { value: question.impurity_charge });
    }
    if !(IMPURITY_MASS_MIN_U..=IMPURITY_MASS_MAX_U).contains(&question.impurity_mass_u) {
        return Err(SliceReject::ImpurityMassOutOfRange { value: question.impurity_mass_u });
    }
    let mut power = 0.0;
    for shell in 0..shells {
        let value = question.shell_volume_m3[shell];
        if !(SHELL_VOLUME_MIN_M3..=SHELL_VOLUME_MAX_M3).contains(&value) {
            return Err(SliceReject::ShellVolumeOutOfRange { shell, value });
        }
        let value = question.electron_temperature_kev[shell];
        if !(ELECTRON_TEMPERATURE_MIN_KEV..=ELECTRON_TEMPERATURE_MAX_KEV).contains(&value) {
            return Err(SliceReject::ElectronTemperatureOutOfRange { shell, value });
        }
        let value = question.ion_temperature_kev[shell];
        if !(ION_TEMPERATURE_MIN_KEV..=ION_TEMPERATURE_MAX_KEV).contains(&value) {
            return Err(SliceReject::IonTemperatureOutOfRange { shell, value });
        }
        let value = question.electron_temperature_kev[shell] / question.ion_temperature_kev[shell];
        if !(TEMPERATURE_RATIO_MIN..=TEMPERATURE_RATIO_MAX).contains(&value) {
            return Err(SliceReject::TemperatureRatioOutOfRange { shell, value });
        }
        let electrons = question.electron_density_per_m3[shell];
        if !(ELECTRON_DENSITY_MIN_PER_M3..=ELECTRON_DENSITY_MAX_PER_M3).contains(&electrons) {
            return Err(SliceReject::ElectronDensityOutOfRange { shell, value: electrons });
        }
        let (deuterium, tritium, impurity) = (
            question.deuterium_density_per_m3[shell],
            question.tritium_density_per_m3[shell],
            question.impurity_density_per_m3[shell],
        );
        for value in [deuterium, tritium, impurity] {
            if !(value >= 0.0) {
                return Err(SliceReject::IonDensityNegative { shell, value });
            }
        }
        let value = tritium / (deuterium + tritium);
        if !(TRITIUM_SHARE_MIN..=TRITIUM_SHARE_MAX).contains(&value) {
            return Err(SliceReject::TritiumShareOutOfRange { shell, value });
        }
        let thermal_charge_over_electrons = thermal_charge(question, shell) / electrons;
        if !(thermal_charge_over_electrons <= THERMAL_CHARGE_OVER_ELECTRONS_HIGHEST_ANSWERED) {
            return Err(SliceReject::ThermalChargeExceedsElectrons { shell, excess: thermal_charge_over_electrons - 1.0 });
        }
        let value = question.beam_power_w[shell];
        if !(value >= 0.0) {
            return Err(SliceReject::BeamPowerOutOfRange { value });
        }
        power += value;
    }
    if !(power > 0.0 && power <= BEAM_POWER_MAX_W) {
        return Err(SliceReject::BeamPowerOutOfRange { value: power });
    }
    let mut shares = 0.0;
    for component in 0..components {
        let value = question.beam_mass_number[component];
        if value != 2.0 && value != 3.0 {
            return Err(SliceReject::BeamSpeciesUnknown { component, value });
        }
        let value = question.beam_energy_kev[component];
        if !(BEAM_ENERGY_MIN_KEV..=BEAM_ENERGY_MAX_KEV).contains(&value) {
            return Err(SliceReject::BeamEnergyOutOfRange { component, value });
        }
        let value = question.beam_power_share[component];
        if !(value > 0.0) {
            return Err(SliceReject::BeamPowerShareOutOfRange { value });
        }
        shares += value;
    }
    if !((shares - 1.0).abs() <= SHARE_SUM_ALLOWANCE) {
        return Err(SliceReject::BeamPowerShareOutOfRange { value: shares });
    }
    let value = question.time_since_beam_change_s;
    if !(0.0..=TIME_SINCE_BEAM_CHANGE_MAX_S).contains(&value) {
        return Err(SliceReject::TimeSinceBeamChangeOutOfRange { value });
    }
    Ok(())
}

/// The reactions a beam ion of one mass has with the thermal plasma: the
/// reaction, the target's mass and the target's density, in the order of
/// [`Slowed::fused`]. A triton meets deuterium only.
fn targets(beam_mass_kg: f64, shell: &Shell) -> [(Reaction, f64, f64); 3] {
    if beam_mass_kg == DEUTERON_MASS_KG {
        [
            (Reaction::DeuteriumTritium, TRITON_MASS_KG, shell.tritium.density_per_m3),
            (Reaction::DeuteriumDeuteriumNeutron, DEUTERON_MASS_KG, shell.deuterium.density_per_m3),
            (Reaction::DeuteriumDeuteriumProton, DEUTERON_MASS_KG, shell.deuterium.density_per_m3),
        ]
    } else {
        [
            (Reaction::DeuteriumTritium, DEUTERON_MASS_KG, shell.deuterium.density_per_m3),
            (Reaction::DeuteriumDeuteriumNeutron, DEUTERON_MASS_KG, 0.0),
            (Reaction::DeuteriumDeuteriumProton, DEUTERON_MASS_KG, 0.0),
        ]
    }
}

/// One beam ion of `beam_mass_kg` born at `birth_energy_kev` in one shell,
/// with energy panels `panel_kev` wide from the birth energy down.
fn slow_and_fuse(
    beam_mass_kg: f64,
    birth_energy_kev: f64,
    shell: &Shell,
    shell_index: usize,
    panel_kev: f64,
    fault: Fault,
) -> Result<Slowed, SliceReject> {
    let stopping = STOPPING_ENERGY_OVER_ION_TEMPERATURE * shell.ion_temperature_kev;
    if birth_energy_kev <= stopping {
        return Ok(Slowed {
            fused: [0.0; 3],
            to_electrons_kev: 0.0,
            to_ions_kev: 0.0,
            fused_energy_kev: 0.0,
            thermalised_kev: birth_energy_kev,
            slowing_time_s: 0.0,
            energy_time_kev_s: 0.0,
            particle_residual: 0.0,
            energy_residual: 0.0,
            spectrum: Vec::new(),
        });
    }
    let plasma = Background::new(shell.electron_density_per_m3, shell.electron_temperature_kev, shell.ion_temperature_kev);
    let ion = Species { charge: 1.0, mass_kg: beam_mass_kg, density_per_m3: 0.0 };
    let targets = targets(beam_mass_kg, shell);

    let span = birth_energy_kev - stopping;
    let panels = ((span / panel_kev).ceil() as usize).max(1);
    let width = span / panels as f64;
    let half = 0.5 * width;

    let mut surviving = 1.0;
    let mut fused = [0.0; 3];
    let mut fused_energy_kev = 0.0;
    let mut to_electrons_kev = 0.0;
    let mut to_ions_kev = 0.0;
    let mut slowing_time_s = 0.0;
    let mut energy_time_kev_s = 0.0;
    let mut spectrum = Vec::with_capacity(8 * panels);
    for panel in (0..panels).rev() {
        let middle = stopping + (panel as f64 + 0.5) * width;
        let top = stopping + (panel as f64 + 1.0) * width;
        // First the chance of fusing through the whole panel, then, with the
        // share still slowing at each node known from it, everything else.
        let mut nodes = [(0.0, 0.0, 0.0, 0.0, 0.0, [0.0; 3]); 8];
        let mut chance = 0.0;
        for (slot, (node, weight)) in nodes.iter_mut().zip(reaction::GAUSS_NODES.iter().zip(reaction::GAUSS_WEIGHTS)) {
            let energy = middle + half * node;
            let weight = half * weight;
            let electrons_rate = plasma.loss_to_electrons_kev_per_s(&ion, energy);
            let fuel_rate = plasma.loss_to_ions_kev_per_s(&ion, energy, &shell.deuterium)
                + plasma.loss_to_ions_kev_per_s(&ion, energy, &shell.tritium);
            let impurity_rate = plasma.loss_to_ions_kev_per_s(&ion, energy, &shell.impurity);
            let total = electrons_rate + fuel_rate + impurity_rate;
            if !total.is_finite() {
                return Err(SliceReject::NonFiniteValue);
            }
            if !(total > 0.0) {
                return Err(SliceReject::SlowingRateNotPositive { shell: shell_index });
            }
            let mut per_kev = [0.0; 3];
            for (rate, (kind, target_mass_kg, density)) in per_kev.iter_mut().zip(targets) {
                if density > 0.0 {
                    *rate = density
                        * reaction::beam_target_rate_for_m3_per_s(
                            |e| bosch_hale::cross_section_or_zero(kind, e),
                            beam_mass_kg,
                            target_mass_kg,
                            energy,
                            shell.ion_temperature_kev,
                            THERMAL_AVERAGE_PANEL_KEV,
                        )
                        / total;
                }
            }
            chance += weight * (per_kev[0] + per_kev[1] + per_kev[2]);
            let ions_rate = if fault == Fault::ImpurityDragNotCredited { fuel_rate } else { fuel_rate + impurity_rate };
            *slot = (energy, weight, electrons_rate / total, ions_rate / total, 1.0 / total, per_kev);
        }
        let fused_here = -surviving * (-chance).exp_m1();
        let mut fusing = 0.0;
        let mut fusing_energy = 0.0;
        let mut fusing_each = [0.0; 3];
        for (energy, weight, electrons_share, ions_share, time, per_kev) in nodes {
            // The panel's chance is spread evenly over its width.
            let here = surviving * (-chance * (top - energy) / width).exp();
            let weight = weight * here;
            let all = per_kev[0] + per_kev[1] + per_kev[2];
            fusing += weight * all;
            fusing_energy += weight * all * energy;
            for (sum, rate) in fusing_each.iter_mut().zip(per_kev) {
                *sum += weight * rate;
            }
            to_electrons_kev += weight * electrons_share;
            to_ions_kev += weight * ions_share;
            slowing_time_s += weight * time;
            energy_time_kev_s += weight * time * energy;
            spectrum.push((energy, weight * time));
        }
        if fusing > 0.0 {
            for (sum, each) in fused.iter_mut().zip(fusing_each) {
                *sum += fused_here * each / fusing;
            }
            fused_energy_kev += fused_here * fusing_energy / fusing;
        }
        surviving -= fused_here;
    }
    if fault == Fault::FusedCountedTwice {
        for share in &mut fused {
            *share *= 2.0;
        }
    }
    let answer = Slowed {
        fused,
        to_electrons_kev,
        to_ions_kev,
        fused_energy_kev,
        thermalised_kev: surviving * stopping,
        slowing_time_s,
        energy_time_kev_s,
        particle_residual: 1.0 - surviving - (fused[0] + fused[1] + fused[2]),
        energy_residual: (birth_energy_kev - to_electrons_kev - to_ions_kev - fused_energy_kev - surviving * stopping)
            / birth_energy_kev,
        spectrum,
    };
    hold(&answer)?;
    Ok(answer)
}

/// Holds one component's answer in one shell to the two ledgers.
fn hold(answer: &Slowed) -> Result<(), SliceReject> {
    for value in [
        answer.fused[0],
        answer.fused[1],
        answer.fused[2],
        answer.to_electrons_kev,
        answer.to_ions_kev,
        answer.thermalised_kev,
        answer.slowing_time_s,
        answer.energy_time_kev_s,
        answer.particle_residual,
        answer.energy_residual,
    ] {
        if !value.is_finite() {
            return Err(SliceReject::NonFiniteValue);
        }
    }
    if !(answer.particle_residual.abs() <= PARTICLE_LEDGER_TOLERANCE) {
        return Err(SliceReject::ParticleLedgerBroken { residual: answer.particle_residual });
    }
    if !(answer.energy_residual.abs() <= ENERGY_LEDGER_TOLERANCE) {
        return Err(SliceReject::EnergyLedgerBroken { residual: answer.energy_residual });
    }
    Ok(())
}

/// `G(u) = ∫₀^u σ_DT(½ μ u′²) u′² du′` in barn m³/s³ at the edges of
/// [`BEAM_BEAM_CELLS`] equal cells, with its slope `σ u²` there.
pub struct BeamBeamTable {
    step: f64,
    value: Vec<f64>,
    slope: Vec<f64>,
}

impl BeamBeamTable {
    pub fn new() -> Self {
        let reduced = DEUTERON_MASS_KG * TRITON_MASS_KG / (DEUTERON_MASS_KG + TRITON_MASS_KG);
        let to_kev = 0.5 * reduced / JOULE_PER_KEV;
        let weight = |u: f64| bosch_hale::cross_section_or_zero(Reaction::DeuteriumTritium, to_kev * u * u) * u * u;
        let step = BEAM_BEAM_SPEED_MAX_M_PER_S / BEAM_BEAM_CELLS as f64;
        let mut value = Vec::with_capacity(BEAM_BEAM_CELLS + 1);
        let mut slope = Vec::with_capacity(BEAM_BEAM_CELLS + 1);
        value.push(0.0);
        slope.push(0.0);
        let mut sum = 0.0;
        for cell in 0..BEAM_BEAM_CELLS {
            sum += reaction::integrate(cell as f64 * step, (cell as f64 + 1.0) * step, 1, weight);
            value.push(sum);
            slope.push(weight((cell as f64 + 1.0) * step));
        }
        Self { step, value, slope }
    }

    /// `G(u)`: the cubic through the two cell edges with the exact slopes there.
    fn at(&self, speed_m_per_s: f64) -> f64 {
        let x = speed_m_per_s / self.step;
        let cell = (x as usize).min(BEAM_BEAM_CELLS - 1);
        let s = x - cell as f64;
        let (g0, g1) = (self.value[cell], self.value[cell + 1]);
        let (d0, d1) = (self.slope[cell] * self.step, self.slope[cell + 1] * self.step);
        g0 * (1.0 + 2.0 * s) * (1.0 - s) * (1.0 - s) + d0 * s * (1.0 - s) * (1.0 - s) + g1 * s * s * (3.0 - 2.0 * s)
            + d1 * s * s * (s - 1.0)
    }

    /// `⟨σ u⟩` in m³/s between a deuteron of speed `deuteron` and a triton of
    /// speed `triton`, averaged over every angle between them.
    pub fn kernel_m3_per_s(&self, deuteron_m_per_s: f64, triton_m_per_s: f64) -> f64 {
        (self.at(deuteron_m_per_s + triton_m_per_s) - self.at((deuteron_m_per_s - triton_m_per_s).abs())) * 1.0e-28
            / (2.0 * deuteron_m_per_s * triton_m_per_s)
    }
}

impl Default for BeamBeamTable {
    fn default() -> Self {
        Self::new()
    }
}

/// One shell at one refinement: each component's answer and the beam-beam
/// D-T reactions per second in the shell.
fn shell_once(
    question: &SliceQuestion,
    shell_index: usize,
    births_per_s: &[f64],
    table: &BeamBeamTable,
    panel_kev: f64,
    fault: Fault,
) -> Result<(Vec<Slowed>, f64), SliceReject> {
    let shell = shell_at(question, shell_index);
    let mut answers = Vec::with_capacity(births_per_s.len());
    for component in 0..births_per_s.len() {
        let mass = if question.beam_mass_number[component] == 2.0 { DEUTERON_MASS_KG } else { TRITON_MASS_KG };
        answers.push(slow_and_fuse(mass, question.beam_energy_kev[component], &shell, shell_index, panel_kev, fault)?);
    }
    let mut beam_beam = 0.0;
    for (a, deuterons) in answers.iter().enumerate() {
        if question.beam_mass_number[a] != 2.0 {
            continue;
        }
        for (b, tritons) in answers.iter().enumerate() {
            if question.beam_mass_number[b] != 3.0 {
                continue;
            }
            let mut pair = 0.0;
            for &(deuteron_kev, deuteron_weight) in &deuterons.spectrum {
                let deuteron_speed = (2.0 * deuteron_kev * JOULE_PER_KEV / DEUTERON_MASS_KG).sqrt();
                let mut inner = 0.0;
                for &(triton_kev, triton_weight) in &tritons.spectrum {
                    let triton_speed = (2.0 * triton_kev * JOULE_PER_KEV / TRITON_MASS_KG).sqrt();
                    inner += triton_weight * table.kernel_m3_per_s(deuteron_speed, triton_speed);
                }
                pair += deuteron_weight * inner;
            }
            beam_beam += births_per_s[a] * births_per_s[b] * pair / question.shell_volume_m3[shell_index];
        }
    }
    if !beam_beam.is_finite() {
        return Err(SliceReject::NonFiniteValue);
    }
    Ok((answers, beam_beam))
}

/// One shell refined until every fused share and the beam-beam rate settle.
fn settle_shell(
    question: &SliceQuestion,
    shell_index: usize,
    births_per_s: &[f64],
    table: &BeamBeamTable,
    refinement_cap: u32,
    fault: Fault,
) -> Result<(Vec<Slowed>, f64, u32), SliceReject> {
    let mut panel_kev = FIRST_PANEL_KEV;
    let mut previous: Option<(Vec<Slowed>, f64)> = None;
    for refinement in 1..=refinement_cap {
        let current = shell_once(question, shell_index, births_per_s, table, panel_kev, fault)?;
        if let Some(previous) = &previous {
            let moved = |now: f64, before: f64| (now - before).abs() > SETTLE_TOLERANCE * now.abs();
            let mut settled = !moved(current.1, previous.1);
            for (now, before) in current.0.iter().zip(&previous.0) {
                for kind in 0..3 {
                    settled &= !moved(now.fused[kind], before.fused[kind]);
                }
            }
            if settled {
                return Ok((current.0, current.1, refinement));
            }
        }
        previous = Some(current);
        panel_kev *= 0.5;
    }
    Err(SliceReject::NotSettled { refinements: refinement_cap })
}

/// What one beam ion does in one uniform plasma: the shares that fuse with
/// thermal tritium or deuterium (D-T, D-D neutron branch, D-D proton branch)
/// and the mean slowing time in seconds, settled and held to both ledgers.
/// For the outside recomputation and for tests; it enforces no range.
#[allow(clippy::too_many_arguments)]
pub fn fused_shares_in_uniform_plasma(
    beam_mass_number: f64,
    beam_energy_kev: f64,
    electron_density_per_m3: f64,
    electron_temperature_kev: f64,
    ion_temperature_kev: f64,
    deuterium_density_per_m3: f64,
    tritium_density_per_m3: f64,
    impurity_density_per_m3: f64,
    impurity_charge: f64,
    impurity_mass_u: f64,
) -> Result<([f64; 3], f64), SliceReject> {
    let question = SliceQuestion {
        shell_volume_m3: vec![1.0],
        electron_temperature_kev: vec![electron_temperature_kev],
        ion_temperature_kev: vec![ion_temperature_kev],
        electron_density_per_m3: vec![electron_density_per_m3],
        deuterium_density_per_m3: vec![deuterium_density_per_m3],
        tritium_density_per_m3: vec![tritium_density_per_m3],
        impurity_density_per_m3: vec![impurity_density_per_m3],
        impurity_charge,
        impurity_mass_u,
        beam_power_w: vec![1.0],
        beam_mass_number: vec![beam_mass_number],
        beam_energy_kev: vec![beam_energy_kev],
        beam_power_share: vec![1.0],
        time_since_beam_change_s: TIME_SINCE_BEAM_CHANGE_MAX_S,
    };
    let table = BeamBeamTable::new();
    let (answers, _, _) = settle_shell(&question, 0, &[1.0], &table, MAX_REFINEMENTS, Fault::None)?;
    Ok((answers[0].fused, answers[0].slowing_time_s))
}

/// The D-T fusion rate of one time slice.
pub fn solve(question: &SliceQuestion) -> Result<SliceFusion, SliceReject> {
    solve_with(question, MAX_REFINEMENTS, Fault::None)
}

fn solve_with(question: &SliceQuestion, refinement_cap: u32, fault: Fault) -> Result<SliceFusion, SliceReject> {
    admit(question)?;
    settle(question, refinement_cap, fault)
}

/// The solve of a question already admitted.
fn settle(question: &SliceQuestion, refinement_cap: u32, fault: Fault) -> Result<SliceFusion, SliceReject> {
    let shells = question.shell_volume_m3.len();
    let components = question.beam_mass_number.len();
    let table = BeamBeamTable::new();
    let dt_kev = Reaction::DeuteriumTritium.energy_kev();
    let ddn_kev = Reaction::DeuteriumDeuteriumNeutron.energy_kev();
    let ddp_kev = Reaction::DeuteriumDeuteriumProton.energy_kev();

    let mut thermal_dt = 0.0;
    let mut thermal_ddn = 0.0;
    let mut thermal_ddp = 0.0;
    let mut beam_target = [0.0; 3];
    let mut beam_beam = 0.0;
    let mut born_per_s = 0.0;
    let mut fast_count = 0.0;
    let mut fast_energy_kev = 0.0;
    let mut to_electrons_kev_per_s = 0.0;
    let mut to_ions_kev_per_s = 0.0;
    let mut thermalised_kev_per_s = 0.0;
    let mut fused_energy_kev_per_s = 0.0;
    let mut fast_fraction_max: f64 = 0.0;
    let mut fast_fraction_shell = 0;
    let mut charge_excess_max = f64::NEG_INFINITY;
    let mut charge_excess_min = f64::INFINITY;
    let mut charge_highest = (0, f64::NEG_INFINITY);
    let mut charge_lowest = (0, f64::INFINITY);
    let mut particle_residual: f64 = 0.0;
    let mut energy_residual: f64 = 0.0;
    let mut refinements = 0;
    let mut density = Vec::with_capacity(shells);
    let mut beam_power_w = 0.0;
    for shell in 0..shells {
        let volume = question.shell_volume_m3[shell];
        let ion_temperature = question.ion_temperature_kev[shell];
        let deuterium = question.deuterium_density_per_m3[shell];
        let tritium = question.tritium_density_per_m3[shell];
        let fit = |kind| bosch_hale::reactivity_m3_per_s(kind, ion_temperature).ok_or(SliceReject::NonFiniteValue);
        let thermal_here = deuterium * tritium * fit(Reaction::DeuteriumTritium)? * volume;
        thermal_dt += thermal_here;
        thermal_ddn += 0.5 * deuterium * deuterium * fit(Reaction::DeuteriumDeuteriumNeutron)? * volume;
        thermal_ddp += 0.5 * deuterium * deuterium * fit(Reaction::DeuteriumDeuteriumProton)? * volume;

        let power = question.beam_power_w[shell];
        beam_power_w += power;
        let mut here = thermal_here;
        let mut fast_here = 0.0;
        if power > 0.0 {
            let births: Vec<f64> = (0..components)
                .map(|c| power * question.beam_power_share[c] / (question.beam_energy_kev[c] * JOULE_PER_KEV))
                .collect();
            let (answers, beam_beam_here, taken) = settle_shell(question, shell, &births, &table, refinement_cap, fault)?;
            refinements = refinements.max(taken);
            beam_beam += beam_beam_here;
            here += beam_beam_here;
            for (answer, born) in answers.iter().zip(&births) {
                born_per_s += born;
                for kind in 0..3 {
                    beam_target[kind] += born * answer.fused[kind];
                }
                here += born * answer.fused[0];
                fast_here += born * answer.slowing_time_s;
                fast_energy_kev += born * answer.energy_time_kev_s;
                to_electrons_kev_per_s += born * answer.to_electrons_kev;
                to_ions_kev_per_s += born * answer.to_ions_kev;
                thermalised_kev_per_s += born * answer.thermalised_kev;
                fused_energy_kev_per_s += born * answer.fused_energy_kev;
                particle_residual = particle_residual.max(answer.particle_residual.abs());
                energy_residual = energy_residual.max(answer.energy_residual.abs());
            }
        }
        fast_count += fast_here;
        let electrons = question.electron_density_per_m3[shell];
        let fast_fraction = fast_here / volume / electrons;
        if fast_fraction > fast_fraction_max {
            fast_fraction_max = fast_fraction;
            fast_fraction_shell = shell;
        }
        // The charge of the thermal ions and of the fast ions this solve
        // finds, over the electrons: with no fast ions the same bits as the door's ratio.
        let ion_charge_over_electrons = (thermal_charge(question, shell) + fast_here / volume) / electrons;
        if ion_charge_over_electrons > charge_highest.1 {
            charge_highest = (shell, ion_charge_over_electrons);
        }
        if ion_charge_over_electrons < charge_lowest.1 {
            charge_lowest = (shell, ion_charge_over_electrons);
        }
        let excess = (thermal_charge(question, shell) + fast_here / volume - electrons) / electrons;
        charge_excess_max = charge_excess_max.max(excess);
        charge_excess_min = charge_excess_min.min(excess);
        density.push(here / volume);
    }
    let beam_target_dt = beam_target[0];
    let dt_rate = thermal_dt + beam_target_dt + beam_beam;
    let dd_neutrons = thermal_ddn + beam_target[1];
    let beam_power_kev_per_s = beam_power_w / JOULE_PER_KEV;
    let power_residual = (beam_power_kev_per_s
        - to_electrons_kev_per_s
        - to_ions_kev_per_s
        - thermalised_kev_per_s
        - fused_energy_kev_per_s)
        / beam_power_kev_per_s;
    let result = SliceFusion {
        thermal_dt_rate_per_s: thermal_dt,
        beam_target_dt_rate_per_s: beam_target_dt,
        beam_beam_dt_rate_per_s: beam_beam,
        dt_rate_per_s: dt_rate,
        dt_fusion_power_w: dt_rate * dt_kev * JOULE_PER_KEV,
        dd_neutron_rate_per_s: dd_neutrons,
        dd_fusion_power_w: (dd_neutrons * ddn_kev + (thermal_ddp + beam_target[2]) * ddp_kev) * JOULE_PER_KEV,
        neutron_rate_per_s: dt_rate + dd_neutrons,
        fast_ion_count: fast_count,
        fast_ion_energy_j: fast_energy_kev * JOULE_PER_KEV,
        fast_ion_energy_time_s: fast_energy_kev * JOULE_PER_KEV / beam_power_w,
        beam_power_to_electrons_w: to_electrons_kev_per_s * JOULE_PER_KEV,
        beam_power_to_ions_w: to_ions_kev_per_s * JOULE_PER_KEV,
        beam_power_thermalised_w: thermalised_kev_per_s * JOULE_PER_KEV,
        beam_target_fused_fraction: (beam_target[0] + beam_target[1] + beam_target[2]) / born_per_s,
        beam_beam_fused_fraction: beam_beam / born_per_s,
        fast_ion_fraction_max: fast_fraction_max,
        charge_excess_max,
        charge_excess_min,
        particle_ledger_residual: particle_residual,
        energy_ledger_residual: energy_residual,
        power_ledger_residual: power_residual,
        refinements,
        dt_rate_density_per_m3_s: density,
    };
    for value in [
        result.dt_fusion_power_w,
        result.dd_fusion_power_w,
        result.neutron_rate_per_s,
        result.fast_ion_energy_j,
        result.fast_ion_energy_time_s,
        result.fast_ion_fraction_max,
        result.beam_target_fused_fraction,
        result.beam_beam_fused_fraction,
        result.charge_excess_max,
        result.charge_excess_min,
        result.power_ledger_residual,
    ] {
        if !value.is_finite() {
            return Err(SliceReject::NonFiniteValue);
        }
    }
    if !(result.power_ledger_residual.abs() <= POWER_LEDGER_TOLERANCE) {
        return Err(SliceReject::PowerLedgerBroken { residual: result.power_ledger_residual });
    }
    if !(result.beam_beam_fused_fraction <= BEAM_BEAM_SHARE_MAX) {
        return Err(SliceReject::BeamBeamShareTooLarge { share: result.beam_beam_fused_fraction });
    }
    if !(result.fast_ion_fraction_max <= FAST_ION_SHARE_MAX) {
        return Err(SliceReject::FastIonShareTooLarge { shell: fast_fraction_shell, share: result.fast_ion_fraction_max });
    }
    // The caller's time against the product formed here, as computed: a
    // caller who multiplies the returned energy time by the same constant
    // forms the same 64-bit number, so the end needs no allowance.
    if !(question.time_since_beam_change_s >= SETTLED_AFTER_ENERGY_TIMES * result.fast_ion_energy_time_s) {
        return Err(SliceReject::BeamChangeTooRecent {
            time_since_beam_change_s: question.time_since_beam_change_s,
            fast_ion_energy_time_s: result.fast_ion_energy_time_s,
        });
    }
    // Quasi-neutrality with the fast ions counted, both ways.
    if !(charge_highest.1 <= ION_CHARGE_OVER_ELECTRONS_HIGHEST_ANSWERED) {
        return Err(SliceReject::IonChargeExceedsElectrons { shell: charge_highest.0, excess: charge_highest.1 - 1.0 });
    }
    if !(charge_lowest.1 >= ION_CHARGE_OVER_ELECTRONS_LOWEST_ANSWERED) {
        return Err(SliceReject::IonChargeBelowElectrons { shell: charge_lowest.0, deficit: 1.0 - charge_lowest.1 });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokamak_dt_beam_target_reference as jet;

    /// JET pulse 42976 at 13.35 s: fifty shells, deuterium beams at 80 kV and
    /// tritium beams at 152 kV, each with its three energies.
    fn jet_42976() -> SliceQuestion {
        SliceQuestion {
            shell_volume_m3: jet::SHELL_VOLUME_M3.to_vec(),
            electron_temperature_kev: jet::ELECTRON_TEMPERATURE_KEV.to_vec(),
            ion_temperature_kev: jet::ION_TEMPERATURE_KEV.to_vec(),
            electron_density_per_m3: jet::ELECTRON_DENSITY_PER_M3.to_vec(),
            deuterium_density_per_m3: jet::DEUTERIUM_DENSITY_PER_M3.to_vec(),
            tritium_density_per_m3: jet::TRITIUM_DENSITY_PER_M3.to_vec(),
            impurity_density_per_m3: jet::IMPURITY_DENSITY_PER_M3.to_vec(),
            impurity_charge: jet::IMPURITY_CHARGE,
            impurity_mass_u: jet::IMPURITY_MASS_U,
            beam_power_w: jet::BEAM_POWER_W.to_vec(),
            beam_mass_number: jet::BEAM_MASS_NUMBER.to_vec(),
            beam_energy_kev: jet::BEAM_ENERGY_KEV.to_vec(),
            beam_power_share: jet::BEAM_POWER_SHARE.to_vec(),
            time_since_beam_change_s: jet::TIME_SINCE_BEAM_CHANGE_S,
        }
    }

    /// Four shells cut from the same slice, for the tests that need many solves.
    fn small() -> SliceQuestion {
        let full = jet_42976();
        let pick = |profile: &Vec<f64>| [2, 15, 30, 45].iter().map(|&i| profile[i]).collect::<Vec<f64>>();
        SliceQuestion {
            shell_volume_m3: pick(&full.shell_volume_m3),
            electron_temperature_kev: pick(&full.electron_temperature_kev),
            ion_temperature_kev: pick(&full.ion_temperature_kev),
            electron_density_per_m3: pick(&full.electron_density_per_m3),
            deuterium_density_per_m3: pick(&full.deuterium_density_per_m3),
            tritium_density_per_m3: pick(&full.tritium_density_per_m3),
            impurity_density_per_m3: pick(&full.impurity_density_per_m3),
            beam_power_w: pick(&full.beam_power_w),
            beam_mass_number: vec![2.0, 3.0],
            beam_energy_kev: vec![80.0, 152.0],
            beam_power_share: vec![0.5, 0.5],
            // A beam steady for two seconds: more than twenty energy times.
            time_since_beam_change_s: 2.0,
            ..full
        }
    }

    /// Rule 7. The profile database's own fusion power for this slice, read
    /// 2026-10-09 ([`REFERENCE_RETRIEVED`]) from
    /// <https://tokamak-profiledb.ccfe.ac.uk/PR08/itpa_pr08/jet/42976/>. It is
    /// the output of the TRANSP analysis of the pulse from its measured
    /// profiles, which the pulse's comment file says agrees with the
    /// measured neutron rate until 13.2 s and then falls below it. It is not
    /// a raw neutron count. The solver is held to its own value and the
    /// ratio is printed; this pulse is not held-back data.
    #[test]
    fn jet_pulse_42976_at_13_35_s() {
        println!("{REFERENCE_NAME}, read {REFERENCE_RETRIEVED} from {REFERENCE_URL}");
        let answer = solve(&jet_42976()).expect("solves");
        let ratio = answer.dt_fusion_power_w / jet::DATABASE_FUSION_POWER_W;
        println!(
            "solver {:.4e} W, database {:.4e} W, ratio {ratio:.4}; thermal {:.4e}, beam-target {:.4e}, beam-beam {:.4e} per second",
            answer.dt_fusion_power_w,
            jet::DATABASE_FUSION_POWER_W,
            answer.thermal_dt_rate_per_s,
            answer.beam_target_dt_rate_per_s,
            answer.beam_beam_dt_rate_per_s
        );
        assert!(ratio > 0.97 && ratio < 1.0, "{ratio}");
        // The plain-Python model of the research record, written first.
        let off = answer.dt_fusion_power_w / jet::MODEL_FUSION_POWER_W - 1.0;
        assert!(off.abs() < 1.0e-5, "{off}");
        assert_eq!(answer.dt_rate_density_per_m3_s.len(), 50);
    }

    #[test]
    fn every_beam_ion_and_every_kev_is_accounted_for() {
        let question = jet_42976();
        let answer = solve(&question).expect("solves");
        assert!(answer.particle_ledger_residual <= PARTICLE_LEDGER_TOLERANCE);
        assert!(answer.energy_ledger_residual <= ENERGY_LEDGER_TOLERANCE);
        assert!(answer.power_ledger_residual.abs() <= POWER_LEDGER_TOLERANCE);
        let power: f64 = question.beam_power_w.iter().sum();
        let handed = answer.beam_power_to_electrons_w + answer.beam_power_to_ions_w + answer.beam_power_thermalised_w;
        // The rest, under one percent, is the energy fused ions carried into fusion.
        assert!(handed < power && handed > 0.99 * power, "{}", handed / power);
        // The shells add to the total.
        let mut total = 0.0;
        for (density, volume) in answer.dt_rate_density_per_m3_s.iter().zip(&question.shell_volume_m3) {
            total += density * volume;
        }
        assert!((total / answer.dt_rate_per_s - 1.0).abs() < 1.0e-12);
        assert!(
            (answer.thermal_dt_rate_per_s + answer.beam_target_dt_rate_per_s + answer.beam_beam_dt_rate_per_s
                - answer.dt_rate_per_s)
                .abs()
                < 1.0e-12 * answer.dt_rate_per_s
        );
        assert!((answer.neutron_rate_per_s - answer.dt_rate_per_s - answer.dd_neutron_rate_per_s).abs() < 1.0e3);
        assert!(answer.beam_beam_fused_fraction > 0.0 && answer.beam_beam_fused_fraction < BEAM_BEAM_SHARE_MAX);
        assert!(answer.fast_ion_fraction_max > 0.01 && answer.fast_ion_fraction_max < 0.3);
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let first = solve(&small()).expect("solves");
        let second = solve(&small()).expect("solves");
        assert_eq!(first.dt_fusion_power_w.to_bits(), second.dt_fusion_power_w.to_bits());
        for (left, right) in first.dt_rate_density_per_m3_s.iter().zip(&second.dt_rate_density_per_m3_s) {
            assert_eq!(left.to_bits(), right.to_bits());
        }
    }

    #[test]
    fn the_three_rates_follow_the_beam_power_and_the_fuel() {
        let base = solve(&small()).expect("solves");
        let mut halved = small();
        for power in &mut halved.beam_power_w {
            *power *= 0.5;
        }
        let halved = solve(&halved).expect("solves");
        assert_eq!(halved.thermal_dt_rate_per_s.to_bits(), base.thermal_dt_rate_per_s.to_bits());
        assert!((halved.beam_target_dt_rate_per_s / base.beam_target_dt_rate_per_s - 0.5).abs() < 1.0e-12);
        assert!((halved.beam_beam_dt_rate_per_s / base.beam_beam_dt_rate_per_s - 0.25).abs() < 1.0e-12);
        // The thermal tritium made deuterium, so that the charge still balances, and no tritium beam: no D-T
        // reaction, and D-D neutrons remain.
        let as_deuterium = |question: &mut SliceQuestion| {
            for (deuterium, tritium) in question.deuterium_density_per_m3.iter_mut().zip(&mut question.tritium_density_per_m3) {
                *deuterium += *tritium;
                *tritium = 0.0;
            }
        };
        // A tritium-free plasma was never compared: it is refused by name, and its physics is held here without the door.
        let mut deuterium = small();
        as_deuterium(&mut deuterium);
        deuterium.beam_mass_number = vec![2.0, 2.0];
        assert!(matches!(solve(&deuterium), Err(SliceReject::TritiumShareOutOfRange { shell: 0, value: 0.0 })));
        let deuterium = settle(&deuterium, MAX_REFINEMENTS, Fault::None).expect("solves");
        assert_eq!(deuterium.dt_rate_per_s, 0.0);
        assert!(deuterium.dd_neutron_rate_per_s > 1.0e15);
        // A tritium beam alone into deuterium: beam-target only from tritons, no beam-beam.
        let mut tritium = small();
        as_deuterium(&mut tritium);
        tritium.beam_mass_number = vec![3.0, 3.0];
        assert!(matches!(solve(&tritium), Err(SliceReject::TritiumShareOutOfRange { .. })));
        let tritium = settle(&tritium, MAX_REFINEMENTS, Fault::None).expect("solves");
        assert_eq!(tritium.thermal_dt_rate_per_s, 0.0);
        assert_eq!(tritium.beam_beam_dt_rate_per_s, 0.0);
        assert!(tritium.beam_target_dt_rate_per_s > 1.0e16);
    }

    /// The four shells of [`small`] at an ion temperature of 20 keV and an
    /// electron temperature of 11 keV, with one deuterium component at the
    /// lowest beam energy, 26 keV: under 1.5 ion temperatures in every shell.
    /// Its thermal ions balance the electrons, since there are no fast ions.
    fn cold_beam() -> SliceQuestion {
        let mut question = small();
        question.deuterium_density_per_m3 = question.electron_density_per_m3.iter().map(|n| 0.5 * n).collect();
        question.tritium_density_per_m3 = question.deuterium_density_per_m3.clone();
        question.impurity_density_per_m3 = vec![0.0; 4];
        question.ion_temperature_kev = vec![20.0; 4];
        question.electron_temperature_kev = vec![11.0; 4];
        question.beam_mass_number = vec![2.0];
        question.beam_energy_kev = vec![BEAM_ENERGY_MIN_KEV];
        question.beam_power_share = vec![1.0];
        question
    }

    /// A component born at or below 1.5 T_i joins the thermal plasma at once.
    #[test]
    fn a_component_born_below_the_stopping_energy_only_heats() {
        let question = cold_beam();
        let answer = solve(&question).expect("solves");
        let power: f64 = question.beam_power_w.iter().sum();
        assert_eq!(answer.beam_target_dt_rate_per_s, 0.0);
        assert_eq!(answer.fast_ion_count, 0.0);
        assert!((answer.beam_power_thermalised_w / power - 1.0).abs() < 1.0e-12);
        assert_eq!(answer.refinements, 2);
    }

    #[test]
    fn a_question_outside_the_ranges_is_named() {
        let good = small();
        let with = |change: fn(&mut SliceQuestion)| {
            let mut question = small();
            change(&mut question);
            solve(&question)
        };
        assert!(solve(&good).is_ok());
        assert!(matches!(with(|q| q.shell_volume_m3.clear()), Err(SliceReject::ShellCountOutOfRange { shells: 0 })));
        assert!(matches!(
            with(|q| q.ion_temperature_kev.push(1.0)),
            Err(SliceReject::ProfileLengthsDiffer { name: "ion_temperature_kev", length: 5, shells: 4 })
        ));
        assert!(matches!(
            with(|q| q.beam_energy_kev.push(1.0)),
            Err(SliceReject::BeamComponentCountOutOfRange { components: 2 })
        ));
        assert!(matches!(with(|q| q.shell_volume_m3[1] = 0.0), Err(SliceReject::ShellVolumeOutOfRange { shell: 1, .. })));
        // The smallest volume above zero, which once reached the solve and failed there as NonFiniteValue.
        assert!(matches!(with(|q| q.shell_volume_m3[1] = 5e-324), Err(SliceReject::ShellVolumeOutOfRange { shell: 1, .. })));
        assert!(matches!(
            with(|q| q.shell_volume_m3[1] = 0.99e-6),
            Err(SliceReject::ShellVolumeOutOfRange { shell: 1, .. })
        ));
        assert!(matches!(with(|q| q.shell_volume_m3[1] = 200.1), Err(SliceReject::ShellVolumeOutOfRange { shell: 1, .. })));
        assert!(matches!(
            with(|q| q.electron_temperature_kev[0] = 13.11),
            Err(SliceReject::ElectronTemperatureOutOfRange { shell: 0, .. })
        ));
        assert!(matches!(
            with(|q| q.electron_temperature_kev[3] = 0.1989),
            Err(SliceReject::ElectronTemperatureOutOfRange { shell: 3, .. })
        ));
        assert!(matches!(
            with(|q| q.ion_temperature_kev[3] = 0.2239),
            Err(SliceReject::IonTemperatureOutOfRange { shell: 3, .. })
        ));
        assert!(matches!(
            with(|q| q.ion_temperature_kev[0] = 41.81),
            Err(SliceReject::IonTemperatureOutOfRange { shell: 0, .. })
        ));
        assert!(matches!(
            with(|q| q.electron_density_per_m3[2] = 4.18e18),
            Err(SliceReject::ElectronDensityOutOfRange { shell: 2, .. })
        ));
        assert!(matches!(with(|q| q.impurity_charge = 6.51), Err(SliceReject::ImpurityChargeOutOfRange { .. })));
        assert!(matches!(with(|q| q.impurity_mass_u = 9.99), Err(SliceReject::ImpurityMassOutOfRange { .. })));
        // Each temperature inside its range, the two together outside the span compared.
        assert!(matches!(
            with(|q| {
                q.electron_temperature_kev[1] = 12.0;
                q.ion_temperature_kev[1] = 5.99;
            }),
            Err(SliceReject::TemperatureRatioOutOfRange { shell: 1, .. })
        ));
        assert!(matches!(
            with(|q| {
                q.electron_temperature_kev[2] = 1.0;
                q.ion_temperature_kev[2] = 14.0;
            }),
            Err(SliceReject::TemperatureRatioOutOfRange { shell: 2, .. })
        ));
        assert!(matches!(
            with(|q| q.electron_density_per_m3[2] = 1.211e20),
            Err(SliceReject::ElectronDensityOutOfRange { shell: 2, .. })
        ));
        assert!(matches!(
            with(|q| q.tritium_density_per_m3[2] = -1.0),
            Err(SliceReject::IonDensityNegative { shell: 2, .. })
        ));
        // No tritium, or no deuterium, in one shell: outside the fuel mix compared.
        assert!(matches!(
            with(|q| q.tritium_density_per_m3[2] = 0.0),
            Err(SliceReject::TritiumShareOutOfRange { shell: 2, value: 0.0 })
        ));
        assert!(matches!(
            with(|q| q.deuterium_density_per_m3[1] = 0.0),
            Err(SliceReject::TritiumShareOutOfRange { shell: 1, value: 1.0 })
        ));
        assert!(matches!(
            with(|q| {
                q.deuterium_density_per_m3[1] = 0.0;
                q.tritium_density_per_m3[1] = 0.0;
            }),
            Err(SliceReject::TritiumShareOutOfRange { shell: 1, .. })
        ));
        assert!(matches!(with(|q| q.impurity_charge = 4.99), Err(SliceReject::ImpurityChargeOutOfRange { .. })));
        assert!(matches!(with(|q| q.impurity_mass_u = 13.01), Err(SliceReject::ImpurityMassOutOfRange { .. })));
        assert!(matches!(
            with(|q| q.deuterium_density_per_m3[1] = 1.0e20),
            Err(SliceReject::ThermalChargeExceedsElectrons { shell: 1, .. })
        ));
        assert!(matches!(
            with(|q| q.time_since_beam_change_s = -1.0e-9),
            Err(SliceReject::TimeSinceBeamChangeOutOfRange { .. })
        ));
        assert!(matches!(
            with(|q| q.time_since_beam_change_s = 100.00000000000001),
            Err(SliceReject::TimeSinceBeamChangeOutOfRange { .. })
        ));
        assert_eq!(with(|q| q.time_since_beam_change_s = f64::NAN), Err(SliceReject::NonFiniteValue));
        assert!(matches!(with(|q| q.beam_power_w[0] = -1.0), Err(SliceReject::BeamPowerOutOfRange { .. })));
        assert!(matches!(with(|q| q.beam_power_w = vec![0.0; 4]), Err(SliceReject::BeamPowerOutOfRange { .. })));
        assert!(matches!(with(|q| q.beam_power_w[0] = 4.6e7), Err(SliceReject::BeamPowerOutOfRange { .. })));
        assert!(matches!(
            with(|q| q.beam_mass_number[1] = 1.0),
            Err(SliceReject::BeamSpeciesUnknown { component: 1, .. })
        ));
        assert!(matches!(
            with(|q| q.beam_energy_kev[1] = 152.1),
            Err(SliceReject::BeamEnergyOutOfRange { component: 1, .. })
        ));
        assert!(matches!(
            with(|q| q.beam_energy_kev[0] = 25.9),
            Err(SliceReject::BeamEnergyOutOfRange { component: 0, .. })
        ));
        assert!(matches!(with(|q| q.beam_power_share[0] = 0.0), Err(SliceReject::BeamPowerShareOutOfRange { .. })));
        assert!(matches!(with(|q| q.beam_power_share[0] = 0.6), Err(SliceReject::BeamPowerShareOutOfRange { .. })));
        assert_eq!(with(|q| q.ion_temperature_kev[0] = f64::NAN), Err(SliceReject::NonFiniteValue));
        assert_eq!(with(|q| q.impurity_charge = f64::INFINITY), Err(SliceReject::NonFiniteValue));
    }

    /// The exact decimal ends of every range are inside it, and shares
    /// written as decimals that miss one in the last place are accepted.
    #[test]
    fn the_exact_ends_of_the_ranges_are_accepted() {
        let mut question = small();
        question.shell_volume_m3 = vec![SHELL_VOLUME_MAX_M3, SHELL_VOLUME_MIN_M3, 10.0, 10.0];
        question.electron_temperature_kev = vec![13.1, 0.199, 10.0, 0.73];
        question.ion_temperature_kev = vec![41.8, 0.224, 5.0, 10.0];
        // Electron over ion temperature: 0.313, 0.888, exactly 2.0 and exactly 0.073.
        assert_eq!(question.electron_temperature_kev[2] / question.ion_temperature_kev[2], TEMPERATURE_RATIO_MAX);
        assert_eq!(question.electron_temperature_kev[3] / question.ion_temperature_kev[3], TEMPERATURE_RATIO_MIN);
        question.electron_density_per_m3 = vec![1.21e20, 4.19e18, 5.0e19, 5.0e19];
        // Thermal ions at 0.97 of the electrons leave room for the fast ions on either side.
        question.deuterium_density_per_m3 = question.electron_density_per_m3.iter().map(|n| 0.485 * n).collect();
        question.tritium_density_per_m3 = question.deuterium_density_per_m3.clone();
        question.impurity_density_per_m3 = vec![0.0; 4];
        question.impurity_charge = 5.0;
        question.impurity_mass_u = 10.0;
        question.beam_mass_number = vec![3.0, 2.0, 2.0];
        question.beam_energy_kev = vec![152.0, 152.0, 26.0];
        question.beam_power_share = vec![0.6, 0.3, 0.1];
        // The whole is exactly the ceiling; none of it goes into the smallest shell.
        question.beam_power_w = vec![4.4e7, 0.0, 9.0e5, 1.0e5];
        question.time_since_beam_change_s = TIME_SINCE_BEAM_CHANGE_MAX_S;
        assert_eq!(question.beam_power_w.iter().sum::<f64>(), BEAM_POWER_MAX_W);
        assert_ne!(0.6 + 0.3 + 0.1, 1.0);
        assert_eq!(admit(&question), Ok(()));
        let answer = solve(&question).expect("solves");
        assert!(answer.dt_fusion_power_w > 0.0);
        assert!(answer.charge_excess_max <= QUASI_NEUTRALITY_FAST_ION_ALLOWANCE && answer.charge_excess_min >= -QUASI_NEUTRALITY_DEFICIT_ALLOWANCE);
        question.impurity_charge = 6.5;
        question.impurity_mass_u = 13.0;
        assert_eq!(admit(&question), Ok(()));
        // The two ends of the fuel mix compared, in shells 0 and 1.
        // 2^60 fuel ions per cubic metre in each shell, split at each end; the share the door forms is that end.
        for (shell, share) in [(0, TRITIUM_SHARE_MIN), (1, TRITIUM_SHARE_MAX)] {
            question.tritium_density_per_m3[shell] = share * 2.0_f64.powi(60);
            question.deuterium_density_per_m3[shell] = 2.0_f64.powi(60) - question.tritium_density_per_m3[shell];
            let formed = question.tritium_density_per_m3[shell]
                / (question.deuterium_density_per_m3[shell] + question.tritium_density_per_m3[shell]);
            assert_eq!(formed, share);
        }
        assert_eq!(admit(&question), Ok(()));
        // No time at all since the beam changed is inside the input's range;
        // whether it is answered is for the solve, which finds the energy time.
        question.time_since_beam_change_s = 0.0;
        assert_eq!(admit(&question), Ok(()));
        assert!(matches!(solve(&question), Err(SliceReject::BeamChangeTooRecent { .. })));
        // A beam born below the stopping energy holds no fast ions: its energy time is zero, and zero seconds is enough.
        let mut cold = cold_beam();
        cold.time_since_beam_change_s = 0.0;
        let cold = solve(&cold).expect("solves");
        assert_eq!(cold.fast_ion_energy_time_s, 0.0);
    }

    /// The steady-state limit: the caller's time since the beam power
    /// changed against five energy times of this solve, at the exact end,
    /// one step inside it, and far inside.
    #[test]
    fn a_slice_too_soon_after_a_beam_change_is_named() {
        let settled = solve(&small()).expect("solves");
        let energy_time = settled.fast_ion_energy_time_s;
        let deposited: f64 = small().beam_power_w.iter().sum();
        assert_eq!(energy_time.to_bits(), (settled.fast_ion_energy_j / deposited).to_bits());
        assert!(energy_time > 0.05 && energy_time < 0.2, "{energy_time}");
        let end = SETTLED_AFTER_ENERGY_TIMES * energy_time;
        // Exactly five energy times is answered, with the same bits as the settled slice.
        let mut question = small();
        question.time_since_beam_change_s = end;
        let at_the_end = solve(&question).expect("the exact end is answered");
        assert_eq!(at_the_end, settled);
        // One step of a 64-bit number sooner is refused, and the refusal carries both times.
        let sooner = f64::from_bits(end.to_bits() - 1);
        question.time_since_beam_change_s = sooner;
        assert_eq!(
            solve(&question),
            Err(SliceReject::BeamChangeTooRecent { time_since_beam_change_s: sooner, fast_ion_energy_time_s: energy_time })
        );
        question.time_since_beam_change_s = 0.0;
        assert!(matches!(solve(&question), Err(SliceReject::BeamChangeTooRecent { .. })));
        // The time changes nothing else: any settled time gives the same answer.
        question.time_since_beam_change_s = TIME_SINCE_BEAM_CHANGE_MAX_S;
        assert_eq!(solve(&question).expect("solves"), settled);
        // The energy time does not depend on how much power the beams carry.
        let mut halved = small();
        for power in &mut halved.beam_power_w {
            *power *= 0.5;
        }
        let halved = solve(&halved).expect("solves");
        assert!((halved.fast_ion_energy_time_s / energy_time - 1.0).abs() < 1.0e-12);
    }

    /// One shell of [`small`], with deuterium beams only so that beam-beam
    /// reactions are none, thermal deuterium and tritium in equal parts
    /// carrying `thermal` of the electrons' charge, and the beam power scaled
    /// by `power`.
    fn one_shell(thermal: f64, power: f64) -> SliceQuestion {
        let full = small();
        let electrons = full.electron_density_per_m3[1];
        SliceQuestion {
            shell_volume_m3: vec![full.shell_volume_m3[1]],
            electron_temperature_kev: vec![full.electron_temperature_kev[1]],
            ion_temperature_kev: vec![full.ion_temperature_kev[1]],
            electron_density_per_m3: vec![electrons],
            deuterium_density_per_m3: vec![0.5 * thermal * electrons],
            tritium_density_per_m3: vec![0.5 * thermal * electrons],
            impurity_density_per_m3: vec![0.0],
            beam_power_w: vec![full.beam_power_w[1] * power],
            beam_mass_number: vec![2.0, 2.0],
            ..full
        }
    }

    /// The fast-ion share a solve finds in a one-shell question, answered or refused.
    fn share_of(question: &SliceQuestion) -> f64 {
        let thermal = (question.deuterium_density_per_m3[0] + question.tritium_density_per_m3[0]) / question.electron_density_per_m3[0];
        match solve(question) {
            Ok(answer) => answer.fast_ion_fraction_max,
            Err(SliceReject::FastIonShareTooLarge { share, .. }) => share,
            Err(SliceReject::IonChargeExceedsElectrons { excess, .. }) => 1.0 + excess - thermal,
            Err(SliceReject::IonChargeBelowElectrons { deficit, .. }) => 1.0 - deficit - thermal,
            Err(other) => panic!("{other:?}"),
        }
    }

    /// The limit of the test-particle picture: fast ions above the largest
    /// share compared. The thermal ions are set at 0.565 of the electrons, so
    /// that the charge balances with the fast ions near the limit.
    #[test]
    fn fast_ions_above_the_largest_share_compared_are_named() {
        assert_eq!(FAST_ION_SHARE_MAX, 0.439);
        let found = share_of(&one_shell(0.565, 1.0));
        // The share follows the beam power in proportion.
        let below = solve(&one_shell(0.565, 0.438 / found)).expect("just below the limit is answered");
        assert!(below.fast_ion_fraction_max > 0.4379 && below.fast_ion_fraction_max <= FAST_ION_SHARE_MAX);
        assert!(below.charge_excess_max.abs() < 0.01, "{}", below.charge_excess_max);
        match solve(&one_shell(0.565, 0.440 / found)) {
            Err(SliceReject::FastIonShareTooLarge { shell, share }) => {
                assert!(share > FAST_ION_SHARE_MAX && share < 0.4401, "{share}");
                assert_eq!(shell, 0);
            }
            other => panic!("accepted fast ions above the largest share compared: {other:?}"),
        }
    }

    /// The fast ions are counted in the charge balance, and it is held both
    /// ways at the extremes compared: thermal ions that balance the electrons
    /// alone, with fast ions over the allowance on top, are refused; thermal
    /// ions under the allowance are answered only where the fast ions make up
    /// the charge.
    #[test]
    fn the_fast_ions_are_counted_in_the_charge_balance_both_ways() {
        assert_eq!(QUASI_NEUTRALITY_FAST_ION_ALLOWANCE, 0.12);
        assert_eq!(QUASI_NEUTRALITY_DEFICIT_ALLOWANCE, 0.215);
        let unit = share_of(&one_shell(1.0, 1.0));
        // Over: 11 percent of fast ions on neutral thermal ions is answered, 13 percent is refused.
        let inside = solve(&one_shell(1.0, 0.11 / unit)).expect("answered");
        assert!((inside.charge_excess_max - 0.11).abs() < 1.0e-4, "{}", inside.charge_excess_max);
        assert_eq!(inside.charge_excess_max, inside.charge_excess_min);
        match solve(&one_shell(1.0, 0.13 / unit)) {
            Err(SliceReject::IonChargeExceedsElectrons { shell: 0, excess }) => assert!((excess - 0.13).abs() < 1.0e-3, "{excess}"),
            other => panic!("accepted fast ions over the electrons: {other:?}"),
        }
        // Under: thermal ions at 0.77 of the electrons are answered with 3 percent of fast ions and refused with 0.5 percent.
        let unit = share_of(&one_shell(0.77, 1.0));
        let made_up = solve(&one_shell(0.77, 0.03 / unit)).expect("answered");
        assert!((made_up.charge_excess_min + 0.20).abs() < 1.0e-3, "{}", made_up.charge_excess_min);
        match solve(&one_shell(0.77, 0.005 / unit)) {
            Err(SliceReject::IonChargeBelowElectrons { shell: 0, deficit }) => assert!((deficit - 0.225).abs() < 1.0e-3, "{deficit}"),
            other => panic!("accepted ions short of the electrons: {other:?}"),
        }
        // A shell of electrons with no ions, given 4 percent of their charge in fuel so that its fuel mix is one
        // compared: inside every range of the door, it is refused after the solve.
        let mut bare = one_shell(0.0, 1.0);
        bare.deuterium_density_per_m3 = vec![1.0e18];
        bare.tritium_density_per_m3 = vec![1.0e18];
        bare.electron_temperature_kev = vec![5.0];
        bare.ion_temperature_kev = vec![5.0];
        bare.electron_density_per_m3 = vec![5.0e19];
        bare.shell_volume_m3 = vec![10.0];
        bare.beam_power_w = vec![1.0e5];
        assert_eq!(admit(&bare), Ok(()));
        match solve(&bare) {
            Err(SliceReject::IonChargeBelowElectrons { shell: 0, deficit }) => assert!(deficit > 0.9, "{deficit}"),
            other => panic!("accepted a shell with no ions: {other:?}"),
        }
        // Where the fast ions are the larger share, an excess above the largest compared is refused.
        let unit = share_of(&one_shell(0.97, 1.0));
        assert!(matches!(solve(&one_shell(0.97, 0.16 / unit)), Err(SliceReject::IonChargeExceedsElectrons { .. })));
    }

    /// Questions that 0.6.0 answered outside anything compared are refused
    /// by name.
    #[test]
    fn questions_outside_anything_compared_are_refused_by_name() {
        let base = || SliceQuestion {
            shell_volume_m3: vec![10.0],
            electron_temperature_kev: vec![5.0],
            ion_temperature_kev: vec![5.0],
            electron_density_per_m3: vec![5.0e19],
            deuterium_density_per_m3: vec![2.0e19],
            tritium_density_per_m3: vec![2.0e19],
            impurity_density_per_m3: vec![1.0e18],
            impurity_charge: 6.0,
            impurity_mass_u: 12.0,
            beam_power_w: vec![1.0e6],
            beam_mass_number: vec![2.0],
            beam_energy_kev: vec![80.0],
            beam_power_share: vec![1.0],
            time_since_beam_change_s: 100.0,
        };
        // A plasma of 1e18 electrons per cubic metre.
        let mut thin = base();
        thin.electron_density_per_m3 = vec![1.0e18];
        thin.deuterium_density_per_m3 = vec![5.0e17];
        thin.tritium_density_per_m3 = vec![5.0e17];
        thin.impurity_density_per_m3 = vec![0.0];
        assert!(matches!(solve(&thin), Err(SliceReject::ElectronDensityOutOfRange { .. })));
        // Neon, and an impurity of mass 6 or 22.
        let mut neon = base();
        neon.impurity_charge = 10.0;
        neon.impurity_density_per_m3 = vec![6.0e17];
        assert!(matches!(solve(&neon), Err(SliceReject::ImpurityChargeOutOfRange { .. })));
        for mass in [6.0, 22.0] {
            let mut odd = base();
            odd.impurity_mass_u = mass;
            assert!(matches!(solve(&odd), Err(SliceReject::ImpurityMassOutOfRange { .. })));
        }
        // Te/Ti of 100, Te of 20 keV, Ti of 50 keV.
        let mut hot = base();
        hot.electron_temperature_kev = vec![20.0];
        hot.ion_temperature_kev = vec![0.2];
        assert!(matches!(solve(&hot), Err(SliceReject::ElectronTemperatureOutOfRange { .. })));
        let mut ratio = base();
        ratio.electron_temperature_kev = vec![12.0];
        ratio.ion_temperature_kev = vec![1.0];
        assert!(matches!(solve(&ratio), Err(SliceReject::TemperatureRatioOutOfRange { .. })));
        let mut ions = base();
        ions.ion_temperature_kev = vec![50.0];
        assert!(matches!(solve(&ions), Err(SliceReject::IonTemperatureOutOfRange { .. })));
        // 2e20 electrons, a 5 keV beam.
        let mut dense = base();
        dense.electron_density_per_m3 = vec![2.0e20];
        dense.deuterium_density_per_m3 = vec![9.0e19];
        dense.tritium_density_per_m3 = vec![9.0e19];
        dense.impurity_density_per_m3 = vec![2.0e18];
        assert!(matches!(solve(&dense), Err(SliceReject::ElectronDensityOutOfRange { .. })));
        let mut slow = base();
        slow.beam_energy_kev = vec![5.0];
        assert!(matches!(solve(&slow), Err(SliceReject::BeamEnergyOutOfRange { .. })));
        // A tritium-free plasma, never compared.
        let mut free = base();
        free.deuterium_density_per_m3 = vec![4.0e19];
        free.tritium_density_per_m3 = vec![0.0];
        assert!(matches!(solve(&free), Err(SliceReject::TritiumShareOutOfRange { shell: 0, value: 0.0 })));
        // The smallest volume above zero.
        let mut tiny = base();
        tiny.shell_volume_m3 = vec![5e-324];
        assert!(matches!(solve(&tiny), Err(SliceReject::ShellVolumeOutOfRange { shell: 0, .. })));
        // The base itself is answered: thermal ions at 0.92 of the electrons and the fast ions above.
        let answer = solve(&base()).expect("solves");
        assert!(answer.charge_excess_min >= -QUASI_NEUTRALITY_DEFICIT_ALLOWANCE, "{}", answer.charge_excess_min);
    }

    /// Quasi-neutrality at its end. Densities a caller writes as decimals
    /// whose thermal charge is exactly 1.025 of the electrons are answered
    /// whatever the rounding, and the largest distance met is printed; the
    /// end itself is exact where the electron density is a power of two.
    #[test]
    fn an_excess_of_exactly_the_allowance_is_answered() {
        assert_eq!(QUASI_NEUTRALITY_ALLOWANCE, 0.025);
        let end = 1.0 + QUASI_NEUTRALITY_ALLOWANCE;
        // Deuterium, tritium and impurity charge as ten-thousandths of the electron density: each row adds to 10250.
        let splits: [(u64, u64, u64, f64); 5] =
            [(5125, 5125, 0, 6.0), (10000, 250, 0, 6.0), (4050, 5000, 200, 6.0), (4000, 5000, 250, 5.0), (3125, 4000, 625, 5.0)];
        let mut farthest: i64 = 0;
        let mut asked = 0;
        for mantissa in (419..=12100).step_by(3) {
            for (deuterium, tritium, impurity, charge) in splits {
                assert_eq!(deuterium as f64 + tritium as f64 + charge * impurity as f64, 10250.0);
                // mantissa x 10^16 electrons per cubic metre: 4.19e18 to 1.21e20.
                let decimal = |parts: u64| format!("{}e12", mantissa * parts).parse::<f64>().unwrap();
                let electrons = format!("{mantissa}e16").parse::<f64>().unwrap();
                let mut question = small();
                question.electron_density_per_m3 = vec![electrons; 4];
                question.deuterium_density_per_m3 = vec![decimal(deuterium); 4];
                question.tritium_density_per_m3 = vec![decimal(tritium); 4];
                question.impurity_density_per_m3 = vec![decimal(impurity); 4];
                question.impurity_charge = charge;
                question.impurity_mass_u = 2.0 * charge;
                let ratio = thermal_charge(&question, 0) / electrons;
                farthest = farthest.max((ratio.to_bits() as i64 - end.to_bits() as i64).abs());
                assert_eq!(admit(&question), Ok(()), "{mantissa} {deuterium} {tritium} {impurity}: {ratio}");
                asked += 1;
            }
        }
        println!("{asked} questions at exactly 1.025; the computed ratio is at most {farthest} units in the last place from the end");
        assert!(farthest < QUASI_NEUTRALITY_END_ALLOWANCE_ULP as i64, "{farthest}");
        // With 2^65 electrons per cubic metre (3.7e19) the ratio is formed without rounding: the allowance is answered
        // to its last unit, one unit beyond is refused by name, and the answer at the end is a solve of the
        // question's own densities. The beam goes into the first shell alone, which is held neutral with few fast
        // ions, so that the other three, which hold no fast ions, are at the end after the solve too.
        let electrons = 2.0_f64.powi(65);
        let with_ratio = |ratio: f64| {
            let mut question = small();
            question.electron_density_per_m3 = vec![electrons; 4];
            question.deuterium_density_per_m3 = vec![0.5 * ratio * electrons; 4];
            question.tritium_density_per_m3 = vec![0.5 * ratio * electrons; 4];
            question.deuterium_density_per_m3[0] = 0.5 * electrons;
            question.tritium_density_per_m3[0] = 0.5 * electrons;
            question.impurity_density_per_m3 = vec![0.0; 4];
            question.beam_power_w = vec![0.02 * question.beam_power_w[0], 0.0, 0.0, 0.0];
            question
        };
        let highest = THERMAL_CHARGE_OVER_ELECTRONS_HIGHEST_ANSWERED;
        assert_eq!(highest.to_bits() - end.to_bits(), QUASI_NEUTRALITY_END_ALLOWANCE_ULP as u64);
        assert_eq!(admit(&with_ratio(highest)), Ok(()));
        let beyond = f64::from_bits(highest.to_bits() + 1);
        assert_eq!(
            admit(&with_ratio(beyond)),
            Err(SliceReject::ThermalChargeExceedsElectrons { shell: 1, excess: beyond - 1.0 })
        );
        let at_the_end = solve(&with_ratio(end)).expect("solves");
        let highest_solved = solve(&with_ratio(highest)).expect("the last unit is answered after the solve too");
        assert!(highest_solved.charge_excess_max <= QUASI_NEUTRALITY_ALLOWANCE + 1.0e-15);
        assert!(matches!(solve(&with_ratio(beyond)), Err(SliceReject::ThermalChargeExceedsElectrons { shell: 1, .. })));
        let neutral = solve(&with_ratio(1.0)).expect("solves");
        // 2.5 percent more of both fuels in the last three shells: 1.025 squared in their thermal rate, since
        // nothing was trimmed.
        let shell_rate = |shell: usize| {
            let question = small();
            0.25 * electrons * electrons
                * bosch_hale::reactivity_m3_per_s(Reaction::DeuteriumTritium, question.ion_temperature_kev[shell]).unwrap()
                * question.shell_volume_m3[shell]
        };
        let expected = (shell_rate(0) + end * end * (shell_rate(1) + shell_rate(2) + shell_rate(3)))
            / (shell_rate(0) + shell_rate(1) + shell_rate(2) + shell_rate(3));
        assert!((at_the_end.thermal_dt_rate_per_s / neutral.thermal_dt_rate_per_s / expected - 1.0).abs() < 1.0e-12);
        assert!(matches!(
            admit(&with_ratio(1.026)),
            Err(SliceReject::ThermalChargeExceedsElectrons { shell: 1, .. })
        ));
    }

    /// Quasi-neutrality at its lower end, as at its upper: densities a
    /// caller writes as decimals whose charge is exactly 0.785 of the
    /// electrons, in a shell without fast ions, are inside whatever the
    /// rounding; one unit below the lowest answered is refused by name after
    /// the solve.
    #[test]
    fn a_shortfall_of_exactly_the_allowance_is_answered() {
        assert_eq!(QUASI_NEUTRALITY_DEFICIT_ALLOWANCE, 0.215);
        let end = 1.0 - QUASI_NEUTRALITY_DEFICIT_ALLOWANCE;
        // Deuterium, tritium and impurity charge as ten-thousandths of the electron density: each row adds to 7850.
        let splits: [(u64, u64, u64, f64); 5] =
            [(3925, 3925, 0, 6.0), (5850, 2000, 0, 6.0), (3500, 3750, 100, 6.0), (3500, 3850, 100, 5.0), (3125, 3000, 345, 5.0)];
        let mut farthest: i64 = 0;
        let mut asked = 0;
        for mantissa in (419..=12100).step_by(3) {
            for (deuterium, tritium, impurity, charge) in splits {
                assert_eq!(deuterium as f64 + tritium as f64 + charge * impurity as f64, 7850.0);
                let decimal = |parts: u64| format!("{}e12", mantissa * parts).parse::<f64>().unwrap();
                let electrons = format!("{mantissa}e16").parse::<f64>().unwrap();
                let mut question = small();
                question.electron_density_per_m3 = vec![electrons; 4];
                question.deuterium_density_per_m3 = vec![decimal(deuterium); 4];
                question.tritium_density_per_m3 = vec![decimal(tritium); 4];
                question.impurity_density_per_m3 = vec![decimal(impurity); 4];
                question.impurity_charge = charge;
                question.impurity_mass_u = 2.0 * charge;
                // The ratio the solve forms for a shell without fast ions.
                let ratio = thermal_charge(&question, 0) / electrons;
                farthest = farthest.max((ratio.to_bits() as i64 - end.to_bits() as i64).abs());
                assert!(ratio >= ION_CHARGE_OVER_ELECTRONS_LOWEST_ANSWERED, "{mantissa} {deuterium} {tritium} {impurity}: {ratio}");
                asked += 1;
            }
        }
        println!("{asked} questions at exactly 0.785; the computed ratio is at most {farthest} units in the last place from the end");
        assert!(farthest < QUASI_NEUTRALITY_DEFICIT_END_ALLOWANCE_ULP as i64, "{farthest}");
        // With 2^65 electrons the ratio is exact. The beam goes into the first shell alone, held neutral.
        let electrons = 2.0_f64.powi(65);
        let with_ratio = |ratio: f64| {
            let mut question = small();
            question.electron_density_per_m3 = vec![electrons; 4];
            question.deuterium_density_per_m3 = vec![0.5 * ratio * electrons; 4];
            question.tritium_density_per_m3 = vec![0.5 * ratio * electrons; 4];
            question.deuterium_density_per_m3[0] = 0.5 * electrons;
            question.tritium_density_per_m3[0] = 0.5 * electrons;
            question.impurity_density_per_m3 = vec![0.0; 4];
            question.beam_power_w = vec![0.02 * question.beam_power_w[0], 0.0, 0.0, 0.0];
            question
        };
        let lowest = ION_CHARGE_OVER_ELECTRONS_LOWEST_ANSWERED;
        assert_eq!(end.to_bits() - lowest.to_bits(), QUASI_NEUTRALITY_DEFICIT_END_ALLOWANCE_ULP as u64);
        let answer = solve(&with_ratio(lowest)).expect("the lowest answered is answered");
        assert_eq!(answer.charge_excess_min, lowest - 1.0);
        solve(&with_ratio(end)).expect("exactly the allowance is answered");
        let below = f64::from_bits(lowest.to_bits() - 1);
        assert_eq!(admit(&with_ratio(below)), Ok(()));
        assert_eq!(
            solve(&with_ratio(below)),
            Err(SliceReject::IonChargeBelowElectrons { shell: 1, deficit: 1.0 - below })
        );
    }

    /// Attack: a refinement stopped before the shares have settled is named.
    #[test]
    fn a_refinement_stopped_early_is_named() {
        assert_eq!(solve_with(&small(), 1, Fault::None), Err(SliceReject::NotSettled { refinements: 1 }));
    }

    /// Attack: impurity drag that slows the ion and is credited to no one leaves energy unaccounted for.
    #[test]
    fn energy_given_to_no_one_is_named() {
        match solve_with(&small(), MAX_REFINEMENTS, Fault::ImpurityDragNotCredited) {
            Err(SliceReject::EnergyLedgerBroken { residual }) => assert!(residual > 1.0e-3, "{residual}"),
            other => panic!("accepted energy given to no one: {:?}", other.is_ok()),
        }
    }

    /// Attack: fused ions counted twice no longer add to one with the slowed.
    #[test]
    fn ions_counted_twice_are_named() {
        match solve_with(&small(), MAX_REFINEMENTS, Fault::FusedCountedTwice) {
            Err(SliceReject::ParticleLedgerBroken { residual }) => assert!(residual.abs() > 1.0e-4, "{residual}"),
            other => panic!("accepted ions counted twice: {:?}", other.is_ok()),
        }
    }

    /// Attack: a falsified answer of one component is named by the ledger it breaks.
    #[test]
    fn a_falsified_component_is_named() {
        let question = small();
        let shell = shell_at(&question, 0);
        let good = slow_and_fuse(DEUTERON_MASS_KG, 80.0, &shell, 0, 8.0, Fault::None).expect("solves");
        assert!(hold(&good).is_ok());
        let mut more = good.clone();
        more.particle_residual = 1.0e-9;
        assert!(matches!(hold(&more), Err(SliceReject::ParticleLedgerBroken { .. })));
        let mut lost = good.clone();
        lost.energy_residual = -1.0e-4;
        assert!(matches!(hold(&lost), Err(SliceReject::EnergyLedgerBroken { .. })));
        let mut broken = good;
        broken.fused[0] = f64::NAN;
        assert_eq!(hold(&broken).err(), Some(SliceReject::NonFiniteValue));
    }

    /// Attack: beams so strong in so thin a plasma that beam-beam reactions,
    /// which the ledgers do not count, take more than their bound.
    #[test]
    fn beam_beam_reactions_beyond_their_bound_are_named() {
        let mut question = small();
        question.shell_volume_m3 = vec![0.05; 4];
        question.beam_power_w = vec![1.1e7; 4];
        match solve(&question) {
            Err(SliceReject::BeamBeamShareTooLarge { share }) => assert!(share > BEAM_BEAM_SHARE_MAX),
            other => panic!("accepted: {other:?}"),
        }
    }

    /// Electrons twice as hot as the ions, every temperature inside its
    /// range, and next to no thermal ions to slow the beam: below 1.5 electron
    /// temperatures the electrons heat a beam ion, and it never stops. It is
    /// named before the charge balance, which this shell also breaks.
    #[test]
    fn a_beam_ion_that_gains_energy_is_named() {
        let mut question = small();
        question.electron_temperature_kev = vec![12.0; 4];
        question.ion_temperature_kev = vec![6.0; 4];
        question.electron_density_per_m3 = vec![5.0e19; 4];
        question.deuterium_density_per_m3 = vec![1.0e10; 4];
        question.tritium_density_per_m3 = vec![1.0e10; 4];
        question.impurity_density_per_m3 = vec![0.0; 4];
        question.beam_energy_kev = vec![26.0, 26.0];
        assert_eq!(admit(&question), Ok(()));
        assert!(matches!(solve(&question), Err(SliceReject::SlowingRateNotPositive { shell: 0 })));
    }

    /// The table of `G` against the integral it stands for, taken directly.
    #[test]
    fn the_beam_beam_table_is_the_integral() {
        let table = BeamBeamTable::new();
        let reduced = DEUTERON_MASS_KG * TRITON_MASS_KG / (DEUTERON_MASS_KG + TRITON_MASS_KG);
        for (deuteron_kev, triton_kev) in [(80.0, 150.0), (40.0, 30.0), (160.0, 160.0), (30.0, 30.0)] {
            let v_d = (2.0 * deuteron_kev * JOULE_PER_KEV / DEUTERON_MASS_KG).sqrt();
            let v_t = (2.0 * triton_kev * JOULE_PER_KEV / TRITON_MASS_KG).sqrt();
            let direct = reaction::integrate((v_d - v_t).abs(), v_d + v_t, 4000, |u| {
                bosch_hale::cross_section_or_zero(Reaction::DeuteriumTritium, 0.5 * reduced * u * u / JOULE_PER_KEV) * u * u
            }) * 1.0e-28
                / (2.0 * v_d * v_t);
            let off = table.kernel_m3_per_s(v_d, v_t) / direct - 1.0;
            assert!(off.abs() < 1.0e-8, "{deuteron_kev} {triton_kev}: {off}");
        }
    }

    /// A finer first panel moves the answer by less than the tolerance asks.
    #[test]
    fn the_settled_answer_does_not_depend_on_the_panels() {
        let question = small();
        let table = BeamBeamTable::new();
        let births = [1.0e19, 1.0e19];
        let (settled, beam_beam, taken) = settle_shell(&question, 1, &births, &table, MAX_REFINEMENTS, Fault::None).expect("settles");
        let (finer, finer_beam_beam) =
            shell_once(&question, 1, &births, &table, FIRST_PANEL_KEV / 2.0_f64.powi(taken as i32 + 1), Fault::None).expect("solves");
        for (now, fine) in settled.iter().zip(&finer) {
            assert!((fine.fused[0] / now.fused[0] - 1.0).abs() < SETTLE_TOLERANCE);
        }
        assert!((finer_beam_beam / beam_beam - 1.0).abs() < SETTLE_TOLERANCE);
    }
}
