#![no_implicit_prelude]

//! The SPERT III E-core, low-initial-power bursts at three inlet
//! temperatures, with the fuel temperature and the water temperature as
//! state, from the start of the transient rod's motion to the first maximum
//! of power.
//!
//! Measured by the Phillips Petroleum Company for the Atomic Energy
//! Commission: IDO-17281 (March 1969), OSTI 4792676. Table III (cold
//! startup, 30 tests) and Table V (hot startup at about 260 and 500 degF, 32
//! tests), each value with a standard deviation. The report's cover says
//! "Distribution of this document is unlimited." The OSTI record page states
//! no licence and no copyright notice. No measured value of the report is
//! stored in the card.
//!
//! This card stands beside `reactor_spert3`, which is unchanged. That card
//! steps the reactivity in at time zero from 5e-5 MW and puts every peak
//! about 22 percent early. The power histories of Appendix D show why
//! (`analysis/10_peak_timing_cause.md` of the research record): the
//! reactivity is ramped in by a rod accelerated at 2000 in/s², over 50 to 85
//! ms, and the bursts ask for a start about five times lower than the
//! report's 50 W. This card carries both. The ramp is the report's design
//! figure, a constant acceleration and a constant worth per inch, and not a
//! recorded rod motion. The starting power is a fitted effective value and
//! not a measurement: the second scoring (`verification/second_party/V-25`) found 7
//! to 11 W across the things it varied, 50 W excluded under this model, and
//! 10 to 15 ms of fixed-time error left in the insertion, which shows as late
//! peaks on the fastest tests.
//!
//! ```text
//! inserted reactivity  ρ_ins = min(ρ_0, K t²)                          dollars
//! net reactivity       ρ = ρ_ins − G (√(T_in + θ_f) − √T_in) − A θ_w
//! power                dP/dt = ((ρ − 1) / l*) P + Σ λ_i C_i            MW per s
//! precursor sums       dC_i/dt = (b_i / l*) P − λ_i C_i                i = 1 to 6
//! fuel temperature     dθ_f/dt = (1 − f_w) P / C_f                     rise above inlet, K
//! water temperature    dθ_w/dt = f_w P / C_w − (2 v / L) θ_w           rise above inlet, K
//! energy released      dE/dt = P                                       MW
//! ```
//!
//! `ρ_0` and `v` are the load: the reactivity inserted and the coolant flow
//! rate. A burst has one reactivity. The first landed step records it in the
//! state, and a later step that carries another is refused by name and the state kept: a second
//! insertion, a withdrawal or a jump is not the one ramp of the equations. Each step may carry
//! its own flow rate: the flow enters only through the sweep of the water temperature, and the
//! sweep integral of the ledger is booked per step at that step's flow. No tested burst changed
//! its flow, and a burst whose flow changes was not compared. `T_in` is the inlet temperature the
//! state was started at. The card has three bands of inlet temperature, the three the report
//! tested and gives constants for: 56 to 92 degF, 251 to 259 degF, 498 to 501 degF. `l*`, `K`,
//! `A` and `C_w` are per band. Between the bands nothing was measured and the card refuses.
//!
//! Compared per band on the tested periods: 0.769 to 1.210 dollars at 56 to 92 degF, 0.919 to
//! 1.207 at 251 to 259 degF, 0.837 to 1.227 at 498 to 501 degF; on the 30 held-back tests peak
//! power met its lines in every band and the time of the peak was missed in the two cooler bands
//! (root mean square 5.22 and 8.46 percent against 5, worst 12.24 and 14.88 percent against 10).
//! Outside those spans the card computes the same ramp and feedback at any reactivity above zero
//! ([`StepReject::ReactivityNotPositive`] below it), and nothing was compared; the Doppler term
//! grows without bound, so a first maximum always comes. The cold band was tested with no flow,
//! 251 to 259 degF at 2.4 to 22 ft/s and 498 to 501 degF at 4.8 to 24 ft/s; the card computes
//! at any flow of zero or more in any band, the cold band included ([`StepReject::NegativeFlow`]
//! below zero). The longest scored time of peak is 13.7 s, the largest energy to the peak in
//! Tables III and V 15 MW s, the largest peak power 410 MW; past them the card computes the same
//! burst to its first maximum, with no limit of time, energy or power. The fuel lump is about a
//! fourteenth of the heat capacity of the fuel and is not a temperature of the fuel itself, so
//! the card has no melting limit to stop at.
//!
//! Locked from the report: `l*` (Table II), the six-group set (Appendix B
//! 3.3, stated for 500 degF and used in all three bands), `K` (Appendix B
//! section 8 and Figs. B-4 and B-5: half of 2000 in/s² times the rod worth
//! per inch), the moderator temperature coefficient (Table II), the share of
//! the power deposited promptly in the water `f_w` (Appendix B), the active
//! length `L`, and the water mass (Appendix A core composition). Water
//! density and specific heat are NIST's. Four numbers are fitted, on 28
//! tests (the 20 cold-startup tests opened by the first protocol and eight
//! 500 degF tests): [`INITIAL_POWER_MW`], [`DOPPLER_STRENGTH_DOLLAR_PER_SQRT_K`],
//! [`FUEL_HEAT_CAPACITY_MWS_PER_K`] and [`WATER_FEEDBACK_WEIGHT`]. The
//! values are those of `analysis/v2/out/f04_card_constants.json`: the fit of
//! the protocol as registered, at six significant figures, which the second
//! scoring redid on its own and reached to all six. The file
//! `out/f03_fit_full.json` now on disk is a later fit and differs in the sixth
//! figure (`analysis/v2/note_which_fit_the_card_carries.md`). Thirty tests
//! were held back, among them the whole series the report names for about 260 degF, whose inlet
//! temperatures were 251 to 259 degF; the protocol is `validation_protocol_v2.md`.
//!
//! The fuel temperature is one lump weighted to where the feedback is made.
//! Its heat capacity comes out near one tenth of the whole UO2. The report
//! measures no fuel temperature; this state is tested only through what it
//! does to power, energy and time. Heat flow from fuel to water is left out:
//! fitted on the opened cold tests with a cooling time constant free, the
//! constant went to infinity (`analysis/v2/out/f01_explore.json`).
//!
//! The card leaves out hot standby and operating power (a start from 1 MW or
//! 19 MW with a steady temperature field), boiling, void, pressure, anything
//! after the first maximum, a reactivity changed during a burst, any other core.
//!
//! The ledger is the neutron population `N = P + Σ C_i` (MW). Summing the
//! equations, `dN/dt = ρ P / l*`. The integral of `ρ P` has two parts in
//! closed form and two by quadrature:
//!
//! ```text
//! ∫ G (√(T_in + θ_f) − √T_in) P dt = (G C_f / (1 − f_w)) d² (2u + s) / 3,   u = √(T_in + θ_f), s = √T_in, d = u − s
//! ∫ θ_w P dt = (C_w / f_w) [ θ_w² / 2 + ∫ (2 v / L) θ_w² dt ]
//! W = ∫ ρ_ins P dt − (first line) − A (second line)
//! population_defect_mw        = N − N_anchor − W / l*
//! residual_conservation_error = |population_defect_mw| / max(N, N_anchor)
//! ```
//!
//! The two remaining integrals, of `ρ_ins P` and of `(2 v / L) θ_w²`, are
//! taken over each step by the two-point Hermite rule, `h (f0 + f1) / 2 +
//! h² (f0' − f1') / 12`, from the values and rates at the two ends of the
//! step. The Runge–Kutta step never evaluates `W`, and the rule is not the
//! Runge–Kutta rule, so a wrong coefficient, a dropped group, a wrong stage
//! weight or an Euler step shows in the defect. The ledger shows the
//! arithmetic and the integrator are right. It cannot show the physics
//! matches the reactor.
//!
//! What the residual is, as the second scoring measured it on the fastest run
//! of each band with a ledger of its own (`verification/second_party/V-25`,
//! `ledger.py`). It names a wrong method. Named on all three runs: explicit
//! Euler, the midpoint and Heun's second-order methods, Kutta's third-order
//! method, Runge–Kutta with one stage weight wrong by 1e-3 and by 1e-5, a
//! delayed group dropped from the power equation, the Doppler strength wrong
//! by 1e-3 and by 1e-5 in the stepping, a group fraction wrong by 1e-4, and
//! Runge–Kutta at two and at three times the ceiling. Not named: a group
//! fraction wrong by 1e-6, and Runge–Kutta at 1.5 times the ceiling. It is
//! calibrated on the card's own method: the limit is 11 times the largest
//! residual the card's own step leaves. It is not a bound on the accuracy of
//! the state. Both integrals are taken from the card's own power, so most of
//! the error of the power cancels in the defect, and the residual reads the
//! true error of the state low by about 11 to 15 times. On two of the three
//! runs the true error of the population against a solution 32 times finer,
//! 3.3e-7 and 3.8e-7, is above the residual's limit of 3e-7 while the residual
//! sits at 2.3e-8 and 2.6e-8, far under it; on the third it is 3.0e-7.
//!
//! An incoming state is held to what the card itself writes: no energy,
//! temperature rise or precursor sum below zero, a power above zero, a clock at or above zero,
//! the peak fields a sample of this run, the card's start at time zero, and every derived field
//! equal bit for bit to the card's own arithmetic on the rest of the state.
//! A state that fails is refused by name and kept as it was. Three things the
//! state cannot show are not refused. While the rod is moving the state holds
//! nothing of where it is sent but the recorded load, so that field rewritten
//! together with the load gives, bit for bit, the one ramp to the new target.
//! After the rod has stopped the equations hold no time, so a clock moved
//! later, with the rest of the state as it was, steps on. And the energy
//! released is tied to the fuel temperature rise only to rounding, so an
//! energy at or above zero that disagrees with it, written into the
//! state and into its peak sample together, is not named.
//!
//! [`ReactorSpert3Fuel::gate_dt_s`] is the static step ceiling:
//! `GATE_PHASE_STEP · l*` of the cold band, 0.43 ms, 0.2 of the time constant of the
//! prompt pole at zero net reactivity, where every run starts. Past the compared spans the step
//! ceiling follows the state: the step itself refuses, state and frame kept, a step longer than
//! 0.2 of the time constant of the fastest pole at the state it starts from or at the state it
//! would land on, or 0.045 of it when that pole is growing ([`GROWTH_PHASE_STEP`],
//! [`StepReject::DtAboveStateCeiling`]): the prompt pole, of size up to `|1 − ρ| / l*`, the
//! feedback pole of the fuel and the water temperature, and the sweep `2 v / L` of the water.
//! At the static ceiling the growing prompt pole passes 0.045 of its time constant per step
//! above a net reactivity of about 1.22 dollars, just above the compared spans (1.21 to 1.23),
//! and a flow above about 226 m/s makes the sweep faster than the static ceiling allows: at 2
//! dollars the state ceiling is 97 µs, at 6 dollars 20 µs. Inside the compared spans, at the
//! static ceiling, the state ceiling refuses no step.
//! [`ReactorSpert3Fuel::floor_dt_s`] is 1e-9 s. The public step applies neither the static
//! ceiling nor the floor; the gateway refuses a step outside them. The state ceiling the step
//! applies.
//!
//! `core` only, and `sqrt` from the platform. The state is `Copy`. An
//! accepted step writes time, power, energy released, net reactivity, the
//! two temperature rises, the highest landed power, the power rate and the
//! conservation residual into [`LastStateFrame64`]. The 16-byte frame proof
//! is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::exceedance::{Exceedance, ExceedanceKind, Side};
use crate::frame::LastStateFrame64;
use crate::normal::{normal_or_zero, normal_or_zero_each};

/// Delayed-neutron group fractions `b_i`, fractions of the total delayed
/// fraction; sum 1. IDO-17281, Appendix B 3.3 (report page 109), stated for
/// 500 degF.
pub const GROUP_FRACTION: [f64; 6] = [0.0352, 0.2014, 0.1858, 0.4006, 0.1437, 0.0333];
/// Delayed-neutron decay constants `λ_i`, per second. Same table.
pub const GROUP_DECAY_PER_S: [f64; 6] = [0.0127, 0.0317, 0.1167, 0.3142, 1.4007, 3.8803];
/// Lowest inlet temperature of each band, K: 56, 251 and 498 degF. IDO-17281 Tables III and V.
/// Written (F − 32) / 1.8 + 273.15, the conversion of the research scripts and of every
/// compared request (held-back tests 38 at 56 degF, 70 at 251 degF and 54 at 498 degF were sent
/// at these numbers). It lies one or two units in the last place below (F + 459.67) / 1.8, the
/// conversion both E-core cards use for a temperature they state, so a tested edge converted
/// either way is inside its band.
pub const BAND_INLET_MIN_K: [f64; 3] = [
    (56.0 - 32.0) / 1.8 + 273.15,
    (251.0 - 32.0) / 1.8 + 273.15,
    (498.0 - 32.0) / 1.8 + 273.15,
];
/// Highest inlet temperature of each band, K: 92, 259 and 501 degF, converted as
/// (F + 459.67) / 1.8, the conversion both E-core cards use: 92 degF is 306.48333333333335 K in
/// both. Each is the smallest double at or above the exact edge, so the tested edge converted
/// either way is inside its band.
pub const BAND_INLET_MAX_K: [f64; 3] = [
    (92.0 + 459.67) / 1.8,
    (259.0 + 459.67) / 1.8,
    (501.0 + 459.67) / 1.8,
];
/// Reduced prompt-neutron generation time `l*` of each band, s. IDO-17281
/// Table II, rows 70, 250 and 500 degF.
pub const BAND_GENERATION_TIME_REDUCED_S: [f64; 3] = [0.00215, 0.00218, 0.00225];
/// Insertion ramp `K` of each band, dollars per s²: half the rod's design
/// acceleration, 2000 in/s², times the rod worth per inch, 0.18 dollar at
/// 500 degF, times the report's multipliers 1.667 (70 degF) and 1.611
/// (250 degF). IDO-17281 Appendix B section 8, Figs. B-4 and B-5.
pub const BAND_RAMP_DOLLAR_PER_S2: [f64; 3] = [300.0, 180.0 * 1.611, 180.0];
/// Moderator temperature coefficient of each band, dollars per K, sign
/// removed: 0.4, 2.0 and 4.0 cents per degF. IDO-17281 Table II.
pub const BAND_MODERATOR_COEFFICIENT_DOLLAR_PER_K: [f64; 3] = [0.0072, 0.036, 0.072];
/// Heat capacity of the water in the core in each band, MW s per K: 120.33
/// kg at 70 degF (1271.5 kg UO2 × 6.7 / 70.8, IDO-17281 Appendix A core
/// composition) scaled by density, times specific heat; NIST Chemistry
/// WebBook at 294.26 K 0.0853 MPa, 399.3 K and 533.15 K at 10.44 MPa.
pub const BAND_WATER_HEAT_CAPACITY_MWS_PER_K: [f64; 3] = [0.50331, 0.48072, 0.46672];
/// Smallest reactivity compared in each band, dollars: the inhour value of the longest
/// scored reactor period of the band (1010, 249 and 592 ms) with the band's
/// `l*`, not rounded: the value the compared request asked, which is this
/// crate's inhour sum or one unit in the last place below it. Below it the card computes the
/// same equations at any reactivity above zero, and nothing was compared.
pub const BAND_REACTIVITY_MIN_DOLLAR: [f64; 3] = [0.7686211839115998, 0.9192304970495506, 0.8366515135277093];
/// Largest reactivity compared in each band, dollars: the inhour value of the
/// shortest tested period (10, 10.3 and 9.7 ms), not rounded: the value the
/// compared request asked, which is this crate's inhour sum or one unit in
/// the last place above it. Above it the card computes the same equations, and nothing was
/// compared.
pub const BAND_REACTIVITY_MAX_DOLLAR: [f64; 3] = [1.2102315121058187, 1.206741353802147, 1.2273310554599017];
/// Smallest coolant flow rate tested in each band, m/s: no flow in the cold band, 2.4 ft/s at
/// 251 to 259 degF and 4.8 ft/s at 498 to 501 degF (IDO-17281 Table V). Each is the largest
/// double at or below the exact value. The card computes at any flow of zero or more in any
/// band; the flow enters only through the sweep of the water temperature.
pub const BAND_FLOW_MIN_M_PER_S: [f64; 3] = [0.0, 0.73152, 1.46304];
/// Largest coolant flow rate tested in each band, m/s: none cold, 22 ft/s at 251 to 259 degF
/// and 24 ft/s at 498 to 501 degF (IDO-17281 Table V). Each is the smallest double at or above
/// the exact value.
pub const BAND_FLOW_MAX_M_PER_S: [f64; 3] = [0.0, 22.0 * 0.3048, 24.0 * 0.3048];
/// Share of the power deposited promptly in the water, `f_w`. IDO-17281
/// Appendix B, heat-source fractions.
pub const WATER_PROMPT_HEATING_FRACTION: f64 = 0.032;
/// Active fuel length `L`, m: 3.19166 ft. IDO-17281 Appendix B hydraulics.
pub const ACTIVE_LENGTH_M: f64 = 3.19166 * 0.3048;
/// Reactor power at the clock's zero, MW. Fitted: 11.6 W, an effective value that also takes
/// up what the design ramp leaves out, and not a measurement of that power. The report says
/// "about 50 W" for cold startup and "10 to 50 watts" for hot startup.
pub const INITIAL_POWER_MW: f64 = 1.16113e-5;
/// Doppler strength `G`, dollars per square root of a kelvin. Fitted.
pub const DOPPLER_STRENGTH_DOLLAR_PER_SQRT_K: f64 = 0.0371601;
/// Fuel heat capacity `C_f`, MW s per K: one lump weighted to where the
/// feedback is made. Fitted.
pub const FUEL_HEAT_CAPACITY_MWS_PER_K: f64 = 0.0260616;
/// Weight on the water-temperature feedback. Fitted; one number for the
/// three bands. It came out below one.
pub const WATER_FEEDBACK_WEIGHT: f64 = 0.28586;
/// `|N − N_anchor − W / l*| / max(N, N_anchor)` must stay at or under this.
/// Set by measurement over ten runs at the ends of each band's span, each at
/// the ceiling to its first maximum: the largest value found is 2.6e-8, on
/// the rise of the fastest 500 degF burst, and the limit is 11 times it. The
/// residual compares the step with the card's own quadrature of the same
/// power, so it names a wrong method and is not a bound on the error: it
/// reads the true error of the state low by about 11 to 15 times. The outside
/// recomputation (`verification/crosscheck/reactor_spert3_fuel`) puts the card's
/// error at the ceiling at 3.1e-7 at worst, and the second scoring
/// (`verification/second_party/V-25`) at 3.3e-7, 3.0e-7 and 3.8e-7 on the fastest
/// run of each band, above this limit on two of the three.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 3.0e-7;
/// Fraction of the prompt time constant `l*` that one step at the ceiling
/// advances, and of the time constant of the fastest pole at the state ceiling.
pub const GATE_PHASE_STEP: f64 = 0.2;
/// Fraction of the time constant of a growing pole that one step advances at most, at the
/// state ceiling. Inside the compared spans the static ceiling gives the growing prompt pole at
/// most 0.0434 of its time constant per step (1.227 dollars at 498 to 501 degF); held to 0.045
/// past them, the ledger residual of runs from 1.5 to 6 dollars stays at or under 3.9e-8 of the
/// limit's 3e-7, as inside them (2.6e-8), where 0.07 lets it reach 1.6e-7. A growing pole is
/// followed more closely than a decaying one because the error of a step grows with it.
pub const GROWTH_PHASE_STEP: f64 = 0.045;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-9;

