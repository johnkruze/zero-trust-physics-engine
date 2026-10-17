//! The grasp_lift_glass_4tu card through the local connection. Stamped from
//! the kit: the whole served contract, attacked from the card's own
//! description by `conformance_support`, and each refusal's sentence word for
//! word from the card's source. Then the card's own worked answers: the lift
//! at the worked grip lands without slip and closes both ledgers, find_flip
//! finds the least grip that lifts the object without slip, a grip under the
//! weight ends where the pads leave the faces, and a grip of zero is no
//! contact, refused before any step. The ledgers' scales are what is in
//! play: a ten-minute hold and a lift at 10 m/s for 515 s close both. The
//! hand moved down along the object on the table brings the fingers to the
//! table, a refusal and not an end; the object resting on the table is a
//! condition, not an event.

mod conformance_support;

use conformance_support::{Door, assert_card_conforms};
use serde_json::{Value, json};

const SOURCE: &str = include_str!("../../genesis_kernel/src/grasp_lift_glass_4tu.rs");

#[test]
fn grasp_lift_glass_4tu_meets_the_served_contract() {
    assert_card_conforms("materials", "grasp_lift_glass_4tu");
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
    door.value("describe_card", &json!({ "domain": "materials", "card": "grasp_lift_glass_4tu" })).expect("described")
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description = described(&mut door);
    let rows = description["rejects"].as_array().expect("rejects");
    let documented = documented();
    assert_eq!(documented.len(), 13);
    for (name, sentence) in documented {
        let row = rows.iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(row["meaning"], sentence.as_str(), "{name}");
    }
    let ends: Vec<&str> = description["declared_ends"].as_array().expect("ends").iter().filter_map(|end| end["reject"].as_str()).collect();
    assert!(ends.contains(&"ObjectSlippedOut") && ends.contains(&"ObjectLanded"), "{ends:?}");
}

