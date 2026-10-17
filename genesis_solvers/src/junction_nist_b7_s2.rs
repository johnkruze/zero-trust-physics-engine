//! One NIST niobium / amorphous-silicon / niobium Josephson junction,
//! D220831b-B7_S2_1.5, at the bath temperature of its measurement: its
//! current-voltage characteristic, and its Shapiro steps under a microwave
//! drive.
//!
//! The junction was made and measured at NIST Boulder: sputtered niobium
//! electrodes, an amorphous-silicon barrier, a nominal diameter of 1.5 um,
//! measured at 4 K (the paper's abstract) at zero applied field. Data:
//! Pete Hopkins (2023), "Nb/a-Si/Nb Josephson junctions for high-density
//! superconducting circuits", NIST Public Data Repository,
//! doi 10.18434/mds2-2921, the numbers behind Olaya, Biesecker,
//! Castellanos-Beltran, Sirois, Hopkins, Dresselhaus and Benz, Applied
//! Physics Letters 122, 182601 (2023), doi 10.1063/5.0148250. NIST's data are
//! not subject to copyright in the United States (17 U.S.C. 105).
//!
//! The model is the shunted junction with its capacitance, stepped in time
//! under a held bias current (and a microwave current when one is asked):
//!
//! ```text
//! phase        d(phase)/dt = (2e/hbar) V
//! voltage      C dV/dt     = I_b(t) - I_d(V) - I_c sin(phase)
//! bias         I_b(t)      = I_dc + I_ac sin(2 pi f t)
//! dissipation  I_d(V)      straight lines between the 199 knots of the
//!                          table; I_d(-V) = -I_d(V); above the last knot,
//!                          the line of slope 1 / R_n through it, R_n the
//!                          junction's normal resistance
//! stored       E           = 1/2 C V^2 + E_J (1 - cos(phase)),  E_J = hbar I_c / 2e
//! source       dW_s/dt     = I_b(t) V
//! dissipated   dW_d/dt     = I_d(V) V
//! ledger       E - E_start = W_s - W_d
//! flux         phase - phase_start = (2e/hbar) * integral of V dt
//! ```
//!
//! A static point of the characteristic is made as the validation protocol
//! of the research record fixed it, before any comparison: the bias is held,
//! the motion settles for (60 + 12 Q) plasma periods, Q being the quality of
//! the small-voltage motion (plasma rate x capacitance x the table's chord
//! resistance at 0.02 of the critical current), and the mean voltage is read
//! over 30 more plasma periods as (hbar/2e) x 2 pi x whole turns of the phase
//! / the time those turns took: each whole turn of the phase is one flux
//! quantum, h/2e, through the junction. A point starts at rest (zero
//! voltage, the phase where the supercurrent carries the bias, or pi/2 above
//! the critical current: the branch a rising sweep reaches) or running (a
//! voltage above the running motion: the branch a falling sweep reaches).
//! Under a drive the mean is read over whole periods of the drive, and a
//! Shapiro step shows as a mean voltage of n h f / 2e.
//!
//! Where the numbers come from, all from the research record
//! `evidence/research/junction_nist_nb_asi_nb/`. One of them, the
//! capacitance, is fitted to two bias currents of the curve this solver was
//! compared with, the zero-field curve's return bracket:
//!
//! - The critical current, 0.298 mA, is stated in the headers of the
//!   junction's two files: the last bias at which the laboratory read zero
//!   voltage.
//! - The dissipative table is the junction's own field-suppressed curve,
//!   `Fig9b-IV-D220831b-B7_S2_1.5-Suppr.dat`, both sweeps, made
//!   single-valued (readings above 0.05 mV, averaged at one current, voltage
//!   made non-decreasing, every interval given a slope of at least 1 percent
//!   of the normal resistance, the origin put in front):
//!   `analysis/out/card_table_D220831b-B7_S2_1.5.csv`, SHA-256
//!   a5f8a100e72350567cb232e56bec56048657903a996f9b6ddb5f6a5db1775c1a. The
//!   knots are copied as decimals, not recomputed.
//! - The normal resistance, 4.816879604231975 ohm, is the voltage over the
//!   current at the top of that curve, 4.4255 mV at 0.91872 mA. Above the
//!   table the dissipative current rises as 1 / R_n from the last knot: well
//!   above the energy gap the quasiparticle current of a tunnel junction is
//!   ohmic, and this line is V / R_n to within 3e-8 A.
//! - The capacitance, 47.21 fF, is fitted: the value at which this model's
//!   return current falls in the middle of the junction's measured return
//!   bracket (0.05567 to 0.06494 mA), two bias currents of the zero-field
//!   curve. No published capacitance was found; a parallel-plate estimate
//!   gives 36.6 fF.
//! - The gap voltage, 2.8602516 mV, is the table's voltage where its plateau
//!   begins (knot 33, at 0.1575 mA): the sum of the two electrodes' gaps over
//!   e. Its frequency, e V_g / h, 691.6 GHz, is where a photon of the drive
//!   breaks a Cooper pair.
//! - e and h are the exact SI values.
//!
//! The comparison. Once, on 2026-10-14, through the engine, with the voltages
//! of the zero-field curve of the junction held back since 2026-10-06
//! (`Fig9a-IV-D220831b-B7_S2_1.5.dat`), but for what the inventory showed
//! before the protocol was fixed: its first two readings near zero bias and
//! its largest current and voltage. By the pass line fixed then: voltage error
//! at most 5 percent of I_c R_n rms on both sweeps, switching and return
//! current within one bias step. It missed the voltage line, 11.84 percent
//! (rising 7.10, falling 15.13), and met the two current lines, which are its
//! inputs. The forecast made before, outside the engine on three sibling
//! junctions, was a miss of 10.5 to 16.2 percent. The numbers are in
//! [`COMPARED_RESULT`] and the record's `RESULT.md`.
//!
//! The junction's bath is held at the measurement temperature: the model has
//! no self-heating, no thermal noise and no magnetic field, and its
//! dissipative current follows the voltage of the instant: the table is the
//! junction's dc curve, quasi-static, which holds where h f/e is small
//! against the 22 uV over which the table's current steps up at the gap
//! (2.8603 to 2.8826 mV, 0.158 to 0.622 mA), f well below 5.4 GHz. The
//! research record's reading of the forecast is that this is what the
//! junction departs from most: on the plateau the model's voltage ripples at
//! the Josephson frequency, 1.4 THz, twice the gap frequency (h f/e 5.7 mV;
//! the junction's own ac Josephson frequency passes the gap frequency above
//! V_g/2, 1.430 mV), where a tunnel junction's quasiparticle current has a
//! memory of the past voltage and a drive's photons spread the step over
//! n h f/e (Tien and Gordon). A drive's frequencies lie far above 5.4 GHz
//! too: its steps' voltages, n h f/2e, are constants of nature, their widths
//! the quasi-static table's.
//!
//! The limits are of the physics and the arithmetic, and one budget. A drive
//! of nonzero amplitude at or above the gap frequency breaks pairs, and is
//! refused by name. The time step is
//! the one the ledger holds: an eighth of the shortest RC time of the table
//! (the steepest interval, 0.0479 ohm, times the capacitance), 2.83e-16 s,
//! and finer where the voltage could pass the top of the table, so that the
//! phase never moves more than 0.0038 rad in one step. A question whose
//! steps, counted before it starts, pass the budget of one solve is refused
//! before it starts: a budget of computing, stated as one. A point whose
//! energy ledger or flux accounting breaks, or whose mean voltage does not
//! settle, is refused by name, with no curve; with no drive and a bias above
//! the critical current, where no state at rest exists, a window whose phase
//! made no whole turn is not settled however alike its halves.
//!
//! Every number is SI: amperes, volts, farads, joules, seconds, hertz.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::fmt;

/// The elementary charge, C (exact, SI 2019).
pub const ELEMENTARY_CHARGE_C: f64 = 1.602176634e-19;
/// The Planck constant, J s (exact, SI 2019).
pub const PLANCK_J_S: f64 = 6.62607015e-34;
/// 2e / hbar = 4 pi e / h: the rate of the phase per volt, rad per V s.
pub const PHASE_RATE_RAD_PER_V_S: f64 =
    4.0 * std::f64::consts::PI * ELEMENTARY_CHARGE_C / PLANCK_J_S;
/// The flux quantum h / 2e, Wb: one whole turn of the phase, as a time
/// integral of the voltage.
pub const FLUX_QUANTUM_WB: f64 = PLANCK_J_S / (2.0 * ELEMENTARY_CHARGE_C);

/// The junction's critical current, A: stated in the headers of both of its
/// files.
pub const CRITICAL_CURRENT_A: f64 = 2.98e-4;
/// The junction's capacitance, F: fitted to the middle of its measured return
/// bracket. See the header.
pub const CAPACITANCE_F: f64 = 4.721174815743865e-14;
/// The junction's normal resistance, ohm: voltage over current at the top of
/// its field-suppressed curve.
pub const NORMAL_RESISTANCE_OHM: f64 = 4.816879604231975;
/// Knots of the dissipative table, the origin first.
pub const TABLE_KNOTS: usize = 199;
/// The knot where the table's plateau begins: the gap voltage.
pub const GAP_KNOT: usize = 33;

