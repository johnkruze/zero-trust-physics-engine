//! A chain of runs: one run's sealed final state carried into the next
//! request, by continuation (the same card starts again from the start fields
//! it takes, each equal to the upstream's sealed value) or by
//! a named coupling (named outputs of one card become named inputs of
//! another), and the `chain_seal` that makes the chain one checkable object.
//!
//! A link after the first carries a `chain` object in its answer:
//!
//! ```text
//! {"kind":"continuation"|"coupling","coupling":null|<name>,
//!  "upstream_run_seal":<64 hex>,"upstream_chain_seal":null|<64 hex>,
//!  "handed":[{"from":"final_state.<name>","to":<path>,"unit":null|<unit>,"value_hex":<hex>},...],
//!  "chain_seal":<64 hex>}
//! ```
//!
//! `chain_seal` is SHA-256 of three texts with nothing between them: the
//! upstream commitment as 64 hex characters (`upstream_chain_seal` when the
//! upstream is itself a chained link, else `upstream_run_seal`), the compact
//! JSON of `kind`, `coupling` and `handed` in that order ([`chain_tail`]),
//! and the downstream seal as 64 hex characters (`run_seal` of a run,
//! `sha256_seal` of a step).
//!
//! `from` is `final_state.<name>` (the upstream's sealed final value) or
//! `inputs.` and a request path (a value of the upstream's sealed request).
//! A request path is `initial.<name>` or `segments[*].load.<name>` (every
//! segment) in a run request, and `parameters.<name>` in a step request. A
//! value is carried bit for bit: the request's number, read as the nearest
//! 64-bit number, has the bits of `value_hex`.
//!
//! This crate knows no card and no coupling. It checks that every link holds
//! alone, that each link names its upstream's seals, that every handed value
//! is the upstream's sealed final value and is the downstream request's
//! value, and that each `chain_seal` is rebuilt from those parts. That the
//! coupling is one the engine serves, and that a continuation carried every
//! field the card starts from, the engine shows when the last request is
//! sent to `verify_tape` with its seal.

use std::fmt;

use serde_json::{Map, Value};

use crate::json_text::is_lower_hex;
use crate::refusal::{Refusal, required, wrong};
use crate::run::check_tape;
use crate::sealed_form::{digest_hex, json_string};

/// The most runs a chain holds, the last one included. The engine holds the
/// same limit.
pub const MAX_CHAIN_LINKS: usize = 16;

/// The kind of a link: the same card going on, or a named coupling.
pub const KIND_CONTINUATION: &str = "continuation";
pub const KIND_COUPLING: &str = "coupling";

/// One value carried across a link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handed {
    /// `final_state.<name>` of the upstream run, or `inputs.<request path>`.
    pub from: String,
    /// Where the value stands in the downstream request.
    pub to: String,
    /// The unit, for a coupling; null for a continuation.
    pub unit: Option<String>,
    /// The exact value: 16 hex digits, or an array of them for a vector.
    pub value_hex: Value,
}

/// The compact JSON a chain seal is taken over between the two seals:
/// `kind`, `coupling` and `handed`, in that order, with no spaces. Each
/// handed entry is `from`, `to`, `unit`, `value_hex` in that order.
pub fn chain_tail(kind: &str, coupling: Option<&str>, handed: &[Handed]) -> String {
    let mut text = format!(
        "{{\"kind\":{},\"coupling\":{},\"handed\":[",
        json_string(kind),
        coupling.map(json_string).unwrap_or_else(|| "null".to_string())
    );
    for (index, entry) in handed.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        text.push_str(&format!(
            "{{\"from\":{},\"to\":{},\"unit\":{},\"value_hex\":{}}}",
            json_string(&entry.from),
            json_string(&entry.to),
            entry
                .unit
                .as_deref()
                .map(json_string)
                .unwrap_or_else(|| "null".to_string()),
            hex_text(&entry.value_hex)
        ));
    }
    text.push_str("]}");
    text
}

