//! What a door answers by itself before the MCP library reads a message.
//!
//! The stdio door reads a line and the hosted door reads a body. Both hand the
//! bytes to [`inspect`], so an unreadable message, a repeated key, a number that
//! is not zero and reads as zero, an id the library would drop, and a message
//! over its size limit are answered in the same words on both.

use serde_json::value::RawValue;

use crate::json_text::{self, Scan};

/// The longest message a door reads: 16 MiB, the limit of the HTTP routes that
/// take the most (`run_trajectory`, `verify_tape`, `run_dataset`,
/// `verify_dataset`, `verify_receipt`, `validate_series`). It counts the whole
/// message, the JSON-RPC envelope included, without the line break.
pub const LINE_LIMIT: usize = 16 * 1024 * 1024;

/// The HTTP default for every other route (`compute_step`, `describe_card`,
/// `describe_solver`, `solve`, `verify_solution`): 2 MiB. A `tools/call`
/// message longer than this for one of those tools is answered with an error.
pub const NARROW_LIMIT: usize = 2 * 1024 * 1024;

/// The tools whose HTTP routes take a 16 MiB body.
pub const WIDE_TOOLS: [&str; 6] = [
    "run_trajectory",
    "verify_tape",
    "run_dataset",
    "verify_dataset",
    "verify_receipt",
    "validate_series",
];

/// What a door does with one message.
pub enum Inspection {
    /// Not a message: an empty line.
    Blank,
    /// The door answers with this JSON text and does not forward the message.
    Answer(String),
    /// The message is refused and has no id, so it is a notification and gets no answer.
    NoAnswer,
    /// Pass it to the MCP library.
    Forward,
}

/// The answer to a message over [`LINE_LIMIT`]. `kept` holds the first
/// `LINE_LIMIT` bytes of it.
pub fn too_long_response(kept: &[u8]) -> String {
    let text = format!(
        "Request too large: a line is at most {LINE_LIMIT} bytes (16 MiB) on this connection, as the largest request is on the web connection. This line is longer and was not read."
    );
    invalid_request(&answer_id(kept, false), &text)
}

/// Every check a door makes before the MCP library sees the message, in the
/// order the stdio door makes them. `line` is the message as it arrived, with
/// its line break when it has one.
pub fn inspect(line: &[u8]) -> Inspection {
    let payload = json_line(line);
    if payload.is_empty() {
        return Inspection::Blank;
    }
    // A tool whose HTTP route takes 2 MiB takes no more here.
    if payload.len() > NARROW_LIMIT {
        if let Some(tool) = narrow_tool(payload) {
            let text = format!(
                "Request too large: {tool} takes at most {NARROW_LIMIT} bytes (2 MiB) on this connection, as on the web connection. This line has {} bytes and was not read.",
                payload.len()
            );
            return Inspection::Answer(invalid_request(&answer_id(payload, true), &text));
        }
    }
    // An unreadable line is answered here and is not forwarded. The library
    // would drop it and write nothing. A repeated key is answered here too,
    // because the library would keep the last copy, and so is a non-zero
    // number the library would read as zero. So is a request whose id the
    // library would not carry: it would drop it in silence.
    let scan = json_text::scan(payload);
    if !matches!(scan, Scan::Unreadable(_)) {
        if let Some(response) = refuse_id(payload) {
            return Inspection::Answer(response);
        }
    }
    let answer = |response: Option<String>| match response {
        Some(response) => Inspection::Answer(response),
        None => Inspection::NoAnswer,
    };
    match scan {
        Scan::Repeated(key) => answer(refusal_response(line, &json_text::repeated_message(&key))),
        Scan::ReadsAsZero { field, token } => answer(refusal_response(
            line,
            &json_text::reads_as_zero_message(&field, &token),
        )),
        Scan::Unreadable(reason) => Inspection::Answer(unreadable_response(line, &reason)),
        Scan::Clean => match shape_fault(payload) {
            Some(response) => Inspection::Answer(response),
            None => Inspection::Forward,
        },
    }
}

