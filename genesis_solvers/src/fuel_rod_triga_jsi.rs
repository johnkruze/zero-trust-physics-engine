//! The instrumented fuel element 6753 of the JSI TRIGA Mark II reactor during
//! a pulse: radial heat conduction through the element, heated by the core's
//! energy release, and the reading of its fuel thermocouple.
//!
//! The element is a standard TRIGA element of the 56-element core of cores
//! 215, 217 and 231 of the Jožef Stefan Institute, Ljubljana (12 wt% uranium,
//! 20 % enriched, U-ZrH1.6), at position B1. From the axis out: a zirconium
//! rod, the fuel meat, a gas gap, a stainless-steel (SS-304) clad, and the pool
//! water. The thermocouple's tip is in the fuel meat.
//!
//! ```text
//! heat source in the meat   q''' = F_B P_core(t) / (N V_meat)                 W per m3
//! conduction                rho c(T) dT/dt = (1/r) d/dr (k r dT/dr) + q'''   in each layer
//! gap                       q'' = h_gap (T_fuel,surface - T_clad,inner)        at the meat's surface
//! water                     q'' = q_conv(T_s - T_0) + q_boil(T_s - T_sat)      at the clad's surface
//! thermocouple              dT_tc/dt = (T_fuel(r_tc) - T_tc) / tau
//! ```
//!
//! `P_core` is the core's thermal power, given as the energy the core has
//! released since the history's start at each time of the history (MW s); over
//! each interval of the history the power is the energy released in it over
//! its length, so the energy deposited in the element is the energy given,
//! however coarse the history. `F_B` is [`ELEMENT_POWER_FACTOR`], 1.387: the
//! hottest element's power over the average element's in a core loaded
//! uniformly with 12 wt% fuel, in ring B (I. Vavtar, master thesis, University
//! of Ljubljana, 2019, Table 4, from Snoj and Ravnik's calculation). Element
//! 6753 stands at B1, in ring B, and the factor is taken as its own. `N` is 56,
//! the elements of the core. The heat source is spread evenly over the meat:
//! the thermocouple's plane is taken to carry the element's average power
//! density. The axial peak at mid-height and the radial dip of the power
//! towards the axis are not resolved, and no source here gives either for this
//! element; this is the assumption through which the reactor_triga_jsi card's
//! fuel temperature was compared with this thermocouple. Heat released in the
//! water, the clad or the zirconium rod is not modelled: the whole share of
//! the element goes into its meat.
//!
//! From the sources, not fitted: the radii of the rod, the meat, the gap and
//! the clad (Švajger et al., Nucl. Eng. Technol. 56 (2024) 1165, Table 1); the
//! meat's length, 15 in (M. Ravnik, Description of TRIGA Reactor, Table 4); the
//! densities of the meat, the rod and the clad (the article, Table 1); the
//! meat's specific heat c(T) = 333 + 0.678 (T - 274.15) J per kg K (the
//! article, eq. 7.2, as in the reactor_triga_jsi card); the meat's
//! conductivity, 13 Btu per h ft degF (University of Texas NETL TRIGA safety
//! analysis report, 1991, Table 4-1); the specific heat and conductivity of
//! zirconium and of SS-304 at 300 K (Incropera and DeWitt, Fundamentals of
//! Heat and Mass Transfer, Table A.1); the thermocouple's radius, 0.3 in from
//! the axis (General Atomics' instrumented element, NETL report section
//! 4.4.5.1; the institute's description says only "in the center of the fuel
//! element"); natural convection from a vertical surface (Churchill and Chu
//! 1975) with water at 300 K, and nucleate boiling (Rohsenow 1952, C_sf 0.013
//! and n 1 for water on stainless steel) with saturated water at 385 K, 1.52
//! bar, about 5 m of water over the element (Incropera and DeWitt, Tables A.6
//! and 10.1). The pool is at the start temperature throughout, as the card
//! takes it.
//!
//! Two constants are fitted, to 39 records of the institute's pulse database
//! (cores 215, 217 and 231, 1.4 to 3.0 printed dollars), on the thermocouple's
//! maximum rise, its time after the power peak and its rise 10 s after it: the gap conductance [`GAP_CONDUCTANCE_W_PER_M2_K`] and the
//! thermocouple's time constant [`THERMOCOUPLE_TIME_CONSTANT_S`]. The first also
//! carries every error of the water side and of the assumption about the power
//! density at the thermocouple; the second every lag of the recording chain.
//! Neither is the element's own value of that quantity.
//!
//! **The energy account.** Finite volumes on the radius (4 cells in the rod, 30
//! in the meat, 4 in the clad) and a surface node of no heat capacity; each
//! step is backward Euler in the enthalpy, solved by Newton's method on the
//! tridiagonal system until every cell's equation and the surface's hold to
//! 1e-13 of the step's energy scale or to 1e-8 J. Every step the energy stored in the
//! element is compared with the energy deposited less the energy that crossed
//! the clad's surface into the water, each computed from the step's own
//! values:
//!
//! ```text
//! step_energy_residual = |ΔE_stored - (Q_in - Q_out) Δt| / max(|ΔE_stored|, Q_in Δt, Q_out Δt, Σ|ΔE_cell|, 100 J)
//! run_energy_residual  = |E_stored(t) - E_stored(0) - Σ (Q_in - Q_out) Δt| / max(Σ Q_in Δt, 100 J)
//! ```
//!
//! A step whose account is open by more than [`STEP_ENERGY_TOLERANCE`], or a run
//! by more than [`RUN_ENERGY_TOLERANCE`], is refused by name. The account shows
//! that the equations were solved and the energy given was spent, not that the
//! model describes the element.
//!
//! The record of the comparison is in
//! `evidence/research/reactor_triga_jsi_element_6753/`. The reference of the
//! tests is the article's adiabatic relation (eq. 7.6, E = m ∫ c dT) and the
//! closed form of steady conduction through the layers,
//! <https://doi.org/10.1016/j.net.2023.11.021>, Retrieved 2026-10-09.

use std::fmt;

use genesis_kernel::math;


/// Radius of the zirconium rod, m. Švajger et al. 2024, Table 1 (0.3175 cm).
pub const ROD_RADIUS_M: f64 = 0.3175e-2;
/// Outer radius of the fuel meat, m. Same table (1.82245 cm).
pub const MEAT_RADIUS_M: f64 = 1.82245e-2;
/// Inner radius of the clad: the outer radius of the gas gap, m. Same table
/// (1.82626 cm).
pub const GAP_RADIUS_M: f64 = 1.82626e-2;
/// Outer radius of the clad, m. Same table (1.87706 cm).
pub const CLAD_RADIUS_M: f64 = 1.87706e-2;
/// Length of the fuel meat, m: 15 in. M. Ravnik, Description of TRIGA Reactor,
/// Table 4.
pub const MEAT_LENGTH_M: f64 = 15.0 * 0.0254;
/// Radius of the thermocouple's tip, m: 0.3 in from the axis, as in General
/// Atomics' instrumented element (NETL TRIGA safety analysis report, 1991,
/// section 4.4.5.1).
pub const THERMOCOUPLE_RADIUS_M: f64 = 0.3 * 0.0254;

