//! The frame layout a card's description publishes is enough to read the
//! frames of a real run. For all thirty cards, a reader that has only the
//! published offsets, widths, types, quantities and flag bits decodes the
//! frames of a run and gets the values the run's state reports, to the
//! rounding of a 32-bit slot.

use serde_json::{Value, json};

use crate::descriptor::{self, FRAME_ROUNDING_SENTENCE, TIME_LABEL_SENTENCE};
use crate::mcp_server::dispatch;
use crate::new_gateway_state;
use crate::view::View;

/// One value of the state: a plain key, or `key[i]` for a component.
fn state_value(state: &Value, key: &str) -> f64 {
    match key.split_once('[') {
        None => state[key].as_f64().unwrap_or_else(|| panic!("state has no number {key}")),
        Some((name, rest)) => {
            let index: usize = rest.trim_end_matches(']').parse().expect("index");
            state[name][index].as_f64().unwrap_or_else(|| panic!("state has no {key}"))
        }
    }
}

fn read_f32(bytes: &[u8], at: &Value) -> f32 {
    assert_eq!(at["type"], "f32");
    assert_eq!(at["width"], 4);
    let start = at["offset"].as_u64().unwrap() as usize;
    f32::from_le_bytes(bytes[start..start + 4].try_into().unwrap())
}

