//! The entry for `{{name}}`: {{Title}}, the company's own specimen. The
//! numbers come from the `genesis_solvers` constants and the sentence of each
//! refusal from the solver's source. Stamped from the kit.
//!
//! The description skeleton. Every sentence here is served, so each keeps
//! the rules a description is held to (`tests/conformance_support`): the
//! company's own specimen is named so, never by what it lacks; a range is
//! written in plain words; no word "check"; and where the solver is compared
//! with measurements, `validation` states the range compared on and the
//! result there in numbers.

use genesis_solvers::{{name}}::{self as model, {{Name}}Field, {{Name}}Question, {{Name}}Reject};
use serde_json::{Value, json};

use crate::solvers::{Answer, Given, Input, Kind, Output, Published, Refusal, Refused, Sections, Shape, Solved, Solver};

const fn number(name: &'static str, unit: &'static str, note: &'static str) -> Output {
    Output { name, unit, shape: Shape::Number, each: &[], note, asked_by: None }
}

const fn temperature_end(name: &'static str, bound: &'static str) -> Input {
    Input {
        name,
        kind: Kind::NumberAbove { above: 0.0, maximum: model::MELTING_POINT_K, unit: "K" },
        refusal: None,
        bound,
        only_when: None,
        asked_by_check: false,
    }
}

pub const {{NAME}}: Solver = Solver {
    name: "{{name}}",
    summary: "{{Title}}, the company's own specimen: steady heat conduction along a 0.5 m aluminium rod with its ends held from above 0 K to 933.47 K and heat generated inside it, settled to 1e-13 of its largest temperature: the peak temperature and the heat leaving each end.",
    inputs: &[
        Input { refusal: Some("EndOutsideSolid"), ..temperature_end("left_temperature_k", "The temperature held at the left end, above 0 K and at most 933.47 K, the rod's melting point.") },
        Input { refusal: Some("EndOutsideSolid"), ..temperature_end("right_temperature_k", "The temperature held at the right end, above 0 K and at most 933.47 K, the rod's melting point.") },
        Input {
            name: "heat_generation_w_per_m3",
            kind: Kind::Number { minimum: -1.0e300, maximum: 1.0e300, unit: "W/m^3" },
            refusal: Some("HeatGenerationBeyondArithmetic"),
            bound: "The heat generated in each cubic metre of the rod, held uniform; negative is heat drawn out. Its reach is the field's: a field above the melting point or at absolute zero is refused by name.",
            only_when: None,
            asked_by_check: false,
        },
    ],
    kept_inside: "The rod, its conductivity, the grid and the tolerances stay inside the solver.",
    result: &[
        number("peak_temperature_k", "K", "The highest temperature of the settled field."),
        number("heat_out_left_w_per_m2", "W/m^2", "Heat leaving through the left end, per square metre of the section."),
        number("heat_out_right_w_per_m2", "W/m^2", "Heat leaving through the right end, per square metre of the section."),
    ],
    invariants: &[number("energy_balance_residual", "1", "")],
    iterations: "Sweeps of the field until it settled.",
    field: field_words,
    answer: "a settled field",
    refusals: REFUSALS,
    door_refusals: &[Refusal {
        name: "HeatGenerationBeyondArithmetic",
        meaning: "The heat generation is more than 1e300 W/m^3 in size, beyond what the field's 64-bit arithmetic carries.",
        sealed: false,
    }],
    sections,
    validation: Some(validation),
    example,
    further_examples: &[],
    parts: &[],
    check: nothing_more,
    solve,
};

/// Each refusal with the sentence above its variant in the solver's source.
/// The two the request itself shows are answered at the door, with no seal.
const REFUSALS: &[Refusal] = &[
    Refusal { name: "NonFiniteValue", meaning: "An input or an intermediate value was NaN or infinite.", sealed: true },
    Refusal { name: "EndOutsideSolid", meaning: "An end is at or below absolute zero, or above the melting point of the rod.", sealed: false },
    Refusal { name: "FieldAboveMeltingPoint", meaning: "The settled field is above the melting point of the rod somewhere.", sealed: true },
    Refusal { name: "FieldBelowAbsoluteZero", meaning: "The settled field reaches absolute zero.", sealed: true },
    Refusal { name: "NotSettled", meaning: "The sweep cap was reached with the field still moving.", sealed: true },
    Refusal { name: "EnergyBalanceBroken", meaning: "The settled field's ends do not carry the heat generated inside.", sealed: true },
];

