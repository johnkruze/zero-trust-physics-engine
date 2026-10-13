//! What the agent door answers for a message that is malformed, and what it
//! holds in memory after lines it refuses.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use protocol_mcp_gateway::guard::{self, Inspection};
use serde_json::{Value, json};

struct Door {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
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
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut door = Door { child, stdin, stdout };
        let hello = door.ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"checks","version":"1"}}}"#.as_bytes());
        assert_eq!(hello["id"], 1, "{hello}");
        door.stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .unwrap();
        door
    }

    fn ask(&mut self, line: &[u8]) -> Value {
        self.stdin.write_all(line).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        let mut answer = String::new();
        self.stdout.read_line(&mut answer).unwrap();
        serde_json::from_str(answer.trim()).unwrap_or_else(|_| panic!("not JSON: {answer:?}"))
    }

    fn resident_mib(&self) -> f64 {
        let out = Command::new("ps")
            .args(["-o", "rss=", "-p", &self.child.id().to_string()])
            .output()
            .expect("ps");
        String::from_utf8_lossy(&out.stdout).trim().parse::<f64>().expect("rss") / 1024.0
    }
}

impl Drop for Door {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut rest = Vec::new();
        let _ = self.stdout.read_to_end(&mut rest);
    }
}

#[test]
fn a_malformed_request_with_a_readable_id_is_answered_under_that_id() {
    let mut door = Door::start();
    for line in [
        r#"{"jsonrpc":"2.0","id":12}"#,
        r#"{"jsonrpc":"2.0","id":12,"method":null}"#,
        r#"{"jsonrpc":"2.0","id":12,"method":7}"#,
        r#"{"jsonrpc":"2.0","id":12,"method":[]}"#,
        r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":5}"#,
        r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":[]}"#,
        r#"{"jsonrpc":2,"id":12,"method":"ping"}"#,
        r#"{"id":12,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":12,"method":"ping","params":7}"#,
        r#"{"jsonrpc":"2.0","id":12,"method":"tools/list","params":{"_meta":5}}"#,
    ] {
        let answer = door.ask(line.as_bytes());
        assert_eq!(answer["id"], 12, "{line} -> {answer}");
        assert_eq!(answer["error"]["code"], -32600, "{line} -> {answer}");
        let sentence = answer["error"]["message"].as_str().unwrap();
        assert!(sentence.starts_with("Invalid Request: "), "{sentence}");
    }
    let answer = door.ask(br#"{"jsonrpc":"2.0","id":"abc","method":[]}"#);
    assert_eq!(answer["id"], "abc", "{answer}");
    // The same message after all that is still answered.
    assert_eq!(door.ask(br#"{"jsonrpc":"2.0","id":13,"method":"ping"}"#)["id"], 13);
}

#[test]
fn a_malformed_message_with_no_usable_id_is_answered_with_a_null_id_and_never_with_none() {
    let mut door = Door::start();
    for line in [
        r#"{"jsonrpc":"2.0","method":5}"#,
        r#"{"jsonrpc":"2.0","method":"ping","params":7}"#,
        r#"[{"jsonrpc":"2.0","id":3,"method":"ping"}]"#,
        r#"{}"#,
        r#"5"#,
        r#"null"#,
        r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#,
    ] {
        let answer = door.ask(line.as_bytes());
        let object = answer.as_object().unwrap();
        assert!(object.contains_key("id"), "{line} -> {answer}");
        assert_eq!(object["id"], Value::Null, "{line} -> {answer}");
        assert!(object.contains_key("error"), "{line} -> {answer}");
    }
}

#[test]
fn a_tool_call_whose_arguments_are_not_an_object_is_refused_as_invalid_params() {
    let mut door = Door::start();
    for (arguments, kind) in [
        ("5", "a number"),
        (r#""text""#, "a string"),
        ("[]", "a list"),
        ("true", "a boolean"),
    ] {
        let line = format!(
            r#"{{"jsonrpc":"2.0","id":31,"method":"tools/call","params":{{"name":"list_cards","arguments":{arguments}}}}}"#
        );
        let answer = door.ask(line.as_bytes());
        assert_eq!(answer["id"], 31, "{answer}");
        assert_eq!(answer["error"]["code"], -32602, "{answer}");
        let sentence = answer["error"]["message"].as_str().unwrap();
        assert!(sentence.contains("'arguments' must be an object"), "{sentence}");
        assert!(sentence.contains(kind), "{sentence}");
    }
    for (line, fragment) in [
        (r#"{"jsonrpc":"2.0","id":32,"method":"tools/call"}"#, "'params' is missing"),
        (r#"{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"arguments":{}}}"#, "'name' is missing"),
        (r#"{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"name":5}}"#, "'name' must be a string"),
    ] {
        let answer = door.ask(line.as_bytes());
        assert_eq!(answer["id"], 32, "{answer}");
        assert_eq!(answer["error"]["code"], -32602, "{answer}");
        assert!(answer["error"]["message"].as_str().unwrap().contains(fragment), "{answer}");
    }
    // An absent or null `arguments` is still an empty object.
    for line in [
        r#"{"jsonrpc":"2.0","id":33,"method":"tools/call","params":{"name":"list_cards"}}"#,
        r#"{"jsonrpc":"2.0","id":33,"method":"tools/call","params":{"name":"list_cards","arguments":null}}"#,
    ] {
        let answer = door.ask(line.as_bytes());
        assert_eq!(answer["id"], 33, "{answer}");
        assert!(answer.get("result").is_some(), "{answer}");
    }
}

#[test]
fn the_shared_guard_answers_a_malformed_request_under_its_id_for_the_hosted_door_too() {
    match guard::inspect(br#"{"jsonrpc":"2.0","id":12,"method":7}"#) {
        Inspection::Answer(text) => {
            let answer: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(answer["id"], 12, "{text}");
            assert_eq!(answer["error"]["code"], -32600, "{text}");
        }
        _ => panic!("the guard did not answer"),
    }
    // The parameter faults of a tool call are asked of the guard apart.
    let call = br#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"list_cards","arguments":3}}"#;
    assert!(matches!(guard::inspect(call), Inspection::Forward));
    let text = guard::tool_call_fault(call).expect("a fault");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(answer["id"], 12, "{text}");
    assert_eq!(answer["error"]["code"], -32602, "{text}");
    assert!(guard::tool_call_fault(br#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"list_cards"}}"#).is_none());
}

#[test]
fn thirty_oversize_lines_leave_the_door_no_larger_than_one() {
    let mut door = Door::start();
    let line = vec![b'y'; 40 * 1024 * 1024];
    let answer = door.ask(&line);
    assert_eq!(answer["error"]["code"], -32600, "{answer}");
    let after_one = door.resident_mib();
    for _ in 0..29 {
        door.ask(&line);
    }
    let after_thirty = door.resident_mib();
    assert!(
        after_thirty <= 2.0 * after_one,
        "resident memory grew from {after_one:.0} MiB after one line to {after_thirty:.0} MiB after thirty"
    );
    assert!(after_thirty < 80.0, "{after_thirty:.0} MiB");
    // The door still answers.
    assert_eq!(door.ask(br#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#)["id"], 9);
}

#[test]
fn repeated_pings_padded_to_eight_mebibytes_do_not_make_the_door_grow() {
    let mut door = Door::start();
    let mut ping = json!({"jsonrpc":"2.0","id":5,"method":"ping"}).to_string().into_bytes();
    ping.resize(8 * 1024 * 1024, b' ');
    assert_eq!(door.ask(&ping)["id"], 5);
    let after_one = door.resident_mib();
    for _ in 0..24 {
        assert_eq!(door.ask(&ping)["id"], 5);
    }
    let after_many = door.resident_mib();
    assert!(
        after_many <= 2.0 * after_one,
        "resident memory grew from {after_one:.0} MiB to {after_many:.0} MiB"
    );
}