/// Density of the U-ZrH1.6 meat, kg per m3. Švajger et al. 2024, Table 1.
pub const MEAT_DENSITY_KG_PER_M3: f64 = 6045.0;
/// Specific heat of the meat at 274.15 K, J per kg K. Švajger et al. 2024, eq. 7.2.
pub const MEAT_HEAT_CAPACITY_BASE_J_PER_KG_K: f64 = 333.0;
/// Its rise with temperature, J per kg K2. Same equation.
pub const MEAT_HEAT_CAPACITY_SLOPE_J_PER_KG_K2: f64 = 0.678;
/// The temperature the meat's heat capacity is counted from, K.
pub const MEAT_HEAT_CAPACITY_REFERENCE_K: f64 = 274.15;
/// Conductivity of the meat, W per m K: 13 Btu per h ft degF, NETL TRIGA safety
/// analysis report, 1991, Table 4-1 (delta-phase U-ZrH, 93 to 650 degC).
pub const MEAT_CONDUCTIVITY_W_PER_M_K: f64 = 13.0 * 1.730735;
/// Density of the zirconium rod, kg per m3. Švajger et al. 2024, Table 1.
pub const ROD_DENSITY_KG_PER_M3: f64 = 6490.0;
/// Specific heat of zirconium at 300 K, J per kg K. Incropera and DeWitt, Table A.1.
pub const ROD_HEAT_CAPACITY_J_PER_KG_K: f64 = 278.0;
/// Conductivity of zirconium at 300 K, W per m K. Same table.
pub const ROD_CONDUCTIVITY_W_PER_M_K: f64 = 22.7;
/// Density of the SS-304 clad, kg per m3. Švajger et al. 2024, Table 1.
pub const CLAD_DENSITY_KG_PER_M3: f64 = 7888.9;
/// Specific heat of SS-304 at 300 K, J per kg K. Incropera and DeWitt, Table A.1.
pub const CLAD_HEAT_CAPACITY_J_PER_KG_K: f64 = 477.0;
/// Conductivity of SS-304 at 300 K, W per m K. Same table.
pub const CLAD_CONDUCTIVITY_W_PER_M_K: f64 = 14.9;

/// The element's power over the average element's: the hot-rod factor of a
/// core loaded uniformly with 12 wt% fuel, 1.387 with 0.003, in ring B (I.
/// Vavtar, master thesis, University of Ljubljana, 2019, Table 4). Element 6753
/// stands at B1, in ring B. The same number is the reactor_triga_jsi card's
/// `HOT_ROD_FACTOR`.
pub const ELEMENT_POWER_FACTOR: f64 = 1.387;
/// Fuel elements in the core: 56. Core maps 215, 217 and 231.
pub const CORE_ELEMENTS: f64 = 56.0;

/// Water at 300 K, the pool: conductivity, W per m K. Incropera and DeWitt, Table A.6.
pub const WATER_CONDUCTIVITY_W_PER_M_K: f64 = 0.613;
/// Water at 300 K: kinematic viscosity, m2 per s (855e-6 Pa s over 997 kg per m3).
pub const WATER_KINEMATIC_VISCOSITY_M2_PER_S: f64 = 855.0e-6 / 997.0;
/// Water at 300 K: thermal diffusivity, m2 per s (0.613 over 997 times 4179).
pub const WATER_DIFFUSIVITY_M2_PER_S: f64 = 0.613 / (997.0 * 4179.0);
/// Water at 300 K: Prandtl number.
pub const WATER_PRANDTL: f64 = 5.83;
/// Water at 300 K: expansion coefficient, per K.
pub const WATER_EXPANSION_PER_K: f64 = 276.1e-6;
/// Saturation temperature at the element, K: 385 K, 1.52 bar, about 5 m of
/// water over the element. Incropera and DeWitt, Table A.6. An assumption of
/// the depth, not a measured value.
pub const SATURATION_TEMPERATURE_K: f64 = 385.0;
/// Saturated water at 385 K: liquid viscosity, Pa s. Incropera and DeWitt, Table A.6.
pub const BOIL_LIQUID_VISCOSITY_PA_S: f64 = 248.0e-6;
/// Saturated water at 385 K: heat of vaporization, J per kg. Same table.
pub const BOIL_LATENT_HEAT_J_PER_KG: f64 = 2225.0e3;
/// Saturated water at 385 K: liquid density, kg per m3 (1 over 1.053e-3). Same table.
pub const BOIL_LIQUID_DENSITY_KG_PER_M3: f64 = 1.0 / 1.053e-3;
/// Saturated water at 385 K: vapour density, kg per m3 (1 over 1.15). Same table.
pub const BOIL_VAPOUR_DENSITY_KG_PER_M3: f64 = 1.0 / 1.15;
/// Saturated water at 385 K: surface tension, N per m. Same table.
pub const BOIL_SURFACE_TENSION_N_PER_M: f64 = 55.2e-3;
/// Saturated water at 385 K: liquid specific heat, J per kg K. Same table.
pub const BOIL_LIQUID_HEAT_CAPACITY_J_PER_KG_K: f64 = 4232.0;
/// Saturated water at 385 K: liquid Prandtl number. Same table.
pub const BOIL_LIQUID_PRANDTL: f64 = 1.53;
/// Rohsenow's surface constant for water on stainless steel. Incropera and
/// DeWitt, Table 10.1.
pub const ROHSENOW_SURFACE_CONSTANT: f64 = 0.013;
/// Standard gravity, m per s2.
pub const GRAVITY_M_PER_S2: f64 = 9.80665;

/// Conductance of the gap between the meat and the clad, W per m2 K, on the
/// meat's surface. Fitted to the opened records; it carries every error of the
/// water side and of the power density taken at the thermocouple.
pub const GAP_CONDUCTANCE_W_PER_M2_K: f64 = 1780.0;
/// Time constant of the thermocouple and its recording chain, s. Fitted to the
/// opened records.
pub const THERMOCOUPLE_TIME_CONSTANT_S: f64 = 1.03;

/// Cells in the zirconium rod, the meat and the clad.
pub const ROD_CELLS: usize = 4;
pub const MEAT_CELLS: usize = 30;
pub const CLAD_CELLS: usize = 4;
/// The longest step, s. An interval of the history is taken in equal steps no
/// longer than this.
pub const STEP_MAX_S: f64 = 2.0e-3;
/// Newton's method stops when the step's equations, cell by cell and at the
/// surface, hold to this fraction of the step's energy scale (the cells'
/// changes taken without sign, the heat deposited and the heat to the water).
pub const NEWTON_IMBALANCE_TOLERANCE: f64 = 1.0e-13;
/// Or to this many joules, whichever is larger: about a hundred times the
/// rounding of the cells' energies in a step.
pub const NEWTON_IMBALANCE_FLOOR_J: f64 = 1.0e-8;
/// Most Newton iterations in one step.
pub const MAX_NEWTON_ITERATIONS: u32 = 30;
/// The energy account of a step must close to this fraction.
pub const STEP_ENERGY_TOLERANCE: f64 = 1.0e-9;
/// The energy account of the whole run must close to this fraction of the
/// energy deposited.
pub const RUN_ENERGY_TOLERANCE: f64 = 1.0e-9;
/// The floor of the energy scale of an account, J: a step that moves less
/// than this is checked to `STEP_ENERGY_TOLERANCE` of it, 0.1 microjoule.
pub const ENERGY_FLOOR_J: f64 = 100.0;

