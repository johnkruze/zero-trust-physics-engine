//! A check by request: verify_tape, verify_solution and verify_dataset given
//! the request as it was sent and the seal the caller was given. The request
//! is computed again and the seals compared. Nothing is stored, so the local,
//! web, keyed hosted and open hosted connections give the same bytes, and the
//! full-answer forms still answer as before.

mod hosted_support;
use hosted_support::answer_text;

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use hosted_support::*;
use serde_json::{Value, json};
use tokio::net::TcpListener;

const KEY: &str = "check-by-request-key";

struct Open {
    child: Child,
    port: u16,
}

impl Open {
    fn start() -> Open {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .args(["--open", "--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut banner = String::new();
        for _ in 0..3 {
            out.read_line(&mut banner).unwrap();
        }
        let address = banner.split("running on ").nth(1).unwrap().split(' ').next().unwrap();
        let port = address.rsplit(':').next().unwrap().parse().unwrap();
        Open { child, port }
    }

    fn tool(&self, tool: &str, arguments: &Value) -> (bool, String) {
        let reply = parse_reply(&raw(self.port, &request_bytes(None, &tool_body(1, tool, arguments)), 120));
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

async fn web() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, protocol_mcp_gateway::app()).await.unwrap();
    });
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, tool: &str, body: &Value) -> (u16, String) {
    let response = reqwest::Client::new()
        .post(format!("{base}/{tool}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(answer_text::as_sent(body))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    answer_text::remember(&text);
    (status, text)
}

fn run_request(ambient: Value) -> Value {
    json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 293.15 }, "dt_s": 1.0,
        "segments": [{ "steps": 60, "load": { "heat_power_w": 5.0, "ambient_k": ambient } }],
        "record_every": 20
    })
}

fn solve_request() -> Value {
    json!({ "solver": "atom_dft", "parameters": { "atomic_number": 1 } })
}

fn dataset_request() -> Value {
    json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "grid",
            "base": { "initial": {}, "dt_s": 1.0, "segments": [{ "steps": 30, "load": { "ambient_k": 293.15 } }], "record_every": 10 },
            "axes": [{ "path": "initial.temperature_k", "values": [283.15, 293.15] },
                     { "path": "segments[0].load.heat_power_w", "values": [0.0, 5.0] }] },
        "store": "frames", "return": "inline", "answer": "full"
    })
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| panic!("not JSON: {text}"))
}

fn flipped(seal: &str) -> String {
    let mut seal = seal.to_string();
    let last = if seal.ends_with('0') { "1" } else { "0" };
    seal.replace_range(63..64, last);
    seal
}