/// Dissipative current at each knot, amperes: column 2 of
/// `card_table_D220831b-B7_S2_1.5.csv`, copied, not recomputed.
pub const TABLE_CURRENT_A: [f64; TABLE_KNOTS] = [
    0.000000000e+00, 9.031500000e-06, 9.280000000e-06, 1.831150000e-05,
    1.856000000e-05, 2.759150000e-05, 2.784000000e-05, 3.687150000e-05,
    3.712000000e-05, 4.615150000e-05, 4.640000000e-05, 5.543150000e-05,
    5.568000000e-05, 6.471150000e-05, 6.496000000e-05, 7.399150000e-05,
    7.424000000e-05, 8.327150000e-05, 8.352000000e-05, 9.255150000e-05,
    9.280000000e-05, 1.018315000e-04, 1.020800000e-04, 1.111115000e-04,
    1.113600000e-04, 1.203915000e-04, 1.206400000e-04, 1.296715000e-04,
    1.299200000e-04, 1.389515000e-04, 1.392000000e-04, 1.482315000e-04,
    1.484800000e-04, 1.575115000e-04, 1.577600000e-04, 1.667915000e-04,
    1.670400000e-04, 1.760715000e-04, 1.763200000e-04, 1.853515000e-04,
    1.856000000e-04, 1.946315000e-04, 1.948800000e-04, 2.039115000e-04,
    2.041600000e-04, 2.131915000e-04, 2.134400000e-04, 2.224715000e-04,
    2.227200000e-04, 2.317515000e-04, 2.320000000e-04, 2.410315000e-04,
    2.412800000e-04, 2.503115000e-04, 2.505600000e-04, 2.595915000e-04,
    2.598400000e-04, 2.688715000e-04, 2.691200000e-04, 2.781515000e-04,
    2.784000000e-04, 2.874315000e-04, 2.876800000e-04, 2.967115000e-04,
    2.969600000e-04, 3.059915000e-04, 3.062400000e-04, 3.152715000e-04,
    3.155200000e-04, 3.245515000e-04, 3.248000000e-04, 3.338315000e-04,
    3.340800000e-04, 3.431115000e-04, 3.433600000e-04, 3.523915000e-04,
    3.526400000e-04, 3.616715000e-04, 3.619200000e-04, 3.709515000e-04,
    3.712000000e-04, 3.802315000e-04, 3.804800000e-04, 3.895115000e-04,
    3.897600000e-04, 3.987915000e-04, 3.990400000e-04, 4.080715000e-04,
    4.083200000e-04, 4.173515000e-04, 4.176000000e-04, 4.266315000e-04,
    4.268800000e-04, 4.359115000e-04, 4.361600000e-04, 4.451915000e-04,
    4.454400000e-04, 4.544715000e-04, 4.547200000e-04, 4.637515000e-04,
    4.640000000e-04, 4.730315000e-04, 4.732800000e-04, 4.823115000e-04,
    4.825600000e-04, 4.915915000e-04, 4.918400000e-04, 5.008715000e-04,
    5.011200000e-04, 5.101515000e-04, 5.104000000e-04, 5.194315000e-04,
    5.196800000e-04, 5.287115000e-04, 5.289600000e-04, 5.379915000e-04,
    5.382400000e-04, 5.472715000e-04, 5.475200000e-04, 5.565515000e-04,
    5.568000000e-04, 5.658315000e-04, 5.660800000e-04, 5.751115000e-04,
    5.753600000e-04, 5.843915000e-04, 5.846400000e-04, 5.936715000e-04,
    5.939200000e-04, 6.029515000e-04, 6.032000000e-04, 6.122315000e-04,
    6.124800000e-04, 6.215115000e-04, 6.217600000e-04, 6.307915000e-04,
    6.310400000e-04, 6.400715000e-04, 6.403200000e-04, 6.493515000e-04,
    6.496000000e-04, 6.586315000e-04, 6.588800000e-04, 6.679115000e-04,
    6.681600000e-04, 6.771915000e-04, 6.774400000e-04, 6.864715000e-04,
    6.867200000e-04, 6.957515000e-04, 6.960000000e-04, 7.050315000e-04,
    7.052800000e-04, 7.143115000e-04, 7.145600000e-04, 7.235915000e-04,
    7.238400000e-04, 7.328715000e-04, 7.331200000e-04, 7.421515000e-04,
    7.424000000e-04, 7.514315000e-04, 7.516800000e-04, 7.607115000e-04,
    7.609600000e-04, 7.699915000e-04, 7.702400000e-04, 7.792715000e-04,
    7.795200000e-04, 7.885515000e-04, 7.888000000e-04, 7.978315000e-04,
    7.980800000e-04, 8.071115000e-04, 8.073600000e-04, 8.163915000e-04,
    8.166400000e-04, 8.256715000e-04, 8.259200000e-04, 8.349515000e-04,
    8.352000000e-04, 8.442315000e-04, 8.444800000e-04, 8.535115000e-04,
    8.537600000e-04, 8.627915000e-04, 8.630400000e-04, 8.720715000e-04,
    8.723200000e-04, 8.813515000e-04, 8.816000000e-04, 8.906315000e-04,
    8.908800000e-04, 8.999115000e-04, 9.001600000e-04, 9.091915000e-04,
    9.094400000e-04, 9.184715000e-04, 9.187200000e-04,
];

/// Voltage at each knot, volts: column 3 of the same file.
pub const TABLE_VOLTAGE_V: [f64; TABLE_KNOTS] = [
    0.000000000e+00, 4.855009000e-04, 4.906392000e-04, 1.148493400e-03,
    1.173461100e-03, 1.367699100e-03, 1.421396500e-03, 1.547354600e-03,
    1.551140500e-03, 1.760679600e-03, 1.762219200e-03, 1.983071900e-03,
    1.983083900e-03, 2.184288300e-03, 2.186244400e-03, 2.352220200e-03,
    2.368449200e-03, 2.505323400e-03, 2.506446500e-03, 2.612389900e-03,
    2.614648900e-03, 2.696866800e-03, 2.705599700e-03, 2.766301100e-03,
    2.766313100e-03, 2.804841900e-03, 2.806709600e-03, 2.829336900e-03,
    2.832088000e-03, 2.848639100e-03, 2.848651000e-03, 2.857037500e-03,
    2.857378200e-03, 2.860251600e-03, 2.860263600e-03, 2.860698600e-03,
    2.860710600e-03, 2.861145600e-03, 2.861157600e-03, 2.861592600e-03,
    2.861604600e-03, 2.862039600e-03, 2.862051600e-03, 2.862486600e-03,
    2.862498600e-03, 2.862933600e-03, 2.862945600e-03, 2.863380700e-03,
    2.863392600e-03, 2.863827700e-03, 2.863839600e-03, 2.864274700e-03,
    2.864286600e-03, 2.864721700e-03, 2.864733600e-03, 2.865168700e-03,
    2.865180600e-03, 2.865615700e-03, 2.865627700e-03, 2.866062700e-03,
    2.866074700e-03, 2.866509700e-03, 2.866521700e-03, 2.866956700e-03,
    2.866968700e-03, 2.867403700e-03, 2.867415700e-03, 2.867850700e-03,
    2.867862700e-03, 2.868297700e-03, 2.868309700e-03, 2.868744700e-03,
    2.868756700e-03, 2.869191700e-03, 2.869203700e-03, 2.869638700e-03,
    2.869650700e-03, 2.870085700e-03, 2.870097700e-03, 2.870532800e-03,
    2.870544700e-03, 2.870979800e-03, 2.870991700e-03, 2.871426800e-03,
    2.871438700e-03, 2.871873800e-03, 2.871885700e-03, 2.872320800e-03,
    2.872332700e-03, 2.872767800e-03, 2.872779800e-03, 2.873214800e-03,
    2.873226800e-03, 2.873661800e-03, 2.873673800e-03, 2.874108800e-03,
    2.874120800e-03, 2.874555800e-03, 2.874567800e-03, 2.875002800e-03,
    2.875014800e-03, 2.875449800e-03, 2.875461800e-03, 2.875896800e-03,
    2.875908800e-03, 2.876343800e-03, 2.876355800e-03, 2.876790800e-03,
    2.876802800e-03, 2.877237800e-03, 2.877249800e-03, 2.877684900e-03,
    2.877696800e-03, 2.878131900e-03, 2.878143800e-03, 2.878578900e-03,
    2.878590800e-03, 2.879025900e-03, 2.879037800e-03, 2.879472900e-03,
    2.879484900e-03, 2.879919900e-03, 2.879931900e-03, 2.880366900e-03,
    2.880378900e-03, 2.880813900e-03, 2.880825900e-03, 2.881260900e-03,
    2.881272900e-03, 2.881707900e-03, 2.881719900e-03, 2.882154900e-03,
    2.882166900e-03, 2.882601900e-03, 2.882613900e-03, 2.895313300e-03,
    2.914255500e-03, 3.078401400e-03, 3.081190300e-03, 3.150883100e-03,
    3.150895000e-03, 3.205804500e-03, 3.205816500e-03, 3.255318200e-03,
    3.261805000e-03, 3.305684000e-03, 3.306668300e-03, 3.351607400e-03,
    3.351619300e-03, 3.397732800e-03, 3.401796400e-03, 3.443933800e-03,
    3.445788900e-03, 3.487251400e-03, 3.487263400e-03, 3.530480500e-03,
    3.530492400e-03, 3.571608300e-03, 3.577804900e-03, 3.618806600e-03,
    3.618818500e-03, 3.661120700e-03, 3.661132600e-03, 3.701743900e-03,
    3.707599500e-03, 3.748670500e-03, 3.748682500e-03, 3.789117200e-03,
    3.789129100e-03, 3.830793900e-03, 3.831324000e-03, 3.873133300e-03,
    3.876894000e-03, 3.917347100e-03, 3.917359000e-03, 3.959124800e-03,
    3.959579100e-03, 3.999792300e-03, 3.999804200e-03, 4.044674500e-03,
    4.045570500e-03, 4.087203100e-03, 4.087215100e-03, 4.127321600e-03,
    4.127839000e-03, 4.169673500e-03, 4.174241900e-03, 4.214505700e-03,
    4.214517600e-03, 4.256624200e-03, 4.256990200e-03, 4.296805600e-03,
    4.296817500e-03, 4.338640300e-03, 4.342741700e-03, 4.383989500e-03,
    4.384001400e-03, 4.425483500e-03, 4.425495500e-03,
];