/// Net reactivity is above one dollar on this frame.
pub const FLAG_PROMPT_SUPERCRITICAL: u64 = 1 << 0;
/// The inserted reactivity is still below the load: the rod is moving.
pub const FLAG_ROD_MOVING: u64 = 1 << 1;

/// Why a step did not advance the reactor or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt`, the reactivity or the flow rate was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this reactor's.
    InvalidCard,
    /// The recorded inlet temperature is in none of the three tested bands: 56 to 92, 251 to 259, 498 to 501 degF.
    InletTemperatureOutsideCard,
    /// The held reactivity is zero or less: no step in reactivity is inserted, and there is no burst.
    ReactivityNotPositive,
    /// The coolant flow rate is below zero: the flow runs through the core from the inlet, and a flow the other way is not a flow rate of this core.
    NegativeFlow,
    /// The state is past the first maximum of power; nothing after it is on the card.
    PastFirstMaximum,
    /// The incoming state holds a time below zero.
    NegativeTime,
    /// The step is longer than 0.2 of the time constant of the fastest pole at the state it starts from or at the state it would land on, or 0.045 of it when that pole is growing: the prompt pole |1 - net reactivity| / l*, the feedback pole of the fuel and the water temperature, or the sweep 2 v / L of the water.
    DtAboveStateCeiling,
    /// The incoming state holds an energy released, a temperature rise or a precursor sum below zero, or a power that is not above zero.
    NegativeState,
    /// The reactivity of the load is not the one the first step of this burst sent the rod to; a burst is one insertion, ramped in and then held.
    ReactivityChangedDuringBurst,
    /// The incoming state is not one this card writes: at time zero it is not the card's start; later, its clock, inserted reactivity and rod flag are not on the ramp to the burst's reactivity, or its power rate, net reactivity, ledger anchor, defect or residual, a flag or a peak field is not what the card computes from the rest of the state.
    InconsistentState,
    /// The trial step created neutron population beyond what the reactivity supplies.
    SpontaneousGeneration,
    /// The trial step destroyed neutron population the reactivity supplies.
    FirstLawBreach,
}

/// What the operator sets for one burst. The first landed step records it in the state. Every
/// later step of that burst must carry the same reactivity; each step may carry its own flow
/// rate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorLoad {
    /// Dollars of reactivity the transient rod inserts in all, `ρ_0`.
    pub reactivity_inserted_dollar: f64,
    /// Average coolant flow rate through the core, `v`.
    pub coolant_flow_m_per_s: f64,
}

/// The locked reactor. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorSpert3Fuel {
    pub group_fraction: [f64; 6],
    pub group_decay_per_s: [f64; 6],
    pub band_inlet_min_k: [f64; 3],
    pub band_inlet_max_k: [f64; 3],
    pub band_generation_time_reduced_s: [f64; 3],
    pub band_ramp_dollar_per_s2: [f64; 3],
    pub band_moderator_coefficient_dollar_per_k: [f64; 3],
    pub band_water_heat_capacity_mws_per_k: [f64; 3],
    pub band_reactivity_min_dollar: [f64; 3],
    pub band_reactivity_max_dollar: [f64; 3],
    pub band_flow_min_m_per_s: [f64; 3],
    pub band_flow_max_m_per_s: [f64; 3],
    pub water_prompt_heating_fraction: f64,
    pub active_length_m: f64,
    pub initial_power_mw: f64,
    pub doppler_strength_dollar_per_sqrt_k: f64,
    pub fuel_heat_capacity_mws_per_k: f64,
    pub water_feedback_weight: f64,
    pub residual_conservation_limit: f64,
    pub gate_phase_step: f64,
    pub growth_phase_step: f64,
    pub floor_dt_s: f64,
}

fn same3(a: &[f64; 3], b: &[f64; 3]) -> bool {
    a[0].to_bits() == b[0].to_bits()
        && a[1].to_bits() == b[1].to_bits()
        && a[2].to_bits() == b[2].to_bits()
}

