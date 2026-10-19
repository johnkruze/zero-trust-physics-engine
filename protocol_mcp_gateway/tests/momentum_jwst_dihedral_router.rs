//! The momentum_jwst_dihedral card through the local connection. Stamped from the kit:
//! the whole served contract, attacked from the card's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! card's source. Add the card's own worked answers below.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/momentum_jwst_dihedral.rs");

#[test]
fn momentum_jwst_dihedral_meets_the_served_contract() {
    assert_card_conforms("aerospace", "momentum_jwst_dihedral");
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
    let description: Value = door.value("describe_card", &json!({ "domain": "aerospace", "card": "momentum_jwst_dihedral" })).expect("described");
    let rows = description["rejects"].as_array().expect("rejects");
    for (name, sentence) in documented() {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
}

#[test]
fn the_worked_run_lands_and_closes_its_ledger() {
    let mut door = Door::start();
    let description: Value = door.value("describe_card", &json!({ "domain": "aerospace", "card": "momentum_jwst_dihedral" })).expect("described");
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-6);
}

fn described(door: &mut Door) -> Value {
    door.value("describe_card", &json!({ "domain": "aerospace", "card": "momentum_jwst_dihedral" })).expect("described")
}

/// The same request on both JWST momentum cards: the attitude the identity,
/// the Sun at a pitch of -20 degrees, rolled 4 degrees towards +J2, at 1 au.
/// The two slopes turn that roll into torque about the boresight J1 (the
/// slope on the -J2 side pushes harder), -28.04 µN m, where the one flat
/// layer of momentum_jwst_srp gives +3.70; at -4 degrees +53.17 against
/// +24.22.
#[test]
fn the_two_cards_take_one_request_and_the_slopes_turn_roll_into_torque_about_the_boresight() {
    let mut door = Door::start();
    let at = |roll_deg: f64| {
        let (p, r) = ((-20.0_f64).to_radians(), roll_deg.to_radians());
        let v = [p.sin(), p.cos() * r.sin(), -p.cos() * r.cos()];
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        json!({ "h_eci_nms": [0.0, 0.0, 0.0], "q1": 0.0, "q2": 0.0, "q3": 0.0, "q4": 1.0,
                "sun_x": v[0] / n, "sun_y": v[1] / n, "sun_z": v[2] / n, "sun_distance_au": 1.0, "dt_s": 60.0 })
    };
    for (roll, dihedral, flat) in [(4.0, -28.04, 3.70), (-4.0, 53.17, 24.22)] {
        let mut torque = |card: &str| {
            let answer = door.value("verify_step", &json!({ "domain": "aerospace", "card": card, "parameters": at(roll) })).expect("the step lands");
            answer["state"]["torque_body_nm"][0].as_f64().unwrap() * 1.0e6
        };
        let (two_slopes, one_layer) = (torque("momentum_jwst_dihedral"), torque("momentum_jwst_srp"));
        assert!((two_slopes - dihedral).abs() < 0.01, "{roll}: {two_slopes}");
        assert!((one_layer - flat).abs() < 0.01, "{roll}: {one_layer}");
    }
}

/// The served words state the record: the four fitted numbers, the two
/// months not in the fit with each measure beside its target (the second,
/// ledger_2026-10-16/RESULT.md), and the torque about the boresight at roll.
#[test]
fn the_served_words_agree_with_the_record() {
    let mut door = Door::start();
    let text = described(&mut door).to_string();
    for words in [
        "0.0704146 rad, 4.03 degrees",
        "-0.0404585, 0.00776958 and -0.171418 m",
        "on 25 of 28 and 22 of 30 days (target 80 percent)",
        "within that allowance on 18 of 28 and 13 of 30 days (target 80 percent)",
        "the torque about J1 is off by -1.6 and +1.0 µN m on average",
        "off by -28.9 and +27.3 µN m beyond 2 degrees of roll",
        "B 18 of 28 days, 64 percent (80)",
        "The two ledgers confirm the arithmetic of one rotation and of two balances",
        "a norm from 0.999999 to 1.000001, both inclusive",
        "with nothing pulled back",
        "up to 3.5 N m s",
        "A 22 of 30 days, 73 percent (80)",
        "B 13 of 30 days, 43 percent (80)",
        "within 0.272 and 0.375 of its spread RMS (target 0.5)",
        "within 0.144 and 0.149 of the RMS torque (target 0.4)",
        "Fitted on the 64 days 12 May to 14 July 2023",
    ] {
        assert!(text.contains(words), "{words}");
    }
    assert!(!text.contains("Not yet scored"), "{text}");
    assert!(!text.contains("Compared once") && !text.contains("over one month"), "{text}");
}
