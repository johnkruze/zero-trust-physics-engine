//! The momentum_jwst_srp card through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the card's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! card's source. Then the card's own worked answers: the worked run closes
//! its ledgers; an impulse that never reaches a huge momentum is named; a Sun
//! distance inside the Sun is refused; the norm's decimal edges land on both
//! sides; and the served words say what the record says.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/momentum_jwst_srp.rs");

#[test]
fn momentum_jwst_srp_meets_the_served_contract() {
    assert_card_conforms("aerospace", "momentum_jwst_srp");
}

/// Each `StepReject` variant's doc sentence, as the card's source writes it.
fn documented() -> Vec<(String, String)> {
    let start = SOURCE.find("pub enum StepReject {").expect("the enum is in the card");
    let mut found = Vec::new();
    let mut doc = String::new();
    for line in SOURCE[start..].lines().skip(1) {
        if line == "}" {
            break;
        }
        let text = line.trim_start();
        if let Some(sentence) = text.strip_prefix("/// ") {
            if !doc.is_empty() {
                doc.push(' ');
            }
            doc.push_str(sentence);
        } else if text.starts_with(|c: char| c.is_ascii_uppercase()) {
            let name: String = text.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            found.push((name, doc.replace("[`", "`").replace("`]", "`")));
            doc.clear();
        }
    }
    found
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "aerospace", "card": "momentum_jwst_srp" })).expect("described");
    let rows = description["rejects"].as_array().expect("rejects");
    for (name, sentence) in documented() {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn the_worked_run_lands_and_closes_its_ledger() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "aerospace", "card": "momentum_jwst_srp" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-6);
}

fn described(door: &mut Door) -> Value {
    door.value("describe_card", &json!({ "domain": "aerospace", "card": "momentum_jwst_srp" })).expect("described")
}

fn step_of(door: &mut Door, parameters: Value) -> (bool, String) {
    door.call("verify_step", &json!({ "domain": "aerospace", "card": "momentum_jwst_srp", "parameters": parameters }))
}

/// The red team's start of 1e17 N m s: 70.73 N m s of impulse booked over a
/// hundred steps of the ceiling, H moved by none of it, and a residual of
/// 7.1e-16 against a scale of |H|. Held to the impulse booked, the first step
/// is refused by the momentum ledger; so is 1e12. The worked start, a hundred
/// steps of the ceiling, completes.
#[test]
fn an_impulse_that_never_reaches_a_huge_momentum_is_named() {
    let mut door = Door::start();
    let description = described(&mut door);
    let load = description["example_run_trajectory"]["segments"][0]["load"].clone();
    for (h0, completes) in [(json!([1.0e17, 0.0, 0.0]), false), (json!([1.0e12, 0.0, 0.0]), false), (json!([-1.4853731309945153, -0.6656560404607257, 1.1653501123098389]), true)] {
        let run = door
            .value(
                "run_trajectory",
                &json!({ "domain": "aerospace", "card": "momentum_jwst_srp", "initial": { "h_eci_nms": h0 }, "dt_s": 50225.48032116796,
                         "segments": [{ "steps": 100, "load": load }], "record_every": 100 }),
            )
            .unwrap_or_else(|| panic!("{h0}"));
        if completes {
            assert_eq!(run["outcome"], "completed", "{run}");
            assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-12, "{run}");
        } else {
            assert_eq!((run["outcome"].clone(), run["reject"].clone(), run["steps_landed"].clone()), (json!("refused"), json!("AngularMomentumBreach"), json!(0)), "{h0}: {run}");
        }
    }
}

/// An observatory at or inside the Sun's radius, 0.00465047 au, is refused by
/// name at a grazing Sun, where the layer stays under the Kapton limit: the
/// red team's 0.001 au and 1e-9 au were answered. Just outside it the step
/// lands.
#[test]
fn a_sun_distance_inside_the_sun_is_refused_by_name() {
    let mut door = Door::start();
    let z: f64 = -1.0e-6;
    let at = |distance: f64| json!({ "h_eci_nms": [0.0, 0.0, 0.0], "q1": 0.0, "q2": 0.0, "q3": 0.0, "q4": 1.0,
        "sun_x": (1.0 - z * z).sqrt(), "sun_y": 0.0, "sun_z": z, "sun_distance_au": distance, "dt_s": 60.0 });
    for distance in [0.001, 1.0e-6, 1.0e-9, 695_700.0 / 149_597_870.7] {
        let (error, text) = step_of(&mut door, at(distance));
        assert!(error && text.contains("(SunDistanceInsideSun)") && text.contains("inside the Sun"), "{distance}: {text}");
    }
    let (error, text) = step_of(&mut door, at(0.0046505));
    assert!(!error, "{text}");
}

/// The norm's edges are 0.999999 and 1.000001, both inclusive, alike on
/// both sides: the red team found 1.000001 landing and 0.999999 refused.
#[test]
fn the_norms_decimal_edges_land_on_both_sides() {
    let mut door = Door::start();
    let at = |q1: f64, q4: f64| {
        let sun_z = if q1 != 0.0 { 1.0 } else { -1.0 };
        json!({ "h_eci_nms": [1.0, 0.0, 0.0], "q1": q1, "q2": 0.0, "q3": 0.0, "q4": q4, "sun_x": 0.0, "sun_y": 0.0, "sun_z": sun_z, "sun_distance_au": 1.0, "dt_s": 60.0 })
    };
    for (q1, q4) in [(1.000001, 0.0), (0.0, 0.999999), (0.0, 1.000001), (0.999999, 0.0)] {
        let (error, text) = step_of(&mut door, at(q1, q4));
        assert!(!error, "{q1} {q4}: {text}");
    }
    let below = f64::from_bits(0.999999_f64.to_bits() - 1);
    for (q1, q4) in [(0.0, below), (below, 0.0)] {
        let (error, text) = step_of(&mut door, at(q1, q4));
        assert!(error && text.contains("QuaternionNotUnit"), "{q1} {q4}: {text}");
    }
}

/// The served words agree with the record: the slews' departure is up to
/// 3.5 N m s (RESULT.md), in every sentence that gives it; the ledgers
/// confirm the arithmetic of one rotation and one balance; the edges of the
/// norm and the Sun's radius are stated; and all three months after the fit
/// are stated with each measure beside its target, the third as
/// ledger_2026-10-16/RESULT.md gives it, not the first month alone.
#[test]
fn the_served_words_agree_with_the_record() {
    let mut door = Door::start();
    let text = described(&mut door).to_string();
    assert!(!text.contains("about 1 N m s"), "the slews' departure is 3.5 N m s in the record");
    assert_eq!(text.matches("up to 3.5 N m s").count(), 2, "{text}");
    for words in [
        "The two ledgers confirm the arithmetic of one rotation and of one balance",
        "a norm from 0.999999 to 1.000001, both inclusive",
        "SunDistanceInsideSun",
        "with nothing pulled back",
        "on 23 of 28, 15 of 28 and 17 of 30 days (target 80 percent)",
        "within that allowance on 14 of 28, 8 of 28 and 8 of 30 days (target 80 percent)",
        "within 0.437, 0.384 and 0.421 of its spread RMS (target 0.5)",
        "On 15 August to 14 September 2023: A 17 of 30 days; B 8 of 30",
        "Fitted on the 34 days 12 May to 14 June 2023",
    ] {
        assert!(text.contains(words), "{words}");
    }
}
