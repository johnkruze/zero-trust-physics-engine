//! Read JSON text closely enough to see a repeated key, and a number that is
//! written as non-zero and reads as zero.
//!
//! `serde_json` keeps the last copy of a key and drops the earlier ones. A
//! request, a tape, a plan and a row are scanned before that happens. An
//! escape such as `a` is the same key as `a`. Each object has its own set.
//!
//! `serde_json` reads `1e-400` as `0.0` and keeps no trace of the token. The
//! same scan finds that token in the text and names the field it sits under.

use std::collections::HashSet;
use std::fmt;

use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum Scan {
    /// One JSON value, and no object repeats a key.
    Clean,
    /// An object repeats `key`. Nothing should be sealed.
    Repeated(String),
    /// `token` has a non-zero digit and reads as zero. `field` is the key it
    /// sits under. Nothing should be computed.
    ReadsAsZero { field: String, token: String },
    /// The text is not one JSON value. The string is the JSON reader's reason.
    Unreadable(String),
}

/// Why a text was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextRefusal {
    /// An object repeats a key.
    Repeated(String),
    /// A number is written as non-zero and reads as zero.
    ReadsAsZero { field: String, token: String },
    /// The text is not one JSON value.
    Unreadable(String),
}

impl fmt::Display for TextRefusal {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TextRefusal::Repeated(key) => {
                write!(out, "Physical Violation: the key '{key}' is repeated.")
            }
            TextRefusal::ReadsAsZero { field, token } => write!(
                out,
                "Physical Violation: '{field}' is written {token}, a number that is not zero and reads as zero."
            ),
            TextRefusal::Unreadable(reason) => {
                write!(out, "The text is not one JSON value: {reason}")
            }
        }
    }
}

/// `true` when `text` is exactly `len` characters from `0-9` and `a-f`.
pub fn is_lower_hex(text: &str, len: usize) -> bool {
    text.len() == len
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// `true` when `text` is made only of hex digits and at least one is a capital
/// letter, so that it would be read as hex but is not written in lowercase.
pub fn has_capital_hex(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_hexdigit())
        && text.bytes().any(|byte| matches!(byte, b'A'..=b'F'))
}

