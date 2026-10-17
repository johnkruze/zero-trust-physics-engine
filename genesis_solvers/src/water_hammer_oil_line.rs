//! Water hammer in a steel hydraulic line of mineral oil when the valve at
//! its end closes fast: the company's own specimen, a 4.0 m run of 12 mm by
//! 1.5 mm seamless steel tube (9 mm bore) from a supply held at one
//! pressure to a valve, filled with the mineral oil of the company's
//! `hydraulic_fluid` card (870 kg/m^3, bulk modulus 1.72 GPa), with free air
//! carried in the oil as the caller asks.
//!
//! What it computes. The steady flow the caller asks, and the transient
//! when the valve's open area falls linearly to zero over the closure time:
//! the method of characteristics along the line, forty sections of 0.1 m, with the
//! free air lumped at each node as an isothermal gas cavity and, where the
//! pressure would fall below the oil's vapour pressure, a vapour cavity
//! there: the line separates into liquid columns and they rejoin (the
//! discrete gas and vapour cavity models, Wylie 1984; Bergant, Simpson and
//! Tijsseling 2006). The core is `line_transient`, whose header gives the
//! equations, the friction and the ledgers.
//!
//! The wave speed. The oil's own compressibility and the tube's stretch
//! give the liquid wave speed,
//!
//! ```text
//! 1/(rho a_l^2) = 1/K + D psi / (E e)
//! psi = 2 (e/D) (1 + nu) + D (1 - nu^2) / (D + e)
//! ```
//!
//! the thick-walled form for a tube anchored against axial movement (Wylie
//! and Streeter 1993), 1365.9 m/s here. Free air lowers it. Wood's (1930)
//! relation for the oil with its air at the valve's steady pressure `p`,
//! with the wall,
//!
//! ```text
//! 1/(rho_m a_w^2) = (1 - alpha)/K + alpha/p + D psi/(E e),   rho_m = (1 - alpha) rho
//! ```
//!
//! is returned with the Joukowsky rise `rho_m a_w V_0` it gives, beside
//! `rho a_l V_0` of the oil with no air. The transient itself does not use
//! `a_w`: the gas sits at the nodes and is compressed by the pressure each
//! node reaches, so a rise that compresses the air stiffens it, and the
//! first rise the line computes is the chord of the gas law and not the
//! tangent at the steady pressure.
//!
//! The ledgers, held at every step (see `line_transient`): energy, from
//! the line at rest at the supply pressure, as kinetic energy, the oil's
//! compression, the wall's strain and the cavities' stored energy, against
//! friction, the work through the valve and the energy of each vapour
//! cavity's last closing step; volume, as the oil stored by compression and
//! the cavities, against what the supply put in and the valve let out.
//! Friction never gives energy to the flow, and a node's cavity never gives
//! energy to the waves; each is stopped by name if it does.
//!
//! The real limits, each by name. The tube's bore first yields at 51.41 MPa
//! over the atmosphere outside (Tresca, Lamé's thick-walled stresses, steel
//! E235 at 235 MPa): a supply up to it is taken, one above it is refused,
//! and a transient that passes it stops at that time and place
//! (`WallYield`). The line equations drop the flow's own convection; they
//! are held to a flow speed of a tenth of the local wave speed of oil and
//! air (`LineMachAboveAcoustic`), where the travel times they give are off
//! by up to ten percent. Free air above a void fraction of a quarter is no
//! longer bubbly flow (Taitel, Bornea and Dukler 1980): at the valve's
//! steady pressure the question is refused (`FreeAirAboveBubblyFlow`), and
//! at any node and step of the transient, where the air expands as the
//! pressure falls, the run stops (`LineVoidAboveBubblyFlow`). A cavity that
//! grows past the oil it is lumped from has left the grid
//! (`CavityLargerThanReach`). Cavitation is not a refusal: it is the vapour
//! cavity model. Choked flow at the valve does not arise in a liquid.
//!
//! Air too little to resolve. Free air under [`free_air_resolved_min`],
//! about 6.75e-29 of the oil's volume, is run as none: even at the oil's
//! vapour pressure, the lowest the line holds, its compliance is under a
//! part in 2^53 of the oil's, below the 64-bit resolution of every number
//! the line computes, and its gas at a node would be smaller than the
//! rounding of the volumes the waves carry.
//!
//! What is left out. The oil's modulus is the card's one value at every
//! pressure, and its viscosity one value (ISO VG 32 at 40 degC). Friction
//! is quasi-steady: Churchill's factor at the instantaneous velocity, which
//! is known to damp a water hammer more slowly than it is measured to damp.
//! The tube does not move (no fluid-structure interaction). Air does not
//! come out of solution or go back into it. The line is level.
//!
//! What was compared. This specimen's own comparison comes later. The wave
//! speed law and the transient of `line_transient` were compared with two
//! measured lines, each run on its own constants. The copper water rig of
//! the Szewalski Institute in Gdańsk (Lewandowski and Adamkowski 2024), in
//! the tests `gdansk_wave_speed_law` and `gdansk_free_oscillation_period`:
//! the source is <https://doi.org/10.3390/app14167173>, Retrieved
//! 2026-10-14. The liquid-nitrogen line of DLR Lampoldshausen (Klein,
//! Traudt and Deeken, Zenodo 15526459, CC BY 4.0), in the tests
//! `dlr_first_surge_by_joukowsky`, `dlr_first_surge_by_the_transient` and
//! `dlr_front_speed`, with nitrogen's properties from the NIST Chemistry
//! WebBook (Span et al. 2000), retrieved 2026-10-14. What they found is in
//! `GDANSK_FINDINGS` and `DLR_FINDINGS`.

use crate::normal::NormalOrZero;
use std::fmt;

use crate::line_transient::{self as line, Extensions, Liquid, LineStop, Pipe, Run, Tamper};

/// Length of the line, metres.
pub const LINE_LENGTH_M: f64 = 4.0;
/// Outer diameter of the tube, metres.
pub const TUBE_OUTER_DIAMETER_M: f64 = 0.012;
/// Wall thickness of the tube, metres.
pub const WALL_THICKNESS_M: f64 = 0.0015;
/// Bore of the tube, metres.
pub const BORE_M: f64 = TUBE_OUTER_DIAMETER_M - 2.0 * WALL_THICKNESS_M;
/// Young's modulus of the steel, pascals.
pub const STEEL_YOUNGS_MODULUS_PA: f64 = 210.0e9;
/// Poisson's ratio of the steel.
pub const STEEL_POISSON_RATIO: f64 = 0.3;
/// Yield strength of the steel, E235, pascals.
pub const STEEL_YIELD_PA: f64 = 235.0e6;
/// Roughness of the drawn bore, metres.
pub const BORE_ROUGHNESS_M: f64 = 1.5e-6;
/// Density of the oil, kg/m^3: the `hydraulic_fluid` card's.
pub const OIL_DENSITY_KG_M3: f64 = 870.0;
/// Bulk modulus of the oil, pascals: the `hydraulic_fluid` card's.
pub const OIL_BULK_MODULUS_PA: f64 = 1.72e9;
/// Kinematic viscosity of the oil, m^2/s: ISO VG 32 at 40 degC.
pub const OIL_KINEMATIC_VISCOSITY_M2_S: f64 = 32.0e-6;
/// Vapour pressure of the oil, absolute pascals: the `hydraulic_fluid` card's.
pub const OIL_VAPOUR_PRESSURE_PA: f64 = 10.0;
/// Sections of the line.
pub const SECTIONS: usize = 40;
/// Pressure outside the tube, absolute pascals.
pub const OUTSIDE_PRESSURE_PA: f64 = line::ATMOSPHERE_PA;
/// The largest Mach number the line equations stand for.
pub const MACH_LIMIT: f64 = 0.1;
/// Void fraction of free air above which the flow is no longer bubbly.
pub const BUBBLY_VOID_LIMIT: f64 = 0.25;
/// The longest run, seconds: a budget of compute, not a physical limit. One
/// second is 13,660 steps, about 0.3 s of computing on one core; by then
/// the wave of every run tried has fallen to a few millionths of its
/// energy.
pub const DURATION_MAX_S: f64 = 1.0;
/// With free air in the oil the closure must span this many steps of the
/// grid: a faster closure rings at the grid's own scale, where the air is
/// lumped at the nodes, and the grid does not resolve it.
pub const CLOSURE_STEPS_MIN_WITH_AIR: f64 = 10.0;
/// The most samples of each history.
pub const SAMPLES: usize = 500;
/// Absolute pressure at which the bore first yields: Tresca on Lamé's
/// stresses at the bore, `p - p_out = sigma_y (r_o^2 - r_i^2) / (2 r_o^2)`,
/// 51.41 MPa over the atmosphere.
pub const WALL_YIELD_PRESSURE_PA: f64 = 51_507_575.0;
/// The fastest steady flow the line equations hold: [`MACH_LIMIT`] of the
/// oil's wave speed with no air, m/s. With air it is lower, and asked of
/// Wood's wave speed at the valve.
pub const VELOCITY_MAX_M_S: f64 = 136.592_910_031_555_27;
/// The most free air a question can carry: the largest 64-bit number whose
/// void fraction at the bore's yield pressure ([`line::void_fraction`]) is at
/// most [`BUBBLY_VOID_LIMIT`], `0.25/0.75 * 51,507,575 / 101,325` to the last
/// bit. One number above, the void fraction is above it at every pressure
/// the line holds. At a lower pressure the limit is reached with less.
pub const FREE_AIR_MAX: f64 = 169.446_747_265_400_12;

/// Free air below this is run as none: at the oil's vapour pressure, the
/// lowest pressure the line holds, its compliance `alpha0 p_atm / p^2` is
/// under 2^-53 of the oil's and the wall's, `1/(rho a^2)`.
pub fn free_air_resolved_min() -> f64 {
    let a = liquid_wave_speed_m_s();
    f64::EPSILON / 2.0 * OIL_VAPOUR_PRESSURE_PA * OIL_VAPOUR_PRESSURE_PA / (line::ATMOSPHERE_PA * OIL_DENSITY_KG_M3 * a * a)
}

/// The public measurement the tests compare the method with.
pub const REFERENCE_NAME: &str = "M. Lewandowski and A. Adamkowski, Experimental Verification of 1D-Simulation Method of Water Hammer Induced in Two Series-Connected Pipes of Different Diameters: Determination of the Pressure Wave Speed, Applied Sciences 14 (2024) 7173, Tables 1 and 2: the measured wave speeds and periods of four runs on the copper rig of the Szewalski Institute, Gdansk";
pub const REFERENCE_URL: &str = "https://doi.org/10.3390/app14167173";
pub const REFERENCE_RETRIEVED: &str = "2026-10-14";

