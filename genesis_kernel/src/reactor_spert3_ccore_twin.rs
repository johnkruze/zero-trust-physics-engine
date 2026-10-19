#![no_implicit_prelude]

//! The SPERT III C-core with subcooled boiling and void: step transients
//! from room temperature at system pressures of 0 to 2500 psig and coolant
//! flows of 0 to 20,000 gpm, through the first maximum of power to the first
//! maximum of the plate temperature at the hot spot after it. The twin of
//! `reactor_spert3_ccore`, fitted on all 111 usable transients of the same
//! record.
//!
//! Measured by the Phillips Petroleum Company for the Atomic Energy
//! Commission: IDO-16918, C. R. Toole, "The Effects of Pressure and Flow on
//! Room Temperature Power Excursions in Spert III" (June 1964), OSTI 4042264,
//! whose appendix tabulates 126 step transients (pressure, initial
//! temperature, flow, period, peak power, energy to the time of peak power,
//! the plate surface temperature at that time and its maximum, read by
//! thermocouples spot-welded to a plate at the flux peak of a central
//! assembly). Core constants from IDO-17138 (Condit, Scott and Johnson,
//! March 1967), Table B-1, and IDO-16918 Table I. No measured value of
//! either report is stored in the card.
//!
//! Three plate-and-water nodes, each per unit of the whole plate area: the
//! core average (`F = 1`), the feedback node (`F = F_r`, fitted, the core as
//! its reactivity sees it: flux times importance) and the hot spot
//! (`F = 2.04 · 1.30`, where the thermocouples sit). Each node holds a plate
//! temperature `T_p`, a film of the water next to the plates `T_f` (share
//! `φ` of the water), the bulk water `T_w` and the latent heat of the vapour
//! in its channels `U`. All `T` are rises above `θ_i`.
//!
//! ```text
//! power        dP/dt   = ((ρ − 1) / l*) P + Σ λ_i C_i
//! precursors   dC_i/dt = (b_i / l*) P − λ_i C_i
//! plate        C_p dT_p/dt = F (1 − f_w) P − H_1 (T_p − T_f) − q_b
//! film         C_f dT_f/dt = F f_w φ P + H_1 (T_p − T_f) − H_2 (T_f − T_w) + (1 − ε) q_b − s C_f T_f
//! bulk water   C_b dT_w/dt = F f_w (1 − φ) P + H_2 (T_f − T_w) + U / τ_c − s C_b T_w
//! vapour       dU/dt       = ε q_b − U / τ_c − s U
//! energy       dE/dt = P
//! reactivity   ρ = ρ_0 − φ [R(min(θ_i + T_f, T_sat)) − R(θ_i)] − (1 − φ) [R(θ_i + T_w) − R(θ_i)]
//!                      − D χ (T_p − T_w) − a_D T_p − K_v α,         α = U / (ρ_g h_fg V_w)
//! boiling      q_b = A q_n q_c / (q_n + q_c) when the wall is above T_sat(P), else 0
//!              q_n = m_n (ΔT_sat / (22.65 e^(−p / 8.7 MPa)))²             (Thom, MW/m²)
//!              q_c = m_c q_Zuber (1 + 0.1 (ρ_f/ρ_g)^0.75 c_p ΔT_sub / h_fg) (Zuber, Ivey–Morris)
//!              q_Zuber = 0.131 ρ_g^½ h_fg [σ g (ρ_f − ρ_g)]^¼
//! ```
//!
//! The feedback is taken on the feedback node; the hot-spot node gives the
//! plate temperature the thermocouples read. `s = 2 v / L` sweeps the water
//! and the vapour out of the core with the flow. `H_1 = A √(h_s² + h_DB(v)²)`,
//! `H_2 = A √(h_fb² + h_DB(v)²)`, with `h_DB` the Dittus–Boelter correlation
//! for water at 25 degC between plates 0.128 in apart. `R` is the integral of
//! the measured isothermal coefficient: 1.8 cents/degC to 37.78 degC
//! (IDO-16918 Table I), then straight through IDO-17138 Table B-1's 1.0, 3.2,
//! 4.6, 8.2 and 16.0 cents/degF at 100, 400, 500, 600 and 650 degF, its last
//! segment continued past 343.33 degC. The film's term is taken at the
//! saturation temperature once the film reaches it: water there boils, and
//! its vapour is the void term. `D χ` is the water the plates displace as
//! they expand, 44 cents per percent of moderator density (Table B-1) times
//! 3 · 17.3e-6 /K · 0.030 in / 0.128 in; `a_D` the plate's own coefficient
//! beyond it (Doppler and plate bowing), fitted. `K_v` is the average void
//! coefficient, 45 cents per percent (IDO-16918 Table I). `V_w` is the water
//! between the plates, 0.159 m³.
//!
//! The water and steam properties at saturation are canonical math, `+ − × /`,
//! `sqrt` and the kernel's `exp` and `log`: the saturation temperature by
//! IAPWS-IF97 region 4 (backward equation 31), the saturated densities, the
//! vapour pressure and its slope by the IAPWS auxiliary equations of Wagner
//! and Pruss (1993), the latent heat by Clausius–Clapeyron from them, and the
//! surface tension by the IAPWS 1994 equation. They hold from the triple
//! point, 611.657 Pa, to the critical point, 22.064 MPa: a system pressure
//! outside that is refused as `PressureOutsideSteamTables`. Measured against
//! steam tables at 100, 200, 300 and 350 degC they agree within 0.1 percent
//! (densities, latent heat) and 2.5 percent (surface tension at 350 degC).
//!
//! Fitted on the 111 usable transients of IDO-16918 (eight constants, below);
//! fixed are `f_w` = 0.026, the share of fission energy the water takes
//! directly (the slowing down of fission neutrons), and `τ_c` = 3 ms, the
//! condensation time of the vapour in subcooled water.
//!
//! The ledger has four balances, each closed on every step:
//!
//! ```text
//! neutrons  N − N_0 − (ρ_0 E − Q) / l*,        N = P + Σ C_i,  Q = ∫ (ρ_0 − ρ) P dt
//! node j    F_j E − C_p T_p − C_f T_f − C_b T_w − U − S_j,   S_j = ∫ s (C_f T_f + C_b T_w + U) dt
//! ```
//!
//! for the core average, the feedback node and the hot spot. `Q` and `S_j`
//! are taken over each step by the two-point Hermite rule from the values
//! and rates at the step's two ends; the Runge–Kutta step never evaluates
//! them, so a wrong method or a falsified term shows in the neutron balance.
//! The residual is the largest of the four, each relative to its own scale.
//!
//! [`ReactorSpert3CcoreTwin::gate_dt_s`] is 0.2 l* = 0.6 ms;
//! [`ReactorSpert3CcoreTwin::floor_dt_s`] is 1e-9 s. The step refuses as
//! `DtAboveStateCeiling` a step that would advance, at the state it starts
//! from or at its trial end, the growing prompt pole by more than 0.0611,
//! the decaying prompt pole by more than 0.5, or the fastest node's heat
//! transfer, boiling, condensation and sweep by more than
//! [`THERMAL_PHASE_STEP`]: the most a 0.6 ms step advances each over the 111
//! compared transients, rounded up.
//! `core` only, with `exp` and `log` from [`crate::math`] and `sqrt` from the
//! platform (IEEE 754, correctly rounded).

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
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
/// Flux peak-to-average ratio at the thermocouples, axial 2.04 times radial 1.30 (IDO-17138 Table B-1).
pub const HOT_SPOT_PEAKING: f64 = 2.04 * 1.30;
/// Plate heat-transfer area, m²: 30 MW over the average core heat flux 9.72e4 Btu/hr ft² (Table B-1).
pub const HEAT_TRANSFER_AREA_M2: f64 = 97.84;
/// Heat capacity of the plates over that area, MW s per K (UO2 and type 304L stainless steel).
pub const PLATE_HEAT_CAPACITY_MWS_PER_K: f64 = 0.14235;
/// Heat capacity of the water between the plates, MW s per K: channels 0.128 in wide, water at 25 degC.
pub const WATER_HEAT_CAPACITY_MWS_PER_K: f64 = 0.66306;
/// Active length, m: 36 in.
pub const ACTIVE_LENGTH_M: f64 = 0.9144;
/// Reactivity per unit fractional decrease of moderator density, dollars: 44.0 cents per percent (Table B-1).
pub const DENSITY_COEFFICIENT_DOLLAR: f64 = 44.0;
/// Average void coefficient, dollars per unit void fraction: 45 cents per percent (IDO-16918 Table I).
pub const VOID_COEFFICIENT_DOLLAR: f64 = 45.0;
/// Water displaced per kelvin of plate above the water: 3 × 17.3e-6 /K × 0.030 in / 0.128 in.
pub const PLATE_DISPLACEMENT_PER_K: f64 = 3.0 * 17.3e-6 * 0.030 / 0.128;
/// Temperatures of the measured isothermal coefficient, degC: 100, 400, 500, 600 and 650 degF.
pub const COEFFICIENT_TEMPERATURE_C: [f64; 5] = [37.78, 204.44, 260.0, 315.56, 343.33];
/// The isothermal coefficient there, dollars per K, sign removed: 1.8 cents/degC (IDO-16918 Table I, flat
/// below 37.78 degC), then 3.2, 4.6, 8.2 and 16.0 cents/degF (IDO-17138 Table B-1).
pub const COEFFICIENT_DOLLAR_PER_K: [f64; 5] = [0.018, 0.0576, 0.0828, 0.1476, 0.288];
/// Hydraulic diameter of the channel between two plates, m: 2 × 0.128 in.
pub const HYDRAULIC_DIAMETER_M: f64 = 2.0 * 0.128 * 0.0254;
/// Water at 25 degC for Dittus–Boelter and the water volume: density kg/m³, viscosity Pa s,
/// conductivity W/m K, Prandtl number, specific heat J/kg K.
pub const WATER_DENSITY_KG_PER_M3: f64 = 997.05;
pub const WATER_VISCOSITY_PA_S: f64 = 8.90e-4;
pub const WATER_CONDUCTIVITY_W_PER_M_K: f64 = 0.6071;
pub const WATER_PRANDTL: f64 = 6.13;
pub const WATER_SPECIFIC_HEAT_J_PER_KG_K: f64 = 4181.3;
/// Share of fission energy deposited directly in the water, `f_w`: the slowing down of fission
/// neutrons, about 4.8 MeV of 185. Fixed.
pub const WATER_DIRECT_HEATING_FRACTION: f64 = 0.026;
/// Condensation time of the vapour in subcooled water, `τ_c`, s. Fixed.
pub const CONDENSATION_TIME_S: f64 = 0.003;
/// Plate-to-film conductance with no flow, `h_s`, W/m² K. Fitted.
pub const PLATE_FILM_CONDUCTANCE_W_PER_M2_K: f64 = 5315.50;
/// Film-to-bulk conductance with no flow, `h_fb`, W/m² K. Fitted.
pub const FILM_BULK_CONDUCTANCE_W_PER_M2_K: f64 = 240.502;
/// Width over which the film's moderator term bends onto the saturation temperature, K: the film
/// boils there, and its vapour is the void term.
pub const FILM_SATURATION_WIDTH_K: f64 = 3.0;
/// Share of the water in the film next to the plates, `φ`. Fitted.
pub const FILM_SHARE: f64 = 0.191875;
/// Peaking of the feedback node, `F_r`: the core's temperatures as its reactivity weighs them. Fitted.
pub const FEEDBACK_PEAKING: f64 = 2.10289;
/// The plate's own temperature coefficient beyond the water it displaces, `a_D`, dollars per K
/// (Doppler and plate bowing). Fitted.
pub const PLATE_COEFFICIENT_DOLLAR_PER_K: f64 = 0.000362933;
/// Multiplier of the Thom nucleate-boiling flux, `m_n`. Fitted.
pub const NUCLEATE_MULTIPLIER: f64 = 2.63504;
/// Multiplier of the Zuber critical heat flux with the Ivey–Morris subcooling term, `m_c`. Fitted.
pub const CRITICAL_FLUX_MULTIPLIER: f64 = 2.06650;
/// Share of the boiling heat that stays as vapour, `ε`; the rest condenses at once in the film. Fitted.
pub const NET_VAPOUR_SHARE: f64 = 0.000159156;
/// Thom correlation: wall superheat 22.65 K at 1 MW/m², falling by e every 8.7 MPa.
pub const THOM_SUPERHEAT_K: f64 = 22.65;
pub const THOM_PRESSURE_MPA: f64 = 8.7;
/// Zuber's constant, the Ivey–Morris subcooling constant and exponent, and gravity, m/s².
pub const ZUBER_CONSTANT: f64 = 0.131;
pub const IVEY_MORRIS_CONSTANT: f64 = 0.1;
pub const IVEY_MORRIS_EXPONENT: f64 = 0.75;
pub const GRAVITY_M_PER_S2: f64 = 9.80665;
/// Compared initial temperatures, K: 21 to 36 degC. Not a limit.
pub const INITIAL_TEMPERATURE_MIN_K: f64 = 21.0 + 273.15;
pub const INITIAL_TEMPERATURE_MAX_K: f64 = 36.0 + 273.15;
/// Compared system pressures, psig: 0 to 2500 with no flow. Not a limit.
pub const PRESSURE_MIN_PSIG: f64 = 0.0;
pub const PRESSURE_MAX_PSIG: f64 = 2500.0;
/// System pressure of every compared flow transient, psig. Not a limit.
pub const FLOW_PRESSURE_PSIG: f64 = 230.0;
/// Compared coolant velocities, m/s: 200 and 20,000 gpm at 18 ft/s per 20,000 gpm. Not a limit.
pub const FLOW_MIN_M_PER_S: f64 = 0.054864;
pub const FLOW_MAX_M_PER_S: f64 = 5.486400000000001;
/// Smallest compared reactivity with no flow, dollars: the inhour value of 9.3 s. Not a limit.
pub const REACTIVITY_MIN_NO_FLOW_DOLLAR: f64 = 0.4122874385206243;
/// Smallest compared reactivity with flow, dollars: the inhour value of 516 ms. Not a limit.
pub const REACTIVITY_MIN_FLOW_DOLLAR: f64 = 0.870348183353308;
/// Largest compared reactivity, dollars: the inhour value of 9.7 ms. Not a limit.
pub const REACTIVITY_MAX_DOLLAR: f64 = 1.3053996316244825;
/// Freezing point of water, K: below it the start is not liquid water.
pub const FREEZING_K: f64 = 273.15;
/// One standard atmosphere, psi, and one psi in MPa.
pub const ATMOSPHERE_PSI: f64 = 14.6959488;
pub const MPA_PER_PSI: f64 = 0.00689475729;
/// Triple-point pressure of water, MPa: the lowest pressure the steam tables hold.
pub const TRIPLE_POINT_PRESSURE_MPA: f64 = 0.000611657;
/// Critical pressure, MPa, temperature, K, and density, kg/m³, of water (IAPWS).
pub const CRITICAL_PRESSURE_MPA: f64 = 22.064;
pub const CRITICAL_TEMPERATURE_K: f64 = 647.096;
pub const CRITICAL_DENSITY_KG_PER_M3: f64 = 322.0;
/// IAPWS-IF97 region 4, `n_1` to `n_10`, read by its backward equation (eq. 31).
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
/// Wagner and Pruss (1993), vapour pressure: `a_1` to `a_6` for τ, τ^1.5, τ^3, τ^3.5, τ^4, τ^7.5.
pub const VAPOUR_PRESSURE_A: [f64; 6] = [-7.85951783, 1.84408259, -11.7866497, 22.6807411, -15.9618719, 1.80122502];
/// Wagner and Pruss, saturated liquid density: `b_1` to `b_6` for τ^(1/3, 2/3, 5/3, 16/3, 43/3, 110/3).
pub const LIQUID_DENSITY_B: [f64; 6] = [1.99274064, 1.09965342, -0.510839303, -1.75493479, -45.5170352, -6.74694450e5];
/// Wagner and Pruss, saturated vapour density: `c_1` to `c_6` for τ^(2/6, 4/6, 8/6, 18/6, 37/6, 71/6).
pub const VAPOUR_DENSITY_C: [f64; 6] = [-2.03150240, -2.68302940, -5.38626492, -17.2991605, -44.7586581, -63.9201063];
/// IAPWS 1994 surface tension: 235.8 mN/m, exponent 1.256, b = −0.625.
pub const SURFACE_TENSION_N_PER_M: f64 = 0.2358;
pub const SURFACE_TENSION_EXPONENT: f64 = 1.256;
pub const SURFACE_TENSION_B: f64 = -0.625;
/// The ledger's residual must stay at or under this. It names a wrong method; it is not a bound
/// on the error of the state.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 3.5e-6;
/// Fraction of `l*` one step at the ceiling advances.
pub const GATE_PHASE_STEP: f64 = 0.2;
/// Growing prompt pole `(ρ_net − 1) / l*` at the largest compared reactivity, 1.305 dollars, rounded up.
pub const GROWTH_PHASE_STEP: f64 = 0.0611;
/// Decaying prompt pole `(1 − ρ_net) / l*`.
pub const DECAY_PHASE_STEP: f64 = 0.5;
/// The fastest node's `H_1/C_p + (H_1 + H_2)/C_f + H_2/C_b + A q_b'/C_p + 1/τ_c + 2 s`.
pub const THERMAL_PHASE_STEP: f64 = 0.6;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-9;

