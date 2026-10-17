//! Fluid hammer in the liquid-nitrogen line of DLR's Fluid Transient Test
//! Facility (FTTF) at Lampoldshausen when its fast valve closes: a named
//! real object. 9.29 m of stainless steel 1.4541 tube, 19 mm bore and
//! 1.5 mm wall, in one and a half spiral turns inside a vacuum jacket, from
//! the high-pressure tank to the valve; the three measuring stations S1, S2
//! and S3 at 6.46, 47.3 and 88.2 percent of the length from the valve
//! (Klein, Traudt and Deeken, "Liquid Nitrogen Fluid Hammer: PS-LN2-Set1",
//! Zenodo 15526459, CC BY 4.0: Table 1 and Figure 1 of the overview).
//!
//! The question is the liquid's temperature, its steady mass flow, the
//! steady pressure at S1, the pressure behind the valve, the valve's stroke
//! time and the time to compute. The line, its wall, its friction and the
//! valve's measured stroke are this file's constants.
//!
//! The liquid. Density, speed of sound and viscosity of compressed liquid
//! nitrogen at the asked temperature, and its vapour pressure, from the NIST
//! Chemistry WebBook (Span et al. 2000; Lemmon and Jacobsen 2004), through
//! `ln2_nitrogen`. Liquid nitrogen stiffens strongly under pressure: at 86 K
//! its speed of sound rises from 771 m/s at 11 bar to 820 m/s at 61 bar,
//! and a surge of 30 to 80 bar runs on a liquid a few percent stiffer than
//! the one it started in. The line carries that: the compliance of the
//! liquid at each pressure, `1/(rho_0 c(p)^2)`, with `rho_0` the density at
//! S1's steady state and `c(p)` the WebBook's speed of sound along the
//! asked temperature. With that compliance the stored volume is the mass
//! the liquid gains over `rho_0` exactly, since `rho beta = 1/c^2`, and
//! the wave runs at `c(p)` against the wall's stretch. The sections carry
//! the stiffest, at the tube's first-yield pressure, and the rest is lumped
//! at the nodes (`line_transient::Stiffening`).
//!
//! The wall. `D psi / (E e)` with the thick-walled factor for a tube
//! anchored against axial movement (Wylie and Streeter 1993), Poisson's
//! ratio 0.3, and Young's modulus at the liquid's temperature from NIST's
//! cryogenic fit for austenitic stainless steel 304 (214 GPa at 85 K, 200
//! GPa at room temperature); 1.4541 is 304 stabilised with titanium.
//!
//! The method: `line_transient`'s characteristics on 93 sections of
//! 9.99 cm, so that S1, S2 and S3 fall within 2.6 mm of a node (6, 44
//! and 82 sections from the valve; against 186 sections the scored
//! quantities of the traced tests move by 0.5 percent or less, the first
//! rise at S3 by 2 to 4 percent), Churchill's friction with one equivalent
//! roughness for the spiral line, and, where the pressure falls to the
//! vapour pressure, a vapour cavity at the node (the discrete vapour cavity
//! model). The supply end holds the pressure that gives the asked steady
//! pressure at S1. The valve: its open fraction follows the stroke measured
//! in the three traced tests, scaled to the asked stroke time; it seals at
//! 0.04 of that stroke (past it the position goes on creeping for 30 ms);
//! and it passes the flow in series with the rest of the circuit to the
//! low-pressure tank, two orifices, the open valve's coefficient twice the
//! circuit's: `Q = c(tau) sqrt(2 dp / rho)`, `c` from the steady flow.
//!
//! The ledgers, held at every step: energy (kinetic, the liquid's
//! compression in the sections and in the lumps, the wall's strain, the
//! vapour cavities) against friction, the valve's work and the cavities'
//! loss; volume, which is mass over `rho_0`, against what the supply put in
//! and the valve let out. Both to about 1e-12 of their scales; the limit is
//! 1e-9.
//!
//! The real limits, each by name. The bore first yields at 25.41 MPa
//! absolute (Tresca on Lamé's stresses, the jacket's vacuum outside, the
//! 200 MPa minimum proof strength of cold-finished 1.4541 tube, EN
//! 10216-5; the steel is stronger cold, so it yields no lower): a supply
//! up to it is taken, one above it is refused, and a surge that passes it
//! stops there (`WallYield`). A temperature at or below absolute zero is no
//! temperature (`TemperatureNotPositive`). Below nitrogen's triple point,
//! 63.151 K, it is solid at every pressure; up to 68.5614 K its melting
//! line (Span et al. 2000) lies under the bore's yield pressure, meeting it
//! at 68.56138 K, so a surge the tube could still hold might freeze the
//! liquid: the conservative rule, which refuses every such question
//! whatever its own surge (`LiquidFreezesBelowWallYield`). Above
//! 109.90876941859975 K the saturated vapour is more than a tenth as dense
//! as the liquid, by the table's own saturated densities, and the vapour
//! cavity model, which gives the cavity no mass, does not hold
//! (`VapourNotLightAgainstLiquid`). The line equations drop the flow's own
//! convection and hold to a tenth of the wave speed. The vapour may take a
//! tenth of the line's volume: past it the separated column's lost mass is
//! a tenth of the line's and the lumped cavity does not stand for it.
//! A stroke under ten steps of the grid rings at the grid's scale. The
//! circuit behind the valve ends in a tank of liquid nitrogen, which
//! exists only from the triple-point pressure, 12,523 Pa, up
//! (`ReturnBelowTriplePoint`).
//!
//! What was compared. Three of the facility's nine tests were opened and
//! fixed the roughness and the valve's seat and series ratio; the other six
//! were held back until a protocol with eleven marks and a forecast of
//! every quantity was committed and fingerprinted ([`PROTOCOL_SHA256`]),
//! then scored once (`scored::dlr_scored_once`; record
//! `evidence/research/hydraulics_robotics/dlr_ln2_line/`). Met, each in at
//! least five of six: the first rise at S2 (all six within 1.7 percent),
//! the front's speed from S1 to S2 (within 0.7 percent), the second surge's
//! time (within 8.1 percent of its 15) and the steady drop from S3 to S1
//! (within 9.3 percent of its 10). Missed: the first rise at S1, 4.8 to 6.8
//! percent high (2 of 6 within 5; it is the overview's dP, published for
//! all nine); the first rise at S3, 13 to 15 percent low in five (1 of 6
//! within 10); the front from S2 to S3, inside 3 percent in the four tests
//! whose dynamic sensor at S3 recorded a signal and unmeasured in two (4 of
//! 6); the plateau at S1, 4.2 to 7.4 percent short (0 of 6 within 3); the
//! second surge's height, 7 to 111 percent high (3 of 6 within 20); the
//! late free oscillation (0 of 6 within 5 and 30). The late fit failed on
//! the computed traces of the two lowest-pressure tests, its leftover 1.0
//! and 3.5 times its amplitude (a fit is taken where its leftover is under
//! half its amplitude, a rule read after the scoring), so its rates there
//! are not rates; on the other four the computed period
//! is 10.0 to 13.1 percent short and the decay 78 to 85 percent under the
//! measured, and on the two, read from the computed trace's envelope, the
//! decay is 84 and 91 percent under.
//!
//! What the model holds: the front runs at the wave speed `a`, not `a - v`
//! against the flow (0.6 to 1.7 percent at the measured flows); the liquid
//! holds the asked temperature through its compression (the WebBook's
//! isentropic speed of sound is used; an 80 bar adiabatic compression warms
//! it about a kelvin); a cavity holds vapour at the vapour pressure and
//! nothing else; friction is quasi-steady, Churchill's factor at the
//! instantaneous velocity; the tube is anchored, and the spiral's curvature
//! is in the fitted roughness alone.

use crate::normal::NormalOrZero;
use std::fmt;

use crate::line_transient::{self as line, Extensions, LineStop, Liquid, Pipe, Run, Stiffening, Tamper, ValveSchedule};
use crate::ln2_nitrogen as n2;

