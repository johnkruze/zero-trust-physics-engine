#![no_implicit_prelude]

//! JET in a heated pulse of its 1996 to 1997 deuterium-tritium campaign
//! (the Mark IIa divertor, DTE1), as one digital twin of ten pulses: the
//! thermal plasma, the deuterium and the tritium beam ions, the ion-cyclotron
//! tail and the fusion alphas, stepped through time under the heating
//! power, with the electron temperature and the effective charge the
//! pulse's own measured traces.
//!
//! Measured at JET (Culham, EFDA-JET) and published in the International
//! Multi-Tokamak Confinement Profile Database, 2008 public release
//! (`https://tokamak-profiledb.ccfe.ac.uk/PR08/`): for each pulse the time
//! traces of heating power, density, current, field, central electron and
//! ion temperature, effective charge, thermal and total plasma energy, D-T
//! fusion power and its thermal part, and the radial profiles. The thermal
//! energy, the fast-ion energy, the fusion power and its thermal part are
//! the output of the TRANSP analysis of each pulse from its measured
//! profiles (R. V. Budny, PPPL), not raw signals. The database page says
//! anyone may make analyses and publications from the public version
//! provided its papers are cited (Nucl. Fusion 40 (2000) 1955; Nucl. Fusion
//! 39 (1999) 2175). No measured value of a pulse is stored in the card.
//!
//! The record is `evidence/research/tokamak_jet_dte1/twin/`. Ten pulses:
//! 38285, 38287, 38415 and 43134 (deuterium, 43134 with a trace of
//! tritium), 42762 and 42982 (an even mix, beams of both species), 42794 and
//! 42997 (three quarters tritium, tritium beams), 43002 (93 percent tritium,
//! beams of both species) and the hot-ion pulse 42976. Four constants are
//! fitted on all ten by `twin/fit_twin.py` (`twin/out/fit_constants.json`):
//! [`CONFINEMENT_MULTIPLIER_ELMY`], [`CONFINEMENT_MULTIPLIER_HOT_ION`],
//! [`SLOWING_TIME_MULTIPLIER`] and [`TAIL_TIME_MULTIPLIER`], by least
//! squares on the thermal energy, the fast-ion energy, the fusion power and
//! its thermal part of every pulse, each scaled by the pulse's own peak; the
//! hot-ion multiplier rests on 42976 alone. The five profile numbers are the
//! means of the ten pulses' TRANSP radial profiles (`twin/survey_shapes.py`,
//! `twin/out/shapes.json`; the beam source, `twin/survey_deposition.py`,
//! `twin/out/deposition.json`); the geometry is the mean of the ten pulses'
//! 0D files.
//!
//! What the card is. Sixteen cells equal in volume, `x_c = (c + ½)/16`,
//! carry the profiles:
//!
//! ```text
//! n_c      = k_n · n_el · s_n(x_c) / mean(s_n)       s_n = (1 − x)^a_n
//! Te_c     = Te0 · (1 − x_c)^a_e                     Ti_c = Ti0 · (1 − x_c)^a_i
//! W_e      = 1.5 · e · V · mean(n_c · Te_c)                                   J
//! W_i      = W_th − W_e = 1.5 · e · V · f_i · mean(n_c · Ti_c)                 the ion temperature Ti0
//! P_th     = E_DT · f_D · f_T · f_main² · V · mean(n_c² · ⟨σv⟩(Ti_c))         W
//! ```
//!
//! The thermal energy the electrons hold follows from their measured
//! temperature; the rest is the ions', whose temperature drives the thermal
//! fusion. Where beams heat the ions above the electrons, as in the hot-ion
//! pulse, the ion temperature rises above the electrons' with no constant
//! for it. Each beam species `s` (deuterium, tritium) is born at its
//! injection energy `E0` and at `E0/2` and `E0/3` in the power shares of the
//! positive-ion injectors, in the cells by the beam source profile
//! `(1 − x)^a_b`, and slows on the electrons and the ions at the classical
//! rate:
//!
//! ```text
//! dE/dt    = −(2E / τ_s) · (1 + (E_c / E)^1.5)
//! τ_s      = k_s · 6.27e8 · A · Te[eV]^1.5 / (n_e[cm^-3] · lnΛ),   lnΛ = 24 − ln(√n_e[cm^-3] / Te[eV])
//! E_c      = 14.8 · A · Te · [Σ_j n_j Z_j² / (n_e A_j)]^(2/3)                keV
//! J_W      = ∫₀^E0 dE / (2 (1 + (E_c/E)^1.5))        J_R = ∫₀^E0 σ(E_cm) v / (2E (1 + (E_c/E)^1.5)) dE
//! τ_f,s    = Σ_c w_c Σ_k (f_k / E_k) · τ_s,c · J_W,k,c                         s
//! ρ_s      = Σ_c w_c Σ_k (f_k/E_k) τ_s,c n_tgt,c J_R,k,c / (Σ_c w_c Σ_k (f_k/E_k) τ_s,c J_W,k,c) / e·1e3
//! dW_s/dt  = P_s − W_s / τ_f,s                       P_bt,s = E_DT · ρ_s · W_s
//! ```
//!
//! `σ` is the D-T cross-section of Bosch and Hale (Nucl. Fusion 32 (1992)
//! 611, its S-factor fit of Table IV, 0.5 to 550 keV in the centre of
//! mass), at `E_cm = E · m_target / (m_beam + m_target)`, on a cold target:
//! a deuteron of the beam on the tritons of the plasma, a triton on the
//! deuterons. `ρ_s` is the fusion rate per joule of beam ions of the
//! classical slowing-down spectrum, and `τ_f,s` the time in which their
//! energy reaches the thermal plasma. The integrals are Gauss-Legendre, eight
//! nodes on each of four equal panels of `[0, E_k]`. The tail of the
//! ion-cyclotron heating and the fusion alphas are one store that
//! thermalises on the electrons:
//!
//! ```text
//! dW_x/dt  = P_icrh + 0.2 · P_fus − W_x / τ_x,      τ_x = k_x · Σ_c w_c τ_s,c(A = 1) / 2
//! τ_E      = H · 0.0562 · Ip^0.93 · Bt^0.15 · n19^0.41 · M^0.19 · R^1.97 · (a/R)^0.58 · κ^0.78 · P^−0.69
//! P        = P_D + P_T + P_icrh + P_ohm          (MW, the heating supplied)
//! dW_th/dt = W_D/τ_f,D + W_T/τ_f,T + W_x/τ_x + P_ohm − W_th / τ_E
//! dE_fus/dt = P_th + P_bt,D + P_bt,T
//! ```
//!
//! `H` is [`CONFINEMENT_MULTIPLIER_ELMY`] in an ELMy H-mode and
//! [`CONFINEMENT_MULTIPLIER_HOT_ION`] in the ELM-free hot-ion H-mode, which
//! the caller names with the load field `hot_ion_regime` (0 or 1). The
//! confinement time is the IPB98(y,2) scaling (ITER Physics Basis, Nucl.
//! Fusion 39 (1999) 2175, equation 20) at the heating supplied, so the
//! thermal energy relaxes to the scaling's value in one confinement time.
//! `f_main = (6 − Z_eff)/5` and `f_i = f_main + (Z_eff − 1)/30` are the
//! fuel and the total ion number per electron with carbon as the one
//! impurity, from the measured effective charge. A cell colder than 0.2 keV,
//! the floor of the reactivity fit, adds no thermal reactions. The map is
//! fixed-step fourth-order Runge–Kutta on (W_th, W_D, W_T, W_x, E_fus) with
//! the load held through the step; every time and rate the load sets is
//! computed once per step.
//!
//! The named limits, each enforced and none pulled back:
//!
//! ```text
//! the load              powers ≥ 0 and their sum > 0 (the scaling's P^−0.69), Ip > 0, Bt > 0, n_el > 0,
//!                       0 ≤ f_T ≤ 1, 1 ≤ Z_eff < 6, Te0 > 0 and lnΛ > 0 in every cell,
//!                       beam energies > 0 with E_cm at most 550 keV, hot_ion_regime 0 or 1
//! Greenwald density     n_el ≤ 1e20 · Ip[MA] / (π a²)
//! safety factor         q ≥ 2 (the ITER guideline formula of tokamak_jet_dte1)
//! the stores            W_D, W_T, W_x ≥ 0, and W_th above the electrons' W_e
//! reactivity fit        Ti0 ≤ 100 keV
//! Troyon beta           β_N ≤ 2.8 of all four stores
//! the clock             t ≥ 0, and a step moves it
//! the step              dt ≤ min(τ_f,D, τ_f,T, τ_x, τ_E) / 8
//! ```
//!
//! The ledger. The four stores change only by what is supplied and what is
//! lost:
//!
//! ```text
//! defect   = (W_th + W_D + W_T + W_x) − anchor − [ Σ (P_D + P_T + P_icrh + P_ohm) dt + 0.2 · E_fus − Σ Q ]
//! Q        = dt/2 · (P0 + P1) + dt²/12 · (P0' − P1'),   P = W_th / τ_E,  P' = (dW_th/dt) / τ_E
//! residual = |defect| / max(W_th + W_D + W_T + W_x, anchor)
//! ```
//!
//! The Runge–Kutta step does not use `Q`, so an Euler step, a wrong stage
//! weight or a dropped term shows in the defect. The ledger shows the
//! arithmetic and the integrator are right. It cannot show the physics
//! matches the tokamak.
//!
//! What is left out: beam-beam reactions and the thermal motion of the
//! target (the solver `tokamak_dt_beam_target` carries both, for one time),
//! D-D fusion, transport and the shapes' change with time, each ELM and
//! sawtooth, the L-mode before the transition, radiation as a separate
//! loss, the particle balance, orbit and charge-exchange losses past the
//! shine-through the caller subtracts, the ion-cyclotron power's split
//! between the tail and the bulk, the current drive, and every other
//! tokamak.
//!
//! The clock. It is a running sum, `t += dt`, from the time the caller
//! states; a clock below zero is [`StepReject::NegativeTime`], and a step too
//! small to move it is [`StepReject::StepBelowClockResolution`]. The card has
//! no end.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, the thermal energy, the fast-ion energy (the
//! three fast stores), the fusion energy, the fusion power, the central ion
//! temperature, the normalised beta, the loss power and the residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::math::{exp, log};
use crate::normal::normal_or_zero;

