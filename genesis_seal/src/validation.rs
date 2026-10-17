//! The seal of a validation: how far one run was from a caller's measurements.
//!
//! The answer of `validate_series` holds the errors of a run's model against
//! measurements of one quantity, the exact model value at every compared step,
//! a hash of the measurements, and one seal over all of it. This module holds
//! the three rules the gateway and every check share: the statistics, the hash
//! of the measurements and the validation seal. `check_validation` rebuilds
//! them from an answer alone. No engine is needed, and no verdict is made:
//! the answer holds no pass line. A comparison may open a commitment to its
//! measurements (`crate::commitment`); the seal then covers the commitment,
//! and the check opens it again from the answer's own inputs.

use serde_json::{Map, Value};

use crate::commitment::{self, Commitment};
use crate::json_text::{self, is_lower_hex};
use crate::refusal::Refusal;
use crate::sealed_form::{digest_hex, json_string, sealed_form};

/// The outcome a comparison of a plan of solves is sealed with.
pub const SOLVES_OUTCOME: &str = "solves";

/// The sentence every answer carries in `does_not_show`.
pub const DOES_NOT_SHOW: &str = "The numbers say how far this run was from these measurements. They are not a verdict. The seal has no key, so it does not show who measured or who computed.";

/// The errors of the model against the measurements, model minus measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Statistics {
    pub rms_error: f64,
    pub mean_signed_error: f64,
    pub max_abs_error: f64,
    /// The first step, in step order, at which the greatest absolute error is reached.
    pub max_abs_error_step: u64,
}

impl Statistics {
    pub fn rms_error_hex(&self) -> String {
        bits_hex(self.rms_error)
    }
    pub fn mean_signed_error_hex(&self) -> String {
        bits_hex(self.mean_signed_error)
    }
    pub fn max_abs_error_hex(&self) -> String {
        bits_hex(self.max_abs_error)
    }
}

/// A statistic left the range of a 64-bit float.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotFinite;

impl std::fmt::Display for NotFinite {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            out,
            "Physical Violation: a statistic of the errors is not a finite number. Nothing is sealed."
        )
    }
}

/// 16 lowercase hex digits of the bits of `value`.
pub fn bits_hex(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// The statistics of the errors, `model - measured`, at `steps`, in step order.
///
/// The rule, so that two implementations agree to the bit:
///
/// 1. The error of item i is the one subtraction `model[i] - measured[i]`.
/// 2. `sum` and `sum_of_squares` start at positive zero and take the items in
///    step order, each by one plain 64-bit addition. The square is its own
///    multiplication, `e * e`, taken before it is added. There is no fused
///    multiply-add and no other order of summing.
/// 3. `mean_signed_error` is `sum / n`, one division, where `n` is the count as a 64-bit float.
/// 4. `rms_error` is the square root of `sum_of_squares / n`: one division, then `sqrt`.
/// 5. `max_abs_error` is the greatest `|e|`, and `max_abs_error_step` is the
///    first step at which it is reached: a later item replaces it only when
///    its `|e|` is strictly greater.
///
/// `None` when there is nothing to compare. The three slices must be of one length.
pub fn statistics(
    steps: &[u64],
    model: &[f64],
    measured: &[f64],
) -> Result<Option<Statistics>, NotFinite> {
    assert_eq!(steps.len(), model.len());
    assert_eq!(steps.len(), measured.len());
    if steps.is_empty() {
        return Ok(None);
    }
    let mut sum = 0.0_f64;
    let mut sum_of_squares = 0.0_f64;
    let mut max_abs = 0.0_f64;
    let mut max_step = steps[0];
    for index in 0..steps.len() {
        let error = model[index] - measured[index];
        sum += error;
        let square = error * error;
        sum_of_squares += square;
        let magnitude = error.abs();
        if index == 0 || magnitude > max_abs {
            max_abs = magnitude;
            max_step = steps[index];
        }
    }
    let count = steps.len() as f64;
    let mean = sum / count;
    let mean_square = sum_of_squares / count;
    let rms = mean_square.sqrt();
    if !(mean.is_finite() && rms.is_finite() && max_abs.is_finite()) {
        return Err(NotFinite);
    }
    Ok(Some(Statistics {
        rms_error: rms,
        mean_signed_error: mean,
        max_abs_error: max_abs,
        max_abs_error_step: max_step,
    }))
}

/// SHA-256 of the sealed form of the `measurements` array, as 64 lowercase hex digits.
pub fn measurements_sha256(measurements: &Value) -> String {
    digest_hex(sealed_form(measurements).as_bytes())
}

/// What the validation seal is taken over.
pub struct SealParts<'a> {
    pub run_seal: &'a str,
    pub measurements_sha256: &'a str,
    pub quantity: &'a str,
    pub component: Option<u64>,
    /// How the run ended: the answer's `outcome`.
    pub outcome: &'a str,
    /// Steps that landed: the answer's `steps_landed`.
    pub steps_landed: u64,
    /// The name of the refusal that stopped the run, or none: the answer's `reject`.
    pub reject: Option<&'a str>,
    pub compared: u64,
    pub not_reached: u64,
    pub statistics: Option<&'a Statistics>,
    pub model_values_hex: &'a [String],
    /// The commitment the comparison opened, or none. Without one the seal
    /// is taken over the five parts alone, as before commitments were taken.
    pub commitment: Option<&'a Commitment>,
}