/// Length of the line from the tank to the valve, metres.
pub const LINE_LENGTH_M: f64 = 9.29;
/// Bore, metres.
pub const BORE_M: f64 = 0.019;
/// Wall thickness, metres.
pub const WALL_THICKNESS_M: f64 = 0.0015;
/// Outer diameter, metres.
pub const OUTER_DIAMETER_M: f64 = BORE_M + 2.0 * WALL_THICKNESS_M;
/// The measuring stations' distances from the valve as shares of the
/// length (the overview's Table 1, with Figure 1's S1 at the valve).
pub const S1_FROM_VALVE: f64 = 0.0646;
pub const S2_FROM_VALVE: f64 = 0.473;
pub const S3_FROM_VALVE: f64 = 0.882;
/// Sections of the line.
pub const SECTIONS: usize = 93;
/// The nodes nearest S1, S2 and S3, counted in sections from the valve.
pub const S1_SECTIONS_FROM_VALVE: usize = 6;
pub const S2_SECTIONS_FROM_VALVE: usize = 44;
pub const S3_SECTIONS_FROM_VALVE: usize = 82;
/// Poisson's ratio of the steel.
pub const STEEL_POISSON_RATIO: f64 = 0.3;
/// The minimum 0.2 percent proof strength of cold-finished 1.4541 tube at
/// room temperature (EN 10216-5), pascals.
pub const STEEL_PROOF_STRENGTH_PA: f64 = 200.0e6;
/// NIST's fit of Young's modulus of 304 stainless steel from 57 K to
/// 293 K, `E = a + bT + cT^2 + dT^3 + eT^4` in GPa (NIST cryogenic
/// material properties, retrieved 2026-10-14; one percent against its data).
pub const STEEL_MODULUS_FIT_GPA: [f64; 5] = [2.100593e2, 1.534883e-1, -1.617390e-3, 5.117060e-6, -6.154600e-9];
/// Absolute pressure at which the bore first yields: Tresca on Lamé's
/// stresses with the jacket's vacuum outside,
/// `p = sigma (r_o^2 - r_i^2) / (2 r_o^2)`.
pub const WALL_YIELD_PRESSURE_PA: f64 = 25_413_223.140_495_87;
/// The equivalent roughness of the spiral line, metres: see
/// [`ROUGHNESS_ACCOUNT`].
pub const ROUGHNESS_M: f64 = 1.253e-5;
/// How the roughness was chosen.
pub const ROUGHNESS_ACCOUNT: &str = "one equivalent roughness for the spiral line, 12.53 um, fitted by least squares in the logarithm of Churchill's factor to the steady pressure drop between S3 and S1 in the three traced tests together (20200806_13, 20200803_11, 20200806_16), each sensor's offset taken from the line at rest at the end of its record, the liquid at the Coriolis meter's density; it leaves those three drops 12.1 percent high, 7.6 and 2.8 percent low. The roughness of 4.6 um fitted earlier to one test alone is not used";
/// The coldest liquid, kelvin: below it nitrogen's melting pressure is
/// under the bore's first-yield pressure, so a surge the tube holds could
/// freeze the line. A conservative rule: the run's own surge may stay under
/// the melting pressure.
pub const COLDEST_K: f64 = 68.5614;
/// The warmest liquid, kelvin: the largest 64-bit temperature at which the
/// table's saturated vapour is at most [`VAPOUR_DENSITY_SHARE_MAX`] as dense
/// as its saturated liquid; one number above, it is more.
pub const WARMEST_K: f64 = 109.908_769_418_599_75;
/// The vapour's density over the liquid's at [`WARMEST_K`].
pub const VAPOUR_DENSITY_SHARE_MAX: f64 = 0.1;
/// The largest Mach number the line equations stand for.
pub const MACH_LIMIT: f64 = 0.1;
/// The share of the line's volume the vapour may take.
pub const VAPOUR_SHARE_LIMIT: f64 = 0.1;
/// The stroke must span this many steps of the grid.
pub const STROKE_STEPS_MIN: f64 = 10.0;
/// The shortest stroke any question may ask, seconds: [`STROKE_STEPS_MIN`]
/// steps of the grid at [`COLDEST_K`] with S1 just above the vapour
/// pressure, where the step is shortest (the sections' wave speed is the
/// liquid's at the yield pressure against the wall, and the wall weighs
/// with the liquid's density at S1). Any other question's steps are longer,
/// and so is its shortest stroke: about 1.049 ms at 83.7 K and 11 bar,
/// 1.255 ms at the warmest liquid.
pub const STROKE_TIME_MIN_S: f64 = 9.524_182_150_414_855e-4;
/// The longest run, seconds: a budget of compute, not a physical limit. Ten
/// seconds are about 95,000 steps, one to two seconds of computing on one
/// core with every history.
pub const DURATION_MAX_S: f64 = 10.0;
/// The most samples of each history.
pub const SAMPLES: usize = 500;
/// The valve's stroke: its open fraction every half millisecond from the
/// start of closure, the mean of the position traces of the three traced
/// tests, each from where it passed 0.95 of open, taken as the open area's
/// share; closed after the last value. From 0.9 to 0.1 of open it takes
/// [`STROKE_REFERENCE_S`].
pub const STROKE_STEP_S: f64 = 0.0005;
pub const STROKE: [f64; 92] = [
    1.0000, 0.9917, 0.9771, 0.9533, 0.9162, 0.8676, 0.8091, 0.7411, 0.6720, 0.6100, 0.5549, 0.5025, 0.4513, 0.3974, 0.3374, 0.2773, 0.2194,
    0.1663, 0.1227, 0.0958, 0.0797, 0.0726, 0.0722, 0.0675, 0.0629, 0.0604, 0.0596, 0.0585, 0.0546, 0.0530, 0.0530, 0.0504, 0.0475, 0.0465,
    0.0463, 0.0434, 0.0411, 0.0406, 0.0392, 0.0368, 0.0349, 0.0335, 0.0321, 0.0308, 0.0301, 0.0298, 0.0295, 0.0286, 0.0275, 0.0262, 0.0243,
    0.0225, 0.0212, 0.0204, 0.0193, 0.0178, 0.0167, 0.0162, 0.0162, 0.0162, 0.0162, 0.0162, 0.0162, 0.0162, 0.0162, 0.0162, 0.0162, 0.0162,
    0.0162, 0.0162, 0.0148, 0.0132, 0.0121, 0.0102, 0.0092, 0.0087, 0.0084, 0.0078, 0.0067, 0.0067, 0.0062, 0.0062, 0.0062, 0.0062, 0.0062,
    0.0062, 0.0062, 0.0062, 0.0062, 0.0062, 0.0062, 0.0,
];
/// The stroke's time from 0.9 to 0.1 of open, seconds.
pub const STROKE_REFERENCE_S: f64 = 0.007_255_5;
/// The valve's coefficient fully open over the coefficient of the
/// restriction behind it, in series: the line's flow passes the open valve
/// and then the rest of the circuit to the low-pressure tank, so the flow
/// falls only when the valve's own opening nears the circuit's. See
/// [`VALVE_ACCOUNT`].
pub const VALVE_SERIES_RATIO: f64 = 2.0;
/// How the valve's ratio was chosen.
pub const VALVE_ACCOUNT: &str = "the series ratio 2.0 and the seat at 0.04 of the stroke, chosen from a grid (ratios 1 to 4, seats 0.04 to 0.12) as the pair whose computed first rise at S1 crosses 10, 25, 50, 75 and 90 percent of its height nearest the times the three traced tests did, counted from the stroke's passing 0.95 of open (root mean square 1.6 ms); seats under 0.03 let the stroke's slow tail leak and leave the rise at S2 ten percent under S1's, where the traced tests had it four percent under";

/// The valve's open share of the circuit's coefficient at a stroke's open
/// fraction: two orifices in series, `c = c_v tau c_d / sqrt((c_v tau)^2 + c_d^2)`,
/// over its value fully open.
pub fn valve_effective_opening(open_fraction: f64, series_ratio: f64, seat: f64) -> f64 {
    let r = series_ratio;
    let tau = ((open_fraction - seat) / (1.0 - seat)).max(0.0);
    tau * (1.0 + r * r).sqrt() / (1.0 + r * r * tau * tau).sqrt()
}

/// The open fraction of the measured stroke at which the valve seals: past
/// it the position goes on creeping toward its last reading while no flow
/// passes. See [`VALVE_ACCOUNT`].
pub const VALVE_SEAT: f64 = 0.04;

/// The SHA-256 of the validation protocol and forecast, written and
/// committed before the six held-back tests' traces were downloaded
/// (`evidence/research/hydraulics_robotics/dlr_ln2_line/validation_protocol.md`).
pub const PROTOCOL_SHA256: &str = "9742e60eb828783056b423e960c65b1d62bcc4142fdb5324a5fcfb24dd4cdf99";

/// The public record the solver is compared with.
pub const REFERENCE_NAME: &str = "S. Klein, T. Traudt and J. Deeken, Liquid Nitrogen Fluid Hammer: PS-LN2-Set1, DLR Institute of Space Propulsion, Lampoldshausen: nine tests on the Fluid Transient Test Facility, the overview's Tables 1 and 3 and Figure 1, and each test's pressure, dynamic pressure, valve, temperature and Coriolis records";
pub const REFERENCE_URL: &str = "https://doi.org/10.5281/zenodo.15526459";
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";

/// Area of the bore, m^2.
pub fn bore_area_m2() -> f64 {
    0.25 * std::f64::consts::PI * BORE_M * BORE_M
}

/// Young's modulus of the steel at a temperature, pascals.
pub fn steel_modulus_pa(temperature_k: f64) -> f64 {
    let [a, b, c, d, e] = STEEL_MODULUS_FIT_GPA;
    let t = temperature_k;
    (a + t * (b + t * (c + t * (d + t * e)))) * 1.0e9
}

/// The tube's wall factor `psi`, thick-walled, anchored against axial movement.
pub fn wall_factor() -> f64 {
    let (d, e, nu) = (BORE_M, WALL_THICKNESS_M, STEEL_POISSON_RATIO);
    2.0 * (e / d) * (1.0 + nu) + d * (1.0 - nu * nu) / (d + e)
}

/// The wall's compliance per pascal at a temperature, `D psi / (E e)`.
pub fn wall_compliance_per_pa(temperature_k: f64) -> f64 {
    BORE_M * wall_factor() / (steel_modulus_pa(temperature_k) * WALL_THICKNESS_M)
}

/// Length of one section, metres.
pub fn section_m() -> f64 {
    LINE_LENGTH_M / SECTIONS as f64
}

/// One question.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineQuestion {
    /// Temperature of the liquid in the line, kelvin.
    pub temperature_k: f64,
    /// Steady mass flow toward the valve before it moves, kg/s.
    pub mass_flow_kg_s: f64,
    /// Steady absolute pressure at S1 before the valve moves.
    pub s1_pressure_pa: f64,
    /// Absolute pressure behind the valve, held.
    pub return_pressure_pa: f64,
    /// The valve's stroke from 0.9 to 0.1 of open, seconds.
    pub stroke_time_s: f64,
    /// Time computed from the start of the stroke, seconds.
    pub duration_s: f64,
}

/// The state of the liquid and the line the question sets.
#[derive(Clone, Debug, PartialEq)]
pub struct LineState {
    pub vapour_pressure_pa: f64,
    pub density_kg_m3: f64,
    pub sound_speed_m_s: f64,
    pub viscosity_pa_s: f64,
    pub steel_modulus_pa: f64,
    /// The wave speed at S1's steady pressure: the liquid and the wall.
    pub wave_speed_m_s: f64,
    /// The sections' wave speed, at the yield pressure.
    pub section_wave_speed_m_s: f64,
    pub velocity_m_s: f64,
    pub reynolds_number: f64,
    pub friction_factor: f64,
    pub supply_pressure_pa: f64,
    /// Joukowsky's rise `m a / A` with the wave speed at S1's steady pressure.
    pub joukowsky_rise_pa: f64,
    /// The time step of the grid.
    pub time_step_s: f64,
    /// The stiffening's pressures and excess compliances.
    pub stiffening: Stiffening,
}

/// What the line did.
#[derive(Clone, Debug, PartialEq)]
pub struct LineHammer {
    pub state: LineState,
    pub steady_s1_pa: f64,
    pub steady_s2_pa: f64,
    pub steady_s3_pa: f64,
    pub steady_valve_pa: f64,
    pub time_step_s: f64,
    pub steps: u64,
    /// The highest pressure at each station, and when.
    pub s1_max_pa: f64,
    pub s1_max_time_s: f64,
    pub s2_max_pa: f64,
    pub s2_max_time_s: f64,
    pub s3_max_pa: f64,
    pub s3_max_time_s: f64,
    pub line_pressure_max_pa: f64,
    pub line_pressure_min_pa: f64,
    pub first_vapour_time_s: Vec<f64>,
    pub first_vapour_position_m: Vec<f64>,
    pub vapour_volume_max_m3: f64,
    pub largest_line_mach: f64,
    pub sample_times_s: Vec<f64>,
    pub s1_history_pa: Vec<f64>,
    pub s2_history_pa: Vec<f64>,
    pub s3_history_pa: Vec<f64>,
    pub valve_history_pa: Vec<f64>,
    pub valve_flow_history_m3_s: Vec<f64>,
    pub valve_vapour_history_m3: Vec<f64>,
    pub kinetic_energy_history_j: Vec<f64>,
    pub liquid_energy_history_j: Vec<f64>,
    pub wall_energy_history_j: Vec<f64>,
    pub vapour_energy_history_j: Vec<f64>,
    pub friction_loss_history_j: Vec<f64>,
    pub valve_work_history_j: Vec<f64>,
    pub cavity_loss_history_j: Vec<f64>,
    pub stored_energy_start_j: f64,
    pub stored_energy_end_j: f64,
    pub friction_loss_j: f64,
    pub valve_work_j: f64,
    pub cavity_loss_j: f64,
    pub largest_energy_ledger_residual: f64,
    pub largest_volume_ledger_residual: f64,
    /// Every step's pressure at S1, S2 and S3, one after the other, then the
    /// final pressure and velocity at every node.
    pub field: Vec<f64>,
}

impl LineHammer {
    /// Every step's pressure at a station (0, 1, 2 for S1, S2, S3).
    pub fn station_every_step_pa(&self, station: usize) -> &[f64] {
        let n = self.steps as usize + 1;
        &self.field[station * n..(station + 1) * n]
    }
}

