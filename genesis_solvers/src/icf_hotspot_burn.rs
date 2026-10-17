//! The burn of a stagnated deuterium-tritium hot spot of inertial
//! confinement fusion: a hot, isobaric hot spot inside a dense shell of
//! cold fuel, from peak compression through ignition to disassembly.
//!
//! This is the company's own derivation, written for the indirect-drive
//! implosions of the National Ignition Facility (NIF) at Lawrence Livermore
//! National Laboratory, and compared with the stagnation parameters and
//! yields LLNL has published for them (the comparison is in
//! `evidence/research/icf_nif_hotspot/`). The solver stores no value of any
//! shot.
//!
//! # The question
//!
//! The caller gives the state of peak compression of the implosion as it
//! would be under ideal hydrodynamics (no burn, no alpha heating, no heat
//! conduction, no radiation): the hot-spot ion temperature `T₀`, its
//! pressure `p₀`, its radius `R₀`, the areal density of the dense fuel
//! around it `(ρR)_s`, and the whole deuterium-tritium fuel mass `M`. At
//! that instant everything is at rest and the shell is at the hot-spot
//! pressure. The solver returns the burn: the yield, the burn-up, the burn
//! history, the neutron-averaged hot-spot state, the energy ledger and the
//! ignition and self-heating measures of the field.
//!
//! # The model
//!
//! Two parts, the field's 0-D hot spot and a 1-D shell.
//!
//! **The hot spot** is one uniform zone of radius `a`, deuterium and tritium
//! in equal numbers `N_f/2` each, alpha ash `N_α`, and electrons
//! `N_e = N_f + 2 N_α`, all at one temperature `T` (ions and electrons
//! together: one temperature). Its thermal energy is `E_h`:
//!
//! ```text
//! E_h = (3/2) (N_f + N_α + N_e) T          p = (2/3) E_h / V          V = (4π/3) a³
//! ```
//!
//! **The shell** is the cold fuel, a spherical shell from `a` outward, cut
//! into [`SHELL_ZONES`] zones of equal mass and followed by one-dimensional
//! Lagrangian hydrodynamics: node radii `r_j` and speeds `u_j`, zone masses
//! `m_i` and internal energies `e_i`, pressure `P_i = (2/3) e_i / V_i + q_i`.
//! `p = (2/3) ρ ε` is the equation of state of a classical ideal gas and of
//! a degenerate electron gas alike, so the shell's adiabat `p ∝ ρ^(5/3)` holds
//! whether or not its electrons are degenerate. `q_i` is von Neumann and
//! Richtmyer's artificial viscosity, quadratic only
//! ([`VISCOSITY_QUADRATIC`]), nonzero only where a zone is compressed (its
//! nodes closing and its volume falling), so that it only ever heats. The outer
//! surface is free (pressure zero), so a rarefaction runs in from it and the
//! shell disassembles on its own sound-crossing time.
//!
//! ```text
//! node 0 (the hot-spot boundary):  (m_1/2 + (3/5) M_h) du_0/dt = 4π a² (p − P_1)
//! node j:                          ((m_j + m_{j+1})/2) du_j/dt = 4π r_j² (P_j − P_{j+1})
//! outer node N:                    (m_N/2) du_N/dt = 4π r_N² P_N
//! zone i:                          de_i/dt = − P_i dV_i/dt
//! hot spot:                        dE_h/dt = P_α,h − P_br − 4π a² p u_0 + (energy brought in by ablated fuel)
//! ```
//!
//! The hot spot's own inertia is that of homologous flow, kinetic energy
//! `(3/10) M_h u_0²`, which adds `(3/5) M_h` to the boundary node. Once the
//! last shell zone has been ablated into the hot spot ("burn-through") the
//! hot spot is the whole fuel and expands freely:
//! `(3/5) M_h du_0/dt = 4π a² p`.
//!
//! **Fusion.** `dN_α/dt = (N_f/2V)² ⟨σv⟩(T) V` reactions per second, with the
//! D-T reactivity of Bosch and Hale (1992), [`crate::bosch_hale`], defined
//! from 0.2 to 100 keV. Each reaction releases 17.589 MeV: a neutron, which
//! leaves, and an alpha particle of
//!
//! ```text
//! E_α = Q (Q + 2 m_n c²) / (2 (m_n c² + m_α c² + Q)) = 3560.80 keV          E_n = Q − E_α = 14028.20 keV
//! ```
//!
//! (two-body kinematics at rest, CODATA 2018 rest energies).
//!
//! **Alpha deposition.** A share `f_α` of the alpha energy is deposited in
//! the hot spot directly, by the fraction of Krokhin and Rozanov (Sov. J.
//! Quantum Electron. 2, 393 (1973)) for alphas born uniformly in a uniform
//! sphere:
//!
//! ```text
//! f_α(τ) = 3τ/2 − 4τ²/5                τ ≤ 1/2          τ = a / ℓ_α
//! f_α(τ) = 1 − 1/(4τ) + 1/(160 τ³)    τ ≥ 1/2
//! ```
//!
//! The 1/(4τ) of the large-τ form is the escaping share of alphas whose
//! energy falls along a straight path as `E(x) = E_α (1 − x/ℓ)²`, the law of
//! a drag proportional to speed (derivation: a slab of depth `x < ℓ`, half
//! the directions outward, the energy carried out `∫∫ (1 − s/μ)² dμ ds/2 =
//! 1/12` per unit of `ℓ`, times the surface share `3ℓ/a` of the sphere).
//! To first order in `1/τ` the escaping share for any law of energy loss is
//! `(3/4a)` times the mean depth at which the alpha's energy is deposited
//! (for the law above, `ℓ/3`), so the length `ℓ_α` is taken as three times
//! that mean depth for the alpha's own slowing in this plasma, which makes
//! the fraction right to that order: `3 ∫ x dE / (E_α − 3T/2)` along the
//! path, from birth down to `3T/2`, with the
//! test-particle energy-loss rates on electrons, deuterons, tritons and ash
//! of [`crate::pb11_reaction::Background`] (the NRL Plasma Formulary rates
//! and Coulomb logarithms). `f_α` is evaluated once at the start of each
//! step.
//!
//! The rest of the alpha energy enters the shell. The shell stops every
//! alpha it receives, and that energy, like the heat conducted out of the
//! hot spot, heats the inner face of the shell and returns as hot-spot
//! fuel: the ablation of Betti et al. (Phys. Plasmas 9, 2277 (2002)).
//! Once the shell is gone, the alpha energy not deposited in the hot spot
//! leaves the fuel.
//!
//! **Ablation.** Heat conducted from the hot spot does not leave it: it
//! heats the cold face of the shell, and the heated fuel joins the hot spot.
//! For the isobaric hot spot of Betti et al., whose temperature falls to
//! the shell as `T(r) = T_c (1 − r̂²)^(2/5) / (1 − 0.15 r̂²)`, `r̂ = r/a`
//! (LLE Review 85 (2000), equation 41, the model of Phys. Plasmas 8, 5257
//! (2001) and 9, 2277 (2002)), energy flowing with the mass gives
//! `4π r² ρ |v| = 4π r² κ |dT/dr| m̄/(5T)` through every radius (enthalpy
//! `5T` per ion and electron pair at constant pressure, `ρ/p = m̄/2T`), and
//! at `r = a`, where `T^(5/2) → T_c^(5/2) (1 − r̂²)/0.85^(5/2)`, with
//! Spitzer's conductivity `κ = κ₀ T^(5/2) / lnΛ`:
//!
//! ```text
//! ṁ_cond = (16π/25) / 0.85^(5/2) · m̄ a κ₀ T^(5/2) / lnΛ        ṁ_α = m̄ (1 − f_α) P_α / (5 T)
//! κ₀ = 3.1616 · 3 (4π ε₀)² / (4 √(2π) √m_e e⁴)                  Braginskii's electron conductivity, charge 1
//! ```
//!
//! taking the hot spot's one temperature as `T_c`. Over the shell's
//! density this is an ablation speed of `6.0e3 T^(5/2) / (R ρ_shell lnΛ)`
//! µm/ns (T in keV, R in µm, ρ in g/cm³), Betti's own equation 32 of that
//! review (a test holds it to 2 %). The ablated mass leaves the innermost
//! shell zone at that zone's specific internal energy and joins the hot
//! spot; the kinetic energy of the node masses that changes with it goes
//! into the hot spot's heat, so the ledger closes exactly. The heat
//! conducted is not a loss of the hot spot's energy: it lowers the hot
//! spot's temperature by adding mass at nearly constant pressure. It is
//! reported, as `5 T ṁ_cond / m̄`, as the conduction power of the field's
//! static power balance.
//!
//! **Radiation.** The hot spot emits bremsstrahlung `P_br = A₀ n_e² √t
//! [Z_eff (1 + 1.78 t^1.34) + 2.12 t (1 + …)] V`, `t = T/m_e c²` (Svensson,
//! Astrophys. J. 258, 335 (1982), as in
//! [`crate::fusion_pb11_thermal::bremsstrahlung_svensson_w_per_m3`]), with
//! the spectrum `e^(−hν/T)` per unit photon energy. A photon leaves the hot
//! spot with the escape probability of a uniform sphere,
//! `P(τ) = (3/4τ)[1 − 1/(2τ²) + (1/τ + 1/(2τ²)) e^(−2τ)]`, `τ = α_ν a`
//! ([`sphere_escape_probability`]), and crosses the shell radially, zone by
//! zone, each keeping `1 − e^(−α_ν Δr)` of what reaches it as heat. `α_ν` is
//! the free-free absorption coefficient with Gaunt factor one and stimulated
//! emission (Rybicki and Lightman (1979), equation 5.18a), at the hot spot's
//! temperature and, in each shell zone, at the zone's own ideal-gas
//! temperature `m̄ ε / 3`. What crosses the last zone leaves the fuel.
//! Christopherson et al. (Phys. Plasmas 25, 012703 (2018)) find from
//! simulations that of the bremsstrahlung about 40 % is absorbed in the hot
//! spot, 40 % in the shell and 20 % leaves; here the shares follow from the
//! state.
//!
//! **The burn before peak compression.** Under ideal hydrodynamics the
//! motion is reversible: the deceleration of the implosion is the mirror
//! image of the expansion after it. The solver first runs the ideal motion
//! forward from the caller's state (no burn, no alpha heating, no
//! conduction, no radiation) until the fusion rate it would carry has fallen
//! to [`START_RATE_FRACTION`] of its value at peak compression, reverses
//! every velocity, and from there runs the whole model forward, through
//! peak compression, to the end of the burn: the step at which the fusion
//! rate has fallen to [`END_RATE_FRACTION`] of its peak. Time zero is the
//! ideal peak compression. With the burn's physics on, the implosion no
//! longer stagnates exactly at the caller's state: the compression work,
//! the ablation and the alpha heating move it, and the neutron-averaged
//! state is returned beside the yield.
//!
//! # What must hold
//!
//! At every step. *Energy*: the hot spot's heat, the shell's internal and
//! kinetic energy and the hot spot's kinetic energy, plus the
//! bremsstrahlung and the alpha energy that left the fuel and the kinetic
//! energy the reaction products carried away, less every alpha energy
//! released, equal their value at the start, within [`ENERGY_TOLERANCE`] of
//! the larger of the starting energy and the alpha energy released
//! ([`HotSpotReject::EnergyLedgerBroken`]). The equations conserve this
//! exactly; the residual is the integrator's truncation. *Particles and
//! mass*: the deuterium and tritium in the hot spot and the shell, plus
//! twice the reactions, equal the fuel at the start; the alphas equal the
//! reactions; the burn-up fraction is at most one
//! ([`HotSpotReject::ParticleLedgerBroken`]). *The mesh*: every zone has a
//! positive volume, mass and internal energy; a step whose Runge-Kutta
//! stages would leave that, or leave the reactivity fit, is halved, at most
//! [`STEP_HALVINGS_MAX`] times, and a step that still cannot be taken is
//! refused by its reason ([`HotSpotReject::TemperatureLeftReactivityFit`],
//! [`HotSpotReject::NonFiniteValue`]). *The physics*: the hot-spot
//! temperature stays within the reactivity fit
//! ([`HotSpotReject::TemperatureLeftReactivityFit`]) and its electrons are
//! not degenerate ([`HotSpotReject::HotSpotDegenerate`]). The Coulomb
//! logarithms are the Formulary's, bounded below by the form `½ ln(1 + Λ²)`
//! ([`bounded_logarithm`]), which equals them where they are large.
//!
//! At the door. Beside the ranges of the five numbers, the geometry: the
//! fuel must hold more than the hot spot
//! ([`HotSpotReject::FuelMassNotAboveHotSpotMass`]), the rest must fill a
//! uniform shell with the given areal density
//! ([`HotSpotReject::ShellMassBelowArealDensity`]), and that shell must be
//! denser than the hot spot, so colder at one pressure: the model is of a
//! hot spot inside colder, denser fuel
//! ([`HotSpotReject::ShellNotDenserThanHotSpot`]).
//!
//! At acceptance. The whole run is repeated at half the step
//! ([`COURANT`] and half of it); the yield and the burn width of the two
//! must agree within [`SETTLE_TOLERANCE`], or both are halved again, at
//! most [`MAX_REFINEMENTS`] times ([`HotSpotReject::NotSettled`]); the finer
//! run is the answer. A third run with the alpha energy switched off gives
//! the yield without alpha heating, its amplification, and the no-alpha
//! Lawson parameter of Betti et al. (2015).
//!
//! # Assumptions, in the field's terms
//!
//! One temperature for ions and electrons in the hot spot; a uniform hot
//! spot (the conduction loss is that of Betti's profile with its centre at
//! the zone's temperature); D:T 50:50; no mix of ablator or shell material
//! into the hot spot, no asymmetry, no residual kinetic energy at peak
//! compression (the solver's peak compression is perfectly spherical and
//! still); no ablator outside the fuel, so nothing tamps the shell's outer
//! surface; the dense shell does not burn except by being ablated into the
//! hot spot; every alpha entering the shell is stopped there; free-free
//! absorption alone, with Gaunt factor one, takes up bremsstrahlung; the
//! shell zones' temperature is read from the ideal gas, which overstates it
//! where the cold fuel is degenerate and so understates its absorption; the
//! 14 MeV neutrons
//! leave without depositing energy; no magnetic field; the drive has ended.
//! Each departs from a NIF implosion, and most make the solver's burn
//! larger than a real one: this is the burn of a clean, spherical
//! implosion with the given stagnation.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::f64::consts::PI;
use std::fmt;

use crate::bosch_hale::{self, DEUTERON_MASS_KG, Reaction, TRITON_MASS_KG};
use crate::fusion_pb11_thermal::bremsstrahlung_svensson_w_per_m3;
use crate::pb11_reaction::{
    self as reaction, ALPHA_MASS_KG, ELECTRON_MASS_KG, ELEMENTARY_CHARGE_C, JOULE_PER_KEV, Species,
    VACUUM_PERMITTIVITY_F_PER_M,
};

/// Pascals in one gigabar.
pub const PA_PER_GBAR: f64 = 1.0e14;
/// Kilograms per square metre in one gram per square centimetre.
pub const KG_PER_M2_PER_G_PER_CM2: f64 = 10.0;
/// Reduced Planck constant, J s (CODATA 2018).
pub const HBAR_J_S: f64 = 1.054_571_817e-34;
/// Rest energy of the neutron, keV (CODATA 2018).
pub const NEUTRON_REST_KEV: f64 = 939_565.420_52;
/// Rest energy of the alpha particle, keV (CODATA 2018).
pub const ALPHA_REST_KEV: f64 = 3_727_379.4066;
/// Mass of a fuel ion with its electron, kg: the mean of the deuteron and the triton, and one electron.
pub const FUEL_ION_PAIR_MASS_KG: f64 = 0.5 * (DEUTERON_MASS_KG + TRITON_MASS_KG) + ELECTRON_MASS_KG;
/// Mass of an alpha with its two electrons, kg.
pub const ASH_PAIR_MASS_KG: f64 = ALPHA_MASS_KG + 2.0 * ELECTRON_MASS_KG;
/// Rest mass that leaves the hot spot per reaction, kg: a deuteron and a triton become an alpha; the electrons stay.
pub const MASS_LEAVING_PER_REACTION_KG: f64 = DEUTERON_MASS_KG + TRITON_MASS_KG - ALPHA_MASS_KG;