/// A JSON-RPC error with the code and sentence given. `id` is the JSON text of the id.
pub fn error_response(id: &str, code: i32, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{code},\"message\":{}}}}}",
        serde_json::to_string(message).expect("the message encodes")
    )
}

fn kind_of(raw: &RawValue) -> &'static str {
    match raw.get().trim_start().as_bytes().first() {
        Some(b'n') => "null",
        Some(b't' | b'f') => "a boolean",
        Some(b'"') => "a string",
        Some(b'[') => "a list",
        Some(b'{') => "an object",
        _ => "a number",
    }
}

fn is_object_or_null(raw: &RawValue) -> bool {
    matches!(raw.get().trim_start().as_bytes().first(), Some(b'n' | b'{'))
}

fn is_string(raw: &RawValue) -> bool {
    raw.get().trim_start().starts_with('"')
}

/// The members of a message the door looks at, kept as the text they have.
#[derive(serde::Deserialize)]
struct Top<'a> {
    #[serde(borrow, default)]
    jsonrpc: Option<&'a RawValue>,
    #[serde(borrow, default)]
    method: Option<&'a RawValue>,
    #[serde(borrow, default)]
    params: Option<&'a RawValue>,
    #[serde(borrow, default)]
    result: Option<&'a RawValue>,
    #[serde(borrow, default)]
    error: Option<&'a RawValue>,
}

#[derive(serde::Deserialize)]
struct CallParams<'a> {
    #[serde(borrow, default)]
    name: Option<&'a RawValue>,
    #[serde(borrow, default)]
    arguments: Option<&'a RawValue>,
}

/// A message that is valid JSON but not a message the MCP library reads. The
/// library would answer it with no `id` member, so the door answers it itself, under the caller's id when the message carries a usable
/// one and under a null id when it does not. A message with no `id` is a
/// notification: it is left to the library, whose answer (if it gives one) the
/// stdio door completes with a null id. The members of `params` are checked
/// against the library's own types only for a message of at most
/// [`NARROW_LIMIT`] bytes, so the check never builds a second copy of a large
/// run request.
fn shape_fault(payload: &[u8]) -> Option<String> {
    let body = payload
        .strip_prefix(b"\xEF\xBB\xBF".as_slice())
        .unwrap_or(payload);
    scan_envelope(body).id?;
    let id = answer_id(body, true);
    let top: Top = serde_json::from_slice(body).ok()?;
    let invalid = |fault: String| Some(error_response(&id, -32600, &format!("Invalid Request: {fault}")));
    match top.jsonrpc {
        Some(version) if serde_json::from_str::<String>(version.get()).is_ok_and(|v| v == "2.0") => {}
        Some(other) => {
            return invalid(format!(
                "'jsonrpc' must be the string \"2.0\"; this message has {}.",
                kind_of(other)
            ));
        }
        None => return invalid("'jsonrpc' is missing; it must be the string \"2.0\".".to_string()),
    }
    match top.method {
        Some(method) if is_string(method) => {}
        Some(other) => {
            return invalid(format!("'method' must be a string; this message has {}.", kind_of(other)));
        }
        None if top.result.is_some() || top.error.is_some() => return None,
        None => {
            return invalid(
                "'method' is missing; a message with an id and no method is neither a request nor a response."
                    .to_string(),
            );
        }
    }
    if let Some(params) = top.params {
        if !is_object_or_null(params) {
            return invalid(format!("'params' must be an object; this message has {}.", kind_of(params)));
        }
    }
    if body.len() <= NARROW_LIMIT
        && serde_json::from_slice::<rmcp::model::ClientJsonRpcMessage>(body).is_err()
    {
        return invalid(
            "a member of 'params' has the wrong type, so the message is not one the server reads.".to_string(),
        );
    }
    None
}