/// Why no transient was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum LineReject {
    /// An input was NaN or infinite.
    InputNotFinite { field: &'static str },
    /// The temperature is not above absolute zero.
    TemperatureNotPositive { temperature_k: f64 },
    /// Below [`COLDEST_K`] liquid nitrogen freezes at a pressure the tube holds: its melting pressure (Span et al. 2000) is under the bore's first-yield pressure, and below the triple point it is solid at every pressure.
    LiquidFreezesBelowWallYield { temperature_k: f64, melting_pressure_pa: Option<f64> },
    /// Above [`WARMEST_K`] the saturated vapour is more than a tenth as dense as the liquid, and the vapour cavity model, which gives the cavity no mass, does not hold.
    VapourNotLightAgainstLiquid { temperature_k: f64, vapour_over_liquid: f64 },
    /// The steady pressure at S1 is not above the liquid's vapour pressure: the line does not hold liquid.
    S1PressureNotAboveVapour { s1_pressure_pa: f64, vapour_pressure_pa: f64 },
    /// The supply pressure the steady flow needs is above the bore's first-yield pressure.
    SupplyAboveWallYield { supply_pressure_pa: f64 },
    /// The mass flow is negative: the line runs from the tank to the valve.
    MassFlowNegative { mass_flow_kg_s: f64 },
    /// The return pressure is below nitrogen's triple-point pressure: the circuit behind the valve ends in a tank of liquid nitrogen, and below it there is none.
    ReturnBelowTriplePoint { return_pressure_pa: f64 },
    /// Friction to the valve takes the pressure below the return pressure: the steady flow cannot run.
    SupplyCannotDriveFlow { valve_pressure_pa: f64 },
    /// The steady pressure somewhere on the line is not above the vapour pressure.
    SteadyBelowVapour { pressure_pa: f64 },
    /// The steady flow is faster than [`MACH_LIMIT`] of the wave speed.
    InitialFlowMachAboveAcoustic { mach: f64 },
    /// The duration is not above zero or is above [`DURATION_MAX_S`].
    DurationOutOfRange { duration_s: f64 },
    /// The stroke time is negative or longer than the duration.
    StrokeTimeOutOfRange { stroke_time_s: f64 },
    /// The stroke is faster than [`STROKE_STEPS_MIN`] steps of the grid, which it does not resolve.
    StrokeFasterThanGrid { stroke_time_s: f64, steps_min_s: f64 },
    /// The transient reached the bore's first-yield pressure.
    WallYield { time_s: f64, position_m: f64, pressure_pa: f64 },
    /// The flow at a node reached [`MACH_LIMIT`] of the local wave speed.
    LineMachAboveAcoustic { time_s: f64, position_m: f64, mach: f64 },
    /// The vapour cavities together took more than [`VAPOUR_SHARE_LIMIT`] of the line's volume.
    VapourOverLineShare { time_s: f64, vapour_over_limit: f64 },
    /// The friction of a section did not settle.
    FrictionNotSettled { time_s: f64, position_m: f64 },
    /// A node did not settle.
    NodeNotSettled { time_s: f64, position_m: f64 },
    /// A value was NaN or infinite.
    NonFiniteValue { time_s: f64 },
    /// The energy ledger did not close.
    EnergyLedgerBroken { time_s: f64, residual: f64 },
    /// The volume ledger did not close.
    VolumeLedgerBroken { time_s: f64, residual: f64 },
    /// A section's friction gave energy to the flow.
    FrictionGaveEnergy { time_s: f64, position_m: f64, work_j: f64 },
    /// A node gave energy to the waves.
    CavityGaveEnergy { time_s: f64, position_m: f64, work_j: f64 },
}

impl fmt::Display for LineReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotFinite { field } => write!(f, "{field} is not a finite number"),
            Self::TemperatureNotPositive { temperature_k } => write!(f, "temperature {temperature_k} K is not above absolute zero"),
            Self::LiquidFreezesBelowWallYield { temperature_k, melting_pressure_pa: Some(melting) } => write!(
                f,
                "at {temperature_k} K liquid nitrogen freezes at {melting} Pa, below the {WALL_YIELD_PRESSURE_PA} Pa at which the tube's bore first yields: a surge the tube holds can freeze the line (the coldest liquid is {COLDEST_K} K)"
            ),
            Self::LiquidFreezesBelowWallYield { temperature_k, melting_pressure_pa: None } => write!(
                f,
                "at {temperature_k} K, below nitrogen's triple point, {} K, nitrogen is solid at every pressure: the line holds no liquid (the coldest liquid is {COLDEST_K} K)",
                n2::TRIPLE_POINT_K
            ),
            Self::VapourNotLightAgainstLiquid { temperature_k, .. } if *temperature_k >= n2::CRITICAL_POINT_K => write!(
                f,
                "at {temperature_k} K, at or above nitrogen's critical temperature, {} K, there is no liquid at any pressure (the warmest liquid is {WARMEST_K} K)",
                n2::CRITICAL_POINT_K
            ),
            Self::VapourNotLightAgainstLiquid { temperature_k, vapour_over_liquid } if *temperature_k > n2::table_warmest_k() => write!(
                f,
                "at {temperature_k} K, warmer than {} K, the table's warmest isotherm, the saturated vapour is more than {vapour_over_liquid} as dense as the liquid, its share there, above {VAPOUR_DENSITY_SHARE_MAX}: the vapour cavity model, which gives the cavity no mass, does not hold (the warmest liquid is {WARMEST_K} K)",
                n2::table_warmest_k()
            ),
            Self::VapourNotLightAgainstLiquid { temperature_k, vapour_over_liquid } => write!(
                f,
                "at {temperature_k} K the saturated vapour is {vapour_over_liquid} as dense as the liquid, above {VAPOUR_DENSITY_SHARE_MAX}: the vapour cavity model, which gives the cavity no mass, does not hold (the warmest liquid is {WARMEST_K} K)"
            ),
            Self::S1PressureNotAboveVapour { s1_pressure_pa, vapour_pressure_pa } => write!(
                f,
                "the steady pressure at S1, {s1_pressure_pa} Pa, is not above the liquid's vapour pressure, {vapour_pressure_pa} Pa"
            ),
            Self::SupplyAboveWallYield { supply_pressure_pa } => write!(
                f,
                "the steady flow needs a supply pressure of {supply_pressure_pa} Pa, above the {WALL_YIELD_PRESSURE_PA} Pa at which the tube's bore first yields"
            ),
            Self::MassFlowNegative { mass_flow_kg_s } => write!(f, "mass flow {mass_flow_kg_s} kg/s is negative: the line runs from the tank to the valve"),
            Self::ReturnBelowTriplePoint { return_pressure_pa } => write!(
                f,
                "return pressure {return_pressure_pa} Pa is below nitrogen's triple-point pressure, {} Pa: no tank of liquid nitrogen holds it",
                n2::TRIPLE_POINT_PA
            ),
            Self::SupplyCannotDriveFlow { valve_pressure_pa } => write!(
                f,
                "the steady pressure at the valve would be {valve_pressure_pa} Pa, not above the return pressure: the steady flow cannot run"
            ),
            Self::SteadyBelowVapour { pressure_pa } => write!(f, "the steady pressure on the line falls to {pressure_pa} Pa, not above the vapour pressure"),
            Self::InitialFlowMachAboveAcoustic { mach } => write!(f, "the steady flow is Mach {mach} against the wave speed, above {MACH_LIMIT}, where the line equations hold"),
            Self::DurationOutOfRange { duration_s } => write!(f, "duration {duration_s} s is not above 0 s and at most {DURATION_MAX_S} s, the solver's budget of compute"),
            Self::StrokeTimeOutOfRange { stroke_time_s } => write!(f, "stroke time {stroke_time_s} s is negative or longer than the duration"),
            Self::StrokeFasterThanGrid { stroke_time_s, steps_min_s } => write!(
                f,
                "a stroke of {stroke_time_s} s is faster than {STROKE_STEPS_MIN} steps of the grid, {steps_min_s} s, which it does not resolve"
            ),
            Self::WallYield { time_s, position_m, pressure_pa } => write!(
                f,
                "at {time_s} s, {position_m} m from the tank, the pressure reached {pressure_pa} Pa, where the tube's bore first yields ({WALL_YIELD_PRESSURE_PA} Pa)"
            ),
            Self::LineMachAboveAcoustic { time_s, position_m, mach } => write!(
                f,
                "at {time_s} s, {position_m} m from the tank, the flow reached Mach {mach} against the local wave speed, above {MACH_LIMIT}"
            ),
            Self::VapourOverLineShare { time_s, vapour_over_limit } => write!(
                f,
                "at {time_s} s the vapour cavities held {vapour_over_limit} times {VAPOUR_SHARE_LIMIT} of the line's volume: the lumped cavity no longer stands for the separated column"
            ),
            Self::FrictionNotSettled { time_s, position_m } => write!(f, "at {time_s} s the friction of the section at {position_m} m did not settle in {} passes", line::MAX_FRICTION_PASSES),
            Self::NodeNotSettled { time_s, position_m } => write!(
                f,
                "at {time_s} s the node at {position_m} m did not settle in {} passes of false position and {} of halving",
                line::MAX_NODE_PASSES,
                line::MAX_BISECTION_PASSES
            ),
            Self::NonFiniteValue { time_s } => write!(f, "at {time_s} s a value of the transient was not finite"),
            Self::EnergyLedgerBroken { time_s, residual } => write!(f, "at {time_s} s the energy ledger was off by {residual} of its scale, above {}", line::ENERGY_RESIDUAL_LIMIT),
            Self::VolumeLedgerBroken { time_s, residual } => write!(f, "at {time_s} s the volume ledger was off by {residual} of its scale, above {}", line::VOLUME_RESIDUAL_LIMIT),
            Self::FrictionGaveEnergy { time_s, position_m, work_j } => write!(f, "at {time_s} s the friction of the section at {position_m} m gave {work_j} J to the flow"),
            Self::CavityGaveEnergy { time_s, position_m, work_j } => write!(f, "at {time_s} s the node at {position_m} m gave {work_j} J to the waves"),
        }
    }
}

fn rejected(stop: LineStop) -> LineReject {
    match stop {
        LineStop::SupplyCannotDriveFlow { valve_pressure_pa } => LineReject::SupplyCannotDriveFlow { valve_pressure_pa },
        LineStop::SteadyBelowVapour { pressure_pa } => LineReject::SteadyBelowVapour { pressure_pa },
        LineStop::WallYield { time_s, position_m, pressure_pa } => LineReject::WallYield { time_s, position_m, pressure_pa },
        LineStop::LineMachAboveAcoustic { time_s, position_m, mach } => LineReject::LineMachAboveAcoustic { time_s, position_m, mach },
        LineStop::CavityLargerThanReach { time_s, cavity_over_reach, .. } => LineReject::VapourOverLineShare { time_s, vapour_over_limit: cavity_over_reach },
        // The line carries no free gas and its run asks no void limit.
        LineStop::VoidAboveBubblyFlow { .. } => unreachable!("ln2_line_dlr_fttf's line carries no free gas"),
        LineStop::FrictionNotSettled { time_s, position_m } => LineReject::FrictionNotSettled { time_s, position_m },
        LineStop::NodeNotSettled { time_s, position_m } => LineReject::NodeNotSettled { time_s, position_m },
        LineStop::NonFiniteValue { time_s } => LineReject::NonFiniteValue { time_s },
        LineStop::EnergyLedgerBroken { time_s, residual } => LineReject::EnergyLedgerBroken { time_s, residual },
        LineStop::VolumeLedgerBroken { time_s, residual } => LineReject::VolumeLedgerBroken { time_s, residual },
        LineStop::FrictionGaveEnergy { time_s, position_m, work_j } => LineReject::FrictionGaveEnergy { time_s, position_m, work_j },
        LineStop::CavityGaveEnergy { time_s, position_m, work_j } => LineReject::CavityGaveEnergy { time_s, position_m, work_j },
    }
}

/// The liquid's properties for a question: none where the table holds no
/// liquid there (refused before by name).
fn liquid_at(temperature_k: f64, pressure_pa: f64) -> Option<n2::LiquidState> {
    n2::liquid(temperature_k, pressure_pa)
}

