//! `POST /run_dataset` and `POST /verify_dataset`.
//!
//! Every row of a dataset is a `run_trajectory` request and carries the run
//! seal that request gets alone. The dataset seal is built by the rules in
//! `genesis_seal`. Checks that need the permit pool, and the check of the
//! sealed form against the crate, are unit tests in `src/dataset_probe.rs`.

mod dataset_cases;

use std::time::{Duration, Instant};

use dataset_cases::{
    SAMPLE_SEED, grid_request, limit_refusals, list_request, plan_refusals, repeated_key_bodies,
    request, run, sample_request,
};
use genesis_seal::dataset::{RowResult, assemble};
use genesis_seal::plan::DatasetRequest;
use genesis_seal::run::{FRAME_LEN, link_0, run_seal, seal_frame, state_digest};
use genesis_seal::sealed_form::digest_hex;
use protocol_mcp_gateway::app;
use protocol_mcp_gateway::dataset::{
    MAX_DATASET_FRAMES, MAX_DATASET_ROWS, MAX_DATASET_STEPS, MAX_INLINE_FRAMES, MAX_INLINE_ROWS,
};
use reqwest::Client;
use serde_json::{Map, Value, json};
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

async fn post_text(base: &str, path: &str, body: &str) -> (u16, String) {
    let client = Client::new();
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
                let text = response.text().await.unwrap();
                return (status, text);
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    post_text(base, path, &serde_json::to_string(body).unwrap()).await
}

/// A dataset that must come back 200, as text and as a value.
async fn dataset(base: &str, body: &Value) -> (String, Value) {
    let (status, text) = post(base, "/run_dataset", body).await;
    assert_eq!(status, 200, "{text}");
    let value: Value = serde_json::from_str(&text).unwrap();
    (text, value)
}

async fn verify(base: &str, head: &Value, rows: &[Value], frames_hex: &Value) -> Value {
    let body = json!({ "head": head, "rows": rows, "frames_hex": frames_hex });
    let (status, text) = post(base, "/verify_dataset", &body).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

fn rows_of(answer: &Value) -> Vec<Value> {
    answer["rows"].as_array().expect("rows").clone()
}

/// The same value with the keys of every object in the opposite order.
fn reversed(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, item) in map.iter().rev() {
                out.insert(key.clone(), reversed(item));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(reversed).collect()),
        other => other.clone(),
    }
}

async fn every_row_seals_as_the_run_alone(plan: Value, rows: usize) {
    let base = spawn_app().await;
    for store in ["frames", "none"] {
        let mut body = plan.clone();
        body["store"] = json!(store);
        let (_, answer) = dataset(&base, &body).await;
        let found = rows_of(&answer);
        assert_eq!(found.len(), rows);
        for row in &found {
            assert_eq!(row["outcome"], "completed", "{row}");
            let (status, alone) = post(&base, "/run_trajectory", &row["request"]).await;
            assert_eq!(status, 200, "{alone}");
            let alone: Value = serde_json::from_str(&alone).unwrap();
            assert_eq!(row["run_seal"], alone["run_seal"], "{store} {row}");
            assert_eq!(row["state_digest"], alone["state_digest"]);
            assert_eq!(row["last_link"], alone["frames"].as_array().unwrap().last().unwrap()["link"]);
        }
    }
}

// T1
#[tokio::test]
async fn every_row_of_a_list_plan_carries_the_run_seal_of_its_request_alone() {
    every_row_seals_as_the_run_alone(list_request("frames"), 3).await;
}

// T1
#[tokio::test]
async fn every_row_of_a_grid_plan_carries_the_run_seal_of_its_request_alone() {
    every_row_seals_as_the_run_alone(grid_request("frames"), 6).await;
}

// T1
#[tokio::test]
async fn every_row_of_a_sample_plan_carries_the_run_seal_of_its_request_alone() {
    every_row_seals_as_the_run_alone(sample_request("frames"), 5).await;
}

// T2
#[tokio::test]
async fn the_same_request_gives_the_same_bytes_twice_and_from_two_gateways() {
    let first = spawn_app().await;
    let second = spawn_app().await;
    for plan in [list_request("frames"), grid_request("none"), sample_request("frames")] {
        let (one, _) = dataset(&first, &plan).await;
        let (again, _) = dataset(&first, &plan).await;
        let (other, _) = dataset(&second, &plan).await;
        assert_eq!(one, again);
        assert_eq!(one, other);
    }
}