/// A `tools/call` whose parameters are wrong: no `params` object, no `name`
/// string, or an `arguments` that is not an object. The library would answer
/// it as an unknown method, -32601, which is not the fault. This answers
/// -32602 (invalid params) under the caller's id and names the fault. The
/// stdio door asks this of every message it forwards. A message with no id
/// is a notification and gets no answer.
pub fn tool_call_fault(payload: &[u8]) -> Option<String> {
    let body = payload
        .strip_prefix(b"\xEF\xBB\xBF".as_slice())
        .unwrap_or(payload);
    scan_envelope(json_line(body)).id?;
    let body = json_line(body);
    let id = answer_id(body, true);
    let top: Top = serde_json::from_slice(body).ok()?;
    let method: String = serde_json::from_str(top.method?.get()).ok()?;
    if method != "tools/call" {
        return None;
    }
    let call = |fault: String| Some(error_response(&id, -32602, &format!("Invalid params: tools/call. {fault}")));
    let Some(params) = top.params.filter(|p| !p.get().trim_start().starts_with('n')) else {
        return call(
            "'params' is missing or null; it must be an object with a 'name' string and an 'arguments' object.".to_string(),
        );
    };
    let parts: CallParams = serde_json::from_str(params.get()).ok()?;
    match parts.name {
        Some(name) if is_string(name) => {}
        Some(other) => {
            return call(format!(
                "'name' must be a string, the tool's name; this call has {}.",
                kind_of(other)
            ));
        }
        None => return call("'name' is missing; it must be a string, the tool's name.".to_string()),
    }
    match parts.arguments {
        Some(arguments) if !is_object_or_null(arguments) => call(format!(
            "'arguments' must be an object of named values; this call has {}.",
            kind_of(arguments)
        )),
        _ => None,
    }
}

/// The tools whose long form holds the answer sent to the engine's byte for
/// byte, and so reads the text of their arguments as it came.
pub const TEXT_TOOLS: [&str; 3] = ["verify_tape", "verify_solution", "verify_dataset"];

/// The `_meta` key under which the stdio door hands the MCP library a
/// `tools/call` of a [`TEXT_TOOLS`] tool together with the exact text of its
/// arguments, which the library itself reads only as values. The door always
/// writes it, over any value the caller put there.
pub const ARGUMENTS_TEXT: &str = "com.zerotrustphysics/arguments_text";

/// The exact text of a `tools/call`'s `arguments` and the tool's name, when
/// the message is one; `None` for any other message.
pub fn arguments_text(payload: &[u8]) -> Option<(String, String)> {
    let body = payload.strip_prefix(b"\xEF\xBB\xBF".as_slice()).unwrap_or(payload);
    let top: Top = serde_json::from_slice(json_line(body)).ok()?;
    let method: String = serde_json::from_str(top.method?.get()).ok()?;
    if method != "tools/call" {
        return None;
    }
    let parts: CallParams = serde_json::from_str(top.params?.get()).ok()?;
    let name: String = serde_json::from_str(parts.name?.get()).ok()?;
    Some((name, parts.arguments?.get().to_string()))
}

/// A `tools/call` of a [`TEXT_TOOLS`] tool written again with the exact text
/// of its arguments under `params._meta`, [`ARGUMENTS_TEXT`], and a line
/// break; `None` for any other message, which the door forwards as it came.
pub fn with_arguments_text(line: &[u8]) -> Option<Vec<u8>> {
    let (name, text) = arguments_text(line)?;
    if !TEXT_TOOLS.contains(&name.as_str()) {
        return None;
    }
    let body = line.strip_prefix(b"\xEF\xBB\xBF".as_slice()).unwrap_or(line);
    let mut message: serde_json::Value = serde_json::from_slice(json_line(body)).ok()?;
    let params = message.get_mut("params")?.as_object_mut()?;
    let meta = params
        .entry("_meta")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
        .as_object_mut()?;
    meta.insert(ARGUMENTS_TEXT.to_string(), serde_json::Value::String(text));
    let mut written = serde_json::to_vec(&message).ok()?;
    written.push(b'\n');
    Some(written)
}

