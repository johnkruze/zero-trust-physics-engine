//! One anchored run, and a check of the tape it writes.
//!
//! Time and the conservation ledger stay on the specimen. A segment only
//! replaces the load. The chain covers recorded frames, so a tape can be
//! checked without the engine.

use genesis_kernel::frame::LastStateFrame64;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::descriptor::{self, Card};
use crate::view::View;
use crate::{DoorError, Live, StepFailure};

pub const MAX_TRAJECTORY_STEPS: u64 = 1_000_000;
pub const MAX_RECORDED_FRAMES: u64 = 10_000;

pub(crate) const TOP_LEVEL: &[&str] = &[
    "domain",
    "card",
    "initial",
    "dt_s",
    "segments",
    "record_every",
];

#[derive(Serialize, Clone)]
pub struct RecordedFrame {
    pub step: u64,
    pub frame_hex: String,
    pub link: String,
}

#[derive(Serialize)]
pub struct TrajectoryResponse {
    pub inputs: Value,
    pub engine_version: String,
    pub outcome: String,
    pub steps_requested: u64,
    pub steps_landed: u64,
    pub reject: Option<String>,
    pub reject_detail: Option<String>,
    #[serde(serialize_with = "serialize_optional_state")]
    pub final_state: Option<Value>,
    #[serde(serialize_with = "serialize_optional_f64")]
    pub final_residual_conservation_error: Option<f64>,
    pub final_residual_conservation_error_hex: Option<String>,
    #[serde(serialize_with = "serialize_optional_f64")]
    pub max_residual_conservation_error: Option<f64>,
    pub max_residual_conservation_error_hex: Option<String>,
    pub state_digest: Option<String>,
    /// Present only when the request asked for a readout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readout: Option<crate::readout::Readout>,
    pub frames: Vec<RecordedFrame>,
    pub run_seal: String,
    /// Present only when the request named its upstream in `from`: the
    /// chain commitment, outside the run seal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<Value>,
    pub receipt_sha256: String,
}

#[derive(Serialize)]
pub struct TapeReport {
    pub holds: bool,
    pub broken_link: Option<u64>,
    /// `chain`, `run_seal`, `state`, or null when the tape holds.
    pub cause: Option<String>,
}

/// A segment's step count. Its load stays in `Prepared::request`, at
/// `segments[i].load`, and is read from there: a copy of every load would hold
/// the request's loads twice.
struct Segment {
    steps: u64,
}

pub(crate) struct Prepared {
    request: Value,
    steps_requested: u64,
    record_every: u64,
    segments: Vec<Segment>,
    live: Live,
}

#[derive(Serialize)]
struct RunTail<'a> {
    outcome: &'a str,
    steps_landed: u64,
    reject: Option<&'a str>,
    state_digest: Option<&'a str>,
}

const CAUSE_CHAIN: &str = "chain";
const CAUSE_RUN_SEAL: &str = "run_seal";
const CAUSE_STATE: &str = "state";

pub async fn run(state: &crate::AppState, request: Value) -> Result<TrajectoryResponse, DoorError> {
    // The readout is not part of the run: the request without it is the one
    // that is prepared, run and sealed. Nor is `from`: a chained run is the
    // same run as the request typed by hand, and its chain is beside it.
    let (request, readout) = crate::readout::take(request);
    let (request, from) = crate::chain::take(request);
    if let Some(from) = from {
        return Box::pin(run_chained(state, request, readout, from)).await;
    }
    let prepared = prepare_in(state.view(), request)?;
    let readout = match readout {
        None => None,
        Some(asked) => Some(crate::readout::Plan::read(
            &asked,
            &prepared.request,
            prepared.steps_requested,
        )?),
    };
    crate::on_pool(state, move || execute_read(prepared, readout)).await
}

/// A run whose request names its upstream in `from`. The request is read
/// and held to its own rules first, the chain is read and held to its
/// limits, and then, under one physics permit, every upstream run is computed
/// again and checked, the handed values are checked, and the run is made.
async fn run_chained(
    state: &crate::AppState,
    request: Value,
    readout: Option<Value>,
    from: Value,
) -> Result<TrajectoryResponse, DoorError> {
    // The request's own shape and numbers are named first; the card's start
    // rules, which a stated ledger meets, come after the chain is checked.
    unknown_key_first(state.view(), &request).map_err(DoorError::Unprocessable)?;
    crate::validate_physics(&request).map_err(DoorError::Unprocessable)?;
    let steps = request
        .get("segments")
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter_map(|segment| segment.get("steps").and_then(Value::as_u64))
                .fold(0_u64, u64::saturating_add)
        })
        .unwrap_or(0);
    let plan = crate::chain::read(state.view(), &request, from, crate::chain::Target::Run, steps)?;
    let view = state.view().clone();
    crate::on_pool(state, move || {
        let carried = plan.carried();
        let (settled, request) = plan.settle()?;
        let prepared = prepare_continued(&view, request, &carried).map_err(|error| error.message().to_string())?;
        let readout = match readout {
            None => None,
            Some(asked) => Some(
                crate::readout::Plan::read(&asked, &prepared.request, prepared.steps_requested)
                    .map_err(|error| error.message().to_string())?,
            ),
        };
        let mut response = execute_read(prepared, readout)?;
        response.chain = Some(settled.chain_value(&response.run_seal));
        seal_trajectory(response)
    })
    .await
}

#[cfg(test)]
pub fn check(body: &Value) -> Result<TapeReport, DoorError> {
    check_in(&View::full(), body)
}

