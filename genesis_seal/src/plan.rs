//! The plan of a dataset, and the expansion of one row.
//!
//! A dataset request carries `domain`, `card`, `plan`, `store` and `return`.
//! A plan is a `list` of runs, a `grid` over a base run, or a `sample` over a
//! base run. Every row is an ordinary `run_trajectory` request.
//!
//! A path names one number of a run: `initial.<field>`, `dt_s`, or
//! `segments[k].load.<field>`, with `[c]` for one component of a 3-vector.
//! Every field of a run is set exactly once, by the base or by one path.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Number, Value};

use crate::card::{Bounds, CardFields, Field};
use crate::json_text::{TextRefusal, is_lower_hex, is_plain_field_name};
use crate::run::frames_recorded;
use crate::sampler;
use crate::sealed_form::{digest_hex, sealed_form};

const REQUEST_KEYS: &[&str] = &["domain", "card", "plan", "store", "return"];
const RUN_KEYS: &[&str] = &["initial", "dt_s", "segments", "record_every"];
/// A segment's keys: its steps, its load, and an instrument's signature over
/// the load (`crate::attest`), which a row carries as the plan states it.
const SEGMENT_KEYS: &[&str] = &["steps", "load", crate::attest::SIGNED];

/// Whether the frames of every row are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    Frames,
    None,
}

/// How the dataset comes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Return {
    Inline,
    File,
}

/// What a plan asks for, counted from the plan alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub rows: u64,
    /// The steps of every row added together.
    pub steps: u64,
    /// The frames every row would record if each of its steps landed.
    pub frames: u64,
    /// The steps of the longest row.
    pub largest_run_steps: u64,
    /// The frames the row that records most would record.
    pub largest_run_frames: u64,
}

/// The limits of one run and of one job. The gateway states the numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub run_steps: u64,
    pub run_frames: u64,
    pub rows: u64,
    pub steps: u64,
    pub frames: u64,
    pub inline_rows: u64,
    pub inline_frames: u64,
}

/// Why a plan was refused. A refused plan seals nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanRefusal {
    /// The text repeats a key, holds a number that reads as zero, or is not JSON.
    Text(TextRefusal),
    /// A required key is missing.
    Missing(String),
    /// A key is not a field of the request, the plan or a run.
    UnknownKey(String),
    /// The request names `solver`: a plan of solves, which the engine
    /// verifies by its request alone and this crate does not read.
    PlanOfSolves,
    /// A value has the wrong JSON type.
    WrongType { field: String, expected: &'static str },
    /// `kind` is not `list`, `grid` or `sample`.
    UnknownKind(String),
    /// `generator` is not `splitmix64_counter_v1`.
    UnknownGenerator(String),
    /// `seed_hex` is not 16 lowercase hex digits.
    SeedNotExact,
    /// The plan has no rows.
    EmptyPlan,
    /// A run has no segments.
    NoSegments,
    /// A segment is not a JSON object.
    SegmentNotAnObject,
    /// A path is not written as a path.
    PathNotRead(String),
    /// A path names `steps` or `record_every`.
    PathCannotVary(String),
    /// A path names no field of the base run.
    PathNamesNoField(String),
    /// A path names a whole 3-vector.
    PathNeedsComponent(String),
    /// A field is set by the base and a path, or by two paths.
    SetTwice(String),
    /// A field is set by neither the base nor a path.
    NotSet(String),
    /// An axis or a choice lists no values.
    NoValues(String),
    /// A draw carries both `uniform` and `choice`, or neither.
    DrawNotOne(String),
    /// A sample interval has `low` above `high`.
    IntervalReversed(String),
    /// A sample interval is wider than a 64-bit number holds.
    IntervalTooWide(String),
    /// An axis value, a choice or a sample bound lies outside the card's published bounds.
    OutsideCard { path: String, value: String },
    /// The plan's counts do not fit a 64-bit whole number.
    TotalsTooLarge,
    /// The plan has no row of that number.
    NoSuchRow { row: u64, rows: u64 },
    /// One run asks for more steps than a run may take.
    RunTooLong(u64),
    /// One run would record more frames than a run may record.
    RunRecordsTooMuch(u64),
    /// The plan asks for more rows than a job may hold.
    TooManyRows(u64),
    /// The plan asks for more steps in total than a job may take.
    TooManySteps(u64),
    /// The plan would record more frames than a job may store.
    TooManyFrames(u64),
    /// The plan is too large to return inline.
    TooLargeForInline,
}

impl fmt::Display for PlanRefusal {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        if let PlanRefusal::Text(text) = self {
            return text.fmt(out);
        }
        if let PlanRefusal::Missing(key) = self {
            return write!(out, "Missing required parameter: '{key}'");
        }
        out.write_str("Physical Violation: ")?;
        match self {
            PlanRefusal::Text(_) | PlanRefusal::Missing(_) => Ok(()),
            PlanRefusal::UnknownKey(key) => {
                write!(out, "'{key}' is not a field of this request.")
            }
            PlanRefusal::PlanOfSolves => out.write_str(
                "this is a plan of solves: its request names 'solver' in place of 'domain' and 'card'. A plan of solves is verified by its request alone: send the request and its dataset_seal to the engine's verify_dataset. genesis_seal reads a dataset of runs.",
            ),
            PlanRefusal::WrongType { field, expected } => {
                write!(out, "'{field}' must be {expected}.")
            }
            PlanRefusal::UnknownKind(kind) => write!(
                out,
                "'{kind}' is not a kind of plan. The kinds are list, grid and sample."
            ),
            PlanRefusal::UnknownGenerator(name) => write!(
                out,
                "'{name}' is not a generator. The generator is {}.",
                sampler::GENERATOR
            ),
            PlanRefusal::SeedNotExact => {
                write!(out, "'seed_hex' must be 16 lowercase hex digits.")
            }
            PlanRefusal::EmptyPlan => write!(out, "the plan has no rows."),
            PlanRefusal::NoSegments => {
                write!(out, "'segments' must contain at least one segment.")
            }
            PlanRefusal::SegmentNotAnObject => {
                write!(out, "each segment must be a JSON object.")
            }
            PlanRefusal::PathNotRead(path) => write!(
                out,
                "'{path}' is not a path. A path is initial.<field>, dt_s, or segments[k].load.<field>, with [0], [1] or [2] for one component of a 3-vector."
            ),
            PlanRefusal::PathCannotVary(path) => write!(
                out,
                "'{path}' cannot vary. 'steps' and 'record_every' are set by the base run."
            ),
            PlanRefusal::PathNamesNoField(path) => {
                write!(out, "'{path}' names no field of the base run.")
            }
            PlanRefusal::PathNeedsComponent(path) => write!(
                out,
                "'{path}' is a 3-vector. A path sets one component, with [0], [1] or [2]."
            ),
            PlanRefusal::SetTwice(path) => write!(out, "'{path}' is set twice."),
            PlanRefusal::NotSet(path) => write!(out, "'{path}' is not set."),
            PlanRefusal::NoValues(path) => write!(out, "'{path}' lists no values."),
            PlanRefusal::DrawNotOne(path) => write!(
                out,
                "the draw on '{path}' must carry 'uniform' or 'choice', and not both."
            ),
            PlanRefusal::IntervalReversed(path) => {
                write!(out, "the interval on '{path}' has 'low' above 'high'.")
            }
            PlanRefusal::IntervalTooWide(path) => write!(
                out,
                "the interval on '{path}' is wider than a 64-bit number holds."
            ),
            PlanRefusal::OutsideCard { path, value } => write!(
                out,
                "'{path}' {value} is outside the card's published bounds for that field."
            ),
            PlanRefusal::TotalsTooLarge => {
                write!(out, "the plan's counts do not fit a 64-bit whole number.")
            }
            PlanRefusal::NoSuchRow { row, rows } => {
                write!(out, "the plan has {rows} rows and no row {row}.")
            }
            PlanRefusal::RunTooLong(limit) => {
                write!(out, "the run asks for more than {limit} steps.")
            }
            PlanRefusal::RunRecordsTooMuch(limit) => {
                write!(out, "the run would record more than {limit} frames.")
            }
            PlanRefusal::TooManyRows(limit) => {
                write!(out, "the dataset asks for more than {limit} runs.")
            }
            PlanRefusal::TooManySteps(limit) => write!(
                out,
                "the dataset asks for more than {limit} steps in total."
            ),
            PlanRefusal::TooManyFrames(limit) => write!(
                out,
                "the dataset would record more than {limit} frames."
            ),
            PlanRefusal::TooLargeForInline => {
                write!(out, "the dataset is too large to return inline.")
            }
        }
    }
}