// T3
#[tokio::test]
async fn the_order_of_keys_at_every_depth_does_not_change_the_dataset_seal() {
    let base = spawn_app().await;
    for plan in [list_request("frames"), grid_request("frames"), sample_request("none")] {
        let (_, plain) = dataset(&base, &plan).await;
        let (_, flipped) = dataset(&base, &reversed(&plan)).await;
        assert_ne!(
            serde_json::to_string(&plan).unwrap(),
            serde_json::to_string(&reversed(&plan)).unwrap()
        );
        for key in ["dataset_seal", "merkle_root", "plan_sha256"] {
            assert_eq!(plain["head"][key], flipped["head"][key], "{key}");
        }
        assert_eq!(plain["rows"], flipped["rows"]);
        assert_eq!(plain["frames_hex"], flipped["frames_hex"]);
    }
}

// T4
#[tokio::test]
async fn the_last_axis_of_a_grid_varies_fastest() {
    let base = spawn_app().await;
    let (_, answer) = dataset(&base, &grid_request("none")).await;
    let rows = rows_of(&answer);
    let expect = |temperature_k: f64, heat_power_w: f64| {
        json!({
            "domain": "energy",
            "card": "thermal",
            "initial": { "temperature_k": temperature_k },
            "dt_s": 0.1,
            "segments": [{
                "steps": 3,
                "load": { "heat_power_w": heat_power_w, "ambient_k": 293.15 }
            }],
            "record_every": 1
        })
    };
    assert_eq!(rows[0]["request"], expect(280.0, 0.0));
    assert_eq!(rows[1]["request"], expect(280.0, 5.0));
    assert_eq!(rows[2]["request"], expect(290.0, 0.0));
    assert_eq!(rows[rows.len() - 1]["request"], expect(300.0, 5.0));
    // The keys of an expanded request are in the order of their bytes.
    let keys: Vec<&String> = rows[0]["request"].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["card", "domain", "dt_s", "initial", "record_every", "segments"]);
}

fn sampler_vectors() -> Vec<Vec<String>> {
    let path = format!("{}/../genesis_seal/vectors/sampler.txt", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{path}: {error}"))
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| line.split_whitespace().map(str::to_string).collect())
        .collect()
}

fn bits(text: &str) -> u64 {
    u64::from_str_radix(text, 16).unwrap()
}

// T5
#[tokio::test]
async fn a_sample_row_is_worked_out_alone_and_follows_the_published_vectors() {
    let base = spawn_app().await;
    let plan = sample_request("none");
    let (_, answer) = dataset(&base, &plan).await;
    let rows = rows_of(&answer);
    let mut sealed = plan.clone();
    sealed.as_object_mut().unwrap().remove("answer");
    let expanded = DatasetRequest::from_value(sealed).unwrap();
    // Row 3 from the crate, with no call for rows 0 to 2, is the gateway's row 3.
    assert_eq!(expanded.row(3).unwrap(), rows[3]["request"]);
    assert_eq!(expanded.row(4).unwrap(), rows[4]["request"]);
    assert_ne!(rows[3]["request"], rows[4]["request"]);

    // Seed 0123456789abcdef, row 0, draw 0: each published uniform vector.
    let vectors = sampler_vectors();
    let mut checked = 0;
    for line in vectors.iter().filter(|line| {
        line[0] == "uniform" && line[1] == SAMPLE_SEED && line[2] == "0" && line[3] == "0"
    }) {
        let low = f64::from_bits(bits(&line[6]));
        let high = f64::from_bits(bits(&line[7]));
        let plan = request(
            json!({
                "kind": "sample",
                "base": {
                    "initial": { "temperature_k": 293.15 },
                    "dt_s": 0.1,
                    "segments": [{ "steps": 1, "load": { "ambient_k": 293.15 } }],
                    "record_every": 1
                },
                "rows": 1,
                "generator": "splitmix64_counter_v1",
                "seed_hex": SAMPLE_SEED,
                "draws": [{ "path": "segments[0].load.heat_power_w", "uniform": { "low": low, "high": high } }]
            }),
            "none",
            "inline",
        );
        let (_, answer) = dataset(&base, &plan).await;
        let drawn = answer["rows"][0]["request"]["segments"][0]["load"]["heat_power_w"]
            .as_f64()
            .unwrap();
        assert_eq!(drawn.to_bits(), bits(&line[8]), "vector {line:?}");
        checked += 1;
    }
    assert_eq!(checked, 5);

    // Seed 0123456789abcdef, row 1, draw 1: choice among 3 gives index 2.
    let choice = vectors
        .iter()
        .find(|line| {
            line[0] == "choice" && line[1] == SAMPLE_SEED && line[2] == "1" && line[3] == "1" && line[5] == "3"
        })
        .expect("a published choice vector");
    let index: usize = choice[6].parse().unwrap();
    let plan = request(
        json!({
            "kind": "sample",
            "base": {
                "initial": { "temperature_k": 293.15 },
                "segments": [{ "steps": 1, "load": { "ambient_k": 293.15 } }],
                "record_every": 1
            },
            "rows": 2,
            "generator": "splitmix64_counter_v1",
            "seed_hex": SAMPLE_SEED,
            "draws": [
                { "path": "segments[0].load.heat_power_w", "uniform": { "low": 0.0, "high": 1.0 } },
                { "path": "dt_s", "choice": [0.05, 0.1, 0.2] }
            ]
        }),
        "none",
        "inline",
    );
    let (_, answer) = dataset(&base, &plan).await;
    let choices = [0.05, 0.1, 0.2];
    assert_eq!(answer["rows"][1]["request"]["dt_s"].as_f64().unwrap(), choices[index]);
}