/// The gap voltage, V: the table's voltage where its plateau begins.
pub const GAP_VOLTAGE_V: f64 = TABLE_VOLTAGE_V[GAP_KNOT];
/// The gap frequency e V_g / h, Hz: a drive photon at or above it breaks a
/// Cooper pair.
pub const GAP_FREQUENCY_HZ: f64 = ELEMENTARY_CHARGE_C * GAP_VOLTAGE_V / PLANCK_J_S;
/// The Josephson energy hbar I_c / 2e, J.
pub const JOSEPHSON_ENERGY_J: f64 = CRITICAL_CURRENT_A / PHASE_RATE_RAD_PER_V_S;
/// The scale of the energy ledger, hbar I_c / e, J: the height of the
/// barrier at zero bias.
pub const ENERGY_SCALE_J: f64 = 2.0 * JOSEPHSON_ENERGY_J;
/// The characteristic voltage I_c R_n, V: the unit of the settle tolerance
/// and of the compared voltage error.
pub const CHARACTERISTIC_VOLTAGE_V: f64 = CRITICAL_CURRENT_A * NORMAL_RESISTANCE_OHM;

/// The largest |E - E_start - (W_s - W_d)| over [`ENERGY_SCALE_J`] a step
/// may land with.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// The largest departure, in flux quanta, of the phase's advance over a
/// window from (2e/hbar) times the integral of the voltage over it.
pub const FLUX_ACCOUNTING_LIMIT_QUANTA: f64 = 1.0e-6;
/// The step is the steepest table interval's RC time divided by this.
pub const STEPS_PER_STEEPEST_RC_TIME: f64 = 8.0;
/// Plasma periods of settling, before the quality term.
pub const SETTLE_PLASMA_PERIODS: f64 = 60.0;
/// Plasma periods of settling per unit of quality.
pub const SETTLE_PLASMA_PERIODS_PER_QUALITY: f64 = 12.0;
/// The fraction of the critical current at which the table's chord
/// resistance sets the quality.
pub const QUALITY_BIAS_FRACTION: f64 = 0.02;
/// Plasma periods over which the mean voltage is read, at least.
pub const AVERAGE_PLASMA_PERIODS: f64 = 30.0;
/// The mean voltages of the two halves of the averaging window must agree
/// within this fraction of [`CHARACTERISTIC_VOLTAGE_V`].
pub const SETTLE_TOLERANCE: f64 = 1.0e-3;
/// Averaging windows a point may take before it is refused as not settled.
/// Each is twice as long as the one before, and one that does not settle
/// counts as more settling.
pub const MAX_WINDOWS: u32 = 4;
/// The most steps one solve computes, counted before it starts with every
/// window of every point: a budget of computing, about 18 s on one core of
/// an Apple M4 Pro (117 ns a step with no drive, 147 under one, measured on
/// 2026-10-14), where a point whose first window settles takes about a third
/// of its count. Each request of the scored comparison of 2026-10-14 (30
/// points) fits.
pub const MAX_STEPS_PER_SOLVE: u64 = 120_000_000;

/// The most bias points one question holds: [`MAX_STEPS_PER_SOLVE`] over the
/// most steps of a point at zero bias with no drive.
pub const MAX_BIAS_POINTS: usize = 36;
/// The largest |I_dc| at which one point alone, with no drive, fits in one
/// solve's budget, A, exactly (a test holds it and the number above it):
/// above it the step the phase needs makes the point's count pass
/// [`MAX_STEPS_PER_SOLVE`]. A point under a drive counts at least as many
/// steps as one with no drive at |I_dc| + |I_ac|, so no |I_dc| + |I_ac|
/// above it is answered either.
pub const CURRENT_REACH_MAX_A: f64 = 0.03345626617838853;

/// The comparison with the held-back zero-field curve, as scored. Copied
/// from the research record's `RESULT.md`.
pub const COMPARED_RESULT: &str = "Compared once, on 2026-10-14, through the engine, with the zero-field curve of the junction held back since 2026-10-06 (Fig9a-IV-D220831b-B7_S2_1.5.dat, 100 bias points rising from rest and 100 falling from the voltage state, 0 to 0.91847 mA), and by the rule fixed beforehand the comparison failed: the voltage line was missed and the two current lines were met. Voltage error, rms over the 175 points scored, 11.84 percent of I_c R_n against a mark of 5 (rising sweep 7.10, falling 15.13; mean -1.22, worst -36.3 at 0.1113 mA falling); with the 25 points whose voltage rippled past the table's top included, 11.11 percent. Switching current 0.30616 mA and return current 0.06494 mA, each on the measured bias step: met, and they are the solver's inputs (the stated critical current, and the return bracket the capacitance was fitted to). The forecast made before, on three sibling junctions outside the engine, was a miss of 10.5 to 16.2 percent. Where it misses: the measured plateau is flat, 2.8498 to 2.8651 mV from 0.33 to 0.57 mA, and the solver's rises from 2.826 to 3.096 mV (+16.8 percent at 0.566 mA rising, +19.5 at 0.594 mA falling); below the critical current on the falling sweep it reads low, by up to 36 percent; above 0.64 mA it is within -8.5 to +3.4 percent. Every proof held, the largest energy residual was 7.3e-8 of hbar I_c / e, and every point settled in its first window. The record is evidence/research/junction_nist_nb_asi_nb/RESULT.md.";

/// The published source the tests compare with.
pub const REFERENCE_NAME: &str = "Pete Hopkins (2023), Nb/a-Si/Nb Josephson junctions for high-density superconducting circuits, NIST Public Data Repository, doi 10.18434/mds2-2921: the field-suppressed and zero-field current-voltage curves of junction D220831b-B7_S2_1.5 (Fig9b and Fig9a)";
pub const REFERENCE_URL: &str = "https://doi.org/10.18434/mds2-2921";
pub const REFERENCE_RETRIEVED: &str = "2026-10-06";

/// Where each point's motion starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// At zero voltage, the phase where the supercurrent carries the bias,
    /// asin(I_dc / I_c), or pi/2 with the sign of the bias above the critical
    /// current: the branch a sweep rising from zero reaches.
    Rest,
    /// At a voltage above the running motion, phase zero, with the sign of
    /// the bias: the branch a sweep falling from the voltage state reaches.
    Running,
}

/// The microwave current added to the held bias.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drive {
    /// No drive: the bias is held.
    None,
    /// I_ac sin(2 pi f t) added to the bias, t from the start of each point.
    Microwave {
        current_amplitude_a: f64,
        frequency_hz: f64,
    },
}

/// One question: the points of a characteristic, how each starts, and the
/// drive.
#[derive(Clone, Debug, PartialEq)]
pub struct JunctionQuestion {
    /// The held bias current of each point, A. Each point is computed on its
    /// own, from its own start.
    pub bias_current_a: Vec<f64>,
    pub start: Start,
    pub drive: Drive,
}

/// The characteristic: one value of each list per point, in the order asked.
#[derive(Clone, Debug, PartialEq)]
pub struct JunctionCurve {
    pub bias_current_a: Vec<f64>,
    /// Mean voltage, V: (hbar/2e) 2 pi whole turns / their time, or over
    /// whole drive periods under a drive.
    pub mean_voltage_v: Vec<f64>,
    /// Mean voltage over the flux quantum, Hz: the rate at which flux quanta
    /// cross the junction. Under a drive, a Shapiro step is a whole multiple
    /// of the drive frequency.
    pub josephson_frequency_hz: Vec<f64>,
    /// Turns of the phase the mean was read over: whole turns when at least
    /// one was made without a drive, otherwise the phase advance over 2 pi.
    pub phase_turns: Vec<f64>,
    /// The accepted window, s: the powers are means over it. With no drive
    /// the mean voltage is read over the whole turns inside it, a shorter
    /// time.
    pub averaging_time_s: Vec<f64>,
    /// Largest |voltage| landed in the accepted window, V.
    pub peak_voltage_v: Vec<f64>,
    /// Mean power the source delivered over the accepted window, W.
    pub source_power_w: Vec<f64>,
    /// Mean power dissipated over the accepted window, W.
    pub dissipated_power_w: Vec<f64>,
    /// 1 where a landed voltage of the accepted window was above the top of
    /// the table, so that the dissipation there followed the normal line;
    /// else 0.
    pub normal_line_reached: Vec<f64>,
    /// The time step of each point, s.
    pub time_step_s: Vec<f64>,
    /// The steps each point took, settling and windows together.
    pub steps: Vec<f64>,
    /// Largest |E - E_start - (W_s - W_d)| / [`ENERGY_SCALE_J`] over every
    /// landed step of every point.
    pub largest_energy_residual: f64,
    /// Largest departure of a window's phase advance from (2e/hbar) times
    /// its voltage integral, in flux quanta.
    pub largest_flux_residual_quanta: f64,
    /// Largest difference of the two half-window means of an accepted window,
    /// over [`CHARACTERISTIC_VOLTAGE_V`].
    pub largest_settle_difference: f64,
    /// Every step taken.
    pub total_steps: u64,
    /// The phase and the voltage each point ended at.
    pub final_phase_rad: Vec<f64>,
    pub final_voltage_v: Vec<f64>,
}

/// Why no characteristic was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum JunctionReject {
    /// An input was NaN or infinite.
    InputNotFinite { field: &'static str },
    /// The question holds no bias current.
    NoBiasPoints,
    /// The drive frequency is zero or negative.
    DriveFrequencyNotPositive { frequency_hz: f64 },
    /// The drive frequency is at or above the gap frequency [`GAP_FREQUENCY_HZ`], and its amplitude is not zero: a photon of the drive breaks a Cooper pair.
    DriveAtOrAboveGapFrequency { frequency_hz: f64 },
    /// The question needs more steps than [`MAX_STEPS_PER_SOLVE`], counted before it starts with every window of every point: the budget of computing one solve is given, a budget and not a limit of the physics.
    StepsBeyondSolveBudget { steps: f64 },
    /// A value of the motion was NaN or infinite.
    NonFiniteValue { point: usize },
    /// A step landed with the stored energy departing from the source's work less the dissipation by more than [`RESIDUAL_CONSERVATION_LIMIT`] of [`ENERGY_SCALE_J`].
    EnergyLedgerBroken { point: usize, step: u64, residual: f64 },
    /// The phase advance over a window departed from 2e/hbar times its voltage integral by more than [`FLUX_ACCOUNTING_LIMIT_QUANTA`] flux quanta.
    FluxAccountingBroken { point: usize, residual_quanta: f64 },
    /// After [`MAX_WINDOWS`] windows the two half-window means of a point still differed by more than [`SETTLE_TOLERANCE`] of the characteristic voltage, or, with no drive and a bias above the critical current, where no state at rest exists, its phase had not made a whole turn.
    NotSettled { point: usize, difference: f64, steps: u64 },
}

