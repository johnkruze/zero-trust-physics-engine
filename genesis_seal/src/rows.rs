//! One row of a dataset, and the rows file, `rows.jsonl`.
//!
//! The rows file holds one row per line, in plan order. Each line is one
//! compact JSON object and ends with a line break. A row is the record of one
//! run without its frames.
//!
//! The keys of a row, and what seals each, are stated in this crate's
//! `AGENTS.md`, under "The rows file".

use std::fmt;
use std::io::{self, BufRead, Write};

use serde_json::{Map, Value};

use crate::json_text::is_lower_hex;
use crate::refusal::{
    Refusal, optional_hash, optional_string, require_exact_keys, require_string, require_whole,
    required, wrong,
};
use crate::run::{
    FRAME_LEN, TapeReport, check_tape, follow_frame, link_0, names_a_card, require_state_exact,
    string_or_null,
};
use crate::sealed_form::json_string;

const ROW_KEYS: &[&str] = &[
    "row",
    "request",
    "outcome",
    "steps_landed",
    "reject",
    "reject_detail",
    "final_state",
    "final_residual_conservation_error_hex",
    "max_residual_conservation_error_hex",
    "state_digest",
    "frames",
    "last_link",
    "run_seal",
    "leaf",
];

const NOT_ADMITTED: &str = "null for a row that was not admitted";

/// How a row ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every requested step landed.
    Completed,
    /// The run reached an end the card declares (a first maximum, a floor, a
    /// last day) and stopped there. The run is sealed.
    Ended,
    /// A step was refused and the run stopped. The run is sealed.
    Refused,
    /// The door refused the request before computing. There is no run seal.
    Unadmitted,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Completed => "completed",
            Outcome::Ended => "ended",
            Outcome::Refused => "refused",
            Outcome::Unadmitted => "unadmitted",
        }
    }

    pub fn read(text: &str) -> Option<Outcome> {
        match text {
            "completed" => Some(Outcome::Completed),
            "ended" => Some(Outcome::Ended),
            "refused" => Some(Outcome::Refused),
            "unadmitted" => Some(Outcome::Unadmitted),
            _ => None,
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(self.as_str())
    }
}

/// The leaf of a row: the compact JSON `row`, `request_sha256`, `outcome`,
/// `run_seal`, in that order. `request_sha256` is the row's `link_0` as hex.
/// `run_seal` is null for a row that was not admitted.
pub fn leaf_text(row: u64, request_sha256: &str, outcome: &str, run_seal: Option<&str>) -> String {
    format!(
        "{{\"row\":{},\"request_sha256\":{},\"outcome\":{},\"run_seal\":{}}}",
        row,
        json_string(request_sha256),
        json_string(outcome),
        string_or_null(run_seal),
    )
}

/// One row of a dataset.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub row: u64,
    pub request: Value,
    pub outcome: Outcome,
    pub steps_landed: Option<u64>,
    pub reject: Option<String>,
    pub reject_detail: Option<String>,
    pub final_state: Option<Map<String, Value>>,
    pub final_residual_conservation_error_hex: Option<String>,
    pub max_residual_conservation_error_hex: Option<String>,
    pub state_digest: Option<String>,
    pub frames: u64,
    pub last_link: Option<String>,
    pub run_seal: Option<String>,
    pub leaf: Value,
}

/// Why a run was not made into a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunRefusal {
    /// The run's record was not read.
    NotRead(Refusal),
    /// The run's own seals do not hold.
    DoesNotHold(TapeReport),
    /// The run's outcome is not `completed`, `ended` or `refused`.
    OutcomeNotKnown(String),
}

impl fmt::Display for RunRefusal {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RunRefusal::NotRead(refusal) => refusal.fmt(out),
            RunRefusal::DoesNotHold(report) => match report.cause {
                Some(cause) => write!(out, "The run does not hold: {cause}."),
                None => write!(out, "The run does not hold."),
            },
            RunRefusal::OutcomeNotKnown(outcome) => write!(
                out,
                "The run's outcome is '{outcome}'. A run is completed, ended or refused."
            ),
        }
    }
}

fn option_value(text: &Option<String>) -> Value {
    match text {
        Some(text) => Value::String(text.clone()),
        None => Value::Null,
    }
}

fn leaf_value(row: u64, request: &Value, outcome: Outcome, run_seal: Option<&str>) -> Value {
    let text = leaf_text(
        row,
        &hex::encode(link_0(request)),
        outcome.as_str(),
        run_seal,
    );
    serde_json::from_str(&text).expect("a leaf is JSON")
}

