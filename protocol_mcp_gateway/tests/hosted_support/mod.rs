//! A small synchronous harness: the real binaries over pipes and sockets,
//! and raw HTTP by hand so a test can send what a library client would not.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[path = "../answer_text/mod.rs"]
pub mod answer_text;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "fixes_{}_{}_{name}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// SHA-256 of a client's key, as the keys file holds it (authentication, not a seal).
pub fn sha(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// SHA-384 of a text, as a receipt or a seal of seal rule v2 is taken.
pub fn seal_sha(text: &str) -> String {
    genesis_seal::sealed_form::digest_hex(text.as_bytes())
}

pub fn thermal_run(steps: u64, temperature_k: f64) -> Value {
    json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": temperature_k }, "dt_s": 0.1,
        "segments": [{ "steps": steps, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": steps
    })
}

pub fn thermal_step() -> Value {
    json!({
        "domain": "energy", "card": "thermal",
        "parameters": { "temperature_k": 293.15, "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 0.1 }
    })
}

/// A file dataset request of one thermal run, with the keys in the order typed.
pub fn file_request(temperature_k: f64) -> Value {
    let mut run = thermal_run(3, temperature_k);
    let object = run.as_object_mut().unwrap();
    object.remove("domain");
    object.remove("card");
    json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "list", "runs": [run] },
        "store": "frames", "return": "file"
    })
}

/// The same request with its keys in the opposite order, at every depth.
pub fn reversed(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(reversed).collect()),
        Value::Object(map) => Value::Object(map.iter().rev().map(|(k, v)| (k.clone(), reversed(v))).collect()),
        other => other.clone(),
    }
}

pub fn client_entry(label: &str, key: &str, max_in_flight: u64, max_permits: u64) -> Value {
    json!({
        "client": label, "key_sha256": sha(key), "cards": [["energy", "thermal"], ["energy", "cell_18650pf"]],
        "solvers": [], "max_in_flight": max_in_flight, "max_permits": max_permits
    })
}

/// What the hosted door answered: the status, the headers' text and the body.
#[derive(Debug)]
pub struct Reply {
    pub status: u16,
    pub head: String,
    pub body: String,
}

pub fn parse_reply(bytes: &[u8]) -> Reply {
    let text = String::from_utf8_lossy(bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    Reply { status, head: head.to_string(), body: body.to_string() }
}

/// Send bytes, read to the end of the connection or the time limit.
pub fn raw(port: u16, bytes: &[u8], seconds: u64) -> Vec<u8> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(seconds))).unwrap();
    let _ = stream.write_all(bytes);
    let mut out = Vec::new();
    let _ = stream.read_to_end(&mut out);
    out
}

