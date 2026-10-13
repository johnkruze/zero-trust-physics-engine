//! `GET /cards` and `POST /describe_card` against the MCP tools `list_cards`
//! and `describe_card`: the same text, byte for byte, and the same refusals.

use std::time::Duration;

use protocol_mcp_gateway::app;
use reqwest::Client;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
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

async fn send(request: reqwest::RequestBuilder) -> (u16, String) {
    let mut last_error = None;
    for _ in 0..20 {
        match request.try_clone().unwrap().send().await {
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
    panic!("never accepted a connection: {last_error:?}");
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    send(
        Client::new()
            .post(format!("{base}{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(body).unwrap()),
    )
    .await
}

fn arguments(value: &Value) -> Map<String, Value> {
    value.as_object().expect("object").clone()
}

fn tool_text(result: &rmcp::model::CallToolResult) -> String {
    result.content[0].as_text().expect("text").text.clone()
}

#[tokio::test]
async fn the_card_routes_give_the_bytes_of_the_tools_for_all_thirty_cards() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let base = spawn_app().await;

    let (status, http_list) = send(Client::new().get(format!("{base}/cards"))).await;
    assert_eq!(status, 200, "{http_list}");
    let listed = client
        .call_tool(CallToolRequestParams::new("list_cards"))
        .await
        .expect("list_cards");
    assert_eq!(listed.is_error, Some(false));
    assert_eq!(tool_text(&listed), http_list);

    let cards = serde_json::from_str::<Value>(&http_list).unwrap()["cards"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(cards.len(), 31);
    for entry in &cards {
        let which = json!({ "domain": entry["domain"], "card": entry["card"] });
        let (status, http_text) = post(&base, "/describe_card", &which).await;
        assert_eq!(status, 200, "{which}: {http_text}");
        let called = client
            .call_tool(CallToolRequestParams::new("describe_card").with_arguments(arguments(&which)))
            .await
            .expect("describe_card");
        assert_eq!(called.is_error, Some(false), "{which}");
        assert_eq!(tool_text(&called), http_text, "{which}");
    }

    client.close().await.expect("close");
}

#[tokio::test]
async fn the_card_routes_refuse_as_the_tools_do() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let base = spawn_app().await;

    let cases = [
        ("an unknown pair", 404, "Card not recognized", json!({ "domain": "energy", "card": "no_such_card" })),
        ("a pair with the domain wrong", 404, "Card not recognized", json!({ "domain": "signal", "card": "thermal" })),
        (
            "an unknown key",
            422,
            "Physical Violation: 'orchestrator' is not a field of this request.",
            json!({ "domain": "energy", "card": "thermal", "orchestrator": "thermal" }),
        ),
        (
            "an unknown key in place of card",
            422,
            "Physical Violation: 'orchestrator' is not a field of this request.",
            json!({ "domain": "energy", "orchestrator": "thermal" }),
        ),
        ("a missing key", 422, "Missing required parameter: 'card'", json!({ "domain": "energy" })),
        ("a non-string", 422, "Physical Violation: 'card' must be a string.", json!({ "domain": "energy", "card": 5 })),
    ];
    for (label, status, sentence, body) in cases {
        let (http_status, http_text) = post(&base, "/describe_card", &body).await;
        assert_eq!(http_status, status, "{label}: {http_text}");
        assert_eq!(http_text, sentence, "{label}");
        let called = client
            .call_tool(CallToolRequestParams::new("describe_card").with_arguments(arguments(&body)))
            .await
            .expect("tool result");
        assert_eq!(called.is_error, Some(true), "{label}");
        assert_eq!(tool_text(&called), http_text, "{label}");
    }

    let (status, repeated) = send(
        Client::new()
            .post(format!("{base}/describe_card"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(r#"{"domain":"energy","card":"thermal","card":"swing"}"#),
    )
    .await;
    assert_eq!(status, 422);
    assert_eq!(repeated, "Physical Violation: the key 'card' is repeated.");

    client.close().await.expect("close");
}
