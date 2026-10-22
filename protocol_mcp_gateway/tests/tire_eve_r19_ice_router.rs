//! The EVE 235/55 R19 tyre-on-ice card on the web and the local connection.
//!
//! A step and four holds of one slip each, chained by restarts, end on the
//! compiled kernel's state to the bit. A run that holds its slip beyond 21 s
//! or rolls beyond the rig's track lands, its frames past the compared hold
//! flagged; a run that changes its slip is refused part way and keeps its
//! proof up to the last step that landed. A restart from a returned distance
//! lands where small steps land. Each refusal a request can reach is named
//! and carries no proof: no contact, a flat tyre, a slip or a speed outside
//! its range, a distance below zero, a start held under 5 s, a time held the
//! step cannot move, a slip inside the band around zero, a step outside the
//! card's floor and ceiling, a missing input, a key the card does not have.
//! The flags above the fitted loads and beyond the compared range are set
//! and the run still verifies. `validate_series` compares the longitudinal force. The local and
//! the web connection return the same bytes.
//!
//! No measurement of the tyre is read here.

mod answer_text;

use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::tire_eve_r19_ice::{
    self, FLAG_BEYOND_COMPARED, FLAG_LOAD_ABOVE_FITTED, FLAG_SLIP_SETTLING, SLIP_SETTLE_S, StepReject, TireEveR19Ice,
    TireLoad, TireState,
};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

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
    let client = Client::builder().timeout(Duration::from_secs(120)).build().unwrap();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client.post(&url).header(reqwest::header::CONTENT_TYPE, "application/json").body(answer_text::as_sent(body)).send().await {
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

fn tire() -> TireEveR19Ice {
    TireEveR19Ice::on_rig_ice()
}

fn load(normal_load_n: f64, inflation_pressure_pa: f64, slip_ratio: f64, speed: f64) -> Value {
    json!({
        "normal_load_n": normal_load_n,
        "inflation_pressure_pa": inflation_pressure_pa,
        "slip_ratio": slip_ratio,
        "carriage_speed_m_s": speed
    })
}

fn step_held(distance_m: f64, slip_held_s: f64, load: &Value, dt_s: f64) -> Value {
    let mut parameters = load.as_object().unwrap().clone();
    parameters.insert("distance_m".into(), json!(distance_m));
    parameters.insert("slip_held_s".into(), json!(slip_held_s));
    parameters.insert("dt_s".into(), json!(dt_s));
    json!({ "domain": "materials", "card": "tire_eve_r19_ice", "parameters": parameters })
}

/// A step whose slip has been held 5 s, the least the card stands behind.
fn step_body(distance_m: f64, load: &Value, dt_s: f64) -> Value {
    step_held(distance_m, SLIP_SETTLE_S, load, dt_s)
}

fn run_held(distance_m: f64, slip_held_s: f64, dt_s: f64, segments: Value) -> Value {
    json!({
        "domain": "materials", "card": "tire_eve_r19_ice",
        "initial": { "distance_m": distance_m, "slip_held_s": slip_held_s }, "dt_s": dt_s,
        "segments": segments, "record_every": 1
    })
}

fn run_body(distance_m: f64, dt_s: f64, segments: Value) -> Value {
    run_held(distance_m, SLIP_SETTLE_S, dt_s, segments)
}

fn start(distance_m: f64) -> TireState {
    TireState::try_start(&tire(), distance_m, SLIP_SETTLE_S).unwrap()
}

fn blank_frame() -> LastStateFrame64 {
    LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof: [0; 16],
    }
}

