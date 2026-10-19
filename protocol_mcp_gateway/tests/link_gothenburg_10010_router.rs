//! The Gothenburg microwave link card on the three connections.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes, and its receipt verifies. A rain event sent minute by minute, one
//! segment a minute, ends on the kernel loop's state to the bit, each frame
//! carries the flag of its own minute, and the tape verifies. Each bound
//! lands at the last number inside and is refused by name at the next
//! outside, in a step, at a start and in a later segment. A missing input, a
//! field the card does not have and a card constant sent as a field are
//! refused. `validate_series` compares the path loss. The local, the web and
//! the hosted connection return the same bytes.
//!
//! The rain rates of the event are the first twelve minutes of the fitted
//! event of 2015-07-28, from the research record; every other number is a
//! round number chosen for the test. No received level of the link is read
//! here.

mod answer_text;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::link_gothenburg_10010::{
    self, LinkGothenburg10010, LinkLoad, LinkState, StepReject,
};
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof, message_number};
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

// ------------------------------------------------------------------ helpers

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
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
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
                answer_text::remember(&text);
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
    post_text(base, path, &answer_text::as_sent(body)).await
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The next 64-bit number away from zero from a positive `x`.
fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

/// The next 64-bit number toward zero from a positive `x`.
fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn link() -> LinkGothenburg10010 {
    LinkGothenburg10010::sublink_587()
}

const MINUTE_S: f64 = 60.0;
const DRY_DB: f64 = 52.325;
/// The gauge's rain rate in the first twelve minutes of the fitted event that
/// starts 2015-07-28T14:26:00Z, in mm/h.
const EVENT_MM_H: [f64; 12] = [6.0, 18.0, 18.0, 18.0, 6.0, 6.0, 6.0, 6.0, 18.0, 6.0, 6.0, 12.0];
/// The excess attenuation of those minutes by the forecast's own model, in dB.
const EVENT_EXCESS_DB: [f64; 12] = [
    2.8205208827748818,
    6.205202544543754,
    6.458529139795127,
    6.579305286605741,
    3.7835545908694073,
    3.8110070932263325,
    3.8240953655085557,
    3.83033533843002,
    6.686642319260911,
    3.8347286620375485,
    3.8354048755680052,
    5.301757370842308,
];

fn step_body(dry_db: f64, water_db: f64, rain_mm_h: f64, dt_s: f64) -> Value {
    json!({
        "domain": "signal",
        "card": "link_gothenburg_10010",
        "parameters": {
            "dry_path_loss_db": dry_db,
            "antenna_water_db": water_db,
            "gauge_minute_elapsed_s": 0.0,
            "gauge_minute_rain_mm_h": 0.0,
            "rain_rate_mm_h": rain_mm_h,
            "dt_s": dt_s
        }
    })
}

/// One segment a minute, each carrying its own rain rate.
fn run_body(dry_db: f64, water_db: f64, rain_mm_h: &[f64], dt_s: f64) -> Value {
    let segments: Vec<Value> = rain_mm_h
        .iter()
        .map(|rate| json!({ "steps": 1, "load": { "rain_rate_mm_h": rate } }))
        .collect();
    json!({
        "domain": "signal",
        "card": "link_gothenburg_10010",
        "initial": {
            "dry_path_loss_db": dry_db,
            "antenna_water_db": water_db,
            "gauge_minute_elapsed_s": 0.0,
            "gauge_minute_rain_mm_h": 0.0
        },
        "dt_s": dt_s,
        "segments": segments,
        "record_every": 1
    })
}

fn blank_frame(proof: [u8; 16]) -> LastStateFrame64 {
    LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof,
    }
}

/// The kernel loop: one step for each rain rate, the state and the frame kept.
fn direct_loop(dry_db: f64, water_db: f64, rain_mm_h: &[f64], dt_s: f64) -> Vec<(LinkState, LastStateFrame64)> {
    let link = link();
    let mut state = LinkState::try_new(&link, dry_db, water_db).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut kept = Vec::new();
    for rate in rain_mm_h {
        link_gothenburg_10010::step(&link, &mut state, &mut frame, LinkLoad { rain_rate_mm_h: *rate }, dt_s).unwrap();
        kept.push((state, frame));
    }
    kept
}

