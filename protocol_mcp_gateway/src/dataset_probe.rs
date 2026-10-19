//! Checks on `run_dataset` that need what an integration test cannot reach:
//! the permit pools, the hold after a job, and the gateway's own sealed form.
//!
//! The hold is `#[cfg(test)]`. `app()` does not sleep.

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde_json::{Value, json};
use tokio::net::TcpListener;

use super::{AppState, dataset, dataset_permit_count, router, sealed_form, trajectory};

#[path = "../tests/dataset_cases/mod.rs"]
mod cases;

const HOLD: Duration = Duration::from_millis(1500);

async fn spawn(state: AppState) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post_text(client: &Client, base: &str, path: &str, body: &str) -> (u16, String) {
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

fn text_of(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

#[test]
fn dataset_jobs_hold_at_most_half_the_pool_and_at_least_one_permit() {
    assert_eq!(dataset_permit_count(1), 1);
    assert_eq!(dataset_permit_count(2), 1);
    assert_eq!(dataset_permit_count(3), 1);
    assert_eq!(dataset_permit_count(4), 2);
    assert_eq!(dataset_permit_count(32), 16);
    let state = AppState::new(4);
    assert_eq!(state.physics_permits.available_permits(), 4);
    assert_eq!(state.dataset_permits.available_permits(), 2);
}

// T7: no permit is taken by any whole-job refusal.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_whole_job_refusal_takes_no_permit_even_when_every_permit_is_held() {
    let state = AppState::new(4);
    let base = spawn(state.clone()).await;
    let client = Client::new();
    let genuine = text_of(&cases::list_request("frames"));
    let (status, answer) = post_text(&client, &base, "/run_dataset", &genuine).await;
    assert_eq!(status, 200, "{answer}");

    // Every permit of both pools is held. A job that asked for one would be 429.
    let physics = Arc::clone(&state.physics_permits).try_acquire_many_owned(4).unwrap();
    let datasets = Arc::clone(&state.dataset_permits).try_acquire_many_owned(2).unwrap();
    let (status, text) = post_text(&client, &base, "/run_dataset", &genuine).await;
    assert_eq!(status, 429, "{text}");

    let mut bodies: Vec<(String, String, &str)> = Vec::new();
    for case in cases::plan_refusals() {
        bodies.push((case.name.to_string(), case.body, case.says));
    }
    for case in cases::limit_refusals(
        dataset::MAX_DATASET_ROWS,
        dataset::MAX_DATASET_STEPS,
        dataset::MAX_DATASET_FRAMES,
        dataset::MAX_INLINE_ROWS,
        dataset::MAX_INLINE_FRAMES,
    ) {
        bodies.push((case.name.to_string(), case.body, case.says));
    }
    for (name, body) in cases::repeated_key_bodies() {
        bodies.push((name.to_string(), body, "repeat"));
    }
    let count = bodies.len();
    for (name, body, says) in bodies {
        let (status, text) = post_text(&client, &base, "/run_dataset", &body).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.contains(says), "{name}: {text}");
        assert_eq!(state.physics_permits.available_permits(), 0, "{name}");
        assert_eq!(state.dataset_permits.available_permits(), 0, "{name}");
    }
    assert!(count >= 14, "{count}");

    // The long form runs the plan again, so with every permit held it is
    // answered as a full pool answers, and holds once one is free.
    let answer: Value = serde_json::from_str(&answer).unwrap();
    let body = json!({ "head": answer["head"], "rows": answer["rows"], "frames_hex": answer["frames_hex"] });
    let (status, text) = post_text(&client, &base, "/verify_dataset", &text_of(&body)).await;
    assert_eq!(status, 429, "{text}");
    assert_eq!(state.physics_permits.available_permits(), 0);
    assert_eq!(state.dataset_permits.available_permits(), 0);

    drop(physics);
    drop(datasets);
    let (status, text) = post_text(&client, &base, "/verify_dataset", &text_of(&body)).await;
    assert_eq!(status, 200, "{text}");
    assert!(text.contains("\"holds\":true"), "{text}");
    assert_eq!(state.physics_permits.available_permits(), 4);
    assert_eq!(state.dataset_permits.available_permits(), 2);
}

