//! A plan of solves, and the comparison of one result of those solves with a
//! caller's measurements.
//!
//! A plan of solves is a `run_dataset` request that names a solver in place
//! of a card: a list of solve requests, or a grid of explicit values over a
//! base request. Every row is an ordinary `solve` request, solved by the code
//! `solve` uses and sealed with the `solution_seal` it carries alone. The
//! rows are sealed under one dataset seal by the rules of `genesis_seal`: the
//! plan hash is SHA-384 of the sealed form of the request, the leaf of a row
//! is the compact JSON of `row`, `request_sha384`, `outcome`, the row's
//! seal and `reject_detail_sha384`, the leaves form the row tree of RFC
//! 6962, and `dataset_seal` is SHA-384 of the plan hash, the root and the compact JSON of the counts.
//!
//! The comparison (`validate_series` given `solves`) runs such a plan and
//! compares one published number of each solve's result with the caller's
//! measurements, by the statistics, the measurement hash and the validation
//! seal of `genesis_seal::validation`. Nothing here is stored.

use genesis_seal::dataset::{Counts, Head, dataset_seal};
use genesis_seal::plan::with_sorted_keys;
use genesis_seal::sealed_form::{digest_hex, sealed_form};
use genesis_seal::tree;
use genesis_seal::validation::{
    DOES_NOT_SHOW, SealParts, Statistics, bits_hex, measurements_sha384, statistics, validation_seal,
};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::dataset::{self, Form, MAX_INLINE_ANSWER_CHARS, Normalized};
use crate::solution::SolutionResponse;
use crate::solvers::{Shape, Solver};
use crate::view::View;
use crate::{AppState, DoorError, trajectory};

/// The most solves one plan of solves holds. A solve can take seconds, and a
/// plan is solved one row after another under one physics permit.
pub const MAX_SOLVES: u64 = 64;

/// The sentence for the three parts of a dataset of solves sent to
/// `verify_dataset`: its rows are solutions, which the check by parts does
/// not read.
pub(crate) const VERIFY_SOLVES_BY_REQUEST: &str = "Physical Violation: the head is that of a plan of solves, whose rows are solutions and are not verified by parts. Verify it by sending its run_dataset request as 'request' with its 'dataset_seal': the solves are solved again and the seal compared.";

/// What a summary answer of a plan of solves says of itself.
pub(crate) const SOLVES_SUMMARY_ABOUT: &str = "One summary row per solve: its solution_seal, outcome, refusal name and every single number of its result in decimals, and for a grid the values the row sets. The head and the dataset_seal are those of the full rows. Send the request again with \"answer\": \"full\" for every solution whole, or verify the seal with verify_dataset given this request and the dataset_seal.";

/// How the seal of a comparison of solves is taken, said in its answer.
pub(crate) const SOLVES_SEALED_AS: &str = "validation_seal is the validation seal of genesis_seal, taken with the dataset_seal of the solves in the place of a run seal, each measured row number in the place of a step, outcome \"solves\", steps_landed the count of solves that converged and reject null. The statistics and measurements_sha384 follow the rules of a run's comparison. A measured row whose solve did not converge is not compared and is counted in not_reached. genesis_seal verify-validation reads this comparison as it reads a run's, from the answer alone; the same request sent again gives the same seal.";

const PLAN_FIELDS: &str = "Its fields are solver, plan and answer.";

fn violation(text: impl std::fmt::Display) -> DoorError {
    DoorError::Unprocessable(format!("Physical Violation: {text}"))
}

fn missing(key: &str) -> DoorError {
    DoorError::Unprocessable(format!("Missing required parameter: '{key}' (InputMissing)."))
}

/// A plan of solves, read and held to its solver's input list.
pub(crate) struct SolvePlan {
    /// The request as it is sealed: without `answer`, and with every base
    /// value an axis replaces taken out.
    request: Value,
    solver: &'static Solver,
    /// Every row's whole `solve` request, keys in the order of their bytes.
    rows: Vec<Value>,
    /// The paths of a grid's axes, in plan order; empty for a list.
    paths: Vec<String>,
}

impl SolvePlan {
    /// Every row's whole `solve` request, in plan order.
    pub(crate) fn rows(&self) -> &[Value] {
        &self.rows
    }
}

