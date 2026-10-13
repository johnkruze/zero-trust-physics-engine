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
//! `ρ_0` is a step at time zero; `v` is the coolant velocity through the
//! core. Both are the load, recorded by the first landed step and held: a
//! later step with another reactivity or another flow is refused by name.
//! `θ_i` and the system pressure are the start. T are rises above `θ_i`.
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
//! The card leaves out boiling, void, any effect of pressure on the
//! transient (pressure is a recorded start field held to the compared span),
//! heat conduction within the plate, the thermocouple's response, anything
//! after the first maximum of the hot-spot plate temperature, and any other
//! core.
//!
//! Declared range, the span the fit and held-back rows cover: initial
//! temperature 21 to 36 degC; pressure 0 to 2500 psig with no flow, and flow
//! 200 to 20,000 gpm (0.054864 to 5.4864 m/s) only at 230 psig; the
//! reactivity of the inhour equation (with this card's `l*` and groups) for
//! the longest and the shortest compared period: 9.3 s to 9.7 ms with no
//! flow, 516 ms to 9.7 ms with flow. Outside it the card refuses by name.
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
//! neither. `core` only, with `exp` and `log` from [`crate::math`] and `sqrt`
//! from the platform.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

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
/// Initial temperature span, K: 21 to 36 degC, the compared rows.
pub const INITIAL_TEMPERATURE_MIN_K: f64 = 21.0 + 273.15;
pub const INITIAL_TEMPERATURE_MAX_K: f64 = 36.0 + 273.15;
/// System pressure span with no flow, psig: 0 to 2500.
pub const PRESSURE_MAX_PSIG: f64 = 2500.0;
/// System pressure with flow, psig: 230, the one pressure at which every compared flow
/// transient was run. The protocol's "225 to 250" counted two flow rows (03040 at 225 and 03042
/// at 250 psig) that are on no line and were never run; the card holds to 230.
pub const FLOW_PRESSURE_MIN_PSIG: f64 = 230.0;
pub const FLOW_PRESSURE_MAX_PSIG: f64 = 230.0;
/// Coolant velocity span with flow, m/s: 200 and 20,000 gpm at 18 ft/s per 20,000 gpm (IDO-16918
/// p. 1), each as `gpm × (18 / 20000) × 0.3048` evaluates; the upper one is the double above 5.4864.
pub const FLOW_MIN_M_PER_S: f64 = 0.054864;
pub const FLOW_MAX_M_PER_S: f64 = 5.486400000000001;
/// Smallest reactivity with no flow, dollars: the inhour value of 9.3 s.
pub const REACTIVITY_MIN_NO_FLOW_DOLLAR: f64 = 0.4122874385206243;
/// Smallest reactivity with flow, dollars: the inhour value of 516 ms.
pub const REACTIVITY_MIN_FLOW_DOLLAR: f64 = 0.870348183353308;
/// Largest reactivity, dollars: the inhour value of 9.7 ms.
pub const REACTIVITY_MAX_DOLLAR: f64 = 1.3053996316244825;
/// Power beyond which the card refuses, MW. The largest the card reaches inside its range is about 1050.
pub const POWER_MAX_MW: f64 = 2000.0;
/// Energy released beyond which the card refuses, MW s. A run at the slowest compared flow insertion
/// with the most flow reaches about 53,000 by the time limit.
pub const ENERGY_MAX_MWS: f64 = 1.0e5;
/// Time beyond which the card refuses, s.
pub const TIME_MAX_S: f64 = 200.0;
/// The ledger's residual must stay at or under this: 11 times the largest the card's own step
/// left at the ceiling over the 67 fit rows run and eight runs at the corners of the range
/// (2.24e-8, the shortest period at the most flow). It names a wrong method; it is not a bound on
/// the error of the state.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.5e-7;
/// Fraction of `l*` one step at the ceiling advances.
pub const GATE_PHASE_STEP: f64 = 0.2;
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
    /// The initial temperature is outside the compared 21 to 36 degC.
    InitialTemperatureOutsideCard,
    /// The system pressure is outside the compared 0 to 2500 psig.
    PressureOutsideCard,
    /// The coolant flow is outside the compared: none, or 200 to 20,000 gpm (0.054864 to 5.4864 m/s) at 230 psig only.
    FlowOutsideCard,
    /// The reactivity is outside the compared: the inhour values of 9.3 s to 9.7 ms with no flow, 516 ms to 9.7 ms with flow.
    ReactivityOutsideCard,
    /// The state is past the first maximum of the hot-spot plate temperature after the power peak; nothing after it is on the card.
    PastPlateTemperatureMaximum,
    /// The step would end after 200 s.
    TimeBeyondCard,
    /// The incoming state holds an energy, a temperature rise or a precursor sum below zero, or a power not above zero.
    NegativeState,
    /// The incoming state or the trial step holds more than 1e5 MW s released.
    EnergyBeyondCard,
    /// The incoming state or the trial step holds a power, or a peak power, above 2000 MW.
    PowerBeyondCard,
    /// The reactivity of the load is not the one of the first step of this run; a run is one step of reactivity.
    ReactivityChangedDuringRun,
    /// The coolant flow of the load is not the one of the first step of this run; no compared test changed its flow.
    FlowChangedDuringRun,
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
    pub constants: [f64; 36],
}

