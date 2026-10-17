//! The {{name}} solver through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the solver's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! solver's source. Add the solver's own worked answers below.

mod conformance_support;

use conformance_support::{Door, assert_solver_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_solvers/src/{{name}}.rs");

#[test]
fn {{name}}_meets_the_served_contract() {
    assert_solver_conforms("{{name}}");
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "{{name}}" })).expect("described");
    let start = SOURCE.find("pub enum {{Name}}Reject {").expect("the enum is in the solver");
    let mut doc = String::new();
    let mut seen = 0;
    for line in SOURCE[start..].lines().skip(1) {
        if line == "}" {
            break;
        }
        let text = line.trim_start();
        if let Some(sentence) = text.strip_prefix("/// ") {
            doc.push_str(sentence);
        } else if text.starts_with(|c: char| c.is_ascii_uppercase()) {
            let name: String = text.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            let row = description["refusals"].as_array().unwrap().iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(row["meaning"], doc.as_str(), "{name}");
            doc.clear();
            seen += 1;
        }
    }
    // The entry's own door refusals are listed after the solver's.
    assert!(seen > 0 && seen <= description["refusals"].as_array().unwrap().len());
}

#[test]
fn the_worked_question_converges() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "{{name}}" })).expect("described");
    let answer = door.value("solve", &description["example_solve"]).expect("answered");
    assert_eq!(answer["outcome"], "converged", "{answer}");
}
