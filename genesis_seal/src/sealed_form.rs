//! The sealed form of a request, and the digest taken over it.
//!
//! The sealed form is a text written for hashing only. It is not JSON, and it
//! is never sent or returned.

use serde_json::Value;
use sha2::{Digest, Sha384};

/// The text a request is hashed as.
///
/// Object keys are sorted at every depth by their UTF-8 bytes. Arrays keep
/// their order. A string is its JSON encoding. `true`, `false` and `null` are
/// themselves. Every number is `x` followed by the 16 lowercase hex digits of
/// the bits of the nearest 64-bit float, so `10`, `10.0` and `1e1` are one
/// text, and `0.0` and `-0.0` are two. There are no spaces.
///
/// No JSON value starts with `x`, so a number is never the text of a string,
/// an object or an array.
pub fn sealed_form(value: &Value) -> String {
    let mut out = String::new();
    write_sealed_form(&mut out, value);
    out
}

fn write_sealed_form(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => {
            let nearest = number.as_f64().expect("a JSON number has a nearest f64");
            out.push('x');
            out.push_str(&format!("{:016x}", nearest.to_bits()));
        }
        Value::String(text) => out.push_str(&json_string(text)),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_sealed_form(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&json_string(key));
                out.push(':');
                write_sealed_form(out, &map[key]);
            }
            out.push('}');
        }
    }
}

/// A string as JSON writes it, between double quotes.
pub fn json_string(text: &str) -> String {
    serde_json::to_string(text).expect("a string encodes as JSON")
}

/// SHA-384 of `bytes`.
pub fn digest(bytes: &[u8]) -> [u8; 48] {
    let hashed = Sha384::digest(bytes);
    let mut out = [0_u8; 48];
    out.copy_from_slice(&hashed);
    out
}

/// SHA-384 of `bytes` as 96 lowercase hex digits.
pub fn digest_hex(bytes: &[u8]) -> String {
    hex::encode(Sha384::digest(bytes))
}

/// SHA-384 of the sealed form of `request`. For a run this is `link_0`.
pub fn request_digest(request: &Value) -> [u8; 48] {
    digest(sealed_form(request).as_bytes())
}