/// `verify_tape` as every connection answers it, as JSON text: a check by
/// request (`request` and `run_seal`), which runs the request again under a
/// physics permit, or the check of a whole answer, which runs no physics.
pub async fn verify(state: &crate::AppState, body: &Value) -> Result<String, DoorError> {
    let object = body.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    if object.contains_key(CHAIN_LINKS) && !object.contains_key(crate::check_by_request::REQUEST) {
        return check_chain_in(state.view(), body);
    }
    let chain_seal_given = if object.contains_key(crate::check_by_request::REQUEST) {
        match object.get(CHAIN_SEAL) {
            None => None,
            Some(Value::String(text)) if crate::json_text::is_lower_hex(text, 64) => Some(text.clone()),
            Some(_) => {
                return Err(DoorError::Unprocessable(
                    "Physical Violation: 'chain_seal' must be 64 lowercase hex digits.".to_string(),
                ));
            }
        }
    } else {
        None
    };
    let mut short = object.clone();
    short.shift_remove(CHAIN_SEAL);
    let Some((request, given)) = crate::check_by_request::read(&short, "run_seal", "the whole run_trajectory answer")? else {
        let report = check_in(state.view(), body)?;
        return serde_json::to_string(&report).map_err(|error| {
            DoorError::Internal(format!(
                "Physical Violation: the response could not be encoded. {error}"
            ))
        });
    };
    let response = run(state, request.clone()).await?;
    let recomputed_chain = response
        .chain
        .as_ref()
        .and_then(|chain| chain.get(CHAIN_SEAL))
        .and_then(Value::as_str)
        .map(str::to_string);
    if chain_seal_given.is_some() && recomputed_chain.is_none() {
        return Err(DoorError::Unprocessable(
            "Physical Violation: 'chain_seal' is given and the request names no upstream in 'from', so it has no chain seal.".to_string(),
        ));
    }
    let text = crate::check_by_request::answer(
        "run_seal",
        "run_trajectory",
        given,
        &response.run_seal,
        Some(&response.receipt_sha256),
        &crate::check_by_request::checked_with_receipt("run", "run_seal"),
        crate::check_by_request::TEXT_NOT_COMPARED,
    )?;
    let Some(recomputed_chain) = recomputed_chain else {
        return Ok(text);
    };
    // A chained request: the chain seal computed again is reported, and,
    // when one is given, compared too.
    let mut report: Map<String, Value> = serde_json::from_str(&text).map_err(|error| {
        DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}"))
    })?;
    let run_holds = report.get("holds") == Some(&Value::Bool(true));
    let chain_holds = chain_seal_given.as_deref().is_none_or(|given| given == recomputed_chain);
    report.insert("holds".into(), Value::Bool(run_holds && chain_holds));
    if run_holds && !chain_holds {
        // The run's seal held and the chain's did not: neither seal computed
        // is handed back.
        report.insert("cause".into(), Value::String(CHAIN_SEAL.to_string()));
        report.shift_remove("recomputed_run_seal");
        report.shift_remove("recomputed_receipt_sha256");
        report.insert(
            "not_held".into(),
            Value::String(crate::check_by_request::not_held(CHAIN_SEAL, "run_trajectory")),
        );
    }
    let checked = report.shift_remove("checked");
    let text_not_compared = report.shift_remove("text_not_compared");
    // The chain seal computed again is returned only beside the chain seal
    // it was compared with: a seal nobody gave is not handed back.
    if run_holds && chain_holds && chain_seal_given.is_some() {
        report.insert("recomputed_chain_seal".into(), Value::String(recomputed_chain));
    }
    report.insert(
        "chain_checked".into(),
        Value::String(
            if chain_seal_given.is_some() { CHAIN_CHECKED } else { CHAIN_CHECKED_WITHOUT_SEAL }.to_string(),
        ),
    );
    if let Some(checked) = checked {
        report.insert("checked".into(), checked);
    }
    if let Some(text_not_compared) = text_not_compared {
        report.insert("text_not_compared".into(), text_not_compared);
    }
    serde_json::to_string(&report).map_err(|error| {
        DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}"))
    })
}

/// The key of a whole chain sent to `verify_tape`.
pub(crate) const CHAIN_LINKS: &str = "links";
/// The key of a chain seal in a check by request.
pub(crate) const CHAIN_SEAL: &str = "chain_seal";

/// What a check by request of a chained run compared, beside the run seal.
pub(crate) const CHAIN_CHECKED: &str = "Every upstream run in 'from' was computed again and its run_seal compared with the seal given, and each handed value was compared bit for bit with the upstream's sealed answer and with the request. The chain_seal given was compared with the chain seal of that computation, and holds covers that comparison; recomputed_chain_seal, given when the check holds, is that chain seal.";

/// What a check by request of a chained run compared when no chain seal was
/// given.
pub(crate) const CHAIN_CHECKED_WITHOUT_SEAL: &str = "Every upstream run in 'from' was computed again and its run_seal compared with the seal given, and each handed value was compared bit for bit with the upstream's sealed answer and with the request. No chain_seal was given, so the chain seal was not compared and is not returned: holds covers the run seal and the upstream runs alone, and run_trajectory's answer to this request carries the chain seal in chain.chain_seal.";

/// `verify_tape` given `{"links": [...]}`: the answers of a chain in order,
/// checked with no physics by the rule of `genesis_seal::chain`. A link that
/// names a pair outside the view is read as a pair the engine does not have.
fn check_chain_in(view: &View, body: &Value) -> Result<String, DoorError> {
    if let Some(links) = body.get(CHAIN_LINKS).and_then(Value::as_array) {
        for link in links {
            if let Some((domain, card)) = link.get("inputs").and_then(card_pair) {
                if view.find(domain, card).is_none() {
                    return Err(DoorError::NotFound("Card not recognized".to_string()));
                }
            }
        }
    }
    let report = genesis_seal::chain::check_chain(body)
        .map_err(|refusal| DoorError::Unprocessable(refusal.to_string()))?;
    let mut object = Map::new();
    object.insert("holds".into(), Value::Bool(report.holds));
    object.insert("links".into(), Value::from(report.links as u64));
    object.insert(
        "broken_link".into(),
        report.broken_link.map(|index| Value::from(index as u64)).unwrap_or(Value::Null),
    );
    object.insert(
        "cause".into(),
        report.cause.map(|cause| Value::String(cause.as_str().to_string())).unwrap_or(Value::Null),
    );
    object.insert(
        "chain_seal".into(),
        report.chain_seal.map(Value::String).unwrap_or(Value::Null),
    );
    object.insert("checked".into(), Value::String(chain_checked_sentence(report.last_is_step)));
    serde_json::to_string(&Value::Object(object)).map_err(|error| {
        DoorError::Internal(format!("Physical Violation: the response could not be encoded. {error}"))
    })
}

fn chain_checked_sentence(last_is_step: bool) -> String {
    let mut text = String::from("No physics was run. Each run's frames, state digest and run seal were recomputed from its answer; each link was checked to name the run_seal and chain_seal of the link before it; each handed value was compared bit for bit with the upstream's sealed final state or request and with the downstream request; and each chain_seal was rebuilt. This shows the chain is consistent with itself; sending the last request with its run_seal and chain_seal shows the engine made it and that the coupling is one it serves.");
    if last_is_step {
        text.push_str(" The last link is a step: its answer holds no frame, so its sha256_seal is shown by sending its request to verify_step again.");
    }
    text
}

/// A tape that names a pair outside the view is read as a tape that names a
/// pair the engine does not have.
pub fn check_in(view: &View, body: &Value) -> Result<TapeReport, DoorError> {
    let object = body.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    if let Some(meant) = crate::check_by_request::meant_by_request(object, "run_seal", "frames") {
        return Err(meant);
    }
    let inputs = required(object, "inputs")?;
    let frames = required(object, "frames")?.as_array().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: 'frames' must be a JSON array.".to_string())
    })?;
    let outcome = require_string(object, "outcome")?;
    let steps_landed = require_landed(object)?;
    let reject = require_reject(object)?;
    let run_seal = require_string(object, "run_seal")?;
    let final_state = required(object, "final_state")?;
    if !(final_state.is_null() || final_state.is_object()) {
        return Err(DoorError::Unprocessable(
            "Physical Violation: 'final_state' must be a JSON object or null.".to_string(),
        ));
    }
    let final_hex = require_optional_string(object, "final_residual_conservation_error_hex")?;
    let max_hex = require_optional_string(object, "max_residual_conservation_error_hex")?;
    let state_digest = require_optional_string(object, "state_digest")?;
    require_exact_tape(
        view,
        inputs,
        frames,
        &run_seal,
        state_digest.as_deref(),
        final_state,
        final_hex.as_deref(),
        max_hex.as_deref(),
    )?;
    Ok(recompute(
        inputs,
        frames,
        &outcome,
        steps_landed,
        reject.as_deref(),
        &run_seal,
        final_state,
        final_hex.as_deref(),
        max_hex.as_deref(),
        state_digest.as_deref(),
    ))
}

