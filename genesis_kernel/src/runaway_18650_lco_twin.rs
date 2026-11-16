#![no_implicit_prelude]

//! Thermal runaway of one 18650 lithium-ion cell, LiCoO2 cathode and graphite
//! anode, held in an oven, heated, or tracked in an accelerating-rate
//! calorimeter, as one lumped body: the twin of `runaway_18650_lco`, fitted
//! on the open records of LiCoO2 18650 cells.
//!
//! The model is `runaway_18650_lco`'s, the lumped abuse model of Hatchard,
//! MacNeil, Basu and Dahn (2001), J. Electrochem. Soc. 148, A755, with the
//! four decomposition stages of Kim, Pesaran and Spotnitz (2007), J. Power
//! Sources 170, 476, every published constant as that card takes it from
//! open reprints (Ostanek 2025, Purdue University Research Repository, doi
//! 10.4231/NWZX-2675, Table 2; Lopez, Jeevarajan and Mukherjee 2015, J.
//! Electrochem. Soc. 162, A2163; Coman et al. 2017, J. Electrochem. Soc. 164,
//! A587), and two stages more, both taking heat:
//!
//! - the separator melts at 130 degC, the temperature Golubkov et al. (2014),
//!   RSC Adv. 4, 3633, give for their cells' separator;
//! - the vent opens at 160 degC, where Golubkov et al. (2014) found the safety
//!   vents of their cells usually opened; the heat its gas carries away is
//!   fitted, and fits to zero, so the stage marks the vent and takes no heat.
//!
//! Seven constants are fitted on 13 open records of LiCoO2 18650 cells (the
//! three oven tests of Hatchard et al. 2001, Golubkov et al. 2014's LCO/NMC
//! cell, Mendoza-Hernandez et al. 2017, and eight cells Lei et al. 2017
//! reprint): the SEI stage's frequency factor, the SEI's thickness in a
//! fresh cell and the thickness that slows the anode stage by e, the cathode
//! stage's frequency factor, the convective coefficient to the oven, and the
//! heats of the melt and of the vent. The fit and each record left out of it in turn are in
//! `evidence/research/thermal_runaway_18650/twin/`.
//!
//! The six stages. Each is a dimensionless amount that only falls; stage `i`
//! releases `H_i` per unit of its amount, in joules (negative for the two
//! that take heat):
//!
//! ```text
//!   SEI decomposition        R_sei = A_sei exp(−E_sei / R T) c_sei
//!   anode with electrolyte   R_ne  = A_ne  exp(−E_ne  / R T) exp(−z / z_ref) c_ne
//!   cathode with electrolyte R_pe  = A_pe  exp(−E_pe  / R T) α (1 − α)
//!   electrolyte              R_e   = A_e   exp(−E_e   / R T) c_e
//!   separator melt           R_m   = k(T; T_m) s
//!   vent                     R_v   = k(T; T_v) g
//!
//!   k(T; T_x) = exp(−(T_x / w)(T_x / T − 1)) / τ:  1/τ at T_x, rising e-fold per w kelvin there
//!
//!   q   = Σ H_i R_i                                (net, W)
//!   L   = h A (T − T_oven) + ε σ A (T⁴ − T_oven⁴)  (zero when adiabatic)
//!   C dT/dt = q + P − L,   C = ρ c_p V
//! ```
//!
//! `c_sei` is the lithium in the solid-electrolyte interphase, `c_ne` the
//! lithium in the graphite, `z` the thickness of the interphase, which the
//! anode reaction grows and which slows it, `α` the converted part of the
//! cathode, `c_e` the electrolyte, `s` the separator not yet melted and `g`
//! the gas the vent lets out still held in the cell. `z + c_ne` never
//! changes; a fresh twin starts at `z` = [`SEI_THICKNESS_START`] and `c_ne` =
//! 0.75, so `z` = `z_0` + 0.75 − `c_ne` always and is not a separate state.
//! The card carries `1 − α`, the unreacted part of the cathode, so that its
//! last part is not lost to rounding. `P` is the heater's power and `T_oven`
//! the oven's temperature, the load.
//!
//! The bounds of the six amounts are where their equations hold: each is a
//! part of what the cell holds, from none to all of it, and the lithium in
//! the graphite at most `z_0` + 0.75, where the SEI thickness is zero. The
//! unreacted cathode is 1 or at most 1 − 2^-33 ([`CATHODE_CONVERTED_LEAST`]),
//! as on `runaway_18650_lco`. Below zero or past its limit an amount is a
//! named refusal. The temperature must be above zero kelvin and below
//! 1811 K, the melting point of iron, where the steel can melts.
//!
//! The onset is reported, not refused: [`FLAG_SELF_HEATING`] is set on a
//! frame whose net stage heat warms the cell at 0.02 K per minute or more,
//! [`FLAG_RUNAWAY`] at 10 K per minute or more, [`FLAG_SEPARATOR_MELTED`]
//! once half the separator has melted and [`FLAG_VENTED`] once half the
//! vent's gas has left. [`FLAG_OUTSIDE_COMPARED`] marks a step whose load is
//! outside those the records were taken under. The runaway's end is
//! [`StepReject::EveryReactantConsumed`]: the heat the six stages still hold
//! or take, in size, falls to [`CONSUMED_HEAT_FRACTION`] of the cell's
//! chemical heat or under, tested at the end of every substep.
//!
//! The step. Each step is divided into substeps of classical fourth-order
//! Runge–Kutta over the temperature, the natural logarithm of each amount and
//! the heat lost, each no longer than [`SUBSTEP_LOG_CHANGE`] over the fastest
//! live stage's rate in its logarithm, nor than a change of
//! [`SUBSTEP_TEMPERATURE_CHANGE_K`] at the substep's starting rate; a stage
//! whose remaining heat is at or under [`SPENT_HEAT_FRACTION`] of the cell's
//! chemical heat finishes at the start of a substep, its last heat booked
//! and put into the temperature; a stage that carries no heat does not set
//! the substep. A step at or under the ceiling whose ledger does not close
//! with those substeps is taken again with both bounds halved, and again, up
//! to [`FINEST_SUBSTEPS`] times shorter, until it closes (since engine
//! 0.15.2: in an oven above about 440 K at 10 s the separator's melt rate,
//! steep in temperature, ran away from the rule inside a substep sized at its
//! start, and 0.15.1 refused the step `FirstLawBreach`); a step that lands
//! with the first substeps lands as it always did. A step that would need
//! more than [`MAX_SUBSTEPS`] is [`StepReject::SubstepsAboveBudget`]. The step
//! ceiling is a hundredth of the convective time constant `C / (h A)`.
//!
//! The ledger. Each substep books the heat each stage released in closed
//! form from its amount, `H_i (c_before − c_after)`, the heater's heat and the
//! heat lost, integrated with the temperature; the heat stored is
//! `C (T − T_start)`, and
//!
//! ```text
//!   defect = C (T − T_start) − (Σ released + heater − lost)
//! ```
//!
//! is held to [`RESIDUAL_CONSERVATION_LIMIT`] of the cell's whole chemical
//! heat, 20,499 J: a step that skips the substeps, or a temperature, an
//! amount or a heat booked otherwise, is named
//! [`StepReject::SpontaneousGeneration`] or [`StepReject::FirstLawBreach`].
//!
//! What is left out: the temperature inside the cell, the mass the vent lets
//! out and the heat capacity it takes with it, the internal short circuit and
//! the electrical energy of the charge, combustion of the vented gas, the
//! aluminium collector's melting, a heat capacity that changes with
//! temperature, and the calorimeter's own heat capacity.
//!
//! A step writes the time, the temperature, the net stage heat, the heat
//! released, three of the amounts, the heat lost and the residual into a
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Number of stages.
pub const STAGES: usize = 6;

/// Gas constant, J/(mol K), as the Hatchard–Kim model is stated with it.
pub const GAS_CONSTANT_J_PER_MOL_K: f64 = 8.314;
/// Stefan–Boltzmann constant, W/(m² K⁴), CODATA 2018.
pub const STEFAN_BOLTZMANN_W_PER_M2_K4: f64 = 5.670374419e-8;