fn hex_text(value: &Value) -> String {
    match value {
        Value::String(text) => json_string(text),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(hex_text).collect();
            format!("[{}]", parts.join(","))
        }
        // Only strings and arrays of them are handed; the reader refuses the rest.
        _ => "null".to_string(),
    }
}

/// SHA-256 of the upstream commitment, [`chain_tail`] and the downstream seal.
pub fn chain_seal(upstream_commitment: &str, tail: &str, downstream_seal: &str) -> String {
    let mut bytes = Vec::with_capacity(128 + tail.len());
    bytes.extend_from_slice(upstream_commitment.as_bytes());
    bytes.extend_from_slice(tail.as_bytes());
    bytes.extend_from_slice(downstream_seal.as_bytes());
    digest_hex(&bytes)
}

/// The exact value of a number as it stands in a request: 16 hex digits of
/// its bits, or an array of those for an array of numbers. `None` when the
/// value is neither.
pub fn request_value_hex(value: &Value) -> Option<Value> {
    match value {
        Value::Number(number) => {
            let float = number.as_f64()?;
            Some(Value::String(format!("{:016x}", float.to_bits())))
        }
        Value::Array(items) if !items.is_empty() => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(request_value_hex(item)?);
            }
            Some(Value::Array(out))
        }
        _ => None,
    }
}

/// The values a path names in a request: one for `initial.<name>` and
/// `parameters.<name>`, one per segment for `segments[*].load.<name>`. A
/// value that is absent is `None` in its place. `Err` when the path is none
/// of the three forms.
pub fn values_at<'a>(request: &'a Value, path: &str) -> Result<Vec<Option<&'a Value>>, String> {
    if let Some(name) = path.strip_prefix("initial.") {
        return Ok(vec![request.get("initial").and_then(|initial| initial.get(name))]);
    }
    if let Some(name) = path.strip_prefix("parameters.") {
        return Ok(vec![request.get("parameters").and_then(|parameters| parameters.get(name))]);
    }
    if let Some(name) = path.strip_prefix("segments[*].load.") {
        let segments = request
            .get("segments")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if segments.is_empty() {
            return Ok(vec![None]);
        }
        return Ok(segments
            .iter()
            .map(|segment| segment.get("load").and_then(|load| load.get(name)))
            .collect());
    }
    Err(format!("'{path}' is not a path a value is handed to"))
}

/// The part of a chain that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainCause {
    /// A run's frames do not follow from its request.
    Chain,
    /// A signed reading of a run's request does not verify.
    LoadSignature,
    /// A run's state digest is not its final state's.
    State,
    /// A run's seal is not rebuilt from its parts.
    RunSeal,
    /// A link does not name the seals of the link before it, or its place.
    Upstream,
    /// The upstream run did not end in a way this kind of link may follow.
    Outcome,
    /// A continuation names another card than its upstream.
    Pieces,
    /// A handed value is not the upstream's sealed final value, or not the
    /// downstream request's value, or the request states a start value that
    /// no handed entry carries.
    Handed,
    /// The chain seal is not rebuilt from its parts.
    ChainSeal,
}

impl ChainCause {
    pub fn as_str(self) -> &'static str {
        match self {
            ChainCause::Chain => "chain",
            ChainCause::LoadSignature => "load_signature",
            ChainCause::State => "state",
            ChainCause::RunSeal => "run_seal",
            ChainCause::Upstream => "upstream",
            ChainCause::Outcome => "outcome",
            ChainCause::Pieces => "pieces",
            ChainCause::Handed => "handed",
            ChainCause::ChainSeal => "chain_seal",
        }
    }
}

impl fmt::Display for ChainCause {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(self.as_str())
    }
}

