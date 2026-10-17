//! A signed load: a reading an instrument signed, verified with no engine.
//!
//! A segment of a run may carry `signed` beside its `load`; a run's
//! `initial`, a solve's `parameters` and one measurement of a comparison may
//! carry `signed` among their own fields. `signed` holds the instrument's
//! Ed25519 public key, the name it gives itself, its time, its reading
//! counter, the hash of its previous reading, a challenge the relying party
//! chose or null, and a 64-byte signature. The instrument signs, and every
//! verifier rebuilds, these bytes:
//!
//! ```text
//! zero-trust-physics/signed-reading/v1\n  ||  sealed_form(R)
//! ```
//!
//! where `R` is the object of the seven fields of `signed` other than `sig`,
//! with the reading under the key of its place: `load` for a segment,
//! `initial`, `parameters` and `measurement` for the others, each the object
//! as the request carries it without its own `signed`. The sealed form is
//! the one the run seal is taken over (`sealed_form`), so a signature covers
//! the exact 64-bit values the model steps with.
//!
//! Because `link_0` hashes the sealed form of the whole request, a signed
//! reading is inside the run seal with no change to any seal rule: a request
//! without `signed` has the bytes it always had.
//!
//! The rule of verification is RFC 8032, section 5.1.7, pure Ed25519, with
//! the strict criteria of `ed25519-dalek`'s `verify_strict`: the key and `R`
//! are each the canonical encoding of a point of the curve and neither is of
//! small order, `S` is below the group order, and the equation is
//! `[S]B = R + [k]A`. The checks are made in that order (key, `R`, `S`, the
//! equation), so every verifier that follows this rule names the same reason.
//! The arithmetic is `ed25519-dalek`'s and `curve25519-dalek`'s: nothing of
//! the curve is written here.
//!
//! A signature shows that the holder of a key stated a reading. Which keys
//! to trust is the verifier's list ([`Trust`]), never the engine's.

use std::fmt;

use serde_json::{Map, Value};

use crate::json_text::is_lower_hex;
use crate::sealed_form::{digest_hex, sealed_form};

/// The first line of every signed text: what the signature is for, and the
/// version of the rule.
pub const DOMAIN_LINE: &str = "zero-trust-physics/signed-reading/v1\n";
/// The key of a signed reading in a segment, an `initial`, a `parameters`
/// object or a measurement.
pub const SIGNED: &str = "signed";
/// The one algorithm of version 1.
pub const ALG_ED25519: &str = "ed25519";
/// The fields of `signed`, each required, in the order they are read.
pub const FIELDS: [&str; 8] = ["alg", "key", "instrument", "time", "seq", "prev", "challenge", "sig"];
/// The most characters of the name an instrument gives itself.
pub const MOST_INSTRUMENT_CHARS: usize = 200;
/// A reading counter is a whole number below 2^53, so the sealed form (a
/// 64-bit float) holds it exactly.
pub const SEQ_BELOW: u64 = 1 << 53;
/// The most hex digits of a challenge: 64 bytes.
pub const MOST_CHALLENGE_DIGITS: usize = 128;

/// The refusal names of a signed reading.
pub const MALFORMED: &str = "LoadSignatureMalformed";
pub const DOES_NOT_VERIFY: &str = "LoadSignatureDoesNotVerify";
pub const ALGORITHM_NOT_SERVED: &str = "LoadSignatureAlgorithmNotServed";
pub const REPLACED: &str = "SignedLoadReplaced";
/// A required field left out, as every request names it.
pub const INPUT_MISSING: &str = "InputMissing";

/// What `load_source` says of a run none of whose loads is signed.
pub const CALLERS_NUMBER: &str = "The caller's number: no load of this run carries an instrument's signature. The seal covers the arithmetic on these loads, not a claim that an instrument measured them.";
/// What `measurement_source` says of a comparison none of whose
/// measurements is signed.
pub const CALLERS_MEASUREMENTS: &str = "The caller's numbers: no measurement carries an instrument's signature. The seal covers the comparison with these numbers, not a claim that an instrument measured them.";
/// The last sentence of every label with a signed reading in it.
pub const WHO_SIGNED: &str = "A signature shows that the holder of that key stated these readings; whether the key is a real instrument's is the verifier's list.";

/// Where a signed reading sits in a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    /// `initial.signed`, over the rest of `initial`.
    Initial,
    /// `segments[i].signed`, over `segments[i].load`.
    Segment(usize),
    /// `parameters.signed`, over the rest of a solve's `parameters`.
    Parameters,
    /// `measurements[i].signed`, over the rest of that measurement.
    Measurement(usize),
}

impl Place {
    /// The place as a path of the request: `initial`, `segments[3]`,
    /// `parameters`, `measurements[3]`.
    pub fn label(self) -> String {
        match self {
            Place::Initial => "initial".to_string(),
            Place::Segment(index) => format!("segments[{index}]"),
            Place::Parameters => "parameters".to_string(),
            Place::Measurement(index) => format!("measurements[{index}]"),
        }
    }

    /// The key the reading is signed under in the signed text. The key is
    /// inside the signature, so a genuine reading moved to another place
    /// does not verify there.
    pub fn key(self) -> &'static str {
        match self {
            Place::Initial => "initial",
            Place::Segment(_) => "load",
            Place::Parameters => "parameters",
            Place::Measurement(_) => "measurement",
        }
    }

    /// The path of one field of the place's `signed`.
    pub fn field(self, field: &str) -> String {
        format!("{}.{SIGNED}.{field}", self.label())
    }
}

impl fmt::Display for Place {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(&self.label())
    }
}

/// Why a signature does not verify under the strict rule, in the order the
/// rule checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strict {
    /// The key is not the canonical encoding of a point of the curve.
    KeyNotCanonicalPoint,
    /// The key is a point of small order: it would verify almost any text.
    KeySmallOrder,
    /// `R` is not the canonical encoding of a point of the curve.
    RNotCanonicalPoint,
    /// `R` is a point of small order.
    RSmallOrder,
    /// `S` is not below the group order: a malleated form of a signature.
    SNotBelowOrder,
    /// `[S]B` is not `R + [k]A` for this text.
    Equation,
}

