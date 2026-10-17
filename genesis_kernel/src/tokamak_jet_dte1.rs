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
//! the load             P_aux ≥ 0, P_ohm ≥ 0, Ip > 0, Bt > 0, n_el > 0, 0 ≤ f_T ≤ 1
//! Greenwald density    n_el ≤ 1e20 · Ip[MA] / (π a²)                         m^-3
//! safety factor        q = 5 a² Bt / (R · Ip[MA]) · (1 + κ²)/2 · (1.17 − 0.65 ε)/(1 − ε²)² ≥ 2
//! the stores           W_th > 0, W_f ≥ 0
//! reactivity fit       0.2 keV ≤ T  and  (1 + α) · T ≤ 100 keV
//! Troyon beta          β_N = β[%] · a · Bt / Ip[MA] ≤ 2.8,  β = 2 μ0 · (2/3)(W_th + W_f)/V / Bt²
//! the clock            t ≥ 0, and a step moves it
//! the step             dt ≤ min(τ_f / 18, 0.31 · τ_E / 46, 2 · τ_f · W_th / W_f / 48),  τ_E = W_th / P_loss
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
//! The compared envelope. The card was compared on the nine pulses of the
//! research record that are ELMy H-modes, the eight fitted and the held-back
//! 42982, run through the card at 1 ms by the protocol's rule
//! (`analysis/pulses.py`). Each bound of the envelope is the smallest or the
//! largest value over the loads of those nine runs, or over every state a
//! step of them started from or landed on under that step's load, rounded
//! outward in the third figure
//! (`evidence/research/tokamak_jet_dte1/fix_0.7.0/envelope_nine.py`). The
//! loads: heating power 0.124 to 24.3 MW, ohmic power 0.173 to 2.44 MW,
//! line-averaged density 1.65e19 to 7.85e19 m^-3 and at most 0.834 of the
//! Greenwald density, plasma current 1.65 to 3.84 MA, toroidal field 1.80 to
//! 4.04 T, edge safety factor 3.66 to 4.14 (the card's formula; JET's own
//! equilibrium value is about a fifth higher). The tritium fraction of the
//! fuel 0 to 0.034, deuterium and the trace-tritium pulse 43134, or 0.451 to
//! 0.504, the held-back pulse 42982 and the fitted pulse 42762 at an even
//! mix (0.503, rounded up by 0.001 to hold the value the protocol's run
//! computes). The state, at the start and after every step: thermal energy
//! 0.462 to 9.57 MJ, fast-ion energy 5.59 kJ to 1.62 MJ, temperature 0.777
//! to 4.30 keV, normalised beta 0.224 to 2.21, confinement time
//! `W_th / P_loss` 0.150 to 5.72 s. The longest compared heated window is
//! 5.475 s, pulse 42982.
//!
//! How it did there, and past it. On the held-back 42982 the thermal energy
//! error was 7.9 percent and the fast-ion 22.5 percent of peak, and the
//! fusion energy was 0.743 of the measured. The fusion power was compared at
//! tritium fractions 0 to 0.034 and 0.451 to 0.504; between them nothing was
//! compared, and at 0.75 to 0.93 three training pulses showed it wrong by 33
//! to 65 percent of its peak. The held-back hot-ion pulse 42976, declared
//! outside the card's regime before it was run, reached what no other pulse
//! did (25.1 MW of heating, 3.1 MW of ohmic power, 4.16 MA, densities below
//! 1.65e19 and above 7.85e19 m^-3, a safety factor of 3.39, fast-ion energy
//! to 3.5 MJ, 6.3 keV, confinement times to 8.5 s) and missed all four
//! marks. Outside the envelope the card computes the same equations, with
//! nothing compared there, until one of the named limits above stops it.
//! Lifting the envelope opens loads below the L-H power threshold, where the
//! plasma is in L-mode and the H-mode scaling the card uses was not
//! compared; the card has no limit there, and the compared runs start their
//! heated window at 0.124 MW.
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
//! The clock. It is a running sum, `t += dt` in 64-bit arithmetic, started
//! from the time the caller states, and the card has no end: a clock below
//! zero is [`StepReject::NegativeTime`], and a step too small to move the
//! clock at its time, which a step of 1e-6 s first is at 2^34 s, is
//! [`StepReject::StepBelowClockResolution`].
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, the two energies, the fusion energy, the
//! fusion power, the temperature, the normalised beta, the loss power and
//! the conservation residual into [`LastStateFrame64`]. The 16-byte frame
//! proof is not touched.
//!
//! The step ceilings. Three times set how fast the stores move: the
//! slowing-down time `τ_f`; the loss time `0.31 · τ_E`, in which the loss
//! power changes the thermal energy; and the coupling time
//! `2 · τ_f · W_th / W_f`, in which a change of the thermal energy changes the
//! power the fast ions give it, since `τ_f` grows as the square root of the
//! temperature. [`TokamakJetDte1::gate_dt_s`] is the largest step the card
//! stands behind anywhere, [`GATE_DT_S`], 1 ms, and the step itself holds
//! `dt` to the ceiling of the state, [`TokamakJetDte1::state_ceiling_dt_s`]:
//! an eighteenth of the first, a forty-sixth of the second and a
//! forty-eighth of the third, whichever is smallest, at the state the step
//! starts from and where its trial step lands, refused above it as
//! [`StepReject::DtAboveStateCeiling`] with nothing changed. Each fraction
//! is the largest whole one at which the 1 ms step still fits at the fastest
//! corner of the compared envelope: there `τ_f` is at least 18.06 ms (0.777
//! keV at 7.85e19 m^-3), `0.31 · τ_E` at least 46.5 ms (a confinement time
//! of 0.150 s) and the coupling time at least 48.1 ms (0.777 keV with 1.62 MJ
//! of fast ions, at any density). Inside the envelope the state ceiling is
//! therefore at least 1.0018 ms and never binds, and every compared run lands
//! as before, bit for bit; over the nine compared runs at 1 ms the three
//! times are at least 28.3 ms, 46.6 ms and 0.30 s. Past the envelope a
//! denser or colder plasma, a shorter confinement time or a fast-ion store
//! large against the thermal one shortens the ceiling with the state, and
//! each step stands where the 1 ms step stood at that corner. Run so, at
//! the state ceiling or 1 ms, whichever is shorter, past every old wall of
//! the envelope the ledger closed with a residual of at most 1.5e-9. The
//! strongest start attacked, eight times the thermal energy in the fast ions
//! at 2e20 m^-3 and 1 keV, passes the ledger's limit after about a hundred
//! steps at the state ceiling and is refused as
//! [`StepReject::FirstLawBreach`] with the state kept: the truncation of the
//! step, which the 1 ms step would carry at a corner of the envelope no
//! compared run reached; at a quarter of the ceiling it closes.
//! In 0.6.0 the ceiling rested on the slowing-down time alone, and states
//! with a confinement time near 37 ms were refused as FirstLawBreach
//! although no energy was lost; the loss time now holds that step.
//! [`TokamakJetDte1::floor_dt_s`] is [`FLOOR_DT_S`], 1e-6 s: at 1 MW it moves
//! a store of 20 MJ by one joule, which 64-bit numbers resolve to nine
//! figures. The public step applies neither the static ceiling nor the
//! floor; the gateway refuses a step outside them.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;
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
/// Smallest heating power compared, W: the smallest of the nine compared runs of the card's regime, 0.1248 MW, rounded down.
pub const HEATING_POWER_COMPARED_MIN_W: f64 = 0.124e6;
/// Largest heating power (neutral beams and ion cyclotron waves) compared, W: 24.22 MW, rounded up. The held-back hot-ion pulse 42976 reached 25.1 MW.
pub const HEATING_POWER_COMPARED_MAX_W: f64 = 24.3e6;
/// Smallest ohmic power compared, W: 0.1738 MW, rounded down.
pub const OHMIC_POWER_COMPARED_MIN_W: f64 = 0.173e6;
/// Largest ohmic power compared, W: 2.436 MW, rounded up.
pub const OHMIC_POWER_COMPARED_MAX_W: f64 = 2.44e6;
/// Smallest line-averaged electron density compared, m^-3: 1.658e19, rounded down.
pub const LINE_DENSITY_COMPARED_MIN_PER_M3: f64 = 1.65e19;
/// Largest line-averaged electron density compared, m^-3: 7.849e19, rounded up.
pub const LINE_DENSITY_COMPARED_MAX_PER_M3: f64 = 7.85e19;
/// Largest fraction of the Greenwald density compared: 0.8331, rounded up.
pub const GREENWALD_FRACTION_COMPARED_MAX: f64 = 0.834;
/// Smallest plasma current compared, A: 1.652 MA, rounded down.
pub const PLASMA_CURRENT_COMPARED_MIN_A: f64 = 1.65e6;
/// Largest plasma current compared, A: 3.835 MA, rounded up. 42976 reached 4.16 MA.
pub const PLASMA_CURRENT_COMPARED_MAX_A: f64 = 3.84e6;
/// Smallest toroidal field compared, T: 1.807 T, rounded down.
pub const TOROIDAL_FIELD_COMPARED_MIN_T: f64 = 1.80;
/// Largest toroidal field compared, T: 4.039 T, rounded up.
pub const TOROIDAL_FIELD_COMPARED_MAX_T: f64 = 4.04;
/// Smallest edge safety factor compared: 3.663, rounded down. 42976 reached 3.39.
pub const SAFETY_FACTOR_COMPARED_MIN: f64 = 3.66;
/// Largest edge safety factor compared: 4.136, rounded up.
pub const SAFETY_FACTOR_COMPARED_MAX: f64 = 4.14;
/// Largest tritium fraction of the lower compared span: 43134, the one compared pulse with tritium below an even mix.
pub const TRITIUM_FRACTION_TRACE_MAX: f64 = 0.034;
/// Smallest tritium fraction of the upper compared span: the held-back pulse 42982.
pub const TRITIUM_FRACTION_EVEN_MIN: f64 = 0.451;
/// Largest tritium fraction of the upper compared span: the fitted pulse 42762 at 0.503, rounded up by 0.001 to hold the value the protocol's run computes.
pub const TRITIUM_FRACTION_EVEN_MAX: f64 = 0.504;
/// Smallest thermal energy compared, J: 0.4624 MJ, rounded down.
pub const THERMAL_ENERGY_COMPARED_MIN_J: f64 = 0.462e6;
/// Largest thermal energy compared, J: 9.566 MJ, rounded up.
pub const THERMAL_ENERGY_COMPARED_MAX_J: f64 = 9.57e6;
/// Smallest fast-ion energy compared, J: 5.592 kJ, rounded down.
pub const FAST_ION_ENERGY_COMPARED_MIN_J: f64 = 5.59e3;
/// Largest fast-ion energy compared, J: 1.618 MJ, rounded up. 42976 reached 3.5 MJ.
pub const FAST_ION_ENERGY_COMPARED_MAX_J: f64 = 1.62e6;
/// Smallest plasma temperature compared, keV: 0.7776 keV at the start of a step of 43134, rounded down.
pub const TEMPERATURE_COMPARED_MIN_KEV: f64 = 0.777;
/// Largest plasma temperature compared, keV: 4.294 keV, rounded up. 42976 reached 6.3 keV.
pub const TEMPERATURE_COMPARED_MAX_KEV: f64 = 4.30;
/// Smallest normalised beta compared: 0.2241, rounded down.
pub const BETA_NORMALISED_COMPARED_MIN: f64 = 0.224;
/// Largest normalised beta compared: 2.202, rounded up.
pub const BETA_NORMALISED_COMPARED_MAX: f64 = 2.21;
/// Smallest confinement time `W_th / P_loss` compared, s: 0.1504 s, rounded down.
pub const CONFINEMENT_TIME_COMPARED_MIN_S: f64 = 0.150;
/// Largest confinement time `W_th / P_loss` compared, s: 5.716 s, rounded up. 42976 reached 8.5 s.
pub const CONFINEMENT_TIME_COMPARED_MAX_S: f64 = 5.72;
/// The longest compared heated window, s: pulse 42982. The card does not end there.
pub const TIME_COMPARED_MAX_S: f64 = 5.475;
/// `|defect| / max(W_th + W_f, anchor)` must stay at or under this. Set by
/// measurement (`analysis/out/06_ledger_on_training.log`): over the eight
/// training pulses run at the ceiling the largest value found is 5.1e-10
/// (pulse 42997, 5425 steps), and the limit is 39 times it. An Euler step in
/// place of the first Runge–Kutta step of those runs gives 4.9e-7 or more.
/// Inside the compared envelope, over a grid of 2,784 starts and loads, one
/// Runge–Kutta step at the ceiling gives at most 4.9e-10, and runs held at the
/// corners of the loads for 5.475 s at most 3.9e-10.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.0e-8;
/// Largest step the card stands behind anywhere, s. Inside the compared
/// envelope it is the whole ceiling; past it the step also holds `dt` to
/// [`TokamakJetDte1::state_ceiling_dt_s`], which follows the state.
pub const GATE_DT_S: f64 = 1.0e-3;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-6;
/// Steps in one slowing-down time at the state ceiling: the slowing-down time
/// is at least 18.06 ms inside the compared envelope (0.777 keV at 7.85e19
/// m^-3), so an eighteenth of it is at least 1.003 ms.
pub const STATE_CEILING_SLOWING_STEPS: f64 = 18.0;
/// Steps in one loss time `0.31 · τ_E` at the state ceiling: the loss time is
/// at least 46.5 ms inside the compared envelope, so a forty-sixth of it is
/// at least 1.011 ms.
pub const STATE_CEILING_LOSS_STEPS: f64 = 46.0;
/// Steps in one coupling time `2 · τ_f · W_th / W_f` at the state ceiling:
/// the time in which a change of the thermal energy changes the power the
/// fast ions give it, since `τ_f` grows as the square root of the
/// temperature. It is at least 48.1 ms inside the compared envelope (0.777 keV
/// and 1.62 MJ of fast ions, at any density), so a forty-eighth of it is at
/// least 1.0018 ms.
pub const STATE_CEILING_COUPLING_STEPS: f64 = 48.0;
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
    /// The heating power is below zero.
    HeatingPowerNegative,
    /// The ohmic power is below zero.
    OhmicPowerNegative,
    /// The plasma current is zero or less: the confinement law takes its logarithm and the safety factor divides by it.
    PlasmaCurrentNotPositive,
    /// The toroidal field is zero or less: the confinement law takes its logarithm and the normalised beta divides by it.
    ToroidalFieldNotPositive,
    /// The tritium fraction of the fuel is below 0 or above 1, which no fraction of the fuel atoms can be.
    FuelMixOutsideCard,
    /// The line-averaged density is zero or less: the slowing-down time divides by it.
    DensityNotPositive,
    /// The line-averaged density is above the Greenwald density of the plasma current.
    DensityAboveGreenwald,
    /// The edge safety factor of the current and the field is below 2.
    SafetyFactorBelowLimit,
    /// The thermal energy is zero or negative, or the fast-ion energy is negative.
    EnergyOutsideCard,
    /// The plasma temperature is below 0.2 keV, the floor of the reactivity fit.
    TemperatureBelowReactivityFit,
    /// The temperature at the centre of the profile is above 100 keV, the ceiling of the reactivity fit.
    TemperatureAboveReactivityFit,
    /// The normalised beta of the two stores is above the Troyon limit of 2.8, at the start of the step or where the trial step lands.
    BetaAboveTroyon,
    /// The clock, the time since the start of the heated window, is negative.
    NegativeTime,
    /// The step is too small to move the clock at its time in 64-bit arithmetic.
    StepBelowClockResolution,
    /// The step is longer than the ceiling of the state it starts from or where its trial step lands: the smallest of an eighteenth of the slowing-down time, a forty-sixth of 0.31 times the confinement time and a forty-eighth of the coupling time of the fast ions and the thermal plasma.
    DtAboveStateCeiling,
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
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`GATE_DT_S`].
    pub gate_dt_s: f64,
    /// Recorded copy of [`FLOOR_DT_S`].
    pub floor_dt_s: f64,
    /// Recorded copy of [`STATE_CEILING_SLOWING_STEPS`].
    pub state_ceiling_slowing_steps: f64,
    /// Recorded copy of [`STATE_CEILING_LOSS_STEPS`].
    pub state_ceiling_loss_steps: f64,
    /// Recorded copy of [`STATE_CEILING_COUPLING_STEPS`].
    pub state_ceiling_coupling_steps: f64,
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
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_dt_s: GATE_DT_S,
            floor_dt_s: FLOOR_DT_S,
            state_ceiling_slowing_steps: STATE_CEILING_SLOWING_STEPS,
            state_ceiling_loss_steps: STATE_CEILING_LOSS_STEPS,
            state_ceiling_coupling_steps: STATE_CEILING_COUPLING_STEPS,
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
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.gate_dt_s.to_bits() == GATE_DT_S.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
            && self.state_ceiling_slowing_steps.to_bits() == STATE_CEILING_SLOWING_STEPS.to_bits()
            && self.state_ceiling_loss_steps.to_bits() == STATE_CEILING_LOSS_STEPS.to_bits()
            && self.state_ceiling_coupling_steps.to_bits() == STATE_CEILING_COUPLING_STEPS.to_bits()
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

    /// Largest step the card stands behind anywhere: [`GATE_DT_S`]. Not
    /// applied by [`step`]. Past the compared envelope the step itself holds
    /// `dt` to the smaller [`TokamakJetDte1::state_ceiling_dt_s`].
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

    /// The step ceiling of a state under a load, s: the smallest of an
    /// eighteenth of the slowing-down time `τ_f`, a forty-sixth of the loss time
    /// `0.31 · τ_E` and a forty-eighth of the coupling time `2 · τ_f · W_th / W_f`
    /// (none with no fast ions). [`step`] refuses a longer step as
    /// [`StepReject::DtAboveStateCeiling`], at the state it starts from and
    /// where its trial step lands. Inside the compared envelope it is at
    /// least 1.0018 ms, above [`GATE_DT_S`].
    #[must_use]
    pub fn state_ceiling_dt_s(&self, thermal_energy_j: f64, fast_ion_energy_j: f64, load: TokamakLoad) -> f64 {
        let d = derived(self, load, &[thermal_energy_j, fast_ion_energy_j, 0.0]);
        let mut ceiling = d.slowing_time_s / self.state_ceiling_slowing_steps;
        let loss = IPB98_EXP_LOSS * (thermal_energy_j / d.loss_power_w) / self.state_ceiling_loss_steps;
        if loss < ceiling {
            ceiling = loss;
        }
        if fast_ion_energy_j > 0.0 {
            let coupling = 2.0 * d.slowing_time_s * thermal_energy_j / fast_ion_energy_j
                / self.state_ceiling_coupling_steps;
            if coupling < ceiling {
                ceiling = coupling;
            }
        }
        ceiling
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
    /// `t_s` here, so the clock counts from the window's start.
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
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    tokamak: &TokamakJetDte1,
    state: &mut TokamakState,
    frame: &mut LastStateFrame64,
    load: TokamakLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(tokamak, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    tokamak: &TokamakJetDte1,
    state: &mut TokamakState,
    frame: &mut LastStateFrame64,
    load: TokamakLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(tokamak, state, load, dt_s)?;
    let advanced = rk4(tokamak, load, &state_vector(state), dt_s);
    commit(tokamak, state, frame, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut TokamakState) {
    state.t_s = normal_or_zero(state.t_s);
    state.thermal_energy_j = normal_or_zero(state.thermal_energy_j);
    state.fast_ion_energy_j = normal_or_zero(state.fast_ion_energy_j);
    state.fusion_energy_j = normal_or_zero(state.fusion_energy_j);
    state.temperature_kev = normal_or_zero(state.temperature_kev);
    state.loss_power_w = normal_or_zero(state.loss_power_w);
    state.confinement_time_s = normal_or_zero(state.confinement_time_s);
    state.slowing_time_s = normal_or_zero(state.slowing_time_s);
    state.fusion_power_w = normal_or_zero(state.fusion_power_w);
    state.fusion_power_thermal_w = normal_or_zero(state.fusion_power_thermal_w);
    state.beta_normalised = normal_or_zero(state.beta_normalised);
    state.greenwald_fraction = normal_or_zero(state.greenwald_fraction);
    state.safety_factor = normal_or_zero(state.safety_factor);
    state.energy_anchor_j = normal_or_zero(state.energy_anchor_j);
    state.energy_supplied_j = normal_or_zero(state.energy_supplied_j);
    state.energy_lost_j = normal_or_zero(state.energy_lost_j);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
}

/// The load held to the card: each field to what it can physically be, the
/// density to the Greenwald density and the current and the field to the
/// safety factor. The step tests this first; a caller may test a load with
/// it before a run.
pub fn load_in_card(tokamak: &TokamakJetDte1, load: TokamakLoad) -> Result<(), StepReject> {
    if load.heating_power_w < 0.0 {
        return Err(StepReject::HeatingPowerNegative);
    }
    if load.ohmic_power_w < 0.0 {
        return Err(StepReject::OhmicPowerNegative);
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
    if !(load.line_density_per_m3 > 0.0) {
        return Err(StepReject::DensityNotPositive);
    }
    if load.line_density_per_m3 > tokamak.greenwald_density_per_m3(load.plasma_current_a) {
        return Err(StepReject::DensityAboveGreenwald);
    }
    if tokamak.safety_factor(load.plasma_current_a, load.toroidal_field_t) < tokamak.safety_factor_min {
        return Err(StepReject::SafetyFactorBelowLimit);
    }
    Ok(())
}

/// A thermal and a fast-ion energy held to the card under a load: the
/// energies positive, the temperature in the reactivity fit and the two
/// stores under the Troyon limit. The step tests this at its start and
/// where its trial step lands; a caller may test a start with it.
pub fn state_in_card(
    tokamak: &TokamakJetDte1,
    thermal_energy_j: f64,
    fast_ion_energy_j: f64,
    load: TokamakLoad,
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
    Ok(())
}

/// A step of `dt_s` from a state under a load is at or under the state's
/// ceiling, [`TokamakJetDte1::state_ceiling_dt_s`].
fn under_state_ceiling(
    tokamak: &TokamakJetDte1,
    thermal_energy_j: f64,
    fast_ion_energy_j: f64,
    load: TokamakLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if dt_s <= tokamak.state_ceiling_dt_s(thermal_energy_j, fast_ion_energy_j, load) {
        Ok(())
    } else {
        Err(StepReject::DtAboveStateCeiling)
    }
}

fn validate(
    tokamak: &TokamakJetDte1,
    state: &TokamakState,
    load: TokamakLoad,
    dt_s: f64,
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
    state_in_card(tokamak, state.thermal_energy_j, state.fast_ion_energy_j, load)?;
    if !(state.t_s >= 0.0) {
        return Err(StepReject::NegativeTime);
    }
    if !(state.t_s + dt_s > state.t_s) {
        return Err(StepReject::StepBelowClockResolution);
    }
    under_state_ceiling(tokamak, state.thermal_energy_j, state.fast_ion_energy_j, load, dt_s)
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
) -> Result<(), StepReject> {
    if !advanced[0].is_finite() || !advanced[1].is_finite() || !advanced[2].is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    state_in_card(tokamak, advanced[0], advanced[1], load)?;
    under_state_ceiling(tokamak, advanced[0], advanced[1], load, dt_s)?;
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
    use std::matches;

    use super::{
        ALPHA_ENERGY_SHARE, BETA_NORMALISED_COMPARED_MAX, CONFINEMENT_TIME_COMPARED_MAX_S,
        CONFINEMENT_TIME_COMPARED_MIN_S, FAST_ION_ENERGY_COMPARED_MAX_J, FAST_ION_ENERGY_COMPARED_MIN_J,
        FLOOR_DT_S, GATE_DT_S, GREENWALD_FRACTION_COMPARED_MAX, HEATING_POWER_COMPARED_MAX_W,
        HEATING_POWER_COMPARED_MIN_W, IPB98_EXP_LOSS, LINE_DENSITY_COMPARED_MAX_PER_M3,
        LINE_DENSITY_COMPARED_MIN_PER_M3, NEAR_DENSITY_LIMIT_FRACTION, OHMIC_POWER_COMPARED_MAX_W,
        OHMIC_POWER_COMPARED_MIN_W, PLASMA_CURRENT_COMPARED_MAX_A, PLASMA_CURRENT_COMPARED_MIN_A,
        PROFILE_EXPONENT, REACTIVITY_COEFF, RESIDUAL_CONSERVATION_LIMIT, SAFETY_FACTOR_COMPARED_MAX,
        SAFETY_FACTOR_COMPARED_MIN, STATE_CEILING_COUPLING_STEPS, STATE_CEILING_LOSS_STEPS,
        STATE_CEILING_SLOWING_STEPS, StepReject, TEMPERATURE_COMPARED_MAX_KEV,
        TEMPERATURE_COMPARED_MIN_KEV, THERMAL_ENERGY_COMPARED_MAX_J, THERMAL_ENERGY_COMPARED_MIN_J,
        TIME_COMPARED_MAX_S, TOROIDAL_FIELD_COMPARED_MAX_T, TOROIDAL_FIELD_COMPARED_MIN_T,
        TRITIUM_FRACTION_EVEN_MAX, TRITIUM_FRACTION_EVEN_MIN, TRITIUM_FRACTION_TRACE_MAX,
        TROYON_BETA_NORMALISED_MAX, TokamakJetDte1, TokamakLoad, TokamakState, advance, commit,
        load_in_card, profile_reactivity_m3_per_s, rates, reactivity_dt_m3_per_s, rk4, state_in_card,
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

    fn above(x: f64) -> f64 {
        f64::from_bits(x.to_bits() + 1)
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

    fn start(thermal_j: f64, fast_j: f64) -> (TokamakJetDte1, TokamakState) {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let state = TokamakState::try_start(&tokamak, thermal_j, fast_j).unwrap();
        (tokamak, state)
    }

    /// The thermal energy of a temperature at a density, J.
    fn thermal_at(kev: f64, density: f64) -> f64 {
        kev / TokamakJetDte1::elmy_h_mode().temperature_kev(1.0, density)
    }

    /// The real `validate`, a chosen integrator, the real `commit`.
    fn step_with(
        tokamak: &TokamakJetDte1,
        state: &mut TokamakState,
        frame: &mut LastStateFrame64,
        load: TokamakLoad,
        dt_s: f64,
        integrate: impl FnOnce(&TokamakJetDte1, TokamakLoad, &[f64; 3], f64) -> [f64; 3],
    ) -> Result<(), StepReject> {
        validate(tokamak, state, load, dt_s)?;
        let advanced = integrate(tokamak, load, &state_vector(state), dt_s);
        commit(tokamak, state, frame, load, dt_s, advanced)
    }

    /// A start inside the compared envelope under [`even_mix`]:
    /// 1.9 keV, a normalised beta of 0.57, a confinement time of 2.0 s.
    fn inside() -> (TokamakJetDte1, TokamakState) {
        start(4.0e6, 0.5e6)
    }

    fn euler(tokamak: &TokamakJetDte1, load: TokamakLoad, y: &[f64; 3], dt_s: f64) -> [f64; 3] {
        advance(y, &rates(tokamak, load, y).0, dt_s)
    }

    fn ledger(end: Result<(), StepReject>) -> bool {
        end == Err(StepReject::FirstLawBreach) || end == Err(StepReject::SpontaneousGeneration)
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
        assert_eq!((STATE_CEILING_SLOWING_STEPS, STATE_CEILING_LOSS_STEPS, STATE_CEILING_COUPLING_STEPS), (18.0, 46.0, 48.0));
        // Every recorded copy is held to its constant, one at a time.
        let alter: [fn(&mut TokamakJetDte1); 18] = [
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
            |t| t.residual_conservation_limit = 1.0,
            |t| t.gate_dt_s = 1.0,
            |t| t.floor_dt_s = 1.0e-12,
            |t| t.state_ceiling_slowing_steps = 1.0,
            |t| t.state_ceiling_loss_steps = 1.0,
            |t| t.state_ceiling_coupling_steps = 1.0,
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
    /// run of the same structure (`analysis/out/05_reference_run.log`). The
    /// start's confinement time, 9.3 s, is above the compared 5.72 s; the
    /// card computes it with the same equations.
    #[test]
    fn a_second_at_the_ceiling_agrees_with_the_forecast_structure() {
        let (tokamak, mut state) = start(2.0e6, 0.5e6);
        let mut frame = blank_frame();
        let mut worst = 0.0_f64;
        assert!(tokamak.confinement_time_s(2.0e6, even_mix()) > CONFINEMENT_TIME_COMPARED_MAX_S);
        for _ in 0..1000 {
            step(&tokamak, &mut state, &mut frame, even_mix(), GATE_DT_S).unwrap();
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
            step(&tokamak, &mut state, &mut frame, deuterium(), GATE_DT_S).unwrap();
        }
        assert!(within(state.thermal_energy_j, 3492366.1133224079, 1.0e-11));
        assert!(within(state.fast_ion_energy_j, 631451.60550995078, 1.0e-11));
        assert_eq!(state.fusion_energy_j, 0.0);
        assert_eq!(state.fusion_power_w, 0.0);
    }

    /// Held for twelve seconds, past the longest compared window, the plasma
    /// settles where the loss power equals the power supplied, and the
    /// confinement time there is the scaling's.
    #[test]
    fn a_held_load_settles_at_the_power_balance() {
        let (tokamak, mut state) = start(1.0e6, 0.0);
        let mut frame = blank_frame();
        for _ in 0..12_000 {
            step(&tokamak, &mut state, &mut frame, deuterium(), GATE_DT_S).unwrap();
        }
        assert!(within(state.t_s, 12.0, 1.0e-12));
        assert!(within(state.loss_power_w, 11.0e6, 1.0e-6), "{}", state.loss_power_w);
        assert!(within(
            state.fast_ion_energy_j / state.slowing_time_s,
            10.0e6,
            1.0e-6
        ));
        assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
    }

    /// Each limit of the card refused by name before the step, with state and
    /// frame kept; the limits of the load at their edges.
    #[test]
    fn every_refusal_is_named_and_rolls_back_state_and_frame() {
        let (tokamak, state) = inside();
        let base = even_mix();
        let low_current = TokamakLoad { plasma_current_a: 1.65e6, toroidal_field_t: 1.80, ..base };
        let cases: [(TokamakLoad, f64, StepReject); 24] = [
            (TokamakLoad { heating_power_w: f64::NAN, ..base }, GATE_DT_S, StepReject::NonFiniteInput),
            (TokamakLoad { tritium_fraction: f64::INFINITY, ..base }, GATE_DT_S, StepReject::NonFiniteInput),
            (TokamakLoad { ohmic_power_w: f64::NEG_INFINITY, ..base }, GATE_DT_S, StepReject::NonFiniteInput),
            (TokamakLoad { tritium_fraction: f64::NAN, ..base }, GATE_DT_S, StepReject::NonFiniteInput),
            (base, f64::NAN, StepReject::NonFiniteInput),
            (base, 0.0, StepReject::NonPositiveDt),
            (base, -1.0e-3, StepReject::NonPositiveDt),
            (TokamakLoad { heating_power_w: -1.0e-9, ..base }, GATE_DT_S, StepReject::HeatingPowerNegative),
            (TokamakLoad { heating_power_w: -20.0e6, ..base }, GATE_DT_S, StepReject::HeatingPowerNegative),
            (TokamakLoad { ohmic_power_w: -1.0e-9, ..base }, GATE_DT_S, StepReject::OhmicPowerNegative),
            (TokamakLoad { plasma_current_a: 0.0, ..base }, GATE_DT_S, StepReject::PlasmaCurrentNotPositive),
            (TokamakLoad { plasma_current_a: -3.8e6, ..base }, GATE_DT_S, StepReject::PlasmaCurrentNotPositive),
            (TokamakLoad { toroidal_field_t: 0.0, ..base }, GATE_DT_S, StepReject::ToroidalFieldNotPositive),
            (TokamakLoad { toroidal_field_t: -4.0, ..base }, GATE_DT_S, StepReject::ToroidalFieldNotPositive),
            (TokamakLoad { tritium_fraction: -1.0e-9, ..base }, GATE_DT_S, StepReject::FuelMixOutsideCard),
            (TokamakLoad { tritium_fraction: above(1.0), ..base }, GATE_DT_S, StepReject::FuelMixOutsideCard),
            (TokamakLoad { tritium_fraction: 1.5, ..base }, GATE_DT_S, StepReject::FuelMixOutsideCard),
            (TokamakLoad { line_density_per_m3: 0.0, ..base }, GATE_DT_S, StepReject::DensityNotPositive),
            (TokamakLoad { line_density_per_m3: -6.0e19, ..base }, GATE_DT_S, StepReject::DensityNotPositive),
            (TokamakLoad { line_density_per_m3: tokamak.greenwald_density_per_m3(1.65e6) * 1.000001, ..low_current }, GATE_DT_S, StepReject::DensityAboveGreenwald),
            (TokamakLoad { line_density_per_m3: above(tokamak.greenwald_density_per_m3(3.8e6)), ..base }, GATE_DT_S, StepReject::DensityAboveGreenwald),
            // 3.84 MA at 1.80 T: an edge safety factor of 1.73.
            (TokamakLoad { plasma_current_a: 3.84e6, toroidal_field_t: 1.80, ..base }, GATE_DT_S, StepReject::SafetyFactorBelowLimit),
            // 3.8 MA at 2.0 T: 1.94.
            (TokamakLoad { toroidal_field_t: 2.0, ..base }, GATE_DT_S, StepReject::SafetyFactorBelowLimit),
            // 2 ms is above the ceiling of this state, 2.04 ms.
            (base, 2.1e-3, StepReject::DtAboveStateCeiling),
        ];
        for (index, (load, dt_s, expected)) in cases.iter().enumerate() {
            let mut trial = state;
            let held = trial;
            let mut frame = blank_frame();
            assert_eq!(step(&tokamak, &mut trial, &mut frame, *load, *dt_s), Err(*expected), "case {index}");
            assert_eq!(trial, held, "case {index}");
            assert_eq!(frame.to_bytes(), blank_frame().to_bytes(), "case {index}");
            proof_intact(&frame);
            if !matches!(*expected, StepReject::NonFiniteInput | StepReject::NonPositiveDt | StepReject::DtAboveStateCeiling) {
                assert_eq!(load_in_card(&tokamak, *load), Err(*expected), "case {index}");
            }
        }
        // Each limit of the load is met at its edge: the value at it is taken.
        let edges = [
            TokamakLoad { heating_power_w: 0.0, ..base },
            TokamakLoad { ohmic_power_w: 0.0, ..base },
            TokamakLoad { tritium_fraction: 0.0, ..base },
            TokamakLoad { tritium_fraction: 1.0, ..base },
            TokamakLoad { line_density_per_m3: tokamak.greenwald_density_per_m3(3.8e6), ..base },
            TokamakLoad { toroidal_field_t: 2.0 * 3.8 / tokamak.safety_factor(1.0e6, 1.0) * 1.000001, ..base },
        ];
        for (index, edge) in edges.iter().enumerate() {
            assert_eq!(load_in_card(&tokamak, *edge), Ok(()), "edge {index}");
        }
        // A state that is not a plasma.
        for (thermal, fast) in [(0.0, 1.0), (-1.0, 0.0), (1.0e6, -1.0), (f64::NAN, 0.0), (1.0e6, f64::INFINITY)] {
            assert!(TokamakState::try_start(&tokamak, thermal, fast).is_none());
        }
        for elapsed in [-1.0e-12, f64::NAN, f64::INFINITY] {
            assert!(TokamakState::try_continue(&tokamak, elapsed, 4.0e6, 0.5e6).is_none());
        }
        let mut frame = blank_frame();
        let mut broken = state;
        broken.fast_ion_energy_j = -1.0;
        let held = broken;
        assert_eq!(step(&tokamak, &mut broken, &mut frame, base, GATE_DT_S), Err(StepReject::EnergyOutsideCard));
        assert_eq!(broken, held);
        assert_eq!(state_in_card(&tokamak, 0.0, 0.5e6, base), Err(StepReject::EnergyOutsideCard));
        let mut broken = state;
        broken.loss_power_w = f64::NAN;
        assert_eq!(step(&tokamak, &mut broken, &mut frame, base, GATE_DT_S), Err(StepReject::NonFiniteState));
        assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
        proof_intact(&frame);
    }

    /// The clock has no end: a step from 5.475 s, the longest compared
    /// window, lands, and so does one from an hour. A negative clock is
    /// refused as NegativeTime, and a step too small to move the clock as
    /// StepBelowClockResolution, each with state and frame kept.
    #[test]
    fn the_clock_runs_past_the_compared_window_and_holds_its_two_limits() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        for (elapsed, dt_s) in [(TIME_COMPARED_MAX_S, GATE_DT_S), (TIME_COMPARED_MAX_S, FLOOR_DT_S), (3600.0, FLOOR_DT_S), (8.0e9, FLOOR_DT_S)] {
            let mut state = TokamakState::try_continue(&tokamak, elapsed, 4.0e6, 0.5e6).unwrap();
            let mut frame = blank_frame();
            assert_eq!(step(&tokamak, &mut state, &mut frame, even_mix(), dt_s), Ok(()), "{elapsed}");
            assert!(state.t_s > elapsed);
            assert_eq!(frame.t, state.t_s);
        }
        let (_, inside_state) = inside();
        let mut cases = [(inside_state, FLOOR_DT_S, StepReject::NegativeTime); 4];
        cases[0].0.t_s = -1.0e-9;
        cases[1].0.t_s = f64::from_bits((-0.0_f64).to_bits() + 1);
        // 2^34 s: the spacing of 64-bit numbers is 3.8e-6 s and a step of 1e-6 s rounds to nothing.
        cases[2] = (TokamakState { t_s: 17_179_869_184.0, ..inside_state }, FLOOR_DT_S, StepReject::StepBelowClockResolution);
        cases[3] = (TokamakState { t_s: 1.0e300, ..inside_state }, GATE_DT_S, StepReject::StepBelowClockResolution);
        for (index, (state, dt_s, expected)) in cases.iter().enumerate() {
            let mut trial = *state;
            let mut frame = blank_frame();
            assert_eq!(step(&tokamak, &mut trial, &mut frame, even_mix(), *dt_s), Err(*expected), "case {index}");
            assert_eq!(trial, *state);
            assert_eq!(frame.to_bytes(), blank_frame().to_bytes());
            proof_intact(&frame);
        }
        // Just under 2^34 s a step of 1e-6 s still moves the clock.
        let mut late = TokamakState { t_s: f64::from_bits(17_179_869_184.0_f64.to_bits() - 1), ..inside_state };
        let mut frame = blank_frame();
        assert_eq!(step(&tokamak, &mut late, &mut frame, even_mix(), FLOOR_DT_S), Ok(()));
        // A run at the ceiling lands every step past 5.475 s, to 6 s, with the ledger closed.
        let (_, mut state) = inside();
        let mut worst = 0.0_f64;
        for _ in 0..6_000 {
            step(&tokamak, &mut state, &mut frame, even_mix(), GATE_DT_S).unwrap();
            worst = worst.max(state.residual_conservation_error);
        }
        assert!(within(state.t_s, 6.0, 1.0e-12));
        assert!(worst < 0.01 * RESIDUAL_CONSERVATION_LIMIT, "{worst}");
    }

    /// Over a grid of loads and starts inside the compared envelope, one step
    /// at the 1 ms ceiling lands, never refused by the ledger or by the state
    /// ceiling, which is at least 1.0018 ms at every start and every landing;
    /// and runs held at the corners of the loads for 5.475 s do the same.
    #[test]
    fn inside_the_compared_envelope_the_state_ceiling_never_binds_and_the_ledger_holds() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let pairs = [(1.65e6, 1.80), (2.5e6, 2.6), (3.8e6, 4.0), (PLASMA_CURRENT_COMPARED_MAX_A, TOROIDAL_FIELD_COMPARED_MAX_T)];
        let compared = |thermal: f64, fast: f64, at: TokamakLoad| {
            let kev = tokamak.temperature_kev(thermal, at.line_density_per_m3);
            let beta = tokamak.beta_normalised(thermal + fast, at.plasma_current_a, at.toroidal_field_t);
            let confinement = tokamak.confinement_time_s(thermal, at);
            (THERMAL_ENERGY_COMPARED_MIN_J..=THERMAL_ENERGY_COMPARED_MAX_J).contains(&thermal)
                && (FAST_ION_ENERGY_COMPARED_MIN_J..=FAST_ION_ENERGY_COMPARED_MAX_J).contains(&fast)
                && (TEMPERATURE_COMPARED_MIN_KEV..=TEMPERATURE_COMPARED_MAX_KEV).contains(&kev)
                && (super::BETA_NORMALISED_COMPARED_MIN..=BETA_NORMALISED_COMPARED_MAX).contains(&beta)
                && (CONFINEMENT_TIME_COMPARED_MIN_S..=CONFINEMENT_TIME_COMPARED_MAX_S).contains(&confinement)
        };
        let mut tried = 0;
        let mut least_ceiling = f64::INFINITY;
        let mut worst = 0.0_f64;
        for heating in [HEATING_POWER_COMPARED_MIN_W, 10.0e6, HEATING_POWER_COMPARED_MAX_W] {
            for ohmic in [OHMIC_POWER_COMPARED_MIN_W, OHMIC_POWER_COMPARED_MAX_W] {
                for (current, field) in pairs {
                    let greenwald = tokamak.greenwald_density_per_m3(current);
                    let densest = if GREENWALD_FRACTION_COMPARED_MAX * greenwald < LINE_DENSITY_COMPARED_MAX_PER_M3 {
                        GREENWALD_FRACTION_COMPARED_MAX * greenwald
                    } else {
                        LINE_DENSITY_COMPARED_MAX_PER_M3
                    };
                    for density in [LINE_DENSITY_COMPARED_MIN_PER_M3, 0.5 * (LINE_DENSITY_COMPARED_MIN_PER_M3 + densest), densest] {
                        for tritium in [0.0, TRITIUM_FRACTION_TRACE_MAX, TRITIUM_FRACTION_EVEN_MIN, TRITIUM_FRACTION_EVEN_MAX] {
                            let at = load(heating, ohmic, density, current, field, tritium);
                            let q = tokamak.safety_factor(current, field);
                            if !(SAFETY_FACTOR_COMPARED_MIN..=SAFETY_FACTOR_COMPARED_MAX).contains(&q) {
                                continue;
                            }
                            assert_eq!(load_in_card(&tokamak, at), Ok(()));
                            for kev in [TEMPERATURE_COMPARED_MIN_KEV, 1.5, 2.5, 3.5, TEMPERATURE_COMPARED_MAX_KEV] {
                                let thermal = kev / tokamak.temperature_kev(1.0, density);
                                for fast in [FAST_ION_ENERGY_COMPARED_MIN_J, 0.3e6, 1.5e6, FAST_ION_ENERGY_COMPARED_MAX_J] {
                                    if !compared(thermal, fast, at) {
                                        continue;
                                    }
                                    tried += 1;
                                    let ceiling = tokamak.state_ceiling_dt_s(thermal, fast, at);
                                    least_ceiling = least_ceiling.min(ceiling);
                                    let (_, mut state) = start(thermal, fast);
                                    let mut frame = blank_frame();
                                    let end = step(&tokamak, &mut state, &mut frame, at, GATE_DT_S);
                                    assert_eq!(end, Ok(()), "{at:?} {thermal} {fast}");
                                    least_ceiling = least_ceiling.min(tokamak.state_ceiling_dt_s(state.thermal_energy_j, state.fast_ion_energy_j, at));
                                    worst = worst.max(state.residual_conservation_error);
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(tried > 400, "{tried}");
        assert!(least_ceiling >= 1.0017e-3, "{least_ceiling}");
        // The largest found is 4.9e-10, a fortieth of the limit.
        assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT, "{worst}");
        // Runs held at the corners, from a start at 2 keV, for 5.475 s.
        let mut runs = 0;
        let mut run_worst = 0.0_f64;
        for heating in [HEATING_POWER_COMPARED_MIN_W, HEATING_POWER_COMPARED_MAX_W] {
            for (current, field) in pairs {
                let greenwald = tokamak.greenwald_density_per_m3(current);
                for density in [LINE_DENSITY_COMPARED_MIN_PER_M3, 0.6 * GREENWALD_FRACTION_COMPARED_MAX * greenwald] {
                    let at = load(heating, OHMIC_POWER_COMPARED_MAX_W, density, current, field, 0.5);
                    let thermal = 2.0 / tokamak.temperature_kev(1.0, density);
                    for fast in [0.1e6, 1.0e6] {
                        if !compared(thermal, fast, at) {
                            continue;
                        }
                        runs += 1;
                        let (_, mut state) = start(thermal, fast);
                        let mut frame = blank_frame();
                        for _ in 0..5_475 {
                            let held = state;
                            let end = step(&tokamak, &mut state, &mut frame, at, GATE_DT_S);
                            if !compared(state.thermal_energy_j, state.fast_ion_energy_j, at) {
                                // 0.9.0 refused this step; the state before it was compared.
                                assert!(compared(held.thermal_energy_j, held.fast_ion_energy_j, at));
                                break;
                            }
                            assert_eq!(end, Ok(()), "{at:?} {thermal} {fast} at {}", state.t_s);
                            run_worst = run_worst.max(state.residual_conservation_error);
                        }
                    }
                }
            }
        }
        assert!(runs >= 10, "{runs}");
        // The largest found is 3.9e-10.
        assert!(run_worst < 0.05 * RESIDUAL_CONSERVATION_LIMIT, "{run_worst}");
    }

    /// The three times of the state ceiling at the fastest corners of the
    /// compared envelope: the slowing-down time at least 18.06 ms, the loss
    /// time 46.5 ms and the coupling time 48.1 ms, so an eighteenth, a
    /// forty-sixth and a forty-eighth of them are each at least 1 ms, and the
    /// 1 ms ceiling is never above the state ceiling there. Each fraction is
    /// the largest whole one that does so.
    #[test]
    fn the_step_ceilings_and_the_floor_are_derived_and_hold_the_ledger() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        assert_eq!(tokamak.gate_dt_s(), Some(GATE_DT_S));
        assert_eq!(tokamak.floor_dt_s(), Some(FLOOR_DT_S));
        let slowing = tokamak.slowing_time_coeff_s * TEMPERATURE_COMPARED_MIN_KEV.sqrt()
            / (LINE_DENSITY_COMPARED_MAX_PER_M3 / 1.0e19);
        assert!(within(slowing, 0.01806, 5.0e-3), "{slowing}");
        assert!(slowing / STATE_CEILING_SLOWING_STEPS >= 1.003e-3);
        assert!(slowing / (STATE_CEILING_SLOWING_STEPS + 1.0) < 1.0e-3);
        let loss = IPB98_EXP_LOSS * CONFINEMENT_TIME_COMPARED_MIN_S;
        assert!(within(loss, 0.0465, 1.0e-12));
        assert!(loss / STATE_CEILING_LOSS_STEPS >= 1.01e-3);
        assert!(loss / (STATE_CEILING_LOSS_STEPS + 1.0) < 1.0e-3);
        // The coupling time does not depend on the density: 2 τ_f W_th / W_f.
        let coupling = |density: f64| {
            let thermal = TEMPERATURE_COMPARED_MIN_KEV / tokamak.temperature_kev(1.0, density);
            2.0 * tokamak.slowing_time_coeff_s * TEMPERATURE_COMPARED_MIN_KEV.sqrt() / (density / 1.0e19) * thermal
                / FAST_ION_ENERGY_COMPARED_MAX_J
        };
        assert!(within(coupling(LINE_DENSITY_COMPARED_MIN_PER_M3), coupling(LINE_DENSITY_COMPARED_MAX_PER_M3), 1.0e-12));
        assert!(within(coupling(1.0e19), 0.0481, 2.0e-3), "{}", coupling(1.0e19));
        assert!(coupling(1.0e19) / STATE_CEILING_COUPLING_STEPS >= 1.0017e-3);
        assert!(coupling(1.0e19) / (STATE_CEILING_COUPLING_STEPS + 1.0) < 1.0e-3);
        // The state ceiling is each of the three where it is the smallest.
        let at = TokamakLoad { line_density_per_m3: LINE_DENSITY_COMPARED_MAX_PER_M3, plasma_current_a: PLASMA_CURRENT_COMPARED_MAX_A, ..even_mix() };
        let thermal = TEMPERATURE_COMPARED_MIN_KEV / tokamak.temperature_kev(1.0, LINE_DENSITY_COMPARED_MAX_PER_M3);
        assert!(within(tokamak.state_ceiling_dt_s(thermal, 0.0, at), slowing / 18.0, 1.0e-12));
        let fast_loss = load(24.3e6, 2.44e6, 1.65e19, 1.65e6, 1.80, 0.5);
        assert!(within(
            tokamak.state_ceiling_dt_s(2.4e6, 0.0, fast_loss),
            IPB98_EXP_LOSS * tokamak.confinement_time_s(2.4e6, fast_loss) / 46.0,
            1.0e-12
        ));
        let cold = thermal_at(0.3, 2.0e19);
        let tau_f = tokamak.slowing_time_coeff_s * 0.3_f64.sqrt() / 2.0;
        assert!(within(
            tokamak.state_ceiling_dt_s(cold, 1.0e6, load(20.0e6, 1.0e6, 2.0e19, 3.8e6, 4.0, 0.5)),
            2.0 * tau_f * cold / 1.0e6 / 48.0,
            1.0e-9
        ));
        // A step at the floor moves both stores and the clock.
        let (_, mut state) = inside();
        let held = state;
        let mut frame = blank_frame();
        step(&tokamak, &mut state, &mut frame, even_mix(), FLOOR_DT_S).unwrap();
        assert!(state.thermal_energy_j != held.thermal_energy_j);
        assert!(state.fast_ion_energy_j != held.fast_ion_energy_j);
        assert!(state.t_s > held.t_s);
        assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
    }

    /// One point past an old wall of the compared envelope: what the step
    /// does there.
    struct Past {
        label: &'static str,
        thermal_j: f64,
        fast_j: f64,
        load: TokamakLoad,
    }

    fn past(label: &'static str, thermal_j: f64, fast_j: f64, load: TokamakLoad) -> Past {
        Past { label, thermal_j, fast_j, load }
    }

    /// Every old wall of the compared envelope, at its edge and well past it.
    fn past_the_old_walls() -> [Past; 30] {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let field_for = |q: f64, current_a: f64| q * (current_a / 1.0e6) / tokamak.safety_factor(1.0e6, 1.0);
        [
            past("heating 0 MW", 2.0e6, 0.0, load(0.0, 0.5e6, 3.0e19, 2.5e6, 2.6, 0.5)),
            past("heating 30 MW", 4.0e6, 0.5e6, load(30.0e6, 1.0e6, 6.0e19, 3.8e6, 4.0, 0.5)),
            past("heating 100 MW", 4.0e6, 0.5e6, load(100.0e6, 1.0e6, 6.0e19, 5.0e6, 5.0, 0.5)),
            past("ohmic 0 MW", 4.0e6, 0.5e6, load(20.0e6, 0.0, 6.0e19, 3.8e6, 4.0, 0.5)),
            past("ohmic 10 MW", 2.0e6, 0.1e6, load(5.0e6, 10.0e6, 4.0e19, 3.8e6, 4.0, 0.5)),
            past("current 1.0 MA at 1.0 T", 0.5e6, 0.05e6, load(2.0e6, 0.5e6, 2.0e19, 1.0e6, 1.0, 0.5)),
            past("current 4.16 MA, 42976", 4.0e6, 0.5e6, load(5.6e6, 3.115e6, 1.5025e19, 4.128e6, 3.827, 0.501)),
            past("current 5 MA at 5 T", 6.0e6, 0.5e6, load(30.0e6, 1.0e6, 6.0e19, 5.0e6, 5.0, 0.5)),
            past("field 8 T", 6.0e6, 0.5e6, load(20.0e6, 1.0e6, 6.0e19, 5.0e6, 8.0, 0.5)),
            past("tritium 0.25", 4.0e6, 0.5e6, TokamakLoad { tritium_fraction: 0.25, ..even_mix() }),
            past("tritium 0.93", 4.0e6, 0.5e6, TokamakLoad { tritium_fraction: 0.93, ..even_mix() }),
            past("tritium 1.0", 4.0e6, 0.5e6, TokamakLoad { tritium_fraction: 1.0, ..even_mix() }),
            past("density 0.5e19", 0.5e6, 0.1e6, load(10.0e6, 1.0e6, 0.5e19, 2.5e6, 2.6, 0.5)),
            past("density 1e20 at 5 MA", 8.0e6, 1.0e6, load(20.0e6, 1.0e6, 1.0e20, 5.0e6, 5.0, 0.5)),
            past("Greenwald 0.95 at 5 MA", 8.0e6, 1.0e6, load(20.0e6, 1.0e6, 0.95 * tokamak.greenwald_density_per_m3(5.0e6), 5.0e6, 5.0, 0.5)),
            past("Greenwald 1.0 at 2.5 MA", 5.0e6, 1.0e6, load(10.0e6, 1.0e6, tokamak.greenwald_density_per_m3(2.5e6), 2.5e6, 2.6, 0.5)),
            past("safety factor 2.05", 4.0e6, 0.5e6, load(20.0e6, 0.5e6, 6.0e19, 3.84e6, field_for(2.05, 3.84e6), 0.5)),
            past("safety factor 3.39, 42976", 4.0e6, 0.5e6, load(20.0e6, 0.5e6, 6.0e19, 3.84e6, field_for(3.39, 3.84e6), 0.5)),
            past("safety factor 8", 2.0e6, 0.3e6, load(10.0e6, 0.5e6, 3.0e19, 2.0e6, field_for(8.0, 2.0e6), 0.5)),
            past("thermal 0.2 MJ", 0.2e6, 0.02e6, load(1.0e6, 0.5e6, 1.0e19, 2.0e6, 2.5, 0.5)),
            past("thermal 20 MJ", 20.0e6, 1.0e6, load(30.0e6, 1.0e6, 1.0e20, 5.0e6, 6.0, 0.5)),
            past("fast ions 0", 4.0e6, 0.0, even_mix()),
            past("fast ions 3.5 MJ, 42976", 4.0e6, 3.5e6, even_mix()),
            past("fast ions 10 times the thermal", 0.3e6, 3.0e6, load(30.0e6, 0.5e6, 0.3e19, 3.8e6, 4.0, 0.5)),
            past("temperature 0.3 keV", thermal_at(0.3, 6.0e19), 0.1e6, load(5.0e6, 1.0e6, 6.0e19, 3.8e6, 4.0, 0.5)),
            past("temperature 0.3 keV, 1 MJ of fast ions", thermal_at(0.3, 2.0e19), 1.0e6, load(20.0e6, 1.0e6, 2.0e19, 3.8e6, 4.0, 0.5)),
            past("temperature 15 keV", thermal_at(15.0, 2.0e19), 0.5e6, load(20.0e6, 1.0e6, 2.0e19, 5.0e6, 5.0, 0.5)),
            past("temperature 30 keV", thermal_at(30.0, 1.0e19), 0.5e6, load(20.0e6, 1.0e6, 1.0e19, 5.0e6, 5.0, 0.5)),
            past("confinement 0.03 s", 2.4e6, 0.1e6, load(24.3e6, 2.44e6, 1.0e19, 1.65e6, 1.80, 0.5)),
            past("confinement 9 s", 2.0e6, 0.5e6, load(1.0e6, 0.2e6, 6.0e19, 3.8e6, 4.0, 0.5)),
        ]
    }

    /// Past every old wall of the compared envelope, at its edge and well
    /// beyond: a step just above the state's ceiling, and one at the 1 ms
    /// ceiling where the state's is shorter, is refused as
    /// DtAboveStateCeiling with state and frame kept; and a run that steps at
    /// the ceiling of each state it reaches, or at 1 ms where that is
    /// shorter, lands every step with the ledger closed, for 2000 steps or
    /// until a limit of physics stops it by name.
    #[test]
    fn past_every_old_wall_the_ledger_closes_at_the_state_ceiling() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let mut bound_below_1_ms = 0;
        let mut worst = 0.0_f64;
        let mut full_runs = 0;
        for point in past_the_old_walls() {
            let label = point.label;
            assert_eq!(load_in_card(&tokamak, point.load), Ok(()), "{label}");
            assert_eq!(state_in_card(&tokamak, point.thermal_j, point.fast_j, point.load), Ok(()), "{label}");
            let (_, fresh) = start(point.thermal_j, point.fast_j);
            let ceiling = tokamak.state_ceiling_dt_s(point.thermal_j, point.fast_j, point.load);
            let mut refused_steps = [above(ceiling), 0.0];
            if ceiling < GATE_DT_S {
                bound_below_1_ms += 1;
                refused_steps[1] = GATE_DT_S;
            }
            for dt_s in refused_steps {
                if dt_s == 0.0 {
                    continue;
                }
                let mut state = fresh;
                let mut frame = blank_frame();
                assert_eq!(step(&tokamak, &mut state, &mut frame, point.load, dt_s), Err(StepReject::DtAboveStateCeiling), "{label} {dt_s}");
                assert_eq!(state, fresh, "{label}");
                assert_eq!(frame.to_bytes(), blank_frame().to_bytes(), "{label}");
            }
            let mut state = fresh;
            let mut frame = blank_frame();
            let mut landed = 0;
            let end = loop {
                if landed == 2_000 {
                    break None;
                }
                let ceiling_now = tokamak.state_ceiling_dt_s(state.thermal_energy_j, state.fast_ion_energy_j, point.load);
                let at_ceiling = if ceiling_now < GATE_DT_S { ceiling_now } else { GATE_DT_S };
                let held = state;
                let mut result = step(&tokamak, &mut state, &mut frame, point.load, at_ceiling);
                if result == Err(StepReject::DtAboveStateCeiling) {
                    // The ceiling is shorter where the trial step lands: the
                    // step is refused whole, and a step of half lands.
                    assert_eq!(state, held, "{label}");
                    result = step(&tokamak, &mut state, &mut frame, point.load, 0.5 * at_ceiling);
                }
                match result {
                    Ok(()) => {
                        landed += 1;
                        worst = worst.max(state.residual_conservation_error);
                        assert!(state.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT, "{label}");
                    }
                    Err(reject) => {
                        assert_eq!(state, held, "{label}");
                        break Some(reject);
                    }
                }
            };
            assert!(!ledger(end.map_or(Ok(()), Err)), "{label}: {end:?}");
            assert!(
                matches!(end, None | Some(StepReject::BetaAboveTroyon) | Some(StepReject::TemperatureAboveReactivityFit)),
                "{label}: {end:?} after {landed}"
            );
            assert!(landed > 100, "{label}: {landed}");
            if end.is_none() {
                full_runs += 1;
            }

            proof_intact(&frame);
        }
        assert!(bound_below_1_ms >= 6, "{bound_below_1_ms}");
        assert!(full_runs >= 20, "{full_runs}");
        // The largest found is 1.42e-9.
        assert!(worst < 1.5e-9, "{worst}");
    }

    /// Rule 10 past the old walls. An Euler step in place of the
    /// Runge–Kutta step, at the state ceiling or 1 ms, is refused by the
    /// ledger, and by nothing else; only where the state ceiling is a small
    /// fraction of a millisecond does the Euler step's error stay under the
    /// ledger's limit, at two of the thirty points.
    #[test]
    fn explicit_euler_past_the_old_walls_is_refused_by_the_ledger() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let mut attacked = 0;
        let mut under_the_limit = 0;
        for point in past_the_old_walls() {
            let ceiling = tokamak.state_ceiling_dt_s(point.thermal_j, point.fast_j, point.load);
            let dt_s = if ceiling < GATE_DT_S { ceiling } else { GATE_DT_S };
            let (_, mut state) = start(point.thermal_j, point.fast_j);
            let held = state;
            let mut frame = blank_frame();
            let end = step_with(&tokamak, &mut state, &mut frame, point.load, dt_s, euler);
            if end == Err(StepReject::DtAboveStateCeiling) {
                continue;
            }
            if end == Ok(()) {
                assert!(dt_s < 0.5 * GATE_DT_S, "{}", point.label);
                under_the_limit += 1;
                continue;
            }
            assert!(ledger(end), "{}: {end:?}", point.label);
            assert_eq!(state, held);
            proof_intact(&frame);
            attacked += 1;
        }
        assert!(attacked >= 20, "{attacked}");
        assert!(under_the_limit <= 2, "{under_the_limit}");
    }

    /// 0.6.0 refused states with a confinement time of tens of milliseconds
    /// as FirstLawBreach at the 1 ms ceiling, from the step's truncation
    /// error; 0.7.0 to 0.9.0 refused them as outside the compared envelope.
    /// The step at 1 ms is now refused as DtAboveStateCeiling, and at the
    /// state's own ceiling the run lands with the ledger closed until the
    /// stores reach the Troyon limit.
    #[test]
    fn a_fast_loss_state_steps_at_its_own_ceiling_and_not_into_the_ledger() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        // 2.4 MJ at 1.65e19 per cubic metre under the largest compared loads
        // is 4.1 keV and a confinement time of 63 ms; at 1.0e19, 6.8 keV and 32 ms.
        for (density, confinement_below) in [(1.65e19, 0.07), (1.0e19, 0.035)] {
            let fast_loss = load(HEATING_POWER_COMPARED_MAX_W, OHMIC_POWER_COMPARED_MAX_W, density, 1.65e6, 1.80, 0.5);
            let thermal = 2.4e6;
            assert!(tokamak.confinement_time_s(thermal, fast_loss) < confinement_below);
            let (_, fresh) = start(thermal, 0.1e6);
            let mut state = fresh;
            let mut frame = blank_frame();
            assert_eq!(step(&tokamak, &mut state, &mut frame, fast_loss, GATE_DT_S), Err(StepReject::DtAboveStateCeiling));
            assert_eq!(state, fresh);
            let ceiling = tokamak.state_ceiling_dt_s(thermal, 0.1e6, fast_loss);
            assert!(ceiling < 0.5 * GATE_DT_S, "{ceiling}");
            // At a forty-sixth of 0.31 of the confinement time.
            assert!(within(ceiling, IPB98_EXP_LOSS * tokamak.confinement_time_s(thermal, fast_loss) / 46.0, 1.0e-12));
            let mut landed = 0;
            let end = loop {
                match step(&tokamak, &mut state, &mut frame, fast_loss, ceiling) {
                    Ok(()) => {
                        landed += 1;
                        assert!(state.residual_conservation_error < 0.05 * RESIDUAL_CONSERVATION_LIMIT);
                    }
                    Err(reject) => break reject,
                }
                assert!(landed < 100_000);
            };
            assert!(landed > 100, "{landed}");
            assert_eq!(end, StepReject::BetaAboveTroyon);
            proof_intact(&frame);
        }
    }

    /// Pulse 42976, a hot-ion pulse declared outside the card's regime before
    /// it was run, missed all four marks and is not in the compared envelope.
    /// Each value only it reached (`fix_0.7.0/envelope_nine.json`) is outside
    /// a compared bound, and the card computes its first load and its own
    /// largest states with the same equations.
    #[test]
    fn the_states_and_loads_only_42976_reached_are_computed() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        assert!(25.134e6 > HEATING_POWER_COMPARED_MAX_W && 24.221e6 <= HEATING_POWER_COMPARED_MAX_W);
        assert!(3.1153e6 > OHMIC_POWER_COMPARED_MAX_W && 2.4360e6 <= OHMIC_POWER_COMPARED_MAX_W);
        assert!(1.1991e19 < LINE_DENSITY_COMPARED_MIN_PER_M3 && 1.6580e19 >= LINE_DENSITY_COMPARED_MIN_PER_M3);
        assert!(8.8684e19 > LINE_DENSITY_COMPARED_MAX_PER_M3 && 7.8493e19 <= LINE_DENSITY_COMPARED_MAX_PER_M3);
        assert!(4.1593e6 > PLASMA_CURRENT_COMPARED_MAX_A && 3.8350e6 <= PLASMA_CURRENT_COMPARED_MAX_A);
        assert!(3.3859 < SAFETY_FACTOR_COMPARED_MIN && 3.6631 >= SAFETY_FACTOR_COMPARED_MIN);
        assert!(9.6603e6 > THERMAL_ENERGY_COMPARED_MAX_J && 9.5657e6 <= THERMAL_ENERGY_COMPARED_MAX_J);
        assert!(3.5221e6 > FAST_ION_ENERGY_COMPARED_MAX_J && 1.6181e6 <= FAST_ION_ENERGY_COMPARED_MAX_J);
        assert!(6.2523 > TEMPERATURE_COMPARED_MAX_KEV && 4.2938 <= TEMPERATURE_COMPARED_MAX_KEV);
        assert!(0.13409 < super::BETA_NORMALISED_COMPARED_MIN && 0.22410 >= super::BETA_NORMALISED_COMPARED_MIN);
        assert!(8.4534 > CONFINEMENT_TIME_COMPARED_MAX_S && 5.7160 <= CONFINEMENT_TIME_COMPARED_MAX_S);
        assert!(PLASMA_CURRENT_COMPARED_MIN_A < 1.66e6 && TOROIDAL_FIELD_COMPARED_MIN_T < 1.81 && OHMIC_POWER_COMPARED_MIN_W < 0.174e6);
        // Its first load: 5.60 MW, 3.12 MW of ohmic power, 1.50e19 per cubic
        // metre, 4.13 MA and 3.83 T. It lands.
        let first = load(5.60e6, 3.115e6, 1.5025e19, 4.128e6, 3.827, 0.501);
        assert_eq!(load_in_card(&tokamak, first), Ok(()));
        let (_, mut state) = start(4.0e6, 0.5e6);
        let mut frame = blank_frame();
        assert_eq!(step(&tokamak, &mut state, &mut frame, first, GATE_DT_S), Ok(()));
        // 3.5 MJ of fast ions, and 6.25 keV.
        let held = load(20.0e6, 1.0e6, 3.0e19, 3.8e6, 4.0, 0.5);
        let hot = 6.25 / tokamak.temperature_kev(1.0, 3.0e19);
        for (thermal, fast) in [(4.0e6, 3.5e6), (hot, 0.5e6)] {
            assert_eq!(state_in_card(&tokamak, thermal, fast, held), Ok(()));
            let (_, mut state) = start(thermal, fast);
            assert_eq!(step(&tokamak, &mut state, &mut frame, held, GATE_DT_S), Ok(()));
        }
    }

    /// The clock is the caller's: a run continued from a returned state at
    /// that state's time is the run straight through, bit for bit, past the
    /// longest compared window.
    #[test]
    fn a_continued_run_keeps_the_clock() {
        let (tokamak, mut straight) = inside();
        let mut frame = blank_frame();
        for _ in 0..7000 {
            step(&tokamak, &mut straight, &mut frame, even_mix(), GATE_DT_S).unwrap();
        }
        let (_, mut first) = inside();
        for _ in 0..5000 {
            step(&tokamak, &mut first, &mut frame, even_mix(), GATE_DT_S).unwrap();
        }
        let mut second = TokamakState::try_continue(&tokamak, first.t_s, first.thermal_energy_j, first.fast_ion_energy_j).unwrap();
        for _ in 0..2000 {
            step(&tokamak, &mut second, &mut frame, even_mix(), GATE_DT_S).unwrap();
        }
        assert!(second.t_s > TIME_COMPARED_MAX_S);
        assert_eq!(second.t_s.to_bits(), straight.t_s.to_bits());
        assert_eq!(second.thermal_energy_j.to_bits(), straight.thermal_energy_j.to_bits());
        assert_eq!(second.fast_ion_energy_j.to_bits(), straight.fast_ion_energy_j.to_bits());
        assert!(within(first.fusion_energy_j + second.fusion_energy_j, straight.fusion_energy_j, 1.0e-12));
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
    /// at the centre of the profile: above the fit. Under the Troyon limit at
    /// 6 MA and 6 T the step names it, and below 0.2 keV it names the floor.
    #[test]
    fn a_temperature_outside_the_reactivity_fit_is_refused() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let load = load(20.0e6, 1.0e6, 1.65e19, 6.0e6, 6.0, 0.5);
        assert!((1.0 + PROFILE_EXPONENT) * tokamak.temperature_kev(46.0e6, 1.65e19) > 100.0);
        assert!(46.0e6 < tokamak.troyon_stored_energy_j(6.0e6, 6.0));
        for (thermal, expected) in [(46.0e6, StepReject::TemperatureAboveReactivityFit), (thermal_at(0.199, 1.65e19), StepReject::TemperatureBelowReactivityFit)] {
            let (_, mut state) = start(thermal, 0.0);
            let held = state;
            let mut frame = blank_frame();
            assert_eq!(step(&tokamak, &mut state, &mut frame, load, GATE_DT_S), Err(expected));
            assert_eq!(state_in_card(&tokamak, thermal, 0.0, load), Err(expected));
            assert_eq!(state, held);
            proof_intact(&frame);
        }
    }

    /// Full heating at 1.65 MA and 1.80 T, from 2 MJ: the two stores rise
    /// past the compared normalised beta of 2.21. At 0.83 of the Greenwald
    /// density, held at 1 ms, the run is stopped where the ceiling of the
    /// state the step would land on falls under 1 ms; stepped at the ceiling
    /// of each state, it settles at a normalised beta of 2.73, where the loss
    /// balances the heating, and lands 20,000 steps. At 2e19 per cubic metre
    /// from 1 MJ, stepped the same way, the stores go on to the Troyon limit,
    /// and the step that would cross it is refused with the state on the last
    /// landed step.
    #[test]
    fn the_troyon_limit_stops_a_run_and_keeps_the_last_state() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let dense = load(HEATING_POWER_COMPARED_MAX_W, OHMIC_POWER_COMPARED_MIN_W, 0.83 * tokamak.greenwald_density_per_m3(1.65e6), 1.65e6, 1.80, 0.0);
        let thin = TokamakLoad { line_density_per_m3: 2.0e19, ..dense };
        for (at, thermal, follow, expected) in [
            (dense, 2.0e6, false, Some(StepReject::DtAboveStateCeiling)),
            (dense, 2.0e6, true, None),
            (thin, 1.0e6, true, Some(StepReject::BetaAboveTroyon)),
        ] {
            let (_, mut state) = start(thermal, 0.1e6);
            let mut frame = blank_frame();
            let mut steps = 0;
            let mut passed_compared = false;
            let end = loop {
                if steps == 20_000 {
                    break None;
                }
                let held = state;
                let frame_t = frame.t;
                let ceiling = tokamak.state_ceiling_dt_s(state.thermal_energy_j, state.fast_ion_energy_j, at);
                let dt_s = if follow && ceiling < GATE_DT_S { ceiling } else { GATE_DT_S };
                let mut result = step(&tokamak, &mut state, &mut frame, at, dt_s);
                if follow && result == Err(StepReject::DtAboveStateCeiling) {
                    // Shorter still where the trial step lands: half lands.
                    result = step(&tokamak, &mut state, &mut frame, at, 0.5 * dt_s);
                }
                match result {
                    Ok(()) => steps += 1,
                    Err(reject) => {
                        assert_eq!(state, held);
                        assert_eq!(frame.t, frame_t);
                        break Some(reject);
                    }
                }
                assert!(state.residual_conservation_error < 0.05 * RESIDUAL_CONSERVATION_LIMIT);
                passed_compared |= state.beta_normalised > BETA_NORMALISED_COMPARED_MAX;
            };
            assert_eq!(end, expected);
            assert!(passed_compared);
            assert!(steps > 10);
            if expected == Some(StepReject::BetaAboveTroyon) {
                assert!(state.beta_normalised <= TROYON_BETA_NORMALISED_MAX);
                assert!(state.beta_normalised > 0.98 * TROYON_BETA_NORMALISED_MAX, "{}", state.beta_normalised);
            }
            if expected.is_none() {
                assert!(within(state.beta_normalised, 2.73, 2.0e-3), "{}", state.beta_normalised);
            }
            proof_intact(&frame);
        }
    }

    /// The strongest start attacked: eight times the thermal energy in the
    /// fast ions, at 2e20 per cubic metre and 1 keV, under 7 MA and 7 T. At
    /// the state ceiling the truncation of the step passes the ledger's limit
    /// after about a hundred steps, and that step is refused as
    /// FirstLawBreach with the state kept; at a quarter of the ceiling the
    /// ledger closes for 3000 steps.
    #[test]
    fn the_strongest_fast_ion_start_is_held_by_the_ledger_at_the_full_ceiling() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let at = load(20.0e6, 1.0e6, 2.0e20, 7.0e6, 7.0, 0.5);
        let thermal = thermal_at(1.0, 2.0e20);
        for (fraction, closes) in [(1.0, false), (0.25, true)] {
            let (_, mut state) = start(thermal, 8.0 * thermal);
            let mut frame = blank_frame();
            let mut landed = 0;
            let mut worst = 0.0_f64;
            let end = loop {
                if landed == 3_000 {
                    break None;
                }
                let dt_s = fraction * tokamak.state_ceiling_dt_s(state.thermal_energy_j, state.fast_ion_energy_j, at);
                let held = state;
                match step(&tokamak, &mut state, &mut frame, at, dt_s) {
                    Ok(()) => {
                        landed += 1;
                        worst = worst.max(state.residual_conservation_error);
                    }
                    Err(reject) => {
                        assert_eq!(state, held);
                        break Some(reject);
                    }
                }
            };
            if closes {
                assert_eq!(end, None);
                assert!(worst < 1.0e-9, "{worst}");
            } else {
                assert_eq!(end, Some(StepReject::FirstLawBreach), "{landed}");
                assert!(landed > 50 && landed < 300, "{landed}");
            }
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
        let mut state = TokamakState::try_start(&tokamak, 0.9 * limit_j, 0.11 * limit_j).unwrap();
        let held = state;
        let mut frame = blank_frame();
        assert_eq!(step(&tokamak, &mut state, &mut frame, load, GATE_DT_S), Err(StepReject::BetaAboveTroyon));
        assert_eq!(state, held);
        proof_intact(&frame);
    }

    /// The flag is set above 0.85 of the Greenwald density. The compared span
    /// of 0.834 does not reach it; from 0.85 to the Greenwald density itself
    /// the frame carries it, and above the Greenwald density the step is refused.
    #[test]
    fn the_density_flag_is_set_between_0_85_and_the_greenwald_density() {
        let tokamak = TokamakJetDte1::elmy_h_mode();
        let greenwald = tokamak.greenwald_density_per_m3(2.5e6);
        assert!(GREENWALD_FRACTION_COMPARED_MAX < NEAR_DENSITY_LIMIT_FRACTION);
        for (fraction, flagged) in [(GREENWALD_FRACTION_COMPARED_MAX, false), (NEAR_DENSITY_LIMIT_FRACTION, false), (0.86, true), (0.95, true), (1.0, true)] {
            let at = load(10.0e6, 1.0e6, fraction * greenwald, 2.5e6, 2.6, 0.5);
            let (_, mut state) = start(5.0e6, 1.0e6);
            let mut frame = blank_frame();
            step(&tokamak, &mut state, &mut frame, at, GATE_DT_S).unwrap();
            assert_eq!(state.near_density_limit, flagged, "{fraction}");
            assert_eq!(frame.flags, if flagged { super::FLAG_NEAR_DENSITY_LIMIT } else { 0 }, "{fraction}");
            proof_intact(&frame);
        }
        let over = load(10.0e6, 1.0e6, above(greenwald), 2.5e6, 2.6, 0.5);
        let (_, mut state) = start(5.0e6, 1.0e6);
        let mut frame = blank_frame();
        assert_eq!(step(&tokamak, &mut state, &mut frame, over, GATE_DT_S), Err(StepReject::DensityAboveGreenwald));
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
            assert!(ledger(end), "{end:?}");
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
