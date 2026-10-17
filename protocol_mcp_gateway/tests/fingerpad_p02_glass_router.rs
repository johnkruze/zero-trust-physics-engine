//! The finger pad of participant P02 twisted on glass, on the web and the
//! local connection.
//!
//! A step and a whole twist end on the compiled kernel's state to the bit. A
//! run that ends exactly on 75 degrees, the end of the twist compared with
//! measurement, carries no flag beyond the compared range; the request that
//! was refused past 75 degrees through engine 0.9 lands to 80, its frames
//! past 75 flagged. Each refusal a request can reach is named and carries no
//! proof: no contact, a rate not above zero, a step past the quadrature step,
//! a start at any twist but zero, a load changed in a later segment, a step
//! outside the card's floor and ceiling, a missing input, a key the card
//! does not have. Every condition the comparison of 2026-10-09 missed lands.
//! The event of full slip and the frame beyond the compared range are the
//! two flags, and the run still verifies. `validate_series` compares the
//! torque. The local and the web connection return the same bytes.
//!
//! No measurement of the finger is read here.

mod answer_text;

use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::fingerpad_p02_glass::{
    self, FLAG_BEYOND_COMPARED, FLAG_FULL_SLIP, FingerPadLoad, FingerPadP02Glass, FingerPadState,
};
use genesis_kernel::frame::LastStateFrame64;
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

fn pad() -> FingerPadP02Glass {
    FingerPadP02Glass::on_glass()
}

fn load(normal_force_n: f64, twist_rate_deg_s: f64) -> Value {
    json!({ "normal_force_n": normal_force_n, "twist_rate_deg_s": twist_rate_deg_s })
}

fn step_body(twist_deg: f64, load: &Value, dt_s: f64) -> Value {
    let mut parameters = load.as_object().unwrap().clone();
    parameters.insert("twist_deg".into(), json!(twist_deg));
    parameters.insert("dt_s".into(), json!(dt_s));
    json!({ "domain": "materials", "card": "fingerpad_p02_glass", "parameters": parameters })
}