/// Freezing point of the pool, K: the start is above it, the pool liquid.
pub const POOL_FREEZING_K: f64 = 273.15;
/// The meat's temperature limit, K: 1150 degC, the safety limit of U-ZrH1.6
/// fuel in a stainless-steel clad, the reactor_triga_jsi card's
/// `FUEL_TEMPERATURE_LIMIT_K` (General Atomics; NETL TRIGA safety analysis
/// report, section 4.5). Above it the meat's hydrogen pressure loads the clad
/// past its strength, and the properties here no longer hold.
pub const FUEL_TEMPERATURE_LIMIT_K: f64 = 1150.0 + 273.15;
/// Zuber's critical heat flux of saturated water at 385 K on the clad, W per
/// m2: 0.149 h_fg rho_v (sigma g (rho_l - rho_v) / rho_v^2)^(1/4), with the
/// saturated properties above (Incropera and DeWitt, eq. 10.7). Above it the
/// surface leaves nucleate boiling, and Rohsenow's law no longer holds.
pub fn critical_heat_flux_w_per_m2() -> f64 {
    let lift = BOIL_SURFACE_TENSION_N_PER_M * GRAVITY_M_PER_S2 * (BOIL_LIQUID_DENSITY_KG_PER_M3 - BOIL_VAPOUR_DENSITY_KG_PER_M3)
        / (BOIL_VAPOUR_DENSITY_KG_PER_M3 * BOIL_VAPOUR_DENSITY_KG_PER_M3);
    0.149 * BOIL_LATENT_HEAT_J_PER_KG * BOIL_VAPOUR_DENSITY_KG_PER_M3 * lift.sqrt().sqrt()
}
/// Most points of a history, and its longest span, s: the capacity of one
/// solve (a history of 600 s takes 300,000 steps of 2 ms), not physics.
pub const HISTORY_POINTS_MAX: usize = 100_000;
pub const HISTORY_SPAN_MAX_S: f64 = 600.0;

/// The compared span: the envelope of the 93 histories compared (the 39 fitted
/// pulses, and the 16 pulses left out of the fit, each from its record's power
/// and from the reactor_triga_jsi card), start temperature rounded outward to
/// 0.01 K, energy to 0.01 MW s, power to 1 MW
/// (`evidence/research/reactor_triga_jsi_element_6753/analysis/out/05_compared_span.json`).
/// Outside it the solver answers with the same equations and sets the
/// answer's flag of that quantity.
pub const START_TEMPERATURE_COMPARED_MIN_K: f64 = 293.14;
pub const START_TEMPERATURE_COMPARED_MAX_K: f64 = 305.53;
/// Energy the core released by the history's end, MW s.
pub const CORE_ENERGY_COMPARED_MIN_MWS: f64 = 6.85;
pub const CORE_ENERGY_COMPARED_MAX_MWS: f64 = 19.41;
/// Highest core power over one interval of the history, MW.
pub const CORE_POWER_COMPARED_MAX_MW: f64 = 994.0;
/// The compared histories: intervals from 1e-5 s to 10 ms (with 1 percent for
/// a clock's rounding; the compared ones were 1 ms and 10 ms), spans to 14 s.
pub const HISTORY_INTERVAL_COMPARED_MIN_S: f64 = 1.0e-5;
pub const HISTORY_INTERVAL_COMPARED_MAX_S: f64 = 0.0101;
pub const HISTORY_SPAN_COMPARED_MAX_S: f64 = 14.0;

/// The question: the start temperature of the element and the pool, and the
/// history of the energy the core released.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementQuestion {
    /// Temperature of the whole element and of the pool at the history's start, K.
    pub start_temperature_k: f64,
    /// Times of the history, s, from its start; each after the one before.
    pub time_s: Vec<f64>,
    /// Energy the core has released since the history's start, MW s, at each
    /// time; never falling.
    pub core_energy_released_mws: Vec<f64>,
}

/// The two fitted constants, for the fit itself and the tests. A served
/// answer uses [`GAP_CONDUCTANCE_W_PER_M2_K`] and [`THERMOCOUPLE_TIME_CONSTANT_S`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fitted {
    pub gap_conductance_w_per_m2_k: f64,
    pub thermocouple_time_constant_s: f64,
}

/// The constants served.
pub const FITTED: Fitted = Fitted {
    gap_conductance_w_per_m2_k: GAP_CONDUCTANCE_W_PER_M2_K,
    thermocouple_time_constant_s: THERMOCOUPLE_TIME_CONSTANT_S,
};

/// The element through the history.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementPulse {
    /// The thermocouple's reading at each time of the history, K.
    pub thermocouple_temperature_k: Vec<f64>,
    /// The meat's temperature at the thermocouple's radius at each time, K.
    pub fuel_temperature_at_thermocouple_k: Vec<f64>,
    /// The highest reading of the thermocouple at a time of the history, K.
    pub peak_thermocouple_temperature_k: f64,
    /// The time of the history at which it was reached, s.
    pub peak_thermocouple_time_s: f64,
    /// The highest temperature of any cell at any step, K.
    pub peak_fuel_temperature_k: f64,
    /// The highest temperature of the clad's surface at any step, K.
    pub peak_clad_surface_temperature_k: f64,
    /// Energy deposited in the element over the history, J.
    pub element_energy_deposited_j: f64,
    /// Energy that crossed the clad's surface into the water, J.
    pub energy_to_water_j: f64,
    /// Energy stored in the element at the end over its start, J.
    pub energy_stored_j: f64,
    /// The largest step energy residual.
    pub largest_step_energy_residual: f64,
    /// The run energy residual at the end.
    pub run_energy_residual: f64,
    /// Steps taken.
    pub steps: u64,
    /// Newton iterations, all steps together.
    pub newton_iterations: u64,
    /// Radius of every cell centre, m, axis outwards, then the clad's surface.
    pub field_radius_m: Vec<f64>,
    /// Temperature there at the end, K.
    pub field_temperature_k: Vec<f64>,
    /// 1 where the start temperature is outside the compared span, else 0.
    pub start_temperature_outside_compared: f64,
    /// 1 where the energy released by the history's end is outside the compared span, else 0.
    pub core_energy_outside_compared: f64,
    /// 1 where the core power over an interval is above the compared span, else 0.
    pub core_power_outside_compared: f64,
    /// 1 where an interval or the span of the history is outside the compared histories', else 0.
    pub history_outside_compared: f64,
}

/// Why no answer is given.
#[derive(Clone, Debug, PartialEq)]
pub enum FuelRodReject {
    /// An input was NaN or infinite.
    InputNotFinite { field: &'static str },
    /// The history has no point, more than 100,000 points, or a different count of times and energies.
    HistoryShape { times: usize, energies: usize },
    /// The start temperature is at or below 273.15 K, where the pool freezes, or at or above 385 K, where the water at the element boils: the pool is liquid water at the start temperature.
    StartTemperatureOutsideLiquidPool { start_temperature_k: f64 },
    /// A time of the history is not after the one before it, the first not after zero: time runs forward.
    HistoryTimesNotRising { index: usize, time_s: f64 },
    /// The history runs past 600 s, the capacity of one solve (300,000 steps of 2 ms).
    HistoryBeyondCapacity { time_s: f64 },
    /// The energy released falls from one time to the next, or is negative: energy released does not fall.
    EnergyFalls { index: usize, core_energy_released_mws: f64 },
    /// The meat passed 1423.15 K (1150 degC), the safety limit of U-ZrH1.6 fuel in a steel clad, where its hydrogen pressure loads the clad past its strength.
    FuelAboveSafetyLimit { time_s: f64, temperature_k: f64 },
    /// The heat flux from the clad passed the critical heat flux of saturated water at 385 K, 1.47 MW per m2 (Zuber): there the surface leaves nucleate boiling.
    CriticalHeatFluxPassed { time_s: f64, heat_flux_w_per_m2: f64 },
    /// A temperature or an energy was NaN or infinite.
    NonFiniteValue { time_s: f64 },
    /// A Newton system had no usable pivot.
    SingularSystem { time_s: f64 },
    /// Newton's method had not settled after 30 iterations of one step.
    NotSettled { time_s: f64, iterations: u32 },
    /// A step's energy account was open by more than 1e-9 of its scale, or the run's by more than 1e-9 of the energy deposited.
    EnergyLedgerBroken { time_s: f64, residual: f64 },
}

impl FuelRodReject {
    /// The variant's name, without its fields.
    pub fn name(&self) -> &'static str {
        match self {
            Self::InputNotFinite { .. } => "InputNotFinite",
            Self::HistoryShape { .. } => "HistoryShape",
            Self::StartTemperatureOutsideLiquidPool { .. } => "StartTemperatureOutsideLiquidPool",
            Self::HistoryTimesNotRising { .. } => "HistoryTimesNotRising",
            Self::HistoryBeyondCapacity { .. } => "HistoryBeyondCapacity",
            Self::EnergyFalls { .. } => "EnergyFalls",
            Self::FuelAboveSafetyLimit { .. } => "FuelAboveSafetyLimit",
            Self::CriticalHeatFluxPassed { .. } => "CriticalHeatFluxPassed",
            Self::NonFiniteValue { .. } => "NonFiniteValue",
            Self::SingularSystem { .. } => "SingularSystem",
            Self::NotSettled { .. } => "NotSettled",
            Self::EnergyLedgerBroken { .. } => "EnergyLedgerBroken",
        }
    }
}