/// The checks, each as `(tool, arguments)`, from the answers the local door gave.
fn checks(stdio: &mut Stdio2) -> Vec<(&'static str, &'static str, Value)> {
    let (error, run) = stdio.tool("run_trajectory", &run_request(json!(310.0)));
    assert!(!error, "{run}");
    let (error, solved) = stdio.tool("solve", &solve_request());
    assert!(!error, "{solved}");
    let (error, dataset) = stdio.tool("run_dataset", &dataset_request());
    assert!(!error, "{dataset}");
    vec![
        ("verify_tape", "run_seal", json!({ "request": run_request(json!(310.0)), "run_seal": parsed(&run)["run_seal"] })),
        ("verify_solution", "solution_seal", json!({ "request": solve_request(), "solution_seal": parsed(&solved)["solution_seal"] })),
        ("verify_dataset", "dataset_seal", json!({ "request": dataset_request(), "dataset_seal": parsed(&dataset)["head"]["dataset_seal"] })),
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn a_check_by_request_gives_the_same_bytes_on_every_connection() {
    let mut stdio = Stdio2::start(None);
    let base = web().await;
    let keyed = Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY), "cards": [["energy", "thermal"]],
            "solvers": ["atom_dft"], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let open = Open::start();

    for (tool, seal_key, arguments) in checks(&mut stdio) {
        let given = arguments[seal_key].as_str().unwrap().to_string();
        let mut wrong = arguments.clone();
        wrong[seal_key] = json!(flipped(&given));
        for (label, arguments, holds) in [("genuine", &arguments, true), ("altered", &wrong, false)] {
            let (error, local) = stdio.tool(tool, arguments);
            assert!(!error, "{tool} {label}: {local}");
            let answer = parsed(&local);
            assert_eq!(answer["holds"], json!(holds), "{tool} {label}: {local}");
            // A check that does not hold hands back no seal.
            let recomputed = if holds { json!(given) } else { Value::Null };
            assert_eq!(answer[format!("recomputed_{seal_key}")], recomputed, "{tool} {label}: {local}");
            assert!(!holds || answer.get("not_held").is_none(), "{local}");
            assert!(holds || !local.contains(&given), "{tool} {label}: the seal came back: {local}");
            assert_eq!(answer["cause"], if holds { Value::Null } else { json!(seal_key) }, "{tool} {label}");
            assert!(answer["compared"].as_str().unwrap().contains("310 and 310.0"), "{local}");

            let (status, over_web) = post(&base, tool, arguments).await;
            assert_eq!((status, over_web.as_str()), (200, local.as_str()), "{tool} {label} web");
            let (status, error, hosted) = keyed.tool(KEY, tool, arguments);
            assert_eq!((status, error, hosted.as_str()), (200, false, local.as_str()), "{tool} {label} hosted");
            let (error, opened) = open.tool(tool, arguments);
            assert_eq!((error, opened.as_str()), (false, local.as_str()), "{tool} {label} open");
        }
    }
}

#[test]
fn a_run_and_a_solve_check_by_request_give_the_receipt_of_the_answer_sent_in_the_same_text() {
    let mut stdio = Stdio2::start(None);
    let (_, written_310_0) = stdio.tool("run_trajectory", &run_request(json!(310.0)));
    let (_, written_310) = stdio.tool("run_trajectory", &run_request(json!(310)));
    let (first, second) = (parsed(&written_310_0), parsed(&written_310));
    assert_eq!(first["run_seal"], second["run_seal"]);
    assert_ne!(first["receipt_sha256"], second["receipt_sha256"]);
    for (request, answer) in [(run_request(json!(310.0)), &first), (run_request(json!(310)), &second)] {
        let (_, text) = stdio.tool("verify_tape", &json!({ "request": request, "run_seal": first["run_seal"] }));
        let report = parsed(&text);
        assert_eq!(report["holds"], json!(true), "{text}");
        assert_eq!(report["recomputed_receipt_sha256"], answer["receipt_sha256"], "{text}");
    }

    let (_, solved) = stdio.tool("solve", &solve_request());
    let solved = parsed(&solved);
    let (_, text) = stdio.tool("verify_solution", &json!({ "request": solve_request(), "solution_seal": solved["solution_seal"] }));
    let report = parsed(&text);
    assert_eq!(report["holds"], json!(true), "{text}");
    assert_eq!(report["recomputed_receipt_sha256"], solved["receipt_sha256"], "{text}");
    let keys: Vec<&str> = report.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["holds", "cause", "recomputed_solution_seal", "recomputed_receipt_sha256", "compared", "text_not_compared"]);
}

#[test]
fn a_dataset_that_returned_files_is_checked_by_request_where_nothing_is_stored() {
    let dir = scratch("by_request_files");
    let mut storing = Stdio2::start(Some(&dir));
    let request = file_request(293.15);
    let (error, stored) = storing.tool("run_dataset", &request);
    assert!(!error, "{stored}");
    let seal = parsed(&stored)["head"]["dataset_seal"].clone();
    // A door with no directory, and the open door, compute the same seal.
    let mut plain = Stdio2::start(None);
    let (error, text) = plain.tool("verify_dataset", &json!({ "request": request, "dataset_seal": seal }));
    assert!(!error, "{text}");
    assert_eq!(parsed(&text)["holds"], json!(true), "{text}");
    let open = Open::start();
    assert_eq!(open.tool("verify_dataset", &json!({ "request": request, "dataset_seal": seal })), (false, text.clone()));
    // The same plan with its keys in another order is the same dataset.
    let (_, again) = plain.tool("verify_dataset", &json!({ "dataset_seal": seal, "request": reversed(&request) }));
    assert_eq!(parsed(&again)["holds"], json!(true), "{again}");
}

