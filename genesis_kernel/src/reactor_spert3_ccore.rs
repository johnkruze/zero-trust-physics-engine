#![no_implicit_prelude]

//! The SPERT III C-core, step transients from room temperature at system
//! pressures of 0 to 2500 psig and coolant flows of 0 to 20,000 gpm, with
//! the plate surface temperature at the hot spot as state, from the step
//! through the first maximum of power to the first maximum of the hot-spot
//! plate temperature after it.
//!
//! Measured by the Phillips Petroleum Company for the Atomic Energy
//! Commission: IDO-16918, C. R. Toole, "The Effects of Pressure and Flow on
//! Room Temperature Power Excursions in Spert III" (June 1964), OSTI 4042264.
//! Its appendix tabulates 126 step transients: pressure, initial
//! temperature, flow, period, peak power, energy to the time of peak power
//! and the maximum fuel-plate surface temperature, read by 5-mil
//! chromel-alumel thermocouples spot-welded to a plate at the flux peak of a
//! central assembly. The temperature is a measured plate SURFACE
//! temperature, not a fuel temperature. Core constants from IDO-17138
//! (Condit, Scott and Johnson, March 1967), Table B-1. No measured value of
//! either report is stored in the card.
//!
//! ```text
//! power            dP/dt  = ((ρ − 1) / l*) P + Σ λ_i C_i                 MW per s, ρ in dollars
//! precursors       dC_i/dt = (b_i / l*) P − λ_i C_i                      i = 1 to 6
//! net reactivity   ρ = ρ_0 − R_w(θ_i + T_w) + R_w(θ_i) − D χ (T_p − T_w)
//! average plate    C_p dT_p/dt  = (1 − f_w) P − H (T_p − T_w)
//! average water    C_w dT_w/dt  = f_w P + H (T_p − T_w) − C_w (2 v / L) T_w
//! hot-spot plate   C_p dT_h/dt  = F (1 − f_w) P − H (T_h − T_hw)
//! hot-spot water   C_w dT_hw/dt = F f_w P + H (T_h − T_hw) − C_w (2 v / L) T_hw
//! energy released  dE/dt = P
//! ```
//!
//! `ρ_0` is a step at time zero, recorded by the first landed step and held:
//! a later step with another reactivity is refused by name. `v` is the
//! coolant velocity through the core, the load of each step: a step may carry
//! another flow than the one before, and `H`, the sweep `2 v / L`, `S` and
//! `S_h` are each taken at that step's flow. `θ_i` and the system pressure are
//! the start. T are rises above `θ_i`.
//! `R_w` is the integral of the measured isothermal temperature coefficient
//! `a(T)`: 1.8 cents/degC to 37.78 degC (IDO-16918 Table I at 20 degC;
//! IDO-17138 Table B-1, −1.0 cent/degF at 100 degF), then straight to 5.76
//! cents/degC at 204.44 degC (Table B-1, −3.2 cents/degF at 400 degF). `D χ`
//! is the water displaced by plate expansion above the water temperature:
//! −44.0 cents per percent density decrease (Table B-1) times
//! 3 · 17.3e-6 /K · 0.030 in / 0.128 in. `F` = 2.04 · 1.30, the axial and
//! radial flux peak-to-average ratios (Table B-1): the thermocouples sit at
//! the flux peak of a central assembly. `H = A h(v)`, `A` = 97.84 m² (Table
//! B-1: 30 MW over the average heat flux), `h(v) = √(h_s² + h_DB(v)²)` with
//! `h_DB` the Dittus–Boelter correlation for water at 25 degC between plates
//! 0.128 in apart. `C_p` and `C_w` are the plates and the water between them
//! over that area. `l*` = 3 ms (IDO-16918 Table I); the delayed groups are
//! Keepin's for thermal fission of U-235 (no set is published for this
//! core); the starting power is the report's "about 10 watts".
//!
//! Two constants are fitted, as the protocol allowed
//! (`evidence/research/reactor_spert3_ccore/validation_protocol.md`):
//! [`STAGNANT_CONDUCTANCE_W_PER_M2_K`] (`h_s`) and
//! [`WATER_DIRECT_HEATING_FRACTION`] (`f_w`), on 72 rows; 39 rows were held
//! back, among them the whole 1000 psig series. `f_w` came out at 0.55: an
//! effective share that takes up the feedback the card leaves out (boiling
//! and void), far above the few percent of fission energy that gamma rays
//! and neutrons put into the water. The held-back comparison missed its
//! marks; `RESULT.md` of the research record states by how much.
//!
//! The card has no boiling, no void, no melting of the plate, no pressure term
//! (the system pressure is a start field that sets only where the water at
//! the start boils), no heat conduction within the plate and no thermocouple
//! response; it ends at the first maximum of the hot-spot plate temperature,
//! and it is one core.
//!
//! Compared range, the span the fit and held-back rows cover: initial
//! temperature 21 to 36 degC; pressure 0 to 2500 psig with no flow, and flow
//! 200 to 20,000 gpm (0.054864 to 5.4864 m/s) only at 230 psig; the
//! reactivity of the inhour equation (with this card's `l*` and groups) for
//! the longest and the shortest compared period: 9.3 s to 9.7 ms (0.412 to
//! 1.305 dollars) with no flow, 516 ms to 9.7 ms (0.870 to 1.305 dollars)
//! with flow; to the plate maximum or 200 s. Compared once with 39 held-back
//! transients, it failed all three lines: peak power 33.3 percent, energy to
//! peak 56.7 percent, plate surface temperature rise 71.0 percent (pass line
//! 20 percent). On the held-back 1000 psig series peak power read 13.5
//! percent, energy 11.7 percent and temperature rise 67.9 percent low: the
//! card gives the same transient at every pressure, where the measured energy
//! to peak doubles from 0 to 2500 psig. Where the hot-spot plate temperature
//! has no maximum before 200 s (long periods, and flow), the comparison took
//! the rise reached at 200 s; those 11 held-back rows read 2.2 times high on
//! average. Flow was compared at 200 to 20,000 gpm at 230 psig only; the
//! Dittus–Boelter term is a turbulent correlation, blended with the fitted
//! stagnant conductance; no compared transient changed its flow. The water
//! properties of the heat transfer are those of 25 degC. The largest energy
//! the card reaches inside the compared range is about 58,000 MW s (53,000
//! from 36 degC), at the slowest compared flow insertion with the most flow,
//! by 200 s; the largest power about 1050 MW.
//!
//! Outside the compared range the card computes the same equations, with
//! nothing compared: from any start of liquid water (273.15 K up to the
//! saturation temperature of water at the system pressure, IAPWS-IF97 region
//! 4, or the critical temperature 647.096 K above the critical pressure
//! 22.064 MPa), at any absolute pressure above zero, any flow of zero or
//! more, any reactivity above zero, for any time. The isothermal coefficient
//! law holds to 204.44 degC and continues straight past it.
//!
//! The ledger has three balances, each closed on every step:
//!
//! ```text
//! neutrons  N − N_0 − (ρ_0 E − Q) / l*,         N = P + Σ C_i,  Q = ∫ (ρ_0 − ρ) P dt
//! energy    E − C_p T_p − C_w T_w − S,          S  = ∫ C_w (2 v / L) T_w dt
//! hot spot  F E − C_p T_h − C_w T_hw − S_h,     S_h = ∫ C_w (2 v / L) T_hw dt
//! ```
//!
//! `Q`, `S` and `S_h` are taken over each step by the two-point Hermite rule
//! from the values and rates at the step's two ends; the Runge–Kutta step
//! never evaluates them, so a wrong method or a falsified term shows in the
//! neutron balance. The two energy balances are linear invariants of the
//! equations and hold to rounding under any Runge–Kutta method; they name a
//! falsified heat path, not a method. The residual is the largest of the
//! three, each relative to its own scale. It shows the arithmetic is right;
//! it cannot show the physics matches the reactor.
//!
//! [`ReactorSpert3Ccore::gate_dt_s`] is 0.2 l* = 0.6 ms;
//! [`ReactorSpert3Ccore::floor_dt_s`] is 1e-9 s. The public step applies
//! neither. The step applies a ceiling of its own that follows the state: at
//! the state it starts from and at its trial end, it refuses as
//! `DtAboveStateCeiling` a step that would advance a pole further than a step
//! of 0.6 ms ever does inside the compared range: the growing prompt pole
//! `(ρ_net − 1) / l*` by 0.0611 (at 1.305 dollars), the decaying prompt pole
//! `(1 − ρ_net) / l*` by 0.5 (at −1.48 dollars net), and the thermal rates
//! `H / C_p + H / C_w + 2 v / L`, whose sum no thermal pole is faster than,
//! by 0.0184 (at 5.4864 m/s). Inside the compared range a step at 0.6 ms
//! never reaches it; past it, the state ceiling binds: above about 1.31
//! dollars net, below about −1.5 dollars net, and above about 5.5 m/s.
//! The three phases are where a 0.6 ms step stands inside the compared range;
//! they are not yet worked out from the step's own error. Measured with the
//! card's own step at the state ceiling, the ledger closes on bursts to 1.5
//! dollars and on flows to 300 m/s; on bursts of 2 dollars and more it names
//! a step after the power peak, where the feedback turns the power faster
//! than the prompt poles say and the decaying prompt pole carries the power's
//! fall, and there a shorter step closes it to the plate maximum: 0.1 ms at 2
//! dollars, 0.023 ms at 5 dollars, 0.001 ms at 30 dollars.
//! `core` only, with `exp` and `log` from [`crate::math`] and `sqrt` from the
//! platform.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

