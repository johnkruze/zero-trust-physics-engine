//! `validate_series`: one run and a caller's measurements of one quantity,
//! the error in numbers, sealed with a hash of the measurements.
//!
//! The run is `run_trajectory`'s own: the same `prepare` and the same loop,
//! watched by an observer that reads the exact value of the quantity after each
//! measured step that lands. The statistics, the hash of the measurements and
//! the seal are `genesis_seal::validation`, the one place the rule is written.
//! The answer sets no threshold. A request may open a commitment its
//! measurements were locked with (`genesis_seal::commitment`): it is opened
//! before a permit, data that do not open it are refused by name, and the
//! seal covers it.

use genesis_seal::commitment::{self, Commitment};
use genesis_seal::validation::{
    DOES_NOT_SHOW, SealParts, Statistics, bits_hex, measurements_sha384, statistics,
    validation_seal,
};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::descriptor;
use crate::trajectory::{self, Prepared};
use crate::{DoorError, Live, QuantityKind};

/// The most measurements one request holds. It is the number of frames a run
/// may record, so an answer is never larger than a run's. A request of this many
/// is some 400 KB, far under the 16 MiB a body may be.
pub const MAX_MEASUREMENTS: usize = 10_000;

pub(crate) const TOP_LEVEL: &[&str] = &["run", "quantity", "component", "measurements", commitment::COMMITMENT];

/// What the input schema says of `commitment`, for a run and a plan of solves.
pub(crate) const COMMITMENT_SAYS: &str = "Optional: a commitment the measurements were locked with before this comparison, opened by this request. sha384 is the SHA-384 of these bytes: the line zero-trust-physics/measurement-commitment/v2 and a line break; the sealed form (the text a request's seal is taken over: keys sorted by their bytes, each number x and the 16 hex digits of its 64-bit float, no spaces) of {\"component\": <this request's component, or null>, \"measurements\": <this request's measurements>, \"quantity\": <this request's quantity>}; a line break; and the salt's bytes, its hex decoded. salt is 32 to 128 lowercase hex digits, 16 to 64 bytes, kept back until the comparison so that the data are not found from the sha384 by trying values. The engine makes the digest again from the request before computing: data that do not open it (a value, a step or a row changed, moved, added or dropped, another quantity or another salt) are refused as CommitmentDoesNotMatch, a salt shorter than 16 bytes as CommitmentSaltTooShort, a field of the wrong form as CommitmentMalformed, and sha384 or salt left out as InputMissing; nothing is computed and nothing is sealed. Opened, the comparison runs as it runs without one, validation_seal takes the commitment as a sixth part, and the answer says so in commitment_opened. A commitment shows that the data were not changed after it was made; when it was made is shown by where its sha384 was published before the comparison.";

/// The commitment a request carries, read and opened by its measurements,
/// quantity and component, before a permit. `None` when it carries none.
pub(crate) fn open_commitment(
    object: &Map<String, Value>,
    quantity: &str,
    measurements: &Value,
) -> Result<Option<Commitment>, DoorError> {
    let refused = |fault: commitment::CommitmentFault| DoorError::Unprocessable(fault.to_string());
    let Some(commitment) = commitment::read_in(object, commitment::COMMITMENT).map_err(refused)? else {
        return Ok(None);
    };
    let component = object.get("component").unwrap_or(&Value::Null);
    commitment.open(quantity, component, measurements).map_err(refused)?;
    Ok(Some(commitment))
}

/// The input schema of `commitment`.
pub(crate) fn commitment_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "sha384": { "type": "string", "pattern": "^[0-9a-f]{96}$", "description": "The SHA-384 committed, 96 lowercase hex digits." },
            "salt": { "type": "string", "description": "The salt the sha384 was made with: 32 to 128 lowercase hex digits, 16 to 64 bytes." }
        },
        "required": commitment::FIELDS,
        "additionalProperties": false,
        "description": COMMITMENT_SAYS
    })
}

struct Plan {
    request: Value,
    run: Prepared,
    quantity: String,
    component: Option<usize>,
    steps: Vec<u64>,
    values: Vec<f64>,
    measurements: Value,
    /// Where the measurements came from: signed by a key, or the caller's.
    measurement_source: Value,
    /// The commitment the measurements opened, if the request carries one.
    commitment: Option<Commitment>,
}