impl Strict {
    /// The word the vectors file and the programs use.
    pub fn as_str(self) -> &'static str {
        match self {
            Strict::KeyNotCanonicalPoint => "key_not_canonical_point",
            Strict::KeySmallOrder => "key_small_order",
            Strict::RNotCanonicalPoint => "r_not_canonical_point",
            Strict::RSmallOrder => "r_small_order",
            Strict::SNotBelowOrder => "s_not_below_order",
            Strict::Equation => "equation",
        }
    }

    /// The reason in words.
    pub fn says(self) -> &'static str {
        match self {
            Strict::KeyNotCanonicalPoint => "the key is not the canonical encoding of a point of the curve",
            Strict::KeySmallOrder => "the key is a point of small order, which would verify almost any text",
            Strict::RNotCanonicalPoint => "the signature's R is not the canonical encoding of a point of the curve",
            Strict::RSmallOrder => "the signature's R is a point of small order",
            Strict::SNotBelowOrder => "the signature's S is not below the group order, so it is a malleated form of a signature (RFC 8032, section 5.1.7)",
            Strict::Equation => "the signature is not one this key made over the signed text of this reading",
        }
    }
}

/// Verify an Ed25519 signature by the strict rule: the key, then `R`, then
/// `S`, then the equation. Every check is the libraries'.
pub fn verify_strict(key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> Result<(), Strict> {
    use curve25519_dalek::edwards::CompressedEdwardsY;
    use curve25519_dalek::scalar::Scalar;

    fn point(bytes: [u8; 32], not_point: Strict, small: Strict) -> Result<(), Strict> {
        let decoded = CompressedEdwardsY(bytes).decompress().ok_or(not_point)?;
        // A point read from an encoding that is not the one it writes, such
        // as y at or above p, or a zero x with its sign bit set, is refused.
        if decoded.compress().to_bytes() != bytes {
            return Err(not_point);
        }
        if decoded.is_small_order() {
            return Err(small);
        }
        Ok(())
    }

    point(*key, Strict::KeyNotCanonicalPoint, Strict::KeySmallOrder)?;
    let mut r = [0_u8; 32];
    r.copy_from_slice(&signature[..32]);
    point(r, Strict::RNotCanonicalPoint, Strict::RSmallOrder)?;
    let mut s = [0_u8; 32];
    s.copy_from_slice(&signature[32..]);
    if Option::<Scalar>::from(Scalar::from_canonical_bytes(s)).is_none() {
        return Err(Strict::SNotBelowOrder);
    }
    let verifying = ed25519_dalek::VerifyingKey::from_bytes(key).map_err(|_| Strict::KeyNotCanonicalPoint)?;
    verifying
        .verify_strict(message, &ed25519_dalek::Signature::from_bytes(signature))
        .map_err(|_| Strict::Equation)
}

/// Why a signed reading was not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestFault {
    /// A field of `signed` is left out: nothing is filled in.
    Missing { place: Place, field: &'static str },
    /// `signed` is not an object, a field is not one of a signed reading, or
    /// a field is of the wrong type, length or form.
    Malformed { place: Place, field: Option<String>, says: String },
    /// `alg` names an algorithm this version does not verify.
    AlgorithmNotServed { place: Place, alg: String },
    /// The signature does not verify by the strict rule.
    DoesNotVerify { place: Place, fingerprint: String, why: Strict },
}

impl AttestFault {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            AttestFault::Missing { .. } => INPUT_MISSING,
            AttestFault::Malformed { .. } => MALFORMED,
            AttestFault::AlgorithmNotServed { .. } => ALGORITHM_NOT_SERVED,
            AttestFault::DoesNotVerify { .. } => DOES_NOT_VERIFY,
        }
    }

    /// Where the fault is.
    pub fn place(&self) -> Place {
        match self {
            AttestFault::Missing { place, .. }
            | AttestFault::Malformed { place, .. }
            | AttestFault::AlgorithmNotServed { place, .. }
            | AttestFault::DoesNotVerify { place, .. } => *place,
        }
    }
}

impl fmt::Display for AttestFault {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AttestFault::Missing { place, field } => {
                write!(out, "Missing required parameter: '{}' ({INPUT_MISSING}).", place.field(field))
            }
            AttestFault::Malformed { place, field: Some(field), says } => {
                write!(out, "Physical Violation: '{}' {says} ({MALFORMED}).", place.field(field))
            }
            AttestFault::Malformed { place, field: None, says } => {
                write!(out, "Physical Violation: '{}.{SIGNED}' {says} ({MALFORMED}).", place.label())
            }
            AttestFault::AlgorithmNotServed { place, alg } => write!(
                out,
                "Physical Violation: '{}' is {}, and version 1 of a signed reading verifies only \"{ALG_ED25519}\" ({ALGORITHM_NOT_SERVED}).",
                place.field("alg"),
                Value::String(alg.clone())
            ),
            AttestFault::DoesNotVerify { place, fingerprint, why } => write!(
                out,
                "Physical Violation: the signature at '{}.{SIGNED}' does not verify under the key {fingerprint}: {}. Nothing is computed and nothing is sealed ({DOES_NOT_VERIFY}).",
                place.label(),
                why.says()
            ),
        }
    }
}

/// A signed reading that verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub place: Place,
    /// The public key, 64 lowercase hex digits: the instrument's identity.
    pub key: String,
    /// The first 16 hex digits of SHA-256 of the key's 32 bytes.
    pub fingerprint: String,
    /// The name the instrument gives itself: a label, never the identity.
    pub instrument: String,
    pub time: String,
    pub seq: u64,
    pub prev: Option<String>,
    pub challenge: Option<String>,
    /// SHA-256 of the signed text, as hex: what the next reading's `prev` names.
    pub reading_sha256: String,
    /// The field names of the reading, sorted: what the instrument measured.
    pub quantities: Vec<String>,
}

/// The first 16 hex digits of SHA-256 of a key's 32 bytes.
pub fn fingerprint(key: &[u8; 32]) -> String {
    digest_hex(key)[..16].to_string()
}

/// The text an instrument signs for `reading` at `place`, with the fields of
/// `signed` other than `sig`. The fields are taken as `signed` holds them;
/// [`verify`] reads them first.
pub fn signed_text(place: Place, signed: &Map<String, Value>, reading: &Value) -> Vec<u8> {
    let mut object = Map::new();
    for field in FIELDS {
        if field == "sig" {
            continue;
        }
        if let Some(value) = signed.get(field) {
            object.insert(field.to_string(), value.clone());
        }
    }
    object.insert(place.key().to_string(), reading.clone());
    let mut text = DOMAIN_LINE.as_bytes().to_vec();
    text.extend_from_slice(sealed_form(&Value::Object(object)).as_bytes());
    text
}

