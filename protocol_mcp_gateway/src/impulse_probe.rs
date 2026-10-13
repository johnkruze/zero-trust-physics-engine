//! The load rule the description of a card states is the rule the door
//! applies. For the battery pack, the spring mount, the drive-mode resonator,
//! the oil column, the cable and the titanium cell, at several step sizes
//! across the card's range, the last load the door accepts and the first it
//! refuses are the neighbours at which the published expression
//! `|x| * dt_s <= C` turns false, evaluated here in the test: one
//! multiplication, compared with the C the description prints.

use serde_json::{Value, json};

use crate::view::View;
use crate::{descriptor, new_gateway_state, verify_step};

/// `(domain, card, load key, component of the load vector or none)`.
const CARDS: [(&str, &str, &str, bool); 6] = [
    ("energy", "battery_sag", "current_a", false),
    ("signal", "resonance", "accel_m_s2", false),
    ("signal", "tesseract", "accel_m_s2", false),
    ("materials", "hydraulic_fluid", "flow_m3_s", false),
    ("materials", "cable_stretch", "force_n", false),
    ("materials", "ti64_lattice", "force_n", true),
];

/// The C a description prints: the number after the last ` = ` before
/// `. The product`.
fn published_constant(rule: &str) -> f64 {
    let before = &rule[..rule.find(". The product").expect("the product sentence")];
    let number = before.rsplit(" = ").next().unwrap().trim_end_matches(" N*s");
    number.trim().parse().unwrap_or_else(|_| panic!("a number in {number}"))
}

async fn refused_by_the_load_rule(card: &(&str, &str, &str, bool), base: &Value, dt_s: f64, load: f64) -> bool {
    let mut request = base.clone();
    request["parameters"]["dt_s"] = json!(dt_s);
    request["parameters"][card.2] = if card.3 { json!([load, 0.0, 0.0]) } else { json!(load) };
    match verify_step(&new_gateway_state(), request).await {
        Ok(_) => false,
        Err(error) => error.message().contains("Nyquist"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_door_applies_the_rule_the_description_states_on_six_cards() {
    for card in CARDS {
        let description = descriptor::describe_value_in(&View::full(), card.0, card.1).unwrap();
        let rule = description["dt_s"]["rule"].as_str().unwrap();
        assert!(rule.contains("one multiplication in 64-bit arithmetic"), "{rule}");
        assert!(rule.contains("no fused multiply-add"), "{rule}");
        let limit = published_constant(rule);
        let floor = description["dt_s"]["floor_s"].as_f64().unwrap();
        let ceiling = description["dt_s"]["ceiling_s"].as_f64().unwrap();
        let base = description["example_verify_step"].clone();
        let mut windows = 0;
        for k in 0..24 {
            // From just over the floor to just under the ceiling, geometrically.
            let dt_s = floor * (ceiling / floor).powf((k as f64 + 0.37) / 24.0);
            let centre = (limit / dt_s).to_bits();
            let mut last_accepted = None;
            let mut first_refused = None;
            for bits in (centre - 4)..=(centre + 4) {
                let load = f64::from_bits(bits);
                let published = !(load * dt_s <= limit);
                // A charging current is refused first by its own published bound.
                let signs: &[f64] = if card.1 == "battery_sag" { &[1.0] } else { &[1.0, -1.0] };
                for sign in signs {
                    let signed = sign * load;
                    let door = refused_by_the_load_rule(&card, &base, dt_s, signed).await;
                    assert_eq!(door, published, "{} dt_s {dt_s:e} load {signed:e}", card.1);
                }
                if published {
                    first_refused.get_or_insert(bits);
                } else {
                    last_accepted = Some(bits);
                }
            }
            if let (Some(accepted), Some(refusal)) = (last_accepted, first_refused) {
                assert_eq!(refusal, accepted + 1, "{} dt_s {dt_s:e}", card.1);
                windows += 1;
            }
        }
        assert!(windows >= 20, "{}: {windows}", card.1);
    }
}