/// SEI decomposition: frequency factor, 1/s. Fitted on the 13 records
/// (1.667e15 as Ostanek 2025, Table 2, prints it): the SEI of a commercial
/// cell decomposes 488 times slower than the graphite material's.
pub const SEI_FREQUENCY_FACTOR_PER_S: f64 = 3.4168e12;
/// SEI decomposition: activation energy, J/mol. Ostanek (2025), Table 2.
pub const SEI_ACTIVATION_J_PER_MOL: f64 = 1.3508e5;
/// SEI decomposition: heat released, J per kg of carbon. Ostanek (2025),
/// Table 2.
pub const SEI_HEAT_J_PER_KG: f64 = 2.57e5;
/// Anode with electrolyte: frequency factor, 1/s. Ostanek (2025), Table 2.
pub const ANODE_FREQUENCY_FACTOR_PER_S: f64 = 2.5e13;
/// Anode with electrolyte: activation energy, J/mol. Ostanek (2025),
/// Table 2.
pub const ANODE_ACTIVATION_J_PER_MOL: f64 = 1.3508e5;
/// Anode with electrolyte: heat released, J per kg of carbon. Ostanek
/// (2025), Table 2.
pub const ANODE_HEAT_J_PER_KG: f64 = 1.714e6;
/// Cathode with electrolyte: frequency factor, 1/s. Fitted on the 13
/// records (6.667e13 as Ostanek 2025, Table 2, prints it; Ostanek et al.
/// 2023 divide it by 1.2 for Hatchard's oven, this fit by 1.168).
pub const CATHODE_FREQUENCY_FACTOR_PER_S: f64 = 5.7072e13;
/// Cathode with electrolyte: activation energy, J/mol. Ostanek (2025),
/// Table 2.
pub const CATHODE_ACTIVATION_J_PER_MOL: f64 = 1.396e5;
/// Cathode with electrolyte: heat released, J per kg of cathode active
/// material. Ostanek (2025), Table 2.
pub const CATHODE_HEAT_J_PER_KG: f64 = 3.14e5;
/// Electrolyte decomposition: frequency factor, 1/s. Ostanek (2025),
/// Table 2.
pub const ELECTROLYTE_FREQUENCY_FACTOR_PER_S: f64 = 5.14e25;
/// Electrolyte decomposition: activation energy, J/mol. Ostanek (2025),
/// Table 2.
pub const ELECTROLYTE_ACTIVATION_J_PER_MOL: f64 = 2.74e5;
/// Electrolyte decomposition: heat released, J per kg of electrolyte.
/// Ostanek (2025), Table 2.
pub const ELECTROLYTE_HEAT_J_PER_KG: f64 = 1.55e5;

/// Carbon in the cell, kg per m³ of cell. Ostanek (2025), Table 2.
pub const CARBON_CONTENT_KG_PER_M3: f64 = 6.104e2;
/// Cathode active material in the cell, kg per m³ of cell. Ostanek (2025),
/// Table 2.
pub const CATHODE_CONTENT_KG_PER_M3: f64 = 1.221e3;
/// Electrolyte in the cell, kg per m³ of cell. Ostanek (2025), Table 2.
pub const ELECTROLYTE_CONTENT_KG_PER_M3: f64 = 406.9;

/// Lithium in the SEI at the start of a fresh cell. Ostanek (2025), Table 2.
pub const SEI_FRACTION_START: f64 = 0.15;
/// Lithium in the graphite at the start of a fresh cell. Ostanek (2025),
/// Table 2.
pub const ANODE_LITHIUM_START: f64 = 0.75;
/// Dimensionless SEI thickness of a fresh cell. Fitted on the 13 records
/// (0.033 as Ostanek 2025, Table 2, prints it).
pub const SEI_THICKNESS_START: f64 = 0.17788;
/// The SEI thickness over which the anode stage slows by e. Fitted on the
/// 13 records (0.033 as Ostanek 2025, Table 2, prints it).
pub const SEI_THICKNESS_REFERENCE: f64 = 0.039853;
/// Converted part of the cathode at the start of a fresh cell. Ostanek
/// (2025), Table 2.
pub const CATHODE_CONVERSION_START: f64 = 0.04;
/// Electrolyte at the start of a fresh cell. Ostanek (2025), Table 2.
pub const ELECTROLYTE_START: f64 = 1.0;
/// Separator not yet melted, at the start of a fresh cell: all of it.
pub const SEPARATOR_START: f64 = 1.0;
/// The vent's gas still held, at the start of a fresh cell: all of it.
pub const VENT_GAS_START: f64 = 1.0;

/// The separator melts here, K: 130 degC, Golubkov et al. (2014).
pub const SEPARATOR_MELT_K: f64 = 403.15;
/// The vent opens here, K: 160 degC, Golubkov et al. (2014).
pub const VENT_OPENING_K: f64 = 433.15;
/// The kelvin over which the melt's and the vent's rate rises e-fold at
/// their temperature.
pub const PHASE_WIDTH_K: f64 = 1.0;
/// The time constant of the melt and of the vent at their temperature, s.
pub const PHASE_TIME_S: f64 = 60.0;
/// Heat the separator takes as it melts, J. Fitted on the 13 records.
pub const SEPARATOR_MELT_HEAT_J: f64 = 145.63;
/// Heat the gas the vent lets out carries away, J. Fitted on the 13
/// records, at its lower bound: the records hold no cooling at the vent, and
/// the stage marks the vent's time (FLAG_VENTED) and takes no heat.
pub const VENT_HEAT_J: f64 = 0.0;

/// Cell radius, m: an 18650 is 18 mm across.
pub const CELL_RADIUS_M: f64 = 0.009;
/// Cell height, m: an 18650 is 65 mm high.
pub const CELL_HEIGHT_M: f64 = 0.065;
/// Density of the cell as one body, kg/m³: the jelly roll's, Hatchard et al.
/// (2001), as Coman et al. (2017), Table I, reprint it.
pub const CELL_DENSITY_KG_PER_M3: f64 = 2580.0;
/// Specific heat of the cell as one body, J/(kg K): the jelly roll's,
/// Hatchard et al. (2001), as Coman et al. (2017), Table I, reprint it.
pub const CELL_SPECIFIC_HEAT_J_PER_KG_K: f64 = 830.0;
/// Convective coefficient from the can to the oven, W/(m² K). Fitted on the
/// 13 records (7.17 as Lopez et al. 2015 and Ostanek 2025, Table 1, state
/// Hatchard's).
pub const CONVECTION_W_PER_M2_K: f64 = 7.8848;
/// Emissivity of the can: Ostanek (2025), Table 1, and Coman et al. (2017),
/// Table I.
pub const CAN_EMISSIVITY: f64 = 0.8;
/// Melting point of iron, K: the steel can melts there.
pub const CAN_MELTING_K: f64 = 1811.0;

/// `π r² H`, m³.
pub const CELL_VOLUME_M3: f64 = core::f64::consts::PI * CELL_RADIUS_M * CELL_RADIUS_M * CELL_HEIGHT_M;
/// `2 π r H + 2 π r²`, m², the can's whole outer surface.
pub const CELL_SURFACE_M2: f64 = 2.0 * core::f64::consts::PI * CELL_RADIUS_M * CELL_HEIGHT_M
    + 2.0 * core::f64::consts::PI * CELL_RADIUS_M * CELL_RADIUS_M;
/// `ρ c_p V`, J/K: 35.42 J/K.
pub const CELL_HEAT_CAPACITY_J_PER_K: f64 = CELL_DENSITY_KG_PER_M3 * CELL_SPECIFIC_HEAT_J_PER_KG_K * CELL_VOLUME_M3;
/// `z + c_ne`, which no reaction changes.
pub const SEI_PLUS_ANODE_LITHIUM: f64 = SEI_THICKNESS_START + ANODE_LITHIUM_START;
/// The heat each stage releases per unit of its amount, J, in stage order:
/// `H W V` for the four decomposition stages, and the melt's and the vent's
/// heat taken, negative.
pub const STAGE_HEAT_J: [f64; STAGES] = [
    SEI_HEAT_J_PER_KG * CARBON_CONTENT_KG_PER_M3 * CELL_VOLUME_M3,
    ANODE_HEAT_J_PER_KG * CARBON_CONTENT_KG_PER_M3 * CELL_VOLUME_M3,
    CATHODE_HEAT_J_PER_KG * CATHODE_CONTENT_KG_PER_M3 * CELL_VOLUME_M3,
    ELECTROLYTE_HEAT_J_PER_KG * ELECTROLYTE_CONTENT_KG_PER_M3 * CELL_VOLUME_M3,
    -SEPARATOR_MELT_HEAT_J,
    -VENT_HEAT_J,
];
/// The cell's whole chemical heat from a fresh start, J: the four
/// decomposition stages' heat at a fresh cell's amounts, 20,499 J. The
/// ledger's scale.
pub const CELL_CHEMICAL_HEAT_J: f64 = STAGE_HEAT_J[0] * SEI_FRACTION_START
    + STAGE_HEAT_J[1] * ANODE_LITHIUM_START
    + STAGE_HEAT_J[2] * (1.0 - CATHODE_CONVERSION_START)
    + STAGE_HEAT_J[3] * ELECTROLYTE_START;