pub(crate) fn prepare(request: Value) -> Result<Prepared, DoorError> {
    prepare_in(&View::full(), request)
}

/// A key the request should not carry is named as unknown before any rule
/// that reads a key's name for its meaning (a key with "temp" in it, for one).
/// Only the parts that can be read are looked at; a part that cannot be read
/// is left to the checks that follow.
fn unknown_key_first(view: &View, request: &Value) -> Result<(), String> {
    let Some(object) = request.as_object() else {
        return Ok(());
    };
    reject_unknown_keys(object, TOP_LEVEL)?;
    let (Some(Value::String(domain)), Some(Value::String(card))) =
        (object.get("domain"), object.get("card"))
    else {
        return Ok(());
    };
    if view.find(domain, card).is_none() {
        return Ok(());
    }
    if let Some(initial) = object.get("initial").and_then(Value::as_object) {
        reject_unknown_keys(initial, descriptor::phase_names(domain, card))?;
    }
    for segment in object
        .get("segments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(segment) = segment.as_object() else {
            continue;
        };
        reject_unknown_keys(segment, &["steps", "load"])?;
        if let Some(load) = segment.get("load").and_then(Value::as_object) {
            reject_unknown_keys(load, descriptor::load_names(domain, card))?;
        }
    }
    Ok(())
}

pub(crate) fn prepare_in(view: &View, request: Value) -> Result<Prepared, DoorError> {
    prepare_with(view, request, &crate::fresh_start::Carried::Nothing)
}

/// A request that names its upstream in `from`, which the chain has checked:
/// the start fields `carried` names carry the upstream's values, so they are
/// not held to a fresh start; the others still are.
pub(crate) fn prepare_continued(
    view: &View,
    request: Value,
    carried: &crate::fresh_start::Carried,
) -> Result<Prepared, DoorError> {
    prepare_with(view, request, carried)
}

fn prepare_with(view: &View, request: Value, carried: &crate::fresh_start::Carried) -> Result<Prepared, DoorError> {
    unknown_key_first(view, &request).map_err(DoorError::Unprocessable)?;
    crate::validate_physics(&request).map_err(DoorError::Unprocessable)?;
    let object = request.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    reject_unknown_keys(object, TOP_LEVEL).map_err(DoorError::Unprocessable)?;
    let domain = require_string(object, "domain")?;
    let card = require_string(object, "card")?;
    let spec = view
        .find(&domain, &card)
        .ok_or_else(|| DoorError::NotFound("Card not recognized".to_string()))?;

    let initial = required_object(object, "initial")?;
    reject_unknown_keys(initial, descriptor::phase_names(&domain, &card))
        .map_err(DoorError::Unprocessable)?;
    let dt = required(object, "dt_s")?.clone();
    let record_every = require_count(object, "record_every").map_err(DoorError::Unprocessable)?;
    let segment_values = required(object, "segments")?.as_array().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: 'segments' must be a JSON array.".to_string())
    })?;
    if segment_values.is_empty() {
        return Err(DoorError::Unprocessable(
            "Physical Violation: 'segments' must contain at least one segment.".to_string(),
        ));
    }

    let mut segments = Vec::with_capacity(segment_values.len());
    let mut steps_requested = 0_u64;
    for segment in segment_values {
        let segment = segment.as_object().ok_or_else(|| {
            DoorError::Unprocessable(
                "Physical Violation: each segment must be a JSON object.".to_string(),
            )
        })?;
        reject_unknown_keys(segment, &["steps", "load"]).map_err(DoorError::Unprocessable)?;
        let steps = require_count(segment, "steps").map_err(DoorError::Unprocessable)?;
        steps_requested = steps_requested.checked_add(steps).ok_or_else(|| {
            DoorError::Unprocessable(
                "Physical Violation: the run asks for more than 1000000 steps.".to_string(),
            )
        })?;
        let load = required_object(segment, "load")?;
        reject_unknown_keys(load, descriptor::load_names(&domain, &card))
            .map_err(DoorError::Unprocessable)?;
        segments.push(Segment { steps });
    }
    if steps_requested > MAX_TRAJECTORY_STEPS {
        return Err(DoorError::Unprocessable(
            "Physical Violation: the run asks for more than 1000000 steps.".to_string(),
        ));
    }
    let frames = recorded_if_complete(steps_requested, record_every).ok_or_else(|| {
        DoorError::Unprocessable(
            "Physical Violation: the run would record more than 10000 frames.".to_string(),
        )
    })?;
    if frames > MAX_RECORDED_FRAMES {
        return Err(DoorError::Unprocessable(
            "Physical Violation: the run would record more than 10000 frames.".to_string(),
        ));
    }

    let merged = merge_parameters(spec, initial, &segment_values[0]["load"], &dt);
    let mut live = Live::open(&domain, &card, &merged).map_err(DoorError::Unprocessable)?;
    for segment in segment_values.iter().skip(1) {
        live.replace_load(&segment["load"])
            .map_err(DoorError::Unprocessable)?;
    }
    live.replace_load(&segment_values[0]["load"])
        .map_err(DoorError::Unprocessable)?;
    // A start that carries a run on arrives with `from`; the card's own
    // start rules, read as the specimen opened, are named first, and the
    // fields `from` carries are the upstream's.
    let first_load = segment_values[0]["load"].as_object().cloned().unwrap_or_default();
    crate::fresh_start::check(spec.card, initial, &first_load, "initial", "segments[0].load", carried)
        .map_err(DoorError::Unprocessable)?;
    crate::fresh_start::check_anchors(spec.card, &live, "initial", carried).map_err(DoorError::Unprocessable)?;
    Ok(Prepared {
        request,
        steps_requested,
        record_every,
        segments,
        live,
    })
}

impl Prepared {
    /// The request as it is sealed.
    #[allow(dead_code)]
    pub(crate) fn request(&self) -> &Value {
        &self.request
    }

    /// The steps the run asks for in all.
    pub(crate) fn steps_requested(&self) -> u64 {
        self.steps_requested
    }
}

/// Called after each landed step, with the step number and the specimen as it
/// stands. A step that refuses is not landed and is not shown.
pub(crate) type Observer<'a> = &'a mut dyn FnMut(u64, &Live) -> Result<(), String>;

pub(crate) fn execute(prepared: Prepared) -> Result<TrajectoryResponse, String> {
    execute_observed(prepared, None)
}

/// The run of `execute`, with the readout asked for, when one is.
fn execute_read(
    prepared: Prepared,
    readout: Option<crate::readout::Plan>,
) -> Result<TrajectoryResponse, String> {
    let Some(mut plan) = readout else {
        return execute(prepared);
    };
    let mut observe = |landed: u64, live: &Live| plan.observe(landed, live);
    let mut response = execute_observed(prepared, Some(&mut observe))?;
    response.readout = Some(plan.finish());
    seal_trajectory(response)
}

/// The sentence of a run refused part way, with the last sentence of the
/// card's own words, that the state is not sealed, made to name the steps:
/// first the last landed step the run is sealed to, then the refused state,
/// at the step after it.
pub(crate) fn refused_detail(detail: &str, steps_landed: u64) -> String {
    let named = if steps_landed == 0 {
        "The run is sealed with no step landed. The state at step 1 was refused and is not sealed.".to_string()
    } else {
        format!(
            "The run is sealed to step {steps_landed}. The state at step {} was refused and is not sealed.",
            steps_landed + 1
        )
    };
    let body = match detail.strip_suffix(" state is not sealed.") {
        // Cut the card's own last sentence ("That state is not sealed.", "A
        // molten state is not sealed.") at its start.
        Some(head) => match head.rfind(". ") {
            Some(at) => &detail[..at + 1],
            None => detail,
        },
        None => detail,
    };
    format!("{body} {named}")
}

