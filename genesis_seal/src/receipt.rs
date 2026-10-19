//! The receipt seal.
//!
//! A receipt is SHA-384 of the exact JSON of every other field of a
//! response. The wire body is that document with `,"receipt_sha384":"<hex>"`
//! inserted before the closing brace. A receipt covers bytes: a response that
//! another JSON writer has written again no longer matches its receipt, though
//! its seals still hold.

use crate::json_text::is_lower_hex;
use crate::sealed_form::digest_hex;

/// The key of the receipt in an answer.
pub const RECEIPT_KEY: &str = "receipt_sha384";

const MARK: &[u8] = b",\"receipt_sha384\":\"";

/// What a receipt says about the bytes it travels with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptCheck {
    /// The bytes before the receipt hash to it.
    Matches,
    /// The bytes before the receipt do not hash to it.
    DoesNotMatch,
    /// The bytes do not end with a receipt.
    Absent,
}

impl ReceiptCheck {
    pub fn as_str(self) -> &'static str {
        match self {
            ReceiptCheck::Matches => "matches",
            ReceiptCheck::DoesNotMatch => "does not match",
            ReceiptCheck::Absent => "absent",
        }
    }
}

/// The receipt of `document`, the JSON object of every other field.
pub fn receipt_of(document: &str) -> String {
    digest_hex(document.as_bytes())
}

/// The wire body: `document` with its receipt inserted before the closing brace.
pub fn with_receipt(document: &str) -> String {
    let receipt = receipt_of(document);
    let mut wire = String::with_capacity(document.len() + receipt.len() + 22);
    wire.push_str(document.strip_suffix('}').unwrap_or(document));
    wire.push_str(",\"receipt_sha384\":\"");
    wire.push_str(&receipt);
    wire.push_str("\"}");
    wire
}

/// The receipt a wire body states, and the receipt its bytes make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptRead {
    /// The 96 hex digits at the end of the wire body.
    pub stated: String,
    /// SHA-384 of the wire body with that tail replaced by the closing brace.
    pub recomputed: String,
}

impl ReceiptRead {
    pub fn matches(&self) -> bool {
        self.stated == self.recomputed
    }
}

/// Read the receipt at the end of a wire body, a signature after it split
/// off first (`answer_signature`). `None` when the bytes do not end with `,"receipt_sha384":"<96 lowercase hex digits>"}`.
pub fn read_receipt(wire: &[u8]) -> Option<ReceiptRead> {
    // An answer the engine signed carries its signature right after the
    // receipt; the receipt is of the answer without it.
    if let Some(text) = std::str::from_utf8(wire).ok().filter(|text| text.contains("\",\"signature\":{")) {
        if let Ok(Some(split)) = crate::answer_signature::split(text) {
            return read_unsigned(split.unsigned.as_bytes());
        }
    }
    read_unsigned(wire)
}

fn read_unsigned(wire: &[u8]) -> Option<ReceiptRead> {
    // The tail is the mark, 96 hex digits, a quote and the closing brace.
    let tail_len = MARK.len() + 96 + 2;
    if wire.len() < tail_len + 1 {
        return None;
    }
    let (head, tail) = wire.split_at(wire.len() - tail_len);
    let rest = tail.strip_prefix(MARK)?;
    let hex = rest.strip_suffix(b"\"}")?;
    let hex = std::str::from_utf8(hex).ok()?;
    if !is_lower_hex(hex, 96) {
        return None;
    }
    let mut document = head.to_vec();
    document.push(b'}');
    Some(ReceiptRead {
        stated: hex.to_string(),
        recomputed: digest_hex(&document),
    })
}

/// Check the receipt at the end of a wire body.
pub fn check_receipt(wire: &[u8]) -> ReceiptCheck {
    match read_receipt(wire) {
        None => ReceiptCheck::Absent,
        Some(read) if read.matches() => ReceiptCheck::Matches,
        Some(_) => ReceiptCheck::DoesNotMatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_receipt_covers_the_bytes_before_it() {
        let document = r#"{"inputs":{"dt_s":0.1},"run_seal":"ab"}"#;
        let wire = with_receipt(document);
        assert_eq!(
            wire,
            format!(
                r#"{{"inputs":{{"dt_s":0.1}},"run_seal":"ab","receipt_sha384":"{}"}}"#,
                receipt_of(document)
            )
        );
        assert_eq!(check_receipt(wire.as_bytes()), ReceiptCheck::Matches);

        let changed = wire.replace("0.1", "0.2");
        assert_eq!(check_receipt(changed.as_bytes()), ReceiptCheck::DoesNotMatch);
        let respaced = wire.replace("\"dt_s\":", "\"dt_s\": ");
        assert_eq!(check_receipt(respaced.as_bytes()), ReceiptCheck::DoesNotMatch);
        assert_eq!(check_receipt(document.as_bytes()), ReceiptCheck::Absent);
        let with_newline = format!("{wire}\n");
        assert_eq!(check_receipt(with_newline.as_bytes()), ReceiptCheck::Absent);

        let read = read_receipt(wire.as_bytes()).unwrap();
        assert!(read.matches());
        assert_eq!(read.stated, receipt_of(document));
        let read = read_receipt(changed.as_bytes()).unwrap();
        assert!(!read.matches());
        assert_eq!(read.stated, receipt_of(document));
        assert_ne!(read.recomputed, read.stated);
        assert_eq!(read_receipt(document.as_bytes()), None);
    }
}
