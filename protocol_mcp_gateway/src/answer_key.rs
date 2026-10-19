//! The operator's answer key: ML-DSA-87 (FIPS 204), held by whoever runs the
//! engine, never in the crate. Every computed answer that ends with a receipt
//! is signed with it as it leaves the engine: the receipt, an issue time, an
//! expiry and a nonce (`genesis_seal::answer_signature`).
//!
//! The key is a 32-byte seed written as 64 lowercase hex digits. A program
//! takes it from `--signing-key <file>`, or from the environment:
//! `ZTP_SIGNING_KEY_FILE=<file>`, or `ZTP_SIGNING_KEY=<64 hex digits>` for a
//! host whose secrets are environment variables. A key file must be a regular
//! file reached without a symbolic link, and on Unix readable by its owner
//! alone; otherwise the program does not start, and says why. The seed is
//! never printed. `--signature-ttl-s <seconds>` sets how long a signature
//! holds, 60 to 604,800 s, 3,600 by default.

use std::path::Path;
use std::sync::Arc;

use genesis_seal::answer_signature::{self, DEFAULT_TTL_S, LEAST_TTL_S, MOST_TTL_S, PublicKey, SigningKey};

/// The environment variable that names a key file.
pub const KEY_FILE_ENV: &str = "ZTP_SIGNING_KEY_FILE";
/// The environment variable that holds the seed itself.
pub const KEY_ENV: &str = "ZTP_SIGNING_KEY";
/// The tools whose answers end with a receipt, and are signed.
pub const SIGNED_TOOLS: [&str; 7] = ["compute_step", "verify_step", "run_trajectory", "solve", "run_dataset", "validate_series", "design_run"];

/// The answer key and how long its signatures hold.
pub struct AnswerKey {
    key: SigningKey,
    ttl_s: u64,
}

impl std::fmt::Debug for AnswerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnswerKey({}, ttl {} s)", self.key.public_key().key_id(), self.ttl_s)
    }
}

impl AnswerKey {
    pub fn new(key: SigningKey, ttl_s: u64) -> Result<AnswerKey, String> {
        if !(LEAST_TTL_S..=MOST_TTL_S).contains(&ttl_s) {
            return Err(format!("--signature-ttl-s must be {LEAST_TTL_S} to {MOST_TTL_S} seconds; it is {ttl_s}."));
        }
        Ok(AnswerKey { key, ttl_s })
    }

    pub fn public_key(&self) -> &PublicKey {
        self.key.public_key()
    }

    pub fn ttl_s(&self) -> u64 {
        self.ttl_s
    }

    /// `text`, an answer ending with its receipt (or a dataset answer whose
    /// head does), with the signature written after the receipt.
    pub fn sign(&self, text: &str) -> Result<String, String> {
        self.key
            .sign_now(text, self.ttl_s)
            .map_err(|refusal| format!("Internal fault: the answer could not be signed ({refusal:?}). No answer is issued for this call."))
    }
}

/// What a program was told of its key.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct KeyArgs {
    pub key_file: Option<String>,
    pub ttl_s: Option<u64>,
}

/// Take `--signing-key <file>` and `--signature-ttl-s <seconds>` out of a
/// program's arguments, each at most once, and give back the others.
pub fn take_key_args(arguments: impl IntoIterator<Item = String>) -> Result<(Vec<String>, KeyArgs), String> {
    let mut rest = Vec::new();
    let mut found = KeyArgs::default();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        let (name, inline) = match argument.split_once('=') {
            Some((name, value)) if name == "--signing-key" || name == "--signature-ttl-s" => (name.to_string(), Some(value.to_string())),
            _ => (argument.clone(), None),
        };
        match name.as_str() {
            "--signing-key" => {
                let value = match inline {
                    Some(value) => value,
                    None => arguments.next().ok_or("--signing-key needs a file.")?,
                };
                if found.key_file.replace(value).is_some() {
                    return Err("--signing-key was given more than once.".to_string());
                }
            }
            "--signature-ttl-s" => {
                let value = match inline {
                    Some(value) => value,
                    None => arguments.next().ok_or("--signature-ttl-s needs a number of seconds.")?,
                };
                let seconds = value
                    .parse::<u64>()
                    .ok()
                    .filter(|_| value.bytes().all(|byte| byte.is_ascii_digit()))
                    .ok_or_else(|| format!("--signature-ttl-s takes a whole number of seconds; '{value}' is not one."))?;
                if found.ttl_s.replace(seconds).is_some() {
                    return Err("--signature-ttl-s was given more than once.".to_string());
                }
            }
            _ => rest.push(argument),
        }
    }
    Ok((rest, found))
}

