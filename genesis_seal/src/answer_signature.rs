//! The engine's signature on an answer: ML-DSA-87 (FIPS 204) over its receipt.
//!
//! A computed answer ends with its receipt, `"receipt_sha384":"<96 hex>"`.
//! An engine given an answer key signs that receipt with an issue time, an
//! expiry and a nonce, and writes the signature right after the receipt, in
//! the same object:
//!
//! ```text
//! …,"seal_rule":"zero-trust-physics/seal/v2","receipt_sha384":"<96 hex>","signature":{"alg":"ML-DSA-87","key_id":"<96 hex>","issued_at":"2026-10-18T21:04:05Z","expires_at":"2026-10-18T22:04:05Z","nonce":"<64 hex>","value":"<base64>"}}
//! ```
//!
//! The answer without `,"signature":{…}` is the unsigned answer, byte for
//! byte, and its receipt covers every byte of it. The signed message is the
//! text [`message`] writes:
//!
//! ```text
//! zero-trust-physics/answer-signature/v1
//! key_id:<96 hex>
//! receipt_sha384:<96 hex>
//! issued_at:<YYYY-MM-DDTHH:MM:SSZ>
//! expires_at:<YYYY-MM-DDTHH:MM:SSZ>
//! nonce:<64 hex>
//! ```
//!
//! each line ended by a line break, signed by ML-DSA.Sign of FIPS 204 with an
//! empty context string, in its deterministic variant (the nonce makes every
//! message distinct). `key_id` is the SHA-384 of the 2,592-byte public key.
//! The key is a 32-byte seed, FIPS 204's ξ, held by the operator.
//!
//! The implementation is the RustCrypto `ml-dsa` crate, version 0.1.1: pure
//! Rust, FIPS 204 final, not validated under FIPS 140-3 (CMVP) and, as its
//! README says, never independently audited.

use crate::json_text::is_lower_hex;
use crate::receipt::{RECEIPT_KEY, read_receipt};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ml_dsa::{EncodedVerifyingKey, MlDsa87, Signature, VerifyingKey};
use sha2::{Digest, Sha384};

/// The algorithm an answer is signed with.
pub const ALGORITHM: &str = "ML-DSA-87";
/// The first line of the signed message.
pub const MESSAGE_TAG: &str = "zero-trust-physics/answer-signature/v1";
/// Bytes of an ML-DSA-87 public key.
pub const PUBLIC_KEY_LEN: usize = 2592;
/// Bytes of an ML-DSA-87 signature.
pub const SIGNATURE_LEN: usize = 4627;
/// Bytes of the seed a key is made from.
pub const SEED_LEN: usize = 32;
/// Bytes of a nonce.
pub const NONCE_LEN: usize = 32;
/// How long a signature holds by default, in seconds.
pub const DEFAULT_TTL_S: u64 = 3600;
/// The shortest and the longest time an operator may give a signature.
pub const LEAST_TTL_S: u64 = 60;
pub const MOST_TTL_S: u64 = 7 * 86_400;
/// How far ahead of the verifier's clock an issue time may be and still hold.
pub const CLOCK_ALLOWANCE_S: i64 = 60;
/// The key of the signature in an answer.
pub const SIGNATURE_KEY: &str = "signature";
/// The keys of a signature, in the order they are written.
pub const FIELDS: [&str; 6] = ["alg", "key_id", "issued_at", "expires_at", "nonce", "value"];

/// What a check of an answer's signature found, first failing first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignatureCause {
    /// The answer carries no signature and a key was given to check it with.
    Absent,
    /// The signature is not of the form: a key missing, out of order or
    /// added, a field of the wrong length or alphabet, a time not of the form.
    Malformed,
    /// `alg` is not `ML-DSA-87`.
    AlgorithmNotServed,
    /// No public key was given to check the signature with.
    KeyNotGiven,
    /// `key_id` is not the SHA-384 of the public key given.
    KeyMismatch,
    /// The bytes before the receipt do not hash to it.
    ReceiptDoesNotMatch,
    /// ML-DSA-87 verification of the message under the key given fails.
    DoesNotVerify,
    /// `issued_at` is later than the verifier's clock by more than the allowance.
    NotYetValid,
    /// The verifier's clock is past `expires_at`.
    Expired,
    /// The nonce is one the verifier has already seen.
    NonceReplayed,
}