/// The published number of a state, by its name, from the hex beside it.
fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn state_numbers(state: &LinkState) -> Vec<(&'static str, f64)> {
    vec![
        ("t_s", state.t_s),
        ("dry_path_loss_db", state.dry_path_loss_db),
        ("antenna_water_db", state.antenna_water_db),
        ("rain_rate_mm_h", state.rain_rate_mm_h),
        ("rain_attenuation_db", state.rain_attenuation_db),
        ("path_loss_db", state.path_loss_db),
        ("received_level_dbm", state.received_level_dbm),
        ("received_power_w", state.received_power_w),
        ("dry_path_power_w", state.dry_path_power_w),
        ("dry_path_energy_j", state.dry_path_energy_j),
        ("received_energy_j", state.received_energy_j),
        ("rain_energy_j", state.rain_energy_j),
        ("antenna_energy_j", state.antenna_energy_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
        ("antenna_account_anchor_db", state.antenna_account_anchor_db),
        ("antenna_built_db", state.antenna_built_db),
        ("antenna_dried_db", state.antenna_dried_db),
        ("antenna_account_defect_db", state.antenna_account_defect_db),
        ("antenna_account_error", state.antenna_account_error),
        ("rain_rate_margin_mm_h", state.rain_rate_margin_mm_h),
        ("gauge_minute_elapsed_s", state.gauge_minute_elapsed_s),
        ("gauge_minute_rain_mm_h", state.gauge_minute_rain_mm_h),
    ]
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// both as the decimal and as the hex beside it; the flag is the kernel's; and
/// nothing else is published.
fn assert_state_is(published_state: &Value, state: &LinkState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in state_numbers(state) {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    assert_eq!(object["raining"], state.raining, "{label}");
    keys.insert("raining".to_string());
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha256\":\"";
    let at = text.rfind(MARKER).expect("a receipt");
    (format!("{}}}", &text[..at]), text[at + MARKER.len()..text.len() - 2].to_string())
}

/// The words of a value outside a bound the card publishes.
fn outside(field: &str, value: f64, unit: &str, upper: bool, limit: f64, name: &str) -> String {
    let (side, kind) = if upper { ("above", "maximum") } else { ("below", "minimum") };
    format!(
        "Physical Failure: '{field}' {} {unit} is {side} {} {unit}, the inclusive {kind} of the link_gothenburg_10010 card ({name}). That state is not sealed.",
        message_number(value),
        message_number(limit)
    )
}

// -------------------------------------------------------------- the step

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_link_matches_the_compiled_step_with_the_same_proof_bytes_and_its_receipt_verifies() {
    let base = spawn_app().await;
    let floor = link().floor_dt_s().unwrap();
    let cases = [
        // A wet minute from a dry link.
        step_body(52.3, 0.0, 12.0, MINUTE_S),
        // The heaviest rain of the fitting data on antennas already soaked.
        step_body(54.0, 2.13, 84.0, MINUTE_S),
        // A dry minute on wet antennas, at the lowest dry level.
        step_body(50.0, 1.5, 0.0, MINUTE_S),
        // One count of the gauge, at the step floor.
        step_body(52.0, 0.25, 6.0, floor),
        // A dry link in no rain: nothing moves, and the step lands.
        step_body(52.3, 0.0, 0.0, 1.0),
    ];
    let mut seals = Vec::new();
    for request in cases {
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);

        let parameters = &request["parameters"];
        let number = |key: &str| parameters[key].as_f64().unwrap();
        let link = link();
        let mut state = LinkState::try_new(&link, number("dry_path_loss_db"), number("antenna_water_db")).unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        let load = LinkLoad { rain_rate_mm_h: number("rain_rate_mm_h") };
        link_gothenburg_10010::step(&link, &mut state, &mut frame, load, number("dt_s")).unwrap();
        assert_eq!(frame.flags & !link_gothenburg_10010::FLAG_RAINING, 0);
        assert_eq!(frame.flags != 0, number("rain_rate_mm_h") > 0.0);
        assert_state_is(&response.state, &state, "step");
        assert_eq!(
            response.residual_conservation_error.to_bits(),
            state.residual_conservation_error.to_bits()
        );
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        let mut naked = frame;
        naked.proof = [0; 16];
        assert_ne!(response.sha256_seal, sha256_hex(&naked.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha256);
        assert_eq!(sha256_hex(document.as_bytes()), receipt);
        let (status, report) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], true, "{report}");
        let changed = answer.replacen("\"t_s\":", "\"t_s\":1", 1);
        assert_ne!(changed, answer);
        let (_, report) = post(&base, "/verify_receipt", &json!({ "response_text": changed })).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], false, "{report}");
        let (_, again) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(again, answer);
        seals.push(response.sha256_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 5, "five different steps, five different proofs");
}

/// A step handed back as a new start continues the same link: the water on
/// the antennas after two minutes taken one at a time is that of two minutes
/// in one run.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_handed_back_as_a_new_start_continues_the_same_link() {
    let base = spawn_app().await;
    let (_, first) = post(&base, "/verify_physics", &step_body(DRY_DB, 0.0, 18.0, MINUTE_S)).await;
    let first: Value = serde_json::from_str(&first).unwrap();
    let water = published(&first["state"], "antenna_water_db");
    let (status, second) = post(&base, "/verify_physics", &step_body(DRY_DB, water, 0.0, MINUTE_S)).await;
    assert_eq!(status, 200, "{second}");
    let second: Value = serde_json::from_str(&second).unwrap();
    let kept = direct_loop(DRY_DB, 0.0, &[18.0, 0.0], MINUTE_S);
    assert_eq!(
        published(&second["state"], "antenna_water_db").to_bits(),
        kept[1].0.antenna_water_db.to_bits()
    );
    assert_eq!(published(&second["state"], "path_loss_db").to_bits(), kept[1].0.path_loss_db.to_bits());
    // The clock and the ledgers start again at each call.
    assert_eq!(published(&second["state"], "t_s"), MINUTE_S);
}