impl ReactorSpert3Fuel {
    /// Card: the SPERT III E-core, low-initial-power bursts. The constructor takes no arguments.
    #[must_use]
    pub fn e_core_low_initial_power() -> Self {
        let reactor = Self {
            group_fraction: GROUP_FRACTION,
            group_decay_per_s: GROUP_DECAY_PER_S,
            band_inlet_min_k: BAND_INLET_MIN_K,
            band_inlet_max_k: BAND_INLET_MAX_K,
            band_generation_time_reduced_s: BAND_GENERATION_TIME_REDUCED_S,
            band_ramp_dollar_per_s2: BAND_RAMP_DOLLAR_PER_S2,
            band_moderator_coefficient_dollar_per_k: BAND_MODERATOR_COEFFICIENT_DOLLAR_PER_K,
            band_water_heat_capacity_mws_per_k: BAND_WATER_HEAT_CAPACITY_MWS_PER_K,
            band_reactivity_min_dollar: BAND_REACTIVITY_MIN_DOLLAR,
            band_reactivity_max_dollar: BAND_REACTIVITY_MAX_DOLLAR,
            band_flow_min_m_per_s: BAND_FLOW_MIN_M_PER_S,
            band_flow_max_m_per_s: BAND_FLOW_MAX_M_PER_S,
            water_prompt_heating_fraction: WATER_PROMPT_HEATING_FRACTION,
            active_length_m: ACTIVE_LENGTH_M,
            initial_power_mw: INITIAL_POWER_MW,
            doppler_strength_dollar_per_sqrt_k: DOPPLER_STRENGTH_DOLLAR_PER_SQRT_K,
            fuel_heat_capacity_mws_per_k: FUEL_HEAT_CAPACITY_MWS_PER_K,
            water_feedback_weight: WATER_FEEDBACK_WEIGHT,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_phase_step: GATE_PHASE_STEP,
            growth_phase_step: GROWTH_PHASE_STEP,
            floor_dt_s: FLOOR_DT_S,
        };
        if reactor.is_valid() {
            reactor
        } else {
            ::core::panic!("E-core fuel-temperature reactor failed is_valid");
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
        same3(&self.band_inlet_min_k, &BAND_INLET_MIN_K)
            && same3(&self.band_inlet_max_k, &BAND_INLET_MAX_K)
            && same3(
                &self.band_generation_time_reduced_s,
                &BAND_GENERATION_TIME_REDUCED_S,
            )
            && same3(&self.band_ramp_dollar_per_s2, &BAND_RAMP_DOLLAR_PER_S2)
            && same3(
                &self.band_moderator_coefficient_dollar_per_k,
                &BAND_MODERATOR_COEFFICIENT_DOLLAR_PER_K,
            )
            && same3(
                &self.band_water_heat_capacity_mws_per_k,
                &BAND_WATER_HEAT_CAPACITY_MWS_PER_K,
            )
            && same3(&self.band_reactivity_min_dollar, &BAND_REACTIVITY_MIN_DOLLAR)
            && same3(&self.band_reactivity_max_dollar, &BAND_REACTIVITY_MAX_DOLLAR)
            && same3(&self.band_flow_min_m_per_s, &BAND_FLOW_MIN_M_PER_S)
            && same3(&self.band_flow_max_m_per_s, &BAND_FLOW_MAX_M_PER_S)
            && self.water_prompt_heating_fraction.to_bits()
                == WATER_PROMPT_HEATING_FRACTION.to_bits()
            && self.active_length_m.to_bits() == ACTIVE_LENGTH_M.to_bits()
            && self.initial_power_mw.to_bits() == INITIAL_POWER_MW.to_bits()
            && self.doppler_strength_dollar_per_sqrt_k.to_bits()
                == DOPPLER_STRENGTH_DOLLAR_PER_SQRT_K.to_bits()
            && self.fuel_heat_capacity_mws_per_k.to_bits()
                == FUEL_HEAT_CAPACITY_MWS_PER_K.to_bits()
            && self.water_feedback_weight.to_bits() == WATER_FEEDBACK_WEIGHT.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.gate_phase_step.to_bits() == GATE_PHASE_STEP.to_bits()
            && self.growth_phase_step.to_bits() == GROWTH_PHASE_STEP.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
    }

    /// The band an inlet temperature falls in: 0 for 56 to 92 degF, 1 for 251 to 259 degF, 2 for
    /// 498 to 501 degF.
    #[must_use]
    pub fn band_of(&self, inlet_temperature_k: f64) -> Option<usize> {
        let mut band = 0;
        while band < 3 {
            if inlet_temperature_k >= self.band_inlet_min_k[band]
                && inlet_temperature_k <= self.band_inlet_max_k[band]
            {
                return Some(band);
            }
            band += 1;
        }
        None
    }

    /// Largest step the card stands behind: `GATE_PHASE_STEP · l*` of the cold band, 0.43 ms.
    /// Past the compared spans the state ceiling of the step binds as well
    /// ([`StepReject::DtAboveStateCeiling`]). Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let l_star = self.band_generation_time_reduced_s[0];
        if l_star.is_finite() && l_star > 0.0 && self.gate_phase_step > 0.0 {
            Some(self.gate_phase_step * l_star)
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
}

/// Integrated reactor. The ledger and peak fields are part of the state; write them only through [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorState {
    /// Time since the transient rod began to move.
    pub t_s: f64,
    /// Reactor power, `P`.
    pub power_mw: f64,
    /// Delayed-neutron precursor sums, `C_i`.
    pub precursor_mw: [f64; 6],
    /// Fuel temperature rise above the inlet temperature, `θ_f`.
    pub fuel_temperature_rise_k: f64,
    /// Water temperature rise above the inlet temperature, `θ_w`.
    pub water_temperature_rise_k: f64,
    /// Energy released since the start, `E`.
    pub energy_mws: f64,
    /// Coolant inlet temperature at the start, `T_in`. It selects the band.
    pub inlet_temperature_k: f64,
    /// `dP/dt` at this state under the load of the last step.
    pub power_rate_mw_per_s: f64,
    /// Net reactivity at this state under the load of the last step. Zero at the start.
    pub reactivity_net_dollar: f64,
    /// `min(ρ_0, K t²)` at this state under the load of the last step.
    pub reactivity_inserted_now_dollar: f64,
    /// `N = P + Σ C_i` at the start.
    pub population_anchor_mw: f64,
    /// `∫ ρ_ins P dt` since the start, by the Hermite rule.
    pub insertion_integral_dollar_mws: f64,
    /// `∫ (2 v / L) θ_w² dt` since the start, by the Hermite rule.
    pub sweep_integral_k2: f64,
    /// Signed `N − N_anchor − W / l*`. Positive is created population.
    pub population_defect_mw: f64,
    /// `|population_defect_mw| / max(N, N_anchor)`.
    pub residual_conservation_error: f64,
    /// Highest power of any landed sample.
    pub peak_power_mw: f64,
    /// Energy released at the sample of [`Self::peak_power_mw`].
    pub peak_energy_mws: f64,
    /// Time of the sample of [`Self::peak_power_mw`].
    pub peak_time_s: f64,
    /// Reactivity of the load of the first landed step, `ρ_0`: where the rod of this burst is
    /// sent. Zero at the start, before any step has landed.
    pub burst_reactivity_dollar: f64,
    /// Coolant flow rate of the load of the last landed step, `v`. Zero at the start.
    pub coolant_flow_held_m_per_s: f64,
    /// Set by the step that lands, with the insertion complete, on a power no longer rising. The next step is refused.
    pub past_first_maximum: bool,
    /// Net reactivity above one dollar.
    pub prompt_supercritical: bool,
    /// The inserted reactivity is below the load.
    pub rod_moving: bool,
}

impl ReactorState {
    /// The reactor in equilibrium at [`INITIAL_POWER_MW`], zero net reactivity, both
    /// temperature rises zero, time 0. `None` unless the inlet temperature is finite and in
    /// one of the three bands.
    #[must_use]
    pub fn try_start(reactor: &ReactorSpert3Fuel, inlet_temperature_k: f64) -> Option<Self> {
        if !reactor.is_valid() || !inlet_temperature_k.is_finite() {
            return None;
        }
        let band = reactor.band_of(inlet_temperature_k)?;
        let l_star = reactor.band_generation_time_reduced_s[band];
        let power_mw = reactor.initial_power_mw;
        let mut precursor_mw = [0.0; 6];
        let mut index = 0;
        while index < 6 {
            precursor_mw[index] = reactor.group_fraction[index] * power_mw
                / (l_star * reactor.group_decay_per_s[index]);
            index += 1;
        }
        let mut state = Self {
            t_s: 0.0,
            power_mw,
            precursor_mw,
            fuel_temperature_rise_k: 0.0,
            water_temperature_rise_k: 0.0,
            energy_mws: 0.0,
            inlet_temperature_k,
            power_rate_mw_per_s: 0.0,
            reactivity_net_dollar: 0.0,
            reactivity_inserted_now_dollar: 0.0,
            population_anchor_mw: 0.0,
            insertion_integral_dollar_mws: 0.0,
            sweep_integral_k2: 0.0,
            population_defect_mw: 0.0,
            residual_conservation_error: 0.0,
            peak_power_mw: power_mw,
            peak_energy_mws: 0.0,
            peak_time_s: 0.0,
            burst_reactivity_dollar: 0.0,
            coolant_flow_held_m_per_s: 0.0,
            past_first_maximum: false,
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

/// The constants of one band and one load, gathered for the step.
#[derive(Clone, Copy)]
struct Terms {
    l_star: f64,
    ramp: f64,
    rho_total: f64,
    doppler: f64,
    sqrt_inlet: f64,
    inlet: f64,
    water_feedback: f64,
    fuel_rate: f64,
    water_rate: f64,
    sweep: f64,
}

fn terms(reactor: &ReactorSpert3Fuel, band: usize, inlet_k: f64, load: ReactorLoad) -> Terms {
    Terms {
        l_star: reactor.band_generation_time_reduced_s[band],
        ramp: reactor.band_ramp_dollar_per_s2[band],
        rho_total: load.reactivity_inserted_dollar,
        doppler: reactor.doppler_strength_dollar_per_sqrt_k,
        sqrt_inlet: inlet_k.sqrt(),
        inlet: inlet_k,
        water_feedback: reactor.water_feedback_weight
            * reactor.band_moderator_coefficient_dollar_per_k[band],
        fuel_rate: (1.0 - reactor.water_prompt_heating_fraction)
            / reactor.fuel_heat_capacity_mws_per_k,
        water_rate: reactor.water_prompt_heating_fraction
            / reactor.band_water_heat_capacity_mws_per_k[band],
        sweep: 2.0 * load.coolant_flow_m_per_s / reactor.active_length_m,
    }
}

/// `min(ρ_0, K t²)`.
fn inserted(terms: &Terms, t_s: f64) -> f64 {
    let ramped = terms.ramp * t_s * t_s;
    if ramped < terms.rho_total {
        ramped
    } else {
        terms.rho_total
    }
}

/// `G (√(T_in + θ_f) − √T_in)`, written so that a small rise loses no figures.
fn doppler_dollar(terms: &Terms, fuel_rise_k: f64) -> f64 {
    terms.doppler * fuel_rise_k / ((terms.inlet + fuel_rise_k).sqrt() + terms.sqrt_inlet)
}

fn net_reactivity(terms: &Terms, t_s: f64, y: &[f64; 10]) -> f64 {
    inserted(terms, t_s) - doppler_dollar(terms, y[7]) - terms.water_feedback * y[8]
}

/// Advance one Runge–Kutta step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    reactor: &ReactorSpert3Fuel,
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
        crate::exceedance::debug_check("reactor_spert3_fuel", exceedances(reactor, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    reactor: &ReactorSpert3Fuel,
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
    reactor: &ReactorSpert3Fuel,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    let band = validate(reactor, state, load, dt_s)?;
    let terms = terms(reactor, band, state.inlet_temperature_k, load);
    let advanced = rk4_through_the_stop(reactor, &terms, state.t_s, &state_vector(state), dt_s);
    commit(reactor, state, frame, &terms, load, dt_s, advanced)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut ReactorState) {
    state.t_s = normal_or_zero(state.t_s);
    state.power_mw = normal_or_zero(state.power_mw);
    normal_or_zero_each(&mut state.precursor_mw);
    state.fuel_temperature_rise_k = normal_or_zero(state.fuel_temperature_rise_k);
    state.water_temperature_rise_k = normal_or_zero(state.water_temperature_rise_k);
    state.energy_mws = normal_or_zero(state.energy_mws);
    state.inlet_temperature_k = normal_or_zero(state.inlet_temperature_k);
    state.power_rate_mw_per_s = normal_or_zero(state.power_rate_mw_per_s);
    state.reactivity_net_dollar = normal_or_zero(state.reactivity_net_dollar);
    state.reactivity_inserted_now_dollar = normal_or_zero(state.reactivity_inserted_now_dollar);
    state.population_anchor_mw = normal_or_zero(state.population_anchor_mw);
    state.insertion_integral_dollar_mws = normal_or_zero(state.insertion_integral_dollar_mws);
    state.sweep_integral_k2 = normal_or_zero(state.sweep_integral_k2);
    state.population_defect_mw = normal_or_zero(state.population_defect_mw);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.peak_power_mw = normal_or_zero(state.peak_power_mw);
    state.peak_energy_mws = normal_or_zero(state.peak_energy_mws);
    state.peak_time_s = normal_or_zero(state.peak_time_s);
    state.burst_reactivity_dollar = normal_or_zero(state.burst_reactivity_dollar);
    state.coolant_flow_held_m_per_s = normal_or_zero(state.coolant_flow_held_m_per_s);
}

/// One Runge–Kutta step, except in the one step in which the rod stops: there the inserted
/// reactivity has a corner, and the step is taken as two Runge–Kutta steps that meet at it.
fn rk4_through_the_stop(
    reactor: &ReactorSpert3Fuel,
    terms: &Terms,
    t_s: f64,
    y: &[f64; 10],
    dt_s: f64,
) -> [f64; 10] {
    let t_new = t_s + dt_s;
    if terms.ramp * t_s * t_s < terms.rho_total && terms.ramp * t_new * t_new >= terms.rho_total {
        let first = (terms.rho_total / terms.ramp).sqrt() - t_s;
        if first > 0.0 && first < dt_s {
            let at_stop = rk4(reactor, terms, t_s, y, first);
            return rk4(reactor, terms, t_s + first, &at_stop, dt_s - first);
        }
    }
    rk4(reactor, terms, t_s, y, dt_s)
}

fn validate(
    reactor: &ReactorSpert3Fuel,
    state: &ReactorState,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<usize, StepReject> {
    if !dt_s.is_finite()
        || !load.reactivity_inserted_dollar.is_finite()
        || !load.coolant_flow_m_per_s.is_finite()
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
    let band = match reactor.band_of(state.inlet_temperature_k) {
        Some(band) => band,
        None => return Err(StepReject::InletTemperatureOutsideCard),
    };
    if !(load.reactivity_inserted_dollar > 0.0) {
        return Err(StepReject::ReactivityNotPositive);
    }
    if load.coolant_flow_m_per_s < 0.0 {
        return Err(StepReject::NegativeFlow);
    }
    if state.past_first_maximum {
        return Err(StepReject::PastFirstMaximum);
    }
    if !(state.t_s >= 0.0) {
        return Err(StepReject::NegativeTime);
    }
    incoming_state_is_the_cards(reactor, state, load, band)?;
    let terms = terms(reactor, band, state.inlet_temperature_k, load);
    if dt_above_state_ceiling(reactor, &terms, state.t_s, &state_vector(state), dt_s) {
        return Err(StepReject::DtAboveStateCeiling);
    }
    Ok(band)
}

/// Whether `dt_s` is longer than the state ceiling at the state `(t, y)` under one load:
/// [`GATE_PHASE_STEP`] of the time constant of the fastest pole there, or
/// [`GROWTH_PHASE_STEP`] of it when that pole is a growing one.
///
/// The fast part of the equations, linearised about the state, is the power and the two
/// temperatures: `dP'/dt = b P' − (P / l*) (G / (2u) θ_f' + A θ_w')`, with `b = (ρ − 1) / l*`
/// and `u = √(T_in + θ_f)`, and `dθ_f'/dt` and `dθ_w'/dt` the shares of `P'`. With
/// `c = (G (1 − f_w) / (2u C_f) + A f_w / C_w) P / l*`, the poles of the power and the
/// feedback are the roots of `s² − b s + c = 0`: real when `b² ≥ 4c`, the faster of size
/// `|b| / 2 + √(b² / 4 − c)`, the prompt pole, at most `|1 − ρ| / l*`, growing above one dollar;
/// a complex pair of size `√c` otherwise, the feedback pole. The sweep of the water temperature,
/// `2 v / L`, is a pole of its own. The delayed-neutron poles, slower than 3.9 per second, are
/// left out.
fn dt_above_state_ceiling(
    reactor: &ReactorSpert3Fuel,
    terms: &Terms,
    t_s: f64,
    y: &[f64; 10],
    dt_s: f64,
) -> bool {
    let rho_net = net_reactivity(terms, t_s, y);
    let b = (rho_net - 1.0) / terms.l_star;
    let u = (terms.inlet + y[7]).sqrt();
    let c = (terms.doppler * terms.fuel_rate / (2.0 * u) + terms.water_feedback * terms.water_rate)
        * y[0]
        / terms.l_star;
    if !(b.is_finite() && c.is_finite() && terms.sweep.is_finite()) {
        return true;
    }
    let prompt_or_feedback = if b * b < 4.0 * c {
        c.sqrt() * dt_s > reactor.gate_phase_step
    } else {
        let size = 0.5 * b.abs() + (0.25 * b * b - c).sqrt();
        let phi = if b > 0.0 {
            reactor.growth_phase_step
        } else {
            reactor.gate_phase_step
        };
        size * dt_s > phi
    };
    prompt_or_feedback || terms.sweep * dt_s > reactor.gate_phase_step
}

/// The incoming state held to what this card itself writes.
///
/// Every landed step leaves the energy, both temperature rises and the precursor sums at or
/// above zero and the power above it, and every derived field (the inserted and the net reactivity, the power rate, the ledger's
/// defect and residual, the flags) equal, bit for bit, to what the card computes again here
/// from the rest of the state under the burst's reactivity. A state at time zero is the card's start
/// and nothing else. A state that fails any of these was not written by this card.
fn incoming_state_is_the_cards(
    reactor: &ReactorSpert3Fuel,
    state: &ReactorState,
    load: ReactorLoad,
    band: usize,
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
        || state.fuel_temperature_rise_k < 0.0
        || state.water_temperature_rise_k < 0.0
    {
        return Err(StepReject::NegativeState);
    }
    let start = match ReactorState::try_start(reactor, state.inlet_temperature_k) {
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
    if load.reactivity_inserted_dollar != state.burst_reactivity_dollar {
        return Err(StepReject::ReactivityChangedDuringBurst);
    }
    // The flow of a step may be another than the last one's: the derived fields do not depend
    // on it, and the sweep integral is booked per step at the flow of that step.
    let terms = terms(reactor, band, state.inlet_temperature_k, load);
    let derived = derive(reactor, &terms, state);
    // Each held as the landed step wrote it, a value below the normal range as zero.
    if state.population_anchor_mw.to_bits() != normal_or_zero(start.population_anchor_mw).to_bits()
        || state.reactivity_inserted_now_dollar.to_bits() != normal_or_zero(derived.inserted_dollar).to_bits()
        || state.rod_moving != derived.rod_moving
        || state.reactivity_net_dollar.to_bits() != normal_or_zero(derived.net_dollar).to_bits()
        || state.power_rate_mw_per_s.to_bits() != normal_or_zero(derived.power_rate_mw_per_s).to_bits()
        || state.prompt_supercritical != derived.prompt_supercritical
        || state.population_defect_mw.to_bits() != normal_or_zero(derived.population_defect_mw).to_bits()
        || state.residual_conservation_error.to_bits() != normal_or_zero(derived.residual).to_bits()
    {
        return Err(StepReject::InconsistentState);
    }
    // The peak fields are a sample this run has landed: the start, the state itself, or one
    // between them.
    if state.peak_power_mw < state.power_mw
        || state.peak_power_mw < reactor.initial_power_mw
        || !(state.peak_time_s >= 0.0 && state.peak_time_s <= state.t_s)
        || !(state.peak_energy_mws >= 0.0 && state.peak_energy_mws <= state.energy_mws)
        || (state.peak_time_s == state.t_s
            && (state.peak_power_mw.to_bits() != state.power_mw.to_bits()
                || state.peak_energy_mws.to_bits() != state.energy_mws.to_bits()))
        || (state.peak_time_s == 0.0
            && (state.peak_power_mw.to_bits() != reactor.initial_power_mw.to_bits()
                || state.peak_energy_mws != 0.0))
    {
        return Err(StepReject::InconsistentState);
    }
    // The flag was not set and the state it describes is past the first maximum.
    if derived.past_first_maximum {
        return Err(StepReject::PastFirstMaximum);
    }
    Ok(())
}

/// The fields of a state that follow from the rest of it under one load.
struct Derived {
    inserted_dollar: f64,
    net_dollar: f64,
    power_rate_mw_per_s: f64,
    population_defect_mw: f64,
    residual: f64,
    rod_moving: bool,
    past_first_maximum: bool,
    prompt_supercritical: bool,
}

/// The derived fields of a state, computed again from the rest of it by the arithmetic
/// [`commit`] wrote them with, operation for operation, so that a state this card landed
/// gives back its own bits. [`incoming_state_is_the_cards`] holds an incoming state to them.
fn derive(reactor: &ReactorSpert3Fuel, terms: &Terms, state: &ReactorState) -> Derived {
    let y = state_vector(state);
    let power_rate_mw_per_s = rates(reactor, terms, state.t_s, &y)[0];
    let inserted_dollar = inserted(terms, state.t_s);
    let net_dollar = net_reactivity(terms, state.t_s, &y);
    let population = population_mw(state);
    let population_defect_mw = population
        - state.population_anchor_mw
        - reactivity_power_integral(reactor, terms, state) / terms.l_star;
    let scale = if population > state.population_anchor_mw {
        population
    } else {
        state.population_anchor_mw
    };
    let rod_moving = inserted_dollar < terms.rho_total;
    Derived {
        inserted_dollar,
        net_dollar,
        power_rate_mw_per_s,
        population_defect_mw,
        residual: population_defect_mw.abs() / scale,
        rod_moving,
        past_first_maximum: !rod_moving && !(power_rate_mw_per_s > 0.0),
        prompt_supercritical: net_dollar > 1.0,
    }
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
    a.t_s.to_bits() == b.t_s.to_bits()
        && a.power_mw.to_bits() == b.power_mw.to_bits()
        && a.fuel_temperature_rise_k.to_bits() == b.fuel_temperature_rise_k.to_bits()
        && a.water_temperature_rise_k.to_bits() == b.water_temperature_rise_k.to_bits()
        && a.energy_mws.to_bits() == b.energy_mws.to_bits()
        && a.inlet_temperature_k.to_bits() == b.inlet_temperature_k.to_bits()
        && a.power_rate_mw_per_s.to_bits() == b.power_rate_mw_per_s.to_bits()
        && a.reactivity_net_dollar.to_bits() == b.reactivity_net_dollar.to_bits()
        && a.reactivity_inserted_now_dollar.to_bits() == b.reactivity_inserted_now_dollar.to_bits()
        && a.population_anchor_mw.to_bits() == b.population_anchor_mw.to_bits()
        && a.insertion_integral_dollar_mws.to_bits() == b.insertion_integral_dollar_mws.to_bits()
        && a.sweep_integral_k2.to_bits() == b.sweep_integral_k2.to_bits()
        && a.population_defect_mw.to_bits() == b.population_defect_mw.to_bits()
        && a.residual_conservation_error.to_bits() == b.residual_conservation_error.to_bits()
        && a.peak_power_mw.to_bits() == b.peak_power_mw.to_bits()
        && a.peak_energy_mws.to_bits() == b.peak_energy_mws.to_bits()
        && a.peak_time_s.to_bits() == b.peak_time_s.to_bits()
        && a.burst_reactivity_dollar.to_bits() == b.burst_reactivity_dollar.to_bits()
        && a.coolant_flow_held_m_per_s.to_bits() == b.coolant_flow_held_m_per_s.to_bits()
        && a.past_first_maximum == b.past_first_maximum
        && a.prompt_supercritical == b.prompt_supercritical
        && a.rod_moving == b.rod_moving
}

/// `h (f0 + f1) / 2 + h² (f0' − f1') / 12`.
fn hermite_rule(h: f64, f0: f64, f1: f64, d0: f64, d1: f64) -> f64 {
    0.5 * h * (f0 + f1) + h * h * (d0 - d1) / 12.0
}

/// `∫ ρ_ins P dt` over the one step in which the rod stops: the power is the cubic Hermite
/// through the two ends of the step, the integral is taken in two pieces that meet where
/// `K t²` reaches `ρ_0`, each by three-point Gauss–Legendre, exact for these polynomials.
fn straddling_insertion_integral(
    terms: &Terms,
    t_old: f64,
    h: f64,
    p0: f64,
    p1: f64,
    d0: f64,
    d1: f64,
) -> f64 {
    let stop = ((terms.rho_total / terms.ramp).sqrt() - t_old) / h;
    let stop = if stop < 0.0 {
        0.0
    } else if stop > 1.0 {
        1.0
    } else {
        stop
    };
    let power = |s: f64| {
        let s2 = s * s;
        let s3 = s2 * s;
        (2.0 * s3 - 3.0 * s2 + 1.0) * p0
            + (s3 - 2.0 * s2 + s) * h * d0
            + (-2.0 * s3 + 3.0 * s2) * p1
            + (s3 - s2) * h * d1
    };
    let piece = |a: f64, b: f64, moving: bool| {
        let mid = 0.5 * (a + b);
        let half = 0.5 * (b - a);
        let mut sum = 0.0;
        let mut index = 0;
        while index < 3 {
            let s = mid + half * GAUSS_NODE[index];
            let t = t_old + s * h;
            let rho = if moving { terms.ramp * t * t } else { terms.rho_total };
            sum += GAUSS_WEIGHT[index] * rho * power(s);
            index += 1;
        }
        half * h * sum
    };
    piece(0.0, stop, true) + piece(stop, 1.0, false)
}

/// Three-point Gauss–Legendre nodes on [-1, 1]: 0 and ±√(3/5).
const GAUSS_NODE: [f64; 3] = [-0.774_596_669_241_483_4, 0.0, 0.774_596_669_241_483_4];
/// Their weights: 5/9, 8/9, 5/9.
const GAUSS_WEIGHT: [f64; 3] = [5.0 / 9.0, 8.0 / 9.0, 5.0 / 9.0];

/// `ρ_ins P` and its time derivative at `(t, y)` with rates `k`.
fn insertion_integrand(terms: &Terms, t_s: f64, y: &[f64; 10], k: &[f64; 10]) -> (f64, f64) {
    let ramped = terms.ramp * t_s * t_s;
    if ramped < terms.rho_total {
        (ramped * y[0], 2.0 * terms.ramp * t_s * y[0] + ramped * k[0])
    } else {
        (terms.rho_total * y[0], terms.rho_total * k[0])
    }
}

/// `W`, the integral of `ρ P` since the start, from the state and the two running integrals.
fn reactivity_power_integral(
    reactor: &ReactorSpert3Fuel,
    terms: &Terms,
    state: &ReactorState,
) -> f64 {
    let u = (terms.inlet + state.fuel_temperature_rise_k).sqrt();
    let d = state.fuel_temperature_rise_k / (u + terms.sqrt_inlet);
    let doppler_part = terms.doppler / terms.fuel_rate * d * d * (2.0 * u + terms.sqrt_inlet) / 3.0;
    let water_part = terms.water_feedback / terms.water_rate
        * (0.5 * state.water_temperature_rise_k * state.water_temperature_rise_k
            + state.sweep_integral_k2);
    let _ = reactor;
    state.insertion_integral_dollar_mws - doppler_part - water_part
}

fn commit(
    reactor: &ReactorSpert3Fuel,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    terms: &Terms,
    load: ReactorLoad,
    dt_s: f64,
    mut advanced: [f64; 10],
) -> Result<(), StepReject> {
    let saved = *state;
    let mut index = 0;
    while index < 10 {
        if !advanced[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    // A value below the normal range is carried as zero (`crate::normal`):
    // the derived fields below are computed from the values the state
    // carries, as the check of an incoming state computes them again.
    normal_or_zero_each(&mut advanced);
    // The state ceiling at the state the step would land on.
    if dt_above_state_ceiling(reactor, terms, state.t_s + dt_s, &advanced, dt_s) {
        return Err(StepReject::DtAboveStateCeiling);
    }
    let y_old = state_vector(state);
    let t_old = state.t_s;
    let k_old = rates(reactor, terms, t_old, &y_old);
    let t_new = t_old + dt_s;
    let k_new = rates(reactor, terms, t_new, &advanced);
    let ramped_old = terms.ramp * t_old * t_old;
    let ramped_new = terms.ramp * t_new * t_new;
    state.insertion_integral_dollar_mws += if ramped_old < terms.rho_total
        && ramped_new >= terms.rho_total
    {
        straddling_insertion_integral(terms, t_old, dt_s, y_old[0], advanced[0], k_old[0], k_new[0])
    } else {
        let (f0, d0) = insertion_integrand(terms, t_old, &y_old, &k_old);
        let (f1, d1) = insertion_integrand(terms, t_new, &advanced, &k_new);
        hermite_rule(dt_s, f0, f1, d0, d1)
    };
    state.sweep_integral_k2 += terms.sweep
        * hermite_rule(
            dt_s,
            y_old[8] * y_old[8],
            advanced[8] * advanced[8],
            2.0 * y_old[8] * k_old[8],
            2.0 * advanced[8] * k_new[8],
        );
    state.insertion_integral_dollar_mws = normal_or_zero(state.insertion_integral_dollar_mws);
    state.sweep_integral_k2 = normal_or_zero(state.sweep_integral_k2);
    state.t_s = t_new;
    state.power_mw = advanced[0];
    index = 0;
    while index < 6 {
        state.precursor_mw[index] = advanced[1 + index];
        index += 1;
    }
    state.fuel_temperature_rise_k = advanced[7];
    state.water_temperature_rise_k = advanced[8];
    state.energy_mws = advanced[9];
    state.power_rate_mw_per_s = normal_or_zero(k_new[0]);
    state.reactivity_inserted_now_dollar = normal_or_zero(inserted(terms, t_new));
    state.reactivity_net_dollar = normal_or_zero(net_reactivity(terms, t_new, &advanced));
    let population = population_mw(state);
    state.population_defect_mw = population
        - state.population_anchor_mw
        - reactivity_power_integral(reactor, terms, state) / terms.l_star;
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
    state.rod_moving = state.reactivity_inserted_now_dollar < terms.rho_total;
    state.past_first_maximum = !state.rod_moving && !(state.power_rate_mw_per_s > 0.0);
    state.prompt_supercritical = state.reactivity_net_dollar > 1.0;
    state.burst_reactivity_dollar = load.reactivity_inserted_dollar;
    state.coolant_flow_held_m_per_s = load.coolant_flow_m_per_s;
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

/// The ten variables in the order of the Runge–Kutta step: P, C_1 to C_6, θ_f, θ_w, E.
fn state_vector(state: &ReactorState) -> [f64; 10] {
    [
        state.power_mw,
        state.precursor_mw[0],
        state.precursor_mw[1],
        state.precursor_mw[2],
        state.precursor_mw[3],
        state.precursor_mw[4],
        state.precursor_mw[5],
        state.fuel_temperature_rise_k,
        state.water_temperature_rise_k,
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
fn rates(reactor: &ReactorSpert3Fuel, terms: &Terms, t_s: f64, y: &[f64; 10]) -> [f64; 10] {
    let rho_net = net_reactivity(terms, t_s, y);
    let mut out = [0.0; 10];
    let mut delayed = 0.0;
    let mut index = 0;
    while index < 6 {
        let decay = reactor.group_decay_per_s[index];
        delayed += decay * y[1 + index];
        out[1 + index] = reactor.group_fraction[index] / terms.l_star * y[0] - decay * y[1 + index];
        index += 1;
    }
    out[0] = (rho_net - 1.0) / terms.l_star * y[0] + delayed;
    out[7] = terms.fuel_rate * y[0];
    out[8] = terms.water_rate * y[0] - terms.sweep * y[8];
    out[9] = y[0];
    out
}

fn advance(y: &[f64; 10], k: &[f64; 10], scale: f64) -> [f64; 10] {
    let mut out = [0.0; 10];
    let mut index = 0;
    while index < 10 {
        out[index] = y[index] + scale * k[index];
        index += 1;
    }
    out
}

/// Classical fourth-order Runge–Kutta.
fn rk4(
    reactor: &ReactorSpert3Fuel,
    terms: &Terms,
    t_s: f64,
    y: &[f64; 10],
    dt_s: f64,
) -> [f64; 10] {
    let half = 0.5 * dt_s;
    let k1 = rates(reactor, terms, t_s, y);
    let k2 = rates(reactor, terms, t_s + half, &advance(y, &k1, half));
    let k3 = rates(reactor, terms, t_s + half, &advance(y, &k2, half));
    let k4 = rates(reactor, terms, t_s + dt_s, &advance(y, &k3, dt_s));
    let mut out = [0.0; 10];
    let mut index = 0;
    while index < 10 {
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
    frame.vel[0] = state.fuel_temperature_rise_k as f32;
    frame.vel[1] = state.water_temperature_rise_k as f32;
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
        && state.fuel_temperature_rise_k.is_finite()
        && state.water_temperature_rise_k.is_finite()
        && state.energy_mws.is_finite()
        && state.inlet_temperature_k.is_finite()
        && state.power_rate_mw_per_s.is_finite()
        && state.reactivity_net_dollar.is_finite()
        && state.reactivity_inserted_now_dollar.is_finite()
        && state.population_anchor_mw.is_finite()
        && state.insertion_integral_dollar_mws.is_finite()
        && state.sweep_integral_k2.is_finite()
        && state.population_defect_mw.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.peak_power_mw.is_finite()
        && state.peak_energy_mws.is_finite()
        && state.peak_time_s.is_finite()
        && state.burst_reactivity_dollar.is_finite()
        && state.coolant_flow_held_m_per_s.is_finite()
}


/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    PROMPT_KIND,
    ROD_KIND,
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
const ROD_KIND: ExceedanceKind = ExceedanceKind::varying(
    "FLAG_ROD_MOVING",
    FLAG_ROD_MOVING,
    "reactivity_inserted_now_dollar",
    "dollar",
    Side::Below,
    "the reactivity the load inserts: the rod is still moving while the inserted reactivity is below it",
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks.
pub fn exceedances(
    reactor: &ReactorSpert3Fuel,
    before: &ReactorState,
    after: &ReactorState,
    load: ReactorLoad,
    dt_s: f64,
) -> crate::exceedance::Exceedances {
    let _ = (reactor, before, dt_s);
    let mut records = crate::exceedance::Exceedances::new();
    records.push_if(after.prompt_supercritical, Exceedance::fixed(PROMPT_KIND, after.reactivity_net_dollar));
    records.push_if(
        after.rod_moving,
        Exceedance::at(ROD_KIND, load.reactivity_inserted_dollar, after.reactivity_inserted_now_dollar),
    );
    records
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
        ACTIVE_LENGTH_M, BAND_FLOW_MAX_M_PER_S, BAND_FLOW_MIN_M_PER_S,
        BAND_GENERATION_TIME_REDUCED_S, BAND_INLET_MAX_K, BAND_INLET_MIN_K, BAND_RAMP_DOLLAR_PER_S2,
        BAND_REACTIVITY_MAX_DOLLAR, BAND_REACTIVITY_MIN_DOLLAR, BAND_WATER_HEAT_CAPACITY_MWS_PER_K, DOPPLER_STRENGTH_DOLLAR_PER_SQRT_K,
        FLAG_PROMPT_SUPERCRITICAL, FLAG_ROD_MOVING, FLOOR_DT_S, FUEL_HEAT_CAPACITY_MWS_PER_K,
        GATE_PHASE_STEP, GROUP_DECAY_PER_S, GROUP_FRACTION, GROWTH_PHASE_STEP, INITIAL_POWER_MW,
        ReactorLoad, ReactorSpert3Fuel, ReactorState, RESIDUAL_CONSERVATION_LIMIT, StepReject,
        Terms, WATER_FEEDBACK_WEIGHT, WATER_PROMPT_HEATING_FRACTION, advance, commit, derive, rates,
        rk4, rk4_through_the_stop, state_vector, step, terms, validate,
    };
    use crate::frame::LastStateFrame64;

    /// Fahrenheit to kelvin, (F + 459.67) / 1.8, as both E-core cards convert.
    fn kelvin(fahrenheit: f64) -> f64 {
        (fahrenheit + 459.67) / 1.8
    }

    /// Fahrenheit to kelvin as the research scripts and every compared request wrote it,
    /// (F - 32) / 1.8 + 273.15, one or two units in the last place lower.
    fn kelvin_as_compared(fahrenheit: f64) -> f64 {
        (fahrenheit - 32.0) / 1.8 + 273.15
    }

    fn feet(per_second: f64) -> f64 {
        per_second * 0.3048
    }

    fn load(rho: f64, flow_fps: f64) -> ReactorLoad {
        ReactorLoad {
            reactivity_inserted_dollar: rho,
            coolant_flow_m_per_s: feet(flow_fps),
        }
    }

    fn start(fahrenheit: f64) -> (ReactorSpert3Fuel, ReactorState) {
        let reactor = ReactorSpert3Fuel::e_core_low_initial_power();
        let state = ReactorState::try_start(&reactor, kelvin(fahrenheit)).unwrap();
        (reactor, state)
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

    /// A step taken with another stepping method in place of Runge–Kutta, through the card's own commit.
    fn step_with(
        method: fn(&ReactorSpert3Fuel, &Terms, f64, &[f64; 10], f64) -> [f64; 10],
        reactor: &ReactorSpert3Fuel,
        state: &mut ReactorState,
        frame: &mut LastStateFrame64,
        load: ReactorLoad,
        dt_s: f64,
    ) -> Result<(), StepReject> {
        let band = validate(reactor, state, load, dt_s)?;
        let terms = terms(reactor, band, state.inlet_temperature_k, load);
        let advanced = method(reactor, &terms, state.t_s, &state_vector(state), dt_s);
        commit(reactor, state, frame, &terms, load, dt_s, advanced)
    }

    fn euler(
        reactor: &ReactorSpert3Fuel,
        terms: &Terms,
        t_s: f64,
        y: &[f64; 10],
        dt_s: f64,
    ) -> [f64; 10] {
        advance(y, &rates(reactor, terms, t_s, y), dt_s)
    }

    /// Runge–Kutta with the stage weights 1, 2, 2, 1 over 5 in place of over 6.
    fn wrong_weights(
        reactor: &ReactorSpert3Fuel,
        terms: &Terms,
        t_s: f64,
        y: &[f64; 10],
        dt_s: f64,
    ) -> [f64; 10] {
        let half = 0.5 * dt_s;
        let k1 = rates(reactor, terms, t_s, y);
        let k2 = rates(reactor, terms, t_s + half, &advance(y, &k1, half));
        let k3 = rates(reactor, terms, t_s + half, &advance(y, &k2, half));
        let k4 = rates(reactor, terms, t_s + dt_s, &advance(y, &k3, dt_s));
        let mut out = [0.0; 10];
        let mut index = 0;
        while index < 10 {
            out[index] = y[index] + dt_s / 5.0 * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]);
            index += 1;
        }
        out
    }

    /// Runge–Kutta that forgets the water-temperature feedback in the power equation.
    fn no_water_feedback(
        reactor: &ReactorSpert3Fuel,
        terms: &Terms,
        t_s: f64,
        y: &[f64; 10],
        dt_s: f64,
    ) -> [f64; 10] {
        let mut altered = *terms;
        altered.water_feedback = 0.0;
        rk4(reactor, &altered, t_s, y, dt_s)
    }

    struct Run {
        previous: ReactorState,
        last: ReactorState,
        steps: u64,
        largest_residual: f64,
        end: Result<(), StepReject>,
    }

    fn run_with(
        method: fn(&ReactorSpert3Fuel, &Terms, f64, &[f64; 10], f64) -> [f64; 10],
        fahrenheit: f64,
        load: ReactorLoad,
        dt_s: f64,
    ) -> Run {
        let (reactor, mut state) = start(fahrenheit);
        let mut frame = marked_frame();
        let mut previous = state;
        let mut steps = 0;
        let mut largest_residual = 0.0;
        let end = loop {
            let before = state;
            match step_with(method, &reactor, &mut state, &mut frame, load, dt_s) {
                Ok(()) => {
                    previous = before;
                    steps += 1;
                    if state.residual_conservation_error > largest_residual {
                        largest_residual = state.residual_conservation_error;
                    }
                }
                Err(reject) => break Err(reject),
            }
            if steps > 3_000_000 {
                break Ok(());
            }
        };
        Run {
            previous,
            last: state,
            steps,
            largest_residual,
            end,
        }
    }

    fn run_to_first_maximum(fahrenheit: f64, load: ReactorLoad, dt_s: f64) -> Run {
        run_with(rk4_through_the_stop, fahrenheit, load, dt_s)
    }

    /// Peak power, energy at the peak and time of the peak from the cubic Hermite
    /// through the last two landed samples.
    fn hermite_peak(previous: &ReactorState, last: &ReactorState) -> (f64, f64, f64) {
        let h = last.t_s - previous.t_s;
        let (p0, p1) = (previous.power_mw, last.power_mw);
        let (d0, d1) = (previous.power_rate_mw_per_s, last.power_rate_mw_per_s);
        let a = 2.0 * p0 - 2.0 * p1 + h * (d0 + d1);
        let b = -3.0 * p0 + 3.0 * p1 - h * (2.0 * d0 + d1);
        let c = h * d0;
        let disc = (4.0 * b * b - 12.0 * a * c).sqrt();
        let mut s = (-2.0 * b + disc) / (6.0 * a);
        if !(s >= 0.0 && s <= 1.0 && 6.0 * a * s + 2.0 * b < 0.0) {
            s = (-2.0 * b - disc) / (6.0 * a);
        }
        assert!(s >= 0.0 && s <= 1.0);
        let h00 = 2.0 * s * s * s - 3.0 * s * s + 1.0;
        let h10 = s * s * s - 2.0 * s * s + s;
        let h01 = -2.0 * s * s * s + 3.0 * s * s;
        let h11 = s * s * s - s * s;
        let power = h00 * p0 + h10 * h * d0 + h01 * p1 + h11 * h * d1;
        let energy = h00 * previous.energy_mws
            + h10 * h * p0
            + h01 * last.energy_mws
            + h11 * h * p1;
        (power, energy, previous.t_s + s * h)
    }

    fn ceiling() -> f64 {
        ReactorSpert3Fuel::e_core_low_initial_power().gate_dt_s().unwrap()
    }

    #[test]
    fn constants_are_locked() {
        let reactor = ReactorSpert3Fuel::e_core_low_initial_power();
        assert!(reactor.is_valid());
        let mut altered = reactor;
        altered.initial_power_mw = 5.0e-5;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.doppler_strength_dollar_per_sqrt_k *= 1.0 + 1.0e-15;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.band_ramp_dollar_per_s2[1] = 300.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.band_reactivity_max_dollar[2] = 2.0;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.group_decay_per_s[5] = 3.01;
        assert!(!altered.is_valid());
        altered = reactor;
        altered.growth_phase_step = 0.2;
        assert!(!altered.is_valid());
        assert_eq!(GROWTH_PHASE_STEP.to_bits(), 0.045_f64.to_bits());
        // The growing prompt pole of the largest compared reactivity of each band advances at
        // most 0.0434 of its time constant in a step at the static ceiling, under the fraction.
        let mut most: f64 = 0.0;
        for band in 0..3 {
            most = most.max((BAND_REACTIVITY_MAX_DOLLAR[band] - 1.0) * ceiling() / BAND_GENERATION_TIME_REDUCED_S[band]);
        }
        assert!(most > 0.0434 && most < GROWTH_PHASE_STEP, "{most}");
        altered = reactor;
        altered.residual_conservation_limit = 1.0;
        assert!(!altered.is_valid());
        let (_, mut state) = start(70.0);
        let mut frame = marked_frame();
        let before = state;
        assert_eq!(
            step(&altered, &mut state, &mut frame, load(1.0, 0.0), 1.0e-4),
            Err(StepReject::InvalidCard)
        );
        assert_eq!(state, before);
        let sum: f64 = GROUP_FRACTION.iter().sum();
        assert!((sum - 1.0).abs() < 1.0e-12);
        assert!(within(ACTIVE_LENGTH_M, 0.972818, 1.0e-6));
    }

    #[test]
    fn the_start_is_the_equilibrium_and_only_the_three_bands_start() {
        let reactor = ReactorSpert3Fuel::e_core_low_initial_power();
        for (fahrenheit, band) in [(56.0, 0), (92.0, 0), (251.0, 1), (259.0, 1), (498.0, 2), (501.0, 2)] {
            let state = ReactorState::try_start(&reactor, kelvin(fahrenheit)).unwrap();
            assert_eq!(reactor.band_of(state.inlet_temperature_k), Some(band));
            assert_eq!(state.power_mw.to_bits(), INITIAL_POWER_MW.to_bits());
            let t = terms(&reactor, band, state.inlet_temperature_k, load(1.0, 0.0));
            let k = rates(&reactor, &t, 0.0, &state_vector(&state));
            // equilibrium: the power rate and every precursor rate vanish to rounding
            assert!(k[0].abs() < 1.0e-12 * INITIAL_POWER_MW / BAND_GENERATION_TIME_REDUCED_S[band]);
            for i in 0..6 {
                assert!(k[1 + i].abs() < 1.0e-12 * state.precursor_mw[i].max(1.0e-30));
            }
            assert_eq!(state.fuel_temperature_rise_k, 0.0);
            assert_eq!(state.water_temperature_rise_k, 0.0);
            assert!(state.rod_moving && !state.past_first_maximum);
        }
        for fahrenheit in [55.0, 93.0, 150.0, 250.0, 260.0, 400.0, 497.0, 502.0, 600.0] {
            assert!(ReactorState::try_start(&reactor, kelvin(fahrenheit)).is_none());
        }
        assert!(ReactorState::try_start(&reactor, f64::NAN).is_none());
        assert!(ReactorState::try_start(&reactor, f64::INFINITY).is_none());
    }

    #[test]
    fn each_tested_edge_lands_by_either_conversion_and_the_next_double_out_of_a_band_is_refused() {
        let reactor = ReactorSpert3Fuel::e_core_low_initial_power();
        let hot_flow = [(0.0, 0.0), (2.4, 22.0), (4.8, 24.0)];
        for (band, (low, high)) in [(56.0, 92.0), (251.0, 259.0), (498.0, 501.0)].into_iter().enumerate() {
            for fahrenheit in [low, high] {
                for k in [kelvin_as_compared(fahrenheit), kelvin(fahrenheit)] {
                    assert_eq!(reactor.band_of(k), Some(band), "{fahrenheit} degF as {k} K");
                    assert!(ReactorState::try_start(&reactor, k).is_some());
                }
            }
            assert_eq!(reactor.band_of(BAND_INLET_MIN_K[band].next_down()), None);
            assert_eq!(reactor.band_of(BAND_INLET_MAX_K[band].next_up()), None);
            // the flow edges: the tested values, read as a decimal in m/s or converted from ft/s
            let (slow, fast) = hot_flow[band];
            let state = ReactorState::try_start(&reactor, kelvin(low)).unwrap();
            for flow in [slow * 0.3048, fast * 0.3048, BAND_FLOW_MIN_M_PER_S[band], BAND_FLOW_MAX_M_PER_S[band]] {
                let mut s = state;
                let mut frame = marked_frame();
                let rho = 0.5 * (BAND_REACTIVITY_MIN_DOLLAR[band] + BAND_REACTIVITY_MAX_DOLLAR[band]);
                let load = ReactorLoad { reactivity_inserted_dollar: rho, coolant_flow_m_per_s: flow };
                assert!(step(&reactor, &mut s, &mut frame, load, 1.0e-4).is_ok(), "{low} degF {flow} m/s");
            }
            // the next doubles out of the tested flows land too: the flow of the band is no wall
            for flow in [BAND_FLOW_MIN_M_PER_S[band].next_down(), BAND_FLOW_MAX_M_PER_S[band].next_up()] {
                let flow = if flow < 0.0 { 0.0 } else { flow };
                let mut s = state;
                let mut frame = marked_frame();
                let rho = 0.5 * (BAND_REACTIVITY_MIN_DOLLAR[band] + BAND_REACTIVITY_MAX_DOLLAR[band]);
                let load = ReactorLoad { reactivity_inserted_dollar: rho, coolant_flow_m_per_s: flow };
                assert!(step(&reactor, &mut s, &mut frame, load, 1.0e-4).is_ok(), "{low} degF {flow} m/s");
            }
        }
        // the tested decimals in m/s: 1.46304 is 4.8 ft/s and 6.7056 is 22 ft/s
        assert_eq!(BAND_FLOW_MIN_M_PER_S[2], 1.46304);
        assert!(6.7056 <= BAND_FLOW_MAX_M_PER_S[1]);
    }

    #[test]
    fn every_refusal_is_named_and_rolls_back_state_and_frame() {
        let (reactor, fresh) = start(70.0);
        let dt = 1.0e-4;
        let refuse = |state: ReactorState, load: ReactorLoad, dt_s: f64, expected: StepReject| {
            let mut s = state;
            let mut frame = marked_frame();
            let frame_before = frame.to_bytes();
            assert_eq!(step(&reactor, &mut s, &mut frame, load, dt_s), Err(expected));
            // compared field by field through Debug so that a NaN planted in the state still compares
            assert_eq!(std::format!("{:?}", s), std::format!("{:?}", state));
            assert_eq!(frame.to_bytes(), frame_before);
        };
        refuse(fresh, load(1.0, 0.0), f64::NAN, StepReject::NonFiniteInput);
        refuse(fresh, load(f64::INFINITY, 0.0), dt, StepReject::NonFiniteInput);
        refuse(
            fresh,
            ReactorLoad { reactivity_inserted_dollar: 1.0, coolant_flow_m_per_s: f64::NAN },
            dt,
            StepReject::NonFiniteInput,
        );
        let mut bad = fresh;
        bad.precursor_mw[3] = f64::NAN;
        refuse(bad, load(1.0, 0.0), dt, StepReject::NonFiniteState);
        refuse(fresh, load(1.0, 0.0), 0.0, StepReject::NonPositiveDt);
        refuse(fresh, load(1.0, 0.0), -dt, StepReject::NonPositiveDt);
        let mut between = fresh;
        between.inlet_temperature_k = kelvin(400.0);
        refuse(between, load(1.0, 14.0), dt, StepReject::InletTemperatureOutsideCard);
        // a reactivity of zero or less inserts nothing, in every band
        refuse(fresh, load(0.0, 0.0), dt, StepReject::ReactivityNotPositive);
        refuse(fresh, load(-0.0, 0.0), dt, StepReject::ReactivityNotPositive);
        refuse(fresh, load(-1.0, 0.0), dt, StepReject::ReactivityNotPositive);
        let (_, hot) = start(500.0);
        refuse(hot, load(-f64::MIN_POSITIVE, 14.0), dt, StepReject::ReactivityNotPositive);
        let (_, warm) = start(259.0);
        refuse(warm, load(0.0, 14.0), dt, StepReject::ReactivityNotPositive);
        // a flow below zero, in every band; zero written with either sign is no flow
        refuse(fresh, load(1.0, -1.0), dt, StepReject::NegativeFlow);
        refuse(
            hot,
            ReactorLoad { reactivity_inserted_dollar: 1.0, coolant_flow_m_per_s: -f64::MIN_POSITIVE },
            dt,
            StepReject::NegativeFlow,
        );
        refuse(warm, load(1.0, -14.0), dt, StepReject::NegativeFlow);
        // the old walls of reactivity and flow, and past them, land in every band: the cold band
        // with flow, the hot bands with none and with more than was tested, each band's
        // reactivity below and above its span
        for (state, rho, flow) in [
            (fresh, 0.767, 0.0),
            (fresh, 1.212, 0.0),
            (fresh, 1.0, 14.0),
            (fresh, 0.1, 100.0),
            (hot, 1.0, 0.0),
            (hot, 1.0, 2.3),
            (hot, 1.0, 24.1),
            (hot, 0.80, 14.0),
            (hot, 2.0, 50.0),
            (warm, 1.0, 2.3),
            (warm, 1.0, 24.0),
            (warm, 0.90, 14.0),
            (warm, 1.22, 14.0),
        ] {
            let mut s = state;
            let mut frame = marked_frame();
            step(&reactor, &mut s, &mut frame, load(rho, flow), dt).unwrap();
            assert_eq!(frame.proof, [0xA5; 16]);
        }
        refuse(between, load(-1.0, -1.0), dt, StepReject::InletTemperatureOutsideCard);
        let mut done = fresh;
        done.past_first_maximum = true;
        refuse(done, load(1.0, 0.0), dt, StepReject::PastFirstMaximum);
        // a clock below zero; the order: the load before the first maximum, the first maximum
        // before the clock
        let mut early = fresh;
        early.t_s = -0.5 * dt;
        refuse(early, load(1.0, 0.0), dt, StepReject::NegativeTime);
        refuse(early, load(0.0, 0.0), dt, StepReject::ReactivityNotPositive);
        refuse(done, load(1.0, -1.0), dt, StepReject::NegativeFlow);
        let mut early_done = early;
        early_done.past_first_maximum = true;
        refuse(early_done, load(1.0, 0.0), dt, StepReject::PastFirstMaximum);
        // a state with a power and an energy far past the old 1000 MW and 30 MW s is no longer
        // refused for them; written by hand it is still not the card's start
        let mut powerful = fresh;
        powerful.power_mw = 2000.0;
        refuse(powerful, load(1.0, 0.0), dt, StepReject::InconsistentState);
        let mut spent = fresh;
        spent.energy_mws = 31.0;
        refuse(spent, load(1.0, 0.0), dt, StepReject::InconsistentState);
        // a trial step too long for the state, from a state the card itself landed: the public
        // step applies no static ceiling, and one step of 0.2 s or of 0.5 s from the rise of a
        // 1.2 dollar cold burst, at 5 MW, is far over the state ceiling
        let mut rising = fresh;
        let mut frame = marked_frame();
        for _ in 0..400 {
            step(&reactor, &mut rising, &mut frame, load(1.2, 0.0), ceiling()).unwrap();
        }
        assert!(rising.power_mw > 5.0 && rising.power_mw < 6.0 && !rising.past_first_maximum);
        refuse(rising, load(1.2, 0.0), 0.2, StepReject::DtAboveStateCeiling);
        refuse(rising, load(1.2, 0.0), 0.5, StepReject::DtAboveStateCeiling);
        refuse(rising, load(1.2, 0.0), f64::MAX, StepReject::DtAboveStateCeiling);
        // the landing past the state ceiling is named at the landing: a fuel temperature that
        // would take the net reactivity far below zero makes the prompt pole too fast
        let t = terms(&reactor, 0, rising.inlet_temperature_k, load(1.2, 0.0));
        let mut advanced = rk4_through_the_stop(&reactor, &t, rising.t_s, &state_vector(&rising), ceiling());
        advanced[7] = 1.0e5;
        let mut landing = rising;
        let mut f = marked_frame();
        assert_eq!(
            commit(&reactor, &mut landing, &mut f, &t, load(1.2, 0.0), ceiling(), advanced),
            Err(StepReject::DtAboveStateCeiling)
        );
        assert_eq!(landing, rising);
        assert_eq!(f.to_bytes(), marked_frame().to_bytes());
    }

    /// The state of a cold burst of 1.1 dollars after `steps` steps at the ceiling, with its frame.
    /// The rod stops at 60.6 ms, in step 141.
    fn cold_burst(steps: usize) -> (ReactorSpert3Fuel, ReactorState, LastStateFrame64) {
        let (reactor, mut state) = start(70.0);
        let mut frame = marked_frame();
        for _ in 0..steps {
            step(&reactor, &mut state, &mut frame, load(1.1, 0.0), ceiling()).unwrap();
        }
        (reactor, state, frame)
    }

    /// One step from `state` under `load` is refused as `expected`, and the state and the frame
    /// are exactly as they were.
    fn refused_and_kept(
        reactor: &ReactorSpert3Fuel,
        state: ReactorState,
        frame: LastStateFrame64,
        load: ReactorLoad,
        expected: StepReject,
    ) {
        let mut s = state;
        let mut f = frame;
        assert_eq!(step(reactor, &mut s, &mut f, load, ceiling()), Err(expected));
        assert_eq!(std::format!("{:?}", s), std::format!("{:?}", state));
        assert_eq!(f.to_bytes(), frame.to_bytes());
    }

    #[test]
    fn a_reactivity_changed_during_the_burst_is_refused_by_name_and_the_state_is_kept() {
        // after the rod has stopped: the two requests of the second scoring (V-25, holes.json).
        // 400 steps at 1.1 dollars, then 1.2: the inserted reactivity would jump 0.1 dollar in
        // one step. Then 0.9: the power would turn and be sealed as a first maximum of 0.049 MW.
        let (reactor, stopped, frame) = cold_burst(400);
        assert!(!stopped.rod_moving && stopped.reactivity_inserted_now_dollar == 1.1);
        assert_eq!(stopped.burst_reactivity_dollar, 1.1);
        refused_and_kept(&reactor, stopped, frame, load(1.2, 0.0), StepReject::ReactivityChangedDuringBurst);
        refused_and_kept(&reactor, stopped, frame, load(0.9, 0.0), StepReject::ReactivityChangedDuringBurst);
        // by the last bit
        let next = f64::from_bits(1.1_f64.to_bits() + 1);
        refused_and_kept(&reactor, stopped, frame, load(next, 0.0), StepReject::ReactivityChangedDuringBurst);
        // while the rod is still moving, and on the second step of the burst
        for steps in [1, 50, 140] {
            let (reactor, moving, frame) = cold_burst(steps);
            assert!(moving.rod_moving);
            refused_and_kept(&reactor, moving, frame, load(1.2, 0.0), StepReject::ReactivityChangedDuringBurst);
            refused_and_kept(&reactor, moving, frame, load(0.9, 0.0), StepReject::ReactivityChangedDuringBurst);
        }
        // a reactivity outside the band's span is no other name: it is not the burst's
        refused_and_kept(&reactor, stopped, frame, load(1.3, 0.0), StepReject::ReactivityChangedDuringBurst);
        // a reactivity of zero or less is named as that first
        refused_and_kept(&reactor, stopped, frame, load(0.0, 0.0), StepReject::ReactivityNotPositive);
        // the same load goes on to the first maximum, and it is the maximum of the run that
        // never stopped to ask: 808 steps, as the second scoring's run of 1.1 dollars alone
        let (mut state, mut frame) = (stopped, frame);
        let mut steps = 400;
        while step(&reactor, &mut state, &mut frame, load(1.1, 0.0), ceiling()).is_ok() {
            steps += 1;
        }
        let whole = run_to_first_maximum(70.0, load(1.1, 0.0), ceiling());
        assert_eq!(steps, 808);
        assert_eq!(whole.steps, 808);
        assert_eq!(state, whole.last);
        assert_eq!(state.peak_power_mw, 77.87511026749317);
        assert_eq!(state.peak_time_s, 0.34701000000000076);
        // The second scoring's own request, at 70 degF written (F - 32) / 1.8 + 273.15, one unit
        // in the last place lower: its peak, to the last bit.
        let reactor = ReactorSpert3Fuel::e_core_low_initial_power();
        let mut compared = ReactorState::try_start(&reactor, kelvin_as_compared(70.0)).unwrap();
        let mut f = marked_frame();
        while step(&reactor, &mut compared, &mut f, load(1.1, 0.0), ceiling()).is_ok() {}
        assert_eq!(compared.peak_power_mw, 77.87511026749316);
        assert_eq!(compared.peak_time_s, 0.34701000000000076);
    }

    /// A flow changed during the burst: each step is computed at its own flow, and the sweep
    /// integral of the ledger is booked per step at that step's flow. The ledger closes on every
    /// step, in both hot bands and in the cold band, which was tested with no flow.
    #[test]
    fn a_flow_changed_during_the_burst_is_computed_at_each_step_with_the_ledger_closed() {
        for (fahrenheit, high, low) in [(259.0, 22.0, 2.4), (500.0, 24.0, 4.8), (70.0, 0.0, 20.0)] {
            let (reactor, mut state) = start(fahrenheit);
            let mut frame = marked_frame();
            for _ in 0..50 {
                step(&reactor, &mut state, &mut frame, load(1.1, high), ceiling()).unwrap();
            }
            assert_eq!(state.coolant_flow_held_m_per_s, feet(high));
            // Alternate the flow every 25 steps to the first maximum.
            let mut steps = 0;
            let mut worst = 0.0_f64;
            let end = loop {
                let flow = if (steps / 25) % 2 == 0 { low } else { high };
                match step(&reactor, &mut state, &mut frame, load(1.1, flow), ceiling()) {
                    Ok(()) => {
                        assert_eq!(state.coolant_flow_held_m_per_s, feet(flow));
                        worst = worst.max(state.residual_conservation_error);
                        steps += 1;
                    }
                    Err(end) => break end,
                }
            };
            assert_eq!(end, StepReject::PastFirstMaximum, "{fahrenheit}");
            assert!(worst < RESIDUAL_CONSERVATION_LIMIT, "{fahrenheit}: {worst:e}");
            let steady_low = run_to_first_maximum(fahrenheit, load(1.1, low), ceiling()).last;
            let steady_high = run_to_first_maximum(fahrenheit, load(1.1, high), ceiling()).last;
            println!(
                "{fahrenheit} degF 1.1 dollars, flow {high} and {low} ft/s in turn: peak {} MW at {} s, worst residual {worst:e}; held at {low}: {} MW, at {high}: {} MW",
                state.peak_power_mw, state.peak_time_s, steady_low.peak_power_mw, steady_high.peak_power_mw
            );
            // The peak lies between those of the two flows held through.
            let (least, most) = if steady_low.peak_power_mw < steady_high.peak_power_mw {
                (steady_low.peak_power_mw, steady_high.peak_power_mw)
            } else {
                (steady_high.peak_power_mw, steady_low.peak_power_mw)
            };
            assert!(state.peak_power_mw >= least * (1.0 - 1.0e-9) && state.peak_power_mw <= most * (1.0 + 1.0e-9));
        }
        // A step integrated at one flow and booked at another is named by the ledger.
        let (reactor, mut state) = start(500.0);
        let mut frame = marked_frame();
        for _ in 0..600 {
            step(&reactor, &mut state, &mut frame, load(1.1, 24.0), ceiling()).unwrap();
        }
        let integrated = terms(&reactor, 2, state.inlet_temperature_k, load(1.1, 24.0));
        let booked = terms(&reactor, 2, state.inlet_temperature_k, load(1.1, 2400.0));
        let advanced = rk4_through_the_stop(&reactor, &integrated, state.t_s, &state_vector(&state), ceiling());
        let mut tried = state;
        let mut tried_frame = frame;
        let got = commit(&reactor, &mut tried, &mut tried_frame, &booked, load(1.1, 2400.0), ceiling(), advanced);
        assert!(
            got == Err(StepReject::SpontaneousGeneration) || got == Err(StepReject::FirstLawBreach)
                || got == Err(StepReject::DtAboveStateCeiling),
            "{got:?}"
        );
        assert_eq!(tried, state);
        assert_eq!(tried_frame.to_bytes(), frame.to_bytes());
        // zero written with either sign is the same flow
        let (reactor, mut state, mut frame) = cold_burst(10);
        let minus = ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: -0.0 };
        let mut other = state;
        let mut other_frame = frame;
        step(&reactor, &mut state, &mut frame, load(1.1, 0.0), ceiling()).unwrap();
        step(&reactor, &mut other, &mut other_frame, minus, ceiling()).unwrap();
        other.coolant_flow_held_m_per_s = 0.0;
        assert_eq!(std::format!("{:?}", state), std::format!("{:?}", other));
    }

    /// Three answers of the held-back run of 2026-10-09 (`runs_v2/held_back/` of the research
    /// record, the binary of SHA-256 be8ee098...), made before the card refused a changed load
    /// or held an incoming state to its own arithmetic: inlet K, reactivity, flow m/s and step
    /// as 64-bit patterns, landed steps, whether the run ended at the refusal, the 23 numbers of
    /// the final state in the order the door publishes them, and the first 48 bytes of the last
    /// frame (the 16 after them are the door's proof).
    const SEALED: [(u32, [u64; 4], u64, bool, [u64; 23], &str); 3] = [
        (
            73,
            [0x40724982d82d82d8, 0x3fefb080640d36d2, 0x0000000000000000, 0x3f3c2e33eff19504],
            2858,
            true,
            [
                0x3ff3a9bcfd4bf0b9, 0x40228cab7ce4f581, 0x40077715155d33d0, 0xbf7af4c632680000,
                0x405b3c7d6662f19c, 0x3fc7decc8122c70f, 0x3fec307ceb39bd37, 0x3fefb080640d36d2,
                0x40724982d82d82d8, 0x4047f67eef33cdb2, 0x40711376f0347767, 0x406f061ac75a2274,
                0x408024b7eb113168, 0x40634a50d9ec5dec, 0x4038fe759dd6257d, 0x3fb0bc8265f8b7a5,
                0x40073cc93a5ba198, 0x0000000000000000, 0x3e322fc000000000, 0x3d8d3943b54ccea7,
                0x40228cab8e913c00, 0x40076eea262418ab, 0x3ff3a7fa1a0cf1a0,
            ],
            "b9f04bfdbca9f33f5c651441a9b83b40e783613febe3d94264f63e3e5c65144132a6d7bb1eca692c0000000000000000",
        ),
        (
            54,
            [0x4080a04fa4fa4fa4, 0x3fedb209b81e6953, 0x4011119ce075f6fd, 0x3f3c2e33eff19504],
            8276,
            true,
            [
                0x400c782d384772fe, 0x401cdc9307db4ad8, 0x401f274cdeda6d23, 0xbf1bfe2c90800000,
                0x40721482f51441eb, 0x3facb0c37f21b67e, 0x3fe702139ca09180, 0x3fedb209b81e6953,
                0x4080a04fa4fa4fa4, 0x405e3ab15199af48, 0x40855ec02e1cd795, 0x4082bc493fed189a,
                0x4092071a7a24ba92, 0x406ea15ebbf5ed50, 0x403a559ba51b9302, 0x3faffc3db8c2618d,
                0x401ce8f18ff3f3ce, 0x3f950e36c2bff9fb, 0x3e34668000000000, 0x3d7d750623e55f26,
                0x401cdc9307db4ad8, 0x401f274cdeda6d23, 0x400c782d384772fe,
            ],
            "fe7247382d780c4098e4e640673af9409d10383f18a490431c86653d98e4e64065f1dfb831a8eb2b0000000000000000",
        ),
        (
            70,
            [0x4078ad1111111110, 0x3ff34ed00594f9cd, 0x4011119ce075f6fd, 0x3f3c2e33eff19504],
            533,
            false,
            [
                0x3fcd56191148fe16, 0x40743d8b50ce538a, 0x401b9bd3e400e0b5, 0x407994b0cc8533c0,
                0x407005dac7482148, 0x3fd9ff8afc59d8a7, 0x3fefc3f413b5d432, 0x3ff34ed00594f9cd,
                0x4078ad1111111110, 0x405bdc48c93ccf04, 0x4083eb2d26cd30a9, 0x40825a2b60a655ba,
                0x4093ba291fbb704b, 0x407bdca7b1d7e92b, 0x4058ed8a82954e44, 0x3fb0818f071c96b1,
                0x4020a8850195a6f7, 0x3f89013eaa34d829, 0xbeec964cf0000000, 0x3e30e1c3ee635d9c,
                0x40743d8b50ce538a, 0x401b9bd3e400e0b5, 0x3fcd56191148fe16,
            ],
            "16fe48111956cd3f5beca1439fdedc40a11f7e3fd62e804358fccf3e5beca14386a5cc431f0e87310000000000000000",
        ),
    ];

    #[test]
    fn a_run_under_one_load_is_what_it_was_before_the_new_refusals_in_every_bit() {
        for (test, [inlet, rho, flow, dt], landed, to_refusal, sealed, frame_hex) in SEALED {
            let reactor = ReactorSpert3Fuel::e_core_low_initial_power();
            let mut state = ReactorState::try_start(&reactor, f64::from_bits(inlet)).unwrap();
            let held = ReactorLoad {
                reactivity_inserted_dollar: f64::from_bits(rho),
                coolant_flow_m_per_s: f64::from_bits(flow),
            };
            let mut frame = marked_frame();
            let mut steps = 0;
            let end = loop {
                if !to_refusal && steps == landed {
                    break Ok(());
                }
                match step(&reactor, &mut state, &mut frame, held, f64::from_bits(dt)) {
                    Ok(()) => steps += 1,
                    Err(reject) => break Err(reject),
                }
            };
            assert_eq!(steps, landed, "test {test}");
            assert_eq!(end, if to_refusal { Err(StepReject::PastFirstMaximum) } else { Ok(()) });
            let got = [
                state.t_s, state.power_mw, state.energy_mws, state.power_rate_mw_per_s,
                state.fuel_temperature_rise_k, state.water_temperature_rise_k,
                state.reactivity_net_dollar, state.reactivity_inserted_now_dollar,
                state.inlet_temperature_k, state.precursor_mw[0], state.precursor_mw[1],
                state.precursor_mw[2], state.precursor_mw[3], state.precursor_mw[4],
                state.precursor_mw[5], state.population_anchor_mw,
                state.insertion_integral_dollar_mws, state.sweep_integral_k2,
                state.population_defect_mw, state.residual_conservation_error,
                state.peak_power_mw, state.peak_energy_mws, state.peak_time_s,
            ];
            for (index, value) in got.iter().enumerate() {
                assert_eq!(value.to_bits(), sealed[index], "test {test}, field {index}");
            }
            let bytes = frame.to_bytes();
            let mut hex = std::string::String::new();
            for byte in &bytes[..48] {
                hex.push_str(&std::format!("{byte:02x}"));
            }
            assert_eq!(hex, frame_hex, "test {test}");
            assert_eq!(&bytes[48..], &[0xA5; 16]);
        }
    }

    #[test]
    fn an_incoming_state_the_card_did_not_write_is_refused_by_name_and_kept() {
        let (reactor, honest, frame) = cold_burst(400);
        let held = load(1.1, 0.0);
        let refuse = |edit: &dyn Fn(&mut ReactorState), expected: StepReject| {
            let mut forged = honest;
            edit(&mut forged);
            refused_and_kept(&reactor, forged, frame, held, expected);
        };
        // the two states of the second scoring (V-25, kernel_state_driver): energy of -1 MW s,
        // which landed and gave a peak energy of 2.214 MW s for 3.215; and a peak power of
        // 1e9 MW, which landed, was written to the frame and stayed
        refuse(&|s| s.energy_mws = -1.0, StepReject::NegativeState);
        refuse(&|s| s.peak_power_mw = 1.0e9, StepReject::InconsistentState);
        // below zero
        refuse(&|s| s.energy_mws = -f64::MIN_POSITIVE, StepReject::NegativeState);
        refuse(&|s| s.fuel_temperature_rise_k = -1.0e-9, StepReject::NegativeState);
        refuse(&|s| s.water_temperature_rise_k = -1.0e-9, StepReject::NegativeState);
        refuse(&|s| s.precursor_mw[5] = -1.0e-9, StepReject::NegativeState);
        refuse(&|s| s.power_mw = 0.0, StepReject::NegativeState);
        refuse(&|s| s.power_mw = -s.power_mw, StepReject::NegativeState);
        // beyond the old limits of 30 MW s and 1000 MW, written by hand: not the card's state
        refuse(&|s| s.energy_mws = 31.0, StepReject::InconsistentState);
        refuse(&|s| s.power_mw = 1000.5, StepReject::InconsistentState);
        refuse(&|s| s.peak_power_mw = 1000.5, StepReject::InconsistentState);
        // not finite
        refuse(&|s| s.peak_energy_mws = f64::NAN, StepReject::NonFiniteState);
        refuse(&|s| s.peak_time_s = f64::INFINITY, StepReject::NonFiniteState);
        refuse(&|s| s.energy_mws = f64::NEG_INFINITY, StepReject::NonFiniteState);
        refuse(&|s| s.burst_reactivity_dollar = f64::NAN, StepReject::NonFiniteState);
        refuse(&|s| s.coolant_flow_held_m_per_s = f64::INFINITY, StepReject::NonFiniteState);
        // the peak fields are not a sample of this run
        refuse(&|s| s.peak_power_mw = 0.5 * s.power_mw, StepReject::InconsistentState);
        refuse(&|s| s.peak_power_mw = 500.0, StepReject::InconsistentState);
        refuse(&|s| s.peak_time_s = s.t_s + 1.0e-3, StepReject::InconsistentState);
        refuse(&|s| s.peak_time_s = -1.0e-3, StepReject::InconsistentState);
        refuse(&|s| s.peak_energy_mws = 2.0 * s.energy_mws, StepReject::InconsistentState);
        refuse(&|s| s.peak_energy_mws = -1.0, StepReject::InconsistentState);
        refuse(
            &|s| {
                s.peak_power_mw = 500.0;
                s.peak_time_s = 0.5 * s.t_s;
                s.peak_energy_mws = 2.0 * s.energy_mws;
            },
            StepReject::InconsistentState,
        );
        // the clock does not agree with the ramp: the rod stopped at 60.6 ms
        refuse(&|s| s.t_s = 0.05, StepReject::InconsistentState);
        refuse(&|s| s.t_s = 0.0, StepReject::InconsistentState);
        refuse(&|s| s.t_s = -0.0, StepReject::InconsistentState);
        refuse(&|s| s.t_s = -1.0e-3, StepReject::NegativeTime);
        refuse(&|s| s.rod_moving = true, StepReject::InconsistentState);
        refuse(&|s| s.reactivity_inserted_now_dollar = 1.0, StepReject::InconsistentState);
        // the load this burst was started with, rewritten in the state to match a new one
        {
            let mut forged = honest;
            forged.burst_reactivity_dollar = 1.2;
            refused_and_kept(&reactor, forged, frame, load(1.2, 0.0), StepReject::InconsistentState);
            refused_and_kept(&reactor, forged, frame, held, StepReject::ReactivityChangedDuringBurst);
        }
        // The same rewriting while the rod is still moving cannot be told from the state, which
        // holds nothing of the target yet but that field; what then runs is, bit for bit, the
        // one ramp to the new target from the start, and no jump.
        {
            let (_, moving, moving_frame) = cold_burst(50);
            let mut renamed = moving;
            renamed.burst_reactivity_dollar = 1.2;
            let mut f = moving_frame;
            while step(&reactor, &mut renamed, &mut f, load(1.2, 0.0), ceiling()).is_ok() {}
            assert_eq!(renamed, run_to_first_maximum(70.0, load(1.2, 0.0), ceiling()).last);
            // a target at or under what is already in is not on the ramp
            let mut under = moving;
            under.burst_reactivity_dollar = 0.1;
            let low = ReactorLoad { reactivity_inserted_dollar: 0.1, coolant_flow_m_per_s: 0.0 };
            refused_and_kept(&reactor, under, moving_frame, low, StepReject::InconsistentState);
            under.burst_reactivity_dollar = 0.8;
            assert!(moving.reactivity_inserted_now_dollar < 0.8);
            let mut f = moving_frame;
            step(&reactor, &mut under, &mut f, load(0.8, 0.0), ceiling()).unwrap();
        }
        // After the rod has stopped the equations hold no time. A clock moved later, the rest of
        // the state as it was, cannot be told from the state and steps on, to the same peak power
        // at a time later by the same amount; a clock moved back before the stop is off the ramp.
        {
            let mut later = honest;
            later.t_s += 1.0;
            let mut f = frame;
            while step(&reactor, &mut later, &mut f, held, ceiling()).is_ok() {}
            let whole = run_to_first_maximum(70.0, held, ceiling()).last;
            assert_eq!(later.peak_power_mw.to_bits(), whole.peak_power_mw.to_bits());
            assert!((later.peak_time_s - whole.peak_time_s - 1.0).abs() < 1.0e-9);
            // An energy inside the limits that is not the fuel's: written into the state alone
            // it no longer is the peak sample the state carries for its own time, and is named;
            // written into the state and the peak sample together it is not, and steps on.
            refuse(&|s| s.energy_mws = 29.9999, StepReject::InconsistentState);
            let mut richer = honest;
            richer.energy_mws = 29.9999;
            richer.peak_energy_mws = 29.9999;
            let mut f = frame;
            step(&reactor, &mut richer, &mut f, held, ceiling()).unwrap();
        }
        // a derived field that is not the card's arithmetic on the rest of the state
        refuse(&|s| s.power_rate_mw_per_s *= 1.0 + 1.0e-15, StepReject::InconsistentState);
        refuse(&|s| s.reactivity_net_dollar = 1.2, StepReject::InconsistentState);
        refuse(&|s| s.prompt_supercritical = !s.prompt_supercritical, StepReject::InconsistentState);
        refuse(&|s| s.population_anchor_mw *= 1.0 + 1.0e-15, StepReject::InconsistentState);
        refuse(&|s| s.population_anchor_mw = 0.0, StepReject::InconsistentState);
        refuse(&|s| s.population_defect_mw = 0.0, StepReject::InconsistentState);
        refuse(&|s| s.residual_conservation_error = 0.0, StepReject::InconsistentState);
        refuse(&|s| s.insertion_integral_dollar_mws = 0.0, StepReject::InconsistentState);
        refuse(&|s| s.inlet_temperature_k = kelvin(80.0), StepReject::InconsistentState);
        refuse(&|s| s.fuel_temperature_rise_k += 1.0, StepReject::InconsistentState);
        // a state at time zero is the card's start and nothing else
        let (_, fresh) = start(70.0);
        for edit in [
            (|s: &mut ReactorState| s.energy_mws = 1.0) as fn(&mut ReactorState),
            |s| s.peak_power_mw = 500.0,
            |s| s.power_mw = 1.0,
            |s| s.fuel_temperature_rise_k = 10.0,
            |s| s.rod_moving = false,
            |s| s.burst_reactivity_dollar = 1.1,
            |s| s.population_anchor_mw = 0.0,
            |s| s.power_rate_mw_per_s = 1.0,
        ] {
            let mut forged = fresh;
            edit(&mut forged);
            refused_and_kept(&reactor, forged, marked_frame(), held, StepReject::InconsistentState);
        }
        // a finished run with its flag cleared is still past the first maximum; before, one more
        // step landed
        let finished = run_to_first_maximum(70.0, held, ceiling()).last;
        assert!(finished.past_first_maximum);
        let mut cleared = finished;
        cleared.past_first_maximum = false;
        refused_and_kept(&reactor, cleared, frame, held, StepReject::PastFirstMaximum);
        cleared.power_rate_mw_per_s = 1.0;
        refused_and_kept(&reactor, cleared, frame, held, StepReject::InconsistentState);
        // and the honest state steps on
        let mut state = honest;
        let mut f = frame;
        step(&reactor, &mut state, &mut f, held, ceiling()).unwrap();
    }

    #[test]
    fn every_landed_state_gives_back_its_own_derived_fields() {
        // the incoming-state check recomputes what `commit` wrote; over whole runs at the
        // ceiling, at a third of it and at the floor the two are the same bits at every step
        for (fahrenheit, rho, flow) in SPAN {
            for dt in [ceiling(), ceiling() / 3.0] {
                let (reactor, mut state) = start(fahrenheit);
                let mut frame = marked_frame();
                let held = load(rho, flow);
                let band = reactor.band_of(state.inlet_temperature_k).unwrap();
                let t = terms(&reactor, band, state.inlet_temperature_k, held);
                let mut steps = 0;
                while step(&reactor, &mut state, &mut frame, held, dt).is_ok() {
                    steps += 1;
                    let d = derive(&reactor, &t, &state);
                    assert_eq!(d.power_rate_mw_per_s.to_bits(), state.power_rate_mw_per_s.to_bits());
                    assert_eq!(d.net_dollar.to_bits(), state.reactivity_net_dollar.to_bits());
                    assert_eq!(d.inserted_dollar.to_bits(), state.reactivity_inserted_now_dollar.to_bits());
                    assert_eq!(d.population_defect_mw.to_bits(), state.population_defect_mw.to_bits());
                    assert_eq!(d.residual.to_bits(), state.residual_conservation_error.to_bits());
                    assert_eq!(d.rod_moving, state.rod_moving);
                    assert_eq!(d.past_first_maximum, state.past_first_maximum);
                    assert_eq!(d.prompt_supercritical, state.prompt_supercritical);
                    if steps > 200_000 {
                        break;
                    }
                }
                assert!(state.past_first_maximum, "{fahrenheit} {rho} {flow} {dt}");
            }
        }
        let (reactor, mut state) = start(70.0);
        let mut frame = marked_frame();
        for _ in 0..2000 {
            step(&reactor, &mut state, &mut frame, load(1.2, 0.0), FLOOR_DT_S).unwrap();
        }
    }

    #[test]
    fn the_reactivity_goes_in_as_the_square_of_time_and_then_holds() {
        for (fahrenheit, band, flow) in [(70.0, 0usize, 0.0), (259.0, 1, 14.0), (500.0, 2, 14.0)] {
            let (reactor, mut state) = start(fahrenheit);
            let mut frame = marked_frame();
            let rho = 1.05;
            let dt = 1.0e-4;
            let k = BAND_RAMP_DOLLAR_PER_S2[band];
            let end = (rho / k).sqrt();
            let mut was_moving = true;
            for _ in 0..1200 {
                step(&reactor, &mut state, &mut frame, load(rho, flow), dt).unwrap();
                let expected = if k * state.t_s * state.t_s < rho { k * state.t_s * state.t_s } else { rho };
                assert_eq!(state.reactivity_inserted_now_dollar.to_bits(), expected.to_bits());
                assert_eq!(state.rod_moving, state.t_s < end);
                assert_eq!(frame.flags & FLAG_ROD_MOVING != 0, state.rod_moving);
                assert!(was_moving || !state.rod_moving);
                was_moving = state.rod_moving;
                // feedback is still negligible: the net reactivity is the inserted one
                assert!((state.reactivity_net_dollar - expected).abs() < 1.0e-3);
            }
            assert!(!state.rod_moving);
            // 70 degF: 1.05 dollars are in after 59 ms; 500 degF: after 76 ms
            assert!(end > 0.055 && end < 0.080);
        }
    }

    #[test]
    fn heat_goes_where_the_equations_put_it() {
        // no flow: the water keeps its share, the fuel keeps the rest
        let run = run_to_first_maximum(70.0, load(1.05, 0.0), ceiling());
        assert_eq!(run.end, Err(StepReject::PastFirstMaximum));
        let s = run.last;
        assert!(within(
            s.fuel_temperature_rise_k,
            (1.0 - WATER_PROMPT_HEATING_FRACTION) * s.energy_mws / FUEL_HEAT_CAPACITY_MWS_PER_K,
            1.0e-12
        ));
        assert!(within(
            s.water_temperature_rise_k,
            WATER_PROMPT_HEATING_FRACTION * s.energy_mws / BAND_WATER_HEAT_CAPACITY_MWS_PER_K[0],
            1.0e-12
        ));
        assert_eq!(s.sweep_integral_k2, 0.0);
        // with flow the water is cooler than its share would make it, and more flow cools it more
        let slow = run_to_first_maximum(500.0, load(1.05, 4.8), ceiling()).last;
        let fast = run_to_first_maximum(500.0, load(1.05, 24.0), ceiling()).last;
        let share = |s: &ReactorState| {
            WATER_PROMPT_HEATING_FRACTION * s.energy_mws / BAND_WATER_HEAT_CAPACITY_MWS_PER_K[2]
        };
        assert!(slow.water_temperature_rise_k < share(&slow));
        assert!(fast.water_temperature_rise_k < slow.water_temperature_rise_k);
        assert!(slow.sweep_integral_k2 > 0.0 && fast.sweep_integral_k2 > 0.0);
        // and the burst that keeps less water feedback peaks higher
        assert!(fast.peak_power_mw > slow.peak_power_mw);
        println!(
            "500 degF 1.05 dollar: 4.8 ft/s peak {} MW water rise {} K; 24 ft/s peak {} MW water rise {} K",
            slow.peak_power_mw, slow.water_temperature_rise_k, fast.peak_power_mw, fast.water_temperature_rise_k
        );
    }

    #[test]
    fn a_step_after_the_first_maximum_is_refused_and_the_peak_is_the_highest_landed_sample() {
        let (reactor, _) = start(70.0);
        let run = run_to_first_maximum(70.0, load(1.2, 0.0), ceiling());
        assert_eq!(run.end, Err(StepReject::PastFirstMaximum));
        assert!(run.previous.power_rate_mw_per_s > 0.0);
        assert!(!(run.last.power_rate_mw_per_s > 0.0));
        assert!(run.last.past_first_maximum && !run.previous.past_first_maximum);
        let highest = if run.previous.power_mw > run.last.power_mw { run.previous } else { run.last };
        assert_eq!(run.last.peak_power_mw.to_bits(), highest.power_mw.to_bits());
        assert_eq!(run.last.peak_time_s.to_bits(), highest.t_s.to_bits());
        assert_eq!(run.last.peak_energy_mws.to_bits(), highest.energy_mws.to_bits());
        let mut state = run.last;
        let mut frame = marked_frame();
        let before = frame.to_bytes();
        assert_eq!(
            step(&reactor, &mut state, &mut frame, load(1.2, 0.0), 1.0e-6),
            Err(StepReject::PastFirstMaximum)
        );
        assert_eq!(state, run.last);
        assert_eq!(frame.to_bytes(), before);
    }

    #[test]
    fn at_the_floor_the_first_step_lands_and_is_not_taken_for_a_maximum() {
        let (reactor, mut state) = start(70.0);
        let mut frame = marked_frame();
        for _ in 0..50 {
            step(&reactor, &mut state, &mut frame, load(BAND_REACTIVITY_MIN_DOLLAR[0], 0.0), FLOOR_DT_S).unwrap();
            assert!(!state.past_first_maximum && state.rod_moving);
        }
        assert!(state.t_s > 0.0);
        assert_eq!(reactor.floor_dt_s(), Some(FLOOR_DT_S));
        assert!(within(reactor.gate_dt_s().unwrap(), 0.2 * 0.00215, 1.0e-15));
    }

    #[test]
    fn the_flags_are_only_flags_and_the_proof_is_not_the_cards() {
        let (reactor, mut state) = start(70.0);
        let mut frame = marked_frame();
        let mut seen_prompt = false;
        loop {
            match step(&reactor, &mut state, &mut frame, load(1.2, 0.0), ceiling()) {
                Ok(()) => {
                    assert_eq!(frame.proof, [0xA5; 16]);
                    assert_eq!(frame.flags & !(FLAG_PROMPT_SUPERCRITICAL | FLAG_ROD_MOVING), 0);
                    assert_eq!(
                        frame.flags & FLAG_PROMPT_SUPERCRITICAL != 0,
                        state.reactivity_net_dollar > 1.0
                    );
                    seen_prompt |= state.prompt_supercritical;
                    assert_eq!(frame.t.to_bits(), state.t_s.to_bits());
                    assert_eq!(frame.pos[0].to_bits(), (state.power_mw as f32).to_bits());
                    assert_eq!(frame.vel[0].to_bits(), (state.fuel_temperature_rise_k as f32).to_bits());
                    assert_eq!(frame.residual.to_bits(), (state.residual_conservation_error as f32).to_bits());
                }
                Err(reject) => {
                    assert_eq!(reject, StepReject::PastFirstMaximum);
                    break;
                }
            }
        }
        assert!(seen_prompt);
        fn is_copy<T: Copy>() {}
        is_copy::<ReactorSpert3Fuel>();
        is_copy::<ReactorState>();
        is_copy::<ReactorLoad>();
    }

    /// The loads the ledger is measured on: the ends of each band's span and its flows.
    const SPAN: [(f64, f64, f64); 10] = [
        (74.0, BAND_REACTIVITY_MIN_DOLLAR[0], 0.0),
        (67.0, 1.0, 0.0),
        (78.0, BAND_REACTIVITY_MAX_DOLLAR[0], 0.0),
        (259.0, BAND_REACTIVITY_MIN_DOLLAR[1], 22.0),
        (259.0, 1.04, 2.4),
        (251.0, BAND_REACTIVITY_MAX_DOLLAR[1], 14.0),
        (501.0, BAND_REACTIVITY_MIN_DOLLAR[2], 14.0),
        (501.0, 0.84, 4.8),
        (500.0, 1.1, 24.0),
        (500.0, BAND_REACTIVITY_MAX_DOLLAR[2], 14.0),
    ];

    #[test]
    fn the_ledger_residual_stays_under_the_limit_over_full_runs_at_the_ceiling() {
        let mut largest = 0.0;
        for (fahrenheit, rho, flow) in SPAN {
            let run = run_to_first_maximum(fahrenheit, load(rho, flow), ceiling());
            assert_eq!(run.end, Err(StepReject::PastFirstMaximum), "{fahrenheit} {rho} {flow}");
            println!(
                "{fahrenheit} degF {rho} dollar {flow} ft/s: {} steps, peak {} MW at {} s, energy {} MW s, fuel rise {} K, water rise {} K, largest residual {:e}",
                run.steps, run.last.peak_power_mw, run.last.peak_time_s, run.last.peak_energy_mws,
                run.last.fuel_temperature_rise_k, run.last.water_temperature_rise_k, run.largest_residual
            );
            if run.largest_residual > largest {
                largest = run.largest_residual;
            }
            // inside the compared spans: under the old margins of 1000 MW, 30 MW s and 20 s
            assert!(run.last.peak_power_mw < 1000.0 && run.last.energy_mws < 30.0);
            assert!(run.last.t_s < 20.0);
        }
        println!("largest residual over the span at the ceiling: {largest:e}");
        assert!(largest < RESIDUAL_CONSERVATION_LIMIT / 10.0);
    }

    #[test]
    fn explicit_euler_at_the_ceiling_is_refused_by_the_ledger() {
        for (fahrenheit, rho, flow) in SPAN {
            let run = run_with(euler, fahrenheit, load(rho, flow), ceiling());
            assert!(
                run.end == Err(StepReject::SpontaneousGeneration)
                    || run.end == Err(StepReject::FirstLawBreach),
                "{fahrenheit} {rho} {flow}: {:?} after {} steps",
                run.end,
                run.steps
            );
            // named before the burst: the power is still under a thousandth of the card's limit
            assert!(run.last.power_mw < 1.0, "{}", run.last.power_mw);
            assert!(!run.last.past_first_maximum);
        }
    }

    #[test]
    fn wrong_stage_weights_a_forgotten_feedback_and_a_falsified_ledger_are_refused() {
        for (fahrenheit, rho, flow) in SPAN {
            let run = run_with(wrong_weights, fahrenheit, load(rho, flow), ceiling());
            assert!(
                run.end == Err(StepReject::SpontaneousGeneration)
                    || run.end == Err(StepReject::FirstLawBreach),
                "{fahrenheit} {rho} {flow}: {:?}",
                run.end
            );
            assert!(!run.last.past_first_maximum);
        }
        // the water feedback forgotten in the stepping: named where the water feedback matters
        let run = run_with(no_water_feedback, 500.0, load(1.1, 4.8), ceiling());
        assert_eq!(run.end, Err(StepReject::SpontaneousGeneration));
        assert!(!run.last.past_first_maximum);
        // a ledger falsified in either direction is named in that direction
        let (reactor, _) = start(70.0);
        let mut state = start(70.0).1;
        let mut frame = marked_frame();
        for _ in 0..400 {
            step(&reactor, &mut state, &mut frame, load(1.1, 0.0), ceiling()).unwrap();
        }
        // The forger writes the derived fields again with the card's own arithmetic, so that
        // the state agrees with itself and only the ledger can tell.
        let t = terms(&reactor, 0, state.inlet_temperature_k, load(1.1, 0.0));
        let consistent = |forged: &mut ReactorState| {
            let d = derive(&reactor, &t, forged);
            forged.power_rate_mw_per_s = d.power_rate_mw_per_s;
            forged.reactivity_net_dollar = d.net_dollar;
            forged.population_defect_mw = d.population_defect_mw;
            forged.residual_conservation_error = d.residual;
        };
        let scale = state.population_anchor_mw * BAND_GENERATION_TIME_REDUCED_S[0];
        for (bias, expected) in [
            (-10.0 * RESIDUAL_CONSERVATION_LIMIT, StepReject::SpontaneousGeneration),
            (10.0 * RESIDUAL_CONSERVATION_LIMIT, StepReject::FirstLawBreach),
        ] {
            let mut forged = state;
            let population: f64 = forged.power_mw + forged.precursor_mw.iter().sum::<f64>();
            forged.insertion_integral_dollar_mws += bias * scale.max(population * BAND_GENERATION_TIME_REDUCED_S[0]);
            // left as it is, the defect it carries is no longer the card's: named as such
            let mut f = marked_frame();
            let mut careless = forged;
            assert_eq!(
                step(&reactor, &mut careless, &mut f, load(1.1, 0.0), ceiling()),
                Err(StepReject::InconsistentState)
            );
            consistent(&mut forged);
            let before = forged;
            let bytes = f.to_bytes();
            assert_eq!(step(&reactor, &mut forged, &mut f, load(1.1, 0.0), ceiling()), Err(expected));
            assert_eq!(forged, before);
            assert_eq!(f.to_bytes(), bytes);
        }
        // a precursor sum raised by hand is created population
        let mut forged = state;
        forged.precursor_mw[0] *= 1.0 + 1.0e-5;
        assert_eq!(
            step(&reactor, &mut forged, &mut frame, load(1.1, 0.0), ceiling()),
            Err(StepReject::InconsistentState)
        );
        consistent(&mut forged);
        assert_eq!(
            step(&reactor, &mut forged, &mut frame, load(1.1, 0.0), ceiling()),
            Err(StepReject::SpontaneousGeneration)
        );
    }

    #[test]
    fn a_dropped_doppler_feedback_runs_away_and_is_named() {
        fn no_doppler(
            reactor: &ReactorSpert3Fuel,
            terms: &Terms,
            t_s: f64,
            y: &[f64; 10],
            dt_s: f64,
        ) -> [f64; 10] {
            let mut altered = *terms;
            altered.doppler = 0.0;
            rk4(reactor, &altered, t_s, y, dt_s)
        }
        let run = run_with(no_doppler, 70.0, load(1.2, 0.0), ceiling());
        assert!(
            run.end == Err(StepReject::SpontaneousGeneration)
                || run.end == Err(StepReject::DtAboveStateCeiling),
            "{:?}",
            run.end
        );
        assert!(!run.last.past_first_maximum);
    }

    #[test]
    fn the_peak_at_the_ceiling_is_the_peak_at_a_sixteenth_of_it_to_eight_figures() {
        let l = load(1.1, 14.0);
        let peak = |dt: f64| {
            let run = run_to_first_maximum(500.0, l, dt);
            hermite_peak(&run.previous, &run.last)
        };
        let fine = peak(ceiling() / 16.0);
        let e1 = (peak(ceiling()).0 - fine.0).abs();
        let e2 = (peak(ceiling() / 2.0).0 - fine.0).abs();
        println!("peak power error at the ceiling {e1:e}, at half {e2:e}, ratio {}", e1 / e2);
        assert!(e1 / fine.0 < 1.0e-8);
        assert!(e2 < e1);
    }

    /// The forecast of `analysis/v2/fm.py` (plain Python, numpy, fixed-step RK4 at this card's
    /// ceiling, peak by cubic Hermite) at the card's four fitted constants, for six of the 28
    /// fit tests: test, inlet degF, flow ft/s, inhour reactivity in dollars, peak power MW,
    /// energy to peak MW s, time of peak s. `analysis/v2/out/f04_card_constants.json`,
    /// 2026-10-09. No held-back test and no measured outcome is in this table.
    const FORECAST: [(u32, f64, f64, f64, f64, f64, f64); 6] = [
        (22, 74.0, 0.0, 0.7686211839115998, 2.3330283812236905, 9.858859577867385, 14.857220067309017),
        (16, 67.0, 0.0, 1.0099978923705826, 12.669770441866513, 1.9331736996471884, 0.8302834443603127),
        (43, 78.0, 0.0, 1.2102315121058187, 304.06929624720766, 6.433311221079879, 0.22327607899943147),
        (63, 501.0, 14.0, 0.8366515135277093, 4.248613716094873, 10.9006309773648, 9.022291311256755),
        (67, 501.0, 4.8, 1.1378357231010046, 164.4303235423768, 5.299032632319804, 0.3157813942963584),
        (60, 500.0, 14.0, 1.2273310554599017, 422.4847922250984, 8.56513170761642, 0.23859569015798254),
    ];

    /// Each band's reactivity span is the inhour value of its longest scored
    /// and shortest tested period with the band's `l*`, not rounded: the
    /// value the compared request asked, within one unit in the last place
    /// of this crate's own inhour sum and outward of it, so every compared
    /// request is inside and nothing beyond it is.
    #[test]
    fn the_reactivity_spans_are_the_inhour_values_of_the_tested_periods() {
        let inhour = |period_s: f64, l_star: f64| {
            let omega = 1.0 / period_s;
            let mut total = omega * l_star;
            let mut index = 0;
            while index < 6 {
                total += GROUP_FRACTION[index] * omega / (omega + GROUP_DECAY_PER_S[index]);
                index += 1;
            }
            total
        };
        let longest = [1.010, 0.249, 0.592];
        let shortest = [0.010, 0.0103, 0.0097];
        for band in 0..3 {
            let l_star = BAND_GENERATION_TIME_REDUCED_S[band];
            let low = inhour(longest[band], l_star);
            let high = inhour(shortest[band], l_star);
            assert!(BAND_REACTIVITY_MIN_DOLLAR[band] <= low && low - BAND_REACTIVITY_MIN_DOLLAR[band] <= 2.0e-16, "{band}: {low}");
            assert!(BAND_REACTIVITY_MAX_DOLLAR[band] >= high && BAND_REACTIVITY_MAX_DOLLAR[band] - high <= 4.0e-16, "{band}: {high}");
        }
        // The forecast's edge tests are inside their bands, at the ends.
        assert_eq!(FORECAST[0].3.to_bits(), BAND_REACTIVITY_MIN_DOLLAR[0].to_bits());
        assert_eq!(FORECAST[2].3.to_bits(), BAND_REACTIVITY_MAX_DOLLAR[0].to_bits());
        assert_eq!(FORECAST[3].3.to_bits(), BAND_REACTIVITY_MIN_DOLLAR[2].to_bits());
        assert_eq!(FORECAST[5].3.to_bits(), BAND_REACTIVITY_MAX_DOLLAR[2].to_bits());
        // The old outward-rounded ends are refused.
        for (band, rho) in [(0, 0.768), (0, 1.211), (1, 0.919), (1, 1.208), (2, 0.836), (2, 1.229)] {
            assert!(!(BAND_REACTIVITY_MIN_DOLLAR[band]..=BAND_REACTIVITY_MAX_DOLLAR[band]).contains(&rho), "{band} {rho}");
        }
    }

    #[test]
    fn the_card_at_its_ceiling_agrees_with_the_forecast() {
        for (test, fahrenheit, flow, rho, power, energy, time) in FORECAST {
            let run = run_to_first_maximum(fahrenheit, load(rho, flow), ceiling());
            assert_eq!(run.end, Err(StepReject::PastFirstMaximum));
            let (p, e, t) = hermite_peak(&run.previous, &run.last);
            println!(
                "test {test}: peak power {:e}, energy {:e}, time {:e} relative to the forecast",
                p / power - 1.0, e / energy - 1.0, t / time - 1.0
            );
            // the forecast takes the step in which the rod stops as one Runge-Kutta step; the card
            // takes it as two that meet at the stop. The largest difference found is 8.0e-6.
            assert!(within(p, power, 2.0e-5), "test {test}");
            assert!(within(e, energy, 2.0e-5), "test {test}");
            assert!(within(t, time, 2.0e-5), "test {test}");
        }
    }

    #[test]
    fn without_feedback_the_power_rises_at_the_inhour_period() {
        // the inhour equation for the cold band: rho = w l* + sum b_i w / (w + lambda_i)
        let w = 1.0 / 0.0593;
        let l_star = BAND_GENERATION_TIME_REDUCED_S[0];
        let mut rho = w * l_star;
        for i in 0..6 {
            rho += GROUP_FRACTION[i] * w / (w + GROUP_DECAY_PER_S[i]);
        }
        let (reactor, state) = start(67.0);
        let mut t = terms(&reactor, 0, state.inlet_temperature_k, load(rho, 0.0));
        t.doppler = 0.0;
        t.water_feedback = 0.0;
        // the card's own rates and Runge–Kutta step with both feedbacks removed, to 1000 MW
        let mut y = state_vector(&state);
        let mut time = 0.0;
        let (mut y_last, mut time_last) = (y, time);
        while y[0] < 1000.0 {
            y_last = y;
            time_last = time;
            y = rk4_through_the_stop(&reactor, &t, time, &y, ceiling());
            time += ceiling();
        }
        let measured = (time - time_last) / (y[0] / y_last[0]).ln();
        println!("period measured on the card {measured} s, inhour 0.0593 s");
        assert!(within(measured, 0.0593, 1.0e-6));
        let _ = (DOPPLER_STRENGTH_DOLLAR_PER_SQRT_K, WATER_FEEDBACK_WEIGHT);
    }

    /// The step for a run past the compared spans: the static ceiling, held under the state
    /// ceiling of the growing prompt pole at the full reactivity, and of the sweep of the water,
    /// a thousandth under.
    fn under_the_state_ceiling(fahrenheit: f64, rho: f64, flow_fps: f64) -> f64 {
        let reactor = ReactorSpert3Fuel::e_core_low_initial_power();
        let band = reactor.band_of(kelvin(fahrenheit)).unwrap();
        let l_star = BAND_GENERATION_TIME_REDUCED_S[band];
        let mut dt = ceiling();
        if rho > 1.0 {
            dt = dt.min(GROWTH_PHASE_STEP * l_star / (rho - 1.0));
        }
        let sweep = 2.0 * feet(flow_fps) / ACTIVE_LENGTH_M;
        if sweep > 0.0 {
            dt = dt.min(GATE_PHASE_STEP / sweep);
        }
        0.999 * dt
    }

    /// The static ceiling covers the compared spans: in each band, across 21 reactivities of its
    /// span at its smallest and largest tested flow, run at the ceiling, no step is refused by the
    /// state ceiling.
    #[test]
    fn inside_the_compared_spans_the_state_ceiling_never_binds_at_the_static_ceiling() {
        let flows = [(0.0, 0.0), (2.4, 22.0), (4.8, 24.0)];
        let temperatures = [(56.0, 92.0), (251.0, 259.0), (498.0, 501.0)];
        for band in 0..3 {
            for index in 0..=20 {
                let low = BAND_REACTIVITY_MIN_DOLLAR[band];
                let high = BAND_REACTIVITY_MAX_DOLLAR[band];
                let rho = if index == 20 { high } else { low + (high - low) * (index as f64) / 20.0 };
                for flow in [flows[band].0, flows[band].1] {
                    for fahrenheit in [temperatures[band].0, temperatures[band].1] {
                        let run = run_to_first_maximum(fahrenheit, load(rho, flow), ceiling());
                        assert_eq!(run.end, Err(StepReject::PastFirstMaximum), "band {band} rho {rho} flow {flow}");
                    }
                }
            }
        }
    }

    /// Past the compared spans the state ceiling binds: above a net reactivity of about 1.22
    /// dollars the growing prompt pole advances more than 0.045 of its time constant in a step at
    /// the static ceiling, and a flow above about 226 m/s makes the sweep of the water faster. A run at the static ceiling is refused there by name, the state and the frame
    /// kept; a run under the state ceiling lands to its first maximum with the ledger closed.
    #[test]
    fn past_the_compared_spans_the_state_ceiling_binds_and_a_run_under_it_lands() {
        let reactor = ReactorSpert3Fuel::e_core_low_initial_power();
        for (fahrenheit, rho, flow) in [(70.0, 2.5, 0.0), (70.0, 5.0, 0.0), (500.0, 3.0, 14.0), (255.0, 2.5, 14.0), (500.0, 1.1, 1000.0)] {
            let (_, mut state) = start(fahrenheit);
            let mut frame = marked_frame();
            let end = loop {
                let before = state;
                let before_frame = frame;
                match step(&reactor, &mut state, &mut frame, load(rho, flow), ceiling()) {
                    Ok(()) => {}
                    Err(end) => {
                        assert_eq!(state, before);
                        assert_eq!(frame.to_bytes(), before_frame.to_bytes());
                        break end;
                    }
                }
            };
            assert_eq!(end, StepReject::DtAboveStateCeiling, "{fahrenheit} degF {rho} dollars {flow} ft/s");
            println!(
                "{fahrenheit} degF {rho} dollars {flow} ft/s at the static ceiling: refused at {} s, net reactivity {} dollars",
                state.t_s, state.reactivity_net_dollar
            );
            let dt = under_the_state_ceiling(fahrenheit, rho, flow);
            let run = run_to_first_maximum(fahrenheit, load(rho, flow), dt);
            println!(
                "  at {dt:e} s: {} steps, peak {} MW at {} s, energy {} MW s, fuel rise {} K, worst residual {:e}",
                run.steps, run.last.peak_power_mw, run.last.peak_time_s, run.last.peak_energy_mws,
                run.last.fuel_temperature_rise_k, run.largest_residual
            );
            assert_eq!(run.end, Err(StepReject::PastFirstMaximum), "{fahrenheit} degF {rho} dollars {flow} ft/s");
            assert!(run.largest_residual < RESIDUAL_CONSERVATION_LIMIT);
        }
    }

    /// The old walls, each band's reactivity span, its tested flows, 20 s, 30 MW s and 1000 MW,
    /// run past to several times beyond: every burst reaches its first maximum, every step with
    /// the ledger closed.
    #[test]
    fn past_every_old_wall_the_burst_reaches_its_first_maximum_with_the_ledger_closed() {
        let mut longest = 0.0_f64;
        let mut most_energy = 0.0_f64;
        let mut most_power = 0.0_f64;
        for (fahrenheit, rho, flow) in [
            (70.0, 0.3, 0.0),
            (70.0, 0.5, 0.0),
            (70.0, 1.0, 20.0),
            (70.0, 1.5, 0.0),
            (70.0, 2.0, 0.0),
            (70.0, 4.0, 0.0),
            (255.0, 0.5, 0.0),
            (255.0, 1.0, 50.0),
            (255.0, 3.0, 14.0),
            (500.0, 0.4, 14.0),
            (500.0, 1.0, 0.0),
            (500.0, 1.0, 100.0),
            (500.0, 2.0, 24.0),
            (500.0, 6.0, 24.0),
        ] {
            let dt = under_the_state_ceiling(fahrenheit, rho, flow);
            let run = run_to_first_maximum(fahrenheit, load(rho, flow), dt);
            println!(
                "{fahrenheit} degF {rho} dollars {flow} ft/s at {dt:e} s: {} steps, peak {} MW at {} s, energy to peak {} MW s, fuel rise {} K, worst residual {:e}",
                run.steps, run.last.peak_power_mw, run.last.peak_time_s, run.last.peak_energy_mws,
                run.last.fuel_temperature_rise_k, run.largest_residual
            );
            assert_eq!(run.end, Err(StepReject::PastFirstMaximum), "{fahrenheit} degF {rho} dollars {flow} ft/s");
            assert!(run.largest_residual < RESIDUAL_CONSERVATION_LIMIT);
            longest = longest.max(run.last.peak_time_s);
            most_energy = most_energy.max(run.last.peak_energy_mws);
            most_power = most_power.max(run.last.peak_power_mw);
        }
        assert!(longest > 3.0 * 20.0, "{longest}");
        assert!(most_energy > 2.0 * 30.0, "{most_energy}");
        assert!(most_power > 5.0 * 1000.0, "{most_power}");
    }
}
