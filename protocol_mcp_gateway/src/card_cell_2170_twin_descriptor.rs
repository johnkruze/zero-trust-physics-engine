//! What an agent reads of the cell_2170_twin card: its fields and bounds, its step floor and
//! ceiling, its refusals word for word from the card's source, its frame, its worked examples and
//! its record. A child module of `descriptor.rs`, so it uses that file's own builders.
//!
//! The card is the twin of the three cells of the Tesla Model 3 2170 dataset, calibrated on all
//! their published data. It supersedes cell_2170_m80 and cell_2170_model3, which stay served
//! under their own IDs.

use genesis_kernel::cell_2170_twin;
use serde_json::Value;

use super::{Advisory, Card, Role, card_dt, finish, obj, pair_examples_recorded, scalar, scalar_value, slots, token};

const DOMAIN: &str = "energy";
const CARD: &str = "cell_2170_model3";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "One Tesla Model 3 2170 lithium-ion cell, any of the three cells m80, m1000 and m448-N of its dataset, as one physical model with memory from -20 to 40 degC: terminal voltage and core temperature from a terminal current and a measured case temperature, with Butler-Volmer charge transfer and Arrhenius kinetics read at the core, the salt's transport limit, and the core warmed by the cell's own heat. Calibrated on all the published data of the three cells; its spec sheet gives the numbers.";

/// The object.
const WHAT: &str = "One Tesla Model 3 2170 lithium-ion cell (21 mm by 70 mm, nickel-cobalt-aluminium, about 4.7 Ah new) from -20 to 40 degrees Celsius: its terminal voltage and the temperature of its core, given the current and the temperature measured on its case each second. One physical model with memory for the cell type and one row of three numbers for each of the three cells of the dataset. The rest voltage is a measured curve with its change with age and with temperature. Below it: an immediate resistance with a part that grows in the cold; the surface reaction as Butler-Volmer with the double layer beside it, its exchange current Arrhenius in the core temperature; a second reaction slow only near full charge; lithium diffusing in the particles as a sphere's eleven decaying modes; hysteresis; and the salt in the electrolyte, whose concentration difference under sustained current follows a Vogel law in the core temperature and whose drop at the terminals grows as atanh of it toward the transport limit, where the salt at one electrode runs out. The core is one heat capacity behind one thermal resistance from the measured case, warmed by the heat of every step, and every temperature law is read at the core. Two ledgers close every step: the energy at the terminals and the heat of the core.";

/// The data.
const DATA: &str = "The Tesla Model 3 2170 Li-ion Cell Dataset of Kollmeyer, Khanum, Naguib and Emadi, McMaster University, Borealis doi 10.5683/SP3/ZVTR4B, version 3.0, CC BY 4.0, with its paper at ITEC 2022, doi 10.1109/ITEC53557.2022.9813996 (cite gives both in full). Three cells from one pack, m80, m1000 and m448-N, each tested in a thermal chamber at 40, 25, 10, 0, -10 and -20 degrees Celsius in that order over eight months of 2021 while it aged from about 4.69 to 4.17 Ah: at each temperature a slow capacity test at 40 degrees, discharges at a third, a half and the whole of the rated current, a pulse test at thirteen charge levels, a slow discharge and charge, eight reordered drive cycles, a highway cycle and a hill cycle. Every published record of the three cells is used: 234 held-current discharges and drive cycles, and the pulse tests, slow loops, capacity tests and the rests after them.";

/// How the card is compared.
const COMPARED: &str = "Every held-current discharge and drive cycle of the three cells computed from the record's first sample with every memory at rest, at one-second steps, with the logged current and case temperature, and its capacity and starting state of charge read from the full charge before it. Error is card minus measurement at the logged samples of each run's span, up to where the card stops, and the root mean square is pooled over every compared second of a block and rate. Fitted: the type's numbers and the three resistance scales, by least squares on the pulse drops, the slow loops, the held-current discharges with the rests after them and the drive cycles of all three cells together. Held out: each cell left out in turn, the type's numbers and the curves fitted on the other two cells, and the left-out cell's own resistance scale fitted on its pulse tests alone; then all its discharges and drive cycles computed. Constants file SHA-256 c016cdb8a1e30d29e436e628b11cd5f699932408bc8e254ab5c9aa4878db6c2e." /*COMPARED*/;

