//! The Panasonic NCR18650PF cell card on the web door.
//!
//! The card takes only the 1C currents compared with measurement, 2.89817 to
//! 2.90065 A, and refuses any other by name. Every discharge that passed started
//! at full charge: every request states `held_current_a`, the held current
//! of the 1C discharge it belongs to, and a start below full charge is taken
//! only as the continuation of that discharge, held to its band, in
//! `verify_step`, `run_trajectory`, `validate_series` and `run_dataset`. A
//! change of exactly 0.83 mA in decimal is inside the band at every current.
//! The description names the dataset's other held currents as fitting data,
//! names the held 0.87 A discharge 3541_dis5_10p.mat and its failed
//! comparison, and names the second scoring.

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

async fn describe(base: &str) -> Value {
    let (status, text) =
        post(base, "/describe_card", &json!({ "domain": "energy", "card": "cell_18650pf" })).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

fn run(initial: Value, currents: &[f64], steps: u64) -> Value {
    let segments: Vec<Value> =
        currents.iter().map(|current_a| json!({ "steps": steps, "load": { "current_a": current_a } })).collect();
    json!({
        "domain": "energy", "card": "cell_18650pf", "initial": initial,
        "dt_s": 1.0, "segments": segments, "record_every": steps * currents.len() as u64
    })
}

async fn completed(base: &str, body: &Value) -> Value {
    let (status, text) = post(base, "/run_trajectory", body).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["outcome"], "completed", "{text}");
    answer
}

async fn refused_whole(base: &str, body: &Value, name: &str) -> String {
    let (status, text) = post(base, "/run_trajectory", body).await;
    assert_eq!(status, 422, "{body}: {text}");
    assert!(text.contains(name), "{name}: {text}");
    assert!(!text.contains("run_seal"), "{text}");
    text
}

fn full(held_current_a: f64) -> Value {
    json!({ "soc": 1.0, "held_current_a": held_current_a })
}

fn one_step(soc: f64, held_current_a: f64, current_a: f64) -> Value {
    json!({ "domain": "energy", "card": "cell_18650pf",
            "parameters": { "soc": soc, "held_current_a": held_current_a, "current_a": current_a, "dt_s": 1.0 } })
}

/// Only the compared 1C currents are taken. Every other current, a rest
/// among them, is refused by name, as a load and as a held current, in a
/// step, in a run and in a later segment of a run.
#[tokio::test]
async fn only_the_compared_1c_currents_are_taken() {
    let base = spawn_app().await;
    for current_a in [2.89817, 2.899, 2.9, 2.90065] {
        completed(&base, &run(full(current_a), &[current_a], 10)).await;
        let (status, text) = post(&base, "/verify_physics", &one_step(1.0, current_a, current_a)).await;
        assert_eq!(status, 200, "{current_a}: {text}");
    }
    for current_a in [0.0, 0.145, 0.87, 1.0, 2.0, 2.89816, 2.90066, 3.0, 3.5, -0.1] {
        let text = refused_whole(&base, &run(full(2.9), &[current_a], 10), "CurrentNotCompared").await;
        assert!(text.contains("2.89817") || text.contains("2.90065"), "{text}");
        refused_whole(&base, &run(full(current_a), &[2.9], 10), "CurrentNotCompared").await;
        for request in [one_step(1.0, 2.9, current_a), one_step(1.0, current_a, current_a)] {
            let (status, text) = post(&base, "/verify_physics", &request).await;
            assert_eq!(status, 422, "{current_a}: {text}");
            assert!(text.contains("CurrentNotCompared"), "{text}");
        }
    }
    refused_whole(&base, &run(full(2.9), &[2.9, 0.0], 5), "CurrentNotCompared").await;
}

