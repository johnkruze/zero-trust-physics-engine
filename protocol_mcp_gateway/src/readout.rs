//! The readout of a run: named published quantities in decimals at the
//! recorded steps, each one's least and greatest value over every landed step
//! with its step and time, and, when asked, the first landed step at which one
//! passes a stated level.
//!
//! The readout is taken out of the request before the run is prepared, so the
//! request that is sealed, the frames and the run seal are those of the same
//! run asked without it. It reads the specimen after each landed step and
//! changes nothing of the run. The receipt covers it, as it covers every byte
//! of the answer.

use serde::Serialize;
use serde_json::Value;

use crate::{DoorError, Live, QuantityKind};

/// The key of a `run_trajectory` request that asks for a readout.
pub(crate) const READOUT: &str = "readout";

/// The most quantities one readout names.
pub const MAX_READOUT_QUANTITIES: usize = 8;

const FIELDS: &[&str] = &["quantities", "first_past"];
const FIRST_PAST_FIELDS: &[&str] = &["quantity", "above", "below"];

/// What the readout says of its numbers, in every readout.
pub(crate) const READOUT_ABOUT: &str = "Decimals of the exact values the run published. time_s is the time since the start of the run: the step times dt_s. values are at the recorded steps; minimum, maximum and first_past are over every landed step. at_minimum and at_maximum hold every other quantity of the readout at the step of that minimum or maximum, and first_past.at_step every other quantity at its step, read from the same landed state. The readout is not part of the sealed request: the run_seal is the seal of the same run asked without it, and the receipt covers it.";

fn violation(text: impl std::fmt::Display) -> DoorError {
    DoorError::Unprocessable(format!("Physical Violation: {text}"))
}

/// One quantity as the readout names it: a number of the state, or one
/// component of a 3-vector written `name[0]`, `name[1]` or `name[2]`.
#[derive(Clone)]
struct Named {
    text: String,
    field: String,
    component: Option<usize>,
}

struct Level {
    quantity: usize,
    above: bool,
    level: f64,
}

/// A readout read and held to the card before the run takes a permit.
pub(crate) struct Plan {
    names: Vec<Named>,
    level: Option<Level>,
    dt_s: f64,
    /// Values at every landed step are kept only for the recorded steps.
    record_every: u64,
    steps_requested: u64,
    values: Vec<Vec<f64>>,
    steps: Vec<u64>,
    pending: Option<(u64, Vec<f64>)>,
    least: Vec<Option<(f64, u64, Vec<f64>)>>,
    most: Vec<Option<(f64, u64, Vec<f64>)>>,
    first: Option<(u64, f64, Vec<f64>)>,
}

/// Take the readout out of a `run_trajectory` request. The request that
/// remains is the one that is run and sealed.
pub(crate) fn take(request: Value) -> (Value, Option<Value>) {
    match request {
        Value::Object(mut object) => {
            let readout = object.shift_remove(READOUT);
            (Value::Object(object), readout)
        }
        other => (other, None),
    }
}

fn read_name(domain: &str, card: &str, text: &str) -> Result<Named, DoorError> {
    let (field, component) = match text.len().checked_sub(3).map(|at| text.split_at(at)) {
        Some((field, "[0]")) => (field, Some(0)),
        Some((field, "[1]")) => (field, Some(1)),
        Some((field, "[2]")) => (field, Some(2)),
        _ => (text, None),
    };
    let kind = crate::state_field_kind(domain, card, field).flatten();
    match (kind, component) {
        (Some(QuantityKind::Number), None) | (Some(QuantityKind::Vector3), Some(_)) => Ok(Named {
            text: text.to_string(),
            field: field.to_string(),
            component,
        }),
        (Some(QuantityKind::Vector3), None) => Err(violation(format!(
            "'readout.quantities' names '{field}', a 3-vector: name one component, {field}[0], {field}[1] or {field}[2]."
        ))),
        (Some(QuantityKind::Flag), _) => Err(violation(format!(
            "'readout.quantities' names '{field}', a flag. A flag is not a number and has no value to read out."
        ))),
        _ if genesis_seal::json_text::is_plain_field_name(field) => {
            // The names the card does publish, so that the next request can
            // be written without reading the card again: a start field and
            // the state's name for the same thing can differ.
            let published: Vec<String> = crate::state_fields(domain, card)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(name, kind)| match kind {
                    QuantityKind::Number => Some(name),
                    QuantityKind::Vector3 => Some(format!("{name}[0], {name}[1], {name}[2]")),
                    QuantityKind::Flag => None,
                })
                .collect();
            Err(violation(format!(
                "'readout.quantities' names '{text}', which the {card} card does not publish as a number. It publishes these numbers: {}.",
                published.join(", ")
            )))
        }
        _ => Err(violation(format!(
            "'readout.quantities' holds a name that is not a field the {card} card publishes."
        ))),
    }
}