/// The stiffening along the asked temperature, from the vapour pressure to
/// the yield pressure in steps of 2.5 bar, the compliance of each over the
/// reference density.
fn stiffening(temperature_k: f64, vapour_pa: f64, rho_0: f64) -> Option<(Stiffening, f64)> {
    let top = liquid_at(temperature_k, WALL_YIELD_PRESSURE_PA)?;
    let compliance = |c: f64| 1.0 / (rho_0 * c * c);
    let top_compliance = compliance(top.sound_speed_m_s);
    let step = 2.5 * n2::PA_PER_BAR;
    let mut pressure_pa = Vec::new();
    let mut excess = Vec::new();
    let mut k = 0usize;
    loop {
        let p = vapour_pa + k as f64 * step;
        // The last step ends at the yield pressure, at least half a step long.
        if p > WALL_YIELD_PRESSURE_PA - 0.5 * step {
            break;
        }
        let s = liquid_at(temperature_k, p)?;
        pressure_pa.push(p);
        excess.push(compliance(s.sound_speed_m_s) - top_compliance);
        k += 1;
    }
    pressure_pa.push(WALL_YIELD_PRESSURE_PA);
    excess.push(0.0);
    Some((Stiffening { pressure_pa, excess_compliance_per_pa: excess }, top.sound_speed_m_s))
}

/// The pipe of the run: its wave speed is the stiffest.
fn pipe(state: &LineState, temperature_k: f64, roughness_m: f64) -> Pipe {
    let wall = wall_compliance_per_pa(temperature_k);
    let top = 1.0 / (state.density_kg_m3 * state.section_wave_speed_m_s * state.section_wave_speed_m_s);
    Pipe {
        length_m: LINE_LENGTH_M,
        sections: SECTIONS,
        diameter_m: BORE_M,
        roughness_m,
        wave_speed_m_s: state.section_wave_speed_m_s,
        wall_share: wall / top,
    }
}

/// Every limit the question shows, and the state it sets.
pub fn admit(question: &LineQuestion) -> Result<LineState, LineReject> {
    admit_with(question, ROUGHNESS_M)
}

fn admit_with(question: &LineQuestion, roughness_m: f64) -> Result<LineState, LineReject> {
    for (field, value) in [
        ("temperature_k", question.temperature_k),
        ("mass_flow_kg_s", question.mass_flow_kg_s),
        ("s1_pressure_pa", question.s1_pressure_pa),
        ("return_pressure_pa", question.return_pressure_pa),
        ("stroke_time_s", question.stroke_time_s),
        ("duration_s", question.duration_s),
    ] {
        if !value.is_finite() {
            return Err(LineReject::InputNotFinite { field });
        }
    }
    let t = question.temperature_k;
    if !(t > 0.0) {
        return Err(LineReject::TemperatureNotPositive { temperature_k: t });
    }
    if t < COLDEST_K {
        let melting_pressure_pa = (t >= n2::TRIPLE_POINT_K).then(|| n2::melting_pressure_pa(t));
        return Err(LineReject::LiquidFreezesBelowWallYield { temperature_k: t, melting_pressure_pa });
    }
    if t > WARMEST_K {
        // The share rises with the temperature; past the table, its share at
        // the table's warmest isotherm, which the warmer liquid's passes.
        let vapour_over_liquid = n2::saturated_vapour_over_liquid(t.min(n2::table_warmest_k())).expect("inside the table");
        return Err(LineReject::VapourNotLightAgainstLiquid { temperature_k: t, vapour_over_liquid });
    }
    let vapour = n2::vapour_pressure_pa(t).expect("inside the table");
    if !(question.s1_pressure_pa > vapour) {
        return Err(LineReject::S1PressureNotAboveVapour { s1_pressure_pa: question.s1_pressure_pa, vapour_pressure_pa: vapour });
    }
    if question.s1_pressure_pa > WALL_YIELD_PRESSURE_PA {
        return Err(LineReject::SupplyAboveWallYield { supply_pressure_pa: question.s1_pressure_pa });
    }
    if question.mass_flow_kg_s < 0.0 {
        return Err(LineReject::MassFlowNegative { mass_flow_kg_s: question.mass_flow_kg_s });
    }
    if !(question.return_pressure_pa >= n2::TRIPLE_POINT_PA) {
        return Err(LineReject::ReturnBelowTriplePoint { return_pressure_pa: question.return_pressure_pa });
    }
    if !(question.duration_s > 0.0 && question.duration_s <= DURATION_MAX_S) {
        return Err(LineReject::DurationOutOfRange { duration_s: question.duration_s });
    }
    if !(question.stroke_time_s >= 0.0 && question.stroke_time_s <= question.duration_s) {
        return Err(LineReject::StrokeTimeOutOfRange { stroke_time_s: question.stroke_time_s });
    }
    let s1 = liquid_at(t, question.s1_pressure_pa).expect("between the vapour pressure and the yield pressure");
    let rho_0 = s1.density_kg_m3;
    let (stiff, c_top) = stiffening(t, vapour, rho_0).expect("the table reaches the yield pressure at every admitted temperature");
    let wall = wall_compliance_per_pa(t);
    let speed = |c: f64| 1.0 / (1.0 / (c * c) + rho_0 * wall).sqrt();
    let section_speed = speed(c_top);
    let dt = section_m() / section_speed;
    if question.stroke_time_s < STROKE_STEPS_MIN * dt {
        return Err(LineReject::StrokeFasterThanGrid { stroke_time_s: question.stroke_time_s, steps_min_s: STROKE_STEPS_MIN * dt });
    }
    let velocity = question.mass_flow_kg_s / (rho_0 * bore_area_m2());
    let wave_speed = speed(s1.sound_speed_m_s);
    let mach = velocity / wave_speed;
    if mach > MACH_LIMIT {
        return Err(LineReject::InitialFlowMachAboveAcoustic { mach });
    }
    let nu = s1.viscosity_pa_s / rho_0;
    let reynolds = velocity * BORE_M / nu;
    let friction_factor = line::churchill_friction_factor(reynolds, roughness_m / BORE_M);
    let mut state = LineState {
        vapour_pressure_pa: vapour,
        density_kg_m3: rho_0,
        sound_speed_m_s: s1.sound_speed_m_s,
        viscosity_pa_s: s1.viscosity_pa_s,
        steel_modulus_pa: steel_modulus_pa(t),
        wave_speed_m_s: wave_speed,
        section_wave_speed_m_s: section_speed,
        velocity_m_s: velocity,
        reynolds_number: reynolds,
        friction_factor,
        supply_pressure_pa: 0.0,
        joukowsky_rise_pa: question.mass_flow_kg_s * wave_speed / bore_area_m2(),
        time_step_s: dt,
        stiffening: stiff,
    };
    // The supply that leaves the asked pressure at S1: the steady drop of
    // the sections from the tank to S1.
    let p = pipe(&state, t, roughness_m);
    let liquid = Liquid { density_kg_m3: rho_0, kinematic_viscosity_m2_s: nu, vapour_pressure_pa: vapour };
    let drop = section_m() * line::friction_per_volume(velocity, &p, &liquid);
    let mut supply = question.s1_pressure_pa;
    for _ in 0..(SECTIONS - S1_SECTIONS_FROM_VALVE) {
        supply += drop;
    }
    if supply > WALL_YIELD_PRESSURE_PA {
        return Err(LineReject::SupplyAboveWallYield { supply_pressure_pa: supply });
    }
    let mut valve = question.s1_pressure_pa;
    for _ in 0..S1_SECTIONS_FROM_VALVE {
        valve -= drop;
    }
    if valve <= vapour {
        return Err(LineReject::SteadyBelowVapour { pressure_pa: valve });
    }
    if question.mass_flow_kg_s > 0.0 && valve <= question.return_pressure_pa {
        return Err(LineReject::SupplyCannotDriveFlow { valve_pressure_pa: valve });
    }
    state.supply_pressure_pa = supply;
    Ok(state)
}

/// The run the core is asked for.
fn line_run(question: &LineQuestion, state: &LineState, roughness_m: f64, series_ratio: f64, seat: f64) -> (Run, Extensions) {
    let liquid = Liquid {
        density_kg_m3: state.density_kg_m3,
        kinematic_viscosity_m2_s: state.viscosity_pa_s / state.density_kg_m3,
        vapour_pressure_pa: state.vapour_pressure_pa,
    };
    let run = Run {
        pipes: vec![pipe(state, question.temperature_k, roughness_m)],
        liquid,
        supply_pressure_pa: state.supply_pressure_pa,
        return_pressure_pa: question.return_pressure_pa,
        initial_flow_m3_s: state.velocity_m_s * bore_area_m2(),
        closure_time_s: question.stroke_time_s,
        duration_s: question.duration_s,
        free_gas_fraction_at_atmosphere: 0.0,
        wall_yield_pressure_pa: WALL_YIELD_PRESSURE_PA,
        mach_limit: MACH_LIMIT,
        samples: SAMPLES,
        tamper: Tamper::None,
    };
    let scale = question.stroke_time_s / STROKE_REFERENCE_S;
    let ext = Extensions {
        stiffening: Some(state.stiffening.clone()),
        valve_schedule: Some(ValveSchedule {
            step_s: STROKE_STEP_S * scale,
            open_fraction: STROKE.iter().map(|tau| valve_effective_opening(*tau, series_ratio, seat)).collect(),
        }),
        vapour_share_limit: Some(VAPOUR_SHARE_LIMIT),
        probe_nodes: vec![
            SECTIONS - S1_SECTIONS_FROM_VALVE,
            SECTIONS - S2_SECTIONS_FROM_VALVE,
            SECTIONS - S3_SECTIONS_FROM_VALVE,
        ],
        friction_table: true,
        void_fraction_limit: None,
    };
    (run, ext)
}

/// Computes the steady flow and the transient of the closure.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &LineQuestion) -> Result<LineHammer, LineReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &LineQuestion) -> Result<LineHammer, LineReject> {
    let state = admit(question)?;
    let (run, ext) = line_run(question, &state, ROUGHNESS_M, VALVE_SERIES_RATIO, VALVE_SEAT);
    solve_run(state, &run, &ext)
}

/// The liquid's stiffening of `crate::line_transient`, which this solver
/// returns with the line, held to the rule here.
impl NormalOrZero for crate::line_transient::Stiffening {
    fn normal_or_zero(&mut self) {
        self.pressure_pa.normal_or_zero();
        self.excess_compliance_per_pa.normal_or_zero();
    }
}

impl NormalOrZero for LineState {
    fn normal_or_zero(&mut self) {
        self.vapour_pressure_pa.normal_or_zero();
        self.density_kg_m3.normal_or_zero();
        self.sound_speed_m_s.normal_or_zero();
        self.viscosity_pa_s.normal_or_zero();
        self.steel_modulus_pa.normal_or_zero();
        self.wave_speed_m_s.normal_or_zero();
        self.section_wave_speed_m_s.normal_or_zero();
        self.velocity_m_s.normal_or_zero();
        self.reynolds_number.normal_or_zero();
        self.friction_factor.normal_or_zero();
        self.supply_pressure_pa.normal_or_zero();
        self.joukowsky_rise_pa.normal_or_zero();
        self.time_step_s.normal_or_zero();
        self.stiffening.normal_or_zero();
    }
}

