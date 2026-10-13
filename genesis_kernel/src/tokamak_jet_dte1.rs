#![no_implicit_prelude]

//! The plasma of the JET tokamak in an ELMy H-mode pulse of its 1996 to 1997
//! campaigns (the Mark IIa divertor, the first deuterium-tritium experiment,
//! DTE1), as two stores of energy stepped through time under the heating
//! power.
//!
//! Measured at JET (Culham, EFDA-JET) and published in the International
//! Multi-Tokamak Confinement Profile Database, 2008 public release
//! (`https://tokamak-profiledb.ccfe.ac.uk/PR08/`): for each pulse the time
//! traces of heating power, density, current, field, thermal and total
//! plasma energy and D-T fusion power. The thermal energy, the fast-ion
//! energy and the fusion power of that database are the output of the TRANSP
//! analysis of each pulse from its measured profiles, not raw signals. The
//! database page says anyone may make analyses and publications from the
//! public version provided its papers are cited (Nucl. Fusion 40 (2000) 1955;
//! Nucl. Fusion 39 (1999) 2175). No measured value of a pulse is stored in
//! the card.
//!
//! The research record is `evidence/research/tokamak_jet_dte1/`. Four
//! constants are fitted, on eight training pulses (38285, 38287, 38415,
//! 42762, 42794, 42997, 43002, 43134), by `analysis/03_fit_and_forecast.py`,
//! and are the values of `analysis/out/03_constants.json`:
//! [`CONFINEMENT_MULTIPLIER`] and [`SLOWING_TIME_COEFF_S`] by least squares
//! on the thermal and the fast-ion energy of the eight; [`PROFILE_EXPONENT`]
//! and [`BEAM_FUSION_COEFF_MW_PER_MJ`] by least squares on the fusion power
//! of pulse 42762 and its thermal part. The geometry and the effective
//! charge are the means of the eight pulses' own values. Two pulses were
//! held back by the protocol and are not in any fit: 42982 and 42976.
//!
//! What the card is. The thermal energy is lost at the power the
//! IPB98(y,2) scaling of the energy confinement time gives (ITER Physics
//! Basis, Nucl. Fusion 39 (1999) 2175, equation 20), solved for the loss
//! power and multiplied once by the fitted constant. The heating power goes
//! to the fast ions and reaches the thermal plasma in a slowing-down time.
//! The D-T fusion power has a thermal part, the Maxwellian reactivity of
//! Bosch and Hale (Nucl. Fusion 32 (1992) 611, Table VII) summed over a
//! temperature profile, and a part from fast ions striking the thermal fuel,
//! proportional to the fast-ion energy and the density. One fifth of the
//! fusion power, the alpha particles, heats the fast-ion store.
//!
//! ```text
//! n19      = n_el / 1e19 m^-3            M = 2 + f_T          f_D = 1 − f_T
//! C        = H · 0.0562 · Ip^0.93 · Bt^0.15 · n19^0.41 · M^0.19 · R^1.97 · (a/R)^0.58 · κ^0.78     Ip in MA
//! P_loss   = 1e6 · (W_th / 1e6 / C)^(1 / 0.31)                               W
//! T        = W_th / (1.5 · n_el · (1 + f_i) · V · 1.602176634e-16)           keV
//! τ_f      = τ_f0 · sqrt(T) / n19                                            s
//! P_th     = E_DT · f_D · f_T · (f_main · n_el)² · V · ⟨σv⟩                  W
//! ⟨σv⟩     = (1/16) Σ_i σv_DT( (1 + α) · T · (1 − x_i)^α ),  x_i = (i + ½)/16
//! P_beam   = 1e6 · k_b · (W_f / 1e6) · n19 · 4 · f_D · f_T                   W
//! P_fus    = P_th + P_beam
//!
//! dW_f /dt  = P_aux + 0.2 · P_fus − W_f / τ_f
//! dW_th/dt  = W_f / τ_f + P_ohm − P_loss
//! dE_fus/dt = P_fus
//! ```
//!
//! `f_main = (6 − Z_eff) / 5` and `f_i = f_main + (Z_eff − 1) / 30` are the
//! main-ion and the total ion number per electron with carbon as the one
//! impurity. A profile cell colder than 0.2 keV, the floor of the reactivity
//! fit, adds no reactions: at 0.2 keV the fit gives 1.3e-32 m³/s, twelve
//! orders under its value at 20 keV. The map is fixed-step fourth-order
//! Runge–Kutta on (W_th, W_f, E_fus) with the load held through the step.
//!
//! The named limits, each enforced and none pulled back:
//!
//! ```text
//! Greenwald density    n_el ≤ 1e20 · Ip[MA] / (π a²)                         m^-3
//! Troyon beta          β_N = β[%] · a · Bt / Ip[MA] ≤ 2.8,  β = 2 μ0 · (2/3)(W_th + W_f)/V / Bt²
//! safety factor        q = 5 a² Bt / (R · Ip[MA]) · (1 + κ²)/2 · (1.17 − 0.65 ε)/(1 − ε²)² ≥ 2
//! reactivity fit       0.2 keV ≤ T  and  (1 + α) · T ≤ 100 keV
//! ```
//!
//! Greenwald: Nucl. Fusion 28 (1988) 2199. Troyon: Plasma Phys. Control.
//! Fusion 26 (1984) 209, the coefficient 2.8. The safety factor is the
//! edge value of the ITER Physics Design Guidelines (Uckan, 1989) without
//! its triangularity term; JET's own equilibrium value for these pulses is
//! about a fifth higher. The same reactivity fit and the same three limits
//! are written, with their sources, in
//! `evidence/research/tokamak_jet_dte1/shared_foundation.md` for the
//! proton-boron solver to use.
//!
//! The ledger. The two stores change only by what is supplied and what is
//! lost:
//!
//! ```text
//! defect   = (W_th + W_f) − anchor − [ Σ (P_aux + P_ohm) dt + 0.2 · E_fus − Σ Q ]
//! Q        = dt/2 · (P0 + P1) + dt²/12 · (P0' − P1')
//! P'       = P_loss · (dW_th/dt) / (0.31 · W_th)
//! residual = |defect| / max(W_th + W_f, anchor)
//! ```
//!
//! `Q` is the loss energy of one step by the corrected trapezoid rule on the
//! loss power and its rate at the two ends of the step. The Runge–Kutta step
//! does not use it, so an Euler step, a wrong stage weight or a dropped term
//! shows in the defect. The ledger shows the arithmetic and the integrator
//! are right. It cannot show the physics matches the tokamak.
//!
//! What is left out: the profiles themselves and any transport, the
//! L-mode and the ELM-free phases, each ELM, sawteeth, radiation as a
//! separate loss (the scaling's loss power includes it), the particle
//! balance (the density is the caller's load), separate ion and electron
//! temperatures, the species and the energy of the beams, shine-through and
//! orbit losses, the current drive, the plasma position and its control, the
//! coils, a disruption, and every other tokamak. V1's file
//! `physics/tokamak.rs` held a radial force balance, a vertical instability
//! and a coil quench with invented constants; none of that is here.
//!
//! The declared range is the envelope the comparison spanned inside the
//! card's regime: the nine pulses of the research record that are ELMy
//! H-modes, the eight fitted and the held-back 42982, run through the card at
//! 1 ms by the protocol's rule (`analysis/pulses.py`). Each bound is the
//! smallest or the largest value over the loads of those nine runs, or over
//! every state a step of them started from or landed on under that step's
//! load, rounded outward in the third figure
//! (`evidence/research/tokamak_jet_dte1/fix_0.7.0/envelope_nine.py`). The
//! held-back pulse 42976 is not in the envelope: a hot-ion pulse, it was
//! declared outside the card's regime before it was run, and it missed all
//! four marks. The states and loads only it reached (up to 25.1 MW, 4.16 MA,
//! 3.1 MW of ohmic power, densities below 1.65e19 and above 7.85e19 m^-3, a safety
//! factor down to 3.39, fast-ion energy to 3.5 MJ, temperature to 6.3 keV,
//! confinement times to 8.5 s, a normalised beta down to 0.13) are refused.
//! The loads: heating power 0.124 to 24.3 MW, ohmic power 0.173 to 2.44 MW,
//! line-averaged density 1.65e19 to 7.85e19 m^-3 and at most 0.834 of the
//! Greenwald density, plasma current 1.65 to 3.84 MA, toroidal field 1.80 to
//! 4.04 T, edge safety factor 3.66 to 4.14. The tritium fraction of the fuel
//! is 0 to 0.034, deuterium and the trace-tritium pulse 43134, or 0.451 to
//! 0.504, the held-back pulse 42982 and the fitted pulse 42762 at an even mix
//! (0.503, rounded up by 0.001 to hold the value the protocol's run
//! computes). The fusion power was compared there and not elsewhere; at 0.75
//! to 0.93 three training pulses showed it wrong, and from 0.034 to 0.451
//! nothing was compared. The state, at the start and after every step: thermal
//! energy 0.462 to 9.57 MJ, fast-ion energy 5.59 kJ to 1.62 MJ, temperature
//! 0.777 to 4.30 keV, normalised beta 0.224 to 2.21, confinement time
//! `W_th / P_loss` 0.150 to 5.72 s. The time since the start of the heated
//! window is 0 to 5.475 s, the longest compared window, and a start states
//! it. Outside any of these the step is refused by a name that ends in
//! `OutsideComparison` or `AboveComparison`, or by the name the bound had
//! before; the physical limits below are tested first and still stand.
//!
//! The pass marks and the result. Fixed before the card was run on either
//! held-back pulse (`analysis/prereg.json`, SHA-256 7badf2b1...ba94bb), over
//! the heated window of a pulse sampled every 25 ms: thermal energy error at
//! most 14.5 percent of its measured peak, root mean square; fast-ion energy
//! error at most 25 percent of its peak; fusion power error at most 25
//! percent of its peak; fusion energy released within 25 percent. Run through
//! the gateway on 2026-10-09 (`sealed_comparison.md`, `runs/`): pulse 42982
//! met the first three (7.9, 22.5 and 23.4 percent) and missed the fourth
//! (18.4 MJ against 24.8 MJ, a ratio of 0.743), so the comparison failed.
//! Pulse 42976, a hot-ion H-mode declared outside the card beforehand, missed
//! all four (19.2, 25.1 and 34.7 percent; 5.4 MJ against 10.3 MJ). The
//! card's fusion power is too low on both.
//!
//! The end of the card and rounding. The clock is a running sum,
//! `t += dt` in 64-bit arithmetic, started from the time the caller states,
//! and each addition rounds by at most half the spacing of 64-bit numbers
//! between 4 s and 8 s, 2^-51 s. A run that ends on 5.475 s in decimal can
//! therefore reach its last step with a clock a little late. The comparison
//! with the end carries
//! [`TokamakJetDte1::time_end_allowance_s`]: [`TIME_END_ALLOWANCE_PER_STEP_S`],
//! 2^-51 s, for each step a run of this step takes to reach the end, and
//! [`TIME_END_ALLOWANCE_EXTRA_STEPS`], two, more for the step itself read
//! from a decimal, which is within one part in 2^53 of itself, over the whole
//! span. It is never more than half the step, and the comparison is with the
//! 64-bit number nearest the end plus the allowance. The clock is stepped at
//! its own value and nothing is pulled back: a run that ends on the end in
//! decimal can read up to the allowance past it, and a step that would end
//! later is [`StepReject::TimeBeyondCard`].
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, the two energies, the fusion energy, the
//! fusion power, the temperature, the normalised beta, the loss power and
//! the conservation residual into [`LastStateFrame64`]. The 16-byte frame
//! proof is not touched.
//!
//! [`TokamakJetDte1::gate_dt_s`] is the step ceiling, [`GATE_DT_S`], 1 ms.
//! Two times set how fast the stores move: the slowing-down time, at least
//! 18.0 ms inside the declared range (0.777 keV at 7.85e19 m^-3), and the
//! time in which the loss power changes the thermal energy,
//! `0.31 · τ_E`, at least 46.5 ms (a confinement time of 0.150 s). The
//! ceiling is under an eighteenth of the first and under a fortieth of the second.
//! In 0.6.0 the ceiling rested on the slowing-down time alone, and states
//! with a confinement time near 37 ms, far outside the compared envelope,
//! were refused as FirstLawBreach although no energy was lost: the step's
//! truncation error passed the ledger's limit. The confinement time is now
//! held to the compared envelope, so [`StepReject::FirstLawBreach`] and
//! [`StepReject::SpontaneousGeneration`] name a ledger defect only.
//! [`TokamakJetDte1::floor_dt_s`] is [`FLOOR_DT_S`], 1e-6 s: at 5.475 s a
//! step of that size still moves the clock, and at 1 MW it moves a store of
//! 20 MJ by one joule, which 64-bit numbers resolve to nine figures. The
//! public step applies neither. The gateway refuses a step outside them.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::math::{exp, log};