/// Above this pressure, Gbar, a hot spot is degenerate at every temperature
/// the reactivity fit holds: its electrons' Fermi energy at `n = p / 2T`
/// reaches `T` already at `T = 100 keV`. A test recomputes it.
pub const PRESSURE_MAX_GBAR: f64 = 4.601_876_765e7;
/// The radius, fuel mass and shell areal density above which the model's
/// volumes and energies would leave the range of 64-bit arithmetic: a
/// mathematical limit, far above any implosion.
pub const RADIUS_MAX_UM: f64 = 1.0e9;
pub const FUEL_MASS_MAX_MG: f64 = 1.0e12;
pub const SHELL_AREAL_DENSITY_MAX_G_PER_CM2: f64 = 1.0e12;

/// The reactivity fit's lowest temperature, keV.
pub const TEMPERATURE_MIN_KEV: f64 = bosch_hale::REACTIVITY_TEMPERATURE_MIN_KEV;
/// The reactivity fit's highest temperature, keV.
pub const TEMPERATURE_MAX_KEV: f64 = bosch_hale::REACTIVITY_TEMPERATURE_MAX_KEV;

/// Zones of equal mass the shell is cut into.
pub const SHELL_ZONES: usize = 64;
/// Zones of equal mass the remaining ablator outside the shell is cut into, when there is one.
pub const ABLATOR_ZONES: usize = 32;
/// Mass of a carbon ion of the ablator with its six electrons, kg (natural carbon, 12.011 u).
pub const ABLATOR_ION_PAIR_MASS_KG: f64 = 12.011 * reaction::ATOMIC_MASS_KG;
/// Charge of the ablator's ions: carbon, fully ionized.
pub const ABLATOR_CHARGE: f64 = 6.0;
/// Courant number: each step is this share of the shortest time of the state (a sound crossing of the hot spot or of a zone, the burn-out, the heating, the ablation of the innermost zone).
pub const COURANT: f64 = 0.05;
/// Quadratic artificial viscosity coefficient (von Neumann and Richtmyer); there is no linear term, so a smooth compression is not damped.
pub const VISCOSITY_QUADRATIC: f64 = 2.0;
/// The innermost zone, ablated below this share of its starting mass, joins the hot spot whole.
pub const ZONE_JOIN_FRACTION: f64 = 1.0e-3;
/// The burn starts where the ideal motion's fusion rate is this share of its value at peak compression.
pub const START_RATE_FRACTION: f64 = 1.0e-4;
/// The burn ends when the fusion rate has fallen to this share of its peak.
pub const END_RATE_FRACTION: f64 = 1.0e-3;
/// Most steps of one run.
pub const MAX_STEPS: usize = 400_000;
/// The energy ledger closes within this share of the larger of the starting energy and the alpha energy released.
pub const ENERGY_TOLERANCE: f64 = 1.0e-5;
/// The particle and mass ledgers close within this share of the fuel.
pub const PARTICLE_TOLERANCE: f64 = 1.0e-10;
/// The yield and the burn width at the step and at half the step agree within this share.
pub const SETTLE_TOLERANCE: f64 = 2.0e-3;
/// Most halvings of one step whose stages leave what the rates are defined on.
pub const STEP_HALVINGS_MAX: u32 = 30;
/// Most halvings of the step after the first pair of runs disagree.
pub const MAX_REFINEMENTS: u32 = 2;
/// Most rows of the burn history returned.
pub const HISTORY_ROWS_MAX: usize = 200;
/// Panels of the eight-point Gauss rule in the alpha path integral.
pub const ALPHA_PANELS: usize = 8;

/// The published shot this solver is held to in its tests, and where it was read.
pub const REFERENCE_NAME: &str = "Abu-Shawareb et al. (Indirect Drive ICF Collaboration), Achievement of target gain larger than unity in an inertial fusion experiment, Phys. Rev. Lett. 132, 065102 (2024), Table I, shot N221204: yield 3.1 +- 0.16 MJ, T_DD 12 +- 1 keV, hot-spot pressure 608 +56/-59 Gbar, volume 12.9 +- 1.5 x 1e5 um3, DSR 2.83 +- 0.17 %, burn width 75 +- 15 ps, burn-up 4.33 %, DT fuel 220 ug";
pub const REFERENCE_URL: &str = "https://doi.org/10.1103/PhysRevLett.132.065102";
/// Date the reference was read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";

/// `ṁ = C m̄ a κ₀ T^(5/2) / lnΛ`: the mass that crosses the edge of the
/// isobaric hot spot of Betti et al. (LLE Review 85 (2000), equation 41),
/// `T = T_c (1 − r²/a²)^(2/5) / (1 − 0.15 r²/a²)`, for which `C = (16π/25) / 0.85^(5/2)`.
pub const ABLATION_COEFFICIENT: f64 = 16.0 * std::f64::consts::PI / 25.0 / 0.666_112_087_039_411;

/// Joules per megajoule.
const J_PER_MJ: f64 = 1.0e6;

/// The alpha particle's birth energy, keV.
pub fn alpha_birth_energy_kev() -> f64 {
    let q = Reaction::DeuteriumTritium.energy_kev();
    q * (q + 2.0 * NEUTRON_REST_KEV) / (2.0 * (NEUTRON_REST_KEV + ALPHA_REST_KEV + q))
}

/// The neutron's birth energy, keV.
pub fn neutron_birth_energy_kev() -> f64 {
    Reaction::DeuteriumTritium.energy_kev() - alpha_birth_energy_kev()
}

/// The fraction of Krokhin and Rozanov (1973) at `τ = a / ℓ_α`.
pub fn alpha_deposition_fraction(tau: f64) -> f64 {
    if tau <= 0.5 { 1.5 * tau - 0.8 * tau * tau } else { 1.0 - 0.25 / tau + 1.0 / (160.0 * tau * tau * tau) }
}

/// Spitzer-Braginskii `κ₀`: `κ = κ₀ T^(5/2) / lnΛ` in W/(m J) with `T` in joules.
pub fn conductivity_coefficient() -> f64 {
    let e2 = ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C;
    let four_pi_eps = 4.0 * PI * VACUUM_PERMITTIVITY_F_PER_M;
    3.1616 * 3.0 * four_pi_eps * four_pi_eps / (4.0 * (2.0 * PI).sqrt() * ELECTRON_MASS_KG.sqrt() * e2 * e2)
}

/// Fermi energy of electrons at a density, keV.
pub fn fermi_energy_kev(electron_density_per_m3: f64) -> f64 {
    HBAR_J_S * HBAR_J_S * math::powf(3.0 * PI * PI * electron_density_per_m3, 2.0 / 3.0) / (2.0 * ELECTRON_MASS_KG) / JOULE_PER_KEV
}

/// A Coulomb logarithm `L` as `½ ln(1 + e^(2L))`, the form `½ ln(1 + Λ²)` of
/// Gericke, Murillo and Schlanges (Phys. Rev. E 65, 036418 (2002)) with the
/// Formulary's `Λ`: equal to `L` where `L` is large (within 2e-4 from 4 up)
/// and above zero where the Formulary's own form goes to zero or below, in
/// the cool, dense hot spot late in a burn.
pub fn bounded_logarithm(formulary: f64) -> f64 {
    if formulary > 0.0 { formulary + 0.5 * math::ln_1p(math::exp(-2.0 * formulary)) } else { 0.5 * math::ln_1p(math::exp(2.0 * formulary)) }
}

/// The Coulomb logarithm of the hot spot's electrons: the Formulary's
/// `24 − ln(√n_e/T_e)` ([`crate::pb11_reaction::coulomb_logarithm_electron`]) bounded by [`bounded_logarithm`].
pub fn electron_logarithm(electron_density_per_m3: f64, temperature_kev: f64) -> f64 {
    bounded_logarithm(reaction::coulomb_logarithm_electron(electron_density_per_m3, temperature_kev))
}

/// Energy an alpha of `energy_kev` gives to the plasma, keV/s: the NRL
/// test-particle rates of [`crate::pb11_reaction::Background`] on electrons
/// (with its relativistic factor) and on each ion species, each Coulomb
/// logarithm bounded by [`bounded_logarithm`].
fn alpha_loss_kev_per_s(
    electron_density_per_m3: f64,
    temperature_kev: f64,
    electron_logarithm: f64,
    relativistic: f64,
    ions: &[Species],
    energy_kev: f64,
) -> f64 {
    let alpha = Species { charge: 2.0, mass_kg: ALPHA_MASS_KG, density_per_m3: 0.0 };
    let speed = (2.0 * energy_kev * JOULE_PER_KEV / ALPHA_MASS_KG).sqrt();
    let e2 = ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C;
    let common = 4.0 * e2 * e2 / (4.0 * PI * VACUUM_PERMITTIVITY_F_PER_M * VACUUM_PERMITTIVITY_F_PER_M * speed) / JOULE_PER_KEV;
    let t_j = temperature_kev * JOULE_PER_KEV;
    let x = ELECTRON_MASS_KG * speed * speed / (2.0 * t_j);
    let (psi, derivative) = reaction::psi_and_derivative(x);
    let mut loss = common * electron_density_per_m3 * electron_logarithm / ELECTRON_MASS_KG
        * (psi - ELECTRON_MASS_KG / ALPHA_MASS_KG * derivative)
        * relativistic;
    for ion in ions {
        if ion.density_per_m3 > 0.0 {
            let x = ion.mass_kg * speed * speed / (2.0 * t_j);
            let (psi, derivative) = reaction::psi_and_derivative(x);
            let logarithm = bounded_logarithm(reaction::coulomb_logarithm_fast_ion(
                electron_density_per_m3,
                temperature_kev,
                &alpha,
                speed,
                ion,
            ));
            loss += common * ion.charge * ion.charge * ion.density_per_m3 * logarithm / ion.mass_kg
                * (psi - ion.mass_kg / ALPHA_MASS_KG * derivative);
        }
    }
    loss
}

/// The alpha range, metres: the straight path from birth down to `3T/2`,
/// `∫ v dE / |dE/dt|`, by the same rates. For the tests and the record; the
/// model uses [`alpha_path_m`].
pub fn alpha_range_m(electron_density_per_m3: f64, temperature_kev: f64, fuel_ions_per_m3: f64, ash_per_m3: f64) -> f64 {
    let ions = [
        Species { charge: 1.0, mass_kg: DEUTERON_MASS_KG, density_per_m3: 0.5 * fuel_ions_per_m3 },
        Species { charge: 1.0, mass_kg: TRITON_MASS_KG, density_per_m3: 0.5 * fuel_ions_per_m3 },
        Species { charge: 2.0, mass_kg: ALPHA_MASS_KG, density_per_m3: ash_per_m3 },
    ];
    let logarithm = electron_logarithm(electron_density_per_m3, temperature_kev);
    let relativistic = reaction::relativistic_exchange_factor(temperature_kev);
    reaction::integrate(1.5 * temperature_kev, alpha_birth_energy_kev(), 8 * ALPHA_PANELS, |energy| {
        let loss = alpha_loss_kev_per_s(electron_density_per_m3, temperature_kev, logarithm, relativistic, &ions, energy);
        (2.0 * energy * JOULE_PER_KEV / ALPHA_MASS_KG).sqrt() / loss
    })
}

/// The alpha path length `ℓ_α`, metres: three times the mean depth at which
/// the alpha's energy is deposited, from birth to `3T/2`, in a plasma of
/// these densities at one temperature.
pub fn alpha_path_m(electron_density_per_m3: f64, temperature_kev: f64, fuel_ions_per_m3: f64, ash_per_m3: f64) -> f64 {
    let ions = [
        Species { charge: 1.0, mass_kg: DEUTERON_MASS_KG, density_per_m3: 0.5 * fuel_ions_per_m3 },
        Species { charge: 1.0, mass_kg: TRITON_MASS_KG, density_per_m3: 0.5 * fuel_ions_per_m3 },
        Species { charge: 2.0, mass_kg: ALPHA_MASS_KG, density_per_m3: ash_per_m3 },
    ];
    let logarithm = electron_logarithm(electron_density_per_m3, temperature_kev);
    let relativistic = reaction::relativistic_exchange_factor(temperature_kev);
    let low = 1.5 * temperature_kev;
    let high = alpha_birth_energy_kev();
    let depth = reaction::integrate(low, high, ALPHA_PANELS, |energy| {
        let loss = alpha_loss_kev_per_s(electron_density_per_m3, temperature_kev, logarithm, relativistic, &ions, energy);
        let speed = (2.0 * energy * JOULE_PER_KEV / ALPHA_MASS_KG).sqrt();
        (energy - low) * speed / loss
    });
    3.0 * depth / (high - low)
}

/// Planck constant, J s (exact, SI 2019).
pub const PLANCK_J_S: f64 = 6.626_070_15e-34;
/// Speed of light, m/s.
const C_M_PER_S: f64 = reaction::SPEED_OF_LIGHT_M_PER_S;
/// Photon energies over the hot-spot temperature, `x = hν/T`, are integrated from 0 to this.
pub const PHOTON_ENERGY_MAX_OVER_T: f64 = 40.0;
/// Panels of the eight-point Gauss rule over `x`.
pub const PHOTON_PANELS: usize = 8;

/// Free-free absorption coefficient, 1/m, with the Gaunt factor one and
/// stimulated emission (Rybicki and Lightman, Radiative Processes in
/// Astrophysics (1979), equation 5.18a, in SI):
/// `α_ν = (4/(3 m_e h c)) (e²/4πε₀)³ (2π/(3 m_e T))^½ n_e Σ Z² n_i ν⁻³ (1 − e^(−hν/T))`.
pub fn free_free_absorption_per_m(
    electron_density_per_m3: f64,
    charge_squared_ion_density_per_m3: f64,
    temperature_kev: f64,
    photon_energy_kev: f64,
) -> f64 {
    let q = ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C / (4.0 * PI * VACUUM_PERMITTIVITY_F_PER_M);
    let t_j = temperature_kev * JOULE_PER_KEV;
    let nu = photon_energy_kev * JOULE_PER_KEV / PLANCK_J_S;
    4.0 / (3.0 * ELECTRON_MASS_KG * PLANCK_J_S * C_M_PER_S) * q * q * q * (2.0 * PI / (3.0 * ELECTRON_MASS_KG * t_j)).sqrt()
        * electron_density_per_m3
        * charge_squared_ion_density_per_m3
        / (nu * nu * nu)
        * -math::exp_m1(-photon_energy_kev / temperature_kev)
}

/// The chance that a photon born uniformly in a uniform sphere of optical
/// radius `τ`, in a direction taken at random, leaves it:
/// `P(τ) = (3/4τ) [1 − 1/(2τ²) + (1/τ + 1/(2τ²)) e^(−2τ)]` (exact). Below
/// τ = 0.05, where the bracket loses digits to cancellation, its series
/// `1 − 3τ/4 + 2τ²/5 − τ³/6 + 2τ⁴/35 − τ⁵/60 + 4τ⁶/945 − τ⁷/1050`, whose
/// next term is below 1e-12 there.
pub fn sphere_escape_probability(tau: f64) -> f64 {
    if tau < 0.05 {
        const C: [f64; 8] = [1.0, -0.75, 0.4, -1.0 / 6.0, 2.0 / 35.0, -1.0 / 60.0, 4.0 / 945.0, -1.0 / 1050.0];
        C.iter().rev().fold(0.0, |sum, c| sum * tau + c)
    } else {
        0.75 / tau * (1.0 - 0.5 / (tau * tau) + (1.0 / tau + 0.5 / (tau * tau)) * math::exp(-2.0 * tau))
    }
}

/// What the alphas and the bremsstrahlung of one step do, worked out at the
/// start of the step and held through it.
#[derive(Clone, Debug)]
struct Transport {
    /// Share of the alpha energy deposited in the hot spot.
    alpha_share: f64,
    /// Share of the emitted bremsstrahlung that leaves the hot spot.
    leaves_hot_spot: f64,
    /// Share of the emitted bremsstrahlung absorbed in each shell zone.
    zone_share: Vec<f64>,
    /// Share of the emitted bremsstrahlung that leaves the fuel.
    escapes: f64,
}

