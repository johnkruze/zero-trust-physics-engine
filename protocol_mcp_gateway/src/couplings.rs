//! The table of couplings: one entry each. A coupling names one card whose
//! sealed run hands values on, one card that takes them, and for each value
//! where it is read in the upstream run, where it stands in the downstream
//! request, its unit, and the downstream card's declared range for it; and
//! the conditions on the two requests under which the values handed carry
//! one meaning in both cards.
//!
//! A coupling is served only where both pieces were compared with
//! measurement on the same object and the value handed is one both pieces
//! take with the same meaning. No physics is added here: a value is carried
//! bit for bit, never converted. A coupling's conditions are decided on the
//! two requests before any physics, and a link that does not meet one is
//! refused as `CouplingConditionNotMet`. A value outside the downstream
//! card's declared range is refused as `CouplingOutsideRange` before
//! anything of the downstream piece runs.
//!
//! A unit is the unit both cards' descriptions give the value, spelled as
//! they spell it, except for the spellings in [`UNIT_SPELLINGS`]: a unit is
//! in every chain seal its coupling gives, so a spelling once served stays.
//!
//! Served: the SPERT III E-core burst between the two SPERT III cards, both
//! compared with IDO-17281 (Phillips Petroleum Company, 1969). What is handed
//! is the test itself, the inlet water temperature at the start and the
//! reactivity inserted, so the two cards' answers are sealed as answers about
//! one burst. And cell m80 of the Tesla Model 3 2170 dataset, rested in its
//! 40 degC block, between the dataset's two cards of it: the charge removed
//! since the laboratory's full recharge from the physical model to the table
//! card, and the charge level from the table card to the physical model. No
//! coupling of capsule_stardust into ablator_slab is served: the card
//! publishes one stagnation-point convective heat flux, while ablator_slab's
//! Stardust stream takes wall pressure, a heat-transfer coefficient, flow
//! enthalpy and radiative heating at the core, which the card does not
//! compute; turning one into the others would be physics added in the
//! coupling, and the slab's Stardust comparison was made with NASA's
//! heating, not the card's. (Besides: the card's compared entry reaches
//! 12,573.44 m/s, a kinetic enthalpy of 79.05 MJ/kg, above the slab's
//! compared 78.31 MJ/kg in the first part of the entry.)

use genesis_kernel::{cell_2170_m80, cell_2170_model3, reactor_spert3, reactor_spert3_fuel};
use serde_json::Value;

/// Where a handed value is read in the upstream run: `final_state.<name>`
/// (its sealed final value) or `inputs.initial.<name>` /
/// `inputs.segments[*].load.<name>` (its sealed request, every segment
/// holding the same value).
pub(crate) struct Mapping {
    pub from: &'static str,
    /// Where it stands in a downstream run request: `initial.<name>` or
    /// `segments[*].load.<name>`. A step request takes it at
    /// `parameters.<name>`.
    pub to: &'static str,
    pub unit: &'static str,
    /// `Some(sentence)` when the value is outside the downstream card's
    /// declared range, given the values already handed by this coupling.
    pub outside: fn(f64, &[(&'static str, f64)]) -> Option<String>,
}

/// The two requests a link joins, as a coupling's conditions read them: the
/// upstream run_trajectory request and the downstream request, each without
/// its `from`, and whether the downstream piece is one step, whose start
/// fields stand in its `parameters`.
pub(crate) struct Joined<'a> {
    pub upstream: &'a Value,
    pub downstream: &'a Value,
    pub step: bool,
}

impl Joined<'_> {
    /// A start field of the upstream run as its request states it.
    fn upstream_start(&self, name: &str) -> Option<f64> {
        self.upstream.get("initial")?.get(name)?.as_f64()
    }

    /// A start field of the downstream piece as its request states it.
    fn downstream_start(&self, name: &str) -> Option<f64> {
        let section = if self.step { "parameters" } else { "initial" };
        self.downstream.get(section)?.get(name)?.as_f64()
    }

    /// The seconds at the end of the upstream run at a current of zero: the
    /// segments at its end whose load holds `current` at zero, their steps
    /// times its step.
    fn upstream_rest_at_end_s(&self, current: &str) -> f64 {
        let dt_s = self.upstream.get("dt_s").and_then(Value::as_f64).unwrap_or(0.0);
        let mut steps = 0.0;
        for segment in self.upstream_rest_segments(current) {
            steps += segment.get("steps").and_then(Value::as_u64).unwrap_or(0) as f64;
        }
        steps * dt_s
    }

    /// The segments at the end of the upstream run whose load holds
    /// `current` at zero, the last first.
    fn upstream_rest_segments<'b>(&'b self, current: &'b str) -> impl Iterator<Item = &'b Value> + 'b {
        let segments = self.upstream.get("segments").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
        segments
            .iter()
            .rev()
            .take_while(move |segment| segment.get("load").and_then(|load| load.get(current)).and_then(Value::as_f64) == Some(0.0))
    }

    /// The `temperature` of each load of the upstream run's last `span_s`
    /// seconds at a current of zero: every segment at its end whose load
    /// holds `current` at zero, the last first, until those seconds are
    /// covered, a segment that reaches into them counted whole. A segment
    /// whose load leaves the temperature out gives none.
    fn upstream_rest_temperatures(&self, current: &str, temperature: &str, span_s: f64) -> Vec<f64> {
        let dt_s = self.upstream.get("dt_s").and_then(Value::as_f64).unwrap_or(0.0);
        let mut covered_s = 0.0;
        let mut temperatures = Vec::new();
        for segment in self.upstream_rest_segments(current) {
            if covered_s >= span_s {
                break;
            }
            if let Some(held) = segment.get("load").and_then(|load| load.get(temperature)).and_then(Value::as_f64) {
                temperatures.push(held);
            }
            covered_s += segment.get("steps").and_then(Value::as_u64).unwrap_or(0) as f64 * dt_s;
        }
        temperatures
    }
}