/// A heating rate of 0.02 K per minute, in K/s: the detection threshold of
/// an accelerating-rate calorimeter.
pub const SELF_HEATING_RATE_K_PER_S: f64 = 0.02 / 60.0;
/// A heating rate of 10 K per minute, in K/s: the onset of thermal runaway
/// as Golubkov et al. (2014) read it.
pub const RUNAWAY_RATE_K_PER_S: f64 = 10.0 / 60.0;

/// The ovens the records held: 145 to 155 degC, K.
pub const COMPARED_OVEN_K: [f64; 2] = [418.15, 428.15];
/// The most heater power the records used, W: the 2 K per minute of the
/// calorimeters, 1.1807 W, rounded up.
pub const COMPARED_HEATER_W: f64 = 1.2;

/// Largest change of the logarithm of a stage's amount, times its stiffness,
/// in one substep.
pub const SUBSTEP_LOG_CHANGE: f64 = 0.1;
/// Largest change of temperature in one substep at the substep's starting
/// rate, K.
pub const SUBSTEP_TEMPERATURE_CHANGE_K: f64 = 1.0;
/// A stage whose remaining heat is at or under this part of
/// [`CELL_CHEMICAL_HEAT_J`] finishes at the start of a substep, its last heat
/// booked: 2^-52.
pub const SPENT_HEAT_FRACTION: f64 = 2.220446049250313e-16;
/// Most substeps in one step: 2^20.
pub const MAX_SUBSTEPS: u32 = 1_048_576;
/// The most a step's substeps are shortened when its ledger does not close:
/// [`SUBSTEP_LOG_CHANGE`] and [`SUBSTEP_TEMPERATURE_CHANGE_K`] over at most
/// 2^20 (since engine 0.15.2).
pub const FINEST_SUBSTEPS: f64 = 1_048_576.0;
/// Step ceiling over the convective time constant `C / (h A)`.
pub const GATE_OVER_CONVECTIVE_TIME: f64 = 0.01;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;
/// `|defect|` over [`CELL_CHEMICAL_HEAT_J`] must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Every reactant is consumed when the heat the six stages still hold or
/// take, in size, is at or under this part of [`CELL_CHEMICAL_HEAT_J`]:
/// 1e-6, 0.0205 J, the ledger's own limit.
pub const CONSUMED_HEAT_FRACTION: f64 = 1.0e-6;
/// Where the equations of the six amounts hold, in stage order: each at
/// most 1, all of it, but the lithium in the graphite, at most
/// [`SEI_PLUS_ANODE_LITHIUM`], where the SEI thickness is zero. Each at
/// least zero.
pub const AMOUNT_LIMITS: [f64; STAGES] = [1.0, SEI_PLUS_ANODE_LITHIUM, 1.0, 1.0, 1.0, 1.0];
/// The least converted part of the cathode, `α`, other than zero, that the
/// card carries: 2^-33, 1.16e-10, as on `runaway_18650_lco`.
pub const CATHODE_CONVERTED_LEAST: f64 = 1.1641532182693481e-10;

/// The net stage heat at the end of the step warms the cell at
/// [`SELF_HEATING_RATE_K_PER_S`] or more: self-heating an accelerating-rate
/// calorimeter detects.
pub const FLAG_SELF_HEATING: u64 = 1 << 0;
/// The net stage heat at the end of the step warms the cell at
/// [`RUNAWAY_RATE_K_PER_S`] or more: thermal runaway.
pub const FLAG_RUNAWAY: u64 = 1 << 1;
/// Half the separator or more has melted.
pub const FLAG_SEPARATOR_MELTED: u64 = 1 << 2;
/// Half the vent's gas or more has left the cell.
pub const FLAG_VENTED: u64 = 1 << 3;
/// The step's load is outside those the records were taken under: an oven
/// outside 145 to 155 degC, a heater above 1.2 W, or a heater in an oven.
pub const FLAG_OUTSIDE_COMPARED: u64 = 1 << 4;

/// Why a step did not advance the cell or the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepReject {
    /// The oven temperature, the heater power, the adiabatic switch or `dt`
    /// was NaN or infinite.
    NonFiniteInput,
    /// The incoming state, or a substep, was not finite.
    NonFiniteState,
    /// `dt` was zero or negative.
    NonPositiveDt,
    /// A recorded constant is not this cell's.
    InvalidCell,
    /// The oven temperature is zero kelvin or below while the cell loses heat
    /// to the oven, adiabatic 0; with adiabatic 1 it does not enter and is not
    /// tested.
    OvenTemperatureNotPositive,
    /// The heater power is below zero: a heater gives heat and does not take
    /// it.
    HeaterPowerNegative,
    /// The adiabatic switch is neither 0 nor 1.
    AdiabaticNotZeroOrOne,
    /// The cell temperature, incoming or in a substep, is zero kelvin or
    /// below.
    TemperatureNotPositive,
    /// The cell temperature, incoming or in a substep, is at or above
    /// 1811 K, the melting point of iron: the steel can melts.
    CanMelting,
    /// The lithium in the SEI is below 0 or above 1: a fraction, it is
    /// between none and all.
    SeiFractionOutsideCell,
    /// The lithium in the graphite is below 0 or above the fresh SEI
    /// thickness plus 0.75: above it the SEI thickness would be below zero.
    AnodeLithiumOutsideCell,
    /// The unreacted part of the cathode is below 0 or above 1, or above
    /// 1 - 2^-33 and under 1: above 1 the converted part would be below zero;
    /// just under 1 its growth is lost to rounding.
    CathodeUnreactedOutsideCell,
    /// The electrolyte is below 0 or above 1: a fraction of the cell's fill,
    /// it is between none and all.
    ElectrolyteOutsideCell,
    /// The separator not yet melted is below 0 or above 1: a part of the
    /// separator, it is between none and all.
    SeparatorOutsideCell,
    /// The vent's gas still held is below 0 or above 1: a part of the gas
    /// the vent lets out, it is between none and all.
    VentGasOutsideCell,
    /// The step would carry the cell to every reactant consumed: the heat the
    /// six stages still hold or take falls to 1e-6 of the cell's chemical
    /// heat or under, 0.0205 J, and the runaway has finished.
    EveryReactantConsumed,
    /// The step would need more than 1,048,576 substeps to follow its fastest
    /// stage.
    SubstepsAboveBudget,
    /// The trial step created heat: the heat stored in the cell rose above
    /// the heat released, plus the heater's, less the heat lost.
    SpontaneousGeneration,
    /// The trial step destroyed heat: the heat stored in the cell fell below
    /// the heat released, plus the heater's, less the heat lost.
    FirstLawBreach,
}

/// What acts on the cell over a step: the oven around it and a heater on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunawayLoad {
    /// Temperature of the oven, or of the surroundings, held over the step.
    pub oven_temperature_k: f64,
    /// Heat a heater puts into the cell, held over the step.
    pub heater_power_w: f64,
    /// 1: no heat crosses the can, as in an accelerating-rate calorimeter
    /// tracking the cell; the oven temperature then does not enter and is not
    /// tested. 0: the can loses heat to the oven by convection and radiation.
    pub adiabatic: f64,
}

/// The locked cell. Every field is a recorded copy of a crate constant and
/// none is a caller input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Runaway18650LcoTwin {
    /// Recorded copies of the four frequency factors, in stage order.
    pub frequency_factor_per_s: [f64; 4],
    /// Recorded copies of the four activation energies.
    pub activation_j_per_mol: [f64; 4],
    /// Recorded copies of [`STAGE_HEAT_J`].
    pub stage_heat_j: [f64; STAGES],
    /// Recorded copies of the six amounts of a fresh cell, the unreacted
    /// cathode as `1 − α`.
    pub start_amounts: [f64; STAGES],
    /// Recorded copies of [`SEPARATOR_MELT_K`] and [`VENT_OPENING_K`].
    pub phase_k: [f64; 2],
    /// Recorded copy of [`PHASE_WIDTH_K`].
    pub phase_width_k: f64,
    /// Recorded copy of [`PHASE_TIME_S`].
    pub phase_time_s: f64,
    /// Recorded copy of [`SEI_PLUS_ANODE_LITHIUM`].
    pub sei_plus_anode_lithium: f64,
    /// Recorded copy of [`SEI_THICKNESS_REFERENCE`].
    pub sei_thickness_reference: f64,
    /// Recorded copy of [`CELL_HEAT_CAPACITY_J_PER_K`].
    pub heat_capacity_j_per_k: f64,
    /// `h A`, W/K.
    pub convective_conductance_w_per_k: f64,
    /// `ε σ A`, W/K⁴.
    pub radiative_conductance_w_per_k4: f64,
    /// Recorded copy of [`CAN_MELTING_K`].
    pub can_melting_k: f64,
    /// Recorded copy of [`CELL_CHEMICAL_HEAT_J`].
    pub chemical_heat_j: f64,
    /// Recorded copies of [`COMPARED_OVEN_K`].
    pub compared_oven_k: [f64; 2],
    /// Recorded copy of [`COMPARED_HEATER_W`].
    pub compared_heater_w: f64,
    /// Recorded copy of [`GATE_OVER_CONVECTIVE_TIME`].
    pub gate_over_convective_time: f64,
    /// Recorded copy of [`GATE_OVER_FLOOR`].
    pub gate_over_floor: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`CONSUMED_HEAT_FRACTION`].
    pub consumed_heat_fraction: f64,
    /// Recorded copies of [`AMOUNT_LIMITS`].
    pub amount_limits: [f64; STAGES],
    /// Recorded copy of [`CATHODE_CONVERTED_LEAST`].
    pub cathode_converted_least: f64,
}