/// Major radius, m. Mean of `RGEO` over the eight training pulses' 0D files, four figures.
pub const MAJOR_RADIUS_M: f64 = 2.899;
/// Minor radius, m. Mean of `AMIN` over the eight training pulses, four figures.
pub const MINOR_RADIUS_M: f64 = 0.9386;
/// Elongation. Mean of `KAPPA` over the eight training pulses, four figures.
pub const ELONGATION: f64 = 1.748;
/// Plasma volume, m³. Mean of `VOL` over the eight training pulses, four figures.
pub const PLASMA_VOLUME_M3: f64 = 81.56;
/// Effective charge. Mean of `ZEFF` over the eight training pulses, four figures.
pub const EFFECTIVE_CHARGE: f64 = 2.174;
/// Charge of the one impurity the dilution assumes, carbon.
pub const IMPURITY_CHARGE: f64 = 6.0;
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
/// One minus the scaling's exponent of the loss power, 1 − 0.69.
pub const IPB98_EXP_LOSS: f64 = 0.31;
/// Fitted multiplier `H` of the confinement time. `analysis/out/03_constants.json`, key `H`.
pub const CONFINEMENT_MULTIPLIER: f64 = 0.829118;
/// Fitted `τ_f0`: the slowing-down time at 1 keV and 1e19 m^-3, s. Key `TAU_F0`.
pub const SLOWING_TIME_COEFF_S: f64 = 0.160795;
/// Fitted exponent `α` of the temperature profile `(1 − x)^α`. Key `ALPHA`.
pub const PROFILE_EXPONENT: f64 = 0.308417;
/// Fitted `k_b`: MW of fusion power per MJ of fast-ion energy at 1e19 m^-3 and an even mix. Key `K_BT`.
pub const BEAM_FUSION_COEFF_MW_PER_MJ: f64 = 0.364521;
/// Cells of the temperature profile, equal in volume.
pub const PROFILE_CELLS: usize = 16;
/// Share of the D-T fusion energy carried by the alpha particle.
pub const ALPHA_ENERGY_SHARE: f64 = 0.2;
/// Energy of one D-T reaction, J: 17.589 MeV (Bosch and Hale 1992, Table I).
pub const DT_REACTION_ENERGY_J: f64 = 17.589e6 * ELEMENTARY_CHARGE_C;
/// Elementary charge, C (SI 2019, exact).
pub const ELEMENTARY_CHARGE_C: f64 = 1.602176634e-19;
/// Gamow constant of T(d,n)⁴He, sqrt(keV). Bosch and Hale 1992, Table VII.
pub const REACTIVITY_GAMOW_SQRT_KEV: f64 = 34.3827;
/// Reduced mass energy of T(d,n)⁴He, keV. Bosch and Hale 1992, Table VII.
pub const REACTIVITY_REDUCED_MASS_KEV: f64 = 1_124_656.0;
/// `C1` to `C7` of T(d,n)⁴He. Bosch and Hale 1992, Table VII.
pub const REACTIVITY_COEFF: [f64; 7] = [
    1.17302e-9,
    1.51361e-2,
    7.51886e-2,
    4.60643e-3,
    1.35000e-2,
    -1.06750e-4,
    1.36600e-5,
];
/// Lowest temperature of the reactivity fit, keV.
pub const REACTIVITY_TEMPERATURE_MIN_KEV: f64 = 0.2;
/// Highest temperature of the reactivity fit, keV.
pub const REACTIVITY_TEMPERATURE_MAX_KEV: f64 = 100.0;
/// Greenwald density per MA of current over `π a²`, m^-3.
pub const GREENWALD_COEFF_PER_M3: f64 = 1.0e20;
/// Troyon limit of the normalised beta, percent m T / MA.
pub const TROYON_BETA_NORMALISED_MAX: f64 = 2.8;
/// Lowest edge safety factor the card accepts.
pub const SAFETY_FACTOR_MIN: f64 = 2.0;
/// Fraction of the Greenwald density above which [`FLAG_NEAR_DENSITY_LIMIT`] is set.
pub const NEAR_DENSITY_LIMIT_FRACTION: f64 = 0.85;
/// Smallest heating power the card accepts, W: the smallest of the nine compared runs of the card's regime, 0.1248 MW, rounded down.
pub const HEATING_POWER_MIN_W: f64 = 0.124e6;
/// Largest heating power (neutral beams and ion cyclotron waves) the card accepts, W: the largest compared, 24.22 MW, rounded up.
pub const HEATING_POWER_MAX_W: f64 = 24.3e6;
/// Smallest ohmic power the card accepts, W: the smallest compared, 0.1738 MW, rounded down.
pub const OHMIC_POWER_MIN_W: f64 = 0.173e6;
/// Largest ohmic power the card accepts, W: the largest compared, 2.436 MW, rounded up.
pub const OHMIC_POWER_MAX_W: f64 = 2.44e6;
/// Smallest line-averaged electron density the card accepts, m^-3: the smallest compared, 1.658e19, rounded down.
pub const LINE_DENSITY_MIN_PER_M3: f64 = 1.65e19;
/// Largest line-averaged electron density the card accepts, m^-3: the largest compared, 7.849e19, rounded up.
pub const LINE_DENSITY_MAX_PER_M3: f64 = 7.85e19;
/// Largest fraction of the Greenwald density the card accepts: the largest compared, 0.8331, rounded up.
pub const GREENWALD_FRACTION_MAX: f64 = 0.834;
/// Smallest plasma current the card accepts, A: the smallest compared, 1.652 MA, rounded down.
pub const PLASMA_CURRENT_MIN_A: f64 = 1.65e6;
/// Largest plasma current the card accepts, A: the largest compared, 3.835 MA, rounded up.
pub const PLASMA_CURRENT_MAX_A: f64 = 3.84e6;
/// Smallest toroidal field the card accepts, T: the smallest compared, 1.807 T, rounded down.
pub const TOROIDAL_FIELD_MIN_T: f64 = 1.80;
/// Largest toroidal field the card accepts, T: the largest compared, 4.039 T, rounded up.
pub const TOROIDAL_FIELD_MAX_T: f64 = 4.04;
/// Smallest edge safety factor the card accepts above the limit of 2: the smallest compared, 3.663, rounded down.
pub const SAFETY_FACTOR_COMPARED_MIN: f64 = 3.66;
/// Largest edge safety factor the card accepts: the largest compared, 4.136, rounded up.
pub const SAFETY_FACTOR_COMPARED_MAX: f64 = 4.14;
/// Largest tritium fraction of the lower span: 43134, the one compared pulse with tritium below an even mix.
pub const TRITIUM_FRACTION_TRACE_MAX: f64 = 0.034;
/// Smallest tritium fraction of the upper span: the held-back pulse 42982.
pub const TRITIUM_FRACTION_EVEN_MIN: f64 = 0.451;
/// Largest tritium fraction of the upper span: the fitted pulse 42762 at 0.503, rounded up by 0.001 to hold the value the protocol's run computes.
pub const TRITIUM_FRACTION_EVEN_MAX: f64 = 0.504;
/// Smallest thermal energy the card accepts, J: the smallest compared, 0.4624 MJ, rounded down.
pub const THERMAL_ENERGY_MIN_J: f64 = 0.462e6;
/// Largest thermal energy the card accepts, J: the largest compared, 9.566 MJ, rounded up.
pub const THERMAL_ENERGY_MAX_J: f64 = 9.57e6;
/// Smallest fast-ion energy the card accepts, J: the smallest compared, 5.592 kJ, rounded down.
pub const FAST_ION_ENERGY_MIN_J: f64 = 5.59e3;
/// Largest fast-ion energy the card accepts, J: the largest compared, 1.618 MJ, rounded up.
pub const FAST_ION_ENERGY_MAX_J: f64 = 1.62e6;
/// Smallest plasma temperature the card accepts, keV: the smallest compared, 0.7776 keV at the start of a step of 43134, rounded down.
pub const TEMPERATURE_COMPARED_MIN_KEV: f64 = 0.777;
/// Largest plasma temperature the card accepts, keV: the largest compared, 4.294 keV, rounded up.
pub const TEMPERATURE_COMPARED_MAX_KEV: f64 = 4.30;
/// Smallest normalised beta the card accepts: the smallest compared, 0.2241, rounded down.
pub const BETA_NORMALISED_COMPARED_MIN: f64 = 0.224;
/// Largest normalised beta the card accepts: the largest compared, 2.202, rounded up.
pub const BETA_NORMALISED_COMPARED_MAX: f64 = 2.21;
/// Smallest confinement time `W_th / P_loss` the card accepts, s: the smallest compared, 0.1504 s, rounded down.
pub const CONFINEMENT_TIME_MIN_S: f64 = 0.150;
/// Largest confinement time `W_th / P_loss` the card accepts, s: the largest compared, 5.716 s, rounded up.
pub const CONFINEMENT_TIME_MAX_S: f64 = 5.72;
/// Time since the start of the heated window beyond which the card refuses, s: the longest compared window, pulse 42982.
pub const TIME_MAX_S: f64 = 5.475;
/// Half the spacing of 64-bit numbers between 4 s and 8 s, 2^-51 s: the
/// most one addition to the clock can round by. The comparison with
/// [`TIME_MAX_S`] allows this much for each step a run takes to reach it.
pub const TIME_END_ALLOWANCE_PER_STEP_S: f64 = 4.440_892_098_500_626e-16;
/// Steps added to the count in [`TokamakJetDte1::time_end_allowance_s`], for
/// the step read from a decimal.
pub const TIME_END_ALLOWANCE_EXTRA_STEPS: f64 = 2.0;
/// `|defect| / max(W_th + W_f, anchor)` must stay at or under this. Set by
/// measurement (`analysis/out/06_ledger_on_training.log`): over the eight
/// training pulses run at the ceiling the largest value found is 5.1e-10
/// (pulse 42997, 5425 steps), and the limit is 39 times it. An Euler step in
/// place of the first Runge–Kutta step of those runs gives 4.9e-7 or more.
/// Inside the declared range, over a grid of 2,784 starts and loads, one
/// Runge–Kutta step at the ceiling gives at most 4.9e-10, and runs held at the
/// corners of the loads to the end of the card at most 3.9e-10.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.0e-8;
/// Largest step the card stands behind, s.
pub const GATE_DT_S: f64 = 1.0e-3;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-6;
/// Vacuum permeability, H/m (CODATA 2018).
pub const VACUUM_PERMEABILITY_H_PER_M: f64 = 1.25663706212e-6;
/// π to 64-bit precision.
pub const PI: f64 = 3.141592653589793;

/// The line-averaged density is above 0.85 of the Greenwald density on this frame.
pub const FLAG_NEAR_DENSITY_LIMIT: u64 = 1 << 0;

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
    /// The heating power is below 0.124 MW or above 24.3 MW, the span of the nine compared pulses of the card's regime.
    HeatingPowerOutsideCard,
    /// The ohmic power is below 0.173 MW or above 2.44 MW, the span of the nine compared pulses of the card's regime.
    OhmicPowerOutsideCard,
    /// The plasma current is below 1.65 MA or above 3.84 MA, the span of the nine compared pulses of the card's regime.
    PlasmaCurrentOutsideCard,
    /// The toroidal field is below 1.80 T or above 4.04 T, the span of the nine compared pulses of the card's regime.
    ToroidalFieldOutsideCard,
    /// The tritium fraction of the fuel is below 0.
    FuelMixOutsideCard,
    /// The tritium fraction of the fuel is above 0.034 and below 0.451, where the fusion power was never compared, or above 0.504, where three training pulses at 0.75 to 0.93 showed it wrong.
    FuelMixOutsideComparison,
    /// The line-averaged density is below 1.65e19 per cubic metre, the smallest compared.
    DensityBelowCard,
    /// The line-averaged density is above 7.85e19 per cubic metre, the largest compared.
    DensityAboveComparison,
    /// The line-averaged density is above the Greenwald density of the plasma current.
    DensityAboveGreenwald,
    /// The line-averaged density is above 0.834 of the Greenwald density of the plasma current, the largest fraction compared.
    GreenwaldFractionAboveComparison,
    /// The edge safety factor of the current and the field is below 2.
    SafetyFactorBelowLimit,
    /// The edge safety factor of the current and the field is below 3.66 or above 4.14, the span of the nine compared pulses of the card's regime.
    SafetyFactorOutsideComparison,
    /// The thermal energy is zero or negative, or the fast-ion energy is negative.
    EnergyOutsideCard,
    /// The plasma temperature is below 0.2 keV, the floor of the reactivity fit.
    TemperatureBelowReactivityFit,
    /// The temperature at the centre of the profile is above 100 keV, the ceiling of the reactivity fit.
    TemperatureAboveReactivityFit,
    /// The normalised beta of the two stores is above the Troyon limit of 2.8, at the start of the step or where the trial step lands.
    BetaAboveTroyon,
    /// The thermal energy is outside 0.462 to 9.57 MJ, or the fast-ion energy outside 5.59 kJ to 1.62 MJ, the spans of the nine compared runs of the card's regime, at the start of the step or where the trial step lands.
    EnergyOutsideComparison,
    /// The plasma temperature is below 0.777 keV or above 4.30 keV, the span of the nine compared runs of the card's regime, at the start of the step or where the trial step lands.
    TemperatureOutsideComparison,
    /// The normalised beta of the two stores is below 0.224 or above 2.21, the span of the nine compared runs of the card's regime, at the start of the step or where the trial step lands.
    BetaOutsideComparison,
    /// The confinement time, the thermal energy over the loss power, is below 0.150 s or above 5.72 s, the span of the nine compared runs of the card's regime, at the start of the step or where the trial step lands.
    ConfinementTimeOutsideComparison,
    /// The clock is negative, or the step would end after 5.475 s, the longest compared window, by more than the rounding allowance of the running sum of the clock: 4.44e-16 s for each step a run of this step takes to reach 5.475 s, and two more.
    TimeBeyondCard,
    /// The trial step created energy beyond what the heating and the fusion supply: a defect of the arithmetic or the integrator.
    SpontaneousGeneration,
    /// The trial step destroyed energy the heating and the fusion supply: a defect of the arithmetic or the integrator.
    FirstLawBreach,
}

/// What is applied to the plasma, held through the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TokamakLoad {
    /// Neutral-beam power less shine-through plus coupled ion-cyclotron power, `P_aux`, W.
    pub heating_power_w: f64,
    /// Ohmic power, `P_ohm`, W.
    pub ohmic_power_w: f64,
    /// Line-averaged electron density, `n_el`, m^-3.
    pub line_density_per_m3: f64,
    /// Plasma current, `Ip`, A.
    pub plasma_current_a: f64,
    /// Toroidal field at the major radius, `Bt`, T.
    pub toroidal_field_t: f64,
    /// Tritium atoms per fuel atom, `f_T`: 0 is deuterium, 0.5 the even mix.
    pub tritium_fraction: f64,
}

/// The locked tokamak. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TokamakJetDte1 {
    /// Recorded copy of [`MAJOR_RADIUS_M`].
    pub major_radius_m: f64,
    /// Recorded copy of [`MINOR_RADIUS_M`].
    pub minor_radius_m: f64,
    /// Recorded copy of [`ELONGATION`].
    pub elongation: f64,
    /// Recorded copy of [`PLASMA_VOLUME_M3`].
    pub plasma_volume_m3: f64,
    /// Recorded copy of [`EFFECTIVE_CHARGE`].
    pub effective_charge: f64,
    /// Recorded copy of [`CONFINEMENT_MULTIPLIER`].
    pub confinement_multiplier: f64,
    /// Recorded copy of [`SLOWING_TIME_COEFF_S`].
    pub slowing_time_coeff_s: f64,
    /// Recorded copy of [`PROFILE_EXPONENT`].
    pub profile_exponent: f64,
    /// Recorded copy of [`BEAM_FUSION_COEFF_MW_PER_MJ`].
    pub beam_fusion_coeff_mw_per_mj: f64,
    /// Recorded copy of [`REACTIVITY_COEFF`].
    pub reactivity_coeff: [f64; 7],
    /// Recorded copy of [`TROYON_BETA_NORMALISED_MAX`].
    pub troyon_beta_normalised_max: f64,
    /// Recorded copy of [`SAFETY_FACTOR_MIN`].
    pub safety_factor_min: f64,
    /// Recorded copy of [`TIME_MAX_S`].
    pub time_max_s: f64,
    /// Recorded copy of [`TIME_END_ALLOWANCE_PER_STEP_S`].
    pub time_end_allowance_per_step_s: f64,
    /// Recorded copy of [`TIME_END_ALLOWANCE_EXTRA_STEPS`].
    pub time_end_allowance_extra_steps: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`FLOOR_DT_S`].
    pub floor_dt_s: f64,
}