/// The result.
const RESULT: &str = "Fitted on all 234 runs: 19.4 mV RMS; by block, C/3, C/2, 1C and drive cycles: 40 degC 9.3, 24.6, 18.4, 13.5; 25 degC 16.2, 18.9, 22.5, 12.4; 10 degC 21.2, 21.2, 16.6, 15.5; 0 degC 19.7, 20.6, 23.5, 20.4; -10 degC 31.1, 38.1, 35.9, 24.9; -20 degC 67.4, 51.7, 49.6, 43.3. Each cell left out in turn: 20.2 mV RMS; 40 degC 9.3, 26.7, 19.7, 14.2; 25 degC 15.7, 20.8, 22.7, 12.4; 10 degC 20.5, 24.0, 21.8, 15.8; 0 degC 18.4, 24.1, 31.9, 21.3; -10 degC 29.6, 41.0, 50.9, 25.2; -20 degC 67.9, 47.9, 68.7, 47.0. -10 and -20 degC together: 32.8 mV fitted, 34.2 held out. The hill cycle: 37.3 mV fitted, 39.3 held out." /*RESULT*/;

/// Where the numbers stop.
const LIMITS: &str = "Three cells of one pack, one laboratory, one chamber each, six temperatures: everything below -10 degC rests on one block, run last, on the oldest cells, so age and the cold are not separated there. The case temperature is the boundary; the core's temperature is computed, not measured, by one heat capacity and one thermal resistance. The range: case temperatures -23.3 to 50.2 degC, currents -13.984 to 16.152 A, a 60 s running mean to 11.949 A discharging and 4.971 A charging, capacities 4.16 to 4.7 Ah, the largest compared values with the tester's or the chamber's stated accuracy beyond them; outside it a step lands with FLAG_OUTSIDE_COMPARED_RUNS. The card steps by one second only. The capacity is the caller's, read from the last full charge. A run starts from a rested cell." /*LIMITS*/;