impl SignatureCause {
    /// The name a refusal carries.
    pub fn name(self) -> &'static str {
        match self {
            SignatureCause::Absent => "SignatureAbsent",
            SignatureCause::Malformed => "SignatureMalformed",
            SignatureCause::AlgorithmNotServed => "SignatureAlgorithmNotServed",
            SignatureCause::KeyNotGiven => "SignatureKeyNotGiven",
            SignatureCause::KeyMismatch => "SignatureKeyMismatch",
            SignatureCause::ReceiptDoesNotMatch => "ReceiptDoesNotMatch",
            SignatureCause::DoesNotVerify => "SignatureDoesNotVerify",
            SignatureCause::NotYetValid => "SignatureNotYetValid",
            SignatureCause::Expired => "SignatureExpired",
            SignatureCause::NonceReplayed => "SignatureNonceReplayed",
        }
    }

    /// One sentence of what was found.
    pub fn sentence(self) -> &'static str {
        match self {
            SignatureCause::Absent => "The answer carries no signature, and a public key was given to verify one with.",
            SignatureCause::Malformed => "The signature is not of the form: alg, key_id (96 hex digits), issued_at and expires_at (YYYY-MM-DDTHH:MM:SSZ), nonce (64 hex digits) and value (the base64 of 4,627 bytes), in that order, right after the receipt.",
            SignatureCause::AlgorithmNotServed => "The signature names an algorithm other than ML-DSA-87, the one the engine signs with.",
            SignatureCause::KeyNotGiven => "No public key was given to verify the signature with.",
            SignatureCause::KeyMismatch => "The signature's key_id is not the SHA-384 of the public key given: it was made by another key, or claims to be.",
            SignatureCause::ReceiptDoesNotMatch => "The bytes before the receipt do not hash to it: the answer was changed after it was signed.",
            SignatureCause::DoesNotVerify => "The ML-DSA-87 signature does not verify over the receipt, the times and the nonce under the public key given.",
            SignatureCause::NotYetValid => "The signature's issue time is later than the verifier's clock.",
            SignatureCause::Expired => "The signature has expired: the verifier's clock is past its expires_at.",
            SignatureCause::NonceReplayed => "The signature's nonce is one the verifier has already seen: the answer was delivered before.",
        }
    }
}

/// An ML-DSA-87 public key.
#[derive(Clone)]
pub struct PublicKey {
    key: VerifyingKey<MlDsa87>,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({})", self.key_id())
    }
}

impl PublicKey {
    /// A key from its 2,592 encoded bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<PublicKey> {
        if bytes.len() != PUBLIC_KEY_LEN {
            return None;
        }
        let encoded = EncodedVerifyingKey::<MlDsa87>::try_from(bytes).ok()?;
        let key = VerifyingKey::<MlDsa87>::decode(&encoded);
        // pkDecode accepts every byte string of the length; a key whose
        // re-encoding differs is not one a seed made.
        if key.encode().as_slice() != bytes {
            return None;
        }
        Some(PublicKey { key, bytes: bytes.to_vec() })
    }

    /// A key from its base64 text (standard alphabet, padded), space around it ignored.
    pub fn from_base64(text: &str) -> Option<PublicKey> {
        let bytes = BASE64.decode(text.trim()).ok()?;
        PublicKey::from_bytes(&bytes)
    }

    /// The 2,592 encoded bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The base64 of the encoded key.
    pub fn to_base64(&self) -> String {
        BASE64.encode(&self.bytes)
    }

    /// SHA-384 of the encoded key, as 96 lowercase hex digits.
    pub fn key_id(&self) -> String {
        hex::encode(Sha384::digest(&self.bytes))
    }

    /// The key's public description: `{"alg","key_id","public_key"}`, compact.
    pub fn description_json(&self) -> String {
        format!(
            "{{\"alg\":\"{ALGORITHM}\",\"key_id\":\"{}\",\"public_key\":\"{}\"}}",
            self.key_id(),
            self.to_base64()
        )
    }

    fn verifies(&self, message: &[u8], signature: &[u8]) -> bool {
        match Signature::<MlDsa87>::try_from(signature) {
            Ok(signature) => self.key.verify_with_context(message, b"", &signature),
            Err(_) => false,
        }
    }
}

/// The signature of an answer, as written in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub alg: String,
    pub key_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub nonce: String,
    /// The base64 of the signature.
    pub value: String,
}

impl Envelope {
    /// The signature's object as written in an answer, compact: `{"alg":…,"value":…}`.
    pub fn json(&self) -> String {
        format!(
            "{{\"alg\":\"{}\",\"key_id\":\"{}\",\"issued_at\":\"{}\",\"expires_at\":\"{}\",\"nonce\":\"{}\",\"value\":\"{}\"}}",
            self.alg, self.key_id, self.issued_at, self.expires_at, self.nonce, self.value
        )
    }
}

/// The signed message of a receipt.
pub fn message(key_id: &str, receipt: &str, issued_at: &str, expires_at: &str, nonce: &str) -> String {
    format!(
        "{MESSAGE_TAG}\nkey_id:{key_id}\n{RECEIPT_KEY}:{receipt}\nissued_at:{issued_at}\nexpires_at:{expires_at}\nnonce:{nonce}\n"
    )
}

/// An answer split at its signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Split {
    /// The answer with `,"signature":{…}` taken out.
    pub unsigned: String,
    /// The signature, read field by field; `None` when it is not of the form.
    pub envelope: Option<Envelope>,
    /// The receipt the signature follows, 96 hex digits.
    pub receipt: String,
}

fn receipt_marks(text: &str) -> Vec<usize> {
    let mark = format!("\"{RECEIPT_KEY}\":\"");
    text.match_indices(&mark).map(|(at, _)| at + mark.len()).collect()
}