impl fmt::Display for FuelRodReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputNotFinite { field } => write!(f, "{field} is not a finite number"),
            Self::HistoryShape { times, energies } => write!(
                f,
                "the history has {times} times and {energies} energies; it takes 1 to {HISTORY_POINTS_MAX} points, as many of each"
            ),
            Self::StartTemperatureOutsideLiquidPool { start_temperature_k } => write!(
                f,
                "start temperature {start_temperature_k} K is not above {POOL_FREEZING_K} K, where the pool freezes, and below {SATURATION_TEMPERATURE_K} K, where the water at the element boils"
            ),
            Self::HistoryTimesNotRising { index, time_s } => write!(
                f,
                "time {time_s} s at index {index} of the history is not after the time before it (the first after 0 s): time runs forward"
            ),
            Self::HistoryBeyondCapacity { time_s } => write!(
                f,
                "the history runs to {time_s} s, past {HISTORY_SPAN_MAX_S} s, the capacity of one solve"
            ),
            Self::EnergyFalls { index, core_energy_released_mws } => write!(
                f,
                "the energy released, {core_energy_released_mws} MW s at index {index} of the history, is negative or below the value before it; energy released does not fall"
            ),
            Self::FuelAboveSafetyLimit { time_s, temperature_k } => write!(
                f,
                "at {time_s} s the meat reached {temperature_k} K, above {FUEL_TEMPERATURE_LIMIT_K} K (1150 degC), the safety limit of U-ZrH1.6 fuel in a steel clad"
            ),
            Self::CriticalHeatFluxPassed { time_s, heat_flux_w_per_m2 } => write!(
                f,
                "at {time_s} s the heat flux from the clad reached {heat_flux_w_per_m2} W/m2, above the critical heat flux {} W/m2 of saturated water at {SATURATION_TEMPERATURE_K} K, where nucleate boiling ends",
                critical_heat_flux_w_per_m2()
            ),
            Self::NonFiniteValue { time_s } => write!(f, "a temperature or an energy was not finite at {time_s} s"),
            Self::SingularSystem { time_s } => write!(f, "the Newton system had no usable pivot at {time_s} s"),
            Self::NotSettled { time_s, iterations } => {
                write!(f, "the step to {time_s} s had not settled after {iterations} Newton iterations")
            }
            Self::EnergyLedgerBroken { time_s, residual } => write!(
                f,
                "at {time_s} s the energy account was open by {residual} of its scale, more than {STEP_ENERGY_TOLERANCE} for a step or {RUN_ENERGY_TOLERANCE} for the run"
            ),
        }
    }
}

/// Every limit the question itself shows, in the order they are asked.
pub fn admit(question: &ElementQuestion) -> Result<(), FuelRodReject> {
    if !question.start_temperature_k.is_finite() {
        return Err(FuelRodReject::InputNotFinite { field: "start_temperature_k" });
    }
    if question.time_s.iter().any(|value| !value.is_finite()) {
        return Err(FuelRodReject::InputNotFinite { field: "time_s" });
    }
    if question.core_energy_released_mws.iter().any(|value| !value.is_finite()) {
        return Err(FuelRodReject::InputNotFinite { field: "core_energy_released_mws" });
    }
    let points = question.time_s.len();
    if points == 0 || points > HISTORY_POINTS_MAX || question.core_energy_released_mws.len() != points {
        return Err(FuelRodReject::HistoryShape {
            times: points,
            energies: question.core_energy_released_mws.len(),
        });
    }
    let start = question.start_temperature_k;
    if !(start > POOL_FREEZING_K && start < SATURATION_TEMPERATURE_K) {
        return Err(FuelRodReject::StartTemperatureOutsideLiquidPool { start_temperature_k: start });
    }
    let mut before_t = 0.0_f64;
    let mut before_e = 0.0_f64;
    for (index, (&t, &e)) in question.time_s.iter().zip(&question.core_energy_released_mws).enumerate() {
        if !(t > before_t) {
            return Err(FuelRodReject::HistoryTimesNotRising { index, time_s: t });
        }
        if t > HISTORY_SPAN_MAX_S {
            return Err(FuelRodReject::HistoryBeyondCapacity { time_s: t });
        }
        if !(e >= before_e) {
            return Err(FuelRodReject::EnergyFalls {
                index,
                core_energy_released_mws: e,
            });
        }
        before_t = t;
        before_e = e;
    }
    Ok(())
}

/// Where a question stands against the compared span: the four flags the
/// answer carries, each true outside it.
pub fn outside_compared(question: &ElementQuestion) -> [bool; 4] {
    let start = question.start_temperature_k;
    let start_out = !(START_TEMPERATURE_COMPARED_MIN_K..=START_TEMPERATURE_COMPARED_MAX_K).contains(&start);
    let mut before_t = 0.0_f64;
    let mut before_e = 0.0_f64;
    let mut power_out = false;
    let mut history_out = false;
    for (&t, &e) in question.time_s.iter().zip(&question.core_energy_released_mws) {
        let interval = t - before_t;
        if !(HISTORY_INTERVAL_COMPARED_MIN_S..=HISTORY_INTERVAL_COMPARED_MAX_S).contains(&interval) || t > HISTORY_SPAN_COMPARED_MAX_S {
            history_out = true;
        }
        if (e - before_e) / interval > CORE_POWER_COMPARED_MAX_MW {
            power_out = true;
        }
        before_t = t;
        before_e = e;
    }
    let energy_out = !(CORE_ENERGY_COMPARED_MIN_MWS..=CORE_ENERGY_COMPARED_MAX_MWS).contains(&before_e);
    [start_out, energy_out, power_out, history_out]
}

/// The element through the history, with the constants served. Each value
/// of the answer below the normal range is written as zero with its sign
/// ([`crate::normal`]).
pub fn solve(question: &ElementQuestion) -> Result<ElementPulse, FuelRodReject> {
    crate::normal::held(solve_with(question, FITTED))
}

/// Every 64-bit number of the answer, held to the rule.
impl crate::normal::NormalOrZero for ElementPulse {
    fn normal_or_zero(&mut self) {
        self.thermocouple_temperature_k.normal_or_zero();
        self.fuel_temperature_at_thermocouple_k.normal_or_zero();
        self.peak_thermocouple_temperature_k.normal_or_zero();
        self.peak_thermocouple_time_s.normal_or_zero();
        self.peak_fuel_temperature_k.normal_or_zero();
        self.peak_clad_surface_temperature_k.normal_or_zero();
        self.element_energy_deposited_j.normal_or_zero();
        self.energy_to_water_j.normal_or_zero();
        self.energy_stored_j.normal_or_zero();
        self.largest_step_energy_residual.normal_or_zero();
        self.run_energy_residual.normal_or_zero();
        self.field_radius_m.normal_or_zero();
        self.field_temperature_k.normal_or_zero();
    }
}