/// The reading `signed` speaks for at its place: the load of a segment, or
/// the object the signed object sits in, without it.
pub fn reading_of(place: Place, holder: &Value) -> Value {
    match place {
        Place::Segment(_) => holder.get("load").cloned().unwrap_or(Value::Null),
        _ => match holder.as_object() {
            Some(object) => {
                let mut without = object.clone();
                without.shift_remove(SIGNED);
                Value::Object(without)
            }
            None => Value::Null,
        },
    }
}

fn malformed(place: Place, field: &str, says: impl Into<String>) -> AttestFault {
    AttestFault::Malformed { place, field: Some(field.to_string()), says: says.into() }
}

fn hex_field(place: Place, signed: &Map<String, Value>, field: &'static str, digits: usize) -> Result<Vec<u8>, AttestFault> {
    match signed.get(field) {
        None => Err(AttestFault::Missing { place, field }),
        Some(Value::String(text)) if is_lower_hex(text, digits) => {
            Ok(hex::decode(text).expect("lowercase hex decodes"))
        }
        Some(_) => Err(malformed(place, field, format!("must be {digits} lowercase hex digits"))),
    }
}

fn optional_hex(place: Place, signed: &Map<String, Value>, field: &'static str) -> Result<Option<String>, AttestFault> {
    match signed.get(field) {
        None => Err(AttestFault::Missing { place, field }),
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(malformed(place, field, "must be a string of lowercase hex digits, or null")),
    }
}

/// Read the fields of `signed` at `place`, verify its signature over
/// `reading`, and return what it says. Every field is required; none is
/// filled in. `prev` and `challenge` may be null.
pub fn verify(place: Place, signed: &Value, reading: &Value) -> Result<Verified, AttestFault> {
    let signed = signed.as_object().ok_or_else(|| AttestFault::Malformed {
        place,
        field: None,
        says: "must be a JSON object".to_string(),
    })?;
    for key in signed.keys() {
        if !FIELDS.contains(&key.as_str()) {
            return Err(AttestFault::Malformed {
                place,
                field: None,
                says: format!(
                    "holds '{key}', which is not a field of a signed reading; it holds alg, key, instrument, time, seq, prev, challenge and sig, and no other"
                ),
            });
        }
    }
    match signed.get("alg") {
        None => return Err(AttestFault::Missing { place, field: "alg" }),
        Some(Value::String(alg)) if alg == ALG_ED25519 => {}
        Some(Value::String(alg)) => {
            return Err(AttestFault::AlgorithmNotServed { place, alg: alg.clone() });
        }
        Some(_) => return Err(malformed(place, "alg", "must be the string \"ed25519\"")),
    }
    let key_bytes = hex_field(place, signed, "key", 64)?;
    let instrument = match signed.get("instrument") {
        None => return Err(AttestFault::Missing { place, field: "instrument" }),
        Some(Value::String(name)) => {
            let count = name.chars().count();
            if count == 0 || count > MOST_INSTRUMENT_CHARS {
                return Err(malformed(place, "instrument", format!("must be a name of 1 to {MOST_INSTRUMENT_CHARS} characters")));
            }
            if name.chars().any(char::is_control) {
                return Err(malformed(place, "instrument", "must hold no control character, such as a line break"));
            }
            name.clone()
        }
        Some(_) => return Err(malformed(place, "instrument", "must be a string")),
    };
    let time = match signed.get("time") {
        None => return Err(AttestFault::Missing { place, field: "time" }),
        Some(Value::String(text)) if Instant::read(text).is_some() => text.clone(),
        Some(_) => {
            return Err(malformed(
                place,
                "time",
                "must be a time in RFC 3339 in UTC, as a string ending in Z, such as \"2026-10-21T14:02:00.000Z\"",
            ));
        }
    };
    let seq = match signed.get("seq") {
        None => return Err(AttestFault::Missing { place, field: "seq" }),
        Some(value) => match value.as_u64() {
            Some(seq) if seq < SEQ_BELOW => seq,
            _ => return Err(malformed(place, "seq", "must be a whole number from 0 to 9007199254740991, below 2^53")),
        },
    };
    let prev = optional_hex(place, signed, "prev")?;
    if let Some(text) = &prev {
        if !is_lower_hex(text, 64) {
            return Err(malformed(place, "prev", "must be 64 lowercase hex digits, the SHA-256 of the previous reading's signed text, or null"));
        }
    }
    let challenge = optional_hex(place, signed, "challenge")?;
    if let Some(text) = &challenge {
        let digits = text.len();
        let hex = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if !hex || digits % 2 != 0 || digits > MOST_CHALLENGE_DIGITS {
            return Err(malformed(
                place,
                "challenge",
                format!("must be lowercase hex, an even count of 2 to {MOST_CHALLENGE_DIGITS} digits, or null"),
            ));
        }
    }
    let sig_bytes = hex_field(place, signed, "sig", 128)?;

    let mut key = [0_u8; 32];
    key.copy_from_slice(&key_bytes);
    let mut sig = [0_u8; 64];
    sig.copy_from_slice(&sig_bytes);
    let text = signed_text(place, signed, reading);
    let print = fingerprint(&key);
    verify_strict(&key, &text, &sig).map_err(|why| AttestFault::DoesNotVerify { place, fingerprint: print.clone(), why })?;
    let mut quantities: Vec<String> = reading.as_object().map(|object| object.keys().cloned().collect()).unwrap_or_default();
    quantities.sort();
    Ok(Verified {
        place,
        key: hex::encode(key),
        fingerprint: print,
        instrument,
        time,
        seq,
        prev,
        challenge,
        reading_sha256: digest_hex(&text),
        quantities,
    })
}

/// Each signed reading of a run request, in place order: `initial`, then
/// each segment. A part that is not there, or is not an object, holds none.
pub fn run_places(request: &Value) -> Vec<(Place, &Value, Value)> {
    let mut found = Vec::new();
    if let Some(initial) = request.get("initial").filter(|value| value.is_object()) {
        if let Some(signed) = initial.get(SIGNED) {
            found.push((Place::Initial, signed, reading_of(Place::Initial, initial)));
        }
    }
    if let Some(segments) = request.get("segments").and_then(Value::as_array) {
        for (index, segment) in segments.iter().enumerate() {
            if let Some(signed) = segment.as_object().and_then(|object| object.get(SIGNED)) {
                found.push((Place::Segment(index), signed, reading_of(Place::Segment(index), segment)));
            }
        }
    }
    found
}