impl From<TextRefusal> for PlanRefusal {
    fn from(text: TextRefusal) -> Self {
        PlanRefusal::Text(text)
    }
}

fn wrong(field: &str, expected: &'static str) -> PlanRefusal {
    PlanRefusal::WrongType {
        field: field.to_string(),
        expected,
    }
}

/// Where a path points.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Place {
    Initial(String),
    Dt,
    Load { segment: usize, field: String },
}

/// One number of a run, named as the plan names it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Path {
    text: String,
    place: Place,
    component: Option<usize>,
}

impl Path {
    /// The path without its component.
    fn field_text(&self) -> &str {
        match self.component {
            Some(_) => &self.text[..self.text.len() - 3],
            None => &self.text,
        }
    }

    fn sets_initial(&self, name: &str) -> bool {
        matches!(&self.place, Place::Initial(field) if field == name)
    }

    fn sets_load(&self, index: usize, name: &str) -> bool {
        matches!(&self.place, Place::Load { segment, field } if *segment == index && field == name)
    }
}

fn read_path(text: &str) -> Result<Path, PlanRefusal> {
    let not_read = || PlanRefusal::PathNotRead(text.to_string());
    let cannot_vary = || PlanRefusal::PathCannotVary(text.to_string());
    if text == "dt_s" {
        return Ok(Path {
            text: text.to_string(),
            place: Place::Dt,
            component: None,
        });
    }
    if text == "record_every" || text == "steps" {
        return Err(cannot_vary());
    }
    if let Some(rest) = text.strip_prefix("initial.") {
        let (field, component) = name_and_component(rest).ok_or_else(not_read)?;
        return Ok(Path {
            text: text.to_string(),
            place: Place::Initial(field),
            component,
        });
    }
    if let Some(rest) = text.strip_prefix("segments[") {
        let (index, rest) = rest.split_once(']').ok_or_else(not_read)?;
        let segment = whole_index(index).ok_or_else(not_read)?;
        if rest == ".steps" {
            return Err(cannot_vary());
        }
        let rest = rest.strip_prefix(".load.").ok_or_else(not_read)?;
        let (field, component) = name_and_component(rest).ok_or_else(not_read)?;
        return Ok(Path {
            text: text.to_string(),
            place: Place::Load { segment, field },
            component,
        });
    }
    Err(not_read())
}

/// `name` or `name[c]`, with `c` one of 0, 1 and 2.
fn name_and_component(text: &str) -> Option<(String, Option<usize>)> {
    let (name, component) = match text.split_once('[') {
        None => (text, None),
        Some((name, tail)) => {
            let component = match tail {
                "0]" => 0,
                "1]" => 1,
                "2]" => 2,
                _ => return None,
            };
            (name, Some(component))
        }
    };
    if !is_plain_field_name(name) {
        return None;
    }
    Some((name.to_string(), component))
}

/// A whole number in decimal digits with no leading zero, so one segment has
/// one spelling.
fn whole_index(text: &str) -> Option<usize> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if text.len() > 1 && text.starts_with('0') {
        return None;
    }
    text.parse().ok()
}

/// One run as a plan states it: a whole run of a list, or the base of a grid
/// or a sample.
#[derive(Debug, Clone)]
struct Run {
    fields: Map<String, Value>,
    steps: u64,
    frames: u64,
}

impl Run {
    fn initial(&self) -> &Map<String, Value> {
        self.fields["initial"]
            .as_object()
            .expect("checked when the run was read")
    }

    fn loads(&self) -> Vec<&Map<String, Value>> {
        self.fields["segments"]
            .as_array()
            .expect("checked when the run was read")
            .iter()
            .map(|segment| {
                segment["load"]
                    .as_object()
                    .expect("checked when the run was read")
            })
            .collect()
    }
}

/// A number, or an array of three numbers.
fn is_settable(value: &Value) -> bool {
    match value {
        Value::Number(_) => true,
        Value::Array(items) => items.len() == 3 && items.iter().all(Value::is_number),
        _ => false,
    }
}

fn read_count(object: &Map<String, Value>, key: &str) -> Result<u64, PlanRefusal> {
    match object.get(key) {
        None => Err(PlanRefusal::Missing(key.to_string())),
        Some(value) => match value.as_u64() {
            Some(count) if count >= 1 => Ok(count),
            _ => Err(wrong(key, "an integer >= 1")),
        },
    }
}

fn read_fields(
    object: &Map<String, Value>,
    key: &str,
    prefix: &str,
) -> Result<(), PlanRefusal> {
    let fields = object
        .get(key)
        .ok_or_else(|| PlanRefusal::Missing(key.to_string()))?
        .as_object()
        .ok_or_else(|| wrong(key, "a JSON object"))?;
    for (name, value) in fields {
        if !is_plain_field_name(name) {
            return Err(PlanRefusal::UnknownKey(name.clone()));
        }
        // A start may carry an instrument's signature over the rest of it: an
        // object, read and verified by the rule of a signed reading when the
        // row's run is.
        if key == "initial" && name == crate::attest::SIGNED {
            if !value.is_object() {
                return Err(wrong(&format!("{prefix}{name}"), "a JSON object"));
            }
            continue;
        }
        if !is_settable(value) {
            return Err(wrong(
                &format!("{prefix}{name}"),
                "a number or an array of three numbers",
            ));
        }
    }
    Ok(())
}

fn refuse_unknown(object: &Map<String, Value>, allowed: &[&str]) -> Result<(), PlanRefusal> {
    for key in object.keys() {
        if !allowed.iter().any(|name| *name == key) {
            return Err(PlanRefusal::UnknownKey(key.clone()));
        }
    }
    Ok(())
}

/// Read a run. A base may leave `dt_s` to a path. A whole run may not.
fn read_run(value: &Value, name: &str, is_base: bool) -> Result<Run, PlanRefusal> {
    let object = value
        .as_object()
        .ok_or_else(|| wrong(name, "a JSON object"))?;
    refuse_unknown(object, RUN_KEYS)?;
    read_fields(object, "initial", "initial.")?;
    match object.get("dt_s") {
        Some(Value::Number(_)) => {}
        Some(_) => return Err(wrong("dt_s", "a number")),
        None if is_base => {}
        None => return Err(PlanRefusal::Missing("dt_s".to_string())),
    }
    let segments = object
        .get("segments")
        .ok_or_else(|| PlanRefusal::Missing("segments".to_string()))?
        .as_array()
        .ok_or_else(|| wrong("segments", "a JSON array"))?;
    if segments.is_empty() {
        return Err(PlanRefusal::NoSegments);
    }
    let mut steps = 0_u64;
    for (index, segment) in segments.iter().enumerate() {
        let segment = segment
            .as_object()
            .ok_or(PlanRefusal::SegmentNotAnObject)?;
        refuse_unknown(segment, SEGMENT_KEYS)?;
        steps = steps
            .checked_add(read_count(segment, "steps")?)
            .ok_or(PlanRefusal::TotalsTooLarge)?;
        read_fields(segment, "load", &format!("segments[{index}].load."))?;
    }
    let record_every = read_count(object, "record_every")?;
    let frames = frames_recorded(steps, record_every).ok_or(PlanRefusal::TotalsTooLarge)?;
    Ok(Run {
        fields: object.clone(),
        steps,
        frames,
    })
}

/// What a draw takes its number from.
#[derive(Debug, Clone)]
enum Draw {
    Uniform { low: Number, high: Number },
    Choice(Vec<Number>),
}

#[derive(Debug, Clone)]
enum Plan {
    List(Vec<Run>),
    Grid {
        base: Run,
        axes: Vec<(Path, Vec<Number>)>,
    },
    Sample {
        base: Run,
        seed: u64,
        draws: Vec<(Path, Draw)>,
    },
}

fn nearest(number: &Number) -> f64 {
    number.as_f64().expect("a JSON number has a nearest f64")
}

fn read_numbers(value: &Value, name: &str, path: &Path) -> Result<Vec<Number>, PlanRefusal> {
    let items = value
        .as_array()
        .ok_or_else(|| wrong(name, "a JSON array of numbers"))?;
    if items.is_empty() {
        return Err(PlanRefusal::NoValues(path.text.clone()));
    }
    let mut numbers = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::Number(number) => numbers.push(number.clone()),
            _ => return Err(wrong(name, "a JSON array of numbers")),
        }
    }
    Ok(numbers)
}