impl fmt::Display for JunctionReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotFinite { field } => write!(f, "{field} is not a finite number"),
            Self::NoBiasPoints => write!(f, "the question holds no bias current"),
            Self::DriveFrequencyNotPositive { frequency_hz } => {
                write!(f, "drive frequency {} Hz is not above zero", said(*frequency_hz))
            }
            Self::DriveAtOrAboveGapFrequency { frequency_hz } => write!(
                f,
                "drive frequency {} Hz is at or above the gap frequency {GAP_FREQUENCY_HZ} Hz (e times the gap voltage {GAP_VOLTAGE_V} V over h): a photon of the drive breaks a Cooper pair",
                said(*frequency_hz)
            ),
            Self::StepsBeyondSolveBudget { steps } => write!(
                f,
                "the question needs {}, more than the budget of {MAX_STEPS_PER_SOLVE} one solve computes, a budget of computing and not a limit of the physics; ask fewer points in each solve, or a bias and drive whose voltage stays nearer the table",
                if steps.is_finite() { format!("{} steps", said(*steps)) } else { "more steps than a 64-bit number holds".to_string() }
            ),
            Self::NonFiniteValue { point } => {
                write!(f, "a value of the motion of point {point} was not finite")
            }
            Self::EnergyLedgerBroken { point, step, residual } => write!(
                f,
                "at step {step} of point {point} the stored energy departed from the source's work less the dissipation by {} of hbar I_c / e, above {RESIDUAL_CONSERVATION_LIMIT}",
                said(*residual)
            ),
            Self::FluxAccountingBroken { point, residual_quanta } => write!(
                f,
                "over a window of point {point} the phase advance departed from 2e/hbar times the voltage integral by {} flux quanta, above {FLUX_ACCOUNTING_LIMIT_QUANTA}",
                said(*residual_quanta)
            ),
            Self::NotSettled { point, difference, steps } => write!(
                f,
                "after {MAX_WINDOWS} windows and {steps} steps point {point} had not settled: its two half-window means still differed by {} of I_c R_n (above {SETTLE_TOLERANCE}), or with no drive and its bias above the critical current, where no state at rest exists, its phase had not made a whole turn",
                said(*difference)
            ),
        }
    }
}

impl std::error::Error for JunctionReject {}

/// A number in a sentence: the shortest decimal that reads back to the same
/// 64-bit number, written with an exponent when it would otherwise run past
/// 24 characters.
fn said(value: f64) -> String {
    let plain = format!("{value}");
    if plain.len() > 24 { format!("{value:e}") } else { plain }
}

/// The steepest interval of the table, ohm: its smallest voltage step over
/// current step, past the origin.
pub fn steepest_interval_ohm() -> f64 {
    let mut least = f64::INFINITY;
    for k in 1..TABLE_KNOTS - 1 {
        let r = (TABLE_VOLTAGE_V[k + 1] - TABLE_VOLTAGE_V[k])
            / (TABLE_CURRENT_A[k + 1] - TABLE_CURRENT_A[k]);
        least = least.min(r);
    }
    least
}

/// The table's step, s: the steepest interval's RC time over
/// [`STEPS_PER_STEEPEST_RC_TIME`]. 2.83e-16 s.
pub fn table_step_s() -> f64 {
    steepest_interval_ohm() * CAPACITANCE_F / STEPS_PER_STEEPEST_RC_TIME
}

/// The phase a step at [`table_step_s`] advances at the top of the table,
/// rad: the most any step is allowed to advance. 0.0038 rad.
pub fn phase_step_limit_rad() -> f64 {
    PHASE_RATE_RAD_PER_V_S * TABLE_VOLTAGE_V[TABLE_KNOTS - 1] * table_step_s()
}

/// The plasma rate sqrt((2e/hbar) I_c / C), rad/s.
pub fn plasma_rate_rad_s() -> f64 {
    (PHASE_RATE_RAD_PER_V_S * CRITICAL_CURRENT_A / CAPACITANCE_F).sqrt()
}

/// The plasma period, s.
pub fn plasma_period_s() -> f64 {
    2.0 * std::f64::consts::PI / plasma_rate_rad_s()
}

/// The quality of the small-voltage motion: plasma rate x capacitance x the
/// table's chord resistance at [`QUALITY_BIAS_FRACTION`] of the critical
/// current.
pub fn quality() -> f64 {
    let current = QUALITY_BIAS_FRACTION * CRITICAL_CURRENT_A;
    plasma_rate_rad_s() * CAPACITANCE_F * table_voltage_at(current) / current
}

/// The settling time of each point, s.
pub fn settle_s() -> f64 {
    (SETTLE_PLASMA_PERIODS + SETTLE_PLASMA_PERIODS_PER_QUALITY * quality()) * plasma_period_s()
}

/// The dissipative current at voltage `v`, A, and whether `v` is above the
/// top of the table.
fn dissipative_current(v: f64, slopes: &[f64; TABLE_KNOTS - 1]) -> (f64, bool) {
    let size = v.abs();
    let top = TABLE_VOLTAGE_V[TABLE_KNOTS - 1];
    let (current, beyond) = if size > top {
        (TABLE_CURRENT_A[TABLE_KNOTS - 1] + (size - top) / NORMAL_RESISTANCE_OHM, true)
    } else {
        let k = TABLE_VOLTAGE_V.partition_point(|&knot| knot <= size).clamp(1, TABLE_KNOTS - 1) - 1;
        (TABLE_CURRENT_A[k] + slopes[k] * (size - TABLE_VOLTAGE_V[k]), false)
    };
    (if v < 0.0 { -current } else { current }, beyond)
}

/// The table's voltage at dissipative current `current` (at least zero), V:
/// straight lines between knots, and above the table the line of slope R_n
/// from the last knot.
pub fn table_voltage_at(current: f64) -> f64 {
    let top = TABLE_CURRENT_A[TABLE_KNOTS - 1];
    if current >= top {
        return TABLE_VOLTAGE_V[TABLE_KNOTS - 1] + (current - top) * NORMAL_RESISTANCE_OHM;
    }
    let k = TABLE_CURRENT_A.partition_point(|&knot| knot <= current).clamp(1, TABLE_KNOTS - 1) - 1;
    TABLE_VOLTAGE_V[k]
        + (TABLE_VOLTAGE_V[k + 1] - TABLE_VOLTAGE_V[k]) / (TABLE_CURRENT_A[k + 1] - TABLE_CURRENT_A[k])
            * (current - TABLE_CURRENT_A[k])
}

fn slopes() -> [f64; TABLE_KNOTS - 1] {
    let mut out = [0.0; TABLE_KNOTS - 1];
    for k in 0..TABLE_KNOTS - 1 {
        out[k] = (TABLE_CURRENT_A[k + 1] - TABLE_CURRENT_A[k])
            / (TABLE_VOLTAGE_V[k + 1] - TABLE_VOLTAGE_V[k]);
    }
    out
}

/// The stored energy 1/2 C V^2 + E_J (1 - cos(phase)), J.
pub fn stored_energy_j(phase_rad: f64, voltage_v: f64) -> f64 {
    0.5 * CAPACITANCE_F * voltage_v * voltage_v + JOSEPHSON_ENERGY_J * (1.0 - math::cos(phase_rad))
}

/// How one point is stepped: its time step, its settling steps, the steps of
/// each half window and the drive's angular rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointPlan {
    pub time_step_s: f64,
    pub settle_steps: u64,
    pub half_window_steps: u64,
    pub drive_rate_rad_s: f64,
    pub drive_amplitude_a: f64,
}

impl PointPlan {
    /// The steps of the point when its first window settles.
    pub fn nominal_steps(&self) -> u64 {
        self.settle_steps + 2 * self.half_window_steps
    }

    /// The most steps the point can take: its settling and every window,
    /// each twice as long as the one before.
    pub fn most_steps(&self) -> u64 {
        let windows = (1u64 << MAX_WINDOWS) - 1;
        self.settle_steps.saturating_add(self.half_window_steps.saturating_mul(2 * windows))
    }
}

/// The plan of a point at bias `bias_a` under `drive`. The step is the
/// table's, made finer where the voltage could pass the top of the table:
/// the largest voltage the bias and the drive can hold on the normal line,
/// (|I_dc| + |I_ac| + I_c) R_n, advances the phase by at most
/// [`phase_step_limit_rad`] in one step. Under a drive the step divides the
/// drive period whole, and each half window is the fewest whole drive
/// periods that span half the averaging time.
pub fn point_plan(bias_a: f64, drive: Drive) -> PointPlan {
    let counts = point_counts(bias_a, drive);
    PointPlan {
        time_step_s: counts.time_step_s,
        settle_steps: counts.settle_steps as u64,
        half_window_steps: counts.half_window_steps as u64,
        drive_rate_rad_s: counts.drive_rate_rad_s,
        drive_amplitude_a: counts.drive_amplitude_a,
    }
}

/// The plan of [`point_plan`] with its counts as 64-bit numbers, so that a
/// count past what 64-bit integers hold is stated, not pulled back.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PointCounts {
    time_step_s: f64,
    settle_steps: f64,
    half_window_steps: f64,
    drive_rate_rad_s: f64,
    drive_amplitude_a: f64,
}

impl PointCounts {
    /// The most steps the point can take: its settling and every window.
    fn most_steps(&self) -> f64 {
        self.settle_steps + self.half_window_steps * (2.0 * ((1_u64 << MAX_WINDOWS) - 1) as f64)
    }
}

