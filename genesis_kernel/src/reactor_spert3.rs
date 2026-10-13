#![no_implicit_prelude]

//! The SPERT III E-core, cold startup, from a step in reactivity to the first
//! maximum of power.
//!
//! Measured by the Phillips Petroleum Company for the Atomic Energy
//! Commission: IDO-17281 (March 1969), OSTI 4792676, Table III, 30 tests,
//! each value with a standard deviation. The report's cover says
//! "Distribution of this document is unlimited." The OSTI record page states
//! no licence and no copyright notice. The card is distributed without
//! restriction by the Department of Energy and is not a confirmed
//! public-domain finding. No measured value of the report is stored in the
//! card.
//!
//! The core is 1271.5 kg of UO2 at 4.8 percent enrichment in light water,
//! cold (primary coolant 56 to 92 degF), no flow, power 5e-5 MW at the start.
//! The card is point kinetics with six delayed-neutron groups and a
//! reactivity feedback linear in the energy released. One constant is fitted:
//! [`FEEDBACK_STRENGTH_DOLLAR_PER_MWS`], the least-squares fit of
//! `z_P² + z_E²` to the 20 opened tests of the protocol (peak power and
//! energy released to peak power, each in units of the stated standard
//! deviation). The value is the one in
//! `analysis/out/03_fit_and_score_B.json`, structure B, key `constants`,
//! rounded by that script to six significant figures. The ten sealed tests
//! were not used in the fit. Everything else is locked from the
//! report, with its table and page beside each constant. The reactivity a
//! caller supplies is, for a measured test, the inhour-equation value of the
//! test's reactor period for this card's own six-group set. That conversion
//! belongs to the caller. The card takes dollars.
//!
//! The declared range: reactivity inserted 0.7686211839115998 to
//! 1.2102315121058187 dollars held, primary coolant 56 to 92 degF, time to
//! 20 s, energy released to 10 MW s, power to 1000 MW. The reactivity span is
//! the inhour value of the longest and the shortest tested reactor period
//! (1010 ms, test 22, and 10 ms, test 43) for this card's set, with no
//! rounding: a reactivity outside it was compared with no measurement and is
//! refused as [`StepReject::ReactivityOutsideCard`]. Tests 22, 42 and 43 are
//! opened tests the constant was fitted on; the ten held-back tests span 0.890
//! to 1.128 dollars. The card runs from the step in reactivity to the first maximum
//! of power and refuses every step after it ([`StepReject::PastFirstMaximum`]).
//! Boiling begins one to two seconds after the peak in the report, heat flow
//! from fuel to water is not in the card, and nothing after the peak is
//! tabulated in Table III.
//!
//! The card leaves out the fuel temperature, the coolant and its flow,
//! pressure, void, hot startup, hot standby, operating power, a rod that
//! moves during the insertion, any other core and any other loading of this
//! core. The coolant temperature is declared range only: the feedback of this
//! card does not depend on it, and the state records it so that a step can
//! refuse a value outside the span.
//!
//! The pass line and where the result is. The protocol fixed before any model
//! ran: peak power and energy to peak, each with a root-mean-square z of at
//! most 1.5, a worst z of at most 3 and a mean of magnitude at most 0.5, and
//! time of peak with a root-mean-square relative error of at most 0.05 and a
//! worst of 0.10. Structure B met the first two and missed the third in
//! leave-one-out prediction on the opened tests: peak power rms 0.44, energy
//! to peak rms 0.86, time of peak rms 0.216 (peaks about 21 percent early).
//! The result is in `analysis/out/03_fit_and_score_B.log` and
//! `analysis/out/07_summary.log` of the research record at
//! `evidence/research/reactor_spert3/`. Compared with the ten held-back tests
//! on 2026-10-08 (`analysis/out/09_sealed/RESULTS_sealed.json`, engine 0.2.1),
//! the card met five of the eight lines fixed beforehand and missed three:
//! the mean of the energy to the peak and both time-of-peak lines. The states
//! the card seals before the peak were compared only with digitised plots of
//! 19 opened tests; on 18 of them the card reaches a tenth of its peak about
//! 26 percent early (`analysis/10_peak_timing_cause.md`).
//!
//! How the peak is read from a sealed run. A step is not sealed when it is
//! refused. A run that ends in [`StepReject::PastFirstMaximum`] therefore
//! ends on the last landed state, the first sample at which the power is no
//! longer rising. The state carries the highest landed sample as
//! `peak_power_mw`, with the energy released and the time at that same
//! sample (`peak_energy_mws`, `peak_time_s`). The true maximum lies within
//! one step of `peak_time_s`. The card does not interpolate. At the sample
//! the power is below the true maximum by a second-order amount in the step.
//! The energy at the sample differs from the energy at the true peak by about
//! the power times the time between them, at most half a step. A comparison
//! that needs the maximum to better than that runs the same card at a smaller
//! step or interpolates through the last three landed samples: that belongs
//! to the caller.
//!
//! `core` only: no heap and no standard prelude. The state is `Copy`. An
//! accepted step writes time, power, energy released, net reactivity, the
//! power rate, the three peak values and the conservation residual into
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched. The six
//! precursor sums travel in the state, not in the frame.
//!
//! ```text
//! net reactivity       ρ = ρ_ins − a E                          dollars
//! power                dP/dt = ((ρ − 1) / l*) P + Σ λ_i C_i     MW per s
//! precursor sums       dC_i/dt = (b_i / l*) P − λ_i C_i         i = 1 to 6
//! energy released     dE/dt = P                                MW
//! ```
//!
//! `l*` is [`GENERATION_TIME_REDUCED_S`], `a` is
//! [`FEEDBACK_STRENGTH_DOLLAR_PER_MWS`], `ρ_ins` is the load held through the
//! step. The start is the equilibrium at [`INITIAL_POWER_MW`] with zero net
//! reactivity, `C_i = b_i P / (l* λ_i)`, and the reactivity is then stepped to
//! `ρ_ins` at time zero. The map is fixed-step fourth-order Runge–Kutta on the
//! eight variables (P, six C_i, E), with `a E` taken from the energy of each
//! stage.
//!
//! The ledger is the neutron population `N = P + Σ C_i` (MW). Summing the
//! equations above, the precursor terms cancel and `dN/dt = ρ P / l*`.
//! Because `P dt = dE`, the net production is the integral of the net
//! reactivity over the energy released:
//!
//! ```text
//! N − N_anchor = (1 / l*) ∫ ρ dE = (1 / l*) [ ρ_ins E − a E² / 2 ]
//! population_defect_mw        = N − N_anchor − W / l*
//! W                           = Σ over steps of  ρ_ins ΔE − (a / 2) ΔE (E_old + E_new)
//! residual_conservation_error = |population_defect_mw| / max(N, N_anchor)
//! ```
//!
//! `−a E² / 2` is the closed-form antiderivative of the feedback and is a
//! polynomial, so the card calls no function of [`crate::math`] and no
//! function of `std`: its arithmetic is `+ − × ÷` and nothing else. The
//! Runge–Kutta step advances P and C_i by the differential equations and
//! never evaluates `W`. A wrong coefficient, a dropped group, a wrong stage
//! weight or an Euler step in its place shows in the defect. `W` is
//! accumulated step by step, each step booked at the reactivity held through
//! it. The ledger shows the arithmetic and the integrator are right. It
//! cannot show the physics matches the reactor, and it books whatever
//! reactivity it is given: it does not name a reactivity changed between
//! steps. The step does.
//!
//! One burst, one reactivity. The card is one step in reactivity at time
//! zero, held to the first maximum of power, which is what each of the 30
//! tests of Table III is: no tested burst changed its reactivity. The first
//! landed step records its load in the state
//! ([`ReactorState::burst_reactivity_dollar`]), and every later step whose
//! load is another number is refused as
//! [`StepReject::ReactivityChangedDuringBurst`], the state and the frame kept:
//! a second insertion, a withdrawal or a jump in the inserted reactivity is
//! not on the card. A state that is not the card's start, bit for bit, is
//! held to the reactivity it records, so a state written by hand with no
//! burst recorded takes no load at all. The recorded reactivity is state and
//! is not published: it is not in the frame, and it is the load every sealed
//! request already carries.
//!
//! An incoming state is held to what the card itself writes. An energy or a
//! precursor sum below zero, or a power not above zero, is
//! [`StepReject::NegativeState`]; an energy, a power or a peak power past the
//! card's limits is named as that limit; a state at time zero that is not
//! the card's start, bit for bit, and a state whose net reactivity, power
//! rate, ledger defect, residual, anchor, flag or peak fields are not what
//! the card computes again from the rest of the state under the load, is
//! [`StepReject::InconsistentState`]. A caller of the public step who
//! rewrites the recorded reactivity together with the load is named by it,
//! because the net reactivity and the power rate of the state are those of
//! the first reactivity. What the state cannot show: a power, an energy and
//! the precursors changed together with every derived field recomputed is
//! the state of another burst and is taken.
//!
//! [`ReactorSpert3::gate_dt_s`] is the step ceiling. It is
//! `GATE_PHASE_STEP · l* / (1 − ρ_min + a E_max)`, 0.2 of the time constant of
//! the fastest pole the card can reach: the prompt pole `(1 − ρ) / l*` at the
//! smallest load, with the feedback of the largest energy the card allows
//! added to `1 − ρ`. [`ReactorSpert3::floor_dt_s`] is
//! [`FLOOR_DT_S`], 1e-9 s. At 20 s a step of that size still moves the clock
//! (spacing of 64-bit numbers there is 3.6e-15 s), at the start it raises the
//! power by `ρ dt / l*`, 4.7e-7 of itself at one dollar, which 64-bit numbers
//! resolve to nine figures, and it raises the energy released by `P dt`. A
//! precursor sum moves at second order in the step. At the start each sum is
//! at equilibrium, and sums 1, 2 and 3 do not change in a step of this size;
//! sums 4, 5 and 6 do, by about 1e-16 to 1e-15 of themselves (sum 4 only at a
//! reactivity of one dollar or more). The public step applies neither.
//! The gateway refuses a step outside them.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;

/// Reduced prompt-neutron generation time `l*`, the generation time divided by
/// the effective delayed fraction. IDO-17281, Table II (page 11), row of
/// coolant inlet 70 degF, 2.15 ms.
pub const GENERATION_TIME_REDUCED_S: f64 = 0.00215;
/// Delayed-neutron group fractions `b_i`, as fractions of the total delayed
/// fraction; sum 1. IDO-17281, Appendix B, section 3.3, table (page 109). The
/// report states the set for 500 degF; it prints none for 70 degF.
pub const GROUP_FRACTION: [f64; 6] = [0.0352, 0.2014, 0.1858, 0.4006, 0.1437, 0.0333];
/// Delayed-neutron decay constants `λ_i`, per second. IDO-17281, Appendix B,
/// section 3.3, table (page 109), same table as [`GROUP_FRACTION`].
pub const GROUP_DECAY_PER_S: [f64; 6] = [0.0127, 0.0317, 0.1167, 0.3142, 1.4007, 3.8803];
/// Reactor power at the start, MW. IDO-17281, summary table of initial
/// conditions, cold startup, 5 x 10^-5 MW (page iv in the scan, cited as page
/// v in the research record). Not certain to be the power at the clock's zero
/// of Table III.
pub const INITIAL_POWER_MW: f64 = 5.0e-5;
/// Feedback strength `a`: dollars of reactivity lost per MW s of energy
/// released. Fitted on the 20 opened tests, least squares on
/// `z_P² + z_E²`; `analysis/out/03_fit_and_score_B.json`, structure B, key
/// `constants`, rounded by the script to six significant figures.
pub const FEEDBACK_STRENGTH_DOLLAR_PER_MWS: f64 = 0.0369864;
/// Smallest reactivity the card accepts, dollars: the inhour value of the
/// longest tested reactor period (1010 ms, test 22; IDO-17281 Table III,
/// report page 16) for this card's set, not rounded. Test 22 is the smallest
/// tested reactivity; below it nothing was compared.
pub const REACTIVITY_MIN_DOLLAR: f64 = 0.7686211839115998;
/// Largest reactivity the card accepts, dollars: the inhour value of the
/// shortest tested reactor period (10 ms, test 43; IDO-17281 Table III) for
/// this card's set, not rounded. Test 43 is the largest tested reactivity;
/// above it nothing was compared.
pub const REACTIVITY_MAX_DOLLAR: f64 = 1.2102315121058187;
/// Lowest primary coolant temperature tested, 56 degF, in kelvin. IDO-17281,
/// Table III.
pub const COOLANT_TEMPERATURE_MIN_K: f64 = (56.0 - 32.0) / 1.8 + 273.15;
/// Highest primary coolant temperature tested, 92 degF, in kelvin. IDO-17281,
/// Table III.
pub const COOLANT_TEMPERATURE_MAX_K: f64 = (92.0 - 32.0) / 1.8 + 273.15;
/// Energy released beyond which the card refuses, MW s. The card's own peak
/// energy at the smallest load is 7.2 MW s; the largest in Table III is 6.9.
pub const ENERGY_MAX_MWS: f64 = 10.0;
/// Power beyond which the card refuses, MW. The card's own largest peak is
/// about 296 MW, at the top of the reactivity span (test 43, 1.2102315
/// dollars); the largest in Table III is 280.
pub const POWER_MAX_MW: f64 = 1000.0;
/// Time since the step in reactivity beyond which the card refuses, s. The
/// card's own latest peak is at 12.2 s; the longest in Table III is 13.7.
pub const TIME_MAX_S: f64 = 20.0;
/// `|N − N_anchor − W / l*| / max(N, N_anchor)` must stay at or under this.
/// Set by measurement over the 20 opened periods, each run at the ceiling to
/// its first maximum: the largest value found is 5.3e-9 (period 10 ms, 214
/// steps), and the limit is 19 times it. A step 2.6 times the ceiling on that
/// period is refused by this limit.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-7;
/// Fraction of the time constant of the fastest pole that one step at the
/// ceiling advances.
pub const GATE_PHASE_STEP: f64 = 0.2;
/// Smallest step the card stands behind, s.
pub const FLOOR_DT_S: f64 = 1.0e-9;

