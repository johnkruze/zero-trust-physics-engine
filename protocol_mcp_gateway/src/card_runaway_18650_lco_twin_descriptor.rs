//! What an agent reads of the runaway_18650_lco_twin card: its fields and bounds, its step
//! floor and ceiling, its refusals word for word from the card's source, its
//! frame, its worked examples and its record. Stamped from the kit; a child
//! module of `descriptor.rs`, so it uses that file's own builders.
//!
//! The card is the twin of runaway_18650_lco, fitted on the open records that
//! card was compared with. Its description leads with what it computes and its
//! numbers against those records, and prints every constant.

use genesis_kernel::runaway_18650_lco_twin as twin;
use serde_json::Value;

use super::{Advisory, Card, Role, finish, obj, pair_examples_recorded, scalar, scalar_value, slots};

const DOMAIN: &str = "energy";
const CARD: &str = "runaway_18650_lco";

/// What the card is, in one sentence of the list (the spec sheet replaces it where it is served).
const SPECIMEN: &str = "Thermal runaway of one LiCoO2 18650 cell in an oven, under a heater or in an adiabatic calorimeter, fitted on 13 open records: onsets within 13.5 K RMS (16.2 with each record left out), highest temperatures within 51.1 K RMS (51.7), Hatchard's 155 degC oven running away at 62.0 minutes against 51.1.";

/// The object.
const WHAT: &str = "One 18650 lithium-ion cell, LiCoO2 against graphite in a carbonate electrolyte, fully charged, as one lumped body in thermal abuse: in an oven held at a temperature, under a heater, or tracked by an accelerating-rate calorimeter. Six stages carry its heat: the four decomposition stages of Hatchard et al. (2001) with the constants of Kim et al. (2007) (the SEI, the lithiated graphite with the electrolyte, slowed as its SEI grows, the autocatalytic LiCoO2 cathode with the electrolyte, and the electrolyte), the separator melting at 130 degC and taking heat, and the vent opening at 160 degC, its gas carrying heat away, the two temperatures Golubkov et al. (2014) give for their cells.";

/// The data.
const DATA: &str = "The open records of LiCoO2 18650 cells: Hatchard's cell in ovens at 145, 150 and 155 degC from 28 degC (Hatchard et al. 2001, as Lopez et al. 2015 Fig. 2a reprint it; Ostanek et al. 2023 time the 155 degC cell's 20 K per minute at 51.1 min and its vent at 41.3 min); Golubkov et al. (2014) Table 3, the LCO/NMC cell heated at about 2 degC per minute (onset 149, rapid runaway 208, highest 853 degC); Mendoza-Hernandez et al. (2017) Table 1 and Fig. 1, a 2,550 mAh LiCoO2 cell at SOC 100 in heat-wait-seek (self-heating 100.0 degC at 0.05 degC per minute, runaway 144.9 degC at 1 degC per minute, about 5.7 h apart); and the LiCoO2 rows at 4.2 V or SOC 100 that Lei et al. (2017) Table 1 reprint: Jhu's 2.6 Ah cell, Sanyo UR18650F, Sony US18650GR, Samsung ICR18650-26D, LG LGDB218650, Chen's Sony 2.6 Ah cell, Mendoza-Hernandez's 0.8 Ah cell and Lu's 4.2 V cell (onsets 116.5 to 142.5 degC, highest temperatures 654.3 to 708.8 degC, and Lu's 370.03). 13 records, 25 numbers.";

