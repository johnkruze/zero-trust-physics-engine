//! A check by request: `verify_tape`, `verify_solution` and `verify_dataset`
//! given the request that was computed and the seal the caller was given.
//!
//! The request is computed again, through the same function and with the same
//! permits and limits as the tool that computed it first, and the seal of
//! that computation is compared with the seal given. Nothing is stored and
//! nothing stored is read, so the check answers the same on every connection,
//! the open one included. A request the engine refuses before computing is
//! refused here in the same words.

use serde_json::{Map, Value};

use crate::DoorError;

/// The key that holds the request in a check by request.
pub(crate) const REQUEST: &str = "request";

/// What a check by request asks of a body: `request` and the seal, alone.
/// `None` when the body holds no `request` and so is a whole answer. The seal
/// is held to 96 lowercase hex digits before anything is computed.
pub(crate) fn read<'a>(
    object: &'a Map<String, Value>,
    seal_key: &str,
    whole: &str,
) -> Result<Option<(&'a Value, &'a str)>, DoorError> {
    let Some(request) = object.get(REQUEST) else {
        return Ok(None);
    };
    for key in object.keys() {
        if key != REQUEST && key != seal_key {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{key}' is not a field of a verification by request: send 'request' and '{seal_key}' alone, or {whole} without 'request'."
            )));
        }
    }
    let seal = match object.get(seal_key) {
        None => {
            return Err(DoorError::Unprocessable(format!(
                "Missing required parameter: '{seal_key}' (InputMissing)."
            )));
        }
        Some(Value::String(text)) if crate::json_text::is_lower_hex(text, 96) => text.as_str(),
        Some(_) => {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{seal_key}' must be 96 lowercase hex digits."
            )));
        }
    };
    Ok(Some((request, seal)))
}

/// The sentence for a whole answer sent back with the request under the
/// answer's own name and its seal, and nothing else that the whole answer
/// needs: the caller meant a check by request.
pub(crate) fn meant_by_request(
    object: &Map<String, Value>,
    seal_key: &str,
    first_missing: &str,
) -> Option<DoorError> {
    if object.contains_key("inputs") && object.contains_key(seal_key) && !object.contains_key(first_missing) {
        Some(DoorError::Unprocessable(format!(
            "Missing required parameter: '{first_missing}' (InputMissing). A verification by request sends the request under 'request', not 'inputs': send {{\"request\": <the request as it was sent>, \"{seal_key}\": <the seal>}} alone, or the whole answer."
        )))
    } else {
        None
    }
}

/// What a check by request of a run or a solution says of the text the
/// caller holds: it was not read, and how a copy is checked in full.
pub(crate) const TEXT_NOT_COMPARED: &str = "The request was computed again and the seals compared. No answer text you hold was compared, so a copy with a decimal changed still holds here. To verify a copy of the answer, run verify_receipt on its exact text, which compares the copy's bytes with the receipt that same copy carries, and compare that receipt_sha384 with recomputed_receipt_sha384, which this tool returns when the seals hold. A copy is verified in full only when the two are equal, and they are equal only when the copy is byte for byte the engine's answer to the request sent in the same text. verify_receipt alone does not show that: the receipt is a plain SHA-384 with no key, so a copy with a decimal changed and its receipt worked out again matches it. The long form of this tool, given the whole answer as it came, computes its request again and compares the engine's answer with the copy, value by value and byte for byte, receipt included.";

/// What a check by request of a dataset says of the rows the caller holds.
pub(crate) const DATASET_TEXT_NOT_COMPARED: &str = "The plan was run again and the seals compared; no rows you hold were compared. Only the head carries a receipt, and verify_receipt compares the head's text alone (the head written as its JSON text) with the receipt the head carries, which is a plain SHA-384 with no key. The long form of this tool takes a full answer (head, rows and frames_hex together, as they came), verifies its exact hex values and seals, then runs its plan again and compares the engine's full answer with the one sent, value by value and byte for byte, naming the first row or field that differs.";

/// What a check by request that does not hold says in place of the seal it
/// computed.
pub(crate) fn not_held(seal_key: &str, from: &str) -> String {
    format!(
        "The {seal_key} given is not the seal this request computes to. The seal computed is not returned, so a verification cannot pass by sending back the engine's own answer: the true {seal_key} of this request is in the answer {from} gives to it, and only a seal kept from that answer, or from another source, shows anything when verified here."
    )
}

/// The answer to a check by request: whether the seal given is the seal the
/// request computes to; when it is, that seal and the receipt of the answer
/// computed again, when the tool has one; when it is not, why, and no seal,
/// with what the seal says of the engine version when it covers one; and
/// what was compared.
#[allow(clippy::too_many_arguments)]
pub(crate) fn answer(
    seal_key: &str,
    from: &str,
    given: &str,
    recomputed: &str,
    receipt: Option<&str>,
    compared: &str,
    text_not_compared: &str,
    version: Option<String>,
) -> Result<String, DoorError> {
    let holds = given == recomputed;
    let mut body = Map::new();
    body.insert("holds".to_string(), Value::Bool(holds));
    body.insert(
        "cause".to_string(),
        if holds { Value::Null } else { Value::String(seal_key.to_string()) },
    );
    if holds {
        body.insert(format!("recomputed_{seal_key}"), Value::String(recomputed.to_string()));
        if let Some(receipt) = receipt {
            body.insert("recomputed_receipt_sha384".to_string(), Value::String(receipt.to_string()));
        }
    } else {
        let mut sentence = not_held(seal_key, from);
        if let Some(version) = version {
            sentence.push(' ');
            sentence.push_str(&version);
        }
        body.insert("not_held".to_string(), Value::String(sentence));
    }
    body.insert("compared".to_string(), Value::String(compared.to_string()));
    body.insert("text_not_compared".to_string(), Value::String(text_not_compared.to_string()));
    serde_json::to_string(&Value::Object(body)).map_err(|error| {
        DoorError::Internal(format!(
            "Physical Violation: the response could not be encoded. {error}"
        ))
    })
}

/// What a check by request of a run or a solution compared, said in its
/// answer. `what` names the computation, `seal_key` its seal.
pub(crate) fn compared_with_receipt(what: &str, seal_key: &str) -> String {
    format!(
        "The request was {what} again and the {seal_key} given was compared with the seal of that computation; holds is that comparison. The seal covers the request's values, so a request sent with a number spelled another way (310 and 310.0) has the same seal. No copy of the answer was read: recomputed_receipt_sha384, given when the comparison holds, is the receipt of the answer computed again, which equals the receipt of a copy only when that copy is byte for byte the engine's answer and the request was sent in the same text, number spellings included."
    )
}

/// Engine version names of the form `<crate>_v0.<minor>.<patch>`, the minor
/// from 0 to 30 and the patch from 0 to 5, other than `ours`: the names a
/// seal given by request is tried under to say which version made it.
pub(crate) fn other_versions(prefix: &str, ours: &str) -> Vec<String> {
    let mut names = Vec::new();
    for minor in 0..=30 {
        for patch in 0..=5 {
            let name = format!("{prefix}_v0.{minor}.{patch}");
            if name != ours {
                names.push(name);
            }
        }
    }
    names
}