/// The element through the history with the two fitted constants given: for
/// the fit and the tests. The question is held to the declared range first.
pub fn solve_with(question: &ElementQuestion, fitted: Fitted) -> Result<ElementPulse, FuelRodReject> {
    admit(question)?;
    Element::new(fitted).run(question)
}

/// The volume of the meat, m3.
pub fn meat_volume_m3() -> f64 {
    std::f64::consts::PI * (MEAT_RADIUS_M * MEAT_RADIUS_M - ROD_RADIUS_M * ROD_RADIUS_M) * MEAT_LENGTH_M
}

/// Specific enthalpy of the meat above `MEAT_HEAT_CAPACITY_REFERENCE_K`, J per kg.
pub fn meat_enthalpy_j_per_kg(temperature_k: f64) -> f64 {
    let x = temperature_k - MEAT_HEAT_CAPACITY_REFERENCE_K;
    MEAT_HEAT_CAPACITY_BASE_J_PER_KG_K * x + 0.5 * MEAT_HEAT_CAPACITY_SLOPE_J_PER_KG_K2 * x * x
}

fn meat_heat_capacity(temperature_k: f64) -> f64 {
    MEAT_HEAT_CAPACITY_BASE_J_PER_KG_K + MEAT_HEAT_CAPACITY_SLOPE_J_PER_KG_K2 * (temperature_k - MEAT_HEAT_CAPACITY_REFERENCE_K)
}

/// Heat flux from the clad's surface into the water, W per m2, and its
/// derivative with the surface temperature, for a pool at `pool_k`.
pub fn water_flux(surface_k: f64, pool_k: f64) -> (f64, f64) {
    let (conv, conv_slope) = convection(surface_k - pool_k);
    let (boil, boil_slope) = boiling(surface_k - SATURATION_TEMPERATURE_K);
    (conv + boil, conv_slope + boil_slope)
}

/// Churchill and Chu's correlation for a vertical surface, water at 300 K.
fn convection(difference_k: f64) -> (f64, f64) {
    let sign = if difference_k < 0.0 { -1.0 } else { 1.0 };
    let dt = difference_k.abs();
    let a = 0.387 / math::powf(1.0 + math::powf(0.492 / WATER_PRANDTL, 9.0 / 16.0), 8.0 / 27.0);
    let rayleigh = GRAVITY_M_PER_S2 * WATER_EXPANSION_PER_K * dt * math::powi(MEAT_LENGTH_M, 3)
        / (WATER_KINEMATIC_VISCOSITY_M2_PER_S * WATER_DIFFUSIVITY_M2_PER_S);
    let root = math::powf(rayleigh, 1.0 / 6.0);
    let base = 0.825 + a * root;
    let nusselt = base * base;
    let scale = WATER_CONDUCTIVITY_W_PER_M_K / MEAT_LENGTH_M;
    let flux = scale * nusselt * dt;
    let slope = scale * (nusselt + base * a * root / 3.0);
    (sign * flux, slope)
}

/// Rohsenow's nucleate boiling for a surface `excess_k` above saturation.
fn boiling(excess_k: f64) -> (f64, f64) {
    if excess_k <= 0.0 {
        return (0.0, 0.0);
    }
    let lead = BOIL_LIQUID_VISCOSITY_PA_S
        * BOIL_LATENT_HEAT_J_PER_KG
        * (GRAVITY_M_PER_S2 * (BOIL_LIQUID_DENSITY_KG_PER_M3 - BOIL_VAPOUR_DENSITY_KG_PER_M3) / BOIL_SURFACE_TENSION_N_PER_M).sqrt();
    let unit = BOIL_LIQUID_HEAT_CAPACITY_J_PER_KG_K / (ROHSENOW_SURFACE_CONSTANT * BOIL_LATENT_HEAT_J_PER_KG * BOIL_LIQUID_PRANDTL);
    let x = unit * excess_k;
    (lead * x * x * x, 3.0 * lead * x * x * unit)
}

#[derive(Clone, Copy, PartialEq)]
enum Material {
    Rod,
    Meat,
    Clad,
}

struct Element {
    fitted: Fitted,
    material: Vec<Material>,
    centre: Vec<f64>,
    mass: Vec<f64>,
    /// Conductance from cell i to cell i+1, W per K; the last is to the surface node.
    conductance: Vec<f64>,
    surface_area: f64,
    /// Share of the meat's heat source each cell takes.
    source_share: Vec<f64>,
    /// The two cells the thermocouple lies between, and the weight of the outer.
    probe: (usize, usize, f64),
}

fn enthalpy(material: Material, temperature_k: f64) -> f64 {
    match material {
        Material::Meat => meat_enthalpy_j_per_kg(temperature_k),
        Material::Rod => ROD_HEAT_CAPACITY_J_PER_KG_K * (temperature_k - MEAT_HEAT_CAPACITY_REFERENCE_K),
        Material::Clad => CLAD_HEAT_CAPACITY_J_PER_KG_K * (temperature_k - MEAT_HEAT_CAPACITY_REFERENCE_K),
    }
}

/// The specific enthalpy from `before` to `after`, J per kg, written as a
/// product with the temperature difference so that a small step keeps its
/// digits.
fn enthalpy_change(material: Material, before: f64, after: f64) -> f64 {
    let difference = after - before;
    match material {
        Material::Meat => {
            let mean = 0.5 * (after + before) - MEAT_HEAT_CAPACITY_REFERENCE_K;
            difference * (MEAT_HEAT_CAPACITY_BASE_J_PER_KG_K + MEAT_HEAT_CAPACITY_SLOPE_J_PER_KG_K2 * mean)
        }
        Material::Rod => ROD_HEAT_CAPACITY_J_PER_KG_K * difference,
        Material::Clad => CLAD_HEAT_CAPACITY_J_PER_KG_K * difference,
    }
}

fn heat_capacity(material: Material, temperature_k: f64) -> f64 {
    match material {
        Material::Meat => meat_heat_capacity(temperature_k),
        Material::Rod => ROD_HEAT_CAPACITY_J_PER_KG_K,
        Material::Clad => CLAD_HEAT_CAPACITY_J_PER_KG_K,
    }
}

fn conductivity(material: Material) -> f64 {
    match material {
        Material::Meat => MEAT_CONDUCTIVITY_W_PER_M_K,
        Material::Rod => ROD_CONDUCTIVITY_W_PER_M_K,
        Material::Clad => CLAD_CONDUCTIVITY_W_PER_M_K,
    }
}

fn density(material: Material) -> f64 {
    match material {
        Material::Meat => MEAT_DENSITY_KG_PER_M3,
        Material::Rod => ROD_DENSITY_KG_PER_M3,
        Material::Clad => CLAD_DENSITY_KG_PER_M3,
    }
}