/// A library answer that has no `id` member (a message the library could not
/// read and the door could not match to an id) with `"id":null` added.
pub fn complete_id(answer: &[u8]) -> Option<Vec<u8>> {
    if !answer.starts_with(b"{\"jsonrpc\":\"2.0\",\"error\":") {
        return None;
    }
    let mut value: serde_json::Value = serde_json::from_slice(json_line(answer)).ok()?;
    let object = value.as_object_mut()?;
    if object.contains_key("id") || !object.contains_key("error") {
        return None;
    }
    object.insert("id".to_string(), serde_json::Value::Null);
    let mut text = serde_json::to_vec(&value).ok()?;
    text.push(b'\n');
    Some(text)
}

/// What a line says about itself at its top level.
#[derive(Default)]
struct Envelope {
    /// Where the value of `id` starts and ends in the line.
    id: Option<(usize, usize)>,
    has_method: bool,
}

/// The top-level `id` and `method` members, read as far as the line can be
/// read. No value is built.
fn scan_envelope(bytes: &[u8]) -> Envelope {
    let mut found = Envelope::default();
    let mut index = 0;
    skip_ws(bytes, &mut index);
    if !eat(bytes, &mut index, b'{') {
        return found;
    }
    loop {
        skip_ws(bytes, &mut index);
        if index >= bytes.len() || bytes[index] == b'}' {
            return found;
        }
        let Some(key) = read_string(bytes, &mut index) else {
            return found;
        };
        skip_ws(bytes, &mut index);
        if !eat(bytes, &mut index, b':') {
            return found;
        }
        skip_ws(bytes, &mut index);
        let start = index;
        if !skip_value(bytes, &mut index) {
            return found;
        }
        match key.as_str() {
            "id" => found.id = Some((start, index)),
            "method" => found.has_method = true,
            _ => {}
        }
        skip_ws(bytes, &mut index);
        if !eat(bytes, &mut index, b',') {
            return found;
        }
    }
}

enum IdKind {
    /// A string, or a whole number from -2^63 to 2^63 - 1. The library carries it.
    Carried,
    /// A number the library would drop: a fraction, an exponent, `-0`, or a
    /// whole number outside the range of a signed 64-bit integer.
    Number,
    /// A value JSON-RPC 2.0 does not allow as an id: `null`, a boolean, a list, an object.
    NotAnId(&'static str),
}

fn id_kind(token: &[u8]) -> IdKind {
    match token.first() {
        Some(b'"') => IdKind::Carried,
        Some(b'n') => IdKind::NotAnId("null"),
        Some(b't' | b'f') => IdKind::NotAnId("a boolean"),
        Some(b'[') => IdKind::NotAnId("a list"),
        Some(b'{') => IdKind::NotAnId("an object"),
        _ => {
            let whole = std::str::from_utf8(token)
                .ok()
                .filter(|text| text.bytes().all(|b| b == b'-' || b.is_ascii_digit()))
                .filter(|text| *text != "-0")
                .and_then(|text| text.parse::<i64>().ok());
            if whole.is_some() {
                IdKind::Carried
            } else {
                IdKind::Number
            }
        }
    }
}

/// An error for a request whose id the MCP library would drop in silence.
/// A message with no `id` is a notification and gets no answer. A message
/// with an `id` and a `method` is a request and always gets one. JSON-RPC 2.0
/// allows a string, a number, or null as an id, and says a number should
/// have no fractional part and null should not be used. A fraction, an
/// exponent form or a whole number out of range is answered under its own
/// token, so the caller can match it. A null id, a boolean, a list or an
/// object cannot be matched to anything, so they get Invalid Request (-32600)
/// with a null id, the id JSON-RPC 2.0 gives a response whose id cannot be
/// determined. Null is treated so, not answered under null as if it were a
/// usable id: the answer to a request with a null id and the answer to an
/// unreadable request would then be the same and the caller could not tell
/// them apart.
pub fn refuse_id(payload: &[u8]) -> Option<String> {
    let envelope = scan_envelope(payload);
    if !envelope.has_method {
        return None;
    }
    let (start, end) = envelope.id?;
    let token = &payload[start..end];
    match id_kind(token) {
        IdKind::Carried => None,
        IdKind::Number => {
            let text = String::from_utf8_lossy(token);
            Some(invalid_request(
                &text,
                &format!(
                    "Invalid Request: the id {text} is not accepted. An id is a string or a whole number from -9223372036854775808 to 9223372036854775807."
                ),
            ))
        }
        IdKind::NotAnId(what) => Some(invalid_request(
            "null",
            &format!(
                "Invalid Request: the id is {what}. An id is a string or a whole number, and a message with no id is a notification."
            ),
        )),
    }
}

/// The id to answer under for a line refused for its size: the caller's own
/// when the line carries a usable one, null when it does not. A line cut at
/// the limit is read as far as it goes, and an id that ends at the cut is not
/// trusted.
pub fn answer_id(bytes: &[u8], complete: bool) -> String {
    let Some((start, end)) = scan_envelope(bytes).id else {
        return "null".to_string();
    };
    if !complete && end >= bytes.len() {
        return "null".to_string();
    }
    let token = &bytes[start..end];
    match id_kind(token) {
        IdKind::NotAnId(_) => "null".to_string(),
        _ => String::from_utf8_lossy(token).into_owned(),
    }
}

/// The tool a `tools/call` line names when it is one of those whose HTTP
/// route takes 2 MiB. Nothing for any other line, or one that cannot be read.
pub fn narrow_tool(payload: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(payload).ok()?;
    if value.get("method")?.as_str()? != "tools/call" {
        return None;
    }
    let name = value.get("params")?.get("name")?.as_str()?;
    if WIDE_TOOLS.contains(&name) {
        None
    } else {
        Some(name.to_string())
    }
}

/// A JSON-RPC Invalid Request error. `id` is the JSON text of the id.
pub fn invalid_request(id: &str, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":-32600,\"message\":{}}}}}",
        serde_json::to_string(message).expect("the message encodes")
    )
}