/// The answer to the check of a chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReport {
    pub holds: bool,
    pub links: usize,
    /// The index of the first link that fails, from 0.
    pub broken_link: Option<usize>,
    pub cause: Option<ChainCause>,
    /// The chain seal of the last link when the chain holds.
    pub chain_seal: Option<String>,
    /// True when the last link is a step, whose frame is not in its answer:
    /// its `sha256_seal` is shown by sending its request again.
    pub last_is_step: bool,
}

fn broken(index: usize, cause: ChainCause, links: usize) -> ChainReport {
    ChainReport {
        holds: false,
        links,
        broken_link: Some(index),
        cause: Some(cause),
        chain_seal: None,
        last_is_step: false,
    }
}

/// Scan the text of a chain, read it, and check it.
pub fn check_chain_text(bytes: &[u8]) -> Result<ChainReport, Refusal> {
    let body = crate::json_text::read(bytes)?;
    check_chain(&body)
}

/// Check a chain without the engine. The body is `{"links": [<answer>, ...]}`:
/// the answers in order, the first a run answer with no `chain`, each later
/// one with its `chain`. Every link but the last is a `run_trajectory`
/// answer; the last may be a `verify_step` answer.
pub fn check_chain(body: &Value) -> Result<ChainReport, Refusal> {
    let object = body.as_object().ok_or(Refusal::NotAnObject)?;
    for key in object.keys() {
        if key != "links" {
            return Err(Refusal::UnknownKey(key.clone()));
        }
    }
    let links = required(object, "links")?
        .as_array()
        .ok_or_else(|| wrong("links", "a JSON array"))?;
    if links.len() < 2 {
        return Err(wrong("links", "an array of at least two answers"));
    }
    if links.len() > MAX_CHAIN_LINKS {
        return Err(wrong("links", "an array of at most 16 answers"));
    }
    let count = links.len();
    let mut readings = Vec::with_capacity(count);
    for (index, link) in links.iter().enumerate() {
        readings.push(read_link(index, link, index + 1 == count)?);
    }
    // Every link holds alone first: a broken run is named before anything
    // that leans on it.
    for (index, reading) in readings.iter().enumerate() {
        if reading.is_step {
            continue;
        }
        let report = check_tape(&links[index])?;
        if !report.holds {
            let cause = match report.cause {
                Some(crate::run::TapeCause::Chain) => ChainCause::Chain,
                Some(crate::run::TapeCause::LoadSignature) => ChainCause::LoadSignature,
                Some(crate::run::TapeCause::State) => ChainCause::State,
                _ => ChainCause::RunSeal,
            };
            return Ok(broken(index, cause, count));
        }
    }
    let mut upstream_commitment = readings[0].seal.clone();
    let mut last_chain_seal = None;
    for index in 1..count {
        let upstream = &readings[index - 1];
        let link = &readings[index];
        let chain = link.chain.as_ref().expect("read_link requires a chain after the first");
        let expected_upstream_chain = if index == 1 { None } else { last_chain_seal.clone() };
        if chain.upstream_run_seal != upstream.seal || chain.upstream_chain_seal != expected_upstream_chain {
            return Ok(broken(index, ChainCause::Upstream, count));
        }
        let outcome_admitted = match chain.kind.as_str() {
            KIND_CONTINUATION => upstream.outcome == "completed",
            _ => upstream.outcome == "completed" || upstream.outcome == "ended",
        };
        if !outcome_admitted || upstream.final_state.is_none() {
            return Ok(broken(index, ChainCause::Outcome, count));
        }
        if chain.kind == KIND_CONTINUATION && upstream.pair != link.pair {
            return Ok(broken(index, ChainCause::Pieces, count));
        }
        if !handed_hold(upstream, link, chain) {
            return Ok(broken(index, ChainCause::Handed, count));
        }
        let tail = chain_tail(&chain.kind, chain.coupling.as_deref(), &chain.handed);
        let rebuilt = chain_seal(&upstream_commitment, &tail, &link.seal);
        if rebuilt != chain.chain_seal {
            return Ok(broken(index, ChainCause::ChainSeal, count));
        }
        upstream_commitment = rebuilt.clone();
        last_chain_seal = Some(rebuilt);
    }
    Ok(ChainReport {
        holds: true,
        links: count,
        broken_link: None,
        cause: None,
        chain_seal: last_chain_seal,
        last_is_step: readings[count - 1].is_step,
    })
}

