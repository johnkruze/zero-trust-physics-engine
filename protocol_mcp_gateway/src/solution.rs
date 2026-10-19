//! One sealed solver result, and a check of the seal it carries.
//!
//! A solver refines a whole field. The seal binds the request, every output
//! number bit for bit, a hash of the final field, and the outcome. A solution
//! can be checked without the engine.
//!
//! Nothing here names a solver. The request is read against the field list of
//! the solver it names, and the entry listed for that solver in `solvers.rs`
//! makes the call.

use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::DoorError;
use crate::solvers::{self, Given, GivenValue, Input, Kind, Published, Shape, Solved, Solver};

pub(crate) const TOP_LEVEL: &[&str] = &["solver", "parameters"];

/// The key of a solve request that names a worked example in place of
/// `parameters`.
pub(crate) const EXAMPLE: &str = "example";

/// The answer to a solve request that names its solver and neither
/// `parameters` nor `example`.
pub(crate) const MISSING_PARAMETERS: &str =
    "Missing required parameter: 'parameters' (or 'example', the name of a worked example describe_solver lists) (InputMissing).";

/// A solve request that names a worked example (`{"solver": ..., "example":
/// "<name>"}`, a name `describe_solver` lists) made into the request it
/// stands for, `{"solver": ..., "parameters": <the example's parameters>}`.
/// That filled request is the one solved, sealed and returned as `inputs`.
/// Any other request is returned unchanged.
pub(crate) fn filled(view: &crate::view::View, request: Value) -> Result<Value, DoorError> {
    let Some(object) = request.as_object() else {
        return Ok(request);
    };
    let Some(example) = object.get(EXAMPLE) else {
        return Ok(request);
    };
    let name = match required(object, "solver")? {
        Value::String(text) => text.as_str(),
        _ => return Err(wrong_type("solver", "a string")),
    };
    let solver = Some(name)
        .filter(|name| view.sees_solver(name))
        .and_then(solvers::find)
        .ok_or_else(|| DoorError::NotFound(crate::SOLVER_NOT_RECOGNIZED.to_string()))?;
    if object.contains_key("parameters") {
        return Err(DoorError::Unprocessable(
            "Physical Violation: a solve request takes 'parameters' or 'example', not both: 'example' fills 'parameters' with a worked example whole.".to_string(),
        ));
    }
    reject_unknown_keys(solver, object, ["solver", EXAMPLE].into_iter())?;
    let Value::String(example) = example else {
        return Err(wrong_type(EXAMPLE, "a string, the name of a worked example describe_solver lists"));
    };
    let parameters = solvers::example_parameters(solver, example).ok_or_else(|| {
        let names: Vec<String> = solvers::example_names(solver).iter().map(|name| format!("\"{name}\"")).collect();
        DoorError::Unprocessable(format!(
            "Physical Violation: 'example' must be the name of a worked example of {}: {}; this request has {}.",
            solver.name,
            names.join(", "),
            Value::String(example.clone())
        ))
    })?;
    Ok(serde_json::json!({ "solver": solver.name, "parameters": parameters }))
}

const OUTCOME_CONVERGED: &str = "converged";
const OUTCOME_REFUSED: &str = "refused";

/// The ingredients of `solution_seal`, in the order they enter it. These are
/// also the names `POST /verify_solution` reports.
const PART_INPUTS: &str = "inputs";
const PART_VERSION: &str = "engine_version";
const PART_RESULT: &str = "result";
const PART_INVARIANTS: &str = "invariants";
const PART_FIELD: &str = "field_sha256";
const PART_OUTCOME: &str = "outcome";
const CAUSE_SEAL: &str = "solution_seal";

/// SHA-256 of each ingredient of `solution_seal`, taken alone. The seal does
/// not depend on these. They let a check name the ingredient that changed.
#[derive(Serialize, Clone)]
pub struct SealParts {
    pub inputs: String,
    pub engine_version: String,
    pub result: String,
    pub invariants: String,
    pub field_sha256: String,
    pub outcome: String,
}

/// Every response field except `receipt_sha256`.
#[derive(Serialize, Clone)]
pub struct SolutionDocument {
    pub inputs: Value,
    pub engine_version: String,
    pub outcome: String,
    pub reject: Option<String>,
    pub reject_detail: Option<String>,
    pub iterations: Option<u64>,
    #[serde(serialize_with = "crate::trajectory::serialize_optional_state")]
    pub result: Option<Value>,
    #[serde(serialize_with = "crate::trajectory::serialize_optional_state")]
    pub invariants: Option<Value>,
    pub field_sha256: Option<String>,
    /// Present only when the request's parameters carry an instrument's
    /// signature: whose it is. A solve answer without it is the caller's
    /// number. Outside the solution seal, which covers the signature in its
    /// inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_source: Option<Value>,
    pub seal_parts: SealParts,
    pub solution_seal: String,
    /// Present only when the request named its upstream in `from`: the
    /// chain commitment, outside the solution seal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<Value>,
}

#[derive(Serialize)]
pub struct SolutionResponse {
    #[serde(flatten)]
    pub document: SolutionDocument,
    pub receipt_sha256: String,
}

#[derive(Serialize)]
pub struct SolutionReport {
    pub holds: bool,
    /// `inputs`, `engine_version`, `result`, `invariants`, `field_sha256`,
    /// `outcome`, `solution_seal`, or null when the solution holds.
    pub cause: Option<String>,
    /// What the check of a whole solution compared, and what it did not.
    pub compared: &'static str,
}

/// What the long form of `verify_solution` says when it solved nothing: a
/// part of the seal that does not hold, a signed reading that does not
/// verify, another engine version, or inputs the engine turns away before
/// solving.
pub const WHOLE_SOLUTION_COMPARED: &str = "No physics was run. The solution seal and each of its parts were recomputed from the answer's exact hex values, and its signed readings verified; cause names the first that does not hold, an answer of another engine version (engine_version), or inputs the engine turns away before solving (answer_text, at inputs). The seal has no key, so a seal that holds shows only that the answer is consistent with itself: an answer is solved again from its request, and compared with the engine's answer byte for byte, only when all of these hold.";

#[derive(Serialize)]
struct SealTail<'a> {
    outcome: &'a str,
    iterations: Option<u64>,
    reject: Option<&'a str>,
}

pub async fn run(state: &crate::AppState, request: Value) -> Result<SolutionResponse, DoorError> {
    // `from` is not part of the solve: a chained solve is the same solve as
    // the request typed by hand, and its chain stands beside it.
    let (request, from) = crate::chain::take(request);
    let request = filled(state.view(), request)?;
    let prepared = prepare_in(state.view(), &request)?;
    if let Some(from) = from {
        let plan = crate::chain::read(state.view(), &request, from, crate::chain::Target::Solve, 0)?;
        return crate::on_pool(state, move || {
            let (settled, request) = plan.settle()?;
            let solved = (prepared.solver.solve)(&prepared.given)?;
            seal_with(prepared.solver, request, solved, Some(&|seal: &str| settled.chain_value(seal)))
        })
        .await;
    }
    crate::on_pool(state, move || {
        let solved = (prepared.solver.solve)(&prepared.given)?;
        seal(prepared.solver, request, solved)
    })
    .await
}

#[cfg(test)]
pub fn check(body: &Value) -> Result<SolutionReport, DoorError> {
    check_in(&crate::view::View::full(), body)
}

/// `verify_solution` as every connection answers it, as JSON text: a check by
/// request (`request` and `solution_seal`), which solves the request again
/// under a physics permit, or the check of a whole solution, which runs no
/// physics.
pub async fn verify(state: &crate::AppState, body: &Value, text: Option<&str>) -> Result<String, DoorError> {
    let object = body.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    let Some((request, given)) = crate::verify_by_request::read(object, CAUSE_SEAL, "the whole solve answer")? else {
        let report = long_form(state, body, text).await?;
        return serde_json::to_string(&report).map_err(|error| {
            DoorError::Internal(format!(
                "Physical Violation: the response could not be encoded. {error}"
            ))
        });
    };
    let chained = request.get(crate::chain::FROM).is_some();
    let response = run(state, request.clone()).await?;
    let version = (given != response.document.solution_seal).then(|| version_words(&response.document, given));
    let text = crate::verify_by_request::answer(
        CAUSE_SEAL,
        "solve",
        given,
        &response.document.solution_seal,
        Some(&response.receipt_sha256),
        &crate::verify_by_request::compared_with_receipt("solved", CAUSE_SEAL),
        crate::verify_by_request::TEXT_NOT_COMPARED,
        version,
    )?;
    if !chained {
        return Ok(text);
    }
    // A chained solve: every upstream run was computed again and held to its
    // seal, and each handed value to the upstream's sealed answer and to the
    // request, before the request was solved again.
    let mut report: Map<String, Value> = serde_json::from_str(&text).map_err(|error| {
        DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}"))
    })?;
    report.insert("chain_checked".into(), Value::String(CHAIN_CHECKED.to_string()));
    if let Some(chain) = &response.document.chain {
        report.insert("recomputed_chain_seal".into(), chain["chain_seal"].clone());
    }
    serde_json::to_string(&report).map_err(|error| {
        DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}"))
    })
}

/// What a check by request of a chained solve compared beside the solution
/// seal.
pub(crate) const CHAIN_CHECKED: &str = "Every upstream run in 'from' was computed again and its run_seal compared with the seal given, and each handed value compared bit for bit with the upstream's sealed answer and with the request, before the request was solved again; holds covers the solution seal and the upstream runs. recomputed_chain_seal is the chain seal of the solve computed again, which solve's answer carries in chain.chain_seal.";