/// Read a key file: a regular file reached without a symbolic link, readable
/// by its owner alone, holding a seed as 64 lowercase hex digits.
pub fn read_key_file(path: &Path) -> Result<SigningKey, String> {
    let named = path.display();
    let metadata = std::fs::symlink_metadata(path).map_err(|error| format!("the signing key {named} cannot be read: {error}."))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("the signing key {named} is a symbolic link; name the file itself."));
    }
    if !metadata.is_file() {
        return Err(format!("the signing key {named} is not a regular file."));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(format!(
                "the signing key {named} can be read by others (mode {mode:o}); it must be readable by its owner alone (chmod 600)."
            ));
        }
    }
    let text = std::fs::read_to_string(path).map_err(|error| format!("the signing key {named} cannot be read: {error}."))?;
    SigningKey::from_seed_hex(&text).map_err(|_| format!("the signing key {named} is not a seed: it must hold 64 lowercase hex digits."))
}

/// The key the arguments and the environment name, or none. A key named
/// and not read stops the program.
pub fn load(args: &KeyArgs) -> Result<Option<Arc<AnswerKey>>, String> {
    let ttl_s = args.ttl_s.unwrap_or(DEFAULT_TTL_S);
    let key = if let Some(path) = &args.key_file {
        Some(read_key_file(Path::new(path))?)
    } else if let Some(path) = std::env::var_os(KEY_FILE_ENV).filter(|value| !value.is_empty()) {
        Some(read_key_file(Path::new(&path)).map_err(|why| format!("{KEY_FILE_ENV}: {why}"))?)
    } else if let Some(seed) = std::env::var(KEY_ENV).ok().filter(|value| !value.is_empty()) {
        Some(SigningKey::from_seed_hex(seed.trim()).map_err(|_| format!("{KEY_ENV} is not a seed: it must hold 64 lowercase hex digits."))?)
    } else {
        None
    };
    if key.is_none() && args.ttl_s.is_some() {
        return Err("--signature-ttl-s was given with no signing key.".to_string());
    }
    key.map(|key| AnswerKey::new(key, ttl_s).map(Arc::new)).transpose()
}

/// The line a program writes on standard error at start, saying whether it signs.
pub fn start_line(program: &str, key: Option<&AnswerKey>) -> String {
    match key {
        Some(key) => format!(
            "{program}: every computed answer is signed, ML-DSA-87, key_id {}, each signature holding {} s.",
            key.public_key().key_id(),
            key.ttl_s()
        ),
        None => format!("{program}: no signing key was given (--signing-key, {KEY_FILE_ENV} or {KEY_ENV}); answers carry their seals and receipt and no signature."),
    }
}

/// Sign a tool's answer text when the tool's answers end with a receipt and
/// the door holds a key; any other text as it is.
pub fn sign_tool_answer(key: Option<&AnswerKey>, tool: &str, text: String) -> Result<String, String> {
    match key {
        Some(key) if SIGNED_TOOLS.contains(&tool) => key.sign(&text),
        _ => Ok(text),
    }
}

/// A tool result as JSON, `{"content":[{"type":"text","text":…}],…}`, with
/// its answer signed when the tool's answers are signed and the result is no
/// error: how the open connection signs the answer its long-work child
/// computed. A result that cannot be signed is answered as a fault, by name.
pub fn sign_result_value(key: Option<&AnswerKey>, tool: &str, mut result: serde_json::Value) -> serde_json::Value {
    let Some(key) = key.filter(|_| SIGNED_TOOLS.contains(&tool)) else {
        return result;
    };
    if result.get("isError").and_then(serde_json::Value::as_bool) == Some(true) {
        return result;
    }
    let Some(slot) = result.pointer_mut("/content/0/text") else {
        return result;
    };
    let Some(text) = slot.as_str() else {
        return result;
    };
    match key.sign(text) {
        Ok(signed) => {
            *slot = serde_json::Value::String(signed);
            result
        }
        Err(sentence) => serde_json::json!({ "content": [{ "type": "text", "text": sentence }], "isError": true }),
    }
}

/// The answer to `GET /answer-key`: this engine's public key, or that it has none.
pub fn answer_key_response(key: Option<&AnswerKey>) -> (axum::http::StatusCode, String) {
    match key {
        Some(key) => (axum::http::StatusCode::OK, description(key)),
        None => (axum::http::StatusCode::NOT_FOUND, "This door holds no answer key: its answers carry no signature.".to_string()),
    }
}