/// Read a plan of solves for a caller of `view`.
pub(crate) fn read(view: &View, request: Value) -> Result<SolvePlan, DoorError> {
    let object = request
        .as_object()
        .ok_or_else(|| violation("the request must be a JSON object."))?;
    for key in object.keys() {
        match key.as_str() {
            "solver" | "plan" => {}
            "store" | "return" => {
                return Err(violation(format!(
                    "'{key}' is not a field of a plan of solves. A plan of solves is returned in the answer and is not stored. {PLAN_FIELDS}"
                )));
            }
            "domain" | "card" => {
                return Err(violation(format!(
                    "'{key}' is not a field of a plan of solves, which names a solver in place of a card. {PLAN_FIELDS}"
                )));
            }
            _ => return Err(violation(format!("'{key}' is not a field of a plan of solves. {PLAN_FIELDS}"))),
        }
    }
    let name = match object.get("solver") {
        Some(Value::String(name)) => name.as_str(),
        Some(_) => return Err(violation("'solver' must be a string.")),
        None => return Err(missing("solver")),
    };
    let solver = Some(name)
        .filter(|name| view.sees_solver(name))
        .and_then(crate::solvers::find)
        .ok_or_else(|| DoorError::NotFound(crate::SOLVER_NOT_RECOGNIZED.to_string()))?;
    let plan = object
        .get("plan")
        .ok_or_else(|| missing("plan"))?
        .as_object()
        .ok_or_else(|| violation("'plan' must be a JSON object."))?;
    let kind = match plan.get("kind") {
        Some(Value::String(kind)) => kind.as_str(),
        Some(_) => return Err(violation("'kind' must be a string.")),
        None => return Err(missing("kind")),
    };
    let parameters_of = |value: &Value, name: &str| -> Result<Map<String, Value>, DoorError> {
        let item = value
            .as_object()
            .ok_or_else(|| violation(format!("'{name}' must be a JSON object.")))?;
        for key in item.keys() {
            if key != "parameters" {
                return Err(violation(format!(
                    "'{key}' is not a field of '{name}'. A solve of a plan is its solve request without solver: 'parameters' alone."
                )));
            }
        }
        item.get("parameters")
            .ok_or_else(|| missing("parameters"))?
            .as_object()
            .cloned()
            .ok_or_else(|| violation("'parameters' must be a JSON object."))
    };
    let whole = |parameters: Map<String, Value>| {
        with_sorted_keys(&serde_json::json!({ "solver": solver.name, "parameters": parameters }))
    };
    let (rows, paths) = match kind {
        "list" => {
            only_keys(plan, &["kind", "solves"])?;
            let items = plan
                .get("solves")
                .ok_or_else(|| missing("solves"))?
                .as_array()
                .ok_or_else(|| violation("'solves' must be a JSON array."))?;
            if items.is_empty() {
                return Err(violation("the plan has no rows."));
            }
            too_many(items.len() as u64)?;
            let mut rows = Vec::with_capacity(items.len());
            for item in items {
                rows.push(whole(parameters_of(item, "solves")?));
            }
            (rows, Vec::new())
        }
        "grid" => {
            only_keys(plan, &["kind", "base", "axes"])?;
            let base = parameters_of(plan.get("base").ok_or_else(|| missing("base"))?, "base")?;
            let items = plan
                .get("axes")
                .ok_or_else(|| missing("axes"))?
                .as_array()
                .ok_or_else(|| violation("'axes' must be a JSON array of objects."))?;
            if items.is_empty() {
                return Err(violation("'axes' must contain at least one axis."));
            }
            let mut axes: Vec<(String, String, &Vec<Value>)> = Vec::with_capacity(items.len());
            let mut count = 1_u64;
            for item in items {
                let axis = item
                    .as_object()
                    .ok_or_else(|| violation("'axes' must be a JSON array of objects."))?;
                only_keys(axis, &["path", "values"])?;
                let path = match axis.get("path") {
                    Some(Value::String(path)) => path.clone(),
                    Some(_) => return Err(violation("'path' must be a string.")),
                    None => return Err(missing("path")),
                };
                let input = path
                    .strip_prefix("parameters.")
                    .filter(|input| genesis_seal::json_text::is_plain_field_name(input))
                    .ok_or_else(|| {
                        violation(format!(
                            "'{path}' is not a path of a plan of solves. A path is parameters.<input>."
                        ))
                    })?
                    .to_string();
                if !solver.inputs.iter().any(|known| known.name == input) {
                    return Err(violation(format!("'{path}' names no input of the {} solver.", solver.name)));
                }
                if axes.iter().any(|(_, known, _)| *known == input) {
                    return Err(violation(format!("'{path}' is set twice.")));
                }
                if base.contains_key(&input) {
                    return Err(violation(format!("'{path}' is set twice.")));
                }
                let values = axis
                    .get("values")
                    .ok_or_else(|| missing("values"))?
                    .as_array()
                    .ok_or_else(|| violation("'values' must be a JSON array."))?;
                if values.is_empty() {
                    return Err(violation(format!("'{path}' lists no values.")));
                }
                count = count.checked_mul(values.len() as u64).unwrap_or(u64::MAX);
                axes.push((path, input, values));
            }
            too_many(count)?;
            let mut rows = Vec::with_capacity(count as usize);
            for index in 0..count {
                let mut parameters = base.clone();
                // The last axis varies fastest.
                let mut rest = index;
                for (_, input, values) in axes.iter().rev() {
                    let length = values.len() as u64;
                    parameters.insert(input.clone(), values[(rest % length) as usize].clone());
                    rest /= length;
                }
                rows.push(whole(parameters));
            }
            (rows, axes.into_iter().map(|(path, _, _)| path).collect())
        }
        "sample" => return Err(violation("'sample' is not a kind of plan of solves. The kinds are list and grid.")),
        other => return Err(violation(format!("'{other}' is not a kind of plan of solves. The kinds are list and grid."))),
    };
    Ok(SolvePlan { request, solver, rows, paths })
}

fn only_keys(object: &Map<String, Value>, allowed: &[&str]) -> Result<(), DoorError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(violation(format!("'{key}' is not a field of this request.")));
        }
    }
    Ok(())
}

fn too_many(rows: u64) -> Result<(), DoorError> {
    if rows > MAX_SOLVES {
        return Err(violation(format!(
            "the plan asks for {rows} solves, more than the {MAX_SOLVES} one plan of solves may hold."
        )));
    }
    Ok(())
}

/// A plan of solves, solved and sealed.
struct Sealed {
    head: Head,
    rows: Vec<Value>,
    summary: Vec<Value>,
    /// The solution of each row, or `None` for a row turned away.
    solutions: Vec<Option<SolutionResponse>>,
}

/// The leaf of a row: the compact JSON of `row`, `request_sha384`, `outcome`,
/// `solution_seal` and `reject_detail_sha384`, in that order. The last is the
/// SHA-384 of the row's refusal sentence, or null when it has none, as in the
/// leaf of a row of runs: the sentence of a row turned away before solving is
/// covered by no solution seal, and is sealed here.
fn leaf_of(row: u64, request: &Value, outcome: &str, seal: Option<&str>, reject_detail: Option<&str>) -> Value {
    let mut leaf = Map::new();
    leaf.insert("row".to_string(), Value::from(row));
    leaf.insert("request_sha384".to_string(), Value::from(digest_hex(sealed_form(request).as_bytes())));
    leaf.insert("outcome".to_string(), Value::from(outcome));
    leaf.insert("solution_seal".to_string(), seal.map_or(Value::Null, Value::from));
    leaf.insert(
        "reject_detail_sha384".to_string(),
        reject_detail.map_or(Value::Null, |sentence| Value::from(digest_hex(sentence.as_bytes()))),
    );
    Value::Object(leaf)
}

/// Every single number of a result, in decimals.
fn numbers_of(result: &Value) -> Value {
    match result.as_object() {
        Some(result) => Value::Object(
            result
                .iter()
                .filter(|(key, value)| !key.ends_with("_hex") && value.is_number())
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ),
        None => Value::Null,
    }
}