/// What a check by request of a solution says when the seal given is not the
/// one computed: the seal covers `engine_version`, and, when the seal given
/// is this request's solution under another version's name, that version
/// beside this engine's (the red team of 0.13.0, D4).
fn version_words(document: &SolutionDocument, given: &str) -> String {
    let ours = &document.engine_version;
    let theirs = crate::verify_by_request::other_versions("genesis_solvers", ours)
        .into_iter()
        .find(|name| seal_with_version(document, name).as_deref() == Some(given));
    match theirs {
        Some(theirs) => format!(
            "The solution_seal covers engine_version: the seal given is this request's solution under engine_version {theirs}, another engine version's, and this engine writes {ours}, so it does not hold here. The long form, given that answer whole, compares it value by value and names both versions."
        ),
        None => format!(
            "The solution_seal covers engine_version, {ours} on this engine: the seal of an answer another engine version gave does not hold here."
        ),
    }
}

/// The solution seal of `document` with `version` in the place of its
/// engine version: what the same solution sealed by another version of the
/// solvers comes to.
pub(crate) fn seal_with_version(document: &SolutionDocument, version: &str) -> Option<String> {
    let ingredients = Ingredients {
        request: crate::sealed_form(&document.inputs).into_bytes(),
        version: version_text(version),
        result_text: values_text(PART_RESULT, document.result.as_ref().and_then(Value::as_object)).ok()?,
        invariants_text: values_text(PART_INVARIANTS, document.invariants.as_ref().and_then(Value::as_object)).ok()?,
        field: field_text(document.field_sha256.as_deref()),
        tail: tail_bytes(&document.outcome, document.iterations, document.reject.as_deref()),
    };
    Some(ingredients.seal())
}

/// What the long form of `verify_solution` answers: the seal recomputed,
/// then the answer the engine writes for it compared with the answer sent.
#[derive(Serialize)]
pub struct LongSolutionReport {
    /// Every byte of the answer sent is the engine's answer to its request.
    pub holds: bool,
    /// Every value of the answer sent is the engine's answer to its request,
    /// computed again, whatever its spelling: true beside a false holds when
    /// only the text differs (a number spelled another way, an escape).
    pub values_hold: bool,
    /// A part of the seal, `solution_seal`, `engine_version`, `answer_text`,
    /// or null when the answer holds.
    pub cause: Option<String>,
    /// With `answer_text` and `engine_version`: the first field where the
    /// answer sent is not the engine's. Left out when there is none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub differs_at: Option<String>,
    /// What is different, in one sentence. Left out when there is nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difference: Option<String>,
    pub compared: &'static str,
    /// A chained solve sent whole with its `from`: what was compared beside
    /// the answer. Left out of every other long form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_checked: Option<&'static str>,
    /// With `chain_checked`, when the answer holds: the chain seal of the
    /// solve computed again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recomputed_chain_seal: Option<Value>,
}

impl LongSolutionReport {
    fn of(holds: bool, cause: Option<&str>, compared: &'static str) -> Self {
        Self { holds, values_hold: holds, cause: cause.map(str::to_string), differs_at: None, difference: None, compared, chain_checked: None, recomputed_chain_seal: None }
    }

    /// The report of a comparison with the engine's answer.
    fn of_compared(found: crate::rebuild::Compared, compared: &'static str) -> Self {
        match found.difference {
            None => Self::of(true, None, compared),
            Some(difference) => Self { values_hold: found.values_hold, ..Self::differs(difference, compared) },
        }
    }

    fn differs(difference: crate::rebuild::Difference, compared: &'static str) -> Self {
        Self {
            differs_at: Some(difference.at),
            difference: Some(difference.says),
            ..Self::of(false, Some(crate::rebuild::CAUSE_ANSWER_TEXT), compared)
        }
    }
}

/// What the long form of `verify_solution` compared when its request was
/// solved again.
pub const WHOLE_SOLUTION_COMPUTED: &str = "The solution seal and each of its parts were recomputed from the answer's exact hex values. Then its request, the answer's inputs, was solved again under a physics permit and the limits of solve, the solution_seal of that solve was compared with the answer's, and the engine's whole answer to that request was compared with the answer sent: value by value, and then byte for byte, its words, decimals and receipt included. holds is true only when every one is the same, so an answer that holds is, every byte, the engine's answer to its request; cause names the first that is not, and answer_text, with differs_at, is a text that differs while the seal holds.";

/// Whether the long form of `verify_solution` solves again, read from the
/// answer alone: every whole answer is solved again once its seal holds.
/// The open connection counts it as a solve.
pub(crate) fn long_form_solves(answer: &Map<String, Value>) -> bool {
    !answer.contains_key(crate::verify_by_request::REQUEST)
}

/// The long form of `verify_solution`: the seal recomputed with no physics,
/// then the request the answer carries solved again under a physics permit,
/// and the engine's answer to it compared with the answer sent, value by
/// value and byte for byte. The seal has no key, so an answer whose parts
/// were rewritten with the seal worked out again holds every seal; only the
/// solve shows its values are the engine's.
async fn long_form(state: &crate::AppState, body: &Value, text: Option<&str>) -> Result<LongSolutionReport, DoorError> {
    // A chained solve carries the upstream's run seal and not its request:
    // it is checked whole only with the `from` solve took, sent beside it.
    let from = body.get(crate::chain::FROM).cloned();
    if from.is_none() && body.get("chain").is_some_and(|chain| !chain.is_null()) {
        return Err(chained_solve_alone());
    }
    let (stripped, stripped_text);
    let (body, text) = if from.is_some() {
        let mut object = body.as_object().cloned().unwrap_or_default();
        object.shift_remove(crate::chain::FROM);
        stripped = Value::Object(object);
        stripped_text = without_member(text, crate::chain::FROM);
        (&stripped, stripped_text.as_deref())
    } else {
        (body, text)
    };
    let seals = check_in(state.view(), body)?;
    if !seals.holds {
        return Ok(LongSolutionReport::of(false, seals.cause.as_deref(), WHOLE_SOLUTION_COMPARED));
    }
    let object = body.as_object().expect("a verified solution is an object");
    // The seal holds: a signature over the parameters is verified next, as a
    // run's loads are after its chain.
    let inputs = object.get("inputs").cloned().unwrap_or(Value::Null);
    if let Err(fault) = genesis_seal::attest::verify_all(&genesis_seal::attest::solve_places(&inputs)) {
        return Ok(LongSolutionReport {
            differs_at: Some(format!("inputs.{}.{}", fault.place().label(), crate::signed_load::SIGNED)),
            difference: Some(fault.to_string()),
            ..LongSolutionReport::of(false, Some(crate::trajectory::CAUSE_LOAD_SIGNATURE), WHOLE_SOLUTION_COMPARED)
        });
    }
    let served = solvers::engine_version();
    if let Some(Value::String(version)) = object.get(PART_VERSION) {
        if *version != served {
            return Ok(LongSolutionReport {
                differs_at: Some(PART_VERSION.to_string()),
                difference: Some(crate::rebuild::another_version(version, &served, CAUSE_SEAL)),
                ..LongSolutionReport::of(false, Some(crate::rebuild::CAUSE_ENGINE_VERSION), WHOLE_SOLUTION_COMPARED)
            });
        }
    }
    // The request is read as `solve` reads it, so a request the engine turns
    // away has no answer here either.
    let prepared = match prepare_in(state.view(), &inputs) {
        Ok(prepared) => prepared,
        Err(refused) => {
            return Ok(LongSolutionReport::differs(
                crate::rebuild::Difference::new(
                    "inputs",
                    format!("The engine solves nothing for these inputs: it turns them away before solving. {}", refused.message()),
                ),
                WHOLE_SOLUTION_COMPARED,
            ));
        }
    };
    if let Some(from) = from {
        drop(prepared);
        return chained_long_form(state, body, text, inputs, from).await;
    }
    let solved = crate::on_pool(state, move || Ok(prepared.solve_and_seal(inputs))).await?;
    let response = match solved {
        Ok(response) => response,
        Err(sentence) => {
            return Ok(LongSolutionReport::differs(
                crate::rebuild::Difference::new(
                    "outcome",
                    format!("The request was solved again and the engine answers it with no seal: {sentence}"),
                ),
                WHOLE_SOLUTION_COMPUTED,
            ));
        }
    };
    let sent_seal = object.get(CAUSE_SEAL).and_then(Value::as_str).unwrap_or_default();
    if response.document.solution_seal != sent_seal {
        return Ok(LongSolutionReport {
            difference: Some("The request was solved again and solves to another solution_seal: the answer is consistent with itself, and its sealed parts are not the engine's solution of this request.".to_string()),
            ..LongSolutionReport::of(false, Some(CAUSE_SEAL), WHOLE_SOLUTION_COMPUTED)
        });
    }
    let engine_text = serde_json::to_string(&response).map_err(|error| {
        DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}"))
    })?;
    let sent_text = crate::rebuild::sent_text(body, text);
    Ok(LongSolutionReport::of_compared(crate::rebuild::compare(&engine_text, body, &sent_text)?, WHOLE_SOLUTION_COMPUTED))
}

/// The cause of the long form of a chained solve whose chain does not hold:
/// the upstream run, a handed value or the coupling, named in its sentence.
pub(crate) const CAUSE_CHAIN: &str = "chain";

/// What the long form of a chained solve, sent whole with its `from`,
/// compared beside the answer.
pub(crate) const WHOLE_CHAIN_CHECKED: &str = "The upstream run in 'from' was computed again and its run_seal compared with the seal given, each handed value compared bit for bit with the upstream's sealed answer and with the answer's inputs, and the request solved again with that from; the whole answer, its chain and chain_seal included, was compared with the engine's answer byte for byte. recomputed_chain_seal is the chain seal of the solve computed again.";

/// The refusal of the long form of a chained solve sent alone: its answer
/// carries the upstream's run seal and not the upstream's request.
fn chained_solve_alone() -> DoorError {
    DoorError::Unprocessable(format!(
        "Physical Violation: this answer is a chained solve (its chain names upstream_run_seal), and the upstream's request is not in it, so the run it was handed from cannot be computed again here ({}). Send the whole answer with from, as solve took it (request, run_seal and coupling), beside it; or its request with from and its solution_seal, the short form.",
        crate::rebuild::NO_REQUEST
    ))
}