/// The worked lift: 2 m/s² for 0.300 s and the speed held for 0.200 s at
/// 1.89 N, just above the least grip without slip. It lands every step, the
/// object leaves the table and follows the hand, no step slides whole, and
/// both ledgers close.
#[test]
fn the_worked_lift_lands_without_slip_and_closes_its_ledgers() {
    let mut door = Door::start();
    let description = described(&mut door);
    let run = door.value("run_trajectory", &description["example_run_trajectory"]).expect("the worked run is answered");
    assert_eq!(run["outcome"], "completed", "{run}");
    assert!(run["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-9, "{run}");
    let last = &run["final_state"];
    assert_eq!(last["slipped"], json!(false), "{last}");
    assert_eq!(last["on_table"], json!(false));
    assert!(last["object_height_m"].as_f64().unwrap() > 0.2);
    let peak = last["peak_load_ratio"].as_f64().unwrap();
    assert!(peak > 0.99 && peak < 1.0, "{peak}");
    // The same request gives the same bytes.
    let again = door.value("run_trajectory", &description["example_run_trajectory"]).unwrap();
    assert_eq!(run.to_string(), again.to_string());
}

/// The moment a robot cares about: find_flip along the grip of both
/// segments finds where `slipped` turns, the least grip that lifts the object
/// without slip, 1.8816 N under the worked lift; at rest 1.142 N holds it.
#[test]
fn find_flip_finds_the_least_grip_that_lifts_without_slip() {
    let mut door = Door::start();
    let description = described(&mut door);
    let mut request = description["example_run_trajectory"].clone();
    request["record_every"] = json!(583);
    let answer = door
        .value(
            "find_flip",
            &json!({ "request": request, "vary": "segments[*].load.grip_force_n", "low": 1.2, "high": 3.0, "level": { "quantity": "slipped" } }),
        )
        .expect("find_flip answers");
    let low = answer["low"]["value"].as_f64().unwrap();
    let high = answer["high"]["value"].as_f64().unwrap();
    assert!(low < high && high - low < 1.0e-9, "{answer}");
    assert!((high - 1.8816199617).abs() < 1.0e-6, "{high}");
}

/// A grip whose friction cannot carry the weight: the object stays on the
/// table, the pads slide up its faces, and the run ends where they leave
/// them. A grip of zero is no contact, refused before any step with no seal.
#[test]
fn a_grip_under_the_weight_ends_and_no_grip_is_no_contact() {
    let mut door = Door::start();
    let description = described(&mut door);
    let mut weak = description["example_run_trajectory"].clone();
    for segment in weak["segments"].as_array_mut().unwrap() {
        segment["load"]["grip_force_n"] = json!(0.8);
        segment["steps"] = json!(4000);
    }
    let run = door.value("run_trajectory", &weak).expect("answered");
    assert_eq!(run["outcome"], "ended", "{run}");
    assert!(run.to_string().contains("ObjectSlippedOut"), "{run}");
    let mut none = description["example_run_trajectory"].clone();
    none["segments"][1]["load"]["grip_force_n"] = json!(0.0);
    let (error, text) = door.call("run_trajectory", &none);
    assert!(error && text.contains("NoContact"), "{text}");
    assert!(!text.contains("run_seal"));
}

const CEILING_S: f64 = 0.0008578765327526418;

fn run_of(door: &mut Door, hand_velocity_m_s: f64, segments: &[(u64, f64, f64)], record_every: u64) -> Value {
    let segments: Vec<Value> = segments
        .iter()
        .map(|&(steps, grip, acceleration)| json!({ "steps": steps, "load": { "grip_force_n": grip, "hand_acceleration_m_s2": acceleration } }))
        .collect();
    door.value(
        "run_trajectory",
        &json!({ "domain": "materials", "card": "grasp_lift_glass_4tu", "initial": { "hand_velocity_m_s": hand_velocity_m_s },
                 "dt_s": CEILING_S, "segments": segments, "record_every": record_every }),
    )
    .expect("answered")
}

/// The red team's two honest runs that fixed scales refused: the worked grip
/// lifted, stopped and held still in the air for ten minutes (refused
/// MomentumLedgerBroken at 92.1 s by a fixed m sqrt(4 g H)), and a lift up to
/// 10 m/s held for 600,000 steps (refused SpontaneousGeneration at 52.7 s,
/// 1.2e-10 J against 1,025 J of the hand's work, by a fixed 2 m g H). Scaled
/// by what is in play, both complete with both ledgers closed at every step,
/// and so does a lift to 2.5 km lowered back onto the table.
#[test]
fn a_ten_minute_hold_and_a_long_lift_close_both_ledgers() {
    let mut door = Door::start();
    let hold = run_of(&mut door, 0.0, &[(350, 1.89, 2.0), (350, 1.89, -2.0), (700_000, 1.89, 0.0)], 100_000);
    assert_eq!(hold["outcome"], "completed", "{}", hold["reject_detail"]);
    let last = &hold["final_state"];
    assert!(last["t_s"].as_f64().unwrap() > 600.0 && last["slipped"] == json!(false) && last["on_table"] == json!(false), "{last}");
    assert!(hold["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-10, "{}", hold["max_residual_conservation_error"]);
    assert!(last["momentum_scale_n_s"].as_f64().unwrap() > 2000.0, "{last}");
    let lift = run_of(&mut door, 0.0, &[(2331, 5.0, 5.0), (600_000, 5.0, 0.0)], 100_000);
    assert_eq!(lift["outcome"], "completed", "{}", lift["reject_detail"]);
    let last = &lift["final_state"];
    assert!(last["hand_work_j"].as_f64().unwrap() > 10_000.0, "{last}");
    assert_eq!(last["hand_work_in_j"], last["hand_work_j"]);
    assert!(lift["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-10, "{}", lift["max_residual_conservation_error"]);
    // Up to 2.5 km and back down: the hand takes back nearly all the work it
    // gave, both sums stay in play, and the run ends where the object lands.
    let there_and_back = run_of(&mut door, 0.0, &[(2331, 5.0, 5.0), (290_000, 5.0, 0.0), (4662, 5.0, -5.0), (400_000, 5.0, 0.0)], 100_000);
    assert_eq!((there_and_back["outcome"].clone(), there_and_back["reject"].clone()), (json!("ended"), json!("ObjectLanded")), "{}", there_and_back["reject_detail"]);
    let last = &there_and_back["final_state"];
    assert!(last["hand_work_out_j"].as_f64().unwrap() > 0.99 * last["hand_work_in_j"].as_f64().unwrap(), "{last}");
    assert!(there_and_back["max_residual_conservation_error"].as_f64().unwrap() <= 1.0e-10);
}

/// The hand moving down at 0.1 m/s along the object resting on the table:
/// the pads slide down its faces to their lower end, at the table's surface,
/// after 349 steps. The object was never let go, so this is not an end: the
/// fingers reach the table, a refusal by name, sealed to the last step that
/// landed. Every landed frame carries FLAG_ON_TABLE, which describe_card
/// serves as a condition and not as an event.
#[test]
fn the_fingers_reaching_the_table_are_refused_and_resting_is_a_condition() {
    let mut door = Door::start();
    let run = run_of(&mut door, -0.1, &[(2000, 1.89, 0.0)], 100);
    assert_eq!(run["outcome"], "refused", "{run}");
    assert_eq!(run["reject"], "PadsReachTable");
    assert_eq!(run["steps_landed"], 349);
    let detail = run["reject_detail"].as_str().unwrap();
    assert!(detail.contains("the fingers reach the table's surface (PadsReachTable)"), "{detail}");
    assert!(run["final_state"]["on_table"] == json!(true));
    let description = described(&mut door);
    let ends: Vec<&str> = description["declared_ends"].as_array().unwrap().iter().filter_map(|end| end["reject"].as_str()).collect();
    assert!(!ends.contains(&"PadsReachTable"), "{ends:?}");
    let flags = description["advisory_flags"].as_array().unwrap();
    let on_table = flags.iter().find(|flag| flag["name"] == "FLAG_ON_TABLE").unwrap();
    assert!(on_table.get("kind").is_none(), "{on_table}");
    assert_eq!(on_table["state"], "on_table");
    for name in ["FLAG_INCIPIENT_SLIP", "FLAG_GROSS_SLIP"] {
        assert_eq!(flags.iter().find(|flag| flag["name"] == name).unwrap()["kind"], "event", "{name}");
    }
}