const FREQUENCY_FACTORS: [f64; 4] = [
    SEI_FREQUENCY_FACTOR_PER_S,
    ANODE_FREQUENCY_FACTOR_PER_S,
    CATHODE_FREQUENCY_FACTOR_PER_S,
    ELECTROLYTE_FREQUENCY_FACTOR_PER_S,
];
const ACTIVATION_ENERGIES: [f64; 4] = [
    SEI_ACTIVATION_J_PER_MOL,
    ANODE_ACTIVATION_J_PER_MOL,
    CATHODE_ACTIVATION_J_PER_MOL,
    ELECTROLYTE_ACTIVATION_J_PER_MOL,
];
const START_AMOUNTS: [f64; STAGES] = [
    SEI_FRACTION_START,
    ANODE_LITHIUM_START,
    1.0 - CATHODE_CONVERSION_START,
    ELECTROLYTE_START,
    SEPARATOR_START,
    VENT_GAS_START,
];
const PHASE_K: [f64; 2] = [SEPARATOR_MELT_K, VENT_OPENING_K];
const CONVECTIVE_CONDUCTANCE_W_PER_K: f64 = CONVECTION_W_PER_M2_K * CELL_SURFACE_M2;
const RADIATIVE_CONDUCTANCE_W_PER_K4: f64 =
    CAN_EMISSIVITY * STEFAN_BOLTZMANN_W_PER_M2_K4 * CELL_SURFACE_M2;

fn same<const N: usize>(a: &[f64; N], b: &[f64; N]) -> bool {
    let mut index = 0;
    while index < N {
        if a[index].to_bits() != b[index].to_bits() {
            return false;
        }
        index += 1;
    }
    true
}

impl Runaway18650LcoTwin {
    /// Card: the twin of one LiCoO2 18650 cell, fitted on the 13 records.
    /// The constructor takes no arguments.
    #[must_use]
    pub fn fitted() -> Self {
        let cell = Self {
            frequency_factor_per_s: FREQUENCY_FACTORS,
            activation_j_per_mol: ACTIVATION_ENERGIES,
            stage_heat_j: STAGE_HEAT_J,
            start_amounts: START_AMOUNTS,
            phase_k: PHASE_K,
            phase_width_k: PHASE_WIDTH_K,
            phase_time_s: PHASE_TIME_S,
            sei_plus_anode_lithium: SEI_PLUS_ANODE_LITHIUM,
            sei_thickness_reference: SEI_THICKNESS_REFERENCE,
            heat_capacity_j_per_k: CELL_HEAT_CAPACITY_J_PER_K,
            convective_conductance_w_per_k: CONVECTIVE_CONDUCTANCE_W_PER_K,
            radiative_conductance_w_per_k4: RADIATIVE_CONDUCTANCE_W_PER_K4,
            can_melting_k: CAN_MELTING_K,
            chemical_heat_j: CELL_CHEMICAL_HEAT_J,
            compared_oven_k: COMPARED_OVEN_K,
            compared_heater_w: COMPARED_HEATER_W,
            gate_over_convective_time: GATE_OVER_CONVECTIVE_TIME,
            gate_over_floor: GATE_OVER_FLOOR,
            residual_conservation_limit: RESIDUAL_CONSERVATION_LIMIT,
            consumed_heat_fraction: CONSUMED_HEAT_FRACTION,
            amount_limits: AMOUNT_LIMITS,
            cathode_converted_least: CATHODE_CONVERTED_LEAST,
        };
        if cell.is_valid() {
            cell
        } else {
            ::core::panic!("cell failed is_valid");
        }
    }

    /// Every recorded copy equals its crate constant bit for bit.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        same(&self.frequency_factor_per_s, &FREQUENCY_FACTORS)
            && same(&self.activation_j_per_mol, &ACTIVATION_ENERGIES)
            && same(&self.stage_heat_j, &STAGE_HEAT_J)
            && same(&self.start_amounts, &START_AMOUNTS)
            && same(&self.phase_k, &PHASE_K)
            && self.phase_width_k.to_bits() == PHASE_WIDTH_K.to_bits()
            && self.phase_time_s.to_bits() == PHASE_TIME_S.to_bits()
            && self.sei_plus_anode_lithium.to_bits() == SEI_PLUS_ANODE_LITHIUM.to_bits()
            && self.sei_thickness_reference.to_bits() == SEI_THICKNESS_REFERENCE.to_bits()
            && self.heat_capacity_j_per_k.to_bits() == CELL_HEAT_CAPACITY_J_PER_K.to_bits()
            && self.convective_conductance_w_per_k.to_bits() == CONVECTIVE_CONDUCTANCE_W_PER_K.to_bits()
            && self.radiative_conductance_w_per_k4.to_bits() == RADIATIVE_CONDUCTANCE_W_PER_K4.to_bits()
            && self.can_melting_k.to_bits() == CAN_MELTING_K.to_bits()
            && self.chemical_heat_j.to_bits() == CELL_CHEMICAL_HEAT_J.to_bits()
            && same(&self.compared_oven_k, &COMPARED_OVEN_K)
            && self.compared_heater_w.to_bits() == COMPARED_HEATER_W.to_bits()
            && self.gate_over_convective_time.to_bits() == GATE_OVER_CONVECTIVE_TIME.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.consumed_heat_fraction.to_bits() == CONSUMED_HEAT_FRACTION.to_bits()
            && same(&self.amount_limits, &AMOUNT_LIMITS)
            && self.cathode_converted_least.to_bits() == CATHODE_CONVERTED_LEAST.to_bits()
    }

    /// `C / (h A)`, s: the time in which convection alone would bring the
    /// cell 1 − 1/e of the way to the oven's temperature.
    #[must_use]
    pub fn convective_time_s(&self) -> f64 {
        self.heat_capacity_j_per_k / self.convective_conductance_w_per_k
    }

    /// Largest step the card stands behind: a hundredth of
    /// [`Self::convective_time_s`]. Not applied by [`step`].
    #[must_use]
    pub fn gate_dt_s(&self) -> Option<f64> {
        let gate = self.gate_over_convective_time * self.convective_time_s();
        if gate.is_finite() && gate > 0.0 { Some(gate) } else { None }
    }

    /// Smallest step the card stands behind: [`Self::gate_dt_s`] divided by
    /// 2^20. Not applied by [`step`].
    #[must_use]
    pub fn floor_dt_s(&self) -> Option<f64> {
        match self.gate_dt_s() {
            Some(gate) if self.gate_over_floor.is_finite() && self.gate_over_floor > 0.0 => {
                Some(gate / self.gate_over_floor)
            }
            _ => None,
        }
    }

    /// The rate constant of stage `stage` at `temperature_k`, 1/s: `A exp(−E
    /// / R T)` for the four decomposition stages, `k(T; T_x)` for the melt
    /// and the vent.
    #[must_use]
    pub fn rate_constant_per_s(&self, stage: usize, temperature_k: f64) -> f64 {
        if stage < 4 {
            self.frequency_factor_per_s[stage]
                * crate::math::exp(-self.activation_j_per_mol[stage] / (GAS_CONSTANT_J_PER_MOL_K * temperature_k))
        } else {
            let at = self.phase_k[stage - 4];
            crate::math::exp(-(at / self.phase_width_k) * (at / temperature_k - 1.0)) / self.phase_time_s
        }
    }

    /// Heat stage `stage` still holds at `amount`, J; negative for the melt
    /// and the vent, which take heat.
    #[must_use]
    pub fn remaining_heat_j(&self, stage: usize, amount: f64) -> f64 {
        self.stage_heat_j[stage] * amount
    }

    /// Heat the six stages still hold or take at `amounts`, in size, J.
    #[must_use]
    pub fn held_heat_j(&self, amounts: &[f64; STAGES]) -> f64 {
        let mut sum = 0.0;
        let mut stage = 0;
        while stage < STAGES {
            sum += self.remaining_heat_j(stage, amounts[stage]).abs();
            stage += 1;
        }
        sum
    }

    /// The held heat at or under which every reactant is consumed, J:
    /// [`CONSUMED_HEAT_FRACTION`] of [`CELL_CHEMICAL_HEAT_J`], 0.0205 J.
    #[must_use]
    pub fn consumed_heat_j(&self) -> f64 {
        self.consumed_heat_fraction * self.chemical_heat_j
    }

    /// Heat the cell loses to an oven, W, by convection and radiation; zero
    /// when adiabatic.
    #[must_use]
    pub fn loss_w(&self, temperature_k: f64, load: &RunawayLoad) -> f64 {
        if load.adiabatic == 1.0 {
            return 0.0;
        }
        let t2 = temperature_k * temperature_k;
        let o2 = load.oven_temperature_k * load.oven_temperature_k;
        self.convective_conductance_w_per_k * (temperature_k - load.oven_temperature_k)
            + self.radiative_conductance_w_per_k4 * (t2 * t2 - o2 * o2)
    }

    /// Whether `load` is outside those the records were taken under.
    #[must_use]
    pub fn outside_compared(&self, load: &RunawayLoad) -> bool {
        if load.heater_power_w > self.compared_heater_w {
            return true;
        }
        if load.adiabatic == 1.0 {
            return false;
        }
        load.heater_power_w > 0.0
            || load.oven_temperature_k < self.compared_oven_k[0]
            || load.oven_temperature_k > self.compared_oven_k[1]
    }
}