/// The held 0.87 A discharge 04-27-17 3541_dis5_10p.mat was compared and
/// failed, so its currents stay refused: from full charge, as a continuation
/// of a hold, and in a single step, at every current it logged and at its
/// band's edges.
#[tokio::test]
async fn the_failed_0p87a_comparison_leaves_its_currents_refused() {
    let base = spawn_app().await;
    for current_a in [0.86805, 0.86888, 0.8693, 0.8697, 0.87053] {
        refused_whole(&base, &run(full(current_a), &[current_a], 10), "CurrentNotCompared").await;
        refused_whole(
            &base,
            &run(json!({ "soc": 0.95, "held_current_a": current_a }), &[current_a], 10),
            "CurrentNotCompared",
        )
        .await;
        let (status, text) = post(&base, "/verify_physics", &one_step(0.5, current_a, current_a)).await;
        assert_eq!(status, 422, "{current_a}: {text}");
        assert!(text.contains("CurrentNotCompared"), "{text}");
    }
}

/// The same in a dataset: a grid over current, answered row by row, and a
/// list plan of the same runs each seal the 2.9 A row alone and leave the
/// currents never compared unadmitted by name.
#[tokio::test]
async fn a_dataset_over_current_seals_only_the_compared_row() {
    let base = spawn_app().await;
    let plan = json!({
        "domain": "energy", "card": "cell_18650pf",
        "plan": { "kind": "grid",
            "base": { "initial": { "soc": 1.0, "held_current_a": 2.9 }, "dt_s": 1.0, "segments": [{ "steps": 600, "load": {} }], "record_every": 600 },
            "axes": [{ "path": "segments[0].load.current_a", "values": [0.0, 0.145, 0.87, 2.9, 3.0] }] },
        "store": "none", "return": "inline"
    });
    let (status, text) = post(&base, "/run_dataset", &plan).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let summary = answer["summary"].as_array().unwrap();
    assert_eq!(summary.len(), 5);
    for (index, row) in summary.iter().enumerate() {
        if index == 3 {
            assert_eq!(row["outcome"], "completed", "{row}");
        } else {
            assert_eq!(row["outcome"], "unadmitted", "{row}");
            assert_eq!(row["reject"], "CurrentNotCompared", "{row}");
            assert!(row["run_seal"].is_null());
        }
    }
    let runs: Vec<Value> = [0.0, 0.145, 0.87, 2.9, 3.0]
        .iter()
        .map(|current_a| json!({ "initial": { "soc": 1.0, "held_current_a": 2.9 }, "dt_s": 1.0, "segments": [{ "steps": 600, "load": { "current_a": current_a } }], "record_every": 600 }))
        .collect();
    let plan = json!({ "domain": "energy", "card": "cell_18650pf", "plan": { "kind": "list", "runs": runs }, "store": "none", "return": "inline", "answer": "full" });
    let (status, text) = post(&base, "/run_dataset", &plan).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let rows = answer["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    for (index, row) in rows.iter().enumerate() {
        if index == 3 {
            assert_eq!(row["outcome"], "completed", "{row}");
        } else {
            assert_eq!(row["outcome"], "unadmitted", "{row}");
            assert!(row["reject_detail"].as_str().unwrap().contains("CurrentNotCompared"), "{row}");
            assert!(row["run_seal"].is_null());
        }
    }
}

