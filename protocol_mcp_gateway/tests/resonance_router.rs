//! The 10 Hz mount card on the web connection: the ledger runs across
//! continued requests.
//!
//! A run cut into requests, each handing back the returned t_s,
//! energy_anchor_j and boundary_work_j with the displacement and the
//! velocity, ends on the same state, ledger and residual as the run sent
//! whole, and the resonant drive cut part way is refused on the same step.
//! A stated ledger the card could not have written is refused by name at the
//! door and carries no proof. A request without the ledger is refused. The
//! description says the ledger runs across continued requests, and the
//! ledger refusals no longer say the mount created or destroyed energy.

use std::time::Duration;

use genesis_kernel::resonance::{
    self, DynamicOscillator, OscillatorLoad, OscillatorState, RESIDUAL_CONSERVATION_LIMIT,
    START_TIME_MAX_S,
};
use genesis_kernel::frame::LastStateFrame64;
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
    let client = Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .unwrap();
    let url = format!("{base}{path}");
    let text = serde_json::to_string(body).unwrap();
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(text.clone())
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

fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn gate() -> f64 {
    DynamicOscillator::mount_10hz().gate_dt_s().unwrap()
}

/// The initial object of a request from a returned state: the phase and the
/// ledger, read from the exact hex.
fn handed_back(state: &Value) -> Value {
    json!({
        "displacement_m": published(state, "displacement_m"),
        "velocity_m_s": published(state, "velocity_m_s"),
        "t_s": published(state, "t_s"),
        "energy_anchor_j": published(state, "energy_anchor_j"),
        "boundary_work_j": published(state, "boundary_work_j"),
    })
}

fn new_start(displacement_m: f64, velocity_m_s: f64) -> Value {
    json!({
        "displacement_m": displacement_m,
        "velocity_m_s": velocity_m_s,
        "t_s": 0.0,
        "energy_anchor_j": DynamicOscillator::mount_10hz().mechanical_energy_j(displacement_m, velocity_m_s),
        "boundary_work_j": 0.0,
    })
}

fn run_body(initial: Value, segments: Value, record_every: u64) -> Value {
    json!({
        "domain": "signal",
        "card": "resonance",
        "initial": initial,
        "dt_s": gate(),
        "segments": segments,
        "record_every": record_every,
    })
}

/// A request that names in `from` the sealed run it continues.
fn from(mut request: Value, upstream: &Value, answer: &Value) -> Value {
    request["from"] = json!({ "request": upstream, "run_seal": answer["run_seal"] });
    request
}

fn step_body(initial: &Value, accel_m_s2: f64) -> Value {
    let mut parameters = initial.clone();
    parameters["accel_m_s2"] = json!(accel_m_s2);
    parameters["dt_s"] = json!(gate());
    json!({ "domain": "signal", "card": "resonance", "parameters": parameters })
}

const LEDGER: [&str; 8] = [
    "t_s",
    "displacement_m",
    "velocity_m_s",
    "mechanical_energy_j",
    "energy_anchor_j",
    "boundary_work_j",
    "energy_defect_j",
    "residual_conservation_error",
];

fn assert_same_state(cut: &Value, whole: &Value) {
    for name in LEDGER {
        assert_eq!(
            published(cut, name).to_bits(),
            published(whole, name).to_bits(),
            "{name}: cut {cut} whole {whole}"
        );
    }
}