/// How the card is compared.
const COMPARED: &str = "Each record is run the way it was taken. An oven: a fresh cell from 28 degC, 240 min; ran away is 10 K per minute within the record's 120 min, the time to runaway the first 20 K per minute, the vent half its gas gone. Golubkov's ramp: adiabatic, a heater giving 2 K per minute with no reaction until 10 K per minute, then off; the onset where the cell's own heating equals the heater's, where the paper's two lines in the rate plot cross, the rapid runaway 10 K per minute. Heat-wait-seek: 50 degC start, 5 K steps at 2 K per minute, 30 min wait, 5 min search, then tracking; the onset the step temperature at which the search first detects the record's threshold (0.05 K per minute for Mendoza-Hernandez 2017, 0.02 where none is stated), the runaway 1 K per minute. Seven constants fitted by least squares on all 13 records together, each number over its scale (onsets and runaway onsets 10 K, highest temperatures 50 K, the oven's times 10 percent in their logarithm, the 145 degC oven's peak 5 K, Mendoza-Hernandez's hours 30 percent); Lu's highest temperature, 370 degC, is compared and not fitted. Then each record left out in turn, the seven constants fitted on the other 12 and the left-out record computed with them. The fit's numbers are those of the engine's own runs; the left-out records' are those of an independent integration of the same equations (scipy LSODA), which the engine's runs follow on these records within 0.2 min and 2.5 K (the engine's frames are 10 s apart and its ramp heater is turned off at the end of a 60 s run).";

/// The result.
const RESULT: &str = "Fitted on all 13 records, every record read when the structure was chosen. Each record, measured / fitted / left out of the fit: Hatchard 155 degC oven, 20 K per minute at 51.1 / 62.0 / 64.8 min, vent at 41.3 / 38.2 min; Hatchard 150 degC, no runaway in 120 min / runaway at 109.7 / 100.0 min; Hatchard 145 degC, peak 148.0 / 148.0 / 148.0 degC and no runaway; Golubkov LCO/NMC, onset 149.0 / 170.0 / 169.3, rapid runaway 208.0 / 186.3 / 185.9, peak 853.0 / 738.4 / 736.2 degC; Mendoza-Hernandez 2017, onset 100.0 / 130.3 / 137.9, runaway 144.9 / 156.2 / 156.8 degC, 5.7 / 2.7 / 1.1 h apart; the reprinted cells, onset and peak in degC, Jhu 131.5 and 708.8 / 130.3 and 704.6 / 130.3 and 704.6, Sanyo UR18650F 131.3 and 708.7 / 130.3 and 704.6 / 130.3 and 704.6, Sony US18650GR 125.2 and 654.3 / 130.3 and 704.6 / 130.5 and 704.8, Samsung ICR18650-26D 116.5 and 683.2 / 130.3 and 704.6 / 130.5 and 704.8, LG LGDB218650 142.5 and 691.9 / 130.3 and 704.6 / 121.7 and 694.2, Chen's Sony 122.0 and 660.0 / 130.3 and 704.6 / 130.5 and 704.8, Mendoza-Hernandez 0.8 Ah onset 124.9 / 130.3 / 130.5, Lu 4.2 V 130.5 and 370.0 / 130.3 and 704.6 / 130.3 and 704.6 (its peak compared, not fitted). Onsets within 13.5 K RMS fitted and 16.2 left out on 10 records, 7.6 and 9.8 on the eight reprinted cells; peaks within 51.1 and 51.7 K RMS on 7. On the records' 23 comparisons, 17 within their targets fitted and 15 left out. Record SHA-256 7075415ce6709167b3ce3b3ffcd3793a55c783cda2af97cc0dc93ac2570cb365.";

/// Where the numbers stop.
const LIMITS: &str = "One body: one temperature for the whole cell, its heat capacity fixed at 35.42 J/K (Hatchard's jelly roll). Not carried: the internal short circuit and the electrical energy of the charge, combustion of the vented gas, the mass the vent lets out, the aluminium collector's melting and the calorimeter's own heat capacity. One cell for all the records: the commercial cells' onsets scatter over 26 K (116.5 to 142.5 degC) and the card gives one. Compared ovens 145 to 155 degC from 28 degC and heaters to 1.2 W; another load is computed by the same equations and each step carries FLAG_OUTSIDE_COMPARED. The two heater tests of the record, Lopez et al. (2015) 20 W and the FAA's 25 W, are not compared: their heaters' contact and losses are not stated. The card refuses by name an oven at or below 0 K where heat crosses the can (OvenTemperatureNotPositive), a negative heater (HeaterPowerNegative), an amount outside where its equation holds, a cell at or above 1811 K, where the steel can melts (CanMelting), and a step that would need more than 1,048,576 substeps (SubstepsAboveBudget); a run ends when the heat its stages still hold or take falls to 0.0205 J (EveryReactantConsumed).";

