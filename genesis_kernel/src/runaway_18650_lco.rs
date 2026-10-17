#![no_implicit_prelude]

//! Thermal runaway of one 18650 lithium-ion cell, LiCoO2 cathode and graphite
//! anode, held in an oven or heated, as one lumped body.
//!
//! The specimen is the 18650 cell of the lumped abuse model of Hatchard,
//! MacNeil, Basu and Dahn (2001), J. Electrochem. Soc. 148, A755, with the
//! four decomposition stages of Kim, Pesaran and Spotnitz (2007), J. Power
//! Sources 170, 476: a cylinder 18 mm across and 65 mm high, LiCoO2 against
//! graphite in an organic carbonate electrolyte, fully charged. The kinetics
//! of the anode and cathode stages were fitted by Dahn's group to
//! accelerating-rate calorimetry of the electrode materials (Richard and Dahn
//! 1999, J. Electrochem. Soc. 146, 2068 and 2078, for the anode; MacNeil and
//! Dahn 2001 for the cathode), and the electrolyte stage is that of Spotnitz
//! and Franklin (2003), J. Power Sources 113, 81. Neither 2001 nor 2007
//! paper is open; every constant of the card is taken from open reprints of
//! them: the frequency factors, activation energies, heats, contents and
//! starting amounts from Ostanek (2025), Purdue University Research
//! Repository, doi 10.4231/NWZX-2675, Table 2 (CC BY 4.0), whose frequency
//! factors, activation energies, heats and starting amounts Lopez,
//! Jeevarajan and Mukherjee (2015), J. Electrochem. Soc. 162, A2163, Table
//! II, print the same; the density and specific heat of the jelly roll from
//! Coman et al. (2017), J. Electrochem. Soc. 164, A587, Table I, citing
//! Hatchard et al.; the convective coefficient 7.17 W/(m² K) and emissivity
//! 0.8 from Lopez et al. (2015) and Ostanek (2025), Table 1. Lopez et al.
//! (2015) print other contents (1.39e3, 1.3e3 and 5.0e2 kg/m³ for carbon,
//! cathode and electrolyte); the card takes Ostanek's 610.4, 1221.0 and
//! 406.9. The card takes those numbers as published and fits nothing.
//!
//! The four stages. Each is a dimensionless amount that only falls, and each
//! releases its heat `H W` per unit of the amount, in J/m³ of cell:
//!
//! ```text
//!   SEI decomposition        R_sei = A_sei exp(−E_sei / R T) c_sei
//!   anode with electrolyte   R_ne  = A_ne  exp(−E_ne  / R T) exp(−z / z_ref) c_ne
//!   cathode with electrolyte R_pe  = A_pe  exp(−E_pe  / R T) α (1 − α)
//!   electrolyte              R_e   = A_e   exp(−E_e   / R T) c_e
//!
//!   dc_sei/dt = −R_sei     dc_ne/dt = −R_ne     dz/dt = +R_ne
//!   dα/dt     = +R_pe      dc_e/dt  = −R_e
//!
//!   q   = V (H_sei W_c R_sei + H_ne W_c R_ne + H_pe W_p R_pe + H_e W_e R_e)
//!   L   = h A (T − T_oven) + ε σ A (T⁴ − T_oven⁴)        (zero when adiabatic)
//!   C dT/dt = q + P − L,   C = ρ c_p V
//! ```
//!
//! `c_sei` is the lithium in the solid-electrolyte interphase, `c_ne` the
//! lithium in the graphite, `z` the thickness of the interphase that the
//! anode reaction grows and that slows it, `α` the converted part of the
//! cathode and `c_e` the electrolyte. `z + c_ne` never changes; this cell
//! starts at `z` = 0.033 and `c_ne` = 0.75, so `z` = 0.783 − `c_ne` always and
//! is not a separate state. The card carries `1 − α`, the unreacted part of
//! the cathode, so that its last part is not lost to rounding. `P` is the
//! heater's power and `T_oven` the oven's temperature, the load.
//!
//! The bounds of the four amounts are where their equations hold. Each is a
//! part of what the cell holds, from none to all of it: the lithium in the SEI
//! and the electrolyte are fractions, at most 1; the unreacted cathode
//! `1 − α` is at most 1, where `α` is 0 and the autocatalytic rate `α (1 − α)`
//! is zero (above it the rate would make cathode), and it is 1 or at most
//! 1 − 2^-33 ([`CATHODE_CONVERTED_LEAST`]): the card carries `1 − α` in 64
//! bits, and a converted part under 1.16e-10 is carried to fewer than 20 bits
//! and its growth is lost to rounding; the lithium in the graphite
//! is at most 0.783, `z + c_ne`, where the SEI thickness `z` is zero (above
//! it the thickness would be below zero). A fresh cell starts at 0.15, 0.75,
//! 0.96 and 1, inside them; a start above a fresh cell's amount is a cell
//! with more of that reactant, and the ledger holds it the same way. Below
//! zero or past its limit an amount is a named refusal. The temperature must
//! be above zero kelvin, and below 1811 K, the melting point of iron, where
//! the steel can melts and the cell is no longer a cell.
//!
//! The onset is reported, not refused. [`FLAG_SELF_HEATING`] is set on a
//! frame whose chemical heat warms the cell at 0.02 K per minute or more, the
//! detection threshold of an accelerating-rate calorimeter, and
//! [`FLAG_RUNAWAY`] on a frame whose chemical heat warms it at 10 K per minute
//! or more, the onset of thermal runaway as Golubkov et al. (2014), RSC Adv.
//! 4, 3633, read it from their cells. The runaway's end is
//! [`StepReject::EveryReactantConsumed`]: the heat the four stages still hold
//! falls to [`CONSUMED_HEAT_FRACTION`] of the cell's chemical heat or under,
//! 0.0205 J, the ledger's own limit, which would warm the cell by under
//! 0.6 mK. It is tested at the end of every substep, so whether a run meets
//! it does not follow the step asked for, and the step that would carry the
//! cell there is not taken. A cell that starts with no more than that heat
//! is spent: it is stepped as a body that gains and loses heat, and no end is
//! met. In this model the anode stage slows itself as its interphase grows,
//! to a rate of `A exp(−E / R T) exp(−0.783 / 0.033)` per second as its
//! lithium runs out: in the oven runs worked here about a quarter of the
//! anode's lithium is left when the cell has cooled, and a cell held at a
//! temperature after the 155 degC oven run's runaway meets the end after 1.6
//! hours at 1,200 K, 5.6 hours at 1,100 K, 24.6 hours at 1,000 K, 6.2 days at
//! 900 K and 60 days at 800 K; at 700 K it has not met it after 410 days.
//!
//! The step. Runaway is stiff: at its peak the electrolyte stage has a rate
//! of 10^5 to 10^11 per second. The card divides each step into substeps.
//! Each substep is classical fourth-order Runge–Kutta over the temperature,
//! the natural logarithm of each amount, and the heat lost, and is no longer
//! than [`SUBSTEP_LOG_CHANGE`] (0.1) over the fastest stage's rate in its
//! logarithm, nor than a change of [`SUBSTEP_TEMPERATURE_CHANGE_K`] (1 K) at
//! the substep's starting rate. In the logarithm of an amount a stage that is
//! nearly spent is not stiff; a stage whose remaining heat is under
//! [`SPENT_HEAT_FRACTION`] (2^-52) of the cell's chemical heat no longer sets
//! the substep. A step that would need more than [`MAX_SUBSTEPS`] substeps is
//! [`StepReject::SubstepsAboveBudget`]. The step ceiling is a hundredth of the
//! cell's convective time constant `C / (h A)`.
//!
//! The ledger. Each substep books the heat each stage released in closed
//! form from its amount, `H W V (c_before − c_after)`, the heater's heat
//! `P h` and the heat lost, integrated with the temperature. The heat stored
//! in the cell is `C (T − T_start)`. The defect
//!
//! ```text
//!   defect = C (T − T_start) − (Σ released + heater − lost)
//! ```
//!
//! is held to [`RESIDUAL_CONSERVATION_LIMIT`] of the cell's whole chemical
//! heat, 20,499 J. The temperature is integrated from the heat rates and the
//! amounts from their logarithms, so the two books agree only as far as the
//! step follows the reactions: a step that skips the substeps, or a
//! temperature, an amount or a heat booked otherwise, is named
//! [`StepReject::SpontaneousGeneration`] or [`StepReject::FirstLawBreach`].
//! The time, the start temperature and the heat sums begin again at each
//! start.
//!
//! What is left out: the temperature inside the cell (one temperature for
//! the whole body; Kim et al. 2007 show the 18650 is close to uniform in an
//! oven), venting and the loss of electrolyte and heat with the vented gas,
//! the internal short circuit, the electrical energy of the charge,
//! combustion of the vented gas, the melting of the separator and of the
//! aluminium collector, a heat capacity that changes with temperature, and
//! the calorimeter's own heat capacity.
//!
//! A step writes the time, the temperature, the chemical heat, the heat
//! released, three of the amounts, the heat lost and the residual into a
//! [`LastStateFrame64`]. The 16-byte frame proof is not touched.

extern crate core;

use core::prelude::v1::*;

use crate::frame::LastStateFrame64;
use crate::normal::normal_or_zero;

/// Gas constant, J/(mol K), as the Hatchard–Kim model is stated with it.
pub const GAS_CONSTANT_J_PER_MOL_K: f64 = 8.314;
/// Stefan–Boltzmann constant, W/(m² K⁴), CODATA 2018.
pub const STEFAN_BOLTZMANN_W_PER_M2_K4: f64 = 5.670374419e-8;