/// This engine's public key description, `{"alg","key_id","public_key"}`.
pub fn description(key: &AnswerKey) -> String {
    key.public_key().description_json()
}

/// Re-exported for the doors: check an answer's signature.
pub use answer_signature::{Check, SignatureCause, SignatureReport, check};

// ---- Verifying an answer's signature at the door ----

/// The field of a verification that gives the key to check a signature with:
/// the base64 of an ML-DSA-87 public key, 2,592 bytes.
pub const PUBLIC_KEY_FIELD: &str = "public_key";
/// The field of a verification that lists the nonces its caller has seen.
pub const SEEN_NONCES_FIELD: &str = "seen_nonces";
/// The cause a verification names when an answer's signature does not hold.
pub const CAUSE_SIGNATURE: &str = "signature";

use serde_json::{Map, Value};

use crate::DoorError;

/// The arguments of a verification with the verifier's own fields and every
/// signature taken out, and what each signature check found.
pub struct Presented {
    /// The arguments as the tool reads them: no `public_key`, no
    /// `seen_nonces`, no `signature` on an answer.
    pub value: Value,
    /// Their exact text, with the same taken out, when the connection gave one.
    pub text: Option<String>,
    /// Each answer's check: `None` for the whole answer, `Some(i)` for link `i`.
    pub checks: Vec<(Option<usize>, SignatureReport)>,
    /// A key was given in the request.
    pub key_given: bool,
}

fn malformed(field: &str, what: &str) -> DoorError {
    DoorError::Unprocessable(format!("Physical Violation: '{field}' must be {what} (SignatureMalformed)."))
}

/// The spans of the members of the top-level object of `text`: for each, its
/// key and the byte range from the key's opening quote to the end of its value.
fn members(text: &str) -> Option<Vec<(String, std::ops::Range<usize>)>> {
    let bytes = text.as_bytes();
    let mut at = bytes.iter().position(|byte| !byte.is_ascii_whitespace())?;
    if bytes[at] != b'{' {
        return None;
    }
    at += 1;
    let mut found = Vec::new();
    loop {
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        match bytes.get(at)? {
            b'}' => return Some(found),
            b',' => {
                at += 1;
                continue;
            }
            b'"' => {}
            _ => return None,
        }
        let start = at;
        let mut stream = serde_json::Deserializer::from_str(&text[at..]).into_iter::<String>();
        let key = stream.next()?.ok()?;
        at += stream.byte_offset();
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if bytes.get(at) != Some(&b':') {
            return None;
        }
        at += 1;
        let mut stream = serde_json::Deserializer::from_str(&text[at..]).into_iter::<serde::de::IgnoredAny>();
        stream.next()?.ok()?;
        at += stream.byte_offset();
        found.push((key, start..at));
    }
}

/// `text` with the top-level members `names` taken out, each with the comma
/// that joined it to its neighbour; every other byte as it was.
fn without_members(text: &str, names: &[&str]) -> Option<String> {
    let spans = members(text)?;
    let mut cut: Vec<std::ops::Range<usize>> = Vec::new();
    for (index, (key, span)) in spans.iter().enumerate() {
        if !names.contains(&key.as_str()) {
            continue;
        }
        let range = if index > 0 {
            // From the end of the member before it, so its comma goes too.
            spans[index - 1].1.end..span.end
        } else if let Some(next) = spans.get(1) {
            span.start..next.1.start
        } else {
            span.clone()
        };
        cut.push(range);
    }
    if cut.is_empty() {
        return Some(text.to_string());
    }
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for range in cut {
        if range.start < at {
            return None;
        }
        out.push_str(&text[at..range.start]);
        at = range.end;
    }
    out.push_str(&text[at..]);
    Some(out)
}

/// The text of each element of the array that is the value of the
/// top-level member `name`, and the range of each in `text`.
fn array_elements(text: &str, name: &str) -> Option<Vec<std::ops::Range<usize>>> {
    let spans = members(text)?;
    let (_, span) = spans.into_iter().find(|(key, _)| key == name)?;
    let bytes = text.as_bytes();
    let mut at = span.start + text[span.start..].find(':')? + 1;
    while at < span.end && bytes[at].is_ascii_whitespace() {
        at += 1;
    }
    if bytes.get(at) != Some(&b'[') {
        return None;
    }
    at += 1;
    let mut found = Vec::new();
    loop {
        while at < span.end && (bytes[at].is_ascii_whitespace() || bytes[at] == b',') {
            at += 1;
        }
        if at >= span.end || bytes[at] == b']' {
            return Some(found);
        }
        let mut stream = serde_json::Deserializer::from_str(&text[at..span.end]).into_iter::<serde::de::IgnoredAny>();
        stream.next()?.ok()?;
        let end = at + stream.byte_offset();
        found.push(at..end);
        at = end;
    }
}

