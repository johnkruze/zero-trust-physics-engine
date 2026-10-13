//! `find_flip`: where an outcome changes.
//!
//! Given a `run_trajectory` or `solve` request, one number of it to vary and
//! a span for that number, the tool halves the span until it holds the place
//! where the outcome changes: a run that completes on one side and stops on
//! the other, one stop name giving way to another, a run that ends where its
//! neighbour stops, or a published quantity crossing a level. It returns the
//! run on each side, each as a request ready to send again with the
//! cryptographic proof that request has, and the width of what lies between.
//!
//! The search is the same every time: the two ends first, then the middle of
//! what is left, the middle taken as `low + (high - low) / 2` in 64-bit
//! arithmetic, until the two values are neighbouring 64-bit numbers or a cap
//! is reached. Every run of it is an ordinary run of the same request typed
//! by hand, with the same proof. All of it is computed under one physics
//! permit. The caps, each named in the answer when it stops the search:
//!
//! - [`FLIP_MAX_RUNS`] runs in one search, the two ends included;
//! - [`FLIP_MAX_STEPS`] steps in all its runs together, the engine's limit
//!   for one run;
//! - [`FLIP_MAX_SOLVES`] solves in one search, the two ends included;
//! - [`FLIP_MAX_SECONDS`] of computing, after which no new run or solve is
//!   started: what bounds a search over a slow solver.
//!
//! Two things shorten a search without changing what it finds. `low` and
//! `high` are taken in either order. And where the number varied is the
//! steps of one segment, a run that stopped inside that segment says how
//! many steps landed, so the search tries that many and one more before it
//! halves again: the last day of a mission is found in four runs.

use serde_json::{Map, Value, json};

use crate::live::DoorError;
use crate::view::View;

/// The most runs one search makes, the two ends included.
pub const FLIP_MAX_RUNS: u64 = 64;
/// The most steps of all the runs of one search together.
pub const FLIP_MAX_STEPS: u64 = crate::trajectory::MAX_TRAJECTORY_STEPS;
/// The most solves one search makes, the two ends included.
pub const FLIP_MAX_SOLVES: u64 = 64;

/// When both ends of a span give the same, this many values evenly between
/// them are run before the search says there is no change (for solves,
/// [`FLIP_SCAN_SOLVES`]).
pub const FLIP_SCAN_RUNS: u64 = 7;
pub const FLIP_SCAN_SOLVES: u64 = 3;
/// A search starts no further run or solve once it has computed this long.
/// It is the one cap that depends on the machine: when it stops a search the
/// answer names it, and the pair is the pair found so far.
pub const FLIP_MAX_SECONDS: u64 = 20;

const KEYS: &[&str] = &["request", "vary", "low", "high", "level"];

/// The name a stop's sentence gives in round brackets, when it gives one.
fn name_in(sentence: &str) -> Option<&str> {
    let mut found = None;
    let mut rest = sentence;
    while let Some(open) = rest.find('(') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(')') else { break };
        let inner = &after[..close];
        if inner.len() > 3
            && inner.chars().next().is_some_and(|first| first.is_ascii_uppercase())
            && inner.chars().all(|ch| ch.is_ascii_alphanumeric())
            && inner.chars().any(|ch| ch.is_ascii_lowercase())
        {
            found = Some(inner);
        }
        rest = &after[close + 1..];
    }
    found
}

pub fn description() -> String {
    format!(
        "Find where an outcome changes. Send request (a run_trajectory or solve request written out, its domain and card or its solver by the name find_model or the lists give; no from, readout or example), vary (the place of the one number to change: \"segments[0].load.current_a\", \"segments[*].load.current_a\" for every segment, \"initial.soc\", \"dt_s\", \"segments[0].steps\", \"parameters.load_n\"; a count is varied in whole numbers), and low and high, the two ends of the span, in either order: the smaller is taken as low. The span is halved to the place where the outcome or the stop name changes or, with level {{\"quantity\": <a published quantity>, \"value\": <number>}}, where that quantity of the final state (of a solve: of result) crosses the value, or, with level {{\"quantity\": <a flag>}}, where a true-or-false value of it turns. When both ends give the same, {FLIP_SCAN_RUNS} values between them ({FLIP_SCAN_SOLVES} for solves) are run to find a band, and the answer says so. The answer gives the run on each side as a request ready to send again, with its outcome, its stop name and sentence and the run_seal or solution_seal that request has when sent alone, the two values in decimals and exact hex, and the width between them. The same request gives the same pair. Bounded, and named when one stops the search: {FLIP_MAX_RUNS} runs (FLIP_MAX_RUNS), {FLIP_MAX_STEPS} steps in all (FLIP_MAX_STEPS), or {FLIP_MAX_SOLVES} solves (FLIP_MAX_SOLVES), the ends included, and no new run after {FLIP_MAX_SECONDS} seconds of computing (FLIP_MAX_SECONDS); send the pair back as the span to narrow it. A side that stops before computing has no proof. Same bytes as POST /find_flip."
    )
}