impl Row {
    /// The row of a run, from the response of `run_trajectory`, and the
    /// frames the run recorded. A response whose seals do not hold is
    /// refused.
    pub fn from_run(row: u64, response: &Value) -> Result<(Row, Vec<[u8; FRAME_LEN]>), RunRefusal> {
        let report = check_tape(response).map_err(RunRefusal::NotRead)?;
        if !report.holds {
            return Err(RunRefusal::DoesNotHold(report));
        }
        let object = response.as_object().ok_or(RunRefusal::NotRead(Refusal::NotAnObject))?;
        let field = |key: &str| object.get(key).and_then(Value::as_str).map(str::to_string);
        let outcome_text = field("outcome").unwrap_or_default();
        let outcome = match Outcome::read(&outcome_text) {
            Some(Outcome::Completed) => Outcome::Completed,
            Some(Outcome::Ended) => Outcome::Ended,
            Some(Outcome::Refused) => Outcome::Refused,
            _ => return Err(RunRefusal::OutcomeNotKnown(outcome_text)),
        };
        let request = object.get("inputs").cloned().unwrap_or(Value::Null);
        let mut link = link_0(&request);
        let mut frames = Vec::new();
        for recorded in object.get("frames").and_then(Value::as_array).into_iter().flatten() {
            let followed = recorded
                .get("frame_hex")
                .and_then(Value::as_str)
                .and_then(|text| hex::decode(text).ok())
                .and_then(|bytes| Some((follow_frame(&link, &bytes)?, bytes)));
            // The tape holds, so every frame is 64 bytes and carries its link.
            let Some((next, bytes)) = followed else {
                return Err(RunRefusal::DoesNotHold(report));
            };
            let mut frame = [0_u8; FRAME_LEN];
            frame.copy_from_slice(&bytes);
            link = next;
            frames.push(frame);
        }
        let run_seal = field("run_seal");
        let leaf = leaf_value(row, &request, outcome, run_seal.as_deref());
        Ok((
            Row {
                row,
                request,
                outcome,
                steps_landed: object.get("steps_landed").and_then(Value::as_u64),
                reject: field("reject"),
                reject_detail: field("reject_detail"),
                final_state: object.get("final_state").and_then(Value::as_object).cloned(),
                final_residual_conservation_error_hex: field("final_residual_conservation_error_hex"),
                max_residual_conservation_error_hex: field("max_residual_conservation_error_hex"),
                state_digest: field("state_digest"),
                frames: frames.len() as u64,
                last_link: Some(hex::encode(link)),
                run_seal,
                leaf,
            },
            frames,
        ))
    }

    /// The row of a request the door refused before computing. `sentence` is
    /// the door's answer.
    pub fn unadmitted(row: u64, request: Value, sentence: &str) -> Row {
        let leaf = leaf_value(row, &request, Outcome::Unadmitted, None);
        Row {
            row,
            request,
            outcome: Outcome::Unadmitted,
            steps_landed: None,
            reject: None,
            reject_detail: Some(sentence.to_string()),
            final_state: None,
            final_residual_conservation_error_hex: None,
            max_residual_conservation_error_hex: None,
            state_digest: None,
            frames: 0,
            last_link: None,
            run_seal: None,
            leaf,
        }
    }

