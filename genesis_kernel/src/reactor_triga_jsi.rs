#![no_implicit_prelude]

//! The TRIGA Mark II reactor of the Jožef Stefan Institute, Ljubljana, with its
//! 56-element core of standard fuel, in pulse mode: from the fire signal of the
//! transient rod through the pulse, the seconds after it and the scram, with the
//! power, the energy released and the core-average fuel temperature as state.
//!
//! Measured by the Jožef Stefan Institute: the pulse table and the per-pulse
//! records of power and fuel temperature at trigapulse.ijs.si (I. Švajger, L.
//! Snoj). The site states no licence; the article the sourced constants come
//! from is CC BY-NC-ND. No measured value of the site is stored in the card.
//! The core is the one loaded as cores 215 (November 2014), 217 (March 2015) and
//! 231 (March and April 2018): 56 elements of 12 wt% uranium, 20 % enriched,
//! stainless clad, U-ZrH1.6.
//!
//! ```text
//! inserted fraction    f(t) = min(1, R t / ρ_0)                            ρ_0 in printed dollars
//! net reactivity       ρ = (K_c ρ_0 − O_c) f(t) − (φ(T) − φ(T_0)) / β − S·[rods dropped]   dollars
//! power                dP/dt = ((ρ − 1) β / Λ) P + Σ λ_i C_i               MW per s
//! precursor sums       dC_i/dt = (β_i / Λ) P − λ_i C_i                     i = 1 to 6
//! fuel temperature     dT/dt = (P − H (T − T_0)) / (m c(T))                K per s
//! energy released      dE/dt = P                                           MW
//! feedback             φ(T) = a_0 x + a_1 x² / 2,   x = T − 294 K
//! heat capacity        c(T) = 333 + 0.678 (T − 274.15)                     J per kg K
//! ```
//!
//! `ρ_0` is the load: the inserted reactivity as the institute's table prints
//! it, in its dollars of 700 pcm. A printed dollar belongs to a rod-worth table,
//! and the tables of the three cores differ: `c`, fixed at the start, says which
//! one, 215, 217 or 231. `T_0` is the fuel temperature the state was started at,
//! which is also the water temperature. The second load value drops the rods:
//! the scram, applied from the start of the step that first carries it.
//!
//! From the sources, not fitted: `β` and the six groups (Švajger et al., Nucl.
//! Eng. Technol. 56 (2024) 1165, Tables 10 and 12); `m`, 56 elements of 2.36 kg;
//! `c(T)` (the same article, eq. 7.2); `R`, the rod's whole worth of 4.16 dollars
//! (the institute's rod table of core 253) over its whole transit of 80 ms (the
//! article, section 7.1); the starting power, 10 W. `S`, 8 dollars, is an
//! assumption and not a sourced value.
//!
//! Ten numbers are fitted, to 36 pulses of the three cores at 1.4 to 3.0 printed
//! dollars, on peak power, width and energy to the scram together:
//! [`DOLLAR_SCALE`] and [`DOLLAR_OFFSET`] for each core's table; the two numbers
//! of the feedback, [`FEEDBACK_PER_K`] and [`FEEDBACK_SLOPE_PER_K2`]; the
//! generation time `Λ`, [`GENERATION_TIME_S`], 29.59 microseconds against the
//! article's 40 with 8 %; and [`HEAT_FLOW_W_PER_K`] from fuel to water, fitted to
//! the thermocouple. Only products are seen by the pulses: with another `Λ` the
//! worth of a printed dollar changes with it, so no one of these is the reactor's
//! own value of that quantity. The protocol is `validation_protocol_v2.md` of
//! `research/reactor_triga_jsi`, with its two amendments.
//!
//! There is no thermocouple in the card. The fuel temperature is the core
//! average: the energy released, less the heat that has left, over the sourced
//! heat capacity. It was compared with the thermocouple at the centre of one
//! element, outside the card, through the sourced hot-rod factor
//! [`HOT_ROD_FACTOR`] and a delay fitted to each recording chain. The step does
//! not use that factor.
//!
//! The card leaves out any other core loading, a start from power, a start
//! outside the starts compared on its rod table (24.00 to 27.94 degC on core
//! 215, 21.31 to 29.23 on core 217, 20.90 to 22.81 on core 231), reactivity
//! below 2.0 or above 3.0 printed dollars, a scram earlier than 1.50 s after the peak, rods held later than
//! 7.45 s after it, a second pulse on warm fuel, the time the rods take to
//! fall, boiling, anything past 14 s, and anything past the largest power,
//! energy and fuel temperature that were compared with a record. The starts
//! are the envelope, table by table, and the scrams the envelope, of the 27
//! opened pulses compared at 2.0 printed dollars and above. The floor of 2.0 printed dollars was set
//! after the held-back rows were scored, and is not a blind choice: of the
//! four held-back rows below 2.0 printed dollars, all of 28 March 2018, the
//! card read 29, 21 and 9 percent low in peak power at 1.4, 1.6 and 1.8
//! dollars and level on the repeat at 1.8, and on the six rows of that day
//! from 2.0 dollars up it was within 3.7 percent in peak power, 2.7 percent in
//! energy within 50 ms of the peak and 5.8 K at the thermocouple. The fit
//! itself reached down to 1.4 printed dollars.
//!
//! The ledger is the neutron population `N = P + Σ C_i` (MW). Summing the
//! equations, `dN/dt = ((ρ − 1) β + Σ β_i) P / Λ`. The right side is integrated
//! beside the step by the two-point Hermite rule, `h (f0 + f1) / 2 + h² (f0' −
//! f1') / 12`, from the values and rates at the two ends of the step:
//!
//! ```text
//! population_defect_mw        = N − N_anchor − ∫ ((ρ − 1) β + Σ β_i) P / Λ dt
//! residual_conservation_error = |population_defect_mw| / max(N, N_anchor)
//! ```
//!
//! The Runge–Kutta step never evaluates that integral, and the rule is not the
//! Runge–Kutta rule, so a wrong coefficient, a dropped group, a wrong stage
//! weight or an Euler step shows in the defect. The ledger closes for any value
//! of the fitted constants: it shows the arithmetic and the integrator are right.
//! It cannot show the physics matches the reactor.
//!
//! [`ReactorTrigaJsi::gate_dt_s`] is the step ceiling: `GATE_PHASE_STEP` of the
//! time constant of the prompt pole after the scram, `Λ / (β (1 + S))`.
//! [`ReactorTrigaJsi::floor_dt_s`] is 1e-9 s. The public step applies neither.
//! The gateway refuses a step outside them. In the fall of the power after the
//! scram a step is taken in pieces ([`SCRAM_PIECE_PHASE_STEP`]).
//!
//! `core` only. The state is `Copy`. An accepted step writes time, power, energy
//! released, net reactivity, the fuel temperature and its highest landed value,
//! the highest landed power, the power rate and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Effective delayed-neutron fraction `β`; the site's dollar is 700 pcm.
/// Švajger et al. 2024, Table 10.
pub const DELAYED_FRACTION: f64 = 0.00700;
/// Prompt-neutron generation time `Λ`, s. Fitted: 29.59 microseconds. The
/// article's Table 10 gives 40 microseconds, with 8 %.
pub const GENERATION_TIME_S: f64 = 29.59e-6;
/// The generation time the article gives, s. Not used by the step.
pub const GENERATION_TIME_PUBLISHED_S: f64 = 40.0e-6;
/// Delayed-neutron group fractions `β_i`. Švajger et al. 2024, Table 12. Their
/// sum is 0.00699891, as printed.
pub const GROUP_FRACTION: [f64; 6] =
    [0.00023097, 0.00153278, 0.00137180, 0.00276451, 0.00080489, 0.00029396];
/// Delayed-neutron decay constants `λ_i`, per second. Same table.
pub const GROUP_DECAY_PER_S: [f64; 6] = [0.0124, 0.0305, 0.1115, 0.3010, 1.1380, 3.0100];
/// Fuel in the core `m`, kg: 56 elements of 2.36 kg. Core maps 215, 217 and
/// 231; Švajger et al. 2024, Table 10.
pub const FUEL_MASS_KG: f64 = 56.0 * 2.36;
/// Fuel specific heat at 274.15 K, J per kg K. Švajger et al. 2024, eq. 7.2.
pub const HEAT_CAPACITY_BASE_J_PER_KG_K: f64 = 333.0;
/// Its rise with temperature, J per kg K². Same equation.
pub const HEAT_CAPACITY_SLOPE_J_PER_KG_K2: f64 = 0.678;
/// The temperature the heat capacity is counted from, K. Same equation.
pub const HEAT_CAPACITY_REFERENCE_K: f64 = 274.15;
/// Insertion rate `R`, printed dollars per s: 4.16 dollars over 80 ms.
pub const RAMP_DOLLAR_PER_S: f64 = 4.16 / 0.080;
/// Reactor power at the fire signal, MW: 10 W. Švajger et al. 2024, section 5.
pub const INITIAL_POWER_MW: f64 = 1.0e-5;
/// The cores whose rod-worth table the card holds.
pub const ROD_TABLE_CORE: [f64; 3] = [215.0, 217.0, 231.0];
/// What one printed dollar of each core's table is worth in the kinetics,
/// `K_c`, in the order of [`ROD_TABLE_CORE`]. Fitted.
pub const DOLLAR_SCALE: [f64; 3] = [0.6921, 0.6677, 0.7191];
/// What is taken from the scaled insertion of each core's table, `O_c`,
/// dollars, in the same order. Fitted. All three are negative: they add.
pub const DOLLAR_OFFSET: [f64; 3] = [-0.1204, -0.1366, -0.04949];
/// Feedback coefficient at 294 K, `a_0`, per K. Fitted.
pub const FEEDBACK_PER_K: f64 = 6.241e-5;
/// Its change with temperature, `a_1`, per K². Fitted.
pub const FEEDBACK_SLOPE_PER_K2: f64 = 8.718e-8;
/// The temperature the feedback is counted from, K.
pub const FEEDBACK_REFERENCE_K: f64 = 294.0;
/// Reactivity the scram removes `S`, dollars. An assumption, not a sourced value.
pub const SCRAM_DOLLAR: f64 = 8.0;
/// Heat flow from fuel to water `H`, W per K. Fitted, to the thermocouple.
pub const HEAT_FLOW_W_PER_K: f64 = 890.6;
/// The calculated hot-rod factor of a core loaded uniformly with 12 wt% fuel:
/// the hottest element's power over the average element's, 1.387 with 0.003, in
/// ring B (I. Vavtar, master thesis, University of Ljubljana, 2019, Table 4).
/// Sourced. Not used by the step: it is the factor through which the fuel
/// temperature was compared with the thermocouple.
pub const HOT_ROD_FACTOR: f64 = 1.387;
/// Smallest inserted reactivity, printed dollars. The smallest fitted is 1.4;
/// the floor was raised to 2.0 after the held-back rows were scored, because
/// below 2.0 the card read 9 to 29 percent low in peak power on three of the
/// four held-back rows there.
pub const REACTIVITY_MIN_DOLLAR: f64 = 2.0;
/// Largest inserted reactivity, printed dollars: the largest fitted.
pub const REACTIVITY_MAX_DOLLAR: f64 = 3.0;
/// Lowest starting fuel temperature on any table, K: 20.90 degC. The start of a
/// pulse is the first thermocouple's reading at the fire signal. The 27 opened
/// pulses compared at 2.0 printed dollars and above started at 20.902 to 29.221
/// degC (pulses 331 and 240). Each table accepts only its own span,
/// [`START_TEMPERATURE_K_BY_TABLE`]; this is the lowest of them. The pulses that
/// started at 20.3 and 32.4 degC were at 1.5 printed dollars, below the card.
pub const START_TEMPERATURE_MIN_K: f64 = 294.05;
/// Highest starting fuel temperature on any table, K: 29.23 degC. See
/// [`START_TEMPERATURE_MIN_K`].
pub const START_TEMPERATURE_MAX_K: f64 = 302.38;
/// Lowest and highest starting fuel temperature of each table, K, in the order of
/// [`ROD_TABLE_CORE`]: the starts of the opened pulses compared at 2.0 printed
/// dollars and above on that table, rounded outward to the hundredth of a kelvin.
/// Core 215: 24.0029 to 27.9375 degC (pulses 216 and 206); core 217: 21.3171 to
/// 29.2207 degC (222 and 240); core 231: 20.9023 to 22.8024 degC (331 and 340).
/// Every held-back row scored at 2.0 printed dollars and above started inside
/// its own table's span. A start inside 20.90 to 29.23 degC but outside its
/// table's span was never compared on that table and is refused.
pub const START_TEMPERATURE_K_BY_TABLE: [(f64, f64); 3] =
    [(297.15, 301.09), (294.46, 302.38), (294.05, 295.96)];
