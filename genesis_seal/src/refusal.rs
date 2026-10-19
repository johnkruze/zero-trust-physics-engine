//! Why a tape, a manifest head, a row or a row proof was not read.
//!
//! A refusal is not a verdict. A record that is refused was not checked. The
//! sentences are the ones the gateway answers with.

use std::fmt;

use serde_json::{Map, Value};

use crate::json_text::{TextRefusal, is_lower_hex};

/// Why a record was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The text repeats a key, holds a number that reads as zero, or is not JSON.
    Text(TextRefusal),
    /// The record is not a JSON object.
    NotAnObject,
    /// A required field is missing.
    Missing(String),
    /// The record carries a key it does not have.
    UnknownKey(String),
    /// A field has the wrong JSON type.
    WrongType { field: String, expected: &'static str },
    /// A hash or a hex value is not the stated count of lowercase hex digits.
    NotExactHex { field: String, digits: usize },
    /// A state field name holds something other than lowercase letters, digits and underscore.
    NotAFieldName(String),
    /// The state has fields and `inputs` names no domain and card.
    NamesNoCard,
    /// A state field is not a hex value, is not a JSON boolean, and has no `_hex` sibling.
    NotSealed(String),
    /// The commitment of a comparison is not of the commitment's form.
    Commitment(crate::commitment::CommitmentFault),
    /// The record was made by seal rule v1 (SHA-256), or names a rule other
    /// than [`crate::SEAL_RULE`]. The field names what showed it.
    SealRuleNotServed(String),
}

/// The name a v1 record is refused by.
pub const SEAL_RULE_NOT_SERVED: &str = "SealRuleNotServed";

/// The sentence a v1 record is refused with.
pub const SEAL_RULE_V1_SENTENCE: &str = "This record was made by seal rule v1 (SHA-256), engine 0.14.3 and earlier; this verifier reads zero-trust-physics/seal/v2 (SHA-384). genesis_seal 0.6.0, kept whole in evidence/verification/engine_0.14.3_served/, verifies it.";

/// The keys only a record of seal rule v1 carries.
pub const V1_KEYS: &[&str] = &[
    "receipt_sha256",
    "sha256_seal",
    "plan_sha256",
    "rows_sha256",
    "frames_sha256",
    "field_sha256",
    "measurements_sha256",
    "request_sha256",
    "reject_detail_sha256",
];

/// The seals whose length tells the rule: 64 hex digits is rule v1.
const SEALS: &[&str] = &["run_seal", "dataset_seal", "solution_seal", "validation_seal", "chain_seal", "design_seal", "merkle_root"];

/// Refuse a record of seal rule v1, or of a rule this crate does not read,
/// before any of its fields is read. A record that names no rule and whose
/// seals are 96 hex digits is read as rule v2 (a row, a tape made by a test).
pub fn refuse_other_rule(object: &Map<String, Value>) -> Result<(), Refusal> {
    if let Some(key) = V1_KEYS.iter().find(|key| object.contains_key(**key)) {
        return Err(Refusal::SealRuleNotServed(format!("it carries '{key}'")));
    }
    match object.get(crate::SEAL_RULE_KEY) {
        None => {}
        Some(Value::String(rule)) if rule == crate::SEAL_RULE => {}
        Some(Value::String(rule)) => return Err(Refusal::SealRuleNotServed(format!("it names seal_rule '{rule}'"))),
        Some(_) => return Err(wrong(crate::SEAL_RULE_KEY, "a string")),
    }
    for key in SEALS {
        if let Some(Value::String(text)) = object.get(*key) {
            if crate::json_text::is_lower_hex(text, 64) {
                return Err(Refusal::SealRuleNotServed(format!("its '{key}' is 64 hex digits")));
            }
        }
    }
    Ok(())
}

impl fmt::Display for Refusal {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Refusal::Text(text) => text.fmt(out),
            Refusal::NotAnObject => {
                write!(out, "Physical Violation: the request must be a JSON object.")
            }
            Refusal::Missing(key) => write!(out, "Missing required parameter: '{key}'"),
            Refusal::UnknownKey(key) => write!(
                out,
                "Physical Violation: '{key}' is not a field of this request."
            ),
            Refusal::WrongType { field, expected } => {
                write!(out, "Physical Violation: '{field}' must be {expected}.")
            }
            Refusal::NotExactHex { field, digits } => write!(
                out,
                "Physical Violation: '{field}' must be {digits} lowercase hex digits."
            ),
            Refusal::NotAFieldName(name) => {
                write!(out, "Physical Violation: '{name}' is not a field name.")
            }
            Refusal::NamesNoCard => {
                write!(out, "Physical Violation: the tape does not name a card.")
            }
            Refusal::NotSealed(name) => write!(
                out,
                "Physical Violation: '{name}' is not a JSON boolean and has no '_hex' sibling."
            ),
            Refusal::Commitment(fault) => fault.fmt(out),
            Refusal::SealRuleNotServed(what) => write!(
                out,
                "Physical Violation: {SEAL_RULE_V1_SENTENCE} ({what}; {SEAL_RULE_NOT_SERVED})"
            ),
        }
    }
}

impl From<TextRefusal> for Refusal {
    fn from(text: TextRefusal) -> Self {
        Refusal::Text(text)
    }
}

pub(crate) fn wrong(field: &str, expected: &'static str) -> Refusal {
    Refusal::WrongType {
        field: field.to_string(),
        expected,
    }
}

pub(crate) fn required<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a Value, Refusal> {
    object
        .get(key)
        .ok_or_else(|| Refusal::Missing(key.to_string()))
}

/// A string that is present and not empty.
pub(crate) fn require_string(object: &Map<String, Value>, key: &str) -> Result<String, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(key.to_string())),
        Some(Value::String(text)) if !text.is_empty() => Ok(text.clone()),
        Some(Value::String(_)) => Err(Refusal::Missing(key.to_string())),
        Some(_) => Err(wrong(key, "a string")),
    }
}

/// A key that is present and holds a string or null.
pub(crate) fn optional_string(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(key.to_string())),
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(wrong(key, "a string or null")),
    }
}

/// A whole number written as a JSON integer.
pub(crate) fn require_whole(object: &Map<String, Value>, key: &str) -> Result<u64, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(key.to_string())),
        Some(value) => value
            .as_u64()
            .ok_or_else(|| wrong(key, "an integer >= 0")),
    }
}

/// A key that is present and holds 96 lowercase hex digits.
pub(crate) fn require_hash(object: &Map<String, Value>, key: &str) -> Result<String, Refusal> {
    let text = require_string(object, key)?;
    require_digits(key, &text, 96)?;
    Ok(text)
}

/// A key that is present and holds 96 lowercase hex digits or null.
pub(crate) fn optional_hash(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, Refusal> {
    let text = optional_string(object, key)?;
    if let Some(text) = &text {
        require_digits(key, text, 96)?;
    }
    Ok(text)
}

pub(crate) fn require_digits(field: &str, text: &str, digits: usize) -> Result<(), Refusal> {
    if is_lower_hex(text, digits) {
        Ok(())
    } else {
        Err(Refusal::NotExactHex {
            field: field.to_string(),
            digits,
        })
    }
}

/// Refuse a key that is not in `allowed`, then a key of `allowed` that is absent.
pub(crate) fn require_exact_keys(
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), Refusal> {
    for key in object.keys() {
        if !allowed.iter().any(|name| *name == key) {
            return Err(Refusal::UnknownKey(key.clone()));
        }
    }
    for name in allowed {
        if !object.contains_key(*name) {
            return Err(Refusal::Missing((*name).to_string()));
        }
    }
    Ok(())
}