/// What the tests found on the Gdansk rig, in the words the description gives.
pub const GDANSK_FINDINGS: &str = "On the four runs of the Gdansk copper rig (Lewandowski and Adamkowski 2024, Tables 1 and 2), run on that rig's own constants. The wave speed law with no free gas (their equation 1 and constants) is high against the speed measured in pipe 2 from the front's passage between two transducers, by 8.6, 7.3, 6.8 and 6.1 percent from the lowest tank pressure to the highest. Free gas by Wood's relation, its amount fitted to the first run's deficit at that run's pressure (6.71e-5 of the volume at one atmosphere, the tank head read as absolute), leaves the other three runs 4.9, 5.9 and 5.6 percent high: the measured speed rises with pressure far more slowly than free gas would make it, so the deficit is not free gas alone. The transient, run on the rig with its two measured wave speeds, gives the period of the free oscillation at the valve 0.13 percent long, 0.11 short, 0.26 long and 0.28 long against the measured periods; their own equation 9 gives 0.09 long, 0.09 short, 0.25 long and 0.23 long. With the law's wave speeds in place of the measured ones the period is 7.7, 6.9, 6.1 and 5.5 percent short.";

/// What the tests found on the DLR liquid-nitrogen line, in the words the
/// description gives.
pub const DLR_FINDINGS: &str = "On the nine fluid-hammer tests of DLR's Fluid Transient Test Facility at Lampoldshausen (Klein, Traudt and Deeken, Zenodo 15526459, CC BY 4.0): 9.29 m of 19 mm stainless line, liquid nitrogen at 3.5 to 21.4 bar flowing at 4.9 to 13.1 m/s, stopped by a valve whose stroke takes about 9 ms; the first surge measured at PS408, 28.5 to 82.6 bar. The record's pressures are gauge, and PS408 sits at station S1, 0.60 m before the valve (the record's Figure 1). The mark a scout posted beforehand (within 5 percent in at least 8 of 9 tests, Joukowsky's rho a v0 with Korteweg's wave speed and nitrogen at the line thermometer's temperature) was missed: 6 of 9, all low, by 0.4 to 6.9 percent. After the fact, and so not a mark met: this solver's transient on that line, with Churchill's friction given a roughness of 4.6 um so that it meets one test's steady pressure drop, and the valve's area falling linearly over 9 ms, gives the first surge within 5 percent in all 9 tests (3.8 percent low to 1.0 percent high), because friction packs the line beyond Joukowsky's rise; that run read the transient 1.10 m before the valve and the record's pressures as absolute, both wrong, so it is not a comparison at the sensor (the line is served as its own solver, ln2_line_dlr_fttf, read at S1 with the pressures as gauge and scored on six held-back tests); with nitrogen's properties taken at the Coriolis meter's density instead of the thermometer's temperature (which puts the liquid 2.2 to 2.8 K colder, inside the thermometer's stated 3.7 K) it gives 8 of 9 (1.1 to 6.5 percent high). The roughness fitted to one test gives the other two traced tests 21 and 14 percent too little steady friction. The front's speed between the last two sensors, from the half-rise times of the 150 kHz traces in the three traced tests, is 803 to 822 m/s, 5.5 to 5.8 percent above Korteweg's at the thermometer's temperature and 2.1 to 3.3 percent above it at the meter's density.";

/// The time step of the grid, seconds: one section over the wave speed.
pub fn time_step_s() -> f64 {
    LINE_LENGTH_M / SECTIONS as f64 / liquid_wave_speed_m_s()
}

/// The Joukowsky rise of the oil with its free air when the rise itself
/// compresses the air: `dp = rho a_c V0`, with `a_c` the wave speed of the
/// chord of the gas law from the steady pressure `p` to `p + dp`,
/// `1/(rho a_c^2) = 1/(rho a_l^2) + alpha0 p_atm / (p (p + dp))`. Found by
/// bisection: the right side rises with `dp` more slowly than `dp`.
pub fn joukowsky_rise_chord_pa(free_air_at_atmosphere: f64, pressure_pa: f64, velocity_m_s: f64) -> f64 {
    let a_l = liquid_wave_speed_m_s();
    let rho = OIL_DENSITY_KG_M3;
    let rise = |dp: f64| {
        let compliance = 1.0 / (rho * a_l * a_l) + free_air_at_atmosphere * line::ATMOSPHERE_PA / (pressure_pa * (pressure_pa + dp));
        rho * velocity_m_s / (rho * compliance).sqrt()
    };
    let (mut lo, mut hi) = (0.0f64, rho * a_l * velocity_m_s);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if rise(mid) > mid { lo = mid } else { hi = mid }
    }
    0.5 * (lo + hi)
}

/// The tube's wall factor `psi`, thick-walled, anchored against axial movement.
pub fn wall_factor() -> f64 {
    let (d, e, nu) = (BORE_M, WALL_THICKNESS_M, STEEL_POISSON_RATIO);
    2.0 * (e / d) * (1.0 + nu) + d * (1.0 - nu * nu) / (d + e)
}

/// The wall's compliance per pascal, `D psi / (E e)`.
pub fn wall_compliance_per_pa() -> f64 {
    BORE_M * wall_factor() / (STEEL_YOUNGS_MODULUS_PA * WALL_THICKNESS_M)
}

/// Wave speed of the oil in the tube with no free air, m/s.
pub fn liquid_wave_speed_m_s() -> f64 {
    let compliance = 1.0 / OIL_BULK_MODULUS_PA + wall_compliance_per_pa();
    (1.0 / (OIL_DENSITY_KG_M3 * compliance)).sqrt()
}

/// Wood's wave speed of the oil with its free air at absolute pressure
/// `pressure_pa`, with the wall, and the void fraction there.
pub fn wood_wave_speed_m_s(free_air_at_atmosphere: f64, pressure_pa: f64) -> (f64, f64) {
    let alpha = line::void_fraction(free_air_at_atmosphere, pressure_pa);
    let compliance = (1.0 - alpha) / OIL_BULK_MODULUS_PA + alpha / pressure_pa + wall_compliance_per_pa();
    let density = (1.0 - alpha) * OIL_DENSITY_KG_M3;
    ((1.0 / (density * compliance)).sqrt(), alpha)
}

/// Absolute pressure at which the bore first yields, [`WALL_YIELD_PRESSURE_PA`].
pub fn wall_yield_pressure_pa() -> f64 {
    WALL_YIELD_PRESSURE_PA
}

/// Area of the bore, m^2.
pub fn bore_area_m2() -> f64 {
    0.25 * std::f64::consts::PI * BORE_M * BORE_M
}

/// One question.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HammerQuestion {
    /// Absolute pressure of the supply at the line's inlet, held.
    pub supply_pressure_pa: f64,
    /// Absolute pressure downstream of the valve, held.
    pub return_pressure_pa: f64,
    /// Steady velocity of the oil toward the valve before it moves.
    pub initial_velocity_m_s: f64,
    /// Time over which the valve's open area falls linearly to zero.
    pub closure_time_s: f64,
    /// Volume of free air per volume of oil at one standard atmosphere.
    pub free_air_fraction: f64,
    /// Time computed from the start of the closure.
    pub duration_s: f64,
}

/// What the line did.
#[derive(Clone, Debug, PartialEq)]
pub struct Hammer {
    pub liquid_wave_speed_m_s: f64,
    pub wood_wave_speed_m_s: f64,
    pub void_fraction_at_valve: f64,
    pub joukowsky_rise_liquid_pa: f64,
    pub joukowsky_rise_wood_pa: f64,
    pub joukowsky_rise_chord_pa: f64,
    pub round_trip_time_s: f64,
    pub time_step_s: f64,
    pub steady_valve_pressure_pa: f64,
    pub valve_pressure_first_rise_pa: f64,
    pub valve_pressure_max_pa: f64,
    pub valve_pressure_max_time_s: f64,
    pub valve_pressure_min_pa: f64,
    pub line_pressure_max_pa: f64,
    pub line_pressure_min_pa: f64,
    pub cavity_volume_max_m3: f64,
    pub largest_line_mach: f64,
    pub first_vapour_time_s: Vec<f64>,
    pub first_vapour_position_m: Vec<f64>,
    pub sample_times_s: Vec<f64>,
    pub valve_pressure_history_pa: Vec<f64>,
    pub midpoint_pressure_history_pa: Vec<f64>,
    pub valve_flow_history_m3_s: Vec<f64>,
    pub valve_cavity_history_m3: Vec<f64>,
    pub node_positions_m: Vec<f64>,
    pub pressure_max_along_line_pa: Vec<f64>,
    pub pressure_min_along_line_pa: Vec<f64>,
    pub kinetic_energy_history_j: Vec<f64>,
    pub oil_compression_energy_history_j: Vec<f64>,
    pub wall_strain_energy_history_j: Vec<f64>,
    pub cavity_energy_history_j: Vec<f64>,
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
    pub steps: u64,
    /// Every step's valve pressure, then the final pressure and velocity at
    /// every node.
    pub field: Vec<f64>,
}