/// 32 bytes from 96 lowercase hex digits.
pub fn hash_from_hex(text: &str) -> Option<[u8; 48]> {
    if !crate::json_text::is_lower_hex(text, 96) {
        return None;
    }
    let bytes = hex::decode(text).ok()?;
    let mut out = [0_u8; 48];
    out.copy_from_slice(&bytes);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_worked_run_request_seals_to_the_published_text_and_hash() {
        let request: Value = serde_json::from_str(
            r#"{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":10,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}"#,
        )
        .unwrap();
        let text = sealed_form(&request);
        assert_eq!(
            text,
            r#"{"card":"thermal","domain":"energy","dt_s":x3fb999999999999a,"initial":{"temperature_k":x4072526666666666},"record_every":x3ff0000000000000,"segments":[{"load":{"ambient_k":x4072526666666666,"heat_power_w":x3ff0000000000000},"steps":x4024000000000000}]}"#
        );
        assert_eq!(text.len(), 253);
        assert_eq!(
            digest_hex(text.as_bytes()),
            "8615887c32e2e2edb7d4be74b822e7f64ea384173a9332d97be6278dec255c3d57c9249ca16c5a5d5dfa6c2e43d9baf9"
        );
        assert_eq!(
            hex::encode(&request_digest(&request)[..16]),
            "8615887c32e2e2edb7d4be74b822e7f6"
        );
    }

    #[test]
    fn the_worked_step_request_seals_to_the_published_text_and_hash() {
        let request: Value = serde_json::from_str(
            r#"{"domain":"energy","card":"thermal","parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}"#,
        )
        .unwrap();
        let text = sealed_form(&request);
        assert_eq!(
            text,
            r#"{"card":"thermal","domain":"energy","parameters":{"ambient_k":x4072526666666666,"dt_s":x3fb999999999999a,"heat_power_w":x3ff0000000000000,"temperature_k":x4072526666666666}}"#
        );
        assert_eq!(
            digest_hex(text.as_bytes()),
            "4a23f3716ec13d3ba102e1cc87ef110874c8ca3b46a01a8b6970281aa79e3db2de38071b7e8601a4ec4871897d2863be"
        );
    }

    #[test]
    fn the_worked_solve_request_seals_to_the_published_text_and_hash() {
        let request: Value =
            serde_json::from_str(r#"{"solver":"atom_dft","parameters":{"atomic_number":10}}"#)
                .unwrap();
        let text = sealed_form(&request);
        assert_eq!(
            text,
            r#"{"parameters":{"atomic_number":x4024000000000000},"solver":"atom_dft"}"#
        );
        assert_eq!(
            digest_hex(text.as_bytes()),
            "cc0f7a4eb3bc318555e9adb45607d6327cd58121ddd5e0ba212764a4492d13e0b777ca71a6c2fe290a79511cf209bf38"
        );
    }

    #[test]
    fn strings_are_their_json_encoding_and_keys_are_ordered_by_their_bytes() {
        assert_eq!(
            sealed_form(&json!([null, true, false, "a\"b\n", -0.0])),
            r#"[null,true,false,"a\"b\n",x8000000000000000]"#
        );
        assert_eq!(
            sealed_form(&json!(
                "\u{0001}\u{001f}\u{0008}\t\n\u{000c}\r\\/\u{007f}\u{00e9}"
            )),
            "\"\\u0001\\u001f\\b\\t\\n\\f\\r\\\\/\u{007f}\u{00e9}\""
        );
        assert_eq!(
            sealed_form(&json!({ "b": 1, "\u{00e9}": 2, "aa": 3, "a": 4, "B": 5 })),
            "{\"B\":x4014000000000000,\"a\":x4010000000000000,\"aa\":x4008000000000000,\"b\":x3ff0000000000000,\"\u{00e9}\":x4000000000000000}"
        );
    }

    #[test]
    fn one_number_has_one_sealed_form_however_it_is_written() {
        for (spellings, hex) in [
            (["0", "0.0", "0e0"], "0000000000000000"),
            (["-0", "-0.0", "-0e0"], "8000000000000000"),
            (["10", "10.0", "1e1"], "4024000000000000"),
            (["0.1", "0.10", "1e-1"], "3fb999999999999a"),
        ] {
            for spelling in spellings {
                let value: Value = serde_json::from_str(spelling).unwrap();
                assert_eq!(sealed_form(&value), format!("x{hex}"), "{spelling}");
            }
        }
    }

    #[test]
    fn negative_zero_seals_apart_from_zero() {
        let zero: Value = serde_json::from_str(r#"{"heat_power_w":0.0}"#).unwrap();
        let negative: Value = serde_json::from_str(r#"{"heat_power_w":-0.0}"#).unwrap();
        assert_ne!(sealed_form(&zero), sealed_form(&negative));
        assert_ne!(request_digest(&zero), request_digest(&negative));
    }

    #[test]
    fn a_number_never_seals_the_same_as_a_string() {
        let number = sealed_form(&json!(293.15));
        assert_eq!(number, "x4072526666666666");
        assert!(serde_json::from_str::<Value>(&number).is_err());
        for other in [
            json!("x4072526666666666"),
            json!("293.15"),
            json!("4072526666666666"),
            json!([293.15]),
            json!({ "x4072526666666666": 293.15 }),
            json!(null),
            json!(true),
        ] {
            let text = sealed_form(&other);
            assert_ne!(text, number, "{other}");
            assert!(!text.starts_with('x'), "{other} is sealed as {text}");
            let held = sealed_form(&json!({ "temperature_k": other }));
            assert_ne!(held, r#"{"temperature_k":x4072526666666666}"#, "{other}");
        }
    }

    #[test]
    fn a_whole_number_no_float_holds_seals_as_the_nearest_float() {
        for (token, hex) in [
            ("9007199254740992", "4340000000000000"),
            ("9007199254740993", "4340000000000000"),
            ("18446744073709551615", "43f0000000000000"),
            ("18446744073709551616", "43f0000000000000"),
            ("-9223372036854775809", "c3e0000000000000"),
        ] {
            let parameters: Value =
                serde_json::from_str(&format!(r#"{{"heat_power_w":{token}}}"#)).unwrap();
            assert_eq!(
                sealed_form(&parameters),
                format!(r#"{{"heat_power_w":x{hex}}}"#),
                "{token}"
            );
        }
    }

    #[test]
    fn the_order_and_spacing_of_a_request_do_not_change_its_digest() {
        let first: Value = serde_json::from_str(
            r#"{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1}"#,
        )
        .unwrap();
        let second: Value = serde_json::from_str(
            "{ \"dt_s\": 1e-1,\n \"initial\": {\"temperature_k\": 2.9315e2},\n \"card\": \"thermal\", \"domain\": \"energy\" }",
        )
        .unwrap();
        assert_eq!(request_digest(&first), request_digest(&second));
    }

    #[test]
    fn a_hash_is_read_from_96_lowercase_hex_digits_only() {
        let text = "8615887c32e2e2edb7d4be74b822e7f64ea384173a9332d97be6278dec255c3d57c9249ca16c5a5d5dfa6c2e43d9baf9";
        assert_eq!(hex::encode(hash_from_hex(text).unwrap()), text);
        assert!(hash_from_hex(&text.to_uppercase()).is_none());
        assert!(hash_from_hex(&text[..94]).is_none());
    }
}
