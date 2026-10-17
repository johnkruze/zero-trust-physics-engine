//! `run_dataset` and `verify_dataset`.
//!
//! A dataset is a declared plan of ordinary runs. Every row is the
//! `run_trajectory` request the plan expands to, run through the same code
//! `run_trajectory` uses after its permit is taken. The rules for the plan,
//! the rows, the row tree and the seal are in the `genesis_seal` crate. This
//! file reads the request, holds it to the card and the limits, takes the
//! permits, runs the rows in plan order, and hands the results to the crate.
//!
//! A fault in the plan refuses the whole job and seals nothing. A row the
//! door refuses before computing is an `unadmitted` row. A run that refuses
//! part way is a `refused` row with its run seal.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use genesis_seal::card::{Bounds, CardFields};
use genesis_seal::dataset::{
    Assembled, Counts, DatasetRefusal, DatasetReport, DatasetWriter, Head, PreparedRow, RowResult, assemble,
    check_dataset, check_dataset_stream, dataset_seal, prepare_row,
};
use genesis_seal::plan::{DatasetRequest, Limits, PlanRefusal, Return, Store};
use genesis_seal::rows::Row;
use genesis_seal::sealed_form::digest_hex;
use serde_json::{Map, Value, json};
use tokio::sync::OwnedSemaphorePermit;

use crate::descriptor;
use crate::view::View;
use crate::{AppState, DoorError, trajectory};

pub const MAX_DATASET_ROWS: u64 = 100_000;
pub const MAX_DATASET_STEPS: u64 = 100_000_000;
pub const MAX_DATASET_FRAMES: u64 = 16_777_216;
pub const MAX_INLINE_ROWS: u64 = 1_000;
pub const MAX_INLINE_FRAMES: u64 = 10_000;

const LIMITS: Limits = Limits {
    run_steps: trajectory::MAX_TRAJECTORY_STEPS,
    run_frames: trajectory::MAX_RECORDED_FRAMES,
    rows: MAX_DATASET_ROWS,
    steps: MAX_DATASET_STEPS,
    frames: MAX_DATASET_FRAMES,
    inline_rows: MAX_INLINE_ROWS,
    inline_frames: MAX_INLINE_FRAMES,
};

const NO_FILE_DIRECTORY: &str =
    "this connection was started without a directory for dataset files.";

/// How a dataset that came back in the answer is checked. A check by seal
/// reads a dataset the gateway stored in files; one returned inline was not
/// stored, and the caller holds its three parts.
pub(crate) const VERIFY_BY_PARTS: &str = "A dataset returned in the answer (return \"inline\") is not stored: verify it by sending its run_dataset request as 'request' with its 'dataset_seal', which runs the plan again, or by sending back the head, rows and frames_hex of its full answer (answer \"full\").";

/// A check by seal on a gateway that stores nothing.
fn no_directory_to_verify() -> String {
    format!("{NO_FILE_DIRECTORY} {VERIFY_BY_PARTS}")
}

/// A check by seal for a seal no stored folder carries.
fn not_stored() -> String {
    format!("no dataset is stored under that seal. {VERIFY_BY_PARTS}")
}

/// What a caller is told when every seal holds and the rows are not, byte for
/// byte, the rows the head names. One sentence: what each of the two means,
/// and which to report.

const BUSY: &str = "Too many concurrent physics tasks.";

/// An inline job holds every row's run until the rows are sealed, whether or
/// not the frames are returned. So the frames it records are limited with
/// `store: "none"` too.
const INLINE_RECORDS_TOO_MUCH: &str =
    "Physical Violation: an inline dataset would record more than 10000 frames.";

fn plan_fault(refusal: PlanRefusal) -> DoorError {
    DoorError::Unprocessable(refusal.to_string())
}

/// The key of a `run_dataset` request that says how much of each row the
/// answer carries. It is read out of the request before the plan is sealed,
/// so the dataset seal is the same whichever form is asked for.
pub(crate) const ANSWER: &str = "answer";

/// The most characters an inline answer may hold. It is counted from the
/// plan before anything runs. It is under 50,343 characters, the smallest
/// answer that agents calling the door could not read.
pub const MAX_INLINE_ANSWER_CHARS: u64 = 40_000;

/// How much of each row an inline answer carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Form {
    /// One summary row per run: the values it sets, its outcome, steps
    /// landed, refusal name and seal.
    Summary,
    /// The summary rows with each run's final state in decimals.
    States,
    /// Every row whole, and the frames when the plan stores them.
    Full,
}

/// A request with its answer form read out, and every base value an axis
/// or a draw replaces taken out of the base.
pub(crate) struct Normalized {
    pub request: Value,
    pub form: Form,
    /// True when the request named its form.
    pub form_given: bool,
    /// The paths whose base value an axis or a draw replaced, in plan order.
    pub replaced: Vec<String>,
}

impl Normalized {
    /// True for a plan of solves: the request names a solver.
    pub(crate) fn is_solves(&self) -> bool {
        self.request.get("solver").is_some()
    }
}

/// What the answer says when an axis or a draw replaced a base value.
pub(crate) const BASE_REPLACED: &str = "base_replaced";

/// Read the answer form out of a request, and take out of a grid's or a
/// sample's base every value an axis or a draw also sets, so the axis value
/// replaces it. The request that is sealed is the one that results, so a plan
/// sent with the value in its base and sent without it is one dataset. A
/// 3-vector the base sets whole is replaced only when the paths set all three
/// of its components.
pub(crate) fn normalize(request: Value) -> Result<Normalized, DoorError> {
    // An axis or a draw that would set a value of a signed reading of the
    // base is refused before the base is read: the value run would not be
    // the value signed.
    crate::signed_load::refuse_replaced_in_plan(&request)?;
    let Value::Object(mut object) = request else {
        return Ok(Normalized { request, form: Form::Summary, form_given: false, replaced: Vec::new() });
    };
    let (form, form_given) = match object.shift_remove(ANSWER) {
        None => (Form::Summary, false),
        Some(Value::String(text)) if text == "summary" => (Form::Summary, true),
        Some(Value::String(text)) if text == "states" => (Form::States, true),
        Some(Value::String(text)) if text == "full" => (Form::Full, true),
        Some(_) => return Err(wrong(ANSWER, "\"summary\", \"states\" or \"full\"")),
    };
    let replaced = replace_base_values(&mut object);
    Ok(Normalized { request: Value::Object(object), form, form_given, replaced })
}

/// Where a path of a plan points: `dt_s`, a field of `initial`, a field of a
/// segment's `load`, or a field of a solve's `parameters`.
enum Spot {
    Dt,
    Initial(String),
    Load(usize, String),
    Parameter(String),
}

/// A path read as far as the replacing of base values needs: where it points,
/// its component, and the path without its component. `None` for a path that
/// is not written as a path; the plan's own reading names it.
fn read_spot(text: &str) -> Option<(Spot, Option<usize>, String)> {
    let (field, component) = match text.len().checked_sub(3).map(|at| text.split_at(at)) {
        Some((field, "[0]")) => (field, Some(0)),
        Some((field, "[1]")) => (field, Some(1)),
        Some((field, "[2]")) => (field, Some(2)),
        _ => (text, None),
    };
    let plain = |name: &str| genesis_seal::json_text::is_plain_field_name(name).then(|| name.to_string());
    let spot = if field == "dt_s" {
        Spot::Dt
    } else if let Some(name) = field.strip_prefix("initial.") {
        Spot::Initial(plain(name)?)
    } else if let Some(name) = field.strip_prefix("parameters.") {
        Spot::Parameter(plain(name)?)
    } else {
        let rest = field.strip_prefix("segments[")?;
        let (index, rest) = rest.split_once(']')?;
        if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) || (index.len() > 1 && index.starts_with('0')) {
            return None;
        }
        Spot::Load(index.parse().ok()?, plain(rest.strip_prefix(".load.")?)?)
    };
    Some((spot, component, field.to_string()))
}

/// The object of `run` that holds the field `spot` names, and the field's key.
fn holder_of<'a>(run: &'a mut Map<String, Value>, spot: &Spot) -> Option<(&'a mut Map<String, Value>, String)> {
    match spot {
        Spot::Dt => Some((run, "dt_s".to_string())),
        Spot::Initial(name) => Some((run.get_mut("initial")?.as_object_mut()?, name.clone())),
        Spot::Parameter(name) => Some((run.get_mut("parameters")?.as_object_mut()?, name.clone())),
        Spot::Load(index, name) => Some((
            run.get_mut("segments")?.as_array_mut()?.get_mut(*index)?.get_mut("load")?.as_object_mut()?,
            name.clone(),
        )),
    }
}

/// The value at `path` in a row's request, for the summary of a grid's or a
/// sample's row.
fn value_at(run: &Value, path: &str) -> Option<Value> {
    let (spot, component, _) = read_spot(path)?;
    let value = match &spot {
        Spot::Dt => run.get("dt_s")?,
        Spot::Initial(name) => run.get("initial")?.get(name)?,
        Spot::Parameter(name) => run.get("parameters")?.get(name)?,
        Spot::Load(index, name) => run.get("segments")?.get(*index)?.get("load")?.get(name)?,
    };
    match component {
        None => Some(value.clone()),
        Some(component) => value.get(component).cloned(),
    }
}