/// Split `text`, the object whose receipt is signed (a whole answer, or a
/// dataset's head), at its signature. `Ok(None)`: the text carries no
/// signature. `Err(Malformed)`: it carries `"signature"` after a receipt in a
/// form that is not the engine's, or more than one receipt is followed by one.
pub fn split(text: &str) -> Result<Option<Split>, SignatureCause> {
    let follow = format!("\",\"{SIGNATURE_KEY}\":");
    let mut found: Option<Split> = None;
    for start in receipt_marks(text) {
        let Some(receipt) = text.get(start..start + 96) else { continue };
        if !is_lower_hex(receipt, 96) {
            continue;
        }
        let after = start + 96;
        let Some(rest) = text.get(after..) else { continue };
        if !rest.starts_with(&follow) {
            continue;
        }
        if found.is_some() {
            return Err(SignatureCause::Malformed);
        }
        // `,"signature":` begins one byte after the closing quote of the receipt.
        let block_at = after + 1;
        let object_at = after + follow.len();
        let object_text = &text[object_at..];
        // The engine's signature holds no brace inside it.
        let Some(close) = object_text.find('}') else { return Err(SignatureCause::Malformed) };
        let object = &object_text[..=close];
        let end = object_at + close + 1;
        let mut unsigned = String::with_capacity(text.len());
        unsigned.push_str(&text[..block_at]);
        unsigned.push_str(&text[end..]);
        found = Some(Split {
            unsigned,
            envelope: read_envelope(object),
            receipt: receipt.to_string(),
        });
    }
    if found.is_none() && text.contains(&format!("\"{SIGNATURE_KEY}\":{{\"alg\"")) {
        return Err(SignatureCause::Malformed);
    }
    Ok(found)
}

/// Read a signature's object, written compact with its six keys in the
/// order [`FIELDS`] gives, into an envelope of the form; `None` when a key is
/// missing, added or out of order, or a field is not of the form.
pub fn read_envelope(object: &str) -> Option<Envelope> {
    let mut rest = object.strip_prefix('{')?.strip_suffix('}')?;
    let mut values = Vec::with_capacity(FIELDS.len());
    for (index, key) in FIELDS.iter().enumerate() {
        if index > 0 {
            rest = rest.strip_prefix(',')?;
        }
        rest = rest.strip_prefix(&format!("\"{key}\":\""))?;
        let close = rest.find('"')?;
        let value = &rest[..close];
        if value.contains('\\') {
            return None;
        }
        values.push(value.to_string());
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        return None;
    }
    let envelope = Envelope {
        alg: values[0].clone(),
        key_id: values[1].clone(),
        issued_at: values[2].clone(),
        expires_at: values[3].clone(),
        nonce: values[4].clone(),
        value: values[5].clone(),
    };
    let form = is_lower_hex(&envelope.key_id, 96)
        && is_lower_hex(&envelope.nonce, 2 * NONCE_LEN)
        && parse_time(&envelope.issued_at).is_some()
        && parse_time(&envelope.expires_at).is_some()
        && !envelope.alg.is_empty();
    form.then_some(envelope)
}

/// What the verifier brings to a check.
#[derive(Debug, Clone, Copy)]
pub struct Check<'a> {
    /// The key to check against; `None` names the signature `KeyNotGiven`.
    pub public_key: Option<&'a PublicKey>,
    /// The verifier's clock, in seconds since 1970-01-01T00:00:00Z.
    pub now_s: i64,
    /// Nonces the verifier has already seen, 64 hex digits each.
    pub seen_nonces: &'a [String],
}

/// What a check of an answer's signature found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureReport {
    /// The answer carries a signature after its receipt.
    pub present: bool,
    /// The first check that does not hold; `None` when every one holds.
    pub cause: Option<SignatureCause>,
    /// The signature as written, when it is of the form.
    pub envelope: Option<Envelope>,
    /// The answer without its signature (the text given when it carries none).
    pub unsigned: String,
}

impl SignatureReport {
    pub fn holds(&self) -> bool {
        self.present && self.cause.is_none()
    }
}