/// The stagnation state of the implosion under ideal hydrodynamics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HotSpotQuestion {
    /// Hot-spot temperature of ions and electrons, keV.
    pub hot_spot_temperature_kev: f64,
    /// Hot-spot pressure, gigabar.
    pub hot_spot_pressure_gbar: f64,
    /// Hot-spot radius, micrometres.
    pub hot_spot_radius_um: f64,
    /// Areal density of the dense fuel around the hot spot, g/cm².
    pub shell_areal_density_g_per_cm2: f64,
    /// Whole deuterium-tritium fuel mass, hot spot and shell, milligrams.
    pub fuel_mass_mg: f64,
    /// Mass of the ablator remaining outside the fuel at peak compression, milligrams; zero for none.
    pub ablator_mass_mg: f64,
}

/// Why no burn was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum HotSpotReject {
    /// An input was NaN or infinite.
    InputNotFinite,
    /// The hot-spot temperature is outside 0.2 to 100 keV (`TEMPERATURE_MIN_KEV` to `TEMPERATURE_MAX_KEV`), where the reactivity fit of Bosch and Hale is defined.
    TemperatureOutsideReactivityFit { temperature_kev: f64 },
    /// The hot-spot pressure is not above zero.
    PressureNotPositive { pressure_gbar: f64 },
    /// The ablator mass is below zero.
    AblatorMassNegative { ablator_mass_mg: f64 },
    /// The radius, the fuel mass or the shell's areal density is above `RADIUS_MAX_UM`, `FUEL_MASS_MAX_MG` or `SHELL_AREAL_DENSITY_MAX_G_PER_CM2`, where the model's volumes and energies would leave the range of 64-bit arithmetic.
    InputTooLargeForArithmetic { name: &'static str, value: f64 },
    /// The hot spot at peak compression is degenerate, its electrons' Fermi energy at or above its temperature; above `PRESSURE_MAX_GBAR` it is so at every temperature the reactivity fit holds.
    StagnationDegenerate { temperature_kev: f64, fermi_energy_kev: f64 },
    /// The hot-spot radius is not above zero.
    RadiusNotPositive { radius_um: f64 },
    /// The shell's areal density is not above zero: with no shell nothing confines the hot spot.
    ShellArealDensityNotPositive { areal_density_g_per_cm2: f64 },
    /// The fuel mass is not more than the hot spot holds at this pressure, temperature and radius: no fuel is left for the shell.
    FuelMassNotAboveHotSpotMass { fuel_mass_mg: f64, hot_spot_mass_mg: f64 },
    /// The fuel left for the shell is less than any uniform shell outside the hot spot with this areal density holds, `4π R² (ρR)_s`.
    ShellMassBelowArealDensity { shell_mass_mg: f64, least_mg: f64 },
    /// The shell is not denser than the hot spot: at one pressure it would be as hot or hotter, and the model is of a hot spot inside colder, denser fuel.
    ShellNotDenserThanHotSpot { shell_density_g_per_cm3: f64, hot_spot_density_g_per_cm3: f64 },
    /// The hot spot's electrons are degenerate, their Fermi energy at or above the temperature: the ideal-gas pressure and the Maxwellian rates of the model do not hold.
    HotSpotDegenerate { time_ps: f64, temperature_kev: f64, fermi_energy_kev: f64 },
    /// During the burn the hot-spot temperature left 0.2 to 100 keV (`TEMPERATURE_MIN_KEV` to `TEMPERATURE_MAX_KEV`), where the reactivity fit is defined.
    TemperatureLeftReactivityFit { time_ps: f64, temperature_kev: f64 },
    /// A shell zone's volume was not above zero or its nodes crossed.
    ShellMeshTangled { time_ps: f64 },
    /// The internal energy of the hot spot or of a shell zone was not above zero.
    InternalEnergyNotPositive { time_ps: f64 },
    /// An intermediate value was NaN or infinite.
    NonFiniteValue { time_ps: f64 },
    /// At a step the energy ledger left `ENERGY_TOLERANCE`.
    EnergyLedgerBroken { time_ps: f64, residual: f64 },
    /// At a step the particle or mass ledger left `PARTICLE_TOLERANCE`, or the burn-up exceeded the fuel.
    ParticleLedgerBroken { time_ps: f64, residual: f64 },
    /// A run reached `MAX_STEPS` with the burn not over.
    StepLimitReached { steps: usize },
    /// The yield or the burn width at the step and at half the step differ by more than `SETTLE_TOLERANCE`.
    NotSettled { yield_change: f64, burn_width_change: f64 },
}

impl fmt::Display for HotSpotReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotFinite => write!(f, "an input was not finite"),
            Self::TemperatureOutsideReactivityFit { temperature_kev } => write!(
                f,
                "hot-spot temperature {temperature_kev} keV is outside {TEMPERATURE_MIN_KEV} to {TEMPERATURE_MAX_KEV} keV, where the D-T reactivity of Bosch and Hale is defined"
            ),
            Self::PressureNotPositive { pressure_gbar } => write!(f, "hot-spot pressure {pressure_gbar} Gbar is not above zero"),
            Self::AblatorMassNegative { ablator_mass_mg } => write!(f, "ablator mass {ablator_mass_mg} mg is below zero"),
            Self::InputTooLargeForArithmetic { name, value } => write!(
                f,
                "{name} {value} is so large that the model's volumes and energies would leave the range of 64-bit arithmetic"
            ),
            Self::StagnationDegenerate { temperature_kev, fermi_energy_kev } => write!(
                f,
                "the hot spot at peak compression is degenerate: Fermi energy {fermi_energy_kev} keV at a temperature of {temperature_kev} keV"
            ),
            Self::RadiusNotPositive { radius_um } => write!(f, "hot-spot radius {radius_um} um is not above zero"),
            Self::ShellArealDensityNotPositive { areal_density_g_per_cm2 } => write!(
                f,
                "shell areal density {areal_density_g_per_cm2} g/cm2 is not above zero: with no shell nothing confines the hot spot"
            ),
            Self::FuelMassNotAboveHotSpotMass { fuel_mass_mg, hot_spot_mass_mg } => write!(
                f,
                "fuel mass {fuel_mass_mg} mg is not more than the {hot_spot_mass_mg} mg the hot spot holds at this pressure, temperature and radius"
            ),
            Self::ShellMassBelowArealDensity { shell_mass_mg, least_mg } => write!(
                f,
                "the {shell_mass_mg} mg of fuel left for the shell is less than the {least_mg} mg any uniform shell outside the hot spot with this areal density holds"
            ),
            Self::ShellNotDenserThanHotSpot { shell_density_g_per_cm3, hot_spot_density_g_per_cm3 } => write!(
                f,
                "the shell's density, {shell_density_g_per_cm3} g/cm3, is not above the hot spot's, {hot_spot_density_g_per_cm3} g/cm3: at one pressure the shell would be as hot or hotter, and the model is of a hot spot inside colder, denser fuel"
            ),
            Self::HotSpotDegenerate { time_ps, temperature_kev, fermi_energy_kev } => write!(
                f,
                "at {time_ps} ps the hot spot's electrons are degenerate: Fermi energy {fermi_energy_kev} keV at a temperature of {temperature_kev} keV"
            ),
            Self::TemperatureLeftReactivityFit { time_ps, temperature_kev } => write!(
                f,
                "at {time_ps} ps the hot-spot temperature is {temperature_kev} keV, outside {TEMPERATURE_MIN_KEV} to {TEMPERATURE_MAX_KEV} keV, where the reactivity fit is defined"
            ),
            Self::ShellMeshTangled { time_ps } => write!(f, "at {time_ps} ps a shell zone's volume was not above zero"),
            Self::InternalEnergyNotPositive { time_ps } => {
                write!(f, "at {time_ps} ps the internal energy of the hot spot or of a shell zone was not above zero")
            }
            Self::NonFiniteValue { time_ps } => write!(f, "at {time_ps} ps a value was not finite"),
            Self::EnergyLedgerBroken { time_ps, residual } => {
                write!(f, "at {time_ps} ps the energy ledger missed by {residual} of the energy, above {ENERGY_TOLERANCE}")
            }
            Self::ParticleLedgerBroken { time_ps, residual } => {
                write!(f, "at {time_ps} ps the particle or mass ledger missed by {residual} of the fuel, above {PARTICLE_TOLERANCE}")
            }
            Self::StepLimitReached { steps } => write!(f, "a run reached {steps} steps with the burn not over"),
            Self::NotSettled { yield_change, burn_width_change } => write!(
                f,
                "at half the step the yield moved by {yield_change} and the burn width by {burn_width_change} of their values, above {SETTLE_TOLERANCE}"
            ),
        }
    }
}

impl std::error::Error for HotSpotReject {}

/// The burn.
#[derive(Clone, Debug, PartialEq)]
pub struct HotSpotBurn {
    /// Fusion yield, MJ: reactions times 17.589 MeV.
    pub yield_mj: f64,
    /// D-T reactions, which is the neutron yield.
    pub neutron_yield: f64,
    /// Fusion yield of the same implosion with the alpha energy switched off, MJ.
    pub yield_without_alpha_heating_mj: f64,
    /// Yield over the yield without alpha heating.
    pub yield_amplification: f64,
    /// Twice the reactions over the deuterium and tritium ions of the whole fuel.
    pub burn_up_fraction: f64,
    /// Full width at half maximum of the fusion rate, ps.
    pub burn_width_ps: f64,
    /// Time of the peak fusion rate after the ideal peak compression, ps.
    pub bang_time_ps: f64,
    /// Peak fusion power, W: reactions per second times 17.589 MeV.
    pub peak_fusion_power_w: f64,
    /// Neutron-averaged hot-spot temperature, keV.
    pub burn_averaged_temperature_kev: f64,
    /// Neutron-averaged hot-spot pressure, Gbar.
    pub burn_averaged_pressure_gbar: f64,
    /// Neutron-averaged hot-spot radius, um.
    pub burn_averaged_radius_um: f64,
    /// Neutron-averaged areal density of the hot spot, its density times its radius, g/cm².
    pub burn_averaged_hot_spot_areal_density_g_per_cm2: f64,
    /// Neutron-averaged areal density of the shell, g/cm².
    pub burn_averaged_shell_areal_density_g_per_cm2: f64,
    /// Hot-spot mass at bang time, mg.
    pub hot_spot_mass_at_bang_mg: f64,
    /// Hot-spot thermal energy at bang time, kJ.
    pub hot_spot_energy_at_bang_kj: f64,
    /// Highest hot-spot temperature, keV.
    pub peak_temperature_kev: f64,
    /// Highest hot-spot pressure, Gbar.
    pub peak_pressure_gbar: f64,
    /// Share of the fuel ablated into the hot spot by the end of the burn.
    pub fuel_ablated_fraction: f64,
    /// 1 when every shell zone was ablated into the hot spot before the burn ended, else 0.
    pub burned_through: f64,
    /// Alpha heating deposited directly in the hot spot over bremsstrahlung and conduction, at bang time.
    pub self_heating_ratio_at_bang: f64,
    /// The largest value of that ratio during the burn.
    pub self_heating_ratio_peak: f64,
    /// Alpha energy deposited in the hot spot up to bang time over the compression work done on it up to bang time.
    pub alpha_energy_over_compression_work_at_bang: f64,
    /// All alpha energy deposited in the hot spot over all compression work done on it.
    pub alpha_energy_over_compression_work: f64,
    /// The generalised Lawson parameter of Betti et al. (2015) from the no-alpha run: `(ρR)^0.61 (0.24 Y₁₆ / M)^0.34` (their equation 11).
    pub lawson_parameter_no_alpha: f64,
    /// The Lawson parameter of Betti et al. (2010), `p τ / (24 T² / ⟨σv⟩ E_α)` at the neutron-averaged state, with `τ` the burn width.
    pub lawson_parameter_pressure_time: f64,
    /// The generalised Lawson criterion of Lindl et al. as PRL 129, 075001 evaluates it (its equation 2): `(p̄/420 Gbar)(R̄/50 µm)`, neutron-averaged. Above one: ignition by that criterion.
    pub lawson_criterion_lindl: f64,
    /// `E_α / (2 E_hs)` (PRL 129, 075001, equation 4, after Christopherson et al.), `E_α` the yield over five and `E_hs` the hot-spot energy at bang time. Above 1.4: ignition by that criterion.
    pub alpha_energy_over_twice_hot_spot_energy: f64,
    /// `Q_α^HS = 0.5 E_α / E_pdV^HS` (Betti et al. 2015, equation 14): half the alpha energy released over the compression work done on the hot spot up to bang time. Above one: the hot spot's burning-plasma regime.
    pub q_alpha_hot_spot: f64,
    /// Alpha energy released, MJ.
    pub alpha_energy_mj: f64,
    /// Alpha energy deposited directly in the hot spot, MJ.
    pub alpha_energy_in_hot_spot_mj: f64,
    /// Alpha energy deposited in the shell and returned with the ablated fuel, MJ.
    pub alpha_energy_through_shell_mj: f64,
    /// Alpha energy that left the fuel after burn-through with no ablator outside, MJ.
    pub alpha_energy_escaped_mj: f64,
    /// Alpha energy deposited in the ablator after the fuel was burned through, MJ.
    pub alpha_energy_in_ablator_mj: f64,
    /// Neutron energy, MJ.
    pub neutron_energy_mj: f64,
    /// Bremsstrahlung that left the fuel, MJ.
    pub bremsstrahlung_mj: f64,
    /// Bremsstrahlung absorbed in the shell, MJ.
    pub bremsstrahlung_absorbed_in_shell_mj: f64,
    /// Heat conducted from the hot spot into the shell and returned with the ablated fuel, MJ.
    pub conduction_returned_mj: f64,
    /// Work done on the hot spot by the shell while compressing it, MJ.
    pub compression_work_mj: f64,
    /// Work done by the hot spot on the shell while expanding, MJ.
    pub expansion_work_mj: f64,
    /// Kinetic energy of the hot-spot flow carried away with the reaction products' mass, MJ.
    pub kinetic_energy_carried_by_products_mj: f64,
    /// Energy of the hot spot, the shell and their motion at the start of the burn, MJ.
    pub energy_at_start_mj: f64,
    /// The same at the end of the burn, MJ.
    pub energy_at_end_mj: f64,
    /// The largest energy ledger residual over every step of the three runs.
    pub energy_residual: f64,
    /// The largest particle and mass ledger residual over every step.
    pub particle_residual: f64,
    /// Time of the start of the burn before the ideal peak compression, ps (negative).
    pub start_time_ps: f64,
    /// Steps of the answering run.
    pub steps: usize,
    /// Halvings of the step beyond the first pair of runs.
    pub refinements: u32,
    /// The density of the shell at the ideal peak compression, g/cm³.
    pub shell_density_g_per_cm3: f64,
    /// The density of the ablator at the ideal peak compression, g/cm³: the shell's pressure and temperature, carbon fully ionized.
    pub ablator_density_g_per_cm3: f64,
    /// The hot-spot mass at the ideal peak compression, mg.
    pub stagnation_hot_spot_mass_mg: f64,
    /// Burn history of the answering run, at most `HISTORY_ROWS_MAX` rows, each a step of it (every n-th step, the bang step and the last): time ps, fusion power W, hot-spot temperature keV, pressure Gbar, radius um.
    pub history: Vec<[f64; 5]>,
}

/// What the model includes in a run.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Physics {
    burn: bool,
    alpha_heating: bool,
    conduction: bool,
    radiation: bool,
}

const FULL: Physics = Physics { burn: true, alpha_heating: true, conduction: true, radiation: true };
const NO_ALPHA: Physics = Physics { burn: true, alpha_heating: false, conduction: true, radiation: true };
const IDEAL: Physics = Physics { burn: false, alpha_heating: false, conduction: false, radiation: false };

/// A fault put in on purpose, for the tests that attack the ledgers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Fault {
    None,
    /// The hot spot does no work on the shell: its heat does not fall as it expands.
    NoExpansionWork,
    /// The ablated fuel arrives without the internal energy the shell zone gave up.
    AblationLosesEnergy,
    /// Each reaction takes three fuel ions.
    ThreeIonsPerReaction,
}