// T6
#[tokio::test]
async fn a_refused_run_and_an_unadmitted_row_are_rows_and_the_dataset_still_seals() {
    let base = spawn_app().await;
    let hot = run(390.0, 3000.0, 60);
    // A heat power no frame slot holds: the door refuses it before computing.
    let too_large = run(290.0, 1.0e39, 3);
    let plan = request(
        json!({ "kind": "list", "runs": [run(290.0, 1.0, 3), hot.clone(), too_large.clone()] }),
        "frames",
        "inline",
    );
    let (_, answer) = dataset(&base, &plan).await;
    let rows = rows_of(&answer);
    assert_eq!(rows[0]["outcome"], "completed");

    assert_eq!(rows[1]["outcome"], "refused", "{}", rows[1]);
    assert!(rows[1]["run_seal"].is_string());
    assert!(rows[1]["steps_landed"].as_u64().unwrap() > 0);
    let mut alone = hot;
    alone["domain"] = json!("energy");
    alone["card"] = json!("thermal");
    let (status, text) = post(&base, "/run_trajectory", &alone).await;
    assert_eq!(status, 200);
    let alone: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(alone["outcome"], "refused");
    assert_eq!(rows[1]["run_seal"], alone["run_seal"]);
    assert_eq!(rows[1]["reject_detail"], alone["reject_detail"]);

    assert_eq!(rows[2]["outcome"], "unadmitted", "{}", rows[2]);
    assert!(rows[2]["run_seal"].is_null());
    assert_eq!(rows[2]["frames"], 0);
    let (status, sentence) = post(&base, "/run_trajectory", &rows[2]["request"]).await;
    assert_eq!(status, 422);
    assert_eq!(rows[2]["reject_detail"], sentence);
    assert!(rows[2]["leaf"]["run_seal"].is_null());
    assert_eq!(rows[2]["leaf"]["outcome"], "unadmitted");

    let head = &answer["head"];
    assert_eq!(head["rows"], 3);
    assert_eq!(head["completed"], 1);
    assert_eq!(head["refused"], 1);
    assert_eq!(head["unadmitted"], 1);

    let report = verify(&base, head, &rows, &answer["frames_hex"]).await;
    assert_eq!(report["holds"], true, "{report}");
    assert_eq!(report["cause"], Value::Null);
}

// T7
#[tokio::test]
async fn a_fault_in_the_plan_refuses_the_whole_job_and_seals_nothing() {
    let base = spawn_app().await;
    for case in plan_refusals() {
        let (status, text) = post_text(&base, "/run_dataset", &case.body).await;
        assert_eq!(status, 422, "{}: {text}", case.name);
        assert!(text.contains(case.says), "{}: {text}", case.name);
        for sealed in ["dataset_seal", "run_seal", "merkle_root", "\"head\""] {
            assert!(!text.contains(sealed), "{} sealed something: {text}", case.name);
        }
    }
    for (name, body) in repeated_key_bodies() {
        let (status, text) = post_text(&base, "/run_dataset", &body).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.contains("repeat"), "{name}: {text}");
        assert!(!text.contains("dataset_seal"), "{name}: {text}");
    }
}