impl Element {
    fn new(fitted: Fitted) -> Self {
        let two_pi_l = 2.0 * std::f64::consts::PI * MEAT_LENGTH_M;
        let mut faces: Vec<(f64, f64, Material)> = Vec::new();
        let layer = |faces: &mut Vec<(f64, f64, Material)>, inner: f64, outer: f64, cells: usize, material: Material| {
            for i in 0..cells {
                let a = inner + (outer - inner) * i as f64 / cells as f64;
                let b = if i + 1 == cells { outer } else { inner + (outer - inner) * (i + 1) as f64 / cells as f64 };
                faces.push((a, b, material));
            }
        };
        layer(&mut faces, 0.0, ROD_RADIUS_M, ROD_CELLS, Material::Rod);
        layer(&mut faces, ROD_RADIUS_M, MEAT_RADIUS_M, MEAT_CELLS, Material::Meat);
        layer(&mut faces, GAP_RADIUS_M, CLAD_RADIUS_M, CLAD_CELLS, Material::Clad);
        let n = faces.len();
        let mut material = Vec::with_capacity(n);
        let mut centre = Vec::with_capacity(n);
        let mut mass = Vec::with_capacity(n);
        let mut source_share = Vec::with_capacity(n);
        let meat_area = MEAT_RADIUS_M * MEAT_RADIUS_M - ROD_RADIUS_M * ROD_RADIUS_M;
        for &(a, b, m) in &faces {
            material.push(m);
            centre.push(0.5 * (a + b));
            let volume = std::f64::consts::PI * (b * b - a * a) * MEAT_LENGTH_M;
            mass.push(volume * density(m));
            source_share.push(if m == Material::Meat { (b * b - a * a) / meat_area } else { 0.0 });
        }
        // Resistance from a cell centre to a face of the same cell, K per W.
        let half = |from: f64, to: f64, m: Material| -> f64 { math::log(to / from).abs() / (conductivity(m) * two_pi_l) };
        let mut conductance = Vec::with_capacity(n);
        for i in 0..n {
            let (_, b, m) = faces[i];
            if i + 1 == n {
                conductance.push(1.0 / half(centre[i], CLAD_RADIUS_M, m));
                continue;
            }
            let (a_next, _, m_next) = faces[i + 1];
            let mut resistance = half(centre[i], b, m) + half(centre[i + 1], a_next, m_next);
            if m == Material::Meat && m_next == Material::Clad {
                resistance = half(centre[i], MEAT_RADIUS_M, m)
                    + 1.0 / (fitted.gap_conductance_w_per_m2_k * two_pi_l * MEAT_RADIUS_M)
                    + half(centre[i + 1], GAP_RADIUS_M, m_next);
            }
            conductance.push(1.0 / resistance);
        }
        let mut probe = (0, 0, 0.0);
        for i in 0..n - 1 {
            if material[i] == Material::Meat
                && material[i + 1] == Material::Meat
                && centre[i] <= THERMOCOUPLE_RADIUS_M
                && THERMOCOUPLE_RADIUS_M < centre[i + 1]
            {
                probe = (i, i + 1, (THERMOCOUPLE_RADIUS_M - centre[i]) / (centre[i + 1] - centre[i]));
            }
        }
        Element {
            fitted,
            material,
            centre,
            mass,
            conductance,
            surface_area: two_pi_l * CLAD_RADIUS_M,
            source_share,
            probe,
        }
    }

    /// The energy stored in the element over a step, J, cell by cell, and the
    /// sum of the cells' changes taken without sign.
    fn stored_change(&self, before: &[f64], after: &[f64]) -> (f64, f64) {
        let mut total = 0.0;
        let mut moved = 0.0;
        for i in 0..self.mass.len() {
            let change = self.mass[i] * enthalpy_change(self.material[i], before[i], after[i]);
            total += change;
            moved += change.abs();
        }
        (total, moved)
    }

    fn probe_temperature(&self, temperatures: &[f64]) -> f64 {
        let (i, j, w) = self.probe;
        temperatures[i] * (1.0 - w) + temperatures[j] * w
    }

    fn stored(&self, temperatures: &[f64]) -> f64 {
        self.mass
            .iter()
            .zip(&self.material)
            .zip(temperatures)
            .map(|((m, &mat), &t)| m * enthalpy(mat, t))
            .sum()
    }

    fn run(&self, question: &ElementQuestion) -> Result<ElementPulse, FuelRodReject> {
        let n = self.mass.len();
        let pool = question.start_temperature_k;
        // Cells, then the surface node.
        let mut t = vec![pool; n + 1];
        let mut thermocouple = pool;
        let stored_start = self.stored(&t[..n]);
        let mut deposited = 0.0_f64;
        let mut to_water = 0.0_f64;
        let mut largest_step = 0.0_f64;
        let mut peak_fuel = pool;
        let mut peak_surface = pool;
        let mut steps = 0_u64;
        let mut newton_total = 0_u64;
        let mut readings = Vec::with_capacity(question.time_s.len());
        let mut probes = Vec::with_capacity(question.time_s.len());
        let mut clock = 0.0_f64;
        let mut before_e = 0.0_f64;
        let element_share = ELEMENT_POWER_FACTOR / CORE_ELEMENTS;
        let tau = self.fitted.thermocouple_time_constant_s;
        let mut old = t.clone();
        for (&t_end, &e_end) in question.time_s.iter().zip(&question.core_energy_released_mws) {
            let interval = t_end - clock;
            let pieces = (interval / STEP_MAX_S).ceil().max(1.0) as u64;
            let element_power_w = (e_end - before_e) * 1.0e6 / interval * element_share;
            for piece in 0..pieces {
                let start = clock + interval * piece as f64 / pieces as f64;
                let end = if piece + 1 == pieces { t_end } else { clock + interval * (piece + 1) as f64 / pieces as f64 };
                let dt = end - start;
                old.copy_from_slice(&t);
                let iterations = self.step(&mut t, &old, dt, element_power_w, pool, end)?;
                newton_total += u64::from(iterations);
                steps += 1;
                let (stored_change, moved) = self.stored_change(&old[..n], &t[..n]);
                let q_in = element_power_w * dt;
                let (flux, _) = water_flux(t[n], pool);
                let q_out = flux * self.surface_area * dt;
                let scale = stored_change.abs().max(q_in.abs()).max(q_out.abs()).max(moved).max(ENERGY_FLOOR_J);
                let residual = (stored_change - (q_in - q_out)).abs() / scale;
                if !residual.is_finite() {
                    return Err(FuelRodReject::NonFiniteValue { time_s: end });
                }
                if residual > STEP_ENERGY_TOLERANCE {
                    return Err(FuelRodReject::EnergyLedgerBroken { time_s: end, residual });
                }
                largest_step = largest_step.max(residual);
                deposited += q_in;
                to_water += q_out;
                for &value in &t[..n] {
                    peak_fuel = peak_fuel.max(value);
                    if value > FUEL_TEMPERATURE_LIMIT_K {
                        return Err(FuelRodReject::FuelAboveSafetyLimit { time_s: end, temperature_k: value });
                    }
                }
                if flux > critical_heat_flux_w_per_m2() {
                    return Err(FuelRodReject::CriticalHeatFluxPassed { time_s: end, heat_flux_w_per_m2: flux });
                }
                peak_surface = peak_surface.max(t[n]);
                // The thermocouple, exactly for a reading that is a straight
                // line in time over the step.
                let u0 = self.probe_temperature(&old);
                let u1 = self.probe_temperature(&t);
                let slope = (u1 - u0) / dt;
                let decay = math::exp(-dt / tau);
                thermocouple = u1 + (thermocouple - u0 + slope * tau) * decay - slope * tau;
            }
            clock = t_end;
            before_e = e_end;
            readings.push(thermocouple);
            probes.push(self.probe_temperature(&t));
        }
        let stored_end = self.stored(&t[..n]);
        let stored = stored_end - stored_start;
        let run_residual = (stored - (deposited - to_water)).abs() / deposited.abs().max(ENERGY_FLOOR_J);
        if !run_residual.is_finite() {
            return Err(FuelRodReject::NonFiniteValue { time_s: clock });
        }
        if run_residual > RUN_ENERGY_TOLERANCE {
            return Err(FuelRodReject::EnergyLedgerBroken {
                time_s: clock,
                residual: run_residual,
            });
        }
        let mut peak_index = 0;
        for (index, &value) in readings.iter().enumerate() {
            if value > readings[peak_index] {
                peak_index = index;
            }
        }
        let mut field_radius_m = self.centre.clone();
        field_radius_m.push(CLAD_RADIUS_M);
        let flag = |outside: bool| if outside { 1.0 } else { 0.0 };
        let [start_out, energy_out, power_out, history_out] = outside_compared(question);
        Ok(ElementPulse {
            start_temperature_outside_compared: flag(start_out),
            core_energy_outside_compared: flag(energy_out),
            core_power_outside_compared: flag(power_out),
            history_outside_compared: flag(history_out),
            peak_thermocouple_temperature_k: readings[peak_index],
            peak_thermocouple_time_s: question.time_s[peak_index],
            thermocouple_temperature_k: readings,
            fuel_temperature_at_thermocouple_k: probes,
            peak_fuel_temperature_k: peak_fuel,
            peak_clad_surface_temperature_k: peak_surface,
            element_energy_deposited_j: deposited,
            energy_to_water_j: to_water,
            energy_stored_j: stored,
            largest_step_energy_residual: largest_step,
            run_energy_residual: run_residual,
            steps,
            newton_iterations: newton_total,
            field_radius_m,
            field_temperature_k: t,
        })
    }