#[derive(Serialize)]
struct Document<'a> {
    inputs: &'a Value,
    engine_version: &'a str,
    outcome: &'a str,
    steps_landed: u64,
    reject: &'a Option<String>,
    reject_detail: &'a Option<String>,
    run_seal: &'a str,
    compared: u64,
    not_reached: u64,
    #[serde(serialize_with = "trajectory::serialize_optional_f64")]
    rms_error: Option<f64>,
    rms_error_hex: Option<String>,
    #[serde(serialize_with = "trajectory::serialize_optional_f64")]
    mean_signed_error: Option<f64>,
    mean_signed_error_hex: Option<String>,
    #[serde(serialize_with = "trajectory::serialize_optional_f64")]
    max_abs_error: Option<f64>,
    max_abs_error_hex: Option<String>,
    max_abs_error_step: Option<u64>,
    model_values: Vec<crate::readout::Exact>,
    model_values_hex: &'a [String],
    load_source: &'a Value,
    measurement_source: &'a Value,
    measurements_sha384: &'a str,
    /// Present only when the request opened a commitment, so an answer
    /// without one has the bytes it had before commitments were taken.
    #[serde(skip_serializing_if = "Option::is_none")]
    commitment_opened: Option<&'static str>,
    validation_seal: &'a str,
    does_not_show: &'static str,
    seal_rule: &'static str,
}

/// The answer as JSON text, with its receipt. Both doors send these bytes.
pub async fn run(state: &crate::AppState, request: Value) -> Result<String, DoorError> {
    // A comparison of solves names a plan of solves in place of a run.
    if request.get("solves").is_some() && request.get("run").is_none() {
        return crate::solve_plan::validate(state, request).await;
    }
    let plan = prepare_in(state.view(), request)?;
    crate::on_pool(state, move || execute(plan)).await
}

fn unprocessable(message: impl Into<String>) -> DoorError {
    DoorError::Unprocessable(message.into())
}