// --------------------------------------------------------------- the event

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rain_event_sent_minute_by_minute_ends_on_the_kernel_loop_and_its_tape_verifies() {
    let base = spawn_app().await;
    let run = run_body(DRY_DB, 0.0, &EVENT_MM_H, MINUTE_S);
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["outcome"], "completed");
    assert_eq!(answer["steps_landed"], 12);
    assert!(answer["reject"].is_null());
    let kept = direct_loop(DRY_DB, 0.0, &EVENT_MM_H, MINUTE_S);
    assert_state_is(&answer["final_state"], &kept[11].0, "the twelfth minute");
    let frames = answer["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 12);
    for (index, recorded) in frames.iter().enumerate() {
        let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
        let (state, frame) = &kept[index];
        // Everything but the 16 bytes of the proof is the kernel's frame.
        assert_eq!(bytes[..48], frame.to_bytes()[..48], "minute {}", index + 1);
        assert_eq!(recorded["step"], index as u64 + 1);
        let path_loss = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert_eq!(path_loss, state.path_loss_db as f32);
        // The forecast's own model, written in plain Python, to a 32-bit slot.
        let excess = state.path_loss_db - DRY_DB;
        assert!((excess - EVENT_EXCESS_DB[index]).abs() < 1.0e-12, "minute {}", index + 1);
        let flags = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
        assert_eq!(flags, link_gothenburg_10010::FLAG_RAINING);
    }
    assert!(answer["max_residual_conservation_error"].as_f64().unwrap() < 1.0e-14);

    let (status, report) = post(&base, "/verify_tape", &answer).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report, json!({ "holds": true, "values_hold": true, "broken_link": null, "cause": null, "compared": protocol_mcp_gateway::LONG_FORM_COMPUTED }));
    // One rain rate changed in the tape's inputs, and it no longer holds.
    let mut changed = answer.clone();
    changed["inputs"]["segments"][3]["load"]["rain_rate_mm_h"] = json!(12.0);
    let (_, report) = post(&base, "/verify_tape", &changed).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");

    // Two dry hours after the event: the flag clears and the link returns to
    // its dry level to a hundred-millionth of a decibel.
    let mut long = EVENT_MM_H.to_vec();
    long.extend([0.0; 180]);
    let (_, text) = post(&base, "/run_trajectory", &run_body(DRY_DB, 0.0, &long, MINUTE_S)).await;
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["outcome"], "completed");
    assert_eq!(answer["final_state"]["raining"], false);
    assert!(published(&answer["final_state"], "path_loss_db") - DRY_DB < 1.0e-8);
    assert_eq!(published(&answer["final_state"], "t_s"), 192.0 * MINUTE_S);
}