pub fn schema() -> Map<String, Value> {
    match json!({
        "type": "object",
        "properties": {
            "request": { "type": "object", "description": "A run_trajectory or solve request as that tool takes it." },
            "vary": { "type": "string", "description": "The place of the one number to vary." },
            "low": { "type": "number", "description": "One end of the span. low and high may come in either order; the smaller is taken as low." },
            "high": { "type": "number", "description": "The other end." },
            "level": {
                "type": "object",
                "properties": { "quantity": { "type": "string" }, "value": { "type": ["number", "boolean"] } },
                "required": ["quantity"],
                "additionalProperties": false
            }
        },
        "required": ["request", "vary", "low", "high"],
        "additionalProperties": false
    }) {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

fn violation(text: impl Into<String>) -> DoorError {
    DoorError::Unprocessable(text.into())
}

#[derive(Clone, Debug, PartialEq)]
enum Seg {
    Key(String),
    Index(usize),
    Every,
}

/// The place `vary` names, read strictly.
fn read_place(text: &str) -> Result<Vec<Seg>, DoorError> {
    let wrong = || {
        violation(format!(
            "Physical Violation: 'vary' must be the place of one number of the request, such as \"segments[0].load.current_a\", \"initial.soc\", \"dt_s\" or \"parameters.load_n\"; this request has {}.",
            Value::String(text.to_string())
        ))
    };
    if text.is_empty() || text.len() > 200 {
        return Err(wrong());
    }
    let mut out = Vec::new();
    for part in text.split('.') {
        let (name, mut rest) = match part.find('[') {
            Some(open) => (&part[..open], &part[open..]),
            None => (part, ""),
        };
        if name.is_empty() || !name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
            return Err(wrong());
        }
        out.push(Seg::Key(name.to_string()));
        while !rest.is_empty() {
            let close = rest.find(']').ok_or_else(wrong)?;
            let inner = &rest[1..close];
            if inner == "*" {
                out.push(Seg::Every);
            } else {
                out.push(Seg::Index(inner.parse::<usize>().map_err(|_| wrong())?));
            }
            rest = &rest[close + 1..];
            if !rest.is_empty() && !rest.starts_with('[') {
                return Err(wrong());
            }
        }
    }
    Ok(out)
}

/// Every number of `value` at the place, mutably. `[*]` takes each item.
fn places<'a>(value: &'a mut Value, place: &[Seg], out: &mut Vec<&'a mut Value>) {
    let Some((first, rest)) = place.split_first() else {
        out.push(value);
        return;
    };
    match first {
        Seg::Key(name) => {
            if let Some(next) = value.get_mut(name.as_str()) {
                places(next, rest, out);
            }
        }
        Seg::Index(index) => {
            if let Some(next) = value.get_mut(*index) {
                places(next, rest, out);
            }
        }
        Seg::Every => {
            if let Some(items) = value.as_array_mut() {
                for item in items {
                    places(item, rest, out);
                }
            }
        }
    }
}

/// The request with the varied number set to `number` at every place.
fn with_value(request: &Value, place: &[Seg], number: f64, whole: bool) -> Value {
    let mut request = request.clone();
    let written = if whole {
        json!(number as u64)
    } else {
        serde_json::Number::from_f64(number).map(Value::Number).unwrap_or(Value::Null)
    };
    let mut found = Vec::new();
    places(&mut request, place, &mut found);
    for slot in found {
        *slot = written.clone();
    }
    request
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Run,
    Solve,
}

struct Level {
    quantity: String,
    /// A number to cross, or nothing for a flag: then what is compared is
    /// whether the flag is true.
    value: Option<f64>,
}

struct Plan {
    tool: Tool,
    request: Value,
    place: Vec<Seg>,
    vary: String,
    low: f64,
    high: f64,
    level: Option<Level>,
    /// The most runs or solves the caps leave.
    most: u64,
    /// Steps one run asks for; 0 for a solve.
    steps_each: u64,
    cap: &'static str,
    /// The varied number is a count: every value tried is a whole number.
    whole: bool,
    /// The varied number is the steps of this one segment of a run.
    steps_segment: Option<usize>,
}

fn number_of(object: &Map<String, Value>, key: &str) -> Result<f64, DoorError> {
    match object.get(key) {
        None => Err(violation(format!("Missing required parameter: '{key}'"))),
        Some(value) => value
            .as_f64()
            .filter(|number| number.is_finite())
            .ok_or_else(|| violation(format!("Physical Violation: '{key}' must be a finite number."))),
    }
}