/// Every constant, printed.
fn constants() -> String {
    format!(
        "Every constant of the card. Fitted on the 13 records: the SEI stage's frequency factor {} per s (published 1.667e15), the SEI thickness of a fresh cell {} (published 0.033), the thickness over which the anode stage slows by e {} (published 0.033), the cathode stage's frequency factor {} per s (published 6.667e13), the convective coefficient to the oven {} W/(m2 K) (published 7.17), the heat the separator takes as it melts {} J and the heat the vent's gas carries away {} J. Published (Ostanek 2025 Table 2; Coman et al. 2017 Table I; Lopez et al. 2015): activation energies {}, {}, {} and {} J/mol for the SEI, anode, cathode and electrolyte stages; the anode's and the electrolyte's frequency factors {} and {} per s; heats {}, {}, {} and {} J/kg with contents of carbon {}, cathode {} and electrolyte {} kg/m3; a fresh cell's amounts 0.15 (SEI lithium), 0.75 (graphite lithium), 0.96 (unreacted cathode), 1 (electrolyte), 1 (separator) and 1 (vent gas); the gas constant {} J/(mol K); a cylinder {} m in radius and {} m high, density {} kg/m3 and specific heat {} J/(kg K), heat capacity {} J/K; emissivity {}; Stefan-Boltzmann {} W/(m2 K4). The melt at {} K (130 degC) and the vent at {} K (160 degC), Golubkov et al. (2014), each at a rate of 1/{} per second at its temperature, rising e-fold per {} K there. Each stage's heat per unit of its amount: {} J. The cell's chemical heat {} J, the can melting at {} K. Compared loads: an oven from {} to {} K and a heater to {} W.",
        twin::SEI_FREQUENCY_FACTOR_PER_S,
        twin::SEI_THICKNESS_START,
        twin::SEI_THICKNESS_REFERENCE,
        twin::CATHODE_FREQUENCY_FACTOR_PER_S,
        twin::CONVECTION_W_PER_M2_K,
        twin::SEPARATOR_MELT_HEAT_J,
        twin::VENT_HEAT_J,
        twin::SEI_ACTIVATION_J_PER_MOL,
        twin::ANODE_ACTIVATION_J_PER_MOL,
        twin::CATHODE_ACTIVATION_J_PER_MOL,
        twin::ELECTROLYTE_ACTIVATION_J_PER_MOL,
        twin::ANODE_FREQUENCY_FACTOR_PER_S,
        twin::ELECTROLYTE_FREQUENCY_FACTOR_PER_S,
        twin::SEI_HEAT_J_PER_KG,
        twin::ANODE_HEAT_J_PER_KG,
        twin::CATHODE_HEAT_J_PER_KG,
        twin::ELECTROLYTE_HEAT_J_PER_KG,
        twin::CARBON_CONTENT_KG_PER_M3,
        twin::CATHODE_CONTENT_KG_PER_M3,
        twin::ELECTROLYTE_CONTENT_KG_PER_M3,
        twin::GAS_CONSTANT_J_PER_MOL_K,
        twin::CELL_RADIUS_M,
        twin::CELL_HEIGHT_M,
        twin::CELL_DENSITY_KG_PER_M3,
        twin::CELL_SPECIFIC_HEAT_J_PER_KG_K,
        twin::CELL_HEAT_CAPACITY_J_PER_K,
        twin::CAN_EMISSIVITY,
        twin::STEFAN_BOLTZMANN_W_PER_M2_K4,
        twin::SEPARATOR_MELT_K,
        twin::VENT_OPENING_K,
        twin::PHASE_TIME_S,
        twin::PHASE_WIDTH_K,
        twin::STAGE_HEAT_J.iter().map(|x| format!("{x}")).collect::<Vec<_>>().join(", "),
        twin::CELL_CHEMICAL_HEAT_J,
        twin::CAN_MELTING_K,
        twin::COMPARED_OVEN_K[0],
        twin::COMPARED_OVEN_K[1],
        twin::COMPARED_HEATER_W,
    )
}

