//! The avionics cold plate on the web connection.
//!
//! Small steps near equilibrium land rises of zero or one unit in the last
//! place of the temperature. The boundary heat a step books is the heat of the
//! rise that landed, so the ledger of a million floor steps is exact, and the
//! run agrees with one step over the same span within the step error the card
//! states. A bath at zero kelvin is outside the card window and is answered
//! TemperatureOutsideCard; NegativeTemperature is the name of a step that
//! would land the plate at or below zero kelvin, and the description says so.

use std::time::Duration;

use genesis_kernel::thermal;
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
    let client = Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .unwrap();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                return (status, response.text().await.unwrap());
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

fn state_number(body: &Value, key: &str) -> f64 {
    let hex = body["final_state"][format!("{key}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{key}_hex in {body}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

async fn run(base: &str, heat_power_w: f64, dt_s: f64, steps: u64) -> Value {
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": dt_s,
        "segments": [{
            "steps": steps,
            "load": { "heat_power_w": heat_power_w, "ambient_k": thermal::REFERENCE_TEMPERATURE_K }
        }],
        "record_every": steps
    });
    let (status, text) = post(base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "completed", "{body}");
    body
}

/// At 1e-6 W a million floor steps used to book 51 % more heat than the plate
/// held; at 7e-7 W they booked 1.577e-5 J while the plate did not move. Now
/// the heat booked is the capacitance times the rise that landed, and the
/// million-step answer is within a million and one step errors of the
/// one-step answer.
#[tokio::test]
async fn a_million_floor_steps_near_equilibrium_book_only_the_rise_that_landed() {
    let base = spawn_app().await;
    let plate = thermal::ThermalNode::avionics_cold_plate();
    let floor_s = plate.floor_dt_s().unwrap();
    let steps: u64 = 1_000_000;
    for heat_power_w in [1.0e-6, 7.0e-7] {
        let many = run(&base, heat_power_w, floor_s, steps).await;
        let span_s = state_number(&many, "t_s");
        let one = run(&base, heat_power_w, span_s, 1).await;
        let rise_k = state_number(&many, "temperature_k") - thermal::REFERENCE_TEMPERATURE_K;
        let booked_j = state_number(&many, "boundary_work_j");
        let landed_j = thermal::THERMAL_CAPACITANCE_J_PER_K * rise_k;
        assert!(
            (booked_j - landed_j).abs() <= 1.0e-18,
            "{heat_power_w} W: booked {booked_j} J for a landed rise worth {landed_j} J"
        );
        for body in [&many, &one] {
            let defect_j = state_number(body, "energy_defect_j");
            assert!(defect_j.abs() <= 1.0e-18, "{heat_power_w} W: defect {defect_j} J");
        }
        let gap_k = (state_number(&many, "temperature_k") - state_number(&one, "temperature_k")).abs();
        assert!(
            gap_k <= (steps + 1) as f64 * thermal::STEP_ERROR_K,
            "{heat_power_w} W: a million steps and one differ by {gap_k} K"
        );
        if heat_power_w == 7.0e-7 {
            assert_eq!(rise_k, 0.0, "the plate moved {rise_k} K");
            assert_eq!(booked_j, 0.0);
        }
    }
}

/// A bath at 0 K is outside the card window and is refused by the window's
/// name. A step that would land the plate below zero kelvin is
/// NegativeTemperature, which is what the description says.
#[tokio::test]
async fn negative_temperature_means_a_landing_at_or_below_zero_kelvin() {
    let base = spawn_app().await;
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "energy", "card": "thermal"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    let meaning = described["rejects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|reject| reject["name"] == "NegativeTemperature")
        .unwrap()["meaning"]
        .clone();
    assert_eq!(meaning, "A step would land the plate at or below zero kelvin.");

    let bath_at_zero = json!({
        "domain": "energy",
        "card": "thermal",
        "parameters": { "temperature_k": 293.15, "heat_power_w": 0.0, "ambient_k": 0.0, "dt_s": 0.1 }
    });
    let (status, text) = post(&base, "/verify_physics", &bath_at_zero).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        "Physical Failure: 'ambient_k' 0 K is below 233.15 K, the inclusive minimum of the thermal card (TemperatureOutsideCard). That state is not sealed."
    );

    let ceiling_s = thermal::ThermalNode::avionics_cold_plate().gate_dt_s().unwrap();
    let pulled_below_zero = json!({
        "domain": "energy",
        "card": "thermal",
        "parameters": { "temperature_k": 293.15, "heat_power_w": -1.0e6, "ambient_k": 293.15, "dt_s": ceiling_s }
    });
    let (status, text) = post(&base, "/verify_physics", &pulled_below_zero).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(NegativeTemperature)"), "{text}");
}

/// The description states the step error the kernel holds, and no longer says
/// a floor step always changes the state by two parts in a hundred million.
#[tokio::test]
async fn the_description_states_the_step_error() {
    let base = spawn_app().await;
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "energy", "card": "thermal"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    let floor_from = described["dt_s"]["floor_from"].as_str().unwrap();
    assert!(floor_from.starts_with("the ceiling divided by 2^20"), "{floor_from}");
    let written = format!("{:?}", thermal::STEP_ERROR_K);
    assert_eq!(written, "2.2737367544323206e-13");
    assert!(floor_from.contains(&format!("{written} K (2^-42 K)")), "{floor_from}");
    assert!(floor_from.contains("N + 1 times"), "{floor_from}");
    assert!(floor_from.contains("can round to zero"), "{floor_from}");
    assert!(!floor_from.contains("mostly rounding"), "{floor_from}");
    let rule = described["dt_s"]["rule"].as_str().unwrap();
    assert!(rule.contains(floor_from), "{rule}");
}