/// Check the signature of `text`, the object whose receipt is signed.
///
/// The checks run in this order, the first that fails named: the form
/// (`SignatureMalformed`), the algorithm, a key given, the key's id, the
/// receipt over the bytes, the ML-DSA-87 equation, the issue time, the
/// expiry, the nonce. An answer with no signature is `SignatureAbsent` when
/// a key was given, and is reported unchecked (`present` false, no cause)
/// when none was.
pub fn check(text: &str, given: Check<'_>) -> SignatureReport {
    let broken = |cause, envelope: Option<Envelope>, unsigned: String| SignatureReport {
        present: true,
        cause: Some(cause),
        envelope,
        unsigned,
    };
    let split = match split(text) {
        Err(cause) => return broken(cause, None, text.to_string()),
        Ok(None) => {
            return SignatureReport {
                present: false,
                cause: given.public_key.map(|_| SignatureCause::Absent),
                envelope: None,
                unsigned: text.to_string(),
            };
        }
        Ok(Some(split)) => split,
    };
    let Some(envelope) = split.envelope.clone() else {
        return broken(SignatureCause::Malformed, None, split.unsigned);
    };
    let unsigned = split.unsigned.clone();
    let signature = match BASE64.decode(&envelope.value) {
        Ok(bytes) if bytes.len() == SIGNATURE_LEN => bytes,
        _ => return broken(SignatureCause::Malformed, Some(envelope), unsigned),
    };
    if envelope.alg != ALGORITHM {
        return broken(SignatureCause::AlgorithmNotServed, Some(envelope), unsigned);
    }
    let Some(key) = given.public_key else {
        return broken(SignatureCause::KeyNotGiven, Some(envelope), unsigned);
    };
    if envelope.key_id != key.key_id() {
        return broken(SignatureCause::KeyMismatch, Some(envelope), unsigned);
    }
    match read_receipt(split.unsigned.as_bytes()) {
        Some(read) if read.matches() && read.stated == split.receipt => {}
        _ => {
            if !receipt_holds_inside(&split.unsigned, &split.receipt) {
                return broken(SignatureCause::ReceiptDoesNotMatch, Some(envelope), unsigned);
            }
        }
    }
    let signed = message(&envelope.key_id, &split.receipt, &envelope.issued_at, &envelope.expires_at, &envelope.nonce);
    if !key.verifies(signed.as_bytes(), &signature) {
        return broken(SignatureCause::DoesNotVerify, Some(envelope), unsigned);
    }
    let issued = parse_time(&envelope.issued_at).expect("read_envelope held the form");
    let expires = parse_time(&envelope.expires_at).expect("read_envelope held the form");
    if issued > given.now_s.saturating_add(CLOCK_ALLOWANCE_S) {
        return broken(SignatureCause::NotYetValid, Some(envelope), unsigned);
    }
    if given.now_s > expires {
        return broken(SignatureCause::Expired, Some(envelope), unsigned);
    }
    if given.seen_nonces.iter().any(|seen| seen == &envelope.nonce) {
        return broken(SignatureCause::NonceReplayed, Some(envelope), unsigned);
    }
    SignatureReport { present: true, cause: None, envelope: Some(envelope), unsigned }
}

/// Verify the signature of an answer a client wrote again: parsed and
/// written back in its own spacing or key order, so the signature no longer
/// stands right after the receipt in the engine's form and [`split`] finds
/// none. The caller reads the signature and the receipt it follows from the
/// answer's values and gives `unsigned`, the answer's text with the
/// signature's member taken out.
///
/// The checks run in this order, the first that fails named: the form, the
/// algorithm, a key given, the key's id, the ML-DSA-87 equation over the
/// receipt the answer carries, the issue time, the expiry, the nonce, and
/// last the receipt over the bytes (`ReceiptDoesNotMatch`). An answer that
/// fails only the last carries a signature the key made over its receipt,
/// and text that receipt does not cover: what changed is for the long form
/// to name, by computing the answer again. The report is always `present`.
pub fn check_rewritten(unsigned: &str, envelope: Option<Envelope>, receipt: &str, given: Check<'_>) -> SignatureReport {
    let broken = |cause, envelope: Option<Envelope>| SignatureReport {
        present: true,
        cause: Some(cause),
        envelope,
        unsigned: unsigned.to_string(),
    };
    let Some(envelope) = envelope else {
        return broken(SignatureCause::Malformed, None);
    };
    if !is_lower_hex(receipt, 96) {
        return broken(SignatureCause::Malformed, Some(envelope));
    }
    let signature = match BASE64.decode(&envelope.value) {
        Ok(bytes) if bytes.len() == SIGNATURE_LEN => bytes,
        _ => return broken(SignatureCause::Malformed, Some(envelope)),
    };
    if envelope.alg != ALGORITHM {
        return broken(SignatureCause::AlgorithmNotServed, Some(envelope));
    }
    let Some(key) = given.public_key else {
        return broken(SignatureCause::KeyNotGiven, Some(envelope));
    };
    if envelope.key_id != key.key_id() {
        return broken(SignatureCause::KeyMismatch, Some(envelope));
    }
    let signed = message(&envelope.key_id, receipt, &envelope.issued_at, &envelope.expires_at, &envelope.nonce);
    if !key.verifies(signed.as_bytes(), &signature) {
        return broken(SignatureCause::DoesNotVerify, Some(envelope));
    }
    let issued = parse_time(&envelope.issued_at).expect("read_envelope held the form");
    let expires = parse_time(&envelope.expires_at).expect("read_envelope held the form");
    if issued > given.now_s.saturating_add(CLOCK_ALLOWANCE_S) {
        return broken(SignatureCause::NotYetValid, Some(envelope));
    }
    if given.now_s > expires {
        return broken(SignatureCause::Expired, Some(envelope));
    }
    if given.seen_nonces.iter().any(|seen| seen == &envelope.nonce) {
        return broken(SignatureCause::NonceReplayed, Some(envelope));
    }
    let covered = match read_receipt(unsigned.as_bytes()) {
        Some(read) if read.matches() && read.stated == receipt => true,
        _ => receipt_holds_inside(unsigned, receipt),
    };
    if !covered {
        return broken(SignatureCause::ReceiptDoesNotMatch, Some(envelope));
    }
    SignatureReport { present: true, cause: None, envelope: Some(envelope), unsigned: unsigned.to_string() }
}

