//! `design_run`: inverse design. A target end state in, the loads of each
//! segment out.
//!
//! Given a `run_trajectory` request, the loads of its segments that the
//! search may set (each a place such as `segments[0].load.heat_power_w`, or
//! `segments[*].load.heat_power_w` for one value held through every segment,
//! with the span the actuator allows), and a target (a quantity of the final
//! state, a value and a tolerance), the engine finds a timeline of loads whose
//! run lands within the tolerance of the target, and returns that run with
//! its proof.
//!
//! Every candidate the search tries is an ordinary run of the card: the
//! request as sent with the set loads' numbers replaced, computed by the same
//! code `run_trajectory` uses and sealed by the same rule. A candidate that
//! crosses one of the card's limits is a refused run, and the search records
//! it by its name; its value is not moved back and nothing is filled in in
//! its place. All the candidates, in the order they were tried, are sealed
//! together as one `run_dataset` plan of kind `list` under its
//! `dataset_seal`, which `verify_dataset` given that plan recomputes.
//!
//! # The search: coordinate bisection
//!
//! The search is plain and the same every time. There is no randomness and
//! no clock in it, so the same request gives the same bits.
//!
//! 1. The request as sent is the start: its numbers at the set places are
//!    the first candidate. Nothing is filled in.
//! 2. On a card that declares its target quantity monotone in the set loads
//!    ([`MONOTONE`]), the two corners of the box (every set load at its
//!    high end, and every one at its low end) are run next. The final state
//!    of every other timeline in the box lies between the corners' within the
//!    card's published rounding allowance, so a target beyond a corner that
//!    computed is refused by name, [`BEYOND_REACH`]: the physics cannot get
//!    there with the loads allowed.
//! 3. Sweeps. In each sweep the set loads are taken in the order given. For
//!    one load, with the others held at the best timeline so far, the two
//!    ends of its span are run. Between two neighbouring values whose runs
//!    complete with the target quantity on either side of the target, the
//!    span is halved (the middle taken as `low + (high - low) / 2` in 64-bit
//!    arithmetic) until a run lands within the tolerance or the two values
//!    are neighbouring 64-bit numbers. When no pair lies either side,
//!    [`DESIGN_SCAN_POINTS`] values evenly between the ends are run (the
//!    target may lie where the response turns, at a peak or a ring) and the
//!    pairs are looked at again. Between a value that completes and one
//!    whose run is refused or ended, the span is halved at most
//!    [`DESIGN_EDGE_HALVINGS`] times toward the edge where the card stops,
//!    looking for a value on the far side of the target. A run that completes,
//!    or that ends at an end its card declares (the event finished), has a
//!    value; a run refused part way has none. The best run of the
//!    line (the least distance to the target) becomes the timeline. A sweep
//!    that does not bring the best run nearer ends the search.
//!
//! # Its caps, each named in the answer when it stops the search
//!
//! - [`DESIGN_MAX_CANDIDATES`] candidates, the start included;
//! - [`DESIGN_MAX_STEPS`] steps in all the candidates together, the engine's
//!   limit for one run;
//! - at most [`crate::dataset::MAX_INLINE_FRAMES`] recorded frames in all the
//!   candidates together, so that the candidates' plan is one
//!   `verify_dataset` recomputes inline (`DESIGN_MAX_FRAMES`);
//! - [`DESIGN_MAX_SWEEPS`] sweeps; [`DESIGN_MAX_LOADS`] set loads.
//!
//! # What it does not do
//!
//! A search that ends without a run inside the tolerance on a card that
//! declares no monotone quantity has outcome `not_reached`, not a refusal: it
//! says that this search, inside its caps, found no timeline, and not that
//! none exists. The search finds a timeline, not the best one by any measure
//! other than the distance to the target; it holds each set load constant
//! through its segment; it sets numbers of `load` only, never a start value,
//! the step or the steps; and coordinate bisection can miss a target that
//! only a change of two loads together reaches.

use genesis_seal::dataset::{RowResult, assemble};
use genesis_seal::plan::DatasetRequest;
use genesis_seal::sealed_form::{digest_hex, sealed_form};
use serde_json::{Map, Value, json};

use crate::descriptor::{Card, Role};
use crate::live::DoorError;
use crate::view::View;

/// The most candidates one search runs, the start included.
pub const DESIGN_MAX_CANDIDATES: u64 = 64;
/// The most steps of all the candidates of one search together.
pub const DESIGN_MAX_STEPS: u64 = crate::trajectory::MAX_TRAJECTORY_STEPS;
/// The most frames all the candidates may record together: the most an
/// inline dataset records, so that the candidates' plan is recomputed inline.
pub const DESIGN_MAX_FRAMES: u64 = crate::dataset::MAX_INLINE_FRAMES;
/// The most sweeps through the set loads.
pub const DESIGN_MAX_SWEEPS: u64 = 4;
/// The most loads one search sets.
pub const DESIGN_MAX_LOADS: usize = 8;
/// When the two ends of a line and the timeline so far give no pair either
/// side of the target, this many values evenly between the ends are run.
pub const DESIGN_SCAN_POINTS: u64 = 7;
/// The most halvings toward the edge where a card stops, on one line.
pub const DESIGN_EDGE_HALVINGS: u64 = 20;
/// The name of the refusal of a target the loads allowed cannot reach.
pub const BEYOND_REACH: &str = "TargetBeyondReach";
/// The name of the refusal of a target outside the range the card declares
/// for the quantity.
pub const OUTSIDE_CARD: &str = "TargetOutsideCard";

const KEYS: &[&str] = &["request", "vary", "target"];
const VARY_KEYS: &[&str] = &["place", "low", "high"];
const TARGET_KEYS: &[&str] = &["quantity", "value", "tolerance"];

/// A card whose target quantity is monotone in some of its loads, derived
/// from the card's own map, with the rounding allowance of a computed run.
pub struct Monotone {
    pub domain: &'static str,
    pub card: &'static str,
    pub quantity: &'static str,
    /// The loads the quantity does not decrease in.
    pub loads: &'static [&'static str],
    /// How far a computed final value may lie from the exact map, for a run
    /// of this many steps.
    pub allowance: fn(u64) -> f64,
    pub why: &'static str,
}

fn thermal_allowance(steps: u64) -> f64 {
    // N steps of the exact map agree with the exact solution within
    // (N + 1) STEP_ERROR_K; two runs compared, twice that.
    2.0 * (steps as f64 + 1.0) * genesis_kernel::thermal::STEP_ERROR_K
}

/// The cards that declare a monotone target. A target beyond a corner of the
/// box is refused as [`BEYOND_REACH`] only on these.
pub const MONOTONE: &[Monotone] = &[Monotone {
    domain: "energy",
    card: "thermal",
    quantity: "temperature_k",
    loads: &["heat_power_w", "ambient_k"],
    allowance: thermal_allowance,
    why: "The cold plate's step is the exact map T' = T + (T_amb + Q R - T)(1 - exp(-dt/RC)), so after N steps the plate temperature is the start's times a positive factor plus a sum, with positive weights, of each step's T_amb + Q R: it does not decrease in any segment's heat_power_w or ambient_k. Each computed step lands within 2^-42 K of the exact map, so a computed run lies within (N + 1) 2^-42 K of the exact one, and two runs are compared within twice that.",
}];

/// The quantities whose declared range a card holds every landed state to,
/// refusing a step that leaves it: a target outside one is refused before
/// any run as [`OUTSIDE_CARD`]. Elsewhere a phase field's range can bound
/// only the start (the Great Britain grid's frequency_hz is the frequency at
/// the start), and a target beyond it is searched for like any other.
pub const HELD_EVERY_STEP: &[(&str, &str, &str)] = &[
    // TemperatureOutsideCard on every trial landing.
    ("energy", "thermal", "temperature_k"),
    // CurrentOutsideCard, VoltageOutsideCard and CopperMelt on every trial landing.
    ("energy", "grid_node", "current_a"),
    ("energy", "grid_node", "voltage_v"),
    ("energy", "grid_node", "temperature_k"),
];

fn monotone_of(domain: &str, card: &str, quantity: &str) -> Option<&'static Monotone> {
    MONOTONE.iter().find(|item| (item.domain, item.card, item.quantity) == (domain, card, quantity))
}

/// The cards the tool serves: every card with at least one load that is one
/// number. The tool sets only such loads.
#[cfg(test)]
pub fn served_cards(view: &View) -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    view.cards()
        .into_iter()
        .filter_map(|card| {
            let loads: Vec<&'static str> = card
                .fields
                .iter()
                .filter(|field| field.role == Role::Load && field.components == 1)
                .map(|field| field.name)
                .collect();
            (!loads.is_empty()).then_some((card.domain, card.card, loads))
        })
        .collect()
}

pub fn description() -> String {
    format!(
        "Designs a run: from a target end state to the loads of each segment that reach it, computed by the engine with no randomness, so the same request gives the same bits. request is a run_trajectory request; its numbers at the places vary names are the start, and vary gives each load's span inside the card's bounds. target is a number of the final state, a value and a tolerance. Every candidate is an ordinary run with its run_seal; one that crosses a limit is recorded refused by its name, and no value is moved back. The answer gives the timeline, its run_trajectory request and run_seal, the value reached and the miss, every candidate in order as a run_dataset plan with its dataset_seal, and a design_seal. Outcome reached; not_reached, meaning this search found none inside its caps, which is not a limit of the physics; or refused: {OUTSIDE_CARD} for a target outside the card's range, {BEYOND_REACH} where the card's map is monotone and a corner of the loads allowed falls short. The search is coordinate bisection, capped at {DESIGN_MAX_CANDIDATES} candidates (DESIGN_MAX_CANDIDATES), {DESIGN_MAX_STEPS} steps (DESIGN_MAX_STEPS) and {DESIGN_MAX_FRAMES} recorded frames (DESIGN_MAX_FRAMES) in all, and {DESIGN_MAX_SWEEPS} sweeps (DESIGN_MAX_SWEEPS). verify_tape, given this whole answer as it came, designs it again and compares the two, byte for byte (the long form)."
    )
}

