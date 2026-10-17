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
//! the scram, applied from the start of the step that first carries it, at any time after
//! the fire signal; while the rod is still moving its ramp goes on under the scram.
//!
//! From the sources, not fitted: `β` and the six groups (Švajger et al., Nucl.
//! Eng. Technol. 56 (2024) 1165, Tables 10 and 12); `m`, 56 elements of 2.36 kg;
//! `c(T)` (the same article, eq. 7.2); `R`, the rod's whole worth of 4.16 dollars
//! (the institute's rod table of core 253, [`ROD_WORTH_DOLLAR`]) over its whole transit of
//! 80 ms (the article, section 7.1); the starting power, 10 W. `S`, 8 dollars, is an
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
//! The card leaves out any other core loading, a start from power, a second pulse on
//! warm fuel, the time the rods take to fall, and boiling. What it was compared on, and
//! how it did there:
//!
//! - Starts of 24.00 to 27.94 degC on core 215, 21.31 to 29.23 on core 217 and 20.90 to
//!   22.81 on core 231 ([`START_TEMPERATURE_K_BY_TABLE`], the envelope table by table of
//!   the 27 opened pulses compared at 2.0 printed dollars and above); the dependence on the
//!   starting temperature was not tested apart.
//! - 2.0 to 3.0 printed dollars (fitted 1.4 to 3.0). Below 2.0 dollars, on the four
//!   held-back rows of 28 March 2018, the card read 29, 21 and 9 percent low in peak power
//!   at 1.4, 1.6 and 1.8 dollars and level on the repeat at 1.8, where 0.03 printed dollars
//!   of the day's reference moves the peak power by 44, 21 and 13 percent; on the six rows
//!   of that day from 2.0 dollars up it was within 3.7 percent in peak power, 2.7 percent in
//!   energy within 50 ms of the peak and 5.8 K at the thermocouple. Nothing unfitted lies
//!   above 2.6 dollars.
//! - Scrams 1.51 to 7.45 s after the peak; an earlier scram was not compared, and no
//!   compared scram came while the transient rod was moving. The longest compared hold was
//!   7.45 s after the peak.
//! - Records to 14 s after the fire signal, energy released to 18.3 MW s, peak power to
//!   954 MW, core-average fuel temperature to 585 K, the fuel temperature compared only
//!   through one thermocouple of one element, through the hot-rod factor 1.387.
//!
//! Outside all of that the card computes the same equations with nothing compared, the
//! quadratic feedback carried past the 294 to about 585 K it was fitted on. It computes
//! from any start in liquid water at one standard atmosphere, 273.15 to 373.124 K, any
//! reactivity above zero up to the rod's whole worth of 4.16 printed dollars, a scram at
//! any time after the fire signal, the rods held as long as the caller holds them, and any
//! time. The highest core-average fuel temperature it reaches is 769.7 K (496.6 degC), at
//! 4.16 dollars on core 231 from 373.124 K with the rods held, 8.1 s after the fire signal;
//! the held pulse then settles towards the temperature where the feedback takes back the
//! whole insertion, about 637 K there. Through the hot-rod factor the hottest element
//! would be at 923 K (650 degC). Both are far below 1150 degC, the temperature limit of
//! U-ZrH1.6 fuel in stainless clad. The card still holds that limit: a core-average fuel
//! temperature above 1423.15 K, in a handed-in state or where a step would land, is refused
//! as [`StepReject::FuelTemperatureAboveFuelLimit`]. No run started at the card's own start
//! reaches it. The card carries no temperature of the hottest element, so the limit is held
//! on the core average, which the hottest element passes first.
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
//! It cannot show the physics matches the reactor. Past the compared span, with 3.25
//! printed dollars and more and the rods dropped in the rise of the pulse or at its peak,
//! the error a step at the static ceiling makes in the rise is carried by a population the
//! scram leaves tens of times smaller, and over a tail of more than 75 s the ledger can name
//! the run; over the 14 s compared the largest residual found there is 3.2e-8, and at half
//! the static ceiling every such run tried lands to 93.9 s.
//!
//! [`ReactorTrigaJsi::gate_dt_s`] is the static step ceiling: `GATE_PHASE_STEP` of the
//! time constant of the prompt pole after the scram, `Λ / (β (1 + S))`.
//! [`ReactorTrigaJsi::floor_dt_s`] is 1e-9 s. The public step applies neither.
//! The gateway refuses a step outside them. The step itself refuses a step longer than
//! [`STATE_PHASE_STEP`] of the time constant of the fastest pole at the state,
//! `Λ / (β |ρ − 1|)`, at its start or where it lands, as `DtAboveStateCeiling`. The feedback
//! after a pulse takes the net reactivity after the scram below −8 dollars: on the compared
//! runs to −9.08, where a step at the static ceiling advances 0.224 of that time constant, and
//! on any run the card computes to −9.87 (4.16 dollars on core 231 from 373.124 K, the rods
//! dropped at the highest fuel temperature, 8.1 s after the fire signal), where it advances
//! 0.241. No run started by the card meets the state ceiling at the static ceiling; a state
//! handed in hotter than any run reaches can. In the fall of the power after the scram a step
//! is taken in pieces ([`SCRAM_PIECE_PHASE_STEP`]), and in the rise of a pulse stronger than
//! any compared too ([`RISE_PIECE_PHASE_STEP`]).
//!
//! `core` only. The state is `Copy`. An accepted step writes time, power, energy
//! released, net reactivity, the fuel temperature and its highest landed value,
//! the highest landed power, the power rate and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

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
/// The transient rod's whole worth, printed dollars: 4.16, on the institute's rod table of
/// core 253. The largest reactivity the rod can insert; a load above it is refused as
/// [`StepReject::ReactivityAboveRodWorth`].
pub const ROD_WORTH_DOLLAR: f64 = 4.16;
/// Insertion rate `R`, printed dollars per s: the rod's whole worth over its whole transit of 80 ms.
pub const RAMP_DOLLAR_PER_S: f64 = ROD_WORTH_DOLLAR / 0.080;
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
/// Water freezes, K. The starting temperature is also the pool water's, so a start below it
/// is refused as [`StepReject::StartTemperatureNotLiquidWater`].
pub const WATER_FREEZING_K: f64 = 273.15;
/// Water boils at one standard atmosphere, K: the saturation temperature at 101.325 kPa of
/// IAPWS-IF97. The pool is open to the air, so a start above it is refused as
/// [`StepReject::StartTemperatureNotLiquidWater`].
pub const WATER_BOILING_K: f64 = 373.124;
/// Smallest inserted reactivity compared, printed dollars. Not applied by the step. The smallest
/// fitted is 1.4; below 2.0, on the four held-back rows of 28 March 2018, the card read 29, 21
/// and 9 percent low in peak power at 1.4, 1.6 and 1.8 dollars, where 0.03 printed dollars of
/// the day's reference moves the peak power by 44, 21 and 13 percent.
pub const REACTIVITY_COMPARED_MIN_DOLLAR: f64 = 2.0;
/// Largest inserted reactivity compared, printed dollars: the largest fitted. Not applied by the
/// step. Nothing unfitted lies above 2.6 dollars.
pub const REACTIVITY_COMPARED_MAX_DOLLAR: f64 = 3.0;
/// Lowest and highest starting fuel temperature compared on each table, K, in the order of
/// [`ROD_TABLE_CORE`]: the starts of the opened pulses compared at 2.0 printed dollars and above
/// on that table, the first thermocouple's reading at the fire signal, rounded outward to the
/// hundredth of a kelvin. Core 215: 24.0029 to 27.9375 degC (pulses 216 and 206); core 217:
/// 21.3171 to 29.2207 degC (222 and 240); core 231: 20.9023 to 22.8024 degC (331 and 340).
/// Every held-back row scored at 2.0 printed dollars and above started inside its own table's
/// span. Not applied by the step: the dependence on the starting temperature was not tested
/// apart, and the card computes from any start in liquid water.
pub const START_TEMPERATURE_K_BY_TABLE: [(f64, f64); 3] =
    [(297.15, 301.09), (294.46, 302.38), (294.05, 295.96)];