/// The validation seal: SHA-256 over these bytes, in this order, with nothing between them.
///
/// 1. `run_seal`, 64 hex characters.
/// 2. `measurements_sha256`, 64 hex characters.
/// 3. The compact JSON `{"quantity","component","outcome","steps_landed","reject","compared","not_reached","max_abs_error_step"}`,
///    the keys in that order, no spaces. `component`, `reject` and `max_abs_error_step` are
///    null when there is none. `quantity`, `outcome` and `reject` are JSON strings.
/// 4. The three lines `rms_error_hex:<hex>\n`, `mean_signed_error_hex:<hex>\n` and
///    `max_abs_error_hex:<hex>\n`, each hex being 16 digits, or the word `null` when
///    nothing was compared.
/// 5. SHA-256 of the `model_values_hex` joined by commas, as 64 hex characters. For
///    no value it is the hash of the empty text.
/// 6. Only when the comparison opened a commitment: the two lines
///    `commitment_sha256:<64 hex>\n` and `commitment_salt:<hex>\n`. A
///    comparison without one has no sixth part, so its seal is the seal of
///    the five.
pub fn validation_seal(parts: &SealParts) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(parts.run_seal.as_bytes());
    bytes.extend_from_slice(parts.measurements_sha256.as_bytes());
    let component = match parts.component {
        Some(component) => component.to_string(),
        None => "null".to_string(),
    };
    let step = match parts.statistics {
        Some(statistics) => statistics.max_abs_error_step.to_string(),
        None => "null".to_string(),
    };
    let reject = match parts.reject {
        Some(name) => json_string(name),
        None => "null".to_string(),
    };
    bytes.extend_from_slice(
        format!(
            "{{\"quantity\":{},\"component\":{component},\"outcome\":{},\"steps_landed\":{},\"reject\":{reject},\"compared\":{},\"not_reached\":{},\"max_abs_error_step\":{step}}}",
            json_string(parts.quantity),
            json_string(parts.outcome),
            parts.steps_landed,
            parts.compared,
            parts.not_reached
        )
        .as_bytes(),
    );
    let (rms, mean, max) = match parts.statistics {
        Some(statistics) => (
            statistics.rms_error_hex(),
            statistics.mean_signed_error_hex(),
            statistics.max_abs_error_hex(),
        ),
        None => ("null".to_string(), "null".to_string(), "null".to_string()),
    };
    bytes.extend_from_slice(format!("rms_error_hex:{rms}\n").as_bytes());
    bytes.extend_from_slice(format!("mean_signed_error_hex:{mean}\n").as_bytes());
    bytes.extend_from_slice(format!("max_abs_error_hex:{max}\n").as_bytes());
    bytes.extend_from_slice(digest_hex(parts.model_values_hex.join(",").as_bytes()).as_bytes());
    if let Some(commitment) = parts.commitment {
        bytes.extend_from_slice(commitment.seal_part().as_bytes());
    }
    digest_hex(&bytes)
}

/// The first part of an answer that does not follow from the rest, in the order checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationCause {
    /// `measurements_sha256` is not the hash of the measurements in `inputs`.
    Measurements,
    /// `inputs` carries a commitment that its measurements, quantity and
    /// component do not open: the digest of them and the salt is not the
    /// commitment's `sha256`.
    Commitment,
    /// Every sealed part holds, and the answer says `commitment_opened` with
    /// no commitment in its `inputs`, or carries one in `inputs` and does not
    /// say so: the gateway writes the sentence exactly when it opened a
    /// commitment. The seal does not cover the sentence; the receipt covers
    /// it as it covers every byte, so with the receipt written again only
    /// this names it.
    CommitmentOpened,
    /// A signed reading of the run's request or of a measurement does not
    /// verify, or is not well formed (`crate::attest`).
    LoadSignature,
    /// `compared`, `not_reached` or the count of model values is not what the measurements and `steps_landed` give.
    Counts,
    /// A stated statistic is not the one the model values and the measurements give.
    Statistics,
    /// The model values are not the ones the statistics and the seal were made from.
    ModelValues,
    /// Every part follows from the rest and `validation_seal` is not the seal of them.
    ValidationSeal,
}

impl ValidationCause {
    pub fn as_str(self) -> &'static str {
        match self {
            ValidationCause::Measurements => "measurements",
            ValidationCause::Commitment => "commitment",
            ValidationCause::CommitmentOpened => "commitment_opened",
            ValidationCause::LoadSignature => "load_signature",
            ValidationCause::Counts => "counts",
            ValidationCause::Statistics => "statistics",
            ValidationCause::ModelValues => "model_values",
            ValidationCause::ValidationSeal => "validation_seal",
        }
    }
}

/// What `check_validation` found, with the parts it rebuilt.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationReport {
    pub holds: bool,
    pub cause: Option<ValidationCause>,
    pub compared: u64,
    pub not_reached: u64,
    pub rebuilt: Option<Statistics>,
    pub measurements_sha256: String,
    pub validation_seal: String,
    /// With `load_signature`: where the reading that does not verify is, and
    /// the sentence of why.
    pub broken_place: Option<String>,
    pub load_fault: Option<String>,
    /// The verified signed readings: of the run's request, then of the
    /// measurements, each in place order.
    pub signed_loads: Vec<crate::attest::Verified>,
    pub signed_measurements: Vec<crate::attest::Verified>,
    /// With a commitment in `inputs`: its `sha256` as the answer carries it,
    /// the one to hold beside the sha256 published before the comparison.
    pub commitment_sha256: Option<String>,
    /// With a commitment: the SHA-256 of the committed bytes, made again from
    /// the answer's measurements, quantity, component and salt.
    pub commitment_recomputed: Option<String>,
    /// The answer says `commitment_opened` (a value other than null).
    pub commitment_opened: bool,
}