/// Delayed-neutron group fractions `b_i`, fractions of the total; sum 1.
/// G. R. Keepin, Physics of Nuclear Kinetics (1965), thermal fission of U-235.
pub const GROUP_FRACTION: [f64; 6] = [0.033, 0.219, 0.196, 0.395, 0.115, 0.042];
/// Delayed-neutron decay constants `λ_i`, per second. Same source.
pub const GROUP_DECAY_PER_S: [f64; 6] = [0.0124, 0.0305, 0.111, 0.301, 1.14, 3.01];
/// Reduced prompt-neutron generation time `l / β_eff`, s. IDO-16918 Table I.
pub const GENERATION_TIME_REDUCED_S: f64 = 0.003;
/// Power at the step, MW: "about 10 watts" (IDO-16918 p. 7).
pub const INITIAL_POWER_MW: f64 = 1.0e-5;
/// Flux peak-to-average ratio at the thermocouples, axial 2.04 times radial
/// 1.30. IDO-17138 Table B-1.
pub const HOT_SPOT_PEAKING: f64 = 2.04 * 1.30;
/// Plate heat-transfer area, m²: 30 MW over the average core heat flux
/// 9.72e4 Btu/hr ft² (IDO-17138 Table B-1).
pub const HEAT_TRANSFER_AREA_M2: f64 = 97.84;
/// Heat capacity of the plates over that area, MW s per K: 0.030 in thick,
/// 1904 plates of 16.8 g U-235 at 93 % as UO2 (10.96 g/cm³, 0.235 J/g K),
/// the rest type 304L stainless steel (7.90 g/cm³, 0.500 J/g K).
pub const PLATE_HEAT_CAPACITY_MWS_PER_K: f64 = 0.14235;
/// Heat capacity of the water between the plates over that area, MW s per K:
/// channels 0.128 in wide, water at 25 degC (997.05 kg/m³, 4181.3 J/kg K).
pub const WATER_HEAT_CAPACITY_MWS_PER_K: f64 = 0.66306;
/// Active length, m: 36 in.
pub const ACTIVE_LENGTH_M: f64 = 0.9144;
/// Reactivity per unit fractional decrease of moderator density, dollars:
/// −44.0 cents per percent (IDO-17138 Table B-1), sign removed.
pub const DENSITY_COEFFICIENT_DOLLAR: f64 = 44.0;
/// Water displaced per kelvin of plate temperature above the water:
/// 3 × 17.3e-6 /K (type 304 stainless steel) × 0.030 in plate / 0.128 in channel.
pub const PLATE_DISPLACEMENT_PER_K: f64 = 3.0 * 17.3e-6 * 0.030 / 0.128;
/// Temperature at which the measured isothermal coefficient starts to grow, degC (100 degF).
pub const COEFFICIENT_KNEE_C: f64 = 37.78;
/// Isothermal temperature coefficient up to the knee, dollars per K, sign removed: 1.8 cents/degC.
pub const COEFFICIENT_COLD_DOLLAR_PER_K: f64 = 0.018;
/// Temperature of the second measured coefficient, degC (400 degF).
pub const COEFFICIENT_HOT_C: f64 = 204.44;
/// Isothermal coefficient there, dollars per K, sign removed: 3.2 cents/degF.
pub const COEFFICIENT_HOT_DOLLAR_PER_K: f64 = 0.0576;
/// Hydraulic diameter of the channel between two plates, m: 2 × 0.128 in.
pub const HYDRAULIC_DIAMETER_M: f64 = 2.0 * 0.128 * 0.0254;
/// Water at 25 degC for Dittus–Boelter: density kg/m³, viscosity Pa s, conductivity W/m K, Prandtl number.
pub const WATER_DENSITY_KG_PER_M3: f64 = 997.05;
pub const WATER_VISCOSITY_PA_S: f64 = 8.90e-4;
pub const WATER_CONDUCTIVITY_W_PER_M_K: f64 = 0.6071;
pub const WATER_PRANDTL: f64 = 6.13;
/// Plate-to-coolant conductance with no flow, `h_s`, W/m² K. Fitted.
pub const STAGNANT_CONDUCTANCE_W_PER_M2_K: f64 = 6403.73;
/// Share of fission energy deposited directly in the water, `f_w`. Fitted;
/// an effective value (see the header).
pub const WATER_DIRECT_HEATING_FRACTION: f64 = 0.551915;
/// Compared initial temperatures, K: 21 to 36 degC. Not a limit: outside it the card computes the
/// same equations, with nothing compared.
pub const INITIAL_TEMPERATURE_MIN_K: f64 = 21.0 + 273.15;
pub const INITIAL_TEMPERATURE_MAX_K: f64 = 36.0 + 273.15;
/// Compared system pressures with no flow, psig: 0 to 2500. Not a limit.
pub const PRESSURE_MAX_PSIG: f64 = 2500.0;
/// System pressure of every compared flow transient, psig: 230. The protocol's "225 to 250"
/// counted two flow rows (03040 at 225 and 03042 at 250 psig) that are on no line and were never
/// run. Not a limit.
pub const FLOW_PRESSURE_MIN_PSIG: f64 = 230.0;
pub const FLOW_PRESSURE_MAX_PSIG: f64 = 230.0;
/// Compared coolant velocities, m/s: 200 and 20,000 gpm at 18 ft/s per 20,000 gpm (IDO-16918
/// p. 1), each as `gpm × (18 / 20000) × 0.3048` evaluates; the upper one is the double above
/// 5.4864. Not a limit.
pub const FLOW_MIN_M_PER_S: f64 = 0.054864;
pub const FLOW_MAX_M_PER_S: f64 = 5.486400000000001;
/// Smallest compared reactivity with no flow, dollars: the inhour value of 9.3 s. Not a limit.
pub const REACTIVITY_MIN_NO_FLOW_DOLLAR: f64 = 0.4122874385206243;
/// Smallest compared reactivity with flow, dollars: the inhour value of 516 ms. Not a limit.
pub const REACTIVITY_MIN_FLOW_DOLLAR: f64 = 0.870348183353308;
/// Largest compared reactivity, dollars: the inhour value of 9.7 ms. Not a limit.
pub const REACTIVITY_MAX_DOLLAR: f64 = 1.3053996316244825;
/// Time to which the compared runs went where the plate temperature had no maximum before it, s.
/// Not a limit.
pub const COMPARED_TIME_S: f64 = 200.0;
/// Freezing point of water, K: below it the start is not liquid water.
pub const FREEZING_K: f64 = 273.15;
/// One standard atmosphere, psi: the gauge pressure of absolute zero is minus this.
pub const ATMOSPHERE_PSI: f64 = 14.6959488;
/// One psi in MPa.
pub const MPA_PER_PSI: f64 = 0.00689475729;
/// Critical pressure of water, MPa (IAPWS-IF97). Above it the upper limit of a liquid start is
/// the critical temperature.
pub const CRITICAL_PRESSURE_MPA: f64 = 22.064;
/// Critical temperature of water, K (IAPWS-IF97).
pub const CRITICAL_TEMPERATURE_K: f64 = 647.096;
/// IAPWS-IF97 region 4, the coefficients `n_1` to `n_10` of the saturation-pressure equation,
/// read by its backward equation (eq. 31) for the saturation temperature.
pub const SATURATION_N: [f64; 10] = [
    0.11670521452767e4,
    -0.72421316598320e6,
    -0.17073846940092e2,
    0.12020824702470e5,
    -0.32325550322333e7,
    0.14915108613530e2,
    -0.48232657361591e4,
    0.40511340542057e6,
    -0.23855557567849,
    0.65017534844798e3,
];
/// The ledger's residual must stay at or under this: 11 times the largest the card's own step
/// left at the ceiling over the 67 fit rows run and eight runs at the corners of the range
/// (2.24e-8, the shortest period at the most flow). It names a wrong method; it is not a bound on
/// the error of the state.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.5e-7;
/// Fraction of `l*` one step at the ceiling advances.
pub const GATE_PHASE_STEP: f64 = 0.2;
/// The state ceiling holds each step to the phases a step at the static ceiling, 0.6 ms, reaches
/// inside the compared range, each rounded up. Growing prompt pole `(ρ_net − 1) / l*`: 0.0611, at the
/// largest compared reactivity, 1.305 dollars (0.0610799).
pub const GROWTH_PHASE_STEP: f64 = 0.0611;
/// Decaying prompt pole `(1 − ρ_net) / l*`: 0.5, at the most negative net reactivity inside the
/// compared range, −1.48 dollars (21 degC, no flow, 1.099 dollars, at 200 s: 0.496).
pub const DECAY_PHASE_STEP: f64 = 0.5;
/// Plate and water heat transfer and sweep, `H / C_p + H / C_w + 2 v / L`: 0.0184, at the largest
/// compared flow, 5.4864 m/s (30.56 per s: 0.01834).
pub const THERMAL_PHASE_STEP: f64 = 0.0184;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-9;

/// Net reactivity is above one dollar on this frame.
pub const FLAG_PROMPT_SUPERCRITICAL: u64 = 1 << 0;
/// The first maximum of power has been passed.
pub const FLAG_PAST_POWER_PEAK: u64 = 1 << 1;

/// Why a step did not advance the reactor or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt`, the reactivity or the flow was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this reactor's.
    InvalidCard,
    /// The system pressure is at or below absolute zero: a gauge pressure at or below −14.6959488 psig.
    PressureNotPositive,
    /// The initial temperature is not that of liquid water at the system pressure: below 273.15 K, or at or above the saturation temperature of water at that pressure (IAPWS-IF97 region 4), or above the critical pressure, 22.064 MPa, at or above the critical temperature, 647.096 K.
    InitialTemperatureNotLiquidWater,
    /// The coolant flow is below zero.
    NegativeFlow,
    /// The held reactivity is zero or less: no step in reactivity is inserted, and there is no burst.
    ReactivityNotPositive,
    /// The state is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card.
    PastPlateTemperatureMaximum,
    /// The incoming state's clock is below zero.
    NegativeTime,
    /// The incoming state holds an energy, a temperature rise, a swept energy, a precursor sum or a flow below zero, or a power not above zero.
    NegativeState,
    /// The step is longer than the state lets it be, at the state it starts from or at its trial end: one Runge–Kutta step would advance the growing prompt pole (net reactivity − 1) / l* by more than 0.0611 of its time constant, the decaying prompt pole (1 − net reactivity) / l* by more than 0.5, or the plate and water heat transfer and sweep by more than 0.0184.
    DtAboveStateCeiling,
    /// The reactivity of the load is not the one of the first step of this run; a run is one step of reactivity.
    ReactivityChangedDuringRun,
    /// The incoming state is not one this card writes: at time zero it is not the card's start; later, a derived field, a flag or a peak field is not what the card computes from the rest of the state.
    InconsistentState,
    /// The trial step created neutrons or energy beyond what the ledger supplies.
    SpontaneousGeneration,
    /// The trial step destroyed neutrons or energy the ledger supplies.
    FirstLawBreach,
}

/// What the operator sets for one transient, recorded by the first landed step and held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorLoad {
    /// Dollars of reactivity stepped in at time zero, `ρ_0`.
    pub reactivity_inserted_dollar: f64,
    /// Coolant velocity through the core, `v`; 20,000 gpm is 18 ft/s.
    pub coolant_flow_m_per_s: f64,
}

/// The locked reactor. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorSpert3Ccore {
    pub group_fraction: [f64; 6],
    pub group_decay_per_s: [f64; 6],
    pub constants: [f64; 41],
}

/// The scalar constants in one list, in the order `is_valid` and `terms` read them.
const SCALARS: [f64; 41] = [
    GENERATION_TIME_REDUCED_S,
    INITIAL_POWER_MW,
    HOT_SPOT_PEAKING,
    HEAT_TRANSFER_AREA_M2,
    PLATE_HEAT_CAPACITY_MWS_PER_K,
    WATER_HEAT_CAPACITY_MWS_PER_K,
    ACTIVE_LENGTH_M,
    DENSITY_COEFFICIENT_DOLLAR,
    PLATE_DISPLACEMENT_PER_K,
    COEFFICIENT_KNEE_C,
    COEFFICIENT_COLD_DOLLAR_PER_K,
    COEFFICIENT_HOT_C,
    COEFFICIENT_HOT_DOLLAR_PER_K,
    HYDRAULIC_DIAMETER_M,
    WATER_DENSITY_KG_PER_M3,
    WATER_VISCOSITY_PA_S,
    WATER_CONDUCTIVITY_W_PER_M_K,
    WATER_PRANDTL,
    STAGNANT_CONDUCTANCE_W_PER_M2_K,
    WATER_DIRECT_HEATING_FRACTION,
    FREEZING_K,
    ATMOSPHERE_PSI,
    MPA_PER_PSI,
    CRITICAL_PRESSURE_MPA,
    CRITICAL_TEMPERATURE_K,
    SATURATION_N[0],
    SATURATION_N[1],
    SATURATION_N[2],
    SATURATION_N[3],
    SATURATION_N[4],
    SATURATION_N[5],
    SATURATION_N[6],
    SATURATION_N[7],
    SATURATION_N[8],
    SATURATION_N[9],
    RESIDUAL_CONSERVATION_LIMIT,
    GATE_PHASE_STEP,
    GROWTH_PHASE_STEP,
    DECAY_PHASE_STEP,
    THERMAL_PHASE_STEP,
    FLOOR_DT_S,
];

const K_L_STAR: usize = 0;
const K_P0: usize = 1;
const K_PEAKING: usize = 2;
const K_AREA: usize = 3;
const K_CP: usize = 4;
const K_CW: usize = 5;
const K_LENGTH: usize = 6;
const K_DENSITY: usize = 7;
const K_DISPLACEMENT: usize = 8;
const K_KNEE: usize = 9;
const K_COLD: usize = 10;
const K_HOT_T: usize = 11;
const K_HOT: usize = 12;
const K_DH: usize = 13;
const K_RHO_W: usize = 14;
const K_MU_W: usize = 15;
const K_K_W: usize = 16;
const K_PR_W: usize = 17;
const K_HS: usize = 18;
const K_FW: usize = 19;
const K_FREEZING: usize = 20;
const K_ATMOSPHERE: usize = 21;
const K_MPA_PER_PSI: usize = 22;
const K_CRITICAL_P: usize = 23;
const K_CRITICAL_T: usize = 24;
const K_SATURATION: usize = 25;
const K_RESIDUAL: usize = 35;
const K_GATE: usize = 36;
const K_GROWTH_PHASE: usize = 37;
const K_DECAY_PHASE: usize = 38;
const K_THERMAL_PHASE: usize = 39;
const K_FLOOR: usize = 40;