fn prepare_in(view: &crate::view::View, request: Value) -> Result<Plan, DoorError> {
    let object = request
        .as_object()
        .ok_or_else(|| unprocessable("Physical Violation: the request must be a JSON object."))?;
    for key in object.keys() {
        if !TOP_LEVEL.contains(&key.as_str()) {
            return Err(DoorError::Unprocessable(crate::locked_or_unknown(key)));
        }
    }
    for key in ["run", "quantity", "measurements"] {
        if !object.contains_key(key) {
            return Err(unprocessable(format!("Missing required parameter: '{key}' (InputMissing).")));
        }
    }
    let run_request = object["run"].clone();
    // The run is refused in the sentences of run_trajectory, a 404 for an unknown pair included.
    let run = trajectory::prepare_in(view, run_request.clone())?;
    let (domain, card) = {
        let run_object = run_request.as_object().expect("a prepared run is an object");
        (
            run_object["domain"].as_str().expect("a prepared run names its domain").to_string(),
            run_object["card"].as_str().expect("a prepared run names its card").to_string(),
        )
    };

    let quantity = match &object["quantity"] {
        Value::String(text) if !text.is_empty() => text.clone(),
        Value::String(_) => return Err(unprocessable("Missing required parameter: 'quantity' (InputMissing).")),
        _ => return Err(unprocessable("Physical Violation: 'quantity' must be a string.")),
    };
    let kind = crate::state_field_kind(&domain, &card, &quantity)
        .expect("a prepared run names a card");
    let component = match kind {
        None => {
            return Err(unprocessable(if genesis_seal::json_text::is_plain_field_name(&quantity) {
                format!(
                    "Physical Violation: 'quantity' is '{quantity}', which the {card} card does not publish."
                )
            } else {
                format!("Physical Violation: 'quantity' is not a field the {card} card publishes.")
            }));
        }
        Some(QuantityKind::Flag) => {
            return Err(unprocessable(format!(
                "Physical Violation: 'quantity' is '{quantity}', a flag. A flag is not a number and has no value to compare."
            )));
        }
        Some(QuantityKind::Number) => match object.get("component") {
            None => None,
            Some(_) => {
                return Err(unprocessable(format!(
                    "Physical Violation: 'component' is refused: '{quantity}' is one number, not a 3-vector."
                )));
            }
        },
        Some(QuantityKind::Vector3) => match object.get("component") {
            None => {
                return Err(unprocessable("Missing required parameter: 'component' (InputMissing)."));
            }
            Some(value) => match value.as_u64() {
                Some(index) if index <= 2 => Some(index as usize),
                _ => {
                    return Err(unprocessable(
                        "Physical Violation: 'component' must be 0, 1 or 2.",
                    ));
                }
            },
        },
    };

    let items = object["measurements"]
        .as_array()
        .ok_or_else(|| unprocessable("Physical Violation: 'measurements' must be a JSON array."))?;
    if items.is_empty() {
        return Err(unprocessable(
            "Physical Violation: 'measurements' must contain at least one measurement.",
        ));
    }
    if items.len() > MAX_MEASUREMENTS {
        return Err(unprocessable(format!(
            "Physical Violation: 'measurements' holds more than {MAX_MEASUREMENTS} measurements."
        )));
    }
    let total = run.steps_requested();
    let mut steps = Vec::with_capacity(items.len());
    let mut values = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let label = format!("measurements[{index}]");
        let item = item.as_object().ok_or_else(|| {
            unprocessable(format!("Physical Violation: '{label}' must be a JSON object."))
        })?;
        for key in item.keys() {
            if key != "step" && key != "value" && key != crate::signed_load::SIGNED {
                return Err(DoorError::Unprocessable(crate::locked_or_unknown(key)));
            }
        }
        for key in ["step", "value"] {
            if !item.contains_key(key) {
                return Err(unprocessable(format!(
                    "Missing required parameter: '{label}.{key}' (InputMissing)."
                )));
            }
        }
        let step = match item["step"].as_u64() {
            Some(step) if step >= 1 => step,
            _ => {
                return Err(unprocessable(format!(
                    "Physical Violation: '{label}.step' must be an integer >= 1."
                )));
            }
        };
        if let Some(&before) = steps.last() {
            if step <= before {
                return Err(unprocessable(format!(
                    "Physical Violation: '{label}.step' must be greater than the step before it."
                )));
            }
        }
        if step > total {
            return Err(unprocessable(format!(
                "Physical Violation: '{label}.step' is {step}, past the {total} steps the run asks for."
            )));
        }
        let value = item["value"].as_f64().ok_or_else(|| {
            unprocessable(format!("Physical Violation: '{label}.value' must be a number."))
        })?;
        crate::accept_f64(&format!("{label}.value"), value).map_err(DoorError::Unprocessable)?;
        steps.push(step);
        values.push(value);
    }
    let measurements = object["measurements"].clone();
    // A commitment is opened by the measurements as sent, before a permit.
    let commitment = open_commitment(object, &quantity, &measurements)?;
    // A measurement may carry an instrument's signature over its step and
    // value, verified here, before a permit, as a run's loads are.
    let readings = crate::signed_load::verify_measurements(&measurements)?;
    let measurement_source = crate::signed_load::measurement_source(&measurements, &readings);
    Ok(Plan {
        request,
        run,
        quantity,
        component,
        steps,
        values,
        measurements,
        measurement_source,
        commitment,
    })
}

/// The exact value of `quantity` in the published state of `live`.
fn read_quantity(live: &Live, quantity: &str, component: Option<usize>) -> Result<f64, String> {
    let state = live.publish().map_err(|failure| failure.detail)?;
    let hex = state
        .get(format!("{quantity}_hex"))
        .ok_or_else(|| "Physical Violation: the state does not carry the quantity.".to_string())?;
    let text = match component {
        None => hex.as_str(),
        Some(index) => hex.get(index).and_then(Value::as_str),
    }
    .ok_or_else(|| "Physical Violation: the quantity is not in the state as expected.".to_string())?;
    u64::from_str_radix(text, 16)
        .map(f64::from_bits)
        .map_err(|_| "Physical Violation: the quantity is not a 64-bit value.".to_string())
}