/// Net reactivity is above one dollar on this frame.
pub const FLAG_PROMPT_SUPERCRITICAL: u64 = 1 << 0;
/// The first maximum of power has been passed.
pub const FLAG_PAST_POWER_PEAK: u64 = 1 << 1;
/// The plate at the hot spot is above the saturation temperature at the system pressure: it boils.
pub const FLAG_HOT_SPOT_BOILING: u64 = 1 << 2;
/// The start or the load is outside the span the card was fitted and compared on.
pub const FLAG_OUTSIDE_COMPARED_RANGE: u64 = 1 << 3;

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
    /// The system pressure is outside the steam tables the boiling term stands on: below the triple point of water, 611.657 Pa absolute (about −14.6072 psig), or at or above the critical pressure, 22.064 MPa (about 3185.4 psig), where water has no saturation temperature.
    PressureOutsideSteamTables,
    /// The initial temperature is not that of liquid water at the system pressure: below 273.15 K, or at or above the saturation temperature of water at that pressure (IAPWS-IF97 region 4).
    InitialTemperatureNotLiquidWater,
    /// The coolant flow is below zero.
    NegativeFlow,
    /// The held reactivity is zero or less: no step in reactivity is inserted, and there is no burst.
    ReactivityNotPositive,
    /// The vapour in a node's channels would fill them: a void fraction of one or more, where there is no water left between the plates.
    VoidFractionAboveOne,
    /// The state is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card.
    PastPlateTemperatureMaximum,
    /// The incoming state's clock is below zero.
    NegativeTime,
    /// The incoming state holds an energy, a vapour store, a swept energy, a precursor or a flow below zero, or a power not above zero.
    NegativeState,
    /// The step is longer than the state lets it be, at the state it starts from or at its trial end: one Runge–Kutta step would advance the growing prompt pole (net reactivity − 1) / l* by more than 0.0611 of its time constant, the decaying prompt pole (1 − net reactivity) / l* by more than 0.5, or the fastest node’s heat transfer, boiling, condensation and sweep by more than 0.6.
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

/// What the operator sets for one transient. The reactivity is recorded by the first landed step and held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorLoad {
    /// Dollars of reactivity stepped in at time zero, `ρ_0`.
    pub reactivity_inserted_dollar: f64,
    /// Coolant velocity through the core, `v`; 20,000 gpm is 18 ft/s.
    pub coolant_flow_m_per_s: f64,
}

/// The number of scalar constants.
const N_SCALARS: usize = 82;

/// The locked reactor. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorSpert3CcoreTwin {
    pub group_fraction: [f64; 6],
    pub group_decay_per_s: [f64; 6],
    pub constants: [f64; N_SCALARS],
}