/// Earliest and latest scram compared, s after the time of the peak power. In the records of
/// the 27 opened pulses compared at 2.0 printed dollars and above the rods dropped 1.510 to
/// 7.446 s after the peak (pulses 219 and 330), and the validation dropped them that long
/// after the card's own peak; rounded outward to the hundredth of a second. Not applied by the
/// step: the rods may drop at any time after the fire signal, or never.
pub const SCRAM_AFTER_PEAK_COMPARED_S: (f64, f64) = (1.50, 7.45);
/// The highest core-average fuel temperature in a run compared with a record, K. Not applied by
/// the step.
pub const FUEL_TEMPERATURE_COMPARED_MAX_K: f64 = 585.0;
/// The temperature limit of U-ZrH1.6 fuel in stainless clad, 1150 degC, in kelvin: above it
/// the hydrogen the hydride gives off loads the cladding past what it holds (the safety limit
/// of TRIGA fuel with stainless cladding, NUREG-1537 and General Atomics' TRIGA fuel
/// qualification). Applied to the core-average fuel temperature the card carries; the card
/// carries no temperature of the hottest element.
pub const FUEL_TEMPERATURE_LIMIT_K: f64 = 1150.0 + 273.15;
/// The largest peak power compared, MW. Not applied by the step.
pub const POWER_COMPARED_MAX_MW: f64 = 954.0;
/// The largest energy released in a run compared with a record, MW s. Not applied by the step.
pub const ENERGY_COMPARED_MAX_MWS: f64 = 18.3;
/// How long after the fire signal the compared records run, s. Not applied by the step.
pub const TIME_COMPARED_MAX_S: f64 = 14.0;
/// `|N − N_anchor − ∫ dN| / max(N, N_anchor)` must stay at or under this. Set by
/// measurement over runs at the ends of the compared span, each at the ceiling through
/// the pulse and the scram to 14 s, and over scrams from the
/// rod's stop to two seconds after the peak, which 0.6.0 accepted: the largest
/// value found is 5.8e-9, and the limit is 34 times it. Over the scrams compared,
/// 1.50 to 7.45 s after the peak, at the ceiling, 5e-5, 2e-5 and
/// 3.1e-6 s and at the floor, the largest found is 3.5e-9. The residual compares
/// the step with the card's own quadrature of the same power, so it is a measure
/// of consistency and not a bound on the error.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 2.0e-7;
/// Fraction of the prompt time constant after the scram that one step at the
/// ceiling advances.
pub const GATE_PHASE_STEP: f64 = 0.2;
/// Fraction of the time constant of the fastest pole at the state, `Λ / (β |ρ − 1|)`, that one
/// step may advance, at the start of the step and where it lands; a longer step is refused as
/// [`StepReject::DtAboveStateCeiling`]. On the compared runs the net reactivity after the scram
/// reaches −9.08 dollars (`|ρ − 1|` = 10.08, 3.0 dollars on core 217 dropped 7.45 s after the
/// peak), where a step at the static ceiling advances 0.224 of that time constant; this
/// fraction is above it, so no compared run meets it.
pub const STATE_PHASE_STEP: f64 = 0.25;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-9;
/// For this many time constants of the prompt pole after the scram, counted from the instant
/// the rods are dropped, a step is taken in pieces: 14 ms.
pub const SCRAM_TRANSIENT_TIME_CONSTANTS: f64 = 30.0;
/// Fraction of that time constant one piece advances at most: an eighth of [`GATE_PHASE_STEP`].
pub const SCRAM_PIECE_PHASE_STEP: f64 = 0.025;
/// Fraction of the time constant of the growing pole, `Λ / (β (ρ − 1))`, one piece advances at
/// most in the rise of a pulse stronger than any compared ([`rise_pieces`]).
pub const RISE_PIECE_PHASE_STEP: f64 = 0.03;
/// Most pieces one step is taken in. A step at the ceiling in the fall after the scram takes eight.
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
    /// The recorded starting temperature, which is also the pool water's, is below 273.15 K, where water freezes, or above 373.124 K, where it boils at one standard atmosphere (IAPWS-IF97).
    StartTemperatureNotLiquidWater,
    /// The recorded rod-worth table is not that of core 215, 217 or 231.
    RodTableOutsideCard,
    /// The inserted reactivity is zero or less: no step in reactivity is inserted, and the inserted fraction min(1, R t / ρ_0) divides by it.
    ReactivityNotPositive,
    /// The inserted reactivity is above 4.16 printed dollars, the transient rod's whole worth on the institute's rod table of core 253: the request asks more than the rod holds.
    ReactivityAboveRodWorth,
    /// The scram value is neither 0 (rods held) nor 1 (rods dropped).
    ScramValueOutsideCard,
    /// The inserted reactivity differs from the one this run started with; the model defines no change of it but the scram.
    ReactivityChangedDuringRun,
    /// The rods were raised after the scram.
    RodsRaisedAfterScram,
    /// The energy released in the handed-in state is negative.
    NegativeEnergy,
    /// A peak field of the handed-in state cannot have come from a run: a peak power below the power, a peak time or peak energy outside the run so far, or a highest fuel temperature below the fuel temperature.
    ImpossiblePeak,
    /// The handed-in state is not physically possible: a power that is not positive, a negative precursor sum, a negative time, a fuel temperature below the starting temperature, or a scram time or scram energy outside the run so far.
    ImpossibleState,
    /// The core-average fuel temperature, in the handed-in state or where the step would land, is above 1423.15 K (1150 degC), the temperature limit of U-ZrH1.6 fuel in stainless clad.
    FuelTemperatureAboveFuelLimit,
    /// The step is longer than 0.25 of the time constant of the fastest pole, Λ / (β |ρ − 1|), at the state it starts from or where it lands.
    DtAboveStateCeiling,
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
    pub rod_worth_dollar: f64,
    pub water_freezing_k: f64,
    pub water_boiling_k: f64,
    pub fuel_temperature_limit_k: f64,
    pub residual_conservation_limit: f64,
    pub gate_phase_step: f64,
    pub state_phase_step: f64,
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
            rod_worth_dollar: ROD_WORTH_DOLLAR,
            water_freezing_k: WATER_FREEZING_K,
            water_boiling_k: WATER_BOILING_K,
            fuel_temperature_limit_k: FUEL_TEMPERATURE_LIMIT_K,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_phase_step: GATE_PHASE_STEP,
            state_phase_step: STATE_PHASE_STEP,
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
            {
                return false;
            }
            index += 1;
        }
        let pairs: [(f64, f64); 21] = [
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
            (self.rod_worth_dollar, ROD_WORTH_DOLLAR),
            (self.water_freezing_k, WATER_FREEZING_K),
            (self.water_boiling_k, WATER_BOILING_K),
            (self.fuel_temperature_limit_k, FUEL_TEMPERATURE_LIMIT_K),
            (self.residual_conservation_limit, RESIDUAL_CONSERVATION_LIMIT),
            (self.gate_phase_step, GATE_PHASE_STEP),
            (self.state_phase_step, STATE_PHASE_STEP),
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

    /// Largest step the card stands behind at any state: `GATE_PHASE_STEP · Λ / (β (1 + S))`,
    /// 0.094 ms. Not applied by [`step`]. It covers every state of the compared runs; past them
    /// the net reactivity after the scram can fall further below −8 dollars, and the step itself
    /// refuses a step longer than [`STATE_PHASE_STEP`] of the time constant at its state, as
    /// [`StepReject::DtAboveStateCeiling`] ([`Self::state_dt_s`]).
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

    /// Whether a starting temperature is liquid water at one standard atmosphere, 273.15 to
    /// 373.124 K, ends included.
    #[must_use]
    pub fn start_is_liquid_water(&self, start_temperature_k: f64) -> bool {
        start_temperature_k >= self.water_freezing_k && start_temperature_k <= self.water_boiling_k
    }

    /// Longest step the card takes from a net reactivity `ρ`, dollars:
    /// `STATE_PHASE_STEP · Λ / (β |ρ − 1|)`. Infinite at `ρ = 1`.
    #[must_use]
    pub fn state_dt_s(&self, reactivity_net_dollar: f64) -> f64 {
        self.state_phase_step * self.generation_time_s
            / (self.delayed_fraction * (reactivity_net_dollar - 1.0).abs())
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
    /// rod table is one of the three and the starting temperature is finite and liquid water at
    /// one standard atmosphere, 273.15 to 373.124 K. The starts compared on each table are
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
        if !reactor.start_is_liquid_water(start_temperature_k) {
            return None;
        }
        reactor.table_of(rod_table_core)?;
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
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    reactor: &ReactorTrigaJsi,
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
    reactor: &ReactorTrigaJsi,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    let table = validate(reactor, state, load, dt_s)?;
    let terms = terms(reactor, state, load, table);
    let y = state_vector(state);
    let rho = within_state_ceiling(reactor, &terms, state.t_s, &y, dt_s)?;
    let pieces = pieces(reactor, state, load, rho, dt_s);
    let advanced = through_the_stop(rk4, reactor, &terms, state.t_s, &y, dt_s, pieces);
    commit(reactor, state, frame, &terms, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut ReactorState) {
    state.t_s = normal_or_zero(state.t_s);
    state.power_mw = normal_or_zero(state.power_mw);
    normal_or_zero_each(&mut state.precursor_mw);
    state.fuel_temperature_k = normal_or_zero(state.fuel_temperature_k);
    state.energy_mws = normal_or_zero(state.energy_mws);
    state.start_temperature_k = normal_or_zero(state.start_temperature_k);
    state.rod_table_core = normal_or_zero(state.rod_table_core);
    state.reactivity_load_dollar = normal_or_zero(state.reactivity_load_dollar);
    state.scram_time_s = normal_or_zero(state.scram_time_s);
    state.energy_at_scram_mws = normal_or_zero(state.energy_at_scram_mws);
    state.power_rate_mw_per_s = normal_or_zero(state.power_rate_mw_per_s);
    state.reactivity_net_dollar = normal_or_zero(state.reactivity_net_dollar);
    state.reactivity_inserted_now_dollar = normal_or_zero(state.reactivity_inserted_now_dollar);
    state.population_anchor_mw = normal_or_zero(state.population_anchor_mw);
    state.population_supplied_mw = normal_or_zero(state.population_supplied_mw);
    state.population_defect_mw = normal_or_zero(state.population_defect_mw);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.peak_power_mw = normal_or_zero(state.peak_power_mw);
    state.peak_energy_mws = normal_or_zero(state.peak_energy_mws);
    state.peak_time_s = normal_or_zero(state.peak_time_s);
    state.peak_fuel_temperature_k = normal_or_zero(state.peak_fuel_temperature_k);
}

/// The state ceiling at `(t, y)` under the step's load: `dt_s` may be at most
/// [`STATE_PHASE_STEP`] of the time constant of the fastest pole there, `Λ / (β |ρ − 1|)`, with
/// the scram of the step counted in `ρ`. Inside the compared runs a step at the static ceiling
/// never meets it. Returns that net reactivity.
fn within_state_ceiling(
    reactor: &ReactorTrigaJsi,
    terms: &Terms,
    t_s: f64,
    y: &[f64; 9],
    dt_s: f64,
) -> Result<f64, StepReject> {
    let rho = net_reactivity(reactor, terms, t_s, y);
    if !rho.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s > reactor.state_dt_s(rho) {
        return Err(StepReject::DtAboveStateCeiling);
    }
    Ok(rho)
}

/// In how many equal pieces a step is taken: the more of [`scram_pieces`] and [`rise_pieces`].
fn pieces(reactor: &ReactorTrigaJsi, state: &ReactorState, load: ReactorLoad, rho: f64, dt_s: f64) -> u32 {
    let scram = scram_pieces(reactor, state, load, dt_s);
    let rise = rise_pieces(reactor, rho, dt_s);
    if rise > scram { rise } else { scram }
}

/// In how many equal pieces a step is taken in the rise of a pulse: so many that each advances
/// at most [`RISE_PIECE_PHASE_STEP`] of the time constant of the growing pole at the step's
/// start, `Λ / (β (ρ − 1))`. The error of a step there is a fixed part of the population; a
/// scram in the rise then leaves a population tens of times smaller, which carries that error
/// for the rest of the run. On a compared pulse the net reactivity is at most the whole
/// insertion's worth, 2.21 dollars (3.0 printed dollars on core 231), where a step at the
/// ceiling advances 0.027 of that time constant: one piece, the step of 0.9.0.
fn rise_pieces(reactor: &ReactorTrigaJsi, rho: f64, dt_s: f64) -> u32 {
    if !(rho > 1.0) {
        return 1;
    }
    let growth_per_s = (rho - 1.0) * reactor.delayed_fraction / reactor.generation_time_s;
    let wanted = dt_s * growth_per_s / RISE_PIECE_PHASE_STEP * (1.0 - 8.0 * f64::EPSILON);
    if !(wanted > 1.0) {
        return 1;
    }
    if wanted >= SCRAM_PIECES_MAX as f64 {
        return SCRAM_PIECES_MAX;
    }
    let whole = wanted as u32;
    if (whole as f64) < wanted { whole + 1 } else { whole }
}

/// In how many equal pieces the fall of the power after the scram asks a step to be taken. One,
/// except while the power falls after the scram: the rods take 8 dollars out at the start of a step, the power then falls with the
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

/// One step of `method`, taken in `pieces` equal pieces (one, except in the fall of the power
/// after the scram). The inserted reactivity has a corner where the rod stops: a piece that
/// holds the stop inside it is taken as two that meet at it, each with its own Hermite
/// integral on its own side of the stop. Before the scram, and after it once the rod has
/// stopped, this is the step of 0.9.0 to the bit.
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
        let h = dt_s / pieces as f64;
        let mut at = *y;
        let mut supplied_mw = 0.0;
        let mut piece = 0;
        while piece < pieces {
            let t = t_s + piece as f64 * h;
            let one = across_the_stop(method, reactor, terms, stop, t, &at, h);
            supplied_mw += one.supplied_mw;
            at = one.y;
            piece += 1;
        }
        return Advanced { y: at, supplied_mw };
    }
    across_the_stop(method, reactor, terms, stop, t_s, y, dt_s)
}

