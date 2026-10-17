//! The served contract of a card and a solver, attacked through the local
//! connection from the model's own description.
//!
//! Nothing here knows a model: every attack is built from what
//! `describe_card` or `describe_solver` says, so a model plugs in by its
//! name. `card_findings` and `solver_findings` return what was found;
//! `assert_card_conforms` and `assert_solver_conforms` fail on any finding
//! that breaks the contract, for a new model's router test.
//!
//! What is attacked, on a card:
//!
//! - every declared bound of a start field and a load field, one 64-bit
//!   number past it (the bound itself where it is exclusive), is refused by
//!   a name the card lists, and no state past it is sealed;
//! - a start or load field left out, and a key the card does not take, are
//!   refused: nothing is filled in, and the caller never sets the specimen;
//! - a step one number under the floor or over the ceiling is refused by a
//!   listed name, and a step of the floor or the ceiling is not refused for
//!   its size;
//! - the worked run reports a residual at or under 1e-6, finite, on every
//!   frame; an `ended` run ends on a declared end; the same request gives the
//!   same bytes, on one connection and on a second;
//! - the short form's stops are the full form's names, and the description's
//!   words: see `text_findings`.
//!
//! On a solver: every input's bound, one number past it (the bound itself
//! where it is exclusive), is refused by the input's own name, in the worked
//! question, or for an input it does not hold in the first further worked
//! question (`example_solve_*`) that does; an input left out and a key it does not take are
//! refused; every refusal named in the inputs, the invariants and the
//! convergence is listed; the worked question gives the same bytes twice and
//! on a second connection; a converged answer's invariants are inside the
//! tolerances the description states; and the description's words.
//!
//! On every model compared with measurements: forecast first, its protocol
//! committed with its fingerprint before its scoring (`forecast_findings`).
//! On every model calibrated to the record: its calibration, the companion
//! it stands beside and the fingerprint and scoring of the record it was
//! compared with, and forecast first for a blind registration beside it
//! (`calibration_findings`).

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

/// How much a finding weighs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weight {
    /// A rule of the contract is broken.
    Contract,
    /// Worth a look; the contract may still hold.
    Note,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub model: String,
    pub rule: &'static str,
    pub weight: Weight,
    pub detail: String,
}

impl Finding {
    pub fn line(&self) -> String {
        let weight = if self.weight == Weight::Contract { "CONTRACT" } else { "note" };
        format!("{} | {} | {} | {}", self.model, weight, self.rule, self.detail)
    }

    pub fn json(&self) -> String {
        json!({
            "model": self.model,
            "layer": "door",
            "weight": if self.weight == Weight::Contract { "contract" } else { "note" },
            "rule": self.rule,
            "detail": self.detail,
        })
        .to_string()
    }
}

/// The local connection, `mcp_stdio`, one request at a time.
pub struct Door {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    next: u64,
}

impl Door {
    /// This crate's `mcp_stdio`, or the binary `CONFORMANCE_BINARY` names:
    /// the sweeps then attack any build, the integration's or the served one,
    /// with no build of it here.
    pub fn start() -> Door {
        match std::env::var("CONFORMANCE_BINARY") {
            Ok(path) if !path.is_empty() => Door::start_binary(&path),
            _ => Door::start_binary(env!("CARGO_BIN_EXE_mcp_stdio")),
        }
    }

    pub fn start_binary(path: &str) -> Door {
        let mut child = Command::new(path).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().expect("mcp_stdio starts");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut door = Door { child, stdin, out, next: 0 };
        door.line(&json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"conformance","version":"1"}}}).to_string());
        door.stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n").unwrap();
        door
    }

    fn line(&mut self, text: &str) -> String {
        self.stdin.write_all(text.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).unwrap();
        answer
    }

    /// `(is_error, text)` of one tool call.
    pub fn call(&mut self, tool: &str, arguments: &Value) -> (bool, String) {
        self.next += 1;
        let body = json!({"jsonrpc":"2.0","id":self.next,"method":"tools/call","params":{"name":tool,"arguments":arguments}}).to_string();
        let answer = self.line(&body);
        let value: Value = serde_json::from_str(&answer).unwrap_or(Value::Null);
        (value["result"]["isError"] == json!(true), value["result"]["content"][0]["text"].as_str().unwrap_or("").to_string())
    }

    /// The JSON of an answer that is not an error.
    pub fn value(&mut self, tool: &str, arguments: &Value) -> Option<Value> {
        let (error, text) = self.call(tool, arguments);
        if error { None } else { serde_json::from_str(&text).ok() }
    }
}

impl Drop for Door {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Out {
    model: String,
    found: Vec<Finding>,
}

impl Out {
    fn new(model: &str) -> Out {
        Out { model: model.to_string(), found: Vec::new() }
    }
    fn contract(&mut self, rule: &'static str, detail: String) {
        self.found.push(Finding { model: self.model.clone(), rule, weight: Weight::Contract, detail });
    }
    fn note(&mut self, rule: &'static str, detail: String) {
        self.found.push(Finding { model: self.model.clone(), rule, weight: Weight::Note, detail });
    }
}

// ----- the words of a description -------------------------------------------

/// Every string of a value, with the path to it, and every key.
fn strings(value: &Value, path: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::String(text) => out.push((path.to_string(), text.clone())),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                strings(item, &format!("{path}[{index}]"), out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                out.push((format!("{path}.{key} (key)"), key.clone()));
                strings(item, &format!("{path}.{key}"), out);
            }
        }
        _ => {}
    }
}

/// The text around byte `at`, for a finding.
fn around(text: &str, at: usize, wide: usize) -> String {
    let mut start = at.saturating_sub(wide);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (at + wide).min(text.len());
    while !text.is_char_boundary(end) {
        end += 1;
    }
    format!("...{}...", &text[start..end])
}

/// Where `word` starts a word of `text`, ignoring case.
fn word_starts(text: &str, word: &str) -> Vec<usize> {
    let lower = text.to_lowercase();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find(word) {
        let at = from + at;
        let before = lower[..at].chars().next_back();
        if !before.is_some_and(|c| c.is_alphanumeric()) {
            found.push(at);
        }
        from = at + word.len();
    }
    found
}

/// Phrases that describe a model by what it lacks: John Kruze's decision 8
/// of 14 October 2026, "A model is never described by what it lacks".
pub const LACKS_STRONG: &[&str] = &[
    "generic specimen",
    "not a named real object",
    "not compared with a measurement",
    "not compared with any measurement",
    "nothing is stated about how closely",
];

/// Phrases that may describe a model by what it lacks, for a look.
pub const LACKS_WEAK: &[&str] = &["left out", "lacks", "not modelled", "does not model", "returns no ", "is not a measurement", "not measured", "no measurement of"];

/// A weak phrase that is about a fit, not the model: "left out of the fit".
fn about_a_fit(text: &str, at: usize) -> bool {
    let rest = &text[at..];
    rest.starts_with("left out of the fit") || rest.starts_with("left out of a fit")
}

/// A refusal of the ledger: its name says so, whatever its sentence says.
fn ledger_refusal(name: &str) -> bool {
    name.ends_with("Breach") || name == "SpontaneousGeneration" || name.ends_with("Broken")
}