fn encode_failed(error: impl std::fmt::Display) -> DoorError {
    DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}"))
}

/// Solve every row in plan order on this thread, which holds the plan's
/// permits, and seal the rows.
fn execute(view: &View, plan: &SolvePlan) -> Result<Sealed, DoorError> {
    let mut counts = Counts {
        // The rows are solves: the version the solvers write, as each row's
        // solution seal does (RT3 W1 of 0.14.0 rc1).
        engine_version: crate::solvers::engine_version(),
        rows: plan.rows.len() as u64,
        completed: 0,
        ended: 0,
        refused: 0,
        unadmitted: 0,
        frames: 0,
    };
    let mut leaves = Vec::with_capacity(plan.rows.len());
    let mut rows = Vec::with_capacity(plan.rows.len());
    let mut summary = Vec::with_capacity(plan.rows.len());
    let mut solutions = Vec::with_capacity(plan.rows.len());
    let mut rows_text = Vec::new();
    for (index, request) in plan.rows.iter().enumerate() {
        let index = index as u64;
        let solved = crate::solution::solve_in_plan(view, request.clone()).map_err(|fault| {
            DoorError::Internal(format!("Physical Violation: row {index} was not sealed. {fault}"))
        })?;
        let mut row = Map::new();
        row.insert("row".to_string(), Value::from(index));
        row.insert("request".to_string(), request.clone());
        let set = dataset::set_of(&plan.paths, request);
        let leaf = match &solved {
            Ok(response) => {
                let document = serde_json::to_value(&response.document).map_err(encode_failed)?;
                let Value::Object(document) = document else {
                    return Err(encode_failed("a solution is not an object"));
                };
                for (key, value) in document {
                    if key != "inputs" && key != "engine_version" {
                        row.insert(key, value);
                    }
                }
                let outcome = response.document.outcome.as_str();
                match outcome {
                    "converged" => counts.completed += 1,
                    _ => counts.refused += 1,
                }
                summary.push(brief_of(
                    index,
                    set,
                    outcome,
                    response.document.reject.as_deref(),
                    Some(&response.document.solution_seal),
                    response.document.result.as_ref().map_or(Value::Null, numbers_of),
                ));
                leaf_of(
                    index,
                    request,
                    outcome,
                    Some(&response.document.solution_seal),
                    response.document.reject_detail.as_deref(),
                )
            }
            Err(sentence) => {
                counts.unadmitted += 1;
                for key in ["outcome", "iterations", "reject", "reject_detail", "result", "invariants", "field_sha384", "seal_parts", "solution_seal"] {
                    row.insert(key.to_string(), Value::Null);
                }
                row.insert("outcome".to_string(), Value::from("unadmitted"));
                row.insert("reject_detail".to_string(), Value::from(sentence.as_str()));
                summary.push(brief_of(index, set, "unadmitted", dataset::name_in_brackets(sentence).as_deref(), None, Value::Null));
                leaf_of(index, request, "unadmitted", None, Some(sentence.as_str()))
            }
        };
        let leaf_text = serde_json::to_string(&leaf).map_err(encode_failed)?;
        leaves.push(tree::leaf_hash(leaf_text.as_bytes()));
        row.insert("leaf".to_string(), leaf);
        let row = Value::Object(row);
        rows_text.extend_from_slice(serde_json::to_string(&row).map_err(encode_failed)?.as_bytes());
        rows_text.push(b'\n');
        rows.push(row);
        solutions.push(solved.ok());
    }
    let plan_sha384 = digest_hex(sealed_form(&plan.request).as_bytes());
    let merkle_root = hex::encode(tree::root(&leaves).expect("a plan has at least one row"));
    let seal = dataset_seal(&plan_sha384, &merkle_root, &counts);
    let head = Head {
        request: plan.request.clone(),
        counts,
        plan_sha384,
        merkle_root,
        dataset_seal: seal,
        rows_sha384: Some(digest_hex(&rows_text)),
        frames_sha384: None,
    };
    Ok(Sealed { head, rows, summary, solutions })
}

/// Solve a plan under one dataset permit and one physics permit, as a dataset
/// job of cards takes them, and seal it.
async fn solve_and_seal(state: &AppState, plan: SolvePlan) -> Result<(SolvePlan, Sealed), DoorError> {
    let workers = dataset::take_pair(state)?;
    let state = state.clone();
    let task = tokio::task::spawn_blocking(move || {
        let sealed = execute(state.view(), &plan);
        #[cfg(test)]
        if !state.permit_hold.is_zero() {
            std::thread::sleep(state.permit_hold);
        }
        drop(workers);
        sealed.map(|sealed| (plan, sealed))
    });
    task.await.unwrap_or_else(|_| {
        Err(DoorError::Internal(
            "Physical Violation: the plan of solves stopped by a fault before it finished. Nothing was sealed.".to_string(),
        ))
    })
}

/// The summary row of one solve: `row`, `set` for a grid, `outcome`,
/// `reject`, `solution_seal` and `result`, every single number of the result
/// in decimals. The rows of an answer and the count before solving are both
/// written by this.
fn brief_of(index: u64, set: Option<Value>, outcome: &str, reject: Option<&str>, seal: Option<&str>, result: Value) -> Value {
    let mut brief = Map::new();
    brief.insert("row".to_string(), Value::from(index));
    if let Some(set) = set {
        brief.insert("set".to_string(), set);
    }
    brief.insert("outcome".to_string(), Value::from(outcome));
    brief.insert("reject".to_string(), reject.map_or(Value::Null, Value::from));
    brief.insert("solution_seal".to_string(), seal.map_or(Value::Null, Value::from));
    brief.insert("result".to_string(), result);
    Value::Object(brief)
}

/// A number written in the most characters a decimal of a 64-bit number
/// takes: a sign, seventeen digits, a point and an exponent of three digits
/// with its sign, 24 in all.
const DECIMAL_MOST: f64 = -1.2345678901234567e-308;

/// The longest refusal name a row turned away can carry where its sentence
/// names no more than the engine's own names: every name a solver lists is
/// shorter (`a_plan_of_solves_is_counted_between_its_least_and_its_most`).
const REFUSAL_NAME_MOST: usize = 64;