#[test]
fn a_check_by_request_is_refused_as_the_tool_that_computes_would_refuse_it() {
    let mut stdio = Stdio2::start(None);
    let seal = "ab".repeat(32);

    // A request the engine refuses before computing: the same sentence.
    let mut outside = run_request(json!(310.0));
    outside["initial"]["temperature_k"] = json!(1.0);
    let (error, computed) = stdio.tool("run_trajectory", &outside);
    assert!(error, "{computed}");
    assert_eq!(stdio.tool("verify_tape", &json!({ "request": outside, "run_seal": seal })), (true, computed));
    let unknown = json!({ "solver": "not_a_solver", "parameters": {} });
    let (error, computed) = stdio.tool("solve", &unknown);
    assert!(error, "{computed}");
    assert_eq!(stdio.tool("verify_solution", &json!({ "request": unknown, "solution_seal": seal })), (true, computed));
    let mut too_many = dataset_request();
    too_many["return"] = json!("sideways");
    let (error, computed) = stdio.tool("run_dataset", &too_many);
    assert!(error, "{computed}");
    assert_eq!(stdio.tool("verify_dataset", &json!({ "request": too_many, "dataset_seal": seal })), (true, computed));

    // The seal is held to its form, and the short form to its two keys.
    let (error, text) = stdio.tool("verify_tape", &json!({ "request": run_request(json!(310.0)), "run_seal": "AB".repeat(32) }));
    assert_eq!((error, text.as_str()), (true, "Physical Violation: 'run_seal' must be 64 lowercase hex digits."));
    let (error, text) = stdio.tool("verify_solution", &json!({ "request": solve_request() }));
    assert_eq!((error, text.as_str()), (true, "Missing required parameter: 'solution_seal' (InputMissing)."));
    let (error, text) = stdio.tool("verify_tape", &json!({ "request": run_request(json!(310.0)), "run_seal": seal, "frames": [] }));
    assert!(error);
    assert_eq!(text, "Physical Violation: 'frames' is not a field of a verification by request: send 'request' and 'run_seal' alone, or the whole run_trajectory answer without 'request'.");
    let (error, text) = stdio.tool("verify_dataset", &json!({ "request": dataset_request(), "dataset_seal": seal, "head": {} }));
    assert!(error);
    assert!(text.starts_with("Physical Violation: 'head' is not a field of a verification by request"), "{text}");

    // The request sent under the answer's own name is pointed to 'request'.
    let (error, text) = stdio.tool("verify_tape", &json!({ "inputs": run_request(json!(310.0)), "run_seal": seal }));
    assert!(error);
    assert!(text.starts_with("Missing required parameter: 'frames' (InputMissing). A verification by request sends the request under 'request'"), "{text}");
}

#[test]
fn the_verification_of_a_whole_solution_says_what_it_did_not_compare() {
    let mut stdio = Stdio2::start(None);
    let (_, solved) = stdio.tool("solve", &solve_request());
    let mut answer = parsed(&solved);
    let (_, text) = stdio.tool("verify_solution", &answer);
    let report = parsed(&text);
    assert_eq!(report["holds"], json!(true), "{text}");
    let compared = report["compared"].as_str().unwrap();
    // Since 0.11.0 the whole answer is compared, its decimals and its receipt
    // included; since 0.12.0 its request is solved again and the engine's
    // answer compared byte for byte, and the sentence says so.
    assert!(compared.starts_with("The solution seal and each of its parts were recomputed from the answer's exact hex values. Then its request, the answer's inputs, was solved again under a physics permit"), "{compared}");
    assert!(compared.contains("value by value, and then byte for byte, its words, decimals and receipt included"), "{compared}");
    // A decimal changed: the seal holds, and the decimal is named.
    let result = answer["result"].as_object_mut().unwrap();
    let name = result.keys().find(|key| !key.ends_with("_hex") && result[*key].is_f64()).unwrap().clone();
    result[&name] = json!(result[&name].as_f64().unwrap() * 2.0);
    let (_, text) = stdio.tool("verify_solution", &answer);
    let report = parsed(&text);
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!(format!("result.{name}"))), "{text}");
}