/// The cell. Write it only through [`RunawayState::try_start`] and [`step`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunawayState {
    /// Time since the start of the run.
    pub t_s: f64,
    /// Temperature of the cell, one for the whole body.
    pub temperature_k: f64,
    /// Lithium in the SEI, `c_sei`, 0 to 1; a fresh cell 0.15.
    pub sei_fraction: f64,
    /// Lithium in the graphite, `c_ne`; a fresh cell 0.75.
    pub anode_lithium_fraction: f64,
    /// Unreacted part of the cathode, `1 − α`, 0 to 1; a fresh cell 0.96.
    pub cathode_unreacted_fraction: f64,
    /// Electrolyte, `c_e`, 0 to 1.
    pub electrolyte_fraction: f64,
    /// Separator not yet melted, `s`, 0 to 1.
    pub separator_unmelted_fraction: f64,
    /// The vent's gas still held in the cell, `g`, 0 to 1.
    pub vent_gas_held_fraction: f64,
    /// SEI thickness, `z` = `z_0` + 0.75 − `c_ne`.
    pub sei_thickness: f64,
    /// Temperature at the start: the anchor of the heat stored.
    pub start_temperature_k: f64,
    /// Heat released by SEI decomposition since the start.
    pub heat_sei_j: f64,
    /// Heat released by the anode with the electrolyte since the start.
    pub heat_anode_j: f64,
    /// Heat released by the cathode with the electrolyte since the start.
    pub heat_cathode_j: f64,
    /// Heat released by electrolyte decomposition since the start.
    pub heat_electrolyte_j: f64,
    /// Heat the separator's melt released since the start: negative, taken.
    pub heat_separator_j: f64,
    /// Heat the vent released since the start: negative, carried away.
    pub heat_vent_j: f64,
    /// Sum of the six.
    pub heat_released_j: f64,
    /// Heat the heater put in since the start.
    pub heater_heat_j: f64,
    /// Heat lost to the oven since the start; negative while the oven warms
    /// the cell.
    pub heat_lost_j: f64,
    /// `C (T − T_start)`.
    pub heat_stored_j: f64,
    /// Net heat of the six stages now, W.
    pub chemical_heat_w: f64,
    /// `chemical_heat_w / C`, K/s.
    pub chemical_heating_k_s: f64,
    /// Signed `heat_stored − (heat_released + heater_heat − heat_lost)`.
    /// Positive is heat from nowhere.
    pub energy_defect_j: f64,
    /// `|energy_defect_j|` over [`CELL_CHEMICAL_HEAT_J`].
    pub residual_conservation_error: f64,
    /// Substeps the last step took; zero at the start.
    pub substeps: f64,
    /// `chemical_heating_k_s` is at or above 0.02 K per minute.
    pub self_heating: bool,
    /// `chemical_heating_k_s` is at or above 10 K per minute.
    pub runaway: bool,
    /// Half the separator or more has melted.
    pub separator_melted: bool,
    /// Half the vent's gas or more has left.
    pub vented: bool,
    /// The last step's load is outside those the records were taken under.
    pub outside_compared: bool,
}

impl RunawayState {
    /// A cell at `temperature_k` with the six amounts given, in stage order,
    /// at time 0 with the heat books empty. A fresh cell is 0.15, 0.75, 0.96,
    /// 1, 1 and 1; each amount may lie anywhere from 0 to its limit in
    /// [`AMOUNT_LIMITS`].
    pub fn try_start(cell: &Runaway18650LcoTwin, temperature_k: f64, amounts: [f64; STAGES]) -> Result<Self, StepReject> {
        let mut finite = temperature_k.is_finite();
        let mut stage = 0;
        while stage < STAGES {
            finite = finite && amounts[stage].is_finite();
            stage += 1;
        }
        if !finite {
            return Err(StepReject::NonFiniteInput);
        }
        if !cell.is_valid() {
            return Err(StepReject::InvalidCell);
        }
        let mut state = Self {
            t_s: 0.0,
            temperature_k,
            sei_fraction: amounts[0],
            anode_lithium_fraction: amounts[1],
            cathode_unreacted_fraction: amounts[2],
            electrolyte_fraction: amounts[3],
            separator_unmelted_fraction: amounts[4],
            vent_gas_held_fraction: amounts[5],
            sei_thickness: 0.0,
            start_temperature_k: temperature_k,
            heat_sei_j: 0.0,
            heat_anode_j: 0.0,
            heat_cathode_j: 0.0,
            heat_electrolyte_j: 0.0,
            heat_separator_j: 0.0,
            heat_vent_j: 0.0,
            heat_released_j: 0.0,
            heater_heat_j: 0.0,
            heat_lost_j: 0.0,
            heat_stored_j: 0.0,
            chemical_heat_w: 0.0,
            chemical_heating_k_s: 0.0,
            energy_defect_j: 0.0,
            residual_conservation_error: 0.0,
            substeps: 0.0,
            self_heating: false,
            runaway: false,
            separator_melted: false,
            vented: false,
            outside_compared: false,
        };
        bounds(cell, &state)?;
        refresh(cell, &mut state);
        if !state_is_finite(&state) {
            return Err(StepReject::NonFiniteState);
        }
        Ok(state)
    }

    /// A fresh cell at `temperature_k`.
    pub fn try_fresh(cell: &Runaway18650LcoTwin, temperature_k: f64) -> Result<Self, StepReject> {
        Self::try_start(cell, temperature_k, cell.start_amounts)
    }

    /// The six amounts, in stage order.
    #[must_use]
    pub fn amounts(&self) -> [f64; STAGES] {
        [
            self.sei_fraction,
            self.anode_lithium_fraction,
            self.cathode_unreacted_fraction,
            self.electrolyte_fraction,
            self.separator_unmelted_fraction,
            self.vent_gas_held_fraction,
        ]
    }

    fn heats(&self) -> [f64; STAGES] {
        [
            self.heat_sei_j,
            self.heat_anode_j,
            self.heat_cathode_j,
            self.heat_electrolyte_j,
            self.heat_separator_j,
            self.heat_vent_j,
        ]
    }

    fn set_amounts(&mut self, amounts: &[f64; STAGES]) {
        self.sei_fraction = amounts[0];
        self.anode_lithium_fraction = amounts[1];
        self.cathode_unreacted_fraction = amounts[2];
        self.electrolyte_fraction = amounts[3];
        self.separator_unmelted_fraction = amounts[4];
        self.vent_gas_held_fraction = amounts[5];
    }

    fn add_heats(&mut self, released: &[f64; STAGES]) {
        self.heat_sei_j += released[0];
        self.heat_anode_j += released[1];
        self.heat_cathode_j += released[2];
        self.heat_electrolyte_j += released[3];
        self.heat_separator_j += released[4];
        self.heat_vent_j += released[5];
    }
}

/// The temperature and the logarithms of the live amounts at one point of a
/// substep.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    temperature_k: f64,
    logs: [f64; STAGES],
}

/// The rates at one point.
#[derive(Clone, Copy, Debug)]
struct Rates {
    temperature_k_s: f64,
    logs_per_s: [f64; STAGES],
    loss_w: f64,
    chemical_w: f64,
    /// The stiffness of each stage in its logarithm, 1/s.
    stiffness_per_s: [f64; STAGES],
}