/// The signed reading of a solve request's `parameters`, when it has one.
pub fn solve_places(request: &Value) -> Vec<(Place, &Value, Value)> {
    let mut found = Vec::new();
    if let Some(parameters) = request.get("parameters").filter(|value| value.is_object()) {
        if let Some(signed) = parameters.get(SIGNED) {
            found.push((Place::Parameters, signed, reading_of(Place::Parameters, parameters)));
        }
    }
    found
}

/// Each signed measurement of a comparison's `measurements`, in order.
pub fn measurement_places(measurements: &Value) -> Vec<(Place, &Value, Value)> {
    let mut found = Vec::new();
    for (index, item) in measurements.as_array().into_iter().flatten().enumerate() {
        if let Some(signed) = item.as_object().and_then(|object| object.get(SIGNED)) {
            found.push((Place::Measurement(index), signed, reading_of(Place::Measurement(index), item)));
        }
    }
    found
}

/// Verify every reading found, in order, and stop at the first that does
/// not verify.
pub fn verify_all(places: &[(Place, &Value, Value)]) -> Result<Vec<Verified>, AttestFault> {
    places.iter().map(|(place, signed, reading)| verify(*place, signed, reading)).collect()
}

/// Every signed reading of a run request, verified.
pub fn verify_run(request: &Value) -> Result<Vec<Verified>, AttestFault> {
    verify_all(&run_places(request))
}

/// Whether the signed readings form unbroken streams: for each key, every
/// reading after its first names the hash of the one before it by the same
/// key in `prev`, and its `seq` is one more. A break is a fact reported, not
/// a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stream {
    Unbroken,
    Breaks { at: Place, says: String },
}

impl Stream {
    /// `unbroken`, or the sentence of the first break.
    pub fn text(&self) -> String {
        match self {
            Stream::Unbroken => "unbroken".to_string(),
            Stream::Breaks { says, .. } => says.clone(),
        }
    }
}

/// The streams of `readings`, taken in place order.
pub fn stream(readings: &[Verified]) -> Stream {
    let mut last: Vec<(&str, &Verified)> = Vec::new();
    for reading in readings {
        match last.iter_mut().find(|(key, _)| *key == reading.key.as_str()) {
            None => last.push((reading.key.as_str(), reading)),
            Some(entry) => {
                let before = entry.1;
                if reading.prev.as_deref() != Some(before.reading_sha256.as_str()) {
                    return Stream::Breaks {
                        at: reading.place,
                        says: format!(
                            "breaks at {}: its prev is not the SHA-256 of the signed text at {}, the reading before it by the key {}",
                            reading.place, before.place, reading.fingerprint
                        ),
                    };
                }
                if reading.seq != before.seq + 1 {
                    return Stream::Breaks {
                        at: reading.place,
                        says: format!(
                            "breaks at {}: its seq is {}, and the reading before it by the key {}, at {}, has seq {}",
                            reading.place, reading.seq, reading.fingerprint, before.place, before.seq
                        ),
                    };
                }
                entry.1 = reading;
            }
        }
    }
    Stream::Unbroken
}

/// The sentence a label's `says` carries when its signed readings are not one
/// unbroken stream, after the sentence of what verified: where the stream
/// breaks, and what such a break is. Empty for an unbroken stream.
fn stream_words(stream: &Stream, readings: &str) -> String {
    match stream {
        Stream::Unbroken => String::new(),
        Stream::Breaks { says, .. } => format!(
            " The signed {readings} are not one unbroken stream: the stream {says}. So a reading of that key is missing there, or the readings were reordered, or spliced from another stream."
        ),
    }
}

/// Places written short: runs of segments or measurements as "a to b".
pub fn places_text(places: &[Place]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut index = 0;
    while index < places.len() {
        let start = places[index];
        let mut end = index;
        while end + 1 < places.len() && next_of(places[end]) == Some(places[end + 1]) {
            end += 1;
        }
        if end > index {
            parts.push(format!("{} to {}", start, places[end]));
        } else {
            parts.push(start.label());
        }
        index = end + 1;
    }
    parts.join(", ")
}

fn next_of(place: Place) -> Option<Place> {
    match place {
        Place::Segment(index) => Some(Place::Segment(index + 1)),
        Place::Measurement(index) => Some(Place::Measurement(index + 1)),
        _ => None,
    }
}

fn span(first: &str, last: &str) -> String {
    if first == last { first.to_string() } else { format!("{first} to {last}") }
}

/// One entry of a label's `keys`: what one key signed.
fn key_entries(readings: &[Verified]) -> Vec<Value> {
    let mut order: Vec<&str> = Vec::new();
    for reading in readings {
        if !order.contains(&reading.key.as_str()) {
            order.push(reading.key.as_str());
        }
    }
    order
        .into_iter()
        .map(|key| {
            let mine: Vec<&Verified> = readings.iter().filter(|reading| reading.key == key).collect();
            let first = mine[0];
            let last = mine[mine.len() - 1];
            let mut challenges: Vec<Value> = Vec::new();
            let mut names: Vec<Value> = Vec::new();
            for reading in &mine {
                let challenge = reading.challenge.clone().map(Value::String).unwrap_or(Value::Null);
                if !challenges.contains(&challenge) {
                    challenges.push(challenge);
                }
                let name = Value::String(reading.instrument.clone());
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            let places: Vec<Place> = mine.iter().map(|reading| reading.place).collect();
            let mut entry = Map::new();
            entry.insert("key".into(), Value::String(key.to_string()));
            entry.insert("fingerprint".into(), Value::String(first.fingerprint.clone()));
            // The name is the instrument's statement about itself; a key
            // that gives two names is shown with both.
            entry.insert(
                "names_itself".into(),
                if names.len() == 1 { names.remove(0) } else { Value::Array(names) },
            );
            entry.insert("readings".into(), Value::from(mine.len() as u64));
            entry.insert("places".into(), Value::String(places_text(&places)));
            entry.insert("time".into(), Value::String(span(&first.time, &last.time)));
            entry.insert("seq".into(), Value::String(span(&first.seq.to_string(), &last.seq.to_string())));
            entry.insert("challenges".into(), Value::Array(challenges));
            Value::Object(entry)
        })
        .collect()
}

fn key_word(readings: &[Verified]) -> &'static str {
    let first = &readings[0].key;
    if readings.iter().all(|reading| &reading.key == first) { "the key shown" } else { "the keys shown" }
}