/// The LIFT rows of `REFUSALS.md`, by model: each refusal name with the
/// words after it, when `CONFORMANCE_REFUSALS` names that file. A LIFT row
/// is a wall at the edge of data that John Kruze decided, 14 October 2026,
/// is lifted in the next engine version.
pub fn lifts() -> &'static std::collections::BTreeMap<String, Vec<(String, String)>> {
    static LIFTS: std::sync::OnceLock<std::collections::BTreeMap<String, Vec<(String, String)>>> = std::sync::OnceLock::new();
    LIFTS.get_or_init(|| {
        let mut table = std::collections::BTreeMap::new();
        let Ok(path) = std::env::var("CONFORMANCE_REFUSALS") else { return table };
        let Ok(text) = std::fs::read_to_string(&path) else { return table };
        let Some(start) = text.find("### Every LIFT, in one place") else { return table };
        for line in text[start..].lines().skip(1) {
            if line.starts_with('#') {
                break;
            }
            let Some(rest) = line.strip_prefix("- **") else { continue };
            let Some((model, names)) = rest.split_once(":**") else { continue };
            let mut rows = Vec::new();
            for (index, piece) in names.split('`').enumerate() {
                if index % 2 == 1 {
                    rows.push((piece.to_string(), String::new()));
                } else if let Some(last) = rows.last_mut() {
                    let words = piece.trim().trim_start_matches(',').trim();
                    if words.starts_with('(') {
                        last.1 = words.split(')').next().unwrap_or("").trim_start_matches('(').to_string();
                    }
                }
            }
            table.insert(model.trim().to_string(), rows);
        }
        table
    })
}

/// Words in a refusal's sentence or name that tie it to the edge of data
/// rather than to a mathematical or physical limit (engine rule 1).
pub const DATA_WORDS: &[&str] = &["compared", "comparison", "measured", "measurement", "scored", "held-back", "held back", "fitted", "fitting span"];
pub const DATA_NAMES: &[&str] = &["Compared", "Comparison", "Measured", "Fitted", "Scored"];

/// The names of a model's own inputs, from its description: a card's
/// `phase_fields` and `load_fields`, a solver's `inputs`. An input's own name
/// is not the edge of data: `measured`, a frame's yes or no, is a field.
/// `describe_card` serves each list as an array of fields, each with its
/// `name`; a list keyed by name is read as well.
pub fn input_names(description: &Value) -> Vec<String> {
    let mut names = Vec::new();
    for key in ["phase_fields", "load_fields"] {
        match description.get(key) {
            Some(Value::Object(fields)) => names.extend(fields.keys().cloned()),
            Some(Value::Array(fields)) => {
                names.extend(fields.iter().filter_map(|field| field.get("name").and_then(Value::as_str)).map(str::to_string))
            }
            _ => {}
        }
    }
    for input in description.get("inputs").and_then(Value::as_array).into_iter().flatten() {
        if let Some(name) = input.get("name").and_then(Value::as_str) {
            names.push(name.to_string());
        }
    }
    names
}

/// `measured_frames` as a refusal name spells it: `MeasuredFrames`.
fn camel(name: &str) -> String {
    name.split('_').filter(|part| !part.is_empty()).map(|part| {
        let mut chars = part.chars();
        chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
    }).collect()
}

/// A refusal's name and meaning with the model's own input names taken out:
/// each input as the meaning quotes it, `name`, and as a name that opens
/// with it spells it (`MeasuredNotZeroOrOne`, of the input `measured`).
fn without_inputs(name: &str, meaning: &str, inputs: &[String]) -> (String, String) {
    let mut name = name.to_string();
    let mut meaning = meaning.to_string();
    for input in inputs {
        meaning = meaning.replace(&format!("`{input}`"), "`an input`");
        let spelled = camel(input);
        if let Some(rest) = name.strip_prefix(spelled.as_str()).filter(|_| !spelled.is_empty()) {
            name = format!("Input{rest}");
        }
    }
    (name, meaning)
}

/// A range in plain words: a digit, at most a short unit, "to" and a digit.
pub fn has_range_in_numbers(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(" to ") {
        let at = from + at;
        let before = &text[at.saturating_sub(14)..at];
        let after = &bytes[at + 4..(at + 7).min(bytes.len())];
        if before.chars().any(|c| c.is_ascii_digit()) && after.iter().any(|b| b.is_ascii_digit()) {
            return true;
        }
        from = at + 4;
    }
    false
}

/// The rules on a description's words, for a model whose list row gives
/// `verdict`. `rejects` are the refusal rows: each `name` and `meaning`.
pub fn text_findings(model: &str, description: &Value, verdict: &str, rejects: &[(String, String)]) -> Vec<Finding> {
    let mut out = Out::new(model);
    let mut all = Vec::new();
    strings(description, "", &mut all);
    let mut checks = Vec::new();
    let mut notation = Vec::new();
    let mut strong = Vec::new();
    let mut shared = Vec::new();
    let mut weak = Vec::new();
    let mut old = Vec::new();
    for (path, text) in &all {
        for at in word_starts(text, "check") {
            checks.push(format!("{path}: {}", around(text, at, 40)));
        }
        for mark in ["..=", "..<"] {
            if let Some(at) = text.find(mark) {
                notation.push(format!("{path}: {}", around(text, at, 30)));
            }
        }
        for phrase in LACKS_STRONG {
            for at in word_starts(text, phrase) {
                // The short form's verdict phrase is the gateway's, shared by
                // every model of its kind: a note on each, not the model's own.
                if path.starts_with(".(short form)") {
                    shared.push(format!("{path}: {}", around(text, at, 50)));
                } else {
                    strong.push(format!("{path}: {}", around(text, at, 50)));
                }
            }
        }
        for phrase in LACKS_WEAK {
            for at in word_starts(text, phrase) {
                if !about_a_fit(&text.to_lowercase(), at) {
                    weak.push(format!("{path}: {}", around(text, at, 50)));
                }
            }
        }
        if let Some(at) = text.find("verify_step").filter(|_| !path.ends_with("(key)")) {
            old.push(format!("{path}: {}", around(text, at, 30)));
        }
    }
    let shown = |list: &[String]| -> String {
        let mut text = list.iter().take(3).cloned().collect::<Vec<_>>().join(" | ");
        if list.len() > 3 {
            text.push_str(&format!(" | and {} more", list.len() - 3));
        }
        text
    };
    if !checks.is_empty() {
        out.contract("no check", format!("{} uses of \"check\": {}", checks.len(), shown(&checks)));
    }
    if !notation.is_empty() {
        out.contract("ranges in plain words", format!("{} ranges in program notation: {}", notation.len(), shown(&notation)));
    }
    if !strong.is_empty() {
        out.contract("not described by what it lacks", format!("{} lines: {}", strong.len(), shown(&strong)));
    }
    if !shared.is_empty() {
        out.note("not described by what it lacks", format!("the gateway's shared verdict phrase, in the short form: {}", shown(&shared)));
    }
    if !weak.is_empty() {
        out.note("not described by what it lacks", format!("{} lines to look at: {}", weak.len(), shown(&weak)));
    }
    if !old.is_empty() {
        out.note("words of 0.11.0", format!("{} mentions of verify_step, served as compute_step from 0.11.0: {}", old.len(), shown(&old)));
    }
    // A refusal is a mathematical or physical limit, never the edge of data.
    let table = lifts();
    let lifted = table.get(model);
    let inputs = input_names(description);
    for (name, meaning) in rejects {
        let (bare_name, bare_meaning) = without_inputs(name, meaning, &inputs);
        let lower = bare_meaning.to_lowercase();
        let by_word = DATA_WORDS.iter().find(|word| lower.contains(*word));
        let by_name = DATA_NAMES.iter().find(|word| bare_name.contains(*word));
        let heuristic = !ledger_refusal(name) && (by_word.is_some() || by_name.is_some());
        let lift = lifted.and_then(|rows| rows.iter().find(|(row, _)| row == name));
        let side = |side: &String| if side.is_empty() { String::new() } else { format!(" ({side})") };
        match (lift, heuristic) {
            (Some((_, row)), true) => out.contract(
                "a refusal is a physical limit",
                format!("{name} is a LIFT row of REFUSALS.md{}, still served, and its sentence still names data: {meaning}", side(row)),
            ),
            (Some((_, row)), false) => out.note(
                "a refusal is a physical limit",
                format!("{name} is a LIFT row of REFUSALS.md{} and is still served, its sentence naming no data: lifted to a physical limit under the same name, or to read: {meaning}", side(row)),
            ),
            (None, true) if !table.is_empty() => out.note(
                "a refusal is a physical limit",
                format!("{name} is tied to data by its words ({}), and REFUSALS.md has it as no LIFT: {meaning}", by_name.copied().or(by_word.copied()).unwrap_or("")),
            ),
            (None, true) => out.contract(
                "a refusal is a physical limit",
                format!("{name} is tied to the edge of data ({}): {meaning}", by_name.copied().or(by_word.copied()).unwrap_or("")),
            ),
            (None, false) => {}
        }
    }
    // A model compared with measurements states the range it was compared on,
    // blind or calibrated to the record.
    if BLIND_WORDS.contains(&verdict) || verdict == CALIBRATED {
        let record = description.get("validation").map(Value::to_string).unwrap_or_default();
        if record.is_empty() {
            out.contract("compared range in numbers", format!("the list says {verdict}, and the description has no validation record"));
        } else if !has_range_in_numbers(&record) {
            out.contract("compared range in numbers", "the validation record states no range in numbers (\"a to b\")".to_string());
        }
    }
    out.found
}

