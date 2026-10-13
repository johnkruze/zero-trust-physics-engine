//! The check of a receipt: `POST /verify_receipt` and the MCP tool of the same name.
//!
//! The rule is `genesis_seal::receipt`, the one the dataset head is checked by.
//! The text of a response must end with `,"receipt_sha256":"<64 lowercase hex>"}`
//! as `append_receipt` writes it. The check takes no permit and runs no physics.

use genesis_seal::receipt::read_receipt;
use serde::Serialize;
use serde_json::Value;

use crate::DoorError;

const COVERS: &str =
    "every byte of the response before the receipt, the decimal values included.";
const DOES_NOT_SHOW: &str = "who produced the response or when. The receipt has no key. Replaying the request through the engine shows the engine produced it.";

#[derive(Serialize)]
pub struct ReceiptReport {
    pub matches: bool,
    pub receipt_sha256: String,
    pub recomputed_sha256: String,
    pub covers: &'static str,
    pub does_not_show: &'static str,
}

/// `{"response_text": "<the exact text of a response>"}` to the report.
pub fn check(body: &Value) -> Result<ReceiptReport, DoorError> {
    let object = body.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    for key in object.keys() {
        if key != "response_text" {
            return Err(DoorError::Unprocessable(crate::locked_or_unknown(key)));
        }
    }
    let text = match object.get("response_text") {
        None => {
            return Err(DoorError::Unprocessable(
                "Missing required parameter: 'response_text'".to_string(),
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
            "Physical Violation: the text does not end with a receipt as this gateway writes one."
                .to_string(),
        )
    })?;
    Ok(ReceiptReport {
        matches: read.matches(),
        receipt_sha256: read.stated,
        recomputed_sha256: read.recomputed,
        covers: COVERS,
        does_not_show: DOES_NOT_SHOW,
    })
}

pub fn schema() -> serde_json::Map<String, Value> {
    match serde_json::json!({
        "type": "object",
        "properties": {
            "response_text": {
                "type": "string",
                "description": "The exact text of a response as it was received, ending with the receipt."
            }
        },
        "required": ["response_text"],
        "additionalProperties": false,
        "description": "The text of one response from verify_step, run_trajectory or solve, or the head of a dataset written as its JSON text."
    }) {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}