/// The rates of the six stages and of the temperature at `point`. A stage
/// that is not live has no rate.
fn rates(cell: &Runaway18650LcoTwin, point: Point, live: [bool; STAGES], load: &RunawayLoad) -> Rates {
    let temperature_k = point.temperature_k;
    let mut logs_per_s = [0.0; STAGES];
    let mut stiffness_per_s = [0.0; STAGES];
    let mut chemical_w = 0.0;
    let mut stage = 0;
    while stage < STAGES {
        if live[stage] {
            let amount = crate::math::exp(point.logs[stage]);
            let k = cell.rate_constant_per_s(stage, temperature_k);
            let (rate, stiffness) = match stage {
                1 => {
                    let slowed = k
                        * crate::math::exp((amount - cell.sei_plus_anode_lithium) / cell.sei_thickness_reference);
                    (slowed, slowed * (1.0 + amount / cell.sei_thickness_reference))
                }
                // With no converted part the autocatalytic rate is zero, and
                // stays zero: there is nothing for the substep to follow.
                2 => (k * (1.0 - amount), if amount < 1.0 { k } else { 0.0 }),
                _ => (k, k),
            };
            logs_per_s[stage] = -rate;
            stiffness_per_s[stage] = stiffness;
            chemical_w += cell.stage_heat_j[stage] * rate * amount;
        }
        stage += 1;
    }
    let loss_w = cell.loss_w(temperature_k, load);
    Rates {
        temperature_k_s: (chemical_w + load.heater_power_w - loss_w) / cell.heat_capacity_j_per_k,
        logs_per_s,
        loss_w,
        chemical_w,
        stiffness_per_s,
    }
}

/// One classical fourth-order Runge–Kutta substep of length `h` from
/// `point`: the point it lands on and the heat lost over it.
fn rk4(cell: &Runaway18650LcoTwin, point: Point, live: [bool; STAGES], load: &RunawayLoad, h: f64, first: &Rates) -> (Point, f64) {
    let shift = |by: &Rates, fraction: f64| {
        let mut logs = point.logs;
        let mut stage = 0;
        while stage < STAGES {
            logs[stage] += fraction * h * by.logs_per_s[stage];
            stage += 1;
        }
        Point {
            temperature_k: point.temperature_k + fraction * h * by.temperature_k_s,
            logs,
        }
    };
    let k1 = *first;
    let k2 = rates(cell, shift(&k1, 0.5), live, load);
    let k3 = rates(cell, shift(&k2, 0.5), live, load);
    let k4 = rates(cell, shift(&k3, 1.0), live, load);
    let mut logs = point.logs;
    let mut stage = 0;
    while stage < STAGES {
        logs[stage] += h / 6.0
            * (k1.logs_per_s[stage] + 2.0 * k2.logs_per_s[stage] + 2.0 * k3.logs_per_s[stage] + k4.logs_per_s[stage]);
        stage += 1;
    }
    let temperature_k = point.temperature_k
        + h / 6.0 * (k1.temperature_k_s + 2.0 * k2.temperature_k_s + 2.0 * k3.temperature_k_s + k4.temperature_k_s);
    let lost_j = h / 6.0 * (k1.loss_w + 2.0 * k2.loss_w + 2.0 * k3.loss_w + k4.loss_w);
    (Point { temperature_k, logs }, lost_j)
}

/// What a step hands to [`commit`]: the landing temperature and amounts,
/// and the heat booked over the step.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Trial {
    temperature_k: f64,
    amounts: [f64; STAGES],
    released_j: [f64; STAGES],
    heater_j: f64,
    lost_j: f64,
    substeps: u32,
}

/// One substep, as the step takes it: the landing point and the heat lost.
type Substep = fn(&Runaway18650LcoTwin, Point, [bool; STAGES], &RunawayLoad, f64, &Rates) -> (Point, f64);

/// The substeps of one step, each sized from the fastest live stage and the
/// temperature's rate.
fn integrate(
    cell: &Runaway18650LcoTwin,
    state: &RunawayState,
    load: &RunawayLoad,
    dt_s: f64,
    substep: Substep,
    sized: bool,
    fineness: f64,
) -> Result<Trial, StepReject> {
    let mut temperature_k = state.temperature_k;
    let mut amounts = state.amounts();
    let mut released_j = [0.0; STAGES];
    let mut lost_j = 0.0;
    let mut left_s = dt_s;
    let mut elapsed_s = 0.0;
    let mut count: u32 = 0;
    let spent_j = SPENT_HEAT_FRACTION * cell.chemical_heat_j;
    // The end is met where the heat held falls to the consumed heat; a cell
    // that starts there is spent and meets no end.
    let reacting = cell.held_heat_j(&amounts) > cell.consumed_heat_j();
    while left_s > 0.0 {
        if count >= MAX_SUBSTEPS {
            return Err(StepReject::SubstepsAboveBudget);
        }
        count += 1;
        let mut live = [false; STAGES];
        let mut logs = [0.0; STAGES];
        let mut stage = 0;
        while stage < STAGES {
            if amounts[stage] > 0.0
                && cell.stage_heat_j[stage] != 0.0
                && cell.remaining_heat_j(stage, amounts[stage]).abs() <= spent_j
            {
                // A spent stage finishes at the start of the substep: its
                // last heat, under 2^-52 of the cell's chemical heat, is
                // booked and goes into the temperature, so that a stage a
                // substep would empty at once is not left to Runge-Kutta.
                let last_j = cell.remaining_heat_j(stage, amounts[stage]);
                released_j[stage] += last_j;
                temperature_k += last_j / cell.heat_capacity_j_per_k;
                amounts[stage] = 0.0;
            }
            if amounts[stage] > 0.0 {
                live[stage] = true;
                logs[stage] = crate::math::log(amounts[stage]);
            }
            stage += 1;
        }
        let point = Point { temperature_k, logs };
        let first = rates(cell, point, live, load);
        if !(first.temperature_k_s.is_finite() && first.loss_w.is_finite() && first.chemical_w.is_finite()) {
            return Err(StepReject::NonFiniteState);
        }
        let mut h = left_s;
        if sized {
            // `fineness` is 1 for a step taken as through engine 0.15.1, and
            // 2, 4, 8, ... when the step is taken again with substeps that
            // much shorter (`step_as_computed`); x / 1 is x, bit for bit.
            let temperature_change_k = SUBSTEP_TEMPERATURE_CHANGE_K / fineness;
            let log_change = SUBSTEP_LOG_CHANGE / fineness;
            let warming = first.temperature_k_s.abs();
            if warming * h > temperature_change_k {
                h = temperature_change_k / warming;
            }
            let mut stage = 0;
            while stage < STAGES {
                // A stage that carries no heat (the vent, its heat fitted to
                // zero) moves nothing else and does not set the substep.
                if live[stage] && cell.stage_heat_j[stage] != 0.0 && first.stiffness_per_s[stage] * h > log_change
                {
                    h = log_change / first.stiffness_per_s[stage];
                }
                stage += 1;
            }
        }
        let (landed, lost) = substep(cell, point, live, load, h, &first);
        if !(landed.temperature_k.is_finite() && lost.is_finite()) {
            return Err(StepReject::NonFiniteState);
        }
        if !(landed.temperature_k > 0.0) {
            return Err(StepReject::TemperatureNotPositive);
        }
        if !(landed.temperature_k < cell.can_melting_k) {
            return Err(StepReject::CanMelting);
        }
        let mut stage = 0;
        while stage < STAGES {
            if live[stage] {
                let change = landed.logs[stage] - logs[stage];
                if change.is_nan() {
                    return Err(StepReject::NonFiniteState);
                }
                // `c (exp(Δ) − 1)` is the amount's change; at least `−c`.
                let delta = amounts[stage] * crate::math::exp_m1(change);
                released_j[stage] -= cell.remaining_heat_j(stage, delta);
                amounts[stage] += delta;
            }
            stage += 1;
        }
        temperature_k = landed.temperature_k;
        lost_j += lost;
        elapsed_s += h;
        if reacting && cell.held_heat_j(&amounts) <= cell.consumed_heat_j() {
            // The ledger is closed on the substeps so far before the end is
            // named, so a step that does not follow the reactions is named
            // by the ledger and does not pass for the end.
            let mut so_far = *state;
            so_far.temperature_k = temperature_k;
            so_far.add_heats(&released_j);
            so_far.heater_heat_j += load.heater_power_w * elapsed_s;
            so_far.heat_lost_j += lost_j;
            so_far.heat_released_j = sum(&so_far.heats());
            so_far.heat_stored_j = cell.heat_capacity_j_per_k * (so_far.temperature_k - so_far.start_temperature_k);
            so_far.energy_defect_j =
                so_far.heat_stored_j - (so_far.heat_released_j + so_far.heater_heat_j - so_far.heat_lost_j);
            close_ledger(cell, &so_far)?;
            return Err(StepReject::EveryReactantConsumed);
        }
        if h >= left_s {
            left_s = 0.0;
        } else {
            left_s -= h;
        }
    }
    Ok(Trial {
        temperature_k,
        amounts,
        released_j,
        heater_j: load.heater_power_w * dt_s,
        lost_j,
        substeps: count,
    })
}