fn amount(name: &'static str, limit: f64, what: &str, refusal: &str) -> super::FieldSpec {
    scalar(name, Role::Phase, "1", Some(0.0), false, Some(limit), false, format!("from 0 to {limit}, inclusive: {what}. Below 0 or above {limit} it is refused as {refusal}"))
}

pub(super) fn card() -> Card {
    let cell = twin::Runaway18650LcoTwin::fitted();
    let gate = cell.gate_dt_s().expect("specimen gate");
    let floor = cell.floor_dt_s().expect("specimen floor");
    let anode_limit = twin::AMOUNT_LIMITS[1];
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "temperature_k",
                Role::Phase,
                "K",
                Some(0.0),
                true,
                Some(twin::CAN_MELTING_K),
                true,
                "exclusive (0, 1811) K: the temperature of the cell at the start, one for the whole body. At zero kelvin and below it is refused as TemperatureNotPositive; at 1811 K, the melting point of iron, where the steel can melts, and above, as CanMelting. A step whose temperature would reach either inside one of its substeps is refused by the same name".into(),
            ),
            amount("sei_fraction", 1.0, "the lithium in the solid-electrolyte interphase at the start, c_sei. A fresh cell is 0.15", "SeiFractionOutsideCell"),
            amount(
                "anode_lithium_fraction",
                anode_limit,
                &format!("the lithium in the graphite at the start, c_ne. A fresh cell is 0.75; the SEI thickness the anode stage grows and is slowed by is {anode_limit} minus this, so a start below 0.75 is a cell whose anode has already reacted"),
                "AnodeLithiumOutsideCell",
            ),
            scalar(
                "cathode_unreacted_fraction",
                Role::Phase,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "from 0 to 1 - 2^-33 (0.99999999988), inclusive, or exactly 1: the unreacted part of the cathode at the start, 1 minus the converted part alpha. A fresh cell is 0.96. At exactly 1 the autocatalytic stage has no converted part and never starts; above 1 - 2^-33 and under 1, below 0 and above 1 it is refused as CathodeUnreactedOutsideCell".into(),
            ),
            amount("electrolyte_fraction", 1.0, "the electrolyte at the start, c_e. A fresh cell is 1", "ElectrolyteOutsideCell"),
            amount("separator_unmelted_fraction", 1.0, "the part of the separator not yet melted at the start. A fresh cell is 1", "SeparatorOutsideCell"),
            amount(
                "vent_gas_held_fraction",
                1.0,
                "the part of the gas the vent lets out still held in the cell at the start. A fresh cell is 1. A start whose six amounts together hold or take 0.0205 J of heat or less, 1e-6 of the cell's chemical heat, is a spent cell: it is stepped as a body that gains and loses heat, and meets no end",
                "VentGasOutsideCell",
            ),
            scalar(
                "oven_temperature_k",
                Role::Load,
                "K",
                None,
                false,
                None,
                false,
                format!(
                    "above 0 K when adiabatic is 0: the temperature of the oven, held over a step. The cell exchanges heat with it by convection, {} W/(m2 K) fitted on Hatchard's ovens, over the can's 41.85 cm2, and by radiation with an emissivity of 0.8; at zero kelvin and below it is refused as OvenTemperatureNotPositive. Compared at 418.15 to 428.15 K (145 to 155 degC); another oven temperature is computed by the same equations and each step carries FLAG_OUTSIDE_COMPARED. When adiabatic is 1 the oven temperature does not enter the step: it is still required, any temperature of 0 K or more is taken",
                    twin::CONVECTION_W_PER_M2_K
                ),
            ),
            scalar(
                "heater_power_w",
                Role::Load,
                "W",
                Some(0.0),
                false,
                None,
                false,
                "at least 0 W: heat a heater puts into the cell, held over a step. 35.42 W warms the cell by 1 K a second with no reaction. Compared up to 1.2 W in a calorimeter (2 K per minute is 1.1807 W); more, or a heater in an oven, is computed by the same equations and each step carries FLAG_OUTSIDE_COMPARED. Below zero it is refused as HeaterPowerNegative".into(),
            ),
            scalar(
                "adiabatic",
                Role::Load,
                "1",
                Some(0.0),
                false,
                Some(1.0),
                false,
                "0 or 1 and nothing between: 1 for a cell in an accelerating-rate calorimeter that tracks it, so that no heat crosses the can; 0 for a cell that loses heat to the oven. Any other value is refused as AdiabaticNotZeroOrOne".into(),
            ),
        ],
        floor,
        "the ceiling divided by 2^20",
        gate,
        "a hundredth of the cell's convective time constant C / (h A)",
        None,
        "Over a step the oven temperature, the heater power and the adiabatic switch are held. Inside a step the card takes substeps, each classical fourth-order Runge-Kutta over the temperature, the natural logarithm of each of the six amounts and the heat lost, each no longer than 0.1 over the fastest live stage's rate in its logarithm nor than 1 K of temperature change at the substep's starting rate; a stage whose remaining heat is at or under 2^-52 of the cell's chemical heat finishes at the start of a substep, its last heat booked and put into the temperature. A step at or under the ceiling whose ledger does not close with those substeps is taken again with substeps half as long in temperature and in each logarithm, and again, up to 2^20 times shorter, until it closes (since engine 0.15.2; an oven above about 440 K at 10 s, where the separator's melt climbs steeply within a substep, needed it); a step that lands with the first substeps lands as it always did. A step that would need more than 1,048,576 substeps is refused as SubstepsAboveBudget. The heat each stage released is booked in closed form from its amount, the heater's heat as its power times the step, the heat lost with the temperature: the ledger holds the heat stored, the heat capacity times the rise from the start, to the heat released plus the heater's less the heat lost, within 1e-6 of the cell's chemical heat, 20,499 J. A run starts with every heat sum at zero; they begin again at each start. The end of the runaway is EveryReactantConsumed: the heat the six stages still hold or take falls to 0.0205 J or under, tested at the end of every substep.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "OvenTemperatureNotPositive",
            "HeaterPowerNegative",
            "AdiabaticNotZeroOrOne",
            "TemperatureNotPositive",
            "CanMelting",
            "SeiFractionOutsideCell",
            "AnodeLithiumOutsideCell",
            "CathodeUnreactedOutsideCell",
            "ElectrolyteOutsideCell",
            "SeparatorOutsideCell",
            "VentGasOutsideCell",
            "EveryReactantConsumed",
            "SubstepsAboveBudget",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory { name: "FLAG_SELF_HEATING", mask: twin::FLAG_SELF_HEATING, seals: true },
            Advisory { name: "FLAG_RUNAWAY", mask: twin::FLAG_RUNAWAY, seals: true },
            Advisory { name: "FLAG_SEPARATOR_MELTED", mask: twin::FLAG_SEPARATOR_MELTED, seals: true },
            Advisory { name: "FLAG_VENTED", mask: twin::FLAG_VENTED, seals: true },
            Advisory { name: "FLAG_OUTSIDE_COMPARED", mask: twin::FLAG_OUTSIDE_COMPARED, seals: true },
        ],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "temperature_k"),
            ("pos[1]", "chemical_heat_w"),
            ("pos[2]", "heat_released_j"),
            ("vel[0]", "anode_lithium_fraction"),
            ("vel[1]", "cathode_unreacted_fraction"),
            ("vel[2]", "electrolyte_fraction"),
            ("force_torque", "The heat the cell has lost to the oven since the start, in joules; negative while the oven warms the cell."),
            ("residual", "Conservation residual: the heat ledger's defect over the cell's chemical heat, 20,499 J."),
            (
                "flags",
                "FLAG_SELF_HEATING: the net stage heat warms the cell at 0.02 K per minute or more. FLAG_RUNAWAY: at 10 K per minute or more. FLAG_SEPARATOR_MELTED: half the separator or more has melted. FLAG_VENTED: half the vent's gas or more has left. FLAG_OUTSIDE_COMPARED: the step's load is outside those the records were taken under. All seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The worked run: a fresh cell from 28 degC in Hatchard's 155 degC oven, at
/// 10 s steps for 6,000 s, through its runaway, and its first step.
pub(super) fn examples() -> (Value, Value) {
    pair_examples_recorded(
        DOMAIN,
        CARD,
        obj(&[
            ("temperature_k", scalar_value(301.15)),
            ("sei_fraction", scalar_value(twin::SEI_FRACTION_START)),
            ("anode_lithium_fraction", scalar_value(twin::ANODE_LITHIUM_START)),
            ("cathode_unreacted_fraction", scalar_value(1.0 - twin::CATHODE_CONVERSION_START)),
            ("electrolyte_fraction", scalar_value(twin::ELECTROLYTE_START)),
            ("separator_unmelted_fraction", scalar_value(twin::SEPARATOR_START)),
            ("vent_gas_held_fraction", scalar_value(twin::VENT_GAS_START)),
        ]),
        obj(&[("oven_temperature_k", scalar_value(428.15)), ("heater_power_w", scalar_value(0.0)), ("adiabatic", scalar_value(0.0))]),
        scalar_value(10.0),
        600,
        10,
    )
}

/// The record, in the five parts every card gives.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        WHAT.to_string(),
        DATA.to_string(),
        COMPARED.to_string(),
        format!("{} {}", RESULT, constants()),
        LIMITS.to_string(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "The oven temperature, the heater power, the adiabatic switch or `dt` was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or a substep, was not finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidCell" => "A recorded constant is not this cell's.",
        "OvenTemperatureNotPositive" => "The oven temperature is zero kelvin or below while the cell loses heat to the oven, adiabatic 0; with adiabatic 1 it does not enter and is not tested.",
        "HeaterPowerNegative" => "The heater power is below zero: a heater gives heat and does not take it.",
        "AdiabaticNotZeroOrOne" => "The adiabatic switch is neither 0 nor 1.",
        "TemperatureNotPositive" => "The cell temperature, incoming or in a substep, is zero kelvin or below.",
        "CanMelting" => "The cell temperature, incoming or in a substep, is at or above 1811 K, the melting point of iron: the steel can melts.",
        "SeiFractionOutsideCell" => "The lithium in the SEI is below 0 or above 1: a fraction, it is between none and all.",
        "AnodeLithiumOutsideCell" => "The lithium in the graphite is below 0 or above the fresh SEI thickness plus 0.75: above it the SEI thickness would be below zero.",
        "CathodeUnreactedOutsideCell" => "The unreacted part of the cathode is below 0 or above 1, or above 1 - 2^-33 and under 1: above 1 the converted part would be below zero; just under 1 its growth is lost to rounding.",
        "ElectrolyteOutsideCell" => "The electrolyte is below 0 or above 1: a fraction of the cell's fill, it is between none and all.",
        "SeparatorOutsideCell" => "The separator not yet melted is below 0 or above 1: a part of the separator, it is between none and all.",
        "VentGasOutsideCell" => "The vent's gas still held is below 0 or above 1: a part of the gas the vent lets out, it is between none and all.",
        "EveryReactantConsumed" => "The step would carry the cell to every reactant consumed: the heat the six stages still hold or take falls to 1e-6 of the cell's chemical heat or under, 0.0205 J, and the runaway has finished.",
        "SubstepsAboveBudget" => "The step would need more than 1,048,576 substeps to follow its fastest stage.",
        "SpontaneousGeneration" => "The trial step created heat: the heat stored in the cell rose above the heat released, plus the heater's, less the heat lost.",
        "FirstLawBreach" => "The trial step destroyed heat: the heat stored in the cell fell below the heat released, plus the heater's, less the heat lost.",
        _ => return None,
    })
}