struct ChainPart {
    kind: String,
    coupling: Option<String>,
    upstream_run_seal: String,
    upstream_chain_seal: Option<String>,
    handed: Vec<Handed>,
    chain_seal: String,
}

struct Link<'a> {
    is_step: bool,
    pair: (String, String),
    inputs: &'a Value,
    outcome: String,
    /// `run_seal` of a run, `sha256_seal` of a step.
    seal: String,
    final_state: Option<&'a Map<String, Value>>,
    chain: Option<ChainPart>,
}

fn field(index: usize, name: &str) -> String {
    format!("links[{index}].{name}")
}

fn read_link<'a>(index: usize, link: &'a Value, last: bool) -> Result<Link<'a>, Refusal> {
    let object = link.as_object().ok_or_else(|| wrong(&format!("links[{index}]"), "a JSON object"))?;
    let inputs = object
        .get("inputs")
        .ok_or_else(|| Refusal::Missing(field(index, "inputs")))?;
    let pair = match (
        inputs.get("domain").and_then(Value::as_str),
        inputs.get("card").and_then(Value::as_str),
    ) {
        (Some(domain), Some(card)) => (domain.to_string(), card.to_string()),
        _ => return Err(Refusal::NamesNoCard),
    };
    let is_step = object.contains_key("sha256_seal") && !object.contains_key("run_seal");
    if is_step && !last {
        return Err(wrong(&format!("links[{index}]"), "a run_trajectory answer: only the last link may be a step"));
    }
    let (seal, outcome, final_state) = if is_step {
        let seal = hash_field(object, index, "sha256_seal")?;
        let state = object
            .get("state")
            .and_then(Value::as_object)
            .ok_or_else(|| wrong(&field(index, "state"), "a JSON object"))?;
        (seal, "completed".to_string(), Some(state))
    } else {
        let seal = hash_field(object, index, "run_seal")?;
        let outcome = object
            .get("outcome")
            .and_then(Value::as_str)
            .ok_or_else(|| Refusal::Missing(field(index, "outcome")))?
            .to_string();
        let final_state = object.get("final_state").and_then(Value::as_object);
        (seal, outcome, final_state)
    };
    let chain = match object.get("chain") {
        None if index == 0 => None,
        Some(_) if index == 0 => {
            return Err(wrong(
                &field(0, "chain"),
                "absent: the first link is where the chain starts, so send its upstream links too",
            ));
        }
        None => return Err(Refusal::Missing(field(index, "chain"))),
        Some(chain) => Some(read_chain(index, chain)?),
    };
    Ok(Link {
        is_step,
        pair,
        inputs,
        outcome,
        seal,
        final_state,
        chain,
    })
}

fn hash_field(object: &Map<String, Value>, index: usize, key: &str) -> Result<String, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(field(index, key))),
        Some(Value::String(text)) if is_lower_hex(text, 64) => Ok(text.clone()),
        Some(_) => Err(Refusal::NotExactHex {
            field: field(index, key),
            digits: 64,
        }),
    }
}

