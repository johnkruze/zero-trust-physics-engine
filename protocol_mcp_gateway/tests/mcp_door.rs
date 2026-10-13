//! The first thing the MCP door does with a line, before the MCP library reads
//! it: the id of a request, the size of a line, and a tape written in capitals.
//! The binary is driven over its pipes, one line at a time.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::{Value, json};

const LINE_LIMIT: usize = 16 * 1024 * 1024;
const NARROW_LIMIT: usize = 2 * 1024 * 1024;

struct Door {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
}

impl Door {
    fn start() -> Door {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn mcp_stdio");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if send.send(line).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let mut door = Door { child, stdin, lines };
        door.send(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#,
        );
        let answer = door.next(30).expect("initialize is answered");
        assert_eq!(answer["id"], 1);
        door.send(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        door
    }

    fn send(&mut self, line: &[u8]) {
        self.stdin.write_all(line).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    fn next(&mut self, seconds: u64) -> Option<Value> {
        let line = self.lines.recv_timeout(Duration::from_secs(seconds)).ok()?;
        Some(serde_json::from_str(&line).expect("an answer is JSON"))
    }

    /// Send a line, then a sentinel behind it. Every answer that comes before
    /// the sentinel's, and the sentinel's, are returned.
    fn ask(&mut self, line: &[u8]) -> Vec<Value> {
        self.send(line);
        self.send(
            br#"{"jsonrpc":"2.0","id":"sentinel","method":"tools/call","params":{"name":"list_solvers","arguments":{}}}"#,
        );
        let mut answers = Vec::new();
        while let Some(answer) = self.next(60) {
            let last = answer["id"] == "sentinel";
            answers.push(answer);
            if last {
                // Let a slower answer to the line land as well.
                while let Some(more) = self.next(1) {
                    answers.push(more);
                }
                return answers;
            }
        }
        panic!("the door stopped answering");
    }
}

impl Drop for Door {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn list_solvers_with_id(id: &str) -> Vec<u8> {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"list_solvers","arguments":{{}}}}}}"#
    )
    .into_bytes()
}

fn not_the_sentinel(answers: &[Value]) -> Vec<&Value> {
    answers.iter().filter(|a| a["id"] != "sentinel").collect()
}