/// The run of `execute`, with the specimen shown to `observer` after each landed
/// step. The observer reads and changes nothing of the run.
pub(crate) fn execute_observed(
    mut prepared: Prepared,
    mut observer: Option<Observer<'_>>,
) -> Result<TrajectoryResponse, String> {
    let mut link = digest_bytes(crate::sealed_form(&prepared.request).as_bytes());
    let mut frames = Vec::new();
    let mut pending: Option<(u64, LastStateFrame64)> = None;
    let mut steps_landed = 0_u64;
    let mut last_residual: Option<f64> = None;
    let mut max_residual: Option<f64> = None;
    let mut refusal: Option<StepFailure> = None;

    'run: for (index, segment) in prepared.segments.iter().enumerate() {
        prepared
            .live
            .replace_load(&prepared.request["segments"][index]["load"])?;
        for _ in 0..segment.steps {
            let snapshot = prepared.live.snapshot();
            let mut frame = zero_frame();
            if let Err(failure) = prepared.live.step_into(&mut frame) {
                prepared.live.restore(snapshot);
                refusal = Some(failure);
                break 'run;
            }
            match prepared.live.audit(&frame) {
                Ok(residual) => {
                    steps_landed += 1;
                    max_residual = Some(match max_residual {
                        Some(current) if residual > current => residual,
                        Some(current) => current,
                        None => residual,
                    });
                    last_residual = Some(residual);
                    if let Some(observe) = observer.as_mut() {
                        observe(steps_landed, &prepared.live)?;
                    }
                    if steps_landed % prepared.record_every == 0 {
                        record(&mut frames, &mut link, steps_landed, &mut frame);
                        pending = None;
                    } else {
                        pending = Some((steps_landed, frame));
                    }
                }
                Err(failure) => {
                    prepared.live.restore(snapshot);
                    refusal = Some(failure);
                    break 'run;
                }
            }
        }
    }
    if let Some((step, mut frame)) = pending {
        record(&mut frames, &mut link, step, &mut frame);
    }
    let last_state = if steps_landed == 0 {
        None
    } else {
        Some(prepared.live.publish().map_err(|failure| failure.detail)?)
    };

    let card = prepared.request["card"].as_str().unwrap_or_default().to_string();
    let (outcome, reject, reject_detail) = match refusal {
        None => ("completed", None, None),
        Some(failure) => {
            // The specimen stands at its last landed state, or at its start.
            let incoming = match &last_state {
                Some(state) => Some(state.clone()),
                None => prepared.live.publish().ok(),
            };
            let end = failure
                .reject_name
                .as_deref()
                .and_then(|name| crate::ends::declared_end(&card, name, incoming.as_ref()));
            match end {
                Some(end) => (
                    "ended",
                    failure.reject_name,
                    Some(crate::ends::sentence(&card, end, steps_landed)),
                ),
                None => (
                    "refused",
                    failure.reject_name,
                    Some(refused_detail(&failure.detail, steps_landed)),
                ),
            }
        }
    };
    let final_hex = hex_of(last_residual);
    let max_hex = hex_of(max_residual);
    let state_digest = match &last_state {
        None => None,
        Some(state) => Some(digest_state(
            state.as_object().expect("published state is an object"),
            final_hex.as_deref().expect("a landed step has a residual"),
            max_hex
                .as_deref()
                .expect("a landed step has a max residual"),
        )?),
    };
    let run_seal = seal_run(
        &link,
        outcome,
        steps_landed,
        reject.as_deref(),
        state_digest.as_deref(),
    );
    seal_trajectory(TrajectoryResponse {
        inputs: prepared.request,
        engine_version: crate::ENGINE_VERSION.to_string(),
        outcome: outcome.to_string(),
        steps_requested: prepared.steps_requested,
        steps_landed,
        reject,
        reject_detail,
        final_state: last_state,
        final_residual_conservation_error: last_residual,
        final_residual_conservation_error_hex: final_hex,
        max_residual_conservation_error: max_residual,
        max_residual_conservation_error_hex: max_hex,
        state_digest,
        readout: None,
        frames,
        run_seal,
        chain: None,
        receipt_sha256: String::new(),
    })
}

/// A hash is 64 lowercase hex digits. A state hex is 16, or an array of those.
/// A frame's `frame_hex` (128 digits) and `link` (64) written with capital
/// letters are refused by name; any other fault in a frame is a chain failure.
fn require_exact_tape(
    view: &View,
    inputs: &Value,
    frames: &[Value],
    run_seal: &str,
    state_digest: Option<&str>,
    final_state: &Value,
    final_hex: Option<&str>,
    max_hex: Option<&str>,
) -> Result<(), DoorError> {
    if !crate::json_text::is_lower_hex(run_seal, 64) {
        return Err(exact_digits("run_seal", 64));
    }
    if let Some(text) = state_digest {
        if !crate::json_text::is_lower_hex(text, 64) {
            return Err(exact_digits("state_digest", 64));
        }
    }
    if let Some(text) = final_hex {
        if !crate::json_text::is_lower_hex(text, 16) {
            return Err(exact_digits("final_residual_conservation_error_hex", 16));
        }
    }
    if let Some(text) = max_hex {
        if !crate::json_text::is_lower_hex(text, 16) {
            return Err(exact_digits("max_residual_conservation_error_hex", 16));
        }
    }
    if let Some(state) = final_state.as_object() {
        require_state_exact("final_state", state, card_pair(inputs).map(|(d, c)| view.resolve(d, c)))?;
    }
    for (index, frame) in frames.iter().enumerate() {
        let Some(object) = frame.as_object() else {
            continue;
        };
        for (key, digits) in [("frame_hex", 128), ("link", 64)] {
            let capital = object
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(crate::json_text::has_capital_hex);
            if capital {
                return Err(exact_digits(&format!("frames[{index}].{key}"), digits));
            }
        }
    }
    Ok(())
}

fn card_pair(inputs: &Value) -> Option<(&str, &str)> {
    let object = inputs.as_object()?;
    let domain = object.get("domain")?.as_str()?;
    let card = object.get("card")?.as_str()?;
    Some((domain, card))
}

fn require_state_exact(
    section: &str,
    state: &Map<String, Value>,
    pair: Option<(&str, &str)>,
) -> Result<(), DoorError> {
    for (key, value) in state {
        if !crate::json_text::is_plain_field_name(key) {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{section}.{key}' is not a field name."
            )));
        }
        match pair {
            Some((domain, card)) => match crate::state_field_is_published(domain, card, key)
            {
                Some(true) => {}
                Some(false) => {
                    return Err(DoorError::Unprocessable(format!(
                        "Physical Violation: '{section}.{key}' is not published by this card."
                    )));
                }
                None => {
                    return Err(DoorError::Unprocessable(
                        "Physical Violation: the tape does not name a card.".to_string(),
                    ));
                }
            },
            None if state.is_empty() => {}
            None => {
                return Err(DoorError::Unprocessable(
                    "Physical Violation: the tape does not name a card.".to_string(),
                ));
            }
        }
        if key.ends_with("_hex") {
            if !is_f64_hex(value) {
                return Err(exact_digits(&format!("{section}.{key}"), 16));
            }
        } else if crate::json_text::is_flag(key) && !value.is_boolean() {
            return Err(DoorError::Unprocessable(format!(
                "Physical Violation: '{section}.{key}' must be a JSON boolean."
            )));
        }
    }
    Ok(())
}

