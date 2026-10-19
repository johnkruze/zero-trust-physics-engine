#![no_implicit_prelude]

//! The SPERT III E-core digital twin: one card for every tabulated test of
//! the core, cold startup, hot startup, hot standby and operating power,
//! from the start of the transient rod's motion through the burst and on
//! for as long as the run is asked to go.
//!
//! Measured by the Phillips Petroleum Company for the Atomic Energy
//! Commission: IDO-17281 (March 1969), OSTI 4792676. Table III (cold
//! startup, 30 tests), Table V (hot startup at about 260 and 500 degF, 32
//! tests) and Table VII (hot standby and operating power, 8 tests), each
//! value with a standard deviation. The report's cover says "Distribution of
//! this document is unlimited." No measured value of the tables is stored in
//! the card. `reactor_spert3` and `reactor_spert3_fuel` are served beside it
//! unchanged.
//!
//! Fitted on all 70 tests, run on to the minimum of the fit's objective
//! (226.98; 237.67 at 0.14.3), at the median test: peak power within 6.3
//! percent, energy to peak within 8.7, time to peak within 4.0 and the
//! hot-spot clad temperature rise within 9.5; in the report's standard
//! deviations, root mean square 0.99, 2.72, 3.22 and 1.34, with 67 of 70, 64
//! of 70, 40 of 66 and 60 of 69 tests within two. Compared with tests left out
//! of the fit, each refitted on the other 69 from a start that never saw it:
//! 6.8, 8.6, 4.1 and 9.2 percent at the median; 2.15, 2.87, 3.33 and 1.39
//! sd. By series, peak power and time to peak at the median, left out: cold
//! startup 5.0 and 3.1 percent; 260 degF 14.9 and 4.2; 500 degF 6.3 and 5.0;
//! hot standby 12.7 and 3.6. The largest gap: the three operating-power tests from 19 MW, energy
//! to peak 82 percent low and time to peak 39 percent early, in the first
//! 0.15 s: Figs. D-68 to D-70 put the reactivity in over 0.10 to 0.16 s,
//! longer for the smaller insertion, where the rod here takes 0.06 to 0.08 s;
//! not the hand-started clock of those tests (IDO-17281 p. 148), Table VII's
//! times agreeing with the figures' from the transient's start within 0.012
//! s. The record:
//! `evidence/research/reactor_spert3/twin/` (`COMPARISON.md`, `score.py`).
//!
//! ```text
//! inserted reactivity   ρ_ins(t) = ρ_0 − R(x(t)),  x(t) = x_0 − a (t − t_d)² / 2 from t_d until x = 0
//! rod worth (Fig. B-4)  R(x) = s (x − x_c (1 − e^(−x / x_c))),  R(x_0) = ρ_0
//! Doppler               ρ_D = (F_D A / κ) ln(1 + κ (T̄ − T̄_0) / T̄_0),  κ = c_κ PF(T_in)
//! moderator             ρ_M = W_M ∫ α_M(T) dT from T_w0 to T_w
//! net reactivity        ρ = ρ_ins − ρ_D − ρ_M                                   dollars
//! power                 dP/dt = ((ρ − 1) / l*) P + Σ λ_i C_i                   MW per s
//! precursors            dC_i/dt = (b_i / l*) P − λ_i C_i                       i = 1 to 6
//! fuel rings            C_j(T_j) dT_j/dt = f_f P v_j + G_(j−1) (T_(j−1) − T_j) − G_j (T_j − T_(j+1))
//! clad                  C_c dT_c/dt = f_c P + G_fc (T_5 − T_c) − q_cw(T_c, T_w)
//! clad to water         q_cw = S [h (T_c − T_w) + q_nb(T_c − T_sat)],  q_nb(Δ) = (Δ e^(p / 6.2 MPa) / 25 K)^4 MW/m², Δ > 0
//! water                 C_w dT_w/dt = f_w P + q_cw − C_w (2 v / L) (T_w − T_in)
//! hot rod               the same rings and clad at F_hot = c_h PF(T_in) times the power, cooled by the same water:
//!                       the clad thermocouple's place, its clad temperature the one compared
//! peak rod              the same rings and clad at PF(T_in) itself, nothing fitted: the fuel temperatures,
//!                       the margin to melting and the FuelMelting stop; it feeds back nothing
//! energy released       dE/dt = P
//! ```
//!
//! `ρ_0` is the load: the reactivity the transient rod is ejected through,
//! in dollars. The start fields are the inlet temperature `T_in`, the system
//! pressure `p`, the coolant flow speed `v` and the initial power `P_0`; the
//! start is the steady state at `P_0`: precursors at equilibrium, each ring,
//! the clad and the water at their steady temperatures. With no flow there
//! is no steady water temperature: the water starts at the inlet temperature
//! and is heated by the start's power, as the cold tests were.
//!
//! From the report: `l*` (Table II, interpolated in the inlet temperature),
//! the six-group set (Appendix B 3.3), the uniform Doppler coefficient of
//! Table II as `A = α_D T_in`, the moderator coefficient `α_M(T)` (Table II),
//! the rod worth curve fitted to Fig. B-4 at 70, 250 and 500 degF (root mean
//! square 0.007 dollar on 37 read points), the rod's acceleration, 2000
//! in/s² (Appendix B section 8), the heat-source fractions (Appendix B), the
//! pellet, gap, clad, rod count, active length, flow area and hydraulic
//! diameter (Appendix A), and the gap conductance of Eq. B-23 with the
//! report's gap widths (0.0030, 0.0025 and 0.0020 in at 70, 250 and 500
//! degF; 0.0015 in at 1 MW). From elsewhere: UO2 heat capacity, enthalpy and
//! conductivity (Fink 2000, J. Nucl. Mater. 279, 95 percent dense), the
//! saturation temperature (IAPWS-IF97 region 4), water density, specific
//! heat, viscosity and conductivity (polynomials fitted to the NIST WebBook
//! isobar at 10.44 MPa, 290 to 540 K, within 0.02, 0.17, 1.05 and 0.04
//! percent), Dittus-Boelter for forced convection and Jens-Lottes for
//! nucleate boiling. Fitted on the 70 tests, eight constants of the card:
//! [`ROD_DELAY_S`], [`ROD_ACCELERATION_GAIN_PER_MPA`], [`DOPPLER_WEIGHT`],
//! [`DOPPLER_PEAKING_SHARE`], [`DOPPLER_SCALE_HOT`] (two),
//! [`HOT_SPOT_SHARE`] and [`NATURAL_CONVECTION_W_PER_M2_K`], with
//! [`MODERATOR_WEIGHT`] held at one; and three start powers, which the
//! startup tables do not give and a caller sends as `initial_power_mw`:
//! 7.9949547 W cold, 4.1397337 W at 260 degF and 8.3415176 W at 500 degF (the
//! report says about 50 W cold and 10 to 50 W hot). Table VII's initial
//! powers are the report's, per test.
//!
//! Two ledgers. The neutron population `N = P + Σ C_i` obeys `dN/dt = ρ P /
//! l*`; the integral `W` of `ρ P` is taken over each step by the two-point
//! Hermite rule from the values and rates at the step's ends, which is not
//! the Runge-Kutta rule, so a wrong coefficient, a dropped group or a wrong
//! stage weight shows in `N − N_anchor − W / l*`. The energy released `E`
//! equals the heat held in the rings, the clad and the water, from Fink's
//! enthalpy in closed form, plus the heat the flow has carried out, and the
//! hot rod's share, and the peak rod's, equals its own heat held plus its heat to the water. The
//! residual is the largest of the three defects, the population's over the
//! population and the two energies' over the energy released or one kelvin
//! of the core, whichever is larger.
//!
//! [`ReactorSpert3Twin::gate_dt_s`] is the static step ceiling, 0.2 of `l*`
//! at 70 degF. Past it the step itself refuses ([`StepReject::DtAboveStateCeiling`])
//! a step longer than [`DECAY_PHASE_STEP`] of the prompt time constant `l* / |1 − ρ|`, or
//! [`GROWTH_PHASE_STEP`] of it when the prompt pole is growing, at the state
//! it starts from or at the state it lands on. A step that crosses the start
//! or the stop of the rod's motion is taken in two Runge-Kutta parts, split
//! there.
//!
//! `core` only; `sqrt` from IEEE 754 and every other function from
//! [`crate::math`]. The state is `Copy`. An accepted step writes time, power,
//! energy released, net reactivity, the average fuel temperature, the
//! hot-spot clad temperature, the water temperature, the peak rod's fuel
//! margin to melting and the residual into [`LastStateFrame64`]. The 16-byte
//! frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

/// Fuel rings of each rod, equal in radial width.
pub const RINGS: usize = 6;
/// Length of the integrated vector: power, six precursors, the average rod's
/// rings and clad, the hot rod's rings and clad, the water, the energy
/// released, the heat the flow carried out and the hot rod's heat to the water,
/// then the peak rod's rings and clad and its heat to the water.
const NY: usize = 15 + 3 * RINGS;
const IP: usize = 0;
const IC: usize = 1;
const IF: usize = 7;
const ICLAD: usize = 7 + RINGS;
const IH: usize = 8 + RINGS;
const IHCLAD: usize = 8 + 2 * RINGS;
const IW: usize = 9 + 2 * RINGS;
const IE: usize = 10 + 2 * RINGS;
const IQ: usize = 11 + 2 * RINGS;
const IQH: usize = 12 + 2 * RINGS;
const IK: usize = 13 + 2 * RINGS;
const IKCLAD: usize = 13 + 3 * RINGS;
const IQK: usize = 14 + 3 * RINGS;

/// Delayed-neutron group fractions `b_i`, fractions of the total delayed
/// fraction; sum 1. IDO-17281, Appendix B 3.3.
pub const GROUP_FRACTION: [f64; 6] = [0.0352, 0.2014, 0.1858, 0.4006, 0.1437, 0.0333];
/// Delayed-neutron decay constants `λ_i`, per second. Same table.
pub const GROUP_DECAY_PER_S: [f64; 6] = [0.0127, 0.0317, 0.1167, 0.3142, 1.4007, 3.8803];

/// The three temperatures of the report's tables, K: 70, 250 and 500 degF.
/// Every quantity given at them is interpolated linearly between them and
/// continued along the nearer segment outside, in the inlet temperature but
/// for the moderator coefficient, which is taken in the water temperature and
/// integrated from the start's water temperature (`moderator_integral`).
pub const TABLE_TEMPERATURE_K: [f64; 3] = [(70.0 + 459.67) / 1.8, (250.0 + 459.67) / 1.8, (500.0 + 459.67) / 1.8];
/// Reduced prompt-neutron generation time `l*`, s. IDO-17281 Table II.
pub const GENERATION_TIME_REDUCED_S: [f64; 3] = [0.00215, 0.00218, 0.00225];
/// Uniform Doppler coefficient, dollars per K, sign removed: 0.72, 0.56 and
/// 0.37 cents per degF. IDO-17281 Table II. `A = α_D T_in`.
pub const DOPPLER_COEFFICIENT_DOLLAR_PER_K: [f64; 3] = [0.0072 * 1.8, 0.0056 * 1.8, 0.0037 * 1.8];
/// Moderator temperature coefficient, dollars per K, sign removed: 0.4, 2.0
/// and 4.0 cents per degF. IDO-17281 Table II.
pub const MODERATOR_COEFFICIENT_DOLLAR_PER_K: [f64; 3] = [0.0072, 0.036, 0.072];
/// Peak-to-average power density at zero power. IDO-17281 Table II.
pub const PEAK_TO_AVERAGE: [f64; 3] = [5.7, 5.2, 3.6];
/// Rod worth per inch above the bottom region, dollars per inch: the slope
/// `s` of `R(x)` fitted to Fig. B-4 of IDO-17281 (the report labels 30, 29
/// and 18 cents per inch).
pub const ROD_WORTH_SLOPE_DOLLAR_PER_IN: [f64; 3] = [0.28995, 0.28964, 0.18104];
/// Length `x_c` of the bottom region over which the rod's worth per inch
/// rises from zero, inches: fitted to Fig. B-4 with the slope above.
pub const ROD_WORTH_BOTTOM_IN: [f64; 3] = [1.2363, 1.5458, 1.9070];
/// Design acceleration of the transient rod, inches per s². IDO-17281
/// Appendix B section 8.
pub const ROD_ACCELERATION_IN_PER_S2: f64 = 2000.0;
/// Radial pellet-to-clad gap at zero power, inches. IDO-17281 Appendix B,
/// gap widths used by PARET at 70, 250 and 500 degF.
pub const GAP_WIDTH_IN: [f64; 3] = [0.0030, 0.0025, 0.0020];
/// Power at which the gap has closed to 1/e of its zero-power width, MW:
/// `1 / ln(4/3)`, so that 1 MW at 500 degF gives the report's 0.0015 in.
pub const GAP_CLOSING_POWER_MW: f64 = 3.476059496782207;
/// Width added to the gap in Eq. B-23's conductance, inches: a closed gap
/// conducts as about 5e4 W/(m² K).
pub const GAP_JUMP_IN: f64 = 0.0005;