fn chars_of(value: &Value) -> u64 {
    serde_json::to_string(value).map_or(u64::MAX, |text| text.len() as u64)
}

/// The least and the most characters the summary answer of `plan` can come
/// to, counted before anything is solved: the head with the whole request,
/// what the answer says of itself and the paths a grid replaced, and for each
/// row its fixed part, the values it sets and each single number its solver
/// publishes. At the most, a row converged with every number the solver can
/// publish in 24 characters, or was refused or turned away under the longest
/// name it can carry; at the least, a row turned away with no name, or
/// converged with the numbers every solution holds in one character each.
/// The answer itself is counted again once it is made.
fn summary_bounds(plan: &SolvePlan, replaced: &[String]) -> (u64, u64) {
    let rows = plan.rows.len() as u64;
    let head = |most: bool| {
        let counted = if most { rows } else { 0 };
        let counts = Counts {
            engine_version: crate::solvers::engine_version(),
            rows,
            completed: counted,
            ended: 0,
            refused: counted,
            unadmitted: counted,
            frames: 0,
        };
        let hash = "0".repeat(96);
        Head {
            request: plan.request.clone(),
            counts,
            plan_sha384: hash.clone(),
            merkle_root: hash.clone(),
            dataset_seal: hash.clone(),
            rows_sha384: Some(hash),
            frames_sha384: None,
        }
        .wire()
        .len() as u64
    };
    let mut body = Map::new();
    body.insert("head".to_string(), Value::Null);
    body.insert("summary".to_string(), Value::Array(Vec::new()));
    body.insert("about".to_string(), Value::from(SOLVES_SUMMARY_ABOUT));
    dataset::note_replaced(&mut body, replaced);
    // The body less its head's `null`, and a comma between two rows.
    let around = chars_of(&Value::Object(body)) - 4 + rows.saturating_sub(1);
    let numbers = |most: bool| {
        let mut result = Map::new();
        for output in plan.solver.result.iter().filter(|output| matches!(output.shape, Shape::Number)) {
            let written = if most { Value::from(DECIMAL_MOST) } else { Value::from(0_u64) };
            match output.family_ends() {
                None if most || output.asked_by.is_none() => {
                    result.insert(output.name.to_string(), written);
                }
                None => {}
                Some((before, after)) => {
                    if most {
                        for label in output.each {
                            result.insert(format!("{before}{label}{after}"), written.clone());
                        }
                    }
                }
            }
        }
        Value::Object(result)
    };
    let (most_numbers, least_numbers) = (numbers(true), numbers(false));
    let sealed: Vec<&str> = plan.solver.refusals.iter().filter(|refusal| refusal.sealed).map(|refusal| refusal.name).collect();
    let seal = "0".repeat(96);
    let (mut least, mut most) = (head(false) + around, head(true) + around);
    for (index, request) in plan.rows.iter().enumerate() {
        let index = index as u64;
        let set = || dataset::set_of(&plan.paths, request);
        let turned_away_name = "A".repeat(REFUSAL_NAME_MOST.max(chars_of(request) as usize));
        let mut row_most = vec![
            chars_of(&brief_of(index, set(), "converged", None, Some(&seal), most_numbers.clone())),
            chars_of(&brief_of(index, set(), "unadmitted", Some(&turned_away_name), None, Value::Null)),
        ];
        let mut row_least = vec![
            chars_of(&brief_of(index, set(), "converged", None, Some(&seal), least_numbers.clone())),
            chars_of(&brief_of(index, set(), "unadmitted", None, None, Value::Null)),
        ];
        if let (Some(longest), Some(shortest)) = (sealed.iter().max_by_key(|name| name.len()), sealed.iter().min_by_key(|name| name.len())) {
            row_most.push(chars_of(&brief_of(index, set(), "refused", Some(longest), Some(&seal), Value::Null)));
            row_least.push(chars_of(&brief_of(index, set(), "refused", Some(shortest), Some(&seal), Value::Null)));
        }
        most = most.saturating_add(row_most.into_iter().max().unwrap_or(u64::MAX));
        least = least.saturating_add(row_least.into_iter().min().unwrap_or(0));
    }
    (least, most)
}

/// `run_dataset` given a plan of solves.
pub(crate) async fn run(state: &AppState, normalized: Normalized) -> Result<String, DoorError> {
    let Normalized { request, form, replaced, .. } = normalized;
    if form == Form::States {
        return Err(violation(
            "'answer' \"states\" is for a plan of runs: a solve has no state, and the summary row of a solve already carries every single number of its result.",
        ));
    }
    let plan = read(state.view(), request)?;
    let (least, most) = summary_bounds(&plan, &replaced);
    if least > MAX_INLINE_ANSWER_CHARS {
        return Err(DoorError::Unprocessable(too_large_before(least, most, form)));
    }
    let (_, sealed) = solve_and_seal(state, plan).await?;
    let text = answer_text(sealed, form, &replaced)?;
    if text.len() as u64 > MAX_INLINE_ANSWER_CHARS {
        return Err(DoorError::Unprocessable(too_large_after(text.len() as u64, form)));
    }
    Ok(text)
}

/// The answer of a solved plan in `form`: the head, then the summary rows
/// and what they are, or every row whole.
fn answer_text(sealed: Sealed, form: Form, replaced: &[String]) -> Result<String, DoorError> {
    let head = serde_json::from_str::<Value>(&sealed.head.wire()).map_err(encode_failed)?;
    let mut body = Map::new();
    body.insert("head".to_string(), head);
    match form {
        Form::Summary | Form::States => {
            body.insert("summary".to_string(), Value::Array(sealed.summary));
            body.insert("about".to_string(), Value::from(SOLVES_SUMMARY_ABOUT));
        }
        Form::Full => {
            body.insert("rows".to_string(), Value::Array(sealed.rows));
        }
    }
    dataset::note_replaced(&mut body, replaced);
    serde_json::to_string(&Value::Object(body)).map_err(encode_failed)
}

