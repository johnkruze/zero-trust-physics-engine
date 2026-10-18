//! The open door, `hosted_door --open`, over real sockets against the real
//! binary: it starts only as open or as keyed; every card and solver the
//! engine serves is listed and answered with the bytes of the local
//! connection; it asks for no key and never offers a sign-in; each of its
//! limits is answered by name; what the three kinds of client send is
//! answered as the Model Context Protocol says; and the pages state the
//! values in force.

mod hosted_support;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use hosted_support::*;
use protocol_mcp_gateway::hosted;
use serde_json::{Value, json};

struct Open {
    child: Child,
    port: u16,
    banner: String,
}

impl Open {
    fn start(arguments: &[&str]) -> Open {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .args(["--open", "--port", "0"])
            .args(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut banner = String::new();
        for _ in 0..3 {
            out.read_line(&mut banner).unwrap();
        }
        let address = banner.split("running on ").nth(1).unwrap_or_else(|| panic!("no address in {banner:?}")).split(' ').next().unwrap();
        assert!(address.starts_with("127.0.0.1:"), "{address}");
        let port = address.rsplit(':').next().unwrap().parse().unwrap();
        Open { child, port, banner }
    }

    /// One request with the headers given, on a connection of its own.
    fn send(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> Reply {
        let mut head = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
        for (name, value) in headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        if method == "POST" {
            head.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        head.push_str("\r\n");
        head.push_str(body);
        parse_reply(&raw(self.port, head.as_bytes(), 120))
    }

    fn post_as(&self, address: Option<&str>, body: &str) -> Reply {
        let mut headers = vec![("Content-Type", "application/json"), ("Accept", "application/json, text/event-stream")];
        if let Some(address) = address {
            headers.push(("X-Client", address));
        }
        self.send("POST", "/mcp", &headers, body)
    }

    fn post(&self, body: &str) -> Reply {
        self.post_as(None, body)
    }

    /// `(is_error, text)` of a tool call.
    fn tool(&self, tool: &str, arguments: &Value) -> (bool, String) {
        let reply = self.post(&tool_body(1, tool, arguments));
        assert_eq!(reply.status, 200, "{reply:?}");
        tool_result(&reply)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

const PING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;

fn header<'a>(reply: &'a Reply, name: &str) -> Option<&'a str> {
    reply.head.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

fn refused_at_start(arguments: &[&str], needle: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_hosted_door")).args(arguments).output().unwrap();
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(2), "{arguments:?}: {said}");
    assert!(said.starts_with("The hosted door did not start: "), "{said}");
    assert!(said.contains(needle), "{arguments:?} said: {said}");
    assert!(output.stdout.is_empty());
}

// ------------------------------------------------------------ starting

#[test]
fn the_door_starts_open_or_keyed_and_never_both() {
    let keys = scratch("open_keys").join("keys.json");
    std::fs::write(&keys, json!([client_entry("a", "k", 2, 1)]).to_string()).unwrap();
    let keys = keys.to_str().unwrap();
    let dir = scratch("open_dir");
    refused_at_start(&["--open", "--keys", keys, "--port", "0"], "--open and --keys cannot be given together");
    refused_at_start(&["--keys", keys, "--open", "--port", "0"], "--open and --keys cannot be given together");
    refused_at_start(&["--port", "0"], "--keys is required");
    refused_at_start(&["--open", "--open", "--port", "0"], "--open was given more than once");
    refused_at_start(&["--open"], "--port is required");
    // Nothing is stored without a key.
    refused_at_start(&["--open", "--port", "0", "--dataset-dir", dir.to_str().unwrap()], "--dataset-dir cannot be given with --open");
    // The open door's arguments belong to the open door.
    for flag in ["--client-address-header", "--requests-per-minute", "--max-in-flight", "--max-in-flight-per-address", "--physics-permits", "--permits-per-address", "--request-seconds"] {
        let value = if flag == "--client-address-header" { "X-Client" } else { "2" };
        refused_at_start(&["--keys", keys, "--port", "0", flag, value], "is an argument of the open door and needs --open");
    }
    refused_at_start(&["--open", "--port", "0", "--requests-per-minute", "0"], "--requests-per-minute needs a whole number from 1 to 1000000");
    refused_at_start(&["--open", "--port", "0", "--requests-per-minute", "2", "--requests-per-minute", "3"], "given more than once");
    refused_at_start(&["--open", "--port", "0", "--client-address-header", "not a header"], "needs the name of an HTTP header");
    refused_at_start(&["--open", "--port", "0", "--physics-permits", "1"], "--physics-permits is 1, fewer than 2");
    refused_at_start(&["--open", "--port", "0", "--max-in-flight", "2", "--max-in-flight-per-address", "3"], "more than the 2 of --max-in-flight");
    // How many permits one address holds is an argument too: a whole number, and no more than the pool.
    refused_at_start(&["--open", "--port", "0", "--permits-per-address", "0"], "--permits-per-address needs a whole number from 1 to 32");
    refused_at_start(&["--open", "--port", "0", "--physics-permits", "2", "--permits-per-address", "3"], "--permits-per-address is 3, more than the 2 of --physics-permits");

    // Started open, it says so plainly, with its limits and where it reads an address.
    let door = Open::start(&[]);
    let first = door.banner.lines().next().unwrap();
    assert!(first.contains("OPEN: no key is asked. Anyone who has the address reaches every card and every solver the engine serves"), "{first}");
    assert!(door.banner.contains("at most 300 a minute from one address"), "{}", door.banner);
    assert!(door.banner.contains("A caller's address is the address of the socket."), "{}", door.banner);
    let named = Open::start(&["--client-address-header", "CF-Connecting-IP"]);
    assert!(named.banner.contains("read from the 'cf-connecting-ip' header, which the front in front of this door must overwrite"), "{}", named.banner);
}

/// How many physics permits one address holds is read at start, and an agent
/// is told the value in force wherever it reads the other limits: in the
/// instructions and in the description of each tool that computes.
#[test]
fn the_permits_one_address_holds_are_read_at_start_and_stated_from_the_value() {
    let hello = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string();
    for (arguments, held, of_pool) in [
        (&["--physics-permits", "4"][..], "1 physics permit", "at most 1 of the 4 physics permits"),
        (&["--physics-permits", "4", "--permits-per-address", "3"][..], "3 physics permits", "at most 3 of the 4 physics permits"),
    ] {
        let door = Open::start(arguments);
        let at_most = of_pool.split(' ').nth(2).unwrap();
        assert!(door.banner.contains(&format!("The physics pool has 4 permits and one address holds at most {at_most} at a time (permits-per-address).")), "{}", door.banner);
        let opened: Value = serde_json::from_str(&door.post(&hello).body).unwrap();
        let instructions = opened["result"]["instructions"].as_str().unwrap();
        let tasks = if at_most == "1" { "1 physics task".to_string() } else { format!("{at_most} physics tasks") };
        assert!(instructions.contains(&format!("; {tasks} at a time;")) && !instructions.contains(of_pool), "{instructions}");
        let listed: Value = serde_json::from_str(&door.post(LIST).body).unwrap();
        for name in ["verify_step", "run_trajectory", "solve", "validate_series", "verify_tape", "verify_solution", "run_dataset", "verify_dataset", "find_flip", "design_run"] {
            let tool = listed["result"]["tools"].as_array().unwrap().iter().find(|tool| tool["name"] == name).unwrap();
            let said = tool["description"].as_str().unwrap();
            assert!(said.contains(&format!("ne address holds at most {held} at a time")), "{name}: {said}");
            assert!(said.len() <= 2_000, "{name}: {}", said.len());
        }
    }
}

/// A door whose starter has closed its output before the banner is written
/// goes on serving: the banner is written in one piece, and a write nobody
/// reads is let go.
#[test]
fn the_door_serves_when_its_starter_has_closed_its_output() {
    for _ in 0..5 {
        // A port that is free now; another process may take it before the door does, and then the next is tried.
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .args(["--open", "--port", &port.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Closed before the door has bound its port and has anything to write.
        drop(child.stdout.take());
        let mut healthy = false;
        let mut exited = None;
        for _ in 0..200 {
            exited = child.try_wait().unwrap();
            if exited.is_some() {
                break;
            }
            if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let _ = stream.write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
                let mut answer = String::new();
                let _ = stream.read_to_string(&mut answer);
                if answer.starts_with("HTTP/1.1 200") {
                    healthy = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let mut said = String::new();
        if exited.is_some() {
            let _ = child.stderr.take().unwrap().read_to_string(&mut said);
            if said.contains("it cannot listen on") {
                continue;
            }
        }
        // It still answers a tool call after the write nobody read.
        let stepped = healthy.then(|| parse_reply(&raw(port, &request_bytes(None, &tool_body(1, "verify_step", &thermal_step())), 60)));
        let _ = child.kill();
        let _ = child.wait();
        assert!(healthy, "the door stopped ({exited:?}): {said}");
        assert!(tool_result(&stepped.unwrap()).1.contains("sha256_seal"));
        return;
    }
    panic!("no free port was found for the door in five tries");
}

// ------------------------------------------- everything, and the same bytes

#[test]
fn every_card_and_solver_is_served_with_the_bytes_of_the_local_connection() {
    let door = Open::start(&[]);
    let mut local = Stdio2::start(None);

    let (_, cards_text) = local.tool("list_cards", &json!({}));
    let cards: Value = serde_json::from_str(&cards_text).unwrap();
    let pairs: Vec<(String, String)> = cards["cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| (card["domain"].as_str().unwrap().to_string(), card["card"].as_str().unwrap().to_string()))
        .collect();
    assert!(pairs.len() >= 19, "{}", pairs.len());

    let run = thermal_run(50, 293.15);
    let (_, run_answer) = local.tool("run_trajectory", &run);
    let dataset = json!({
        "domain": "energy", "card": "thermal",
        "plan": {"kind": "grid", "base": {"initial": {}, "dt_s": 0.1, "segments": [{"steps": 5, "load": {"ambient_k": 293.15}}], "record_every": 1},
                 "axes": [{"path": "initial.temperature_k", "values": [280.0, 290.0]}, {"path": "segments[0].load.heat_power_w", "values": [0.0, 5.0]}]},
        "store": "frames", "return": "inline", "answer": "full"
    });
    let (_, dataset_answer) = local.tool("run_dataset", &dataset);
    let mut calls: Vec<(&str, Value)> = vec![
        ("list_cards", json!({})),
        ("list_solvers", json!({})),
        ("describe_solver", json!({"solver": "atom_dft"})),
        ("solve", json!({"solver": "atom_dft", "parameters": {"atomic_number": 3}})),
        ("solve", json!({"solver": "atom_dft", "parameters": {"atomic_number": 99}})),
        ("verify_step", thermal_step()),
        ("run_trajectory", run.clone()),
        ("verify_tape", serde_json::from_str(&run_answer).unwrap()),
        ("verify_receipt", json!({"response_text": run_answer})),
        ("run_dataset", dataset),
        ("verify_dataset", serde_json::from_str(&dataset_answer).unwrap()),
        ("validate_series", json!({"run": run, "quantity": "temperature_k", "measurements": [{"step": 10, "value": 293.2}, {"step": 50, "value": 293.3}]})),
        ("describe_card", json!({"domain": "energy", "card": "nope"})),
        ("verify_step", json!({"domain": "energy", "card": "nope", "parameters": {}})),
        ("no_such_tool", json!({})),
    ];
    for (domain, card) in &pairs {
        calls.push(("describe_card", json!({"domain": domain, "card": card})));
    }
    // Every solver the engine lists: its description, and the worked example
    // of that description as a request, which is a solve with its proof.
    let (_, solvers_text) = local.tool("list_solvers", &json!({}));
    let solvers: Value = serde_json::from_str(&solvers_text).unwrap();
    let solver_names: Vec<String> = solvers["solvers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|solver| solver["solver"].as_str().unwrap().to_string())
        .collect();
    assert!(solver_names.len() >= 12, "{solver_names:?}");
    for name in &solver_names {
        let which = json!({"solver": name});
        let (_, described) = local.tool("describe_solver", &which);
        let described: Value = serde_json::from_str(&described).unwrap();
        calls.push(("describe_solver", which));
        calls.push(("solve", described["example_solve"].clone()));
    }
    let mut sealed = 0;
    for (tool, arguments) in &calls {
        let here = door.tool(tool, arguments);
        let there = local.tool(tool, arguments);
        assert_eq!(here, there, "{tool} {arguments:.200}");
        if here.1.contains("receipt_sha256") && !here.0 {
            sealed += 1;
        }
    }
    assert!(sealed >= 5 + solver_names.len(), "{sealed} answers carried a receipt");

    // The list of tools: the engine's seventeen. The catalogue is left to
    // list_cards and list_solvers, which name every card and solver.
    let reply = door.post(LIST);
    let listed: Value = serde_json::from_str(&reply.body).unwrap();
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 18);
    let catalogue = format!("{}{}", door.tool("list_cards", &json!({})).1, door.tool("list_solvers", &json!({})).1);
    for (_, card) in &pairs {
        assert!(catalogue.contains(&format!("\"{card}\"")), "{card} is not listed");
    }
    for name in &solver_names {
        assert!(catalogue.contains(&format!("\"{name}\"")), "{name} is not listed");
    }
    // Without the open connection's own words and hints, the list is the local connection's, byte for byte,
    // but for what this connection does not serve and so does not describe: a dataset stored as files.
    let local_list = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map(|mut child| {
            let mut stdin = child.stdin.take().unwrap();
            let mut out = BufReader::new(child.stdout.take().unwrap());
            let mut line = String::new();
            stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":{\"name\":\"t\",\"version\":\"1\"}}}\n").unwrap();
            out.read_line(&mut line).unwrap();
            stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n").unwrap();
            line.clear();
            stdin.write_all(LIST.as_bytes()).unwrap();
            stdin.write_all(b"\n").unwrap();
            out.read_line(&mut line).unwrap();
            let _ = child.kill();
            let _ = child.wait();
            serde_json::from_str::<Value>(&line).unwrap()
        })
        .unwrap();
    let mut stripped = listed.clone();
    let mut local_list = local_list;
    let there = local_list["result"]["tools"].as_array_mut().unwrap();
    for (tool, there) in stripped["result"]["tools"].as_array_mut().unwrap().iter_mut().zip(there.iter_mut()) {
        let object = tool.as_object_mut().unwrap();
        assert_eq!(object["annotations"]["readOnlyHint"], json!(true));
        // Each tool carries its title twice: at the top, as revision 2025-06-18 has it, and among the hints.
        assert!(object["title"].is_string() && object["title"] == object["annotations"]["title"], "{object:?}");
        object.shift_remove("annotations");
        let description = object["description"].as_str().unwrap().to_string();
        assert!(!description.contains("\"file\"") && !description.contains("stored as files"), "{description}");
        let own = match description.find(" On this open connection") {
            Some(at) => {
                object.insert("description".to_string(), json!(description[..at]));
                description[..at].to_string()
            }
            None => description,
        };
        if ["run_dataset", "verify_dataset"].contains(&object["name"].as_str().unwrap()) {
            let local_said = there["description"].as_str().unwrap().to_string();
            let files = local_said.strip_prefix(own.as_str()).unwrap_or_else(|| panic!("{local_said}"));
            assert!(files.contains("return \"file\"") && files.matches(". ").count() == 0, "{files}");
            there["description"] = json!(own);
        }
    }
    assert_eq!(serde_json::to_string(&stripped).unwrap(), serde_json::to_string(&local_list).unwrap());
    // What this connection holds a tool to is inside that tool's description, with what a call that finds no permit is answered.
    let said = |name: &str| tools.iter().find(|tool| tool["name"] == name).unwrap()["description"].as_str().unwrap().to_string();
    for name in ["verify_step", "run_trajectory", "solve", "validate_series", "verify_tape", "verify_solution", "run_dataset", "verify_dataset", "find_flip", "design_run"] {
        let text = said(name);
        assert!(text.contains("ne address holds at most 1 physics permit at a time"), "{name}: {text}");
        assert!(text.contains("is answered \"Too many concurrent physics tasks.\" and nothing waits: the same call sent later is answered once one is free. An answer is waited for 60 seconds, and long work is stopped there."), "{name}: {text}");
    }
    for name in ["run_dataset", "verify_dataset"] {
        assert!(said(name).contains("On this open connection a dataset is returned inline and holds at most 300 runs (OPEN_MAX_DATASET_ROWS), 1000000 steps in all (OPEN_MAX_DATASET_STEPS), 10000 frames and an answer counted from the plan at 4194304 bytes (OPEN_MAX_DATASET_ANSWER_BYTES) and 40000 characters, up to about 140 summary rows;"), "{name}");
    }
    for name in ["list_cards", "describe_card", "list_solvers", "describe_solver", "verify_receipt", "cite", "find_model"] {
        assert!(!said(name).contains("On this open connection"), "{name}");
    }
}

// ------------------------------------------------------- no key, no sign-in

#[test]
fn no_key_is_asked_and_no_sign_in_is_ever_offered() {
    let door = Open::start(&[]);
    let json = ("Content-Type", "application/json");
    // With no key, with a key nobody issued, with two, with a scheme that is not Bearer: the same answer.
    let plain = door.send("POST", "/mcp", &[json], PING);
    assert_eq!((plain.status, plain.body.as_str()), (200, r#"{"jsonrpc":"2.0","id":1,"result":{}}"#));
    for headers in [
        vec![json, ("Authorization", "Bearer anything-at-all")],
        vec![json, ("Authorization", "Basic dXNlcjpwYXNz")],
        vec![json, ("Authorization", "Bearer a"), ("Authorization", "Bearer b")],
        vec![json, ("Authorization", "")],
    ] {
        let reply = door.send("POST", "/mcp", &headers, PING);
        assert_eq!((reply.status, reply.body.as_str()), (200, plain.body.as_str()), "{headers:?}");
    }
    // Where a client looks for a way to sign in there is nothing: a 404 with no body.
    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
        "/.well-known/oauth-authorization-server",
        "/.well-known/oauth-authorization-server/mcp",
        "/.well-known/openid-configuration",
        "/.well-known/openid-configuration/mcp",
        "/mcp/.well-known/openid-configuration",
        "/register",
        "/authorize",
        "/token",
        "/",
        "/sse",
    ] {
        let reply = door.send("GET", path, &[], "");
        assert_eq!((reply.status, reply.body.as_str()), (404, ""), "{path}: {reply:?}");
    }
    let reply = door.send("POST", "/register", &[json], "{}");
    assert_eq!(reply.status, 404);
    // No answer of this door carries a challenge, and none is a 401.
    let too_long = "x".repeat(hosted::OPEN_MAX_BODY_BYTES + 1);
    let replies = [
        door.send("POST", "/mcp", &[json], PING),
        door.send("GET", "/mcp", &[], ""),
        door.send("DELETE", "/mcp", &[], ""),
        door.send("POST", "/mcp", &[], PING),
        door.send("POST", "/mcp", &[json], "not json"),
        door.send("POST", "/mcp", &[json], &too_long),
        door.send("GET", "/.well-known/oauth-protected-resource", &[], ""),
        door.send("GET", "/health", &[], ""),
    ];
    for reply in &replies {
        assert_ne!(reply.status, 401, "{reply:?}");
        assert!(!reply.head.to_ascii_lowercase().contains("www-authenticate"), "{reply:?}");
    }
    assert_eq!(replies[7].status, 200);
}

// ------------------------------------------------------ the protocol

#[test]
fn what_a_remote_client_sends_is_answered_as_the_protocol_says() {
    let door = Open::start(&[]);
    let json = ("Content-Type", "application/json");
    let accept = ("Accept", "application/json, text/event-stream");
    let initialize = |version: &str| {
        json!({"jsonrpc":"2.0","id":"a","method":"initialize","params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string()
    };
    // initialize: a version the door speaks comes back; no session is made; the limits are in the instructions.
    let with_initialize = || hosted::SUPPORTED_VERSIONS.into_iter().filter(|version| hosted::begins_with_initialize(version));
    assert_eq!(with_initialize().count(), 4);
    for asked in with_initialize() {
        let reply = door.send("POST", "/mcp", &[json, accept], &initialize(asked));
        assert_eq!(reply.status, 200);
        assert_eq!(header(&reply, "content-type"), Some("application/json"));
        assert!(header(&reply, "mcp-session-id").is_none());
        let body: Value = serde_json::from_str(&reply.body).unwrap();
        assert_eq!(body["result"]["protocolVersion"], json!(asked));
        assert_eq!(body["result"]["serverInfo"]["name"], "Zero-Trust Physics");
        assert!(body["result"]["capabilities"]["tools"].is_object());
        let instructions = body["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("This open connection asks no key"), "{instructions}");
        assert!(instructions.contains("From one address: at most 300 requests a minute"), "{instructions}");
    }
    // An initialize settles its version in its body: one that asks for the revision with no
    // initialize, or for one the door does not speak, is offered the newest that has one.
    for asked in ["2026-07-28", "2031-01-01"] {
        for headers in [vec![json, accept], vec![json, accept, ("MCP-Protocol-Version", "2026-07-28")]] {
            let reply = door.send("POST", "/mcp", &headers, &initialize(asked));
            let body: Value = serde_json::from_str(&reply.body).unwrap_or_else(|_| panic!("{reply:?}"));
            assert_eq!(body["result"]["protocolVersion"], json!("2025-11-25"), "{reply:?}");
        }
    }
    // A header that names a revision the door does not speak is a 400 on every request, this one included.
    let reply = door.send("POST", "/mcp", &[json, accept, ("MCP-Protocol-Version", "2031-01-01")], &initialize("2031-01-01"));
    assert_eq!(reply.status, 400, "{reply:?}");
    assert!(reply.body.contains("-32022"), "{reply:?}");
    // A notification, and a response to a request the door never made: 202 with no body.
    for body in [
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}"#,
        r#"{"jsonrpc":"2.0","id":9,"result":{}}"#,
    ] {
        let reply = door.send("POST", "/mcp", &[json, accept, ("MCP-Protocol-Version", "2025-06-18")], body);
        assert_eq!((reply.status, reply.body.as_str()), (202, ""), "{body}: {reply:?}");
    }
    // ping, tools/list and tools/call under every version that begins with initialize, with the headers of a session ignored.
    for version in with_initialize() {
        let headers = [json, accept, ("MCP-Protocol-Version", version), ("Mcp-Session-Id", "abc"), ("Last-Event-ID", "7")];
        assert_eq!(door.send("POST", "/mcp", &headers, PING).body, r#"{"jsonrpc":"2.0","id":1,"result":{}}"#);
        let listed = door.send("POST", "/mcp", &headers, LIST);
        assert!(listed.status == 200 && listed.body.contains("\"run_trajectory\""), "{:.200}", listed.body);
        let called = door.send("POST", "/mcp", &headers, &tool_body(3, "verify_step", &thermal_step()));
        assert!(called.status == 200 && called.body.contains("sha256_seal"), "{:.200}", called.body);
        assert!(header(&called, "mcp-session-id").is_none());
    }
    // A request of revision 2026-07-28 needs no initialize before it: server/discover is answered
    // with the revisions the door speaks and the limits. `tests/hosted_revisions.rs` holds the rest.
    let discover = r#"{"jsonrpc":"2.0","id":"d","method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}"#;
    let reply = door.send("POST", "/mcp", &[json, accept, ("MCP-Protocol-Version", "2026-07-28"), ("Mcp-Method", "server/discover")], discover);
    assert_eq!(reply.status, 200, "{reply:?}");
    let found: Value = serde_json::from_str(&reply.body).unwrap();
    assert_eq!(found["result"]["supportedVersions"], json!(hosted::SUPPORTED_VERSIONS));
    assert!(found["result"]["instructions"].as_str().unwrap().contains("From one address: at most 300 requests a minute"), "{found}");
    // A request of a revision the door does not speak is a 400 with the list of those it does.
    let reply = door.send("POST", "/mcp", &[json, accept, ("MCP-Protocol-Version", "2031-01-01"), ("Mcp-Method", "tools/list")], LIST);
    assert_eq!(reply.status, 400, "{reply:?}");
    let refused: Value = serde_json::from_str(&reply.body).unwrap();
    assert_eq!((&refused["error"]["code"], &refused["error"]["data"]["supported"]), (&json!(-32022), &json!(hosted::SUPPORTED_VERSIONS)));
    // No event stream and no session to end.
    for method in ["GET", "DELETE", "PUT"] {
        let reply = door.send(method, "/mcp", &[("Accept", "text/event-stream")], "");
        assert_eq!(reply.status, 405, "{method}");
        assert_eq!(header(&reply, "allow"), Some("POST, OPTIONS"));
    }
    // A batch, a body that is not JSON-RPC, a wrong content type, a client that will not take JSON.
    assert_eq!(door.send("POST", "/mcp", &[json], "[]").status, 400);
    assert_eq!(door.send("POST", "/mcp", &[("Content-Type", "text/plain")], PING).status, 415);
    assert_eq!(door.send("POST", "/mcp", &[json, ("Accept", "text/event-stream")], PING).status, 406);
    let unknown = door.send("POST", "/mcp", &[json], r#"{"jsonrpc":"2.0","id":4,"method":"resources/list"}"#);
    assert!(unknown.body.contains("-32601"), "{unknown:?}");
    // A browser page: the preflight is answered, and the answer may be read from any origin.
    let preflight = door.send("OPTIONS", "/mcp", &[("Origin", "https://example.org"), ("Access-Control-Request-Method", "POST"), ("Access-Control-Request-Headers", "content-type,mcp-protocol-version")], "");
    assert_eq!(preflight.status, 204);
    assert_eq!(header(&preflight, "access-control-allow-origin"), Some("*"));
    assert!(header(&preflight, "access-control-allow-headers").unwrap().contains("mcp-protocol-version"));
    assert!(header(&preflight, "access-control-allow-methods").unwrap().contains("POST"));
    for origin in ["https://claude.ai", "https://chatgpt.com", "null"] {
        let reply = door.send("POST", "/mcp", &[json, accept, ("Origin", origin)], PING);
        assert_eq!(reply.status, 200, "{origin}");
        assert_eq!(header(&reply, "access-control-allow-origin"), Some("*"));
    }
    // Started with origins named, only those are served.
    let narrow = Open::start(&["--allowed-origin", "https://app.example"]);
    assert_eq!(narrow.send("POST", "/mcp", &[json, ("Origin", "https://app.example")], PING).status, 200);
    assert_eq!(narrow.send("POST", "/mcp", &[json, ("Origin", "https://other.example")], PING).status, 403);
    assert_eq!(narrow.send("POST", "/mcp", &[json], PING).status, 200);
}

#[tokio::test]
async fn a_standard_mcp_client_with_no_key_connects_lists_and_calls() {
    use rmcp::ServiceExt;
    use rmcp::model::CallToolRequestParams;
    use rmcp::transport::StreamableHttpClientTransport;
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

    let door = tokio::task::spawn_blocking(|| Open::start(&[])).await.unwrap();
    let config = StreamableHttpClientTransportConfig::with_uri(format!("http://127.0.0.1:{}/mcp", door.port));
    let client = ().serve(StreamableHttpClientTransport::from_config(config)).await.expect("connect");
    let info = client.peer_info().expect("what the server said of itself");
    assert_eq!(info.server_info.as_ref().unwrap().name, "Zero-Trust Physics");
    assert!(info.instructions.as_deref().unwrap_or("").contains("This open connection asks no key"));
    let listed = client.list_tools(None).await.expect("tools/list");
    assert_eq!(listed.tools.len(), 18);
    let schemas = serde_json::to_string(&listed.tools).unwrap();
    assert!(schemas.contains("list_cards") && schemas.contains("list_solvers") && !schemas.contains("ti64_lattice"));
    let called = client
        .call_tool(CallToolRequestParams::new("verify_step").with_arguments(thermal_step().as_object().unwrap().clone()))
        .await
        .expect("verify_step");
    assert_eq!(called.is_error, Some(false));
    assert!(called.content[0].as_text().unwrap().text.contains("sha256_seal"));
    client.cancel().await.expect("close");
}

// ------------------------------------------------------------ the limits

#[test]
fn requests_from_one_address_are_limited_by_the_minute_and_answered_by_name() {
    let door = Open::start(&["--client-address-header", "X-Client", "--requests-per-minute", "5"]);
    for _ in 0..5 {
        assert_eq!(door.post_as(Some("198.51.100.1"), PING).status, 200);
    }
    let sixth = door.post_as(Some("198.51.100.1"), PING);
    assert_eq!(sixth.status, 429, "{sixth:?}");
    assert_eq!(sixth.body, "Too many requests from this address: the open connection takes at most 5 a minute from one address (requests-per-minute). Send the next one in 12 seconds.");
    assert_eq!(header(&sixth, "retry-after"), Some("12"));
    // Every kind of request is counted, and a refused one costs the door nothing more.
    assert_eq!(door.send("GET", "/mcp", &[("X-Client", "198.51.100.1")], "").status, 429);
    // Another address has its own allowance, and so has the socket's address when the header is absent.
    assert_eq!(door.post_as(Some("198.51.100.2"), PING).status, 200);
    assert_eq!(door.post(PING).status, 200);
    // The address of an IPv6 caller is its network of 64 bits.
    for n in 0..5 {
        assert_eq!(door.post_as(Some(&format!("2001:db8:0:1::{n}")), PING).status, 200);
    }
    assert_eq!(door.post_as(Some("2001:db8:0:1:ffff::9"), PING).status, 429);
    assert_eq!(door.post_as(Some("2001:db8:0:2::1"), PING).status, 200);
    // A header that is not one address means the front appends to what the caller wrote: refused by name.
    for bad in ["203.0.113.9, 198.51.100.1", "unknown", "198.51.100.1:443", ""] {
        let reply = door.post_as(Some(bad), PING);
        assert_eq!(reply.status, 400, "{bad:?}");
        assert_eq!(reply.body, "Bad Request: the 'x-client' header must hold one address, written by the front that serves this connection (client-address-header).");
    }
    let twice = door.send("POST", "/mcp", &[("Content-Type", "application/json"), ("X-Client", "203.0.113.9"), ("X-Client", "198.51.100.1")], PING);
    assert_eq!(twice.status, 400);
    // The health check is not counted.
    for _ in 0..20 {
        assert_eq!(door.send("GET", "/health", &[("X-Client", "198.51.100.1")], "").status, 200);
    }

    // Without the flag no header is believed: a caller that names another address for itself
    // on every request is still one address, the socket's.
    let plain = Open::start(&["--requests-per-minute", "5"]);
    let mut served = 0;
    for n in 0..12 {
        let address = format!("203.0.113.{n}");
        let reply = plain.send("POST", "/mcp", &[("Content-Type", "application/json"), ("X-Client", &address), ("X-Forwarded-For", &address), ("CF-Connecting-IP", &address)], PING);
        served += usize::from(reply.status == 200);
    }
    assert_eq!(served, 5);
}

/// A connection that has sent its headers and part of its body: it holds a
/// place for a body until the door gives up on it.
fn stalled(port: u16, address: &str) -> TcpStream {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .write_all(format!("POST /mcp HTTP/1.1\r\nHost: x\r\nX-Client: {address}\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n{{\"jsonrpc\"").as_bytes())
        .unwrap();
    stream
}

#[test]
fn requests_in_flight_are_limited_for_one_address_and_for_the_door() {
    let door = Open::start(&["--client-address-header", "X-Client", "--max-in-flight", "3", "--max-in-flight-per-address", "2"]);
    let started = std::time::Instant::now();
    // Two bodies of one address are on their way: the address is at its limit, and others are served.
    let mut first = vec![stalled(door.port, "198.51.100.1"), stalled(door.port, "198.51.100.1")];
    std::thread::sleep(Duration::from_millis(300));
    let third = door.post_as(Some("198.51.100.1"), PING);
    assert_eq!(third.status, 429, "{third:?}");
    assert_eq!(third.body, "Too many requests in flight from this address: the open connection runs at most 2 at once from one address (max-in-flight-per-address). Send this one again when one has been answered.");
    assert_eq!(header(&third, "retry-after"), Some("1"));
    assert_eq!(door.post_as(Some("198.51.100.2"), PING).status, 200);
    // The door lets in four bodies for each request it may run: twelve here. Five more addresses with two
    // each make twelve on their way, and the next body, from anyone, is told so by name.
    let mut others = Vec::new();
    for n in 2..7 {
        others.push(stalled(door.port, &format!("198.51.100.{n}")));
        others.push(stalled(door.port, &format!("198.51.100.{n}")));
    }
    std::thread::sleep(Duration::from_millis(300));
    let fourth = door.post_as(Some("198.51.100.9"), PING);
    assert_eq!(fourth.status, 429, "{fourth:?}");
    assert_eq!(fourth.body, "Too many request bodies on their way: the open connection receives at most 12 at once (4 for each of max-in-flight). Send this one again.");
    assert_eq!(header(&fourth, "retry-after"), Some("1"));
    // The door gives up on a body after OPEN_BODY_READ_SECONDS, answers it 408, and the places come back.
    let mut answer = Vec::new();
    first[0].set_read_timeout(Some(Duration::from_secs(30))).unwrap();
    first[0].read_to_end(&mut answer).unwrap();
    let reply = parse_reply(&answer);
    assert_eq!(reply.status, 408, "{reply:?}");
    let waited = started.elapsed();
    assert!(waited >= Duration::from_secs(hosted::OPEN_BODY_READ_SECONDS) && waited < Duration::from_secs(hosted::OPEN_BODY_READ_SECONDS + 8), "{waited:?}");
    std::thread::sleep(Duration::from_millis(800));
    drop(others);
    assert_eq!(door.post_as(Some("198.51.100.1"), PING).status, 200);
    assert_eq!(door.post_as(Some("198.51.100.3"), PING).status, 200);
}

#[test]
fn a_body_over_the_limit_is_refused_by_name_and_one_at_the_limit_is_read() {
    let door = Open::start(&[]);
    let limit = hosted::OPEN_MAX_BODY_BYTES;
    let sentence = format!("Request too large: a request is at most {limit} bytes on the open connection (OPEN_MAX_BODY_BYTES). This one is longer and was not read.");
    // By its stated length: refused before a byte of the body is read, however long it says it is.
    for stated in [limit + 1, 16 * 1024 * 1024, usize::MAX / 4] {
        let head = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {stated}\r\n\r\n");
        let reply = parse_reply(&raw(door.port, head.as_bytes(), 10));
        assert_eq!(reply.status, 413, "{stated}: {reply:?}");
        let body: Value = serde_json::from_str(&reply.body).unwrap();
        assert_eq!(body["error"]["message"], json!(sentence));
        assert_eq!(body["id"], Value::Null);
    }
    // With no stated length (chunked), it is cut off at the limit, under the caller's id.
    let padding = "x".repeat(limit);
    let long = format!(r#"{{"jsonrpc":"2.0","id":77,"method":"ping","params":{{"pad":"{padding}"}}}}"#);
    let mut chunked = format!("POST /mcp HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n", long.len()).into_bytes();
    chunked.extend_from_slice(long.as_bytes());
    chunked.extend_from_slice(b"\r\n0\r\n\r\n");
    let reply = parse_reply(&raw(door.port, &chunked, 20));
    assert_eq!(reply.status, 413, "{:.300}", reply.head);
    assert!(reply.body.contains(&sentence) && reply.body.contains("\"id\":77"), "{:.300}", reply.body);
    // A body of exactly the limit is read and answered.
    let envelope = r#"{"jsonrpc":"2.0","id":78,"method":"ping","params":{"pad":""}}"#.len();
    let exact = format!(r#"{{"jsonrpc":"2.0","id":78,"method":"ping","params":{{"pad":"{}"}}}}"#, "x".repeat(limit - envelope));
    assert_eq!(exact.len(), limit);
    let reply = door.post(&exact);
    assert_eq!((reply.status, reply.body.as_str()), (200, r#"{"jsonrpc":"2.0","id":78,"result":{}}"#));
    // The largest saved answer of a validated run (2.25 MB, the cantilever's) fits back through for
    // a check: a tape of that size is read.
    let tape = json!({"inputs": {}, "frames": [], "outcome": "completed", "steps_landed": 0, "reject": null, "run_seal": "ab".repeat(32),
        "final_state": null, "final_residual_conservation_error_hex": null, "max_residual_conservation_error_hex": null, "state_digest": null,
        "padding": "x".repeat(2_400_000)});
    let reply = door.post(&tool_body(5, "verify_tape", &tape));
    assert_eq!(reply.status, 200, "{:.300}", reply.body);
}

/// A map of this crate, where it is on the disk. The maps are beside the
/// code and are not in its Git history, so a checkout of the code alone has
/// none: the pages are held to the code where they are present.
fn pages(names: &[&str]) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| std::fs::read_to_string(format!("{}/{name}", env!("CARGO_MANIFEST_DIR"))).ok())
        .collect()
}

#[test]
fn the_pages_state_the_limits_of_the_open_door_that_are_in_force() {
    let rules = hosted::OpenRules::for_this_machine();
    assert_eq!(
        (rules.requests_per_minute, rules.max_in_flight, rules.max_in_flight_per_address, rules.permits_per_address, rules.max_body_bytes, rules.request.as_secs()),
        (300, 16, 4, 1, 4_194_304, 60)
    );
    assert_eq!((rules.max_dataset_rows, rules.max_dataset_steps, rules.max_dataset_answer_bytes, rules.max_tracked_addresses), (300, 1_000_000, 4_194_304, 16_384));
    assert!(rules.physics_permits >= 2);
    assert_eq!(hosted::OPEN_CONNECTION_LIMITS.body_read.as_secs(), 10);
    for page in pages(&["AGENTS.md", "HOSTING.md"]) {
        for stated in [
            "OPEN_REQUESTS_PER_MINUTE` = 300",
            "OPEN_MAX_IN_FLIGHT` = 16",
            "OPEN_MAX_IN_FLIGHT_PER_ADDRESS` = 4",
            "OPEN_PHYSICS_PERMITS_AT_LEAST` = 2",
            "OPEN_PERMITS_PER_ADDRESS` = 1",
            "OPEN_LIGHT_RUN_STEPS` = 10000",
            "OPEN_MAX_BODY_BYTES` = 4194304",
            "OPEN_BODY_READ_SECONDS` = 10",
            "OPEN_REQUEST_SECONDS` = 60",
            "OPEN_MAX_DATASET_ROWS` = 300",
            "OPEN_MAX_DATASET_STEPS` = 1000000",
            "OPEN_MAX_DATASET_ANSWER_BYTES` = 4194304",
            "OPEN_MAX_TRACKED_ADDRESSES` = 16384",
        ] {
            assert!(page.contains(stated), "a page does not state {stated}");
        }
    }
    // The page that states how a dataset's answer is counted states the open door's own number.
    assert_eq!(hosted::OPEN_DATASET_ROW_BYTES, 4_000);
    for page in pages(&["HOSTING.md"]) {
        assert!(page.contains("`OPEN_DATASET_ROW_BYTES` = 4000 bytes for each run"));
    }
    // What the door prints at start, and what an agent reads when the connection opens, are written from the same values;
    // since 0.14.2 the opening leaves the busy answer and the wait to each tool that computes.
    let statement = rules.statement();
    for part in ["at most 300 a minute", "at most 16 in flight at once and 4 from one address", "a body of at most 4194304 bytes", "at most 1000000 steps and 10000 recorded states", "at most 300 runs", "1000000 steps in total", "one address holds at most 1 at a time (permits-per-address)", "waited for 60 seconds"] {
        assert!(statement.contains(part), "{part}: {statement}");
    }
    for part in ["and a call of a tool other than run_trajectory, verify_tape, run_dataset, verify_dataset, verify_receipt, validate_series at most 2097152 bytes, then HTTP 413 as well (NARROW_LIMIT, as on every connection)", "at most 40000 characters, as on every connection: a plan reaches that first, a grid on one axis at about 140 summary rows and a list sooner, its head repeating each run's request, so a plan of 300 runs is refused by it"] {
        assert!(statement.contains(part), "{part}: {statement}");
    }
    let opening = rules.said_at_opening();
    for part in ["This open connection asks no key and stores nothing. ", "From one address: at most 300 requests a minute and 4 in flight (16 in all), then HTTP 429 with Retry-After; 1 physics task at a time; a body of at most 4194304 bytes to run_trajectory, verify_tape, run_dataset, verify_dataset, verify_receipt or validate_series and 2097152 to any other tool, then HTTP 413."] {
        assert!(opening.contains(part), "{part}: {opening}");
    }
    assert!(opening.ends_with(" Each tool that computes states its own limits in its description.") && !opening.contains("waited for"), "{opening}");
    // The operator's page names the argument for each limit that has one.
    for page in pages(&["HOSTING.md"]) {
        for flag in ["`--requests-per-minute`", "`--max-in-flight`", "`--max-in-flight-per-address`", "`--physics-permits`", "`--permits-per-address`", "`--request-seconds`"] {
            assert!(page.contains(flag), "HOSTING.md does not name {flag}");
        }
    }
}
