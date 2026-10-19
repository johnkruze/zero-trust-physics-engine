//! The reactor_spert3_ccore_twin card through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the card's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! card's source. Add the card's own worked answers below.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/reactor_spert3_ccore_twin.rs");

#[test]
fn reactor_spert3_ccore_twin_meets_the_served_contract() {
    assert_card_conforms("energy", "reactor_spert3_ccore_twin");
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
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "reactor_spert3_ccore_twin" })).expect("described");
    let rows = description["rejects"].as_array().expect("rejects");
    for (name, sentence) in documented() {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn the_worked_run_lands_and_closes_its_ledger() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "reactor_spert3_ccore_twin" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 3.5e-6);
}

/// The worked run, the 2500 psig transient at 1.25 dollars, carried on: it ends at the first maximum of the
/// hot-spot plate temperature, and its peak and plate maximum are the card's.
#[test]
fn the_worked_run_ends_at_the_plate_maximum_with_its_numbers() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "reactor_spert3_ccore_twin" })).expect("described");
    let mut request = description["example_run_trajectory"].clone();
    request["segments"][0]["steps"] = json!(6000);
    let run = door.value("run_trajectory", &request).expect("the worked run is answered");
    assert_eq!(run["outcome"], "ended", "{run}");
    let last = &run["final_state"];
    let peak = last["peak_power_mw"].as_f64().unwrap();
    let energy = last["peak_energy_mws"].as_f64().unwrap();
    let rise = last["plate_rise_max_k"].as_f64().unwrap();
    // A 1.25 dollar step at 2500 psig from 25 degC: about 650 MW, 15 MW s and 370 K of plate rise.
    assert!(peak > 400.0 && peak < 1000.0, "{peak}");
    assert!(energy > 10.0 && energy < 25.0, "{energy}");
    assert!(rise > 250.0 && rise < 500.0, "{rise}");
    assert_eq!(last["past_plate_maximum"], true);
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 3.5e-6);
}

/// Pressure suppresses the void: the same 11 ms step at 0 psig takes back reactivity by vapour,
/// at 2500 psig almost none, and the plate reaches higher.
#[test]
fn pressure_suppresses_the_void_and_lifts_the_plate() {
    let mut door = Door::start();
    let run = |door: &mut Door, psig: f64| {
        let request = json!({
            "domain": "energy", "card": "reactor_spert3_ccore_twin",
            "initial": { "initial_temperature_k": 298.15, "system_pressure_psig": psig },
            "dt_s": 1.0e-4,
            "segments": [{ "steps": 8000, "load": { "reactivity_inserted_dollar": 1.27, "coolant_flow_m_per_s": 0.0 } }],
            "record_every": 1000,
        });
        door.value("run_trajectory", &request).expect("answered")
    };
    let low = run(&mut door, 0.0);
    let high = run(&mut door, 2500.0);
    for answer in [&low, &high] {
        assert_eq!(answer["outcome"], "ended", "{answer}");
    }
    let void = |answer: &Value| answer["final_state"]["void_feedback_dollar"].as_f64().unwrap();
    let rise = |answer: &Value| answer["final_state"]["plate_rise_max_k"].as_f64().unwrap();
    assert!(void(&low) > 10.0 * void(&high), "{} {}", void(&low), void(&high));
    assert!(rise(&high) > rise(&low) + 100.0, "{} {}", rise(&low), rise(&high));
}

/// A pressure at or above the critical pressure has no saturation temperature: refused by name.
#[test]
fn a_pressure_past_the_steam_tables_is_refused_by_name() {
    let mut door = Door::start();
    let request = json!({
        "domain": "energy", "card": "reactor_spert3_ccore_twin",
        "parameters": { "initial_temperature_k": 298.15, "system_pressure_psig": 3200.0,
                        "reactivity_inserted_dollar": 1.1, "coolant_flow_m_per_s": 0.0, "dt_s": 1.0e-4 },
    });
    let (error, text) = door.call("compute_step", &request);
    assert!(error, "{text}");
    assert!(text.contains("PressureOutsideSteamTables"), "{text}");
}

/// find_model finds the twin for the SPERT III C-core, and first for its boiling.
#[test]
fn find_model_finds_the_twin() {
    let mut door = Door::start();
    let keys = |door: &mut Door, description: &str| -> Vec<String> {
        let answer = door.value("find_model", &json!({ "description": description, "limit": 5 })).expect("answered");
        answer["matches"].as_array().map(|rows| rows.iter().map(|row| format!("{}/{}", row["domain"].as_str().unwrap_or(""), row["name"].as_str().or(row["card"].as_str()).unwrap_or(""))).collect()).unwrap_or_default()
    };
    for description in ["SPERT III C-core", "the SPERT III C-core reactor", "SPERT III C-core step transient under pressure and flow"] {
        let found = keys(&mut door, description);
        assert!(found.iter().any(|key| key.ends_with("reactor_spert3_ccore_twin")), "{description}: {found:?}");
    }
    let found = keys(&mut door, "SPERT III C-core with subcooled boiling and void feedback under pressure");
    assert!(found.first().is_some_and(|key| key.ends_with("reactor_spert3_ccore_twin")), "{found:?}");
}