/// A short run cut in two: 1,000 steps from (0.005 m, 0.1 m/s) at +3 m/s^2,
/// straight and cut after 400. In 0.6.0 the cut run ended with a residual of
/// 7.895e-11, an anchor of 0.022857 J and t_s 0.19099 where the straight run
/// had 2.0778e-10, 0.054348 J and 0.31831. Now the two agree to the bit.
#[tokio::test]
async fn a_run_cut_and_handed_back_ends_on_the_straight_runs_state_and_ledger() {
    let base = spawn_app().await;
    let load = json!({ "accel_m_s2": 3.0 });
    let (status, whole) = post(
        &base,
        "/run_trajectory",
        &run_body(new_start(0.005, 0.1), json!([{ "steps": 1000, "load": load }]), 1000),
    )
    .await;
    assert_eq!(status, 200, "{whole}");
    let whole: Value = serde_json::from_str(&whole).unwrap();
    let first_request = run_body(new_start(0.005, 0.1), json!([{ "steps": 400, "load": load }]), 400);
    let (status, first) = post(&base, "/run_trajectory", &first_request).await;
    assert_eq!(status, 200, "{first}");
    let first: Value = serde_json::from_str(&first).unwrap();
    // Handed back, the clock and the ledger carry the run on: the request
    // names the sealed first part in from, and without it is refused by name.
    let second_request = run_body(handed_back(&first["final_state"]), json!([{ "steps": 600, "load": load }]), 600);
    let (status, text) = post(&base, "/run_trajectory", &second_request).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    let (status, second) = post(&base, "/run_trajectory", &from(second_request, &first_request, &first)).await;
    assert_eq!(status, 200, "{second}");
    let second: Value = serde_json::from_str(&second).unwrap();
    assert_same_state(&second["final_state"], &whole["final_state"]);
    assert_eq!(
        published(&whole["final_state"], "displacement_m").to_bits(),
        0x3f60a4230abc7ea8
    );
    assert_eq!(
        published(&whole["final_state"], "velocity_m_s").to_bits(),
        0xbfb30a0fda77c17f
    );
    let t_s = published(&second["final_state"], "t_s");
    assert!(t_s > 0.3183 && t_s < 0.3184, "t_s {t_s}");
    // The frame slot t carries the same running time.
    let frame_t = second["frames"][0]["t"].as_f64().unwrap_or(t_s);
    assert!((frame_t - t_s).abs() < 1.0e-12, "frame t {frame_t}");

    // Single steps handed back one at a time end where a run of ten does.
    let (_, ten) = post(
        &base,
        "/run_trajectory",
        &run_body(new_start(0.005, 0.1), json!([{ "steps": 10, "load": load }]), 10),
    )
    .await;
    let ten: Value = serde_json::from_str(&ten).unwrap();
    let mut initial = new_start(0.005, 0.1);
    let mut last = Value::Null;
    for before in 0..10_u64 {
        let mut body = step_body(&initial, 3.0);
        if before > 0 {
            let upstream = run_body(new_start(0.005, 0.1), json!([{ "steps": before, "load": load }]), before);
            let (_, up) = post(&base, "/run_trajectory", &upstream).await;
            body = from(body, &upstream, &serde_json::from_str(&up).unwrap());
        }
        let (status, answer) = post(&base, "/verify_physics", &body).await;
        assert_eq!(status, 200, "{answer}");
        let answer: Value = serde_json::from_str(&answer).unwrap();
        initial = handed_back(&answer["state"]);
        last = answer["state"].clone();
    }
    assert_same_state(&last, &ten["final_state"]);
}

/// A long run cut in two. Sent whole, the resonant square wave of
/// +/-3 m/s^2 is refused as SpontaneousGeneration after 447,824 steps. In
/// 0.6.0 the same drive in two legs of 299,870 and 149,954 steps completed
/// with a residual of 3.36e-7. Now the second leg, handed the ledger, is
/// refused on the same step with the same state.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_reach_of_the_ledger_is_the_same_whole_and_in_two_legs() {
    let base = spawn_app().await;
    // 157 steps a half wave: segments of 157 steps alternating +3 and -3.
    let segments = |from: u64, to: u64| {
        let mut out = Vec::new();
        let mut n = from;
        while n < to {
            let end = ((n / 157) + 1) * 157;
            let end = end.min(to);
            let accel = if (n / 157) % 2 == 0 { 3.0 } else { -3.0 };
            out.push(json!({ "steps": end - n, "load": { "accel_m_s2": accel } }));
            n = end;
        }
        Value::Array(out)
    };
    let rest = new_start(0.0, 0.0);
    let (status, whole) = post(
        &base,
        "/run_trajectory",
        &run_body(rest.clone(), segments(0, 449_824), 10_000),
    )
    .await;
    assert_eq!(status, 200, "{whole}");
    let whole: Value = serde_json::from_str(&whole).unwrap();
    assert_eq!(whole["outcome"], "refused", "{}", whole["reject_detail"]);
    assert_eq!(whole["reject"], "SpontaneousGeneration");
    let landed = whole["steps_landed"].as_u64().unwrap();
    assert!((447_816..=447_832).contains(&landed), "landed {landed}");

    let first_request = run_body(rest, segments(0, 299_870), 10_000);
    let (status, first) = post(&base, "/run_trajectory", &first_request).await;
    assert_eq!(status, 200, "{first}");
    let first: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first["outcome"], "completed");
    // The second leg names the first in from: its ledger is checked against
    // the first leg's seal, and cannot be restated.
    let second_request = run_body(handed_back(&first["final_state"]), segments(299_870, 449_824), 10_000);
    let (status, second) = post(&base, "/run_trajectory", &from(second_request, &first_request, &first)).await;
    assert_eq!(status, 200, "{second}");
    let second: Value = serde_json::from_str(&second).unwrap();
    assert_eq!(second["outcome"], "refused");
    assert_eq!(second["reject"], "SpontaneousGeneration");
    assert_eq!(
        299_870 + second["steps_landed"].as_u64().unwrap(),
        landed
    );
    assert_same_state(&second["final_state"], &whole["final_state"]);
    let residual = published(&second["final_state"], "residual_conservation_error");
    assert!(residual > 0.999 * RESIDUAL_CONSERVATION_LIMIT && residual <= RESIDUAL_CONSERVATION_LIMIT);
    // The refusal says what the ledger saw, and not that the mount created energy.
    let detail = second["reject_detail"].as_str().unwrap();
    assert!(
        detail.contains("the step method's own error summed over the run since its anchor"),
        "{detail}"
    );
    assert!(!detail.contains("created mechanical energy"), "{detail}");
}