/// The refusal of a plan of solves whose answer cannot be under the limit,
/// before anything is solved: the least its summary can come to is above it.
/// A summary is said with the most it can come to; the whole rows are longer.
fn too_large_before(least: u64, most: u64, form: Form) -> String {
    let most = if form == Form::Full { String::new() } else { format!(" and at most {most}") };
    let mut text = format!(
        "Physical Violation: the answer would come to at least {least} characters{most}, more than the {MAX_INLINE_ANSWER_CHARS} one inline answer may have. It is counted from the plan before anything is solved, and nothing was solved. To make it smaller:"
    );
    to_make_it_smaller(&mut text, form);
    text
}

/// The refusal of an answer of a plan of solves above the limit, counted
/// once it is made: the decimals of a solution are not known in length
/// until it is solved.
fn too_large_after(counted: u64, form: Form) -> String {
    let mut text = format!(
        "Physical Violation: the answer comes to {counted} characters, more than the {MAX_INLINE_ANSWER_CHARS} one inline answer may have."
    );
    if form == Form::Full {
        text.push_str(" The whole solutions were counted once they were solved, and none is returned.");
    } else {
        text.push_str(" Its summary was counted once the plan was solved, and none is returned.");
    }
    text.push_str(" To make it smaller:");
    to_make_it_smaller(&mut text, form);
    text
}

fn to_make_it_smaller(text: &mut String, form: Form) {
    if form == Form::Full {
        text.push_str(" leave out 'answer' for one summary row per solve, with its seal, outcome and every single number of its result in decimals; or send fewer solves.");
    } else {
        text.push_str(" send fewer solves.");
    }
}

/// The dataset seal a plan of solves comes to, worked out again for a check
/// by request. Nothing is stored or read from storage.
pub(crate) async fn recompute_seal(state: &AppState, normalized: Normalized, given: &str) -> Result<(String, Option<String>), DoorError> {
    let plan = read(state.view(), normalized.request)?;
    let (plan, sealed) = solve_and_seal(state, plan).await?;
    let words = (sealed.head.dataset_seal != given).then(|| version_words(&plan, &sealed, given));
    Ok((sealed.head.dataset_seal, words))
}

/// The dataset seal of a solved plan with `kernel` in its counts and each
/// solution sealed under `solvers`: what the same plan solved by another
/// version comes to. A row turned away has no solution and keeps its leaf.
fn seal_under(plan: &SolvePlan, sealed: &Sealed, kernel: &str, solvers: &str) -> Option<String> {
    let mut leaves = Vec::with_capacity(plan.rows.len());
    for (index, ((request, solution), row)) in plan.rows.iter().zip(&sealed.solutions).zip(&sealed.rows).enumerate() {
        let leaf = match solution {
            Some(response) => {
                let seal = crate::solution::seal_with_version(&response.document, solvers)?;
                leaf_of(index as u64, request, &response.document.outcome, Some(&seal), response.document.reject_detail.as_deref())
            }
            None => row.get("leaf")?.clone(),
        };
        leaves.push(tree::leaf_hash(serde_json::to_string(&leaf).ok()?.as_bytes()));
    }
    let root = hex::encode(tree::root(&leaves)?);
    let counts = Counts { engine_version: kernel.to_string(), ..sealed.head.counts.clone() };
    Some(dataset_seal(&sealed.head.plan_sha384, &root, &counts))
}

/// What a check by request of a plan of solves says when the seal given is
/// not the one computed: the dataset seal covers `engine_version` through its
/// counts and through each row's solution seal, and, when the seal given is
/// this plan's under another version's names, those beside this engine's
/// (the red team of 0.13.0, D4). Tried: each earlier kernel with the solvers
/// one minor version before it, as the engine's releases paired them through
/// 0.13.x, each with this engine's other half, and the pairs a release made
/// otherwise (`PAIRS_OFF_THE_RULE`: 0.14.0's kernel 0.12.0 with solvers
/// 0.12.0).
fn version_words(plan: &SolvePlan, sealed: &Sealed, given: &str) -> String {
    let ours_kernel = sealed.head.counts.engine_version.clone();
    let ours_solvers = crate::solvers::engine_version();
    version_words_of(plan, sealed, given, &ours_kernel, &ours_solvers)
}

/// `version_words` for an engine that writes `ours_kernel` and
/// `ours_solvers`: a test reads it as a later engine would.
fn version_words_of(plan: &SolvePlan, sealed: &Sealed, given: &str, ours_kernel: &str, ours_solvers: &str) -> String {
    let (ours_kernel, ours_solvers) = (ours_kernel.to_string(), ours_solvers.to_string());
    let theirs = version_pairs(&ours_kernel, &ours_solvers)
        .into_iter()
        .filter(|(kernel, solvers)| !(kernel == &ours_kernel && solvers == &ours_solvers))
        .find(|(kernel, solvers)| seal_under(plan, sealed, kernel, solvers).as_deref() == Some(given));
    match theirs {
        Some((kernel, solvers)) => format!(
            "The dataset_seal covers engine_version, through its counts and each row's solution_seal: the seal given is this plan's dataset seal with engine_version {kernel} in its counts and {solvers} in its solution seals, another engine version's, and this engine writes {ours_kernel} in its counts and {ours_solvers} in its solution seals, so it does not hold here."
        ),
        None => format!(
            "The dataset_seal covers engine_version, through its counts ({ours_kernel} on this engine) and each row's solution_seal ({ours_solvers}): the seal of a plan another engine version solved does not hold here."
        ),
    }
}

/// The kernel and solvers versions an engine release paired otherwise than
/// each kernel with the solvers one minor version before it. Engine 0.14.0
/// moved the solvers to 0.12.0 (canonical math) and kept the kernel at
/// 0.12.0 (0.14.0 to 0.14.3); the next engine sets both to 0.13.0, the
/// three cards and the solver of 0.14.3 counted. The pairs of every earlier
/// release follow the rule: 0.8.0 with 0.7.0 (0.8.1 to 0.9.3), 0.9.0 with
/// 0.8.0 (0.10.x), 0.12.0 with 0.11.0 (0.13.x). A release whose pair breaks
/// the rule adds it here; a test holds this engine's own pair to the pairs a
/// later engine tries.
const PAIRS_OFF_THE_RULE: &[(&str, &str)] = &[
    ("genesis_kernel_v0.12.0", "genesis_solvers_v0.12.0"),
    ("genesis_kernel_v0.13.0", "genesis_solvers_v0.13.0"),
];