fn execute(plan: Plan) -> Result<String, String> {
    let Plan {
        request,
        run,
        quantity,
        component,
        steps,
        values,
        measurements,
        measurement_source,
        commitment,
    } = plan;
    let mut model: Vec<f64> = Vec::new();
    let mut observe = |landed: u64, live: &Live| -> Result<(), String> {
        // Steps land one at a time and the measured steps rise, so the next
        // measured step is the only one that can land now.
        if model.len() < steps.len() && steps[model.len()] == landed {
            model.push(read_quantity(live, &quantity, component)?);
        }
        Ok(())
    };
    let response = trajectory::execute_observed(run, Some(&mut observe))?;

    let compared = model.len();
    let measured = &values[..compared];
    let landed_steps = &steps[..compared];
    let stats: Option<Statistics> =
        statistics(landed_steps, &model, measured).map_err(|error| error.to_string())?;
    let model_values_hex: Vec<String> = model.iter().map(|value| bits_hex(*value)).collect();
    let hash = measurements_sha384(&measurements);
    let not_reached = (steps.len() - compared) as u64;
    let seal = validation_seal(&SealParts {
        run_seal: &response.run_seal,
        measurements_sha384: &hash,
        quantity: &quantity,
        component: component.map(|index| index as u64),
        outcome: &response.outcome,
        steps_landed: response.steps_landed,
        reject: response.reject.as_deref(),
        compared: compared as u64,
        not_reached,
        statistics: stats.as_ref(),
        model_values_hex: &model_values_hex,
        commitment: commitment.as_ref(),
    });
    let document = serde_json::to_string(&Document {
        inputs: &request,
        engine_version: &response.engine_version,
        outcome: &response.outcome,
        steps_landed: response.steps_landed,
        reject: &response.reject,
        reject_detail: &response.reject_detail,
        run_seal: &response.run_seal,
        compared: compared as u64,
        not_reached,
        rms_error: stats.map(|s| s.rms_error),
        rms_error_hex: stats.map(|s| s.rms_error_hex()),
        mean_signed_error: stats.map(|s| s.mean_signed_error),
        mean_signed_error_hex: stats.map(|s| s.mean_signed_error_hex()),
        max_abs_error: stats.map(|s| s.max_abs_error),
        max_abs_error_hex: stats.map(|s| s.max_abs_error_hex()),
        max_abs_error_step: stats.map(|s| s.max_abs_error_step),
        model_values: model.iter().map(|value| crate::readout::Exact(*value)).collect(),
        model_values_hex: &model_values_hex,
        load_source: &response.load_source,
        measurement_source: &measurement_source,
        measurements_sha384: &hash,
        commitment_opened: commitment.as_ref().map(|_| commitment::OPENED),
        validation_seal: &seal,
        does_not_show: DOES_NOT_SHOW,
        seal_rule: crate::SEAL_RULE,
    })
    .map_err(|error| format!("Physical Violation: the response could not be encoded. {error}"))?;
    let receipt = crate::digest_hex(document.as_bytes());
    Ok(crate::append_receipt(&document, &receipt))
}

