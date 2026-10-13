//! The copper fuse link on the web connection: the electrical bounds hold on
//! every landed step, and the description says which numbers start again on
//! a restart.
//!
//! In 0.6.0 a held voltage at the bound rang the node to 1037.75 V and sealed
//! it, and that returned state was refused as a start. A landed step past
//! either bound is now refused by name, so every sealed state is one the door
//! accepts.

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::grid_node::{self, GridLoad, GridNode, GridNodeState, StepReject};
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
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
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

/// The kernel loop: landed steps, the last landed state and the refusal.
fn kernel_hold(start: GridNodeState, load: GridLoad, steps: u64) -> (u64, GridNodeState, Option<StepReject>) {
    let node = GridNode::copper_fuse_link();
    let mut state = start;
    let mut frame = blank_frame();
    for landed in 0..steps {
        if let Err(reject) = grid_node::step(&node, &mut state, &mut frame, load, 1.0e-6) {
            return (landed, state, Some(reject));
        }
    }
    (steps, state, None)
}

fn run_request(initial: Value, load: Value, steps: u64) -> Value {
    json!({
        "domain": "energy", "card": "grid_node",
        "initial": initial,
        "dt_s": 1e-6,
        "segments": [{ "steps": steps, "load": load }],
        "record_every": steps
    })
}