fn read_chain(index: usize, chain: &Value) -> Result<ChainPart, Refusal> {
    let name = |key: &str| field(index, &format!("chain.{key}"));
    let object = chain.as_object().ok_or_else(|| wrong(&name(""), "a JSON object"))?;
    for key in object.keys() {
        if !["kind", "coupling", "upstream_run_seal", "upstream_chain_seal", "handed", "chain_seal"]
            .contains(&key.as_str())
        {
            return Err(Refusal::UnknownKey(name(key)));
        }
    }
    let kind = match object.get("kind") {
        Some(Value::String(text)) if text == KIND_CONTINUATION || text == KIND_COUPLING => text.clone(),
        Some(_) => return Err(wrong(&name("kind"), "\"continuation\" or \"coupling\"")),
        None => return Err(Refusal::Missing(name("kind"))),
    };
    let coupling = match (kind.as_str(), object.get("coupling")) {
        (KIND_CONTINUATION, Some(Value::Null)) => None,
        (KIND_COUPLING, Some(Value::String(text))) if !text.is_empty() => Some(text.clone()),
        (KIND_CONTINUATION, _) => return Err(wrong(&name("coupling"), "null for a continuation")),
        (_, _) => return Err(wrong(&name("coupling"), "the coupling's name")),
    };
    let hash = |key: &str| -> Result<Option<String>, Refusal> {
        match object.get(key) {
            None => Err(Refusal::Missing(name(key))),
            Some(Value::Null) => Ok(None),
            Some(Value::String(text)) if is_lower_hex(text, 64) => Ok(Some(text.clone())),
            Some(_) => Err(Refusal::NotExactHex { field: name(key), digits: 64 }),
        }
    };
    let upstream_run_seal = hash("upstream_run_seal")?.ok_or_else(|| Refusal::Missing(name("upstream_run_seal")))?;
    let upstream_chain_seal = hash("upstream_chain_seal")?;
    let chain_seal = hash("chain_seal")?.ok_or_else(|| Refusal::Missing(name("chain_seal")))?;
    let entries = object
        .get("handed")
        .ok_or_else(|| Refusal::Missing(name("handed")))?
        .as_array()
        .ok_or_else(|| wrong(&name("handed"), "a JSON array"))?;
    if entries.is_empty() {
        return Err(wrong(&name("handed"), "an array of at least one value"));
    }
    let mut handed = Vec::with_capacity(entries.len());
    for (at, entry) in entries.iter().enumerate() {
        let entry_name = |key: &str| name(&format!("handed[{at}].{key}"));
        let entry = entry.as_object().ok_or_else(|| wrong(&entry_name(""), "a JSON object"))?;
        let text = |key: &str| -> Result<String, Refusal> {
            entry
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| wrong(&entry_name(key), "a string"))
        };
        let from = text("from")?;
        let to = text("to")?;
        let unit = match entry.get("unit") {
            Some(Value::Null) => None,
            Some(Value::String(unit)) => Some(unit.clone()),
            _ => return Err(wrong(&entry_name("unit"), "a string or null")),
        };
        let value_hex = entry
            .get("value_hex")
            .cloned()
            .ok_or_else(|| Refusal::Missing(entry_name("value_hex")))?;
        if !is_hex_value(&value_hex) {
            return Err(Refusal::NotExactHex { field: entry_name("value_hex"), digits: 16 });
        }
        if entry.len() != 4 {
            return Err(Refusal::UnknownKey(entry_name("")));
        }
        handed.push(Handed { from, to, unit, value_hex });
    }
    Ok(ChainPart {
        kind,
        coupling,
        upstream_run_seal,
        upstream_chain_seal,
        handed,
        chain_seal,
    })
}

fn is_hex_value(value: &Value) -> bool {
    match value {
        Value::String(text) => is_lower_hex(text, 16),
        Value::Array(items) => !items.is_empty() && items.iter().all(is_hex_value),
        _ => false,
    }
}