/// Earliest scram, s after the time of the peak power: the rods may drop only
/// from a step that starts this long after the peak. In the records of the 27
/// opened pulses compared at 2.0 printed dollars and above the rods dropped
/// 1.510 to 7.446 s after the peak (pulses 219 and 330), and the validation
/// dropped them that long after the card's own peak. The span is that
/// envelope, rounded outward to the hundredth of a second.
pub const SCRAM_AFTER_PEAK_MIN_S: f64 = 1.50;
/// Latest instant the rods may still be held, s after the time of the peak
/// power. See [`SCRAM_AFTER_PEAK_MIN_S`].
pub const SCRAM_AFTER_PEAK_MAX_S: f64 = 7.45;
/// Core-average fuel temperature beyond which the card refuses, K. The highest
/// in a run compared with a record is 585 K.
pub const FUEL_TEMPERATURE_MAX_K: f64 = 600.0;
/// Power beyond which the card refuses, MW. The largest compared is 954 MW.
pub const POWER_MAX_MW: f64 = 1000.0;
/// Energy released beyond which the card refuses, MW s. The largest in a run
/// compared with a record is 18.3 MW s.
pub const ENERGY_MAX_MWS: f64 = 20.0;
/// Time beyond which the card refuses, s.
pub const TIME_MAX_S: f64 = 14.0;
/// What a run's clock may pass [`TIME_MAX_S`] by, s. The clock is a running sum
/// of the steps and is off by a few units in the last place a step; a run whose
/// steps make 14 s keeps its last step.
pub const TIME_END_ALLOWANCE_S: f64 = 1.0e-9;
/// `|N − N_anchor − ∫ dN| / max(N, N_anchor)` must stay at or under this. Set by
/// measurement over runs at the ends of the span, each at the ceiling through
/// the pulse and the scram to the end of the card, and over scrams from the
/// rod's stop to two seconds after the peak, which 0.6.0 accepted: the largest
/// value found is 5.8e-9, and the limit is 34 times it. Over the scrams the card
/// accepts now, 1.50 to 7.45 s after the peak, at the ceiling, 5e-5, 2e-5 and
/// 3.1e-6 s and at the floor, the largest found is 3.5e-9. The residual compares
/// the step with the card's own quadrature of the same power, so it is a check
/// of consistency and not a bound on the error.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.0e-7;
/// Fraction of the prompt time constant after the scram that one step at the
/// ceiling advances.
pub const GATE_PHASE_STEP: f64 = 0.2;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-9;
/// For this many time constants of the prompt pole after the scram, counted from the instant
/// the rods are dropped, a step is taken in pieces: 14 ms.
pub const SCRAM_TRANSIENT_TIME_CONSTANTS: f64 = 30.0;
/// Fraction of that time constant one piece advances at most: an eighth of [`GATE_PHASE_STEP`].
pub const SCRAM_PIECE_PHASE_STEP: f64 = 0.025;
/// Most pieces one step is taken in. A step at the ceiling takes eight.
pub const SCRAM_PIECES_MAX: u32 = 64;

/// Net reactivity is above one dollar on this frame.
pub const FLAG_PROMPT_SUPERCRITICAL: u64 = 1 << 0;
/// The inserted reactivity is still below the load: the rod is moving.
pub const FLAG_ROD_MOVING: u64 = 1 << 1;
/// The rods have been dropped.
pub const FLAG_RODS_DROPPED: u64 = 1 << 2;

/// Why a step did not advance the reactor or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt`, the reactivity or the scram value was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this reactor's.
    InvalidCard,
    /// The recorded starting fuel temperature is outside the starts compared on its rod table at 2.0 printed dollars and above: 24.00 to 27.94 degC on core 215, 21.31 to 29.23 on core 217, 20.90 to 22.81 on core 231.
    StartTemperatureOutsideCard,
    /// The recorded rod-worth table is not that of core 215, 217 or 231.
    RodTableOutsideCard,
    /// The inserted reactivity is outside 2.0 to 3.0 printed dollars.
    ReactivityOutsideCard,
    /// The scram value is neither 0 (rods held) nor 1 (rods dropped).
    ScramValueOutsideCard,
    /// The inserted reactivity differs from the one this run started with; the model defines no change of it but the scram.
    ReactivityChangedDuringRun,
    /// The rods were dropped while the transient rod was still moving.
    ScramBeforeRodStopped,
    /// The rods were dropped earlier than 1.50 s after the peak power, earlier than any scram compared with a record.
    ScramBeforeCompared,
    /// The step would end with the rods still held more than 7.45 s after the peak power, later than any scram compared with a record.
    RodsHeldBeyondCompared,
    /// The rods were raised after the scram.
    RodsRaisedAfterScram,
    /// The energy released in the handed-in state is negative.
    NegativeEnergy,
    /// A peak field of the handed-in state cannot have come from a run: a peak power below the power, a peak time or peak energy outside the run so far, or a highest fuel temperature below the fuel temperature.
    ImpossiblePeak,
    /// The handed-in state has a power that is not positive, a negative precursor sum, a negative time, a fuel temperature below the starting temperature, or a scram time or scram energy outside the run so far.
    StateOutsideCard,
    /// The step would end after 14 s.
    TimeBeyondCard,
    /// The trial step released more than 20 MW s, beyond what was compared with a record.
    EnergyBeyondCard,
    /// The trial step reached more than 1000 MW, beyond what was compared with a record.
    PowerBeyondCard,
    /// The trial step took the core-average fuel temperature above 600 K, beyond what was compared with a record.
    FuelTemperatureBeyondCard,
    /// The trial step created neutron population beyond what the reactivity supplies.
    SpontaneousGeneration,
    /// The trial step destroyed neutron population the reactivity supplies.
    FirstLawBreach,
}

/// What the operator sets, held through the step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorLoad {
    /// Reactivity the transient rod inserts in all, `ρ_0`, in the dollars of the institute's table.
    pub reactivity_inserted_dollar: f64,
    /// 0 while the rods are held, 1 from the step in which they are dropped.
    pub rods_dropped: f64,
}

/// The locked reactor. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorTrigaJsi {
    pub delayed_fraction: f64,
    pub generation_time_s: f64,
    pub group_fraction: [f64; 6],
    pub group_decay_per_s: [f64; 6],
    pub fuel_mass_kg: f64,
    pub heat_capacity_base_j_per_kg_k: f64,
    pub heat_capacity_slope_j_per_kg_k2: f64,
    pub heat_capacity_reference_k: f64,
    pub ramp_dollar_per_s: f64,
    pub initial_power_mw: f64,
    pub rod_table_core: [f64; 3],
    pub dollar_scale: [f64; 3],
    pub dollar_offset: [f64; 3],
    pub feedback_per_k: f64,
    pub feedback_slope_per_k2: f64,
    pub feedback_reference_k: f64,
    pub scram_dollar: f64,
    pub heat_flow_w_per_k: f64,
    pub reactivity_min_dollar: f64,
    pub reactivity_max_dollar: f64,
    pub start_temperature_min_k: f64,
    pub start_temperature_max_k: f64,
    pub start_temperature_k_by_table: [(f64, f64); 3],
    pub scram_after_peak_min_s: f64,
    pub scram_after_peak_max_s: f64,
    pub fuel_temperature_max_k: f64,
    pub power_max_mw: f64,
    pub energy_max_mws: f64,
    pub time_max_s: f64,
    pub residual_conservation_limit: f64,
    pub gate_phase_step: f64,
    pub floor_dt_s: f64,
}

