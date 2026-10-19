//! The grid_gb_twin card through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the card's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! card's source. Add the card's own worked answers below.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/grid_gb_twin.rs");

#[test]
fn grid_gb_twin_meets_the_served_contract() {
    assert_card_conforms("energy", "grid_gb");
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
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "grid_gb" })).expect("described");
    let rows = description["rejects"].as_array().expect("rejects");
    for (name, sentence) in documented() {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn the_worked_run_lands_and_closes_its_ledger() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "grid_gb" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-6);
}

/// The worked minute: 17 November 2025, IFA, 1007 MW from 49.948 Hz with
/// the volumes NESO held in that block. Measured lowest 49.753 Hz; the twin
/// lands within 2 mHz of it, with no flag.
#[test]
fn the_worked_minute_lands_on_the_measured_fall() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "grid_gb" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    let lowest = run["final_state"]["lowest_frequency_hz"].as_f64().expect("lowest");
    assert!((lowest - 49.753).abs() < 0.002, "{lowest}");
    let t = run["final_state"]["t_s"].as_f64().unwrap();
    assert!((t - 60.0).abs() < 1e-6, "{t}");
}

/// A negative volume and a delivery time of zero are refused by name before
/// any physics.
#[test]
fn a_negative_volume_or_a_zero_delivery_time_is_refused_by_name() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "grid_gb" })).expect("described");
    for (field, value, name) in [("dm_low_mw", -1.0, "ServiceVolumeNegative"), ("fast_delivery_s", 0.0, "DeliveryTimeNotPositive")] {
        let mut request = description["example_run_trajectory"].clone();
        request["initial"][field] = json!(value);
        let (error, text) = door.call("run_trajectory", &request);
        assert!(error && text.contains(name), "{field}: {text}");
    }
}