/// Why no transient was returned.
#[derive(Clone, Debug, PartialEq)]
pub enum HammerReject {
    /// An input was NaN or infinite.
    InputNotFinite { field: &'static str },
    /// The supply pressure is above the pressure at which the tube's bore first yields, [`WALL_YIELD_PRESSURE_PA`], 51.41 MPa over the atmosphere outside.
    SupplyAboveWallYield { supply_pressure_pa: f64 },
    /// The return pressure is not above the oil's vapour pressure, [`OIL_VAPOUR_PRESSURE_PA`].
    ReturnNotAboveVapour { return_pressure_pa: f64 },
    /// The return pressure is not below the supply pressure: no flow runs toward the valve.
    ReturnNotBelowSupply { return_pressure_pa: f64 },
    /// The initial velocity is negative: the line runs from the supply to the valve.
    VelocityNegative { initial_velocity_m_s: f64 },
    /// The duration is not above zero or is above [`DURATION_MAX_S`].
    DurationOutOfRange { duration_s: f64 },
    /// The closure time is negative or longer than the duration.
    ClosureTimeOutOfRange { closure_time_s: f64 },
    /// The free air fraction is negative.
    FreeAirNegative { free_air_fraction: f64 },
    /// With free air in the oil the closure is faster than [`CLOSURE_STEPS_MIN_WITH_AIR`] steps of the grid, which it does not resolve.
    ClosureFasterThanGrid { closure_time_s: f64 },
    /// The free air at the valve's steady pressure is a void fraction above [`BUBBLY_VOID_LIMIT`]: past it the air and oil no longer flow as bubbles (Taitel, Bornea and Dukler 1980).
    FreeAirAboveBubblyFlow { void_fraction: f64 },
    /// Friction along the line takes the whole supply pressure before the valve: the steady pressure at the valve is not above the return pressure.
    SupplyCannotDriveFlow { valve_pressure_pa: f64 },
    /// The steady flow is faster than [`MACH_LIMIT`] of Wood's wave speed at the valve: the line equations, which drop the flow's own convection, do not hold there.
    InitialFlowMachAboveAcoustic { mach: f64 },
    /// The transient reached the pressure at which the tube's bore first yields: past it the wall is not elastic.
    WallYield { time_s: f64, position_m: f64, pressure_pa: f64 },
    /// The flow at a node reached [`MACH_LIMIT`] of the local wave speed of the oil and its air during the transient.
    LineMachAboveAcoustic { time_s: f64, position_m: f64, mach: f64 },
    /// The free air at a node expanded, as the pressure there fell, to a void fraction above [`BUBBLY_VOID_LIMIT`] during the transient: past it the air and oil no longer flow as bubbles.
    LineVoidAboveBubblyFlow { time_s: f64, position_m: f64, void_fraction: f64 },
    /// A node's cavity of gas and vapour grew past the oil it is lumped from: between two sections larger than its reach, at the valve longer than one section, so that the separated column has left the grid.
    CavityLargerThanReach { time_s: f64, position_m: f64, cavity_over_reach: f64 },
    /// The friction of a section did not settle in `MAX_FRICTION_PASSES` passes.
    FrictionNotSettled { time_s: f64, position_m: f64 },
    /// A node's cavity did not settle in `MAX_NODE_PASSES` passes of false position and `MAX_BISECTION_PASSES` of halving.
    NodeNotSettled { time_s: f64, position_m: f64 },
    /// A value of the transient was NaN or infinite.
    NonFiniteValue { time_s: f64 },
    /// The energy ledger did not close to `ENERGY_RESIDUAL_LIMIT` of the energy scale at a step.
    EnergyLedgerBroken { time_s: f64, residual: f64 },
    /// The volume ledger did not close to `VOLUME_RESIDUAL_LIMIT` of the volume scale at a step.
    VolumeLedgerBroken { time_s: f64, residual: f64 },
    /// A section's friction gave energy to the flow.
    FrictionGaveEnergy { time_s: f64, position_m: f64, work_j: f64 },
    /// A node's cavity gave energy to the waves.
    CavityGaveEnergy { time_s: f64, position_m: f64, work_j: f64 },
}

impl fmt::Display for HammerReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotFinite { field } => write!(f, "{field} is not a finite number"),
            Self::SupplyAboveWallYield { supply_pressure_pa } => write!(
                f,
                "supply pressure {supply_pressure_pa} Pa is above {} Pa, where the tube's bore first yields",
                wall_yield_pressure_pa()
            ),
            Self::ReturnNotAboveVapour { return_pressure_pa } => write!(
                f,
                "return pressure {return_pressure_pa} Pa is not above the oil's vapour pressure, {OIL_VAPOUR_PRESSURE_PA} Pa"
            ),
            Self::ReturnNotBelowSupply { return_pressure_pa } => write!(
                f,
                "return pressure {return_pressure_pa} Pa is not below the supply pressure: no flow runs toward the valve"
            ),
            Self::VelocityNegative { initial_velocity_m_s } => write!(
                f,
                "initial velocity {initial_velocity_m_s} m/s is negative: the line runs from the supply to the valve"
            ),
            Self::DurationOutOfRange { duration_s } => write!(
                f,
                "duration {duration_s} s is not above 0 s and at most {DURATION_MAX_S} s"
            ),
            Self::ClosureTimeOutOfRange { closure_time_s } => write!(
                f,
                "closure time {closure_time_s} s is negative or longer than the duration"
            ),
            Self::FreeAirNegative { free_air_fraction } => {
                write!(f, "free air fraction {free_air_fraction} is negative")
            }
            Self::ClosureFasterThanGrid { closure_time_s } => write!(
                f,
                "with free air in the oil a closure of {closure_time_s} s is faster than {CLOSURE_STEPS_MIN_WITH_AIR} steps of the grid, {} s, which it does not resolve",
                CLOSURE_STEPS_MIN_WITH_AIR * time_step_s()
            ),
            Self::FreeAirAboveBubblyFlow { void_fraction } => write!(
                f,
                "the free air is a void fraction of {void_fraction} at the valve's steady pressure, above {BUBBLY_VOID_LIMIT}: past it the air and oil no longer flow as bubbles"
            ),
            Self::SupplyCannotDriveFlow { valve_pressure_pa } => write!(
                f,
                "friction takes the supply pressure before the valve: the steady pressure there would be {valve_pressure_pa} Pa, not above the return pressure"
            ),
            Self::InitialFlowMachAboveAcoustic { mach } => write!(
                f,
                "the steady flow is Mach {mach} against Wood's wave speed at the valve, above {MACH_LIMIT}, where the line equations hold"
            ),
            Self::WallYield { time_s, position_m, pressure_pa } => write!(
                f,
                "at {time_s} s, {position_m} m from the supply, the pressure reached {pressure_pa} Pa, above the {} Pa at which the tube's bore first yields",
                wall_yield_pressure_pa()
            ),
            Self::LineVoidAboveBubblyFlow { time_s, position_m, void_fraction } => write!(
                f,
                "at {time_s} s, {position_m} m from the supply, the free air expanded to a void fraction of {void_fraction}, above {BUBBLY_VOID_LIMIT}: past it the air and oil no longer flow as bubbles"
            ),
            Self::LineMachAboveAcoustic { time_s, position_m, mach } => write!(
                f,
                "at {time_s} s, {position_m} m from the supply, the flow reached Mach {mach} against the local wave speed of the oil and its air, above {MACH_LIMIT}"
            ),
            Self::CavityLargerThanReach { time_s, position_m, cavity_over_reach } => write!(
                f,
                "at {time_s} s, {position_m} m from the supply, the cavity grew to {cavity_over_reach} times the oil it is lumped from"
            ),
            Self::FrictionNotSettled { time_s, position_m } => write!(
                f,
                "at {time_s} s the friction of the section at {position_m} m did not settle in {} passes",
                line::MAX_FRICTION_PASSES
            ),
            Self::NodeNotSettled { time_s, position_m } => write!(
                f,
                "at {time_s} s the cavity at {position_m} m did not settle in {} passes of false position and {} of halving",
                line::MAX_NODE_PASSES,
                line::MAX_BISECTION_PASSES
            ),
            Self::NonFiniteValue { time_s } => write!(f, "at {time_s} s a value of the transient was not finite"),
            Self::EnergyLedgerBroken { time_s, residual } => write!(
                f,
                "at {time_s} s the energy ledger was off by {residual} of its scale, above {}",
                line::ENERGY_RESIDUAL_LIMIT
            ),
            Self::VolumeLedgerBroken { time_s, residual } => write!(
                f,
                "at {time_s} s the volume ledger was off by {residual} of its scale, above {}",
                line::VOLUME_RESIDUAL_LIMIT
            ),
            Self::FrictionGaveEnergy { time_s, position_m, work_j } => write!(
                f,
                "at {time_s} s the friction of the section at {position_m} m gave {work_j} J to the flow"
            ),
            Self::CavityGaveEnergy { time_s, position_m, work_j } => write!(
                f,
                "at {time_s} s the cavity at {position_m} m gave {work_j} J to the waves"
            ),
        }
    }
}

fn pipe() -> Pipe {
    let wall = wall_compliance_per_pa();
    Pipe {
        length_m: LINE_LENGTH_M,
        sections: SECTIONS,
        diameter_m: BORE_M,
        roughness_m: BORE_ROUGHNESS_M,
        wave_speed_m_s: liquid_wave_speed_m_s(),
        wall_share: wall / (1.0 / OIL_BULK_MODULUS_PA + wall),
    }
}

fn oil() -> Liquid {
    Liquid {
        density_kg_m3: OIL_DENSITY_KG_M3,
        kinematic_viscosity_m2_s: OIL_KINEMATIC_VISCOSITY_M2_S,
        vapour_pressure_pa: OIL_VAPOUR_PRESSURE_PA,
    }
}

/// The steady pressure at the valve for a supply pressure and a velocity.
pub fn steady_valve_pressure_pa(supply_pressure_pa: f64, initial_velocity_m_s: f64) -> f64 {
    let p = pipe();
    let dx = LINE_LENGTH_M / SECTIONS as f64;
    let drop = dx * line::friction_per_volume(initial_velocity_m_s, &p, &oil());
    let mut gauge = 0.0;
    for _ in 0..SECTIONS {
        gauge -= drop;
    }
    supply_pressure_pa + gauge
}

/// Every limit the question shows, in the order they are asked.
pub fn admit(question: &HammerQuestion) -> Result<(), HammerReject> {
    for (field, value) in [
        ("supply_pressure_pa", question.supply_pressure_pa),
        ("return_pressure_pa", question.return_pressure_pa),
        ("initial_velocity_m_s", question.initial_velocity_m_s),
        ("closure_time_s", question.closure_time_s),
        ("free_air_fraction", question.free_air_fraction),
        ("duration_s", question.duration_s),
    ] {
        if !value.is_finite() {
            return Err(HammerReject::InputNotFinite { field });
        }
    }
    if question.supply_pressure_pa > WALL_YIELD_PRESSURE_PA {
        return Err(HammerReject::SupplyAboveWallYield { supply_pressure_pa: question.supply_pressure_pa });
    }
    if question.return_pressure_pa <= OIL_VAPOUR_PRESSURE_PA {
        return Err(HammerReject::ReturnNotAboveVapour { return_pressure_pa: question.return_pressure_pa });
    }
    if question.return_pressure_pa >= question.supply_pressure_pa {
        return Err(HammerReject::ReturnNotBelowSupply { return_pressure_pa: question.return_pressure_pa });
    }
    if question.initial_velocity_m_s < 0.0 {
        return Err(HammerReject::VelocityNegative { initial_velocity_m_s: question.initial_velocity_m_s });
    }
    if !(question.duration_s > 0.0 && question.duration_s <= DURATION_MAX_S) {
        return Err(HammerReject::DurationOutOfRange { duration_s: question.duration_s });
    }
    if !(question.closure_time_s >= 0.0 && question.closure_time_s <= question.duration_s) {
        return Err(HammerReject::ClosureTimeOutOfRange { closure_time_s: question.closure_time_s });
    }
    if question.free_air_fraction < 0.0 {
        return Err(HammerReject::FreeAirNegative { free_air_fraction: question.free_air_fraction });
    }
    if question.free_air_fraction >= free_air_resolved_min() && question.closure_time_s < CLOSURE_STEPS_MIN_WITH_AIR * time_step_s() {
        return Err(HammerReject::ClosureFasterThanGrid { closure_time_s: question.closure_time_s });
    }
    let valve = steady_valve_pressure_pa(question.supply_pressure_pa, question.initial_velocity_m_s);
    if question.initial_velocity_m_s > 0.0 && valve <= question.return_pressure_pa {
        return Err(HammerReject::SupplyCannotDriveFlow { valve_pressure_pa: valve });
    }
    let (wood, void_fraction) = wood_wave_speed_m_s(question.free_air_fraction, valve);
    if void_fraction > BUBBLY_VOID_LIMIT {
        return Err(HammerReject::FreeAirAboveBubblyFlow { void_fraction });
    }
    let mach = question.initial_velocity_m_s / wood;
    if mach > MACH_LIMIT || question.initial_velocity_m_s > VELOCITY_MAX_M_S {
        return Err(HammerReject::InitialFlowMachAboveAcoustic { mach });
    }
    Ok(())
}