/// The paths of a grid's axes or a sample's draws, in plan order.
pub(crate) fn varied_paths(request: &Value) -> Vec<String> {
    let plan = &request["plan"];
    let key = match plan.get("kind").and_then(Value::as_str) {
        Some("grid") => "axes",
        Some("sample") => "draws",
        _ => return Vec::new(),
    };
    plan.get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(|item| item.get("path")?.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn replace_base_values(object: &mut Map<String, Value>) -> Vec<String> {
    let paths = varied_paths(&Value::Object(object.clone()));
    let Some(base) = object
        .get_mut("plan")
        .and_then(|plan| plan.get_mut("base"))
        .and_then(Value::as_object_mut)
    else {
        return Vec::new();
    };
    let mut replaced = Vec::new();
    for path in &paths {
        let Some((spot, component, field)) = read_spot(path) else { continue };
        if component.is_some() && !(0..3).all(|c| paths.contains(&format!("{field}[{c}]"))) {
            continue;
        }
        if let Some((holder, key)) = holder_of(base, &spot) {
            if holder.shift_remove(&key).is_some() {
                replaced.push(field);
            }
        }
    }
    replaced
}

/// Run a plan and answer with `head` and one summary row per run, or with
/// `head`, `rows` and `frames_hex` when the full rows are asked for, or, for
/// `return: "file"`, with `head` and `files`, as JSON text. A plan of solves
/// goes to its own runner.
pub(crate) async fn run(state: &AppState, request: Value) -> Result<String, DoorError> {
    let normalized = normalize(request)?;
    if normalized.is_solves() {
        return crate::solve_plan::run(state, normalized).await;
    }
    // With return "file" the answer is the head and the names of the files,
    // whatever the form: the rows are in the rows file.
    let Normalized { request, form, replaced, .. } = normalized;
    let (request, directory) = admit(state, request, true)?;
    if request.returns() == Return::Inline {
        let counted = inline_answer_chars(&request, form);
        if counted > MAX_INLINE_ANSWER_CHARS {
            return Err(DoorError::Unprocessable(too_large_inline(counted, form, request.store() == Store::Frames)));
        }
    }
    let workers = take_workers(state, &request)?;
    let state = state.clone();
    let task = tokio::task::spawn_blocking(move || {
        let worker_count = workers.len();
        let result = match directory {
            Some(directory) => run_to_files(&state, &request, &directory, worker_count, &replaced),
            None => run_inline(&request, form, &replaced),
        };
        #[cfg(test)]
        if !state.permit_hold.is_zero() {
            std::thread::sleep(state.permit_hold);
        }
        drop(workers);
        result
    });
    task.await.unwrap_or_else(|_| {
        Err(DoorError::Internal(
            "Physical Violation: the dataset job stopped before it finished. Nothing was written."
                .to_string(),
        ))
    })
}

/// The refusal of an inline answer counted above the limit, with what to change.
pub(crate) fn too_large_inline(counted: u64, form: Form, stores_frames: bool) -> String {
    let mut text = format!(
        "Physical Violation: the inline answer would come to about {counted} characters, more than the {MAX_INLINE_ANSWER_CHARS} one inline answer may have. It is counted from the plan before anything runs. To make it smaller:"
    );
    if form != Form::Summary {
        text.push_str(" leave out 'answer' for one summary row per run, with the values it sets, its outcome, steps landed, refusal name and seal;");
    }
    if form == Form::Full {
        if stores_frames {
            text.push_str(" store \"none\" returns no frames;");
        }
    }
    text.push_str(" send fewer runs; or, on a connection that stores datasets as files, return \"file\", which stores the rows and answers with the head, and verify_dataset verifies it by its dataset_seal. Nothing was run.");
    text
}

/// The characters a summary row of a card's run holds beyond its varied
/// values: the keys, the row number, the outcome, the steps landed, the
/// refusal's name and the run seal. With the final states asked for, each
/// number of the final state adds its name and 28 characters, each 3-vector
/// its name and 84, each flag its name and 9.
const SUMMARY_ROW_FIXED_CHARS: u64 = 220;

/// The characters a whole row of a card's run holds beyond its request and
/// its final state: the keys, the outcome, the refusal and its sentence (up
/// to 1,000 characters), the two ledger errors, the state digest, the last
/// link, the run seal and the leaf. Each number of the final state adds twice
/// its name and 60 characters, with its hex; each 3-vector twice its name and
/// 150; each flag its name and 9.
const FULL_ROW_FIXED_CHARS: u64 = 1_900;

/// The characters of the head beyond the request it repeats: the counts, the
/// five hashes and the receipt.
const HEAD_FIXED_CHARS: u64 = 900;

/// The characters of the final state of one row of `card`, in decimals alone
/// (`full` false) or with their hex (`full` true).
fn state_chars(domain: &str, card: &str, full: bool) -> u64 {
    crate::state_fields(domain, card)
        .unwrap_or_default()
        .iter()
        .map(|(name, kind)| {
            let name = name.len() as u64;
            match (kind, full) {
                (crate::QuantityKind::Number, false) => name + 28,
                (crate::QuantityKind::Vector3, false) => name + 84,
                (crate::QuantityKind::Number, true) => 2 * name + 60,
                (crate::QuantityKind::Vector3, true) => 2 * name + 150,
                (crate::QuantityKind::Flag, _) => name + 9,
            }
        })
        .sum()
}

/// The values a grid's or a sample's row sets, by path, for its summary row.
pub(crate) fn set_of(paths: &[String], row: &Value) -> Option<Value> {
    if paths.is_empty() {
        return None;
    }
    let mut set = Map::new();
    for path in paths {
        set.insert(path.clone(), value_at(row, path).unwrap_or(Value::Null));
    }
    Some(Value::Object(set))
}

/// The characters an inline answer of `form` can come to, counted from the
/// plan before anything runs: the head with the whole request, and for each
/// row its summary or its whole row; with the full rows, 128 characters for
/// each recorded state when the plan stores frames.
pub(crate) fn inline_answer_chars(request: &DatasetRequest, form: Form) -> u64 {
    let totals = request.totals();
    let request_chars = serde_json::to_string(request.value()).map(|text| text.len() as u64).unwrap_or(u64::MAX);
    let mut counted = request_chars.saturating_add(HEAD_FIXED_CHARS).saturating_add(SUMMARY_ABOUT.len() as u64);
    let paths = varied_paths(request.value());
    let per_row = match form {
        Form::Summary => SUMMARY_ROW_FIXED_CHARS,
        Form::States => SUMMARY_ROW_FIXED_CHARS + state_chars(request.domain(), request.card(), false),
        Form::Full => FULL_ROW_FIXED_CHARS + state_chars(request.domain(), request.card(), true),
    };
    for index in 0..totals.rows {
        let Ok(row) = request.row(index) else { return u64::MAX };
        let own = match form {
            Form::Summary | Form::States => set_of(&paths, &row).map_or(0, |set| set.to_string().len() as u64 + 8),
            Form::Full => row.to_string().len() as u64,
        };
        counted = counted.saturating_add(per_row).saturating_add(own);
        if counted > MAX_INLINE_ANSWER_CHARS.saturating_mul(4) {
            return counted;
        }
    }
    if form == Form::Full && request.store() == Store::Frames {
        counted = counted.saturating_add(totals.frames.saturating_add(1).saturating_mul(128));
    }
    counted
}

/// What a summary answer says of itself.
pub(crate) const SUMMARY_ABOUT: &str = "One summary row per run: for a grid or a sample the values the row sets, then its outcome, steps landed, refusal name and run seal, and with \"answer\": \"states\" its final state in decimals. The head and the dataset_seal are those of the full rows. Send the request again with \"answer\": \"states\" for each final state, or \"full\" for every row whole; verify the seal with verify_dataset given this request and the dataset_seal; or replay one row's request through run_trajectory, with a readout, to read its quantities.";

/// The name of a refusal in round brackets at the end of a door's sentence,
/// as an unadmitted row carries it; `None` when the sentence gives none.
pub(crate) fn name_in_brackets(sentence: &str) -> Option<String> {
    let open = sentence.rfind('(')?;
    let inner = &sentence[open + 1..];
    let close = inner.find(')')?;
    let name = &inner[..close];
    let first = name.chars().next()?;
    (first.is_ascii_uppercase() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then(|| name.to_string())
}

/// The final state in decimals: every key of the state but the `_hex` ones.
pub(crate) fn decimals_of(state: &Value) -> Value {
    match state.as_object() {
        Some(state) => Value::Object(
            state
                .iter()
                .filter(|(key, _)| !key.ends_with("_hex"))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ),
        None => Value::Null,
    }
}

/// One summary row of a card's run, with its final state in decimals when
/// `states` is true.
fn summary_row(index: u64, result: &RowResult, set: Option<Value>, states: bool) -> Value {
    let mut row = Map::new();
    row.insert("row".to_string(), Value::from(index));
    if let Some(set) = set {
        row.insert("set".to_string(), set);
    }
    match result {
        RowResult::Run(response) => {
            row.insert("outcome".to_string(), response["outcome"].clone());
            row.insert("steps_landed".to_string(), response["steps_landed"].clone());
            row.insert("reject".to_string(), response["reject"].clone());
            row.insert("run_seal".to_string(), response["run_seal"].clone());
            if states {
                row.insert("final_state".to_string(), decimals_of(&response["final_state"]));
            }
        }
        RowResult::Unadmitted(sentence) => {
            row.insert("outcome".to_string(), Value::from("unadmitted"));
            row.insert("steps_landed".to_string(), Value::Null);
            row.insert("reject".to_string(), name_in_brackets(sentence).map_or(Value::Null, Value::from));
            row.insert("run_seal".to_string(), Value::Null);
            if states {
                row.insert("final_state".to_string(), Value::Null);
            }
        }
    }
    Value::Object(row)
}

/// A card's fields with every bound open. A grid is held to the card's field
/// lists and to "set exactly once" as a whole, and each of its rows to the
/// card's bounds alone, as the rows of a list are: a row with a value outside
/// a bound is an unadmitted row with the door's sentence and the refusal's
/// name, and the other rows run.
fn without_bounds(fields: &CardFields) -> CardFields {
    let open = |list: &[genesis_seal::card::Field]| {
        list.iter()
            .map(|field| genesis_seal::card::Field { bounds: Bounds::OPEN, ..field.clone() })
            .collect()
    };
    CardFields { phase: open(&fields.phase), load: open(&fields.load), dt_s: Bounds::OPEN }
}

/// A plan read and held to the card and to the limits, as `run_dataset` holds
/// it, and the directory a file job writes to. With `stores` false nothing is
/// written, so a plan that returns files asks for no directory. The request
/// is the one `normalize` gives.
fn admit(
    state: &AppState,
    request: Value,
    stores: bool,
) -> Result<(DatasetRequest, Option<Arc<PathBuf>>), DoorError> {
    let request = DatasetRequest::from_value(request).map_err(plan_fault)?;
    if state.view().find(request.domain(), request.card()).is_none() {
        return Err(DoorError::NotFound(crate::CARD_NOT_RECOGNIZED.to_string()));
    }
    let description = descriptor::describe_value_in(state.view(), request.domain(), request.card())
        .ok_or_else(|| DoorError::NotFound(crate::CARD_NOT_RECOGNIZED.to_string()))?;
    let fields = CardFields::from_description(&description).map_err(|refusal| {
        DoorError::Internal(format!(
            "Physical Violation: the card's description was not read. {refusal}"
        ))
    })?;
    if request.value()["plan"]["kind"] == "grid" {
        request.check_card(&without_bounds(&fields)).map_err(plan_fault)?;
    } else {
        request.check_card(&fields).map_err(plan_fault)?;
    }
    let to_file = request.returns() == Return::File;
    let directory = match (to_file && stores, &state.dataset_dir) {
        (true, None) => return Err(DoorError::Unprocessable(NO_FILE_DIRECTORY.to_string())),
        (true, Some(directory)) => Some(Arc::clone(directory)),
        (false, _) => None,
    };
    request.check_limits(&LIMITS).map_err(plan_fault)?;
    if !to_file && request.totals().frames > MAX_INLINE_FRAMES {
        return Err(DoorError::Unprocessable(INLINE_RECORDS_TOO_MUCH.to_string()));
    }
    Ok((request, directory))
}

/// The workers of a job. One dataset permit and one physics permit make a
/// worker. A job needs one worker, and a missing permit is a busy door. A
/// plan that returns files takes more pairs while both pools have one free,
/// up to its rows, and never waits for a permit. An inline plan takes one
/// pair. Every permit goes back when the job's task ends, by error or by
/// panic, because the task owns them.
fn take_workers(state: &AppState, request: &DatasetRequest) -> Result<Vec<Pair>, DoorError> {
    let mut workers = vec![take_pair(state)?];
    if request.returns() == Return::File {
        let wanted = request.totals().rows.min(usize::MAX as u64) as usize;
        while workers.len() < wanted {
            match take_pair(state) {
                Ok(pair) => workers.push(pair),
                Err(_) => break,
            }
        }
    }
    Ok(workers)
}

/// The dataset seal a plan computes to, worked out again: the plan held to
/// the card and the limits as `run_dataset` holds it, under the same permits,
/// its rows run in plan order and sealed as they arrive. Nothing is written
/// and nothing stored is read. Beside it, when it is not the seal `given`,
/// what the seal says of the engine version (`run_plan_version_words`).
async fn recompute_seal(state: &AppState, request: Value, given: &str) -> Result<(String, Option<String>), DoorError> {
    let normalized = normalize(request)?;
    if normalized.is_solves() {
        return crate::solve_plan::recompute_seal(state, normalized, given).await;
    }
    let (request, _) = admit(state, normalized.request, false)?;
    let workers = take_workers(state, &request)?;
    let state = state.clone();
    let task = tokio::task::spawn_blocking(move || {
        let worker_count = workers.len();
        let frames = match request.store() {
            Store::Frames => Some(Discard),
            Store::None => None,
        };
        let result = DatasetWriter::new(&request, crate::ENGINE_VERSION, std::io::sink(), frames)
            .map_err(not_sealed)
            .and_then(|mut writer| {
                run_ordered(&state, &request, worker_count, |row| writer.push_prepared(row).map_err(not_sealed))?;
                writer.finish().map_err(not_sealed)
            })
            .map(|written| written.head);
        #[cfg(test)]
        if !state.permit_hold.is_zero() {
            std::thread::sleep(state.permit_hold);
        }
        drop(workers);
        result
    });
    let head = task.await.unwrap_or_else(|_| {
        Err(DoorError::Internal(
            "Physical Violation: the dataset job stopped before it finished. Nothing was written."
                .to_string(),
        ))
    })?;
    let words = (head.dataset_seal != given).then(|| run_plan_version_words(&head, given));
    Ok((head.dataset_seal, words))
}

/// What a check by request of a plan of runs says when the seal given is not
/// the one computed: the dataset seal covers `engine_version` through its
/// counts, and, when the seal given is this plan's under another version's
/// name, that version beside this engine's (the red team of 0.13.0, D4).
fn run_plan_version_words(head: &Head, given: &str) -> String {
    let ours = &head.counts.engine_version;
    let theirs = crate::verify_by_request::other_versions("genesis_kernel", ours).into_iter().find(|name| {
        let counts = Counts { engine_version: name.clone(), ..head.counts.clone() };
        dataset_seal(&head.plan_sha256, &head.merkle_root, &counts) == given
    });
    match theirs {
        Some(theirs) => format!(
            "The dataset_seal covers engine_version through its counts: the seal given is this plan's dataset seal with engine_version {theirs}, another engine version's, and this engine writes {ours}, so it does not hold here."
        ),
        None => format!(
            "The dataset_seal covers engine_version through its counts, {ours} on this engine: the seal of a dataset another engine version made does not hold here."
        ),
    }
}

/// A frame sink that keeps nothing: a check by request needs the seal, which
/// covers the count of frames and not their file.
struct Discard;

impl std::io::Write for Discard {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Read for Discard {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl std::io::Seek for Discard {
    fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
        Ok(0)
    }
}

/// What a check by request of a dataset compared, said in its answer.
pub(crate) const DATASET_BY_REQUEST_COMPARED: &str = "The plan was run again, row by row, and the dataset_seal given was compared with the seal of that run; holds is that comparison. The seal covers the plan's values, every row's run seal and the counts, so a plan sent with a number spelled another way (310 and 310.0) has the same seal. No copy of the rows or the head was read and nothing was stored: the decimal copies in a copy of the rows and the head's receipt were not compared.";

/// The same of a plan of solves, whose rows carry solution seals: until
/// 0.14.0 rc2 it was answered with the words of a plan of runs, "every row's
/// run seal" (the red team of rc1, RT3 W6).
pub(crate) const DATASET_OF_SOLVES_BY_REQUEST_COMPARED: &str = "The plan was solved again, row by row, and the dataset_seal given was compared with the seal of that plan; holds is that comparison. The seal covers the plan's values, every row's solution seal and the counts, so a plan sent with a number spelled another way (310 and 310.0) has the same seal. No copy of the rows or the head was read and nothing was stored: the decimal copies in a copy of the rows and the head's receipt were not compared.";

pub(crate) type Pair = (OwnedSemaphorePermit, crate::PhysicsPermit);

/// One dataset permit and one physics permit, or a busy door. A dataset
/// permit taken when no physics permit is free goes back at once.
pub(crate) fn take_pair(state: &AppState) -> Result<Pair, DoorError> {
    let dataset_permit = Arc::clone(&state.dataset_permits)
        .try_acquire_owned()
        .map_err(|_| DoorError::TooMany(BUSY.to_string()))?;
    let physics_permit = crate::acquire_physics_permit(state)
        .map_err(|(_, message)| DoorError::TooMany(message))?;
    Ok((dataset_permit, physics_permit))
}

/// What a row came to: the run of its request as `POST /run_trajectory`
/// answers it, or the door's refusal of it before computing.
fn run_row(request: &DatasetRequest, index: u64) -> Result<RowResult, DoorError> {
    let row_request = request.row(index).map_err(plan_fault)?;
    match trajectory::prepare(row_request) {
        // The door refuses this row on its own before computing.
        Err(error) => Ok(RowResult::Unadmitted(error.message().to_string())),
        Ok(prepared) => {
            let response = trajectory::execute(prepared).map_err(DoorError::Unprocessable)?;
            // The response as `POST /run_trajectory` writes it.
            let text = serde_json::to_string(&response).map_err(|error| {
                DoorError::Internal(format!(
                    "Physical Violation: the response could not be encoded. {error}"
                ))
            })?;
            let parsed = serde_json::from_str::<Value>(&text).map_err(|error| {
                DoorError::Internal(format!(
                    "Physical Violation: the response could not be read back. {error}"
                ))
            })?;
            Ok(RowResult::Run(parsed))
        }
    }
}

/// Run the rows one after another, in plan order, and seal them.
fn run_inline(request: &DatasetRequest, form: Form, replaced: &[String]) -> Result<String, DoorError> {
    let planned = request.totals().rows;
    let mut results = Vec::with_capacity(planned as usize);
    for index in 0..planned {
        results.push(run_row(request, index)?);
    }
    answer_of_results(request, form, replaced, &results)
}

/// The rows of a plan run one after another in plan order, for a test that
/// writes a dataset again from them as anyone can, by the published rules.
#[cfg(test)]
pub(crate) fn results_of(request: &DatasetRequest) -> Vec<RowResult> {
    (0..request.totals().rows).map(|index| run_row(request, index).unwrap_or_else(|error| panic!("{}", error.message()))).collect()
}

/// The answer of a plan whose rows are `results`, sealed and written as
/// `run_dataset` writes it.
pub(crate) fn answer_of_results(request: &DatasetRequest, form: Form, replaced: &[String], results: &[RowResult]) -> Result<String, DoorError> {
    let mut assembled = assemble(request, crate::ENGINE_VERSION, results).map_err(|refusal| {
        DoorError::Internal(format!(
            "Physical Violation: the rows were not sealed. {refusal}"
        ))
    })?;
    // The head names the two files these bytes would make, so a reader can
    // write both and give them to the program in `genesis_seal`. The head is
    // the same whichever form the answer takes.
    assembled.head.rows_sha256 = Some(digest_hex(&assembled.rows_text()));
    assembled.head.frames_sha256 = assembled.frames_bytes().map(|bytes| digest_hex(&bytes));
    match form {
        Form::Full => answer(&assembled, replaced),
        Form::Summary | Form::States => {
            let paths = varied_paths(request.value());
            let mut summary = Vec::with_capacity(results.len());
            for (index, result) in results.iter().enumerate() {
                let set = set_of(&paths, &assembled.rows[index].request);
                summary.push(summary_row(index as u64, result, set, form == Form::States));
            }
            let mut body = Map::new();
            body.insert("head".to_string(), head_value(&assembled.head.wire())?);
            body.insert("summary".to_string(), Value::Array(summary));
            body.insert("about".to_string(), Value::String(SUMMARY_ABOUT.to_string()));
            note_replaced(&mut body, replaced);
            encoded(&Value::Object(body))
        }
    }
}

/// Say in the answer which base values an axis or a draw replaced, when one did.
pub(crate) fn note_replaced(body: &mut Map<String, Value>, replaced: &[String]) {
    if !replaced.is_empty() {
        body.insert(
            BASE_REPLACED.to_string(),
            Value::Array(replaced.iter().map(|path| Value::String(path.clone())).collect()),
        );
    }
}

/// `head`, `rows` and `frames_hex`, in that order, as compact JSON text.
fn answer(assembled: &Assembled, replaced: &[String]) -> Result<String, DoorError> {
    let head = head_value(&assembled.head.wire())?;
    let rows: Vec<Value> = assembled.rows.iter().map(Row::to_value).collect();
    let frames_hex = match assembled.frames_bytes() {
        Some(bytes) => Value::String(hex::encode(bytes)),
        None => Value::Null,
    };
    let mut body = Map::new();
    body.insert("head".to_string(), head);
    body.insert("rows".to_string(), Value::Array(rows));
    body.insert("frames_hex".to_string(), frames_hex);
    note_replaced(&mut body, replaced);
    encoded(&Value::Object(body))
}

fn head_value(wire: &str) -> Result<Value, DoorError> {
    serde_json::from_str::<Value>(wire).map_err(|error| {
        DoorError::Internal(format!(
            "Physical Violation: the head could not be read back. {error}"
        ))
    })
}

/// A test makes row `index` slow, or makes it panic.
#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) enum RowHook {
    Slow(u64, std::time::Duration),
    Panic(u64),
}

/// The rows that wait for their turn, and the workers' view of the job.
struct Queue {
    /// The next row no worker has taken.
    next_claim: u64,
    /// The rows the writer has taken. Rows below this are done.
    taken: u64,
    /// Finished rows that wait for the rows before them.
    ready: BTreeMap<u64, PreparedRow>,
    /// The first fault. Every worker stops once it is set.
    failed: Option<DoorError>,
}

struct Job<'a> {
    request: &'a DatasetRequest,
    queue: Mutex<Queue>,
    changed: Condvar,
    /// A worker takes no row at or past `taken + window`, so no more than
    /// `window` rows are running or waiting at any time.
    window: u64,
}

impl Job<'_> {
    fn fail(&self, error: DoorError) {
        let mut queue = self.queue.lock().expect("the queue is not poisoned");
        queue.failed.get_or_insert(error);
        self.changed.notify_all();
    }

    /// The next row for a worker, or `None` when there is none to take or
    /// the job failed. Waits while the window is full.
    fn claim(&self) -> Option<u64> {
        let rows = self.request.totals().rows;
        let mut queue = self.queue.lock().expect("the queue is not poisoned");
        loop {
            if queue.failed.is_some() || queue.next_claim >= rows {
                return None;
            }
            if queue.next_claim < queue.taken + self.window {
                let index = queue.next_claim;
                queue.next_claim += 1;
                return Some(index);
            }
            queue = self.changed.wait(queue).expect("the queue is not poisoned");
        }
    }

    fn finish_row(&self, index: u64, result: PreparedRow, state: &AppState) {
        let mut queue = self.queue.lock().expect("the queue is not poisoned");
        queue.ready.insert(index, result);
        #[cfg(test)]
        state
            .window_high_water
            .fetch_max(queue.ready.len(), std::sync::atomic::Ordering::SeqCst);
        #[cfg(not(test))]
        let _ = state;
        self.changed.notify_all();
    }

    /// Row `index`, when it is finished. Waits for it. The error is the
    /// job's first fault.
    fn take(&self, index: u64) -> Result<PreparedRow, DoorError> {
        let mut queue = self.queue.lock().expect("the queue is not poisoned");
        loop {
            if let Some(error) = queue.failed.take() {
                // Keep the fault set for the workers, and report it once.
                queue.failed = Some(DoorError::Internal(error.message().to_string()));
                return Err(error);
            }
            if let Some(result) = queue.ready.remove(&index) {
                queue.taken = index + 1;
                self.changed.notify_all();
                return Ok(result);
            }
            queue = self.changed.wait(queue).expect("the queue is not poisoned");
        }
    }
}

fn worker(job: &Job, state: &AppState) {
    while let Some(index) = job.claim() {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            #[cfg(test)]
            match state.row_hook {
                Some(RowHook::Slow(row, pause)) if row == index => std::thread::sleep(pause),
                Some(RowHook::Panic(row)) if row == index => panic!("a row's run panicked"),
                _ => {}
            }
            // The run, the check of the row and the building of its line and
            // leaf all happen here, on the worker. The consumer only orders.
            let result = run_row(job.request, index)?;
            prepare_row(job.request, index, &result).map_err(not_sealed)
        }));
        match outcome {
            Ok(Ok(result)) => job.finish_row(index, result, state),
            Ok(Err(error)) => job.fail(error),
            Err(_) => job.fail(DoorError::Internal(
                "Physical Violation: a run stopped by a fault. The dataset was not written."
                    .to_string(),
            )),
        }
    }
}