// ----- forecast first ---------------------------------------------------------

/// A time as Git (`%cI`) or a registration writes it, ISO 8601, in seconds
/// since 1970 in UTC; a date alone is the start of its day, and marked so.
pub fn utc_seconds(text: &str) -> Option<(i64, bool)> {
    let number = |range: std::ops::Range<usize>| text.get(range).and_then(|part| part.parse::<i64>().ok());
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    if text.get(4..5) != Some("-") || text.get(7..8) != Some("-") {
        return None;
    }
    // Days from 1970-01-01 to the date, by the civil-calendar rule.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    if text.len() == 10 {
        return Some((days * 86_400, true));
    }
    if text.get(10..11) != Some("T") {
        return None;
    }
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let mut rest = &text[19..];
    if let Some(fraction) = rest.strip_prefix('.') {
        rest = fraction.trim_start_matches(|c: char| c.is_ascii_digit());
    }
    let offset = match rest {
        "Z" | "+00:00" => 0,
        zone if zone.len() == 6 => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            sign * (zone[1..3].parse::<i64>().ok()? * 3600 + zone[4..6].parse::<i64>().ok()? * 60)
        }
        _ => return None,
    };
    Some((days * 86_400 + hour * 3600 + minute * 60 + second - offset, false))
}

fn git(dir: &std::path::Path, args: &[&str]) -> Option<String> {
    let done = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    done.status.success().then(|| String::from_utf8_lossy(&done.stdout).trim().to_string())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The folder above the gateway crate that holds `evidence/research`: the
/// Spectrum folder a registration's paths are relative to.
fn spectrum_root() -> Option<std::path::PathBuf> {
    let mut here = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    loop {
        if here.join("evidence").join("research").is_dir() {
            return Some(here);
        }
        if !here.pop() {
            return None;
        }
    }
}

/// The word of a model calibrated to the record.
pub const CALIBRATED: &str = "calibrated to the record";
/// The words of a comparison made blind, each held to forecast first.
pub const BLIND_WORDS: [&str; 3] = ["met", "missed", "met in part"];
/// What the served description of a model calibrated to the record says of
/// its comparison.
pub const NOT_A_BLIND_TEST: &str = "not a blind test";

fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The registration of `model` in the kit's registry, and its text.
fn registration(model: &str) -> (String, Option<String>) {
    let gateway = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let relative = format!("kit/forecasts/{model}.json");
    let text = std::fs::read_to_string(gateway.join(&relative)).ok();
    (relative, text)
}

/// Forecast first. A model whose list row says met, missed or met in part
/// was compared with measurements; its protocol (the pass marks and the
/// forecast) must have been fixed, fingerprinted and committed before its
/// scoring. The kit's registry, `kit/forecasts/<model>.json` (written and
/// committed by `kit/forecast.py`, or backfilled from an older record),
/// says so, and Git's record is read for the order: the commit that carried
/// the protocol's SHA-256 or the protocol itself (`protocol.committed`), or
/// the registration's own first commit. A compared model fails when it has
/// no registration, when the registration does not show a committed
/// fingerprint older than its scoring, when a held-back file was opened
/// before the protocol was fixed, or when the protocol on disk is not the
/// one fingerprinted.
///
/// A model calibrated to the record (its row says so, or its registration
/// carries a `calibration`) is held to `calibration_findings` instead, and
/// to forecast first only for a blind registration in the same record.
/// `description` is the model's full description and `served` the name of
/// every served model.
pub fn forecast_findings(model: &str, verdict: &str, description: &Value, served: &[String]) -> Vec<Finding> {
    let mut out = Out::new(model);
    let (relative, text) = registration(model);
    let record = text.as_deref().and_then(|text| serde_json::from_str::<Value>(text).ok());
    if verdict == CALIBRATED || record.as_ref().is_some_and(|record| !record["calibration"].is_null()) {
        return calibration_findings(model, verdict, record.as_ref(), description, served);
    }
    if !BLIND_WORDS.contains(&verdict) {
        return out.found;
    }
    let Some(text) = text else {
        out.contract(FORECAST_FIRST, format!("the list says {verdict}, and {relative} is missing: no committed, fingerprinted protocol older than its scoring"));
        return out.found;
    };
    let Ok(record) = serde_json::from_str::<Value>(&text) else {
        out.contract(FORECAST_FIRST, format!("{relative} is not JSON"));
        return out.found;
    };
    blind_findings(&mut out, verdict, &relative, &record, true, false);
    out.found
}

const FORECAST_FIRST: &str = "forecast first";

/// A blind registration, `record`'s top level, held to forecast first. A
/// scoring is required where the list gives the scoring's word; a blind
/// registration beside a calibration may be pending. In a record that also
/// carries a calibration the protocol was fixed by the first commit of the
/// registration that carries its SHA-256, which a calibration recorded
/// before it does not.
fn blind_findings(out: &mut Out, verdict: &str, relative: &str, record: &Value, scoring_required: bool, calibrated: bool) {
    const RULE: &str = FORECAST_FIRST;
    let gateway = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sha256 = record["protocol"]["sha256"].as_str().unwrap_or("");
    if !is_sha256(sha256) {
        out.contract(RULE, format!("{relative} names no SHA-256 of the protocol"));
        return;
    }
    // When the protocol was fixed, from Git.
    let fixed = match record["protocol"]["committed"].as_object() {
        Some(committed) => {
            let crate_name = committed.get("crate").and_then(Value::as_str).unwrap_or("");
            let commit = committed.get("commit").and_then(Value::as_str).unwrap_or("");
            let crate_dir = gateway.join("..").join(crate_name);
            match git(&crate_dir, &["show", "-s", "--format=%cI", commit]) {
                None => {
                    out.contract(RULE, format!("the commit {commit} that is to carry the protocol is not in {crate_name}'s Git"));
                    return;
                }
                Some(when) => {
                    let shown = git(&crate_dir, &["show", "--format=%B", commit]).unwrap_or_default();
                    let adds = || {
                        git(&crate_dir, &["diff-tree", "-r", "--no-commit-id", "--name-only", "--root", commit])
                            .unwrap_or_default()
                            .lines()
                            .any(|name| {
                                Command::new("git").arg("-C").arg(&crate_dir).args(["show", &format!("{commit}:{name}")]).output()
                                    .map(|body| sha256_hex(&body.stdout) == sha256).unwrap_or(false)
                            })
                    };
                    // A pre-registration that fingerprints several files at
                    // once: the commit carries the manifest's fingerprint, the
                    // SHA-256 of its list of file hashes, one of them the
                    // protocol's, as json.dumps(list, sort_keys=True) writes it.
                    let manifest = || -> Option<String> {
                        let named = committed.get("manifest")?;
                        let text = std::fs::read_to_string(spectrum_root()?.join(named.get("path")?.as_str()?)).ok()?;
                        let manifest: Value = serde_json::from_str(&text).ok()?;
                        let list = manifest.get(named.get("list")?.as_str()?)?.as_object()?;
                        if !list.values().any(|hash| hash.as_str() == Some(sha256)) {
                            return None;
                        }
                        let mut pairs: Vec<(&String, &Value)> = list.iter().collect();
                        pairs.sort_by(|a, b| a.0.cmp(b.0));
                        let written = pairs.iter().map(|(file, hash)| format!("{}: {}", Value::String((*file).clone()), hash)).collect::<Vec<_>>().join(", ");
                        let fingerprint = sha256_hex(format!("{{{written}}}").as_bytes());
                        (manifest.get(named.get("fingerprint")?.as_str()?)?.as_str() == Some(fingerprint.as_str())).then_some(fingerprint)
                    };
                    let carried = manifest().filter(|fingerprint| shown.contains(&fingerprint[..8]));
                    if !shown.contains(&sha256[..8]) && carried.is_none() && !adds() {
                        out.contract(RULE, format!("the commit {commit} of {crate_name} carries neither the protocol's SHA-256 nor the protocol, nor a manifest's fingerprint that holds it"));
                        return;
                    }
                    utc_seconds(&when).map(|(seconds, _)| (seconds, when))
                }
            }
        }
        // A registration backfilled from an older record with no commit of
        // the protocol: its own commit is of today, and fixes nothing.
        None if record["registered_by"].as_str().unwrap_or("").contains("backfill") => {
            out.contract(RULE, format!("the list says {verdict}, and no commit carries the protocol's SHA-256 or the protocol: the record {relative} was backfilled from names none before the scoring at {}", record["scored"]["utc"].as_str().unwrap_or("an unstated time")));
            return;
        }
        // Beside a calibration, the first commit of the registration whose
        // record carries this protocol's SHA-256.
        None if calibrated => git(gateway, &["log", "--format=%H %cI", "--", relative]).and_then(|log| {
            log.lines().rev().find_map(|line| {
                let (commit, when) = line.split_once(' ')?;
                let body = git(gateway, &["show", &format!("{commit}:{relative}")])?;
                let then: Value = serde_json::from_str(&body).ok()?;
                (then["protocol"]["sha256"].as_str() == Some(sha256)).then(|| utc_seconds(when).map(|(seconds, _)| (seconds, when.to_string()))).flatten()
            })
        }),
        None => git(gateway, &["log", "--format=%cI", "--", relative])
            .and_then(|log| log.lines().last().map(str::to_string))
            .and_then(|when| utc_seconds(&when).map(|(seconds, _)| (seconds, when))),
    };
    let Some((fixed, fixed_text)) = fixed else {
        out.contract(RULE, format!("{relative} is not committed: the protocol's fingerprint is in no commit"));
        return;
    };
    // The protocol on disk is the one fingerprinted.
    let path = record["protocol"]["path"].as_str().unwrap_or("");
    match spectrum_root().map(|root| root.join(path)).filter(|full| full.is_file()) {
        Some(full) => {
            let on_disk = sha256_hex(&std::fs::read(&full).unwrap_or_default());
            if on_disk != sha256 {
                out.contract(RULE, format!("the protocol on disk, {path}, is not the one fingerprinted ({on_disk}, against {sha256})"));
            }
        }
        None => out.note(RULE, format!("the protocol {path} is not on this disk, so its fingerprint is not compared")),
    }
    for opened in record["opened"].as_array().into_iter().flatten() {
        let when = opened["utc"].as_str().unwrap_or("");
        if !utc_seconds(when).is_some_and(|(seconds, _)| seconds > fixed) {
            out.contract(RULE, format!("{} was opened at {when}, not after the protocol was fixed at {fixed_text}", opened["path"].as_str().unwrap_or("a held-back file")));
        }
    }
    if !scoring_required && record["scored"].is_null() {
        return;
    }
    let scored = record["scored"]["utc"].as_str().unwrap_or("");
    match utc_seconds(scored) {
        None => out.contract(RULE, format!("the list says {verdict}, and {relative} gives no time of the scoring")),
        Some((seconds, true)) if seconds == fixed - fixed.rem_euclid(86_400) => out.contract(
            RULE,
            format!("the protocol was fixed at {fixed_text} and the model scored on the same day, {scored}, and the record gives no time to order them"),
        ),
        Some((seconds, _)) if seconds <= fixed => {
            out.contract(RULE, format!("the protocol was fixed at {fixed_text}, not before the scoring at {scored}"))
        }
        Some(_) => {}
    }
}

/// Calibrated to the record. A model whose structure or constants were
/// chosen with the record it is compared with open is served under the
/// sixth word, beside the model compared blind, and its registration's
/// `calibration` (written by `kit/forecast.py calibrate`) holds:
///
/// 1. `companion_to` names a served model, not itself, whose own record has
///    been scored (`scored.utc`);
/// 2. `calibration.protocol.sha256` is 64 hex characters and is the
///    SHA-256 of the file on disk (on a disk with the company's records);
/// 3. `calibration.scored` gives the time of the comparison, with the
///    output's and the program's SHA-256;
/// 4. the list's word is `calibrated to the record`, and the served
///    description's validation result says it is not a blind test;
/// 5. a blind registration in the same record is held to every rule of
///    forecast first.
///
/// The word is `calibrated to the record` until that blind registration is
/// scored; then it is the scoring's, met, met in part or missed, as the
/// record's `verdict` states it, and forecast first holds the whole of it.
pub fn calibration_findings(model: &str, verdict: &str, record: Option<&Value>, description: &Value, served: &[String]) -> Vec<Finding> {
    const RULE: &str = CALIBRATED;
    let mut out = Out::new(model);
    let relative = format!("kit/forecasts/{model}.json");
    let Some(record) = record else {
        out.contract(RULE, format!("the list says {verdict}, and {relative} is missing or not JSON: no calibration names its companion, the fingerprint of the record it is compared with and its scoring"));
        return out.found;
    };
    let calibration = &record["calibration"];
    if !calibration.is_object() {
        out.contract(RULE, format!("the list says {verdict}, and {relative} carries no calibration: none names its companion, the fingerprint of the record it is compared with and its scoring"));
    } else {
        // 1. The companion: served, not itself, its own record scored.
        let companion = calibration["companion_to"].as_str().unwrap_or("");
        if companion.is_empty() || companion == model {
            out.contract(RULE, "calibration.companion_to names no served model beside it".to_string());
        } else {
            if !served.iter().any(|name| name == companion) {
                out.contract(RULE, format!("calibration.companion_to, {companion}, is not a served model"));
            }
            let (companion_relative, text) = registration(companion);
            let scored = text.as_deref().and_then(|text| serde_json::from_str::<Value>(text).ok()).is_some_and(|own| own["scored"]["utc"].is_string());
            if !scored {
                out.contract(RULE, format!("calibration.companion_to, {companion}: its own record, {companion_relative}, is not scored (no scored.utc)"));
            }
        }
        // 2. The fingerprint of the record it is compared with.
        let sha256 = calibration["protocol"]["sha256"].as_str().unwrap_or("");
        let path = calibration["protocol"]["path"].as_str().unwrap_or("");
        if !is_sha256(sha256) {
            out.contract(RULE, "calibration.protocol.sha256 is not 64 hex characters".to_string());
        } else {
            match spectrum_root() {
                None => out.note(RULE, format!("calibration.protocol, {path}: the company's records are not on this disk, so its fingerprint is not compared")),
                Some(root) => match std::fs::read(root.join(path)) {
                    Err(_) => out.contract(RULE, format!("calibration.protocol, {path}, is not on this disk, so it is not the file fingerprinted")),
                    Ok(bytes) => {
                        let on_disk = sha256_hex(&bytes);
                        if on_disk != sha256 {
                            out.contract(RULE, format!("calibration.protocol on disk, {path}, is not the one fingerprinted ({on_disk}, against {sha256})"));
                        }
                    }
                },
            }
        }
        // 3. The comparison's scoring: its time, the output's and the program's SHA-256.
        if utc_seconds(calibration["scored"]["utc"].as_str().unwrap_or("")).is_none() {
            out.contract(RULE, "calibration.scored gives no time of the comparison".to_string());
        }
        for part in ["output", "program"] {
            if !is_sha256(calibration["scored"][part]["sha256"].as_str().unwrap_or("")) {
                out.contract(RULE, format!("calibration.scored.{part} gives no SHA-256"));
            }
        }
    }
    // 4, and the word: calibrated to the record until a blind registration
    // in the same record is scored, then that scoring's.
    let blind = record["protocol"].is_object();
    let scored = blind && !record["scored"].is_null();
    let stated = record["verdict"].as_str();
    if scored {
        match stated.filter(|word| BLIND_WORDS.contains(word)) {
            None => out.contract(RULE, format!("its blind registration is scored, and {relative} gives no verdict of that scoring (met, met in part or missed)")),
            Some(word) if word != verdict => out.contract(RULE, format!("its blind registration is scored, {word}, and the list says {verdict}: the word served is that scoring's")),
            Some(_) => {}
        }
    } else {
        if let Some(word) = stated.filter(|word| *word != CALIBRATED) {
            out.contract(RULE, format!("{relative} gives the verdict {word}, and no blind registration of it is scored: until one is, the verdict is calibrated to the record"));
        }
        if verdict != CALIBRATED {
            out.contract(RULE, format!("the list says {verdict}, and no blind registration of it is scored: until one is, the word served is calibrated to the record"));
        } else if !description["validation"]["result"].as_str().unwrap_or("").contains(NOT_A_BLIND_TEST) {
            out.contract(RULE, "the served description's validation result does not say it is not a blind test".to_string());
        }
    }
    // 5. A blind registration beside it keeps every rule of forecast first.
    if blind {
        blind_findings(&mut out, verdict, &relative, record, scored, true);
    }
    out.found
}

/// The name of every served model, from `list_cards` and `list_solvers`.
/// Writes a sweep's findings as JSON lines to the file `CONFORMANCE_REPORT`
/// names, when it is set, after the lines already in it: the card sweep and
/// the solver sweep run in one command and are handed one file, so each adds
/// its own lines and neither writes over the other's. A sweep starts from no
/// file, or from one it means to add to.
pub fn report(lines: &[String]) {
    use std::io::Write;
    let Ok(path) = std::env::var("CONFORMANCE_REPORT") else {
        return;
    };
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).expect("the report is opened");
    for line in lines {
        writeln!(file, "{line}").expect("the report is written");
    }
}