/// The text of a JSON object as it came with one member taken out, its other
/// members' texts kept byte for byte; None when there is no text or it is not
/// an object.
fn without_member(text: Option<&str>, key: &str) -> Option<String> {
    use serde::de::{Deserializer, MapAccess, Visitor};
    use serde_json::value::RawValue;
    struct Members<'a>(Vec<(String, &'a RawValue)>);
    impl<'de: 'a, 'a> serde::Deserialize<'de> for Members<'a> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Each<'a>(std::marker::PhantomData<&'a ()>);
            impl<'de: 'a, 'a> Visitor<'de> for Each<'a> {
                type Value = Members<'a>;
                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a JSON object")
                }
                fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                    let mut members = Vec::new();
                    while let Some((name, raw)) = map.next_entry::<String, &'de RawValue>()? {
                        members.push((name, raw));
                    }
                    Ok(Members(members))
                }
            }
            deserializer.deserialize_map(Each(std::marker::PhantomData))
        }
    }
    let members: Members = serde_json::from_str(text?).ok()?;
    let kept: Vec<String> = members
        .0
        .iter()
        .filter(|(name, _)| name != key)
        .map(|(name, raw)| format!("{}:{}", serde_json::to_string(name).unwrap_or_default(), raw.get()))
        .collect();
    Some(format!("{{{}}}", kept.join(",")))
}

/// The long form of a chained solve, its seal already held: the request the
/// answer carries solved again with the `from` sent beside it (each upstream
/// run computed again and held to its seal, each handed value to the
/// upstream's answer and to the request, as solve does), then the whole
/// answer compared with the engine's, byte for byte.
async fn chained_long_form(state: &crate::AppState, body: &Value, text: Option<&str>, inputs: Value, from: Value) -> Result<LongSolutionReport, DoorError> {
    let chained = |report: LongSolutionReport| LongSolutionReport { chain_checked: Some(WHOLE_CHAIN_CHECKED), ..report };
    let mut request = inputs;
    if let Some(object) = request.as_object_mut() {
        object.insert(crate::chain::FROM.into(), from);
    }
    let response = match run(state, request).await {
        Ok(response) => response,
        Err(DoorError::Unprocessable(sentence)) => {
            return Ok(chained(LongSolutionReport {
                differs_at: Some(crate::chain::FROM.to_string()),
                difference: Some(format!("The request was solved again with the from sent, and the chain does not hold: {sentence}")),
                ..LongSolutionReport::of(false, Some(CAUSE_CHAIN), WHOLE_SOLUTION_COMPUTED)
            }));
        }
        Err(other) => return Err(other),
    };
    let sent_seal = body.get(CAUSE_SEAL).and_then(Value::as_str).unwrap_or_default();
    if response.document.solution_seal != sent_seal {
        return Ok(chained(LongSolutionReport {
            difference: Some("The request was solved again and solves to another solution_seal: the answer is consistent with itself, and its sealed parts are not the engine's solution of this request.".to_string()),
            ..LongSolutionReport::of(false, Some(CAUSE_SEAL), WHOLE_SOLUTION_COMPUTED)
        }));
    }
    let engine_text = serde_json::to_string(&response).map_err(|error| {
        DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}"))
    })?;
    let sent_text = crate::rebuild::sent_text(body, text);
    let report = chained(LongSolutionReport::of_compared(crate::rebuild::compare(&engine_text, body, &sent_text)?, WHOLE_SOLUTION_COMPUTED));
    if !report.holds {
        return Ok(report);
    }
    let seal = response.document.chain.as_ref().map(|chain| chain["chain_seal"].clone());
    Ok(LongSolutionReport { recomputed_chain_seal: seal, ..report })
}

pub fn check_in(view: &crate::view::View, body: &Value) -> Result<SolutionReport, DoorError> {
    let object = body.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    if let Some(meant) = crate::verify_by_request::meant_by_request(object, CAUSE_SEAL, "seal_parts") {
        return Err(meant);
    }
    let inputs = required(object, "inputs")?;
    let engine_version = match required(object, PART_VERSION)? {
        Value::String(text) => text.as_str(),
        _ => return Err(wrong_type(PART_VERSION, "a string")),
    };
    let outcome = match required(object, "outcome")? {
        Value::String(text) => text.as_str(),
        _ => return Err(wrong_type("outcome", "a string")),
    };
    let iterations = match required(object, "iterations")? {
        Value::Null => None,
        value => Some(
            value
                .as_u64()
                .ok_or_else(|| wrong_type("iterations", "an integer >= 0 or null"))?,
        ),
    };
    let reject = optional_string(object, "reject")?;
    let result = optional_object(object, "result")?;
    let invariants = optional_object(object, "invariants")?;
    let field = optional_string(object, PART_FIELD)?;
    let posted = required(object, "seal_parts")?
        .as_object()
        .ok_or_else(|| wrong_type("seal_parts", "a JSON object"))?;
    let solution_seal = match required(object, "solution_seal")? {
        Value::String(text) => text.as_str(),
        _ => return Err(wrong_type("solution_seal", "a string")),
    };
    let mut posted_parts = Vec::with_capacity(6);
    for part in [
        PART_INPUTS,
        PART_VERSION,
        PART_RESULT,
        PART_INVARIANTS,
        PART_FIELD,
        PART_OUTCOME,
    ] {
        match posted.get(part) {
            Some(Value::String(text)) => posted_parts.push((part, text.as_str())),
            Some(_) => return Err(wrong_type("seal_parts", "six strings")),
            None => {
                return Err(DoorError::Unprocessable(format!(
                    "Missing required parameter: 'seal_parts.{part}' (InputMissing)."
                )));
            }
        }
    }
    let solver = inputs
        .get("solver")
        .and_then(Value::as_str)
        .filter(|name| view.sees_solver(name))
        .and_then(solvers::find);
    require_exact_solution(solution_seal, field, &posted_parts, solver, result, invariants)?;

    let texts = match (
        values_text(PART_RESULT, result),
        values_text(PART_INVARIANTS, invariants),
    ) {
        (Err(_), _) => return Ok(fails(PART_RESULT)),
        (_, Err(_)) => return Ok(fails(PART_INVARIANTS)),
        (Ok(result_text), Ok(invariants_text)) => (result_text, invariants_text),
    };
    let ingredients = Ingredients {
        request: crate::sealed_form(inputs).into_bytes(),
        version: version_text(engine_version),
        result_text: texts.0,
        invariants_text: texts.1,
        field: field_text(field),
        tail: tail_bytes(outcome, iterations, reject),
    };
    let parts = ingredients.parts();
    let recomputed = [
        &parts.inputs,
        &parts.engine_version,
        &parts.result,
        &parts.invariants,
        &parts.field_sha256,
        &parts.outcome,
    ];
    for ((part, posted_digest), digest) in posted_parts.iter().zip(recomputed) {
        if posted_digest != digest {
            return Ok(fails(part));
        }
    }
    if ingredients.seal() != solution_seal {
        return Ok(fails(CAUSE_SEAL));
    }
    Ok(SolutionReport {
        holds: true,
        cause: None,
        compared: WHOLE_SOLUTION_COMPARED,
    })
}

/// One row of a plan of solves, solved on the calling thread, which holds the
/// plan's physics permit. `Ok(Err(sentence))` is a request the door or the
/// solver turned away without a seal, in the words `solve` answers it with.
/// `Err` is a fault in sealing a solution, which stops the plan.
pub(crate) fn solve_in_plan(
    view: &crate::view::View,
    request: Value,
) -> Result<Result<SolutionResponse, String>, String> {
    let prepared = match prepare_in(view, &request) {
        Ok(prepared) => prepared,
        Err(error) => return Ok(Err(error.message().to_string())),
    };
    let solved = match (prepared.solver.solve)(&prepared.given) {
        Ok(solved) => solved,
        Err(sentence) => return Ok(Err(sentence)),
    };
    seal(prepared.solver, request, solved).map(Ok)
}

/// A request that may reach its solver: the solver's entry, and every value
/// of `parameters` read and inside its bounds.
pub(crate) struct Prepared {
    solver: &'static Solver,
    given: Given,
}

impl Prepared {
    /// The solve and its seal, as `solve` makes them for the same request.
    /// It runs holding a physics permit.
    pub(crate) fn solve_and_seal(self, request: Value) -> Result<SolutionResponse, String> {
        let solved = (self.solver.solve)(&self.given)?;
        seal(self.solver, request, solved)
    }
}

/// The request as the solver takes it, or the reason it never reaches the
/// solver.
#[cfg(test)]
pub(crate) fn prepare(request: &Value) -> Result<Prepared, DoorError> {
    prepare_in(&crate::view::View::full(), request)
}

pub(crate) fn prepare_in(view: &crate::view::View, request: &Value) -> Result<Prepared, DoorError> {
    let object = request.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    let name = match required(object, "solver")? {
        Value::String(text) => text.as_str(),
        _ => return Err(wrong_type("solver", "a string")),
    };
    let solver = Some(name)
        .filter(|name| view.sees_solver(name))
        .and_then(solvers::find)
        .ok_or_else(|| DoorError::NotFound(crate::SOLVER_NOT_RECOGNIZED.to_string()))?;
    reject_unknown_keys(solver, object, TOP_LEVEL.iter().copied())?;
    let parameters = object
        .get("parameters")
        .ok_or_else(|| DoorError::Unprocessable(MISSING_PARAMETERS.to_string()))?
        .as_object()
        .ok_or_else(|| wrong_type("parameters", "a JSON object"))?;
    // An instrument's signature over the parameters is verified before the
    // inputs are read, and the inputs are read without it: the solver takes
    // the reading and knows nothing of who signed it.
    let given = if parameters.contains_key(crate::signed_load::SIGNED) {
        crate::signed_load::verify_solve(request)?;
        let mut reading = parameters.clone();
        reading.shift_remove(crate::signed_load::SIGNED);
        read_parameters(solver, &reading)?
    } else {
        read_parameters(solver, parameters)?
    };
    Ok(Prepared { solver, given })
}