/// Fuel rods in the core. IDO-17281 Appendix A: 48 × 25 + 12 × 16.
pub const FUEL_RODS: f64 = 1392.0;
/// Active length, m: 38.3 in. IDO-17281 Appendix A.
pub const ACTIVE_LENGTH_M: f64 = 38.3 * 0.0254;
/// Pellet radius, m: 0.420 in diameter. IDO-17281 Appendix A.
pub const PELLET_RADIUS_M: f64 = 0.210 * 0.0254;
/// Clad inner radius, m: 0.466 in outer diameter less 0.020 in wall.
pub const CLAD_INNER_RADIUS_M: f64 = 0.213 * 0.0254;
/// Clad outer radius, m: 0.466 in diameter. IDO-17281 Appendix A.
pub const CLAD_OUTER_RADIUS_M: f64 = 0.233 * 0.0254;
/// Pellet density, kg/m³: 10.5 g/cm³. IDO-17281 Appendix A.
pub const PELLET_DENSITY_KG_PER_M3: f64 = 10500.0;
/// Molar mass of the UO2, kg/mol, for Fink's heat capacity per mole.
pub const UO2_MOLAR_MASS_KG_PER_MOL: f64 = 0.27003;
/// Melting point of UO2, K. A hot-rod ring that reaches it is refused.
pub const UO2_MELTING_POINT_K: f64 = 3120.0;
/// Clad density, kg/m³, conductivity, W/(m K), and specific heat, J/(kg K):
/// type 347/348 stainless steel near 100 degC.
pub const CLAD_DENSITY_KG_PER_M3: f64 = 7960.0;
pub const CLAD_CONDUCTIVITY_W_PER_M_K: f64 = 16.2;
pub const CLAD_SPECIFIC_HEAT_J_PER_KG_K: f64 = 500.0;
/// Coolant flow area of the core, m²: 48 × 4.29 + 4 × 3.16 + 8 × 2.80 in².
/// IDO-17281 Appendix A.
pub const FLOW_AREA_M2: f64 = 240.9 * 0.0254 * 0.0254;
/// Hydraulic diameter of the rod lattice, m: 0.585 in square pitch, 0.466 in rods.
pub const HYDRAULIC_DIAMETER_M: f64 = 0.46905 * 0.0254;
/// Shares of the power deposited in the fuel, the clad and the water.
/// IDO-17281 Appendix B, heat-source fractions.
pub const FUEL_HEATING_FRACTION: f64 = 0.964;
pub const CLAD_HEATING_FRACTION: f64 = 0.004;
pub const WATER_HEATING_FRACTION: f64 = 0.032;

/// Fink (2000) UO2 heat capacity: `C1`, J/(mol K); `θ`, K; `C2`, J/(mol K²);
/// `C3`, J/mol; `Ea`, K.
pub const FINK_C1: f64 = 81.613;
pub const FINK_THETA_K: f64 = 548.68;
pub const FINK_C2: f64 = 2.285e-3;
pub const FINK_C3: f64 = 2.360e7;
pub const FINK_EA_K: f64 = 18531.7;
/// IAPWS-IF97 region 4 coefficients `n1` to `n10`, for the saturation temperature.
pub const IF97_N: [f64; 10] = [
    0.11670521452767e4,
    -0.72421316703206e6,
    -0.17073846940092e2,
    0.12020824702470e5,
    -0.32325550322333e7,
    0.14915108613530e2,
    -0.48232657361591e4,
    0.40511340542057e6,
    -0.23855557567849,
    0.65017534844798e3,
];
/// Lowest and highest pressure of IF97 region 4, MPa: the triple point and the critical point.
pub const PRESSURE_MIN_MPA: f64 = 0.000611657;
pub const PRESSURE_MAX_MPA: f64 = 22.064;
/// Water density, kg/m³, specific heat, kJ/(kg K), natural log of the
/// viscosity in µPa s, and conductivity, W/(m K): polynomials in `(T − 400
/// K) / 100 K`, lowest power first, fitted to the NIST WebBook isobar at
/// 10.44 MPa, 290 to 540 K.
pub const WATER_DENSITY_POLY: [f64; 5] = [942.6019703, -82.0372547, -20.1833234, 0.6887558, -2.6653447];
pub const WATER_SPECIFIC_HEAT_POLY: [f64; 5] = [4.2312966, 0.1643199, 0.1121548, 0.0594067, 0.0336235];
pub const WATER_LOG_VISCOSITY_POLY: [f64; 5] = [5.3983936, -0.8253788, 0.325252, -0.1577025, 0.0437374];
pub const WATER_CONDUCTIVITY_POLY: [f64; 5] = [0.6890632, 0.0070676, -0.0553522, 0.0087972, -0.0027397];
/// Jens-Lottes: wall superheat 25 K at 1 MW/m², falling as `e^(−p / 6.2 MPa)`.
pub const JENS_LOTTES_K: f64 = 25.0;
pub const JENS_LOTTES_PRESSURE_MPA: f64 = 6.2;

/// Delay from the clock's zero to the start of the rod's motion, s. Fitted.
pub const ROD_DELAY_S: f64 = 0.0010369315;
/// Weight `F_D` of the average fuel temperature rise in the Doppler
/// feedback: its slope at the start is `F_D A / T̄_0`. Fitted.
pub const DOPPLER_WEIGHT: f64 = 0.92835769;
/// Share `c_κ` of the peak-to-average power density that sets how the
/// Doppler feedback saturates, `κ = c_κ PF(T_in)`. Fitted.
pub const DOPPLER_PEAKING_SHARE: f64 = 0.26584318;
/// The Doppler coefficient at 250 and at 500 degF over Table II's, the 70
/// degF value as printed. Fitted.
pub const DOPPLER_SCALE_HOT: [f64; 2] = [1.1674972, 1.4828376];
/// The rod's acceleration at the system pressure over its design value,
/// less one, per MPa: `a = 2000 in/s² (1 + g p)`. Fitted.
pub const ROD_ACCELERATION_GAIN_PER_MPA: f64 = 0.024930321;
/// Weight `W_M` on the moderator feedback: held at one, Table II's
/// coefficient as printed (fitted, it moved 21 percent between refits at no
/// change in the fit).
pub const MODERATOR_WEIGHT: f64 = 1.0;
/// Share `c_h` of the peak-to-average power density at the clad
/// thermocouple: the hot rod runs at `F_hot = c_h PF(T_in)` times the average rod. Fitted.
pub const HOT_SPOT_SHARE: f64 = 1.4328386;
/// Heat transfer coefficient added to Dittus-Boelter's, W/(m² K): free
/// convection, the whole of it with no flow. Fitted.
pub const NATURAL_CONVECTION_W_PER_M2_K: f64 = 3707.8868;

/// `max(|N − N_anchor − W / l*| / N, energy defects)` must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Fraction of the prompt time constant one step at the ceiling advances.
pub const GATE_PHASE_STEP: f64 = 0.2;
/// Fraction of the time constant of a growing prompt pole one step advances at most.
pub const GROWTH_PHASE_STEP: f64 = 0.06;
/// Fraction of the time constant of a decaying prompt pole one step advances at most.
pub const DECAY_PHASE_STEP: f64 = 0.4;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-9;

/// The transient rod is moving.
pub const FLAG_ROD_MOVING: u64 = 1 << 0;
/// The power is below the highest power of the run: the burst has passed its peak.
pub const FLAG_PAST_PEAK: u64 = 1 << 1;
/// The hot-spot clad is above the saturation temperature: nucleate boiling.
pub const FLAG_HOT_SPOT_BOILING: u64 = 1 << 2;
/// The net reactivity is above one dollar.
pub const FLAG_PROMPT_SUPERCRITICAL: u64 = 1 << 3;
/// The run is farther than one from the nearest of the 70 tested conditions,
/// in [`compared_distance`]: inlet temperature in 25 K, pressure in factors of
/// 2, flow in 1.5 m/s, initial power in factors of 10 and reactivity in 0.1 dollar.
pub const FLAG_OUTSIDE_COMPARED: u64 = 1 << 4;