/// Whether a verification's arguments carry a whole answer, whose signature
/// is to be read: a run, a solution, a design, a dataset's three parts, or a
/// chain's links. A verification by request carries none.
fn carries_answer(object: &Map<String, Value>) -> bool {
    !object.contains_key("request")
        && ["inputs", "head", "links", "tool", "frames", "solution_seal", "run_seal"].iter().any(|key| object.contains_key(*key))
}

/// Read the verifier's fields, take every signature out of the answer, and
/// check each against the key given, or else this engine's own.
pub fn present(state: &crate::AppState, value: &Value, text: Option<&str>) -> Result<Presented, DoorError> {
    let Some(object) = value.as_object() else {
        return Ok(Presented { value: value.clone(), text: text.map(str::to_string), checks: Vec::new(), key_given: false });
    };
    let given_key = match object.get(PUBLIC_KEY_FIELD) {
        None => None,
        Some(Value::String(text)) => Some(PublicKey::from_base64(text).ok_or_else(|| {
            malformed(PUBLIC_KEY_FIELD, "the base64 of an ML-DSA-87 public key, 2,592 bytes")
        })?),
        Some(_) => return Err(malformed(PUBLIC_KEY_FIELD, "a string")),
    };
    let seen: Vec<String> = match object.get(SEEN_NONCES_FIELD) {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(nonce) if crate::json_text::is_lower_hex(nonce, 64) => Ok(nonce.clone()),
                _ => Err(malformed(SEEN_NONCES_FIELD, "an array of nonces, each 64 lowercase hex digits")),
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(malformed(SEEN_NONCES_FIELD, "an array of nonces, each 64 lowercase hex digits")),
    };
    let mut cleaned = object.clone();
    cleaned.shift_remove(PUBLIC_KEY_FIELD);
    cleaned.shift_remove(SEEN_NONCES_FIELD);
    let mut text = match text {
        Some(text) => without_members(text, &[PUBLIC_KEY_FIELD, SEEN_NONCES_FIELD]),
        None => None,
    };
    let key_given = given_key.is_some();
    let engine_key = state.answer_key().map(|key| key.public_key().clone());
    let key = given_key.or(engine_key);
    let now_s = answer_signature::now_s();
    let check_with = |answer_text: &str| check(answer_text, Check { public_key: key.as_ref(), now_s, seen_nonces: &seen });
    let mut checks = Vec::new();
    if !carries_answer(&cleaned) {
        return Ok(Presented { value: Value::Object(cleaned), text, checks, key_given });
    }
    let whole_text = text.clone().unwrap_or_else(|| serde_json::to_string(&Value::Object(cleaned.clone())).unwrap_or_default());
    if let Some(Value::Array(links)) = cleaned.get("links").cloned() {
        // Each link is an answer of its own, signed alone.
        let elements = array_elements(&whole_text, "links").filter(|found| found.len() == links.len());
        let mut unsigned_links = Vec::with_capacity(links.len());
        let mut rebuilt = String::new();
        let mut at = 0;
        for (index, link) in links.iter().enumerate() {
            let link_text = match &elements {
                Some(found) => whole_text[found[index].clone()].to_string(),
                None => serde_json::to_string(link).unwrap_or_default(),
            };
            let report = check_with(&link_text);
            if let Some(found) = &elements {
                rebuilt.push_str(&whole_text[at..found[index].start]);
                rebuilt.push_str(&report.unsigned);
                at = found[index].end;
            }
            let mut link = link.clone();
            if let Some(object) = link.as_object_mut() {
                object.shift_remove(answer_signature::SIGNATURE_KEY);
            }
            unsigned_links.push(link);
            checks.push((Some(index), report));
        }
        if elements.is_some() {
            rebuilt.push_str(&whole_text[at..]);
            text = text.map(|_| rebuilt);
        }
        cleaned.insert("links".to_string(), Value::Array(unsigned_links));
    } else {
        let report = check_with(&whole_text);
        if text.is_some() {
            text = Some(report.unsigned.clone());
        }
        cleaned.shift_remove(answer_signature::SIGNATURE_KEY);
        if let Some(Value::Object(head)) = cleaned.get_mut("head") {
            head.shift_remove(answer_signature::SIGNATURE_KEY);
        }
        checks.push((None, report));
    }
    Ok(Presented { value: Value::Object(cleaned), text, checks, key_given })
}