// -------------------------------------------------------------- the bounds

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "signal", "card": "link_gothenburg_10010"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    let phase = |index: usize, key: &str| described["phase_fields"][index][key].as_f64().unwrap();
    let floor_db = link_gothenburg_10010::DRY_PATH_LOSS_FLOOR_DB;
    assert_eq!(phase(0, "minimum"), floor_db);
    assert!(described["phase_fields"][0]["maximum"].is_null());
    assert_eq!((phase(1, "minimum"), phase(1, "maximum")), (0.0, 2.13));
    assert_eq!(described["load_fields"][0]["minimum"].as_f64().unwrap(), 0.0);
    assert!(described["load_fields"][0]["maximum"].is_null());
    let ceiling = link_gothenburg_10010::GATE_DT_S;
    assert_eq!(described["dt_s"]["ceiling_s"].as_f64().unwrap(), ceiling);

    // (dry level, antenna water, rain rate, the words of the refusal, or none)
    let cases: Vec<(f64, f64, f64, Option<String>)> = vec![
        (floor_db, 0.0, 6.0, None),
        (below(floor_db), 0.0, 6.0, Some(outside("dry_path_loss_db", below(floor_db), "dB", false, floor_db, "DryPathLossBelowFreeSpace"))),
        (below(50.0), 0.0, 6.0, None),
        (above(54.0), 0.0, 6.0, None),
        (124.667, 0.0, 6.0, None),
        (52.0, 2.13, 6.0, None),
        (52.0, above(2.13), 6.0, Some(outside("antenna_water_db", above(2.13), "dB", true, 2.13, "AntennaWaterOutsideCard"))),
        (52.0, -0.01, 6.0, Some(outside("antenna_water_db", -0.01, "dB", false, 0.0, "AntennaWaterOutsideCard"))),
        (52.0, 0.0, 84.0, None),
        (52.0, 0.0, above(84.0), None),
        (52.0, 0.0, 150.0, None),
        (52.0, 0.0, -6.0, Some(outside("rain_rate_mm_h", -6.0, "mm/h", false, 0.0, "NegativeRainRate"))),
    ];
    for (dry_db, water_db, rain, refusal) in cases {
        let label = format!("{dry_db} {water_db} {rain}");
        let (status, text) = post(&base, "/verify_physics", &step_body(dry_db, water_db, rain, MINUTE_S)).await;
        let (run_status, run_text) =
            post(&base, "/run_trajectory", &run_body(dry_db, water_db, &[rain, 0.0], MINUTE_S)).await;
        match refusal {
            None => {
                assert_eq!(status, 200, "{label}: {text}");
                assert_eq!(run_status, 200, "{label}: {run_text}");
                assert!(run_text.contains("\"outcome\":\"completed\""), "{label}: {run_text}");
            }
            Some(words) => {
                assert_eq!(status, 422, "{label}: {text}");
                assert_eq!(text, words, "{label}");
                assert_eq!(run_status, 422, "{label}: {run_text}");
                assert_eq!(run_text, words, "{label}");
            }
        }
    }

    // A rain rate below zero in a later segment is refused before any step;
    // one past the fitted 84 mm/h runs to the end.
    let (status, text) = post(&base, "/run_trajectory", &run_body(52.0, 0.0, &[6.0, 12.0, -1.0], MINUTE_S)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(NegativeRainRate)"), "{text}");
    assert!(!text.contains("run_seal"), "{text}");
    let (status, text) = post(&base, "/run_trajectory", &run_body(52.0, 0.0, &[6.0, 12.0, 90.0], MINUTE_S)).await;
    assert_eq!(status, 200, "{text}");
    assert!(text.contains("\"outcome\":\"completed\""), "{text}");

    // The step: the ceiling and the floor land, and the next number outside
    // each, zero and a negative step do not.
    let floor = link().floor_dt_s().unwrap();
    for (dt_s, lands) in [
        (MINUTE_S, true),
        (above(MINUTE_S), true),
        (3600.0, true),
        (ceiling, true),
        (above(ceiling), false),
        (floor, true),
        (below(floor), false),
        (0.0, false),
        (-60.0, false),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body(52.0, 0.0, 6.0, dt_s)).await;
        if lands {
            assert_eq!(status, 200, "{dt_s}: {text}");
        } else {
            assert_eq!(status, 422, "{dt_s}: {text}");
            assert!(text.contains("dt_s") || text.contains("dt"), "{dt_s}: {text}");
            assert!(!text.contains("sha256_seal"), "{dt_s}: {text}");
        }
    }

    // The kernel names the same limits when it is called with no door in front.
    let link = link();
    let mut state = LinkState::try_new(&link, 52.0, 0.0).unwrap();
    let mut frame = blank_frame([0; 16]);
    assert_eq!(
        link_gothenburg_10010::step(&link, &mut state, &mut frame, LinkLoad { rain_rate_mm_h: -1.0 }, MINUTE_S),
        Err(StepReject::NegativeRainRate)
    );
    assert_eq!(
        link_gothenburg_10010::step(&link, &mut state, &mut frame, LinkLoad { rain_rate_mm_h: above(84.0) }, MINUTE_S),
        Ok(())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_input_a_field_the_card_does_not_have_and_a_card_constant_are_refused() {
    let base = spawn_app().await;
    let good = step_body(52.0, 0.0, 6.0, MINUTE_S);
    let (status, _) = post(&base, "/verify_physics", &good).await;
    assert_eq!(status, 200);

    for key in [
        "dry_path_loss_db",
        "antenna_water_db",
        "gauge_minute_elapsed_s",
        "gauge_minute_rain_mm_h",
        "rain_rate_mm_h",
        "dt_s",
    ] {
        let mut step = good.clone();
        step["parameters"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}' (InputMissing)."));
    }
    // A segment with no rain rate: nothing is filled in.
    let mut run = run_body(52.0, 0.0, &[6.0, 6.0], MINUTE_S);
    run["segments"][1]["load"].as_object_mut().unwrap().remove("rain_rate_mm_h");
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("rain_rate_mm_h"), "{text}");
    assert!(!text.contains("run_seal"), "{text}");

    // The card's constants, the sent level, a frequency and a length are not
    // fields of a request.
    for key in [
        "antenna_water_max_db",
        "antenna_wetting_time_s",
        "antenna_drying_time_s",
        "rain_coefficient_db_per_km",
        "rain_exponent",
        "sent_level_dbm",
        "frequency_ghz",
        "path_length_km",
    ] {
        let mut step = good.clone();
        step["parameters"][key] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(key), "{key}: {text}");
        assert!(!text.contains("sha256_seal"), "{key}: {text}");
        let mut run = run_body(52.0, 0.0, &[6.0], MINUTE_S);
        run["segments"][0]["load"][key] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(!text.contains("run_seal"), "{key}: {text}");
    }
    // A rain rate that is not a number.
    let mut step = good.clone();
    step["parameters"]["rain_rate_mm_h"] = json!("heavy");
    let (status, text) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 422, "{text}");
    assert!(!text.contains("sha256_seal"), "{text}");
}

// ------------------------------------------- what the comparison did not cover

/// A step started part way through a gauge minute.
fn continued_step_body(water_db: f64, elapsed_s: f64, minute_mm_h: f64, rain_mm_h: f64, dt_s: f64) -> Value {
    let mut body = step_body(52.3, water_db, rain_mm_h, dt_s);
    body["parameters"]["gauge_minute_elapsed_s"] = json!(elapsed_s);
    body["parameters"]["gauge_minute_rain_mm_h"] = json!(minute_mm_h);
    body
}

/// On engine 0.6.0 a rain of 1e-30 mm/h turned on the full wetting of the
/// antennas with a proof; in 0.10 a rate under one gauge count a minute was
/// refused. Now it is computed, in a step and in a run, with the rain term at
/// that rate and the antenna water building as at any rate above zero, and the
/// run's tape verifies. 0 mm/h is dry.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rain_rate_under_one_gauge_count_is_computed_and_its_tape_verifies() {
    let base = spawn_app().await;
    for rate in [1.0e-30, 1.0e-46, 3.0, below(6.0)] {
        let (status, text) = post(&base, "/verify_physics", &step_body(52.9, 0.0, rate, MINUTE_S)).await;
        assert_eq!(status, 200, "{rate}: {text}");
        let mut run = run_body(52.9, 0.0, &[rate], MINUTE_S);
        run["segments"][0]["steps"] = json!(10);
        let (status, text) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 200, "{rate}: {text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(answer["outcome"], "completed", "{rate}");
        let kept = direct_loop(52.9, 0.0, &[rate; 10], MINUTE_S);
        assert_state_is(&answer["final_state"], &kept[9].0, "light rain");
        assert!(published(&answer["final_state"], "residual_conservation_error") < 1.0e-12);
        let (status, report) = post(&base, "/verify_tape", &answer).await;
        assert_eq!(status, 200, "{report}");
        assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true);
    }
    let (status, text) = post(&base, "/verify_physics", &step_body(52.9, 0.0, 0.0, MINUTE_S)).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(published(&answer["state"], "path_loss_db"), 52.9);
}