    /// One backward Euler step in the enthalpy, by Newton's method. Returns the
    /// iterations taken.
    fn step(&self, t: &mut [f64], old: &[f64], dt: f64, power_w: f64, pool: f64, time_s: f64) -> Result<u32, FuelRodReject> {
        let n = self.mass.len();
        let mut lower = vec![0.0; n + 1];
        let mut diag = vec![0.0; n + 1];
        let mut upper = vec![0.0; n + 1];
        let mut rhs = vec![0.0; n + 1];
        for iteration in 1..=MAX_NEWTON_ITERATIONS {
            let mut imbalance = 0.0_f64;
            let mut scale = dt * power_w.abs();
            for i in 0..n {
                let mat = self.material[i];
                let left = if i == 0 { 0.0 } else { self.conductance[i - 1] };
                let right = self.conductance[i];
                let stored = self.mass[i] * enthalpy_change(mat, old[i], t[i]);
                scale += stored.abs();
                let mut f = stored - dt * power_w * self.source_share[i];
                f -= dt * right * (t[i + 1] - t[i]);
                if i > 0 {
                    f -= dt * left * (t[i - 1] - t[i]);
                }
                rhs[i] = -f;
                diag[i] = self.mass[i] * heat_capacity(mat, t[i]) + dt * (left + right);
                lower[i] = if i == 0 { 0.0 } else { -dt * left };
                upper[i] = -dt * right;
                imbalance += f.abs();
            }
            let g = self.conductance[n - 1];
            let (flux, slope) = water_flux(t[n], pool);
            let f_s = dt * (g * (t[n - 1] - t[n]) - self.surface_area * flux);
            scale += dt * self.surface_area * flux.abs();
            rhs[n] = -f_s;
            lower[n] = dt * g;
            diag[n] = dt * (-g - self.surface_area * slope);
            upper[n] = 0.0;
            imbalance += f_s.abs();
            if !imbalance.is_finite() {
                return Err(FuelRodReject::NonFiniteValue { time_s });
            }
            // Settled: the step's equations hold at the present values.
            if imbalance <= (NEWTON_IMBALANCE_TOLERANCE * scale).max(NEWTON_IMBALANCE_FLOOR_J) {
                return Ok(iteration);
            }
            // Thomas algorithm.
            for i in 1..=n {
                if diag[i - 1] == 0.0 || !diag[i - 1].is_finite() {
                    return Err(FuelRodReject::SingularSystem { time_s });
                }
                let w = lower[i] / diag[i - 1];
                diag[i] -= w * upper[i - 1];
                rhs[i] -= w * rhs[i - 1];
            }
            if diag[n] == 0.0 || !diag[n].is_finite() {
                return Err(FuelRodReject::SingularSystem { time_s });
            }
            let mut next = rhs[n] / diag[n];
            t[n] += next;
            for i in (0..n).rev() {
                next = (rhs[i] - upper[i] * next) / diag[i];
                t[i] += next;
            }
            if t.iter().any(|value| !value.is_finite()) {
                return Err(FuelRodReject::NonFiniteValue { time_s });
            }
        }
        Err(FuelRodReject::NotSettled {
            time_s,
            iterations: MAX_NEWTON_ITERATIONS,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pulse(energy_mws: f64, span_s: f64, interval_s: f64) -> ElementQuestion {
        // A pulse of `energy_mws` released evenly over 20 ms from 0.3 s.
        let points = (span_s / interval_s).round() as usize;
        let mut time_s = Vec::with_capacity(points);
        let mut energy = Vec::with_capacity(points);
        for k in 1..=points {
            let t = k as f64 * interval_s;
            time_s.push(t);
            let fraction = ((t - 0.3) / 0.02).clamp(0.0, 1.0);
            energy.push(energy_mws * fraction);
        }
        ElementQuestion {
            start_temperature_k: 297.0,
            time_s,
            core_energy_released_mws: energy,
        }
    }

    /// The article's adiabatic relation, E = m ∫ c dT (Švajger et al. 2024,
    /// eq. 7.6): with no gap conductance worth the name the meat keeps the
    /// energy given, and its enthalpy rise is the element's share of it.
    #[test]
    fn adiabatic_meat_against_the_article() {
        let question = pulse(12.0, 1.0, 0.01);
        let fitted = Fitted {
            gap_conductance_w_per_m2_k: 1.0e-12,
            thermocouple_time_constant_s: 1.0,
        };
        let answer = solve_with(&question, fitted).expect("solved");
        let share = 12.0e6 * ELEMENT_POWER_FACTOR / CORE_ELEMENTS;
        assert!((answer.element_energy_deposited_j - share).abs() / share < 1e-12);
        let meat_mass = meat_volume_m3() * MEAT_DENSITY_KG_PER_M3;
        // The meat's mean enthalpy rise against the energy, with the rod and the
        // clad taking almost nothing by conduction through 1e-12 W per m2 K.
        let element = Element::new(fitted);
        let meat_enthalpy: f64 = (0..element.mass.len())
            .filter(|&i| element.material[i] == Material::Meat)
            .map(|i| element.mass[i] * (meat_enthalpy_j_per_kg(answer.field_temperature_k[i]) - meat_enthalpy_j_per_kg(297.0)))
            .sum();
        // Heat reaches the rod by conduction within the meat; it is in the
        // element still.
        let rod_enthalpy: f64 = (0..element.mass.len())
            .filter(|&i| element.material[i] == Material::Rod)
            .map(|i| element.mass[i] * ROD_HEAT_CAPACITY_J_PER_KG_K * (answer.field_temperature_k[i] - 297.0))
            .sum();
        assert!(((meat_enthalpy + rod_enthalpy) - share).abs() / share < 1e-9, "{meat_enthalpy} {rod_enthalpy} {share}");
        // The uniform rise the article's relation gives for the meat alone.
        let target = meat_enthalpy_j_per_kg(297.0) + share / meat_mass;
        let x = {
            let a = 0.5 * MEAT_HEAT_CAPACITY_SLOPE_J_PER_KG_K2;
            let b = MEAT_HEAT_CAPACITY_BASE_J_PER_KG_K;
            (-b + (b * b + 4.0 * a * target).sqrt()) / (2.0 * a)
        };
        let uniform = MEAT_HEAT_CAPACITY_REFERENCE_K + x;
        // The outer meat, far from the rod, is at the adiabatic temperature.
        let outer = ROD_CELLS + MEAT_CELLS - 1;
        assert!((answer.field_temperature_k[outer] - uniform).abs() < 0.05, "{} {uniform}", answer.field_temperature_k[outer]);
        assert!(answer.energy_to_water_j.abs() < 1e-3);
    }

    /// Steady conduction through the layers against its closed form: a long
    /// steady heating at constant power, with the clad's surface temperature
    /// as computed, gives the meat's temperature at the thermocouple from the
    /// annulus solution with an insulated inner face.
    #[test]
    fn steady_conduction_against_the_closed_form() {
        let fitted = Fitted {
            gap_conductance_w_per_m2_k: 2000.0,
            thermocouple_time_constant_s: 0.5,
        };
        let element = Element::new(fitted);
        let n = element.mass.len();
        // March a constant element power of 300 W far past every time constant.
        let power_w = 300.0;
        let mut t = vec![300.0; n + 1];
        let mut old = t.clone();
        for _ in 0..4000 {
            old.copy_from_slice(&t);
            element.step(&mut t, &old, 2.0, power_w, 300.0, 0.0).expect("step");
        }
        let (flux, _) = water_flux(t[n], 300.0);
        assert!((flux * element.surface_area - power_w).abs() / power_w < 1e-6, "steady");
        // Closed form: from the clad's surface inwards.
        let k_c = CLAD_CONDUCTIVITY_W_PER_M_K;
        let k_m = MEAT_CONDUCTIVITY_W_PER_M_K;
        let two_pi_l = 2.0 * std::f64::consts::PI * MEAT_LENGTH_M;
        let clad_inner = t[n] + power_w * (CLAD_RADIUS_M / GAP_RADIUS_M).ln() / (k_c * two_pi_l);
        let meat_surface = clad_inner + power_w / (fitted.gap_conductance_w_per_m2_k * two_pi_l * MEAT_RADIUS_M);
        let q = power_w / meat_volume_m3();
        let a = ROD_RADIUS_M;
        let b = MEAT_RADIUS_M;
        let at = |r: f64| meat_surface + q / (4.0 * k_m) * (b * b - r * r) - q * a * a / (2.0 * k_m) * (b / r).ln();
        let exact = at(THERMOCOUPLE_RADIUS_M);
        let computed = element.probe_temperature(&t);
        assert!((computed - exact).abs() < 0.02 * (exact - t[n]), "{computed} {exact}");
    }

    #[test]
    fn the_energy_account_closes_on_a_pulse() {
        let answer = solve_with(
            &pulse(15.0, 12.0, 0.01),
            Fitted {
                gap_conductance_w_per_m2_k: 800.0,
                thermocouple_time_constant_s: 1.5,
            },
        )
        .expect("solved");
        assert!(answer.largest_step_energy_residual <= STEP_ENERGY_TOLERANCE);
        assert!(answer.run_energy_residual <= RUN_ENERGY_TOLERANCE);
        let balance = answer.energy_stored_j - (answer.element_energy_deposited_j - answer.energy_to_water_j);
        assert!(balance.abs() <= 1e-9 * answer.element_energy_deposited_j);
        assert!(answer.peak_thermocouple_temperature_k > 400.0);
        assert!(answer.peak_thermocouple_time_s > 1.0);
    }

    #[test]
    fn same_input_same_bits() {
        let a = solve(&pulse(10.0, 3.0, 0.01)).expect("a");
        let b = solve(&pulse(10.0, 3.0, 0.01)).expect("b");
        assert_eq!(a, b);
    }

    #[test]
    fn the_limits_are_named() {
        let mut question = pulse(10.0, 2.0, 0.01);
        question.start_temperature_k = SATURATION_TEMPERATURE_K;
        assert!(matches!(solve(&question), Err(FuelRodReject::StartTemperatureOutsideLiquidPool { .. })));
        question.start_temperature_k = POOL_FREEZING_K;
        assert!(matches!(solve(&question), Err(FuelRodReject::StartTemperatureOutsideLiquidPool { .. })));
        let mut falls = pulse(10.0, 2.0, 0.01);
        falls.core_energy_released_mws[60] = falls.core_energy_released_mws[59] - 0.5;
        assert!(matches!(solve(&falls), Err(FuelRodReject::EnergyFalls { index: 60, .. })));
        let mut question = pulse(10.0, 2.0, 0.01);
        question.time_s[10] = question.time_s[9];
        assert!(matches!(solve(&question), Err(FuelRodReject::HistoryTimesNotRising { index: 10, .. })));
        let mut question = pulse(10.0, 2.0, 0.01);
        question.time_s[3] = f64::NAN;
        assert!(matches!(solve(&question), Err(FuelRodReject::InputNotFinite { field: "time_s" })));
        let mut question = pulse(10.0, 2.0, 0.01);
        question.core_energy_released_mws.pop();
        assert!(matches!(solve(&question), Err(FuelRodReject::HistoryShape { .. })));
        let mut question = pulse(10.0, 2.0, 0.01);
        question.time_s.push(HISTORY_SPAN_MAX_S * 1.001);
        question.core_energy_released_mws.push(10.0);
        assert!(matches!(solve(&question), Err(FuelRodReject::HistoryBeyondCapacity { .. })));
        // A pulse four times the largest compared heats the meat past 1150 degC.
        assert!(matches!(solve(&pulse(100.0, 2.0, 0.01)), Err(FuelRodReject::FuelAboveSafetyLimit { .. })));
    }

    /// Past the compared span the same equations answer, with the flag of
    /// that quantity set.
    #[test]
    fn past_the_compared_span_it_answers_and_flags() {
        let inside = solve(&pulse(12.0, 3.0, 0.01)).expect("inside");
        assert_eq!(
            [inside.start_temperature_outside_compared, inside.core_energy_outside_compared, inside.core_power_outside_compared, inside.history_outside_compared],
            [0.0; 4]
        );
        let mut warm = pulse(12.0, 3.0, 0.01);
        warm.start_temperature_k = 330.0;
        let warm = solve(&warm).expect("warm pool");
        assert_eq!(warm.start_temperature_outside_compared, 1.0);
        let coarse = solve(&pulse(12.0, 30.0, 0.1)).expect("coarse history");
        assert_eq!(coarse.history_outside_compared, 1.0);
        assert!(coarse.run_energy_residual <= RUN_ENERGY_TOLERANCE);
        let strong = solve(&pulse(30.0, 3.0, 0.01)).expect("a strong pulse");
        assert_eq!((strong.core_energy_outside_compared, strong.core_power_outside_compared), (1.0, 1.0));
        assert!(strong.peak_fuel_temperature_k < FUEL_TEMPERATURE_LIMIT_K);
    }

    #[test]
    fn the_critical_heat_flux_is_zubers() {
        let q = critical_heat_flux_w_per_m2();
        assert!((q - 1.47e6).abs() < 0.01e6, "{q}");
    }

    /// A ledger broken on purpose is named: an account checked against a
    /// stored energy that leaves a cell out.
    #[test]
    fn a_broken_account_is_named() {
        let element = Element::new(FITTED);
        let n = element.mass.len();
        let mut t = vec![300.0; n + 1];
        let old = t.clone();
        element.step(&mut t, &old, 1e-3, 5.0e6, 300.0, 1e-3).expect("step");
        let full = element.stored(&t[..n]) - element.stored(&old[..n]);
        let short = element.stored(&t[..n - 1]) - element.stored(&old[..n - 1]);
        let q_in = 5.0e6 * 1e-3;
        let residual_full = (full - q_in).abs() / q_in;
        let residual_short = (short - q_in).abs() / q_in;
        assert!(residual_full < 1e-9, "{residual_full}");
        assert!(residual_short > STEP_ENERGY_TOLERANCE, "{residual_short}");
    }
}