pub fn served_names(door: &mut Door) -> Vec<String> {
    verdicts(door).into_iter().map(|(_, name, _)| name).collect()
}

/// The words of the refusal sentences a model gave: a sentence is served
/// too, and holds to the same words as a description.
fn sentence_findings(out: &mut Out, sentences: &[String]) {
    let mut notation = Vec::new();
    let mut checks = Vec::new();
    for sentence in sentences {
        for mark in ["..=", "..<"] {
            if let Some(at) = sentence.find(mark) {
                notation.push(around(sentence, at, 40));
            }
        }
        for at in word_starts(sentence, "check") {
            checks.push(around(sentence, at, 40));
        }
    }
    notation.dedup();
    checks.dedup();
    if !notation.is_empty() {
        out.contract("ranges in plain words", format!("{} refusal sentences give a range in program notation: {}", notation.len(), notation.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")));
    }
    if !checks.is_empty() {
        out.contract("no check", format!("{} refusal sentences say \"check\": {}", checks.len(), checks.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")));
    }
    // A sentence served at a refusal that speaks of data (a range compared,
    // measured or fitted), for a look: the meaning in the description is held
    // to the rule (`text_findings`); the sentence the caller reads is given
    // here, its quoted field paths taken out.
    let mut data = Vec::new();
    for sentence in sentences {
        let bare: String = sentence.split('\'').enumerate().filter(|(index, _)| index % 2 == 0).map(|(_, part)| part).collect::<Vec<_>>().join("");
        let lower = bare.to_lowercase();
        if let Some(word) = DATA_WORDS.iter().find(|word| lower.contains(*word)) {
            data.push(format!("({word}) {}", around(sentence, sentence.to_lowercase().find(word).unwrap_or(0), 60)));
        }
    }
    data.dedup();
    if !data.is_empty() {
        out.note("a refusal is a physical limit", format!("{} refusal sentences speak of data: {}", data.len(), data.iter().take(3).cloned().collect::<Vec<_>>().join(" | ")));
    }
}

// ----- a refusal's name -------------------------------------------------------

/// The name a sentence gives in round brackets, the last one: a word that
/// begins with a capital and holds no space.
pub fn name_in(sentence: &str) -> Option<String> {
    let mut found = None;
    let mut rest = sentence;
    while let Some(open) = rest.find('(') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(')') else { break };
        let inner = &after[..close];
        if inner.len() > 3
            && inner.chars().next().is_some_and(|first| first.is_ascii_uppercase())
            && inner.chars().all(|letter| letter.is_ascii_alphanumeric())
            && inner.chars().any(|letter| letter.is_ascii_lowercase())
        {
            found = Some(inner.to_string());
        }
        rest = &after[close + 1..];
    }
    found
}

fn below(x: f64) -> f64 {
    if x == 0.0 { -f64::MIN_POSITIVE } else if x > 0.0 { f64::from_bits(x.to_bits() - 1) } else { -f64::from_bits((-x).to_bits() + 1) }
}

fn above(x: f64) -> f64 {
    if x == 0.0 { f64::MIN_POSITIVE } else if x > 0.0 { f64::from_bits(x.to_bits() + 1) } else { -f64::from_bits((-x).to_bits() - 1) }
}

/// What a request came back as.
enum Came {
    /// Turned away before computing, with its sentence.
    Stopped(String),
    /// Answered, with the outcome, the steps that landed and the name of a stop part way.
    Answered { outcome: String, landed: u64, reject: Option<String>, body: Value },
}

fn came(door: &mut Door, tool: &str, request: &Value) -> Came {
    let (error, text) = door.call(tool, request);
    if error {
        return Came::Stopped(text);
    }
    let body: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    Came::Answered {
        outcome: body["outcome"].as_str().unwrap_or("").to_string(),
        landed: body["steps_landed"].as_u64().unwrap_or(0),
        reject: body["reject"].as_str().map(str::to_string),
        body,
    }
}

// ----- a card -------------------------------------------------------------------

/// The verdict word of every model, from `list_cards` and `list_solvers`.
pub fn verdicts(door: &mut Door) -> Vec<(String, String, String)> {
    let mut rows = Vec::new();
    if let Some(list) = door.value("list_cards", &json!({})) {
        for row in list["cards"].as_array().into_iter().flatten() {
            rows.push((row["domain"].as_str().unwrap_or("").to_string(), row["card"].as_str().unwrap_or("").to_string(), row["against_measurement"].as_str().unwrap_or("").to_string()));
        }
    }
    if let Some(list) = door.value("list_solvers", &json!({})) {
        for row in list["solvers"].as_array().into_iter().flatten() {
            rows.push((String::new(), row["solver"].as_str().unwrap_or("").to_string(), row["against_measurement"].as_str().unwrap_or("").to_string()));
        }
    }
    rows
}

fn names_of(rows: &Value, key: &str) -> Vec<(String, String)> {
    rows[key]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| (row["name"].as_str().unwrap_or("").to_string(), row["meaning"].as_str().unwrap_or("").to_string()))
        .collect()
}

/// The worked run of a card cut to `steps` steps of one segment, each recorded.
fn worked_run(description: &Value, steps: u64) -> Value {
    let mut run = description["example_run_trajectory"].clone();
    let first = run["segments"][0].clone();
    run["segments"] = json!([first]);
    let given = run["segments"][0]["steps"].as_u64().unwrap_or(1);
    run["segments"][0]["steps"] = json!(given.min(steps));
    run["record_every"] = json!(1);
    run
}

/// Every attack on one card.
pub fn card_findings(door: &mut Door, domain: &str, card: &str, verdict: &str) -> Vec<Finding> {
    let mut out = Out::new(card);
    let named = json!({ "domain": domain, "card": card });
    let Some(description) = door.value("describe_card", &named) else {
        out.contract("described", "describe_card gave no description".to_string());
        return out.found;
    };
    let rejects = names_of(&description, "rejects");
    let listed: BTreeSet<String> = rejects.iter().map(|(name, _)| name.clone()).collect();
    let short = door.value("describe_card", &json!({ "domain": domain, "card": card, "form": "short" }));
    let mut served = description.clone();
    served["(short form)"] = short.clone().map(|short| json!({ "accuracy_against_measurement": short["accuracy_against_measurement"], "full_form": short["full_form"] })).unwrap_or(Value::Null);
    out.found.extend(text_findings(card, &served, verdict, &rejects));
    let names = served_names(door);
    out.found.extend(forecast_findings(card, verdict, &description, &names));
    match short {
        Some(short) => {
            let stops: BTreeSet<String> = short["stops"].as_array().into_iter().flatten().filter_map(|n| n.as_str().map(str::to_string)).collect();
            if stops != listed {
                out.contract("refusal names listed", format!("the short form's stops are not the full form's names: {:?}", stops.symmetric_difference(&listed).collect::<Vec<_>>()));
            }
        }
        None => out.contract("described", "describe_card with form short gave no description".to_string()),
    }
    let ends: Vec<String> = description["declared_ends"].as_array().into_iter().flatten().filter_map(|row| row["reject"].as_str().map(str::to_string)).collect();
    for end in &ends {
        if !listed.contains(end) {
            out.contract("refusal names listed", format!("the declared end {end} is not among the card's names"));
        }
    }
    let mut seen_names = BTreeSet::new();
    let mut sentences: Vec<String> = Vec::new();
    let run = worked_run(&description, 1);

    // Every declared bound, one number past it.
    for (section, list) in [("initial", "phase_fields"), ("load", "load_fields")] {
        for field in description[list].as_array().into_iter().flatten() {
            let name = field["name"].as_str().unwrap_or("").to_string();
            let vector = field["components"] != json!(1);
            let mut past = Vec::new();
            if let Some(minimum) = field["minimum"].as_f64() {
                past.push(("minimum", minimum, if field["minimum_exclusive"] == json!(true) { minimum } else { below(minimum) }));
            }
            if let Some(maximum) = field["maximum"].as_f64() {
                past.push(("maximum", maximum, if field["maximum_exclusive"] == json!(true) { maximum } else { above(maximum) }));
            }
            for (which, bound, value) in past {
                if value != 0.0 && !value.is_normal() {
                    continue;
                }
                let mut request = run.clone();
                let target = if section == "initial" { &mut request["initial"] } else { &mut request["segments"][0]["load"] };
                target[&name] = if vector { json!([value, 0.0, 0.0]) } else { json!(value) };
                match came(door, "run_trajectory", &request) {
                    Came::Stopped(sentence) => {
                        match name_in(&sentence) {
                            Some(given) if listed.contains(&given) => {
                                seen_names.insert(given);
                            }
                            Some(given) => out.contract("refusal names listed", format!("{name} = {value:e} past its {which} {bound:e} is refused as {given}, which the card does not list")),
                            None => out.contract("limits refused by name", format!("{name} = {value:e} past its {which} {bound:e} is refused with no name: {sentence}")),
                        }
                        sentences.push(sentence);
                    }
                    Came::Answered { outcome, landed, reject, .. } => {
                        if landed > 0 {
                            out.contract("limits refused by name", format!("{name} = {value:e}, past its declared {which} {bound:e}, was answered ({outcome}, {landed} step landed)"));
                        } else if let Some(reject) = reject {
                            seen_names.insert(reject);
                        }
                    }
                }
            }
            // A field left out is refused, never filled in.
            let mut request = run.clone();
            let target = if section == "initial" { &mut request["initial"] } else { &mut request["segments"][0]["load"] };
            if let Some(map) = target.as_object_mut() {
                if map.remove(&name).is_some() {
                    if let Came::Answered { outcome, .. } = came(door, "run_trajectory", &request) {
                        out.contract("nothing filled in", format!("a run with {section} field {name} left out was answered ({outcome})"));
                    }
                }
            }
        }
        // A key the card does not take: the caller never sets the specimen.
        let mut request = run.clone();
        let target = if section == "initial" { &mut request["initial"] } else { &mut request["segments"][0]["load"] };
        if target.is_object() {
            target["specimen_constant"] = json!(1.0);
            if let Came::Answered { outcome, .. } = came(door, "run_trajectory", &request) {
                out.contract("fixed constants", format!("a run with an unknown {section} key was answered ({outcome})"));
            }
        }
    }

    // The step floor and ceiling.
    let floor = description["dt_s"]["floor_s"].as_f64();
    let ceiling = description["dt_s"]["ceiling_s"].as_f64();
    match (floor, ceiling) {
        (Some(floor), Some(ceiling)) if floor > 0.0 && floor <= ceiling && ceiling.is_finite() => {
            for (dt_s, what) in [(below(floor), "under the floor"), (above(ceiling), "over the ceiling")] {
                let mut request = run.clone();
                request["dt_s"] = json!(dt_s);
                match came(door, "run_trajectory", &request) {
                    Came::Stopped(sentence) => match name_in(&sentence) {
                        Some(given) if listed.contains(&given) => {}
                        other => out.contract("step ceiling and floor", format!("a step {what} ({dt_s:e} s) is refused as {other:?}, a name the card does not list: {sentence}")),
                    },
                    Came::Answered { outcome, .. } => out.contract("step ceiling and floor", format!("a step {what} ({dt_s:e} s) was answered ({outcome})")),
                }
            }
            for (dt_s, what) in [(floor, "floor"), (ceiling, "ceiling")] {
                let mut request = run.clone();
                request["dt_s"] = json!(dt_s);
                if let Came::Stopped(sentence) = came(door, "run_trajectory", &request) {
                    if sentence.contains("'dt_s'") {
                        out.contract("step ceiling and floor", format!("a step of the {what} itself ({dt_s:e} s) is refused for its size: {sentence}"));
                    }
                }
            }
        }
        other => out.contract("step ceiling and floor", format!("the description declares no floor under a finite ceiling: {other:?}")),
    }

    sentence_findings(&mut out, &sentences);

    // The worked run: the ledger at every frame, the same bytes, ends.
    let worked = worked_run(&description, 200);
    let (error, first) = door.call("run_trajectory", &worked);
    if error {
        out.contract("worked example runs", format!("the worked run cut to 200 steps is refused: {first}"));
    } else {
        let body: Value = serde_json::from_str(&first).unwrap_or(Value::Null);
        let largest = body["max_residual_conservation_error"].as_f64().unwrap_or(f64::NAN);
        if !(largest.is_finite() && largest <= 1.0e-6) {
            out.contract("ledger held at every step", format!("the worked run's largest residual is {largest:e}"));
        }
        for frame in body["frames"].as_array().into_iter().flatten() {
            let hex = frame["frame_hex"].as_str().unwrap_or("");
            if hex.len() == 128 {
                let bytes: Vec<u8> = (0..64).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0)).collect();
                let residual = f32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]);
                if !residual.is_finite() || f64::from(residual) > 1.0e-6 {
                    out.contract("ledger held at every step", format!("frame {} carries residual {residual:e}", frame["step"]));
                    break;
                }
            }
        }
        if body["outcome"] == "ended" {
            let reject = body["reject"].as_str().unwrap_or("");
            if !ends.iter().any(|end| end == reject) {
                out.contract("an end is the event finishing", format!("the worked run ended as {reject}, which the card does not declare an end"));
            }
        }
        if let Some(reject) = body["reject"].as_str() {
            if !listed.contains(reject) {
                out.contract("refusal names listed", format!("the worked run stopped as {reject}, which the card does not list"));
            }
        }
        let (_, again) = door.call("run_trajectory", &worked);
        let mut second_door = Door::start();
        let (_, elsewhere) = second_door.call("run_trajectory", &worked);
        if again != first || elsewhere != first {
            out.contract("same input, same bits", "the worked run gave different bytes on a second call or a second connection".to_string());
        }
        // From 0.11.0 the worked step is example_compute_step, sent to compute_step.
        let (tool, step) = match description.get("example_compute_step") {
            Some(step) => ("compute_step", step.clone()),
            None => ("verify_step", description["example_verify_step"].clone()),
        };
        let (_, one) = door.call(tool, &step);
        let (_, other) = second_door.call(tool, &step);
        if one != other {
            out.contract("same input, same bits", "the worked step gave different bytes on a second connection".to_string());
        }
    }
    out.found
}