async fn run(arguments: Value) -> Value {
    let state = new_gateway_state();
    let result = dispatch(&state, "run_trajectory", arguments.as_object().unwrap().clone()).await;
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let text = serde_json::to_value(&result).unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn the_published_layout_reads_the_frames_of_a_real_run_on_every_card() {
    let mut checked = 0;
    for card in View::full().cards() {
        let description = descriptor::describe_value_in(&View::full(), card.domain, card.card).unwrap();
        let slots = description["frame_slots"].as_array().unwrap();
        assert_eq!(slots.len(), 11, "{}", card.card);

        let example = description["example_run_trajectory"].clone();
        let mut one_step = example.clone();
        one_step["segments"] = json!([{
            "steps": 1,
            "load": example["segments"][0]["load"].clone(),
        }]);
        one_step["record_every"] = json!(1);

        for (label, request) in [("example", example), ("one step", one_step)] {
            let answer = run(request).await;
            let frames = answer["frames"].as_array().unwrap();
            let last = frames.last().expect("a frame");
            assert_eq!(last["step"], answer["steps_landed"], "{} {label}", card.card);
            let bytes = hex::decode(last["frame_hex"].as_str().unwrap()).unwrap();
            assert_eq!(bytes.len(), 64);
            let state = &answer["final_state"];
            let by_name = |name: &str| slots.iter().find(|slot| slot["slot"] == name).unwrap();

            // t: a 64-bit number.
            let t = by_name("t");
            assert_eq!((t["type"].as_str(), t["width"].as_u64()), (Some("f64"), Some(8)));
            let start = t["offset"].as_u64().unwrap() as usize;
            let t_frame = f64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
            assert_eq!(t_frame.to_bits(), state_value(state, t["state"].as_str().unwrap()).to_bits());

            // The six position and velocity slots, force_torque and residual.
            for name in ["pos[0]", "pos[1]", "pos[2]", "vel[0]", "vel[1]", "vel[2]", "force_torque", "residual"] {
                let at = by_name(name);
                let slot = read_f32(&bytes, at);
                let key = at["state"].as_str().unwrap();
                assert!(!at["unit"].as_str().unwrap().is_empty(), "{} {name}", card.card);
                let expected = match (card.card, name) {
                    ("satellite_lageos1", n) if n.starts_with("vel[") => {
                        let i = &n[4..5];
                        let exact = state_value(state, &format!("position_m[{i}]"));
                        let rounded = read_f32(&bytes, by_name(&format!("pos[{i}]")));
                        (exact - f64::from(rounded)) as f32
                    }
                    ("solar_system", n) if n.starts_with("vel[") => {
                        let i = &n[4..5];
                        let exact = state_value(state, &format!("earth_to_mars_m[{i}]"));
                        let rounded = read_f32(&bytes, by_name(&format!("pos[{i}]")));
                        (exact - f64::from(rounded)) as f32
                    }
                    ("satellite_lageos1", "force_torque") => {
                        (state_value(state, key) - 12_270_000.0) as f32
                    }
                    // The cantilever's slot is the amplitude ceiling less the
                    // envelope, as its row says.
                    ("cantilever_eth", "force_torque") => {
                        assert!(at["holds"].as_str().unwrap().contains("0.01579 − envelope_m"));
                        (0.01579 - state_value(state, key)) as f32
                    }
                    ("ti64_lattice", "force_torque") => {
                        let t = &state["tension_n"];
                        let (mut best, mut size) = (t[0].as_f64().unwrap(), t[0].as_f64().unwrap().abs());
                        for k in 1..3 {
                            let v = t[k].as_f64().unwrap();
                            if v.abs() > size {
                                best = v;
                                size = v.abs();
                            }
                        }
                        best as f32
                    }
                    _ => state_value(state, key) as f32,
                };
                assert_eq!(slot.to_bits(), expected.to_bits(), "{} {label} {name} ({key})", card.card);
            }

            // flags: the bits the card publishes, set by the state's own booleans.
            let flags = by_name("flags");
            assert_eq!((flags["type"].as_str(), flags["width"].as_u64()), (Some("u64"), Some(8)));
            let start = flags["offset"].as_u64().unwrap() as usize;
            let word = u64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
            let mut expected = 0_u64;
            let bits = flags["bits"].as_array().unwrap();
            assert_eq!(bits.len(), card.advisory.len());
            for bit in bits {
                if state[bit["state"].as_str().unwrap()] == true {
                    expected |= bit["mask"].as_u64().unwrap();
                }
            }
            assert_eq!(word, expected, "{} {label} flags", card.card);

            // proof: 16 bytes at the published offset, the first 16 bytes of the link before.
            let proof = by_name("proof");
            assert_eq!((proof["type"].as_str(), proof["width"].as_u64()), (Some("bytes"), Some(16)));
            let start = proof["offset"].as_u64().unwrap() as usize;
            if frames.len() >= 2 {
                let before = hex::decode(frames[frames.len() - 2]["link"].as_str().unwrap()).unwrap();
                assert_eq!(bytes[start..start + 16], before[..16], "{} {label} proof", card.card);
            }
            checked += 1;
        }

        // The same words on every card.
        let rule = description["frame_rule"].as_str().unwrap();
        assert!(rule.contains(FRAME_ROUNDING_SENTENCE), "{}", card.card);
        let label = description["time_label"].as_str().unwrap();
        if descriptor::COMPENSATED_CLOCKS.contains(&card.card) {
            assert_eq!(label, descriptor::COMPENSATED_TIME_LABEL_SENTENCE, "{}", card.card);
        } else {
            assert!(label.starts_with(TIME_LABEL_SENTENCE), "{}", card.card);
        }
    }
    assert_eq!(checked, 80);
}

/// Ten steps of 0.1 s read 0.9999999999999999 s: the sentence is true.
#[test]
fn the_time_label_example_is_what_the_sum_gives() {
    let mut t_s = 0.0_f64;
    for _ in 0..10 {
        t_s += 0.1;
    }
    assert_eq!(t_s, 0.9999999999999999);
    assert!(TIME_LABEL_SENTENCE.contains("0.9999999999999999"));
}

/// The satellite's sentence says what the time places, and no other card's does.
#[test]
fn only_the_satellite_says_the_time_places_the_sun_and_the_moon() {
    for card in View::full().cards() {
        let label = descriptor::describe_value_in(&View::full(), card.domain, card.card).unwrap()["time_label"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(label.contains("Sun and the Moon"), card.card == "satellite_lageos1", "{}", card.card);
    }
}
