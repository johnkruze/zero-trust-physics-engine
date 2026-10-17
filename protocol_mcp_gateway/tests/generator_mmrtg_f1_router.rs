//! The Curiosity generator card on the HTTP door: the end of the record.
//!
//! Every step, the first included, compares its landing with mission day
//! 4,087 plus one rounding tolerance, 2^-22 s, and a step past it lands with
//! FLAG_PAST_RECORD: the record ended that day, the generator did not. A
//! restart from a returned mission time gets the verdict the run straight
//! through gets. The card stops where the decay heat left falls under the
//! ledger's resolution. The description states the clock, the tolerance,
//! where the energy totals count from, and the second party's repeat of the
//! comparison.
//!
//! No measurement is read here.

use std::time::Duration;

use genesis_kernel::generator_mmrtg_f1;
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
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client.post(&url).json(body).send().await {
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

async fn run(base: &str, mission_time_s: f64, dt_s: f64, steps: u64) -> Value {
    let body = json!({
        "domain": "energy", "card": "generator_mmrtg_f1",
        "initial": { "mission_time_s": mission_time_s },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": {} }],
        "record_every": 1
    });
    let (status, text) = post(base, "/run_trajectory", &body).await;
    assert_eq!(status, 200, "{body}: {text}");
    serde_json::from_str(&text).unwrap()
}

async fn describe(base: &str) -> Value {
    let (status, text) = post(
        base,
        "/describe_card",
        &json!({ "domain": "energy", "card": "generator_mmrtg_f1" }),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

/// Ten steps of 4,224,499.9 s from 310,871,801 s land on the last day, and
/// so does the one step from the ninth returned mission time. In 0.6.0 the
/// restart was refused.
#[tokio::test]
async fn a_restart_from_a_returned_mission_time_lands_where_the_run_straight_through_lands() {
    let base = spawn_app().await;
    let dt_s = 4_224_499.9;
    let straight = run(&base, 310_871_801.0, dt_s, 10).await;
    assert_eq!(straight["outcome"], "completed", "{straight}");
    assert_eq!(straight["steps_landed"], 10);
    let end_s = straight["final_state"]["t_s"].as_f64().unwrap();
    assert!((end_s - generator_mmrtg_f1::MISSION_TIME_MAX_S).abs() <= generator_mmrtg_f1::MISSION_END_TOLERANCE_S);

    let nine = run(&base, 310_871_801.0, dt_s, 9).await;
    let ninth_s = nine["final_state"]["t_s"].as_f64().unwrap();
    let restart = run(&base, ninth_s, dt_s, 1).await;
    assert_eq!(restart["outcome"], "completed", "{restart}");
    assert_eq!(restart["steps_landed"], 1);
    let restart_end_s = restart["final_state"]["t_s"].as_f64().unwrap();
    assert!((restart_end_s - end_s).abs() <= generator_mmrtg_f1::MISSION_END_TOLERANCE_S);

    // A single step from the decimal start and step lands on the day itself.
    let single = run(&base, 348_892_300.1, dt_s, 1).await;
    assert_eq!(single["outcome"], "completed", "{single}");
    assert_eq!(single["final_state"]["t_s"].as_f64().unwrap(), generator_mmrtg_f1::MISSION_TIME_MAX_S);

    // The same through the single step of verify_physics.
    let (status, text) = post(
        &base,
        "/verify_physics",
        &json!({
            "domain": "energy", "card": "generator_mmrtg_f1",
            "parameters": { "mission_time_s": ninth_s, "dt_s": dt_s }
        }),
    )
    .await;
    assert_eq!(status, 200, "{text}");
}

/// Steps past the last day of the record complete: two steps at the floor
/// whose end in decimal is 0.89 s past it, a step of a day from it, and a
/// thousand days past it, each with its proof. A start past where the decay
/// heat left falls under the ledger's resolution is refused by name.
#[tokio::test]
async fn steps_past_the_last_day_of_the_record_complete() {
    let base = spawn_app().await;
    let floor_s = 4.028797086407726;
    let two = run(&base, 353_116_792.832_405_8, floor_s, 2).await;
    assert_eq!(two["outcome"], "completed", "{two}");
    assert_eq!(two["steps_landed"], 2);
    assert!(two["run_seal"].is_string(), "{two}");
    let day = run(&base, generator_mmrtg_f1::MISSION_TIME_MAX_S, 86_400.0, 1_000).await;
    assert_eq!(day["outcome"], "completed", "{day}");
    assert_eq!(day["steps_landed"], 1_000);
    let end_s = day["final_state"]["t_s"].as_f64().unwrap();
    assert_eq!(end_s, generator_mmrtg_f1::MISSION_TIME_MAX_S + 1_000.0 * 86_400.0);
    assert_eq!(day["final_state"]["range_margin_s"].as_f64().unwrap(), 0.0);
    let generator = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
    let ledger_end_s = generator.ledger_end_mission_time_s().unwrap();
    let body = json!({
        "domain": "energy", "card": "generator_mmrtg_f1",
        "initial": { "mission_time_s": 1.01 * ledger_end_s },
        "dt_s": 86_400.0,
        "segments": [{ "steps": 1, "load": {} }],
        "record_every": 1
    });
    let (status, text) = post(&base, "/run_trajectory", &body).await;
    assert_ne!(status, 200, "{text}");
    assert!(text.contains("DecayHeatBelowLedgerResolution"), "{text}");
}

/// The description states the tolerance and the compensated clock, says
/// where the energy totals count from, carries no known defect of a first
/// step, and records the second party's repeat of the comparison.
#[tokio::test]
async fn the_description_states_the_clock_the_totals_and_the_second_party() {
    let base = spawn_app().await;
    let card = describe(&base).await;
    let text = card.to_string();
    assert!(text.contains("MISSION_END_TOLERANCE_S, 2^-22 s or about 2.38e-7 s: four spacings of 64-bit numbers between 2^28 s and 2^29 s, where the last day lies"), "{text}");
    assert!(text.contains("compares its landing with mission day 4,087 plus one tolerance"), "{text}");
    assert!(text.contains("lands with FLAG_PAST_RECORD set"), "{text}");
    assert!(!text.contains("2^-23 s") && !text.contains("mission day 2,808 plus"), "{text}");
    assert_eq!(generator_mmrtg_f1::MISSION_END_TOLERANCE_S, 2.0_f64.powi(-22));
    assert_eq!(generator_mmrtg_f1::MISSION_TIME_MAX_S, 4_087.0 * 86_400.0);
    assert!(text.contains("A restart from a returned t_s begins with no carry"));
    assert!(text.contains("count from the start of this run, the anchor of the ledger, and not from landing"));
    assert!(!text.contains("MISSION_END_ALLOWANCE_PER_STEP_S"));
    assert!(!text.contains("a single first step that ends exactly on the last mission day can be refused"));
    let limits = card["validation"]["limits"].as_str().unwrap();
    assert!(!limits.contains("not yet repeated by a second party"), "{limits}");
    assert!(!limits.contains("Known defect"), "{limits}");
    assert!(limits.contains("Repeated by a second party, Grok, on 2026-10-07"), "{limits}");
    assert!(limits.contains("evidence/verification/second_party/V-11"), "{limits}");
    // The published top is where the decay heat left is the ledger's resolution.
    let mission = card["phase_fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["name"] == "mission_time_s")
        .unwrap();
    let top = mission["maximum"].as_f64().unwrap();
    let generator = generator_mmrtg_f1::GeneratorMmrtgF1::curiosity();
    assert_eq!(top, generator.ledger_end_mission_time_s().unwrap());
    let bound = mission["bound"].as_str().unwrap();
    assert!(bound.contains("about 2,070 years after landing"), "{bound}");
    assert!(bound.contains("DecayHeatBelowLedgerResolution"), "{bound}");
    // The summary says the miss, and the record says what is served.
    let summary = card["specimen"].as_str().unwrap();
    assert!(summary.starts_with("One radioisotope generator, flight unit F1, on the Curiosity rover: its daily-average electrical output from landing on, with no load. Compared on the 1,279 held-back days 2,809 to 4,087"), "{summary}");
    assert!(summary.contains("so it failed by the rule fixed beforehand"), "{summary}");
    assert!(summary.contains("FLAG_PAST_RECORD"), "{summary}");
    assert!(limits.contains("The card serves mission days 2,809 to 4,087, the days it was scored on, as the model that was scored, with no constant changed since"), "{limits}");
    assert!(!limits.contains("ends at mission day 2,808"), "{limits}");
    assert!(card["declared_ends"].as_array().is_none_or(|ends| ends.is_empty()), "{}", card["declared_ends"]);
}