/// The door's sentence for a row the crate refuses to seal.
fn not_sealed(refusal: DatasetRefusal) -> DoorError {
    DoorError::Internal(format!(
        "Physical Violation: the rows were not sealed. {refusal}"
    ))
}

/// Run the rows on `workers` threads, check each on the worker that ran it,
/// and give each prepared row to `consume` in plan order, whatever order the runs finish in. `consume` runs on the
/// calling thread alone. A fault anywhere stops the job and is returned.
fn run_ordered(
    state: &AppState,
    request: &DatasetRequest,
    workers: usize,
    mut consume: impl FnMut(PreparedRow) -> Result<(), DoorError>,
) -> Result<(), DoorError> {
    let workers = workers.max(1);
    let job = Job {
        request,
        queue: Mutex::new(Queue {
            next_claim: 0,
            taken: 0,
            ready: BTreeMap::new(),
            failed: None,
        }),
        changed: Condvar::new(),
        window: WINDOW_PER_WORKER * workers as u64,
    };
    let rows = request.totals().rows;
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| worker(&job, state));
        }
        // A panic in `consume` must not leave the workers waiting for a row
        // that will not be taken.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for index in 0..rows {
                consume(job.take(index)?)?;
            }
            Ok(())
        }))
        .unwrap_or_else(|_| {
            Err(DoorError::Internal(
                "Physical Violation: the rows were not sealed. The dataset was not written."
                    .to_string(),
            ))
        });
        if let Err(error) = &outcome {
            job.fail(DoorError::Internal(error.message().to_string()));
        }
        outcome
    })
}

/// The rows a file job may hold, running or finished, for each worker.
const WINDOW_PER_WORKER: u64 = 2;

/// A folder made for one job inside the dataset directory. It is removed
/// when it goes out of scope, unless the job keeps it.
struct JobFolder {
    path: PathBuf,
    armed: bool,
}