/// A condition a coupling holds on the two requests it joins, decided before
/// any physics: where it is not met, the values handed do not carry one
/// meaning in both cards.
pub(crate) struct Condition {
    /// What the condition holds, as `describe_card` states it.
    pub says: &'static str,
    /// `Some(what the requests hold instead)` when it is not met. A field it
    /// reads that a request leaves out does not decide it: that request is
    /// refused by name when it is read.
    pub unmet: fn(&Joined) -> Option<String>,
}

pub(crate) struct Coupling {
    pub name: &'static str,
    pub upstream: (&'static str, &'static str),
    pub downstream: (&'static str, &'static str),
    pub mappings: &'static [Mapping],
    /// The conditions on the two requests, each decided before any physics.
    pub conditions: &'static [Condition],
    /// What is handed and why the pairing is served, in one sentence.
    pub meaning: &'static str,
}

impl Coupling {
    /// The first condition the two requests do not meet, as the sentence of
    /// the refusal `CouplingConditionNotMet`; `None` when they meet every one.
    pub(crate) fn unmet(&self, joined: &Joined) -> Option<String> {
        self.conditions.iter().find_map(|condition| {
            (condition.unmet)(joined).map(|instead| {
                format!(
                    "the coupling '{}' joins two runs only where {}; {instead}, and outside its conditions the values it hands do not carry one meaning in both cards, so neither run is computed",
                    self.name, condition.says
                )
            })
        })
    }
}

/// The spellings of a unit that a served coupling carries and that the two
/// cards' descriptions spell otherwise, each with the descriptions'
/// spelling. A coupling's unit is in every chain seal it gives
/// (`handed[].unit`), so a spelling once served stays; a new coupling spells
/// its units as the descriptions do. The table is static, so its test,
/// `every_unit_is_the_unit_both_cards_give_the_value`, is its registration.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const UNIT_SPELLINGS: &[(&str, &str)] = &[
    // The SPERT III couplings since engine 0.8.0; both cards' descriptions
    // give the reactivity in "dollar".
    ("dollars", "dollar"),
];

/// A coupling's unit as the cards' descriptions spell it.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn unit_as_described(unit: &str) -> &str {
    UNIT_SPELLINGS
        .iter()
        .find(|(served, _)| *served == unit)
        .map_or(unit, |(_, described)| *described)
}

fn outside_span(value: f64, min: f64, max: f64, unit: &str, what: &str) -> Option<String> {
    if value >= min && value <= max {
        None
    } else {
        Some(format!("{value} {unit} is outside {what}, {min} to {max} {unit}"))
    }
}

fn spert3_coolant(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    outside_span(
        value,
        reactor_spert3::WATER_FREEZING_K,
        reactor_spert3::WATER_CRITICAL_K,
        "K",
        "reactor_spert3's declared coolant temperature, liquid water",
    )
}

/// A reactivity the card takes for the step in reactivity: above zero.
fn above_zero(value: f64, what: &str) -> Option<String> {
    if value > 0.0 {
        None
    } else {
        Some(format!("{value} dollars is not above zero, {what}"))
    }
}

fn spert3_reactivity(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    above_zero(value, "reactor_spert3's declared reactivity of the step in reactivity")
}

fn fuel_band(inlet_k: f64) -> Option<usize> {
    (0..3).find(|&band| {
        inlet_k >= reactor_spert3_fuel::BAND_INLET_MIN_K[band] && inlet_k <= reactor_spert3_fuel::BAND_INLET_MAX_K[band]
    })
}

fn fuel_inlet(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    match fuel_band(value) {
        Some(_) => None,
        None => Some(format!(
            "{value} K is in none of reactor_spert3_fuel's three inlet bands, {:?} to {:?} K",
            reactor_spert3_fuel::BAND_INLET_MIN_K,
            reactor_spert3_fuel::BAND_INLET_MAX_K
        )),
    }
}

fn fuel_reactivity(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    above_zero(value, "reactor_spert3_fuel's declared reactivity")
}

/// Cell m80 in each card: the 40 degC block of cell_2170_m80, the first of
/// its six; the 40 degC block of cell_2170_model3, the last of its five; and
/// cell m80, the first row of cell_2170_model3's cells.
const M80_BLOCK_40: usize = 0;
#[cfg(test)]
const MODEL3_BLOCK_40: usize = 4;
const MODEL3_CELL_M80: usize = 0;

/// An hour at zero current: the rest the dataset's pulse test gives before
/// each charge level, and the rests after which cell_2170_m80's rest-voltage
/// tables were read. Each card starts a cell at rest, so the upstream run
/// ends with such an hour, at the case temperatures of
/// [`m80_block_40_case_c`].
pub(crate) const CELL_RESTED_S: f64 = 3_600.0;

/// The case temperatures the upstream's hour of rest is held to: those of
/// the compared tests of cell_2170_m80's 40 degC block, 39.063 to 45.535
/// degC. An hour is a rest only where the cell's memories fade inside it,
/// and how fast they fade hangs on the case temperature. By
/// cell_2170_model3's own constants the slowest diffusion mode takes 34 s at
/// 39.063 degC and the electrolyte drop 108 s, so an hour there leaves under
/// 1e-14 of either; at -12.9 degC, the coldest case that card was compared
/// at, they take 20.6 and 12.8 minutes, and an hour leaves 5.4 and 0.94
/// percent of them (an hour after 1C, 8.8 mV at the terminal and 0.85
/// percent of capacity between the surface and the bulk). cell_2170_m80's
/// relaxing branch reads its time constant at the case temperature too, and
/// its tables of the 40 degC block were read from tests in this band. The
/// test `the_40_degc_block_s_rest_leaves_no_memory_and_a_cold_hour_does`
/// holds the numbers to the kernel.
fn m80_block_40_case_c() -> (f64, f64) {
    (cell_2170_m80::BLOCK_CASE_MIN_C[M80_BLOCK_40], cell_2170_m80::BLOCK_CASE_MAX_C[M80_BLOCK_40])
}

fn m80_cell_number() -> f64 {
    cell_2170_model3::CELLS[MODEL3_CELL_M80].cell_number
}

fn m80_block_40_c() -> f64 {
    cell_2170_m80::BLOCK_CHAMBER_C[M80_BLOCK_40]
}