fn sum(values: &[f64; STAGES]) -> f64 {
    let mut total = 0.0;
    let mut stage = 0;
    while stage < STAGES {
        total += values[stage];
        stage += 1;
    }
    total
}

/// Advance one step of `dt_s` and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    cell: &Runaway18650LcoTwin,
    state: &mut RunawayState,
    frame: &mut LastStateFrame64,
    load: RunawayLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    let before = *state;
    let stepped = step_landed(cell, state, frame, load, dt_s);
    #[cfg(any(test, debug_assertions, feature = "check_exceedances"))]
    if stepped.is_ok() {
        crate::exceedance::debug_check("runaway_18650_lco_twin", exceedances(cell, &before, state, load, dt_s), frame.flags);
    }
    stepped
}

/// The step itself; [`step`] is it with, in a build that checks
/// (`crate::exceedance::CHECKED`), the check that [`exceedances`] stands for
/// the flags it wrote.
fn step_landed(
    cell: &Runaway18650LcoTwin,
    state: &mut RunawayState,
    frame: &mut LastStateFrame64,
    load: RunawayLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    step_as_computed(cell, state, frame, load, dt_s)?;
    normal_or_zero_state(state);
    Ok(())
}

/// The step as its equations compute it, before a value of the landed
/// state below the normal range is written as zero.
fn step_as_computed(
    cell: &Runaway18650LcoTwin,
    state: &mut RunawayState,
    frame: &mut LastStateFrame64,
    load: RunawayLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(cell, state, &load, dt_s)?;
    // The step with the substeps as through engine 0.15.1: a step that lands
    // so lands as it always did.
    let mut fineness: f64 = 1.0;
    loop {
        let outcome = match integrate(cell, state, &load, dt_s, rk4, true, fineness) {
            Ok(trial) => commit(cell, state, frame, &load, dt_s, trial),
            Err(reject) => Err(reject),
        };
        // The ledger did not close: inside a substep sized at its start, a
        // stage whose rate climbs steeply with the temperature (the
        // separator's melt in an oven above about 440 K at 10 s) ran away
        // from the Runge-Kutta rule. The step is taken again with substeps
        // half as long in temperature and in each stage's logarithm, and
        // again, until it lands, a step needs more than MAX_SUBSTEPS
        // (SubstepsAboveBudget), or the substeps are 2^20 times shorter. The
        // ledger's limit is not changed. A step above the ceiling is not the
        // card's and is named by the ledger, as before.
        let refused = core::matches!(outcome, Err(StepReject::SpontaneousGeneration | StepReject::FirstLawBreach));
        let mine = match cell.gate_dt_s() {
            Some(gate_dt_s) => dt_s <= gate_dt_s,
            None => false,
        };
        if !refused || !mine || !(fineness < FINEST_SUBSTEPS) {
            return outcome;
        }
        fineness *= 2.0;
    }
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]).
fn normal_or_zero_state(state: &mut RunawayState) {
    state.t_s = normal_or_zero(state.t_s);
    state.temperature_k = normal_or_zero(state.temperature_k);
    state.sei_fraction = normal_or_zero(state.sei_fraction);
    state.anode_lithium_fraction = normal_or_zero(state.anode_lithium_fraction);
    state.cathode_unreacted_fraction = normal_or_zero(state.cathode_unreacted_fraction);
    state.electrolyte_fraction = normal_or_zero(state.electrolyte_fraction);
    state.separator_unmelted_fraction = normal_or_zero(state.separator_unmelted_fraction);
    state.vent_gas_held_fraction = normal_or_zero(state.vent_gas_held_fraction);
    state.sei_thickness = normal_or_zero(state.sei_thickness);
    state.start_temperature_k = normal_or_zero(state.start_temperature_k);
    state.heat_sei_j = normal_or_zero(state.heat_sei_j);
    state.heat_anode_j = normal_or_zero(state.heat_anode_j);
    state.heat_cathode_j = normal_or_zero(state.heat_cathode_j);
    state.heat_electrolyte_j = normal_or_zero(state.heat_electrolyte_j);
    state.heat_separator_j = normal_or_zero(state.heat_separator_j);
    state.heat_vent_j = normal_or_zero(state.heat_vent_j);
    state.heat_released_j = normal_or_zero(state.heat_released_j);
    state.heater_heat_j = normal_or_zero(state.heater_heat_j);
    state.heat_lost_j = normal_or_zero(state.heat_lost_j);
    state.heat_stored_j = normal_or_zero(state.heat_stored_j);
    state.chemical_heat_w = normal_or_zero(state.chemical_heat_w);
    state.chemical_heating_k_s = normal_or_zero(state.chemical_heating_k_s);
    state.energy_defect_j = normal_or_zero(state.energy_defect_j);
    state.residual_conservation_error = normal_or_zero(state.residual_conservation_error);
    state.substeps = normal_or_zero(state.substeps);
}

fn validate(cell: &Runaway18650LcoTwin, state: &RunawayState, load: &RunawayLoad, dt_s: f64) -> Result<(), StepReject> {
    if !(dt_s.is_finite()
        && load.oven_temperature_k.is_finite()
        && load.heater_power_w.is_finite()
        && load.adiabatic.is_finite())
    {
        return Err(StepReject::NonFiniteInput);
    }
    if !state_is_finite(state) {
        return Err(StepReject::NonFiniteState);
    }
    if dt_s <= 0.0 {
        return Err(StepReject::NonPositiveDt);
    }
    if !cell.is_valid() {
        return Err(StepReject::InvalidCell);
    }
    if load.heater_power_w < 0.0 {
        return Err(StepReject::HeaterPowerNegative);
    }
    if !(load.adiabatic == 0.0 || load.adiabatic == 1.0) {
        return Err(StepReject::AdiabaticNotZeroOrOne);
    }
    // The oven temperature enters only where heat crosses the can.
    if load.adiabatic == 0.0 && !(load.oven_temperature_k > 0.0) {
        return Err(StepReject::OvenTemperatureNotPositive);
    }
    bounds(cell, state)?;
    // The books are closed again on the state as it stands; the recorded
    // derived values are not trusted.
    let mut held = *state;
    refresh(cell, &mut held);
    close_ledger(cell, &held)
}

/// The physical bounds of the temperature and the six amounts.
fn bounds(cell: &Runaway18650LcoTwin, state: &RunawayState) -> Result<(), StepReject> {
    if !(state.temperature_k > 0.0) {
        return Err(StepReject::TemperatureNotPositive);
    }
    if !(state.temperature_k < cell.can_melting_k) {
        return Err(StepReject::CanMelting);
    }
    let amounts = state.amounts();
    let names = [
        StepReject::SeiFractionOutsideCell,
        StepReject::AnodeLithiumOutsideCell,
        StepReject::CathodeUnreactedOutsideCell,
        StepReject::ElectrolyteOutsideCell,
        StepReject::SeparatorOutsideCell,
        StepReject::VentGasOutsideCell,
    ];
    let mut stage = 0;
    while stage < STAGES {
        if !(amounts[stage] >= 0.0 && amounts[stage] <= cell.amount_limits[stage]) {
            return Err(names[stage]);
        }
        stage += 1;
    }
    // `1 − u` is exact for an unreacted part `u` from 0.5 to 1.
    let converted = 1.0 - amounts[2];
    if converted > 0.0 && converted < cell.cathode_converted_least {
        return Err(StepReject::CathodeUnreactedOutsideCell);
    }
    Ok(())
}

/// `Ok` when the defect of `state` is inside the limit.
fn close_ledger(cell: &Runaway18650LcoTwin, state: &RunawayState) -> Result<(), StepReject> {
    let gate = cell.residual_conservation_limit * cell.chemical_heat_j;
    if !state.energy_defect_j.is_finite() {
        return Err(StepReject::NonFiniteState);
    }
    if state.energy_defect_j > gate {
        return Err(StepReject::SpontaneousGeneration);
    }
    if state.energy_defect_j < -gate {
        return Err(StepReject::FirstLawBreach);
    }
    Ok(())
}

/// Book the trial, close the ledger, and publish.
fn commit(
    cell: &Runaway18650LcoTwin,
    state: &mut RunawayState,
    frame: &mut LastStateFrame64,
    load: &RunawayLoad,
    dt_s: f64,
    trial: Trial,
) -> Result<(), StepReject> {
    let saved = *state;
    state.t_s += dt_s;
    state.temperature_k = trial.temperature_k;
    state.set_amounts(&trial.amounts);
    state.add_heats(&trial.released_j);
    state.heater_heat_j += trial.heater_j;
    state.heat_lost_j += trial.lost_j;
    state.substeps = trial.substeps as f64;
    state.outside_compared = cell.outside_compared(load);
    refresh(cell, state);
    if !state_is_finite(state) {
        *state = saved;
        return Err(StepReject::NonFiniteState);
    }
    if let Err(reject) = bounds(cell, state) {
        *state = saved;
        return Err(reject);
    }
    if let Err(reject) = close_ledger(cell, state) {
        *state = saved;
        return Err(reject);
    }
    publish(state, frame);
    Ok(())
}