static JOB_FOLDERS: AtomicU64 = AtomicU64::new(0);

impl JobFolder {
    /// A new folder with a name no other job, and no other gateway on this
    /// directory, uses. Its name is not 64 hex digits, so it is never taken
    /// for a stored dataset.
    fn create(directory: &Path) -> Result<JobFolder, DoorError> {
        let name = format!(
            ".job_{}_{}",
            std::process::id(),
            JOB_FOLDERS.fetch_add(1, Ordering::SeqCst)
        );
        let path = directory.join(name);
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .map_err(|error| not_written(&error))?;
        }
        Ok(JobFolder { path, armed: true })
    }

    fn keep(mut self) {
        self.armed = false;
    }
}

impl Drop for JobFolder {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

fn not_written(error: &std::io::Error) -> DoorError {
    DoorError::Internal(format!(
        "Physical Violation: the dataset files could not be written. {error}."
    ))
}

/// A new file only the account that runs the gateway may read or write.
fn private_file(path: &Path, readable: bool) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(readable)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

const HEAD_FILE: &str = "head.json";
const ROWS_FILE: &str = "rows.jsonl";
const FRAMES_FILE: &str = "frames.bin";

/// Run a plan, write its files in a folder of their own, and name that
/// folder by the dataset seal. The answer is `head` and `files`, with no
/// path.
fn run_to_files(
    state: &AppState,
    request: &DatasetRequest,
    directory: &Path,
    workers: usize,
    replaced: &[String],
) -> Result<String, DoorError> {
    let folder = JobFolder::create(directory)?;
    let rows_out = private_file(&folder.path.join(ROWS_FILE), false)
        .map(|file| std::io::BufWriter::with_capacity(1 << 20, file))
        .map_err(|error| not_written(&error))?;
    let frames_out = match request.store() {
        Store::Frames => Some(
            private_file(&folder.path.join(FRAMES_FILE), true).map_err(|error| not_written(&error))?,
        ),
        Store::None => None,
    };
    let mut writer = DatasetWriter::new(request, crate::ENGINE_VERSION, rows_out, frames_out)
        .map_err(not_sealed)?;
    run_ordered(state, request, workers, |row| {
        writer.push_prepared(row).map_err(not_sealed)
    })?;
    let written = writer.finish().map_err(not_sealed)?;
    let head = written.head;
    let head_text = head.wire();
    let has_frames = written.frames.is_some();
    drop(written.rows);
    drop(written.frames);
    {
        use std::io::Write;
        private_file(&folder.path.join(HEAD_FILE), false)
            .and_then(|mut file| file.write_all(head_text.as_bytes()))
            .map_err(|error| not_written(&error))?;
    }

    let name = head.dataset_seal.clone();
    let final_path = directory.join(&name);
    // The head the answer carries: this job's, or, for a plan run before, the
    // one the folder holds, as it was first stored.
    let mut answer_head = head_text.clone();
    if std::fs::rename(&folder.path, &final_path).is_ok() {
        folder.keep();
    } else if std::fs::symlink_metadata(&final_path)
        .map(|found| found.is_dir() || found.file_type().is_symlink())
        .unwrap_or(false)
    {
        // The same plan was run before. Its folder must be this dataset's, and
        // its files must be the ones its head names, before it is answered from.
        let stored_text = confirm_stored_folder(&name, &final_path, &head, &folder.path.join(ROWS_FILE))?;
        answer_head = stored_text;
    } else {
        return Err(DoorError::Internal(format!(
            "Physical Violation: the folder {name} could not be made."
        )));
    }
    let mut files = Map::new();
    files.insert("folder".to_string(), Value::String(name));
    files.insert("head".to_string(), Value::String(HEAD_FILE.to_string()));
    files.insert("rows".to_string(), Value::String(ROWS_FILE.to_string()));
    files.insert(
        "frames".to_string(),
        if has_frames {
            Value::String(FRAMES_FILE.to_string())
        } else {
            Value::Null
        },
    );
    let mut body = Map::new();
    body.insert("head".to_string(), head_value(&answer_head)?);
    body.insert("files".to_string(), Value::Object(files));
    note_replaced(&mut body, replaced);
    encoded(&Value::Object(body))
}

/// The most bytes a stored head can hold: the dataset request as it was
/// received, which came through a door that reads no more than the line
/// limit and which the head writes out again in at most four times its
/// length (a number written `1e15` is written out as eighteen characters),
/// and the head's thirteen short fields.
pub(crate) const MAX_STORED_HEAD_BYTES: u64 = 4 * crate::guard::LINE_LIMIT as u64 + 65_536;

/// What one row holds beyond its request: the state, the seals, the leaf and
/// the refusal's sentence. The largest on any card is under 2,300 bytes.
const ROW_BEYOND_ITS_REQUEST_BYTES: u64 = 8_192;

/// The most bytes the rows file of a stored dataset can hold, from its head
/// alone. A row is its request and the rest. Every row's request is made of
/// the head's request, written out again in at most four times its length.
/// In a list plan each run is written once, so all the requests together are
/// no longer than that; in a grid or a sample the base is written in every
/// row, so each row's request may be.
pub(crate) fn most_rows_bytes(head: &genesis_seal::dataset::Head, head_bytes: u64) -> u64 {
    let rows = head.counts.rows;
    let requests = head_bytes.saturating_mul(4).saturating_add(1_024);
    let rest = rows.saturating_mul(ROW_BEYOND_ITS_REQUEST_BYTES);
    if head.request["plan"]["kind"] == "list" {
        requests.saturating_add(rows.saturating_mul(256)).saturating_add(rest)
    } else {
        rows.saturating_mul(requests).saturating_add(rest)
    }
}

/// The most bytes of a stored frame file that are read, from its head alone:
/// twice its true size, which is the 64-byte header and 64 bytes for each
/// frame the head counts. A file of a wrong length inside that is read and
/// named for what it is, as before; a file past it is not read.
pub(crate) fn most_frames_bytes(frames: u64) -> u64 {
    frames.saturating_add(1).saturating_mul(128)
}

/// True when a stored head is the head of the dataset `head` seals: the same
/// engine and counts, plan hash, row tree and dataset seal, the same frame
/// file, and a request of the same sealed form. The hash of the rows file is
/// not part of it: a row keeps its request's numbers as they were written,
/// so the rows of one dataset sent with `0.0` and sent with `0` are the same
/// values in two texts. The caller holds the rows to each other value for
/// value when their hashes differ.
fn seals_the_same_dataset(stored: &genesis_seal::dataset::Head, head: &genesis_seal::dataset::Head) -> bool {
    use genesis_seal::sealed_form::sealed_form;
    stored.counts == head.counts
        && stored.plan_sha256 == head.plan_sha256
        && stored.merkle_root == head.merkle_root
        && stored.dataset_seal == head.dataset_seal
        && stored.frames_sha256 == head.frames_sha256
        && stored.rows_sha256.is_some() == head.rows_sha256.is_some()
        && sealed_form(&stored.request) == sealed_form(&head.request)
}

/// True when the stored rows file and the rows file this job wrote hold the
/// same rows, value for value: the same count of lines, and each stored line
/// the same as this job's in its sealed form, where a number is its 64 bits
/// and the order of keys does not count. A decimal, a sentence or a count
/// that differs in value is a difference. The stored file is read a line at a
/// time and for no more than the size it had when it was opened.
fn rows_hold_the_same_values(stored: genesis_seal::stored_file::RegularFile, mine: &Path) -> bool {
    use genesis_seal::sealed_form::sealed_form;
    use std::io::BufRead;
    let Ok(mine) = std::fs::File::open(mine) else {
        return false;
    };
    let mut stored = std::io::BufReader::with_capacity(1 << 20, stored.reader());
    let mut mine = std::io::BufReader::with_capacity(1 << 20, mine);
    let (mut theirs, mut ours) = (Vec::new(), Vec::new());
    loop {
        theirs.clear();
        ours.clear();
        let (Ok(read_theirs), Ok(read_ours)) = (stored.read_until(b'\n', &mut theirs), mine.read_until(b'\n', &mut ours)) else {
            return false;
        };
        if read_theirs == 0 || read_ours == 0 {
            return read_theirs == 0 && read_ours == 0;
        }
        if theirs == ours {
            continue;
        }
        // The stored line is read with the scan every record goes through, so
        // a repeated key or a number that reads as zero is not taken for a row.
        let (Ok(theirs), Ok(ours)) = (genesis_seal::json_text::read(&theirs), serde_json::from_slice::<Value>(&ours)) else {
            return false;
        };
        if sealed_form(&theirs) != sealed_form(&ours) {
            return false;
        }
    }
}

/// The folder of a plan that was run before, held to this job. Its head must
/// be this dataset's: the bytes this job wrote, or a head with a receipt that
/// matches its bytes that seals the same dataset, so a request typed with its
/// keys in another order or its numbers spelled another way is the same
/// dataset. Then the rows file and the frame file must be no larger than the
/// head's counts allow and, by their hash read as a stream, the files the
/// stored head names. Where the stored rows are not this job's rows byte for
/// byte, they must be this job's rows value for value. The answer is the
/// stored head's text, as it was stored.
fn confirm_stored_folder(
    name: &str,
    folder: &Path,
    head: &genesis_seal::dataset::Head,
    rows_of_this_job: &Path,
) -> Result<String, DoorError> {
    use genesis_seal::receipt::{ReceiptCheck, check_receipt};
    use genesis_seal::stored_file::{StoredFileRefusal, open_regular, require_plain_folder};
    let not_this = || {
        DoorError::Internal(format!(
            "Physical Violation: the folder {name} already holds a head that is not this dataset's. Nothing was overwritten."
        ))
    };
    require_plain_folder(folder).map_err(|refusal| {
        DoorError::Internal(format!(
            "Physical Violation: the folder {name} already exists and {}. Nothing was overwritten.",
            refusal.what()
        ))
    })?;
    let head_not_read = |what: String| {
        DoorError::Internal(format!(
            "Physical Violation: the folder {name} already exists and its head was not read. {what}."
        ))
    };
    let stored_file = open_regular(&folder.join(HEAD_FILE)).map_err(|refusal| head_not_read(refusal.what()))?;
    if stored_file.size() > MAX_STORED_HEAD_BYTES {
        return Err(head_not_read(format!(
            "is {} bytes, more than the {MAX_STORED_HEAD_BYTES} bytes a head can hold",
            stored_file.size()
        )));
    }
    let stored_bytes = stored_file
        .read_all()
        .map_err(|error| head_not_read(StoredFileRefusal::NotRead(folder.join(HEAD_FILE), error).what()))?;
    let stored = genesis_seal::dataset::Head::read(&stored_bytes).map_err(|_| not_this())?;
    let same_bytes = stored_bytes == head.wire().as_bytes();
    if !same_bytes && !(check_receipt(&stored_bytes) == ReceiptCheck::Matches && seals_the_same_dataset(&stored, head)) {
        return Err(not_this());
    }
    let differs = |file: &str| {
        DoorError::Internal(format!(
            "Physical Violation: the folder {name} holds the head of this dataset, but its {file} is not the file the head names. Nothing was overwritten."
        ))
    };
    // A file larger than the head's counts allow is not the file the head
    // names, and is not read to find that out.
    let hash_of = |file: &str, most: u64| -> Result<Option<String>, DoorError> {
        match open_regular(&folder.join(file)) {
            Ok(opened) if opened.size() > most => Err(differs(file)),
            Ok(opened) => opened.sha256_hex().map(Some).map_err(|_| differs(file)),
            Err(refusal) if refusal.is_missing() => Ok(None),
            Err(_) => Err(differs(file)),
        }
    };
    let most_rows = most_rows_bytes(&stored, stored_bytes.len() as u64);
    if hash_of(ROWS_FILE, most_rows)? != stored.rows_sha256 || stored.rows_sha256.is_none() {
        return Err(differs(ROWS_FILE));
    }
    if hash_of(FRAMES_FILE, most_frames_bytes(stored.counts.frames))? != stored.frames_sha256 {
        return Err(differs(FRAMES_FILE));
    }
    // The same dataset sent with a number spelled another way: the stored
    // rows are another text of the same rows. They are held to this job's
    // rows value for value before the folder is answered from.
    if stored.rows_sha256 != head.rows_sha256 {
        let same_values = match open_regular(&folder.join(ROWS_FILE)) {
            Ok(opened) if opened.size() <= most_rows => rows_hold_the_same_values(opened, rows_of_this_job),
            _ => false,
        };
        if !same_values {
            return Err(not_this());
        }
    }
    String::from_utf8(stored_bytes).map_err(|_| not_this())
}

/// Check a dataset. The request is the `run_dataset` request with the seal
/// the caller was given, which runs the plan again under the permits and the
/// limits of `run_dataset`; or the three parts a `run_dataset` answer
/// carries, or the seal of a dataset the gateway stored, which take no
/// physics permit and run no physics.
pub(crate) async fn verify(state: &AppState, body: &Value, text: Option<&str>) -> Result<String, DoorError> {
    let object = body.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    if let Some((request, given)) =
        crate::verify_by_request::read(object, "dataset_seal", "'dataset_seal' alone, or 'head', 'rows' and 'frames_hex',")?
    {
        let of_solves = request.get("solver").is_some();
        let (recomputed, version) = recompute_seal(state, request.clone(), given).await?;
        let compared = if of_solves { DATASET_OF_SOLVES_BY_REQUEST_COMPARED } else { DATASET_BY_REQUEST_COMPARED };
        return crate::verify_by_request::answer("dataset_seal", "run_dataset", given, &recomputed, None, compared, crate::verify_by_request::DATASET_TEXT_NOT_COMPARED, version);
    }
    if object.contains_key("dataset_seal") {
        if object.len() != 1 {
            return Err(DoorError::Unprocessable(
                "Physical Violation: send 'request' and 'dataset_seal', or 'dataset_seal' alone, or 'head', 'rows' and 'frames_hex', and no mixture of them."
                    .to_string(),
            ));
        }
        let directory = state
            .dataset_dir
            .clone()
            .ok_or_else(|| DoorError::Unprocessable(no_directory_to_verify()))?;
        let seal = match object.get("dataset_seal") {
            Some(Value::String(text))
                if text.len() == 64
                    && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) =>
            {
                text.clone()
            }
            _ => return Err(wrong("dataset_seal", "64 lowercase hex digits")),
        };
        let one_at_a_time = Arc::clone(&state.stored_verifications)
            .try_acquire_owned()
            .map_err(|_| DoorError::TooMany(BUSY.to_string()))?;
        let folder = Arc::clone(&directory);
        let named = seal.clone();
        let sealed = tokio::task::spawn_blocking(move || verify_stored(&folder, &named))
            .await
            .unwrap_or_else(|_| {
                Err(DoorError::Internal(
                    "Physical Violation: the verification stopped before it finished.".to_string(),
                ))
            })?;
        // Every seal holds: the stored head's plan is run again, into a folder
        // of its own, and the stored files are held to its files byte for byte.
        let answer = rerun_stored(state, directory, seal, sealed).await;
        drop(one_at_a_time);
        return answer;
    }
    verify_parts(state, body, text).await
}