impl ReactorSpert3Ccore {
    /// Card: the SPERT III C-19/52 core, step transients from room temperature. No arguments.
    #[must_use]
    pub fn c_core_step_transients() -> Self {
        let reactor = Self {
            group_fraction: GROUP_FRACTION,
            group_decay_per_s: GROUP_DECAY_PER_S,
            constants: SCALARS,
        };
        if reactor.is_valid() {
            reactor
        } else {
            ::core::panic!("C-core reactor failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        let mut index = 0;
        while index < 6 {
            if self.group_fraction[index].to_bits() != GROUP_FRACTION[index].to_bits()
                || self.group_decay_per_s[index].to_bits() != GROUP_DECAY_PER_S[index].to_bits()
            {
                return false;
            }
            index += 1;
        }
        index = 0;
        while index < SCALARS.len() {
            if self.constants[index].to_bits() != SCALARS[index].to_bits() {
                return false;
            }
            index += 1;
        }
        true
    }

    /// Reactivity in dollars whose stable period is `period_s`, by the inhour equation with this
    /// card's `l*` and groups, summed in group order.
    #[must_use]
    pub fn inhour_dollar(&self, period_s: f64) -> f64 {
        let mut sum = self.constants[K_L_STAR] / period_s;
        let mut index = 0;
        while index < 6 {
            sum += self.group_fraction[index] / (1.0 + self.group_decay_per_s[index] * period_s);
            index += 1;
        }
        sum
    }

    /// Plate-to-coolant conductance at a coolant velocity, W/m² K: `√(h_s² + h_DB(v)²)`.
    #[must_use]
    pub fn conductance_w_per_m2_k(&self, coolant_flow_m_per_s: f64) -> f64 {
        let c = &self.constants;
        let forced = if coolant_flow_m_per_s > 0.0 {
            let reynolds = c[K_RHO_W] * coolant_flow_m_per_s * c[K_DH] / c[K_MU_W];
            0.023
                * crate::math::exp(0.8 * crate::math::log(reynolds))
                * crate::math::exp(0.4 * crate::math::log(c[K_PR_W]))
                * c[K_K_W]
                / c[K_DH]
        } else {
            0.0
        };
        (c[K_HS] * c[K_HS] + forced * forced).sqrt()
    }

    /// Largest step the card stands behind at every state of the compared range:
    /// `GATE_PHASE_STEP · l*`, 0.6 ms, the time constant of the prompt pole at zero net
    /// reactivity, where every run starts. Not applied by [`step`]. Past the compared range the
    /// state ceiling of [`step`] binds as well (`DtAboveStateCeiling`, [`state_dt_ceiling_s`]).
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let l_star = self.constants[K_L_STAR];
        let gate = self.constants[K_GATE];
        if l_star.is_finite() && l_star > 0.0 && gate > 0.0 {
            Some(gate * l_star)
        } else {
            None
        }
    }

    /// Smallest step the card stands behind: [`FLOOR_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        let floor = self.constants[K_FLOOR];
        if floor.is_finite() && floor > 0.0 {
            Some(floor)
        } else {
            None
        }
    }

    /// Absolute pressure in MPa of a gauge pressure in psig: `(psig + 14.6959488) · 0.00689475729`.
    #[must_use]
    pub fn absolute_pressure_mpa(&self, system_pressure_psig: f64) -> f64 {
        (system_pressure_psig + self.constants[K_ATMOSPHERE]) * self.constants[K_MPA_PER_PSI]
    }

    /// Whether a system pressure is above absolute zero.
    #[must_use]
    pub fn pressure_is_positive(&self, system_pressure_psig: f64) -> bool {
        self.absolute_pressure_mpa(system_pressure_psig) > 0.0
    }

    /// The saturation temperature of water at a gauge pressure, K: IAPWS-IF97 region 4, backward
    /// equation (eq. 31), with `p` in MPa absolute; at or above the critical pressure, 22.064 MPa,
    /// the critical temperature 647.096 K. `+ − × /` and `sqrt` only. Not finite where the
    /// equation has no real answer, at a pressure far under the triple point.
    #[must_use]
    pub fn saturation_temperature_k(&self, system_pressure_psig: f64) -> f64 {
        let c = &self.constants;
        let p = self.absolute_pressure_mpa(system_pressure_psig);
        if p >= c[K_CRITICAL_P] {
            return c[K_CRITICAL_T];
        }
        let n = |i: usize| c[K_SATURATION + i - 1];
        let beta = p.sqrt().sqrt();
        let beta2 = beta * beta;
        let e = beta2 + n(3) * beta + n(6);
        let f = n(1) * beta2 + n(4) * beta + n(7);
        let g = n(2) * beta2 + n(5) * beta + n(8);
        let d = 2.0 * g / (-f - (f * f - 4.0 * e * g).sqrt());
        let sum = n(10) + d;
        (sum - (sum * sum - 4.0 * (n(9) + n(10) * d)).sqrt()) / 2.0
    }

    /// Whether a start at this temperature and gauge pressure is liquid water: at or above
    /// 273.15 K and below the saturation temperature at that pressure (or the critical
    /// temperature, above the critical pressure).
    #[must_use]
    pub fn initial_temperature_is_liquid_water(&self, initial_temperature_k: f64, system_pressure_psig: f64) -> bool {
        initial_temperature_k >= self.constants[K_FREEZING]
            && initial_temperature_k < self.saturation_temperature_k(system_pressure_psig)
    }

    /// Whether an initial temperature is in the compared 21 to 36 degC. Not a limit.
    #[must_use]
    pub fn initial_temperature_compared(&self, initial_temperature_k: f64) -> bool {
        initial_temperature_k >= INITIAL_TEMPERATURE_MIN_K && initial_temperature_k <= INITIAL_TEMPERATURE_MAX_K
    }
}

/// The rates of the poles at `y` under these terms, per second: the growing prompt pole
/// `(ρ_net − 1) / l*` (zero when `ρ_net` is at or under one dollar), the decaying prompt pole
/// `(1 − ρ_net) / l*` (zero when it is at or over one dollar), and `H / C_p + H / C_w + 2 v / L`, the
/// trace of the plate and water pair, which no pole of that pair is faster than.
fn pole_rates_per_s(terms: &Terms, y: &[f64; 12]) -> [f64; 3] {
    let rho_net = terms.rho_0 - feedback(terms, y);
    let excess = (rho_net - 1.0) / terms.l_star;
    let h = terms.conductance_mw_per_k;
    let thermal = h / terms.cp + h / terms.cw + terms.sweep;
    if excess > 0.0 {
        [excess, 0.0, thermal]
    } else {
        [0.0, -excess, thermal]
    }
}

/// The longest step the state ceiling lets cover `y`: the least of each pole's phase over its rate.
fn state_ceiling(reactor: &ReactorSpert3Ccore, terms: &Terms, y: &[f64; 12]) -> f64 {
    let c = &reactor.constants;
    let rates = pole_rates_per_s(terms, y);
    let phases = [c[K_GROWTH_PHASE], c[K_DECAY_PHASE], c[K_THERMAL_PHASE]];
    let mut ceiling = f64::INFINITY;
    let mut index = 0;
    while index < 3 {
        if rates[index] > 0.0 {
            let longest = phases[index] / rates[index];
            if longest < ceiling {
                ceiling = longest;
            }
        }
        index += 1;
    }
    ceiling
}

/// The longest step the state ceiling of [`step`] lets start from `state` under `load`, s: the
/// least of [`GROWTH_PHASE_STEP`], [`DECAY_PHASE_STEP`] and [`THERMAL_PHASE_STEP`], each over the
/// rate of its pole there. The trial end of a step is held to the same ceiling. `None` when the
/// card or the state is not one the card holds.
#[must_use]
pub fn state_dt_ceiling_s(reactor: &ReactorSpert3Ccore, state: &ReactorState, load: ReactorLoad) -> Option<f64> {
    if !reactor.is_valid() || !state_is_finite(state) {
        return None;
    }
    let terms = terms(reactor, state.initial_temperature_k, load);
    let ceiling = state_ceiling(reactor, &terms, &state_vector(state));
    if ceiling > 0.0 && ceiling.is_finite() {
        Some(ceiling)
    } else {
        None
    }
}

/// Integrated reactor. The ledger, the derived fields and the peak fields are part of the state;
/// write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorState {
    /// Time since the step.
    pub t_s: f64,
    /// Reactor power, `P`.
    pub power_mw: f64,
    /// Delayed-neutron precursor sums, `C_i`.
    pub precursor_mw: [f64; 6],
    /// Core-average plate temperature rise, `T_p`.
    pub plate_rise_k: f64,
    /// Core-average water temperature rise, `T_w`.
    pub water_rise_k: f64,
    /// Hot-spot plate surface temperature rise, `T_h`: the quantity the thermocouples measured.
    pub hot_spot_plate_rise_k: f64,
    /// Hot-spot water temperature rise, `T_hw`.
    pub hot_spot_water_rise_k: f64,
    /// Energy released since the step, `E`.
    pub energy_mws: f64,
    /// Initial temperature of plates and water, `θ_i`.
    pub initial_temperature_k: f64,
    /// System pressure, psig. No equation of the transient reads it; it sets where the water at the start boils.
    pub system_pressure_psig: f64,
    /// `dP/dt` at this state under the run's load.
    pub power_rate_mw_per_s: f64,
    /// `dT_h/dt` at this state under the run's load.
    pub hot_spot_plate_rate_k_per_s: f64,
    /// Net reactivity at this state under the run's load. Zero at the start.
    pub reactivity_net_dollar: f64,
    /// `N = P + Σ C_i` at the start.
    pub population_anchor_mw: f64,
    /// `∫ (ρ_0 − ρ) P dt` since the start, by the Hermite rule.
    pub feedback_integral_dollar_mws: f64,
    /// `∫ C_w (2 v / L) T_w dt`, by the Hermite rule.
    pub swept_energy_mws: f64,
    /// `∫ C_w (2 v / L) T_hw dt`, by the Hermite rule.
    pub hot_spot_swept_energy_mws: f64,
    /// Signed neutron balance; positive is created population.
    pub population_defect_mw: f64,
    /// Signed energy balance; positive is created energy.
    pub energy_defect_mws: f64,
    /// Signed hot-spot energy balance; positive is created energy.
    pub hot_spot_energy_defect_mws: f64,
    /// Largest of the three balances, each relative to its own scale.
    pub residual_conservation_error: f64,
    /// Highest landed power before the first maximum of power was passed.
    pub peak_power_mw: f64,
    /// Energy released at that sample.
    pub peak_energy_mws: f64,
    /// Time of that sample.
    pub peak_time_s: f64,
    /// Highest landed hot-spot plate surface temperature rise.
    pub plate_rise_max_k: f64,
    /// Time of that sample.
    pub plate_rise_max_time_s: f64,
    /// Reactivity of the first landed step's load, `ρ_0`. Zero at the start.
    pub run_reactivity_dollar: f64,
    /// Coolant flow of the last landed step's load; the derived fields of the state are taken at it. Zero at the start.
    pub run_coolant_flow_m_per_s: f64,
    /// Set by the first landed step on which the power is no longer rising.
    pub past_power_peak: bool,
    /// Set by the first landed step after the power peak on which the hot-spot plate temperature is no longer rising. The next step is refused.
    pub past_plate_maximum: bool,
    /// Net reactivity above one dollar.
    pub prompt_supercritical: bool,
}