/// Every request states the held current of the 1C discharge it belongs to,
/// and a request without it is refused, in every tool that computes. A start
/// below full charge is the continuation of that discharge: it is held to
/// the held current's band, so a restart from a returned state cannot change
/// the current, and it matches the straight run.
#[tokio::test]
async fn a_start_below_full_charge_is_taken_only_as_a_continuation() {
    let base = spawn_app().await;
    for soc in [1.0, 0.5, 0.7227957482602845, 0.055] {
        let text = refused_whole(&base, &run(json!({ "soc": soc }), &[2.9], 10), "held_current_a").await;
        assert!(!text.contains("run_seal"), "{text}");
        let (status, text) = post(
            &base,
            "/verify_physics",
            &json!({ "domain": "energy", "card": "cell_18650pf", "parameters": { "soc": soc, "current_a": 2.9, "dt_s": 1.0 } }),
        )
        .await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains("held_current_a"), "{text}");
    }
    // a fresh run at another current from a returned state
    refused_whole(&base, &run(json!({ "soc": 0.7227957482602845, "held_current_a": 2.9 }), &[1.0], 100), "CurrentNotCompared").await;

    // a straight run, and the same run cut in two
    let straight = completed(&base, &run(full(2.89817), &[2.89817], 1100)).await;
    let first = completed(&base, &run(full(2.89817), &[2.89817], 1000)).await;
    let soc = first["final_state"]["soc"].as_f64().unwrap();
    refused_whole(&base, &run(json!({ "soc": soc, "held_current_a": 2.89817 }), &[2.89817], 100), "ContinuationWithoutUpstream").await;
    let mut carried_on = run(json!({ "soc": soc, "held_current_a": 2.89817 }), &[2.89817], 100);
    carried_on["from"] = json!({ "request": run(full(2.89817), &[2.89817], 1000), "run_seal": first["run_seal"] });
    let second = completed(&base, &carried_on).await;
    let end = |answer: &Value, key: &str| {
        answer["final_state"][key].as_f64().unwrap().to_bits()
    };
    assert_eq!(end(&second, "soc"), end(&straight, "soc"));
    assert_eq!(end(&second, "terminal_v"), end(&straight, "terminal_v"));

    // the restart no longer sidesteps the held-current rule
    let text = refused_whole(
        &base,
        &run(json!({ "soc": soc, "held_current_a": 2.89817 }), &[2.90065], 100),
        "CurrentChangedDuringDischarge",
    )
    .await;
    assert!(text.contains("the 2.89817 A of 'held_current_a': a run"), "{text}");
    assert!(!text.contains("in 'initial'"), "{text}");
    // a single step has no 'initial', and its refusal names none
    let (status, text) = post(&base, "/verify_physics", &one_step(soc, 2.89817, 2.90065)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("CurrentChangedDuringDischarge"), "{text}");
    assert!(text.contains("the 2.89817 A of 'held_current_a': a run"), "{text}");
    assert!(!text.contains("in 'initial'"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &one_step(0.5, 2.9, 2.9)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");

    // validate_series and run_dataset read the start the same way
    let without = run(json!({ "soc": 0.5 }), &[2.9], 10);
    let (status, text) = post(
        &base,
        "/validate_series",
        &json!({ "run": without, "quantity": "terminal_v", "measurements": [{ "step": 10, "value": 3.6 }] }),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("held_current_a"), "{text}");
    let changed = run(json!({ "soc": 0.5, "held_current_a": 2.89817 }), &[2.90065], 10);
    let (status, text) = post(
        &base,
        "/validate_series",
        &json!({ "run": changed, "quantity": "terminal_v", "measurements": [{ "step": 10, "value": 3.6 }] }),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("CurrentChangedDuringDischarge"), "{text}");
    let continued = run(json!({ "soc": 0.5, "held_current_a": 2.9 }), &[2.9], 10);
    let (status, text) = post(
        &base,
        "/validate_series",
        &json!({ "run": continued, "quantity": "terminal_v", "measurements": [{ "step": 10, "value": 3.6 }] }),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("ContinuationWithoutUpstream"), "{text}");
    let list = |initial: Value| {
        json!({
            "domain": "energy", "card": "cell_18650pf",
            "plan": { "kind": "list", "runs": [
                { "initial": initial, "dt_s": 1.0, "segments": [{ "steps": 10, "load": { "current_a": 2.90065 } }], "record_every": 10 }
            ] },
            "store": "none", "return": "inline", "answer": "full"
        })
    };
    let (status, text) = post(&base, "/run_dataset", &list(json!({ "soc": 0.5 }))).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("held_current_a"), "{text}");
    let (status, text) = post(&base, "/run_dataset", &list(json!({ "soc": 0.5, "held_current_a": 2.89817 }))).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let row = &answer["rows"][0];
    assert_eq!(row["outcome"], "unadmitted", "{row}");
    assert!(row["reject_detail"].as_str().unwrap().contains("CurrentChangedDuringDischarge"));
    let (status, text) = post(&base, "/run_dataset", &list(json!({ "soc": 0.5, "held_current_a": 2.9 }))).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "unadmitted", "{text}");
    assert!(answer["rows"][0]["reject_detail"].as_str().unwrap().contains("ContinuationWithoutUpstream"), "{text}");
}