impl TokamakJetDte1 {
    /// Card: JET in an ELMy H-mode pulse of 1996 to 1997. The constructor takes no arguments.
    #[must_use]
    pub fn elmy_h_mode() -> Self {
        let tokamak = Self {
            major_radius_m: MAJOR_RADIUS_M,
            minor_radius_m: MINOR_RADIUS_M,
            elongation: ELONGATION,
            plasma_volume_m3: PLASMA_VOLUME_M3,
            effective_charge: EFFECTIVE_CHARGE,
            confinement_multiplier: CONFINEMENT_MULTIPLIER,
            slowing_time_coeff_s: SLOWING_TIME_COEFF_S,
            profile_exponent: PROFILE_EXPONENT,
            beam_fusion_coeff_mw_per_mj: BEAM_FUSION_COEFF_MW_PER_MJ,
            reactivity_coeff: REACTIVITY_COEFF,
            troyon_beta_normalised_max: TROYON_BETA_NORMALISED_MAX,
            safety_factor_min: SAFETY_FACTOR_MIN,
            time_max_s: TIME_MAX_S,
            time_end_allowance_per_step_s: TIME_END_ALLOWANCE_PER_STEP_S,
            time_end_allowance_extra_steps: TIME_END_ALLOWANCE_EXTRA_STEPS,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_dt_s: GATE_DT_S,
            floor_dt_s: FLOOR_DT_S,
        };
        if tokamak.is_valid() {
            tokamak
        } else {
            ::core::panic!("JET tokamak failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        let mut index = 0;
        while index < 7 {
            if self.reactivity_coeff[index].to_bits() != REACTIVITY_COEFF[index].to_bits() {
                return false;
            }
            index += 1;
        }
        self.major_radius_m.to_bits() == MAJOR_RADIUS_M.to_bits()
            && self.minor_radius_m.to_bits() == MINOR_RADIUS_M.to_bits()
            && self.elongation.to_bits() == ELONGATION.to_bits()
            && self.plasma_volume_m3.to_bits() == PLASMA_VOLUME_M3.to_bits()
            && self.effective_charge.to_bits() == EFFECTIVE_CHARGE.to_bits()
            && self.confinement_multiplier.to_bits() == CONFINEMENT_MULTIPLIER.to_bits()
            && self.slowing_time_coeff_s.to_bits() == SLOWING_TIME_COEFF_S.to_bits()
            && self.profile_exponent.to_bits() == PROFILE_EXPONENT.to_bits()
            && self.beam_fusion_coeff_mw_per_mj.to_bits() == BEAM_FUSION_COEFF_MW_PER_MJ.to_bits()
            && self.troyon_beta_normalised_max.to_bits() == TROYON_BETA_NORMALISED_MAX.to_bits()
            && self.safety_factor_min.to_bits() == SAFETY_FACTOR_MIN.to_bits()
            && self.time_max_s.to_bits() == TIME_MAX_S.to_bits()
            && self.time_end_allowance_per_step_s.to_bits()
                == TIME_END_ALLOWANCE_PER_STEP_S.to_bits()
            && self.time_end_allowance_extra_steps.to_bits()
                == TIME_END_ALLOWANCE_EXTRA_STEPS.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
            && self.major_radius_m > self.minor_radius_m
            && self.minor_radius_m > 0.0
            && self.elongation > 0.0
            && self.plasma_volume_m3 > 0.0
            && self.effective_charge >= 1.0
            && self.effective_charge < IMPURITY_CHARGE
            && self.confinement_multiplier > 0.0
            && self.slowing_time_coeff_s > 0.0
            && self.profile_exponent >= 0.0
            && self.beam_fusion_coeff_mw_per_mj >= 0.0
    }

    /// How far past [`TIME_MAX_S`] the running sum of the clock may read at
    /// the end of a step of `dt_s`, seconds:
    /// [`TIME_END_ALLOWANCE_PER_STEP_S`] for each step a run of this step
    /// takes to reach the end, and [`TIME_END_ALLOWANCE_EXTRA_STEPS`] more;
    /// never more than half the step. Zero for a step that is not positive.
    #[must_use]
    pub fn time_end_allowance_s(&self, dt_s: f64) -> f64 {
        if !(dt_s > 0.0) {
            return 0.0;
        }
        let steps = self.time_max_s / dt_s + self.time_end_allowance_extra_steps;
        let allowance_s = steps * self.time_end_allowance_per_step_s;
        if allowance_s < 0.5 * dt_s { allowance_s } else { 0.5 * dt_s }
    }

    /// Largest step the card stands behind: [`GATE_DT_S`]. Not applied by [`step`].
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

    /// Main fuel ions per electron, `(6 − Z_eff) / 5`.
    #[must_use]
    pub fn main_ion_fraction(&self) -> f64 {
        (IMPURITY_CHARGE - self.effective_charge) / (IMPURITY_CHARGE - 1.0)
    }

    /// All ions per electron, `f_main + (Z_eff − 1) / 30`.
    #[must_use]
    pub fn ion_fraction(&self) -> f64 {
        self.main_ion_fraction()
            + (self.effective_charge - 1.0) / (IMPURITY_CHARGE * (IMPURITY_CHARGE - 1.0))
    }

    /// Greenwald density of a plasma current, m^-3.
    #[must_use]
    pub fn greenwald_density_per_m3(&self, plasma_current_a: f64) -> f64 {
        GREENWALD_COEFF_PER_M3 * (plasma_current_a / 1.0e6)
            / (PI * self.minor_radius_m * self.minor_radius_m)
    }

    /// Edge safety factor of a current and a field.
    #[must_use]
    pub fn safety_factor(&self, plasma_current_a: f64, toroidal_field_t: f64) -> f64 {
        let aspect = self.minor_radius_m / self.major_radius_m;
        let shape = (1.17 - 0.65 * aspect) / ((1.0 - aspect * aspect) * (1.0 - aspect * aspect))
            * (1.0 + self.elongation * self.elongation)
            / 2.0;
        5.0 * self.minor_radius_m * self.minor_radius_m * toroidal_field_t
            / (self.major_radius_m * (plasma_current_a / 1.0e6))
            * shape
    }

    /// Normalised beta of the two stores under a current and a field, percent m T / MA.
    #[must_use]
    pub fn beta_normalised(&self, stored_energy_j: f64, plasma_current_a: f64, toroidal_field_t: f64) -> f64 {
        let pressure_pa = (2.0 / 3.0) * stored_energy_j / self.plasma_volume_m3;
        let beta_percent = 100.0 * 2.0 * VACUUM_PERMEABILITY_H_PER_M * pressure_pa
            / (toroidal_field_t * toroidal_field_t);
        beta_percent * self.minor_radius_m * toroidal_field_t / (plasma_current_a / 1.0e6)
    }

    /// The energy of the two stores together at the Troyon limit of a current and a field, J.
    #[must_use]
    pub fn troyon_stored_energy_j(&self, plasma_current_a: f64, toroidal_field_t: f64) -> f64 {
        self.troyon_beta_normalised_max * (plasma_current_a / 1.0e6) * toroidal_field_t
            / (100.0 * self.minor_radius_m)
            / (2.0 * VACUUM_PERMEABILITY_H_PER_M)
            * 1.5
            * self.plasma_volume_m3
    }

    /// The largest energy the two stores can hold together anywhere on the card:
    /// the Troyon limit at the largest current and the largest field, J.
    #[must_use]
    pub fn largest_stored_energy_j(&self) -> f64 {
        self.troyon_stored_energy_j(PLASMA_CURRENT_MAX_A, TOROIDAL_FIELD_MAX_T)
    }

    /// Plasma temperature of a thermal energy at a density, keV.
    #[must_use]
    pub fn temperature_kev(&self, thermal_energy_j: f64, line_density_per_m3: f64) -> f64 {
        thermal_energy_j
            / (1.5
                * line_density_per_m3
                * (1.0 + self.ion_fraction())
                * self.plasma_volume_m3
                * (1.0e3 * ELEMENTARY_CHARGE_C))
    }

    /// Energy confinement time `W_th / P_loss` of a thermal energy under a load, s.
    /// The same number a landed step writes as `confinement_time_s`.
    #[must_use]
    pub fn confinement_time_s(&self, thermal_energy_j: f64, load: TokamakLoad) -> f64 {
        thermal_energy_j / derived(self, load, &[thermal_energy_j, 0.0, 0.0]).loss_power_w
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

/// Mean of the reactivity over the profile `(1 + α) T (1 − x)^α`, m³/s.
#[must_use]
pub fn profile_reactivity_m3_per_s(temperature_kev: f64, profile_exponent: f64) -> f64 {
    let centre = (1.0 + profile_exponent) * temperature_kev;
    let mut sum = 0.0;
    let mut index = 0;
    while index < PROFILE_CELLS {
        let x = (index as f64 + 0.5) / PROFILE_CELLS as f64;
        let cell = centre * exp(profile_exponent * log(1.0 - x));
        if cell >= REACTIVITY_TEMPERATURE_MIN_KEV {
            sum += reactivity_dt_m3_per_s(cell);
        }
        index += 1;
    }
    sum / PROFILE_CELLS as f64
}

/// Integrated plasma. The derived and ledger fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TokamakState {
    /// Time since the start of the heated window.
    pub t_s: f64,
    /// Thermal energy of the plasma, `W_th`.
    pub thermal_energy_j: f64,
    /// Energy of the fast ions, `W_f`.
    pub fast_ion_energy_j: f64,
    /// D-T fusion energy released since the start, `E_fus`.
    pub fusion_energy_j: f64,
    /// Plasma temperature `T` at this state under the load of the last step. Zero at the start.
    pub temperature_kev: f64,
    /// Loss power `P_loss` at this state under the load of the last step. Zero at the start.
    pub loss_power_w: f64,
    /// Energy confinement time `W_th / P_loss`. Zero at the start.
    pub confinement_time_s: f64,
    /// Slowing-down time `τ_f`. Zero at the start.
    pub slowing_time_s: f64,
    /// D-T fusion power `P_fus`. Zero at the start.
    pub fusion_power_w: f64,
    /// The thermal part of the fusion power, `P_th`. Zero at the start.
    pub fusion_power_thermal_w: f64,
    /// Normalised beta of the two stores, percent m T / MA. Zero at the start.
    pub beta_normalised: f64,
    /// Line-averaged density over the Greenwald density. Zero at the start.
    pub greenwald_fraction: f64,
    /// Edge safety factor of the load of the last step. Zero at the start.
    pub safety_factor: f64,
    /// `W_th + W_f` at the state the ledger started from.
    pub energy_anchor_j: f64,
    /// `Σ (P_aux + P_ohm) dt` since the anchor.
    pub energy_supplied_j: f64,
    /// `Σ Q`, the loss energy since the anchor by the corrected trapezoid rule.
    pub energy_lost_j: f64,
    /// Signed defect of the ledger. Positive is created energy.
    pub energy_defect_j: f64,
    /// `|energy_defect_j| / max(W_th + W_f, anchor)`.
    pub residual_conservation_error: f64,
    /// The density is above 0.85 of the Greenwald density.
    pub near_density_limit: bool,
}

impl TokamakState {
    /// The plasma with a thermal and a fast-ion energy at time 0: [`TokamakState::try_continue`] at zero.
    /// `None` unless both are finite, the thermal energy is above zero and the fast-ion energy is not negative.
    #[must_use]
    pub fn try_start(tokamak: &TokamakJetDte1, thermal_energy_j: f64, fast_ion_energy_j: f64) -> Option<Self> {
        Self::try_continue(tokamak, 0.0, thermal_energy_j, fast_ion_energy_j)
    }

    /// The plasma with a thermal and a fast-ion energy at `elapsed_time_s`
    /// since the start of the heated window, nothing released, ledger
    /// anchored. A run continued from a returned state states that state's
    /// `t_s` here, so the end of the card is counted from the window's start.
    /// `None` unless the time and both energies are finite, the time is not
    /// negative, the thermal energy is above zero and the fast-ion energy is not negative.
    #[must_use]
    pub fn try_continue(
        tokamak: &TokamakJetDte1,
        elapsed_time_s: f64,
        thermal_energy_j: f64,
        fast_ion_energy_j: f64,
    ) -> Option<Self> {
        if !tokamak.is_valid()
            || !elapsed_time_s.is_finite()
            || !(elapsed_time_s >= 0.0)
            || !thermal_energy_j.is_finite()
            || !fast_ion_energy_j.is_finite()
            || !(thermal_energy_j > 0.0)
            || !(fast_ion_energy_j >= 0.0)
        {
            return None;
        }
        let anchor = thermal_energy_j + fast_ion_energy_j;
        if !anchor.is_finite() {
            return None;
        }
        Some(Self {
            t_s: elapsed_time_s,
            thermal_energy_j,
            fast_ion_energy_j,
            fusion_energy_j: 0.0,
            temperature_kev: 0.0,
            loss_power_w: 0.0,
            confinement_time_s: 0.0,
            slowing_time_s: 0.0,
            fusion_power_w: 0.0,
            fusion_power_thermal_w: 0.0,
            beta_normalised: 0.0,
            greenwald_fraction: 0.0,
            safety_factor: 0.0,
            energy_anchor_j: anchor,
            energy_supplied_j: 0.0,
            energy_lost_j: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            near_density_limit: false,
        })
    }
}

/// The powers and times of one state under one load.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Derived {
    loss_power_w: f64,
    temperature_kev: f64,
    slowing_time_s: f64,
    fusion_power_thermal_w: f64,
    fusion_power_w: f64,
}

/// Advance one Runge–Kutta step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    tokamak: &TokamakJetDte1,
    state: &mut TokamakState,
    frame: &mut LastStateFrame64,
    load: TokamakLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(tokamak, state, load, dt_s, Envelope::Compared)?;
    let advanced = rk4(tokamak, load, &state_vector(state), dt_s);
    commit(tokamak, state, frame, load, dt_s, advanced, Envelope::Compared)
}

/// Whether a step is held to the envelope of the comparison. The public
/// step always is; the tests of the integrator against the forecast script,
/// whose reference runs start outside it, are not.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Envelope {
    Compared,
    #[cfg(test)]
    Structure,
}