/// The accounts integrated beside the state.
const ACCOUNTS: usize = 13;
const A_REACTIONS: usize = 0;
const A_ALPHA: usize = 1;
const A_ALPHA_HOT: usize = 2;
const A_ALPHA_SHELL: usize = 3;
const A_ALPHA_ESCAPED: usize = 4;
const A_BREMSSTRAHLUNG: usize = 5;
const A_CARRIED: usize = 6;
const A_CONDUCTION: usize = 7;
const A_COMPRESSION: usize = 8;
const A_EXPANSION: usize = 9;
const A_ABLATED: usize = 10;
const A_RADIATION_TO_SHELL: usize = 11;
const A_ALPHA_ABLATOR: usize = 12;

#[derive(Clone, Debug)]
struct State {
    /// Node radii, m. `r[0]` is the hot spot's radius.
    r: Vec<f64>,
    /// Node speeds, m/s.
    u: Vec<f64>,
    /// Zone masses, kg.
    mass: Vec<f64>,
    /// Zone internal energies, J.
    energy: Vec<f64>,
    /// Hot-spot thermal energy, J.
    hot_energy: f64,
    /// Deuterium and tritium ions in the hot spot.
    fuel_ions: f64,
    /// Alpha ash in the hot spot.
    alphas: f64,
    /// The innermost zones that are fuel; the zones outside them are ablator.
    fuel_zones: usize,
    accounts: [f64; ACCOUNTS],
}

impl State {
    fn zones(&self) -> usize {
        self.mass.len()
    }

    fn hot_mass(&self) -> f64 {
        self.fuel_ions * FUEL_ION_PAIR_MASS_KG + self.alphas * ASH_PAIR_MASS_KG
    }

    fn hot_volume(&self) -> f64 {
        4.0 / 3.0 * PI * self.r[0] * self.r[0] * self.r[0]
    }

    fn particles(&self) -> f64 {
        2.0 * self.fuel_ions + 3.0 * self.alphas
    }

    fn temperature_kev(&self) -> f64 {
        2.0 * self.hot_energy / (3.0 * self.particles()) / JOULE_PER_KEV
    }

    fn pressure_pa(&self) -> f64 {
        2.0 * self.hot_energy / (3.0 * self.hot_volume())
    }

    fn electron_density(&self) -> f64 {
        (self.fuel_ions + 2.0 * self.alphas) / self.hot_volume()
    }

    fn node_masses(&self) -> Vec<f64> {
        let n = self.zones();
        let mut nodes = vec![0.0; n + 1];
        for (i, m) in self.mass.iter().enumerate() {
            nodes[i] += 0.5 * m;
            nodes[i + 1] += 0.5 * m;
        }
        nodes[0] += 0.6 * self.hot_mass();
        nodes
    }

    fn kinetic(&self) -> f64 {
        self.node_masses().iter().zip(&self.u).map(|(m, u)| 0.5 * m * u * u).sum()
    }

    /// Thermal, internal and kinetic energy of the hot spot and the shell, J.
    fn total_energy(&self) -> f64 {
        self.hot_energy + self.energy.iter().sum::<f64>() + self.kinetic()
    }

    fn zone_volume(&self, i: usize) -> f64 {
        4.0 / 3.0 * PI * (math::powi(self.r[i + 1], 3) - math::powi(self.r[i], 3))
    }

    /// Areal density of the fuel zones, kg/m².
    fn shell_areal_density(&self) -> f64 {
        (0..self.fuel_zones).map(|i| self.mass[i] / self.zone_volume(i) * (self.r[i + 1] - self.r[i])).sum()
    }

    /// Ion density, electron density and temperature of zone `i`, from its
    /// composition: deuterium-tritium, or carbon fully ionized.
    fn zone_plasma(&self, i: usize) -> (f64, f64, f64) {
        let v = self.zone_volume(i);
        let pressure = 2.0 / 3.0 * self.energy[i] / v;
        if i < self.fuel_zones {
            let ions = self.mass[i] / v / FUEL_ION_PAIR_MASS_KG;
            (ions, ions, FUEL_ION_PAIR_MASS_KG * self.energy[i] / self.mass[i] / 3.0 / JOULE_PER_KEV)
        } else {
            let ions = self.mass[i] / v / ABLATOR_ION_PAIR_MASS_KG;
            (ions, ABLATOR_CHARGE * ions, pressure / ((1.0 + ABLATOR_CHARGE) * ions) / JOULE_PER_KEV)
        }
    }

    /// `self + h · d`, with `d` a derivative of the same shape.
    fn plus(&self, d: &State, h: f64) -> State {
        let add = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x + h * y).collect::<Vec<_>>();
        let mut accounts = self.accounts;
        for (a, b) in accounts.iter_mut().zip(d.accounts) {
            *a += h * b;
        }
        State {
            r: add(&self.r, &d.r),
            u: add(&self.u, &d.u),
            mass: add(&self.mass, &d.mass),
            energy: add(&self.energy, &d.energy),
            hot_energy: self.hot_energy + h * d.hot_energy,
            fuel_ions: self.fuel_ions + h * d.fuel_ions,
            alphas: self.alphas + h * d.alphas,
            fuel_zones: self.fuel_zones,
            accounts,
        }
    }
}

/// Rates at one state.
#[derive(Clone, Copy, Debug, Default)]
struct Rates {
    reactions_per_s: f64,
    temperature_kev: f64,
    pressure_pa: f64,
    alpha_to_hot_spot_w: f64,
    conduction_w: f64,
    bremsstrahlung_w: f64,
    ablation_kg_per_s: f64,
}

struct Model {
    physics: Physics,
    fault: Fault,
    /// Starting mass of one shell zone, kg.
    zone_mass: f64,
    /// Fuel ions of the whole fuel at the start.
    fuel_total: f64,
}

impl Model {
    /// The derivative of the state, with the step's transport held.
    fn rhs(&self, s: &State, transport: &Transport) -> (State, Rates) {
        let n = s.zones();
        let a = s.r[0];
        let volume = s.hot_volume();
        let t_kev = s.temperature_kev();
        let p = s.pressure_pa();
        let hot_mass = s.hot_mass();
        let ne = s.electron_density();
        let fuel_density = s.fuel_ions / volume;
        let reactions = if self.physics.burn {
            0.25 * fuel_density * fuel_density * bosch_hale::reactivity_m3_per_s(Reaction::DeuteriumTritium, t_kev).unwrap_or(f64::NAN) * volume
        } else {
            0.0
        };
        let alpha_w = if self.physics.alpha_heating { reactions * alpha_birth_energy_kev() * JOULE_PER_KEV } else { 0.0 };
        let alpha_hot = transport.alpha_share * alpha_w;
        let alpha_rest = alpha_w - alpha_hot;
        let logarithm = electron_logarithm(ne, t_kev);
        let emitted = if self.physics.radiation {
            let charge = (s.fuel_ions + 4.0 * s.alphas) / (s.fuel_ions + 2.0 * s.alphas);
            bremsstrahlung_svensson_w_per_m3(ne, charge, t_kev) * volume
        } else {
            0.0
        };
        let bremsstrahlung = emitted * transport.leaves_hot_spot;
        let escaping = emitted * transport.escapes;
        let t_j = t_kev * JOULE_PER_KEV;
        let carried = 0.3 * s.u[0] * s.u[0] * MASS_LEAVING_PER_REACTION_KG * reactions;
        let mut d = State {
            r: s.u.clone(),
            u: vec![0.0; n + 1],
            mass: vec![0.0; n],
            energy: vec![0.0; n],
            hot_energy: 0.0,
            fuel_ions: -2.0 * reactions,
            alphas: reactions,
            fuel_zones: s.fuel_zones,
            accounts: [0.0; ACCOUNTS],
        };
        if self.fault == Fault::ThreeIonsPerReaction {
            d.fuel_ions = -3.0 * reactions;
        }
        let work = 4.0 * PI * a * a * p * s.u[0];
        let (alpha_shell, alpha_escaped, conduction_mass, ablation, brought_in) = if n > 0 {
            let areas: Vec<f64> = s.r.iter().map(|r| 4.0 * PI * r * r).collect();
            let mut pressure = vec![0.0; n];
            for i in 0..n {
                let v = s.zone_volume(i);
                let rho = s.mass[i] / v;
                let du = s.u[i + 1] - s.u[i];
                let dv = areas[i + 1] * s.u[i + 1] - areas[i] * s.u[i];
                // The viscosity acts only where the zone is being compressed: where its nodes close and its volume falls.
                let q = if du < 0.0 && dv < 0.0 { VISCOSITY_QUADRATIC * rho * du * du } else { 0.0 };
                pressure[i] = 2.0 / 3.0 * s.energy[i] / v + q;
                d.energy[i] = -pressure[i] * dv + emitted * transport.zone_share[i];
            }
            let nodes = s.node_masses();
            d.u[0] = areas[0] * (p - pressure[0]) / nodes[0];
            for j in 1..n {
                d.u[j] = areas[j] * (pressure[j - 1] - pressure[j]) / nodes[j];
            }
            d.u[n] = areas[n] * pressure[n - 1] / nodes[n];
            // Ablation is of fuel. Once the fuel is gone and the ablator is
            // reached, the conducted heat stays in the hot spot and the alpha
            // energy not deposited there heats the ablator's inner zone.
            let fuel = s.fuel_zones > 0;
            let conduction_mass = if self.physics.conduction && fuel {
                ABLATION_COEFFICIENT * FUEL_ION_PAIR_MASS_KG * a * conductivity_coefficient() * t_j * t_j * t_j.sqrt() / logarithm
            } else {
                0.0
            };
            let ablation = if fuel { conduction_mass + FUEL_ION_PAIR_MASS_KG * alpha_rest / (5.0 * t_j) } else { 0.0 };
            if !fuel {
                d.energy[0] += alpha_rest;
            }
            let rho0 = s.mass[0] / s.zone_volume(0);
            let specific = s.energy[0] / s.mass[0];
            d.r[0] += ablation / (areas[0] * rho0);
            d.mass[0] = -ablation;
            d.energy[0] -= specific * ablation;
            d.fuel_ions += ablation / FUEL_ION_PAIR_MASS_KG;
            // Node 0 gains 0.6 of the ablated mass (the hot spot's inertia) and loses half of it (zone 1's half); node 1 loses the other half.
            let kinetic_change = 0.5 * s.u[0] * s.u[0] * (0.6 - 0.5) * ablation - 0.5 * s.u[1] * s.u[1] * 0.5 * ablation;
            let brought = if self.fault == Fault::AblationLosesEnergy { -kinetic_change } else { specific * ablation - kinetic_change };
            if fuel { (alpha_rest, 0.0, conduction_mass, ablation, brought) } else { (0.0, 0.0, 0.0, 0.0, 0.0) }
        } else {
            d.u[0] = 4.0 * PI * a * a * p / (0.6 * hot_mass);
            (0.0, alpha_rest, 0.0, 0.0, 0.0)
        };
        let work_done = if self.fault == Fault::NoExpansionWork { 0.0 } else { work };
        d.hot_energy = alpha_hot + alpha_shell - bremsstrahlung - work_done + brought_in;
        let conduction_w = 5.0 * t_j * conduction_mass / FUEL_ION_PAIR_MASS_KG;
        d.accounts[A_REACTIONS] = reactions;
        d.accounts[A_ALPHA] = alpha_w;
        d.accounts[A_ALPHA_HOT] = alpha_hot;
        d.accounts[A_ALPHA_SHELL] = alpha_shell;
        d.accounts[A_ALPHA_ESCAPED] = alpha_escaped;
        d.accounts[A_BREMSSTRAHLUNG] = escaping;
        d.accounts[A_RADIATION_TO_SHELL] = bremsstrahlung - escaping;
        d.accounts[A_CARRIED] = carried;
        d.accounts[A_CONDUCTION] = conduction_w;
        d.accounts[A_COMPRESSION] = (-work).max(0.0);
        d.accounts[A_EXPANSION] = work.max(0.0);
        d.accounts[A_ABLATED] = ablation;
        if n > 0 && s.fuel_zones == 0 {
            d.accounts[A_ALPHA_ABLATOR] = alpha_rest;
        }
        let rates = Rates {
            reactions_per_s: reactions,
            temperature_kev: t_kev,
            pressure_pa: p,
            alpha_to_hot_spot_w: alpha_hot,
            conduction_w,
            bremsstrahlung_w: bremsstrahlung,
            ablation_kg_per_s: ablation,
        };
        (d, rates)
    }

    /// The alpha share deposited in the hot spot and the fate of the
    /// bremsstrahlung, at this state. The hot spot emits the spectrum
    /// `e^(−hν/T)` per unit photon energy; a photon of energy `hν` leaves it
    /// with [`sphere_escape_probability`] of its own free-free optical radius
    /// and crosses the shell zones radially, each absorbing `1 − e^(−α Δr)` of
    /// what reaches it at the zone's own temperature, `m̄ ε/3`.
    fn transport(&self, s: &State) -> Transport {
        let n = s.zones();
        let volume = s.hot_volume();
        let alpha_share = if self.physics.alpha_heating {
            let path = alpha_path_m(s.electron_density(), s.temperature_kev(), s.fuel_ions / volume, s.alphas / volume);
            alpha_deposition_fraction(s.r[0] / path)
        } else {
            0.0
        };
        let mut zone_share = vec![0.0; n];
        let mut leaves = 0.0;
        let mut escapes = 0.0;
        if self.physics.radiation {
            let t = s.temperature_kev();
            let ne = s.electron_density();
            let z2 = (s.fuel_ions + 4.0 * s.alphas) / volume;
            // (electron density, charge squared times ion density, temperature, thickness) of each zone.
            let zones: Vec<(f64, f64, f64, f64)> = (0..n)
                .map(|i| {
                    let (ions, electrons, temperature) = s.zone_plasma(i);
                    let charge = if i < s.fuel_zones { 1.0 } else { ABLATOR_CHARGE };
                    (electrons, charge * charge * ions, temperature, s.r[i + 1] - s.r[i])
                })
                .collect();
            let width = PHOTON_ENERGY_MAX_OVER_T / PHOTON_PANELS as f64;
            for panel in 0..PHOTON_PANELS {
                let middle = (panel as f64 + 0.5) * width;
                for (node, weight) in reaction::GAUSS_NODES.iter().zip(reaction::GAUSS_WEIGHTS) {
                    let x = middle + 0.5 * width * node;
                    let w = 0.5 * width * weight * math::exp(-x);
                    let photon = x * t;
                    let out = sphere_escape_probability(free_free_absorption_per_m(ne, z2, t, photon) * s.r[0]);
                    leaves += w * out;
                    let mut carried = out;
                    for (i, (electrons, charged, zone_t, thickness)) in zones.iter().enumerate() {
                        let kept = -math::exp_m1(-free_free_absorption_per_m(*electrons, *charged, *zone_t, photon) * thickness);
                        zone_share[i] += w * carried * kept;
                        carried *= 1.0 - kept;
                    }
                    escapes += w * carried;
                }
            }
        }
        Transport { alpha_share, leaves_hot_spot: leaves, zone_share, escapes }
    }

    /// The innermost zone joins the hot spot whole: its mass and its internal
    /// energy go into the hot spot, and the new boundary node, the old node
    /// 1, takes the speed that keeps the kinetic energy of the nodes: the
    /// kinetic energy of the old boundary node and of node 1 is given to the
    /// new boundary node with its new mass, in node 1's direction.
    fn join_zone(&self, s: &State) -> State {
        let before = s.kinetic();
        let mut next = State {
            r: s.r[1..].to_vec(),
            u: s.u[1..].to_vec(),
            mass: s.mass[1..].to_vec(),
            energy: s.energy[1..].to_vec(),
            hot_energy: s.hot_energy + s.energy[0],
            fuel_ions: s.fuel_ions + s.mass[0] / FUEL_ION_PAIR_MASS_KG,
            alphas: s.alphas,
            fuel_zones: s.fuel_zones - 1,
            accounts: s.accounts,
        };
        next.accounts[A_ABLATED] += s.mass[0];
        let nodes = next.node_masses();
        let others: f64 = nodes.iter().zip(&next.u).skip(1).map(|(m, u)| 0.5 * m * u * u).sum();
        let left = (before - others).max(0.0);
        let speed = (2.0 * left / nodes[0]).sqrt();
        next.u[0] = if s.u[1] < 0.0 { -speed } else { speed };
        // What rounding leaves of the kinetic energy goes to the hot spot's heat.
        next.hot_energy += before - next.kinetic();
        next
    }