/// `parameters` read against the field list of `solver`: no key outside the
/// list, every key the request must give, no key that goes with a choice the
/// request did not make, each value held to its kind and its bounds, and then
/// the solver's own check of the limits that span fields.
pub(crate) fn read_parameters(
    solver: &Solver,
    parameters: &Map<String, Value>,
) -> Result<Given, DoorError> {
    reject_unknown_keys(solver, parameters, solver.inputs.iter().map(|input| input.name))?;
    let mut given = Given::new(Vec::with_capacity(solver.inputs.len()));
    for input in solver.inputs {
        if let Some((switch, word)) = input.only_when {
            // The switch is listed before this input. It holds no word when
            // it goes with a choice this request did not make itself.
            let chosen = given.word_if_given(switch);
            if chosen != Some(word) {
                if parameters.contains_key(input.name) {
                    let has = match chosen {
                        Some(chosen) => format!("'{switch}' \"{chosen}\""),
                        None => format!("no '{switch}'"),
                    };
                    return Err(DoorError::Unprocessable(format!(
                        "Physical Violation: '{}' is an input only when '{switch}' is \"{word}\"; this request has {has} ({}).",
                        input.name,
                        crate::INPUT_NOT_WITH_CHOICE
                    )));
                }
                continue;
            }
        }
        let value = required(parameters, input.name)?;
        given.hold(input.name, read_input(input, value)?);
    }
    (solver.check)(&given).map_err(DoorError::Unprocessable)?;
    Ok(given)
}

/// One value of `parameters`, held to the kind and the bounds of its input.
/// No value is moved to a bound. Where the entry's check asks the bounds, the
/// value is held to its kind here and to its bounds there.
fn read_input(input: &Input, given: &Value) -> Result<GivenValue, DoorError> {
    let name = input.name;
    let bounds_here = !input.asked_by_check;
    match &input.kind {
        Kind::Count {
            minimum,
            maximum,
            spoken,
        } => {
            let outside = |shown: String| {
                refused(
                    format!("{spoken} {shown} is outside {minimum} to {maximum}"),
                    input.refusal,
                )
            };
            match (given.as_u64(), given.as_i64()) {
                (Some(count), _) if (*minimum..=*maximum).contains(&count) => {
                    Ok(GivenValue::Count(count))
                }
                (Some(count), _) => Err(outside(count.to_string())),
                (None, Some(negative)) => Err(outside(negative.to_string())),
                // `-0`, an integer above `u64::MAX` and one below `i64::MIN`
                // are read as floats and their digits are not kept. Each is
                // outside every count range, so the range is the cause, and
                // the value is named as it was read, not as it was written.
                (None, None) if integer_held_as_float(given) => Err(refused(
                    format!(
                        "{spoken} read as {given} is outside {minimum} to {maximum}"
                    ),
                    input.refusal,
                )),
                (None, None) => Err(must_be(
                    name,
                    "an integer written as digits, with no decimal point and no exponent",
                    given,
                )),
            }
        }
        Kind::Number {
            minimum,
            maximum,
            unit,
        } => {
            let number = given
                .as_f64()
                .ok_or_else(|| must_be(name, "a number", given))?;
            let number = normal_number(name, number)?;
            if bounds_here && !(*minimum..=*maximum).contains(&number) {
                return Err(refused(
                    format!(
                        "'{name}' {given} is outside {}",
                        solvers::range_words(*minimum, *maximum, unit)
                    ),
                    input.refusal,
                ));
            }
            Ok(GivenValue::Number(number))
        }
        Kind::NumberAbove {
            above,
            maximum,
            unit,
        } => {
            let number = given
                .as_f64()
                .ok_or_else(|| must_be(name, "a number", given))?;
            let number = normal_number(name, number)?;
            if bounds_here && !(number > *above && number <= *maximum) {
                return Err(refused(
                    format!("'{name}' {given} is not above {above} {unit} or is above {maximum} {unit}"),
                    input.refusal,
                ));
            }
            Ok(GivenValue::Number(number))
        }
        Kind::NumberWithin {
            minimum,
            minimum_exclusive,
            maximum,
            maximum_exclusive,
            unit,
        } => {
            let number = given
                .as_f64()
                .ok_or_else(|| must_be(name, "a number", given))?;
            let number = normal_number(name, number)?;
            let low = if *minimum_exclusive { number > *minimum } else { number >= *minimum };
            let high = if *maximum_exclusive { number < *maximum } else { number <= *maximum };
            if bounds_here && !(low && high) {
                let from = if *minimum_exclusive { "above" } else { "from" };
                let to = if *maximum_exclusive { "and below" } else { "to" };
                return Err(refused(
                    format!("'{name}' {given} is outside {from} {minimum} {unit} {to} {maximum} {unit}"),
                    input.refusal,
                ));
            }
            Ok(GivenValue::Number(number))
        }
        Kind::Word { allowed } => {
            let Value::String(word) = given else {
                return Err(must_be(name, "a string", given));
            };
            if !allowed.contains(&word.as_str()) {
                let quoted: Vec<String> = allowed.iter().map(|word| format!("\"{word}\"")).collect();
                let choice = match quoted.as_slice() {
                    [one] => one.clone(),
                    many => format!("one of {}", many.join(", ")),
                };
                return Err(refused(
                    format!("'{name}' must be {choice}; this request has {given}"),
                    input.refusal,
                ));
            }
            Ok(GivenValue::Word(word.clone()))
        }
        Kind::Flag => match given {
            Value::Bool(flag) => Ok(GivenValue::Flag(*flag)),
            _ => Err(must_be(name, "true or false", given)),
        },
        Kind::Numbers {
            fewest,
            most,
            minimum,
            maximum,
            unit,
        } => {
            let Value::Array(items) = given else {
                return Err(must_be(name, "an array of numbers", given));
            };
            if bounds_here && items.len() > *most {
                return Err(refused(
                    format!("'{name}' holds {} numbers, more than {most}", items.len()),
                    input.refusal,
                ));
            }
            if bounds_here && items.len() < *fewest {
                return Err(refused(
                    format!("'{name}' holds {} numbers, fewer than {fewest}", items.len()),
                    input.refusal,
                ));
            }
            let mut numbers = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                let place = format!("{name}[{index}]");
                let number = item
                    .as_f64()
                    .ok_or_else(|| must_be(&place, "a number", item))?;
                let number = normal_number(&place, number)?;
                if bounds_here && !(*minimum..=*maximum).contains(&number) {
                    return Err(refused(
                        format!(
                            "'{place}' {item} is outside {}",
                            solvers::range_words(*minimum, *maximum, unit)
                        ),
                        input.refusal,
                    ));
                }
                numbers.push(number);
            }
            Ok(GivenValue::Numbers(numbers))
        }
        Kind::NumbersWithin {
            fewest,
            most,
            minimum,
            minimum_exclusive,
            maximum,
            maximum_exclusive,
            unit,
        } => {
            let Value::Array(items) = given else {
                return Err(must_be(name, "an array of numbers", given));
            };
            if bounds_here && items.len() > *most {
                return Err(refused(
                    format!("'{name}' holds {} numbers, more than {most}", items.len()),
                    input.refusal,
                ));
            }
            if bounds_here && items.len() < *fewest {
                return Err(refused(
                    format!("'{name}' holds {} numbers, fewer than {fewest}", items.len()),
                    input.refusal,
                ));
            }
            let mut numbers = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                let place = format!("{name}[{index}]");
                let number = item
                    .as_f64()
                    .ok_or_else(|| must_be(&place, "a number", item))?;
                let number = normal_number(&place, number)?;
                if bounds_here && !solvers::within(number, *minimum, *minimum_exclusive, *maximum, *maximum_exclusive) {
                    return Err(refused(
                        format!(
                            "'{place}' {item} is outside {}",
                            solvers::range_within_words(*minimum, *minimum_exclusive, *maximum, *maximum_exclusive, unit)
                        ),
                        input.refusal,
                    ));
                }
                numbers.push(number);
            }
            Ok(GivenValue::Numbers(numbers))
        }
    }
}

/// `true` for a float that a plain integer token can also be read as: minus
/// zero, a value at or above 2^64, or a value at or below -2^63. A count
/// never lies there, so the range refuses it whatever the spelling.
fn integer_held_as_float(given: &Value) -> bool {
    const TWO_TO_63: f64 = 9_223_372_036_854_775_808.0;
    match given.as_f64() {
        Some(read) if given.is_f64() => {
            (read == 0.0 && read.is_sign_negative()) || read >= 2.0 * TWO_TO_63 || read <= -TWO_TO_63
        }
        _ => false,
    }
}

/// The number rule of a solve request: a subnormal is refused wherever zero
/// and larger numbers are accepted, as every card's request refuses it.
fn normal_number(name: &str, number: f64) -> Result<f64, DoorError> {
    if number.is_finite() && !number.is_subnormal() {
        Ok(number)
    } else {
        Err(DoorError::Unprocessable(format!(
            "Physical Violation: '{name}' is not a finite normal number."
        )))
    }
}

/// A value of the wrong kind, named with what the request holds.
fn must_be(name: &str, kind: &str, given: &Value) -> DoorError {
    DoorError::Unprocessable(format!(
        "Physical Violation: '{name}' must be {kind}; this request has {given}."
    ))
}

/// A value outside its bound, with the name of the refusal when the input
/// has one.
fn refused(sentence: String, refusal: Option<&str>) -> DoorError {
    DoorError::Unprocessable(match refusal {
        Some(refusal) => format!("Physical Violation: {sentence} ({refusal})."),
        None => format!("Physical Violation: {sentence}."),
    })
}

pub(crate) fn seal(
    solver: &Solver,
    request: Value,
    solved: Solved,
) -> Result<SolutionResponse, String> {
    seal_with(solver, request, solved, None)
}