fn is_f64_hex(value: &Value) -> bool {
    match value {
        Value::String(text) => crate::json_text::is_lower_hex(text, 16),
        Value::Array(items) if !items.is_empty() => items.iter().all(|item| {
            item.as_str()
                .is_some_and(|text| crate::json_text::is_lower_hex(text, 16))
        }),
        _ => false,
    }
}

fn exact_digits(name: &str, digits: usize) -> DoorError {
    DoorError::Unprocessable(format!(
        "Physical Violation: '{name}' must be {digits} lowercase hex digits."
    ))
}

fn recompute(
    inputs: &Value,
    frames: &[Value],
    outcome: &str,
    steps_landed: u64,
    reject: Option<&str>,
    run_seal: &str,
    final_state: &Value,
    final_hex: Option<&str>,
    max_hex: Option<&str>,
    state_digest: Option<&str>,
) -> TapeReport {
    let mut link = digest_bytes(crate::sealed_form(inputs).as_bytes());
    for (index, frame) in frames.iter().enumerate() {
        let Some(object) = frame.as_object() else {
            return broken(index);
        };
        let Some(frame_hex) = object.get("frame_hex").and_then(Value::as_str) else {
            return broken(index);
        };
        let Some(stored_link) = object.get("link").and_then(Value::as_str) else {
            return broken(index);
        };
        let Some(bytes) = decode_fixed(frame_hex, 64) else {
            return broken(index);
        };
        if bytes[48..64] != link[..16] {
            return broken(index);
        }
        let digest = digest_bytes(&bytes);
        let Some(stored) = decode_fixed(stored_link, 32) else {
            return broken(index);
        };
        if stored.as_slice() != digest.as_slice() {
            return broken(index);
        }
        link = digest;
    }
    let expected = match expected_state_digest(steps_landed, final_state, final_hex, max_hex) {
        Some(digest) => digest,
        None => return state_broken(),
    };
    if expected.as_deref() != state_digest {
        return state_broken();
    }
    let seal = seal_run(&link, outcome, steps_landed, reject, expected.as_deref());
    if seal != run_seal {
        return TapeReport {
            holds: false,
            broken_link: Some(frames.len() as u64),
            cause: Some(CAUSE_RUN_SEAL.to_string()),
        };
    }
    TapeReport {
        holds: true,
        broken_link: None,
        cause: None,
    }
}

fn broken(index: usize) -> TapeReport {
    TapeReport {
        holds: false,
        broken_link: Some(index as u64),
        cause: Some(CAUSE_CHAIN.to_string()),
    }
}

fn state_broken() -> TapeReport {
    TapeReport {
        holds: false,
        broken_link: None,
        cause: Some(CAUSE_STATE.to_string()),
    }
}

fn record(
    frames: &mut Vec<RecordedFrame>,
    link: &mut [u8; 32],
    step: u64,
    frame: &mut LastStateFrame64,
) {
    frame.proof.copy_from_slice(&link[..16]);
    let bytes = frame.to_bytes();
    let digest = digest_bytes(&bytes);
    *link = digest;
    frames.push(RecordedFrame {
        step,
        frame_hex: hex::encode(bytes),
        link: hex::encode(digest),
    });
}

fn seal_run(
    link: &[u8; 32],
    outcome: &str,
    steps_landed: u64,
    reject: Option<&str>,
    state_digest: Option<&str>,
) -> String {
    let link_hex = hex::encode(link);
    let tail = serde_json::to_vec(&RunTail {
        outcome,
        steps_landed,
        reject,
        state_digest,
    })
    .expect("run tail encodes");
    let mut bytes = link_hex.into_bytes();
    bytes.extend_from_slice(&tail);
    crate::digest_hex(&bytes)
}

fn hex_of(value: Option<f64>) -> Option<String> {
    value.map(|value| format!("{:016x}", value.to_bits()))
}

/// `Some(None)` is a null digest. `None` means the posted state does not match
/// a digest this tape can hold.
fn expected_state_digest(
    steps_landed: u64,
    final_state: &Value,
    final_hex: Option<&str>,
    max_hex: Option<&str>,
) -> Option<Option<String>> {
    if steps_landed == 0 {
        if final_state.is_null() && final_hex.is_none() && max_hex.is_none() {
            Some(None)
        } else {
            None
        }
    } else {
        let state = final_state.as_object()?;
        let final_hex = final_hex?;
        let max_hex = max_hex?;
        digest_state(state, final_hex, max_hex).ok().map(Some)
    }
}

fn digest_state(
    state: &Map<String, Value>,
    final_hex: &str,
    max_hex: &str,
) -> Result<String, String> {
    let text = state_digest_text(state, final_hex, max_hex)?;
    Ok(crate::digest_hex(text.as_bytes()))
}

fn state_digest_text(
    state: &Map<String, Value>,
    final_hex: &str,
    max_hex: &str,
) -> Result<String, String> {
    let mut lines = Vec::new();
    for (key, value) in state {
        if key.ends_with("_hex") {
            lines.push((key.clone(), render_hex(value)?));
        } else if value.is_boolean() {
            lines.push((
                key.clone(),
                if value.as_bool().expect("boolean") {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ));
        }
    }
    lines.sort_by(|left, right| left.0.cmp(&right.0));
    let mut text = String::new();
    for (key, value) in lines {
        text.push_str(&key);
        text.push(':');
        text.push_str(&value);
        text.push('\n');
    }
    text.push_str("final_residual_conservation_error_hex:");
    text.push_str(final_hex);
    text.push('\n');
    text.push_str("max_residual_conservation_error_hex:");
    text.push_str(max_hex);
    text.push('\n');
    Ok(text)
}

fn render_hex(value: &Value) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Array(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                let Some(text) = item.as_str() else {
                    return Err(
                        "Physical Violation: a state hex array entry is not a string.".to_string(),
                    );
                };
                parts.push(text);
            }
            Ok(parts.join(","))
        }
        _ => Err("Physical Violation: a state hex field is not a string or an array.".to_string()),
    }
}

/// The response before its receipt, by reference: sealing it copies none of
/// the request, the state or the frames.
#[derive(Serialize)]
struct TrajectoryDocument<'a> {
    inputs: &'a Value,
    engine_version: &'a str,
    outcome: &'a str,
    steps_requested: u64,
    steps_landed: u64,
    reject: &'a Option<String>,
    reject_detail: &'a Option<String>,
    #[serde(serialize_with = "serialize_optional_state")]
    final_state: &'a Option<Value>,
    #[serde(serialize_with = "serialize_optional_f64")]
    final_residual_conservation_error: Option<f64>,
    final_residual_conservation_error_hex: &'a Option<String>,
    #[serde(serialize_with = "serialize_optional_f64")]
    max_residual_conservation_error: Option<f64>,
    max_residual_conservation_error_hex: &'a Option<String>,
    state_digest: &'a Option<String>,
    #[serde(skip_serializing_if = "no_readout")]
    readout: &'a Option<crate::readout::Readout>,
    frames: &'a Vec<RecordedFrame>,
    run_seal: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    chain: &'a Option<Value>,
}