/// One 60 s step and sixty 1 s steps at 12 mm/h reach the same path loss and
/// now report the same energies, the integral of the card's own powers over
/// the minute.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_step_and_sixty_steps_report_the_same_energies() {
    let base = spawn_app().await;
    let one = run_body(52.3, 0.0, &[12.0], MINUTE_S);
    let mut sixty = run_body(52.3, 0.0, &[12.0], 1.0);
    sixty["segments"][0]["steps"] = json!(60);
    sixty["record_every"] = json!(60);
    let (_, one) = post(&base, "/run_trajectory", &one).await;
    let (_, sixty) = post(&base, "/run_trajectory", &sixty).await;
    let one: Value = serde_json::from_str(&one).unwrap();
    let sixty: Value = serde_json::from_str(&sixty).unwrap();
    assert_eq!(one["outcome"], "completed");
    assert_eq!(sixty["outcome"], "completed");
    let (a, b) = (&one["final_state"], &sixty["final_state"]);
    assert_eq!(published(a, "path_loss_db").to_bits(), published(b, "path_loss_db").to_bits());
    for name in ["received_energy_j", "rain_energy_j", "antenna_energy_j", "dry_path_energy_j"] {
        let (x, y) = (published(a, name), published(b, name));
        assert!((x / y - 1.0).abs() < 1.0e-13, "{name}: {x} {y}");
    }
    assert!((published(a, "received_energy_j") / 4.67295e-7 - 1.0).abs() < 1.0e-5);
    assert!((published(a, "antenna_energy_j") / 7.0908e-8 - 1.0).abs() < 1.0e-4);
    // Frame slot vel[2] carries the received energy, so it agrees too.
    let slot = |answer: &Value| {
        let bytes = hex::decode(answer["frames"][0]["frame_hex"].as_str().unwrap()).unwrap();
        f32::from_le_bytes(bytes[28..32].try_into().unwrap())
    };
    let (x, y) = (slot(&one), slot(&sixty));
    assert_eq!(x, published(a, "received_energy_j") as f32);
    assert!((x / y - 1.0).abs() < 1.0e-6, "{x} {y}");
}

/// A rain changed inside a gauge minute, where 0.10 refused the run, is
/// computed: the run completes, lands on the kernel's own loop bit for bit,
/// and its tape verifies.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rain_changed_inside_a_gauge_minute_is_computed_and_its_tape_verifies() {
    let base = spawn_app().await;
    let mut run = run_body(52.3, 0.0, &[36.0, 0.0], 10.0);
    run["segments"][1]["steps"] = json!(5);
    run["record_every"] = json!(6);
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["outcome"], "completed", "{text}");
    assert_eq!(answer["steps_landed"], 6);
    let mut kept = direct_loop(52.3, 0.0, &[36.0], 10.0);
    let link = link();
    let mut state = kept[0].0;
    let mut frame = kept[0].1;
    for _ in 0..5 {
        link_gothenburg_10010::step(&link, &mut state, &mut frame, LinkLoad { rain_rate_mm_h: 0.0 }, 10.0).unwrap();
    }
    kept.push((state, frame));
    assert_state_is(&answer["final_state"], &state, "changed inside the minute");
    assert_eq!(published(&answer["final_state"], "gauge_minute_elapsed_s"), 0.0);
    let (status, report) = post(&base, "/verify_tape", &answer).await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true);
}