pub fn schema_in(view: &crate::view::View) -> Map<String, Value> {
    let run = descriptor::run_request_schema_in(view);
    match json!({
        "type": "object",
        "properties": {
            "run": run,
            "solves": {
                "type": "object",
                "description": format!("In place of run: a plan of solves as run_dataset takes it (solver and plan, a list or a grid, at most {} solves). quantity is then one number the solver publishes in its result, and each measurement names a row of the plan, numbered from 0, in place of a step.", crate::solve_plan::MAX_SOLVES)
            },
            "quantity": {
                "type": "string",
                "description": "The name of one number the card publishes in its state, a plain name that has a '_hex' sibling in the state. A flag is refused."
            },
            "component": {
                "type": "integer",
                "minimum": 0,
                "maximum": 2,
                "description": "0, 1 or 2: which of the three components of a 3-vector quantity is compared. Required when the quantity is a 3-vector, and refused when it is not. Any other value is refused."
            },
            "measurements": {
                "type": "array",
                "minItems": 1,
                "maxItems": MAX_MEASUREMENTS,
                "items": {
                    "type": "object",
                    "properties": {
                        "step": { "type": "integer", "minimum": 1, "description": "With run: the landed step the value is compared with. The steps rise strictly and none is past the run's total." },
                        "row": { "type": "integer", "minimum": 0, "description": "With solves, in place of step: the row of the plan whose solve the value is compared with, numbered from 0. The rows rise strictly." },
                        "value": { "type": "number", "description": "The measurement, in the unit of the quantity." },
                        "signed": crate::signed_load::schema()
                    },
                    "required": ["value"],
                    "additionalProperties": false
                },
                "description": format!("At most {MAX_MEASUREMENTS}. A measurement is compared with the state after landed step k. There is no interpolation and no time matching: a measurement is placed by its step, its time since the start divided by dt_s.")
            },
            "commitment": commitment_schema()
        },
        "required": ["quantity", "measurements"],
        "additionalProperties": false,
        "description": "One run, or one plan of solves, and measurements of one quantity. The answer is the error in numbers, sealed with a hash of the measurements. It sets no threshold. A request holds run or solves, not both."
    }) {
        Value::Object(map) => map,
        _ => unreachable!("schema is an object"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use std::time::Duration;

    fn request(step: u64, quantity: &str) -> Value {
        json!({
            "run": {
                "domain": "energy",
                "card": "thermal",
                "initial": { "temperature_k": 293.15 },
                "dt_s": 0.1,
                "segments": [{ "steps": 5, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
                "record_every": 1
            },
            "quantity": quantity,
            "measurements": [{ "step": step, "value": 294.0 }]
        })
    }

    /// With the one permit held, a request that is refused is still a 422 and one
    /// that would run is a 429. The refusals all come before the permit is asked for.
    #[tokio::test]
    async fn every_refusal_comes_before_the_permit_and_a_full_pool_is_429() {
        let state = AppState::with_hold(1, Duration::from_millis(600));
        let holder = {
            let state = state.clone();
            tokio::spawn(async move { run(&state, request(3, "temperature_k")).await })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(matches!(
            run(&state, request(6, "temperature_k")).await,
            Err(DoorError::Unprocessable(_))
        ));
        assert!(matches!(
            run(&state, request(3, "no_such_field")).await,
            Err(DoorError::Unprocessable(_))
        ));
        match run(&state, request(3, "temperature_k")).await {
            Err(DoorError::TooMany(message)) => {
                assert_eq!(message, "Too many concurrent physics tasks.");
            }
            _ => panic!("a full pool is 429"),
        }
        assert!(holder.await.unwrap().is_ok());
        // The permit went back.
        assert!(run(&state, request(3, "temperature_k")).await.is_ok());
    }

    /// A commitment that the measurements do not open, and one of the wrong
    /// form, are refused before a permit is asked for: with the one permit
    /// held each is still a 422, and the one that opens is a 429.
    #[tokio::test]
    async fn a_commitment_is_opened_before_the_permit() {
        let state = AppState::with_hold(1, Duration::from_millis(600));
        let holder = {
            let state = state.clone();
            tokio::spawn(async move { run(&state, request(3, "temperature_k")).await })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        let salt = "000102030405060708090a0b0c0d0e0f";
        let measurements = request(3, "temperature_k")["measurements"].clone();
        let sha384 = commitment::commitment_sha384("temperature_k", &Value::Null, &measurements, salt);
        let committed = |sha384: &str, salt: &str| {
            let mut body = request(3, "temperature_k");
            body["commitment"] = json!({ "sha384": sha384, "salt": salt });
            body
        };
        for (body, name) in [
            (committed(&"ab".repeat(48), salt), commitment::DOES_NOT_MATCH),
            (committed(&sha384, "00"), commitment::SALT_TOO_SHORT),
            (committed("ab", salt), commitment::MALFORMED),
        ] {
            match run(&state, body).await {
                Err(DoorError::Unprocessable(said)) => assert!(said.ends_with(&format!("({name}).")), "{said}"),
                _ => panic!("{name} is refused before the permit"),
            }
        }
        assert!(matches!(run(&state, committed(&sha384, salt)).await, Err(DoorError::TooMany(_))));
        assert!(holder.await.unwrap().is_ok());
        assert!(run(&state, committed(&sha384, salt)).await.is_ok());
    }
}