/// `load_source` of a run: segment by segment, whether each load carries a
/// signature that verified, and whose. `segments` is the count of the run's
/// segments; `readings` its verified readings in place order.
pub fn load_source(segments: usize, readings: &[Verified]) -> Value {
    let mut label = Map::new();
    let signed_segments: Vec<Place> =
        readings.iter().map(|reading| reading.place).filter(|place| matches!(place, Place::Segment(_))).collect();
    label.insert("segments".into(), Value::from(segments as u64));
    label.insert("signed".into(), Value::from(signed_segments.len() as u64));
    if readings.is_empty() {
        label.insert("says".into(), Value::String(CALLERS_NUMBER.to_string()));
        return Value::Object(label);
    }
    let initial_signed = readings.iter().any(|reading| reading.place == Place::Initial);
    label.insert(
        "initial".into(),
        Value::String(if initial_signed { "signed" } else { "the caller's number" }.to_string()),
    );
    let stream = stream(readings);
    label.insert("stream".into(), Value::String(stream.text()));
    label.insert("keys".into(), Value::Array(key_entries(readings)));
    let keys = key_word(readings);
    let unsigned: Vec<Place> =
        (0..segments).map(Place::Segment).filter(|place| !signed_segments.contains(place)).collect();
    let initial_words = if initial_signed {
        ", and so does its initial state"
    } else {
        "; its initial state is the caller's number"
    };
    let opening = if unsigned.is_empty() {
        format!("Every load of this run carries a signature that verifies under {keys}{initial_words}.")
    } else if signed_segments.is_empty() {
        format!(
            "The initial state of this run carries a signature that verifies under {keys}; no load carries one, so every load is the caller's number."
        )
    } else {
        format!(
            "{} of the {segments} loads of this run carry a signature that verifies under {keys}{initial_words}; the loads at {} are the caller's number.",
            signed_segments.len(),
            places_text(&unsigned)
        )
    };
    let broken = stream_words(&stream, "readings");
    label.insert("says".into(), Value::String(format!("{opening}{broken} {WHO_SIGNED}")));
    Value::Object(label)
}

/// `load_source` of a solve whose `parameters` carry a signature that
/// verified.
pub fn parameters_source(reading: &Verified) -> Value {
    let readings = std::slice::from_ref(reading);
    let mut label = Map::new();
    label.insert("parameters".into(), Value::String("signed".to_string()));
    label.insert("keys".into(), Value::Array(key_entries(readings)));
    label.insert(
        "says".into(),
        Value::String(format!("The parameters of this solve carry a signature that verifies under the key shown. {WHO_SIGNED}")),
    );
    Value::Object(label)
}

/// `measurement_source` of a comparison: whether each measurement carries a
/// signature that verified, and whose.
pub fn measurement_source(measurements: usize, readings: &[Verified]) -> Value {
    let mut label = Map::new();
    label.insert("measurements".into(), Value::from(measurements as u64));
    label.insert("signed".into(), Value::from(readings.len() as u64));
    if readings.is_empty() {
        label.insert("says".into(), Value::String(CALLERS_MEASUREMENTS.to_string()));
        return Value::Object(label);
    }
    let stream = stream(readings);
    label.insert("stream".into(), Value::String(stream.text()));
    label.insert("keys".into(), Value::Array(key_entries(readings)));
    let keys = key_word(readings);
    let signed: Vec<Place> = readings.iter().map(|reading| reading.place).collect();
    let unsigned: Vec<Place> =
        (0..measurements).map(Place::Measurement).filter(|place| !signed.contains(place)).collect();
    let opening = if unsigned.is_empty() {
        format!("Every measurement compared carries a signature that verifies under {keys}.")
    } else {
        format!(
            "{} of the {measurements} measurements carry a signature that verifies under {keys}; the measurements at {} are the caller's numbers.",
            readings.len(),
            places_text(&unsigned)
        )
    };
    let broken = stream_words(&stream, "measurements");
    label.insert("says".into(), Value::String(format!("{opening}{broken} {WHO_SIGNED}")));
    Value::Object(label)
}

/// A time as a signed reading states it: RFC 3339 in UTC, ending in `Z`,
/// with up to nine digits of a second. It orders as time does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nanos: u32,
}

impl Instant {
    /// `YYYY-MM-DDTHH:MM:SS[.f]Z`, every part in range (a second of 60 is a
    /// leap second), or nothing.
    pub fn read(text: &str) -> Option<Instant> {
        let bytes = text.as_bytes();
        if bytes.len() < 20 || bytes.len() > 30 || *bytes.last()? != b'Z' {
            return None;
        }
        let digits = |from: usize, count: usize| -> Option<u32> {
            let part = bytes.get(from..from + count)?;
            if !part.iter().all(u8::is_ascii_digit) {
                return None;
            }
            std::str::from_utf8(part).ok()?.parse().ok()
        };
        let separators = [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')];
        if separators.iter().any(|(at, byte)| bytes.get(*at) != Some(byte)) {
            return None;
        }
        let (year, month, day) = (digits(0, 4)?, digits(5, 2)?, digits(8, 2)?);
        let (hour, minute, second) = (digits(11, 2)?, digits(14, 2)?, digits(17, 2)?);
        let nanos = match &bytes[19..bytes.len() - 1] {
            [] => 0,
            [b'.', fraction @ ..] if (1..=9).contains(&fraction.len()) && fraction.iter().all(u8::is_ascii_digit) => {
                let mut value: u32 = std::str::from_utf8(fraction).ok()?.parse().ok()?;
                for _ in fraction.len()..9 {
                    value *= 10;
                }
                value
            }
            _ => return None,
        };
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => return None,
        };
        if day == 0 || day > days || hour > 23 || minute > 59 || second > 60 {
            return None;
        }
        Some(Instant { year, month, day, hour, minute, second, nanos })
    }
}

/// The verifier's own list of the keys it trusts: each key with the name the
/// verifier gives it, the quantities it may sign, and the span it is valid
/// for. The engine never holds one.
///
/// The file is JSON: `{"keys": [{"key": <64 hex>, "name": <text>,
/// "quantities": [<field name>, ...], "valid_from": <time>, "valid_to":
/// <time>}]}`. `key` and `name` are required; `quantities`, `valid_from` and
/// `valid_to` may be left out, and then do not bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trust {
    entries: Vec<TrustEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrustEntry {
    key: String,
    name: String,
    quantities: Option<Vec<String>>,
    valid_from: Option<(Instant, String)>,
    valid_to: Option<(Instant, String)>,
}