/// Restart family: the seconds spent in the gauge minute and the rate of the
/// last step are reported, and a start may state any value of them on the
/// card without from: a single step of 10 s handed back as a new start may
/// change its rain at once. The bounds of the clock are still refused by
/// name, and so is a rate below zero.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_restart_inside_a_gauge_minute_may_change_its_rain() {
    let base = spawn_app().await;
    let (status, first) = post(&base, "/verify_physics", &step_body(52.3, 0.0, 36.0, 10.0)).await;
    assert_eq!(status, 200, "{first}");
    let first: Value = serde_json::from_str(&first).unwrap();
    let state = &first["state"];
    let elapsed = published(state, "gauge_minute_elapsed_s");
    let minute = published(state, "gauge_minute_rain_mm_h");
    let water = published(state, "antenna_water_db");
    assert_eq!((elapsed, minute), (10.0, 36.0));
    let (status, text) = post(&base, "/verify_physics", &continued_step_body(water, elapsed, minute, 0.0, 10.0)).await;
    assert_eq!(status, 200, "{text}");
    let mut run = run_body(52.3, water, &[0.0], 10.0);
    run["initial"]["gauge_minute_elapsed_s"] = json!(elapsed);
    run["initial"]["gauge_minute_rain_mm_h"] = json!(minute);
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{text}");
    assert!(!text.contains("ContinuationWithoutUpstream"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &continued_step_body(0.0, below(60.0), 0.0, 0.0, 1.0e-3)).await;
    assert_eq!(status, 200, "{text}");
    for elapsed in [60.0, -0.5] {
        let (status, text) = post(&base, "/verify_physics", &continued_step_body(0.0, elapsed, 0.0, 0.0, 1.0)).await;
        assert_eq!(status, 422, "{elapsed}: {text}");
        assert!(text.contains("(GaugeMinuteOutsideCard)"), "{elapsed}: {text}");
    }
    let (status, text) = post(&base, "/verify_physics", &continued_step_body(0.0, 0.0, 90.0, 0.0, 1.0)).await;
    assert_eq!(status, 200, "{text}");
    let (status, text) = post(&base, "/verify_physics", &continued_step_body(0.0, 0.0, -1.0, 0.0, 1.0)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(NegativeRainRate)"), "{text}");
}

// ------------------------------------------------------- compared with a series

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_the_path_loss_of_a_run_and_carries_its_run_seal() {
    let base = spawn_app().await;
    let run = run_body(DRY_DB, 0.0, &EVENT_MM_H, MINUTE_S);
    let (_, alone) = post(&base, "/run_trajectory", &run).await;
    let alone: Value = serde_json::from_str(&alone).unwrap();
    let kept = direct_loop(DRY_DB, 0.0, &EVENT_MM_H, MINUTE_S);

    let exact: Vec<Value> = kept
        .iter()
        .enumerate()
        .map(|(index, (state, _))| json!({ "step": index + 1, "value": state.path_loss_db }))
        .collect();
    let body = json!({ "run": run, "quantity": "path_loss_db", "measurements": exact });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["compared"], 12);
    assert_eq!(answer["rms_error"], 0.0);
    assert_eq!(answer["max_abs_error"], 0.0);

    // A link that read half a decibel more than the card at every minute.
    let offset: Vec<Value> = kept
        .iter()
        .enumerate()
        .map(|(index, (state, _))| json!({ "step": index + 1, "value": state.path_loss_db + 0.5 }))
        .collect();
    let body = json!({ "run": run, "quantity": "path_loss_db", "measurements": offset });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert!((answer["rms_error"].as_f64().unwrap() - 0.5).abs() < 1.0e-12, "{text}");
    assert!((answer["mean_signed_error"].as_f64().unwrap().abs() - 0.5).abs() < 1.0e-12, "{text}");
    assert!(answer.get("pass").is_none() && answer.get("verdict").is_none());

    // The flag is not a quantity, and neither is a name the card does not publish.
    for quantity in ["raining", "rain_rate", "snr_db"] {
        let body = json!({ "run": run, "quantity": quantity, "measurements": [{ "step": 1, "value": 0.0 }] });
        let (status, text) = post(&base, "/validate_series", &body).await;
        assert_eq!(status, 422, "{quantity}: {text}");
        assert!(!text.contains("run_seal"), "{text}");
    }
}

// ------------------------------------------------------ the three connections

const KEY_ALL: &str = "key-for-the-client-that-sees-everything";
const KEY_LINK: &str = "key-for-the-link-client";
const KEY_THERMAL: &str = "key-for-the-thermal-client";

fn client_entry(label: &str, key: &str, cards: Value) -> Value {
    json!({ "client": label, "key_sha256": sha256_hex(key.as_bytes()), "cards": cards, "solvers": [], "max_in_flight": 8, "max_permits": 1 })
}

/// The local connection: `mcp_stdio` as a child process.
struct Local {
    _child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Local {
    async fn start() -> Local {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut local = Local { _child: child, stdin, out };
        let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
        local.raw(&init.to_string()).await;
        local
            .stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .unwrap();
        local
    }

    async fn raw(&mut self, line: &str) -> String {
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.write_all(b"\n").await.unwrap();
        self.stdin.flush().await.unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).await.unwrap();
        answer.trim_end_matches('\n').to_string()
    }

    /// The whole answer line of a tool call, as the hosted connection frames it.
    async fn call(&mut self, name: &str, arguments: &Value) -> String {
        let line = answer_text::tool_call(7, name, arguments);
        self.raw(&line.to_string()).await
    }
}

/// The hosted connection: `hosted_door` as a child process, with a keys file.
struct Hosted {
    _child: Child,
    base: String,
    _dir: PathBuf,
}