    /// The row as a JSON object, with its keys in the order of the table above.
    pub fn to_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("row".into(), Value::from(self.row));
        object.insert("request".into(), self.request.clone());
        object.insert("outcome".into(), Value::from(self.outcome.as_str()));
        object.insert(
            "steps_landed".into(),
            self.steps_landed.map_or(Value::Null, Value::from),
        );
        object.insert("reject".into(), option_value(&self.reject));
        object.insert("reject_detail".into(), option_value(&self.reject_detail));
        object.insert(
            "final_state".into(),
            self.final_state.clone().map_or(Value::Null, Value::Object),
        );
        object.insert(
            "final_residual_conservation_error_hex".into(),
            option_value(&self.final_residual_conservation_error_hex),
        );
        object.insert(
            "max_residual_conservation_error_hex".into(),
            option_value(&self.max_residual_conservation_error_hex),
        );
        object.insert("state_digest".into(), option_value(&self.state_digest));
        object.insert("frames".into(), Value::from(self.frames));
        object.insert("last_link".into(), option_value(&self.last_link));
        object.insert("run_seal".into(), option_value(&self.run_seal));
        object.insert("leaf".into(), self.leaf.clone());
        Value::Object(object)
    }

    /// The row as one line of the rows file, without its line break.
    pub fn to_line(&self) -> String {
        serde_json::to_string(&self.to_value()).expect("a row encodes as JSON")
    }

    /// Scan one line of the rows file, then read it.
    pub fn read_line(line: &[u8]) -> Result<Row, Refusal> {
        Row::read(&crate::json_text::read(line)?)
    }

    /// Read a row. A missing key, a key the row does not have, a value of the
    /// wrong type and a hex value that is not exact are refused by name.
    pub fn read(value: &Value) -> Result<Row, Refusal> {
        let object = value.as_object().ok_or(Refusal::NotAnObject)?;
        require_exact_keys(object, ROW_KEYS)?;
        let row = require_whole(object, "row")?;
        let request = required(object, "request")?;
        if !request.is_object() {
            return Err(wrong("request", "a JSON object"));
        }
        let outcome = Outcome::read(&require_string(object, "outcome")?).ok_or_else(|| {
            wrong(
                "outcome",
                "\"completed\", \"ended\", \"refused\" or \"unadmitted\"",
            )
        })?;
        let reject = optional_string(object, "reject")?;
        let reject_detail = optional_string(object, "reject_detail")?;
        let final_state = match required(object, "final_state")? {
            Value::Null => None,
            Value::Object(state) => Some(state.clone()),
            _ => return Err(wrong("final_state", "a JSON object or null")),
        };
        if let Some(state) = &final_state {
            require_state_exact("final_state", state, names_a_card(request))?;
        }
        let final_hex = optional_hex(object, "final_residual_conservation_error_hex")?;
        let max_hex = optional_hex(object, "max_residual_conservation_error_hex")?;
        let state_digest = optional_hash(object, "state_digest")?;
        let frames = require_whole(object, "frames")?;
        let last_link = optional_hash(object, "last_link")?;
        let run_seal = optional_hash(object, "run_seal")?;
        let leaf = required(object, "leaf")?;
        if !leaf.is_object() {
            return Err(wrong("leaf", "a JSON object"));
        }
        let steps_landed = match required(object, "steps_landed")? {
            Value::Null => None,
            value => Some(
                value
                    .as_u64()
                    .ok_or_else(|| wrong("steps_landed", "an integer >= 0 or null"))?,
            ),
        };
        if outcome == Outcome::Unadmitted {
            for (key, is_null) in [
                ("steps_landed", steps_landed.is_none()),
                ("reject", reject.is_none()),
                ("final_state", final_state.is_none()),
                ("final_residual_conservation_error_hex", final_hex.is_none()),
                ("max_residual_conservation_error_hex", max_hex.is_none()),
                ("state_digest", state_digest.is_none()),
                ("last_link", last_link.is_none()),
                ("run_seal", run_seal.is_none()),
            ] {
                if !is_null {
                    return Err(wrong(key, NOT_ADMITTED));
                }
            }
            if frames != 0 {
                return Err(wrong("frames", "0 for a row that was not admitted"));
            }
        } else {
            if steps_landed.is_none() {
                return Err(wrong("steps_landed", "an integer >= 0 for a run"));
            }
            for (key, is_null) in [("last_link", last_link.is_none()), ("run_seal", run_seal.is_none())] {
                if is_null {
                    return Err(Refusal::NotExactHex {
                        field: key.to_string(),
                        digits: 64,
                    });
                }
            }
        }
        Ok(Row {
            row,
            request: request.clone(),
            outcome,
            steps_landed,
            reject,
            reject_detail,
            final_state,
            final_residual_conservation_error_hex: final_hex,
            max_residual_conservation_error_hex: max_hex,
            state_digest,
            frames,
            last_link,
            run_seal,
            leaf: leaf.clone(),
        })
    }
}

fn optional_hex(object: &Map<String, Value>, key: &str) -> Result<Option<String>, Refusal> {
    let text = optional_string(object, key)?;
    if let Some(text) = &text {
        if !is_lower_hex(text, 16) {
            return Err(Refusal::NotExactHex {
                field: key.to_string(),
                digits: 16,
            });
        }
    }
    Ok(text)
}

/// The lines of a rows file. Nothing is returned when the file is empty,
/// does not end with a line break, or holds an empty line.
pub fn lines(bytes: &[u8]) -> Option<Vec<&[u8]>> {
    let body = bytes.strip_suffix(b"\n")?;
    let lines: Vec<&[u8]> = body.split(|byte| *byte == b'\n').collect();
    if lines.iter().any(|line| line.is_empty()) {
        return None;
    }
    Some(lines)
}

/// Write one row as one line.
pub fn write_row<W: Write>(out: &mut W, row: &Row) -> io::Result<()> {
    out.write_all(row.to_line().as_bytes())?;
    out.write_all(b"\n")
}

/// The bytes of a rows file.
pub fn rows_text(rows: &[Row]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for row in rows {
        bytes.extend_from_slice(row.to_line().as_bytes());
        bytes.push(b'\n');
    }
    bytes
}