/// A stored dataset whose seals hold, held to the dataset the engine makes
/// for its head's plan: the plan is run again under `run_dataset`'s permits
/// and limits, into a folder of its own that is removed after, and the stored
/// head, rows and frames are compared with that run's byte for byte. The
/// answer of the seals stands when a seal does not hold.
async fn rerun_stored(state: &AppState, directory: Arc<PathBuf>, seal: String, sealed: String) -> Result<String, DoorError> {
    let mut answer: Value = serde_json::from_str(&sealed)
        .map_err(|error| DoorError::Internal(format!("Physical Violation: the verification could not be read again. {error}")))?;
    if answer["holds"] != Value::Bool(true) {
        return Ok(sealed);
    }
    let stored = directory.join(&seal);
    let head = std::fs::read(stored.join(HEAD_FILE)).map_err(|error| not_written(&error))?;
    let head_value: Value = serde_json::from_slice(&head)
        .map_err(|error| DoorError::Internal(format!("Physical Violation: the stored head could not be read again. {error}")))?;
    let normalized = normalize(head_value["request"].clone())?;
    let (differs, values_hold) = match admit(state, normalized.request, true) {
        Err(DoorError::Unprocessable(sentence) | DoorError::NotFound(sentence)) => (Some((
            None,
            crate::rebuild::Difference::new(
                "head.request",
                format!("The engine runs no dataset for this plan: it turns it away before running. {sentence}"),
            ),
        )), false),
        Err(other) => return Err(other),
        Ok((request, _)) => {
            let workers = take_workers(state, &request)?;
            let state = state.clone();
            tokio::task::spawn_blocking(move || {
                let worker_count = workers.len();
                let found = rerun_and_compare(&state, &request, &directory, &stored, worker_count);
                drop(workers);
                found
            })
            .await
            .unwrap_or_else(|_| {
                Err(DoorError::Internal(
                    "Physical Violation: the dataset job stopped before it finished. Nothing was written.".to_string(),
                ))
            })?
        }
    };
    answer["values_hold"] = Value::Bool(values_hold);
    if let Some((row, difference)) = differs {
        answer["holds"] = Value::Bool(false);
        answer["cause"] = Value::String(crate::rebuild::CAUSE_ANSWER_TEXT.to_string());
        answer["row"] = row.map(Value::from).unwrap_or(Value::Null);
        answer["merkle_root"] = Value::Null;
        answer["dataset_seal"] = Value::Null;
        answer["differs_at"] = Value::String(difference.at);
        answer["difference"] = Value::String(difference.says);
    }
    encoded(&answer)
}

/// The first byte where two files differ, or where one ends before the
/// other: `None` when they are the same bytes.
fn first_byte_apart(stored: &Path, engine: &Path) -> std::io::Result<Option<u64>> {
    use std::io::Read;
    let open = |path: &Path| -> std::io::Result<Option<std::io::BufReader<std::fs::File>>> {
        match std::fs::File::open(path) {
            Ok(file) => Ok(Some(std::io::BufReader::with_capacity(1 << 20, file))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    };
    let (mut left, mut right) = match (open(stored)?, open(engine)?) {
        (None, None) => return Ok(None),
        (Some(_), None) | (None, Some(_)) => return Ok(Some(0)),
        (Some(left), Some(right)) => (left, right),
    };
    let mut at = 0_u64;
    let (mut a, mut b) = (vec![0_u8; 1 << 16], vec![0_u8; 1 << 16]);
    loop {
        let read_a = left.read(&mut a)?;
        let mut read_b = 0;
        while read_b < read_a {
            let more = right.read(&mut b[read_b..read_a])?;
            if more == 0 {
                break;
            }
            read_b += more;
        }
        if read_a == 0 {
            return Ok((right.read(&mut b[..1])? > 0).then_some(at));
        }
        if let Some(place) = a[..read_b].iter().zip(&b[..read_b]).position(|(x, y)| x != y) {
            return Ok(Some(at + place as u64));
        }
        if read_b < read_a {
            return Ok(Some(at + read_b as u64));
        }
        at += read_a as u64;
    }
}

/// The count of line breaks in a file before a byte: the row a byte of a
/// rows file belongs to.
fn row_of_byte(path: &Path, byte: u64) -> Option<u64> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file).take(byte);
    let mut block = vec![0_u8; 1 << 16];
    let mut rows = 0_u64;
    loop {
        let read = reader.read(&mut block).ok()?;
        if read == 0 {
            return Some(rows);
        }
        rows += block[..read].iter().filter(|byte| **byte == b'\n').count() as u64;
    }
}

/// Run a stored dataset's plan again into a folder of its own in the same
/// directory, compare the stored folder's files with it byte for byte, and
/// remove it: the first file and place that differ, with the row of a rows
/// file, or `None` when the stored dataset is, every byte, the engine's.
fn rerun_and_compare(
    state: &AppState,
    request: &DatasetRequest,
    directory: &Path,
    stored: &Path,
    workers: usize,
) -> Result<(Option<(Option<u64>, crate::rebuild::Difference)>, bool), DoorError> {
    use crate::rebuild::Difference;
    let folder = JobFolder::create(directory)?;
    let rows_out = private_file(&folder.path.join(ROWS_FILE), false)
        .map(|file| std::io::BufWriter::with_capacity(1 << 20, file))
        .map_err(|error| not_written(&error))?;
    let frames_out = match request.store() {
        Store::Frames => Some(private_file(&folder.path.join(FRAMES_FILE), true).map_err(|error| not_written(&error))?),
        Store::None => None,
    };
    let mut writer = DatasetWriter::new(request, crate::ENGINE_VERSION, rows_out, frames_out).map_err(not_sealed)?;
    run_ordered(state, request, workers, |row| writer.push_prepared(row).map_err(not_sealed))?;
    let written = writer.finish().map_err(not_sealed)?;
    let head_text = written.head.wire();
    drop(written.rows);
    drop(written.frames);
    let apart = |file: &str| first_byte_apart(&stored.join(file), &folder.path.join(file)).map_err(|error| not_written(&error));
    let rows_apart = apart(ROWS_FILE)?;
    let frames_apart = apart(FRAMES_FILE)?;
    let stored_head = std::fs::read(stored.join(HEAD_FILE)).map_err(|error| not_written(&error))?;
    let head_sent: Value = serde_json::from_slice(&stored_head).unwrap_or(Value::Null);
    let head_compared = crate::rebuild::compare_under("head", &head_text, &head_sent, &String::from_utf8_lossy(&stored_head))?;
    // Every value holds when each stored row is the engine's row value for
    // value, the frames are its bytes, and the head is its head value for
    // value, whatever the spelling of a number.
    let values_hold = (rows_apart.is_none() || rows_values_hold(&stored.join(ROWS_FILE), &folder.path.join(ROWS_FILE)))
        && frames_apart.is_none()
        && head_compared.values_hold;
    if let Some(byte) = rows_apart {
        let row = row_of_byte(&stored.join(ROWS_FILE), byte);
        let at = row.map_or_else(|| "rows".to_string(), |row| format!("rows[{row}]"));
        // The row is named at its first field that differs, when both files
        // hold it as a line of JSON.
        let line_of = |path: &Path| -> Option<String> {
            use std::io::BufRead;
            let file = std::fs::File::open(path).ok()?;
            std::io::BufReader::new(file).lines().nth(row? as usize)?.ok()
        };
        let named = match (line_of(&stored.join(ROWS_FILE)), line_of(&folder.path.join(ROWS_FILE))) {
            (Some(theirs), Some(ours)) => serde_json::from_str::<Value>(&theirs)
                .ok()
                .and_then(|sent| crate::rebuild::compare_under(&at, &ours, &sent, &theirs).ok().and_then(|found| found.difference)),
            _ => None,
        };
        let difference = named.unwrap_or_else(|| Difference::new(
            at.clone(),
            format!("The stored {ROWS_FILE} is not the rows file the engine writes for this plan, run again: it first differs at byte {byte}, in {at}. Its seals hold, so it is consistent with itself, and it is not the engine's text."),
        ));
        return Ok((Some((row, difference)), values_hold));
    }
    if let Some(byte) = frames_apart {
        return Ok((Some((None, Difference::new(
            "frames",
            format!("The stored {FRAMES_FILE} is not the frame file the engine writes for this plan, run again: it first differs at byte {byte}."),
        ))), values_hold));
    }
    if stored_head != head_text.as_bytes() {
        let says = head_compared
            .difference
            .unwrap_or_else(|| Difference::new("head", "The stored head is not the head the engine writes for this plan, run again."));
        return Ok((Some((None, says)), values_hold));
    }
    Ok((None, values_hold))
}