impl Plan {
    /// Read a readout for a run of `domain`/`card` whose request, without the
    /// readout, is `request`. Every fault is named before a permit is taken.
    pub(crate) fn read(
        readout: &Value,
        request: &Value,
        steps_requested: u64,
    ) -> Result<Plan, DoorError> {
        let object = readout
            .as_object()
            .ok_or_else(|| violation("'readout' must be a JSON object."))?;
        for key in object.keys() {
            if !FIELDS.contains(&key.as_str()) {
                return Err(violation(format!(
                    "'readout.{key}' is not a field of a readout. Its fields are quantities and first_past."
                )));
            }
        }
        let domain = request["domain"].as_str().unwrap_or_default();
        let card = request["card"].as_str().unwrap_or_default();
        let listed = object
            .get("quantities")
            .ok_or_else(|| DoorError::Unprocessable("Missing required parameter: 'readout.quantities'".into()))?
            .as_array()
            .ok_or_else(|| violation("'readout.quantities' must be a JSON array of names."))?;
        if listed.is_empty() {
            return Err(violation("'readout.quantities' must name at least one quantity."));
        }
        if listed.len() > MAX_READOUT_QUANTITIES {
            return Err(violation(format!(
                "'readout.quantities' names more than {MAX_READOUT_QUANTITIES} quantities."
            )));
        }
        let mut names: Vec<Named> = Vec::with_capacity(listed.len());
        for item in listed {
            let text = item
                .as_str()
                .ok_or_else(|| violation("'readout.quantities' must be a JSON array of names."))?;
            if names.iter().any(|named| named.text == text) {
                return Err(violation(format!("'readout.quantities' names '{text}' twice.")));
            }
            names.push(read_name(domain, card, text)?);
        }
        let level = match object.get("first_past") {
            None => None,
            Some(value) => {
                let first = value
                    .as_object()
                    .ok_or_else(|| violation("'readout.first_past' must be a JSON object."))?;
                for key in first.keys() {
                    if !FIRST_PAST_FIELDS.contains(&key.as_str()) {
                        return Err(violation(format!(
                            "'readout.first_past.{key}' is not a field of first_past. Its fields are quantity and one of above or below."
                        )));
                    }
                }
                let quantity = first
                    .get("quantity")
                    .ok_or_else(|| DoorError::Unprocessable("Missing required parameter: 'readout.first_past.quantity'".into()))?
                    .as_str()
                    .ok_or_else(|| violation("'readout.first_past.quantity' must be a string."))?;
                let index = names.iter().position(|named| named.text == quantity).ok_or_else(|| {
                    violation(format!(
                        "'readout.first_past.quantity' is '{quantity}', which readout.quantities does not name."
                    ))
                })?;
                let (above, given) = match (first.get("above"), first.get("below")) {
                    (Some(level), None) => (true, level),
                    (None, Some(level)) => (false, level),
                    _ => {
                        return Err(violation(
                            "'readout.first_past' takes one of above or below, not both and not neither.",
                        ));
                    }
                };
                let key = if above { "above" } else { "below" };
                let level = given
                    .as_f64()
                    .ok_or_else(|| violation(format!("'readout.first_past.{key}' must be a number.")))?;
                crate::accept_f64(&format!("readout.first_past.{key}"), level)
                    .map_err(DoorError::Unprocessable)?;
                Some(Level { quantity: index, above, level })
            }
        };
        let dt_s = request["dt_s"].as_f64().unwrap_or(f64::NAN);
        let record_every = request["record_every"].as_u64().unwrap_or(1).max(1);
        let count = names.len();
        Ok(Plan {
            names,
            level,
            dt_s,
            record_every,
            steps_requested,
            values: vec![Vec::new(); count],
            steps: Vec::new(),
            pending: None,
            least: vec![None; count],
            most: vec![None; count],
            first: None,
        })
    }