/// The kernel and solvers versions a plan's seal is tried under, for an
/// engine that writes `ours_kernel` and `ours_solvers`: each kernel with the
/// solvers one minor version before it, each with this engine's other half,
/// and the pairs of `PAIRS_OFF_THE_RULE`.
fn version_pairs(ours_kernel: &str, ours_solvers: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for minor in 0..=30_u32 {
        let kernel = format!("genesis_kernel_v0.{minor}.0");
        if let Some(before) = minor.checked_sub(1) {
            pairs.push((kernel.clone(), format!("genesis_solvers_v0.{before}.0")));
        }
        pairs.push((kernel, ours_solvers.to_string()));
        pairs.push((ours_kernel.to_string(), format!("genesis_solvers_v0.{minor}.0")));
        // From the work orders of 18 October 2026 a plan's counts name the
        // solvers' version, the one its solution seals carry.
        let solvers = format!("genesis_solvers_v0.{minor}.0");
        pairs.push((solvers.clone(), solvers));
    }
    for (kernel, solvers) in PAIRS_OFF_THE_RULE {
        let pair = (kernel.to_string(), solvers.to_string());
        if !pairs.contains(&pair) {
            pairs.push(pair);
        }
    }
    pairs
}

/// The keys of a comparison of solves.
pub(crate) const COMPARISON_KEYS: &[&str] = &["solves", "quantity", "measurements"];

#[derive(Serialize)]
struct Comparison<'a> {
    inputs: &'a Value,
    engine_version: &'a str,
    dataset_seal: &'a str,
    solves: u64,
    converged: u64,
    refused: u64,
    unadmitted: u64,
    compared: u64,
    not_reached: u64,
    not_reached_rows: &'a [u64],
    #[serde(serialize_with = "trajectory::serialize_optional_f64")]
    rms_error: Option<f64>,
    rms_error_hex: Option<String>,
    #[serde(serialize_with = "trajectory::serialize_optional_f64")]
    mean_signed_error: Option<f64>,
    mean_signed_error_hex: Option<String>,
    #[serde(serialize_with = "trajectory::serialize_optional_f64")]
    max_abs_error: Option<f64>,
    max_abs_error_hex: Option<String>,
    max_abs_error_row: Option<u64>,
    model_values: Vec<crate::readout::Exact>,
    model_values_hex: &'a [String],
    /// Present only when a measurement carries an instrument's signature.
    #[serde(skip_serializing_if = "Option::is_none")]
    measurement_source: Option<Value>,
    measurements_sha384: &'a str,
    /// Present only when the request opened a commitment.
    #[serde(skip_serializing_if = "Option::is_none")]
    commitment_opened: Option<&'static str>,
    validation_seal: &'a str,
    sealed_as: &'static str,
    does_not_show: &'static str,
    seal_rule: &'static str,
}