/// Major radius, m. Mean of `RGEO` over the ten pulses' 0D files, four figures.
pub const MAJOR_RADIUS_M: f64 = 2.903;
/// Minor radius, m. Mean of `AMIN` over the ten pulses, four figures.
pub const MINOR_RADIUS_M: f64 = 0.9382;
/// Elongation. Mean of `KAPPA` over the ten pulses, four figures.
pub const ELONGATION: f64 = 1.755;
/// Plasma volume, m³. Mean of `VOL` over the ten pulses, four figures.
pub const PLASMA_VOLUME_M3: f64 = 81.88;
/// Volume-averaged over line-averaged electron density, the mean of the ten pulses' profiles.
pub const DENSITY_VOLUME_OVER_LINE: f64 = 0.956;
/// Exponent `a_n` of the density profile `(1 − x)^a_n`, the mean of the ten pulses' profiles.
pub const DENSITY_EXPONENT: f64 = 0.14;
/// Exponent `a_e` of the electron temperature profile, the mean of the ten pulses' profiles.
pub const ELECTRON_TEMPERATURE_EXPONENT: f64 = 0.793;
/// Exponent `a_b` of the beam source profile, the mean of the ten pulses' beam sources.
pub const BEAM_SOURCE_EXPONENT: f64 = 0.47;
/// Exponent `a_i` of the ion temperature profile, the mean of the ten pulses' profiles.
pub const ION_TEMPERATURE_EXPONENT: f64 = 1.106;
/// Fitted multiplier `H` of the confinement time in an ELMy H-mode. Key `H`.
pub const CONFINEMENT_MULTIPLIER_ELMY: f64 = 0.799621;
/// Fitted multiplier `H` of the confinement time in the ELM-free hot-ion H-mode. Key `H_HOT`.
pub const CONFINEMENT_MULTIPLIER_HOT_ION: f64 = 1.77573;
/// Fitted multiplier `k_s` of the classical slowing-down time. Key `K_S`.
pub const SLOWING_TIME_MULTIPLIER: f64 = 1.13763;
/// Fitted multiplier `k_x` of the thermalisation time of the tail and the alphas. Key `K_X`.
pub const TAIL_TIME_MULTIPLIER: f64 = 1.42815;
/// Charge of the one impurity the dilution assumes, carbon.
pub const IMPURITY_CHARGE: f64 = 6.0;
/// Mass number of the impurity, carbon-12.
pub const IMPURITY_MASS_NUMBER: f64 = 12.0;
/// Mass of the deuteron, kg (CODATA 2018).
pub const DEUTERON_MASS_KG: f64 = 3.3435837724e-27;
/// Mass of the triton, kg (CODATA 2018).
pub const TRITON_MASS_KG: f64 = 5.0073567446e-27;
/// Atomic mass unit, kg (CODATA 2018).
pub const ATOMIC_MASS_KG: f64 = 1.66053906660e-27;
/// Elementary charge, C (SI 2019, exact).
pub const ELEMENTARY_CHARGE_C: f64 = 1.602176634e-19;
/// Energy of one D-T reaction, J: 17.589 MeV (Bosch and Hale 1992, Table I).
pub const DT_REACTION_ENERGY_J: f64 = 17.589e6 * ELEMENTARY_CHARGE_C;
/// Share of the D-T fusion energy carried by the alpha particle.
pub const ALPHA_ENERGY_SHARE: f64 = 0.2;
/// Power shares of a deuterium beam at its full, half and third energy.
pub const DEUTERIUM_BEAM_SHARES: [f64; 3] = [0.77, 0.17, 0.06];
/// Power shares of a tritium beam at its full, half and third energy.
pub const TRITIUM_BEAM_SHARES: [f64; 3] = [0.65, 0.21, 0.14];
/// The classical slowing-down time's coefficient, s, with Te in eV and n_e in cm^-3 (NRL Plasma Formulary).
pub const SLOWING_TIME_COEFF_S: f64 = 6.27e8;
/// The critical energy's coefficient, keV per keV of electron temperature per mass number.
pub const CRITICAL_ENERGY_COEFF: f64 = 14.8;
/// Constant of the Coulomb logarithm `24 − ln(√n_e[cm^-3] / Te[eV])`.
pub const COULOMB_LOG_CONSTANT: f64 = 24.0;
/// Coefficient of the IPB98(y,2) scaling: seconds, with MA, T, 1e19 m^-3, MW, m.
pub const IPB98_COEFF: f64 = 0.0562;
/// Exponent of the plasma current in IPB98(y,2).
pub const IPB98_EXP_CURRENT: f64 = 0.93;
/// Exponent of the toroidal field in IPB98(y,2).
pub const IPB98_EXP_FIELD: f64 = 0.15;
/// Exponent of the line-averaged density in IPB98(y,2).
pub const IPB98_EXP_DENSITY: f64 = 0.41;
/// Exponent of the mean ion mass in IPB98(y,2).
pub const IPB98_EXP_MASS: f64 = 0.19;
/// Exponent of the major radius in IPB98(y,2).
pub const IPB98_EXP_RADIUS: f64 = 1.97;
/// Exponent of the inverse aspect ratio in IPB98(y,2).
pub const IPB98_EXP_ASPECT: f64 = 0.58;
/// Exponent of the elongation in IPB98(y,2).
pub const IPB98_EXP_ELONGATION: f64 = 0.78;
/// Exponent of the heating power in IPB98(y,2).
pub const IPB98_EXP_POWER: f64 = -0.69;
/// Gamow constant of T(d,n)⁴He, sqrt(keV). Bosch and Hale 1992, Tables IV and VII.
pub const REACTIVITY_GAMOW_SQRT_KEV: f64 = 34.3827;
/// Reduced mass energy of T(d,n)⁴He, keV. Bosch and Hale 1992, Table VII.
pub const REACTIVITY_REDUCED_MASS_KEV: f64 = 1_124_656.0;
/// `C1` to `C7` of the T(d,n)⁴He reactivity. Bosch and Hale 1992, Table VII.
pub const REACTIVITY_COEFF: [f64; 7] = [
    1.17302e-9,
    1.51361e-2,
    7.51886e-2,
    4.60643e-3,
    1.35000e-2,
    -1.06750e-4,
    1.36600e-5,
];
/// `A1` to `A5` of the T(d,n)⁴He S-factor, 0.5 to 550 keV. Bosch and Hale 1992, Table IV (as ASCOT5 and FIDASIM carry it).
pub const CROSS_SECTION_A: [f64; 5] = [6.927e4, 7.454e8, 2.050e6, 5.2002e4, 0.0];
/// `B1` to `B4` of the same fit.
pub const CROSS_SECTION_B: [f64; 4] = [6.38e1, -9.95e-1, 6.981e-5, 1.728e-4];
/// Lowest centre-of-mass energy of the cross-section fit, keV; below it the card counts no reactions.
pub const CROSS_SECTION_ENERGY_MIN_KEV: f64 = 0.5;
/// Highest centre-of-mass energy of the cross-section fit's low piece, keV.
pub const CROSS_SECTION_ENERGY_MAX_KEV: f64 = 550.0;
/// Largest injection energy of a deuterium beam, keV: a deuteron on a triton at 550 keV in the centre of mass.
pub const DEUTERIUM_BEAM_ENERGY_MAX_KEV: f64 = CROSS_SECTION_ENERGY_MAX_KEV * (DEUTERON_MASS_KG + TRITON_MASS_KG) / TRITON_MASS_KG;
/// Largest injection energy of a tritium beam, keV: a triton on a deuteron at 550 keV in the centre of mass.
pub const TRITIUM_BEAM_ENERGY_MAX_KEV: f64 = CROSS_SECTION_ENERGY_MAX_KEV * (DEUTERON_MASS_KG + TRITON_MASS_KG) / DEUTERON_MASS_KG;
/// Lowest temperature of the reactivity fit, keV: a colder cell adds no reactions.
pub const REACTIVITY_TEMPERATURE_MIN_KEV: f64 = 0.2;
/// Highest temperature of the reactivity fit, keV.
pub const REACTIVITY_TEMPERATURE_MAX_KEV: f64 = 100.0;
/// Cells of the profiles, equal in volume.
pub const PROFILE_CELLS: usize = 16;
/// Panels of each slowing-down integral.
pub const ENERGY_PANELS: usize = 4;
/// Gauss-Legendre nodes on [−1, 1], eight.
pub const GAUSS_NODES: [f64; 8] = [
    -0.9602898564975363,
    -0.7966664774136267,
    -0.525532409916329,
    -0.1834346424956498,
    0.1834346424956498,
    0.525532409916329,
    0.7966664774136267,
    0.9602898564975363,
];
/// Gauss-Legendre weights of the eight nodes.
pub const GAUSS_WEIGHTS: [f64; 8] = [
    0.1012285362903763,
    0.2223810344533745,
    0.3137066458778873,
    0.362683783378362,
    0.362683783378362,
    0.3137066458778873,
    0.2223810344533745,
    0.1012285362903763,
];
/// Greenwald density per MA of current over `π a²`, m^-3.
pub const GREENWALD_COEFF_PER_M3: f64 = 1.0e20;
/// Troyon limit of the normalised beta, percent m T / MA.
pub const TROYON_BETA_NORMALISED_MAX: f64 = 2.8;
/// Lowest edge safety factor the card accepts.
pub const SAFETY_FACTOR_MIN: f64 = 2.0;
/// Fraction of the Greenwald density above which [`FLAG_NEAR_DENSITY_LIMIT`] is set.
pub const NEAR_DENSITY_LIMIT_FRACTION: f64 = 0.85;
/// `|defect| / max(stores, anchor)` must stay at or under this. Over the ten
/// compared runs through the engine at 1 ms the largest found is 5.1e-11
/// (pulse 38415, `twin/runs/INDEX.json`), and the limit is 390 times it.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.0e-8;
/// Largest step the card stands behind anywhere, s.
pub const GATE_DT_S: f64 = 1.0e-3;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-6;
/// Steps in the shortest of the four times at the state ceiling: over the ten
/// compared runs the shortest is 19.7 ms (`twin/out/envelope.json`), so the
/// ceiling there is at least 2.47 ms and the 1 ms step never meets it.
pub const STATE_CEILING_STEPS: f64 = 8.0;
/// Vacuum permeability, H/m (CODATA 2018).
pub const VACUUM_PERMEABILITY_H_PER_M: f64 = 1.25663706212e-6;
/// π to 64-bit precision.
pub const PI: f64 = 3.141592653589793;

/// The compared envelope: the smallest and largest value over the loads of
/// the ten compared runs, or over every state they start from or land on,
/// rounded outward in the third figure (`twin/envelope_twin.py`,
/// `twin/out/envelope.json`).
pub mod compared {
    /// Heating supplied, `P_D + P_T + P_icrh + P_ohm`, W.
    pub const HEATING_POWER_W: (f64, f64) = (1.31e6, 23.8e6);
    /// Line-averaged density, m^-3.
    pub const LINE_DENSITY_PER_M3: (f64, f64) = (1.19e19, 8.87e19);
    /// Plasma current, A.
    pub const PLASMA_CURRENT_A: (f64, f64) = (1.65e6, 4.16e6);
    /// Toroidal field, T.
    pub const TOROIDAL_FIELD_T: (f64, f64) = (1.80, 4.04);
    /// Central electron temperature, keV.
    pub const ELECTRON_TEMPERATURE_KEV: (f64, f64) = (1.76, 12.0);
    /// Effective charge.
    pub const EFFECTIVE_CHARGE: (f64, f64) = (1.21, 5.30);
    /// Tritium fraction of the fuel.
    pub const TRITIUM_FRACTION: (f64, f64) = (0.0, 0.928);
    /// Thermal energy, J.
    pub const THERMAL_ENERGY_J: (f64, f64) = (0.462e6, 15.2e6);
    /// Fast-ion energy of the three fast stores, J.
    pub const FAST_ION_ENERGY_J: (f64, f64) = (5.59e3, 4.06e6);
    /// Central ion temperature, keV.
    pub const ION_TEMPERATURE_KEV: (f64, f64) = (0.0568, 35.1);
    /// Time since the start of the heated window, s.
    pub const TIME_S: (f64, f64) = (0.0, 5.48);
}

/// The line-averaged density is above 0.85 of the Greenwald density on this frame.
pub const FLAG_NEAR_DENSITY_LIMIT: u64 = 1 << 0;
/// The load or the landed state is outside the compared envelope ([`compared`]); the step computes the same equations.
pub const FLAG_OUTSIDE_COMPARED: u64 = 1 << 1;

