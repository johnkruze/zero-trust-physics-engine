//! A chamber of air charged or vented through one pneumatic valve: the
//! company's own specimen, a valve of sonic conductance 1.0e-8 m^3/(s Pa)
//! (1.0 dm^3/(s bar)) and critical pressure ratio 0.30 between a reservoir
//! of air held at one pressure and a rigid 100 cm^3 chamber, a cylinder as
//! long as it is wide (50.31 mm) whose wall stays at 293.15 K.
//!
//! The valve. The mass flow is the form of ISO 6358-1:2013, from the
//! upstream side 1 to the downstream side 2,
//!
//! ```text
//! q_m = C rho_0 p_1 sqrt(T_0 / T_1)                              p_2/p_1 <= b   choked
//! q_m = C rho_0 p_1 sqrt(T_0 / T_1) [1 - ((p_2/p_1 - b)/(1 - b))^2]^m   b < p_2/p_1 <= 1
//! ```
//!
//! with `rho_0 = 1.185 kg/m^3` and `T_0 = 293.15 K` the standard's
//! reference air, `m = 0.5`, the subsonic index the standard takes when none
//! is measured, and the same `C` and `b` whichever way the air goes. The
//! upstream side is whichever side is at the higher pressure; choking is a
//! regime the answer reports, sample by sample, not a limit.
//!
//! The chamber. Air as an ideal gas (R = 287.05 J/(kg K), gamma 1.4) of one
//! pressure and one temperature, exchanging heat with the wall by a film
//! coefficient of 30 W/(m^2 K) over its 0.011 93 m^2 of wall:
//!
//! ```text
//! dm/dt = q_in - q_out
//! dU/dt = q_in c_p T_r - q_out c_p T - h A (T - T_w),    U = m c_v T,  p V = m R T
//! ```
//!
//! Air comes in at the reservoir's 293.15 K and leaves at the chamber's own
//! temperature: an adiabatic valve keeps the stagnation enthalpy.
//!
//! The step. Classical fourth-order Runge-Kutta at 20 us over the chamber's
//! mass and energy, carried as their changes from the start, with the mass
//! and the enthalpy through the valve and the heat to the wall carried as
//! three more states, each summed with its rounding (Neumaier's sum), so that
//! the ledgers
//!
//! ```text
//! (m - m_0) - M_valve = 0
//! (U - U_0) - H_valve + Q_wall = 0
//! ```
//!
//! close to a few parts in 1e16 of what moved at every step and substep, over
//! the longest run, and break by name when a step moves what it does not
//! book. The pressure is `p_0 + (gamma - 1)(U - U_0)/V`, exact for an ideal
//! gas in a rigid chamber. Near equal pressures the subsonic form's flow
//! falls as the square root of the difference, its slope without bound, so
//! the chamber reaches the reservoir's pressure in a finite time and its own
//! rate, how fast its pressure answers a change of itself through the valve
//! and the wall, outruns any fixed step: where that rate times the step
//! passes 0.2 the step is cut into substeps of Runge-Kutta, and where it
//! passes 1 over the time left in the step, or past sixteen substeps, the
//! rest of the step is one backward-Euler step, the valve's flow and the
//! wall's heat taken at its end, which cannot carry the chamber past the
//! reservoir's pressure. The last step is what is left of 20 us, so that the
//! run ends at the asked time. Every step's pressure lies between the two
//! pressures given, to the rounding of the pressure moved.
//!
//! The second law is held step by step: the entropy of the chamber's air less
//! what the air carried across the boundary of the chamber and its valve,
//! which lies on the reservoir's side (air that comes in brings the
//! reservoir's entropy; air that leaves carries its own temperature at the
//! reservoir's pressure, as an adiabatic throttle leaves it), and less the
//! heat carried to the wall at its temperature, is `S_gen`; it never falls.
//! The throttling through the valve, either way, the mixing of the air that
//! comes in and the heat that crosses the film all make it rise.
//!
//! The real limits, each by name. The valve is rated to 1.0 MPa over the
//! atmosphere; a pressure above it is not this valve. The mean free path
//! reaches a hundredth of the throat (Knudsen number 0.01) at 268.986 Pa for
//! the 2.53 mm throat that passes this conductance, and below it slip flow
//! begins at the throat's wall; the ISO form is a law of flow with no slip
//! at the walls, and a pressure below it is refused (`RarefiedFlow`). The
//! chamber's pressure stays between the two pressures given, so no run goes
//! below it. Air cooled below its critical temperature, 132.53 K, can
//! condense, and the ideal gas is not air there
//! (`ChamberBelowCriticalTemperature`); no admitted question reaches it, the
//! coldest admitted run, a vent from the valve's rating to the lowest
//! pressure taken, cooling the air to 157.7 K.
//!
//! What is left out. The pipe between the valve and the chamber, and the
//! time the valve takes to open: the valve is open from the start. The
//! film coefficient is one number. Air is dry.
//!
//! What was compared. This specimen's own comparison comes later. The ISO
//! form with constant `C` and `b` was measured, by charging and venting a
//! tank, to move with the supply pressure on two proportional pressure
//! regulators (Varga and Keski-Honkola 2012, EPJ Web of Conferences 25,
//! 01096, Tables 1 and 2): on the VPPM-6L-L-1-G18-0L10H from C = 0.90e-8
//! m^3/(s Pa) and b = 0.56 at 200 kPa to C = 1.55e-8 to 1.65e-8 and b =
//! 0.36 to 0.46 from 400 to 800 kPa. A valve whose `C` and `b` move like that is not this
//! valve; this specimen's are one pair. The source is
//! <https://doi.org/10.1051/epjconf/20122501096>, Retrieved 2026-10-14.

use genesis_kernel::math;
use crate::normal::NormalOrZero;
use std::fmt;

/// The public measurement the description cites.
pub const REFERENCE_NAME: &str = "Z. Varga and P. Keski-Honkola, Determination of flow rate characteristics for pneumatic valves, EPJ Web of Conferences 25 (2012) 01096, Tables 1 and 2";
pub const REFERENCE_URL: &str = "https://doi.org/10.1051/epjconf/20122501096";
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";

/// Sonic conductance of the valve, m^3/(s Pa).
pub const SONIC_CONDUCTANCE_M3_S_PA: f64 = 1.0e-8;
/// Critical pressure ratio of the valve.
pub const CRITICAL_PRESSURE_RATIO: f64 = 0.30;
/// Subsonic index of the valve: ISO 6358-1:2013's when none is measured.
pub const SUBSONIC_INDEX: f64 = 0.5;
/// Density of the standard's reference air, kg/m^3 (ISO 8778).
pub const REFERENCE_DENSITY_KG_M3: f64 = 1.185;
/// Temperature of the standard's reference air, K.
pub const REFERENCE_TEMPERATURE_K: f64 = 293.15;
/// Gas constant of dry air, J/(kg K).
pub const AIR_GAS_CONSTANT_J_KG_K: f64 = 287.05;
/// Ratio of the specific heats of air.
pub const AIR_GAMMA: f64 = 1.4;
/// Critical temperature of air, K.
pub const AIR_CRITICAL_TEMPERATURE_K: f64 = 132.53;
/// Kinetic diameter of an air molecule, m, for the mean free path.
pub const AIR_MOLECULE_DIAMETER_M: f64 = 3.66e-10;
/// Boltzmann's constant, J/K.
pub const BOLTZMANN_J_K: f64 = 1.380_649e-23;
/// Volume of the chamber, m^3.
pub const CHAMBER_VOLUME_M3: f64 = 1.0e-4;
/// Temperature of the chamber's wall, of the reservoir's air and of the
/// chamber's air at the start, K.
pub const WALL_TEMPERATURE_K: f64 = 293.15;
/// Film coefficient of the heat exchanged with the wall, W/(m^2 K).
pub const FILM_COEFFICIENT_W_M2_K: f64 = 30.0;
/// Pressure of the atmosphere outside, Pa.
pub const ATMOSPHERE_PA: f64 = 101_325.0;
/// The valve's rated pressure over the atmosphere, Pa.
pub const VALVE_RATING_GAUGE_PA: f64 = 1.0e6;
/// The Knudsen number at the valve's throat at which slip flow begins.
pub const KNUDSEN_LIMIT: f64 = 0.01;
/// The lowest pressure taken, Pa: [`rarefied_below_pa`] itself, the
/// pressure at which the mean free path at the wall temperature is a
/// hundredth of the throat. A test holds the two equal to the bit.
pub const PRESSURE_MIN_PA: f64 = 268.986_313_449_787_76;
/// The time step, s: a five-thousandth of the chamber's charging time, the
/// chamber's volume over `R T C rho_0`. The last step ends at the asked time.
pub const TIME_STEP_S: f64 = 2.0e-5;
/// The longest run, s.
pub const DURATION_MAX_S: f64 = 20.0;
/// The most samples of each history.
pub const SAMPLES: usize = 1000;
/// A substep of Runge-Kutta is taken so that the chamber's own rate (how
/// fast its pressure answers a change of itself through the valve and the
/// wall) times the substep is at most this.
pub const SUBSTEP_SHARE: f64 = 0.2;
/// Where that rate times the time left in a step is above this, the rest of
/// the step is one backward-Euler step.
pub const IMPLICIT_SHARE: f64 = 1.0;
/// The most substeps of Runge-Kutta in one step; the rest of a step past
/// them is one backward-Euler step.
pub const SUBSTEPS_MAX: u32 = 16;
/// Iterations allowed for the root of one backward-Euler step.
pub const IMPLICIT_ITERATIONS_MAX: u32 = 400;
/// The ledgers' residual over their scales, held at every step and substep.
/// The ledgers carry the chamber's mass and energy as their changes from the
/// start, so rounding adds a few parts in 1e17 of what moved in a step, and
/// the scales are the larger of the chamber's fill and what has moved
/// through the valve and the wall.
pub const LEDGER_RESIDUAL_LIMIT: f64 = 1.0e-10;
/// How far the entropy generated may fall in one step, over its scale
/// `m c_p`, before it is a breach of the second law and not rounding.
pub const ENTROPY_FALL_LIMIT: f64 = 1.0e-12;