// ----- a solver -----------------------------------------------------------------

/// The whole description of a solver: its parts given only when asked for
/// are fetched and set in.
fn whole_solver(door: &mut Door, solver: &str) -> Option<Value> {
    let mut description = door.value("describe_solver", &json!({ "solver": solver }))?;
    if let Some(more) = description.get("more").and_then(Value::as_object).cloned() {
        for key in more.keys().filter(|key| *key != "how") {
            if let Some(part) = door.value("describe_solver", &json!({ "solver": solver, "part": key })) {
                description[key] = part[key].clone();
            }
        }
    }
    Some(description)
}

/// Run one solve with `parameters`.
fn solve(door: &mut Door, solver: &str, parameters: &Value) -> Came {
    came(door, "solve", &json!({ "solver": solver, "parameters": parameters }))
}

/// Every attack on one solver. `at_the_bounds` also solves each input at
/// both of its bounds, which takes a full solve each.
pub fn solver_findings(door: &mut Door, solver: &str, verdict: &str, at_the_bounds: bool) -> Vec<Finding> {
    let mut out = Out::new(solver);
    let Some(description) = whole_solver(door, solver) else {
        out.contract("described", "describe_solver gave no description".to_string());
        return out.found;
    };
    let refusals = names_of(&description, "refusals");
    let listed: BTreeSet<String> = refusals.iter().map(|(name, _)| name.clone()).collect();
    let short = door.value("describe_solver", &json!({ "solver": solver, "form": "short" }));
    let mut served = description.clone();
    served["(short form)"] = short.clone().map(|short| json!({ "accuracy_against_measurement": short["accuracy_against_measurement"], "full_form": short["full_form"] })).unwrap_or(Value::Null);
    out.found.extend(text_findings(solver, &served, verdict, &refusals));
    let names = served_names(door);
    out.found.extend(forecast_findings(solver, verdict, &description, &names));
    if let Some(short) = short {
        let stops: BTreeSet<String> = short["refusals"].as_array().into_iter().flatten().filter_map(|n| n.as_str().map(str::to_string)).collect();
        if stops != listed {
            out.contract("refusal names listed", format!("the short form's refusals are not the full form's names: {:?}", stops.symmetric_difference(&listed).collect::<Vec<_>>()));
        }
    }
    let mut named = Vec::new();
    for input in description["inputs"].as_array().into_iter().flatten() {
        if let Some(name) = input["refusal"].as_str() {
            named.push(("input", name.to_string()));
        }
    }
    for invariant in description["invariants"].as_array().into_iter().flatten() {
        if let Some(name) = invariant["refusal"].as_str() {
            named.push(("invariant", name.to_string()));
        }
    }
    if let Some(name) = description["convergence"]["refusal"].as_str() {
        named.push(("convergence", name.to_string()));
    }
    for (where_, text) in named {
        for name in text.split(|c: char| !c.is_ascii_alphanumeric()).filter(|word| word.len() > 3 && word.starts_with(|c: char| c.is_ascii_uppercase()) && word.chars().any(|c| c.is_ascii_lowercase())) {
            if !listed.contains(name) {
                out.contract("refusal names listed", format!("the {where_} names the refusal {name}, which is not among the solver's refusals"));
            }
        }
    }
    let example = description["example_solve"]["parameters"].clone();
    if !example.is_object() {
        out.contract("worked example runs", "the description has no worked question".to_string());
        return out.found;
    }
    // The worked questions an input is attacked in: the worked question, then
    // each further one, for an input only a further one holds (a drive's
    // frequency, say).
    let mut questions = vec![example.clone()];
    if let Some(all) = description.as_object() {
        for (key, further) in all {
            if key.starts_with("example_solve_") && further["parameters"].is_object() {
                questions.push(further["parameters"].clone());
            }
        }
    }
    let mut sentences: Vec<String> = Vec::new();
    for input in description["inputs"].as_array().into_iter().flatten() {
        let name = input["name"].as_str().unwrap_or("").to_string();
        let Some((example, value)) = questions.iter().find_map(|question| question.get(&name).map(|value| (question.clone(), value.clone()))) else { continue };
        let kind = input["type"].as_str().unwrap_or("");
        let refusal = input["refusal"].as_str().map(str::to_string);
        let (Some(minimum), Some(maximum)) = (input["minimum"].as_f64(), input["maximum"].as_f64()) else { continue };
        if minimum == maximum {
            out.note("a refusal is a physical limit", format!("{name} takes one value only, {minimum}"));
        }
        let with = |number: f64| -> Value {
            let mut parameters = example.clone();
            parameters[&name] = match kind {
                "integer" => json!(number as i64),
                "array of numbers" => {
                    let mut list = value.as_array().cloned().unwrap_or_default();
                    if list.is_empty() {
                        list.push(json!(number));
                    } else {
                        list[0] = json!(number);
                    }
                    Value::Array(list)
                }
                _ => json!(number),
            };
            parameters
        };
        let low_exclusive = input["minimum_exclusive"] == json!(true);
        let high_exclusive = input["maximum_exclusive"] == json!(true);
        let past = match kind {
            "integer" => [minimum - 1.0, maximum + 1.0],
            _ => [if low_exclusive { minimum } else { below(minimum) }, if high_exclusive { maximum } else { above(maximum) }],
        };
        // A bound of 1e300 or more in size is no bound: one past it is not a number JSON carries.
        let bounded = |bound: f64| bound.abs() < 1.0e300;
        for (which, number) in [("minimum", past[0]), ("maximum", past[1])] {
            // A subnormal is the number rule's, refused with no name by design.
            if !bounded(number) || (number != 0.0 && !number.is_normal()) {
                continue;
            }
            match solve(door, solver, &with(number)) {
                Came::Stopped(sentence) => {
                    sentences.push(sentence.clone());
                    match (name_in(&sentence), &refusal) {
                        (Some(given), Some(own)) if &given == own => {}
                        (Some(given), _) if listed.contains(&given) => out.note("limits refused by name", format!("{name} = {number:e} past its {which} is refused as {given}, not its own {refusal:?}")),
                        (given, _) => out.contract("limits refused by name", format!("{name} = {number:e} past its {which} is refused as {given:?}, a name the solver does not list: {sentence}")),
                    }
                }
                Came::Answered { outcome, reject, .. } => {
                    if outcome == "converged" {
                        out.contract("limits refused by name", format!("{name} = {number:e}, past its declared {which}, was answered (converged)"));
                    } else if let Some(reject) = reject {
                        if !listed.contains(&reject) {
                            out.contract("refusal names listed", format!("{name} = {number:e} came back refused as {reject}, which the solver does not list"));
                        }
                    }
                }
            }
        }
        if at_the_bounds {
            for (which, number, exclusive) in [("minimum", minimum, low_exclusive), ("maximum", maximum, high_exclusive)] {
                if exclusive || !bounded(number) {
                    continue;
                }
                if let Came::Stopped(sentence) = solve(door, solver, &with(number)) {
                    sentences.push(sentence.clone());
                    if name_in(&sentence).is_some() && name_in(&sentence) == refusal {
                        out.note(
                            "limits refused by name",
                            format!("{name} = {number:e}, its own declared {which}, is refused under its own name in the worked question: the bound served is not the bound for this question (it hangs on another input, or the description's range is wider than the solver's): {sentence}"),
                        );
                    }
                }
            }
        }
        // Left out: refused, never filled in.
        let mut parameters = example.clone();
        parameters.as_object_mut().unwrap().remove(&name);
        if let Came::Answered { outcome, .. } = solve(door, solver, &parameters) {
            out.contract("nothing filled in", format!("a solve with {name} left out was answered ({outcome})"));
        }
    }
    let mut parameters = example.clone();
    parameters["specimen_constant"] = json!(1.0);
    if let Came::Answered { outcome, .. } = solve(door, solver, &parameters) {
        out.contract("fixed constants", format!("a solve with an unknown key was answered ({outcome})"));
    }
    sentence_findings(&mut out, &sentences);
    // The worked question: converged, inside its invariants, the same bytes.
    let request = json!({ "solver": solver, "parameters": example });
    let (error, first) = door.call("solve", &request);
    if error {
        out.contract("worked example runs", format!("the worked question is refused: {first}"));
        return out.found;
    }
    let body: Value = serde_json::from_str(&first).unwrap_or(Value::Null);
    if body["outcome"] != "converged" {
        out.note("worked example runs", format!("the worked question comes back {} as {}", body["outcome"], body["reject"]));
    }
    if let Some(reject) = body["reject"].as_str() {
        if !listed.contains(reject) {
            out.contract("refusal names listed", format!("the worked question came back refused as {reject}, which the solver does not list"));
        }
    }
    let (_, again) = door.call("solve", &request);
    let mut second_door = Door::start();
    let (_, elsewhere) = second_door.call("solve", &request);
    if again != first || elsewhere != first {
        out.contract("same input, same bits", "the worked question gave different bytes on a second call or a second connection".to_string());
    }
    out.found
}