#[test]
fn the_descriptions_lead_with_the_short_form_and_name_the_receipt_rule() {
    let listed = stdio_tools();
    // Since 0.10.1 each opens with what the tool does, and the worked example,
    // whose length follows the worked request, is said at the request input.
    for (tool, lead) in [
        ("verify_tape", "Verifies the seal of a run. Short form"),
        ("verify_solution", "Verifies the seal of a solution. Short form"),
        ("verify_dataset", "Verifies the seal of a dataset. Short form"),
    ] {
        let text = listed[tool]["description"].as_str().unwrap();
        assert!(text.starts_with(lead), "{tool}: {text}");
        let at_request = listed[tool]["inputSchema"]["properties"]["request"]["description"].as_str().unwrap();
        assert_eq!(at_request.matches("Worked example of the short form, as the whole arguments: {\"request\":").count(), 1, "{tool}: {at_request}");
    }
    for tool in ["verify_receipt", "solve"] {
        assert!(listed[tool]["description"].as_str().unwrap().contains("310 and the same request sent with 310.0 give different receipts and the same seal"), "{tool}");
    }
}

fn stdio_tools() -> serde_json::Map<String, Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    use std::io::Write;
    let mut stdin = child.stdin.take().unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    stdin.write_all(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}).to_string().as_bytes()).unwrap();
    stdin.write_all(b"\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n").unwrap();
    stdin.flush().unwrap();
    out.read_line(&mut line).unwrap();
    line.clear();
    out.read_line(&mut line).unwrap();
    let _ = child.kill();
    let _ = child.wait();
    let listed: Value = serde_json::from_str(&line).unwrap();
    listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| (tool["name"].as_str().unwrap().to_string(), tool.clone()))
        .collect()
}

/// The long form holds the answer sent byte for byte, on the local, the web
/// and the open connection alike: the solve's answer as it came holds; the
/// same answer with its first decimal spelled another way (the same 64-bit
/// number), with its outcome written with a unicode escape, or with its
/// receipt worked out again over a re-spelled text does not, and each is
/// named where it differs. The three connections give the same bytes.
#[tokio::test]
async fn the_long_form_holds_the_bytes_sent_on_every_connection() {
    let base = web().await;
    let open = Open::start();
    let mut local = Stdio2::start(None);
    let (_, honest) = local.tool("solve", &json!({ "solver": "atom_dft", "parameters": { "atomic_number": 1 } }));
    let answer: Value = serde_json::from_str(&honest).unwrap();
    let value = answer["result"]["total_energy_ha"].as_f64().unwrap();
    let at = honest.find("\"total_energy_ha\":").unwrap() + "\"total_energy_ha\":".len();
    let end = at + honest[at..].find(',').unwrap();
    let respelled = format!("{}{:.25e}{}", &honest[..at], value, &honest[end..]);
    assert_ne!(respelled, honest);
    let escaped = honest.replacen("\"outcome\":\"converged\"", "\"outcome\":\"\\u0063onverged\"", 1);
    assert_ne!(escaped, honest);
    let resigned = {
        let tail = ",\"receipt_sha256\":\"";
        let cut = respelled.rfind(tail).unwrap();
        format!("{}{tail}{}\"}}", &respelled[..cut], sha(&format!("{}}}", &respelled[..cut])))
    };
    // Since values_hold: the first three hold every value, the last carries a receipt that is not the engine's.
    for (label, text, expected) in [
        ("as it came", honest.clone(), (json!(true), json!(true), Value::Null, Value::Null)),
        ("a decimal spelled another way", respelled, (json!(false), json!(true), json!("answer_text"), json!("result.total_energy_ha"))),
        ("the outcome with an escape", escaped, (json!(false), json!(true), json!("answer_text"), json!("outcome"))),
        ("re-spelled and its receipt worked out again", resigned, (json!(false), json!(false), json!("answer_text"), json!("receipt_sha256"))),
    ] {
        let (error, on_local) = local.tool_raw("verify_solution", &text);
        assert!(!error, "{label}: {on_local}");
        let on_web = reqwest::Client::new()
            .post(format!("{base}/verify_solution"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(text.clone())
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let body = format!("{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"verify_solution\",\"arguments\":{text}}}}}");
        let reply = parse_reply(&raw(open.port, &request_bytes(None, &body), 120));
        let (_, on_open) = tool_result(&reply);
        assert_eq!(on_local, on_web, "{label}");
        assert_eq!(on_local, on_open, "{label}");
        let report: Value = serde_json::from_str(&on_local).unwrap();
        assert_eq!((report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone(), report["differs_at"].clone()), expected, "{label}: {report}");
    }
}