/// What the verifier's list says of one reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trusted {
    /// The key is on the list, under this name, and the reading is inside
    /// what the list binds it to.
    Yes(String),
    /// Why the list does not trust this reading.
    No(String),
}

impl Trusted {
    pub fn text(&self) -> String {
        match self {
            Trusted::Yes(name) => format!("yes, {}", Value::String(name.clone())),
            Trusted::No(why) => format!("no: {why}"),
        }
    }
}

impl Trust {
    /// Read a trust list from the text of its file.
    pub fn read(bytes: &[u8]) -> Result<Trust, String> {
        let value = crate::json_text::read(bytes).map_err(|refusal| refusal.to_string())?;
        let object = value.as_object().ok_or("the trust list must be a JSON object with 'keys'")?;
        for key in object.keys() {
            if key != "keys" {
                return Err(format!("the trust list holds '{key}'; it holds 'keys' and no other"));
            }
        }
        let items = object
            .get("keys")
            .and_then(Value::as_array)
            .ok_or("the trust list must hold 'keys', an array")?;
        let mut entries = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            let at = format!("keys[{index}]");
            let item = item.as_object().ok_or(format!("'{at}' must be a JSON object"))?;
            for key in item.keys() {
                if !["key", "name", "quantities", "valid_from", "valid_to"].contains(&key.as_str()) {
                    return Err(format!(
                        "'{at}' holds '{key}'; an entry holds key, name, quantities, valid_from and valid_to"
                    ));
                }
            }
            let key = match item.get("key") {
                Some(Value::String(text)) if is_lower_hex(text, 64) => text.clone(),
                _ => return Err(format!("'{at}.key' must be 64 lowercase hex digits")),
            };
            if entries.iter().any(|entry: &TrustEntry| entry.key == key) {
                return Err(format!("'{at}.key' is listed twice"));
            }
            let name = match item.get("name") {
                Some(Value::String(text)) if !text.is_empty() && !text.chars().any(char::is_control) => text.clone(),
                _ => return Err(format!("'{at}.name' must be a name with no control character")),
            };
            let quantities = match item.get("quantities") {
                None => None,
                Some(Value::Array(names)) => Some(
                    names
                        .iter()
                        .map(|name| name.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()
                        .ok_or(format!("'{at}.quantities' must be an array of field names"))?,
                ),
                Some(_) => return Err(format!("'{at}.quantities' must be an array of field names")),
            };
            let time = |field: &str| -> Result<Option<(Instant, String)>, String> {
                match item.get(field) {
                    None => Ok(None),
                    Some(Value::String(text)) => Instant::read(text)
                        .map(|instant| Some((instant, text.clone())))
                        .ok_or(format!("'{at}.{field}' must be a time in RFC 3339 in UTC, ending in Z")),
                    Some(_) => Err(format!("'{at}.{field}' must be a time in RFC 3339 in UTC, ending in Z")),
                }
            };
            entries.push(TrustEntry {
                key,
                name,
                quantities,
                valid_from: time("valid_from")?,
                valid_to: time("valid_to")?,
            });
        }
        Ok(Trust { entries })
    }