pub fn schema() -> Map<String, Value> {
    match json!({
        "type": "object",
        "properties": {
            "request": { "type": "object", "description": "A run_trajectory request as that tool takes it, with no from or readout, and a number at every place vary names: the search's start. Nothing is filled in." },
            "vary": {
                "type": "array",
                "minItems": 1,
                "maxItems": DESIGN_MAX_LOADS,
                "items": {
                    "type": "object",
                    "properties": {
                        "place": { "type": "string", "description": "segments[<i>].load.<name> for one segment's load, or segments[*].load.<name> for one value held through every segment. The load is one number of the card." },
                        "low": { "type": "number", "description": "The low end of the span the actuator allows, at or inside the card's declared bound of the load." },
                        "high": { "type": "number", "description": "The high end, above low." }
                    },
                    "required": ["place", "low", "high"],
                    "additionalProperties": false
                },
                "description": "From 1 to 8 loads the search sets, in the order a sweep takes them. The search sets loads only, never a start value, the step or the steps, and each load is held constant through its segment. low and high are the span the actuator allows; each lies at or inside the card's declared bound of the load, and a load the card bounds by nothing of its own (the cold plate's heat_power_w) is bounded by this span alone."
            },
            "target": {
                "type": "object",
                "properties": {
                    "quantity": { "type": "string", "description": "A number the card publishes in the final state, such as temperature_k." },
                    "value": { "type": "number", "description": "The value the run's final state is to reach." },
                    "tolerance": { "type": "number", "description": "How near is reached: the final value within value plus or minus tolerance. Above zero." }
                },
                "required": ["quantity", "value", "tolerance"],
                "additionalProperties": false,
                "description": "The end state to reach: the quantity of the run's final state within value plus or minus tolerance, after its last step, or, for a run that ends at an end its card declares (a reactor past its first maximum of power), where the event finished. A run refused part way gives no value. A target outside the range the card declares for that quantity is refused before any run (TargetOutsideCard)."
            }
        },
        "required": ["request", "vary", "target"],
        "additionalProperties": false
    }) {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

fn violation(text: impl Into<String>) -> DoorError {
    DoorError::Unprocessable(text.into())
}

fn hex(number: f64) -> String {
    format!("{:016x}", number.to_bits())
}

fn exact(number: f64) -> Value {
    serde_json::Number::from_f64(number).map(Value::Number).unwrap_or(Value::Null)
}

fn exact_from_hex(text: &str) -> Option<f64> {
    (text.len() == 16).then(|| u64::from_str_radix(text, 16).ok()).flatten().map(f64::from_bits)
}

/// One load the search sets: its segment (or every segment) and its span.
#[derive(Clone, Debug)]
struct Knob {
    place: String,
    /// `None` for `segments[*]`.
    segment: Option<usize>,
    name: String,
    low: f64,
    high: f64,
}

struct Target {
    quantity: String,
    value: f64,
    tolerance: f64,
}

struct Plan {
    body: Value,
    request: Value,
    domain: &'static str,
    card: &'static str,
    knobs: Vec<Knob>,
    target: Target,
    start: Vec<f64>,
    most: u64,
    cap: &'static str,
    steps_each: u64,
    monotone: Option<&'static Monotone>,
}

fn number_of(object: &Map<String, Value>, key: &str, of: &str) -> Result<f64, DoorError> {
    match object.get(key) {
        None => Err(violation(format!("Missing required parameter: '{of}{key}' (InputMissing)."))),
        Some(value) => value
            .as_f64()
            .filter(|number| number.is_finite())
            .ok_or_else(|| violation(format!("Physical Violation: '{of}{key}' must be a finite number."))),
    }
}

/// `segments[<i>].load.<name>` or `segments[*].load.<name>`, read strictly.
fn read_place(text: &str, segments: usize) -> Result<(Option<usize>, String), DoorError> {
    let wrong = || {
        violation(format!(
            "Physical Violation: a place in 'vary' must be segments[<i>].load.<name> or segments[*].load.<name>, the load of a segment; this request has {}. The search sets loads only: never a start value, the step or the steps.",
            Value::String(text.to_string())
        ))
    };
    let rest = text.strip_prefix("segments[").ok_or_else(wrong)?;
    let close = rest.find(']').ok_or_else(wrong)?;
    let index = &rest[..close];
    let name = rest[close + 1..].strip_prefix(".load.").ok_or_else(wrong)?;
    if name.is_empty() || name.len() > 100 || !name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Err(wrong());
    }
    let segment = if index == "*" {
        None
    } else {
        let index = index.parse::<usize>().map_err(|_| wrong())?;
        if index >= segments {
            return Err(violation(format!(
                "Physical Violation: '{text}' names segment {index}, and the request has {segments} segments."
            )));
        }
        Some(index)
    };
    Ok((segment, name.to_string()))
}

/// Whether a value lies inside a field's declared bound.
fn inside(card: &Card, name: &str, value: f64) -> Result<(), String> {
    let Some(field) = card.fields.iter().find(|field| field.name == name) else {
        return Ok(());
    };
    let below = field.minimum.is_some_and(|minimum| if field.minimum_exclusive { value <= minimum } else { value < minimum });
    let above = field.maximum.is_some_and(|maximum| if field.maximum_exclusive { value >= maximum } else { value > maximum });
    if below || above { Err(field.bound.clone()) } else { Ok(()) }
}

fn read(view: &View, body: &Value) -> Result<Plan, DoorError> {
    let object = body.as_object().ok_or_else(|| violation("Physical Violation: the request must be a JSON object."))?;
    for key in object.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(violation(format!(
                "Physical Violation: '{key}' is not a key of design_run; it takes request, vary and target."
            )));
        }
    }
    let request = match object.get("request") {
        None => return Err(violation("Missing required parameter: 'request' (InputMissing).")),
        Some(Value::Object(request)) => request,
        Some(_) => return Err(violation("Physical Violation: 'request' must be a JSON object, a run_trajectory request.")),
    };
    for key in [crate::chain::FROM, crate::readout::READOUT] {
        if request.contains_key(key) {
            return Err(violation(format!(
                "Physical Violation: design_run takes a run_trajectory request written out, with no '{key}'."
            )));
        }
    }
    let (Some(domain), Some(card)) = (request.get("domain").and_then(Value::as_str), request.get("card").and_then(Value::as_str)) else {
        return Err(violation("Physical Violation: 'request' must be a run_trajectory request: domain, card, initial, dt_s, segments, record_every."));
    };
    let card = view.find(domain, card).ok_or_else(|| DoorError::NotFound(crate::CARD_NOT_RECOGNIZED.to_string()))?;
    let segments = request
        .get("segments")
        .and_then(Value::as_array)
        .ok_or_else(|| violation("Physical Violation: 'request' has no 'segments', so it is not a run_trajectory request."))?;
    let steps_each = segments
        .iter()
        .filter_map(|segment| segment.get("steps").and_then(Value::as_u64))
        .fold(0_u64, u64::saturating_add);

    // The loads to set.
    let vary = match object.get("vary") {
        None => return Err(violation("Missing required parameter: 'vary' (InputMissing).")),
        Some(Value::Array(items)) if !items.is_empty() && items.len() <= DESIGN_MAX_LOADS => items,
        Some(Value::Array(_)) => {
            return Err(violation(format!(
                "Physical Violation: 'vary' must list from 1 to {DESIGN_MAX_LOADS} loads (DESIGN_MAX_LOADS)."
            )));
        }
        Some(_) => return Err(violation("Physical Violation: 'vary' must be an array of {\"place\", \"low\", \"high\"}.")),
    };
    let mut knobs: Vec<Knob> = Vec::new();
    let mut start = Vec::new();
    for (index, item) in vary.iter().enumerate() {
        let of = format!("vary[{index}].");
        let item = item.as_object().ok_or_else(|| violation(format!("Physical Violation: 'vary[{index}]' must be a JSON object with place, low and high.")))?;
        for key in item.keys() {
            if !VARY_KEYS.contains(&key.as_str()) {
                return Err(violation(format!("Physical Violation: '{of}{key}' is not a key of a load to set; it takes place, low and high.")));
            }
        }
        let place = match item.get("place") {
            None => return Err(violation(format!("Missing required parameter: '{of}place' (InputMissing)."))),
            Some(Value::String(text)) => text.clone(),
            Some(_) => return Err(violation(format!("Physical Violation: '{of}place' must be a string."))),
        };
        let (segment, name) = read_place(&place, segments.len())?;
        // A search that would set a value of a signed reading is refused, as
        // find_flip's is: every candidate would carry a value its instrument
        // did not sign.
        crate::signed_load::refuse_replaced_in_search(&Value::Object(request.clone()), &place)?;
        let field = card.fields.iter().find(|field| field.name == name && field.role == Role::Load);
        match field {
            None => {
                let loads: Vec<&str> = card.fields.iter().filter(|field| field.role == Role::Load && field.components == 1).map(|field| field.name).collect();
                return Err(violation(format!(
                    "Physical Violation: '{place}' names '{name}', which is not a load of {}. Its loads that are one number: {}.",
                    card.card,
                    if loads.is_empty() { "none, so design_run does not serve this card".to_string() } else { loads.join(", ") }
                )));
            }
            Some(field) if field.components != 1 => {
                return Err(violation(format!(
                    "Physical Violation: '{name}' of {} has {} components; design_run sets a load that is one number.",
                    card.card, field.components
                )));
            }
            Some(_) => {}
        }
        // Each place is set by one entry of vary.
        if knobs.iter().any(|knob| knob.name == name && (knob.segment.is_none() || segment.is_none() || knob.segment == segment)) {
            return Err(violation(format!(
                "Physical Violation: '{place}' sets a load another entry of 'vary' sets; each load of a segment is set once."
            )));
        }
        let low = number_of(item, "low", &of)?;
        let high = number_of(item, "high", &of)?;
        if !(low < high) {
            return Err(violation(format!(
                "Physical Violation: '{of}low' is {} and '{of}high' is {}; a span needs low below high.",
                crate::message_number(low),
                crate::message_number(high)
            )));
        }
        // The span is held inside the card's own bound: no value is moved to it.
        for (end, value) in [("low", low), ("high", high)] {
            if let Err(bound) = inside(card, &name, value) {
                return Err(violation(format!(
                    "Physical Violation: '{of}{end}' is {} and the card's bound of {name} is {bound}. The span is held inside the card's own bound, and no value is moved to it.",
                    crate::message_number(value)
                )));
            }
        }
        // The start is the request's own number at the place: nothing is filled in.
        let indices: Vec<usize> = match segment {
            Some(index) => vec![index],
            None => (0..segments.len()).collect(),
        };
        let mut stated: Option<f64> = None;
        for index in indices {
            let number = segments[index].get("load").and_then(|load| load.get(&name)).and_then(Value::as_f64).ok_or_else(|| {
                violation(format!(
                    "Physical Violation: '{place}' names a load that segment {index} of the request holds no number for. The request is sent whole, with a number at every place to set: it is the search's start, and nothing is filled in."
                ))
            })?;
            match stated {
                None => stated = Some(number),
                Some(first) if first.to_bits() != number.to_bits() => {
                    return Err(violation(format!(
                        "Physical Violation: '{place}' holds one value through every segment, and the request's segments hold {} and {} there. The start is the request as sent, so they are one number.",
                        crate::message_number(first),
                        crate::message_number(number)
                    )));
                }
                Some(_) => {}
            }
        }
        let stated = stated.unwrap_or(f64::NAN);
        if !(low <= stated && stated <= high) {
            return Err(violation(format!(
                "Physical Violation: the request holds {} at '{place}', outside its span from {} to {}. The request as sent is the search's start, so its value lies in the span.",
                crate::message_number(stated),
                crate::message_number(low),
                crate::message_number(high)
            )));
        }
        start.push(stated);
        knobs.push(Knob { place, segment, name, low, high });
    }

    // The target.
    let target = match object.get("target") {
        None => return Err(violation("Missing required parameter: 'target' (InputMissing).")),
        Some(Value::Object(target)) => target,
        Some(_) => return Err(violation("Physical Violation: 'target' must be a JSON object with quantity, value and tolerance.")),
    };
    for key in target.keys() {
        if !TARGET_KEYS.contains(&key.as_str()) {
            return Err(violation(format!("Physical Violation: 'target.{key}' is not a key of target; it takes quantity, value and tolerance.")));
        }
    }
    let quantity = match target.get("quantity") {
        None => return Err(violation("Missing required parameter: 'target.quantity' (InputMissing).")),
        Some(Value::String(text)) if !text.is_empty() && text.len() <= 100 => text.clone(),
        Some(_) => return Err(violation("Physical Violation: 'target.quantity' must be the name of a number of the final state.")),
    };
    let value = number_of(target, "value", "target.")?;
    let tolerance = number_of(target, "tolerance", "target.")?;
    if !(tolerance > 0.0) {
        return Err(violation(format!(
            "Physical Violation: 'target.tolerance' is {}; it must be above zero.",
            crate::message_number(tolerance)
        )));
    }
    // A target the card cannot hold: outside a range the card holds every
    // landed state to (HELD_EVERY_STEP), wider by the tolerance. A range that
    // bounds only the start is not a limit of the end state.
    if HELD_EVERY_STEP.iter().any(|(held_domain, held_card, held)| (*held_domain, *held_card, *held) == (card.domain, card.card, quantity.as_str())) {
        if let Some(field) = card.fields.iter().find(|field| field.name == quantity && field.role == Role::Phase && field.components == 1) {
            let below = field.minimum.is_some_and(|minimum| value + tolerance < minimum || (field.minimum_exclusive && value + tolerance == minimum));
            let above = field.maximum.is_some_and(|maximum| value - tolerance > maximum || (field.maximum_exclusive && value - tolerance == maximum));
            if below || above {
                return Err(violation(format!(
                    "Physical Violation: the target {quantity} of {} within {} lies outside the card's range of {quantity}, {}, which the card holds every landed state to: no run of the card holds that state ({OUTSIDE_CARD}).",
                    crate::message_number(value),
                    crate::message_number(tolerance),
                    field.bound
                )));
            }
        }
    }

    // The caps the request leaves.
    let mut first = DatasetRequest::from_value(candidates_plan(card, &[run_of(&Value::Object(request.clone()))]))
        .map_err(|refusal| violation(refusal.to_string()))?
        .totals();
    first.frames = first.frames.max(1);
    let by_steps = DESIGN_MAX_STEPS / steps_each.max(1);
    let by_frames = DESIGN_MAX_FRAMES / first.frames;
    let (most, cap) = [(DESIGN_MAX_CANDIDATES, "DESIGN_MAX_CANDIDATES"), (by_steps, "DESIGN_MAX_STEPS"), (by_frames, "DESIGN_MAX_FRAMES")]
        .into_iter()
        .min_by_key(|(most, _)| *most)
        .expect("three caps");
    if most < 3 {
        return Err(violation(format!(
            "Physical Violation: the run asks for {steps_each} steps and records {} frames, and a search needs at least three candidates: more than the {DESIGN_MAX_STEPS} steps (DESIGN_MAX_STEPS) or the {DESIGN_MAX_FRAMES} recorded frames (DESIGN_MAX_FRAMES) all the candidates of one search may take together. A shorter run, or a larger record_every, can be designed.",
            first.frames
        )));
    }
    let monotone = monotone_of(card.domain, card.card, &quantity)
        .filter(|declared| knobs.iter().all(|knob| declared.loads.contains(&knob.name.as_str())));
    Ok(Plan {
        body: body.clone(),
        request: Value::Object(request.clone()),
        domain: card.domain,
        card: card.card,
        knobs,
        target: Target { quantity, value, tolerance },
        start,
        most,
        cap,
        steps_each,
        monotone,
    })
}