impl ReactorState {
    /// The reactor in equilibrium at [`INITIAL_POWER_MW`] with every rise zero, time 0. `None`
    /// unless the initial temperature and the pressure are finite, the pressure is above absolute
    /// zero and the start is liquid water.
    #[must_use]
    pub fn try_start(
        reactor: &ReactorSpert3Ccore,
        initial_temperature_k: f64,
        system_pressure_psig: f64,
    ) -> Option<Self> {
        if !reactor.is_valid()
            || !initial_temperature_k.is_finite()
            || !system_pressure_psig.is_finite()
            || !reactor.pressure_is_positive(system_pressure_psig)
            || !reactor.initial_temperature_is_liquid_water(initial_temperature_k, system_pressure_psig)
        {
            return None;
        }
        let l_star = reactor.constants[K_L_STAR];
        let power_mw = reactor.constants[K_P0];
        let mut precursor_mw = [0.0; 6];
        let mut index = 0;
        while index < 6 {
            precursor_mw[index] =
                reactor.group_fraction[index] * power_mw / (l_star * reactor.group_decay_per_s[index]);
            index += 1;
        }
        let mut state = Self {
            t_s: 0.0,
            power_mw,
            precursor_mw,
            plate_rise_k: 0.0,
            water_rise_k: 0.0,
            hot_spot_plate_rise_k: 0.0,
            hot_spot_water_rise_k: 0.0,
            energy_mws: 0.0,
            initial_temperature_k,
            system_pressure_psig,
            power_rate_mw_per_s: 0.0,
            hot_spot_plate_rate_k_per_s: 0.0,
            reactivity_net_dollar: 0.0,
            population_anchor_mw: 0.0,
            feedback_integral_dollar_mws: 0.0,
            swept_energy_mws: 0.0,
            hot_spot_swept_energy_mws: 0.0,
            population_defect_mw: 0.0,
            energy_defect_mws: 0.0,
            hot_spot_energy_defect_mws: 0.0,
            residual_conservation_error: 0.0,
            peak_power_mw: power_mw,
            peak_energy_mws: 0.0,
            peak_time_s: 0.0,
            plate_rise_max_k: 0.0,
            plate_rise_max_time_s: 0.0,
            run_reactivity_dollar: 0.0,
            run_coolant_flow_m_per_s: 0.0,
            past_power_peak: false,
            past_plate_maximum: false,
            prompt_supercritical: false,
        };
        state.population_anchor_mw = population_mw(&state);
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }
}

/// The constants of one start and one load, gathered for the step.
#[derive(Clone, Copy)]
struct Terms {
    l_star: f64,
    rho_0: f64,
    initial_c: f64,
    base: f64,
    displacement: f64,
    plate_share: f64,
    water_share: f64,
    peaking: f64,
    conductance_mw_per_k: f64,
    cp: f64,
    cw: f64,
    sweep: f64,
    knee: f64,
    cold: f64,
    slope: f64,
}

fn terms(reactor: &ReactorSpert3Ccore, initial_temperature_k: f64, load: ReactorLoad) -> Terms {
    let c = &reactor.constants;
    let initial_c = initial_temperature_k - 273.15;
    let mut t = Terms {
        l_star: c[K_L_STAR],
        rho_0: load.reactivity_inserted_dollar,
        initial_c,
        base: 0.0,
        displacement: c[K_DENSITY] * c[K_DISPLACEMENT],
        plate_share: 1.0 - c[K_FW],
        water_share: c[K_FW],
        peaking: c[K_PEAKING],
        conductance_mw_per_k: c[K_AREA] * reactor.conductance_w_per_m2_k(load.coolant_flow_m_per_s) / 1.0e6,
        cp: c[K_CP],
        cw: c[K_CW],
        sweep: 2.0 * load.coolant_flow_m_per_s / c[K_LENGTH],
        knee: c[K_KNEE],
        cold: c[K_COLD],
        slope: (c[K_HOT] - c[K_COLD]) / (c[K_HOT_T] - c[K_KNEE]),
    };
    t.base = coefficient_integral(&t, initial_c);
    t
}

/// The isothermal coefficient at `temperature_c`, dollars per K.
fn coefficient(terms: &Terms, temperature_c: f64) -> f64 {
    if temperature_c <= terms.knee {
        terms.cold
    } else {
        terms.cold + terms.slope * (temperature_c - terms.knee)
    }
}

/// The integral of the coefficient from 0 degC to `temperature_c`, dollars.
fn coefficient_integral(terms: &Terms, temperature_c: f64) -> f64 {
    if temperature_c <= terms.knee {
        terms.cold * temperature_c
    } else {
        let above = temperature_c - terms.knee;
        terms.cold * temperature_c + 0.5 * terms.slope * above * above
    }
}

/// `ρ_0 − ρ`: the reactivity the temperatures take back.
fn feedback(terms: &Terms, y: &[f64; 12]) -> f64 {
    coefficient_integral(terms, terms.initial_c + y[8]) - terms.base + terms.displacement * (y[7] - y[8])
}

/// Advance one Runge–Kutta step and write the frame. A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    reactor: &ReactorSpert3Ccore,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(reactor, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    reactor: &ReactorSpert3Ccore,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(reactor, state, load, dt_s)?;
    let terms = terms(reactor, state.initial_temperature_k, load);
    within_state_ceiling(reactor, &terms, &state_vector(state), dt_s)?;
    let advanced = rk4(reactor, &terms, &state_vector(state), dt_s);
    commit(reactor, state, frame, &terms, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut ReactorState) {
    state.t_s = normal_or_zero(state.t_s);
    state.power_mw = normal_or_zero(state.power_mw);
    normal_or_zero_each(&mut state.precursor_mw);
    state.plate_rise_k = normal_or_zero(state.plate_rise_k);
    state.water_rise_k = normal_or_zero(state.water_rise_k);
    state.hot_spot_plate_rise_k = normal_or_zero(state.hot_spot_plate_rise_k);
    state.hot_spot_water_rise_k = normal_or_zero(state.hot_spot_water_rise_k);
    state.energy_mws = normal_or_zero(state.energy_mws);
    state.initial_temperature_k = normal_or_zero(state.initial_temperature_k);
    state.system_pressure_psig = normal_or_zero(state.system_pressure_psig);
    state.power_rate_mw_per_s = normal_or_zero(state.power_rate_mw_per_s);
    state.hot_spot_plate_rate_k_per_s = normal_or_zero(state.hot_spot_plate_rate_k_per_s);
    state.reactivity_net_dollar = normal_or_zero(state.reactivity_net_dollar);
    state.population_anchor_mw = normal_or_zero(state.population_anchor_mw);
    state.feedback_integral_dollar_mws = normal_or_zero(state.feedback_integral_dollar_mws);
    state.swept_energy_mws = normal_or_zero(state.swept_energy_mws);
    state.hot_spot_swept_energy_mws = normal_or_zero(state.hot_spot_swept_energy_mws);
    state.population_defect_mw = normal_or_zero(state.population_defect_mw);
    state.energy_defect_mws = normal_or_zero(state.energy_defect_mws);
    state.hot_spot_energy_defect_mws = normal_or_zero(state.hot_spot_energy_defect_mws);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.peak_power_mw = normal_or_zero(state.peak_power_mw);
    state.peak_energy_mws = normal_or_zero(state.peak_energy_mws);
    state.peak_time_s = normal_or_zero(state.peak_time_s);
    state.plate_rise_max_k = normal_or_zero(state.plate_rise_max_k);
    state.plate_rise_max_time_s = normal_or_zero(state.plate_rise_max_time_s);
    state.run_reactivity_dollar = normal_or_zero(state.run_reactivity_dollar);
    state.run_coolant_flow_m_per_s = normal_or_zero(state.run_coolant_flow_m_per_s);
}

/// The state ceiling: for each pole, `dt_s` times its rate at `y` at most its phase.
fn within_state_ceiling(
    reactor: &ReactorSpert3Ccore,
    terms: &Terms,
    y: &[f64; 12],
    dt_s: f64,
) -> Result<(), StepReject> {
    let c = &reactor.constants;
    let rates = pole_rates_per_s(terms, y);
    if dt_s * rates[0] > c[K_GROWTH_PHASE] || dt_s * rates[1] > c[K_DECAY_PHASE] || dt_s * rates[2] > c[K_THERMAL_PHASE] {
        Err(StepReject::DtAboveStateCeiling)
    } else {
        Ok(())
    }
}