/// The 70 tested conditions of IDO-17281 Tables III, V and VII, in the card's
/// start fields and load: inlet temperature K, pressure MPa, flow m/s, initial
/// power MW (the three fitted start powers for the startup tests) and the
/// reactivity sent, dollars (the inhour value of the printed period for the
/// startups, Table VII's insertion for the rest). Conditions only: no measured
/// outcome of the tables is here.
pub const COMPARED_CONDITIONS: [[f64; 5]; 70] = [
    [296.48333333333335, 0.0853, 0.0, 7.9949547e-06, 0.768622], // test 22
    [294.2611111111111, 0.0853, 0.0, 7.9949547e-06, 0.890085], // test 18
    [290.37222222222226, 0.0853, 0.0, 7.9949547e-06, 0.933346], // test 13
    [290.9277777777778, 0.0853, 0.0, 7.9949547e-06, 0.937262], // test 14
    [291.48333333333335, 0.0853, 0.0, 7.9949547e-06, 0.972309], // test 39
    [294.2611111111111, 0.0853, 0.0, 7.9949547e-06, 0.973376], // test 23
    [298.7055555555556, 0.0853, 0.0, 7.9949547e-06, 0.981415], // test 45
    [292.03888888888895, 0.0853, 0.0, 7.9949547e-06, 0.981998], // test 15
    [300.9277777777778, 0.0853, 0.0, 7.9949547e-06, 0.982086], // test 50
    [298.15000000000003, 0.0853, 0.0, 7.9949547e-06, 0.982381], // test 44
    [293.7055555555556, 0.0853, 0.0, 7.9949547e-06, 0.983103], // test 46
    [306.48333333333335, 0.0853, 0.0, 7.9949547e-06, 0.983144], // test 71
    [293.15000000000003, 0.0853, 0.0, 7.9949547e-06, 0.984979], // test 17
    [292.5944444444445, 0.0853, 0.0, 7.9949547e-06, 0.986262], // test 74
    [292.03888888888895, 0.0853, 0.0, 7.9949547e-06, 0.987572], // test 87
    [292.5944444444445, 0.0853, 0.0, 7.9949547e-06, 0.99029], // test 73
    [300.9277777777778, 0.0853, 0.0, 7.9949547e-06, 0.99831], // test 51
    [297.5944444444445, 0.0853, 0.0, 7.9949547e-06, 1.001533], // test 49
    [292.5944444444445, 0.0853, 0.0, 7.9949547e-06, 1.009989], // test 16
    [286.48333333333335, 0.0853, 0.0, 7.9949547e-06, 1.013991], // test 38
    [293.7055555555556, 0.0853, 0.0, 7.9949547e-06, 1.028949], // test 19
    [295.37222222222226, 0.0853, 0.0, 7.9949547e-06, 1.045389], // test 20
    [293.15000000000003, 0.0853, 0.0, 7.9949547e-06, 1.049964], // test 75
    [295.37222222222226, 0.0853, 0.0, 7.9949547e-06, 1.086001], // test 21
    [292.5944444444445, 0.0853, 0.0, 7.9949547e-06, 1.087414], // test 40
    [296.48333333333335, 0.0853, 0.0, 7.9949547e-06, 1.092043], // test 48
    [295.37222222222226, 0.0853, 0.0, 7.9949547e-06, 1.098717], // test 47
    [294.2611111111111, 0.0853, 0.0, 7.9949547e-06, 1.12771], // test 41
    [298.7055555555556, 0.0853, 0.0, 7.9949547e-06, 1.164758], // test 42
    [298.7055555555556, 0.0853, 0.0, 7.9949547e-06, 1.210365], // test 43
    [399.2611111111111, 10.44, 4.2672, 4.1397337e-06, 0.751401], // test 24
    [399.2611111111111, 10.44, 4.2672, 4.1397337e-06, 0.75787], // test 25
    [399.2611111111111, 10.44, 6.7056000000000004, 4.1397337e-06, 0.919241], // test 36
    [399.2611111111111, 10.44, 4.2672, 4.1397337e-06, 0.932461], // test 26
    [399.2611111111111, 10.44, 0.7315200000000001, 4.1397337e-06, 0.970007], // test 30
    [399.2611111111111, 10.44, 5.7912, 4.1397337e-06, 0.975297], // test 37
    [399.2611111111111, 10.44, 6.7056000000000004, 4.1397337e-06, 0.978097], // test 33
    [399.2611111111111, 10.44, 4.2672, 4.1397337e-06, 0.97867], // test 27
    [399.2611111111111, 10.44, 6.7056000000000004, 4.1397337e-06, 0.987953], // test 35
    [399.2611111111111, 10.44, 6.7056000000000004, 4.1397337e-06, 1.000715], // test 34
    [399.2611111111111, 10.44, 4.2672, 4.1397337e-06, 1.033574], // test 28
    [399.2611111111111, 10.44, 0.7315200000000001, 4.1397337e-06, 1.037079], // test 31
    [399.2611111111111, 10.44, 0.7315200000000001, 4.1397337e-06, 1.089915], // test 32
    [399.2611111111111, 10.44, 4.2672, 4.1397337e-06, 1.10215], // test 29
    [394.8166666666667, 10.44, 4.2672, 4.1397337e-06, 1.206769], // test 70
    [533.15, 10.44, 4.2672, 8.3415176e-06, 0.642335], // test 52
    [533.15, 10.44, 4.2672, 8.3415176e-06, 0.761913], // test 53
    [533.7055555555556, 10.44, 4.2672, 8.3415176e-06, 0.836652], // test 63
    [533.7055555555556, 10.44, 1.4630400000000001, 8.3415176e-06, 0.83997], // test 64
    [533.15, 10.44, 7.315200000000001, 8.3415176e-06, 0.868624], // test 65
    [532.0388888888889, 10.44, 4.2672, 8.3415176e-06, 0.927981], // test 54
    [533.15, 10.44, 4.2672, 8.3415176e-06, 1.001424], // test 55
    [533.7055555555556, 10.44, 4.2672, 8.3415176e-06, 1.04207], // test 56
    [533.15, 10.44, 4.2672, 8.3415176e-06, 1.093531], // test 57
    [533.15, 10.44, 4.2672, 8.3415176e-06, 1.099566], // test 62
    [533.15, 10.44, 4.2672, 8.3415176e-06, 1.118023], // test 61
    [533.15, 10.44, 4.2672, 8.3415176e-06, 1.133069], // test 68
    [533.7055555555556, 10.44, 1.4630400000000001, 8.3415176e-06, 1.137854], // test 67
    [533.7055555555556, 10.44, 7.315200000000001, 8.3415176e-06, 1.150591], // test 66
    [533.15, 10.44, 4.2672, 8.3415176e-06, 1.152896], // test 58
    [533.15, 10.44, 4.2672, 8.3415176e-06, 1.166908], // test 59
    [533.15, 10.44, 4.2672, 8.3415176e-06, 1.227331], // test 60
    [540.3722222222223, 10.44, 4.2672, 1.1, 0.86], // test 79
    [536.4833333333333, 10.44, 4.2672, 1.3, 1.08], // test 80
    [535.3722222222223, 10.44, 4.2672, 0.9, 1.17], // test 81
    [535.3722222222223, 10.44, 4.2672, 1.1, 1.25], // test 83
    [535.9277777777778, 10.44, 4.2672, 1.2, 1.29], // test 82
    [535.9277777777778, 10.44, 4.2672, 19.0, 0.46], // test 84
    [534.8166666666667, 10.44, 4.2672, 19.0, 0.87], // test 85
    [534.2611111111112, 10.44, 4.2672, 19.0, 1.17], // test 86
];
/// The scales of [`compared_distance`]: one unit of distance is 25 K of inlet
/// temperature, a factor of 2 in pressure, 1.5 m/s of flow, a factor of 10 in
/// initial power or 0.1 dollar of reactivity.
pub const COMPARED_SCALES: [f64; 5] = [25.0, 1.0, 1.5, 1.0, 0.1];

/// Distance of a run's conditions from the nearest tested condition, in the
/// units of [`COMPARED_SCALES`], Euclidean; `None` for the reactivity leaves
/// it out (a start before its first step). A run farther than one is flagged
/// [`FLAG_OUTSIDE_COMPARED`].
#[must_use]
pub fn compared_distance(inlet_k: f64, pressure_mpa: f64, flow_m_per_s: f64, initial_power_mw: f64, reactivity_dollar: Option<f64>) -> f64 {
    let decade = crate::math::log(10.0);
    let power_decades = |p: f64| crate::math::log(if p > 1.0e-12 { p } else { 1.0e-12 }) / decade;
    let octave = crate::math::log(2.0);
    let pressure_octaves = |p: f64| crate::math::log(if p > 1.0e-12 { p } else { 1.0e-12 }) / octave;
    let mut nearest = f64::INFINITY;
    let mut i = 0;
    while i < COMPARED_CONDITIONS.len() {
        let c = &COMPARED_CONDITIONS[i];
        let d = [
            (inlet_k - c[0]) / COMPARED_SCALES[0],
            (pressure_octaves(pressure_mpa) - pressure_octaves(c[1])) / COMPARED_SCALES[1],
            (flow_m_per_s - c[2]) / COMPARED_SCALES[2],
            (power_decades(initial_power_mw) - power_decades(c[3])) / COMPARED_SCALES[3],
            match reactivity_dollar { Some(r) => (r - c[4]) / COMPARED_SCALES[4], None => 0.0 },
        ];
        let squared = d[0] * d[0] + d[1] * d[1] + d[2] * d[2] + d[3] * d[3] + d[4] * d[4];
        if squared < nearest {
            nearest = squared;
        }
        i += 1;
    }
    nearest.sqrt()
}

/// Edges of the box the 70 tests' conditions span, each start field and the reactivity (the range in words; the flag is [`compared_distance`]).
pub const COMPARED_INLET_MIN_K: f64 = (53.0 + 459.67) / 1.8;
pub const COMPARED_INLET_MAX_K: f64 = (513.0 + 459.67) / 1.8;
pub const COMPARED_FLOW_MAX_M_PER_S: f64 = 24.0 * 0.3048;
pub const COMPARED_POWER_MAX_MW: f64 = 19.0;
pub const COMPARED_REACTIVITY_MIN_DOLLAR: f64 = 0.46;
pub const COMPARED_REACTIVITY_MAX_DOLLAR: f64 = 1.29;
pub const COMPARED_PRESSURE_MIN_MPA: f64 = 0.0853;
pub const COMPARED_PRESSURE_MAX_MPA: f64 = 10.44;

/// Why a step did not advance the reactor or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The reactivity or `dt` was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this card's.
    InvalidSpecimen,
    /// The reactivity is below zero: the transient rod is ejected, never driven in.
    ReactivityNegative,
    /// The reactivity of the load is not the one the first step of this run sent the rod through; a run is one ejection.
    ReactivityChangedDuringRun,
    /// The incoming state is not one this card writes: a power at or below zero, a temperature at or below zero, a clock below zero, or a start outside the card.
    StateOutsideCard,
    /// The step is longer than 0.4 of the prompt time constant, or than the growth share of it while the prompt pole grows, at the state it starts from or lands on.
    DtAboveStateCeiling,
    /// A ring of the peak rod, at the core's own peak-to-average power density, reached the melting point of UO2, 3120 K.
    FuelMelting,
    /// The water in the core reached its saturation temperature: bulk boiling, where the single-phase water of the card no longer holds.
    WaterAtSaturation,
    /// The trial step created neutrons or heat from nothing.
    SpontaneousGeneration,
    /// The trial step destroyed neutrons or heat.
    FirstLawBreach,
}

/// Held across one step: the reactivity of the transient rod's ejection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorLoad {
    pub reactivity_inserted_dollar: f64,
}

/// The card. Every field is a recorded copy of a fitted constant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorSpert3Twin {
    pub rod_delay_s: f64,
    pub doppler_weight: f64,
    pub doppler_peaking_share: f64,
    pub doppler_scale_hot: [f64; 2],
    pub rod_acceleration_gain_per_mpa: f64,
    pub moderator_weight: f64,
    pub hot_spot_share: f64,
    pub natural_convection_w_per_m2_k: f64,
}

impl ReactorSpert3Twin {
    /// The SPERT III E-core twin with its fitted constants.
    #[must_use]
    pub fn e_core() -> Self {
        Self {
            rod_delay_s: ROD_DELAY_S,
            doppler_weight: DOPPLER_WEIGHT,
            doppler_peaking_share: DOPPLER_PEAKING_SHARE,
            doppler_scale_hot: DOPPLER_SCALE_HOT,
            rod_acceleration_gain_per_mpa: ROD_ACCELERATION_GAIN_PER_MPA,
            moderator_weight: MODERATOR_WEIGHT,
            hot_spot_share: HOT_SPOT_SHARE,
            natural_convection_w_per_m2_k: NATURAL_CONVECTION_W_PER_M2_K,
        }
    }

    /// Every recorded copy equals its constant, bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.rod_delay_s.to_bits() == ROD_DELAY_S.to_bits()
            && self.doppler_weight.to_bits() == DOPPLER_WEIGHT.to_bits()
            && self.doppler_peaking_share.to_bits() == DOPPLER_PEAKING_SHARE.to_bits()
            && self.doppler_scale_hot[0].to_bits() == DOPPLER_SCALE_HOT[0].to_bits()
            && self.doppler_scale_hot[1].to_bits() == DOPPLER_SCALE_HOT[1].to_bits()
            && self.rod_acceleration_gain_per_mpa.to_bits() == ROD_ACCELERATION_GAIN_PER_MPA.to_bits()
            && self.moderator_weight.to_bits() == MODERATOR_WEIGHT.to_bits()
            && self.hot_spot_share.to_bits() == HOT_SPOT_SHARE.to_bits()
            && self.natural_convection_w_per_m2_k.to_bits() == NATURAL_CONVECTION_W_PER_M2_K.to_bits()
    }

    /// The static step ceiling: 0.2 of `l*` at 70 degF. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        if self.is_valid() { Some(GATE_PHASE_STEP * GENERATION_TIME_REDUCED_S[0]) } else { None }
    }

    /// The smallest step the card stands behind. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        if self.is_valid() { Some(FLOOR_DT_S) } else { None }
    }
}

