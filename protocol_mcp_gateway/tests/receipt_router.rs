//! `POST /verify_receipt` and the MCP tool of the same name.
//!
//! A receipt is the SHA-384 of the exact bytes of a response before it. The
//! check reads those bytes and nothing else. It does not run the engine, so a
//! response made with no engine and given a receipt of its own matches (R5).

mod dataset_cases;

use std::time::Duration;

use dataset_cases::{request, run};
use protocol_mcp_gateway::app;
use reqwest::Client;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha384};
use tokio::net::TcpListener;

const TAIL_REFUSAL: &str =
    "Physical Violation: the text does not end with a receipt as this verifier writes one.";
const COVERS: &str = "every byte of the response before the receipt, the decimal values included.";
const DOES_NOT_SHOW: &str = "who produced the response or when: the receipt has no key. The engine's signature after it, when the response carries one, shows which key issued it and when, and is reported under signature. Replaying the request through the engine shows the engine produced it.";

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
    let client = Client::new();
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

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    post_text(base, path, &serde_json::to_string(body).unwrap()).await
}

fn sha384_hex(bytes: &[u8]) -> String {
    hex::encode(Sha384::digest(bytes))
}

/// The answer of the check for a text, which must be 200.
async fn checked(base: &str, text: &str) -> Value {
    let (status, body) = post(base, "/verify_receipt", &json!({ "response_text": text })).await;
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

async fn refused(base: &str, body: &Value) -> String {
    let (status, text) = post(base, "/verify_receipt", body).await;
    assert_eq!(status, 422, "{text}");
    text
}

fn step() -> Value {
    json!({
        "domain": "energy",
        "card": "thermal",
        "parameters": {
            "temperature_k": 293.15,
            "heat_power_w": 1.0,
            "ambient_k": 293.15,
            "dt_s": 0.1
        }
    })
}

fn trajectory() -> Value {
    json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{ "steps": 4, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": 1
    })
}

fn solve() -> Value {
    json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10 } })
}

async fn genuine(base: &str, path: &str, body: &Value) -> String {
    let (status, text) = post(base, path, body).await;
    assert_eq!(status, 200, "{text}");
    text
}

/// The tail every response ends with, and the text before it as a document.
fn split_tail(text: &str) -> (&str, &str) {
    let at = text.rfind(",\"receipt_sha384\":\"").expect("a receipt");
    (&text[..at], &text[at..])
}

// R1
#[tokio::test]
async fn a_genuine_response_from_each_of_the_four_builders_matches() {
    let base = spawn_app().await;

    for (label, path, body) in [
        ("verify_physics", "/verify_physics", step()),
        ("run_trajectory", "/run_trajectory", trajectory()),
        ("solve", "/solve", solve()),
    ] {
        let text = genuine(&base, path, &body).await;
        let report = checked(&base, &text).await;
        assert_eq!(report["matches"], true, "{label}: {report}");
        let stated = serde_json::from_str::<Value>(&text).unwrap()["receipt_sha384"].clone();
        assert_eq!(report["receipt_sha384"], stated, "{label}");
        assert_eq!(report["recomputed_sha384"], stated, "{label}");
        let (document, _) = split_tail(&text);
        assert_eq!(
            report["recomputed_sha384"],
            sha384_hex(format!("{document}}}").as_bytes()),
            "{label}"
        );
        // Since the 0.13.0 candidate's fix pass the response's engine_version
        // is said beside matches, with whether it is this engine's.
        let keys: Vec<&str> = report.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            ["matches", "engine_version", "engine_version_is_this_engines", "receipt_sha384", "recomputed_sha384", "covers", "does_not_show", "match_proves"],
            "{label}"
        );
        assert_eq!(report["engine_version"], serde_json::from_str::<Value>(&text).unwrap()["engine_version"], "{label}");
        assert_eq!(report["engine_version_is_this_engines"], true, "{label}");
        assert_eq!(report["covers"], COVERS);
        assert_eq!(report["does_not_show"], DOES_NOT_SHOW);
    }

    // The head of an inline dataset, written as `verify_dataset` writes it:
    // the head object read and encoded again, keys in the order received. Its
    // receipt covers the head's own fields only (`request`, `engine_version`,
    // the counts, `plan_sha384`, `merkle_root`, `dataset_seal`, `rows_sha384`
    // and `frames_sha384`). The rows and the frames are covered by the two
    // file hashes inside the head, not by the receipt directly.
    let plan = json!({ "kind": "list", "runs": [run(293.15, 1.0, 3), run(300.0, 2.0, 2)] });
    let answer: Value = serde_json::from_str(
        &genuine(&base, "/run_dataset", &request(plan, "frames", "inline")).await,
    )
    .unwrap();
    let head_text = serde_json::to_string(&answer["head"]).unwrap();
    let report = checked(&base, &head_text).await;
    assert_eq!(report["matches"], true, "{report}");
    assert_eq!(report["receipt_sha384"], answer["head"]["receipt_sha384"]);
    assert_eq!(report["engine_version"], answer["head"]["engine_version"]);
    assert_eq!(report["engine_version_is_this_engines"], true);
}

