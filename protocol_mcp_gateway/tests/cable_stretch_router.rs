//! The tendon through the door: two faults of engine 0.6.0, each held here.
//!
//! - A run sealed a state whose elongation was a subnormal number, and the
//!   door refused that same state as a start. A landing on a subnormal
//!   elongation or velocity is now refused as `SubnormalState` and not sealed.
//! - The elongation had no range of its own: a large velocity of the other
//!   sign kept the tension in range for a braid shortened by 1 m or stretched
//!   past its ultimate elongation. The elongation is now held to
//!   `[0, x_ult)`: below is `Slack`, at or above is `TensileFailure`.
//!
//! The requests are the ones that showed each fault.

use std::time::Duration;

use genesis_kernel::cable_stretch;
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::net::TcpListener;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::new();
    for _ in 0..20 {
        match client
            .post(format!("{base}{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                return (status, response.text().await.unwrap());
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(25)).await,
        }
    }
    panic!("{path} never accepted a connection");
}

fn run_request(elongation_m: f64, velocity_m_s: f64, dt_s: f64, steps: u64) -> Value {
    json!({
        "domain": "materials",
        "card": "cable_stretch",
        "initial": { "elongation_m": elongation_m, "velocity_m_s": velocity_m_s },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": { "force_n": 0.0 } }],
        "record_every": steps
    })
}

fn step_request(elongation_m: f64, velocity_m_s: f64, dt_s: f64) -> Value {
    json!({
        "domain": "materials",
        "card": "cable_stretch",
        "parameters": {
            "elongation_m": elongation_m,
            "velocity_m_s": velocity_m_s,
            "force_n": 0.0,
            "dt_s": dt_s
        }
    })
}

fn field<'a>(card: &'a Value, name: &str) -> &'a Value {
    for field in card["phase_fields"].as_array().unwrap() {
        if field["name"] == name {
            return field;
        }
    }
    panic!("no field {name}");
}

const CEILING_S: f64 = 2.1577766334817883e-06;
const FLOOR_S: f64 = 2.057816155893124e-12;

#[tokio::test]
async fn a_run_that_would_land_on_a_subnormal_elongation_is_refused_by_name() {
    let base = spawn_app().await;
    let tendon = cable_stretch::Tendon::dyneema();
    assert_eq!(tendon.gate_dt_s().unwrap().to_bits(), CEILING_S.to_bits());

    for steps in [1, 2] {
        let (status, text) = post(&base, "/run_trajectory", &run_request(2.2251e-308, 0.0, CEILING_S, steps)).await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "refused", "{body}");
        assert_eq!(body["reject"], "SubnormalState", "{body}");
        assert_eq!(body["steps_landed"], 0, "{body}");
        let detail = body["reject_detail"].as_str().unwrap();
        assert!(detail.contains("(SubnormalState)"), "{detail}");
        // Nothing past the start is sealed.
        assert!(body["final_state"].is_null(), "{body}");
        assert!(body["frames"].as_array().unwrap().is_empty(), "{body}");
    }

    // The single step says the same.
    let (status, text) = post(&base, "/verify_physics", &step_request(2.2251e-308, 0.0, CEILING_S)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(SubnormalState)"), "{text}");
    assert!(!text.contains("seal\""), "{text}");

    // The state the 0.6.0 engine sealed is still refused at the door.
    let (status, text) = post(
        &base,
        "/run_trajectory",
        &run_request(2.2246550541640664e-308, -4.1237015814905787e-306, CEILING_S, 1),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("is not a finite normal number"), "{text}");
}

#[tokio::test]
async fn four_starts_outside_the_card_are_turned_away_by_name() {
    let base = spawn_app().await;
    let cases = [
        (-0.001, 725.6933909687426, "Slack"),
        (-1.0, 469928.1475272422, "Slack"),
        (0.03, -7415.051030799522, "TensileFailure"),
        (1.0, -456951.8496728388, "TensileFailure"),
    ];
    for (x, v, name) in cases {
        let (status, text) = post(&base, "/run_trajectory", &run_request(x, v, FLOOR_S, 10)).await;
        assert_eq!(status, 422, "x {x}: {text}");
        assert!(text.contains(&format!("({name})")), "x {x}: {text}");
        assert!(text.contains("'elongation_m'"), "x {x}: {text}");
        assert!(!text.contains("run_seal"), "{text}");
        let (status, text) = post(&base, "/verify_physics", &step_request(x, v, FLOOR_S)).await;
        assert_eq!(status, 422, "x {x}: {text}");
        assert!(text.contains(&format!("({name})")), "x {x}: {text}");
    }
}

#[tokio::test]
async fn the_elongation_range_is_published_and_held_on_both_sides() {
    let base = spawn_app().await;
    let tendon = cable_stretch::Tendon::dyneema();
    let x_ult = tendon.ultimate_elongation_m();
    let (status, text) = post(&base, "/describe_card", &json!({ "domain": "materials", "card": "cable_stretch" })).await;
    assert_eq!(status, 200, "{text}");
    let card: Value = serde_json::from_str(&text).unwrap();

    let elongation = field(&card, "elongation_m");
    assert_eq!(elongation["minimum"].as_f64(), Some(0.0));
    assert_eq!(elongation["minimum_exclusive"], false);
    assert_eq!(elongation["maximum"].as_f64().map(f64::to_bits), Some(x_ult.to_bits()));
    assert_eq!(elongation["maximum_exclusive"], true);
    let sentence = elongation["bound"].as_str().unwrap();
    assert!(sentence.starts_with("[0, 2.7999999999999994e-2) m, the ultimate elongation"), "{sentence}");
    assert!(sentence.contains("refused as Slack"), "{sentence}");
    assert!(sentence.contains("refused as TensileFailure"), "{sentence}");
    assert!(sentence.contains("The damper term does not widen this range."), "{sentence}");
    let velocity = field(&card, "velocity_m_s")["bound"].as_str().unwrap();
    assert!(velocity.contains("[-k x / c, (4.948008429403924e3 - k x) / c) m/s"), "{velocity}");
    let rejects: Vec<&str> = card["rejects"]
        .as_array()
        .map(|list| list.iter().filter_map(|item| item.as_str().or_else(|| item["name"].as_str())).collect())
        .unwrap_or_default();
    assert!(rejects.contains(&"SubnormalState"), "{}", card["rejects"]);

    // The largest elongation accepted, the first refused, and the two zeros.
    let below = f64::from_bits(x_ult.to_bits() - 1);
    let (status, text) = post(&base, "/run_trajectory", &run_request(below, 0.0, FLOOR_S, 1)).await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = post(&base, "/run_trajectory", &run_request(x_ult, -1.0, FLOOR_S, 1)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(TensileFailure)"), "{text}");
    for zero in [0.0, -0.0] {
        let (status, text) = post(&base, "/run_trajectory", &run_request(zero, 0.0, FLOOR_S, 1)).await;
        assert_eq!(status, 200, "{text}");
    }
    let (status, text) = post(&base, "/run_trajectory", &run_request(-1.0e-300, 1.0, FLOOR_S, 1)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(Slack)"), "{text}");
}
