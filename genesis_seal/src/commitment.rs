//! A commitment to the measurements of a comparison: a salted SHA-256 made
//! before the comparison and opened by it.
//!
//! A laboratory or an agent that holds measurements back commits to them
//! first: it publishes the SHA-256 of a fixed tag, the committed text of the
//! measurements, the quantity they are of and its component, and a salt it
//! keeps. The comparison then carries the measurements, the commitment's
//! `sha256` and the salt. The engine computes the digest again from the
//! request alone, refuses a request whose measurements are not the committed
//! ones before any physics (`CommitmentDoesNotMatch`), and the validation
//! seal covers the commitment. A commitment shows that the data were not
//! changed after it was made. When it was made is shown by where its sha256
//! was published, not by the engine; the salt keeps the data from being found
//! by trying values against the published sha256 before they are opened.
//!
//! The committed bytes, so that a program with any SHA-256 makes the same:
//!
//! 1. `zero-trust-physics/measurement-commitment/v1` and a line break;
//! 2. the sealed form (`crate::sealed_form`) of the object
//!    `{"component": <the request's component, or null>, "measurements":
//!    <the request's measurements array>, "quantity": <the request's
//!    quantity>}`, so each number is the exact 64-bit value the seal takes and
//!    no spelling, spacing or key order of the JSON moves it;
//! 3. a line break;
//! 4. the salt's bytes: its hex digits decoded, 16 to 64 bytes.

use serde_json::{Map, Value, json};

use crate::json_text::is_lower_hex;
use crate::sealed_form::{digest_hex, sealed_form};

/// The first line of the committed bytes: what they are for, and the version
/// of the rule.
pub const DOMAIN_LINE: &str = "zero-trust-physics/measurement-commitment/v1\n";
/// The key of a commitment in a request of `validate_series`.
pub const COMMITMENT: &str = "commitment";
/// The fields of a commitment, each required, in the order they are read.
pub const FIELDS: [&str; 2] = ["sha256", "salt"];
/// The fewest hex digits of a salt: 16 bytes, past what trying every salt finds.
pub const LEAST_SALT_DIGITS: usize = 32;
/// The most hex digits of a salt: 64 bytes.
pub const MOST_SALT_DIGITS: usize = 128;

/// The refusal names of a commitment.
pub const MALFORMED: &str = "CommitmentMalformed";
pub const SALT_TOO_SHORT: &str = "CommitmentSaltTooShort";
pub const DOES_NOT_MATCH: &str = "CommitmentDoesNotMatch";
/// A required field left out, as every request names it.
pub const INPUT_MISSING: &str = "InputMissing";

/// What an answer says of a commitment it opened, under `commitment_opened`.
pub const OPENED: &str = "The measurements, the quantity and the component of this request are the ones committed: the SHA-256 of the tag zero-trust-physics/measurement-commitment/v1, their committed text and the salt is commitment.sha256, and validation_seal covers the commitment. A commitment shows that the data were not changed after it was made; when it was made is shown by where its sha256 was published before this comparison, not by the engine.";

/// A commitment as a request carries it: the digest and the salt, each in
/// lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commitment {
    pub sha256: String,
    pub salt: String,
}

/// Why a commitment is not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitmentFault {
    /// A field of the commitment is left out. `field` is its whole path.
    Missing { field: String },
    /// The commitment, or a field of it, is not of the commitment's form.
    Malformed { field: String, says: String },
    /// The salt has fewer than `LEAST_SALT_DIGITS` hex digits.
    SaltTooShort { field: String, digits: usize },
    /// The digest of the request's measurements, quantity, component and
    /// salt is not the commitment's `sha256`.
    DoesNotMatch { computed: String, committed: String },
}

impl CommitmentFault {
    /// The name the refusal carries.
    pub fn name(&self) -> &'static str {
        match self {
            CommitmentFault::Missing { .. } => INPUT_MISSING,
            CommitmentFault::Malformed { .. } => MALFORMED,
            CommitmentFault::SaltTooShort { .. } => SALT_TOO_SHORT,
            CommitmentFault::DoesNotMatch { .. } => DOES_NOT_MATCH,
        }
    }
}

