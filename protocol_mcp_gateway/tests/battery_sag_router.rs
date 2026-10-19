//! The 12S pack at the door: a full pack refuses every charging current, and
//! each door refusal of the current carries the name `describe_card` lists
//! for its cause.

use std::time::Duration;

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
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(format!("{base}{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(body).unwrap())
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
    panic!("never accepted a connection: {last_error:?}");
}

async fn step(base: &str, parameters: Value) -> (u16, String) {
    post(
        base,
        "/verify_physics",
        &json!({"domain": "energy", "card": "battery_sag", "parameters": parameters}),
    )
    .await
}

async fn trajectory(base: &str, initial: Value, dt_s: f64, segments: Value) -> (u16, String) {
    post(
        base,
        "/run_trajectory",
        &json!({
            "domain": "energy",
            "card": "battery_sag",
            "initial": initial,
            "dt_s": dt_s,
            "segments": segments,
            "record_every": 1
        }),
    )
    .await
}

/// 5.9e-14 A of charge into a full pack for 0.1 s once landed and was sealed
/// at 50.4 V, because `50.4 + |I| * 0.06` rounds back to 50.4.
#[tokio::test]
async fn a_full_pack_refuses_every_charging_current_by_name_and_seals_nothing() {
    let base = spawn_app().await;
    for current_a in [
        -5.921189464667501e-14,
        -5.0e-14,
        -f64::MIN_POSITIVE,
        -1.0,
        -22.0,
    ] {
        let (status, text) = step(&base, json!({"soc": 1.0, "current_a": current_a, "dt_s": 0.1})).await;
        assert_eq!(status, 422, "current {current_a}: {text}");
        assert!(text.contains("(OvervoltageCutoff)"), "current {current_a}: {text}");
        assert!(!text.contains("sha384_seal"), "{text}");
    }
    // At rest and under discharge a full pack still lands.
    for current_a in [0.0, 5.0e-14, 22.0] {
        let (status, text) = step(&base, json!({"soc": 1.0, "current_a": current_a, "dt_s": 0.1})).await;
        assert_eq!(status, 200, "current {current_a}: {text}");
    }
    // A run that charges a pack to full and then sends a tiny charge stops
    // on that step by name: the refusal does not depend on how soc 1 was reached.
    let (status, text) = trajectory(
        &base,
        json!({"soc": 1.0}),
        0.1,
        json!([
            {"steps": 2, "load": {"current_a": 0.0}},
            {"steps": 1, "load": {"current_a": -5.921189464667501e-14}}
        ]),
    )
    .await;
    assert!(text.contains("OvervoltageCutoff"), "status {status}: {text}");
}

/// A charge of 120 A or more is past the overvoltage rule at every state of
/// charge the card accepts, so the door names it.
#[tokio::test]
async fn the_door_names_the_charging_minimum_as_overvoltage() {
    let base = spawn_app().await;
    for soc in [0.001, 0.5, 1.0] {
        let (status, text) = step(&base, json!({"soc": soc, "current_a": -120.0, "dt_s": 0.1})).await;
        assert_eq!(status, 422, "soc {soc}: {text}");
        assert!(text.contains("exclusive minimum"), "soc {soc}: {text}");
        assert!(text.contains("(OvervoltageCutoff)"), "soc {soc}: {text}");
    }
}

/// The current Nyquist limit at the door carries the name the card lists.
#[tokio::test]
async fn the_door_names_the_current_nyquist_limit() {
    let base = spawn_app().await;
    let (status, text) = step(
        &base,
        json!({"soc": 1.0, "current_a": 36.36363636363636, "dt_s": 6.600000000000001}),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("Nyquist limit"), "{text}");
    assert!(text.contains("(CurrentAboveNyquist)."), "{text}");
    // The same limit met by a load change inside a run.
    let (status, text) = trajectory(
        &base,
        json!({"soc": 1.0}),
        6.600000000000001,
        json!([
            {"steps": 1, "load": {"current_a": 1.0}},
            {"steps": 1, "load": {"current_a": 36.36363636363636}}
        ]),
    )
    .await;
    assert!(text.contains("(CurrentAboveNyquist)"), "status {status}: {text}");
}