/// A solution sealed, with the chain of a chained solve written beside its
/// seal and covered by its receipt.
pub(crate) fn seal_with(
    solver: &Solver,
    request: Value,
    solved: Solved,
    chain: Option<&dyn Fn(&str) -> Value>,
) -> Result<SolutionResponse, String> {
    let parameters = request.get("parameters").and_then(Value::as_object);
    let (outcome, reject, reject_detail, iterations, result, invariants, field) = match solved {
        Solved::Answer(answer) => (
            OUTCOME_CONVERGED,
            None,
            None,
            Some(answer.iterations),
            Some(section_object(solver, PART_RESULT, &answer.result, parameters)?),
            Some(section_object(solver, PART_INVARIANTS, &answer.invariants, parameters)?),
            Some(field_digest(&answer.field)),
        ),
        Solved::Refused(refusal) => {
            let listed = solver.refusal(refusal.name).ok_or_else(|| {
                format!(
                    "Physical Violation: {} returned the refusal {}, which its entry does not list. Nothing is sealed.",
                    solver.name, refusal.name
                )
            })?;
            // The door checks a limit the request shows before a permit is
            // taken. A solver that still names the question as outside its
            // range is not sealed, and its sentence carries the refusal's
            // name, as the door's does: an entry that has not already put
            // the name at the end of the sentence has it put there.
            if !listed.sealed {
                let name = format!("({})", refusal.name);
                return Err(if refusal.sentence.ends_with(&name) {
                    format!("Physical Violation: {}.", refusal.sentence)
                } else {
                    format!("Physical Violation: {} {name}.", refusal.sentence)
                });
            }
            (
                OUTCOME_REFUSED,
                Some(refusal.name),
                Some(format!(
                    "Solver Refusal: {} did not return {} ({}). {}.",
                    solver.name, solver.answer, refusal.name, refusal.sentence
                )),
                refusal.iterations,
                None,
                None,
                None,
            )
        }
    };
    sealed(request, outcome, reject, reject_detail, iterations, result, invariants, field, chain)
}

/// A solution's document sealed under this engine's version, with its
/// receipt: the same for a solve and for a solution the long form of
/// `verify_solution` writes again from its sealed parts.
#[allow(clippy::too_many_arguments)]
fn sealed(
    request: Value,
    outcome: &str,
    reject: Option<&str>,
    reject_detail: Option<String>,
    iterations: Option<u64>,
    result: Option<Value>,
    invariants: Option<Value>,
    field: Option<String>,
    chain: Option<&dyn Fn(&str) -> Value>,
) -> Result<SolutionResponse, String> {
    let version = solvers::engine_version();
    let ingredients = Ingredients {
        request: crate::sealed_form(&request).into_bytes(),
        version: version_text(&version),
        result_text: values_text(PART_RESULT, result.as_ref().and_then(Value::as_object))?,
        invariants_text: values_text(
            PART_INVARIANTS,
            invariants.as_ref().and_then(Value::as_object),
        )?,
        field: field_text(field.as_deref()),
        tail: tail_bytes(outcome, iterations, reject),
    };
    let load_source = crate::signed_load::solve_source_of(&request);
    let document = SolutionDocument {
        inputs: request,
        engine_version: version,
        outcome: outcome.to_string(),
        reject: reject.map(str::to_string),
        reject_detail,
        iterations,
        result,
        invariants,
        field_sha256: field,
        load_source,
        seal_parts: ingredients.parts(),
        solution_seal: ingredients.seal(),
        chain: None,
    };
    let mut document = document;
    if let Some(chain) = chain {
        document.chain = Some(chain(&document.solution_seal));
    }
    let text = serde_json::to_string(&document).map_err(|error| {
        format!("Physical Violation: the response could not be encoded. {error}")
    })?;
    let response = SolutionResponse {
        document,
        receipt_sha256: crate::digest_hex(text.as_bytes()),
    };
    let wire = serde_json::to_string(&response).map_err(|error| {
        format!("Physical Violation: the response could not be encoded. {error}")
    })?;
    if wire != crate::append_receipt(&text, &response.receipt_sha256) {
        return Err(
            "Physical Violation: the receipt does not cover the response bytes.".to_string(),
        );
    }
    Ok(response)
}

/// `result` or `invariants` as the response carries it: each value the solver
/// returned and, beside it, its exact hex in the same shape. A name outside
/// the solver's field list, a shape other than the listed one, a name given
/// twice, a listed name the solver left out, a value the request asked for
/// and did not get, and a value it got without asking are each a fault, and
/// nothing is sealed.
fn section_object(
    solver: &Solver,
    section: &str,
    values: &[(String, Published)],
    parameters: Option<&Map<String, Value>>,
) -> Result<Value, String> {
    let mut object = Map::new();
    for (name, value) in values {
        if solver.output_shape(section, name) != Some(value.shape()) {
            return Err(format!(
                "Physical Violation: {} returned '{section}.{name}', which its field list does not publish in that shape. Nothing is sealed.",
                solver.name
            ));
        }
        if object.contains_key(name) {
            return Err(format!(
                "Physical Violation: {} returned '{section}.{name}' twice. Nothing is sealed.",
                solver.name
            ));
        }
        let (decimal, hex) = match value {
            Published::Number(number) => (crate::finite_number(*number)?, crate::f64_hex(*number)),
            Published::Numbers(numbers) => decimals_and_hexes(numbers)?,
            Published::Lists(lists) => {
                let mut decimals = Vec::with_capacity(lists.len());
                let mut hexes = Vec::with_capacity(lists.len());
                for numbers in lists {
                    let (decimal, hex) = decimals_and_hexes(numbers)?;
                    decimals.push(decimal);
                    hexes.push(hex);
                }
                (Value::Array(decimals), Value::Array(hexes))
            }
        };
        object.insert(name.clone(), decimal);
        object.insert(format!("{name}_hex"), hex);
    }
    for output in solver.outputs(section) {
        if !output.each.is_empty() {
            continue;
        }
        // A value every solution holds, or one this request asked for.
        let asked = match output.asked_by {
            None => true,
            Some(flag) => {
                parameters.and_then(|parameters| parameters.get(flag)) == Some(&Value::Bool(true))
            }
        };
        let returned = object.contains_key(output.name);
        if asked && !returned {
            return Err(format!(
                "Physical Violation: {} did not return '{section}.{}'. Nothing is sealed.",
                solver.name, output.name
            ));
        }
        if returned && !asked {
            return Err(format!(
                "Physical Violation: {} returned '{section}.{}', which this request did not ask for. Nothing is sealed.",
                solver.name, output.name
            ));
        }
    }
    Ok(Value::Object(object))
}

/// An array of numbers, and the array of their exact hex in the same order.
fn decimals_and_hexes(numbers: &[f64]) -> Result<(Value, Value), String> {
    let mut decimals = Vec::with_capacity(numbers.len());
    let mut hexes = Vec::with_capacity(numbers.len());
    for number in numbers {
        decimals.push(crate::finite_number(*number)?);
        hexes.push(crate::f64_hex(*number));
    }
    Ok((Value::Array(decimals), Value::Array(hexes)))
}

struct Ingredients {
    request: Vec<u8>,
    version: String,
    result_text: String,
    invariants_text: String,
    field: String,
    tail: Vec<u8>,
}

impl Ingredients {
    /// Request, then the engine version line, then every hex line sorted by
    /// key (`invariants.` sorts before `result.`), then the field hash, then
    /// the tail.
    fn seal(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.request);
        hasher.update(self.version.as_bytes());
        hasher.update(self.invariants_text.as_bytes());
        hasher.update(self.result_text.as_bytes());
        hasher.update(self.field.as_bytes());
        hasher.update(&self.tail);
        hex::encode(hasher.finalize())
    }

    fn parts(&self) -> SealParts {
        SealParts {
            inputs: crate::digest_hex(&self.request),
            engine_version: crate::digest_hex(self.version.as_bytes()),
            result: crate::digest_hex(self.result_text.as_bytes()),
            invariants: crate::digest_hex(self.invariants_text.as_bytes()),
            field_sha256: crate::digest_hex(self.field.as_bytes()),
            outcome: crate::digest_hex(&self.tail),
        }
    }
}

/// The engine version as it enters the seal: one line, `engine_version:`,
/// the version, and a line break. A solution claimed for another version of
/// the solvers is another seal.
fn version_text(version: &str) -> String {
    format!("engine_version:{version}\n")
}

fn tail_bytes(outcome: &str, iterations: Option<u64>, reject: Option<&str>) -> Vec<u8> {
    serde_json::to_vec(&SealTail {
        outcome,
        iterations,
        reject,
    })
    .expect("seal tail encodes")
}

/// The field hash as it enters the seal: its 64 hex characters, or `null`.
fn field_text(field: Option<&str>) -> String {
    field.unwrap_or("null").to_string()
}