// T7
#[tokio::test]
async fn each_limit_of_a_job_refuses_it_before_anything_runs() {
    let base = spawn_app().await;
    let started = Instant::now();
    for case in limit_refusals(
        MAX_DATASET_ROWS,
        MAX_DATASET_STEPS,
        MAX_DATASET_FRAMES,
        MAX_INLINE_ROWS,
        MAX_INLINE_FRAMES,
    ) {
        let (status, text) = post_text(&base, "/run_dataset", &case.body).await;
        assert_eq!(status, 422, "{}: {text}", case.name);
        assert!(text.contains(case.says), "{}: {text}", case.name);
        assert!(!text.contains("dataset_seal"), "{}: {text}", case.name);
    }
    // Over a hundred million steps were asked for in all. None were taken.
    assert!(started.elapsed() < Duration::from_secs(60));
}

// T7
#[tokio::test]
async fn an_inline_job_that_stores_nothing_still_has_a_limit_on_recorded_frames() {
    let base = spawn_app().await;
    let run = json!({
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{ "steps": 10000, "load": { "ambient_k": 293.15, "heat_power_w": 1.0 } }],
        "record_every": 1
    });
    // Two rows of 10,000 recorded frames each: 20,000 frames held, none returned.
    let over = request(json!({ "kind": "list", "runs": [run.clone(), run.clone()] }), "none", "inline");
    let (status, text) = post(&base, "/run_dataset", &over).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("an inline dataset would record more than 10000 frames."), "{text}");
    assert!(!text.contains("dataset_seal"), "{text}");
    // One such row is on the limit and runs.
    let on = request(json!({ "kind": "list", "runs": [run] }), "none", "inline");
    let (status, text) = post(&base, "/run_dataset", &on).await;
    assert_eq!(status, 200, "{text}");
}

// T7
#[tokio::test]
async fn a_bound_of_the_card_is_refused_and_not_pulled_back() {
    let base = spawn_app().await;
    let plan = request(
        json!({
            "kind": "sample",
            "base": {
                "initial": {},
                "dt_s": 0.1,
                "segments": [{ "steps": 1, "load": { "ambient_k": 293.15 } }],
                "record_every": 1
            },
            "rows": 3,
            "generator": "splitmix64_counter_v1",
            "seed_hex": SAMPLE_SEED,
            "draws": [
                { "path": "initial.temperature_k", "uniform": { "low": 250.0, "high": 398.15 } },
                { "path": "segments[0].load.heat_power_w", "uniform": { "low": 0.0, "high": 1.0 } }
            ]
        }),
        "none",
        "inline",
    );
    let (status, text) = post(&base, "/run_dataset", &plan).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("initial.temperature_k"), "{text}");
    assert!(text.contains("outside the card's published bounds"), "{text}");
}

// T8
#[tokio::test]
async fn verify_dataset_holds_on_a_genuine_answer_and_says_how_chains_were_checked() {
    let base = spawn_app().await;
    for plan in [list_request("frames"), grid_request("frames"), sample_request("frames")] {
        let (_, answer) = dataset(&base, &plan).await;
        let report = verify(&base, &answer["head"], &rows_of(&answer), &answer["frames_hex"]).await;
        assert_eq!(report["holds"], true, "{report}");
        assert_eq!(report["cause"], Value::Null);
        assert_eq!(report["chains"], "rebuilt from the frames");
        assert_eq!(report["receipt"], "matches");
        assert_eq!(report["rows_file"], "matches the hash in the head");
        assert_eq!(report["frames_file"], "matches the hash in the head");
        assert_eq!(report["merkle_root"], answer["head"]["merkle_root"]);
        assert_eq!(report["dataset_seal"], answer["head"]["dataset_seal"]);
        // Without the frames the chains are checked from the last link on.
        let without = verify(&base, &answer["head"], &rows_of(&answer), &Value::Null).await;
        assert_eq!(without["holds"], true, "{without}");
        assert_eq!(without["chains"], "not rebuilt: the frames were not given");
        assert_eq!(without["frames_file"], "not given");
    }
    for plan in [list_request("none"), grid_request("none"), sample_request("none")] {
        let (_, answer) = dataset(&base, &plan).await;
        assert_eq!(answer["frames_hex"], Value::Null);
        assert_eq!(answer["head"]["frames_sha256"], Value::Null);
        let report = verify(&base, &answer["head"], &rows_of(&answer), &answer["frames_hex"]).await;
        assert_eq!(report["holds"], true, "{report}");
        assert_eq!(report["chains"], "not rebuilt: the plan stores no frames");
        assert_eq!(report["receipt"], "matches");
    }
}