/// How many steps a search may take in all, for the open connection's count
/// of long work. Zero when the request cannot be read.
pub(crate) fn steps_at_most(arguments: &Map<String, Value>) -> u64 {
    let steps = arguments
        .get("request")
        .and_then(|request| request.get("segments"))
        .and_then(Value::as_array)
        .map(|segments| segments.iter().filter_map(|segment| segment.get("steps").and_then(Value::as_u64)).fold(0_u64, u64::saturating_add))
        .unwrap_or(0);
    steps.saturating_mul(DESIGN_MAX_CANDIDATES).min(DESIGN_MAX_STEPS)
}

/// A run request without its domain and card: a run of a list plan.
fn run_of(request: &Value) -> Value {
    let mut run = request.clone();
    if let Some(object) = run.as_object_mut() {
        object.shift_remove("domain");
        object.shift_remove("card");
    }
    run
}

/// The `run_dataset` request that holds the candidates, in the order tried.
fn candidates_plan(card: &Card, runs: &[Value]) -> Value {
    json!({
        "domain": card.domain,
        "card": card.card,
        "plan": { "kind": "list", "runs": runs },
        "store": "none",
        "return": "inline"
    })
}

/// The request with each set load at its value.
fn with_values(plan: &Plan, values: &[f64]) -> Value {
    let mut request = plan.request.clone();
    if let Some(segments) = request.get_mut("segments").and_then(Value::as_array_mut) {
        for (knob, value) in plan.knobs.iter().zip(values) {
            for (index, segment) in segments.iter_mut().enumerate() {
                if knob.segment.is_some_and(|only| only != index) {
                    continue;
                }
                if let Some(load) = segment.get_mut("load").and_then(Value::as_object_mut) {
                    load.insert(knob.name.clone(), exact(*value));
                }
            }
        }
    }
    request
}

/// One candidate: a timeline and what the engine answered for it.
#[derive(Clone)]
struct Candidate {
    values: Vec<f64>,
    request: Value,
    outcome: String,
    reject: Option<String>,
    reject_detail: Option<String>,
    steps_landed: Option<u64>,
    run_seal: Option<String>,
    /// The target quantity of the final state, when the run completed or
    /// ended at an end its card declares.
    reached: Option<f64>,
    /// The run's own load_source: where its loads came from, as every run
    /// answer says it. None for a candidate refused at the door.
    load_source: Option<Value>,
    result: RowResult,
}

impl Candidate {
    fn miss(&self, target: &Target) -> Option<f64> {
        self.reached.map(|reached| reached - target.value)
    }

    /// -1 below the target, 1 above, 0 within the tolerance, none when the
    /// run gave no value.
    fn side(&self, target: &Target) -> Option<i8> {
        let miss = self.miss(target)?;
        Some(if miss.abs() <= target.tolerance {
            0
        } else if miss < 0.0 {
            -1
        } else {
            1
        })
    }

    fn distance(&self, target: &Target) -> Option<f64> {
        self.miss(target).map(f64::abs)
    }
}

struct Search<'a> {
    plan: &'a Plan,
    tried: Vec<Candidate>,
    /// The index of the best candidate so far: the least distance.
    best: Option<usize>,
    stopped_by: Option<&'static str>,
    hit: Option<usize>,
}