/// Read the next row from `input`. `Ok(None)` is the end of the file.
pub fn read_row<R: BufRead>(input: &mut R) -> io::Result<Option<Result<Row, Refusal>>> {
    let mut line = Vec::new();
    if input.read_until(b'\n', &mut line)? == 0 {
        return Ok(None);
    }
    if line.last() == Some(&b'\n') {
        line.pop();
    }
    Ok(Some(Row::read_line(&line)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> Value {
        json!({ "card": "thermal", "domain": "energy", "dt_s": 0.1 })
    }

    #[test]
    fn the_leaf_is_compact_json_with_four_keys_in_order() {
        assert_eq!(
            leaf_text(7, "ab", "completed", Some("cd")),
            r#"{"row":7,"request_sha256":"ab","outcome":"completed","run_seal":"cd"}"#
        );
        assert_eq!(
            leaf_text(0, "ab", "unadmitted", None),
            r#"{"row":0,"request_sha256":"ab","outcome":"unadmitted","run_seal":null}"#
        );
    }

    #[test]
    fn a_row_that_was_not_admitted_is_written_and_read_back() {
        let row = Row::unadmitted(3, request(), "Physical Violation: the load is above the limit.");
        let line = row.to_line();
        assert_eq!(
            line,
            format!(
                r#"{{"row":3,"request":{{"card":"thermal","domain":"energy","dt_s":0.1}},"outcome":"unadmitted","steps_landed":null,"reject":null,"reject_detail":"Physical Violation: the load is above the limit.","final_state":null,"final_residual_conservation_error_hex":null,"max_residual_conservation_error_hex":null,"state_digest":null,"frames":0,"last_link":null,"run_seal":null,"leaf":{{"row":3,"request_sha256":"{}","outcome":"unadmitted","run_seal":null}}}}"#,
                hex::encode(link_0(&request()))
            )
        );
        assert_eq!(Row::read_line(line.as_bytes()).unwrap(), row);
        let mut file = Vec::new();
        write_row(&mut file, &row).unwrap();
        write_row(&mut file, &row).unwrap();
        assert_eq!(file, rows_text(&[row.clone(), row.clone()]));
        assert_eq!(lines(&file).unwrap().len(), 2);
        let mut input = file.as_slice();
        assert_eq!(read_row(&mut input).unwrap(), Some(Ok(row.clone())));
        assert_eq!(read_row(&mut input).unwrap(), Some(Ok(row)));
        assert_eq!(read_row(&mut input).unwrap(), None);
    }

    #[test]
    fn a_rows_file_ends_each_line_and_holds_no_empty_line() {
        assert!(lines(b"").is_none());
        assert!(lines(b"{}").is_none());
        assert!(lines(b"{}\n\n").is_none());
        assert!(lines(b"\n").is_none());
        assert_eq!(lines(b"{}\n[]\n").unwrap(), [&b"{}"[..], &b"[]"[..]]);
    }

    #[test]
    fn a_malformed_row_is_refused_by_name() {
        let good = Row::unadmitted(0, request(), "refused").to_value();
        let edited = |edit: &dyn Fn(&mut Value)| {
            let mut value = good.clone();
            edit(&mut value);
            Row::read(&value).unwrap_err().to_string()
        };
        assert_eq!(
            edited(&|row| {
                row.as_object_mut().unwrap().remove("leaf");
            }),
            "Missing required parameter: 'leaf'"
        );
        assert_eq!(
            edited(&|row| row["note"] = json!("x")),
            "Physical Violation: 'note' is not a field of this request."
        );
        assert_eq!(
            edited(&|row| row["outcome"] = json!("done")),
            "Physical Violation: 'outcome' must be \"completed\", \"ended\", \"refused\" or \"unadmitted\"."
        );
        assert_eq!(
            edited(&|row| row["run_seal"] = json!("ab".repeat(32))),
            "Physical Violation: 'run_seal' must be null for a row that was not admitted."
        );
        assert_eq!(
            edited(&|row| row["frames"] = json!(1)),
            "Physical Violation: 'frames' must be 0 for a row that was not admitted."
        );
        assert_eq!(
            edited(&|row| row["outcome"] = json!("completed")),
            "Physical Violation: 'steps_landed' must be an integer >= 0 for a run."
        );
        assert_eq!(
            edited(&|row| {
                row["outcome"] = json!("completed");
                row["steps_landed"] = json!(0);
            }),
            "Physical Violation: 'last_link' must be 64 lowercase hex digits."
        );
        assert_eq!(
            edited(&|row| row["state_digest"] = json!("AB".repeat(32))),
            "Physical Violation: 'state_digest' must be 64 lowercase hex digits."
        );
        assert_eq!(
            edited(&|row| row["final_residual_conservation_error_hex"] = json!("0")),
            "Physical Violation: 'final_residual_conservation_error_hex' must be 16 lowercase hex digits."
        );
        assert_eq!(
            Row::read_line(br#"{"row":0,"row":1}"#).unwrap_err().to_string(),
            "Physical Violation: the key 'row' is repeated."
        );
    }
}