/// Net reactivity is above one dollar on this frame.
pub const FLAG_PROMPT_SUPERCRITICAL: u64 = 1 << 0;

/// Why a step did not advance the reactor or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// `dt` or the held reactivity was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or the trial step, was non-finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this reactor's.
    InvalidCard,
    /// The held reactivity is below 0.7686211839115998 dollar or above 1.2102315121058187 dollars, the inhour values of the longest and the shortest tested reactor period.
    ReactivityOutsideCard,
    /// The recorded primary coolant temperature is outside 56 to 92 degF.
    CoolantTemperatureOutsideCard,
    /// The state is past the first maximum of power; nothing after it is on the card.
    PastFirstMaximum,
    /// The step would end after 20 s.
    TimeBeyondCard,
    /// The trial step released more than 10 MW s.
    EnergyBeyondCard,
    /// The trial step reached more than 1000 MW.
    PowerBeyondCard,
    /// The reactivity of the load is not the one of the first step of this burst; a burst is one step in reactivity, held to the first maximum of power.
    ReactivityChangedDuringBurst,
    /// The incoming state holds an energy released or a precursor sum below zero, or a power that is not above zero.
    NegativeState,
    /// The incoming state is not one this card writes: at time zero it is not the card's start, or a field the card derives from the rest of the state does not hold the value the card derives.
    InconsistentState,
    /// The trial step created neutron population beyond what the reactivity supplies.
    SpontaneousGeneration,
    /// The trial step destroyed neutron population the reactivity supplies.
    FirstLawBreach,
}

/// The reactivity inserted by the control rod, held through the step. The first landed step
/// records it in the state, and every later step of that burst carries the same number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorLoad {
    /// Dollars of reactivity inserted, `ρ_ins`.
    pub reactivity_inserted_dollar: f64,
}

/// The locked reactor. Every field is a recorded copy and none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactorSpert3 {
    /// Recorded copy of [`GENERATION_TIME_REDUCED_S`].
    pub generation_time_reduced_s: f64,
    /// Recorded copy of [`GROUP_FRACTION`].
    pub group_fraction: [f64; 6],
    /// Recorded copy of [`GROUP_DECAY_PER_S`].
    pub group_decay_per_s: [f64; 6],
    /// Recorded copy of [`INITIAL_POWER_MW`].
    pub initial_power_mw: f64,
    /// Recorded copy of [`FEEDBACK_STRENGTH_DOLLAR_PER_MWS`].
    pub feedback_strength_dollar_per_mws: f64,
    /// Recorded copy of [`REACTIVITY_MIN_DOLLAR`].
    pub reactivity_min_dollar: f64,
    /// Recorded copy of [`REACTIVITY_MAX_DOLLAR`].
    pub reactivity_max_dollar: f64,
    /// Recorded copy of [`COOLANT_TEMPERATURE_MIN_K`].
    pub coolant_temperature_min_k: f64,
    /// Recorded copy of [`COOLANT_TEMPERATURE_MAX_K`].
    pub coolant_temperature_max_k: f64,
    /// Recorded copy of [`ENERGY_MAX_MWS`].
    pub energy_max_mws: f64,
    /// Recorded copy of [`POWER_MAX_MW`].
    pub power_max_mw: f64,
    /// Recorded copy of [`TIME_MAX_S`].
    pub time_max_s: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`GATE_PHASE_STEP`].
    pub gate_phase_step: f64,
    /// Recorded copy of [`FLOOR_DT_S`].
    pub floor_dt_s: f64,
}