/// One smooth piece of `method` from `t_s` over `h`, or two that meet at the rod's stop when it
/// lies inside.
fn across_the_stop(
    method: fn(&ReactorTrigaJsi, &Terms, f64, &[f64; 9], f64) -> [f64; 9],
    reactor: &ReactorTrigaJsi,
    terms: &Terms,
    stop: f64,
    t_s: f64,
    y: &[f64; 9],
    h: f64,
) -> Advanced {
    let first = stop - t_s;
    if first > 0.0 && first < h {
        let at_stop = method(reactor, terms, t_s, y, first);
        let one = supplied(reactor, terms, t_s, y, &at_stop, first, true);
        let end = method(reactor, terms, stop, &at_stop, h - first);
        let two = supplied(reactor, terms, stop, &at_stop, &end, h - first, false);
        return Advanced { y: end, supplied_mw: one + two };
    }
    let end = method(reactor, terms, t_s, y, h);
    let moving = t_s < stop;
    Advanced { y: end, supplied_mw: supplied(reactor, terms, t_s, y, &end, h, moving) }
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
    if !reactor.start_is_liquid_water(state.start_temperature_k) {
        return Err(StepReject::StartTemperatureNotLiquidWater);
    }
    let table = match reactor.table_of(state.rod_table_core) {
        Some(table) => table,
        None => return Err(StepReject::RodTableOutsideCard),
    };
    if !(load.reactivity_inserted_dollar > 0.0) {
        return Err(StepReject::ReactivityNotPositive);
    }
    if load.reactivity_inserted_dollar > reactor.rod_worth_dollar {
        return Err(StepReject::ReactivityAboveRodWorth);
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
        return Err(StepReject::ImpossibleState);
    }
    let mut index = 0;
    while index < 6 {
        if state.precursor_mw[index] < 0.0 {
            return Err(StepReject::ImpossibleState);
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
    if state.fuel_temperature_k > reactor.fuel_temperature_limit_k
        || state.peak_fuel_temperature_k > reactor.fuel_temperature_limit_k
    {
        return Err(StepReject::FuelTemperatureAboveFuelLimit);
    }
    if state.t_s > 0.0
        && state.reactivity_load_dollar.to_bits() != load.reactivity_inserted_dollar.to_bits()
    {
        return Err(StepReject::ReactivityChangedDuringRun);
    }
    if state.rods_dropped && load.rods_dropped != 1.0 {
        return Err(StepReject::RodsRaisedAfterScram);
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
    if y[7] > reactor.fuel_temperature_limit_k {
        return Err(StepReject::FuelTemperatureAboveFuelLimit);
    }
    let t_new = state.t_s + dt_s;
    within_state_ceiling(reactor, terms, t_new, &y, dt_s)?;
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
        Advanced, DELAYED_FRACTION, DOLLAR_OFFSET, DOLLAR_SCALE, ENERGY_COMPARED_MAX_MWS,
        FLAG_PROMPT_SUPERCRITICAL, FLAG_ROD_MOVING, FLAG_RODS_DROPPED, FUEL_MASS_KG,
        FUEL_TEMPERATURE_COMPARED_MAX_K, GENERATION_TIME_S, GROUP_FRACTION, HEAT_FLOW_W_PER_K,
        INITIAL_POWER_MW, POWER_COMPARED_MAX_MW, RAMP_DOLLAR_PER_S, REACTIVITY_COMPARED_MAX_DOLLAR,
        REACTIVITY_COMPARED_MIN_DOLLAR, RESIDUAL_CONSERVATION_LIMIT, RISE_PIECE_PHASE_STEP,
        ROD_TABLE_CORE, ROD_WORTH_DOLLAR, ReactorLoad, ReactorState, ReactorTrigaJsi,
        SCRAM_AFTER_PEAK_COMPARED_S, START_TEMPERATURE_K_BY_TABLE, STATE_PHASE_STEP, StepReject,
        TIME_COMPARED_MAX_S, Terms, WATER_BOILING_K, WATER_FREEZING_K, advance, commit, feedback,
        feedback_slope, net_reactivity, pieces, rates, rise_pieces, rk4, scram_pieces, state_vector,
        step, supplied, terms, through_the_stop, validate, within_state_ceiling,
    };
    use crate::frame::LastStateFrame64;

    type Method = fn(&ReactorTrigaJsi, &Terms, f64, &[f64; 9], f64) -> [f64; 9];

    /// The starts compared on each table, K: core 215, 217 and 231.
    const T215: (f64, f64) = START_TEMPERATURE_K_BY_TABLE[0];
    const T217: (f64, f64) = START_TEMPERATURE_K_BY_TABLE[1];
    const T231: (f64, f64) = START_TEMPERATURE_K_BY_TABLE[2];
    /// The limit of U-ZrH1.6 fuel in stainless clad, 1150 degC, K.
    const FUEL_LIMIT_K: f64 = super::FUEL_TEMPERATURE_LIMIT_K;

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

    /// `|ρ − 1| β / Λ`, per second.
    fn pole_per_s(rho: f64) -> f64 {
        (rho - 1.0).abs() * DELAYED_FRACTION / GENERATION_TIME_S
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
        let y = state_vector(state);
        let rho = within_state_ceiling(reactor, &terms, state.t_s, &y, dt_s)?;
        let n = pieces(reactor, state, load, rho, dt_s);
        let advanced = through_the_stop(method, reactor, &terms, state.t_s, &y, dt_s, n);
        commit(reactor, state, frame, &terms, load, dt_s, advanced)
    }

    /// The stepping of 0.9.0, through the card's own commit: a step taken in pieces assumes the
    /// rod has stopped, and every piece is integrated on the far side of the stop.
    fn step_as_if_stopped(
        reactor: &ReactorTrigaJsi,
        state: &mut ReactorState,
        frame: &mut LastStateFrame64,
        load: ReactorLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        let table = validate(reactor, state, load, dt_s)?;
        let terms = terms(reactor, state, load, table);
        let y = state_vector(state);
        let rho = within_state_ceiling(reactor, &terms, state.t_s, &y, dt_s)?;
        let n = pieces(reactor, state, load, rho, dt_s);
        let advanced = if n > 1 {
            let h = dt_s / n as f64;
            let mut at = y;
            let mut supplied_mw = 0.0;
            for piece in 0..n {
                let t = state.t_s + piece as f64 * h;
                let next = rk4(reactor, &terms, t, &at, h);
                supplied_mw += supplied(reactor, &terms, t, &at, &next, h, false);
                at = next;
            }
            Advanced { y: at, supplied_mw }
        } else {
            through_the_stop(rk4, reactor, &terms, state.t_s, &y, dt_s, 1)
        };
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
        /// The largest `|ρ − 1|` of any landed sample.
        fastest: f64,
        /// The highest fuel temperature of any landed sample.
        hottest_k: f64,
        end: Result<(), StepReject>,
    }

    /// A pulse from `kelvin` stepped with `method` to `until_s`, the rods dropped from the first
    /// step that starts at or after `scram_s`.
    fn run_on(method: Method, kelvin: f64, core: f64, rho: f64, dt_s: f64, scram_s: f64, until_s: f64) -> Run {
        run_until(method, kelvin, core, rho, dt_s, until_s, |s| s.t_s >= scram_s)
    }

    /// A pulse stepped to `until_s`, the rods dropped from the first step at whose start `drop` holds.
    fn run_until(
        method: Method,
        kelvin: f64,
        core: f64,
        rho: f64,
        dt_s: f64,
        until_s: f64,
        drop: impl Fn(&ReactorState) -> bool,
    ) -> Run {
        let (reactor, mut state) = start_on(kelvin, core);
        let mut frame = marked_frame();
        let mut out = Run { last: state, steps: 0, largest_residual: 0.0, fastest: 0.0, hottest_k: kelvin, end: Ok(()) };
        while state.t_s + dt_s <= until_s {
            let l = if state.rods_dropped || drop(&state) { dropped(rho) } else { load(rho) };
            out.end = step_with(method, &reactor, &mut state, &mut frame, l, dt_s);
            if out.end.is_err() {
                break;
            }
            out.steps += 1;
            out.largest_residual = out.largest_residual.max(state.residual_conservation_error);
            out.fastest = out.fastest.max((state.reactivity_net_dollar - 1.0).abs());
            out.hottest_k = out.hottest_k.max(state.fuel_temperature_k);
        }
        out.last = state;
        out
    }

    /// Each start in liquid water at an end, each table, at the rod's whole worth.
    fn whole_worth_extremes() -> [(f64, f64); 6] {
        [
            (WATER_FREEZING_K, 215.0),
            (WATER_BOILING_K, 215.0),
            (WATER_FREEZING_K, 217.0),
            (WATER_BOILING_K, 217.0),
            (WATER_FREEZING_K, 231.0),
            (WATER_BOILING_K, 231.0),
        ]
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
        altered.rod_worth_dollar = 5.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.residual_conservation_limit = 1.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.water_boiling_k = 400.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.state_phase_step = 1.0;
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
        // the rate is the rod's whole worth over its 80 ms, the bits of 0.9.0's 4.16 / 0.080
        assert_eq!(ROD_WORTH_DOLLAR, 4.16);
        assert_eq!(RAMP_DOLLAR_PER_S.to_bits(), (4.16_f64 / 0.080).to_bits());
        assert!(within(RAMP_DOLLAR_PER_S, 52.0, 1.0e-12));
        assert!(within(reactor.rod_stop_s(ROD_WORTH_DOLLAR), 0.080, 1.0e-12));
        assert!(within(ceiling(), 9.39365e-5, 1.0e-5));
        // the fitted generation time is 3.3 times the article's 8 % below the article's value
        assert!(within(GENERATION_TIME_S, 29.59e-6, 1.0e-12));
        // every printed dollar compared is prompt supercritical in the kinetics on every table;
        // the whole worth of the rod is 2.91 to 3.04 dollars in the kinetics
        for table in 0..3 {
            assert!(reactor.worth_dollar(table, REACTIVITY_COMPARED_MIN_DOLLAR) > 1.0);
            assert!(reactor.worth_dollar(table, REACTIVITY_COMPARED_MAX_DOLLAR) < 2.3);
            let whole = reactor.worth_dollar(table, ROD_WORTH_DOLLAR);
            assert!(whole > 2.91 && whole < 3.05, "{whole}");
        }
        assert_eq!((REACTIVITY_COMPARED_MIN_DOLLAR, REACTIVITY_COMPARED_MAX_DOLLAR), (2.0, 3.0));
        assert_eq!(SCRAM_AFTER_PEAK_COMPARED_S, (1.50, 7.45));
        assert_eq!((WATER_FREEZING_K, WATER_BOILING_K), (273.15, 373.124));
        assert_eq!(super::HOT_ROD_FACTOR, 1.387);
        // the state ceiling at zero net reactivity, and after the scram with no feedback, where
        // it is a quarter over the static ceiling
        assert!(within(reactor.state_dt_s(0.0), STATE_PHASE_STEP * GENERATION_TIME_S / DELAYED_FRACTION, 1.0e-15));
        assert!(within(reactor.state_dt_s(-8.0), ceiling() * STATE_PHASE_STEP / super::GATE_PHASE_STEP, 1.0e-12));
        assert_eq!(reactor.state_dt_s(1.0), f64::INFINITY);
    }

    #[test]
    fn the_start_is_not_stationary_and_any_start_in_liquid_water_on_the_three_tables_starts() {
        let reactor = ReactorTrigaJsi::core_56_pulse_mode();
        for (table, core) in ROD_TABLE_CORE.into_iter().enumerate() {
            let (low, high) = START_TEMPERATURE_K_BY_TABLE[table];
            for kelvin in [WATER_FREEZING_K, low, 0.5 * (low + high), high, 350.0, WATER_BOILING_K] {
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
        assert!(T215.0 > T231.0 && T215.1 < T217.1);
        // the ends of liquid water start; one unit in the last place beyond each does not
        for core in ROD_TABLE_CORE {
            assert!(ReactorState::try_start(&reactor, WATER_FREEZING_K, core).is_some());
            assert!(ReactorState::try_start(&reactor, WATER_BOILING_K, core).is_some());
            assert!(ReactorState::try_start(&reactor, f64::from_bits(WATER_FREEZING_K.to_bits() - 1), core).is_none());
            assert!(ReactorState::try_start(&reactor, f64::from_bits(WATER_BOILING_K.to_bits() + 1), core).is_none());
            // starts compared on another table, and the starts 0.6.0 and 0.9.0 refused, now start
            for celsius in [0.0, 20.0, 20.3, 22.0, 26.0, 29.24, 32.4, 33.0, 60.0, 99.97] {
                assert!(ReactorState::try_start(&reactor, celsius + 273.15, core).is_some(), "{celsius}");
            }
            // ice, and water at 100 degC, past the boiling point at one atmosphere
            for celsius in [-0.01, -20.0, 100.0, 120.0] {
                assert!(ReactorState::try_start(&reactor, celsius + 273.15, core).is_none(), "{celsius}");
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
        let lands = |state: ReactorState, load: ReactorLoad, dt_s: f64| {
            let mut s = state;
            let mut frame = marked_frame();
            assert_eq!(step(&reactor, &mut s, &mut frame, load, dt_s), Ok(()), "{load:?}");
            assert!(s.residual_conservation_error <= RESIDUAL_CONSERVATION_LIMIT);
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
        // a handed-in start in ice or in boiling water
        for kelvin in [f64::from_bits(WATER_FREEZING_K.to_bits() - 1), 250.0, f64::from_bits(WATER_BOILING_K.to_bits() + 1), 373.15, 400.0] {
            let mut other = fresh;
            other.start_temperature_k = kelvin;
            other.fuel_temperature_k = kelvin;
            other.peak_fuel_temperature_k = kelvin;
            refuse(other, load(2.0), dt, StepReject::StartTemperatureNotLiquidWater);
        }
        // inside liquid water, outside the starts compared on the state's own table: the card computes
        for (kelvin, core) in [(296.0, 231.0), (294.2, 215.0), (302.0, 215.0), (294.2, 217.0), (320.0, 217.0), (WATER_FREEZING_K, 231.0), (WATER_BOILING_K, 215.0)] {
            let mut other = fresh;
            other.rod_table_core = core;
            other.start_temperature_k = kelvin;
            other.fuel_temperature_k = kelvin;
            other.peak_fuel_temperature_k = kelvin;
            lands(other, load(2.0), dt);
        }
        let mut other_core = fresh;
        other_core.rod_table_core = 253.0;
        refuse(other_core, load(2.0), dt, StepReject::RodTableOutsideCard);
        other_core.rod_table_core = 216.0;
        refuse(other_core, load(2.0), dt, StepReject::RodTableOutsideCard);
        // the reactivity: above zero up to the rod's whole worth lands, ends included
        for rho in [f64::from_bits(1), 1.0e-3, 0.5, 1.4, 1.99, REACTIVITY_COMPARED_MIN_DOLLAR, REACTIVITY_COMPARED_MAX_DOLLAR, 3.01, 3.5, ROD_WORTH_DOLLAR] {
            lands(fresh, load(rho), dt);
        }
        // a reactivity so small that the ramp's rate in the kinetics, the table's worth of it
        // over it times 52 per second, passes the largest 64-bit number: the arithmetic's limit
        refuse(fresh, load(f64::MIN_POSITIVE), dt, StepReject::NonFiniteState);
        lands(fresh, load(1.0e-300), dt);
        for rho in [0.0, -0.0, -f64::from_bits(1), -1.0, -2.5] {
            refuse(fresh, load(rho), dt, StepReject::ReactivityNotPositive);
        }
        for rho in [f64::from_bits(ROD_WORTH_DOLLAR.to_bits() + 1), 4.17, 5.0, 100.0] {
            refuse(fresh, load(rho), dt, StepReject::ReactivityAboveRodWorth);
        }
        refuse(
            fresh,
            ReactorLoad { reactivity_inserted_dollar: 2.0, rods_dropped: 0.5 },
            dt,
            StepReject::ScramValueOutsideCard,
        );
        // the scram at the fire signal, while the rod moves, lands
        lands(fresh, dropped(2.0), dt);
        lands(fresh, dropped(ROD_WORTH_DOLLAR), ceiling());
        // a scram after the rod has stopped and before 1.50 s after the peak lands
        let mut early = fresh;
        early.t_s = 1.0;
        early.peak_time_s = 0.17;
        early.reactivity_load_dollar = 2.0;
        lands(early, dropped(2.0), dt);
        // rods held past 7.45 s after the peak land
        let mut held = early;
        held.t_s = 0.17 + 7.45;
        lands(held, load(2.0), dt);
        // a step that ends after 14 s lands
        let mut late = fresh;
        late.t_s = TIME_COMPARED_MAX_S - 0.5 * dt;
        late.peak_time_s = late.t_s - 2.0;
        late.reactivity_load_dollar = 2.0;
        late.rods_dropped = true;
        late.scram_time_s = late.t_s - 0.5;
        lands(late, dropped(2.0), dt);
        lands(late, dropped(2.0), 1.0e-6);
        // past the compared energy and fuel temperature a step lands; a power handed in with
        // no population behind it is the ledger's
        let mut spent = fresh;
        spent.energy_mws = 30.0;
        lands(spent, load(2.0), dt);
        let mut hot = fresh;
        hot.fuel_temperature_k = 700.0;
        hot.peak_fuel_temperature_k = 700.0;
        lands(hot, load(2.0), dt);
        let mut powerful = fresh;
        powerful.power_mw = 2000.0;
        powerful.peak_power_mw = 2000.0;
        refuse(powerful, load(2.0), dt, StepReject::SpontaneousGeneration);
        // a handed-in fuel temperature no run reaches, with the rods dropped: the fastest pole
        // there is beyond the state ceiling at the static ceiling
        let mut hotter = fresh;
        hotter.fuel_temperature_k = 1200.0;
        hotter.peak_fuel_temperature_k = 1200.0;
        refuse(hotter, dropped(2.0), ceiling(), StepReject::DtAboveStateCeiling);
    }

    /// The fuel's own limit, 1150 degC: a handed-in state above it is refused, and so is a step
    /// that would land above it, with the state and the frame unchanged. Just under it a
    /// handed-in state is stepped.
    #[test]
    fn the_fuel_limit_of_1150_degc_is_refused_by_name() {
        let (reactor, fresh) = start(25.0);
        let table = reactor.table_of(fresh.rod_table_core).unwrap();
        assert_eq!(FUEL_LIMIT_K.to_bits(), 1423.15_f64.to_bits());
        let refuse = |forged: ReactorState, load: ReactorLoad, dt: f64| {
            let mut s = forged;
            let mut f = marked_frame();
            let bytes = f.to_bytes();
            assert_eq!(step(&reactor, &mut s, &mut f, load, dt), Err(StepReject::FuelTemperatureAboveFuelLimit));
            assert_eq!(s, forged);
            assert_eq!(f.to_bytes(), bytes);
        };
        let mut over = fresh;
        over.fuel_temperature_k = FUEL_LIMIT_K + 1.0e-9 * FUEL_LIMIT_K;
        over.peak_fuel_temperature_k = over.fuel_temperature_k;
        refuse(over, load(2.0), 1.0e-6);
        refuse(over, dropped(2.0), 1.0e-6);
        let mut peak_over = fresh;
        peak_over.fuel_temperature_k = 1000.0;
        peak_over.peak_fuel_temperature_k = 1500.0;
        refuse(peak_over, load(2.0), 1.0e-6);
        // just under the limit a handed-in state is stepped (the scram's fast pole there keeps
        // the step short)
        let mut under = fresh;
        under.fuel_temperature_k = FUEL_LIMIT_K - 1.0;
        under.peak_fuel_temperature_k = under.fuel_temperature_k;
        let mut s = under;
        let mut f = marked_frame();
        assert_eq!(step(&reactor, &mut s, &mut f, load(2.0), 1.0e-6), Ok(()));
        // a trial step handed back with its fuel above the limit is refused where it would land
        let mut honest = run_on(rk4, 298.15, 217.0, 3.0, ceiling(), f64::INFINITY, 0.5).last;
        let before = honest;
        let t = terms(&reactor, &honest, load(3.0), table);
        let mut advanced = through_the_stop(rk4, &reactor, &t, honest.t_s, &state_vector(&honest), ceiling(), 1);
        advanced.y[7] = FUEL_LIMIT_K + 0.5;
        let mut f = marked_frame();
        let bytes = f.to_bytes();
        assert_eq!(commit(&reactor, &mut honest, &mut f, &t, load(3.0), ceiling(), advanced), Err(StepReject::FuelTemperatureAboveFuelLimit));
        assert_eq!(honest, before);
        assert_eq!(f.to_bytes(), bytes);
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
        refuse(forged, StepReject::ImpossibleState);
        forged = state;
        forged.power_mw = -1.0;
        refuse(forged, StepReject::ImpossibleState);
        forged = state;
        forged.precursor_mw[2] = -1.0;
        refuse(forged, StepReject::ImpossibleState);
        // a negative clock is not a state any run reaches
        forged = state;
        forged.t_s = -1.0;
        refuse(forged, StepReject::ImpossibleState);
        forged = state;
        forged.t_s = -f64::from_bits(1);
        refuse(forged, StepReject::ImpossibleState);
        forged = state;
        forged.fuel_temperature_k = state.start_temperature_k - 1.0;
        refuse(forged, StepReject::ImpossibleState);
        forged = state;
        forged.scram_time_s = state.t_s + 1.0;
        refuse(forged, StepReject::ImpossibleState);
        forged = state;
        forged.energy_at_scram_mws = state.energy_mws + 1.0;
        refuse(forged, StepReject::ImpossibleState);
        forged = state;
        forged.energy_at_scram_mws = -1.0;
        refuse(forged, StepReject::ImpossibleState);
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
        for _ in 0..200 {
            step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
        }
        assert!(state.rod_moving);
        assert_eq!(state.reactivity_load_dollar, rho);
        let before = state;
        // a larger insertion while the rod moves, a smaller one, and one a single bit away
        for other in [2.5, 3.0, 1.5, 0.5, f64::from_bits(rho.to_bits() + 1)] {
            assert_eq!(step(&reactor, &mut state, &mut frame, load(other), dt), Err(StepReject::ReactivityChangedDuringRun));
            assert_eq!(step(&reactor, &mut state, &mut frame, dropped(other), dt), Err(StepReject::ReactivityChangedDuringRun));
            assert_eq!(state, before);
        }
        while state.t_s < 1.0 {
            step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
        }
        assert!(!state.rod_moving);
        let before = state;
        // after the rod has stopped: a second insertion, or a withdrawal, is not in the model
        for other in [3.0, 2.5, 2.0000001] {
            assert_eq!(step(&reactor, &mut state, &mut frame, load(other), dt), Err(StepReject::ReactivityChangedDuringRun));
            assert_eq!(step(&reactor, &mut state, &mut frame, dropped(other), dt), Err(StepReject::ReactivityChangedDuringRun));
            assert_eq!(state, before);
        }
        let power_before = state.power_mw;
        assert_eq!(state.energy_at_scram_mws, 0.0);
        // the scram 0.83 s after the peak, earlier than any compared, lands
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
        // rise plus the heat that left, at a compared pulse and at the rod's whole worth
        for (celsius, rho) in [(25.0, 2.0), (99.0, ROD_WORTH_DOLLAR), (0.0, 0.5)] {
            let (reactor, mut state) = start(celsius);
            let mut frame = marked_frame();
            let dt = 5.0e-5;
            let mut left_j = 0.0;
            let mut previous = state;
            while state.t_s < 2.0 {
                step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
                left_j += 0.5
                    * dt
                    * HEAT_FLOW_W_PER_K
                    * ((previous.fuel_temperature_k - state.start_temperature_k)
                        + (state.fuel_temperature_k - state.start_temperature_k));
                previous = state;
            }
            let enthalpy = |t: f64| {
                let x = t - super::HEAT_CAPACITY_REFERENCE_K;
                FUEL_MASS_KG * (super::HEAT_CAPACITY_BASE_J_PER_KG_K * x + 0.5 * super::HEAT_CAPACITY_SLOPE_J_PER_KG_K2 * x * x)
            };
            let stored = enthalpy(state.fuel_temperature_k) - enthalpy(state.start_temperature_k);
            assert!(within(stored + left_j, state.energy_mws * 1.0e6, 1.0e-6), "{celsius} {rho}: {} {} {}", stored, left_j, state.energy_mws);
            if rho == 2.0 {
                // the heat that leaves in two seconds is a few hundredths of the energy
                assert!(left_j > 0.01 * state.energy_mws * 1.0e6 && left_j < 0.05 * state.energy_mws * 1.0e6);
            }
        }
        // the highest fuel temperature is kept after the fuel has begun to cool
        let (reactor, mut s) = start(25.0);
        let mut frame = marked_frame();
        while s.t_s < 6.0 {
            let l = if s.t_s >= 3.0 { dropped(2.0) } else { load(2.0) };
            step(&reactor, &mut s, &mut frame, l, ceiling()).unwrap();
            assert!(s.peak_fuel_temperature_k >= s.fuel_temperature_k);
        }
        assert!(s.peak_fuel_temperature_k > s.fuel_temperature_k);
        assert!(s.fuel_temperature_k - s.start_temperature_k > 150.0);
        // the feedback and its slope agree, across the fitted span and past it to the hottest
        // fuel any run reaches
        for t in [273.15, 294.0, 300.0, 350.0, 500.0, 585.0, 599.0, 700.0, 770.0] {
            let d = 1.0e-3;
            let numeric = (feedback(&reactor, t + d) - feedback(&reactor, t - d)) / (2.0 * d);
            assert!(within(feedback_slope(&reactor, t), numeric, 1.0e-6));
            assert!(feedback_slope(&reactor, t) > 0.0);
        }
        assert_eq!(feedback(&reactor, 294.0), 0.0);
        // 6.2 pcm per K at 294 K, 8.9 at 600 K, 10.4 at 770 K
        assert!(within(feedback_slope(&reactor, 294.0), 6.241e-5, 1.0e-12));
        assert!(within(feedback_slope(&reactor, 600.0), 8.909e-5, 1.0e-3));
        assert!(within(feedback_slope(&reactor, 770.0), 1.039e-4, 1.0e-3));
    }

    /// The corners of the compared span: starting temperature in K, table, printed dollars. Each
    /// start is at an end of its own table's span.
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
            let run = run_on(rk4, kelvin, core, rho, ceiling(), span_scram(rho), TIME_COMPARED_MAX_S);
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
            // the card's pulses in the compared span stay under the largest compared
            assert!(run.last.peak_power_mw < POWER_COMPARED_MAX_MW);
            assert!(run.last.energy_at_scram_mws < ENERGY_COMPARED_MAX_MWS);
            assert!(run.last.peak_fuel_temperature_k < FUEL_TEMPERATURE_COMPARED_MAX_K);
            assert!(run.last.rods_dropped);
            assert!(run.last.t_s > TIME_COMPARED_MAX_S - ceiling());
        }
        println!("largest residual over the span at the ceiling: {largest:e}");
        assert!(largest < RESIDUAL_CONSERVATION_LIMIT / 10.0);
    }

    /// A run goes on past the 14 s the records compared run to, at four step sizes, to three times
    /// that, with the ledger closed at every step.
    #[test]
    fn a_run_goes_on_past_the_fourteen_seconds_compared_with_its_ledger_closed() {
        for dt_s in [5.0e-5, 8.75e-5, 2.0e-5, 7.0e-5] {
            let run = run_on(rk4, 298.15, 217.0, 2.0, dt_s, 3.0, 3.0 * TIME_COMPARED_MAX_S);
            assert_eq!(run.end, Ok(()), "{dt_s}");
            assert!(run.last.t_s > 3.0 * TIME_COMPARED_MAX_S - dt_s, "{}", run.last.t_s);
            assert!(run.largest_residual < RESIDUAL_CONSERVATION_LIMIT / 10.0, "{dt_s}: {:e}", run.largest_residual);
            // the core keeps cooling into the water after the scram
            assert!(run.last.fuel_temperature_k < run.last.peak_fuel_temperature_k - 40.0);
            assert!(run.last.power_mw < HEAT_FLOW_W_PER_K * (run.last.fuel_temperature_k - run.last.start_temperature_k) * 1.0e-6);
            println!("{dt_s} s: {} steps to {} s, fuel {} K, largest residual {:e}", run.steps, run.last.t_s, run.last.fuel_temperature_k, run.largest_residual);
        }
    }

    /// A 3.0 dollar pulse left without a scram passes 600 K after 5.2 to 6.4 s on cores 215
    /// and 231, where 0.9.0 refused it, and is computed on, with the ledger closed, to three
    /// times the 14 s compared; the power settles where the feedback and the heat flow to the
    /// water balance.
    #[test]
    fn a_pulse_left_without_a_scram_is_computed_on_and_settles() {
        // at each end of each table's own span, at the ceiling: the time of the last landed step
        // under 600 K, where 0.9.0 refused the next as FuelTemperatureBeyondCard; on core 217 it
        // refused the rods held past 7.45 s after the peak, at 7.558 s
        let under_600 = [[Some(6.105), Some(5.198)], [None, None], [Some(6.428), Some(5.890)]];
        for (table, core) in ROD_TABLE_CORE.into_iter().enumerate() {
            let (low, high) = START_TEMPERATURE_K_BY_TABLE[table];
            for (end, kelvin) in [low, high].into_iter().enumerate() {
                let (reactor, mut state) = start_on(kelvin, core);
                let mut frame = marked_frame();
                let mut last_under = None;
                let mut largest: f64 = 0.0;
                while state.t_s + ceiling() <= 3.0 * TIME_COMPARED_MAX_S {
                    let t = state.t_s;
                    step(&reactor, &mut state, &mut frame, load(3.0), ceiling()).unwrap();
                    largest = largest.max(state.residual_conservation_error);
                    if state.fuel_temperature_k > 600.0 && last_under.is_none() {
                        last_under = Some(t);
                    }
                }
                println!("{kelvin} K, table {core}, held: last step under 600 K at {last_under:?} s, highest {} K, at {} s fuel {} K, power {} MW, residual {largest:e}", state.peak_fuel_temperature_k, state.t_s, state.fuel_temperature_k, state.power_mw);
                match under_600[table][end] {
                    Some(t) => assert!((last_under.unwrap() - t).abs() < 1.0e-3, "{kelvin} K, table {core}: {last_under:?}"),
                    None => assert!(last_under.is_none_or(|t| t > 7.0), "{kelvin} K, table {core}: {last_under:?}"),
                }
                assert!(largest < RESIDUAL_CONSERVATION_LIMIT / 10.0);
                // past 7.45 s after the peak and three times the 14 s compared, the energy far past
                // the largest compared
                assert!(state.t_s - state.peak_time_s > 5.0 * SCRAM_AFTER_PEAK_COMPARED_S.1);
                assert!(state.energy_mws > ENERGY_COMPARED_MAX_MWS);
                // the fuel cools towards where the feedback and the heat flow to the water balance:
                // the power is under the heat flow to the water, the net reactivity under zero
                let to_water = HEAT_FLOW_W_PER_K * (state.fuel_temperature_k - state.start_temperature_k) * 1.0e-6;
                assert!(state.power_mw < to_water && state.power_mw > 0.2 * to_water, "{} {}", state.power_mw, to_water);
                assert!(state.reactivity_net_dollar < 0.0 && state.reactivity_net_dollar > -0.5);
                assert!(state.fuel_temperature_k < state.peak_fuel_temperature_k);
            }
        }
    }

    /// The hottest the card's fuel gets: the rod's whole worth, the rods held, from either end of
    /// liquid water on each table, for as long as a run of a million steps at the ceiling lasts,
    /// 93.9 s. The highest core-average fuel temperature is 769.7 K, at 4.16 dollars on core 231
    /// from 373.124 K, 8.1 s after the fire signal; through the hot-rod factor the hottest element
    /// is at 923 K. Both are far below the limit of the fuel, 1150 degC. Held on, the pulse
    /// settles where the feedback takes back the whole insertion.
    #[test]
    fn the_hottest_fuel_any_run_reaches_is_far_below_the_fuel_limit() {
        let mut hottest = (0.0, 0.0, 0.0);
        for (kelvin, core) in whole_worth_extremes() {
            let run = run_on(rk4, kelvin, core, ROD_WORTH_DOLLAR, ceiling(), f64::INFINITY, 1.0e6 * ceiling());
            assert_eq!(run.end, Ok(()), "{kelvin} {core}");
            assert_eq!(run.steps, 999_999, "{kelvin} {core}");
            assert!(run.largest_residual < RESIDUAL_CONSERVATION_LIMIT / 10.0, "{kelvin} {core}: {:e}", run.largest_residual);
            assert_eq!(run.last.peak_fuel_temperature_k, run.hottest_k);
            // the power past 2.3 times the largest compared, the energy past 2.2 times
            assert!(run.last.peak_power_mw > 2.3 * POWER_COMPARED_MAX_MW && run.last.peak_power_mw < 2.9 * POWER_COMPARED_MAX_MW);
            assert!(run.last.energy_mws > 2.2 * ENERGY_COMPARED_MAX_MWS);
            println!("{kelvin} K, table {core}, 4.16 dollars held: peak {} MW at {} s, energy {} MW s, highest fuel {} K, residual {:e}", run.last.peak_power_mw, run.last.peak_time_s, run.last.energy_mws, run.hottest_k, run.largest_residual);
            if run.hottest_k > hottest.0 {
                hottest = (run.hottest_k, kelvin, core);
            }
        }
        assert_eq!((hottest.1, hottest.2), (WATER_BOILING_K, 231.0));
        assert!((hottest.0 - 769.74).abs() < 0.01, "{}", hottest.0);
        let element = WATER_BOILING_K + super::HOT_ROD_FACTOR * (hottest.0 - WATER_BOILING_K);
        assert!((element - 923.2).abs() < 0.1, "{element}");
        assert!(element < FUEL_LIMIT_K - 450.0);
        // a weaker insertion, or a scram, makes it cooler
        for (rho, scram) in [(3.0, f64::INFINITY), (ROD_WORTH_DOLLAR, 0.0), (ROD_WORTH_DOLLAR, 0.0934), (ROD_WORTH_DOLLAR, 3.0)] {
            let run = run_on(rk4, WATER_BOILING_K, 231.0, rho, ceiling(), scram, 12.0);
            assert_eq!(run.end, Ok(()));
            assert!(run.hottest_k < hottest.0, "{rho} {scram}: {}", run.hottest_k);
        }
        // held on, to 400 s: the net reactivity falls to the 1.6e-4 dollar the start is short
        // of stationary, the fuel temperature to where the feedback takes back the whole
        // insertion's worth, and the power to the heat flow to the water
        let run = run_on(rk4, WATER_BOILING_K, 231.0, ROD_WORTH_DOLLAR, ceiling(), f64::INFINITY, 400.0);
        assert_eq!(run.end, Ok(()));
        let reactor = ReactorTrigaJsi::core_56_pulse_mode();
        let worth = reactor.worth_dollar(2, ROD_WORTH_DOLLAR) - (DELAYED_FRACTION - 0.00699891) / DELAYED_FRACTION;
        let wanted = feedback(&reactor, WATER_BOILING_K) + worth * DELAYED_FRACTION;
        let a = 0.5 * super::FEEDBACK_SLOPE_PER_K2;
        let x = (-super::FEEDBACK_PER_K + (super::FEEDBACK_PER_K * super::FEEDBACK_PER_K + 4.0 * a * wanted).sqrt()) / (2.0 * a);
        let settled_k = super::FEEDBACK_REFERENCE_K + x;
        println!("held 400 s: fuel {} K against {settled_k} K, power {} MW, net {} dollar, residual {:e}", run.last.fuel_temperature_k, run.last.power_mw, run.last.reactivity_net_dollar, run.largest_residual);
        assert!((run.last.fuel_temperature_k - settled_k).abs() < 0.05, "{} {settled_k}", run.last.fuel_temperature_k);
        assert!((settled_k - 636.6).abs() < 0.1);
        let to_water = HEAT_FLOW_W_PER_K * (run.last.fuel_temperature_k - WATER_BOILING_K) * 1.0e-6;
        assert!(within(run.last.power_mw, to_water, 1.0e-3), "{} {to_water}", run.last.power_mw);
        assert!(run.last.reactivity_net_dollar.abs() < 1.0e-3);
        assert!(run.largest_residual < RESIDUAL_CONSERVATION_LIMIT / 10.0);
        // the energy is more than five times the largest compared
        assert!(run.last.energy_mws > 5.0 * ENERGY_COMPARED_MAX_MWS);
    }

    /// Below and above the compared reactivity, to the rod's whole worth and down to a
    /// thousandth of a dollar, and from either end of liquid water, the card computes the same
    /// equations with the ledger closed over 14 s at the ceiling, the rods dropped 2.5 s after
    /// the fire signal.
    #[test]
    fn past_the_compared_reactivity_and_starts_the_ledger_closes() {
        let mut largest: f64 = 0.0;
        for core in ROD_TABLE_CORE {
            let mut peaks = std::vec::Vec::new();
            for rho in [1.0e-3, 0.5, 1.0, 1.4, 1.8, 2.0, 3.0, 3.5, ROD_WORTH_DOLLAR] {
                for kelvin in [WATER_FREEZING_K, 298.15, WATER_BOILING_K] {
                    let run = run_on(rk4, kelvin, core, rho, ceiling(), 2.5, TIME_COMPARED_MAX_S);
                    assert_eq!(run.end, Ok(()), "{kelvin} {core} {rho}");
                    largest = largest.max(run.largest_residual);
                    if kelvin == 298.15 {
                        peaks.push(run.last.peak_power_mw);
                    }
                }
            }
            println!("table {core}: peaks {peaks:?} MW");
            // the stronger the insertion the stronger the pulse
            assert!(peaks.windows(2).all(|pair| pair[0] < pair[1]), "{peaks:?}");
        }
        println!("largest residual {largest:e}");
        assert!(largest < RESIDUAL_CONSERVATION_LIMIT / 4.0);
    }

    /// A scram lands at any time after the fire signal: at the first step, while the rod moves,
    /// in the step that crosses the rod's stop, before the peak, at it, and from there to long
    /// after any compared, at three steps, on the corners of the compared span and at the rod's
    /// whole worth.
    #[test]
    fn a_scram_lands_at_any_time_after_the_fire_signal() {
        let mut largest: f64 = 0.0;
        let mut probes = 0;
        let corners = SPAN.into_iter().chain(whole_worth_extremes().into_iter().map(|(k, c)| (k, c, ROD_WORTH_DOLLAR)));
        for (kelvin, core, rho) in corners {
            for dt_s in [ceiling(), 5.0e-5, 2.0e-5] {
                let (reactor, _) = start_on(kelvin, core);
                let stop = reactor.rod_stop_s(rho);
                let free = run_on(rk4, kelvin, core, rho, dt_s, f64::INFINITY, 0.3);
                let peak = free.last.peak_time_s;
                for at in [0.0, 0.5 * stop, stop - 0.5 * dt_s, stop, 0.5 * (stop + peak), peak, peak + 0.01, peak + 1.0] {
                    let run = run_on(rk4, kelvin, core, rho, dt_s, at, at + 0.05);
                    assert_eq!(run.end, Ok(()), "{kelvin} K {core} {rho} dollar, scram at {at} s, step {dt_s}");
                    assert!(run.last.rods_dropped && run.last.scram_time_s >= at && run.last.scram_time_s < at + dt_s);
                    largest = largest.max(run.largest_residual);
                    probes += 1;
                }
            }
        }
        // and long after any compared: 30 s after the fire signal, at the ceiling
        for (kelvin, core, rho) in [(T231.1, 231.0, 3.0), (WATER_BOILING_K, 231.0, ROD_WORTH_DOLLAR), (298.15, 217.0, 2.0)] {
            let run = run_on(rk4, kelvin, core, rho, ceiling(), 30.0, 31.0);
            assert_eq!(run.end, Ok(()));
            assert!(run.last.scram_time_s - run.last.peak_time_s > 4.0 * SCRAM_AFTER_PEAK_COMPARED_S.1);
            largest = largest.max(run.largest_residual);
            probes += 1;
        }
        println!("{probes} scrams, largest residual {largest:e}");
        assert!(largest < RESIDUAL_CONSERVATION_LIMIT / 4.0);
        // the pieces of the fall: none before the scram, eight at the ceiling in the fall, one after it
        let (reactor, mut state) = start(25.0);
        let mut frame = marked_frame();
        assert_eq!(scram_pieces(&reactor, &state, load(2.0), ceiling()), 1);
        while state.t_s - state.peak_time_s < SCRAM_AFTER_PEAK_COMPARED_S.0 || state.t_s < 1.0 {
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

    /// The scram while the transient rod is still moving: its ramp goes on under the 8 dollars,
    /// and a step taken in the pieces of the fall that holds the rod's stop is split there, each
    /// part with its own integral on its own side of the stop. The pulse is the one taken at a
    /// sixteenth of the step. The stepping of 0.9.0, which took every piece of the fall as past
    /// the stop, is named by the ledger at the first step of such a scram.
    #[test]
    fn a_scram_while_the_rod_moves_is_stepped_across_the_stop() {
        for rho in [2.0, 3.0, ROD_WORTH_DOLLAR] {
            let (reactor, _) = start(25.0);
            let stop = reactor.rod_stop_s(rho);
            for dt_s in [ceiling(), 5.0e-5, 2.0e-5] {
                // the scram from the step that starts three steps before the stop: the next steps
                // are in pieces, and one of them holds the stop
                let steps_before = (stop / dt_s).floor() as u64 - 3;
                let at = steps_before as f64 * dt_s - 0.5 * dt_s;
                let coarse = run_on(rk4, 298.15, 217.0, rho, dt_s, at, 0.5);
                assert_eq!(coarse.end, Ok(()), "{rho} {dt_s}");
                let fine_dt = dt_s / 16.0;
                let fine = run_until(rk4, 298.15, 217.0, rho, fine_dt, 0.5 - 0.5 * fine_dt, |s| s.t_s >= coarse.last.scram_time_s - 0.5 * fine_dt);
                assert_eq!(fine.end, Ok(()));
                assert!((fine.last.scram_time_s - coarse.last.scram_time_s).abs() < fine_dt);
                println!("{rho} dollar, step {dt_s}: scram at {} s, rod stops at {stop} s, power at 0.5 s {} against {} MW, residual {:e}", coarse.last.scram_time_s, coarse.last.power_mw, fine.last.power_mw, coarse.largest_residual);
                assert!(coarse.last.scram_time_s < stop);
                // as close as the pulse at the ceiling is to the pulse at a sixteenth of it
                assert!(within(coarse.last.power_mw, fine.last.power_mw, 1.0e-4), "{} {}", coarse.last.power_mw, fine.last.power_mw);
                assert!(coarse.largest_residual < RESIDUAL_CONSERVATION_LIMIT / 10.0);
                assert_eq!(coarse.last.reactivity_inserted_now_dollar, rho);
                // the stepping of 0.9.0 is refused by the ledger at the first step of the scram,
                // whose pieces lie before the stop; at 2e-5 s, in two pieces, its error on
                // the small power before the pulse is under the ledger's limit
                if dt_s < 5.0e-5 {
                    continue;
                }
                let (reactor, mut state) = start(25.0);
                let mut frame = marked_frame();
                let mut refused = None;
                while state.t_s < stop + 4.0 * dt_s {
                    let l = if state.rods_dropped || state.t_s >= at { dropped(rho) } else { load(rho) };
                    let before = state;
                    let bytes = frame.to_bytes();
                    if let Err(reject) = step_as_if_stopped(&reactor, &mut state, &mut frame, l, dt_s) {
                        assert_eq!(state, before);
                        assert_eq!(frame.to_bytes(), bytes);
                        refused = Some((reject, state.t_s));
                        break;
                    }
                }
                let (reject, t) = refused.unwrap_or_else(|| std::panic!("{rho} {dt_s}: the old stepping landed"));
                assert!(reject == StepReject::SpontaneousGeneration || reject == StepReject::FirstLawBreach, "{reject:?}");
                // at the first step of the scram, or at 4.16 dollars already in the pieces of the rise
                assert!(t < stop && (t >= at || rho == ROD_WORTH_DOLLAR), "{t} {stop}");
            }
        }
        // under the scram the rod goes on to its stop: both flags on the frame, the inserted
        // reactivity at the ramp, and the net reactivity 8 dollars under the table's worth of it
        let (reactor, mut state) = start(25.0);
        let mut frame = marked_frame();
        let rho = 3.0;
        while state.t_s < 0.03 {
            step(&reactor, &mut state, &mut frame, dropped(rho), 5.0e-5).unwrap();
            if state.t_s < reactor.rod_stop_s(rho) {
                assert_eq!(frame.flags & (FLAG_ROD_MOVING | FLAG_RODS_DROPPED), FLAG_ROD_MOVING | FLAG_RODS_DROPPED);
                assert!(within(state.reactivity_inserted_now_dollar, RAMP_DOLLAR_PER_S * state.t_s, 1.0e-12));
                let worth = reactor.worth_dollar(1, rho) * state.reactivity_inserted_now_dollar / rho;
                assert!(within(state.reactivity_net_dollar, worth - super::SCRAM_DOLLAR, 1.0e-6));
            }
        }
        assert_eq!(state.scram_time_s, 0.0);
        assert!(state.power_mw < INITIAL_POWER_MW);
    }

    /// The static ceiling covers every state a run of the card reaches: on the compared runs the
    /// net reactivity after the scram reaches −9.08 dollars, where a step at the static ceiling
    /// advances 0.224 of the time constant of the fastest pole; at the rod's whole worth with the
    /// rods dropped at the highest fuel temperature, −9.87 dollars and 0.241. The state ceiling,
    /// 0.25, is not met there; a state handed in hotter than any run reaches meets it, is
    /// refused with state and frame kept, and lands at a step under it with the ledger closed.
    #[test]
    fn the_static_ceiling_covers_every_state_a_run_reaches_and_the_state_ceiling_holds_the_rest() {
        let reactor = ReactorTrigaJsi::core_56_pulse_mode();
        // the compared runs, at the ceiling, the rods dropped up to the latest compared
        let mut compared: f64 = 0.0;
        for (kelvin, core, rho) in SPAN {
            for after in [SCRAM_AFTER_PEAK_COMPARED_S.0, 4.0, SCRAM_AFTER_PEAK_COMPARED_S.1 - 0.01] {
                if rho > 2.5 && core != 217.0 && after > 5.0 {
                    // held that long these passed 600 K, beyond any compared fuel temperature
                    continue;
                }
                let run = run_until(rk4, kelvin, core, rho, ceiling(), TIME_COMPARED_MAX_S, |s| s.t_s > 1.0 && s.t_s - s.peak_time_s >= after);
                assert_eq!(run.end, Ok(()));
                compared = compared.max(run.fastest);
            }
        }
        println!("compared: largest |rho - 1| {compared}, the static ceiling advances {} of its time constant", ceiling() * pole_per_s(1.0 + compared));
        assert!(compared > 10.0 && compared < 10.09, "{compared}");
        assert!(ceiling() < reactor.state_dt_s(1.0 - compared));
        // every start in liquid water at the rod's whole worth, the rods dropped where the held
        // pulse is hottest
        let mut reached: f64 = 0.0;
        for (kelvin, core) in whole_worth_extremes() {
            let held = run_on(rk4, kelvin, core, ROD_WORTH_DOLLAR, ceiling(), f64::INFINITY, 12.0);
            let hottest_at = {
                let (_, mut s) = start_on(kelvin, core);
                let mut f = marked_frame();
                let mut at = 0.0;
                while s.t_s < 12.0 {
                    step(&reactor, &mut s, &mut f, load(ROD_WORTH_DOLLAR), ceiling()).unwrap();
                    if s.fuel_temperature_k == held.hottest_k {
                        at = s.t_s;
                    }
                }
                at
            };
            let run = run_on(rk4, kelvin, core, ROD_WORTH_DOLLAR, ceiling(), hottest_at, hottest_at + 1.0);
            assert_eq!(run.end, Ok(()), "{kelvin} {core}");
            assert!(run.largest_residual < RESIDUAL_CONSERVATION_LIMIT / 4.0);
            reached = reached.max(run.fastest);
        }
        println!("any run: largest |rho - 1| {reached}, the static ceiling advances {} of its time constant", ceiling() * pole_per_s(1.0 + reached));
        assert!(reached > 10.86 && reached < 10.87, "{reached}");
        assert!(ceiling() * pole_per_s(1.0 + reached) < 0.2415);
        assert!(ceiling() < reactor.state_dt_s(1.0 - reached));
        // a state hotter than any run reaches: a run after its scram, its fuel set to 1000 K
        let mut state = run_on(rk4, 298.15, 217.0, 3.0, ceiling(), 3.0, 3.5).last;
        state.fuel_temperature_k = 1000.0;
        state.peak_fuel_temperature_k = 1000.0;
        let table = reactor.table_of(state.rod_table_core).unwrap();
        let rho = net_reactivity(&reactor, &terms(&reactor, &state, dropped(3.0), table), state.t_s, &state_vector(&state));
        let limit = reactor.state_dt_s(rho);
        println!("handed-in at 1000 K: net {rho} dollar, state ceiling {limit:e} s");
        assert!(limit < ceiling());
        for dt_s in [ceiling(), f64::from_bits(limit.to_bits() + 1)] {
            let mut s = state;
            let mut f = marked_frame();
            let bytes = f.to_bytes();
            assert_eq!(step(&reactor, &mut s, &mut f, dropped(3.0), dt_s), Err(StepReject::DtAboveStateCeiling));
            assert_eq!(s, state);
            assert_eq!(f.to_bytes(), bytes);
        }
        let mut s = state;
        let mut f = marked_frame();
        for _ in 0..1000 {
            assert_eq!(step(&reactor, &mut s, &mut f, dropped(3.0), limit), Ok(()));
            assert!(s.residual_conservation_error < RESIDUAL_CONSERVATION_LIMIT);
        }
        // the ceiling is held where the step lands too: a trial step handed back with its fuel at
        // 1000 K is refused there
        let mut honest = run_on(rk4, 298.15, 217.0, 3.0, ceiling(), 3.0, 3.5).last;
        let before = honest;
        let t = terms(&reactor, &honest, dropped(3.0), table);
        let mut advanced = through_the_stop(rk4, &reactor, &t, honest.t_s, &state_vector(&honest), ceiling(), 1);
        advanced.y[7] = 1000.0;
        let mut f = marked_frame();
        assert_eq!(commit(&reactor, &mut honest, &mut f, &t, dropped(3.0), ceiling(), advanced), Err(StepReject::DtAboveStateCeiling));
        assert_eq!(honest, before);
    }

    /// In the rise of a pulse stronger than any compared a step is taken in pieces of at most
    /// 0.03 of the time constant of the growing pole; every step of a compared pulse is one piece.
    #[test]
    fn a_step_in_the_rise_of_a_pulse_stronger_than_any_compared_is_taken_in_pieces() {
        let reactor = ReactorTrigaJsi::core_56_pulse_mode();
        // the net reactivity of a compared pulse is at most its whole insertion's worth
        let compared = (0..3).map(|table| reactor.worth_dollar(table, REACTIVITY_COMPARED_MAX_DOLLAR)).fold(0.0, f64::max);
        assert!(within(compared, 2.2068, 1.0e-4), "{compared}");
        assert!(ceiling() * pole_per_s(compared) < 0.0269 && 0.0269 < RISE_PIECE_PHASE_STEP);
        assert_eq!(rise_pieces(&reactor, compared, ceiling()), 1);
        // at the rod's whole worth, two
        let whole = reactor.worth_dollar(2, ROD_WORTH_DOLLAR);
        assert_eq!(rise_pieces(&reactor, whole, ceiling()), 2);
        assert_eq!(rise_pieces(&reactor, whole, 0.5 * ceiling()), 1);
        assert_eq!(rise_pieces(&reactor, 0.5, ceiling()), 1);
        assert_eq!(rise_pieces(&reactor, -9.0, ceiling()), 1);
        assert_eq!(rise_pieces(&reactor, whole, 1.0), super::SCRAM_PIECES_MAX);
        // on the compared span, no step of the rise is pieced
        for (kelvin, core, rho) in SPAN {
            let (_, mut state) = start_on(kelvin, core);
            let mut frame = marked_frame();
            let table = reactor.table_of(core).unwrap();
            while state.t_s < 0.3 {
                let t = terms(&reactor, &state, load(rho), table);
                let net = net_reactivity(&reactor, &t, state.t_s, &state_vector(&state));
                assert!(net <= compared);
                assert_eq!(pieces(&reactor, &state, load(rho), net, ceiling()), 1);
                step(&reactor, &mut state, &mut frame, load(rho), ceiling()).unwrap();
            }
        }
        // the rods dropped in the rise or at the peak of a strong pulse: the ledger closes at the
        // ceiling over the 14 s compared
        let mut largest: f64 = 0.0;
        for core in ROD_TABLE_CORE {
            for rho in [3.25, 3.5, 4.0, ROD_WORTH_DOLLAR] {
                let peak = run_on(rk4, 298.15, core, rho, ceiling(), f64::INFINITY, 0.3).last.peak_time_s;
                let stop = reactor.rod_stop_s(rho);
                for at in [stop, 0.5 * (stop + peak), peak, peak + 0.01] {
                    let run = run_on(rk4, 298.15, core, rho, ceiling(), at, TIME_COMPARED_MAX_S);
                    assert_eq!(run.end, Ok(()), "{core} {rho} {at}");
                    largest = largest.max(run.largest_residual);
                }
            }
        }
        println!("strong pulses, the rods dropped in the rise, over 14 s at the ceiling: largest residual {largest:e}");
        assert!(largest < RESIDUAL_CONSERVATION_LIMIT / 4.0);
        // over a long tail the error made in the rise is carried by a population tens of times
        // smaller, and the ledger can stop a run: at half the ceiling it lands to 93.9 s
        let half = run_on(rk4, WATER_BOILING_K, 231.0, ROD_WORTH_DOLLAR, 0.5 * ceiling(), 0.0924, 93.9);
        assert_eq!(half.end, Ok(()));
        assert!(half.largest_residual < RESIDUAL_CONSERVATION_LIMIT, "{:e}", half.largest_residual);
    }

    #[test]
    fn explicit_euler_at_the_ceiling_is_refused_by_the_ledger() {
        for (kelvin, core, rho) in SPAN.into_iter().chain([(WATER_BOILING_K, 231.0, ROD_WORTH_DOLLAR), (WATER_FREEZING_K, 217.0, 0.5)]) {
            let run = run_on(euler, kelvin, core, rho, ceiling(), 6.0, 11.9);
            assert!(
                run.end == Err(StepReject::SpontaneousGeneration)
                    || run.end == Err(StepReject::FirstLawBreach),
                "{kelvin} {rho}: {:?} after {} steps",
                run.end,
                run.steps
            );
            // named before the pulse: the power is still under a megawatt
            assert!(run.last.power_mw < 1.0, "{}", run.last.power_mw);
        }
    }

    #[test]
    fn wrong_stage_weights_and_a_falsified_ledger_are_refused() {
        for (kelvin, core, rho) in SPAN.into_iter().chain([(WATER_BOILING_K, 231.0, ROD_WORTH_DOLLAR), (WATER_FREEZING_K, 217.0, 0.5)]) {
            let run = run_on(wrong_weights, kelvin, core, rho, ceiling(), 6.0, 11.9);
            assert!(
                run.end == Err(StepReject::SpontaneousGeneration)
                    || run.end == Err(StepReject::FirstLawBreach),
                "{kelvin} {rho}: {:?}",
                run.end
            );
            assert!(run.last.power_mw < 1.0);
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

    /// With the feedback dropped the pulse never turns, and the ledger, whose integral carries
    /// the feedback, names the step in the rise, before 2 s.
    #[test]
    fn a_dropped_feedback_runs_away_and_is_refused() {
        for (kelvin, core, rho) in SPAN.into_iter().chain([(WATER_BOILING_K, 231.0, ROD_WORTH_DOLLAR)]) {
            let run = run_on(no_feedback, kelvin, core, rho, ceiling(), 6.0, 11.9);
            println!("{kelvin} {core} {rho}: {:?} at {} s, power {} MW", run.end, run.last.t_s, run.last.power_mw);
            assert!(
                run.end == Err(StepReject::SpontaneousGeneration)
                    || run.end == Err(StepReject::FirstLawBreach)
                    || run.end == Err(StepReject::NonFiniteState),
                "{kelvin} {rho}: {:?}",
                run.end
            );
            assert!(run.last.t_s < 2.0);
        }
    }

    #[test]
    fn the_pulse_at_the_ceiling_is_the_pulse_at_a_sixteenth_of_it() {
        for (kelvin, core, rho) in [(298.15, 217.0, 2.0), (T231.0, 231.0, 3.0), (T215.1, 215.0, 2.0), (WATER_BOILING_K, 231.0, ROD_WORTH_DOLLAR), (WATER_FREEZING_K, 215.0, 1.4)] {
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
            assert!(within(coarse.last.peak_power_mw, fine.last.peak_power_mw, 2.0e-4));
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