fn rejected(stop: LineStop) -> HammerReject {
    match stop {
        LineStop::SupplyCannotDriveFlow { valve_pressure_pa } => HammerReject::SupplyCannotDriveFlow { valve_pressure_pa },
        LineStop::SteadyBelowVapour { pressure_pa } => HammerReject::SupplyCannotDriveFlow { valve_pressure_pa: pressure_pa },
        LineStop::WallYield { time_s, position_m, pressure_pa } => HammerReject::WallYield { time_s, position_m, pressure_pa },
        LineStop::LineMachAboveAcoustic { time_s, position_m, mach } => HammerReject::LineMachAboveAcoustic { time_s, position_m, mach },
        LineStop::CavityLargerThanReach { time_s, position_m, cavity_over_reach } => {
            HammerReject::CavityLargerThanReach { time_s, position_m, cavity_over_reach }
        }
        LineStop::VoidAboveBubblyFlow { time_s, position_m, void_fraction } => HammerReject::LineVoidAboveBubblyFlow { time_s, position_m, void_fraction },
        LineStop::FrictionNotSettled { time_s, position_m } => HammerReject::FrictionNotSettled { time_s, position_m },
        LineStop::NodeNotSettled { time_s, position_m } => HammerReject::NodeNotSettled { time_s, position_m },
        LineStop::NonFiniteValue { time_s } => HammerReject::NonFiniteValue { time_s },
        LineStop::EnergyLedgerBroken { time_s, residual } => HammerReject::EnergyLedgerBroken { time_s, residual },
        LineStop::VolumeLedgerBroken { time_s, residual } => HammerReject::VolumeLedgerBroken { time_s, residual },
        LineStop::FrictionGaveEnergy { time_s, position_m, work_j } => HammerReject::FrictionGaveEnergy { time_s, position_m, work_j },
        LineStop::CavityGaveEnergy { time_s, position_m, work_j } => HammerReject::CavityGaveEnergy { time_s, position_m, work_j },
    }
}

/// The run the core is asked for.
fn line_run(question: &HammerQuestion) -> Run {
    Run {
        pipes: vec![pipe()],
        liquid: oil(),
        supply_pressure_pa: question.supply_pressure_pa,
        return_pressure_pa: question.return_pressure_pa,
        initial_flow_m3_s: question.initial_velocity_m_s * bore_area_m2(),
        closure_time_s: question.closure_time_s,
        duration_s: question.duration_s,
        free_gas_fraction_at_atmosphere: if question.free_air_fraction < free_air_resolved_min() { 0.0 } else { question.free_air_fraction },
        wall_yield_pressure_pa: wall_yield_pressure_pa(),
        mach_limit: MACH_LIMIT,
        samples: SAMPLES,
        tamper: Tamper::None,
    }
}

/// Computes the steady flow and the transient of the closure.
///
/// Each value of the answer below the normal range is written as zero
/// with its sign ([`crate::normal`]).
pub fn solve(question: &HammerQuestion) -> Result<Hammer, HammerReject> {
    crate::normal::held(solve_as_computed(question))
}

/// The answer as the equations compute it, before a value below the
/// normal range is written as zero.
fn solve_as_computed(question: &HammerQuestion) -> Result<Hammer, HammerReject> {
    admit(question)?;
    solve_run(question, &line_run(question))
}