/// The reactor. Fields after `reactivity_power_integral` are derived from
/// the rest at every landed step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorState {
    pub t_s: f64,
    /// Start fields: held for the whole run.
    pub inlet_temperature_k: f64,
    pub system_pressure_mpa: f64,
    pub coolant_flow_m_per_s: f64,
    pub initial_power_mw: f64,
    /// The average rod's fuel temperature and the water's at the start, K: the zero of the feedback.
    pub start_fuel_average_k: f64,
    pub start_water_k: f64,
    /// The reactivity of the run, recorded by its first step; zero before it.
    pub reactivity_inserted_dollar: f64,
    pub reactivity_recorded: bool,
    pub power_mw: f64,
    pub precursor_mw: [f64; 6],
    pub fuel_k: [f64; RINGS],
    pub clad_k: f64,
    pub hot_fuel_k: [f64; RINGS],
    pub hot_clad_k: f64,
    pub water_k: f64,
    pub energy_released_mws: f64,
    pub heat_carried_out_mws: f64,
    pub hot_heat_to_water_mws: f64,
    /// The peak rod: the same rings and clad at the core's own peak-to-average
    /// power density, `PF(T_in)`, nothing fitted; it gives the fuel
    /// temperatures, the margin to melting and the `FuelMelting` stop. The hot
    /// rod above stands for the clad thermocouple only.
    pub peak_fuel_k: [f64; RINGS],
    pub peak_clad_k: f64,
    pub peak_heat_to_water_mws: f64,
    pub peak_heat_anchor_mws: f64,
    /// Ledger anchors and the booked integral of `ρ P`.
    pub population_anchor_mw: f64,
    pub heat_anchor_mws: f64,
    pub hot_heat_anchor_mws: f64,
    pub reactivity_power_integral: f64,
    /// Derived.
    pub inserted_reactivity_dollar: f64,
    pub doppler_dollar: f64,
    pub moderator_dollar: f64,
    pub net_reactivity_dollar: f64,
    pub rod_position_in: f64,
    pub fuel_average_k: f64,
    pub hot_fuel_centre_k: f64,
    pub saturation_k: f64,
    pub hot_clad_heat_flux_mw_per_m2: f64,
    pub population_defect_mw: f64,
    pub energy_defect_mws: f64,
    pub hot_energy_defect_mws: f64,
    pub peak_fuel_centre_k: f64,
    pub peak_energy_defect_mws: f64,
    pub residual_conservation_error: f64,
    /// The highest power of the run, when it was reached and the energy released by then.
    pub peak_power_mw: f64,
    pub time_of_peak_s: f64,
    pub energy_to_peak_mws: f64,
    /// The highest hot-spot clad temperature of the run, and when.
    pub hot_clad_max_k: f64,
    pub time_of_hot_clad_max_s: f64,
    pub fuel_melt_margin_k: f64,
    pub rod_moving: bool,
    pub past_peak: bool,
    pub hot_spot_boiling: bool,
    pub prompt_supercritical: bool,
    pub outside_compared: bool,
}

/// What a run holds fixed, computed from its start fields and the card.
#[derive(Clone, Copy, Debug)]
struct Terms {
    l_star: f64,
    doppler_a: f64,
    doppler_weight: f64,
    doppler_kappa: f64,
    moderator_weight: f64,
    hot_factor: f64,
    peak_factor: f64,
    rho_total: f64,
    rod_slope: f64,
    rod_bottom: f64,
    rod_start_in: f64,
    rod_delay_s: f64,
    rod_acceleration: f64,
    rod_end_s: f64,
    start_fuel_k: f64,
    start_water_k: f64,
    saturation_k: f64,
    boiling_scale: f64,
    h_single: f64,
    sweep_per_s: f64,
    water_capacity: f64,
    clad_capacity: f64,
    ring_mass_kg: [f64; RINGS],
    ring_share: [f64; RINGS],
    ring_geometry: [f64; RINGS],
    gap_width_in: f64,
    surface_m2: f64,
}

/// Linear in the inlet temperature through the three table points, continued along the nearer segment.
fn table(t_k: f64, values: &[f64; 3]) -> f64 {
    let x = &TABLE_TEMPERATURE_K;
    if t_k <= x[1] {
        values[0] + (values[1] - values[0]) * (t_k - x[0]) / (x[1] - x[0])
    } else {
        values[1] + (values[2] - values[1]) * (t_k - x[1]) / (x[2] - x[1])
    }
}

/// `∫ α_M dT` from `TABLE_TEMPERATURE_K[1]` to `t_k`, the coefficient linear on each side.
fn moderator_integral(t_k: f64) -> f64 {
    let x = &TABLE_TEMPERATURE_K;
    let a = &MODERATOR_COEFFICIENT_DOLLAR_PER_K;
    let slope = if t_k <= x[1] { (a[1] - a[0]) / (x[1] - x[0]) } else { (a[2] - a[1]) / (x[2] - x[1]) };
    let d = t_k - x[1];
    a[1] * d + 0.5 * slope * d * d
}

fn moderator_coefficient(t_k: f64) -> f64 {
    table(t_k, &MODERATOR_COEFFICIENT_DOLLAR_PER_K)
}

fn poly(c: &[f64; 5], t_k: f64) -> f64 {
    let x = (t_k - 400.0) / 100.0;
    c[0] + x * (c[1] + x * (c[2] + x * (c[3] + x * c[4])))
}

/// Saturation temperature at `p_mpa`, K: IAPWS-IF97 region 4.
#[must_use]
pub fn saturation_temperature_k(p_mpa: f64) -> f64 {
    let n = &IF97_N;
    let beta = p_mpa.sqrt().sqrt();
    let e = beta * beta + n[2] * beta + n[5];
    let f = n[0] * beta * beta + n[3] * beta + n[6];
    let g = n[1] * beta * beta + n[4] * beta + n[7];
    let d = 2.0 * g / (-f - (f * f - 4.0 * e * g).sqrt());
    let s = n[9] + d;
    0.5 * (s - (s * s - 4.0 * (n[8] + n[9] * d)).sqrt())
}

/// UO2 specific heat, J/(kg K). Fink (2000).
fn uo2_specific_heat(t_k: f64) -> f64 {
    let x = FINK_THETA_K / t_k;
    let e = crate::math::exp(x);
    let em1 = crate::math::exp_m1(x);
    let molar = FINK_C1 * x * x * e / (em1 * em1) + 2.0 * FINK_C2 * t_k + FINK_C3 * FINK_EA_K * crate::math::exp(-FINK_EA_K / t_k) / (t_k * t_k);
    molar / UO2_MOLAR_MASS_KG_PER_MOL
}

/// UO2 enthalpy, J/kg, from a zero at 0 K of the Einstein term. Fink (2000).
fn uo2_enthalpy(t_k: f64) -> f64 {
    let em1 = crate::math::exp_m1(FINK_THETA_K / t_k);
    (FINK_C1 * FINK_THETA_K / em1 + FINK_C2 * t_k * t_k + FINK_C3 * crate::math::exp(-FINK_EA_K / t_k)) / UO2_MOLAR_MASS_KG_PER_MOL
}

/// UO2 conductivity, W/(m K), 95 percent dense. Fink (2000).
fn uo2_conductivity(t_k: f64) -> f64 {
    let t = t_k / 1000.0;
    100.0 / (7.5408 + 17.692 * t + 3.6142 * t * t) + 6400.0 / (t * t * t.sqrt()) * crate::math::exp(-16.35 / t)
}

/// Gap conductance, W/(m² K): IDO-17281 Eq. B-23 over the gap width plus the jump width.
fn gap_conductance(gap_k: f64, width_in: f64) -> f64 {
    let t_f = gap_k * 1.8 - 459.67;
    let k_btu = (6.43 + 7.43e-3 * t_f - 1.07e-6 * t_f * t_f) / (22.56 + 2.54e4 * width_in);
    k_btu * 1.730_734_666_371_39 / ((width_in + GAP_JUMP_IN) * 0.0254)
}

/// The rod's worth below `x` inches, dollars.
fn rod_worth(terms: &Terms, x_in: f64) -> f64 {
    terms.rod_slope * (x_in + terms.rod_bottom * crate::math::exp_m1(-x_in / terms.rod_bottom))
}

/// The rod's start `x_0`, inches: `R(x_0) = ρ_0` by Newton's method from the right.
fn rod_start(slope: f64, bottom: f64, rho: f64) -> f64 {
    if !(rho > 0.0) {
        return 0.0;
    }
    let mut x = rho / slope + bottom;
    for _ in 0..200 {
        let f = slope * (x + bottom * crate::math::exp_m1(-x / bottom)) - rho;
        let d = -slope * crate::math::exp_m1(-x / bottom);
        let next = x - f / d;
        if !(next < x) {
            break;
        }
        x = next;
    }
    x
}

fn terms(card: &ReactorSpert3Twin, state: &ReactorState, rho_total: f64) -> Terms {
    let inlet_k = state.inlet_temperature_k;
    let p = state.system_pressure_mpa;
    let rod_slope = table(inlet_k, &ROD_WORTH_SLOPE_DOLLAR_PER_IN);
    let rod_bottom = table(inlet_k, &ROD_WORTH_BOTTOM_IN);
    let rod_start_in = rod_start(rod_slope, rod_bottom, rho_total);
    let rod_acceleration = ROD_ACCELERATION_IN_PER_S2 * (1.0 + card.rod_acceleration_gain_per_mpa * p);
    let rod_end_s = card.rod_delay_s + (2.0 * rod_start_in / rod_acceleration).sqrt();
    let density = poly(&WATER_DENSITY_POLY, inlet_k);
    let specific_heat = poly(&WATER_SPECIFIC_HEAT_POLY, inlet_k);
    let viscosity = crate::math::exp(poly(&WATER_LOG_VISCOSITY_POLY, inlet_k)) * 1.0e-6;
    let conductivity = poly(&WATER_CONDUCTIVITY_POLY, inlet_k);
    let v = state.coolant_flow_m_per_s;
    let forced = if v > 0.0 {
        let re = density * v * HYDRAULIC_DIAMETER_M / viscosity;
        let pr = specific_heat * 1000.0 * viscosity / conductivity;
        0.023 * crate::math::powf(re, 0.8) * crate::math::powf(pr, 0.4) * conductivity / HYDRAULIC_DIAMETER_M
    } else {
        0.0
    };
    let uo2_kg = core::f64::consts::PI * PELLET_RADIUS_M * PELLET_RADIUS_M * ACTIVE_LENGTH_M * PELLET_DENSITY_KG_PER_M3 * FUEL_RODS;
    let n = RINGS as f64;
    let mut ring_mass_kg = [0.0; RINGS];
    let mut ring_share = [0.0; RINGS];
    let mut ring_geometry = [0.0; RINGS];
    for j in 0..RINGS {
        let jf = j as f64;
        ring_share[j] = ((jf + 1.0) * (jf + 1.0) - jf * jf) / (n * n);
        ring_mass_kg[j] = uo2_kg * ring_share[j];
        // 2 π L N / ln(r_(j+1) / r_j) between node radii (j + 1/2) Δr; the last, node to pellet surface.
        let ratio = if j + 1 < RINGS { (jf + 1.5) / (jf + 0.5) } else { n / (n - 0.5) };
        ring_geometry[j] = 2.0 * core::f64::consts::PI * ACTIVE_LENGTH_M * FUEL_RODS / crate::math::log(ratio);
    }
    let clad_kg = core::f64::consts::PI
        * (CLAD_OUTER_RADIUS_M * CLAD_OUTER_RADIUS_M - CLAD_INNER_RADIUS_M * CLAD_INNER_RADIUS_M)
        * ACTIVE_LENGTH_M
        * CLAD_DENSITY_KG_PER_M3
        * FUEL_RODS;
    let gap_width_in = table(inlet_k, &GAP_WIDTH_IN) * crate::math::exp(-state.initial_power_mw / GAP_CLOSING_POWER_MW);
    Terms {
        l_star: table(inlet_k, &GENERATION_TIME_REDUCED_S),
        doppler_a: table(
            inlet_k,
            &[
                DOPPLER_COEFFICIENT_DOLLAR_PER_K[0],
                DOPPLER_COEFFICIENT_DOLLAR_PER_K[1] * card.doppler_scale_hot[0],
                DOPPLER_COEFFICIENT_DOLLAR_PER_K[2] * card.doppler_scale_hot[1],
            ],
        ) * inlet_k,
        doppler_weight: card.doppler_weight,
        doppler_kappa: card.doppler_peaking_share * table(inlet_k, &PEAK_TO_AVERAGE),
        moderator_weight: card.moderator_weight,
        hot_factor: card.hot_spot_share * table(inlet_k, &PEAK_TO_AVERAGE),
        peak_factor: table(inlet_k, &PEAK_TO_AVERAGE),
        rho_total,
        rod_slope,
        rod_bottom,
        rod_start_in,
        rod_delay_s: card.rod_delay_s,
        rod_acceleration,
        rod_end_s,
        start_fuel_k: state.start_fuel_average_k,
        start_water_k: state.start_water_k,
        saturation_k: saturation_temperature_k(p),
        boiling_scale: crate::math::exp(p / JENS_LOTTES_PRESSURE_MPA) / JENS_LOTTES_K,
        h_single: forced + card.natural_convection_w_per_m2_k,
        sweep_per_s: 2.0 * v / ACTIVE_LENGTH_M,
        water_capacity: density * specific_heat * FLOW_AREA_M2 * ACTIVE_LENGTH_M * 1.0e-3,
        clad_capacity: clad_kg * CLAD_SPECIFIC_HEAT_J_PER_KG_K * 1.0e-6,
        ring_mass_kg,
        ring_share,
        ring_geometry,
        gap_width_in,
        surface_m2: 2.0 * core::f64::consts::PI * CLAD_OUTER_RADIUS_M * ACTIVE_LENGTH_M * FUEL_RODS,
    }
}