impl std::fmt::Display for CommitmentFault {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommitmentFault::Missing { field } => {
                write!(out, "Missing required parameter: '{field}' ({INPUT_MISSING}).")
            }
            CommitmentFault::Malformed { field, says } => {
                write!(out, "Physical Violation: '{field}' {says} ({MALFORMED}).")
            }
            CommitmentFault::SaltTooShort { field, digits } => write!(
                out,
                "Physical Violation: '{field}' is {digits} hex digits, and a salt is at least {LEAST_SALT_DIGITS}, 16 bytes: a shorter salt leaves the committed data to be found by trying salts against the sha256 ({SALT_TOO_SHORT})."
            ),
            CommitmentFault::DoesNotMatch { computed, committed } => write!(
                out,
                "Physical Violation: the measurements, the quantity and the component of this request are not the ones committed: the SHA-256 of the tag, their committed text and the salt is {computed}, and commitment.sha256 is {committed}. Nothing is computed and nothing is sealed ({DOES_NOT_MATCH})."
            ),
        }
    }
}

fn malformed(field: String, says: &str) -> CommitmentFault {
    CommitmentFault::Malformed { field, says: says.to_string() }
}

/// Read a commitment at `at` (`commitment` in a request, `inputs.commitment`
/// in an answer). Every field is required and none is filled in.
pub fn read(value: &Value, at: &str) -> Result<Commitment, CommitmentFault> {
    let object = value
        .as_object()
        .ok_or_else(|| malformed(at.to_string(), "must be a JSON object of sha256 and salt"))?;
    for key in object.keys() {
        if !FIELDS.contains(&key.as_str()) {
            return Err(malformed(
                format!("{at}.{key}"),
                "is not a field of a commitment: its fields are sha256 and salt",
            ));
        }
    }
    for field in FIELDS {
        if !object.contains_key(field) {
            return Err(CommitmentFault::Missing { field: format!("{at}.{field}") });
        }
    }
    let sha256 = match &object["sha256"] {
        Value::String(text) if is_lower_hex(text, 64) => text.clone(),
        _ => {
            return Err(malformed(
                format!("{at}.sha256"),
                "must be 64 lowercase hex digits, the SHA-256 committed",
            ));
        }
    };
    let field = format!("{at}.salt");
    let salt = match &object["salt"] {
        Value::String(text) => text.clone(),
        _ => return Err(malformed(field, "must be a string of lowercase hex digits")),
    };
    if !salt.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(malformed(field, "must be lowercase hex digits"));
    }
    if salt.len() < LEAST_SALT_DIGITS {
        return Err(CommitmentFault::SaltTooShort { field, digits: salt.len() });
    }
    if salt.len() % 2 != 0 {
        return Err(malformed(field, "must be an even count of hex digits, whole bytes"));
    }
    if salt.len() > MOST_SALT_DIGITS {
        return Err(malformed(field, "must be at most 128 hex digits, 64 bytes"));
    }
    Ok(Commitment { sha256, salt })
}

/// The commitment of a request's object, if it carries one.
pub fn read_in(object: &Map<String, Value>, at: &str) -> Result<Option<Commitment>, CommitmentFault> {
    object.get(COMMITMENT).map(|value| read(value, at)).transpose()
}

/// The text of the measurements a commitment is taken over: the sealed form
/// of `{"component", "measurements", "quantity"}`. `component` is the
/// request's component, or null when it has none.
pub fn committed_text(quantity: &str, component: &Value, measurements: &Value) -> String {
    sealed_form(&json!({
        "component": component,
        "measurements": measurements,
        "quantity": quantity,
    }))
}