// R2
#[tokio::test]
async fn a_changed_decimal_beside_its_hex_holds_for_the_tape_and_breaks_the_receipt() {
    let base = spawn_app().await;
    let text = genuine(&base, "/run_trajectory", &trajectory()).await;

    // `final_state.temperature_k` is a decimal with `temperature_k_hex` beside it.
    let state_at = text.find("\"final_state\":{").expect("final_state");
    let key_at = state_at + text[state_at..].find("\"temperature_k\":").unwrap() + "\"temperature_k\":".len();
    let end_at = key_at + text[key_at..].find(',').unwrap();
    let token = &text[key_at..end_at];
    let last = token.chars().last().unwrap();
    assert!(last.is_ascii_digit(), "{token}");
    let other = if last == '7' { '3' } else { '7' };
    let changed_token = format!("{}{other}", &token[..token.len() - 1]);
    let changed = format!("{}{changed_token}{}", &text[..key_at], &text[end_at..]);
    assert_ne!(changed, text);
    assert_eq!(changed.len(), text.len());

    // The seals are over the exact hex, and they still hold; since 0.11.0
    // the long form writes the answer again from them and names the decimal
    // that is not its hex's value.
    let (status, tape) = post_text(&base, "/verify_tape", &changed).await;
    assert_eq!(status, 200, "{tape}");
    let tape: Value = serde_json::from_str(&tape).unwrap();
    assert_eq!(tape["holds"], false, "{tape}");
    assert_eq!((tape["cause"].clone(), tape["differs_at"].clone()), (json!("answer_text"), json!("final_state.temperature_k")), "{tape}");

    // The receipt covers the decimal: it does not match.
    let report = checked(&base, &changed).await;
    assert_eq!(report["matches"], false, "{report}");
    assert_eq!(report["receipt_sha384"], serde_json::from_str::<Value>(&text).unwrap()["receipt_sha384"]);
    assert_ne!(report["recomputed_sha384"], report["receipt_sha384"]);

    // The unedited text matches, so the digit alone made the difference.
    assert_eq!(checked(&base, &text).await["matches"], true);
}