/// Why a step did not advance the plasma or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` or a load value was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this tokamak's.
    InvalidCard,
    /// A beam, ion-cyclotron or ohmic power is below zero.
    HeatingPowerNegative,
    /// The heating supplied is zero: the confinement time of the scaling, `P^-0.69`, has no value.
    NoHeatingPower,
    /// A beam's injection energy is zero or less, or its centre-of-mass energy on the target is above 550 keV, the top of the cross-section fit: above 917.25 keV for deuterium and 1373.68 keV for tritium.
    BeamEnergyOutsideCrossSection,
    /// The plasma current is zero or less.
    PlasmaCurrentNotPositive,
    /// The toroidal field is zero or less.
    ToroidalFieldNotPositive,
    /// The tritium fraction of the fuel is below 0 or above 1.
    FuelMixOutsideCard,
    /// The effective charge is below 1, or at or above 6, where carbon leaves no fuel.
    EffectiveChargeOutsideCard,
    /// The line-averaged density is zero or less.
    DensityNotPositive,
    /// The line-averaged density is above the Greenwald density of the plasma current.
    DensityAboveGreenwald,
    /// The edge safety factor of the current and the field is below 2.
    SafetyFactorBelowLimit,
    /// The central electron temperature is zero or less, or the Coulomb logarithm of a cell is zero or less.
    ElectronTemperatureOutsideCard,
    /// `hot_ion_regime` is neither 0 nor 1.
    RegimeNotNamed,
    /// A fast store is negative, or the thermal energy is not above the electrons' energy at the load's electron temperature, so the ions would hold none.
    EnergyOutsideCard,
    /// The central ion temperature is above 100 keV, the ceiling of the reactivity fit.
    TemperatureAboveReactivityFit,
    /// The normalised beta of the four stores is above the Troyon limit of 2.8.
    BetaAboveTroyon,
    /// The clock is negative.
    NegativeTime,
    /// The step is too small to move the clock at its time.
    StepBelowClockResolution,
    /// The step is longer than an eighth of the shortest of the beam ions' and the tail's thermalisation times and the confinement time of the load.
    DtAboveStateCeiling,
    /// The trial step created energy beyond what the heating and the fusion supply.
    SpontaneousGeneration,
    /// The trial step destroyed energy the heating and the fusion supply.
    FirstLawBreach,
}

/// What is applied to the plasma, held through the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwinLoad {
    /// Deuterium beam power absorbed (injected less shine-through and first-orbit loss), `P_D`, W.
    pub deuterium_beam_power_w: f64,
    /// Tritium beam power absorbed, `P_T`, W.
    pub tritium_beam_power_w: f64,
    /// Full injection energy of the deuterium beams, keV.
    pub deuterium_beam_energy_kev: f64,
    /// Full injection energy of the tritium beams, keV.
    pub tritium_beam_energy_kev: f64,
    /// Coupled ion-cyclotron power, `P_icrh`, W.
    pub icrh_power_w: f64,
    /// Ohmic power, `P_ohm`, W.
    pub ohmic_power_w: f64,
    /// Line-averaged electron density, `n_el`, m^-3.
    pub line_density_per_m3: f64,
    /// Plasma current, `Ip`, A.
    pub plasma_current_a: f64,
    /// Toroidal field at the major radius, `Bt`, T.
    pub toroidal_field_t: f64,
    /// Tritium atoms per fuel atom, `f_T`.
    pub tritium_fraction: f64,
    /// Central electron temperature, `Te0`, keV: the pulse's measured trace.
    pub electron_temperature_kev: f64,
    /// Line-averaged effective charge, `Z_eff`: the pulse's measured trace.
    pub effective_charge: f64,
    /// 1 for the ELM-free hot-ion H-mode, 0 for an ELMy H-mode.
    pub hot_ion_regime: f64,
}

/// The locked tokamak. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TokamakJetDte1Twin {
    /// Recorded copy of [`MAJOR_RADIUS_M`].
    pub major_radius_m: f64,
    /// Recorded copy of [`MINOR_RADIUS_M`].
    pub minor_radius_m: f64,
    /// Recorded copy of [`ELONGATION`].
    pub elongation: f64,
    /// Recorded copy of [`PLASMA_VOLUME_M3`].
    pub plasma_volume_m3: f64,
    /// Recorded copy of [`DENSITY_VOLUME_OVER_LINE`].
    pub density_volume_over_line: f64,
    /// Recorded copy of [`DENSITY_EXPONENT`].
    pub density_exponent: f64,
    /// Recorded copy of [`ELECTRON_TEMPERATURE_EXPONENT`].
    pub electron_temperature_exponent: f64,
    /// Recorded copy of [`BEAM_SOURCE_EXPONENT`].
    pub beam_source_exponent: f64,
    /// Recorded copy of [`ION_TEMPERATURE_EXPONENT`].
    pub ion_temperature_exponent: f64,
    /// Recorded copy of [`CONFINEMENT_MULTIPLIER_ELMY`].
    pub confinement_multiplier_elmy: f64,
    /// Recorded copy of [`CONFINEMENT_MULTIPLIER_HOT_ION`].
    pub confinement_multiplier_hot_ion: f64,
    /// Recorded copy of [`SLOWING_TIME_MULTIPLIER`].
    pub slowing_time_multiplier: f64,
    /// Recorded copy of [`TAIL_TIME_MULTIPLIER`].
    pub tail_time_multiplier: f64,
    /// Recorded copy of [`DEUTERIUM_BEAM_SHARES`].
    pub deuterium_beam_shares: [f64; 3],
    /// Recorded copy of [`TRITIUM_BEAM_SHARES`].
    pub tritium_beam_shares: [f64; 3],
    /// Recorded copy of [`REACTIVITY_COEFF`].
    pub reactivity_coeff: [f64; 7],
    /// Recorded copy of [`CROSS_SECTION_A`].
    pub cross_section_a: [f64; 5],
    /// Recorded copy of [`CROSS_SECTION_B`].
    pub cross_section_b: [f64; 4],
    /// Recorded copy of [`TROYON_BETA_NORMALISED_MAX`].
    pub troyon_beta_normalised_max: f64,
    /// Recorded copy of [`SAFETY_FACTOR_MIN`].
    pub safety_factor_min: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`FLOOR_DT_S`].
    pub floor_dt_s: f64,
    /// Recorded copy of [`STATE_CEILING_STEPS`].
    pub state_ceiling_steps: f64,
}

fn same_bits<const N: usize>(a: &[f64; N], b: &[f64; N]) -> bool {
    let mut index = 0;
    while index < N {
        if a[index].to_bits() != b[index].to_bits() {
            return false;
        }
        index += 1;
    }
    true
}

impl TokamakJetDte1Twin {
    /// Card: JET in a DTE1 heated pulse, the twin of ten pulses. The constructor takes no arguments.
    #[must_use]
    pub fn dte1() -> Self {
        let tokamak = Self {
            major_radius_m: MAJOR_RADIUS_M,
            minor_radius_m: MINOR_RADIUS_M,
            elongation: ELONGATION,
            plasma_volume_m3: PLASMA_VOLUME_M3,
            density_volume_over_line: DENSITY_VOLUME_OVER_LINE,
            density_exponent: DENSITY_EXPONENT,
            electron_temperature_exponent: ELECTRON_TEMPERATURE_EXPONENT,
            beam_source_exponent: BEAM_SOURCE_EXPONENT,
            ion_temperature_exponent: ION_TEMPERATURE_EXPONENT,
            confinement_multiplier_elmy: CONFINEMENT_MULTIPLIER_ELMY,
            confinement_multiplier_hot_ion: CONFINEMENT_MULTIPLIER_HOT_ION,
            slowing_time_multiplier: SLOWING_TIME_MULTIPLIER,
            tail_time_multiplier: TAIL_TIME_MULTIPLIER,
            deuterium_beam_shares: DEUTERIUM_BEAM_SHARES,
            tritium_beam_shares: TRITIUM_BEAM_SHARES,
            reactivity_coeff: REACTIVITY_COEFF,
            cross_section_a: CROSS_SECTION_A,
            cross_section_b: CROSS_SECTION_B,
            troyon_beta_normalised_max: TROYON_BETA_NORMALISED_MAX,
            safety_factor_min: SAFETY_FACTOR_MIN,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_dt_s: GATE_DT_S,
            floor_dt_s: FLOOR_DT_S,
            state_ceiling_steps: STATE_CEILING_STEPS,
        };
        if tokamak.is_valid() {
            tokamak
        } else {
            ::core::panic!("JET twin: a recorded constant is not the card's");
        }
    }