fn cp() -> f64 {
    AIR_GAMMA * AIR_GAS_CONSTANT_J_KG_K / (AIR_GAMMA - 1.0)
}
fn cv() -> f64 {
    AIR_GAS_CONSTANT_J_KG_K / (AIR_GAMMA - 1.0)
}

/// A number as a sentence shows it: the shortest decimal that reads back to
/// the same 64-bit number, with an exponent when it is under a millionth or
/// past 1e21 in size, so that no sentence prints hundreds of digits.
pub fn shown(value: f64) -> String {
    let size = value.abs();
    if size == 0.0 || (1.0e-6..1.0e21).contains(&size) {
        format!("{value}")
    } else {
        format!("{value:e}")
    }
}

/// Diameter of the chamber, a cylinder as long as it is wide, m.
pub fn chamber_diameter_m() -> f64 {
    math::cbrt(4.0 * CHAMBER_VOLUME_M3 / std::f64::consts::PI)
}

/// Area of the chamber's wall, both ends and the side, m^2.
pub fn chamber_wall_area_m2() -> f64 {
    let d = chamber_diameter_m();
    1.5 * std::f64::consts::PI * d * d
}

/// Diameter of the throat that passes the sonic conductance when choked,
/// m: `C rho_0 sqrt(R T_0) = A sqrt(gamma) (2/(gamma+1))^((gamma+1)/(2(gamma-1)))`.
pub fn throat_diameter_m() -> f64 {
    let g = AIR_GAMMA;
    let flux = g.sqrt() * math::powf(2.0 / (g + 1.0), (g + 1.0) / (2.0 * (g - 1.0)));
    let area = SONIC_CONDUCTANCE_M3_S_PA * REFERENCE_DENSITY_KG_M3 * (AIR_GAS_CONSTANT_J_KG_K * REFERENCE_TEMPERATURE_K).sqrt() / flux;
    (4.0 * area / std::f64::consts::PI).sqrt()
}

/// The pressure at which the mean free path of air at the wall temperature
/// is a hundredth of the throat, Knudsen number [`KNUDSEN_LIMIT`]: below it
/// slip flow begins at the throat's wall, Pa.
pub fn rarefied_below_pa() -> f64 {
    BOLTZMANN_J_K * WALL_TEMPERATURE_K / (2f64.sqrt() * std::f64::consts::PI * AIR_MOLECULE_DIAMETER_M * AIR_MOLECULE_DIAMETER_M * KNUDSEN_LIMIT * throat_diameter_m())
}

/// The highest pressure the valve is rated to, absolute, Pa.
pub fn valve_rating_pa() -> f64 {
    ATMOSPHERE_PA + VALVE_RATING_GAUGE_PA
}

/// The ISO 6358 mass flow from side 1 to side 2, kg/s, and whether it is
/// choked, given both pressures and their difference `dp = p_1 - p_2`, not
/// negative. The regime is read from `p_2/p_1`; the subsonic form's `1 - x^2`
/// is taken as `(1 - x)(1 + x)` with `1 - x = dp/(p_1 (1 - b))`, so that near
/// equal pressures the flow is the difference's and not its rounding's.
pub fn iso6358_flow(p1: f64, t1: f64, p2: f64, dp: f64) -> (f64, bool) {
    let choked = SONIC_CONDUCTANCE_M3_S_PA * REFERENCE_DENSITY_KG_M3 * p1 * (REFERENCE_TEMPERATURE_K / t1).sqrt();
    if p2 / p1 <= CRITICAL_PRESSURE_RATIO {
        (choked, true)
    } else {
        let short = dp / (p1 * (1.0 - CRITICAL_PRESSURE_RATIO));
        let x = 1.0 - short;
        (choked * subsonic_share((short * (1.0 + x)).max(0.0)), false)
    }
}

/// `s^m`, the subsonic form's share of the choked flow, for `s = 1 - x^2`:
/// the square root when the index is a half, as it is here.
fn subsonic_share(s: f64) -> f64 {
    if SUBSONIC_INDEX == 0.5 { s.sqrt() } else { math::powf(s, SUBSONIC_INDEX) }
}

/// The ISO 6358 mass flow from side 1 to side 2, kg/s, and whether it is
/// choked. Needs `p1 >= p2`.
pub fn iso6358_mass_flow(p1: f64, t1: f64, p2: f64) -> (f64, bool) {
    iso6358_flow(p1, t1, p2, p1 - p2)
}

/// One question.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChamberQuestion {
    /// Absolute pressure of the reservoir on the other side of the valve, held.
    pub reservoir_pressure_pa: f64,
    /// Absolute pressure of the chamber's air at the start.
    pub chamber_start_pressure_pa: f64,
    /// Time from the valve's opening.
    pub duration_s: f64,
}

/// What the chamber did.
#[derive(Clone, Debug, PartialEq)]
pub struct Chamber {
    pub start_mass_flow_kg_s: f64,
    pub start_choked: bool,
    pub final_pressure_pa: f64,
    pub final_temperature_k: f64,
    pub temperature_min_k: f64,
    pub temperature_max_k: f64,
    /// Time at which the flow last stopped being choked, or zero.
    pub choked_until_s: f64,
    /// Time at which 95 percent of the pressure difference was covered, if
    /// it was.
    pub time_to_95_percent_s: Vec<f64>,
    pub mass_through_valve_kg: f64,
    pub enthalpy_through_valve_j: f64,
    pub heat_to_wall_j: f64,
    pub entropy_generated_j_k: f64,
    pub sample_times_s: Vec<f64>,
    pub pressure_history_pa: Vec<f64>,
    pub temperature_history_k: Vec<f64>,
    pub mass_flow_history_kg_s: Vec<f64>,
    pub choked_history: Vec<f64>,
    pub mass_ledger_residual: f64,
    pub energy_ledger_residual: f64,
    pub largest_entropy_fall: f64,
    /// The pieces stepped: steps and substeps of Runge-Kutta and
    /// backward-Euler steps.
    pub steps: u64,
    /// The steps of Runge-Kutta and their substeps alone.
    pub explicit_steps: u64,
    /// The backward-Euler steps alone.
    pub implicit_steps: u64,
    /// The chamber's pressure and temperature at the start and at the end of
    /// every step.
    pub field: Vec<f64>,
}

