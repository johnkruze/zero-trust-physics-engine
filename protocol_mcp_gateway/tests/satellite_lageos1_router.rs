//! The LAGEOS-1 satellite card on the web connection: the range it serves is
//! the range that was compared with published orbits.
//!
//! The card was compared on two orbits, LAGEOS-1 and LAGEOS-2, on 28
//! calendar days, 2026-08-23 to 2026-09-19, in runs of at most seven days.
//! A run past seven days from the published state is refused by name, and so
//! is a restart that would carry a forecast past seven days: the time since
//! the published state is a required input, handed back with every returned
//! state. An orbit other than the two compared orbits, polar, equatorial and
//! retrograde ones among them, and a date outside the 28 days are refused by
//! name with no proof. A single first step that ends exactly on the last date,
//! or exactly on seven days, lands. The description states what was compared
//! and the second scoring of the comparison, and carries no known defect.
//!
//! The start states are the saved validation requests of the compared runs.

use std::time::Duration;

use genesis_kernel::satellite_lageos1::{self, SatelliteLageos1};
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
    let client = Client::builder().timeout(Duration::from_secs(600)).build().unwrap();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
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
    panic!("{path} never accepted a connection: {last_error:?}");
}

async fn run(base: &str, body: &Value) -> Value {
    let (status, text) = post(base, "/run_trajectory", body).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn tape_holds(base: &str, body: &Value) -> bool {
    let (status, text) = post(base, "/verify_tape", body).await;
    assert_eq!(status, 200, "{text}");
    let verdict: Value = serde_json::from_str(&text).unwrap();
    verdict["holds"] == true
}

/// LAGEOS-1 at 2026-09-06 0 h UTC, the start of the seven-day run of week
/// 260912, in the card's frame.
const START_EPOCH_UTC_S: f64 = 841_924_800.0;
const START_POSITION_M: [f64; 3] = [295558.7678903363, -5108650.370522532, -11100383.23606335];
const START_VELOCITY_M_S: [f64; 3] = [-4821.08544993592, -3331.9602315669836, 1405.5539438152634];

/// LAGEOS-1 at 2026-08-23 0 h UTC, the start of week 260829.
const FIRST_POSITION_M: [f64; 3] = [3767126.6844992605, 8956617.400906362, 7564717.856833777];
const FIRST_VELOCITY_M_S: [f64; 3] = [4119.825668508274, 1676.7840658085788, -4053.2216929389388];

const SEVEN_DAYS_S: f64 = 604_800.0;

fn initial(epoch_utc_s: f64, forecast_elapsed_s: f64) -> Value {
    json!({
        "epoch_utc_s": epoch_utc_s,
        "forecast_elapsed_s": forecast_elapsed_s,
        "position_m": START_POSITION_M,
        "velocity_m_s": START_VELOCITY_M_S
    })
}

fn trajectory(initial: Value, dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "satellite_lageos1",
        "initial": initial,
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": {} }],
        "record_every": record_every
    })
}

fn one_step(initial: &Value, dt_s: f64) -> Value {
    let mut parameters = initial.as_object().unwrap().clone();
    parameters.insert("dt_s".to_string(), json!(dt_s));
    json!({ "domain": "aerospace", "card": "satellite_lageos1", "parameters": parameters })
}

const HORIZON: &str = "the forecast would end more than seven days, 604800 s, from the state taken from a published orbit, counted as forecast_elapsed_s at the start plus t_s, or the stated forecast_elapsed_s is below zero; the compared runs were seven days long (HorizonOutsideCard).";
const ORBIT: &str = "Physical Failure: the orbit is not closed, or is in neither of the two compared orbits: LAGEOS-1, semi-major axis 12 265 to 12 275 km, eccentricity 0.0034 to 0.0044 and inclination 109.80 to 109.83 degrees, and LAGEOS-2, 12 158 to 12 166 km, 0.0132 to 0.0138 and 52.63 to 52.66 degrees; no other orbit was compared (OrbitOutsideCard). That state is not sealed.";