    /// Every recorded copy holds its constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        same_bits(&self.deuterium_beam_shares, &DEUTERIUM_BEAM_SHARES)
            && same_bits(&self.tritium_beam_shares, &TRITIUM_BEAM_SHARES)
            && same_bits(&self.reactivity_coeff, &REACTIVITY_COEFF)
            && same_bits(&self.cross_section_a, &CROSS_SECTION_A)
            && same_bits(&self.cross_section_b, &CROSS_SECTION_B)
            && same_bits(
                &[
                    self.major_radius_m,
                    self.minor_radius_m,
                    self.elongation,
                    self.plasma_volume_m3,
                    self.density_volume_over_line,
                    self.density_exponent,
                    self.electron_temperature_exponent,
                    self.beam_source_exponent,
                    self.ion_temperature_exponent,
                    self.confinement_multiplier_elmy,
                    self.confinement_multiplier_hot_ion,
                    self.slowing_time_multiplier,
                    self.tail_time_multiplier,
                    self.troyon_beta_normalised_max,
                    self.safety_factor_min,
                    self.residual_conservation_limit,
                    self.gate_dt_s,
                    self.floor_dt_s,
                    self.state_ceiling_steps,
                ],
                &[
                    MAJOR_RADIUS_M,
                    MINOR_RADIUS_M,
                    ELONGATION,
                    PLASMA_VOLUME_M3,
                    DENSITY_VOLUME_OVER_LINE,
                    DENSITY_EXPONENT,
                    ELECTRON_TEMPERATURE_EXPONENT,
                    BEAM_SOURCE_EXPONENT,
                    ION_TEMPERATURE_EXPONENT,
                    CONFINEMENT_MULTIPLIER_ELMY,
                    CONFINEMENT_MULTIPLIER_HOT_ION,
                    SLOWING_TIME_MULTIPLIER,
                    TAIL_TIME_MULTIPLIER,
                    TROYON_BETA_NORMALISED_MAX,
                    SAFETY_FACTOR_MIN,
                    RESIDUAL_CONSERVATION_LIMIT,
                    GATE_DT_S,
                    FLOOR_DT_S,
                    STATE_CEILING_STEPS,
                ],
            )
    }

    /// Largest step the card stands behind anywhere: [`GATE_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.gate_dt_s.is_finite() && self.gate_dt_s > 0.0 {
            Some(self.gate_dt_s)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`FLOOR_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        if self.floor_dt_s.is_finite() && self.floor_dt_s > 0.0 {
            Some(self.floor_dt_s)
        } else {
            None
        }
    }

    /// The step ceiling of a load, s: an eighth of the shortest of the
    /// thermalisation times of the two beam species and the tail and the
    /// confinement time. `None` for a load outside the card.
    #[must_use]
    pub fn state_ceiling_dt_s(&self, load: TwinLoad) -> Option<f64> {
        if load_in_card(self, load).is_err() {
            return None;
        }
        Some(plasma(self, load).ceiling_s / self.state_ceiling_steps)
    }

    /// Greenwald density of a plasma current, m^-3.
    #[must_use]
    pub fn greenwald_density_per_m3(&self, plasma_current_a: f64) -> f64 {
        GREENWALD_COEFF_PER_M3 * (plasma_current_a / 1.0e6) / (PI * self.minor_radius_m * self.minor_radius_m)
    }

    /// Edge safety factor of a current and a field (the formula of `tokamak_jet_dte1`).
    #[must_use]
    pub fn safety_factor(&self, plasma_current_a: f64, toroidal_field_t: f64) -> f64 {
        let aspect = self.minor_radius_m / self.major_radius_m;
        let shape = (1.17 - 0.65 * aspect) / ((1.0 - aspect * aspect) * (1.0 - aspect * aspect))
            * (1.0 + self.elongation * self.elongation)
            / 2.0;
        5.0 * self.minor_radius_m * self.minor_radius_m * toroidal_field_t / (self.major_radius_m * (plasma_current_a / 1.0e6)) * shape
    }

    /// Normalised beta of the stored energy under a current and a field, percent m T / MA.
    #[must_use]
    pub fn beta_normalised(&self, stored_energy_j: f64, plasma_current_a: f64, toroidal_field_t: f64) -> f64 {
        let pressure_pa = (2.0 / 3.0) * stored_energy_j / self.plasma_volume_m3;
        let beta_percent = 100.0 * 2.0 * VACUUM_PERMEABILITY_H_PER_M * pressure_pa / (toroidal_field_t * toroidal_field_t);
        beta_percent * self.minor_radius_m * toroidal_field_t / (plasma_current_a / 1.0e6)
    }

    /// The energy the electrons hold at the load's measured temperature, J. `None` for a load outside the card.
    #[must_use]
    pub fn electron_energy_j(&self, load: TwinLoad) -> Option<f64> {
        if load_in_card(self, load).is_err() {
            return None;
        }
        Some(plasma(self, load).electron_energy_j)
    }
}

/// Maxwellian reactivity of T(d,n)⁴He at `temperature_kev`, m³/s. Bosch and Hale 1992, equations 12 to 14.
#[must_use]
pub fn reactivity_dt_m3_per_s(temperature_kev: f64) -> f64 {
    let c = &REACTIVITY_COEFF;
    let t = temperature_kev;
    let theta = t / (1.0 - t * (c[1] + t * (c[3] + t * c[5])) / (1.0 + t * (c[2] + t * (c[4] + t * c[6]))));
    let xi = exp(log(REACTIVITY_GAMOW_SQRT_KEV * REACTIVITY_GAMOW_SQRT_KEV / (4.0 * theta)) / 3.0);
    1.0e-6 * c[0] * theta * (xi / (REACTIVITY_REDUCED_MASS_KEV * t * t * t)).sqrt() * exp(-3.0 * xi)
}

/// D-T cross-section at a centre-of-mass energy, m². Zero below 0.5 keV, where the fit begins.
#[must_use]
pub fn cross_section_dt_m2(energy_cm_kev: f64) -> f64 {
    if !(energy_cm_kev >= CROSS_SECTION_ENERGY_MIN_KEV) {
        return 0.0;
    }
    let a = &CROSS_SECTION_A;
    let b = &CROSS_SECTION_B;
    let e = energy_cm_kev;
    let s = (a[0] + e * (a[1] + e * (a[2] + e * (a[3] + e * a[4])))) / (1.0 + e * (b[0] + e * (b[1] + e * (b[2] + e * b[3]))));
    s / (e * exp(REACTIVITY_GAMOW_SQRT_KEV / e.sqrt())) * 1.0e-31
}

/// Everything the load sets, computed once per step.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Plasma {
    /// Fuel ions per electron.
    main_ion_fraction: f64,
    /// Electron density of each cell, m^-3.
    density: [f64; PROFILE_CELLS],
    /// `(1 − x_c)^a_i` of each cell.
    ion_shape: [f64; PROFILE_CELLS],
    /// `W_e`, J.
    electron_energy_j: f64,
    /// Joules of ion energy per keV of central ion temperature.
    ion_energy_per_kev_j: f64,
    /// Thermalisation time of the deuterium and the tritium beam ions, s.
    beam_time_s: [f64; 2],
    /// Fusion reactions per second per joule of deuterium and of tritium beam ions.
    beam_rate_per_j_s: [f64; 2],
    /// Thermalisation time of the tail and the alphas, s.
    tail_time_s: f64,
    /// Confinement time `τ_E`, s.
    confinement_time_s: f64,
    /// The heating supplied, W.
    heating_w: f64,
    /// The shortest of the four times, s.
    ceiling_s: f64,
}

/// `(1 − x_c)^exponent` of each cell.
fn shape(exponent: f64) -> [f64; PROFILE_CELLS] {
    let mut out = [0.0; PROFILE_CELLS];
    let mut cell = 0;
    while cell < PROFILE_CELLS {
        let x = (cell as f64 + 0.5) / PROFILE_CELLS as f64;
        out[cell] = exp(exponent * log(1.0 - x));
        cell += 1;
    }
    out
}

fn sum(values: &[f64; PROFILE_CELLS]) -> f64 {
    let mut total = 0.0;
    let mut cell = 0;
    while cell < PROFILE_CELLS {
        total += values[cell];
        cell += 1;
    }
    total
}

/// The electron temperature of each cell, keV, and its Coulomb logarithm; `None` where a logarithm is not above zero.
fn electrons(tokamak: &TokamakJetDte1Twin, load: TwinLoad) -> Option<([f64; PROFILE_CELLS], [f64; PROFILE_CELLS], [f64; PROFILE_CELLS])> {
    let density_shape = shape(tokamak.density_exponent);
    let mean_shape = sum(&density_shape) / PROFILE_CELLS as f64;
    let electron_shape = shape(tokamak.electron_temperature_exponent);
    let mut density = [0.0; PROFILE_CELLS];
    let mut temperature = [0.0; PROFILE_CELLS];
    let mut coulomb = [0.0; PROFILE_CELLS];
    let mut cell = 0;
    while cell < PROFILE_CELLS {
        density[cell] = tokamak.density_volume_over_line * load.line_density_per_m3 * density_shape[cell] / mean_shape;
        temperature[cell] = load.electron_temperature_kev * electron_shape[cell];
        coulomb[cell] = COULOMB_LOG_CONSTANT - log((density[cell] * 1.0e-6).sqrt() / (temperature[cell] * 1.0e3));
        if !(coulomb[cell] > 0.0) {
            return None;
        }
        cell += 1;
    }
    Some((density, temperature, coulomb))
}

/// The beam species: mass number, mass, target mass, target fuel share, injection energy, power shares.
fn beam_species(tokamak: &TokamakJetDte1Twin, load: TwinLoad, main: f64, species: usize) -> (f64, f64, f64, f64, f64, [f64; 3]) {
    let tritium = load.tritium_fraction;
    if species == 0 {
        (DEUTERON_MASS_KG / ATOMIC_MASS_KG, DEUTERON_MASS_KG, TRITON_MASS_KG, main * tritium, load.deuterium_beam_energy_kev, tokamak.deuterium_beam_shares)
    } else {
        (TRITON_MASS_KG / ATOMIC_MASS_KG, TRITON_MASS_KG, DEUTERON_MASS_KG, main * (1.0 - tritium), load.tritium_beam_energy_kev, tokamak.tritium_beam_shares)
    }
}

/// Everything the load sets. The load must have passed [`load_in_card`].
fn plasma(tokamak: &TokamakJetDte1Twin, load: TwinLoad) -> Plasma {
    let z = load.effective_charge;
    let main = (IMPURITY_CHARGE - z) / (IMPURITY_CHARGE - 1.0);
    let impurity = (z - 1.0) / (IMPURITY_CHARGE * (IMPURITY_CHARGE - 1.0));
    let ions = main + impurity;
    let tritium = load.tritium_fraction;
    let deuterium = 1.0 - tritium;
    let (density, temperature, coulomb) = match electrons(tokamak, load) {
        Some(found) => found,
        None => ([1.0; PROFILE_CELLS], [1.0; PROFILE_CELLS], [1.0; PROFILE_CELLS]),
    };
    let ion_shape = shape(tokamak.ion_temperature_exponent);
    let source_shape = shape(tokamak.beam_source_exponent);
    let source_total = sum(&source_shape);
    let kev_j = 1.0e3 * ELEMENTARY_CHARGE_C;
    let mut electron_sum = 0.0;
    let mut ion_sum = 0.0;
    let mut unit_time = [0.0; PROFILE_CELLS];
    let mut tail_sum = 0.0;
    let mut cell = 0;
    while cell < PROFILE_CELLS {
        electron_sum += density[cell] * temperature[cell];
        ion_sum += density[cell] * ion_shape[cell];
        let te_ev = temperature[cell] * 1.0e3;
        unit_time[cell] = tokamak.slowing_time_multiplier * SLOWING_TIME_COEFF_S * te_ev * te_ev.sqrt() / (density[cell] * 1.0e-6 * coulomb[cell]);
        tail_sum += source_shape[cell] / source_total * unit_time[cell];
        cell += 1;
    }
    let cells = PROFILE_CELLS as f64;
    let electron_energy_j = 1.5 * kev_j * tokamak.plasma_volume_m3 * electron_sum / cells;
    let ion_energy_per_kev_j = 1.5 * kev_j * tokamak.plasma_volume_m3 * ions * ion_sum / cells;
    let bracket = main * (deuterium / (DEUTERON_MASS_KG / ATOMIC_MASS_KG) + tritium / (TRITON_MASS_KG / ATOMIC_MASS_KG))
        + impurity * IMPURITY_CHARGE * IMPURITY_CHARGE / IMPURITY_MASS_NUMBER;
    let bracket_power = exp(2.0 / 3.0 * log(bracket));
    let mut beam_time_s = [0.0; 2];
    let mut beam_rate_per_j_s = [0.0; 2];
    let mut species = 0;
    while species < 2 {
        let (mass_number, mass, target_mass, target_share, energy, shares) = beam_species(tokamak, load, main, species);
        let mut held = 0.0;
        let mut fused = 0.0;
        let mut component = 0;
        while component < 3 {
            let top = energy / (component as f64 + 1.0);
            let weight = shares[component] / top;
            let width = top / ENERGY_PANELS as f64;
            let mut energy_weight = [0.0; PROFILE_CELLS];
            let mut fusion_weight = [0.0; PROFILE_CELLS];
            let mut panel = 0;
            while panel < ENERGY_PANELS {
                let mut node = 0;
                while node < 8 {
                    let e = (panel as f64 + 0.5) * width + 0.5 * width * GAUSS_NODES[node];
                    let w = 0.5 * width * GAUSS_WEIGHTS[node];
                    let speed = (2.0 * e * kev_j / mass).sqrt();
                    let reacting = cross_section_dt_m2(e * target_mass / (mass + target_mass)) * speed;
                    let mut c = 0;
                    while c < PROFILE_CELLS {
                        let ratio = CRITICAL_ENERGY_COEFF * mass_number * temperature[c] * bracket_power / e;
                        let slowing = 1.0 + ratio * ratio.sqrt();
                        energy_weight[c] += w / (2.0 * slowing);
                        fusion_weight[c] += w * reacting / (2.0 * e * slowing);
                        c += 1;
                    }
                    node += 1;
                }
                panel += 1;
            }
            let mut c = 0;
            while c < PROFILE_CELLS {
                let born = source_shape[c] / source_total * weight * mass_number * unit_time[c];
                held += born * energy_weight[c];
                fused += born * target_share * density[c] * fusion_weight[c];
                c += 1;
            }
            component += 1;
        }
        beam_time_s[species] = held;
        beam_rate_per_j_s[species] = fused / held / kev_j;
        species += 1;
    }
    let tail_time_s = tokamak.tail_time_multiplier * tail_sum / 2.0;
    let n19 = load.line_density_per_m3 / 1.0e19;
    let aspect = tokamak.minor_radius_m / tokamak.major_radius_m;
    let multiplier = if load.hot_ion_regime == 1.0 {
        tokamak.confinement_multiplier_hot_ion
    } else {
        tokamak.confinement_multiplier_elmy
    };
    let heating_w = load.deuterium_beam_power_w + load.tritium_beam_power_w + load.icrh_power_w + load.ohmic_power_w;
    let confinement_time_s = multiplier
        * IPB98_COEFF
        * exp(IPB98_EXP_CURRENT * log(load.plasma_current_a / 1.0e6))
        * exp(IPB98_EXP_FIELD * log(load.toroidal_field_t))
        * exp(IPB98_EXP_DENSITY * log(n19))
        * exp(IPB98_EXP_MASS * log(2.0 + tritium))
        * exp(IPB98_EXP_RADIUS * log(tokamak.major_radius_m))
        * exp(IPB98_EXP_ASPECT * log(aspect))
        * exp(IPB98_EXP_ELONGATION * log(tokamak.elongation))
        * exp(IPB98_EXP_POWER * log(heating_w / 1.0e6));
    let mut ceiling_s = confinement_time_s;
    for time in [beam_time_s[0], beam_time_s[1], tail_time_s] {
        if time < ceiling_s {
            ceiling_s = time;
        }
    }
    Plasma {
        main_ion_fraction: main,
        density,
        ion_shape,
        electron_energy_j,
        ion_energy_per_kev_j,
        beam_time_s,
        beam_rate_per_j_s,
        tail_time_s,
        confinement_time_s,
        heating_w,
        ceiling_s,
    }
}

/// Integrated plasma. The derived and ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwinState {
    /// Time since the start of the heated window, s.
    pub t_s: f64,
    /// Thermal energy of the plasma, `W_th`, J.
    pub thermal_energy_j: f64,
    /// Energy of the deuterium beam ions, `W_D`, J.
    pub deuterium_beam_ion_energy_j: f64,
    /// Energy of the tritium beam ions, `W_T`, J.
    pub tritium_beam_ion_energy_j: f64,
    /// Energy of the ion-cyclotron tail and the fusion alphas, `W_x`, J.
    pub tail_ion_energy_j: f64,
    /// D-T fusion energy released since the start, `E_fus`, J.
    pub fusion_energy_j: f64,
    /// `W_D + W_T + W_x`, J.
    pub fast_ion_energy_j: f64,
    /// The electrons' share of the thermal energy at their measured temperature, `W_e`, J. Zero at the start.
    pub electron_energy_j: f64,
    /// Central ion temperature `Ti0`, keV. Zero at the start.
    pub ion_temperature_kev: f64,
    /// Loss power `W_th / τ_E`, W. Zero at the start.
    pub loss_power_w: f64,
    /// Confinement time `τ_E`, s. Zero at the start.
    pub confinement_time_s: f64,
    /// D-T fusion power, W. Zero at the start.
    pub fusion_power_w: f64,
    /// Its thermal part, W. Zero at the start.
    pub fusion_power_thermal_w: f64,
    /// Its part from deuterium beam ions on the plasma's tritons, W. Zero at the start.
    pub fusion_power_deuterium_beam_w: f64,
    /// Its part from tritium beam ions on the plasma's deuterons, W. Zero at the start.
    pub fusion_power_tritium_beam_w: f64,
    /// Normalised beta of the four stores, percent m T / MA. Zero at the start.
    pub beta_normalised: f64,
    /// Line-averaged density over the Greenwald density. Zero at the start.
    pub greenwald_fraction: f64,
    /// Edge safety factor of the load of the last step. Zero at the start.
    pub safety_factor: f64,
    /// The four stores at the state the ledger started from, J.
    pub energy_anchor_j: f64,
    /// `Σ (P_D + P_T + P_icrh + P_ohm) dt` since the anchor, J.
    pub energy_supplied_j: f64,
    /// `Σ Q`, the loss energy since the anchor by the corrected trapezoid rule, J.
    pub energy_lost_j: f64,
    /// Signed defect of the ledger, J. Positive is created energy.
    pub energy_defect_j: f64,
    /// `|energy_defect_j| / max(stores, anchor)`.
    pub residual_conservation_error: f64,
    /// The density is above 0.85 of the Greenwald density.
    pub near_density_limit: bool,
    /// The load or the landed state is outside the compared envelope.
    pub outside_compared: bool,
}

impl TwinState {
    /// The plasma with its four stores at time 0: [`TwinState::try_continue`] at zero.
    #[must_use]
    pub fn try_start(
        tokamak: &TokamakJetDte1Twin,
        thermal_energy_j: f64,
        deuterium_beam_ion_energy_j: f64,
        tritium_beam_ion_energy_j: f64,
        tail_ion_energy_j: f64,
    ) -> Option<Self> {
        Self::try_continue(tokamak, 0.0, thermal_energy_j, deuterium_beam_ion_energy_j, tritium_beam_ion_energy_j, tail_ion_energy_j)
    }

    /// The plasma with its four stores at `elapsed_time_s`, nothing
    /// released, the ledger anchored. `None` unless every number is finite,
    /// the time and the fast stores are not negative and the thermal energy
    /// is above zero.
    #[must_use]
    pub fn try_continue(
        tokamak: &TokamakJetDte1Twin,
        elapsed_time_s: f64,
        thermal_energy_j: f64,
        deuterium_beam_ion_energy_j: f64,
        tritium_beam_ion_energy_j: f64,
        tail_ion_energy_j: f64,
    ) -> Option<Self> {
        if !tokamak.is_valid()
            || !elapsed_time_s.is_finite()
            || !(elapsed_time_s >= 0.0)
            || !thermal_energy_j.is_finite()
            || !(thermal_energy_j > 0.0)
            || !deuterium_beam_ion_energy_j.is_finite()
            || !(deuterium_beam_ion_energy_j >= 0.0)
            || !tritium_beam_ion_energy_j.is_finite()
            || !(tritium_beam_ion_energy_j >= 0.0)
            || !tail_ion_energy_j.is_finite()
            || !(tail_ion_energy_j >= 0.0)
        {
            return None;
        }
        let fast = deuterium_beam_ion_energy_j + tritium_beam_ion_energy_j + tail_ion_energy_j;
        let anchor = thermal_energy_j + fast;
        if !anchor.is_finite() {
            return None;
        }
        Some(Self {
            t_s: elapsed_time_s,
            thermal_energy_j,
            deuterium_beam_ion_energy_j,
            tritium_beam_ion_energy_j,
            tail_ion_energy_j,
            fusion_energy_j: 0.0,
            fast_ion_energy_j: fast,
            electron_energy_j: 0.0,
            ion_temperature_kev: 0.0,
            loss_power_w: 0.0,
            confinement_time_s: 0.0,
            fusion_power_w: 0.0,
            fusion_power_thermal_w: 0.0,
            fusion_power_deuterium_beam_w: 0.0,
            fusion_power_tritium_beam_w: 0.0,
            beta_normalised: 0.0,
            greenwald_fraction: 0.0,
            safety_factor: 0.0,
            energy_anchor_j: anchor,
            energy_supplied_j: 0.0,
            energy_lost_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            near_density_limit: false,
            outside_compared: false,
        })
    }
}

/// The powers of one state under one load.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Powers {
    ion_temperature_kev: f64,
    loss_power_w: f64,
    thermal_w: f64,
    deuterium_beam_w: f64,
    tritium_beam_w: f64,
    fusion_w: f64,
}

const STORES: usize = 5;

/// Advance one Runge–Kutta step and write the frame. A rejected step leaves
/// `state` and `frame` as they were. A value of the landed state below the
/// normal range is written as zero with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    tokamak: &TokamakJetDte1Twin,
    state: &mut TwinState,
    frame: &mut LastStateFrame64,
    load: TwinLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(tokamak, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("tokamak_jet_dte1_twin", exceedances(tokamak, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    tokamak: &TokamakJetDte1Twin,
    state: &mut TwinState,
    frame: &mut LastStateFrame64,
    load: TwinLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(tokamak, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

fn step_as_computed(
    tokamak: &TokamakJetDte1Twin,
    state: &mut TwinState,
    frame: &mut LastStateFrame64,
    load: TwinLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    let p = validate(tokamak, state, load, dt_s)?;
    let advanced = rk4(tokamak, &p, load, &state_vector(state), dt_s);
    commit(tokamak, &p, state, frame, load, dt_s, advanced)
}

fn normal_or_zero_state(state: &mut TwinState) {
    state.t_s = normal_or_zero(state.t_s);
    state.thermal_energy_j = normal_or_zero(state.thermal_energy_j);
    state.deuterium_beam_ion_energy_j = normal_or_zero(state.deuterium_beam_ion_energy_j);
    state.tritium_beam_ion_energy_j = normal_or_zero(state.tritium_beam_ion_energy_j);
    state.tail_ion_energy_j = normal_or_zero(state.tail_ion_energy_j);
    state.fusion_energy_j = normal_or_zero(state.fusion_energy_j);
    state.fast_ion_energy_j = normal_or_zero(state.fast_ion_energy_j);
    state.electron_energy_j = normal_or_zero(state.electron_energy_j);
    state.ion_temperature_kev = normal_or_zero(state.ion_temperature_kev);
    state.loss_power_w = normal_or_zero(state.loss_power_w);
    state.confinement_time_s = normal_or_zero(state.confinement_time_s);
    state.fusion_power_w = normal_or_zero(state.fusion_power_w);
    state.fusion_power_thermal_w = normal_or_zero(state.fusion_power_thermal_w);
    state.fusion_power_deuterium_beam_w = normal_or_zero(state.fusion_power_deuterium_beam_w);
    state.fusion_power_tritium_beam_w = normal_or_zero(state.fusion_power_tritium_beam_w);
    state.beta_normalised = normal_or_zero(state.beta_normalised);
    state.greenwald_fraction = normal_or_zero(state.greenwald_fraction);
    state.safety_factor = normal_or_zero(state.safety_factor);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.energy_supplied_j = normal_or_zero(state.energy_supplied_j);
    state.energy_lost_j = normal_or_zero(state.energy_lost_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

/// The load held to the card. The step tests this first; a caller may test a load with it before a run.
pub fn load_in_card(tokamak: &TokamakJetDte1Twin, load: TwinLoad) -> Result<(), StepReject> {
    if load.deuterium_beam_power_w < 0.0 || load.tritium_beam_power_w < 0.0 || load.icrh_power_w < 0.0 || load.ohmic_power_w < 0.0 {
        return Err(StepReject::HeatingPowerNegative);
    }
    if !(load.deuterium_beam_power_w + load.tritium_beam_power_w + load.icrh_power_w + load.ohmic_power_w > 0.0) {
        return Err(StepReject::NoHeatingPower);
    }
    if !(load.deuterium_beam_energy_kev > 0.0)
        || !(load.tritium_beam_energy_kev > 0.0)
        || load.deuterium_beam_energy_kev > DEUTERIUM_BEAM_ENERGY_MAX_KEV
        || load.tritium_beam_energy_kev > TRITIUM_BEAM_ENERGY_MAX_KEV
    {
        return Err(StepReject::BeamEnergyOutsideCrossSection);
    }
    if !(load.plasma_current_a > 0.0) {
        return Err(StepReject::PlasmaCurrentNotPositive);
    }
    if !(load.toroidal_field_t > 0.0) {
        return Err(StepReject::ToroidalFieldNotPositive);
    }
    if load.tritium_fraction < 0.0 || load.tritium_fraction > 1.0 {
        return Err(StepReject::FuelMixOutsideCard);
    }
    if load.effective_charge < 1.0 || load.effective_charge >= IMPURITY_CHARGE {
        return Err(StepReject::EffectiveChargeOutsideCard);
    }
    if !(load.hot_ion_regime == 0.0 || load.hot_ion_regime == 1.0) {
        return Err(StepReject::RegimeNotNamed);
    }
    if !(load.line_density_per_m3 > 0.0) {
        return Err(StepReject::DensityNotPositive);
    }
    if load.line_density_per_m3 > tokamak.greenwald_density_per_m3(load.plasma_current_a) {
        return Err(StepReject::DensityAboveGreenwald);
    }
    if tokamak.safety_factor(load.plasma_current_a, load.toroidal_field_t) < tokamak.safety_factor_min {
        return Err(StepReject::SafetyFactorBelowLimit);
    }
    if !(load.electron_temperature_kev > 0.0) || electrons(tokamak, load).is_none() {
        return Err(StepReject::ElectronTemperatureOutsideCard);
    }
    Ok(())
}

/// The four stores held to the card under a load: the fast stores not
/// negative, the thermal energy above the electrons', the central ion
/// temperature in the reactivity fit and the stores under the Troyon limit.
fn stores_in_card(tokamak: &TokamakJetDte1Twin, p: &Plasma, y: &[f64; STORES], load: TwinLoad) -> Result<(), StepReject> {
    if !(y[1] >= 0.0) || !(y[2] >= 0.0) || !(y[3] >= 0.0) || !(y[0] > p.electron_energy_j) {
        return Err(StepReject::EnergyOutsideCard);
    }
    if (y[0] - p.electron_energy_j) / p.ion_energy_per_kev_j > REACTIVITY_TEMPERATURE_MAX_KEV {
        return Err(StepReject::TemperatureAboveReactivityFit);
    }
    if tokamak.beta_normalised(y[0] + y[1] + y[2] + y[3], load.plasma_current_a, load.toroidal_field_t) > tokamak.troyon_beta_normalised_max {
        return Err(StepReject::BetaAboveTroyon);
    }
    Ok(())
}

/// A start held to the card under a load: what the step tests at its start. A caller may test a start with it.
pub fn state_in_card(tokamak: &TokamakJetDte1Twin, state: &TwinState, load: TwinLoad) -> Result<(), StepReject> {
    load_in_card(tokamak, load)?;
    stores_in_card(tokamak, &plasma(tokamak, load), &state_vector(state), load)
}

fn validate(tokamak: &TokamakJetDte1Twin, state: &TwinState, load: TwinLoad, dt_s: f64) -> Result<Plasma, StepReject> {
    if !dt_s.is_finite()
        || !load.deuterium_beam_power_w.is_finite()
        || !load.tritium_beam_power_w.is_finite()
        || !load.deuterium_beam_energy_kev.is_finite()
        || !load.tritium_beam_energy_kev.is_finite()
        || !load.icrh_power_w.is_finite()
        || !load.ohmic_power_w.is_finite()
        || !load.line_density_per_m3.is_finite()
        || !load.plasma_current_a.is_finite()
        || !load.toroidal_field_t.is_finite()
        || !load.tritium_fraction.is_finite()
        || !load.electron_temperature_kev.is_finite()
        || !load.effective_charge.is_finite()
        || !load.hot_ion_regime.is_finite()
    {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !tokamak.is_valid() {
        return Err(StepReject::InvalidCard);
    }
    load_in_card(tokamak, load)?;
    let p = plasma(tokamak, load);
    if !plasma_is_finite(&p) {
        return Err(StepReject::NonFiniteState);
    }
    stores_in_card(tokamak, &p, &state_vector(state), load)?;
    if !(state.t_s >= 0.0) {
        return Err(StepReject::NegativeTime);
    }
    if !(state.t_s + dt_s > state.t_s) {
        return Err(StepReject::StepBelowClockResolution);
    }
    if dt_s > p.ceiling_s / tokamak.state_ceiling_steps {
        return Err(StepReject::DtAboveStateCeiling);
    }
    Ok(p)
}

fn plasma_is_finite(p: &Plasma) -> bool {
    p.electron_energy_j.is_finite()
        && p.ion_energy_per_kev_j > 0.0
        && p.ion_energy_per_kev_j.is_finite()
        && p.beam_time_s[0] > 0.0
        && p.beam_time_s[1] > 0.0
        && p.beam_time_s[0].is_finite()
        && p.beam_time_s[1].is_finite()
        && p.beam_rate_per_j_s[0].is_finite()
        && p.beam_rate_per_j_s[1].is_finite()
        && p.tail_time_s > 0.0
        && p.tail_time_s.is_finite()
        && p.confinement_time_s > 0.0
        && p.confinement_time_s.is_finite()
}

/// The powers of the stores `y` under the load.
fn powers(tokamak: &TokamakJetDte1Twin, p: &Plasma, load: TwinLoad, y: &[f64; STORES]) -> Powers {
    let ion_temperature_kev = (y[0] - p.electron_energy_j) / p.ion_energy_per_kev_j;
    let tritium = load.tritium_fraction;
    let deuterium = 1.0 - tritium;
    let thermal_w = if deuterium * tritium > 0.0 && ion_temperature_kev > 0.0 {
        let mut total = 0.0;
        let mut cell = 0;
        while cell < PROFILE_CELLS {
            let t = ion_temperature_kev * p.ion_shape[cell];
            if t >= REACTIVITY_TEMPERATURE_MIN_KEV {
                total += p.density[cell] * p.density[cell] * reactivity_dt_m3_per_s(t);
            }
            cell += 1;
        }
        DT_REACTION_ENERGY_J * deuterium * tritium * p.main_ion_fraction * p.main_ion_fraction * tokamak.plasma_volume_m3 * total
            / PROFILE_CELLS as f64
    } else {
        0.0
    };
    let deuterium_beam_w = DT_REACTION_ENERGY_J * p.beam_rate_per_j_s[0] * y[1];
    let tritium_beam_w = DT_REACTION_ENERGY_J * p.beam_rate_per_j_s[1] * y[2];
    Powers {
        ion_temperature_kev,
        loss_power_w: y[0] / p.confinement_time_s,
        thermal_w,
        deuterium_beam_w,
        tritium_beam_w,
        fusion_w: thermal_w + deuterium_beam_w + tritium_beam_w,
    }
}

/// The right-hand side of the five equations at `y`.
fn rates(tokamak: &TokamakJetDte1Twin, p: &Plasma, load: TwinLoad, y: &[f64; STORES]) -> ([f64; STORES], Powers) {
    let w = powers(tokamak, p, load, y);
    let deuterium = y[1] / p.beam_time_s[0];
    let tritium = y[2] / p.beam_time_s[1];
    let tail = y[3] / p.tail_time_s;
    (
        [
            deuterium + tritium + tail + load.ohmic_power_w - w.loss_power_w,
            load.deuterium_beam_power_w - deuterium,
            load.tritium_beam_power_w - tritium,
            load.icrh_power_w + ALPHA_ENERGY_SHARE * w.fusion_w - tail,
            w.fusion_w,
        ],
        w,
    )
}

fn advance(y: &[f64; STORES], k: &[f64; STORES], scale: f64) -> [f64; STORES] {
    let mut out = [0.0; STORES];
    let mut index = 0;
    while index < STORES {
        out[index] = y[index] + scale * k[index];
        index += 1;
    }
    out
}

/// Classical fourth-order Runge–Kutta.
fn rk4(tokamak: &TokamakJetDte1Twin, p: &Plasma, load: TwinLoad, y: &[f64; STORES], dt_s: f64) -> [f64; STORES] {
    let k1 = rates(tokamak, p, load, y).0;
    let k2 = rates(tokamak, p, load, &advance(y, &k1, 0.5 * dt_s)).0;
    let k3 = rates(tokamak, p, load, &advance(y, &k2, 0.5 * dt_s)).0;
    let k4 = rates(tokamak, p, load, &advance(y, &k3, dt_s)).0;
    let mut out = [0.0; STORES];
    let mut index = 0;
    while index < STORES {
        out[index] = y[index] + dt_s / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    out
}

fn inside(value: f64, bounds: (f64, f64)) -> bool {
    value >= bounds.0 && value <= bounds.1
}

/// Whether the load and the landed state are inside the compared envelope.
fn in_compared(load: TwinLoad, p: &Plasma, state: &TwinState) -> bool {
    inside(p.heating_w, compared::HEATING_POWER_W)
        && inside(load.line_density_per_m3, compared::LINE_DENSITY_PER_M3)
        && inside(load.plasma_current_a, compared::PLASMA_CURRENT_A)
        && inside(load.toroidal_field_t, compared::TOROIDAL_FIELD_T)
        && inside(load.electron_temperature_kev, compared::ELECTRON_TEMPERATURE_KEV)
        && inside(load.effective_charge, compared::EFFECTIVE_CHARGE)
        && inside(load.tritium_fraction, compared::TRITIUM_FRACTION)
        && inside(state.thermal_energy_j, compared::THERMAL_ENERGY_J)
        && inside(state.fast_ion_energy_j, compared::FAST_ION_ENERGY_J)
        && inside(state.ion_temperature_kev, compared::ION_TEMPERATURE_KEV)
        && inside(state.t_s, compared::TIME_S)
}

fn commit(
    tokamak: &TokamakJetDte1Twin,
    p: &Plasma,
    state: &mut TwinState,
    frame: &mut LastStateFrame64,
    load: TwinLoad,
    dt_s: f64,
    advanced: [f64; STORES],
) -> Result<(), StepReject> {
    let mut index = 0;
    while index < STORES {
        if !advanced[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    stores_in_card(tokamak, p, &advanced, load)?;
    let saved = *state;
    let old = state_vector(state);
    let (rate_old, _) = rates(tokamak, p, load, &old);
    let (rate_new, w) = rates(tokamak, p, load, &advanced);
    let loss_old = old[0] / p.confinement_time_s;
    let loss_new = advanced[0] / p.confinement_time_s;
    let loss_energy = 0.5 * dt_s * (loss_old + loss_new)
        + dt_s * dt_s / 12.0 * (rate_old[0] / p.confinement_time_s - rate_new[0] / p.confinement_time_s);
    state.t_s += dt_s;
    state.thermal_energy_j = advanced[0];
    state.deuterium_beam_ion_energy_j = advanced[1];
    state.tritium_beam_ion_energy_j = advanced[2];
    state.tail_ion_energy_j = advanced[3];
    state.fusion_energy_j = advanced[4];
    state.fast_ion_energy_j = advanced[1] + advanced[2] + advanced[3];
    state.electron_energy_j = p.electron_energy_j;
    state.ion_temperature_kev = w.ion_temperature_kev;
    state.loss_power_w = w.loss_power_w;
    state.confinement_time_s = p.confinement_time_s;
    state.fusion_power_w = w.fusion_w;
    state.fusion_power_thermal_w = w.thermal_w;
    state.fusion_power_deuterium_beam_w = w.deuterium_beam_w;
    state.fusion_power_tritium_beam_w = w.tritium_beam_w;
    state.beta_normalised = tokamak.beta_normalised(
        state.thermal_energy_j + state.fast_ion_energy_j,
        load.plasma_current_a,
        load.toroidal_field_t,
    );
    state.greenwald_fraction = load.line_density_per_m3 / tokamak.greenwald_density_per_m3(load.plasma_current_a);
    state.safety_factor = tokamak.safety_factor(load.plasma_current_a, load.toroidal_field_t);
    state.energy_supplied_j += p.heating_w * dt_s;
    state.energy_lost_j += loss_energy;
    let stored = state.thermal_energy_j + state.fast_ion_energy_j;
    state.energy_defect_j = stored
        - state.energy_anchor_j
        - (state.energy_supplied_j + ALPHA_ENERGY_SHARE * state.fusion_energy_j - state.energy_lost_j);
    let scale = if stored > state.energy_anchor_j { stored } else { state.energy_anchor_j };
    state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    state.near_density_limit = state.greenwald_fraction > NEAR_DENSITY_LIMIT_FRACTION;
    state.outside_compared = !in_compared(load, p, state);
    if !state_is_finite(state) || !(scale > 0.0) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > tokamak.residual_conservation_limit {
        let created = state.energy_defect_j > 0.0;
        *state = saved;
        return Err(if created { StepReject::SpontaneousGeneration } else { StepReject::FirstLawBreach });
    }
    publish(state, frame);
    Ok(())
}

/// The five variables in the order of the Runge–Kutta step: W_th, W_D, W_T, W_x, E_fus.
fn state_vector(state: &TwinState) -> [f64; STORES] {
    [
        state.thermal_energy_j,
        state.deuterium_beam_ion_energy_j,
        state.tritium_beam_ion_energy_j,
        state.tail_ion_energy_j,
        state.fusion_energy_j,
    ]
}

fn publish(state: &TwinState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.thermal_energy_j as f32;
    frame.pos[1] = state.fast_ion_energy_j as f32;
    frame.pos[2] = state.fusion_energy_j as f32;
    frame.vel[0] = state.fusion_power_w as f32;
    frame.vel[1] = state.ion_temperature_kev as f32;
    frame.vel[2] = state.beta_normalised as f32;
    frame.force_torque = state.loss_power_w as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.near_density_limit {
        flags |= FLAG_NEAR_DENSITY_LIMIT;
    }
    if state.outside_compared {
        flags |= FLAG_OUTSIDE_COMPARED;
    }
    frame.flags = flags;
}

fn state_is_finite(state: &TwinState) -> bool {
    state.t_s.is_finite()
        && state.thermal_energy_j.is_finite()
        && state.deuterium_beam_ion_energy_j.is_finite()
        && state.tritium_beam_ion_energy_j.is_finite()
        && state.tail_ion_energy_j.is_finite()
        && state.fusion_energy_j.is_finite()
        && state.fast_ion_energy_j.is_finite()
        && state.electron_energy_j.is_finite()
        && state.ion_temperature_kev.is_finite()
        && state.loss_power_w.is_finite()
        && state.confinement_time_s.is_finite()
        && state.fusion_power_w.is_finite()
        && state.fusion_power_thermal_w.is_finite()
        && state.fusion_power_deuterium_beam_w.is_finite()
        && state.fusion_power_tritium_beam_w.is_finite()
        && state.beta_normalised.is_finite()
        && state.greenwald_fraction.is_finite()
        && state.safety_factor.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.energy_supplied_j.is_finite()
        && state.energy_lost_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
}


/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    DENSITY_KIND,
    HEATING_LOW,
    HEATING_HIGH,
    LINE_DENSITY_LOW,
    LINE_DENSITY_HIGH,
    CURRENT_LOW,
    CURRENT_HIGH,
    FIELD_LOW,
    FIELD_HIGH,
    ELECTRON_TEMPERATURE_LOW,
    ELECTRON_TEMPERATURE_HIGH,
    CHARGE_LOW,
    CHARGE_HIGH,
    TRITIUM_LOW,
    TRITIUM_HIGH,
    THERMAL_LOW,
    THERMAL_HIGH,
    FAST_ION_LOW,
    FAST_ION_HIGH,
    ION_TEMPERATURE_LOW,
    ION_TEMPERATURE_HIGH,
    TIME_LOW,
    TIME_HIGH,
];

use crate::exceedance::{Exceedance, ExceedanceKind, Exceedances, Side};

const DENSITY_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_NEAR_DENSITY_LIMIT",
    FLAG_NEAR_DENSITY_LIMIT,
    "greenwald_fraction",
    "1",
    Side::Above,
    NEAR_DENSITY_LIMIT_FRACTION,
    "0.85 of the Greenwald density at the step's plasma current",
);
const fn outside(quantity: &'static str, unit: &'static str, side: Side, limit: f64, words: &'static str) -> ExceedanceKind {
    ExceedanceKind::fixed("FLAG_OUTSIDE_COMPARED", FLAG_OUTSIDE_COMPARED, quantity, unit, side, limit, words)
}
const HEATING_WORDS: &str = "the heating supplied over the compared runs, P_D + P_T + P_icrh + P_ohm, 1.31 to 23.8 MW";
const HEATING_LOW: ExceedanceKind = outside("heating_power_w", "W", Side::Below, compared::HEATING_POWER_W.0, HEATING_WORDS);
const HEATING_HIGH: ExceedanceKind = outside("heating_power_w", "W", Side::Above, compared::HEATING_POWER_W.1, HEATING_WORDS);
const LINE_DENSITY_WORDS: &str = "the line-averaged density over the compared runs, 1.19e19 to 8.87e19 m^-3";
const LINE_DENSITY_LOW: ExceedanceKind = outside("line_density_per_m3", "m^-3", Side::Below, compared::LINE_DENSITY_PER_M3.0, LINE_DENSITY_WORDS);
const LINE_DENSITY_HIGH: ExceedanceKind = outside("line_density_per_m3", "m^-3", Side::Above, compared::LINE_DENSITY_PER_M3.1, LINE_DENSITY_WORDS);
const CURRENT_WORDS: &str = "the plasma current over the compared runs, 1.65 to 4.16 MA";
const CURRENT_LOW: ExceedanceKind = outside("plasma_current_a", "A", Side::Below, compared::PLASMA_CURRENT_A.0, CURRENT_WORDS);
const CURRENT_HIGH: ExceedanceKind = outside("plasma_current_a", "A", Side::Above, compared::PLASMA_CURRENT_A.1, CURRENT_WORDS);
const FIELD_WORDS: &str = "the toroidal field over the compared runs, 1.80 to 4.04 T";
const FIELD_LOW: ExceedanceKind = outside("toroidal_field_t", "T", Side::Below, compared::TOROIDAL_FIELD_T.0, FIELD_WORDS);
const FIELD_HIGH: ExceedanceKind = outside("toroidal_field_t", "T", Side::Above, compared::TOROIDAL_FIELD_T.1, FIELD_WORDS);
const ELECTRON_TEMPERATURE_WORDS: &str = "the central electron temperature over the compared runs, 1.76 to 12.0 keV";
const ELECTRON_TEMPERATURE_LOW: ExceedanceKind = outside("electron_temperature_kev", "keV", Side::Below, compared::ELECTRON_TEMPERATURE_KEV.0, ELECTRON_TEMPERATURE_WORDS);
const ELECTRON_TEMPERATURE_HIGH: ExceedanceKind = outside("electron_temperature_kev", "keV", Side::Above, compared::ELECTRON_TEMPERATURE_KEV.1, ELECTRON_TEMPERATURE_WORDS);
const CHARGE_WORDS: &str = "the effective charge over the compared runs, 1.21 to 5.30";
const CHARGE_LOW: ExceedanceKind = outside("effective_charge", "1", Side::Below, compared::EFFECTIVE_CHARGE.0, CHARGE_WORDS);
const CHARGE_HIGH: ExceedanceKind = outside("effective_charge", "1", Side::Above, compared::EFFECTIVE_CHARGE.1, CHARGE_WORDS);
const TRITIUM_WORDS: &str = "the tritium fraction of the fuel over the compared runs, 0 to 0.928";
const TRITIUM_LOW: ExceedanceKind = outside("tritium_fraction", "1", Side::Below, compared::TRITIUM_FRACTION.0, TRITIUM_WORDS);
const TRITIUM_HIGH: ExceedanceKind = outside("tritium_fraction", "1", Side::Above, compared::TRITIUM_FRACTION.1, TRITIUM_WORDS);
const THERMAL_WORDS: &str = "the thermal energy over every state the compared runs start from or land on, 0.462 to 15.2 MJ";
const THERMAL_LOW: ExceedanceKind = outside("thermal_energy_j", "J", Side::Below, compared::THERMAL_ENERGY_J.0, THERMAL_WORDS);
const THERMAL_HIGH: ExceedanceKind = outside("thermal_energy_j", "J", Side::Above, compared::THERMAL_ENERGY_J.1, THERMAL_WORDS);
const FAST_ION_WORDS: &str = "the fast-ion energy of the three fast stores over every state the compared runs start from or land on, 5.59 kJ to 4.06 MJ";
const FAST_ION_LOW: ExceedanceKind = outside("fast_ion_energy_j", "J", Side::Below, compared::FAST_ION_ENERGY_J.0, FAST_ION_WORDS);
const FAST_ION_HIGH: ExceedanceKind = outside("fast_ion_energy_j", "J", Side::Above, compared::FAST_ION_ENERGY_J.1, FAST_ION_WORDS);
const ION_TEMPERATURE_WORDS: &str = "the central ion temperature over every state the compared runs start from or land on, 0.0568 to 35.1 keV";
const ION_TEMPERATURE_LOW: ExceedanceKind = outside("ion_temperature_kev", "keV", Side::Below, compared::ION_TEMPERATURE_KEV.0, ION_TEMPERATURE_WORDS);
const ION_TEMPERATURE_HIGH: ExceedanceKind = outside("ion_temperature_kev", "keV", Side::Above, compared::ION_TEMPERATURE_KEV.1, ION_TEMPERATURE_WORDS);
const TIME_WORDS: &str = "the time since the start of the heated window over the compared runs, 0 to 5.48 s";
const TIME_LOW: ExceedanceKind = outside("t_s", "s", Side::Below, compared::TIME_S.0, TIME_WORDS);
const TIME_HIGH: ExceedanceKind = outside("t_s", "s", Side::Above, compared::TIME_S.1, TIME_WORDS);

/// The records of a value outside a window of the compared envelope, one per
/// edge it is past, by the comparison [`inside`] holds it to.
fn window(records: &mut Exceedances, value: f64, bounds: (f64, f64), low: ExceedanceKind, high: ExceedanceKind) {
    records.push_if(!(value >= bounds.0), Exceedance::fixed(low, value));
    records.push_if(!(value <= bounds.1), Exceedance::fixed(high, value));
}

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks. The
/// conditions are those the step writes `near_density_limit` and
/// [`in_compared`] by.
pub fn exceedances(
    tokamak: &TokamakJetDte1Twin,
    before: &TwinState,
    after: &TwinState,
    load: TwinLoad,
    dt_s: f64,
) -> Exceedances {
    let _ = (before, dt_s);
    let mut records = Exceedances::new();
    records.push_if(after.near_density_limit, Exceedance::fixed(DENSITY_KIND, after.greenwald_fraction));
    if after.outside_compared {
        let p = plasma(tokamak, load);
        window(&mut records, p.heating_w, compared::HEATING_POWER_W, HEATING_LOW, HEATING_HIGH);
        window(&mut records, load.line_density_per_m3, compared::LINE_DENSITY_PER_M3, LINE_DENSITY_LOW, LINE_DENSITY_HIGH);
        window(&mut records, load.plasma_current_a, compared::PLASMA_CURRENT_A, CURRENT_LOW, CURRENT_HIGH);
        window(&mut records, load.toroidal_field_t, compared::TOROIDAL_FIELD_T, FIELD_LOW, FIELD_HIGH);
        window(&mut records, load.electron_temperature_kev, compared::ELECTRON_TEMPERATURE_KEV, ELECTRON_TEMPERATURE_LOW, ELECTRON_TEMPERATURE_HIGH);
        window(&mut records, load.effective_charge, compared::EFFECTIVE_CHARGE, CHARGE_LOW, CHARGE_HIGH);
        window(&mut records, load.tritium_fraction, compared::TRITIUM_FRACTION, TRITIUM_LOW, TRITIUM_HIGH);
        window(&mut records, after.thermal_energy_j, compared::THERMAL_ENERGY_J, THERMAL_LOW, THERMAL_HIGH);
        window(&mut records, after.fast_ion_energy_j, compared::FAST_ION_ENERGY_J, FAST_ION_LOW, FAST_ION_HIGH);
        window(&mut records, after.ion_temperature_kev, compared::ION_TEMPERATURE_KEV, ION_TEMPERATURE_LOW, ION_TEMPERATURE_HIGH);
        window(&mut records, after.t_s, compared::TIME_S, TIME_LOW, TIME_HIGH);
    }
    records
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::prelude::v1::*;
    use std::{assert, assert_eq, vec::Vec};

    use super::*;

    fn blank() -> LastStateFrame64 {
        LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [0xA5; 16] }
    }

    /// The record pulse 42982 near its peak, the load of the conformance kit.
    fn record() -> TwinLoad {
        TwinLoad {
            deuterium_beam_power_w: 10.5e6,
            tritium_beam_power_w: 9.5e6,
            deuterium_beam_energy_kev: 80.0,
            tritium_beam_energy_kev: 150.0,
            icrh_power_w: 2.4e6,
            ohmic_power_w: 0.5e6,
            line_density_per_m3: 6.4e19,
            plasma_current_a: 3.8e6,
            toroidal_field_t: 3.9,
            tritium_fraction: 0.451,
            electron_temperature_kev: 7.4,
            effective_charge: 2.4,
            hot_ion_regime: 0.0,
        }
    }

    fn start(card: &TokamakJetDte1Twin) -> TwinState {
        TwinState::try_start(card, 9.0e6, 0.5e6, 0.7e6, 0.4e6).unwrap()
    }

    fn run(card: &TokamakJetDte1Twin, load: TwinLoad, steps: usize) -> Result<TwinState, (usize, StepReject)> {
        let mut state = start(card);
        let mut frame = blank();
        for index in 0..steps {
            step(card, &mut state, &mut frame, load, GATE_DT_S).map_err(|reject| (index, reject))?;
        }
        Ok(state)
    }

    /// Bosch and Hale's Table VIII: 1.136e-16 cm³/s at 10 keV, 4.330e-16 at 20 keV.
    #[test]
    fn the_reactivity_is_bosch_and_hales() {
        assert!((reactivity_dt_m3_per_s(10.0) / 1.136e-22 - 1.0).abs() < 2.0e-3);
        assert!((reactivity_dt_m3_per_s(20.0) / 4.330e-22 - 1.0).abs() < 2.0e-3);
    }

    /// The cross-section averaged over a Maxwellian at 10 and 20 keV is the
    /// reactivity fit within 1.3 percent (the solvers' bosch_hale.rs holds the
    /// same fit to it from 0.2 to 100 keV).
    #[test]
    fn the_cross_section_over_a_maxwellian_is_the_reactivity() {
        let reduced = DEUTERON_MASS_KG * TRITON_MASS_KG / (DEUTERON_MASS_KG + TRITON_MASS_KG);
        let kev = 1.0e3 * ELEMENTARY_CHARGE_C;
        for t in [10.0_f64, 20.0] {
            let mut sum = 0.0;
            let n = 200_000;
            let top = 60.0 * t;
            let h = top / n as f64;
            for i in 0..n {
                let e = (i as f64 + 0.5) * h;
                sum += cross_section_dt_m2(e) * e * std::primitive::f64::exp(-e / t) * h;
            }
            let sv = (8.0 / (PI * reduced)).sqrt() * (kev * t).powf(-1.5) * sum * kev * kev;
            let ratio = sv / reactivity_dt_m3_per_s(t);
            assert!((ratio - 1.0).abs() < 0.013, "{t} keV: {ratio}");
        }
    }

    /// The beam-target rate needs fuel of the other species: tritons of a beam
    /// on a tritium plasma make none, deuterons on a deuterium plasma none.
    #[test]
    fn a_beam_fuses_only_with_the_other_species() {
        let card = TokamakJetDte1Twin::dte1();
        for (tritium, beam) in [(1.0, 1usize), (0.0, 0usize)] {
            let load = TwinLoad { tritium_fraction: tritium, ..record() };
            let p = plasma(&card, load);
            assert_eq!(p.beam_rate_per_j_s[beam], 0.0);
            assert!(p.beam_rate_per_j_s[1 - beam] > 0.0);
            let w = powers(&card, &p, load, &[9.0e6, 0.5e6, 0.7e6, 0.4e6, 0.0]);
            assert_eq!(w.thermal_w, 0.0);
        }
    }

    /// Five seconds of the record load at 1 ms: the ledger closes far under
    /// its limit, and the run is the same bits twice.
    #[test]
    fn a_long_run_closes_its_ledger_and_repeats_bit_for_bit() {
        let card = TokamakJetDte1Twin::dte1();
        let a = run(&card, record(), 5000).unwrap();
        let b = run(&card, record(), 5000).unwrap();
        assert!(a.residual_conservation_error < 1.0e-10, "{}", a.residual_conservation_error);
        assert_eq!(std::format!("{a:?}"), std::format!("{b:?}"));
        assert_eq!(a.thermal_energy_j.to_bits(), b.thermal_energy_j.to_bits());
        assert!(a.fusion_power_w > 0.0 && a.fusion_power_thermal_w > 0.0);
        assert!((a.fusion_power_w - a.fusion_power_thermal_w - a.fusion_power_deuterium_beam_w - a.fusion_power_tritium_beam_w).abs() <= 1.0e-9 * a.fusion_power_w);
    }

    /// The beam ions relax to their power times their thermalisation time.
    #[test]
    fn the_beam_stores_relax_to_power_times_their_time() {
        let card = TokamakJetDte1Twin::dte1();
        let load = record();
        let p = plasma(&card, load);
        let s = run(&card, load, 3000).unwrap();
        assert!((s.deuterium_beam_ion_energy_j / (load.deuterium_beam_power_w * p.beam_time_s[0]) - 1.0).abs() < 1.0e-6);
        assert!((s.tritium_beam_ion_energy_j / (load.tritium_beam_power_w * p.beam_time_s[1]) - 1.0).abs() < 1.0e-6);
    }

    /// The ion temperature is what the thermal energy leaves after the electrons.
    #[test]
    fn the_ions_hold_what_the_electrons_leave() {
        let card = TokamakJetDte1Twin::dte1();
        let load = record();
        let p = plasma(&card, load);
        let s = run(&card, load, 1).unwrap();
        let expected = (s.thermal_energy_j - p.electron_energy_j) / p.ion_energy_per_kev_j;
        assert_eq!(s.ion_temperature_kev.to_bits(), expected.to_bits());
        assert_eq!(s.electron_energy_j.to_bits(), p.electron_energy_j.to_bits());
        let below = TwinState::try_start(&card, p.electron_energy_j, 0.0, 0.0, 0.0).unwrap();
        let mut state = below;
        assert_eq!(step(&card, &mut state, &mut blank(), load, GATE_DT_S), Err(StepReject::EnergyOutsideCard));
        assert_eq!(state, below);
    }

    /// The hot-ion regime's multiplier and nothing else changes with the regime.
    #[test]
    fn the_regime_sets_the_confinement_multiplier() {
        let card = TokamakJetDte1Twin::dte1();
        let elmy = plasma(&card, record());
        let hot = plasma(&card, TwinLoad { hot_ion_regime: 1.0, ..record() });
        let ratio = hot.confinement_time_s / elmy.confinement_time_s;
        assert!((ratio / (CONFINEMENT_MULTIPLIER_HOT_ION / CONFINEMENT_MULTIPLIER_ELMY) - 1.0).abs() < 1.0e-14);
        assert_eq!(hot.beam_time_s, elmy.beam_time_s);
    }

    /// Every limit of the load by name, the state kept.
    #[test]
    fn each_limit_of_the_load_is_refused_by_name() {
        let card = TokamakJetDte1Twin::dte1();
        let cases: Vec<(TwinLoad, StepReject)> = std::vec![
            (TwinLoad { icrh_power_w: -1.0, ..record() }, StepReject::HeatingPowerNegative),
            (
                TwinLoad { deuterium_beam_power_w: 0.0, tritium_beam_power_w: 0.0, icrh_power_w: 0.0, ohmic_power_w: 0.0, ..record() },
                StepReject::NoHeatingPower
            ),
            (TwinLoad { deuterium_beam_energy_kev: 0.0, ..record() }, StepReject::BeamEnergyOutsideCrossSection),
            (
                TwinLoad { tritium_beam_energy_kev: f64::from_bits(TRITIUM_BEAM_ENERGY_MAX_KEV.to_bits() + 1), ..record() },
                StepReject::BeamEnergyOutsideCrossSection
            ),
            (TwinLoad { plasma_current_a: 0.0, ..record() }, StepReject::PlasmaCurrentNotPositive),
            (TwinLoad { toroidal_field_t: 0.0, ..record() }, StepReject::ToroidalFieldNotPositive),
            (TwinLoad { tritium_fraction: 1.5, ..record() }, StepReject::FuelMixOutsideCard),
            (TwinLoad { effective_charge: 6.0, ..record() }, StepReject::EffectiveChargeOutsideCard),
            (TwinLoad { hot_ion_regime: 0.5, ..record() }, StepReject::RegimeNotNamed),
            (TwinLoad { line_density_per_m3: 0.0, ..record() }, StepReject::DensityNotPositive),
            (TwinLoad { line_density_per_m3: 2.0e20, ..record() }, StepReject::DensityAboveGreenwald),
            (TwinLoad { plasma_current_a: 9.0e6, ..record() }, StepReject::SafetyFactorBelowLimit),
            (TwinLoad { electron_temperature_kev: 0.0, ..record() }, StepReject::ElectronTemperatureOutsideCard),
            (TwinLoad { electron_temperature_kev: 1.0e-9, ..record() }, StepReject::ElectronTemperatureOutsideCard),
        ];
        for (load, expected) in cases {
            let mut state = start(&card);
            let before = state;
            assert_eq!(step(&card, &mut state, &mut blank(), load, GATE_DT_S), Err(expected));
            assert_eq!(state, before);
        }
        let mut state = start(&card);
        let ceiling = card.state_ceiling_dt_s(record()).unwrap();
        assert!(ceiling > GATE_DT_S);
        assert_eq!(step(&card, &mut state, &mut blank(), record(), ceiling * 1.0001), Err(StepReject::DtAboveStateCeiling));
    }

    /// One second of the record load at 1 ms, ELMy and hot-ion, against the
    /// structure in plain Python floats (`twin/model_twin.py`, the reference
    /// of `twin/out/reference_run.txt`): every store within 1e-12 of it.
    #[test]
    fn one_second_of_the_record_load_is_the_python_structure() {
        let card = TokamakJetDte1Twin::dte1();
        let expected = [
            (0.0, [8740752.422457144, 463671.64455551637, 713309.0975899815, 537580.5968089973, 4612818.691672256], 7.474442275387155, 4535487.441828749, 1484751.614340726),
            (1.0, [17873494.89858578, 463671.64455551637, 713309.0975899815, 1345051.1722658528, 19455500.233411986], 27.007373275107213, 33642607.695364125, 30591871.867876105),
        ];
        for (regime, stores, ti0, fusion, thermal) in expected {
            let s = run(&card, TwinLoad { hot_ion_regime: regime, ..record() }, 1000).unwrap();
            let got = state_vector(&s);
            for index in 0..5 {
                assert!((got[index] / stores[index] - 1.0).abs() < 1.0e-12, "{regime} {index}: {} against {}", got[index], stores[index]);
            }
            assert!((s.ion_temperature_kev / ti0 - 1.0).abs() < 1.0e-12);
            assert!((s.fusion_power_w / fusion - 1.0).abs() < 1.0e-12);
            assert!((s.fusion_power_thermal_w / thermal - 1.0).abs() < 1.0e-12);
        }
    }

    /// The compared envelope is flagged and nothing else changes.
    #[test]
    fn a_load_outside_the_compared_envelope_is_flagged() {
        let card = TokamakJetDte1Twin::dte1();
        let inside = record();
        let mut state = start(&card);
        let mut frame = blank();
        step(&card, &mut state, &mut frame, inside, GATE_DT_S).unwrap();
        assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED, 0, "{state:?}");
        let outside = TwinLoad { tritium_fraction: 0.95, ..record() };
        let mut state = start(&card);
        step(&card, &mut state, &mut frame, outside, GATE_DT_S).unwrap();
        assert_eq!(frame.flags & FLAG_OUTSIDE_COMPARED, FLAG_OUTSIDE_COMPARED);
        assert_eq!(frame.proof, [0xA5; 16]);
    }
}