/// Why no run was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum ChamberReject {
    /// An input was NaN or infinite.
    InputNotFinite { field: &'static str },
    /// A pressure is above the valve's rating, [`VALVE_RATING_GAUGE_PA`] over the atmosphere.
    PressureAboveValveRating { pressure_pa: f64 },
    /// A pressure is below [`PRESSURE_MIN_PA`], where the mean free path is a hundredth of the valve's throat, Knudsen number [`KNUDSEN_LIMIT`], and slip flow begins: the ISO form is a law of flow with no slip at the walls.
    RarefiedFlow { pressure_pa: f64 },
    /// The duration is not above zero or is above [`DURATION_MAX_S`].
    DurationOutOfRange { duration_s: f64 },
    /// The chamber's air cooled below the critical temperature of air, [`AIR_CRITICAL_TEMPERATURE_K`], where it can condense. No admitted question reaches it: the coldest admitted run, a vent from the valve's rating to the lowest pressure, cools the air to 157.7 K.
    ChamberBelowCriticalTemperature { temperature_k: f64, time_s: f64 },
    /// A value of the run was NaN or infinite.
    NonFiniteValue { time_s: f64 },
    /// The root of a backward-Euler step did not settle in [`IMPLICIT_ITERATIONS_MAX`] iterations.
    StepNotSettled { time_s: f64 },
    /// The mass ledger did not close to [`LEDGER_RESIDUAL_LIMIT`] at a step.
    MassLedgerBroken { residual: f64, time_s: f64 },
    /// The energy ledger did not close to [`LEDGER_RESIDUAL_LIMIT`] at a step, or the air that came in did not bring the reservoir's enthalpy.
    EnergyLedgerBroken { residual: f64, time_s: f64 },
    /// The entropy generated fell in a step by more than [`ENTROPY_FALL_LIMIT`] of its scale: the second law broken.
    EntropyFell { fall: f64, time_s: f64 },
}

impl fmt::Display for ChamberReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotFinite { field } => write!(f, "{field} is not a finite number"),
            Self::PressureAboveValveRating { pressure_pa } => write!(
                f,
                "pressure {} Pa is above the valve's rating, {} Pa absolute",
                shown(*pressure_pa),
                shown(valve_rating_pa())
            ),
            Self::RarefiedFlow { pressure_pa } => write!(
                f,
                "pressure {} Pa is below {} Pa, where the mean free path is a hundredth of the valve's throat and slip flow begins",
                shown(*pressure_pa),
                shown(PRESSURE_MIN_PA)
            ),
            Self::DurationOutOfRange { duration_s } => {
                write!(f, "duration {} s is not above 0 s and at most {} s", shown(*duration_s), shown(DURATION_MAX_S))
            }
            Self::ChamberBelowCriticalTemperature { temperature_k, time_s } => write!(
                f,
                "at {} s the chamber's air cooled to {} K, below the critical temperature of air, {} K",
                shown(*time_s),
                shown(*temperature_k),
                shown(AIR_CRITICAL_TEMPERATURE_K)
            ),
            Self::NonFiniteValue { time_s } => write!(f, "at {} s a value of the run was not finite", shown(*time_s)),
            Self::StepNotSettled { time_s } => write!(
                f,
                "at {} s the root of a backward-Euler step did not settle in {IMPLICIT_ITERATIONS_MAX} iterations",
                shown(*time_s)
            ),
            Self::MassLedgerBroken { residual, time_s } => write!(
                f,
                "at {} s the mass ledger was off by {} of its scale, above {}",
                shown(*time_s),
                shown(*residual),
                shown(LEDGER_RESIDUAL_LIMIT)
            ),
            Self::EnergyLedgerBroken { residual, time_s } => write!(
                f,
                "at {} s the energy ledger was off by {} of its scale, above {}",
                shown(*time_s),
                shown(*residual),
                shown(LEDGER_RESIDUAL_LIMIT)
            ),
            Self::EntropyFell { fall, time_s } => write!(
                f,
                "at {} s the entropy generated fell by {} of its scale in one step, above {}",
                shown(*time_s),
                shown(*fall),
                shown(ENTROPY_FALL_LIMIT)
            ),
        }
    }
}

/// Every limit the question shows.
pub fn admit(question: &ChamberQuestion) -> Result<(), ChamberReject> {
    for (field, value) in [
        ("reservoir_pressure_pa", question.reservoir_pressure_pa),
        ("chamber_start_pressure_pa", question.chamber_start_pressure_pa),
        ("duration_s", question.duration_s),
    ] {
        if !value.is_finite() {
            return Err(ChamberReject::InputNotFinite { field });
        }
    }
    for pressure_pa in [question.reservoir_pressure_pa, question.chamber_start_pressure_pa] {
        if pressure_pa > valve_rating_pa() {
            return Err(ChamberReject::PressureAboveValveRating { pressure_pa });
        }
        if pressure_pa < PRESSURE_MIN_PA {
            return Err(ChamberReject::RarefiedFlow { pressure_pa });
        }
    }
    if !(question.duration_s > 0.0 && question.duration_s <= DURATION_MAX_S) {
        return Err(ChamberReject::DurationOutOfRange { duration_s: question.duration_s });
    }
    Ok(())
}

/// The number of steps of a run: [`TIME_STEP_S`] each, the last one shorter
/// so that the run ends at the asked time. A duration within a part in 1e9
/// of a whole number of steps is that number of steps.
pub fn step_count(duration_s: f64) -> u64 {
    let ratio = duration_s / TIME_STEP_S;
    let whole = ratio.round();
    if whole >= 1.0 && (ratio - whole).abs() <= 1.0e-9 * whole { whole as u64 } else { (ratio.ceil() as u64).max(1) }
}

/// A wrong step a test may ask for. Every served run passes `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tamper {
    None,
    /// The energy is stepped by explicit Euler while the enthalpy and the
    /// heat are stepped by Runge-Kutta.
    EulerEnergy,
    /// Air that comes in brings the chamber's enthalpy, not the reservoir's.
    WrongInflowEnthalpy,
    /// The valve's flow goes from the lower pressure to the higher.
    FlowUphill,
    /// Every step and substep adds a part in 1e9 of the chamber's mass to
    /// the chamber, booked nowhere.
    LeakMass,
    /// A backward-Euler step takes its heat out of the chamber's air and
    /// does not book it to the wall.
    ImplicitHeatUnbooked,
}

/// The chamber at the start, from which its state is carried as changes.
#[derive(Clone, Copy, Debug)]
struct Start {
    m0: f64,
    u0: f64,
    p0: f64,
}

/// The chamber's state from the changes of its mass and energy.
#[derive(Clone, Copy, Debug)]
struct State {
    m: f64,
    u: f64,
    t: f64,
    p: f64,
}

impl Start {
    /// The state from the changes of mass and energy, each a sum carried
    /// with its rounding. The pressure is `p_0 + (gamma - 1) du / V`, exact
    /// for an ideal gas in a rigid chamber, taken with the product's own
    /// rounding (a fused multiply-add), so that the pressure is known to its
    /// last bit near the start and after the chamber has moved far from it.
    fn state(&self, dm: Sum, du: Sum) -> State {
        let m = (self.m0 + dm.total) + dm.carry;
        let u = (self.u0 + du.total) + du.carry;
        let c = (AIR_GAMMA - 1.0) / CHAMBER_VOLUME_M3;
        let high = c * du.total;
        let low = c.mul_add(du.total, -high);
        State { m, u, t: u / (m * cv()), p: ((self.p0 + high) + low) + c * du.carry }
    }
}

/// The rates of the chamber's mass and energy, of the mass and enthalpy
/// through the valve and the heat to the wall, and of the mass and enthalpy
/// that came in; with the valve's flow into the chamber and its regime.
fn rates(here: &State, reservoir: f64, tamper: Tamper) -> ([f64; 7], f64, bool) {
    let (t, p) = (here.t, here.p);
    let (q_in, choked, h) = if reservoir >= p {
        let (q, choked) = iso6358_flow(reservoir, WALL_TEMPERATURE_K, p, reservoir - p);
        let h = if tamper == Tamper::WrongInflowEnthalpy { cp() * t } else { cp() * WALL_TEMPERATURE_K };
        (q, choked, h)
    } else {
        let (q, choked) = iso6358_flow(p, t, reservoir, p - reservoir);
        (-q, choked, cp() * t)
    };
    let q_in = if tamper == Tamper::FlowUphill { -q_in } else { q_in };
    let heat = FILM_COEFFICIENT_W_M2_K * chamber_wall_area_m2() * (t - WALL_TEMPERATURE_K);
    let enthalpy = q_in * h;
    // What comes in, alone: its mass and the enthalpy it brings.
    let (mass_in, enthalpy_in) = if q_in > 0.0 { (q_in, enthalpy) } else { (0.0, 0.0) };
    ([q_in, enthalpy - heat, q_in, enthalpy, heat, mass_in, enthalpy_in], q_in, choked)
}