/// A stated ledger the card could not have written is refused at the door
/// by name and carries no proof; one inside the card is answered.
#[tokio::test]
async fn a_stated_ledger_the_card_could_not_have_written_is_refused_by_name() {
    let base = spawn_app().await;
    let oscillator = DynamicOscillator::mount_10hz();
    let gate_j = RESIDUAL_CONSERVATION_LIMIT * oscillator.stroke_energy_j();
    let x = 0.005;
    let e = oscillator.mechanical_energy_j(x, 0.0);
    let ledger = |t_s: f64, anchor: f64, work: f64| {
        json!({
            "displacement_m": x,
            "velocity_m_s": 0.0,
            "t_s": t_s,
            "energy_anchor_j": anchor,
            "boundary_work_j": work,
        })
    };
    let refused = [
        (ledger(START_TIME_MAX_S.next_up(), e, 0.0), "StartTimeOutsideCard"),
        (ledger(-1.0, e, 0.0), "StartTimeOutsideCard"),
        (ledger(1.0, -1.0e-3, e + 1.0e-3), "LedgerNotOfThisState"),
        (ledger(1.0, 2.0, e - 2.0), "LedgerNotOfThisState"),
        (ledger(0.0, e - 1.0e-3, 1.0e-3), "LedgerNotOfThisState"),
        (ledger(1.0, e - 2.0 * gate_j, 0.0), "LedgerNotOfThisState"),
        (ledger(1.0, e, 2.0 * gate_j), "LedgerNotOfThisState"),
        (ledger(0.0, 0.0, 0.0), "LedgerNotOfThisState"),
    ];
    for (initial, name) in refused {
        let (status, text) = post(&base, "/verify_physics", &step_body(&initial, 0.0)).await;
        assert_eq!(status, 422, "{initial}: {text}");
        assert!(text.contains(name), "{initial}: {text}");
        assert!(!text.contains("run_seal") && !text.contains("\"proof"), "{text}");
        let (status, text) = post(
            &base,
            "/run_trajectory",
            &run_body(initial.clone(), json!([{ "steps": 3, "load": { "accel_m_s2": 0.0 } }]), 1),
        )
        .await;
        assert_eq!(status, 422, "{initial}: {text}");
        assert!(text.contains(name), "{initial}: {text}");
    }
    // A fresh start is answered. A stated ledger inside the card that is not
    // a fresh start's carries a run on: the card takes it, and without from
    // the door refuses it by name, before any physics.
    let (status, text) = post(&base, "/verify_physics", &step_body(&ledger(0.0, e, 0.0), 0.0)).await;
    assert_eq!(status, 200, "{text}");
    let carried = [
        ledger(START_TIME_MAX_S, e, 0.0),
        ledger(1.0, e - 0.5 * gate_j, 0.0),
        ledger(1.0, 0.0, e),
    ];
    for initial in carried {
        let (status, text) = post(&base, "/verify_physics", &step_body(&initial, 0.0)).await;
        assert_eq!(status, 422, "{initial}: {text}");
        assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{initial}: {text}");
    }
    // A request without the ledger is refused.
    let (status, text) = post(
        &base,
        "/verify_physics",
        &json!({
            "domain": "signal",
            "card": "resonance",
            "parameters": { "displacement_m": x, "velocity_m_s": 0.0, "accel_m_s2": 0.0, "dt_s": gate() }
        }),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("t_s"), "{text}");
    // A start above the card is refused for its energy before its ledger is read.
    let over = new_start(0.03, 1.0);
    let (status, text) = post(&base, "/verify_physics", &step_body(&over, 0.0)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("EnergyAboveCard"), "{text}");
}