/// Every handed value is the upstream's sealed final value and the
/// downstream request's value. For a continuation into a run, every start
/// value the request states is handed.
fn handed_hold(upstream: &Link, link: &Link, chain: &ChainPart) -> bool {
    let Some(state) = upstream.final_state else {
        return false;
    };
    for entry in &chain.handed {
        if let Some(name) = entry.from.strip_prefix("final_state.") {
            if state.get(&format!("{name}_hex")) != Some(&entry.value_hex) {
                return false;
            }
        } else if let Some(path) = entry.from.strip_prefix("inputs.") {
            let Ok(values) = values_at(upstream.inputs, path) else {
                return false;
            };
            for value in values {
                if value.and_then(request_value_hex).as_ref() != Some(&entry.value_hex) {
                    return false;
                }
            }
        } else {
            return false;
        }
        let Ok(values) = values_at(link.inputs, &entry.to) else {
            return false;
        };
        for value in values {
            if value.and_then(request_value_hex).as_ref() != Some(&entry.value_hex) {
                return false;
            }
        }
    }
    if chain.kind == KIND_CONTINUATION && !link.is_step {
        let Some(initial) = link.inputs.get("initial").and_then(Value::as_object) else {
            return false;
        };
        for key in initial.keys() {
            let path = format!("initial.{key}");
            if !chain.handed.iter().any(|entry| entry.to == path) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_tail_is_compact_json_in_its_order() {
        let handed = vec![
            Handed {
                from: "final_state.t_s".into(),
                to: "initial.t_s".into(),
                unit: None,
                value_hex: json!("3ff0000000000000"),
            },
            Handed {
                from: "final_state.position_m".into(),
                to: "initial.position_m".into(),
                unit: Some("m".into()),
                value_hex: json!(["0000000000000000", "3ff0000000000000"]),
            },
        ];
        assert_eq!(
            chain_tail(KIND_CONTINUATION, None, &handed),
            r#"{"kind":"continuation","coupling":null,"handed":[{"from":"final_state.t_s","to":"initial.t_s","unit":null,"value_hex":"3ff0000000000000"},{"from":"final_state.position_m","to":"initial.position_m","unit":"m","value_hex":["0000000000000000","3ff0000000000000"]}]}"#
        );
        assert_eq!(
            chain_tail(KIND_COUPLING, Some("a_to_b"), &[]),
            r#"{"kind":"coupling","coupling":"a_to_b","handed":[]}"#
        );
    }

    #[test]
    fn the_chain_seal_is_the_hash_of_the_three_texts() {
        let up = "ab".repeat(32);
        let down = "cd".repeat(32);
        let tail = chain_tail(KIND_CONTINUATION, None, &[]);
        let mut joined = up.clone();
        joined.push_str(&tail);
        joined.push_str(&down);
        assert_eq!(chain_seal(&up, &tail, &down), digest_hex(joined.as_bytes()));
        assert_ne!(chain_seal(&down, &tail, &up), chain_seal(&up, &tail, &down));
    }

    #[test]
    fn a_value_is_read_as_its_bits_and_a_path_names_every_segment() {
        assert_eq!(request_value_hex(&json!(1.0)), Some(json!("3ff0000000000000")));
        assert_eq!(request_value_hex(&json!(1)), Some(json!("3ff0000000000000")));
        assert_eq!(request_value_hex(&json!(-0.0)), Some(json!("8000000000000000")));
        assert_eq!(request_value_hex(&json!("1")), None);
        assert_eq!(request_value_hex(&json!([])), None);
        let request = json!({
            "initial": {"x": 2.0},
            "segments": [{"steps": 1, "load": {"r": 1.0}}, {"steps": 1, "load": {}}]
        });
        assert_eq!(values_at(&request, "initial.x").unwrap(), vec![Some(&json!(2.0))]);
        assert_eq!(
            values_at(&request, "segments[*].load.r").unwrap(),
            vec![Some(&json!(1.0)), None]
        );
        assert!(values_at(&request, "dt_s").is_err());
    }

    #[test]
    fn a_body_that_is_not_a_chain_is_refused_by_name() {
        assert_eq!(check_chain(&json!([])).unwrap_err(), Refusal::NotAnObject);
        assert_eq!(
            check_chain(&json!({"links": [], "x": 1})).unwrap_err(),
            Refusal::UnknownKey("x".into())
        );
        assert!(check_chain(&json!({"links": [{}]})).is_err());
    }
}