fn kernel_load(value: &Value) -> TireLoad {
    TireLoad {
        normal_load_n: value["normal_load_n"].as_f64().unwrap(),
        inflation_pressure_pa: value["inflation_pressure_pa"].as_f64().unwrap(),
        slip_ratio: value["slip_ratio"].as_f64().unwrap(),
        carriage_speed_m_s: value["carriage_speed_m_s"].as_f64().unwrap(),
    }
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// Every number of the published state is the kernel state's, to the bit.
fn assert_state_is(published: &Value, state: &TireState, label: &str) {
    for (name, value) in [
        ("t_s", state.t_s),
        ("distance_m", state.distance_m),
        ("rim_travel_m", state.rim_travel_m),
        ("slip_ratio", state.slip_ratio),
        ("slip_held_s", state.slip_held_s),
        ("longitudinal_force_n", state.longitudinal_force_n),
        ("wheel_work_j", state.wheel_work_j),
        ("drawbar_work_j", state.drawbar_work_j),
        ("slip_heat_j", state.slip_heat_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
    ] {
        assert_eq!(published[format!("{name}_hex")], hex_of(value), "{label} {name}");
    }
    assert_eq!(published["load_above_fitted"], state.load_above_fitted, "{label}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_tyre_is_the_compiled_step_to_the_bit() {
    let base = spawn_app().await;
    let held = load(5_000.0, 250_000.0, 0.12, 0.06);
    let (status, text) = post(&base, "/verify_physics", &step_body(0.25, &held, 16.0)).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let mut state = start(0.25);
    let mut frame = blank_frame();
    tire_eve_r19_ice::step(&tire(), &mut state, &mut frame, kernel_load(&held), 16.0).unwrap();
    assert_state_is(&answer["state"], &state, "one hold");
    assert_eq!(answer["state"]["slip_held_s"], 21.0);
    // 0.228109 x 1.2^-0.132823 x 5000 N.
    let force = answer["state"]["longitudinal_force_n"].as_f64().unwrap();
    assert!((force - 1_113.26).abs() < 0.01, "{force}");
    assert!(answer["sha384_seal"].as_str().is_some_and(|seal| seal.len() == 96));
    assert!(answer["receipt_sha384"].as_str().is_some_and(|seal| seal.len() == 96));
    assert_eq!(answer["engine_version"], "genesis_kernel_v0.13.1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn four_holds_chained_by_restarts_end_on_the_kernel_state_and_each_limit_ends_a_run() {
    let base = spawn_app().await;
    // Four slips of braking, each its own run of four steps from 5 s to 21 s,
    // each started where the last ended.
    let slips = [0.05, 0.06, 0.07, 0.20];
    let mut distance_m = 0.0_f64;
    let mut state = start(0.0);
    for slip in slips {
        let segments = json!([{ "steps": 4, "load": load(6_500.0, 150_000.0, -slip, 0.06) }]);
        let (status, text) = post(&base, "/run_trajectory", &run_body(distance_m, 4.0, segments)).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
        assert_eq!(tape["steps_landed"], 4);
        state = start(state.distance_m);
        let mut frame = blank_frame();
        let held = TireLoad {
            normal_load_n: 6_500.0,
            inflation_pressure_pa: 150_000.0,
            slip_ratio: -slip,
            carriage_speed_m_s: 0.06,
        };
        for _ in 0..4 {
            tire_eve_r19_ice::step(&tire(), &mut state, &mut frame, held, 4.0).unwrap();
        }
        assert_state_is(&tape["final_state"], &state, "a hold of braking");
        assert_eq!(tape["final_state"]["load_above_fitted"], true);
        assert_eq!(tape["final_state"]["slip_held_s"], 21.0);
        assert!(state.wheel_work_j < 0.0 && state.slip_heat_j > 0.0);
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
        // Every frame carries the flag: 6500 N is above the fitted loads.
        for recorded in tape["frames"].as_array().unwrap() {
            let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
            let flags = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
            assert_eq!(flags, FLAG_LOAD_ABOVE_FITTED);
        }
        // The returned distance reads back as the kernel's to the bit.
        distance_m = tape["final_state"]["distance_m"].as_f64().unwrap();
        assert_eq!(distance_m.to_bits(), state.distance_m.to_bits());
    }

    // Every run here is sealed and its tape holds; a refusal is refused, and
    // no outcome is ended: the card declares no end.
    let refused_at = |segments: Value, dt_s: f64, start_m: f64| {
        let base = base.clone();
        async move {
            let (status, text) = post(&base, "/run_trajectory", &run_body(start_m, dt_s, segments)).await;
            assert_eq!(status, 200, "{text}");
            let tape: Value = serde_json::from_str(&text).unwrap();
            assert_ne!(tape["outcome"], "ended", "{text}");
            assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 96));
            let (status, report) = post(&base, "/verify_tape", &tape).await;
            assert_eq!(status, 200, "{report}");
            assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
            tape
        }
    };
    // Five steps of 4 s from 5 s hold the slip to 25 s. Through engine 0.9 the
    // fifth was refused as HoldBeyondMeasured; it lands, and its frame alone
    // carries FLAG_BEYOND_COMPARED, the hold past the 21 s compared.
    let five = json!([{ "steps": 5, "load": load(5_000.0, 250_000.0, 0.10, 0.06) }]);
    let tape = refused_at(five, 4.0, 0.0).await;
    assert_eq!(tape["outcome"], "completed", "{tape}");
    assert_eq!(tape["steps_landed"], 5);
    assert_eq!(tape["final_state"]["slip_held_s"], 25.0);
    assert_eq!(tape["final_state"]["beyond_compared"], true);
    let flags: Vec<u64> = tape["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|recorded| {
            let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
            u64::from_le_bytes(bytes[40..48].try_into().unwrap())
        })
        .collect();
    assert_eq!(flags.last(), Some(&FLAG_BEYOND_COMPARED));
    assert!(flags[..flags.len() - 1].iter().all(|flags| *flags == 0), "{flags:?}");

    // A change of slip in a later segment starts the hold again at zero: the
    // first two steps of the new slip, which start at 0 s and 4 s held, land
    // with FLAG_SLIP_SETTLING, the steady force of the new slip; through
    // engine 0.11 the first was refused as SlipNotSettled. The third, from
    // 8 s held, carries no flag.
    let changed = json!([
        { "steps": 2, "load": load(5_000.0, 250_000.0, 0.10, 0.06) },
        { "steps": 3, "load": load(5_000.0, 250_000.0, 0.12, 0.06) }
    ]);
    let tape = refused_at(changed, 4.0, 0.0).await;
    assert_eq!(tape["outcome"], "completed", "{tape}");
    assert_eq!(tape["steps_landed"], 5);
    assert_eq!(tape["final_state"]["slip_held_s"], 12.0);
    let flags: Vec<u64> = tape["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|recorded| {
            let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
            u64::from_le_bytes(bytes[40..48].try_into().unwrap())
        })
        .collect();
    assert_eq!(flags, [0, 0, FLAG_SLIP_SETTLING, FLAG_SLIP_SETTLING, 0]);
    let card = tire();
    assert_eq!(
        tape["final_state"]["longitudinal_force_n"].as_f64().unwrap().to_bits(),
        card.longitudinal_force_n(5_000.0, 0.12).to_bits()
    );
    // A run from the instant its slip is set, 0 s held, lands every step
    // through its first 5 s, flagged, and its ledger closes: bit for bit the
    // kernel's loop.
    let from_zero = json!([{ "steps": 40, "load": load(6_500.0, 250_000.0, -0.07, 0.06) }]);
    let (status, text) = post(&base, "/run_trajectory", &run_held(0.0, 0.0, 0.25, from_zero)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{text}");
    let mut state = tire_eve_r19_ice::TireState::try_start(&card, 0.0, 0.0).unwrap();
    let mut frame = blank_frame();
    let held = tire_eve_r19_ice::TireLoad {
        normal_load_n: 6_500.0,
        inflation_pressure_pa: 250_000.0,
        slip_ratio: -0.07,
        carriage_speed_m_s: 0.06,
    };
    for (index, recorded) in tape["frames"].as_array().unwrap().iter().enumerate() {
        let before = state.slip_held_s;
        tire_eve_r19_ice::step(&card, &mut state, &mut frame, held, 0.25).unwrap();
        let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
        let flags = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
        assert_eq!(flags & FLAG_SLIP_SETTLING != 0, before < SLIP_SETTLE_S, "frame {index}");
        assert_eq!(flags, frame.flags, "frame {index}");
    }
    assert_eq!(tape["final_state"]["slip_held_s"], 10.0);
    assert!(tape["max_residual_conservation_error"].as_f64().unwrap() < 1.0e-13);

    // A change of load at the same slip keeps the hold.
    let same_slip = json!([
        { "steps": 2, "load": load(5_000.0, 250_000.0, 0.10, 0.06) },
        { "steps": 2, "load": load(6_000.0, 300_000.0, 0.10, 0.05) }
    ]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, 4.0, same_slip)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{text}");
    assert_eq!(tape["final_state"]["slip_held_s"], 21.0);

    // A hold that rolls past 6 m, the end of the rig's track: through engine
    // 0.9 it ended there, after two steps. The end of the track is not an
    // end of the tyre, and the run completes, the same force at every step.
    let past = json!([{ "steps": 4, "load": load(5_000.0, 250_000.0, 0.10, 0.06) }]);
    let tape = refused_at(past, 4.0, 5.5).await;
    assert_eq!(tape["outcome"], "completed", "{tape}");
    assert_eq!(tape["steps_landed"], 4);
    let mut state = tire_eve_r19_ice::TireState::try_start(&tire(), 5.5, 5.0).unwrap();
    let mut frame = blank_frame();
    for _ in 0..4 {
        tire_eve_r19_ice::step(&tire(), &mut state, &mut frame, kernel_load(&load(5_000.0, 250_000.0, 0.10, 0.06)), 4.0).unwrap();
    }
    assert_state_is(&tape["final_state"], &state, "a hold past the end of the track");
    assert!(state.distance_m > 6.0);
    let declared = &tape["declared_ends"];
    assert!(declared.is_null() || declared.as_array().is_some_and(|ends| ends.is_empty()), "{declared}");
    let _ = StepReject::DistanceBelowZero;
}

/// On 0.6.0 a distance returned by small steps, handed back as a new start,
/// was refused on one step to the end of the track that small steps from the
/// same start reached. Both now land.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_restart_from_a_returned_distance_lands_where_small_steps_land() {
    let base = spawn_app().await;
    let held = load(5_000.0, 250_000.0, 0.12, 0.06);
    let mut distance_m = 0.0_f64;
    for steps in [1_600, 1_600, 1_600, 1_600, 1_600, 400] {
        let segments = json!([{ "steps": steps, "load": held }]);
        let mut body = run_body(distance_m, 0.01, segments);
        body["record_every"] = json!(steps);
        let (status, text) = post(&base, "/run_trajectory", &body).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{text}");
        distance_m = tape["final_state"]["distance_m"].as_f64().unwrap();
    }
    assert_eq!(distance_m, 5.040000000000356);
    // one step of 16 s to the end
    let (status, text) = post(&base, "/verify_physics", &step_body(distance_m, &held, 16.0)).await;
    assert_eq!(status, 200, "{text}");
    let one: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(one["state"]["distance_m"], 6.000000000000356);
    // 1,600 steps of 0.01 s over the same 16 s
    let mut body = run_body(distance_m, 0.01, json!([{ "steps": 1_600, "load": held }]));
    body["record_every"] = json!(1_600);
    let (status, text) = post(&base, "/run_trajectory", &body).await;
    assert_eq!(status, 200, "{text}");
    let many: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(many["outcome"], "completed", "{text}");
    assert_eq!(many["final_state"]["distance_m"], 6.000000000000961);
    assert_eq!(one["state"]["longitudinal_force_n"], many["final_state"]["longitudinal_force_n"]);
    // A step that ends beyond 6 m lands too: through engine 0.9 it was
    // refused as TravelBeyondTrack.
    let (status, text) =
        post(&base, "/verify_physics", &step_body(distance_m + 1.0e-9, &held, 16.0)).await;
    assert_eq!(status, 200, "{text}");
    assert!(text.contains("sha384_seal"), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_limit_a_request_can_reach_is_named_and_carries_no_proof() {
    let base = spawn_app().await;
    let good = load(5_000.0, 250_000.0, 0.10, 0.06);
    let with = |key: &str, value: f64| {
        let mut changed = good.clone();
        changed[key] = json!(value);
        changed
    };
    let floor = tire().floor_dt_s().unwrap();
    let cases: Vec<(Value, &str)> = vec![
        (step_body(0.0, &with("normal_load_n", 0.0), 1.0), "NoContact"),
        (step_body(0.0, &with("normal_load_n", -5_000.0), 1.0), "NoContact"),
        (step_body(0.0, &with("inflation_pressure_pa", 0.0), 1.0), "TyreFlat"),
        (step_body(0.0, &with("inflation_pressure_pa", -10_000.0), 1.0), "TyreFlat"),
        (step_body(0.0, &with("slip_ratio", 0.21), 1.0), "SlipOutsideCard"),
        (step_body(0.0, &with("slip_ratio", -0.21), 1.0), "SlipOutsideCard"),
        // Inside the published minimum and maximum, and inside the band the constants do not cover.
        (step_body(0.0, &with("slip_ratio", 0.03), 1.0), "SlipOutsideCard"),
        (step_body(0.0, &with("slip_ratio", -0.049), 1.0), "SlipOutsideCard"),
        (step_body(0.0, &with("slip_ratio", 0.0), 1.0), "SlipOutsideCard"),
        (step_body(0.0, &with("carriage_speed_m_s", 0.04), 1.0), "CarriageSpeedOutsideCard"),
        (step_body(0.0, &with("carriage_speed_m_s", 13.9), 1.0), "CarriageSpeedOutsideCard"),
        (step_body(-0.1, &good, 1.0), "DistanceBelowZero"),
        // A time held below zero.
        (step_held(0.0, -0.001, &good, 1.0), "SlipHeldBelowZero"),
        (step_held(0.0, -5.0, &good, 1.0), "SlipHeldBelowZero"),
        // A time held the step cannot move.
        (step_held(0.0, 1.152_921_504_606_847e18, &good, 1.0), "StepTooSmallToResolve"),
        (step_body(0.0, &good, 4_194_305.0), "dt_s"),
        (step_body(0.0, &good, floor / 2.0), "dt_s"),
        (step_body(0.0, &good, 0.0), "dt_s"),
    ];
    for (body, name) in &cases {
        let (status, text) = post(&base, "/verify_physics", body).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.contains(name), "{name}: {text}");
        assert!(!text.contains("sha384_seal"), "{name}: {text}");
    }
    // The ends of each range land, and what lay past the compared range
    // through engine 0.9 lands now, flagged.
    for (body, beyond) in [
        (step_body(0.0, &with("normal_load_n", 3_500.0), 1.0), false),
        (step_body(0.0, &with("normal_load_n", 6_500.0), 1.0), false),
        (step_body(0.0, &with("normal_load_n", 3_499.0), 1.0), true),
        (step_body(0.0, &with("normal_load_n", 6_501.0), 1.0), true),
        (step_body(0.0, &with("normal_load_n", 1.0), 1.0), true),
        (step_body(0.0, &with("normal_load_n", 1.0e6), 1.0), true),
        (step_body(0.0, &with("inflation_pressure_pa", 149_000.0), 1.0), true),
        (step_body(0.0, &with("inflation_pressure_pa", 351_000.0), 1.0), true),
        (step_body(0.0, &with("slip_ratio", 0.05), 1.0), false),
        (step_body(0.0, &with("slip_ratio", -0.20), 1.0), false),
        (step_body(0.0, &with("carriage_speed_m_s", 0.07), 1.0), false),
        (step_body(6.1, &good, 1.0), false),
        (step_body(5.5, &good, 16.0), false),
        (step_body(0.0, &good, floor), false),
        (step_body(0.0, &good, 16.5), true),
        (step_body(0.0, &good, 4_194_304.0), true),
        (step_held(0.0, 20.0, &good, 1.0), false),
        (step_held(0.0, 20.0, &good, 1.5), true),
        (step_held(0.0, 21.001, &good, 1.0), true),
        (step_held(0.0, 21.0 - floor, &good, floor), false),
    ] {
        let (status, text) = post(&base, "/verify_physics", &body).await;
        assert_eq!(status, 200, "{body}: {text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(answer["state"]["beyond_compared"], beyond, "{body}: {text}");
    }
    // A missing input, a constant of the card and an unknown key are refused.
    let mut missing = step_body(0.0, &good, 1.0);
    missing["parameters"].as_object_mut().unwrap().remove("inflation_pressure_pa");
    let (status, text) = post(&base, "/verify_physics", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("inflation_pressure_pa"), "{text}");
    // The time held is a continuation input: a start without it is refused.
    let mut missing = step_body(0.0, &good, 1.0);
    missing["parameters"].as_object_mut().unwrap().remove("slip_held_s");
    let (status, text) = post(&base, "/verify_physics", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("slip_held_s") && !text.contains("sha384_seal"), "{text}");
    let mut missing = run_body(0.0, 1.0, json!([{ "steps": 1, "load": good }]));
    missing["initial"].as_object_mut().unwrap().remove("slip_held_s");
    let (status, text) = post(&base, "/run_trajectory", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("slip_held_s") && !text.contains("run_seal"), "{text}");
    for key in ["mu_traction", "slip_exponent", "ice_temperature_k"] {
        let mut extra = step_body(0.0, &good, 1.0);
        extra["parameters"][key] = json!(0.9);
        let (status, text) = post(&base, "/verify_physics", &extra).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(key) && !text.contains("sha384_seal"), "{key}: {text}");
    }
    // A slip off the card in a later segment refuses the whole run before its first step.
    let segments = json!([
        { "steps": 1, "load": good },
        { "steps": 1, "load": with("slip_ratio", 0.01) }
    ]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, 1.0, segments)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("SlipOutsideCard") && !text.contains("run_seal"), "{text}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_the_force_and_carries_the_run_seal() {
    let base = spawn_app().await;
    let segments = json!([{ "steps": 4, "load": load(5_000.0, 250_000.0, 0.12, 0.06) }]);
    let run = run_body(0.0, 4.0, segments);
    let (status, alone) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{alone}");
    let alone: Value = serde_json::from_str(&alone).unwrap();
    let force = alone["final_state"]["longitudinal_force_n"].as_f64().unwrap();
    let body = json!({
        "run": run, "quantity": "longitudinal_force_n",
        "measurements": [{ "step": 1, "value": force + 10.0 }, { "step": 4, "value": force - 10.0 }]
    });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert!(text.contains("10"), "{text}");
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

    async fn tool(&mut self, name: &str, arguments: &Value) -> (String, bool) {
        let line = answer_text::tool_call(7, name, arguments);
        let answer: Value = serde_json::from_str(&self.raw(&line.to_string()).await).unwrap();
        (
            answer["result"]["content"][0]["text"].as_str().unwrap().to_string(),
            answer["result"]["isError"] == true,
        )
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_local_connection_lists_the_tyre_and_gives_the_bytes_of_the_web_connection() {
    let web = spawn_app().await;
    let mut local = Local::start().await;
    let (listed, error) = local.tool("list_cards", &json!({})).await;
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let entry = listed["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["card"] == "tire_eve_r19_ice")
        .expect("the tyre is listed");
    assert_eq!(entry["domain"], "materials");
    // Since 0.14.4 a row gives what the model computes, the comparison and the gap; the whole spec sheet is the description's.
    assert!(!entry["specimen"].as_str().unwrap().contains("Fitted on"));

    let which = json!({ "domain": "materials", "card": "tire_eve_r19_ice" });
    let (described, error) = local.tool("describe_card", &which).await;
    assert!(!error);
    assert!(described.contains("Fitted on 72 of the 252 runs of the EVE ice dataset"), "{described}");
    let (status, over_web) = post(&web, "/describe_card", &which).await;
    assert_eq!(status, 200);
    assert_eq!(described, over_web);
    let description: Value = serde_json::from_str(&described).unwrap();
    assert!(description["validation"]["result"].as_str().unwrap().starts_with(
        "On the 60 of the 90 runs outside the fit that hold a slip of 5 to 20 percent, as 60 cells of three repeats each"
    ));
    // Sentences held in the result and the limits.
    let result = description["validation"]["result"].as_str().unwrap();
    for words in [
        "One run outside the fit, number 153, is a faulty record: it rolled for about 90 s like the others, and its channels are on other scales",
        "vertical force reads 1,849 N at a nominal 3,500 N",
        "wheel angle spans 0 to 11.75 where every other file spans 0 to 360",
        "without it the three errors are 0.031, 0.034 and 0.029",
        "The card reads high on all six conditions at 6.5 kN, by 0.023 on average",
        "0.034 over the 60 cells (target 0.045), 0.040 over the 24 cells at 2.5 bar (target 0.045) and 0.029 over the 36 cells at 6.5 kN (target 0.045)",
        "binary 7a2f3cc8e75aadbbdaf4f74289803ddf1149dbe882d94cadc96f668061b943c7; a later build returns all 108 saved answers byte for byte. A request now states slip_held_s, so those saved requests are refused as they were sent.",
        "A second comparison program, on the same machine and on its own reduction of the raw files, gives the same results",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    assert!(!result.contains("half the time"), "{result}");
    let limits = description["validation"]["limits"].as_str().unwrap();
    for words in [
        "The error follows the test block, not the load or the pressure",
        "run from 0.185 to 0.287, and the standard deviation between them is 0.029, against 0.005 to 0.011 between repeats",
        "the ratio rose by 0.010 to 0.024 from the first repeat to the third",
        "Any model that does not know the block lands near 0.03, and the card is at that floor",
        "A user should carry about ±0.03 (one standard deviation) on the ratio for the session",
        "not a figure for a tyre on ice at a stated temperature or at road speed",
        "the exponent fitted to them alone is -0.107 ± 0.014, where the card holds -0.133",
        "worth 0.002 to 0.003 of the 0.03 error",
        "about -62 N with a standard deviation of 49 N, is not subtracted from the measured forces",
        "it can catch a corrupted step, and it cannot catch a wrong constant, a wrong exponent or a wrong ice",
        "compared at 6.5 kN with a root-mean-square error of 0.029 (target 0.045)",
        "with the change placed where the wheel speed steps: every one-second window from 5 s to 20 s after a change is inside all five targets, and every one before 5 s is outside one",
        "the first second gives 0.0477 at 6.5 kN (target 0.045)",
        "a second comparison program, on the same machine, gives the same results",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    // Re-pinned for 0.10.1: after the kernel's 13 names the list holds the gateway's own two, DtBelowFloor and DtAboveCeiling.
    assert_eq!(description["rejects"].as_array().map(Vec::len), Some(15), "{}", description["rejects"]);
    // The constant comparator is named for what it is.
    assert!(result.contains("geometric mean with no slip term (least squares on the logarithm) gives 0.036"), "{result}");

    // The worked examples of the description run as written, the same bytes on both.
    let step = description["example_compute_step"].clone();
    let (over_local, error) = local.tool("verify_step", &step).await;
    assert!(!error, "{over_local}");
    let (status, over_web) = post(&web, "/verify_physics", &step).await;
    assert_eq!(status, 200, "{over_web}");
    assert_eq!(over_local, over_web);
    let run = description["example_run_trajectory"].clone();
    let (over_local, error) = local.tool("run_trajectory", &run).await;
    assert!(!error, "{over_local}");
    let (status, over_web) = post(&web, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{over_web}");
    assert_eq!(over_local, over_web);
    let tape: Value = serde_json::from_str(&over_local).unwrap();
    assert_eq!(tape["outcome"], "completed");
    assert_eq!(tape["steps_landed"], 4);

    // A refusal is the same sentence on both, marked as an error on the local connection.
    let mut off = step.clone();
    off["parameters"]["slip_ratio"] = json!(0.02);
    let (over_local, error) = local.tool("verify_step", &off).await;
    assert!(error);
    let (status, over_web) = post(&web, "/verify_physics", &off).await;
    assert_eq!(status, 422);
    assert!(over_local.contains("SlipOutsideCard") && over_web.contains("SlipOutsideCard"));
}