/// SEI decomposition: frequency factor, 1/s. Ostanek (2025), Table 2.
pub const SEI_FREQUENCY_FACTOR_PER_S: f64 = 1.667e15;
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
/// Cathode with electrolyte: frequency factor, 1/s. Ostanek (2025),
/// Table 2.
pub const CATHODE_FREQUENCY_FACTOR_PER_S: f64 = 6.667e13;
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
/// Electrolyte decomposition: heat released, J per kg of electrolyte. Ostanek
/// (2025), Table 2.
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
/// Dimensionless SEI thickness at the start of a fresh cell. Ostanek
/// (2025), Table 2.
pub const SEI_THICKNESS_START: f64 = 0.033;
/// The reference SEI thickness of the anode stage's slowing term. Ostanek
/// (2025), Table 2.
pub const SEI_THICKNESS_REFERENCE: f64 = 0.033;
/// Converted part of the cathode at the start of a fresh cell. Ostanek
/// (2025), Table 2.
pub const CATHODE_CONVERSION_START: f64 = 0.04;
/// Electrolyte at the start of a fresh cell. Ostanek (2025), Table 2.
pub const ELECTROLYTE_START: f64 = 1.0;

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
/// Convective coefficient from the can to the oven, W/(m² K): Hatchard et al.
/// (2001), as Lopez et al. (2015) and Ostanek (2025), Table 1, state it.
pub const CONVECTION_W_PER_M2_K: f64 = 7.17;
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
/// `z + c_ne`, which no reaction changes: 0.033 + 0.75.
pub const SEI_PLUS_ANODE_LITHIUM: f64 = SEI_THICKNESS_START + ANODE_LITHIUM_START;
/// The cell's whole chemical heat from a fresh start, J: `V (H_sei W_c 0.15 +
/// H_ne W_c 0.75 + H_pe W_p 0.96 + H_e W_e 1)`, 20,499 J. The ledger's scale.
pub const CELL_CHEMICAL_HEAT_J: f64 = CELL_VOLUME_M3
    * (SEI_HEAT_J_PER_KG * CARBON_CONTENT_KG_PER_M3 * SEI_FRACTION_START
        + ANODE_HEAT_J_PER_KG * CARBON_CONTENT_KG_PER_M3 * ANODE_LITHIUM_START
        + CATHODE_HEAT_J_PER_KG * CATHODE_CONTENT_KG_PER_M3 * (1.0 - CATHODE_CONVERSION_START)
        + ELECTROLYTE_HEAT_J_PER_KG * ELECTROLYTE_CONTENT_KG_PER_M3 * ELECTROLYTE_START);

/// A self-heating rate of 0.02 K per minute, in K/s: the detection threshold
/// of an accelerating-rate calorimeter.
pub const SELF_HEATING_RATE_K_PER_S: f64 = 0.02 / 60.0;
/// A self-heating rate of 10 K per minute, in K/s: the onset of thermal
/// runaway as Golubkov et al. (2014) read it.
pub const RUNAWAY_RATE_K_PER_S: f64 = 10.0 / 60.0;

/// Largest change of the logarithm of a stage's amount, times its stiffness,
/// in one substep.
pub const SUBSTEP_LOG_CHANGE: f64 = 0.1;
/// Largest change of temperature in one substep at the substep's starting
/// rate, K.
pub const SUBSTEP_TEMPERATURE_CHANGE_K: f64 = 1.0;
/// A stage whose remaining heat is under this part of
/// [`CELL_CHEMICAL_HEAT_J`] no longer sets the substep: 2^-52.
pub const SPENT_HEAT_FRACTION: f64 = 2.220446049250313e-16;
/// Most substeps in one step: 2^20.
pub const MAX_SUBSTEPS: u32 = 1_048_576;
/// Step ceiling over the convective time constant `C / (h A)`.
pub const GATE_OVER_CONVECTIVE_TIME: f64 = 0.01;
/// Step ceiling divided by step floor: 2^20.
pub const GATE_OVER_FLOOR: f64 = 1_048_576.0;
/// `|defect|` over [`CELL_CHEMICAL_HEAT_J`] must stay at or under this.
pub const RESIDUAL_CONSERVATION_LIMIT: f64 = 1.0e-6;
/// Every reactant is consumed when the heat the four stages still hold is at
/// or under this part of [`CELL_CHEMICAL_HEAT_J`]: 1e-6, 0.0205 J, the
/// ledger's own limit, heat that would warm the cell by under 0.6 mK.
pub const CONSUMED_HEAT_FRACTION: f64 = 1.0e-6;
/// Where the equations of the four amounts hold, in stage order: the lithium
/// in the SEI at most 1, a fraction; the lithium in the graphite at most
/// [`SEI_PLUS_ANODE_LITHIUM`], 0.783, where the SEI thickness is zero; the
/// unreacted cathode at most 1, where its converted part is zero; the
/// electrolyte at most 1, all of it. Each at least zero.
pub const AMOUNT_LIMITS: [f64; 4] = [1.0, SEI_PLUS_ANODE_LITHIUM, 1.0, 1.0];
/// The least converted part of the cathode, `α`, other than zero, that the
/// card carries: 2^-33, 1.16e-10. The card holds the unreacted part `1 − α`
/// in 64 bits, which resolves `α` to 2^-53; at 2^-33 that is 2^-20 of `α`,
/// under the ledger's 1e-6, and nearer zero the autocatalytic growth of `α`
/// is lost to rounding (from 4.4e-16 down it does not grow at all). At zero
/// the stage has no rate and never starts.
pub const CATHODE_CONVERTED_LEAST: f64 = 1.1641532182693481e-10;

