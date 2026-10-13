//! A plan of solves, and the comparison of one result of those solves with a
//! caller's measurements.
//!
//! A plan of solves is a `run_dataset` request that names a solver in place
//! of a card: a list of solve requests, or a grid of explicit values over a
//! base request. Every row is an ordinary `solve` request, solved by the code
//! `solve` uses and sealed with the `solution_seal` it carries alone. The
//! rows are sealed under one dataset seal by the rules of `genesis_seal`: the
//! plan hash is SHA-256 of the sealed form of the request, the leaf of a row
//! is the compact JSON of `row`, `request_sha256`, `outcome` and the row's
//! seal, the leaves form the row tree of RFC 6962, and `dataset_seal` is
//! SHA-256 of the plan hash, the root and the compact JSON of the counts.
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
    DOES_NOT_SHOW, SealParts, Statistics, bits_hex, measurements_sha256, statistics, validation_seal,
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
pub(crate) const CHECK_SOLVES_BY_REQUEST: &str = "Physical Violation: the head is that of a plan of solves, whose rows are solutions and are not checked by parts. Check it by sending its run_dataset request as 'request' with its 'dataset_seal': the solves are solved again and the seal compared.";

/// What a summary answer of a plan of solves says of itself.
pub(crate) const SOLVES_SUMMARY_ABOUT: &str = "One summary row per solve: its solution_seal, outcome, refusal name and every single number of its result in decimals, and for a grid the values the row sets. The head and the dataset_seal are those of the full rows. Send the request again with \"answer\": \"full\" for every solution whole, or check the seal with verify_dataset given this request and the dataset_seal.";

/// How the seal of a comparison of solves is taken, said in its answer.
pub(crate) const SOLVES_SEALED_AS: &str = "validation_seal is the validation seal of genesis_seal, taken with the dataset_seal of the solves in the place of a run seal, each measured row number in the place of a step, outcome \"solves\", steps_landed the count of solves that converged and reject null. The statistics and measurements_sha256 follow the rules of a run's comparison. A measured row whose solve did not converge is not compared and is counted in not_reached. genesis_seal verify-validation reads this comparison as it reads a run's, from the answer alone; the same request sent again gives the same seal.";

const PLAN_FIELDS: &str = "Its fields are solver, plan and answer.";

fn violation(text: impl std::fmt::Display) -> DoorError {
    DoorError::Unprocessable(format!("Physical Violation: {text}"))
}