/// The scalar constants in one list, in the order `is_valid` and `terms` read them.
const SCALARS: [f64; 36] = [
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
    INITIAL_TEMPERATURE_MIN_K,
    INITIAL_TEMPERATURE_MAX_K,
    PRESSURE_MAX_PSIG,
    FLOW_PRESSURE_MIN_PSIG,
    FLOW_PRESSURE_MAX_PSIG,
    FLOW_MIN_M_PER_S,
    FLOW_MAX_M_PER_S,
    REACTIVITY_MIN_NO_FLOW_DOLLAR,
    REACTIVITY_MIN_FLOW_DOLLAR,
    REACTIVITY_MAX_DOLLAR,
    POWER_MAX_MW,
    ENERGY_MAX_MWS,
    TIME_MAX_S,
    RESIDUAL_CONSERVATION_LIMIT,
    GATE_PHASE_STEP,
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
const K_TMIN: usize = 20;
const K_TMAX: usize = 21;
const K_PMAX: usize = 22;
const K_FPMIN: usize = 23;
const K_FPMAX: usize = 24;
const K_VMIN: usize = 25;
const K_VMAX: usize = 26;
const K_RMIN0: usize = 27;
const K_RMINF: usize = 28;
const K_RMAX: usize = 29;
const K_POWER_MAX: usize = 30;
const K_ENERGY_MAX: usize = 31;
const K_TIME_MAX: usize = 32;
const K_RESIDUAL: usize = 33;
const K_GATE: usize = 34;
const K_FLOOR: usize = 35;

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

    /// Largest step the card stands behind: `GATE_PHASE_STEP · l*`, 0.6 ms. Not applied by [`step`].
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

    /// Whether an initial temperature is in the compared span.
    #[must_use]
    pub fn initial_temperature_on_card(&self, initial_temperature_k: f64) -> bool {
        initial_temperature_k >= self.constants[K_TMIN] && initial_temperature_k <= self.constants[K_TMAX]
    }

    /// Whether a system pressure is in the compared span.
    #[must_use]
    pub fn pressure_on_card(&self, system_pressure_psig: f64) -> bool {
        system_pressure_psig >= 0.0 && system_pressure_psig <= self.constants[K_PMAX]
    }

    /// Whether a coolant flow was compared at this pressure.
    #[must_use]
    pub fn flow_on_card(&self, system_pressure_psig: f64, coolant_flow_m_per_s: f64) -> bool {
        let c = &self.constants;
        coolant_flow_m_per_s == 0.0
            || (coolant_flow_m_per_s >= c[K_VMIN]
                && coolant_flow_m_per_s <= c[K_VMAX]
                && system_pressure_psig >= c[K_FPMIN]
                && system_pressure_psig <= c[K_FPMAX])
    }

    /// Whether a reactivity was compared with this flow.
    #[must_use]
    pub fn reactivity_on_card(&self, coolant_flow_m_per_s: f64, reactivity_inserted_dollar: f64) -> bool {
        let c = &self.constants;
        let least = if coolant_flow_m_per_s == 0.0 { c[K_RMIN0] } else { c[K_RMINF] };
        reactivity_inserted_dollar >= least && reactivity_inserted_dollar <= c[K_RMAX]
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
    /// System pressure, psig. Held to the compared span; no equation reads it.
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
    /// Coolant flow of the first landed step's load. Zero at the start.
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
    /// unless the initial temperature and the pressure are finite and on the card.
    #[must_use]
    pub fn try_start(
        reactor: &ReactorSpert3Ccore,
        initial_temperature_k: f64,
        system_pressure_psig: f64,
    ) -> Option<Self> {
        if !reactor.is_valid()
            || !initial_temperature_k.is_finite()
            || !system_pressure_psig.is_finite()
            || !reactor.initial_temperature_on_card(initial_temperature_k)
            || !reactor.pressure_on_card(system_pressure_psig)
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
#[must_use]
pub fn step(
    reactor: &ReactorSpert3Ccore,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(reactor, state, load, dt_s)?;
    let terms = terms(reactor, state.initial_temperature_k, load);
    let advanced = rk4(reactor, &terms, &state_vector(state), dt_s);
    commit(reactor, state, frame, &terms, load, dt_s, advanced)
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
    if !reactor.initial_temperature_on_card(state.initial_temperature_k) {
        return Err(StepReject::InitialTemperatureOutsideCard);
    }
    if !reactor.pressure_on_card(state.system_pressure_psig) {
        return Err(StepReject::PressureOutsideCard);
    }
    if !reactor.flow_on_card(state.system_pressure_psig, load.coolant_flow_m_per_s) {
        return Err(StepReject::FlowOutsideCard);
    }
    if !reactor.reactivity_on_card(load.coolant_flow_m_per_s, load.reactivity_inserted_dollar) {
        return Err(StepReject::ReactivityOutsideCard);
    }
    if state.past_plate_maximum {
        return Err(StepReject::PastPlateTemperatureMaximum);
    }
    if !(state.t_s >= 0.0) || !(state.t_s + dt_s <= reactor.constants[K_TIME_MAX]) {
        return Err(StepReject::TimeBeyondCard);
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
    {
        return Err(StepReject::NegativeState);
    }
    if state.energy_mws > reactor.constants[K_ENERGY_MAX] {
        return Err(StepReject::EnergyBeyondCard);
    }
    if state.power_mw > reactor.constants[K_POWER_MAX] || state.peak_power_mw > reactor.constants[K_POWER_MAX] {
        return Err(StepReject::PowerBeyondCard);
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
    if load.coolant_flow_m_per_s != state.run_coolant_flow_m_per_s {
        return Err(StepReject::FlowChangedDuringRun);
    }
    let terms = terms(reactor, state.initial_temperature_k, load);
    let derived = derive(&terms, reactor, state);
    if state.population_anchor_mw.to_bits() != start.population_anchor_mw.to_bits()
        || state.power_rate_mw_per_s.to_bits() != derived.power_rate.to_bits()
        || state.hot_spot_plate_rate_k_per_s.to_bits() != derived.plate_rate.to_bits()
        || state.reactivity_net_dollar.to_bits() != derived.net_dollar.to_bits()
        || state.prompt_supercritical != derived.prompt_supercritical
        || state.population_defect_mw.to_bits() != derived.population_defect.to_bits()
        || state.energy_defect_mws.to_bits() != derived.energy_defect.to_bits()
        || state.hot_spot_energy_defect_mws.to_bits() != derived.hot_spot_defect.to_bits()
        || state.residual_conservation_error.to_bits() != derived.residual.to_bits()
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
    advanced: [f64; 12],
) -> Result<(), StepReject> {
    let saved = *state;
    let mut index = 0;
    while index < 12 {
        if !advanced[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    if advanced[11] > reactor.constants[K_ENERGY_MAX] {
        return Err(StepReject::EnergyBeyondCard);
    }
    if advanced[0] > reactor.constants[K_POWER_MAX] {
        return Err(StepReject::PowerBeyondCard);
    }
    let y_old = state_vector(state);
    let k_old = rates(reactor, terms, &y_old);
    let k_new = rates(reactor, terms, &advanced);
    let (f0, d0) = feedback_integrand(terms, &y_old, &k_old);
    let (f1, d1) = feedback_integrand(terms, &advanced, &k_new);
    state.feedback_integral_dollar_mws += hermite_rule(dt_s, f0, f1, d0, d1);
    let swept = terms.cw * terms.sweep;
    state.swept_energy_mws += swept * hermite_rule(dt_s, y_old[8], advanced[8], k_old[8], k_new[8]);
    state.hot_spot_swept_energy_mws += swept * hermite_rule(dt_s, y_old[10], advanced[10], k_old[10], k_new[10]);
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
    state.power_rate_mw_per_s = k_new[0];
    state.hot_spot_plate_rate_k_per_s = k_new[9];
    state.reactivity_net_dollar = terms.rho_0 - feedback(terms, &advanced);
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

    use super::{
        advance, commit, rates, rk4, state_vector, step, terms, validate, ENERGY_MAX_MWS, FLAG_PAST_POWER_PEAK,
        FLOW_MAX_M_PER_S, FLOW_MIN_M_PER_S, GROUP_FRACTION, INITIAL_POWER_MW, INITIAL_TEMPERATURE_MAX_K,
        INITIAL_TEMPERATURE_MIN_K, POWER_MAX_MW, REACTIVITY_MAX_DOLLAR, REACTIVITY_MIN_FLOW_DOLLAR,
        REACTIVITY_MIN_NO_FLOW_DOLLAR, RESIDUAL_CONSERVATION_LIMIT, ReactorLoad, ReactorSpert3Ccore, ReactorState,
        StepReject, TIME_MAX_S, Terms,
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
        end: StepReject,
    }

    fn run_with(method: Method, celsius: f64, psig: f64, load: ReactorLoad, dt_s: f64, most: u64) -> Run {
        let (reactor, mut state) = start(celsius, psig);
        let mut frame = marked_frame();
        let mut steps = 0;
        let mut largest_residual = 0.0f64;
        let end = loop {
            match step_with(method, &reactor, &mut state, &mut frame, load, dt_s) {
                Ok(()) => {
                    steps += 1;
                    largest_residual = largest_residual.max(state.residual_conservation_error);
                    assert_eq!(frame.proof, [0xA5; 16]);
                }
                Err(reject) => break reject,
            }
            if steps >= most {
                break StepReject::TimeBeyondCard;
            }
        };
        Run { last: state, steps, largest_residual, end }
    }

    #[test]
    fn constants_are_locked_and_the_range_is_the_inhour_value_of_the_compared_periods() {
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

    #[test]
    fn the_start_is_the_equilibrium_and_only_compared_starts_start() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        for (celsius, psig) in [(21.0, 0.0), (36.0, 2500.0), (25.0, 1000.0), (30.0, 230.0)] {
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
        assert!(ReactorState::try_start(&reactor, INITIAL_TEMPERATURE_MIN_K, 0.0).is_some());
        assert!(ReactorState::try_start(&reactor, INITIAL_TEMPERATURE_MAX_K, 0.0).is_some());
        assert!(ReactorState::try_start(&reactor, INITIAL_TEMPERATURE_MIN_K.next_down(), 0.0).is_none());
        assert!(ReactorState::try_start(&reactor, INITIAL_TEMPERATURE_MAX_K.next_up(), 0.0).is_none());
        assert!(ReactorState::try_start(&reactor, 300.0, -0.1).is_none());
        assert!(ReactorState::try_start(&reactor, 300.0, 2500.1).is_none());
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
        let mut warm = fresh;
        warm.initial_temperature_k = 40.0 + 273.15;
        refuse(warm, load(1.0, 0.0), dt, StepReject::InitialTemperatureOutsideCard);
        let mut pressed = fresh;
        pressed.system_pressure_psig = 2600.0;
        refuse(pressed, load(1.0, 0.0), dt, StepReject::PressureOutsideCard);
        // flow is on the card only at 230 psig, and only from 200 to 20,000 gpm
        refuse(fresh, load(1.0, gpm(5000.0)), dt, StepReject::FlowOutsideCard);
        let (_, loop230) = start(30.0, 230.0);
        refuse(loop230, load(1.0, gpm(199.0)), dt, StepReject::FlowOutsideCard);
        refuse(loop230, load(1.0, FLOW_MAX_M_PER_S.next_up()), dt, StepReject::FlowOutsideCard);
        refuse(loop230, load(1.0, -gpm(1000.0)), dt, StepReject::FlowOutsideCard);
        let (_, high) = start(30.0, 2500.0);
        refuse(high, load(1.0, gpm(20000.0)), dt, StepReject::FlowOutsideCard);
        // flow was compared at 230 psig only: 225 and 250 psig with flow are refused
        for psig in [225.0, 229.9, 230.1, 250.0] {
            let (_, other) = start(30.0, psig);
            refuse(other, load(1.0, gpm(5000.0)), dt, StepReject::FlowOutsideCard);
            refuse(other, load(1.0, FLOW_MAX_M_PER_S), dt, StepReject::FlowOutsideCard);
        }
        // the reactivity span: 9.3 s to 9.7 ms with no flow, 516 ms to 9.7 ms with flow
        refuse(fresh, load(REACTIVITY_MIN_NO_FLOW_DOLLAR.next_down(), 0.0), dt, StepReject::ReactivityOutsideCard);
        refuse(fresh, load(REACTIVITY_MAX_DOLLAR.next_up(), 0.0), dt, StepReject::ReactivityOutsideCard);
        refuse(loop230, load(0.8, gpm(1000.0)), dt, StepReject::ReactivityOutsideCard);
        let mut late = fresh;
        late.t_s = TIME_MAX_S - 0.5 * dt;
        refuse(late, load(1.0, 0.0), dt, StepReject::TimeBeyondCard);
        let mut powerful = fresh;
        powerful.power_mw = POWER_MAX_MW * 2.0;
        refuse(powerful, load(1.0, 0.0), dt, StepReject::PowerBeyondCard);
        let mut spent = fresh;
        spent.energy_mws = ENERGY_MAX_MWS * 1.01;
        refuse(spent, load(1.0, 0.0), dt, StepReject::EnergyBeyondCard);
        let mut negative = fresh;
        negative.water_rise_k = -1.0;
        refuse(negative, load(1.0, 0.0), dt, StepReject::NegativeState);
        let mut moved = fresh;
        moved.plate_rise_k = 1.0;
        refuse(moved, load(1.0, 0.0), dt, StepReject::InconsistentState);
        // every edge of the range lands
        for (state, l) in [
            (fresh, load(REACTIVITY_MIN_NO_FLOW_DOLLAR, 0.0)),
            (fresh, load(REACTIVITY_MAX_DOLLAR, 0.0)),
            (loop230, load(REACTIVITY_MIN_FLOW_DOLLAR, FLOW_MIN_M_PER_S)),
            (loop230, load(REACTIVITY_MAX_DOLLAR, FLOW_MAX_M_PER_S)),
            (high, load(1.0, 0.0)),
        ] {
            let mut s = state;
            let mut frame = marked_frame();
            assert!(step(&reactor, &mut s, &mut frame, l, ceiling()).is_ok());
        }
    }

    #[test]
    fn a_short_period_run_passes_the_power_peak_then_ends_at_the_plate_maximum_with_the_ledger_closed() {
        let rho = ReactorSpert3Ccore::c_core_step_transients().inhour_dollar(0.0107);
        let run = run_with(rk4, 25.0, 0.0, load(rho, 0.0), ceiling(), 10_000);
        assert_eq!(run.end, StepReject::PastPlateTemperatureMaximum);
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
    fn the_flag_marks_the_power_peak_and_the_load_cannot_change_during_a_run() {
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
                assert_eq!(step(&reactor, &mut s, &mut f, load(1.1, gpm(6000.0)), ceiling()), Err(StepReject::FlowChangedDuringRun));
                assert_eq!(s, state);
                assert_eq!(f.to_bytes(), frame.to_bytes());
            }
        }
        assert!(seen_flag);
    }

    #[test]
    fn a_wrong_method_or_a_falsified_heat_path_is_named_by_the_ledger() {
        let rho = ReactorSpert3Ccore::c_core_step_transients().inhour_dollar(0.0107);
        for method in [euler as Method, wrong_weights, no_water_feedback, leaky_heat_path] {
            let run = run_with(method, 25.0, 0.0, load(rho, 0.0), ceiling(), 10_000);
            assert!(
                run.end == StepReject::SpontaneousGeneration || run.end == StepReject::FirstLawBreach,
                "{:?} after {} steps",
                run.end,
                run.steps
            );
        }
        // with flow, too
        let run = run_with(euler, 30.0, 230.0, load(rho, gpm(20000.0)), ceiling(), 10_000);
        assert!(run.end == StepReject::SpontaneousGeneration || run.end == StepReject::FirstLawBreach);
        // a falsified ledger in the handed-in state is not one the card wrote
        let (reactor, mut state) = start(25.0, 0.0);
        let mut frame = marked_frame();
        for _ in 0..50 {
            step(&reactor, &mut state, &mut frame, load(rho, 0.0), ceiling()).unwrap();
        }
        for field in 0..4 {
            let mut forged = state;
            match field {
                0 => forged.feedback_integral_dollar_mws = forged.feedback_integral_dollar_mws * 1.01 + 1.0e-9,
                1 => forged.swept_energy_mws += 1.0e-9,
                2 => forged.peak_power_mw *= 0.5,
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

    /// Three compared rows (held back) run to their end at the ceiling: the card's answers as
    /// the comparison of 2026-10-11 recorded them (`analysis/out/rust_held_rows.tsv` of the
    /// research record), pinned bit for bit so that any change to the equations shows.
    #[test]
    fn three_compared_rows_give_the_answers_the_comparison_recorded() {
        let reactor = ReactorSpert3Ccore::c_core_step_transients();
        // record, degC, psig, gpm, period s, steps landed, end, peak power, peak energy, plate rise max
        let rows: [(&str, f64, f64, f64, f64, u64, StepReject, u64, u64, u64); 3] = [
            ("01676", 25.0, 0.0, 0.0, 10.1 / 1000.0, 369, StepReject::PastPlateTemperatureMaximum, PIN[0], PIN[1], PIN[2]),
            ("02985", 33.0, 230.0, 10000.0, 9.7 / 1000.0, 344, StepReject::PastPlateTemperatureMaximum, PIN[3], PIN[4], PIN[5]),
            ("01425", 25.0, 0.0, 0.0, 9300.0 / 1000.0, 333_333, StepReject::TimeBeyondCard, PIN[6], PIN[7], PIN[8]),
        ];
        for (record, celsius, psig, flow_gpm, period_s, landed, end, power, energy, plate) in rows {
            let (_, mut state) = start(celsius, psig);
            let mut frame = marked_frame();
            let l = load(reactor.inhour_dollar(period_s), gpm(flow_gpm));
            let mut steps = 0u64;
            let reject = loop {
                match step(&reactor, &mut state, &mut frame, l, ceiling()) {
                    Ok(()) => steps += 1,
                    Err(reject) => break reject,
                }
            };
            assert_eq!((steps, reject), (landed, end), "{record}");
            assert_eq!(state.peak_power_mw.to_bits(), power, "{record}");
            assert_eq!(state.peak_energy_mws.to_bits(), energy, "{record}");
            assert_eq!(state.plate_rise_max_k.to_bits(), plate, "{record}");
        }
        // 01425 ends at the time limit, where the comparison took the largest rise reached
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