const K_L_STAR: usize = 0;
const K_P0: usize = 1;
const K_PEAKING: usize = 2;
const K_AREA: usize = 3;
const K_CP: usize = 4;
const K_CW: usize = 5;
const K_LENGTH: usize = 6;
const K_DENSITY: usize = 7;
const K_VOID: usize = 8;
const K_DISPLACEMENT: usize = 9;
const K_COEF_T: usize = 10; // 5
const K_COEF_A: usize = 15; // 5
const K_DH: usize = 20;
const K_RHO_W: usize = 21;
const K_MU_W: usize = 22;
const K_K_W: usize = 23;
const K_PR_W: usize = 24;
const K_CPL: usize = 25;
const K_FW: usize = 26;
const K_TAU: usize = 27;
const K_HS: usize = 28;
const K_HFB: usize = 29;
const K_PHI: usize = 30;
const K_FR: usize = 31;
const K_AD: usize = 32;
const K_MN: usize = 33;
const K_MC: usize = 34;
const K_EPS: usize = 35;
const K_THOM_T: usize = 36;
const K_THOM_P: usize = 37;
const K_ZUBER: usize = 38;
const K_IM: usize = 39;
const K_IM_EXP: usize = 40;
const K_G: usize = 41;
const K_FREEZING: usize = 42;
const K_ATMOSPHERE: usize = 43;
const K_MPA_PER_PSI: usize = 44;
const K_TRIPLE_P: usize = 45;
const K_CRITICAL_P: usize = 46;
const K_CRITICAL_T: usize = 47;
const K_CRITICAL_RHO: usize = 48;
const K_SATURATION: usize = 49; // 10
const K_VP_A: usize = 59; // 6
const K_LD_B: usize = 65; // 6
const K_VD_C: usize = 71; // 6
const K_SIGMA: usize = 77;
const K_SIGMA_EXP: usize = 78;
const K_SIGMA_B: usize = 79;
const K_RESIDUAL: usize = 80;
const K_FILM_W: usize = 81;

/// The scalar constants in one list, in the order `is_valid` and `terms` read them.
const SCALARS: [f64; N_SCALARS] = [
    GENERATION_TIME_REDUCED_S,
    INITIAL_POWER_MW,
    HOT_SPOT_PEAKING,
    HEAT_TRANSFER_AREA_M2,
    PLATE_HEAT_CAPACITY_MWS_PER_K,
    WATER_HEAT_CAPACITY_MWS_PER_K,
    ACTIVE_LENGTH_M,
    DENSITY_COEFFICIENT_DOLLAR,
    VOID_COEFFICIENT_DOLLAR,
    PLATE_DISPLACEMENT_PER_K,
    COEFFICIENT_TEMPERATURE_C[0],
    COEFFICIENT_TEMPERATURE_C[1],
    COEFFICIENT_TEMPERATURE_C[2],
    COEFFICIENT_TEMPERATURE_C[3],
    COEFFICIENT_TEMPERATURE_C[4],
    COEFFICIENT_DOLLAR_PER_K[0],
    COEFFICIENT_DOLLAR_PER_K[1],
    COEFFICIENT_DOLLAR_PER_K[2],
    COEFFICIENT_DOLLAR_PER_K[3],
    COEFFICIENT_DOLLAR_PER_K[4],
    HYDRAULIC_DIAMETER_M,
    WATER_DENSITY_KG_PER_M3,
    WATER_VISCOSITY_PA_S,
    WATER_CONDUCTIVITY_W_PER_M_K,
    WATER_PRANDTL,
    WATER_SPECIFIC_HEAT_J_PER_KG_K,
    WATER_DIRECT_HEATING_FRACTION,
    CONDENSATION_TIME_S,
    PLATE_FILM_CONDUCTANCE_W_PER_M2_K,
    FILM_BULK_CONDUCTANCE_W_PER_M2_K,
    FILM_SHARE,
    FEEDBACK_PEAKING,
    PLATE_COEFFICIENT_DOLLAR_PER_K,
    NUCLEATE_MULTIPLIER,
    CRITICAL_FLUX_MULTIPLIER,
    NET_VAPOUR_SHARE,
    THOM_SUPERHEAT_K,
    THOM_PRESSURE_MPA,
    ZUBER_CONSTANT,
    IVEY_MORRIS_CONSTANT,
    IVEY_MORRIS_EXPONENT,
    GRAVITY_M_PER_S2,
    FREEZING_K,
    ATMOSPHERE_PSI,
    MPA_PER_PSI,
    TRIPLE_POINT_PRESSURE_MPA,
    CRITICAL_PRESSURE_MPA,
    CRITICAL_TEMPERATURE_K,
    CRITICAL_DENSITY_KG_PER_M3,
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
    VAPOUR_PRESSURE_A[0],
    VAPOUR_PRESSURE_A[1],
    VAPOUR_PRESSURE_A[2],
    VAPOUR_PRESSURE_A[3],
    VAPOUR_PRESSURE_A[4],
    VAPOUR_PRESSURE_A[5],
    LIQUID_DENSITY_B[0],
    LIQUID_DENSITY_B[1],
    LIQUID_DENSITY_B[2],
    LIQUID_DENSITY_B[3],
    LIQUID_DENSITY_B[4],
    LIQUID_DENSITY_B[5],
    VAPOUR_DENSITY_C[0],
    VAPOUR_DENSITY_C[1],
    VAPOUR_DENSITY_C[2],
    VAPOUR_DENSITY_C[3],
    VAPOUR_DENSITY_C[4],
    VAPOUR_DENSITY_C[5],
    SURFACE_TENSION_N_PER_M,
    SURFACE_TENSION_EXPONENT,
    SURFACE_TENSION_B,
    RESIDUAL_CONSERVATION_LIMIT,
    FILM_SATURATION_WIDTH_K,
];

/// The step-ceiling constants, kept apart from [`SCALARS`] so their indices stay readable.
const CEILING: [f64; 5] = [GATE_PHASE_STEP, GROWTH_PHASE_STEP, DECAY_PHASE_STEP, THERMAL_PHASE_STEP, FLOOR_DT_S];

/// `x^y` for `x > 0` as `exp(y log x)`, from the kernel's own functions.
fn power(x: f64, y: f64) -> f64 {
    crate::math::exp(y * crate::math::log(x))
}

/// Water and steam at saturation at one pressure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saturation {
    /// Absolute pressure, MPa.
    pub pressure_mpa: f64,
    /// Saturation temperature, K (IAPWS-IF97 eq. 31).
    pub temperature_k: f64,
    /// Saturated liquid and vapour densities, kg/m³ (Wagner and Pruss).
    pub liquid_density_kg_per_m3: f64,
    pub vapour_density_kg_per_m3: f64,
    /// Latent heat, J/kg (Clausius–Clapeyron).
    pub latent_heat_j_per_kg: f64,
    /// Surface tension, N/m (IAPWS 1994).
    pub surface_tension_n_per_m: f64,
}

