//! One run: its chain, its state digest, its run seal, and the check of the
//! tape it writes.
//!
//! A recorded frame is 64 bytes: `t` 8 bytes, `pos` 3 x 4, `vel` 3 x 4,
//! `force_torque` 4, `residual` 4, `flags` 8, `proof` 16. This crate reads
//! none of the numbers. It reads and writes the proof slot, bytes 48 to 63,
//! and hashes all 64.

use std::fmt;

use serde_json::{Map, Value};

use crate::json_text::is_lower_hex;
use crate::refusal::{
    Refusal, optional_string, require_digits, require_string, require_whole, required, wrong,
};
use crate::sealed_form::{digest, digest_hex, json_string, request_digest};

/// The length of a recorded frame, in bytes.
pub const FRAME_LEN: usize = 64;
/// The first byte of the proof slot.
pub const PROOF_START: usize = 48;
/// The length of the proof slot, in bytes.
pub const PROOF_LEN: usize = 16;

/// The part of a tape that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapeCause {
    /// A frame does not carry the previous link, or does not hash to its stored link.
    Chain,
    /// The state digest rebuilt from the final state is not the stored one.
    State,
    /// The frames and the state digest match and the run seal does not.
    RunSeal,
}

impl TapeCause {
    pub fn as_str(self) -> &'static str {
        match self {
            TapeCause::Chain => "chain",
            TapeCause::State => "state",
            TapeCause::RunSeal => "run_seal",
        }
    }
}

impl fmt::Display for TapeCause {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(self.as_str())
    }
}

/// The answer to the check of one tape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapeReport {
    pub holds: bool,
    /// The index of the first failing frame on a chain failure, the frame
    /// count on a run seal failure, and nothing otherwise.
    pub broken_link: Option<u64>,
    pub cause: Option<TapeCause>,
}

/// `link_0` of a run: SHA-256 of the sealed form of the whole request.
pub fn link_0(request: &Value) -> [u8; 32] {
    request_digest(request)
}

/// Write the first 16 bytes of `link` into the proof slot of `frame`, and
/// return the next link, SHA-256 of the 64 frame bytes.
pub fn seal_frame(link: &[u8; 32], frame: &mut [u8; FRAME_LEN]) -> [u8; 32] {
    frame[PROOF_START..].copy_from_slice(&link[..PROOF_LEN]);
    digest(frame)
}

/// The next link, when `frame` is 64 bytes and its proof slot holds the first
/// 16 bytes of `link`.
pub fn follow_frame(link: &[u8; 32], frame: &[u8]) -> Option<[u8; 32]> {
    if frame.len() != FRAME_LEN || frame[PROOF_START..] != link[..PROOF_LEN] {
        return None;
    }
    Some(digest(frame))
}

/// The count of frames a run records when `steps_landed` steps land: one at
/// every multiple of `record_every`, and one at the last landed step.
pub fn frames_recorded(steps_landed: u64, record_every: u64) -> Option<u64> {
    if record_every == 0 {
        return None;
    }
    let periodic = steps_landed / record_every;
    if steps_landed % record_every == 0 {
        Some(periodic)
    } else {
        periodic.checked_add(1)
    }
}

/// The text the state digest is taken over.
///
/// Every key that ends in `_hex` and every boolean key, sorted by key, one
/// `key:value` line each. An array of hex strings is joined with commas. The
/// two residual lines follow. Nothing is returned when a hex value is not a
/// string or an array of strings.
pub fn state_digest_text(
    state: &Map<String, Value>,
    final_hex: &str,
    max_hex: &str,
) -> Option<String> {
    let mut lines = Vec::new();
    for (key, value) in state {
        if key.ends_with("_hex") {
            lines.push((key.as_str(), render_hex(value)?));
        } else if let Some(flag) = value.as_bool() {
            lines.push((key.as_str(), if flag { "true" } else { "false" }.to_string()));
        }
    }
    lines.sort_by(|left, right| left.0.cmp(right.0));
    let mut text = String::new();
    for (key, value) in lines {
        text.push_str(key);
        text.push(':');
        text.push_str(&value);
        text.push('\n');
    }
    text.push_str("final_residual_conservation_error_hex:");
    text.push_str(final_hex);
    text.push('\n');
    text.push_str("max_residual_conservation_error_hex:");
    text.push_str(max_hex);
    text.push('\n');
    Some(text)
}