    /// One step of the classical fourth-order Runge-Kutta rule, or why it
    /// cannot be taken: a stage leaves the reactivity fit while the burn is
    /// on (its temperature returned), or a value is not finite or a zone or
    /// the hot spot is left without volume, mass or energy.
    fn try_step(&self, s: &State, dt: f64, transport: &Transport) -> Result<State, Option<f64>> {
        let admissible = |x: &State| -> Result<(), Option<f64>> {
            let t = x.temperature_kev();
            if !(x.hot_energy > 0.0 && x.fuel_ions > 0.0 && t.is_finite()) {
                return Err(None);
            }
            if self.physics.burn && !(TEMPERATURE_MIN_KEV..=TEMPERATURE_MAX_KEV).contains(&t) {
                return Err(Some(t));
            }
            let whole = x.energy.iter().all(|e| *e > 0.0)
                && x.mass.iter().all(|m| *m > 0.0)
                && x.r.windows(2).all(|pair| pair[1] > pair[0])
                && x.r[0] > 0.0;
            if whole { Ok(()) } else { Err(None) }
        };
        let finite = |d: &State| -> Result<(), Option<f64>> {
            let ok = d.hot_energy.is_finite()
                && d.u.iter().all(|v| v.is_finite())
                && d.energy.iter().all(|v| v.is_finite())
                && d.r.iter().all(|v| v.is_finite());
            if ok { Ok(()) } else { Err(None) }
        };
        let (k1, _) = self.rhs(s, transport);
        finite(&k1)?;
        let a = s.plus(&k1, 0.5 * dt);
        admissible(&a)?;
        let (k2, _) = self.rhs(&a, transport);
        finite(&k2)?;
        let b = s.plus(&k2, 0.5 * dt);
        admissible(&b)?;
        let (k3, _) = self.rhs(&b, transport);
        finite(&k3)?;
        let c = s.plus(&k3, dt);
        admissible(&c)?;
        let (k4, _) = self.rhs(&c, transport);
        finite(&k4)?;
        let mut next = s.plus(&k1, dt / 6.0);
        next = next.plus(&k2, dt / 3.0);
        next = next.plus(&k3, dt / 3.0);
        let next = next.plus(&k4, dt / 6.0);
        admissible(&next)?;
        Ok(next)
    }

    /// The step, halved until its stages stay where the rates are defined,
    /// at most [`STEP_HALVINGS_MAX`] times. Returns the state and the step
    /// taken, or the temperature outside the fit that the last try reached.
    fn step(&self, s: &State, dt: f64, transport: &Transport) -> Result<(State, f64), Option<f64>> {
        let mut h = dt;
        let mut why = None;
        for _ in 0..=STEP_HALVINGS_MAX {
            match self.try_step(s, h, transport) {
                Ok(next) => return Ok((next, h)),
                Err(reason) => why = reason,
            }
            h *= 0.5;
        }
        Err(why)
    }

    /// The shortest time of the state, s.
    fn time_scale(&self, s: &State, rates: &Rates, d: &State) -> f64 {
        let hot_sound = (5.0 / 3.0 * rates.pressure_pa * s.hot_volume() / s.hot_mass()).sqrt();
        let mut shortest = s.r[0] / hot_sound;
        // The hot spot's boundary may move or turn faster than sound crosses it once the shell is thin or gone.
        shortest = shortest.min(s.r[0] / s.u[0].abs());
        if d.u[0] != 0.0 {
            shortest = shortest.min((s.r[0] / d.u[0].abs()).sqrt());
        }
        for i in 0..s.zones() {
            let v = s.zone_volume(i);
            let rho = s.mass[i] / v;
            let sound = (5.0 / 3.0 * 2.0 / 3.0 * s.energy[i] / v / rho).sqrt();
            let du = (s.u[i + 1] - s.u[i]).abs();
            shortest = shortest.min((s.r[i + 1] - s.r[i]) / (sound + 2.0 * VISCOSITY_QUADRATIC * du));
        }
        if s.zones() > 0 && rates.ablation_kg_per_s > 0.0 {
            shortest = shortest.min(s.mass[0] / rates.ablation_kg_per_s);
        }
        if rates.reactions_per_s > 0.0 {
            shortest = shortest.min(s.fuel_ions / (2.0 * rates.reactions_per_s));
        }
        let heating = (d.hot_energy + 4.0 * PI * s.r[0] * s.r[0] * rates.pressure_pa * s.u[0]).abs();
        if heating > 0.0 {
            shortest = shortest.min(s.hot_energy / heating);
        }
        shortest
    }
}

/// The question's numbers in SI and the state at the ideal peak compression.
struct Stagnation {
    state: State,
    zone_mass: f64,
    fuel_total: f64,
    shell_density: f64,
    hot_mass: f64,
    ablator_density: f64,
}

/// The named limits the question itself shows. A value the question's
/// numbers give at peak compression that is not finite (a hot spot so small
/// its volume is zero in 64 bits, its density 0/0) is no limit the question
/// shows: the question is admitted, and the solve refuses it by its own
/// name, [`HotSpotReject::NonFiniteValue`] at 0 ps, sealed, as any value of
/// the burn that is not finite.
pub fn admit(question: &HotSpotQuestion) -> Result<(), HotSpotReject> {
    match stagnation(question) {
        Ok(_) | Err(HotSpotReject::NonFiniteValue { .. }) => Ok(()),
        Err(reject) => Err(reject),
    }
}

/// `value`, when it is finite; otherwise [`HotSpotReject::NonFiniteValue`]
/// at peak compression, 0 ps, before a comparison reads it.
fn finite_at_stagnation(value: f64) -> Result<f64, HotSpotReject> {
    if value.is_finite() { Ok(value) } else { Err(HotSpotReject::NonFiniteValue { time_ps: 0.0 }) }
}

fn stagnation(question: &HotSpotQuestion) -> Result<Stagnation, HotSpotReject> {
    let q = *question;
    for value in [
        q.hot_spot_temperature_kev,
        q.hot_spot_pressure_gbar,
        q.hot_spot_radius_um,
        q.shell_areal_density_g_per_cm2,
        q.fuel_mass_mg,
        q.ablator_mass_mg,
    ] {
        if !value.is_finite() {
            return Err(HotSpotReject::InputNotFinite);
        }
    }
    if !(TEMPERATURE_MIN_KEV..=TEMPERATURE_MAX_KEV).contains(&q.hot_spot_temperature_kev) {
        return Err(HotSpotReject::TemperatureOutsideReactivityFit { temperature_kev: q.hot_spot_temperature_kev });
    }
    if !(q.hot_spot_pressure_gbar > 0.0) {
        return Err(HotSpotReject::PressureNotPositive { pressure_gbar: q.hot_spot_pressure_gbar });
    }
    if !(q.ablator_mass_mg >= 0.0) {
        return Err(HotSpotReject::AblatorMassNegative { ablator_mass_mg: q.ablator_mass_mg });
    }
    if !(q.hot_spot_radius_um > 0.0) {
        return Err(HotSpotReject::RadiusNotPositive { radius_um: q.hot_spot_radius_um });
    }
    if !(q.shell_areal_density_g_per_cm2 > 0.0) {
        return Err(HotSpotReject::ShellArealDensityNotPositive { areal_density_g_per_cm2: q.shell_areal_density_g_per_cm2 });
    }
    for (name, value, most) in [
        ("hot-spot radius in um", q.hot_spot_radius_um, RADIUS_MAX_UM),
        ("shell areal density in g/cm2", q.shell_areal_density_g_per_cm2, SHELL_AREAL_DENSITY_MAX_G_PER_CM2),
        ("fuel mass in mg", q.fuel_mass_mg, FUEL_MASS_MAX_MG),
        ("ablator mass in mg", q.ablator_mass_mg, FUEL_MASS_MAX_MG),
    ] {
        if value > most {
            return Err(HotSpotReject::InputTooLargeForArithmetic { name, value });
        }
    }
    let stagnation_density = q.hot_spot_pressure_gbar * PA_PER_GBAR / (2.0 * q.hot_spot_temperature_kev * JOULE_PER_KEV);
    // Each value a comparison below reads is finite, or it is refused as
    // itself before the comparison reads it (the breadth tests of 16 October
    // 2026: a NaN density reached ShellNotDenserThanHotSpot).
    let stagnation_fermi = finite_at_stagnation(fermi_energy_kev(stagnation_density))?;
    if !(q.hot_spot_temperature_kev > stagnation_fermi) {
        return Err(HotSpotReject::StagnationDegenerate { temperature_kev: q.hot_spot_temperature_kev, fermi_energy_kev: stagnation_fermi });
    }
    let p = q.hot_spot_pressure_gbar * PA_PER_GBAR;
    let radius = q.hot_spot_radius_um * 1.0e-6;
    let volume = 4.0 / 3.0 * PI * radius * radius * radius;
    let fuel_ions = p / (2.0 * q.hot_spot_temperature_kev * JOULE_PER_KEV) * volume;
    let hot_mass = finite_at_stagnation(fuel_ions * FUEL_ION_PAIR_MASS_KG)?;
    let fuel_mass = q.fuel_mass_mg * 1.0e-6;
    let shell_mass = finite_at_stagnation(fuel_mass - hot_mass)?;
    if !(shell_mass > 0.0) {
        return Err(HotSpotReject::FuelMassNotAboveHotSpotMass { fuel_mass_mg: q.fuel_mass_mg, hot_spot_mass_mg: hot_mass * 1.0e6 });
    }
    let sigma = q.shell_areal_density_g_per_cm2 * KG_PER_M2_PER_G_PER_CM2;
    let least = finite_at_stagnation(4.0 * PI * radius * radius * sigma)?;
    if !(shell_mass > least) {
        return Err(HotSpotReject::ShellMassBelowArealDensity { shell_mass_mg: shell_mass * 1.0e6, least_mg: least * 1.0e6 });
    }
    // A uniform shell of thickness d: M_s = 4π σ (R² + R d + d²/3), so d² + 3 R d + 3 R² − 3 M_s/(4π σ) = 0.
    let thickness = 0.5 * (-3.0 * radius + (3.0 * shell_mass / (PI * sigma) - 3.0 * radius * radius).sqrt());
    let shell_density = finite_at_stagnation(sigma / thickness)?;
    let hot_density = finite_at_stagnation(hot_mass / volume)?;
    if !(shell_density > hot_density) {
        return Err(HotSpotReject::ShellNotDenserThanHotSpot {
            shell_density_g_per_cm3: shell_density / 1.0e3,
            hot_spot_density_g_per_cm3: hot_density / 1.0e3,
        });
    }
    let zone_mass = shell_mass / SHELL_ZONES as f64;
    let mut r = Vec::with_capacity(SHELL_ZONES + 1);
    r.push(radius);
    for i in 0..SHELL_ZONES {
        let inner: f64 = r[i];
        r.push(math::cbrt(inner * inner * inner + 3.0 * zone_mass / (4.0 * PI * shell_density)));
    }
    let mut mass = vec![zone_mass; SHELL_ZONES];
    // The ablator outside, at the shell's pressure and temperature: carbon
    // fully ionized has 1 + Z particles per ion against the fuel's 2, so its
    // density is the shell's times (m_C/(1 + Z)) / (m̄/2).
    let ablator_density = shell_density * (ABLATOR_ION_PAIR_MASS_KG / (1.0 + ABLATOR_CHARGE)) / (FUEL_ION_PAIR_MASS_KG / 2.0);
    let ablator_mass = q.ablator_mass_mg * 1.0e-6;
    let ablator_zones = if ablator_mass > 0.0 { ABLATOR_ZONES } else { 0 };
    for _ in 0..ablator_zones {
        let inner: f64 = *r.last().expect("nodes");
        let zone = ablator_mass / ABLATOR_ZONES as f64;
        r.push(math::cbrt(inner * inner * inner + 3.0 * zone / (4.0 * PI * ablator_density)));
        mass.push(zone);
    }
    let zones = SHELL_ZONES + ablator_zones;
    let energy = (0..zones).map(|i| 1.5 * p * 4.0 / 3.0 * PI * (math::powi(r[i + 1], 3) - math::powi(r[i], 3))).collect();
    let state = State {
        r,
        u: vec![0.0; zones + 1],
        mass,
        energy,
        hot_energy: 1.5 * p * volume,
        fuel_ions,
        alphas: 0.0,
        fuel_zones: SHELL_ZONES,
        accounts: [0.0; ACCOUNTS],
    };
    Ok(Stagnation { state, zone_mass, fuel_total: fuel_mass / FUEL_ION_PAIR_MASS_KG, shell_density, hot_mass, ablator_density })
}

/// One run of the model.
struct Run {
    end: State,
    history: Vec<Sample>,
    energy_residual: f64,
    particle_residual: f64,
    start_energy: f64,
    steps: usize,
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    time_s: f64,
    rates: Rates,
    radius_m: f64,
    hot_areal_density: f64,
    shell_areal_density: f64,
    hot_mass: f64,
    hot_energy: f64,
    alpha_hot_cumulative: f64,
    compression_cumulative: f64,
}

fn ps(time_s: f64) -> f64 {
    time_s * 1.0e12
}

/// How a run ends.
#[derive(Clone, Copy)]
enum Until {
    /// The burn is over: the rate has fallen to `END_RATE_FRACTION` of its peak.
    BurnOver,
    /// The ideal motion's diagnostic rate has fallen to this many reactions per second.
    IdealRateBelow(f64),
}