impl ReactorSpert3CcoreTwin {
    /// Card: the SPERT III C-19/52 core with subcooled boiling and void. No arguments.
    #[must_use]
    pub fn c_core_twin() -> Self {
        let reactor = Self { group_fraction: GROUP_FRACTION, group_decay_per_s: GROUP_DECAY_PER_S, constants: SCALARS };
        if reactor.is_valid() {
            reactor
        } else {
            ::core::panic!("C-core twin failed is_valid");
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
        while index < N_SCALARS {
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

    /// Dittus–Boelter for water at 25 degC between the plates, W/m² K; zero with no flow.
    #[must_use]
    pub fn dittus_boelter_w_per_m2_k(&self, coolant_flow_m_per_s: f64) -> f64 {
        let c = &self.constants;
        if coolant_flow_m_per_s > 0.0 {
            let reynolds = c[K_RHO_W] * coolant_flow_m_per_s * c[K_DH] / c[K_MU_W];
            0.023 * power(reynolds, 0.8) * power(c[K_PR_W], 0.4) * c[K_K_W] / c[K_DH]
        } else {
            0.0
        }
    }

    /// Largest step the card stands behind at every state of the compared range: 0.2 l*, 0.6 ms.
    /// Not applied by [`step`]; the state ceiling of [`step`] is.
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let l_star = self.constants[K_L_STAR];
        if l_star.is_finite() && l_star > 0.0 && CEILING[0] > 0.0 { Some(CEILING[0] * l_star) } else { None }
    }

    /// Smallest step the card stands behind: [`FLOOR_DT_S`]. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        let floor = CEILING[4];
        if floor.is_finite() && floor > 0.0 { Some(floor) } else { None }
    }

    /// Absolute pressure in MPa of a gauge pressure in psig.
    #[must_use]
    pub fn absolute_pressure_mpa(&self, system_pressure_psig: f64) -> f64 {
        (system_pressure_psig + self.constants[K_ATMOSPHERE]) * self.constants[K_MPA_PER_PSI]
    }

    /// Whether the steam tables hold at this gauge pressure: from the triple point, inclusive, to
    /// the critical pressure, exclusive.
    #[must_use]
    pub fn pressure_in_steam_tables(&self, system_pressure_psig: f64) -> bool {
        let p = self.absolute_pressure_mpa(system_pressure_psig);
        p >= self.constants[K_TRIPLE_P] && p < self.constants[K_CRITICAL_P]
    }

    /// The saturation temperature of water at a gauge pressure, K: IAPWS-IF97 region 4, backward
    /// equation (eq. 31). Meant for pressures in the steam tables.
    #[must_use]
    pub fn saturation_temperature_k(&self, system_pressure_psig: f64) -> f64 {
        let c = &self.constants;
        let p = self.absolute_pressure_mpa(system_pressure_psig);
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

    /// Water and steam at saturation at a gauge pressure. `None` outside the steam tables
    /// (`PressureOutsideSteamTables`) or where a value is not finite and positive.
    #[must_use]
    pub fn saturation(&self, system_pressure_psig: f64) -> Option<Saturation> {
        if !self.pressure_in_steam_tables(system_pressure_psig) {
            return None;
        }
        let c = &self.constants;
        let pressure_mpa = self.absolute_pressure_mpa(system_pressure_psig);
        let t = self.saturation_temperature_k(system_pressure_psig);
        let tc = c[K_CRITICAL_T];
        let tau = 1.0 - t / tc;
        if !(t.is_finite() && tau > 0.0 && tau < 1.0) {
            return None;
        }
        let a = |i: usize| c[K_VP_A + i];
        let b = |i: usize| c[K_LD_B + i];
        let cc = |i: usize| c[K_VD_C + i];
        let st = tau.sqrt();
        // Vapour pressure, ln(p/p_c) = (T_c/T) S(τ), and its slope.
        let s = a(0) * tau + a(1) * tau * st + a(2) * tau * tau * tau + a(3) * tau * tau * tau * st + a(4) * power(tau, 4.0) + a(5) * power(tau, 7.5);
        let ds = a(0) + 1.5 * a(1) * st + 3.0 * a(2) * tau * tau + 3.5 * a(3) * tau * tau * st + 4.0 * a(4) * tau * tau * tau + 7.5 * a(5) * power(tau, 6.5);
        let p_sat = c[K_CRITICAL_P] * crate::math::exp(tc / t * s);
        // d/dT of (T_c/T) S(1 − T/T_c) = −T_c S / T² − ds / T.
        let dp_dt = p_sat * (-tc / (t * t) * s - ds / t);
        let liquid = c[K_CRITICAL_RHO]
            * (1.0
                + b(0) * power(tau, 1.0 / 3.0)
                + b(1) * power(tau, 2.0 / 3.0)
                + b(2) * power(tau, 5.0 / 3.0)
                + b(3) * power(tau, 16.0 / 3.0)
                + b(4) * power(tau, 43.0 / 3.0)
                + b(5) * power(tau, 110.0 / 3.0));
        let vapour = c[K_CRITICAL_RHO]
            * crate::math::exp(
                cc(0) * power(tau, 2.0 / 6.0)
                    + cc(1) * power(tau, 4.0 / 6.0)
                    + cc(2) * power(tau, 8.0 / 6.0)
                    + cc(3) * power(tau, 18.0 / 6.0)
                    + cc(4) * power(tau, 37.0 / 6.0)
                    + cc(5) * power(tau, 71.0 / 6.0),
            );
        let latent = t * dp_dt * 1.0e6 * (1.0 / vapour - 1.0 / liquid);
        let sigma = c[K_SIGMA] * power(tau, c[K_SIGMA_EXP]) * (1.0 + c[K_SIGMA_B] * tau);
        let all = [t, liquid, vapour, latent, sigma];
        let mut index = 0;
        while index < 5 {
            if !(all[index].is_finite() && all[index] > 0.0) {
                return None;
            }
            index += 1;
        }
        if !(liquid > vapour) {
            return None;
        }
        Some(Saturation {
            pressure_mpa,
            temperature_k: t,
            liquid_density_kg_per_m3: liquid,
            vapour_density_kg_per_m3: vapour,
            latent_heat_j_per_kg: latent,
            surface_tension_n_per_m: sigma,
        })
    }

    /// Whether a start at this temperature and gauge pressure is liquid water: at or above
    /// 273.15 K and below the saturation temperature at that pressure.
    #[must_use]
    pub fn initial_temperature_is_liquid_water(&self, initial_temperature_k: f64, system_pressure_psig: f64) -> bool {
        initial_temperature_k >= self.constants[K_FREEZING]
            && initial_temperature_k < self.saturation_temperature_k(system_pressure_psig)
    }

    /// Whether a start and a load are inside the span the card was fitted and compared on.
    #[must_use]
    pub fn compared(&self, initial_temperature_k: f64, system_pressure_psig: f64, load: ReactorLoad) -> bool {
        let start = initial_temperature_k >= INITIAL_TEMPERATURE_MIN_K
            && initial_temperature_k <= INITIAL_TEMPERATURE_MAX_K
            && system_pressure_psig >= PRESSURE_MIN_PSIG
            && system_pressure_psig <= PRESSURE_MAX_PSIG;
        let v = load.coolant_flow_m_per_s;
        let rho = load.reactivity_inserted_dollar;
        let flow = if v == 0.0 {
            rho >= REACTIVITY_MIN_NO_FLOW_DOLLAR && rho <= REACTIVITY_MAX_DOLLAR
        } else {
            system_pressure_psig == FLOW_PRESSURE_PSIG
                && v >= FLOW_MIN_M_PER_S
                && v <= FLOW_MAX_M_PER_S
                && rho >= REACTIVITY_MIN_FLOW_DOLLAR
                && rho <= REACTIVITY_MAX_DOLLAR
        };
        start && flow
    }
}

/// The 20 variables of the Runge–Kutta step: P, C_1 to C_6, E, then for the core average, the
/// feedback node and the hot spot in turn: T_p, T_f, T_w, U.
const NV: usize = 20;
const NODE0: usize = 8;
/// The feedback node's and the hot spot's first index.
const FEEDBACK: usize = 12;
const HOT: usize = 16;

/// The constants of one start and one load, gathered for the step.
#[derive(Clone, Copy)]
struct Terms {
    l_star: f64,
    rho_0: f64,
    initial_c: f64,
    base: f64,
    film_base: f64,
    film_width: f64,
    saturation_c: f64,
    saturation_k: f64,
    initial_k: f64,
    peaking: [f64; 3],
    plate_share: f64,
    water_share: f64,
    film_share: f64,
    h1: f64,
    h2: f64,
    cp: f64,
    cf: f64,
    cb: f64,
    sweep: f64,
    tau: f64,
    eps: f64,
    displacement: f64,
    plate_coefficient: f64,
    void_per_mws: f64,
    void_coefficient: f64,
    area: f64,
    nucleate_per_k2: f64,
    critical_mw_per_m2: f64,
    subcooling_per_k: f64,
    coef_t: [f64; 5],
    coef_a: [f64; 5],
}

/// `None` where the steam tables do not hold at the start's pressure.
fn terms(reactor: &ReactorSpert3CcoreTwin, initial_temperature_k: f64, system_pressure_psig: f64, load: ReactorLoad) -> Option<Terms> {
    let c = &reactor.constants;
    let sat = reactor.saturation(system_pressure_psig)?;
    let h_db = reactor.dittus_boelter_w_per_m2_k(load.coolant_flow_m_per_s);
    let thom = c[K_THOM_T] * crate::math::exp(-sat.pressure_mpa / c[K_THOM_P]);
    let rg = sat.vapour_density_kg_per_m3;
    let rf = sat.liquid_density_kg_per_m3;
    let hfg = sat.latent_heat_j_per_kg;
    let zuber = c[K_ZUBER] * rg.sqrt() * hfg * (sat.surface_tension_n_per_m * c[K_G] * (rf - rg)).sqrt().sqrt() / 1.0e6;
    let water_volume_m3 = c[K_CW] * 1.0e6 / (c[K_RHO_W] * c[K_CPL]);
    let mut t = Terms {
        l_star: c[K_L_STAR],
        rho_0: load.reactivity_inserted_dollar,
        initial_c: initial_temperature_k - 273.15,
        base: 0.0,
        film_base: 0.0,
        film_width: c[K_FILM_W],
        saturation_c: sat.temperature_k - 273.15,
        saturation_k: sat.temperature_k,
        initial_k: initial_temperature_k,
        peaking: [1.0, c[K_FR], c[K_PEAKING]],
        plate_share: 1.0 - c[K_FW],
        water_share: c[K_FW],
        film_share: c[K_PHI],
        h1: c[K_AREA] * (c[K_HS] * c[K_HS] + h_db * h_db).sqrt() / 1.0e6,
        h2: c[K_AREA] * (c[K_HFB] * c[K_HFB] + h_db * h_db).sqrt() / 1.0e6,
        cp: c[K_CP],
        cf: c[K_PHI] * c[K_CW],
        cb: (1.0 - c[K_PHI]) * c[K_CW],
        sweep: 2.0 * load.coolant_flow_m_per_s / c[K_LENGTH],
        tau: c[K_TAU],
        eps: c[K_EPS],
        displacement: c[K_DENSITY] * c[K_DISPLACEMENT],
        plate_coefficient: c[K_AD],
        void_per_mws: 1.0e6 / (rg * hfg * water_volume_m3),
        void_coefficient: c[K_VOID],
        area: c[K_AREA],
        nucleate_per_k2: c[K_MN] / (thom * thom),
        critical_mw_per_m2: c[K_MC] * zuber,
        subcooling_per_k: c[K_IM] * power(rf / rg, c[K_IM_EXP]) * c[K_CPL] / hfg,
        coef_t: [c[K_COEF_T], c[K_COEF_T + 1], c[K_COEF_T + 2], c[K_COEF_T + 3], c[K_COEF_T + 4]],
        coef_a: [c[K_COEF_A], c[K_COEF_A + 1], c[K_COEF_A + 2], c[K_COEF_A + 3], c[K_COEF_A + 4]],
    };
    t.base = coefficient_integral(&t, t.initial_c);
    let (film_start, _) = film_c(&t, 0.0);
    t.film_base = coefficient_integral(&t, film_start);
    Some(t)
}

/// The isothermal coefficient at `temperature_c`, dollars per K: flat to the first point, straight
/// between the points, the last segment continued.
fn coefficient(terms: &Terms, temperature_c: f64) -> f64 {
    let (ct, ca) = (&terms.coef_t, &terms.coef_a);
    if temperature_c <= ct[0] {
        return ca[0];
    }
    let mut i = 0;
    while i < 3 && temperature_c > ct[i + 1] {
        i += 1;
    }
    ca[i] + (ca[i + 1] - ca[i]) / (ct[i + 1] - ct[i]) * (temperature_c - ct[i])
}

/// The integral of the coefficient from 0 degC to `temperature_c`, dollars.
fn coefficient_integral(terms: &Terms, temperature_c: f64) -> f64 {
    let (ct, ca) = (&terms.coef_t, &terms.coef_a);
    if temperature_c <= ct[0] {
        return ca[0] * temperature_c;
    }
    let mut sum = ca[0] * ct[0];
    let mut i = 0;
    loop {
        let slope = (ca[i + 1] - ca[i]) / (ct[i + 1] - ct[i]);
        let last = i == 3 || temperature_c <= ct[i + 1];
        let top = if last { temperature_c } else { ct[i + 1] };
        let x = top - ct[i];
        sum += ca[i] * x + 0.5 * slope * x * x;
        if last {
            return sum;
        }
        i += 1;
    }
}

/// The boiling heat flux of one node, MW/m², and its slope in the wall temperature, MW/m² K.
fn boiling(terms: &Terms, plate_rise_k: f64, water_rise_k: f64) -> (f64, f64) {
    let superheat = terms.initial_k + plate_rise_k - terms.saturation_k;
    if !(superheat > 0.0) {
        return (0.0, 0.0);
    }
    let mut subcooling = terms.saturation_k - (terms.initial_k + water_rise_k);
    if subcooling < 0.0 {
        subcooling = 0.0;
    }
    let qn = terms.nucleate_per_k2 * superheat * superheat;
    let qc = terms.critical_mw_per_m2 * (1.0 + terms.subcooling_per_k * subcooling);
    let sum = qn + qc;
    let q = qn * qc / sum;
    let slope = qc * qc * (2.0 * qn / superheat) / (sum * sum);
    (q, slope)
}

/// The film's temperature in degC as the reactivity takes it, and its slope in the film's own:
/// `T_sat − w ln(1 + e^((T_sat − θ)/w))`, which follows the film below saturation and bends
/// smoothly onto the saturation temperature above it, over [`FILM_SATURATION_WIDTH_K`].
fn film_c(terms: &Terms, film_rise_k: f64) -> (f64, f64) {
    let w = terms.film_width;
    let x = (terms.saturation_c - (terms.initial_c + film_rise_k)) / w;
    let (softplus, slope) = if x > 0.0 {
        let e = crate::math::exp(-x);
        (x + crate::math::ln_1p(e), 1.0 / (1.0 + e))
    } else {
        let e = crate::math::exp(x);
        (crate::math::ln_1p(e), e / (1.0 + e))
    };
    (terms.saturation_c - w * softplus, slope)
}

/// The parts of `ρ_0 − ρ` on the feedback node: moderator (film and bulk), plate, void.
fn feedback_parts(terms: &Terms, y: &[f64; NV]) -> [f64; 3] {
    let (tp, tf, tw, u) = (y[FEEDBACK], y[FEEDBACK + 1], y[FEEDBACK + 2], y[FEEDBACK + 3]);
    let (film, _) = film_c(terms, tf);
    let moderator = terms.film_share * (coefficient_integral(terms, film) - terms.film_base)
        + (1.0 - terms.film_share) * (coefficient_integral(terms, terms.initial_c + tw) - terms.base);
    let plate = terms.displacement * (tp - tw) + terms.plate_coefficient * tp;
    let void = terms.void_coefficient * terms.void_per_mws * u;
    [moderator, plate, void]
}

/// `ρ_0 − ρ`: the reactivity the temperatures and the vapour take back.
fn feedback(terms: &Terms, y: &[f64; NV]) -> f64 {
    let parts = feedback_parts(terms, y);
    parts[0] + parts[1] + parts[2]
}

/// The rate of `ρ_0 − ρ` at `y` with rates `k`.
fn feedback_rate(terms: &Terms, y: &[f64; NV], k: &[f64; NV]) -> f64 {
    let (tf, tw) = (y[FEEDBACK + 1], y[FEEDBACK + 2]);
    let (film, slope) = film_c(terms, tf);
    let film_rate = coefficient(terms, film) * slope * k[FEEDBACK + 1];
    terms.film_share * film_rate
        + (1.0 - terms.film_share) * coefficient(terms, terms.initial_c + tw) * k[FEEDBACK + 2]
        + terms.displacement * (k[FEEDBACK] - k[FEEDBACK + 2])
        + terms.plate_coefficient * k[FEEDBACK]
        + terms.void_coefficient * terms.void_per_mws * k[FEEDBACK + 3]
}

/// The right-hand side of the equations at `y`.
fn rates(reactor: &ReactorSpert3CcoreTwin, terms: &Terms, y: &[f64; NV]) -> [f64; NV] {
    let rho_net = terms.rho_0 - feedback(terms, y);
    let mut out = [0.0; NV];
    let mut delayed = 0.0;
    let mut index = 0;
    while index < 6 {
        let decay = reactor.group_decay_per_s[index];
        delayed += decay * y[1 + index];
        out[1 + index] = reactor.group_fraction[index] / terms.l_star * y[0] - decay * y[1 + index];
        index += 1;
    }
    let p = y[0];
    out[0] = (rho_net - 1.0) / terms.l_star * p + delayed;
    out[7] = p;
    let mut node = 0;
    while node < 3 {
        let b = NODE0 + 4 * node;
        let f = terms.peaking[node];
        let (tp, tf, tw, u) = (y[b], y[b + 1], y[b + 2], y[b + 3]);
        let (flux, _) = boiling(terms, tp, tw);
        let qb = terms.area * flux;
        out[b] = (f * terms.plate_share * p - terms.h1 * (tp - tf) - qb) / terms.cp;
        out[b + 1] = (f * terms.water_share * terms.film_share * p + terms.h1 * (tp - tf) - terms.h2 * (tf - tw)
            + (1.0 - terms.eps) * qb)
            / terms.cf
            - terms.sweep * tf;
        out[b + 2] = (f * terms.water_share * (1.0 - terms.film_share) * p + terms.h2 * (tf - tw) + u / terms.tau) / terms.cb
            - terms.sweep * tw;
        out[b + 3] = terms.eps * qb - u / terms.tau - terms.sweep * u;
        node += 1;
    }
    out
}

/// The rates of the poles at `y`, per second: the growing prompt pole, the decaying prompt pole,
/// and the fastest node's sum of heat transfer, boiling slope, condensation and sweep.
fn pole_rates_per_s(terms: &Terms, y: &[f64; NV]) -> [f64; 3] {
    let rho_net = terms.rho_0 - feedback(terms, y);
    let excess = (rho_net - 1.0) / terms.l_star;
    let fixed = terms.h1 / terms.cp + (terms.h1 + terms.h2) / terms.cf + terms.h2 / terms.cb + 1.0 / terms.tau + 2.0 * terms.sweep;
    let mut thermal = fixed;
    let mut node = 0;
    while node < 3 {
        let b = NODE0 + 4 * node;
        let (_, slope) = boiling(terms, y[b], y[b + 2]);
        let rate = fixed + terms.area * slope / terms.cp;
        if rate > thermal {
            thermal = rate;
        }
        node += 1;
    }
    if excess > 0.0 { [excess, 0.0, thermal] } else { [0.0, -excess, thermal] }
}

/// The state ceiling: for each pole, `dt_s` times its rate at `y` at most its phase.
fn within_state_ceiling(terms: &Terms, y: &[f64; NV], dt_s: f64) -> Result<(), StepReject> {
    let rates = pole_rates_per_s(terms, y);
    if dt_s * rates[0] > CEILING[1] || dt_s * rates[1] > CEILING[2] || dt_s * rates[2] > CEILING[3] {
        Err(StepReject::DtAboveStateCeiling)
    } else {
        Ok(())
    }
}

/// The rates of the three poles the state ceiling holds at `state` under `load`, per second: the
/// growing prompt pole, the decaying prompt pole, and the fastest node's heat transfer, boiling
/// slope, condensation and sweep. `None` when the card or the state is not one the card holds.
#[must_use]
pub fn state_pole_rates_per_s(reactor: &ReactorSpert3CcoreTwin, state: &ReactorState, load: ReactorLoad) -> Option<[f64; 3]> {
    if !reactor.is_valid() || !state_is_finite(state) {
        return None;
    }
    let terms = terms(reactor, state.initial_temperature_k, state.system_pressure_psig, load)?;
    Some(pole_rates_per_s(&terms, &state_vector(state)))
}

/// The longest step the state ceiling of [`step`] lets start from `state` under `load`, s.
/// `None` when the card or the state is not one the card holds.
#[must_use]
pub fn state_dt_ceiling_s(reactor: &ReactorSpert3CcoreTwin, state: &ReactorState, load: ReactorLoad) -> Option<f64> {
    if !reactor.is_valid() || !state_is_finite(state) {
        return None;
    }
    let terms = terms(reactor, state.initial_temperature_k, state.system_pressure_psig, load)?;
    let rates = pole_rates_per_s(&terms, &state_vector(state));
    let phases = [CEILING[1], CEILING[2], CEILING[3]];
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
    if ceiling > 0.0 && ceiling.is_finite() { Some(ceiling) } else { None }
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
    /// Energy released since the step, `E`.
    pub energy_mws: f64,
    /// Core average: plate, film and bulk water rises, and the latent heat of its vapour.
    pub plate_rise_k: f64,
    pub film_rise_k: f64,
    pub water_rise_k: f64,
    pub void_latent_mws: f64,
    /// Feedback node: the same four.
    pub feedback_plate_rise_k: f64,
    pub feedback_film_rise_k: f64,
    pub feedback_water_rise_k: f64,
    pub feedback_void_latent_mws: f64,
    /// Hot spot: the same four. `hot_spot_plate_rise_k` is the quantity the thermocouples measured.
    pub hot_spot_plate_rise_k: f64,
    pub hot_spot_film_rise_k: f64,
    pub hot_spot_water_rise_k: f64,
    pub hot_spot_void_latent_mws: f64,
    /// Initial temperature of plates and water, `θ_i`.
    pub initial_temperature_k: f64,
    /// System pressure, psig.
    pub system_pressure_psig: f64,
    /// Saturation temperature of water at the system pressure, K.
    pub saturation_temperature_k: f64,
    /// The hot-spot plate temperature, K: `θ_i + T_h`.
    pub hot_spot_plate_temperature_k: f64,
    /// `dP/dt` at this state under the run's load.
    pub power_rate_mw_per_s: f64,
    /// `dT_h/dt` at this state under the run's load.
    pub hot_spot_plate_rate_k_per_s: f64,
    /// Net reactivity at this state under the run's load. Zero at the start.
    pub reactivity_net_dollar: f64,
    /// The reactivity the water takes back (film and bulk), the plates take back (displacement,
    /// Doppler and bowing) and the vapour takes back, dollars.
    pub moderator_feedback_dollar: f64,
    pub plate_feedback_dollar: f64,
    pub void_feedback_dollar: f64,
    /// Void fraction in the feedback node's channels.
    pub void_fraction: f64,
    /// `N = P + Σ C_i` at the start.
    pub population_anchor_mw: f64,
    /// `∫ (ρ_0 − ρ) P dt` since the start, by the Hermite rule.
    pub feedback_integral_dollar_mws: f64,
    /// `∫ s (C_f T_f + C_b T_w + U) dt` of each node, by the Hermite rule.
    pub swept_energy_mws: f64,
    pub feedback_swept_energy_mws: f64,
    pub hot_spot_swept_energy_mws: f64,
    /// Signed neutron balance; positive is created population.
    pub population_defect_mw: f64,
    /// Signed energy balances of each node; positive is created energy.
    pub energy_defect_mws: f64,
    pub feedback_energy_defect_mws: f64,
    pub hot_spot_energy_defect_mws: f64,
    /// Largest of the four balances, each relative to its own scale.
    pub residual_conservation_error: f64,
    /// Highest landed power before the first maximum of power was passed.
    pub peak_power_mw: f64,
    /// Energy released at that sample.
    pub peak_energy_mws: f64,
    /// Time of that sample.
    pub peak_time_s: f64,
    /// Hot-spot plate rise at that sample: the plate temperature at the time of peak power.
    pub peak_plate_rise_k: f64,
    /// Highest landed hot-spot plate rise.
    pub plate_rise_max_k: f64,
    /// Time of that sample.
    pub plate_rise_max_time_s: f64,
    /// Reactivity of the first landed step's load, `ρ_0`. Zero at the start.
    pub run_reactivity_dollar: f64,
    /// Coolant flow of the last landed step's load; the derived fields are taken at it. Zero at the start.
    pub run_coolant_flow_m_per_s: f64,
    /// Set by the first landed step on which the power is no longer rising.
    pub past_power_peak: bool,
    /// Set by the first landed step after the power peak on which the hot-spot plate temperature is no longer rising. The next step is refused.
    pub past_plate_maximum: bool,
    /// Net reactivity above one dollar.
    pub prompt_supercritical: bool,
    /// The hot-spot plate is above the saturation temperature.
    pub hot_spot_boiling: bool,
    /// The start or the last landed load is outside the compared span.
    pub outside_compared_range: bool,
}

impl ReactorState {
    /// The reactor in equilibrium at [`INITIAL_POWER_MW`] with every rise zero, time 0. `None`
    /// unless the start is finite, the pressure is in the steam tables and the start is liquid water.
    #[must_use]
    pub fn try_start(reactor: &ReactorSpert3CcoreTwin, initial_temperature_k: f64, system_pressure_psig: f64) -> Option<Self> {
        if !reactor.is_valid()
            || !initial_temperature_k.is_finite()
            || !system_pressure_psig.is_finite()
            || !reactor.pressure_in_steam_tables(system_pressure_psig)
            || !reactor.initial_temperature_is_liquid_water(initial_temperature_k, system_pressure_psig)
        {
            return None;
        }
        let sat = reactor.saturation(system_pressure_psig)?;
        let l_star = reactor.constants[K_L_STAR];
        let power_mw = reactor.constants[K_P0];
        let mut precursor_mw = [0.0; 6];
        let mut index = 0;
        while index < 6 {
            precursor_mw[index] = reactor.group_fraction[index] * power_mw / (l_star * reactor.group_decay_per_s[index]);
            index += 1;
        }
        let mut state = Self {
            t_s: 0.0,
            power_mw,
            precursor_mw,
            energy_mws: 0.0,
            plate_rise_k: 0.0,
            film_rise_k: 0.0,
            water_rise_k: 0.0,
            void_latent_mws: 0.0,
            feedback_plate_rise_k: 0.0,
            feedback_film_rise_k: 0.0,
            feedback_water_rise_k: 0.0,
            feedback_void_latent_mws: 0.0,
            hot_spot_plate_rise_k: 0.0,
            hot_spot_film_rise_k: 0.0,
            hot_spot_water_rise_k: 0.0,
            hot_spot_void_latent_mws: 0.0,
            initial_temperature_k,
            system_pressure_psig,
            saturation_temperature_k: normal_or_zero(sat.temperature_k),
            hot_spot_plate_temperature_k: initial_temperature_k,
            power_rate_mw_per_s: 0.0,
            hot_spot_plate_rate_k_per_s: 0.0,
            reactivity_net_dollar: 0.0,
            moderator_feedback_dollar: 0.0,
            plate_feedback_dollar: 0.0,
            void_feedback_dollar: 0.0,
            void_fraction: 0.0,
            population_anchor_mw: 0.0,
            feedback_integral_dollar_mws: 0.0,
            swept_energy_mws: 0.0,
            feedback_swept_energy_mws: 0.0,
            hot_spot_swept_energy_mws: 0.0,
            population_defect_mw: 0.0,
            energy_defect_mws: 0.0,
            feedback_energy_defect_mws: 0.0,
            hot_spot_energy_defect_mws: 0.0,
            residual_conservation_error: 0.0,
            peak_power_mw: power_mw,
            peak_energy_mws: 0.0,
            peak_time_s: 0.0,
            peak_plate_rise_k: 0.0,
            plate_rise_max_k: 0.0,
            plate_rise_max_time_s: 0.0,
            run_reactivity_dollar: 0.0,
            run_coolant_flow_m_per_s: 0.0,
            past_power_peak: false,
            past_plate_maximum: false,
            prompt_supercritical: false,
            hot_spot_boiling: false,
            outside_compared_range: false,
        };
        state.population_anchor_mw = population_mw(&state);
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }
}

/// Advance one Runge–Kutta step and write the frame. A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    reactor: &ReactorSpert3CcoreTwin,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(reactor, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("reactor_spert3_ccore_twin", exceedances(reactor, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    reactor: &ReactorSpert3CcoreTwin,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(reactor, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

fn step_as_computed(
    reactor: &ReactorSpert3CcoreTwin,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    let terms = validate(reactor, state, load, dt_s)?;
    within_state_ceiling(&terms, &state_vector(state), dt_s)?;
    let advanced = rk4(reactor, &terms, &state_vector(state), dt_s);
    commit(reactor, state, frame, &terms, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero with its sign.
fn normal_or_zero_state(state: &mut ReactorState) {
    normal_or_zero_each(&mut state.precursor_mw);
    let mut fields = scalar_fields(state);
    normal_or_zero_each(&mut fields);
    set_scalar_fields(state, &fields);
}

fn validate(reactor: &ReactorSpert3CcoreTwin, state: &ReactorState, load: ReactorLoad, dt_s: f64) -> Result<Terms, StepReject> {
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
    if !reactor.pressure_in_steam_tables(state.system_pressure_psig) {
        return Err(StepReject::PressureOutsideSteamTables);
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
    let terms = match terms(reactor, state.initial_temperature_k, state.system_pressure_psig, load) {
        Some(terms) => terms,
        None => return Err(StepReject::PressureOutsideSteamTables),
    };
    incoming_state_is_the_cards(reactor, state, load)?;
    Ok(terms)
}

/// The incoming state held to what this card itself writes.
fn incoming_state_is_the_cards(reactor: &ReactorSpert3CcoreTwin, state: &ReactorState, load: ReactorLoad) -> Result<(), StepReject> {
    let y = state_vector(state);
    let mut index = 1;
    while index < NV {
        // Every variable but the power is a sum or a rise that the card keeps at zero or more.
        if y[index] < 0.0 {
            return Err(StepReject::NegativeState);
        }
        index += 1;
    }
    if !(state.power_mw > 0.0)
        || state.swept_energy_mws < 0.0
        || state.feedback_swept_energy_mws < 0.0
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
        return if same_bits(state, &start) { Ok(()) } else { Err(StepReject::InconsistentState) };
    }
    if load.reactivity_inserted_dollar != state.run_reactivity_dollar {
        return Err(StepReject::ReactivityChangedDuringRun);
    }
    // The derived fields were written at the flow of the last landed step; this step may carry another.
    let written = ReactorLoad {
        reactivity_inserted_dollar: load.reactivity_inserted_dollar,
        coolant_flow_m_per_s: state.run_coolant_flow_m_per_s,
    };
    let terms = match terms(reactor, state.initial_temperature_k, state.system_pressure_psig, written) {
        Some(terms) => terms,
        None => return Err(StepReject::InconsistentState),
    };
    let derived = derive(reactor, &terms, state, written);
    let held = derived_fields_of(state);
    let mut i = 0;
    while i < held.len() {
        if held[i].to_bits() != normal_or_zero(derived.values[i]).to_bits() {
            return Err(StepReject::InconsistentState);
        }
        i += 1;
    }
    if state.population_anchor_mw.to_bits() != normal_or_zero(start.population_anchor_mw).to_bits()
        || state.saturation_temperature_k.to_bits() != start.saturation_temperature_k.to_bits()
        || state.prompt_supercritical != derived.prompt_supercritical
        || state.hot_spot_boiling != derived.hot_spot_boiling
        || state.outside_compared_range != derived.outside_compared_range
    {
        return Err(StepReject::InconsistentState);
    }
    // The peak fields are samples this run has landed.
    if state.peak_power_mw < reactor.constants[K_P0]
        || (!state.past_power_peak && state.peak_power_mw < state.power_mw)
        || !(state.peak_time_s >= 0.0 && state.peak_time_s <= state.t_s)
        || !(state.peak_energy_mws >= 0.0 && state.peak_energy_mws <= state.energy_mws)
        || !(state.peak_plate_rise_k >= 0.0 && state.peak_plate_rise_k <= state.plate_rise_max_k)
        || (state.peak_time_s == state.t_s
            && (state.peak_power_mw.to_bits() != state.power_mw.to_bits()
                || state.peak_energy_mws.to_bits() != state.energy_mws.to_bits()
                || state.peak_plate_rise_k.to_bits() != state.hot_spot_plate_rise_k.to_bits()))
        || state.plate_rise_max_k < state.hot_spot_plate_rise_k
        || !(state.plate_rise_max_time_s >= 0.0 && state.plate_rise_max_time_s <= state.t_s)
        || (state.plate_rise_max_time_s == state.t_s && state.plate_rise_max_k.to_bits() != state.hot_spot_plate_rise_k.to_bits())
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

/// The derived fields in the order [`derived_fields_of`] lists them.
const N_DERIVED: usize = 15;

struct Derived {
    values: [f64; N_DERIVED],
    prompt_supercritical: bool,
    hot_spot_boiling: bool,
    outside_compared_range: bool,
}

fn derived_fields_of(s: &ReactorState) -> [f64; N_DERIVED] {
    [
        s.hot_spot_plate_temperature_k,
        s.power_rate_mw_per_s,
        s.hot_spot_plate_rate_k_per_s,
        s.reactivity_net_dollar,
        s.moderator_feedback_dollar,
        s.plate_feedback_dollar,
        s.void_feedback_dollar,
        s.void_fraction,
        s.population_defect_mw,
        s.energy_defect_mws,
        s.feedback_energy_defect_mws,
        s.hot_spot_energy_defect_mws,
        s.residual_conservation_error,
        s.run_reactivity_dollar,
        s.run_coolant_flow_m_per_s,
    ]
}

/// The derived fields computed again from the rest of the state by the arithmetic [`commit`]
/// wrote them with, operation for operation.
fn derive(reactor: &ReactorSpert3CcoreTwin, terms: &Terms, state: &ReactorState, load: ReactorLoad) -> Derived {
    let y = state_vector(state);
    let k = rates(reactor, terms, &y);
    let parts = feedback_parts(terms, &y);
    let net = terms.rho_0 - (parts[0] + parts[1] + parts[2]);
    let (population_defect, defects, residual) = balances(terms, state);
    let superheat = terms.initial_k + y[HOT] - terms.saturation_k;
    Derived {
        values: [
            state.initial_temperature_k + y[HOT],
            k[0],
            k[HOT],
            net,
            parts[0],
            parts[1],
            parts[2],
            terms.void_per_mws * y[FEEDBACK + 3],
            population_defect,
            defects[0],
            defects[1],
            defects[2],
            residual,
            load.reactivity_inserted_dollar,
            load.coolant_flow_m_per_s,
        ],
        prompt_supercritical: net > 1.0,
        hot_spot_boiling: superheat > 0.0,
        outside_compared_range: !reactor.compared(state.initial_temperature_k, state.system_pressure_psig, load),
    }
}

/// The four balances and the residual.
fn balances(terms: &Terms, state: &ReactorState) -> (f64, [f64; 3], f64) {
    let population = population_mw(state);
    let population_defect = population
        - state.population_anchor_mw
        - (terms.rho_0 * state.energy_mws - state.feedback_integral_dollar_mws) / terms.l_star;
    let population_scale = if population > state.population_anchor_mw { population } else { state.population_anchor_mw };
    let y = state_vector(state);
    let swept = [state.swept_energy_mws, state.feedback_swept_energy_mws, state.hot_spot_swept_energy_mws];
    let mut defects = [0.0; 3];
    let mut residual = population_defect.abs() / population_scale;
    let mut node = 0;
    while node < 3 {
        let b = NODE0 + 4 * node;
        let deposited = terms.peaking[node] * state.energy_mws;
        let held = terms.cp * y[b] + terms.cf * y[b + 1] + terms.cb * y[b + 2] + y[b + 3] + swept[node];
        defects[node] = deposited - held;
        if state.energy_mws > 0.0 {
            let e = defects[node].abs() / deposited;
            if e > residual {
                residual = e;
            }
        }
        node += 1;
    }
    (population_defect, defects, residual)
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
        && a.hot_spot_boiling == b.hot_spot_boiling
        && a.outside_compared_range == b.outside_compared_range
}

const N_FIELDS: usize = 44;

fn scalar_fields(s: &ReactorState) -> [f64; N_FIELDS] {
    [
        s.t_s,
        s.power_mw,
        s.energy_mws,
        s.plate_rise_k,
        s.film_rise_k,
        s.water_rise_k,
        s.void_latent_mws,
        s.feedback_plate_rise_k,
        s.feedback_film_rise_k,
        s.feedback_water_rise_k,
        s.feedback_void_latent_mws,
        s.hot_spot_plate_rise_k,
        s.hot_spot_film_rise_k,
        s.hot_spot_water_rise_k,
        s.hot_spot_void_latent_mws,
        s.initial_temperature_k,
        s.system_pressure_psig,
        s.saturation_temperature_k,
        s.hot_spot_plate_temperature_k,
        s.power_rate_mw_per_s,
        s.hot_spot_plate_rate_k_per_s,
        s.reactivity_net_dollar,
        s.moderator_feedback_dollar,
        s.plate_feedback_dollar,
        s.void_feedback_dollar,
        s.void_fraction,
        s.population_anchor_mw,
        s.feedback_integral_dollar_mws,
        s.swept_energy_mws,
        s.feedback_swept_energy_mws,
        s.hot_spot_swept_energy_mws,
        s.population_defect_mw,
        s.energy_defect_mws,
        s.feedback_energy_defect_mws,
        s.hot_spot_energy_defect_mws,
        s.residual_conservation_error,
        s.peak_power_mw,
        s.peak_energy_mws,
        s.peak_time_s,
        s.peak_plate_rise_k,
        s.plate_rise_max_k,
        s.plate_rise_max_time_s,
        s.run_reactivity_dollar,
        s.run_coolant_flow_m_per_s,
    ]
}

/// The inverse of [`scalar_fields`].
fn set_scalar_fields(s: &mut ReactorState, f: &[f64; N_FIELDS]) {
    s.t_s = f[0];
    s.power_mw = f[1];
    s.energy_mws = f[2];
    s.plate_rise_k = f[3];
    s.film_rise_k = f[4];
    s.water_rise_k = f[5];
    s.void_latent_mws = f[6];
    s.feedback_plate_rise_k = f[7];
    s.feedback_film_rise_k = f[8];
    s.feedback_water_rise_k = f[9];
    s.feedback_void_latent_mws = f[10];
    s.hot_spot_plate_rise_k = f[11];
    s.hot_spot_film_rise_k = f[12];
    s.hot_spot_water_rise_k = f[13];
    s.hot_spot_void_latent_mws = f[14];
    s.initial_temperature_k = f[15];
    s.system_pressure_psig = f[16];
    s.saturation_temperature_k = f[17];
    s.hot_spot_plate_temperature_k = f[18];
    s.power_rate_mw_per_s = f[19];
    s.hot_spot_plate_rate_k_per_s = f[20];
    s.reactivity_net_dollar = f[21];
    s.moderator_feedback_dollar = f[22];
    s.plate_feedback_dollar = f[23];
    s.void_feedback_dollar = f[24];
    s.void_fraction = f[25];
    s.population_anchor_mw = f[26];
    s.feedback_integral_dollar_mws = f[27];
    s.swept_energy_mws = f[28];
    s.feedback_swept_energy_mws = f[29];
    s.hot_spot_swept_energy_mws = f[30];
    s.population_defect_mw = f[31];
    s.energy_defect_mws = f[32];
    s.feedback_energy_defect_mws = f[33];
    s.hot_spot_energy_defect_mws = f[34];
    s.residual_conservation_error = f[35];
    s.peak_power_mw = f[36];
    s.peak_energy_mws = f[37];
    s.peak_time_s = f[38];
    s.peak_plate_rise_k = f[39];
    s.plate_rise_max_k = f[40];
    s.plate_rise_max_time_s = f[41];
    s.run_reactivity_dollar = f[42];
    s.run_coolant_flow_m_per_s = f[43];
}

/// `h (f0 + f1) / 2 + h² (f0' − f1') / 12`.
fn hermite_rule(h: f64, f0: f64, f1: f64, d0: f64, d1: f64) -> f64 {
    0.5 * h * (f0 + f1) + h * h * (d0 - d1) / 12.0
}

/// `(ρ_0 − ρ) P` and its time derivative at `y` with rates `k`.
fn feedback_integrand(terms: &Terms, y: &[f64; NV], k: &[f64; NV]) -> (f64, f64) {
    let taken = feedback(terms, y);
    (taken * y[0], feedback_rate(terms, y, k) * y[0] + taken * k[0])
}

/// The swept power of node `b` and its rate: `s (C_f T_f + C_b T_w + U)`.
fn swept_integrand(terms: &Terms, b: usize, y: &[f64; NV], k: &[f64; NV]) -> (f64, f64) {
    let s = terms.sweep;
    (
        s * (terms.cf * y[b + 1] + terms.cb * y[b + 2] + y[b + 3]),
        s * (terms.cf * k[b + 1] + terms.cb * k[b + 2] + k[b + 3]),
    )
}

#[allow(clippy::too_many_arguments)]
fn commit(
    reactor: &ReactorSpert3CcoreTwin,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    terms: &Terms,
    load: ReactorLoad,
    dt_s: f64,
    mut advanced: [f64; NV],
) -> Result<(), StepReject> {
    let saved = *state;
    let mut index = 0;
    while index < NV {
        if !advanced[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    // A value below the normal range is carried as zero (`crate::normal`): the derived fields
    // below are computed from the values the state carries.
    normal_or_zero_each(&mut advanced);
    within_state_ceiling(terms, &advanced, dt_s)?;
    let mut node = 0;
    while node < 3 {
        if terms.void_per_mws * advanced[NODE0 + 4 * node + 3] >= 1.0 {
            return Err(StepReject::VoidFractionAboveOne);
        }
        node += 1;
    }
    let y_old = state_vector(state);
    let k_old = rates(reactor, terms, &y_old);
    let k_new = rates(reactor, terms, &advanced);
    let (f0, d0) = feedback_integrand(terms, &y_old, &k_old);
    let (f1, d1) = feedback_integrand(terms, &advanced, &k_new);
    state.feedback_integral_dollar_mws = normal_or_zero(state.feedback_integral_dollar_mws + hermite_rule(dt_s, f0, f1, d0, d1));
    let mut swept = [state.swept_energy_mws, state.feedback_swept_energy_mws, state.hot_spot_swept_energy_mws];
    node = 0;
    while node < 3 {
        let b = NODE0 + 4 * node;
        let (s0, e0) = swept_integrand(terms, b, &y_old, &k_old);
        let (s1, e1) = swept_integrand(terms, b, &advanced, &k_new);
        swept[node] = normal_or_zero(swept[node] + hermite_rule(dt_s, s0, s1, e0, e1));
        node += 1;
    }
    state.swept_energy_mws = swept[0];
    state.feedback_swept_energy_mws = swept[1];
    state.hot_spot_swept_energy_mws = swept[2];
    state.t_s += dt_s;
    set_vector(state, &advanced);
    state.run_reactivity_dollar = load.reactivity_inserted_dollar;
    state.run_coolant_flow_m_per_s = load.coolant_flow_m_per_s;
    let derived = derive(reactor, terms, state, load);
    let v = derived.values;
    state.hot_spot_plate_temperature_k = normal_or_zero(v[0]);
    state.power_rate_mw_per_s = normal_or_zero(v[1]);
    state.hot_spot_plate_rate_k_per_s = normal_or_zero(v[2]);
    state.reactivity_net_dollar = normal_or_zero(v[3]);
    state.moderator_feedback_dollar = normal_or_zero(v[4]);
    state.plate_feedback_dollar = normal_or_zero(v[5]);
    state.void_feedback_dollar = normal_or_zero(v[6]);
    state.void_fraction = normal_or_zero(v[7]);
    state.population_defect_mw = v[8];
    state.energy_defect_mws = v[9];
    state.feedback_energy_defect_mws = v[10];
    state.hot_spot_energy_defect_mws = v[11];
    state.residual_conservation_error = v[12];
    state.prompt_supercritical = derived.prompt_supercritical;
    state.hot_spot_boiling = derived.hot_spot_boiling;
    state.outside_compared_range = derived.outside_compared_range;
    if !state.past_power_peak && state.power_mw > state.peak_power_mw {
        state.peak_power_mw = state.power_mw;
        state.peak_energy_mws = state.energy_mws;
        state.peak_time_s = state.t_s;
        state.peak_plate_rise_k = state.hot_spot_plate_rise_k;
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
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > reactor.constants[K_RESIDUAL] {
        let created = largest_defect_is_created(terms, state);
        *state = saved;
        return Err(if created { StepReject::SpontaneousGeneration } else { StepReject::FirstLawBreach });
    }
    publish(state, frame);
    Ok(())
}

/// The sign of the balance that sets the residual: positive is created.
fn largest_defect_is_created(terms: &Terms, state: &ReactorState) -> bool {
    let population = population_mw(state);
    let scale = if population > state.population_anchor_mw { population } else { state.population_anchor_mw };
    let mut largest = state.population_defect_mw.abs() / scale;
    let mut created = state.population_defect_mw > 0.0;
    if state.energy_mws > 0.0 {
        let defects = [state.energy_defect_mws, state.feedback_energy_defect_mws, state.hot_spot_energy_defect_mws];
        let mut node = 0;
        while node < 3 {
            let e = defects[node].abs() / (terms.peaking[node] * state.energy_mws);
            if e > largest {
                largest = e;
                created = defects[node] > 0.0;
            }
            node += 1;
        }
    }
    created
}

/// The twenty variables in the order of the Runge–Kutta step.
fn state_vector(s: &ReactorState) -> [f64; NV] {
    [
        s.power_mw,
        s.precursor_mw[0],
        s.precursor_mw[1],
        s.precursor_mw[2],
        s.precursor_mw[3],
        s.precursor_mw[4],
        s.precursor_mw[5],
        s.energy_mws,
        s.plate_rise_k,
        s.film_rise_k,
        s.water_rise_k,
        s.void_latent_mws,
        s.feedback_plate_rise_k,
        s.feedback_film_rise_k,
        s.feedback_water_rise_k,
        s.feedback_void_latent_mws,
        s.hot_spot_plate_rise_k,
        s.hot_spot_film_rise_k,
        s.hot_spot_water_rise_k,
        s.hot_spot_void_latent_mws,
    ]
}

fn set_vector(s: &mut ReactorState, y: &[f64; NV]) {
    s.power_mw = y[0];
    let mut i = 0;
    while i < 6 {
        s.precursor_mw[i] = y[1 + i];
        i += 1;
    }
    s.energy_mws = y[7];
    s.plate_rise_k = y[8];
    s.film_rise_k = y[9];
    s.water_rise_k = y[10];
    s.void_latent_mws = y[11];
    s.feedback_plate_rise_k = y[12];
    s.feedback_film_rise_k = y[13];
    s.feedback_water_rise_k = y[14];
    s.feedback_void_latent_mws = y[15];
    s.hot_spot_plate_rise_k = y[16];
    s.hot_spot_film_rise_k = y[17];
    s.hot_spot_water_rise_k = y[18];
    s.hot_spot_void_latent_mws = y[19];
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

fn advance(y: &[f64; NV], k: &[f64; NV], scale: f64) -> [f64; NV] {
    let mut out = [0.0; NV];
    let mut index = 0;
    while index < NV {
        out[index] = y[index] + scale * k[index];
        index += 1;
    }
    out
}

/// Classical fourth-order Runge–Kutta.
fn rk4(reactor: &ReactorSpert3CcoreTwin, terms: &Terms, y: &[f64; NV], dt_s: f64) -> [f64; NV] {
    let half = 0.5 * dt_s;
    let k1 = rates(reactor, terms, y);
    let k2 = rates(reactor, terms, &advance(y, &k1, half));
    let k3 = rates(reactor, terms, &advance(y, &k2, half));
    let k4 = rates(reactor, terms, &advance(y, &k3, dt_s));
    let mut out = [0.0; NV];
    let mut index = 0;
    while index < NV {
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
    frame.vel[0] = state.hot_spot_plate_temperature_k as f32;
    frame.vel[1] = state.void_feedback_dollar as f32;
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
    if state.hot_spot_boiling {
        flags |= FLAG_HOT_SPOT_BOILING;
    }
    if state.outside_compared_range {
        flags |= FLAG_OUTSIDE_COMPARED_RANGE;
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

/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    PROMPT_KIND,
    PEAK_KIND,
    BOILING_KIND,
    INITIAL_LOW,
    INITIAL_HIGH,
    PRESSURE_LOW,
    PRESSURE_HIGH,
    REACTIVITY_NO_FLOW_LOW,
    REACTIVITY_HIGH,
    FLOW_PRESSURE,
    FLOW_LOW,
    FLOW_HIGH,
    REACTIVITY_FLOW_LOW,
];

const PROMPT_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_PROMPT_SUPERCRITICAL",
    FLAG_PROMPT_SUPERCRITICAL,
    "reactivity_net_dollar",
    "dollar",
    Side::Above,
    1.0,
    "one dollar of net reactivity: prompt critical",
);
const PEAK_KIND: ExceedanceKind = ExceedanceKind::varying(
    "FLAG_PAST_POWER_PEAK",
    FLAG_PAST_POWER_PEAK,
    "power_mw",
    "MW",
    Side::AtOrBelow,
    "the largest power so far (peak_power_mw), passed once the power stopped rising: the extent is the fall from it",
);
const BOILING_KIND: ExceedanceKind = ExceedanceKind::varying(
    "FLAG_HOT_SPOT_BOILING",
    FLAG_HOT_SPOT_BOILING,
    "hot_spot_plate_temperature_k",
    "K",
    Side::Above,
    "the saturation temperature of water at the system pressure (IAPWS-IF97 region 4)",
);
const fn outside(quantity: &'static str, unit: &'static str, side: Side, limit: f64, words: &'static str) -> ExceedanceKind {
    ExceedanceKind::fixed("FLAG_OUTSIDE_COMPARED_RANGE", FLAG_OUTSIDE_COMPARED_RANGE, quantity, unit, side, limit, words)
}
const INITIAL_WORDS: &str = "the initial temperatures of the fitted transients, 21 to 36 degC";
const INITIAL_LOW: ExceedanceKind = outside("initial_temperature_k", "K", Side::Below, INITIAL_TEMPERATURE_MIN_K, INITIAL_WORDS);
const INITIAL_HIGH: ExceedanceKind = outside("initial_temperature_k", "K", Side::Above, INITIAL_TEMPERATURE_MAX_K, INITIAL_WORDS);
const PRESSURE_WORDS: &str = "the system pressures of the fitted transients, 0 to 2500 psig";
const PRESSURE_LOW: ExceedanceKind = outside("system_pressure_psig", "psig", Side::Below, PRESSURE_MIN_PSIG, PRESSURE_WORDS);
const PRESSURE_HIGH: ExceedanceKind = outside("system_pressure_psig", "psig", Side::Above, PRESSURE_MAX_PSIG, PRESSURE_WORDS);
const REACTIVITY_NO_FLOW_LOW: ExceedanceKind = outside(
    "reactivity_inserted_dollar",
    "dollar",
    Side::Below,
    REACTIVITY_MIN_NO_FLOW_DOLLAR,
    "the smallest inserted reactivity of the fitted transients with no flow",
);
const REACTIVITY_HIGH: ExceedanceKind = outside(
    "reactivity_inserted_dollar",
    "dollar",
    Side::Above,
    REACTIVITY_MAX_DOLLAR,
    "the largest inserted reactivity of the fitted transients",
);
const FLOW_PRESSURE: ExceedanceKind = outside(
    "system_pressure_psig",
    "psig",
    Side::OtherThan,
    FLOW_PRESSURE_PSIG,
    "the system pressure of every fitted transient with flow, 230 psig",
);
const FLOW_WORDS: &str = "the coolant flows of the fitted transients with flow, 0.18 to 18 ft/s";
const FLOW_LOW: ExceedanceKind = outside("coolant_flow_m_per_s", "m/s", Side::Below, FLOW_MIN_M_PER_S, FLOW_WORDS);
const FLOW_HIGH: ExceedanceKind = outside("coolant_flow_m_per_s", "m/s", Side::Above, FLOW_MAX_M_PER_S, FLOW_WORDS);
const REACTIVITY_FLOW_LOW: ExceedanceKind = outside(
    "reactivity_inserted_dollar",
    "dollar",
    Side::Below,
    REACTIVITY_MIN_FLOW_DOLLAR,
    "the smallest inserted reactivity of the fitted transients with flow",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
/// The conditions are those `derive` and the step write the four flags'
/// booleans by; the compared range is [`ReactorSpert3CcoreTwin::compared`].
pub fn exceedances(
    reactor: &ReactorSpert3CcoreTwin,
    before: &ReactorState,
    after: &ReactorState,
    load: ReactorLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (before, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    let s = after;
    records.push_if(s.prompt_supercritical, Exceedance::fixed(PROMPT_KIND, s.reactivity_net_dollar));
    records.push_if(s.past_power_peak, Exceedance::at(PEAK_KIND, s.peak_power_mw, s.power_mw));
    records.push_if(
        s.hot_spot_boiling,
        Exceedance::at(BOILING_KIND, reactor.saturation_temperature_k(s.system_pressure_psig), s.hot_spot_plate_temperature_k),
    );
    if s.outside_compared_range {
        let initial = s.initial_temperature_k;
        let pressure = s.system_pressure_psig;
        records.push_if(!(initial >= INITIAL_TEMPERATURE_MIN_K), Exceedance::fixed(INITIAL_LOW, initial));
        records.push_if(!(initial <= INITIAL_TEMPERATURE_MAX_K), Exceedance::fixed(INITIAL_HIGH, initial));
        records.push_if(!(pressure >= PRESSURE_MIN_PSIG), Exceedance::fixed(PRESSURE_LOW, pressure));
        records.push_if(!(pressure <= PRESSURE_MAX_PSIG), Exceedance::fixed(PRESSURE_HIGH, pressure));
        let v = load.coolant_flow_m_per_s;
        let rho = load.reactivity_inserted_dollar;
        if v == 0.0 {
            records.push_if(!(rho >= REACTIVITY_MIN_NO_FLOW_DOLLAR), Exceedance::fixed(REACTIVITY_NO_FLOW_LOW, rho));
        } else {
            records.push_if(pressure != FLOW_PRESSURE_PSIG, Exceedance::fixed(FLOW_PRESSURE, pressure));
            records.push_if(!(v >= FLOW_MIN_M_PER_S), Exceedance::fixed(FLOW_LOW, v));
            records.push_if(!(v <= FLOW_MAX_M_PER_S), Exceedance::fixed(FLOW_HIGH, v));
            records.push_if(!(rho >= REACTIVITY_MIN_FLOW_DOLLAR), Exceedance::fixed(REACTIVITY_FLOW_LOW, rho));
        }
        records.push_if(!(rho <= REACTIVITY_MAX_DOLLAR), Exceedance::fixed(REACTIVITY_HIGH, rho));
    }
    records
}
