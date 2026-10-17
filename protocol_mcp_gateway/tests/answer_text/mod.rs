//! An answer sent back as it came.
//!
//! Since 0.12.0 the long form of a verification holds the answer sent to the
//! engine's answer byte for byte, so an agent sends back the text it was
//! given. A test holds an answer as a value; here each answer the engine
//! gave a test (every answer that ends with its receipt) is remembered by its
//! value, and a request is written as JSON text with every remembered answer
//! inside it written as the text it came as, byte for byte. A value the test
//! changed is not an answer the engine gave, and is written as its JSON text,
//! as anyone would write it.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

/// The answers given so far, by their receipt.
fn given() -> &'static Mutex<HashMap<String, Vec<(Value, String)>>> {
    static GIVEN: OnceLock<Mutex<HashMap<String, Vec<(Value, String)>>>> = OnceLock::new();
    GIVEN.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Remember the text of an answer, when it is one that ends with a receipt.
pub fn remember(text: &str) {
    if !text.contains("\"receipt_sha256\"") {
        return;
    }
    let Ok(value) = serde_json::from_str::<Value>(text) else { return };
    let Some(receipt) = value.get("receipt_sha256").and_then(Value::as_str).map(str::to_string) else { return };
    let mut given = given().lock().unwrap();
    let entries = given.entry(receipt).or_default();
    if !entries.iter().any(|(held, _)| *held == value) {
        entries.push((value, text.to_string()));
    }
}

/// The text of `body` with every answer the engine gave inside it written as
/// it came.
pub fn as_sent(body: &Value) -> String {
    let given = given().lock().unwrap();
    let mut out = String::new();
    write(&given, body, &mut out);
    out
}

fn write(given: &HashMap<String, Vec<(Value, String)>>, value: &Value, out: &mut String) {
    if let Some(receipt) = value.get("receipt_sha256").and_then(Value::as_str) {
        if let Some((_, text)) = given.get(receipt).and_then(|entries| entries.iter().find(|(held, _)| held == value)) {
            out.push_str(text);
            return;
        }
    }
    match value {
        Value::Object(map) => {
            out.push('{');
            for (index, (key, item)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap());
                out.push(':');
                write(given, item, out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write(given, item, out);
            }
            out.push(']');
        }
        other => out.push_str(&serde_json::to_string(other).unwrap()),
    }
}

/// A tools/call message as JSON-RPC text, its arguments written by
/// [`as_sent`].
pub struct ToolCall(pub String);

impl std::fmt::Display for ToolCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `{"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}}`
/// with every answer inside the arguments written as it came.
pub fn tool_call(id: u64, name: &str, arguments: &Value) -> ToolCall {
    ToolCall(format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"tools/call\",\"params\":{{\"name\":{},\"arguments\":{}}}}}",
        serde_json::to_string(name).unwrap(),
        as_sent(arguments)
    ))
}