impl ReactorSpert3 {
    /// Card: the SPERT III E-core, cold startup. The constructor takes no arguments.
    #[must_use]
    pub fn e_core_cold_startup() -> Self {
        let reactor = Self {
            generation_time_reduced_s: GENERATION_TIME_REDUCED_S,
            group_fraction: GROUP_FRACTION,
            group_decay_per_s: GROUP_DECAY_PER_S,
            initial_power_mw: INITIAL_POWER_MW,
            feedback_strength_dollar_per_mws: FEEDBACK_STRENGTH_DOLLAR_PER_MWS,
            reactivity_min_dollar: REACTIVITY_MIN_DOLLAR,
            reactivity_max_dollar: REACTIVITY_MAX_DOLLAR,
            coolant_temperature_min_k: COOLANT_TEMPERATURE_MIN_K,
            coolant_temperature_max_k: COOLANT_TEMPERATURE_MAX_K,
            energy_max_mws: ENERGY_MAX_MWS,
            power_max_mw: POWER_MAX_MW,
            time_max_s: TIME_MAX_S,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            gate_phase_step: GATE_PHASE_STEP,
            floor_dt_s: FLOOR_DT_S,
        };
        if reactor.is_valid() {
            reactor
        } else {
            ::core::panic!("E-core reactor failed is_valid");
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        let mut index = 0;
        let mut fraction_sum = 0.0;
        while index < 6 {
            if self.group_fraction[index].to_bits() != GROUP_FRACTION[index].to_bits()
                || self.group_decay_per_s[index].to_bits() != GROUP_DECAY_PER_S[index].to_bits()
                || !(self.group_decay_per_s[index] > 0.0)
                || !(self.group_fraction[index] > 0.0)
            {
                return false;
            }
            fraction_sum += self.group_fraction[index];
            index += 1;
        }
        self.generation_time_reduced_s.to_bits() == GENERATION_TIME_REDUCED_S.to_bits()
            && self.initial_power_mw.to_bits() == INITIAL_POWER_MW.to_bits()
            && self.feedback_strength_dollar_per_mws.to_bits()
                == FEEDBACK_STRENGTH_DOLLAR_PER_MWS.to_bits()
            && self.reactivity_min_dollar.to_bits() == REACTIVITY_MIN_DOLLAR.to_bits()
            && self.reactivity_max_dollar.to_bits() == REACTIVITY_MAX_DOLLAR.to_bits()
            && self.coolant_temperature_min_k.to_bits() == COOLANT_TEMPERATURE_MIN_K.to_bits()
            && self.coolant_temperature_max_k.to_bits() == COOLANT_TEMPERATURE_MAX_K.to_bits()
            && self.energy_max_mws.to_bits() == ENERGY_MAX_MWS.to_bits()
            && self.power_max_mw.to_bits() == POWER_MAX_MW.to_bits()
            && self.time_max_s.to_bits() == TIME_MAX_S.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.gate_phase_step.to_bits() == GATE_PHASE_STEP.to_bits()
            && self.floor_dt_s.to_bits() == FLOOR_DT_S.to_bits()
            && self.generation_time_reduced_s > 0.0
            && self.initial_power_mw > 0.0
            && self.feedback_strength_dollar_per_mws > 0.0
            && self.reactivity_min_dollar > 0.0
            && self.reactivity_max_dollar > self.reactivity_min_dollar
            && self.coolant_temperature_max_k > self.coolant_temperature_min_k
            && self.energy_max_mws > 0.0
            && self.power_max_mw > self.initial_power_mw
            && self.time_max_s > 0.0
            && self.residual_conservation_limit > 0.0
            && self.gate_phase_step > 0.0
            && self.floor_dt_s > 0.0
            && fraction_sum > 1.0 - 1.0e-12
            && fraction_sum < 1.0 + 1.0e-12
    }

    /// Largest step the card stands behind: `GATE_PHASE_STEP · l* / (1 − ρ_min + a E_max)`,
    /// 0.2 of the time constant of the fastest pole the card can reach, about 0.7 ms.
    /// Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let loss = 1.0 - self.reactivity_min_dollar
            + self.feedback_strength_dollar_per_mws * self.energy_max_mws;
        if loss.is_finite() && loss > 0.0 && self.generation_time_reduced_s > 0.0 {
            Some(self.gate_phase_step * self.generation_time_reduced_s / loss)
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
    /// Time since the step in reactivity.
    pub t_s: f64,
    /// Reactor power, `P`.
    pub power_mw: f64,
    /// Delayed-neutron precursor sums, `C_i`.
    pub precursor_mw: [f64; 6],
    /// Energy released since the step in reactivity, `E`.
    pub energy_mws: f64,
    /// `dP/dt` at this state under the load of the last step.
    pub power_rate_mw_per_s: f64,
    /// `ρ_ins − a E` at this state under the load of the last step. Zero at the start.
    pub reactivity_net_dollar: f64,
    /// Primary coolant temperature at the start. Declared range only; the card's feedback does not use it.
    pub coolant_temperature_k: f64,
    /// `N = P + Σ C_i` at the state the ledger started from.
    pub population_anchor_mw: f64,
    /// `W`, the integral of net reactivity over energy released since the anchor, in closed form.
    pub reactivity_energy_integral_dollar_mws: f64,
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
    /// Reactivity of the load of the first landed step, `ρ_ins` of this burst. Zero at the
    /// start, before any step has landed. Not written to the frame.
    pub burst_reactivity_dollar: f64,
    /// Set by the step that lands with the power no longer rising. The next step is refused.
    pub past_first_maximum: bool,
    /// Net reactivity above one dollar.
    pub prompt_supercritical: bool,
}

impl ReactorState {
    /// The reactor in equilibrium at [`INITIAL_POWER_MW`], zero net reactivity, time 0.
    /// `None` unless the temperature is finite and within 56 to 92 degF.
    #[must_use]
    pub fn try_cold_start(reactor: &ReactorSpert3, coolant_temperature_k: f64) -> Option<Self> {
        if !reactor.is_valid()
            || !coolant_temperature_k.is_finite()
            || coolant_temperature_k < reactor.coolant_temperature_min_k
            || coolant_temperature_k > reactor.coolant_temperature_max_k
        {
            return None;
        }
        let power_mw = reactor.initial_power_mw;
        let mut precursor_mw = [0.0; 6];
        let mut index = 0;
        while index < 6 {
            precursor_mw[index] = reactor.group_fraction[index] * power_mw
                / (reactor.generation_time_reduced_s * reactor.group_decay_per_s[index]);
            index += 1;
        }
        let mut state = Self {
            t_s: 0.0,
            power_mw,
            precursor_mw,
            energy_mws: 0.0,
            power_rate_mw_per_s: 0.0,
            reactivity_net_dollar: 0.0,
            coolant_temperature_k,
            population_anchor_mw: 0.0,
            reactivity_energy_integral_dollar_mws: 0.0,
            population_defect_mw: 0.0,
            residual_conservation_error: 0.0,
            peak_power_mw: power_mw,
            peak_energy_mws: 0.0,
            peak_time_s: 0.0,
            burst_reactivity_dollar: 0.0,
            past_first_maximum: false,
            prompt_supercritical: false,
        };
        let y = state_vector(&state);
        state.power_rate_mw_per_s = rates(reactor, 0.0, &y)[0];
        state.population_anchor_mw = population_mw(&state);
        if !state_is_finite(&state) {
            return None;
        }
        Some(state)
    }
}

/// Advance one Runge–Kutta step and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
#[must_use]
pub fn step(
    reactor: &ReactorSpert3,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(reactor, state, load, dt_s)?;
    let advanced = rk4(
        reactor,
        load.reactivity_inserted_dollar,
        &state_vector(state),
        dt_s,
    );
    commit(reactor, state, frame, load, dt_s, advanced)
}

fn validate(
    reactor: &ReactorSpert3,
    state: &ReactorState,
    load: ReactorLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    if !dt_s.is_finite() || !load.reactivity_inserted_dollar.is_finite() {
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
    if load.reactivity_inserted_dollar < reactor.reactivity_min_dollar
        || load.reactivity_inserted_dollar > reactor.reactivity_max_dollar
    {
        return Err(StepReject::ReactivityOutsideCard);
    }
    if state.coolant_temperature_k < reactor.coolant_temperature_min_k
        || state.coolant_temperature_k > reactor.coolant_temperature_max_k
    {
        return Err(StepReject::CoolantTemperatureOutsideCard);
    }
    if state.past_first_maximum {
        return Err(StepReject::PastFirstMaximum);
    }
    if !(state.t_s >= 0.0) || !(state.t_s + dt_s <= reactor.time_max_s) {
        return Err(StepReject::TimeBeyondCard);
    }
    incoming_state_is_the_cards(reactor, state, load)
}

/// The incoming state held to what this card itself writes.
///
/// Every landed step leaves the energy and the precursor sums at or above zero and the power
/// above it, the energy and the power inside the card's limits, and every derived field (the
/// net reactivity, the power rate, the ledger's defect and residual, the flags) equal, bit
/// for bit, to what the card computes again here from the rest of the state under the burst's
/// reactivity. A state at time zero is the card's start and nothing else. A state that fails
/// any of these was not written by this card. What it cannot show: a power, an energy and
/// precursors changed together with every derived field recomputed, which is another burst's
/// state and not an impossible one.
fn incoming_state_is_the_cards(
    reactor: &ReactorSpert3,
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
    if !(state.power_mw > 0.0) || state.energy_mws < 0.0 {
        return Err(StepReject::NegativeState);
    }
    if state.energy_mws > reactor.energy_max_mws {
        return Err(StepReject::EnergyBeyondCard);
    }
    if state.power_mw > reactor.power_max_mw || state.peak_power_mw > reactor.power_max_mw {
        return Err(StepReject::PowerBeyondCard);
    }
    if state.t_s == 0.0 {
        // No step has landed: the card's start takes any reactivity of the card.
        return if is_the_cold_start(reactor, state) {
            Ok(())
        } else {
            Err(StepReject::InconsistentState)
        };
    }
    // One burst, one reactivity. Every landed step records its load, so a load equal to the
    // recorded one continues the burst.
    if load.reactivity_inserted_dollar != state.burst_reactivity_dollar {
        return Err(StepReject::ReactivityChangedDuringBurst);
    }
    let start = match ReactorState::try_cold_start(reactor, state.coolant_temperature_k) {
        Some(start) => start,
        None => return Err(StepReject::InconsistentState),
    };
    let rho_inserted = load.reactivity_inserted_dollar;
    let net_dollar = rho_inserted - reactor.feedback_strength_dollar_per_mws * state.energy_mws;
    let power_rate_mw_per_s = rates(reactor, rho_inserted, &state_vector(state))[0];
    let population = population_mw(state);
    let defect_mw = population
        - state.population_anchor_mw
        - state.reactivity_energy_integral_dollar_mws / reactor.generation_time_reduced_s;
    let scale = if population > state.population_anchor_mw {
        population
    } else {
        state.population_anchor_mw
    };
    if state.population_anchor_mw.to_bits() != start.population_anchor_mw.to_bits()
        || state.reactivity_net_dollar.to_bits() != net_dollar.to_bits()
        || state.power_rate_mw_per_s.to_bits() != power_rate_mw_per_s.to_bits()
        || state.prompt_supercritical != (net_dollar > 1.0)
        || state.population_defect_mw.to_bits() != defect_mw.to_bits()
        || state.residual_conservation_error.to_bits() != (defect_mw.abs() / scale).to_bits()
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
    if !(power_rate_mw_per_s > 0.0) {
        return Err(StepReject::PastFirstMaximum);
    }
    Ok(())
}

/// Whether `state` is, bit for bit, the start this card makes for the coolant temperature the
/// state records: no step has landed and no burst has begun.
fn is_the_cold_start(reactor: &ReactorSpert3, state: &ReactorState) -> bool {
    let start = match ReactorState::try_cold_start(reactor, state.coolant_temperature_k) {
        Some(start) => start,
        None => return false,
    };
    let mut index = 0;
    while index < 6 {
        if state.precursor_mw[index].to_bits() != start.precursor_mw[index].to_bits() {
            return false;
        }
        index += 1;
    }
    state.t_s.to_bits() == start.t_s.to_bits()
        && state.power_mw.to_bits() == start.power_mw.to_bits()
        && state.energy_mws.to_bits() == start.energy_mws.to_bits()
        && state.power_rate_mw_per_s.to_bits() == start.power_rate_mw_per_s.to_bits()
        && state.reactivity_net_dollar.to_bits() == start.reactivity_net_dollar.to_bits()
        && state.coolant_temperature_k.to_bits() == start.coolant_temperature_k.to_bits()
        && state.population_anchor_mw.to_bits() == start.population_anchor_mw.to_bits()
        && state.reactivity_energy_integral_dollar_mws.to_bits()
            == start.reactivity_energy_integral_dollar_mws.to_bits()
        && state.population_defect_mw.to_bits() == start.population_defect_mw.to_bits()
        && state.residual_conservation_error.to_bits()
            == start.residual_conservation_error.to_bits()
        && state.peak_power_mw.to_bits() == start.peak_power_mw.to_bits()
        && state.peak_energy_mws.to_bits() == start.peak_energy_mws.to_bits()
        && state.peak_time_s.to_bits() == start.peak_time_s.to_bits()
        && state.burst_reactivity_dollar.to_bits() == start.burst_reactivity_dollar.to_bits()
        && state.past_first_maximum == start.past_first_maximum
        && state.prompt_supercritical == start.prompt_supercritical
}

fn commit(
    reactor: &ReactorSpert3,
    state: &mut ReactorState,
    frame: &mut LastStateFrame64,
    load: ReactorLoad,
    dt_s: f64,
    advanced: [f64; 8],
) -> Result<(), StepReject> {
    let saved = *state;
    let mut index = 0;
    while index < 8 {
        if !advanced[index].is_finite() {
            return Err(StepReject::NonFiniteState);
        }
        index += 1;
    }
    if advanced[7] > reactor.energy_max_mws {
        return Err(StepReject::EnergyBeyondCard);
    }
    if advanced[0] > reactor.power_max_mw {
        return Err(StepReject::PowerBeyondCard);
    }
    let rho_inserted = load.reactivity_inserted_dollar;
    let energy_old = state.energy_mws;
    state.t_s += dt_s;
    state.power_mw = advanced[0];
    index = 0;
    while index < 6 {
        state.precursor_mw[index] = advanced[1 + index];
        index += 1;
    }
    state.energy_mws = advanced[7];
    let energy_step = state.energy_mws - energy_old;
    state.reactivity_energy_integral_dollar_mws += rho_inserted * energy_step
        - 0.5
            * reactor.feedback_strength_dollar_per_mws
            * energy_step
            * (energy_old + state.energy_mws);
    state.reactivity_net_dollar =
        rho_inserted - reactor.feedback_strength_dollar_per_mws * state.energy_mws;
    state.power_rate_mw_per_s = rates(reactor, rho_inserted, &state_vector(state))[0];
    let population = population_mw(state);
    state.population_defect_mw = population
        - state.population_anchor_mw
        - state.reactivity_energy_integral_dollar_mws / reactor.generation_time_reduced_s;
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
    state.past_first_maximum = !(state.power_rate_mw_per_s > 0.0);
    state.prompt_supercritical = state.reactivity_net_dollar > 1.0;
    state.burst_reactivity_dollar = rho_inserted;
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if !(scale > 0.0) {
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

/// The eight variables in the order of the Runge–Kutta step: P, C_1 to C_6, E.
fn state_vector(state: &ReactorState) -> [f64; 8] {
    [
        state.power_mw,
        state.precursor_mw[0],
        state.precursor_mw[1],
        state.precursor_mw[2],
        state.precursor_mw[3],
        state.precursor_mw[4],
        state.precursor_mw[5],
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

/// The right-hand side of the four equations at `y`, with `a E` from the energy in `y`.
fn rates(reactor: &ReactorSpert3, rho_inserted_dollar: f64, y: &[f64; 8]) -> [f64; 8] {
    let rho_net = rho_inserted_dollar - reactor.feedback_strength_dollar_per_mws * y[7];
    let l_star = reactor.generation_time_reduced_s;
    let mut out = [0.0; 8];
    let mut delayed = 0.0;
    let mut index = 0;
    while index < 6 {
        let decay = reactor.group_decay_per_s[index];
        delayed += decay * y[1 + index];
        out[1 + index] = reactor.group_fraction[index] / l_star * y[0] - decay * y[1 + index];
        index += 1;
    }
    out[0] = (rho_net - 1.0) / l_star * y[0] + delayed;
    out[7] = y[0];
    out
}

fn advance(y: &[f64; 8], k: &[f64; 8], scale: f64) -> [f64; 8] {
    let mut out = [0.0; 8];
    let mut index = 0;
    while index < 8 {
        out[index] = y[index] + scale * k[index];
        index += 1;
    }
    out
}

/// Classical fourth-order Runge–Kutta.
fn rk4(reactor: &ReactorSpert3, rho_inserted_dollar: f64, y: &[f64; 8], dt_s: f64) -> [f64; 8] {
    let k1 = rates(reactor, rho_inserted_dollar, y);
    let k2 = rates(reactor, rho_inserted_dollar, &advance(y, &k1, 0.5 * dt_s));
    let k3 = rates(reactor, rho_inserted_dollar, &advance(y, &k2, 0.5 * dt_s));
    let k4 = rates(reactor, rho_inserted_dollar, &advance(y, &k3, dt_s));
    let mut out = [0.0; 8];
    let mut index = 0;
    while index < 8 {
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
    frame.vel[0] = state.peak_power_mw as f32;
    frame.vel[1] = state.peak_energy_mws as f32;
    frame.vel[2] = state.peak_time_s as f32;
    frame.force_torque = state.power_rate_mw_per_s as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = if state.prompt_supercritical {
        FLAG_PROMPT_SUPERCRITICAL
    } else {
        0
    };
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
        && state.energy_mws.is_finite()
        && state.power_rate_mw_per_s.is_finite()
        && state.reactivity_net_dollar.is_finite()
        && state.coolant_temperature_k.is_finite()
        && state.population_anchor_mw.is_finite()
        && state.reactivity_energy_integral_dollar_mws.is_finite()
        && state.population_defect_mw.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.peak_power_mw.is_finite()
        && state.peak_energy_mws.is_finite()
        && state.peak_time_s.is_finite()
        && state.burst_reactivity_dollar.is_finite()
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
        COOLANT_TEMPERATURE_MAX_K, COOLANT_TEMPERATURE_MIN_K, ENERGY_MAX_MWS,
        FEEDBACK_STRENGTH_DOLLAR_PER_MWS, FLAG_PROMPT_SUPERCRITICAL, FLOOR_DT_S,
        GATE_PHASE_STEP, GENERATION_TIME_REDUCED_S, GROUP_DECAY_PER_S, GROUP_FRACTION,
        INITIAL_POWER_MW, POWER_MAX_MW, REACTIVITY_MAX_DOLLAR, REACTIVITY_MIN_DOLLAR,
        ReactorLoad, ReactorSpert3, ReactorState, RESIDUAL_CONSERVATION_LIMIT, StepReject,
        TIME_MAX_S, advance, commit, population_mw, rates, rk4, state_vector, step, validate,
    };
    use crate::frame::LastStateFrame64;

    /// Fahrenheit to kelvin as the research scripts write it.
    fn kelvin(fahrenheit: f64) -> f64 {
        (fahrenheit - 32.0) / 1.8 + 273.15
    }

    /// The 20 opened tests of the protocol: test number, primary coolant degF,
    /// and the inhour-equation reactivity in dollars of the tabulated reactor
    /// period for this card's six-group set and generation time.
    /// `analysis/02_structures.py`, `inhour_rho`, called by the scratch script
    /// `fc.py` on `common.opened_tests()`, 2026-10-07. Periods and temperatures
    /// are inputs; no measured outcome is in this table.
    const OPENED: [(u32, f64, f64); 20] = [
        (22, 74.0, 0.7686211839115998),
        (13, 63.0, 0.9333518712956813),
        (14, 64.0, 0.9372672225604647),
        (39, 65.0, 0.9723163983875146),
        (23, 70.0, 0.9733763113187229),
        (45, 78.0, 0.9814013015886512),
        (46, 69.0, 0.983104783538945),
        (71, 92.0, 0.983104783538945),
        (17, 68.0, 0.9849826193672231),
        (74, 67.0, 0.9862671236313341),
        (51, 82.0, 0.9982818460448638),
        (49, 76.0, 1.0015184704712061),
        (16, 67.0, 1.0099978923705826),
        (19, 69.0, 1.0289530946694856),
        (20, 72.0, 1.0453791131984211),
        (75, 68.0, 1.0499736521774554),
        (40, 67.0, 1.087436264588105),
        (48, 74.0, 1.0920117761564323),
        (42, 78.0, 1.164652027326959),
        (43, 78.0, 1.2102315121058187),
    ];

    /// The forecast of structure B, `analysis/02_structures.py` equations with
    /// the constant 0.0369864 of `analysis/out/03_fit_and_score_B.json`, load
    /// the inhour reactivity above. Made by the scratch script `fc2.py`
    /// (a read-only copy of `analysis/` with `common.py` and
    /// `02_structures.py`): `scipy.integrate.solve_ivp`, LSODA, rtol 1e-12,
    /// atol 1e-16, terminal event at the first maximum of power. Checked by
    /// fixed-step RK4 at 2e-5 s with a cubic Hermite peak in the same script:
    /// the two agree to 1e-9 relative or better on all three quantities.
    /// Columns: test, peak power MW, energy released to peak MW s, time of peak
    /// s, then the report's Table III values for the same test (peak power,
    /// energy to peak, time to peak), kept for the comparison in the report.
    const FORECAST: [(u32, f64, f64, f64, f64, f64, f64); 8] = [
        (22, 2.1246087309581156, 7.235251822533072, 12.227976374091075, 2.1, 6.9, 13.7),
        (14, 5.72430615719737, 4.512822665935842, 2.5297300115869605, 5.6, 5.3, 3.2),
        (17, 8.871512715238797, 2.7342738173437344, 1.1168748479869794, 8.6, 3.3, 1.2),
        (16, 13.265880608507153, 1.9158978201721397, 0.6873629849174443, 12.0, 2.0, 0.86),
        (20, 28.5895834112238, 2.1278744263413545, 0.42662526186224325, 26.0, 2.0, 0.54),
        (40, 64.92784401185119, 2.925803017342673, 0.29066140111715555, 59.0, 2.8, 0.39),
        (42, 188.00280431236735, 4.775126190804758, 0.18471011800016954, 170.0, 4.6, 0.25),
        (43, 295.5982345727385, 5.941492757921232, 0.15285191807492354, 280.0, 6.0, 0.23),
    ];

    fn blank_frame() -> LastStateFrame64 {
        LastStateFrame64 {
            t: -1.0,
            pos: [1.0, 2.0, 3.0],
            vel: [4.0, 5.0, 6.0],
            force_torque: 7.0,
            residual: 8.0,
            flags: 9,
            proof: [0xA5; 16],
        }
    }

    fn proof_intact(frame: &LastStateFrame64) {
        let mut index = 0;
        while index < 16 {
            assert_eq!(frame.proof[index], 0xA5, "proof byte {index} was resealed");
            index += 1;
        }
    }

    fn assert_copy<T: Copy>() {}

    fn state_bits(state: &ReactorState) -> [u64; 22] {
        [
            state.t_s.to_bits(),
            state.power_mw.to_bits(),
            state.precursor_mw[0].to_bits(),
            state.precursor_mw[1].to_bits(),
            state.precursor_mw[2].to_bits(),
            state.precursor_mw[3].to_bits(),
            state.precursor_mw[4].to_bits(),
            state.precursor_mw[5].to_bits(),
            state.energy_mws.to_bits(),
            state.power_rate_mw_per_s.to_bits(),
            state.reactivity_net_dollar.to_bits(),
            state.coolant_temperature_k.to_bits(),
            state.population_anchor_mw.to_bits(),
            state.reactivity_energy_integral_dollar_mws.to_bits(),
            state.population_defect_mw.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.peak_power_mw.to_bits(),
            state.peak_energy_mws.to_bits(),
            state.peak_time_s.to_bits(),
            u64::from(state.past_first_maximum),
            u64::from(state.prompt_supercritical),
            state.burst_reactivity_dollar.to_bits(),
        ]
    }

    fn load(rho: f64) -> ReactorLoad {
        ReactorLoad {
            reactivity_inserted_dollar: rho,
        }
    }

    fn within(got: f64, expected: f64, relative: f64) -> bool {
        (got - expected).abs() <= relative * expected.abs()
    }

    fn start(fahrenheit: f64) -> (ReactorSpert3, ReactorState) {
        let reactor = ReactorSpert3::e_core_cold_startup();
        let state = ReactorState::try_cold_start(&reactor, kelvin(fahrenheit)).unwrap();
        (reactor, state)
    }

    /// The same card with the feedback switched off. Not valid; used only by the test-only steps.
    fn without_feedback() -> ReactorSpert3 {
        let mut reactor = ReactorSpert3::e_core_cold_startup();
        reactor.feedback_strength_dollar_per_mws = 0.0;
        reactor
    }

    /// The real `validate`, a chosen integrator, the real `commit`.
    fn step_with(
        reactor: &ReactorSpert3,
        state: &mut ReactorState,
        frame: &mut LastStateFrame64,
        rho: f64,
        dt_s: f64,
        integrate: impl FnOnce(&ReactorSpert3, f64, &[f64; 8], f64) -> [f64; 8],
    ) -> Result<(), StepReject> {
        validate(reactor, state, load(rho), dt_s)?;
        let advanced = integrate(reactor, rho, &state_vector(state), dt_s);
        commit(reactor, state, frame, load(rho), dt_s, advanced)
    }

    /// The real `validate` and `commit`, the real Runge–Kutta step, one alteration of its result.
    fn step_altered(
        reactor: &ReactorSpert3,
        state: &mut ReactorState,
        frame: &mut LastStateFrame64,
        rho: f64,
        dt_s: f64,
        alter: impl FnOnce(&ReactorState, &mut [f64; 8]),
    ) -> Result<(), StepReject> {
        let held = *state;
        step_with(reactor, state, frame, rho, dt_s, |r, rho, y, dt| {
            let mut out = rk4(r, rho, y, dt);
            alter(&held, &mut out);
            out
        })
    }

    fn euler(reactor: &ReactorSpert3, rho: f64, y: &[f64; 8], dt_s: f64) -> [f64; 8] {
        advance(y, &rates(reactor, rho, y), dt_s)
    }

    /// Fourth-order weights with the energy of the feedback frozen at the start of the step.
    fn feedback_frozen_at_the_start(
        reactor: &ReactorSpert3,
        rho: f64,
        y: &[f64; 8],
        dt_s: f64,
    ) -> [f64; 8] {
        let frozen = rho - reactor.feedback_strength_dollar_per_mws * y[7];
        let mut no_feedback = *reactor;
        no_feedback.feedback_strength_dollar_per_mws = 0.0;
        rk4(&no_feedback, frozen, y, dt_s)
    }

    /// Midpoint rule, second order.
    fn midpoint(reactor: &ReactorSpert3, rho: f64, y: &[f64; 8], dt_s: f64) -> [f64; 8] {
        let k1 = rates(reactor, rho, y);
        let k2 = rates(reactor, rho, &advance(y, &k1, 0.5 * dt_s));
        advance(y, &k2, dt_s)
    }

    struct Run {
        previous: ReactorState,
        last: ReactorState,
        end: StepReject,
        steps: u32,
        worst_residual: f64,
        always_rising: bool,
        frame: LastStateFrame64,
    }

    /// Step at `dt_s` under a held load until the card refuses.
    fn run_to_first_maximum(fahrenheit: f64, rho: f64, dt_s: f64) -> Run {
        let (reactor, mut state) = start(fahrenheit);
        let mut frame = blank_frame();
        let mut previous = state;
        let mut steps = 0;
        let mut worst_residual = 0.0_f64;
        let mut always_rising = true;
        loop {
            previous = if state.past_first_maximum { previous } else { state };
            match step(&reactor, &mut state, &mut frame, load(rho), dt_s) {
                Ok(()) => {
                    steps += 1;
                    if state.residual_conservation_error > worst_residual {
                        worst_residual = state.residual_conservation_error;
                    }
                    if !state.past_first_maximum && !(state.power_rate_mw_per_s > 0.0) {
                        always_rising = false;
                    }
                }
                Err(end) => {
                    return Run {
                        previous,
                        last: state,
                        end,
                        steps,
                        worst_residual,
                        always_rising,
                        frame,
                    };
                }
            }
        }
    }

    /// Cubic Hermite through two landed samples, using the power, the power
    /// rate and the energy of each. Returns peak power, energy released and
    /// time at the stationary point of the power. The caller's part of reading
    /// the peak, not the card's.
    fn hermite_peak(previous: &ReactorState, last: &ReactorState) -> (f64, f64, f64) {
        let h = last.t_s - previous.t_s;
        let (p0, p1) = (previous.power_mw, last.power_mw);
        let (d0, d1) = (previous.power_rate_mw_per_s * h, last.power_rate_mw_per_s * h);
        let derivative = |s: f64| {
            (6.0 * s * s - 6.0 * s) * p0
                + (3.0 * s * s - 4.0 * s + 1.0) * d0
                + (-6.0 * s * s + 6.0 * s) * p1
                + (3.0 * s * s - 2.0 * s) * d1
        };
        let (mut low, mut high) = (0.0_f64, 1.0_f64);
        let mut round = 0;
        while round < 200 {
            let middle = 0.5 * (low + high);
            if derivative(middle) > 0.0 {
                low = middle;
            } else {
                high = middle;
            }
            round += 1;
        }
        let s = 0.5 * (low + high);
        let (a, b, c, d) = (
            2.0 * s * s * s - 3.0 * s * s + 1.0,
            s * s * s - 2.0 * s * s + s,
            -2.0 * s * s * s + 3.0 * s * s,
            s * s * s - s * s,
        );
        let power = a * p0 + b * d0 + c * p1 + d * d1;
        let energy = a * previous.energy_mws
            + b * p0 * h
            + c * last.energy_mws
            + d * p1 * h;
        (power, energy, previous.t_s + s * h)
    }

    fn ceiling() -> f64 {
        ReactorSpert3::e_core_cold_startup().gate_dt_s().unwrap()
    }

    #[test]
    fn constants_are_locked() {
        assert_eq!(GENERATION_TIME_REDUCED_S.to_bits(), 0.00215_f64.to_bits());
        let fractions = [0.0352, 0.2014, 0.1858, 0.4006, 0.1437, 0.0333_f64];
        let decays = [0.0127, 0.0317, 0.1167, 0.3142, 1.4007, 3.8803_f64];
        let mut index = 0;
        while index < 6 {
            assert_eq!(GROUP_FRACTION[index].to_bits(), fractions[index].to_bits());
            assert_eq!(GROUP_DECAY_PER_S[index].to_bits(), decays[index].to_bits());
            index += 1;
        }
        assert_eq!(INITIAL_POWER_MW.to_bits(), 5.0e-5_f64.to_bits());
        assert_eq!(
            FEEDBACK_STRENGTH_DOLLAR_PER_MWS.to_bits(),
            0.0369864_f64.to_bits()
        );
        assert_eq!(REACTIVITY_MIN_DOLLAR.to_bits(), 0.7686211839115998_f64.to_bits());
        assert_eq!(REACTIVITY_MAX_DOLLAR.to_bits(), 1.2102315121058187_f64.to_bits());
        // The span is the smallest and the largest reactivity of the tests, opened and held back.
        assert_eq!(REACTIVITY_MIN_DOLLAR.to_bits(), OPENED[0].2.to_bits());
        assert_eq!(REACTIVITY_MAX_DOLLAR.to_bits(), OPENED[19].2.to_bits());
        assert_eq!(COOLANT_TEMPERATURE_MIN_K.to_bits(), kelvin(56.0).to_bits());
        assert_eq!(COOLANT_TEMPERATURE_MAX_K.to_bits(), kelvin(92.0).to_bits());
        assert_eq!(ENERGY_MAX_MWS.to_bits(), 10.0_f64.to_bits());
        assert_eq!(POWER_MAX_MW.to_bits(), 1000.0_f64.to_bits());
        assert_eq!(TIME_MAX_S.to_bits(), 20.0_f64.to_bits());
        assert_eq!(RESIDUAL_CONSERVATION_LIMIT.to_bits(), 1.0e-7_f64.to_bits());
        assert_eq!(GATE_PHASE_STEP.to_bits(), 0.2_f64.to_bits());
        assert_eq!(FLOOR_DT_S.to_bits(), 1.0e-9_f64.to_bits());
        assert_eq!(FLAG_PROMPT_SUPERCRITICAL, 1);

        let reactor = ReactorSpert3::e_core_cold_startup();
        assert!(reactor.is_valid());
        assert_eq!(reactor.generation_time_reduced_s.to_bits(), GENERATION_TIME_REDUCED_S.to_bits());
        assert_eq!(reactor.initial_power_mw.to_bits(), INITIAL_POWER_MW.to_bits());
        assert_eq!(reactor.group_fraction, GROUP_FRACTION);
        assert_eq!(reactor.group_decay_per_s, GROUP_DECAY_PER_S);

        // Every recorded copy, changed by one bit, is invalid.
        let mut tampered = std::vec::Vec::new();
        let mut one = reactor;
        one.generation_time_reduced_s = f64::from_bits(one.generation_time_reduced_s.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.initial_power_mw = f64::from_bits(one.initial_power_mw.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.feedback_strength_dollar_per_mws =
            f64::from_bits(one.feedback_strength_dollar_per_mws.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.reactivity_min_dollar = f64::from_bits(one.reactivity_min_dollar.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.reactivity_max_dollar = f64::from_bits(one.reactivity_max_dollar.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.coolant_temperature_min_k = f64::from_bits(one.coolant_temperature_min_k.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.coolant_temperature_max_k = f64::from_bits(one.coolant_temperature_max_k.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.energy_max_mws = f64::from_bits(one.energy_max_mws.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.power_max_mw = f64::from_bits(one.power_max_mw.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.time_max_s = f64::from_bits(one.time_max_s.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.residual_conservation_limit =
            f64::from_bits(one.residual_conservation_limit.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.gate_phase_step = f64::from_bits(one.gate_phase_step.to_bits() + 1);
        tampered.push(one);
        let mut one = reactor;
        one.floor_dt_s = f64::from_bits(one.floor_dt_s.to_bits() + 1);
        tampered.push(one);
        let mut group = 0;
        while group < 6 {
            let mut one = reactor;
            one.group_fraction[group] = f64::from_bits(one.group_fraction[group].to_bits() + 1);
            tampered.push(one);
            let mut one = reactor;
            one.group_decay_per_s[group] =
                f64::from_bits(one.group_decay_per_s[group].to_bits() + 1);
            tampered.push(one);
            group += 1;
        }
        let mut count = 0;
        for card in tampered.iter() {
            assert!(!card.is_valid());
            count += 1;
        }
        assert_eq!(count, 25);
        let mut nan = reactor;
        nan.feedback_strength_dollar_per_mws = f64::NAN;
        assert!(!nan.is_valid());
        // The fractions sum to one.
        let mut sum = 0.0;
        index = 0;
        while index < 6 {
            sum += GROUP_FRACTION[index];
            index += 1;
        }
        assert!((sum - 1.0).abs() < 1.0e-15);
    }

    #[test]
    fn the_cold_start_is_the_equilibrium_at_the_locked_power() {
        let (reactor, state) = start(70.0);
        assert_eq!(state.t_s, 0.0);
        assert_eq!(state.power_mw.to_bits(), 5.0e-5_f64.to_bits());
        assert_eq!(state.energy_mws, 0.0);
        let mut index = 0;
        while index < 6 {
            let expected = GROUP_FRACTION[index] * 5.0e-5 / (0.00215 * GROUP_DECAY_PER_S[index]);
            assert_eq!(state.precursor_mw[index].to_bits(), expected.to_bits());
            index += 1;
        }
        assert_eq!(state.coolant_temperature_k.to_bits(), kelvin(70.0).to_bits());
        assert_eq!(state.peak_power_mw.to_bits(), 5.0e-5_f64.to_bits());
        assert_eq!(state.peak_energy_mws, 0.0);
        assert_eq!(state.peak_time_s, 0.0);
        assert!(!state.past_first_maximum);
        assert!(!state.prompt_supercritical);
        assert_eq!(state.residual_conservation_error, 0.0);
        assert_eq!(state.population_defect_mw, 0.0);
        let mut population = state.power_mw;
        index = 0;
        while index < 6 {
            population += state.precursor_mw[index];
            index += 1;
        }
        assert_eq!(state.population_anchor_mw.to_bits(), population.to_bits());
        // Each rate is zero to rounding at zero net reactivity.
        let r = rates(&reactor, 0.0, &state_vector(&state));
        let scale = 5.0e-5 / 0.00215;
        index = 0;
        while index < 7 {
            assert!(r[index].abs() <= 1.0e-12 * scale, "rate {index} is {}", r[index]);
            index += 1;
        }
        assert_eq!(r[7].to_bits(), 5.0e-5_f64.to_bits());

        assert!(ReactorState::try_cold_start(&reactor, COOLANT_TEMPERATURE_MIN_K).is_some());
        assert!(ReactorState::try_cold_start(&reactor, COOLANT_TEMPERATURE_MAX_K).is_some());
        assert!(ReactorState::try_cold_start(&reactor, kelvin(55.9)).is_none());
        assert!(ReactorState::try_cold_start(&reactor, kelvin(92.1)).is_none());
        assert!(ReactorState::try_cold_start(&reactor, f64::NAN).is_none());
        assert!(ReactorState::try_cold_start(&reactor, f64::INFINITY).is_none());
        assert!(ReactorState::try_cold_start(&reactor, f64::NEG_INFINITY).is_none());
        let mut broken = reactor;
        broken.initial_power_mw = 1.0;
        assert!(ReactorState::try_cold_start(&broken, kelvin(70.0)).is_none());
    }

    #[test]
    fn at_zero_net_reactivity_and_no_feedback_the_state_stays_at_equilibrium() {
        let reactor = without_feedback();
        let (_, state) = start(70.0);
        let start_y = state_vector(&state);
        let mut y = start_y;
        let dt = ceiling();
        let mut t = 0.0;
        while t < 1.0 {
            y = rk4(&reactor, 0.0, &y, dt);
            t += dt;
        }
        let mut index = 0;
        while index < 7 {
            assert!(
                within(y[index], start_y[index], 1.0e-12),
                "variable {index}: {} against {}",
                y[index],
                start_y[index]
            );
            index += 1;
        }
        assert!(within(y[7], 5.0e-5 * t, 1.0e-12));
    }

    #[test]
    fn every_refusal_is_named_and_rolls_back_state_and_frame() {
        let (reactor, fresh) = start(70.0);
        let mut frame = blank_frame();
        let held = frame.to_bytes();
        let ok = 0.95;

        let mut refuse = |mut state: ReactorState,
                          rho: f64,
                          dt_s: f64,
                          card: &ReactorSpert3,
                          expect: StepReject| {
            let saved = state_bits(&state);
            let got = step(card, &mut state, &mut frame, load(rho), dt_s);
            assert_eq!(got, Err(expect), "rho {rho} dt {dt_s}");
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
            proof_intact(&frame);
        };

        refuse(fresh, ok, f64::NAN, &reactor, StepReject::NonFiniteInput);
        refuse(fresh, ok, f64::INFINITY, &reactor, StepReject::NonFiniteInput);
        refuse(fresh, ok, f64::NEG_INFINITY, &reactor, StepReject::NonFiniteInput);
        refuse(fresh, f64::NAN, 1.0e-3, &reactor, StepReject::NonFiniteInput);
        refuse(fresh, f64::INFINITY, 1.0e-3, &reactor, StepReject::NonFiniteInput);
        refuse(fresh, ok, 0.0, &reactor, StepReject::NonPositiveDt);
        refuse(fresh, ok, -1.0e-3, &reactor, StepReject::NonPositiveDt);

        let mut nan_power = fresh;
        nan_power.power_mw = f64::NAN;
        refuse(nan_power, ok, 1.0e-3, &reactor, StepReject::NonFiniteState);
        let mut nan_precursor = fresh;
        nan_precursor.precursor_mw[4] = f64::INFINITY;
        refuse(nan_precursor, ok, 1.0e-3, &reactor, StepReject::NonFiniteState);
        let mut nan_ledger = fresh;
        nan_ledger.population_anchor_mw = f64::NAN;
        refuse(nan_ledger, ok, 1.0e-3, &reactor, StepReject::NonFiniteState);

        let mut tampered = reactor;
        tampered.feedback_strength_dollar_per_mws = 0.0;
        refuse(fresh, ok, 1.0e-3, &tampered, StepReject::InvalidCard);

        refuse(fresh, 0.7679, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        refuse(fresh, 1.2111, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        // The old outward-rounded ends, which no test reached, and the 64-bit numbers just past
        // the tested ends.
        refuse(fresh, 0.768, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        refuse(fresh, 1.211, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        let below = f64::from_bits(REACTIVITY_MIN_DOLLAR.to_bits() - 1);
        let above = f64::from_bits(REACTIVITY_MAX_DOLLAR.to_bits() + 1);
        refuse(fresh, below, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        refuse(fresh, above, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        refuse(fresh, 0.0, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        refuse(fresh, -1.0, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        refuse(fresh, 2.0, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);

        let mut cold = fresh;
        cold.coolant_temperature_k = kelvin(55.0);
        refuse(cold, ok, 1.0e-3, &reactor, StepReject::CoolantTemperatureOutsideCard);
        let mut hot = fresh;
        hot.coolant_temperature_k = kelvin(93.0);
        refuse(hot, ok, 1.0e-3, &reactor, StepReject::CoolantTemperatureOutsideCard);

        let mut late = fresh;
        late.t_s = TIME_MAX_S - 0.5;
        refuse(late, ok, 1.0, &reactor, StepReject::TimeBeyondCard);
        let mut early = fresh;
        early.t_s = -1.0;
        refuse(early, ok, 1.0e-3, &reactor, StepReject::TimeBeyondCard);
        refuse(fresh, ok, TIME_MAX_S + 1.0, &reactor, StepReject::TimeBeyondCard);
        refuse(fresh, ok, f64::MAX, &reactor, StepReject::TimeBeyondCard);

        // A state written by hand is not one the card writes. At time zero only the card's
        // start is taken; an energy or a power past the card's limits is named as that, and
        // an energy below zero or a power not above zero as a negative state.
        let mut full = fresh;
        full.burst_reactivity_dollar = ok;
        full.energy_mws = 9.9999;
        full.power_mw = 100.0;
        refuse(full, ok, 1.0e-3, &reactor, StepReject::InconsistentState);
        let mut over = full;
        over.energy_mws = 10.0001;
        refuse(over, ok, 1.0e-3, &reactor, StepReject::EnergyBeyondCard);
        let mut loud = fresh;
        loud.burst_reactivity_dollar = 1.2;
        loud.power_mw = 999.9999;
        loud.precursor_mw = [1.0e5; 6];
        refuse(loud, 1.2, 1.0e-3, &reactor, StepReject::InconsistentState);
        loud.power_mw = 1000.0001;
        refuse(loud, 1.2, 1.0e-3, &reactor, StepReject::PowerBeyondCard);
        let mut peaked = fresh;
        peaked.peak_power_mw = 1000.0001;
        refuse(peaked, ok, 1.0e-3, &reactor, StepReject::PowerBeyondCard);
        let mut negative = fresh;
        negative.energy_mws = -1.0e-9;
        refuse(negative, ok, 1.0e-3, &reactor, StepReject::NegativeState);
        let mut dark = fresh;
        dark.power_mw = 0.0;
        refuse(dark, ok, 1.0e-3, &reactor, StepReject::NegativeState);
        let mut emptied = fresh;
        emptied.precursor_mw[3] = -1.0;
        refuse(emptied, ok, 1.0e-3, &reactor, StepReject::NegativeState);
        // A state the card wrote, with one derived field or one peak field moved by a bit.
        let mut full = fresh;
        let mut scratch = blank_frame();
        for _ in 0..50 {
            step(&reactor, &mut full, &mut scratch, load(ok), 1.0e-3).unwrap();
        }
        let moved: [fn(&mut ReactorState); 9] = [
            |s| s.reactivity_net_dollar = f64::from_bits(s.reactivity_net_dollar.to_bits() + 1),
            |s| s.power_rate_mw_per_s = f64::from_bits(s.power_rate_mw_per_s.to_bits() + 1),
            |s| s.population_defect_mw = f64::from_bits(s.population_defect_mw.to_bits() + 1),
            |s| s.residual_conservation_error = 0.0,
            |s| s.population_anchor_mw = f64::from_bits(s.population_anchor_mw.to_bits() + 1),
            |s| s.prompt_supercritical = !s.prompt_supercritical,
            |s| s.peak_power_mw = 0.5 * s.power_mw,
            |s| s.peak_time_s = s.t_s + 1.0,
            |s| s.peak_energy_mws = s.energy_mws + 1.0,
        ];
        for change in moved {
            let mut state = full;
            change(&mut state);
            refuse(state, ok, 1.0e-3, &reactor, StepReject::InconsistentState);
        }
        // The power or the energy moved with nothing else: the derived fields no longer follow.
        let mut louder = full;
        louder.power_mw *= 1.000_001;
        refuse(louder, ok, 1.0e-3, &reactor, StepReject::InconsistentState);
        let mut spent = full;
        spent.energy_mws *= 1.000_001;
        refuse(spent, ok, 1.0e-3, &reactor, StepReject::InconsistentState);
        // The landing past a limit is still named at the landing.
        let mut landing = full;
        let mut advanced = rk4(&reactor, ok, &state_vector(&landing), 1.0e-3);
        advanced[7] = 10.0001;
        assert_eq!(
            commit(&reactor, &mut landing, &mut scratch, load(ok), 1.0e-3, advanced),
            Err(StepReject::EnergyBeyondCard)
        );
        let mut advanced = rk4(&reactor, ok, &state_vector(&landing), 1.0e-3);
        advanced[0] = 1000.0001;
        assert_eq!(
            commit(&reactor, &mut landing, &mut scratch, load(ok), 1.0e-3, advanced),
            Err(StepReject::PowerBeyondCard)
        );
        assert_eq!(state_bits(&landing), state_bits(&full));

        let mut after = fresh;
        after.past_first_maximum = true;
        refuse(after, ok, 1.0e-3, &reactor, StepReject::PastFirstMaximum);

        // The edges of the span land.
        for rho in [REACTIVITY_MIN_DOLLAR, REACTIVITY_MAX_DOLLAR] {
            let mut state = fresh;
            let mut landed = blank_frame();
            step(&reactor, &mut state, &mut landed, load(rho), 1.0e-4).unwrap();
        }
        // The order of the checks: the step size before the load, the load before the state's clock.
        refuse(fresh, f64::NAN, f64::NAN, &reactor, StepReject::NonFiniteInput);
        refuse(nan_power, 0.0, 0.0, &reactor, StepReject::NonFiniteState);
        refuse(fresh, 0.0, 0.0, &reactor, StepReject::NonPositiveDt);
        refuse(late, 0.0, 1.0, &reactor, StepReject::ReactivityOutsideCard);

        // A reactivity that is not the burst's. The state without its burst recorded is not
        // the start, and no load continues it.
        refuse(full, 1.0, 1.0e-3, &reactor, StepReject::ReactivityChangedDuringBurst);
        let mut unrecorded = full;
        unrecorded.burst_reactivity_dollar = 0.0;
        refuse(unrecorded, ok, 1.0e-3, &reactor, StepReject::ReactivityChangedDuringBurst);
        let mut nan_burst = fresh;
        nan_burst.burst_reactivity_dollar = f64::NAN;
        refuse(nan_burst, ok, 1.0e-3, &reactor, StepReject::NonFiniteState);
        // The span, the first maximum and the clock are named before it.
        refuse(full, 2.0, 1.0e-3, &reactor, StepReject::ReactivityOutsideCard);
        let mut after_other = full;
        after_other.past_first_maximum = true;
        refuse(after_other, 1.0, 1.0e-3, &reactor, StepReject::PastFirstMaximum);
        let mut late_other = full;
        late_other.t_s = TIME_MAX_S - 0.5;
        refuse(late_other, 1.0, 1.0, &reactor, StepReject::TimeBeyondCard);
    }

    /// The request that was sealed before this refusal existed: 100 steps at the ceiling at
    /// 1.1 dollars, then one step at 1.2 (it landed, with the net reactivity at 1.19999), or
    /// one at 0.9 (it landed as a "first maximum" of 0.0172 MW at 0.0714 s).
    #[test]
    fn a_reactivity_changed_during_the_burst_is_refused_and_the_burst_goes_on_unchanged() {
        let (reactor, mut state) = start(70.0);
        let mut frame = blank_frame();
        let dt = ceiling();
        assert_eq!(state.burst_reactivity_dollar, 0.0);

        let refused = |state: &ReactorState, frame: &LastStateFrame64, rho: f64| {
            let mut tried = *state;
            let mut tried_frame = *frame;
            assert_eq!(
                step(&reactor, &mut tried, &mut tried_frame, load(rho), dt),
                Err(StepReject::ReactivityChangedDuringBurst),
                "rho {rho}"
            );
            assert_eq!(state_bits(&tried), state_bits(state));
            assert_eq!(tried_frame.to_bytes(), frame.to_bytes());
            proof_intact(&tried_frame);
        };

        let mut count = 0;
        while count < 100 {
            step(&reactor, &mut state, &mut frame, load(1.1), dt).unwrap();
            assert_eq!(state.burst_reactivity_dollar.to_bits(), 1.1_f64.to_bits());
            // From the first landed step on, every other reactivity is refused: above, below,
            // each edge of the span, and the 64-bit number next to the burst's own.
            if count == 0 || count == 1 || count == 49 || count == 99 {
                refused(&state, &frame, 1.2);
                refused(&state, &frame, 0.9);
                refused(&state, &frame, REACTIVITY_MIN_DOLLAR);
                refused(&state, &frame, REACTIVITY_MAX_DOLLAR);
                refused(&state, &frame, f64::from_bits(1.1_f64.to_bits() + 1));
                refused(&state, &frame, f64::from_bits(1.1_f64.to_bits() - 1));
            }
            count += 1;
        }
        assert!((state.t_s - 100.0 * dt).abs() < 1.0e-12);
        assert!(state.power_rate_mw_per_s > 0.0 && !state.past_first_maximum);
        // The state the old step sealed after the change is not reached: the net reactivity is
        // still the burst's own less the feedback, and no maximum has been declared.
        assert!(state.reactivity_net_dollar < 1.1);
        assert_eq!(
            state.reactivity_net_dollar.to_bits(),
            (1.1 - FEEDBACK_STRENGTH_DOLLAR_PER_MWS * state.energy_mws).to_bits()
        );
        // A reactivity outside the span is named as that, as before.
        let mut tried = state;
        let mut tried_frame = frame;
        assert_eq!(
            step(&reactor, &mut tried, &mut tried_frame, load(1.3), dt),
            Err(StepReject::ReactivityOutsideCard)
        );

        // After the refusals the burst goes on under its own reactivity and ends, bit for bit,
        // where the same burst ends when nothing else was ever asked of it.
        let end = loop {
            if let Err(end) = step(&reactor, &mut state, &mut frame, load(1.1), dt) {
                break end;
            }
        };
        assert_eq!(end, StepReject::PastFirstMaximum);
        let whole = run_to_first_maximum(70.0, 1.1, dt);
        assert_eq!(state_bits(&state), state_bits(&whole.last));
        assert_eq!(frame.to_bytes(), whole.frame.to_bytes());
        // Past the first maximum, that is the name, whatever the load.
        let mut tried = state;
        assert_eq!(
            step(&reactor, &mut tried, &mut frame, load(1.2), dt),
            Err(StepReject::PastFirstMaximum)
        );

        // The start takes any reactivity of the span, and a new start is a new burst.
        for rho in [REACTIVITY_MIN_DOLLAR, 0.9, 1.1, 1.2, REACTIVITY_MAX_DOLLAR] {
            let (reactor, mut state) = start(70.0);
            let mut frame = blank_frame();
            step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
            assert_eq!(state.burst_reactivity_dollar.to_bits(), rho.to_bits());
        }
    }

    /// Three answers the door sealed for this card before the step refused a changed
    /// reactivity: the runs to the refusal of the opened tests 22, 20 and 43 in the rehearsal of
    /// the first comparison (`analysis/out/09_rehearsal_engine_0.2.1/` of the research record,
    /// engine 0.2.1; the engine of 0.4.0 gives the same states). Coolant K, reactivity and step
    /// as 64-bit patterns, landed steps, the prompt flag, and the 19 numbers of the final state
    /// in the order the door publishes them.
    const SEALED: [(u32, [u64; 3], u32, bool, [u64; 19]); 3] = [
        (
            22,
            [0x407287bbbbbbbbbb, 0x3fe8988b73fd384d, 0x3f47693780a0b823],
            17116,
            false,
            [
                0x402874faeb6fed36, 0x4000ff32da7a7ed8, 0x401cf1fc9a9e94ee, 0xbf11cc455e400000,
                0x3fe007feafeb2e96, 0x407287bbbbbbbbbb, 0x405cdfaf0aba0040, 0x4083df1a3dd0a361,
                0x407f335b8d03ddf9, 0x408855ab60410162, 0x40583e7eec5594b1, 0x4020e067ce03ad70,
                0x3fd2046f5f7ddbaf, 0x40125fd9522bc8ca, 0x3da1000000000000, 0x3cf04b0fefb88b4b,
                0x4000ff32dc57983d, 0x401cf06eb0b1922c, 0x4028749d4691eab3,
            ],
        ),
        (
            20,
            [0x407275f49f49f49f, 0x3ff0b9df72f1bd3d, 0x3f47693780a0b823],
            598,
            false,
            [
                0x3fdb57e3d33bb6f1, 0x403c968853e1fc78, 0x400129d7ec00619c, 0xc0145d834d7935c0,
                0x3feee9b225e25d15, 0x407275f49f49f49f, 0x40419557dea2fff2, 0x406919770e47c9fd,
                0x40670b38ff7face1, 0x407898067221bf8f, 0x4060b9c4316fdf66, 0x403ba3c051cd1010,
                0x3fd2046f5f7ddbaf, 0x400142e70accce92, 0xbda8000000000000, 0x3d087b575e444aa1,
                0x403c96ec30a785b7, 0x400100031e17978f, 0x3fdb4c2f377b6695,
            ],
        ),
        (
            43,
            [0x4072ab49f49f49f4, 0x3ff35d1bb7d15475, 0x3f47693780a0b823],
            214,
            false,
            [
                0x3fc391f4658659e1, 0x4072798db7ee3215, 0x4017d01e13f2ddf3, 0xc04c958799a42e00,
                0x3fefae6dd4b93a10, 0x4072ab49f49f49f4, 0x405860e709700b7f, 0x40816c89f3e5e94d,
                0x40800d6174ec75a2, 0x409141beee3a4ccf, 0x4078645295d671e2, 0x4055dc99399a6e12,
                0x3fd2046f5f7ddbaf, 0x401a329094aee92c, 0xbe827e9400000000, 0x3dc8dd99c92aba69,
                0x4072798db7ee3215, 0x4017d01e13f2ddf3, 0x3fc391f4658659e1,
            ],
        ),
    ];

    /// A burst under one reactivity is, bit for bit, the burst the card sealed before the
    /// refusal existed: the recorded reactivity changes no number of it.
    #[test]
    fn a_burst_under_one_reactivity_is_the_sealed_answer_bit_for_bit() {
        for (test, [coolant, rho, dt], steps, prompt, sealed) in SEALED {
            let reactor = ReactorSpert3::e_core_cold_startup();
            let mut state =
                ReactorState::try_cold_start(&reactor, f64::from_bits(coolant)).unwrap();
            let mut frame = blank_frame();
            let held = load(f64::from_bits(rho));
            let mut landed = 0;
            let end = loop {
                match step(&reactor, &mut state, &mut frame, held, f64::from_bits(dt)) {
                    Ok(()) => landed += 1,
                    Err(end) => break end,
                }
            };
            assert_eq!(end, StepReject::PastFirstMaximum, "test {test}");
            assert_eq!(landed, steps, "test {test}");
            let got = [
                state.t_s,
                state.power_mw,
                state.energy_mws,
                state.power_rate_mw_per_s,
                state.reactivity_net_dollar,
                state.coolant_temperature_k,
                state.precursor_mw[0],
                state.precursor_mw[1],
                state.precursor_mw[2],
                state.precursor_mw[3],
                state.precursor_mw[4],
                state.precursor_mw[5],
                state.population_anchor_mw,
                state.reactivity_energy_integral_dollar_mws,
                state.population_defect_mw,
                state.residual_conservation_error,
                state.peak_power_mw,
                state.peak_energy_mws,
                state.peak_time_s,
            ];
            let mut index = 0;
            while index < 19 {
                assert_eq!(got[index].to_bits(), sealed[index], "test {test} number {index}");
                index += 1;
            }
            assert!(state.past_first_maximum);
            assert_eq!(state.prompt_supercritical, prompt, "test {test}");
            assert_eq!(state.burst_reactivity_dollar.to_bits(), rho, "test {test}");
        }
    }

    /// The recorded reactivity is state and is not in the frame: two bursts that differ only
    /// in it cannot be made, and the frame of a landed step is the nine slots it always was.
    #[test]
    fn the_recorded_reactivity_is_not_written_to_the_frame() {
        let (reactor, mut state) = start(70.0);
        let mut frame = blank_frame();
        step(&reactor, &mut state, &mut frame, load(1.1), ceiling()).unwrap();
        let mut expected = blank_frame();
        expected.t = state.t_s;
        expected.pos = [
            state.power_mw as f32,
            state.energy_mws as f32,
            state.reactivity_net_dollar as f32,
        ];
        expected.vel = [
            state.peak_power_mw as f32,
            state.peak_energy_mws as f32,
            state.peak_time_s as f32,
        ];
        expected.force_torque = state.power_rate_mw_per_s as f32;
        expected.residual = state.residual_conservation_error as f32;
        expected.flags = FLAG_PROMPT_SUPERCRITICAL;
        assert_eq!(frame.to_bytes(), expected.to_bytes());
    }

    /// The recorded reactivity rewritten together with the load was once one consistent pair
    /// that landed. The step now recomputes the net reactivity and the power rate from the
    /// state under the load, and names the state as not the card's. The gateway hands no
    /// state in.
    #[test]
    fn the_recorded_reactivity_rewritten_with_the_load_is_named_as_an_inconsistent_state() {
        let (reactor, mut state) = start(70.0);
        let mut frame = blank_frame();
        let mut count = 0;
        while count < 100 {
            step(&reactor, &mut state, &mut frame, load(1.1), ceiling()).unwrap();
            count += 1;
        }
        // The field alone, the load as it was: named.
        let mut alone = state;
        alone.burst_reactivity_dollar = 1.2;
        let mut tried_frame = frame;
        assert_eq!(
            step(&reactor, &mut alone, &mut tried_frame, load(1.1), ceiling()),
            Err(StepReject::ReactivityChangedDuringBurst)
        );
        // The field and the load together: the net reactivity and the power rate the state
        // holds are those of 1.1 dollars, so the state is not one the card writes at 1.2.
        let mut both = state;
        both.burst_reactivity_dollar = 1.2;
        assert_eq!(
            step(&reactor, &mut both, &mut tried_frame, load(1.2), ceiling()),
            Err(StepReject::InconsistentState)
        );
        // The start with a reactivity already recorded is not the start: no load lands on it.
        let (_, marked) = start(70.0);
        for rho in [1.1, 1.0] {
            let mut tried = marked;
            tried.burst_reactivity_dollar = 1.0;
            assert_eq!(
                step(&reactor, &mut tried, &mut tried_frame, load(rho), ceiling()),
                Err(StepReject::InconsistentState)
            );
        }
    }

    #[test]
    fn a_step_after_the_first_maximum_is_refused_and_the_maximum_is_found_on_the_right_step() {
        let (reactor, mut state) = start(78.0);
        let mut frame = blank_frame();
        let rho = 1.164652027326959;
        let dt = ceiling();
        let mut landed_after_flag = 0;
        let mut previous_rate = f64::INFINITY;
        let end = loop {
            match step(&reactor, &mut state, &mut frame, load(rho), dt) {
                Ok(()) => {
                    if state.past_first_maximum {
                        landed_after_flag += 1;
                        // The step that sets the flag is the first with a rate that is not positive.
                        assert!(!(state.power_rate_mw_per_s > 0.0));
                        assert!(previous_rate > 0.0);
                    } else {
                        assert!(state.power_rate_mw_per_s > 0.0);
                    }
                    previous_rate = state.power_rate_mw_per_s;
                }
                Err(end) => break end,
            }
        };
        assert_eq!(end, StepReject::PastFirstMaximum);
        assert_eq!(landed_after_flag, 1);
        assert!(state.past_first_maximum);
        // The true maximum, from the forecast of test 42, lies in the last step.
        let forecast_time_s = 0.18471011800016954;
        assert!(state.t_s >= forecast_time_s);
        assert!(state.t_s - dt < forecast_time_s);
        // And the card goes on refusing it, with state and frame untouched.
        let saved = state_bits(&state);
        let bytes = frame.to_bytes();
        let mut tries = 0;
        while tries < 3 {
            assert_eq!(
                step(&reactor, &mut state, &mut frame, load(rho), dt),
                Err(StepReject::PastFirstMaximum)
            );
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), bytes);
            tries += 1;
        }
        proof_intact(&frame);
    }

    #[test]
    fn the_peak_values_are_the_highest_landed_sample() {
        for (_, fahrenheit, rho) in [OPENED[0], OPENED[8], OPENED[19]] {
            let run = run_to_first_maximum(fahrenheit, rho, ceiling());
            assert_eq!(run.end, StepReject::PastFirstMaximum);
            let last = run.last;
            let previous = run.previous;
            // The state keeps the higher of the last two samples, with its own energy and time.
            let (power, energy, time) = if last.power_mw > previous.power_mw {
                (last.power_mw, last.energy_mws, last.t_s)
            } else {
                (previous.power_mw, previous.energy_mws, previous.t_s)
            };
            assert_eq!(last.peak_power_mw.to_bits(), power.to_bits());
            assert_eq!(last.peak_energy_mws.to_bits(), energy.to_bits());
            assert_eq!(last.peak_time_s.to_bits(), time.to_bits());
            // The frame of the last landed step carries them.
            assert_eq!(run.frame.t.to_bits(), last.t_s.to_bits());
            assert_eq!(run.frame.vel[0], last.peak_power_mw as f32);
            assert_eq!(run.frame.vel[1], last.peak_energy_mws as f32);
            assert_eq!(run.frame.vel[2], last.peak_time_s as f32);
            assert_eq!(run.frame.pos[0], last.power_mw as f32);
            assert_eq!(run.frame.pos[1], last.energy_mws as f32);
            assert_eq!(run.frame.pos[2], last.reactivity_net_dollar as f32);
            assert_eq!(run.frame.force_torque, last.power_rate_mw_per_s as f32);
            assert_eq!(run.frame.residual, last.residual_conservation_error as f32);
            proof_intact(&run.frame);
            // The interpolation by the caller lies within one step of the sample and at or above it.
            let (peak, peak_energy, peak_time) = hermite_peak(&previous, &last);
            assert!(peak >= last.peak_power_mw * (1.0 - 1.0e-12));
            assert!((peak_time - last.peak_time_s).abs() <= ceiling());
            assert!(peak_energy > 0.0);
            assert!(run.always_rising);
        }
    }

    #[test]
    fn the_prompt_flag_is_only_a_flag() {
        // Test 43's load starts above one dollar and the flag is on the frame.
        let (reactor, mut state) = start(78.0);
        let mut frame = blank_frame();
        step(&reactor, &mut state, &mut frame, load(1.2102315121058187), 1.0e-4).unwrap();
        assert!(state.prompt_supercritical);
        assert_eq!(frame.flags, FLAG_PROMPT_SUPERCRITICAL);
        // Test 22's load stays under one dollar for the whole run.
        let (reactor, mut state) = start(74.0);
        let mut frame = blank_frame();
        step(&reactor, &mut state, &mut frame, load(0.7686211839115998), 1.0e-3).unwrap();
        assert!(!state.prompt_supercritical);
        assert_eq!(frame.flags, 0);
        // The feedback takes the load below one dollar before the maximum of a load above it.
        let run = run_to_first_maximum(78.0, 1.2102315121058187, ceiling());
        assert!(!run.last.prompt_supercritical);
        assert_eq!(run.frame.flags, 0);
        assert!(run.last.reactivity_net_dollar < 1.0);
    }

    #[test]
    fn types_are_copy_and_the_step_is_core_only() {
        assert_copy::<ReactorSpert3>();
        assert_copy::<ReactorState>();
        assert_copy::<ReactorLoad>();
        assert_copy::<StepReject>();
        // The module is `no_implicit_prelude` and names no allocating type; the
        // step works on arrays of eight numbers on the stack.
        assert_eq!(core::mem::size_of::<[f64; 8]>(), 64);
    }

    #[test]
    fn the_proof_is_not_the_cards() {
        let (reactor, mut state) = start(70.0);
        let mut frame = blank_frame();
        let mut count = 0;
        while count < 300 {
            step(&reactor, &mut state, &mut frame, load(0.98), 1.0e-3).unwrap();
            proof_intact(&frame);
            count += 1;
        }
        assert_eq!(
            step(&reactor, &mut state, &mut frame, load(0.98), 0.0),
            Err(StepReject::NonPositiveDt)
        );
        proof_intact(&frame);
        assert_eq!(
            step(&reactor, &mut state, &mut frame, load(5.0), 1.0e-3),
            Err(StepReject::ReactivityOutsideCard)
        );
        proof_intact(&frame);
        let got = step_altered(&reactor, &mut state, &mut frame, 0.98, 1.0e-3, |_, y| {
            y[0] *= 1.001
        });
        assert!(got.is_err());
        proof_intact(&frame);
        let mut tampered = reactor;
        tampered.generation_time_reduced_s = 0.002;
        assert_eq!(
            step(&tampered, &mut state, &mut frame, load(0.98), 1.0e-3),
            Err(StepReject::InvalidCard)
        );
        proof_intact(&frame);
    }

    #[test]
    fn explicit_euler_at_the_ceiling_is_refused_by_the_ledger() {
        let (reactor, mut state) = start(68.0);
        let mut frame = blank_frame();
        let rho = 0.9849826193672231;
        let dt = ceiling();
        let mut landed = 0;
        let end = loop {
            match step_with(&reactor, &mut state, &mut frame, rho, dt, euler) {
                Ok(()) => landed += 1,
                Err(end) => break end,
            }
        };
        assert_eq!(end, StepReject::SpontaneousGeneration);
        assert!(!state.past_first_maximum, "refused before the maximum");
        assert!(landed > 0);
        // The same load, the same step, the Runge–Kutta map: lands to the maximum.
        let run = run_to_first_maximum(68.0, rho, dt);
        assert_eq!(run.end, StepReject::PastFirstMaximum);
        assert!(run.steps > landed);
        // Euler also fails on a long-period load and on the shortest period.
        for (fahrenheit, rho) in [(74.0, 0.7686211839115998), (78.0, 1.2102315121058187)] {
            let (reactor, mut state) = start(fahrenheit);
            let mut frame = blank_frame();
            let end = loop {
                match step_with(&reactor, &mut state, &mut frame, rho, dt, euler) {
                    Ok(()) => {}
                    Err(end) => break end,
                }
            };
            assert!(
                end == StepReject::SpontaneousGeneration || end == StepReject::FirstLawBreach,
                "{end:?}"
            );
        }
    }

    #[test]
    fn wrong_stage_weights_and_a_biased_ledger_are_refused_both_ways() {
        let rho = 0.9849826193672231;
        let dt = ceiling();
        let (reactor, mut state) = start(68.0);
        let mut frame = blank_frame();
        let end = loop {
            match step_with(&reactor, &mut state, &mut frame, rho, dt, feedback_frozen_at_the_start) {
                Ok(()) => {}
                Err(end) => break end,
            }
        };
        assert!(end == StepReject::SpontaneousGeneration || end == StepReject::FirstLawBreach);
        assert!(!state.past_first_maximum);
        // The midpoint rule at the ceiling lands: its error at this step is under the limit.
        // At twice the ceiling it is refused.
        let (reactor, mut state) = start(68.0);
        let mut frame = blank_frame();
        let end = loop {
            match step_with(&reactor, &mut state, &mut frame, rho, 8.0 * dt, midpoint) {
                Ok(()) => {}
                Err(end) => break end,
            }
        };
        assert!(end == StepReject::SpontaneousGeneration || end == StepReject::FirstLawBreach);
        assert!(!state.past_first_maximum);

        // A run to the middle of the rise, then one biased step each way.
        let (reactor, mut state) = start(68.0);
        let mut frame = blank_frame();
        let mut count = 0;
        while count < 800 {
            step(&reactor, &mut state, &mut frame, load(rho), dt).unwrap();
            count += 1;
        }
        let saved = state_bits(&state);
        let held = frame.to_bytes();

        let got = step_altered(&reactor, &mut state, &mut frame, rho, dt, |held, y| {
            y[0] += 3.0 * RESIDUAL_CONSERVATION_LIMIT * population_mw(held)
        });
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);
        let got = step_altered(&reactor, &mut state, &mut frame, rho, dt, |held, y| {
            y[0] -= 3.0 * RESIDUAL_CONSERVATION_LIMIT * population_mw(held)
        });
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(state_bits(&state), saved);
        assert_eq!(frame.to_bytes(), held);

        // One group dropped from the sums.
        for group in 1..7 {
            let got = step_altered(&reactor, &mut state, &mut frame, rho, dt, |_, y| {
                y[group] = 0.0
            });
            assert_eq!(got, Err(StepReject::FirstLawBreach), "group {group}");
            assert_eq!(state_bits(&state), saved);
            assert_eq!(frame.to_bytes(), held);
        }
        // The energy booked wrongly in either direction.
        let got = step_altered(&reactor, &mut state, &mut frame, rho, dt, |_, y| {
            y[7] *= 1.0 + 1.0e-3
        });
        assert!(got.is_err());
        assert_eq!(state_bits(&state), saved);

        // A bias just under the limit lands; just over it is refused.
        let under = RESIDUAL_CONSERVATION_LIMIT * 0.5;
        let got = step_altered(&reactor, &mut state, &mut frame, rho, dt, |held, y| {
            y[0] += under * population_mw(held)
        });
        assert!(got.is_ok());
        proof_intact(&frame);
    }

    #[test]
    fn a_dropped_feedback_runs_away_and_is_refused_at_the_edge_of_the_card() {
        // The real checks, the real map, the feedback coefficient set to zero in
        // the map and the ledger.
        let dropped = without_feedback();
        let real = ReactorSpert3::e_core_cold_startup();
        let (_, mut state) = start(78.0);
        let mut frame = blank_frame();
        let rho = REACTIVITY_MAX_DOLLAR;
        let end = loop {
            if let Err(end) = validate(&real, &state, load(rho), ceiling()) {
                break end;
            }
            let advanced = rk4(&dropped, rho, &state_vector(&state), ceiling());
            match commit(&dropped, &mut state, &mut frame, load(rho), ceiling(), advanced) {
                Ok(()) => assert!(!state.past_first_maximum),
                Err(end) => break end,
            }
        };
        // The first step of the altered map lands a state whose net reactivity holds no
        // feedback. The real card's checks recompute it from the energy and name the state.
        assert_eq!(end, StepReject::InconsistentState, "{end:?}");
        assert!(!state.past_first_maximum);
        assert_eq!(state.t_s, ceiling());
        // Left to itself, the altered map runs to the edge of the card.
        let end = loop {
            let advanced = rk4(&dropped, rho, &state_vector(&state), ceiling());
            match commit(&dropped, &mut state, &mut frame, load(rho), ceiling(), advanced) {
                Ok(()) => assert!(!state.past_first_maximum),
                Err(end) => break end,
            }
        };
        assert!(
            end == StepReject::PowerBeyondCard || end == StepReject::EnergyBeyondCard,
            "{end:?}"
        );
        assert!(state.power_mw > 100.0);
        // The real card, with its feedback, peaks at 296 MW at this load instead.
        let run = run_to_first_maximum(78.0, rho, ceiling());
        assert_eq!(run.end, StepReject::PastFirstMaximum);
        assert!(run.last.peak_power_mw < 400.0);
    }

    #[test]
    fn the_step_ceiling_and_floor_are_present_and_derived() {
        let reactor = ReactorSpert3::e_core_cold_startup();
        let ceiling_s = reactor.gate_dt_s().unwrap();
        let loss = 1.0 - REACTIVITY_MIN_DOLLAR + FEEDBACK_STRENGTH_DOLLAR_PER_MWS * ENERGY_MAX_MWS;
        assert_eq!(
            ceiling_s.to_bits(),
            (GATE_PHASE_STEP * GENERATION_TIME_REDUCED_S / loss).to_bits()
        );
        assert!(ceiling_s > 7.1e-4 && ceiling_s < 7.2e-4, "{ceiling_s}");
        let floor_s = reactor.floor_dt_s().unwrap();
        assert_eq!(floor_s.to_bits(), 1.0e-9_f64.to_bits());
        assert!(floor_s < ceiling_s);
        // 0.2 of the fastest pole's time constant: the fastest pole of the card
        // is the prompt pole (1 - rho + a E) / l* at the smallest load and the
        // largest energy, 280 per second.
        let fastest_per_s = loss / GENERATION_TIME_REDUCED_S;
        assert!(fastest_per_s > 279.0 && fastest_per_s < 281.0);
        assert!(within(ceiling_s * fastest_per_s, 0.2, 1.0e-12));
        // A broken card has none.
        let mut broken = reactor;
        broken.generation_time_reduced_s = 0.0;
        assert!(broken.gate_dt_s().is_none());
        broken = reactor;
        broken.floor_dt_s = 0.0;
        assert!(broken.floor_dt_s().is_none());
        broken = reactor;
        broken.reactivity_min_dollar = 100.0;
        broken.feedback_strength_dollar_per_mws = 0.0;
        assert!(broken.gate_dt_s().is_none());

        // The floor still moves the clock at the end of the card, and a step at
        // the floor from the start moves the power, the energy, the clock and
        // the closed-form integral of the ledger.
        let at_the_end_s = TIME_MAX_S - floor_s;
        assert!(at_the_end_s + floor_s != at_the_end_s);
        let (reactor, fresh) = start(70.0);
        let mut state = fresh;
        let mut frame = blank_frame();
        step(&reactor, &mut state, &mut frame, load(1.0), floor_s).unwrap();
        assert!(state.t_s == floor_s);
        assert!(state.power_mw > fresh.power_mw);
        assert!(state.energy_mws > fresh.energy_mws);
        assert!(state.population_anchor_mw != 0.0);
        assert!(state.reactivity_energy_integral_dollar_mws != 0.0);
        proof_intact(&frame);
        // And the real step at the ceiling stands: no refusal on the whole span.
        let mut state = fresh;
        step(&reactor, &mut state, &mut frame, load(REACTIVITY_MAX_DOLLAR), ceiling_s).unwrap();
        let mut state = fresh;
        step(&reactor, &mut state, &mut frame, load(REACTIVITY_MIN_DOLLAR), ceiling_s).unwrap();
    }

    /// `ρ(ω) = ω l* + Σ b_i ω / (ω + λ_i)`, the inhour equation of the card's own set.
    fn inhour_dollar(omega_per_s: f64) -> f64 {
        let mut total = omega_per_s * GENERATION_TIME_REDUCED_S;
        let mut index = 0;
        while index < 6 {
            total += GROUP_FRACTION[index] * omega_per_s / (omega_per_s + GROUP_DECAY_PER_S[index]);
            index += 1;
        }
        total
    }

    /// The two ends of the span, run at the ceiling and at half of it, reach the first maximum
    /// inside the card: the slowest in 17,098 steps at the ceiling, 7.24 MW s at 12.23 s, the
    /// fastest in 214 steps, about 295.6 MW.
    #[test]
    fn the_ends_of_the_span_reach_the_first_maximum_inside_the_card() {
        let slow = run_to_first_maximum(74.0, REACTIVITY_MIN_DOLLAR, ceiling());
        let fast = run_to_first_maximum(78.0, REACTIVITY_MAX_DOLLAR, ceiling());
        println!("slowest {} steps, {} MW s at {} s", slow.steps, slow.last.peak_energy_mws, slow.last.peak_time_s);
        println!("fastest {} steps, {} MW", fast.steps, fast.last.peak_power_mw);
        assert_eq!(slow.end, StepReject::PastFirstMaximum);
        assert_eq!(fast.end, StepReject::PastFirstMaximum);
        assert_eq!(slow.steps, 17_098);
        assert_eq!(fast.steps, 214);
        assert!((slow.last.peak_energy_mws - 7.2358).abs() < 1.0e-3);
        assert!((slow.last.peak_time_s - 12.228).abs() < 1.0e-3);
        assert!((fast.last.peak_power_mw - 295.570).abs() < 1.0e-2);
        for dt in [ceiling(), ceiling() / 2.0] {
            for (fahrenheit, rho) in [(74.0, REACTIVITY_MIN_DOLLAR), (78.0, REACTIVITY_MAX_DOLLAR)] {
                let run = run_to_first_maximum(fahrenheit, rho, dt);
                assert_eq!(run.end, StepReject::PastFirstMaximum, "rho {rho} dt {dt}");
                assert!(run.last.t_s < TIME_MAX_S);
                assert!(run.last.energy_mws < ENERGY_MAX_MWS);
                assert!(run.last.peak_power_mw < POWER_MAX_MW);
            }
        }
    }

    /// The ends of the span are the inhour values of the longest and the shortest tested
    /// reactor period, 1010 ms (test 22) and 10 ms (test 43), for the card's own set, computed
    /// again here; the research script sums the groups with numpy, so the last bit may differ.
    #[test]
    fn the_reactivity_span_is_the_inhour_value_of_the_tested_periods() {
        let longest = inhour_dollar(1.0 / 1.010);
        let shortest = inhour_dollar(1.0 / 0.010);
        println!("inhour 1010 ms {longest:?}, 10 ms {shortest:?}");
        assert!((longest - REACTIVITY_MIN_DOLLAR).abs() <= 4.0e-16, "{longest}");
        assert!((shortest - REACTIVITY_MAX_DOLLAR).abs() <= 4.0e-16, "{shortest}");
        // Every opened test lies inside the span.
        for (test, _, rho) in OPENED {
            assert!((REACTIVITY_MIN_DOLLAR..=REACTIVITY_MAX_DOLLAR).contains(&rho), "test {test}");
        }
    }

    #[test]
    fn without_feedback_the_asymptotic_period_matches_the_inhour_equation() {
        let reactor = without_feedback();
        let (_, fresh) = start(70.0);
        // load, end of the run, start of the window over which the growth rate is read
        for (rho, end_s, window_s) in [(0.77, 30.0, 20.0), (1.0, 8.0, 4.0), (1.21, 0.25, 0.12)] {
            let dt = ceiling();
            let mut y = state_vector(&fresh);
            let mut t = 0.0;
            let mut at_window = (0.0, 0.0);
            let mut set = false;
            while t < end_s {
                y = rk4(&reactor, rho, &y, dt);
                t += dt;
                if !set && t >= window_s {
                    at_window = (t, y[0]);
                    set = true;
                }
            }
            let omega = (y[0] / at_window.1).ln() / (t - at_window.0);
            let rho_back = inhour_dollar(omega);
            println!("rho {rho}: growth rate {omega} per s, period {} ms, inhour back {rho_back}", 1000.0 / omega);
            assert!(
                within(rho_back, rho, 1.0e-6),
                "load {rho}: inhour of the measured rate is {rho_back}"
            );
        }
    }

    #[test]
    fn the_ledger_residual_stays_under_the_limit_over_a_full_run_at_the_ceiling() {
        let mut worst = 0.0_f64;
        let mut worst_test = 0;
        for (test, fahrenheit, rho) in OPENED {
            let run = run_to_first_maximum(fahrenheit, rho, ceiling());
            assert_eq!(run.end, StepReject::PastFirstMaximum, "test {test}");
            assert!(run.always_rising, "test {test}");
            assert!(run.worst_residual < RESIDUAL_CONSERVATION_LIMIT);
            if run.worst_residual > worst {
                worst = run.worst_residual;
                worst_test = test;
            }
        }
        println!("largest ledger residual at the ceiling: {worst:e} (test {worst_test})");
        assert_eq!(worst_test, 43);
        assert!(worst > 4.0e-9 && worst < 7.0e-9, "{worst:e}");
        // The limit is more than ten times the largest value found.
        assert!(RESIDUAL_CONSERVATION_LIMIT > 10.0 * worst);
        // A step 2.6 times the ceiling on the shortest period is refused by the ledger.
        let run = run_to_first_maximum(78.0, 1.2102315121058187, 1.87e-3);
        assert!(
            run.end == StepReject::FirstLawBreach || run.end == StepReject::SpontaneousGeneration,
            "{:?}",
            run.end
        );
    }

    #[test]
    fn halving_the_step_changes_the_peak_by_the_fourth_order_ratio() {
        let reactor = ReactorSpert3::e_core_cold_startup();
        let rho = 1.2102315121058187;
        // The state at 0.15 s, 2 ms before the peak of the shortest period, at
        // 150, 300, 600 and 1200 steps: halving the step.
        let mut power = [0.0_f64; 4];
        let mut energy = [0.0_f64; 4];
        let mut index = 0;
        while index < 4 {
            let steps = 150u32 << index;
            let dt = 0.15 / (steps as f64);
            let mut y = state_vector(&ReactorState::try_cold_start(&reactor, kelvin(78.0)).unwrap());
            let mut count = 0;
            while count < steps {
                y = rk4(&reactor, rho, &y, dt);
                count += 1;
            }
            power[index] = y[0];
            energy[index] = y[7];
            index += 1;
        }
        let power_first = (power[0] - power[1]).abs();
        let power_second = (power[1] - power[2]).abs();
        let power_third = (power[2] - power[3]).abs();
        let energy_first = (energy[0] - energy[1]).abs();
        let energy_second = (energy[1] - energy[2]).abs();
        println!(
            "power at 0.15 s changes on halving {power_first:e} {power_second:e} {power_third:e}; ratios {} {}",
            power_first / power_second,
            power_second / power_third
        );
        println!("energy ratios {}", energy_first / energy_second);
        assert!(within(power_first / power_second, 16.0, 0.1), "{}", power_first / power_second);
        assert!(within(power_second / power_third, 16.0, 0.1), "{}", power_second / power_third);
        assert!(within(energy_first / energy_second, 16.0, 0.1), "{}", energy_first / energy_second);

        // The peak power as the caller reads it, by cubic Hermite through the
        // last two landed samples, at 1.5, 0.75, 0.375 and 0.1875 of the
        // ceiling: the change falls by better than a factor of 6 at each
        // halving. Which pair of samples holds the peak changes with the step,
        // so the ratio is not 16 to the digit here.
        let mut peaks = [0.0_f64; 4];
        index = 0;
        while index < 4 {
            let dt = ceiling() * 1.5 / ((1u32 << index) as f64);
            let run = run_to_first_maximum(78.0, rho, dt);
            assert_eq!(run.end, StepReject::PastFirstMaximum);
            peaks[index] = hermite_peak(&run.previous, &run.last).0;
            index += 1;
        }
        let first = (peaks[0] - peaks[1]).abs();
        let second = (peaks[1] - peaks[2]).abs();
        let third = (peaks[2] - peaks[3]).abs();
        println!("peak power changes on halving: {first:e} {second:e} {third:e}");
        assert!(first / second > 6.0, "{}", first / second);
        assert!(second / third > 6.0, "{}", second / third);
    }

    #[test]
    fn the_card_at_its_ceiling_agrees_with_the_forecast_of_structure_b() {
        let mut worst = [0.0_f64; 3];
        for (test, forecast_power, forecast_energy, forecast_time, measured_power, measured_energy, measured_time) in FORECAST {
            let (_, fahrenheit, rho) = OPENED.iter().copied().find(|row| row.0 == test).unwrap();
            let run = run_to_first_maximum(fahrenheit, rho, ceiling());
            assert_eq!(run.end, StepReject::PastFirstMaximum, "test {test}");
            let (power, energy, time) = hermite_peak(&run.previous, &run.last);
            let errors = [
                (power - forecast_power).abs() / forecast_power,
                (energy - forecast_energy).abs() / forecast_energy,
                (time - forecast_time).abs() / forecast_time,
            ];
            println!(
                "test {test}: card P {power:.7} E {energy:.7} t {time:.7} | forecast P {forecast_power:.7} E {forecast_energy:.7} t {forecast_time:.7} | measured P {measured_power} E {measured_energy} t {measured_time} | rel err {:.2e} {:.2e} {:.2e} | sample P {:.7} E {:.7} t {:.7}",
                errors[0], errors[1], errors[2], run.last.peak_power_mw, run.last.peak_energy_mws, run.last.peak_time_s
            );
            let mut k = 0;
            while k < 3 {
                if errors[k] > worst[k] {
                    worst[k] = errors[k];
                }
                k += 1;
            }
            // Interpolated by the caller. Largest found: power 2.4e-8, energy
            // 1.4e-6 (the 10 ms period), time 3.5e-7.
            assert!(errors[0] < 1.0e-7, "test {test} power {}", errors[0]);
            assert!(errors[1] < 3.0e-6, "test {test} energy {}", errors[1]);
            assert!(errors[2] < 1.0e-6, "test {test} time {}", errors[2]);
            // The card's own sample, with no interpolation: the time within one
            // step, the power within the second-order sampling error, the energy
            // within the power times half a step.
            let step_s = ceiling();
            assert!((run.last.peak_time_s - forecast_time).abs() <= step_s);
            assert!(run.last.peak_power_mw <= forecast_power * (1.0 + 2.0e-6));
            assert!(run.last.peak_power_mw >= forecast_power * (1.0 - 5.0e-4));
            assert!(
                (run.last.peak_energy_mws - forecast_energy).abs()
                    <= forecast_power * step_s * 0.5 * (1.0 + 1.0e-3) + 2.0e-6 * forecast_energy
            );
        }
        println!("worst relative differences {:e} {:e} {:e}", worst[0], worst[1], worst[2]);
    }
}