/// The JSON on this line, without the trailing newline rmcp would strip.
pub fn json_line(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// A tool error with the caller's id. A notification has no id and gets no
/// answer. A line that repeats a key but is not one JSON value is answered
/// with a null id, which the caller of a request still accepts. A request
/// that states in `params._meta` a revision with no `initialize` is answered
/// as that revision writes a result, with `resultType` first.
pub fn refusal_response(line: &[u8], text: &str) -> Option<String> {
    let parsed = serde_json::from_slice::<serde_json::Value>(json_line(line)).ok();
    let id = match &parsed {
        Some(value) => value.get("id")?.clone(),
        None => serde_json::Value::Null,
    };
    let stated = parsed
        .as_ref()
        .and_then(|value| value.get("params"))
        .and_then(|params| params.get("_meta"))
        .and_then(|meta| meta.get("io.modelcontextprotocol/protocolVersion"))
        .and_then(serde_json::Value::as_str);
    let mut result = serde_json::Map::new();
    if stated.is_some_and(|version| !crate::mcp_server::begins_with_initialize(version)) {
        result.insert("resultType".to_string(), serde_json::json!("complete"));
    }
    result.insert("content".to_string(), serde_json::json!([{ "type": "text", "text": text }]));
    result.insert("isError".to_string(), serde_json::json!(true));
    let body = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result });
    Some(serde_json::to_string(&body).expect("the refusal encodes"))
}

/// A JSON-RPC parse error. The id is the caller's when the line still carries
/// one, and null when it does not. The reason is the JSON reader's own text.
pub fn unreadable_response(line: &[u8], reason: &str) -> String {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": recover_id(json_line(line)),
        "error": {
            "code": -32700,
            "message": format!("the line could not be read as JSON: {reason}")
        }
    });
    serde_json::to_string(&body).expect("the refusal encodes")
}