/// Rebuild the statistics, the hash of the measurements and the validation
/// seal from the text of an answer alone, and name the first part that fails.
///
/// The answer is read with the scan every route uses. It must carry `inputs`
/// (with `quantity`, `component` when the quantity is a vector, and
/// `measurements`), `steps_landed`, `run_seal`, `compared`, `not_reached`, the
/// three statistics each with its `_hex`, `max_abs_error_step`,
/// `model_values_hex`, `measurements_sha256` and `validation_seal`. Other keys
/// are ignored, so a whole answer can be posted as it is. Only the `_hex`
/// values are read; a decimal copy is covered by the receipt.
///
/// A comparison of a plan of solves is read the same way, with `dataset_seal`
/// in the place of `run_seal`: each measurement names a `row` in the place of
/// a `step`, `not_reached_rows` lists the measured rows whose solve did not
/// converge, `converged` stands in the seal in the place of `steps_landed`,
/// the outcome is `solves`, the reject is null, and `max_abs_error_row` is
/// read in the place of `max_abs_error_step`. An answer that carries
/// `dataset_seal` and no `run_seal` is read as one.
///
/// The check cannot rebuild `run_seal`: the answer carries no frames. Replaying
/// `inputs.run` through `run_trajectory` shows the run seal, and so shows that
/// the model values are those of the engine. This check shows only that the
/// answer was not altered after it was sealed.
///
/// Where the causes are told apart: when the seal of the rebuilt statistics and
/// the stated model values is the stated seal, the model values are the sealed
/// ones, and a different stated statistic is `statistics`. When it is not, and
/// the stated statistics are not the rebuilt ones, the model values are not the
/// ones sealed: `model_values`. When the stated statistics are the rebuilt ones
/// and the seal still fails, it is `validation_seal`.
///
/// After the hash of the measurements, a commitment that `inputs` carries is
/// opened again from the answer's own measurements, quantity and component
/// (`crate::commitment`): one they do not open is `commitment`, and the seal
/// is taken with the commitment as its sixth part. Last, when every sealed
/// part holds: an answer that says `commitment_opened` with no commitment in
/// `inputs`, or carries one in `inputs` and does not say so, is
/// `commitment_opened`. A commitment that is not
/// of the commitment's form is refused, not read. Then every signed reading
/// is verified: the run's loads and initial, then each measurement that
/// carries `signed`. A reading that does not verify is `load_signature`. The
/// hash covers each measurement's signature, since it is taken over the
/// array's text.
pub fn check_validation(bytes: &[u8]) -> Result<ValidationReport, Refusal> {
    let value = json_text::read(bytes)?;
    let answer = value.as_object().ok_or(Refusal::NotAnObject)?;
    let inputs = object_of(answer, "inputs")?;
    let quantity = string_of(inputs, "inputs.quantity", "quantity")?;
    let component = match inputs.get("component") {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.as_u64().ok_or_else(|| {
            wrong("inputs.component", "0, 1 or 2")
        })?),
    };
    let measurements = array_of(inputs, "inputs.measurements", "measurements")?;
    let commitment = commitment::read_in(inputs, "inputs.commitment").map_err(Refusal::Commitment)?;
    // A comparison of solves carries the dataset seal of its solves and no run seal.
    let solves = answer.contains_key("dataset_seal") && !answer.contains_key("run_seal");
    let (steps_landed, outcome, reject, run_seal, not_reached_rows) = if solves {
        let rows = array_of(answer, "not_reached_rows", "not_reached_rows")?;
        let mut listed = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            listed.push(row.as_u64().ok_or_else(|| {
                wrong(&format!("not_reached_rows[{index}]"), "an integer >= 0")
            })?);
        }
        (
            whole_of(answer, "converged")?,
            SOLVES_OUTCOME.to_string(),
            None,
            hash_of(answer, "dataset_seal")?,
            Some(listed),
        )
    } else {
        let reject = match answer.get("reject") {
            Some(Value::Null) => None,
            Some(Value::String(name)) => Some(name.clone()),
            None => return Err(Refusal::Missing("reject".to_string())),
            Some(_) => return Err(wrong("reject", "a string or null")),
        };
        (
            whole_of(answer, "steps_landed")?,
            string_of(answer, "outcome", "outcome")?,
            reject,
            hash_of(answer, "run_seal")?,
            None,
        )
    };
    let place = if solves { "row" } else { "step" };
    let stated_hash = hash_of(answer, "measurements_sha256")?;
    let stated_seal = hash_of(answer, "validation_seal")?;
    let stated_compared = whole_of(answer, "compared")?;
    let stated_not_reached = whole_of(answer, "not_reached")?;
    let model_values_hex = hex_list_of(answer, "model_values_hex")?;

    let mut steps = Vec::new();
    let mut measured = Vec::new();
    let mut later = 0_u64;
    for (index, item) in measurements.iter().enumerate() {
        let label = format!("inputs.measurements[{index}]");
        let item = item.as_object().ok_or_else(|| wrong(&label, "a JSON object"))?;
        let step = item
            .get(place)
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                wrong(
                    &format!("{label}.{place}"),
                    if solves { "an integer >= 0" } else { "an integer >= 1" },
                )
            })?;
        let number = item
            .get("value")
            .and_then(Value::as_f64)
            .ok_or_else(|| wrong(&format!("{label}.value"), "a number"))?;
        let reached = match &not_reached_rows {
            Some(rows) => !rows.contains(&step),
            None => step <= steps_landed,
        };
        if reached {
            steps.push(step);
            measured.push(number);
        } else {
            later += 1;
        }
    }
    // Every row listed as not reached is a measured row, listed once.
    let listed_hold = match &not_reached_rows {
        Some(rows) => {
            rows.len() as u64 == later
                && rows.windows(2).all(|pair| pair[0] < pair[1])
        }
        None => true,
    };

    let computed_hash = measurements_sha256(&Value::Array(measurements.clone()));
    let stated_statistics = stated_statistics(
        answer,
        stated_compared,
        if solves { "max_abs_error_row" } else { "max_abs_error_step" },
    )?;
    let model: Vec<f64> = model_values_hex
        .iter()
        .map(|digits| f64::from_bits(u64::from_str_radix(digits, 16).expect("16 hex digits")))
        .collect();

    let counts_hold = listed_hold
        && stated_compared == steps.len() as u64
        && stated_not_reached == later
        && model.len() == steps.len();
    let rebuilt = if counts_hold {
        statistics(&steps, &model, &measured).map_err(|_| wrong("model_values_hex", "values whose errors are finite numbers"))?
    } else {
        None
    };
    let seal_of = |statistics: Option<&Statistics>| {
        validation_seal(&SealParts {
            run_seal: &run_seal,
            measurements_sha256: &stated_hash,
            quantity: &quantity,
            component,
            outcome: &outcome,
            steps_landed,
            reject: reject.as_deref(),
            compared: stated_compared,
            not_reached: stated_not_reached,
            statistics,
            model_values_hex: &model_values_hex,
            commitment: commitment.as_ref(),
        })
    };
    let component_given = inputs.get("component").cloned().unwrap_or(Value::Null);
    let commitment_recomputed = commitment.as_ref().map(|commitment| {
        commitment::commitment_sha256(&quantity, &component_given, &Value::Array(measurements.clone()), &commitment.salt)
    });
    let commitment_opens = match (&commitment, &commitment_recomputed) {
        (Some(commitment), Some(recomputed)) => *recomputed == commitment.sha256,
        _ => true,
    };
    let commitment_opened = answer.get("commitment_opened").is_some_and(|said| !said.is_null());

    let run_request = inputs.get("run").cloned().unwrap_or(Value::Null);
    let measured_array = Value::Array(measurements.clone());
    let signed = crate::attest::verify_run(&run_request).and_then(|loads| {
        let places = crate::attest::measurement_places(&measured_array);
        Ok((loads, crate::attest::verify_all(&places)?))
    });
    let (signed_loads, signed_measurements, signature_fault) = match signed {
        Ok((loads, measured)) => (loads, measured, None),
        Err(fault) => (Vec::new(), Vec::new(), Some(fault)),
    };
    let cause = if computed_hash != stated_hash {
        Some(ValidationCause::Measurements)
    } else if !commitment_opens {
        Some(ValidationCause::Commitment)
    } else if signature_fault.is_some() {
        Some(ValidationCause::LoadSignature)
    } else if !counts_hold {
        Some(ValidationCause::Counts)
    } else if seal_of(rebuilt.as_ref()) == stated_seal {
        if same_statistics(rebuilt.as_ref(), stated_statistics.as_ref()) {
            None
        } else {
            Some(ValidationCause::Statistics)
        }
    } else if !same_statistics(rebuilt.as_ref(), stated_statistics.as_ref()) {
        Some(ValidationCause::ModelValues)
    } else {
        Some(ValidationCause::ValidationSeal)
    };
    // Last: every sealed part holds, and the sentence outside the seal says
    // a commitment was opened where the inputs carry none, or the reverse.
    let cause = match cause {
        None if commitment_opened != commitment.is_some() => Some(ValidationCause::CommitmentOpened),
        cause => cause,
    };
    Ok(ValidationReport {
        holds: cause.is_none(),
        cause,
        compared: stated_compared,
        not_reached: stated_not_reached,
        rebuilt,
        measurements_sha256: computed_hash,
        validation_seal: seal_of(rebuilt.as_ref()),
        broken_place: signature_fault.as_ref().map(|fault| fault.place().label()),
        load_fault: signature_fault.as_ref().map(ToString::to_string),
        signed_loads,
        signed_measurements,
        commitment_sha256: commitment.as_ref().map(|commitment| commitment.sha256.clone()),
        commitment_recomputed,
        commitment_opened,
    })
}