/// The chemical heat at the end of the step warms the cell at
/// [`SELF_HEATING_RATE_K_PER_S`] or more: self-heating an accelerating-rate
/// calorimeter detects.
pub const FLAG_SELF_HEATING: u64 = 1 << 0;
/// The chemical heat at the end of the step warms the cell at
/// [`RUNAWAY_RATE_K_PER_S`] or more: thermal runaway.
pub const FLAG_RUNAWAY: u64 = 1 << 1;

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
    /// The lithium in the graphite is below 0 or above 0.783: above it the
    /// SEI thickness, 0.783 less this, would be below zero.
    AnodeLithiumOutsideCell,
    /// The unreacted part of the cathode is below 0 or above 1, or above
    /// 1 - 2^-33 and under 1: above 1 the converted part would be below zero
    /// and the cathode stage would make cathode; just under 1 the converted
    /// part, under 1.16e-10, is carried to fewer than 20 bits and its growth
    /// is lost to rounding.
    CathodeUnreactedOutsideCell,
    /// The electrolyte is below 0 or above 1: a fraction of the cell's fill,
    /// it is between none and all.
    ElectrolyteOutsideCell,
    /// The step would carry the cell to every reactant consumed: the heat the
    /// lithium in the SEI and in the graphite, the unreacted cathode and the
    /// electrolyte still hold falls to 1e-6 of the cell's chemical heat or
    /// under, 0.0205 J, and the runaway has finished.
    EveryReactantConsumed,
    /// The step would need more than 1,048,576 substeps to follow its fastest
    /// reaction.
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
pub struct Runaway18650Lco {
    /// Recorded copies of the four frequency factors, in stage order: SEI,
    /// anode, cathode, electrolyte.
    pub frequency_factor_per_s: [f64; 4],
    /// Recorded copies of the four activation energies.
    pub activation_j_per_mol: [f64; 4],
    /// Recorded copies of each stage's heat times its content, J/m³.
    pub heat_density_j_per_m3: [f64; 4],
    /// Recorded copies of the four amounts of a fresh cell, the unreacted
    /// cathode as `1 − α`.
    pub start_amounts: [f64; 4],
    /// Recorded copy of [`SEI_PLUS_ANODE_LITHIUM`].
    pub sei_plus_anode_lithium: f64,
    /// Recorded copy of [`SEI_THICKNESS_REFERENCE`].
    pub sei_thickness_reference: f64,
    /// Recorded copy of [`CELL_VOLUME_M3`].
    pub volume_m3: f64,
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
    /// Recorded copy of [`GATE_OVER_CONVECTIVE_TIME`].
    pub gate_over_convective_time: f64,
    /// Recorded copy of [`GATE_OVER_FLOOR`].
    pub gate_over_floor: f64,
    /// Recorded copy of [`RESIDUAL_CONSERVATION_LIMIT`].
    pub residual_conservation_limit: f64,
    /// Recorded copy of [`CONSUMED_HEAT_FRACTION`].
    pub consumed_heat_fraction: f64,
    /// Recorded copies of [`AMOUNT_LIMITS`].
    pub amount_limits: [f64; 4],
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
const HEAT_DENSITIES: [f64; 4] = [
    SEI_HEAT_J_PER_KG * CARBON_CONTENT_KG_PER_M3,
    ANODE_HEAT_J_PER_KG * CARBON_CONTENT_KG_PER_M3,
    CATHODE_HEAT_J_PER_KG * CATHODE_CONTENT_KG_PER_M3,
    ELECTROLYTE_HEAT_J_PER_KG * ELECTROLYTE_CONTENT_KG_PER_M3,
];
const START_AMOUNTS: [f64; 4] = [
    SEI_FRACTION_START,
    ANODE_LITHIUM_START,
    1.0 - CATHODE_CONVERSION_START,
    ELECTROLYTE_START,
];
const CONVECTIVE_CONDUCTANCE_W_PER_K: f64 = CONVECTION_W_PER_M2_K * CELL_SURFACE_M2;
const RADIATIVE_CONDUCTANCE_W_PER_K4: f64 =
    CAN_EMISSIVITY * STEFAN_BOLTZMANN_W_PER_M2_K4 * CELL_SURFACE_M2;

impl Runaway18650Lco {
    /// Card: the 18650 LiCoO2 cell of Hatchard et al. (2001) and Kim et al.
    /// (2007). The constructor takes no arguments.
    #[must_use]
    pub fn hatchard_kim() -> Self {
        let cell = Self {
            frequency_factor_per_s: FREQUENCY_FACTORS,
            activation_j_per_mol: ACTIVATION_ENERGIES,
            heat_density_j_per_m3: HEAT_DENSITIES,
            start_amounts: START_AMOUNTS,
            sei_plus_anode_lithium: SEI_PLUS_ANODE_LITHIUM,
            sei_thickness_reference: SEI_THICKNESS_REFERENCE,
            volume_m3: CELL_VOLUME_M3,
            heat_capacity_j_per_k: CELL_HEAT_CAPACITY_J_PER_K,
            convective_conductance_w_per_k: CONVECTIVE_CONDUCTANCE_W_PER_K,
            radiative_conductance_w_per_k4: RADIATIVE_CONDUCTANCE_W_PER_K4,
            can_melting_k: CAN_MELTING_K,
            chemical_heat_j: CELL_CHEMICAL_HEAT_J,
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
        let same4 = |a: &[f64; 4], b: &[f64; 4]| {
            let mut index = 0;
            while index < 4 {
                if a[index].to_bits() != b[index].to_bits() {
                    return false;
                }
                index += 1;
            }
            true
        };
        same4(&self.frequency_factor_per_s, &FREQUENCY_FACTORS)
            && same4(&self.activation_j_per_mol, &ACTIVATION_ENERGIES)
            && same4(&self.heat_density_j_per_m3, &HEAT_DENSITIES)
            && same4(&self.start_amounts, &START_AMOUNTS)
            && self.sei_plus_anode_lithium.to_bits() == SEI_PLUS_ANODE_LITHIUM.to_bits()
            && self.sei_thickness_reference.to_bits() == SEI_THICKNESS_REFERENCE.to_bits()
            && self.volume_m3.to_bits() == CELL_VOLUME_M3.to_bits()
            && self.heat_capacity_j_per_k.to_bits() == CELL_HEAT_CAPACITY_J_PER_K.to_bits()
            && self.convective_conductance_w_per_k.to_bits() == CONVECTIVE_CONDUCTANCE_W_PER_K.to_bits()
            && self.radiative_conductance_w_per_k4.to_bits() == RADIATIVE_CONDUCTANCE_W_PER_K4.to_bits()
            && self.can_melting_k.to_bits() == CAN_MELTING_K.to_bits()
            && self.chemical_heat_j.to_bits() == CELL_CHEMICAL_HEAT_J.to_bits()
            && self.gate_over_convective_time.to_bits() == GATE_OVER_CONVECTIVE_TIME.to_bits()
            && self.gate_over_floor.to_bits() == GATE_OVER_FLOOR.to_bits()
            && self.residual_conservation_limit.to_bits() == RESIDUAL_CONSERVATION_LIMIT.to_bits()
            && self.consumed_heat_fraction.to_bits() == CONSUMED_HEAT_FRACTION.to_bits()
            && same4(&self.amount_limits, &AMOUNT_LIMITS)
            && self.cathode_converted_least.to_bits() == CATHODE_CONVERTED_LEAST.to_bits()
    }

    /// `C / (h A)`, s: the time in which convection alone would bring the
    /// cell 1 − 1/e of the way to the oven's temperature, 1180.5 s.
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

    /// The rate constant of stage `stage` at `temperature_k`,
    /// `A exp(−E / R T)`, 1/s.
    #[must_use]
    pub fn rate_constant_per_s(&self, stage: usize, temperature_k: f64) -> f64 {
        self.frequency_factor_per_s[stage]
            * crate::math::exp(-self.activation_j_per_mol[stage] / (GAS_CONSTANT_J_PER_MOL_K * temperature_k))
    }

    /// Heat stage `stage` still holds at `amount`, J.
    #[must_use]
    pub fn remaining_heat_j(&self, stage: usize, amount: f64) -> f64 {
        self.heat_density_j_per_m3[stage] * self.volume_m3 * amount
    }

    /// Heat the four stages still hold at `amounts`, J.
    #[must_use]
    pub fn held_heat_j(&self, amounts: &[f64; 4]) -> f64 {
        let mut sum = 0.0;
        let mut stage = 0;
        while stage < 4 {
            sum += self.remaining_heat_j(stage, amounts[stage]);
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

    /// Heat the cell loses to an oven at `oven_k`, W, by convection and
    /// radiation; zero when adiabatic.
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
    /// Lithium in the graphite, `c_ne`, 0 to 0.783; a fresh cell 0.75.
    pub anode_lithium_fraction: f64,
    /// Unreacted part of the cathode, `1 − α`, 0 to 1; a fresh cell 0.96.
    pub cathode_unreacted_fraction: f64,
    /// Electrolyte, `c_e`, 0 to 1.
    pub electrolyte_fraction: f64,
    /// SEI thickness, `z` = 0.783 − `c_ne`.
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
    /// Sum of the four.
    pub heat_released_j: f64,
    /// Heat the heater put in since the start.
    pub heater_heat_j: f64,
    /// Heat lost to the oven since the start; negative while the oven warms
    /// the cell.
    pub heat_lost_j: f64,
    /// `C (T − T_start)`.
    pub heat_stored_j: f64,
    /// Chemical heat release of the four stages now, W.
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
}

impl RunawayState {
    /// A cell at `temperature_k` with the four amounts given, at time 0 with
    /// the heat books empty. A fresh cell is 0.15, 0.75, 0.96 and 1; each
    /// amount may lie anywhere from 0 to its limit in [`AMOUNT_LIMITS`].
    pub fn try_start(
        cell: &Runaway18650Lco,
        temperature_k: f64,
        sei_fraction: f64,
        anode_lithium_fraction: f64,
        cathode_unreacted_fraction: f64,
        electrolyte_fraction: f64,
    ) -> Result<Self, StepReject> {
        if !(temperature_k.is_finite()
            && sei_fraction.is_finite()
            && anode_lithium_fraction.is_finite()
            && cathode_unreacted_fraction.is_finite()
            && electrolyte_fraction.is_finite())
        {
            return Err(StepReject::NonFiniteInput);
        }
        if !cell.is_valid() {
            return Err(StepReject::InvalidCell);
        }
        let mut state = Self {
            t_s: 0.0,
            temperature_k,
            sei_fraction,
            anode_lithium_fraction,
            cathode_unreacted_fraction,
            electrolyte_fraction,
            sei_thickness: 0.0,
            start_temperature_k: temperature_k,
            heat_sei_j: 0.0,
            heat_anode_j: 0.0,
            heat_cathode_j: 0.0,
            heat_electrolyte_j: 0.0,
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
        };
        bounds(cell, &state)?;
        refresh(cell, &mut state);
        if !state_is_finite(&state) {
            return Err(StepReject::NonFiniteState);
        }
        Ok(state)
    }

    /// A fresh cell at `temperature_k`.
    pub fn try_fresh(cell: &Runaway18650Lco, temperature_k: f64) -> Result<Self, StepReject> {
        Self::try_start(
            cell,
            temperature_k,
            cell.start_amounts[0],
            cell.start_amounts[1],
            cell.start_amounts[2],
            cell.start_amounts[3],
        )
    }

    fn amounts(&self) -> [f64; 4] {
        [
            self.sei_fraction,
            self.anode_lithium_fraction,
            self.cathode_unreacted_fraction,
            self.electrolyte_fraction,
        ]
    }
}

/// The temperature, the logarithms of the live amounts and the heat lost, at
/// one point of a substep.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    temperature_k: f64,
    logs: [f64; 4],
}

/// The rates at one point.
#[derive(Clone, Copy, Debug)]
struct Rates {
    temperature_k_s: f64,
    logs_per_s: [f64; 4],
    loss_w: f64,
    chemical_w: f64,
    /// The stiffness of each stage in its logarithm, 1/s.
    stiffness_per_s: [f64; 4],
}

/// The rates of the four stages and of the temperature at `point`. A stage
/// that is not live has no rate.
fn rates(cell: &Runaway18650Lco, point: Point, live: [bool; 4], load: &RunawayLoad) -> Rates {
    let temperature_k = point.temperature_k;
    let mut logs_per_s = [0.0; 4];
    let mut stiffness_per_s = [0.0; 4];
    let mut chemical_w = 0.0;
    let mut stage = 0;
    while stage < 4 {
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
            chemical_w += cell.heat_density_j_per_m3[stage] * cell.volume_m3 * rate * amount;
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
fn rk4(cell: &Runaway18650Lco, point: Point, live: [bool; 4], load: &RunawayLoad, h: f64, first: &Rates) -> (Point, f64) {
    let shift = |by: &Rates, fraction: f64| {
        let mut logs = point.logs;
        let mut stage = 0;
        while stage < 4 {
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
    while stage < 4 {
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
    amounts: [f64; 4],
    released_j: [f64; 4],
    heater_j: f64,
    lost_j: f64,
    substeps: u32,
}

/// One substep, as the step takes it: the landing point, the heat lost and
/// the amounts after it.
type Substep = fn(&Runaway18650Lco, Point, [bool; 4], &RunawayLoad, f64, &Rates) -> (Point, f64);

/// The substeps of one step, each sized from the fastest live reaction and
/// the temperature's rate.
fn integrate(
    cell: &Runaway18650Lco,
    state: &RunawayState,
    load: &RunawayLoad,
    dt_s: f64,
    substep: Substep,
    sized: bool,
) -> Result<Trial, StepReject> {
    let mut temperature_k = state.temperature_k;
    let mut amounts = state.amounts();
    let mut released_j = [0.0; 4];
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
        let mut live = [false; 4];
        let mut logs = [0.0; 4];
        let mut stage = 0;
        while stage < 4 {
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
            let warming = first.temperature_k_s.abs();
            if warming * h > SUBSTEP_TEMPERATURE_CHANGE_K {
                h = SUBSTEP_TEMPERATURE_CHANGE_K / warming;
            }
            let mut stage = 0;
            while stage < 4 {
                if live[stage]
                    && cell.remaining_heat_j(stage, amounts[stage]) > spent_j
                    && first.stiffness_per_s[stage] * h > SUBSTEP_LOG_CHANGE
                {
                    h = SUBSTEP_LOG_CHANGE / first.stiffness_per_s[stage];
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
        while stage < 4 {
            if live[stage] {
                let change = landed.logs[stage] - logs[stage];
                if change.is_nan() {
                    return Err(StepReject::NonFiniteState);
                }
                // `c (exp(Δ) − 1)` is the amount's change; at least `−c`.
                let delta = amounts[stage] * crate::math::exp_m1(change);
                released_j[stage] -= cell.remaining_heat_j(stage, delta);
                // `|c · e|` rounds to at most `c` for `e` ≥ −1, so the sum is
                // never below zero.
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
            so_far.heat_sei_j += released_j[0];
            so_far.heat_anode_j += released_j[1];
            so_far.heat_cathode_j += released_j[2];
            so_far.heat_electrolyte_j += released_j[3];
            so_far.heater_heat_j += load.heater_power_w * elapsed_s;
            so_far.heat_lost_j += lost_j;
            so_far.heat_released_j = so_far.heat_sei_j + so_far.heat_anode_j + so_far.heat_cathode_j + so_far.heat_electrolyte_j;
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

/// Advance one step of `dt_s` and write the frame.
///
/// A rejected step leaves `state` and `frame` as they were.
///
/// A value of the landed state below the normal range is written as zero
/// with its sign ([`crate::normal`]).
#[must_use]
pub fn step(
    cell: &Runaway18650Lco,
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
    cell: &Runaway18650Lco,
    state: &mut RunawayState,
    frame: &mut LastStateFrame64,
    load: RunawayLoad,
    dt_s: f64,
) -> Result<(), StepReject> {
    validate(cell, state, &load, dt_s)?;
    let trial = integrate(cell, state, &load, dt_s, rk4, true)?;
    commit(cell, state, frame, dt_s, trial)
}

/// Each value of a landed state below the normal range written as zero
/// with its sign ([`crate::normal`]): the state carried to the next step
/// and published holds no subnormal number.
fn normal_or_zero_state(state: &mut RunawayState) {
    state.t_s = normal_or_zero(state.t_s);
    state.temperature_k = normal_or_zero(state.temperature_k);
    state.sei_fraction = normal_or_zero(state.sei_fraction);
    state.anode_lithium_fraction = normal_or_zero(state.anode_lithium_fraction);
    state.cathode_unreacted_fraction = normal_or_zero(state.cathode_unreacted_fraction);
    state.electrolyte_fraction = normal_or_zero(state.electrolyte_fraction);
    state.sei_thickness = normal_or_zero(state.sei_thickness);
    state.start_temperature_k = normal_or_zero(state.start_temperature_k);
    state.heat_sei_j = normal_or_zero(state.heat_sei_j);
    state.heat_anode_j = normal_or_zero(state.heat_anode_j);
    state.heat_cathode_j = normal_or_zero(state.heat_cathode_j);
    state.heat_electrolyte_j = normal_or_zero(state.heat_electrolyte_j);
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

fn validate(cell: &Runaway18650Lco, state: &RunawayState, load: &RunawayLoad, dt_s: f64) -> Result<(), StepReject> {
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

/// The physical bounds of the temperature and the four amounts.
fn bounds(cell: &Runaway18650Lco, state: &RunawayState) -> Result<(), StepReject> {
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
    ];
    let mut stage = 0;
    while stage < 4 {
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
fn close_ledger(cell: &Runaway18650Lco, state: &RunawayState) -> Result<(), StepReject> {
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
    cell: &Runaway18650Lco,
    state: &mut RunawayState,
    frame: &mut LastStateFrame64,
    dt_s: f64,
    trial: Trial,
) -> Result<(), StepReject> {
    let saved = *state;
    state.t_s += dt_s;
    state.temperature_k = trial.temperature_k;
    state.sei_fraction = trial.amounts[0];
    state.anode_lithium_fraction = trial.amounts[1];
    state.cathode_unreacted_fraction = trial.amounts[2];
    state.electrolyte_fraction = trial.amounts[3];
    state.heat_sei_j += trial.released_j[0];
    state.heat_anode_j += trial.released_j[1];
    state.heat_cathode_j += trial.released_j[2];
    state.heat_electrolyte_j += trial.released_j[3];
    state.heater_heat_j += trial.heater_j;
    state.heat_lost_j += trial.lost_j;
    state.substeps = trial.substeps as f64;
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
fn refresh(cell: &Runaway18650Lco, state: &mut RunawayState) {
    state.sei_thickness = cell.sei_plus_anode_lithium - state.anode_lithium_fraction;
    state.heat_released_j = state.heat_sei_j + state.heat_anode_j + state.heat_cathode_j + state.heat_electrolyte_j;
    state.heat_stored_j = cell.heat_capacity_j_per_k * (state.temperature_k - state.start_temperature_k);
    state.energy_defect_j = state.heat_stored_j - (state.heat_released_j + state.heater_heat_j - state.heat_lost_j);
    state.residual_conservation_error = state.energy_defect_j.abs() / cell.chemical_heat_j;
    let amounts = state.amounts();
    let mut live = [false; 4];
    let mut logs = [0.0; 4];
    let mut stage = 0;
    while stage < 4 {
        if amounts[stage] > 0.0 && state.temperature_k > 0.0 {
            live[stage] = true;
            logs[stage] = crate::math::log(amounts[stage]);
        }
        stage += 1;
    }
    // The chemical heat does not depend on the oven or the heater.
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
        | (if state.runaway { FLAG_RUNAWAY } else { 0 });
}

fn state_is_finite(state: &RunawayState) -> bool {
    state.t_s.is_finite()
        && state.temperature_k.is_finite()
        && state.sei_fraction.is_finite()
        && state.anode_lithium_fraction.is_finite()
        && state.cathode_unreacted_fraction.is_finite()
        && state.electrolyte_fraction.is_finite()
        && state.sei_thickness.is_finite()
        && state.start_temperature_k.is_finite()
        && state.heat_sei_j.is_finite()
        && state.heat_anode_j.is_finite()
        && state.heat_cathode_j.is_finite()
        && state.heat_electrolyte_j.is_finite()
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


#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::*;

    const PROOF: [u8; 16] = [0xab; 16];

    fn blank() -> LastStateFrame64 {
        LastStateFrame64 {
            t: -1.0,
            pos: [-1.0; 3],
            vel: [-1.0; 3],
            force_torque: -1.0,
            residual: -1.0,
            flags: 0xdead_beef,
            proof: PROOF,
        }
    }

    fn oven(oven_temperature_k: f64) -> RunawayLoad {
        RunawayLoad {
            oven_temperature_k,
            heater_power_w: 0.0,
            adiabatic: 0.0,
        }
    }

    fn calorimeter(heater_power_w: f64) -> RunawayLoad {
        RunawayLoad {
            oven_temperature_k: 298.15,
            heater_power_w,
            adiabatic: 1.0,
        }
    }

    fn assert_copy<T: Copy>() {}

    fn relative(got: f64, expected: f64) -> f64 {
        (got - expected).abs() / expected.abs()
    }

    fn bits(state: &RunawayState) -> [u64; 24] {
        [
            state.t_s.to_bits(),
            state.temperature_k.to_bits(),
            state.sei_fraction.to_bits(),
            state.anode_lithium_fraction.to_bits(),
            state.cathode_unreacted_fraction.to_bits(),
            state.electrolyte_fraction.to_bits(),
            state.sei_thickness.to_bits(),
            state.start_temperature_k.to_bits(),
            state.heat_sei_j.to_bits(),
            state.heat_anode_j.to_bits(),
            state.heat_cathode_j.to_bits(),
            state.heat_electrolyte_j.to_bits(),
            state.heat_released_j.to_bits(),
            state.heater_heat_j.to_bits(),
            state.heat_lost_j.to_bits(),
            state.heat_stored_j.to_bits(),
            state.chemical_heat_w.to_bits(),
            state.chemical_heating_k_s.to_bits(),
            state.energy_defect_j.to_bits(),
            state.residual_conservation_error.to_bits(),
            state.substeps.to_bits(),
            state.self_heating as u64,
            state.runaway as u64,
            0,
        ]
    }

    fn fresh(temperature_k: f64) -> (Runaway18650Lco, RunawayState) {
        let cell = Runaway18650Lco::hatchard_kim();
        let state = RunawayState::try_fresh(&cell, temperature_k).unwrap();
        (cell, state)
    }

    /// The temperature of the run at each of `times`, stepping at `dt`, and
    /// the highest temperature with its time.
    fn oven_run(oven_k: f64, dt: f64, until_s: f64, times: &[f64]) -> (std::vec::Vec<f64>, (f64, f64), f64, RunawayState) {
        let (cell, mut state) = fresh(298.15);
        let mut frame = blank();
        let mut at = std::vec::Vec::new();
        let mut peak = (0.0, 0.0);
        let mut worst: f64 = 0.0;
        while state.t_s < until_s {
            step(&cell, &mut state, &mut frame, oven(oven_k), dt).unwrap();
            worst = worst.max(state.residual_conservation_error);
            if state.temperature_k > peak.0 {
                peak = (state.temperature_k, state.t_s);
            }
            for &t in times {
                if state.t_s == t {
                    at.push(state.temperature_k);
                }
            }
        }
        (at, peak, worst, state)
    }

    #[test]
    fn the_card_locks_its_constants() {
        let cell = Runaway18650Lco::hatchard_kim();
        assert!(cell.is_valid());
        assert_eq!(cell.frequency_factor_per_s, [1.667e15, 2.5e13, 6.667e13, 5.14e25]);
        assert_eq!(cell.activation_j_per_mol, [1.3508e5, 1.3508e5, 1.396e5, 2.74e5]);
        assert_eq!(cell.start_amounts, [0.15, 0.75, 0.96, 1.0]);
        assert_eq!(cell.residual_conservation_limit, 1.0e-6);
        assert_eq!(cell.consumed_heat_fraction, 1.0e-6);
        assert_eq!(cell.amount_limits, [1.0, 0.783, 1.0, 1.0]);
        let mut frame = blank();
        let held = frame.to_bytes();
        for index in 0..18 {
            let mut other = cell;
            match index {
                15 => other.consumed_heat_fraction = 1.0e-3,
                16 => other.amount_limits[1] = 0.75,
                17 => other.cathode_converted_least = 0.0,
                0 => other.frequency_factor_per_s[3] *= 1.000001,
                1 => other.activation_j_per_mol[0] = 1.35e5,
                2 => other.heat_density_j_per_m3[1] *= 0.5,
                3 => other.start_amounts[2] = 1.0,
                4 => other.sei_plus_anode_lithium = 0.8,
                5 => other.sei_thickness_reference = 0.05,
                6 => other.volume_m3 *= 2.0,
                7 => other.heat_capacity_j_per_k = 40.0,
                8 => other.convective_conductance_w_per_k = 0.0,
                9 => other.radiative_conductance_w_per_k4 = 0.0,
                10 => other.can_melting_k = 2000.0,
                11 => other.chemical_heat_j = 1.0,
                12 => other.gate_over_convective_time = 1.0,
                13 => other.gate_over_floor = 2.0,
                _ => other.residual_conservation_limit = 1.0,
            }
            assert!(!other.is_valid(), "{index}");
            assert_eq!(RunawayState::try_fresh(&other, 300.0), Err(StepReject::InvalidCell));
            let (_, mut state) = fresh(300.0);
            let before = bits(&state);
            assert_eq!(step(&other, &mut state, &mut frame, oven(400.0), 1.0), Err(StepReject::InvalidCell));
            assert_eq!(bits(&state), before);
            assert_eq!(frame.to_bytes(), held);
        }
    }

    /// The derived constants follow from the published ones.
    #[test]
    fn the_derived_constants_follow_from_the_published_ones() {
        let cell = Runaway18650Lco::hatchard_kim();
        assert!(relative(CELL_VOLUME_M3, 1.6540485321150e-5) < 1.0e-12, "{CELL_VOLUME_M3:e}");
        assert!(relative(CELL_SURFACE_M2, 4.1846014145816e-3) < 1.0e-12, "{CELL_SURFACE_M2:e}");
        assert!(relative(CELL_HEAT_CAPACITY_J_PER_K, 35.419795266711) < 1.0e-12);
        assert_eq!(SEI_PLUS_ANODE_LITHIUM, 0.783);
        // Each stage's heat for a fresh cell: 389.2, 12,978.8, 6,087.9 and 1,043.2 J.
        let stage = [389.212837, 12978.809385, 6087.861916, 1043.200139];
        let mut sum = 0.0;
        for index in 0..4 {
            let heat = cell.remaining_heat_j(index, START_AMOUNTS[index]);
            assert!((heat - stage[index]).abs() < 1.0e-3, "{index} {heat}");
            sum += heat;
        }
        assert!(relative(sum, CELL_CHEMICAL_HEAT_J) < 1.0e-14);
        assert!(relative(CELL_CHEMICAL_HEAT_J, 20499.084276422564) < 1.0e-12);
        // The adiabatic rise of a fresh cell that releases all of it: 578.8 K.
        assert!((CELL_CHEMICAL_HEAT_J / CELL_HEAT_CAPACITY_J_PER_K - 578.75).abs() < 0.01);
        assert!(relative(cell.convective_time_s(), 1180.5184892005) < 1.0e-12);
        assert!(relative(cell.gate_dt_s().unwrap(), 11.805184892005) < 1.0e-12);
        assert_eq!(cell.floor_dt_s().unwrap(), cell.gate_dt_s().unwrap() / 1_048_576.0);
        // The rate constants at 155 degC.
        assert!(relative(cell.rate_constant_per_s(0, 428.15), 5.514081e-2) < 1.0e-6);
        assert!(relative(cell.rate_constant_per_s(3, 428.15), 1.912222e-8) < 1.0e-6);
    }

    #[test]
    fn types_are_copy() {
        assert_copy::<Runaway18650Lco>();
        assert_copy::<RunawayState>();
        assert_copy::<RunawayLoad>();
        assert_copy::<StepReject>();
    }

    /// A fresh cell put in an oven at 155 degC from 25 degC, at 1 s, against
    /// an independent integration of the same equations outside this crate
    /// (scipy LSODA, relative tolerance 1e-11, the platform's exp:
    /// evidence/research/thermal_runaway_18650/analysis/reference_lsoda.py and
    /// card_oven_428.15_from_298.15.json). The card runs away near 1,417 s and
    /// peaks at 796.5 K at 1,519 s; a quarter of the anode's lithium is left.
    #[test]
    fn an_oven_at_155_c_runs_away_as_an_independent_integration_does() {
        let times = [600.0, 1200.0, 1400.0, 1500.0, 2400.0];
        let (at, peak, worst, state) = oven_run(428.15, 1.0, 2400.0, &times);
        let reference = [394.03338031855924, 440.5564259924509, 451.9229883601563, 476.1609802429484, 448.9598386646654];
        for index in 0..5 {
            let tolerance = if index == 3 { 0.05 } else { 0.002 };
            assert!((at[index] - reference[index]).abs() < tolerance, "{} {} {}", times[index], at[index], reference[index]);
        }
        assert!((peak.0 - 796.5054215719847).abs() < 0.2, "{peak:?}");
        assert!((peak.1 - 1519.35).abs() <= 1.0, "{peak:?}");
        assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
        assert!((state.anode_lithium_fraction - 0.2518234313487455).abs() < 2.0e-5);
        assert_eq!([state.sei_fraction, state.cathode_unreacted_fraction, state.electrolyte_fraction], [0.0; 3]);
        assert!(!state.runaway);
    }

    /// At 145 degC the same cell heats past its oven by 49 K and comes back
    /// down: no runaway in the sense of the paper, but the 10 K per minute
    /// flag is reached at 452.6 K. The independent integration
    /// (card_oven_418.15_from_298.15.json) peaks at 467.087 K at 7,945 s.
    #[test]
    fn an_oven_at_145_c_peaks_at_194_c_and_cools() {
        let (at, peak, worst, state) = oven_run(418.15, 5.0, 9000.0, &[3000.0, 7000.0, 8400.0]);
        let reference = [424.97622368553823, 432.5802797088931, 437.6454691083099];
        for index in 0..3 {
            assert!((at[index] - reference[index]).abs() < 0.01, "{index} {} {}", at[index], reference[index]);
        }
        assert!((peak.0 - 467.087384504695).abs() < 0.05, "{peak:?}");
        assert!((peak.1 - 7945.0).abs() <= 5.0, "{peak:?}");
        assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT);
        assert!(state.anode_lithium_fraction > 0.59);
        assert!(state.electrolyte_fraction > 0.99);
    }

    /// The answer does not follow the step a caller asks for: steps of 1 s,
    /// 5 s and the ceiling land on the same temperature at 1,500 s, inside
    /// the runaway, within 0.1 K, and on the same anode lithium after it.
    #[test]
    fn the_runaway_does_not_follow_the_step_asked_for() {
        let cell = Runaway18650Lco::hatchard_kim();
        let gate = cell.gate_dt_s().unwrap();
        let mut ends = std::vec::Vec::new();
        for dt in [1.0, 5.0, gate] {
            let (cell, mut state) = fresh(298.15);
            let mut frame = blank();
            let steps = (3000.0 / dt) as usize;
            let mut most: f64 = 0.0;
            for _ in 0..steps {
                step(&cell, &mut state, &mut frame, oven(428.15), dt).unwrap();
                most = most.max(state.substeps);
            }
            assert!(most < 2000.0, "{dt} {most}");
            ends.push((state.anode_lithium_fraction, state.heat_released_j));
        }
        for end in &ends[1..] {
            assert!((end.0 - ends[0].0).abs() < 1.0e-6, "{end:?} {:?}", ends[0]);
            assert!((end.1 - ends[0].1).abs() < 0.05, "{end:?} {:?}", ends[0]);
        }
    }

    /// The flags appear in order on the way to the runaway: self-heating near
    /// 356 K (83 degC), runaway near 453.5 K. The independent integration
    /// crosses 0.02 K per minute at 356.13 K and 10 K per minute at 453.48 K.
    #[test]
    fn the_flags_mark_self_heating_and_runaway_and_land() {
        let (cell, mut state) = fresh(298.15);
        let mut frame = blank();
        let mut first_self = None;
        let mut first_runaway = None;
        while state.t_s < 1600.0 {
            step(&cell, &mut state, &mut frame, oven(428.15), 0.5).unwrap();
            assert_eq!(frame.flags & !(FLAG_SELF_HEATING | FLAG_RUNAWAY), 0);
            assert_eq!(frame.flags & FLAG_SELF_HEATING != 0, state.self_heating);
            assert_eq!(frame.flags & FLAG_RUNAWAY != 0, state.runaway);
            if state.runaway {
                assert!(state.self_heating);
            }
            if first_self.is_none() && state.self_heating {
                first_self = Some(state.temperature_k);
            }
            if first_runaway.is_none() && state.runaway {
                first_runaway = Some((state.t_s, state.temperature_k));
            }
            assert_eq!(frame.proof, PROOF);
        }
        let first_self = first_self.unwrap();
        let (time, temperature) = first_runaway.unwrap();
        assert!((first_self - 356.126).abs() < 0.2, "{first_self}");
        assert!((temperature - 453.483).abs() < 0.2, "{temperature}");
        assert!((time - 1417.25).abs() <= 1.0, "{time}");
        assert_eq!(FLAG_SELF_HEATING, 1);
        assert_eq!(FLAG_RUNAWAY, 2);
    }

    /// In a calorimeter that tracks the cell, a fresh cell started at 150 degC
    /// releases all but the anode's slowed lithium and the ledger closes
    /// over the whole release: the rise is the heat over the heat capacity.
    #[test]
    fn an_adiabatic_cell_rises_by_its_heat_over_its_heat_capacity() {
        let (cell, mut state) = fresh(423.15);
        let mut frame = blank();
        let mut worst: f64 = 0.0;
        let mut steps = 0;
        while steps < 600 {
            step(&cell, &mut state, &mut frame, calorimeter(0.0), 10.0).unwrap();
            worst = worst.max(state.residual_conservation_error);
            steps += 1;
        }
        assert_eq!(state.heat_lost_j, 0.0);
        let rise = state.temperature_k - 423.15;
        assert!(relative(rise * CELL_HEAT_CAPACITY_J_PER_K, state.heat_released_j) < 1.0e-6);
        assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT, "{worst:e}");
        assert!(state.temperature_k > 900.0, "{}", state.temperature_k);
        let anode_left = state.anode_lithium_fraction;
        assert!(anode_left > 0.0 && anode_left < 0.3, "{anode_left}");
        assert_eq!(state.heat_sei_j + state.heat_anode_j + state.heat_cathode_j + state.heat_electrolyte_j, state.heat_released_j);
    }

    /// A heater alone, with no reaction left, warms the cell by its power
    /// over the heat capacity, and the ledger books the heater's heat.
    #[test]
    fn a_heater_warms_a_spent_cell_by_its_power_over_its_heat_capacity() {
        let cell = Runaway18650Lco::hatchard_kim();
        let mut warmed = std::vec::Vec::new();
        // with nothing left at all, and with 1e-300 of the electrolyte: the
        // same body, the same heater, the same temperature to the bit
        for electrolyte in [0.0, 1.0e-300] {
            let mut state = RunawayState::try_start(&cell, 300.0, 0.0, 0.0, 0.0, electrolyte).unwrap();
            let mut frame = blank();
            for _ in 0..10 {
                step(&cell, &mut state, &mut frame, calorimeter(35.41979526671116), 1.0).unwrap();
            }
            assert!((state.temperature_k - 310.0).abs() < 1.0e-9, "{}", state.temperature_k);
            assert!(relative(state.heater_heat_j, 354.1979526671116) < 1.0e-14);
            assert!(state.residual_conservation_error < 1.0e-12);
            warmed.push(state.temperature_k.to_bits());
        }
        assert_eq!(warmed[0], warmed[1]);
    }

    /// A cell that starts with no more than the consumed heat, 0.0205 J, is
    /// spent: it is stepped as a body that gains and loses heat and meets no
    /// end, in an oven, under a heater, and held at 1,700 K with one ulp of
    /// subnormal anode lithium, at the ceiling and at 1 s alike (where 0.12.0
    /// as frozen ended the ceiling's run and not the 1 s run).
    #[test]
    fn a_spent_cell_is_stepped_and_meets_no_end() {
        let cell = Runaway18650Lco::hatchard_kim();
        assert!((cell.consumed_heat_j() - 0.020499084276422564).abs() < 1.0e-15);
        let mut spent = RunawayState::try_start(&cell, 500.0, 0.0, 0.0, 0.0, 0.0).unwrap();
        let mut frame = blank();
        for _ in 0..100 {
            step(&cell, &mut spent, &mut frame, oven(300.0), 10.0).unwrap();
        }
        assert!(spent.temperature_k < 500.0 && spent.temperature_k > 300.0);
        assert!(relative(spent.heat_stored_j, -spent.heat_lost_j) < 1.0e-9);
        assert_eq!(spent.heat_released_j, 0.0);
        // just under the consumed heat in the electrolyte: spent too
        let just = 0.99 * cell.consumed_heat_j() / cell.remaining_heat_j(3, 1.0);
        let mut nearly = RunawayState::try_start(&cell, 900.0, 0.0, 0.0, 0.0, just).unwrap();
        step(&cell, &mut nearly, &mut frame, calorimeter(0.0), 1.0).unwrap();
        assert!(nearly.electrolyte_fraction < just);
        for dt in [cell.gate_dt_s().unwrap(), 1.0] {
            let mut held = RunawayState::try_start(&cell, 1700.0, 0.0, f64::MIN_POSITIVE, 0.0, 0.0).unwrap();
            for _ in 0..3000 {
                step(&cell, &mut held, &mut frame, oven(1700.0), dt).unwrap();
            }
            assert!(held.residual_conservation_error < 1.0e-12);
        }
    }

    /// The end is the event, not the step: a cell with a part of its
    /// electrolyte left, at 560 K in a calorimeter (1.04 J held, burning at
    /// 1.4 per second), meets it at the same time at every step asked for,
    /// each within its own length, the state and the frame kept. A cell held
    /// at 1,200 K by an oven after the 155 degC oven run, a quarter of its
    /// anode lithium left, meets it after 1.63 hours at 1 s and at the
    /// ceiling; at 900 K after 6.2 days.
    #[test]
    fn every_reactant_consumed_is_met_at_the_same_time_at_every_step() {
        let cell = Runaway18650Lco::hatchard_kim();
        let mut stops = std::vec::Vec::new();
        for dt in [0.01, 0.1, 0.5, 1.0, cell.gate_dt_s().unwrap()] {
            let mut state = RunawayState::try_start(&cell, 560.0, 0.0, 0.0, 0.0, 1.0e-3).unwrap();
            let mut frame = blank();
            let mut worst: f64 = 0.0;
            let reject = loop {
                let kept = bits(&state);
                let held = frame.to_bytes();
                match step(&cell, &mut state, &mut frame, calorimeter(0.0), dt) {
                    Ok(()) => worst = worst.max(state.residual_conservation_error),
                    Err(reject) => {
                        assert_eq!(bits(&state), kept);
                        assert_eq!(frame.to_bytes(), held);
                        break reject;
                    }
                }
                assert!(state.t_s < 100.0);
            };
            assert_eq!(reject, StepReject::EveryReactantConsumed, "{dt}");
            assert!(cell.held_heat_j(&state.amounts()) > cell.consumed_heat_j());
            assert!(worst < 1.0e-9, "{worst:e}");
            stops.push((dt, state.t_s));
        }
        let (fine_dt, fine_t) = stops[0];
        // ln(1.0432 / 0.0205) / 1.41 per second, a little sooner as the cell warms
        assert!(fine_t > 2.7 && fine_t < 2.9, "{fine_t}");
        for (dt, t) in &stops {
            assert!(*t < fine_t + fine_dt && t + dt > fine_t, "{dt}: {t} against {fine_t}");
        }
        let after = |temperature_k: f64, dt: f64| {
            let mut state = RunawayState::try_start(&cell, temperature_k, 0.0, 0.2518238636363292, 0.0, 0.0).unwrap();
            let mut frame = blank();
            let load = oven(temperature_k);
            loop {
                match step(&cell, &mut state, &mut frame, load, dt) {
                    Ok(()) => assert!(state.residual_conservation_error < 1.0e-7),
                    Err(reject) => {
                        assert_eq!(reject, StepReject::EveryReactantConsumed);
                        break state.t_s;
                    }
                }
            }
        };
        let gate = cell.gate_dt_s().unwrap();
        let (coarse, fine) = (after(1200.0, gate), after(1200.0, 1.0));
        assert!(coarse <= fine && fine < coarse + gate, "{coarse} {fine}");
        assert!((fine / 3600.0 - 1.63).abs() < 0.005, "{fine}");
        let days = after(900.0, gate) / 86_400.0;
        assert!((days - 6.24).abs() < 0.01, "{days}");
    }

    /// Rule 10 at the end: explicit Euler with the card's own substeps, on a
    /// cell whose electrolyte burns out at 900 K, is named by the ledger,
    /// closed at the substep the end falls in, and does not pass for the end;
    /// the card's own step there is the end.
    #[test]
    fn a_wrong_stepping_method_at_the_end_is_named_by_the_ledger_and_not_passed_for_the_end() {
        let cell = Runaway18650Lco::hatchard_kim();
        let start = RunawayState::try_start(&cell, 900.0, 0.0, 0.0, 0.0, 1.0).unwrap();
        let mut frame = blank();
        let held = frame.to_bytes();
        let mut state = start;
        let got = step_with(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0, euler, true);
        assert!(got == Err(StepReject::SpontaneousGeneration) || got == Err(StepReject::FirstLawBreach), "{got:?}");
        assert_eq!(bits(&state), bits(&start));
        assert_eq!(frame.to_bytes(), held);
        assert_eq!(step(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0), Err(StepReject::EveryReactantConsumed));
        assert_eq!(bits(&state), bits(&start));
        // a trial forged at the end: its booked heat halved is named, not ended
        let honest = integrate(&cell, &start, &calorimeter(0.0), 1.0e-11, rk4, true).unwrap();
        let mut forged = honest;
        forged.released_j[3] *= 0.5;
        let mut state = start;
        assert_eq!(commit(&cell, &mut state, &mut frame, 1.0e-11, forged), Err(StepReject::SpontaneousGeneration));
        assert_eq!(frame.to_bytes(), held);
    }

    /// Past the fresh cell's amounts, where 0.12.0 as frozen refused: a cell
    /// with all of its SEI lithium, its graphite at 0.783 (no SEI thickness)
    /// and its cathode unconverted, and one between, in a calorimeter from
    /// 150 degC. The ledger closes over the whole release, the rise is the
    /// heat over the heat capacity, and a cathode with no converted part
    /// never starts to convert, as its autocatalytic rate says.
    #[test]
    fn past_the_fresh_cells_amounts_the_equations_hold_and_the_ledger_closes() {
        let cell = Runaway18650Lco::hatchard_kim();
        let least = 1.0 - CATHODE_CONVERTED_LEAST;
        assert_eq!(1.0 - least, CATHODE_CONVERTED_LEAST);
        for amounts in [[1.0, 0.783, 1.0, 1.0], [0.5, 0.78, 0.99, 1.0], [0.15000000000000002, 0.7500000000000001, 0.9600000000000001, 1.0], [0.15, 0.75, least, 1.0]] {
            let mut state = RunawayState::try_start(&cell, 423.15, amounts[0], amounts[1], amounts[2], amounts[3]).unwrap();
            let held_at_start = cell.held_heat_j(&state.amounts());
            let mut frame = blank();
            let mut worst: f64 = 0.0;
            for _ in 0..600 {
                step(&cell, &mut state, &mut frame, calorimeter(0.0), 10.0).unwrap();
                worst = worst.max(state.residual_conservation_error);
            }
            assert!(worst < 0.1 * RESIDUAL_CONSERVATION_LIMIT, "{amounts:?} {worst:e}");
            let rise = state.temperature_k - 423.15;
            assert!(relative(rise * CELL_HEAT_CAPACITY_J_PER_K, state.heat_released_j) < 1.0e-6, "{amounts:?}");
            // what was released is what the amounts no longer hold
            let released = held_at_start - cell.held_heat_j(&state.amounts());
            assert!((released - state.heat_released_j).abs() < 1.0e-6 * CELL_CHEMICAL_HEAT_J, "{amounts:?}");
            assert!(state.temperature_k > 800.0, "{amounts:?} {}", state.temperature_k);
            if amounts[2] == 1.0 {
                assert_eq!(state.cathode_unreacted_fraction, 1.0);
                assert_eq!(state.heat_cathode_j, 0.0);
            } else {
                assert_eq!(state.cathode_unreacted_fraction, 0.0);
            }
        }
        // just under 1, where the converted part is lost to rounding, the
        // start is refused by name; at 1 and at 1 - 2^-33 it is taken
        for unreacted in [f64::from_bits(1.0_f64.to_bits() - 1), 1.0 - 4.0e-16, f64::from_bits(least.to_bits() + 1)] {
            assert_eq!(
                RunawayState::try_start(&cell, 423.15, 0.15, 0.75, unreacted, 1.0),
                Err(StepReject::CathodeUnreactedOutsideCell),
                "{unreacted}"
            );
        }
        // the converted part's growth from 2^-33 is the autocatalytic one: at
        // a held 600 K its rate is 46.8 per second, and it grows 2^20-fold, to
        // 1.22e-4, in ln(2^20 (1 - a0) / (1 - a1)) / k, 0.2963 s, while the
        // heat it has released, under 1 J, warms the cell by under 0.03 K
        let mut state = RunawayState::try_start(&cell, 600.0, 0.0, 0.0, least, 0.0).unwrap();
        let mut frame = blank();
        let k = cell.rate_constant_per_s(2, 600.0);
        assert!((k - 46.80).abs() < 0.05, "{k}");
        let target = CATHODE_CONVERTED_LEAST * 1_048_576.0;
        while 1.0 - state.cathode_unreacted_fraction < target {
            step(&cell, &mut state, &mut frame, oven(600.0), 1.0e-4).unwrap();
        }
        let expected = (1_048_576.0 * (1.0 - CATHODE_CONVERTED_LEAST) / (1.0 - target)).ln() / k;
        assert!(state.t_s >= expected && state.t_s < expected + 2.0e-4, "{} {expected}", state.t_s);
        assert!(state.temperature_k - 600.0 < 0.03);
    }

    /// With adiabatic 1 the oven temperature does not enter: 0 K, below it
    /// and 1e-300 K land on the same state, to the bit, as 298.15 K. With
    /// adiabatic 0 it enters, and 0 K is refused.
    #[test]
    fn an_oven_temperature_that_does_not_enter_is_not_refused() {
        let (cell, start) = fresh(423.15);
        let mut landed = std::vec::Vec::new();
        for oven_k in [298.15, 0.0, -5.0, 1.0e-300] {
            let mut state = start;
            let mut frame = blank();
            let load = RunawayLoad { oven_temperature_k: oven_k, heater_power_w: 0.0, adiabatic: 1.0 };
            step(&cell, &mut state, &mut frame, load, 10.0).unwrap();
            landed.push((bits(&state), frame.to_bytes()));
        }
        for other in &landed[1..] {
            assert_eq!(other, &landed[0]);
        }
        let mut state = start;
        let mut frame = blank();
        assert_eq!(step(&cell, &mut state, &mut frame, oven(0.0), 10.0), Err(StepReject::OvenTemperatureNotPositive));
        assert_eq!(bits(&state), bits(&start));
    }

    /// The end: the step that would carry the heat held to the consumed heat
    /// is EveryReactantConsumed, with the state and the frame kept. At 900 K
    /// a thousandth of the electrolyte (1.04 J) burns out inside the first
    /// step; a step of 1 ms after 0.5 s at 560 K lands and a later one ends.
    #[test]
    fn every_reactant_consumed_is_the_end() {
        let cell = Runaway18650Lco::hatchard_kim();
        let start = RunawayState::try_start(&cell, 900.0, 0.0, 0.0, 0.0, 1.0e-3).unwrap();
        let mut state = start;
        let mut frame = blank();
        let held = frame.to_bytes();
        assert_eq!(step(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0), Err(StepReject::EveryReactantConsumed));
        assert_eq!(bits(&state), bits(&start));
        assert_eq!(frame.to_bytes(), held);
        let mut state = RunawayState::try_start(&cell, 560.0, 0.0, 0.0, 0.0, 1.0e-3).unwrap();
        for _ in 0..500 {
            step(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0e-3).unwrap();
        }
        assert!(state.electrolyte_fraction < 1.0e-3 && state.electrolyte_fraction > 4.0e-4, "{}", state.electrolyte_fraction);
        let mut steps = 0;
        let reject = loop {
            match step(&cell, &mut state, &mut frame, calorimeter(0.0), 0.5) {
                Ok(()) => steps += 1,
                Err(reject) => break reject,
            }
            assert!(steps < 100);
        };
        assert_eq!(reject, StepReject::EveryReactantConsumed);
        assert!(steps >= 3, "{steps}");
        assert!(cell.held_heat_j(&state.amounts()) > cell.consumed_heat_j());
        assert!(state.residual_conservation_error < 1.0e-9);
    }

    /// The real `validate`, a chosen way of stepping, the real `commit`.
    fn step_with(
        cell: &Runaway18650Lco,
        state: &mut RunawayState,
        frame: &mut LastStateFrame64,
        load: RunawayLoad,
        dt_s: f64,
        substep: Substep,
        sized: bool,
    ) -> Result<(), StepReject> {
        validate(cell, state, &load, dt_s)?;
        let trial = integrate(cell, state, &load, dt_s, substep, sized)?;
        commit(cell, state, frame, dt_s, trial)
    }

    /// Explicit Euler over the same variables.
    fn euler(cell: &Runaway18650Lco, point: Point, live: [bool; 4], load: &RunawayLoad, h: f64, first: &Rates) -> (Point, f64) {
        let _ = (cell, live, load);
        let mut logs = point.logs;
        for stage in 0..4 {
            logs[stage] += h * first.logs_per_s[stage];
        }
        (
            Point {
                temperature_k: point.temperature_k + h * first.temperature_k_s,
                logs,
            },
            h * first.loss_w,
        )
    }

    /// The state of the 155 degC oven run at `t_s`, stepping at 1 s.
    fn oven_state_at(t_s: f64) -> (Runaway18650Lco, RunawayState) {
        let (cell, mut state) = fresh(298.15);
        let mut frame = blank();
        while state.t_s < t_s {
            step(&cell, &mut state, &mut frame, oven(428.15), 1.0).unwrap();
        }
        (cell, state)
    }

    /// Rule 10: a wrong stepping method is named. Explicit Euler with the
    /// card's own substeps, and the card's Runge-Kutta without them, are each
    /// refused by the ledger in the runaway; the card's own step lands.
    #[test]
    fn a_wrong_stepping_method_is_named_by_the_ledger() {
        let (cell, start) = oven_state_at(1490.0);
        let mut frame = blank();
        let held = frame.to_bytes();
        let mut state = start;
        // Euler books more heat into the temperature than the reactants
        // carried: heat from nowhere.
        let got = step_with(&cell, &mut state, &mut frame, oven(428.15), 10.0, euler, true);
        assert_eq!(got, Err(StepReject::SpontaneousGeneration));
        assert_eq!(bits(&state), bits(&start));
        assert_eq!(frame.to_bytes(), held);
        // One Runge-Kutta step of 10 s across the runaway, with no substeps:
        // the reactants consume more than the temperature books.
        let got = step_with(&cell, &mut state, &mut frame, oven(428.15), 10.0, rk4, false);
        assert_eq!(got, Err(StepReject::FirstLawBreach));
        assert_eq!(bits(&state), bits(&start));
        assert_eq!(frame.to_bytes(), held);
        step(&cell, &mut state, &mut frame, oven(428.15), 10.0).unwrap();
        assert!(state.residual_conservation_error < 0.1 * RESIDUAL_CONSERVATION_LIMIT);
        assert_eq!(frame.proof, PROOF);
    }

    /// Rule 10: a falsified ledger is named, both ways. A temperature, a heat
    /// sum or an amount altered between steps, and a trial whose booked heat
    /// is altered, are each refused with the state and the frame kept.
    #[test]
    fn a_falsified_ledger_is_named_both_ways() {
        let (cell, start) = oven_state_at(1450.0);
        let mut frame = blank();
        let held = frame.to_bytes();
        let gate = RESIDUAL_CONSERVATION_LIMIT * CELL_CHEMICAL_HEAT_J;
        let attempt = |alter: &dyn Fn(&mut RunawayState), expected: StepReject, frame: &mut LastStateFrame64| {
            let mut altered = start;
            alter(&mut altered);
            let before = bits(&altered);
            assert_eq!(step(&cell, &mut altered, frame, oven(428.15), 1.0), Err(expected));
            assert_eq!(bits(&altered), before);
        };
        attempt(&|s| s.temperature_k += 0.01, StepReject::SpontaneousGeneration, &mut frame);
        attempt(&|s| s.temperature_k -= 0.01, StepReject::FirstLawBreach, &mut frame);
        attempt(&|s| s.heat_anode_j += 10.0 * gate, StepReject::FirstLawBreach, &mut frame);
        attempt(&|s| s.heat_lost_j -= 10.0 * gate, StepReject::FirstLawBreach, &mut frame);
        attempt(&|s| s.heater_heat_j -= 10.0 * gate, StepReject::SpontaneousGeneration, &mut frame);
        attempt(&|s| s.start_temperature_k += 0.01, StepReject::FirstLawBreach, &mut frame);
        assert_eq!(frame.to_bytes(), held);
        // A trial whose amounts fell without the heat being booked.
        let mut state = start;
        let load = oven(428.15);
        let honest = integrate(&cell, &state, &load, 1.0, rk4, true).unwrap();
        let mut forged = honest;
        forged.released_j[1] *= 0.5;
        assert_eq!(commit(&cell, &mut state, &mut frame, 1.0, forged), Err(StepReject::SpontaneousGeneration));
        let mut forged = honest;
        forged.temperature_k -= 0.01;
        assert_eq!(commit(&cell, &mut state, &mut frame, 1.0, forged), Err(StepReject::FirstLawBreach));
        let mut forged = honest;
        forged.lost_j += 1.0;
        assert_eq!(commit(&cell, &mut state, &mut frame, 1.0, forged), Err(StepReject::SpontaneousGeneration));
        let mut forged = honest;
        forged.amounts[3] = 1.5;
        assert_eq!(commit(&cell, &mut state, &mut frame, 1.0, forged), Err(StepReject::ElectrolyteOutsideCell));
        let mut forged = honest;
        forged.amounts[0] = -1.0e-12;
        assert_eq!(commit(&cell, &mut state, &mut frame, 1.0, forged), Err(StepReject::SeiFractionOutsideCell));
        assert_eq!(bits(&state), bits(&start));
        assert_eq!(frame.to_bytes(), held);
        commit(&cell, &mut state, &mut frame, 1.0, honest).unwrap();
        assert_eq!(frame.proof, PROOF);
    }

    #[test]
    fn every_refusal_is_named_and_leaves_state_and_frame_bit_identical() {
        let (cell, start) = fresh(350.0);
        let mut frame = blank();
        let held = frame.to_bytes();
        let mut attempt = |alter: &dyn Fn(&mut RunawayState), load: RunawayLoad, dt_s: f64, expected: StepReject| {
            let mut altered = start;
            alter(&mut altered);
            let before = bits(&altered);
            assert_eq!(step(&cell, &mut altered, &mut frame, load, dt_s), Err(expected));
            assert_eq!(bits(&altered), before);
            assert_eq!(frame.to_bytes(), held);
        };
        let none = |_: &mut RunawayState| {};
        attempt(&none, oven(400.0), f64::NAN, StepReject::NonFiniteInput);
        attempt(&none, oven(f64::INFINITY), 1.0, StepReject::NonFiniteInput);
        attempt(&none, calorimeter(f64::NAN), 1.0, StepReject::NonFiniteInput);
        attempt(&none, RunawayLoad { adiabatic: f64::NAN, ..oven(400.0) }, 1.0, StepReject::NonFiniteInput);
        attempt(&|s| s.temperature_k = f64::NAN, oven(400.0), 1.0, StepReject::NonFiniteState);
        attempt(&|s| s.heat_lost_j = f64::INFINITY, oven(400.0), 1.0, StepReject::NonFiniteState);
        attempt(&none, oven(400.0), 0.0, StepReject::NonPositiveDt);
        attempt(&none, oven(400.0), -1.0, StepReject::NonPositiveDt);
        attempt(&none, oven(0.0), 1.0, StepReject::OvenTemperatureNotPositive);
        attempt(&none, oven(-5.0), 1.0, StepReject::OvenTemperatureNotPositive);
        attempt(&none, calorimeter(-1.0e-9), 1.0, StepReject::HeaterPowerNegative);
        attempt(&none, RunawayLoad { adiabatic: 0.5, ..oven(400.0) }, 1.0, StepReject::AdiabaticNotZeroOrOne);
        attempt(&none, RunawayLoad { adiabatic: 2.0, ..oven(400.0) }, 1.0, StepReject::AdiabaticNotZeroOrOne);
        attempt(&|s| s.sei_fraction = 1.0000000000000002, oven(400.0), 1.0, StepReject::SeiFractionOutsideCell);
        attempt(&|s| s.anode_lithium_fraction = -1.0e-300, oven(400.0), 1.0, StepReject::AnodeLithiumOutsideCell);
        attempt(&|s| s.anode_lithium_fraction = 0.7830000000000001, oven(400.0), 1.0, StepReject::AnodeLithiumOutsideCell);
        attempt(&|s| s.cathode_unreacted_fraction = 1.0000000000000002, oven(400.0), 1.0, StepReject::CathodeUnreactedOutsideCell);
        attempt(&|s| s.electrolyte_fraction = 1.0000001, oven(400.0), 1.0, StepReject::ElectrolyteOutsideCell);
        // the oven temperature is tested where it enters: with adiabatic 0
        attempt(&none, RunawayLoad { adiabatic: 0.5, ..oven(0.0) }, 1.0, StepReject::AdiabaticNotZeroOrOne);
        // A heater that would carry the cell past the can's melting point
        // inside the step: refused, not clamped.
        attempt(&none, calorimeter(1.0e6), 1.0, StepReject::CanMelting);
        // A substep that would land at or below zero kelvin is named, not
        // clamped. No load of the card's own step reaches it, so a stepping
        // method that does is the attack.
        let mut cold = RunawayState::try_fresh(&cell, 1.0).unwrap();
        let before = bits(&cold);
        let through_zero: Substep = |_, point, _, _, _, _| (Point { temperature_k: -point.temperature_k, logs: point.logs }, 0.0);
        assert_eq!(
            step_with(&cell, &mut cold, &mut frame, oven(300.0), 1.0, through_zero, true),
            Err(StepReject::TemperatureNotPositive)
        );
        assert_eq!(bits(&cold), before);
        // A cell at 1 K in an oven at 1e-9 K cools and stays above zero.
        step(&cell, &mut cold, &mut frame, oven(1.0e-9), 1.0).unwrap();
        assert!(cold.temperature_k > 0.0 && cold.temperature_k < 1.0);
        // The starting state is held to the same bounds by name.
        for (temperature, amounts, expected) in [
            (0.0, [0.15, 0.75, 0.96, 1.0], StepReject::TemperatureNotPositive),
            (-1.0, [0.15, 0.75, 0.96, 1.0], StepReject::TemperatureNotPositive),
            (1811.0, [0.15, 0.75, 0.96, 1.0], StepReject::CanMelting),
            (300.0, [1.5, 0.75, 0.96, 1.0], StepReject::SeiFractionOutsideCell),
            (300.0, [0.15, 0.8, 0.96, 1.0], StepReject::AnodeLithiumOutsideCell),
            (300.0, [0.15, 0.75, 1.01, 1.0], StepReject::CathodeUnreactedOutsideCell),
            (300.0, [0.15, 0.75, 0.96, -0.1], StepReject::ElectrolyteOutsideCell),
        ] {
            assert_eq!(
                RunawayState::try_start(&cell, temperature, amounts[0], amounts[1], amounts[2], amounts[3]),
                Err(expected),
                "{temperature} {amounts:?}"
            );
        }
        assert_eq!(RunawayState::try_start(&cell, f64::NAN, 0.15, 0.75, 0.96, 1.0), Err(StepReject::NonFiniteInput));
        assert!(RunawayState::try_start(&cell, 1810.999, 0.15, 0.75, 0.96, 1.0).is_ok());
        assert!(RunawayState::try_start(&cell, 300.0, 0.0, 0.0, 0.0, 0.0).is_ok());
        assert!(RunawayState::try_start(&cell, 300.0, 1.0, 0.783, 1.0, 1.0).is_ok());
        assert_eq!(frame.proof, PROOF);
    }

    /// A step that would need more than 2^20 substeps is refused by name. The
    /// card's own substeps follow each stage in its logarithm, about ten to an
    /// e-fold and one to a kelvin, so its own step from any state it takes
    /// needs a few thousand at the most; a stepping method that does not move
    /// the amounts keeps the fastest stage stiff and runs the budget out, and
    /// the state and the frame are kept. A cell whose cathode has no
    /// converted part, held at 900 K, has nothing to follow in that stage and
    /// takes one substep.
    #[test]
    fn a_step_beyond_the_substep_budget_is_named() {
        let (cell, start) = fresh(1000.0);
        let mut state = start;
        let mut frame = blank();
        let held = frame.to_bytes();
        let stuck: Substep = |_, point, _, _, _, _| (point, 0.0);
        assert_eq!(
            step_with(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0, stuck, true),
            Err(StepReject::SubstepsAboveBudget)
        );
        assert_eq!(bits(&state), bits(&start));
        assert_eq!(frame.to_bytes(), held);
        // the card's own step from the same state lands, in under 3,000 substeps
        step(&cell, &mut state, &mut frame, calorimeter(0.0), 1.0).unwrap();
        assert!(state.substeps > 100.0 && state.substeps < 3000.0, "{}", state.substeps);
        let mut unseeded = RunawayState::try_start(&cell, 900.0, 0.0, 0.0, 1.0, 0.0).unwrap();
        step(&cell, &mut unseeded, &mut frame, oven(900.0), cell.gate_dt_s().unwrap()).unwrap();
        assert_eq!(unseeded.substeps, 1.0);
        assert_eq!(unseeded.cathode_unreacted_fraction, 1.0);
    }

    /// The frame carries the declared slots.
    #[test]
    fn the_frame_carries_the_declared_slots() {
        let (cell, mut state) = oven_state_at(1450.0);
        let mut frame = blank();
        step(&cell, &mut state, &mut frame, oven(428.15), 1.0).unwrap();
        assert_eq!(frame.t, state.t_s);
        assert_eq!(frame.pos[0], state.temperature_k as f32);
        assert_eq!(frame.pos[1], state.chemical_heat_w as f32);
        assert_eq!(frame.pos[2], state.heat_released_j as f32);
        assert_eq!(frame.vel[0], state.anode_lithium_fraction as f32);
        assert_eq!(frame.vel[1], state.cathode_unreacted_fraction as f32);
        assert_eq!(frame.vel[2], state.electrolyte_fraction as f32);
        assert_eq!(frame.force_torque, state.heat_lost_j as f32);
        assert_eq!(frame.residual, state.residual_conservation_error as f32);
        assert_eq!(frame.flags, FLAG_SELF_HEATING | FLAG_RUNAWAY);
        assert_eq!(frame.proof, PROOF);
        assert!(relative(state.sei_thickness + state.anode_lithium_fraction, 0.783) < 1.0e-15);
    }

    /// A run cut in two by handing back the returned temperature and amounts
    /// as a new start continues the run: the same temperature after it to
    /// within rounding, with the heat books begun again.
    #[test]
    fn a_returned_state_handed_back_continues_the_run() {
        let (cell, mut whole) = fresh(298.15);
        let mut frame = blank();
        for _ in 0..200 {
            step(&cell, &mut whole, &mut frame, oven(428.15), 10.0).unwrap();
        }
        let (_, mut first) = fresh(298.15);
        for _ in 0..140 {
            step(&cell, &mut first, &mut frame, oven(428.15), 10.0).unwrap();
        }
        let mut second = RunawayState::try_start(
            &cell,
            first.temperature_k,
            first.sei_fraction,
            first.anode_lithium_fraction,
            first.cathode_unreacted_fraction,
            first.electrolyte_fraction,
        )
        .unwrap();
        for _ in 0..60 {
            step(&cell, &mut second, &mut frame, oven(428.15), 10.0).unwrap();
        }
        assert_eq!(second.temperature_k.to_bits(), whole.temperature_k.to_bits());
        assert_eq!(second.anode_lithium_fraction.to_bits(), whole.anode_lithium_fraction.to_bits());
        assert!(relative(first.heat_released_j + second.heat_released_j, whole.heat_released_j) < 1.0e-12);
    }

    #[test]
    fn the_step_ceiling_and_floor_are_present_and_a_tiny_step_is_not_clamped() {
        let (cell, mut state) = fresh(400.0);
        let mut broken = cell;
        broken.gate_over_convective_time = f64::NAN;
        assert_eq!(broken.gate_dt_s(), None);
        assert_eq!(broken.floor_dt_s(), None);
        let floor = cell.floor_dt_s().unwrap();
        let mut frame = blank();
        let before = state.temperature_k;
        step(&cell, &mut state, &mut frame, oven(500.0), floor).unwrap();
        assert_eq!(state.t_s, floor);
        assert!(state.temperature_k > before);
        assert_eq!(state.substeps, 1.0);
    }
}