#[test]
fn a_request_with_an_id_the_library_would_drop_is_answered() {
    let mut door = Door::start();
    // An id that is a string or a whole number in range is carried as before.
    for id in ["0", "-1", "9223372036854775807", "-9223372036854775808", "\"abc\"", "\"\""] {
        let answers = door.ask(&list_solvers_with_id(id));
        let got = not_the_sentinel(&answers);
        assert_eq!(got.len(), 1, "id {id}");
        assert!(got[0].get("result").is_some(), "id {id}: {}", got[0]);
        assert_eq!(got[0]["id"], serde_json::from_str::<Value>(id).unwrap(), "id {id}");
    }
    // A number the library does not carry is answered under its own token.
    for id in [
        "1.5",
        "1e2",
        "1.0",
        "-0",
        "9223372036854775808",
        "18446744073709551615",
        "-9223372036854775809",
        "12345678901234567890123",
    ] {
        door.send(&list_solvers_with_id(id));
        let line = door.lines.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|_| panic!("id {id} got no answer"));
        assert!(line.starts_with(&format!(r#"{{"jsonrpc":"2.0","id":{id},"error":{{"code":-32600,"#)), "id {id}: {line}");
        assert!(line.contains("Invalid Request: the id"), "{line}");
    }
    // A value that is not an id is Invalid Request with a null id.
    for (id, what) in [
        ("null", "null"),
        ("true", "a boolean"),
        ("false", "a boolean"),
        ("[1]", "a list"),
        ("[]", "a list"),
        ("{}", "an object"),
        (r#"{"a":1}"#, "an object"),
    ] {
        door.send(&list_solvers_with_id(id));
        let line = door.lines.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|_| panic!("id {id} got no answer"));
        let answer: Value = serde_json::from_str(&line).unwrap();
        assert!(answer["id"].is_null(), "id {id}: {line}");
        assert_eq!(answer["error"]["code"], -32600, "id {id}");
        assert_eq!(
            answer["error"]["message"],
            format!("Invalid Request: the id is {what}. An id is a string or a whole number, and a message with no id is a notification.")
        );
    }
    // The id may come after the method, and a repeated key does not hide it.
    door.send(br#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list_solvers","arguments":{}},"id":[2]}"#);
    let answer = door.next(10).expect("answered");
    assert_eq!(answer["error"]["code"], -32600);
    // A message with no id is a notification and gets no answer.
    let answers = door.ask(br#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list_solvers","arguments":{}}}"#);
    assert!(not_the_sentinel(&answers).is_empty(), "{answers:?}");
}

fn receipt_line(id: &str, target: usize) -> Vec<u8> {
    let head = format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"verify_receipt","arguments":{{"response_text":""#
    );
    let tail = r#""}}}"#;
    let pad = target - head.len() - tail.len();
    let mut line = head.into_bytes();
    line.extend(std::iter::repeat_n(b'a', pad));
    line.extend_from_slice(tail.as_bytes());
    assert_eq!(line.len(), target);
    line
}

#[test]
fn a_line_over_16_mib_is_read_through_and_refused_and_the_door_goes_on() {
    let mut door = Door::start();
    // Exactly the limit is read and answered by the tool.
    let answers = door.ask(&receipt_line("5", LINE_LIMIT));
    let got = not_the_sentinel(&answers);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["id"], 5);
    assert_eq!(got[0]["result"]["isError"], true, "{}", got[0]);
    assert!(got[0]["result"]["content"][0]["text"].as_str().unwrap().contains("does not end with a receipt"));

    // One byte over is refused under the caller's id.
    let answers = door.ask(&receipt_line("6", LINE_LIMIT + 1));
    let got = not_the_sentinel(&answers);
    assert_eq!(got.len(), 1, "{answers:?}");
    assert_eq!(got[0]["id"], 6);
    assert_eq!(got[0]["error"]["code"], -32600);
    assert!(got[0]["error"]["message"].as_str().unwrap().starts_with("Request too large: a line is at most 16777216 bytes"));

    // Far over: 40 MiB. The next request is answered as if nothing happened.
    let answers = door.ask(&receipt_line("\"big\"", 40 * 1024 * 1024));
    let got = not_the_sentinel(&answers);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0]["id"], "big");
    assert_eq!(got[0]["error"]["code"], -32600);

    // An id after the cut cannot be read and the answer carries a null id.
    let mut line = br#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"verify_receipt","arguments":{"response_text":""#.to_vec();
    line.extend(std::iter::repeat_n(b'a', LINE_LIMIT));
    line.extend_from_slice(br#""}},"id":9}"#);
    let answers = door.ask(&line);
    let got = not_the_sentinel(&answers);
    assert_eq!(got.len(), 1);
    assert!(got[0]["id"].is_null());
    assert_eq!(got[0]["error"]["code"], -32600);

    // An id cut in half at the limit is not trusted either.
    let head = br#"{"jsonrpc":"2.0","method":"x","params":{"p":""#;
    let mut line = head.to_vec();
    line.extend(std::iter::repeat_n(b'a', LINE_LIMIT - head.len() - b"\"},\"id\":12".len()));
    line.extend_from_slice(br#""},"id":1234567"#);
    assert!(line.len() > LINE_LIMIT);
    let answers = door.ask(&line);
    let got = not_the_sentinel(&answers);
    assert_eq!(got.len(), 1);
    assert!(got[0]["id"].is_null(), "{}", got[0]);

    // And the door still works.
    let answers = door.ask(&list_solvers_with_id("7"));
    assert_eq!(not_the_sentinel(&answers)[0]["id"], 7);
}

#[test]
fn the_tools_whose_http_route_takes_2_mib_take_2_mib_here_too() {
    let mut door = Door::start();
    for (tool, id) in [("verify_step", "11"), ("solve", "12"), ("describe_card", "13"), ("verify_solution", "14")] {
        let head = format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{tool}","arguments":{{"x":""#
        );
        let mut line = head.into_bytes();
        line.extend(std::iter::repeat_n(b' ', NARROW_LIMIT));
        line.extend_from_slice(br#""}}}"#);
        let answers = door.ask(&line);
        let got = not_the_sentinel(&answers);
        assert_eq!(got.len(), 1, "{tool}");
        assert_eq!(got[0]["id"], id.parse::<i64>().unwrap());
        assert_eq!(got[0]["error"]["code"], -32600);
        let message = got[0]["error"]["message"].as_str().unwrap();
        assert!(
            message.starts_with(&format!("Request too large: {tool} takes at most 2097152 bytes (2 MiB)")),
            "{message}"
        );
    }
    // A tool with a 16 MiB route reads the same line.
    let answers = door.ask(&receipt_line("15", NARROW_LIMIT + 1000));
    let got = not_the_sentinel(&answers);
    assert_eq!(got[0]["result"]["isError"], true);
    // A 2 MiB line for a narrow tool, exactly at the limit, goes through.
    let head = r#"{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"describe_card","arguments":{"domain":"energy","card":"thermal"},"pad":""#;
    let tail = r#""}}"#;
    let mut line = head.as_bytes().to_vec();
    line.extend(std::iter::repeat_n(b' ', NARROW_LIMIT - head.len() - tail.len()));
    line.extend_from_slice(tail.as_bytes());
    assert_eq!(line.len(), NARROW_LIMIT);
    let answers = door.ask(&line);
    assert_ne!(not_the_sentinel(&answers)[0]["error"]["code"], -32600, "{answers:?}");
}

#[test]
fn verify_tape_refuses_capital_letters_in_a_frame_by_name_on_the_mcp_door() {
    let mut door = Door::start();
    let run = json!({
        "domain": "energy", "card": "thermal", "initial": {"temperature_k": 293.15}, "dt_s": 0.1,
        "segments": [{"steps": 3, "load": {"heat_power_w": 5.0, "ambient_k": 293.15}}], "record_every": 1
    });
    let call = |id: i64, tool: &str, arguments: &Value| {
        serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":tool,"arguments":arguments}})).unwrap()
    };
    let answers = door.ask(&call(21, "run_trajectory", &run));
    let tape: Value = serde_json::from_str(
        not_the_sentinel(&answers)[0]["result"]["content"][0]["text"].as_str().unwrap(),
    )
    .unwrap();
    let answers = door.ask(&call(22, "verify_tape", &tape));
    let report = not_the_sentinel(&answers)[0]["result"]["content"][0]["text"].as_str().unwrap().to_string();
    assert!(report.contains("\"holds\":true"), "{report}");

    for (frame, key, digits) in [(0, "frame_hex", 128), (2, "link", 64), (1, "frame_hex", 128)] {
        let mut edited = tape.clone();
        let low = edited["frames"][frame][key].as_str().unwrap().to_string();
        edited["frames"][frame][key] = json!(low.to_uppercase());
        assert_ne!(low, low.to_uppercase());
        let answers = door.ask(&call(23, "verify_tape", &edited));
        let got = not_the_sentinel(&answers)[0].clone();
        assert_eq!(got["result"]["isError"], true, "{got}");
        assert_eq!(
            got["result"]["content"][0]["text"],
            format!("Physical Violation: 'frames[{frame}].{key}' must be {digits} lowercase hex digits.")
        );
    }
}

#[test]
fn validate_series_takes_16_mib_here_as_its_http_route_does() {
    let mut door = Door::start();
    let head = r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"validate_series","arguments":{"run":{},"quantity":"x","measurements":[],"note":""#;
    let mut line = head.as_bytes().to_vec();
    line.extend(std::iter::repeat_n(b'a', NARROW_LIMIT + 1000));
    line.extend_from_slice(br#""}}}"#);
    let answers = door.ask(&line);
    let got = not_the_sentinel(&answers);
    assert_eq!(got.len(), 1);
    // Read by the tool, which refuses the request by name; not refused for its size.
    assert_eq!(got[0]["result"]["isError"], true, "{}", got[0]);
}