/// Whether every line of a stored rows file is the engine's row value for
/// value, in the engine's key order, whatever the spelling of a number.
fn rows_values_hold(stored: &Path, engine: &Path) -> bool {
    use std::io::BufRead;
    let (Ok(theirs), Ok(ours)) = (std::fs::File::open(stored), std::fs::File::open(engine)) else {
        return false;
    };
    let mut theirs = std::io::BufReader::with_capacity(1 << 20, theirs).lines();
    let mut ours = std::io::BufReader::with_capacity(1 << 20, ours).lines();
    loop {
        match (theirs.next(), ours.next()) {
            (None, None) => return true,
            (Some(Ok(theirs)), Some(Ok(ours))) => {
                if theirs == ours {
                    continue;
                }
                let (Ok(theirs), Ok(ours)) = (genesis_seal::json_text::read(theirs.as_bytes()), serde_json::from_str::<Value>(&ours)) else {
                    return false;
                };
                if crate::rebuild::first_difference(&ours, &theirs).is_some() {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

/// Check the folder the gateway stored under `seal`.
pub(crate) fn verify_stored(directory: &Path, seal: &str) -> Result<String, DoorError> {
    use genesis_seal::stored_file::{open_regular, require_plain_folder};
    let folder = directory.join(seal);
    // The folder and its three files must be what they say: a folder, and
    // regular files, none of them a link, a pipe or a device. Nothing else is
    // opened, and no file is read for more bytes than its size at open.
    let unread = |name: &str, what: String| {
        DoorError::Internal(format!(
            "Physical Violation: the stored file {name} was not read. It {what}."
        ))
    };
    match require_plain_folder(&folder) {
        Ok(()) => {}
        Err(refusal) if refusal.is_missing() => {
            return Err(DoorError::NotFound(not_stored()));
        }
        Err(refusal) => {
            return Err(DoorError::Internal(format!(
                "Physical Violation: the stored dataset {seal} was not opened. It {}.",
                refusal.what()
            )));
        }
    }
    let too_large = |name: &str, size: u64, most: u64, of_what: &str| {
        unread(name, format!("is {size} bytes, more than the {most} bytes {of_what}"))
    };
    let open = |name: &str| open_regular(&folder.join(name)).map_err(|refusal| unread(name, refusal.what()));
    let read = |name: &str, file: genesis_seal::stored_file::RegularFile| -> Result<Vec<u8>, DoorError> {
        file.read_all().map_err(|error| unread(name, format!("could not be read: {error}")))
    };
    // A stored file is an ordinary file, and an ordinary file can be given any
    // apparent size at almost no cost on disk. So no file is read past what it
    // can hold: the head by what a head can hold, and the rows and the frames
    // by what the head's own counts allow. The counts are held to what one job
    // may make, so a head cannot allow more than the door could have stored.
    let head_file = open(HEAD_FILE)?;
    if head_file.size() > MAX_STORED_HEAD_BYTES {
        return Err(too_large(HEAD_FILE, head_file.size(), MAX_STORED_HEAD_BYTES, "a head can hold"));
    }
    let head = read(HEAD_FILE, head_file)?;
    let stored = match genesis_seal::dataset::Head::read(&head) {
        Ok(stored) => stored,
        // A head that is not read is refused in the crate's own sentence, as
        // before, and no other file is opened.
        Err(_) => {
            let refused = check_dataset_stream(&head, &[], None::<std::io::Empty>)
                .map_err(|refusal| DoorError::Unprocessable(refusal.to_string()))?;
            return answer_of_report(&refused.report, &head, &[], None, false);
        }
    };
    let counts = &stored.counts;
    if counts.rows > MAX_DATASET_ROWS || counts.frames > MAX_DATASET_FRAMES {
        return Err(DoorError::Internal(format!(
            "Physical Violation: the stored head states {} rows and {} frames, more than the {MAX_DATASET_ROWS} rows or the {MAX_DATASET_FRAMES} frames one dataset may have. Its files were not read.",
            counts.rows, counts.frames
        )));
    }
    let rows_file = open(ROWS_FILE)?;
    let most_rows = most_rows_bytes(&stored, head.len() as u64);
    if rows_file.size() > most_rows {
        return Err(too_large(ROWS_FILE, rows_file.size(), most_rows, "the head's count of rows allows"));
    }
    let rows = read(ROWS_FILE, rows_file)?;
    // The frame file is read as a stream, one frame at a time, so a check
    // holds the head and the rows file and no more of the frames.
    let frames = match open_regular(&folder.join(FRAMES_FILE)) {
        Ok(file) if file.size() > most_frames_bytes(counts.frames) => {
            return Err(too_large(
                FRAMES_FILE,
                file.size(),
                most_frames_bytes(counts.frames),
                "the head's count of frames allows",
            ));
        }
        Ok(file) => Some(std::io::BufReader::with_capacity(1 << 20, file.reader())),
        Err(refusal) if refusal.is_missing() => None,
        Err(refusal) => return Err(unread(FRAMES_FILE, refusal.what())),
    };
    let checked = check_dataset_stream(&head, &rows, frames)
        .map_err(|refusal| DoorError::Unprocessable(refusal.to_string()))?;
    answer_of_report(&checked.report, &head, &rows, None, false)
}

/// The keys of a dataset check by parts. All three are required together.
pub(crate) const PARTS_KEYS: &[&str] = &["head", "rows", "frames_hex"];

/// The long form of a dataset: the head, rows and frames of a full answer.
/// The rows, the root and the seal are recomputed with no physics; when every
/// seal holds, the head's plan is run again under `run_dataset`'s permits and
/// limits, and the engine's full answer to it is compared with the answer
/// sent, value by value and byte for byte.
async fn verify_parts(state: &AppState, body: &Value, text: Option<&str>) -> Result<String, DoorError> {
    let object = body.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    for key in object.keys() {
        if !PARTS_KEYS.contains(&key.as_str()) {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{key}' is not a field of this request."
            )));
        }
    }
    let head = required(object, "head")?;
    if !head.is_object() {
        return Err(wrong("head", "a JSON object"));
    }
    if head["request"].get("solver").is_some() {
        return Err(DoorError::Unprocessable(crate::solve_plan::VERIFY_SOLVES_BY_REQUEST.to_string()));
    }
    let rows = required(object, "rows")?
        .as_array()
        .ok_or_else(|| wrong("rows", "a JSON array"))?;
    let frames = match required(object, "frames_hex")? {
        Value::Null => None,
        Value::String(text) => Some(decode_lower_hex(text).ok_or_else(|| {
            DoorError::Unprocessable(
                "Physical Violation: 'frames_hex' must be lowercase hex digits, two for each byte."
                    .to_string(),
            )
        })?),
        _ => return Err(wrong("frames_hex", "a string of lowercase hex digits or null")),
    };

    let head_bytes = encoded(head)?.into_bytes();
    let mut rows_bytes = Vec::new();
    for row in rows {
        rows_bytes.extend_from_slice(encoded(row)?.as_bytes());
        rows_bytes.push(b'\n');
    }
    let checked = check_dataset(&head_bytes, &rows_bytes, frames.as_deref())
        .map_err(|refusal| DoorError::Unprocessable(refusal.to_string()))?;
    if !checked.holds || another_version_of(&head_bytes, &checked).is_some() {
        return answer_of_report(&checked, &head_bytes, &rows_bytes, None, false);
    }
    // Every seal holds, and a seal has no key: the plan is run again and the
    // engine's full answer to it compared with the answer sent.
    let request = head["request"].clone();
    let (differs, values_hold) = match rerun_full(state, request).await {
        Ok(engine_text) => {
            // The rows are compared first, so the first row that is not the
            // engine's is named before the head that seals it.
            let engine: Value = serde_json::from_str(&engine_text).map_err(|error| {
                DoorError::Internal(format!("Physical Violation: the engine's answer could not be read again. {error}"))
            })?;
            let in_rows = crate::rebuild::first_difference_under("rows", &engine["rows"], &body["rows"]);
            let sent_text = crate::rebuild::sent_text(body, text);
            let found = match in_rows {
                Some(difference) => crate::rebuild::Compared { difference: Some(difference), values_hold: false },
                None => crate::rebuild::compare(&engine_text, body, &sent_text)?,
            };
            (found.difference.map(|difference| (row_of_path(&difference.at), difference)), found.values_hold)
        }
        Err(DoorError::Unprocessable(sentence) | DoorError::NotFound(sentence)) => (Some((
            None,
            crate::rebuild::Difference::new(
                "head.request",
                format!("The engine runs no dataset for this plan: it turns it away before running. {sentence}"),
            ),
        )), false),
        Err(other) => return Err(other),
    };
    answer_of_report(&checked, &head_bytes, &rows_bytes, differs, values_hold)
}

/// The row a path of a dataset's answer names: `rows[3].final_state` is row 3.
fn row_of_path(path: &str) -> Option<u64> {
    let rest = path.strip_prefix("rows[")?;
    rest[..rest.find(']')?].parse().ok()
}

/// The full answer `run_dataset` gives a plan, run again under its permits
/// and limits: the head, every row whole and the frames, as JSON text.
/// Nothing is stored, whatever the plan's return mode.
async fn rerun_full(state: &AppState, request: Value) -> Result<String, DoorError> {
    let normalized = normalize(request)?;
    if normalized.is_solves() {
        return Err(DoorError::Unprocessable(crate::solve_plan::VERIFY_SOLVES_BY_REQUEST.to_string()));
    }
    let (request, _) = admit(state, normalized.request, false)?;
    let workers = take_workers(state, &request)?;
    #[cfg(test)]
    let hold = state.permit_hold;
    let replaced = normalized.replaced;
    let task = tokio::task::spawn_blocking(move || {
        let result = run_inline(&request, Form::Full, &replaced);
        #[cfg(test)]
        if !hold.is_zero() {
            std::thread::sleep(hold);
        }
        drop(workers);
        result
    });
    task.await.unwrap_or_else(|_| {
        Err(DoorError::Internal(
            "Physical Violation: the dataset job stopped before it finished. Nothing was written."
                .to_string(),
        ))
    })
}

#[cfg(test)]
pub(crate) fn report(head: &[u8], rows: &[u8], frames: Option<&[u8]>) -> Result<String, DoorError> {
    let report = check_dataset(head, rows, frames)
        .map_err(|refusal| DoorError::Unprocessable(refusal.to_string()))?;
    answer_of_report(&report, head, rows, None, false)
}

/// A dataset of another engine version, named: its head's `engine_version`
/// when it is not this engine's, and what that means, with what the seals
/// recomputed by this engine's rules say.
fn another_version_of(head: &[u8], report: &DatasetReport) -> Option<crate::rebuild::Difference> {
    let head: Value = serde_json::from_slice(head).ok()?;
    let version = head.get("engine_version")?.as_str()?;
    if version == crate::ENGINE_VERSION {
        return None;
    }
    let seals = match (report.holds, report.cause) {
        (true, _) => "hold".to_string(),
        (false, Some(cause)) => format!("do not hold, at {}", cause.as_str()),
        (false, None) => "do not hold".to_string(),
    };
    Some(crate::rebuild::Difference::new(
        "head.engine_version",
        format!(
            "The dataset names engine_version {version}, and this engine is {}. An engine writes only its own datasets, so the text of a dataset of {version} is not compared here and holds is false. Its seals, recomputed by this engine's rules, {seals}; from genesis_kernel_v0.10.0 a row's leaf also seals its reject_detail, and a dataset of an earlier version is verified by that version's rules, which the programs that confirm a proof with no engine choose by the version.",
            crate::ENGINE_VERSION
        ),
    ))
}

/// The answer of a dataset's verification: its seals' report, another engine
/// version named first, and `differs`, the first place where a dataset whose
/// seals hold is not the engine's dataset for its plan run again, with its
/// row.
fn answer_of_report(
    report: &DatasetReport,
    head: &[u8],
    rows: &[u8],
    differs: Option<(Option<u64>, crate::rebuild::Difference)>,
    values_hold: bool,
) -> Result<String, DoorError> {
    // A dataset of another engine version is named first: the leaf rule
    // moved in genesis_kernel_v0.10.0, so its seals may not hold here.
    let another = another_version_of(head, report);
    let differs = if report.holds && another.is_none() { differs } else { None };
    let holds = report.holds && another.is_none() && differs.is_none();
    // The root and the seal rebuilt from the rows are returned only when the
    // check holds: a rebuilt seal beside a seal that does not hold would be a
    // seal to write into the head and send back, and it shows only that the
    // rows are consistent with themselves.
    let (merkle_root, dataset_seal) = if holds {
        (report.merkle_root.clone(), report.dataset_seal.clone())
    } else {
        (None, None)
    };
    // What was done to the rows, in plain words, as genesis_seal
    // verify-dataset says it, when a seal does not hold.
    let fault = match (&another, report.holds) {
        (None, false) => genesis_seal::fault::fault_of(head, rows, report, MAX_DATASET_ROWS),
        _ => None,
    };
    // The row of the first break, as the seal crate names it; where it names
    // none (rows dropped or repeated), the first row the fault shows at; for
    // a text that differs, its row.
    let row = report
        .row
        .or(fault.as_ref().and_then(|fault| fault.row))
        .or(differs.as_ref().and_then(|(row, _)| *row));
    let cause = match (&another, &differs) {
        (Some(_), _) => Some(crate::rebuild::CAUSE_ENGINE_VERSION),
        (None, Some(_)) => Some(crate::rebuild::CAUSE_ANSWER_TEXT),
        (None, None) => report.cause.map(|cause| cause.as_str()),
    };
    let mut answer = json!({
        "holds": holds,
        "values_hold": report.holds && another.is_none() && values_hold,
        "cause": cause,
        "row": row,
        "broken_link": report.broken_link,
        "chains": report.chains.as_str(),
        "merkle_root": merkle_root,
        "dataset_seal": dataset_seal,
        "receipt": report.receipt.as_str(),
        "rows_file": report.rows_file.as_str(),
        "frames_file": report.frames_file.as_str(),
    });
    // A dataset that holds has the eleven keys above and no more. `fault` is
    // added when a seal does not hold; `differs_at` and `difference` when the
    // seals hold and the text is not the engine's, or the dataset names
    // another engine version.
    if let Some(fault) = fault {
        answer["fault"] = Value::String(fault.says);
    }
    if let Some(difference) = another.or(differs.map(|(_, difference)| difference)) {
        answer["differs_at"] = Value::String(difference.at);
        answer["difference"] = Value::String(difference.says);
    }
    encoded(&answer)
}

fn required<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a Value, DoorError> {
    object
        .get(key)
        .ok_or_else(|| DoorError::Unprocessable(format!("Missing required parameter: '{key}' (InputMissing).")))
}

fn wrong(field: &str, expected: &str) -> DoorError {
    DoorError::Unprocessable(format!("Physical Violation: '{field}' must be {expected}."))
}

fn encoded(value: &Value) -> Result<String, DoorError> {
    serde_json::to_string(value).map_err(|error| {
        DoorError::Internal(format!(
            "Physical Violation: the response could not be encoded. {error}"
        ))
    })
}

/// The bytes of lowercase hex digits, two for each byte.
fn decode_lower_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    hex::decode(text).ok()
}

/// One worked plan of each kind, each a whole request that runs as written,
/// for a card of the view: the cold plate when the view holds it, and the
/// first card of the view when it does not. A view with no card has none.
///
/// The cold plate's three show both kinds of path and a base that leaves out
/// what an axis or a draw sets. Another card's are built from the run its
/// own description gives as an example, varied in `dt_s`, the one path every
/// card has. A card whose step floor is its step ceiling steps by that one
/// step and by no other, and a card whose floor is above half its example's
/// step cannot take half of it, so the plans of either vary a load of the
/// run's first segment instead: half the example's value, and the value.
pub fn worked_plans_in(view: &View) -> Option<[(&'static str, Value); 3]> {
    let cards = view.cards();
    let card = cards
        .iter()
        .find(|item| (item.domain, item.card) == ("energy", "thermal"))
        .or_else(|| cards.first())?;
    let request = |plan: Value| {
        json!({ "domain": card.domain, "card": card.card, "plan": plan, "store": "none", "return": "inline" })
    };
    if (card.domain, card.card) == ("energy", "thermal") {
        let base = json!({
            "initial": {},
            "dt_s": 1.0,
            "segments": [{ "steps": 60, "load": { "ambient_k": 293.15 } }],
            "record_every": 60
        });
        return Some([
            (
                "list",
                request(json!({
                    "kind": "list",
                    "runs": [{
                        "initial": { "temperature_k": 293.15 },
                        "dt_s": 1.0,
                        "segments": [{ "steps": 60, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
                        "record_every": 60
                    }]
                })),
            ),
            (
                "grid",
                request(json!({
                    "kind": "grid",
                    "base": base,
                    "axes": [
                        { "path": "initial.temperature_k", "values": [283.15, 293.15] },
                        { "path": "segments[0].load.heat_power_w", "values": [0.0, 5.0, 10.0] }
                    ]
                })),
            ),
            (
                "sample",
                request(json!({
                    "kind": "sample",
                    "base": base,
                    "rows": 4,
                    "generator": "splitmix64_counter_v1",
                    "seed_hex": "0000000000000001",
                    "draws": [
                        { "path": "initial.temperature_k", "choice": [283.15, 293.15] },
                        { "path": "segments[0].load.heat_power_w", "uniform": { "low": 0.0, "high": 10.0 } }
                    ]
                })),
            ),
        ]);
    }
    let description = descriptor::describe_value_in(view, card.domain, card.card)?;
    let mut run = description["example_run_trajectory"].clone();
    let object = run.as_object_mut()?;
    object.remove("domain");
    object.remove("card");
    let dt_s = object.get("dt_s")?.as_f64()?;
    let mut base = run.clone();
    // The line-of-sight card steps by one sensor frame within half a
    // millisecond, so half its step is under its floor; its first load is the
    // 0-or-1 `measured`, which has no half. Its plans vary the count rate.
    let (path, value) = if card.floor_s == card.ceiling_s || 0.5 * dt_s < card.floor_s {
        let name = if card.card == "los_jwst_fine_guide" {
            "counts_per_s"
        } else {
            *card.load_names.first()?
        };
        let value = base["segments"][0]["load"].as_object_mut()?.remove(name)?.as_f64()?;
        (format!("segments[0].load.{name}"), value)
    } else {
        base.as_object_mut()?.remove("dt_s");
        ("dt_s".to_string(), dt_s)
    };
    let half = serde_json::from_str::<Value>(&crate::exact_f64_token(0.5 * value)).ok()?;
    let whole = serde_json::from_str::<Value>(&crate::exact_f64_token(value)).ok()?;
    // A draw is taken from the lower number to the higher, whatever the sign.
    let (low, high) = if value < 0.0 { (&whole, &half) } else { (&half, &whole) };
    Some([
        ("list", request(json!({ "kind": "list", "runs": [run] }))),
        (
            "grid",
            request(json!({
                "kind": "grid",
                "base": base,
                "axes": [{ "path": path, "values": [half, whole] }]
            })),
        ),
        (
            "sample",
            request(json!({
                "kind": "sample",
                "base": base,
                "rows": 2,
                "generator": "splitmix64_counter_v1",
                "seed_hex": "0000000000000001",
                "draws": [{ "path": path, "uniform": { "low": low, "high": high } }]
            })),
        ),
    ])
}

/// The most characters a run's answer, or a dataset row, holds beyond its own
/// request and its recorded states: the final state and the seals. Stated in
/// the descriptions of `run_trajectory` and `run_dataset`, and held by a
/// test on all thirty cards. The largest is the solar system, whose
/// state is thirteen bodies' positions and velocities and the two ledgers:
/// about 6,350 characters on its worked run. The line of sight of the
/// telescope follows with about 4,870, and the physical 2170 cell with about
/// 3,240.
pub(crate) const REST_OF_AN_ANSWER_CHARS: usize = 8_000;

/// What the `run_dataset` tool says of itself on every connection: what it
/// does, what a plan is, the forms of an answer, what a row can be and what
/// the seal shows. How a plan is written, how an answer is counted and the
/// plan of solves are said at the inputs they are about ([`plan_says_in`],
/// [`answer_says`], [`solves_say_in`]), which the tool list places there.
pub fn run_description() -> String {
    "Runs a declared plan of runs on one card, or of solves of one solver, and seals the rows under one root. A plan is a list of runs, a grid over a base run, or a seeded sample over a base run; a run is a run_trajectory request without domain and card. A plan of solves names solver in place of domain and card and is a list or a grid of solve requests. The answer is small by default: the head, which holds the request, the counts and the dataset_seal, and one summary row per run; answer \"states\" adds each final state in decimals and \"full\" gives every row whole, under the same dataset_seal. A row is completed, ended, refused or unadmitted: an ended or a refused row is a run sealed up to its last landed step, and an unadmitted row is a request turned away before computing, with no seal. The seal shows the dataset was not altered after it was sealed. It does not show the engine made it: a row's request sent to run_trajectory gives the run seal to compare. The input schema says how each input is written, with a worked plan of each kind.".to_string()
}

/// What the `plan` input of `run_dataset` says to an agent of this view: how
/// a path is written, what a varied value replaces, what cannot vary, which
/// rows a bound stops, and one worked plan of each kind.
pub(crate) fn plan_says_in(view: &View) -> String {
    let mut text = String::from(
        "A path names one number: initial.<field>, dt_s, or segments[0].load.<field>, with the segment's number in square brackets and [0], [1] or [2] after a 3-vector field for one of its components. steps and record_every are not paths and cannot vary: every row of a grid or a sample takes them from the base run. A value an axis or a draw sets replaces the base's value of that field, and the answer's base_replaced names each field replaced; a 3-vector the base sets whole is replaced only when the paths set all three of its components. Two paths may not set one number. Each row of a grid is held to the card's bounds alone, as each run of a list is: a value outside a bound makes that row unadmitted and the other rows run. A sample's interval or choice outside a bound refuses the plan.",
    );
    if let Some(plans) = worked_plans_in(view) {
        text.push_str(" One worked plan of each kind, each a whole request:");
        for (kind, plan) in plans {
            text.push_str(&format!(" {kind}: {plan}"));
        }
        text.push('.');
    }
    text
}

/// What the `answer` input of `run_dataset` says after its three forms: what
/// a summary row holds, what each kind of row carries, and how an inline
/// answer is counted before anything runs.
pub(crate) fn answer_says() -> String {
    format!(
        "The head and the dataset_seal are the same in all three forms. A summary row holds row, set (the values a grid's or a sample's row sets), outcome, steps_landed, reject and run_seal; the quantities of one row are read by sending its request to run_trajectory with a readout. An ended row is a run that stopped at an end its card declares: it has a run seal, the end's reject name in reject and the sentence naming the end in reject_detail. A refused row is a run the card refused part way: it has a run seal, the refusal's name in reject and its sentence in reject_detail. An unadmitted row is a request turned away before computing: it has no run seal, and its sentence, with the refusal's name in round brackets when the sentence gives one, is its reject_detail; its summary row gives that name in reject. What a sentence that begins \"Physical Failure\" or \"Physical Violation\" means is in the description of run_trajectory. An inline answer is counted from the plan before anything runs, and one above {MAX_INLINE_ANSWER_CHARS} characters is refused with what to change: a summary row counts {SUMMARY_ROW_FIXED_CHARS} characters and the values it sets; with states, each number of the final state adds its name and 28 characters (84 for a 3-vector, 9 for a flag); a whole row counts its own request and {FULL_ROW_FIXED_CHARS} characters, and for each number twice its name and 60 characters (150 for a 3-vector, 9 for a flag); store \"frames\" adds 128 characters for each recorded state to a full answer; the head repeats the request."
    )
}

/// What the `solver` input of `run_dataset` says to an agent of this view: how
/// a plan of solves is written, how its rows are sealed, what a summary row
/// holds, which tools read it, and one worked plan.
pub(crate) fn solves_say_in(view: &View) -> String {
    let mut text = format!(
        "A plan of solves is answered inline: a list, {{\"kind\":\"list\",\"solves\":[{{\"parameters\":{{...}}}}]}}, or a grid, {{\"kind\":\"grid\",\"base\":{{\"parameters\":{{...}}}},\"axes\":[{{\"path\":\"parameters.<input>\",\"values\":[...]}}]}}, of at most {} solves. Each row is a solve request, solved as solve solves it and carrying the solution_seal solve gives it; a request turned away without a seal, by the rules of a request or by the solver, is an unadmitted row. The rows are sealed under one dataset_seal by the rules of a plan of runs, with solution_seal in the leaf in the place of run_seal, and completed in the counts counts the solves that converged. A summary row has row, set, outcome, reject, solution_seal and every single number of the result in decimals; answer \"full\" gives every solution whole, and is refused when it comes to more than {MAX_INLINE_ANSWER_CHARS} characters. verify_dataset verifies a plan of solves by its request and dataset_seal, and validate_series compares one number of its results with measurements.",
        crate::solve_plan::MAX_SOLVES
    );
    if let Some(plan) = crate::solve_plan::worked_plan_in(view) {
        text.push_str(&format!(" Worked plan of solves: {plan}."));
    }
    text
}

pub fn run_schema() -> Map<String, Value> {
    object_map(json!({
        "type": "object",
        "properties": {
            "domain": { "type": "string" },
            "card": { "type": "string" },
            "plan": {
                "oneOf": [
                    {
                        "type": "object",
                        "description": "A list of runs. Each is a run_trajectory request without domain and card.",
                        "properties": {
                            "kind": { "const": "list" },
                            "runs": { "type": "array", "minItems": 1, "items": { "type": "object" } }
                        },
                        "required": ["kind", "runs"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "description": "With solver: a list of solves. Each is a solve request without solver.",
                        "properties": {
                            "kind": { "const": "list" },
                            "solves": { "type": "array", "minItems": 1, "items": { "type": "object" } }
                        },
                        "required": ["kind", "solves"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "description": "A base run (with solver: a base solve request without solver) and axes of explicit values. The rows are the product of the axes, the last axis varying fastest.",
                        "properties": {
                            "kind": { "const": "grid" },
                            "base": { "type": "object" },
                            "axes": {
                                "type": "array",
                                "minItems": 1,
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "path": { "type": "string" },
                                        "values": { "type": "array", "minItems": 1 }
                                    },
                                    "required": ["path", "values"],
                                    "additionalProperties": false
                                }
                            }
                        },
                        "required": ["kind", "base", "axes"],
                        "additionalProperties": false
                    },
                    {
                        "type": "object",
                        "description": "A base run and draws from the counter sampler. Row n is worked out without the rows before it.",
                        "properties": {
                            "kind": { "const": "sample" },
                            "base": { "type": "object" },
                            "rows": { "type": "integer", "minimum": 1 },
                            "generator": { "const": "splitmix64_counter_v1" },
                            "seed_hex": { "type": "string", "pattern": "^[0-9a-f]{16}$" },
                            "draws": {
                                "type": "array",
                                "minItems": 1,
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "path": { "type": "string" },
                                        "uniform": {
                                            "type": "object",
                                            "properties": {
                                                "low": { "type": "number" },
                                                "high": { "type": "number" }
                                            },
                                            "required": ["low", "high"],
                                            "additionalProperties": false
                                        },
                                        "choice": { "type": "array", "minItems": 1 }
                                    },
                                    "required": ["path"],
                                    "additionalProperties": false
                                }
                            }
                        },
                        "required": ["kind", "base", "rows", "generator", "seed_hex", "draws"],
                        "additionalProperties": false
                    }
                ]
            },
            "store": { "type": "string", "enum": ["frames", "none"] },
            "return": { "type": "string", "enum": ["inline", "file"] },
            "solver": { "type": "string", "description": "A plan of solves: the solver, in place of domain and card. It takes no store and no return." },
            "answer": { "type": "string", "enum": ["summary", "states", "full"], "description": "Leave out, or \"summary\", for the head and one short summary row per row: the values it sets, outcome, steps landed, refusal name and seal; \"states\" adds each run's final state in decimals; \"full\" gives every row whole. With return \"file\" the answer is the head and the names of the files whatever this says. Not part of the sealed request." }
        },
        "required": ["plan"],
        "additionalProperties": false,
        "description": format!(
            "A plan of runs on one card (domain, card, plan, store and return, all required) or of solves of one solver (solver and plan). Rows <= {MAX_DATASET_ROWS}, steps in total <= {MAX_DATASET_STEPS}, stored frames <= {MAX_DATASET_FRAMES}; inline: an answer counted from the plan <= {MAX_INLINE_ANSWER_CHARS} characters, which a plan reaches first, a grid on one axis at about 140 summary rows and a list sooner, its head repeating each run's request (a plan of more than {MAX_INLINE_ROWS} rows is refused before it is counted, as too large to return inline), and frames <= {MAX_INLINE_FRAMES}; solves <= {}. With return \"file\" the answer is the head and the names of the files stored, on a connection started with a directory for them.",
            crate::solve_plan::MAX_SOLVES
        ),
    }))
}

pub fn verify_schema() -> Map<String, Value> {
    object_map(json!({
        "type": "object",
        "properties": {
            "request": { "type": "object", "description": "With dataset_seal alone: the run_dataset request as it was sent (the head's request). The plan is run again, under the permits and the limits of run_dataset, and its seal is compared with dataset_seal. Nothing is stored or read from storage, so it works whatever the return mode and on every connection. Its fields are run_dataset's." },
            "head": { "type": "object", "description": "With rows and frames_hex: the head a run_dataset answer carries." },
            "rows": { "type": "array", "items": { "type": "object" }, "description": "With head and frames_hex: the rows a full run_dataset answer (answer \"full\") carries." },
            "frames_hex": {
                "type": ["string", "null"],
                "pattern": "^([0-9a-f]{2})*$",
                "description": "With head and rows: the frames a run_dataset answer carries, or null."
            },
            "dataset_seal": {
                "type": "string",
                "pattern": "^[0-9a-f]{64}$",
                "description": "With request: the dataset_seal of the run_dataset answer, compared with the seal the plan runs to. Alone: the seal of a dataset stored as files (return \"file\"), read from the connection's own folder of that name. A dataset returned in the answer is not stored: it is verified with its request, or by its head, rows and frames_hex."
            }
        },
        "additionalProperties": false,
        "description": "One of three forms. request and dataset_seal, the short form: the plan is run again and its seal compared. The three parts a full run_dataset answer carries (head, rows and frames_hex, all three required together, as they came), the long form: the rows, the root and the seal are recomputed from their exact values, then the head's plan is run again under the permits and the limits of run_dataset and the engine's full answer to it compared with the parts sent, value by value and then byte for byte, so a dataset that holds is every byte the engine's. dataset_seal alone: the stored dataset of that seal, its seals recomputed from its files and its plan run again into a folder of its own, the stored files compared with that run's byte for byte. The long form's answer carries holds, true when the answer sent is every byte the engine's, and values_hold beside it, true when every value of it is the engine's answer to its request computed again, whatever its spelling: each number by its 64-bit value, strings, flags and null exactly, keys and their order, the receipt string among them. A copy whose numbers a client wrote again in its own spelling (0.0001 for 1.0e-4, 12 for 12.0) has holds false and values_hold true, and differs_at names the first place its text differs."
    }))
}

fn object_map(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("schema is an object"),
    }
}

#[cfg(test)]
mod answer_bounds {
    use super::*;

    /// On every card, each worked plan's inline answer, in both forms, is no
    /// larger than the count `run_dataset` makes of it before anything runs,
    /// and the head and the seal do not depend on the form.
    #[test]
    fn every_inline_answer_is_within_its_count_and_the_seal_does_not_follow_the_form() {
        for card in descriptor::all() {
            let view = View::limited([(card.domain.to_string(), card.card.to_string())], []);
            let plans = worked_plans_in(&view).expect("every card has worked plans");
            for (kind, plan) in plans {
                let request = DatasetRequest::from_value(plan).unwrap_or_else(|e| panic!("{e}"));
                let mut heads = Vec::new();
                for form in [Form::Summary, Form::States, Form::Full] {
                    let counted = inline_answer_chars(&request, form);
                    let text = run_inline(&request, form, &[]).unwrap_or_else(|e| panic!("{} {kind}: {}", card.card, e.message()));
                    assert!(
                        (text.len() as u64) <= counted,
                        "{} {kind} {form:?}: {} characters, counted {counted}",
                        card.card,
                        text.len()
                    );
                    let body = serde_json::from_str::<Value>(&text).unwrap();
                    // The full rows, checked as they were made, hold with no
                    // row named: on every card each decimal of a final state
                    // is the value of the hex beside it.
                    if form == Form::Full {
                        let state = crate::new_gateway_state();
                        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                        let checked = runtime
                            .block_on(verify(&state, &body, Some(&text)))
                            .unwrap_or_else(|e| panic!("{} {kind}: {}", card.card, e.message()));
                        let checked: Value = serde_json::from_str(&checked).unwrap();
                        assert_eq!(checked["holds"], true, "{} {kind}: {checked}", card.card);
                        assert_eq!(checked["row"], Value::Null, "{} {kind}: {checked}", card.card);
                        assert!(checked.get("fault").is_none() && checked.get("what_to_report").is_none(), "{} {kind}: {checked}", card.card);
                    }
                    heads.push(body["head"].clone());
                }
                assert_eq!(heads[0], heads[1], "{} {kind}", card.card);
                assert_eq!(heads[0], heads[2], "{} {kind}", card.card);
            }
        }
    }

    #[test]
    fn an_axis_replaces_the_base_value_it_names_and_the_answer_says_so() {
        let plan = json!({
            "domain": "energy", "card": "thermal", "answer": "full",
            "plan": { "kind": "grid",
                "base": { "initial": { "temperature_k": 293.15, "v": [1, 2, 3] }, "dt_s": 1.0,
                          "segments": [{ "steps": 2, "load": { "ambient_k": 293.15, "heat_power_w": 1.0 } }], "record_every": 1 },
                "axes": [{ "path": "segments[0].load.heat_power_w", "values": [0.0, 5.0] },
                         { "path": "initial.v[0]", "values": [1] }] },
            "store": "none", "return": "inline"
        });
        let Ok(normalized) = normalize(plan) else { panic!("not read") };
        assert_eq!(normalized.form, Form::Full);
        assert!(normalized.form_given);
        assert_eq!(normalized.replaced, vec!["segments[0].load.heat_power_w".to_string()]);
        assert!(normalized.request.get("answer").is_none());
        let base = &normalized.request["plan"]["base"];
        assert!(base["segments"][0]["load"].get("heat_power_w").is_none());
        // A 3-vector set by one component only is left for the plan's reading to refuse.
        assert_eq!(base["initial"]["v"], json!([1, 2, 3]));
        assert!(normalize(json!({ "answer": "rows" })).is_err());
        assert_eq!(name_in_brackets("Physical Violation: x is outside (ReactivityOutsideCard)."), Some("ReactivityOutsideCard".to_string()));
        assert_eq!(name_in_brackets("Physical Violation: x (a note) y."), None);
    }

    /// Rows whose numbers were written again after one was taken out carry
    /// no number that tells where they came from: each is looked for among
    /// the rows of the plan nearest its place, and no further than the search
    /// reaches.
    #[test]
    fn a_row_out_of_its_place_is_looked_for_nearest_first_and_only_as_far_as_the_search_reaches() {
        let run = |power: f64| {
            json!({ "initial": { "temperature_k": 293.15 }, "dt_s": 1.0, "record_every": 1,
                    "segments": [{ "steps": 2, "load": { "heat_power_w": power, "ambient_k": 293.15 } }] })
        };
        let plan = json!({ "domain": "energy", "card": "thermal", "store": "none", "return": "inline",
                           "plan": { "kind": "list", "runs": (0..8).map(|n| run(f64::from(n))).collect::<Vec<_>>() } });
        let request = DatasetRequest::from_value(plan).unwrap_or_else(|e| panic!("{e}"));
        let body: Value = serde_json::from_str(&run_inline(&request, Form::Full, &[]).unwrap_or_else(|e| panic!("{}", e.message()))).unwrap();
        let renumbered = |mut rows: Vec<Value>| -> Vec<String> {
            for (place, row) in rows.iter_mut().enumerate() {
                row["row"] = json!(place);
                row["leaf"]["row"] = json!(place);
            }
            rows.iter().map(|row| serde_json::to_string(row).unwrap()).collect()
        };
        let made = body["rows"].as_array().unwrap().clone();
        let read = |lines: &[String], search_most: u64| -> (Option<u64>, Option<u64>, String) {
            let plan = DatasetRequest::from_value(body["head"]["request"].clone()).unwrap_or_else(|e| panic!("{e}"));
            let lines: Vec<&[u8]> = lines.iter().map(String::as_bytes).collect();
            match genesis_seal::fault::first_out_of_place(plan, &lines, search_most) {
                Some((place, row, says)) => (Some(place), row, says),
                None => (None, None, String::new()),
            }
        };
        // Row 2 taken out: the row in its place is found one row on.
        let mut dropped = made.clone();
        dropped.remove(2);
        let dropped = renumbered(dropped);
        assert_eq!(
            read(&dropped, 8),
            (Some(2), Some(3), "A row was dropped: row 2 is the first row missing, and the row in its place is row 3. Rows read: 7; rows in the plan: 8.".to_string())
        );
        // Row 7 written in the place of row 0: found when the search reaches seven rows on, and said only to be changed when it does not.
        let mut moved = made.clone();
        moved[0] = made[7].clone();
        let moved = renumbered(moved);
        assert_eq!(read(&moved, 64).1, Some(7));
        assert_eq!(read(&moved, 2), (Some(0), None, "A row was changed: the request of row 0 is not the request the plan gives it.".to_string()));
        // The search writes out no more rows of the plan than it is given, whatever the plan's size.
        assert_eq!(genesis_seal::fault::search_most(4 << 20, MAX_DATASET_ROWS), 2);
    }
}