// R3
#[tokio::test]
async fn any_other_change_to_the_bytes_breaks_the_receipt_or_leaves_the_form() {
    let base = spawn_app().await;
    let text = genuine(&base, "/run_trajectory", &trajectory()).await;
    let stated = serde_json::from_str::<Value>(&text).unwrap()["receipt_sha384"].clone();

    // One byte changed anywhere else (a letter of the run seal): matches false,
    // and the receipt as stated is still the original.
    let seal = serde_json::from_str::<Value>(&text).unwrap()["run_seal"].as_str().unwrap().to_string();
    let flipped_seal = format!("{}{}", if seal.starts_with('0') { '1' } else { '0' }, &seal[1..]);
    let changed = text.replacen(&seal, &flipped_seal, 1);
    assert_ne!(changed, text);
    let report = checked(&base, &changed).await;
    assert_eq!(report["matches"], false, "one byte: {report}");
    assert_eq!(report["receipt_sha384"], stated);

    // A key moved: `engine_version` taken from its place and put last.
    let version = r#""engine_version":"genesis_kernel_v0.13.1""#;
    assert!(text.contains(&format!("{version},")));
    let (document, tail) = split_tail(&text);
    let moved = format!("{},{version}{tail}", document.replacen(&format!("{version},"), "", 1));
    assert_ne!(moved, text);
    assert_eq!(moved.len(), text.len());
    let report = checked(&base, &moved).await;
    assert_eq!(report["matches"], false, "key moved: {report}");

    // Whitespace added inside the document: matches false.
    let spaced = text.replacen("\"inputs\":", "\"inputs\": ", 1);
    let report = checked(&base, &spaced).await;
    assert_eq!(report["matches"], false, "whitespace: {report}");

    // The receipt hex changed (last digit): the tail is still in form, so the
    // check runs and matches is false. The recomputed hash is the true one.
    let hex_end = text.len() - 2;
    let last = text.as_bytes()[hex_end - 1] as char;
    let other = if last == 'a' { 'b' } else { 'a' };
    let forged = format!("{}{other}\"}}", &text[..hex_end - 1]);
    let report = checked(&base, &forged).await;
    assert_eq!(report["matches"], false, "receipt hex: {report}");
    assert_eq!(report["recomputed_sha384"], stated);
    assert_ne!(report["receipt_sha384"], stated);

    // Whitespace after the closing brace leaves the form: refused, 422.
    for tail in ["\n", " "] {
        let body = json!({ "response_text": format!("{text}{tail}") });
        assert_eq!(refused(&base, &body).await, TAIL_REFUSAL);
    }
}

// R4
#[tokio::test]
async fn a_text_with_no_receipt_in_form_and_a_malformed_request_are_refused() {
    let base = spawn_app().await;
    let text = genuine(&base, "/solve", &solve()).await;
    let (document, tail) = split_tail(&text);

    let upper = format!("{document}{}", tail.to_ascii_uppercase().replace("RECEIPT_SHA256", "receipt_sha384"));
    let short = format!("{document}{}\"}}", &tail[..tail.len() - 3]);
    for (label, bad) in [
        ("no receipt", format!("{document}}}")),
        ("uppercase hex", upper),
        ("63 hex digits", short),
        ("empty", String::new()),
        ("not a response", "hello".to_string()),
    ] {
        let body = json!({ "response_text": bad });
        assert_eq!(refused(&base, &body).await, TAIL_REFUSAL, "{label}");
    }
    assert_eq!(
        refused(&base, &json!({ "response_text": text, "extra": 1 })).await,
        "Physical Violation: 'extra' is not a field of this request."
    );
    assert_eq!(
        refused(&base, &json!({ "extra": 1 })).await,
        "Physical Violation: 'extra' is not a field of this request."
    );
    assert_eq!(
        refused(&base, &json!({})).await,
        "Missing required parameter: 'response_text' (InputMissing)."
    );
    for wrong in [json!(5), json!(null), json!({ "a": 1 }), json!([text.clone()]), json!(true)] {
        assert_eq!(
            refused(&base, &json!({ "response_text": wrong })).await,
            "Physical Violation: 'response_text' must be a string.",
            "{wrong}"
        );
    }
    assert_eq!(
        refused(&base, &json!([1])).await,
        "Physical Violation: the request must be a JSON object."
    );

    let (status, repeated) = post_text(
        &base,
        "/verify_receipt",
        &format!(
            "{{\"response_text\":{},\"response_text\":{}}}",
            serde_json::to_string(&text).unwrap(),
            serde_json::to_string(&text).unwrap()
        ),
    )
    .await;
    assert_eq!(status, 422);
    assert_eq!(repeated, "Physical Violation: the key 'response_text' is repeated.");
}