/// A receipt inside a larger text, as a dataset's head carries one inside an
/// inline answer: the receipt is of the object that ends right after it.
fn receipt_holds_inside(unsigned: &str, receipt: &str) -> bool {
    let tail = format!(",\"{RECEIPT_KEY}\":\"{receipt}\"}}");
    let Some(at) = unsigned.find(&tail) else { return false };
    let end = at + tail.len();
    // The receipted object starts at the brace that opens the object this
    // tail closes.
    let Some(start) = opening_brace(unsigned, at) else { return false };
    read_receipt(&unsigned.as_bytes()[start..end]).is_some_and(|read| read.matches())
}

/// The index of the `{` that opens the object enclosing byte `at`, read as
/// JSON from the start of `text`.
fn opening_brace(text: &str, at: usize) -> Option<usize> {
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in text.bytes().enumerate().take(at) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => stack.push((byte, index)),
            b'}' | b']' => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.iter().rev().find(|(byte, _)| *byte == b'{').map(|(_, index)| *index)
}

/// Seconds since 1970-01-01T00:00:00Z on this machine's clock.
pub fn now_s() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(elapsed) => i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX),
        Err(before) => -i64::try_from(before.duration().as_secs()).unwrap_or(i64::MAX),
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// `YYYY-MM-DDTHH:MM:SSZ` of a time in seconds since 1970-01-01T00:00:00Z.
pub fn format_time(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

/// The seconds of a time written `YYYY-MM-DDTHH:MM:SSZ`, years 1970 to 9999;
/// `None` for any other text, a day the month does not have included.
pub fn parse_time(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() != 20 {
        return None;
    }
    let digits = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = text.get(range)?;
        if !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        part.parse().ok()
    };
    for (at, byte) in [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':'), (19, b'Z')] {
        if bytes[at] != byte {
            return None;
        }
    }
    let (year, month, day) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
    let (hour, minute, second) = (digits(11..13)?, digits(14..16)?, digits(17..19)?);
    if year < 1970 || !(1..=12).contains(&month) || day < 1 || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    (format_time(seconds) == text).then_some(seconds)
}

pub use signing::*;

/// Making a signature. Signing with a given nonce is always here, for the
/// tests and the outside programs' fixtures; drawing a nonce and a key from
/// the operating system needs the feature `sign`, which the gateway enables.
mod signing {
    use super::*;
    use ml_dsa::{B32, Keypair as _, SigningKey as MlDsaSigningKey};