/// SHA-256 of [`state_digest_text`], as hex.
pub fn state_digest(state: &Map<String, Value>, final_hex: &str, max_hex: &str) -> Option<String> {
    state_digest_text(state, final_hex, max_hex).map(|text| digest_hex(text.as_bytes()))
}

fn render_hex(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                parts.push(item.as_str()?);
            }
            Some(parts.join(","))
        }
        _ => None,
    }
}

/// The state digest a tape must carry.
///
/// `Some(None)` is a null digest: no step landed, and the state and both
/// residuals are null. `None` means the state cannot belong to this tape.
pub fn expected_state_digest(
    steps_landed: u64,
    final_state: Option<&Map<String, Value>>,
    final_hex: Option<&str>,
    max_hex: Option<&str>,
) -> Option<Option<String>> {
    if steps_landed == 0 {
        if final_state.is_none() && final_hex.is_none() && max_hex.is_none() {
            Some(None)
        } else {
            None
        }
    } else {
        state_digest(final_state?, final_hex?, max_hex?).map(Some)
    }
}

/// The compact JSON the run seal ends with: `outcome`, `steps_landed`,
/// `reject`, `state_digest`, in that order, with no spaces.
pub fn run_tail(
    outcome: &str,
    steps_landed: u64,
    reject: Option<&str>,
    state_digest: Option<&str>,
) -> String {
    format!(
        "{{\"outcome\":{},\"steps_landed\":{},\"reject\":{},\"state_digest\":{}}}",
        json_string(outcome),
        steps_landed,
        string_or_null(reject),
        string_or_null(state_digest),
    )
}

pub(crate) fn string_or_null(text: Option<&str>) -> String {
    match text {
        Some(text) => json_string(text),
        None => "null".to_string(),
    }
}

/// SHA-256 of the last link as 64 hex characters followed by [`run_tail`].
pub fn run_seal(
    last_link: &[u8; 32],
    outcome: &str,
    steps_landed: u64,
    reject: Option<&str>,
    state_digest: Option<&str>,
) -> String {
    let mut bytes = hex::encode(last_link).into_bytes();
    bytes.extend_from_slice(run_tail(outcome, steps_landed, reject, state_digest).as_bytes());
    digest_hex(&bytes)
}

/// Scan the text of a tape, read it, and check it.
pub fn check_tape_text(bytes: &[u8]) -> Result<TapeReport, Refusal> {
    let body = crate::json_text::read(bytes)?;
    check_tape(&body)
}

/// Check a tape without the engine.
///
/// The tape carries `inputs`, `frames`, `outcome`, `steps_landed`, `reject`,
/// `run_seal`, `final_state`, the two residual hex fields and `state_digest`.
/// Other fields are ignored, so a trajectory response can be given as it is.
/// The chain is checked first, then the state digest, then the run seal.
pub fn check_tape(body: &Value) -> Result<TapeReport, Refusal> {
    let object = body.as_object().ok_or(Refusal::NotAnObject)?;
    let inputs = required(object, "inputs")?;
    let frames = required(object, "frames")?
        .as_array()
        .ok_or_else(|| wrong("frames", "a JSON array"))?;
    let outcome = require_string(object, "outcome")?;
    let steps_landed = require_whole(object, "steps_landed")?;
    let reject = optional_string(object, "reject")?;
    let run_seal = require_string(object, "run_seal")?;
    let final_state = required(object, "final_state")?;
    if !(final_state.is_null() || final_state.is_object()) {
        return Err(wrong("final_state", "a JSON object or null"));
    }
    let final_state = final_state.as_object();
    let final_hex = optional_string(object, "final_residual_conservation_error_hex")?;
    let max_hex = optional_string(object, "max_residual_conservation_error_hex")?;
    let state_digest = optional_string(object, "state_digest")?;

    require_digits("run_seal", &run_seal, 64)?;
    if let Some(text) = &state_digest {
        require_digits("state_digest", text, 64)?;
    }
    if let Some(text) = &final_hex {
        require_digits("final_residual_conservation_error_hex", text, 16)?;
    }
    if let Some(text) = &max_hex {
        require_digits("max_residual_conservation_error_hex", text, 16)?;
    }
    if let Some(state) = final_state {
        require_state_exact("final_state", state, names_a_card(inputs))?;
    }
    require_lowercase_frames(frames)?;

    let mut link = link_0(inputs);
    for (index, frame) in frames.iter().enumerate() {
        match follow_recorded(&link, frame) {
            Some(next) => link = next,
            None => {
                return Ok(TapeReport {
                    holds: false,
                    broken_link: Some(index as u64),
                    cause: Some(TapeCause::Chain),
                });
            }
        }
    }
    let state_broken = TapeReport {
        holds: false,
        broken_link: None,
        cause: Some(TapeCause::State),
    };
    let Some(expected) = expected_state_digest(
        steps_landed,
        final_state,
        final_hex.as_deref(),
        max_hex.as_deref(),
    ) else {
        return Ok(state_broken);
    };
    if expected != state_digest {
        return Ok(state_broken);
    }
    let rebuilt = self::run_seal(
        &link,
        &outcome,
        steps_landed,
        reject.as_deref(),
        expected.as_deref(),
    );
    if rebuilt != run_seal {
        return Ok(TapeReport {
            holds: false,
            broken_link: Some(frames.len() as u64),
            cause: Some(TapeCause::RunSeal),
        });
    }
    Ok(TapeReport {
        holds: true,
        broken_link: None,
        cause: None,
    })
}