/// The statistics an answer states, by their hex. All null when nothing was compared.
fn stated_statistics(
    answer: &Map<String, Value>,
    compared: u64,
    step_key: &'static str,
) -> Result<Option<Statistics>, Refusal> {
    let rms = optional_hex_of(answer, "rms_error_hex")?;
    let mean = optional_hex_of(answer, "mean_signed_error_hex")?;
    let max = optional_hex_of(answer, "max_abs_error_hex")?;
    let step = match answer.get(step_key) {
        None => return Err(Refusal::Missing(step_key.into())),
        Some(Value::Null) => None,
        Some(value) => Some(value.as_u64().ok_or_else(|| {
            wrong(
                step_key,
                if step_key == "max_abs_error_row" { "an integer >= 0 or null" } else { "an integer >= 1 or null" },
            )
        })?),
    };
    match (rms, mean, max, step) {
        (None, None, None, None) => Ok(None),
        (Some(rms_error), Some(mean_signed_error), Some(max_abs_error), Some(max_abs_error_step)) => {
            Ok(Some(Statistics {
                rms_error,
                mean_signed_error,
                max_abs_error,
                max_abs_error_step,
            }))
        }
        // A mixture of null and numbers is not an answer this check reads.
        _ => Err(wrong(
            "rms_error_hex",
            if compared == 0 {
                "null with the other statistics"
            } else {
                "a number with the other statistics"
            },
        )),
    }
}

/// Equal to the bit.
fn same_statistics(left: Option<&Statistics>, right: Option<&Statistics>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.rms_error.to_bits() == right.rms_error.to_bits()
                && left.mean_signed_error.to_bits() == right.mean_signed_error.to_bits()
                && left.max_abs_error.to_bits() == right.max_abs_error.to_bits()
                && left.max_abs_error_step == right.max_abs_error_step
        }
        _ => false,
    }
}

fn wrong(field: &str, expected: &'static str) -> Refusal {
    Refusal::WrongType {
        field: field.to_string(),
        expected,
    }
}

fn object_of<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a Map<String, Value>, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(key.to_string())),
        Some(Value::Object(inner)) => Ok(inner),
        Some(_) => Err(wrong(key, "a JSON object")),
    }
}

fn array_of<'a>(
    object: &'a Map<String, Value>,
    label: &str,
    key: &str,
) -> Result<&'a Vec<Value>, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(label.to_string())),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(wrong(label, "a JSON array")),
    }
}

fn string_of(object: &Map<String, Value>, label: &str, key: &str) -> Result<String, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(label.to_string())),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(wrong(label, "a string")),
    }
}

fn whole_of(object: &Map<String, Value>, key: &str) -> Result<u64, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(key.to_string())),
        Some(value) => value.as_u64().ok_or_else(|| wrong(key, "an integer >= 0")),
    }
}

fn hash_of(object: &Map<String, Value>, key: &str) -> Result<String, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(key.to_string())),
        Some(Value::String(text)) if is_lower_hex(text, 64) => Ok(text.clone()),
        Some(Value::String(_)) => Err(Refusal::NotExactHex {
            field: key.to_string(),
            digits: 64,
        }),
        Some(_) => Err(wrong(key, "a string")),
    }
}

fn optional_hex_of(object: &Map<String, Value>, key: &str) -> Result<Option<f64>, Refusal> {
    match object.get(key) {
        None => Err(Refusal::Missing(key.to_string())),
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if is_lower_hex(text, 16) => Ok(Some(f64::from_bits(
            u64::from_str_radix(text, 16).expect("16 hex digits"),
        ))),
        Some(Value::String(_)) => Err(Refusal::NotExactHex {
            field: key.to_string(),
            digits: 16,
        }),
        Some(_) => Err(wrong(key, "a string or null")),
    }
}