fn read_path_of(object: &Map<String, Value>) -> Result<Path, PlanRefusal> {
    let text = object
        .get("path")
        .ok_or_else(|| PlanRefusal::Missing("path".to_string()))?
        .as_str()
        .ok_or_else(|| wrong("path", "a string"))?;
    read_path(text)
}

fn read_axis(value: &Value) -> Result<(Path, Vec<Number>), PlanRefusal> {
    let object = value
        .as_object()
        .ok_or_else(|| wrong("axes", "a JSON array of objects"))?;
    refuse_unknown(object, &["path", "values"])?;
    let path = read_path_of(object)?;
    let values = object
        .get("values")
        .ok_or_else(|| PlanRefusal::Missing("values".to_string()))?;
    let values = read_numbers(values, "values", &path)?;
    Ok((path, values))
}

fn read_draw(value: &Value) -> Result<(Path, Draw), PlanRefusal> {
    let object = value
        .as_object()
        .ok_or_else(|| wrong("draws", "a JSON array of objects"))?;
    refuse_unknown(object, &["path", "uniform", "choice"])?;
    let path = read_path_of(object)?;
    let draw = match (object.get("uniform"), object.get("choice")) {
        (Some(uniform), None) => {
            let interval = uniform
                .as_object()
                .ok_or_else(|| wrong("uniform", "a JSON object"))?;
            refuse_unknown(interval, &["low", "high"])?;
            let bound = |key: &str| match interval.get(key) {
                None => Err(PlanRefusal::Missing(key.to_string())),
                Some(Value::Number(number)) => Ok(number.clone()),
                Some(_) => Err(wrong(key, "a number")),
            };
            let low = bound("low")?;
            let high = bound("high")?;
            if nearest(&low) > nearest(&high) {
                return Err(PlanRefusal::IntervalReversed(path.text.clone()));
            }
            if !sampler::width(nearest(&low), nearest(&high)).is_finite() {
                return Err(PlanRefusal::IntervalTooWide(path.text.clone()));
            }
            Draw::Uniform { low, high }
        }
        (None, Some(choice)) => Draw::Choice(read_numbers(choice, "choice", &path)?),
        _ => return Err(PlanRefusal::DrawNotOne(path.text.clone())),
    };
    Ok((path, draw))
}

/// Which parts of one field the paths set.
#[derive(Default)]
struct Slots {
    whole: bool,
    components: [bool; 3],
}

/// Hold the paths to "set exactly once" against the base, without a card:
/// a path may not name what the base sets, two paths may not name one number,
/// a 3-vector set by components has all three, and `dt_s` is set.
fn check_paths(base: &Run, paths: &[&Path]) -> Result<(), PlanRefusal> {
    let loads = base.loads();
    let mut set: BTreeMap<&str, Slots> = BTreeMap::new();
    for path in paths {
        let set_twice = || PlanRefusal::SetTwice(path.text.clone());
        let in_base = match &path.place {
            Place::Dt => base.fields.contains_key("dt_s"),
            Place::Initial(field) => base.initial().contains_key(field),
            Place::Load { segment, field } => loads
                .get(*segment)
                .ok_or_else(|| PlanRefusal::PathNamesNoField(path.text.clone()))?
                .contains_key(field),
        };
        if in_base {
            return Err(set_twice());
        }
        let slots = set.entry(path.field_text()).or_default();
        match path.component {
            None => {
                if slots.whole || slots.components.contains(&true) {
                    return Err(set_twice());
                }
                slots.whole = true;
            }
            Some(component) => {
                if slots.whole || slots.components[component] {
                    return Err(set_twice());
                }
                slots.components[component] = true;
            }
        }
    }
    for (field, slots) in &set {
        if slots.whole {
            continue;
        }
        for (component, is_set) in slots.components.iter().enumerate() {
            if !is_set {
                return Err(PlanRefusal::NotSet(format!("{field}[{component}]")));
            }
        }
    }
    if !base.fields.contains_key("dt_s") && !set.contains_key("dt_s") {
        return Err(PlanRefusal::NotSet("dt_s".to_string()));
    }
    Ok(())
}

/// Put `number` where `path` points.
fn set(run: &mut Map<String, Value>, path: &Path, number: Number) {
    let checked = "checked when the plan was read";
    let (holder, field) = match &path.place {
        Place::Dt => {
            run.insert("dt_s".to_string(), Value::Number(number));
            return;
        }
        Place::Initial(field) => (
            run.get_mut("initial")
                .and_then(Value::as_object_mut)
                .expect(checked),
            field,
        ),
        Place::Load { segment, field } => (
            run.get_mut("segments")
                .and_then(Value::as_array_mut)
                .and_then(|segments| segments.get_mut(*segment))
                .and_then(|segment| segment.get_mut("load"))
                .and_then(Value::as_object_mut)
                .expect(checked),
            field,
        ),
    };
    match path.component {
        None => {
            holder.insert(field.clone(), Value::Number(number));
        }
        Some(component) => {
            let vector = holder
                .entry(field.clone())
                .or_insert_with(|| Value::Array(vec![Value::Null; 3]));
            vector[component] = Value::Number(number);
        }
    }
}

/// The same value with the keys of every object in the order of their UTF-8
/// bytes. A row's request is written this way, so its text does not depend on
/// the order the plan was typed in.
pub fn with_sorted_keys(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(with_sorted_keys).collect()),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut sorted = Map::new();
            for key in keys {
                sorted.insert(key.clone(), with_sorted_keys(&map[key]));
            }
            Value::Object(sorted)
        }
        other => other.clone(),
    }
}

/// One request to `run_dataset`, read and held to the rules that need no card.
#[derive(Debug, Clone)]
pub struct DatasetRequest {
    value: Value,
    domain: String,
    card: String,
    plan: Plan,
    store: Store,
    returns: Return,
    totals: Totals,
}

fn read_name(object: &Map<String, Value>, key: &str) -> Result<String, PlanRefusal> {
    match object.get(key) {
        None => Err(PlanRefusal::Missing(key.to_string())),
        Some(Value::String(text)) if !text.is_empty() => Ok(text.clone()),
        Some(Value::String(_)) => Err(PlanRefusal::Missing(key.to_string())),
        Some(_) => Err(wrong(key, "a string")),
    }
}

fn multiply(left: u64, right: u64) -> Result<u64, PlanRefusal> {
    left.checked_mul(right).ok_or(PlanRefusal::TotalsTooLarge)
}

impl DatasetRequest {
    /// Scan the text of a request for a repeated key and for a number that
    /// reads as zero, then read it.
    pub fn read(bytes: &[u8]) -> Result<DatasetRequest, PlanRefusal> {
        DatasetRequest::from_value(crate::json_text::read(bytes)?)
    }

    /// Read a request that has already been scanned.
    pub fn from_value(value: Value) -> Result<DatasetRequest, PlanRefusal> {
        let object = value
            .as_object()
            .ok_or_else(|| wrong("the request", "a JSON object"))?;
        // A plan of solves is named for what it is, before its keys are read
        // as a plan of runs' (RT3 W2 of 0.14.0 rc1).
        if object.contains_key("solver") {
            return Err(PlanRefusal::PlanOfSolves);
        }
        refuse_unknown(object, REQUEST_KEYS)?;
        let domain = read_name(object, "domain")?;
        let card = read_name(object, "card")?;
        let plan = object
            .get("plan")
            .ok_or_else(|| PlanRefusal::Missing("plan".to_string()))?
            .as_object()
            .ok_or_else(|| wrong("plan", "a JSON object"))?;
        let store = match read_name(object, "store")?.as_str() {
            "frames" => Store::Frames,
            "none" => Store::None,
            _ => return Err(wrong("store", "\"frames\" or \"none\"")),
        };
        let returns = match read_name(object, "return")?.as_str() {
            "inline" => Return::Inline,
            "file" => Return::File,
            _ => return Err(wrong("return", "\"inline\" or \"file\"")),
        };
        let (plan, totals) = read_plan(plan)?;
        Ok(DatasetRequest {
            domain,
            card,
            plan,
            store,
            returns,
            totals,
            value,
        })
    }

    /// The request as it was read.
    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn card(&self) -> &str {
        &self.card
    }

    pub fn store(&self) -> Store {
        self.store
    }

    pub fn returns(&self) -> Return {
        self.returns
    }

