//! The titanium lattice card: restarts, the start velocity and the turning point.
//!
//! A state the card returned is handed back as a start with its clock, its
//! held load and its ledger, and steps on as the run straight through, bit for
//! bit. The start velocity is held to the largest speed a run inside the
//! ranges lands. A held force just over half the yield force is refused at the
//! turning point that passes 880 MPa between two landings.

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::ti64_lattice;
use protocol_mcp_gateway::app;
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

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::new();
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

async fn run(base: &str, body: &Value) -> Value {
    let (status, text) = post(base, "/run_trajectory", body).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

const CEILING_S: f64 = 1.366_945_975_716_392_8e-7;

/// A new start at this phase under no force at the reference temperature,
/// with the ledger anchored at its own energy.
fn fresh(position_m: [f64; 3], velocity_m_s: [f64; 3]) -> Value {
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let state = ti64_lattice::Ti64LatticeState::try_phase(&cell, position_m, velocity_m_s).unwrap();
    json!({
        "position_m": position_m,
        "velocity_m_s": velocity_m_s,
        "start_t_s": 0.0,
        "start_force_n": [0.0, 0.0, 0.0],
        "start_temperature_k": 293.15,
        "start_energy_anchor_j": state.energy_anchor_j,
        "start_boundary_work_j": 0.0
    })
}

/// The start a returned state hands back: the phase, the clock, the held load
/// and the ledger, each as the state gives it.
fn handed_back(state: &Value) -> Value {
    json!({
        "position_m": state["position_m"],
        "velocity_m_s": state["velocity_m_s"],
        "start_t_s": state["t_s"],
        "start_force_n": state["applied_force_n"],
        "start_temperature_k": state["temperature_k"],
        "start_energy_anchor_j": state["energy_anchor_j"],
        "start_boundary_work_j": state["boundary_work_j"]
    })
}

/// A request that names in `from` the sealed run it continues: a returned
/// state handed back carries its clock and its ledger on.
fn from(mut request: Value, upstream: &Value, answer: &Value) -> Value {
    request["from"] = json!({ "request": upstream, "run_seal": answer["run_seal"] });
    request
}

fn load(force_n: [f64; 3], temperature_k: f64) -> Value {
    json!({"force_n": force_n, "torque_nm": [0.0, 0.0, 0.0], "temperature_k": temperature_k})
}

fn trajectory(initial: Value, dt_s: f64, segments: Vec<(u64, Value)>) -> Value {
    let total: u64 = segments.iter().map(|(steps, _)| steps).sum();
    let segments: Vec<Value> = segments
        .into_iter()
        .map(|(steps, load)| json!({"steps": steps, "load": load}))
        .collect();
    json!({
        "domain": "materials",
        "card": "ti64_lattice",
        "initial": initial,
        "dt_s": dt_s,
        "segments": segments,
        "record_every": total
    })
}

/// Every field of a returned state, as 64-bit patterns, but the reported
/// torque, which is the applied couple of the last step and not in the motion.
fn state_bits(state: &Value) -> Map<String, Value> {
    let mut fields = state.as_object().unwrap().clone();
    fields.retain(|key, _| key.ends_with("_hex") && key != "torque_nm_hex");
    assert!(fields.len() >= 13, "{state}");
    fields
}

/// The published maximum of one phase field.
async fn published_maximum(base: &str, name: &str) -> f64 {
    let card = describe(base).await;
    card["phase_fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["name"] == name)
        .unwrap_or_else(|| panic!("{name} in {card}"))["maximum"]
        .as_f64()
        .unwrap()
}

/// The mechanical energy of a start under its force and temperature, by the
/// formula describe_card publishes for energy_anchor_j.
fn energy_j(position_m: [f64; 3], velocity_m_s: [f64; 3], force_n: [f64; 3], temperature_k: f64) -> f64 {
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let k = cell.stiffness_n_m()[0];
    let mut energy = 0.5
        * cell.mass_kg()
        * (velocity_m_s[0] * velocity_m_s[0] + velocity_m_s[1] * velocity_m_s[1] + velocity_m_s[2] * velocity_m_s[2]);
    for axis in 0..3 {
        let stretch = position_m[axis] - ti64_lattice::thermal_extension_m(cell.length_m[axis], temperature_k);
        energy += 0.5 * k * stretch * stretch - force_n[axis] * position_m[axis];
    }
    energy
}

/// 60 steps from -x_yield under +1760 N land the node at 31.6 m/s, above the
/// 22.6 m/s the card used to publish, and that state was refused as a start.
/// It is now a start, and the restart is the 95-step run straight through.
#[tokio::test]
async fn a_state_landed_under_a_held_force_is_a_start() {
    let base = spawn_app().await;
    let x_yield = 1.546_572_934_973_638e-4;
    let start = fresh([-x_yield, 0.0, 0.0], [0.0; 3]);
    let held = load([1760.0, 0.0, 0.0], 293.15);
    let first_request = trajectory(start.clone(), CEILING_S, vec![(60, held.clone())]);
    let first = run(&base, &first_request).await;
    assert_eq!(first["outcome"], "completed", "{first}");
    let landed = &first["final_state"];
    let speed = landed["velocity_m_s"][0].as_f64().unwrap();
    assert!(speed > 31.6 && speed < 31.7, "{speed}");
    let straight = run(&base, &trajectory(start, CEILING_S, vec![(95, held.clone())])).await;
    assert_eq!(straight["outcome"], "completed", "{straight}");
    let by_hand = trajectory(handed_back(landed), CEILING_S, vec![(35, held.clone())]);
    let (status, text) = post(&base, "/run_trajectory", &by_hand).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    let restart = run(&base, &from(by_hand, &first_request, &first)).await;
    assert_eq!(restart["outcome"], "completed", "{restart}");
    assert_eq!(state_bits(&restart["final_state"]), state_bits(&straight["final_state"]));

    // The node redirected at -3520 N and then left free for 2000 steps never
    // yields, and the state at 31.6 m/s that opens that run is a start as
    // well.
    let redirected = vec![(48, load([-3520.0, 0.0, 0.0], 293.15)), (2000, load([0.0; 3], 293.15))];
    let carried = run(&base, &from(trajectory(handed_back(landed), CEILING_S, redirected), &first_request, &first)).await;
    assert_eq!(carried["outcome"], "completed", "{carried}");
}

/// The largest held force from -x_yield lands the node near 2 omega x_yield,
/// 45.25 m/s. Heated as it moves, a run under forces and temperatures in
/// range lands faster still, and that state too is a start.
#[tokio::test]
async fn the_fastest_states_a_run_lands_are_starts() {
    let base = spawn_app().await;
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let x_yield = 1.546_572_934_973_638e-4;
    let bound = published_maximum(&base, "velocity_m_s").await;
    let held_bound = 2.0 * cell.max_natural_frequency_rad_s() * x_yield;
    assert!((held_bound - 45.256_300_174_207_13).abs() < 1.0e-9, "{held_bound}");
    assert!(bound > 66.0 && bound < 66.2, "{bound}");

    // Held force, held temperature: the speed after the sweep across the
    // yield stretch.
    let start = fresh([-x_yield, 0.0, 0.0], [0.0; 3]);
    let pulled = load([3520.0, 0.0, 0.0], 293.15);
    let swept_request = trajectory(start.clone(), CEILING_S, vec![(78, pulled.clone())]);
    let swept = run(&base, &swept_request).await;
    assert_eq!(swept["outcome"], "completed", "{swept}");
    let speed = swept["final_state"]["velocity_m_s"][0].as_f64().unwrap();
    assert!(speed > 45.0 && speed <= held_bound, "{speed}");
    // Handed back, it is a start. It is bound for yield, and the card's own
    // yield test refuses the step that passes it.
    let restart = run(
        &base,
        &from(trajectory(handed_back(&swept["final_state"]), CEILING_S, vec![(400, pulled.clone())]), &swept_request, &swept),
    )
    .await;
    assert_eq!(restart["outcome"], "refused", "{restart}");
    assert_eq!(restart["reject"], "YieldReached", "{restart}");

    // Heated as it moves: the temperature of each step keeps the stretch
    // a little short of yield in compression, up to the beta transus. The
    // plan is found on the kernel's own step.
    let alpha_l = ti64_lattice::THERMAL_EXPANSION_PER_K * cell.length_m[0];
    let mut state = ti64_lattice::Ti64LatticeState::try_phase(&cell, [-0.99 * x_yield, 0.0, 0.0], [0.0; 3]).unwrap();
    let mut frame = LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof: [0; 16],
    };
    let mut plan: Vec<(u64, Value)> = Vec::new();
    loop {
        let wanted = 293.15 + (state.position_m[0] + 0.99 * x_yield) / alpha_l;
        let temperature_k = wanted.clamp(293.15, ti64_lattice::BETA_TRANSUS_K);
        let hot = temperature_k == ti64_lattice::BETA_TRANSUS_K;
        let stretch = state.position_m[0]
            - ti64_lattice::thermal_extension_m(cell.length_m[0], temperature_k);
        if hot && stretch > 0.0 {
            break;
        }
        let step_load = ti64_lattice::Ti64Load {
            force_n: [3520.0, 0.0, 0.0],
            torque_nm: [0.0; 3],
            temperature_k,
        };
        ti64_lattice::step(&cell, &mut state, &mut frame, step_load, CEILING_S).unwrap();
        plan.push((1, load([3520.0, 0.0, 0.0], temperature_k)));
        assert!(plan.len() < 2_000);
    }
    let fast = state.velocity_m_s[0];
    assert!(fast > 1.25 * held_bound && fast < bound, "{fast} m/s");
    let ramp_start = fresh([-0.99 * x_yield, 0.0, 0.0], [0.0; 3]);
    let ramped_request = trajectory(ramp_start, CEILING_S, plan);
    let ramped = run(&base, &ramped_request).await;
    assert_eq!(ramped["outcome"], "completed", "{ramped}");
    assert_eq!(
        ramped["final_state"]["velocity_m_s_hex"][0],
        format!("{:016x}", fast.to_bits())
    );
    // Handed back with its hot temperature, the state is a start: its
    // position is past the cold yield position and its speed past 2 omega
    // x_yield. Held hot and pulled, it lands its next step.
    let hot = load([3520.0, 0.0, 0.0], ti64_lattice::BETA_TRANSUS_K);
    let next = run(&base, &from(trajectory(handed_back(&ramped["final_state"]), CEILING_S, vec![(1, hot)]), &ramped_request, &ramped)).await;
    assert_eq!(next["outcome"], "completed", "{next}");
}

/// The published speed is inclusive; the next number above it is refused at
/// the door by name.
#[tokio::test]
async fn a_start_faster_than_the_bound_is_refused_as_yield() {
    let base = spawn_app().await;
    let bound = published_maximum(&base, "velocity_m_s").await;
    for (speed, accepted) in [(bound, true), (bound.next_up(), false), (-bound.next_up(), false)] {
        let mut start = fresh([0.0; 3], [0.0; 3]);
        start["velocity_m_s"] = json!([0.0, speed, 0.0]);
        start["start_energy_anchor_j"] = json!(energy_j([0.0; 3], [0.0, speed, 0.0], [0.0; 3], 293.15));
        let (status, text) = post(&base, "/run_trajectory", &trajectory(start, CEILING_S, vec![(1, load([0.0; 3], 293.15))])).await;
        if accepted {
            assert_eq!(status, 200, "{text}");
        } else {
            assert_eq!(status, 422, "{text}");
            assert!(text.contains("'velocity_m_s[1]'") && text.contains("(YieldReached)"), "{text}");
        }
    }
}

/// 1760.04 N held from rest peaks at 880.02 MPa between two landings, each
/// under 880 MPa, and used to complete a million steps. It is refused at the
/// first turning point. Half the yield force, 1760 N, peaks at 880 MPa and
/// lands.
#[tokio::test]
async fn a_force_just_over_half_yield_is_refused_at_its_turning_point() {
    let base = spawn_app().await;
    let dt_s = 1.363_297_598_465_000_3e-7;
    let rest = fresh([0.0; 3], [0.0; 3]);
    let over = run(&base, &trajectory(rest.clone(), dt_s, vec![(1_000_000, load([1760.04, 0.0, 0.0], 293.15))])).await;
    assert_eq!(over["outcome"], "refused", "{over}");
    assert_eq!(over["reject"], "YieldReached", "{over}");
    assert_eq!(over["steps_landed"], 157, "{over}");
    let landed_mpa = over["final_state"]["von_mises_mpa"].as_f64().unwrap();
    assert!(landed_mpa < 880.0, "{landed_mpa}");
    let half = run(&base, &trajectory(rest, dt_s, vec![(20_000, load([1760.0, 0.0, 0.0], 293.15))])).await;
    assert_eq!(half["outcome"], "completed", "{half}");

    let card = describe(&base).await;
    let meaning = card["rejects"].to_string();
    assert!(meaning.contains("at a turning point of the motion inside the step"), "{meaning}");
}

async fn describe(base: &str) -> Value {
    let (status, text) = post(base, "/describe_card", &json!({"domain": "materials", "card": "ti64_lattice"})).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

/// A run of 600 steps and the same run as 300 steps followed by 300 from the
/// returned state agree in every field, the ledger and the clock included.
#[tokio::test]
async fn a_restart_carries_the_clock_and_the_ledger() {
    let base = spawn_app().await;
    let start = fresh([2.0e-5, -1.0e-5, 0.0], [1.0, 2.0, -3.0]);
    let held = load([700.0, -300.0, 50.0], 293.15);
    let straight = run(&base, &trajectory(start.clone(), CEILING_S, vec![(600, held.clone())])).await;
    let first_request = trajectory(start, CEILING_S, vec![(300, held.clone())]);
    let first = run(&base, &first_request).await;
    let second = run(&base, &from(trajectory(handed_back(&first["final_state"]), CEILING_S, vec![(300, held)]), &first_request, &first)).await;
    for body in [&straight, &first, &second] {
        assert_eq!(body["outcome"], "completed", "{body}");
    }
    let a = state_bits(&straight["final_state"]);
    let b = state_bits(&second["final_state"]);
    assert_eq!(a, b);
    for key in ["t_s_hex", "energy_anchor_j_hex", "boundary_work_j_hex", "residual_conservation_error_hex"] {
        assert!(a.contains_key(key), "{key}");
    }
}

/// A run at the step ceiling of a million steps, with all three ligaments
/// swinging at 99 percent of yield, spends 89 percent of the conservation
/// limit. Its ledger handed back by hand is refused by name: a start that
/// carries a run on arrives with from. Named in from, a second million steps
/// is refused as ChainStepsAboveLimit: a chain holds at most a million steps
/// in all, so a ledger is weighed over at most a million steps of one chain.
#[tokio::test]
async fn a_ledger_handed_back_is_weighed_over_the_whole_run() {
    let base = spawn_app().await;
    let cell = ti64_lattice::Ti64LatticeCell::annealed_column();
    let w = cell.max_natural_frequency_rad_s();
    let amplitude_m = 0.99 * ti64_lattice::uniaxial_yield_strain() * cell.length_m[0];
    let start = fresh([0.0; 3], [w * amplitude_m; 3]);
    let free = load([0.0; 3], 293.15);
    let first_request = trajectory(start, CEILING_S, vec![(1_000_000, free.clone())]);
    let first = run(&base, &first_request).await;
    assert_eq!(first["outcome"], "completed", "{first}");
    let spent = first["final_state"]["residual_conservation_error"].as_f64().unwrap() / 1.0e-6;
    assert!(spent > 0.85 && spent < 0.9, "{spent}");
    let by_hand = trajectory(handed_back(&first["final_state"]), CEILING_S, vec![(1_000_000, free)]);
    let (status, text) = post(&base, "/run_trajectory", &by_hand).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{text}");
    let (status, text) = post(&base, "/run_trajectory", &from(by_hand, &first_request, &first)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(ChainStepsAboveLimit)") && !text.contains("run_seal"), "{text}");
}

/// A start without its clock, held load or ledger is refused naming the
/// missing field; a start that states what no run writes is refused as
/// StartNotWritten.
#[tokio::test]
async fn a_start_without_its_continuation_or_one_no_run_writes_is_refused() {
    let base = spawn_app().await;
    let one = |initial: Value| trajectory(initial, CEILING_S, vec![(1, load([0.0; 3], 293.15))]);
    for key in ["start_t_s", "start_force_n", "start_temperature_k", "start_energy_anchor_j", "start_boundary_work_j"] {
        let mut start = fresh([1.0e-5, 0.0, 0.0], [0.0; 3]);
        start.as_object_mut().unwrap().remove(key);
        let (status, text) = post(&base, "/run_trajectory", &one(start)).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(key), "{key}: {text}");
    }
    let cases = [
        ("start_t_s", json!(-1.0e-9)),
        ("start_boundary_work_j", json!(0.25)),
        ("start_energy_anchor_j", json!(0.0)),
    ];
    for (key, value) in cases {
        let mut start = fresh([1.0e-4, 0.0, 0.0], [0.0; 3]);
        start[key] = value;
        let (status, text) = post(&base, "/run_trajectory", &one(start)).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains("StartNotWritten"), "{key}: {text}");
    }
    let mut start = fresh([0.0; 3], [0.0; 3]);
    start["start_temperature_k"] = json!(1268.16);
    let (status, text) = post(&base, "/run_trajectory", &one(start)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("TemperatureOutsideCard"), "{text}");
    // A position past the cold yield position is inside the published span.
    // Stated at 293.15 K it is past yield on the incoming state; stated at
    // 1000 K, where it stands short of yield, it lands.
    let position_m = [2.0e-4, 0.0, 0.0];
    for (temperature_k, lands) in [(293.15, false), (1000.0, true)] {
        let mut start = fresh([0.0; 3], [0.0; 3]);
        start["position_m"] = json!(position_m);
        start["start_temperature_k"] = json!(temperature_k);
        start["start_energy_anchor_j"] = json!(energy_j(position_m, [0.0; 3], [0.0; 3], temperature_k));
        let body = run(&base, &trajectory(start, CEILING_S, vec![(1, load([0.0; 3], 1000.0))])).await;
        if lands {
            assert_eq!(body["outcome"], "completed", "{body}");
        } else {
            assert_eq!(body["outcome"], "refused", "{body}");
            assert_eq!(body["reject"], "YieldReached", "{body}");
            assert_eq!(body["steps_landed"], 0, "{body}");
        }
    }
    // Further out than the published span is past yield at every
    // temperature and is refused at the door by name.
    let highest = describe(&base).await["phase_fields"][0]["maximum"].as_f64().unwrap();
    assert!(highest > 3.2e-4 && highest < 3.3e-4, "{highest}");
    let mut start = fresh([0.0; 3], [0.0; 3]);
    start["position_m"] = json!([highest.next_up(), 0.0, 0.0]);
    let (status, text) = post(&base, "/run_trajectory", &one(start)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("(YieldReached)"), "{text}");
}