/// SHA-256 of the point count as a little-endian `u64`, then each sample as
/// little-endian `f64` bytes.
fn field_digest(density: &[f64]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((density.len() as u64).to_le_bytes());
    for sample in density {
        hasher.update(sample.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

/// One `{section}.{key}:{hex}` line for every key that ends in `_hex`, sorted
/// by key, each line ending in a newline. A null section has no lines. The
/// hex of an array is `[`, its items joined by `,` in the order given, and
/// `]`, and an item that is itself an array is written the same way, so one
/// item changed, removed, added or moved is another text.
/// Decimal tokens are not in this text: a client that parses and re-encodes
/// a response keeps the hex and may not keep the last bit of a decimal.
pub(crate) fn values_text(section: &str, object: Option<&Map<String, Value>>) -> Result<String, String> {
    let Some(object) = object else {
        return Ok(String::new());
    };
    let mut lines = Vec::new();
    for (key, value) in object {
        if !key.ends_with("_hex") {
            continue;
        }
        let mut hex = String::new();
        if !write_hex(&mut hex, value) {
            return Err(format!(
                "Physical Violation: '{section}.{key}' is not a string or an array of strings."
            ));
        }
        lines.push((key.as_str(), hex));
    }
    lines.sort_by(|left, right| left.0.cmp(right.0));
    let mut text = String::new();
    for (key, hex) in lines {
        text.push_str(section);
        text.push('.');
        text.push_str(key);
        text.push(':');
        text.push_str(&hex);
        text.push('\n');
    }
    Ok(text)
}

/// A hex string as it is, and an array in brackets with commas between its
/// items. False for any other value.
fn write_hex(text: &mut String, value: &Value) -> bool {
    match value {
        Value::String(hex) => {
            text.push_str(hex);
            true
        }
        Value::Array(items) => {
            text.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    text.push(',');
                }
                if !write_hex(text, item) {
                    return false;
                }
            }
            text.push(']');
            true
        }
        _ => false,
    }
}

fn require_exact_solution(
    solution_seal: &str,
    field: Option<&str>,
    parts: &[(&str, &str)],
    solver: Option<&Solver>,
    result: Option<&Map<String, Value>>,
    invariants: Option<&Map<String, Value>>,
) -> Result<(), DoorError> {
    if !crate::json_text::is_lower_hex(solution_seal, 64) {
        return Err(exact_digits("solution_seal", 64));
    }
    if let Some(text) = field {
        if !crate::json_text::is_lower_hex(text, 64) {
            return Err(exact_digits("field_sha256", 64));
        }
    }
    for (part, text) in parts {
        if !crate::json_text::is_lower_hex(text, 64) {
            return Err(exact_digits(&format!("seal_parts.{part}"), 64));
        }
    }
    require_section_hex(PART_RESULT, solver, result)?;
    require_section_hex(PART_INVARIANTS, solver, invariants)?;
    Ok(())
}

/// Every key of a section is a name the solution's solver publishes there,
/// and every `_hex` value is exact and has the shape the solver publishes it
/// in: one hex, an array of them, or an array of arrays of them.
fn require_section_hex(
    section: &str,
    solver: Option<&Solver>,
    object: Option<&Map<String, Value>>,
) -> Result<(), DoorError> {
    let Some(object) = object else {
        return Ok(());
    };
    for (key, value) in object {
        if !crate::json_text::is_plain_field_name(key) {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{section}.{key}' is not a field name."
            )));
        }
        let Some(shape) = solver.and_then(|solver| solver.published_shape(section, key)) else {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{section}.{key}' is not published by this solver."
            )));
        };
        if key.ends_with("_hex") {
            let name = format!("{section}.{key}");
            match shape {
                Shape::Number if !is_exact_hex(value) => return Err(exact_digits(&name, 16)),
                Shape::Numbers if !is_exact_hex_array(value) => {
                    return Err(DoorError::Unprocessable(format!(
                        "Physical Violation: '{name}' must be an array, and each item 16 lowercase hex digits."
                    )));
                }
                Shape::Lists
                    if !value
                        .as_array()
                        .is_some_and(|lists| lists.iter().all(is_exact_hex_array)) =>
                {
                    return Err(DoorError::Unprocessable(format!(
                        "Physical Violation: '{name}' must be an array of arrays, and each item 16 lowercase hex digits."
                    )));
                }
                _ => {}
            }
        } else if crate::json_text::is_flag(key) && !value.is_boolean() {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{section}.{key}' must be a JSON boolean."
            )));
        }
    }
    Ok(())
}

fn is_exact_hex(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|text| crate::json_text::is_lower_hex(text, 16))
}

fn is_exact_hex_array(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(is_exact_hex))
}

fn exact_digits(name: &str, digits: usize) -> DoorError {
    DoorError::Unprocessable(format!(
        "Physical Violation: '{name}' must be {digits} lowercase hex digits."
    ))
}

fn fails(part: &str) -> SolutionReport {
    SolutionReport {
        holds: false,
        cause: Some(part.to_string()),
        compared: WHOLE_SOLUTION_COMPARED,
    }
}

fn required<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a Value, DoorError> {
    object
        .get(key)
        .ok_or_else(|| DoorError::Unprocessable(format!("Missing required parameter: '{key}' (InputMissing).")))
}

fn wrong_type(key: &str, expected: &str) -> DoorError {
    DoorError::Unprocessable(format!("Physical Violation: '{key}' must be {expected}."))
}

fn optional_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, DoorError> {
    match required(object, key)? {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text.as_str())),
        _ => Err(wrong_type(key, "a string or null")),
    }
}

fn optional_object<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a Map<String, Value>>, DoorError> {
    match required(object, key)? {
        Value::Null => Ok(None),
        Value::Object(map) => Ok(Some(map)),
        _ => Err(wrong_type(key, "a JSON object or null")),
    }
}

