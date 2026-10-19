//! The Ryugu crater card on the web connection.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes, and its receipt verifies. The whole growth, 1,600 steps from 1.75 m,
//! ends on the kernel loop's state to the bit: the crater stops on step 1,597
//! at 8.571304329477922 m, the frames from there carry the flag, and the tape
//! verifies. A step handed back as a new start goes on with the wall at the speed the card sets from its radius.
//! Each refusal a request can reach is named and carries no proof: a radius
//! under the ledger's resolution or above the radius at which the crater stops, a step of
//! zero or less, a step outside the card's floor and ceiling, a missing
//! input, a wall speed or a load the card does not take.
//!
//! No measurement of the crater is read here.

mod answer_text;

use std::time::Duration;

use genesis_kernel::crater_ryugu_sci::{
    self, CraterLoad, CraterRyuguSci, CraterState, FINAL_RADIUS_M, RADIUS_FLOOR_M,
};
use genesis_kernel::frame::LastStateFrame64;
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

/// The start of the research record's reference run and of the worked
/// examples: 1.75 m, the card's floor until engine 0.9.
const START_M: f64 = 1.75;

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

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

fn crater() -> CraterRyuguSci {
    CraterRyuguSci::sci_impact()
}

fn ceiling() -> f64 {
    crater().gate_dt_s().unwrap()
}

fn floor() -> f64 {
    crater().floor_dt_s().unwrap()
}

const NO_LOAD: CraterLoad = CraterLoad {};

fn step_body(radius_m: f64, dt_s: f64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "crater_ryugu_sci",
        "parameters": { "radius_m": radius_m, "dt_s": dt_s }
    })
}