/// Why a candidate was not run: a cap reached.
struct Capped;

impl<'a> Search<'a> {
    fn new(plan: &'a Plan) -> Self {
        Self { plan, tried: Vec::new(), best: None, stopped_by: None, hit: None }
    }

    fn done(&self) -> bool {
        self.hit.is_some() || self.stopped_by.is_some()
    }

    /// The candidate of these values: run once, then found again.
    fn eval(&mut self, values: &[f64]) -> Result<Result<usize, Capped>, String> {
        if let Some(found) = self.tried.iter().position(|candidate| {
            candidate.values.iter().zip(values).all(|(a, b)| a.to_bits() == b.to_bits())
        }) {
            return Ok(Ok(found));
        }
        if self.tried.len() as u64 >= self.plan.most {
            self.stopped_by.get_or_insert(self.plan.cap);
            return Ok(Err(Capped));
        }
        let request = with_values(self.plan, values);
        let candidate = match crate::trajectory::prepare(request.clone()) {
            // The door refuses this candidate before computing: an unadmitted row.
            Err(error) => {
                let sentence = error.message().to_string();
                Candidate {
                    values: values.to_vec(),
                    request,
                    outcome: "unadmitted".to_string(),
                    reject: crate::dataset::name_in_brackets(&sentence),
                    reject_detail: Some(sentence.clone()),
                    steps_landed: None,
                    run_seal: None,
                    reached: None,
                    load_source: None,
                    result: RowResult::Unadmitted(sentence),
                }
            }
            Ok(prepared) => {
                let response = crate::trajectory::execute(prepared)?;
                let text = serde_json::to_string(&response).map_err(|error| format!("Physical Violation: the response could not be encoded. {error}"))?;
                let parsed: Value = serde_json::from_str(&text).map_err(|error| format!("Physical Violation: the response could not be read back. {error}"))?;
                // A run that completed, or that ended at an end its card
                // declares (the event finished), has a final state to compare.
                let reached = if response.outcome == "completed" || response.outcome == "ended" {
                    let state = response.final_state.as_ref().and_then(Value::as_object);
                    let read = state
                        .and_then(|state| state.get(&format!("{}_hex", self.plan.target.quantity)))
                        .and_then(Value::as_str)
                        .and_then(exact_from_hex);
                    match read {
                        Some(read) => Some(read),
                        None => {
                            let names: Vec<&str> = state.map(|state| state.keys().filter_map(|key| key.strip_suffix("_hex")).collect()).unwrap_or_default();
                            return Err(format!(
                                "Physical Violation: 'target.quantity' is '{}', and the final state of {} holds no single number of that name. It holds: {}.",
                                self.plan.target.quantity,
                                self.plan.card,
                                names.join(", ")
                            ));
                        }
                    }
                } else {
                    None
                };
                Candidate {
                    values: values.to_vec(),
                    request,
                    outcome: response.outcome.clone(),
                    reject: response.reject.clone(),
                    reject_detail: response.reject_detail.clone(),
                    steps_landed: Some(response.steps_landed),
                    run_seal: Some(response.run_seal.clone()),
                    reached,
                    load_source: Some(response.load_source.clone()),
                    result: RowResult::Run(parsed),
                }
            }
        };
        let index = self.tried.len();
        let target = &self.plan.target;
        if let Some(distance) = candidate.distance(target) {
            let better = match self.best {
                None => true,
                Some(best) => distance < self.tried[best].distance(target).unwrap_or(f64::INFINITY),
            };
            if better {
                self.best = Some(index);
            }
            if candidate.side(target) == Some(0) && self.hit.is_none() {
                self.hit = Some(index);
            }
        }
        self.tried.push(candidate);
        Ok(Ok(index))
    }

    fn side(&self, index: usize) -> Option<i8> {
        self.tried[index].side(&self.plan.target)
    }

    fn current(&self) -> Vec<f64> {
        match self.best {
            Some(best) => self.tried[best].values.clone(),
            None => self.plan.start.clone(),
        }
    }

    /// Halve between two values of load `k` whose runs lie either side of
    /// the target, until one lands within the tolerance, the two are
    /// neighbours, or a candidate between them gives no value.
    fn bisect(&mut self, base: &[f64], k: usize, mut low: (f64, i8), mut high: (f64, i8)) -> Result<(), String> {
        loop {
            if self.done() {
                return Ok(());
            }
            let middle = low.0 + (high.0 - low.0) / 2.0;
            if !(middle > low.0 && middle < high.0) {
                return Ok(());
            }
            let mut values = base.to_vec();
            values[k] = middle;
            let Ok(index) = self.eval(&values)? else { return Ok(()) };
            match self.side(index) {
                Some(0) => return Ok(()),
                Some(side) if side == low.1 => low = (middle, side),
                Some(side) => high = (middle, side),
                // A stop between two runs either side of the target: this
                // line has no edge to follow, and the best run so far stands.
                None => return Ok(()),
            }
        }
    }

    /// Halve toward the edge where the card stops, from a value that gave a
    /// value (`from`) to one that gave none (`toward`), at most
    /// DESIGN_EDGE_HALVINGS times, looking for a value on the far side of
    /// the target; then bisect there.
    fn toward_edge(&mut self, base: &[f64], k: usize, mut from: (f64, i8), mut toward: f64) -> Result<(), String> {
        for _ in 0..DESIGN_EDGE_HALVINGS {
            if self.done() {
                return Ok(());
            }
            let middle = from.0 + (toward - from.0) / 2.0;
            if middle == from.0 || middle == toward {
                return Ok(());
            }
            let mut values = base.to_vec();
            values[k] = middle;
            let Ok(index) = self.eval(&values)? else { return Ok(()) };
            match self.side(index) {
                Some(0) => return Ok(()),
                None => toward = middle,
                Some(side) if side == from.1 => from = (middle, side),
                Some(side) => {
                    let (low, high) = if from.0 < middle { (from, (middle, side)) } else { ((middle, side), from) };
                    return self.bisect(base, k, low, high);
                }
            }
        }
        Ok(())
    }

    /// The first pair of neighbouring values whose runs complete either side
    /// of the target, the lower value first.
    fn bracket(&self, found: &[(f64, usize)]) -> Option<((f64, i8), (f64, i8))> {
        found.windows(2).find_map(|pair| match (self.side(pair[0].1), self.side(pair[1].1)) {
            (Some(a), Some(b)) if a * b < 0 => Some(((pair[0].0, a), (pair[1].0, b))),
            _ => None,
        })
    }