impl NormalOrZero for Hammer {
    fn normal_or_zero(&mut self) {
        self.liquid_wave_speed_m_s.normal_or_zero();
        self.wood_wave_speed_m_s.normal_or_zero();
        self.void_fraction_at_valve.normal_or_zero();
        self.joukowsky_rise_liquid_pa.normal_or_zero();
        self.joukowsky_rise_wood_pa.normal_or_zero();
        self.joukowsky_rise_chord_pa.normal_or_zero();
        self.round_trip_time_s.normal_or_zero();
        self.time_step_s.normal_or_zero();
        self.steady_valve_pressure_pa.normal_or_zero();
        self.valve_pressure_first_rise_pa.normal_or_zero();
        self.valve_pressure_max_pa.normal_or_zero();
        self.valve_pressure_max_time_s.normal_or_zero();
        self.valve_pressure_min_pa.normal_or_zero();
        self.line_pressure_max_pa.normal_or_zero();
        self.line_pressure_min_pa.normal_or_zero();
        self.cavity_volume_max_m3.normal_or_zero();
        self.largest_line_mach.normal_or_zero();
        self.first_vapour_time_s.normal_or_zero();
        self.first_vapour_position_m.normal_or_zero();
        self.sample_times_s.normal_or_zero();
        self.valve_pressure_history_pa.normal_or_zero();
        self.midpoint_pressure_history_pa.normal_or_zero();
        self.valve_flow_history_m3_s.normal_or_zero();
        self.valve_cavity_history_m3.normal_or_zero();
        self.node_positions_m.normal_or_zero();
        self.pressure_max_along_line_pa.normal_or_zero();
        self.pressure_min_along_line_pa.normal_or_zero();
        self.kinetic_energy_history_j.normal_or_zero();
        self.oil_compression_energy_history_j.normal_or_zero();
        self.wall_strain_energy_history_j.normal_or_zero();
        self.cavity_energy_history_j.normal_or_zero();
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

/// What the core carries beyond the line: the bubbly-flow limit held at
/// every node and step.
fn extensions() -> Extensions {
    Extensions { void_fraction_limit: Some(BUBBLY_VOID_LIMIT), ..Extensions::default() }
}

fn solve_run(question: &HammerQuestion, run: &Run) -> Result<Hammer, HammerReject> {
    solve_run_with(question, run, &extensions())
}

fn solve_run_with(question: &HammerQuestion, run: &Run, ext: &Extensions) -> Result<Hammer, HammerReject> {
    let done = line::run_with(run, ext).map_err(rejected)?;
    let a_l = liquid_wave_speed_m_s();
    let valve0 = done.steady_pressures_pa[SECTIONS];
    let (a_w, void_fraction) = wood_wave_speed_m_s(question.free_air_fraction, valve0);
    let v0 = question.initial_velocity_m_s;
    let window = question.closure_time_s + 2.0 * LINE_LENGTH_M / a_l;
    let first_rise = done
        .valve_pressure_every_step_pa
        .iter()
        .enumerate()
        .filter(|(n, _)| *n as f64 * done.time_step_s <= window)
        .map(|(_, p)| *p)
        .fold(valve0, f64::max)
        - valve0;
    let column = |pick: fn(&line::Ledger) -> f64| done.ledger_samples.iter().map(pick).collect::<Vec<f64>>();
    let mut field = done.valve_pressure_every_step_pa.clone();
    field.extend_from_slice(&done.final_pressures_pa);
    field.extend_from_slice(&done.final_velocities_m_s);
    Ok(Hammer {
        liquid_wave_speed_m_s: a_l,
        wood_wave_speed_m_s: a_w,
        void_fraction_at_valve: void_fraction,
        joukowsky_rise_liquid_pa: OIL_DENSITY_KG_M3 * a_l * v0,
        joukowsky_rise_wood_pa: (1.0 - void_fraction) * OIL_DENSITY_KG_M3 * a_w * v0,
        joukowsky_rise_chord_pa: joukowsky_rise_chord_pa(question.free_air_fraction, valve0, v0),
        round_trip_time_s: 2.0 * LINE_LENGTH_M / a_w,
        time_step_s: done.time_step_s,
        steady_valve_pressure_pa: valve0,
        valve_pressure_first_rise_pa: first_rise,
        valve_pressure_max_pa: done.valve_pressure_max_pa,
        valve_pressure_max_time_s: done.valve_pressure_max_time_s,
        valve_pressure_min_pa: done.valve_pressure_min_pa,
        line_pressure_max_pa: done.line_pressure_max_pa,
        line_pressure_min_pa: done.line_pressure_min_pa,
        cavity_volume_max_m3: done.cavity_volume_max_m3,
        largest_line_mach: done.largest_mach,
        first_vapour_time_s: done.first_vapour_time_s.clone(),
        first_vapour_position_m: done.first_vapour_position_m.clone(),
        sample_times_s: done.sample_times_s.clone(),
        valve_pressure_history_pa: done.valve_pressure_pa.clone(),
        midpoint_pressure_history_pa: done.midpoint_pressure_pa.clone(),
        valve_flow_history_m3_s: done.valve_flow_m3_s.clone(),
        valve_cavity_history_m3: done.valve_cavity_m3.clone(),
        node_positions_m: done.node_positions_m.clone(),
        pressure_max_along_line_pa: done.pressure_max_along_line_pa.clone(),
        pressure_min_along_line_pa: done.pressure_min_along_line_pa.clone(),
        kinetic_energy_history_j: column(|l| l.kinetic_j),
        oil_compression_energy_history_j: column(|l| l.liquid_compression_j),
        wall_strain_energy_history_j: column(|l| l.wall_strain_j),
        cavity_energy_history_j: column(|l| l.cavity_j),
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
        steps: done.steps,
        field,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(supply_mpa: f64, velocity: f64, closure: f64, air: f64, duration: f64) -> HammerQuestion {
        HammerQuestion {
            supply_pressure_pa: supply_mpa * 1.0e6,
            return_pressure_pa: 0.2e6,
            initial_velocity_m_s: velocity,
            closure_time_s: closure,
            free_air_fraction: air,
            duration_s: duration,
        }
    }

    #[test]
    fn the_specimen_numbers() {
        let a = liquid_wave_speed_m_s();
        assert!((a - 1365.9).abs() < 0.05, "{a}");
        let yield_gauge = wall_yield_pressure_pa() - OUTSIDE_PRESSURE_PA;
        assert!((yield_gauge - 51.406_25e6).abs() < 1.0, "{yield_gauge}");
        let (ro, ri) = (0.5 * TUBE_OUTER_DIAMETER_M, 0.5 * BORE_M);
        let tresca = OUTSIDE_PRESSURE_PA + STEEL_YIELD_PA * (ro * ro - ri * ri) / (2.0 * ro * ro);
        assert!((tresca / WALL_YIELD_PRESSURE_PA - 1.0).abs() < 1e-15, "{tresca}");
        assert!((time_step_s() - 7.3210e-5).abs() < 1e-9, "{}", time_step_s());
        let (wood, alpha) = wood_wave_speed_m_s(0.0, 10.0e6);
        assert_eq!(alpha, 0.0);
        assert!((wood - a).abs() < 1e-9);
        // One percent of free air at atmosphere, at 1 MPa: a tenth of a
        // percent of void, and the wave speed falls by more than a third.
        let (wood, alpha) = wood_wave_speed_m_s(0.01, 1.0e6);
        assert!((alpha - 1.0122e-3).abs() < 1e-6, "{alpha}");
        assert!(wood < 0.66 * a, "{wood}");
        // With no air the chord is the tangent.
        assert!((joukowsky_rise_chord_pa(0.0, 2.0e6, 1.0) / (OIL_DENSITY_KG_M3 * a) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn instant_closure_with_no_air_gives_the_joukowsky_rise() {
        let q = question(10.0, 2.0, 0.0, 0.0, 0.02);
        let h = solve(&q).unwrap();
        // The first step after the closure is the Joukowsky step, to rounding.
        let step = h.field[1] - h.field[0];
        assert!((step / h.joukowsky_rise_liquid_pa - 1.0).abs() < 1e-12, "{step}");
        // Over the round trip the pressure at the valve goes on rising by the
        // friction the moving column had: the line packs, toward the supply
        // pressure plus the Joukowsky rise.
        let friction_drop = q.supply_pressure_pa - h.steady_valve_pressure_pa;
        let packed = h.valve_pressure_first_rise_pa - h.joukowsky_rise_liquid_pa;
        assert!(packed > 0.9 * friction_drop && packed < 1.0 * friction_drop, "{packed} {friction_drop}");
        // The reflection comes back from the supply after 2L/a.
        let back = h.field.iter().position(|p| *p < h.steady_valve_pressure_pa).unwrap() as f64 * h.time_step_s;
        assert!((back / (2.0 * LINE_LENGTH_M / h.liquid_wave_speed_m_s) - 1.0).abs() < 0.02, "{back}");
        assert!(h.largest_energy_ledger_residual < 1e-12, "{}", h.largest_energy_ledger_residual);
        assert!(h.largest_volume_ledger_residual < 1e-12, "{}", h.largest_volume_ledger_residual);
        assert_eq!(h.cavity_loss_j, 0.0);
    }

    /// Free air lowers the rise, and the rise is the chord of the gas law,
    /// not Wood's tangent at the steady pressure.
    #[test]
    fn free_air_lowers_the_rise_to_the_chord_of_the_gas_law() {
        let clear = solve(&question(2.0, 1.0, 0.001, 0.0, 0.05)).unwrap();
        let aerated = solve(&question(2.0, 1.0, 0.001, 0.01, 0.05)).unwrap();
        let packed = clear.valve_pressure_first_rise_pa - clear.joukowsky_rise_liquid_pa;
        let plateau = aerated.valve_pressure_first_rise_pa - packed;
        assert!(plateau < 0.9 * clear.joukowsky_rise_liquid_pa);
        assert!((plateau / aerated.joukowsky_rise_chord_pa - 1.0).abs() < 0.01, "{plateau} {}", aerated.joukowsky_rise_chord_pa);
        assert!(aerated.joukowsky_rise_wood_pa < 0.95 * aerated.joukowsky_rise_chord_pa);
        assert!(aerated.largest_energy_ledger_residual < 1e-11);
        assert!(aerated.largest_volume_ledger_residual < 1e-11);
        // The backward step of the lumped air damps: here 12 percent of the
        // energy at the start over the first 50 ms, against friction's 40.
        let (loss, friction) = (aerated.cavity_loss_j / aerated.stored_energy_start_j, aerated.friction_loss_j / aerated.stored_energy_start_j);
        assert!(loss > 0.11 && loss < 0.13 && friction > 0.38 && friction < 0.42, "{loss} {friction}");
    }

    #[test]
    fn a_low_supply_separates_the_column_and_the_ledgers_close() {
        let h = solve(&question(1.0, 1.5, 0.0, 0.0, 0.2)).unwrap();
        // The column leaves the valve when the pressure there falls to the
        // vapour pressure, at the first return of the wave.
        assert_eq!(h.first_vapour_position_m[0], LINE_LENGTH_M);
        assert!(h.first_vapour_time_s[0] < 2.2 * LINE_LENGTH_M / h.liquid_wave_speed_m_s);
        assert_eq!(h.valve_pressure_min_pa, OIL_VAPOUR_PRESSURE_PA);
        // When the columns meet again the pressure passes the first rise.
        assert!(h.valve_pressure_max_pa > h.steady_valve_pressure_pa + h.valve_pressure_first_rise_pa);
        assert!(h.cavity_loss_j > 0.0);
        assert!(h.largest_energy_ledger_residual < 1e-11);
        assert!(h.largest_volume_ledger_residual < 1e-11);
    }

    #[test]
    fn the_question_is_refused_by_name_at_its_limits() {
        let base = question(10.0, 2.0, 0.001, 0.0, 0.1);
        let with = |change: fn(&mut HammerQuestion)| {
            let mut q = base;
            change(&mut q);
            admit(&q).unwrap_err()
        };
        assert!(matches!(with(|q| q.supply_pressure_pa = f64::NAN), HammerReject::InputNotFinite { field: "supply_pressure_pa" }));
        assert!(matches!(with(|q| q.supply_pressure_pa = 51.6e6), HammerReject::SupplyAboveWallYield { .. }));
        assert!((VELOCITY_MAX_M_S / (MACH_LIMIT * liquid_wave_speed_m_s()) - 1.0).abs() < 1e-15);
        assert!((FREE_AIR_MAX - 169.45).abs() < 0.01, "{FREE_AIR_MAX}");
        assert!(matches!(with(|q| q.return_pressure_pa = OIL_VAPOUR_PRESSURE_PA), HammerReject::ReturnNotAboveVapour { .. }));
        assert!(matches!(with(|q| q.return_pressure_pa = q.supply_pressure_pa), HammerReject::ReturnNotBelowSupply { .. }));
        assert!(matches!(with(|q| q.initial_velocity_m_s = -0.1), HammerReject::VelocityNegative { .. }));
        assert!(matches!(with(|q| q.duration_s = 0.0), HammerReject::DurationOutOfRange { .. }));
        assert!(matches!(with(|q| q.duration_s = 1.0000000000000002), HammerReject::DurationOutOfRange { .. }));
        assert!(matches!(with(|q| q.closure_time_s = 0.2), HammerReject::ClosureTimeOutOfRange { .. }));
        assert!(matches!(with(|q| q.free_air_fraction = -1e-6), HammerReject::FreeAirNegative { .. }));
        assert!(matches!(with(|q| { q.free_air_fraction = 1e-3; q.closure_time_s = 0.0 }), HammerReject::ClosureFasterThanGrid { .. }));
        assert!(matches!(with(|q| { q.supply_pressure_pa = 1.0e6; q.return_pressure_pa = 0.99e6 }), HammerReject::SupplyCannotDriveFlow { .. }));
        assert!(matches!(with(|q| { q.supply_pressure_pa = 0.3e6; q.free_air_fraction = 1.0; q.initial_velocity_m_s = 0.1 }), HammerReject::FreeAirAboveBubblyFlow { .. }));
        assert!(matches!(
            with(|q| {
                q.supply_pressure_pa = 0.25e6;
                q.return_pressure_pa = 0.11e6;
                q.free_air_fraction = 0.36;
                q.initial_velocity_m_s = 3.0
            }),
            HammerReject::InitialFlowMachAboveAcoustic { .. }
        ));
        // Inside every limit the question is admitted.
        admit(&base).unwrap();
        admit(&question(51.0, 0.0, 0.0, 0.0, 1.0)).unwrap();
        admit(&HammerQuestion { supply_pressure_pa: WALL_YIELD_PRESSURE_PA, ..question(51.0, 0.0, 0.0, 0.0, 1.0) }).unwrap();
    }

    /// A wave that would burst the tube stops at that time and place.
    #[test]
    fn a_rise_to_the_wall_yield_stops_by_name() {
        let q = question(45.0, 6.0, 0.0, 0.0, 0.05);
        admit(&q).unwrap();
        match solve(&q) {
            Err(HammerReject::WallYield { time_s, position_m, pressure_pa }) => {
                assert_eq!(position_m, LINE_LENGTH_M);
                assert!(time_s < 2.0 * LINE_LENGTH_M / liquid_wave_speed_m_s());
                assert!(pressure_pa >= wall_yield_pressure_pa());
            }
            other => panic!("{other:?}"),
        }
    }

    /// A column that separates from the valve and runs back past the next
    /// node has left the grid.
    #[test]
    fn a_column_that_leaves_the_grid_stops_by_name() {
        let q = HammerQuestion { return_pressure_pa: 0.02e6, ..question(0.3, 5.5, 0.0, 0.0, 0.2) };
        admit(&q).unwrap();
        match solve(&q) {
            Err(HammerReject::CavityLargerThanReach { cavity_over_reach, .. }) => assert!(cavity_over_reach > 1.0),
            other => panic!("{other:?}"),
        }
    }

    /// Air at a low pressure carries sound so slowly that the flow reaches
    /// a tenth of it.
    #[test]
    fn flow_near_the_speed_of_sound_of_aerated_oil_stops_by_name() {
        let q = question(0.4, 2.5, 0.001, 2.0e-4, 0.2);
        admit(&q).unwrap();
        match solve(&q) {
            Err(HammerReject::LineMachAboveAcoustic { mach, .. }) => assert!(mach > MACH_LIMIT),
            other => panic!("{other:?}"),
        }
    }

    fn tampered(question: &HammerQuestion, tamper: Tamper) -> Result<Hammer, HammerReject> {
        let mut run = line_run(question);
        run.tamper = tamper;
        solve_run(question, &run)
    }

    #[test]
    fn a_falsified_step_is_stopped_by_name() {
        let clear = question(10.0, 2.0, 0.001, 0.0, 0.05);
        let aerated = question(2.0, 1.0, 0.001, 0.01, 0.05);
        assert!(matches!(tampered(&clear, Tamper::None), Ok(_)));
        assert!(matches!(tampered(&clear, Tamper::ValveWorkDropped), Err(HammerReject::EnergyLedgerBroken { .. })));
        assert!(matches!(tampered(&clear, Tamper::WaveGain), Err(HammerReject::EnergyLedgerBroken { .. } | HammerReject::VolumeLedgerBroken { .. })));
        assert!(matches!(tampered(&clear, Tamper::FrictionPushes), Err(HammerReject::FrictionGaveEnergy { .. })));
        assert!(matches!(tampered(&aerated, Tamper::CavityForward), Err(HammerReject::CavityGaveEnergy { .. })));
    }

    #[test]
    fn the_same_question_gives_the_same_bits() {
        let q = question(5.0, 1.5, 0.002, 0.002, 0.05);
        let (a, b) = (solve(&q).unwrap(), solve(&q).unwrap());
        assert_eq!(a, b);
    }

    /// A line at rest stays at rest to the bit, with no air and with the
    /// most air, at the yield pressure itself, for the longest run: every
    /// node is its own step's root there.
    #[test]
    fn a_line_at_rest_stays_at_rest_to_the_bit() {
        for (supply, air) in [(WALL_YIELD_PRESSURE_PA, 0.0), (WALL_YIELD_PRESSURE_PA, FREE_AIR_MAX), (10.0e6, 0.01), (11.0, 1.0e-5)] {
            let q = HammerQuestion { supply_pressure_pa: supply, return_pressure_pa: 10.5, initial_velocity_m_s: 0.0, closure_time_s: 0.001, free_air_fraction: air, duration_s: DURATION_MAX_S };
            let h = solve(&q).unwrap_or_else(|e| panic!("{supply} {air}: {e}"));
            assert!(h.field[..=h.steps as usize].iter().all(|p| *p == supply), "{supply} {air}");
            assert_eq!((h.line_pressure_max_pa, h.line_pressure_min_pa), (supply, supply));
            assert_eq!((h.cavity_loss_j, h.friction_loss_j, h.stored_energy_end_j), (0.0, 0.0, 0.0));
            assert_eq!((h.largest_energy_ledger_residual, h.largest_volume_ledger_residual), (0.0, 0.0));
        }
    }

    /// Creeping flows, to the smallest normal velocity, and a supply a pascal
    /// over the vapour pressure with a little air: Poiseuille's friction where
    /// Churchill's laminar term passes the 64-bit range, and both ledgers
    /// closing on scales at least the rounding of the terms they add.
    #[test]
    fn creeping_flow_near_the_vapour_pressure_holds_its_ledgers() {
        for q in [
            HammerQuestion { supply_pressure_pa: 11.0, return_pressure_pa: 10.2, initial_velocity_m_s: 1.0e-6, closure_time_s: 0.001, free_air_fraction: 1.0e-5, duration_s: 0.2 },
            HammerQuestion { initial_velocity_m_s: 1.0e-30, ..question(10.0, 2.0, 0.001, 0.0, 0.05) },
            HammerQuestion { initial_velocity_m_s: f64::MIN_POSITIVE, ..question(10.0, 2.0, 0.001, 0.01, 0.05) },
            HammerQuestion { supply_pressure_pa: 20.0, return_pressure_pa: 10.5, initial_velocity_m_s: 1.0e-4, closure_time_s: 0.001, free_air_fraction: 1.0e-6, duration_s: DURATION_MAX_S },
        ] {
            let h = solve(&q).unwrap_or_else(|e| panic!("{q:?}: {e}"));
            assert!(h.largest_energy_ledger_residual < 0.5 * line::ENERGY_RESIDUAL_LIMIT && h.largest_volume_ledger_residual < 0.5 * line::VOLUME_RESIDUAL_LIMIT, "{q:?}");
            assert!(h.steady_valve_pressure_pa.is_finite() && h.friction_loss_j >= 0.0, "{q:?}");
        }
        // The laminar limit is the formula's own to rounding where both are
        // computed, and finite where the formula's term is not.
        for re in [1.0e-24, 2.0e-25] {
            assert!((line::churchill_friction_factor(re, 1.0e-4) * re / 64.0 - 1.0).abs() < 1e-14, "{re}");
        }
        for re in [1.0e-26, 1.0e-300, f64::MIN_POSITIVE] {
            assert_eq!(line::churchill_friction_factor(re, 1.0e-4), 64.0 / re);
        }
    }

    /// The most free air is exact: the largest number with a void fraction
    /// of at most a quarter at the yield pressure, taken there at rest; one
    /// number above it is refused at every pressure, and so is the largest
    /// 64-bit number, whose void fraction is one.
    #[test]
    fn the_most_free_air_is_the_largest_accepted() {
        assert!(line::void_fraction(FREE_AIR_MAX, WALL_YIELD_PRESSURE_PA) <= BUBBLY_VOID_LIMIT);
        assert!(line::void_fraction(FREE_AIR_MAX.next_up(), WALL_YIELD_PRESSURE_PA) > BUBBLY_VOID_LIMIT);
        assert!((FREE_AIR_MAX / (BUBBLY_VOID_LIMIT / (1.0 - BUBBLY_VOID_LIMIT) * WALL_YIELD_PRESSURE_PA / line::ATMOSPHERE_PA) - 1.0).abs() < 1e-15);
        let rest = HammerQuestion { supply_pressure_pa: WALL_YIELD_PRESSURE_PA, return_pressure_pa: 1.0e5, initial_velocity_m_s: 0.0, closure_time_s: 0.001, free_air_fraction: FREE_AIR_MAX, duration_s: 0.05 };
        admit(&rest).unwrap();
        for supply in [WALL_YIELD_PRESSURE_PA, 10.0e6, 1.0e3] {
            let q = HammerQuestion { supply_pressure_pa: supply, return_pressure_pa: 11.0, free_air_fraction: FREE_AIR_MAX.next_up(), ..rest };
            assert!(matches!(admit(&q), Err(HammerReject::FreeAirAboveBubblyFlow { void_fraction }) if void_fraction > BUBBLY_VOID_LIMIT), "{supply}");
        }
        assert!(matches!(admit(&HammerQuestion { free_air_fraction: f64::MAX, ..rest }), Err(HammerReject::FreeAirAboveBubblyFlow { void_fraction }) if void_fraction == 1.0));
    }

    /// Air under the resolved least is run as none, to the bit; just above it
    /// the line computes it, and it moves the answer by no more than its
    /// compliance says.
    #[test]
    fn air_too_small_to_resolve_is_run_as_none() {
        let least = free_air_resolved_min();
        assert!((least / 6.750_221_9e-29 - 1.0).abs() < 1e-6, "{least}");
        let clear = question(10.0, 2.0, 0.001, 0.0, 0.05);
        let none = solve(&clear).unwrap();
        for air in [f64::MIN_POSITIVE, 1.0e-100, least.next_down()] {
            let h = solve(&HammerQuestion { free_air_fraction: air, ..clear }).unwrap();
            assert_eq!(h.field, none.field, "{air}");
        }
        // A closure faster than the grid's ten steps is no ringing of air the
        // line does not carry.
        admit(&HammerQuestion { free_air_fraction: least.next_down(), closure_time_s: 0.0, ..clear }).unwrap();
        for air in [least, 1.0e-20, 1.0e-16, 1.0e-14] {
            let h = solve(&HammerQuestion { free_air_fraction: air, ..clear }).unwrap_or_else(|e| panic!("{air}: {e}"));
            assert!((h.valve_pressure_max_pa / none.valve_pressure_max_pa - 1.0).abs() < 1e-12, "{air}");
        }
    }

    /// Air that expands at a node, as the pressure there falls, past the
    /// void limit stops the run there by name: the aerated line whose valve
    /// falls from 1 MPa to 0.47 MPa, with the limit cut below the void its air
    /// reaches.
    #[test]
    fn air_expanding_past_the_void_limit_stops_by_name() {
        let q = question(1.0, 1.0, 0.001, 0.01, 0.2);
        let h = solve(&q).unwrap();
        let most = line::void_fraction(q.free_air_fraction, h.line_pressure_min_pa);
        assert!(most > 1.5e-3 && most < BUBBLY_VOID_LIMIT, "{most}");
        let cut = Extensions { void_fraction_limit: Some(1.5e-3), ..Extensions::default() };
        match solve_run_with(&q, &line_run(&q), &cut) {
            Err(reject @ HammerReject::LineVoidAboveBubblyFlow { void_fraction, .. }) => {
                assert!(void_fraction > 1.5e-3, "{void_fraction}");
                assert!(reject.to_string().contains("expanded to a void fraction of"), "{reject}");
            }
            other => panic!("{other:?}"),
        }
    }

    /// Where the ledgers' scales are their floors, a supply eleven pascals
    /// absolute with a little air and a creeping flow, a falsified step is
    /// still stopped by name.
    #[test]
    fn a_falsified_step_is_stopped_by_name_near_the_vapour_pressure() {
        let q = HammerQuestion { supply_pressure_pa: 20.0, return_pressure_pa: 10.5, initial_velocity_m_s: 1.0e-4, closure_time_s: 0.001, free_air_fraction: 1.0e-6, duration_s: 0.2 };
        assert!(matches!(tampered(&q, Tamper::None), Ok(_)));
        assert!(matches!(tampered(&q, Tamper::ValveWorkDropped), Err(HammerReject::EnergyLedgerBroken { .. })), "{:?}", tampered(&q, Tamper::ValveWorkDropped).err());
        assert!(matches!(tampered(&q, Tamper::WaveGain), Err(HammerReject::EnergyLedgerBroken { .. } | HammerReject::VolumeLedgerBroken { .. })), "{:?}", tampered(&q, Tamper::WaveGain).err());
        assert!(matches!(tampered(&q, Tamper::FrictionPushes), Err(HammerReject::FrictionGaveEnergy { .. })));
        assert!(matches!(tampered(&q, Tamper::CavityForward), Err(HammerReject::CavityGaveEnergy { .. })), "{:?}", tampered(&q, Tamper::CavityForward).err());
    }

    /// The yield's sentence names pressures in pascals as they are, the one
    /// reached above the yield.
    #[test]
    fn the_yield_sentence_names_the_pressure_reached_above_the_yield() {
        match solve(&question(45.0, 6.0, 0.0, 0.0, 0.05)) {
            Err(reject @ HammerReject::WallYield { pressure_pa, .. }) => {
                assert!(pressure_pa > WALL_YIELD_PRESSURE_PA);
                assert!(reject.to_string().contains(&format!("reached {pressure_pa} Pa, above the 51507575 Pa")), "{reject}");
            }
            other => panic!("{other:?}"),
        }
    }
}

#[cfg(test)]
mod gdansk {
    //! The rig of Lewandowski and Adamkowski (2024), Szewalski Institute of
    //! Fluid-Flow Machinery, Gdansk: two copper pipes in series from a tank
    //! held at pressure, pipe 1 of 20 mm bore and 49.3 m, then pipe 2 of
    //! 16 mm bore and 58.9 m, both with a 1 mm wall, closed by a ball valve
    //! in about 2 ms at the end of pipe 2. Water at 23 degC. Their Table 1
    //! gives, for four runs, the tank's pressure head, the flow, the wave
    //! speed measured in pipe 2 from the front's passage between two
    //! transducers 29.392 m apart and the one they derived from it for
    //! pipe 1; their Table 2 the period of the free oscillation at the
    //! valve, from the last 20 periods of each run. Copper E = 120 GPa,
    //! Poisson 0.35, and K = 2.21 GPa are theirs.
    use super::*;

    /// (tank head m, flow L/h, a2 measured m/s, a1 derived m/s, period s)
    pub(super) const RUNS: [(f64, f64, f64, f64, f64); 4] = [
        (26.4, 113.7, 1222.0, 1192.0, 0.3120),
        (51.6, 240.8, 1236.0, 1206.0, 0.3090),
        (83.3, 361.1, 1242.0, 1211.0, 0.3065),
        (119.0, 530.1, 1251.0, 1221.0, 0.3043),
    ];
    const WATER_DENSITY: f64 = 997.54;
    const WATER_BULK_MODULUS: f64 = 2.21e9;
    const WATER_VISCOSITY: f64 = 0.9325e-6;
    const WATER_VAPOUR: f64 = 2810.0;
    const COPPER_E: f64 = 120.0e9;
    const COPPER_POISSON: f64 = 0.35;
    const G: f64 = 9.81;

    /// Their equation (1): the thin-walled law, anchored throughout.
    fn korteweg(d: f64, e: f64) -> f64 {
        let c1 = 1.0 - COPPER_POISSON * COPPER_POISSON;
        (WATER_BULK_MODULUS / WATER_DENSITY / (1.0 + WATER_BULK_MODULUS * d * c1 / (COPPER_E * e))).sqrt()
    }

    /// The same law with free gas, isothermal, Wood's relation at one
    /// pressure: `1/(rho a^2) = 1/(rho a_K^2) + alpha0 p_atm / p^2`.
    fn with_free_gas(a_k: f64, alpha0: f64, p: f64) -> f64 {
        let compliance = 1.0 / (WATER_DENSITY * a_k * a_k) + alpha0 * line::ATMOSPHERE_PA / (p * p);
        1.0 / (WATER_DENSITY * compliance).sqrt()
    }

    /// The tank head read as absolute: at 119.0 m it is 1.165 MPa, under the
    /// 1.2 MPa absolute the authors give as the tank's most; read as gauge
    /// it would be 1.27 MPa.
    fn tank_pressure(head_m: f64) -> f64 {
        head_m * WATER_DENSITY * G
    }

    #[test]
    fn gdansk_wave_speed_law() {
        let a2 = korteweg(0.016, 0.001);
        let a1 = korteweg(0.020, 0.001);
        assert!((a2 - 1326.8).abs() < 0.1, "{a2}");
        assert!((a1 / a2 - 0.975).abs() < 0.001, "{}", a1 / a2);
        // The law with no free gas is high in every run.
        let high: Vec<f64> = RUNS.iter().map(|run| a2 / run.2 - 1.0).collect();
        eprintln!("korteweg over measured pipe 2: {high:?}");
        assert!(high.iter().all(|h| *h > 0.060 && *h < 0.086), "{high:?}");
        // Free gas fitted to the first run's deficit at its pressure leaves
        // too little gas at the higher pressures: the law then misses high.
        let p1 = tank_pressure(RUNS[0].0);
        let alpha0 = (1.0 / (WATER_DENSITY * RUNS[0].2 * RUNS[0].2) - 1.0 / (WATER_DENSITY * a2 * a2)) * p1 * p1 / line::ATMOSPHERE_PA;
        assert!((alpha0 - 6.71e-5).abs() < 0.02e-5, "{alpha0}");
        let misses: Vec<f64> = RUNS.iter().map(|run| with_free_gas(a2, alpha0, tank_pressure(run.0)) / run.2 - 1.0).collect();
        eprintln!("free gas fitted at run 1, over measured: {misses:?}");
        assert!(misses[0].abs() < 1e-12);
        assert!(misses[1..].iter().all(|m| *m > 0.048 && *m < 0.060), "{misses:?}");
    }

    /// The period of the free oscillation at the valve after the closure,
    /// from the upward crossings of the supply pressure, over the 20 periods
    /// from the third crossing.
    fn period(run: (f64, f64, f64, f64, f64), a1: f64, a2: f64) -> (f64, f64) {
        // A step that fits both pipes: pipe 1 in n1 sections, pipe 2 in the
        // whole number of sections nearest its wave speed's.
        let mut best = (0usize, 0usize, f64::INFINITY);
        for n1 in 30..=120usize {
            let dt = 49.3 / n1 as f64 / a1;
            let n2 = (58.9 / (a2 * dt)).round() as usize;
            let moved = (58.9 / (n2 as f64 * dt) / a2 - 1.0).abs();
            if moved < best.2 {
                best = (n1, n2, moved);
            }
        }
        let pipe = |length_m: f64, sections: usize, d: f64, a: f64| Pipe {
            length_m,
            sections,
            diameter_m: d,
            roughness_m: 1.5e-6,
            wave_speed_m_s: a,
            wall_share: 0.2,
        };
        let spec = Run {
            pipes: vec![pipe(49.3, best.0, 0.020, a1), pipe(58.9, best.1, 0.016, a2)],
            liquid: Liquid { density_kg_m3: WATER_DENSITY, kinematic_viscosity_m2_s: WATER_VISCOSITY, vapour_pressure_pa: WATER_VAPOUR },
            supply_pressure_pa: tank_pressure(run.0),
            return_pressure_pa: line::ATMOSPHERE_PA,
            initial_flow_m3_s: run.1 / 3.6e6,
            closure_time_s: 0.002,
            duration_s: 22.0 * run.4,
            free_gas_fraction_at_atmosphere: 0.0,
            wall_yield_pressure_pa: f64::INFINITY,
            mach_limit: 0.1,
            samples: 1,
            tamper: Tamper::None,
        };
        let done = line::run(&spec).expect("the rig runs");
        assert!(done.largest_energy_residual < 1e-11 && done.largest_volume_residual < 1e-11);
        // The fundamental of the valve pressure over the developed phase,
        // from the second period on: the peak of its Hann-windowed Fourier
        // magnitude, found by golden section.
        let p = &done.valve_pressure_every_step_pa;
        let dt = done.time_step_s;
        let from = (run.4 / dt) as usize;
        let window: Vec<f64> = p[from..].iter().map(|v| v - spec.supply_pressure_pa).collect();
        let count = window.len() as f64;
        let magnitude = |frequency: f64| {
            let (mut re, mut im) = (0.0, 0.0);
            for (k, v) in window.iter().enumerate() {
                let hann = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * k as f64 / (count - 1.0)).cos();
                let phase = 2.0 * std::f64::consts::PI * frequency * k as f64 * dt;
                re += hann * v * phase.cos();
                im += hann * v * phase.sin();
            }
            re * re + im * im
        };
        let (mut lo, mut hi) = (0.7 / run.4, 1.4 / run.4);
        // A coarse scan first, so the golden section starts on the peak.
        let mut best_f = lo;
        let mut best_m = 0.0;
        for k in 0..=140 {
            let f = lo + (hi - lo) * k as f64 / 140.0;
            let m = magnitude(f);
            if m > best_m {
                best_m = m;
                best_f = f;
            }
        }
        let step = (hi - lo) / 140.0;
        lo = best_f - step;
        hi = best_f + step;
        let golden = 0.5 * (5f64.sqrt() - 1.0);
        for _ in 0..60 {
            let a = hi - golden * (hi - lo);
            let b = lo + golden * (hi - lo);
            if magnitude(a) > magnitude(b) { hi = b } else { lo = a }
        }
        (2.0 / (lo + hi), best.2)
    }

    /// Their equation (9) for two pipes in series between a tank and a
    /// closed valve: `(a1 A2)/(a2 A1) tan(w L1/a1) tan(w L2/a2) = 1`, its
    /// lowest root, as a period.
    fn equation_nine(a1: f64, a2: f64) -> f64 {
        let ratio = (a1 / a2) * (0.016f64 * 0.016) / (0.020 * 0.020);
        let h = |w: f64| ratio * (w * 49.3 / a1).tan() * (w * 58.9 / a2).tan() - 1.0;
        // Below the first pole of either tangent the product rises from zero.
        let pole = 0.5 * std::f64::consts::PI * (a1 / 49.3).min(a2 / 58.9);
        let (mut lo, mut hi) = (1e-9, pole * (1.0 - 1e-12));
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if h(mid) < 0.0 { lo = mid } else { hi = mid }
        }
        2.0 * std::f64::consts::PI / (0.5 * (lo + hi))
    }

    #[test]
    fn gdansk_free_oscillation_period() {
        let mut measured_speeds = Vec::new();
        let mut law_speeds = Vec::new();
        for run in RUNS {
            let (t, moved) = period(run, run.3, run.2);
            assert!(moved < 2e-3, "{moved}");
            measured_speeds.push(t / run.4 - 1.0);
            let (t, _) = period(run, korteweg(0.020, 0.001), korteweg(0.016, 0.001));
            law_speeds.push(t / run.4 - 1.0);
        }
        let nine: Vec<f64> = RUNS.iter().map(|run| equation_nine(run.3, run.2) / run.4 - 1.0).collect();
        eprintln!("equation nine from the measured wave speeds over the measured period: {nine:?}");
        eprintln!("period from the measured wave speeds over the measured period: {measured_speeds:?}");
        eprintln!("period from the wave speed law over the measured period: {law_speeds:?}");
        assert!(measured_speeds.iter().all(|e| e.abs() < 0.004), "{measured_speeds:?}");
        assert!(law_speeds.iter().all(|e| *e < -0.05 && *e > -0.09), "{law_speeds:?}");
    }
}

#[cfg(test)]
mod dlr {
    //! The Fluid Transient Test Facility of DLR Lampoldshausen (Klein,
    //! Traudt and Deeken, "Liquid Nitrogen Fluid Hammer: PS-LN2-Set1",
    //! Zenodo 15526459, CC BY 4.0): 9.29 m of stainless steel 1.4541 line,
    //! 19 mm bore and 1.5 mm wall, from a held tank to a fast valve, liquid
    //! nitrogen at 3.5 to 21.4 bar and 4.9 to 13.1 m/s. Each row: the test,
    //! the overview table's v0 (m/s), P0 and dP at PS408 (bar, gauge: the
    //! record's pressures are gauge; PS408's steady and peak values are the
    //! table's). This module reads the transient at 88.2 percent of the
    //! length from the tank, 1.10 m before the valve, where this record
    //! first placed PS408; the record's Figure 1 puts it at S1, 0.60 m
    //! before the valve, as ln2_line_dlr_fttf reads it. The line thermometer TI407
    //! over the last second before closure (K), NIST's density, sound speed
    //! and viscosity of nitrogen there (Span et al. 2000 through the NIST
    //! Chemistry WebBook, retrieved 2026-10-14) and the vapour pressure at
    //! that temperature (Pa); then the Coriolis meter's density over the
    //! same second, the temperature at which NIST's nitrogen has that
    //! density at P0, and the sound speed, viscosity and vapour pressure
    //! there.
    use super::*;

    pub(super) const TESTS: [(&str, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64); 9] = [
        ("20200806_13", 4.86, 3.49, 28.46, 85.38, 768.655, 769.661, 1.199990e-04, 237401.8, 782.19, 82.607, 799.13, 1.323114e-04, 180465.0),
        ("20200806_5", 6.35, 5.48, 37.87, 86.05, 766.003, 764.65, 1.177270e-04, 252941.1, 778.03, 83.599, 790.71, 1.281788e-04, 199499.0),
        ("20200803_8", 7.55, 7.31, 45.3, 86.05, 766.632, 766.667, 1.181470e-04, 252941.1, 778.7, 83.580, 792.78, 1.287007e-04, 199105.1),
        ("20200803_11", 8.99, 10.43, 55.51, 86.02, 767.841, 770.386, 1.189800e-04, 252218.0, 779.38, 83.641, 795.30, 1.291588e-04, 200337.5),
        ("20200806_15", 10.02, 12.51, 61.03, 85.98, 768.737, 773.046, 1.196150e-04, 251270.5, 779.19, 83.816, 795.57, 1.288552e-04, 203861.6),
        ("20200805_9", 10.98, 15.83, 65.7, 87.05, 764.616, 765.422, 1.161390e-04, 277551.4, 777.87, 84.309, 793.82, 1.274293e-04, 214037.8),
        ("20200803_15", 11.61, 17.55, 69.68, 88.84, 756.349, 748.528, 1.098960e-04, 325990.3, 768.87, 86.298, 775.10, 1.194769e-04, 258907.0),
        ("20200805_10", 12.14, 20.06, 74.65, 86.17, 770.314, 779.054, 1.205530e-04, 255833.3, 782.35, 83.635, 804.86, 1.314375e-04, 200217.3),
        ("20200806_16", 13.08, 21.39, 82.63, 85.69, 773.045, 785.337, 1.228100e-04, 244520.4, 783.28, 83.521, 807.30, 1.322693e-04, 197924.7),
    ];
    /// Steady pressures at S1 and S3 before closure, bar, in the three
    /// tests whose traces are on disk: (test, S1 PS402, S3 PS408).
    const STEADY: [(&str, f64, f64); 3] = [("20200806_13", 4.065, 3.482), ("20200803_11", 12.869, 10.441), ("20200806_16", 26.073, 21.389)];
    const LENGTH: f64 = 9.29;
    const BORE: f64 = 0.019;
    const WALL: f64 = 0.0015;
    /// Young's modulus of 1.4541 at room temperature, Pa; a handbook value.
    const STEEL_E: f64 = 200.0e9;
    const S1: f64 = 0.0646;
    const S3: f64 = 0.882;
    /// The valve's stroke from 0.9 to 0.1 of open took 7.2, 6.5 and 8.0 ms
    /// in the three traced tests: an area falling linearly over 9 ms.
    const CLOSURE: f64 = 0.009;

    /// The scout's Korteweg form: `a = [rho (1/K + D/(E e))]^(-1/2)`, `K = rho c^2`.
    fn korteweg(rho: f64, c: f64) -> f64 {
        1.0 / (1.0 / (c * c) + rho * BORE / (STEEL_E * WALL)).sqrt()
    }

    #[test]
    fn dlr_first_surge_by_joukowsky() {
        let mut at_thermometer = Vec::new();
        let mut at_meter = Vec::new();
        for t in TESTS {
            let (v0, dp) = (t.1, t.3 * 1e5);
            at_thermometer.push(t.5 * korteweg(t.5, t.6) * v0 / dp - 1.0);
            at_meter.push(t.9 * korteweg(t.9, t.11) * v0 / dp - 1.0);
        }
        let within = |e: &[f64]| e.iter().filter(|x| x.abs() <= 0.05).count();
        eprintln!("Joukowsky at TI407: {at_thermometer:?}, within 5 percent {}", within(&at_thermometer));
        eprintln!("Joukowsky at the Coriolis density: {at_meter:?}, within 5 percent {}", within(&at_meter));
        assert_eq!(within(&at_thermometer), 6);
        assert_eq!(within(&at_meter), 8);
    }

    /// The wall roughness that gives test 20200806_13's measured friction
    /// between S1 and S3 under Churchill's law, at the Coriolis density.
    fn fitted_roughness() -> f64 {
        let t = TESTS[0];
        let (rho, nu, v0) = (t.9, t.12 / t.9, t.1);
        let gradient = (STEADY[0].1 - STEADY[0].2) * 1e5 / ((S3 - S1) * LENGTH);
        let f_measured = gradient * BORE / (0.5 * rho * v0 * v0);
        let reynolds = v0 * BORE / nu;
        let (mut lo, mut hi) = (0.0f64, 1.0e-3f64);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if line::churchill_friction_factor(reynolds, mid / BORE) < f_measured { lo = mid } else { hi = mid }
        }
        0.5 * (lo + hi)
    }

    fn surge(t: (&str, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64), at_meter: bool, roughness: f64) -> f64 {
        let (rho, c, mu, vapour) = if at_meter { (t.9, t.11, t.12, t.13) } else { (t.5, t.6, t.7, t.8) };
        let a = korteweg(rho, c);
        let pipe = Pipe {
            length_m: LENGTH,
            sections: 170,
            diameter_m: BORE,
            roughness_m: roughness,
            wave_speed_m_s: a,
            wall_share: rho * a * a * BORE / (STEEL_E * WALL),
        };
        let liquid = Liquid { density_kg_m3: rho, kinematic_viscosity_m2_s: mu / rho, vapour_pressure_pa: vapour };
        let area = 0.25 * std::f64::consts::PI * BORE * BORE;
        // The supply that leaves the table's P0 at S3 under the steady flow.
        let drop_to_s3 = S3 * LENGTH * line::friction_per_volume(t.1, &pipe, &liquid);
        let spec = Run {
            pipes: vec![pipe],
            liquid,
            supply_pressure_pa: t.2 * 1e5 + drop_to_s3,
            return_pressure_pa: 0.6e5,
            initial_flow_m3_s: t.1 * area,
            closure_time_s: CLOSURE,
            duration_s: 0.022,
            free_gas_fraction_at_atmosphere: 0.0,
            wall_yield_pressure_pa: f64::INFINITY,
            mach_limit: 0.1,
            samples: 1,
            tamper: Tamper::None,
        };
        let done = line::run(&spec).expect("the line runs");
        assert!(done.largest_energy_residual < 1e-11 && done.largest_volume_residual < 1e-11);
        // S3 is node 150 of 170.
        assert!((done.node_positions_m[150] / LENGTH - S3).abs() < 5e-4);
        let steady = done.steady_pressures_pa[150];
        assert!((steady - t.2 * 1e5).abs() < 0.002 * drop_to_s3, "{steady}");
        (done.pressure_max_along_line_pa[150] - steady) / (t.3 * 1e5) - 1.0
    }

    #[test]
    fn dlr_first_surge_by_the_transient() {
        let roughness = fitted_roughness();
        // The fitted friction against the other two traced tests' gradients.
        for (k, steady) in STEADY.iter().enumerate().skip(1) {
            let t = TESTS.iter().find(|t| t.0 == steady.0).unwrap();
            let pipe = Pipe { length_m: LENGTH, sections: 170, diameter_m: BORE, roughness_m: roughness, wave_speed_m_s: 800.0, wall_share: 0.0 };
            let liquid = Liquid { density_kg_m3: t.9, kinematic_viscosity_m2_s: t.12 / t.9, vapour_pressure_pa: t.13 };
            let predicted = line::friction_per_volume(t.1, &pipe, &liquid) * (S3 - S1) * LENGTH / 1e5;
            eprintln!("friction S1 to S3, test {}: {predicted:.3} bar predicted, {:.3} measured ({k})", steady.0, steady.1 - steady.2);
        }
        let mut at_thermometer = Vec::new();
        let mut at_meter = Vec::new();
        for t in TESTS {
            at_thermometer.push(surge(t, false, roughness));
            at_meter.push(surge(t, true, roughness));
        }
        let within = |e: &[f64]| e.iter().filter(|x| x.abs() <= 0.05).count();
        eprintln!("roughness {roughness}");
        eprintln!("transient at TI407: {at_thermometer:?}, within 5 percent {}", within(&at_thermometer));
        eprintln!("transient at the Coriolis density: {at_meter:?}, within 5 percent {}", within(&at_meter));
        assert_eq!(within(&at_thermometer), 9);
        assert!(at_thermometer.iter().all(|e| *e > -0.039 && *e < 0.010), "{at_thermometer:?}");
        assert_eq!(within(&at_meter), 8);
        assert!(at_meter.iter().all(|e| *e > 0.011 && *e < 0.066), "{at_meter:?}");
    }
}

#[cfg(test)]
mod dlr_front {
    //! The front's speed between S3 and S2 (3.7996 m apart) in the three
    //! traced tests: the times at which the dynamic sensors PD409 (S3) and
    //! PD406 (S2) cross half their first rise, read from the 150 kHz traces
    //! by the script in `evidence/research/hydraulics_robotics/analysis/`.
    use super::dlr::TESTS;

    const HALF_RISE_GAP_S: [(&str, f64); 3] = [("20200806_13", 4.726_102e-3), ("20200803_11", 4.730_097e-3), ("20200806_16", 4.624_801e-3)];
    const S3_TO_S2_M: f64 = (0.882 - 0.473) * 9.29;

    fn korteweg(rho: f64, c: f64) -> f64 {
        1.0 / (1.0 / (c * c) + rho * 0.019 / (200.0e9 * 0.0015)).sqrt()
    }

    #[test]
    fn dlr_front_speed() {
        for (test, gap) in HALF_RISE_GAP_S {
            let t = TESTS.iter().find(|t| t.0 == test).unwrap();
            let measured = S3_TO_S2_M / gap;
            let thermometer = korteweg(t.5, t.6) / measured - 1.0;
            let meter = korteweg(t.9, t.11) / measured - 1.0;
            eprintln!("{test}: front {measured:.1} m/s; Korteweg at TI407 {thermometer:+.4}, at the Coriolis density {meter:+.4}");
            assert!(thermometer < -0.05 && thermometer > -0.06, "{thermometer}");
            assert!(meter < -0.02 && meter > -0.035, "{meter}");
        }
    }
}