fn field_words() -> String {
    format!(
        "SHA-256 of the point count ({}) as 8 little-endian bytes, then the settled temperature in kelvin at each point from the left end to the right as little-endian f64 bytes.",
        model::CELLS + 2
    )
}

fn sections() -> Sections {
    Sections {
        locked: json!({
            "model": "Steady heat conduction along a rod of one conductivity, k d2T/dx2 + q = 0, its two ends held at the temperatures asked, settled by successive over-relaxation.",
            "rod_length_m": model::ROD_LENGTH_M,
            "conductivity_w_per_m_k": model::CONDUCTIVITY_W_PER_M_K,
            "melting_point_k": model::MELTING_POINT_K,
            "cells": model::CELLS,
            "relaxation_weight": model::RELAXATION_WEIGHT,
            "max_sweeps": model::MAX_SWEEPS,
        }),
        invariants: json!([{
            "name": "energy_balance_residual",
            "rule": "The heat leaving through the two ends equals the heat generated inside.",
            "tolerance": model::ENERGY_BALANCE_TOLERANCE,
            "unit": "fraction of the heat generated",
            "held": "at acceptance",
            "refusal": "EnergyBalanceBroken",
        }]),
        convergence: json!({
            "settle_tolerance": model::SETTLE_TOLERANCE,
            "rule": "No point of the field moves by more than the settle tolerance times the field's largest temperature in one sweep before a result is returned.",
            "refusal": "NotSettled",
        }),
        reference: json!({
            "kind": "The company's own derivation: the exact field is a parabola, which the solver's test holds the settled field to within 1e-8 K.",
            "test": "the_settled_field_is_the_parabola_of_its_derivation",
        }),
    }
}

/// The record in the five parts, in the words of engine 0.12.0: the
/// company's own specimen says what it is and what it holds, and claims no
/// outside recomputation it does not have (`OWN_SPECIMEN_SOLVER_RESULT`,
/// which find_model's test holds an own-specimen solver to).
fn validation() -> [String; 5] {
    [
        "{{Title}}, the company's own specimen: a 0.5 m aluminium rod with its ends held from above 0 K to 933.47 K and heat generated inside it.".to_string(),
        "The company's own constants for this specimen, stated in its description under locked.".to_string(),
        "The ledgers stated under invariants: the heat leaving through the two ends equals the heat generated inside, on every settled field.".to_string(),
        crate::descriptor::OWN_SPECIMEN_SOLVER_RESULT.to_string(),
        "The company's own specimen: its numbers are those of the rod the company specified, and an answer is that rod's under the solver's equations and inside its stated limits.".to_string(),
    ]
}

fn example() -> Value {
    json!({ "left_temperature_k": 300.0, "right_temperature_k": 350.0, "heat_generation_w_per_m3": 1.0e5 })
}

/// Every bound is in the field list.
fn nothing_more(_given: &Given) -> Result<(), String> {
    Ok(())
}

fn solve(given: &Given) -> Result<Solved, String> {
    let question = {{Name}}Question {
        left_temperature_k: given.number("left_temperature_k")?,
        right_temperature_k: given.number("right_temperature_k")?,
        heat_generation_w_per_m3: given.number("heat_generation_w_per_m3")?,
    };
    Ok(solved(model::solve(&question)))
}

/// What the solver returned, in the words every solver shares.
pub fn solved(returned: Result<{{Name}}Field, {{Name}}Reject>) -> Solved {
    let field = match returned {
        Ok(field) => field,
        Err(reject) => return Solved::Refused(Refused { name: reject.name(), sentence: reject.to_string(), iterations: None }),
    };
    let named = |name: &str, value: f64| (name.to_string(), Published::Number(value));
    Solved::Answer(Answer {
        iterations: u64::from(field.sweeps),
        result: vec![
            named("peak_temperature_k", field.peak_temperature_k),
            named("heat_out_left_w_per_m2", field.heat_out_left_w_per_m2),
            named("heat_out_right_w_per_m2", field.heat_out_right_w_per_m2),
        ],
        invariants: vec![named("energy_balance_residual", field.energy_balance_residual)],
        field: field.temperature_k,
    })
}