    /// One line: load `k` across its span, the others held.
    fn line(&mut self, k: usize) -> Result<(), String> {
        let base = self.current();
        let knob = &self.plan.knobs[k];
        let mut points: Vec<f64> = vec![knob.low, base[k], knob.high];
        points.dedup_by(|a, b| a.to_bits() == b.to_bits());
        let mut found: Vec<(f64, usize)> = Vec::new();
        for value in points {
            let mut values = base.clone();
            values[k] = value;
            let Ok(index) = self.eval(&values)? else { return Ok(()) };
            found.push((value, index));
            if self.done() {
                return Ok(());
            }
        }
        // A pair of neighbouring values whose runs lie either side of the target.
        if let Some((low, high)) = self.bracket(&found) {
            return self.bisect(&base, k, low, high);
        }
        // None: the target may lie inside the span where the response turns
        // (a peak, a ring), so DESIGN_SCAN_POINTS values evenly between the
        // ends are run, and the pairs are looked at again.
        let (from, to) = (knob.low, knob.high);
        for point in 1..=DESIGN_SCAN_POINTS {
            let value = from + (to - from) * (point as f64) / ((DESIGN_SCAN_POINTS + 1) as f64);
            if !(value > from && value < to) || found.iter().any(|(tried, _)| tried.to_bits() == value.to_bits()) {
                continue;
            }
            let mut values = base.clone();
            values[k] = value;
            let Ok(index) = self.eval(&values)? else { return Ok(()) };
            found.push((value, index));
            if self.done() {
                return Ok(());
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((low, high)) = self.bracket(&found) {
            return self.bisect(&base, k, low, high);
        }
        // A value that gave a value beside one that gave none: toward the
        // edge, nearest the target first.
        let mut edges: Vec<(f64, (f64, i8), f64)> = Vec::new();
        for pair in found.windows(2) {
            match (self.side(pair[0].1), self.side(pair[1].1)) {
                (Some(a), None) => edges.push((self.tried[pair[0].1].distance(&self.plan.target).unwrap_or(f64::INFINITY), (pair[0].0, a), pair[1].0)),
                (None, Some(b)) => edges.push((self.tried[pair[1].1].distance(&self.plan.target).unwrap_or(f64::INFINITY), (pair[1].0, b), pair[0].0)),
                _ => {}
            }
        }
        edges.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, from, toward) in edges {
            self.toward_edge(&base, k, from, toward)?;
            if self.done() {
                return Ok(());
            }
        }
        Ok(())
    }
}

/// What the search came to.
struct Found {
    outcome: &'static str,
    reject: Option<String>,
    reject_detail: Option<String>,
    /// The run the answer gives: the timeline reached, the corner that falls
    /// short, or the nearest run.
    run: Option<usize>,
    sweeps: u64,
}

fn search(view: &View, plan: &Plan) -> Result<Value, String> {
    let _ = view;
    let mut search = Search::new(plan);
    let target = &plan.target;
    let _ = search.eval(&plan.start)?;
    let mut found: Option<Found> = None;
    // A card whose map is monotone in the loads set: the two corners of the box.
    if let Some(declared) = plan.monotone {
        if !search.done() {
            let allowance = (declared.allowance)(plan.steps_each);
            let highs: Vec<f64> = plan.knobs.iter().map(|knob| knob.high).collect();
            let lows: Vec<f64> = plan.knobs.iter().map(|knob| knob.low).collect();
            for (corner, above) in [(highs, true), (lows, false)] {
                if search.done() {
                    break;
                }
                let Ok(index) = search.eval(&corner)? else { break };
                let candidate = &search.tried[index];
                let Some(reached) = candidate.reached else { continue };
                let short = if above { target.value - target.tolerance > reached + allowance } else { target.value + target.tolerance < reached - allowance };
                if short && search.hit.is_none() {
                    found = Some(Found {
                        outcome: "refused",
                        reject: Some(BEYOND_REACH.to_string()),
                        reject_detail: Some(format!(
                            "Physical Violation: with every load set at its {} end the run completes at {} = {}, and the target is {} within {}. {} No timeline of the loads allowed reaches it ({BEYOND_REACH}).",
                            if above { "high" } else { "low" },
                            target.quantity,
                            crate::message_number(reached),
                            crate::message_number(target.value),
                            crate::message_number(target.tolerance),
                            declared.why
                        )),
                        run: Some(index),
                        sweeps: 0,
                    });
                    break;
                }
            }
        }
    }
    let mut sweeps = 0;
    if found.is_none() {
        while !search.done() && sweeps < DESIGN_MAX_SWEEPS {
            sweeps += 1;
            let before = search.best.and_then(|best| search.tried[best].distance(target));
            for k in 0..plan.knobs.len() {
                search.line(k)?;
                if search.done() {
                    break;
                }
            }
            let after = search.best.and_then(|best| search.tried[best].distance(target));
            if !search.done() && before.is_some() && before == after {
                break;
            }
            if before.is_none() && after.is_none() {
                break;
            }
        }
        if !search.done() && sweeps >= DESIGN_MAX_SWEEPS {
            search.stopped_by = Some("DESIGN_MAX_SWEEPS");
        }
        found = Some(match search.hit {
            Some(hit) => Found { outcome: "reached", reject: None, reject_detail: None, run: Some(hit), sweeps },
            None => Found { outcome: "not_reached", reject: None, reject_detail: None, run: search.best, sweeps },
        });
    }
    let found = found.expect("an outcome");
    answer(plan, &search, found)
}

const ABOUT: &str = "Every candidate is an ordinary run: run.request sent to run_trajectory gives the same run and run_seal, and with that run_seal verify_tape recomputes it. candidates.request is a run_dataset plan of every candidate in the order tried; with its dataset_seal verify_dataset recomputes them all. design_seal is the hash of the text of this request (the rule of a request's text) followed by the JSON {\"outcome\",\"reject\",\"run_seal\",\"dataset_seal\",\"reached_hex\",\"miss_hex\"} in that order with no spaces; the same request gives the same bits. not_reached says what this search found inside its caps, and not that no timeline exists. This whole answer, sent back to verify_tape as it came, is its long form: the design seal is recomputed from its parts, the request is designed again, and the two answers are compared value by value and byte for byte.";

fn answer(plan: &Plan, search: &Search, found: Found) -> Result<Value, String> {
    let target = &plan.target;
    let card = crate::descriptor::find(plan.domain, plan.card).ok_or(crate::CARD_NOT_RECOGNIZED)?;
    let runs: Vec<Value> = search.tried.iter().map(|candidate| run_of(&candidate.request)).collect();
    let plan_request = candidates_plan(card, &runs);
    let dataset = DatasetRequest::from_value(plan_request.clone()).map_err(|refusal| refusal.to_string())?;
    let results: Vec<RowResult> = search.tried.iter().map(|candidate| candidate.result.clone()).collect();
    let assembled = assemble(&dataset, crate::ENGINE_VERSION, &results).map_err(|refusal| format!("Physical Violation: the candidates were not sealed. {refusal}"))?;
    let head = &assembled.head;

    let run = found.run.map(|index| &search.tried[index]);
    let run_value = run.map(|candidate| {
        let miss = candidate.miss(target);
        json!({
            "request": candidate.request,
            "outcome": candidate.outcome,
            "reject": candidate.reject,
            "reject_detail": candidate.reject_detail,
            "steps_landed": candidate.steps_landed,
            "run_seal": candidate.run_seal,
            "load_source": candidate.load_source,
            "reached": candidate.reached.map(|reached| json!({
                "quantity": target.quantity,
                "value": exact(reached),
                "value_hex": hex(reached),
                "miss": miss.map(exact),
                "miss_hex": miss.map(hex),
            })),
        })
    });
    let timeline = run.map(|candidate| {
        let segments = candidate.request.get("segments").and_then(Value::as_array).cloned().unwrap_or_default();
        Value::Array(
            segments
                .iter()
                .enumerate()
                .map(|(index, segment)| json!({ "segment": index, "steps": segment.get("steps"), "load": segment.get("load") }))
                .collect(),
        )
    });
    let rows: Vec<Value> = search
        .tried
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            json!({
                "row": index,
                "set": candidate.values.iter().map(|value| exact(*value)).collect::<Vec<_>>(),
                "outcome": candidate.outcome,
                "reject": candidate.reject,
                "miss": candidate.miss(target).map(exact),
                "run_seal": candidate.run_seal,
            })
        })
        .collect();
    let reached_hex = run.and_then(|candidate| candidate.reached).map(hex);
    let miss_hex = run.and_then(|candidate| candidate.miss(target)).map(hex);
    let run_seal = run.and_then(|candidate| candidate.run_seal.clone());
    let sealed = design_seal(&plan.body, found.outcome, found.reject.as_deref(), run_seal.as_deref(), &head.dataset_seal, reached_hex.as_deref(), miss_hex.as_deref());

    let mut body = Map::new();
    body.insert("tool".to_string(), json!("design_run"));
    body.insert("inputs".to_string(), plan.body.clone());
    body.insert("outcome".to_string(), json!(found.outcome));
    body.insert("reject".to_string(), json!(found.reject));
    body.insert("reject_detail".to_string(), json!(found.reject_detail));
    body.insert(
        "target".to_string(),
        json!({
            "quantity": target.quantity,
            "value": exact(target.value),
            "value_hex": hex(target.value),
            "tolerance": exact(target.tolerance),
            "tolerance_hex": hex(target.tolerance),
            "at": "the final state of a run that completed, after its last step, or that ended at an end its card declares, where the event finished",
        }),
    );
    body.insert("timeline".to_string(), timeline.unwrap_or(Value::Null));
    body.insert("run".to_string(), run_value.unwrap_or(Value::Null));
    body.insert(
        "candidates".to_string(),
        json!({
            "request": plan_request,
            "dataset_seal": head.dataset_seal,
            "rows": head.counts.rows,
            "completed": head.counts.completed,
            "ended": head.counts.ended,
            "refused": head.counts.refused,
            "unadmitted": head.counts.unadmitted,
            "summary": rows,
        }),
    );
    body.insert(
        "search".to_string(),
        json!({
            "method": "coordinate bisection",
            "sets": plan.knobs.iter().map(|knob| knob.place.clone()).collect::<Vec<_>>(),
            "sweeps": found.sweeps,
            "candidates_at_most": plan.most,
            "stopped_by": search.stopped_by,
            "monotone": plan.monotone.map(|declared| declared.why),
        }),
    );
    body.insert("engine_version".to_string(), json!(crate::ENGINE_VERSION));
    body.insert("about".to_string(), json!(ABOUT));
    body.insert("design_seal".to_string(), json!(sealed));
    Ok(Value::Object(body))
}

/// The hash that binds a design: the text of the request, then its outcome,
/// the run given, the candidates' dataset seal, and the value reached and
/// the miss as exact hex.
pub(crate) fn design_seal(
    body: &Value,
    outcome: &str,
    reject: Option<&str>,
    run_seal: Option<&str>,
    dataset_seal: &str,
    reached_hex: Option<&str>,
    miss_hex: Option<&str>,
) -> String {
    #[derive(serde::Serialize)]
    struct Tail<'a> {
        outcome: &'a str,
        reject: Option<&'a str>,
        run_seal: Option<&'a str>,
        dataset_seal: &'a str,
        reached_hex: Option<&'a str>,
        miss_hex: Option<&'a str>,
    }
    let tail = serde_json::to_string(&Tail { outcome, reject, run_seal, dataset_seal, reached_hex, miss_hex }).expect("the tail encodes");
    let mut text = sealed_form(body);
    text.push_str(&tail);
    digest_hex(text.as_bytes())
}

/// The tool. The request is read and held to its caps first; then, under one
/// physics permit, the search is made. The answer ends with its receipt.
pub async fn design(state: &crate::AppState, body: Value) -> Result<String, DoorError> {
    let plan = read(state.view(), &body)?;
    let view = state.view().clone();
    let answer = crate::on_pool(state, move || search(&view, &plan)).await?;
    let document = serde_json::to_string(&answer)
        .map_err(|error| DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}")))?;
    Ok(genesis_seal::receipt::with_receipt(&crate::with_seal_rule(&document)))
}

/// What the long form of a design compared: `verify_tape` given the whole
/// `design_run` answer.
pub const DESIGN_LONG_FORM_COMPUTED: &str = "The design seal was recomputed from the answer's own parts: the text of its request, then its outcome, refusal name, run seal, candidates' dataset seal and the exact hex of the value reached and of the miss. Then its request was designed again under a physics permit and the limits of design_run, the design seal of that design was compared with the answer's, and the engine's whole answer to that request was compared with the answer sent: value by value, and then byte for byte, its timeline, candidates, words, decimals and receipt included. holds is true only when every one is the same, so an answer that holds is, every byte, the engine's design of its request; cause names the first that is not (design_seal, engine_version, or answer_text with differs_at, a text that differs while the design seal holds).";