/// A frame's `frame_hex` and `link` written with capital letters are refused
/// by name, as every other hex value of a tape is. Any other fault in a frame
/// is a chain failure at its index, found when the chain is followed.
fn require_lowercase_frames(frames: &[Value]) -> Result<(), Refusal> {
    for (index, frame) in frames.iter().enumerate() {
        let Some(object) = frame.as_object() else {
            continue;
        };
        for (key, digits) in [("frame_hex", 2 * 64), ("link", 64)] {
            let capital = object
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(crate::json_text::has_capital_hex);
            if capital {
                return Err(Refusal::NotExactHex {
                    field: format!("frames[{index}].{key}"),
                    digits,
                });
            }
        }
    }
    Ok(())
}

/// One entry of `frames` in a tape: `frame_hex` is 128 hex characters and
/// `link` is 64. Anything else is a chain failure at that index.
fn follow_recorded(link: &[u8; 32], frame: &Value) -> Option<[u8; 32]> {
    let object = frame.as_object()?;
    let frame_hex = object.get("frame_hex")?.as_str()?;
    let stored_link = object.get("link")?.as_str()?;
    let bytes = hex::decode(frame_hex).ok()?;
    let next = follow_frame(link, &bytes)?;
    let stored = hex::decode(stored_link).ok()?;
    if stored.as_slice() != next.as_slice() {
        return None;
    }
    Some(next)
}

/// `true` when `inputs` carries a `domain` string and a `card` string.
pub(crate) fn names_a_card(inputs: &Value) -> bool {
    inputs.as_object().is_some_and(|object| {
        object.get("domain").is_some_and(Value::is_string)
            && object.get("card").is_some_and(Value::is_string)
    })
}

/// Refuse a state the digest text could be misled by.
///
/// A name holds lowercase letters, digits and underscore. A value under a
/// `_hex` name is 16 lowercase hex digits or an array of those. Any other
/// value is a JSON boolean, or the decimal of a number whose `_hex` sibling
/// is in the state. This crate knows no card, so it cannot refuse a name a
/// card does not publish.
pub(crate) fn require_state_exact(
    section: &str,
    state: &Map<String, Value>,
    names_a_card: bool,
) -> Result<(), Refusal> {
    for (key, value) in state {
        if !crate::json_text::is_plain_field_name(key) {
            return Err(Refusal::NotAFieldName(format!("{section}.{key}")));
        }
        if !names_a_card {
            return Err(Refusal::NamesNoCard);
        }
        if key.ends_with("_hex") {
            if !is_f64_hex(value) {
                return Err(Refusal::NotExactHex {
                    field: format!("{section}.{key}"),
                    digits: 16,
                });
            }
        } else if !value.is_boolean() && !state.contains_key(&format!("{key}_hex")) {
            return Err(Refusal::NotSealed(format!("{section}.{key}")));
        }
    }
    Ok(())
}