fn no_readout(readout: &&Option<crate::readout::Readout>) -> bool {
    readout.is_none()
}

/// Takes bytes written to it and says whether they are, in order, the bytes
/// of `head` followed by those of `tail`. It keeps none of them.
struct SameAs<'a> {
    head: &'a [u8],
    tail: &'a [u8],
    at: usize,
    same: bool,
}

impl std::io::Write for SameAs<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let end = self.at + bytes.len();
        let h = self.head.len();
        if end > h + self.tail.len() {
            self.same = false;
        } else if self.same {
            let split = h.saturating_sub(self.at).min(bytes.len());
            let (first, second) = bytes.split_at(split);
            let in_tail = (self.at + split).saturating_sub(h);
            if (split > 0 && first != &self.head[self.at..self.at + split])
                || second != &self.tail[in_tail..in_tail + second.len()]
            {
                self.same = false;
            }
        }
        self.at = end;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn seal_trajectory(mut response: TrajectoryResponse) -> Result<TrajectoryResponse, String> {
    let encode_error =
        |error: serde_json::Error| format!("Physical Violation: the response could not be encoded. {error}");
    let document = serde_json::to_string(&TrajectoryDocument {
        inputs: &response.inputs,
        engine_version: &response.engine_version,
        outcome: &response.outcome,
        steps_requested: response.steps_requested,
        steps_landed: response.steps_landed,
        reject: &response.reject,
        reject_detail: &response.reject_detail,
        final_state: &response.final_state,
        final_residual_conservation_error: response.final_residual_conservation_error,
        final_residual_conservation_error_hex: &response.final_residual_conservation_error_hex,
        max_residual_conservation_error: response.max_residual_conservation_error,
        max_residual_conservation_error_hex: &response.max_residual_conservation_error_hex,
        state_digest: &response.state_digest,
        readout: &response.readout,
        frames: &response.frames,
        run_seal: &response.run_seal,
        chain: &response.chain,
    })
    .map_err(encode_error)?;
    response.receipt_sha256 = crate::digest_hex(document.as_bytes());
    // The response with its receipt must be the document with the receipt
    // appended. It is written to a comparison and not to a string.
    debug_assert!(document.ends_with('}'));
    let tail = format!(",\"receipt_sha256\":\"{}\"}}", response.receipt_sha256);
    let mut compare = SameAs {
        head: &document.as_bytes()[..document.len() - 1],
        tail: tail.as_bytes(),
        at: 0,
        same: true,
    };
    serde_json::to_writer(&mut compare, &response).map_err(encode_error)?;
    if !compare.same || compare.at != compare.head.len() + compare.tail.len() {
        return Err(
            "Physical Violation: the receipt does not cover the response bytes.".to_string(),
        );
    }
    Ok(response)
}

fn digest_bytes(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn decode_fixed(text: &str, len: usize) -> Option<Vec<u8>> {
    let bytes = hex::decode(text).ok()?;
    if bytes.len() == len {
        Some(bytes)
    } else {
        None
    }
}

fn zero_frame() -> LastStateFrame64 {
    LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof: [0; 16],
    }
}

fn recorded_if_complete(steps: u64, every: u64) -> Option<u64> {
    let periodic = steps / every;
    if steps % every == 0 {
        Some(periodic)
    } else {
        periodic.checked_add(1)
    }
}

fn merge_parameters(
    spec: &Card,
    initial: &Map<String, Value>,
    load: &Value,
    dt: &Value,
) -> Value {
    let mut merged = Map::new();
    for name in &spec.phase_names {
        if let Some(value) = initial.get(*name) {
            merged.insert((*name).to_string(), value.clone());
        }
    }
    if let Some(load) = load.as_object() {
        for name in &spec.load_names {
            if let Some(value) = load.get(*name) {
                merged.insert((*name).to_string(), value.clone());
            }
        }
    }
    merged.insert("dt_s".to_string(), dt.clone());
    Value::Object(merged)
}

fn required<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a Value, DoorError> {
    object
        .get(key)
        .ok_or_else(|| DoorError::Unprocessable(format!("Missing required parameter: '{key}'")))
}

fn required_object<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a Map<String, Value>, DoorError> {
    required(object, key)?.as_object().ok_or_else(|| {
        DoorError::Unprocessable(format!(
            "Physical Violation: '{key}' must be a JSON object."
        ))
    })
}

fn require_string(object: &Map<String, Value>, key: &str) -> Result<String, DoorError> {
    match object.get(key) {
        None => Err(DoorError::Unprocessable(format!(
            "Missing required parameter: '{key}'"
        ))),
        Some(Value::String(text)) if !text.is_empty() => Ok(text.clone()),
        Some(Value::String(_)) => Err(DoorError::Unprocessable(format!(
            "Missing required parameter: '{key}'"
        ))),
        Some(_) => Err(DoorError::Unprocessable(format!(
            "Physical Violation: '{key}' must be a string."
        ))),
    }
}

fn require_count(object: &Map<String, Value>, key: &str) -> Result<u64, String> {
    match object.get(key) {
        None => Err(format!("Missing required parameter: '{key}'")),
        Some(value) => match value.as_u64() {
            Some(count) if count >= 1 => Ok(count),
            _ => Err(format!(
                "Physical Violation: '{key}' must be an integer >= 1."
            )),
        },
    }
}

fn require_landed(object: &Map<String, Value>) -> Result<u64, DoorError> {
    match object.get("steps_landed") {
        None => Err(DoorError::Unprocessable(
            "Missing required parameter: 'steps_landed'".to_string(),
        )),
        Some(value) => value.as_u64().ok_or_else(|| {
            DoorError::Unprocessable(
                "Physical Violation: 'steps_landed' must be an integer >= 0.".to_string(),
            )
        }),
    }
}