/// The load held to the card: each field to its span, the density to the
/// Greenwald density and the current and the field to the safety factor,
/// each limit of physics before the envelope of the comparison. The step
/// tests this first; a caller may test a load with it before a run.
pub fn load_in_card(tokamak: &TokamakJetDte1, load: TokamakLoad) -> Result<(), StepReject> {
    if load.heating_power_w < HEATING_POWER_MIN_W || load.heating_power_w > HEATING_POWER_MAX_W {
        return Err(StepReject::HeatingPowerOutsideCard);
    }
    if load.ohmic_power_w < OHMIC_POWER_MIN_W || load.ohmic_power_w > OHMIC_POWER_MAX_W {
        return Err(StepReject::OhmicPowerOutsideCard);
    }
    if load.plasma_current_a < PLASMA_CURRENT_MIN_A || load.plasma_current_a > PLASMA_CURRENT_MAX_A {
        return Err(StepReject::PlasmaCurrentOutsideCard);
    }
    if load.toroidal_field_t < TOROIDAL_FIELD_MIN_T || load.toroidal_field_t > TOROIDAL_FIELD_MAX_T {
        return Err(StepReject::ToroidalFieldOutsideCard);
    }
    if load.tritium_fraction < 0.0 {
        return Err(StepReject::FuelMixOutsideCard);
    }
    if load.tritium_fraction > TRITIUM_FRACTION_EVEN_MAX
        || (load.tritium_fraction > TRITIUM_FRACTION_TRACE_MAX
            && load.tritium_fraction < TRITIUM_FRACTION_EVEN_MIN)
    {
        return Err(StepReject::FuelMixOutsideComparison);
    }
    if load.line_density_per_m3 < LINE_DENSITY_MIN_PER_M3 {
        return Err(StepReject::DensityBelowCard);
    }
    if load.line_density_per_m3 > LINE_DENSITY_MAX_PER_M3 {
        return Err(StepReject::DensityAboveComparison);
    }
    let greenwald = tokamak.greenwald_density_per_m3(load.plasma_current_a);
    if load.line_density_per_m3 > greenwald {
        return Err(StepReject::DensityAboveGreenwald);
    }
    if load.line_density_per_m3 > GREENWALD_FRACTION_MAX * greenwald {
        return Err(StepReject::GreenwaldFractionAboveComparison);
    }
    let safety_factor = tokamak.safety_factor(load.plasma_current_a, load.toroidal_field_t);
    if safety_factor < tokamak.safety_factor_min {
        return Err(StepReject::SafetyFactorBelowLimit);
    }
    if safety_factor < SAFETY_FACTOR_COMPARED_MIN || safety_factor > SAFETY_FACTOR_COMPARED_MAX {
        return Err(StepReject::SafetyFactorOutsideComparison);
    }
    Ok(())
}

/// A thermal and a fast-ion energy held to the card under a load: the
/// energies positive, the temperature in the reactivity fit, the two stores
/// under the Troyon limit, and then the envelope of the comparison, the two
/// energies, the temperature, the normalised beta and the confinement time.
/// The step tests this at its start and where its trial step lands; a caller
/// may test a start with it.
pub fn state_in_card(
    tokamak: &TokamakJetDte1,
    thermal_energy_j: f64,
    fast_ion_energy_j: f64,
    load: TokamakLoad,
) -> Result<(), StepReject> {
    state_in(tokamak, thermal_energy_j, fast_ion_energy_j, load, Envelope::Compared)
}

fn state_in(
    tokamak: &TokamakJetDte1,
    thermal_energy_j: f64,
    fast_ion_energy_j: f64,
    load: TokamakLoad,
    envelope: Envelope,
) -> Result<(), StepReject> {
    if !(thermal_energy_j > 0.0) || !(fast_ion_energy_j >= 0.0) {
        return Err(StepReject::EnergyOutsideCard);
    }
    temperature_in_fit(tokamak, thermal_energy_j, load)?;
    if tokamak.beta_normalised(
        thermal_energy_j + fast_ion_energy_j,
        load.plasma_current_a,
        load.toroidal_field_t,
    ) > tokamak.troyon_beta_normalised_max
    {
        return Err(StepReject::BetaAboveTroyon);
    }
    if envelope != Envelope::Compared {
        return Ok(());
    }
    if thermal_energy_j < THERMAL_ENERGY_MIN_J
        || thermal_energy_j > THERMAL_ENERGY_MAX_J
        || fast_ion_energy_j < FAST_ION_ENERGY_MIN_J
        || fast_ion_energy_j > FAST_ION_ENERGY_MAX_J
    {
        return Err(StepReject::EnergyOutsideComparison);
    }
    let temperature = tokamak.temperature_kev(thermal_energy_j, load.line_density_per_m3);
    if temperature < TEMPERATURE_COMPARED_MIN_KEV || temperature > TEMPERATURE_COMPARED_MAX_KEV {
        return Err(StepReject::TemperatureOutsideComparison);
    }
    let beta = tokamak.beta_normalised(
        thermal_energy_j + fast_ion_energy_j,
        load.plasma_current_a,
        load.toroidal_field_t,
    );
    if beta < BETA_NORMALISED_COMPARED_MIN || beta > BETA_NORMALISED_COMPARED_MAX {
        return Err(StepReject::BetaOutsideComparison);
    }
    let confinement_time_s = tokamak.confinement_time_s(thermal_energy_j, load);
    if !(confinement_time_s >= CONFINEMENT_TIME_MIN_S) || confinement_time_s > CONFINEMENT_TIME_MAX_S {
        return Err(StepReject::ConfinementTimeOutsideComparison);
    }
    Ok(())
}

fn validate(
    tokamak: &TokamakJetDte1,
    state: &TokamakState,
    load: TokamakLoad,
    dt_s: f64,
    envelope: Envelope,
) -> Result<(), StepReject> {
    if !dt_s.is_finite()
        || !load.heating_power_w.is_finite()
        || !load.ohmic_power_w.is_finite()
        || !load.line_density_per_m3.is_finite()
        || !load.plasma_current_a.is_finite()
        || !load.toroidal_field_t.is_finite()
        || !load.tritium_fraction.is_finite()
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
    state_in(tokamak, state.thermal_energy_j, state.fast_ion_energy_j, load, envelope)?;
    if !(state.t_s >= 0.0)
        || !(state.t_s + dt_s <= tokamak.time_max_s + tokamak.time_end_allowance_s(dt_s))
    {
        return Err(StepReject::TimeBeyondCard);
    }
    Ok(())
}

fn temperature_in_fit(tokamak: &TokamakJetDte1, thermal_energy_j: f64, load: TokamakLoad) -> Result<(), StepReject> {
    let temperature = tokamak.temperature_kev(thermal_energy_j, load.line_density_per_m3);
    if temperature < REACTIVITY_TEMPERATURE_MIN_KEV {
        return Err(StepReject::TemperatureBelowReactivityFit);
    }
    if (1.0 + tokamak.profile_exponent) * temperature > REACTIVITY_TEMPERATURE_MAX_KEV {
        return Err(StepReject::TemperatureAboveReactivityFit);
    }
    Ok(())
}

fn commit(
    tokamak: &TokamakJetDte1,
    state: &mut TokamakState,
    frame: &mut LastStateFrame64,
    load: TokamakLoad,
    dt_s: f64,
    advanced: [f64; 3],
    envelope: Envelope,
) -> Result<(), StepReject> {
    if !advanced[0].is_finite() || !advanced[1].is_finite() || !advanced[2].is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    state_in(tokamak, advanced[0], advanced[1], load, envelope)?;
    let beta = tokamak.beta_normalised(
        advanced[0] + advanced[1],
        load.plasma_current_a,
        load.toroidal_field_t,
    );
    let saved = *state;
    let old = state_vector(state);
    let (rate_old, derived_old) = rates(tokamak, load, &old);
    let (rate_new, derived_new) = rates(tokamak, load, &advanced);
    let loss_rate_old = derived_old.loss_power_w * rate_old[0] / (IPB98_EXP_LOSS * old[0]);
    let loss_rate_new = derived_new.loss_power_w * rate_new[0] / (IPB98_EXP_LOSS * advanced[0]);
    let loss_energy = 0.5 * dt_s * (derived_old.loss_power_w + derived_new.loss_power_w)
        + dt_s * dt_s / 12.0 * (loss_rate_old - loss_rate_new);
    state.t_s += dt_s;
    state.thermal_energy_j = advanced[0];
    state.fast_ion_energy_j = advanced[1];
    state.fusion_energy_j = advanced[2];
    state.temperature_kev = derived_new.temperature_kev;
    state.loss_power_w = derived_new.loss_power_w;
    state.confinement_time_s = advanced[0] / derived_new.loss_power_w;
    state.slowing_time_s = derived_new.slowing_time_s;
    state.fusion_power_w = derived_new.fusion_power_w;
    state.fusion_power_thermal_w = derived_new.fusion_power_thermal_w;
    state.beta_normalised = beta;
    state.greenwald_fraction =
        load.line_density_per_m3 / tokamak.greenwald_density_per_m3(load.plasma_current_a);
    state.safety_factor = tokamak.safety_factor(load.plasma_current_a, load.toroidal_field_t);
    state.energy_supplied_j += (load.heating_power_w + load.ohmic_power_w) * dt_s;
    state.energy_lost_j += loss_energy;
    let stored = state.thermal_energy_j + state.fast_ion_energy_j;
    state.energy_defect_j = stored
        - state.energy_anchor_j
        - (state.energy_supplied_j + ALPHA_ENERGY_SHARE * state.fusion_energy_j - state.energy_lost_j);
    let scale = if stored > state.energy_anchor_j {
        stored
    } else {
        state.energy_anchor_j
    };
    state.residual_conservation_error = state.energy_defect_j.abs() / scale;
    state.near_density_limit = state.greenwald_fraction > NEAR_DENSITY_LIMIT_FRACTION;
    if !state_is_finite(state) || !(scale > 0.0) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > tokamak.residual_conservation_limit {
        let created = state.energy_defect_j > 0.0;
        *state = saved;
        return Err(if created {
            StepReject::SpontaneousGeneration
        } else {
            StepReject::FirstLawBreach
        });
    }
    publish(state, frame);
    Ok(())
}

/// The three variables in the order of the Runge–Kutta step: W_th, W_f, E_fus.
fn state_vector(state: &TokamakState) -> [f64; 3] {
    [state.thermal_energy_j, state.fast_ion_energy_j, state.fusion_energy_j]
}

fn derived(tokamak: &TokamakJetDte1, load: TokamakLoad, y: &[f64; 3]) -> Derived {
    let n19 = load.line_density_per_m3 / 1.0e19;
    let aspect = tokamak.minor_radius_m / tokamak.major_radius_m;
    let coefficient = tokamak.confinement_multiplier
        * IPB98_COEFF
        * exp(IPB98_EXP_CURRENT * log(load.plasma_current_a / 1.0e6))
        * exp(IPB98_EXP_FIELD * log(load.toroidal_field_t))
        * exp(IPB98_EXP_DENSITY * log(n19))
        * exp(IPB98_EXP_MASS * log(2.0 + load.tritium_fraction))
        * exp(IPB98_EXP_RADIUS * log(tokamak.major_radius_m))
        * exp(IPB98_EXP_ASPECT * log(aspect))
        * exp(IPB98_EXP_ELONGATION * log(tokamak.elongation));
    let loss_power_w = 1.0e6 * exp(log(y[0] / 1.0e6 / coefficient) / IPB98_EXP_LOSS);
    let temperature_kev = tokamak.temperature_kev(y[0], load.line_density_per_m3);
    let slowing_time_s = tokamak.slowing_time_coeff_s * temperature_kev.sqrt() / n19;
    let tritium = load.tritium_fraction;
    let deuterium = 1.0 - tritium;
    let main_density = tokamak.main_ion_fraction() * load.line_density_per_m3;
    let fusion_power_thermal_w = if deuterium * tritium > 0.0 {
        DT_REACTION_ENERGY_J
            * deuterium
            * tritium
            * main_density
            * main_density
            * tokamak.plasma_volume_m3
            * profile_reactivity_m3_per_s(temperature_kev, tokamak.profile_exponent)
    } else {
        0.0
    };
    let fusion_power_beam_w =
        1.0e6 * tokamak.beam_fusion_coeff_mw_per_mj * (y[1] / 1.0e6) * n19 * 4.0 * deuterium * tritium;
    Derived {
        loss_power_w,
        temperature_kev,
        slowing_time_s,
        fusion_power_thermal_w,
        fusion_power_w: fusion_power_thermal_w + fusion_power_beam_w,
    }
}

/// The right-hand side of the three equations at `y`.
fn rates(tokamak: &TokamakJetDte1, load: TokamakLoad, y: &[f64; 3]) -> ([f64; 3], Derived) {
    let d = derived(tokamak, load, y);
    let thermalised_w = y[1] / d.slowing_time_s;
    (
        [
            thermalised_w + load.ohmic_power_w - d.loss_power_w,
            load.heating_power_w + ALPHA_ENERGY_SHARE * d.fusion_power_w - thermalised_w,
            d.fusion_power_w,
        ],
        d,
    )
}

fn advance(y: &[f64; 3], k: &[f64; 3], scale: f64) -> [f64; 3] {
    [y[0] + scale * k[0], y[1] + scale * k[1], y[2] + scale * k[2]]
}

