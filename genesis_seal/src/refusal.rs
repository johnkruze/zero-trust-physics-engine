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

/// A key that is present and holds 64 lowercase hex digits.
pub(crate) fn require_hash(object: &Map<String, Value>, key: &str) -> Result<String, Refusal> {
    let text = require_string(object, key)?;
    require_digits(key, &text, 64)?;
    Ok(text)
}

/// A key that is present and holds 64 lowercase hex digits or null.
pub(crate) fn optional_hash(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, Refusal> {
    let text = optional_string(object, key)?;
    if let Some(text) = &text {
        require_digits(key, text, 64)?;
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