pub fn request_bytes(key: Option<&str>, body: &str) -> Vec<u8> {
    let mut head = String::from("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n");
    if let Some(key) = key {
        head.push_str(&format!("Authorization: Bearer {key}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head.into_bytes()
}

/// A tools/call message, with every answer the engine gave inside its
/// arguments written as it came (`answer_text`).
pub fn tool_body(id: u64, tool: &str, arguments: &Value) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"tools/call\",\"params\":{{\"name\":{},\"arguments\":{}}}}}",
        serde_json::to_string(tool).unwrap(),
        answer_text::as_sent(arguments)
    )
}

/// `(is_error, text)` of a tool answer. An answer is remembered, so it is
/// sent back as it came.
pub fn tool_result(reply: &Reply) -> (bool, String) {
    let value: Value = serde_json::from_str(&reply.body).unwrap_or_else(|_| panic!("not JSON: {reply:?}"));
    let text = value["result"]["content"][0]["text"].as_str().unwrap_or("").to_string();
    answer_text::remember(&text);
    (value["result"]["isError"] == json!(true), text)
}

pub struct Hosted {
    pub child: Child,
    pub port: u16,
    pub dir: PathBuf,
}

impl Hosted {
    pub fn start(entries: Vec<Value>, dataset_dir: Option<&Path>) -> Hosted {
        let dir = scratch("hosted");
        let keys = dir.join("keys.json");
        std::fs::write(&keys, serde_json::to_string(&Value::Array(entries)).unwrap()).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_hosted_door"));
        // A door with no answer key serves unsigned answers only when told so.
        command.arg("--unsigned");
        command.arg("--port").arg("0").arg("--keys").arg(&keys);
        if let Some(path) = dataset_dir {
            command.arg("--dataset-dir").arg(path);
        }
        let mut child = command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
        let address = line.split("running on ").nth(1).unwrap_or_else(|| panic!("no address in {line:?}")).split(' ').next().unwrap();
        let port = address.rsplit(':').next().unwrap().parse().unwrap();
        Hosted { child, port, dir }
    }

    pub fn post(&self, key: &str, body: &str) -> Reply {
        parse_reply(&raw(self.port, &request_bytes(Some(key), body), 120))
    }

    pub fn tool(&self, key: &str, tool: &str, arguments: &Value) -> (u16, bool, String) {
        let reply = self.post(key, &tool_body(1, tool, arguments));
        if reply.status == 200 {
            let (error, text) = tool_result(&reply);
            (200, error, text)
        } else {
            (reply.status, false, reply.body)
        }
    }

    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for Hosted {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct Stdio2 {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    next: u64,
    /// The answer to `initialize`, as the connection gave it.
    #[allow(dead_code)]
    pub hello: String,
}

impl Stdio2 {
    pub fn start(dataset_dir: Option<&Path>) -> Stdio2 {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"));
        if let Some(path) = dataset_dir {
            command.arg("--dataset-dir").arg(path);
        }
        let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut door = Stdio2 { child, stdin, out, next: 1, hello: String::new() };
        door.hello = door.line(&json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}).to_string());
        door.stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n").unwrap();
        door
    }

    fn line(&mut self, text: &str) -> String {
        self.stdin.write_all(text.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).unwrap();
        answer
    }

    /// Every text block of a tool answer, in order.
    pub fn blocks(&mut self, tool: &str, arguments: &Value) -> Vec<String> {
        self.next += 1;
        let answer = self.line(&tool_body(self.next, tool, arguments));
        let value: Value = serde_json::from_str(&answer).unwrap();
        let blocks: Vec<String> = value["result"]["content"]
            .as_array()
            .map(|blocks| blocks.iter().map(|block| block["text"].as_str().unwrap_or("").to_string()).collect())
            .unwrap_or_default();
        blocks.iter().for_each(|block| answer_text::remember(block));
        blocks
    }

    /// `(is_error, text)` of a call whose arguments are the text given, byte
    /// for byte.
    pub fn tool_raw(&mut self, tool: &str, arguments_text: &str) -> (bool, String) {
        self.next += 1;
        let line = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"tools/call\",\"params\":{{\"name\":{},\"arguments\":{arguments_text}}}}}",
            self.next,
            serde_json::to_string(tool).unwrap()
        );
        let answer = self.line(&line);
        let value: Value = serde_json::from_str(&answer).unwrap();
        let text = value["result"]["content"][0]["text"].as_str().unwrap_or("").to_string();
        (value["result"]["isError"] == json!(true), text)
    }

    /// `(is_error, text)`. An answer is remembered, so it is sent back as it came.
    pub fn tool(&mut self, tool: &str, arguments: &Value) -> (bool, String) {
        self.next += 1;
        let answer = self.line(&tool_body(self.next, tool, arguments));
        let value: Value = serde_json::from_str(&answer).unwrap();
        let text = value["result"]["content"][0]["text"].as_str().unwrap_or("").to_string();
        answer_text::remember(&text);
        (value["result"]["isError"] == json!(true), text)
    }
}

impl Drop for Stdio2 {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn make_fifo(path: &Path) {
    assert!(Command::new("mkfifo").arg(path).status().unwrap().success());
}

/// The folder a file job stored, from its answer.
pub fn stored_folder(dir: &Path, text: &str) -> PathBuf {
    let answer: Value = serde_json::from_str(text).unwrap_or_else(|_| panic!("not JSON: {text}"));
    dir.join(answer["files"]["folder"].as_str().unwrap_or_else(|| panic!("no folder in {text}")))
}