// R5
#[tokio::test]
async fn a_response_with_a_receipt_made_by_hand_matches_so_the_receipt_is_not_proof_of_origin() {
    let base = spawn_app().await;
    let document = r#"{"inputs":{"made":"with no engine"},"engine_version":"genesis_kernel_v0.13.0","state":{"temperature_k":1.0}}"#;
    let receipt = sha384_hex(document.as_bytes());
    let wire = format!("{},\"receipt_sha384\":\"{receipt}\"}}", &document[..document.len() - 1]);
    let report = checked(&base, &wire).await;
    assert_eq!(report["matches"], true, "{report}");
    assert_eq!(report["receipt_sha384"], receipt);
    assert_eq!(report["does_not_show"], DOES_NOT_SHOW);
    // Since 0.14.4 the answer says what the match proves: the bytes only.
    let proves = report["match_proves"].as_str().unwrap();
    assert!(proves.starts_with("Only that the bytes before the receipt are unchanged"), "{proves}");
    assert!(proves.contains("anyone can write a matching receipt for any text"), "{proves}");
    assert!(proves.contains("needs engine_version_is_this_engines true and a replay"), "{proves}");
    // A receipt written by hand over an invented version, and over text that
    // is not JSON, matches too, and the answer says what that leaves open.
    let fake = r#"{"engine_version":"fake_v1.0","state":{"temperature_k":1.0}}"#;
    let wire = format!("{},\"receipt_sha384\":\"{}\"}}", &fake[..fake.len() - 1], sha384_hex(fake.as_bytes()));
    let report = checked(&base, &wire).await;
    assert_eq!((report["matches"].clone(), report["engine_version_is_this_engines"].clone()), (json!(true), json!(false)), "{report}");
    assert!(report["match_proves"].as_str().unwrap().contains("The text names an engine_version this engine does not write."), "{report}");
    let not_json = "not JSON {";
    let wire = format!("{not_json},\"receipt_sha384\":\"{}\"}}", sha384_hex(format!("{not_json}}}").as_bytes()));
    let report = checked(&base, &wire).await;
    assert_eq!(report["matches"], true, "{report}");
    assert!(report.get("engine_version").is_none(), "{report}");
    assert!(report["match_proves"].as_str().unwrap().contains("The text names no engine_version."), "{report}");
}

fn arguments(value: &Value) -> Map<String, Value> {
    value.as_object().expect("object").clone()
}

fn tool_text(result: &rmcp::model::CallToolResult) -> String {
    result.content[0].as_text().expect("text").text.clone()
}

// R6
#[tokio::test]
async fn the_tool_gives_the_bytes_of_the_route_and_the_list_has_sixteen_names() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let base = spawn_app().await;

    let listed = client.list_tools(None).await.expect("tools/list");
    let names: Vec<String> = listed.tools.iter().map(|tool| tool.name.to_string()).collect();
    assert_eq!(
        names,
        [
            "list_cards",
            "describe_card",
            "compute_step",
            "run_trajectory",
            "verify_tape",
            "list_solvers",
            "describe_solver",
            "solve",
            "verify_solution",
            "run_dataset",
            "verify_dataset",
            "verify_receipt",
            "validate_series",
            "cite",
            "find_model",
            "find_flip",
            "design_run",
            "verify_step",
        ]
    );
    assert_eq!(listed.tools.len(), 18);

    let text = genuine(&base, "/solve", &solve()).await;
    let altered = text.replacen("\"converged\"", "\"converge_\"", 1);
    assert_ne!(altered, text);
    let (document, _) = split_tail(&text);
    let cases = [
        ("a match", 200, json!({ "response_text": text })),
        ("a mismatch", 200, json!({ "response_text": altered })),
        ("no receipt", 422, json!({ "response_text": format!("{document}}}") })),
        ("an unknown key", 422, json!({ "response_text": text, "extra": 1 })),
        ("a missing key", 422, json!({})),
        ("a non-string", 422, json!({ "response_text": 5 })),
    ];
    for (label, status, body) in cases {
        let (http_status, http_text) = post(&base, "/verify_receipt", &body).await;
        assert_eq!(http_status, status, "{label}: {http_text}");
        let called = client
            .call_tool(CallToolRequestParams::new("verify_receipt").with_arguments(arguments(&body)))
            .await
            .expect("tool result");
        assert_eq!(called.is_error, Some(status != 200), "{label}");
        assert_eq!(tool_text(&called), http_text, "{label}");
    }
    let (_, mismatch) = post(&base, "/verify_receipt", &json!({ "response_text": altered })).await;
    assert_eq!(serde_json::from_str::<Value>(&mismatch).unwrap()["matches"], false);

    // The body limit is 16 MiB, as for the other wide routes.
    let big = "x".repeat(3 * 1024 * 1024);
    let (status, answer) = post(&base, "/verify_receipt", &json!({ "response_text": big })).await;
    assert_eq!(status, 422, "{answer}");
    assert_eq!(answer, TAIL_REFUSAL);

    client.close().await.expect("close");
}