/// The cause of a design whose seal does not follow from its own parts, or
/// whose request designs to another seal.
pub(crate) const CAUSE_DESIGN_SEAL: &str = "design_seal";

/// What the long form of a design answers.
#[derive(serde::Serialize)]
pub struct LongDesignReport {
    /// Every byte of the answer sent is the engine's design of its request.
    pub holds: bool,
    /// Every value of the answer sent is the engine's design of its request,
    /// designed again, whatever its spelling.
    pub values_hold: bool,
    pub cause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub differs_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difference: Option<String>,
    pub compared: &'static str,
}

impl LongDesignReport {
    fn of(holds: bool, cause: Option<&str>, difference: Option<crate::rebuild::Difference>) -> Self {
        Self {
            holds,
            values_hold: holds,
            cause: cause.map(str::to_string),
            differs_at: difference.as_ref().map(|difference| difference.at.clone()),
            difference: difference.map(|difference| difference.says),
            compared: DESIGN_LONG_FORM_COMPUTED,
        }
    }
}

/// Whether an answer given to `verify_tape` with no `request` is the whole
/// answer of `design_run`, which its long form designs again.
pub(crate) fn is_design_answer(answer: &Map<String, Value>) -> bool {
    answer.get("tool").and_then(Value::as_str) == Some("design_run") && !answer.contains_key("request")
}

/// The design seal an answer's own parts come to, by [`design_seal`]'s rule,
/// or the first part that is not of the form the engine writes.
fn seal_of_parts(answer: &Map<String, Value>) -> Result<String, crate::rebuild::Difference> {
    use crate::rebuild::Difference;
    let not_written = |at: &str| Difference::new(at, format!("'{at}' in the answer sent is not of the form the engine writes, so the design seal cannot be recomputed from it."));
    let text_or_null = |value: Option<&Value>, at: &str| -> Result<Option<String>, Difference> {
        match value {
            Some(Value::String(text)) => Ok(Some(text.clone())),
            Some(Value::Null) => Ok(None),
            _ => Err(not_written(at)),
        }
    };
    let inputs = answer.get("inputs").filter(|inputs| inputs.is_object()).ok_or_else(|| not_written("inputs"))?;
    let outcome = answer.get("outcome").and_then(Value::as_str).ok_or_else(|| not_written("outcome"))?;
    let reject = text_or_null(answer.get("reject"), "reject")?;
    let (run_seal, reached_hex, miss_hex) = match answer.get("run") {
        Some(Value::Null) => (None, None, None),
        Some(Value::Object(run)) => {
            let run_seal = text_or_null(run.get("run_seal"), "run.run_seal")?;
            match run.get("reached") {
                Some(Value::Null) => (run_seal, None, None),
                Some(Value::Object(reached)) => (
                    run_seal,
                    text_or_null(reached.get("value_hex"), "run.reached.value_hex")?,
                    text_or_null(reached.get("miss_hex"), "run.reached.miss_hex")?,
                ),
                _ => return Err(not_written("run.reached")),
            }
        }
        _ => return Err(not_written("run")),
    };
    let dataset_seal = answer
        .get("candidates")
        .and_then(|candidates| candidates.get("dataset_seal"))
        .and_then(Value::as_str)
        .ok_or_else(|| not_written("candidates.dataset_seal"))?;
    Ok(design_seal(inputs, outcome, reject.as_deref(), run_seal.as_deref(), dataset_seal, reached_hex.as_deref(), miss_hex.as_deref()))
}