/// `validate_series` given `solves`: one published number of each solve's
/// result, compared with the caller's measurements, each at a row of the plan.
pub(crate) async fn validate(state: &AppState, request: Value) -> Result<String, DoorError> {
    let object = request
        .as_object()
        .ok_or_else(|| violation("the request must be a JSON object."))?;
    for key in object.keys() {
        if !COMPARISON_KEYS.contains(&key.as_str()) && key != genesis_seal::commitment::COMMITMENT {
            return Err(violation(format!(
                "'{key}' is not a field of a comparison of solves. Its fields are solves, quantity and measurements, and commitment when the measurements were committed."
            )));
        }
    }
    for key in COMPARISON_KEYS {
        if !object.contains_key(*key) {
            return Err(missing(key));
        }
    }
    let normalized = dataset::normalize(object["solves"].clone())?;
    if normalized.form_given {
        return Err(violation(
            "'solves.answer' is not a field of a comparison: the comparison answers with its numbers and not with the rows.",
        ));
    }
    if !normalized.is_solves() {
        return Err(violation("'solves' must be a plan of solves: solver and plan, as run_dataset takes it."));
    }
    let plan = read(state.view(), normalized.request)?;
    let quantity = match &object["quantity"] {
        Value::String(text) if !text.is_empty() => text.clone(),
        Value::String(_) => return Err(missing("quantity")),
        _ => return Err(violation("'quantity' must be a string.")),
    };
    let solver = plan.solver.name;
    match (quantity.ends_with("_hex"), plan.solver.published_shape("result", &quantity)) {
        (false, Some(Shape::Number)) => {}
        (false, Some(_)) => {
            return Err(violation(format!(
                "'quantity' is '{quantity}', an array. A comparison of solves takes one number of the result."
            )));
        }
        _ => {
            return Err(violation(format!(
                "'quantity' is '{quantity}', which the {solver} solver does not publish as a number of its result."
            )));
        }
    }
    let items = object["measurements"]
        .as_array()
        .ok_or_else(|| violation("'measurements' must be a JSON array."))?;
    if items.is_empty() {
        return Err(violation("'measurements' must contain at least one measurement."));
    }
    let planned = plan.rows.len() as u64;
    let mut measured_rows = Vec::with_capacity(items.len());
    let mut values = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let label = format!("measurements[{index}]");
        let item = item
            .as_object()
            .ok_or_else(|| violation(format!("'{label}' must be a JSON object.")))?;
        for key in item.keys() {
            if key != "row" && key != "value" && key != crate::signed_load::SIGNED {
                return Err(violation(format!(
                    "'{label}.{key}' is not a field of a measurement of solves. Its fields are row and value, and signed when an instrument signed it."
                )));
            }
        }
        for key in ["row", "value"] {
            if !item.contains_key(key) {
                return Err(missing(&format!("{label}.{key}")));
            }
        }
        let row = item["row"]
            .as_u64()
            .ok_or_else(|| violation(format!("'{label}.row' must be an integer >= 0.")))?;
        if let Some(&before) = measured_rows.last() {
            if row <= before {
                return Err(violation(format!("'{label}.row' must be greater than the row before it.")));
            }
        }
        if row >= planned {
            return Err(violation(format!(
                "'{label}.row' is {row}, and the plan has {planned} rows, numbered from 0."
            )));
        }
        let value = item["value"]
            .as_f64()
            .ok_or_else(|| violation(format!("'{label}.value' must be a number.")))?;
        crate::accept_f64(&format!("{label}.value"), value).map_err(DoorError::Unprocessable)?;
        measured_rows.push(row);
        values.push(value);
    }
    let measurements = object["measurements"].clone();
    // A commitment is opened by the measurements as sent, before a permit.
    let commitment = crate::validation::open_commitment(object, &quantity, &measurements)?;
    // A signed measurement is verified before the solves take a permit.
    let readings = crate::signed_load::verify_measurements(&measurements)?;
    let measurement_source =
        (!readings.is_empty()).then(|| crate::signed_load::measurement_source(&measurements, &readings));
    let (_, sealed) = solve_and_seal(state, plan).await?;

    let hex_key = format!("{quantity}_hex");
    let mut compared_rows = Vec::new();
    let mut model = Vec::new();
    let mut measured = Vec::new();
    let mut not_reached_rows = Vec::new();
    for (row, value) in measured_rows.iter().zip(&values) {
        let exact = sealed.solutions[*row as usize]
            .as_ref()
            .filter(|solution| solution.document.outcome == "converged")
            .and_then(|solution| solution.document.result.as_ref())
            .and_then(|result| result.get(&hex_key))
            .and_then(Value::as_str)
            .and_then(|text| u64::from_str_radix(text, 16).ok())
            .map(f64::from_bits);
        match exact {
            Some(model_value) => {
                compared_rows.push(*row);
                model.push(model_value);
                measured.push(*value);
            }
            None => not_reached_rows.push(*row),
        }
    }
    let stats: Option<Statistics> = statistics(&compared_rows, &model, &measured)
        .map_err(|error| DoorError::Unprocessable(error.to_string()))?;
    let model_values_hex: Vec<String> = model.iter().map(|value| bits_hex(*value)).collect();
    let hash = measurements_sha384(&measurements);
    let counts = &sealed.head.counts;
    let engine_version = crate::solvers::engine_version();
    let seal = validation_seal(&SealParts {
        run_seal: &sealed.head.dataset_seal,
        measurements_sha384: &hash,
        quantity: &quantity,
        component: None,
        outcome: "solves",
        steps_landed: counts.completed,
        reject: None,
        compared: compared_rows.len() as u64,
        not_reached: not_reached_rows.len() as u64,
        statistics: stats.as_ref(),
        model_values_hex: &model_values_hex,
        commitment: commitment.as_ref(),
    });
    let document = serde_json::to_string(&Comparison {
        inputs: &request,
        engine_version: &engine_version,
        dataset_seal: &sealed.head.dataset_seal,
        solves: counts.rows,
        converged: counts.completed,
        refused: counts.refused,
        unadmitted: counts.unadmitted,
        compared: compared_rows.len() as u64,
        not_reached: not_reached_rows.len() as u64,
        not_reached_rows: &not_reached_rows,
        rms_error: stats.map(|s| s.rms_error),
        rms_error_hex: stats.map(|s| s.rms_error_hex()),
        mean_signed_error: stats.map(|s| s.mean_signed_error),
        mean_signed_error_hex: stats.map(|s| s.mean_signed_error_hex()),
        max_abs_error: stats.map(|s| s.max_abs_error),
        max_abs_error_hex: stats.map(|s| s.max_abs_error_hex()),
        max_abs_error_row: stats.map(|s| s.max_abs_error_step),
        model_values: model.iter().map(|value| crate::readout::Exact(*value)).collect(),
        model_values_hex: &model_values_hex,
        measurement_source,
        measurements_sha384: &hash,
        commitment_opened: commitment.as_ref().map(|_| genesis_seal::commitment::OPENED),
        validation_seal: &seal,
        sealed_as: SOLVES_SEALED_AS,
        does_not_show: DOES_NOT_SHOW,
        seal_rule: crate::SEAL_RULE,
    })
    .map_err(encode_failed)?;
    let receipt = crate::digest_hex(document.as_bytes());
    Ok(crate::append_receipt(&document, &receipt))
}