impl ReactorTrigaJsi {
    /// Card: the JSI TRIGA Mark II, 56-element core, pulse mode. The constructor takes no arguments.
    #[must_use]
    pub fn core_56_pulse_mode() -> Self {
        let reactor = Self {
            delayed_fraction: DELAYED_FRACTION,
            generation_time_s: GENERATION_TIME_S,
            group_fraction: GROUP_FRACTION,
            group_decay_per_s: GROUP_DECAY_PER_S,
            fuel_mass_kg: FUEL_MASS_KG,
            heat_capacity_base_j_per_kg_k: HEAT_CAPACITY_BASE_J_PER_KG_K,
            heat_capacity_slope_j_per_kg_k2: HEAT_CAPACITY_SLOPE_J_PER_KG_K2,
            heat_capacity_reference_k: HEAT_CAPACITY_REFERENCE_K,
            ramp_dollar_per_s: RAMP_DOLLAR_PER_S,
            initial_power_mw: INITIAL_POWER_MW,
            rod_table_core: ROD_TABLE_CORE,
            dollar_scale: DOLLAR_SCALE,
            dollar_offset: DOLLAR_OFFSET,
            feedback_per_k: FEEDBACK_PER_K,
            feedback_slope_per_k2: FEEDBACK_SLOPE_PER_K2,
            feedback_reference_k: FEEDBACK_REFERENCE_K,
            scram_dollar: SCRAM_DOLLAR,
            heat_flow_w_per_k: HEAT_FLOW_W_PER_K,
            reactivity_min_dollar: REACTIVITY_MIN_DOLLAR,
            reactivity_max_dollar: REACTIVITY_MAX_DOLLAR,
            start_temperature_min_k: START_TEMPERATURE_MIN_K,
            start_temperature_max_k: START_TEMPERATURE_MAX_K,
            start_temperature_k_by_table: START_TEMPERATURE_K_BY_TABLE,
            scram_after_peak_min_s: SCRAM_AFTER_PEAK_MIN_S,
            scram_after_peak_max_s: SCRAM_AFTER_PEAK_MAX_S,
            fuel_temperature_max_k: FUEL_TEMPERATURE_MAX_K,
            power_max_mw: POWER_MAX_MW,
            energy_max_mws: ENERGY_MAX_MWS,
            time_max_s: TIME_MAX_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_phase_step: GATE_PHASE_STEP,
            floor_dt_s: FLOOR_DT_S,
        };
        if reactor.is_valid() {
            reactor
        } else {
            ::core::panic!("JSI TRIGA reactor failed is_valid");
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
        while index < 3 {
            if self.rod_table_core[index].to_bits() != ROD_TABLE_CORE[index].to_bits()
                || self.dollar_scale[index].to_bits() != DOLLAR_SCALE[index].to_bits()
                || self.dollar_offset[index].to_bits() != DOLLAR_OFFSET[index].to_bits()
                || self.start_temperature_k_by_table[index].0.to_bits()
                    != START_TEMPERATURE_K_BY_TABLE[index].0.to_bits()
                || self.start_temperature_k_by_table[index].1.to_bits()
                    != START_TEMPERATURE_K_BY_TABLE[index].1.to_bits()
            {
                return false;
            }
            index += 1;
        }
        let pairs: [(f64, f64); 26] = [
            (self.delayed_fraction, DELAYED_FRACTION),
            (self.generation_time_s, GENERATION_TIME_S),
            (self.fuel_mass_kg, FUEL_MASS_KG),
            (self.heat_capacity_base_j_per_kg_k, HEAT_CAPACITY_BASE_J_PER_KG_K),
            (self.heat_capacity_slope_j_per_kg_k2, HEAT_CAPACITY_SLOPE_J_PER_KG_K2),
            (self.heat_capacity_reference_k, HEAT_CAPACITY_REFERENCE_K),
            (self.ramp_dollar_per_s, RAMP_DOLLAR_PER_S),
            (self.initial_power_mw, INITIAL_POWER_MW),
            (self.feedback_per_k, FEEDBACK_PER_K),
            (self.feedback_slope_per_k2, FEEDBACK_SLOPE_PER_K2),
            (self.feedback_reference_k, FEEDBACK_REFERENCE_K),
            (self.scram_dollar, SCRAM_DOLLAR),
            (self.heat_flow_w_per_k, HEAT_FLOW_W_PER_K),
            (self.reactivity_min_dollar, REACTIVITY_MIN_DOLLAR),
            (self.reactivity_max_dollar, REACTIVITY_MAX_DOLLAR),
            (self.start_temperature_min_k, START_TEMPERATURE_MIN_K),
            (self.start_temperature_max_k, START_TEMPERATURE_MAX_K),
            (self.scram_after_peak_min_s, SCRAM_AFTER_PEAK_MIN_S),
            (self.scram_after_peak_max_s, SCRAM_AFTER_PEAK_MAX_S),
            (self.fuel_temperature_max_k, FUEL_TEMPERATURE_MAX_K),
            (self.power_max_mw, POWER_MAX_MW),
            (self.energy_max_mws, ENERGY_MAX_MWS),
            (self.time_max_s, TIME_MAX_S),
            (self.residual_conservation_limit, RESIDUAL_CONSERVATION_LIMIT),
            (self.gate_phase_step, GATE_PHASE_STEP),
            (self.floor_dt_s, FLOOR_DT_S),
        ];
        index = 0;
        while index < pairs.len() {
            if pairs[index].0.to_bits() != pairs[index].1.to_bits() {
                return false;
            }
            index += 1;
        }
        true
    }

    /// Largest step the card stands behind: `GATE_PHASE_STEP · Λ / (β (1 + S))`, 0.094 ms.
    /// Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let pole = self.delayed_fraction * (1.0 + self.scram_dollar) / self.generation_time_s;
        if pole.is_finite() && pole > 0.0 && self.gate_phase_step > 0.0 {
            Some(self.gate_phase_step / pole)
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

    /// Time at which the transient rod stops for an inserted reactivity, s: `ρ_0 / R`.
    #[must_use]
    pub fn rod_stop_s(&self, reactivity_inserted_dollar: f64) -> f64 {
        reactivity_inserted_dollar / self.ramp_dollar_per_s
    }

    /// Where a core's rod-worth table sits in [`ROD_TABLE_CORE`]. `None` for any number that
    /// is not exactly one of the three; nothing is rounded to a core.
    #[must_use]
    pub fn table_of(&self, rod_table_core: f64) -> Option<usize> {
        let mut index = 0;
        while index < 3 {
            if rod_table_core == self.rod_table_core[index] {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    /// Whether a starting fuel temperature lies inside the starts compared on the table at
    /// `table`, ends included.
    #[must_use]
    pub fn start_compared_on(&self, table: usize, start_temperature_k: f64) -> bool {
        let (low, high) = self.start_temperature_k_by_table[table];
        start_temperature_k >= low && start_temperature_k <= high
    }

    /// What the whole insertion is worth in the kinetics, dollars: `K_c ρ_0 − O_c`.
    #[must_use]
    pub fn worth_dollar(&self, table: usize, reactivity_inserted_dollar: f64) -> f64 {
        self.dollar_scale[table] * reactivity_inserted_dollar - self.dollar_offset[table]
    }
}

/// Integrated reactor. The ledger and peak fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorState {
    /// Time since the fire signal.
    pub t_s: f64,
    /// Reactor power, `P`.
    pub power_mw: f64,
    /// Delayed-neutron precursor sums, `C_i`.
    pub precursor_mw: [f64; 6],
    /// Core-average fuel temperature, `T`.
    pub fuel_temperature_k: f64,
    /// Energy released since the fire signal, `E`.
    pub energy_mws: f64,
    /// Fuel and water temperature at the start, `T_0`.
    pub start_temperature_k: f64,
    /// The core whose rod-worth table the printed dollars belong to: 215, 217 or 231.
    pub rod_table_core: f64,
    /// The inserted reactivity this run started with. Zero until the first step lands.
    pub reactivity_load_dollar: f64,
    /// The rods have been dropped.
    pub rods_dropped: bool,
    /// Time at which the rods were dropped. Zero until then.
    pub scram_time_s: f64,
    /// Energy released at the instant the rods were dropped. Zero until then.
    pub energy_at_scram_mws: f64,
    /// `dP/dt` at this state under the load of the last step.
    pub power_rate_mw_per_s: f64,
    /// Net reactivity at this state under the load of the last step. Zero at the start.
    pub reactivity_net_dollar: f64,
    /// `ρ_0 f(t)` at this state under the load of the last step, printed dollars.
    pub reactivity_inserted_now_dollar: f64,
    /// `N = P + Σ C_i` at the start.
    pub population_anchor_mw: f64,
    /// `∫ ((ρ − 1) β + Σ β_i) P / Λ dt` since the start, by the Hermite rule.
    pub population_supplied_mw: f64,
    /// Signed `N − N_anchor − population_supplied_mw`. Positive is created population.
    pub population_defect_mw: f64,
    /// `|population_defect_mw| / max(N, N_anchor)`.
    pub residual_conservation_error: f64,
    /// Highest power of any landed sample.
    pub peak_power_mw: f64,
    /// Energy released at the sample of [`Self::peak_power_mw`].
    pub peak_energy_mws: f64,
    /// Time of the sample of [`Self::peak_power_mw`].
    pub peak_time_s: f64,
    /// Highest core-average fuel temperature of any landed sample.
    pub peak_fuel_temperature_k: f64,
    /// Net reactivity above one dollar.
    pub prompt_supercritical: bool,
    /// The inserted reactivity is below the load.
    pub rod_moving: bool,
}

impl ReactorState {
    /// The reactor at [`INITIAL_POWER_MW`] at zero net reactivity, the fuel at the starting
    /// temperature, time 0, each precursor sum at its equilibrium with that power,
    /// `β_i P / (Λ λ_i)`. The start is not stationary: the printed groups sum to 0.00699891 and
    /// the power equation uses [`DELAYED_FRACTION`], 0.00700, so at zero net reactivity the power
    /// falls at 0.037 per second, as if 1.6e-4 dollar were taken out. `None` unless the
    /// rod table is one of the three and the starting temperature is finite and inside the
    /// starts compared on that table at 2.0 printed dollars and above,
    /// [`START_TEMPERATURE_K_BY_TABLE`].
    #[must_use]
    pub fn try_start(
        reactor: &ReactorTrigaJsi,
        start_temperature_k: f64,
        rod_table_core: f64,
    ) -> Option<Self> {
        if !reactor.is_valid() || !start_temperature_k.is_finite() {
            return None;
        }
        if start_temperature_k < reactor.start_temperature_min_k
            || start_temperature_k > reactor.start_temperature_max_k
        {
            return None;
        }
        let table = reactor.table_of(rod_table_core)?;
        if !reactor.start_compared_on(table, start_temperature_k) {
            return None;
        }
        let power_mw = reactor.initial_power_mw;
        let mut precursor_mw = [0.0; 6];
        let mut index = 0;
        while index < 6 {
            precursor_mw[index] = reactor.group_fraction[index] * power_mw
                / (reactor.generation_time_s * reactor.group_decay_per_s[index]);
            index += 1;
        }
        let mut state = Self {
            t_s: 0.0,
            power_mw,
            precursor_mw,
            fuel_temperature_k: start_temperature_k,
            energy_mws: 0.0,
            start_temperature_k,
            rod_table_core,
            reactivity_load_dollar: 0.0,
            rods_dropped: false,
            scram_time_s: 0.0,
            energy_at_scram_mws: 0.0,
            power_rate_mw_per_s: 0.0,
            reactivity_net_dollar: 0.0,
            reactivity_inserted_now_dollar: 0.0,
            population_anchor_mw: 0.0,
            population_supplied_mw: 0.0,
            population_defect_mw: 0.0,
            residual_conservation_error: 0.0,
            peak_power_mw: power_mw,
            peak_energy_mws: 0.0,
            peak_time_s: 0.0,
            peak_fuel_temperature_k: start_temperature_k,
            prompt_supercritical: false,
            rod_moving: true,
        };
        state.population_anchor_mw = population_mw(&state);
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }
}

/// The constants of one run, gathered for the step.
#[derive(Clone, Copy)]
struct Terms {
    rho_total: f64,
    worth: f64,
    scram: f64,
    start_k: f64,
    feedback_at_start: f64,
    group_sum: f64,
}

/// `φ(T)`: reactivity lost between 294 K and `T`, as a fraction.
fn feedback(reactor: &ReactorTrigaJsi, temperature_k: f64) -> f64 {
    let x = temperature_k - reactor.feedback_reference_k;
    reactor.feedback_per_k * x + 0.5 * reactor.feedback_slope_per_k2 * x * x
}

/// `dφ/dT`.
fn feedback_slope(reactor: &ReactorTrigaJsi, temperature_k: f64) -> f64 {
    let x = temperature_k - reactor.feedback_reference_k;
    reactor.feedback_per_k + reactor.feedback_slope_per_k2 * x
}

fn terms(reactor: &ReactorTrigaJsi, state: &ReactorState, load: ReactorLoad, table: usize) -> Terms {
    let mut group_sum = 0.0;
    let mut index = 0;
    while index < 6 {
        group_sum += reactor.group_fraction[index];
        index += 1;
    }
    Terms {
        rho_total: load.reactivity_inserted_dollar,
        worth: reactor.worth_dollar(table, load.reactivity_inserted_dollar),
        scram: if load.rods_dropped == 1.0 { reactor.scram_dollar } else { 0.0 },
        start_k: state.start_temperature_k,
        feedback_at_start: feedback(reactor, state.start_temperature_k),
        group_sum,
    }
}

/// `ρ_0 f(t) = min(ρ_0, R t)`, printed dollars.
fn inserted(reactor: &ReactorTrigaJsi, terms: &Terms, t_s: f64) -> f64 {
    let ramped = reactor.ramp_dollar_per_s * t_s;
    if ramped < terms.rho_total {
        ramped
    } else {
        terms.rho_total
    }
}

fn net_reactivity(reactor: &ReactorTrigaJsi, terms: &Terms, t_s: f64, y: &[f64; 9]) -> f64 {
    terms.worth * (inserted(reactor, terms, t_s) / terms.rho_total)
        - (feedback(reactor, y[7]) - terms.feedback_at_start) / reactor.delayed_fraction
        - terms.scram
}

/// Advance one Runge–Kutta step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    reactor: &ReactorTrigaJsi,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    let table = validate(reactor, state, load, dt_s)?;
    let terms = terms(reactor, state, load, table);
    let pieces = scram_pieces(reactor, state, load, dt_s);
    let advanced =
        through_the_stop(rk4, reactor, &terms, state.t_s, &state_vector(state), dt_s, pieces);
    commit(reactor, state, frame, &terms, load, dt_s, advanced)
}

/// In how many equal pieces a step is taken. One, except while the power falls after the
/// scram: the rods take 8 dollars out at the start of a step, the power then falls with the
/// time constant of the prompt pole, `Λ / (β (1 + S))`, 0.47 ms, and a step at the ceiling is a
/// fifth of that. Over the thirty-odd steps of that fall the fourth-order error of the step and
/// of the ledger's rule adds up to a millionth of the power, which passes the ledger's limit
/// when the power is a large part of the population, near the peak. Taken in pieces of at most
/// [`SCRAM_PIECE_PHASE_STEP`] of the time constant, each with its own integral, the same step
/// is four thousand times closer.
fn scram_pieces(reactor: &ReactorTrigaJsi, state: &ReactorState, load: ReactorLoad, dt_s: f64) -> u32 {
    if load.rods_dropped != 1.0 {
        return 1;
    }
    let time_constant =
        reactor.generation_time_s / (reactor.delayed_fraction * (1.0 + reactor.scram_dollar));
    let since = if state.rods_dropped { state.t_s - state.scram_time_s } else { 0.0 };
    if !(since < SCRAM_TRANSIENT_TIME_CONSTANTS * time_constant) {
        return 1;
    }
    // Eight units in the last place are given back, so that a step exactly at the ceiling,
    // whose quotient is 8 to within rounding, takes eight pieces and not nine.
    let wanted = dt_s / (SCRAM_PIECE_PHASE_STEP * time_constant) * (1.0 - 8.0 * f64::EPSILON);
    if !(wanted > 1.0) {
        return 1;
    }
    if wanted >= SCRAM_PIECES_MAX as f64 {
        return SCRAM_PIECES_MAX;
    }
    let whole = wanted as u32;
    if (whole as f64) < wanted { whole + 1 } else { whole }
}

/// What a stepping method returns: the nine variables at the end of the step and the ledger's
/// integral over it.
#[derive(Clone, Copy)]
struct Advanced {
    y: [f64; 9],
    supplied_mw: f64,
}

/// One step of `method`, except in the one step in which the rod stops: there the inserted
/// reactivity has a corner, and the step is taken as two that meet at it, each with its own
/// Hermite integral. A step in the fall of the power after the scram is taken in `pieces`.
fn through_the_stop(
    method: fn(&ReactorTrigaJsi, &Terms, f64, &[f64; 9], f64) -> [f64; 9],
    reactor: &ReactorTrigaJsi,
    terms: &Terms,
    t_s: f64,
    y: &[f64; 9],
    dt_s: f64,
    pieces: u32,
) -> Advanced {
    let stop = terms.rho_total / reactor.ramp_dollar_per_s;
    if pieces > 1 {
        // After the scram the rod has stopped: every piece is smooth.
        let h = dt_s / pieces as f64;
        let mut at = *y;
        let mut supplied_mw = 0.0;
        let mut piece = 0;
        while piece < pieces {
            let t = t_s + piece as f64 * h;
            let next = method(reactor, terms, t, &at, h);
            supplied_mw += supplied(reactor, terms, t, &at, &next, h, false);
            at = next;
            piece += 1;
        }
        return Advanced { y: at, supplied_mw };
    }
    let first = stop - t_s;
    if first > 0.0 && first < dt_s {
        let at_stop = method(reactor, terms, t_s, y, first);
        let one = supplied(reactor, terms, t_s, y, &at_stop, first, true);
        let end = method(reactor, terms, stop, &at_stop, dt_s - first);
        let two = supplied(reactor, terms, stop, &at_stop, &end, dt_s - first, false);
        return Advanced { y: end, supplied_mw: one + two };
    }
    let end = method(reactor, terms, t_s, y, dt_s);
    let moving = t_s < stop;
    Advanced { y: end, supplied_mw: supplied(reactor, terms, t_s, y, &end, dt_s, moving) }
}

/// `((ρ − 1) β + Σ β_i) P / Λ` and its time derivative at `(t, y)`; `moving` says on which
/// side of the rod's stop the piece lies.
fn supply_integrand(
    reactor: &ReactorTrigaJsi,
    terms: &Terms,
    t_s: f64,
    y: &[f64; 9],
    moving: bool,
) -> (f64, f64) {
    let per_s = terms.worth * reactor.ramp_dollar_per_s / terms.rho_total;
    let rho = if moving { per_s * t_s } else { terms.worth }
        - (feedback(reactor, y[7]) - terms.feedback_at_start) / reactor.delayed_fraction
        - terms.scram;
    let k = rates_with(reactor, terms, rho, y);
    let rho_rate = if moving { per_s } else { 0.0 }
        - feedback_slope(reactor, y[7]) * k[7] / reactor.delayed_fraction;
    let gain = ((rho - 1.0) * reactor.delayed_fraction + terms.group_sum) / reactor.generation_time_s;
    (
        gain * y[0],
        reactor.delayed_fraction / reactor.generation_time_s * rho_rate * y[0] + gain * k[0],
    )
}

/// The ledger's integral over one smooth piece: `h (f0 + f1) / 2 + h² (f0' − f1') / 12`.
fn supplied(
    reactor: &ReactorTrigaJsi,
    terms: &Terms,
    t_s: f64,
    y0: &[f64; 9],
    y1: &[f64; 9],
    h: f64,
    moving: bool,
) -> f64 {
    let (f0, d0) = supply_integrand(reactor, terms, t_s, y0, moving);
    let (f1, d1) = supply_integrand(reactor, terms, t_s + h, y1, moving);
    0.5 * h * (f0 + f1) + h * h * (d0 - d1) / 12.0
}

/// The checks of a step, in the order the refusals are listed. Returns the place of the
/// state's rod table.
fn validate(
    reactor: &ReactorTrigaJsi,
    state: &ReactorState,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<usize, StepReject> {
    if !dt_s.is_finite()
        || !load.reactivity_inserted_dollar.is_finite()
        || !load.rods_dropped.is_finite()
    {
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
    if state.start_temperature_k < reactor.start_temperature_min_k
        || state.start_temperature_k > reactor.start_temperature_max_k
    {
        return Err(StepReject::StartTemperatureOutsideCard);
    }
    let table = match reactor.table_of(state.rod_table_core) {
        Some(table) => table,
        None => return Err(StepReject::RodTableOutsideCard),
    };
    if !reactor.start_compared_on(table, state.start_temperature_k) {
        return Err(StepReject::StartTemperatureOutsideCard);
    }
    if load.reactivity_inserted_dollar < reactor.reactivity_min_dollar
        || load.reactivity_inserted_dollar > reactor.reactivity_max_dollar
    {
        return Err(StepReject::ReactivityOutsideCard);
    }
    if load.rods_dropped != 0.0 && load.rods_dropped != 1.0 {
        return Err(StepReject::ScramValueOutsideCard);
    }
    if state.energy_mws < 0.0 {
        return Err(StepReject::NegativeEnergy);
    }
    if !(state.power_mw > 0.0)
        || state.t_s < 0.0
        || state.fuel_temperature_k < state.start_temperature_k
        || state.scram_time_s < 0.0
        || state.scram_time_s > state.t_s
        || state.energy_at_scram_mws < 0.0
        || state.energy_at_scram_mws > state.energy_mws
    {
        return Err(StepReject::StateOutsideCard);
    }
    let mut index = 0;
    while index < 6 {
        if state.precursor_mw[index] < 0.0 {
            return Err(StepReject::StateOutsideCard);
        }
        index += 1;
    }
    if state.peak_power_mw < state.power_mw
        || state.peak_time_s < 0.0
        || state.peak_time_s > state.t_s
        || state.peak_energy_mws < 0.0
        || state.peak_energy_mws > state.energy_mws
        || state.peak_fuel_temperature_k < state.fuel_temperature_k
    {
        return Err(StepReject::ImpossiblePeak);
    }
    if state.t_s > 0.0
        && state.reactivity_load_dollar.to_bits() != load.reactivity_inserted_dollar.to_bits()
    {
        return Err(StepReject::ReactivityChangedDuringRun);
    }
    let dropping = load.rods_dropped == 1.0;
    if state.rods_dropped && !dropping {
        return Err(StepReject::RodsRaisedAfterScram);
    }
    if dropping
        && !state.rods_dropped
        && reactor.ramp_dollar_per_s * state.t_s < load.reactivity_inserted_dollar
    {
        return Err(StepReject::ScramBeforeRodStopped);
    }
    // The scram envelope is counted from the peak so far. While the power still rises the peak
    // is the present sample, so a scram before the pulse is refused here too. The allowance is
    // the clock's, so that a run whose steps make the edge exactly keeps it.
    if dropping
        && !state.rods_dropped
        && state.t_s - state.peak_time_s < reactor.scram_after_peak_min_s - TIME_END_ALLOWANCE_S
    {
        return Err(StepReject::ScramBeforeCompared);
    }
    if !dropping
        && !(state.t_s + dt_s - state.peak_time_s
            <= reactor.scram_after_peak_max_s + TIME_END_ALLOWANCE_S)
    {
        return Err(StepReject::RodsHeldBeyondCompared);
    }
    if !(state.t_s + dt_s <= reactor.time_max_s + TIME_END_ALLOWANCE_S) {
        return Err(StepReject::TimeBeyondCard);
    }
    Ok(table)
}

fn commit(
    reactor: &ReactorTrigaJsi,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    terms: &Terms,
    load: ReactorLoad,
    dt_s: f64,
    advanced: Advanced,
) -> Result<(), StepReject> {
    let saved = *state;
    let y = advanced.y;
    let mut index = 0;
    while index < 9 {
        if !y[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    if !advanced.supplied_mw.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if y[8] > reactor.energy_max_mws {
        return Err(StepReject::EnergyBeyondCard);
    }
    if y[0] > reactor.power_max_mw {
        return Err(StepReject::PowerBeyondCard);
    }
    if y[7] > reactor.fuel_temperature_max_k {
        return Err(StepReject::FuelTemperatureBeyondCard);
    }
    let t_new = state.t_s + dt_s;
    if !state.rods_dropped && load.rods_dropped == 1.0 {
        state.rods_dropped = true;
        state.scram_time_s = state.t_s;
        state.energy_at_scram_mws = state.energy_mws;
    }
    state.reactivity_load_dollar = load.reactivity_inserted_dollar;
    state.population_supplied_mw += advanced.supplied_mw;
    state.t_s = t_new;
    state.power_mw = y[0];
    index = 0;
    while index < 6 {
        state.precursor_mw[index] = y[1 + index];
        index += 1;
    }
    state.fuel_temperature_k = y[7];
    state.energy_mws = y[8];
    state.reactivity_inserted_now_dollar = inserted(reactor, terms, t_new);
    state.reactivity_net_dollar = net_reactivity(reactor, terms, t_new, &y);
    state.power_rate_mw_per_s = rates(reactor, terms, t_new, &y)[0];
    let population = population_mw(state);
    state.population_defect_mw =
        population - state.population_anchor_mw - state.population_supplied_mw;
    let scale = if population > state.population_anchor_mw {
        population
    } else {
        state.population_anchor_mw
    };
    state.residual_conservation_error = state.population_defect_mw.abs() / scale;
    if state.power_mw > state.peak_power_mw {
        state.peak_power_mw = state.power_mw;
        state.peak_energy_mws = state.energy_mws;
        state.peak_time_s = state.t_s;
    }
    if state.fuel_temperature_k > state.peak_fuel_temperature_k {
        state.peak_fuel_temperature_k = state.fuel_temperature_k;
    }
    state.rod_moving = state.reactivity_inserted_now_dollar < terms.rho_total;
    state.prompt_supercritical = state.reactivity_net_dollar > 1.0;
    if !state_is_finite(state) || !(scale > 0.0) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if state.residual_conservation_error > reactor.residual_conservation_limit {
        let created = state.population_defect_mw > 0.0;
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

/// The nine variables in the order of the Runge–Kutta step: P, C_1 to C_6, T, E.
fn state_vector(state: &ReactorState) -> [f64; 9] {
    [
        state.power_mw,
        state.precursor_mw[0],
        state.precursor_mw[1],
        state.precursor_mw[2],
        state.precursor_mw[3],
        state.precursor_mw[4],
        state.precursor_mw[5],
        state.fuel_temperature_k,
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

/// The right-hand side of the equations at `(t, y)`.
fn rates(reactor: &ReactorTrigaJsi, terms: &Terms, t_s: f64, y: &[f64; 9]) -> [f64; 9] {
    rates_with(reactor, terms, net_reactivity(reactor, terms, t_s, y), y)
}

/// The right-hand side with the net reactivity given.
fn rates_with(reactor: &ReactorTrigaJsi, terms: &Terms, rho_net: f64, y: &[f64; 9]) -> [f64; 9] {
    let mut out = [0.0; 9];
    let mut delayed = 0.0;
    let mut index = 0;
    while index < 6 {
        let decay = reactor.group_decay_per_s[index];
        delayed += decay * y[1 + index];
        out[1 + index] =
            reactor.group_fraction[index] / reactor.generation_time_s * y[0] - decay * y[1 + index];
        index += 1;
    }
    out[0] = (rho_net - 1.0) * reactor.delayed_fraction / reactor.generation_time_s * y[0] + delayed;
    let heat_capacity = reactor.fuel_mass_kg
        * (reactor.heat_capacity_base_j_per_kg_k
            + reactor.heat_capacity_slope_j_per_kg_k2 * (y[7] - reactor.heat_capacity_reference_k));
    out[7] = (y[0] * 1.0e6 - reactor.heat_flow_w_per_k * (y[7] - terms.start_k)) / heat_capacity;
    out[8] = y[0];
    out
}

fn advance(y: &[f64; 9], k: &[f64; 9], scale: f64) -> [f64; 9] {
    let mut out = [0.0; 9];
    let mut index = 0;
    while index < 9 {
        out[index] = y[index] + scale * k[index];
        index += 1;
    }
    out
}

/// Classical fourth-order Runge–Kutta.
fn rk4(
    reactor: &ReactorTrigaJsi,
    terms: &Terms,
    t_s: f64,
    y: &[f64; 9],
    dt_s: f64,
) -> [f64; 9] {
    let half = 0.5 * dt_s;
    let k1 = rates(reactor, terms, t_s, y);
    let k2 = rates(reactor, terms, t_s + half, &advance(y, &k1, half));
    let k3 = rates(reactor, terms, t_s + half, &advance(y, &k2, half));
    let k4 = rates(reactor, terms, t_s + dt_s, &advance(y, &k3, dt_s));
    let mut out = [0.0; 9];
    let mut index = 0;
    while index < 9 {
        out[index] =
            y[index] + dt_s / 6.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
        index += 1;
    }
    out
}

fn publish(state: &ReactorState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.power_mw as f32;
    frame.pos[1] = state.energy_mws as f32;
    frame.pos[2] = state.reactivity_net_dollar as f32;
    frame.vel[0] = state.fuel_temperature_k as f32;
    frame.vel[1] = state.peak_fuel_temperature_k as f32;
    frame.vel[2] = state.peak_power_mw as f32;
    frame.force_torque = state.power_rate_mw_per_s as f32;
    frame.residual = state.residual_conservation_error as f32;
    let mut flags = 0;
    if state.prompt_supercritical {
        flags |= FLAG_PROMPT_SUPERCRITICAL;
    }
    if state.rod_moving {
        flags |= FLAG_ROD_MOVING;
    }
    if state.rods_dropped {
        flags |= FLAG_RODS_DROPPED;
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
    state.t_s.is_finite()
        && state.power_mw.is_finite()
        && state.fuel_temperature_k.is_finite()
        && state.energy_mws.is_finite()
        && state.start_temperature_k.is_finite()
        && state.rod_table_core.is_finite()
        && state.reactivity_load_dollar.is_finite()
        && state.scram_time_s.is_finite()
        && state.energy_at_scram_mws.is_finite()
        && state.power_rate_mw_per_s.is_finite()
        && state.reactivity_net_dollar.is_finite()
        && state.reactivity_inserted_now_dollar.is_finite()
        && state.population_anchor_mw.is_finite()
        && state.population_supplied_mw.is_finite()
        && state.population_defect_mw.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.peak_power_mw.is_finite()
        && state.peak_energy_mws.is_finite()
        && state.peak_time_s.is_finite()
        && state.peak_fuel_temperature_k.is_finite()
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;
    use std::println;

    use super::{
        Advanced, DELAYED_FRACTION, DOLLAR_OFFSET, DOLLAR_SCALE, ENERGY_MAX_MWS,
        FLAG_PROMPT_SUPERCRITICAL, FLAG_ROD_MOVING, FLAG_RODS_DROPPED, FUEL_MASS_KG,
        FUEL_TEMPERATURE_MAX_K, GENERATION_TIME_S, GROUP_FRACTION, INITIAL_POWER_MW, POWER_MAX_MW,
        RAMP_DOLLAR_PER_S, RESIDUAL_CONSERVATION_LIMIT, ROD_TABLE_CORE, ReactorLoad, ReactorState,
        ReactorTrigaJsi, SCRAM_AFTER_PEAK_MAX_S, SCRAM_AFTER_PEAK_MIN_S, START_TEMPERATURE_K_BY_TABLE,
        START_TEMPERATURE_MAX_K, START_TEMPERATURE_MIN_K, StepReject, TIME_MAX_S, Terms, advance, commit, feedback, feedback_slope,
        rates, rk4, scram_pieces, state_vector, step, terms, through_the_stop, validate,
    };
    use crate::frame::LastStateFrame64;

    type Method = fn(&ReactorTrigaJsi, &Terms, f64, &[f64; 9], f64) -> [f64; 9];

    /// The starts compared on each table, K: core 215, 217 and 231.
    const T215: (f64, f64) = START_TEMPERATURE_K_BY_TABLE[0];
    const T217: (f64, f64) = START_TEMPERATURE_K_BY_TABLE[1];
    const T231: (f64, f64) = START_TEMPERATURE_K_BY_TABLE[2];

    fn load(rho: f64) -> ReactorLoad {
        ReactorLoad { reactivity_inserted_dollar: rho, rods_dropped: 0.0 }
    }

    fn dropped(rho: f64) -> ReactorLoad {
        ReactorLoad { reactivity_inserted_dollar: rho, rods_dropped: 1.0 }
    }

    /// A start at `kelvin` on the table of `core`.
    fn start_on(kelvin: f64, core: f64) -> (ReactorTrigaJsi, ReactorState) {
        let reactor = ReactorTrigaJsi::core_56_pulse_mode();
        let state = ReactorState::try_start(&reactor, kelvin, core).unwrap();
        (reactor, state)
    }

    /// A start at `celsius` on the table of core 217.
    fn start(celsius: f64) -> (ReactorTrigaJsi, ReactorState) {
        start_on(celsius + 273.15, 217.0)
    }

    fn marked_frame() -> LastStateFrame64 {
        let mut frame = LastStateFrame64::from_bytes([0u8; 64]);
        frame.proof = [0xA5; 16];
        frame.t = -1.0;
        frame
    }

    fn within(got: f64, expected: f64, relative: f64) -> bool {
        (got - expected).abs() <= relative * expected.abs()
    }

    fn ceiling() -> f64 {
        ReactorTrigaJsi::core_56_pulse_mode().gate_dt_s().unwrap()
    }

    /// A step taken with another stepping method in place of Runge–Kutta, through the card's own commit.
    fn step_with(
        method: Method,
        reactor: &ReactorTrigaJsi,
        state: &mut ReactorState,
        frame: &mut LastStateFrame64,
        load: ReactorLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        let table = validate(reactor, state, load, dt_s)?;
        let terms = terms(reactor, state, load, table);
        let pieces = scram_pieces(reactor, state, load, dt_s);
        let advanced = through_the_stop(method, reactor, &terms, state.t_s, &state_vector(state), dt_s, pieces);
        commit(reactor, state, frame, &terms, load, dt_s, advanced)
    }

    fn euler(reactor: &ReactorTrigaJsi, terms: &Terms, t_s: f64, y: &[f64; 9], dt_s: f64) -> [f64; 9] {
        advance(y, &rates(reactor, terms, t_s, y), dt_s)
    }

    /// Runge–Kutta with the weights 1, 2, 2, 1 replaced by 1, 2, 1, 2.
    fn wrong_weights(reactor: &ReactorTrigaJsi, terms: &Terms, t_s: f64, y: &[f64; 9], dt_s: f64) -> [f64; 9] {
        let half = 0.5 * dt_s;
        let k1 = rates(reactor, terms, t_s, y);
        let k2 = rates(reactor, terms, t_s + half, &advance(y, &k1, half));
        let k3 = rates(reactor, terms, t_s + half, &advance(y, &k2, half));
        let k4 = rates(reactor, terms, t_s + dt_s, &advance(y, &k3, dt_s));
        let mut out = [0.0; 9];
        for i in 0..9 {
            out[i] = y[i] + dt_s / 6.0 * (k1[i] + 2.0 * k2[i] + k3[i] + 2.0 * k4[i]);
        }
        out
    }

    /// Runge–Kutta on equations whose fuel never heats: the feedback is dropped.
    fn no_feedback(reactor: &ReactorTrigaJsi, terms: &Terms, t_s: f64, y: &[f64; 9], dt_s: f64) -> [f64; 9] {
        let cold = |t: f64, v: &[f64; 9]| {
            let mut frozen = *v;
            frozen[7] = terms.start_k;
            let mut k = rates(reactor, terms, t, &frozen);
            k[7] = 0.0;
            k
        };
        let half = 0.5 * dt_s;
        let k1 = cold(t_s, y);
        let k2 = cold(t_s + half, &advance(y, &k1, half));
        let k3 = cold(t_s + half, &advance(y, &k2, half));
        let k4 = cold(t_s + dt_s, &advance(y, &k3, dt_s));
        let mut out = [0.0; 9];
        for i in 0..9 {
            out[i] = y[i] + dt_s / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        out
    }

    struct Run {
        last: ReactorState,
        steps: u64,
        largest_residual: f64,
        end: Result<(), StepReject>,
    }

    /// A pulse from `kelvin` stepped with `method` to `until_s`, the rods dropped at `scram_s`.
    fn run_on(method: Method, kelvin: f64, core: f64, rho: f64, dt_s: f64, scram_s: f64, until_s: f64) -> Run {
        let (reactor, mut state) = start_on(kelvin, core);
        let mut frame = marked_frame();
        let mut steps = 0;
        let mut largest = 0.0;
        let mut end = Ok(());
        while state.t_s + dt_s <= until_s {
            let l = if state.t_s >= scram_s { dropped(rho) } else { load(rho) };
            end = step_with(method, &reactor, &mut state, &mut frame, l, dt_s);
            if end.is_err() {
                break;
            }
            steps += 1;
            if state.residual_conservation_error > largest {
                largest = state.residual_conservation_error;
            }
        }
        Run { last: state, steps, largest_residual: largest, end }
    }


    #[test]
    fn constants_are_locked() {
        let reactor = ReactorTrigaJsi::core_56_pulse_mode();
        assert!(reactor.is_valid());
        let mut altered = reactor;
        altered.dollar_scale[1] = 1.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.dollar_offset[2] = 0.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.feedback_per_k *= 1.0 + 1.0e-15;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.generation_time_s = super::GENERATION_TIME_PUBLISHED_S;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.rod_table_core[0] = 253.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.group_decay_per_s[5] = 3.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.reactivity_max_dollar = 5.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.residual_conservation_limit = 1.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.start_temperature_k_by_table[2].1 = START_TEMPERATURE_MAX_K;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.start_temperature_k_by_table[0].0 = START_TEMPERATURE_MIN_K;
        assert!(!altered.is_valid());
        let (_, mut state) = start(25.0);
        let mut frame = marked_frame();
        let before = state;
        assert_eq!(
            step(&altered, &mut state, &mut frame, load(2.0), 5.0e-5),
            Err(StepReject::InvalidCard)
        );
        assert_eq!(state, before);
        let sum: f64 = GROUP_FRACTION.iter().sum();
        assert!(within(sum, 0.00699891, 1.0e-9));
        assert!(within(FUEL_MASS_KG, 132.16, 1.0e-12));
        assert!(within(RAMP_DOLLAR_PER_S, 52.0, 1.0e-12));
        assert!(within(ceiling(), 9.39365e-5, 1.0e-5));
        // the fitted generation time is 3.3 times the article's 8 % below the article's value
        assert!(within(GENERATION_TIME_S, 29.59e-6, 1.0e-12));
        // every printed dollar in range is prompt supercritical in the kinetics on every table
        for table in 0..3 {
            assert!(reactor.worth_dollar(table, super::REACTIVITY_MIN_DOLLAR) > 1.0);
            assert!(reactor.worth_dollar(table, 3.0) < 2.3);
        }
        assert_eq!(super::HOT_ROD_FACTOR, 1.387);
    }

    #[test]
    fn the_start_is_not_stationary_and_only_the_compared_starts_and_the_three_tables_start() {
        let reactor = ReactorTrigaJsi::core_56_pulse_mode();
        for (table, core) in ROD_TABLE_CORE.into_iter().enumerate() {
            let (low, high) = START_TEMPERATURE_K_BY_TABLE[table];
            for kelvin in [low, 0.5 * (low + high), high] {
                let state = ReactorState::try_start(&reactor, kelvin, core).unwrap();
                assert_eq!(state.power_mw.to_bits(), INITIAL_POWER_MW.to_bits());
                assert_eq!(state.rod_table_core, core);
                let table = reactor.table_of(core).unwrap();
                let t = terms(&reactor, &state, load(2.0), table);
                let k = rates(&reactor, &t, 0.0, &state_vector(&state));
                for i in 0..6 {
                    assert!(k[1 + i].abs() < 1.0e-12 * state.precursor_mw[i]);
                }
                // the printed groups sum to 0.00699891, not 0.007: the power falls at 0.037 per second
                assert!(within(k[0] / state.power_mw, (0.00699891 - DELAYED_FRACTION) / GENERATION_TIME_S, 1.0e-6));
                assert!(within(-k[0] / state.power_mw, 0.0368, 1.0e-2));
                // as if 1.6e-4 dollar were taken out
                assert!(within((DELAYED_FRACTION - 0.00699891) / DELAYED_FRACTION, 1.557e-4, 1.0e-3));
                assert_eq!(state.fuel_temperature_k, state.start_temperature_k);
                assert_eq!(state.peak_fuel_temperature_k, state.start_temperature_k);
                assert!(state.rod_moving && !state.rods_dropped);
            }
        }
        // the starts compared on each table at 2.0 printed dollars and above, rounded outward to
        // the hundredth of a kelvin: core 215 24.0029 to 27.9375 degC, core 217 21.3171 to
        // 29.2207, core 231 20.9023 to 22.8024
        assert_eq!(START_TEMPERATURE_K_BY_TABLE, [(297.15, 301.09), (294.46, 302.38), (294.05, 295.96)]);
        for ((low, high), (first, last)) in
            START_TEMPERATURE_K_BY_TABLE.into_iter().zip([(24.0029, 27.9375), (21.3171, 29.2207), (20.9023, 22.8024)])
        {
            assert!(low <= first + 273.15 && first + 273.15 - low < 0.01);
            assert!(high >= last + 273.15 && high - (last + 273.15) < 0.01);
        }
        // the published minimum and maximum are the union of the three
        assert_eq!((START_TEMPERATURE_MIN_K, START_TEMPERATURE_MAX_K), (294.05, 302.38));
        assert_eq!(START_TEMPERATURE_MIN_K, T231.0);
        assert_eq!(START_TEMPERATURE_MAX_K, T217.1);
        assert!(T215.0 > START_TEMPERATURE_MIN_K && T215.1 < START_TEMPERATURE_MAX_K);
        // on each table the exact ends start; one unit in the last place beyond each does not
        for (table, core) in ROD_TABLE_CORE.into_iter().enumerate() {
            let (low, high) = START_TEMPERATURE_K_BY_TABLE[table];
            assert!(ReactorState::try_start(&reactor, low, core).is_some(), "{core}");
            assert!(ReactorState::try_start(&reactor, high, core).is_some(), "{core}");
            assert!(ReactorState::try_start(&reactor, f64::from_bits(low.to_bits() - 1), core).is_none(), "{core}");
            assert!(ReactorState::try_start(&reactor, f64::from_bits(high.to_bits() + 1), core).is_none(), "{core}");
        }
        // inside the union but never compared on that table
        assert!(ReactorState::try_start(&reactor, 296.0, 231.0).is_none());
        assert!(ReactorState::try_start(&reactor, 294.2, 215.0).is_none());
        assert!(ReactorState::try_start(&reactor, 302.0, 215.0).is_none());
        assert!(ReactorState::try_start(&reactor, 294.2, 217.0).is_none());
        assert!(ReactorState::try_start(&reactor, 302.0, 231.0).is_none());
        // 0.6.0 accepted 20 to 33 degC: its ends, the 20.3 and 32.4 degC of the pulses at 1.5
        // printed dollars, and the ends of the compared envelope less a hundredth
        for core in ROD_TABLE_CORE {
            for celsius in [20.0, 20.3, 20.89, 29.24, 32.4, 33.0, 19.9, 33.1, 35.0, 0.0, 100.0] {
                assert!(ReactorState::try_start(&reactor, celsius + 273.15, core).is_none());
            }
        }
        assert!(ReactorState::try_start(&reactor, f64::NAN, 215.0).is_none());
        assert!(ReactorState::try_start(&reactor, f64::INFINITY, 215.0).is_none());
        // a core between two tables, a later core, and a core one unit in the last place off
        for core in [216.0, 218.0, 224.0, 253.0, 0.0, -215.0, 215.5, f64::from_bits(215.0_f64.to_bits() + 1), f64::NAN] {
            assert!(ReactorState::try_start(&reactor, 298.15, core).is_none(), "{core}");
            assert!(reactor.table_of(core).is_none());
        }
    }

    #[test]
    fn every_refusal_is_named_and_rolls_back_state_and_frame() {
        let (reactor, fresh) = start(25.0);
        let dt = 5.0e-5;
        let refuse = |state: ReactorState, load: ReactorLoad, dt_s: f64, expected: StepReject| {
            let mut s = state;
            let mut frame = marked_frame();
            let frame_before = frame.to_bytes();
            assert_eq!(step(&reactor, &mut s, &mut frame, load, dt_s), Err(expected));
            // compared field by field through Debug so that a NaN planted in the state still compares
            assert_eq!(std::format!("{:?}", s), std::format!("{:?}", state));
            assert_eq!(frame.to_bytes(), frame_before);
        };
        refuse(fresh, load(2.0), f64::NAN, StepReject::NonFiniteInput);
        refuse(fresh, load(f64::INFINITY), dt, StepReject::NonFiniteInput);
        refuse(
            fresh,
            ReactorLoad { reactivity_inserted_dollar: 2.0, rods_dropped: f64::NAN },
            dt,
            StepReject::NonFiniteInput,
        );
        let mut bad = fresh;
        bad.precursor_mw[3] = f64::NAN;
        refuse(bad, load(2.0), dt, StepReject::NonFiniteState);
        refuse(fresh, load(2.0), 0.0, StepReject::NonPositiveDt);
        refuse(fresh, load(2.0), -dt, StepReject::NonPositiveDt);
        let mut warm = fresh;
        warm.start_temperature_k = 320.0;
        warm.fuel_temperature_k = 320.0;
        warm.peak_fuel_temperature_k = 320.0;
        refuse(warm, load(2.0), dt, StepReject::StartTemperatureOutsideCard);
        // inside 20.90 to 29.23 degC but outside the starts compared on the state's own table
        for (kelvin, core) in [(296.0, 231.0), (294.2, 215.0), (302.0, 215.0), (294.2, 217.0)] {
            let mut other = fresh;
            other.rod_table_core = core;
            other.start_temperature_k = kelvin;
            other.fuel_temperature_k = kelvin;
            other.peak_fuel_temperature_k = kelvin;
            refuse(other, load(2.0), dt, StepReject::StartTemperatureOutsideCard);
        }
        let mut other_core = fresh;
        other_core.rod_table_core = 253.0;
        refuse(other_core, load(2.0), dt, StepReject::RodTableOutsideCard);
        other_core.rod_table_core = 216.0;
        refuse(other_core, load(2.0), dt, StepReject::RodTableOutsideCard);
        // the exact ends of the reactivity land; one unit in the last place beyond is refused
        for rho in [super::REACTIVITY_MIN_DOLLAR, super::REACTIVITY_MAX_DOLLAR] {
            let mut s = fresh;
            let mut frame = marked_frame();
            assert_eq!(step(&reactor, &mut s, &mut frame, load(rho), dt), Ok(()));
        }
        assert_eq!(super::REACTIVITY_MIN_DOLLAR, 2.0);
        refuse(fresh, load(f64::from_bits(2.0_f64.to_bits() - 1)), dt, StepReject::ReactivityOutsideCard);
        // the smallest fitted reactivity is below the floor set after the held-back rows were scored
        refuse(fresh, load(1.4), dt, StepReject::ReactivityOutsideCard);
        refuse(fresh, load(f64::from_bits(3.0_f64.to_bits() + 1)), dt, StepReject::ReactivityOutsideCard);
        refuse(fresh, load(1.99), dt, StepReject::ReactivityOutsideCard);
        refuse(fresh, load(3.01), dt, StepReject::ReactivityOutsideCard);
        refuse(
            fresh,
            ReactorLoad { reactivity_inserted_dollar: 2.0, rods_dropped: 0.5 },
            dt,
            StepReject::ScramValueOutsideCard,
        );
        // a scram after the rod has stopped and before 1.50 s after the peak
        let mut early = fresh;
        early.t_s = 1.0;
        early.peak_time_s = 0.17;
        early.reactivity_load_dollar = 2.0;
        refuse(early, dropped(2.0), dt, StepReject::ScramBeforeCompared);
        // rods held past 7.45 s after the peak
        let mut held = early;
        held.t_s = 0.17 + SCRAM_AFTER_PEAK_MAX_S;
        refuse(held, load(2.0), dt, StepReject::RodsHeldBeyondCompared);
        let mut late = fresh;
        late.t_s = TIME_MAX_S - 0.5 * dt;
        late.peak_time_s = late.t_s - 2.0;
        late.reactivity_load_dollar = 2.0;
        late.rods_dropped = true;
        late.scram_time_s = late.t_s - 0.5;
        refuse(late, dropped(2.0), dt, StepReject::TimeBeyondCard);
        let mut powerful = fresh;
        powerful.power_mw = POWER_MAX_MW * 2.0;
        powerful.peak_power_mw = POWER_MAX_MW * 2.0;
        refuse(powerful, load(2.0), dt, StepReject::PowerBeyondCard);
        let mut spent = fresh;
        spent.energy_mws = ENERGY_MAX_MWS;
        spent.power_mw = 100.0;
        spent.peak_power_mw = 100.0;
        refuse(spent, load(2.0), dt, StepReject::EnergyBeyondCard);
        let mut hot = fresh;
        hot.fuel_temperature_k = FUEL_TEMPERATURE_MAX_K - 1.0e-4;
        hot.peak_fuel_temperature_k = hot.fuel_temperature_k;
        hot.power_mw = 900.0;
        hot.peak_power_mw = 900.0;
        refuse(hot, load(2.0), dt, StepReject::FuelTemperatureBeyondCard);
    }

    #[test]
    fn a_handed_in_state_that_no_run_could_have_made_is_refused_by_name() {
        let (reactor, _) = start(25.0);
        let dt = 5.0e-5;
        let mut state = start(25.0).1;
        let mut frame = marked_frame();
        for _ in 0..8000 {
            step(&reactor, &mut state, &mut frame, load(2.0), dt).unwrap();
        }
        assert!(state.peak_power_mw > 10.0 && state.energy_mws > 1.0);
        let refuse = |forged: ReactorState, expected: StepReject| {
            let mut s = forged;
            let mut f = marked_frame();
            let bytes = f.to_bytes();
            assert_eq!(step(&reactor, &mut s, &mut f, load(2.0), dt), Err(expected));
            assert_eq!(s, forged);
            assert_eq!(f.to_bytes(), bytes);
        };
        let mut forged = state;
        forged.energy_mws = -1.0e-9;
        refuse(forged, StepReject::NegativeEnergy);
        forged = state;
        forged.energy_mws = -5.0;
        forged.peak_energy_mws = -6.0;
        refuse(forged, StepReject::NegativeEnergy);
        forged = state;
        forged.peak_power_mw = 0.5 * state.power_mw;
        refuse(forged, StepReject::ImpossiblePeak);
        forged = state;
        forged.peak_time_s = state.t_s + 1.0;
        refuse(forged, StepReject::ImpossiblePeak);
        forged = state;
        forged.peak_time_s = -0.1;
        refuse(forged, StepReject::ImpossiblePeak);
        forged = state;
        forged.peak_energy_mws = state.energy_mws * 2.0;
        refuse(forged, StepReject::ImpossiblePeak);
        forged = state;
        forged.peak_energy_mws = -1.0;
        refuse(forged, StepReject::ImpossiblePeak);
        forged = state;
        forged.peak_fuel_temperature_k = state.fuel_temperature_k - 1.0;
        refuse(forged, StepReject::ImpossiblePeak);
        forged = state;
        forged.power_mw = 0.0;
        refuse(forged, StepReject::StateOutsideCard);
        forged = state;
        forged.power_mw = -1.0;
        refuse(forged, StepReject::StateOutsideCard);
        forged = state;
        forged.precursor_mw[2] = -1.0;
        refuse(forged, StepReject::StateOutsideCard);
        forged = state;
        forged.t_s = -1.0;
        refuse(forged, StepReject::StateOutsideCard);
        forged = state;
        forged.fuel_temperature_k = state.start_temperature_k - 1.0;
        refuse(forged, StepReject::StateOutsideCard);
        forged = state;
        forged.scram_time_s = state.t_s + 1.0;
        refuse(forged, StepReject::StateOutsideCard);
        forged = state;
        forged.energy_at_scram_mws = state.energy_mws + 1.0;
        refuse(forged, StepReject::StateOutsideCard);
        forged = state;
        forged.energy_at_scram_mws = -1.0;
        refuse(forged, StepReject::StateOutsideCard);
        // the honest state still steps
        let mut honest = state;
        assert_eq!(step(&reactor, &mut honest, &mut frame, load(2.0), dt), Ok(()));
    }

    #[test]
    fn the_reactivity_cannot_change_after_the_start_and_the_scram_goes_one_way() {
        let (reactor, mut state) = start(25.0);
        let mut frame = marked_frame();
        let dt = 5.0e-5;
        let rho = 2.0;
        // the scram is refused while the rod is moving, from the first step on
        let before = state;
        assert_eq!(step(&reactor, &mut state, &mut frame, dropped(rho), dt), Err(StepReject::ScramBeforeRodStopped));
        assert_eq!(state, before);
        for _ in 0..200 {
            step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
        }
        assert!(state.rod_moving);
        assert_eq!(state.reactivity_load_dollar, rho);
        let before = state;
        // a larger insertion while the rod moves, another, and one a single bit away
        for other in [2.5, 3.0, f64::from_bits(rho.to_bits() + 1)] {
            assert_eq!(step(&reactor, &mut state, &mut frame, load(other), dt), Err(StepReject::ReactivityChangedDuringRun));
            assert_eq!(state, before);
        }
        // a smaller one is below the card's floor, and is named as that
        assert_eq!(step(&reactor, &mut state, &mut frame, load(1.5), dt), Err(StepReject::ReactivityOutsideCard));
        assert_eq!(state, before);
        assert_eq!(step(&reactor, &mut state, &mut frame, dropped(rho), dt), Err(StepReject::ScramBeforeRodStopped));
        assert_eq!(state, before);
        while state.t_s < 1.0 {
            step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
        }
        assert!(!state.rod_moving);
        // the rod has stopped, and the scram is still earlier than any compared
        let before = state;
        assert_eq!(step(&reactor, &mut state, &mut frame, dropped(rho), dt), Err(StepReject::ScramBeforeCompared));
        assert_eq!(state, before);
        while state.t_s - state.peak_time_s < SCRAM_AFTER_PEAK_MIN_S {
            step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
        }
        let before = state;
        // after the rod has stopped: a second insertion, or a withdrawal, is not in the model
        for other in [3.0, 2.5, 2.0000001] {
            assert_eq!(step(&reactor, &mut state, &mut frame, load(other), dt), Err(StepReject::ReactivityChangedDuringRun));
            assert_eq!(step(&reactor, &mut state, &mut frame, dropped(other), dt), Err(StepReject::ReactivityChangedDuringRun));
            assert_eq!(state, before);
        }
        let power_before = state.power_mw;
        assert_eq!(state.energy_at_scram_mws, 0.0);
        step(&reactor, &mut state, &mut frame, dropped(rho), dt).unwrap();
        assert!(state.rods_dropped);
        assert_eq!(state.scram_time_s, before.t_s);
        // the energy at the scram is the energy at the instant the rods dropped, not a step later
        assert_eq!(state.energy_at_scram_mws, before.energy_mws);
        assert!(state.energy_mws > state.energy_at_scram_mws);
        assert!(frame.flags & FLAG_RODS_DROPPED != 0);
        for _ in 0..400 {
            step(&reactor, &mut state, &mut frame, dropped(rho), dt).unwrap();
        }
        assert_eq!(state.energy_at_scram_mws, before.energy_mws);
        // the prompt jump: the power falls to about a ninth and more within 20 ms
        assert!(state.power_mw < 0.2 * power_before, "{} {}", state.power_mw, power_before);
        let before = state;
        assert_eq!(step(&reactor, &mut state, &mut frame, load(rho), dt), Err(StepReject::RodsRaisedAfterScram));
        assert_eq!(state, before);
    }

    #[test]
    fn the_reactivity_goes_in_at_the_rods_rate_and_then_holds_at_its_tables_worth() {
        for (table, core) in ROD_TABLE_CORE.into_iter().enumerate() {
            let (low, high) = START_TEMPERATURE_K_BY_TABLE[table];
            let (reactor, mut state) = start_on(0.5 * (low + high), core);
            let mut frame = marked_frame();
            let rho = 2.6;
            let dt = 5.0e-5;
            let stop = reactor.rod_stop_s(rho);
            assert!(within(stop, 0.05, 1.0e-12));
            let worth = DOLLAR_SCALE[table] * rho - DOLLAR_OFFSET[table];
            assert!(within(reactor.worth_dollar(table, rho), worth, 1.0e-15));
            let mut landed_past_stop = false;
            while state.t_s < 0.06 {
                step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
                if state.t_s < stop {
                    assert!(within(state.reactivity_inserted_now_dollar, RAMP_DOLLAR_PER_S * state.t_s, 1.0e-12));
                    assert!(state.rod_moving);
                    assert!(frame.flags & FLAG_ROD_MOVING != 0);
                    // nothing has heated: the net reactivity is the table's worth of the part inserted
                    assert!(within(state.reactivity_net_dollar, worth * state.reactivity_inserted_now_dollar / rho, 1.0e-6));
                } else if state.t_s > stop + dt && !landed_past_stop {
                    landed_past_stop = true;
                    assert_eq!(state.reactivity_inserted_now_dollar, rho);
                    assert!(!state.rod_moving);
                    assert!(frame.flags & FLAG_ROD_MOVING == 0);
                    assert!(within(state.reactivity_net_dollar, worth, 1.0e-4));
                }
            }
            assert!(landed_past_stop);
            assert!(state.prompt_supercritical && frame.flags & FLAG_PROMPT_SUPERCRITICAL != 0);
        }
        // a step that straddles the stop lands where two steps meeting at the stop land
        let (reactor, _) = start(25.0);
        let mut frame = marked_frame();
        let rho = 2.6;
        let stop = reactor.rod_stop_s(rho);
        let mut fine = start(25.0).1;
        let mut coarse;
        let h = 1.0e-5;
        while fine.t_s < stop - 2.5 * h {
            step(&reactor, &mut fine, &mut frame, load(rho), h).unwrap();
        }
        coarse = fine;
        step(&reactor, &mut coarse, &mut frame, load(rho), 5.0 * h).unwrap();
        let first = stop - fine.t_s;
        step(&reactor, &mut fine, &mut frame, load(rho), first).unwrap();
        step(&reactor, &mut fine, &mut frame, load(rho), 5.0 * h - first).unwrap();
        assert!(within(coarse.power_mw, fine.power_mw, 1.0e-9), "{} {}", coarse.power_mw, fine.power_mw);
        // the same printed dollars are three different pulses on the three tables; no start was
        // compared on all three, so 24.00 degC on 215 and 217 and 22.81 degC on 231
        let peaks: std::vec::Vec<f64> = ROD_TABLE_CORE
            .into_iter()
            .zip([T215.0, T215.0, T231.1])
            .map(|(core, kelvin)| run_on(rk4, kelvin, core, 2.5, ceiling(), 100.0, 0.5).last.peak_power_mw)
            .collect();
        println!("2.5 printed dollars on the tables of 215, 217, 231: {peaks:?} MW");
        // the card's power is before each session's channel factor; core 217's table reads lowest
        assert!(peaks[1] < peaks[0] && peaks[1] < peaks[2]);
        assert!(peaks[0] > 1.08 * peaks[1] && peaks[2] > 1.08 * peaks[1]);
    }

    #[test]
    fn heat_goes_where_the_equations_put_it() {
        // with the heat flow to water counted, the energy released equals the fuel's enthalpy
        // rise plus the heat that left
        let (reactor, mut state) = start(25.0);
        let mut frame = marked_frame();
        let dt = 5.0e-5;
        let mut left_j = 0.0;
        let mut previous = state;
        while state.t_s < 2.0 {
            step(&reactor, &mut state, &mut frame, load(2.0), dt).unwrap();
            left_j += 0.5
                * dt
                * super::HEAT_FLOW_W_PER_K
                * ((previous.fuel_temperature_k - state.start_temperature_k)
                    + (state.fuel_temperature_k - state.start_temperature_k));
            previous = state;
        }
        let enthalpy = |t: f64| {
            let x = t - super::HEAT_CAPACITY_REFERENCE_K;
            FUEL_MASS_KG * (super::HEAT_CAPACITY_BASE_J_PER_KG_K * x + 0.5 * super::HEAT_CAPACITY_SLOPE_J_PER_KG_K2 * x * x)
        };
        let stored = enthalpy(state.fuel_temperature_k) - enthalpy(state.start_temperature_k);
        assert!(within(stored + left_j, state.energy_mws * 1.0e6, 1.0e-6), "{} {} {}", stored, left_j, state.energy_mws);
        // the heat that leaves in two seconds is a few hundredths of the energy
        assert!(left_j > 0.01 * state.energy_mws * 1.0e6 && left_j < 0.05 * state.energy_mws * 1.0e6);
        // the highest fuel temperature is kept after the fuel has begun to cool
        let (_, mut s) = start(25.0);
        while s.t_s < 6.0 {
            let l = if s.t_s >= 3.0 { dropped(2.0) } else { load(2.0) };
            step(&reactor, &mut s, &mut frame, l, ceiling()).unwrap();
            assert!(s.peak_fuel_temperature_k >= s.fuel_temperature_k);
        }
        assert!(s.peak_fuel_temperature_k > s.fuel_temperature_k);
        assert!(s.fuel_temperature_k - s.start_temperature_k > 150.0);
        // the feedback and its slope agree
        for t in [294.0, 300.0, 350.0, 500.0, 599.0] {
            let d = 1.0e-3;
            let numeric = (feedback(&reactor, t + d) - feedback(&reactor, t - d)) / (2.0 * d);
            assert!(within(feedback_slope(&reactor, t), numeric, 1.0e-6));
        }
        assert_eq!(feedback(&reactor, 294.0), 0.0);
        // 6.2 pcm per K at 294 K, 8.9 at 600 K
        assert!(within(feedback_slope(&reactor, 294.0), 6.241e-5, 1.0e-12));
        assert!(within(feedback_slope(&reactor, 600.0), 8.909e-5, 1.0e-3));
    }

    /// The corners of the span: starting temperature in K, table, printed dollars. Each start
    /// is at an end of its own table's span.
    const SPAN: [(f64, f64, f64); 8] = [
        (T215.0, 215.0, 2.0),
        (T215.1, 215.0, 3.0),
        (T217.0, 217.0, 3.0),
        (298.15, 217.0, 2.0),
        (T217.1, 217.0, 2.5),
        (T231.0, 231.0, 3.0),
        (T231.1, 231.0, 2.0),
        (T231.1, 231.0, 3.0),
    ];

    /// The scram time used at the ends of the span: 7.3 s after a weak pulse, 2.5 s after a strong one.
    fn span_scram(rho: f64) -> f64 {
        if rho < 2.2 { 7.3 } else { 2.5 }
    }

    #[test]
    fn the_ledger_residual_stays_under_the_limit_over_full_runs_at_the_ceiling() {
        let mut largest = 0.0;
        for (kelvin, core, rho) in SPAN {
            let run = run_on(rk4, kelvin, core, rho, ceiling(), span_scram(rho), TIME_MAX_S);
            assert_eq!(run.end, Ok(()), "{kelvin} {core} {rho}");
            println!(
                "{kelvin} K, table {core}, {rho} dollar: {} steps, peak {} MW at {} s, energy {} MW s ({} at the scram), fuel {} K, highest {} K, largest residual {:e}",
                run.steps, run.last.peak_power_mw, run.last.peak_time_s, run.last.energy_mws,
                run.last.energy_at_scram_mws, run.last.fuel_temperature_k,
                run.last.peak_fuel_temperature_k, run.largest_residual
            );
            if run.largest_residual > largest {
                largest = run.largest_residual;
            }
            assert!(run.last.peak_power_mw < POWER_MAX_MW && run.last.energy_mws < ENERGY_MAX_MWS);
            assert!(run.last.peak_fuel_temperature_k < FUEL_TEMPERATURE_MAX_K);
            assert!(run.last.rods_dropped);
            assert!(run.last.t_s > TIME_MAX_S - ceiling());
        }
        println!("largest residual over the span at the ceiling: {largest:e}");
        assert!(largest < RESIDUAL_CONSERVATION_LIMIT / 10.0);
    }

    /// A run whose steps make the card's last second lands its last step, at four step sizes;
    /// one step more is refused by name.
    #[test]
    fn a_run_that_ends_exactly_at_the_end_of_the_card_keeps_its_last_step() {
        let (reactor, _) = start(25.0);
        for (dt_s, steps) in [(5.0e-5, 280_000u64), (8.75e-5, 160_000), (2.0e-5, 700_000), (7.0e-5, 200_000)] {
            let mut state = start(25.0).1;
            let mut frame = marked_frame();
            for n in 0..steps {
                let l = if state.t_s >= 3.0 { dropped(2.0) } else { load(2.0) };
                assert_eq!(step(&reactor, &mut state, &mut frame, l, dt_s), Ok(()), "step {n} of {steps} at {dt_s}");
            }
            assert!((state.t_s - TIME_MAX_S).abs() < super::TIME_END_ALLOWANCE_S, "{}", state.t_s);
            let before = state;
            assert_eq!(step(&reactor, &mut state, &mut frame, dropped(2.0), dt_s), Err(StepReject::TimeBeyondCard));
            assert_eq!(state, before);
        }
    }

    /// Past what was compared with a record the card refuses and does not answer: a 3.0 dollar
    /// pulse left without a scram is refused, as the energy or the fuel temperature passes the
    /// largest compared or as the rods are held past the latest scram compared, whichever is first.
    #[test]
    fn a_strong_pulse_left_without_a_scram_is_refused_beyond_what_was_compared() {
        // at each end of each table's own span, at the ceiling: the refusal and the time of the
        // last landed step after the fire signal. On cores 215 and 231 the fuel temperature
        // passes 600 K after 5.2 to 6.1 s and 5.9 to 6.4 s; on core 217 the rods are held past
        // 7.45 s after the peak, at 7.56 s, before either limit.
        let expected = [
            [(StepReject::FuelTemperatureBeyondCard, 6.105), (StepReject::FuelTemperatureBeyondCard, 5.198)],
            [(StepReject::RodsHeldBeyondCompared, 7.558), (StepReject::RodsHeldBeyondCompared, 7.558)],
            [(StepReject::FuelTemperatureBeyondCard, 6.428), (StepReject::FuelTemperatureBeyondCard, 5.890)],
        ];
        for (table, core) in ROD_TABLE_CORE.into_iter().enumerate() {
            let (low, high) = START_TEMPERATURE_K_BY_TABLE[table];
            for (end, kelvin) in [low, high].into_iter().enumerate() {
                let run = run_on(rk4, kelvin, core, 3.0, ceiling(), 100.0, TIME_MAX_S);
                println!("{kelvin} K, table {core}: refused {:?} at {} s, {} s after the peak, energy {} MW s, fuel {} K", run.end, run.last.t_s, run.last.t_s - run.last.peak_time_s, run.last.energy_mws, run.last.fuel_temperature_k);
                let (reject, t_s) = expected[table][end];
                assert_eq!(run.end, Err(reject), "{kelvin} K, table {core}");
                assert!((run.last.t_s - t_s).abs() < 1.0e-3, "{kelvin} K, table {core}: {}", run.last.t_s);
                assert!(run.last.t_s > 4.0 && run.last.t_s - run.last.peak_time_s <= SCRAM_AFTER_PEAK_MAX_S + 1.0e-9);
                assert!(run.last.energy_mws <= ENERGY_MAX_MWS && run.last.fuel_temperature_k <= FUEL_TEMPERATURE_MAX_K);
            }
        }
        // no pulse in range reaches the power limit
        for (table, core) in ROD_TABLE_CORE.into_iter().enumerate() {
            let (low, high) = START_TEMPERATURE_K_BY_TABLE[table];
            for kelvin in [low, high] {
                let run = run_on(rk4, kelvin, core, 3.0, ceiling(), 100.0, 0.4);
                assert_eq!(run.end, Ok(()));
                assert!(run.last.peak_power_mw < 0.95 * POWER_MAX_MW, "{}", run.last.peak_power_mw);
            }
        }
    }

    /// The scram is accepted only inside the envelope compared with the records, 1.50 to 7.45 s
    /// after the peak, and both ends land exactly: at 5e-5 s a scram 30,000 steps after the peak
    /// lands and one 29,999 steps after is refused; a held step ending 149,000 steps after the
    /// peak lands and the next held step is refused, while a scram there lands. A scram before
    /// the pulse, while the power still rises, is refused as earlier than any compared.
    #[test]
    fn the_scram_is_held_to_the_envelope_compared_with_the_records() {
        assert_eq!((SCRAM_AFTER_PEAK_MIN_S, SCRAM_AFTER_PEAK_MAX_S), (1.50, 7.45));
        // the scram after the card's own peak in the 27 compared runs: 1.5098 to 7.4456 s
        assert!(SCRAM_AFTER_PEAK_MIN_S <= 1.5098 && 1.5098 - SCRAM_AFTER_PEAK_MIN_S < 0.01);
        assert!(SCRAM_AFTER_PEAK_MAX_S >= 7.4456 && SCRAM_AFTER_PEAK_MAX_S - 7.4456 < 0.01);
        let dt = 5.0e-5;
        for (kelvin, core, rho) in SPAN {
            let (reactor, mut state) = start_on(kelvin, core);
            let mut frame = marked_frame();
            // after the rod has stopped and while the power still rises: the peak is now
            while state.t_s < reactor.rod_stop_s(rho) + 4.0 * dt {
                step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
            }
            assert_eq!(state.peak_time_s, state.t_s);
            let before = state;
            assert_eq!(step(&reactor, &mut state, &mut frame, dropped(rho), dt), Err(StepReject::ScramBeforeCompared));
            assert_eq!(state, before);
            while state.t_s < 1.0 {
                step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
            }
            let peak = state.peak_time_s;
            let since = |s: &ReactorState| ((s.t_s - s.peak_time_s) / dt).round() as u64;
            while since(&state) < 29_999 {
                step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
            }
            let before = state;
            let mut f = marked_frame();
            let bytes = f.to_bytes();
            assert_eq!(step(&reactor, &mut state, &mut f, dropped(rho), dt), Err(StepReject::ScramBeforeCompared));
            assert_eq!(state, before);
            assert_eq!(f.to_bytes(), bytes);
            step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
            assert_eq!(since(&state), 30_000);
            let mut scram = state;
            assert_eq!(step(&reactor, &mut scram, &mut frame, dropped(rho), dt), Ok(()), "{kelvin} {core} {rho}");
            assert_eq!(scram.scram_time_s, state.t_s);
            if rho > 2.5 {
                // held that long, 3.0 dollars passes the largest fuel temperature or energy compared
                continue;
            }
            while since(&state) < 148_999 {
                step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
            }
            assert_eq!(state.peak_time_s, peak);
            assert_eq!(step(&reactor, &mut state, &mut frame, load(rho), dt), Ok(()), "{kelvin} {core} {rho}");
            assert_eq!(since(&state), 149_000);
            let before = state;
            let mut f = marked_frame();
            let bytes = f.to_bytes();
            assert_eq!(step(&reactor, &mut state, &mut f, load(rho), dt), Err(StepReject::RodsHeldBeyondCompared));
            assert_eq!(state, before);
            assert_eq!(f.to_bytes(), bytes);
            assert_eq!(step(&reactor, &mut state, &mut frame, dropped(rho), dt), Ok(()));
            assert!(state.rods_dropped);
        }
    }

    /// The second scoring's finding (V-37, range_edge2): a scram after the rod had stopped was
    /// refused as SpontaneousGeneration when it fell near the peak. Those scrams are now
    /// refused as earlier than any compared. Every scram the card accepts, from 1.50 to 7.45 s
    /// after the peak, lands at every step from the floor to the ceiling and keeps its ledger.
    #[test]
    fn a_scram_inside_the_compared_envelope_lands_at_every_step_from_floor_to_ceiling() {
        let reactor = ReactorTrigaJsi::core_56_pulse_mode();
        let mut largest: f64 = 0.0;
        let mut probes = 0;
        let afters = [SCRAM_AFTER_PEAK_MIN_S, 1.6, 2.0, 3.0, 4.5, 6.0, 7.0];
        for (kelvin, core, rho) in SPAN {
            for dt_s in [ceiling(), 5.0e-5, 2.0e-5, 3.1e-6] {
                // the finest step only over the first scrams, for time
                let last = if dt_s < 1.0e-5 { 2.0 } else { SCRAM_AFTER_PEAK_MAX_S };
                let (_, mut state) = start_on(kelvin, core);
                let mut frame = marked_frame();
                let mut wanted = afters.iter().copied().filter(|a| *a < last - dt_s).chain(core::iter::once(last - dt_s)).peekable();
                while state.t_s < 1.0 || wanted.peek().is_some() {
                    if state.t_s >= 1.0 {
                        let after = *wanted.peek().unwrap();
                        if state.t_s - state.peak_time_s >= after {
                            wanted.next();
                            // the scram and 0.05 s after it
                            let mut scram = state;
                            let mut f = marked_frame();
                            let mut n = 0;
                            while n < (0.05 / dt_s) as u64 {
                                assert_eq!(step(&reactor, &mut scram, &mut f, dropped(rho), dt_s), Ok(()), "{kelvin} K {core} {rho} dollar, {after} s after the peak, step {dt_s}");
                                largest = largest.max(scram.residual_conservation_error);
                                n += 1;
                            }
                            assert!(scram.scram_time_s - scram.peak_time_s >= after && scram.scram_time_s - scram.peak_time_s < after + dt_s);
                            probes += 1;
                            if dt_s == ceiling() {
                                // the floor: the scram and 300 steps at the floor, then on at the ceiling
                                let mut floor = state;
                                for _ in 0..300 {
                                    assert_eq!(step(&reactor, &mut floor, &mut f, dropped(rho), super::FLOOR_DT_S), Ok(()));
                                }
                                for _ in 0..400 {
                                    assert_eq!(step(&reactor, &mut floor, &mut f, dropped(rho), ceiling()), Ok(()));
                                    largest = largest.max(floor.residual_conservation_error);
                                }
                                probes += 1;
                            }
                            continue;
                        }
                    }
                    match step(&reactor, &mut state, &mut frame, load(rho), dt_s) {
                        Ok(()) => largest = largest.max(state.residual_conservation_error),
                        // held, 3.0 dollars passes the largest fuel temperature or energy compared
                        Err(StepReject::FuelTemperatureBeyondCard | StepReject::EnergyBeyondCard) if rho > 2.5 => break,
                        Err(other) => std::panic!("{kelvin} K {core} {rho}: {other:?}"),
                    }
                }
            }
        }
        println!("{probes} scrams, largest residual {largest:e}");
        assert!(probes >= 150, "{probes}");
        assert!(largest < RESIDUAL_CONSERVATION_LIMIT / 10.0);
        // the pieces: none before the scram, eight at the ceiling in the fall, one after it
        let (_, mut state) = start(25.0);
        let mut frame = marked_frame();
        assert_eq!(scram_pieces(&reactor, &state, load(2.0), ceiling()), 1);
        while state.t_s - state.peak_time_s < SCRAM_AFTER_PEAK_MIN_S || state.t_s < 1.0 {
            step(&reactor, &mut state, &mut frame, load(2.0), ceiling()).unwrap();
        }
        let scram_s = state.t_s;
        assert_eq!(scram_pieces(&reactor, &state, dropped(2.0), ceiling()), 8);
        assert_eq!(scram_pieces(&reactor, &state, dropped(2.0), 2.0e-5), 2);
        assert_eq!(scram_pieces(&reactor, &state, dropped(2.0), super::FLOOR_DT_S), 1);
        assert_eq!(scram_pieces(&reactor, &state, dropped(2.0), 1.0), super::SCRAM_PIECES_MAX);
        while state.t_s < scram_s + 0.02 {
            step(&reactor, &mut state, &mut frame, dropped(2.0), ceiling()).unwrap();
        }
        assert_eq!(scram_pieces(&reactor, &state, dropped(2.0), ceiling()), 1);
    }

    #[test]
    fn explicit_euler_at_the_ceiling_is_refused_by_the_ledger() {
        for (kelvin, core, rho) in SPAN {
            let run = run_on(euler, kelvin, core, rho, ceiling(), 6.0, 11.9);
            assert!(
                run.end == Err(StepReject::SpontaneousGeneration)
                    || run.end == Err(StepReject::FirstLawBreach),
                "{kelvin} {rho}: {:?} after {} steps",
                run.end,
                run.steps
            );
            // named before the pulse: the power is still under a thousandth of the card's limit
            assert!(run.last.power_mw < 1.0e-3 * POWER_MAX_MW, "{}", run.last.power_mw);
        }
    }

    #[test]
    fn wrong_stage_weights_and_a_falsified_ledger_are_refused() {
        for (kelvin, core, rho) in SPAN {
            let run = run_on(wrong_weights, kelvin, core, rho, ceiling(), 6.0, 11.9);
            assert!(
                run.end == Err(StepReject::SpontaneousGeneration)
                    || run.end == Err(StepReject::FirstLawBreach),
                "{kelvin} {rho}: {:?}",
                run.end
            );
            assert!(run.last.power_mw < 1.0e-3 * POWER_MAX_MW);
        }
        // a ledger falsified in either direction is named in that direction
        let (reactor, _) = start(25.0);
        let mut state = start(25.0).1;
        let mut frame = marked_frame();
        for _ in 0..3500 {
            step(&reactor, &mut state, &mut frame, load(2.0), ceiling()).unwrap();
        }
        let population: f64 = state.power_mw + state.precursor_mw.iter().sum::<f64>();
        for (bias, expected) in [
            (-10.0 * RESIDUAL_CONSERVATION_LIMIT, StepReject::SpontaneousGeneration),
            (10.0 * RESIDUAL_CONSERVATION_LIMIT, StepReject::FirstLawBreach),
        ] {
            let mut forged = state;
            forged.population_supplied_mw += bias * population;
            let before = forged;
            let mut f = marked_frame();
            let bytes = f.to_bytes();
            assert_eq!(step(&reactor, &mut forged, &mut f, load(2.0), ceiling()), Err(expected));
            assert_eq!(forged, before);
            assert_eq!(f.to_bytes(), bytes);
        }
        // a precursor sum raised by hand is created population
        let mut forged = state;
        forged.precursor_mw[0] *= 1.0 + 1.0e-4;
        assert_eq!(
            step(&reactor, &mut forged, &mut frame, load(2.0), ceiling()),
            Err(StepReject::SpontaneousGeneration)
        );
        // an integral the stepping method reports wrongly is named too
        let mut lied = state;
        let table = validate(&reactor, &lied, load(2.0), ceiling()).unwrap();
        let t = terms(&reactor, &lied, load(2.0), table);
        let honest = through_the_stop(rk4, &reactor, &t, lied.t_s, &state_vector(&lied), ceiling(), 1);
        let wrong = Advanced { y: honest.y, supplied_mw: honest.supplied_mw + 1.0e-4 * population };
        assert_eq!(
            commit(&reactor, &mut lied, &mut frame, &t, load(2.0), ceiling(), wrong),
            Err(StepReject::FirstLawBreach)
        );
        assert_eq!(lied, state);
    }

    #[test]
    fn a_dropped_feedback_runs_away_and_is_refused() {
        for (kelvin, core, rho) in SPAN {
            let run = run_on(no_feedback, kelvin, core, rho, ceiling(), 6.0, 11.9);
            assert!(
                run.end == Err(StepReject::SpontaneousGeneration)
                    || run.end == Err(StepReject::FirstLawBreach)
                    || run.end == Err(StepReject::PowerBeyondCard),
                "{kelvin} {rho}: {:?}",
                run.end
            );
            assert!(run.last.t_s < 2.0);
        }
    }

    #[test]
    fn the_pulse_at_the_ceiling_is_the_pulse_at_a_sixteenth_of_it() {
        for (kelvin, core, rho) in [(298.15, 217.0, 2.0), (T231.0, 231.0, 3.0), (T215.1, 215.0, 2.0)] {
            let scram = span_scram(rho);
            let coarse = run_on(rk4, kelvin, core, rho, ceiling(), scram, 9.0);
            let fine = run_on(rk4, kelvin, core, rho, ceiling() / 16.0, scram, 9.0);
            assert_eq!(coarse.end, Ok(()));
            assert_eq!(fine.end, Ok(()));
            println!(
                "{kelvin} K {rho} dollar: peak {} against {} MW, highest fuel temperature {} against {} K, energy {} against {}",
                coarse.last.peak_power_mw, fine.last.peak_power_mw,
                coarse.last.peak_fuel_temperature_k, fine.last.peak_fuel_temperature_k,
                coarse.last.energy_mws, fine.last.energy_mws
            );
            // the peak is the highest landed sample, so it is under the true maximum by up to the
            // curvature over half a step
            assert!(within(coarse.last.peak_power_mw, fine.last.peak_power_mw, 1.0e-4));
            assert!(within(coarse.last.peak_fuel_temperature_k, fine.last.peak_fuel_temperature_k, 1.0e-5));
            // the scram lands on a step: the coarse run drops the rods up to one step later
            assert!(within(coarse.last.energy_mws, fine.last.energy_mws, 2.0e-4));
        }
    }

    #[test]
    fn the_flags_are_only_flags_and_the_proof_is_not_the_cards() {
        fn is_copy<T: Copy>() {}
        is_copy::<ReactorTrigaJsi>();
        is_copy::<ReactorState>();
        is_copy::<ReactorLoad>();
        let (reactor, mut state) = start(25.0);
        let mut frame = marked_frame();
        let all = FLAG_PROMPT_SUPERCRITICAL | FLAG_ROD_MOVING | FLAG_RODS_DROPPED;
        while state.t_s < 3.0 {
            let l = if state.t_s >= 2.0 { dropped(2.2) } else { load(2.2) };
            step(&reactor, &mut state, &mut frame, l, 5.0e-5).unwrap();
            assert_eq!(frame.flags & !all, 0);
            assert_eq!(frame.proof, [0xA5; 16]);
            assert_eq!(frame.t, state.t_s);
        }
        assert_eq!(frame.pos[0], state.power_mw as f32);
        assert_eq!(frame.pos[1], state.energy_mws as f32);
        assert_eq!(frame.vel[0], state.fuel_temperature_k as f32);
        assert_eq!(frame.vel[1], state.peak_fuel_temperature_k as f32);
        assert_eq!(frame.vel[2], state.peak_power_mw as f32);
    }
}