/// The top-level `id`, or null when that value cannot be read. The rest of
/// the line may be unreadable. A later broken value does not hide an earlier id.
fn recover_id(bytes: &[u8]) -> serde_json::Value {
    let mut index = 0;
    skip_ws(bytes, &mut index);
    if !eat(bytes, &mut index, b'{') {
        return serde_json::Value::Null;
    }
    loop {
        skip_ws(bytes, &mut index);
        if index >= bytes.len() || bytes[index] == b'}' {
            return serde_json::Value::Null;
        }
        let Some(key) = read_string(bytes, &mut index) else {
            return serde_json::Value::Null;
        };
        skip_ws(bytes, &mut index);
        if !eat(bytes, &mut index, b':') {
            return serde_json::Value::Null;
        }
        skip_ws(bytes, &mut index);
        if key == "id" {
            return read_one_value(bytes, &mut index).unwrap_or(serde_json::Value::Null);
        }
        if !skip_value(bytes, &mut index) {
            return serde_json::Value::Null;
        }
        skip_ws(bytes, &mut index);
        if index >= bytes.len() {
            return serde_json::Value::Null;
        }
        match bytes[index] {
            b',' => index += 1,
            _ => return serde_json::Value::Null,
        }
    }
}

fn read_one_value(bytes: &[u8], index: &mut usize) -> Option<serde_json::Value> {
    let start = *index;
    if !skip_value(bytes, index) {
        return None;
    }
    serde_json::from_slice(&bytes[start..*index]).ok()
}

fn skip_ws(bytes: &[u8], index: &mut usize) {
    while *index < bytes.len() && matches!(bytes[*index], b' ' | b'\t' | b'\n' | b'\r') {
        *index += 1;
    }
}

fn eat(bytes: &[u8], index: &mut usize, byte: u8) -> bool {
    if *index < bytes.len() && bytes[*index] == byte {
        *index += 1;
        true
    } else {
        false
    }
}

fn skip_value(bytes: &[u8], index: &mut usize) -> bool {
    if *index >= bytes.len() {
        return false;
    }
    match bytes[*index] {
        b'"' => skip_string(bytes, index),
        b't' => skip_literal(bytes, index, b"true"),
        b'f' => skip_literal(bytes, index, b"false"),
        b'n' => skip_literal(bytes, index, b"null"),
        b'-' | b'0'..=b'9' => skip_number(bytes, index),
        b'{' => skip_container(bytes, index, b'{', b'}'),
        b'[' => skip_container(bytes, index, b'[', b']'),
        _ => false,
    }
}

fn skip_literal(bytes: &[u8], index: &mut usize, literal: &[u8]) -> bool {
    if bytes[*index..].starts_with(literal) {
        *index += literal.len();
        true
    } else {
        false
    }
}

/// A JSON number, including one that does not fit in a double. Fitting is
/// decided later, when the token is parsed.
fn skip_number(bytes: &[u8], index: &mut usize) -> bool {
    let start = *index;
    if bytes.get(*index) == Some(&b'-') {
        *index += 1;
    }
    if *index >= bytes.len() || !bytes[*index].is_ascii_digit() {
        *index = start;
        return false;
    }
    if bytes[*index] == b'0' {
        *index += 1;
    } else {
        while *index < bytes.len() && bytes[*index].is_ascii_digit() {
            *index += 1;
        }
    }
    if *index < bytes.len() && bytes[*index] == b'.' {
        *index += 1;
        let fraction = *index;
        while *index < bytes.len() && bytes[*index].is_ascii_digit() {
            *index += 1;
        }
        if *index == fraction {
            *index = start;
            return false;
        }
    }
    if *index < bytes.len() && matches!(bytes[*index], b'e' | b'E') {
        *index += 1;
        if *index < bytes.len() && matches!(bytes[*index], b'+' | b'-') {
            *index += 1;
        }
        let exponent = *index;
        while *index < bytes.len() && bytes[*index].is_ascii_digit() {
            *index += 1;
        }
        if *index == exponent {
            *index = start;
            return false;
        }
    }
    true
}

fn skip_container(bytes: &[u8], index: &mut usize, open: u8, close: u8) -> bool {
    if !eat(bytes, index, open) {
        return false;
    }
    skip_ws(bytes, index);
    if eat(bytes, index, close) {
        return true;
    }
    loop {
        if open == b'{' && !skip_string(bytes, index) {
            return false;
        }
        if open == b'{' {
            skip_ws(bytes, index);
            if !eat(bytes, index, b':') {
                return false;
            }
            skip_ws(bytes, index);
        }
        if !skip_value(bytes, index) {
            return false;
        }
        skip_ws(bytes, index);
        if eat(bytes, index, close) {
            return true;
        }
        if !eat(bytes, index, b',') {
            return false;
        }
        skip_ws(bytes, index);
    }
}