impl NormalOrZero for LineHammer {
    fn normal_or_zero(&mut self) {
        self.state.normal_or_zero();
        self.steady_s1_pa.normal_or_zero();
        self.steady_s2_pa.normal_or_zero();
        self.steady_s3_pa.normal_or_zero();
        self.steady_valve_pa.normal_or_zero();
        self.time_step_s.normal_or_zero();
        self.s1_max_pa.normal_or_zero();
        self.s1_max_time_s.normal_or_zero();
        self.s2_max_pa.normal_or_zero();
        self.s2_max_time_s.normal_or_zero();
        self.s3_max_pa.normal_or_zero();
        self.s3_max_time_s.normal_or_zero();
        self.line_pressure_max_pa.normal_or_zero();
        self.line_pressure_min_pa.normal_or_zero();
        self.first_vapour_time_s.normal_or_zero();
        self.first_vapour_position_m.normal_or_zero();
        self.vapour_volume_max_m3.normal_or_zero();
        self.largest_line_mach.normal_or_zero();
        self.sample_times_s.normal_or_zero();
        self.s1_history_pa.normal_or_zero();
        self.s2_history_pa.normal_or_zero();
        self.s3_history_pa.normal_or_zero();
        self.valve_history_pa.normal_or_zero();
        self.valve_flow_history_m3_s.normal_or_zero();
        self.valve_vapour_history_m3.normal_or_zero();
        self.kinetic_energy_history_j.normal_or_zero();
        self.liquid_energy_history_j.normal_or_zero();
        self.wall_energy_history_j.normal_or_zero();
        self.vapour_energy_history_j.normal_or_zero();
        self.friction_loss_history_j.normal_or_zero();
        self.valve_work_history_j.normal_or_zero();
        self.cavity_loss_history_j.normal_or_zero();
        self.stored_energy_start_j.normal_or_zero();
        self.stored_energy_end_j.normal_or_zero();
        self.friction_loss_j.normal_or_zero();
        self.valve_work_j.normal_or_zero();
        self.cavity_loss_j.normal_or_zero();
        self.largest_energy_ledger_residual.normal_or_zero();
        self.largest_volume_ledger_residual.normal_or_zero();
        self.field.normal_or_zero();
    }
}

fn solve_run(state: LineState, run: &Run, ext: &Extensions) -> Result<LineHammer, LineReject> {
    let done = line::run_with(run, ext).map_err(rejected)?;
    let dt = done.time_step_s;
    let peak = |k: usize| {
        let series = &done.probe_pressures_pa[k];
        let (i, p) = series.iter().enumerate().fold((0usize, f64::NEG_INFINITY), |best, (i, p)| if *p > best.1 { (i, *p) } else { best });
        (p, i as f64 * dt)
    };
    let (s1_max, s1_t) = peak(0);
    let (s2_max, s2_t) = peak(1);
    let (s3_max, s3_t) = peak(2);
    let column = |pick: fn(&line::Ledger) -> f64| done.ledger_samples.iter().map(pick).collect::<Vec<f64>>();
    let every = |k: usize| -> Vec<f64> {
        let stride = (done.steps as usize + 1).div_ceil(SAMPLES).max(1);
        let series = &done.probe_pressures_pa[k];
        (0..series.len()).filter(|i| i % stride == 0 || *i == series.len() - 1).map(|i| series[i]).collect()
    };
    let mut field = Vec::with_capacity(3 * (done.steps as usize + 1) + 2 * (SECTIONS + 1));
    for k in 0..3 {
        field.extend_from_slice(&done.probe_pressures_pa[k]);
    }
    field.extend_from_slice(&done.final_pressures_pa);
    field.extend_from_slice(&done.final_velocities_m_s);
    let steady = &done.steady_pressures_pa;
    Ok(LineHammer {
        steady_s1_pa: steady[SECTIONS - S1_SECTIONS_FROM_VALVE],
        steady_s2_pa: steady[SECTIONS - S2_SECTIONS_FROM_VALVE],
        steady_s3_pa: steady[SECTIONS - S3_SECTIONS_FROM_VALVE],
        steady_valve_pa: steady[SECTIONS],
        time_step_s: dt,
        steps: done.steps,
        s1_max_pa: s1_max,
        s1_max_time_s: s1_t,
        s2_max_pa: s2_max,
        s2_max_time_s: s2_t,
        s3_max_pa: s3_max,
        s3_max_time_s: s3_t,
        line_pressure_max_pa: done.line_pressure_max_pa,
        line_pressure_min_pa: done.line_pressure_min_pa,
        first_vapour_time_s: done.first_vapour_time_s.clone(),
        first_vapour_position_m: done.first_vapour_position_m.clone(),
        vapour_volume_max_m3: done.vapour_volume_max_m3,
        largest_line_mach: done.largest_mach,
        sample_times_s: done.sample_times_s.clone(),
        s1_history_pa: every(0),
        s2_history_pa: every(1),
        s3_history_pa: every(2),
        valve_history_pa: done.valve_pressure_pa.clone(),
        valve_flow_history_m3_s: done.valve_flow_m3_s.clone(),
        valve_vapour_history_m3: done.valve_vapour_m3.clone(),
        kinetic_energy_history_j: column(|l| l.kinetic_j),
        liquid_energy_history_j: column(|l| l.liquid_compression_j + l.liquid_lump_j),
        wall_energy_history_j: column(|l| l.wall_strain_j),
        vapour_energy_history_j: column(|l| l.cavity_j),
        friction_loss_history_j: column(|l| l.friction_loss_j),
        valve_work_history_j: column(|l| l.valve_work_j),
        cavity_loss_history_j: column(|l| l.cavity_loss_j),
        stored_energy_start_j: done.ledger_start.stored_j(),
        stored_energy_end_j: done.ledger_end.stored_j(),
        friction_loss_j: done.ledger_end.friction_loss_j,
        valve_work_j: done.ledger_end.valve_work_j,
        cavity_loss_j: done.ledger_end.cavity_loss_j,
        largest_energy_ledger_residual: done.largest_energy_residual,
        largest_volume_ledger_residual: done.largest_volume_residual,
        field,
        state,
    })
}

#[cfg(test)]
pub(crate) fn tampered(question: &LineQuestion, tamper: Tamper) -> Result<LineHammer, LineReject> {
    let state = admit(question)?;
    let (mut run, ext) = line_run(question, &state, ROUGHNESS_M, VALVE_SERIES_RATIO, VALVE_SEAT);
    run.tamper = tamper;
    solve_run(state, &run, &ext)
}

#[cfg(test)]
mod forecast {
    //! The forecast posted before the six held-back tests' traces were
    //! opened: the questions of all nine tests as the protocol sets them
    //! (`evidence/research/hydraulics_robotics/dlr_ln2_line/validation_protocol.md`),
    //! and a fingerprint of every number of each answer, so that the solver
    //! served is the solver that made the forecast. The forecast was made at
    //! this crate's commit 279c776 (2026-10-14) with the platform's math on
    //! an Apple M4 Pro; since genesis_solvers 0.12.0 the solver computes with
    //! genesis_kernel::math, which moved its numbers by canonical math alone
    //! (at most 3.2e-10 of a worked answer's), and these are its
    //! fingerprints. The forecast's own were, in the order below:
    //! 0x252b8b595aa1b725, 0x7ce5bdb546dea793, 0xe90c69e387a71d09,
    //! 0x7c3a56c6e678c088, 0xee87530c67a5bfd6, 0x4ec4d56cbfe33e0d,
    //! 0x62fce5898b2f041e, 0x2f58e759e969d035, 0xe2f625d189355359.
    use super::*;

    /// (test, the Coriolis meter's density kg/m^3, its mass flow kg/s, the
    /// overview's P0 plus the atmosphere Pa, the fingerprint of the field)
    pub(crate) const QUESTIONS: [(&str, f64, f64, f64, u64); 9] = [
        ("20200806_5", 778.0304294117647, 1.3818981176470588, 649325.0, 0x049cfe9b8c2328d2),
        ("20200803_8", 778.7022647058824, 1.6458472352941178, 832325.0, 0xd9321ad32133e26a),
        ("20200806_15", 779.1922888888889, 2.160375666666667, 1352325.0, 0xb7ad17db99500e36),
        ("20200805_9", 777.8748470588235, 2.3573461764705885, 1684325.0, 0x07dfed51f0a359bd),
        ("20200803_15", 768.8745117647059, 2.5303167647058826, 1856325.0, 0xc95af11c95607bf2),
        ("20200805_10", 782.3472705882352, 2.6250385294117646, 2107325.0, 0x407e51950329f0c2),
        ("20200806_13", 782.1853555555556, 1.0642113333333334, 450325.0, 0x7d908ff3563cfa29),
        ("20200803_11", 779.3799333333332, 1.973860166666667, 1144325.0, 0x36e87aaa82b09b57),
        ("20200806_16", 783.2819470588234, 2.823437529411765, 2240325.0, 0x15d3c920e34284c3),
    ];
    /// The return pressure, the stroke time and the duration of every
    /// forecast question.
    pub(super) const RETURN_PA: f64 = 1.70e5;
    pub(super) const DURATION_S: f64 = 1.6;

    pub(crate) fn question(density: f64, mass_flow: f64, s1_pa: f64) -> LineQuestion {
        LineQuestion {
            temperature_k: n2::temperature_at_density_k(density, s1_pa).expect("a liquid of that density"),
            mass_flow_kg_s: mass_flow,
            s1_pressure_pa: s1_pa,
            return_pressure_pa: RETURN_PA,
            stroke_time_s: STROKE_REFERENCE_S,
            duration_s: DURATION_S,
        }
    }

    pub(super) fn fingerprint(h: &LineHammer) -> u64 {
        h.field.iter().fold(0u64, |x, f| x.rotate_left(7) ^ f.to_bits())
    }

    #[test]
    fn the_forecast_is_this_solver() {
        let mut differ = Vec::new();
        for (test, density, mass_flow, s1_pa, print) in QUESTIONS {
            let h = solve(&question(density, mass_flow, s1_pa)).unwrap();
            eprintln!("{test} {:#018x}", fingerprint(&h));
            if fingerprint(&h) != print {
                differ.push(test);
            }
        }
        assert!(differ.is_empty(), "{differ:?}");
    }
}

#[cfg(test)]
mod scored {
    //! The one scoring of the six held-back tests, 2026-10-15 UTC, by
    //! `evidence/research/hydraulics_robotics/dlr_ln2_line/analysis/score.py`
    //! (its output `scored.json`): each quantity's band, the forecast's
    //! computed values and the measured, in the order 20200806_5,
    //! 20200803_8, 20200806_15, 20200805_9, 20200803_15, 20200805_10. NaN
    //! where the fixed rule found no value: the dynamic sensor at S3 recorded
    //! no signal in 20200805_9 and 20200805_10.
    use super::forecast::{question, QUESTIONS};
    use super::*;

