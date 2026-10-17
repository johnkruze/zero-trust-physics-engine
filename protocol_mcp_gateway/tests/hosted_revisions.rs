//! The revisions of the Model Context Protocol the hosted connection speaks,
//! over real sockets against the real binary, with a key and without one:
//! each revision that begins with `initialize`, and revision 2026-07-28, which
//! has none and states its version in every request. For each: how a client
//! begins, `tools/list`, a `tools/call` with the bytes of the local
//! connection, the answer to a notification, `GET` and `DELETE`; then what a
//! request of a revision nobody speaks is answered, and that the two kinds of
//! door answer every one of these alike.

mod hosted_support;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use hosted_support::*;
use protocol_mcp_gateway::hosted;
use serde_json::{Value, json};

const KEY: &str = "key-that-sees-everything";
const NEWEST: &str = "2026-07-28";

/// A hosted door of one kind or the other.
enum Door {
    Open { child: Child, port: u16 },
    Keyed(Hosted),
}

impl Door {
    fn open() -> Door {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .args(["--open", "--port", "0", "--requests-per-minute", "100000"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // The open door's banner is three lines. All three are read before the
        // pipe is let go: a door still writing its banner into a closed pipe
        // stops with "Broken pipe", and the test is then refused a connection.
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut banner = String::new();
        for _ in 0..3 {
            out.read_line(&mut banner).unwrap();
        }
        let address = banner.split("running on ").nth(1).unwrap_or_else(|| panic!("no address in {banner:?}")).split(' ').next().unwrap();
        let port = address.rsplit(':').next().unwrap().parse().unwrap();
        Door::Open { child, port }
    }

    /// A door with one key, which sees every card and solver the engine serves.
    fn keyed(local: &mut Local) -> Door {
        let cards: Value = serde_json::from_str(&local.tool_text("list_cards", &json!({}))).unwrap();
        let pairs: Vec<Value> = cards["cards"].as_array().unwrap().iter().map(|card| json!([card["domain"], card["card"]])).collect();
        let solvers: Value = serde_json::from_str(&local.tool_text("list_solvers", &json!({}))).unwrap();
        let names: Vec<Value> = solvers["solvers"].as_array().unwrap().iter().map(|solver| solver["solver"].clone()).collect();
        Door::Keyed(Hosted::start(
            vec![json!({"client": "all", "key_sha256": sha(KEY), "cards": pairs, "solvers": names, "max_in_flight": 4, "max_permits": 1})],
            None,
        ))
    }

    fn name(&self) -> &'static str {
        match self {
            Door::Open { .. } => "open",
            Door::Keyed(_) => "keyed",
        }
    }

    fn port(&self) -> u16 {
        match self {
            Door::Open { port, .. } => *port,
            Door::Keyed(hosted) => hosted.port,
        }
    }

    /// One request on a connection of its own, with the headers given and,
    /// on the keyed door, the key.
    fn send(&self, method: &str, headers: &[(&str, &str)], body: &str) -> Reply {
        let mut head = format!("{method} /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
        if matches!(self, Door::Keyed(_)) {
            head.push_str(&format!("Authorization: Bearer {KEY}\r\n"));
        }
        for (name, value) in headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        if method == "POST" {
            head.push_str(&format!("Content-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\n", body.len()));
        }
        head.push_str("\r\n");
        head.push_str(body);
        parse_reply(&raw(self.port(), head.as_bytes(), 120))
    }

    /// A request of a revision that begins with `initialize`: the version in the header alone.
    fn legacy(&self, version: &str, body: &str) -> Reply {
        self.send("POST", &[("MCP-Protocol-Version", version)], body)
    }

    /// A request of revision 2026-07-28, as a client of it writes one: the
    /// version and the capabilities in `_meta`, and the three headers.
    fn modern(&self, id: Value, method: &str, params: Value) -> Reply {
        let body = modern_body(id, method, params.clone());
        let mut headers = vec![("MCP-Protocol-Version", NEWEST), ("Mcp-Method", method)];
        let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        if method == "tools/call" {
            headers.push(("Mcp-Name", &name));
        }
        self.send("POST", &headers, &body)
    }
}

impl Drop for Door {
    fn drop(&mut self) {
        if let Door::Open { child, .. } = self {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn modern_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": NEWEST,
        "io.modelcontextprotocol/clientInfo": {"name": "t", "version": "1"},
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

fn modern_body(id: Value, method: &str, mut params: Value) -> String {
    params["_meta"] = modern_meta();
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string()
}

/// The local connection, `mcp_stdio`, spoken to line by line. With a version
/// it begins with `initialize`; with none it is sent requests of revision
/// 2026-07-28 and nothing before them.
struct Local {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    next: u64,
}

impl Local {
    fn start(initialize: Option<&str>) -> Local {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut local = Local { child, stdin, out, next: 100 };
        if let Some(version) = initialize {
            let answer = local.line(&json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string());
            assert_eq!(answer["result"]["protocolVersion"], json!(version), "{answer}");
            local.stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n").unwrap();
        }
        local
    }

    fn line(&mut self, text: &str) -> Value {
        self.stdin.write_all(text.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).unwrap();
        serde_json::from_str(&answer).unwrap_or_else(|_| panic!("not JSON: {answer}"))
    }

    fn tool_text(&mut self, tool: &str, arguments: &Value) -> String {
        self.next += 1;
        let answer = self.line(&tool_body(self.next, tool, arguments));
        answer["result"]["content"][0]["text"].as_str().unwrap().to_string()
    }
}

impl Drop for Local {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn header<'a>(reply: &'a Reply, name: &str) -> Option<&'a str> {
    reply.head.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

fn body(reply: &Reply) -> Value {
    serde_json::from_str(&reply.body).unwrap_or_else(|_| panic!("not JSON: {reply:?}"))
}

/// The `result` of an answer, as the bytes it was sent in.
fn result_bytes(answer: &Value) -> String {
    assert!(answer.get("result").is_some(), "{answer:.400}");
    serde_json::to_string(&answer["result"]).unwrap()
}

/// The calls whose results are held against the local connection's: a step
/// with its proof, two descriptions, a refusal by a card, and a tool nobody has.
fn calls() -> Vec<(&'static str, Value)> {
    vec![
        ("verify_step", thermal_step()),
        ("run_trajectory", thermal_run(20, 293.15)),
        ("list_cards", json!({})),
        ("list_solvers", json!({})),
        ("describe_card", json!({"domain": "energy", "card": "thermal"})),
        ("verify_step", json!({"domain": "energy", "card": "nope", "parameters": {}})),
        ("no_such_tool", json!({})),
    ]
}

fn both(local: &mut Local) -> [Door; 2] {
    [Door::open(), Door::keyed(local)]
}

// ------------------------------------------- the revisions with initialize

#[test]
fn each_revision_that_begins_with_initialize_is_spoken_with_a_key_and_without() {
    let mut any = Local::start(Some("2025-06-18"));
    let doors = both(&mut any);
    let with_initialize: Vec<&str> = hosted::SUPPORTED_VERSIONS.iter().copied().filter(|version| hosted::begins_with_initialize(version)).collect();
    assert_eq!(with_initialize, ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"]);
    for version in with_initialize {
        let mut local = Local::start(Some(version));
        let local_list = local.line(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        for door in &doors {
            let at = format!("{} {version}", door.name());
            // The handshake: the version asked for comes back, and no session is made.
            let initialize = json!({"jsonrpc":"2.0","id":"a","method":"initialize","params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string();
            for reply in [door.send("POST", &[], &initialize), door.legacy(version, &initialize)] {
                assert_eq!(reply.status, 200, "{at}: {reply:?}");
                assert_eq!(header(&reply, "content-type"), Some("application/json"));
                assert!(header(&reply, "mcp-session-id").is_none());
                let answer = body(&reply);
                assert_eq!(answer["id"], json!("a"));
                assert_eq!(answer["result"]["protocolVersion"], json!(version), "{at}");
                assert_eq!(answer["result"]["serverInfo"]["name"], "Zero-Trust Physics");
                assert!(answer["result"]["capabilities"]["tools"].is_object());
                assert!(answer["result"].get("resultType").is_none(), "{at}");
            }
            // The notification that ends it: 202 with no body.
            let reply = door.legacy(version, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
            assert_eq!((reply.status, reply.body.as_str()), (202, ""), "{at}");
            // ping, and the list: seventeen tools, and with a key the local connection's list byte for byte.
            assert_eq!(door.legacy(version, r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).body, r#"{"jsonrpc":"2.0","id":1,"result":{}}"#, "{at}");
            let listed = body(&door.legacy(version, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#));
            assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 18, "{at}");
            for absent in ["resultType", "ttlMs", "cacheScope"] {
                assert!(listed["result"].get(absent).is_none(), "{at}: {absent}");
            }
            if matches!(door, Door::Keyed(_)) {
                assert_eq!(result_bytes(&listed), result_bytes(&local_list), "{at}");
            }
            // A call: the whole result is the local connection's, byte for byte.
            for (tool, arguments) in calls() {
                let reply = door.legacy(version, &tool_body(7, tool, &arguments));
                assert_eq!(reply.status, 200, "{at} {tool}");
                assert!(header(&reply, "mcp-session-id").is_none());
                let there = local.line(&tool_body(7, tool, &arguments));
                assert_eq!(reply.body, serde_json::to_string(&there).unwrap(), "{at} {tool}");
            }
            // No event stream to open and no session to end.
            for method in ["GET", "DELETE"] {
                let reply = door.send(method, &[("MCP-Protocol-Version", version), ("Accept", "text/event-stream"), ("Mcp-Session-Id", "abc")], "");
                assert_eq!(reply.status, 405, "{at} {method}");
                assert!(header(&reply, "allow").unwrap().contains("POST"));
            }
            // A method the door does not have, in the way of these revisions: the error, under 200.
            let reply = door.legacy(version, r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#);
            assert_eq!((reply.status, reply.body.as_str()), (200, r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"Method not found"}}"#), "{at}");
        }
    }
}

// ------------------------------------------------ the revision with none

#[test]
fn revision_2026_07_28_is_spoken_with_no_handshake_with_a_key_and_without() {
    let mut local = Local::start(None);
    let doors = both(&mut Local::start(Some("2025-06-18")));
    let local_found = local.line(&modern_body(json!("d"), "server/discover", json!({})));
    let local_list = local.line(&modern_body(json!(2), "tools/list", json!({})));
    let supported: Vec<Value> = hosted::SUPPORTED_VERSIONS.iter().map(|version| json!(version)).collect();
    assert_eq!(local_found["result"]["supportedVersions"], json!(supported), "the local connection publishes the same list");

    for door in &doors {
        let at = door.name();
        // No handshake: the first request a client sends may be any of these.
        // server/discover: the one list of revisions, what the server can do, and who it is.
        let reply = door.modern(json!("d"), "server/discover", json!({}));
        assert_eq!(reply.status, 200, "{at}: {reply:?}");
        assert_eq!(header(&reply, "content-type"), Some("application/json"));
        assert!(header(&reply, "mcp-session-id").is_none());
        let found = body(&reply);
        assert_eq!(found["id"], json!("d"));
        let result = &found["result"];
        assert_eq!(result["resultType"], json!("complete"));
        assert_eq!(result["supportedVersions"], json!(supported), "{at}");
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"], json!("Zero-Trust Physics"));
        assert_eq!(result["_meta"]["io.modelcontextprotocol/serverInfo"]["version"], json!(env!("CARGO_PKG_VERSION")));
        assert_eq!((&result["ttlMs"], &result["cacheScope"]), (&json!(0), &json!("private")));
        match door {
            // With a key it is the local connection's answer, byte for byte.
            Door::Keyed(_) => assert_eq!(result_bytes(&found), result_bytes(&local_found)),
            // Without one, the limits follow the local connection's instructions.
            Door::Open { .. } => {
                let instructions = result["instructions"].as_str().unwrap();
                assert!(instructions.starts_with(local_found["result"]["instructions"].as_str().unwrap()));
                assert!(instructions.contains("This open connection asks no key"), "{instructions}");
                let mut same = found.clone();
                same["result"]["instructions"] = local_found["result"]["instructions"].clone();
                assert_eq!(result_bytes(&same), result_bytes(&local_found));
            }
        }

        // tools/list: seventeen tools (sixteen and verify_step, the earlier name of compute_step), with the type of the result and the two hints the revision requires.
        let listed = body(&door.modern(json!(2), "tools/list", json!({})));
        let keys: Vec<&String> = listed["result"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["resultType", "ttlMs", "cacheScope", "tools"], "{at}");
        assert_eq!((&listed["result"]["resultType"], &listed["result"]["ttlMs"], &listed["result"]["cacheScope"]), (&json!("complete"), &json!(0), &json!("private")));
        assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 18);
        if matches!(door, Door::Keyed(_)) {
            assert_eq!(result_bytes(&listed), result_bytes(&local_list), "{at}");
        }
        // The tools are the same seventeen an earlier revision is given.
        let earlier = body(&door.legacy("2025-11-25", r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#));
        assert_eq!(listed["result"]["tools"], earlier["result"]["tools"], "{at}");

        // tools/call: the whole answer is the local connection's for the same request, byte for byte.
        for (tool, arguments) in calls() {
            let params = json!({"name": tool, "arguments": arguments});
            let reply = door.modern(json!(7), "tools/call", params.clone());
            assert_eq!(reply.status, 200, "{at} {tool}: {reply:?}");
            assert!(header(&reply, "mcp-session-id").is_none());
            let there = local.line(&modern_body(json!(7), "tools/call", params));
            assert_eq!(reply.body, serde_json::to_string(&there).unwrap(), "{at} {tool}");
            let here = body(&reply);
            assert_eq!(here["result"]["resultType"], json!("complete"), "{at} {tool}");
            // And its text is what a client of an earlier revision is given.
            let earlier = body(&door.legacy("2025-11-25", &tool_body(7, tool, &arguments)));
            assert_eq!(here["result"]["content"], earlier["result"]["content"], "{at} {tool}");
            assert_eq!(here["result"]["isError"], earlier["result"]["isError"], "{at} {tool}");
            assert!(earlier["result"].get("resultType").is_none());
        }
        // A tool name that cannot be a header as it stands goes as Base64, and is read.
        let odd = modern_body(json!(8), "tools/call", json!({"name": "no such tool", "arguments": {}}));
        let reply = door.send("POST", &[("MCP-Protocol-Version", NEWEST), ("Mcp-Method", "tools/call"), ("Mcp-Name", "=?base64?bm8gc3VjaCB0b29s?=")], &odd);
        assert_eq!(reply.status, 200, "{at}: {reply:?}");
        assert_eq!(body(&reply)["result"]["content"][0]["text"], json!("Tool not recognized: no such tool"));
        // A refusal a guard writes before the tool is a result too, and carries its type.
        let repeated = r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"list_cards","arguments":{"a":1,"a":2},"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}"#;
        let reply = door.send("POST", &[("MCP-Protocol-Version", NEWEST), ("Mcp-Method", "tools/call"), ("Mcp-Name", "list_cards")], repeated);
        assert_eq!(reply.body, r#"{"jsonrpc":"2.0","id":9,"result":{"resultType":"complete","content":[{"type":"text","text":"Physical Violation: the key 'a' is repeated."}],"isError":true}}"#, "{at}");
        assert_eq!(reply.body, serde_json::to_string(&local.line(repeated)).unwrap(), "{at}: the local connection writes the same");
        // The same refusal for a request of an earlier revision has no type, as before.
        let earlier = door.legacy("2025-11-25", r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"list_cards","arguments":{"a":1,"a":2}}}"#);
        assert_eq!(earlier.body, r#"{"jsonrpc":"2.0","id":9,"result":{"content":[{"type":"text","text":"Physical Violation: the key 'a' is repeated."}],"isError":true}}"#, "{at}");

        // A notification: the revision defines none from a client over HTTP, and one that comes is accepted.
        let reply = door.send("POST", &[("MCP-Protocol-Version", NEWEST)], r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":7}}"#);
        assert_eq!((reply.status, reply.body.as_str()), (202, ""), "{at}");
        // GET and DELETE: 405, as the revision tells a server that has no stream and no session.
        for method in ["GET", "DELETE"] {
            let reply = door.send(method, &[("MCP-Protocol-Version", NEWEST), ("Accept", "text/event-stream"), ("Mcp-Session-Id", "abc"), ("Last-Event-ID", "7")], "");
            assert_eq!(reply.status, 405, "{at} {method}");
            assert!(header(&reply, "allow").unwrap().contains("POST"));
        }
        // What the revision removed, and what the door never had: 404 with the error.
        for method in ["ping", "resources/list", "logging/setLevel", "subscriptions/listen"] {
            let reply = door.modern(json!(4), method, json!({}));
            assert_eq!((reply.status, reply.body.as_str()), (404, r#"{"jsonrpc":"2.0","id":4,"error":{"code":-32601,"message":"Method not found"}}"#), "{at} {method}");
        }
        // The headers of a session of an earlier revision are ignored, and nothing is kept
        // between requests: an earlier revision and this one are answered turn about.
        for _ in 0..2 {
            assert_eq!(door.legacy("2025-06-18", r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).status, 200);
            let reply = door.send("POST", &[("MCP-Protocol-Version", NEWEST), ("Mcp-Method", "tools/list"), ("Mcp-Session-Id", "abc"), ("Last-Event-ID", "7")], &modern_body(json!(5), "tools/list", json!({})));
            assert_eq!(reply.status, 200, "{at}");
            assert!(header(&reply, "mcp-session-id").is_none());
        }
    }
}

#[test]
fn a_request_that_states_its_revision_the_wrong_way_is_told_how_by_both_doors() {
    let doors = both(&mut Local::start(Some("2025-06-18")));
    let call = json!({"name": "list_cards", "arguments": {}});
    let listing = modern_body(json!(1), "tools/list", json!({}));
    let calling = modern_body(json!(1), "tools/call", call.clone());
    let version = ("MCP-Protocol-Version", NEWEST);
    let lists = ("Mcp-Method", "tools/list");
    let calls = ("Mcp-Method", "tools/call");
    // (headers, body, status, code, a part of the sentence)
    let cases: Vec<(Vec<(&str, &str)>, String, u16, i64, &str)> = vec![
        // The body does not state its revision, or what its client can do.
        (vec![version, lists], r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#.to_string(), 400, -32602, "io.modelcontextprotocol/protocolVersion and io.modelcontextprotocol/clientCapabilities"),
        (vec![version, lists], r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#.to_string(), 400, -32602, "wrong type for io.modelcontextprotocol/clientCapabilities."),
        (vec![version, lists], r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":[]}}}"#.to_string(), 400, -32602, "wrong type for io.modelcontextprotocol/clientCapabilities."),
        (vec![version, lists], r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":20260728,"io.modelcontextprotocol/clientCapabilities":{}}}}"#.to_string(), 400, -32602, "wrong type for io.modelcontextprotocol/protocolVersion."),
        // server/discover states them whatever the header says.
        (vec![("MCP-Protocol-Version", "2025-11-25"), ("Mcp-Method", "server/discover")], r#"{"jsonrpc":"2.0","id":1,"method":"server/discover"}"#.to_string(), 400, -32602, "every server/discover"),
        (vec![("Mcp-Method", "server/discover")], r#"{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}"#.to_string(), 400, -32602, "every server/discover"),
        // The headers do not say what the body says.
        (vec![lists], listing.clone(), 400, -32020, "the MCP-Protocol-Version header is missing; params._meta states revision '2026-07-28'."),
        (vec![("MCP-Protocol-Version", "2025-11-25"), lists], listing.clone(), 400, -32020, "the MCP-Protocol-Version header value '2025-11-25' does not match the revision '2026-07-28' in params._meta."),
        (vec![version, ("MCP-Protocol-Version", "2025-11-25"), lists], listing.clone(), 400, -32020, "given more than once with different values"),
        (vec![version], listing.clone(), 400, -32020, "the Mcp-Method header is missing; the body's method is 'tools/list'."),
        (vec![version, calls], listing.clone(), 400, -32020, "the Mcp-Method header value 'tools/call' does not match the method 'tools/list' in the body."),
        (vec![version, ("Mcp-Method", "Tools/List")], listing.clone(), 400, -32020, "does not match the method 'tools/list'"),
        (vec![version, calls], calling.clone(), 400, -32020, "the Mcp-Name header is missing; the body calls the tool 'list_cards'."),
        (vec![version, calls, ("Mcp-Name", "solve")], calling.clone(), 400, -32020, "the Mcp-Name header value 'solve' does not match the tool name 'list_cards' in the body."),
        (vec![version, calls, ("Mcp-Name", "=?base64?c29sdmU=?=")], calling.clone(), 400, -32020, "the Mcp-Name header value 'solve' does not match the tool name 'list_cards' in the body."),
        (vec![version, calls, ("Mcp-Name", "=?base64?***?=")], calling.clone(), 400, -32020, "written as Base64 and does not decode"),
        (vec![version, calls, ("Mcp-Name", "list_cards"), ("Mcp-Name", "solve")], calling.clone(), 400, -32020, "the Mcp-Name header is not text, or is given more than once"),
        // A call with no name is a call with wrong parameters, as in every revision.
        (vec![version, calls], modern_body(json!(1), "tools/call", json!({})), 200, -32602, "'name' is missing"),
    ];
    let mut answers: Vec<Vec<String>> = Vec::new();
    for door in &doors {
        let mut given = Vec::new();
        for (headers, sent, status, code, part) in &cases {
            let reply = door.send("POST", headers, sent);
            let at = format!("{} {headers:?} {sent:.120}", door.name());
            assert_eq!(reply.status, *status, "{at}: {reply:?}");
            assert_eq!(header(&reply, "content-type"), Some("application/json"), "{at}");
            let answer = body(&reply);
            assert_eq!(answer["id"], json!(1), "{at}");
            assert_eq!(answer["error"]["code"], json!(code), "{at}: {answer}");
            assert!(answer["error"]["message"].as_str().unwrap().contains(part), "{at}: {answer}");
            given.push(reply.body);
        }
        // The same request with headers that agree is answered.
        assert_eq!(door.send("POST", &[version, calls, ("Mcp-Name", "list_cards")], &calling).status, 200);
        // A header name is read whatever its case.
        assert_eq!(door.send("POST", &[("mcp-protocol-version", NEWEST), ("MCP-METHOD", "tools/list")], &listing).status, 200);
        answers.push(given);
    }
    assert_eq!(answers[0], answers[1], "the two doors answer alike");
}

// --------------------------------------------- a revision nobody speaks

#[test]
fn a_revision_the_door_does_not_speak_is_answered_alike_with_a_key_and_without() {
    let doors = both(&mut Local::start(Some("2025-06-18")));
    let supported: Vec<Value> = hosted::SUPPORTED_VERSIONS.iter().map(|version| json!(version)).collect();
    let refusal = |id: Value, requested: &str| {
        json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32022, "message": "Unsupported protocol version", "data": {"supported": supported, "requested": requested}}}).to_string()
    };
    let initialize = |version: &str| {
        json!({"jsonrpc":"2.0","id":"a","method":"initialize","params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string()
    };
    let future_meta = r#"{"_meta":{"io.modelcontextprotocol/protocolVersion":"2031-01-01","io.modelcontextprotocol/clientCapabilities":{}}}"#;
    let mut answers: Vec<Vec<String>> = Vec::new();
    for door in &doors {
        let at = door.name();
        let mut given = Vec::new();
        // The header names a revision the door does not speak: 400, the revision asked for and the
        // list of those spoken, under the request's id. So it is for a request of the newest kind,
        // for one of the earlier kind, for an initialize, for a notification and for a response.
        for requested in ["2031-01-01", "1999-01-01", "2024-10-07", "DRAFT-2026-v1", "latest"] {
            let header = [("MCP-Protocol-Version", requested), ("Mcp-Method", "tools/list")];
            for (sent, id) in [
                (format!(r#"{{"jsonrpc":"2.0","id":6,"method":"tools/list","params":{future_meta}}}"#), json!(6)),
                (format!(r#"{{"jsonrpc":"2.0","id":"d","method":"server/discover","params":{future_meta}}}"#), json!("d")),
                (r#"{"jsonrpc":"2.0","id":6,"method":"tools/list"}"#.to_string(), json!(6)),
                (r#"{"jsonrpc":"2.0","id":6,"method":"ping"}"#.to_string(), json!(6)),
                (initialize(requested), json!("a")),
                (initialize("2025-06-18"), json!("a")),
                (r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string(), Value::Null),
                (r#"{"jsonrpc":"2.0","id":9,"result":{}}"#.to_string(), Value::Null),
                // A body a guard refuses is still of a revision nobody speaks.
                (r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"list_cards","arguments":{"a":1,"a":2}}}"#.to_string(), json!(6)),
            ] {
                let reply = door.send("POST", &header, &sent);
                assert_eq!(reply.status, 400, "{at} {requested} {sent:.80}: {reply:?}");
                assert_eq!(header_of(&reply), Some("application/json"));
                assert_eq!(reply.body, refusal(id, requested), "{at} {requested} {sent:.80}");
                given.push(reply.body);
            }
        }
        // An initialize settles its version in its body: one that asks for a revision the door does
        // not speak, or for the one with no initialize, under no header or under one the door
        // speaks, is offered the newest revision that has an initialize.
        for asked in ["2031-01-01", "1999-01-01", NEWEST] {
            for headers in [vec![], vec![("MCP-Protocol-Version", "2025-11-25")], vec![("MCP-Protocol-Version", NEWEST)]] {
                let reply = door.send("POST", &headers, &initialize(asked));
                assert_eq!(reply.status, 200, "{at} {asked} {headers:?}: {reply:?}");
                let answer = body(&reply);
                assert_eq!(answer["result"]["protocolVersion"], json!("2025-11-25"), "{at} {asked} {headers:?}");
                assert!(answer["result"].get("resultType").is_none());
            }
        }
        // With no header a request is read as 2025-03-26, as the specification says.
        assert_eq!(door.send("POST", &[], r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#).body, r#"{"jsonrpc":"2.0","id":1,"result":{}}"#);
        answers.push(given);
    }
    assert_eq!(answers[0], answers[1], "the two doors answer alike");
}

fn header_of(reply: &Reply) -> Option<&str> {
    header(reply, "content-type")
}

// ------------------------------------------------------- a library client

/// The client of the Rust SDK in each of its three ways of beginning: with
/// `server/discover` and no way back, with `server/discover` and `initialize`
/// to fall back on, and with `initialize`.
#[tokio::test]
async fn a_library_client_connects_in_each_of_its_ways_to_both_doors() {
    use rmcp::model::{CallToolRequestParams, ProtocolVersion};
    use rmcp::service::{ClientLifecycleMode, ClientServiceExt};
    use rmcp::transport::StreamableHttpClientTransport;
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

    let doors = tokio::task::spawn_blocking(|| both(&mut Local::start(Some("2025-06-18")))).await.unwrap();
    let newest = vec![ProtocolVersion::V_2026_07_28];
    let ways = [
        (ClientLifecycleMode::Discover { preferred_versions: newest.clone() }, NEWEST),
        (ClientLifecycleMode::Auto { preferred_versions: newest, legacy_version: None }, NEWEST),
        (ClientLifecycleMode::Initialize, "2025-11-25"),
    ];
    for door in &doors {
        for (way, version) in &ways {
            let at = format!("{} {way:?}", door.name());
            let mut config = StreamableHttpClientTransportConfig::with_uri(format!("http://127.0.0.1:{}/mcp", door.port()));
            if matches!(door, Door::Keyed(_)) {
                config = config.auth_header(KEY);
            }
            let client = ()
                .serve_with_lifecycle(StreamableHttpClientTransport::from_config(config), way.clone())
                .await
                .unwrap_or_else(|error| panic!("{at}: {error:?}"));
            let info = client.peer_info().expect("what the server said of itself");
            assert_eq!(info.protocol_version.as_str(), *version, "{at}");
            assert_eq!(info.server_info.as_ref().unwrap().name, "Zero-Trust Physics", "{at}");
            let listed = client.list_tools(None).await.unwrap_or_else(|error| panic!("{at}: {error:?}"));
            assert_eq!(listed.tools.len(), 18, "{at}");
            let called = client
                .call_tool(CallToolRequestParams::new("verify_step").with_arguments(thermal_step().as_object().unwrap().clone()))
                .await
                .unwrap_or_else(|error| panic!("{at}: {error:?}"));
            assert_eq!(called.is_error, Some(false), "{at}");
            assert!(called.content[0].as_text().unwrap().text.contains("sha256_seal"), "{at}");
            client.cancel().await.expect("close");
        }
    }
}

// ------------------------------------------------------------- the pages

#[test]
fn the_operators_page_names_every_revision_the_door_speaks() {
    // The page is beside the code and not in its Git history: where it is on
    // the disk, it names every revision.
    if let Ok(page) = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/HOSTING.md")) {
        for version in hosted::SUPPORTED_VERSIONS {
            assert!(page.contains(version), "HOSTING.md does not name {version}");
        }
    }
    assert_eq!(hosted::FIRST_VERSION_WITHOUT_INITIALIZE, NEWEST);
    assert_eq!(hosted::SUPPORTED_VERSIONS.last(), Some(&NEWEST));
}