fn per_cell() -> String {
    cell_2170_twin::CELLS
        .iter()
        .map(|row| {
            format!(
                "cell {}: capacity at its first capacity test {} Ah, charge-return ratio {}, resistance scale {}",
                row.name,
                crate::message_number(row.reference_capacity_ah),
                crate::message_number(row.charge_return_ratio),
                crate::message_number(row.resistance_scale)
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn bands() -> String {
    let mut out = Vec::new();
    for block in 0..cell_2170_twin::BLOCK_COUNT {
        out.push(format!(
            "{} to {} Ah from {} to {} degC (the {} degC block)",
            token(cell_2170_twin::BLOCK_CAPACITY_LEAST_AH[block]),
            token(cell_2170_twin::BLOCK_CAPACITY_MOST_AH[block]),
            token(cell_2170_twin::BLOCK_CASE_TEMPERATURE_LEAST_C[block]),
            token(cell_2170_twin::BLOCK_CASE_TEMPERATURE_MOST_C[block]),
            token(cell_2170_twin::BLOCK_CHAMBER_C[block]),
        ));
    }
    out.join(", ")
}

pub(super) fn card() -> Card {
    let cell = cell_2170_twin::Cell2170Twin::cell();
    let gate = cell.gate_dt_s().expect("specimen gate");
    let floor = cell.floor_dt_s().expect("specimen floor");
    let rows = cell_2170_twin::CELLS;
    let numbers = rows.iter().map(|row| row.cell_number);
    let least = numbers.clone().fold(f64::INFINITY, f64::min);
    let most = numbers.fold(f64::NEG_INFINITY, f64::max);
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            scalar(
                "cell_number",
                Role::Phase,
                "1",
                Some(least),
                false,
                Some(most),
                false,
                format!(
                    "exactly one of {}: the number in the dataset's name for the cell. It picks the numbers that belong to one cell; the equations, the curves and the fitted numbers are the cell type's. {}. The scale multiplies the three resistances and divides the two exchange currents; the ratio is for reading capacity_ah from a recharge and the step does not use it. Any other value is refused as CellNotRecognized; nothing is rounded to a cell",
                    crate::card_cell_2170_twin::cells_in_words(),
                    per_cell()
                ),
            ),
            scalar(
                "state_of_charge",
                Role::Phase,
                "1",
                Some(cell_2170_twin::CURVE_FIRST_STATE_OF_CHARGE),
                false,
                Some(cell_2170_twin::CURVE_LAST_STATE_OF_CHARGE),
                false,
                format!(
                    "inclusive [{}, {}], the ends of the measured rest-voltage curves (StateOfChargeOutsideCard): 1 after a full recharge, and 1 plus the top-up in ampere-hours divided by capacity_ah when the tester topped the cell up afterwards. Compared from 0 to {}; a state of charge outside that sets FLAG_OUTSIDE_COMPARED_RUNS. The cell is taken as rested: every memory starts at zero and the core at the case temperature of the first step",
                    token(cell_2170_twin::CURVE_FIRST_STATE_OF_CHARGE),
                    token(cell_2170_twin::CURVE_LAST_STATE_OF_CHARGE),
                    token(cell_2170_twin::STATE_OF_CHARGE_MAX_COMPARED),
                ),
            ),
            scalar(
                "capacity_ah",
                Role::Phase,
                "Ah",
                Some(0.0),
                true,
                None,
                false,
                format!(
                    "above 0 Ah (CapacityNotPositive): the capacity from the last full charge, which falls as the cell ages; the caller's number, which the card never changes. Compared on {} to {} Ah; outside it sets FLAG_OUTSIDE_COMPARED_RUNS. Each block of tests was measured at one age of the cell, so each capacity was compared in its own block's band at the case temperatures that block spanned: {}. A capacity and a case temperature that no one block holds land with FLAG_OUTSIDE_COMPARED_RUNS",
                    token(cell_2170_twin::CAPACITY_MIN_COMPARED),
                    token(cell_2170_twin::CAPACITY_MAX_COMPARED),
                    bands(),
                ),
            ),
            scalar(
                "case_temperature_least_c",
                Role::Phase,
                "degC",
                Some(-cell_2170_twin::KELVIN_OFFSET),
                true,
                None,
                false,
                format!(
                    "above -273.15 degC, absolute zero (CaseTemperatureBelowAbsoluteZero), required: the least case temperature of the run so far. A run starts rested and states it equal to its first segment's case_temperature_c, and a single step equal to its case_temperature_c. A span of case temperature past {} K in a run sets FLAG_OUTSIDE_COMPARED_RUNS. A value above case_temperature_most_c is refused as CaseTemperatureSpanInverted",
                    token(cell_2170_twin::CASE_SPAN_MAX_COMPARED),
                ),
            ),
            scalar(
                "case_temperature_most_c",
                Role::Phase,
                "degC",
                Some(-cell_2170_twin::KELVIN_OFFSET),
                true,
                None,
                false,
                "above -273.15 degC, required: the most case temperature of the run so far, stated equal to the first case_temperature_c at a rested start".into(),
            ),
            scalar(
                "current_a",
                Role::Load,
                "A",
                None,
                false,
                None,
                false,
                format!(
                    "any current: each line of the card is the exact solution for a held current at any size, and a large one ends at a voltage limit or where the salt runs out (ElectrolyteDepleted). A positive current discharges the cell and a negative current charges it. Compared from {} to {} A, the largest charging and discharging currents of the compared runs plus the tester's stated 0.024 A; a 60 s running mean compared to {} A discharging and {} A charging. Outside either it sets FLAG_OUTSIDE_COMPARED_RUNS; a running mean above {} A discharging sets FLAG_SUSTAINED_ABOVE_1C",
                    token(-cell_2170_twin::CHARGE_MAX_COMPARED),
                    token(cell_2170_twin::DISCHARGE_MAX_COMPARED),
                    token(cell_2170_twin::SUSTAINED_DISCHARGE_MAX_COMPARED),
                    token(cell_2170_twin::SUSTAINED_CHARGE_MAX_COMPARED),
                    token(cell_2170_twin::SUSTAINED_FLAG_COMPARED),
                ),
            ),
            scalar(
                "case_temperature_c",
                Role::Load,
                "degC",
                Some(-cell_2170_twin::KELVIN_OFFSET),
                true,
                None,
                false,
                format!(
                    "above -273.15 degC, absolute zero (CaseTemperatureBelowAbsoluteZero): the temperature measured on the cell's case at the end of the step, the boundary of the core. The core is warmed above it by the cell's own heat through a thermal resistance of {} K/W and a heat capacity of {} J/K, and every temperature law is read at the core. Compared from {} to {} degC, the sensor's range in the compared runs with the chamber's 0.3 K beyond it; outside it sets FLAG_OUTSIDE_COMPARED_RUNS, as do a change past {} K between two steps and a core more than {} K from its case. A core at or below the electrolyte's glass temperature, {} K, is refused as CoreAtElectrolyteGlassTemperature",
                    token(cell_2170_twin::CORE_THERMAL_RESISTANCE_K_PER_W),
                    token(cell_2170_twin::CORE_HEAT_CAPACITY_J_PER_K),
                    token(cell_2170_twin::CASE_MIN_COMPARED),
                    token(cell_2170_twin::CASE_MAX_COMPARED),
                    token(cell_2170_twin::CASE_CHANGE_MAX_COMPARED),
                    token(cell_2170_twin::CORE_RISE_MAX_COMPARED),
                    token(cell_2170_twin::ELECTROLYTE_GLASS_TEMPERATURE_K),
                ),
            ),
        ],
        floor,
        "a locked 1 s, the ceiling itself: every fit and every comparison was made at one second, and measurements logged once a second cannot test a shorter step. Where the current reverses, the rule for the surface reaction's overpotential changing sign takes the whole step again from zero with the other law, so the answer depends on the step",
        gate,
        "a locked 1 s, the sampling of the measurements every fit and every comparison was made on",
        None,
        "Over a step the current and the case temperature are held. The range compared with measurement is a range, not a wall: a capacity, a case temperature or a current outside it, a capacity outside its block's band at a step's case temperature, a jump or a span of case temperature past what was compared, a running mean past what was compared, a core further from its case than was computed in the compared runs, and a state of charge past the compared start each land with FLAG_OUTSIDE_COMPARED_RUNS, every line the card's own equation and both ledgers closed. A case temperature at or below absolute zero in any segment of a run refuses the whole run before its first step. A start from a returned state is not served on this card: through from it is refused as ContinuationNotOnCard, and without from a pair other than a fresh start's is refused as ContinuationWithoutUpstream, because the card's memories, its core temperature among them, are not start fields and would start again from rest. A run starts from a rested cell: the state of charge and the capacity as given, the core at the case temperature of its first step, every other memory at zero, and both ledgers anchored there. A run ends in a sealed refusal when the terminal voltage would fall below 2.8 V (UndervoltageLimit) or rise above 4.242 V (OvervoltageLimit), when the salt at one electrode would run out (ElectrolyteDepleted), or when the state of charge or its value at the particle surface would leave the measured curves, -0.05 to 1.05 (StateOfChargeOutsideCard, SurfaceOutsideCurve); the last landed state is the last step on the card. A step of zero or less is refused as NonPositiveDt before the step floor is tested.".into(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidCell",
            "CellNotRecognized",
            "CapacityNotPositive",
            "CaseTemperatureBelowAbsoluteZero",
            "CoreAtElectrolyteGlassTemperature",
            "CaseTemperatureSpanInverted",
            "StateOfChargeOutsideCard",
            "SurfaceOutsideCurve",
            "ElectrolyteDepleted",
            "UndervoltageLimit",
            "OvervoltageLimit",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![
            Advisory { name: "FLAG_SUSTAINED_ABOVE_1C", mask: cell_2170_twin::FLAG_SUSTAINED_ABOVE_1C, seals: true },
            Advisory { name: "FLAG_OUTSIDE_COMPARED_RUNS", mask: cell_2170_twin::FLAG_OUTSIDE_COMPARED_RUNS, seals: true },
        ],
        slots(&[
            ("t", "Time since the anchor, in seconds."),
            ("pos[0]", "terminal_v"),
            ("pos[1]", "charge_removed_ah"),
            ("pos[2]", "surface_state_of_charge"),
            ("vel[0]", "reaction_overpotential_v"),
            ("vel[1]", "core_temperature_c"),
            ("vel[2]", "electrolyte_drop_v"),
            ("force_torque", "Running mean of the current with a 60 s time constant, in amperes, positive discharging."),
            ("residual", "Conservation residual: the larger of the energy ledger's and the core's heat ledger's, each over the cell's energy from empty to full."),
            (
                "flags",
                "FLAG_SUSTAINED_ABOVE_1C: the running mean of the current is above the rated current and a margin, discharging. FLAG_OUTSIDE_COMPARED_RUNS: the step lies outside the range compared with measurement. Both seal.",
            ),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// The worked run: cell m80 at the -20 degC block from full, at the rated current with the case
/// held at -20 degC, and its first step.
pub(super) fn examples() -> (Value, Value) {
    let cell = cell_2170_twin::Cell2170Twin::cell();
    pair_examples_recorded(
        DOMAIN,
        CARD,
        obj(&[
            ("cell_number", scalar_value(cell_2170_twin::CELLS[0].cell_number)),
            ("state_of_charge", scalar_value(1.0)),
            ("capacity_ah", scalar_value(EXAMPLE_CAPACITY_AH)),
            ("case_temperature_least_c", scalar_value(EXAMPLE_CASE_C)),
            ("case_temperature_most_c", scalar_value(EXAMPLE_CASE_C)),
        ]),
        obj(&[("current_a", scalar_value(4.5)), ("case_temperature_c", scalar_value(EXAMPLE_CASE_C))]),
        card_dt(cell.gate_dt_s()),
        EXAMPLE_STEPS,
        60,
    )
}

/// The worked run's capacity: cell m80 in its -20 degC block.
pub(crate) const EXAMPLE_CAPACITY_AH: f64 = 4.2;
/// The worked run's case temperature.
pub(crate) const EXAMPLE_CASE_C: f64 = -20.0;
/// The steps the worked run asks for: ten minutes at the rated current in the cold.
pub(crate) const EXAMPLE_STEPS: u64 = 600;

/// The record, in the five parts every card gives.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [WHAT.into(), DATA.into(), COMPARED.into(), RESULT.into(), LIMITS.into()]
}

/// Each refusal's sentence, word for word from the doc comment above the variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "Current, case temperature or `dt` was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was non-finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidCell" => "A recorded constant or curve entry is not this cell type's.",
        "CellNotRecognized" => "The cell number of the state names none of the cells the card holds.",
        "CapacityNotPositive" => "Capacity is zero or less: the charge is counted against it.",
        "CaseTemperatureBelowAbsoluteZero" => "Case temperature is at or below absolute zero, -273.15 degC.",
        "CoreAtElectrolyteGlassTemperature" => "The core is at or below the electrolyte's glass temperature, where its Vogel law has no value: the electrolyte no longer conducts.",
        "CaseTemperatureSpanInverted" => "The least case temperature the state carries is above the most it carries.",
        "StateOfChargeOutsideCard" => "State of charge, incoming or landing, is outside the ends of the curves, -0.05 and 1.05, by more than the rounding allowance of its running sum.",
        "SurfaceOutsideCurve" => "Landing surface state of charge is outside the ends of the curves.",
        "ElectrolyteDepleted" => "The salt's concentration difference reached its limit: the electrolyte at one electrode is depleted and the cell cannot carry the current.",
        "UndervoltageLimit" => "Landing terminal voltage is below 2.8 V.",
        "OvervoltageLimit" => "Landing terminal voltage is above 4.242 V.",
        "SpontaneousGeneration" => "The trial step created energy from nothing, in the energy ledger or the core's heat ledger.",
        "FirstLawBreach" => "The trial step destroyed energy, in the energy ledger or the core's heat ledger.",
        _ => return None,
    })
}
