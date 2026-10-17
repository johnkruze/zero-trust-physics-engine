//! The drive-mode resonator, `signal/tesseract`, through the door.
//!
//! Three things are held here:
//!
//! - the ledger belongs to the run: a state handed back carries its
//!   `energy_defect_j`, a start without it is refused, and a run cut in two
//!   and continued with it is the run straight through;
//! - 1/128 m is the card's amplitude limit and is not called a fold, and no
//!   frame carries a fold flag;
//! - a state outside the published bounds is refused by name on every route:
//!   a run that would reach it is sealed up to its last landed step and the
//!   state itself is not sealed.

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::tesseract;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::net::TcpListener;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, protocol_mcp_gateway::app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::new();
    for _ in 0..20 {
        match client
            .post(format!("{base}{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                return (status, response.text().await.unwrap());
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(25)).await,
        }
    }
    panic!("{path} never accepted a connection");
}

async fn describe(base: &str) -> Value {
    let (status, text) = post(
        base,
        "/describe_card",
        &json!({ "domain": "signal", "card": "tesseract" }),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn run(base: &str, request: &Value) -> Value {
    let (status, text) = post(base, "/run_trajectory", request).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

const CEILING_S: f64 = 3.978873577297384e-05;

fn load(accel_m_s2: f64) -> Value {
    json!({ "accel_m_s2": accel_m_s2, "omega_ext_rad_s": 0.0, "sense_velocity_m_s": 1.0 })
}

fn request(initial: Value, segments: Value) -> Value {
    let steps: u64 = segments
        .as_array()
        .unwrap()
        .iter()
        .map(|segment| segment["steps"].as_u64().unwrap())
        .sum();
    json!({
        "domain": "signal",
        "card": "tesseract",
        "initial": initial,
        "dt_s": CEILING_S,
        "segments": segments,
        "record_every": steps
    })
}

/// The state a run returned, as the next run's start: its displacement, its
/// velocity and its ledger defect.
fn handed_back(body: &Value) -> Value {
    let state = &body["final_state"];
    json!({
        "displacement_m": state["displacement_m"],
        "velocity_m_s": state["velocity_m_s"],
        "energy_defect_j": state["energy_defect_j"],
    })
}

/// A start without the ledger is refused by name, on a run and on a step.
#[tokio::test]
async fn a_start_without_energy_defect_j_is_refused() {
    let base = spawn_app().await;
    let card = describe(&base).await;
    let phase: Vec<&str> = card["phase_fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap())
        .collect();
    assert_eq!(phase, ["displacement_m", "velocity_m_s", "energy_defect_j"]);

    let mut step = card["example_compute_step"].clone();
    assert_eq!(step["parameters"]["energy_defect_j"], json!(0.0));
    step["parameters"]
        .as_object_mut()
        .unwrap()
        .remove("energy_defect_j");
    let (status, text) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'energy_defect_j'"), "{text}");
    assert!(!text.contains("sha256_seal"), "{text}");

    let mut trajectory = card["example_run_trajectory"].clone();
    trajectory["initial"]
        .as_object_mut()
        .unwrap()
        .remove("energy_defect_j");
    let (status, text) = post(&base, "/run_trajectory", &trajectory).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'energy_defect_j'"), "{text}");
    assert!(!text.contains("run_seal"), "{text}");
}

/// A run cut in two and continued from its returned state with that state's
/// energy_defect_j ends where the run straight through ends, with the same
/// displacement and velocity bits and the same ledger defect to within one
/// rounding of the energy.
#[tokio::test]
async fn a_run_cut_and_continued_with_its_defect_is_the_run_straight_through() {
    let base = spawn_app().await;
    let rest = json!({ "displacement_m": 0.0, "velocity_m_s": 0.0, "energy_defect_j": 0.0 });
    let segments = json!([
        { "steps": 300, "load": load(800.0) },
        { "steps": 700, "load": load(0.0) },
        { "steps": 500, "load": load(-600.0) }
    ]);
    let straight = run(&base, &request(rest.clone(), segments.clone())).await;
    assert_eq!(straight["outcome"], "completed");

    let first_request = request(
        rest,
        json!([
            { "steps": 300, "load": load(800.0) },
            { "steps": 400, "load": load(0.0) }
        ]),
    );
    let first = run(&base, &first_request).await;
    assert_eq!(first["outcome"], "completed");
    let start = handed_back(&first);
    assert!(start["energy_defect_j"].as_f64().unwrap() != 0.0);
    // The carried defect carries the run on: the request names the sealed
    // first part in from, and without it is refused by name.
    let mut second_request = request(
        start,
        json!([
            { "steps": 300, "load": load(0.0) },
            { "steps": 500, "load": load(-600.0) }
        ]),
    );
    let (status, text) = post(&base, "/run_trajectory", &second_request).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    second_request["from"] = json!({ "request": first_request, "run_seal": first["run_seal"] });
    let second = run(&base, &second_request).await;
    assert_eq!(second["outcome"], "completed");
    for key in ["displacement_m_hex", "velocity_m_s_hex"] {
        assert_eq!(
            second["final_state"][key], straight["final_state"][key],
            "{key}"
        );
    }
    let card = tesseract::Tesseract::drive_mode();
    let apart = (second["final_state"]["energy_defect_j"].as_f64().unwrap()
        - straight["final_state"]["energy_defect_j"].as_f64().unwrap())
    .abs();
    assert!(apart <= 1.0e-6 * card.defect_limit_j(), "{apart}");
}

/// The carried defect reaches the ledger's gate. The state with a defect of
/// zero lands its step; a start whose defect sits at the limit carries a run
/// on and, stated without the sealed run that wrote it, is refused by name.
#[tokio::test]
async fn a_carried_defect_at_the_limit_is_refused_on_the_next_step() {
    let base = spawn_app().await;
    let card = tesseract::Tesseract::drive_mode();
    let limit_j = card.defect_limit_j();
    let edge = json!({
        "displacement_m": tesseract::AMPLITUDE_LIMIT_M,
        "velocity_m_s": 0.0,
        "energy_defect_j": 0.0
    });
    let segments = json!([{ "steps": 1, "load": load(0.0) }]);
    let fresh = run(&base, &request(edge.clone(), segments.clone())).await;
    assert_eq!(fresh["outcome"], "completed");
    let moved_j = fresh["final_state"]["energy_defect_j"].as_f64().unwrap();
    assert!(moved_j != 0.0);

    // A defect stated at the limit carries a run on. The card takes it at the
    // start; without from, naming the sealed run that carried it there, the
    // door refuses it by name, so a stated defect cannot be given without the
    // run that wrote it. (On the card, the next step from it is refused as
    // SpontaneousGeneration or FirstLawBreach.)
    let mut carried = edge;
    carried["energy_defect_j"] = json!(limit_j.copysign(moved_j));
    let (status, text) = post(&base, "/run_trajectory", &request(carried, segments)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream).") && !text.contains("run_seal"), "{text}");
}

/// The state a run reached in six steps, 23.47 m/s, was sealed with a flag
/// on 0.6.0; now the run is refused by name on the step that would leave the
/// speed limit, its first step keeps its proof, and a start on that state is
/// turned away at the door.
#[tokio::test]
async fn a_state_outside_the_bounds_is_refused_whether_reached_or_started_on() {
    let base = spawn_app().await;
    let reached = run(
        &base,
        &request(
            json!({ "displacement_m": 0.0, "velocity_m_s": 0.0, "energy_defect_j": 0.0 }),
            json!([{ "steps": 6, "load": load(98696.04401089357) }]),
        ),
    )
    .await;
    assert_eq!(reached["outcome"], "refused");
    assert_eq!(reached["reject"], "AmplitudeOutsideCard");
    assert_eq!(reached["steps_landed"], 1);
    assert!(reached["run_seal"].is_string());
    let detail = reached["reject_detail"].as_str().unwrap();
    assert!(
        detail.starts_with("Physical Failure: the step would land the displacement outside [-0.0078125, 0.0078125] m or the velocity outside [-4.066797753299356, 4.066797753299356] m/s"),
        "{detail}"
    );
    assert!(detail.ends_with("(AmplitudeOutsideCard). The run is sealed to step 1. The state at step 2 was refused and is not sealed."), "{detail}");
    let speed_m_s = reached["final_state"]["velocity_m_s"].as_f64().unwrap();
    assert!(speed_m_s.abs() <= tesseract::Tesseract::drive_mode().speed_limit_m_s());

    let (status, text) = post(
        &base,
        "/run_trajectory",
        &request(
            json!({
                "displacement_m": 0.0028063067582247345,
                "velocity_m_s": 23.46976127355659,
                "energy_defect_j": 0.0
            }),
            json!([{ "steps": 1, "load": load(0.0) }]),
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.starts_with("Physical Failure: 'velocity_m_s' 23.46976127355659 m/s is above 4.066797753299356 m/s"), "{text}");
    assert!(!text.contains("run_seal"), "{text}");
}

/// 1/128 m is called the amplitude limit, the two branches are placed at
/// 4.026 mm, and no frame or state carries a fold flag: a mass released at
/// 7.75 mm rings for 400 steps with every frame's flags zero.
#[tokio::test]
async fn the_amplitude_limit_is_not_called_a_fold_and_no_frame_is_flagged() {
    let base = spawn_app().await;
    let card = describe(&base).await;
    let text = card.to_string();
    assert!(!text.contains("fold amplitude"), "{text}");
    assert!(!text.contains("FLAG_DUFFING_FOLD"), "{text}");
    assert!(text.contains("two coexisting branches from an amplitude of 4.026 mm"), "{text}");
    assert!(text.contains("AmplitudeOutsideCard"), "{text}");

    let mut released = request(
        json!({ "displacement_m": 0.00775, "velocity_m_s": 0.0, "energy_defect_j": 0.0 }),
        json!([{ "steps": 400, "load": load(0.0) }]),
    );
    released["record_every"] = json!(1);
    let body = run(&base, &released).await;
    assert_eq!(body["outcome"], "completed");
    assert!(body["final_state"].get("folded").is_none(), "{body}");
    let frames = body["frames"].as_array().expect("frames");
    assert_eq!(frames.len(), 400);
    for frame in frames {
        let text = frame["frame_hex"].as_str().unwrap();
        let mut bytes = [0u8; 64];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[2 * index..2 * index + 2], 16).unwrap();
        }
        assert_eq!(LastStateFrame64::from_bytes(bytes).flags, 0, "{frame}");
    }
}
