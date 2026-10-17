//! A panic inside a tool call is answered by name on every connection, and
//! the connection goes on serving: the stdio door, the HTTP door, the hosted
//! door with a key and the open one. A test tool, `panic_for_test`, and a
//! test route, `/panic_for_test`, panic inside the engine's own code.
//!
//! Before 0.12.0 the stdio door called the tool in the library's own task,
//! and a panic there left the request unanswered for ever: the grasped
//! object's `describe_card`, with a flag that had no row in `flag_state`,
//! held the router tests of its session for twelve minutes. The last test
//! here calls a tool as 0.11.0 did and shows that.

use std::time::Duration;

use rmcp::ServiceExt;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::mcp_server::{INTERNAL_FAULT, McpGateway, PANIC_FOR_TEST};

/// The stdio door in this process: what is written to it, and the ids it
/// answers within `wait`, with each answer.
async fn stdio_answers(gateway: McpGateway, calls: &[(u64, &str)], wait: Duration) -> Vec<(u64, Value)> {
    let (mut client_write, server_read) = tokio::io::duplex(1 << 22);
    let (client_read, server_write) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = gateway.serve((server_read, server_write)).await {
            let _ = running.waiting().await;
        }
    });
    let mut lines = vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"fault","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    ];
    for (id, tool) in calls {
        lines.push(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":{}}}));
    }
    for line in lines {
        client_write.write_all(format!("{line}\n").as_bytes()).await.unwrap();
    }
    client_write.flush().await.unwrap();
    let mut reader = BufReader::new(client_read);
    let mut answered = Vec::new();
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        let mut line = String::new();
        match tokio::time::timeout_at(deadline, reader.read_line(&mut line)).await {
            Ok(Ok(read)) if read > 0 => {
                let reply: Value = serde_json::from_str(line.trim()).unwrap();
                if let Some(id) = reply["id"].as_u64().filter(|id| *id != 1) {
                    answered.push((id, reply));
                    if answered.len() == calls.len() {
                        break;
                    }
                }
            }
            _ => break,
        }
    }
    answered
}

fn text_of(reply: &Value) -> &str {
    reply["result"]["content"][0]["text"].as_str().unwrap_or("")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panic_inside_a_tool_call_on_the_stdio_door_is_answered_by_name_and_the_next_request_is_answered() {
    let answered = stdio_answers(McpGateway::new(), &[(2, PANIC_FOR_TEST), (3, "list_cards"), (4, PANIC_FOR_TEST), (5, "list_solvers")], Duration::from_secs(30)).await;
    let ids: Vec<u64> = answered.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids.len(), 4, "every request is answered: {ids:?}");
    for (id, reply) in &answered {
        match id {
            2 | 4 => {
                assert_eq!(reply["result"]["isError"], true, "{reply}");
                assert_eq!(text_of(reply), crate::mcp_server::internal_fault(PANIC_FOR_TEST), "{reply}");
                assert!(text_of(reply).contains(&format!("({INTERNAL_FAULT})")));
                assert!(!text_of(reply).contains("seal\""), "no proof: {reply}");
            }
            3 => assert!(text_of(reply).contains("\"cards\""), "{reply}"),
            5 => assert!(text_of(reply).contains("\"solvers\""), "{reply}"),
            _ => unreachable!(),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panicking_handler_of_the_http_door_is_answered_by_name_and_the_door_goes_on() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, crate::app()).await.unwrap() });
    let client = reqwest::Client::new();
    for _ in 0..2 {
        let panicked = client.post(format!("http://127.0.0.1:{port}/panic_for_test")).send().await.expect("the door answers");
        assert_eq!(panicked.status().as_u16(), 500);
        assert_eq!(panicked.text().await.unwrap(), crate::mcp_server::internal_fault("panic_for_test"));
        let listed = client.get(format!("http://127.0.0.1:{port}/cards")).send().await.unwrap();
        assert_eq!(listed.status().as_u16(), 200);
        assert!(listed.text().await.unwrap().contains("\"cards\""));
    }
}

/// The hosted door, with a key and open: a panic in a tool call is the
/// call's answer, by name, under its own id, and the next call is answered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panic_inside_a_tool_call_on_the_hosted_door_with_a_key_and_on_the_open_one_is_answered_by_name() {
    let key = "fault-key";
    let file = json!([{
        "client": "fault", "key_sha256": crate::keys::sha256_hex(key.as_bytes()),
        "cards": [["energy", "thermal"]], "solvers": [], "max_in_flight": 4, "max_permits": 2
    }]);
    let keyed = crate::hosted::router(crate::keys::Keys::parse(file.to_string().as_bytes()).unwrap(), None, Vec::new()).unwrap();
    let open = crate::open_door::router(crate::open_door::OpenRules::for_this_machine(), Vec::new()).unwrap();
    for (label, app, bearer) in [("keyed", keyed, Some(key)), ("open", open, None)] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await.unwrap() });
        let url = format!("http://127.0.0.1:{port}/mcp");
        let call = |id: u64, tool: &str| json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":{}}}).to_string();
        for (id, tool) in [(7, PANIC_FOR_TEST), (8, "list_cards"), (9, PANIC_FOR_TEST), (10, "list_solvers")] {
            let mut request = reqwest::Client::new()
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .header("MCP-Protocol-Version", "2025-06-18")
                .body(call(id, tool));
            if let Some(key) = bearer {
                request = request.header("Authorization", format!("Bearer {key}"));
            }
            let response = request.send().await.unwrap_or_else(|error| panic!("{label} {id}: {error}"));
            assert_eq!(response.status().as_u16(), 200, "{label} {id}");
            let reply: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
            assert_eq!(reply["id"], id, "{label}: {reply}");
            if tool == PANIC_FOR_TEST {
                assert_eq!(reply["result"]["isError"], true, "{label}: {reply}");
                assert_eq!(text_of(&reply), crate::mcp_server::internal_fault(PANIC_FOR_TEST), "{label}: {reply}");
            } else {
                assert!(reply["result"]["isError"] != true && text_of(&reply).contains("\"count\""), "{label}: {reply}");
            }
        }
    }
}

/// What 0.11.0 did: the tool called in the library's own task, with no
/// guard. The panicking call is never answered.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn as_engine_0_11_0_called_a_tool_a_panic_left_its_request_unanswered() {
    let mut unguarded = McpGateway::new();
    unguarded.unguarded = true;
    let answered = stdio_answers(unguarded, &[(2, PANIC_FOR_TEST), (3, "list_cards")], Duration::from_secs(5)).await;
    let ids: Vec<u64> = answered.iter().map(|(id, _)| *id).collect();
    assert!(!ids.contains(&2), "{ids:?}");
    println!("0.11.0's call path: answered {ids:?} of [2, 3] within 5 s");
}