/// The committed bytes: the tag line, the committed text, a line break and
/// the salt's bytes.
pub fn committed_bytes(quantity: &str, component: &Value, measurements: &Value, salt: &str) -> Vec<u8> {
    let mut bytes = DOMAIN_LINE.as_bytes().to_vec();
    bytes.extend_from_slice(committed_text(quantity, component, measurements).as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(&hex::decode(salt).expect("a salt is read as whole bytes of hex"));
    bytes
}

/// SHA-256 of the committed bytes, as 64 lowercase hex digits.
pub fn commitment_sha256(quantity: &str, component: &Value, measurements: &Value, salt: &str) -> String {
    digest_hex(&committed_bytes(quantity, component, measurements, salt))
}

impl Commitment {
    /// The commitment opened by these measurements, or why it is not.
    pub fn open(&self, quantity: &str, component: &Value, measurements: &Value) -> Result<(), CommitmentFault> {
        let computed = commitment_sha256(quantity, component, measurements, &self.salt);
        if computed == self.sha256 {
            Ok(())
        } else {
            Err(CommitmentFault::DoesNotMatch { computed, committed: self.sha256.clone() })
        }
    }

    /// The sixth part of the validation seal, present only with a
    /// commitment: `commitment_sha256:<hex>` and `commitment_salt:<hex>`,
    /// each with a line break.
    pub fn seal_part(&self) -> String {
        format!("commitment_sha256:{}\ncommitment_salt:{}\n", self.sha256, self.salt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worked example of `protocol_mcp_gateway/REFERENCE.md`, "A
    /// commitment to the measurements".
    const SALT: &str = "000102030405060708090a0b0c0d0e0f";

    fn worked() -> Value {
        serde_json::from_str(r#"[{"step":1,"value":293.2},{"step":3,"value":293.3},{"step":5,"value":293.4}]"#).unwrap()
    }

    #[test]
    fn the_worked_commitment_is_the_published_text_and_digest() {
        let text = committed_text("temperature_k", &Value::Null, &worked());
        assert_eq!(
            text,
            r#"{"component":null,"measurements":[{"step":x3ff0000000000000,"value":x4072533333333333},{"step":x4008000000000000,"value":x407254cccccccccd},{"step":x4014000000000000,"value":x4072566666666666}],"quantity":"temperature_k"}"#
        );
        assert_eq!(text.len(), 221);
        let bytes = committed_bytes("temperature_k", &Value::Null, &worked(), SALT);
        assert_eq!(bytes.len(), 283);
        assert!(bytes.starts_with(b"zero-trust-physics/measurement-commitment/v1\n{\"component\":null,"));
        assert!(bytes.ends_with(&[b'}', b'\n', 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]));
        let sha256 = commitment_sha256("temperature_k", &Value::Null, &worked(), SALT);
        assert_eq!(sha256, WORKED_SHA256);
        let commitment = Commitment { sha256, salt: SALT.to_string() };
        assert_eq!(commitment.open("temperature_k", &Value::Null, &worked()), Ok(()));
    }

    const WORKED_SHA256: &str = "82937bf8c51c983ab59ea7d5414dc9d31d88147cd52ff4a986ce14ea386a95b9";

    #[test]
    fn spelling_spacing_and_key_order_do_not_move_the_digest_and_every_value_does() {
        let respelled: Value = serde_json::from_str(
            "[ {\"value\": 2.932e2, \"step\": 1.0}, {\"value\":293.30,\"step\":3}, {\"step\":5e0,\"value\":293.4} ]",
        )
        .unwrap();
        let digest = |quantity: &str, component: &Value, measurements: &Value, salt: &str| {
            commitment_sha256(quantity, component, measurements, salt)
        };
        let honest = digest("temperature_k", &Value::Null, &worked(), SALT);
        assert_eq!(digest("temperature_k", &Value::Null, &respelled, SALT), honest);

        // One digit of one measurement.
        let mut changed = worked();
        changed[1]["value"] = json!(293.31);
        assert_ne!(digest("temperature_k", &Value::Null, &changed, SALT), honest);
        // One unit in the last place of one measurement.
        let mut nudged = worked();
        nudged[2]["value"] = json!(f64::from_bits(293.4_f64.to_bits() + 1));
        assert_ne!(digest("temperature_k", &Value::Null, &nudged, SALT), honest);
        // A measurement moved to another step.
        let mut moved = worked();
        moved[1]["step"] = json!(4);
        assert_ne!(digest("temperature_k", &Value::Null, &moved, SALT), honest);
        // The rows in another order.
        let mut reordered = worked();
        reordered.as_array_mut().unwrap().swap(0, 2);
        assert_ne!(digest("temperature_k", &Value::Null, &reordered, SALT), honest);
        // A row dropped.
        let mut dropped = worked();
        dropped.as_array_mut().unwrap().pop();
        assert_ne!(digest("temperature_k", &Value::Null, &dropped, SALT), honest);
        // Another quantity, a component, another salt.
        assert_ne!(digest("ambient_k", &Value::Null, &worked(), SALT), honest);
        assert_ne!(digest("temperature_k", &json!(0), &worked(), SALT), honest);
        assert_ne!(digest("temperature_k", &Value::Null, &worked(), "000102030405060708090a0b0c0d0e0e"), honest);
        // A row of a plan of solves is another key than a step.
        let rows: Value = serde_json::from_str(r#"[{"row":1,"value":293.2},{"row":3,"value":293.3},{"row":5,"value":293.4}]"#).unwrap();
        assert_ne!(digest("temperature_k", &Value::Null, &rows, SALT), honest);
    }

    #[test]
    fn a_commitment_is_read_with_every_field_and_a_salt_of_at_least_sixteen_bytes() {
        let read_at = |value: Value| read(&value, "commitment");
        let sha = "ab".repeat(32);
        assert_eq!(
            read_at(json!({ "sha256": sha, "salt": SALT })),
            Ok(Commitment { sha256: sha.clone(), salt: SALT.to_string() })
        );
        for (value, name, sentence) in [
            (json!({ "sha256": sha }), INPUT_MISSING, "Missing required parameter: 'commitment.salt' (InputMissing)."),
            (json!({ "salt": SALT }), INPUT_MISSING, "Missing required parameter: 'commitment.sha256' (InputMissing)."),
            (json!({}), INPUT_MISSING, "Missing required parameter: 'commitment.sha256' (InputMissing)."),
            (
                json!({ "sha256": sha, "salt": "00010203040506070809" }),
                SALT_TOO_SHORT,
                "Physical Violation: 'commitment.salt' is 20 hex digits, and a salt is at least 32, 16 bytes: a shorter salt leaves the committed data to be found by trying salts against the sha256 (CommitmentSaltTooShort).",
            ),
            (
                json!({ "sha256": sha, "salt": "" }),
                SALT_TOO_SHORT,
                "Physical Violation: 'commitment.salt' is 0 hex digits, and a salt is at least 32, 16 bytes: a shorter salt leaves the committed data to be found by trying salts against the sha256 (CommitmentSaltTooShort).",
            ),
            (
                json!({ "sha256": sha, "salt": SALT, "pepper": "00" }),
                MALFORMED,
                "Physical Violation: 'commitment.pepper' is not a field of a commitment: its fields are sha256 and salt (CommitmentMalformed).",
            ),
            (
                json!("ab"),
                MALFORMED,
                "Physical Violation: 'commitment' must be a JSON object of sha256 and salt (CommitmentMalformed).",
            ),
            (
                Value::Null,
                MALFORMED,
                "Physical Violation: 'commitment' must be a JSON object of sha256 and salt (CommitmentMalformed).",
            ),
        ] {
            let fault = read_at(value.clone()).unwrap_err();
            assert_eq!((fault.name(), fault.to_string().as_str()), (name, sentence), "{value}");
        }
        for (value, field) in [
            (json!({ "sha256": sha.to_uppercase(), "salt": SALT }), "sha256"),
            (json!({ "sha256": &sha[..62], "salt": SALT }), "sha256"),
            (json!({ "sha256": 1, "salt": SALT }), "sha256"),
            (json!({ "sha256": sha, "salt": SALT.to_uppercase() }), "salt"),
            (json!({ "sha256": sha, "salt": format!("{SALT}0") }), "salt"),
            (json!({ "sha256": sha, "salt": "0".repeat(130) }), "salt"),
            (json!({ "sha256": sha, "salt": 12 }), "salt"),
            (json!({ "sha256": sha, "salt": null }), "salt"),
        ] {
            let fault = read_at(value.clone()).unwrap_err();
            assert_eq!(fault.name(), MALFORMED, "{value}: {fault}");
            assert!(fault.to_string().starts_with(&format!("Physical Violation: 'commitment.{field}' ")), "{fault}");
        }
        // The longest salt is taken.
        assert!(read_at(json!({ "sha256": sha, "salt": "0".repeat(128) })).is_ok());
    }

    #[test]
    fn a_commitment_opened_by_other_measurements_is_refused_by_name_with_both_digests() {
        let sha256 = commitment_sha256("temperature_k", &Value::Null, &worked(), SALT);
        let commitment = Commitment { sha256: sha256.clone(), salt: SALT.to_string() };
        let mut changed = worked();
        changed[0]["value"] = json!(293.3);
        let fault = commitment.open("temperature_k", &Value::Null, &changed).unwrap_err();
        assert_eq!(fault.name(), DOES_NOT_MATCH);
        let computed = commitment_sha256("temperature_k", &Value::Null, &changed, SALT);
        assert_eq!(
            fault.to_string(),
            format!("Physical Violation: the measurements, the quantity and the component of this request are not the ones committed: the SHA-256 of the tag, their committed text and the salt is {computed}, and commitment.sha256 is {sha256}. Nothing is computed and nothing is sealed (CommitmentDoesNotMatch).")
        );
        let wrong_salt = Commitment { sha256, salt: "ff".repeat(16) };
        assert_eq!(wrong_salt.open("temperature_k", &Value::Null, &worked()).unwrap_err().name(), DOES_NOT_MATCH);
    }

    #[test]
    fn the_seal_part_is_two_lines() {
        let commitment = Commitment { sha256: "ab".repeat(32), salt: SALT.to_string() };
        assert_eq!(
            commitment.seal_part(),
            format!("commitment_sha256:{}\ncommitment_salt:{SALT}\n", "ab".repeat(32))
        );
    }
}
