//! The seal of a solve's answer, recomputed from the answer alone: the rule
//! the engine seals a solution by (`protocol_mcp_gateway/src/solution.rs`,
//! `Ingredients`), so a chain that ends in a solve, as a coupling of a card
//! into a solver hands one, is verified with no engine.
//!
//! Six parts, each the SHA-384 of its text: `inputs`, the sealed form of the
//! request ([`crate::sealed_form::sealed_form`]); `engine_version`, the line
//! `engine_version:<version>` and a line break; `result` and `invariants`,
//! one `<section>.<key>:<hex>` line for each key of that section ending in
//! `_hex`, sorted by key (an array's hex in brackets, its items joined by
//! commas); `field_sha384`, its 96 hex characters or `null`; and `outcome`,
//! the compact JSON `{"outcome":…,"iterations":…,"reject":…}`. The solution
//! seal is the SHA-384 of the request's sealed form, the version line, the
//! invariants lines, the result lines, the field text and the outcome JSON,
//! in that order, with nothing between them.
//!
//! What this crate cannot show: that the hex values are the solver's own
//! numbers for that request, and the field the solver's field. The engine
//! shows that by solving the request again (`verify_solution`, or
//! `verify_tape` given the chain's links).

use serde_json::{Map, Value};
use sha2::{Digest, Sha384};

use crate::json_text::is_lower_hex;
use crate::refusal::{Refusal, required, wrong};
use crate::sealed_form::{digest_hex, sealed_form};

/// The six parts of a solution seal, in the order the answer names them.
pub const PARTS: [&str; 6] = ["inputs", "engine_version", "result", "invariants", "field_sha384", "outcome"];

/// What the recomputation of a solve answer's seal found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolutionCheck {
    pub holds: bool,
    /// The first part that does not hash to its posted digest, or
    /// `solution_seal` when every part holds and the seal does not.
    pub cause: Option<&'static str>,
}

fn write_hex(text: &mut String, value: &Value) -> bool {
    match value {
        Value::String(hex) => {
            text.push_str(hex);
            true
        }
        Value::Array(items) => {
            text.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    text.push(',');
                }
                if !write_hex(text, item) {
                    return false;
                }
            }
            text.push(']');
            true
        }
        _ => false,
    }
}

/// The hex lines of one section, or `None` when a `_hex` value is not a
/// string or an array of them.
fn values_text(section: &str, object: Option<&Map<String, Value>>) -> Option<String> {
    let Some(object) = object else { return Some(String::new()) };
    let mut lines = Vec::new();
    for (key, value) in object {
        if !key.ends_with("_hex") {
            continue;
        }
        let mut hex = String::new();
        if !write_hex(&mut hex, value) {
            return None;
        }
        lines.push((key.as_str(), hex));
    }
    lines.sort_by(|left, right| left.0.cmp(right.0));
    let mut text = String::new();
    for (key, hex) in lines {
        text.push_str(&format!("{section}.{key}:{hex}\n"));
    }
    Some(text)
}

fn optional_object<'a>(object: &'a Map<String, Value>, key: &str) -> Result<Option<&'a Map<String, Value>>, Refusal> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(inner)) => Ok(Some(inner)),
        Some(_) => Err(wrong(key, "a JSON object or null")),
    }
}

fn optional_string<'a>(object: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>, Refusal> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.as_str())),
        Some(_) => Err(wrong(key, "a string or null")),
    }
}

/// Recompute the seal of a solve answer from its own values.
pub fn check_solution(body: &Value) -> Result<SolutionCheck, Refusal> {
    let object = body.as_object().ok_or(Refusal::NotAnObject)?;
    let inputs = required(object, "inputs")?;
    let version = required(object, "engine_version")?.as_str().ok_or_else(|| wrong("engine_version", "a string"))?;
    let outcome = required(object, "outcome")?.as_str().ok_or_else(|| wrong("outcome", "a string"))?;
    let iterations = match required(object, "iterations")? {
        Value::Null => None,
        value => Some(value.as_u64().ok_or_else(|| wrong("iterations", "an integer >= 0 or null"))?),
    };
    let reject = optional_string(object, "reject")?;
    let field = optional_string(object, "field_sha384")?;
    if field.is_some_and(|text| !is_lower_hex(text, 96)) {
        return Err(Refusal::NotExactHex { field: "field_sha384".into(), digits: 96 });
    }
    let posted = required(object, "seal_parts")?.as_object().ok_or_else(|| wrong("seal_parts", "a JSON object"))?;
    let seal = required(object, "solution_seal")?.as_str().ok_or_else(|| wrong("solution_seal", "a string"))?;
    if !is_lower_hex(seal, 96) {
        return Err(Refusal::NotExactHex { field: "solution_seal".into(), digits: 96 });
    }
    let fails = |part: &'static str| Ok(SolutionCheck { holds: false, cause: Some(part) });
    let (Some(result_text), Some(invariants_text)) =
        (values_text("result", optional_object(object, "result")?), values_text("invariants", optional_object(object, "invariants")?))
    else {
        return fails("result");
    };
    let request = sealed_form(inputs);
    let version_line = format!("engine_version:{version}\n");
    let field_text = field.unwrap_or("null").to_string();
    let mut tail = Map::new();
    tail.insert("outcome".into(), Value::from(outcome));
    tail.insert("iterations".into(), iterations.map_or(Value::Null, Value::from));
    tail.insert("reject".into(), reject.map_or(Value::Null, Value::from));
    let tail = serde_json::to_vec(&Value::Object(tail)).expect("the outcome encodes");
    let recomputed = [
        digest_hex(request.as_bytes()),
        digest_hex(version_line.as_bytes()),
        digest_hex(result_text.as_bytes()),
        digest_hex(invariants_text.as_bytes()),
        digest_hex(field_text.as_bytes()),
        digest_hex(&tail),
    ];
    for (part, digest) in PARTS.iter().zip(&recomputed) {
        match posted.get(*part) {
            Some(Value::String(text)) if text == digest => {}
            Some(Value::String(_)) => return fails(part),
            Some(_) => return Err(wrong(&format!("seal_parts.{part}"), "a string")),
            None => return Err(Refusal::Missing(format!("seal_parts.{part}"))),
        }
    }
    let mut hasher = Sha384::new();
    hasher.update(request.as_bytes());
    hasher.update(version_line.as_bytes());
    hasher.update(invariants_text.as_bytes());
    hasher.update(result_text.as_bytes());
    hasher.update(field_text.as_bytes());
    hasher.update(&tail);
    if hex::encode(hasher.finalize()) != seal {
        return fails("solution_seal");
    }
    Ok(SolutionCheck { holds: true, cause: None })
}