    type Row = (&'static str, f64, [f64; 6], [f64; 6]);
    const SCORED: [Row; 11] = [
        ("first_rise_s1_bar", 0.05, [39.70121150229621, 47.790493126902724, 63.94516443782595, 70.08514049307466, 74.21456216301223, 79.5668943086823], [37.87609905149914, 45.30808462981172, 61.03260220841823, 65.62300124123499, 69.6968292393675, 74.60829582309543]),
        ("first_rise_s2_bar", 0.05, [36.55852815013923, 43.661695492027825, 58.06515125578929, 63.53636981389391, 67.42214317691094, 71.72464434267022], [36.24801830702402, 44.39966665519662, 58.74951974060138, 63.14468521028233, 67.21059202391214, 71.8832891150497]),
        ("first_rise_s3_bar", 0.1, [22.634730289123507, 27.246896484326072, 36.3362486568719, 39.742633115996085, 42.54771340731611, 44.67061587611269], [23.24429688651765, 32.030694629601896, 41.629649121628404, 45.897352579123094, 49.00921296558204, 51.58724506742183]),
        ("front_speed_s1_s2_m_s", 0.03, [790.8228242927237, 793.8067510107978, 798.7797020405139, 798.0557738433444, 782.0620266686566, 808.9034362333895], [788.2738133933302, 796.7366678411618, 801.6954918823341, 798.4954309290638, 776.754434300136, 809.3896975721511]),
        ("front_speed_s2_s3_m_s", 0.03, [756.5313650483155, 757.3451152599886, 757.0264380443007, 753.6686713847486, 741.761893744626, 760.4239221959198], [774.4480986018198, 770.3648526314655, 756.3085508336177, f64::NAN, 739.0361701349749, f64::NAN]),
        ("plateau_s1_ms", 0.03, [21.33172561259169, 21.171512563600928, 20.894933746265924, 20.851410191978864, 21.240012585446568, 20.499399389842388], [23.034912568663216, 22.530819922277978, 21.81207819501018, 21.814321246260747, 22.470023980445223, 21.456349926602186]),
        ("second_surge_s1_ms", 0.15, [141.83155282431304, 122.6277547604825, 94.87496309281099, 84.9209994717878, 83.58418806143689, 76.97974681796754], [145.41338785651055, 129.80469201198105, 102.08269923076902, 92.41040736649424, 82.61363405178281, 75.93992915973702]),
        ("second_surge_rise_s1_bar", 0.2, [32.750742521849496, 39.45927436343843, 54.91690920014916, 62.966255945044736, 64.52749672647747, 69.9642732875458], [15.523191190699144, 28.371342640111717, 51.362462500218214, 58.364444890635006, 55.805753035067504, 57.052863683095424]),
        ("late_period_ms", 0.05, [47.6122725655392, 47.56215391965298, 47.22262240538358, 47.28775418845789, 48.34149500946788, 46.62786891291114], [74.7609776246622, 59.94650334401751, 54.37050842503053, 53.50452993583543, 54.027685674261114, 51.82248360303611]),
        ("late_decay_per_s", 0.3, [-1.6829626644310807, -0.6218395622982831, 0.5084138026412606, 0.5922569954894421, 0.648752121653448, 0.6445243798520471], [9.070696111115387, 4.907274192937267, 3.497028916197745, 3.323712664969147, 2.9594794209817747, 3.0002441534019244]),
        ("steady_drop_s3_s1_bar", 0.1, [1.125660301962304, 1.5874160949232232, 2.7152432489003253, 3.231709132717892, 3.757202985381941, 3.9784486460088573], [1.1028636274496337, 1.6565614912201525, 2.959829229313577, 3.3798999739135613, 4.140349987854396, 4.222953736673102]),
    ];

    /// The late fit's amplitude and the root mean square of what it leaves,
    /// bar, of the forecast's computed S1 traces (`forecast/forecast.json`,
    /// committed before the scoring) and of the measured ones (`scored.json`),
    /// in the order of the scored tests.
    const LATE_FIT_COMPUTED: [(f64, f64); 6] = [
        (1.5457925039846192, 5.421198710147837),
        (5.017231052261657, 4.995915491824018),
        (14.416645965519274, 2.3578761711908087),
        (18.37773173062532, 2.1583417598055905),
        (21.487418099202834, 1.9224111540882884),
        (22.63506847752476, 2.374407676018229),
    ];
    const LATE_FIT_MEASURED: [(f64, f64); 6] = [
        (0.31438769935081956, 0.01652170496590274),
        (1.0584556831733116, 0.01971844076942018),
        (3.281874508152142, 0.04951556047269732),
        (4.474304342559047, 0.05682722083781238),
        (5.992803067895986, 0.0784177337582236),
        (5.425173300856286, 0.06969807703637527),
    ];
    /// A late fit whose leftover is half its amplitude or more has not fitted
    /// one damped cosine to the trace: its period and decay are not the
    /// trace's. The rule is read after the scoring, so it moves no mark.
    pub(super) const LATE_FIT_FAILS_AT_RMS_OVER_AMPLITUDE: f64 = 0.5;

    /// The late fit failed on the two lowest-pressure tests' computed traces
    /// (20200806_5 and 20200803_8, leftover 3.5 and 1.0 times the amplitude:
    /// their computed late oscillation is not one damped cosine, and the
    /// decay rates fitted there, -1.68 and -0.62 per s, are not rates), and
    /// held on every other trace, computed and measured (at most 0.16). On
    /// the four it held, the computed decay is 78.1 to 85.5 percent under
    /// the measured and the period 10.0 to 13.1 percent short: the two marks
    /// stay missed, 0 of 4 inside their bands.
    #[test]
    fn the_late_fit_fails_on_two_computed_traces_and_the_rest_miss() {
        let fails = |(amplitude, rms): (f64, f64)| rms >= LATE_FIT_FAILS_AT_RMS_OVER_AMPLITUDE * amplitude;
        let failed: Vec<usize> = (0..6).filter(|&k| fails(LATE_FIT_COMPUTED[k])).collect();
        assert_eq!(failed, [0, 1]);
        assert!(LATE_FIT_COMPUTED[2..].iter().chain(&LATE_FIT_MEASURED).all(|(a, r)| r / a <= 0.17));
        let row = |name: &str| SCORED.iter().find(|r| r.0 == name).unwrap();
        let (_, decay_band, decay_c, decay_m) = row("late_decay_per_s");
        let (_, period_band, period_c, period_m) = row("late_period_ms");
        let under: Vec<f64> = (2..6).map(|k| 1.0 - decay_c[k] / decay_m[k]).collect();
        let short: Vec<f64> = (2..6).map(|k| 1.0 - period_c[k] / period_m[k]).collect();
        let range = |v: &[f64]| (v.iter().copied().fold(f64::INFINITY, f64::min), v.iter().copied().fold(f64::NEG_INFINITY, f64::max));
        let ((u0, u1), (s0, s1)) = (range(&under), range(&short));
        assert!((u0 - 0.781).abs() < 5e-4 && (u1 - 0.855).abs() < 5e-4, "{under:?}");
        assert!((s0 - 0.100).abs() < 5e-4 && (s1 - 0.131).abs() < 5e-4, "{short:?}");
        assert!(under.iter().all(|u| *u > *decay_band) && short.iter().all(|s| *s > *period_band));
    }

    /// Each mark met or missed by its rule, five of six inside its band:
    /// met at S2's first rise, the front from S1 to S2, the second surge's
    /// time and the steady drop; missed at the rest. The first rises at S1
    /// and S2 are recomputed here from the solver's own traces by the rule
    /// of `score_rules.py`, so the table's computed numbers are this solver's.
    #[test]
    fn dlr_scored_once() {
        let met: Vec<&str> = SCORED
            .iter()
            .filter(|(_, band, computed, measured)| computed.iter().zip(measured).filter(|(c, m)| (*c / *m - 1.0).abs() <= *band).count() >= 5)
            .map(|row| row.0)
            .collect();
        assert_eq!(met, ["first_rise_s2_bar", "front_speed_s1_s2_m_s", "second_surge_s1_ms", "steady_drop_s3_s1_bar"]);
        let atm_bar = 1.01325;
        for (k, (test, density, mass_flow, s1_pa, _)) in QUESTIONS[..6].iter().enumerate() {
            let h = solve(&question(*density, *mass_flow, *s1_pa)).unwrap();
            let dt_ms = h.time_step_s * 1000.0;
            let bar = |series: &[f64]| series.iter().map(|p| p / 1.0e5 - atm_bar).collect::<Vec<f64>>();
            let (s1, s2) = (bar(h.station_every_step_pa(0)), bar(h.station_every_step_pa(1)));
            let first = (40.0 / dt_ms).floor() as usize;
            let rise1 = s1[1..=first].iter().copied().fold(f64::NEG_INFINITY, f64::max) - s1[0];
            let half = s1[0] + 0.5 * rise1;
            let up = s1.iter().position(|p| *p >= half).unwrap();
            let down = up + s1[up..].iter().position(|p| *p <= half).unwrap();
            let rise2 = s2[1..down].iter().copied().fold(f64::NEG_INFINITY, f64::max) - s2[0];
            assert!((rise1 / SCORED[0].2[k] - 1.0).abs() < 1e-9, "{test} {rise1}");
            assert!((rise2 / SCORED[1].2[k] - 1.0).abs() < 1e-9, "{test} {rise2}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(temperature_k: f64, mass_flow: f64, s1_bar_abs: f64, duration: f64) -> LineQuestion {
        LineQuestion {
            temperature_k,
            mass_flow_kg_s: mass_flow,
            s1_pressure_pa: s1_bar_abs * 1.0e5,
            return_pressure_pa: 1.70e5,
            stroke_time_s: STROKE_REFERENCE_S,
            duration_s: duration,
        }
    }

    #[test]
    fn the_specimen_numbers() {
        // Tresca on Lame's stresses with the vacuum outside, 200 MPa.
        let (ro, ri) = (0.5 * OUTER_DIAMETER_M, 0.5 * BORE_M);
        let tresca = STEEL_PROOF_STRENGTH_PA * (ro * ro - ri * ri) / (2.0 * ro * ro);
        assert!((tresca / WALL_YIELD_PRESSURE_PA - 1.0).abs() < 1e-15, "{tresca}");
        // NIST's fit: about 199.5 GPa at room temperature and 214 GPa at 85 K.
        assert!((steel_modulus_pa(293.0) / 199.5e9 - 1.0).abs() < 0.002, "{}", steel_modulus_pa(293.0));
        assert!((steel_modulus_pa(85.0) / 214.2e9 - 1.0).abs() < 0.002, "{}", steel_modulus_pa(85.0));
        // The coldest liquid: the melting line meets the yield pressure at 68.56138 K.
        assert!(n2::melting_pressure_pa(COLDEST_K) >= WALL_YIELD_PRESSURE_PA);
        assert!(n2::melting_pressure_pa(68.56137) < WALL_YIELD_PRESSURE_PA && n2::melting_pressure_pa(68.56139) > WALL_YIELD_PRESSURE_PA);
        // The warmest: the saturated vapour a tenth as dense as the liquid.
        assert!((n2::saturated_vapour_over_liquid(WARMEST_K).unwrap() - VAPOUR_DENSITY_SHARE_MAX).abs() < 1e-4);
        // The stroke passes 0.9 and 0.1 of open STROKE_REFERENCE_S apart.
        let cross = |level: f64| {
            let k = STROKE.iter().position(|v| *v <= level).unwrap();
            STROKE_STEP_S * ((k - 1) as f64 + (STROKE[k - 1] - level) / (STROKE[k - 1] - STROKE[k]))
        };
        // The table keeps four digits of the mean trace; its own crossings are 0.2 us short.
        assert!((cross(0.1) - cross(0.9) - STROKE_REFERENCE_S).abs() < 1e-6, "{}", cross(0.1) - cross(0.9));
        assert!(STROKE.windows(2).all(|w| w[1] <= w[0]) && STROKE[0] == 1.0 && STROKE[STROKE.len() - 1] == 0.0);
        // The stations fall within 2.6 mm of their nodes.
        for (share, sections) in [(S1_FROM_VALVE, S1_SECTIONS_FROM_VALVE), (S2_FROM_VALVE, S2_SECTIONS_FROM_VALVE), (S3_FROM_VALVE, S3_SECTIONS_FROM_VALVE)] {
            assert!((share * LINE_LENGTH_M - sections as f64 * section_m()).abs() < 2.6e-3);
        }
        // The valve: closed at its seat, fully open at one, rising between.
        assert_eq!(valve_effective_opening(VALVE_SEAT, VALVE_SERIES_RATIO, VALVE_SEAT), 0.0);
        assert!((valve_effective_opening(1.0, VALVE_SERIES_RATIO, VALVE_SEAT) - 1.0).abs() < 1e-15);
    }

    /// Liquid nitrogen at 83.7 K and 11.4 bar, the conditions of the traced
    /// test 20200803_11: the wave speed is the liquid's and the wall's, and
    /// a surge of 50 bar runs on a stiffer liquid.
    #[test]
    fn the_state_and_the_stiffening() {
        let state = admit(&question(83.7, 1.9739, 11.44325, 0.05)).unwrap();
        assert!((state.density_kg_m3 - 779.38).abs() < 0.05, "{}", state.density_kg_m3);
        assert!((state.sound_speed_m_s - 795.6).abs() < 0.2, "{}", state.sound_speed_m_s);
        assert!((state.wave_speed_m_s - 783.75).abs() < 0.2, "{}", state.wave_speed_m_s);
        // The sections carry the stiffest liquid, at the yield pressure.
        assert!(state.section_wave_speed_m_s > 950.0 && state.section_wave_speed_m_s < 955.0, "{}", state.section_wave_speed_m_s);
        let stiff = &state.stiffening;
        assert_eq!(stiff.pressure_pa[0], state.vapour_pressure_pa);
        assert_eq!(*stiff.pressure_pa.last().unwrap(), WALL_YIELD_PRESSURE_PA);
        assert_eq!(*stiff.excess_compliance_per_pa.last().unwrap(), 0.0);
        assert!(stiff.excess_compliance_per_pa.windows(2).all(|w| w[1] < w[0]));
        // Joukowsky's m a / A with the wave speed at S1.
        assert!((state.joukowsky_rise_pa / (1.9739 * state.wave_speed_m_s / bore_area_m2()) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn the_ledgers_close_and_the_steady_state_is_the_asked_one() {
        let q = question(83.7, 1.9739, 11.44325, 0.05);
        let h = solve(&q).unwrap();
        assert!(h.largest_energy_ledger_residual < 1e-12, "{}", h.largest_energy_ledger_residual);
        assert!(h.largest_volume_ledger_residual < 1e-12, "{}", h.largest_volume_ledger_residual);
        // The supply is set by Churchill's formula and the line runs on its
        // table, within 2e-6 of it: S1's steady pressure is the asked one to a
        // hundredth of a pascal.
        assert!((h.steady_s1_pa - q.s1_pressure_pa).abs() < 0.01, "{}", h.steady_s1_pa);
        assert!(h.steady_s3_pa > h.steady_s2_pa && h.steady_s2_pa > h.steady_s1_pa && h.steady_s1_pa > h.steady_valve_pa);
        // The first surge at S1: Joukowsky's rise with the stiffening and the
        // line's packing, a few percent over m a / A.
        let rise = h.s1_max_pa - h.steady_s1_pa;
        assert!(rise > h.state.joukowsky_rise_pa && rise < 1.08 * h.state.joukowsky_rise_pa, "{rise} {}", h.state.joukowsky_rise_pa);
        // Energy: friction and the valve take, and the lumps' backward step a little.
        assert!(h.friction_loss_j > 0.0 && h.cavity_loss_j >= 0.0);
        assert!(h.cavity_loss_j < 0.05 * h.stored_energy_start_j, "{} {}", h.cavity_loss_j, h.stored_energy_start_j);
    }

    /// A small surge runs at the liquid's wave speed at the steady state; a
    /// large one, on a liquid it stiffens, runs faster.
    #[test]
    fn a_larger_surge_runs_faster() {
        let speed = |mass_flow: f64| {
            let h = solve(&question(83.7, mass_flow, 21.0, 0.03)).unwrap();
            let (s1, s2) = (h.station_every_step_pa(0), h.station_every_step_pa(1));
            let rise = h.s1_max_pa - h.steady_s1_pa;
            let at = |series: &[f64], base: f64| {
                let level = base + 0.25 * rise;
                let k = series.iter().position(|p| *p >= level).unwrap();
                (k as f64 - 1.0 + (level - series[k - 1]) / (series[k] - series[k - 1])) * h.time_step_s
            };
            let gap = at(s2, h.steady_s2_pa) - at(s1, h.steady_s1_pa);
            ((S2_SECTIONS_FROM_VALVE - S1_SECTIONS_FROM_VALVE) as f64 * section_m() / gap, h.state.wave_speed_m_s)
        };
        let (small, a) = speed(0.05);
        let (large, _) = speed(2.8);
        assert!((small / a - 1.0).abs() < 0.005, "{small} {a}");
        assert!(large > 1.015 * small, "{large} {small}");
    }

    #[test]
    fn the_question_is_refused_by_name_at_its_limits() {
        let base = question(83.7, 1.9739, 11.44325, 0.05);
        let with = |change: fn(&mut LineQuestion)| {
            let mut q = base;
            change(&mut q);
            admit(&q).unwrap_err()
        };
        assert!(matches!(with(|q| q.temperature_k = f64::NAN), LineReject::InputNotFinite { field: "temperature_k" }));
        assert!(matches!(with(|q| q.temperature_k = 0.0), LineReject::TemperatureNotPositive { .. }));
        assert!(matches!(with(|q| q.temperature_k = -1.0), LineReject::TemperatureNotPositive { .. }));
        assert!(matches!(with(|q| q.temperature_k = 68.56), LineReject::LiquidFreezesBelowWallYield { melting_pressure_pa: Some(_), .. }));
        assert!(matches!(with(|q| q.temperature_k = COLDEST_K.next_down()), LineReject::LiquidFreezesBelowWallYield { melting_pressure_pa: Some(_), .. }));
        assert!(matches!(with(|q| q.temperature_k = 60.0), LineReject::LiquidFreezesBelowWallYield { melting_pressure_pa: None, .. }));
        assert!(matches!(with(|q| q.temperature_k = 109.91), LineReject::VapourNotLightAgainstLiquid { .. }));
        assert!(matches!(with(|q| q.temperature_k = WARMEST_K.next_up()), LineReject::VapourNotLightAgainstLiquid { .. }));
        assert!(matches!(with(|q| q.s1_pressure_pa = 2.0e5), LineReject::S1PressureNotAboveVapour { .. }));
        // With a flow the supply is S1's pressure and the friction to the tank.
        assert!(matches!(with(|q| q.s1_pressure_pa = WALL_YIELD_PRESSURE_PA), LineReject::SupplyAboveWallYield { supply_pressure_pa } if supply_pressure_pa > WALL_YIELD_PRESSURE_PA));
        assert!(matches!(with(|q| q.s1_pressure_pa = WALL_YIELD_PRESSURE_PA - 1.0e4), LineReject::SupplyAboveWallYield { supply_pressure_pa } if supply_pressure_pa > WALL_YIELD_PRESSURE_PA));
        assert!(matches!(with(|q| { q.s1_pressure_pa = WALL_YIELD_PRESSURE_PA.next_up(); q.mass_flow_kg_s = 0.0 }), LineReject::SupplyAboveWallYield { .. }));
        assert!(matches!(with(|q| q.mass_flow_kg_s = -0.1), LineReject::MassFlowNegative { .. }));
        assert!(matches!(with(|q| q.return_pressure_pa = 0.0), LineReject::ReturnBelowTriplePoint { .. }));
        assert!(matches!(with(|q| q.return_pressure_pa = n2::TRIPLE_POINT_PA.next_down()), LineReject::ReturnBelowTriplePoint { .. }));
        assert!(matches!(with(|q| q.duration_s = 0.0), LineReject::DurationOutOfRange { .. }));
        assert!(matches!(with(|q| q.duration_s = DURATION_MAX_S.next_up()), LineReject::DurationOutOfRange { .. }));
        assert!(matches!(with(|q| q.stroke_time_s = 0.06), LineReject::StrokeTimeOutOfRange { .. }));
        assert!(matches!(with(|q| q.stroke_time_s = -0.001), LineReject::StrokeTimeOutOfRange { .. }));
        assert!(matches!(with(|q| q.stroke_time_s = 0.0005), LineReject::StrokeFasterThanGrid { .. }));
        assert!(matches!(with(|q| q.return_pressure_pa = 11.4e5), LineReject::SupplyCannotDriveFlow { .. }));
        assert!(matches!(with(|q| { q.s1_pressure_pa = 2.3e5; q.mass_flow_kg_s = 6.0 }), LineReject::SteadyBelowVapour { .. }));
        assert!(matches!(with(|q| q.mass_flow_kg_s = 18.0), LineReject::InitialFlowMachAboveAcoustic { .. }));
        // Inside every limit the question is admitted, at both ends of the
        // liquid, at the longest run, with the tank at the triple point and,
        // at rest, with S1 at the yield pressure itself.
        admit(&base).unwrap();
        admit(&question(COLDEST_K, 1.0, 20.0, 0.05)).unwrap();
        admit(&question(WARMEST_K, 1.0, 20.0, 0.05)).unwrap();
        admit(&LineQuestion { duration_s: DURATION_MAX_S, ..base }).unwrap();
        admit(&LineQuestion { return_pressure_pa: n2::TRIPLE_POINT_PA, ..base }).unwrap();
        admit(&LineQuestion { s1_pressure_pa: WALL_YIELD_PRESSURE_PA, mass_flow_kg_s: 0.0, ..base }).unwrap();
        // The shortest stroke published is the coldest liquid's, the least of all.
        let lightest = LineQuestion { temperature_k: COLDEST_K, mass_flow_kg_s: 0.0, s1_pressure_pa: n2::vapour_pressure_pa(COLDEST_K).unwrap().next_up(), ..base };
        admit(&LineQuestion { stroke_time_s: STROKE_TIME_MIN_S, ..lightest }).unwrap();
        let below = admit(&LineQuestion { stroke_time_s: STROKE_TIME_MIN_S.next_down(), ..lightest });
        assert!(matches!(below, Err(LineReject::StrokeFasterThanGrid { .. })), "{below:?}");
    }

    /// No sentence prints a number that is not one, and each says the limit
    /// its temperature reached: absolute zero, the triple point, the melting
    /// line, the table's saturated densities, the table's end, the critical
    /// point.
    #[test]
    fn the_temperature_refusals_say_their_own_limit() {
        let base = question(83.7, 1.9739, 11.44325, 0.05);
        let said = |t: f64| admit(&LineQuestion { temperature_k: t, ..base }).unwrap_err().to_string();
        for (t, words) in [
            (0.0, "not above absolute zero"),
            (-1.0, "not above absolute zero"),
            (10.0, "below nitrogen's triple point, 63.151 K, nitrogen is solid at every pressure"),
            (63.0, "below nitrogen's triple point"),
            (64.0, "liquid nitrogen freezes at 38"),
            (WARMEST_K.next_up(), "the saturated vapour is 0.10000000000000005 as dense as the liquid, above 0.1"),
            (110.5, "warmer than 110 K, the table's warmest isotherm, the saturated vapour is more than 0.10069739675020195"),
            (126.192, "at or above nitrogen's critical temperature, 126.192 K, there is no liquid"),
            (1.0e300, "critical temperature"),
        ] {
            let sentence = said(t);
            assert!(sentence.contains(words), "{t}: {sentence}");
            assert!(!sentence.contains("NaN") && !sentence.contains("inf") && !sentence.contains(" -1") || t < 0.0, "{t}: {sentence}");
            if t < n2::TRIPLE_POINT_K {
                assert!(!sentence.contains(" Pa"), "{t}: {sentence}");
            }
        }
    }

    /// The warmest liquid is the table's own edge to the last bit: its
    /// saturated vapour at most a tenth as dense as its liquid, one number
    /// above more; and the share rises across the edge.
    #[test]
    fn the_warmest_liquid_is_where_the_vapour_passes_a_tenth() {
        let share = |t: f64| n2::saturated_vapour_over_liquid(t).unwrap();
        assert!(share(WARMEST_K) <= VAPOUR_DENSITY_SHARE_MAX, "{}", share(WARMEST_K));
        assert!(share(WARMEST_K.next_up()) > VAPOUR_DENSITY_SHARE_MAX, "{}", share(WARMEST_K.next_up()));
        let mut t = 109.9;
        let mut last = share(t);
        while t < 110.0 {
            t += 1.0e-4;
            let now = share(t.min(110.0));
            assert!(now >= last && (now <= VAPOUR_DENSITY_SHARE_MAX) == (t <= WARMEST_K), "{t} {now}");
            last = now;
        }
    }

    /// The shortest stroke published is ten steps of the grid at the coldest
    /// liquid just above its vapour pressure, where the liquid is lightest
    /// and stiffest and the step shortest; no other liquid's grid is finer.
    #[test]
    fn the_shortest_stroke_is_the_coldest_liquids() {
        let steps_at = |t: f64, s1: f64| {
            let q = LineQuestion { temperature_k: t, mass_flow_kg_s: 0.0, s1_pressure_pa: s1, return_pressure_pa: 1.7e5, stroke_time_s: 0.0072555, duration_s: 0.05 };
            STROKE_STEPS_MIN * admit(&q).unwrap().time_step_s
        };
        let lowest = |t: f64| n2::vapour_pressure_pa(t).unwrap().next_up();
        eprintln!("shortest stroke {:e}", steps_at(COLDEST_K, lowest(COLDEST_K)));
        assert_eq!(steps_at(COLDEST_K, lowest(COLDEST_K)), STROKE_TIME_MIN_S);
        let mut t = COLDEST_K;
        while t < WARMEST_K {
            for s1 in [lowest(t), 2.0e6, 1.0e7, WALL_YIELD_PRESSURE_PA] {
                if s1 > n2::vapour_pressure_pa(t).unwrap() {
                    assert!(steps_at(t, s1) >= STROKE_TIME_MIN_S, "{t} {s1}");
                }
            }
            t += 0.25;
        }
        assert!((steps_at(83.7, 1.14e6) - 1.049e-3).abs() < 5e-7 && (steps_at(WARMEST_K, 2.0e6) - 1.255e-3).abs() < 5e-7, "{} {}", steps_at(83.7, 1.14e6), steps_at(WARMEST_K, 2.0e6));
    }

    /// A surge that would pass the tube's first yield stops at that time and place.
    #[test]
    fn a_rise_to_the_wall_yield_stops_by_name() {
        let q = question(83.7, 2.6, 225.0, 0.03);
        admit(&q).unwrap();
        match solve(&q) {
            Err(LineReject::WallYield { position_m, pressure_pa, .. }) => {
                assert!(pressure_pa >= WALL_YIELD_PRESSURE_PA, "{pressure_pa}");
                assert!(position_m > 0.5 * LINE_LENGTH_M, "{position_m}");
            }
            other => panic!("{other:?}"),
        }
    }

    /// The vapour's room. On this line friction and the tank bring the
    /// column back before its vapour reaches a tenth of the line: the most
    /// any question tried reached is 5.6 percent (1.48e-4 m^3 at 100 K and
    /// 8 bar, 2 kg/s). The same run with the room cut to 2 percent stops by
    /// name when the vapour passes it.
    #[test]
    fn vapour_past_its_room_stops_by_name() {
        let q = question(83.7, 2.0, 2.25, 0.3);
        let h = solve(&q).unwrap();
        let line_volume = bore_area_m2() * LINE_LENGTH_M;
        assert!(h.vapour_volume_max_m3 > 0.03 * line_volume && h.vapour_volume_max_m3 < VAPOUR_SHARE_LIMIT * line_volume, "{}", h.vapour_volume_max_m3);
        let state = admit(&q).unwrap();
        let (run, mut ext) = line_run(&q, &state, ROUGHNESS_M, VALVE_SERIES_RATIO, VALVE_SEAT);
        ext.vapour_share_limit = Some(0.02);
        match solve_run(state, &run, &ext) {
            Err(LineReject::VapourOverLineShare { vapour_over_limit, .. }) => assert!(vapour_over_limit > 1.0),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_falsified_step_is_stopped_by_name() {
        let q = question(83.7, 1.9739, 11.44325, 0.03);
        assert!(tampered(&q, Tamper::None).is_ok());
        assert!(matches!(tampered(&q, Tamper::ValveWorkDropped), Err(LineReject::EnergyLedgerBroken { .. })));
        assert!(matches!(tampered(&q, Tamper::WaveGain), Err(LineReject::EnergyLedgerBroken { .. } | LineReject::VolumeLedgerBroken { .. })));
        assert!(matches!(tampered(&q, Tamper::FrictionPushes), Err(LineReject::FrictionGaveEnergy { .. })));
        assert!(matches!(tampered(&q, Tamper::LumpForward), Err(LineReject::CavityGaveEnergy { .. })));
        assert!(matches!(tampered(&q, Tamper::LumpEnergyDropped), Err(LineReject::EnergyLedgerBroken { .. })));
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let q = question(86.0, 2.2, 13.5, 0.05);
        assert_eq!(solve(&q).unwrap(), solve(&q).unwrap());
    }

    /// The cavity at the valve forms at the liquid's vapour pressure and the
    /// column comes back: the second surge.
    #[test]
    fn the_line_cavitates_at_the_vapour_pressure_and_the_columns_rejoin() {
        let h = solve(&question(83.7, 1.9739, 11.44325, 0.15)).unwrap();
        assert!(!h.first_vapour_time_s.is_empty());
        assert!((h.line_pressure_min_pa / h.state.vapour_pressure_pa - 1.0).abs() < 1e-14, "{}", h.line_pressure_min_pa);
        assert!(h.vapour_volume_max_m3 > 0.0);
        let s1 = h.station_every_step_pa(0);
        let after = (0.06 / h.time_step_s) as usize;
        let second = s1[after..].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(second > h.steady_s1_pa + 0.5 * (h.s1_max_pa - h.steady_s1_pa), "{second}");
        assert!(h.largest_energy_ledger_residual < 1e-12 && h.largest_volume_ledger_residual < 1e-12);
    }
}

#[cfg(test)]
mod edges {
    //! The line at rest and creeping near its two pressure edges, the
    //! vapour pressure and the yield; the falsified steps attacked there too.
    use super::*;

    fn at(temperature_k: f64, mass_flow: f64, s1_pa: f64, duration: f64) -> LineQuestion {
        LineQuestion { temperature_k, mass_flow_kg_s: mass_flow, s1_pressure_pa: s1_pa, return_pressure_pa: 1.70e5, stroke_time_s: STROKE_REFERENCE_S, duration_s: duration }
    }

    fn vapour(t: f64) -> f64 {
        n2::vapour_pressure_pa(t).unwrap()
    }

    /// A line at rest stays at rest to the bit, from one number above the
    /// vapour pressure to the yield pressure itself: every node is its own
    /// step's root there, so nothing moves, nothing is lost and both ledgers
    /// read zero.
    #[test]
    fn a_line_at_rest_stays_at_rest_to_the_bit() {
        let pv = vapour(83.7);
        for s1 in [pv.next_up(), pv + 1.0, 11.44325e5, WALL_YIELD_PRESSURE_PA * (1.0 - 1.0e-9), WALL_YIELD_PRESSURE_PA] {
            let h = solve(&at(83.7, 0.0, s1, 2.0)).unwrap_or_else(|e| panic!("{s1}: {e}"));
            assert!(h.field.iter().take(3 * (h.steps as usize + 1)).all(|p| *p == s1), "{s1}");
            assert_eq!((h.line_pressure_max_pa, h.line_pressure_min_pa), (s1, s1));
            assert_eq!((h.cavity_loss_j, h.friction_loss_j, h.valve_work_j, h.stored_energy_end_j), (0.0, 0.0, 0.0, 0.0));
            assert_eq!((h.largest_energy_ledger_residual, h.largest_volume_ledger_residual), (0.0, 0.0));
        }
        // With its valve shut by a flow of zero, the return pressure does not
        // reach the line: one above the line's pressure changes nothing.
        let shut = solve(&LineQuestion { return_pressure_pa: 2.0e7, ..at(83.7, 0.0, 5.0e5, 0.05) }).unwrap();
        assert_eq!(shut.field, solve(&at(83.7, 0.0, 5.0e5, 0.05)).unwrap().field);
    }

    /// Creeping flows, down to the smallest normal number, and lines a
    /// nanopascal over their vapour pressure, run for the longest run: the
    /// friction is Poiseuille's below a Reynolds number of 1.6e-25, and both
    /// ledgers close under their limit, judged on scales at least the
    /// rounding of the terms they add.
    #[test]
    fn creeping_flow_near_saturation_holds_its_ledgers() {
        let pv = vapour(83.7);
        for (m, s1, duration) in [
            (1.0e-30, 11.44325e5, 0.05),
            (f64::MIN_POSITIVE, 11.44325e5, 0.05),
            (1.0e-30, pv + 1.0, 0.05),
            (1.0e-3, pv + 1.0, 2.0),
            (1.0e-5, pv + 3.0, 2.0),
            (1.0e-4, pv + 10.0, 0.05),
            (1.0e-30, pv + 1.0e-9, DURATION_MAX_S),
            (1.0e-12, pv + 1.0e-3, DURATION_MAX_S),
        ] {
            let h = solve(&at(83.7, m, s1, duration)).unwrap_or_else(|e| panic!("{m} {s1} {duration}: {e}"));
            assert!(h.largest_energy_ledger_residual < 0.5 * line::ENERGY_RESIDUAL_LIMIT, "{m} {s1}: {}", h.largest_energy_ledger_residual);
            assert!(h.largest_volume_ledger_residual < 0.5 * line::VOLUME_RESIDUAL_LIMIT, "{m} {s1}: {}", h.largest_volume_ledger_residual);
            assert!(h.state.friction_factor.is_finite() && h.friction_loss_j.is_finite(), "{m}");
        }
        // At the warmest and the coldest liquid too, the coldest into a tank at
        // the triple point, below its vapour pressure of 0.31 bar.
        for t in [COLDEST_K, WARMEST_K] {
            let q = LineQuestion { return_pressure_pa: n2::TRIPLE_POINT_PA, ..at(t, 1.0e-5, vapour(t) + 3.0, 0.05) };
            solve(&q).unwrap_or_else(|e| panic!("{t}: {e}"));
        }
    }

    /// Where the floor of the ledgers' scales is what judges them, a creeping
    /// flow a pascal over its vapour pressure, every falsified step is still
    /// stopped by name.
    #[test]
    fn a_falsified_step_is_stopped_by_name_near_saturation() {
        let q = at(83.7, 1.0e-3, vapour(83.7) + 1.0, 0.05);
        assert!(tampered(&q, Tamper::None).is_ok());
        assert!(matches!(tampered(&q, Tamper::ValveWorkDropped), Err(LineReject::EnergyLedgerBroken { .. })), "{:?}", tampered(&q, Tamper::ValveWorkDropped).err());
        assert!(matches!(tampered(&q, Tamper::WaveGain), Err(LineReject::EnergyLedgerBroken { .. } | LineReject::VolumeLedgerBroken { .. })), "{:?}", tampered(&q, Tamper::WaveGain).err());
        assert!(matches!(tampered(&q, Tamper::FrictionPushes), Err(LineReject::FrictionGaveEnergy { .. })));
        assert!(matches!(tampered(&q, Tamper::LumpForward), Err(LineReject::CavityGaveEnergy { .. })), "{:?}", tampered(&q, Tamper::LumpForward).err());
        assert!(matches!(tampered(&q, Tamper::LumpEnergyDropped), Err(LineReject::EnergyLedgerBroken { .. })), "{:?}", tampered(&q, Tamper::LumpEnergyDropped).err());
    }

    /// A surge a little past the yield pressure, from a line just under it,
    /// stops there, and the pressure its sentence names is above the yield.
    #[test]
    fn a_surge_just_past_the_yield_names_a_pressure_above_it() {
        let q = at(83.7, 1.0e-6, WALL_YIELD_PRESSURE_PA * (1.0 - 1.0e-9), 0.03);
        match solve(&q) {
            Err(reject @ LineReject::WallYield { pressure_pa, .. }) => {
                assert!(pressure_pa > WALL_YIELD_PRESSURE_PA, "{pressure_pa}");
                assert!(reject.to_string().contains(&format!("reached {pressure_pa} Pa")), "{reject}");
            }
            other => panic!("{other:?}"),
        }
    }
}