    /// The count of rows, the steps in total and the frames in total.
    pub fn totals(&self) -> Totals {
        self.totals
    }

    /// SHA-384 of the sealed form of the whole request, as hex.
    pub fn plan_sha384(&self) -> String {
        digest_hex(sealed_form(&self.value).as_bytes())
    }

    /// Hold the plan to the card: every key is one of the card's fields,
    /// every field is set exactly once, and every axis value, choice and
    /// sample bound lies inside the published bounds.
    pub fn check_card(&self, card: &CardFields) -> Result<(), PlanRefusal> {
        match &self.plan {
            Plan::List(runs) => {
                for run in runs {
                    check_run(run, card, &[])?;
                }
                Ok(())
            }
            Plan::Grid { base, axes } => {
                let paths: Vec<&Path> = axes.iter().map(|(path, _)| path).collect();
                check_run(base, card, &paths)?;
                for (path, values) in axes {
                    for value in values {
                        check_bound(path, value, card)?;
                    }
                }
                Ok(())
            }
            Plan::Sample { base, draws, .. } => {
                let paths: Vec<&Path> = draws.iter().map(|(path, _)| path).collect();
                check_run(base, card, &paths)?;
                for (path, draw) in draws {
                    match draw {
                        Draw::Uniform { low, high } => {
                            check_bound(path, low, card)?;
                            check_bound(path, high, card)?;
                        }
                        Draw::Choice(values) => {
                            for value in values {
                                check_bound(path, value, card)?;
                            }
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Hold the plan to the limits of one run and of one job.
    pub fn check_limits(&self, limits: &Limits) -> Result<(), PlanRefusal> {
        let totals = self.totals;
        let stores_frames = self.store == Store::Frames;
        if totals.largest_run_steps > limits.run_steps {
            return Err(PlanRefusal::RunTooLong(limits.run_steps));
        }
        if totals.largest_run_frames > limits.run_frames {
            return Err(PlanRefusal::RunRecordsTooMuch(limits.run_frames));
        }
        if totals.rows > limits.rows {
            return Err(PlanRefusal::TooManyRows(limits.rows));
        }
        if totals.steps > limits.steps {
            return Err(PlanRefusal::TooManySteps(limits.steps));
        }
        if stores_frames && totals.frames > limits.frames {
            return Err(PlanRefusal::TooManyFrames(limits.frames));
        }
        if self.returns == Return::Inline
            && (totals.rows > limits.inline_rows
                || (stores_frames && totals.frames > limits.inline_frames))
        {
            return Err(PlanRefusal::TooLargeForInline);
        }
        Ok(())
    }

    /// The whole `run_trajectory` request of row `index`, worked out without
    /// the rows before it. Its keys are in the order of their bytes.
    pub fn row(&self, index: u64) -> Result<Value, PlanRefusal> {
        if index >= self.totals.rows {
            return Err(PlanRefusal::NoSuchRow {
                row: index,
                rows: self.totals.rows,
            });
        }
        let run = match &self.plan {
            Plan::List(runs) => runs[index as usize].fields.clone(),
            Plan::Grid { base, axes } => {
                let mut run = base.fields.clone();
                // The last axis varies fastest.
                let mut rest = index;
                for (path, values) in axes.iter().rev() {
                    let count = values.len() as u64;
                    set(&mut run, path, values[(rest % count) as usize].clone());
                    rest /= count;
                }
                run
            }
            Plan::Sample { base, seed, draws } => {
                let mut run = base.fields.clone();
                for (position, (path, draw)) in draws.iter().enumerate() {
                    let k = sampler::whole_number(sampler::draw_bits(*seed, index, position as u64));
                    let number = match draw {
                        Draw::Uniform { low, high } => {
                            let value = sampler::uniform(k, nearest(low), nearest(high));
                            Number::from_f64(value)
                                .ok_or_else(|| PlanRefusal::IntervalTooWide(path.text.clone()))?
                        }
                        Draw::Choice(values) => {
                            values[sampler::choice_index(k, values.len() as u64) as usize].clone()
                        }
                    };
                    set(&mut run, path, number);
                }
                run
            }
        };
        let mut request = Map::new();
        request.insert("domain".to_string(), Value::String(self.domain.clone()));
        request.insert("card".to_string(), Value::String(self.card.clone()));
        for (key, value) in run {
            request.insert(key, value);
        }
        Ok(with_sorted_keys(&Value::Object(request)))
    }
}

fn read_plan(plan: &Map<String, Value>) -> Result<(Plan, Totals), PlanRefusal> {
    let kind = read_name(plan, "kind")?;
    let part = |key: &str| {
        plan.get(key)
            .ok_or_else(|| PlanRefusal::Missing(key.to_string()))
    };
    match kind.as_str() {
        "list" => {
            refuse_unknown(plan, &["kind", "runs"])?;
            let items = part("runs")?
                .as_array()
                .ok_or_else(|| wrong("runs", "a JSON array"))?;
            if items.is_empty() {
                return Err(PlanRefusal::EmptyPlan);
            }
            let mut runs = Vec::with_capacity(items.len());
            let mut totals = Totals {
                rows: items.len() as u64,
                steps: 0,
                frames: 0,
                largest_run_steps: 0,
                largest_run_frames: 0,
            };
            for item in items {
                let run = read_run(item, "runs", false)?;
                totals.steps = totals
                    .steps
                    .checked_add(run.steps)
                    .ok_or(PlanRefusal::TotalsTooLarge)?;
                totals.frames = totals
                    .frames
                    .checked_add(run.frames)
                    .ok_or(PlanRefusal::TotalsTooLarge)?;
                totals.largest_run_steps = totals.largest_run_steps.max(run.steps);
                totals.largest_run_frames = totals.largest_run_frames.max(run.frames);
                runs.push(run);
            }
            Ok((Plan::List(runs), totals))
        }
        "grid" => {
            refuse_unknown(plan, &["kind", "base", "axes"])?;
            let base = read_run(part("base")?, "base", true)?;
            let items = part("axes")?
                .as_array()
                .ok_or_else(|| wrong("axes", "a JSON array of objects"))?;
            let mut axes = Vec::with_capacity(items.len());
            let mut rows = 1_u64;
            for item in items {
                let axis = read_axis(item)?;
                rows = multiply(rows, axis.1.len() as u64)?;
                axes.push(axis);
            }
            let paths: Vec<&Path> = axes.iter().map(|(path, _)| path).collect();
            check_paths(&base, &paths)?;
            let totals = totals_of(&base, rows)?;
            Ok((Plan::Grid { base, axes }, totals))
        }
        "sample" => {
            refuse_unknown(
                plan,
                &["kind", "base", "rows", "generator", "seed_hex", "draws"],
            )?;
            let base = read_run(part("base")?, "base", true)?;
            let rows = match part("rows")?.as_u64() {
                Some(0) => return Err(PlanRefusal::EmptyPlan),
                Some(rows) => rows,
                None => return Err(wrong("rows", "an integer >= 1")),
            };
            let generator = read_name(plan, "generator")?;
            if generator != sampler::GENERATOR {
                return Err(PlanRefusal::UnknownGenerator(generator));
            }
            let seed_hex = part("seed_hex")?.as_str().ok_or(PlanRefusal::SeedNotExact)?;
            if !is_lower_hex(seed_hex, 16) {
                return Err(PlanRefusal::SeedNotExact);
            }
            let seed = u64::from_str_radix(seed_hex, 16).map_err(|_| PlanRefusal::SeedNotExact)?;
            let items = part("draws")?
                .as_array()
                .ok_or_else(|| wrong("draws", "a JSON array of objects"))?;
            let mut draws = Vec::with_capacity(items.len());
            for item in items {
                draws.push(read_draw(item)?);
            }
            let paths: Vec<&Path> = draws.iter().map(|(path, _)| path).collect();
            check_paths(&base, &paths)?;
            let totals = totals_of(&base, rows)?;
            Ok((Plan::Sample { base, seed, draws }, totals))
        }
        _ => Err(PlanRefusal::UnknownKind(kind)),
    }
}

/// The totals of `rows` rows that all take the steps of `base`.
fn totals_of(base: &Run, rows: u64) -> Result<Totals, PlanRefusal> {
    Ok(Totals {
        rows,
        steps: multiply(rows, base.steps)?,
        frames: multiply(rows, base.frames)?,
        largest_run_steps: base.steps,
        largest_run_frames: base.frames,
    })
}

fn check_arity(value: &Value, field: &Field, name: &str) -> Result<(), PlanRefusal> {
    match (field.components, value) {
        (1, Value::Number(_)) | (3, Value::Array(_)) => Ok(()),
        (1, _) => Err(wrong(name, "a number")),
        _ => Err(wrong(name, "an array of three numbers")),
    }
}

/// Hold one run and the paths over it to the card's field lists.
fn check_run(run: &Run, card: &CardFields, paths: &[&Path]) -> Result<(), PlanRefusal> {
    let loads = run.loads();
    for (key, value) in run.initial() {
        let field = card
            .phase_field(key)
            .ok_or_else(|| PlanRefusal::UnknownKey(key.clone()))?;
        check_arity(value, field, &format!("initial.{key}"))?;
    }
    for (index, load) in loads.iter().enumerate() {
        for (key, value) in *load {
            let field = card
                .load_field(key)
                .ok_or_else(|| PlanRefusal::UnknownKey(key.clone()))?;
            check_arity(value, field, &format!("segments[{index}].load.{key}"))?;
        }
    }
    for path in paths {
        let field = match &path.place {
            Place::Dt => continue,
            Place::Initial(field) => card.phase_field(field),
            Place::Load { field, .. } => card.load_field(field),
        };
        let field = field.ok_or_else(|| PlanRefusal::PathNamesNoField(path.text.clone()))?;
        match (field.components, path.component) {
            (1, None) | (3, Some(_)) => {}
            (3, None) => return Err(PlanRefusal::PathNeedsComponent(path.text.clone())),
            _ => return Err(PlanRefusal::PathNamesNoField(path.text.clone())),
        }
    }
    for field in &card.phase {
        let by_path = paths.iter().any(|path| path.sets_initial(&field.name));
        if !run.initial().contains_key(&field.name) && !by_path {
            return Err(PlanRefusal::NotSet(format!("initial.{}", field.name)));
        }
    }
    for (index, load) in loads.iter().enumerate() {
        for field in &card.load {
            let by_path = paths.iter().any(|path| path.sets_load(index, &field.name));
            if !load.contains_key(&field.name) && !by_path {
                return Err(PlanRefusal::NotSet(format!(
                    "segments[{index}].load.{}",
                    field.name
                )));
            }
        }
    }
    Ok(())
}

/// Refuse a number the caller stated for `path` that lies outside the card's
/// published bounds. Nothing is pulled back to a bound.
fn check_bound(path: &Path, number: &Number, card: &CardFields) -> Result<(), PlanRefusal> {
    let bounds = match &path.place {
        Place::Dt => card.dt_s,
        Place::Initial(field) => card.phase_field(field).map_or(Bounds::OPEN, |f| f.bounds),
        Place::Load { field, .. } => card.load_field(field).map_or(Bounds::OPEN, |f| f.bounds),
    };
    if bounds.holds(nearest(number)) {
        Ok(())
    } else {
        Err(PlanRefusal::OutsideCard {
            path: path.text.clone(),
            value: number.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn card(name: &str) -> CardFields {
        let text = match name {
            "thermal" => include_str!("../tests/fixtures/describe_thermal.json"),
            "ti64_lattice" => include_str!("../tests/fixtures/describe_ti64_lattice.json"),
            _ => include_str!("../tests/fixtures/describe_satellite_lageos1.json"),
        };
        CardFields::from_description(&serde_json::from_str(text).unwrap()).unwrap()
    }

    fn thermal_run() -> Value {
        json!({
            "initial": { "temperature_k": 293.15 },
            "dt_s": 0.1,
            "segments": [{ "steps": 10, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
            "record_every": 1
        })
    }

    fn request(plan: Value) -> Value {
        json!({ "domain": "energy", "card": "thermal", "plan": plan, "store": "frames", "return": "file" })
    }

    fn thermal_grid() -> Value {
        request(json!({
            "kind": "grid",
            "base": {
                "initial": { "temperature_k": 293.15 },
                "dt_s": 0.1,
                "segments": [{ "steps": 10, "load": {} }, { "steps": 5, "load": { "heat_power_w": 0.0 } }],
                "record_every": 4
            },
            "axes": [
                { "path": "segments[0].load.heat_power_w", "values": [0.0, 1.0, 2.0] },
                { "path": "segments[0].load.ambient_k", "values": [283.15, 293.15] },
                { "path": "segments[1].load.ambient_k", "values": [300] }
            ]
        }))
    }

    fn thermal_sample() -> Value {
        request(json!({
            "kind": "sample",
            "base": {
                "initial": {},
                "segments": [{ "steps": 10, "load": { "ambient_k": 293.15 } }],
                "record_every": 1
            },
            "rows": 1000,
            "generator": "splitmix64_counter_v1",
            "seed_hex": "0123456789abcdef",
            "draws": [
                { "path": "initial.temperature_k", "uniform": { "low": 233.15, "high": 398.0 } },
                { "path": "segments[0].load.heat_power_w", "uniform": { "low": -5.0, "high": 5.0 } },
                { "path": "dt_s", "choice": [0.05, 0.1, 0.2] }
            ]
        }))
    }

    fn read(value: Value) -> Result<DatasetRequest, PlanRefusal> {
        DatasetRequest::from_value(value)
    }

    fn refusal(value: Value) -> PlanRefusal {
        match read(value) {
            Ok(plan) => plan
                .check_card(&card("thermal"))
                .expect_err("the plan was accepted"),
            Err(refusal) => refusal,
        }
    }

    #[test]
    fn a_list_row_is_its_run_with_the_pair() {
        let plan = read(request(json!({ "kind": "list", "runs": [thermal_run()] }))).unwrap();
        plan.check_card(&card("thermal")).unwrap();
        assert_eq!(plan.totals().rows, 1);
        let row = plan.row(0).unwrap();
        let alone: Value = serde_json::from_str(
            r#"{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":10,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}"#,
        )
        .unwrap();
        assert_eq!(sealed_form(&row), sealed_form(&alone));
        assert_eq!(
            digest_hex(sealed_form(&row).as_bytes()),
            "8615887c32e2e2edb7d4be74b822e7f64ea384173a9332d97be6278dec255c3d57c9249ca16c5a5d5dfa6c2e43d9baf9"
        );
        assert_eq!(
            serde_json::to_string(&row).unwrap(),
            r#"{"card":"thermal","domain":"energy","dt_s":0.1,"initial":{"temperature_k":293.15},"record_every":1,"segments":[{"load":{"ambient_k":293.15,"heat_power_w":1.0},"steps":10}]}"#
        );
    }

    #[test]
    fn a_grid_is_the_product_of_its_axes_with_the_last_axis_fastest() {
        let plan = read(thermal_grid()).unwrap();
        plan.check_card(&card("thermal")).unwrap();
        let totals = plan.totals();
        assert_eq!(totals.rows, 6);
        assert_eq!(totals.steps, 90);
        assert_eq!(totals.frames, 24);
        assert_eq!(totals.largest_run_steps, 15);
        assert_eq!(totals.largest_run_frames, 4);
        let mut seen = Vec::new();
        for index in 0..6 {
            let row = plan.row(index).unwrap();
            let load = &row["segments"][0]["load"];
            seen.push((
                load["heat_power_w"].as_f64().unwrap().to_bits(),
                load["ambient_k"].as_f64().unwrap().to_bits(),
            ));
            assert_eq!(row["segments"][1]["load"]["ambient_k"], json!(300));
            assert_eq!(row["segments"][1]["load"]["heat_power_w"], json!(0.0));
            assert_eq!(row["dt_s"], json!(0.1));
        }
        let bits = |value: f64| value.to_bits();
        assert_eq!(
            seen,
            [
                (bits(0.0), bits(283.15)),
                (bits(0.0), bits(293.15)),
                (bits(1.0), bits(283.15)),
                (bits(1.0), bits(293.15)),
                (bits(2.0), bits(283.15)),
                (bits(2.0), bits(293.15)),
            ]
        );
        assert_eq!(
            plan.row(6).unwrap_err(),
            PlanRefusal::NoSuchRow { row: 6, rows: 6 }
        );
    }

    #[test]
    fn a_sampled_row_carries_the_published_draws() {
        let plan = read(thermal_sample()).unwrap();
        plan.check_card(&card("thermal")).unwrap();
        assert_eq!(plan.totals().rows, 1000);
        assert_eq!(plan.totals().steps, 10_000);
        let seed = 0x0123_4567_89ab_cdef;
        let choices = [0.05_f64, 0.1, 0.2];
        for index in [0, 1, 2, 500, 999] {
            let row = plan.row(index).unwrap();
            let k = |draw| sampler::whole_number(sampler::draw_bits(seed, index, draw));
            assert_eq!(
                row["initial"]["temperature_k"].as_f64().unwrap().to_bits(),
                sampler::uniform(k(0), 233.15, 398.0).to_bits()
            );
            assert_eq!(
                row["segments"][0]["load"]["heat_power_w"].as_f64().unwrap().to_bits(),
                sampler::uniform(k(1), -5.0, 5.0).to_bits()
            );
            assert_eq!(
                row["dt_s"].as_f64().unwrap().to_bits(),
                choices[sampler::choice_index(k(2), 3) as usize].to_bits()
            );
            assert_eq!(row["segments"][0]["load"]["ambient_k"], json!(293.15));
        }
    }

    #[test]
    fn a_sampled_number_reads_back_from_its_text_to_the_same_bits() {
        let plan = read(thermal_sample()).unwrap();
        for index in 0..1000 {
            let row = plan.row(index).unwrap();
            let text = serde_json::to_string(&row).unwrap();
            let back: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(sealed_form(&back), sealed_form(&row), "row {index}");
        }
    }

    #[test]
    fn row_n_expands_without_the_rows_before_it() {
        let first = read(thermal_sample()).unwrap();
        let all: Vec<Value> = (0..1000).map(|index| first.row(index).unwrap()).collect();
        // A second reading of the plan that is asked for one row only, and
        // for rows in falling order.
        let second = read(thermal_sample()).unwrap();
        assert_eq!(second.row(999).unwrap(), all[999]);
        for index in (0..1000).rev().step_by(37) {
            assert_eq!(second.row(index).unwrap(), all[index as usize]);
        }
        let mut distinct: Vec<String> = all.iter().map(sealed_form).collect();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 1000);
    }

    #[test]
    fn the_far_rows_of_a_large_plan_are_reached_directly() {
        let mut body = thermal_sample();
        body["plan"]["rows"] = json!(u64::MAX);
        body["plan"]["base"]["segments"][0]["steps"] = json!(1);
        let plan = read(body).unwrap();
        assert_eq!(plan.totals().rows, u64::MAX);
        let last = plan.row(u64::MAX - 1).unwrap();
        let k = sampler::whole_number(sampler::draw_bits(0x0123_4567_89ab_cdef, u64::MAX - 1, 0));
        assert_eq!(
            last["initial"]["temperature_k"].as_f64().unwrap().to_bits(),
            sampler::uniform(k, 233.15, 398.0).to_bits()
        );
        let mut body = thermal_sample();
        body["plan"]["rows"] = json!(u64::MAX);
        assert_eq!(read(body).unwrap_err(), PlanRefusal::TotalsTooLarge);
    }

    #[test]
    fn a_3_vector_is_set_by_the_base_whole_or_by_three_paths() {
        let lattice = card("ti64_lattice");
        let body = |draws: Value, load: Value| {
            json!({ "domain": "materials", "card": "ti64_lattice", "store": "none", "return": "inline", "plan": {
                "kind": "sample",
                "base": {
                    "initial": {
                        "position_m": [0.0, 0.0, 0.0], "velocity_m_s": [0.0, 0.0, 0.0],
                        "start_t_s": 0.0, "start_force_n": [0.0, 0.0, 0.0], "start_temperature_k": 293.15,
                        "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0
                    },
                    "dt_s": 1.0e-7,
                    "segments": [{ "steps": 4, "load": load }],
                    "record_every": 1
                },
                "rows": 8, "generator": "splitmix64_counter_v1", "seed_hex": "00000000000000ff",
                "draws": draws
            }})
        };
        let three = json!([
            { "path": "segments[0].load.force_n[0]", "choice": [0.0] },
            { "path": "segments[0].load.force_n[1]", "choice": [0.0] },
            { "path": "segments[0].load.force_n[2]", "uniform": { "low": 0.0, "high": 100.0 } }
        ]);
        let rest = json!({ "torque_nm": [0.0, 0.0, 0.0], "temperature_k": 300.0 });
        let plan = read(body(three.clone(), rest.clone())).unwrap();
        plan.check_card(&lattice).unwrap();
        let row = plan.row(5).unwrap();
        let force = row["segments"][0]["load"]["force_n"].as_array().unwrap();
        assert_eq!(force.len(), 3);
        assert_eq!(force[0], json!(0.0));
        assert_eq!(force[1], json!(0.0));
        let k = sampler::whole_number(sampler::draw_bits(0xff, 5, 2));
        assert_eq!(
            force[2].as_f64().unwrap().to_bits(),
            sampler::uniform(k, 0.0, 100.0).to_bits()
        );

        let two = json!([three[0], three[2]]);
        assert_eq!(
            read(body(two, rest.clone())).unwrap_err(),
            PlanRefusal::NotSet("segments[0].load.force_n[1]".to_string())
        );
        let whole = json!([{ "path": "segments[0].load.force_n", "choice": [0.0] }]);
        assert_eq!(
            read(body(whole, rest.clone()))
                .unwrap()
                .check_card(&lattice)
                .unwrap_err(),
            PlanRefusal::PathNeedsComponent("segments[0].load.force_n".to_string())
        );
        let scalar = json!([{ "path": "segments[0].load.temperature_k[0]", "choice": [300.0] },
            { "path": "segments[0].load.temperature_k[1]", "choice": [300.0] },
            { "path": "segments[0].load.temperature_k[2]", "choice": [300.0] }]);
        let with_force = json!({ "torque_nm": [0.0, 0.0, 0.0], "force_n": [0.0, 0.0, 0.0] });
        assert_eq!(
            read(body(scalar, with_force))
                .unwrap()
                .check_card(&lattice)
                .unwrap_err(),
            PlanRefusal::PathNamesNoField("segments[0].load.temperature_k[0]".to_string())
        );
        let both = json!({ "torque_nm": [0.0, 0.0, 0.0], "temperature_k": 300.0, "force_n": [0.0, 0.0, 0.0] });
        assert_eq!(
            read(body(three, both)).unwrap_err(),
            PlanRefusal::SetTwice("segments[0].load.force_n[0]".to_string())
        );
        let short = json!({ "torque_nm": [0.0, 0.0], "temperature_k": 300.0, "force_n": [0.0, 0.0, 0.0] });
        assert!(matches!(
            read(body(json!([]), short)).unwrap_err(),
            PlanRefusal::WrongType { .. }
        ));
    }

    #[test]
    fn a_repeated_key_in_a_plan_is_refused_by_name() {
        let text = br#"{"domain":"energy","card":"thermal","plan":{"kind":"list","runs":[{"initial":{"temperature_k":293.15,"temperature_k":300.0},"dt_s":0.1,"segments":[{"steps":1,"load":{"heat_power_w":0.0,"ambient_k":293.15}}],"record_every":1}]},"store":"none","return":"inline"}"#;
        assert_eq!(
            DatasetRequest::read(text).unwrap_err().to_string(),
            "Physical Violation: the key 'temperature_k' is repeated."
        );
        let zero = br#"{"domain":"energy","card":"thermal","plan":{"kind":"list","runs":[{"initial":{"temperature_k":293.15},"dt_s":1e-400,"segments":[{"steps":1,"load":{"heat_power_w":0.0,"ambient_k":293.15}}],"record_every":1}]},"store":"none","return":"inline"}"#;
        assert_eq!(
            DatasetRequest::read(zero).unwrap_err().to_string(),
            "Physical Violation: 'dt_s' is written 1e-400, a number that is not zero and reads as zero."
        );
        assert!(matches!(
            DatasetRequest::read(b"{").unwrap_err(),
            PlanRefusal::Text(TextRefusal::Unreadable(_))
        ));
    }

    #[test]
    fn an_unknown_key_a_card_constant_and_a_file_path_are_refused_by_name() {
        let mut body = thermal_grid();
        body["workers"] = json!(4);
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("workers".to_string()));
        let mut body = thermal_grid();
        body["path"] = json!("/tmp/out");
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("path".to_string()));
        let mut body = thermal_grid();
        body["plan"]["from"] = json!(0);
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("from".to_string()));
        let mut body = thermal_grid();
        body["plan"]["base"]["domain"] = json!("energy");
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("domain".to_string()));
        let mut body = thermal_grid();
        body["plan"]["base"]["segments"][0]["hold"] = json!(1);
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("hold".to_string()));
        let mut body = thermal_grid();
        body["plan"]["axes"][0]["count"] = json!(3);
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("count".to_string()));
        // A card constant is not one of the card's fields.
        let mut body = thermal_grid();
        body["plan"]["base"]["segments"][1]["load"]["plate_capacitance_j_k"] = json!(590.625);
        assert_eq!(
            refusal(body).to_string(),
            "Physical Violation: 'plate_capacitance_j_k' is not a field of this request."
        );
        let mut body = thermal_grid();
        body["plan"]["axes"][0]["path"] = json!("segments[0].load.plate_resistance_k_w");
        assert_eq!(
            refusal(body).to_string(),
            "Physical Violation: 'segments[0].load.plate_resistance_k_w' names no field of the base run."
        );
        // A file path where a number belongs.
        let mut body = thermal_grid();
        body["plan"]["base"]["initial"]["temperature_k"] = json!("/etc/passwd");
        assert!(matches!(refusal(body), PlanRefusal::WrongType { .. }));
        let mut body = thermal_grid();
        body["store"] = json!("/tmp/frames.bin");
        assert_eq!(
            refusal(body).to_string(),
            "Physical Violation: 'store' must be \"frames\" or \"none\"."
        );
    }

    #[test]
    fn a_path_that_names_no_field_of_the_base_run_is_refused() {
        for (path, expected) in [
            ("initial.pressure_pa", "names no field of the base run"),
            ("segments[0].load.temperature_k", "names no field of the base run"),
            ("segments[2].load.heat_power_w", "names no field of the base run"),
            ("segments[0].load.heat_power_w[1]", "is not set"),
            ("segments[0].steps", "cannot vary"),
            ("record_every", "cannot vary"),
            ("segments[00].load.heat_power_w", "is not a path"),
            ("segments[0].heat_power_w", "is not a path"),
            ("segments[-1].load.heat_power_w", "is not a path"),
            ("initial.temperature_k[3]", "is not a path"),
            ("initial", "is not a path"),
            ("initial.Temperature_K", "is not a path"),
            ("card", "is not a path"),
            ("", "is not a path"),
        ] {
            let mut body = thermal_grid();
            body["plan"]["axes"][0]["path"] = json!(path);
            let message = refusal(body).to_string();
            assert!(message.contains(expected), "{path}: {message}");
        }
    }

    #[test]
    fn a_field_set_twice_or_not_at_all_is_refused() {
        // By the base and by an axis.
        let mut body = thermal_grid();
        body["plan"]["axes"][0]["path"] = json!("initial.temperature_k");
        assert_eq!(
            refusal(body),
            PlanRefusal::SetTwice("initial.temperature_k".to_string())
        );
        let mut body = thermal_grid();
        body["plan"]["axes"][0]["path"] = json!("dt_s");
        assert_eq!(refusal(body), PlanRefusal::SetTwice("dt_s".to_string()));
        // By two axes.
        let mut body = thermal_grid();
        body["plan"]["axes"][1]["path"] = json!("segments[0].load.heat_power_w");
        assert_eq!(
            refusal(body),
            PlanRefusal::SetTwice("segments[0].load.heat_power_w".to_string())
        );
        // Not at all: a load field, the phase field, and the step.
        let mut body = thermal_grid();
        body["plan"]["axes"].as_array_mut().unwrap().remove(1);
        assert_eq!(
            refusal(body).to_string(),
            "Physical Violation: 'segments[0].load.ambient_k' is not set."
        );
        let mut body = thermal_grid();
        body["plan"]["base"]["initial"] = json!({});
        assert_eq!(
            refusal(body),
            PlanRefusal::NotSet("initial.temperature_k".to_string())
        );
        let mut body = thermal_grid();
        body["plan"]["base"].as_object_mut().unwrap().remove("dt_s");
        assert_eq!(refusal(body), PlanRefusal::NotSet("dt_s".to_string()));
        // A list run sets every field itself.
        let mut run = thermal_run();
        run["segments"][0]["load"].as_object_mut().unwrap().remove("ambient_k");
        assert_eq!(
            refusal(request(json!({ "kind": "list", "runs": [run] }))),
            PlanRefusal::NotSet("segments[0].load.ambient_k".to_string())
        );
        let mut run = thermal_run();
        run.as_object_mut().unwrap().remove("dt_s");
        assert_eq!(
            refusal(request(json!({ "kind": "list", "runs": [run] }))),
            PlanRefusal::Missing("dt_s".to_string())
        );
    }

    #[test]
    fn an_empty_plan_is_refused() {
        assert_eq!(
            refusal(request(json!({ "kind": "list", "runs": [] }))),
            PlanRefusal::EmptyPlan
        );
        let mut body = thermal_sample();
        body["plan"]["rows"] = json!(0);
        assert_eq!(refusal(body), PlanRefusal::EmptyPlan);
        let mut body = thermal_grid();
        body["plan"]["axes"][1]["values"] = json!([]);
        assert_eq!(
            refusal(body),
            PlanRefusal::NoValues("segments[0].load.ambient_k".to_string())
        );
        let mut body = thermal_sample();
        body["plan"]["draws"][2]["choice"] = json!([]);
        assert_eq!(refusal(body), PlanRefusal::NoValues("dt_s".to_string()));
        assert_eq!(
            PlanRefusal::EmptyPlan.to_string(),
            "Physical Violation: the plan has no rows."
        );
    }

    #[test]
    fn a_generator_other_than_the_published_one_and_a_seed_that_is_not_16_hex_digits_are_refused() {
        let mut body = thermal_sample();
        body["plan"]["generator"] = json!("mt19937");
        assert_eq!(
            refusal(body).to_string(),
            "Physical Violation: 'mt19937' is not a generator. The generator is splitmix64_counter_v1."
        );
        for seed in [
            json!("0123456789abcde"),
            json!("0123456789abcdef0"),
            json!("0123456789ABCDEF"),
            json!("0x23456789abcdef"),
            json!("+123456789abcdef"),
            json!(81985529216486895_u64),
            json!(null),
        ] {
            let mut body = thermal_sample();
            body["plan"]["seed_hex"] = seed.clone();
            assert_eq!(refusal(body), PlanRefusal::SeedNotExact, "{seed}");
        }
        let mut body = thermal_sample();
        body["plan"].as_object_mut().unwrap().remove("seed_hex");
        assert_eq!(refusal(body), PlanRefusal::Missing("seed_hex".to_string()));
        let mut body = thermal_sample();
        body["plan"]["rows"] = json!(10.0);
        assert!(matches!(refusal(body), PlanRefusal::WrongType { .. }));
    }

    #[test]
    fn a_kind_is_stated_and_is_one_of_three() {
        let mut body = thermal_grid();
        body["plan"].as_object_mut().unwrap().remove("kind");
        assert_eq!(refusal(body), PlanRefusal::Missing("kind".to_string()));
        let mut body = thermal_grid();
        body["plan"]["kind"] = json!("sweep");
        assert_eq!(refusal(body), PlanRefusal::UnknownKind("sweep".to_string()));
        // A grid does not take a list's key, and there is no stepped axis.
        let mut body = thermal_grid();
        body["plan"]["runs"] = json!([]);
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("runs".to_string()));
        let mut body = thermal_grid();
        body["plan"]["axes"][0] =
            json!({ "path": "segments[0].load.heat_power_w", "from": 0.0, "to": 2.0, "count": 3 });
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("from".to_string()));
    }

    #[test]
    fn a_value_outside_the_card_is_refused_and_not_pulled_back() {
        // The thermal window is [233.15, 398.15): its upper end is not admissible.
        let mut body = thermal_sample();
        body["plan"]["draws"][0]["uniform"]["high"] = json!(398.15);
        assert_eq!(
            refusal(body).to_string(),
            "Physical Violation: 'initial.temperature_k' 398.15 is outside the card's published bounds for that field."
        );
        let mut body = thermal_sample();
        body["plan"]["draws"][0]["uniform"]["low"] = json!(233.14999999999998);
        assert!(matches!(refusal(body), PlanRefusal::OutsideCard { .. }));
        let mut body = thermal_sample();
        body["plan"]["draws"][0]["uniform"]["high"] = json!(398.1499999999999);
        read(body).unwrap().check_card(&card("thermal")).unwrap();
        // The step ceiling is 23.625 s and is admissible. The next number up is not.
        let mut body = thermal_sample();
        body["plan"]["draws"][2]["choice"] = json!([0.05, 23.625]);
        read(body).unwrap().check_card(&card("thermal")).unwrap();
        let mut body = thermal_sample();
        body["plan"]["draws"][2]["choice"] = json!([0.05, 23.625000000000004]);
        assert_eq!(
            refusal(body),
            PlanRefusal::OutsideCard {
                path: "dt_s".to_string(),
                value: "23.625000000000004".to_string()
            }
        );
        let mut body = thermal_grid();
        body["plan"]["axes"][1]["values"] = json!([283.15, 500.0]);
        assert!(matches!(refusal(body), PlanRefusal::OutsideCard { .. }));
        // A field with no published bound takes any number.
        let mut body = thermal_grid();
        body["plan"]["axes"][0]["values"] = json!([-1.0e30, 1.0e30]);
        read(body).unwrap().check_card(&card("thermal")).unwrap();
    }

    #[test]
    fn a_sample_interval_is_closed_ordered_and_of_finite_width() {
        let mut body = thermal_sample();
        body["plan"]["draws"][1]["uniform"] = json!({ "low": 5.0, "high": -5.0 });
        assert_eq!(
            refusal(body),
            PlanRefusal::IntervalReversed("segments[0].load.heat_power_w".to_string())
        );
        let mut body = thermal_sample();
        body["plan"]["draws"][1]["uniform"] = json!({ "low": -1.0e308, "high": 1.0e308 });
        assert_eq!(
            refusal(body),
            PlanRefusal::IntervalTooWide("segments[0].load.heat_power_w".to_string())
        );
        let mut body = thermal_sample();
        body["plan"]["draws"][1]["uniform"] = json!({ "low": 2.0, "high": 2.0 });
        let plan = read(body).unwrap();
        assert_eq!(plan.row(7).unwrap()["segments"][0]["load"]["heat_power_w"], json!(2.0));
        let mut body = thermal_sample();
        body["plan"]["draws"][1]["uniform"] = json!({ "low": 2.0, "high": 3.0, "step": 0.5 });
        assert_eq!(refusal(body), PlanRefusal::UnknownKey("step".to_string()));
        let mut body = thermal_sample();
        body["plan"]["draws"][1]["choice"] = json!([1.0]);
        assert_eq!(
            refusal(body),
            PlanRefusal::DrawNotOne("segments[0].load.heat_power_w".to_string())
        );
        let mut body = thermal_sample();
        body["plan"]["draws"][1] = json!({ "path": "segments[0].load.heat_power_w" });
        assert!(matches!(refusal(body), PlanRefusal::DrawNotOne(_)));
    }

    #[test]
    fn steps_and_record_every_are_whole_numbers_of_at_least_one() {
        for (pointer, value) in [
            ("/plan/base/record_every", json!(0)),
            ("/plan/base/record_every", json!(1.0)),
            ("/plan/base/segments/0/steps", json!(0)),
            ("/plan/base/segments/0/steps", json!(10.0)),
            ("/plan/base/segments/0/steps", json!("10")),
        ] {
            let mut body = thermal_grid();
            *body.pointer_mut(pointer).unwrap() = value;
            let message = refusal(body).to_string();
            assert!(message.contains("must be an integer >= 1"), "{message}");
        }
        let mut body = thermal_grid();
        body["plan"]["base"]["segments"] = json!([]);
        assert_eq!(refusal(body), PlanRefusal::NoSegments);
    }

    #[test]
    fn the_limits_refuse_by_the_published_sentences() {
        let limits = Limits {
            run_steps: 1_000_000,
            run_frames: 10_000,
            rows: 100_000,
            steps: 100_000_000,
            frames: 16_777_216,
            inline_rows: 1_000,
            inline_frames: 10_000,
        };
        let plan = read(thermal_sample()).unwrap();
        plan.check_limits(&limits).unwrap();
        let limited = |edit: &dyn Fn(&mut Value)| {
            let mut body = thermal_sample();
            edit(&mut body);
            read(body).unwrap().check_limits(&limits).unwrap_err().to_string()
        };
        assert_eq!(
            limited(&|body| body["plan"]["rows"] = json!(100_001)),
            "Physical Violation: the dataset asks for more than 100000 runs."
        );
        assert_eq!(
            limited(&|body| {
                body["plan"]["rows"] = json!(100_000);
                body["plan"]["base"]["segments"][0]["steps"] = json!(1_001);
                body["plan"]["base"]["record_every"] = json!(1_001);
            }),
            "Physical Violation: the dataset asks for more than 100000000 steps in total."
        );
        assert_eq!(
            limited(&|body| {
                body["plan"]["rows"] = json!(100_000);
                body["plan"]["base"]["segments"][0]["steps"] = json!(1_000);
                body["plan"]["base"]["record_every"] = json!(5);
            }),
            "Physical Violation: the dataset would record more than 16777216 frames."
        );
        assert_eq!(
            limited(&|body| body["plan"]["base"]["segments"][0]["steps"] = json!(1_000_001)),
            "Physical Violation: the run asks for more than 1000000 steps."
        );
        assert_eq!(
            limited(&|body| body["plan"]["base"]["segments"][0]["steps"] = json!(10_001)),
            "Physical Violation: the run would record more than 10000 frames."
        );
        assert_eq!(
            limited(&|body| {
                body["return"] = json!("inline");
                body["plan"]["rows"] = json!(1_001);
            }),
            "Physical Violation: the dataset is too large to return inline."
        );
        assert_eq!(
            limited(&|body| {
                body["return"] = json!("inline");
                body["plan"]["rows"] = json!(1_000);
                body["plan"]["base"]["segments"][0]["steps"] = json!(11);
            }),
            "Physical Violation: the dataset is too large to return inline."
        );
    }

    #[test]
    fn the_order_of_keys_and_the_spelling_of_numbers_do_not_change_the_plan_hash_or_a_row() {
        let first = DatasetRequest::read(
            br#"{"domain":"energy","card":"thermal","plan":{"kind":"grid","base":{"initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":10,"load":{"ambient_k":293.15}}],"record_every":1},"axes":[{"path":"segments[0].load.heat_power_w","values":[0,1,2]}]},"store":"frames","return":"file"}"#,
        )
        .unwrap();
        let second = DatasetRequest::read(
            b"{ \"return\": \"file\", \"store\": \"frames\",\n \"plan\": { \"axes\": [ { \"values\": [0.0, 1e0, 2.00], \"path\": \"segments[0].load.heat_power_w\" } ],\n \"base\": { \"record_every\": 1, \"segments\": [ { \"load\": { \"ambient_k\": 2.9315e2 }, \"steps\": 10 } ], \"dt_s\": 1e-1, \"initial\": { \"temperature_k\": 293.150 } }, \"kind\": \"grid\" },\n \"card\": \"thermal\", \"domain\": \"energy\" }",
        )
        .unwrap();
        assert_eq!(first.plan_sha384(), second.plan_sha384());
        for index in 0..3 {
            assert_eq!(
                sealed_form(&first.row(index).unwrap()),
                sealed_form(&second.row(index).unwrap())
            );
        }
        // The order of the axes is part of the plan: it sets the order of the rows.
        let third = read(thermal_grid()).unwrap();
        let mut swapped = thermal_grid();
        swapped["plan"]["axes"].as_array_mut().unwrap().swap(0, 1);
        let fourth = read(swapped).unwrap();
        assert_ne!(third.plan_sha384(), fourth.plan_sha384());
        assert_ne!(
            sealed_form(&third.row(1).unwrap()),
            sealed_form(&fourth.row(1).unwrap())
        );
    }

    #[test]
    fn a_satellite_list_run_carries_whole_vectors_and_an_empty_load() {
        let satellite = card("satellite_lageos1");
        let run = json!({
            "initial": { "epoch_utc_s": 842443200.0, "position_m": [-9107816.6, -3763438.4, 7345405.3], "velocity_m_s": [1640.2, 4058.4, 4146.0] },
            "dt_s": 10.0,
            "segments": [{ "steps": 1356, "load": {} }],
            "record_every": 12
        });
        let body = json!({ "domain": "aerospace", "card": "satellite_lageos1", "plan": { "kind": "list", "runs": [run.clone(), run] }, "store": "frames", "return": "file" });
        let plan = read(body).unwrap();
        plan.check_card(&satellite).unwrap();
        assert_eq!(plan.totals().rows, 2);
        assert_eq!(plan.totals().steps, 2712);
        assert_eq!(plan.totals().frames, 226);
    }
}
