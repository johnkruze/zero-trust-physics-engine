//! The InSight entry capsule card on the web connection and the local one.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes. The whole entry, with no lift and with the lift measured on the
//! flight, ends at the flown parachute deployment, PastFlightDeployment, the
//! end the card declares, and its last landed state is the kernel loop's, to
//! the bit. The tape of each run verifies and its receipt matches. The bound
//! on lift is refused by name at the last representable number outside it,
//! in a step, in the first segment of a run and in a later one. A state off
//! the card's tables, under the landing site or past the deployment is refused
//! by name before any step; a state off the compared corridor or under the
//! old compared floor is computed; a restart from a returned state lands. The
//! local connection gives the bytes of the web connection.

mod answer_text;

use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::capsule_insight::{
    self, CapsuleInsight, CapsuleLoad, CapsuleState, StepReject,
};
use genesis_kernel::frame::LastStateFrame64;
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
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

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

fn capsule() -> CapsuleInsight {
    CapsuleInsight::entry_capsule()
}

fn ceiling() -> f64 {
    capsule().gate_dt_s().unwrap()
}

fn floor() -> f64 {
    capsule().floor_dt_s().unwrap()
}

/// The lift over drag measured on the first 150 s of the flight.
const FLIGHT_LIFT: f64 = -0.031;
/// More steps than either entry lands before the floor.
const STEPS_ASKED: u64 = 3_000;

fn entry() -> (f64, f64, f64) {
    (
        capsule_insight::ENTRY_RADIUS_M,
        capsule_insight::ENTRY_RADIAL_SPEED_M_S,
        capsule_insight::ENTRY_TRANSVERSE_SPEED_M_S,
    )
}

fn step_body(phase: (f64, f64, f64), lift: f64, dt_s: f64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "capsule_insight",
        "parameters": {
            "radius_m": phase.0,
            "radial_speed_m_s": phase.1,
            "transverse_speed_m_s": phase.2,
            "lift_over_drag": lift,
            "dt_s": dt_s
        }
    })
}