/// How fast the chamber's pressure answers a change of itself, 1/s: the
/// valve's `|d(dp/dt)/dp|`, `gamma R T_in / V` times the slope of the ISO
/// form, and the wall's `h A / (m c_v)`. Infinite at equal pressures, where
/// the subsonic form's slope is.
fn stiffness(here: &State, reservoir: f64) -> f64 {
    let (t, p) = (here.t, here.p);
    let b = CRITICAL_PRESSURE_RATIO;
    let index = SUBSONIC_INDEX;
    let wall = FILM_COEFFICIENT_W_M2_K * chamber_wall_area_m2() / (here.m * cv());
    let valve = if reservoir >= p {
        let c = SONIC_CONDUCTANCE_M3_S_PA * REFERENCE_DENSITY_KG_M3 * (REFERENCE_TEMPERATURE_K / WALL_TEMPERATURE_K).sqrt();
        if p / reservoir <= b {
            0.0
        } else {
            let short = (reservoir - p) / (reservoir * (1.0 - b));
            let x = 1.0 - short;
            let s = short * (1.0 + x);
            if s <= 0.0 {
                f64::INFINITY
            } else {
                AIR_GAMMA * AIR_GAS_CONSTANT_J_KG_K * WALL_TEMPERATURE_K / CHAMBER_VOLUME_M3 * 2.0 * index * c * x * subsonic_share(s) / s / (1.0 - b)
            }
        }
    } else {
        let c = SONIC_CONDUCTANCE_M3_S_PA * REFERENCE_DENSITY_KG_M3 * (REFERENCE_TEMPERATURE_K / t).sqrt();
        let ratio = reservoir / p;
        let slope = if ratio <= b {
            c
        } else {
            let short = (p - reservoir) / (p * (1.0 - b));
            let x = 1.0 - short;
            let s = short * (1.0 + x);
            if s <= 0.0 {
                f64::INFINITY
            } else {
                let share = subsonic_share(s);
                c * share + 2.0 * index * c * x * ratio * share / s / (1.0 - b)
            }
        };
        AIR_GAMMA * AIR_GAS_CONSTANT_J_KG_K * t / CHAMBER_VOLUME_M3 * slope
    };
    valve + wall
}

/// What one piece of a step moved: the chamber's mass and energy, the mass
/// and enthalpy through the valve, the heat to the wall, the mass and
/// enthalpy that came in; and the entropy carried across the boundary.
struct Moved {
    rows: [f64; 7],
    entropy_flux: f64,
}

/// Entropy of the chamber's air, J/K, from its own reference at the wall
/// temperature and the atmosphere's pressure.
fn chamber_entropy(m: f64, t: f64, p: f64) -> f64 {
    m * specific_entropy(t, p)
}

/// Specific entropy of air at a temperature and pressure, from the same reference.
fn specific_entropy(t: f64, p: f64) -> f64 {
    cp() * math::log(t / WALL_TEMPERATURE_K) - AIR_GAS_CONSTANT_J_KG_K * math::log(p / ATMOSPHERE_PA)
}

/// The specific entropy the valve's air carries across the boundary of the
/// chamber and its valve, which lies on the reservoir's side: air that comes
/// in, the reservoir's; air that leaves, its own temperature at the
/// reservoir's pressure, as an adiabatic throttle leaves it. So the
/// throttling is generated inside the boundary whichever way the air goes.
fn crossing_entropy(q_in: f64, t: f64, valve: &Valve) -> f64 {
    if q_in >= 0.0 { valve.entropy_in } else { specific_entropy(t, valve.reservoir) }
}

/// The reservoir's side of the valve: its pressure and the specific entropy
/// of its air.
struct Valve {
    reservoir: f64,
    entropy_in: f64,
}

/// A sum carried with its rounding (Neumaier's), so that a long run of
/// small changes adds to a large total with no drift.
#[derive(Clone, Copy, Debug, Default)]
struct Sum {
    total: f64,
    carry: f64,
}

impl Sum {
    fn add(&mut self, x: f64) {
        let t = self.total + x;
        self.carry += if self.total.abs() >= x.abs() { (self.total - t) + x } else { (x - t) + self.total };
        self.total = t;
    }
    fn value(&self) -> f64 {
        self.total + self.carry
    }
}

/// One step or substep of classical fourth-order Runge-Kutta, `h` long.
fn explicit_piece(start: &Start, dm: Sum, du: Sum, valve: &Valve, h: f64, tamper: Tamper) -> Moved {
    let flux_of = |step: f64, k: &[f64; 7]| -> ([f64; 7], f64) {
        let stage = |sum: Sum, rate: f64| Sum { total: sum.total + step * rate, carry: sum.carry };
        let here = start.state(stage(dm, k[0]), stage(du, k[1]));
        let (r, q_in, _) = rates(&here, valve.reservoir, tamper);
        (r, q_in * crossing_entropy(q_in, here.t, valve) - r[4] / WALL_TEMPERATURE_K)
    };
    let (k1, e1) = flux_of(0.0, &[0.0; 7]);
    let (k2, e2) = flux_of(0.5 * h, &k1);
    let (k3, e3) = flux_of(0.5 * h, &k2);
    let (k4, e4) = flux_of(h, &k3);
    let mut rows: [f64; 7] = std::array::from_fn(|i| h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]));
    if tamper == Tamper::EulerEnergy {
        rows[1] = h * k1[1];
    }
    Moved { rows, entropy_flux: h / 6.0 * (e1 + 2.0 * e2 + 2.0 * e3 + e4) }
}

/// One backward-Euler step `h` long from the chamber's state: the valve's
/// flow and the heat to the wall taken at the step's end, so that the step
/// cannot carry the chamber past the reservoir's pressure. The unknown is the
/// square root `sigma` of the end's pressure difference to the reservoir, in
/// which the subsonic form is smooth; for each `sigma` the step's change of
/// mass and energy follows in closed form from the energy balance, written in
/// the changes alone, and the root is bracketed and found by the Illinois
/// method, the bracket first narrowed about the root foreseen from the last
/// two steps' roots, `guess`. `None` when it does not settle.
fn implicit_piece(here: &State, valve: &Valve, h: f64, guess: &mut [f64; 2]) -> Option<Moved> {
    let reservoir = valve.reservoir;
    let film = FILM_COEFFICIENT_W_M2_K * chamber_wall_area_m2();
    let gm1 = AIR_GAMMA - 1.0;
    let (m, u, t) = (here.m, here.u, here.t);
    // The reservoir's pressure less the chamber's now.
    let gap = reservoir - here.p;
    // The step's change of energy with no flow through the valve: the
    // wall's heat alone, taken at the end.
    let still_du = -h * film * (t - WALL_TEMPERATURE_K) / (1.0 + h * film / (m * cv()));
    let still_gap = gap - gm1 * still_du / CHAMBER_VOLUME_M3;
    let inflow = still_gap > 0.0;
    // For a sigma: the residual, in pascals for a charge and in kilograms
    // for a vent; the step's change of mass; and its change of energy.
    let end = |sigma: f64| -> (f64, f64, f64) {
        let square = sigma * sigma;
        if inflow {
            let (q, _) = iso6358_flow(reservoir, WALL_TEMPERATURE_K, reservoir - square, square);
            let dm = h * q;
            let keep = 1.0 + h * film / ((m + dm) * cv());
            let du = (dm * cp() * WALL_TEMPERATURE_K - h * film * (u / ((m + dm) * cv()) - WALL_TEMPERATURE_K)) / keep;
            (gm1 * du / CHAMBER_VOLUME_M3 - (gap - square), dm, du)
        } else {
            let du = (square + gap) * CHAMBER_VOLUME_M3 / gm1;
            let dm = (h * film * m * (WALL_TEMPERATURE_K - t) - du * (m + h * film / cv())) / ((u + du) * (1.0 - AIR_GAMMA) - u - h * film * WALL_TEMPERATURE_K);
            let t_end = (u + du) / ((m + dm) * cv());
            let (q, _) = iso6358_flow(reservoir + square, t_end, reservoir, square);
            (dm + h * q, dm, du)
        }
    };
    let (dm, du) = if still_gap == 0.0 {
        (0.0, still_du)
    } else {
        // Each point is (sigma, residual, change of mass, change of energy).
        let at = |sigma: f64| -> (f64, f64, f64, f64) {
            let (g, dm, du) = end(sigma);
            (sigma, g, dm, du)
        };
        let top = still_gap.abs().sqrt();
        let mut low = at(0.0);
        // The size of the residual's terms near the root sets what rounding
        // is: near a steady state the gaps are small, and the wall's heat,
        // taken from the temperature to its last bit, sets the floor.
        let heat_term = h * film * t;
        let tolerance = 8.0 * f64::EPSILON
            * if inflow {
                gap.abs() + still_gap.abs() + gm1 * heat_term / CHAMBER_VOLUME_M3
            } else {
                low.1.abs() + m * (heat_term + (gap * CHAMBER_VOLUME_M3 / gm1).abs()) / (AIR_GAMMA * u)
            };
        let mut root = if low.1.abs() <= tolerance { Some(low) } else { None };
        if root.is_none() && low.1 >= 0.0 {
            // The residual rises with sigma for a charge and for a vent
            // alike, from below zero: rounding alone put it above.
            return None;
        }
        // The root foreseen from the last two steps' roots, and parts in 1e9,
        // 1e6 and 1e3 of it to the side where the root lies, narrow the
        // bracket: near a steady state the root moves little and smoothly.
        let foreseen = if guess[1] > 0.0 { (2.0 * guess[0] - guess[1]).max(0.5 * guess[0]) } else { guess[0] };
        let mut high: Option<(f64, f64, f64, f64)> = None;
        if root.is_none() && foreseen > 0.0 && foreseen < top {
            let first = at(foreseen);
            let side = if first.1 < 0.0 { 1.0 } else { -1.0 };
            let trials = [foreseen, foreseen * (1.0 + side * 1.0e-9), foreseen * (1.0 + side * 1.0e-6), foreseen * (1.0 + side * 1.0e-3)];
            for (k, trial) in trials.into_iter().enumerate() {
                if !(trial > low.0 && trial < top) || high.is_some() || root.is_some() {
                    continue;
                }
                let point = if k == 0 { first } else { at(trial) };
                if point.1.abs() <= tolerance {
                    root = Some(point);
                } else if point.1 < 0.0 {
                    low = point;
                } else {
                    high = Some(point);
                }
            }
        }
        let mut high = match high {
            Some(point) => point,
            None => at(top),
        };
        if root.is_none() && high.1.abs() <= tolerance {
            root = Some(high);
        } else if root.is_none() && high.1 < 0.0 {
            return None;
        }
        // Illinois steps, and a halving wherever a step did not halve the
        // bracket, until the residual is rounding or the bracket is a part in
        // 1e15 wide.
        let (mut ga, mut gb) = (low.1, high.1);
        let (mut side, mut iterations, mut halve) = (0, 0, false);
        while root.is_none() {
            iterations += 1;
            if iterations > IMPLICIT_ITERATIONS_MAX {
                return None;
            }
            let (a, b) = (low.0, high.0);
            let width = b - a;
            let secant = (a * gb - b * ga) / (gb - ga);
            if width <= 4.0 * f64::EPSILON * b {
                root = Some(if low.1.abs() <= high.1.abs() { low } else { high });
                break;
            }
            let c = if !halve && secant > a && secant < b { secant } else { 0.5 * (a + b) };
            let point = at(c);
            if point.1.abs() <= tolerance {
                root = Some(point);
            } else if point.1 < 0.0 {
                low = point;
                ga = point.1;
                if side == -1 {
                    gb *= 0.5;
                }
                side = -1;
            } else {
                high = point;
                gb = point.1;
                if side == 1 {
                    ga *= 0.5;
                }
                side = 1;
            }
            halve = high.0 - low.0 > 0.5 * width;
        }
        let (sigma, _, dm, du) = root.expect("found");
        *guess = [sigma, guess[0]];
        (dm, du)
    };
    let t_end = (u + du) / ((m + dm) * cv());
    let heat = h * film * (t_end - WALL_TEMPERATURE_K);
    let h_valve = if dm > 0.0 { cp() * WALL_TEMPERATURE_K } else { cp() * t_end };
    let enthalpy = dm * h_valve;
    let (mass_in, enthalpy_in) = if dm > 0.0 { (dm, enthalpy) } else { (0.0, 0.0) };
    let entropy_flux = dm * crossing_entropy(dm, t_end, valve) - heat / WALL_TEMPERATURE_K;
    Some(Moved { rows: [dm, enthalpy - heat, dm, enthalpy, heat, mass_in, enthalpy_in], entropy_flux })
}