impl Hosted {
    async fn start(keys: &Value) -> Hosted {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("link_hosted_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("keys.json");
        std::fs::write(&file, serde_json::to_string_pretty(keys).unwrap()).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .arg("--port")
            .arg("0")
            .arg("--keys")
            .arg(&file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        out.read_line(&mut line).await.unwrap();
        let address = line
            .split("running on ")
            .nth(1)
            .unwrap_or_else(|| panic!("no address in {line:?}"))
            .split(' ')
            .next()
            .unwrap()
            .to_string();
        tokio::spawn(async move {
            let mut rest = String::new();
            while out.read_line(&mut rest).await.unwrap_or(0) > 0 {
                rest.clear();
            }
        });
        Hosted { _child: child, base: format!("http://{address}"), _dir: dir }
    }

    async fn call(&self, key: &str, tool: &str, arguments: &Value) -> String {
        let body = answer_text::tool_call(7, tool, arguments);
        let response = reqwest::Client::new()
            .post(format!("{}/mcp", self.base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Authorization", format!("Bearer {key}"))
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        response.text().await.unwrap()
    }
}

/// The text of a tool's result, and whether the connection marked it an error.
fn result_text(answer: &str) -> (String, bool) {
    let value: Value = serde_json::from_str(answer).unwrap();
    let text = value["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text in {answer}"))
        .to_string();
    answer_text::remember(&text);
    (text, value["result"]["isError"] == true)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_three_connections_list_the_link_and_give_the_same_bytes_for_its_description_and_its_tools() {
    let web = spawn_app().await;
    let mut local = Local::start().await;

    // Thirty cards, the link the twenty-second, in the signal domain, and the
    // line an agent reads in the list says what is fitted to the link itself.
    let (listed, error) = result_text(&local.call("list_cards", &json!({})).await);
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 40);
    assert_eq!(cards[26]["domain"], "signal");
    assert_eq!(cards[26]["card"], "link_gothenburg_10010");
    let line = cards[26]["specimen"].as_str().unwrap();
    assert!(line.contains("Recommendation ITU-R P.838-3 with nothing fitted"), "{line}");
    assert!(line.contains("Fitted on this link's own readings in the 18 rain events of June and July 2015"), "{line}");
    let on_web: Value = Client::new().get(format!("{web}/cards")).send().await.unwrap().json().await.unwrap();
    assert_eq!(on_web, listed);

    let all_pairs: Vec<Value> = cards.iter().map(|card| json!([card["domain"], card["card"]])).collect();
    let keys = json!([
        client_entry("all", KEY_ALL, Value::Array(all_pairs)),
        client_entry("link", KEY_LINK, json!([["signal", "link_gothenburg_10010"]])),
        client_entry("thermal", KEY_THERMAL, json!([["energy", "thermal"]])),
    ]);
    let hosted = Hosted::start(&keys).await;

    let pair = json!({"domain": "signal", "card": "link_gothenburg_10010"});
    let on_local = local.call("describe_card", &pair).await;
    let (described, error) = result_text(&on_local);
    assert!(!error);
    let (status, on_web) = post(&web, "/describe_card", &pair).await;
    assert_eq!(status, 200);
    assert_eq!(described, on_web);
    for key in [KEY_ALL, KEY_LINK] {
        assert_eq!(hosted.call(key, "describe_card", &pair).await, on_local, "{key}");
    }

    let description: Value = serde_json::from_str(&described).unwrap();
    let phase: Vec<&str> = description["phase_fields"].as_array().unwrap().iter().map(|field| field["name"].as_str().unwrap()).collect();
    assert_eq!(
        phase,
        ["dry_path_loss_db", "antenna_water_db", "gauge_minute_elapsed_s", "gauge_minute_rain_mm_h"]
    );
    let load: Vec<&str> = description["load_fields"].as_array().unwrap().iter().map(|field| field["name"].as_str().unwrap()).collect();
    assert_eq!(load, ["rain_rate_mm_h"]);
    let refusals: Vec<&str> = description["rejects"].as_array().unwrap().iter().map(|reject| reject["name"].as_str().unwrap()).collect();
    assert_eq!(
        refusals,
        [
            "NonFiniteInput",
            "NonFiniteState",
            "NonPositiveDt",
            "InvalidLink",
            "NegativeRainRate",
            "GaugeMinuteOutsideCard",
            "DryPathLossBelowFreeSpace",
            "AntennaWaterOutsideCard",
            "SpontaneousGeneration",
            "FirstLawBreach",
            "AntennaAccountBreach",
            // The gateway's own two for this card follow the kernel's eleven.
            "DtBelowFloor",
            "DtAboveCeiling"
        ]
    );
    for reject in description["rejects"].as_array().unwrap() {
        assert!(reject["meaning"].as_str().is_some_and(|meaning| !meaning.is_empty()), "{reject}");
    }
    assert_eq!(description["advisory_flags"][0]["name"], "FLAG_RAINING");
    let record = description["validation"].as_object().unwrap();
    let parts: Vec<&str> = record.keys().map(String::as_str).collect();
    assert_eq!(parts, ["object", "data", "pass_line", "result", "limits"]);
    assert!(record["data"].as_str().unwrap().contains("10.5281/zenodo.7107689"));
    assert!(record["pass_line"].as_str().unwrap().contains("f60bfcc70553f95fef5d35963b6550120de3c67e712c057e40c5d37c51784625"));

    // The four tools that compute: the same bytes on the three connections.
    let step = description["example_compute_step"].clone();
    let run = run_body(DRY_DB, 0.0, &EVENT_MM_H, MINUTE_S);
    let dataset = json!({
        "domain": "signal", "card": "link_gothenburg_10010",
        "plan": { "kind": "list", "runs": [
            { "initial": { "dry_path_loss_db": 52.0, "antenna_water_db": 0.0, "gauge_minute_elapsed_s": 0.0, "gauge_minute_rain_mm_h": 0.0 }, "dt_s": MINUTE_S,
              "segments": [{ "steps": 5, "load": { "rain_rate_mm_h": 12.0 } }, { "steps": 5, "load": { "rain_rate_mm_h": 0.0 } }], "record_every": 5 },
            { "initial": { "dry_path_loss_db": 52.0, "antenna_water_db": 0.0, "gauge_minute_elapsed_s": 0.0, "gauge_minute_rain_mm_h": 0.0 }, "dt_s": MINUTE_S,
              "segments": [{ "steps": 5, "load": { "rain_rate_mm_h": -90.0 } }], "record_every": 5 }
        ] },
        "store": "frames", "return": "inline", "answer": "full"
    });
    let series = json!({
        "run": run, "quantity": "path_loss_db",
        "measurements": [{ "step": 1, "value": 55.0 }, { "step": 6, "value": 56.0 }, { "step": 12, "value": 57.5 }]
    });
    let mut tape_text = String::new();
    for (tool, route, arguments) in [
        ("verify_step", "/verify_physics", &step),
        ("run_trajectory", "/run_trajectory", &run),
        ("run_dataset", "/run_dataset", &dataset),
        ("validate_series", "/validate_series", &series),
    ] {
        let on_local = local.call(tool, arguments).await;
        let (text, error) = result_text(&on_local);
        assert!(!error, "{tool}: {text}");
        let (status, on_web) = post(&web, route, arguments).await;
        assert_eq!(status, 200, "{tool}: {on_web}");
        assert_eq!(text, on_web, "{tool}");
        for key in [KEY_ALL, KEY_LINK] {
            assert_eq!(hosted.call(key, tool, arguments).await, on_local, "{tool} {key}");
        }
        if tool == "run_trajectory" {
            tape_text = text;
        }
    }
    // The dataset's second row rains upward: not admitted, by name.
    let (text, _) = result_text(&local.call("run_dataset", &dataset).await);
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["rows"][0]["outcome"], "completed");
    assert_eq!(answer["rows"][1]["outcome"], "unadmitted");
    assert_eq!(
        answer["rows"][1]["reject_detail"],
        outside("rain_rate_mm_h", -90.0, "mm/h", false, 0.0, "NegativeRainRate")
    );

    let tape: Value = serde_json::from_str(&tape_text).unwrap();
    let parts = json!({ "head": answer["head"], "rows": answer["rows"], "frames_hex": answer["frames_hex"] });
    for (tool, route, arguments) in [("verify_tape", "/verify_tape", &tape), ("verify_dataset", "/verify_dataset", &parts)] {
        let on_local = local.call(tool, arguments).await;
        let (text, error) = result_text(&on_local);
        assert!(!error, "{tool}: {text}");
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], true, "{tool}: {report}");
        let (status, on_web) = post(&web, route, arguments).await;
        assert_eq!(status, 200, "{tool}: {on_web}");
        assert_eq!(text, on_web, "{tool}");
        for key in [KEY_ALL, KEY_LINK] {
            assert_eq!(hosted.call(key, tool, arguments).await, on_local, "{tool} {key}");
        }
    }

