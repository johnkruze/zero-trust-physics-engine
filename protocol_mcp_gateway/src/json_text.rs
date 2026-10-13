//! Read JSON text closely enough to see a repeated key, and a number that is
//! written as non-zero and reads as zero.
//!
//! `serde_json` keeps the last copy of a key and drops the earlier ones.
//! A request, a tape, and a solution are scanned before that happens. An
//! escape such as `\u0061` is the same key as `a`. Each object has its own set.
//!
//! `serde_json` reads `1e-400` as `0.0` and keeps no trace of the token. The
//! same scan finds that token in the text and names the field it sits under.

use std::collections::HashSet;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

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

/// `true` when `text` is made only of hex digits and at least one is a capital
/// letter, so that it would be read as hex but is not written in lowercase.
pub fn has_capital_hex(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_hexdigit())
        && text.bytes().any(|byte| matches!(byte, b'A'..=b'F'))
}

/// `true` when `text` is exactly `len` characters from `0-9` and `a-f`.
pub fn is_lower_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// A field name is lowercase letters, digits, and underscores. A line break,
/// a colon, or any other character is refused before a published-name lookup.
pub fn is_plain_field_name(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

/// Bool fields a card state publishes. A present flag that is not a JSON
/// boolean is refused by name. A missing flag is left for the digest.
pub fn is_flag(key: &str) -> bool {
    matches!(
        key,
        "surge"
            | "derated"
            | "stroke_limited"
            | "pll_slipped"
            | "column_soft"
            | "column_limited"
            | "tracking_degraded"
            | "control_limited"
            | "sag_warning"
            | "past_fitting_span"
            | "drogue_mach_window"
            | "past_flight_deployment"
            | "in_earth_shadow"
    )
}

pub fn repeated_message(key: &str) -> String {
    format!("Physical Violation: the key '{key}' is repeated.")
}

pub fn reads_as_zero_message(field: &str, token: &str) -> String {
    format!(
        "Physical Violation: '{field}' is written {token}, a number that is not zero and reads as zero."
    )
}

/// Scan `bytes` for a repeated object key at any depth, then for a number
/// that is written as non-zero and reads as zero.
///
/// Trailing junk after one value is [`Scan::Unreadable`], so the caller can
/// report it the same way it reports a line that is not JSON. A repeated key
/// is reported even when more bytes follow the value.
pub fn scan(bytes: &[u8]) -> Scan {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let parsed: Result<ScanVisitor, _> = Deserialize::deserialize(&mut deserializer);
    match parsed {
        Ok(_) => match deserializer.end() {
            Ok(()) => match number_read_as_zero(bytes) {
                Some((field, token)) => Scan::ReadsAsZero { field, token },
                None => Scan::Clean,
            },
            Err(error) => Scan::Unreadable(error.to_string()),
        },
        Err(error) => match repeated_key(&error.to_string()) {
            Some(key) => Scan::Repeated(key),
            None => Scan::Unreadable(error.to_string()),
        },
    }
}

fn repeated_key(message: &str) -> Option<String> {
    const MARK: &str = "repeated key ";
    let start = message.find(MARK)? + MARK.len();
    let rest = &message[start..];
    let end = rest.find(" at line").unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some(rest[..end].to_string())
}

/// Where the scan stands: inside an object, under its latest key, or inside
/// an array, at an index under the key that holds the array.
enum Place {
    Object { key: String },
    Array { key: String, index: usize },
}

/// The name a number at this place is refused under: its key, or its key and
/// index. The same names `walk_physics` uses.
fn field_name(places: &[Place]) -> String {
    match places.last() {
        Some(Place::Object { key }) => key.clone(),
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
        && serde_json::from_slice::<f64>(token).is_ok_and(|read| read == 0.0)
}

/// The first number that is written as non-zero and reads as zero, as
/// `(field, token)`. `bytes` is one JSON value that has already been read
/// once, so every quote closes and every number is well formed.
fn number_read_as_zero(bytes: &[u8]) -> Option<(String, String)> {
    let mut places: Vec<Place> = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                let start = index;
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += if bytes[index] == b'\\' { 2 } else { 1 };
                }
                index += 1;
                let mut after = index;
                while after < bytes.len() && bytes[after].is_ascii_whitespace() {
                    after += 1;
                }
                if bytes.get(after) == Some(&b':') {
                    if let Some(Place::Object { key }) = places.last_mut() {
                        *key = serde_json::from_slice(bytes.get(start..index)?).ok()?;
                    }
                }
            }
            b'{' => {
                places.push(Place::Object { key: String::new() });
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
                if let Some(Place::Array { index: item, .. }) = places.last_mut() {
                    *item += 1;
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
                if token_reads_as_zero(token) {
                    return Some((
                        field_name(&places),
                        String::from_utf8_lossy(token).into_owned(),
                    ));
                }
            }
            _ => index += 1,
        }
    }
    None
}

struct ScanVisitor;

impl<'de> Deserialize<'de> for ScanVisitor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ScanVisitor).map(|()| ScanVisitor)
    }
}

impl<'de> Visitor<'de> for ScanVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_string<E>(self, _value: String) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|_: ScanVisitor| ())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element::<ScanVisitor>()?.is_some() {}
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!("repeated key {key}")));
            }
            let _value: ScanVisitor = map.next_value()?;
        }
        Ok(())
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
            scan(br#"{"a":1,"\u0061":2}"#),
            Scan::Repeated("a".to_string())
        );
        assert!(matches!(scan(br#"{"a":1e400}"#), Scan::Unreadable(_)));
        assert!(matches!(scan(br#"{"a":1,}"#), Scan::Unreadable(_)));
        assert!(matches!(scan(br#"+1"#), Scan::Unreadable(_)));
        assert!(matches!(scan(br#"{"a":1}{"b":2}"#), Scan::Unreadable(_)));
        let Scan::Unreadable(reason) = scan(br#"{"a":1e400}"#) else {
            panic!("1e400 parsed");
        };
        assert!(reason.contains("out of range"), "{reason}");
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
        // The smallest subnormal is read as itself. Float armor refuses it later.
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
        assert_eq!(found(r#"{"\u0061\"b":1e-400}"#), "a\"b");
        assert_eq!(
            scan(br#"{"note":"1e-400","n\"1e-400":[true,false,null,"\\",1]}"#),
            Scan::Clean
        );
        assert_eq!(
            reads_as_zero_message("heat_power_w", "1e-400"),
            "Physical Violation: 'heat_power_w' is written 1e-400, a number that is not zero and reads as zero."
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
        assert!(is_flag("derated"));
        assert!(!is_flag("temperature_k"));
    }
}