fn flip_digit(text: &str, at: usize) -> String {
    let mut bytes = text.as_bytes().to_vec();
    bytes[at] = if bytes[at] == b'0' { b'1' } else { b'0' };
    String::from_utf8(bytes).unwrap()
}

struct Broken {
    cause: &'static str,
    row: Option<u64>,
}

async fn assert_break(base: &str, name: &str, head: &Value, rows: &[Value], frames: &Value, want: Broken) {
    let report = verify(base, head, rows, frames).await;
    assert_eq!(report["holds"], false, "{name}: {report}");
    assert_eq!(report["cause"], want.cause, "{name}: {report}");
    assert_eq!(report["row"], json!(want.row), "{name}: {report}");
}

// T9
#[tokio::test]
async fn verify_dataset_names_the_first_break_for_each_attack() {
    let base = spawn_app().await;
    let (_, answer) = dataset(&base, &list_request("frames")).await;
    let head = answer["head"].clone();
    let rows = rows_of(&answer);
    let frames = answer["frames_hex"].clone();
    assert_eq!(rows.len(), 3);

    let mut swapped = rows.clone();
    swapped.swap(0, 1);
    assert_break(&base, "two rows swapped", &head, &swapped, &frames, Broken { cause: "plan", row: Some(0) }).await;

    let mut dropped = rows.clone();
    dropped.remove(1);
    assert_break(&base, "a row dropped", &head, &dropped, &frames, Broken { cause: "plan", row: None }).await;

    // The doubled last leaf: the same head and the last row twice.
    let mut doubled = rows.clone();
    doubled.push(rows[2].clone());
    assert_break(&base, "the last row repeated", &head, &doubled, &frames, Broken { cause: "plan", row: None }).await;

    // A row of another plan with the same card.
    let (_, other) = dataset(&base, &grid_request("frames")).await;
    let mut foreign = rows.clone();
    foreign[1] = rows_of(&other)[1].clone();
    assert_break(&base, "a row from another plan", &head, &foreign, &frames, Broken { cause: "plan", row: Some(1) }).await;

    // One byte of the body of the first frame of row 0. It shows at frame 1.
    let mut text = frames.as_str().unwrap().to_string();
    let at = 2 * 64 + 4;
    text = flip_digit(&text, at);
    let report = verify(&base, &head, &rows, &Value::String(text)).await;
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["cause"], "chain", "{report}");
    assert_eq!(report["row"], 0);
    assert_eq!(report["broken_link"], 1);

    let mut edited = rows.clone();
    let digit = edited[1]["final_state"]["temperature_k_hex"].as_str().unwrap().to_string();
    edited[1]["final_state"]["temperature_k_hex"] = json!(flip_digit(&digit, 15));
    assert_break(&base, "a state hex digit", &head, &edited, &frames, Broken { cause: "state", row: Some(1) }).await;

    let mut edited = rows.clone();
    let seal = edited[1]["run_seal"].as_str().unwrap().to_string();
    edited[1]["run_seal"] = json!(flip_digit(&seal, 0));
    assert_break(&base, "a run seal", &head, &edited, &frames, Broken { cause: "run_seal", row: Some(1) }).await;

    let mut edited = rows.clone();
    let leaf = edited[1]["leaf"]["request_sha256"].as_str().unwrap().to_string();
    edited[1]["leaf"]["request_sha256"] = json!(flip_digit(&leaf, 0));
    assert_break(&base, "a leaf", &head, &edited, &frames, Broken { cause: "leaf", row: Some(1) }).await;

    let mut edited = head.clone();
    let root = edited["merkle_root"].as_str().unwrap().to_string();
    edited["merkle_root"] = json!(flip_digit(&root, 0));
    assert_break(&base, "the merkle root", &edited, &rows, &frames, Broken { cause: "root", row: None }).await;
    let report = verify(&base, &edited, &rows, &frames).await;
    assert_eq!(report["receipt"], "does not match");

    let mut edited = head.clone();
    edited["completed"] = json!(edited["completed"].as_u64().unwrap() + 1);
    assert_break(&base, "a count in the head", &edited, &rows, &frames, Broken { cause: "dataset_seal", row: None }).await;

    let mut edited = head.clone();
    let seal = edited["dataset_seal"].as_str().unwrap().to_string();
    edited["dataset_seal"] = json!(flip_digit(&seal, 0));
    assert_break(&base, "the dataset seal", &edited, &rows, &frames, Broken { cause: "dataset_seal", row: None }).await;
}