fn point_counts(bias_a: f64, drive: Drive) -> PointCounts {
    let (amplitude, frequency) = match drive {
        Drive::None => (0.0, 0.0),
        Drive::Microwave { current_amplitude_a, frequency_hz } => (current_amplitude_a, frequency_hz),
    };
    let reach_v = (bias_a.abs() + amplitude.abs() + CRITICAL_CURRENT_A) * NORMAL_RESISTANCE_OHM;
    let top_v = TABLE_VOLTAGE_V[TABLE_KNOTS - 1];
    let mut dt = table_step_s();
    if reach_v > top_v {
        dt = phase_step_limit_rad() / (PHASE_RATE_RAD_PER_V_S * reach_v);
    }
    let half_average_s = 0.5 * AVERAGE_PLASMA_PERIODS * plasma_period_s();
    if frequency > 0.0 {
        let period = 1.0 / frequency;
        let per_period = (period / dt).ceil();
        dt = period / per_period;
        let periods = (half_average_s / period).ceil().max(1.0);
        PointCounts {
            time_step_s: dt,
            settle_steps: (settle_s() / dt).ceil(),
            half_window_steps: periods * per_period,
            drive_rate_rad_s: 2.0 * std::f64::consts::PI * frequency,
            drive_amplitude_a: amplitude,
        }
    } else {
        PointCounts {
            time_step_s: dt,
            settle_steps: (settle_s() / dt).ceil(),
            half_window_steps: (half_average_s / dt).ceil(),
            drive_rate_rad_s: 0.0,
            drive_amplitude_a: 0.0,
        }
    }
}

/// The limits the question itself shows, in the order the solver asks them.
/// Nothing is stepped.
pub fn admit(question: &JunctionQuestion) -> Result<u64, JunctionReject> {
    if question.bias_current_a.is_empty() {
        return Err(JunctionReject::NoBiasPoints);
    }
    if question.bias_current_a.iter().any(|value| !value.is_finite()) {
        return Err(JunctionReject::InputNotFinite { field: "bias_current_a" });
    }
    if let Drive::Microwave { current_amplitude_a, frequency_hz } = question.drive {
        if !current_amplitude_a.is_finite() {
            return Err(JunctionReject::InputNotFinite { field: "drive_current_amplitude_a" });
        }
        if !frequency_hz.is_finite() {
            return Err(JunctionReject::InputNotFinite { field: "drive_frequency_hz" });
        }
        if !(frequency_hz > 0.0) {
            return Err(JunctionReject::DriveFrequencyNotPositive { frequency_hz });
        }
        // With no amplitude there is no photon, and the frequency only
        // sets the windows.
        if frequency_hz >= GAP_FREQUENCY_HZ && current_amplitude_a != 0.0 {
            return Err(JunctionReject::DriveAtOrAboveGapFrequency { frequency_hz });
        }
    }
    // Counted as 64-bit numbers, each count a whole number, exact to 2^53:
    // a count past the budget is stated as it is.
    let mut steps = 0.0_f64;
    for &bias in &question.bias_current_a {
        let counts = point_counts(bias, question.drive);
        let most = if counts.time_step_s > 0.0 { counts.most_steps() } else { f64::INFINITY };
        steps += most;
    }
    if !(steps <= MAX_STEPS_PER_SOLVE as f64) {
        return Err(JunctionReject::StepsBeyondSolveBudget { steps });
    }
    Ok(steps as u64)
}

/// The numerical choices. The served solver uses [`SERVED`]; the tests break
/// one at a time.
#[derive(Clone, Copy, Debug)]
struct Numerics {
    /// Forward Euler in place of Runge-Kutta, for the tests.
    euler: bool,
    /// Added to the source's work after every step, J, for the tests.
    work_bias_j: f64,
    /// The phase rate is multiplied by this in the phase equation only, for
    /// the tests.
    phase_rate_factor: f64,
    /// The settling steps are multiplied by this, for the tests.
    settle_factor: f64,
    /// The settle tolerance, for the tests.
    settle_tolerance: f64,
}

const SERVED: Numerics = Numerics {
    euler: false,
    work_bias_j: 0.0,
    phase_rate_factor: 1.0,
    settle_factor: 1.0,
    settle_tolerance: SETTLE_TOLERANCE,
};

/// Compute the characteristic for one question, or name the limit.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &JunctionQuestion) -> Result<JunctionCurve, JunctionReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &JunctionQuestion) -> Result<JunctionCurve, JunctionReject> {
    admit(question)?;
    run(question, &SERVED)
}

impl NormalOrZero for JunctionCurve {
    fn normal_or_zero(&mut self) {
        self.bias_current_a.normal_or_zero();
        self.mean_voltage_v.normal_or_zero();
        self.josephson_frequency_hz.normal_or_zero();
        self.phase_turns.normal_or_zero();
        self.averaging_time_s.normal_or_zero();
        self.peak_voltage_v.normal_or_zero();
        self.source_power_w.normal_or_zero();
        self.dissipated_power_w.normal_or_zero();
        self.normal_line_reached.normal_or_zero();
        self.time_step_s.normal_or_zero();
        self.steps.normal_or_zero();
        self.largest_energy_residual.normal_or_zero();
        self.largest_flux_residual_quanta.normal_or_zero();
        self.largest_settle_difference.normal_or_zero();
        self.final_phase_rad.normal_or_zero();
        self.final_voltage_v.normal_or_zero();
    }
}

fn run(question: &JunctionQuestion, numerics: &Numerics) -> Result<JunctionCurve, JunctionReject> {
    let n = question.bias_current_a.len();
    let mut curve = JunctionCurve {
        bias_current_a: question.bias_current_a.clone(),
        mean_voltage_v: Vec::with_capacity(n),
        josephson_frequency_hz: Vec::with_capacity(n),
        phase_turns: Vec::with_capacity(n),
        averaging_time_s: Vec::with_capacity(n),
        peak_voltage_v: Vec::with_capacity(n),
        source_power_w: Vec::with_capacity(n),
        dissipated_power_w: Vec::with_capacity(n),
        normal_line_reached: Vec::with_capacity(n),
        time_step_s: Vec::with_capacity(n),
        steps: Vec::with_capacity(n),
        largest_energy_residual: 0.0,
        largest_flux_residual_quanta: 0.0,
        largest_settle_difference: 0.0,
        total_steps: 0,
        final_phase_rad: Vec::with_capacity(n),
        final_voltage_v: Vec::with_capacity(n),
    };
    let slopes = slopes();
    for (point, &bias) in question.bias_current_a.iter().enumerate() {
        let plan = point_plan(bias, question.drive);
        let done = one_point(point, bias, question.start, &plan, &slopes, numerics)?;
        curve.mean_voltage_v.push(done.mean_voltage_v);
        curve.josephson_frequency_hz.push(done.mean_voltage_v / FLUX_QUANTUM_WB);
        curve.phase_turns.push(done.turns);
        curve.averaging_time_s.push(done.averaging_time_s);
        curve.peak_voltage_v.push(done.peak_voltage_v);
        curve.source_power_w.push(done.source_power_w);
        curve.dissipated_power_w.push(done.dissipated_power_w);
        curve.normal_line_reached.push(if done.normal_line { 1.0 } else { 0.0 });
        curve.time_step_s.push(plan.time_step_s);
        curve.steps.push(done.steps as f64);
        curve.largest_energy_residual = curve.largest_energy_residual.max(done.largest_residual);
        curve.largest_flux_residual_quanta = curve.largest_flux_residual_quanta.max(done.flux_residual_quanta);
        curve.largest_settle_difference = curve.largest_settle_difference.max(done.settle_difference);
        curve.total_steps += done.steps;
        curve.final_phase_rad.push(done.phase_rad);
        curve.final_voltage_v.push(done.voltage_v);
    }
    Ok(curve)
}

/// The start of a point: (phase, voltage).
pub fn start_state(bias_a: f64, start: Start) -> (f64, f64) {
    let sign = if bias_a < 0.0 { -1.0 } else { 1.0 };
    match start {
        Start::Rest => {
            let ratio = bias_a / CRITICAL_CURRENT_A;
            if ratio.abs() <= 1.0 {
                (math::asin(ratio), 0.0)
            } else {
                (sign * 0.5 * std::f64::consts::PI, 0.0)
            }
        }
        Start::Running => {
            // Above the running motion: the larger of the table's voltage at
            // 1.2 times the bias and at 0.3 of the critical current, and three
            // times the plasma rate as a voltage (the running motion near the
            // return current peaks near twice it).
            let size = bias_a.abs();
            let above = table_voltage_at(1.2 * size)
                .max(table_voltage_at(0.3 * CRITICAL_CURRENT_A))
                .max(3.0 * plasma_rate_rad_s() / PHASE_RATE_RAD_PER_V_S);
            (0.0, sign * above)
        }
    }
}

struct PointDone {
    mean_voltage_v: f64,
    turns: f64,
    averaging_time_s: f64,
    peak_voltage_v: f64,
    source_power_w: f64,
    dissipated_power_w: f64,
    normal_line: bool,
    steps: u64,
    largest_residual: f64,
    flux_residual_quanta: f64,
    settle_difference: f64,
    phase_rad: f64,
    voltage_v: f64,
}

/// The motion: phase, voltage, net work W_s - W_d, source work, dissipated
/// work, voltage integral.
type Motion = [f64; 6];

struct Stepper<'a> {
    bias_a: f64,
    plan: &'a PointPlan,
    slopes: &'a [f64; TABLE_KNOTS - 1],
    numerics: &'a Numerics,
}

impl Stepper<'_> {
    fn rates(&self, y: &Motion, t: f64) -> Motion {
        let bias = if self.plan.drive_amplitude_a != 0.0 {
            self.bias_a + self.plan.drive_amplitude_a * math::sin(self.plan.drive_rate_rad_s * t)
        } else {
            self.bias_a
        };
        let (dissipative, _) = dissipative_current(y[1], self.slopes);
        let supercurrent = CRITICAL_CURRENT_A * math::sin(y[0]);
        [
            PHASE_RATE_RAD_PER_V_S * self.numerics.phase_rate_factor * y[1],
            (bias - dissipative - supercurrent) / CAPACITANCE_F,
            (bias - dissipative) * y[1],
            bias * y[1],
            dissipative * y[1],
            y[1],
        ]
    }

    fn step(&self, y: &Motion, t: f64, dt: f64) -> Motion {
        let k1 = self.rates(y, t);
        if self.numerics.euler {
            let mut out = *y;
            for i in 0..6 {
                out[i] += dt * k1[i];
            }
            return out;
        }
        let k2 = self.rates(&along(y, 0.5 * dt, &k1), t + 0.5 * dt);
        let k3 = self.rates(&along(y, 0.5 * dt, &k2), t + 0.5 * dt);
        let k4 = self.rates(&along(y, dt, &k3), t + dt);
        let mut out = *y;
        for i in 0..6 {
            out[i] += dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        out
    }
}

