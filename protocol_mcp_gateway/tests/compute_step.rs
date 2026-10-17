//! compute_step, the tool that computes one step, and verify_step, its
//! earlier name, served beside it for one version: on every connection the
//! two take the same arguments and give the same answer, byte for byte, and
//! the web connection answers the same at POST /compute_step and at its
//! earlier route, POST /verify_physics.

mod hosted_support;

use hosted_support::*;
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// A landed step, a step refused by name, a key missing, and a card that is
/// not served.
fn steps() -> Vec<Value> {
    let landed = thermal_step();
    let mut refused = thermal_step();
    refused["parameters"]["temperature_k"] = json!(-1.0);
    let mut missing = thermal_step();
    missing["parameters"].as_object_mut().unwrap().remove("ambient_k");
    let mut unknown = thermal_step();
    unknown["card"] = json!("no_such_card");
    vec![landed, refused, missing, unknown]
}

#[test]
fn the_tool_list_holds_compute_step_and_its_earlier_name_says_what_it_is() {
    let reply = protocol_mcp_gateway::mcp_server::tools_in(&protocol_mcp_gateway::view::View::full());
    let names: Vec<&str> = reply.iter().map(|tool| tool.name.as_ref()).collect();
    assert_eq!(names.iter().filter(|name| **name == "compute_step").count(), 1);
    assert_eq!(names.last(), Some(&"verify_step"), "the earlier name is listed last: {names:?}");
    assert!(names.iter().position(|name| *name == "compute_step") < names.iter().position(|name| *name == "run_trajectory"));
    let described = |name: &str| reply.iter().find(|tool| tool.name == name).unwrap().description.clone().unwrap().to_string();
    assert!(described("compute_step").starts_with("Computes one anchored step of one card"));
    let earlier = described("verify_step");
    assert!(earlier.starts_with("Computes one anchored step of one card, as compute_step does"), "{earlier}");
    assert!(earlier.contains("the earlier name of compute_step"), "{earlier}");
    // The two take the same input schema.
    let schema = |name: &str| Value::Object(reply.iter().find(|tool| tool.name == name).unwrap().input_schema.as_ref().clone());
    assert_eq!(schema("compute_step"), schema("verify_step"));
}

#[test]
fn compute_step_and_verify_step_answer_the_same_bytes_on_the_local_and_the_hosted_connection() {
    let mut door = Stdio2::start(None);
    let hosted = Hosted::start(vec![client_entry("all", "k1", 4, 2)], None);
    for step in steps() {
        let now = door.tool("compute_step", &step);
        let before = door.tool("verify_step", &step);
        assert_eq!(now, before, "{step}");
        let (status, error, text) = hosted.tool("k1", "compute_step", &step);
        assert_eq!((status, error, text.clone()), (200, now.0, now.1.clone()), "{step}");
        assert_eq!(hosted.tool("k1", "verify_step", &step), (200, error, text), "{step}");
    }
    let (error, text) = door.tool("compute_step", &steps()[2]);
    assert!(error);
    assert_eq!(text, "Missing required parameter: 'ambient_k' (InputMissing).");
}

#[tokio::test]
async fn the_web_connection_answers_the_same_at_compute_step_and_at_verify_physics() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, protocol_mcp_gateway::app()).await.unwrap();
    });
    let client = reqwest::Client::new();
    for step in steps() {
        let mut answers = Vec::new();
        for route in ["compute_step", "verify_physics"] {
            let reply = client.post(format!("http://{address}/{route}")).json(&step).send().await.unwrap();
            answers.push((reply.status().as_u16(), reply.text().await.unwrap()));
        }
        assert_eq!(answers[0], answers[1], "{step}");
    }
}

/// The red team of 0.13.0: a whole compute_step answer sent to verify_tape,
/// which verifies a run, was refused as "Missing required parameter:
/// 'frames' (InputMissing)". It is named as a step's answer, with how a step
/// is verified; and that way verifies it.
#[test]
fn a_steps_answer_sent_to_verify_tape_is_named_with_how_a_step_is_verified() {
    let mut stdio = Stdio2::start(None);
    let (error, text) = stdio.tool("compute_step", &thermal_step());
    assert!(!error, "{text}");
    let step: Value = serde_json::from_str(&text).unwrap();
    let (error, refused) = stdio.tool("verify_tape", &step);
    assert!(error, "{refused}");
    assert_eq!(
        refused,
        "Physical Violation: this is the answer of one step (compute_step): it carries sha256_seal and no frames, and verify_tape verifies a run. A step is verified by request: send its inputs to compute_step, or to verify_step, its earlier name, and compare the sha256_seal it answers with this one; the step is computed again under a physics permit."
    );
    for tool in ["compute_step", "verify_step"] {
        let (error, again) = stdio.tool(tool, &step["inputs"]);
        assert!(!error, "{again}");
        let again: Value = serde_json::from_str(&again).unwrap();
        assert_eq!(again["sha256_seal"], step["sha256_seal"], "{tool}");
    }
}