fn validate(
    reactor: &ReactorSpert3Ccore,
    state: &ReactorState,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.reactivity_inserted_dollar.is_finite() || !load.coolant_flow_m_per_s.is_finite() {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !reactor.is_valid() {
        return Err(StepReject::InvalidCard);
    }
    if !reactor.pressure_is_positive(state.system_pressure_psig) {
        return Err(StepReject::PressureNotPositive);
    }
    if !reactor.initial_temperature_is_liquid_water(state.initial_temperature_k, state.system_pressure_psig) {
        return Err(StepReject::InitialTemperatureNotLiquidWater);
    }
    if load.coolant_flow_m_per_s < 0.0 {
        return Err(StepReject::NegativeFlow);
    }
    if !(load.reactivity_inserted_dollar > 0.0) {
        return Err(StepReject::ReactivityNotPositive);
    }
    if state.past_plate_maximum {
        return Err(StepReject::PastPlateTemperatureMaximum);
    }
    if !(state.t_s >= 0.0) {
        return Err(StepReject::NegativeTime);
    }
    incoming_state_is_the_cards(reactor, state, load)
}

/// The incoming state held to what this card itself writes.
fn incoming_state_is_the_cards(
    reactor: &ReactorSpert3Ccore,
    state: &ReactorState,
    load: ReactorLoad,
) -> Result<(), StepReject> {
    let mut index = 0;
    while index < 6 {
        if state.precursor_mw[index] < 0.0 {
            return Err(StepReject::NegativeState);
        }
        index += 1;
    }
    if !(state.power_mw > 0.0)
        || state.energy_mws < 0.0
        || state.plate_rise_k < 0.0
        || state.water_rise_k < 0.0
        || state.hot_spot_plate_rise_k < 0.0
        || state.hot_spot_water_rise_k < 0.0
        || state.swept_energy_mws < 0.0
        || state.hot_spot_swept_energy_mws < 0.0
        || state.run_coolant_flow_m_per_s < 0.0
    {
        return Err(StepReject::NegativeState);
    }
    let start = match ReactorState::try_start(reactor, state.initial_temperature_k, state.system_pressure_psig) {
        Some(start) => start,
        None => return Err(StepReject::InconsistentState),
    };
    if state.t_s == 0.0 {
        return if same_bits(state, &start) {
            Ok(())
        } else {
            Err(StepReject::InconsistentState)
        };
    }
    if load.reactivity_inserted_dollar != state.run_reactivity_dollar {
        return Err(StepReject::ReactivityChangedDuringRun);
    }
    // The derived fields were written at the flow of the last landed step; this step may carry another.
    let written = ReactorLoad {
        reactivity_inserted_dollar: load.reactivity_inserted_dollar,
        coolant_flow_m_per_s: state.run_coolant_flow_m_per_s,
    };
    let terms = terms(reactor, state.initial_temperature_k, written);
    let derived = derive(&terms, reactor, state);
    // Each held as the landed step wrote it, a value below the normal range as zero.
    if state.population_anchor_mw.to_bits() != normal_or_zero(start.population_anchor_mw).to_bits()
        || state.power_rate_mw_per_s.to_bits() != normal_or_zero(derived.power_rate).to_bits()
        || state.hot_spot_plate_rate_k_per_s.to_bits() != normal_or_zero(derived.plate_rate).to_bits()
        || state.reactivity_net_dollar.to_bits() != normal_or_zero(derived.net_dollar).to_bits()
        || state.prompt_supercritical != derived.prompt_supercritical
        || state.population_defect_mw.to_bits() != normal_or_zero(derived.population_defect).to_bits()
        || state.energy_defect_mws.to_bits() != normal_or_zero(derived.energy_defect).to_bits()
        || state.hot_spot_energy_defect_mws.to_bits() != normal_or_zero(derived.hot_spot_defect).to_bits()
        || state.residual_conservation_error.to_bits() != normal_or_zero(derived.residual).to_bits()
    {
        return Err(StepReject::InconsistentState);
    }
    // The peak fields are samples this run has landed.
    if state.peak_power_mw < reactor.constants[K_P0]
        || (!state.past_power_peak && state.peak_power_mw < state.power_mw)
        || !(state.peak_time_s >= 0.0 && state.peak_time_s <= state.t_s)
        || !(state.peak_energy_mws >= 0.0 && state.peak_energy_mws <= state.energy_mws)
        || (state.peak_time_s == state.t_s
            && (state.peak_power_mw.to_bits() != state.power_mw.to_bits()
                || state.peak_energy_mws.to_bits() != state.energy_mws.to_bits()))
        || state.plate_rise_max_k < state.hot_spot_plate_rise_k
        || !(state.plate_rise_max_time_s >= 0.0 && state.plate_rise_max_time_s <= state.t_s)
        || (state.plate_rise_max_time_s == state.t_s
            && state.plate_rise_max_k.to_bits() != state.hot_spot_plate_rise_k.to_bits())
    {
        return Err(StepReject::InconsistentState);
    }
    // A flag not set on a state that is past what it marks.
    if !state.past_power_peak && !(state.power_rate_mw_per_s > 0.0) {
        return Err(StepReject::InconsistentState);
    }
    if state.past_power_peak && !(state.hot_spot_plate_rate_k_per_s > 0.0) {
        return Err(StepReject::PastPlateTemperatureMaximum);
    }
    Ok(())
}

/// The fields of a state that follow from the rest of it under one load.
struct Derived {
    power_rate: f64,
    plate_rate: f64,
    net_dollar: f64,
    population_defect: f64,
    energy_defect: f64,
    hot_spot_defect: f64,
    residual: f64,
    prompt_supercritical: bool,
}

/// The derived fields computed again from the rest of the state by the arithmetic [`commit`]
/// wrote them with, operation for operation.
fn derive(terms: &Terms, reactor: &ReactorSpert3Ccore, state: &ReactorState) -> Derived {
    let y = state_vector(state);
    let k = rates(reactor, terms, &y);
    let net_dollar = terms.rho_0 - feedback(terms, &y);
    let (population_defect, energy_defect, hot_spot_defect, residual) = balances(terms, state);
    Derived {
        power_rate: k[0],
        plate_rate: k[9],
        net_dollar,
        population_defect,
        energy_defect,
        hot_spot_defect,
        residual,
        prompt_supercritical: net_dollar > 1.0,
    }
}

/// The three balances and the residual.
fn balances(terms: &Terms, state: &ReactorState) -> (f64, f64, f64, f64) {
    let population = population_mw(state);
    let population_defect = population
        - state.population_anchor_mw
        - (terms.rho_0 * state.energy_mws - state.feedback_integral_dollar_mws) / terms.l_star;
    let population_scale = if population > state.population_anchor_mw {
        population
    } else {
        state.population_anchor_mw
    };
    let held = terms.cp * state.plate_rise_k + terms.cw * state.water_rise_k + state.swept_energy_mws;
    let energy_defect = state.energy_mws - held;
    let hot_energy = terms.peaking * state.energy_mws;
    let hot_held = terms.cp * state.hot_spot_plate_rise_k
        + terms.cw * state.hot_spot_water_rise_k
        + state.hot_spot_swept_energy_mws;
    let hot_spot_defect = hot_energy - hot_held;
    let mut residual = population_defect.abs() / population_scale;
    if state.energy_mws > 0.0 {
        let e = energy_defect.abs() / state.energy_mws;
        let h = hot_spot_defect.abs() / hot_energy;
        if e > residual {
            residual = e;
        }
        if h > residual {
            residual = h;
        }
    }
    (population_defect, energy_defect, hot_spot_defect, residual)
}

/// Every field of two states equal bit for bit.
fn same_bits(a: &ReactorState, b: &ReactorState) -> bool {
    let mut index = 0;
    while index < 6 {
        if a.precursor_mw[index].to_bits() != b.precursor_mw[index].to_bits() {
            return false;
        }
        index += 1;
    }
    let fa = scalar_fields(a);
    let fb = scalar_fields(b);
    index = 0;
    while index < fa.len() {
        if fa[index].to_bits() != fb[index].to_bits() {
            return false;
        }
        index += 1;
    }
    a.past_power_peak == b.past_power_peak
        && a.past_plate_maximum == b.past_plate_maximum
        && a.prompt_supercritical == b.prompt_supercritical
}

fn scalar_fields(s: &ReactorState) -> [f64; 27] {
    [
        s.t_s,
        s.power_mw,
        s.plate_rise_k,
        s.water_rise_k,
        s.hot_spot_plate_rise_k,
        s.hot_spot_water_rise_k,
        s.energy_mws,
        s.initial_temperature_k,
        s.system_pressure_psig,
        s.power_rate_mw_per_s,
        s.hot_spot_plate_rate_k_per_s,
        s.reactivity_net_dollar,
        s.population_anchor_mw,
        s.feedback_integral_dollar_mws,
        s.swept_energy_mws,
        s.hot_spot_swept_energy_mws,
        s.population_defect_mw,
        s.energy_defect_mws,
        s.hot_spot_energy_defect_mws,
        s.residual_conservation_error,
        s.peak_power_mw,
        s.peak_energy_mws,
        s.peak_time_s,
        s.plate_rise_max_k,
        s.plate_rise_max_time_s,
        s.run_reactivity_dollar,
        s.run_coolant_flow_m_per_s,
    ]
}

/// `h (f0 + f1) / 2 + h² (f0' − f1') / 12`.
fn hermite_rule(h: f64, f0: f64, f1: f64, d0: f64, d1: f64) -> f64 {
    0.5 * h * (f0 + f1) + h * h * (d0 - d1) / 12.0
}

/// `(ρ_0 − ρ) P` and its time derivative at `y` with rates `k`.
fn feedback_integrand(terms: &Terms, y: &[f64; 12], k: &[f64; 12]) -> (f64, f64) {
    let taken = feedback(terms, y);
    let taken_rate = coefficient(terms, terms.initial_c + y[8]) * k[8] + terms.displacement * (k[7] - k[8]);
    (taken * y[0], taken_rate * y[0] + taken * k[0])
}

fn commit(
    reactor: &ReactorSpert3Ccore,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    terms: &Terms,
    load: ReactorLoad,
    dt_s: f64,
    mut advanced: [f64; 12],
) -> Result<(), StepReject> {
    let saved = *state;
    let mut index = 0;
    while index < 12 {
        if !advanced[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    // A value below the normal range is carried as zero (`crate::normal`):
    // the derived fields below are computed from the values the state
    // carries, as the check of an incoming state computes them again.
    normal_or_zero_each(&mut advanced);
    within_state_ceiling(reactor, terms, &advanced, dt_s)?;
    let y_old = state_vector(state);
    let k_old = rates(reactor, terms, &y_old);
    let k_new = rates(reactor, terms, &advanced);
    let (f0, d0) = feedback_integrand(terms, &y_old, &k_old);
    let (f1, d1) = feedback_integrand(terms, &advanced, &k_new);
    state.feedback_integral_dollar_mws += hermite_rule(dt_s, f0, f1, d0, d1);
    let swept = terms.cw * terms.sweep;
    state.swept_energy_mws += swept * hermite_rule(dt_s, y_old[8], advanced[8], k_old[8], k_new[8]);
    state.hot_spot_swept_energy_mws += swept * hermite_rule(dt_s, y_old[10], advanced[10], k_old[10], k_new[10]);
    state.feedback_integral_dollar_mws = normal_or_zero(state.feedback_integral_dollar_mws);
    state.swept_energy_mws = normal_or_zero(state.swept_energy_mws);
    state.hot_spot_swept_energy_mws = normal_or_zero(state.hot_spot_swept_energy_mws);
    state.t_s += dt_s;
    state.power_mw = advanced[0];
    index = 0;
    while index < 6 {
        state.precursor_mw[index] = advanced[1 + index];
        index += 1;
    }
    state.plate_rise_k = advanced[7];
    state.water_rise_k = advanced[8];
    state.hot_spot_plate_rise_k = advanced[9];
    state.hot_spot_water_rise_k = advanced[10];
    state.energy_mws = advanced[11];
    state.power_rate_mw_per_s = normal_or_zero(k_new[0]);
    state.hot_spot_plate_rate_k_per_s = normal_or_zero(k_new[9]);
    state.reactivity_net_dollar = normal_or_zero(terms.rho_0 - feedback(terms, &advanced));
    let (population_defect, energy_defect, hot_spot_defect, residual) = balances(terms, state);
    state.population_defect_mw = population_defect;
    state.energy_defect_mws = energy_defect;
    state.hot_spot_energy_defect_mws = hot_spot_defect;
    state.residual_conservation_error = residual;
    if !state.past_power_peak && state.power_mw > state.peak_power_mw {
        state.peak_power_mw = state.power_mw;
        state.peak_energy_mws = state.energy_mws;
        state.peak_time_s = state.t_s;
    }
    if state.hot_spot_plate_rise_k > state.plate_rise_max_k {
        state.plate_rise_max_k = state.hot_spot_plate_rise_k;
        state.plate_rise_max_time_s = state.t_s;
    }
    let was_past_peak = state.past_power_peak;
    if !state.past_power_peak && !(state.power_rate_mw_per_s > 0.0) {
        state.past_power_peak = true;
    }
    if was_past_peak && !(state.hot_spot_plate_rate_k_per_s > 0.0) {
        state.past_plate_maximum = true;
    }
    state.prompt_supercritical = state.reactivity_net_dollar > 1.0;
    state.run_reactivity_dollar = load.reactivity_inserted_dollar;
    state.run_coolant_flow_m_per_s = load.coolant_flow_m_per_s;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > reactor.constants[K_RESIDUAL] {
        let created = largest_defect_is_created(terms, state);
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

/// The sign of the balance that sets the residual: positive is created.
fn largest_defect_is_created(terms: &Terms, state: &ReactorState) -> bool {
    let population = population_mw(state);
    let scale = if population > state.population_anchor_mw {
        population
    } else {
        state.population_anchor_mw
    };
    let mut largest = state.population_defect_mw.abs() / scale;
    let mut created = state.population_defect_mw > 0.0;
    if state.energy_mws > 0.0 {
        let e = state.energy_defect_mws.abs() / state.energy_mws;
        if e > largest {
            largest = e;
            created = state.energy_defect_mws > 0.0;
        }
        let h = state.hot_spot_energy_defect_mws.abs() / (terms.peaking * state.energy_mws);
        if h > largest {
            created = state.hot_spot_energy_defect_mws > 0.0;
        }
    }
    created
}

/// The twelve variables in the order of the Runge–Kutta step: P, C_1 to C_6, T_p, T_w, T_h, T_hw, E.
fn state_vector(state: &ReactorState) -> [f64; 12] {
    [
        state.power_mw,
        state.precursor_mw[0],
        state.precursor_mw[1],
        state.precursor_mw[2],
        state.precursor_mw[3],
        state.precursor_mw[4],
        state.precursor_mw[5],
        state.plate_rise_k,
        state.water_rise_k,
        state.hot_spot_plate_rise_k,
        state.hot_spot_water_rise_k,
        state.energy_mws,
    ]
}

fn population_mw(state: &ReactorState) -> f64 {
    state.power_mw
        + state.precursor_mw[0]
        + state.precursor_mw[1]
        + state.precursor_mw[2]
        + state.precursor_mw[3]
        + state.precursor_mw[4]
        + state.precursor_mw[5]
}

/// The right-hand side of the equations at `y`.
fn rates(reactor: &ReactorSpert3Ccore, terms: &Terms, y: &[f64; 12]) -> [f64; 12] {
    let rho_net = terms.rho_0 - feedback(terms, y);
    let mut out = [0.0; 12];
    let mut delayed = 0.0;
    let mut index = 0;
    while index < 6 {
        let decay = reactor.group_decay_per_s[index];
        delayed += decay * y[1 + index];
        out[1 + index] = reactor.group_fraction[index] / terms.l_star * y[0] - decay * y[1 + index];
        index += 1;
    }
    out[0] = (rho_net - 1.0) / terms.l_star * y[0] + delayed;
    let h = terms.conductance_mw_per_k;
    out[7] = (terms.plate_share * y[0] - h * (y[7] - y[8])) / terms.cp;
    out[8] = (terms.water_share * y[0] + h * (y[7] - y[8])) / terms.cw - terms.sweep * y[8];
    out[9] = (terms.peaking * terms.plate_share * y[0] - h * (y[9] - y[10])) / terms.cp;
    out[10] = (terms.peaking * terms.water_share * y[0] + h * (y[9] - y[10])) / terms.cw - terms.sweep * y[10];
    out[11] = y[0];
    out
}

fn advance(y: &[f64; 12], k: &[f64; 12], scale: f64) -> [f64; 12] {
    let mut out = [0.0; 12];
    let mut index = 0;
    while index < 12 {
        out[index] = y[index] + scale * k[index];
        index += 1;
    }
    out
}

/// Classical fourth-order Runge–Kutta.
fn rk4(reactor: &ReactorSpert3Ccore, terms: &Terms, y: &[f64; 12], dt_s: f64) -> [f64; 12] {
    let half = 0.5 * dt_s;
    let k1 = rates(reactor, terms, y);
    let k2 = rates(reactor, terms, &advance(y, &k1, half));
    let k3 = rates(reactor, terms, &advance(y, &k2, half));
    let k4 = rates(reactor, terms, &advance(y, &k3, dt_s));
    let mut out = [0.0; 12];
    let mut index = 0;
    while index < 12 {
        out[index] = y[index] + dt_s / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    out
}

fn publish(state: &ReactorState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.power_mw as f32;
    frame.pos[1] = state.energy_mws as f32;
    frame.pos[2] = state.reactivity_net_dollar as f32;
    frame.vel[0] = state.hot_spot_plate_rise_k as f32;
    frame.vel[1] = state.water_rise_k as f32;
    frame.vel[2] = state.peak_power_mw as f32;
    frame.force_torque = state.power_rate_mw_per_s as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.prompt_supercritical {
        flags |= FLAG_PROMPT_SUPERCRITICAL;
    }
    if state.past_power_peak {
        flags |= FLAG_PAST_POWER_PEAK;
    }
    frame.flags = flags;
}

fn state_is_finite(state: &ReactorState) -> bool {
    let mut index = 0;
    while index < 6 {
        if !state.precursor_mw[index].is_finite() {
            return false;
        }
        index += 1;
    }
    let fields = scalar_fields(state);
    index = 0;
    while index < fields.len() {
        if !fields[index].is_finite() {
            return false;
        }
        index += 1;
    }
    true
}


#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::format;
    use std::matches;

    use super::{
        advance, commit, rates, rk4, state_dt_ceiling_s, state_vector, step, terms, validate, within_state_ceiling,
        ATMOSPHERE_PSI, CRITICAL_TEMPERATURE_K, FLAG_PAST_POWER_PEAK, FLOW_MAX_M_PER_S, FLOW_MIN_M_PER_S, FREEZING_K,
        GROUP_FRACTION, INITIAL_POWER_MW, MPA_PER_PSI, REACTIVITY_MAX_DOLLAR, REACTIVITY_MIN_FLOW_DOLLAR,
        REACTIVITY_MIN_NO_FLOW_DOLLAR, RESIDUAL_CONSERVATION_LIMIT, ReactorLoad, ReactorSpert3Ccore, ReactorState,
        StepReject, Terms,
    };
    use crate::frame::LastStateFrame64;

    fn gpm(flow_gpm: f64) -> f64 {
        flow_gpm * (18.0 / 20000.0) * 0.3048
    }

    fn load(rho: f64, flow_m_per_s: f64) -> ReactorLoad {
        ReactorLoad { reactivity_inserted_dollar: rho, coolant_flow_m_per_s: flow_m_per_s }
    }

    fn start(celsius: f64, psig: f64) -> (ReactorSpert3Ccore, ReactorState) {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        let state = ReactorState::try_start(&reactor, celsius + 273.15, psig).unwrap();
        (reactor, state)
    }

    fn marked_frame() -> LastStateFrame64 {
        let mut frame = LastStateFrame64::from_bytes([0u8; 64]);
        frame.proof = [0xA5; 16];
        frame.t = -1.0;
        frame
    }

    fn ceiling() -> f64 {
        ReactorSpert3Ccore::c_core_step_transients().gate_dt_s().unwrap()
    }

    /// The gauge pressure in psig of an absolute pressure in MPa.
    fn psig_of_mpa(mpa: f64) -> f64 {
        mpa / MPA_PER_PSI - ATMOSPHERE_PSI
    }

    type Method = fn(&ReactorSpert3Ccore, &Terms, &[f64; 12], f64) -> [f64; 12];

    fn step_with(
        method: Method,
        reactor: &ReactorSpert3Ccore,
        state: &mut ReactorState,
        frame: &mut LastStateFrame64,
        load: ReactorLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        validate(reactor, state, load, dt_s)?;
        let t = terms(reactor, state.initial_temperature_k, load);
        within_state_ceiling(reactor, &t, &state_vector(state), dt_s)?;
        let advanced = method(reactor, &t, &state_vector(state), dt_s);
        commit(reactor, state, frame, &t, load, dt_s, advanced)
    }

    fn euler(reactor: &ReactorSpert3Ccore, t: &Terms, y: &[f64; 12], dt_s: f64) -> [f64; 12] {
        advance(y, &rates(reactor, t, y), dt_s)
    }

    fn wrong_weights(reactor: &ReactorSpert3Ccore, t: &Terms, y: &[f64; 12], dt_s: f64) -> [f64; 12] {
        let mut out = rk4(reactor, t, y, dt_s);
        let k1 = rates(reactor, t, y);
        out[0] += dt_s * 1.0e-3 * k1[0];
        out
    }

    /// Runge–Kutta that forgets the water's temperature feedback in the power equation.
    fn no_water_feedback(reactor: &ReactorSpert3Ccore, t: &Terms, y: &[f64; 12], dt_s: f64) -> [f64; 12] {
        let mut altered = *t;
        altered.cold = 0.0;
        altered.slope = 0.0;
        altered.base = 0.0;
        rk4(reactor, &altered, y, dt_s)
    }

    /// Runge–Kutta that loses one percent of the heat the plates pass to the water.
    fn leaky_heat_path(reactor: &ReactorSpert3Ccore, t: &Terms, y: &[f64; 12], dt_s: f64) -> [f64; 12] {
        let mut out = rk4(reactor, t, y, dt_s);
        out[8] -= 0.01 * dt_s * t.conductance_mw_per_k * (y[7] - y[8]) / t.cw;
        out
    }

    struct Run {
        last: ReactorState,
        steps: u64,
        largest_residual: f64,
        /// The refusal that stopped the run; `None` when it took every step asked.
        end: Option<StepReject>,
    }

    fn run_from(
        method: Method,
        reactor: &ReactorSpert3Ccore,
        mut state: ReactorState,
        load: ReactorLoad,
        dt_s: f64,
        most: u64,
    ) -> Run {
        let mut frame = marked_frame();
        let mut steps = 0;
        let mut largest_residual = 0.0f64;
        let end = loop {
            if steps >= most {
                break None;
            }
            let before = state;
            let before_frame = frame.to_bytes();
            match step_with(method, reactor, &mut state, &mut frame, load, dt_s) {
                Ok(()) => {
                    steps += 1;
                    largest_residual = largest_residual.max(state.residual_conservation_error);
                    assert_eq!(frame.proof, [0xA5; 16]);
                }
                Err(reject) => {
                    // every refusal keeps the state and the frame
                    assert_eq!(format!("{:?}", state), format!("{:?}", before));
                    assert_eq!(frame.to_bytes(), before_frame);
                    break Some(reject);
                }
            }
        };
        Run { last: state, steps, largest_residual, end }
    }

    fn run_with(method: Method, celsius: f64, psig: f64, load: ReactorLoad, dt_s: f64, most: u64) -> Run {
        let (reactor, state) = start(celsius, psig);
        run_from(method, &reactor, state, load, dt_s, most)
    }

    /// The step at the start: the static ceiling, or the state ceiling at the start where that is shorter.
    fn start_dt(celsius: f64, psig: f64, l: ReactorLoad) -> f64 {
        let (reactor, state) = start(celsius, psig);
        let state_ceiling = state_dt_ceiling_s(&reactor, &state, l).unwrap();
        if state_ceiling < ceiling() { state_ceiling } else { ceiling() }
    }

    #[test]
    fn constants_are_locked_and_the_compared_range_is_the_inhour_value_of_the_compared_periods() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        assert!(reactor.is_valid());
        for index in 0..reactor.constants.len() {
            let mut altered = reactor;
            altered.constants[index] = f64::from_bits(altered.constants[index].to_bits() ^ 1);
            assert!(!altered.is_valid(), "constant {index}");
        }
        let mut altered = reactor;
        altered.group_decay_per_s[5] = 3.0;
        assert!(!altered.is_valid());
        let (_, mut state) = start(25.0, 0.0);
        let mut frame = marked_frame();
        let before = state;
        assert_eq!(step(&altered, &mut state, &mut frame, load(1.0, 0.0), 1.0e-4), Err(StepReject::InvalidCard));
        assert_eq!(state, before);
        let sum: f64 = GROUP_FRACTION.iter().sum();
        assert!((sum - 1.0).abs() < 1.0e-12);
        assert_eq!(reactor.inhour_dollar(9.3).to_bits(), REACTIVITY_MIN_NO_FLOW_DOLLAR.to_bits());
        assert_eq!(reactor.inhour_dollar(0.516).to_bits(), REACTIVITY_MIN_FLOW_DOLLAR.to_bits());
        assert_eq!(reactor.inhour_dollar(9.7 / 1000.0).to_bits(), REACTIVITY_MAX_DOLLAR.to_bits());
        assert_eq!(gpm(200.0), FLOW_MIN_M_PER_S);
        assert_eq!(gpm(20000.0), FLOW_MAX_M_PER_S);
        assert_eq!(ceiling(), 0.2 * 0.003);
        // the conductance with no flow is the fitted one; at 20,000 gpm Dittus–Boelter adds about 21,000
        assert_eq!(reactor.conductance_w_per_m2_k(0.0), 6403.73);
        let full = reactor.conductance_w_per_m2_k(FLOW_MAX_M_PER_S);
        assert!(full > 21_000.0 && full < 23_500.0, "{full}");
    }

    /// IAPWS-IF97 region 4, backward equation, against the verification values the release
    /// gives for it (IAPWS R7-97(2012), Table 35).
    #[test]
    fn the_saturation_temperature_is_iapws_if97_region_4() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        for (mpa, kelvin) in [(0.1, 372.755919), (1.0, 453.035632), (10.0, 584.149488)] {
            let t = reactor.saturation_temperature_k(psig_of_mpa(mpa));
            assert!((t / kelvin - 1.0).abs() < 1.0e-6, "{mpa} MPa: {t}");
        }
        // at 0 psig water boils at 99.97 degC
        let atmospheric = reactor.saturation_temperature_k(0.0);
        assert!((atmospheric - 373.124).abs() < 1.0e-3, "{atmospheric}");
        // at the critical pressure and above, the critical temperature
        assert_eq!(reactor.saturation_temperature_k(psig_of_mpa(22.064) + 1.0e-6), CRITICAL_TEMPERATURE_K);
        assert_eq!(reactor.saturation_temperature_k(1.0e6), CRITICAL_TEMPERATURE_K);
        let below = reactor.saturation_temperature_k(psig_of_mpa(22.0));
        assert!(below < CRITICAL_TEMPERATURE_K && below > 646.0, "{below}");
        // at the triple point the saturation temperature is the freezing point: nothing below it is liquid
        let triple = reactor.saturation_temperature_k(psig_of_mpa(611.213e-6));
        assert!((triple - FREEZING_K).abs() < 1.0e-4, "{triple}");
        assert!(reactor.saturation_temperature_k(psig_of_mpa(1.0e-4)) < FREEZING_K);
    }

    #[test]
    fn the_start_is_the_equilibrium_and_every_start_of_liquid_water_starts() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        for (celsius, psig) in [(21.0, 0.0), (36.0, 2500.0), (25.0, 1000.0), (30.0, 230.0), (0.0, 0.0), (99.9, 0.0), (300.0, 2500.0), (373.9, 5000.0)] {
            let state = ReactorState::try_start(&reactor, celsius + 273.15, psig).unwrap();
            assert_eq!(state.power_mw, INITIAL_POWER_MW);
            // no reactivity stepped in: the start is the equilibrium
            let t = terms(&reactor, state.initial_temperature_k, load(0.0, 0.0));
            let k = rates(&reactor, &t, &state_vector(&state));
            assert!(k[0].abs() < 1.0e-12 * INITIAL_POWER_MW / 0.003);
            for i in 1..7 {
                assert!(k[i].abs() < 1.0e-12 * state.precursor_mw[i - 1]);
            }
        }
        let boiling = reactor.saturation_temperature_k(0.0);
        assert!(ReactorState::try_start(&reactor, FREEZING_K, 0.0).is_some());
        assert!(ReactorState::try_start(&reactor, FREEZING_K.next_down(), 0.0).is_none());
        assert!(ReactorState::try_start(&reactor, boiling.next_down(), 0.0).is_some());
        assert!(ReactorState::try_start(&reactor, boiling, 0.0).is_none());
        assert!(ReactorState::try_start(&reactor, CRITICAL_TEMPERATURE_K.next_down(), 1.0e5).is_some());
        assert!(ReactorState::try_start(&reactor, CRITICAL_TEMPERATURE_K, 1.0e5).is_none());
        assert!(ReactorState::try_start(&reactor, 300.0, -ATMOSPHERE_PSI).is_none());
        assert!(ReactorState::try_start(&reactor, 300.0, -20.0).is_none());
        // just above absolute zero water boils below freezing: no start is liquid
        assert!(ReactorState::try_start(&reactor, 274.0, -14.69).is_none());
        assert!(ReactorState::try_start(&reactor, f64::NAN, 0.0).is_none());
        assert!(ReactorState::try_start(&reactor, 300.0, f64::INFINITY).is_none());
    }

    #[test]
    fn every_refusal_is_named_and_rolls_back_state_and_frame() {
        let (reactor, fresh) = start(25.0, 0.0);
        let dt = 1.0e-4;
        let refuse = |state: ReactorState, load: ReactorLoad, dt_s: f64, expected: StepReject| {
            let mut s = state;
            let mut frame = marked_frame();
            let before = frame.to_bytes();
            assert_eq!(step(&reactor, &mut s, &mut frame, load, dt_s), Err(expected));
            assert_eq!(format!("{:?}", s), format!("{:?}", state));
            assert_eq!(frame.to_bytes(), before);
        };
        refuse(fresh, load(1.0, 0.0), f64::NAN, StepReject::NonFiniteInput);
        refuse(fresh, load(f64::INFINITY, 0.0), dt, StepReject::NonFiniteInput);
        refuse(fresh, load(1.0, f64::NAN), dt, StepReject::NonFiniteInput);
        let mut bad = fresh;
        bad.precursor_mw[2] = f64::NAN;
        refuse(bad, load(1.0, 0.0), dt, StepReject::NonFiniteState);
        refuse(fresh, load(1.0, 0.0), 0.0, StepReject::NonPositiveDt);
        refuse(fresh, load(1.0, 0.0), -dt, StepReject::NonPositiveDt);
        // pressure at or below absolute zero
        for psig in [-ATMOSPHERE_PSI, -20.0, -1.0e6] {
            let mut vacuum = fresh;
            vacuum.system_pressure_psig = psig;
            refuse(vacuum, load(1.0, 0.0), dt, StepReject::PressureNotPositive);
        }
        // a start that is not liquid water at its pressure
        let boiling = reactor.saturation_temperature_k(0.0);
        for (kelvin, psig) in [
            (FREEZING_K.next_down(), 0.0),
            (250.0, 2500.0),
            (boiling, 0.0),
            (400.0, 0.0),
            (CRITICAL_TEMPERATURE_K, 5000.0),
            (700.0, 1.0e5),
            (274.0, -14.69),
        ] {
            let mut odd = fresh;
            odd.initial_temperature_k = kelvin;
            odd.system_pressure_psig = psig;
            refuse(odd, load(1.0, 0.0), dt, StepReject::InitialTemperatureNotLiquidWater);
        }
        // a flow below zero
        for flow in [-1.0e-12, -gpm(1000.0), -100.0] {
            refuse(fresh, load(1.0, flow), dt, StepReject::NegativeFlow);
        }
        // a reactivity of zero or less
        for rho in [0.0, -0.0, -0.5, -1.0e-300] {
            refuse(fresh, load(rho, 0.0), dt, StepReject::ReactivityNotPositive);
        }
        // a clock below zero
        let mut early = fresh;
        early.t_s = -1.0;
        refuse(early, load(1.0, 0.0), dt, StepReject::NegativeTime);
        let mut negative = fresh;
        negative.water_rise_k = -1.0;
        refuse(negative, load(1.0, 0.0), dt, StepReject::NegativeState);
        let mut moved = fresh;
        moved.plate_rise_k = 1.0;
        refuse(moved, load(1.0, 0.0), dt, StepReject::InconsistentState);
        // a step longer than the state lets it be: 5 dollars at 0.6 ms, 300 m/s at 0.6 ms
        refuse(fresh, load(5.0, 0.0), ceiling(), StepReject::DtAboveStateCeiling);
        refuse(fresh, load(1.2, 300.0), ceiling(), StepReject::DtAboveStateCeiling);
        // every edge of the compared range lands, and so do the old walls and far past them
        let (_, loop230) = start(30.0, 230.0);
        let (_, high) = start(30.0, 2500.0);
        let (_, hot) = start(250.0, 2500.0);
        let (_, cold) = start(0.0, 0.0);
        let (_, supercritical) = start(370.0, 3500.0);
        for (state, l, dt_s) in [
            (fresh, load(REACTIVITY_MIN_NO_FLOW_DOLLAR, 0.0), ceiling()),
            (fresh, load(REACTIVITY_MAX_DOLLAR, 0.0), ceiling()),
            (loop230, load(REACTIVITY_MIN_FLOW_DOLLAR, FLOW_MIN_M_PER_S), ceiling()),
            (loop230, load(REACTIVITY_MAX_DOLLAR, FLOW_MAX_M_PER_S), ceiling()),
            (high, load(1.0, 0.0), ceiling()),
            (high, load(1.0, gpm(20000.0)), ceiling()),
            (fresh, load(1.0, gpm(5000.0)), ceiling()),
            (fresh, load(1.0e-6, 0.0), ceiling()),
            (fresh, load(0.1, 0.0), ceiling()),
            (fresh, load(3.0, 0.0), 9.0e-5),
            (fresh, load(1.2, 300.0), 1.0e-5),
            (hot, load(1.2, 0.0), ceiling()),
            (cold, load(1.2, 0.0), ceiling()),
            (supercritical, load(1.2, 0.0), ceiling()),
        ] {
            let mut s = state;
            let mut frame = marked_frame();
            assert!(step(&reactor, &mut s, &mut frame, l, dt_s).is_ok(), "{l:?} from {} K", state.initial_temperature_k);
        }
    }

    #[test]
    fn a_short_period_run_passes_the_power_peak_then_ends_at_the_plate_maximum_with_the_ledger_closed() {
        let rho = ReactorSpert3Ccore::c_core_step_transients().inhour_dollar(0.0107);
        let run = run_with(rk4, 25.0, 0.0, load(rho, 0.0), ceiling(), 10_000);
        assert_eq!(run.end, Some(StepReject::PastPlateTemperatureMaximum));
        let s = run.last;
        assert!(s.past_power_peak && s.past_plate_maximum);
        assert!(s.peak_time_s < s.plate_rise_max_time_s);
        assert!(s.peak_power_mw > 300.0 && s.peak_power_mw < 1500.0, "{}", s.peak_power_mw);
        assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(run.largest_residual > 0.0);
        // every joule released is in the plates, the water or swept out, to rounding
        assert!(s.energy_defect_mws.abs() <= 1.0e-12 * s.energy_mws);
        assert!(s.hot_spot_energy_defect_mws.abs() <= 1.0e-12 * s.energy_mws * 2.652);
        // the next step is refused and the state kept
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        let mut again = s;
        let mut frame = marked_frame();
        assert_eq!(step(&reactor, &mut again, &mut frame, load(rho, 0.0), ceiling()), Err(StepReject::PastPlateTemperatureMaximum));
        assert_eq!(again, s);
    }

    #[test]
    fn the_flag_marks_the_power_peak_and_the_reactivity_cannot_change_during_a_run() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        let (_, mut state) = start(30.0, 230.0);
        let mut frame = marked_frame();
        let l = load(1.1, gpm(5000.0));
        let mut seen_flag = false;
        for _ in 0..2000 {
            if step(&reactor, &mut state, &mut frame, l, ceiling()).is_err() {
                break;
            }
            if frame.flags & FLAG_PAST_POWER_PEAK != 0 {
                seen_flag = true;
            }
            if state.t_s > 0.01 && !state.past_power_peak {
                let mut s = state;
                let mut f = frame;
                assert_eq!(step(&reactor, &mut s, &mut f, load(1.2, gpm(5000.0)), ceiling()), Err(StepReject::ReactivityChangedDuringRun));
                assert_eq!(s, state);
                assert_eq!(f.to_bytes(), frame.to_bytes());
                // another flow lands
                assert!(step(&reactor, &mut s, &mut f, load(1.1, gpm(6000.0)), ceiling()).is_ok());
            }
        }
        assert!(seen_flag);
    }

    /// The flow may change from step to step: each step is taken at its own flow, the swept
    /// energies are booked at that step's flow, and the ledger closes through every change.
    #[test]
    fn a_run_whose_flow_changes_closes_its_ledger_at_every_step() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        let (_, mut state) = start(25.0, 0.0);
        let mut frame = marked_frame();
        let rho = 1.15;
        let mut largest = 0.0f64;
        let mut swept_before_flow = None;
        let mut end = None;
        for index in 0..100_000u64 {
            // no flow to 0.1 s, then 20,000 gpm to 0.2 s, then 30 m/s, then no flow again from 0.3 s
            let flow = match index {
                0..=166 => 0.0,
                167..=333 => FLOW_MAX_M_PER_S,
                334..=499 => 30.0,
                _ => 0.0,
            };
            let l = load(rho, flow);
            let dt = if state_dt_ceiling_s(&reactor, &state, l).unwrap() < ceiling() { 1.0e-4 } else { ceiling() };
            match step(&reactor, &mut state, &mut frame, l, dt) {
                Ok(()) => largest = largest.max(state.residual_conservation_error),
                Err(reject) => {
                    end = Some(reject);
                    break;
                }
            }
            if index == 166 {
                assert_eq!(state.swept_energy_mws, 0.0);
                swept_before_flow = Some(state.swept_energy_mws);
            }
            if index == 400 {
                assert!(state.swept_energy_mws > 0.0 && state.hot_spot_swept_energy_mws > 0.0);
            }
            assert!(state.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT, "{index}");
        }
        assert!(swept_before_flow.is_some());
        assert_eq!(end, Some(StepReject::PastPlateTemperatureMaximum));
        assert!(largest <= RESIDUAL_CONSERVATION_LIMIT, "{largest}");
        assert!(state.past_plate_maximum);
    }

    #[test]
    fn a_wrong_method_or_a_falsified_heat_path_is_named_by_the_ledger() {
        let rho = ReactorSpert3Ccore::c_core_step_transients().inhour_dollar(0.0107);
        let named = |end: Option<StepReject>| end == Some(StepReject::SpontaneousGeneration) || end == Some(StepReject::FirstLawBreach);
        for method in [euler as Method, wrong_weights, no_water_feedback, leaky_heat_path] {
            let run = run_with(method, 25.0, 0.0, load(rho, 0.0), ceiling(), 10_000);
            assert!(named(run.end), "{:?} after {} steps", run.end, run.steps);
        }
        // with flow, too
        let run = run_with(euler, 30.0, 230.0, load(rho, gpm(20000.0)), ceiling(), 10_000);
        assert!(named(run.end));
        // and past the compared range: 3 dollars, a start at 250 degC, 30 m/s at 0 psig
        for (celsius, psig, l) in [(25.0, 0.0, load(3.0, 0.0)), (250.0, 2500.0, load(1.2, 0.0)), (25.0, 0.0, load(1.2, 30.0))] {
            let dt = start_dt(celsius, psig, l);
            for method in [euler as Method, wrong_weights, leaky_heat_path] {
                let run = run_with(method, celsius, psig, l, dt, 100_000);
                assert!(named(run.end), "{l:?} {celsius} degC: {:?} after {} steps", run.end, run.steps);
            }
            let right = run_with(rk4, celsius, psig, l, dt, 100_000);
            assert_eq!(right.end, Some(StepReject::PastPlateTemperatureMaximum), "{l:?}");
        }
        // a falsified ledger in the handed-in state is not one the card wrote
        let (reactor, mut state) = start(25.0, 0.0);
        let mut frame = marked_frame();
        for _ in 0..50 {
            step(&reactor, &mut state, &mut frame, load(rho, 0.0), ceiling()).unwrap();
        }
        for field in 0..5 {
            let mut forged = state;
            match field {
                0 => forged.feedback_integral_dollar_mws = forged.feedback_integral_dollar_mws * 1.01 + 1.0e-9,
                1 => forged.swept_energy_mws += 1.0e-9,
                2 => forged.peak_power_mw *= 0.5,
                3 => forged.run_coolant_flow_m_per_s = 1.0,
                _ => forged.residual_conservation_error = 0.0,
            }
            let before = forged;
            let mut f = frame;
            assert_eq!(step(&reactor, &mut forged, &mut f, load(rho, 0.0), ceiling()), Err(StepReject::InconsistentState), "field {field}");
            assert_eq!(format!("{:?}", forged), format!("{:?}", before));
        }
    }

    #[test]
    fn the_largest_residual_at_the_ceiling_is_under_the_limit_at_the_corners() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        let fast = REACTIVITY_MAX_DOLLAR;
        let mut largest = 0.0f64;
        for (celsius, psig, l) in [
            (21.0, 0.0, load(fast, 0.0)),
            (36.0, 2500.0, load(fast, 0.0)),
            (36.0, 230.0, load(fast, FLOW_MAX_M_PER_S)),
            (36.0, 230.0, load(fast, FLOW_MIN_M_PER_S)),
            (21.0, 0.0, load(reactor.inhour_dollar(0.05), 0.0)),
        ] {
            let run = run_with(rk4, celsius, psig, l, ceiling(), 20_000);
            largest = largest.max(run.largest_residual);
        }
        assert!(largest <= RESIDUAL_CONSERVATION_LIMIT, "{largest}");
        assert!(largest > RESIDUAL_CONSERVATION_LIMIT / 20.0, "{largest}");
    }

    /// (a) Inside the compared range, a step at the static ceiling never reaches the state
    /// ceiling: at the corners, and at the most negative net reactivity the compared range
    /// reaches (21 degC, no flow, 1.0989 dollars, by 200 s). (b) Past the range the state
    /// ceiling binds, at the start and at the trial end of a step, and keeps state and frame.
    #[test]
    fn the_state_ceiling_never_binds_inside_the_compared_range_and_binds_past_it() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        let to_200_s = 333_333;
        let mut least_room = f64::INFINITY;
        let mut deepest = (0.0f64, None);
        for (celsius, psig, l, most) in [
            (21.0, 0.0, load(1.0989, 0.0), to_200_s),
            (21.0, 0.0, load(REACTIVITY_MAX_DOLLAR, 0.0), 10_000),
            (21.0, 0.0, load(REACTIVITY_MIN_NO_FLOW_DOLLAR, 0.0), to_200_s),
            (21.0, 230.0, load(REACTIVITY_MAX_DOLLAR, FLOW_MAX_M_PER_S), 10_000),
            (21.0, 230.0, load(REACTIVITY_MIN_FLOW_DOLLAR, FLOW_MAX_M_PER_S), to_200_s),
        ] {
            let (_, mut state) = start(celsius, psig);
            let mut frame = marked_frame();
            for _ in 0..most {
                let room = state_dt_ceiling_s(&reactor, &state, l).unwrap() / ceiling();
                least_room = least_room.min(room);
                assert!(room > 1.0, "{l:?} at {} s: {room}", state.t_s);
                match step(&reactor, &mut state, &mut frame, l, ceiling()) {
                    Ok(()) => {}
                    Err(reject) => {
                        assert_eq!(reject, StepReject::PastPlateTemperatureMaximum, "{l:?}");
                        break;
                    }
                }
            }
            if state.reactivity_net_dollar < deepest.0 {
                deepest = (state.reactivity_net_dollar, Some(state));
            }
        }
        // the room is less than one percent at the deepest net reactivity: the phase there is 0.496
        assert!(least_room < 1.01, "{least_room}");
        assert!(deepest.0 < -1.47 && deepest.0 > -1.48, "{}", deepest.0);
        // (b) at the start: 3 dollars at the static ceiling is refused, at its own state ceiling it runs to its end
        let (_, fresh) = start(25.0, 0.0);
        for l in [load(3.0, 0.0), load(1.2, 300.0)] {
            let mut s = fresh;
            let mut frame = marked_frame();
            assert_eq!(step(&reactor, &mut s, &mut frame, l, ceiling()), Err(StepReject::DtAboveStateCeiling));
            assert_eq!(s, fresh);
            assert_eq!(frame.to_bytes(), marked_frame().to_bytes());
            let at = state_dt_ceiling_s(&reactor, &fresh, l).unwrap();
            assert!(at < ceiling());
            assert_eq!(step(&reactor, &mut s, &mut frame, l, at.next_up()), Err(StepReject::DtAboveStateCeiling));
            let run = run_from(rk4, &reactor, fresh, l, at, 3_000_000);
            assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT, "{l:?}");
            assert!(run.steps > 400, "{l:?}");
        }
        // (b) at the trial end: from the deepest state, a step the start allows and its end does not
        let state = deepest.1.unwrap();
        let l = load(1.0989, 0.0);
        let mut dt = state_dt_ceiling_s(&reactor, &state, l).unwrap();
        let t = terms(&reactor, state.initial_temperature_k, l);
        while within_state_ceiling(&reactor, &t, &state_vector(&state), dt).is_err() {
            dt = dt.next_down();
        }
        let mut s = state;
        let mut frame = marked_frame();
        assert_eq!(step(&reactor, &mut s, &mut frame, l, dt), Err(StepReject::DtAboveStateCeiling));
        assert_eq!(format!("{:?}", s), format!("{:?}", state));
        assert_eq!(frame.to_bytes(), marked_frame().to_bytes());
        // and at the static ceiling the same state goes on, past the 200 s the comparison stopped at,
        // until the ledger, the state ceiling or the end names its limit
        let run = run_from(rk4, &reactor, state, l, ceiling(), 1_000_000);
        assert!(run.last.t_s > 400.0, "{}", run.last.t_s);
        assert!(
            matches!(
                run.end,
                Some(StepReject::SpontaneousGeneration | StepReject::FirstLawBreach | StepReject::DtAboveStateCeiling)
            ),
            "{:?} at {} s",
            run.end,
            run.last.t_s
        );
        assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT);
    }

    /// Past every old wall the card computes the same equations and closes its ledger at every step.
    #[test]
    fn past_every_old_wall_the_ledger_closes() {
        // starts outside 21 to 36 degC: 0, 99 degC at 0 psig, 250 degC at 2500 psig, 370 degC at 3500 psig
        for (celsius, psig) in [(0.0, 0.0), (99.0, 0.0), (250.0, 2500.0), (370.0, 3500.0)] {
            let run = run_with(rk4, celsius, psig, load(1.2, 0.0), ceiling(), 100_000);
            assert_eq!(run.end, Some(StepReject::PastPlateTemperatureMaximum), "{celsius} degC");
            assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT, "{celsius} degC");
        }
        // no equation reads the pressure: the same transient, bit for bit, at 0 and far above 2500 psig
        let at_zero = run_with(rk4, 25.0, 0.0, load(1.2, 0.0), ceiling(), 100_000);
        for psig in [1000.0, 2500.0, 5000.0, 50_000.0] {
            let run = run_with(rk4, 25.0, psig, load(1.2, 0.0), ceiling(), 100_000);
            let mut same = run.last;
            same.system_pressure_psig = 0.0;
            assert_eq!(format!("{:?}", same), format!("{:?}", at_zero.last), "{psig} psig");
        }
        // flow at pressures other than 230 psig, and far above 20,000 gpm
        for (psig, flow) in [(0.0, gpm(5000.0)), (2500.0, FLOW_MAX_M_PER_S), (225.0, 30.0), (0.0, 100.0)] {
            let l = load(1.2, flow);
            let run = run_with(rk4, 25.0, psig, l, start_dt(25.0, psig, l), 100_000);
            if flow < 50.0 {
                assert_eq!(run.end, Some(StepReject::PastPlateTemperatureMaximum), "{flow} m/s at {psig} psig");
            } else {
                // at 100 m/s the flow carries the heat away and the plate temperature is still rising
                // after 100,000 steps (4.6 s): the run is completed
                assert_eq!(run.end, None, "{flow} m/s at {psig} psig");
                assert!(run.last.past_power_peak && !run.last.past_plate_maximum);
            }
            assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT);
        }
        // reactivities below 0.412 and far above 1.305 dollars; above 2000 MW
        for (rho, peak_low, peak_high) in [(1.5, 2400.0, 2700.0), (2.0, 9000.0, 10_000.0), (3.0, 33_000.0, 35_500.0), (5.0, 110_000.0, 120_000.0)] {
            let l = load(rho, 0.0);
            let dt = 0.5 * start_dt(25.0, 0.0, l);
            let run = run_with(rk4, 25.0, 0.0, l, dt, 1_000_000);
            assert_eq!(run.end, Some(StepReject::PastPlateTemperatureMaximum), "{rho} dollars");
            assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT, "{rho} dollars");
            assert!(run.last.peak_power_mw > peak_low && run.last.peak_power_mw < peak_high, "{rho}: {}", run.last.peak_power_mw);
        }
        for rho in [1.0e-3, 0.1, 0.3] {
            let run = run_with(rk4, 25.0, 0.0, load(rho, 0.0), ceiling(), 200_000);
            assert_eq!(run.end, None, "{rho} dollars");
            assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT, "{rho} dollars");
            assert!(run.last.power_mw > INITIAL_POWER_MW, "{rho}");
        }
        // past 200 s and 1e5 MW s: the slowest compared flow insertion with the most flow, to 600 s,
        // settles and stays completed
        let run = run_with(rk4, 25.0, 230.0, load(REACTIVITY_MIN_FLOW_DOLLAR, FLOW_MAX_M_PER_S), ceiling(), 1_000_000);
        assert_eq!(run.end, None);
        assert!(run.largest_residual <= RESIDUAL_CONSERVATION_LIMIT);
        assert!(run.last.t_s > 599.0 && run.last.energy_mws > 1.5e5, "{} MW s", run.last.energy_mws);
        assert!(!run.last.past_plate_maximum);
    }

    /// Three compared rows (held back) run to their end at the ceiling: the card's answers as
    /// the comparison of 2026-10-11 recorded them (`analysis/out/rust_held_rows.tsv` of the
    /// research record), pinned bit for bit so that any change to the equations shows. 01425 has
    /// no plate maximum before 200 s, where the comparison read it; it is run to 200 s.
    #[test]
    fn three_compared_rows_give_the_answers_the_comparison_recorded() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        // record, degC, psig, gpm, period s, steps landed, end, peak power, peak energy, plate rise max
        let rows: [(&str, f64, f64, f64, f64, u64, Option<StepReject>, u64, u64, u64); 3] = [
            ("01676", 25.0, 0.0, 0.0, 10.1 / 1000.0, 369, Some(StepReject::PastPlateTemperatureMaximum), PIN[0], PIN[1], PIN[2]),
            ("02985", 33.0, 230.0, 10000.0, 9.7 / 1000.0, 344, Some(StepReject::PastPlateTemperatureMaximum), PIN[3], PIN[4], PIN[5]),
            ("01425", 25.0, 0.0, 0.0, 9300.0 / 1000.0, 333_333, None, PIN[6], PIN[7], PIN[8]),
        ];
        for (record, celsius, psig, flow_gpm, period_s, landed, end, power, energy, plate) in rows {
            let l = load(reactor.inhour_dollar(period_s), gpm(flow_gpm));
            let run = run_with(rk4, celsius, psig, l, ceiling(), 333_333);
            assert_eq!((run.steps, run.end), (landed, end), "{record}");
            assert_eq!(run.last.peak_power_mw.to_bits(), power, "{record}");
            assert_eq!(run.last.peak_energy_mws.to_bits(), energy, "{record}");
            assert_eq!(run.last.plate_rise_max_k.to_bits(), plate, "{record}");
        }
        // 01425 at 200 s, where the comparison took the largest rise reached
        assert!((f64::from_bits(PIN[8]) / 99.457652733 - 1.0).abs() < 1.0e-9);
    }

    /// The bits of peak_power_mw, peak_energy_mws and plate_rise_max_k of 01676, 02985 and 01425.
    /// As decimals: peak power 894.4316 MW (01676), 987.2440 MW (02985), 0.4422857 MW (01425); the
    /// interpolated peaks the comparison scored are within one step of these samples.
    const PIN: [u64; 9] = [
        4651078719653893591, 4625779392977927991, 4643263208202169195,
        4651895105694386749, 4625957257751255551, 4642785033475615019,
        4601639130209859418, 4622192992007546292, 4636699127058525782,
    ];
}