fn require_optional_string(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, DoorError> {
    match object.get(key) {
        None => Err(DoorError::Unprocessable(format!(
            "Missing required parameter: '{key}'"
        ))),
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(DoorError::Unprocessable(format!(
            "Physical Violation: '{key}' must be a string or null."
        ))),
    }
}

fn require_reject(object: &Map<String, Value>) -> Result<Option<String>, DoorError> {
    match object.get("reject") {
        None => Err(DoorError::Unprocessable(
            "Missing required parameter: 'reject'".to_string(),
        )),
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(DoorError::Unprocessable(
            "Physical Violation: 'reject' must be a string or null.".to_string(),
        )),
    }
}

fn reject_unknown_keys(object: &Map<String, Value>, allowed: &[&str]) -> Result<(), String> {
    for key in object.keys() {
        if !allowed.iter().any(|allowed_key| *allowed_key == key) {
            return Err(crate::locked_or_unknown(key));
        }
    }
    Ok(())
}

pub(crate) fn serialize_optional_f64<S>(value: &Option<f64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(value) => crate::serialize_exact_f64(value, serializer),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn serialize_optional_state<S>(
    value: &Option<Value>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(value) => crate::serialize_state_value(value, serializer),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hash() -> String {
        "ab".repeat(32)
    }

    fn tape(final_state: Value) -> Value {
        let hash = hash();
        json!({
            "inputs": {"domain": "energy", "card": "thermal"},
            "frames": [],
            "outcome": "completed",
            "steps_landed": 1,
            "reject": null,
            "run_seal": hash,
            "final_state": final_state,
            "final_residual_conservation_error_hex": "0000000000000000",
            "max_residual_conservation_error_hex": "0000000000000000",
            "state_digest": hash
        })
    }

    #[test]
    fn a_state_hex_or_flag_that_is_not_exact_is_refused_by_name() {
        let stuffed = tape(json!({
            "temperature_k": 9999.0,
            "derated": "true",
            "temperature_k_hex": "4072526666666666\nderated:false"
        }));
        let Err(error) = check(&stuffed) else {
            panic!("a stuffed hex was accepted");
        };
        let message = error.message();
        assert!(
            message.contains("final_state.derated") || message.contains("temperature_k_hex"),
            "{message}"
        );

        let flag = tape(json!({
            "derated": "true",
            "temperature_k_hex": "4072526666666666"
        }));
        let Err(error) = check(&flag) else {
            panic!("a string flag was accepted");
        };
        let message = error.message();
        assert!(message.contains("final_state.derated"), "{message}");
        assert!(message.contains("JSON boolean"), "{message}");

        let upper = tape(json!({
            "derated": false,
            "temperature_k_hex": "407252666666666A"
        }));
        let Err(error) = check(&upper) else {
            panic!("an uppercase hex was accepted");
        };
        let message = error.message();
        assert!(message.contains("final_state.temperature_k_hex"), "{message}");

        let wrong = tape(json!({
            "derated": false,
            "temperature_k_hex": "4072526666666660"
        }));
        let Ok(report) = check(&wrong) else {
            panic!("a well-formed hex is a verdict");
        };
        assert!(!report.holds);
        assert_eq!(report.cause.as_deref(), Some(CAUSE_STATE));
    }

    #[test]
    fn the_titanium_card_publishes_no_always_false_flag() {
        for name in ["yielded", "residual_breach"] {
            assert_eq!(
                crate::state_field_is_published("materials", "ti64_lattice", name),
                Some(false),
                "{name}"
            );
            assert!(!crate::json_text::is_flag(name), "{name}");
        }
        assert_eq!(
            crate::state_field_is_published("materials", "ti64_lattice", "von_mises_mpa"),
            Some(true)
        );
    }

    #[test]
    fn an_unpublished_state_name_is_refused() {
        let extra = tape(json!({
            "temperature_k_hex": "4072526666666666",
            "not_a_field": "4072526666666666"
        }));
        let Err(error) = check(&extra) else {
            panic!("an unpublished name was accepted");
        };
        let message = error.message();
        assert!(message.contains("final_state.not_a_field"), "{message}");
        assert!(message.contains("is not published by this card"), "{message}");
    }

    #[tokio::test]
    async fn a_state_name_that_carries_other_lines_is_refused() {
        let state = crate::new_gateway_state();
        let response = run(
            &state,
            json!({
                "domain": "energy",
                "card": "thermal",
                "initial": { "temperature_k": 293.15 },
                "dt_s": 0.1,
                "segments": [{ "steps": 4, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
                "record_every": 1
            }),
        )
        .await
        .unwrap_or_else(|error| panic!("the plate runs: {}", error.message()));
        let wire = serde_json::to_string(&response).expect("response encodes");
        let posted: Value = serde_json::from_str(&wire).expect("response parses");
        let final_state = posted["final_state"].as_object().expect("state");
        assert!(final_state.contains_key("derated"));
        let text = state_digest_text(
            final_state,
            posted["final_residual_conservation_error_hex"].as_str().unwrap(),
            posted["max_residual_conservation_error_hex"].as_str().unwrap(),
        )
        .expect("state text");
        let mut lines: Vec<&str> = text.lines().collect();
        lines.pop();
        lines.pop();
        let body = format!("{}\n", lines.join("\n"));
        assert!(body.ends_with("\n") && body.len() > 18 && body.as_bytes()[body.len() - 18] == b':');
        let value = &body[body.len() - 17..body.len() - 1];
        let key = &body[..body.len() - 18];
        assert!(key.contains('\n'), "the packed name does not carry another line");
        assert_eq!(value.len(), 16);

        let mut forged = posted.clone();
        let mut packed = serde_json::Map::new();
        packed.insert(key.to_string(), json!(value));
        forged["final_state"] = serde_json::Value::Object(packed);
        assert!(!forged["final_state"].as_object().unwrap().contains_key("derated"));
        assert_eq!(forged["run_seal"], posted["run_seal"]);
        let Err(error) = check(&forged) else {
            panic!("a name carrying other lines was accepted");
        };
        let message = error.message();
        assert!(message.contains("is not a field name"), "{message}");
        assert!(message.contains("final_state."), "{message}");
    }

    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
            self.0
        }
    }

    /// Seventeen significant digits, a few ULPs above `center`.
    fn token(center: f64, lcg: &mut Lcg) -> String {
        let bits = center.to_bits();
        let sign = bits & 0x8000_0000_0000_0000;
        let magnitude = bits & !sign;
        let step = (lcg.next() % 16) + 1;
        let value = f64::from_bits(sign | magnitude.wrapping_add(step));
        format!("{value:.16e}")
    }

    /// Seventeen significant digits, a few ULPs below a positive `center`.
    fn token_below(center: f64, lcg: &mut Lcg) -> String {
        let step = (lcg.next() % 16) + 1;
        let value = f64::from_bits(center.to_bits() - step);
        format!("{value:.16e}")
    }

    async fn holds(state: &crate::AppState, name: &str, text: &str) {
        let request: Value = serde_json::from_str(text).unwrap_or_else(|error| {
            panic!("{name} does not parse: {error}\n{text}");
        });
        let sent_dt = request["dt_s"].as_f64().unwrap().to_bits();
        let response = run(state, request).await.unwrap_or_else(|error| {
            panic!("{name} refused: {}\n{text}", error.message());
        });
        let wire = serde_json::to_string(&response).expect("response encodes");
        let posted: Value = serde_json::from_str(&wire).expect("response parses");
        let report = check(&posted).unwrap_or_else(|error| {
            panic!("{name} tape refused: {}\n{wire}", error.message());
        });
        assert!(
            report.holds,
            "{name} tape {:?} broken_link {:?}",
            report.cause, report.broken_link
        );
        assert_eq!(
            posted["inputs"]["dt_s"].as_f64().unwrap().to_bits(),
            sent_dt,
            "{name} echoed a different dt than the text that was sent"
        );
    }

    #[tokio::test]
    async fn long_decimals_round_trip_through_verify_tape() {
        let state = crate::new_gateway_state();
        let mut lcg = Lcg(0x00C0_FFEE);
        let thermal_dt = 0.05;
        let resonance_dt = genesis_kernel::resonance::DynamicOscillator::mount_10hz()
            .gate_dt_s()
            .unwrap()
            * 0.25;
        let hydraulic_dt = genesis_kernel::hydraulic_fluid::FluidColumn::mineral_oil()
            .gate_dt_s()
            .unwrap()
            * 0.25;
        let grid_dt = genesis_kernel::grid_node::GridNode::copper_fuse_link()
            .gate_dt_s()
            .unwrap()
            * 0.25;
        let swing_dt = genesis_kernel::swing::SwingMachine::synchronous_60hz()
            .gate_dt_s()
            .unwrap()
            * 0.25;
        let pressure = genesis_kernel::hydraulic_fluid::REFERENCE_PRESSURE_PA;
        let grid_temp = genesis_kernel::grid_node::REFERENCE_TEMPERATURE_K;
        let grid_voltage = genesis_kernel::grid_node::CATALOG_VOLTAGE_V;

        for sample in 0..6 {
            let temperature = token(293.15, &mut lcg);
            let heat = token(1.0, &mut lcg);
            let ambient = token(293.15, &mut lcg);
            let dt = token_below(thermal_dt, &mut lcg);
            holds(
                &state,
                &format!("thermal-{sample}"),
                &format!(
                    r#"{{"domain":"energy","card":"thermal","initial":{{"temperature_k":{temperature}}},"dt_s":{dt},"segments":[{{"steps":2,"load":{{"heat_power_w":{heat},"ambient_k":{ambient}}}}}],"record_every":1}}"#
                ),
            )
            .await;

            let displacement = token(1.0e-4, &mut lcg);
            let velocity = token(1.0e-4, &mut lcg);
            let accel = token(1.0, &mut lcg);
            let dt = token_below(resonance_dt, &mut lcg);
            // A new start: no time, no booked work, the anchor at its own energy.
            let anchor = genesis_kernel::resonance::DynamicOscillator::mount_10hz()
                .mechanical_energy_j(
                    displacement.parse::<f64>().unwrap(),
                    velocity.parse::<f64>().unwrap(),
                );
            holds(
                &state,
                &format!("resonance-{sample}"),
                &format!(
                    r#"{{"domain":"signal","card":"resonance","initial":{{"displacement_m":{displacement},"velocity_m_s":{velocity},"t_s":0.0,"energy_anchor_j":{anchor:?},"boundary_work_j":0.0}},"dt_s":{dt},"segments":[{{"steps":2,"load":{{"accel_m_s2":{accel}}}}}],"record_every":1}}"#
                ),
            )
            .await;

            let soc = token_below(0.8, &mut lcg);
            let current = token(1.0, &mut lcg);
            let dt = token_below(0.05, &mut lcg);
            holds(
                &state,
                &format!("battery-{sample}"),
                &format!(
                    r#"{{"domain":"energy","card":"battery_sag","initial":{{"soc":{soc}}},"dt_s":{dt},"segments":[{{"steps":2,"load":{{"current_a":{current}}}}}],"record_every":1}}"#
                ),
            )
            .await;

            let pressure = token(pressure, &mut lcg);
            let flow = token(1.0e-8, &mut lcg);
            let dt = token_below(hydraulic_dt, &mut lcg);
            holds(
                &state,
                &format!("hydraulic-{sample}"),
                &format!(
                    r#"{{"domain":"materials","card":"hydraulic_fluid","initial":{{"pressure_pa":{pressure}}},"dt_s":{dt},"segments":[{{"steps":2,"load":{{"flow_m3_s":{flow}}}}}],"record_every":1}}"#
                ),
            )
            .await;

            let temperature = token(grid_temp, &mut lcg);
            let upstream = token(grid_voltage, &mut lcg);
            let load = token(1.0, &mut lcg);
            let ambient = token(300.0, &mut lcg);
            let dt = token_below(grid_dt, &mut lcg);
            holds(
                &state,
                &format!("grid-{sample}"),
                &format!(
                    r#"{{"domain":"energy","card":"grid_node","initial":{{"current_a":0.0,"voltage_v":0.0,"temperature_k":{temperature}}},"dt_s":{dt},"segments":[{{"steps":2,"load":{{"upstream_voltage_v":{upstream},"load_current_a":{load},"ambient_k":{ambient}}}}}],"record_every":1}}"#
                ),
            )
            .await;

            let delta = token(0.3, &mut lcg);
            let pll = token(0.05, &mut lcg);
            let power = token(0.8, &mut lcg);
            let dt = token_below(swing_dt, &mut lcg);
            holds(
                &state,
                &format!("swing-{sample}"),
                &format!(
                    r#"{{"domain":"energy","card":"swing","initial":{{"delta_rad":{delta},"delta_omega_rad_s":0.0,"pll_err_rad":{pll}}},"dt_s":{dt},"segments":[{{"steps":2,"load":{{"p_mech_pu":{power}}}}}],"record_every":1}}"#
                ),
            )
            .await;
        }

        holds(
            &state,
            "thermal-long-dt",
            r#"{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":5.73168e-5,"segments":[{"steps":2,"load":{"heat_power_w":0.0,"ambient_k":293.15}}],"record_every":1}"#,
        )
        .await;
    }

    #[tokio::test]
    async fn the_neighbor_below_233_15_is_not_sealed() {
        let text = r#"{"domain":"energy","card":"thermal","parameters":{"temperature_k":233.14999999999998,"heat_power_w":0.0,"ambient_k":293.15,"dt_s":0.1}}"#;
        let value: Value = serde_json::from_str(text).unwrap();
        let error = crate::verify_step(&crate::new_gateway_state(), value)
            .await
            .unwrap_err();
        assert!(
            error.message().contains("TemperatureOutsideCard"),
            "{}",
            error.message()
        );
    }
}

#[cfg(test)]
mod seal_tests {
    use super::SameAs;
    use std::io::Write;

    fn compared(head: &str, tail: &str, pieces: &[&str]) -> bool {
        let mut compare = SameAs { head: head.as_bytes(), tail: tail.as_bytes(), at: 0, same: true };
        for piece in pieces {
            compare.write_all(piece.as_bytes()).unwrap();
        }
        compare.same && compare.at == head.len() + tail.len()
    }

    #[test]
    fn the_comparison_accepts_the_bytes_of_head_then_tail_however_they_are_cut() {
        let (head, tail) = ("{\"a\":1", ",\"r\":\"x\"}");
        let whole = format!("{head}{tail}");
        assert!(compared(head, tail, &[&whole]));
        for cut in 0..=whole.len() {
            assert!(compared(head, tail, &[&whole[..cut], &whole[cut..]]), "cut {cut}");
        }
        let singles: Vec<String> = whole.chars().map(String::from).collect();
        let pieces: Vec<&str> = singles.iter().map(String::as_str).collect();
        assert!(compared(head, tail, &pieces));
    }

    #[test]
    fn the_comparison_refuses_a_changed_byte_a_short_write_and_a_long_write() {
        let (head, tail) = ("{\"a\":1", ",\"r\":\"x\"}");
        let whole = format!("{head}{tail}");
        for at in 0..whole.len() {
            let mut bytes = whole.clone().into_bytes();
            bytes[at] ^= 1;
            let changed = String::from_utf8(bytes).unwrap();
            for cut in [0, at, whole.len()] {
                assert!(!compared(head, tail, &[&changed[..cut], &changed[cut..]]), "{at} {cut}");
            }
        }
        assert!(!compared(head, tail, &[&whole[..whole.len() - 1]]));
        assert!(!compared(head, tail, &[&whole, "}"]));
    }
}