/// Clad-to-water heat flux, W/m²: single phase plus Jens-Lottes above saturation.
fn clad_flux(terms: &Terms, clad_k: f64, water_k: f64) -> f64 {
    let superheat = clad_k - terms.saturation_k;
    let boiling = if superheat > 0.0 {
        let s = superheat * terms.boiling_scale;
        let s2 = s * s;
        s2 * s2 * 1.0e6
    } else {
        0.0
    };
    terms.h_single * (clad_k - water_k) + boiling
}

/// Resistance from the outer ring's node to the clad's mid-wall, K per MW of the whole core.
fn outer_resistance(terms: &Terms, outer_k: f64, clad_k: f64) -> f64 {
    let fuel = 1.0 / (terms.ring_geometry[RINGS - 1] * uo2_conductivity(outer_k));
    let gap_area = 2.0 * core::f64::consts::PI * PELLET_RADIUS_M * ACTIVE_LENGTH_M * FUEL_RODS;
    let gap = 1.0 / (gap_area * gap_conductance(0.5 * (outer_k + clad_k), terms.gap_width_in));
    let mid = 0.5 * (CLAD_INNER_RADIUS_M + CLAD_OUTER_RADIUS_M);
    let clad = crate::math::log(mid / CLAD_INNER_RADIUS_M)
        / (2.0 * core::f64::consts::PI * ACTIVE_LENGTH_M * FUEL_RODS * CLAD_CONDUCTIVITY_W_PER_M_K);
    (fuel + gap + clad) * 1.0e6
}

/// Conductance between ring `j` and ring `j + 1`, MW per K of the whole core.
fn ring_conductance(terms: &Terms, j: usize, a_k: f64, b_k: f64) -> f64 {
    terms.ring_geometry[j] * uo2_conductivity(0.5 * (a_k + b_k)) * 1.0e-6
}

/// Rates of one rod's rings and clad under `power_mw` (the whole core's at
/// this rod's power), and its heat to the water, MW.
fn rod_rates(terms: &Terms, power_mw: f64, fuel: &[f64], clad_k: f64, water_k: f64, out: &mut [f64]) -> f64 {
    let mut flows = [0.0; RINGS];
    for j in 0..RINGS - 1 {
        flows[j] = ring_conductance(terms, j, fuel[j], fuel[j + 1]) * (fuel[j] - fuel[j + 1]);
    }
    flows[RINGS - 1] = (fuel[RINGS - 1] - clad_k) / outer_resistance(terms, fuel[RINGS - 1], clad_k);
    let to_water = terms.surface_m2 * clad_flux(terms, clad_k, water_k) * 1.0e-6;
    for j in 0..RINGS {
        let inflow = if j == 0 { 0.0 } else { flows[j - 1] };
        let capacity = terms.ring_mass_kg[j] * uo2_specific_heat(fuel[j]) * 1.0e-6;
        out[j] = (FUEL_HEATING_FRACTION * power_mw * terms.ring_share[j] + inflow - flows[j]) / capacity;
    }
    out[RINGS] = (CLAD_HEATING_FRACTION * power_mw + flows[RINGS - 1] - to_water) / terms.clad_capacity;
    to_water
}

fn average(terms: &Terms, fuel: &[f64]) -> f64 {
    let mut sum = 0.0;
    for j in 0..RINGS {
        sum += terms.ring_share[j] * fuel[j];
    }
    sum
}

/// Inserted reactivity and its rate at `t_s`.
fn inserted(terms: &Terms, t_s: f64) -> (f64, f64) {
    if t_s <= terms.rod_delay_s {
        return (0.0, 0.0);
    }
    if t_s >= terms.rod_end_s {
        return (terms.rho_total, 0.0);
    }
    let tau = t_s - terms.rod_delay_s;
    let x = terms.rod_start_in - 0.5 * terms.rod_acceleration * tau * tau;
    let worth_per_in = -terms.rod_slope * crate::math::exp_m1(-x / terms.rod_bottom);
    (terms.rho_total - rod_worth(terms, x), worth_per_in * terms.rod_acceleration * tau)
}

/// (ρ_D, ρ_M, net ρ, dρ/dt given the rates).
fn reactivity(terms: &Terms, t_s: f64, y: &[f64; NY], k: Option<&[f64; NY]>) -> (f64, f64, f64, f64) {
    let (rho_ins, rate_ins) = inserted(terms, t_s);
    let fuel_k = average(terms, &y[IF..IF + RINGS]);
    let doppler_k = terms.start_fuel_k + terms.doppler_kappa * (fuel_k - terms.start_fuel_k);
    let doppler = terms.doppler_a * terms.doppler_weight / terms.doppler_kappa * crate::math::log(doppler_k / terms.start_fuel_k);
    let moderator = terms.moderator_weight * (moderator_integral(y[IW]) - moderator_integral(terms.start_water_k));
    let net = rho_ins - doppler - moderator;
    let rate = match k {
        Some(k) => {
            let fuel_rate = average(terms, &k[IF..IF + RINGS]);
            rate_ins - terms.doppler_a * terms.doppler_weight * fuel_rate / doppler_k - terms.moderator_weight * moderator_coefficient(y[IW]) * k[IW]
        }
        None => 0.0,
    };
    (doppler, moderator, net, rate)
}

fn rates(terms: &Terms, inlet_k: f64, t_s: f64, y: &[f64; NY]) -> [f64; NY] {
    let mut k = [0.0; NY];
    let (_, _, rho, _) = reactivity(terms, t_s, y, None);
    let p = y[IP];
    let mut delayed = 0.0;
    for i in 0..6 {
        delayed += GROUP_DECAY_PER_S[i] * y[IC + i];
        k[IC + i] = GROUP_FRACTION[i] * p / terms.l_star - GROUP_DECAY_PER_S[i] * y[IC + i];
    }
    k[IP] = (rho - 1.0) / terms.l_star * p + delayed;
    let mut rod = [0.0; RINGS + 1];
    let to_water = rod_rates(terms, p, &y[IF..IF + RINGS], y[ICLAD], y[IW], &mut rod);
    k[IF..IF + RINGS + 1].copy_from_slice(&rod);
    let hot_to_water = rod_rates(terms, p * terms.hot_factor, &y[IH..IH + RINGS], y[IHCLAD], y[IW], &mut rod);
    k[IH..IH + RINGS + 1].copy_from_slice(&rod);
    let peak_to_water = rod_rates(terms, p * terms.peak_factor, &y[IK..IK + RINGS], y[IKCLAD], y[IW], &mut rod);
    k[IK..IK + RINGS + 1].copy_from_slice(&rod);
    let carried = terms.water_capacity * terms.sweep_per_s * (y[IW] - inlet_k);
    k[IW] = (WATER_HEATING_FRACTION * p + to_water - carried) / terms.water_capacity;
    k[IE] = p;
    k[IQ] = carried;
    k[IQH] = hot_to_water;
    k[IQK] = peak_to_water;
    k
}

fn advance(y: &[f64; NY], k: &[f64; NY], scale: f64) -> [f64; NY] {
    let mut out = *y;
    for i in 0..NY {
        out[i] = y[i] + scale * k[i];
    }
    out
}