fn number(state: &Value, key: &str) -> f64 {
    let hex = state[format!("{key}_hex")].as_str().unwrap_or_else(|| panic!("{key}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

const BOUND: f64 = 595.7150486594088;

#[tokio::test]
async fn a_held_voltage_at_the_bound_is_refused_where_it_rings_past_the_bound() {
    let base = spawn_app().await;
    let node = GridNode::copper_fuse_link();
    assert_eq!(node.melt_voltage_v().to_bits(), BOUND.to_bits());
    assert_eq!(node.melt_current_a().to_bits(), BOUND.to_bits());
    // D1a: 157 steps from rest at the bound. 0.6.0 sealed voltage_v 1037.75 V.
    let request = run_request(
        json!({"current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15}),
        json!({"upstream_voltage_v": BOUND, "load_current_a": 0.0, "ambient_k": 293.15}),
        157,
    );
    let (status, text) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let load = GridLoad { upstream_voltage_v: BOUND, load_current_a: 0.0, ambient_k: 293.15 };
    let (landed, last, reject) = kernel_hold(GridNodeState::try_rest(&node).unwrap(), load, 157);
    assert_eq!(reject, Some(StepReject::VoltageOutsideCard));
    assert_eq!(body["outcome"], "refused", "{body}");
    assert_eq!(body["reject"], "VoltageOutsideCard", "{body}");
    assert_eq!(body["steps_landed"], landed, "{body}");
    assert_eq!(
        body["reject_detail"],
        format!("Physical Failure: the grid step landed with |voltage_v| above 595.7150486594088 V, the inclusive maximum of the grid_node card (VoltageOutsideCard). The run is sealed to step {landed}. The state at step {} was refused and is not sealed.", landed + 1)
    );
    let state = &body["final_state"];
    assert_eq!(number(state, "voltage_v").to_bits(), last.voltage_v.to_bits());
    assert!(number(state, "voltage_v").abs() <= BOUND);
    assert!(number(state, "voltage_spike_v") <= BOUND);

    // The last sealed state is a start the door accepts. The load holds the
    // shunt voltage still for one step, so the step lands inside the bounds.
    let step = json!({
        "domain": "energy", "card": "grid_node",
        "parameters": {
            "current_a": number(state, "current_a"),
            "voltage_v": number(state, "voltage_v"),
            "temperature_k": number(state, "temperature_k"),
            "upstream_voltage_v": number(state, "voltage_v"),
            "load_current_a": number(state, "current_a"),
            "ambient_k": 293.15, "dt_s": 1e-6
        }
    });
    let (status, text) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 200, "{text}");
}

#[tokio::test]
async fn a_held_current_rings_the_feeder_past_its_bound_and_is_refused() {
    let base = spawn_app().await;
    let node = GridNode::copper_fuse_link();
    // D1b at 0.75 of the bound: 0.6.0 sealed current_a 770.8 A at the bound itself.
    let held = 0.75 * BOUND;
    let request = run_request(
        json!({"current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15}),
        json!({"upstream_voltage_v": 0.0, "load_current_a": held, "ambient_k": 293.15}),
        200,
    );
    let (status, text) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let load = GridLoad { upstream_voltage_v: 0.0, load_current_a: held, ambient_k: 293.15 };
    let (landed, _, reject) = kernel_hold(GridNodeState::try_rest(&node).unwrap(), load, 200);
    assert_eq!(reject, Some(StepReject::CurrentOutsideCard));
    assert_eq!(body["reject"], "CurrentOutsideCard", "{body}");
    assert_eq!(body["steps_landed"], landed, "{body}");
    assert!(body["reject_detail"].as_str().unwrap().contains("|current_a| above 595.7150486594088 A"), "{body}");
    assert!(number(&body["final_state"], "current_a").abs() <= BOUND);

    // A request at the bound itself is refused on the voltage first.
    let request = run_request(
        json!({"current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15}),
        json!({"upstream_voltage_v": 0.0, "load_current_a": BOUND, "ambient_k": 293.15}),
        100,
    );
    let (_, text) = post(&base, "/run_trajectory", &request).await;
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["reject"], "VoltageOutsideCard", "{body}");
}

#[tokio::test]
async fn a_single_step_that_lands_past_the_voltage_bound_is_refused() {
    let base = spawn_app().await;
    // D1c: 0.6.0 sealed voltage_v 619.29 V.
    let step = json!({
        "domain": "energy", "card": "grid_node",
        "parameters": {
            "current_a": BOUND, "voltage_v": BOUND, "temperature_k": 233.15,
            "upstream_voltage_v": -BOUND, "load_current_a": -BOUND, "ambient_k": 233.15, "dt_s": 1e-6
        }
    });
    let (status, text) = post(&base, "/verify_physics", &step).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("VoltageOutsideCard") && text.contains("not sealed"), "{text}");
}

#[tokio::test]
async fn the_description_names_the_bounds_and_what_starts_again() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_card", &json!({"domain": "energy", "card": "grid_node"})).await;
    assert_eq!(status, 200, "{text}");
    let card: Value = serde_json::from_str(&text).unwrap();
    let names: Vec<&str> = card["rejects"].as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"CurrentOutsideCard") && names.contains(&"VoltageOutsideCard"), "{names:?}");
    for reject in card["rejects"].as_array().unwrap() {
        assert!(!reject["meaning"].as_str().unwrap().is_empty(), "{reject}");
    }
    let slot = card["frame_slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slot| slot["slot"] == "vel[2]")
        .unwrap();
    assert_eq!(slot["state"], "voltage_spike_v");
    let rule = card["dt_s"]["rule"].as_str().unwrap();
    assert!(rule.contains("energy_anchor_j, boundary_work_j") && rule.contains("starts all of them again") && rule.contains("voltage_spike_v is the largest |voltage_v| since that start"), "{rule}");
    let field = |name: &str| {
        card["phase_fields"]
            .as_array()
            .unwrap()
            .iter()
            .chain(card["load_fields"].as_array().unwrap())
            .find(|field| field["name"] == name)
            .unwrap()
            .to_string()
    };
    assert!(field("upstream_voltage_v").contains("refused as VoltageOutsideCard"));
    assert!(field("load_current_a").contains("refused as CurrentOutsideCard"));
    assert!(field("current_a").contains("CurrentOutsideCard"));
    assert!(field("voltage_v").contains("VoltageOutsideCard"));
}

#[tokio::test]
async fn a_restart_matches_the_state_and_starts_the_peak_again() {
    let base = spawn_app().await;
    // The straight run and the restart agree on current, voltage and
    // temperature; the peak and the ledger start again, as the card says.
    let straight = json!({
        "domain": "energy", "card": "grid_node",
        "initial": {"current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15},
        "dt_s": 1e-6,
        "segments": [
            {"steps": 200, "load": {"upstream_voltage_v": 48.0, "load_current_a": 1.0, "ambient_k": 293.15}},
            {"steps": 200, "load": {"upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 253.15}}
        ],
        "record_every": 200
    });
    let (_, text) = post(&base, "/run_trajectory", &straight).await;
    let straight: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(straight["outcome"], "completed", "{straight}");
    let first = run_request(
        json!({"current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15}),
        json!({"upstream_voltage_v": 48.0, "load_current_a": 1.0, "ambient_k": 293.15}),
        200,
    );
    let (_, text) = post(&base, "/run_trajectory", &first).await;
    let first: Value = serde_json::from_str(&text).unwrap();
    let mid = &first["final_state"];
    let again = run_request(
        json!({
            "current_a": number(mid, "current_a"),
            "voltage_v": number(mid, "voltage_v"),
            "temperature_k": number(mid, "temperature_k")
        }),
        json!({"upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 253.15}),
        200,
    );
    let (_, text) = post(&base, "/run_trajectory", &again).await;
    let again: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(again["outcome"], "completed", "{again}");
    for key in ["current_a", "voltage_v", "temperature_k"] {
        assert_eq!(number(&again["final_state"], key).to_bits(), number(&straight["final_state"], key).to_bits(), "{key}");
    }
    assert!(number(&again["final_state"], "voltage_spike_v") < number(&straight["final_state"], "voltage_spike_v"));
}