fn is_f64_hex(value: &Value) -> bool {
    match value {
        Value::String(text) => is_lower_hex(text, 16),
        Value::Array(items) if !items.is_empty() => items
            .iter()
            .all(|item| item.as_str().is_some_and(|text| is_lower_hex(text, 16))),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hash() -> String {
        "ab".repeat(32)
    }

    fn tape(final_state: Value) -> Value {
        let hash = hash();
        json!({
            "inputs": {"domain": "energy", "card": "thermal"},
            "frames": [],
            "outcome": "completed",
            "steps_landed": 1,
            "reject": null,
            "run_seal": hash,
            "final_state": final_state,
            "final_residual_conservation_error_hex": "0000000000000000",
            "max_residual_conservation_error_hex": "0000000000000000",
            "state_digest": hash
        })
    }

    #[test]
    fn a_state_hex_or_flag_that_is_not_exact_is_refused_by_name() {
        let stuffed = tape(json!({
            "temperature_k": 9999.0,
            "temperature_k_hex": "4072526666666666\nderated:false"
        }));
        assert_eq!(
            check_tape(&stuffed).unwrap_err().to_string(),
            "Physical Violation: 'final_state.temperature_k_hex' must be 16 lowercase hex digits."
        );

        let flag = tape(json!({
            "derated": "true",
            "temperature_k_hex": "4072526666666666"
        }));
        assert_eq!(
            check_tape(&flag).unwrap_err(),
            Refusal::NotSealed("final_state.derated".to_string())
        );

        let upper = tape(json!({
            "derated": false,
            "temperature_k_hex": "407252666666666A"
        }));
        assert!(
            check_tape(&upper)
                .unwrap_err()
                .to_string()
                .contains("final_state.temperature_k_hex")
        );

        let empty_array = tape(json!({ "position_m_hex": [] }));
        assert!(check_tape(&empty_array).is_err());

        let wrong = tape(json!({
            "derated": false,
            "temperature_k_hex": "4072526666666660"
        }));
        let report = check_tape(&wrong).expect("a well-formed hex is a verdict");
        assert!(!report.holds);
        assert_eq!(report.cause, Some(TapeCause::State));
        assert_eq!(report.broken_link, None);
    }

    #[test]
    fn a_state_name_that_carries_other_lines_is_refused() {
        let packed = tape(json!({
            "derated:false\ntemperature_k_hex": "4072526666666666"
        }));
        let error = check_tape(&packed).unwrap_err();
        assert!(error.to_string().contains("is not a field name"), "{error}");
        assert!(error.to_string().contains("final_state."), "{error}");
    }

    #[test]
    fn a_state_on_a_tape_that_names_no_card_is_refused() {
        let mut nameless = tape(json!({ "temperature_k_hex": "4072526666666666" }));
        nameless["inputs"] = json!({ "domain": "energy" });
        assert_eq!(check_tape(&nameless).unwrap_err(), Refusal::NamesNoCard);
        assert_eq!(
            Refusal::NamesNoCard.to_string(),
            "Physical Violation: the tape does not name a card."
        );
    }

    #[test]
    fn a_missing_field_or_a_field_of_the_wrong_type_is_refused_by_name() {
        for key in [
            "inputs",
            "frames",
            "outcome",
            "steps_landed",
            "reject",
            "run_seal",
            "final_state",
            "final_residual_conservation_error_hex",
            "max_residual_conservation_error_hex",
            "state_digest",
        ] {
            let mut body = tape(json!({ "temperature_k_hex": "4072526666666666" }));
            body.as_object_mut().unwrap().remove(key);
            assert_eq!(
                check_tape(&body).unwrap_err().to_string(),
                format!("Missing required parameter: '{key}'")
            );
        }
        let state = json!({ "temperature_k_hex": "4072526666666666" });
        let mut body = tape(state.clone());
        body["frames"] = json!({});
        assert_eq!(
            check_tape(&body).unwrap_err().to_string(),
            "Physical Violation: 'frames' must be a JSON array."
        );
        let mut body = tape(state.clone());
        body["steps_landed"] = json!(1.0);
        assert_eq!(
            check_tape(&body).unwrap_err().to_string(),
            "Physical Violation: 'steps_landed' must be an integer >= 0."
        );
        let mut body = tape(state.clone());
        body["reject"] = json!(7);
        assert_eq!(
            check_tape(&body).unwrap_err().to_string(),
            "Physical Violation: 'reject' must be a string or null."
        );
        let mut body = tape(state.clone());
        body["final_state"] = json!([]);
        assert_eq!(
            check_tape(&body).unwrap_err().to_string(),
            "Physical Violation: 'final_state' must be a JSON object or null."
        );
        let mut body = tape(state.clone());
        body["run_seal"] = json!("AB".repeat(32));
        assert_eq!(
            check_tape(&body).unwrap_err().to_string(),
            "Physical Violation: 'run_seal' must be 64 lowercase hex digits."
        );
        let mut body = tape(state);
        body["outcome"] = json!("");
        assert_eq!(
            check_tape(&body).unwrap_err().to_string(),
            "Missing required parameter: 'outcome'"
        );
        assert_eq!(check_tape(&json!([])).unwrap_err(), Refusal::NotAnObject);
    }

    #[test]
    fn a_tape_text_that_repeats_a_key_is_refused_before_it_is_read() {
        let text = br#"{"inputs":{"domain":"energy","card":"thermal","dt_s":0.1,"dt_s":0.2},"frames":[]}"#;
        assert_eq!(
            check_tape_text(text).unwrap_err().to_string(),
            "Physical Violation: the key 'dt_s' is repeated."
        );
    }

    #[test]
    fn the_run_tail_is_compact_json_with_four_keys_in_order() {
        assert_eq!(
            run_tail("completed", 10, None, Some("ab")),
            r#"{"outcome":"completed","steps_landed":10,"reject":null,"state_digest":"ab"}"#
        );
        assert_eq!(
            run_tail("refused", 0, Some("AltitudeOutsideCard"), None),
            r#"{"outcome":"refused","steps_landed":0,"reject":"AltitudeOutsideCard","state_digest":null}"#
        );
    }

    #[test]
    fn a_frame_is_recorded_at_each_multiple_and_at_the_last_landed_step() {
        assert_eq!(frames_recorded(0, 1), Some(0));
        assert_eq!(frames_recorded(10, 1), Some(10));
        assert_eq!(frames_recorded(10, 3), Some(4));
        assert_eq!(frames_recorded(9, 3), Some(3));
        assert_eq!(frames_recorded(2, 5), Some(1));
        assert_eq!(frames_recorded(5, 0), None);
    }

    #[test]
    fn a_sealed_frame_is_followed_and_a_changed_one_is_not() {
        let link = link_0(&json!({ "domain": "energy", "card": "thermal" }));
        let mut frame = [7_u8; FRAME_LEN];
        let next = seal_frame(&link, &mut frame);
        assert_eq!(frame[PROOF_START..], link[..PROOF_LEN]);
        assert_eq!(follow_frame(&link, &frame), Some(next));
        let mut changed = frame;
        changed[0] ^= 1;
        assert_ne!(follow_frame(&link, &changed), Some(next));
        changed = frame;
        changed[63] ^= 1;
        assert_eq!(follow_frame(&link, &changed), None);
        assert_eq!(follow_frame(&link, &frame[..63]), None);
    }

    #[test]
    fn capital_letters_in_a_frame_are_refused_by_name_and_other_faults_stay_chain_failures() {
        let mut upper_frame = tape(Value::Null);
        upper_frame["frames"] = json!([{"frame_hex": "AB".repeat(64), "link": "ab".repeat(32)}]);
        assert_eq!(
            check_tape(&upper_frame).unwrap_err().to_string(),
            "Physical Violation: 'frames[0].frame_hex' must be 128 lowercase hex digits."
        );

        let mut upper_link = tape(Value::Null);
        upper_link["frames"] = json!([
            {"frame_hex": "ab".repeat(64), "link": "ab".repeat(32)},
            {"frame_hex": "ab".repeat(64), "link": "AB".repeat(32)}
        ]);
        assert_eq!(
            check_tape(&upper_link).unwrap_err().to_string(),
            "Physical Violation: 'frames[1].link' must be 64 lowercase hex digits."
        );

        // The same frame in lowercase, and a frame that is not hex at all,
        // are still chain failures at their index.
        for frame in [
            json!({"frame_hex": "ab".repeat(64), "link": "ab".repeat(32)}),
            json!({"frame_hex": "zz", "link": "ab".repeat(32)}),
            json!("not an object"),
        ] {
            let mut body = tape(Value::Null);
            body["frames"] = json!([frame]);
            let report = check_tape(&body).expect("a report");
            assert!(!report.holds);
            assert_eq!(report.broken_link, Some(0));
            assert_eq!(report.cause, Some(TapeCause::Chain));
        }
    }
}