fn rk4(terms: &Terms, inlet_k: f64, t_s: f64, y: &[f64; NY], h: f64) -> [f64; NY] {
    let k1 = rates(terms, inlet_k, t_s, y);
    let k2 = rates(terms, inlet_k, t_s + 0.5 * h, &advance(y, &k1, 0.5 * h));
    let k3 = rates(terms, inlet_k, t_s + 0.5 * h, &advance(y, &k2, 0.5 * h));
    let k4 = rates(terms, inlet_k, t_s + h, &advance(y, &k3, h));
    let mut out = *y;
    for i in 0..NY {
        out[i] = y[i] + h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
    out
}

/// `ρ P` and its rate at a state.
fn rho_power(terms: &Terms, inlet_k: f64, t_s: f64, y: &[f64; NY]) -> (f64, f64) {
    let k = rates(terms, inlet_k, t_s, y);
    let (_, _, rho, rate) = reactivity(terms, t_s, y, Some(&k));
    (rho * y[IP], rate * y[IP] + rho * k[IP])
}

/// One Runge-Kutta part and the Hermite integral of `ρ P` over it.
fn part(terms: &Terms, inlet_k: f64, t_s: f64, y: &[f64; NY], h: f64) -> ([f64; NY], f64) {
    let next = rk4(terms, inlet_k, t_s, y, h);
    let (f0, d0) = rho_power(terms, inlet_k, t_s, y);
    let (f1, d1) = rho_power(terms, inlet_k, t_s + h, &next);
    (next, h * (f0 + f1) / 2.0 + h * h * (d0 - d1) / 12.0)
}

/// The step, split at the start and the stop of the rod's motion.
fn integrate(terms: &Terms, inlet_k: f64, t_s: f64, y: &[f64; NY], h: f64) -> ([f64; NY], f64) {
    let end = t_s + h;
    let mut cuts = [end; 3];
    let mut count = 0;
    for b in [terms.rod_delay_s, terms.rod_end_s] {
        if b > t_s && b < end {
            cuts[count] = b;
            count += 1;
        }
    }
    let mut now = t_s;
    let mut state = *y;
    let mut integral = 0.0;
    for cut in cuts.iter().take(count + 1) {
        let width = if *cut == end { end - now } else { *cut - now };
        let (next, w) = part(terms, inlet_k, now, &state, width);
        state = next;
        integral += w;
        now = *cut;
    }
    (state, integral)
}

fn heat_held(terms: &Terms, fuel: &[f64], clad_k: f64) -> f64 {
    let mut sum = 0.0;
    for j in 0..RINGS {
        sum += terms.ring_mass_kg[j] * uo2_enthalpy(fuel[j]) * 1.0e-6;
    }
    sum + terms.clad_capacity * clad_k
}

fn core_heat(terms: &Terms, y: &[f64; NY]) -> f64 {
    heat_held(terms, &y[IF..IF + RINGS], y[ICLAD]) + terms.water_capacity * y[IW]
}

fn vector(state: &ReactorState) -> [f64; NY] {
    let mut y = [0.0; NY];
    y[IP] = state.power_mw;
    y[IC..IC + 6].copy_from_slice(&state.precursor_mw);
    y[IF..IF + RINGS].copy_from_slice(&state.fuel_k);
    y[ICLAD] = state.clad_k;
    y[IH..IH + RINGS].copy_from_slice(&state.hot_fuel_k);
    y[IHCLAD] = state.hot_clad_k;
    y[IW] = state.water_k;
    y[IE] = state.energy_released_mws;
    y[IQ] = state.heat_carried_out_mws;
    y[IQH] = state.hot_heat_to_water_mws;
    y[IK..IK + RINGS].copy_from_slice(&state.peak_fuel_k);
    y[IKCLAD] = state.peak_clad_k;
    y[IQK] = state.peak_heat_to_water_mws;
    y
}

fn unpack(state: &mut ReactorState, y: &[f64; NY]) {
    state.power_mw = y[IP];
    state.precursor_mw.copy_from_slice(&y[IC..IC + 6]);
    state.fuel_k.copy_from_slice(&y[IF..IF + RINGS]);
    state.clad_k = y[ICLAD];
    state.hot_fuel_k.copy_from_slice(&y[IH..IH + RINGS]);
    state.hot_clad_k = y[IHCLAD];
    state.water_k = y[IW];
    state.energy_released_mws = y[IE];
    state.heat_carried_out_mws = y[IQ];
    state.hot_heat_to_water_mws = y[IQH];
    state.peak_fuel_k.copy_from_slice(&y[IK..IK + RINGS]);
    state.peak_clad_k = y[IKCLAD];
    state.peak_heat_to_water_mws = y[IQK];
}

/// Steady rings and clad of one rod at `power_mw`, cooled by water at `water_k`.
fn steady_rod(terms: &Terms, power_mw: f64, water_k: f64) -> Option<([f64; RINGS], f64)> {
    let rod_heat = (FUEL_HEATING_FRACTION + CLAD_HEATING_FRACTION) * power_mw * 1.0e6 / terms.surface_m2;
    if !(terms.h_single > 0.0) {
        return None;
    }
    let mut lo = water_k;
    let mut hi = water_k + rod_heat / terms.h_single;
    for _ in 0..300 {
        let mid = 0.5 * (lo + hi);
        if !(mid > lo && mid < hi) {
            break;
        }
        if clad_flux(terms, mid, water_k) < rod_heat {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let clad_k = 0.5 * (lo + hi);
    let fuel_heat = FUEL_HEATING_FRACTION * power_mw;
    let mut fuel = [clad_k; RINGS];
    let mut outer = clad_k;
    for _ in 0..200 {
        let next = clad_k + fuel_heat * outer_resistance(terms, outer, clad_k);
        if next == outer {
            break;
        }
        outer = next;
    }
    fuel[RINGS - 1] = outer;
    for j in (0..RINGS - 1).rev() {
        let jf = j as f64 + 1.0;
        let crossing = fuel_heat * jf * jf / ((RINGS * RINGS) as f64);
        let mut inner = fuel[j + 1];
        for _ in 0..200 {
            let next = fuel[j + 1] + crossing / ring_conductance(terms, j, inner, fuel[j + 1]);
            if next == inner {
                break;
            }
            inner = next;
        }
        fuel[j] = inner;
    }
    Some((fuel, clad_k))
}

impl ReactorState {
    /// The steady reactor at `initial_power_mw`, the inlet temperature,
    /// pressure and flow given. `None` outside the card: an inlet at or below
    /// 273.15 K or at or above saturation, a pressure outside 0.000611657 to
    /// 22.064 MPa, a flow below zero, a power at or below zero, or a steady
    /// state at or past melting or saturation.
    #[must_use]
    pub fn try_start(
        card: &ReactorSpert3Twin,
        inlet_temperature_k: f64,
        system_pressure_mpa: f64,
        coolant_flow_m_per_s: f64,
        initial_power_mw: f64,
    ) -> Option<Self> {
        if !card.is_valid() {
            return None;
        }
        start_with(card, inlet_temperature_k, system_pressure_mpa, coolant_flow_m_per_s, initial_power_mw)
    }
}

/// The start under any constants: the card's own through [`ReactorState::try_start`].
pub(crate) fn start_with(
    card: &ReactorSpert3Twin,
    inlet_temperature_k: f64,
    system_pressure_mpa: f64,
    coolant_flow_m_per_s: f64,
    initial_power_mw: f64,
) -> Option<ReactorState> {
    let finite = inlet_temperature_k.is_finite() && system_pressure_mpa.is_finite() && coolant_flow_m_per_s.is_finite() && initial_power_mw.is_finite();
    if !finite || !(system_pressure_mpa > PRESSURE_MIN_MPA && system_pressure_mpa < PRESSURE_MAX_MPA) {
        return None;
    }
    if !(inlet_temperature_k > 273.15 && inlet_temperature_k < saturation_temperature_k(system_pressure_mpa)) {
        return None;
    }
    if !(coolant_flow_m_per_s >= 0.0) || !(initial_power_mw > 0.0) {
        return None;
    }
    let mut state = ReactorState {
        t_s: 0.0,
        inlet_temperature_k,
        system_pressure_mpa,
        coolant_flow_m_per_s,
        initial_power_mw,
        start_fuel_average_k: inlet_temperature_k,
        start_water_k: inlet_temperature_k,
        reactivity_inserted_dollar: 0.0,
        reactivity_recorded: false,
        power_mw: initial_power_mw,
        precursor_mw: [0.0; 6],
        fuel_k: [inlet_temperature_k; RINGS],
        clad_k: inlet_temperature_k,
        hot_fuel_k: [inlet_temperature_k; RINGS],
        hot_clad_k: inlet_temperature_k,
        water_k: inlet_temperature_k,
        energy_released_mws: 0.0,
        heat_carried_out_mws: 0.0,
        hot_heat_to_water_mws: 0.0,
        peak_fuel_k: [inlet_temperature_k; RINGS],
        peak_clad_k: inlet_temperature_k,
        peak_heat_to_water_mws: 0.0,
        peak_heat_anchor_mws: 0.0,
        population_anchor_mw: 0.0,
        heat_anchor_mws: 0.0,
        hot_heat_anchor_mws: 0.0,
        reactivity_power_integral: 0.0,
        inserted_reactivity_dollar: 0.0,
        doppler_dollar: 0.0,
        moderator_dollar: 0.0,
        net_reactivity_dollar: 0.0,
        rod_position_in: 0.0,
        fuel_average_k: inlet_temperature_k,
        hot_fuel_centre_k: inlet_temperature_k,
        saturation_k: 0.0,
        hot_clad_heat_flux_mw_per_m2: 0.0,
        population_defect_mw: 0.0,
        energy_defect_mws: 0.0,
        hot_energy_defect_mws: 0.0,
        peak_fuel_centre_k: inlet_temperature_k,
        peak_energy_defect_mws: 0.0,
        residual_conservation_error: 0.0,
        peak_power_mw: initial_power_mw,
        time_of_peak_s: 0.0,
        energy_to_peak_mws: 0.0,
        hot_clad_max_k: inlet_temperature_k,
        time_of_hot_clad_max_s: 0.0,
        fuel_melt_margin_k: 0.0,
        rod_moving: false,
        past_peak: false,
        hot_spot_boiling: false,
        prompt_supercritical: false,
        outside_compared: false,
    };
    let t = terms(card, &state, 0.0);
    let sweep = t.water_capacity * t.sweep_per_s;
    let water_k = if sweep > 0.0 { inlet_temperature_k + initial_power_mw / sweep } else { inlet_temperature_k };
    if !(water_k < t.saturation_k) {
        return None;
    }
    let (fuel, clad) = steady_rod(&t, initial_power_mw, water_k)?;
    let (hot_fuel, hot_clad) = steady_rod(&t, initial_power_mw * t.hot_factor, water_k)?;
    let (peak_fuel, peak_clad) = steady_rod(&t, initial_power_mw * t.peak_factor, water_k)?;
    state.water_k = water_k;
    state.start_water_k = water_k;
    state.fuel_k = fuel;
    state.clad_k = clad;
    state.hot_fuel_k = hot_fuel;
    state.hot_clad_k = hot_clad;
    state.peak_fuel_k = peak_fuel;
    state.peak_clad_k = peak_clad;
    state.start_fuel_average_k = average(&t, &fuel);
    for i in 0..6 {
        state.precursor_mw[i] = GROUP_FRACTION[i] * initial_power_mw / (GROUP_DECAY_PER_S[i] * t.l_star);
    }
    let t = terms(card, &state, 0.0);
    let y = vector(&state);
    state.population_anchor_mw = population(&state);
    state.heat_anchor_mws = core_heat(&t, &y);
    state.hot_heat_anchor_mws = heat_held(&t, &state.hot_fuel_k, state.hot_clad_k);
    state.peak_heat_anchor_mws = heat_held(&t, &state.peak_fuel_k, state.peak_clad_k);
    state.hot_clad_max_k = hot_clad;
    derive(&t, &mut state);
    if !finite_state(&state) || !(state.peak_fuel_k[0] < UO2_MELTING_POINT_K) {
        return None;
    }
    Some(state)
}

fn population(state: &ReactorState) -> f64 {
    let mut n = state.power_mw;
    for c in state.precursor_mw {
        n += c;
    }
    n
}

/// Every derived field from the rest of the state.
fn derive(terms: &Terms, state: &mut ReactorState) {
    let y = vector(state);
    let (doppler, moderator, net, _) = reactivity(terms, state.t_s, &y, None);
    let (rho_ins, _) = inserted(terms, state.t_s);
    state.inserted_reactivity_dollar = rho_ins;
    state.doppler_dollar = doppler;
    state.moderator_dollar = moderator;
    state.net_reactivity_dollar = net;
    state.rod_position_in = if state.t_s <= terms.rod_delay_s {
        terms.rod_start_in
    } else if state.t_s >= terms.rod_end_s {
        0.0
    } else {
        let tau = state.t_s - terms.rod_delay_s;
        terms.rod_start_in - 0.5 * terms.rod_acceleration * tau * tau
    };
    state.fuel_average_k = average(terms, &state.fuel_k);
    state.hot_fuel_centre_k = state.hot_fuel_k[0];
    state.peak_fuel_centre_k = state.peak_fuel_k[0];
    state.saturation_k = terms.saturation_k;
    state.hot_clad_heat_flux_mw_per_m2 = clad_flux(terms, state.hot_clad_k, state.water_k) * 1.0e-6;
    state.population_defect_mw = population(state) - state.population_anchor_mw - state.reactivity_power_integral / terms.l_star;
    let core = core_heat(terms, &y) - state.heat_anchor_mws + state.heat_carried_out_mws;
    state.energy_defect_mws = state.energy_released_mws - core;
    let hot_in = terms.hot_factor * (FUEL_HEATING_FRACTION + CLAD_HEATING_FRACTION) * state.energy_released_mws;
    state.hot_energy_defect_mws = hot_in - (heat_held(terms, &state.hot_fuel_k, state.hot_clad_k) - state.hot_heat_anchor_mws) - state.hot_heat_to_water_mws;
    let peak_in = terms.peak_factor * (FUEL_HEATING_FRACTION + CLAD_HEATING_FRACTION) * state.energy_released_mws;
    state.peak_energy_defect_mws = peak_in - (heat_held(terms, &state.peak_fuel_k, state.peak_clad_k) - state.peak_heat_anchor_mws) - state.peak_heat_to_water_mws;
    let n = population(state).max(state.population_anchor_mw);
    let kelvin = terms.water_capacity + terms.clad_capacity + 0.3;
    let energy_scale = state.energy_released_mws.max(kelvin);
    let hot_scale = (terms.hot_factor * state.energy_released_mws).max(kelvin);
    let peak_scale = (terms.peak_factor * state.energy_released_mws).max(kelvin);
    state.residual_conservation_error = (state.population_defect_mw.abs() / n)
        .max(state.energy_defect_mws.abs() / energy_scale)
        .max(state.hot_energy_defect_mws.abs() / hot_scale)
        .max(state.peak_energy_defect_mws.abs() / peak_scale);
    state.fuel_melt_margin_k = UO2_MELTING_POINT_K - state.peak_fuel_k[0].max(state.peak_fuel_k[1]);
    state.rod_moving = state.t_s > terms.rod_delay_s && state.t_s < terms.rod_end_s;
    state.past_peak = state.power_mw < state.peak_power_mw;
    state.hot_spot_boiling = state.hot_clad_k > terms.saturation_k;
    state.prompt_supercritical = net > 1.0;
    state.outside_compared = !(compared_distance(
        state.inlet_temperature_k,
        state.system_pressure_mpa,
        state.coolant_flow_m_per_s,
        state.initial_power_mw,
        if state.reactivity_recorded { Some(state.reactivity_inserted_dollar) } else { None },
    ) <= 1.0);
}

/// Advance one step and write the frame. A refused step leaves `state` and
/// `frame` as they were. A value of the landed state below the normal range
/// is written as zero with its sign ([`crate::normal`]).
pub fn step(
    card: &ReactorSpert3Twin,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(card, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("reactor_spert3_twin", exceedances(card, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    card: &ReactorSpert3Twin,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !load.reactivity_inserted_dollar.is_finite() || !dt_s.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !card.is_valid() {
        return Err(StepReject::InvalidSpecimen);
    }
    step_with(card, state, frame, load, dt_s)
}

/// The step under any constants: the card's own through [`step`].
pub(crate) fn step_with(
    card: &ReactorSpert3Twin,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    let mut next = step_as_computed(card, state, load, dt_s)?;
    normal_or_zero_state(&mut next);
    *state = next;
    publish(state, frame);
    Ok(())
}

fn normal_or_zero_state(state: &mut ReactorState) {
    let s = state;
    for v in [
        &mut s.t_s,
        &mut s.inlet_temperature_k,
        &mut s.system_pressure_mpa,
        &mut s.coolant_flow_m_per_s,
        &mut s.initial_power_mw,
        &mut s.start_fuel_average_k,
        &mut s.start_water_k,
        &mut s.reactivity_inserted_dollar,
        &mut s.power_mw,
        &mut s.clad_k,
        &mut s.hot_clad_k,
        &mut s.water_k,
        &mut s.energy_released_mws,
        &mut s.heat_carried_out_mws,
        &mut s.hot_heat_to_water_mws,
        &mut s.peak_clad_k,
        &mut s.peak_heat_to_water_mws,
        &mut s.peak_heat_anchor_mws,
        &mut s.peak_fuel_centre_k,
        &mut s.peak_energy_defect_mws,
        &mut s.population_anchor_mw,
        &mut s.heat_anchor_mws,
        &mut s.hot_heat_anchor_mws,
        &mut s.reactivity_power_integral,
        &mut s.inserted_reactivity_dollar,
        &mut s.doppler_dollar,
        &mut s.moderator_dollar,
        &mut s.net_reactivity_dollar,
        &mut s.rod_position_in,
        &mut s.fuel_average_k,
        &mut s.hot_fuel_centre_k,
        &mut s.saturation_k,
        &mut s.hot_clad_heat_flux_mw_per_m2,
        &mut s.population_defect_mw,
        &mut s.energy_defect_mws,
        &mut s.hot_energy_defect_mws,
        &mut s.residual_conservation_error,
        &mut s.peak_power_mw,
        &mut s.time_of_peak_s,
        &mut s.energy_to_peak_mws,
        &mut s.hot_clad_max_k,
        &mut s.time_of_hot_clad_max_s,
        &mut s.fuel_melt_margin_k,
    ] {
        *v = normal_or_zero(*v);
    }
    normal_or_zero_each(&mut s.precursor_mw);
    normal_or_zero_each(&mut s.fuel_k);
    normal_or_zero_each(&mut s.hot_fuel_k);
    normal_or_zero_each(&mut s.peak_fuel_k);
}

fn step_as_computed(card: &ReactorSpert3Twin, state: &ReactorState, load: ReactorLoad, dt_s: f64) -> Result<ReactorState, StepReject> {
    if !load.reactivity_inserted_dollar.is_finite() || !dt_s.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !finite_state(state) {
        return Err(StepReject::NonFiniteState);
    }
    if !(dt_s > 0.0) {
        return Err(StepReject::NonPositiveDt);
    }
    if load.reactivity_inserted_dollar < 0.0 {
        return Err(StepReject::ReactivityNegative);
    }
    if state.reactivity_recorded && load.reactivity_inserted_dollar.to_bits() != state.reactivity_inserted_dollar.to_bits() {
        return Err(StepReject::ReactivityChangedDuringRun);
    }
    if !state_inside(state) {
        return Err(StepReject::StateOutsideCard);
    }
    let mut start = *state;
    start.reactivity_inserted_dollar = load.reactivity_inserted_dollar;
    start.reactivity_recorded = true;
    let terms = terms(card, &start, load.reactivity_inserted_dollar);
    let y = vector(&start);
    if above_state_ceiling(&terms, state.t_s, &y, dt_s) {
        return Err(StepReject::DtAboveStateCeiling);
    }
    let (landed, integral) = integrate(&terms, state.inlet_temperature_k, state.t_s, &y, dt_s);
    let mut next = start;
    unpack(&mut next, &landed);
    next.t_s = state.t_s + dt_s;
    next.reactivity_power_integral = state.reactivity_power_integral + integral;
    if !(landed.iter().all(|v| v.is_finite())) || !(next.power_mw > 0.0) {
        return Err(StepReject::NonFiniteState);
    }
    if above_state_ceiling(&terms, next.t_s, &landed, dt_s) {
        return Err(StepReject::DtAboveStateCeiling);
    }
    if next.peak_fuel_k.iter().any(|t| !(*t < UO2_MELTING_POINT_K)) {
        return Err(StepReject::FuelMelting);
    }
    if !(next.water_k < terms.saturation_k) {
        return Err(StepReject::WaterAtSaturation);
    }
    if next.power_mw > next.peak_power_mw {
        next.peak_power_mw = next.power_mw;
        next.time_of_peak_s = next.t_s;
        next.energy_to_peak_mws = next.energy_released_mws;
    }
    if next.hot_clad_k > next.hot_clad_max_k {
        next.hot_clad_max_k = next.hot_clad_k;
        next.time_of_hot_clad_max_s = next.t_s;
    }
    derive(&terms, &mut next);
    if !finite_state(&next) {
        return Err(StepReject::NonFiniteState);
    }
    if !(next.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT) {
        let created = next.population_defect_mw > 0.0 || next.energy_defect_mws < 0.0 || next.hot_energy_defect_mws < 0.0 || next.peak_energy_defect_mws < 0.0;
        return Err(if created { StepReject::SpontaneousGeneration } else { StepReject::FirstLawBreach });
    }
    Ok(next)
}

fn above_state_ceiling(terms: &Terms, t_s: f64, y: &[f64; NY], dt_s: f64) -> bool {
    let (_, _, rho, _) = reactivity(terms, t_s, y, None);
    let share = if rho > 1.0 { GROWTH_PHASE_STEP } else { DECAY_PHASE_STEP };
    !(dt_s * (rho - 1.0).abs() / terms.l_star <= share)
}

fn state_inside(state: &ReactorState) -> bool {
    let temps_positive = state.fuel_k.iter().chain(state.hot_fuel_k.iter()).chain(state.peak_fuel_k.iter()).all(|t| *t > 0.0)
        && state.clad_k > 0.0
        && state.hot_clad_k > 0.0
        && state.peak_clad_k > 0.0
        && state.water_k > 0.0
        && state.start_fuel_average_k > 0.0
        && state.start_water_k > 0.0;
    temps_positive
        && state.power_mw > 0.0
        && state.t_s >= 0.0
        && state.initial_power_mw > 0.0
        && state.coolant_flow_m_per_s >= 0.0
        && state.system_pressure_mpa > PRESSURE_MIN_MPA
        && state.system_pressure_mpa < PRESSURE_MAX_MPA
        && state.inlet_temperature_k > 273.15
        && state.inlet_temperature_k < saturation_temperature_k(state.system_pressure_mpa)
        && state.precursor_mw.iter().all(|c| *c >= 0.0)
        && state.energy_released_mws >= 0.0
}

fn finite_state(state: &ReactorState) -> bool {
    let s = state;
    [
        s.t_s,
        s.inlet_temperature_k,
        s.system_pressure_mpa,
        s.coolant_flow_m_per_s,
        s.initial_power_mw,
        s.start_fuel_average_k,
        s.start_water_k,
        s.reactivity_inserted_dollar,
        s.power_mw,
        s.clad_k,
        s.hot_clad_k,
        s.water_k,
        s.energy_released_mws,
        s.heat_carried_out_mws,
        s.hot_heat_to_water_mws,
        s.peak_clad_k,
        s.peak_heat_to_water_mws,
        s.peak_heat_anchor_mws,
        s.peak_fuel_centre_k,
        s.peak_energy_defect_mws,
        s.population_anchor_mw,
        s.heat_anchor_mws,
        s.hot_heat_anchor_mws,
        s.reactivity_power_integral,
        s.inserted_reactivity_dollar,
        s.doppler_dollar,
        s.moderator_dollar,
        s.net_reactivity_dollar,
        s.rod_position_in,
        s.fuel_average_k,
        s.hot_fuel_centre_k,
        s.saturation_k,
        s.hot_clad_heat_flux_mw_per_m2,
        s.population_defect_mw,
        s.energy_defect_mws,
        s.hot_energy_defect_mws,
        s.residual_conservation_error,
        s.peak_power_mw,
        s.time_of_peak_s,
        s.energy_to_peak_mws,
        s.hot_clad_max_k,
        s.time_of_hot_clad_max_s,
        s.fuel_melt_margin_k,
    ]
    .iter()
    .chain(s.precursor_mw.iter())
    .chain(s.fuel_k.iter())
    .chain(s.hot_fuel_k.iter())
    .chain(s.peak_fuel_k.iter())
    .all(|v| v.is_finite())
}

fn publish(state: &ReactorState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.power_mw as f32;
    frame.pos[1] = state.energy_released_mws as f32;
    frame.pos[2] = state.net_reactivity_dollar as f32;
    frame.vel[0] = state.fuel_average_k as f32;
    frame.vel[1] = state.hot_clad_k as f32;
    frame.vel[2] = state.water_k as f32;
    frame.force_torque = state.fuel_melt_margin_k as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.rod_moving {
        flags |= FLAG_ROD_MOVING;
    }
    if state.past_peak {
        flags |= FLAG_PAST_PEAK;
    }
    if state.hot_spot_boiling {
        flags |= FLAG_HOT_SPOT_BOILING;
    }
    if state.prompt_supercritical {
        flags |= FLAG_PROMPT_SUPERCRITICAL;
    }
    if state.outside_compared {
        flags |= FLAG_OUTSIDE_COMPARED;
    }
    frame.flags = flags;
}

/// The inhour reactivity of a stable period, dollars, at the inlet temperature: `l* ω + Σ b_i ω / (ω + λ_i)`.
#[must_use]
pub fn inhour_dollar(inlet_temperature_k: f64, period_s: f64) -> f64 {
    let omega = 1.0 / period_s;
    let mut rho = table(inlet_temperature_k, &GENERATION_TIME_REDUCED_S) * omega;
    for i in 0..6 {
        rho += GROUP_FRACTION[i] * omega / (omega + GROUP_DECAY_PER_S[i]);
    }
    rho
}

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    ROD_AFTER_DELAY,
    ROD_BEFORE_END,
    PEAK_KIND,
    BOILING_KIND,
    PROMPT_KIND,
    DISTANCE_KIND,
];

const ROD_AFTER_DELAY: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_ROD_MOVING",
    FLAG_ROD_MOVING,
    "t_s",
    "s",
    Side::Above,
    ROD_DELAY_S,
    "the rod moves from its delay after the start to the time it is out: the time it starts",
);
const ROD_BEFORE_END: ExceedanceKind = ExceedanceKind::varying(
    "FLAG_ROD_MOVING",
    FLAG_ROD_MOVING,
    "t_s",
    "s",
    Side::Below,
    "the rod moves from its delay after the start to the time it is out: the time it is out, which the inserted reactivity sets",
);
const PEAK_KIND: ExceedanceKind = ExceedanceKind::varying(
    "FLAG_PAST_PEAK",
    FLAG_PAST_PEAK,
    "power_mw",
    "MW",
    Side::Below,
    "the largest power so far (peak_power_mw): the extent is the fall from it",
);
const BOILING_KIND: ExceedanceKind = ExceedanceKind::varying(
    "FLAG_HOT_SPOT_BOILING",
    FLAG_HOT_SPOT_BOILING,
    "hot_clad_k",
    "K",
    Side::Above,
    "the saturation temperature of the water at the system pressure (saturation_k)",
);
const PROMPT_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_PROMPT_SUPERCRITICAL",
    FLAG_PROMPT_SUPERCRITICAL,
    "net_reactivity_dollar",
    "dollar",
    Side::Above,
    1.0,
    "one dollar of net reactivity: prompt critical",
);
const DISTANCE_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_COMPARED",
    FLAG_OUTSIDE_COMPARED,
    "compared_distance",
    "1",
    Side::Above,
    1.0,
    "one unit of distance from the nearest of the 70 tested conditions (compared_distance): 25 K of inlet temperature, a factor of 2 in pressure, 1.5 m/s of flow, a factor of 10 in initial power or 0.1 dollar of reactivity",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The conditions are those `derive` writes the five flags' booleans by.
pub fn exceedances(
    card: &ReactorSpert3Twin,
    before: &ReactorState,
    after: &ReactorState,
    load: ReactorLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = dt_s;
    let mut records = crate::exceedance::Exceedances::new();
    let s = after;
    if s.rod_moving {
        let mut start = *before;
        start.reactivity_inserted_dollar = load.reactivity_inserted_dollar;
        start.reactivity_recorded = true;
        let rod_end_s = terms(card, &start, load.reactivity_inserted_dollar).rod_end_s;
        records.push(Exceedance::at(ROD_AFTER_DELAY, card.rod_delay_s, s.t_s));
        records.push(Exceedance::at(ROD_BEFORE_END, rod_end_s, s.t_s));
    }
    records.push_if(s.past_peak, Exceedance::at(PEAK_KIND, s.peak_power_mw, s.power_mw));
    records.push_if(s.hot_spot_boiling, Exceedance::at(BOILING_KIND, s.saturation_k, s.hot_clad_k));
    records.push_if(s.prompt_supercritical, Exceedance::fixed(PROMPT_KIND, s.net_reactivity_dollar));
    if s.outside_compared {
        let distance = compared_distance(
            s.inlet_temperature_k,
            s.system_pressure_mpa,
            s.coolant_flow_m_per_s,
            s.initial_power_mw,
            if s.reactivity_recorded { Some(s.reactivity_inserted_dollar) } else { None },
        );
        records.push(Exceedance::fixed(DISTANCE_KIND, distance));
    }
    records
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::*;

    fn blank() -> LastStateFrame64 {
        LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [7; 16] }
    }

    fn f_to_k(f: f64) -> f64 {
        (f + 459.67) / 1.8
    }

    /// Runs to `t_end_s` at the ceiling, or less where the state ceiling asks; returns the last state.
    fn run(card: &ReactorSpert3Twin, mut state: ReactorState, rho: f64, t_end_s: f64) -> (ReactorState, f64) {
        let gate = card.gate_dt_s().unwrap();
        let mut frame = blank();
        let mut worst: f64 = 0.0;
        while state.t_s < t_end_s {
            let r = state.net_reactivity_dollar - 1.0;
            let share = if r > 0.0 { GROWTH_PHASE_STEP } else { DECAY_PHASE_STEP };
            let dt = gate.min(0.8 * share * GENERATION_TIME_REDUCED_S[0] / r.abs().max(1e-9));
            step(card, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: rho }, dt).unwrap();
            worst = worst.max(state.residual_conservation_error);
            assert_eq!(frame.proof, [7; 16]);
        }
        (state, worst)
    }

    #[test]
    fn the_card_holds_its_constants_and_a_moved_one_is_refused() {
        let card = ReactorSpert3Twin::e_core();
        assert!(card.is_valid());
        let mut moved = card;
        moved.doppler_weight = f64::from_bits(DOPPLER_WEIGHT.to_bits() + 1);
        assert!(!moved.is_valid());
        let mut state = ReactorState::try_start(&card, f_to_k(70.0), 0.0853, 0.0, 1.17719e-5).unwrap();
        let before = state;
        let mut frame = blank();
        assert_eq!(step(&moved, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.0 }, 1e-4), Err(StepReject::InvalidSpecimen));
        assert_eq!(state, before);
    }

    #[test]
    fn the_saturation_temperature_is_iapws_if97() {
        assert!((saturation_temperature_k(0.1) - 372.755_919_1).abs() < 1e-6);
        assert!((saturation_temperature_k(1.0) - 453.035_632_4).abs() < 1e-6);
        assert!((saturation_temperature_k(10.0) - 584.149_488_0).abs() < 1e-6);
    }

    #[test]
    fn the_compared_flag_marks_distance_from_the_tested_conditions_not_their_box() {
        // every tested condition is at distance zero
        for c in COMPARED_CONDITIONS {
            assert_eq!(compared_distance(c[0], c[1], c[2], c[3], Some(c[4])), 0.0);
        }
        // inside the box, far from every test: 350 degF, 5 MPa, no flow, 5 MW (the V&V report's point)
        let far = compared_distance(f_to_k(350.0), 5.0, 0.0, 5.0, Some(1.0));
        assert!(far > 1.0, "{far}");
        let card = ReactorSpert3Twin::e_core();
        let mut state = ReactorState::try_start(&card, f_to_k(350.0), 5.0, 0.0, 5.0).unwrap();
        let mut frame = LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [0; 16] };
        step(&card, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.0 }, 1.0e-5).unwrap();
        assert!(frame.flags & FLAG_OUTSIDE_COMPARED != 0);
        // near a test: test 81's conditions, 2 K warmer and 0.03 dollar more, stay compared
        let near = compared_distance(f_to_k(504.0) + 2.0, 10.44, 14.0 * 0.3048, 0.9, Some(1.20));
        assert!(near < 1.0, "{near}");
        // a cold start of one dollar from 50 W, as the worked example, is compared
        assert!(compared_distance(f_to_k(70.0), 0.0853, 0.0, 5.0e-5, Some(1.0)) <= 1.0);
    }

    #[test]
    fn the_rod_worth_curve_starts_at_the_rods_start_and_ends_at_the_whole_insertion() {
        let card = ReactorSpert3Twin::e_core();
        let state = ReactorState::try_start(&card, f_to_k(70.0), 0.0853, 0.0, 1e-5).unwrap();
        let t = terms(&card, &state, 1.21);
        assert!((rod_worth(&t, t.rod_start_in) - 1.21).abs() < 1e-12);
        assert_eq!(inserted(&t, 0.0).0, 0.0);
        assert_eq!(inserted(&t, t.rod_end_s + 1e-6).0, 1.21);
        // Fig. B-4 at 70 degF: about 0.78 dollar at 3.9 in.
        assert!((rod_worth(&t, 3.9) - 0.79).abs() < 0.02);
    }

    #[test]
    fn a_steady_start_at_19_mw_stays_steady_with_no_insertion() {
        let card = ReactorSpert3Twin::e_core();
        let start = ReactorState::try_start(&card, f_to_k(505.0), 10.44, 14.0 * 0.3048, 19.0).unwrap();
        assert!(start.fuel_average_k > 600.0 && start.fuel_average_k < 1100.0, "{}", start.fuel_average_k);
        let (end, worst) = run(&card, start, 0.0, 0.2);
        assert!((end.power_mw / 19.0 - 1.0).abs() < 1e-6, "{}", end.power_mw);
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT);
    }

    #[test]
    fn a_cold_one_dollar_burst_peaks_and_closes_its_ledgers() {
        let card = ReactorSpert3Twin::e_core();
        let start = ReactorState::try_start(&card, f_to_k(76.0), 0.0853, 0.0, 1.17719e-5).unwrap();
        let (end, worst) = run(&card, start, 1.0, 1.5);
        assert!(end.past_peak);
        assert!(end.peak_power_mw > 5.0 && end.peak_power_mw < 20.0, "{}", end.peak_power_mw);
        assert!(end.time_of_peak_s > 0.7 && end.time_of_peak_s < 1.2, "{}", end.time_of_peak_s);
        assert!(worst <= RESIDUAL_CONSERVATION_LIMIT, "{worst}");
        assert!(end.hot_clad_max_k > end.inlet_temperature_k);
    }

    #[test]
    fn a_second_reactivity_and_a_negative_one_are_refused_with_the_state_whole() {
        let card = ReactorSpert3Twin::e_core();
        let mut state = ReactorState::try_start(&card, f_to_k(70.0), 0.0853, 0.0, 1e-5).unwrap();
        let mut frame = blank();
        let before = state;
        assert_eq!(step(&card, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: -0.1 }, 1e-4), Err(StepReject::ReactivityNegative));
        assert_eq!(state, before);
        step(&card, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.0 }, 1e-4).unwrap();
        let landed = state;
        assert_eq!(step(&card, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.1 }, 1e-4), Err(StepReject::ReactivityChangedDuringRun));
        assert_eq!(state, landed);
    }

    #[test]
    fn a_falsified_ledger_is_named() {
        let card = ReactorSpert3Twin::e_core();
        let mut state = ReactorState::try_start(&card, f_to_k(70.0), 0.0853, 0.0, 1e-5).unwrap();
        let mut frame = blank();
        step(&card, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.0 }, 1e-4).unwrap();
        let mut neutrons = state;
        neutrons.population_anchor_mw *= 0.5;
        assert_eq!(step(&card, &mut neutrons, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.0 }, 1e-4), Err(StepReject::SpontaneousGeneration));
        let mut heat = state;
        heat.heat_anchor_mws += 1.0;
        assert!(step(&card, &mut heat, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.0 }, 1e-4).is_err());
    }

    #[test]
    fn the_peak_rod_runs_at_the_cores_own_peaking_and_the_hot_rod_at_the_clad_thermocouples() {
        // test 86's start, 502 degF, 19 MW: the peak rod at PF = 3.6 has its centre at 1942 K, the
        // hot rod at 1.43 PF, the clad thermocouple's place, far hotter; the margin to melting is the peak rod's
        let card = ReactorSpert3Twin::e_core();
        let state = ReactorState::try_start(&card, f_to_k(502.0), 10.44, 14.0 * 0.3048, 19.0).unwrap();
        assert!((state.peak_fuel_centre_k - 1942.4).abs() < 0.5, "{}", state.peak_fuel_centre_k);
        assert!(state.hot_fuel_centre_k > state.peak_fuel_centre_k + 500.0, "{}", state.hot_fuel_centre_k);
        assert_eq!(state.fuel_melt_margin_k, UO2_MELTING_POINT_K - state.peak_fuel_k[0].max(state.peak_fuel_k[1]));
        // its heat ledger closes with the others at the start
        assert!(state.peak_energy_defect_mws.abs() < 1.0e-9, "{}", state.peak_energy_defect_mws);
    }

    #[test]
    fn the_hot_rod_melting_is_refused_by_name() {
        let card = ReactorSpert3Twin::e_core();
        let mut state = ReactorState::try_start(&card, f_to_k(505.0), 10.44, 14.0 * 0.3048, 19.0).unwrap();
        let mut frame = blank();
        let load = ReactorLoad { reactivity_inserted_dollar: 4.0 };
        let mut named = None;
        for _ in 0..200_000 {
            let r = state.net_reactivity_dollar - 1.0;
            let share = if r > 0.0 { GROWTH_PHASE_STEP } else { DECAY_PHASE_STEP };
            let dt = card.gate_dt_s().unwrap().min(0.8 * share * GENERATION_TIME_REDUCED_S[0] / r.abs().max(1e-9));
            let before = state;
            match step(&card, &mut state, &mut frame, load, dt) {
                Ok(()) => {}
                Err(reject) => {
                    assert_eq!(state, before);
                    named = Some(reject);
                    break;
                }
            }
        }
        assert_eq!(named, Some(StepReject::FuelMelting));
    }

    #[test]
    fn the_same_run_gives_the_same_bits() {
        let card = ReactorSpert3Twin::e_core();
        let go = || {
            let start = ReactorState::try_start(&card, f_to_k(500.0), 10.44, 14.0 * 0.3048, 1.24711e-5).unwrap();
            let mut frame = blank();
            let mut state = start;
            for _ in 0..200 {
                step(&card, &mut state, &mut frame, ReactorLoad { reactivity_inserted_dollar: 1.15 }, 4e-4).unwrap();
            }
            frame.to_bytes()
        };
        assert_eq!(go(), go());
    }
}