/// A key outside the solver's own list is named, with what the solver keeps
/// to itself.
fn reject_unknown_keys<'a>(
    solver: &Solver,
    object: &Map<String, Value>,
    allowed: impl Iterator<Item = &'a str> + Clone,
) -> Result<(), DoorError> {
    for key in object.keys() {
        if !allowed.clone().any(|allowed_key| allowed_key == key) {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{key}' is not an input of this solver. {}",
                solver.kept_inside
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver_atom_dft::{self, ATOM_DFT};
    use genesis_solvers::atom_dft::{self, AtomGroundState, AtomReject, Orbital};
    use serde_json::json;

    fn request() -> Value {
        json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } })
    }

    /// The seal of what `atom_dft` returned, as `run` makes it.
    fn seal_atom(
        request: Value,
        returned: Result<AtomGroundState, AtomReject>,
    ) -> Result<SolutionResponse, String> {
        seal(&ATOM_DFT, request, solver_atom_dft::solved(returned))
    }

    fn posted(response: &SolutionResponse) -> Value {
        serde_json::from_str(&serde_json::to_string(response).unwrap()).unwrap()
    }

    /// Every atom in range converges, so a refusal cannot be reached through
    /// the door. The seal of one is built here from the solver's own refusals.
    #[test]
    fn a_refusal_is_sealed_with_no_result_and_the_seal_binds_it() {
        let refusals = [
            (AtomReject::NotConverged { iterations: 80 }, Some(80_u64)),
            (AtomReject::VirialBroken { residual_ha: 0.5 }, None),
            (
                AtomReject::ElectronCountBroken {
                    electron_count: 4.0,
                    atomic_number: 2,
                },
                None,
            ),
        ];
        let mut seals = Vec::new();
        for (reject, iterations) in refusals {
            let name = solver_atom_dft::reject_name(&reject);
            let response = seal_atom(request(), Err(reject)).expect("a refusal is sealed");
            let body = posted(&response);
            assert_eq!(body["outcome"], "refused");
            assert_eq!(body["reject"], name);
            assert!(body["reject_detail"].as_str().unwrap().contains(name));
            assert_eq!(body["iterations"], json!(iterations));
            assert!(body["result"].is_null());
            assert!(body["invariants"].is_null());
            assert!(body["field_sha256"].is_null());
            assert_eq!(body["solution_seal"].as_str().unwrap().len(), 64);
            assert_eq!(body["receipt_sha256"].as_str().unwrap().len(), 64);
            let report = check(&body).unwrap_or_else(|_| panic!("{name} parses"));
            assert!(report.holds, "{name} {:?}", report.cause);

            let mut renamed = body.clone();
            renamed["reject"] = json!("NegativeDensity");
            let report = check(&renamed).unwrap_or_else(|_| panic!("{name} parses"));
            assert!(!report.holds);
            assert_eq!(report.cause.as_deref(), Some("outcome"));

            let mut promoted = body.clone();
            promoted["outcome"] = json!("converged");
            let report = check(&promoted).unwrap_or_else(|_| panic!("{name} parses"));
            assert!(!report.holds);
            assert_eq!(report.cause.as_deref(), Some("outcome"));
            seals.push(body["solution_seal"].as_str().unwrap().to_string());
        }
        seals.sort();
        seals.dedup();
        assert_eq!(seals.len(), 3, "each refusal has its own seal");
    }

    #[test]
    fn a_range_refusal_from_the_solver_is_not_sealed() {
        let message = match seal_atom(
            request(),
            Err(AtomReject::AtomicNumberOutOfRange { atomic_number: 19 }),
        ) {
            Ok(_) => panic!("an out-of-range atom was sealed"),
            Err(message) => message,
        };
        // Re-pinned for 0.10.1: a refusal the solver itself returns unsealed
        // carries its name, as the door's sentence does. The words before
        // the name are the solver's own, the range in words.
        assert_eq!(
            message,
            "Physical Violation: atomic number 19 is outside 1 to 18 (AtomicNumberOutOfRange)."
        );
        let door = match prepare(&json!({
            "solver": "atom_dft",
            "parameters": { "atomic_number": 19 }
        })) {
            Ok(_) => panic!("an out-of-range atom passed the door"),
            Err(error) => error.message().to_string(),
        };
        // The door refuses first, with the same name and the range in words.
        assert_eq!(door, "Physical Violation: atomic number 19 is outside 1 to 18 (AtomicNumberOutOfRange).");
    }

    /// An entry that has already put the refusal's name at the end of an
    /// unsealed sentence keeps it once: the name is not written twice.
    #[test]
    fn an_unsealed_refusal_that_already_ends_with_its_name_carries_it_once() {
        use genesis_solvers::pipe_flow_superpipe::PipeReject;
        let solved = crate::solver_pipe_flow_superpipe::solved(Err(PipeReject::LaminarFlow { bulk_reynolds_number: 1000.0 }));
        let request = json!({ "solver": "pipe_flow_superpipe", "parameters": {} });
        let message = match seal(&crate::solver_pipe_flow_superpipe::PIPE_FLOW_SUPERPIPE, request, solved) {
            Ok(_) => panic!("a laminar flow was sealed"),
            Err(message) => message,
        };
        assert_eq!(
            message,
            "Physical Violation: bulk Reynolds number 1000 is below 2300: the flow is laminar, and this solver is of turbulent flow (LaminarFlow)."
        );
        assert_eq!(message.matches("(LaminarFlow)").count(), 1);
    }

    #[test]
    fn the_field_hash_is_the_count_then_little_endian_samples() {
        let density = [0.0_f64, 1.5, -2.25];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&3_u64.to_le_bytes());
        for sample in density {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        assert_eq!(field_digest(&density), crate::digest_hex(&bytes));
        assert_ne!(field_digest(&density), field_digest(&density[..2]));
    }

    fn hash() -> String {
        "ab".repeat(32)
    }

    #[test]
    fn a_hash_or_hex_that_is_not_exact_is_refused_by_name() {
        let mut body = json!({
            "inputs": { "solver": "atom_dft", "parameters": { "atomic_number": 1 } },
            "engine_version": "genesis_solvers_v0.8.0",
            "outcome": "converged",
            "iterations": 1,
            "reject": null,
            "result": { "total_energy_ha_hex": "0123456789abcdef\nresult.total_energy_ha_hex:0123456789abcdef" },
            "invariants": null,
            "field_sha256": hash(),
            "seal_parts": {
                "inputs": hash(),
                "engine_version": hash(),
                "result": hash(),
                "invariants": hash(),
                "field_sha256": hash(),
                "outcome": hash()
            },
            "solution_seal": hash()
        });
        let Err(error) = check(&body) else {
            panic!("a stuffed hex was accepted");
        };
        let message = error.message();
        assert!(
            message.contains("result.total_energy_ha_hex"),
            "{message}"
        );
        assert!(message.contains("16 lowercase hex digits"), "{message}");

        body["result"] = json!(null);
        body["field_sha256"] = json!(format!("{}{}", "not-a-hash", hash()));
        let Err(error) = check(&body) else {
            panic!("a bad field hash was accepted");
        };
        let message = error.message();
        assert!(message.contains("field_sha256"), "{message}");
        assert!(message.contains("64 lowercase hex digits"), "{message}");
    }

    #[test]
    fn an_unpublished_solution_name_is_refused() {
        let body = json!({
            "inputs": { "solver": "atom_dft", "parameters": { "atomic_number": 1 } },
            "engine_version": "genesis_solvers_v0.8.0",
            "outcome": "converged",
            "iterations": 1,
            "reject": null,
            "result": {
                "not_a_field": "0123456789abcdef",
                "total_energy_ha_hex": "0123456789abcdef"
            },
            "invariants": null,
            "field_sha256": hash(),
            "seal_parts": {
                "inputs": hash(),
                "engine_version": hash(),
                "result": hash(),
                "invariants": hash(),
                "field_sha256": hash(),
                "outcome": hash()
            },
            "solution_seal": hash()
        });
        let Err(error) = check(&body) else {
            panic!("an unpublished name was accepted");
        };
        let message = error.message();
        assert!(message.contains("result.not_a_field"), "{message}");
        assert!(message.contains("is not published by this solver"), "{message}");
    }

    /// The lines of a real solution, packed into one invariants name. The seal
    /// bytes are the same. The name is refused before that seal is trusted.
    #[test]
    fn a_solution_name_that_carries_other_lines_is_refused() {
        let solved = atom_dft::solve(2).expect("helium converges");
        let response = seal_atom(request(), Ok(solved)).expect("helium is sealed");
        let body = posted(&response);
        let result_text =
            values_text(PART_RESULT, body["result"].as_object()).expect("result text");
        let invariants_text =
            values_text(PART_INVARIANTS, body["invariants"].as_object()).expect("invariants text");
        let combined = format!("{invariants_text}{result_text}");
        let prefix = "invariants.";
        assert!(combined.starts_with(prefix), "{combined}");
        let rest = &combined[prefix.len()..];
        assert!(rest.len() > 18 && rest.as_bytes()[rest.len() - 18] == b':');
        let value = &rest[rest.len() - 17..rest.len() - 1];
        let key = &rest[..rest.len() - 18];
        assert!(key.contains('\n'), "the packed name does not carry another line");
        assert_eq!(value.len(), 16);

        let mut forged = body.clone();
        let mut packed = serde_json::Map::new();
        packed.insert(key.to_string(), json!(value));
        forged["invariants"] = serde_json::Value::Object(packed);
        forged["result"] = Value::Null;
        let packed_text =
            values_text(PART_INVARIANTS, forged["invariants"].as_object()).expect("packed text");
        assert_eq!(packed_text, combined);
        forged["seal_parts"]["invariants"] = json!(crate::digest_hex(packed_text.as_bytes()));
        forged["seal_parts"]["result"] = json!(crate::digest_hex(b""));
        let ingredients = Ingredients {
            request: crate::sealed_form(&forged["inputs"]).into_bytes(),
            version: version_text(forged["engine_version"].as_str().expect("version")),
            result_text: String::new(),
            invariants_text: packed_text,
            field: field_text(forged["field_sha256"].as_str()),
            tail: tail_bytes(
                forged["outcome"].as_str().expect("outcome"),
                forged["iterations"].as_u64(),
                forged["reject"].as_str(),
            ),
        };
        assert_eq!(
            ingredients.seal(),
            body["solution_seal"].as_str().expect("seal")
        );

        let wire = serde_json::to_string(&forged).expect("forged body encodes");
        let parsed: serde_json::Value = serde_json::from_str(&wire).expect("forged body parses");
        assert_eq!(parsed["solution_seal"], body["solution_seal"]);
        let Err(error) = check(&parsed) else {
            panic!("a name carrying other lines was accepted");
        };
        let message = error.message();
        assert!(message.contains("is not a field name"), "{message}");
        assert!(message.contains("invariants."), "{message}");
    }

    /// A ground state with made-up numbers, so the bits do not depend on the
    /// machine that runs the solver.
    fn made_up_state() -> AtomGroundState {
        AtomGroundState {
            atomic_number: 3,
            iterations: 7,
            total_energy_ha: -7.25,
            kinetic_energy_ha: 7.125,
            electron_nucleus_energy_ha: -16.5,
            hartree_energy_ha: 3.875,
            exchange_correlation_energy_ha: -1.75,
            electron_count: 3.0000000001,
            virial_residual_ha: -0.000001953125,
            orbitals: vec![
                Orbital { n: 1, l: 0, occupation: 2.0, eigenvalue_ha: -1.875 },
                Orbital { n: 2, l: 0, occupation: 1.0, eigenvalue_ha: -0.1 },
                Orbital { n: 2, l: 1, occupation: 0.0, eigenvalue_ha: -0.0 },
            ],
            radial_grid_bohr: vec![0.5, 1.0, 2.0],
            electron_density: vec![0.0, 1.5, -2.25],
        }
    }

    /// The proof of an `atom_dft` solution is the one the gateway made when
    /// `atom_dft` was the only solver it could serve. The seals, the seal
    /// parts and every value of the result below were written by that code,
    /// version 0.2.2, from the same made-up state and the same refusals, and
    /// none has moved. Two things have, with the solvers crate going to
    /// version 0.3.0 when a second solver was added, to version 0.4.0 when
    /// seven more were, to version 0.5.0 when three more were and to
    /// version 0.6.0 when their ranges were narrowed, to version 0.7.0
    /// when pipe_flow_superpipe was added and to version 0.8.0 when two
    /// inputs gained a floor: the `engine_version`
    /// field, and the six
    /// receipts, each of which covers that field. The new receipts were
    /// computed outside this code each time, as the SHA-256 of each
    /// response's text before its receipt, closed with a brace; each text
    /// with the earlier version put back gives the earlier receipt. In 0.7.0
    /// the seal came to cover the engine version, a line of its own after the
    /// request, so every seal and receipt moved; each was worked out again
    /// with Python's hashlib from the rule in `REFERENCE.md`, with the solvers
    /// at version 0.6.0; the rule without the version line still gives the
    /// 0.6.0 seal, and with the solvers at 0.5.0 it gives the values first
    /// written for 0.7.0. In 0.8.0 the solvers went to 0.7.0, and the seals
    /// and receipts were worked out again the same way; with the version
    /// 0.6.0 put back the same rule gives the 0.7.0 values. In 0.10.0 the
    /// solvers went to 0.8.0, and the seals and receipts were worked out
    /// again the same way; with the version 0.7.0 put back the same rule
    /// gives the values pinned for 0.8.0. In the engine after 0.10.1 the
    /// solvers went to 0.9.0, and the seal, its engine_version part and the
    /// receipt, and the seals and receipts of the five refusals below, were
    /// worked out again the same way; with the version 0.8.0
    /// put back the same rule gives the values pinned before. In engine
    /// 0.12.0 the solvers went to 0.10.0: the seal, its engine_version part
    /// and the receipt, and the five refusals' seals and receipts, were
    /// reproduced at 0.9.0 with Python's hashlib from the rule (request,
    /// version line, hex lines, field, tail; the receipt over the answer's own
    /// bytes) and then taken at 0.10.0. In engine 0.13.0 the solvers went to
    /// 0.11.0: the same rule, run on this test's own answer texts, reproduces
    /// every 0.10.0 pin with the version put back (seal 0ec742ce...9c56,
    /// engine_version part 4c5b6265...4786, receipt 2a1c8e94...e668) and
    /// gave the 0.11.0 values. In the solvers' 0.12.0, the canonical math for
    /// engine 0.14.0, the same rule, run on this test's own answer texts,
    /// reproduces every 0.11.0 pin with the version put back (seal
    /// 10688f6c...5ff0, engine_version part eb36a60b...f390, receipt
    /// c5b6d0b6...6878) and gives the 0.12.0 values pinned here. The hash of
    /// the request, of the result, of the invariants, of the field and of the
    /// outcome have not moved: of the six seal parts only `engine_version`
    /// has.
    #[test]
    fn the_proof_of_an_atom_is_bit_for_bit_the_proof_before_solvers_were_named() {
        let request = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 3 } });
        let response = seal_atom(request, Ok(made_up_state())).expect("sealed");
        assert_eq!(
            response.document.solution_seal,
            "6fd55b35beeb5d4c27adf07c2968ef25f846a871a2f4cddb00a7365d64ae35ea"
        );
        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            concat!(
                r#"{"inputs":{"solver":"atom_dft","parameters":{"atomic_number":3}},"engine_version":"genesis_solvers_v0.12.0","outcome":"converged","reject":null,"reject_detail":null,"iterations":7,"#,
                r#""result":{"total_energy_ha":-7.25e0,"total_energy_ha_hex":"c01d000000000000","kinetic_energy_ha":7.125e0,"kinetic_energy_ha_hex":"401c800000000000","electron_nucleus_energy_ha":-1.65e1,"electron_nucleus_energy_ha_hex":"c030800000000000","hartree_energy_ha":3.875e0,"hartree_energy_ha_hex":"400f000000000000","exchange_correlation_energy_ha":-1.75e0,"exchange_correlation_energy_ha_hex":"bffc000000000000","#,
                r#""eigenvalue_1s_ha":-1.875e0,"eigenvalue_1s_ha_hex":"bffe000000000000","occupation_1s":2.0e0,"occupation_1s_hex":"4000000000000000","eigenvalue_2s_ha":-1.0e-1,"eigenvalue_2s_ha_hex":"bfb999999999999a","occupation_2s":1.0e0,"occupation_2s_hex":"3ff0000000000000","eigenvalue_2p_ha":-0.0e0,"eigenvalue_2p_ha_hex":"8000000000000000","occupation_2p":0.0e0,"occupation_2p_hex":"0000000000000000"},"#,
                r#""invariants":{"electron_count":3.0000000001e0,"electron_count_hex":"4008000000036f9c","virial_residual_ha":-1.953125e-6,"virial_residual_ha_hex":"bec0624dd2f1a9fc"},"#,
                r#""field_sha256":"1a9df62a92e88e4a0cf1cd97fa0cedfde9535f809f238b045b96e8c6033923c5","#,
                r#""seal_parts":{"inputs":"e57be020a41482d3fc714652587ebb43e497ba6ce06e40cb8c3b6e2e2693c98b","engine_version":"e54ba43c1f265471435c46a330ca4e5ef1e80b1e6c44fb8e3a664f4ba299a2b0","result":"1c15b61ca9b42bdba55397c2dceb2da92eca1b61e58d22fd42cd34163afdf58e","invariants":"590bcb8e8f0d5b8941c276f3a620b5a9960996cdd65a05a0a297305d75a975c1","field_sha256":"fec27d21016c7846505738c86f771be21347f175fdc1108680e8e6df46db079c","outcome":"daec2f73173111450f5ffd61d25450b3e9da9d0d55f370590d4ad61736c4bdea"},"#,
                r#""solution_seal":"6fd55b35beeb5d4c27adf07c2968ef25f846a871a2f4cddb00a7365d64ae35ea","receipt_sha256":"99912ec592a889d009af7464a0fe0daab0a44a941ab1ac432d368639d8a84446"}"#,
            )
        );
        let report = check(&posted(&response)).unwrap_or_else(|_| panic!("the solution parses"));
        assert!(report.holds, "{:?}", report.cause);

        let refusals = [
            (
                AtomReject::NotConverged { iterations: 80 },
                "Solver Refusal: atom_dft did not return a ground state (NotConverged). not converged after 80 iterations.",
                "31405c831bc56e30b64f2520998b0a0d937a2b2618fbd2ef052381e04c6a3179",
                "3d94cce52ec2e2f70cf9318ea489075160594b26aa879a065563e861234794ed",
            ),
            (
                AtomReject::VirialBroken { residual_ha: 0.5 },
                "Solver Refusal: atom_dft did not return a ground state (VirialBroken). virial residual 0.5 Ha.",
                "0564435477fbe0888db5ee10ed2e41070daae67b3069af3b41a8566f34c32254",
                "65b4b727cfb71431653689a676043b9c02fae5f6effb1f8017538abc7abea218",
            ),
            (
                AtomReject::ElectronCountBroken { electron_count: 4.0, atomic_number: 2 },
                "Solver Refusal: atom_dft did not return a ground state (ElectronCountBroken). electron count 4 is not atomic number 2.",
                "004207555228a9489081a85c2b3df6e0655b9ded5c46008d332c74d4a352c5d2",
                "e9bba7708be346e3ed36b4db752030ce49bd023749f2b913eef6b1d55edebeaf",
            ),
            (
                AtomReject::NegativeDensity,
                "Solver Refusal: atom_dft did not return a ground state (NegativeDensity). electron density was negative.",
                "68c36f6425250f0098e896cb60892c4f45532c2e3f14105c6518c76091a180d2",
                "03deb8f089456393b34abfd4b7cde1327e3109a204ed90cf2baafcbb11059e88",
            ),
            (
                AtomReject::OrbitalNotBound { n: 3, l: 1 },
                "Solver Refusal: atom_dft did not return a ground state (OrbitalNotBound). no bound orbital for n=3 l=1.",
                "db60f94fc0d9cf8cf664782f107bf21f14fb1081caef1bb740b0d4c8b7d32f97",
                "d9f5266c81a03e50790dedc669e65e3f6968dcece9cec15ad6fd4e132508f784",
            ),
        ];
        for (reject, detail, solution_seal, receipt) in refusals {
            let response = seal_atom(self::request(), Err(reject)).expect("a refusal is sealed");
            assert_eq!(response.document.reject_detail.as_deref(), Some(detail));
            assert_eq!(response.document.solution_seal, solution_seal, "{detail}");
            assert_eq!(response.receipt_sha256, receipt, "{detail}");
        }
    }

    /// The hex of an array enters the proof in brackets, item by item in the
    /// order given, so no two arrangements of the same numbers share a text.
    #[test]
    fn the_text_of_an_array_keeps_every_item_and_its_place() {
        let text = |value: Value| {
            let object = json!({ "b_hex": value, "a_hex": "3ff0000000000000", "a": 1.0 });
            values_text(PART_RESULT, object.as_object()).expect("text")
        };
        let one = "3ff0000000000000";
        let two = "4000000000000000";
        assert_eq!(
            text(json!([one, two])),
            format!("result.a_hex:{one}\nresult.b_hex:[{one},{two}]\n")
        );
        assert_eq!(
            text(json!([[one, two], [], [one]])),
            format!("result.a_hex:{one}\nresult.b_hex:[[{one},{two}],[],[{one}]]\n")
        );
        let arrangements = [
            json!(one),
            json!([one]),
            json!([[one]]),
            json!([]),
            json!([[]]),
            json!([[], []]),
            json!([one, two]),
            json!([two, one]),
            json!([[one, two]]),
            json!([[one], [two]]),
            json!([[one, two], []]),
            json!([[], [one, two]]),
        ];
        let mut texts: Vec<String> = arrangements.iter().cloned().map(text).collect();
        texts.sort();
        texts.dedup();
        assert_eq!(texts.len(), arrangements.len(), "two arrangements share a text");

        for not_hex in [json!(1.5), json!([one, 2]), json!([[one], null]), json!({ "a": one })] {
            let object = json!({ "b_hex": not_hex });
            assert!(values_text(PART_RESULT, object.as_object()).is_err(), "{not_hex}");
        }
    }

    /// A solver that returns a name outside its field list, a listed name in
    /// another shape, a name twice, or less than its list, is not sealed.
    #[test]
    fn a_result_the_field_list_does_not_name_is_not_sealed() {
        let section = |values: Vec<(&str, Published)>| {
            let values: Vec<(String, Published)> = values
                .into_iter()
                .map(|(name, value)| (name.to_string(), value))
                .collect();
            section_object(&ATOM_DFT, PART_INVARIANTS, &values, None)
        };
        let whole = section(vec![
            ("electron_count", Published::Number(2.0)),
            ("virial_residual_ha", Published::Number(0.0)),
        ])
        .expect("the two invariants");
        assert_eq!(whole.as_object().unwrap().len(), 4);

        let unlisted = section(vec![
            ("electron_count", Published::Number(2.0)),
            ("virial_residual_ha", Published::Number(0.0)),
            ("spin", Published::Number(0.0)),
        ])
        .unwrap_err();
        assert!(unlisted.contains("'invariants.spin'"), "{unlisted}");
        let reshaped = section(vec![
            ("electron_count", Published::Numbers(vec![2.0])),
            ("virial_residual_ha", Published::Number(0.0)),
        ])
        .unwrap_err();
        assert!(reshaped.contains("'invariants.electron_count'"), "{reshaped}");
        assert!(reshaped.contains("in that shape"), "{reshaped}");
        let twice = section(vec![
            ("electron_count", Published::Number(2.0)),
            ("electron_count", Published::Number(2.0)),
            ("virial_residual_ha", Published::Number(0.0)),
        ])
        .unwrap_err();
        assert!(twice.contains("twice"), "{twice}");
        let short = section(vec![("electron_count", Published::Number(2.0))]).unwrap_err();
        assert!(short.contains("did not return 'invariants.virial_residual_ha'"), "{short}");
        let not_finite = section(vec![
            ("electron_count", Published::Number(f64::NAN)),
            ("virial_residual_ha", Published::Number(0.0)),
        ])
        .unwrap_err();
        assert!(not_finite.contains("not finite"), "{not_finite}");
    }

    /// The red team of 0.13.0, D4: a solution seal given by request that is
    /// this request's under another version's name is said with both
    /// versions; one of no version names this engine's alone.
    #[test]
    fn a_seal_of_another_version_is_said_with_both_versions() {
        let request = serde_json::json!({ "solver": "atom_dft", "parameters": { "atomic_number": 3 } });
        let prepared = prepare(&request).expect("prepared");
        let response = prepared.solve_and_seal(request).expect("sealed");
        let document = &response.document;
        assert_eq!(seal_with_version(document, &document.engine_version).as_deref(), Some(document.solution_seal.as_str()));
        let theirs = seal_with_version(document, "genesis_solvers_v0.10.0").unwrap();
        assert_ne!(theirs, document.solution_seal);
        let said = version_words(document, &theirs);
        assert_eq!(
            said,
            "The solution_seal covers engine_version: the seal given is this request's solution under engine_version genesis_solvers_v0.10.0, another engine version's, and this engine writes genesis_solvers_v0.12.0, so it does not hold here. The long form, given that answer whole, compares it value by value and names both versions."
        );
        assert_eq!(
            version_words(document, &"0".repeat(64)),
            "The solution_seal covers engine_version, genesis_solvers_v0.12.0 on this engine: the seal of an answer another engine version gave does not hold here."
        );
    }

}
