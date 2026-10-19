//! The tokamak_jet_dte1_twin card through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the card's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! card's source. Add the card's own worked answers below.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/tokamak_jet_dte1_twin.rs");

#[test]
fn tokamak_jet_dte1_twin_meets_the_served_contract() {
    assert_card_conforms("energy", "tokamak_jet_dte1");
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
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "tokamak_jet_dte1" })).expect("described");
    let rows = description["rejects"].as_array().expect("rejects");
    for (name, sentence) in documented() {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn the_worked_run_lands_and_closes_its_ledger() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "tokamak_jet_dte1" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-6);
}

/// The worked run, a second of the record pulse 42982 near its peak, is the
/// structure in plain Python floats (`twin/out/reference_run.txt`): fusion
/// power 4.535487 MW of which 1.484752 MW thermal, a central ion temperature
/// of 7.474 keV and 4.6128 MJ released, within 1e-12.
#[test]
fn the_worked_run_is_the_python_structure() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "tokamak_jet_dte1" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("answered");
    let state = &run["final_state"];
    for (field, expected) in [
        ("thermal_energy_j", 8740752.422457144),
        ("fusion_energy_j", 4612818.691672256),
        ("fusion_power_w", 4535487.441828749),
        ("fusion_power_thermal_w", 1484751.614340726),
        ("ion_temperature_kev", 7.474442275387155),
    ] {
        let got = state[field].as_f64().unwrap_or_else(|| panic!("{field}: {state}"));
        assert!((got / expected - 1.0).abs() < 1.0e-12, "{field}: {got} against {expected}");
    }
    assert_eq!(state["outside_compared"], false);
}