fn hex_list_of(object: &Map<String, Value>, key: &str) -> Result<Vec<String>, Refusal> {
    let items = array_of(object, key, key)?;
    let mut out = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        match item {
            Value::String(text) if is_lower_hex(text, 16) => out.push(text.clone()),
            Value::String(_) => {
                return Err(Refusal::NotExactHex {
                    field: format!("{key}[{index}]"),
                    digits: 16,
                });
            }
            _ => return Err(wrong(&format!("{key}[{index}]"), "a string")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn measurements() -> Value {
        json!([{"step": 1, "value": 3.9}, {"step": 2, "value": 3.8}, {"step": 5, "value": 3.0}])
    }

    /// An answer of the shape the gateway writes, made by the rules of this module.
    fn answer(model: &[f64], steps_landed: u64) -> Value {
        answer_with(measurements(), model, steps_landed, None)
    }

    /// The same, over these measurements, opening `commitment` when one is
    /// given: every part of the answer follows from the rest by the rules of
    /// this module, as a forger who knows them writes it.
    fn answer_with(measurements: Value, model: &[f64], steps_landed: u64, commitment: Option<&Commitment>) -> Value {
        let items = measurements.as_array().unwrap();
        let mut steps = Vec::new();
        let mut measured = Vec::new();
        for item in items {
            let step = item["step"].as_u64().unwrap();
            if step <= steps_landed {
                steps.push(step);
                measured.push(item["value"].as_f64().unwrap());
            }
        }
        let hash = measurements_sha256(&measurements);
        let stats = statistics(&steps, model, &measured).unwrap();
        let hexes: Vec<String> = model.iter().map(|value| bits_hex(*value)).collect();
        let compared = steps.len() as u64;
        let not_reached = items.len() as u64 - compared;
        let run_seal = "ab".repeat(32);
        let seal = validation_seal(&SealParts {
            run_seal: &run_seal,
            measurements_sha256: &hash,
            quantity: "terminal_v",
            component: None,
            outcome: "completed",
            steps_landed,
            reject: None,
            compared,
            not_reached,
            statistics: stats.as_ref(),
            model_values_hex: &hexes,
            commitment,
        });
        let null = Value::Null;
        let hex = |pick: fn(&Statistics) -> String| match &stats {
            Some(stats) => Value::String(pick(stats)),
            None => null.clone(),
        };
        let mut inputs = json!({"run": {}, "quantity": "terminal_v", "measurements": measurements});
        if let Some(commitment) = commitment {
            inputs["commitment"] = json!({"sha256": commitment.sha256, "salt": commitment.salt});
        }
        let mut written = json!({
            "inputs": inputs,
            "outcome": "completed",
            "steps_landed": steps_landed,
            "reject": null,
            "run_seal": run_seal,
            "compared": compared,
            "not_reached": not_reached,
            "rms_error_hex": hex(Statistics::rms_error_hex),
            "mean_signed_error_hex": hex(Statistics::mean_signed_error_hex),
            "max_abs_error_hex": hex(Statistics::max_abs_error_hex),
            "max_abs_error_step": stats.as_ref().map(|s| s.max_abs_error_step),
            "model_values_hex": hexes,
            "measurements_sha256": hash,
            "validation_seal": seal,
        });
        if commitment.is_some() {
            written["commitment_opened"] = json!(commitment::OPENED);
        }
        written
    }

    fn check(value: &Value) -> ValidationReport {
        check_validation(serde_json::to_string(value).unwrap().as_bytes()).unwrap()
    }

    /// A comparison of a plan of solves of the shape the gateway writes:
    /// rows 0, 2 and 3 measured, row 2's solve did not converge.
    fn solves_answer() -> Value {
        let measurements = json!([{"row": 0, "value": 1.5}, {"row": 2, "value": 2.5}, {"row": 3, "value": 4.0}]);
        let rows = [0_u64, 3];
        let model = [1.25_f64, 4.5];
        let measured = [1.5_f64, 4.0];
        let stats = statistics(&rows, &model, &measured).unwrap().unwrap();
        let hexes: Vec<String> = model.iter().map(|value| bits_hex(*value)).collect();
        let hash = measurements_sha256(&measurements);
        let dataset_seal = "cd".repeat(32);
        let seal = validation_seal(&SealParts {
            run_seal: &dataset_seal,
            measurements_sha256: &hash,
            quantity: "sound_speed_m_s",
            component: None,
            outcome: SOLVES_OUTCOME,
            steps_landed: 3,
            reject: None,
            compared: 2,
            not_reached: 1,
            statistics: Some(&stats),
            model_values_hex: &hexes,
            commitment: None,
        });
        json!({
            "inputs": {"solves": {}, "quantity": "sound_speed_m_s", "measurements": measurements},
            "dataset_seal": dataset_seal,
            "solves": 4,
            "converged": 3,
            "refused": 1,
            "unadmitted": 0,
            "compared": 2,
            "not_reached": 1,
            "not_reached_rows": [2],
            "rms_error_hex": stats.rms_error_hex(),
            "mean_signed_error_hex": stats.mean_signed_error_hex(),
            "max_abs_error_hex": stats.max_abs_error_hex(),
            "max_abs_error_row": stats.max_abs_error_step,
            "model_values_hex": hexes,
            "measurements_sha256": hash,
            "validation_seal": seal,
        })
    }

    #[test]
    fn a_comparison_of_solves_is_read_with_its_dataset_seal_and_rows() {
        let genuine = solves_answer();
        let report = check(&genuine);
        assert!(report.holds, "{:?}", report.cause);
        assert_eq!((report.compared, report.not_reached), (2, 1));
        assert_eq!(report.validation_seal, genuine["validation_seal"].as_str().unwrap());

        let mut edited = genuine.clone();
        edited["dataset_seal"] = json!("ce".repeat(32));
        assert_eq!(check(&edited).cause, Some(ValidationCause::ValidationSeal));
        let mut edited = genuine.clone();
        edited["converged"] = json!(4);
        assert_eq!(check(&edited).cause, Some(ValidationCause::ValidationSeal));
        let mut edited = genuine.clone();
        edited["not_reached_rows"] = json!([]);
        assert_eq!(check(&edited).cause, Some(ValidationCause::Counts));
        // Another row listed as not reached compares other rows than were sealed.
        let mut edited = genuine.clone();
        edited["not_reached_rows"] = json!([3]);
        assert!(!check(&edited).holds);
        let mut edited = genuine.clone();
        assert_eq!(genuine["max_abs_error_row"], json!(3));
        edited["max_abs_error_row"] = json!(0);
        assert_eq!(check(&edited).cause, Some(ValidationCause::Statistics));
        let mut edited = genuine;
        edited["inputs"]["measurements"][0]["value"] = json!(1.75);
        assert_eq!(check(&edited).cause, Some(ValidationCause::Measurements));
    }

    #[test]
    fn the_statistics_follow_the_rule_in_step_order() {
        let stats = statistics(&[1, 2, 4], &[1.0, 2.0, 4.0], &[0.5, 2.5, 3.0])
            .unwrap()
            .unwrap();
        // errors 0.5, -0.5, 1.0
        assert_eq!(stats.mean_signed_error, 1.0 / 3.0);
        assert_eq!(stats.rms_error, (1.5_f64 / 3.0).sqrt());
        assert_eq!(stats.max_abs_error, 1.0);
        assert_eq!(stats.max_abs_error_step, 4);
        assert_eq!(statistics(&[], &[], &[]), Ok(None));
    }

    #[test]
    fn a_tie_for_the_greatest_error_goes_to_the_first_step() {
        let stats = statistics(&[3, 7], &[2.0, 0.0], &[0.0, 2.0]).unwrap().unwrap();
        assert_eq!(stats.max_abs_error_step, 3);
        assert_eq!(stats.mean_signed_error, 0.0);
    }

    #[test]
    fn errors_that_overflow_are_refused_not_sealed() {
        let big = 1.0e200;
        assert_eq!(statistics(&[1], &[big], &[-big]), Err(NotFinite));
    }

    #[test]
    fn a_genuine_answer_holds_and_a_run_that_stopped_short_holds() {
        let report = check(&answer(&[3.9, 3.7, 3.0], 5));
        assert!(report.holds, "{report:?}");
        assert_eq!(report.compared, 3);
        let report = check(&answer(&[3.9, 3.7], 3));
        assert!(report.holds, "{report:?}");
        assert_eq!(report.not_reached, 1);
        let report = check(&answer(&[], 0));
        assert!(report.holds, "{report:?}");
        assert_eq!(report.rebuilt, None);
    }

    #[test]
    fn a_changed_statistic_a_changed_model_value_and_a_changed_hash_are_named() {
        let genuine = answer(&[3.9, 3.7, 3.0], 5);

        let mut changed = genuine.clone();
        changed["mean_signed_error_hex"] = json!(bits_hex(0.25));
        assert_eq!(check(&changed).cause, Some(ValidationCause::Statistics));

        let mut changed = genuine.clone();
        changed["model_values_hex"][1] = json!(bits_hex(3.6));
        assert_eq!(check(&changed).cause, Some(ValidationCause::ModelValues));

        let mut changed = genuine.clone();
        changed["measurements_sha256"] = json!("cd".repeat(32));
        assert_eq!(check(&changed).cause, Some(ValidationCause::Measurements));

        let mut changed = genuine.clone();
        changed["inputs"]["measurements"][0]["value"] = json!(3.91);
        assert_eq!(check(&changed).cause, Some(ValidationCause::Measurements));

        let mut changed = genuine.clone();
        changed["validation_seal"] = json!("ef".repeat(32));
        assert_eq!(check(&changed).cause, Some(ValidationCause::ValidationSeal));

        let mut changed = genuine.clone();
        changed["not_reached"] = json!(1);
        assert_eq!(check(&changed).cause, Some(ValidationCause::Counts));

        let mut changed = genuine;
        changed["run_seal"] = json!("12".repeat(32));
        assert_eq!(check(&changed).cause, Some(ValidationCause::ValidationSeal));
    }

    #[test]
    fn an_edited_outcome_steps_landed_or_reject_is_named_by_the_seal_alone() {
        let genuine = answer(&[3.9, 3.7, 3.0], 5);
        for (field, edited) in [
            ("outcome", json!("refused")),
            ("reject", json!("FirstLawBreach")),
            ("steps_landed", json!(6)),
        ] {
            let mut changed = genuine.clone();
            changed[field] = edited;
            let report = check(&changed);
            assert!(!report.holds, "{field}");
            assert!(
                matches!(report.cause, Some(ValidationCause::ValidationSeal | ValidationCause::Counts)),
                "{field}: {:?}",
                report.cause
            );
        }
        // An answer without `reject` is refused by name, not read as none.
        let mut changed = genuine;
        changed.as_object_mut().unwrap().remove("reject");
        assert_eq!(
            check_validation(serde_json::to_string(&changed).unwrap().as_bytes())
                .unwrap_err()
                .to_string(),
            "Missing required parameter: 'reject'"
        );
    }

    #[test]
    fn a_respelled_measurement_gives_the_same_hash() {
        let one = json!([{"step": 1, "value": 1.0}]);
        let other: Value = serde_json::from_str(r#"[{"value":1,"step":1}]"#).unwrap();
        assert_eq!(measurements_sha256(&one), measurements_sha256(&other));
        let moved = json!([{"step": 1, "value": 1.0000000000000002}]);
        assert_ne!(measurements_sha256(&one), measurements_sha256(&moved));
    }

    #[test]
    fn the_seal_is_taken_over_the_stated_bytes() {
        let stats = Statistics {
            rms_error: 1.0,
            mean_signed_error: 0.5,
            max_abs_error: 2.0,
            max_abs_error_step: 7,
        };
        let hexes = vec![bits_hex(1.5)];
        let seal = validation_seal(&SealParts {
            run_seal: &"11".repeat(32),
            measurements_sha256: &"22".repeat(32),
            quantity: "position_m",
            component: Some(2),
            outcome: "refused",
            steps_landed: 3,
            reject: Some("FirstLawBreach"),
            compared: 1,
            not_reached: 4,
            statistics: Some(&stats),
            model_values_hex: &hexes,
            commitment: None,
        });
        let mut text = String::new();
        text.push_str(&"11".repeat(32));
        text.push_str(&"22".repeat(32));
        text.push_str(r#"{"quantity":"position_m","component":2,"outcome":"refused","steps_landed":3,"reject":"FirstLawBreach","compared":1,"not_reached":4,"max_abs_error_step":7}"#);
        text.push_str("rms_error_hex:3ff0000000000000\n");
        text.push_str("mean_signed_error_hex:3fe0000000000000\n");
        text.push_str("max_abs_error_hex:4000000000000000\n");
        text.push_str(&digest_hex(b"3ff8000000000000"));
        assert_eq!(seal, digest_hex(text.as_bytes()));
    }

    #[test]
    fn an_answer_that_cannot_be_read_is_refused_by_name() {
        let mut changed = answer(&[3.9, 3.7, 3.0], 5);
        changed["model_values_hex"][0] = json!("3FF0000000000000");
        let refusal = check_validation(serde_json::to_string(&changed).unwrap().as_bytes()).unwrap_err();
        assert_eq!(
            refusal.to_string(),
            "Physical Violation: 'model_values_hex[0]' must be 16 lowercase hex digits."
        );
        let mut changed = answer(&[3.9, 3.7, 3.0], 5);
        changed.as_object_mut().unwrap().remove("compared");
        assert_eq!(
            check_validation(serde_json::to_string(&changed).unwrap().as_bytes())
                .unwrap_err()
                .to_string(),
            "Missing required parameter: 'compared'"
        );
    }

    const SALT: &str = "000102030405060708090a0b0c0d0e0f";

    /// The commitment the measurements of `measurements()` open, of
    /// `terminal_v`, with no component.
    fn committed() -> Commitment {
        Commitment {
            sha256: commitment::commitment_sha256("terminal_v", &Value::Null, &measurements(), SALT),
            salt: SALT.to_string(),
        }
    }

    #[test]
    fn an_honest_commitment_opens_and_the_seal_takes_it_as_a_sixth_part() {
        let commitment = committed();
        let genuine = answer_with(measurements(), &[3.9, 3.7, 3.0], 5, Some(&commitment));
        let report = check(&genuine);
        assert!(report.holds, "{:?}", report.cause);
        assert_eq!(report.commitment_sha256.as_deref(), Some(commitment.sha256.as_str()));
        assert_eq!(report.commitment_recomputed.as_deref(), Some(commitment.sha256.as_str()));
        assert_eq!(report.validation_seal, genuine["validation_seal"].as_str().unwrap());
        // The same comparison without a commitment is sealed over the five
        // parts alone, and the seal with one is those bytes and two lines.
        let plain = answer(&[3.9, 3.7, 3.0], 5);
        let report = check(&plain);
        assert!(report.holds && report.commitment_sha256.is_none() && report.commitment_recomputed.is_none());
        assert_ne!(plain["validation_seal"], genuine["validation_seal"]);
        let five = |commitment: Option<&Commitment>| {
            let stats = statistics(&[1, 2, 5], &[3.9, 3.7, 3.0], &[3.9, 3.8, 3.0]).unwrap().unwrap();
            let hexes: Vec<String> = [3.9_f64, 3.7, 3.0].iter().map(|value| bits_hex(*value)).collect();
            validation_seal(&SealParts {
                run_seal: &"ab".repeat(32),
                measurements_sha256: &measurements_sha256(&measurements()),
                quantity: "terminal_v",
                component: None,
                outcome: "completed",
                steps_landed: 5,
                reject: None,
                compared: 3,
                not_reached: 0,
                statistics: Some(&stats),
                model_values_hex: &hexes,
                commitment,
            })
        };
        assert_eq!(plain["validation_seal"], five(None));
        assert_eq!(genuine["validation_seal"], five(Some(&commitment)));
    }

    #[test]
    fn the_sixth_part_is_the_two_lines_after_the_five() {
        let stats = Statistics { rms_error: 1.0, mean_signed_error: 0.5, max_abs_error: 2.0, max_abs_error_step: 7 };
        let hexes = vec![bits_hex(1.5)];
        let commitment = Commitment { sha256: "33".repeat(32), salt: SALT.to_string() };
        let parts = |commitment: Option<&Commitment>| {
            validation_seal(&SealParts {
                run_seal: &"11".repeat(32),
                measurements_sha256: &"22".repeat(32),
                quantity: "position_m",
                component: Some(2),
                outcome: "refused",
                steps_landed: 3,
                reject: Some("FirstLawBreach"),
                compared: 1,
                not_reached: 4,
                statistics: Some(&stats),
                model_values_hex: &hexes,
                commitment,
            })
        };
        let mut text = String::new();
        text.push_str(&"11".repeat(32));
        text.push_str(&"22".repeat(32));
        text.push_str(r#"{"quantity":"position_m","component":2,"outcome":"refused","steps_landed":3,"reject":"FirstLawBreach","compared":1,"not_reached":4,"max_abs_error_step":7}"#);
        text.push_str("rms_error_hex:3ff0000000000000\n");
        text.push_str("mean_signed_error_hex:3fe0000000000000\n");
        text.push_str("max_abs_error_hex:4000000000000000\n");
        text.push_str(&digest_hex(b"3ff8000000000000"));
        assert_eq!(parts(None), digest_hex(text.as_bytes()));
        text.push_str(&format!("commitment_sha256:{}\ncommitment_salt:{SALT}\n", "33".repeat(32)));
        assert_eq!(parts(Some(&commitment)), digest_hex(text.as_bytes()));
    }

    #[test]
    fn data_that_do_not_open_the_commitment_are_named_though_every_other_part_follows() {
        let commitment = committed();
        // One digit of one measurement: 3.8 for 3.9, the hash, the statistics
        // and the seal all made again.
        let mut one_digit = measurements();
        one_digit[0]["value"] = json!(3.8);
        // The rows in another order.
        let mut reordered = measurements();
        reordered.as_array_mut().unwrap().swap(0, 1);
        // A row dropped.
        let mut dropped = measurements();
        dropped.as_array_mut().unwrap().pop();
        for (label, data, model) in [
            ("one digit", one_digit, vec![3.9, 3.7, 3.0]),
            ("reordered", reordered, vec![3.9, 3.7, 3.0]),
            ("dropped", dropped, vec![3.9, 3.7]),
        ] {
            let forged = answer_with(data, &model, 5, Some(&commitment));
            let report = check(&forged);
            assert_eq!(report.cause, Some(ValidationCause::Commitment), "{label}");
            assert_eq!(report.commitment_sha256.as_deref(), Some(commitment.sha256.as_str()), "{label}");
            assert_ne!(report.commitment_recomputed.as_deref(), Some(commitment.sha256.as_str()), "{label}");
        }
        // Another salt under the same sha256, the seal made again with it.
        let wrong_salt = Commitment { sha256: commitment.sha256.clone(), salt: "ff".repeat(16) };
        let forged = answer_with(measurements(), &[3.9, 3.7, 3.0], 5, Some(&wrong_salt));
        assert_eq!(check(&forged).cause, Some(ValidationCause::Commitment));
        // Another quantity named in inputs: the commitment is of terminal_v.
        let mut genuine = answer_with(measurements(), &[3.9, 3.7, 3.0], 5, Some(&commitment));
        genuine["inputs"]["quantity"] = json!("soc");
        assert!(!check(&genuine).holds);
        // The commitment taken out of an answer sealed with it: the seal speaks.
        let mut taken = answer_with(measurements(), &[3.9, 3.7, 3.0], 5, Some(&commitment));
        taken["inputs"].as_object_mut().unwrap().remove("commitment");
        assert_eq!(check(&taken).cause, Some(ValidationCause::ValidationSeal));
        // A commitment put into an answer sealed without one: the seal speaks.
        let mut put = answer(&[3.9, 3.7, 3.0], 5);
        put["inputs"]["commitment"] = json!({"sha256": commitment.sha256, "salt": SALT});
        assert_eq!(check(&put).cause, Some(ValidationCause::ValidationSeal));
        // A changed measurement whose hash was not made again is still
        // named by the hash first.
        let mut edited = answer_with(measurements(), &[3.9, 3.7, 3.0], 5, Some(&commitment));
        edited["inputs"]["measurements"][1]["value"] = json!(3.81);
        assert_eq!(check(&edited).cause, Some(ValidationCause::Measurements));
    }

    #[test]
    fn a_commitment_not_of_the_commitments_form_is_refused_not_read() {
        let commitment = committed();
        let genuine = answer_with(measurements(), &[3.9, 3.7, 3.0], 5, Some(&commitment));
        for (edit, sentence) in [
            (
                json!({"sha256": commitment.sha256, "salt": "0001020304050607"}),
                "Physical Violation: 'inputs.commitment.salt' is 16 hex digits, and a salt is at least 32, 16 bytes: a shorter salt leaves the committed data to be found by trying salts against the sha256 (CommitmentSaltTooShort).",
            ),
            (json!({"sha256": commitment.sha256}), "Missing required parameter: 'inputs.commitment.salt' (InputMissing)."),
            (json!({"salt": SALT}), "Missing required parameter: 'inputs.commitment.sha256' (InputMissing)."),
            (
                json!({"sha256": commitment.sha256.to_uppercase(), "salt": SALT}),
                "Physical Violation: 'inputs.commitment.sha256' must be 64 lowercase hex digits, the SHA-256 committed (CommitmentMalformed).",
            ),
        ] {
            let mut changed = genuine.clone();
            changed["inputs"]["commitment"] = edit;
            let refusal = check_validation(serde_json::to_string(&changed).unwrap().as_bytes()).unwrap_err();
            assert_eq!(refusal.to_string(), sentence);
        }
    }

    #[test]
    fn a_comparison_of_solves_opens_its_commitment_with_rows_in_the_place_of_steps() {
        let mut genuine = solves_answer();
        let measured = genuine["inputs"]["measurements"].clone();
        let commitment = Commitment {
            sha256: commitment::commitment_sha256("sound_speed_m_s", &Value::Null, &measured, SALT),
            salt: SALT.to_string(),
        };
        genuine["inputs"]["commitment"] = json!({"sha256": commitment.sha256, "salt": SALT});
        genuine["commitment_opened"] = json!(commitment::OPENED);
        // Sealed again with the sixth part.
        let stats = statistics(&[0, 3], &[1.25, 4.5], &[1.5, 4.0]).unwrap().unwrap();
        let hexes: Vec<String> = [1.25_f64, 4.5].iter().map(|value| bits_hex(*value)).collect();
        genuine["validation_seal"] = json!(validation_seal(&SealParts {
            run_seal: genuine["dataset_seal"].as_str().unwrap(),
            measurements_sha256: genuine["measurements_sha256"].as_str().unwrap(),
            quantity: "sound_speed_m_s",
            component: None,
            outcome: SOLVES_OUTCOME,
            steps_landed: 3,
            reject: None,
            compared: 2,
            not_reached: 1,
            statistics: Some(&stats),
            model_values_hex: &hexes,
            commitment: Some(&commitment),
        }));
        assert!(check(&genuine).holds, "{:?}", check(&genuine).cause);
        let mut other_salt = genuine.clone();
        other_salt["inputs"]["commitment"]["salt"] = json!("ee".repeat(16));
        assert!(!check(&other_salt).holds);
    }

    /// The red team of 0.13.0: `commitment_opened` is outside the seal, so
    /// the sentence written into an answer that opened no commitment, or left
    /// out of one that did, was seen by the receipt alone; with the receipt
    /// written again it passed. It is named by cause, `commitment_opened`,
    /// and every seal of the answer still holds. Null is no claim.
    #[test]
    fn a_commitment_opened_said_or_left_out_against_the_inputs_is_named() {
        let plain = answer(&[3.9, 3.7, 3.0], 5);
        let mut claimed = plain.clone();
        claimed["commitment_opened"] = json!(commitment::OPENED);
        let report = check(&claimed);
        assert!(!report.holds);
        assert_eq!(report.cause, Some(ValidationCause::CommitmentOpened));
        assert_eq!(report.cause.unwrap().as_str(), "commitment_opened");
        assert!(report.commitment_opened && report.commitment_sha256.is_none());
        assert_eq!(report.validation_seal, plain["validation_seal"].as_str().unwrap());
        let mut null = plain.clone();
        null["commitment_opened"] = Value::Null;
        assert!(check(&null).holds);

        let genuine = answer_with(measurements(), &[3.9, 3.7, 3.0], 5, Some(&committed()));
        assert!(check(&genuine).holds);
        let mut unsaid = genuine.clone();
        unsaid.as_object_mut().unwrap().remove("commitment_opened");
        let report = check(&unsaid);
        assert_eq!(report.cause, Some(ValidationCause::CommitmentOpened));
        assert!(!report.commitment_opened && report.commitment_sha256.is_some());
        // Data that do not open the commitment are named first.
        let mut both = unsaid.clone();
        both["inputs"]["commitment"]["salt"] = json!("ee".repeat(16));
        assert_eq!(check(&both).cause, Some(ValidationCause::Commitment));
    }

}