/// The capacity of cell m80 in its 40 degC block, the same 64-bit number in
/// both cards: cell_2170_m80's capacity of the block and cell_2170_model3's
/// reference capacity of the cell, each the ampere-hours of the block's
/// capacity test, which began at the laboratory's full recharge.
fn m80_capacity_40_ah() -> f64 {
    cell_2170_m80::CAPACITY_AH[M80_BLOCK_40]
}

fn stated(value: f64, name: &str, side: &str) -> String {
    format!("the {side} request states {name} {value}")
}

fn upstream_cell_is_m80(joined: &Joined) -> Option<String> {
    let cell = joined.upstream_start("cell_number")?;
    (cell != m80_cell_number()).then(|| stated(cell, "cell_number", "upstream"))
}

fn downstream_cell_is_m80(joined: &Joined) -> Option<String> {
    let cell = joined.downstream_start("cell_number")?;
    (cell != m80_cell_number()).then(|| stated(cell, "cell_number", "downstream"))
}

fn upstream_capacity_of_block_40(joined: &Joined) -> Option<String> {
    let capacity = joined.upstream_start("capacity_ah")?;
    (capacity.to_bits() != m80_capacity_40_ah().to_bits()).then(|| stated(capacity, "capacity_ah", "upstream"))
}

fn downstream_capacity_of_block_40(joined: &Joined) -> Option<String> {
    let capacity = joined.downstream_start("capacity_ah")?;
    (capacity.to_bits() != m80_capacity_40_ah().to_bits()).then(|| stated(capacity, "capacity_ah", "downstream"))
}

fn upstream_in_block_40(joined: &Joined) -> Option<String> {
    let block = joined.upstream_start("block_chamber_c")?;
    (block != m80_block_40_c()).then(|| stated(block, "block_chamber_c", "upstream"))
}

fn downstream_in_block_40(joined: &Joined) -> Option<String> {
    let block = joined.downstream_start("block_chamber_c")?;
    (block != m80_block_40_c()).then(|| stated(block, "block_chamber_c", "downstream"))
}

fn upstream_ends_rested(joined: &Joined) -> Option<String> {
    let rested_s = joined.upstream_rest_at_end_s("current_a");
    (rested_s < CELL_RESTED_S).then(|| format!("the upstream run ends with {rested_s} s at a current_a of zero"))
}

fn upstream_rests_in_block_40(joined: &Joined) -> Option<String> {
    let (least, most) = m80_block_40_case_c();
    joined
        .upstream_rest_temperatures("current_a", "case_temperature_c", CELL_RESTED_S)
        .into_iter()
        .find(|held| !(*held >= least && *held <= most))
        .map(|held| format!("the upstream request holds a case_temperature_c of {held} in its last {CELL_RESTED_S} s at a current_a of zero"))
}

fn m80_block_40_charge(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    outside_span(
        value,
        cell_2170_m80::CHARGE_TOP_AH[M80_BLOCK_40],
        cell_2170_m80::CHARGE_DEEP_AH[M80_BLOCK_40],
        "Ah",
        "the tables of cell_2170_m80's 40 degC block",
    )
}