/// One worked plan of solves for a caller of `view`: a list holding the
/// worked solve of the atom when the view holds it, else of the view's first
/// solver. A view with no solver has none.
pub(crate) fn worked_plan_in(view: &View) -> Option<Value> {
    let listed = view.solvers();
    let solver = listed
        .iter()
        .find(|solver| solver.name == "atom_dft")
        .or_else(|| listed.first())?;
    let example = crate::solvers::example_solve(solver);
    Some(serde_json::json!({
        "solver": solver.name,
        "plan": { "kind": "list", "solves": [{ "parameters": example["parameters"] }] }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn list(solver: &str, rows: Vec<Value>) -> Value {
        let solves: Vec<Value> = rows.into_iter().map(|parameters| json!({ "parameters": parameters })).collect();
        json!({ "solver": solver, "plan": { "kind": "list", "solves": solves } })
    }

    /// The least and the most counted before solving, and the answer made.
    fn counted_and_made(view: &View, request: Value, form: Form) -> (u64, u64, u64) {
        let plan = read(view, request).expect("a plan");
        let (least, most) = summary_bounds(&plan, &[]);
        let sealed = execute(view, &plan).expect("solved");
        let made = answer_text(sealed, form, &[]).expect("encoded").len() as u64;
        (least, made, most)
    }

    /// The red team of 0.13.0, D3: the count before solving is a floor and
    /// a ceiling of the answer, and is never below it. For every solver: its
    /// worked example alone and twice beside a row turned away, in the
    /// summary; and every refusal name a solver lists is under the longest
    /// name the count allows a row turned away.
    #[test]
    fn a_plan_of_solves_is_counted_between_its_least_and_its_most() {
        let view = View::full();
        for solver in view.solvers() {
            for refusal in solver.listed_refusals() {
                assert!(refusal.name.len() <= REFUSAL_NAME_MOST, "{} {}", solver.name, refusal.name);
            }
            let example = (solver.example)();
            for rows in [vec![example.clone()], vec![example.clone(), json!({}), example.clone()]] {
                let (least, made, most) = counted_and_made(&view, list(solver.name, rows.clone()), Form::Summary);
                assert!(least <= made && made <= most, "{}: {least} <= {made} <= {most}", solver.name);
                let (_, whole, _) = counted_and_made(&view, list(solver.name, rows), Form::Full);
                assert!(least <= whole, "{}: {least} <= {whole} whole", solver.name);
            }
        }
    }

    /// The red team's plans: 50 solves of the elastic plate, the open
    /// connection's count of it, come to about 36,750 characters and are
    /// answered; 64 of propeller_apc_10x7e, the engine's own count on every
    /// connection, come to about 32,000 and are answered; and 64 of
    /// blast_wave_tnt_air come to more than the limit, counted once solved.
    /// Each made answer lies between the least and the most counted before.
    #[test]
    fn the_open_connections_counts_are_answered_and_a_plan_over_the_limit_is_counted_once_made() {
        let view = View::full();
        let elastic: Vec<Value> =
            (0..50).map(|i| json!({ "traction_mpa": 106.26 + f64::from(i) * 0.01, "points_x_mm": [], "points_y_mm": [] })).collect();
        let propeller = vec![(view.solvers().into_iter().find(|s| s.name == "propeller_apc_10x7e").unwrap().example)(); 64];
        let blast = vec![(view.solvers().into_iter().find(|s| s.name == "blast_wave_tnt_air").unwrap().example)(); 64];
        for (solver, rows, fits) in [
            ("stress_plate_hole_s235_elastic", elastic, true),
            ("propeller_apc_10x7e", propeller, true),
            ("blast_wave_tnt_air", blast, false),
        ] {
            let (least, made, most) = counted_and_made(&view, list(solver, rows), Form::Summary);
            assert!(least <= made && made <= most, "{solver}: {least} <= {made} <= {most}");
            assert!(least <= MAX_INLINE_ANSWER_CHARS, "{solver}: {least}");
            assert_eq!(made <= MAX_INLINE_ANSWER_CHARS, fits, "{solver}: {made}");
        }
    }

    /// The fix pass of 0.14.0 (INTEGRATION_0.14.md, note 3): engine 0.14.0
    /// pairs kernel 0.12.0 with solvers 0.12.0, not with the solvers one
    /// minor version before, so a later engine that moves both crates must
    /// still name 0.14.0's pair for a plan's seal of 0.14.0. This engine moves
    /// both to 0.13.0, a pair off the rule as well. Read here as an engine
    /// writing solvers 0.14.0 in its counts and its solution seals would read
    /// it, as plans of solves are counted since the work orders of 18 October
    /// 2026; and every pair this engine and the served ones wrote is named.
    #[test]
    fn a_later_engine_names_the_version_pair_of_0_14_0_for_a_plan_seal() {
        let view = View::full();
        let example = (view.solvers().into_iter().find(|s| s.name == "propeller_apc_10x7e").unwrap().example)();
        let plan = read(&view, list("propeller_apc_10x7e", vec![example])).expect("a plan");
        let sealed = execute(&view, &plan).expect("solved");
        let (later_kernel, later_solvers) = ("genesis_solvers_v0.14.0", "genesis_solvers_v0.14.0");
        for (kernel, solvers) in [
            ("genesis_kernel_v0.12.0", "genesis_solvers_v0.12.0"),
            ("genesis_kernel_v0.12.0", "genesis_solvers_v0.11.0"),
            ("genesis_kernel_v0.9.0", "genesis_solvers_v0.8.0"),
            ("genesis_kernel_v0.8.0", "genesis_solvers_v0.7.0"),
        ] {
            let given = seal_under(&plan, &sealed, kernel, solvers).expect("a seal under that pair");
            let said = version_words_of(&plan, &sealed, &given, later_kernel, later_solvers);
            assert!(
                said.contains(&format!("with engine_version {kernel} in its counts and {solvers} in its solution seals, another engine version's, and this engine writes {later_kernel} in its counts and {later_solvers} in its solution seals")),
                "{kernel} {solvers}: {said}"
            );
        }
        // This engine's own pair is one a later engine tries.
        let ours = (crate::solvers::engine_version(), crate::solvers::engine_version());
        assert!(version_pairs(later_kernel, later_solvers).contains(&ours), "{ours:?}");
        // The seals this engine writes are as before: its own pair holds.
        assert_eq!(seal_under(&plan, &sealed, &ours.0, &ours.1).as_deref(), Some(sealed.head.dataset_seal.as_str()));
    }

    /// A plan whose request alone is longer than the limit is refused before
    /// anything is solved, with the least and the most it can come to.
    #[test]
    fn a_plan_whose_least_is_over_the_limit_is_refused_before_solving() {
        let view = View::full();
        let ends: Vec<f64> = (1..=32).map(|i| f64::from(i) * 9.7 + 0.1).collect();
        let row = json!({
            "biomass_g_per_l": 0.9, "glucose_g_per_l": 20.8, "galacturonate_g_per_l": 4.6, "yeast_extract_g_per_l": 2.0,
            "ph": 3.5, "feed_glucose_g_per_l": 32.0, "feed_galacturonate_g_per_l": 94.0,
            "segment_end_h": ends, "feed_ml": vec![0.1; 32], "water_ml": vec![-0.1; 32], "sample_ml": vec![0.3; 32],
        });
        let plan = read(&view, list("culture_trichoderma_atp", vec![row; 64])).expect("a plan");
        let (least, most) = summary_bounds(&plan, &[]);
        assert!(least > MAX_INLINE_ANSWER_CHARS && least < most, "{least} {most}");
        let sentence = too_large_before(least, most, Form::Summary);
        assert!(sentence.contains(&format!("at least {least} characters and at most {most}, more than the 42500")), "{sentence}");
        assert!(sentence.ends_with("nothing was solved. To make it smaller: send fewer solves."), "{sentence}");
        let whole = too_large_before(least, most, Form::Full);
        assert!(whole.contains(&format!("at least {least} characters, more than")) && whole.ends_with("or send fewer solves."), "{whole}");
    }
}