fn run_body(radius_m: f64, dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "crater_ryugu_sci",
        "initial": { "radius_m": radius_m },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": {} }],
        "record_every": record_every
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

/// The kernel loop: `steps` steps of `dt_s` from this radius. The last state,
/// the largest residual, the step on which the crater stopped, and the state
/// at every `keep_every`-th step.
fn direct_loop(radius_m: f64, dt_s: f64, steps: u64, keep_every: u64) -> (CraterState, f64, Option<u64>, Vec<CraterState>) {
    let crater = crater();
    let mut state = CraterState::try_radius(&crater, radius_m).unwrap();
    let mut frame = blank_frame([0; 16]);
    let (mut worst, mut stopped_on, mut kept) = (0.0_f64, None, Vec::new());
    for count in 1..=steps {
        crater_ryugu_sci::step(&crater, &mut state, &mut frame, NO_LOAD, dt_s).unwrap();
        worst = worst.max(state.residual_conservation_error);
        if state.stopped && stopped_on.is_none() {
            stopped_on = Some(count);
        }
        if count % keep_every == 0 {
            kept.push(state);
        }
    }
    (state, worst, stopped_on, kept)
}

fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

/// Every published number is the kernel's, to the bit, as the decimal and as
/// the hex beside it; the flag is the kernel's; nothing else is published.
fn assert_state_is(published_state: &Value, state: &CraterState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in [
        ("t_s", state.t_s),
        ("radius_m", state.radius_m),
        ("wall_speed_m_s", state.wall_speed_m_s),
        ("radius_over_final", state.radius_over_final),
        ("radius_to_go_m", state.radius_to_go_m),
        ("kinetic_j_per_kg", state.kinetic_j_per_kg),
        ("given_up_j_per_kg", state.given_up_j_per_kg),
        ("energy_defect_j_per_kg", state.energy_defect_j_per_kg),
        ("residual_conservation_error", state.residual_conservation_error),
    ] {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    assert_eq!(object["stopped"], state.stopped, "{label}");
    keys.insert("stopped".to_string());
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha256\":\"";
    let at = text.rfind(MARKER).expect("a receipt");
    (format!("{}}}", &text[..at]), text[at + MARKER.len()..text.len() - 2].to_string())
}

/// The refusal is named and the answer carries no proof.
fn names(text: &str, name: &str) -> bool {
    text.contains(&format!("({name})")) && !text.contains("sha256_seal") && !text.contains("run_seal")
}

/// Time, radius, wall speed and flag word of one recorded frame.
fn slots(frame_hex: &str) -> (f64, f32, f32, f32, u64) {
    let bytes = hex::decode(frame_hex).unwrap();
    assert_eq!(bytes.len(), 64);
    let f32_at = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    (
        f64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        f32_at(8),
        f32_at(12),
        f32_at(32),
        u64::from_le_bytes(bytes[40..48].try_into().unwrap()),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_crater_matches_the_compiled_step_with_the_same_proof_bytes_and_its_receipt_verifies() {
    let base = spawn_app().await;
    let cases = [
        step_body(START_M, ceiling()),
        step_body(START_M, floor()),
        step_body(RADIUS_FLOOR_M, 1.0e-6),
        step_body(4.0, 0.1),
        step_body(8.5, ceiling()),
        // The last step of the growth, and a crater that has already stopped.
        step_body(8.5713043, ceiling()),
        step_body(FINAL_RADIUS_M, ceiling()),
    ];
    let mut seals = Vec::new();
    for request in cases {
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);
        let number = |key: &str| request["parameters"][key].as_f64().unwrap();
        let crater = crater();
        let mut state = CraterState::try_radius(&crater, number("radius_m")).unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        crater_ryugu_sci::step(&crater, &mut state, &mut frame, NO_LOAD, number("dt_s")).unwrap();
        assert_eq!(frame.flags & !crater_ryugu_sci::FLAG_STOPPED, 0);
        assert_eq!(frame.flags != 0, state.stopped);
        assert_state_is(&response.state, &state, "step");
        assert_eq!(
            response.residual_conservation_error.to_bits(),
            state.residual_conservation_error.to_bits()
        );
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha256);
        assert_eq!(sha256_hex(document.as_bytes()), receipt);
        let (status, report) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], true, "{report}");
        let changed = answer.replacen("\"t_s\":", "\"t_s\":1", 1);
        let (_, report) = post(&base, "/verify_receipt", &json!({ "response_text": changed })).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["matches"], false, "{report}");
        let (_, again) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(again, answer);
        seals.push((response.sha256_seal, state.stopped));
    }
    // The last two steps end stopped; the first five do not.
    assert_eq!(seals.iter().map(|(_, stopped)| *stopped).collect::<Vec<_>>(), [false, false, false, false, false, true, true]);
    let mut distinct: Vec<&String> = seals.iter().map(|(seal, _)| seal).collect();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 7, "seven different steps, seven different proofs");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_handed_back_as_a_new_start_continues_the_same_growth() {
    let base = spawn_app().await;
    let mut radius_m = 3.0;
    for _ in 0..5 {
        let (status, answer) = post(&base, "/verify_physics", &step_body(radius_m, ceiling())).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        radius_m = published(&response.state, "radius_m");
    }
    let (state, _, _, _) = direct_loop(3.0, ceiling(), 5, 1);
    // Each single step starts on the impact's own curve at its radius, so the
    // chain and the run differ only by the ledger's residual: under a part in 10^9.
    assert!(((radius_m - state.radius_m) / state.radius_m).abs() < 1.0e-9, "{radius_m} {}", state.radius_m);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_whole_growth_stops_on_step_1597_at_the_final_radius_and_its_tape_verifies() {
    let base = spawn_app().await;
    let request = run_body(START_M, 0.25, 1_600, 100);
    assert_eq!(
        serde_json::to_string(&request).unwrap(),
        r#"{"domain":"aerospace","card":"crater_ryugu_sci","initial":{"radius_m":1.75},"dt_s":0.25,"segments":[{"steps":1600,"load":{}}],"record_every":100}"#
    );
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{}", &answer[..answer.len().min(400)]);
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 1_600);
    let frames = tape["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 16);

    let (state, worst, stopped_on, kept) = direct_loop(START_M, 0.25, 1_600, 100);
    assert_eq!(stopped_on, Some(1_597));
    let last = &tape["final_state"];
    assert_state_is(last, &state, "1,600 steps");
    assert_eq!(published(last, "radius_m").to_bits(), FINAL_RADIUS_M.to_bits());
    assert_eq!(published(last, "radius_m"), 8.571304329477922);
    // The protocol's line: the stop agrees with the closed form of the fitted
    // thirteen-shot law, 8.571304329477925 m in the research record, to 1 part
    // in 10^9. The six-figure constants of 0.6.0 stopped 3.45e-8 away.
    let closed_form = 8.571304329477925_f64;
    let agreement = (published(last, "radius_m") - closed_form).abs() / closed_form;
    assert!(agreement <= 1.0e-9, "{agreement:e}");
    assert!((8.571304624905444_f64 - closed_form).abs() / closed_form > 1.0e-9);
    assert_eq!(published(last, "wall_speed_m_s"), 0.0);
    assert_eq!(published(last, "t_s"), 400.0);
    assert_eq!(last["stopped"], true);
    let published_worst = tape["max_residual_conservation_error"].as_f64().unwrap();
    assert_eq!(published_worst.to_bits(), worst.to_bits());
    assert!(((worst - 5.8204711758746565e-8) / 5.8204711758746565e-8).abs() < 1.0e-3, "{worst:e}");

    // Each recorded frame is the kernel loop's radius at that step. Only the
    // last frame, at step 1,600, carries the flag.
    for (index, frame) in frames.iter().enumerate() {
        let (t_s, radius, speed, to_go, flags) = slots(frame["frame_hex"].as_str().unwrap());
        let kernel = &kept[index];
        assert_eq!(t_s, 25.0 * (index as f64 + 1.0));
        assert_eq!(radius.to_bits(), (kernel.radius_m as f32).to_bits(), "frame {index}");
        assert_eq!(speed.to_bits(), (kernel.wall_speed_m_s as f32).to_bits(), "frame {index}");
        assert_eq!(to_go.to_bits(), (kernel.radius_to_go_m as f32).to_bits(), "frame {index}");
        assert_eq!(flags, u64::from(index == 15), "frame {index}");
    }
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");

    // Recorded at every step from 1,590: the flag first appears on step 1,597.
    let (state_1590, _, _, _) = direct_loop(START_M, 0.25, 1_590, 1_590);
    let (status, answer) = post(&base, "/run_trajectory", &run_body(state_1590.radius_m, 0.25, 10, 1)).await;
    assert_eq!(status, 200, "{answer}");
    let tail: Value = serde_json::from_str(&answer).unwrap();
    let flags: Vec<u64> = tail["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| slots(frame["frame_hex"].as_str().unwrap()).4)
        .collect();
    assert_eq!(flags, [0, 0, 0, 0, 0, 0, 1, 1, 1, 1]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    for radius_m in [RADIUS_FLOOR_M, above(RADIUS_FLOOR_M), 1.0, below(FINAL_RADIUS_M), FINAL_RADIUS_M] {
        // Near the floor the wall moves at 214 m/s, and a step of a microsecond lands.
        let dt_s = if radius_m < 1.5 { 1.0e-6 } else { ceiling() };
        let (status, text) = post(&base, "/verify_physics", &step_body(radius_m, dt_s)).await;
        assert_eq!(status, 200, "{radius_m}: {text}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(radius_m, dt_s, 3, 1)).await;
        assert_eq!(status, 200, "{radius_m}: {text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["inputs"]["initial"]["radius_m"].as_f64().unwrap().to_bits(), radius_m.to_bits());
    }
    for (radius_m, name) in [
        (below(RADIUS_FLOOR_M), "RadiusBelowLedgerResolution"),
        (0.1, "RadiusBelowLedgerResolution"),
        (0.0, "RadiusBelowLedgerResolution"),
        (-4.0, "RadiusBelowLedgerResolution"),
        (above(FINAL_RADIUS_M), "RadiusAboveFinal"),
        (8.8, "RadiusAboveFinal"),
        (7250.0, "RadiusAboveFinal"),
    ] {
        assert!(CraterState::try_radius(&crater(), radius_m).is_none());
        let (status, text) = post(&base, "/verify_physics", &step_body(radius_m, ceiling())).await;
        assert_eq!(status, 422, "{radius_m}: {text}");
        assert!(names(&text, name), "{radius_m}: {text}");
        assert!(text.ends_with("That state is not sealed."), "{text}");
        let (status, run_text) = post(&base, "/run_trajectory", &run_body(radius_m, ceiling(), 3, 1)).await;
        assert_eq!(status, 422, "{radius_m}: {run_text}");
        assert_eq!(run_text, text);
    }
    for dt_s in [floor(), ceiling(), 1.0e-3] {
        let (status, text) = post(&base, "/verify_physics", &step_body(4.0, dt_s)).await;
        assert_eq!(status, 200, "{dt_s}: {text}");
    }
    for (dt_s, words) in [
        (below(floor()), "the step floor of the crater_ryugu_sci card"),
        (1.0e-12, "the step floor of the crater_ryugu_sci card"),
        (above(ceiling()), "the step ceiling of the crater_ryugu_sci card"),
        (1.0, "the step ceiling of the crater_ryugu_sci card"),
        (0.0, "(NonPositiveDt) on the crater_ryugu_sci card"),
        (-0.25, "(NonPositiveDt) on the crater_ryugu_sci card"),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body(4.0, dt_s)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains(words) && !text.contains("sha256_seal"), "{dt_s}: {text}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(4.0, dt_s, 3, 1)).await;
        assert_eq!(status, 422, "{dt_s}: {text}");
        assert!(text.contains(words) && !text.contains("run_seal"), "{dt_s}: {text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_input_a_wall_speed_a_load_and_a_card_constant_are_refused() {
    let base = spawn_app().await;
    let good = step_body(4.0, ceiling());
    let (status, _) = post(&base, "/verify_physics", &good).await;
    assert_eq!(status, 200);
    for key in ["radius_m", "dt_s"] {
        let mut step = good.clone();
        step["parameters"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, format!("Missing required parameter: '{key}' (InputMissing)."));
    }
    // The wall's speed, the impactor, Ryugu and the card's constants are not
    // fields of a request: in a step, at the start of a run and in a load.
    for key in [
        "wall_speed_m_s",
        "velocity_m_s",
        "projectile_mass_kg",
        "projectile_speed_m_s",
        "gravity_m_s2",
        "final_radius_m",
        "growth_power",
        "gravity_work_m_s2",
        "scaling_prefactor",
        "cohesion_pa",
        "gate_dt_s",
        "residual_conservation_limit",
        "stopped",
    ] {
        let sentence = format!("Physical Violation: '{key}' is not a field of this request.");
        let mut step = good.clone();
        step["parameters"][key] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, sentence);
        let mut in_phase = run_body(4.0, ceiling(), 3, 1);
        in_phase["initial"][key] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_phase).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, sentence);
        let mut in_load = run_body(4.0, ceiling(), 3, 1);
        in_load["segments"][0]["load"][key] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert_eq!(text, sentence);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_card_is_listed_and_described_with_its_record() {
    let base = spawn_app().await;
    let list: Value = Client::new().get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let cards = list["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 40);
    assert_eq!(cards[20]["domain"], "aerospace");
    assert_eq!(cards[20]["card"], "crater_ryugu_sci");
    assert!(cards[20]["specimen"].as_str().unwrap().contains("nothing fitted to Ryugu"));
    let (status, text) = post(&base, "/describe_card", &json!({ "domain": "aerospace", "card": "crater_ryugu_sci" })).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["phase_fields"][0]["name"], "radius_m");
    // Re-pinned for 0.10.1: after the kernel's 9 names the list holds the gateway's own two, DtBelowFloor and DtAboveCeiling.
    // The kernel's nine, DtBelowFloor and DtAboveCeiling, and, since the fix
    // pass of 0.14.0, ContinuationNotOnCard: a continuation before the stop.
    assert_eq!(body["rejects"].as_array().map(Vec::len), Some(12));
    let record = &body["validation"];
    assert!(record["limits"].as_str().unwrap().contains("Nothing is fitted to Ryugu"));
    // Sentences held in the one line of the list, the result and the limits.
    assert!(cards[20]["specimen"].as_str().unwrap().contains(
        "Gap: on Ryugu the card stops 1.32 m, 18.2 percent, above the measured apparent radius, outside the allowance of 0.97 m;"
    ));
    let result = record["result"].as_str().unwrap();
    for words in [
        "On Ryugu: the card stops at 8.571 m against the measured apparent radius of 7.25 m, 1.32 m or 18.2 percent above (allowance 0.97 m",
        "Those intervals are +/- 31 to 38 percent wide",
        "The mean, 7.87 percent, follows from Schmidt's published line",
        "with any one of four of the nine 1 g shots left out it is 10.4 to 11.8 percent (target 10)",
        "The four errors are all of one sign, a bias of the nine-shot line",
        "7.25 m is the apparent radius, which is what the law predicts: the apparent diameter is 14.5 +/- 0.8 m and the rim-to-rim diameter 17.6 +/- 0.7 m",
        "The same team's submitted manuscript gives 6.5 +/- 0.4 m by another reduction",
        "6.9 to 7.7 m by the dry-sand line of Housen and Holsapple, \"agrees well with the apparent radius\"",
        "8.571304329477925 m (target 1 part in 10^9)",
        "stops at 8.571304329477922 m, 4 parts in 10^16 from the closed form",
        "stopped at 8.571304624905444 m, 3.45 parts in 10^8 from the closed form",
        "That agreement is by construction: on the stopping step the card sets the radius to a typed constant",
        "differ from that run in the eighth figure and beyond",
        "the team's submitted manuscript says growth would stop by about 300 s",
        "binary 425ef2b93d69be6f05da11eaf6ad4c4debecc37d548672950465d68ef2bf3b04; two later builds, 05efbd06… and 043d5765…, return its answer byte for byte",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    assert!(!result.contains("to the last bit"), "{result}");
    let limits = record["limits"].as_str().unwrap();
    for words in [
        "the measured diameter and a prediction of 8.6 m worked out by hand were read before the record's hash was taken, and all of Schmidt's first table, the four centrifuge craters outside the fit included, was read while it was transcribed",
        "The impactor was a hollow copper shell 13 cm across, of 2 kg; the card models it as a solid copper sphere of the same mass, of radius 3.76 cm, at the full 2 km/s",
        "the same law gives 7.83 m, 8.0 percent above the measurement and inside the allowance",
        "(the shell, 2 km/s, g = 1.2e-4 m/s2) it gives 7.50 m, 3.5 percent above",
        "Across twenty reasonable readings of the inputs the prediction runs from 7.15 to 8.80 m, wider than the gap",
        "each move the prediction more than the rotation term does, and neither is in the allowance",
        "The 18.2 percent belongs to the scaling of dense Ottawa Flintshot sand with these inputs",
        "Its time constant, 1.318295773573555, comes from seven points read by eye at 1 g",
        "the open sources say growth for about 200 s and a stop by about 300 s. No time of Ryugu's crater is in the comparison.",
        "a second comparison program, on the same machine, gives the same numbers",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    // Both worked examples run as written.
    let (status, answer) = post(&base, "/verify_physics", &body["example_compute_step"]).await;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = post(&base, "/run_trajectory", &body["example_run_trajectory"]).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["final_state"]["stopped"], true);
}

/// Where the card stood at 1.75 m, a run from the floor, 0.1622 m, at a
/// step of 2 microseconds lands on the kernel's own loop bit for bit with the
/// ledger closed, and its tape verifies; at the ceiling its first step is
/// refused by the ledger, sealed with nothing landed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_from_the_floor_lands_on_the_kernel_loop_and_its_tape_verifies() {
    let base = spawn_app().await;
    let (status, answer) = post(&base, "/run_trajectory", &run_body(RADIUS_FLOOR_M, 2.0e-6, 20_000, 1_000)).await;
    assert_eq!(status, 200, "{}", &answer[..answer.len().min(400)]);
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    let (state, worst, _, _) = direct_loop(RADIUS_FLOOR_M, 2.0e-6, 20_000, 1_000);
    assert_state_is(&tape["final_state"], &state, "from the floor");
    assert!(worst < 1.0e-6, "{worst:e}");
    assert!(state.radius_m > 0.5, "{}", state.radius_m);
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true);
    let (status, answer) = post(&base, "/run_trajectory", &run_body(RADIUS_FLOOR_M, ceiling(), 3, 1)).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "refused", "{answer}");
    assert_eq!(tape["steps_landed"], 0);
}