/// A run of 70,000 steps of 10 s from a compared start. The first 60,480,
/// seven days, land and are sealed; the next is refused by name.
#[tokio::test]
async fn a_run_past_seven_days_is_refused_at_seven_days() {
    let base = spawn_app().await;
    let body = run(&base, &trajectory(initial(START_EPOCH_UTC_S, 0.0), 10.0, 70_000, 10_000)).await;
    assert_eq!(body["outcome"], "refused", "{body}");
    assert_eq!(body["reject"], "HorizonOutsideCard");
    assert_eq!(
        body["reject_detail"],
        format!("Physical Failure: {HORIZON} The run is sealed to step 60480. The state at step 60481 was refused and is not sealed.")
    );
    assert_eq!(body["steps_landed"], 60_480);
    assert_eq!(body["final_state"]["t_s"], SEVEN_DAYS_S);
    assert_eq!(body["final_state"]["forecast_elapsed_s"], SEVEN_DAYS_S);
    assert!(tape_holds(&base, &body).await);
}

/// A forecast carried on by restarts from its own returned states stops where
/// the run straight through stops, and a start that leaves out the time since
/// the published state is refused.
#[tokio::test]
async fn a_restart_carries_the_time_since_the_published_state_and_cannot_pass_seven_days() {
    let base = spawn_app().await;
    // A state stated as 604 700 s old carries a forecast on: without from it
    // is refused by name. Continued from the sealed run that took it there,
    // ten 10 s steps land and the eleventh would pass seven days.
    let stated = trajectory(initial(START_EPOCH_UTC_S, SEVEN_DAYS_S - 100.0), 10.0, 20, 1);
    let (status, text) = post(&base, "/run_trajectory", &stated).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    let upstream = trajectory(initial(START_EPOCH_UTC_S, 0.0), 10.0, 60_470, 10_000);
    let up = run(&base, &upstream).await;
    assert_eq!(up["final_state"]["forecast_elapsed_s"], SEVEN_DAYS_S - 100.0);
    let handed = json!({
        "epoch_utc_s": up["final_state"]["epoch_utc_s"],
        "forecast_elapsed_s": up["final_state"]["forecast_elapsed_s"],
        "position_m": up["final_state"]["position_m"],
        "velocity_m_s": up["final_state"]["velocity_m_s"]
    });
    let mut carried_on = trajectory(handed, 10.0, 20, 1);
    carried_on["from"] = json!({ "request": upstream, "run_seal": up["run_seal"] });
    let body = run(&base, &carried_on).await;
    assert_eq!(body["outcome"], "refused", "{body}");
    assert_eq!(body["reject"], "HorizonOutsideCard");
    assert_eq!(body["steps_landed"], 10);
    let end = &body["final_state"];
    assert_eq!(end["forecast_elapsed_s"], SEVEN_DAYS_S);
    assert_eq!(end["t_s"], 100.0);
    assert!(tape_holds(&base, &body).await);
    // Handed back as it was returned, the state takes no step.
    let again = json!({
        "epoch_utc_s": end["epoch_utc_s"],
        "forecast_elapsed_s": end["forecast_elapsed_s"],
        "position_m": end["position_m"],
        "velocity_m_s": end["velocity_m_s"]
    });
    // Handed back as it was returned, the state carries the forecast on, and a
    // run that was refused part way is not carried on: no step is taken.
    let (status, text) = post(&base, "/verify_physics", &one_step(&again, 10.0)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    let mut step = one_step(&again, 10.0);
    step["from"] = json!({ "request": carried_on, "run_seal": body["run_seal"] });
    let (status, text) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(UpstreamOutcomeNotCarried)"), "{text}");
    // A start without the time since the published state is refused.
    let mut unstated = initial(START_EPOCH_UTC_S, 0.0);
    unstated.as_object_mut().unwrap().remove("forecast_elapsed_s");
    for (path, request) in [
        ("/verify_physics", one_step(&unstated, 10.0)),
        ("/run_trajectory", trajectory(unstated.clone(), 10.0, 1, 1)),
    ] {
        let (status, text) = post(&base, path, &request).await;
        assert_eq!(status, 422, "{text}");
        assert_eq!(text, "Missing required parameter: 'forecast_elapsed_s'");
    }
    // Below zero and past seven days, refused by name before any step.
    for (stated, message) in [
        (-1.0, "Physical Failure: 'forecast_elapsed_s' -1 s is below 0 s, the inclusive minimum of the satellite_lageos1 card (HorizonOutsideCard). That state is not sealed."),
        (604_801.0, "Physical Failure: 'forecast_elapsed_s' 604801 s is above 604800 s, the inclusive maximum of the satellite_lageos1 card (HorizonOutsideCard). That state is not sealed."),
    ] {
        for (path, request) in [
            ("/verify_physics", one_step(&initial(START_EPOCH_UTC_S, stated), 10.0)),
            ("/run_trajectory", trajectory(initial(START_EPOCH_UTC_S, stated), 10.0, 1, 1)),
        ] {
            let (status, text) = post(&base, path, &request).await;
            assert_eq!(status, 422, "{path} {stated}: {text}");
            assert_eq!(text, message, "{path} {stated}");
        }
    }
}

/// Three orbits: circles at 12 270 km, equatorial, polar and retrograde.
/// None was compared; each is refused by name with no proof.
#[tokio::test]
async fn orbits_that_were_never_compared_are_refused_by_name() {
    let base = spawn_app().await;
    for velocity_m_s in [
        [0.0, 4804.88671850997, 0.0],
        [0.0, -894.7425285009462, 5699.6292470109165],
        [0.0, -6594.371775511863, 6.980032713678558e-13],
    ] {
        let start = json!({
            "epoch_utc_s": 840_715_200.0,
            "forecast_elapsed_s": 0.0,
            "position_m": [12_270_000.0, 0.0, 0.0],
            "velocity_m_s": velocity_m_s
        });
        let (status, text) = post(&base, "/run_trajectory", &trajectory(start, 10.0, 8_640, 8_640)).await;
        assert_eq!(status, 422, "{velocity_m_s:?}: {text}");
        assert_eq!(text, ORBIT);
        assert!(!text.contains("run_seal"));
    }
}

/// Dates outside the compared days: a compared start moved to 2017-01-01,
/// which 0.6.0 answered, and a day before and after the 28 compared days.
#[tokio::test]
async fn dates_outside_the_compared_days_are_refused_by_name() {
    let base = spawn_app().await;
    for epoch_utc_s in [536_500_800.0, 840_715_200.0 - 86_400.0] {
        let (status, text) =
            post(&base, "/run_trajectory", &trajectory(initial(epoch_utc_s, 0.0), 10.0, 8_640, 12)).await;
        assert_eq!(status, 422, "{text}");
        assert_eq!(
            text,
            format!(
                "Physical Failure: 'epoch_utc_s' {} s is below 840715200 s, the inclusive minimum of the satellite_lageos1 card (EpochOutsideCard). That state is not sealed.",
                protocol_mcp_gateway::message_number(epoch_utc_s)
            )
        );
    }
    let late = satellite_lageos1::EPOCH_MAX_S + 86_400.0;
    let (status, text) = post(&base, "/run_trajectory", &trajectory(initial(late, 0.0), 10.0, 1, 1)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("EpochOutsideCard"), "{text}");
    // The first compared start is the first date of the card.
    let first = json!({
        "epoch_utc_s": satellite_lageos1::EPOCH_MIN_S,
        "forecast_elapsed_s": 0.0,
        "position_m": FIRST_POSITION_M,
        "velocity_m_s": FIRST_VELOCITY_M_S
    });
    let body = run(&base, &trajectory(first, 10.0, 12, 12)).await;
    assert_eq!(body["outcome"], "completed", "{body}");
}

/// A single first step that ends exactly on the last date lands for a
/// decimal step, through the step and through a run of one step, and so does
/// one that ends exactly on seven days.
#[tokio::test]
async fn a_single_first_step_to_the_last_date_or_to_seven_days_lands() {
    let base = spawn_app().await;
    let last_s = satellite_lageos1::EPOCH_MAX_S;
    for dt_s in [10.0, 3.3, 0.1, 0.7, 1.9, 9.9, 0.3] {
        let at_end = initial(last_s - dt_s, 0.0);
        let (status, text) = post(&base, "/verify_physics", &one_step(&at_end, dt_s)).await;
        assert_eq!(status, 200, "dt {dt_s}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["state"]["epoch_utc_s"].as_f64().unwrap().to_bits(), last_s.to_bits(), "dt {dt_s}");
        let body = run(&base, &trajectory(at_end, dt_s, 1, 1)).await;
        assert_eq!(body["outcome"], "completed", "dt {dt_s}: {body}");
        assert_eq!(body["final_state"]["epoch_utc_s"].as_f64().unwrap().to_bits(), last_s.to_bits());

        // A forecast stated one step short of seven days carries a forecast
        // on: the card takes it, and without from the door refuses it by name.
        let at_seven_days = initial(START_EPOCH_UTC_S, SEVEN_DAYS_S - dt_s);
        let (status, text) = post(&base, "/verify_physics", &one_step(&at_seven_days, dt_s)).await;
        assert_eq!(status, 422, "dt {dt_s}: {text}");
        assert!(text.ends_with("(ContinuationWithoutUpstream)."), "dt {dt_s}: {text}");
    }
    // A run whose end in decimal is exactly seven days lands every step,
    // carried on from the sealed run that took the forecast to 604 200 s.
    let upstream = trajectory(initial(START_EPOCH_UTC_S, 0.0), 10.0, 60_420, 10_000);
    let up = run(&base, &upstream).await;
    assert_eq!(up["final_state"]["forecast_elapsed_s"], SEVEN_DAYS_S - 600.0);
    let handed = json!({
        "epoch_utc_s": up["final_state"]["epoch_utc_s"],
        "forecast_elapsed_s": up["final_state"]["forecast_elapsed_s"],
        "position_m": up["final_state"]["position_m"],
        "velocity_m_s": up["final_state"]["velocity_m_s"]
    });
    let mut carried_on = trajectory(handed, 0.6, 1_000, 1_000);
    carried_on["from"] = json!({ "request": upstream, "run_seal": up["run_seal"] });
    let body = run(&base, &carried_on).await;
    assert_eq!(body["outcome"], "completed", "{body}");
    assert_eq!(body["steps_landed"], 1_000);
}

/// What the description serves is what was compared.
#[tokio::test]
async fn the_description_states_what_was_compared_and_its_second_scoring() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_card", &json!({ "domain": "aerospace", "card": "satellite_lageos1" })).await;
    assert_eq!(status, 200, "{text}");
    let card: Value = serde_json::from_str(&text).unwrap();
    let fields = card["phase_fields"].as_array().unwrap();
    let names: Vec<&str> = fields.iter().map(|field| field["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["epoch_utc_s", "forecast_elapsed_s", "position_m", "velocity_m_s"]);
    let forecast = fields.iter().find(|field| field["name"] == "forecast_elapsed_s").unwrap();
    assert_eq!(forecast["minimum"], 0.0);
    assert_eq!(forecast["maximum"], SEVEN_DAYS_S);
    let epoch = fields.iter().find(|field| field["name"] == "epoch_utc_s").unwrap();
    assert_eq!(epoch["minimum"], 840_715_200.0);
    let card_itself = SatelliteLageos1::lageos1();
    assert_eq!(epoch["maximum"].as_f64().unwrap(), card_itself.last_start_epoch_utc_s().unwrap());
    let limits = card["validation"]["limits"].as_str().unwrap();
    assert!(limits.contains("compared on 28 calendar days, 2026-08-23 to 2026-09-19"), "{limits}");
    assert!(limits.contains("Repeated independently on 2026-10-07 by Grok"), "{limits}");
    assert!(!limits.contains("not yet repeated by a second party"), "{limits}");
    assert!(!limits.contains("six weeks of one season"), "{limits}");
    assert!(!limits.contains("Known defect"), "{limits}");
    assert!(card.to_string().contains("HorizonOutsideCard"));
}
