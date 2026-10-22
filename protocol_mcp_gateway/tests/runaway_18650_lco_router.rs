//! The runaway_18650_lco_twin card through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the card's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! card's source. Add the card's own worked answers below.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/runaway_18650_lco_twin.rs");

#[test]
fn runaway_18650_lco_twin_meets_the_served_contract() {
    assert_card_conforms("energy", "runaway_18650_lco");
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
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "runaway_18650_lco" })).expect("described");
    let rows = description["rejects"].as_array().expect("rejects");
    for (name, sentence) in documented() {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn the_worked_run_lands_and_closes_its_ledger() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "runaway_18650_lco" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-6);
}

/// The worked run, Hatchard's 155 degC oven from 28 degC at 10 s, ends on
/// the kernel loop's state to the bit: it melts its separator, vents and
/// runs away, and each frame carries the flags the kernel sets.
#[test]
fn the_worked_oven_run_ends_on_the_kernel_loop_state() {
    use genesis_kernel::frame::LastStateFrame64;
    use genesis_kernel::runaway_18650_lco_twin::*;
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "runaway_18650_lco" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    let cell = Runaway18650LcoTwin::fitted();
    let mut state = RunawayState::try_fresh(&cell, 301.15).unwrap();
    let mut frame = LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [0; 16] };
    let load = RunawayLoad { oven_temperature_k: 428.15, heater_power_w: 0.0, adiabatic: 0.0 };
    let mut seen = 0;
    for _ in 0..600 {
        step(&cell, &mut state, &mut frame, load, 10.0).unwrap();
        seen |= frame.flags;
    }
    assert_eq!(seen, FLAG_SELF_HEATING | FLAG_RUNAWAY | FLAG_SEPARATOR_MELTED | FLAG_VENTED);
    let last = &run["final_state"];
    assert_eq!(last["temperature_k"].as_f64().unwrap().to_bits(), state.temperature_k.to_bits());
    assert_eq!(last["anode_lithium_fraction"].as_f64().unwrap().to_bits(), state.anode_lithium_fraction.to_bits());
    assert_eq!(last["vent_gas_held_fraction"].as_f64().unwrap().to_bits(), state.vent_gas_held_fraction.to_bits());
}


/// Engine 0.15.2, work orders of 21 October: the card's own worked state in
/// hotter ovens at 10 s, refused FirstLawBreach part way on 0.15.1 (455 K at
/// step 46, 452 K at 48, 520 K at 23, 700 K at 7), now runs its 600 steps,
/// and each tape verifies by request.
#[test]
fn hot_ovens_at_ten_seconds_run_and_verify() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "energy", "card": "runaway_18650_lco" })).expect("described");
    for oven_k in [452.0, 455.0, 458.09, 460.0, 520.0, 700.0] {
        let mut request = description["example_run_trajectory"].clone();
        request["segments"][0]["load"]["oven_temperature_k"] = json!(oven_k);
        let run = door.value("run_trajectory", &request).expect("answered");
        assert_eq!(run["outcome"], "completed", "{oven_k}: {}", run["reject_detail"]);
        assert_eq!(run["steps_landed"], 600, "{oven_k}");
        let verified = door.value("verify_tape", &json!({ "request": request, "run_seal": run["run_seal"] })).expect("verified");
        assert_eq!(verified["holds"], true, "{oven_k}: {verified}");
    }
}