fn along(y: &Motion, scale: f64, k: &Motion) -> Motion {
    let mut out = *y;
    for i in 0..6 {
        out[i] += scale * k[i];
    }
    out
}

/// Whole turns of the phase crossed, upward and downward: the first and the
/// last crossing of a multiple of 2 pi, each with its time and its multiple.
#[derive(Clone, Copy, Default)]
struct Crossings {
    up_first: Option<(f64, f64)>,
    up_last: Option<(f64, f64)>,
    down_first: Option<(f64, f64)>,
    down_last: Option<(f64, f64)>,
}

const TWO_PI: f64 = 2.0 * std::f64::consts::PI;

impl Crossings {
    fn observe(&mut self, phase_old: f64, phase_new: f64, t_old: f64, dt: f64) {
        let a = (phase_old / TWO_PI).floor();
        let b = (phase_new / TWO_PI).floor();
        if b > a {
            let level = b;
            let t = t_old + dt * (TWO_PI * level - phase_old) / (phase_new - phase_old);
            if self.up_first.is_none() {
                self.up_first = Some((t, level));
            }
            self.up_last = Some((t, level));
        } else if b < a {
            let level = a;
            let t = t_old + dt * (TWO_PI * level - phase_old) / (phase_new - phase_old);
            if self.down_first.is_none() {
                self.down_first = Some((t, level));
            }
            self.down_last = Some((t, level));
        }
    }

    /// The mean voltage and the turns over a stretch that began at
    /// `phase_start` and ended at `phase_end` after `duration`: whole turns
    /// over their time when at least one was made in the direction of the
    /// phase's advance, otherwise the advance over the duration.
    fn mean(&self, phase_start: f64, phase_end: f64, duration: f64) -> (f64, f64) {
        let (first, last) = if phase_end >= phase_start {
            (self.up_first, self.up_last)
        } else {
            (self.down_first, self.down_last)
        };
        if let (Some((t0, n0)), Some((t1, n1))) = (first, last) {
            let turns = n1 - n0;
            if turns.abs() >= 1.0 && t1 > t0 {
                return (TWO_PI * turns / (PHASE_RATE_RAD_PER_V_S * (t1 - t0)), turns);
            }
        }
        (
            (phase_end - phase_start) / (PHASE_RATE_RAD_PER_V_S * duration),
            (phase_end - phase_start) / TWO_PI,
        )
    }
}

fn finite6(y: &Motion) -> bool {
    y.iter().all(|value| value.is_finite())
}