    /// What the list says of `reading`.
    pub fn judge(&self, reading: &Verified) -> Trusted {
        let Some(entry) = self.entries.iter().find(|entry| entry.key == reading.key) else {
            return Trusted::No(format!(
                "the key {} is not on the list, whatever name it gives itself",
                reading.fingerprint
            ));
        };
        if let Some(allowed) = &entry.quantities {
            if let Some(other) = reading.quantities.iter().find(|quantity| !allowed.contains(quantity)) {
                return Trusted::No(format!(
                    "the list binds {} to {}, and the reading at {} states {other}",
                    Value::String(entry.name.clone()),
                    allowed.join(", "),
                    reading.place
                ));
            }
        }
        let at = Instant::read(&reading.time).expect("a verified reading's time reads");
        if let Some((from, text)) = &entry.valid_from {
            if at < *from {
                return Trusted::No(format!("the reading's time, {}, is before {text}, from when the list holds the key valid", reading.time));
            }
        }
        if let Some((to, text)) = &entry.valid_to {
            if at > *to {
                return Trusted::No(format!("the reading's time, {}, is after {text}, until when the list holds the key valid", reading.time));
            }
        }
        Trusted::Yes(entry.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    fn test_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn signed_load(key: &SigningKey, load: &Value, seq: u64, prev: Option<&str>, challenge: Option<&str>) -> Value {
        let mut signed = Map::new();
        signed.insert("alg".into(), json!("ed25519"));
        signed.insert("key".into(), json!(hex::encode(key.verifying_key().to_bytes())));
        signed.insert("instrument".into(), json!("test shunt"));
        signed.insert("time".into(), json!("2026-10-21T14:02:00.000Z"));
        signed.insert("seq".into(), json!(seq));
        signed.insert("prev".into(), prev.map(|text| json!(text)).unwrap_or(Value::Null));
        signed.insert("challenge".into(), challenge.map(|text| json!(text)).unwrap_or(Value::Null));
        let text = signed_text(Place::Segment(0), &signed, load);
        signed.insert("sig".into(), json!(hex::encode(key.sign(&text).to_bytes())));
        Value::Object(signed)
    }

    #[test]
    fn a_signed_reading_verifies_and_one_changed_value_does_not() {
        let key = test_key(7);
        let load = json!({ "current_a": 12.5 });
        let signed = signed_load(&key, &load, 1047, None, None);
        let verified = verify(Place::Segment(0), &signed, &load).expect("the reading verifies");
        assert_eq!(verified.seq, 1047);
        assert_eq!(verified.quantities, vec!["current_a".to_string()]);
        // The same value spelled another way is the same text.
        for same in [json!({ "current_a": 12.50 }), serde_json::from_str::<Value>(r#"{"current_a":1.25e1}"#).unwrap()] {
            assert!(verify(Place::Segment(0), &signed, &same).is_ok());
        }
        let edited = json!({ "current_a": 12.501 });
        let fault = verify(Place::Segment(0), &signed, &edited).unwrap_err();
        assert_eq!(fault.name(), DOES_NOT_VERIFY);
        assert!(matches!(fault, AttestFault::DoesNotVerify { why: Strict::Equation, .. }));
    }

    #[test]
    fn a_reading_moved_to_another_place_does_not_verify() {
        let key = test_key(9);
        let load = json!({ "current_a": 12.5 });
        let signed = signed_load(&key, &load, 1, None, None);
        for place in [Place::Initial, Place::Parameters, Place::Measurement(0)] {
            assert!(verify(place, &signed, &load).is_err(), "{place}");
        }
        // The place's index is not in the text: a reading moved to another
        // segment verifies there, and the stream shows where it came from.
        assert!(verify(Place::Segment(5), &signed, &load).is_ok());
        let renamed = json!({ "ambient_k": 12.5 });
        assert!(verify(Place::Segment(0), &signed, &renamed).is_err());
    }

    #[test]
    fn every_field_is_required_and_held_to_its_form() {
        let key = test_key(3);
        let load = json!({ "current_a": 1.0 });
        let signed = signed_load(&key, &load, 5, None, None);
        for field in FIELDS {
            let mut without = signed.clone();
            without.as_object_mut().unwrap().shift_remove(field);
            let fault = verify(Place::Segment(2), &without, &load).unwrap_err();
            assert_eq!(fault.to_string(), format!("Missing required parameter: 'segments[2].signed.{field}' (InputMissing)."));
        }
        for (field, value) in [
            ("key", json!("ab")),
            ("key", json!("AB".repeat(32))),
            ("sig", json!("00".repeat(63))),
            ("seq", json!(1.0)),
            ("seq", json!(-1)),
            ("seq", json!(9007199254740992_u64)),
            ("prev", json!("ab")),
            ("challenge", json!("abc")),
            ("challenge", json!("XY")),
            ("challenge", json!("")),
            ("time", json!("2026-10-21 14:02:00Z")),
            ("time", json!("2026-02-30T00:00:00Z")),
            ("time", json!("2026-10-21T14:02:00+00:00")),
            ("instrument", json!("")),
            ("instrument", json!("shunt-01\ntrusted: yes")),
            ("alg", json!(7)),
        ] {
            let mut changed = signed.clone();
            changed[field] = value.clone();
            let fault = verify(Place::Segment(0), &changed, &load).unwrap_err();
            assert_eq!(fault.name(), MALFORMED, "{field} {value}: {fault}");
            assert!(fault.to_string().contains(&format!("'segments[0].signed.{field}'")), "{fault}");
        }
        let mut other = signed.clone();
        other["alg"] = json!("ecdsa-p256-sha256");
        assert_eq!(verify(Place::Segment(0), &other, &load).unwrap_err().name(), ALGORITHM_NOT_SERVED);
        let mut extra = signed.clone();
        extra["extra"] = json!(1);
        assert_eq!(verify(Place::Segment(0), &extra, &load).unwrap_err().name(), MALFORMED);
        assert_eq!(verify(Place::Segment(0), &json!([]), &load).unwrap_err().name(), MALFORMED);
    }

    #[test]
    fn a_malleated_signature_and_a_substituted_key_do_not_verify() {
        let key = test_key(11);
        let load = json!({ "current_a": 2.0 });
        let signed = signed_load(&key, &load, 1, None, None);
        // S + L: the same signature in a second form.
        let sig = hex::decode(signed["sig"].as_str().unwrap()).unwrap();
        // L, the group order, little-endian.
        let order: [u8; 32] = [
            0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
        ];
        let mut s = [0_u8; 32];
        let mut carry = 0_u16;
        for index in 0..32 {
            let total = sig[32 + index] as u16 + order[index] as u16 + carry;
            s[index] = total as u8;
            carry = total >> 8;
        }
        let mut malleated = signed.clone();
        malleated["sig"] = json!(format!("{}{}", hex::encode(&sig[..32]), hex::encode(s)));
        assert!(matches!(
            verify(Place::Segment(0), &malleated, &load),
            Err(AttestFault::DoesNotVerify { why: Strict::SNotBelowOrder, .. })
        ));
        // Another key under the same signature.
        let mut substituted = signed.clone();
        substituted["key"] = json!(hex::encode(test_key(12).verifying_key().to_bytes()));
        assert!(matches!(
            verify(Place::Segment(0), &substituted, &load),
            Err(AttestFault::DoesNotVerify { why: Strict::Equation, .. })
        ));
        // The identity as a key verifies the signature (0, R=identity, S=0)
        // over any text; it is refused as a point of small order.
        let mut identity = [0_u8; 32];
        identity[0] = 1;
        let mut weak = signed.clone();
        weak["key"] = json!(hex::encode(identity));
        weak["sig"] = json!(format!("{}{}", hex::encode(identity), "00".repeat(32)));
        assert!(matches!(
            verify(Place::Segment(0), &weak, &load),
            Err(AttestFault::DoesNotVerify { why: Strict::KeySmallOrder, .. })
        ));
    }

    #[test]
    fn a_signature_made_for_another_purpose_does_not_verify_as_a_reading() {
        let key = test_key(13);
        let load = json!({ "current_a": 2.0 });
        let mut signed = signed_load(&key, &load, 1, None, None);
        // The device signed the same bytes with no domain line.
        let text = signed_text(Place::Segment(0), signed.as_object().unwrap(), &load);
        let bare = &text[DOMAIN_LINE.len()..];
        signed["sig"] = json!(hex::encode(key.sign(bare).to_bytes()));
        assert!(verify(Place::Segment(0), &signed, &load).is_err());
    }

    #[test]
    fn the_stream_names_the_first_break() {
        let key = test_key(21);
        let mut readings = Vec::new();
        let mut prev: Option<String> = None;
        for seq in 100..106_u64 {
            let load = json!({ "current_a": seq as f64 / 100.0 });
            let signed = signed_load(&key, &load, seq, prev.as_deref(), Some("9f2e"));
            let verified = verify(Place::Segment((seq - 100) as usize), &signed, &load).unwrap();
            prev = Some(verified.reading_sha256.clone());
            readings.push(verified);
        }
        assert_eq!(stream(&readings), Stream::Unbroken);
        let mut dropped = readings.clone();
        dropped.remove(3);
        assert!(matches!(stream(&dropped), Stream::Breaks { at: Place::Segment(4), .. }));
        let mut swapped = readings.clone();
        swapped.swap(2, 3);
        assert!(matches!(stream(&swapped), Stream::Breaks { at: Place::Segment(3), .. }));
        let label = load_source(6, &readings);
        assert_eq!(label["signed"], 6);
        assert_eq!(label["stream"], "unbroken");
        assert_eq!(label["keys"][0]["places"], "segments[0] to segments[5]");
        assert_eq!(label["keys"][0]["seq"], "100 to 105");
        assert_eq!(label["keys"][0]["challenges"], json!(["9f2e"]));
        let partly = load_source(8, &readings);
        assert!(partly["says"].as_str().unwrap().contains("the loads at segments[6] to segments[7] are the caller's number"), "{partly}");
        assert_eq!(load_source(3, &[])["says"], CALLERS_NUMBER);
        // An unbroken stream adds nothing to what the label says.
        assert!(!label["says"].as_str().unwrap().contains("stream"), "{label}");
    }

    /// A stream that is not unbroken (a reading dropped, two reordered, one
    /// spliced in from another stream) is said in `says`, not only in
    /// `stream`, in the place of the first break.
    #[test]
    fn a_broken_stream_is_said_in_the_sentence() {
        let key = test_key(22);
        let other = test_key(23);
        let mut readings = Vec::new();
        let mut prev: Option<String> = None;
        for seq in 100..106_u64 {
            let load = json!({ "current_a": seq as f64 / 100.0 });
            let signed = signed_load(&key, &load, seq, prev.as_deref(), Some("9f2e"));
            let verified = verify(Place::Segment((seq - 100) as usize), &signed, &load).unwrap();
            prev = Some(verified.reading_sha256.clone());
            readings.push(verified);
        }
        let reorder = |readings: &mut Vec<Verified>| {
            for (index, reading) in readings.iter_mut().enumerate() {
                reading.place = Place::Segment(index);
            }
        };
        let mut dropped = readings.clone();
        dropped.remove(3);
        reorder(&mut dropped);
        let mut swapped = readings.clone();
        swapped.swap(2, 3);
        reorder(&mut swapped);
        let mut spliced = readings.clone();
        let load = json!({ "current_a": 9.0 });
        spliced[4] = verify(Place::Segment(4), &signed_load(&other, &load, 104, Some(&readings[3].reading_sha256), Some("9f2e")), &load).unwrap();
        let mut spliced_same_key = readings.clone();
        spliced_same_key[4] = verify(Place::Segment(4), &signed_load(&key, &load, 104, Some(&"0".repeat(64)), Some("9f2e")), &load).unwrap();
        for (what, broken, at) in [("dropped", &dropped, 3), ("reordered", &swapped, 2), ("spliced", &spliced_same_key, 4)] {
            let label = load_source(broken.len(), broken);
            let says = label["says"].as_str().unwrap();
            let stream = label["stream"].as_str().unwrap();
            assert!(stream.starts_with(&format!("breaks at segments[{at}]")), "{what}: {label}");
            assert!(says.contains(&format!(" The signed readings are not one unbroken stream: the stream {stream}. So a reading of that key is missing there, or the readings were reordered, or spliced from another stream. ")), "{what}: {says}");
            assert!(says.starts_with("Every load of this run carries a signature that verifies under the key shown"), "{what}: {says}");
            assert!(says.ends_with(WHO_SIGNED), "{what}: {says}");
        }
        // A reading of a second key put in the place of one of the first: the
        // second key's reading is a stream of its own, and the first key's
        // stream breaks where its reading is missing.
        let label = load_source(6, &spliced);
        assert!(label["stream"].as_str().unwrap().starts_with("breaks at segments[5]: its prev is not the SHA-256 of the signed text at segments[3]"), "{label}");
        assert!(label["says"].as_str().unwrap().contains("the caller's number. The signed readings are not one unbroken stream: the stream breaks at segments[5]"), "{label}");
        // The same of measurements compared.
        let measured: Vec<Verified> = dropped
            .iter()
            .enumerate()
            .map(|(index, reading)| Verified { place: Place::Measurement(index), ..reading.clone() })
            .collect();
        let label = measurement_source(measured.len(), &measured);
        assert!(label["says"].as_str().unwrap().contains(" The signed measurements are not one unbroken stream: the stream breaks at measurements[3]"), "{label}");
    }

    #[test]
    fn the_trust_list_names_the_key_its_quantities_and_its_span() {
        let key = test_key(31);
        let load = json!({ "current_a": 1.0 });
        let reading = verify(Place::Segment(0), &signed_load(&key, &load, 1, None, None), &load).unwrap();
        let listed = format!(
            r#"{{"keys":[{{"key":"{}","name":"bench shunt","quantities":["current_a"],"valid_from":"2026-10-01T00:00:00Z","valid_to":"2026-12-31T23:59:59.999Z"}}]}}"#,
            reading.key
        );
        let trust = Trust::read(listed.as_bytes()).unwrap();
        assert_eq!(trust.judge(&reading), Trusted::Yes("bench shunt".into()));
        let voltage = Trust::read(listed.replace("current_a", "voltage_v").as_bytes()).unwrap();
        assert!(matches!(voltage.judge(&reading), Trusted::No(_)));
        let early = Trust::read(listed.replace("2026-12-31T23:59:59.999Z", "2026-10-21T14:01:59.999999999Z").as_bytes()).unwrap();
        assert!(matches!(early.judge(&reading), Trusted::No(_)));
        let other = Trust::read(br#"{"keys":[]}"#).unwrap();
        assert!(other.judge(&reading).text().starts_with("no: the key "));
        assert!(Trust::read(br#"{"keys":[{"key":"ab","name":"x"}]}"#).is_err());
    }

    #[test]
    fn a_time_reads_only_in_its_one_form_and_orders_as_time() {
        let early = Instant::read("2026-10-21T14:02:00Z").unwrap();
        let later = Instant::read("2026-10-21T14:02:00.5Z").unwrap();
        assert!(early < later);
        assert_eq!(Instant::read("2026-10-21T14:02:00.500Z"), Some(later));
        assert!(Instant::read("2024-02-29T00:00:00Z").is_some());
        assert!(Instant::read("2026-12-31T23:59:60Z").is_some());
        for wrong in ["2026-10-21T24:00:00Z", "2026-13-01T00:00:00Z", "2026-10-21T14:02:00.Z", "2026-10-21T14:02:00.1234567890Z", "2026-10-21t14:02:00Z", "+2026-10-21T14:02:00Z"] {
            assert!(Instant::read(wrong).is_none(), "{wrong}");
        }
    }
}