fn read(view: &View, body: &Value) -> Result<Plan, DoorError> {
    let object = body
        .as_object()
        .ok_or_else(|| violation("Physical Violation: the request must be a JSON object."))?;
    for key in object.keys() {
        if !KEYS.contains(&key.as_str()) {
            return Err(violation(format!(
                "Physical Violation: '{key}' is not a key of find_flip; it takes request, vary, low, high and level."
            )));
        }
    }
    let request = match object.get("request") {
        None => return Err(violation("Missing required parameter: 'request'")),
        Some(Value::Object(request)) => request,
        Some(_) => return Err(violation("Physical Violation: 'request' must be a JSON object, a run_trajectory or solve request.")),
    };
    for key in [crate::chain::FROM, crate::readout::READOUT, crate::solution::EXAMPLE] {
        if request.contains_key(key) {
            return Err(violation(format!(
                "Physical Violation: find_flip takes a run_trajectory or solve request written out, with no '{key}'."
            )));
        }
    }
    let tool = if request.contains_key("solver") {
        let name = request.get("solver").and_then(Value::as_str).unwrap_or("");
        if !view.sees_solver(name) || !crate::solvers::is_known(name) {
            return Err(DoorError::NotFound("Solver not recognized".to_string()));
        }
        Tool::Solve
    } else {
        let domain = request.get("domain").and_then(Value::as_str);
        let card = request.get("card").and_then(Value::as_str);
        match (domain, card) {
            (Some(domain), Some(card)) => {
                if view.find(domain, card).is_none() {
                    return Err(DoorError::NotFound("Card not recognized".to_string()));
                }
            }
            _ => {
                return Err(violation(
                    "Physical Violation: 'request' must be a run_trajectory request (domain, card, initial, dt_s, segments, record_every) or a solve request (solver, parameters).",
                ));
            }
        }
        if !request.contains_key("segments") {
            return Err(violation(
                "Physical Violation: find_flip varies a run or a solve; 'request' has domain and card and no 'segments', so it is not a run_trajectory request.",
            ));
        }
        Tool::Run
    };
    let vary = match object.get("vary") {
        None => return Err(violation("Missing required parameter: 'vary'")),
        Some(Value::String(text)) => text.clone(),
        Some(_) => return Err(violation("Physical Violation: 'vary' must be a string.")),
    };
    let place = read_place(&vary)?;
    let request = Value::Object(request.clone());
    // The place holds a number in the request as sent: nothing is filled in.
    let mut probe = request.clone();
    let mut found = Vec::new();
    places(&mut probe, &place, &mut found);
    if found.is_empty() || !found.iter().all(|slot| slot.is_number()) {
        return Err(violation(format!(
            "Physical Violation: 'vary' names '{vary}', and the request holds no number there. The request is sent whole, with a number at the place to vary; nothing is filled in."
        )));
    }
    let low = number_of(object, "low")?;
    let high = number_of(object, "high")?;
    if low == high {
        return Err(violation(format!(
            "Physical Violation: 'low' and 'high' are both {}; a span needs two different ends.",
            crate::message_number(low)
        )));
    }
    // The ends come in either order: the smaller is the low end.
    let (low, high) = if low < high { (low, high) } else { (high, low) };
    let level = match object.get("level") {
        None => None,
        Some(Value::Object(level)) => {
            for key in level.keys() {
                if key != "quantity" && key != "value" {
                    return Err(violation(format!("Physical Violation: '{key}' is not a key of level; it takes quantity and value.")));
                }
            }
            let quantity = match level.get("quantity") {
                Some(Value::String(text)) if !text.is_empty() && text.len() <= 100 => text.clone(),
                _ => return Err(violation("Physical Violation: 'level.quantity' must be the name of a published quantity.")),
            };
            // A flag (a true-or-false value the model publishes) is named with
            // value true or false, or with no value: the search finds where it turns.
            let value = match level.get("value") {
                None | Some(Value::Bool(_)) => None,
                Some(_) => Some(number_of(level, "value")?),
            };
            Some(Level { quantity, value })
        }
        Some(_) => return Err(violation("Physical Violation: 'level' must be a JSON object with quantity and value.")),
    };
    // A count is varied in whole numbers: a run's steps and record_every,
    // and a solver's input that is a count.
    let last_key = place.iter().rev().find_map(|seg| match seg {
        Seg::Key(name) => Some(name.as_str()),
        _ => None,
    });
    let whole = match tool {
        Tool::Run => matches!(last_key, Some("steps" | "record_every")),
        Tool::Solve => request
            .get("solver")
            .and_then(Value::as_str)
            .and_then(crate::solvers::find)
            .is_some_and(|solver| {
                solver.inputs.iter().any(|input| Some(input.name) == last_key && matches!(input.kind, crate::solvers::Kind::Count { .. }))
            }),
    };
    if whole && (low.fract() != 0.0 || high.fract() != 0.0 || low < 0.0 || high > 9.0e15) {
        return Err(violation(format!(
            "Physical Violation: '{vary}' is a count, so 'low' and 'high' must be whole numbers, not {} and {}.",
            crate::message_number(low),
            crate::message_number(high)
        )));
    }
    let (most, steps_each, cap) = match tool {
        Tool::Solve => (FLIP_MAX_SOLVES, 0, "FLIP_MAX_SOLVES"),
        Tool::Run => {
            // The steps of the longest run the search can make: with the
            // steps themselves varied, the run at 'high'.
            let longest = if whole { with_value(&request, &place, high, true) } else { request.clone() };
            let steps = longest
                .get("segments")
                .and_then(Value::as_array)
                .map(|segments| {
                    segments
                        .iter()
                        .filter_map(|segment| segment.get("steps").and_then(Value::as_u64))
                        .fold(0_u64, u64::saturating_add)
                })
                .unwrap_or(0);
            let by_steps = FLIP_MAX_STEPS / steps.max(1);
            if by_steps < 3 {
                return Err(violation(format!(
                    "Physical Violation: the run asks for {steps} steps, and a search needs at least three runs: more than the {FLIP_MAX_STEPS} steps all the runs of one search may take together (FLIP_MAX_STEPS). A shorter run can be searched."
                )));
            }
            if by_steps < FLIP_MAX_RUNS { (by_steps, steps, "FLIP_MAX_STEPS") } else { (FLIP_MAX_RUNS, steps, "FLIP_MAX_RUNS") }
        }
    };
    let steps_segment = match (tool, place.as_slice()) {
        (Tool::Run, [Seg::Key(segments), Seg::Index(index), Seg::Key(steps)]) if segments == "segments" && steps == "steps" => Some(*index),
        _ => None,
    };
    Ok(Plan { tool, request, place, vary, low, high, level, most, steps_each, cap, whole, steps_segment })
}

/// How many steps a search may take in all, for the open connection's count
/// of long work. Zero when the request cannot be read.
pub(crate) fn steps_at_most(arguments: &Map<String, Value>) -> u64 {
    let steps = arguments
        .get("request")
        .and_then(|request| request.get("segments"))
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter_map(|segment| segment.get("steps").and_then(Value::as_u64))
                .fold(0_u64, u64::saturating_add)
        })
        .unwrap_or(0);
    steps.saturating_mul(FLIP_MAX_RUNS).min(FLIP_MAX_STEPS)
}