/// Past one JSON string, by the rules of [`read_string`], without keeping its
/// text: a string that is only stepped over is not copied, however long it
/// is. A message of 16 MiB that is one long string costs no second 16 MiB.
fn skip_string(bytes: &[u8], index: &mut usize) -> bool {
    if !eat(bytes, index, b'"') {
        return false;
    }
    while *index < bytes.len() {
        let byte = bytes[*index];
        *index += 1;
        match byte {
            b'"' => return true,
            b'\\' => {
                if read_escape(bytes, index).is_none() {
                    return false;
                }
            }
            0x00..=0x1F => return false,
            0x20..=0x7F => {}
            _ => {
                if read_utf8(bytes, index, byte).is_none() {
                    return false;
                }
            }
        }
    }
    false
}

/// One JSON string, with its escapes applied. A broken escape or a byte that
/// is not UTF-8 ends the read.
fn read_string(bytes: &[u8], index: &mut usize) -> Option<String> {
    if !eat(bytes, index, b'"') {
        return None;
    }
    let mut text = String::new();
    while *index < bytes.len() {
        let byte = bytes[*index];
        *index += 1;
        match byte {
            b'"' => return Some(text),
            b'\\' => text.push(read_escape(bytes, index)?),
            0x00..=0x1F => return None,
            0x20..=0x7F => text.push(byte as char),
            _ => text.push_str(read_utf8(bytes, index, byte)?),
        }
    }
    None
}

fn read_escape(bytes: &[u8], index: &mut usize) -> Option<char> {
    let escaped = *bytes.get(*index)?;
    *index += 1;
    match escaped {
        b'"' | b'\\' | b'/' => Some(escaped as char),
        b'b' => Some('\u{0008}'),
        b'f' => Some('\u{000c}'),
        b'n' => Some('\n'),
        b'r' => Some('\r'),
        b't' => Some('\t'),
        b'u' => read_unicode(bytes, index),
        _ => None,
    }
}

fn read_unicode(bytes: &[u8], index: &mut usize) -> Option<char> {
    let unit = read_hex4(bytes, index)?;
    if (0xD800..=0xDBFF).contains(&unit) {
        if bytes.get(*index) == Some(&b'\\') && bytes.get(*index + 1) == Some(&b'u') {
            *index += 2;
            let low = read_hex4(bytes, index)?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return None;
            }
            let code = 0x1_0000 + (((unit as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
            return char::from_u32(code);
        }
        return None;
    }
    if (0xDC00..=0xDFFF).contains(&unit) {
        return None;
    }
    char::from_u32(u32::from(unit))
}

fn read_hex4(bytes: &[u8], index: &mut usize) -> Option<u16> {
    let end = index.checked_add(4)?;
    let digits = std::str::from_utf8(bytes.get(*index..end)?).ok()?;
    let value = u16::from_str_radix(digits, 16).ok()?;
    *index = end;
    Some(value)
}

fn read_utf8<'a>(bytes: &'a [u8], index: &mut usize, first: u8) -> Option<&'a str> {
    let width = match first {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return None,
    };
    let start = *index - 1;
    let end = start + width;
    let slice = bytes.get(start..end)?;
    let text = std::str::from_utf8(slice).ok()?;
    *index = end;
    Some(text)
}