    /// Why an answer key was not read.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum KeyRefusal {
        /// The text is not 64 lowercase hex digits, a line break after them allowed.
        NotASeed,
    }

    /// Why an answer was not signed.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SignRefusal {
        /// The text carries no receipt that holds, or more than one.
        NoReceipt,
        /// The text already carries a signature.
        AlreadySigned,
        /// The time to live is outside [`LEAST_TTL_S`] to [`MOST_TTL_S`].
        TtlOutOfRange,
        /// The operating system gave no random bytes.
        NoRandomness,
    }

    /// The operator's answer key.
    pub struct SigningKey {
        key: MlDsaSigningKey<MlDsa87>,
        public: PublicKey,
    }

    impl std::fmt::Debug for SigningKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // Never the seed.
            write!(f, "SigningKey({})", self.public.key_id())
        }
    }

    impl SigningKey {
        /// The key FIPS 204's KeyGen_internal makes from the seed ξ.
        pub fn from_seed(seed: &[u8; SEED_LEN]) -> SigningKey {
            let xi: B32 = (*seed).into();
            let key = MlDsaSigningKey::<MlDsa87>::from_seed(&xi);
            let encoded = key.verifying_key().encode();
            let public = PublicKey::from_bytes(encoded.as_slice()).expect("a seed's key encodes");
            SigningKey { key, public }
        }

        /// The key from its seed written as 64 lowercase hex digits, one line
        /// break after them allowed: the key file's text.
        pub fn from_seed_hex(text: &str) -> Result<SigningKey, KeyRefusal> {
            let text = text.strip_suffix('\n').unwrap_or(text);
            if !is_lower_hex(text, 2 * SEED_LEN) {
                return Err(KeyRefusal::NotASeed);
            }
            let mut seed = [0_u8; SEED_LEN];
            hex::decode_to_slice(text, &mut seed).map_err(|_| KeyRefusal::NotASeed)?;
            let key = SigningKey::from_seed(&seed);
            seed.fill(0);
            Ok(key)
        }

        /// A new key from the operating system's randomness, and its seed as
        /// 64 hex digits for the key file.
        #[cfg(feature = "sign")]
        pub fn generate() -> Result<(SigningKey, String), SignRefusal> {
            let mut seed = random_bytes()?;
            let key = SigningKey::from_seed(&seed);
            let text = hex::encode(seed);
            seed.fill(0);
            Ok((key, text))
        }

        pub fn public_key(&self) -> &PublicKey {
            &self.public
        }

        /// The signature of a receipt at a time, for a nonce.
        pub fn envelope(&self, receipt: &str, issued_s: i64, ttl_s: u64, nonce: &[u8; NONCE_LEN]) -> Result<Envelope, SignRefusal> {
            if !(LEAST_TTL_S..=MOST_TTL_S).contains(&ttl_s) {
                return Err(SignRefusal::TtlOutOfRange);
            }
            let key_id = self.public.key_id();
            let issued_at = format_time(issued_s);
            let expires_at = format_time(issued_s.saturating_add(ttl_s as i64));
            let nonce = hex::encode(nonce);
            let signed = message(&key_id, receipt, &issued_at, &expires_at, &nonce);
            let signature = self
                .key
                .expanded_key()
                .sign_deterministic(signed.as_bytes(), b"")
                .expect("an empty context is in range");
            Ok(Envelope {
                alg: ALGORITHM.to_string(),
                key_id,
                issued_at,
                expires_at,
                nonce,
                value: BASE64.encode(signature.encode().as_slice()),
            })
        }

        /// `text`, an unsigned answer (or a dataset's head) ending with its
        /// receipt, with its signature written after the receipt.
        pub fn sign_text(&self, text: &str, issued_s: i64, ttl_s: u64, nonce: &[u8; NONCE_LEN]) -> Result<String, SignRefusal> {
            if split(text).map_or(true, |found| found.is_some()) {
                return Err(SignRefusal::AlreadySigned);
            }
            let marks = receipt_marks(text);
            let [start] = marks.as_slice() else { return Err(SignRefusal::NoReceipt) };
            let start = *start;
            let receipt = text.get(start..start + 96).ok_or(SignRefusal::NoReceipt)?;
            if !is_lower_hex(receipt, 96) || text.as_bytes().get(start + 96) != Some(&b'"') {
                return Err(SignRefusal::NoReceipt);
            }
            let holds = read_receipt(text.as_bytes()).is_some_and(|read| read.matches() && read.stated == receipt)
                || receipt_holds_inside(text, receipt);
            if !holds {
                return Err(SignRefusal::NoReceipt);
            }
            let envelope = self.envelope(receipt, issued_s, ttl_s, nonce)?;
            let at = start + 97;
            let mut signed = String::with_capacity(text.len() + 6400);
            signed.push_str(&text[..at]);
            signed.push_str(",\"");
            signed.push_str(SIGNATURE_KEY);
            signed.push_str("\":");
            signed.push_str(&envelope.json());
            signed.push_str(&text[at..]);
            Ok(signed)
        }

        /// [`SigningKey::sign_text`] at this machine's clock with a nonce from
        /// the operating system.
        #[cfg(feature = "sign")]
        pub fn sign_now(&self, text: &str, ttl_s: u64) -> Result<String, SignRefusal> {
            let nonce = random_bytes()?;
            self.sign_text(text, now_s(), ttl_s, &nonce)
        }
    }

    /// 32 bytes from the operating system's randomness.
    #[cfg(feature = "sign")]
    pub fn random_bytes() -> Result<[u8; 32], SignRefusal> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| SignRefusal::NoRandomness)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt::with_receipt;

    fn test_key(label: &str) -> SigningKey {
        let digest = Sha384::digest(format!("a test answer key, never an operator's: {label}").as_bytes());
        let mut seed = [0_u8; SEED_LEN];
        seed.copy_from_slice(&digest[..SEED_LEN]);
        SigningKey::from_seed(&seed)
    }

    const ISSUED: i64 = 1_792_357_445; // 2026-10-18T21:04:05Z

    fn signed_answer(key: &SigningKey) -> (String, String) {
        let unsigned = with_receipt(r#"{"inputs":{"dt_s":0.1},"run_seal":"ab","seal_rule":"zero-trust-physics/seal/v2"}"#);
        let signed = key.sign_text(&unsigned, ISSUED, 3600, &[7_u8; NONCE_LEN]).unwrap();
        (unsigned, signed)
    }

    fn given<'a>(key: &'a PublicKey, now_s: i64, seen: &'a [String]) -> Check<'a> {
        Check { public_key: Some(key), now_s, seen_nonces: seen }
    }

    #[test]
    fn times_are_written_and_read_back() {
        assert_eq!(format_time(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_time(ISSUED), "2026-10-18T21:04:05Z");
        assert_eq!(parse_time("2026-10-18T21:04:05Z"), Some(ISSUED));
        assert_eq!(parse_time("2028-02-29T00:00:00Z").map(format_time).as_deref(), Some("2028-02-29T00:00:00Z"));
        for bad in ["2026-02-30T00:00:00Z", "2026-10-18 21:04:05Z", "2026-10-18T21:04:05", "2026-10-18T24:00:00Z", "+026-10-18T21:04:05Z"] {
            assert_eq!(parse_time(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_signed_answer_holds_and_its_unsigned_bytes_are_the_answer() {
        let key = test_key("one");
        let (unsigned, signed) = signed_answer(&key);
        assert!(signed.starts_with(&unsigned[..unsigned.len() - 1]));
        assert!(signed.contains(r#"","signature":{"alg":"ML-DSA-87","key_id":""#));
        let report = check(&signed, given(key.public_key(), ISSUED + 10, &[]));
        assert_eq!(report.cause, None);
        assert!(report.holds());
        assert_eq!(report.unsigned, unsigned);
        let envelope = report.envelope.unwrap();
        assert_eq!(envelope.issued_at, "2026-10-18T21:04:05Z");
        assert_eq!(envelope.expires_at, "2026-10-18T22:04:05Z");
        assert_eq!(envelope.nonce, "07".repeat(32));
        assert_eq!(envelope.key_id.len(), 96);
    }

    #[test]
    fn each_attack_fails_by_its_name() {
        let key = test_key("one");
        let other = test_key("two");
        let (unsigned, signed) = signed_answer(&key);
        let public = key.public_key();
        let now = ISSUED + 10;
        let name = |text: &str, check_with: Check<'_>| check(text, check_with).cause.map(SignatureCause::name);

        // A forged signature: the value of another key's signature under this key's id.
        let (_, by_other) = signed_answer(&other);
        let other_value = split(&by_other).unwrap().unwrap().envelope.unwrap().value;
        let mine = split(&signed).unwrap().unwrap().envelope.unwrap();
        let forged = signed.replace(&mine.value, &other_value);
        assert_eq!(name(&forged, given(public, now, &[])), Some("SignatureDoesNotVerify"));
        // A time moved inside the signature.
        let moved = signed.replace("2026-10-18T22:04:05Z", "2026-10-19T22:04:05Z");
        assert_eq!(name(&moved, given(public, now, &[])), Some("SignatureDoesNotVerify"));
        // Expired.
        assert_eq!(name(&signed, given(public, ISSUED + 3601, &[])), Some("SignatureExpired"));
        assert_eq!(name(&signed, given(public, ISSUED + 3600, &[])), None);
        // Not yet valid.
        assert_eq!(name(&signed, given(public, ISSUED - 61, &[])), Some("SignatureNotYetValid"));
        assert_eq!(name(&signed, given(public, ISSUED - 60, &[])), None);
        // Replayed nonce.
        let seen = vec!["07".repeat(32)];
        assert_eq!(name(&signed, given(public, now, &seen)), Some("SignatureNonceReplayed"));
        // A value changed after signing, the receipt left as it was.
        let tampered = signed.replace("0.1", "0.2");
        assert_eq!(name(&tampered, given(public, now, &[])), Some("ReceiptDoesNotMatch"));
        // A value changed and the receipt made again: the signature is over the old receipt.
        let rewritten_unsigned = with_receipt(r#"{"inputs":{"dt_s":0.2},"run_seal":"ab","seal_rule":"zero-trust-physics/seal/v2"}"#);
        let at = rewritten_unsigned.len() - 1;
        let rewritten = format!("{},\"signature\":{}}}", &rewritten_unsigned[..at], mine.json());
        assert_eq!(name(&rewritten, given(public, now, &[])), Some("SignatureDoesNotVerify"));
        // The wrong key.
        assert_eq!(name(&signed, given(other.public_key(), now, &[])), Some("SignatureKeyMismatch"));
        // No key, no signature, malformed, another algorithm.
        assert_eq!(name(&signed, Check { public_key: None, now_s: now, seen_nonces: &[] }), Some("SignatureKeyNotGiven"));
        assert_eq!(name(&unsigned, given(public, now, &[])), Some("SignatureAbsent"));
        assert_eq!(name(&unsigned, Check { public_key: None, now_s: now, seen_nonces: &[] }), None);
        assert_eq!(name(&signed.replace("\"alg\":\"ML-DSA-87\"", "\"alg\":\"ML-DSA-65\""), given(public, now, &[])), Some("SignatureAlgorithmNotServed"));
        assert_eq!(name(&signed.replace("\"alg\":\"ML-DSA-87\",", ""), given(public, now, &[])), Some("SignatureMalformed"));
        assert_eq!(name(&signed.replace("T21:04:05Z", "T21:04:5Z"), given(public, now, &[])), Some("SignatureMalformed"));
    }

    #[test]
    fn a_head_inside_an_answer_is_signed_after_its_own_receipt() {
        let key = test_key("one");
        let head = with_receipt(r#"{"request":{"a":1},"dataset_seal":"cd","seal_rule":"zero-trust-physics/seal/v2"}"#);
        let answer = format!(r#"{{"head":{head},"rows":[{{"row":0}}],"about":"x"}}"#);
        let signed = key.sign_text(&answer, ISSUED, 3600, &[9_u8; NONCE_LEN]).unwrap();
        let report = check(&signed, given(key.public_key(), ISSUED, &[]));
        assert_eq!(report.cause, None);
        assert_eq!(report.unsigned, answer);
        let tampered = signed.replace("\"a\":1", "\"a\":2");
        assert_eq!(check(&tampered, given(key.public_key(), ISSUED, &[])).cause, Some(SignatureCause::ReceiptDoesNotMatch));
        // The rows sit outside the head's receipt: a row changed is not the signature's to name.
        let row_changed = signed.replace("\"row\":0", "\"row\":1");
        assert_eq!(check(&row_changed, given(key.public_key(), ISSUED, &[])).cause, None);
    }

    /// A signed answer a client wrote again: split finds no signature in the
    /// engine's form, and the signature read from the values is verified over
    /// the receipt it carries, the bytes named last.
    #[test]
    fn a_signed_answer_written_again_is_verified_over_its_receipt_and_its_bytes_named() {
        let key = test_key("one");
        let public = key.public_key();
        let now = ISSUED + 10;
        let (unsigned, signed) = signed_answer(&key);
        let envelope = split(&signed).unwrap().unwrap().envelope.unwrap();
        let receipt = split(&signed).unwrap().unwrap().receipt;
        // As a JSON client writes it back: a space after every colon and comma.
        let spaced = signed.replace("\":", "\": ").replace(",\"", ", \"");
        assert_eq!(split(&spaced), Ok(None), "not the engine's form");
        assert_eq!(check(&spaced, given(public, now, &[])).cause, Some(SignatureCause::Absent), "what split alone finds");
        let spaced_unsigned = unsigned.replace("\":", "\": ").replace(",\"", ", \"");
        let report = check_rewritten(&spaced_unsigned, Some(envelope.clone()), &receipt, given(public, now, &[]));
        assert!(report.present);
        assert_eq!(report.cause, Some(SignatureCause::ReceiptDoesNotMatch));
        // The same bytes the receipt covers: it holds.
        assert_eq!(check_rewritten(&unsigned, Some(envelope.clone()), &receipt, given(public, now, &[])).cause, None);
        // Each check before the bytes is named first.
        let seen = vec![envelope.nonce.clone()];
        let name = |envelope: Option<Envelope>, receipt: &str, check_with: Check<'_>| {
            check_rewritten(&spaced_unsigned, envelope, receipt, check_with).cause.map(SignatureCause::name)
        };
        assert_eq!(name(Some(envelope.clone()), &receipt, given(public, now, &seen)), Some("SignatureNonceReplayed"));
        assert_eq!(name(Some(envelope.clone()), &receipt, given(public, ISSUED + 3601, &[])), Some("SignatureExpired"));
        assert_eq!(name(Some(envelope.clone()), &"0".repeat(96), given(public, now, &[])), Some("SignatureDoesNotVerify"));
        assert_eq!(name(Some(envelope.clone()), &receipt, given(test_key("two").public_key(), now, &[])), Some("SignatureKeyMismatch"));
        assert_eq!(name(Some(envelope.clone()), &receipt, Check { public_key: None, now_s: now, seen_nonces: &[] }), Some("SignatureKeyNotGiven"));
        assert_eq!(name(None, &receipt, given(public, now, &[])), Some("SignatureMalformed"));
        // The envelope as a client writes it, read back compact.
        assert_eq!(read_envelope(&envelope.json()), Some(envelope.clone()));
        assert_eq!(read_envelope(&envelope.json().replace("\"alg\":\"ML-DSA-87\",", "")), None);
    }

    #[test]
    fn signing_refuses_what_it_cannot_sign() {
        let key = test_key("one");
        let (unsigned, signed) = signed_answer(&key);
        assert_eq!(key.sign_text(&signed, ISSUED, 3600, &[1; 32]), Err(SignRefusal::AlreadySigned));
        assert_eq!(key.sign_text(&unsigned.replace("0.1", "0.2"), ISSUED, 3600, &[1; 32]), Err(SignRefusal::NoReceipt));
        assert_eq!(key.sign_text(r#"{"a":1}"#, ISSUED, 3600, &[1; 32]), Err(SignRefusal::NoReceipt));
        assert_eq!(key.sign_text(&unsigned, ISSUED, 59, &[1; 32]), Err(SignRefusal::TtlOutOfRange));
        assert_eq!(SigningKey::from_seed_hex("AB").unwrap_err(), KeyRefusal::NotASeed);
        let seed = "ab".repeat(32);
        let a = SigningKey::from_seed_hex(&seed).unwrap();
        let b = SigningKey::from_seed_hex(&format!("{seed}\n")).unwrap();
        assert_eq!(a.public_key().key_id(), b.public_key().key_id());
        assert!(!format!("{a:?}").contains(&seed));
        let round = PublicKey::from_base64(&a.public_key().to_base64()).unwrap();
        assert_eq!(round.bytes(), a.public_key().bytes());
    }
}