/// Charges or vents the chamber.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &ChamberQuestion) -> Result<Chamber, ChamberReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &ChamberQuestion) -> Result<Chamber, ChamberReject> {
    admit(question)?;
    run(question, Tamper::None)
}

impl NormalOrZero for Chamber {
    fn normal_or_zero(&mut self) {
        self.start_mass_flow_kg_s.normal_or_zero();
        self.final_pressure_pa.normal_or_zero();
        self.final_temperature_k.normal_or_zero();
        self.temperature_min_k.normal_or_zero();
        self.temperature_max_k.normal_or_zero();
        self.choked_until_s.normal_or_zero();
        self.time_to_95_percent_s.normal_or_zero();
        self.mass_through_valve_kg.normal_or_zero();
        self.enthalpy_through_valve_j.normal_or_zero();
        self.heat_to_wall_j.normal_or_zero();
        self.entropy_generated_j_k.normal_or_zero();
        self.sample_times_s.normal_or_zero();
        self.pressure_history_pa.normal_or_zero();
        self.temperature_history_k.normal_or_zero();
        self.mass_flow_history_kg_s.normal_or_zero();
        self.choked_history.normal_or_zero();
        self.mass_ledger_residual.normal_or_zero();
        self.energy_ledger_residual.normal_or_zero();
        self.largest_entropy_fall.normal_or_zero();
        self.field.normal_or_zero();
    }
}