/// A change of exactly 0.83 mA in decimal is inside the band at every
/// current, between segments and from the held current; 0.84 mA is outside
/// it.
#[tokio::test]
async fn a_change_of_exactly_the_band_is_taken_at_every_current() {
    let base = spawn_app().await;
    for (first, later) in [(2.899, 2.89983), (2.89817, 2.899), (2.89982, 2.90065), (2.90065, 2.89982)] {
        completed(&base, &run(full(first), &[first, later], 5)).await;
        // Below full charge the card takes the change; the start carries a
        // discharge on, so without from the door refuses it by name.
        refused_whole(&base, &run(json!({ "soc": 0.9, "held_current_a": first }), &[later], 5), "ContinuationWithoutUpstream").await;
    }
    for (first, later) in [(2.899, 2.89984), (2.89982, 2.89817)] {
        let text = refused_whole(&base, &run(full(first), &[first, later], 5), "CurrentChangedDuringDischarge").await;
        assert!(text.contains("is more than 0.00083 A from the held current"), "{text}");
    }
}

/// Every sentence the card serves about its range and its record is true.
#[tokio::test]
async fn the_description_states_what_was_compared_and_both_scorings() {
    let base = spawn_app().await;
    let card = describe(&base).await;
    let names: Vec<&str> = card["rejects"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"CurrentNotCompared"));
    assert!(names.contains(&"StartNotAtFullCharge"));
    assert!(!names.contains(&"CurrentOutsideCard"));
    let not_compared = card["rejects"].as_array().unwrap().iter()
        .find(|r| r["name"] == "CurrentNotCompared").unwrap()["meaning"].as_str().unwrap().to_owned();
    assert!(not_compared.contains("the held 0.87 A among them, which was compared and failed"), "{not_compared}");
    let text = card.to_string();
    for gone in [
        "though it takes any held current",
        "the card takes any held current",
        "no constant-current discharge at any other rate",
        "not yet repeated by a second party",
        "Known defect",
        "No held-back discharge exists at any rate",
        "no held-back discharge at any other current exists",
        "which initial states",
    ] {
        assert!(!text.contains(gone), "{gone}");
    }
    let limits = card["validation"]["limits"].as_str().unwrap();
    for words in [
        "the 0.87 A holds of the pulse test, which set the resistance",
        "The comparison that passed covered discharges from full charge only",
        "04-27-17 3541_dis5_10p.mat",
        "it failed: the voltage error during the holds was 56.7 millivolts root mean square against the line of 50",
        "left its table at soc 0.03 and 2.66 V (StateOfChargeOutsideCard)",
        "0.87 A is therefore refused: evidence/research/panasonic_18650pf/held_0p87A_comparison",
        "Run by one party and scored twice, the second time by a second party",
        "evidence/checks/second_party/V-07",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    let rule = card["dt_s"]["rule"].as_str().unwrap();
    assert!(rule.contains("Every discharge that passed the comparison started at full charge"), "{rule}");
    assert!(rule.contains("04-27-17 3541_dis5_10p.mat"), "{rule}");
    assert!(rule.contains("A returned state does not carry held_current_a"), "{rule}");
    let result = card["validation"]["result"].as_str().unwrap();
    assert!(result.contains("+0.04 percent at steps of 0.1 s and by +0.014 percent at the protocol's step of 1 s"), "{result}");
    assert!(rule.contains("lands its last step"), "{rule}");
    // the worked examples run as published
    for (key, path) in [("example_verify_step", "/verify_physics"), ("example_run_trajectory", "/run_trajectory")] {
        let (status, text) = post(&base, path, &card[key]).await;
        assert_eq!(status, 200, "{key}: {text}");
        assert!(!text.contains("\"outcome\":\"refused\""), "{key}: {text}");
    }
}