#[tokio::test]
async fn verify_dataset_refuses_what_it_cannot_read() {
    let base = spawn_app().await;
    let (_, answer) = dataset(&base, &list_request("frames")).await;
    let good = json!({ "head": answer["head"], "rows": answer["rows"], "frames_hex": answer["frames_hex"] });
    let cases = [
        ("an unknown key", { let mut b = good.clone(); b["extra"] = json!(1); b }, "'extra' is not a field"),
        ("no head", { let mut b = good.clone(); b.as_object_mut().unwrap().remove("head"); b }, "Missing required parameter: 'head'"),
        ("no frames_hex", { let mut b = good.clone(); b.as_object_mut().unwrap().remove("frames_hex"); b }, "Missing required parameter: 'frames_hex'"),
        ("capital hex", { let mut b = good.clone(); b["frames_hex"] = json!("AB"); b }, "lowercase hex"),
        ("odd hex", { let mut b = good.clone(); b["frames_hex"] = json!("abc"); b }, "lowercase hex"),
        ("rows not an array", { let mut b = good.clone(); b["rows"] = json!({}); b }, "'rows' must be a JSON array"),
        ("no rows", { let mut b = good.clone(); b["rows"] = json!([]); b }, "one row on each line"),
    ];
    for (name, body, says) in cases {
        let (status, text) = post(&base, "/verify_dataset", &body).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.contains(says), "{name}: {text}");
    }
    let repeated = r#"{"head":{},"head":{},"rows":[],"frames_hex":null}"#;
    let (status, text) = post_text(&base, "/verify_dataset", repeated).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("repeat"), "{text}");
}

/// A run with no engine behind it: made-up frames, made-up state, every
/// seal built by the published rules.
fn fabricated_run(request: Value, steps: u64) -> Value {
    let mut link = link_0(&request);
    let mut frames = Vec::new();
    for step in 1..=steps {
        let mut frame = [7_u8; FRAME_LEN];
        frame[0..8].copy_from_slice(&step.to_le_bytes());
        link = seal_frame(&link, &mut frame);
        frames.push(json!({ "step": step, "frame_hex": hex::encode(frame), "link": hex::encode(link) }));
    }
    let final_state = json!({
        "temperature_k": 1000000.0,
        "temperature_k_hex": "412e848000000000",
        "derated": false
    });
    let residual_hex = "0000000000000000";
    let digest = state_digest(final_state.as_object().unwrap(), residual_hex, residual_hex).unwrap();
    let seal = run_seal(&link, "completed", steps, None, Some(&digest));
    json!({
        "inputs": request,
        "engine_version": "no_engine",
        "outcome": "completed",
        "steps_requested": steps,
        "steps_landed": steps,
        "reject": null,
        "reject_detail": null,
        "final_state": final_state,
        "final_residual_conservation_error": 0.0,
        "final_residual_conservation_error_hex": residual_hex,
        "max_residual_conservation_error": 0.0,
        "max_residual_conservation_error_hex": residual_hex,
        "state_digest": digest,
        "frames": frames,
        "run_seal": seal
    })
}