fn report_value(report: &SignatureReport) -> Value {
    let mut object = Map::new();
    let holds = match report.cause {
        None if report.present => Value::Bool(true),
        Some(SignatureCause::KeyNotGiven) => Value::Null,
        _ => Value::Bool(false),
    };
    object.insert("holds".into(), holds);
    object.insert("cause".into(), report.cause.map_or(Value::Null, |cause| Value::from(cause.name())));
    let envelope = report.envelope.as_ref();
    for (key, value) in [
        ("key_id", envelope.map(|e| e.key_id.clone())),
        ("issued_at", envelope.map(|e| e.issued_at.clone())),
        ("expires_at", envelope.map(|e| e.expires_at.clone())),
        ("nonce", envelope.map(|e| e.nonce.clone())),
    ] {
        object.insert(key.into(), value.map_or(Value::Null, Value::from));
    }
    let sentence = match report.cause {
        None => "The ML-DSA-87 signature verifies over the receipt, the times and the nonce under the key it was checked with, the receipt covers the answer's bytes, and the signature has not expired.".to_string(),
        Some(SignatureCause::KeyNotGiven) => "The answer is signed and no public_key was given, and this door holds no answer key, so the signature was not checked: give public_key.".to_string(),
        Some(cause) => cause.sentence().to_string(),
    };
    object.insert("sentence".into(), Value::from(sentence));
    Value::Object(object)
}

impl Presented {
    /// The checks that do not hold: a signature that fails, or an answer
    /// with none when a key was given. A signature left unchecked for want
    /// of a key does not fail the answer.
    fn failing(&self) -> Option<&(Option<usize>, SignatureReport)> {
        self.checks.iter().find(|(_, report)| match report.cause {
            None | Some(SignatureCause::KeyNotGiven) => false,
            Some(SignatureCause::Absent) => self.key_given,
            Some(_) => true,
        })
    }

    fn signature_entry(&self) -> Option<(String, Value)> {
        let reported: Vec<&(Option<usize>, SignatureReport)> = self
            .checks
            .iter()
            .filter(|(_, report)| report.present || (self.key_given && report.cause.is_some()))
            .collect();
        if reported.is_empty() {
            return None;
        }
        match reported.as_slice() {
            [(None, report)] => Some(("signature".to_string(), report_value(report))),
            _ => Some((
                "signatures".to_string(),
                Value::Array(self.checks.iter().map(|(_, report)| report_value(report)).collect()),
            )),
        }
    }

    /// The answer of a verification whose signature does not hold: nothing
    /// is computed.
    pub fn failure_answer(&self) -> Option<String> {
        let (link, report) = self.failing()?;
        let mut object = Map::new();
        object.insert("holds".into(), Value::Bool(false));
        object.insert("values_hold".into(), Value::Bool(false));
        object.insert("cause".into(), Value::from(CAUSE_SIGNATURE));
        if let Some(index) = link {
            object.insert("broken_link".into(), Value::from(*index as u64));
        }
        let (key, value) = self.signature_entry().unwrap_or_else(|| ("signature".to_string(), report_value(report)));
        object.insert(key, value);
        object.insert(
            "compared".into(),
            Value::from("SIGNATURE_CHECKED: the answer's signature was checked before anything else, and nothing was computed."),
        );
        serde_json::to_string(&Value::Object(object)).ok()
    }