fn missing(key: &str) -> DoorError {
    DoorError::Unprocessable(format!("Missing required parameter: '{key}'"))
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
        .ok_or_else(|| DoorError::NotFound("Solver not recognized".to_string()))?;
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

/// The leaf of a row: the compact JSON of `row`, `request_sha256`, `outcome`
/// and `solution_seal`, in that order.
fn leaf_of(row: u64, request: &Value, outcome: &str, seal: Option<&str>) -> Value {
    let mut leaf = Map::new();
    leaf.insert("row".to_string(), Value::from(row));
    leaf.insert("request_sha256".to_string(), Value::from(digest_hex(sealed_form(request).as_bytes())));
    leaf.insert("outcome".to_string(), Value::from(outcome));
    leaf.insert("solution_seal".to_string(), seal.map_or(Value::Null, Value::from));
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
        engine_version: crate::ENGINE_VERSION.to_string(),
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
        let mut brief = Map::new();
        brief.insert("row".to_string(), Value::from(index));
        if let Some(set) = dataset::set_of(&plan.paths, request) {
            brief.insert("set".to_string(), set);
        }
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
                brief.insert("outcome".to_string(), Value::from(outcome));
                brief.insert("reject".to_string(), response.document.reject.clone().map_or(Value::Null, Value::from));
                brief.insert("solution_seal".to_string(), Value::from(response.document.solution_seal.as_str()));
                brief.insert(
                    "result".to_string(),
                    response.document.result.as_ref().map_or(Value::Null, numbers_of),
                );
                leaf_of(index, request, outcome, Some(&response.document.solution_seal))
            }
            Err(sentence) => {
                counts.unadmitted += 1;
                for key in ["outcome", "iterations", "reject", "reject_detail", "result", "invariants", "field_sha256", "seal_parts", "solution_seal"] {
                    row.insert(key.to_string(), Value::Null);
                }
                row.insert("outcome".to_string(), Value::from("unadmitted"));
                row.insert("reject_detail".to_string(), Value::from(sentence.as_str()));
                brief.insert("outcome".to_string(), Value::from("unadmitted"));
                brief.insert("reject".to_string(), dataset::name_in_brackets(sentence).map_or(Value::Null, Value::from));
                brief.insert("solution_seal".to_string(), Value::Null);
                brief.insert("result".to_string(), Value::Null);
                leaf_of(index, request, "unadmitted", None)
            }
        };
        let leaf_text = serde_json::to_string(&leaf).map_err(encode_failed)?;
        leaves.push(tree::leaf_hash(leaf_text.as_bytes()));
        row.insert("leaf".to_string(), leaf);
        let row = Value::Object(row);
        rows_text.extend_from_slice(serde_json::to_string(&row).map_err(encode_failed)?.as_bytes());
        rows_text.push(b'\n');
        rows.push(row);
        summary.push(Value::Object(brief));
        solutions.push(solved.ok());
    }
    let plan_sha256 = digest_hex(sealed_form(&plan.request).as_bytes());
    let merkle_root = hex::encode(tree::root(&leaves).expect("a plan has at least one row"));
    let seal = dataset_seal(&plan_sha256, &merkle_root, &counts);
    let head = Head {
        request: plan.request.clone(),
        counts,
        plan_sha256,
        merkle_root,
        dataset_seal: seal,
        rows_sha256: Some(digest_hex(&rows_text)),
        frames_sha256: None,
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

/// The characters a summary of `plan` can come to, counted before anything
/// is solved: the head with the whole request, and for each row its fixed
/// part, the values it sets and every single number the solver publishes in
/// its result.
fn summary_chars(plan: &SolvePlan) -> u64 {
    let request_chars = plan.request.to_string().len() as u64;
    let numbers: u64 = plan
        .solver
        .result
        .iter()
        .filter(|output| matches!(output.shape, Shape::Number))
        .map(|output| (output.each.len().max(1) as u64) * (output.name.len() as u64 + 16 + 28))
        .sum();
    let mut counted = request_chars + 900 + SOLVES_SUMMARY_ABOUT.len() as u64;
    for row in &plan.rows {
        let set = dataset::set_of(&plan.paths, row).map_or(0, |set| set.to_string().len() as u64 + 8);
        counted = counted.saturating_add(300 + numbers + set);
    }
    counted
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
    let counted = summary_chars(&plan);
    if counted > MAX_INLINE_ANSWER_CHARS {
        return Err(DoorError::Unprocessable(too_large(counted, form, true)));
    }
    let (_, sealed) = solve_and_seal(state, plan).await?;
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
    dataset::note_replaced(&mut body, &replaced);
    let text = serde_json::to_string(&Value::Object(body)).map_err(encode_failed)?;
    if form == Form::Full && text.len() as u64 > MAX_INLINE_ANSWER_CHARS {
        return Err(DoorError::Unprocessable(too_large(text.len() as u64, form, false)));
    }
    Ok(text)
}

/// The refusal of an answer of a plan of solves above the limit, with what
/// to change. A summary is counted before anything is solved. The whole
/// solutions of a solver that publishes arrays are not known in length until
/// they are solved, so a full answer is counted when it is made.
fn too_large(counted: u64, form: Form, before: bool) -> String {
    let mut text = format!(
        "Physical Violation: the answer would come to about {counted} characters, more than the {MAX_INLINE_ANSWER_CHARS} one inline answer may have."
    );
    if before {
        text.push_str(" It is counted from the plan before anything is solved, and nothing was solved.");
    } else {
        text.push_str(" The whole solutions were counted once they were solved, and none is returned.");
    }
    text.push_str(" To make it smaller:");
    if form == Form::Full {
        text.push_str(" leave out 'answer' for one summary row per solve, with its seal, outcome and every single number of its result in decimals;");
    }
    text.push_str(" or send fewer solves.");
    text
}

/// The dataset seal a plan of solves comes to, worked out again for a check
/// by request. Nothing is stored or read from storage.
pub(crate) async fn recompute_seal(state: &AppState, normalized: Normalized) -> Result<String, DoorError> {
    let plan = read(state.view(), normalized.request)?;
    let (_, sealed) = solve_and_seal(state, plan).await?;
    Ok(sealed.head.dataset_seal)
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
    measurements_sha256: &'a str,
    validation_seal: &'a str,
    sealed_as: &'static str,
    does_not_show: &'static str,
}

/// `validate_series` given `solves`: one published number of each solve's
/// result, compared with the caller's measurements, each at a row of the plan.
pub(crate) async fn validate(state: &AppState, request: Value) -> Result<String, DoorError> {
    let object = request
        .as_object()
        .ok_or_else(|| violation("the request must be a JSON object."))?;
    for key in object.keys() {
        if !COMPARISON_KEYS.contains(&key.as_str()) {
            return Err(violation(format!(
                "'{key}' is not a field of a comparison of solves. Its fields are solves, quantity and measurements."
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
            if key != "row" && key != "value" {
                return Err(violation(format!(
                    "'{label}.{key}' is not a field of a measurement of solves. Its fields are row and value."
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
    let hash = measurements_sha256(&measurements);
    let counts = &sealed.head.counts;
    let seal = validation_seal(&SealParts {
        run_seal: &sealed.head.dataset_seal,
        measurements_sha256: &hash,
        quantity: &quantity,
        component: None,
        outcome: "solves",
        steps_landed: counts.completed,
        reject: None,
        compared: compared_rows.len() as u64,
        not_reached: not_reached_rows.len() as u64,
        statistics: stats.as_ref(),
        model_values_hex: &model_values_hex,
    });
    let document = serde_json::to_string(&Comparison {
        inputs: &request,
        engine_version: crate::ENGINE_VERSION,
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
        measurements_sha256: &hash,
        validation_seal: &seal,
        sealed_as: SOLVES_SEALED_AS,
        does_not_show: DOES_NOT_SHOW,
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