// T13
#[tokio::test]
async fn a_dataset_made_without_the_engine_holds_and_a_replayed_row_exposes_it() {
    let base = spawn_app().await;
    let mut sealed = list_request("frames");
    sealed.as_object_mut().unwrap().remove("answer");
    let plan = DatasetRequest::from_value(sealed).unwrap();
    let results: Vec<RowResult> = (0..plan.totals().rows)
        .map(|index| {
            let request = plan.row(index).unwrap();
            let steps = request["segments"][0]["steps"].as_u64().unwrap();
            RowResult::Run(fabricated_run(request, steps))
        })
        .collect();
    let mut made = assemble(&plan, "no_engine", &results).unwrap();
    made.head.rows_sha256 = Some(digest_hex(&made.rows_text()));
    made.head.frames_sha256 = made.frames_bytes().map(|bytes| digest_hex(&bytes));
    let head: Value = serde_json::from_str(&made.head.wire()).unwrap();
    let rows: Vec<Value> = made.rows.iter().map(|row| row.to_value()).collect();
    let frames = Value::String(hex::encode(made.frames_bytes().unwrap()));

    let report = verify(&base, &head, &rows, &frames).await;
    assert_eq!(report["holds"], true, "an unkeyed seal holds without the engine: {report}");
    assert_eq!(report["cause"], Value::Null);

    // Replaying any row through the engine gives another run seal.
    for row in &rows {
        let (status, text) = post(&base, "/run_trajectory", &row["request"]).await;
        assert_eq!(status, 200, "{text}");
        let replay: Value = serde_json::from_str(&text).unwrap();
        assert_ne!(replay["run_seal"], row["run_seal"], "{row}");
    }
}

// T14
#[tokio::test]
async fn the_program_in_genesis_seal_verifies_what_the_door_returned() {
    let base = spawn_app().await;
    let (_, answer) = dataset(&base, &grid_request("frames")).await;
    let dir = std::env::temp_dir().join(format!("run_dataset_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let head = serde_json::to_vec(&answer["head"]).unwrap();
    let mut rows = Vec::new();
    for row in answer["rows"].as_array().unwrap() {
        rows.extend_from_slice(serde_json::to_string(row).unwrap().as_bytes());
        rows.push(b'\n');
    }
    let frames = hex::decode(answer["frames_hex"].as_str().unwrap()).unwrap();
    std::fs::write(dir.join("head.json"), &head).unwrap();
    std::fs::write(dir.join("rows.jsonl"), &rows).unwrap();
    std::fs::write(dir.join("frames.bin"), &frames).unwrap();

    // The program is built by the genesis_seal crate's own tests. Use it when it is there.
    let program = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../genesis_seal/target/debug/genesis_seal");
    if program.is_file() {
        let output = std::process::Command::new(&program)
            .args(["verify-dataset"])
            .arg(dir.join("head.json"))
            .arg(dir.join("rows.jsonl"))
            .arg(dir.join("frames.bin"))
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        assert_eq!(output.status.code(), Some(0), "{text} {}", String::from_utf8_lossy(&output.stderr));
        println!("T14: ran the program {}\n{text}", program.display());
    } else {
        let report = genesis_seal::dataset::check_dataset(&head, &rows, Some(&frames)).unwrap();
        assert!(report.holds);
        assert_eq!(report.rows_file, genesis_seal::dataset::FileCheck::Matches);
        assert_eq!(report.frames_file, genesis_seal::dataset::FileCheck::Matches);
        println!("T14: the program was not built; called check_dataset on the file bytes");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// One dataset of 1,000 rows of 100 steps, the last frame of each row kept.
/// Run by hand: `cargo test --offline --release -- --ignored --nocapture`.
#[tokio::test]
#[ignore]
async fn one_thousand_rows_of_one_hundred_steps_take_this_long() {
    let base = spawn_app().await;
    let powers: Vec<f64> = (0..1000).map(|i| i as f64 * 0.01).collect();
    let plan = request(
        json!({
            "kind": "grid",
            "base": {
                "initial": { "temperature_k": 293.15 },
                "dt_s": 0.1,
                "segments": [{ "steps": 100, "load": { "ambient_k": 293.15 } }],
                "record_every": 100
            },
            "axes": [{ "path": "segments[0].load.heat_power_w", "values": powers }]
        }),
        "frames",
        "inline",
    );
    let started = Instant::now();
    let (text, answer) = dataset(&base, &plan).await;
    let seconds = started.elapsed().as_secs_f64();
    assert_eq!(answer["head"]["rows"], 1000);
    println!("M7: 1000 rows x 100 steps, one frame each: {seconds:.3} s, {} bytes", text.len());
}
