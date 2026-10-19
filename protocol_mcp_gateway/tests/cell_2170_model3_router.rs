//! The cell_2170_twin card through the local connection: the whole served contract, attacked
//! from the card's own description by `conformance_support`, each refusal's sentence word for
//! word from the card's source, the worked run in the cold, and succession from both cards it
//! supersedes.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/cell_2170_twin.rs");

#[test]
fn cell_2170_twin_meets_the_served_contract() {
    assert_card_conforms("energy", "cell_2170_model3");
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

fn described(door: &mut Door) -> Value {
    door.value("describe_card", &json!({ "domain": "energy", "card": "cell_2170_model3" })).expect("described")
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description = described(&mut door);
    let rows = description["rejects"].as_array().expect("rejects");
    for (name, sentence) in documented() {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn no_placeholder_is_served() {
    let mut door = Door::start();
    let text = described(&mut door).to_string();
    assert!(!text.contains("@@"), "a placeholder is served");
    let listed = door.value("list_cards", &json!({})).expect("listed").to_string();
    assert!(!listed.contains("@@"));
}

/// The worked run: m80 at the -20 degC block from full at the rated current, the case held at
/// -20 degC. It lands every step, closes both ledgers, and the core warms above the case.
#[test]
fn the_worked_run_in_the_cold_warms_the_core_and_closes_both_ledgers() {
    let mut door = Door::start();
    let description = described(&mut door);
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-10);
    let last = &run["final_state"];
    let core = last["core_temperature_c"].as_f64().unwrap();
    let case = last["case_temperature_c"].as_f64().unwrap();
    assert_eq!(case, -20.0);
    assert!(core > case + 0.5, "the core warms by its own heat: {core} degC against {case}");
    assert!(last["heat_stored_j"].as_f64().unwrap() > 0.0 && last["heat_to_case_j"].as_f64().unwrap() > 0.0);
    let v = last["terminal_v"].as_f64().unwrap();
    assert!(v > 2.8 && v < 4.2, "{v}");
}

/// A held discharge in the cold runs to the lower limit and ends there in a sealed refusal.
#[test]
fn a_cold_discharge_ends_at_the_lower_limit() {
    let mut door = Door::start();
    let request = json!({
        "domain": "energy", "card": "cell_2170_model3",
        "initial": { "cell_number": 1000.0, "state_of_charge": 1.0, "capacity_ah": 4.2,
                     "case_temperature_least_c": -20.0, "case_temperature_most_c": -20.0 },
        "dt_s": 1.0,
        "segments": [{ "steps": 5000, "load": { "current_a": 4.5, "case_temperature_c": -20.0 } }],
        "record_every": 500,
    });
    let run = door.value("run_trajectory", &request).expect("answered");
    let text = run.to_string();
    assert!(text.contains("UndervoltageLimit") || text.contains("ElectrolyteDepleted"), "{text}");
}

#[test]
fn a_cell_the_card_does_not_hold_is_refused_by_name() {
    let mut door = Door::start();
    let request = json!({
        "domain": "energy", "card": "cell_2170_model3",
        "parameters": { "cell_number": 448.5, "state_of_charge": 0.9, "capacity_ah": 4.4,
                        "case_temperature_least_c": 25.0, "case_temperature_most_c": 25.0,
                        "current_a": 1.0, "case_temperature_c": 25.0, "dt_s": 1.0 },
    });
    let (error, text) = door.call("compute_step", &request);
    assert!(error, "{text}");
    assert!(text.contains("CellNotRecognized"), "{text}");
}

/// find_model gives the twin first for the cell in the cold, the older cards after it.
#[test]
fn find_model_finds_the_twin_first_in_the_cold() {
    let mut door = Door::start();
    let keys = |door: &mut Door, description: &str| -> Vec<String> {
        let answer = door.value("find_model", &json!({ "description": description, "limit": 5 })).expect("answered");
        answer["matches"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .map(|row| format!("{}/{}", row["domain"].as_str().unwrap_or(""), row["name"].as_str().or(row["card"].as_str()).unwrap_or("")))
                    .collect()
            })
            .unwrap_or_default()
    };
    for description in [
        "a lithium cell discharging in the cold",
        "Tesla Model 3 2170 cell at -20 degrees Celsius",
        "cold-weather battery sizing with a Tesla 2170 cell",
    ] {
        let found = keys(&mut door, description);
        assert!(found.first().is_some_and(|key| key.ends_with("cell_2170_model3")), "{description}: {found:?}");
    }
}
