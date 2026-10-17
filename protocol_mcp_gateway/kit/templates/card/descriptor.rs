//! What an agent reads of the {{name}} card: its fields and bounds, its step
//! floor and ceiling, its refusals word for word from the card's source, its
//! frame, its worked examples and its record. Stamped from the kit; a child
//! module of `descriptor.rs`, so it uses that file's own builders.
//!
//! The description skeleton. Every sentence here is served, so each keeps
//! the rules a description is held to (`tests/conformance_support`): the
//! company's own specimen is named so, never by what it lacks; a range is
//! written in plain words, "0 to 933.47 K"; no word "check"; and where the
//! model is compared with measurements, the record states the range compared
//! on and the result there in numbers.

use genesis_kernel::{{name}};
use serde_json::Value;

use super::{Card, Role, finish, obj, pair_examples_of, scalar, scalar_value, slots, token};

const DOMAIN: &str = "{{domain}}";
const CARD: &str = "{{name}}";

/// What the card is, in one sentence of the list.
const SPECIMEN: &str = "{{Title}}, the company's own specimen: a body of 500 J/K warmed by a heater and cooled through 0.5 W/K to a bath, from above absolute zero to its melting point of 933.47 K.";

pub(super) fn card() -> Card {
    let specimen = {{name}}::{{Name}}::specimen();
    let gate = specimen.gate_dt_s().expect("specimen gate");
    let floor = specimen.floor_dt_s().expect("specimen floor");
    let melt = {{name}}::MELTING_POINT_K;
    let temperature = |name: &'static str, role: Role, of: &str| {
        scalar(
            name,
            role,
            "K",
            Some(0.0),
            true,
            Some(melt),
            true,
            format!("above 0 K and below {} K, the melting point of the body: {of}", token(melt)),
        )
    };
    finish(
        DOMAIN,
        CARD,
        SPECIMEN,
        vec![
            temperature("temperature_k", Role::Phase, "the body's temperature at the start"),
            scalar(
                "heat_power_w",
                Role::Load,
                "W",
                None,
                false,
                None,
                false,
                "no bound of its own: positive is heat into the body. The body's temperature is what the card holds".to_string(),
            ),
            temperature("ambient_k", Role::Load, "the bath's temperature"),
        ],
        floor,
        "the ceiling divided by 2^20",
        gate,
        "the step at which the body moves two hundredths of its distance to equilibrium",
        None,
        "The load is held across each step.".to_string(),
        &[
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidSpecimen",
            "BelowAbsoluteZero",
            "AboveMeltingPoint",
            "SpontaneousGeneration",
            "FirstLawBreach",
        ],
        vec![],
        slots(&[
            ("t", "Time since the start of the run, in seconds."),
            ("pos[0]", "temperature_k"),
            ("pos[1]", "ambient_k"),
            ("pos[2]", "heat_power_w"),
            ("vel[0]", "internal_energy_j"),
            ("vel[1]", "boundary_work_j"),
            ("vel[2]", "heat_to_ambient_w"),
            ("force_torque", "melt_margin_k"),
            ("residual", "Conservation residual."),
            ("flags", "No flag."),
            ("proof", "Not written by the step."),
        ]),
    )
}

/// One step at the ceiling from 300 K under 50 W, and four such steps.
pub(super) fn examples() -> (Value, Value) {
    let specimen = {{name}}::{{Name}}::specimen();
    pair_examples_of(
        DOMAIN,
        CARD,
        obj(&[("temperature_k", scalar_value(300.0))]),
        obj(&[("heat_power_w", scalar_value(50.0)), ("ambient_k", scalar_value({{name}}::REFERENCE_TEMPERATURE_K))]),
        super::card_dt(specimen.gate_dt_s()),
        4,
    )
}

/// The record, in the five parts every card gives: the object, the data, the
/// pass line, the result and the limits, in the words of engine 0.11.0. A
/// card compared with measurements states the range compared on and the
/// result there in numbers; the company's own specimen says what it is and
/// what it holds.
pub(super) fn validation(_specimen: &str) -> [String; 5] {
    [
        format!("{CARD}, the company's own specimen: {SPECIMEN} Every constant of it is stated in this description and fixed in the card."),
        "The company's own constants for this specimen, fixed in the card and listed in its description and in its step rule.".to_string(),
        "Every step holds the card's own equations and its conservation ledger to 1e-6 of the heat from 293.15 to 933.47 K, from the step floor to the ceiling.".to_string(),
        "Held: the card's tests run it from the floor to the ceiling, and every step holds its conservation ledger to its stated limit.".to_string(),
        "The company's own specimen: its numbers are those of the body the company specified, from above 0 K to its melting point, 933.47 K, where it stops by name.".to_string(),
    ]
}

/// Each refusal's sentence, word for word from the doc comment above the
/// variant in the card's source.
pub(super) fn meaning(name: &str) -> Option<&'static str> {
    Some(match name {
        "NonFiniteInput" => "The heater power, the bath temperature or `dt` was NaN or infinite.",
        "NonFiniteState" => "The incoming state, or the trial step, was not finite.",
        "NonPositiveDt" => "`dt` was zero or negative.",
        "InvalidSpecimen" => "A recorded constant is not this specimen's.",
        "BelowAbsoluteZero" => "The body or the bath is at or below absolute zero, incoming or landing.",
        "AboveMeltingPoint" => "The body or the bath is at or above the melting point of the body, incoming or landing.",
        "SpontaneousGeneration" => "The trial step created heat from nothing.",
        "FirstLawBreach" => "The trial step destroyed heat.",
        _ => return None,
    })
}