fn run(question: &ChamberQuestion, tamper: Tamper) -> Result<Chamber, ChamberReject> {
    let reservoir = question.reservoir_pressure_pa;
    let dt = TIME_STEP_S;
    let steps = step_count(question.duration_s);
    let stride = ((steps as usize + 1).div_ceil(SAMPLES)).max(1);
    let p_start = question.chamber_start_pressure_pa;
    let m0 = p_start * CHAMBER_VOLUME_M3 / (AIR_GAS_CONSTANT_J_KG_K * WALL_TEMPERATURE_K);
    let start = Start { m0, u0: m0 * cv() * WALL_TEMPERATURE_K, p0: p_start };
    // State: the chamber's mass and energy as their changes from the start,
    // the mass and enthalpy through the valve, the heat to the wall, and the
    // mass and the enthalpy that came in, each summed with its rounding.
    // Carrying the changes, not the totals, and summing them so, keeps a
    // step's rounding to a part in 1e16 of what it moved.
    let mut sums = [Sum::default(); 7];
    let mut y = [0.0f64; 7];
    let valve = Valve { reservoir, entropy_in: specific_entropy(WALL_TEMPERATURE_K, reservoir) };
    let mass_scale = m0.max(reservoir * CHAMBER_VOLUME_M3 / (AIR_GAS_CONSTANT_J_KG_K * WALL_TEMPERATURE_K));
    let energy_scale = mass_scale * cv() * WALL_TEMPERATURE_K;
    let (_, q_start, choked_start) = rates(&start.state(Sum::default(), Sum::default()), reservoir, tamper);
    let mut out = Chamber {
        start_mass_flow_kg_s: q_start,
        start_choked: choked_start,
        final_pressure_pa: p_start,
        final_temperature_k: WALL_TEMPERATURE_K,
        temperature_min_k: WALL_TEMPERATURE_K,
        temperature_max_k: WALL_TEMPERATURE_K,
        choked_until_s: 0.0,
        time_to_95_percent_s: Vec::new(),
        mass_through_valve_kg: 0.0,
        enthalpy_through_valve_j: 0.0,
        heat_to_wall_j: 0.0,
        entropy_generated_j_k: 0.0,
        sample_times_s: vec![0.0],
        pressure_history_pa: vec![p_start],
        temperature_history_k: vec![WALL_TEMPERATURE_K],
        mass_flow_history_kg_s: vec![q_start],
        choked_history: vec![if choked_start { 1.0 } else { 0.0 }],
        mass_ledger_residual: 0.0,
        energy_ledger_residual: 0.0,
        largest_entropy_fall: 0.0,
        steps: 0,
        explicit_steps: 0,
        implicit_steps: 0,
        field: vec![p_start, WALL_TEMPERATURE_K],
    };
    let target = p_start + 0.95 * (reservoir - p_start);
    let entropy0 = chamber_entropy(m0, WALL_TEMPERATURE_K, p_start);
    // The entropy that crosses the boundary, carried in or out by air and
    // out by heat, summed with the same weights as the steps.
    let mut entropy_sum = Sum::default();
    let mut entropy_flux = 0.0;
    let mut generated_before = 0.0;
    let (mut moved_mass, mut moved_energy) = (0.0f64, 0.0f64);
    // The last two backward-Euler steps' roots, to narrow the next one's bracket.
    let mut guess = [0.0f64; 2];
    for n in 1..=steps {
        let time = if n == steps { question.duration_s } else { n as f64 * dt };
        let mut left = if n == steps { question.duration_s - (steps - 1) as f64 * dt } else { dt };
        let mut substeps = 0u32;
        let mut here = start.state(sums[0], sums[1]);
        // The step: Runge-Kutta while it follows the chamber's own rate, in
        // substeps where it needs them, and one backward-Euler step for the
        // rest where even they cannot.
        while left > 0.0 {
            let rate = stiffness(&here, reservoir);
            let implicit = !(rate * left <= IMPLICIT_SHARE) || substeps >= SUBSTEPS_MAX;
            let h = if implicit || rate * left <= SUBSTEP_SHARE { left } else { SUBSTEP_SHARE / rate };
            let at = time - (left - h).max(0.0);
            let mut moved = if implicit {
                out.implicit_steps += 1;
                let mut moved = implicit_piece(&here, &valve, h, &mut guess).ok_or(ChamberReject::StepNotSettled { time_s: at })?;
                if tamper == Tamper::ImplicitHeatUnbooked {
                    moved.rows[4] = 0.0;
                }
                moved
            } else {
                out.explicit_steps += 1;
                substeps += 1;
                explicit_piece(&start, sums[0], sums[1], &valve, h, tamper)
            };
            if tamper == Tamper::LeakMass {
                moved.rows[0] += 1.0e-9 * here.m;
            }
            left = if h >= left { 0.0 } else { left - h };
            let before = y;
            for ((sum, state), change) in sums.iter_mut().zip(y.iter_mut()).zip(moved.rows) {
                sum.add(change);
                *state = sum.value();
            }
            entropy_sum.add(moved.entropy_flux);
            entropy_flux = entropy_sum.value();
            out.steps += 1;
            here = start.state(sums[0], sums[1]);
            if !(here.t.is_finite() && here.p.is_finite() && y.iter().all(|v| v.is_finite()) && entropy_flux.is_finite()) || here.m <= 0.0 {
                return Err(ChamberReject::NonFiniteValue { time_s: at });
            }
            // The ledgers, at every step and substep. Rounding grows with
            // what has been moved, so the scales do too.
            moved_mass += (y[2] - before[2]).abs();
            moved_energy += (y[3] - before[3]).abs() + (y[4] - before[4]).abs();
            let mass_residual = (y[0] - y[2]) / mass_scale.max(moved_mass);
            // The energy ledger, and what came in brought the reservoir's enthalpy.
            let energy_scale_now = energy_scale.max(moved_energy);
            let energy_residual = (y[1] - y[3] + y[4]) / energy_scale_now;
            let inflow_residual = (y[6] - cp() * WALL_TEMPERATURE_K * y[5]) / energy_scale_now;
            let energy_residual = if inflow_residual.abs() > energy_residual.abs() { inflow_residual } else { energy_residual };
            out.mass_ledger_residual = out.mass_ledger_residual.max(mass_residual.abs());
            out.energy_ledger_residual = out.energy_ledger_residual.max(energy_residual.abs());
            if mass_residual.abs() > LEDGER_RESIDUAL_LIMIT {
                return Err(ChamberReject::MassLedgerBroken { residual: mass_residual, time_s: at });
            }
            if energy_residual.abs() > LEDGER_RESIDUAL_LIMIT {
                return Err(ChamberReject::EnergyLedgerBroken { residual: energy_residual, time_s: at });
            }
            if here.t < AIR_CRITICAL_TEMPERATURE_K {
                return Err(ChamberReject::ChamberBelowCriticalTemperature { temperature_k: here.t, time_s: at });
            }
            out.temperature_min_k = out.temperature_min_k.min(here.t);
            out.temperature_max_k = out.temperature_max_k.max(here.t);
        }
        // The second law, at every step.
        let generated = chamber_entropy(here.m, here.t, here.p) - entropy0 - entropy_flux;
        let fall = (generated_before - generated) / (mass_scale * cp());
        out.largest_entropy_fall = out.largest_entropy_fall.max(fall);
        if fall > ENTROPY_FALL_LIMIT {
            return Err(ChamberReject::EntropyFell { fall, time_s: time });
        }
        generated_before = generated;
        out.entropy_generated_j_k = generated;
        let (t, p) = (here.t, here.p);
        let (_, q_in, choked) = rates(&here, reservoir, tamper);
        if choked {
            out.choked_until_s = time;
        }
        if out.time_to_95_percent_s.is_empty() && (p - target) * (reservoir - p_start) >= 0.0 && reservoir != p_start {
            out.time_to_95_percent_s.push(time);
        }
        out.field.push(p);
        out.field.push(t);
        if (n as usize) % stride == 0 || n == steps {
            out.sample_times_s.push(time);
            out.pressure_history_pa.push(p);
            out.temperature_history_k.push(t);
            out.mass_flow_history_kg_s.push(q_in);
            out.choked_history.push(if choked { 1.0 } else { 0.0 });
        }
        out.final_pressure_pa = p;
        out.final_temperature_k = t;
    }
    out.mass_through_valve_kg = y[2];
    out.enthalpy_through_valve_j = y[3];
    out.heat_to_wall_j = y[4];
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(reservoir: f64, start: f64, duration: f64) -> ChamberQuestion {
        ChamberQuestion { reservoir_pressure_pa: reservoir, chamber_start_pressure_pa: start, duration_s: duration }
    }

    #[test]
    fn the_specimen_numbers() {
        assert!((chamber_diameter_m() - 0.050_31).abs() < 1e-5, "{}", chamber_diameter_m());
        assert!((chamber_wall_area_m2() - 0.011_93).abs() < 1e-5, "{}", chamber_wall_area_m2());
        assert!((throat_diameter_m() - 2.53e-3).abs() < 0.01e-3, "{}", throat_diameter_m());
        let floor = rarefied_below_pa();
        assert!(floor > 250.0 && floor < 290.0, "{floor}");
        // The form is continuous at b and falls to zero at equal pressures.
        let (at_b, choked) = iso6358_mass_flow(6.0e5, 293.15, 6.0e5 * CRITICAL_PRESSURE_RATIO);
        let (above_b, _) = iso6358_mass_flow(6.0e5, 293.15, 6.0e5 * CRITICAL_PRESSURE_RATIO * (1.0 + 1e-12));
        assert!(choked && (at_b / above_b - 1.0).abs() < 1e-9);
        assert_eq!(iso6358_mass_flow(6.0e5, 293.15, 6.0e5).0, 0.0);
        // 1.0 dm^3/(s bar) at 6 bar absolute: 6.0 dm^3/s of reference air.
        assert!((at_b / REFERENCE_DENSITY_KG_M3 - 6.0e-3).abs() < 1e-12);
    }

    #[test]
    fn a_charge_from_atmosphere_is_choked_then_settles_and_the_ledgers_close() {
        let c = solve(&ask(7.0e5, ATMOSPHERE_PA, 20.0)).unwrap();
        eprintln!(
            "charge: choked until {} s, 95 percent at {:?}, T max {}, final {} Pa {} K, heat {} J, S_gen {} J/K, residuals {:e} {:e} fall {:e}",
            c.choked_until_s, c.time_to_95_percent_s, c.temperature_max_k, c.final_pressure_pa, c.final_temperature_k, c.heat_to_wall_j, c.entropy_generated_j_k, c.mass_ledger_residual, c.energy_ledger_residual, c.largest_entropy_fall
        );
        assert!(c.start_choked);
        // Choked while the chamber is under b of the reservoir: the first
        // part of the charge, at a near-constant mass flow.
        assert!(c.choked_until_s > 0.0 && c.choked_until_s < 0.1, "{}", c.choked_until_s);
        // Charging heats the air; the wall then takes the heat back.
        assert!(c.temperature_max_k > 330.0);
        assert!((c.final_pressure_pa / 7.0e5 - 1.0).abs() < 1e-6, "{}", c.final_pressure_pa);
        assert!((c.final_temperature_k - WALL_TEMPERATURE_K).abs() < 0.5, "{}", c.final_temperature_k);
        assert!(c.mass_ledger_residual < 1e-13 && c.energy_ledger_residual < 1e-12, "{} {}", c.mass_ledger_residual, c.energy_ledger_residual);
        assert!(c.entropy_generated_j_k > 0.0);
    }

    /// With no heat to the wall the charge of an empty chamber is the
    /// classical one: the final temperature is gamma times the reservoir's
    /// when the chamber starts empty; from a start of p0 it is
    /// `T = gamma T_r p / (p - p0 + gamma p0)`, from `p V / (gamma - 1) - p0 V / (gamma - 1) = c_p T_r dm`.
    #[test]
    fn the_first_law_of_a_fast_charge() {
        let c = solve(&ask(7.0e5, ATMOSPHERE_PA, 0.06)).unwrap();
        let p0 = ATMOSPHERE_PA;
        let gamma = AIR_GAMMA;
        // The film takes little in 60 ms; with it the gas is a little cooler.
        let adiabatic = gamma * WALL_TEMPERATURE_K * c.final_pressure_pa / (c.final_pressure_pa - p0 + gamma * p0);
        eprintln!("fast charge to {} Pa: {} K against adiabatic {} K", c.final_pressure_pa, c.final_temperature_k, adiabatic);
        assert!(c.final_temperature_k < adiabatic && c.final_temperature_k > adiabatic - 5.0);
    }

    #[test]
    fn a_vent_cools_the_air_and_the_ledgers_close() {
        let c = solve(&ask(ATMOSPHERE_PA, 1.0e6, 5.0)).unwrap();
        eprintln!("vent: choked until {} T min {} final {} {} S_gen {}", c.choked_until_s, c.temperature_min_k, c.final_pressure_pa, c.final_temperature_k, c.entropy_generated_j_k);
        assert!(c.start_choked && c.start_mass_flow_kg_s < 0.0);
        assert!(c.temperature_min_k < 250.0);
        assert!((c.final_pressure_pa / ATMOSPHERE_PA - 1.0).abs() < 1e-6);
        assert!(c.mass_ledger_residual < 1e-13 && c.energy_ledger_residual < 1e-12, "{} {}", c.mass_ledger_residual, c.energy_ledger_residual);
    }

    #[test]
    fn the_question_is_refused_by_name_at_its_limits() {
        assert!(matches!(admit(&ask(f64::NAN, 1e5, 1.0)), Err(ChamberReject::InputNotFinite { .. })));
        assert!(matches!(admit(&ask(valve_rating_pa() * (1.0 + 1e-15) + 1.0, 1e5, 1.0)), Err(ChamberReject::PressureAboveValveRating { .. })));
        assert!(matches!(admit(&ask(1e5, 100.0, 1.0)), Err(ChamberReject::RarefiedFlow { .. })));
        assert!(matches!(admit(&ask(1e5, 2e5, 0.0)), Err(ChamberReject::DurationOutOfRange { .. })));
        assert!(matches!(admit(&ask(1e5, 2e5, 20.000000000000004)), Err(ChamberReject::DurationOutOfRange { .. })));
        admit(&ask(valve_rating_pa(), PRESSURE_MIN_PA, 20.0)).unwrap();
        assert!(matches!(admit(&ask(1e5, PRESSURE_MIN_PA.next_down(), 1.0)), Err(ChamberReject::RarefiedFlow { .. })));
    }

    /// The published floor is the computed limit to the bit: the floor itself
    /// is taken, and one 64-bit number below it is refused by its name. The
    /// red team's 268.987 Pa and 268.99999 Pa, inside the limit, are taken.
    #[test]
    fn the_published_floor_is_the_computed_limit() {
        assert_eq!(PRESSURE_MIN_PA.to_bits(), rarefied_below_pa().to_bits(), "{} {}", PRESSURE_MIN_PA, rarefied_below_pa());
        for floor in [PRESSURE_MIN_PA, 268.987, 268.99, 268.999_99] {
            admit(&ask(floor, 1.0e5, 1.0)).unwrap();
            admit(&ask(1.0e5, floor, 1.0)).unwrap();
        }
        assert!(matches!(admit(&ask(PRESSURE_MIN_PA.next_down(), 1.0e5, 1.0)), Err(ChamberReject::RarefiedFlow { .. })));
        assert!(matches!(admit(&ask(valve_rating_pa().next_up(), 1.0e5, 1.0)), Err(ChamberReject::PressureAboveValveRating { .. })));
        // A vent to the floor itself runs: the chamber stays between the two.
        let c = solve(&ask(PRESSURE_MIN_PA, ATMOSPHERE_PA, 1.0)).unwrap();
        assert!(c.final_pressure_pa >= PRESSURE_MIN_PA, "{}", c.final_pressure_pa);
    }

    /// Questions the red team found refused on rounding alone, the energy or
    /// mass ledger drifting past 1e-10 over 20 s (10 of 176), now close to a
    /// few parts in 1e16: the chamber's mass and energy are carried as their
    /// changes, summed with their rounding.
    #[test]
    fn ordinary_twenty_second_runs_close_their_ledgers_to_rounding() {
        for (reservoir, start) in [
            (1000.0, 269.0),
            (815.406_262_063_778_1, 269.0),
            (815.406_262_063_778_1, 407.703_131_031_889_03),
            (1075.918_767_221_925, 269.0),
            (3261.378_811_392_93, 269.0),
            (3261.378_811_392_93, 1630.689_405_696_465),
            (4303.350_162_306_124, 269.0),
            (4303.350_162_306_124, 2151.675_081_153_062),
            (5678.218_842_511_82, 2839.109_421_255_91),
            (13_044.530_372_481_746, 269.0),
            (13_044.530_372_481_746, 6522.265_186_240_873),
        ] {
            let c = solve(&ask(reservoir, start, 20.0)).unwrap_or_else(|e| panic!("{reservoir} {start}: {e}"));
            assert!(c.mass_ledger_residual < 1e-15 && c.energy_ledger_residual < 1e-14, "{reservoir} {start}: {:e} {:e}", c.mass_ledger_residual, c.energy_ledger_residual);
            assert!((c.final_pressure_pa / reservoir - 1.0).abs() < 1e-15, "{reservoir} {start}: {}", c.final_pressure_pa);
        }
    }

    /// Near equal pressures the subsonic flow falls as the square root of the
    /// difference, and the chamber reaches the reservoir's pressure in a
    /// finite time. Where the chamber's own rate outruns the step, the step
    /// is cut into substeps, and past sixteen of them, or a rate times the
    /// time left above one, one backward-Euler step ends it: it cannot pass
    /// the reservoir's pressure. Before, Runge-Kutta at 20 us settled about
    /// 1.36e-8 of the pressure short of the reservoir, and a charge of 1e-6
    /// Pa ended below its own start. Now every step's pressure lies between
    /// the two pressures given, to the rounding of the pressure moved, the
    /// chamber reaches the reservoir, and the entropy generated is not
    /// negative.
    #[test]
    fn near_equal_pressures_the_chamber_reaches_the_reservoir_and_stays_between_the_two() {
        for (reservoir, start, reaches) in [
            (700_000.05, 7.0e5, true),
            (7.0e5 - 0.05, 7.0e5, true),
            (300.000_001, 300.0, true),
            (300.0, 300.000_001, true),
            (269.0f64.next_up(), 269.0, true),
            (1000.0, 999.999_999, true),
            (7.0e5, ATMOSPHERE_PA, false),
            (ATMOSPHERE_PA, 1.0e6, false),
            (PRESSURE_MIN_PA, valve_rating_pa(), false),
            (valve_rating_pa(), PRESSURE_MIN_PA, false),
        ] {
            let c = solve(&ask(reservoir, start, 1.0)).unwrap();
            let (low, high) = (reservoir.min(start), reservoir.max(start));
            let rounding = 4.0 * f64::EPSILON * (high - low) + high * f64::EPSILON;
            for (k, p) in c.field.iter().step_by(2).enumerate() {
                assert!(*p >= low - rounding && *p <= high + rounding, "{reservoir} {start}: step {k} at {p}, outside [{low}, {high}]");
            }
            // Not negative, to the rounding of the chamber's entropy.
            assert!(c.entropy_generated_j_k >= -1.0e-18, "{reservoir} {start}: {}", c.entropy_generated_j_k);
            if reaches {
                assert_eq!(c.time_to_95_percent_s, vec![TIME_STEP_S], "{reservoir} {start}");
                assert!((c.final_pressure_pa - reservoir).abs() <= rounding, "{reservoir} {start}: {}", c.final_pressure_pa);
            }
        }
        // The charge of 0.05 Pa at 7 bar: before, 81 percent of the step and
        // no 95 percent time; now the reservoir's pressure.
        let c = solve(&ask(700_000.05, 7.0e5, 1.0)).unwrap();
        assert!(c.implicit_steps > 0 && c.explicit_steps > 0);
        assert!((c.final_pressure_pa - 700_000.05).abs() < 1e-9, "{}", c.final_pressure_pa);
    }

    /// The run ends at the asked time: the last step is the part of 20 us
    /// that is left, and the last sample is at the duration itself.
    #[test]
    fn the_run_ends_at_the_asked_time() {
        for (duration, steps) in [(1.0e-5, 1u64), (3.0e-5, 2), (2.0e-5, 1), (0.06, 3000), (1.0, 50_000), (20.0, 1_000_000), (2.225_073_858_507_201_4e-308, 1), (1.000_01e-3, 51)] {
            assert_eq!(step_count(duration), steps, "{duration}");
            if duration < 2.0 {
                let c = solve(&ask(valve_rating_pa(), PRESSURE_MIN_PA, duration)).unwrap();
                assert_eq!(c.sample_times_s.last().copied(), Some(duration), "{duration}");
                assert_eq!(c.field.len() as u64, 2 * (steps + 1), "{duration}");
            }
        }
        // Half a step charges half as far as a step, near enough: no longer
        // the same answer as a whole step.
        let half = solve(&ask(valve_rating_pa(), PRESSURE_MIN_PA, 1.0e-5)).unwrap();
        let whole = solve(&ask(valve_rating_pa(), PRESSURE_MIN_PA, 2.0e-5)).unwrap();
        assert!(half.final_pressure_pa < whole.final_pressure_pa);
    }

    /// The second law's ledger has its boundary on the reservoir's side of the
    /// valve, so the throttling is generated inside it whichever way the air
    /// goes. Derived from the chamber's balances (`m c_v dT/dt = q(h_in - c_v
    /// T) - Q`, `m ds = m c_v dT/T + m R dv/v`), the rate of generation is,
    /// for a vent, `q R ln(p/p_r) + Q (1/T_w - 1/T)`: the throttling and the
    /// heat across the film; the chamber's own air expands isentropically.
    /// For a charge it is `q [c_p (T_w/T - 1 - ln(T_w/T)) - R ln(p/p_r)] + Q
    /// (1/T_w - 1/T)`: mixing, throttling and the film. Before, a vent's air
    /// left with the chamber's own entropy, and the ledger held the film
    /// alone. The integral of each rate over the run, by the trapezoid rule
    /// on every 20 us step's pressure and temperature, meets the engine's
    /// entropy generated; and a vent's is many times the film's part.
    #[test]
    fn the_entropy_generated_holds_the_throttling_both_ways() {
        let film = FILM_COEFFICIENT_W_M2_K * chamber_wall_area_m2();
        let r = AIR_GAS_CONSTANT_J_KG_K;
        for (reservoir, start) in [(ATMOSPHERE_PA, 1.0e6), (7.0e5, ATMOSPHERE_PA), (2.0e5, 3.0e5), (3.0e5, 2.0e5)] {
            let c = solve(&ask(reservoir, start, 0.2)).unwrap();
            let rate = |p: f64, t: f64| -> (f64, f64) {
                let heat = film * (t - WALL_TEMPERATURE_K);
                let across = heat * (1.0 / WALL_TEMPERATURE_K - 1.0 / t);
                if reservoir >= p {
                    let q = iso6358_mass_flow(reservoir, WALL_TEMPERATURE_K, p).0;
                    let x = WALL_TEMPERATURE_K / t;
                    (q * (cp() * (x - 1.0 - x.ln()) - r * (p / reservoir).ln()) + across, across)
                } else {
                    let q = iso6358_mass_flow(p, t, reservoir).0;
                    (q * r * (p / reservoir).ln() + across, across)
                }
            };
            let (mut total, mut film_part) = (0.0, 0.0);
            let points: Vec<(f64, f64)> = c.field.chunks(2).map(|pair| rate(pair[0], pair[1])).collect();
            for pair in points.windows(2) {
                total += 0.5 * TIME_STEP_S * (pair[0].0 + pair[1].0);
                film_part += 0.5 * TIME_STEP_S * (pair[0].1 + pair[1].1);
            }
            let off = c.entropy_generated_j_k / total - 1.0;
            eprintln!("{reservoir} from {start}: S_gen {} J/K against {total} by the rates ({off:e}); the film's part {film_part}", c.entropy_generated_j_k);
            assert!(off.abs() < 1e-3, "{reservoir} {start}: {off}");
            if reservoir < start {
                assert!(c.entropy_generated_j_k > 5.0 * film_part, "{reservoir} {start}");
            }
        }
    }

    /// `ChamberBelowCriticalTemperature` is kept as a limit of the ideal gas,
    /// though no admitted question reaches it: the coldest admitted run, a
    /// vent from the valve's rating to the lowest pressure taken, cools the
    /// air to 157.7 K, 25 K above air's critical temperature.
    #[test]
    fn the_coldest_admitted_run_stays_above_the_critical_temperature() {
        let coldest = solve(&ask(PRESSURE_MIN_PA, valve_rating_pa(), 1.0)).unwrap();
        assert!((coldest.temperature_min_k - 157.7).abs() < 0.05, "{}", coldest.temperature_min_k);
        for reservoir in [1000.0, 1.0e4, ATMOSPHERE_PA, 5.0e5] {
            let c = solve(&ask(reservoir, valve_rating_pa(), 1.0)).unwrap();
            assert!(c.temperature_min_k >= coldest.temperature_min_k, "{reservoir}: {}", c.temperature_min_k);
        }
        assert!(coldest.temperature_min_k > AIR_CRITICAL_TEMPERATURE_K + 25.0);
    }

    /// A refusal's sentence prints each number as the shortest decimal that
    /// reads back to it, with an exponent past a millionth: no sentence of
    /// hundreds of digits.
    #[test]
    fn a_refusal_prints_short_numbers() {
        let sentence = ChamberReject::EnergyLedgerBroken { residual: -1.000_000_061_758_194_9e-10, time_s: 16.4038 }.to_string();
        assert_eq!(sentence, "at 16.4038 s the energy ledger was off by -1.000000061758195e-10 of its scale, above 1e-10");
        let tiny = ChamberReject::DurationOutOfRange { duration_s: -2.225_073_858_507_201_4e-308 }.to_string();
        assert_eq!(tiny, "duration -2.2250738585072014e-308 s is not above 0 s and at most 20 s");
        assert_eq!(shown(268.986_313_449_787_76), "268.98631344978776");
        assert_eq!(shown(1.0e21), "1e21");
        assert_eq!(shown(1.0e-6), "0.000001");
    }

    #[test]
    fn a_falsified_step_is_stopped_by_name() {
        let q = ask(7.0e5, ATMOSPHERE_PA, 0.5);
        assert!(run(&q, Tamper::None).is_ok());
        assert!(matches!(run(&q, Tamper::EulerEnergy), Err(ChamberReject::EnergyLedgerBroken { .. })));
        assert!(matches!(run(&q, Tamper::WrongInflowEnthalpy), Err(ChamberReject::EnergyLedgerBroken { .. })));
        assert!(matches!(run(&q, Tamper::FlowUphill), Err(ChamberReject::EntropyFell { .. } | ChamberReject::NonFiniteValue { .. })));
        let uphill_vent = run(&ask(ATMOSPHERE_PA, 1.0e6, 0.5), Tamper::FlowUphill);
        // A vent turned uphill brings air in with the chamber's enthalpy, not
        // the reservoir's: the energy ledger's inflow row stops it first.
        assert!(matches!(uphill_vent, Err(ChamberReject::EnergyLedgerBroken { .. } | ChamberReject::EntropyFell { .. })), "{uphill_vent:?}");
        assert!(matches!(run(&q, Tamper::LeakMass), Err(ChamberReject::MassLedgerBroken { .. })));
        // Near equal pressures, where the backward-Euler steps run: the
        // mass leak, and heat taken from the air and booked nowhere.
        let near = ask(1000.0, 269.0, 0.5);
        let honest = run(&near, Tamper::None).unwrap();
        assert!(honest.implicit_steps > 1000, "{}", honest.implicit_steps);
        assert!(matches!(run(&near, Tamper::LeakMass), Err(ChamberReject::MassLedgerBroken { .. })));
        assert!(matches!(run(&near, Tamper::ImplicitHeatUnbooked), Err(ChamberReject::EnergyLedgerBroken { .. })));
    }

    /// The slowest questions of the range, 20 s at the rating, the floor and
    /// between, each well under a second here.
    #[test]
    fn the_slowest_runs_are_quick() {
        for (reservoir, start) in [(7.0e5, ATMOSPHERE_PA), (valve_rating_pa(), PRESSURE_MIN_PA), (PRESSURE_MIN_PA, valve_rating_pa()), (3.0e5, 2.0e5), (valve_rating_pa(), ATMOSPHERE_PA)] {
            let started = std::time::Instant::now();
            let c = solve(&ask(reservoir, start, DURATION_MAX_S)).unwrap();
            eprintln!("20 s from {start} to {reservoir}: {:?}, {} steps and substeps of Runge-Kutta, {} backward-Euler", started.elapsed(), c.explicit_steps, c.implicit_steps);
            assert!(started.elapsed().as_secs_f64() < 5.0);
        }
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let q = ask(5.0e5, 2.0e5, 1.0);
        assert_eq!(solve(&q).unwrap(), solve(&q).unwrap());
    }
}