    /// Read the specimen after landed step `landed`.
    pub(crate) fn observe(&mut self, landed: u64, live: &Live) -> Result<(), String> {
        let state = live.publish().map_err(|failure| failure.detail)?;
        let mut now = Vec::with_capacity(self.names.len());
        for named in &self.names {
            now.push(exact(&state, named)?);
        }
        for (index, value) in now.iter().enumerate() {
            if self.least[index].as_ref().is_none_or(|(least, _, _)| value < least) {
                self.least[index] = Some((*value, landed, now.clone()));
            }
            if self.most[index].as_ref().is_none_or(|(most, _, _)| value > most) {
                self.most[index] = Some((*value, landed, now.clone()));
            }
        }
        if self.first.is_none() {
            if let Some(level) = &self.level {
                let value = now[level.quantity];
                let past = if level.above { value > level.level } else { value < level.level };
                if past {
                    self.first = Some((landed, value, now.clone()));
                }
            }
        }
        if landed % self.record_every == 0 || landed == self.steps_requested {
            self.keep(landed, now);
            self.pending = None;
        } else {
            self.pending = Some((landed, now));
        }
        Ok(())
    }

    fn keep(&mut self, landed: u64, now: Vec<f64>) {
        self.steps.push(landed);
        for (index, value) in now.into_iter().enumerate() {
            self.values[index].push(value);
        }
    }

    /// The readout of the run as it ended. The last landed step is recorded,
    /// as a run records it.
    pub(crate) fn finish(mut self) -> Readout {
        if let Some((landed, now)) = self.pending.take() {
            self.keep(landed, now);
        }
        let dt_s = self.dt_s;
        let time = |step: u64| Exact(step as f64 * dt_s);
        let names = &self.names;
        // Every other quantity of the readout at one landed step.
        let others = |skip: usize, all: &[f64]| AtStep(
            names
                .iter()
                .zip(all)
                .enumerate()
                .filter(|(index, _)| *index != skip)
                .map(|(_, (named, value))| (named.text.clone(), Exact(*value)))
                .collect(),
        );
        let quantities = self
            .names
            .iter()
            .enumerate()
            .map(|(index, named)| {
                let least = self.least[index].as_ref();
                let most = self.most[index].as_ref();
                QuantityReadout {
                    quantity: named.text.clone(),
                    values: self.values[index].iter().map(|value| Exact(*value)).collect(),
                    minimum: least.map(|(value, _, _)| Exact(*value)),
                    minimum_step: least.map(|(_, step, _)| *step),
                    minimum_time_s: least.map(|(_, step, _)| time(*step)),
                    at_minimum: least.map(|(_, _, all)| others(index, all)),
                    maximum: most.map(|(value, _, _)| Exact(*value)),
                    maximum_step: most.map(|(_, step, _)| *step),
                    maximum_time_s: most.map(|(_, step, _)| time(*step)),
                    at_maximum: most.map(|(_, _, all)| others(index, all)),
                }
            })
            .collect();
        let first_past = self.level.as_ref().map(|level| FirstPast {
            quantity: self.names[level.quantity].text.clone(),
            above: level.above.then_some(Exact(level.level)),
            below: (!level.above).then_some(Exact(level.level)),
            step: self.first.as_ref().map(|(step, _, _)| *step),
            time_s: self.first.as_ref().map(|(step, _, _)| time(*step)),
            value: self.first.as_ref().map(|(_, value, _)| Exact(*value)),
            at_step: self.first.as_ref().map(|(_, _, all)| others(level.quantity, all)),
        });
        Readout {
            steps: self.steps.clone(),
            time_s: self.steps.iter().map(|step| time(*step)).collect(),
            quantities,
            first_past,
            about: READOUT_ABOUT,
        }
    }
}