#[cfg(test)]
mod tests {
    /// The stdio door hands a verification the exact text of its arguments
    /// under params._meta, over any value the caller put there, and passes
    /// every other message as it came.
    #[test]
    fn a_verification_carries_the_exact_text_of_its_arguments() {
        let arguments = r#"{"inputs":{"solver":"x"},"result":{"v":1.0e-4,"s":"\u0063onverged"},"receipt_sha384":"ab"}"#;
        let line = format!(r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"verify_solution","arguments":{arguments},"_meta":{{"{}":"forged","progressToken":1}}}}}}"#, super::ARGUMENTS_TEXT);
        let carried = super::with_arguments_text(format!("{line}\n").as_bytes()).expect("a verification is carried");
        assert!(carried.ends_with(b"\n"));
        let message: serde_json::Value = serde_json::from_slice(&carried).unwrap();
        assert_eq!(message["params"]["_meta"][super::ARGUMENTS_TEXT], arguments);
        assert_eq!(message["params"]["_meta"]["progressToken"], 1);
        assert_eq!(message["params"]["arguments"], serde_json::from_str::<serde_json::Value>(arguments).unwrap());
        assert_eq!(message["id"], 3);
        for tool in ["verify_tape", "verify_dataset"] {
            let line = format!(r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#);
            let carried: serde_json::Value = serde_json::from_slice(&super::with_arguments_text(line.as_bytes()).unwrap()).unwrap();
            assert_eq!(carried["params"]["_meta"][super::ARGUMENTS_TEXT], arguments, "{tool}");
        }
        for line in [
            format!(r#"{{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{{"name":"solve","arguments":{arguments}}}}}"#),
            r#"{"jsonrpc":"2.0","id":6,"method":"tools/list"}"#.to_string(),
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string(),
        ] {
            assert!(super::with_arguments_text(line.as_bytes()).is_none(), "{line}");
        }
    }

    /// Stepping over a string follows the rules of reading one: the same
    /// strings are taken and refused, and the same bytes are passed.
    #[test]
    fn stepping_over_a_string_takes_and_refuses_what_reading_it_does() {
        let samples: [&[u8]; 16] = [
            b"\"plain\" rest",
            b"\"\"",
            b"\"an escape \\n and a quote \\\" inside\"",
            b"\"a unicode escape \\u00e9 and a pair \\ud83d\\ude00\"",
            b"\"a lone surrogate \\ud83d\"",
            b"\"a bad escape \\x\"",
            b"\"a short unicode escape \\u00\"",
            b"\"a control byte \x01 inside\"",
            b"\"a tab \t inside\"",
            b"\"not closed",
            b"\"ends in a backslash \\",
            "\"two bytes \u{e9}, three \u{20ac}, four \u{1f600}\"".as_bytes(),
            b"\"a stray continuation \x80\"",
            b"\"a cut sequence \xe2\x82\"",
            b"no quote",
            b"",
        ];
        let mut taken = 0;
        for sample in samples {
            let (mut read_to, mut stepped_to) = (0, 0);
            let read = super::read_string(sample, &mut read_to).is_some();
            let stepped = super::skip_string(sample, &mut stepped_to);
            assert_eq!(stepped, read, "{:?}", String::from_utf8_lossy(sample));
            if read {
                assert_eq!(stepped_to, read_to, "{:?}", String::from_utf8_lossy(sample));
                taken += 1;
            }
        }
        assert!((4..=12).contains(&taken), "{taken} of 16 were taken: the samples should hold both kinds");
        let long = format!("\"{}\"", "y".repeat(1 << 20));
        let mut index = 0;
        assert!(super::skip_string(long.as_bytes(), &mut index));
        assert_eq!(index, long.len());
    }

    /// The id of a message that is one long string is still found, and the
    /// answer to an over-long message is what it was.
    #[test]
    fn the_id_of_an_over_long_message_is_found_past_a_long_string() {
        let line = format!("{{\"jsonrpc\":\"2.0\",\"params\":{{\"pad\":\"{}\"}},\"id\":41,\"method\":\"ping\"}}", "y".repeat(1 << 16));
        assert_eq!(super::answer_id(line.as_bytes(), true), "41");
        let cut = &line.as_bytes()[..line.len() - 30];
        assert_eq!(super::answer_id(cut, false), "null");
        let first = format!("{{\"id\":\"a\",\"params\":{{\"pad\":\"{}", "y".repeat(1 << 16));
        assert_eq!(super::answer_id(first.as_bytes(), false), "\"a\"");
    }
}
