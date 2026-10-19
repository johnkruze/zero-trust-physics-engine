//! The check of a receipt: `POST /verify_receipt` and the MCP tool of the same name.
//!
//! The rule is `genesis_seal::receipt`, the one the dataset head is checked by.
//! The text of a response must end with `,"receipt_sha384":"<96 lowercase hex>"}`
//! as `append_receipt` writes it. The check takes no permit and runs no physics.

use genesis_seal::receipt::read_receipt;
use serde::Serialize;
use serde_json::Value;

use crate::DoorError;

const COVERS: &str =
    "every byte of the response before the receipt, the decimal values included.";
const DOES_NOT_SHOW: &str = "who produced the response or when: the receipt has no key. The engine's signature after it, when the response carries one, shows which key issued it and when, and is reported under signature. Replaying the request through the engine shows the engine produced it.";

/// What `matches` proves, said in every answer beside it (0.14.4, after G4's
/// red team: a receipt written by hand over invented text matches).
fn match_proves(matches: bool, engine_version_is_this_engines: Option<bool>) -> String {
    if !matches {
        return "Nothing: the bytes before the receipt are not the bytes it was written over. The text, or the receipt, was changed after the receipt was written.".to_string();
    }
    let version = match engine_version_is_this_engines {
        Some(true) => "The text names an engine_version this engine writes, which anyone can also write.",
        Some(false) => "The text names an engine_version this engine does not write.",
        None => "The text names no engine_version.",
    };
    format!(
        "Only that the bytes before the receipt are unchanged since the receipt was written. It is not a verification: anyone can write a matching receipt for any text. {version} That this engine produced the answer needs engine_version_is_this_engines true and a replay of its request (verify_tape, verify_solution or verify_dataset given the whole answer)."
    )
}

#[derive(Serialize)]
pub struct ReceiptReport {
    /// The bytes before the receipt hash to it: unchanged since it was
    /// written, and nothing more ([`ReceiptReport::match_proves`]).
    pub matches: bool,
    /// The `engine_version` the response names, when it names one: a run's,
    /// a step's, a solve's, a comparison's or a dataset head's. The receipt
    /// covers it as it covers every byte, whichever version it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_version: Option<String>,
    /// Whether that version is the one this engine writes on that kind of
    /// answer: `genesis_kernel_v…` on a card's answer and the head of a
    /// dataset of runs, `genesis_solvers_v…` on a solve, a plan of solves'
    /// head, a comparison of solves and a search over a solve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_version_is_this_engines: Option<bool>,
    pub receipt_sha384: String,
    pub recomputed_sha384: String,
    /// What the check of the engine's signature found, when the response
    /// carries one or a public_key was given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Value>,
    pub covers: &'static str,
    pub does_not_show: &'static str,
    /// What `matches` proves, in words: the bytes, not their origin.
    pub match_proves: String,
}

/// `{"response_text": "<the exact text of a response>"}` to the report, with
/// no key of this engine's to check a signature with.
#[cfg_attr(not(test), allow(dead_code))]
pub fn verify(body: &Value) -> Result<ReceiptReport, DoorError> {
    verify_with(None, body)
}

/// The same at a door, whose own answer key checks a signature when the
/// request gives no `public_key`.
pub fn verify_in(state: &crate::AppState, body: &Value) -> Result<ReceiptReport, DoorError> {
    verify_with(state.answer_key().map(|key| key.public_key().clone()), body)
}

fn verify_with(engine_key: Option<genesis_seal::answer_signature::PublicKey>, body: &Value) -> Result<ReceiptReport, DoorError> {
    let object = body.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    for key in object.keys() {
        if key != "response_text" && key != crate::answer_key::PUBLIC_KEY_FIELD && key != crate::answer_key::SEEN_NONCES_FIELD {
            return Err(DoorError::Unprocessable(crate::locked_or_unknown(key)));
        }
    }
    let text = match object.get("response_text") {
        None => {
            return Err(DoorError::Unprocessable(
                "Missing required parameter: 'response_text' (InputMissing).".to_string(),
            ));
        }
        Some(Value::String(text)) => text,
        Some(_) => {
            return Err(DoorError::Unprocessable(
                "Physical Violation: 'response_text' must be a string.".to_string(),
            ));
        }
    };
    let read = read_receipt(text.as_bytes()).ok_or_else(|| {
        DoorError::Unprocessable(
            "Physical Violation: the text does not end with a receipt as this verifier writes one."
                .to_string(),
        )
    })?;
    let answer = serde_json::from_str::<Value>(text).ok();
    let engine_version = answer
        .as_ref()
        .and_then(|answer| answer.get("engine_version").and_then(Value::as_str).map(str::to_string));
    let engine_version_is_this_engines = engine_version.as_deref().map(|version| {
        let ours = if answer.as_ref().is_some_and(of_solves) { crate::solvers::engine_version() } else { crate::ENGINE_VERSION.to_string() };
        version == ours
    });
    let signature = crate::answer_key::receipt_signature(engine_key, object, text)?;
    let matches = read.matches();
    Ok(ReceiptReport {
        matches,
        engine_version,
        engine_version_is_this_engines,
        receipt_sha384: read.stated,
        recomputed_sha384: read.recomputed,
        signature,
        covers: COVERS,
        does_not_show: DOES_NOT_SHOW,
        match_proves: match_proves(matches, engine_version_is_this_engines),
    })
}

/// Whether `answer` is of solves, so that this engine writes the solvers'
/// version on it: a solve (`inputs.solver`), a plan of solves' head
/// (`request.solver`), a comparison of solves (`inputs.solves`) and a search
/// over a solve (`inputs.request.solver`). Every other answer is of a card's
/// runs or steps and carries the kernel's version. Until the work orders of
/// 18 October 2026 either version was taken on any answer, so a plan of
/// solves of 0.13.1, which named the kernel's version as 0.14.3 did, was
/// called this engine's (RT3 W1 of 0.14.0 rc1).
fn of_solves(answer: &Value) -> bool {
    let has_solver = |value: Option<&Value>| value.and_then(|value| value.get("solver")).is_some();
    let inputs = answer.get("inputs");
    has_solver(inputs)
        || has_solver(answer.get("request"))
        || inputs.and_then(|inputs| inputs.get("solves")).is_some()
        || has_solver(inputs.and_then(|inputs| inputs.get("request")))
}

pub fn schema() -> serde_json::Map<String, Value> {
    match serde_json::json!({
        "type": "object",
        "properties": {
            "response_text": {
                "type": "string",
                "description": "The exact text of a response as it was received, ending with the receipt, and the engine's signature after it when it carries one."
            },
            "public_key": { "type": "string", "description": "The base64 of the ML-DSA-87 public key (2,592 bytes) to check the answer's signature with; without it, this engine's own." },
            "seen_nonces": { "type": "array", "items": { "type": "string", "pattern": "^[0-9a-f]{64}$" }, "description": "Nonces of signatures already seen: one of them again is SignatureNonceReplayed." }
        },
        "required": ["response_text"],
        "additionalProperties": false,
        "description": "The text of one response from compute_step, run_trajectory or solve, or the head of a dataset written as its JSON text. A signature after the receipt is checked against public_key or else this engine's own key, and reported under signature."
    }) {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}