/// Classical fourth-order Runge–Kutta.
fn rk4(tokamak: &TokamakJetDte1, load: TokamakLoad, y: &[f64; 3], dt_s: f64) -> [f64; 3] {
    let k1 = rates(tokamak, load, y).0;
    let k2 = rates(tokamak, load, &advance(y, &k1, 0.5 * dt_s)).0;
    let k3 = rates(tokamak, load, &advance(y, &k2, 0.5 * dt_s)).0;
    let k4 = rates(tokamak, load, &advance(y, &k3, dt_s)).0;
    let mut out = [0.0; 3];
    let mut index = 0;
    while index < 3 {
        out[index] =
            y[index] + dt_s / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    out
}

fn publish(state: &TokamakState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.thermal_energy_j as f32;
    frame.pos[1] = state.fast_ion_energy_j as f32;
    frame.pos[2] = state.fusion_energy_j as f32;
    frame.vel[0] = state.fusion_power_w as f32;
    frame.vel[1] = state.temperature_kev as f32;
    frame.vel[2] = state.beta_normalised as f32;
    frame.force_torque = state.loss_power_w as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.near_density_limit {
        FLAG_NEAR_DENSITY_LIMIT
    } else {
        0
    };
}

fn state_is_finite(state: &TokamakState) -> bool {
    state.t_s.is_finite()
        && state.thermal_energy_j.is_finite()
        && state.fast_ion_energy_j.is_finite()
        && state.fusion_energy_j.is_finite()
        && state.temperature_kev.is_finite()
        && state.loss_power_w.is_finite()
        && state.confinement_time_s.is_finite()
        && state.slowing_time_s.is_finite()
        && state.fusion_power_w.is_finite()
        && state.fusion_power_thermal_w.is_finite()
        && state.beta_normalised.is_finite()
        && state.greenwald_fraction.is_finite()
        && state.safety_factor.is_finite()
        && state.energy_anchor_j.is_finite()
        && state.energy_supplied_j.is_finite()
        && state.energy_lost_j.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        ALPHA_ENERGY_SHARE, BETA_NORMALISED_COMPARED_MAX, BETA_NORMALISED_COMPARED_MIN,
        CONFINEMENT_TIME_MAX_S, CONFINEMENT_TIME_MIN_S, Envelope,
        FAST_ION_ENERGY_MAX_J, FAST_ION_ENERGY_MIN_J, FLOOR_DT_S, GATE_DT_S,
        GREENWALD_FRACTION_MAX, HEATING_POWER_MAX_W, HEATING_POWER_MIN_W, IPB98_EXP_LOSS,
        LINE_DENSITY_MAX_PER_M3, LINE_DENSITY_MIN_PER_M3, OHMIC_POWER_MAX_W, OHMIC_POWER_MIN_W,
        PLASMA_CURRENT_MAX_A, PLASMA_CURRENT_MIN_A, PROFILE_EXPONENT, REACTIVITY_COEFF,
        RESIDUAL_CONSERVATION_LIMIT, SAFETY_FACTOR_COMPARED_MAX, SAFETY_FACTOR_COMPARED_MIN,
        StepReject, TEMPERATURE_COMPARED_MAX_KEV, TEMPERATURE_COMPARED_MIN_KEV, THERMAL_ENERGY_MAX_J, THERMAL_ENERGY_MIN_J,
        TIME_END_ALLOWANCE_EXTRA_STEPS, TIME_END_ALLOWANCE_PER_STEP_S, TIME_MAX_S,
        TOROIDAL_FIELD_MAX_T, TOROIDAL_FIELD_MIN_T, TRITIUM_FRACTION_EVEN_MAX,
        TRITIUM_FRACTION_EVEN_MIN, TRITIUM_FRACTION_TRACE_MAX, TROYON_BETA_NORMALISED_MAX,
        TokamakJetDte1, TokamakLoad, TokamakState, advance, commit, load_in_card,
        profile_reactivity_m3_per_s, rates, reactivity_dt_m3_per_s, rk4, state_in_card,
        state_vector, step, validate,
    };
    use crate::frame::LastStateFrame64;

    fn blank_frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: -1.0,
            pos: [1.0, 2.0, 3.0],
            vel: [4.0, 5.0, 6.0],
            force_torque: 7.0,
            residual: 8.0,
            flags: 9,
            proof: [0xA5; 16],
        }
    }

    fn proof_intact(frame: &LastStateFrame64) {
        let mut index = 0;
        while index < 16 {
            assert_eq!(frame.proof[index], 0xA5, "proof byte {index} was resealed");
            index += 1;
        }
    }

    fn assert_copy<T: Copy>() {}

    fn within(got: f64, expected: f64, relative: f64) -> bool {
        (got - expected).abs() <= relative * expected.abs()
    }

    /// An even D-T mix at 3.8 MA and 4.0 T, the first case of `analysis/05_reference_run.py`.
    fn even_mix() -> TokamakLoad {
        TokamakLoad {
            heating_power_w: 20.0e6,
            ohmic_power_w: 0.5e6,
            line_density_per_m3: 6.0e19,
            plasma_current_a: 3.8e6,
            toroidal_field_t: 4.0,
            tritium_fraction: 0.5,
        }
    }

    /// Deuterium at 2.5 MA and 2.6 T, the second case of `analysis/05_reference_run.py`.
    fn deuterium() -> TokamakLoad {
        TokamakLoad {
            heating_power_w: 10.0e6,
            ohmic_power_w: 1.0e6,
            line_density_per_m3: 4.0e19,
            plasma_current_a: 2.5e6,
            toroidal_field_t: 2.6,
            tritium_fraction: 0.0,
        }
    }

    fn start(thermal_j: f64, fast_j: f64) -> (TokamakJetDte1, TokamakState) {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let state = TokamakState::try_start(&tokamak, thermal_j, fast_j).unwrap();
        (tokamak, state)
    }

    /// The real `validate`, a chosen integrator, the real `commit`, held to the envelope.
    fn step_with(
        tokamak: &TokamakJetDte1,
        state: &mut TokamakState,
        frame: &mut LastStateFrame64,
        load: TokamakLoad,
        dt_s: f64,
        integrate: impl FnOnce(&TokamakJetDte1, TokamakLoad, &[f64; 3], f64) -> [f64; 3],
    ) -> Result<(), StepReject> {
        validate(tokamak, state, load, dt_s, Envelope::Compared)?;
        let advanced = integrate(tokamak, load, &state_vector(state), dt_s);
        commit(tokamak, state, frame, load, dt_s, advanced, Envelope::Compared)
    }

    /// The real step without the envelope of the comparison: the structure
    /// the forecast script runs, from starts the comparison never reached.
    fn step_structure(
        tokamak: &TokamakJetDte1,
        state: &mut TokamakState,
        frame: &mut LastStateFrame64,
        load: TokamakLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(tokamak, state, load, dt_s, Envelope::Structure)?;
        let advanced = rk4(tokamak, load, &state_vector(state), dt_s);
        commit(tokamak, state, frame, load, dt_s, advanced, Envelope::Structure)
    }

    /// A start inside the envelope of the comparison under [`even_mix`]:
    /// 1.9 keV, a normalised beta of 0.57, a confinement time of 2.0 s.
    fn inside() -> (TokamakJetDte1, TokamakState) {
        start(4.0e6, 0.5e6)
    }

    fn euler(tokamak: &TokamakJetDte1, load: TokamakLoad, y: &[f64; 3], dt_s: f64) -> [f64; 3] {
        advance(y, &rates(tokamak, load, y).0, dt_s)
    }

    /// The allowance is what its derivation says: 2^-49 s a step, the steps a
    /// run takes to reach the end and two more, and never half a step or more.
    #[test]
    fn the_time_end_allowance_is_its_derivation() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let spacing_s = f64::from_bits(TIME_MAX_S.to_bits() + 1) - TIME_MAX_S;
        assert_eq!(TIME_END_ALLOWANCE_PER_STEP_S, 0.5 * spacing_s);
        assert_eq!(TIME_END_ALLOWANCE_EXTRA_STEPS, 2.0);
        let ceiling_s = tokamak.gate_dt_s().unwrap();
        let floor_s = tokamak.floor_dt_s().unwrap();
        assert_eq!(
            tokamak.time_end_allowance_s(ceiling_s),
            (TIME_MAX_S / ceiling_s + 2.0) * TIME_END_ALLOWANCE_PER_STEP_S
        );
        let at_floor = tokamak.time_end_allowance_s(floor_s);
        assert_eq!(at_floor, (TIME_MAX_S / floor_s + 2.0) * TIME_END_ALLOWANCE_PER_STEP_S);
        // inside the step floor it is under a tenth of the step
        assert!(at_floor < 0.1 * floor_s);
        // far under the floor it is held to half the step
        assert_eq!(tokamak.time_end_allowance_s(1.0e-6 * floor_s), 0.5e-6 * floor_s);
        assert_eq!(tokamak.time_end_allowance_s(0.0), 0.0);
        assert_eq!(tokamak.time_end_allowance_s(-1.0), 0.0);
        let mut altered = tokamak;
        altered.time_end_allowance_per_step_s = 1.0e-9;
        assert!(!altered.is_valid());
        let mut altered = tokamak;
        altered.time_end_allowance_extra_steps = 3.0;
        assert!(!altered.is_valid());
    }

    /// Runs that end on the end of the card in decimal land every step at
    /// twelve step sizes, and the step after is refused by name.
    #[test]
    fn a_decimal_run_to_the_end_of_the_card_lands_every_step() {
        // (the step as a decimal, steps to the end)
        let sizes: [(&str, usize); 12] = [
            ("0.001", 5_475),
            ("0.00073", 7_500),
            ("0.0005", 10_950),
            ("0.000365", 15_000),
            ("0.0003", 18_250),
            ("0.00025", 21_900),
            ("0.000219", 25_000),
            ("0.0002", 27_375),
            ("0.00015", 36_500),
            ("0.0001", 54_750),
            ("0.00005", 109_500),
            ("0.000025", 219_000),
        ];
        let mut past_the_end = 0;
        for (text, count) in sizes {
            let dt_s: f64 = text.parse().unwrap();
            let (tokamak, mut state) = inside();
            let mut frame = blank_frame();
            for landed in 0..count {
                assert_eq!(
                    step(&tokamak, &mut state, &mut frame, even_mix(), dt_s),
                    Ok(()),
                    "dt {text}: step {} of {count}",
                    landed + 1
                );
            }
            assert!((state.t_s - TIME_MAX_S).abs() <= tokamak.time_end_allowance_s(dt_s));
            if state.t_s > TIME_MAX_S {
                past_the_end += 1;
            }
            let before = state;
            let held = frame.to_bytes();
            assert_eq!(
                step(&tokamak, &mut state, &mut frame, even_mix(), dt_s),
                Err(StepReject::TimeBeyondCard)
            );
            assert_eq!(state, before);
            assert_eq!(frame.to_bytes(), held);
        }
        assert!(past_the_end > 0);
    }

    /// A step that would end later than the end of the card by a spacing
    /// more than the allowance is refused by name and nothing moves; every
    /// end inside the allowance lands at its own value.
    #[test]
    fn a_step_beyond_the_time_allowance_is_refused_by_name() {
        let spacing_s = f64::from_bits(TIME_MAX_S.to_bits() + 1) - TIME_MAX_S;
        for dt_s in [0.001, 0.0005, 0.0001, 0.00005] {
            let (tokamak, fresh) = inside();
            let allowance_s = tokamak.time_end_allowance_s(dt_s);
            // the comparison is with the 64-bit number nearest the end plus the allowance
            let whole_spacings = (((TIME_MAX_S + allowance_s) - TIME_MAX_S) / spacing_s) as u64;
            assert!(whole_spacings >= 1);
            let mut landed_past_the_end = 0;
            let mut refused = 0;
            for spacings in (0..whole_spacings + 4).rev().take(8).chain(0..2) {
                let end_s = f64::from_bits(TIME_MAX_S.to_bits() + spacings);
                // the clock whose sum with the step is this end
                let mut t_s = end_s - dt_s;
                while t_s + dt_s < end_s {
                    t_s = f64::from_bits(t_s.to_bits() + 1);
                }
                assert_eq!(t_s + dt_s, end_s);
                let mut state = fresh;
                state.t_s = t_s;
                let before = state;
                let mut frame = blank_frame();
                let result = step(&tokamak, &mut state, &mut frame, even_mix(), dt_s);
                if end_s <= TIME_MAX_S + allowance_s {
                    assert_eq!(result, Ok(()));
                    assert_eq!(state.t_s, end_s);
                    if spacings > 0 {
                        landed_past_the_end += 1;
                    }
                } else {
                    assert_eq!(result, Err(StepReject::TimeBeyondCard));
                    assert_eq!(state, before);
                    assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
                    refused += 1;
                }
            }
            assert!(landed_past_the_end >= 1);
            assert_eq!(refused, 3);
            // one allowance and a half beyond is refused
            let mut state = fresh;
            state.t_s = TIME_MAX_S + 1.5 * allowance_s - dt_s;
            let mut frame = blank_frame();
            assert_eq!(
                step(&tokamak, &mut state, &mut frame, even_mix(), dt_s),
                Err(StepReject::TimeBeyondCard)
            );
        }
    }

    #[test]
    fn constants_are_locked() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        assert!(tokamak.is_valid());
        assert_eq!(tokamak.confinement_multiplier, 0.829118);
        assert_eq!(tokamak.slowing_time_coeff_s, 0.160795);
        assert_eq!(tokamak.profile_exponent, 0.308417);
        assert_eq!(tokamak.beam_fusion_coeff_mw_per_mj, 0.364521);
        assert_eq!(tokamak.major_radius_m, 2.899);
        assert_eq!(tokamak.minor_radius_m, 0.9386);
        assert_eq!(tokamak.elongation, 1.748);
        assert_eq!(tokamak.plasma_volume_m3, 81.56);
        assert_eq!(tokamak.effective_charge, 2.174);
        assert_eq!(REACTIVITY_COEFF[0], 1.17302e-9);
        assert_eq!(ALPHA_ENERGY_SHARE, 0.2);
        // Every recorded copy is held to its constant, one at a time.
        let alter: [fn(&mut TokamakJetDte1); 16] = [
            |t| t.major_radius_m += 1.0e-3,
            |t| t.minor_radius_m += 1.0e-3,
            |t| t.elongation += 1.0e-3,
            |t| t.plasma_volume_m3 += 1.0e-3,
            |t| t.effective_charge += 1.0e-3,
            |t| t.confinement_multiplier *= 1.5,
            |t| t.slowing_time_coeff_s *= 1.5,
            |t| t.profile_exponent *= 1.5,
            |t| t.beam_fusion_coeff_mw_per_mj *= 1.5,
            |t| t.reactivity_coeff[6] *= 1.5,
            |t| t.troyon_beta_normalised_max = 10.0,
            |t| t.safety_factor_min = 0.5,
            |t| t.time_max_s = 1.0e9,
            |t| t.residual_conservation_limit = 1.0,
            |t| t.gate_dt_s = 1.0,
            |t| t.floor_dt_s = 1.0e-12,
        ];
        for change in alter {
            let mut altered = tokamak;
            change(&mut altered);
            assert!(!altered.is_valid());
            let mut state = TokamakState::try_start(&tokamak, 2.0e6, 0.5e6).unwrap();
            let held = state;
            let mut frame = blank_frame();
            assert_eq!(
                step(&altered, &mut state, &mut frame, even_mix(), GATE_DT_S),
                Err(StepReject::InvalidCard)
            );
            assert_eq!(state, held);
            proof_intact(&frame);
        }
    }

    /// The reactivity against the 64-bit evaluation of the same fit in
    /// `analysis/05_reference_run.py`, and against the values of Bosch and
    /// Hale's Table VIII as they are commonly quoted (6.857e-21, 1.136e-16,
    /// 4.330e-16 and 8.649e-16 cm³/s at 1, 10, 20 and 50 keV; the paper is
    /// not open and was not fetched, so the second comparison is to four
    /// figures from memory of that table and is not a sourced reference).
    #[test]
    fn the_reactivity_is_the_bosch_hale_fit() {
        for (kev, reference) in [
            (1.0, 6.8568842495932544e-27),
            (10.0, 1.1361654705836214e-22),
            (20.0, 4.3302018955382073e-22),
            (50.0, 8.6490849454868903e-22),
            (100.0, 8.4476624586658349e-22),
        ] {
            assert!(within(reactivity_dt_m3_per_s(kev), reference, 1.0e-13), "{kev}");
        }
        for (kev, quoted_cm3) in [(1.0, 6.857e-21), (10.0, 1.136e-16), (20.0, 4.330e-16), (50.0, 8.649e-16)] {
            assert!(within(reactivity_dt_m3_per_s(kev) * 1.0e6, quoted_cm3, 5.0e-4), "{kev}");
        }
        assert!(within(
            profile_reactivity_m3_per_s(5.0, PROFILE_EXPONENT),
            1.6532693819112194e-23,
            1.0e-13
        ));
        // A flat profile is the reactivity itself.
        assert!(within(profile_reactivity_m3_per_s(12.0, 0.0), reactivity_dt_m3_per_s(12.0), 1.0e-14));
    }

    /// 1000 steps at the ceiling under a held load against the plain-Python
    /// run of the same structure (`analysis/out/05_reference_run.log`).
    #[test]
    fn a_second_at_the_ceiling_agrees_with_the_forecast_structure() {
        let (tokamak, mut state) = start(2.0e6, 0.5e6);
        let mut frame = blank_frame();
        let mut worst = 0.0_f64;
        // The start's confinement time, 9.3 s, is above the envelope: the
        // public step refuses it, and the structure is run without it.
        assert_eq!(
            step(&tokamak, &mut state.clone(), &mut blank_frame(), even_mix(), GATE_DT_S),
            Err(StepReject::ConfinementTimeOutsideComparison)
        );
        for _ in 0..1000 {
            step_structure(&tokamak, &mut state, &mut frame, even_mix(), GATE_DT_S).unwrap();
            worst = worst.max(state.residual_conservation_error);
        }
        assert!(within(state.thermal_energy_j, 8287198.7043702248, 1.0e-11));
        assert!(within(state.fast_ion_energy_j, 1092766.6910203076, 1.0e-11));
        assert!(within(state.fusion_energy_j, 2635617.1506323256, 1.0e-11));
        assert!(within(state.temperature_kev, 3.9053580733902336, 1.0e-11));
        assert!(within(state.loss_power_w, 21041016.569105957, 1.0e-10));
        assert!(within(state.slowing_time_s, 0.052960456437011354, 1.0e-11));
        assert!(within(state.fusion_power_w, 3222437.0650198795, 1.0e-10));
        assert!(within(state.fusion_power_thermal_w, 832418.62315539806, 1.0e-10));
        // The forecast script writes the vacuum permeability as 4π × 1e-7; the card uses the CODATA 2018 value.
        assert!(within(state.beta_normalised, 1.1898975810266619, 1.0e-9));
        assert!(within(state.safety_factor, 3.8835285730888298, 1.0e-13));
        assert!(within(state.greenwald_fraction, 6.0e19 / 1.3730065977486959e20, 1.0e-13));
        assert!(worst < 1.0e-10, "{worst}");
        assert!(within(state.t_s, 1.0, 1.0e-12));
        assert_eq!(frame.t, state.t_s);
        assert_eq!(frame.pos[0], state.thermal_energy_j as f32);
        assert_eq!(frame.vel[0], state.fusion_power_w as f32);
        assert_eq!(frame.flags, 0);
        proof_intact(&frame);

        let (tokamak, mut state) = start(1.0e6, 0.0);
        for _ in 0..1000 {
            step_structure(&tokamak, &mut state, &mut frame, deuterium(), GATE_DT_S).unwrap();
        }
        assert!(within(state.thermal_energy_j, 3492366.1133224079, 1.0e-11));
        assert!(within(state.fast_ion_energy_j, 631451.60550995078, 1.0e-11));
        assert_eq!(state.fusion_energy_j, 0.0);
        assert_eq!(state.fusion_power_w, 0.0);
    }

    /// Held long enough, the plasma settles where the loss power equals the
    /// power supplied, and the confinement time there is the scaling's.
    #[test]
    fn a_held_load_settles_at_the_power_balance() {
        let (tokamak, mut state) = start(1.0e6, 0.0);
        let mut frame = blank_frame();
        // Twelve seconds pass the end of the card, so the clock is held at
        // the start; nothing else reads it.
        for _ in 0..12_000 {
            state.t_s = 0.0;
            step_structure(&tokamak, &mut state, &mut frame, deuterium(), GATE_DT_S).unwrap();
        }
        assert!(within(state.loss_power_w, 11.0e6, 1.0e-6), "{}", state.loss_power_w);
        assert!(within(
            state.fast_ion_energy_j / state.slowing_time_s,
            10.0e6,
            1.0e-6
        ));
        assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
    }

    /// A load from its six numbers.
    fn load(heating_w: f64, ohmic_w: f64, density: f64, current_a: f64, field_t: f64, tritium: f64) -> TokamakLoad {
        TokamakLoad {
            heating_power_w: heating_w,
            ohmic_power_w: ohmic_w,
            line_density_per_m3: density,
            plasma_current_a: current_a,
            toroidal_field_t: field_t,
            tritium_fraction: tritium,
        }
    }

    /// Each load refused by name before the step, with state and frame kept.
    #[test]
    fn every_refusal_is_named_and_rolls_back_state_and_frame() {
        let (tokamak, state) = inside();
        let base = even_mix();
        let low_current = TokamakLoad { plasma_current_a: 1.65e6, toroidal_field_t: 1.80, ..base };
        let mid_current = TokamakLoad { plasma_current_a: 2.5e6, toroidal_field_t: 2.6, ..base };
        let cases: [(TokamakLoad, f64, StepReject); 35] = [
            (TokamakLoad { heating_power_w: f64::NAN, ..base }, GATE_DT_S, StepReject::NonFiniteInput),
            (TokamakLoad { tritium_fraction: f64::INFINITY, ..base }, GATE_DT_S, StepReject::NonFiniteInput),
            (TokamakLoad { ohmic_power_w: f64::NEG_INFINITY, ..base }, GATE_DT_S, StepReject::NonFiniteInput),
            (base, f64::NAN, StepReject::NonFiniteInput),
            (base, 0.0, StepReject::NonPositiveDt),
            (base, -1.0e-3, StepReject::NonPositiveDt),
            (TokamakLoad { heating_power_w: 0.0, ..base }, GATE_DT_S, StepReject::HeatingPowerOutsideCard),
            (TokamakLoad { heating_power_w: HEATING_POWER_MIN_W * 0.999999, ..base }, GATE_DT_S, StepReject::HeatingPowerOutsideCard),
            (TokamakLoad { heating_power_w: HEATING_POWER_MAX_W * 1.000001, ..base }, GATE_DT_S, StepReject::HeatingPowerOutsideCard),
            (TokamakLoad { heating_power_w: 30.0e6, ..base }, GATE_DT_S, StepReject::HeatingPowerOutsideCard),
            (TokamakLoad { ohmic_power_w: 0.0, ..base }, GATE_DT_S, StepReject::OhmicPowerOutsideCard),
            (TokamakLoad { ohmic_power_w: OHMIC_POWER_MIN_W * 0.999999, ..base }, GATE_DT_S, StepReject::OhmicPowerOutsideCard),
            (TokamakLoad { ohmic_power_w: OHMIC_POWER_MAX_W * 1.000001, ..base }, GATE_DT_S, StepReject::OhmicPowerOutsideCard),
            (TokamakLoad { plasma_current_a: PLASMA_CURRENT_MIN_A * 0.999999, ..low_current }, GATE_DT_S, StepReject::PlasmaCurrentOutsideCard),
            (TokamakLoad { plasma_current_a: PLASMA_CURRENT_MAX_A * 1.000001, ..base }, GATE_DT_S, StepReject::PlasmaCurrentOutsideCard),
            (TokamakLoad { toroidal_field_t: TOROIDAL_FIELD_MIN_T * 0.999999, ..low_current }, GATE_DT_S, StepReject::ToroidalFieldOutsideCard),
            (TokamakLoad { toroidal_field_t: TOROIDAL_FIELD_MAX_T * 1.000001, ..base }, GATE_DT_S, StepReject::ToroidalFieldOutsideCard),
            (TokamakLoad { tritium_fraction: -1.0e-9, ..base }, GATE_DT_S, StepReject::FuelMixOutsideCard),
            // Never compared: between the trace-tritium pulse and the held-back pulses.
            (TokamakLoad { tritium_fraction: 0.0341, ..base }, GATE_DT_S, StepReject::FuelMixOutsideComparison),
            (TokamakLoad { tritium_fraction: 0.25, ..base }, GATE_DT_S, StepReject::FuelMixOutsideComparison),
            (TokamakLoad { tritium_fraction: 0.4509, ..base }, GATE_DT_S, StepReject::FuelMixOutsideComparison),
            // Shown wrong: the training pulses at 0.75 to 0.93, and beyond.
            (TokamakLoad { tritium_fraction: 0.5041, ..base }, GATE_DT_S, StepReject::FuelMixOutsideComparison),
            (TokamakLoad { tritium_fraction: 0.93, ..base }, GATE_DT_S, StepReject::FuelMixOutsideComparison),
            (TokamakLoad { tritium_fraction: 1.0, ..base }, GATE_DT_S, StepReject::FuelMixOutsideComparison),
            (TokamakLoad { line_density_per_m3: LINE_DENSITY_MIN_PER_M3 * 0.999999, ..base }, GATE_DT_S, StepReject::DensityBelowCard),
            (TokamakLoad { line_density_per_m3: LINE_DENSITY_MAX_PER_M3 * 1.000001, plasma_current_a: PLASMA_CURRENT_MAX_A, ..base }, GATE_DT_S, StepReject::DensityAboveComparison),
            (TokamakLoad { line_density_per_m3: tokamak.greenwald_density_per_m3(1.65e6) * 1.000001, ..low_current }, GATE_DT_S, StepReject::DensityAboveGreenwald),
            (TokamakLoad { line_density_per_m3: GREENWALD_FRACTION_MAX * tokamak.greenwald_density_per_m3(2.5e6) * 1.000001, ..mid_current }, GATE_DT_S, StepReject::GreenwaldFractionAboveComparison),
            // 3.84 MA at 1.80 T: an edge safety factor of 1.73.
            (TokamakLoad { plasma_current_a: 3.84e6, toroidal_field_t: 1.80, ..base }, GATE_DT_S, StepReject::SafetyFactorBelowLimit),
            // 3.0 MA at 3.4 T: 4.18; 3.84 MA at 3.8 T: 3.65.
            (TokamakLoad { plasma_current_a: 3.0e6, toroidal_field_t: 3.4, ..base }, GATE_DT_S, StepReject::SafetyFactorOutsideComparison),
            (TokamakLoad { plasma_current_a: 3.84e6, toroidal_field_t: 3.8, ..base }, GATE_DT_S, StepReject::SafetyFactorOutsideComparison),
            (base, TIME_MAX_S + GATE_DT_S, StepReject::TimeBeyondCard),
            (base, 6.0, StepReject::TimeBeyondCard),
            (base, 21.0, StepReject::TimeBeyondCard),
            (TokamakLoad { tritium_fraction: f64::NAN, ..base }, GATE_DT_S, StepReject::NonFiniteInput),
        ];
        for (index, (load, dt_s, expected)) in cases.iter().enumerate() {
            let mut trial = state;
            let held = trial;
            let mut frame = blank_frame();
            assert_eq!(step(&tokamak, &mut trial, &mut frame, *load, *dt_s), Err(*expected), "case {index}");
            assert_eq!(trial, held, "case {index}");
            assert_eq!(frame.t, -1.0);
            assert_eq!(frame.flags, 9);
            proof_intact(&frame);
            if *expected != StepReject::NonFiniteInput && *expected != StepReject::NonPositiveDt && *expected != StepReject::TimeBeyondCard {
                assert_eq!(load_in_card(&tokamak, *load), Err(*expected), "case {index}");
            }
        }
        // A state that is not a plasma.
        for (thermal, fast) in [(0.0, 1.0), (-1.0, 0.0), (1.0e6, -1.0), (f64::NAN, 0.0), (1.0e6, f64::INFINITY)] {
            assert!(TokamakState::try_start(&tokamak, thermal, fast).is_none());
        }
        for elapsed in [-1.0e-12, f64::NAN, f64::INFINITY] {
            assert!(TokamakState::try_continue(&tokamak, elapsed, 4.0e6, 0.5e6).is_none());
        }
        let mut broken = state;
        broken.fast_ion_energy_j = -1.0;
        let held = broken;
        let mut frame = blank_frame();
        assert_eq!(step(&tokamak, &mut broken, &mut frame, base, GATE_DT_S), Err(StepReject::EnergyOutsideCard));
        assert_eq!(broken, held);
        let mut broken = state;
        broken.loss_power_w = f64::NAN;
        assert_eq!(step(&tokamak, &mut broken, &mut frame, base, GATE_DT_S), Err(StepReject::NonFiniteState));
        let mut broken = state;
        broken.t_s = -1.0e-9;
        assert_eq!(step(&tokamak, &mut broken, &mut frame, base, GATE_DT_S), Err(StepReject::TimeBeyondCard));
        proof_intact(&frame);
    }

    /// The value at each bound of a load is accepted, each with a start the
    /// envelope holds under it.
    #[test]
    fn the_value_at_each_bound_of_a_load_is_accepted() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let base = even_mix();
        let cases: [((f64, f64), TokamakLoad); 13] = [
            ((4.0e6, 0.5e6), TokamakLoad { heating_power_w: HEATING_POWER_MIN_W, ..base }),
            ((4.0e6, 0.5e6), TokamakLoad { heating_power_w: HEATING_POWER_MAX_W, ..base }),
            ((4.0e6, 0.5e6), TokamakLoad { ohmic_power_w: OHMIC_POWER_MIN_W, ..base }),
            ((4.0e6, 0.5e6), TokamakLoad { ohmic_power_w: OHMIC_POWER_MAX_W, ..base }),
            ((4.0e6, 0.5e6), TokamakLoad { tritium_fraction: 0.0, ..base }),
            ((4.0e6, 0.5e6), TokamakLoad { tritium_fraction: TRITIUM_FRACTION_TRACE_MAX, ..base }),
            ((4.0e6, 0.5e6), TokamakLoad { tritium_fraction: TRITIUM_FRACTION_EVEN_MIN, ..base }),
            ((4.0e6, 0.5e6), TokamakLoad { tritium_fraction: TRITIUM_FRACTION_EVEN_MAX, ..base }),
            ((4.0e6, 0.5e6), TokamakLoad { plasma_current_a: PLASMA_CURRENT_MAX_A, toroidal_field_t: TOROIDAL_FIELD_MAX_T, ..base }),
            // The smallest current, field and density together, on a plasma of 0.6 MJ.
            ((0.6e6, 0.1e6), load(10.0e6, 1.0e6, LINE_DENSITY_MIN_PER_M3, PLASMA_CURRENT_MIN_A, TOROIDAL_FIELD_MIN_T, 0.5)),
            ((8.0e6, 1.0e6), TokamakLoad { line_density_per_m3: LINE_DENSITY_MAX_PER_M3, ..base }),
            ((5.0e6, 1.0e6), load(10.0e6, 1.0e6, GREENWALD_FRACTION_MAX * tokamak.greenwald_density_per_m3(2.5e6), 2.5e6, 2.6, 0.5)),
            ((4.0e6, 0.5e6), TokamakLoad { line_density_per_m3: tokamak.greenwald_density_per_m3(3.8e6) * GREENWALD_FRACTION_MAX * 0.5, ..base }),
        ];
        for (index, ((thermal, fast), load)) in cases.iter().enumerate() {
            let (_, mut trial) = start(*thermal, *fast);
            let mut frame = blank_frame();
            assert_eq!(state_in_card(&tokamak, *thermal, *fast, *load), Ok(()), "case {index}");
            assert_eq!(step(&tokamak, &mut trial, &mut frame, *load, GATE_DT_S), Ok(()), "case {index}");
            assert_eq!(frame.flags, 0, "case {index}");
        }
        // The safety factor's compared span, met from both sides by the field at 3.0 MA.
        let field_at = |q: f64| q / tokamak.safety_factor(3.0e6, 1.0);
        for (q, accepted) in [(SAFETY_FACTOR_COMPARED_MIN, true), (SAFETY_FACTOR_COMPARED_MAX, true), (0.999 * SAFETY_FACTOR_COMPARED_MIN, false), (1.001 * SAFETY_FACTOR_COMPARED_MAX, false)] {
            let at = TokamakLoad { plasma_current_a: 3.0e6, toroidal_field_t: field_at(q), ..base };
            let landed = load_in_card(&tokamak, at);
            if accepted {
                // the field rounds; inside the span by a part in 1e15 either way
                let q_back = tokamak.safety_factor(at.plasma_current_a, at.toroidal_field_t);
                assert_eq!(landed.is_ok(), q_back >= SAFETY_FACTOR_COMPARED_MIN && q_back <= SAFETY_FACTOR_COMPARED_MAX);
                let nudged = TokamakLoad { toroidal_field_t: field_at(q) * (1.0 + if q < 4.0 { 1.0e-12 } else { -1.0e-12 }), ..at };
                assert_eq!(load_in_card(&tokamak, nudged), Ok(()));
            } else {
                assert_eq!(landed, Err(StepReject::SafetyFactorOutsideComparison));
            }
        }
    }

    /// Each bound of the state is refused by name, at the start of a step, and
    /// the limits of physics are tested before the envelope.
    #[test]
    fn each_state_outside_the_envelope_is_refused_by_name() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let base = even_mix();
        let low = load(10.0e6, 1.0e6, 2.0e19, 1.65e6, 1.80, 0.5);
        let high_field = load(10.0e6, 1.0e6, 2.0e19, PLASMA_CURRENT_MAX_A, TOROIDAL_FIELD_MAX_T, 0.5);
        let dense = TokamakLoad { line_density_per_m3: LINE_DENSITY_MAX_PER_M3, ..base };
        let thin = TokamakLoad { line_density_per_m3: LINE_DENSITY_MIN_PER_M3, ..base };
        let cases: [((f64, f64), TokamakLoad, StepReject); 14] = [
            ((THERMAL_ENERGY_MIN_J * 0.999, 0.1e6), thin, StepReject::EnergyOutsideComparison),
            ((THERMAL_ENERGY_MAX_J * 1.001, 0.5e6), dense, StepReject::EnergyOutsideComparison),
            ((4.0e6, FAST_ION_ENERGY_MIN_J * 0.999), base, StepReject::EnergyOutsideComparison),
            ((4.0e6, 0.0), base, StepReject::EnergyOutsideComparison),
            ((4.0e6, FAST_ION_ENERGY_MAX_J * 1.001), base, StepReject::EnergyOutsideComparison),
            // 0.72 keV and 6.9 keV.
            ((2.0e6, 0.5e6), dense, StepReject::TemperatureOutsideComparison),
            ((4.0e6, 0.5e6), thin, StepReject::TemperatureOutsideComparison),
            // A normalised beta of 2.50 and of 0.131.
            ((3.0e6, 0.85e6), low, StepReject::BetaOutsideComparison),
            ((1.0e6, 0.05e6), high_field, StepReject::BetaOutsideComparison),
            // A confinement time of 0.063 s and of 9.3 s.
            ((2.4e6, 0.1e6), TokamakLoad { line_density_per_m3: LINE_DENSITY_MIN_PER_M3, ..low }, StepReject::ConfinementTimeOutsideComparison),
            ((2.0e6, 0.5e6), base, StepReject::ConfinementTimeOutsideComparison),
            // The limits of physics come first: 0.064 keV is below the fit
            // before it is below the envelope, 23 MJ at 3.8 MA and 4.0 T is
            // above the Troyon limit before it is above 9.57 MJ, and 46 MJ at
            // 1.65e19 per cubic metre is 103 keV at the centre of the profile.
            ((0.2e6, 0.0), dense, StepReject::TemperatureBelowReactivityFit),
            ((22.0e6, 1.0e6), dense, StepReject::BetaAboveTroyon),
            ((46.0e6, 0.0), thin, StepReject::TemperatureAboveReactivityFit),
        ];
        for (index, ((thermal, fast), load, expected)) in cases.iter().enumerate() {
            assert_eq!(state_in_card(&tokamak, *thermal, *fast, *load), Err(*expected), "case {index}");
            let mut state = TokamakState::try_start(&tokamak, *thermal, *fast).unwrap();
            let held = state;
            let mut frame = blank_frame();
            assert_eq!(step(&tokamak, &mut state, &mut frame, *load, GATE_DT_S), Err(*expected), "case {index}");
            assert_eq!(state, held, "case {index}");
            proof_intact(&frame);
        }
        // The confinement time the step tests is the one a landed step writes.
        let (_, mut state) = inside();
        let mut frame = blank_frame();
        step(&tokamak, &mut state, &mut frame, base, GATE_DT_S).unwrap();
        assert_eq!(
            tokamak.confinement_time_s(state.thermal_energy_j, base).to_bits(),
            state.confinement_time_s.to_bits()
        );
    }

    /// 0.6.0 refused in-range states with a confinement time of tens of
    /// milliseconds as FirstLawBreach at the ceiling, from the step's
    /// truncation error. That region is outside the envelope and is now
    /// refused by its own name before the step, at every step size; the same
    /// state stepped without the envelope lands at a tenth of the ceiling.
    #[test]
    fn a_fast_loss_state_is_refused_by_its_confinement_time_and_not_by_the_ledger() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let fast_loss = load(HEATING_POWER_MAX_W, OHMIC_POWER_MAX_W, LINE_DENSITY_MIN_PER_M3, 1.65e6, 1.80, 0.5);
        // 2.4 MJ there is 4.1 keV, a normalised beta of 1.6 with the fast
        // ions and a confinement time of 63 ms: every other bound holds.
        let thermal = 2.4e6;
        assert!(tokamak.temperature_kev(thermal, LINE_DENSITY_MIN_PER_M3) < TEMPERATURE_COMPARED_MAX_KEV);
        assert!(tokamak.beta_normalised(thermal + 0.1e6, 1.65e6, 1.80) < BETA_NORMALISED_COMPARED_MAX);
        assert!(tokamak.confinement_time_s(thermal, fast_loss) < 0.07);
        for dt_s in [GATE_DT_S, 1.0e-4, FLOOR_DT_S] {
            let (_, mut state) = start(thermal, 0.1e6);
            let mut frame = blank_frame();
            assert_eq!(
                step(&tokamak, &mut state, &mut frame, fast_loss, dt_s),
                Err(StepReject::ConfinementTimeOutsideComparison)
            );
        }
        let (_, mut state) = start(thermal, 0.1e6);
        let mut frame = blank_frame();
        assert_eq!(step_structure(&tokamak, &mut state, &mut frame, fast_loss, 1.0e-4), Ok(()));
        // The loss time is 0.31 of the confinement time: 20 ms here, against
        // 46.5 ms at the smallest confinement time the envelope allows.
        assert!(IPB98_EXP_LOSS * CONFINEMENT_TIME_MIN_S > 40.0 * GATE_DT_S);
    }

    /// Pulse 42976, a hot-ion pulse declared outside the card's regime before
    /// it was run, missed all four marks and is not in the envelope. Each
    /// value only it reached (`fix_0.7.0/envelope_nine.json`) is outside a
    /// bound, and its first load and its own largest states are refused by name.
    #[test]
    fn the_states_and_loads_only_42976_reached_are_refused() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        assert!(25.134e6 > HEATING_POWER_MAX_W && 24.221e6 <= HEATING_POWER_MAX_W);
        assert!(3.1153e6 > OHMIC_POWER_MAX_W && 2.4360e6 <= OHMIC_POWER_MAX_W);
        assert!(1.1991e19 < LINE_DENSITY_MIN_PER_M3 && 1.6580e19 >= LINE_DENSITY_MIN_PER_M3);
        assert!(8.8684e19 > LINE_DENSITY_MAX_PER_M3 && 7.8493e19 <= LINE_DENSITY_MAX_PER_M3);
        assert!(4.1593e6 > PLASMA_CURRENT_MAX_A && 3.8350e6 <= PLASMA_CURRENT_MAX_A);
        assert!(3.3859 < SAFETY_FACTOR_COMPARED_MIN && 3.6631 >= SAFETY_FACTOR_COMPARED_MIN);
        assert!(9.6603e6 > THERMAL_ENERGY_MAX_J && 9.5657e6 <= THERMAL_ENERGY_MAX_J);
        assert!(3.5221e6 > FAST_ION_ENERGY_MAX_J && 1.6181e6 <= FAST_ION_ENERGY_MAX_J);
        assert!(6.2523 > TEMPERATURE_COMPARED_MAX_KEV && 4.2938 <= TEMPERATURE_COMPARED_MAX_KEV);
        assert!(0.13409 < BETA_NORMALISED_COMPARED_MIN && 0.22410 >= BETA_NORMALISED_COMPARED_MIN);
        assert!(8.4534 > CONFINEMENT_TIME_MAX_S && 5.7160 <= CONFINEMENT_TIME_MAX_S);
        // Its first load: 5.60 MW, 3.12 MW of ohmic power, 1.50e19 per cubic
        // metre, 4.13 MA and 3.83 T. Each of three values is refused alone.
        let first = load(5.60e6, 3.115e6, 1.5025e19, 4.128e6, 3.827, 0.501);
        assert_eq!(load_in_card(&tokamak, first), Err(StepReject::OhmicPowerOutsideCard));
        let first = TokamakLoad { ohmic_power_w: 1.0e6, ..first };
        assert_eq!(load_in_card(&tokamak, first), Err(StepReject::PlasmaCurrentOutsideCard));
        let first = TokamakLoad { plasma_current_a: 3.8e6, ..first };
        assert_eq!(load_in_card(&tokamak, first), Err(StepReject::DensityBelowCard));
        // 3.5 MJ of fast ions, and 6.25 keV, under a load the envelope holds.
        let held = load(20.0e6, 1.0e6, 3.0e19, 3.8e6, 4.0, 0.5);
        assert_eq!(load_in_card(&tokamak, held), Ok(()));
        assert_eq!(state_in_card(&tokamak, 4.0e6, 3.5e6, held), Err(StepReject::EnergyOutsideComparison));
        let hot = 6.25 / tokamak.temperature_kev(1.0, 3.0e19);
        assert!(hot < THERMAL_ENERGY_MAX_J);
        assert_eq!(state_in_card(&tokamak, hot, 0.5e6, held), Err(StepReject::TemperatureOutsideComparison));
    }

    /// Over a grid of loads and starts inside the envelope, one step at the
    /// ceiling lands or is refused by a bound of the envelope, never by the
    /// ledger; and runs held at the corners of the loads to the end of the
    /// card do the same.
    #[test]
    fn inside_the_envelope_the_ledger_holds_at_the_ceiling() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let pairs = [(1.65e6, 1.80), (2.5e6, 2.6), (3.8e6, 4.0), (PLASMA_CURRENT_MAX_A, TOROIDAL_FIELD_MAX_T)];
        let mut tried = 0;
        let mut landed = 0;
        let mut worst = 0.0_f64;
        let ledger = |end: Result<(), StepReject>| {
            end == Err(StepReject::FirstLawBreach) || end == Err(StepReject::SpontaneousGeneration)
        };
        for heating in [HEATING_POWER_MIN_W, 10.0e6, HEATING_POWER_MAX_W] {
            for ohmic in [OHMIC_POWER_MIN_W, OHMIC_POWER_MAX_W] {
                for (current, field) in pairs {
                    let greenwald = tokamak.greenwald_density_per_m3(current);
                    let densest = if GREENWALD_FRACTION_MAX * greenwald < LINE_DENSITY_MAX_PER_M3 {
                        GREENWALD_FRACTION_MAX * greenwald
                    } else {
                        LINE_DENSITY_MAX_PER_M3
                    };
                    for density in [LINE_DENSITY_MIN_PER_M3, 0.5 * (LINE_DENSITY_MIN_PER_M3 + densest), densest] {
                        for tritium in [0.0, TRITIUM_FRACTION_TRACE_MAX, TRITIUM_FRACTION_EVEN_MIN, TRITIUM_FRACTION_EVEN_MAX] {
                            let at = load(heating, ohmic, density, current, field, tritium);
                            assert_eq!(load_in_card(&tokamak, at), Ok(()));
                            for kev in [TEMPERATURE_COMPARED_MIN_KEV, 1.5, 2.5, 3.5, TEMPERATURE_COMPARED_MAX_KEV] {
                                let thermal = kev / tokamak.temperature_kev(1.0, density);
                                for fast in [FAST_ION_ENERGY_MIN_J, 0.3e6, 1.5e6, FAST_ION_ENERGY_MAX_J] {
                                    if state_in_card(&tokamak, thermal, fast, at).is_err() {
                                        continue;
                                    }
                                    tried += 1;
                                    let (_, mut state) = start(thermal, fast);
                                    let mut frame = blank_frame();
                                    let end = step(&tokamak, &mut state, &mut frame, at, GATE_DT_S);
                                    assert!(!ledger(end), "{at:?} {thermal} {fast}: {end:?}");
                                    if end.is_ok() {
                                        landed += 1;
                                        worst = worst.max(state.residual_conservation_error);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(tried > 500, "{tried}");
        assert!(landed > 400, "{landed}");
        // The largest found is 4.9e-10, a fortieth of the limit.
        assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT, "{worst}");
        // Runs held at the corners, from a start at 2 keV, to the end of the card.
        let mut runs = 0;
        let mut run_worst = 0.0_f64;
        for heating in [HEATING_POWER_MIN_W, HEATING_POWER_MAX_W] {
            for (current, field) in pairs {
                let greenwald = tokamak.greenwald_density_per_m3(current);
                for density in [LINE_DENSITY_MIN_PER_M3, 0.6 * GREENWALD_FRACTION_MAX * greenwald] {
                    let at = load(heating, OHMIC_POWER_MAX_W, density, current, field, 0.5);
                    if load_in_card(&tokamak, at).is_err() {
                        continue;
                    }
                    let thermal = 2.0 / tokamak.temperature_kev(1.0, density);
                    for fast in [0.1e6, 1.0e6] {
                        if state_in_card(&tokamak, thermal, fast, at).is_err() {
                            continue;
                        }
                        runs += 1;
                        let (_, mut state) = start(thermal, fast);
                        let mut frame = blank_frame();
                        let end = loop {
                            match step(&tokamak, &mut state, &mut frame, at, GATE_DT_S) {
                                Ok(()) => run_worst = run_worst.max(state.residual_conservation_error),
                                Err(reject) => break reject,
                            }
                        };
                        assert!(!ledger(Err(end)), "{at:?} {thermal} {fast}: {end:?} at {}", state.t_s);
                    }
                }
            }
        }
        assert!(runs >= 10, "{runs}");
        // The largest found is 3.9e-10.
        assert!(run_worst < 0.05 * RESIDUAL_CONSERVATION_LIMIT, "{run_worst}");
    }

    /// The clock is the caller's: a run continued from a returned state at
    /// that state's time is the run straight through, bit for bit, and the
    /// end of the card cannot be passed by starting again. A single first
    /// step that ends exactly on the end lands.
    #[test]
    fn a_continued_run_keeps_the_clock_and_cannot_pass_the_end() {
        let (tokamak, mut straight) = inside();
        let mut frame = blank_frame();
        for _ in 0..2000 {
            step(&tokamak, &mut straight, &mut frame, even_mix(), GATE_DT_S).unwrap();
        }
        let (_, mut first) = inside();
        for _ in 0..1000 {
            step(&tokamak, &mut first, &mut frame, even_mix(), GATE_DT_S).unwrap();
        }
        let mut second = TokamakState::try_continue(&tokamak, first.t_s, first.thermal_energy_j, first.fast_ion_energy_j).unwrap();
        for _ in 0..1000 {
            step(&tokamak, &mut second, &mut frame, even_mix(), GATE_DT_S).unwrap();
        }
        assert_eq!(second.t_s.to_bits(), straight.t_s.to_bits());
        assert_eq!(second.thermal_energy_j.to_bits(), straight.thermal_energy_j.to_bits());
        assert_eq!(second.fast_ion_energy_j.to_bits(), straight.fast_ion_energy_j.to_bits());
        assert!(within(first.fusion_energy_j + second.fusion_energy_j, straight.fusion_energy_j, 1.0e-12));
        // Continued at the end, the first step is refused.
        let mut late = TokamakState::try_continue(&tokamak, TIME_MAX_S, 4.0e6, 0.5e6).unwrap();
        let held = late;
        assert_eq!(step(&tokamak, &mut late, &mut frame, even_mix(), FLOOR_DT_S), Err(StepReject::TimeBeyondCard));
        assert_eq!(late, held);
        // A single first step that ends exactly on 5.475 s in decimal lands, at four step sizes.
        for (start_text, dt_text) in [("5.474", "0.001"), ("5.4745", "0.0005"), ("5.4749", "0.0001"), ("5.474999", "0.000001")] {
            let elapsed: f64 = start_text.parse().unwrap();
            let dt_s: f64 = dt_text.parse().unwrap();
            let mut last = TokamakState::try_continue(&tokamak, elapsed, 4.0e6, 0.5e6).unwrap();
            assert_eq!(step(&tokamak, &mut last, &mut frame, even_mix(), dt_s), Ok(()), "{start_text} + {dt_text}");
            assert!((last.t_s - TIME_MAX_S).abs() <= tokamak.time_end_allowance_s(dt_s));
            assert_eq!(step(&tokamak, &mut last, &mut frame, even_mix(), dt_s), Err(StepReject::TimeBeyondCard));
        }
        // One step of 1 ms and a thousand of 1 µs over the same span agree.
        let (_, mut one) = inside();
        step(&tokamak, &mut one, &mut frame, even_mix(), GATE_DT_S).unwrap();
        let (_, mut many) = inside();
        for _ in 0..1000 {
            step(&tokamak, &mut many, &mut frame, even_mix(), FLOOR_DT_S).unwrap();
        }
        assert!(within(one.thermal_energy_j, many.thermal_energy_j, 1.0e-10));
        assert!(within(one.fast_ion_energy_j, many.fast_ion_energy_j, 1.0e-10));
    }

    /// 46 MJ of thermal energy at 1.65e19 per cubic metre is 79 keV, 103 keV
    /// at the centre of the profile: above the fit. No start the published
    /// bounds allow reaches it; the card's own step names it.
    #[test]
    fn a_temperature_above_the_reactivity_fit_is_refused() {
        let (tokamak, mut state) = start(46.0e6, 0.0);
        let held = state;
        let mut frame = blank_frame();
        let load = TokamakLoad { line_density_per_m3: LINE_DENSITY_MIN_PER_M3, ..even_mix() };
        assert!((1.0 + PROFILE_EXPONENT) * tokamak.temperature_kev(46.0e6, LINE_DENSITY_MIN_PER_M3) > 100.0);
        assert!(tokamak.temperature_kev(THERMAL_ENERGY_MAX_J, LINE_DENSITY_MIN_PER_M3) < 17.0);
        assert!((1.0 + PROFILE_EXPONENT) * tokamak.temperature_kev(THERMAL_ENERGY_MAX_J, LINE_DENSITY_MIN_PER_M3) < 22.0);
        assert_eq!(step(&tokamak, &mut state, &mut frame, load, GATE_DT_S), Err(StepReject::TemperatureAboveReactivityFit));
        assert_eq!(state, held);
        proof_intact(&frame);
    }

    /// Full heating at 1.65 MA, 1.80 T and 0.83 of the Greenwald density, from
    /// 2 MJ: the two stores rise until the normalised beta reaches the
    /// envelope's 2.21, and the step that would cross it is refused with the
    /// state on the last landed step. Without the envelope, at 2e19 per cubic
    /// metre from 1 MJ, the stores go on to the Troyon limit and stop there.
    #[test]
    fn the_beta_envelope_and_the_troyon_limit_stop_a_run_and_keep_the_last_state() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let dense = load(HEATING_POWER_MAX_W, OHMIC_POWER_MIN_W, 0.83 * tokamak.greenwald_density_per_m3(1.65e6), 1.65e6, 1.80, 0.0);
        let thin = TokamakLoad { line_density_per_m3: 2.0e19, ..dense };
        for (envelope, load, thermal, expected, limit) in [
            (true, dense, 2.0e6, StepReject::BetaOutsideComparison, BETA_NORMALISED_COMPARED_MAX),
            (false, thin, 1.0e6, StepReject::BetaAboveTroyon, TROYON_BETA_NORMALISED_MAX),
        ] {
            let (_, mut state) = start(thermal, 0.1e6);
            let mut frame = blank_frame();
            let mut steps = 0;
            let end = loop {
                let held = state;
                let frame_t = frame.t;
                let result = if envelope {
                    step(&tokamak, &mut state, &mut frame, load, GATE_DT_S)
                } else {
                    step_structure(&tokamak, &mut state, &mut frame, load, GATE_DT_S)
                };
                match result {
                    Ok(()) => steps += 1,
                    Err(reject) => {
                        assert_eq!(state, held);
                        assert_eq!(frame.t, frame_t);
                        break reject;
                    }
                }
                assert!(steps < 6_000);
            };
            assert_eq!(end, expected);
            assert!(steps > 10);
            assert!(state.beta_normalised <= limit);
            assert!(state.beta_normalised > 0.98 * limit, "{}", state.beta_normalised);
            proof_intact(&frame);
        }
    }

    /// The energy at the Troyon limit is the energy whose normalised beta is 2.8,
    /// and a start above it is refused before it is stepped.
    #[test]
    fn a_start_above_the_troyon_limit_is_refused() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let load = even_mix();
        let limit_j = tokamak.troyon_stored_energy_j(load.plasma_current_a, load.toroidal_field_t);
        assert!(within(
            tokamak.beta_normalised(limit_j, load.plasma_current_a, load.toroidal_field_t),
            TROYON_BETA_NORMALISED_MAX,
            1.0e-14
        ));
        assert!(within(tokamak.largest_stored_energy_j(), 2.253e7, 1.0e-3), "{}", tokamak.largest_stored_energy_j());
        let mut state = TokamakState::try_start(&tokamak, 0.9 * limit_j, 0.11 * limit_j).unwrap();
        let held = state;
        let mut frame = blank_frame();
        assert_eq!(step(&tokamak, &mut state, &mut frame, load, GATE_DT_S), Err(StepReject::BetaAboveTroyon));
        assert_eq!(state, held);
        proof_intact(&frame);
    }

    /// The flag is set above 0.85 of the Greenwald density, which the
    /// compared span of 0.834 does not reach: at that span the frame carries
    /// no flag, and a density above it is refused by name.
    #[test]
    fn the_density_flag_is_not_reached_inside_the_envelope() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let at_span = load(10.0e6, 1.0e6, GREENWALD_FRACTION_MAX * tokamak.greenwald_density_per_m3(2.5e6), 2.5e6, 2.6, 0.5);
        let (_, mut state) = start(5.0e6, 1.0e6);
        let mut frame = blank_frame();
        step(&tokamak, &mut state, &mut frame, at_span, GATE_DT_S).unwrap();
        assert!(!state.near_density_limit);
        assert_eq!(frame.flags, 0);
        assert!(GREENWALD_FRACTION_MAX < super::NEAR_DENSITY_LIMIT_FRACTION);
        let above = TokamakLoad { line_density_per_m3: 0.86 * tokamak.greenwald_density_per_m3(2.5e6), ..at_span };
        assert_eq!(step(&tokamak, &mut state, &mut frame, above, GATE_DT_S), Err(StepReject::GreenwaldFractionAboveComparison));
        proof_intact(&frame);
    }

    #[test]
    fn types_are_copy_and_the_proof_is_not_the_cards() {
        assert_copy::<TokamakJetDte1>();
        assert_copy::<TokamakState>();
        assert_copy::<TokamakLoad>();
        assert_copy::<StepReject>();
        let (tokamak, mut state) = inside();
        let mut frame = blank_frame();
        for _ in 0..50 {
            step(&tokamak, &mut state, &mut frame, even_mix(), GATE_DT_S).unwrap();
            proof_intact(&frame);
        }
    }

    /// Rule 10. An Euler step in place of the Runge–Kutta step is refused by
    /// the ledger at the ceiling, on both loads, in its first step.
    #[test]
    fn explicit_euler_at_the_ceiling_is_refused_by_the_ledger() {
        for (thermal, fast, load) in [(4.0e6, 0.5e6, even_mix()), (3.0e6, 0.5e6, deuterium())] {
            let (tokamak, mut state) = start(thermal, fast);
            let held = state;
            let mut frame = blank_frame();
            let end = step_with(&tokamak, &mut state, &mut frame, load, GATE_DT_S, euler);
            assert!(
                end == Err(StepReject::SpontaneousGeneration) || end == Err(StepReject::FirstLawBreach),
                "{end:?}"
            );
            assert_eq!(state, held);
            assert_eq!(frame.t, -1.0);
            proof_intact(&frame);
            // The real step lands from the same state.
            assert_eq!(step(&tokamak, &mut state, &mut frame, load, GATE_DT_S), Ok(()));
            assert!(state.residual_conservation_error < 1.0e-12);
        }
    }

    /// Rule 10. Energy added to or taken from the result of the real step is
    /// named in the direction it was falsified, and a dropped alpha heating
    /// term is refused.
    #[test]
    fn a_falsified_result_is_refused_both_ways() {
        let (tokamak, state) = inside();
        for (bias, expected) in [
            (1.0e-7, StepReject::SpontaneousGeneration),
            (-1.0e-7, StepReject::FirstLawBreach),
        ] {
            let mut trial = state;
            let mut frame = blank_frame();
            let end = step_with(&tokamak, &mut trial, &mut frame, even_mix(), GATE_DT_S, |t, load, y, dt| {
                let mut out = rk4(t, load, y, dt);
                out[0] *= 1.0 + bias;
                out
            });
            assert_eq!(end, Err(expected));
            assert_eq!(trial, state);
            proof_intact(&frame);
        }
        // The alpha heating left out of the fast-ion equation: 0.2 of 1.4 MW over 1 ms in 4.5 MJ.
        let mut trial = state;
        let mut frame = blank_frame();
        let end = step_with(&tokamak, &mut trial, &mut frame, even_mix(), GATE_DT_S, |t, load, y, dt| {
            let mut out = rk4(t, load, y, dt);
            out[1] -= ALPHA_ENERGY_SHARE * (out[2] - y[2]);
            out
        });
        assert_eq!(end, Err(StepReject::FirstLawBreach));
        assert_eq!(trial, state);
    }

    #[test]
    fn the_step_ceiling_and_floor_are_present_and_hold_the_ledger() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        assert_eq!(tokamak.gate_dt_s(), Some(GATE_DT_S));
        assert_eq!(tokamak.floor_dt_s(), Some(FLOOR_DT_S));
        // The ceiling is under an eighteenth of the shortest slowing-down time
        // inside the declared range, 18.0 ms, and under a fortieth of the shortest
        // loss time, 0.31 of the smallest confinement time, 46.5 ms.
        let shortest = tokamak.slowing_time_coeff_s * TEMPERATURE_COMPARED_MIN_KEV.sqrt()
            / (LINE_DENSITY_MAX_PER_M3 / 1.0e19);
        assert!(within(shortest, 0.01806, 5.0e-3), "{shortest}");
        assert!(GATE_DT_S <= shortest / 18.0, "{shortest}");
        assert!(GATE_DT_S < IPB98_EXP_LOSS * CONFINEMENT_TIME_MIN_S / 40.0);
        // A step at the floor moves both stores and the clock at the end of the card.
        let (_, mut state) = inside();
        let held = state;
        let mut frame = blank_frame();
        step(&tokamak, &mut state, &mut frame, even_mix(), FLOOR_DT_S).unwrap();
        assert!(state.thermal_energy_j != held.thermal_energy_j);
        assert!(state.fast_ion_energy_j != held.fast_ion_energy_j);
        assert!(TIME_MAX_S - FLOOR_DT_S < TIME_MAX_S);
        assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
    }

    /// Halving the step changes the state after a quarter second by the
    /// fourth-order ratio.
    #[test]
    fn halving_the_step_changes_the_state_by_the_fourth_order_ratio() {
        let run = |dt_s: f64, steps: u32| {
            let (tokamak, mut state) = inside();
            let mut frame = blank_frame();
            for _ in 0..steps {
                step(&tokamak, &mut state, &mut frame, even_mix(), dt_s).unwrap();
            }
            state.thermal_energy_j
        };
        let coarse = run(1.0e-3, 256);
        let middle = run(0.5e-3, 512);
        let fine = run(0.25e-3, 1024);
        let ratio = (coarse - middle) / (middle - fine);
        assert!(ratio > 12.0 && ratio < 20.0, "{ratio}");
        assert!(within(coarse, fine, 1.0e-9));
    }
}