/// One side of the search: a value of the varied number and what the engine
/// answered for it.
#[derive(Clone)]
struct Side {
    value: f64,
    /// What is compared between two sides.
    class: String,
    row: Map<String, Value>,
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

const NO_PROOF: &str = "This request stops before computing, so no proof is issued for it.";

/// A value of the varied number as the answer writes it: a count as a whole
/// number (4087, not 4087.0), as the request itself holds it.
fn written(plan: &Plan, value: f64) -> Value {
    if plan.whole { json!(value as u64) } else { exact(value) }
}

fn evaluate(view: &View, plan: &Plan, value: f64) -> Result<Side, String> {
    let request = with_value(&plan.request, &plan.place, value, plan.whole);
    let mut row = Map::new();
    row.insert("value".to_string(), written(plan, value));
    row.insert("value_hex".to_string(), json!(hex(value)));
    let stopped_first = |sentence: String, mut row: Map<String, Value>, request: Value, seal_key: &str| {
        let name = name_in(&sentence).map(str::to_string);
        let class = match (&plan.level, &name) {
            (Some(_), _) => "no value: stopped before computing".to_string(),
            (None, Some(name)) => format!("stopped before computing ({name})"),
            (None, None) => "stopped before computing".to_string(),
        };
        row.insert("outcome".to_string(), json!("stopped before computing"));
        row.insert("reject".to_string(), json!(name));
        row.insert("reject_detail".to_string(), json!(sentence));
        row.insert(seal_key.to_string(), Value::Null);
        row.insert("no_proof".to_string(), json!(NO_PROOF));
        row.insert("request".to_string(), request);
        Side { value, class, row }
    };
    let level_class = |published: Option<&Value>, row: &mut Map<String, Value>, outcome: &str| -> Result<Option<String>, String> {
        let Some(level) = &plan.level else { return Ok(None) };
        let Some(published) = published.and_then(Value::as_object) else {
            return Ok(Some(format!("no value: {outcome}")));
        };
        let Some(level_value) = level.value else {
            // A flag: a true-or-false value the model publishes.
            let flag = published.get(&level.quantity).and_then(Value::as_bool).ok_or_else(|| {
                let names: Vec<&str> = published.iter().filter(|(_, value)| value.is_boolean()).map(|(key, _)| key.as_str()).collect();
                format!(
                    "Physical Violation: 'level.quantity' is '{}' with no number to cross, and this model publishes no flag of that name. Its flags: {}. A quantity that is a number takes 'value', the number to cross.",
                    level.quantity,
                    if names.is_empty() { "none".to_string() } else { names.join(", ") }
                )
            })?;
            row.insert("quantity".to_string(), json!({ "name": level.quantity, "value": flag }));
            return Ok(Some(if flag { "flag true".to_string() } else { "flag false".to_string() }));
        };
        let read = published
            .get(&format!("{}_hex", level.quantity))
            .and_then(Value::as_str)
            .and_then(exact_from_hex)
            .ok_or_else(|| {
                let names: Vec<&str> = published.keys().filter_map(|key| key.strip_suffix("_hex")).collect();
                format!(
                    "Physical Violation: 'level.quantity' is '{}', and this model publishes no single number of that name. It publishes: {}.",
                    level.quantity,
                    names.join(", ")
                )
            })?;
        row.insert(
            "quantity".to_string(),
            json!({ "name": level.quantity, "value": exact(read), "value_hex": hex(read) }),
        );
        Ok(Some(if read >= level_value { "at or above the level".to_string() } else { "below the level".to_string() }))
    };
    match plan.tool {
        Tool::Run => {
            let prepared = match crate::trajectory::prepare_in(view, request.clone()) {
                Ok(prepared) => prepared,
                Err(DoorError::Unprocessable(sentence)) => return Ok(stopped_first(sentence, row, request, "run_seal")),
                Err(other) => return Err(other.message().to_string()),
            };
            let answer = crate::trajectory::execute(prepared)?;
            let class = match level_class(answer.final_state.as_ref(), &mut row, &answer.outcome)? {
                Some(class) => class,
                None => match &answer.reject {
                    Some(name) => format!("{} ({name})", answer.outcome),
                    None => answer.outcome.clone(),
                },
            };
            row.insert("outcome".to_string(), json!(answer.outcome));
            row.insert("reject".to_string(), json!(answer.reject));
            row.insert("reject_detail".to_string(), json!(answer.reject_detail));
            row.insert("steps_landed".to_string(), json!(answer.steps_landed));
            row.insert("run_seal".to_string(), json!(answer.run_seal));
            row.insert("request".to_string(), request);
            Ok(Side { value, class, row })
        }
        Tool::Solve => {
            let prepared = match crate::solution::prepare_in(view, &request) {
                Ok(prepared) => prepared,
                Err(DoorError::Unprocessable(sentence)) => return Ok(stopped_first(sentence, row, request, "solution_seal")),
                Err(other) => return Err(other.message().to_string()),
            };
            let answer = match prepared.solve_and_seal(request.clone()) {
                Ok(answer) => answer.document,
                // A solver's stop that comes back as a sentence and no seal.
                Err(sentence) => return Ok(stopped_first(sentence, row, request, "solution_seal")),
            };
            let class = match level_class(answer.result.as_ref(), &mut row, &answer.outcome)? {
                Some(class) => class,
                None => match &answer.reject {
                    Some(name) => format!("{} ({name})", answer.outcome),
                    None => answer.outcome.clone(),
                },
            };
            row.insert("outcome".to_string(), json!(answer.outcome));
            row.insert("reject".to_string(), json!(answer.reject));
            row.insert("reject_detail".to_string(), json!(answer.reject_detail));
            row.insert("solution_seal".to_string(), json!(answer.solution_seal));
            row.insert("request".to_string(), request);
            Ok(Side { value, class, row })
        }
    }
}

const ABOUT: &str = "Each side is an ordinary request: send it to the tool named for its whole answer, or with its seal to verify_tape or verify_solution, and the proof is the same. The pair is the first change the halving met; a span can hold more than one. The width is high minus low in 64-bit arithmetic; of a count, neighbours are one apart.";

fn search(view: &View, plan: &Plan) -> Result<Value, String> {
    let began = std::time::Instant::now();
    let mut low = evaluate(view, plan, plan.low)?;
    let mut high = evaluate(view, plan, plan.high)?;
    let mut runs = 2_u64;
    let tool = if plan.tool == Tool::Run { "run_trajectory" } else { "solve" };
    let mut answer = Map::new();
    answer.insert("tool".to_string(), json!(tool));
    answer.insert("vary".to_string(), json!(plan.vary));
    answer.insert(
        "compared".to_string(),
        match &plan.level {
            Some(Level { quantity, value: Some(value) }) => json!(format!("whether {quantity} is at or above {}", crate::message_number(*value))),
            Some(Level { quantity, value: None }) => json!(format!("whether the flag {quantity} is true")),
            None => json!("the outcome and the stop name"),
        },
    );
    let mut stopped_by = Value::Null;
    let mut neighbours = false;
    let out_of_time = || began.elapsed().as_secs() >= FLIP_MAX_SECONDS;
    let middle_of = |low: f64, high: f64| -> Option<f64> {
        let middle = if plan.whole { low + ((high - low) / 2.0).floor() } else { low + (high - low) / 2.0 };
        (middle > low && middle < high).then_some(middle)
    };
    // Where the steps of one segment are varied, a run that stopped inside
    // that segment says how many of its steps landed: that many, and one
    // more, are where the outcome changes, so they are tried before the
    // span is halved again. A hint is only a choice of the next value inside
    // the span; the pair is still a pair the engine answered either side of.
    let hint_of = |side: &Side| -> Option<f64> {
        let index = plan.steps_segment?;
        let landed = side.row.get("steps_landed")?.as_u64()?;
        let before = plan
            .request
            .get("segments")?
            .as_array()?
            .iter()
            .take(index)
            .filter_map(|segment| segment.get("steps").and_then(Value::as_u64))
            .fold(0_u64, u64::saturating_add);
        let asked = side.value as u64;
        (landed >= before && landed - before < asked).then(|| (landed - before) as f64)
    };
    let mut hints: Vec<f64> = Vec::new();
    for side in [&low, &high] {
        if let Some(hint) = hint_of(side) {
            hints.extend([hint, hint + 1.0]);
        }
    }
    // Both ends the same: a band between them may still differ, so a few
    // values evenly between are run before the search says there is none.
    let mut scanned = 0_u64;
    // Whether the pair is the edge of a band where the model computes, met
    // after a change from one stop name to another was passed over.
    let mut passed_a_stop_change = false;
    let stops_first = |side: &Side| side.class.contains("stopped before computing");
    if low.class == high.class {
        let points = if plan.tool == Tool::Run { FLIP_SCAN_RUNS } else { FLIP_SCAN_SOLVES };
        let (from, to) = (low.value, high.value);
        let mut last_tried = f64::NAN;
        // A change from one stop name to another, met while looking for a
        // value that computes.
        let mut name_change: Option<(Side, Side)> = None;
        let mut found_in_scan = false;
        // With a level, a value that stops before computing has no value to
        // compare, and the search is for the level alone.
        let stops_first_all = plan.level.is_some();
        for point in 1..=points {
            let mut value = from + (to - from) * (point as f64) / ((points + 1) as f64);
            if plan.whole {
                value = value.round();
            }
            if !(value > low.value && value < to) || value == last_tried {
                continue;
            }
            if runs >= plan.most {
                stopped_by = json!(plan.cap);
                break;
            }
            if out_of_time() {
                stopped_by = json!("FLIP_MAX_SECONDS");
                break;
            }
            last_tried = value;
            let side = evaluate(view, plan, value)?;
            runs += 1;
            scanned += 1;
            if let Some(hint) = hint_of(&side) {
                hints.extend([hint, hint + 1.0]);
            }
            if side.class == low.class {
                low = side;
            } else if stops_first(&low) && stops_first(&side) && !stops_first_all {
                // Both ends stop before computing and so does this value, by
                // another name: a band where the model computes may lie
                // further in, and that band is what a search is for. The
                // change of name is kept in case no value computes.
                if name_change.is_none() {
                    name_change = Some((low.clone(), side.clone()));
                }
                low = side;
            } else {
                // The first value that differs: the change lies between the
                // last value run before it and this one.
                passed_a_stop_change = name_change.is_some();
                high = side;
                found_in_scan = true;
                break;
            }
        }
        if !found_in_scan {
            if let Some((below, above)) = name_change.take() {
                low = below;
                high = above;
            }
        }
    }
    let flips = low.class != high.class;
    if flips {
        loop {
            let Some(halfway) = middle_of(low.value, high.value) else {
                neighbours = true;
                break;
            };
            hints.retain(|hint| *hint > low.value && *hint < high.value);
            let middle = if hints.is_empty() { halfway } else { hints.remove(0) };
            if runs >= plan.most {
                stopped_by = json!(plan.cap);
                break;
            }
            if out_of_time() {
                stopped_by = json!("FLIP_MAX_SECONDS");
                break;
            }
            let side = evaluate(view, plan, middle)?;
            runs += 1;
            if let Some(hint) = hint_of(&side) {
                hints.extend([hint, hint + 1.0]);
            }
            if side.class == low.class {
                low = side;
            } else {
                high = side;
            }
        }
    }
    let width = high.value - low.value;
    answer.insert("flips".to_string(), json!(flips));
    if scanned > 0 {
        answer.insert(
            "scanned".to_string(),
            json!(if flips {
                if passed_a_stop_change {
                    format!("Both ends of the span stop before computing, so values evenly between them were run until one computed ({scanned} of them); the pair is the low edge of that band, where the model starts to answer. The span holds other changes: below low.value one stop name gives way to another, and the band has an edge above it: search from high.value to the span's high end for it.")
                } else {
                    format!("Both ends of the span gave the same, so values evenly between them were run until one differed ({scanned} of them); the pair is the change on its low side. The band may have another edge above it: search from high.value to the span's high end for it.")
                }
            } else {
                format!("Both ends of the span gave the same, so {scanned} values evenly between them were run, and each gave the same again. A band narrower than the spacing between them can still lie in the span.")
            }),
        );
    }
    if !flips {
        answer.insert(
            "no_flip".to_string(),
            json!(format!("No change was found: the ends of the span and every value run between them give {}. A wider or a narrower span, or another number to vary, may hold one.", low.class)),
        );
    }
    answer.insert("low_is".to_string(), json!(low.class));
    answer.insert("high_is".to_string(), json!(high.class));
    answer.insert("low".to_string(), Value::Object(low.row));
    answer.insert("high".to_string(), Value::Object(high.row));
    answer.insert("width".to_string(), written(plan, width));
    answer.insert("width_hex".to_string(), json!(hex(width)));
    answer.insert("neighbours".to_string(), json!(neighbours));
    match plan.tool {
        Tool::Run => {
            answer.insert("runs".to_string(), json!(runs));
            answer.insert("steps_asked_at_most".to_string(), json!(runs.saturating_mul(plan.steps_each)));
        }
        Tool::Solve => {
            answer.insert("solves".to_string(), json!(runs));
        }
    }
    answer.insert("stopped_by".to_string(), stopped_by);
    answer.insert(
        "engine_version".to_string(),
        json!(if plan.tool == Tool::Run { crate::ENGINE_VERSION.to_string() } else { crate::solvers::engine_version() }),
    );
    answer.insert("about".to_string(), json!(ABOUT));
    Ok(Value::Object(answer))
}

/// The tool. The request is read and held to its caps first; then, under one
/// physics permit, the search is made.
pub async fn find(state: &crate::AppState, body: Value) -> Result<String, DoorError> {
    let plan = read(state.view(), &body)?;
    let view = state.view().clone();
    let answer = crate::on_pool(state, move || search(&view, &plan)).await?;
    serde_json::to_string(&answer)
        .map_err(|error| DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(current: f64) -> Value {
        json!({
            "domain": "energy", "card": "battery_sag", "initial": { "soc": 1.0 }, "dt_s": 0.1,
            "segments": [{ "steps": 6000, "load": { "current_a": current } }], "record_every": 6000
        })
    }

    fn found(body: Value) -> Value {
        let view = View::full();
        let plan = read(&view, &body).unwrap();
        search(&view, &plan).unwrap()
    }

    #[test]
    fn the_pack_completes_on_one_side_and_stops_on_the_other_at_neighbouring_numbers() {
        let answer = found(json!({ "request": pack(60.0), "vary": "segments[0].load.current_a", "low": 50.0, "high": 80.0 }));
        assert_eq!(answer["flips"], true);
        assert_eq!(answer["low_is"], "completed");
        assert_eq!(answer["high_is"], "refused (UndervoltageCutoff)");
        assert_eq!(answer["neighbours"], true);
        assert_eq!(answer["stopped_by"], Value::Null);
        let low = answer["low"]["value"].as_f64().unwrap();
        let high = answer["high"]["value"].as_f64().unwrap();
        assert_eq!(low.next_up(), high);
        assert_eq!(answer["width"].as_f64().unwrap(), high - low);
        assert_eq!(answer["width_hex"], hex(high - low));
        assert_eq!(answer["low"]["value_hex"], hex(low));
        // The page's two runs are either side of it.
        assert!(71.44 < low && high < 71.45, "{low} {high}");
        // Each side's proof is the proof the same request has when sent alone.
        for side in ["low", "high"] {
            let alone = crate::trajectory::execute(crate::trajectory::prepare(answer[side]["request"].clone()).unwrap()).unwrap();
            assert_eq!(answer[side]["run_seal"], alone.run_seal, "{side}");
            assert_eq!(answer[side]["steps_landed"], alone.steps_landed);
        }
        assert_eq!(answer["high"]["steps_landed"], 5999);
        // The same request gives the same pair.
        let again = found(json!({ "request": pack(60.0), "vary": "segments[0].load.current_a", "low": 50.0, "high": 80.0 }));
        assert_eq!(answer, again);
    }

    #[test]
    fn a_span_with_no_change_is_scanned_and_says_so() {
        let answer = found(json!({ "request": pack(60.0), "vary": "segments[*].load.current_a", "low": 10.0, "high": 20.0 }));
        assert_eq!(answer["flips"], false);
        assert_eq!(answer["runs"], 2 + FLIP_SCAN_RUNS);
        assert!(answer["no_flip"].as_str().unwrap().contains("completed"));
        assert!(answer["scanned"].as_str().unwrap().contains("7 values evenly between"));
        assert_eq!(answer["low"]["outcome"], "completed");
    }

    #[test]
    fn a_published_quantity_crossing_a_level_is_found() {
        let answer = found(json!({
            "request": pack(60.0), "vary": "segments[0].load.current_a", "low": 10.0, "high": 60.0,
            "level": { "quantity": "terminal_v", "value": 40.0 }
        }));
        assert_eq!(answer["flips"], true);
        assert_eq!((answer["low_is"].as_str(), answer["high_is"].as_str()), (Some("at or above the level"), Some("below the level")));
        assert!(answer["low"]["quantity"]["value"].as_f64().unwrap() >= 40.0);
        assert!(answer["high"]["quantity"]["value"].as_f64().unwrap() < 40.0);
        assert_eq!(answer["neighbours"], true);
        let view = View::full();
        let plan = read(&view, &json!({
            "request": pack(60.0), "vary": "segments[0].load.current_a", "low": 10.0, "high": 60.0,
            "level": { "quantity": "no_such", "value": 40.0 }
        })).unwrap();
        let error = search(&view, &plan).unwrap_err();
        assert!(error.contains("'no_such'") && error.contains("terminal_v"), "{error}");
    }

    #[test]
    fn a_count_is_varied_in_whole_numbers() {
        // The generator ends at mission day 4,087: by counting steps of one day, the last run that completes and the first that ends.
        let run = json!({ "domain": "energy", "card": "generator_mmrtg_f1", "initial": { "mission_time_s": 0.0 }, "dt_s": 86400.0, "segments": [{ "steps": 100, "load": {} }], "record_every": 5000 });
        let answer = found(json!({ "request": run, "vary": "segments[0].steps", "low": 100, "high": 5000 }));
        assert_eq!((answer["low_is"].as_str(), answer["high_is"].as_str()), (Some("completed"), Some("ended (MissionTimeOutsideCard)")));
        assert_eq!((answer["low"]["value"].as_f64(), answer["high"]["value"].as_f64()), (Some(4087.0), Some(4088.0)));
        assert_eq!(answer["low"]["request"]["segments"][0]["steps"], json!(4087));
        assert!(answer["low"]["request"]["segments"][0]["steps"].is_u64());
        assert_eq!((answer["neighbours"].as_bool(), answer["width"].as_f64()), (Some(true), Some(1.0)));
        // A count is written as a whole number, as the request holds it.
        assert!(answer["low"]["value"].is_u64() && answer["high"]["value"].is_u64() && answer["width"].is_u64(), "{answer}");
        // The run at the high end says where it ended, so the search goes
        // there: the two ends, 4,087 steps and 4,088.
        assert_eq!(answer["runs"], 4);
        // A long run leaves few runs under FLIP_MAX_STEPS; the steps that
        // landed still find the last step in one search. LAGEOS-1's card
        // ends its forecast at seven days: 60,480 steps of 10 s.
        let described = crate::descriptor::describe_value_in(&View::full(), "aerospace", "satellite_lageos1").unwrap();
        let orbit = described["example_run_trajectory"].clone();
        let dt = orbit["dt_s"].as_f64().unwrap();
        let last = (7.0 * 86400.0 / dt).round() as u64;
        let answer = found(json!({ "request": orbit, "vary": "segments[0].steps", "low": 1356, "high": 100000 }));
        assert_eq!((answer["low"]["value"].as_u64(), answer["high"]["value"].as_u64()), (Some(last), Some(last + 1)), "{answer}");
        assert_eq!((answer["neighbours"].as_bool(), answer["stopped_by"].is_null(), answer["runs"].as_u64()), (Some(true), true, Some(4)));
        // The ends in either order give the same pair.
        let run = json!({ "domain": "energy", "card": "generator_mmrtg_f1", "initial": { "mission_time_s": 0.0 }, "dt_s": 86400.0, "segments": [{ "steps": 100, "load": {} }], "record_every": 5000 });
        let forward = found(json!({ "request": run, "vary": "segments[0].steps", "low": 100, "high": 5000 }));
        let backward = found(json!({ "request": run, "vary": "segments[0].steps", "low": 5000, "high": 100 }));
        assert_eq!(forward, backward);
        let view = View::full();
        let error = read(&view, &json!({ "request": run, "vary": "segments[0].steps", "low": 100.5, "high": 5000 })).err().unwrap();
        assert!(error.message().contains("is a count"), "{}", error.message());
        // A solver's count: the atom solver answers to atomic number 18.
        let answer = found(json!({ "request": { "solver": "atom_dft", "parameters": { "atomic_number": 1 } }, "vary": "parameters.atomic_number", "low": 1, "high": 40 }));
        assert_eq!((answer["low"]["value"].as_f64(), answer["high"]["value"].as_f64()), (Some(18.0), Some(19.0)));
        assert_eq!(answer["neighbours"], true);
    }

    #[test]
    fn a_band_between_two_ends_that_stop_alike_is_found_by_the_scan() {
        // The grid card answers losses of 693 to 1402 MW; 100 and 3000 MW both stop before computing, by the same name.
        let run = json!({
            "domain": "energy", "card": "grid_gb",
            "initial": { "frequency_hz": 49.942, "inertia_mw_s": 241000.0, "demand_mw": 27132.0 },
            "dt_s": 0.02, "segments": [{ "steps": 100, "load": { "loss_mw": 996.0 } }], "record_every": 100
        });
        let answer = found(json!({ "request": run, "vary": "segments[0].load.loss_mw", "low": 100.0, "high": 3000.0 }));
        assert_eq!(answer["flips"], true, "{answer}");
        assert!(answer["scanned"].as_str().unwrap().contains("until one differed"));
        assert_eq!((answer["low_is"].as_str(), answer["high_is"].as_str()), (Some("stopped before computing (LossOutsideCard)"), Some("completed")));
        assert_eq!(answer["high"]["value"], 693.0);
        assert_eq!(answer["neighbours"], true);
    }

    #[test]
    fn a_band_behind_a_change_of_stop_name_is_found() {
        // The accumulator's worked charge answers from 55 to 67 bar. Below
        // 37 bar and above 86 bar it stops as PressureOutsideCard, and
        // between those and the band as PressureOutsideChargeBands. Asked 30
        // to 90 bar, the search passes over the change of name and gives the
        // edge where the model starts to answer.
        let described = crate::descriptor::describe_value_in(&View::full(), "energy", "accumulator_sbo450").unwrap();
        let run = described["example_run_trajectory"].clone();
        let answer = found(json!({ "request": run, "vary": "initial.gas_pressure_pa", "low": 3.0e6, "high": 9.0e6 }));
        assert_eq!(answer["flips"], true, "{answer}");
        assert_eq!(answer["low_is"], "stopped before computing (PressureOutsideChargeBands)");
        assert!(!answer["high_is"].as_str().unwrap().contains("stopped before computing"), "{answer}");
        let (low, high) = (answer["low"]["value"].as_f64().unwrap(), answer["high"]["value"].as_f64().unwrap());
        assert!(5.0e6 < low && high < 6.0e6 && low.next_up() == high, "{low} {high}");
        assert!(answer["high"]["run_seal"].as_str().is_some_and(|seal| seal.len() == 64));
        assert!(answer["scanned"].as_str().unwrap().contains("until one computed") && answer["scanned"].as_str().unwrap().contains("one stop name gives way to another"));
        // Where no value between computes, the change of name is the pair, as before.
        let answer = found(json!({ "request": run, "vary": "initial.gas_pressure_pa", "low": 3.0e6, "high": 3.9e6 }));
        assert_eq!((answer["low_is"].as_str(), answer["high_is"].as_str()), (Some("stopped before computing (PressureOutsideCard)"), Some("stopped before computing (PressureOutsideChargeBands)")), "{answer}");
        assert_eq!(answer["high"]["value"], 3.7e6);
    }

    #[test]
    fn a_flag_turning_is_found() {
        // The pack's sag warning: false at a small current, true at a large one.
        let answer = found(json!({ "request": pack(10.0), "vary": "segments[0].load.current_a", "low": 1.0, "high": 60.0, "level": { "quantity": "sag_warning" } }));
        assert_eq!(answer["flips"], true, "{answer}");
        assert_eq!((answer["low_is"].as_str(), answer["high_is"].as_str()), (Some("flag false"), Some("flag true")));
        assert_eq!((answer["low"]["quantity"]["value"].as_bool(), answer["high"]["quantity"]["value"].as_bool()), (Some(false), Some(true)));
        assert_eq!(answer["neighbours"], true);
        assert_eq!(answer["compared"], "whether the flag sag_warning is true");
        let view = View::full();
        let plan = read(&view, &json!({ "request": pack(10.0), "vary": "segments[0].load.current_a", "low": 1.0, "high": 60.0, "level": { "quantity": "terminal_v", "value": true } })).unwrap();
        let error = search(&view, &plan).unwrap_err();
        assert!(error.contains("no flag of that name") && error.contains("sag_warning"), "{error}");
    }

    #[test]
    fn the_caps_hold_and_are_named() {
        let view = View::full();
        // A long run leaves room for few runs: the cap by steps stops the search, by name.
        let mut long = pack(71.0);
        long["segments"][0]["steps"] = json!(6000);
        long["segments"].as_array_mut().unwrap().push(json!({ "steps": 194_000, "load": { "current_a": 0.001 } }));
        long["record_every"] = json!(200_000);
        let plan = read(&view, &json!({ "request": long, "vary": "segments[0].load.current_a", "low": 50.0, "high": 80.0 })).unwrap();
        assert_eq!((plan.most, plan.cap), (5, "FLIP_MAX_STEPS"));
        let answer = search(&view, &plan).unwrap();
        assert_eq!(answer["runs"], 5);
        assert_eq!(answer["stopped_by"], "FLIP_MAX_STEPS");
        assert_eq!(answer["neighbours"], false);
        assert_eq!(answer["steps_asked_at_most"], 1_000_000);
        assert_eq!(answer["width"].as_f64().unwrap(), 30.0 / 8.0);
        // A run too long for three runs is named before anything runs.
        let mut longer = pack(71.0);
        longer["segments"][0]["steps"] = json!(400_000);
        let error = read(&view, &json!({ "request": longer, "vary": "segments[0].load.current_a", "low": 50.0, "high": 80.0 })).err().unwrap();
        assert!(error.message().contains("FLIP_MAX_STEPS"), "{}", error.message());
        // A solve is held to its own cap. With the cap the first build had,
        // eight, a search over a solver took eight or nine calls; with 64,
        // and FLIP_MAX_SECONDS over a slow solver, the bearing's edge is one call.
        let body = json!({
            "request": { "solver": "journal_bearing_sheffield", "parameters": { "load_n": 10000, "speed_rpm": 400 } },
            "vary": "parameters.load_n", "low": 10000, "high": 30000
        });
        let plan = read(&view, &body).unwrap();
        assert_eq!((plan.most, plan.cap), (FLIP_MAX_SOLVES, "FLIP_MAX_SOLVES"));
        let whole = search(&view, &plan).unwrap();
        assert_eq!((whole["neighbours"].as_bool(), whole["stopped_by"].is_null()), (Some(true), true), "{whole}");
        assert_eq!((whole["low"]["value"].as_f64(), whole["high"]["value"].as_f64()), (Some(20000.0), Some(20000.000000000004)));
        assert!(whole["solves"].as_u64().unwrap() <= FLIP_MAX_SOLVES);
        let mut plan = read(&view, &body).unwrap();
        plan.most = 8;
        let answer = search(&view, &plan).unwrap();
        assert_eq!(answer["solves"], 8);
        assert_eq!(answer["stopped_by"], "FLIP_MAX_SOLVES");
        assert_eq!(answer["low_is"], "converged");
        assert_eq!(answer["high_is"], "stopped before computing (LoadOutOfRange)");
        assert_eq!(answer["high"]["solution_seal"], Value::Null);
        assert!(answer["high"]["no_proof"].is_string());
        assert!(answer["low"]["solution_seal"].as_str().unwrap().len() == 64);
        let low = answer["low"]["value"].as_f64().unwrap();
        let high = answer["high"]["value"].as_f64().unwrap();
        assert!(low <= 20000.0 && 20000.0 < high && high - low == 20000.0 / 64.0, "{low} {high}");
    }

    #[test]
    fn a_request_out_of_shape_is_named_and_nothing_is_filled_in() {
        let view = View::full();
        let message = |body: Value| read(&view, &body).err().map(|error| error.message().to_string()).unwrap();
        assert!(message(json!({ "vary": "dt_s", "low": 1, "high": 2 })).contains("'request'"));
        assert!(message(json!({ "request": pack(1.0), "low": 1, "high": 2 })).contains("'vary'"));
        assert!(message(json!({ "request": pack(1.0), "vary": "segments[0].load.voltage_v", "low": 1, "high": 2 })).contains("nothing is filled in"));
        assert!(message(json!({ "request": pack(1.0), "vary": "segments[0].load", "low": 1, "high": 2 })).contains("holds no number"));
        assert!(message(json!({ "request": pack(1.0), "vary": "a b", "low": 1, "high": 2 })).contains("'vary' must be the place"));
        assert!(message(json!({ "request": pack(1.0), "vary": "dt_s", "low": 2, "high": 2 })).contains("a span needs two different ends"));
        assert!(message(json!({ "request": pack(1.0), "vary": "dt_s", "low": 1, "high": 2, "steps": 3 })).contains("'steps'"));
        let mut chained = pack(1.0);
        chained["from"] = json!({});
        assert!(message(json!({ "request": chained, "vary": "dt_s", "low": 1, "high": 2 })).contains("no 'from'"));
        assert_eq!(message(json!({ "request": { "solver": "none", "parameters": {} }, "vary": "parameters.x", "low": 1, "high": 2 })), "Solver not recognized");
        assert_eq!(message(json!({ "request": { "domain": "energy", "card": "none", "segments": [] }, "vary": "dt_s", "low": 1, "high": 2 })), "Card not recognized");
    }
}