/// The derived values of the temperature, the amounts and the sums now in
/// the state.
fn refresh(cell: &Runaway18650LcoTwin, state: &mut RunawayState) {
    state.sei_thickness = cell.sei_plus_anode_lithium - state.anode_lithium_fraction;
    state.heat_released_j = sum(&state.heats());
    state.heat_stored_j = cell.heat_capacity_j_per_k * (state.temperature_k - state.start_temperature_k);
    state.energy_defect_j = state.heat_stored_j - (state.heat_released_j + state.heater_heat_j - state.heat_lost_j);
    state.residual_conservation_error = state.energy_defect_j.abs() / cell.chemical_heat_j;
    let amounts = state.amounts();
    let mut live = [false; STAGES];
    let mut logs = [0.0; STAGES];
    let mut stage = 0;
    while stage < STAGES {
        if amounts[stage] > 0.0 && state.temperature_k > 0.0 {
            live[stage] = true;
            logs[stage] = crate::math::log(amounts[stage]);
        }
        stage += 1;
    }
    // The stage heat does not depend on the oven or the heater.
    let quiet = RunawayLoad {
        oven_temperature_k: state.temperature_k,
        heater_power_w: 0.0,
        adiabatic: 1.0,
    };
    let now = rates(
        cell,
        Point {
            temperature_k: state.temperature_k,
            logs,
        },
        live,
        &quiet,
    );
    state.chemical_heat_w = now.chemical_w;
    state.chemical_heating_k_s = now.chemical_w / cell.heat_capacity_j_per_k;
    state.self_heating = state.chemical_heating_k_s >= SELF_HEATING_RATE_K_PER_S;
    state.runaway = state.chemical_heating_k_s >= RUNAWAY_RATE_K_PER_S;
    state.separator_melted = state.separator_unmelted_fraction <= 0.5;
    state.vented = state.vent_gas_held_fraction <= 0.5;
}

fn publish(state: &RunawayState, frame: &mut LastStateFrame64) {
    frame.t = state.t_s;
    frame.pos[0] = state.temperature_k as f32;
    frame.pos[1] = state.chemical_heat_w as f32;
    frame.pos[2] = state.heat_released_j as f32;
    frame.vel[0] = state.anode_lithium_fraction as f32;
    frame.vel[1] = state.cathode_unreacted_fraction as f32;
    frame.vel[2] = state.electrolyte_fraction as f32;
    frame.force_torque = state.heat_lost_j as f32;
    frame.residual = state.residual_conservation_error as f32;
    frame.flags = (if state.self_heating { FLAG_SELF_HEATING } else { 0 })
        | (if state.runaway { FLAG_RUNAWAY } else { 0 })
        | (if state.separator_melted { FLAG_SEPARATOR_MELTED } else { 0 })
        | (if state.vented { FLAG_VENTED } else { 0 })
        | (if state.outside_compared { FLAG_OUTSIDE_COMPARED } else { 0 });
}

fn state_is_finite(state: &RunawayState) -> bool {
    let amounts = state.amounts();
    let heats = state.heats();
    let mut finite = true;
    let mut stage = 0;
    while stage < STAGES {
        finite = finite && amounts[stage].is_finite() && heats[stage].is_finite();
        stage += 1;
    }
    finite
        && state.t_s.is_finite()
        && state.temperature_k.is_finite()
        && state.sei_thickness.is_finite()
        && state.start_temperature_k.is_finite()
        && state.heat_released_j.is_finite()
        && state.heater_heat_j.is_finite()
        && state.heat_lost_j.is_finite()
        && state.heat_stored_j.is_finite()
        && state.chemical_heat_w.is_finite()
        && state.chemical_heating_k_s.is_finite()
        && state.energy_defect_j.is_finite()
        && state.residual_conservation_error.is_finite()
        && state.substeps.is_finite()
}


/// Every limit a flag of this card can mark: what [`exceedances`] reports
/// beside a flag's bit (`crate::exceedance`).
pub const EXCEEDANCE_KINDS: &[crate::exceedance::ExceedanceKind] = &[
    SELF_HEATING_KIND,
    RUNAWAY_KIND,
    SEPARATOR_KIND,
    VENTED_KIND,
    HEATER_HIGH,
    HEATER_IN_OVEN,
    OVEN_LOW,
    OVEN_HIGH,
];

use crate::exceedance::{Exceedance, ExceedanceKind, Exceedances, Side};

const SELF_HEATING_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_SELF_HEATING",
    FLAG_SELF_HEATING,
    "chemical_heating_k_s",
    "K/s",
    Side::AtOrAbove,
    SELF_HEATING_RATE_K_PER_S,
    "0.02 K a minute, the self-heating an accelerating-rate calorimeter detects",
);
const RUNAWAY_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_RUNAWAY",
    FLAG_RUNAWAY,
    "chemical_heating_k_s",
    "K/s",
    Side::AtOrAbove,
    RUNAWAY_RATE_K_PER_S,
    "10 K a minute: thermal runaway",
);
const SEPARATOR_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_SEPARATOR_MELTED",
    FLAG_SEPARATOR_MELTED,
    "separator_unmelted_fraction",
    "1",
    Side::AtOrBelow,
    0.5,
    "half the separator unmelted",
);
const VENTED_KIND: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_VENTED",
    FLAG_VENTED,
    "vent_gas_held_fraction",
    "1",
    Side::AtOrBelow,
    0.5,
    "half the vent's gas still held in the cell",
);
const HEATER_HIGH: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_COMPARED",
    FLAG_OUTSIDE_COMPARED,
    "heater_power_w",
    "W",
    Side::Above,
    COMPARED_HEATER_W,
    "the largest heater power of the records, 1.2 W",
);
const HEATER_IN_OVEN: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_COMPARED",
    FLAG_OUTSIDE_COMPARED,
    "heater_power_w",
    "W",
    Side::Above,
    0.0,
    "no heater in an oven: the oven records were heated by the oven alone, the heater records taken adiabatic",
);
const OVEN_WORDS: &str = "the oven temperatures of the records, 145 to 155 degC; not tested when adiabatic";
const OVEN_LOW: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_COMPARED",
    FLAG_OUTSIDE_COMPARED,
    "oven_temperature_k",
    "K",
    Side::Below,
    COMPARED_OVEN_K[0],
    OVEN_WORDS,
);
const OVEN_HIGH: ExceedanceKind = ExceedanceKind::fixed(
    "FLAG_OUTSIDE_COMPARED",
    FLAG_OUTSIDE_COMPARED,
    "oven_temperature_k",
    "K",
    Side::Above,
    COMPARED_OVEN_K[1],
    OVEN_WORDS,
);

/// The limits the step from `before` to `after` under `load` over `dt_s`
/// is past: one record for each limit a flag set on its frame marks. The
/// conditions are those the step writes its flags' booleans by and
/// [`Runaway18650LcoTwin::outside_compared`], each limit it is past.
pub fn exceedances(
    cell: &Runaway18650LcoTwin,
    before: &RunawayState,
    after: &RunawayState,
    load: RunawayLoad,
    dt_s: f64,
) -> Exceedances {
    let _ = (before, dt_s);
    let mut records = Exceedances::new();
    records.push_if(after.self_heating, Exceedance::fixed(SELF_HEATING_KIND, after.chemical_heating_k_s));
    records.push_if(after.runaway, Exceedance::fixed(RUNAWAY_KIND, after.chemical_heating_k_s));
    records.push_if(after.separator_melted, Exceedance::fixed(SEPARATOR_KIND, after.separator_unmelted_fraction));
    records.push_if(after.vented, Exceedance::fixed(VENTED_KIND, after.vent_gas_held_fraction));
    if after.outside_compared {
        let heater = load.heater_power_w;
        records.push_if(heater > cell.compared_heater_w, Exceedance::at(HEATER_HIGH, cell.compared_heater_w, heater));
        if load.adiabatic != 1.0 {
            let oven = load.oven_temperature_k;
            records.push_if(heater > 0.0, Exceedance::fixed(HEATER_IN_OVEN, heater));
            records.push_if(oven < cell.compared_oven_k[0], Exceedance::at(OVEN_LOW, cell.compared_oven_k[0], oven));
            records.push_if(oven > cell.compared_oven_k[1], Exceedance::at(OVEN_HIGH, cell.compared_oven_k[1], oven));
        }
    }
    records
}

#[cfg(test)]
mod tests;