fn model3_start_charge(value: f64, _: &[(&'static str, f64)]) -> Option<String> {
    let (first, last) = (cell_2170_model3::CURVE_FIRST_STATE_OF_CHARGE, cell_2170_model3::CURVE_LAST_STATE_OF_CHARGE);
    if value >= first && value <= last {
        None
    } else {
        Some(format!(
            "a state of charge of {value} is outside the ends of cell_2170_model3's rest-voltage curves, {first} to {last}"
        ))
    }
}

const UPSTREAM_CELL_M80: Condition = Condition {
    says: "the upstream cell is m80, cell_number 80",
    unmet: upstream_cell_is_m80,
};
const DOWNSTREAM_CELL_M80: Condition = Condition {
    says: "the downstream cell is m80, cell_number 80",
    unmet: downstream_cell_is_m80,
};
const UPSTREAM_ENDS_RESTED: Condition = Condition {
    says: "the upstream run ends with at least 3600 s at a current_a of zero, a cell at rest",
    unmet: upstream_ends_rested,
};
const UPSTREAM_RESTS_IN_BLOCK_40: Condition = Condition {
    says: "the upstream run's last 3600 s at a current_a of zero hold a case_temperature_c from 39.063 to 45.535 degC, the case temperatures of the 40 degC block's compared tests: there an hour leaves under 1e-14 of the cell's slowest diffusion mode and of its electrolyte drop, and colder they outlast an hour",
    unmet: upstream_rests_in_block_40,
};

pub(crate) static COUPLINGS: &[Coupling] = &[
    Coupling {
        name: "spert3_burst_to_fuel_card",
        upstream: ("energy", "reactor_spert3"),
        downstream: ("energy", "reactor_spert3_fuel"),
        mappings: &[
            Mapping {
                from: "final_state.coolant_temperature_k",
                to: "initial.inlet_temperature_k",
                unit: "K",
                outside: fuel_inlet,
            },
            Mapping {
                from: "inputs.segments[*].load.reactivity_inserted_dollar",
                to: "segments[*].load.reactivity_inserted_dollar",
                unit: "dollars",
                outside: fuel_reactivity,
            },
        ],
        conditions: &[],
        meaning: "One SPERT III E-core burst (IDO-17281) handed from the point-kinetics card to the card with fuel and water temperature: the water temperature the burst started at and the reactivity inserted, each carried bit for bit. Both cards were compared with the same report's tests; the coupling carries the test, not a computed field. The reactivity is taken from the upstream request, which holds one value of it in every segment: an upstream burst whose segments insert more than one reactivity is refused as CouplingSourceNotOneValue, decided after the upstream run is computed again and its seal held, before the downstream card runs.",
    },
    Coupling {
        name: "spert3_fuel_burst_to_point_card",
        upstream: ("energy", "reactor_spert3_fuel"),
        downstream: ("energy", "reactor_spert3"),
        mappings: &[
            Mapping {
                from: "final_state.inlet_temperature_k",
                to: "initial.coolant_temperature_k",
                unit: "K",
                outside: spert3_coolant,
            },
            Mapping {
                from: "inputs.segments[*].load.reactivity_inserted_dollar",
                to: "segments[*].load.reactivity_inserted_dollar",
                unit: "dollars",
                outside: spert3_reactivity,
            },
        ],
        conditions: &[],
        meaning: "One SPERT III E-core burst (IDO-17281) handed from the card with fuel and water temperature to the point-kinetics card: the inlet water temperature and the reactivity inserted, each carried bit for bit. It carries the test, not a computed field: the inlet temperature is the start value, held unchanged through the run. The point-kinetics card was compared on the cold-startup tests only, at a primary coolant of 56 to 92 degF; its feedback does not depend on the coolant temperature, so a hot-startup burst handed to it is the same burst it gives cold, and nothing was compared there. The reactivity is taken from the upstream request, which holds one value of it in every segment: an upstream burst whose segments insert more than one reactivity is refused as CouplingSourceNotOneValue, decided after the upstream run is computed again and its seal held, before the downstream card runs.",
    },
    Coupling {
        name: "cell_2170_model3_to_m80",
        upstream: ("energy", "cell_2170_model3"),
        downstream: ("energy", "cell_2170_m80"),
        mappings: &[Mapping {
            from: "final_state.charge_removed_ah",
            to: "initial.charge_removed_ah",
            unit: "Ah",
            outside: m80_block_40_charge,
        }],
        conditions: &[
            UPSTREAM_CELL_M80,
            Condition {
                says: "the upstream capacity_ah is 4.67683635335689 Ah, the capacity of the 40 degC block, the same 64-bit number",
                unmet: upstream_capacity_of_block_40,
            },
            Condition {
                says: "the downstream run is in the 40 degC block, block_chamber_c 40",
                unmet: downstream_in_block_40,
            },
            UPSTREAM_ENDS_RESTED,
            UPSTREAM_RESTS_IN_BLOCK_40,
        ],
        meaning: "Cell m80 of the Tesla Model 3 2170 dataset (Kollmeyer et al., Borealis, doi 10.5683/SP3/ZVTR4B), rested in its 40 degC block, handed from the physical model to the table card: the charge removed since the laboratory's full recharge, 4.2 V held at 25 degC until the current falls to about 0.02 A, in Ah, carried bit for bit. Both cards were compared with this cell's tests and both count the charge removed from that full. In the 40 degC block the capacity test that fixes the table card's capacity and the shape of its rest-voltage table began at that full, and its capacity, 4.67683635335689 Ah, is the physical model's reference capacity of cell m80, the same 64-bit number; in each of the other five blocks that test began from a top-up 22 to 52 mAh above full, so the table card counts its charge there from another start, and the coupling joins only the 40 degC block. The physical model is given that capacity, so the two cards hold the cell with one capacity, one full and one charge level; the table card holds one capacity for the block and does not follow the cell's ageing inside it. The table card starts a cell at rest, nothing held by its relaxing branch, so the upstream run ends with an hour at zero current, held at the case temperatures of the 40 degC block's compared tests, 39.063 to 45.535 degC, where by the physical model's own constants the slowest diffusion mode takes 34 s or less and the electrolyte drop 108 s or less; at -12.9 degC they take 21 and 13 minutes, and an hour leaves 5.4 percent of the slowest mode. The current is not handed, since the table card takes a charging current as positive and the physical model a discharging one; nor is the case temperature, which each card is given as measured at every step.",
    },
    Coupling {
        name: "cell_2170_m80_to_model3",
        upstream: ("energy", "cell_2170_m80"),
        downstream: ("energy", "cell_2170_model3"),
        mappings: &[Mapping {
            from: "final_state.charge_level",
            to: "initial.state_of_charge",
            unit: "1",
            outside: model3_start_charge,
        }],
        conditions: &[
            Condition {
                says: "the upstream run is in the 40 degC block, block_chamber_c 40",
                unmet: upstream_in_block_40,
            },
            UPSTREAM_ENDS_RESTED,
            UPSTREAM_RESTS_IN_BLOCK_40,
            DOWNSTREAM_CELL_M80,
            Condition {
                says: "the downstream capacity_ah is 4.67683635335689 Ah, the capacity of the 40 degC block, the same 64-bit number",
                unmet: downstream_capacity_of_block_40,
            },
        ],
        meaning: "Cell m80 of the Tesla Model 3 2170 dataset (Kollmeyer et al., Borealis, doi 10.5683/SP3/ZVTR4B), rested in its 40 degC block, handed from the table card to the physical model: the table card's charge level, carried bit for bit as the physical model's state of charge. In the 40 degC block the charge level is 1 less the charge removed since the laboratory's full recharge over 4.67683635335689 Ah, the capacity of the block's capacity test, which began at that full; the state of charge is 1 at the same full and falls by the charge removed over the capacity the physical model is given. The two are one quantity only where the physical model is given cell m80 and that capacity, its reference capacity of the cell; in each of the other five blocks the table card's capacity was read from a test that began 22 to 52 mAh above full, and the coupling does not join them. The physical model starts a cell at rest, every memory at zero, so the upstream run ends with an hour at zero current, held at the case temperatures of the 40 degC block's compared tests, 39.063 to 45.535 degC, where by the physical model's own constants the slowest diffusion mode takes 34 s or less and the electrolyte drop 108 s or less; at -12.9 degC they take 21 and 13 minutes, and an hour leaves 5.4 percent of the slowest mode. The current is not handed, since the two cards take its sign opposite ways; nor is the case temperature, which each card is given as measured at every step.",
    },
];

pub(crate) fn find(name: &str) -> Option<&'static Coupling> {
    COUPLINGS.iter().find(|coupling| coupling.name == name)
}

/// The couplings a card hands from or takes, as `describe_card` states
/// them under `couplings`: only those whose two cards the view holds, so a
/// card outside the view is not named.
pub(crate) fn table_in(view: &crate::view::View, domain: &str, card: &str) -> Value {
    Value::Array(
        COUPLINGS
            .iter()
            .filter(|coupling| coupling.upstream == (domain, card) || coupling.downstream == (domain, card))
            .filter(|coupling| {
                view.find(coupling.upstream.0, coupling.upstream.1).is_some()
                    && view.find(coupling.downstream.0, coupling.downstream.1).is_some()
            })
            .map(|coupling| {
                let mut entry = serde_json::Map::new();
                entry.insert("name".into(), coupling.name.into());
                entry.insert("upstream".into(), format!("{}/{}", coupling.upstream.0, coupling.upstream.1).into());
                entry.insert("downstream".into(), format!("{}/{}", coupling.downstream.0, coupling.downstream.1).into());
                entry.insert(
                    "handed".into(),
                    coupling
                        .mappings
                        .iter()
                        .map(|mapping| serde_json::json!({ "from": mapping.from, "to": mapping.to, "unit": mapping.unit }))
                        .collect::<Vec<_>>()
                        .into(),
                );
                // A coupling with no condition is listed as it was before conditions were served.
                if !coupling.conditions.is_empty() {
                    entry.insert(
                        "conditions".into(),
                        coupling.conditions.iter().map(|condition| Value::from(condition.says)).collect::<Vec<_>>().into(),
                    );
                }
                entry.insert("meaning".into(), coupling.meaning.into());
                Value::Object(entry)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::descriptor::Role;

    /// The field of a card's description with this name, and its role.
    fn field<'a>(card: &'a crate::descriptor::Card, name: &str) -> &'a crate::descriptor::FieldSpec {
        card.fields.iter().find(|field| field.name == name).unwrap_or_else(|| panic!("{}: no field {name}", card.card))
    }

    /// Every coupling names two cards the engine serves; every source names
    /// a value the upstream card gives (a number of its published state, a
    /// start field or a load of its request) and every place a field of the
    /// downstream card of the kind its path declares (a start field at
    /// `initial.`, a load at `segments[*].load.`, and either among a step's
    /// parameters).
    #[test]
    fn every_coupling_names_cards_the_engine_serves_and_fields_they_take() {
        for coupling in COUPLINGS {
            let up = crate::descriptor::find(coupling.upstream.0, coupling.upstream.1).expect("upstream card");
            let down = crate::descriptor::find(coupling.downstream.0, coupling.downstream.1).expect("downstream card");
            let published = crate::state_fields(up.domain, up.card).expect("the upstream publishes a state");
            for mapping in coupling.mappings {
                if let Some(name) = mapping.from.strip_prefix("inputs.initial.") {
                    assert!(up.phase_names.contains(&name), "{}", mapping.from);
                    assert!(field(up, name).role == Role::Phase, "{}", mapping.from);
                } else if let Some(name) = mapping.from.strip_prefix("inputs.segments[*].load.") {
                    assert!(up.load_names.contains(&name), "{}", mapping.from);
                    assert!(field(up, name).role == Role::Load, "{}", mapping.from);
                } else {
                    let name = mapping.from.strip_prefix("final_state.").unwrap_or_else(|| panic!("{}", mapping.from));
                    assert!(
                        published.iter().any(|(field, kind)| field == name && *kind == crate::QuantityKind::Number),
                        "{}: {} publishes no number {name}",
                        coupling.name,
                        up.card
                    );
                }
                let name = if let Some(name) = mapping.to.strip_prefix("initial.") {
                    assert!(down.phase_names.contains(&name), "{}", mapping.to);
                    assert!(field(down, name).role == Role::Phase, "{}", mapping.to);
                    name
                } else {
                    let name = mapping.to.strip_prefix("segments[*].load.").expect("a load path");
                    assert!(down.load_names.contains(&name), "{}", mapping.to);
                    assert!(field(down, name).role == Role::Load, "{}", mapping.to);
                    name
                };
                // A step takes the value among its parameters.
                assert!(down.parameter_names.contains(&name), "{} parameters.{name}", coupling.name);
                // One number is handed into one number.
                assert_eq!(field(down, name).components, 1, "{}", mapping.to);
            }
            assert_eq!(COUPLINGS.iter().filter(|other| other.name == coupling.name).count(), 1);
        }
        let names: Vec<&str> = COUPLINGS.iter().map(|coupling| coupling.name).collect();
        assert_eq!(
            names,
            ["spert3_burst_to_fuel_card", "spert3_fuel_burst_to_point_card", "cell_2170_model3_to_m80", "cell_2170_m80_to_model3"]
        );
    }

    /// The unit of a value the upstream card gives: the unit its description
    /// gives a start field or a load, or the unit of a published number (a
    /// number published under a start field's name has that field's unit).
    fn source_unit(card: &crate::descriptor::Card, from: &str) -> &'static str {
        let name = from
            .strip_prefix("inputs.initial.")
            .or_else(|| from.strip_prefix("inputs.segments[*].load."))
            .or_else(|| from.strip_prefix("final_state."))
            .unwrap_or_else(|| panic!("{from}"));
        match card.fields.iter().find(|field| field.name == name) {
            Some(field) => field.unit,
            None => crate::descriptor::unit_of_known(name).unwrap_or_else(|| panic!("{}: no unit for {name}", card.card)),
        }
    }

    /// Each coupling's unit is the unit both cards' descriptions give the
    /// value, as they spell it, or a spelling in UNIT_SPELLINGS, each of
    /// which a served coupling carries.
    #[test]
    fn every_unit_is_the_unit_both_cards_give_the_value() {
        for coupling in COUPLINGS {
            let up = crate::descriptor::find(coupling.upstream.0, coupling.upstream.1).unwrap();
            let down = crate::descriptor::find(coupling.downstream.0, coupling.downstream.1).unwrap();
            for mapping in coupling.mappings {
                let to = mapping.to.strip_prefix("initial.").or_else(|| mapping.to.strip_prefix("segments[*].load.")).unwrap();
                let described = unit_as_described(mapping.unit);
                assert_eq!(described, source_unit(up, mapping.from), "{} {}", coupling.name, mapping.from);
                assert_eq!(described, field(down, to).unit, "{} {}", coupling.name, mapping.to);
            }
        }
        // Every other spelling is served, and none is a descriptions' own.
        for (served, described) in UNIT_SPELLINGS {
            assert_ne!(served, described);
            assert!(COUPLINGS.iter().flat_map(|coupling| coupling.mappings).any(|mapping| mapping.unit == *served), "{served}");
        }
        assert_eq!(unit_as_described("dollars"), "dollar");
        assert_eq!(unit_as_described("Ah"), "Ah");
    }

    fn joined<'a>(upstream: &'a Value, downstream: &'a Value, step: bool) -> Joined<'a> {
        Joined { upstream, downstream, step }
    }

    /// What the cell m80 couplings rest on, held to the kernel: the 40 degC
    /// block in both cards, cell m80 in the physical model, and one capacity
    /// of the block, the same 64-bit number in both; the table card's tables
    /// of that block reach its capacity.
    #[test]
    fn cell_m80_in_its_40_degc_block_is_one_capacity_in_both_cards() {
        assert_eq!(cell_2170_m80::BLOCK_CHAMBER_C[M80_BLOCK_40], 40.0);
        assert_eq!(cell_2170_model3::BLOCK_CHAMBER_C[MODEL3_BLOCK_40], 40.0);
        let m80 = cell_2170_model3::CELLS[MODEL3_CELL_M80];
        assert_eq!((m80.name, m80.cell_number), ("m80", 80.0));
        assert_eq!(m80.reference_capacity_ah.to_bits(), cell_2170_m80::CAPACITY_AH[M80_BLOCK_40].to_bits());
        assert_eq!(cell_2170_m80::CHARGE_DEEP_AH[M80_BLOCK_40].to_bits(), cell_2170_m80::CAPACITY_AH[M80_BLOCK_40].to_bits());
        // The capacity is one the physical model compared in its 40 degC block.
        let least = cell_2170_model3::BLOCK_CAPACITY_LEAST_AH[MODEL3_BLOCK_40];
        let most = cell_2170_model3::BLOCK_CAPACITY_MOST_AH[MODEL3_BLOCK_40];
        assert!(least <= m80_capacity_40_ah() && m80_capacity_40_ah() <= most);
        // In every other block the table card's capacity is not the physical
        // model's reference capacity: those capacity tests began above full.
        for block in 1..cell_2170_m80::BLOCK_COUNT {
            assert!(cell_2170_m80::CAPACITY_AH[block] < m80.reference_capacity_ah, "{block}");
        }
        // The conditions say the numbers they hold.
        let says = |name: &str, index: usize| find(name).unwrap().conditions[index].says;
        assert!(says("cell_2170_model3_to_m80", 1).contains(&format!("{} Ah", m80_capacity_40_ah())));
        assert!(says("cell_2170_model3_to_m80", 2).contains(&format!("block_chamber_c {}", m80_block_40_c())));
        assert!(says("cell_2170_model3_to_m80", 3).contains(&format!("at least {CELL_RESTED_S} s")));
        let (least, most) = m80_block_40_case_c();
        assert_eq!((least, most), (39.063, 45.535));
        let band = format!("last {CELL_RESTED_S} s at a current_a of zero hold a case_temperature_c from {least} to {most} degC");
        assert!(says("cell_2170_model3_to_m80", 4).contains(&band));
        assert!(says("cell_2170_m80_to_model3", 1).contains(&format!("at least {CELL_RESTED_S} s")));
        assert!(says("cell_2170_m80_to_model3", 2).contains(&band));
        assert!(says("cell_2170_m80_to_model3", 4).contains(&format!("{} Ah", m80_capacity_40_ah())));
        assert!(says("cell_2170_m80_to_model3", 3).contains(&format!("cell_number {}", m80_cell_number())));
        for name in ["cell_2170_model3_to_m80", "cell_2170_m80_to_model3"] {
            let meaning = find(name).unwrap().meaning;
            assert!(meaning.contains(&format!("{} Ah", m80_capacity_40_ah())), "{name}");
            assert!(meaning.contains(&format!("{least} to {most} degC")), "{name}");
        }
    }

    /// The time of a memory of cell_2170_model3 at a case temperature, by
    /// the card's own law: its time at 25 degC times exp(activation (1/T -
    /// 1/298.15 K)).
    fn model3_time_s(time_at_25_c_s: f64, activation_k: f64, case_c: f64) -> f64 {
        let x = 1.0 / (case_c + cell_2170_model3::KELVIN_OFFSET) - 1.0 / cell_2170_model3::REFERENCE_TEMPERATURE_K;
        time_at_25_c_s * (activation_k * x).exp()
    }

    /// Why the hour of rest is held to the 40 degC block's case
    /// temperatures, from the kernel's constants: there an hour leaves under
    /// 1e-14 of cell_2170_model3's slowest diffusion mode and of its
    /// electrolyte drop, whose times fall as the case warms; at -12.9 degC,
    /// the coldest case that card was compared at, an hour leaves 5.4
    /// percent of the slowest mode and 0.94 percent of the drop. The numbers
    /// are the ones the doc of m80_block_40_case_c and the couplings'
    /// meanings give.
    #[test]
    fn the_40_degc_block_s_rest_leaves_no_memory_and_a_cold_hour_does() {
        let slowest = |case_c: f64| {
            model3_time_s(
                cell_2170_model3::DIFFUSION_TIME_S * cell_2170_model3::DIFFUSION_MODE_TIME_FRACTION[0],
                cell_2170_model3::DIFFUSION_ACTIVATION_K,
                case_c,
            )
        };
        let drop = |case_c: f64| model3_time_s(cell_2170_model3::ELECTROLYTE_TIME_S, cell_2170_model3::ELECTROLYTE_ACTIVATION_K, case_c);
        // The slowest mode is the first.
        for fraction in &cell_2170_model3::DIFFUSION_MODE_TIME_FRACTION[1..] {
            assert!(*fraction < cell_2170_model3::DIFFUSION_MODE_TIME_FRACTION[0]);
        }
        let left = |time_s: f64| (-CELL_RESTED_S / time_s).exp();
        let (least, most) = m80_block_40_case_c();
        assert!(slowest(least) > slowest(most) && drop(least) > drop(most));
        assert_eq!((slowest(least).round(), drop(least).round()), (34.0, 108.0));
        assert!(left(slowest(least)) < 1e-14 && left(drop(least)) < 1e-14);
        // The band is inside the physical model's 40 degC block and its compared case temperatures.
        assert!(cell_2170_model3::BLOCK_CASE_TEMPERATURE_LEAST_C[MODEL3_BLOCK_40] <= least);
        assert!(most <= cell_2170_model3::BLOCK_CASE_TEMPERATURE_MOST_C[MODEL3_BLOCK_40]);
        // At -12.9 degC: 20.6 and 12.8 minutes, and an hour leaves 5.4 and 0.94 percent.
        let cold = cell_2170_model3::CASE_TEMPERATURE_MIN_C;
        assert_eq!(cold, -12.9);
        assert_eq!(((slowest(cold) / 6.0).round() / 10.0, (drop(cold) / 6.0).round() / 10.0), (20.6, 12.8));
        assert_eq!(((left(slowest(cold)) * 1000.0).round() / 10.0, (left(drop(cold)) * 10_000.0).round() / 100.0), (5.4, 0.94));
    }

    /// Each condition is met by the requests that meet it and named on any
    /// that does not; a field a request leaves out decides nothing.
    #[test]
    fn each_condition_holds_its_requests_and_names_the_others() {
        let model3 = |cell: f64, capacity: f64, rest: u64| {
            serde_json::json!({
                "domain": "energy", "card": "cell_2170_model3",
                "initial": { "cell_number": cell, "state_of_charge": 1.0, "capacity_ah": capacity, "case_temperature_least_c": 40.0, "case_temperature_most_c": 40.0 },
                "dt_s": 1.0,
                "segments": [
                    { "steps": 600, "load": { "current_a": 4.5, "case_temperature_c": 40.0 } },
                    { "steps": rest, "load": { "current_a": 0.0, "case_temperature_c": 40.0 } }
                ],
                "record_every": 600
            })
        };
        let m80 = |block: f64, rest: u64| {
            serde_json::json!({
                "domain": "energy", "card": "cell_2170_m80",
                "initial": { "block_chamber_c": block, "charge_removed_ah": 0.5 },
                "dt_s": 1.0,
                "segments": [
                    { "steps": 600, "load": { "current_a": -4.5, "case_temperature_c": 40.0 } },
                    { "steps": rest, "load": { "current_a": 0.0, "case_temperature_c": 40.0 } }
                ],
                "record_every": 600
            })
        };
        let forward = find("cell_2170_model3_to_m80").unwrap();
        let reverse = find("cell_2170_m80_to_model3").unwrap();
        let named = |coupling: &Coupling, up: &Value, down: &Value, step: bool| coupling.unmet(&joined(up, down, step));
        let capacity = m80_capacity_40_ah();
        // Met.
        for step in [false, true] {
            let (up, mut down) = (model3(80.0, capacity, 3600), m80(40.0, 0));
            if step {
                down = serde_json::json!({ "domain": "energy", "card": "cell_2170_m80", "parameters": down["initial"] });
            }
            assert_eq!(named(forward, &up, &down, step), None);
            let (up, mut down) = (m80(40.0, 3600), model3(80.0, capacity, 0));
            if step {
                down = serde_json::json!({ "domain": "energy", "card": "cell_2170_model3", "parameters": down["initial"] });
            }
            assert_eq!(named(reverse, &up, &down, step), None);
        }
        // An hour in two segments at the end is an hour.
        let mut split = model3(80.0, capacity, 1800);
        split["segments"].as_array_mut().unwrap().push(serde_json::json!({ "steps": 1800, "load": { "current_a": 0.0, "case_temperature_c": 45.535 } }));
        assert_eq!(named(forward, &split, &m80(40.0, 0), false), None);
        // The rest's case temperature: the band's two ends are inside it.
        let rest_at = |mut request: Value, case_c: f64| {
            let segments = request["segments"].as_array_mut().unwrap();
            segments.last_mut().unwrap()["load"]["case_temperature_c"] = serde_json::json!(case_c);
            request
        };
        for case_c in [39.063, 40.0, 45.535] {
            assert_eq!(named(forward, &rest_at(model3(80.0, capacity, 3600), case_c), &m80(40.0, 0), false), None, "{case_c}");
            assert_eq!(named(reverse, &rest_at(m80(40.0, 3600), case_c), &model3(80.0, capacity, 0), false), None, "{case_c}");
        }
        // A cold rest before the last hour is not in it; an hour's rest
        // whose last half hour is cold is not a rest in the band.
        let mut warmed = rest_at(model3(80.0, capacity, 1800), -12.9);
        warmed["segments"].as_array_mut().unwrap().push(serde_json::json!({ "steps": 3600, "load": { "current_a": 0.0, "case_temperature_c": 40.0 } }));
        assert_eq!(named(forward, &warmed, &m80(40.0, 0), false), None);
        let mut cooled = model3(80.0, capacity, 3600);
        cooled["segments"].as_array_mut().unwrap().push(serde_json::json!({ "steps": 1800, "load": { "current_a": 0.0, "case_temperature_c": -12.9 } }));
        assert!(named(forward, &cooled, &m80(40.0, 0), false).unwrap().contains("holds a case_temperature_c of -12.9 in its last 3600 s"));
        // A segment that reaches into the last hour is counted whole.
        let mut reaching = rest_at(model3(80.0, capacity, 1800), 25.0);
        reaching["segments"].as_array_mut().unwrap().push(serde_json::json!({ "steps": 3599, "load": { "current_a": 0.0, "case_temperature_c": 40.0 } }));
        assert!(named(forward, &reaching, &m80(40.0, 0), false).unwrap().contains("holds a case_temperature_c of 25 in its last 3600 s"));
        // Not met, each named with what the request holds.
        let cases: [(&Coupling, Value, Value, &str); 15] = [
            (forward, model3(1000.0, capacity, 3600), m80(40.0, 0), "the upstream request states cell_number 1000"),
            (forward, model3(80.0, 4.6768, 3600), m80(40.0, 0), "the upstream request states capacity_ah 4.6768"),
            (forward, model3(80.0, capacity.next_down(), 3600), m80(40.0, 0), "the upstream request states capacity_ah 4.676836353356889"),
            (forward, model3(80.0, capacity, 3600), m80(25.0, 0), "the downstream request states block_chamber_c 25"),
            (forward, model3(80.0, capacity, 3599), m80(40.0, 0), "the upstream run ends with 3599 s at a current_a of zero"),
            (reverse, m80(25.0, 3600), model3(80.0, capacity, 0), "the upstream request states block_chamber_c 25"),
            (reverse, m80(40.0, 0), model3(80.0, capacity, 0), "the upstream run ends with 0 s at a current_a of zero"),
            (reverse, m80(40.0, 3600), model3(448.0, capacity, 0), "the downstream request states cell_number 448"),
            (reverse, m80(40.0, 3600), model3(80.0, 4.6768, 0), "the downstream request states capacity_ah 4.6768"),
            // A cold rest, and one past the block's warmest compared case.
            (forward, rest_at(model3(80.0, capacity, 3600), -12.9), m80(40.0, 0), "the upstream request holds a case_temperature_c of -12.9 in its last 3600 s at a current_a of zero"),
            (forward, rest_at(model3(80.0, capacity, 3600), 25.0), m80(40.0, 0), "the upstream request holds a case_temperature_c of 25 in its last 3600 s"),
            (forward, rest_at(model3(80.0, capacity, 3600), 39.063_f64.next_down()), m80(40.0, 0), "the upstream request holds a case_temperature_c of 39.062999999999995"),
            (reverse, rest_at(m80(40.0, 3600), -20.0), model3(80.0, capacity, 0), "the upstream request holds a case_temperature_c of -20 in its last 3600 s at a current_a of zero"),
            (reverse, rest_at(m80(40.0, 3600), 25.0), model3(80.0, capacity, 0), "the upstream request holds a case_temperature_c of 25 in its last 3600 s"),
            (reverse, rest_at(m80(40.0, 3600), 45.535_f64.next_up()), model3(80.0, capacity, 0), "the upstream request holds a case_temperature_c of 45.535000000000004"),
        ];
        for (coupling, up, down, instead) in cases {
            let sentence = named(coupling, &up, &down, false).unwrap_or_else(|| panic!("{instead}"));
            assert!(sentence.contains(instead), "{sentence}");
            assert!(sentence.starts_with(&format!("the coupling '{}' joins two runs only where ", coupling.name)), "{sentence}");
        }
        // A rest that is not at the end is no rest at the end.
        let mut early = model3(80.0, capacity, 3600);
        early["segments"].as_array_mut().unwrap().push(serde_json::json!({ "steps": 1, "load": { "current_a": 0.5, "case_temperature_c": 40.0 } }));
        assert!(named(forward, &early, &m80(40.0, 0), false).unwrap().contains("ends with 0 s"));
        // A field left out decides nothing: the request is refused for it when it is read.
        let mut bare = m80(40.0, 0);
        bare["initial"].as_object_mut().unwrap().shift_remove("block_chamber_c");
        assert_eq!(named(forward, &model3(80.0, capacity, 3600), &bare, false), None);
        // The SPERT III couplings hold no condition. Their one refusal of a
        // reactivity that is not one value is named where they are
        // described, with when it is decided: after the upstream run.
        for name in ["spert3_burst_to_fuel_card", "spert3_fuel_burst_to_point_card"] {
            assert!(find(name).unwrap().conditions.is_empty(), "{name}");
            let meaning = find(name).unwrap().meaning;
            assert!(meaning.contains("refused as CouplingSourceNotOneValue, decided after the upstream run is computed again and its seal held, before the downstream card runs"), "{meaning}");
        }
    }

    #[test]
    fn a_cell_value_outside_the_downstream_range_is_named() {
        assert!(m80_block_40_charge(cell_2170_m80::CHARGE_TOP_AH[0], &[]).is_none());
        assert!(m80_block_40_charge(cell_2170_m80::CHARGE_DEEP_AH[0], &[]).is_none());
        assert!(m80_block_40_charge(cell_2170_m80::CHARGE_TOP_AH[0].next_down(), &[]).is_some());
        assert!(m80_block_40_charge(cell_2170_m80::CHARGE_DEEP_AH[0].next_up(), &[]).is_some());
        assert!(model3_start_charge(cell_2170_model3::CURVE_LAST_STATE_OF_CHARGE, &[]).is_none());
        assert!(model3_start_charge(cell_2170_model3::CURVE_FIRST_STATE_OF_CHARGE, &[]).is_none());
        let past = model3_start_charge(cell_2170_model3::CURVE_LAST_STATE_OF_CHARGE.next_up(), &[]).unwrap();
        assert!(past.contains("rest-voltage curves, -0.05 to 1.05"), "{past}");
    }

    #[test]
    fn a_value_outside_the_downstream_range_is_named_and_one_inside_is_not() {
        let cold = reactor_spert3::COOLANT_TEMPERATURE_MIN_K;
        assert!(fuel_inlet(cold, &[]).is_none());
        assert!(fuel_inlet(400.0, &[]).is_some(), "between the fuel card's bands");
        // The point card takes any liquid water, the fuel card's hot bands included.
        let hot = reactor_spert3_fuel::BAND_INLET_MIN_K[2];
        assert!(spert3_coolant(hot, &[]).is_none());
        assert!(spert3_coolant(cold, &[]).is_none());
        assert!(spert3_coolant(reactor_spert3::WATER_CRITICAL_K.next_up(), &[]).is_some());
        assert!(spert3_coolant(reactor_spert3::WATER_FREEZING_K.next_down(), &[]).is_some());
        let handed = [("initial.inlet_temperature_k", cold)];
        // Both cards take any reactivity above zero.
        assert!(fuel_reactivity(reactor_spert3::REACTIVITY_MAX_DOLLAR.next_up(), &handed).is_none());
        assert!(fuel_reactivity(0.0, &handed).is_some());
        assert!(spert3_reactivity(reactor_spert3_fuel::BAND_REACTIVITY_MAX_DOLLAR[2], &[]).is_none());
        assert!(spert3_reactivity(-1.0, &[]).is_some());
        // Every tested coolant temperature of reactor_spert3 the fuel card admits.
        for t in [reactor_spert3::COOLANT_TEMPERATURE_MIN_K, reactor_spert3::COOLANT_TEMPERATURE_MAX_K] {
            assert!(fuel_inlet(t, &[]).is_none(), "{t}");
        }
    }
}
