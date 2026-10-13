//! The EVE 235/55 R19 tyre-on-ice card on the web and the local connection.
//!
//! A step and four holds of one slip each, chained by restarts, end on the
//! compiled kernel's state to the bit. A run that would hold its slip beyond
//! 21 s, change its slip, or roll beyond the track is refused part way and
//! keeps its proof up to the last step that landed. A restart from a returned
//! distance lands where small steps land. Each refusal a request can reach is
//! named and carries no proof: a load, a pressure, a slip, a speed, a start
//! or a time held outside its range, a slip inside the band around zero, a
//! step outside the card's floor and ceiling, a missing input, a key the card
//! does not have. The flag above the fitted loads is set and the run still
//! verifies. `validate_series` compares the longitudinal force. The local and
//! the web connection return the same bytes.
//!
//! No measurement of the tyre is read here.

use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::tire_eve_r19_ice::{
    self, FLAG_LOAD_ABOVE_FITTED, SLIP_SETTLE_S, StepReject, TireEveR19Ice, TireLoad, TireState,
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
    assert!(answer["sha256_seal"].as_str().is_some_and(|seal| seal.len() == 64));
    assert!(answer["receipt_sha256"].as_str().is_some_and(|seal| seal.len() == 64));
    assert_eq!(answer["engine_version"], "genesis_kernel_v0.9.0");
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

    // Five steps of 4 s from 5 s would hold the slip to 25 s. The fifth is
    // refused and the run keeps its proof up to the fourth.
    let refused_at = |segments: Value, dt_s: f64, start_m: f64| {
        let base = base.clone();
        async move {
            let (status, text) = post(&base, "/run_trajectory", &run_body(start_m, dt_s, segments)).await;
            assert_eq!(status, 200, "{text}");
            let tape: Value = serde_json::from_str(&text).unwrap();
            // The end of the track is the end the card declares: a run is
            // ended there; every other limit refuses.
            let ended = tape["reject"] == "TravelBeyondTrack";
            assert_eq!(tape["outcome"], if ended { "ended" } else { "refused" }, "{text}");
            assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 64));
            let (status, report) = post(&base, "/verify_tape", &tape).await;
            assert_eq!(status, 200, "{report}");
            assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
            tape
        }
    };
    let five = json!([{ "steps": 5, "load": load(5_000.0, 250_000.0, 0.10, 0.06) }]);
    let tape = refused_at(five, 4.0, 0.0).await;
    assert_eq!(tape["reject"], format!("{:?}", StepReject::HoldBeyondMeasured));
    assert_eq!(tape["steps_landed"], 4);
    assert!(tape["reject_detail"].as_str().unwrap().contains("HoldBeyondMeasured"));
    assert_eq!(tape["final_state"]["load_above_fitted"], false);

    // A change of slip in a later segment starts the hold again at zero: the
    // first step of the new slip is refused.
    let changed = json!([
        { "steps": 2, "load": load(5_000.0, 250_000.0, 0.10, 0.06) },
        { "steps": 2, "load": load(5_000.0, 250_000.0, 0.12, 0.06) }
    ]);
    let tape = refused_at(changed, 4.0, 0.0).await;
    assert_eq!(tape["reject"], format!("{:?}", StepReject::SlipNotSettled));
    assert_eq!(tape["steps_landed"], 2);

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

    // A hold that would roll past 6 m is ended there, at the end of the track
    // the card declares.
    let past = json!([{ "steps": 4, "load": load(5_000.0, 250_000.0, 0.10, 0.06) }]);
    let tape = refused_at(past, 4.0, 5.5).await;
    assert_eq!(tape["outcome"], "ended");
    assert_eq!(tape["reject"], format!("{:?}", StepReject::TravelBeyondTrack));
    assert_eq!(tape["steps_landed"], 2);
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
    // a step that truly ends beyond 6 m is still refused by name
    let (status, text) =
        post(&base, "/verify_physics", &step_body(distance_m + 1.0e-9, &held, 16.0)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("TravelBeyondTrack") && !text.contains("sha256_seal"), "{text}");
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
        (step_body(0.0, &with("normal_load_n", 3_499.0), 1.0), "NormalLoadOutsideCard"),
        (step_body(0.0, &with("normal_load_n", 6_501.0), 1.0), "NormalLoadOutsideCard"),
        (step_body(0.0, &with("inflation_pressure_pa", 149_000.0), 1.0), "InflationPressureOutsideCard"),
        (step_body(0.0, &with("inflation_pressure_pa", 351_000.0), 1.0), "InflationPressureOutsideCard"),
        (step_body(0.0, &with("slip_ratio", 0.21), 1.0), "SlipOutsideCard"),
        (step_body(0.0, &with("slip_ratio", -0.21), 1.0), "SlipOutsideCard"),
        // Inside the published minimum and maximum, and inside the band the constants do not cover.
        (step_body(0.0, &with("slip_ratio", 0.03), 1.0), "SlipOutsideCard"),
        (step_body(0.0, &with("slip_ratio", -0.049), 1.0), "SlipOutsideCard"),
        (step_body(0.0, &with("slip_ratio", 0.0), 1.0), "SlipOutsideCard"),
        (step_body(0.0, &with("carriage_speed_m_s", 0.04), 1.0), "CarriageSpeedOutsideCard"),
        (step_body(0.0, &with("carriage_speed_m_s", 13.9), 1.0), "CarriageSpeedOutsideCard"),
        (step_body(-0.1, &good, 1.0), "TravelBeyondTrack"),
        (step_body(6.1, &good, 1.0), "TravelBeyondTrack"),
        // A start on the track whose first step ends beyond it.
        (step_body(5.5, &good, 16.0), "TravelBeyondTrack"),
        // A slip held less than 5 s, or more than 21 s, at the start.
        (step_held(0.0, 0.0, &good, 1.0), "SlipNotSettled"),
        (step_held(0.0, 4.999, &good, 1.0), "SlipNotSettled"),
        (step_held(0.0, 21.001, &good, 1.0), "HoldBeyondMeasured"),
        // A start inside the hold whose first step ends beyond 21 s.
        (step_held(0.0, 20.0, &good, 1.5), "HoldBeyondMeasured"),
        (step_held(0.0, 21.0, &good, 1.0), "HoldBeyondMeasured"),
        (step_body(0.0, &good, 16.5), "dt_s"),
        (step_body(0.0, &good, 21.0), "dt_s"),
        (step_body(0.0, &good, floor / 2.0), "dt_s"),
        (step_body(0.0, &good, 0.0), "dt_s"),
    ];
    for (body, name) in &cases {
        let (status, text) = post(&base, "/verify_physics", body).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.contains(name), "{name}: {text}");
        assert!(!text.contains("sha256_seal"), "{name}: {text}");
    }
    // The ends of each range land.
    for body in [
        step_body(0.0, &with("normal_load_n", 3_500.0), 1.0),
        step_body(0.0, &with("normal_load_n", 6_500.0), 1.0),
        step_body(0.0, &with("slip_ratio", 0.05), 1.0),
        step_body(0.0, &with("slip_ratio", -0.20), 1.0),
        step_body(0.0, &with("carriage_speed_m_s", 0.07), 1.0),
        step_body(6.0 - 0.06 * 16.0 - 1.0e-9, &good, 16.0),
        step_body(0.0, &good, floor),
        step_held(0.0, 20.0, &good, 1.0),
        step_held(0.0, 21.0 - floor, &good, floor),
    ] {
        let (status, text) = post(&base, "/verify_physics", &body).await;
        assert_eq!(status, 200, "{body}: {text}");
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
    assert!(text.contains("slip_held_s") && !text.contains("sha256_seal"), "{text}");
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
        assert!(text.contains(key) && !text.contains("sha256_seal"), "{key}: {text}");
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
        let line = json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":name,"arguments":arguments}});
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
    // The line an agent sees in the list says that the ratio is fitted.
    assert!(entry["specimen"].as_str().unwrap().contains("fitted to this tyre's own measurements"));

    let which = json!({ "domain": "materials", "card": "tire_eve_r19_ice" });
    let (described, error) = local.tool("describe_card", &which).await;
    assert!(!error);
    let (status, over_web) = post(&web, "/describe_card", &which).await;
    assert_eq!(status, 200);
    assert_eq!(described, over_web);
    let description: Value = serde_json::from_str(&described).unwrap();
    assert!(description["validation"]["result"].as_str().unwrap().starts_with(
        "Ninety runs were held back. On 2026-10-09 the card was compared with the 60 of them that hold a scored slip, as 60 cells of three repeats each, and all five lines fixed beforehand were met"
    ));
    // Sentences held in the result and the limits.
    let result = description["validation"]["result"].as_str().unwrap();
    for words in [
        "One held-back run, number 153, is a faulty record: it rolled for about 90 s like the others, and its channels are on other scales",
        "vertical force reads 1,849 N at a nominal 3,500 N",
        "wheel angle spans 0 to 11.75 where every other file spans 0 to 360",
        "without it the three errors are 0.031, 0.034 and 0.029, and those are the figures that describe the tyre",
        "The card read high on all six conditions at 6.5 kN, by 0.023 on average",
        "SHA-256 7a2f3cc8e75aadbbdaf4f74289803ddf1149dbe882d94cadc96f668061b943c7) was overwritten by a later build",
        "which returned all 108 saved answers byte for byte. Since engine 0.7.0 a request states slip_held_s, so those saved requests are refused as they were sent.",
        "evidence/checks/second_party/V-33/",
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
        "it is not a figure for a tyre on ice at a stated temperature or at road speed",
        "the exponent fitted to them alone is -0.107 ± 0.014, where the card holds -0.133",
        "worth 0.002 to 0.003 of the 0.03 error",
        "about -62 N with a standard deviation of 49 N, is not subtracted from the measured forces",
        "it can catch a corrupted step, and it cannot catch a wrong constant, a wrong exponent or a wrong ice",
        "A run whose end in decimal is exactly the track's 6 m lands every step",
        "1,000 steps of 0.1 s at 0.06 m/s, rolled as runs of 16 s each started from the distance the last returned, land 1,000 and read 6.000000000000005 m",
        "with the change placed where the wheel speed steps, every one-second window from 5 s to 20 s after a change met all five lines, and every one before 5 s failed one",
        "Engine 0.7.0 changed every answer of this card",
        "It was scored twice, the second time by a second scorer, on one machine",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    assert_eq!(description["rejects"].as_array().map(Vec::len), Some(13), "{}", description["rejects"]);
    // The constant comparator is named for what it is.
    assert!(result.contains("geometric mean with no slip term (least squares on the logarithm) gave 0.036"), "{result}");

    // The worked examples of the description run as written, the same bytes on both.
    let step = description["example_verify_step"].clone();
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