fn run(model: &Model, start: &State, start_time: f64, courant: f64, until: Until) -> Result<Run, HotSpotReject> {
    let mut s = start.clone();
    let mut t = start_time;
    let start_energy = s.total_energy();
    let mut history = Vec::new();
    let mut peak: f64 = 0.0;
    let mut energy_residual: f64 = 0.0;
    let mut particle_residual: f64 = 0.0;
    let mut steps = 0;
    loop {
        while s.fuel_zones > 0 && s.mass[0] < ZONE_JOIN_FRACTION * model.zone_mass {
            s = model.join_zone(&s);
        }
        // The physics at this state.
        let t_kev = s.temperature_kev();
        if !t_kev.is_finite() || !s.hot_energy.is_finite() {
            return Err(HotSpotReject::NonFiniteValue { time_ps: ps(t) });
        }
        if !(s.hot_energy > 0.0) || s.energy.iter().any(|e| !(*e > 0.0)) {
            return Err(HotSpotReject::InternalEnergyNotPositive { time_ps: ps(t) });
        }
        for i in 0..s.zones() {
            if !(s.r[i + 1] > s.r[i]) {
                return Err(HotSpotReject::ShellMeshTangled { time_ps: ps(t) });
            }
        }
        let diagnostic = match until {
            Until::IdealRateBelow(_) => {
                if !(TEMPERATURE_MIN_KEV..=TEMPERATURE_MAX_KEV).contains(&t_kev) {
                    return Err(HotSpotReject::TemperatureLeftReactivityFit { time_ps: ps(t), temperature_kev: t_kev });
                }
                let density = s.fuel_ions / s.hot_volume();
                0.25 * density * density * bosch_hale::reactivity_m3_per_s(Reaction::DeuteriumTritium, t_kev).unwrap_or(0.0) * s.hot_volume()
            }
            Until::BurnOver => 0.0,
        };
        if model.physics.burn && !(TEMPERATURE_MIN_KEV..=TEMPERATURE_MAX_KEV).contains(&t_kev) {
            return Err(HotSpotReject::TemperatureLeftReactivityFit { time_ps: ps(t), temperature_kev: t_kev });
        }
        let ne = s.electron_density();
        let fermi = fermi_energy_kev(ne);
        if !(t_kev > fermi) {
            return Err(HotSpotReject::HotSpotDegenerate { time_ps: ps(t), temperature_kev: t_kev, fermi_energy_kev: fermi });
        }
        let transport = model.transport(&s);
        let f_alpha = transport.alpha_share;
        let (d, rates) = model.rhs(&s, &transport);
        if !rates.reactions_per_s.is_finite() || !f_alpha.is_finite() || !transport.leaves_hot_spot.is_finite() {
            return Err(HotSpotReject::NonFiniteValue { time_ps: ps(t) });
        }
        // The ledgers.
        let a = &s.accounts;
        let scale = start_energy.max(a[A_ALPHA]);
        let residual = (s.total_energy() + a[A_BREMSSTRAHLUNG] + a[A_ALPHA_ESCAPED] + a[A_CARRIED] - a[A_ALPHA] - start_energy) / scale;
        if !residual.is_finite() {
            return Err(HotSpotReject::NonFiniteValue { time_ps: ps(t) });
        }
        energy_residual = energy_residual.max(residual.abs());
        if !(residual.abs() <= ENERGY_TOLERANCE) {
            return Err(HotSpotReject::EnergyLedgerBroken { time_ps: ps(t), residual });
        }
        let shell_ions: f64 = s.mass[..s.fuel_zones].iter().sum::<f64>() / FUEL_ION_PAIR_MASS_KG;
        let ions = (s.fuel_ions + shell_ions + 2.0 * a[A_REACTIONS] - model.fuel_total) / model.fuel_total;
        let ash = (s.alphas - a[A_REACTIONS]) / model.fuel_total;
        let burn_up = 2.0 * a[A_REACTIONS] / model.fuel_total;
        let particle = ions.abs().max(ash.abs());
        particle_residual = particle_residual.max(particle);
        if !(particle <= PARTICLE_TOLERANCE) || !(burn_up <= 1.0) || !(s.fuel_ions > 0.0) {
            return Err(HotSpotReject::ParticleLedgerBroken { time_ps: ps(t), residual: particle });
        }
        let volume = s.hot_volume();
        history.push(Sample {
            time_s: t,
            rates,
            radius_m: s.r[0],
            hot_areal_density: s.hot_mass() / volume * s.r[0],
            shell_areal_density: s.shell_areal_density(),
            hot_mass: s.hot_mass(),
            hot_energy: s.hot_energy,
            alpha_hot_cumulative: a[A_ALPHA_HOT] + a[A_ALPHA_SHELL],
            compression_cumulative: a[A_COMPRESSION],
        });
        // The end.
        match until {
            Until::IdealRateBelow(rate) => {
                if diagnostic <= rate {
                    break;
                }
            }
            Until::BurnOver => {
                peak = peak.max(rates.reactions_per_s);
                let falling = history.len() > 1 && rates.reactions_per_s < history[history.len() - 2].rates.reactions_per_s;
                if peak > 0.0 && falling && rates.reactions_per_s <= END_RATE_FRACTION * peak {
                    break;
                }
            }
        }
        if steps >= MAX_STEPS {
            return Err(HotSpotReject::StepLimitReached { steps });
        }
        let dt = courant * model.time_scale(&s, &rates, &d);
        if !(dt > 0.0) || !dt.is_finite() {
            return Err(HotSpotReject::NonFiniteValue { time_ps: ps(t) });
        }
        // A step that cannot be taken within the halvings is the state's own
        // limit: name it by what the last state shows.
        let (next, taken) = match model.step(&s, dt, &transport) {
            Ok(taken) => taken,
            Err(Some(temperature_kev)) => {
                return Err(HotSpotReject::TemperatureLeftReactivityFit { time_ps: ps(t), temperature_kev });
            }
            Err(None) => return Err(HotSpotReject::NonFiniteValue { time_ps: ps(t) }),
        };
        s = next;
        t += taken;
        steps += 1;
    }
    Ok(Run { end: s, history, energy_residual, particle_residual, start_energy, steps })
}

/// The burn of a stagnated hot spot.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &HotSpotQuestion) -> Result<HotSpotBurn, HotSpotReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &HotSpotQuestion) -> Result<HotSpotBurn, HotSpotReject> {
    solve_with(question, Fault::None)
}

impl NormalOrZero for HotSpotBurn {
    fn normal_or_zero(&mut self) {
        self.yield_mj.normal_or_zero();
        self.neutron_yield.normal_or_zero();
        self.yield_without_alpha_heating_mj.normal_or_zero();
        self.yield_amplification.normal_or_zero();
        self.burn_up_fraction.normal_or_zero();
        self.burn_width_ps.normal_or_zero();
        self.bang_time_ps.normal_or_zero();
        self.peak_fusion_power_w.normal_or_zero();
        self.burn_averaged_temperature_kev.normal_or_zero();
        self.burn_averaged_pressure_gbar.normal_or_zero();
        self.burn_averaged_radius_um.normal_or_zero();
        self.burn_averaged_hot_spot_areal_density_g_per_cm2.normal_or_zero();
        self.burn_averaged_shell_areal_density_g_per_cm2.normal_or_zero();
        self.hot_spot_mass_at_bang_mg.normal_or_zero();
        self.hot_spot_energy_at_bang_kj.normal_or_zero();
        self.peak_temperature_kev.normal_or_zero();
        self.peak_pressure_gbar.normal_or_zero();
        self.fuel_ablated_fraction.normal_or_zero();
        self.burned_through.normal_or_zero();
        self.self_heating_ratio_at_bang.normal_or_zero();
        self.self_heating_ratio_peak.normal_or_zero();
        self.alpha_energy_over_compression_work_at_bang.normal_or_zero();
        self.alpha_energy_over_compression_work.normal_or_zero();
        self.lawson_parameter_no_alpha.normal_or_zero();
        self.lawson_parameter_pressure_time.normal_or_zero();
        self.lawson_criterion_lindl.normal_or_zero();
        self.alpha_energy_over_twice_hot_spot_energy.normal_or_zero();
        self.q_alpha_hot_spot.normal_or_zero();
        self.alpha_energy_mj.normal_or_zero();
        self.alpha_energy_in_hot_spot_mj.normal_or_zero();
        self.alpha_energy_through_shell_mj.normal_or_zero();
        self.alpha_energy_escaped_mj.normal_or_zero();
        self.alpha_energy_in_ablator_mj.normal_or_zero();
        self.neutron_energy_mj.normal_or_zero();
        self.bremsstrahlung_mj.normal_or_zero();
        self.bremsstrahlung_absorbed_in_shell_mj.normal_or_zero();
        self.conduction_returned_mj.normal_or_zero();
        self.compression_work_mj.normal_or_zero();
        self.expansion_work_mj.normal_or_zero();
        self.kinetic_energy_carried_by_products_mj.normal_or_zero();
        self.energy_at_start_mj.normal_or_zero();
        self.energy_at_end_mj.normal_or_zero();
        self.energy_residual.normal_or_zero();
        self.particle_residual.normal_or_zero();
        self.start_time_ps.normal_or_zero();
        self.shell_density_g_per_cm3.normal_or_zero();
        self.ablator_density_g_per_cm3.normal_or_zero();
        self.stagnation_hot_spot_mass_mg.normal_or_zero();
        self.history.normal_or_zero();
    }
}

/// The start of the burn: the ideal motion run forward from peak compression
/// until its fusion rate is `START_RATE_FRACTION` of the rate there, every
/// velocity reversed.
fn burn_start(stag: &Stagnation, courant: f64) -> Result<(State, f64), HotSpotReject> {
    let ideal = Model { physics: IDEAL, fault: Fault::None, zone_mass: stag.zone_mass, fuel_total: stag.fuel_total };
    let s = &stag.state;
    let density = s.fuel_ions / s.hot_volume();
    let rate0 = 0.25 * density * density
        * bosch_hale::reactivity_m3_per_s(Reaction::DeuteriumTritium, s.temperature_kev()).unwrap_or(f64::NAN)
        * s.hot_volume();
    let out = run(&ideal, s, 0.0, courant, Until::IdealRateBelow(START_RATE_FRACTION * rate0))?;
    let elapsed = out.history.last().map_or(0.0, |h| h.time_s);
    let mut start = out.end;
    for u in &mut start.u {
        *u = -*u;
    }
    start.accounts = [0.0; ACCOUNTS];
    Ok((start, -elapsed))
}

struct Summary {
    reactions: f64,
    burn_width_s: f64,
    bang: usize,
}

fn summarize(out: &Run) -> Summary {
    let h = &out.history;
    let mut bang = 0;
    for (i, sample) in h.iter().enumerate() {
        if sample.rates.reactions_per_s > h[bang].rates.reactions_per_s {
            bang = i;
        }
    }
    let half = 0.5 * h[bang].rates.reactions_per_s;
    let crossing = |i: usize, j: usize| {
        let (a, b) = (&h[i], &h[j]);
        a.time_s + (half - a.rates.reactions_per_s) * (b.time_s - a.time_s) / (b.rates.reactions_per_s - a.rates.reactions_per_s)
    };
    let first = h.iter().position(|x| x.rates.reactions_per_s >= half).unwrap_or(0);
    let last = h.iter().rposition(|x| x.rates.reactions_per_s >= half).unwrap_or(h.len() - 1);
    let rise = if first == 0 { h[0].time_s } else { crossing(first - 1, first) };
    let fall = if last + 1 >= h.len() { h[last].time_s } else { crossing(last, last + 1) };
    Summary { reactions: out.end.accounts[A_REACTIONS], burn_width_s: fall - rise, bang }
}

/// The neutron-weighted mean of a quantity over the history, by the trapezoid rule in time.
fn burn_average(h: &[Sample], quantity: impl Fn(&Sample) -> f64) -> f64 {
    let mut num = 0.0;
    let mut den = 0.0;
    for pair in h.windows(2) {
        let dt = pair[1].time_s - pair[0].time_s;
        num += 0.5 * dt * (pair[0].rates.reactions_per_s * quantity(&pair[0]) + pair[1].rates.reactions_per_s * quantity(&pair[1]));
        den += 0.5 * dt * (pair[0].rates.reactions_per_s + pair[1].rates.reactions_per_s);
    }
    num / den
}

