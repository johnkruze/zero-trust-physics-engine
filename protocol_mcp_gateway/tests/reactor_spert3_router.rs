//! The reactor_spert3_twin card through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the card's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! card's source. Add the card's own worked answers below.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/reactor_spert3_twin.rs");

#[test]
fn reactor_spert3_twin_meets_the_served_contract() {
    assert_card_conforms("energy", "reactor_spert3");
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
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "reactor_spert3" })).expect("described");
    let rows = description["rejects"].as_array().expect("rejects");
    for (name, sentence) in documented() {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn the_worked_run_lands_and_closes_its_ledger() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "reactor_spert3" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-6);
}

/// Engine 0.15.2, work order of 21 October: the worked start at 4e-4 s,
/// inside the card's ceiling of 4.3e-4 s, stopped DtAboveStateCeiling at
/// 1.3228 dollars on 0.15.1 (192 steps landed). It now runs its steps, the
/// state's own ceiling held by pieces inside the step, and its tape verifies
/// by request.
#[test]
fn a_step_inside_the_ceiling_runs_past_one_point_three_dollars() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "reactor_spert3" })).expect("described");
    for dollars in [1.3228, 1.35, 1.5] {
        let mut request = description["example_run_trajectory"].clone();
        request["dt_s"] = json!(4.0e-4);
        request["segments"][0]["steps"] = json!(2000);
        request["segments"][0]["load"]["reactivity_inserted_dollar"] = json!(dollars);
        let run = door.value("run_trajectory", &request).expect("answered");
        assert_ne!(run["reject"], "DtAboveStateCeiling", "{dollars}: {}", run["reject_detail"]);
        assert_eq!(run["outcome"], "completed", "{dollars}: {}", run["reject_detail"]);
        let verified = door.value("verify_tape", &json!({ "request": request, "run_seal": run["run_seal"] })).expect("verified");
        assert_eq!(verified["holds"], true, "{dollars}: {verified}");
    }
}