// T11
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_dataset_is_busy_when_the_physics_pool_is_full_and_gives_back_what_it_took() {
    let state = AppState::new(4);
    let base = spawn(state.clone()).await;
    let client = Client::new();
    let physics = Arc::clone(&state.physics_permits).try_acquire_many_owned(4).unwrap();
    let body = text_of(&cases::list_request("none"));
    let (status, text) = post_text(&client, &base, "/run_dataset", &body).await;
    assert_eq!(status, 429, "{text}");
    assert_eq!(text, "Too many concurrent physics tasks.");
    // The dataset permit it took first went back.
    assert_eq!(state.dataset_permits.available_permits(), 2);
    drop(physics);
    let (status, text) = post_text(&client, &base, "/run_dataset", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(state.physics_permits.available_permits(), 4);
    assert_eq!(state.dataset_permits.available_permits(), 2);
}

// T11
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_dataset_jobs_hold_a_pool_of_four_and_a_third_is_refused_while_single_runs_go_on() {
    let state = AppState::with_hold(4, HOLD);
    let base = spawn(state.clone()).await;
    let client = Client::builder().timeout(Duration::from_secs(30)).build().unwrap();
    let dataset_body = text_of(&cases::list_request("none"));
    let run_body = text_of(&json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{ "steps": 2, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": 1
    }));

    let mut jobs = Vec::new();
    for _ in 0..2 {
        let (client, base, body) = (client.clone(), base.clone(), dataset_body.clone());
        jobs.push(tokio::spawn(async move { post_text(&client, &base, "/run_dataset", &body).await }));
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(state.dataset_permits.available_permits(), 0);
    assert_eq!(state.physics_permits.available_permits(), 2);

    // A third dataset job finds no dataset permit, though two physics permits are free.
    let (status, text) = post_text(&client, &base, "/run_dataset", &dataset_body).await;
    assert_eq!(status, 429, "{text}");
    assert_eq!(text, "Too many concurrent physics tasks.");
    assert_eq!(state.physics_permits.available_permits(), 2, "the refused job took a physics permit");

    // Two single runs still get the two permits that are left.
    let mut singles = Vec::new();
    for _ in 0..2 {
        let (client, base, body) = (client.clone(), base.clone(), run_body.clone());
        singles.push(tokio::spawn(async move { post_text(&client, &base, "/run_trajectory", &body).await }));
    }
    for single in singles {
        let (status, text) = single.await.unwrap();
        assert_eq!(status, 200, "{text}");
    }
    for job in jobs {
        let (status, text) = job.await.unwrap();
        assert_eq!(status, 200, "{text}");
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(state.physics_permits.available_permits(), 4);
    assert_eq!(state.dataset_permits.available_permits(), 2);
}

// T10
#[test]
fn the_sealed_form_here_is_the_sealed_form_of_the_crate() {
    let worked: Value = serde_json::from_str(
        r#"{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":10,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}"#,
    )
    .unwrap();
    let expected = r#"{"card":"thermal","domain":"energy","dt_s":x3fb999999999999a,"initial":{"temperature_k":x4072526666666666},"record_every":x3ff0000000000000,"segments":[{"load":{"ambient_k":x4072526666666666,"heat_power_w":x3ff0000000000000},"steps":x4024000000000000}]}"#;
    assert_eq!(expected.len(), 253);
    assert_eq!(sealed_form(&worked), expected);
    assert_eq!(genesis_seal::sealed_form::sealed_form(&worked), expected);
    assert_eq!(
        genesis_seal::sealed_form::digest_hex(expected.as_bytes()),
        "8615887c32e2e2edb7d4be74b822e7f64ea384173a9332d97be6278dec255c3d57c9249ca16c5a5d5dfa6c2e43d9baf9"
    );

    let requests = [
        worked,
        // Keys out of order, a nested array, zero and negative zero.
        serde_json::from_str(r#"{"b":[1,2.5,{"z":-0.0,"a":0}],"a":"x","c":null,"d":true}"#).unwrap(),
        // A whole number no 64-bit float holds, and the exponent spellings of one.
        serde_json::from_str(r#"{"n":[9007199254740993,10,10.0,1e1,1E+1,18446744073709551615]}"#).unwrap(),
        // Strings: quote, backslash, solidus, controls, non-ASCII.
        serde_json::from_str(r#"{"s":"a\"b\\c/d\b\t\n\f\r\u0001\u001f é 😀","é":1}"#).unwrap(),
        // Small and large magnitudes, and keys that sort by bytes, not by letters.
        serde_json::from_str(r#"{"x":5e-324,"y":1.7976931348623157e308,"B":1,"a":2,"_":3,"é":4,"z":[]}"#).unwrap(),
        // An expanded row of a dataset.
        serde_json::from_str(&text_of(&json!({
            "domain": "energy", "card": "thermal", "initial": { "temperature_k": 280.0 },
            "dt_s": 0.1, "record_every": 1,
            "segments": [{ "steps": 3, "load": { "heat_power_w": 0.1, "ambient_k": 293.15 } }]
        })))
        .unwrap(),
    ];
    assert!(requests.len() >= 5);
    for request in &requests {
        assert_eq!(
            sealed_form(request),
            genesis_seal::sealed_form::sealed_form(request),
            "{request}"
        );
    }
}

// T10
#[tokio::test]
async fn the_crate_and_verify_tape_agree_on_a_genuine_tape_and_on_edits_to_it() {
    let state = AppState::new(4);
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{ "steps": 6, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": 1
    });
    let response = trajectory::run(&state, request).await.ok().expect("a run");
    let tape: Value = serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();

    let mut edits: Vec<(&str, Value)> = vec![("genuine", tape.clone())];
    let mut chain = tape.clone();
    let frame = chain["frames"][2]["frame_hex"].as_str().unwrap().to_string();
    chain["frames"][2]["frame_hex"] = json!(format!("00{}", &frame[2..]));
    edits.push(("a frame byte", chain));
    let mut state_edit = tape.clone();
    let digit = state_edit["final_state"]["temperature_k_hex"].as_str().unwrap().to_string();
    state_edit["final_state"]["temperature_k_hex"] =
        json!(format!("{}{}", &digit[..15], if digit.ends_with('0') { '1' } else { '0' }));
    edits.push(("a state digit", state_edit));
    let mut seal = tape.clone();
    let text = seal["run_seal"].as_str().unwrap().to_string();
    seal["run_seal"] = json!(format!("{}{}", if text.starts_with('0') { '1' } else { '0' }, &text[1..]));
    edits.push(("the run seal", seal));

    for (name, body) in edits {
        let here = trajectory::check(&body).ok().expect("a report");
        let there = genesis_seal::run::check_tape(&body).expect("a report");
        assert_eq!(here.holds, there.holds, "{name}");
        assert_eq!(here.broken_link, there.broken_link, "{name}");
        assert_eq!(here.cause.as_deref(), there.cause.map(|cause| cause.as_str()), "{name}");
        assert_eq!(here.holds, name == "genuine", "{name}");
    }
}

#[tokio::test]
async fn the_crate_and_verify_tape_refuse_capital_letters_in_a_frame_with_one_sentence() {
    let state = AppState::new(4);
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{ "steps": 4, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": 1
    });
    let response = trajectory::run(&state, request).await.ok().expect("a run");
    let tape: Value = serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap();
    for (index, key) in [(0, "frame_hex"), (1, "link"), (3, "frame_hex"), (2, "link")] {
        let mut edited = tape.clone();
        let text = edited["frames"][index][key].as_str().unwrap().to_uppercase();
        edited["frames"][index][key] = json!(text);
        let here = trajectory::check(&edited).err().expect("a refusal").message().to_string();
        let there = genesis_seal::run::check_tape(&edited).err().expect("a refusal").to_string();
        assert_eq!(here, there, "frames[{index}].{key}");
        assert!(here.contains(&format!("'frames[{index}].{key}'")), "{here}");
    }
    // Two frames in capitals: both name the first.
    let mut two = tape.clone();
    for index in [3, 1] {
        let text = two["frames"][index]["link"].as_str().unwrap().to_uppercase();
        two["frames"][index]["link"] = json!(text);
    }
    let here = trajectory::check(&two).err().expect("a refusal").message().to_string();
    let there = genesis_seal::run::check_tape(&two).err().expect("a refusal").to_string();
    assert_eq!(here, there);
    assert!(here.contains("'frames[1].link'"), "{here}");
}