fn run_body(twist_deg: f64, dt_s: f64, segments: Value, record_every: u64) -> Value {
    json!({
        "domain": "materials", "card": "fingerpad_p02_glass",
        "initial": { "twist_deg": twist_deg }, "dt_s": dt_s,
        "segments": segments, "record_every": record_every
    })
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

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// The kernel's state after `steps` steps from zero twist.
fn kernel_run(held: FingerPadLoad, dt_s: f64, steps: usize) -> FingerPadState {
    let mut state = FingerPadState::try_start(&pad(), 0.0).unwrap();
    let mut frame = blank_frame();
    for _ in 0..steps {
        fingerpad_p02_glass::step(&pad(), &mut state, &mut frame, held, dt_s).unwrap();
    }
    state
}

/// Every number of the published state is the kernel state's, to the bit.
fn assert_state_is(published: &Value, state: &FingerPadState, label: &str) {
    for (name, value) in [
        ("t_s", state.t_s),
        ("twist_deg", state.twist_deg),
        ("torque_n_m", state.torque_n_m),
        ("slip_torque_limit_n_m", state.slip_torque_limit_n_m),
        ("stuck_fraction", state.stuck_fraction),
        ("plate_work_j", state.plate_work_j),
        ("stored_energy_j", state.stored_energy_j),
        ("friction_heat_j", state.friction_heat_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
        ("held_normal_force_n", state.held_normal_force_n),
        ("held_twist_rate_deg_s", state.held_twist_rate_deg_s),
        ("stored_energy_at_start_j", state.stored_energy_at_start_j),
    ] {
        assert_eq!(published[format!("{name}_hex")], hex_of(value), "{label} {name}");
    }
    assert_eq!(published["full_slip"], state.full_slip, "{label}");
    // The two warnings of the build that was scored are gone from the state.
    assert!(published.get("load_above_fitted").is_none(), "{label}");
    assert!(published.get("rate_above_fitted").is_none(), "{label}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_finger_pad_is_the_compiled_step_to_the_bit() {
    let base = spawn_app().await;
    let held = load(2.0, 20.0);
    let (status, text) = post(&base, "/verify_physics", &step_body(0.0, &held, 0.01)).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let state = kernel_run(FingerPadLoad { normal_force_n: 2.0, twist_rate_deg_s: 20.0 }, 0.01, 1);
    assert_state_is(&answer["state"], &state, "one step");
    // At 2 N and 20 degrees a second the limit is the reference, 10.3527 N mm.
    let limit = answer["state"]["slip_torque_limit_n_m"].as_f64().unwrap();
    assert!((limit - 0.0103527).abs() < 1.0e-12, "{limit}");
    let torque = answer["state"]["torque_n_m"].as_f64().unwrap();
    assert!(torque > 0.0 && torque < 0.1 * limit, "{torque}");
    // The state carries the load the twist is made under.
    assert_eq!(answer["state"]["held_normal_force_n"], 2.0);
    assert_eq!(answer["state"]["held_twist_rate_deg_s"], 20.0);
    assert!(answer["sha256_seal"].as_str().is_some_and(|seal| seal.len() == 64));
    assert!(answer["receipt_sha256"].as_str().is_some_and(|seal| seal.len() == 64));
    assert_eq!(answer["engine_version"], "genesis_kernel_v0.12.0");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_twist_to_75_degrees_keeps_its_last_step_and_the_next_is_refused_by_name() {
    let base = spawn_app().await;
    // The largest force and the fastest rate of the card: 150 steps of 0.01 s
    // end exactly on 75 degrees.
    let segments = json!([{ "steps": 150, "load": load(2.07, 50.0) }]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, 0.01, segments, 1)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 150);
    let held = FingerPadLoad { normal_force_n: 2.07, twist_rate_deg_s: 50.0 };
    let state = kernel_run(held, 0.01, 150);
    // The largest plate work inside the range, as the description gives it.
    assert!((state.plate_work_j - 0.0126).abs() < 5.0e-5, "{}", state.plate_work_j);
    assert_state_is(&tape["final_state"], &state, "the whole twist");
    assert!(state.full_slip);
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
    // No frame carries a flag before the full slip; from it on each carries
    // the event's flag and no other. The event falls at 39 to 40 degrees.
    let frames = tape["frames"].as_array().unwrap();
    let flags_of = |recorded: &Value| {
        let bytes = hex::decode(recorded["frame_hex"].as_str().unwrap()).unwrap();
        u64::from_le_bytes(bytes[40..48].try_into().unwrap())
    };
    assert_eq!(flags_of(&frames[0]), 0);
    assert_eq!(flags_of(&frames[149]), FLAG_FULL_SLIP);
    let first = frames.iter().position(|frame| flags_of(frame) != 0).unwrap();
    assert!(frames[first..].iter().all(|frame| flags_of(frame) == FLAG_FULL_SLIP));
    let event_twist = (first + 1) as f64 * 0.5;
    assert!((38.9..=40.4).contains(&event_twist), "{event_twist}");

    // The exact end at other pairings of rate and step: 20 x 0.01 x 375,
    // 5 x 0.01 x 1500, 50 x 0.005 x 300, 50 x 0.0001 x 15000 and
    // 10 x 0.004 x 1875, at each end of the force band.
    for (force, rate, dt_s, steps) in [
        (2.0, 20.0, 0.01, 375_u64),
        (1.93, 5.0, 0.01, 1500),
        (2.07, 50.0, 0.005, 300),
        (1.93, 50.0, 0.0001, 15000),
        (2.07, 10.0, 0.004, 1875),
    ] {
        let segments = json!([{ "steps": steps, "load": load(force, rate) }]);
        let (status, text) =
            post(&base, "/run_trajectory", &run_body(0.0, dt_s, segments, steps)).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{rate} {dt_s}: {}", tape["reject_detail"]);
        assert_eq!(tape["steps_landed"], steps, "{rate} {dt_s}");
        let twist = tape["final_state"]["twist_deg"].as_f64().unwrap();
        assert!((twist - 75.0).abs() < 4.0e-9, "{twist}");
    }

    // Through engine 0.9 one step more was refused as TwistBeyondMeasured, and
    // the 160 steps that answered to 80 degrees in 0.6.0 were refused at the
    // same step. Both land now; each frame past 75 degrees carries
    // FLAG_BEYOND_COMPARED, and the tape holds.
    for steps in [151_u64, 160] {
        let segments = json!([{ "steps": steps, "load": load(2.07, 50.0) }]);
        let (status, text) =
            post(&base, "/run_trajectory", &run_body(0.0, 0.01, segments, 1)).await;
        assert_eq!(status, 200, "{text}");
        let tape: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{steps}");
        assert_eq!(tape["steps_landed"], steps, "{steps}");
        let frames = tape["frames"].as_array().unwrap();
        assert_eq!(flags_of(&frames[149]), FLAG_FULL_SLIP);
        for frame in &frames[150..] {
            assert_eq!(flags_of(frame), FLAG_FULL_SLIP | FLAG_BEYOND_COMPARED);
        }
        let state = kernel_run(held, 0.01, steps as usize);
        assert_state_is(&tape["final_state"], &state, "past 75 degrees");
        assert_eq!(tape["final_state"]["beyond_compared"], true);
        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{report}");
    }
}

/// The description states the twist range as the twist that was compared,
/// zero to 75 degrees, and gives the largest work inside the range as the
/// engine computes it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_description_states_the_compared_twist_and_the_largest_work() {
    let base = spawn_app().await;
    let which = json!({ "domain": "materials", "card": "fingerpad_p02_glass" });
    let (status, text) = post(&base, "/describe_card", &which).await;
    assert_eq!(status, 200, "{text}");
    for words in [
        "past 75 degrees a run goes on, each frame carrying FLAG_BEYOND_COMPARED",
        "A run goes on past 75 degrees, the end of the twist compared with measurement",
        "the work of a twist to 75 degrees at 2.07 N and 50 degrees a second is 0.0126 J",
        "refused as TwistStepAboveQuadrature",
    ] {
        assert!(text.contains(words), "{words}");
    }
    for words in ["0.015 J", "409,600", "end beyond 80 degrees", "ends exactly on 80 degrees", "TwistBeyondMeasured", "NormalForceOutsideCard"] {
        assert!(!text.contains(words), "{words}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_limit_a_request_can_reach_is_named_and_carries_no_proof() {
    let base = spawn_app().await;
    let good = load(2.0, 20.0);
    let floor = pad().floor_dt_s().unwrap();
    let below = |x: f64| f64::from_bits(x.to_bits() - 1);
    let above = |x: f64| f64::from_bits(x.to_bits() + 1);
    let cases: Vec<(Value, &str)> = vec![
        // A force and a rate of zero or less.
        (step_body(0.0, &load(0.0, 20.0), 0.01), "NoContact"),
        (step_body(0.0, &load(-2.0, 20.0), 0.01), "NoContact"),
        (step_body(0.0, &load(2.0, 0.0), 0.01), "TwistRateNotPositive"),
        (step_body(0.0, &load(2.0, -20.0), 0.01), "TwistRateNotPositive"),
        // A step that turns the plate past 0.05 of the characteristic twist.
        (step_body(0.0, &load(2.0, 100.0), 0.01), "TwistStepAboveQuadrature"),
        (step_body(0.0, &load(2.0, 1000.0), 0.001), "TwistStepAboveQuadrature"),
        // A start at any twist but zero, one that would read as zero in a
        // 32-bit frame slot included.
        (step_body(1.0e-46, &good, 0.01), "TwistNotStartedFromZero"),
        (step_body(1.0e-300, &good, 0.01), "TwistNotStartedFromZero"),
        (step_body(30.0, &good, 0.01), "TwistNotStartedFromZero"),
        (step_body(80.0, &good, 0.01), "TwistNotStartedFromZero"),
        (step_body(80.1, &good, 0.01), "TwistNotStartedFromZero"),
        (step_body(-0.1, &good, 0.01), "TwistNotStartedFromZero"),
        (step_body(0.0, &good, 0.011), "dt_s"),
        (step_body(0.0, &good, floor / 2.0), "dt_s"),
        (step_body(0.0, &good, 0.0), "dt_s"),
    ];
    for (body, name) in &cases {
        let (status, text) = post(&base, "/verify_physics", body).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.contains(name), "{name}: {text}");
        assert!(!text.contains("sha256_seal"), "{name}: {text}");
    }
    // The same of a run: no start but zero, and no proof.
    for twist in [1.0e-46, 30.2] {
        let segments = json!([{ "steps": 1, "load": load(2.0, 20.0) }]);
        let (status, text) = post(&base, "/run_trajectory", &run_body(twist, 0.01, segments, 1)).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains("TwistNotStartedFromZero") && !text.contains("run_seal"), "{text}");
    }
    // Every condition the comparison of 2026-10-09 missed lands, and what lies
    // past the forces and rates measured lands flagged.
    for (body, beyond) in [
        (step_body(0.0, &load(1.93, 20.0), 0.01), false),
        (step_body(0.0, &load(2.07, 20.0), 0.01), false),
        (step_body(0.0, &load(2.0, 5.0), 0.01), false),
        (step_body(0.0, &load(2.0, 50.0), 0.01), false),
        (step_body(0.0, &load(below(1.93), 20.0), 0.01), false),
        (step_body(0.0, &load(above(2.07), 20.0), 0.01), false),
        (step_body(0.0, &load(0.5, 20.0), 0.01), false),
        (step_body(0.0, &load(1.0, 20.0), 0.01), false),
        (step_body(0.0, &load(5.0, 20.0), 0.01), false),
        (step_body(0.0, &load(10.0, 20.0), 0.01), false),
        (step_body(0.0, &load(2.0, 100.0), 0.004), false),
        (step_body(0.0, &load(below(0.5), 20.0), 0.01), true),
        (step_body(0.0, &load(above(10.0), 20.0), 0.01), true),
        (step_body(0.0, &load(2.0, below(5.0)), 0.01), true),
        (step_body(0.0, &load(2.0, above(100.0)), 0.004), true),
        (step_body(0.0, &load(0.01, 20.0), 0.01), true),
        (step_body(0.0, &load(500.0, 20.0), 0.01), true),
        (step_body(0.0, &load(2.0, 0.1), 0.01), true),
        (step_body(0.0, &good, 0.01), false),
        (step_body(0.0, &good, floor), false),
        (step_body(-0.0, &good, 0.01), false),
    ] {
        let (status, text) = post(&base, "/verify_physics", &body).await;
        assert_eq!(status, 200, "{body}: {text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(answer["state"]["beyond_compared"], beyond, "{body}: {text}");
    }
    // A missing input is refused: nothing defaults.
    for key in ["normal_force_n", "twist_rate_deg_s", "twist_deg", "dt_s"] {
        let mut missing = step_body(0.0, &good, 0.01);
        missing["parameters"].as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/verify_physics", &missing).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains(key), "{text}");
    }
    // A constant of the card, the held load of a state and an unknown key are refused.
    for key in ["load_exponent", "slip_torque_reference_n_m", "friction_coefficient", "held_normal_force_n"] {
        let mut extra = step_body(0.0, &good, 0.01);
        extra["parameters"][key] = json!(0.9);
        let (status, text) = post(&base, "/verify_physics", &extra).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(key) && !text.contains("sha256_seal"), "{key}: {text}");
    }
    // A force of no contact in a later segment refuses the whole run before its first step.
    let segments = json!([{ "steps": 1, "load": good }, { "steps": 1, "load": load(0.0, 20.0) }]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, 0.01, segments, 1)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("NoContact") && !text.contains("run_seal"), "{text}");
}

/// No measured twist changed its force or its rate, and a run whose later
/// segment does is refused by name, inside the band as well. A later segment
/// that repeats the load continues the twist. No run and no step starts part
/// way, so a twist cannot be taken up under another load in a second request.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_load_changed_in_a_later_segment_is_refused_by_name() {
    let base = spawn_app().await;
    for changed in [load(2.05, 20.0), load(2.0, 50.0), load(2.000_000_000_000_000_4, 20.0)] {
        let segments = json!([{ "steps": 10, "load": load(2.0, 20.0) }, { "steps": 10, "load": changed }]);
        let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, 0.01, segments, 1)).await;
        assert!(text.contains("LoadChangedDuringTwist"), "{status}: {text}");
        if status == 200 {
            let tape: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(tape["outcome"], "refused", "{text}");
            assert!(tape["steps_landed"].as_u64().unwrap() <= 10, "{text}");
        } else {
            assert_eq!(status, 422, "{text}");
            assert!(!text.contains("run_seal"), "{text}");
        }
    }
    let segments = json!([{ "steps": 10, "load": load(2.0, 20.0) }, { "steps": 10, "load": load(2.0, 20.0) }]);
    let (status, text) = post(&base, "/run_trajectory", &run_body(0.0, 0.01, segments, 1)).await;
    assert_eq!(status, 200, "{text}");
    let tape: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(tape["outcome"], "completed");
    let state = kernel_run(FingerPadLoad { normal_force_n: 2.0, twist_rate_deg_s: 20.0 }, 0.01, 20);
    assert_state_is(&tape["final_state"], &state, "two segments of one load");

    // What the build that was scored answered and this one refuses: a step
    // at the twist the run reached, under another force, and a new run from
    // there.
    let twist = tape["final_state"]["twist_deg"].as_f64().unwrap();
    assert!(twist > 3.9);
    for held in [load(2.0, 20.0), load(2.07, 20.0)] {
        let (status, text) = post(&base, "/verify_physics", &step_body(twist, &held, 0.01)).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains("TwistNotStartedFromZero") && !text.contains("sha256_seal"), "{text}");
        let segments = json!([{ "steps": 5, "load": held }]);
        let (status, text) = post(&base, "/run_trajectory", &run_body(twist, 0.01, segments, 1)).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains("TwistNotStartedFromZero") && !text.contains("run_seal"), "{text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_series_compares_the_torque_and_carries_the_run_seal() {
    let base = spawn_app().await;
    let segments = json!([{ "steps": 375, "load": load(2.0, 20.0) }]);
    let run = run_body(0.0, 0.01, segments, 125);
    let (status, alone) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{alone}");
    let alone: Value = serde_json::from_str(&alone).unwrap();
    let torque = alone["final_state"]["torque_n_m"].as_f64().unwrap();
    let body = json!({
        "run": run, "quantity": "torque_n_m",
        "measurements": [{ "step": 250, "value": torque + 0.001 }, { "step": 375, "value": torque - 0.001 }]
    });
    let (status, text) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["run_seal"], alone["run_seal"]);
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
async fn the_local_connection_lists_the_finger_pad_and_gives_the_bytes_of_the_web_connection() {
    let web = spawn_app().await;
    let mut local = Local::start().await;
    let (listed, error) = local.tool("list_cards", &json!({})).await;
    assert!(!error);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 37);
    let entry = cards
        .iter()
        .find(|card| card["card"] == "fingerpad_p02_glass")
        .expect("the finger pad is listed");
    assert_eq!(entry["domain"], "materials");
    // The line an agent sees in the list says that the constants are fitted.
    assert!(
        entry["specimen"]
            .as_str()
            .unwrap()
            .contains("eight constants fitted to this finger's own trials")
    );

    let which = json!({ "domain": "materials", "card": "fingerpad_p02_glass" });
    let (described, error) = local.tool("describe_card", &which).await;
    assert!(!error);
    let (status, over_web) = post(&web, "/describe_card", &which).await;
    assert_eq!(status, 200);
    assert_eq!(described, over_web);
    let description: Value = serde_json::from_str(&described).unwrap();
    // The line in the list carries the standing: fitted, compared, missed,
    // and the range that is left.
    let line = entry["specimen"].as_str().unwrap();
    for words in [
        "and missed",
        "+9.4, +29.3 and -13.0 percent",
        "It answers at every force and rate above zero with those misses stated",
    ] {
        assert!(line.contains(words), "{words}: {line}");
    }
    // The record's object says the plate held its rate only part of the way.
    let object = description["validation"]["object"].as_str().unwrap();
    assert!(!object.contains("through 80 degrees at a held rate"), "{object}");
    assert!(object.contains("at a held rate to between 77 and 79.7 degrees and then slowing to a stop at 80"), "{object}");
    let result = description["validation"]["result"].as_str().unwrap();
    assert!(result.contains("no fitted constant moved") && !result.contains("no constant moved"), "{result}");
    for words in [
        "by the rule fixed beforehand the comparison failed: of the twelve lines three were met and nine missed",
        "35.63 against 27.55 at 10 N (+29.3 percent)",
        "A simpler rule did better than the card on the slip torque at each condition",
        "this one answers at every force and rate above zero, with these misses stated",
        "At 2 N and 10 degrees a second it is just outside",
        "numerically a two-constant power law of load",
        "n = 0.76 +/- 0.06 and k = 6.74 +/- 1.31 mm",
        "with the paper's own k, the mean of seven fingers, they read +22.3 and +40.8 percent",
        "evidence/verification/second_party/V-40/",
        "SHA-256 9b7ad4088cd7af1b3255eba36cc99cc2f7297c779f76bbdc8432920f13ff7c0a",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let limits = description["validation"]["limits"].as_str().unwrap();
    for words in [
        "All eight constants are fitted to this finger's own trials",
        "nothing there is a test",
        "one trial about the mean of its condition inside a block, 5.2 percent",
        "block to block, 13 to 15 percent",
        "finger to finger, 17 to 26 percent",
        "not randomised because the camera was refocused for each force",
        "does not know the block",
        "+13.6 percent at 0.5 N, +9.4 at 1 N, +21.7 at 5 N and +29.3 at 10 N",
        "read 3.9 percent higher on this finger",
        "The twist at full slip is the card's weakest output",
        "the experiments were conducted on separate days",
        "differ by 0.67 percent at 100",
        "closes for any value of the eight constants",
        "cannot catch a wrong constant",
        "A miss, and it stays on the record",
        "it was scored twice, the second time by a second scorer",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    // What the description once said and no longer says.
    for words in ["in the same session as the fitted trials", "taken in the same conditions without the camera", "a second scoring has not been made", "FLAG_LOAD_ABOVE_FITTED", "FLAG_RATE_ABOVE_FITTED"] {
        assert!(!described.contains(words), "{words}");
    }
    let flags = description["advisory_flags"].as_array().unwrap();
    assert_eq!(flags.len(), 2, "{flags:?}");
    assert_eq!(flags[0]["name"], "FLAG_FULL_SLIP");
    assert_eq!(flags[0]["kind"], "event");
    assert_eq!(flags[1]["name"], "FLAG_BEYOND_COMPARED");
    for words in ["FLAG_FULL_SLIP, an event and not a warning", "1.1 to 6.9 percent under its limit"] {
        assert!(described.contains(words), "{words}");
    }
    for words in [
        "All eight constants are fitted to this finger's own trials",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    // Re-pinned for 0.10.1: after the kernel's 11 names the list holds the gateway's own three for this card, DtBelowFloor and DtAboveCeiling and ContinuationNotOnCard.
    assert_eq!(description["rejects"].as_array().map(Vec::len), Some(15), "{}", description["rejects"]);

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
    assert_eq!(tape["steps_landed"], 375);
    assert_eq!(tape["frames"].as_array().map(Vec::len), Some(15));

    // A refusal is the same sentence on both, marked as an error on the local connection.
    let mut off = step.clone();
    off["parameters"]["normal_force_n"] = json!(0.0);
    let (over_local, error) = local.tool("verify_step", &off).await;
    assert!(error);
    let (status, over_web) = post(&web, "/verify_physics", &off).await;
    assert_eq!(status, 422);
    assert!(over_local.contains("NoContact") && over_web.contains("NoContact"));
}