/// A field name is lowercase letters, digits, and underscores. A line break,
/// a colon, or any other character is refused.
pub fn is_plain_field_name(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

/// Scan `bytes`, then read them. A text that is not [`Scan::Clean`] is
/// refused by name.
pub fn read(bytes: &[u8]) -> Result<Value, TextRefusal> {
    match scan(bytes) {
        Scan::Clean => serde_json::from_slice(bytes)
            .map_err(|error| TextRefusal::Unreadable(error.to_string())),
        Scan::Repeated(key) => Err(TextRefusal::Repeated(key)),
        Scan::ReadsAsZero { field, token } => Err(TextRefusal::ReadsAsZero { field, token }),
        Scan::Unreadable(reason) => Err(TextRefusal::Unreadable(reason)),
    }
}

/// Scan `bytes` for a repeated object key at any depth, then for a number
/// that is written as non-zero and reads as zero.
///
/// Trailing bytes after one value are [`Scan::Unreadable`]. A repeated key is
/// reported even when more bytes follow the value, and when the text stops
/// being JSON after the repeat.
pub fn scan(bytes: &[u8]) -> Scan {
    let mut stream = serde_json::Deserializer::from_slice(bytes).into_iter::<Value>();
    match stream.next() {
        Some(Ok(_)) => {
            let end = stream.byte_offset();
            let walked = walk(&bytes[..end]);
            if let Some(key) = walked.repeated {
                return Scan::Repeated(key);
            }
            if let Err(error) = serde_json::from_slice::<Value>(bytes) {
                return Scan::Unreadable(error.to_string());
            }
            match walked.reads_as_zero {
                Some((field, token)) => Scan::ReadsAsZero { field, token },
                None => Scan::Clean,
            }
        }
        Some(Err(error)) => {
            // The reader stopped inside the text. A key repeated before that
            // point was read first, so it is the fault that is named.
            let read = bytes_read(bytes, error.line(), error.column());
            match walk(&bytes[..read]).repeated {
                Some(key) => Scan::Repeated(key),
                None => Scan::Unreadable(error.to_string()),
            }
        }
        None => match serde_json::from_slice::<Value>(bytes) {
            Ok(_) => Scan::Clean,
            Err(error) => Scan::Unreadable(error.to_string()),
        },
    }
}

/// The count of bytes the JSON reader had taken when it stopped at `line` and
/// `column`. Lines count from 1. A column is the count of bytes into its line.
fn bytes_read(bytes: &[u8], line: usize, column: usize) -> usize {
    let mut line_start = 0;
    let mut lines_passed = 1;
    for (index, byte) in bytes.iter().enumerate() {
        if lines_passed == line {
            break;
        }
        if *byte == b'\n' {
            lines_passed += 1;
            line_start = index + 1;
        }
    }
    line_start.saturating_add(column).min(bytes.len())
}

/// Where the scan stands: inside an object, under its latest key, or inside
/// an array, at an index under the key that holds the array.
enum Place {
    Object {
        seen: HashSet<String>,
        key: String,
        next_string_is_key: bool,
    },
    Array {
        key: String,
        index: usize,
    },
}

struct Walked {
    repeated: Option<String>,
    reads_as_zero: Option<(String, String)>,
}

/// The name a number at this place is refused under: its key, or its key and
/// index.
fn field_name(places: &[Place]) -> String {
    match places.last() {
        Some(Place::Object { key, .. }) => key.clone(),
        Some(Place::Array { key, index }) => format!("{key}[{index}]"),
        None => "value".to_string(),
    }
}

/// `true` when the token has a non-zero digit before its exponent and the
/// number reader returns zero for it.
fn token_reads_as_zero(token: &[u8]) -> bool {
    let mantissa = token
        .split(|byte| matches!(byte, b'e' | b'E'))
        .next()
        .unwrap_or(token);
    mantissa.iter().any(|byte| matches!(byte, b'1'..=b'9'))
        && serde_json::from_slice::<f64>(token).is_ok_and(|read| read.to_bits() << 1 == 0)
}

/// Walk text the JSON reader has accepted, or the part of a text it accepted
/// before it stopped. The walk stops at the first repeated key.
fn walk(bytes: &[u8]) -> Walked {
    let mut places: Vec<Place> = Vec::new();
    let mut reads_as_zero = None;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                let start = index;
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += if bytes[index] == b'\\' { 2 } else { 1 };
                }
                if index >= bytes.len() {
                    break;
                }
                index += 1;
                if let Some(Place::Object {
                    seen,
                    key,
                    next_string_is_key,
                }) = places.last_mut()
                {
                    if *next_string_is_key {
                        let Ok(text) = serde_json::from_slice::<String>(&bytes[start..index])
                        else {
                            break;
                        };
                        if !seen.insert(text.clone()) {
                            return Walked {
                                repeated: Some(text),
                                reads_as_zero,
                            };
                        }
                        *key = text;
                        *next_string_is_key = false;
                    }
                }
            }
            b'{' => {
                places.push(Place::Object {
                    seen: HashSet::new(),
                    key: String::new(),
                    next_string_is_key: true,
                });
                index += 1;
            }
            b'[' => {
                let key = field_name(&places);
                places.push(Place::Array { key, index: 0 });
                index += 1;
            }
            b'}' | b']' => {
                places.pop();
                index += 1;
            }
            b',' => {
                match places.last_mut() {
                    Some(Place::Array { index: item, .. }) => *item += 1,
                    Some(Place::Object {
                        next_string_is_key, ..
                    }) => *next_string_is_key = true,
                    None => {}
                }
                index += 1;
            }
            b'-' | b'0'..=b'9' => {
                let start = index;
                while index < bytes.len()
                    && matches!(bytes[index], b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E')
                {
                    index += 1;
                }
                let token = &bytes[start..index];
                if reads_as_zero.is_none() && token_reads_as_zero(token) {
                    reads_as_zero = Some((
                        field_name(&places),
                        String::from_utf8_lossy(token).into_owned(),
                    ));
                }
            }
            _ => index += 1,
        }
    }
    Walked {
        repeated: None,
        reads_as_zero,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nested_repeat_and_a_unicode_escape_are_the_same_key() {
        assert_eq!(scan(br#"{"a":1,"b":2}"#), Scan::Clean);
        assert_eq!(
            scan(br#"{"outer":{"k":1,"k":2}}"#),
            Scan::Repeated("k".to_string())
        );
        assert_eq!(
            scan(br#"{"a":[{"b":1,"b":2}]}"#),
            Scan::Repeated("b".to_string())
        );
        assert_eq!(
            scan(br#"{"a":1,"a":2}"#),
            Scan::Repeated("a".to_string())
        );
        assert!(matches!(scan(br#"{"a":1e400}"#), Scan::Unreadable(_)));
        assert!(matches!(scan(br#"{"a":1,}"#), Scan::Unreadable(_)));
        assert!(matches!(scan(br#"+1"#), Scan::Unreadable(_)));
        assert!(matches!(scan(br#"{"a":1}{"b":2}"#), Scan::Unreadable(_)));
        assert!(matches!(scan(b""), Scan::Unreadable(_)));
        let Scan::Unreadable(reason) = scan(br#"{"a":1e400}"#) else {
            panic!("1e400 parsed");
        };
        assert!(reason.contains("out of range"), "{reason}");
    }

    #[test]
    fn the_same_key_in_two_objects_is_not_a_repeat() {
        assert_eq!(scan(br#"{"a":{"k":1},"b":{"k":2}}"#), Scan::Clean);
        assert_eq!(scan(br#"[{"k":1},{"k":2}]"#), Scan::Clean);
        assert_eq!(scan(br#"{"k":"k","j":["k","k"]}"#), Scan::Clean);
        assert_eq!(scan(br#"{"k":{"k":{"k":1}}}"#), Scan::Clean);
    }

    #[test]
    fn a_repeat_is_named_when_it_comes_before_the_fault_and_not_when_it_comes_after() {
        assert_eq!(
            scan(br#"{"a":1,"a":2}{"b":1}"#),
            Scan::Repeated("a".to_string())
        );
        assert_eq!(scan(br#"{"a":1,"a":2,}"#), Scan::Repeated("a".to_string()));
        assert_eq!(scan(br#"{"a":1,"a""#), Scan::Repeated("a".to_string()));
        assert_eq!(
            scan(b"{\"a\":1,\n\"b\":{\"c\":1,\n \"c\":2},\n\"d\":}"),
            Scan::Repeated("c".to_string())
        );
        assert!(matches!(
            scan(br#"{"a":1e400,"b":1,"b":2}"#),
            Scan::Unreadable(_)
        ));
        assert!(matches!(
            scan(br#"{"a":1,} {"b":1,"b":2}"#),
            Scan::Unreadable(_)
        ));
        assert!(matches!(scan(br#"{"a":1,"a"#), Scan::Unreadable(_)));
    }

    #[test]
    fn a_number_written_as_non_zero_that_reads_as_zero_is_found_by_name() {
        for zero in ["0", "0.0", "-0.0", "-0", "0e0", "0E+5", "0.000", "0e-400"] {
            let text = format!(r#"{{"heat_power_w":{zero}}}"#);
            assert_eq!(scan(text.as_bytes()), Scan::Clean, "{zero}");
        }
        for token in [
            "1e-400",
            "-1e-400",
            "1E-400",
            "0.1e-400",
            "4.9e-325",
            "1e-99999999999999999999",
        ] {
            let text = format!(r#"{{"dt_s":0.1,"heat_power_w":{token},"ambient_k":293.15}}"#);
            assert_eq!(
                scan(text.as_bytes()),
                Scan::ReadsAsZero {
                    field: "heat_power_w".to_string(),
                    token: token.to_string()
                },
                "{token}"
            );
        }
        // The smallest subnormal is read as itself.
        for stored in ["5e-324", "2.5e-324", "1e-320", "1e-18"] {
            let text = format!(r#"{{"dt_s":{stored}}}"#);
            assert_eq!(scan(text.as_bytes()), Scan::Clean, "{stored}");
        }

        let found = |text: &str| match scan(text.as_bytes()) {
            Scan::ReadsAsZero { field, .. } => field,
            other => panic!("{text} scanned as {other:?}"),
        };
        assert_eq!(found("1e-400"), "value");
        assert_eq!(found(r#"{"position_m":[0.0,1e-400,0.0]}"#), "position_m[1]");
        assert_eq!(found(r#"{"a":[[0,0],[0,1e-400]]}"#), "a[1][1]");
        assert_eq!(
            found(r#"{"segments":[{"steps":1,"load":{"flow_m3_s":1e-400}}]}"#),
            "flow_m3_s"
        );
        assert_eq!(found(r#"{"first":{"inner":1},"second":1e-400}"#), "second");
        assert_eq!(found(r#"{"a\"b":1e-400}"#), "a\"b");
        assert_eq!(
            scan(br#"{"note":"1e-400","n\"1e-400":[true,false,null,"\\",1]}"#),
            Scan::Clean
        );
        assert_eq!(
            TextRefusal::ReadsAsZero {
                field: "heat_power_w".to_string(),
                token: "1e-400".to_string()
            }
            .to_string(),
            "Physical Violation: 'heat_power_w' is written 1e-400, a number that is not zero and reads as zero."
        );
        assert_eq!(
            TextRefusal::Repeated("dt_s".to_string()).to_string(),
            "Physical Violation: the key 'dt_s' is repeated."
        );
    }

    #[test]
    fn a_field_name_is_lowercase_letters_digits_and_underscore() {
        assert!(is_plain_field_name("temperature_k"));
        assert!(is_plain_field_name("eigenvalue_1s_ha"));
        assert!(!is_plain_field_name(""));
        assert!(!is_plain_field_name("Temperature_k"));
        assert!(!is_plain_field_name("a:b"));
        assert!(!is_plain_field_name("a\nb"));
    }

    #[test]
    fn hex_digits_are_lowercase_and_exact() {
        assert!(is_lower_hex("4072526666666666", 16));
        assert!(!is_lower_hex("4072526666666666", 64));
        assert!(!is_lower_hex("407252666666666G", 16));
        assert!(!is_lower_hex("407252666666666A", 16));
    }

    #[test]
    fn capital_hex_is_found_only_in_text_that_would_read_as_hex() {
        assert!(has_capital_hex("407252666666666A"));
        assert!(has_capital_hex("Ab"));
        assert!(!has_capital_hex("407252666666666a"));
        assert!(!has_capital_hex("4072526666666666"));
        assert!(!has_capital_hex("407252666666666G"));
        assert!(!has_capital_hex("ABCDEFG"));
        assert!(!has_capital_hex(""));
    }
}