    // A refusal is a tool result marked as an error, with the words of the
    // web connection, on the local and the hosted connection alike.
    let bad = step_body(52.0, 0.0, -90.0, MINUTE_S);
    let on_local = local.call("verify_step", &bad).await;
    let (text, error) = result_text(&on_local);
    assert!(error);
    let (status, on_web) = post(&web, "/verify_physics", &bad).await;
    assert_eq!(status, 422);
    assert_eq!(text, on_web);
    assert_eq!(text, outside("rain_rate_mm_h", -90.0, "mm/h", false, 0.0, "NegativeRainRate"));
    assert_eq!(hosted.call(KEY_LINK, "verify_step", &bad).await, on_local);

    // The key that lists the link alone sees one card.
    let (seen, _) = result_text(&hosted.call(KEY_LINK, "list_cards", &json!({})).await);
    let seen: Value = serde_json::from_str(&seen).unwrap();
    assert_eq!(seen["cards"].as_array().unwrap().len(), 1);
    assert_eq!(seen["cards"][0]["card"], "link_gothenburg_10010");

    // A key that does not list the link gets the answers of a card that does
    // not exist, byte for byte, and no word of the link.
    let invented = "qqqqqqqqqqqqqqqqqqqqq";
    assert_eq!(invented.len(), "link_gothenburg_10010".len());
    let rename = |value: &Value| {
        let mut value = value.clone();
        value["card"] = json!(invented);
        value
    };
    let mut session = Vec::new();
    for (tool, hidden, fake) in [
        ("describe_card", pair.clone(), rename(&pair)),
        ("verify_step", step.clone(), rename(&step)),
        ("run_trajectory", run.clone(), rename(&run)),
        ("run_dataset", dataset.clone(), rename(&dataset)),
    ] {
        let on_hidden = hosted.call(KEY_THERMAL, tool, &hidden).await;
        let on_fake = hosted.call(KEY_THERMAL, tool, &fake).await;
        assert_eq!(on_hidden, on_fake, "{tool}");
        session.push(on_hidden);
    }
    assert_eq!(result_text(&hosted.call(KEY_THERMAL, "describe_card", &pair).await).0, "Card not recognized (UnknownCard).");
    session.push(hosted.call(KEY_THERMAL, "list_cards", &json!({})).await);
    let text = session.join("\n").to_lowercase();
    for word in ["gothenburg", "antenna", "rain_rate", "openmrg", "path_loss", "p.838"] {
        assert!(!text.contains(word), "the word '{word}' of the hidden card was sent");
    }
}