fn one_point(
    point: usize,
    bias_a: f64,
    start: Start,
    plan: &PointPlan,
    slopes: &[f64; TABLE_KNOTS - 1],
    numerics: &Numerics,
) -> Result<PointDone, JunctionReject> {
    let stepper = Stepper { bias_a, plan, slopes, numerics };
    let dt = plan.time_step_s;
    let (phase0, voltage0) = start_state(bias_a, start);
    let energy_start = stored_energy_j(phase0, voltage0);
    let mut y: Motion = [phase0, voltage0, 0.0, 0.0, 0.0, 0.0];
    let mut n: u64 = 0;
    let mut largest_residual = 0.0_f64;
    let top_v = TABLE_VOLTAGE_V[TABLE_KNOTS - 1];

    // One step, the ledger closed on it. Returns the new motion.
    let advance = |y: &Motion, n: &mut u64, largest: &mut f64| -> Result<Motion, JunctionReject> {
        let t = *n as f64 * dt;
        let mut next = stepper.step(y, t, dt);
        next[3] += numerics.work_bias_j;
        next[2] += numerics.work_bias_j;
        if !finite6(&next) {
            return Err(JunctionReject::NonFiniteValue { point });
        }
        *n += 1;
        let residual = (stored_energy_j(next[0], next[1]) - energy_start - next[2]).abs() / ENERGY_SCALE_J;
        if !(residual <= RESIDUAL_CONSERVATION_LIMIT) {
            return Err(JunctionReject::EnergyLedgerBroken { point, step: *n, residual });
        }
        *largest = largest.max(residual);
        Ok(next)
    };

    let settle_steps = ((plan.settle_steps as f64) * numerics.settle_factor).ceil() as u64;
    for _ in 0..settle_steps {
        y = advance(&y, &mut n, &mut largest_residual)?;
    }
    let mut last_difference = f64::INFINITY;
    for window in 0..MAX_WINDOWS {
        let half_steps = plan.half_window_steps << window;
        let start_y = y;
        let mut whole = Crossings::default();
        let mut halves = [Crossings::default(), Crossings::default()];
        let mut peak = 0.0_f64;
        let mut beyond = false;
        let mut middle_y = y;
        for half in 0..2 {
            for _ in 0..half_steps {
                let t_old = n as f64 * dt;
                let next = advance(&y, &mut n, &mut largest_residual)?;
                whole.observe(y[0], next[0], t_old, dt);
                halves[half].observe(y[0], next[0], t_old, dt);
                y = next;
                peak = peak.max(y[1].abs());
                beyond |= y[1].abs() > top_v;
            }
            if half == 0 {
                middle_y = y;
            }
        }
        let half_s = half_steps as f64 * dt;
        let duration = 2.0 * half_s;
        let driven = plan.drive_amplitude_a != 0.0 || plan.drive_rate_rad_s != 0.0;
        let (mean, turns, first, second) = if driven {
            let advance_rad = y[0] - start_y[0];
            (
                advance_rad / (PHASE_RATE_RAD_PER_V_S * duration),
                advance_rad / TWO_PI,
                (middle_y[0] - start_y[0]) / (PHASE_RATE_RAD_PER_V_S * half_s),
                (y[0] - middle_y[0]) / (PHASE_RATE_RAD_PER_V_S * half_s),
            )
        } else {
            let (mean, turns) = whole.mean(start_y[0], y[0], duration);
            let (first, _) = halves[0].mean(start_y[0], middle_y[0], half_s);
            let (second, _) = halves[1].mean(middle_y[0], y[0], half_s);
            (mean, turns, first, second)
        };
        let flux_residual_quanta = ((y[0] - start_y[0])
            - PHASE_RATE_RAD_PER_V_S * (y[5] - start_y[5]))
            .abs()
            / TWO_PI;
        if !(flux_residual_quanta <= FLUX_ACCOUNTING_LIMIT_QUANTA) {
            return Err(JunctionReject::FluxAccountingBroken { point, residual_quanta: flux_residual_quanta });
        }
        let difference = (first - second).abs() / CHARACTERISTIC_VOLTAGE_V;
        last_difference = difference;
        // With no drive a bias above the critical current has no state at
        // rest: a window whose phase made no whole turn is still on its way,
        // however alike its halves, and counts as more settling.
        let creeping = plan.drive_amplitude_a == 0.0 && bias_a.abs() > CRITICAL_CURRENT_A && turns.abs() < 1.0;
        if difference <= numerics.settle_tolerance && !creeping {
            return Ok(PointDone {
                mean_voltage_v: mean,
                turns,
                averaging_time_s: duration,
                peak_voltage_v: peak,
                source_power_w: (y[3] - start_y[3]) / duration,
                dissipated_power_w: (y[4] - start_y[4]) / duration,
                normal_line: beyond,
                steps: n,
                largest_residual,
                flux_residual_quanta,
                settle_difference: difference,
                phase_rad: y[0],
                voltage_v: y[1],
            });
        }
    }
    Err(JunctionReject::NotSettled { point, difference: last_difference, steps: n })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(bias: &[f64], start: Start) -> JunctionQuestion {
        JunctionQuestion { bias_current_a: bias.to_vec(), start, drive: Drive::None }
    }

    fn driven(bias: &[f64], start: Start, amplitude: f64, frequency: f64) -> JunctionQuestion {
        JunctionQuestion {
            bias_current_a: bias.to_vec(),
            start,
            drive: Drive::Microwave { current_amplitude_a: amplitude, frequency_hz: frequency },
        }
    }

    fn mv(curve: &JunctionCurve, k: usize) -> f64 {
        curve.mean_voltage_v[k] * 1e3
    }

    /// The table as copied: 199 knots, both columns rising strictly, the origin
    /// first, the first, plateau and last rows of the CSV, the slope floor of
    /// 1 percent of the normal resistance, and the derived scales.
    #[test]
    fn the_constants_are_the_research_records() {
        assert_eq!(TABLE_CURRENT_A.len(), 199);
        assert_eq!((TABLE_CURRENT_A[0], TABLE_VOLTAGE_V[0]), (0.0, 0.0));
        assert_eq!((TABLE_CURRENT_A[1], TABLE_VOLTAGE_V[1]), (9.0315e-6, 4.855009e-4));
        assert_eq!((TABLE_CURRENT_A[198], TABLE_VOLTAGE_V[198]), (9.1872e-4, 4.4254955e-3));
        assert_eq!((TABLE_CURRENT_A[GAP_KNOT], GAP_VOLTAGE_V), (1.575115e-4, 2.8602516e-3));
        for k in 0..TABLE_KNOTS - 1 {
            assert!(TABLE_CURRENT_A[k + 1] > TABLE_CURRENT_A[k], "{k}");
            assert!(TABLE_VOLTAGE_V[k + 1] > TABLE_VOLTAGE_V[k], "{k}");
        }
        // The slope floor: no interval steeper than 1 percent of R_n.
        let steepest = steepest_interval_ohm();
        assert!((steepest - 0.0479).abs() < 0.0001, "{steepest}");
        assert!(steepest >= 0.00994 * NORMAL_RESISTANCE_OHM, "{steepest}");
        // R_n is the voltage over the current at the top of the suppressed
        // curve, whose raw reading the table's last knot rounds.
        let top = TABLE_VOLTAGE_V[198] / TABLE_CURRENT_A[198];
        assert!((top / NORMAL_RESISTANCE_OHM - 1.0).abs() < 1e-4, "{top}");
        assert!((ENERGY_SCALE_J / 1.961472e-19 - 1.0).abs() < 1e-6, "{ENERGY_SCALE_J}");
        assert!((plasma_rate_rad_s() / 4.379e12 - 1.0).abs() < 1e-3);
        assert!((table_step_s() / 2.83e-16 - 1.0).abs() < 0.002, "{}", table_step_s());
        assert!((phase_step_limit_rad() - 0.0038).abs() < 0.0001);
        assert!((GAP_FREQUENCY_HZ / 6.916e11 - 1.0).abs() < 1e-3, "{GAP_FREQUENCY_HZ}");
        assert!((quality() - 11.11).abs() < 0.01, "{}", quality());
        assert!((FLUX_QUANTUM_WB / 2.067833848e-15 - 1.0).abs() < 1e-9);
        assert!((CHARACTERISTIC_VOLTAGE_V / 1.4354e-3 - 1.0).abs() < 1e-4);
    }

    /// The two bounds the gateway publishes are the arithmetic of the plan.
    #[test]
    fn the_published_bounds_follow_from_the_step_count() {
        let zero = point_plan(0.0, Drive::None);
        assert_eq!(MAX_BIAS_POINTS as u64, MAX_STEPS_PER_SOLVE / zero.most_steps());
        assert!(point_plan(CURRENT_REACH_MAX_A, Drive::None).most_steps() <= MAX_STEPS_PER_SOLVE);
        assert!(point_plan(CURRENT_REACH_MAX_A.next_up(), Drive::None).most_steps() > MAX_STEPS_PER_SOLVE);
        assert_eq!(admit(&held(&[-CURRENT_REACH_MAX_A], Start::Rest)).map(|_| ()), Ok(()));
        assert!(matches!(admit(&held(&[CURRENT_REACH_MAX_A.next_up()], Start::Rest)), Err(JunctionReject::StepsBeyondSolveBudget { .. })));
        // A drive counts at least the steps of no drive at |I_dc| + |I_ac|: at
        // zero bias the largest amplitude answered hangs on the frequency
        // (whole drive periods in each half window) and is never above it.
        for (frequency, most) in [(1.0e10, 0.009208), (1.0e11, 0.026163), (6.5e11, 0.033436), (6.9e11, 0.033218)] {
            let at = |amplitude: f64| point_plan(0.0, Drive::Microwave { current_amplitude_a: amplitude, frequency_hz: frequency }).most_steps() <= MAX_STEPS_PER_SOLVE;
            assert!(at(most) && !at(most + 1.0e-6) && most < CURRENT_REACH_MAX_A, "{frequency}");
        }
        assert_eq!(admit(&held(&vec![1e-4; MAX_BIAS_POINTS], Start::Rest)).map(|_| ()), Ok(()));
        assert!(matches!(
            admit(&held(&vec![1e-4; MAX_BIAS_POINTS + 1], Start::Rest)),
            Err(JunctionReject::StepsBeyondSolveBudget { .. })
        ));
        assert!(matches!(
            admit(&held(&[CURRENT_REACH_MAX_A * 1.01], Start::Rest)),
            Err(JunctionReject::StepsBeyondSolveBudget { .. })
        ));
    }

    /// The limits the question shows, each by name, before anything is stepped.
    #[test]
    fn every_limit_of_the_question_is_named() {
        assert_eq!(admit(&held(&[], Start::Rest)), Err(JunctionReject::NoBiasPoints));
        assert_eq!(
            admit(&held(&[1e-4, f64::NAN], Start::Rest)),
            Err(JunctionReject::InputNotFinite { field: "bias_current_a" })
        );
        assert_eq!(
            admit(&driven(&[1e-4], Start::Rest, f64::INFINITY, 1e11)),
            Err(JunctionReject::InputNotFinite { field: "drive_current_amplitude_a" })
        );
        assert_eq!(
            admit(&driven(&[1e-4], Start::Rest, 1e-4, f64::NAN)),
            Err(JunctionReject::InputNotFinite { field: "drive_frequency_hz" })
        );
        for frequency in [0.0, -1e9] {
            assert_eq!(
                admit(&driven(&[1e-4], Start::Rest, 1e-4, frequency)),
                Err(JunctionReject::DriveFrequencyNotPositive { frequency_hz: frequency })
            );
        }
        // The gap frequency itself breaks pairs; the number below it does not.
        assert_eq!(
            admit(&driven(&[1e-4], Start::Rest, 1e-4, GAP_FREQUENCY_HZ)),
            Err(JunctionReject::DriveAtOrAboveGapFrequency { frequency_hz: GAP_FREQUENCY_HZ })
        );
        let below = f64::from_bits(GAP_FREQUENCY_HZ.to_bits() - 1);
        assert!(admit(&driven(&[1e-4], Start::Rest, 1e-4, below)).is_ok());
        // With no amplitude there is no photon: the gap frequency and above
        // only set the windows.
        for frequency in [GAP_FREQUENCY_HZ, 6.92e11, 1.0e12] {
            assert!(admit(&driven(&[2e-4], Start::Running, 0.0, frequency)).is_ok(), "{frequency}");
        }
        // A drive so slow that one period passes one solve's steps.
        assert!(matches!(
            admit(&driven(&[1e-4], Start::Rest, 1e-4, 1e8)),
            Err(JunctionReject::StepsBeyondSolveBudget { .. })
        ));
        // The sentences name the numbers.
        let text = JunctionReject::DriveAtOrAboveGapFrequency { frequency_hz: 7e11 }.to_string();
        assert!(text.contains("breaks a Cooper pair") && text.contains("700000000000 Hz"), "{text}");
        // Short, and the count as it is, not pulled back to 64-bit integers.
        let text = JunctionReject::DriveFrequencyNotPositive { frequency_hz: -1e-300 }.to_string();
        assert!(text.contains("-1e-300 Hz"), "{text}");
        match admit(&driven(&[1e-4], Start::Rest, 1e-4, 1e-280)) {
            Err(JunctionReject::StepsBeyondSolveBudget { steps }) => assert!(steps > 1e290 && steps.is_finite(), "{steps:e}"),
            other => panic!("{other:?}"),
        }
        let text = admit(&held(&[1e308], Start::Rest)).unwrap_err().to_string();
        assert!(text.starts_with("the question needs more steps than a 64-bit number holds"), "{text}");
        assert!(!text.contains("18446744073709551615"));
    }

    /// Below the critical current a point started at rest stays at zero
    /// voltage, the phase where the supercurrent carries the bias, and its
    /// ledger closes to rounding. At the stated critical current, the last
    /// bias the laboratory read at zero voltage, the junction holds; just above
    /// it, it switches to the plateau.
    #[test]
    fn rest_holds_up_to_the_critical_current_and_switches_above_it() {
        let curve = solve(&held(&[0.0, 1.5e-4, 2.98e-4, 3.0e-4], Start::Rest)).expect("solves");
        for k in 0..3 {
            assert!(curve.mean_voltage_v[k].abs() < 1e-12, "{k} {}", curve.mean_voltage_v[k]);
            assert!(curve.phase_turns[k].abs() < 1e-6);
            assert_eq!(curve.normal_line_reached[k], 0.0);
        }
        let phase = (1.5e-4 / CRITICAL_CURRENT_A).asin();
        assert!((curve.final_phase_rad[1] - phase).abs() < 1e-9);
        assert!(mv(&curve, 3) > 2.7 && mv(&curve, 3) < 2.9, "{}", mv(&curve, 3));
        assert!(curve.largest_energy_residual < 1e-6);
        assert!(curve.largest_flux_residual_quanta < 1e-9);
    }

    /// Above the critical current no state at rest exists: a point started at
    /// rest creeps toward its switch, and two halves of a window alike at
    /// nearly zero are not a settled zero. Where the switch takes longer than
    /// every window the point is refused as NotSettled; where it comes
    /// sooner the point is read on the plateau.
    #[test]
    fn a_point_creeping_above_the_critical_current_is_not_settled_at_zero() {
        // 1e-12 and 1e-10 above it once came back settled at 1.6e-14 and
        // 1.6e-12 V; 1e-8 at 5.6e-10 V, and with its windows it now switches.
        for epsilon in [1e-12, 1e-10] {
            match solve(&held(&[CRITICAL_CURRENT_A * (1.0 + epsilon)], Start::Rest)) {
                Err(JunctionReject::NotSettled { point: 0, .. }) => {}
                other => panic!("{epsilon}: {:?}", other.map(|c| c.mean_voltage_v)),
            }
        }
        for epsilon in [1e-8, 1e-6] {
            let switched = solve(&held(&[CRITICAL_CURRENT_A * (1.0 + epsilon)], Start::Rest)).expect("solves");
            assert!(mv(&switched, 0) > 2.7 && switched.phase_turns[0] >= 1.0, "{epsilon}: {}", mv(&switched, 0));
        }
        // At the critical current itself the state at rest holds.
        let held_at = solve(&held(&[CRITICAL_CURRENT_A], Start::Rest)).expect("solves");
        assert!(held_at.mean_voltage_v[0].abs() < 1e-12);
    }

    /// The plateau as the stand-in of the research record gave it at 0.40 mA,
    /// 2.8905 mV, whole turns of the phase read over their time, and the
    /// ledger's account of it: the powers are means over the whole window,
    /// not whole turns, and differ by the stored energy's change over it,
    /// 1.15e-3 of the source's here.
    #[test]
    fn the_plateau_at_0_4_ma_is_the_stand_ins() {
        let curve = solve(&held(&[4.0e-4], Start::Rest)).expect("solves");
        assert!((mv(&curve, 0) - 2.8905).abs() < 0.0005, "{}", mv(&curve, 0));
        let turns = curve.phase_turns[0];
        assert!(turns >= 50.0 && turns.fract() == 0.0, "{turns}");
        let balance = curve.source_power_w[0] / curve.dissipated_power_w[0] - 1.0;
        assert!(balance.abs() < 0.002, "{balance}");
        // The source's mean power is the bias times the mean voltage.
        let power = 4.0e-4 * curve.mean_voltage_v[0];
        assert!((curve.source_power_w[0] / power - 1.0).abs() < 0.01);
        assert!(curve.largest_energy_residual < 1e-6, "{}", curve.largest_energy_residual);
        assert!(curve.largest_settle_difference <= SETTLE_TOLERANCE);
    }

    /// The hysteresis: at 0.1 mA the junction sits at zero voltage when it
    /// comes from rest and runs at about 1.96 mV when it comes from the
    /// voltage state; the running branch returns to zero between 0.0596 and
    /// 0.0610 mA, inside the measured return bracket the capacitance was
    /// fitted to (0.05567 mA at zero voltage, 0.06494 mA in the voltage state).
    #[test]
    fn nist_return_bracket_and_critical_current() {
        let rest = solve(&held(&[1.0e-4], Start::Rest)).expect("solves");
        assert!(rest.mean_voltage_v[0].abs() < 1e-12);
        let running = solve(&held(&[5.567e-5, 5.96e-5, 6.10e-5, 6.494e-5, 1.0e-4], Start::Running)).expect("solves");
        assert!(running.mean_voltage_v[0].abs() < 1e-12);
        assert!(running.mean_voltage_v[1].abs() < 1e-12);
        assert!(mv(&running, 2) > 0.9, "{}", mv(&running, 2));
        assert!(mv(&running, 3) > 1.2, "{}", mv(&running, 3));
        assert!((mv(&running, 4) - 1.963).abs() < 0.002, "{}", mv(&running, 4));
    }

    /// The model is odd in the bias: a negative bias gives the negative of
    /// the voltage, bit for bit.
    #[test]
    fn a_negative_bias_mirrors_a_positive_one() {
        let plus = solve(&held(&[4.0e-4, 1.0e-4], Start::Running)).expect("solves");
        let minus = solve(&held(&[-4.0e-4, -1.0e-4], Start::Running)).expect("solves");
        for k in 0..2 {
            assert_eq!(minus.mean_voltage_v[k], -plus.mean_voltage_v[k], "{k}");
            assert_eq!(minus.phase_turns[k], -plus.phase_turns[k], "{k}");
            assert_eq!(minus.dissipated_power_w[k], plus.dissipated_power_w[k], "{k}");
        }
    }

    /// Where the card drafted on 2026-10-06 stopped, above 0.807 mA, the
    /// ripple carries the voltage past the table's top and the dissipation
    /// follows the normal line: the point is computed, flagged, and its ledger
    /// closes. At 5 mA the mean voltage is the normal line's, I R_n, within a
    /// percent, and the step is finer, so that the phase moves no more per step.
    #[test]
    fn above_the_table_the_normal_line_carries_the_dissipation_and_the_ledger_closes() {
        let curve = solve(&held(&[9.0e-4, 5.0e-3], Start::Rest)).expect("solves");
        assert_eq!(curve.normal_line_reached, vec![1.0, 1.0]);
        assert!(curve.peak_voltage_v[0] > TABLE_VOLTAGE_V[TABLE_KNOTS - 1]);
        let ohmic = 5.0e-3 * NORMAL_RESISTANCE_OHM;
        assert!((curve.mean_voltage_v[1] / ohmic - 1.0).abs() < 0.01, "{}", curve.mean_voltage_v[1]);
        assert!(curve.time_step_s[1] < curve.time_step_s[0]);
        let phase_step = PHASE_RATE_RAD_PER_V_S * curve.peak_voltage_v[1] * curve.time_step_s[1];
        assert!(phase_step <= phase_step_limit_rad() * 1.0001, "{phase_step}");
        assert!(curve.largest_energy_residual < 1e-6, "{}", curve.largest_energy_residual);
    }

    /// A Shapiro step: under 650 GHz at 0.2 mA, from 0.25 to 0.35 mA the mean
    /// voltage is twice h f / 2e, to rounding, and a step of half that order
    /// holds at 0.15 mA. The constants of nature, not the table, set it.
    #[test]
    fn a_shapiro_step_is_n_h_f_over_2e() {
        let frequency = 6.5e11;
        let curve = solve(&driven(&[1.5e-4, 2.5e-4, 3.0e-4, 3.5e-4], Start::Running, 2.0e-4, frequency)).expect("solves");
        let quantum = FLUX_QUANTUM_WB * frequency;
        assert!((curve.mean_voltage_v[0] / (0.5 * quantum) - 1.0).abs() < 1e-10, "{}", curve.mean_voltage_v[0]);
        for k in 1..4 {
            assert!((curve.mean_voltage_v[k] / (2.0 * quantum) - 1.0).abs() < 1e-10, "{k} {}", curve.mean_voltage_v[k]);
            assert!((curve.josephson_frequency_hz[k] / frequency - 2.0).abs() < 1e-10);
        }
        // Whole drive periods: two turns of the phase per period.
        let periods = curve.averaging_time_s[1] * frequency;
        assert!((periods - periods.round()).abs() < 1e-6, "{periods}");
        assert!((curve.phase_turns[1] / periods.round() - 2.0).abs() < 1e-9);
        assert!(curve.largest_energy_residual < 1e-6);
    }

    /// Where the driven junction has no steady mean, its point is refused by
    /// name, with no curve: at 500 GHz and 0.1 mA, from 0.1 mA running.
    #[test]
    fn a_driven_point_that_does_not_settle_is_refused_by_name() {
        match solve(&driven(&[1.0e-4], Start::Running, 1.0e-4, 5.0e11)) {
            Err(JunctionReject::NotSettled { point: 0, difference, steps }) => {
                assert!(difference > SETTLE_TOLERANCE, "{difference}");
                assert_eq!(steps, point_plan(1.0e-4, Drive::Microwave { current_amplitude_a: 1.0e-4, frequency_hz: 5.0e11 }).most_steps());
            }
            other => panic!("{other:?}"),
        }
    }

    /// Forward Euler creates energy on the plateau, and the ledger names it.
    #[test]
    fn explicit_euler_is_refused_by_the_ledger() {
        let numerics = Numerics { euler: true, ..SERVED };
        match run(&held(&[4.0e-4], Start::Running), &numerics) {
            Err(JunctionReject::EnergyLedgerBroken { point: 0, step, residual }) => {
                assert!(residual > RESIDUAL_CONSERVATION_LIMIT);
                assert!(step < 100_000, "{step}");
            }
            other => panic!("{other:?}"),
        }
    }

    /// A source's work altered by 1e-3 of the scale after each step is named
    /// on the first step, either way.
    #[test]
    fn an_altered_work_is_refused_on_the_first_step() {
        for bias in [1e-3 * ENERGY_SCALE_J, -1e-3 * ENERGY_SCALE_J] {
            let numerics = Numerics { work_bias_j: bias, ..SERVED };
            match run(&held(&[1.0e-4], Start::Rest), &numerics) {
                Err(JunctionReject::EnergyLedgerBroken { point: 0, step: 1, .. }) => {}
                other => panic!("{other:?}"),
            }
        }
    }

    /// A phase that runs one part in ten million fast against its voltage does
    /// not move the energy ledger past its limit (the error integrates to a
    /// bounded term over whole turns), and the flux accounting names it.
    #[test]
    fn a_phase_rate_off_by_one_part_in_ten_million_breaks_the_flux_accounting() {
        let numerics = Numerics { phase_rate_factor: 1.0 + 1e-7, ..SERVED };
        match run(&held(&[4.0e-4], Start::Rest), &numerics) {
            Err(JunctionReject::FluxAccountingBroken { point: 0, residual_quanta }) => {
                assert!(residual_quanta > FLUX_ACCOUNTING_LIMIT_QUANTA, "{residual_quanta}");
            }
            other => panic!("{other:?}"),
        }
    }

    /// A settle tolerance of zero is never met on the plateau, and the point is
    /// refused after its windows, by name.
    #[test]
    fn a_tolerance_that_cannot_be_met_is_refused_as_not_settled() {
        let numerics = Numerics { settle_tolerance: 0.0, settle_factor: 0.05, ..SERVED };
        assert!(matches!(
            run(&held(&[4.0e-4], Start::Rest), &numerics),
            Err(JunctionReject::NotSettled { point: 0, .. })
        ));
    }

    /// The same question gives the same bits.
    #[test]
    fn the_same_question_gives_the_same_bits() {
        let question = driven(&[2.0e-4], Start::Rest, 1.0e-4, 3.0e11);
        let first = solve(&question);
        let second = solve(&question);
        assert_eq!(first, second);
    }

    /// The scored comparison at four of its points, as the engine gave them
    /// on 2026-10-14 (evidence/research/junction_nist_nb_asi_nb/RESULT.md):
    /// on the plateau at 0.334 and 0.473 mA rising, the worst point at 0.111 mA
    /// falling, and 0.826 mA falling, above the table, on the normal line.
    #[test]
    fn the_scored_comparison_at_four_points() {
        let vc_mv = CHARACTERISTIC_VOLTAGE_V * 1e3;
        let rising = solve(&held(&[0.000333990568984765, 0.00047315330606175], Start::Rest)).expect("solves");
        assert_eq!(rising.mean_voltage_v, vec![0.0028263306061555174, 0.002968828227797874]);
        assert!(((mv(&rising, 0) - 2.86507596) / vc_mv + 0.0270).abs() < 0.0005);
        assert!(((mv(&rising, 1) - 2.85245616) / vc_mv - 0.0811).abs() < 0.0005);
        let falling = solve(&held(&[0.000111330189661589, 0.0008256989066567801], Start::Running)).expect("solves");
        assert_eq!(falling.mean_voltage_v, vec![0.0020653733411307145, 0.0039705513770079285]);
        assert!(((mv(&falling, 0) - 2.58634236) / vc_mv + 0.3629).abs() < 0.0005);
        assert_eq!(falling.normal_line_reached, vec![0.0, 1.0]);
        assert!(COMPARED_RESULT.contains("11.84 percent") && COMPARED_RESULT.contains("the comparison failed"));
    }

}