/// The exact value of one named quantity in a published state, read from its hex.
fn exact(state: &Value, named: &Named) -> Result<f64, String> {
    let hex = state
        .get(format!("{}_hex", named.field))
        .ok_or_else(|| "Physical Violation: the state does not carry the quantity.".to_string())?;
    let text = match named.component {
        None => hex.as_str(),
        Some(index) => hex.get(index).and_then(Value::as_str),
    }
    .ok_or_else(|| "Physical Violation: the quantity is not in the state as expected.".to_string())?;
    u64::from_str_radix(text, 16)
        .map(f64::from_bits)
        .map_err(|_| "Physical Violation: the quantity is not a 64-bit value.".to_string())
}

/// A number written as the engine writes a state's decimals.
#[derive(Clone, Copy)]
pub struct Exact(pub f64);

impl Serialize for Exact {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::serialize_exact_f64(&self.0, serializer)
    }
}

/// The other quantities of a readout at one landed step, by name, in the
/// order the readout names them.
pub struct AtStep(pub Vec<(String, Exact)>);

impl Serialize for AtStep {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, value) in &self.0 {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}

#[derive(Serialize)]
pub struct QuantityReadout {
    pub quantity: String,
    pub values: Vec<Exact>,
    pub minimum: Option<Exact>,
    pub minimum_step: Option<u64>,
    pub minimum_time_s: Option<Exact>,
    /// Every other quantity of the readout at the minimum's step.
    pub at_minimum: Option<AtStep>,
    pub maximum: Option<Exact>,
    pub maximum_step: Option<u64>,
    pub maximum_time_s: Option<Exact>,
    /// Every other quantity of the readout at the maximum's step.
    pub at_maximum: Option<AtStep>,
}

#[derive(Serialize)]
pub struct FirstPast {
    pub quantity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub above: Option<Exact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub below: Option<Exact>,
    pub step: Option<u64>,
    pub time_s: Option<Exact>,
    pub value: Option<Exact>,
    /// Every other quantity of the readout at that step.
    pub at_step: Option<AtStep>,
}

#[derive(Serialize)]
pub struct Readout {
    pub steps: Vec<u64>,
    pub time_s: Vec<Exact>,
    pub quantities: Vec<QuantityReadout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_past: Option<FirstPast>,
    pub about: &'static str,
}

/// The declaration of the readout key in `run_trajectory`'s schema.
pub(crate) fn schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "quantities": {
                "type": "array",
                "minItems": 1,
                "maxItems": MAX_READOUT_QUANTITIES,
                "items": { "type": "string" },
                "description": "Names of numbers the card publishes in its state; one component of a 3-vector is written name[0], name[1] or name[2]."
            },
            "first_past": {
                "type": "object",
                "properties": {
                    "quantity": { "type": "string" },
                    "above": { "type": "number" },
                    "below": { "type": "number" }
                },
                "additionalProperties": false,
                "description": "A quantity of the list and one of above or below: the first landed step at which it is strictly above (or below) that level, with its time, its value and every other listed quantity at that step (at_step); null when no landed step passes it."
            }
        },
        "required": ["quantities"],
        "additionalProperties": false,
        "description": "Optional. The named quantities in decimals at the recorded steps, each one's minimum and maximum over every landed step with its step, its time and every other listed quantity at that step (at_minimum, at_maximum), and first_past when asked. Not part of the sealed request: the run_seal is that of the same run without it."
    })
}