fn run_body(phase: (f64, f64, f64), lift: f64, dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "aerospace",
        "card": "capsule_insight",
        "initial": {
            "radius_m": phase.0,
            "radial_speed_m_s": phase.1,
            "transverse_speed_m_s": phase.2
        },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": { "lift_over_drag": lift } }],
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

/// The kernel loop: steps of `dt_s` from the entry state until the kernel refuses.
fn direct_loop(lift: f64, dt_s: f64, limit: u64) -> (CapsuleState, u64, Option<StepReject>, f64, u64) {
    let capsule = capsule();
    let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
    let mut frame = blank_frame([0; 16]);
    let load = CapsuleLoad { lift_over_drag: lift };
    let mut landed = 0;
    let mut peak_g = 0.0;
    let mut peak_step = 0;
    while landed < limit {
        match capsule_insight::step(&capsule, &mut state, &mut frame, load, dt_s) {
            Ok(()) => {
                landed += 1;
                if state.load_g > peak_g {
                    peak_g = state.load_g;
                    peak_step = landed;
                }
            }
            Err(reject) => return (state, landed, Some(reject), peak_g, peak_step),
        }
    }
    (state, landed, None, peak_g, peak_step)
}

/// The phase of the kernel loop after `steps` steps from the entry state.
fn phase_after(lift: f64, steps: u64) -> (f64, f64, f64) {
    let capsule = capsule();
    let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
    let mut frame = blank_frame([0; 16]);
    for _ in 0..steps {
        capsule_insight::step(&capsule, &mut state, &mut frame, CapsuleLoad { lift_over_drag: lift }, ceiling())
            .unwrap();
    }
    (state.radius_m, state.radial_speed_m_s, state.transverse_speed_m_s)
}

/// The middle of the compared corridor at its top knot, 126.5 km.
fn top_of_the_corridor() -> (f64, f64, f64) {
    let last = capsule_insight::CORRIDOR_KNOTS - 1;
    let speed = 0.5
        * (capsule_insight::CORRIDOR_AIR_SPEED_LOW_M_S[last] + capsule_insight::CORRIDOR_AIR_SPEED_HIGH_M_S[last]);
    let sine = 0.5 * (capsule_insight::CORRIDOR_PATH_SINE_LOW[last] + capsule_insight::CORRIDOR_PATH_SINE_HIGH[last]);
    let radius_m = capsule_insight::DATUM_RADIUS_M + capsule_insight::CORRIDOR_TOP_M;
    let radial = sine * speed;
    let across = (speed * speed - radial * radial).sqrt();
    (radius_m, radial, across + capsule_insight::AIR_ROTATION_RATE_RAD_S * radius_m)
}

/// The published number of a state, by its name, from the hex beside it.
fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn state_numbers(state: &CapsuleState) -> Vec<(&'static str, f64)> {
    vec![
        ("t_s", state.t_s),
        ("altitude_m", state.altitude_m),
        ("radius_m", state.radius_m),
        ("radial_speed_m_s", state.radial_speed_m_s),
        ("transverse_speed_m_s", state.transverse_speed_m_s),
        ("angle_rad", state.angle_rad),
        ("ground_distance_m", state.ground_distance_m),
        ("air_speed_m_s", state.air_speed_m_s),
        ("mach", state.mach),
        ("load_g", state.load_g),
        ("dynamic_pressure_pa", state.dynamic_pressure_pa),
        ("drag_loss_j_kg", state.drag_loss_j_kg),
        ("energy_anchor_j_kg", state.energy_anchor_j_kg),
        ("energy_defect_j_kg", state.energy_defect_j_kg),
        ("residual_conservation_error", state.residual_conservation_error),
    ]
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// both as the decimal and as the hex beside it, and nothing else is published.
fn assert_state_is(published_state: &Value, state: &CapsuleState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in state_numbers(state) {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    assert_eq!(object["lift_acting"], state.lift_acting, "{label}");
    keys.insert("lift_acting".to_string());
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
}

fn split_receipt(text: &str) -> (String, String) {
    const MARKER: &str = ",\"receipt_sha256\":\"";
    let at = text.rfind(MARKER).expect("a receipt");
    (format!("{}}}", &text[..at]), text[at + MARKER.len()..text.len() - 2].to_string())
}

// -------------------------------------------------------------- the step

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_capsule_matches_the_compiled_step_with_the_same_proof_bytes() {
    let base = spawn_app().await;
    let datum = capsule_insight::DATUM_RADIUS_M;
    let cases = [
        step_body(entry(), 0.0, ceiling()),
        step_body(entry(), FLIGHT_LIFT, ceiling()),
        step_body(entry(), capsule_insight::LIFT_OVER_DRAG_LIMIT, floor()),
        // States the compared entry returns: at the peak of the load, and a
        // second before the compared floor.
        step_body(phase_after(FLIGHT_LIFT, 1065), -capsule_insight::LIFT_OVER_DRAG_LIMIT, ceiling()),
        step_body(phase_after(FLIGHT_LIFT, 2100), FLIGHT_LIFT, 0.05),
    ];
    let _ = datum;
    let mut seals = Vec::new();
    for request in cases {
        let text = serde_json::to_string(&request).unwrap();
        let (status, answer) = post_text(&base, "/verify_physics", &text).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);

        // The compiled step, with the proof the gateway writes.
        let parameters = &request["parameters"];
        let number = |name: &str| parameters[name].as_f64().unwrap();
        let capsule = capsule();
        let mut state = CapsuleState::try_state(
            &capsule,
            number("radius_m"),
            number("radial_speed_m_s"),
            number("transverse_speed_m_s"),
        )
        .unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        capsule_insight::step(
            &capsule,
            &mut state,
            &mut frame,
            CapsuleLoad { lift_over_drag: number("lift_over_drag") },
            number("dt_s"),
        )
        .unwrap();
        assert_eq!(frame.flags & !capsule_insight::FLAG_LIFT_ACTING, 0);
        assert_eq!(
            frame.flags & capsule_insight::FLAG_LIFT_ACTING != 0,
            number("lift_over_drag") != 0.0
        );
        assert_state_is(&response.state, &state, "step");
        assert_eq!(
            response.residual_conservation_error.to_bits(),
            state.residual_conservation_error.to_bits()
        );
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        // The proof is in the bytes: the same frame with a zero proof hashes apart.
        let mut naked = frame;
        naked.proof = [0; 16];
        assert_ne!(response.sha256_seal, sha256_hex(&naked.to_bytes()));
        let (document, receipt) = split_receipt(&answer);
        assert_eq!(receipt, response.receipt_sha256);
        assert_eq!(sha256_hex(document.as_bytes()), receipt);
        seals.push(response.sha256_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 5, "five different steps, five different proofs");
}

// ---------------------------------------------------------------- the run

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_whole_entry_ends_at_the_flown_deployment_with_the_state_of_the_kernel_loop() {
    let base = spawn_app().await;
    // (lift, steps that land, the step of the largest load, that load in g):
    // the model in table form, `card_reference.json` of the research record,
    // to the flown deployment.
    for (lift, steps, peak_step, peak_g) in [
        (0.0, 2321_u64, 1047_u64, 7.600699812494663_f64),
        (FLIGHT_LIFT, 2106, 1065, 7.951853813798471),
    ] {
        let request = run_body(entry(), lift, ceiling(), STEPS_ASKED, 1);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "lift {lift}: {answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();

        let (state, landed, reject, direct_peak_g, direct_peak_step) = direct_loop(lift, ceiling(), STEPS_ASKED);
        assert_eq!(reject, Some(StepReject::PastFlightDeployment), "lift {lift}");
        assert_eq!(landed, steps, "lift {lift}");
        assert_eq!(direct_peak_step, peak_step, "lift {lift}");
        assert_eq!(direct_peak_g.to_bits(), peak_g.to_bits(), "lift {lift}");
        // The flown parachute deployment is the end the card declares: the entry is sealed as ended there.
        assert_eq!(tape["outcome"], "ended", "lift {lift}");
        assert_eq!(tape["reject"], "PastFlightDeployment", "lift {lift}");
        assert_eq!(
            tape["reject_detail"],
            format!("Ended: the run reached the end the capsule_insight card declares (PastFlightDeployment): the flown parachute deployment, the last row of the flight's record, altitude under 6164.62 m and speed through the air under 352.1169 m/s at once, where the card's entry ends; the step that first crosses both lands, and with the flight's lift that step is at 5793 m and Mach 1.470, the lowest state the compared run reached. The step past it is not taken, and the run is sealed to step {landed}."),
            "lift {lift}"
        );
        assert_eq!(tape["steps_landed"], landed, "lift {lift}");
        assert_eq!(tape["steps_requested"], STEPS_ASKED);
        assert_state_is(&tape["final_state"], &state, &format!("lift {lift}"));
        // Both stop at the first state under both deployment conditions.
        assert!(state.altitude_m < capsule_insight::FLIGHT_DEPLOYMENT_ALTITUDE_M);
        assert!(state.air_speed_m_s < capsule_insight::FLIGHT_DEPLOYMENT_AIR_SPEED_M_S);
        assert!(tape["max_residual_conservation_error"].as_f64().unwrap() < 2.0e-12);

        // One frame for each landed step, and the frame of the largest load holds it.
        let frames = tape["frames"].as_array().unwrap();
        assert_eq!(frames.len() as u64, landed, "lift {lift}");

        // The tape verifies, and the receipt of the response matches.
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "lift {lift}: {report}");
        let (status, receipt) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{receipt}");
        let receipt: Value = serde_json::from_str(&receipt).unwrap();
        assert_eq!(receipt["matches"], true, "lift {lift}: {receipt}");
        assert_eq!(receipt["receipt_sha256"], tape["receipt_sha256"]);

        // The same request gives the same proof.
        let (_, again) = post(&base, "/run_trajectory", &request).await;
        let again: Value = serde_json::from_str(&again).unwrap();
        assert_eq!(again["run_seal"], tape["run_seal"]);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_later_segment_changes_the_lift_and_a_changed_tape_breaks() {
    let base = spawn_app().await;
    let (radius_m, radial, transverse) = entry();
    // The flight's lift to 150 s and then the card's limit upward, a change
    // that was compared, then a third lift that keeps the capsule in the
    // corridor.
    let request = json!({
        "domain": "aerospace",
        "card": "capsule_insight",
        "initial": { "radius_m": radius_m, "radial_speed_m_s": radial, "transverse_speed_m_s": transverse },
        "dt_s": ceiling(),
        "segments": [
            { "steps": 1500, "load": { "lift_over_drag": FLIGHT_LIFT } },
            { "steps": 300, "load": { "lift_over_drag": 0.08 } },
            { "steps": 300, "load": { "lift_over_drag": -0.02 } }
        ],
        "record_every": 100
    });
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let mut tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{answer}");
    assert_eq!(tape["steps_landed"], 2100);
    // The kernel loop with the lift changed at steps 1500 and 1800.
    let capsule = capsule();
    let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
    let mut frame = blank_frame([0; 16]);
    for (steps, lift) in [(1500, FLIGHT_LIFT), (300, 0.08), (300, -0.02)] {
        for _ in 0..steps {
            capsule_insight::step(&capsule, &mut state, &mut frame, CapsuleLoad { lift_over_drag: lift }, ceiling())
                .unwrap();
        }
    }
    assert_state_is(&tape["final_state"], &state, "three segments");
    assert_eq!(tape["final_state"]["lift_acting"], true);
    // A changed number in the state breaks the tape.
    tape["final_state"]["altitude_m_hex"] = json!("40d0000000000000");
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");

    // No lift to 60 s, then -0.05 and +0.05: a change no compared run made.
    // The run leaves the corridor the compared runs flew, where engines 0.7
    // to 0.9 refused it, and goes on, on the kernel loop's state to the bit.
    let mut wander = request.clone();
    wander["segments"] = json!([
        { "steps": 600, "load": { "lift_over_drag": 0.0 } },
        { "steps": 600, "load": { "lift_over_drag": -0.05 } },
        { "steps": 300, "load": { "lift_over_drag": 0.05 } }
    ]);
    let (status, answer) = post(&base, "/run_trajectory", &wander).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{answer}");
    assert_eq!(tape["steps_landed"], 1500);
    let mut state = CapsuleState::try_entry_interface(&capsule).unwrap();
    for (steps, lift) in [(600, 0.0), (600, -0.05), (300, 0.05)] {
        for _ in 0..steps {
            capsule_insight::step(&capsule, &mut state, &mut frame, CapsuleLoad { lift_over_drag: lift }, ceiling())
                .unwrap();
        }
    }
    assert_state_is(&tape["final_state"], &state, "wandering run");
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true);
}

// -------------------------------------------------------------- the limits

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_lift_lands_at_its_limit_and_is_refused_by_name_at_the_next_number_outside() {
    let base = spawn_app().await;
    let limit = capsule_insight::LIFT_OVER_DRAG_LIMIT;
    let (_, described) = post(&base, "/describe_card", &json!({"domain": "aerospace", "card": "capsule_insight"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    assert_eq!(described["load_fields"][0]["minimum"].as_f64().unwrap().to_bits(), (-limit).to_bits());
    assert_eq!(described["load_fields"][0]["maximum"].as_f64().unwrap().to_bits(), limit.to_bits());

    for inside in [limit, -limit, 0.0, -0.0] {
        let (status, text) = post(&base, "/verify_physics", &step_body(entry(), inside, ceiling())).await;
        assert_eq!(status, 200, "{inside}: {text}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(entry(), inside, ceiling(), 3, 1)).await;
        assert_eq!(status, 200, "{inside}: {text}");
        assert!(text.contains("\"outcome\":\"completed\""), "{text}");
    }
    for outside in [above(limit), -above(limit), 1.0, -1.0] {
        let (status, text) = post(&base, "/verify_physics", &step_body(entry(), outside, ceiling())).await;
        assert_eq!(status, 422, "{outside}: {text}");
        assert!(text.contains("'lift_over_drag'") && text.contains("LiftOutsideCard"), "{text}");
        assert!(!text.contains("sha256_seal"), "{text}");
        // In the first segment of a run and in a later one: the whole run is
        // refused before its first step.
        for position in [0, 1] {
            let mut run = run_body(entry(), 0.0, ceiling(), 3, 1);
            let segment = run["segments"][0].clone();
            run["segments"] = json!([segment.clone(), segment]);
            run["segments"][position]["load"]["lift_over_drag"] = json!(outside);
            let (status, text) = post(&base, "/run_trajectory", &run).await;
            assert_eq!(status, 422, "{outside} segment {position}: {text}");
            assert!(text.contains("'lift_over_drag'") && text.contains("LiftOutsideCard"), "{text}");
            assert!(!text.contains("run_seal"), "{text}");
        }
    }
    // A run segment with no lift named is refused: nothing is filled in.
    let mut run = run_body(entry(), 0.0, ceiling(), 3, 1);
    run["segments"][0]["load"] = json!({});
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("lift_over_drag") && !text.contains("run_seal"), "{text}");
    let mut step = step_body(entry(), 0.0, ceiling());
    step["parameters"].as_object_mut().unwrap().remove("lift_over_drag");
    let (status, text) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("lift_over_drag"), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_start_off_the_card_is_refused_by_name_and_the_edges_of_the_card_land() {
    let base = spawn_app().await;
    let datum = capsule_insight::DATUM_RADIUS_M;
    let named = |text: &str, name: &str| {
        text.contains(&format!("({name}). That state is not sealed.")) && !text.contains("sha256_seal") && !text.contains("run_seal")
    };
    // (phase, the refusal).
    let off: [((f64, f64, f64), &str); 6] = [
        ((above(datum + 128_000.0), -1000.0, 5000.0), "AltitudeOutsideCard"),
        ((below(datum + capsule_insight::LANDING_SITE_ALTITUDE_M), -100.0, 600.0), "BelowLandingSite"),
        // 173 m/s through air whose speed of sound is 235 m/s.
        ((datum + 10_000.0, -50.0, 400.0), "MachOutsideCard"),
        ((datum + 60_000.0, -1000.0, 6_200.0), "AirSpeedOutsideCard"),
        ((datum + 60_000.0, -6_000.0, 240.0), "AirSpeedOutsideCard"),
        // 6 km at 300 m/s: past the flown deployment.
        ((datum + 6_000.0, -100.0, 517.0), "PastFlightDeployment"),
    ];
    for (phase, name) in off {
        let (status, text) = post(&base, "/verify_physics", &step_body(phase, 0.0, ceiling())).await;
        assert_eq!(status, 422, "{phase:?}: {text}");
        assert!(named(&text, name), "{phase:?} {name}: {text}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(phase, 0.0, ceiling(), 3, 1)).await;
        assert_eq!(status, 422, "{phase:?}: {text}");
        assert!(named(&text, name), "{phase:?} {name}: {text}");
        assert!(CapsuleState::try_state(&capsule(), phase.0, phase.1, phase.2).is_none());
    }
    // Where engines 0.7 to 0.9 refused, the card computes: climbing at 4 km,
    // Mach 1.27 at 9 km, 5804 m/s at 100 km, climbing at 60 km, the top of
    // the air tables, 4 km at Mach 1.57 and the bottom of the old tables.
    for phase in [
        (datum + 4_000.0, 50.0, 600.0),
        (datum + 9_000.0, -150.0, 494.0),
        (datum + 100_000.0, -1000.0, 5_954.0),
        (datum + 60_000.0, 100.0, 5_700.0),
        (datum + 128_000.0, -100.0, 5000.0),
        (below(datum + 4_000.0), -100.0, 600.0),
    ] {
        let (status, text) = post(&base, "/verify_physics", &step_body(phase, 0.0, ceiling() / 8.0)).await;
        assert_eq!(status, 200, "{phase:?}: {text}");
        assert!(text.contains("sha256_seal"), "{text}");
    }
    // The top knot of the corridor is on the card: a capsule going down
    // from it lands.
    let (status, text) = post(&base, "/verify_physics", &step_body(top_of_the_corridor(), 0.0, floor())).await;
    assert_eq!(status, 200, "{text}");
    // The step: its ceiling and floor land, and the next number outside each is refused.
    for dt_s in [ceiling(), floor()] {
        let (status, text) = post(&base, "/verify_physics", &step_body(entry(), FLIGHT_LIFT, dt_s)).await;
        assert_eq!(status, 200, "{dt_s}: {text}");
    }
    let (status, text) = post(&base, "/verify_physics", &step_body(entry(), 0.0, above(ceiling()))).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        format!(
            // Re-pinned for 0.10.1: a step above the ceiling carries the name DtAboveCeiling.
            "Physical Violation: 'dt_s' {} is above 0.1 s, the step ceiling of the capsule_insight card (DtAboveCeiling).",
            above(ceiling())
        )
    );
    let (status, text) = post(&base, "/verify_physics", &step_body(entry(), 0.0, below(floor()))).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'dt_s'") && text.contains("capsule_insight"), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_card_constant_sent_as_a_field_is_refused() {
    let base = spawn_app().await;
    for constant in [
        "capsule_mass_kg",
        "reference_area_m2",
        "air_density_kg_m3",
        "drag_by_speed",
        "datum_radius_m",
        "lift_over_drag_limit",
        "residual_conservation_limit",
        "altitude_m",
    ] {
        let mut step = step_body(entry(), 0.0, ceiling());
        step["parameters"][constant] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &step).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert_eq!(text, format!("Physical Violation: '{constant}' is not a field of this request."));

        let mut in_phase = run_body(entry(), 0.0, ceiling(), 3, 1);
        in_phase["initial"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_phase).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");

        let mut in_load = run_body(entry(), 0.0, ceiling(), 3, 1);
        in_load["segments"][0]["load"][constant] = json!(1.0);
        let (status, text) = post(&base, "/run_trajectory", &in_load).await;
        assert_eq!(status, 422, "{constant}: {text}");
        assert!(text.contains(constant) && !text.contains("run_seal"), "{text}");
    }
    // The lift is a load, not a part of the starting state.
    let mut run = run_body(entry(), 0.0, ceiling(), 3, 1);
    run["initial"]["lift_over_drag"] = json!(0.0);
    let (status, text) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("lift_over_drag") && !text.contains("run_seal"), "{text}");
}

/// A run past the flight's parachute deployment, once sealed down to 4010 m
/// and Mach 1.20. It now ends at the flown deployment, the end the card
/// declares, and no landed state is past both conditions.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_entry_past_the_flights_deployment_is_refused_by_name_and_a_restart_lands() {
    let base = spawn_app().await;
    for lift in [0.0, -capsule_insight::LIFT_OVER_DRAG_LIMIT] {
        let (status, text) = post(&base, "/run_trajectory", &run_body(entry(), lift, ceiling(), STEPS_ASKED, 3_000)).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "ended", "lift {lift}");
        assert_eq!(tape["reject"], "PastFlightDeployment", "lift {lift}");
        let last = &tape["final_state"];
        assert!(published(last, "altitude_m") < capsule_insight::FLIGHT_DEPLOYMENT_ALTITUDE_M, "lift {lift}");
        assert!(published(last, "air_speed_m_s") < capsule_insight::FLIGHT_DEPLOYMENT_AIR_SPEED_M_S, "lift {lift}");
        assert!(last.get("past_flight_deployment").is_none(), "{last}");
    }
    // With the most lift away from Mars the capsule slows above the
    // deployment altitude, where engines 0.7 to 0.9 stopped it at Mach 1.46;
    // it goes on and no landed state is past both conditions.
    let lift = capsule_insight::LIFT_OVER_DRAG_LIMIT;
    let (status, text) = post(&base, "/run_trajectory", &run_body(entry(), lift, ceiling(), STEPS_ASKED, 3_000)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert!(tape["outcome"] == "completed" || tape["outcome"] == "ended", "{}", tape["outcome"]);
    let last = &tape["final_state"];
    assert!(published(last, "mach") < capsule_insight::COMPARED_MACH_FLOOR, "{last}");

    // A restart from a state the run returns lands, and lands on the phase
    // of the run that went straight on.
    let (_, first) = post(&base, "/run_trajectory", &run_body(entry(), FLIGHT_LIFT, ceiling(), 1_500, 1_500)).await;
    let first: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first["outcome"], "completed");
    let at = &first["final_state"];
    let restart = (
        published(at, "radius_m"),
        published(at, "radial_speed_m_s"),
        published(at, "transverse_speed_m_s"),
    );
    let (status, then) = post(&base, "/run_trajectory", &run_body(restart, FLIGHT_LIFT, ceiling(), 500, 500)).await;
    assert_eq!(status, 200, "{then}");
    let then: Value = serde_json::from_str(&then).unwrap();
    assert_eq!(then["outcome"], "completed", "{then}");
    let straight = phase_after(FLIGHT_LIFT, 2_000);
    let ended = &then["final_state"];
    assert_eq!(published(ended, "radius_m").to_bits(), straight.0.to_bits());
    assert_eq!(published(ended, "radial_speed_m_s").to_bits(), straight.1.to_bits());
    assert_eq!(published(ended, "transverse_speed_m_s").to_bits(), straight.2.to_bits());
    // The time label restarts with the ledger; no limit depends on it.
    assert!((published(ended, "t_s") - 50.0).abs() < 1.0e-9);
}

// ---------------------------------------------------------------- dataset

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_grid_of_three_lifts_gives_rows_whose_proofs_are_the_runs_alone() {
    let base = spawn_app().await;
    let lifts = [-0.05, 0.0, 0.05];
    let mut plan_base = run_body(entry(), 0.0, ceiling(), 1_200, 400);
    plan_base.as_object_mut().unwrap().remove("domain");
    plan_base.as_object_mut().unwrap().remove("card");
    plan_base["segments"][0]["load"].as_object_mut().unwrap().remove("lift_over_drag");
    let request = json!({
        "domain": "aerospace",
        "card": "capsule_insight",
        "plan": { "kind": "grid", "base": plan_base, "axes": [
            { "path": "segments[0].load.lift_over_drag", "values": lifts }
        ] },
        "store": "none",
        "return": "inline", "answer": "full"
    });
    let (status, text) = post(&base, "/run_dataset", &request).await;
    assert_eq!(status, 200, "{text}");
    let dataset: Value = serde_json::from_str(&text).unwrap();
    let rows = dataset["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3, "{dataset}");
    let mut altitudes = Vec::new();
    for (row, lift) in rows.iter().zip(lifts) {
        let (_, alone) = post(&base, "/run_trajectory", &run_body(entry(), lift, ceiling(), 1_200, 400)).await;
        let alone: Value = serde_json::from_str(&alone).unwrap();
        assert_eq!(row["run_seal"], alone["run_seal"], "lift {lift}");
        altitudes.push(published(&alone["final_state"], "altitude_m"));
    }
    // Lift toward Mars brings the capsule down, lift away from Mars holds it up.
    assert!(altitudes[0] < altitudes[1] - 1_000.0 && altitudes[1] < altitudes[2] - 1_000.0, "{altitudes:?}");
    let (status, report) = post(&base, "/verify_dataset", &dataset).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
}

// ------------------------------------------------------ the local connection

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

    async fn call(&mut self, name: &str, arguments: &Value) -> String {
        let line = answer_text::tool_call(7, name, arguments);
        self.raw(&line.to_string()).await
    }
}

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
async fn the_local_connection_lists_the_capsule_and_gives_the_bytes_of_the_web_connection() {
    let web = spawn_app().await;
    let mut local = Local::start().await;

    // Thirty cards, the InSight capsule the nineteenth, in the aerospace domain.
    let (listed, error) = result_text(&local.call("list_cards", &json!({})).await);
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 37);
    assert_eq!(cards[18]["domain"], "aerospace");
    assert_eq!(cards[18]["card"], "capsule_insight");
    let on_web: Value = Client::new().get(format!("{web}/cards")).send().await.unwrap().json().await.unwrap();
    assert_eq!(on_web, listed);

    let pair = json!({"domain": "aerospace", "card": "capsule_insight"});
    let (described, error) = result_text(&local.call("describe_card", &pair).await);
    assert!(!error);
    let (status, on_web) = post(&web, "/describe_card", &pair).await;
    assert_eq!(status, 200);
    assert_eq!(described, on_web);
    let described: Value = serde_json::from_str(&described).unwrap();
    // Re-pinned for 0.10.1: after the kernel's 12 names the list holds the gateway's own two, DtBelowFloor and DtAboveCeiling.
    assert_eq!(described["rejects"].as_array().unwrap().len(), 14);

    // The description's own examples run as written, on both connections, to the same bytes.
    let step = described["example_compute_step"].clone();
    let (stepped, error) = result_text(&local.call("verify_step", &step).await);
    assert!(!error, "{stepped}");
    let (status, on_web) = post(&web, "/verify_physics", &step).await;
    assert_eq!(status, 200);
    assert_eq!(stepped, on_web);

    let run = described["example_run_trajectory"].clone();
    let (ran, error) = result_text(&local.call("run_trajectory", &run).await);
    assert!(!error, "{ran}");
    let (status, on_web) = post(&web, "/run_trajectory", &run).await;
    assert_eq!(status, 200);
    assert_eq!(ran, on_web);
    let tape: Value = serde_json::from_str(&ran).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], 2000);
    assert_eq!(tape["frames"].as_array().unwrap().len(), 200);
    let (checked, error) = result_text(&local.call("verify_tape", &tape).await);
    assert!(!error, "{checked}");
    assert!(checked.contains("\"holds\":true"), "{checked}");

    // A refusal is the same text on both.
    let outside = step_body(entry(), 0.09, ceiling());
    let (refused, error) = result_text(&local.call("verify_step", &outside).await);
    assert!(error);
    let (status, on_web) = post(&web, "/verify_physics", &outside).await;
    assert_eq!(status, 422);
    assert_eq!(refused, on_web);
    assert!(refused.contains("LiftOutsideCard"), "{refused}");
}