    /// A verification's answer with what each signature check found, after
    /// `values_hold` (or last).
    pub fn annotate(&self, answer: String) -> String {
        let Some((key, entry)) = self.signature_entry() else {
            return answer;
        };
        let Ok(Value::Object(object)) = serde_json::from_str::<Value>(&answer) else {
            return answer;
        };
        let mut out = Map::new();
        let mut placed = false;
        for (name, value) in object {
            let after = name == "values_hold";
            out.insert(name, value);
            if after {
                out.insert(key.clone(), entry.clone());
                placed = true;
            }
        }
        if !placed {
            out.insert(key, entry);
        }
        serde_json::to_string(&Value::Object(out)).unwrap_or(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_is_taken_out_of_the_text_with_its_comma_and_nothing_else() {
        let text = r#"{"a": 1, "public_key":"k" ,"b":[1,{"c":"}"}],"seen_nonces":[]}"#;
        assert_eq!(without_members(text, &[PUBLIC_KEY_FIELD, SEEN_NONCES_FIELD]).unwrap(), r#"{"a": 1 ,"b":[1,{"c":"}"}]}"#);
        assert_eq!(without_members(r#"{"public_key":"k","a":1}"#, &[PUBLIC_KEY_FIELD]).unwrap(), r#"{"a":1}"#);
        assert_eq!(without_members(r#"{"public_key":"k"}"#, &[PUBLIC_KEY_FIELD]).unwrap(), r#"{}"#);
        assert_eq!(without_members(r#"{"a":1}"#, &[PUBLIC_KEY_FIELD]).unwrap(), r#"{"a":1}"#);
        let links = r#"{"links":[{"x":1}, {"y":[2]}],"public_key":"k"}"#;
        let found = array_elements(links, "links").unwrap();
        assert_eq!(found.iter().map(|range| &links[range.clone()]).collect::<Vec<_>>(), vec![r#"{"x":1}"#, r#"{"y":[2]}"#]);
    }

    #[test]
    fn key_arguments_are_taken_once_each() {
        let (rest, found) = take_key_args(["--dataset-dir".to_string(), "d".to_string(), "--signing-key".to_string(), "k".to_string(), "--signature-ttl-s=120".to_string()]).unwrap();
        assert_eq!(rest, vec!["--dataset-dir".to_string(), "d".to_string()]);
        assert_eq!(found, KeyArgs { key_file: Some("k".to_string()), ttl_s: Some(120) });
        assert!(take_key_args(["--signing-key".to_string(), "a".to_string(), "--signing-key".to_string(), "b".to_string()]).is_err());
        assert!(take_key_args(["--signature-ttl-s".to_string(), "1e3".to_string()]).is_err());
        assert!(AnswerKey::new(SigningKey::from_seed(&[1; 32]), 59).is_err());
    }
}

/// `verify_tape`, `verify_solution` or `verify_dataset`, with the answer's
/// signature checked first: a signature that does not hold is answered by
/// name and nothing is computed; one that holds, or is left unchecked, is
/// said in the tool's answer, and the tool reads the answer without it.
pub async fn verify_signed(state: &crate::AppState, tool: &str, value: &Value, text: Option<&str>) -> Result<String, DoorError> {
    let presented = present(state, value, text)?;
    if let Some(failed) = presented.failure_answer() {
        return Ok(failed);
    }
    let text = presented.text.as_deref();
    let answer = match tool {
        "verify_tape" => crate::trajectory::verify(state, &presented.value, text).await?,
        "verify_solution" => crate::solution::verify(state, &presented.value, text).await?,
        "verify_dataset" => crate::dataset::verify(state, &presented.value, text).await?,
        _ => return Err(DoorError::NotFound(format!("Tool not recognized: {tool}"))),
    };
    Ok(presented.annotate(answer))
}

/// What `verify_receipt` says of a response's signature: checked against
/// the `public_key` given, or else this engine's own; `None` when the
/// response carries none and no key was given.
pub fn receipt_signature(engine_key: Option<PublicKey>, object: &Map<String, Value>, text: &str) -> Result<Option<Value>, DoorError> {
    let given = match object.get(PUBLIC_KEY_FIELD) {
        None => None,
        Some(Value::String(key)) => Some(PublicKey::from_base64(key).ok_or_else(|| {
            malformed(PUBLIC_KEY_FIELD, "the base64 of an ML-DSA-87 public key, 2,592 bytes")
        })?),
        Some(_) => return Err(malformed(PUBLIC_KEY_FIELD, "a string")),
    };
    let seen: Vec<String> = match object.get(SEEN_NONCES_FIELD) {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(nonce) if crate::json_text::is_lower_hex(nonce, 64) => Ok(nonce.clone()),
                _ => Err(malformed(SEEN_NONCES_FIELD, "an array of nonces, each 64 lowercase hex digits")),
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(malformed(SEEN_NONCES_FIELD, "an array of nonces, each 64 lowercase hex digits")),
    };
    let key_given = given.is_some();
    let key = given.or(engine_key);
    let report = check(text, Check { public_key: key.as_ref(), now_s: answer_signature::now_s(), seen_nonces: &seen });
    if !report.present && !key_given {
        return Ok(None);
    }
    Ok(Some(report_value(&report)))
}