/// The long form of a design: `verify_tape` given the whole `design_run`
/// answer. The design seal is recomputed from the answer's own parts; then,
/// since only the search writes a design, its request is designed again
/// under a physics permit, and that whole answer, receipt included, is
/// compared with the answer sent.
pub(crate) async fn long_form(state: &crate::AppState, body: &Value, text: Option<&str>) -> Result<LongDesignReport, DoorError> {
    use crate::rebuild::Difference;
    let answer = body.as_object().ok_or_else(|| violation("Physical Violation: the request must be a JSON object."))?;
    let sent_seal = answer.get("design_seal").and_then(Value::as_str);
    match seal_of_parts(answer) {
        Err(difference) => return Ok(LongDesignReport::of(false, Some(CAUSE_DESIGN_SEAL), Some(difference))),
        Ok(seal) if Some(seal.as_str()) != sent_seal => {
            return Ok(LongDesignReport::of(
                false,
                Some(CAUSE_DESIGN_SEAL),
                Some(Difference::new(
                    "design_seal",
                    "The design seal the answer carries is not the hash of its own parts by the rule in its about: a part was changed after it was sealed, or the seal was.",
                )),
            ));
        }
        Ok(_) => {}
    }
    if let Some(version) = answer.get("engine_version").and_then(Value::as_str) {
        if version != crate::ENGINE_VERSION {
            return Ok(LongDesignReport::of(
                false,
                Some(crate::rebuild::CAUSE_ENGINE_VERSION),
                Some(Difference::new(
                    "engine_version",
                    format!(
                        "The answer names engine_version {version}, and this engine is {}. An engine writes only its own answers, so the text of a design of {version} is not compared here and holds is false. Its design seal follows from its own parts. Its inputs, sent to design_run, are designed again by {}.",
                        crate::ENGINE_VERSION,
                        crate::ENGINE_VERSION
                    ),
                )),
            ));
        }
    }
    let inputs = answer.get("inputs").cloned().unwrap_or(Value::Null);
    let plan = match read(state.view(), &inputs) {
        Ok(plan) => plan,
        Err(refused) => {
            return Ok(LongDesignReport::of(
                false,
                Some(crate::rebuild::CAUSE_ANSWER_TEXT),
                Some(Difference::new("inputs", format!("The answer's inputs are not a request design_run takes, so the engine writes no design for them: {}", refused.message()))),
            ));
        }
    };
    let view = state.view().clone();
    let again = match crate::on_pool(state, move || search(&view, &plan)).await {
        Ok(again) => again,
        Err(DoorError::TooMany(message)) => return Err(DoorError::TooMany(message)),
        Err(refused) => {
            return Ok(LongDesignReport::of(
                false,
                Some(crate::rebuild::CAUSE_ANSWER_TEXT),
                Some(Difference::new("outcome", format!("The request was designed again and the engine answers it with no design: {}", refused.message()))),
            ));
        }
    };
    if again.get("design_seal").and_then(Value::as_str) != sent_seal {
        return Ok(LongDesignReport::of(
            false,
            Some(CAUSE_DESIGN_SEAL),
            Some(Difference::new(
                "design_seal",
                "The request was designed again and designs to another design_seal: the answer is consistent with itself, and its parts are not the engine's design of this request.",
            )),
        ));
    }
    let document = serde_json::to_string(&again)
        .map_err(|error| DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}")))?;
    let engine_text = genesis_seal::receipt::with_receipt(&crate::with_seal_rule(&document));
    let sent_text = crate::rebuild::sent_text(body, text);
    let found = crate::rebuild::compare(&engine_text, body, &sent_text)?;
    Ok(match found.difference {
        Some(difference) => LongDesignReport {
            values_hold: found.values_hold,
            ..LongDesignReport::of(false, Some(crate::rebuild::CAUSE_ANSWER_TEXT), Some(difference))
        },
        None => LongDesignReport::of(true, None, None),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plate(power: f64, steps: u64) -> Value {
        json!({
            "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 10.0,
            "segments": [{ "steps": steps, "load": { "heat_power_w": power, "ambient_k": 293.15 } }], "record_every": steps
        })
    }

    pub(super) fn designed(body: Value) -> Value {
        let view = View::full();
        let plan = read(&view, &body).unwrap_or_else(|error| panic!("{}", error.message()));
        search(&view, &plan).unwrap()
    }

    fn refusal(body: Value) -> String {
        read(&View::full(), &body).err().map(|error| error.message().to_string()).expect("refused")
    }

    /// A load an instrument signed is run as it was signed: a design that
    /// would set it is refused by name before any run, in one segment or in
    /// every one.
    #[test]
    fn a_design_that_would_set_a_signed_load_is_refused_by_name() {
        let mut request = plate(0.0, 60);
        request["segments"][0]["signed"] = json!({ "alg": "ed25519" });
        for place in ["segments[0].load.heat_power_w", "segments[*].load.heat_power_w"] {
            let said = refusal(json!({
                "request": request,
                "vary": [{ "place": place, "low": 0.0, "high": 40.0 }],
                "target": { "quantity": "temperature_k", "value": 315.0, "tolerance": 0.01 }
            }));
            assert!(said.ends_with("(SignedLoadReplaced)."), "{place}: {said}");
            assert!(said.contains("'segments[0].signed'"), "{said}");
        }
        assert_eq!(refusal(json!({ "vary": [], "target": {} })), "Missing required parameter: 'request' (InputMissing).");
    }

    #[test]
    fn the_plate_reaches_a_temperature_at_a_time_with_bounded_heater_power() {
        // 315 K at 600 s (60 steps of 10 s), heater 0 to 40 W. The plate's
        // time constant is 1,181.25 s, so 40 W held gives 325.0 K there.
        let body = json!({
            "request": plate(0.0, 60),
            "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 40.0 }],
            "target": { "quantity": "temperature_k", "value": 315.0, "tolerance": 0.01 }
        });
        let answer = designed(body.clone());
        assert_eq!(answer["outcome"], "reached", "{answer}");
        let miss = answer["run"]["reached"]["miss"].as_f64().unwrap();
        assert!(miss.abs() <= 0.01, "{miss}");
        let power = answer["timeline"][0]["load"]["heat_power_w"].as_f64().unwrap();
        assert!(0.0 < power && power < 40.0);
        // The run given is the run of its request sent alone.
        let alone = crate::trajectory::execute(crate::trajectory::prepare(answer["run"]["request"].clone()).unwrap()).unwrap();
        assert_eq!(answer["run"]["run_seal"], json!(alone.run_seal));
        // The same request gives the same bits.
        assert_eq!(answer, designed(body));
        // Every candidate holds its power inside the span: none is moved there.
        for row in answer["candidates"]["summary"].as_array().unwrap() {
            let set = row["set"][0].as_f64().unwrap();
            assert!((0.0..=40.0).contains(&set), "{row}");
        }
        assert!(answer["candidates"]["rows"].as_u64().unwrap() <= DESIGN_MAX_CANDIDATES);
        assert!(answer["search"]["monotone"].is_string());
    }

    #[test]
    fn a_target_the_heater_cannot_reach_is_refused_by_name_with_the_corner_that_falls_short() {
        // 40 W for 600 s cannot bring the plate to 333.15 K: the most it gives
        // is the high corner, 325.0 K.
        let answer = designed(json!({
            "request": plate(0.0, 60),
            "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 40.0 }],
            "target": { "quantity": "temperature_k", "value": 333.15, "tolerance": 0.01 }
        }));
        assert_eq!((answer["outcome"].as_str(), answer["reject"].as_str()), (Some("refused"), Some(BEYOND_REACH)), "{answer}");
        assert_eq!(answer["run"]["request"]["segments"][0]["load"]["heat_power_w"], 40.0);
        assert!(answer["run"]["reached"]["value"].as_f64().unwrap() < 333.15);
        // The two corners and the start: no search past the proof.
        assert_eq!(answer["candidates"]["rows"], 2);
        assert!(answer["reject_detail"].as_str().unwrap().contains("(TargetBeyondReach)"));
    }

    #[test]
    fn a_target_outside_the_cards_range_is_refused_before_any_run() {
        let message = refusal(json!({
            "request": plate(0.0, 60),
            "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 200.0 }],
            "target": { "quantity": "temperature_k", "value": 450.0, "tolerance": 0.01 }
        }));
        assert!(message.contains("(TargetOutsideCard)"), "{message}");
        // A range that bounds only the start is not a limit of the end: the
        // grid's frequency below its start range is searched for, not refused.
        let described = crate::descriptor::describe_value_in(&View::full(), "energy", "grid_gb").unwrap();
        let run = described["example_run_trajectory"].clone();
        let loss = run["segments"][0]["load"]["loss_mw"].as_f64().unwrap();
        let body = json!({
            "request": run,
            "vary": [{ "place": "segments[*].load.loss_mw", "low": loss, "high": loss }],
            "target": { "quantity": "frequency_hz", "value": 49.7, "tolerance": 0.001 }
        });
        let said = refusal(body);
        assert!(!said.contains("TargetOutsideCard") && said.contains("low below high"), "{said}");
    }

    #[test]
    fn a_candidate_past_the_ceiling_is_recorded_refused_and_the_search_goes_under_it() {
        // Up to 200 W: the high end overheats the plate past 398.15 K, which
        // is refused (200 W held for 600 s would take it to about 452 K); the
        // target 390 K is reached below that edge.
        let answer = designed(json!({
            "request": plate(0.0, 60),
            "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 200.0 }],
            "target": { "quantity": "temperature_k", "value": 390.0, "tolerance": 0.01 }
        }));
        assert_eq!(answer["outcome"], "reached", "{answer}");
        let rows = answer["candidates"]["summary"].as_array().unwrap();
        let refused: Vec<&Value> = rows.iter().filter(|row| row["outcome"] == "refused").collect();
        assert!(!refused.is_empty(), "{answer}");
        for row in &refused {
            assert_eq!(row["reject"], "TemperatureOutsideCard");
            assert!(row["run_seal"].as_str().is_some_and(|seal| seal.len() == 96));
        }
        // The high end is in the record at 200 W as tried: not moved back.
        assert!(rows.iter().any(|row| row["set"][0] == 200.0 && row["outcome"] == "refused"));
        assert!(answer["candidates"]["refused"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn a_timeline_of_three_segments_is_found_by_sweeps() {
        let request = json!({
            "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 10.0,
            "segments": [
                { "steps": 20, "load": { "heat_power_w": 0.0, "ambient_k": 293.15 } },
                { "steps": 20, "load": { "heat_power_w": 0.0, "ambient_k": 293.15 } },
                { "steps": 20, "load": { "heat_power_w": 0.0, "ambient_k": 293.15 } }
            ],
            "record_every": 60
        });
        // Each segment's heater up to 15 W, all three at 15 W give 305.1 K:
        // no one segment alone reaches 300 K, so the sweeps set more than one.
        let answer = designed(json!({
            "request": request,
            "vary": [
                { "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 15.0 },
                { "place": "segments[1].load.heat_power_w", "low": 0.0, "high": 15.0 },
                { "place": "segments[2].load.heat_power_w", "low": 0.0, "high": 15.0 }
            ],
            "target": { "quantity": "temperature_k", "value": 300.0, "tolerance": 0.005 }
        }));
        assert_eq!(answer["outcome"], "reached", "{answer}");
        assert_eq!(answer["timeline"].as_array().unwrap().len(), 3);
        let set: Vec<f64> = (0..3).map(|i| answer["timeline"][i]["load"]["heat_power_w"].as_f64().unwrap()).collect();
        assert!(set.iter().filter(|power| **power > 0.0).count() >= 2, "{set:?}");
        assert!(answer["run"]["reached"]["miss"].as_f64().unwrap().abs() <= 0.005);
    }

    #[test]
    fn the_candidates_are_sealed_as_a_dataset_the_engine_recomputes() {
        let answer = designed(json!({
            "request": plate(0.0, 60),
            "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 200.0 }],
            "target": { "quantity": "temperature_k", "value": 390.0, "tolerance": 0.01 }
        }));
        let plan = DatasetRequest::from_value(answer["candidates"]["request"].clone()).unwrap();
        let mut results = Vec::new();
        for index in 0..plan.totals().rows {
            let row = plan.row(index).unwrap();
            let response = crate::trajectory::execute(crate::trajectory::prepare(row).unwrap()).unwrap();
            results.push(RowResult::Run(serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap()));
        }
        let again = assemble(&plan, crate::ENGINE_VERSION, &results).unwrap();
        assert_eq!(answer["candidates"]["dataset_seal"], json!(again.head.dataset_seal));
        // The design seal follows its rule.
        let run = &answer["run"];
        let sealed = design_seal(
            &answer["inputs"],
            answer["outcome"].as_str().unwrap(),
            answer["reject"].as_str(),
            run["run_seal"].as_str(),
            answer["candidates"]["dataset_seal"].as_str().unwrap(),
            run["reached"]["value_hex"].as_str(),
            run["reached"]["miss_hex"].as_str(),
        );
        assert_eq!(answer["design_seal"], json!(sealed));
    }

    #[test]
    fn the_fuse_link_is_heated_to_a_temperature_at_a_time_with_a_bounded_load_current() {
        let described = crate::descriptor::describe_value_in(&View::full(), "energy", "grid_node").unwrap();
        let mut run = described["example_run_trajectory"].clone();
        // The link at rest: since 0.11.0 the worked run starts it at 10 A on 48 V.
        run["initial"] = json!({ "current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15 });
        run["segments"] = json!([{ "steps": 2000, "load": { "upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 293.15 } }]);
        run["record_every"] = json!(2000);
        let answer = designed(json!({
            "request": run,
            "vary": [{ "place": "segments[0].load.load_current_a", "low": 0.0, "high": 40.0 }],
            "target": { "quantity": "temperature_k", "value": 300.0, "tolerance": 0.01 }
        }));
        assert_eq!(answer["outcome"], "reached", "{answer}");
        assert!(answer["search"]["monotone"].is_null());
        eprintln!("fuse: {} A, {} candidates", answer["timeline"][0]["load"]["load_current_a"], answer["candidates"]["rows"]);
    }

    #[test]
    fn the_node_voltage_is_brought_to_a_value_and_a_ring_past_the_bound_is_recorded_refused() {
        // The upstream voltage up to the card's bound: held from rest, a
        // voltage above about 0.55 of the bound rings the node past it, and
        // that candidate is refused as VoltageOutsideCard and recorded.
        let described = crate::descriptor::describe_value_in(&View::full(), "energy", "grid_node").unwrap();
        let bound = crate::descriptor::grid_melt_voltage_v(&genesis_kernel::grid_node::GridNode::copper_fuse_link());
        let mut run = described["example_run_trajectory"].clone();
        run["segments"] = json!([{ "steps": 500, "load": { "upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 293.15 } }]);
        run["record_every"] = json!(500);
        let answer = designed(json!({
            "request": run,
            "vary": [{ "place": "segments[0].load.upstream_voltage_v", "low": 0.0, "high": bound }],
            "target": { "quantity": "voltage_v", "value": 48.0, "tolerance": 0.001 }
        }));
        assert_eq!(answer["outcome"], "reached", "{answer}");
        let rows = answer["candidates"]["summary"].as_array().unwrap();
        assert!(rows.iter().any(|row| row["outcome"] == "refused" && row["reject"] == "VoltageOutsideCard"), "{answer}");
        eprintln!("node: {} V upstream, {} candidates", answer["timeline"][0]["load"]["upstream_voltage_v"], answer["candidates"]["rows"]);
    }

    #[test]
    fn a_request_out_of_shape_is_named_and_nothing_is_filled_in() {
        let vary = |place: &str, low: f64, high: f64| json!([{ "place": place, "low": low, "high": high }]);
        let target = json!({ "quantity": "temperature_k", "value": 330.0, "tolerance": 0.01 });
        let body = |request: Value, vary: Value| json!({ "request": request, "vary": vary, "target": target.clone() });
        // A load the request holds no number for is not filled in.
        let mut bare = plate(0.0, 60);
        bare["segments"][0]["load"].as_object_mut().unwrap().remove("heat_power_w");
        assert!(refusal(body(bare, vary("segments[0].load.heat_power_w", 0.0, 10.0))).contains("nothing is filled in"));
        // The search sets loads only.
        assert!(refusal(body(plate(0.0, 60), vary("initial.temperature_k", 250.0, 300.0))).contains("sets loads only"));
        assert!(refusal(body(plate(0.0, 60), vary("dt_s", 1.0, 2.0))).contains("sets loads only"));
        // A span outside the card's bound is refused, not moved.
        assert!(refusal(body(plate(0.0, 60), vary("segments[0].load.ambient_k", 200.0, 300.0))).contains("no value is moved to it"));
        // A start outside its span.
        assert!(refusal(body(plate(50.0, 60), vary("segments[0].load.heat_power_w", 0.0, 10.0))).contains("outside its span"));
        // A load the card does not have.
        assert!(refusal(body(plate(0.0, 60), vary("segments[0].load.current_a", 0.0, 10.0))).contains("not a load of thermal"));
        // The same load twice.
        let twice = json!([{ "place": "segments[*].load.heat_power_w", "low": 0.0, "high": 1.0 }, { "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 1.0 }]);
        assert!(refusal(body(plate(0.0, 60), twice)).contains("set once"));
        // A tolerance of zero.
        assert!(refusal(json!({ "request": plate(0.0, 60), "vary": vary("segments[0].load.heat_power_w", 0.0, 10.0), "target": { "quantity": "temperature_k", "value": 330.0, "tolerance": 0.0 } })).contains("above zero"));
        // A run too long for three candidates.
        assert!(refusal(body(plate(0.0, 400_000), vary("segments[0].load.heat_power_w", 0.0, 10.0))).contains("DESIGN_MAX_STEPS"));
        // An unknown key, a chained request.
        assert!(refusal(json!({ "request": plate(0.0, 60), "vary": vary("segments[0].load.heat_power_w", 0.0, 10.0), "target": target, "seed": 1 })).contains("'seed'"));
        let mut chained = plate(0.0, 60);
        chained["from"] = json!({});
        assert!(refusal(body(chained, vary("segments[0].load.heat_power_w", 0.0, 10.0))).contains("no 'from'"));
        // A quantity the final state does not hold is named after the first run.
        let view = View::full();
        let plan = read(&view, &json!({ "request": plate(0.0, 60), "vary": vary("segments[0].load.heat_power_w", 0.0, 10.0), "target": { "quantity": "no_such", "value": 1.0, "tolerance": 0.1 } })).unwrap();
        let error = search(&view, &plan).err().unwrap();
        assert!(error.contains("'no_such'") && error.contains("temperature_k"), "{error}");
    }

    /// On every served card: the card's worked run, its first load that is
    /// one number swept over a span about its worked value, and a target
    /// that a run inside the span is known to reach (the final value of the
    /// card's first phase quantity at a point of the span). The search starts
    /// from the low end and is held to reach it. The table it prints is the
    /// list in the report of which cards the tool serves and how.
    pub(super) fn sweep_of_every_served_card() -> Vec<(String, String, String)> {
        let view = View::full();
        let mut table = Vec::new();
        for (domain, name, loads) in served_cards(&view) {
            let card = crate::descriptor::find(domain, name).unwrap();
            let described = crate::descriptor::describe_value_in(&view, domain, name).unwrap();
            let run = described["example_run_trajectory"].clone();
            let quantity = card
                .fields
                .iter()
                .find(|field| field.role == Role::Phase && field.components == 1)
                .map(|field| field.name.to_string());
            let mut said = "no span".to_string();
            for load in &loads {
                let Some(value) = run["segments"][0]["load"][load].as_f64() else { continue };
                // A span about the worked value inside the card's bound: both
                // sides, then one, halving the width until it fits.
                let mut width = if value == 0.0 { 1.0 } else { 0.25 * value.abs() };
                let mut spans = Vec::new();
                for _ in 0..12 {
                    for (low, high) in [(value - width, value + width), (value, value + width), (value - width, value)] {
                        if inside(card, load, low).is_ok() && inside(card, load, high).is_ok() {
                            spans.push((low, high));
                        }
                    }
                    width /= 2.0;
                }
                let at = |power: f64| {
                    let mut request = run.clone();
                    for segment in request["segments"].as_array_mut().unwrap() {
                        segment["load"][load.to_string()] = exact(power);
                    }
                    crate::trajectory::prepare(request).ok().and_then(|prepared| crate::trajectory::execute(prepared).ok())
                };
                let completes = |power: f64| at(power).is_some_and(|run| run.outcome == "completed" || run.outcome == "ended");
                // The first span whose low end and a point at 0.7 of it both complete.
                let Some((low, high)) = spans.into_iter().find(|(low, high)| completes(*low) && completes(low + 0.7 * (high - low))) else {
                    said = format!("{load}: no span about {value} inside the card's bound where the runs complete");
                    continue;
                };
                // Every segment set to one value held through: the start at the low end.
                let place = format!("segments[*].load.{load}");
                let mut start = run.clone();
                for segment in start["segments"].as_array_mut().unwrap() {
                    segment["load"][load.to_string()] = exact(low);
                }
                let point = low + 0.7 * (high - low);
                let known = at(point).unwrap();
                let state = known.final_state.clone().unwrap_or(Value::Null);
                let low_state = at(low).and_then(|run| run.final_state).unwrap_or(Value::Null);
                let read_hex = |state: &Value, quantity: &str| state.get(format!("{quantity}_hex")).and_then(Value::as_str).and_then(exact_from_hex);
                // The card's first phase quantity that moves with the load, or
                // else the first number of the final state that does.
                let moves = |quantity: &str| match (read_hex(&state, quantity), read_hex(&low_state, quantity)) {
                    (Some(a), Some(b)) => a != b && a.is_finite() && b.is_finite(),
                    _ => false,
                };
                let chosen = quantity.clone().filter(|quantity| moves(quantity)).or_else(|| {
                    let keys: Vec<String> = state.as_object().map(|object| object.keys().filter_map(|key| key.strip_suffix("_hex").map(str::to_string)).collect()).unwrap_or_default();
                    keys.into_iter().find(|key| moves(key))
                });
                let Some(quantity) = chosen else {
                    said = format!("{load}: no number of the final state moves with {load} over the span");
                    continue;
                };
                let value = read_hex(&state, &quantity).unwrap();
                let tolerance = (read_hex(&low_state, &quantity).unwrap() - value).abs() * 1.0e-3;
                let body = json!({
                    "request": start,
                    "vary": [{ "place": place, "low": low, "high": high }],
                    "target": { "quantity": quantity, "value": value, "tolerance": tolerance }
                });
                let plan = match read(&view, &body) {
                    Ok(plan) => plan,
                    Err(error) => {
                        said = format!("{load}: {}", error.message());
                        continue;
                    }
                };
                let answer = search(&view, &plan).unwrap();
                said = format!(
                    "{load} -> {quantity}: {} in {} candidates",
                    answer["outcome"].as_str().unwrap(),
                    answer["candidates"]["rows"]
                );
                break;
            }
            table.push((domain.to_string(), name.to_string(), said));
        }
        table
    }

    #[test]
    fn every_served_card_reaches_a_target_a_run_of_its_span_is_known_to_reach() {
        let table = sweep_of_every_served_card();
        for (domain, card, said) in &table {
            eprintln!("| {domain}/{card} | {said} |");
        }
        for (domain, card, said) in &table {
            if said.contains(" -> ") {
                assert!(said.contains(": reached in "), "{domain}/{card}: {said}");
            }
        }
    }

    #[test]
    fn the_map_states_the_caps_in_force() {
        // The map is on the disk and not in Git: where it is present it states each cap.
        let Ok(page) = std::fs::read_to_string(format!("{}/AGENTS.md", env!("CARGO_MANIFEST_DIR"))) else { return };
        for stated in [
            format!("`DESIGN_MAX_CANDIDATES` = {DESIGN_MAX_CANDIDATES}"),
            format!("`DESIGN_MAX_STEPS` = {DESIGN_MAX_STEPS}"),
            format!("`DESIGN_MAX_FRAMES` = {DESIGN_MAX_FRAMES}"),
            format!("`DESIGN_MAX_SWEEPS` = {DESIGN_MAX_SWEEPS}"),
            format!("`DESIGN_MAX_LOADS` = {DESIGN_MAX_LOADS}"),
            format!("`DESIGN_SCAN_POINTS` = {DESIGN_SCAN_POINTS}"),
            format!("`DESIGN_EDGE_HALVINGS` = {DESIGN_EDGE_HALVINGS}"),
        ] {
            assert!(page.contains(&stated), "AGENTS.md does not state {stated}");
        }
    }

    #[test]
    fn the_served_cards_are_those_with_a_load_that_is_one_number() {
        let served = served_cards(&View::full());
        assert!(served.iter().any(|(_, card, loads)| *card == "thermal" && loads.contains(&"heat_power_w")));
        assert!(served.iter().any(|(_, card, loads)| *card == "grid_node" && loads.contains(&"load_current_a")));
        for (domain, card, loads) in &served {
            eprintln!("{domain}/{card}: {}", loads.join(", "));
        }
        let all = View::full().cards();
        for card in all {
            if !served.iter().any(|(_, name, _)| *name == card.card) {
                eprintln!("not served: {}/{}", card.domain, card.card);
            }
        }
    }
}