/// The answer of a continued step is the kernel's step from the kernel's
/// continued state, to the bit.
#[tokio::test]
async fn a_continued_step_is_the_kernel_step_from_the_stated_ledger() {
    let base = spawn_app().await;
    let oscillator = DynamicOscillator::mount_10hz();
    let mut state = OscillatorState::try_phase(&oscillator, 0.004, -0.2).unwrap();
    let mut frame = LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof: [0; 16],
    };
    for _ in 0..50 {
        resonance::step(&oscillator, &mut state, &mut frame, OscillatorLoad { accel_m_s2: 2.0 }, gate()).unwrap();
    }
    let initial = json!({
        "displacement_m": state.displacement_m,
        "velocity_m_s": state.velocity_m_s,
        "t_s": state.t_s,
        "energy_anchor_j": state.energy_anchor_j,
        "boundary_work_j": state.boundary_work_j,
    });
    resonance::step(&oscillator, &mut state, &mut frame, OscillatorLoad { accel_m_s2: 2.0 }, gate()).unwrap();
    // The step carries the 50 steps on: it names their sealed run in from,
    // whose final state is the kernel's to the bit.
    let upstream = run_body(new_start(0.004, -0.2), json!([{ "steps": 50, "load": { "accel_m_s2": 2.0 } }]), 50);
    let (_, up) = post(&base, "/run_trajectory", &upstream).await;
    let up: Value = serde_json::from_str(&up).unwrap();
    let (status, answer) = post(&base, "/verify_physics", &from(step_body(&initial, 2.0), &upstream, &up)).await;
    assert_eq!(status, 200, "{answer}");
    let answer: Value = serde_json::from_str(&answer).unwrap();
    for (name, value) in [
        ("t_s", state.t_s),
        ("displacement_m", state.displacement_m),
        ("velocity_m_s", state.velocity_m_s),
        ("energy_anchor_j", state.energy_anchor_j),
        ("boundary_work_j", state.boundary_work_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
    ] {
        assert_eq!(published(&answer["state"], name).to_bits(), value.to_bits(), "{name}");
    }
}

/// What an agent reads: the three ledger fields, the two new refusals, and
/// sentences that say the ledger runs across continued requests.
#[tokio::test]
async fn the_description_says_the_ledger_runs_across_continued_requests() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_card", &json!({ "domain": "signal", "card": "resonance" })).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let names: Vec<&str> = body["phase_fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec!["displacement_m", "velocity_m_s", "t_s", "energy_anchor_j", "boundary_work_j"]
    );
    let rejects: Vec<&str> = body["rejects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reject| reject["name"].as_str().unwrap())
        .collect();
    assert!(rejects.contains(&"StartTimeOutsideCard"));
    assert!(rejects.contains(&"LedgerNotOfThisState"));
    let ceiling = body["dt_s"]["ceiling_from"].as_str().unwrap();
    assert!(ceiling.contains("The reach counts across continued requests"), "{ceiling}");
    assert!(ceiling.contains("175,815 under one of +/-4.8 m/s^2, which rings it at 0.99 of the stroke"), "{ceiling}");
    assert!(!ceiling.contains("at the full stroke"), "{ceiling}");
    let all = text.as_str();
    assert!(all.contains("The engine keeps nothing between requests"));
    assert!(all.contains("a request at t_s 0 with no booked work is a new start with a full limit"));
    assert!(!all.contains("Time since the anchor"));
    assert_eq!(body["example_compute_step"]["parameters"]["t_s"], json!(0.0));
    assert_eq!(body["example_compute_step"]["parameters"]["energy_anchor_j"], json!(0.0));
    assert_eq!(body["example_compute_step"]["parameters"]["boundary_work_j"], json!(0.0));
}