fn solve_with(question: &HotSpotQuestion, fault: Fault) -> Result<HotSpotBurn, HotSpotReject> {
    let stag = stagnation(question)?;
    let model = |physics| Model { physics, fault, zone_mass: stag.zone_mass, fuel_total: stag.fuel_total };
    // The run at the Courant number and at half of it; where the two differ
    // by more than the tolerance, both are halved again, at most
    // `MAX_REFINEMENTS` times.
    let mut courant = COURANT;
    let (start, start_time) = burn_start(&stag, courant)?;
    let mut coarse = run(&model(FULL), &start, start_time, courant, Until::BurnOver)?;
    let mut refinements = 0;
    let (fine, start_fine, start_time_fine) = loop {
        let (start_fine, start_time_fine) = burn_start(&stag, 0.5 * courant)?;
        let fine = run(&model(FULL), &start_fine, start_time_fine, 0.5 * courant, Until::BurnOver)?;
        let sc = summarize(&coarse);
        let sf = summarize(&fine);
        let yield_change = (sf.reactions - sc.reactions).abs() / sf.reactions;
        let width_change = (sf.burn_width_s - sc.burn_width_s).abs() / sf.burn_width_s;
        if yield_change <= SETTLE_TOLERANCE && width_change <= SETTLE_TOLERANCE {
            break (fine, start_fine, start_time_fine);
        }
        if refinements >= MAX_REFINEMENTS {
            return Err(HotSpotReject::NotSettled { yield_change, burn_width_change: width_change });
        }
        refinements += 1;
        courant *= 0.5;
        coarse = fine;
    };
    let no_alpha = run(&model(NO_ALPHA), &start_fine, start_time_fine, 0.5 * courant, Until::BurnOver)?;
    let sf = summarize(&fine);
    let sn = summarize(&no_alpha);
    let q_j = Reaction::DeuteriumTritium.energy_kev() * JOULE_PER_KEV;
    let h = &fine.history;
    let bang = &h[sf.bang];
    let a = &fine.end.accounts;
    let mj = |joules: f64| joules / J_PER_MJ;
    let temperature = burn_average(h, |x| x.rates.temperature_kev);
    let pressure = burn_average(h, |x| x.rates.pressure_pa);
    let ratio = |x: &Sample| x.rates.alpha_to_hot_spot_w / (x.rates.bremsstrahlung_w + x.rates.conduction_w);
    let self_heating_peak = h.iter().map(ratio).fold(0.0, f64::max);
    // Betti et al. (2015): the no-alpha run's neutron-averaged total areal density, its yield in 1e16 neutrons, the fuel mass in mg.
    let total_areal = |x: &Sample| (x.hot_areal_density + x.shell_areal_density) / KG_PER_M2_PER_G_PER_CM2;
    let areal_no_alpha = burn_average(&no_alpha.history, total_areal);
    let lawson_no_alpha = math::powf(areal_no_alpha, 0.61) * math::powf(0.24 * sn.reactions / 1.0e16 / question.fuel_mass_mg, 0.34);
    let reactivity = bosch_hale::reactivity_m3_per_s(Reaction::DeuteriumTritium, temperature).unwrap_or(f64::NAN);
    let t_j = temperature * JOULE_PER_KEV;
    let lawson_pressure_time =
        pressure * sf.burn_width_s * reactivity * alpha_birth_energy_kev() * JOULE_PER_KEV / (24.0 * t_j * t_j);
    // Every n-th step, n the least that leaves at most HISTORY_ROWS_MAX rows, with the bang step and the last step.
    let stride = h.len().div_ceil(HISTORY_ROWS_MAX - 2).max(1);
    let mut rows: Vec<usize> = (0..h.len()).step_by(stride).collect();
    rows.push(sf.bang);
    rows.push(h.len() - 1);
    rows.sort_unstable();
    rows.dedup();
    let history = rows
        .iter()
        .map(|&i| &h[i])
        .map(|x| [ps(x.time_s), x.rates.reactions_per_s * q_j, x.rates.temperature_kev, x.rates.pressure_pa / PA_PER_GBAR, x.radius_m * 1.0e6])
        .collect();
    let energy_residual = coarse.energy_residual.max(fine.energy_residual).max(no_alpha.energy_residual);
    let particle_residual = coarse.particle_residual.max(fine.particle_residual).max(no_alpha.particle_residual);
    Ok(HotSpotBurn {
        yield_mj: mj(sf.reactions * q_j),
        neutron_yield: sf.reactions,
        yield_without_alpha_heating_mj: mj(sn.reactions * q_j),
        yield_amplification: sf.reactions / sn.reactions,
        burn_up_fraction: 2.0 * sf.reactions / stag.fuel_total,
        burn_width_ps: ps(sf.burn_width_s),
        bang_time_ps: ps(bang.time_s),
        peak_fusion_power_w: bang.rates.reactions_per_s * q_j,
        burn_averaged_temperature_kev: temperature,
        burn_averaged_pressure_gbar: pressure / PA_PER_GBAR,
        burn_averaged_radius_um: burn_average(h, |x| x.radius_m) * 1.0e6,
        burn_averaged_hot_spot_areal_density_g_per_cm2: burn_average(h, |x| x.hot_areal_density) / KG_PER_M2_PER_G_PER_CM2,
        burn_averaged_shell_areal_density_g_per_cm2: burn_average(h, |x| x.shell_areal_density) / KG_PER_M2_PER_G_PER_CM2,
        hot_spot_mass_at_bang_mg: bang.hot_mass * 1.0e6,
        hot_spot_energy_at_bang_kj: bang.hot_energy / 1.0e3,
        peak_temperature_kev: h.iter().map(|x| x.rates.temperature_kev).fold(0.0, f64::max),
        peak_pressure_gbar: h.iter().map(|x| x.rates.pressure_pa).fold(0.0, f64::max) / PA_PER_GBAR,
        fuel_ablated_fraction: a[A_ABLATED] / (question.fuel_mass_mg * 1.0e-6),
        burned_through: if fine.end.fuel_zones == 0 { 1.0 } else { 0.0 },
        self_heating_ratio_at_bang: ratio(bang),
        self_heating_ratio_peak: self_heating_peak,
        alpha_energy_over_compression_work_at_bang: bang.alpha_hot_cumulative / bang.compression_cumulative,
        alpha_energy_over_compression_work: (a[A_ALPHA_HOT] + a[A_ALPHA_SHELL]) / a[A_COMPRESSION],
        lawson_parameter_no_alpha: lawson_no_alpha,
        lawson_parameter_pressure_time: lawson_pressure_time,
        lawson_criterion_lindl: (pressure / PA_PER_GBAR / 420.0) * (burn_average(h, |x| x.radius_m) * 1.0e6 / 50.0),
        alpha_energy_over_twice_hot_spot_energy: (sf.reactions * q_j / 5.0) / (2.0 * bang.hot_energy),
        q_alpha_hot_spot: 0.5 * a[A_ALPHA] / bang.compression_cumulative,
        alpha_energy_mj: mj(a[A_ALPHA]),
        alpha_energy_in_hot_spot_mj: mj(a[A_ALPHA_HOT]),
        alpha_energy_through_shell_mj: mj(a[A_ALPHA_SHELL]),
        alpha_energy_escaped_mj: mj(a[A_ALPHA_ESCAPED]),
        alpha_energy_in_ablator_mj: mj(a[A_ALPHA_ABLATOR]),
        neutron_energy_mj: mj(sf.reactions * neutron_birth_energy_kev() * JOULE_PER_KEV),
        bremsstrahlung_mj: mj(a[A_BREMSSTRAHLUNG]),
        bremsstrahlung_absorbed_in_shell_mj: mj(a[A_RADIATION_TO_SHELL]),
        conduction_returned_mj: mj(a[A_CONDUCTION]),
        compression_work_mj: mj(a[A_COMPRESSION]),
        expansion_work_mj: mj(a[A_EXPANSION]),
        kinetic_energy_carried_by_products_mj: mj(a[A_CARRIED]),
        energy_at_start_mj: mj(fine.start_energy),
        energy_at_end_mj: mj(fine.end.total_energy()),
        energy_residual,
        particle_residual,
        start_time_ps: ps(start_time_fine),
        steps: fine.steps,
        refinements,
        shell_density_g_per_cm3: stag.shell_density / 1.0e3,
        ablator_density_g_per_cm3: stag.ablator_density / 1.0e3,
        stagnation_hot_spot_mass_mg: stag.hot_mass * 1.0e6,
        history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(t: f64, p: f64, r: f64, rho_r: f64, m: f64) -> HotSpotQuestion {
        HotSpotQuestion {
            hot_spot_temperature_kev: t,
            hot_spot_pressure_gbar: p,
            hot_spot_radius_um: r,
            shell_areal_density_g_per_cm2: rho_r,
            fuel_mass_mg: m,
            ablator_mass_mg: 0.0,
        }
    }

    fn tamped(t: f64, p: f64, r: f64, rho_r: f64, m: f64, ablator: f64) -> HotSpotQuestion {
        HotSpotQuestion { ablator_mass_mg: ablator, ..question(t, p, r, rho_r, m) }
    }

    /// The breadth tests of 16 October 2026 (random_conformance, D3): inside
    /// the declared ranges, a hot spot of 1e-110 um has a volume of zero in
    /// 64 bits and a density of 0/0, NaN, which the shell's comparison read
    /// and refused as ShellNotDenserThanHotSpot, "the hot spot's, NaN
    /// g/cm3"; a corner at the smallest normal numbers gave a shell of -inf
    /// g/cm3. A value that is not finite is refused as itself,
    /// NonFiniteValue at 0 ps, before any physical comparison reads it: the
    /// question is admitted, as it shows no named limit, and the solve
    /// refuses it, sealed, as any value of the burn that is not finite. A
    /// question whose values are finite is answered as before.
    #[test]
    fn a_value_not_finite_at_peak_compression_is_refused_as_itself() {
        let tiny = HotSpotQuestion { hot_spot_radius_um: 1.0e-110, ..question(4.0, 150.0, 30.0, 0.8, 0.2) };
        let corner = HotSpotQuestion {
            hot_spot_temperature_kev: 0.2,
            hot_spot_pressure_gbar: f64::MIN_POSITIVE,
            hot_spot_radius_um: f64::MIN_POSITIVE,
            shell_areal_density_g_per_cm2: 1.0e12,
            fuel_mass_mg: f64::MIN_POSITIVE,
            ablator_mass_mg: 1.0e12,
        };
        for q in [tiny, corner] {
            assert_eq!(admit(&q), Ok(()), "{q:?}");
            assert_eq!(solve(&q).err(), Some(HotSpotReject::NonFiniteValue { time_ps: 0.0 }), "{q:?}");
        }
        // Finite values are compared as before.
        assert_eq!(admit(&HotSpotQuestion { hot_spot_radius_um: 1.0e-100, ..tiny }), Ok(()));
        assert!(matches!(admit(&question(4.0, 150.0, 30.0, 0.05, 0.2)), Err(HotSpotReject::ShellNotDenserThanHotSpot { .. })));
    }

    /// A sub-ignited hot spot, quick to run.
    fn sub_ignited() -> HotSpotQuestion {
        question(4.0, 150.0, 35.0, 0.8, 0.2)
    }

    /// A hot spot that ignites and burns through its shell.
    fn igniting() -> HotSpotQuestion {
        question(7.0, 300.0, 30.0, 0.8, 0.2)
    }

    #[test]
    fn the_reaction_energy_is_shared_by_two_body_kinematics() {
        let alpha = alpha_birth_energy_kev();
        let neutron = neutron_birth_energy_kev();
        assert!((alpha + neutron - 17_589.0).abs() < 1.0e-9);
        assert!((alpha - 3560.80).abs() < 0.01, "{alpha}");
        assert!((neutron - 14028.20).abs() < 0.01, "{neutron}");
        // The non-relativistic share, Q m_n / (m_n + m_α), is 20 keV lower.
        let classical = 17_589.0 * NEUTRON_REST_KEV / (NEUTRON_REST_KEV + ALPHA_REST_KEV);
        assert!((alpha - classical - 19.7).abs() < 0.2, "{}", alpha - classical);
    }

    /// Zylstra and Hurricane, Phys. Plasmas 26, 062701 (2019), equation 12 and
    /// Table I, the fit to Maynard-Deutsch stopping, "accurate to ~2%":
    /// `ρλ = a T^c/(1 + b T^c) (1 + d (ρ/100)^e)`. The range by this solver's
    /// rates (the NRL Formulary's, with their Coulomb logarithms) at
    /// hot-spot densities and temperatures, against it.
    #[test]
    fn the_alpha_range_is_zylstra_and_hurricane_s() {
        let fit = |t: f64, rho: f64| {
            let (a, b, c, d, e) = (0.01705, 0.01133, 1.0706, 1.5793, 0.1987);
            a * t.powf(c) / (1.0 + b * t.powf(c)) * (1.0 + d * (rho / 100.0).powf(e))
        };
        let mut worst: f64 = 0.0;
        for rho in [30.0, 100.0, 300.0] {
            for t in [5.0, 10.0, 20.0, 40.0] {
                let ions = rho * 1.0e3 / FUEL_ION_PAIR_MASS_KG;
                let ours = alpha_range_m(ions, t, ions, 0.0) * rho * 1.0e3 / KG_PER_M2_PER_G_PER_CM2;
                let off = ours / fit(t, rho) - 1.0;
                println!("rho {rho} g/cm3, T {t} keV: range {ours:.4} g/cm2, fit {:.4}, {off:+.3}", fit(t, rho));
                worst = worst.max(off.abs());
            }
        }
        assert!(worst < 0.15, "{worst}");
        // The deposition length is longer than the range: the alpha gives up its energy later along its path than the law E(x) = E0 (1 - x/l)^2.
        let ions = 100.0e3 / FUEL_ION_PAIR_MASS_KG;
        let ratio = alpha_path_m(ions, 10.0, ions, 0.0) / alpha_range_m(ions, 10.0, ions, 0.0);
        assert!(ratio > 1.2 && ratio < 1.8, "{ratio}");
    }

    /// Betti et al., LLE Review 85 (2000), equation 32: the ablation speed
    /// of the hot spot's edge into the shell, `6e3 T^(5/2) / (R ρ_shell Λ)`
    /// µm/ns, T in keV, R in µm, ρ in g/cm³.
    #[test]
    fn the_ablation_is_betti_s() {
        for (t, r_um, rho, logarithm) in [(5.0_f64, 30.0_f64, 300.0_f64, 5.0_f64), (10.0, 50.0, 100.0, 4.0)] {
            let t_j = t * JOULE_PER_KEV;
            let mass_rate = ABLATION_COEFFICIENT * FUEL_ION_PAIR_MASS_KG * r_um * 1.0e-6 * conductivity_coefficient() * t_j * t_j * t_j.sqrt() / logarithm;
            let speed = mass_rate / (4.0 * PI * (r_um * 1.0e-6).powi(2) * rho * 1.0e3);
            let betti = 6.0e3 * t.powf(2.5) / (r_um * rho * logarithm) * 1.0e3;
            assert!((speed / betti - 1.0).abs() < 0.02, "{speed} {betti}");
        }
    }

    /// Munro, Nucl. Fusion 56, 036001 (2016): the D-T neutron's energy at zero temperature, 14.0283920 MeV (CODATA 2010).
    #[test]
    fn the_neutron_energy_is_munro_s() {
        assert!((neutron_birth_energy_kev() / 14_028.392_0 - 1.0).abs() < 2.0e-5, "{}", neutron_birth_energy_kev());
    }

    #[test]
    fn the_krokhin_rozanov_fraction_is_continuous_and_bounded() {
        assert_eq!(alpha_deposition_fraction(0.0), 0.0);
        let below = alpha_deposition_fraction(0.5);
        let above = alpha_deposition_fraction(0.5 + 1.0e-12);
        assert!((below - 0.55).abs() < 1.0e-12 && (above - 0.55).abs() < 1.0e-9, "{below} {above}");
        let mut last = 0.0;
        for k in 1..=4000 {
            let f = alpha_deposition_fraction(k as f64 * 0.005);
            assert!(f > last && f < 1.0, "{k} {f}");
            last = f;
        }
        assert!((alpha_deposition_fraction(1.0e6) - 1.0).abs() < 1.0e-6);
    }

    /// The escape probability of a uniform sphere against a Monte Carlo of
    /// 200,000 photons computed outside the engine
    /// (`evidence/research/icf_nif_hotspot/analysis/`): 0.992524, 0.928814,
    /// 0.527142, 0.235846 at optical radii 0.01, 0.1, 1 and 3, each within
    /// its statistical error of about 0.001.
    #[test]
    fn the_sphere_escape_probability_is_the_one_counted_outside() {
        for (tau, counted) in [(0.01, 0.992_524), (0.1, 0.928_814), (1.0, 0.527_142), (3.0, 0.235_846)] {
            let p = sphere_escape_probability(tau);
            assert!((p - counted).abs() < 2.5e-3, "{tau}: {p} {counted}");
        }
        let series = sphere_escape_probability(0.05 - 1.0e-15);
        let exact = sphere_escape_probability(0.05);
        assert!((series - exact).abs() < 1.0e-12, "{series} {exact}");
        assert_eq!(sphere_escape_probability(0.0), 1.0);
        assert!((sphere_escape_probability(1.0e4) - 0.75e-4).abs() < 1.0e-9);
    }

    /// Rybicki and Lightman's equation 5.18b in cgs, `3.7e8 T^-1/2 Z² n_e n_i ν⁻³ (1 − e^(−hν/kT))` cm⁻¹, is the SI form here with its constant rounded to two figures.
    #[test]
    fn the_free_free_absorption_is_the_textbook_one() {
        let kelvin = 1.0e7;
        let t_kev = kelvin * 1.380_649e-23 / JOULE_PER_KEV;
        let (ne, nu) = (1.0e20, 1.0e17);
        let photon_kev = PLANCK_J_S * nu / JOULE_PER_KEV;
        let cgs = 3.7e8 / kelvin.sqrt() * ne * ne / (nu * nu * nu) * -(-photon_kev / t_kev).exp_m1();
        let si = free_free_absorption_per_m(ne * 1.0e6, ne * 1.0e6, t_kev, photon_kev) / 100.0;
        assert!((si / cgs - 1.0).abs() < 0.005, "{si} {cgs}");
    }

    /// The NRL Plasma Formulary's parallel electron conductivity for charge
    /// one, `3.2 n_e T_e τ_e / m_e` with `τ_e = 3.44e5 T_e[eV]^(3/2) / (n_e[cm⁻³] lnΛ)` s,
    /// is Braginskii's 3.1616 rounded: the coefficient here is within 1.5 % of it.
    #[test]
    fn the_conductivity_is_braginskii_s() {
        let (n, t_ev, logarithm) = (1.0e31, 5000.0, 4.0);
        let tau = 3.44e5 * f64::powf(t_ev, 1.5) / (n * 1.0e-6 * logarithm);
        let t_j = t_ev * ELEMENTARY_CHARGE_C;
        let formulary = 3.2 * n * t_j * tau / ELECTRON_MASS_KG;
        let here = conductivity_coefficient() * t_j * t_j * t_j.sqrt() / logarithm / ELEMENTARY_CHARGE_C * ELEMENTARY_CHARGE_C;
        // formulary κ is in W/(m K) per (J/K): with T in joules both are W/(m J).
        let ratio = here / formulary;
        assert!((ratio - 3.1616 / 3.2).abs() < 0.015, "{ratio}");
    }

    #[test]
    fn the_bounded_logarithm_is_the_formulary_s_where_it_is_large() {
        for l in [4.0, 6.0, 10.0, 20.0] {
            assert!((bounded_logarithm(l) - l).abs() < 2.0e-4, "{l}");
        }
        for l in [1.0, 0.0, -1.0, -10.0] {
            assert!(bounded_logarithm(l) > 0.0 && bounded_logarithm(l) > l, "{l}");
        }
        assert!((bounded_logarithm(0.0) - 0.5 * 2.0_f64.ln()).abs() < 1.0e-15);
    }

    #[test]
    fn a_sub_ignited_burn_closes_its_ledgers() {
        let burn = solve(&sub_ignited()).expect("burns");
        println!("{burn:?}");
        assert!(burn.energy_residual <= ENERGY_TOLERANCE, "{}", burn.energy_residual);
        assert!(burn.particle_residual <= PARTICLE_TOLERANCE, "{}", burn.particle_residual);
        // The ledger in the published accounts: the change of the energy of
        // the hot spot and the shell is the alpha energy less what left.
        let change = burn.energy_at_end_mj - burn.energy_at_start_mj;
        let net = burn.alpha_energy_mj - burn.bremsstrahlung_mj - burn.alpha_energy_escaped_mj - burn.kinetic_energy_carried_by_products_mj;
        assert!((change - net).abs() <= ENERGY_TOLERANCE * burn.energy_at_start_mj.max(burn.alpha_energy_mj), "{change} {net}");
        let split = burn.alpha_energy_in_hot_spot_mj + burn.alpha_energy_through_shell_mj + burn.alpha_energy_escaped_mj + burn.alpha_energy_in_ablator_mj;
        assert!((split / burn.alpha_energy_mj - 1.0).abs() < 1.0e-9, "{split}");
        let q = alpha_birth_energy_kev() + neutron_birth_energy_kev();
        assert!((burn.alpha_energy_mj / burn.yield_mj - alpha_birth_energy_kev() / q).abs() < 1.0e-9);
        assert!((burn.neutron_energy_mj + burn.alpha_energy_mj - burn.yield_mj).abs() < 1.0e-9 * burn.yield_mj);
        assert!(burn.yield_amplification > 1.0 && burn.yield_amplification < 10.0);
        assert!(burn.burn_up_fraction > 0.0 && burn.burn_up_fraction < 1.0e-3);
        assert_eq!(burn.burned_through, 0.0);
        assert_eq!(burn.alpha_energy_escaped_mj, 0.0);
        assert!(burn.self_heating_ratio_at_bang < 1.0);
        assert!(burn.start_time_ps < 0.0 && burn.burn_width_ps > 0.0);
    }

    #[test]
    fn an_igniting_burn_closes_its_ledgers_through_burn_through() {
        let burn = solve(&igniting()).expect("burns");
        println!("{:?}", HotSpotBurn { history: vec![], ..burn.clone() });
        assert!(burn.energy_residual <= ENERGY_TOLERANCE, "{}", burn.energy_residual);
        assert!(burn.particle_residual <= PARTICLE_TOLERANCE);
        assert_eq!(burn.burned_through, 1.0);
        assert!(burn.alpha_energy_escaped_mj > 0.0);
        assert!(burn.yield_amplification > 100.0);
        // The burn-up of a clean, fully burned fuel: below the ρR / (ρR + 6 g/cm²) of a uniform sphere at this areal density, and above zero.
        assert!(burn.burn_up_fraction > 0.05 && burn.burn_up_fraction < 0.5, "{}", burn.burn_up_fraction);
        let change = burn.energy_at_end_mj - burn.energy_at_start_mj;
        let net = burn.alpha_energy_mj - burn.bremsstrahlung_mj - burn.alpha_energy_escaped_mj - burn.kinetic_energy_carried_by_products_mj;
        assert!((change - net).abs() <= ENERGY_TOLERANCE * burn.alpha_energy_mj, "{change} {net}");
    }

    /// The remaining ablator tamps the shell: the same hot spot burns longer
    /// and yields more with it, its ledgers close, and with no ablator the
    /// answer is the one without the input.
    #[test]
    fn an_ablator_outside_the_fuel_tamps_it_and_its_ledgers_close() {
        let bare = solve(&sub_ignited()).expect("burns");
        let with = solve(&tamped(4.0, 150.0, 35.0, 0.8, 0.2, 0.2)).expect("burns");
        println!("bare: yield {} MJ, width {} ps; tamped: yield {} MJ, width {} ps, ablator {} g/cm3", bare.yield_mj, bare.burn_width_ps, with.yield_mj, with.burn_width_ps, with.ablator_density_g_per_cm3);
        assert!(with.energy_residual <= ENERGY_TOLERANCE && with.particle_residual <= PARTICLE_TOLERANCE);
        assert!(with.burn_width_ps > bare.burn_width_ps, "{} {}", with.burn_width_ps, bare.burn_width_ps);
        assert!((with.ablator_density_g_per_cm3 / with.shell_density_g_per_cm3 - ABLATOR_ION_PAIR_MASS_KG / 7.0 / (FUEL_ION_PAIR_MASS_KG / 2.0)).abs() < 1.0e-12);
        let none = solve(&tamped(4.0, 150.0, 35.0, 0.8, 0.2, 0.0)).expect("burns");
        assert_eq!(none.yield_mj.to_bits(), bare.yield_mj.to_bits());
        assert!(matches!(admit(&tamped(4.0, 150.0, 35.0, 0.8, 0.2, -0.1)), Err(HotSpotReject::AblatorMassNegative { .. })));
        // Burned through into the ablator: the alpha energy not deposited in the hot spot heats the ablator, and every account still closes.
        let ignited = solve(&tamped(7.0, 300.0, 30.0, 0.8, 0.2, 0.2)).expect("burns");
        assert_eq!(ignited.burned_through, 1.0);
        assert!(ignited.alpha_energy_in_ablator_mj > 0.0 && ignited.alpha_energy_escaped_mj == 0.0);
        let split = ignited.alpha_energy_in_hot_spot_mj + ignited.alpha_energy_through_shell_mj + ignited.alpha_energy_in_ablator_mj;
        assert!((split / ignited.alpha_energy_mj - 1.0).abs() < 1.0e-9);
        assert!(ignited.energy_residual <= ENERGY_TOLERANCE);
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let first = solve(&sub_ignited()).expect("burns");
        let second = solve(&sub_ignited()).expect("burns");
        assert_eq!(first.yield_mj.to_bits(), second.yield_mj.to_bits());
        assert_eq!(first.history.len(), second.history.len());
        for (a, b) in first.history.iter().zip(&second.history) {
            for (x, y) in a.iter().zip(b) {
                assert_eq!(x.to_bits(), y.to_bits());
            }
        }
    }

    /// Rule 8: break what must hold, on purpose, and require the named limit.
    #[test]
    fn a_hot_spot_that_does_no_work_breaks_the_energy_ledger() {
        match solve_with(&sub_ignited(), Fault::NoExpansionWork) {
            Err(HotSpotReject::EnergyLedgerBroken { residual, .. }) => assert!(residual.abs() > ENERGY_TOLERANCE),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ablated_fuel_that_arrives_without_its_energy_breaks_the_energy_ledger() {
        match solve_with(&sub_ignited(), Fault::AblationLosesEnergy) {
            Err(HotSpotReject::EnergyLedgerBroken { residual, .. }) => assert!(residual.abs() > ENERGY_TOLERANCE),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_reaction_that_takes_three_ions_breaks_the_particle_ledger() {
        match solve_with(&sub_ignited(), Fault::ThreeIonsPerReaction) {
            Err(HotSpotReject::ParticleLedgerBroken { residual, .. }) => assert!(residual > PARTICLE_TOLERANCE),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_question_s_own_limits_are_named() {
        let good = sub_ignited();
        let with = |change: fn(&mut HotSpotQuestion)| {
            let mut q = good;
            change(&mut q);
            admit(&q)
        };
        assert_eq!(with(|q| q.hot_spot_radius_um = f64::NAN), Err(HotSpotReject::InputNotFinite));
        assert_eq!(with(|q| q.fuel_mass_mg = f64::INFINITY), Err(HotSpotReject::InputNotFinite));
        assert!(matches!(with(|q| q.hot_spot_temperature_kev = 0.199), Err(HotSpotReject::TemperatureOutsideReactivityFit { .. })));
        assert!(matches!(with(|q| q.hot_spot_temperature_kev = 100.01), Err(HotSpotReject::TemperatureOutsideReactivityFit { .. })));
        assert!(matches!(with(|q| q.hot_spot_pressure_gbar = 0.0), Err(HotSpotReject::PressureNotPositive { .. })));
        assert!(matches!(with(|q| q.hot_spot_radius_um = -1.0), Err(HotSpotReject::RadiusNotPositive { .. })));
        assert!(matches!(with(|q| q.shell_areal_density_g_per_cm2 = 0.0), Err(HotSpotReject::ShellArealDensityNotPositive { .. })));
        // 150 Gbar at 4 keV in 35 um holds 0.0088 mg.
        assert!(matches!(with(|q| q.fuel_mass_mg = 0.008), Err(HotSpotReject::FuelMassNotAboveHotSpotMass { .. })));
        // 0.8 g/cm2 on a 35 um sphere needs at least 0.123 mg of shell.
        assert!(matches!(with(|q| q.fuel_mass_mg = 0.12), Err(HotSpotReject::ShellMassBelowArealDensity { .. })));
        // 0.19 mg spread as 0.05 g/cm2 outside 35 um is a shell of 2.1 g/cm3 around a hot spot of 49 g/cm3.
        assert!(matches!(with(|q| q.shell_areal_density_g_per_cm2 = 0.05), Err(HotSpotReject::ShellNotDenserThanHotSpot { .. })));
        assert_eq!(admit(&good), Ok(()));
        assert_eq!(admit(&question(TEMPERATURE_MIN_KEV, 5.0, 35.0, 0.8, 0.2)), Ok(()));
        assert_eq!(admit(&question(TEMPERATURE_MAX_KEV, 150.0, 35.0, 0.8, 0.2)), Ok(()));
    }

    #[test]
    fn above_the_pressure_ceiling_every_hot_spot_is_degenerate() {
        let n = PRESSURE_MAX_GBAR * PA_PER_GBAR / (2.0 * TEMPERATURE_MAX_KEV * JOULE_PER_KEV);
        assert!((fermi_energy_kev(n) / TEMPERATURE_MAX_KEV - 1.0).abs() < 1.0e-6, "{}", fermi_energy_kev(n));
        assert!(matches!(admit(&question(TEMPERATURE_MAX_KEV, PRESSURE_MAX_GBAR * 1.0001, 35.0, 0.8, 0.2)), Err(HotSpotReject::StagnationDegenerate { .. })));
        assert!(matches!(admit(&question(4.0, 1.0e5, 35.0, 0.8, 0.2)), Err(HotSpotReject::StagnationDegenerate { .. })));
        assert!(matches!(admit(&question(4.0, 150.0, RADIUS_MAX_UM * 2.0, 0.8, 0.2)), Err(HotSpotReject::InputTooLargeForArithmetic { .. })));
        assert!(matches!(admit(&question(4.0, 150.0, 35.0, 0.8, FUEL_MASS_MAX_MG * 2.0)), Err(HotSpotReject::InputTooLargeForArithmetic { .. })));
    }

    #[test]
    fn the_shell_holds_the_fuel_left_for_it() {
        let q = sub_ignited();
        let stag = stagnation(&q).expect("admitted");
        let s = &stag.state;
        let shell: f64 = s.mass.iter().sum();
        assert!(((shell + s.hot_mass()) / (q.fuel_mass_mg * 1.0e-6) - 1.0).abs() < 1.0e-12);
        let areal = s.shell_areal_density() / KG_PER_M2_PER_G_PER_CM2;
        assert!((areal / q.shell_areal_density_g_per_cm2 - 1.0).abs() < 1.0e-12, "{areal}");
        assert!((s.pressure_pa() / PA_PER_GBAR / q.hot_spot_pressure_gbar - 1.0).abs() < 1.0e-12);
        assert!((s.temperature_kev() / q.hot_spot_temperature_kev - 1.0).abs() < 1.0e-12);
        for i in 0..s.zones() {
            let p = 2.0 / 3.0 * s.energy[i] / s.zone_volume(i);
            assert!((p / s.pressure_pa() - 1.0).abs() < 1.0e-9);
        }
    }

    /// Rule 7. N221204 as PRL 132 065102, Table I, prints it, read
    /// Retrieved 2026-10-14 ([`REFERENCE_RETRIEVED`]) from
    /// <https://doi.org/10.1103/PhysRevLett.132.065102>, made into the
    /// solver's question by the rule of
    /// `evidence/research/icf_nif_hotspot/validation_protocol.md`: `T_DD`,
    /// the pressure, the equivalent radius of the neutron-image volume, the
    /// DSR areal density (18.5 times the DSR) less the hot spot's own, and
    /// the 0.220 mg of fuel. The hot spot those numbers make holds 0.085 mg,
    /// against the 0.084 +- 0.010 mg Pak et al. (PRE 109, 025203) infer; the
    /// shell they leave is 8.3 g/cm3 around a hot spot of 66 g/cm3, and the
    /// question is refused by name. With a shell three times the hot spot's
    /// density (a reading made after the scoring, not gated) the solver's
    /// clean burn goes through the whole shell: the yield is above the
    /// measured 3.1 MJ and the burn width far below the measured 75 ps, a
    /// miss of the burn-width mark.
    #[test]
    fn nif_n221204_as_published() {
        println!("{REFERENCE_NAME}, read {REFERENCE_RETRIEVED} from {REFERENCE_URL}");
        let (t, p, volume_um3, dsr, fuel) = (12.0, 608.0, 12.9e5, 0.0283, 0.220);
        let radius = (3.0 * volume_um3 / (4.0 * PI)).cbrt();
        let density = FUEL_ION_PAIR_MASS_KG * p * PA_PER_GBAR / (2.0 * t * JOULE_PER_KEV);
        let hot_mass_mg = density * 4.0 / 3.0 * PI * (radius * 1.0e-6).powi(3) * 1.0e6;
        assert!((hot_mass_mg - 0.084).abs() <= 0.010, "{hot_mass_mg}");
        let hot_areal = density * radius * 1.0e-6 / KG_PER_M2_PER_G_PER_CM2;
        assert!((hot_areal - 0.44).abs() <= 0.06, "{hot_areal}");
        let published = question(t, p, radius, 18.5 * dsr - hot_areal, fuel);
        match admit(&published) {
            Err(HotSpotReject::ShellNotDenserThanHotSpot { shell_density_g_per_cm3, hot_spot_density_g_per_cm3 }) => {
                assert!((shell_density_g_per_cm3 - 8.28).abs() < 0.05, "{shell_density_g_per_cm3}");
                assert!((hot_spot_density_g_per_cm3 - 66.04).abs() < 0.05, "{hot_spot_density_g_per_cm3}");
            }
            other => panic!("{other:?}"),
        }
        // The shell of three times the hot spot's density.
        let shell_mass = fuel * 1.0e-6 - hot_mass_mg * 1.0e-6;
        let rho = 3.0 * density;
        let r = radius * 1.0e-6;
        let outer = (r * r * r + 3.0 * shell_mass / (4.0 * PI * rho)).cbrt();
        let dense = question(t, p, radius, rho * (outer - r) / KG_PER_M2_PER_G_PER_CM2, fuel);
        let burn = solve(&dense).expect("burns");
        println!("dense shell: yield {} MJ against 3.1, burn width {} ps against 75, burn-up {} against 0.0433", burn.yield_mj, burn.burn_width_ps, burn.burn_up_fraction);
        assert!(burn.yield_mj > 3.1, "{}", burn.yield_mj);
        assert_eq!(burn.burned_through, 1.0);
        assert!(burn.burn_width_ps < 0.7 * 75.0, "{}", burn.burn_width_ps);
    }

    /// N221204 re-evaluated after the fact, not gated: the consistent reading
    /// of an independent second party (evidence/research/icf_nif_hotspot/
    /// second_party/, analysis/reevaluate_consistent.py). The same
    /// temperature, pressure, radius and shell areal density as the
    /// registered question; the fuel mass is the stagnated mass, the hot
    /// spot's and a uniform shell of that areal density at 255 or
    /// 510 g/cm^3. The yield and the burn-up over the whole fuel come within
    /// 0.7 to 1.0 of the measured 3.1 MJ and 4.33 percent; the burn is 3 times
    /// too short and burns through. out/consistent.json holds these numbers as
    /// the platform's math gave them; these are genesis_kernel::math's (since
    /// genesis_solvers 0.12.0), within 5e-12 of them.
    #[test]
    fn nif_n221204_stagnated_mass() {
        let (t, p, radius, sigma) = (12.0, 608.0, 67.53056246593466, 0.0776038256167414);
        for (stagnated_mg, yield_mj, burn_width_ps, burn_up) in [
            (0.13169370648325554, 2.1861092098257755, 27.73182403587192, 0.04920230148193162),
            (0.13066903769359978, 2.9724751621704697, 28.01154490485601, 0.0674254923498441),
        ] {
            let burn = solve(&question(t, p, radius, sigma, stagnated_mg)).expect("burns");
            assert_eq!((burn.yield_mj, burn.burn_width_ps, burn.burn_up_fraction), (yield_mj, burn_width_ps, burn_up));
            assert_eq!(burn.burned_through, 1.0);
            let whole_fuel = burn.burn_up_fraction * stagnated_mg / 0.220;
            assert!((0.029..0.041).contains(&whole_fuel), "{whole_fuel}");
        }
        // The registered question, the same but for the 0.220 mg of fuel in one shell.
        assert!(matches!(admit(&question(t, p, radius, sigma, 0.22)), Err(HotSpotReject::ShellNotDenserThanHotSpot { .. })));
    }

    #[test]
    fn a_degenerate_hot_spot_is_refused_by_name() {
        // 0.3 keV at 2000 Gbar: Fermi energy 5.9 keV, at the door.
        match admit(&question(0.3, 2000.0, 20.0, 13.9, 1.0)) {
            Err(HotSpotReject::StagnationDegenerate { temperature_kev, fermi_energy_kev }) => {
                assert!(fermi_energy_kev >= temperature_kev)
            }
            other => panic!("{other:?}"),
        }
        // 2 keV at 400 Gbar in 20 um inside 1.5 g/cm2: radiative collapse into degenerate matter during the burn.
        match solve(&question(2.0, 400.0, 20.0, 1.5, 0.2)) {
            Err(HotSpotReject::HotSpotDegenerate { temperature_kev, fermi_energy_kev, .. }) => assert!(fermi_energy_kev >= temperature_kev),
            other => panic!("{other:?}"),
        }
    }
}