// ----- for a new model's router test -------------------------------------------

fn fail_on_contract(found: Vec<Finding>) {
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(Finding::line).collect();
    for finding in &found {
        println!("{}", finding.line());
    }
    assert!(broken.is_empty(), "the contract is broken:\n{}", broken.join("\n"));
}

/// Every attack on one card, through a new local connection; fails on any
/// finding that breaks the contract.
pub fn assert_card_conforms(domain: &str, card: &str) {
    let mut door = Door::start();
    let verdict = verdicts(&mut door).into_iter().find(|(d, c, _)| d == domain && c == card).map(|(_, _, verdict)| verdict);
    let verdict = verdict.unwrap_or_else(|| panic!("{domain}/{card} is not listed"));
    fail_on_contract(card_findings(&mut door, domain, card, &verdict));
}

/// Every attack on one solver, its bounds solved too; fails on any finding
/// that breaks the contract.
pub fn assert_solver_conforms(solver: &str) {
    let mut door = Door::start();
    let verdict = verdicts(&mut door).into_iter().find(|(d, s, _)| d.is_empty() && s == solver).map(|(_, _, verdict)| verdict);
    let verdict = verdict.unwrap_or_else(|| panic!("{solver} is not listed"));
    fail_on_contract(solver_findings(&mut door, solver, &verdict, true));
}
