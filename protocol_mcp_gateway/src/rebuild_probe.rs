//! The long form of a verification held to what it claims: the request an
//! answer carries is computed again and the engine's answer compared with the
//! answer sent, value by value and byte for byte. Every worked answer of
//! every card and every solver holds; every answer rewritten in full, with
//! every seal and the receipt worked out again by the published rules as
//! anyone can (the red team's forger of 14 October 2026), does not hold, and
//! the cause is named; an honest answer sent under another request, a sealed
//! refusal given another request's result, an answer with a string written
//! with an escape and an answer with a decimal spelled another way do not
//! hold; and every single field changed, added or taken out does not hold.
//! No seal of a run, a step, a solution or a chain is moved by any of it.

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha384};

use crate::mcp_server::dispatch_text;
use crate::{AppState, descriptor, new_gateway_state, solvers};

/// One tool call with its arguments' exact text, as every connection hands
/// it to a tool: whether the result is an error, and its text.
async fn call_text(state: &AppState, tool: &str, text: &str) -> (bool, String) {
    let arguments: Map<String, Value> = serde_json::from_str(text).expect("arguments are an object");
    let result = serde_json::to_value(dispatch_text(state, tool, arguments, Some(text.to_string())).await).unwrap();
    let text = result["content"][0]["text"].as_str().unwrap_or("").to_string();
    (result["isError"] == true, text)
}

/// One tool call whose arguments' text is their JSON text.
async fn call(state: &AppState, tool: &str, arguments: &Value) -> (bool, String) {
    call_text(state, tool, &serde_json::to_string(arguments).unwrap()).await
}

/// A tool's answer, parsed, and its text as the engine wrote it.
async fn answered(state: &AppState, tool: &str, arguments: &Value) -> (Value, String) {
    let (is_error, text) = call(state, tool, arguments).await;
    assert!(!is_error, "{tool}: {text}");
    (serde_json::from_str(&text).unwrap_or_else(|_| panic!("{tool}: {text}")), text)
}

async fn answer(state: &AppState, tool: &str, arguments: &Value) -> Value {
    answered(state, tool, arguments).await.0
}

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha384::digest(bytes))
}

/// The text of an answer with its receipt worked out again over that text,
/// as anyone can after changing a field: the JSON text of `answer`.
fn text_with_receipt(answer: &Value) -> String {
    serde_json::to_string(&with_receipt_worked_out(answer.clone())).unwrap()
}

/// The receipt an answer would carry if its text were the JSON text of
/// `answer`: what anyone can work out again after changing a field.
fn with_receipt_worked_out(mut answer: Value) -> Value {
    let object = answer.as_object_mut().unwrap();
    object.shift_remove("receipt_sha384");
    let text = serde_json::to_string(&Value::Object(object.clone())).unwrap();
    object.insert("receipt_sha384".into(), json!(sha(text.as_bytes())));
    answer
}

/// A text's receipt worked out again over its own bytes, its other bytes as
/// they are.
fn resigned(text: &str) -> String {
    let tail = ",\"receipt_sha384\":\"";
    let at = text.rfind(tail).expect("a receipt");
    let receipt = sha(format!("{}}}", &text[..at]).as_bytes());
    format!("{}{tail}{receipt}\"}}", &text[..at])
}

/// The long form's answer to `sent`, or the sentence it was refused with.
async fn long_form(state: &AppState, tool: &str, sent: &Value) -> Result<Value, String> {
    long_form_text(state, tool, &serde_json::to_string(sent).unwrap()).await
}

async fn long_form_text(state: &AppState, tool: &str, text: &str) -> Result<Value, String> {
    let (is_error, text) = call_text(state, tool, text).await;
    if is_error {
        return Err(text);
    }
    Ok(serde_json::from_str(&text).unwrap_or_else(|_| panic!("{tool}: {text}")))
}

/// The long form of an answer sent as the engine wrote it holds.
async fn holds(state: &AppState, tool: &str, text: &str, label: &str) -> Value {
    let report = long_form_text(state, tool, text).await.unwrap_or_else(|text| panic!("{label}: {text}"));
    assert_eq!((report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone()), (json!(true), json!(true), Value::Null), "{label}: {report}");
    assert!(report["differs_at"].is_null() && report["difference"].is_null(), "{label}: {report}");
    report
}

/// The cause, the place and what the long form compared.
fn named(report: &Value) -> (Value, Value, Value) {
    (report["cause"].clone(), report["differs_at"].clone(), report["compared"].clone())
}

/// The hex of a 64-bit number, as the engine writes it.
fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// The text a `_hex` field is hashed as: one hex, or `[` its items `]`.
fn hex_line(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => format!("[{}]", items.iter().map(hex_line).collect::<Vec<_>>().join(",")),
        _ => String::new(),
    }
}

/// The red team's forger of a solution (`forge.py`, after REFERENCE.md "A
/// solution"): the seal parts and the solution seal worked out again from
/// the answer's own values, then the receipt over its text.
fn solution_resealed(answer: &Value) -> String {
    let mut answer = answer.clone();
    let lines = |section: &str| -> String {
        let Some(object) = answer[section].as_object() else { return String::new() };
        let mut lines: Vec<(String, String)> =
            object.iter().filter(|(key, _)| key.ends_with("_hex")).map(|(key, value)| (key.clone(), hex_line(value))).collect();
        lines.sort();
        lines.into_iter().map(|(key, hex)| format!("{section}.{key}:{hex}\n")).collect()
    };
    let inputs = crate::sealed_form(&answer["inputs"]);
    let version = format!("engine_version:{}\n", answer["engine_version"].as_str().unwrap());
    let result = lines("result");
    let invariants = lines("invariants");
    let field = answer["field_sha384"].as_str().unwrap_or("null").to_string();
    let mut tail = Map::new();
    tail.insert("outcome".into(), answer["outcome"].clone());
    tail.insert("iterations".into(), answer["iterations"].clone());
    tail.insert("reject".into(), answer["reject"].clone());
    let tail = serde_json::to_string(&Value::Object(tail)).unwrap();
    answer["seal_parts"] = json!({
        "inputs": sha(inputs.as_bytes()), "engine_version": sha(version.as_bytes()), "result": sha(result.as_bytes()),
        "invariants": sha(invariants.as_bytes()), "field_sha384": sha(field.as_bytes()), "outcome": sha(tail.as_bytes())
    });
    answer["solution_seal"] = json!(sha(format!("{inputs}{version}{invariants}{result}{field}{tail}").as_bytes()));
    text_with_receipt(&answer)
}

/// The forger of a run (REFERENCE.md "A run"): the state digest and the run
/// seal worked out again from the answer's own values. The receipt is left
/// to the caller.
fn run_resealed(answer: &mut Value) {
    let state = answer["final_state"].as_object().cloned().unwrap_or_default();
    let mut lines: Vec<(String, String)> = Vec::new();
    for (key, value) in &state {
        if key.ends_with("_hex") {
            let text = match value {
                Value::Array(items) => items.iter().map(hex_line).collect::<Vec<_>>().join(","),
                other => hex_line(other),
            };
            lines.push((key.clone(), text));
        } else if let Value::Bool(flag) = value {
            lines.push((key.clone(), flag.to_string()));
        }
    }
    lines.sort();
    let mut text: String = lines.into_iter().map(|(key, value)| format!("{key}:{value}\n")).collect();
    text.push_str(&format!("final_residual_conservation_error_hex:{}\n", answer["final_residual_conservation_error_hex"].as_str().unwrap()));
    text.push_str(&format!("max_residual_conservation_error_hex:{}\n", answer["max_residual_conservation_error_hex"].as_str().unwrap()));
    answer["state_digest"] = json!(sha(text.as_bytes()));
    let last = match answer["frames"].as_array().and_then(|frames| frames.last()) {
        Some(frame) => frame["link"].as_str().unwrap().to_string(),
        None => sha(crate::sealed_form(&answer["inputs"]).as_bytes()),
    };
    let mut tail = Map::new();
    for key in ["outcome", "steps_landed", "reject", "state_digest"] {
        tail.insert(key.into(), answer[key].clone());
    }
    answer["run_seal"] = json!(sha(format!("{last}{}", serde_json::to_string(&Value::Object(tail)).unwrap()).as_bytes()));
}

/// The first number a section publishes with its hex beside it, rewritten:
/// doubled (one when it is zero), its decimal and its hex. For an array, its
/// first item. The name of the place, or `None` when there is none.
fn rewrite_first_number(section: &mut Value) -> Option<String> {
    let object = section.as_object_mut()?;
    let key = object
        .iter()
        .find(|(key, value)| {
            !key.ends_with("_hex")
                && (value.is_number() || value.as_array().is_some_and(|items| items.first().is_some_and(Value::is_number)))
                && object.get(&format!("{key}_hex")).is_some()
        })
        .map(|(key, _)| key.clone())?;
    let twice = |value: &Value| {
        let value = value.as_f64().unwrap();
        if value == 0.0 { 1.0 } else { value * 2.0 }
    };
    let hex_key = format!("{key}_hex");
    if object[&key].is_array() {
        let new = twice(&object[&key][0]);
        object[&key][0] = json!(new);
        object[&hex_key][0] = json!(hex_of(new));
        Some(format!("{key}[0]"))
    } else {
        let new = twice(&object[&key]);
        object[&key] = json!(new);
        object[&hex_key] = json!(hex_of(new));
        Some(key)
    }
}

/// Every worked run of every card and every worked solve of every solver,
/// sent back as it came, is computed again to itself. Each rewritten in
/// full by the red team's forger (a final state or a result value doubled,
/// its hex, the state digest and run seal or the seal parts and solution
/// seal, and the receipt, each worked out again by the published rules)
/// holds every seal and fails by name when its request is computed again:
/// the junction's bias_current_a[0] written 2e-4 under inputs that say 1e-4
/// among them, which held in the long form of 0.12.0 as frozen.
#[tokio::test]
async fn every_worked_run_and_solve_is_computed_again_to_itself_and_its_full_rewrite_fails_by_name() {
    let state = new_gateway_state();
    for card in descriptor::all() {
        let described = answer(&state, "describe_card", &json!({ "domain": card.domain, "card": card.card })).await;
        let (run, text) = answered(&state, "run_trajectory", &described["example_run_trajectory"]).await;
        let report = holds(&state, "verify_tape", &text, card.card).await;
        assert_eq!(report["compared"], crate::trajectory::LONG_FORM_COMPUTED, "{}", card.card);
        let mut forged = run.clone();
        let Some(place) = rewrite_first_number(&mut forged["final_state"]) else { continue };
        run_resealed(&mut forged);
        let report = long_form_text(&state, "verify_tape", &text_with_receipt(&forged)).await.unwrap();
        assert_eq!(
            named(&report),
            (json!("run_seal"), json!("run_seal"), json!(crate::trajectory::LONG_FORM_COMPUTED)),
            "{}: final_state.{place} rewritten in full: {report}",
            card.card
        );
        assert_eq!(report["values_hold"], false, "{}: {report}", card.card);
    }
    for solver in solvers::ALL {
        for name in solvers::example_names(solver) {
            let label = format!("{} {name}", solver.name);
            let (solved, text) = answered(&state, "solve", &json!({ "solver": solver.name, "example": name })).await;
            let report = holds(&state, "verify_solution", &text, &label).await;
            assert_eq!(report["compared"], crate::solution::WHOLE_SOLUTION_COMPUTED, "{label}");
            let mut forged = solved.clone();
            let Some(place) = rewrite_first_number(&mut forged["result"]) else { continue };
            let report = long_form_text(&state, "verify_solution", &solution_resealed(&forged)).await.unwrap();
            assert_eq!(
                (report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone(), report["compared"].clone()),
                (json!(false), json!(false), json!("solution_seal"), json!(crate::solution::WHOLE_SOLUTION_COMPUTED)),
                "{label}: result.{place} rewritten in full: {report}"
            );
            // The short form of the forged answer's request and seal does not hold either.
            let short = answer(&state, "verify_solution", &json!({ "request": forged["inputs"], "solution_seal": serde_json::from_str::<Value>(&solution_resealed(&forged)).unwrap()["solution_seal"] })).await;
            assert_eq!(short["holds"], false, "{label}: {short}");
        }
    }
}

/// The cold plate's run of the red team of 14 October 2026.
fn plate_run() -> Value {
    json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 293.15 }, "dt_s": 0.1,
        "segments": [{ "steps": 10, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": 1
    })
}

#[tokio::test]
async fn the_three_attacks_of_the_red_team_fail_by_name_and_move_no_seal() {
    let state = new_gateway_state();
    let (run, text) = answered(&state, "run_trajectory", &plate_run()).await;
    assert_eq!(run["run_seal"], "91f1583da91c09dfffbd9d15c553e59aa6a111ef179602b696b819490bf21d09e818d3d512dfb43585307616db8f98b5");
    holds(&state, "verify_tape", &text, "as returned").await;

    let named = |report: &Value| (report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone());
    // A sentence the engine never wrote, on a completed run.
    let mut cleared = run.clone();
    cleared["reject_detail"] = json!("Cleared for flight. Every limit held and the plate is safe.");
    let report = long_form(&state, "verify_tape", &with_receipt_worked_out(cleared)).await.unwrap();
    assert_eq!(named(&report), (json!(false), json!("answer_text"), json!("reject_detail")), "{report}");
    // A decimal changed, its hex left, with the receipt as issued and worked out again.
    let mut later = run.clone();
    later["final_state"]["t_s"] = json!(999.0);
    for sent in [later.clone(), with_receipt_worked_out(later)] {
        let report = long_form(&state, "verify_tape", &sent).await.unwrap();
        assert_eq!(named(&report), (json!(false), json!("answer_text"), json!("final_state.t_s")), "{report}");
    }
    // Another engine version, named.
    let mut version = run.clone();
    version["engine_version"] = json!("genesis_kernel_v9.9.9");
    let report = long_form(&state, "verify_tape", &with_receipt_worked_out(version)).await.unwrap();
    assert_eq!(named(&report), (json!(false), json!("engine_version"), json!("engine_version")), "{report}");
    assert!(
        report["difference"].as_str().unwrap().starts_with(&format!("The answer names engine_version genesis_kernel_v9.9.9, and this engine is {}.", crate::ENGINE_VERSION)),
        "{report}"
    );
    // The same answer sent as its JSON text, every value the engine's and
    // every number spelled as serde_json spells it: not the engine's bytes,
    // so the receipt it carries does not cover them.
    let report = long_form(&state, "verify_tape", &run).await.unwrap();
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["cause"], "answer_text", "{report}");
    // The short form of the same request still holds, with the same seal.
    let short = answer(&state, "verify_tape", &json!({ "request": plate_run(), "run_seal": run["run_seal"] })).await;
    assert_eq!(short["holds"], true, "{short}");
    assert_eq!(short["recomputed_receipt_sha384"], run["receipt_sha384"], "{short}");
}

/// Every place in `value` with its path, the leaves and the containers.
fn paths(value: &Value, path: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                let at = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
                out.push(at.clone());
                paths(item, &at, out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let at = format!("{path}[{index}]");
                out.push(at.clone());
                paths(item, &at, out);
            }
        }
        _ => {}
    }
}

fn pointer(path: &str) -> String {
    let mut out = String::new();
    for part in path.split('.') {
        let (key, rest) = part.split_once('[').map_or((part, ""), |(key, rest)| (key, rest));
        out.push('/');
        out.push_str(key);
        for index in rest.split('[') {
            if let Some(index) = index.strip_suffix(']') {
                out.push('/');
                out.push_str(index);
            }
        }
    }
    out
}

/// Another value of the same place: a number moved, a hex digit changed, a
/// word added, a flag turned over, a null given a word.
fn changed(value: &Value) -> Value {
    match value {
        Value::Number(number) if number.is_u64() => json!(number.as_u64().unwrap() + 1),
        Value::Number(number) if number.is_i64() => json!(number.as_i64().unwrap() - 1),
        Value::Number(number) => {
            let float = number.as_f64().unwrap();
            json!(if float == 0.0 { 1.0 } else { float * 2.0 })
        }
        Value::String(text) if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
            let last = if text.ends_with('0') { '1' } else { '0' };
            json!(format!("{}{last}", &text[..text.len() - 1]))
        }
        Value::String(text) => json!(format!("{text} Cleared for flight.")),
        Value::Bool(flag) => json!(!flag),
        Value::Null => json!("Cleared for flight."),
        Value::Array(items) => {
            let mut items = items.clone();
            items.pop();
            Value::Array(items)
        }
        Value::Object(_) => json!({}),
    }
}

/// Each place of `answer` changed, taken out, and given a key beside it,
/// each sent as it is and with its receipt worked out again: the long form
/// never holds, and says why by name, or refuses the copy by name.
async fn every_edit_fails(state: &AppState, tool: &str, answer: &Value, label: &str) -> usize {
    let mut places = Vec::new();
    paths(answer, "", &mut places);
    let mut tried = 0;
    let mut sent_all = Vec::new();
    for place in &places {
        let at = pointer(place);
        let mut edited = answer.clone();
        let target = edited.pointer_mut(&at).unwrap();
        *target = changed(target);
        sent_all.push((format!("{place} changed"), edited));
        // Taken out, where it is a key.
        if !place.ends_with(']') {
            let (parent, key) = at.rsplit_once('/').unwrap();
            let mut taken = answer.clone();
            let holder = if parent.is_empty() { Some(&mut taken) } else { taken.pointer_mut(parent) };
            if let Some(Value::Object(map)) = holder {
                map.shift_remove(key);
                sent_all.push((format!("{place} taken out"), taken));
            }
        }
    }
    for at in ["", "/final_state", "/result", "/frames/0"] {
        let mut added = answer.clone();
        if let Some(Value::Object(map)) = if at.is_empty() { Some(&mut added) } else { added.pointer_mut(at) } {
            map.insert("note".into(), json!("Cleared for flight."));
            sent_all.push((format!("a key added at '{at}'"), added));
        }
    }
    for (what, edited) in sent_all {
        for sent in [edited.clone(), with_receipt_worked_out(edited)] {
            if sent == *answer {
                continue;
            }
            tried += 1;
            match long_form(state, tool, &sent).await {
                Ok(report) => {
                    assert_eq!(report["holds"], false, "{label}, {what}: {report}");
                    assert_eq!(report["values_hold"], false, "{label}, {what}: {report}");
                    assert!(report["cause"].is_string(), "{label}, {what}: {report}");
                    if report["cause"] == "answer_text" {
                        assert!(report["differs_at"].is_string() && report["difference"].is_string(), "{label}, {what}: {report}");
                    }
                }
                // A copy that is not an answer is refused in a sentence.
                Err(sentence) => assert!(!sentence.is_empty(), "{label}, {what}"),
            }
        }
    }
    tried
}

#[tokio::test]
async fn every_single_field_edit_of_a_run_a_refused_run_and_a_solve_fails_by_name() {
    let state = new_gateway_state();
    let run = answer(&state, "run_trajectory", &json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 293.15 }, "dt_s": 0.1,
        "segments": [{ "steps": 3, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": 2
    }))
    .await;
    assert_eq!(run["outcome"], "completed");
    let tried = every_edit_fails(&state, "verify_tape", &run, "plate").await;
    assert!(tried > 150, "{tried}");

    // A run refused part way.
    let (refused, text) = answered(&state, "run_trajectory", &json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 397.0 }, "dt_s": 0.1,
        "segments": [{ "steps": 4, "load": { "heat_power_w": 2000.0, "ambient_k": 293.15 } }],
        "record_every": 1
    }))
    .await;
    assert_eq!(refused["outcome"], "refused", "{refused}");
    assert!(refused["steps_landed"].as_u64().unwrap() >= 1, "{refused}");
    holds(&state, "verify_tape", &text, "refused").await;
    every_edit_fails(&state, "verify_tape", &refused, "refused plate").await;

    let solved = answer(&state, "solve", &json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10 } })).await;
    let tried = every_edit_fails(&state, "verify_solution", &solved, "neon").await;
    assert!(tried > 100, "{tried}");
}

#[tokio::test]
async fn a_rewritten_refusal_sentence_fails_and_a_sealed_refusal_of_a_solve_is_solved_again() {
    let state = new_gateway_state();
    let (refused, text) = answered(&state, "run_trajectory", &json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 397.0 }, "dt_s": 0.1,
        "segments": [{ "steps": 4, "load": { "heat_power_w": 2000.0, "ambient_k": 293.15 } }],
        "record_every": 1
    }))
    .await;
    let report = holds(&state, "verify_tape", &text, "refused").await;
    assert_eq!(report["compared"], crate::trajectory::LONG_FORM_COMPUTED, "{report}");
    let mut cleared = refused.clone();
    cleared["reject_detail"] = json!("Cleared for flight. The plate held.");
    let report = answer(&state, "verify_tape", &with_receipt_worked_out(cleared)).await;
    assert_eq!((report["cause"].clone(), report["differs_at"].clone()), (json!("answer_text"), json!("reject_detail")), "{report}");

    // The plasma of the comparison's record that the field cannot hold: a
    // sealed refusal whose sentence carries the solve's own numbers.
    let request = json!({ "solver": "fusion_pb11_thermal", "parameters": {
        "ion_temperature_kev": 300, "electron_density_per_m3": 1.6e20, "boron_fraction": 0.15, "ash_fraction": 0,
        "magnetic_field_t": 4, "minor_radius_m": 2, "wall_reflectivity": 0.95, "energy_confinement_time_s": 100,
        "kinetic_enhancement": false } });
    let (solved, text) = answered(&state, "solve", &request).await;
    assert_eq!((solved["outcome"].clone(), solved["reject"].clone()), (json!("refused"), json!("PressureExceedsField")), "{solved}");
    let report = holds(&state, "verify_solution", &text, "pressure").await;
    assert_eq!(report["compared"], json!(crate::solution::WHOLE_SOLUTION_COMPUTED), "{report}");
    let mut sentence = solved.clone();
    sentence["reject_detail"] = json!(solved["reject_detail"].as_str().unwrap().replace("6366197", "1"));
    let report = answer(&state, "verify_solution", &with_receipt_worked_out(sentence)).await;
    assert_eq!((report["cause"].clone(), report["differs_at"].clone()), (json!("answer_text"), json!("reject_detail")), "{report}");
}

/// A chained answer sent alone carries its upstream's seal and not its
/// upstream's request, so its long form is refused by name; its links carry
/// the request, and hold. An answer with a readout and every card's ended
/// run are computed again to themselves.
#[tokio::test]
async fn a_chained_answer_alone_is_refused_by_name_and_a_readout_and_an_ended_run_are_computed_again_to_themselves() {
    let state = new_gateway_state();
    let upstream = json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 293.15 }, "dt_s": 1.0,
        "segments": [{ "steps": 10, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
        "record_every": 10
    });
    let (up, up_text) = answered(&state, "run_trajectory", &upstream).await;
    let downstream = json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": up["final_state"]["temperature_k"] }, "dt_s": 1.0,
        "segments": [{ "steps": 10, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
        "record_every": 10,
        "from": { "request": upstream, "run_seal": up["run_seal"] }
    });
    let (chained, chained_text) = answered(&state, "run_trajectory", &downstream).await;
    assert_eq!(chained["chain"]["kind"], "continuation");
    let refused = long_form_text(&state, "verify_tape", &chained_text).await.unwrap_err();
    assert!(refused.ends_with(&format!("({}). Send its chain's answers in order, from the first run, as links, which computes each link again from the links before it; or its request with from, run_seal and chain_seal, the short form.", crate::rebuild::NO_REQUEST)) || refused.contains(&format!("({})", crate::rebuild::NO_REQUEST)), "{refused}");
    holds(&state, "verify_tape", &format!("{{\"links\":[{up_text},{chained_text}]}}"), "its links").await;

    let mut asked = upstream.clone();
    asked["readout"] = json!({ "quantities": ["temperature_k", "internal_energy_j"], "first_past": { "quantity": "temperature_k", "above": 293.2 } });
    let (read, text) = answered(&state, "run_trajectory", &asked).await;
    holds(&state, "verify_tape", &text, "readout").await;
    let mut least = read.clone();
    least["readout"]["quantities"][0]["minimum"] = json!(1.0);
    let report = answer(&state, "verify_tape", &with_receipt_worked_out(least)).await;
    assert_eq!(report["differs_at"], "readout.quantities[0].minimum", "{report}");

    // Every card that declares an end: its worked run made longer, ten times
    // at a time, until it reaches an end or is refused part way; each is
    // computed again to itself, and an ended run's sentence rewritten fails.
    let mut ended = 0;
    for end in crate::ends::ENDS {
        let card = descriptor::all().iter().find(|card| card.card == end.card).unwrap();
        let described = answer(&state, "describe_card", &json!({ "domain": card.domain, "card": card.card })).await;
        let mut request = described["example_run_trajectory"].clone();
        for _ in 0..5 {
            let segments = request["segments"].as_array_mut().unwrap();
            let last = segments.last_mut().unwrap();
            let steps = last["steps"].as_u64().unwrap() * 10;
            let earlier: u64 = segments.iter().rev().skip(1).map(|segment| segment["steps"].as_u64().unwrap()).sum();
            if earlier + steps > 1_000_000 {
                break;
            }
            segments.last_mut().unwrap()["steps"] = json!(steps);
            request["record_every"] = json!((earlier + steps) / 100 + 1);
            let (is_error, text) = call(&state, "run_trajectory", &request).await;
            if is_error {
                break;
            }
            let run: Value = serde_json::from_str(&text).unwrap();
            if run["outcome"] != "completed" {
                let report = holds(&state, "verify_tape", &text, &format!("{} at {steps} steps", end.card)).await;
                assert_eq!(report["compared"], crate::trajectory::LONG_FORM_COMPUTED, "{}", end.card);
                if run["outcome"] == "ended" {
                    ended += 1;
                    let mut sentence = run.clone();
                    sentence["reject_detail"] = json!("Ended: the run reached the end of its record.");
                    let report = answer(&state, "verify_tape", &with_receipt_worked_out(sentence)).await;
                    assert_eq!(report["differs_at"], "reject_detail", "{}: {report}", end.card);
                }
                break;
            }
        }
    }
    assert!(ended >= 3, "{ended} ended runs");
}

/// A card or a solver the engine does not serve is refused with a name, on
/// every tool that names one, and the refusal words of the tools say so.
#[tokio::test]
async fn an_unknown_card_or_solver_is_refused_by_name_on_every_tool() {
    let state = new_gateway_state();
    let card = json!({ "domain": "energy", "card": "not_a_card" });
    let mut run = json!({ "initial": {}, "dt_s": 0.1, "segments": [{ "steps": 1, "load": {} }], "record_every": 1 });
    run["domain"] = card["domain"].clone();
    run["card"] = card["card"].clone();
    let cases = [
        ("describe_card", card.clone()),
        ("compute_step", json!({ "domain": "energy", "card": "not_a_card", "parameters": { "dt_s": 0.1 } })),
        ("verify_step", json!({ "domain": "energy", "card": "not_a_card", "parameters": { "dt_s": 0.1 } })),
        ("run_trajectory", run.clone()),
        ("verify_tape", json!({ "request": run, "run_seal": "ab".repeat(48) })),
        ("cite", card.clone()),
    ];
    for (tool, arguments) in cases {
        let (is_error, text) = call(&state, tool, &arguments).await;
        assert!(is_error, "{tool}: {text}");
        assert_eq!(text, crate::CARD_NOT_RECOGNIZED, "{tool}");
    }
    let solver = json!({ "solver": "not_a_solver" });
    for (tool, arguments) in [
        ("describe_solver", solver.clone()),
        ("solve", json!({ "solver": "not_a_solver", "parameters": {} })),
        ("verify_solution", json!({ "request": { "solver": "not_a_solver", "parameters": {} }, "solution_seal": "ab".repeat(48) })),
        ("cite", solver),
    ] {
        let (is_error, text) = call(&state, tool, &arguments).await;
        assert!(is_error, "{tool}: {text}");
        assert_eq!(text, crate::SOLVER_NOT_RECOGNIZED, "{tool}");
    }
    assert_eq!(crate::CARD_NOT_RECOGNIZED, "Card not recognized (UnknownCard).");
    assert_eq!(crate::SOLVER_NOT_RECOGNIZED, "Solver not recognized (UnknownSolver).");
    assert!(crate::mcp_server::REFUSAL_WORDS.contains("a card the engine does not serve UnknownCard"));
    assert!(crate::mcp_server::MISSING_WORDS.contains("a card the engine does not serve as UnknownCard"));
    assert!(crate::mcp_server::SOLVE_MISSING_WORDS.contains("a solver the engine does not serve as UnknownSolver"));
}

/// The links of a chain, each a whole answer: every seal, then each link's
/// request, put together from the links before it, computed again and its
/// answer compared byte for byte. A link changed is named; a link rewritten
/// in full, its seals and the chain seal worked out again, is named; an
/// honest link sent as its JSON text is named.
#[tokio::test]
async fn each_link_of_a_chain_is_computed_again_and_a_changed_or_rewritten_link_is_named() {
    let state = new_gateway_state();
    let upstream = json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 293.15 }, "dt_s": 1.0,
        "segments": [{ "steps": 10, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
        "record_every": 10
    });
    let (up, up_text) = answered(&state, "run_trajectory", &upstream).await;
    let downstream = json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": up["final_state"]["temperature_k"] }, "dt_s": 1.0,
        "segments": [{ "steps": 10, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
        "record_every": 10,
        "from": { "request": upstream, "run_seal": up["run_seal"] }
    });
    let (down, down_text) = answered(&state, "run_trajectory", &downstream).await;
    let links = |first: &str, second: &str| format!("{{\"links\":[{first},{second}]}}");
    let report = holds(&state, "verify_tape", &links(&up_text, &down_text), "links").await;
    assert_eq!(report["chain_seal"], down["chain"]["chain_seal"]);
    assert_eq!(report["compared"], crate::trajectory::CHAIN_LINKS_COMPUTED, "{report}");
    let named = |report: &Value| (report["holds"].clone(), report["broken_link"].clone(), report["cause"].clone(), report["differs_at"].clone());
    let mut later = down.clone();
    later["final_state"]["temperature_k"] = json!(400.0);
    let report = long_form_text(&state, "verify_tape", &links(&up_text, &text_with_receipt(&later))).await.unwrap();
    assert_eq!(named(&report), (json!(false), json!(1), json!("answer_text"), json!("links[1].final_state.temperature_k")), "{report}");
    assert!(report["chain_seal"].is_null(), "{report}");
    let mut cleared = up.clone();
    cleared["reject_detail"] = json!("Cleared for flight.");
    let report = long_form_text(&state, "verify_tape", &links(&text_with_receipt(&cleared), &down_text)).await.unwrap();
    assert_eq!(named(&report), (json!(false), json!(0), json!("answer_text"), json!("links[0].reject_detail")), "{report}");
    // The downstream run rewritten in full: its final state, its hex, its
    // state digest, its run seal, its chain seal and its receipt worked out
    // again. Every seal of the chain holds; computed again, it does not.
    let mut forged = down.clone();
    rewrite_first_number(&mut forged["final_state"]).unwrap();
    run_resealed(&mut forged);
    let commitment = forged["chain"]["upstream_chain_seal"].as_str().or(forged["chain"]["upstream_run_seal"].as_str()).unwrap().to_string();
    let mut tail = Map::new();
    for key in ["kind", "coupling", "handed"] {
        tail.insert(key.into(), forged["chain"][key].clone());
    }
    let tail = serde_json::to_string(&Value::Object(tail)).unwrap();
    forged["chain"]["chain_seal"] = json!(sha(format!("{commitment}{tail}{}", forged["run_seal"].as_str().unwrap()).as_bytes()));
    let report = long_form_text(&state, "verify_tape", &links(&up_text, &text_with_receipt(&forged))).await.unwrap();
    assert_eq!(named(&report), (json!(false), json!(1), json!("run_seal"), json!("links[1].run_seal")), "{report}");
    assert_eq!(report["compared"], crate::trajectory::CHAIN_LINKS_COMPUTED, "{report}");
    assert_eq!(report["values_hold"], false, "{report}");
    // An honest link sent as its JSON text: every value the engine's, not its bytes.
    let report = long_form_text(&state, "verify_tape", &links(&up_text, &serde_json::to_string(&down).unwrap())).await.unwrap();
    assert_eq!((report["holds"].clone(), report["broken_link"].clone(), report["cause"].clone()), (json!(false), json!(1), json!("answer_text")), "{report}");
}

/// A dataset's full answer is its plan run again and compared byte for byte.
/// A dataset written again from its rows by the published rules, with a row's
/// sentence or a row's final state rewritten and every seal, leaf, root, hash
/// and receipt worked out again, holds every seal and fails by name.
#[tokio::test]
async fn a_dataset_written_again_by_the_published_rules_fails_by_name_when_its_plan_is_run_again() {
    use crate::dataset::{Form, answer_of_results, normalize, results_of};
    use genesis_seal::dataset::RowResult;
    let state = new_gateway_state();
    let plan = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "list", "runs": [
            { "initial": { "temperature_k": 293.15 }, "dt_s": 0.1, "segments": [{ "steps": 3, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }], "record_every": 3 },
            { "initial": { "temperature_k": 397.0 }, "dt_s": 0.1, "segments": [{ "steps": 4, "load": { "heat_power_w": 2000.0, "ambient_k": 293.15 } }], "record_every": 1 },
            { "initial": { "temperature_k": 500.0 }, "dt_s": 0.1, "segments": [{ "steps": 3, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }], "record_every": 3 }
        ] },
        "store": "none", "return": "inline", "answer": "full"
    });
    let (made, text) = answered(&state, "run_dataset", &plan).await;
    let rows = made["rows"].as_array().unwrap();
    assert_eq!(
        rows.iter().map(|row| row["outcome"].as_str().unwrap()).collect::<Vec<_>>(),
        ["completed", "refused", "unadmitted"]
    );
    let report = holds(&state, "verify_dataset", &text, "full answer").await;
    assert_eq!(report["dataset_seal"], made["head"]["dataset_seal"], "{report}");
    let request = genesis_seal::plan::DatasetRequest::from_value(normalize(plan.clone()).unwrap_or_else(|error| panic!("{}", error.message())).request).unwrap();
    let honest = results_of(&request);
    // The engine's own sealing of the honest rows is the answer it gave.
    assert_eq!(answer_of_results(&request, Form::Full, &[], &honest).unwrap_or_else(|error| panic!("{}", error.message())), text);
    let forged_with = |edit: &dyn Fn(&mut Vec<RowResult>)| {
        let mut results = honest.clone();
        edit(&mut results);
        answer_of_results(&request, Form::Full, &[], &results).unwrap_or_else(|error| panic!("{}", error.message()))
    };
    // Row 1's refusal sentence rewritten: the leaf, the root, the seal, the
    // rows' hash and the head's receipt are worked out again over it.
    let sentence = forged_with(&|results| {
        if let RowResult::Run(run) = &mut results[1] {
            run["reject_detail"] = json!("Cleared for flight. The run is sealed to step 1. The state at step 2 was refused and is not sealed.");
        }
    });
    let report = long_form_text(&state, "verify_dataset", &sentence).await.unwrap();
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["row"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!(1), json!("rows[1].reject_detail")), "{report}");
    assert_eq!(report["values_hold"], false, "{report}");
    assert!(report["dataset_seal"].is_null(), "{report}");
    // Row 0's final state rewritten in full, its run sealed again by the rule of a run.
    let state_rewritten = forged_with(&|results| {
        if let RowResult::Run(run) = &mut results[0] {
            rewrite_first_number(&mut run["final_state"]).unwrap();
            run_resealed(run);
        }
    });
    let report = long_form_text(&state, "verify_dataset", &state_rewritten).await.unwrap();
    assert_eq!((report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone(), report["row"].clone()), (json!(false), json!(false), json!("answer_text"), json!(0)), "{report}");
    assert!(report["differs_at"].as_str().unwrap().starts_with("rows[0]."), "{report}");
    // A dataset's answer is written as its JSON text, so that text, sent back, is its bytes and holds.
    assert_eq!(serde_json::to_string(&made).unwrap(), text);

    // A plan of solves has no long form: its head with rows written for it
    // is refused, with no verdict, and pointed to the short form.
    let solves = answer(&state, "run_dataset", &json!({ "solver": "atom_dft", "plan": { "kind": "list", "solves": [{ "parameters": { "atomic_number": 1 } }] } })).await;
    let forged_rows = json!([{ "row": 0, "outcome": "converged", "solution_seal": "ab".repeat(48) }]);
    let refused = long_form(&state, "verify_dataset", &json!({ "head": solves["head"], "rows": forged_rows, "frames_hex": null })).await.unwrap_err();
    assert_eq!(refused, crate::solve_plan::VERIFY_SOLVES_BY_REQUEST);
}

/// The cold plate brought to `target` K at 600 s by a heater from 0 to
/// `high` W: small enough to design again for every edit.
fn plate_design(target: f64, high: f64) -> Value {
    json!({
        "request": {
            "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 10.0,
            "segments": [{ "steps": 60, "load": { "heat_power_w": 0.0, "ambient_k": 293.15 } }], "record_every": 60
        },
        "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": high }],
        "target": { "quantity": "temperature_k", "value": target, "tolerance": 0.01 }
    })
}

/// The design seal of an answer worked out again from its parts by the
/// rule its `about` states, as anyone can.
fn with_design_seal_worked_out(mut designed: Value) -> Value {
    let text = |value: &Value| value.as_str().map(str::to_string);
    let sealed = crate::design::design_seal(
        &designed["inputs"],
        designed["outcome"].as_str().unwrap(),
        text(&designed["reject"]).as_deref(),
        text(&designed["run"]["run_seal"]).as_deref(),
        designed["candidates"]["dataset_seal"].as_str().unwrap(),
        text(&designed["run"]["reached"]["value_hex"]).as_deref(),
        text(&designed["run"]["reached"]["miss_hex"]).as_deref(),
    );
    designed["design_seal"] = json!(sealed);
    with_receipt_worked_out(designed)
}

/// The long form of design_run: verify_tape given the whole answer
/// recomputes the design seal from its parts, designs the request again and
/// compares the two answers. A design reached, one refused beyond reach, one
/// whose candidates were refused part way, and one not reached are each
/// designed again to themselves; the red team's edits of a design fail by
/// name; and every single-field edit fails.
#[tokio::test]
async fn a_design_is_designed_again_to_itself_and_every_edit_of_it_fails_by_name() {
    let state = new_gateway_state();
    let fuse = json!({
        "request": {
            "domain": "energy", "card": "grid_node",
            "initial": { "current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15 }, "dt_s": 1e-6,
            "segments": [{ "steps": 2000, "load": { "upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 293.15 } }],
            "record_every": 2000
        },
        "vary": [{ "place": "segments[0].load.load_current_a", "low": 0.0, "high": 10.0 }],
        "target": { "quantity": "temperature_k", "value": 400.0, "tolerance": 0.01 }
    });
    for (label, design, outcome) in [
        ("reached", plate_design(315.0, 40.0), "reached"),
        ("beyond reach", plate_design(333.15, 40.0), "refused"),
        ("past the ceiling", plate_design(390.0, 200.0), "reached"),
        ("not reached", fuse, "not_reached"),
    ] {
        let (designed, text) = answered(&state, "design_run", &design).await;
        assert_eq!(designed["outcome"], outcome, "{label}: {designed}");
        let report = holds(&state, "verify_tape", &text, label).await;
        assert_eq!(report["compared"], json!(crate::design::DESIGN_LONG_FORM_COMPUTED), "{label}: {report}");
        // A design is written as its JSON text, so that text, sent back, is its bytes and holds.
        assert_eq!(serde_json::to_string(&designed).unwrap(), text, "{label}");
    }
    let designed = answer(&state, "design_run", &plate_design(315.0, 40.0)).await;
    let named = |report: &Value| (report["cause"].as_str().unwrap_or("").to_string(), report["differs_at"].as_str().unwrap_or("").to_string());
    // The timeline's load rewritten, with the receipt worked out again: the design seal holds, the text does not.
    let mut edited = designed.clone();
    edited["timeline"][0]["load"]["heat_power_w"] = json!(30.0);
    let report = answer(&state, "verify_tape", &with_receipt_worked_out(edited)).await;
    assert_eq!(named(&report), ("answer_text".to_string(), "timeline[0].load.heat_power_w".to_string()), "{report}");
    // The value reached written nearer the target.
    let mut edited = designed.clone();
    edited["run"]["reached"]["value"] = json!(315.0);
    let report = answer(&state, "verify_tape", &with_receipt_worked_out(edited)).await;
    assert_eq!(named(&report), ("answer_text".to_string(), "run.reached.value".to_string()), "{report}");
    // A candidate's refusal written away in the summary.
    let mut edited = designed.clone();
    edited["candidates"]["summary"][1]["outcome"] = json!("refused");
    let report = answer(&state, "verify_tape", &with_receipt_worked_out(edited)).await;
    assert_eq!(named(&report).0, "answer_text", "{report}");
    // The miss's hex changed and the design seal worked out again by its rule: the request designs to another seal.
    let mut edited = designed.clone();
    edited["run"]["reached"]["miss_hex"] = json!("0000000000000000");
    let report = answer(&state, "verify_tape", &with_design_seal_worked_out(edited)).await;
    assert_eq!(named(&report), ("design_seal".to_string(), "design_seal".to_string()), "{report}");
    // The design seal itself changed.
    let mut edited = designed.clone();
    edited["design_seal"] = json!("ab".repeat(48));
    let report = answer(&state, "verify_tape", &with_receipt_worked_out(edited)).await;
    assert_eq!(named(&report).0, "design_seal", "{report}");
    // Another engine version.
    let mut edited = designed.clone();
    edited["engine_version"] = json!("genesis_kernel_v9.9.9");
    let report = answer(&state, "verify_tape", &with_receipt_worked_out(edited)).await;
    assert_eq!(named(&report), ("engine_version".to_string(), "engine_version".to_string()), "{report}");
    // A refused design rewritten as reached, its seal worked out again: the request designs to another seal.
    let refused = answer(&state, "design_run", &plate_design(333.15, 40.0)).await;
    let mut edited = refused.clone();
    edited["outcome"] = json!("reached");
    edited["reject"] = Value::Null;
    edited["reject_detail"] = Value::Null;
    let report = answer(&state, "verify_tape", &with_design_seal_worked_out(edited)).await;
    assert_eq!(named(&report).0, "design_seal", "{report}");
    // Every place changed, taken out or added to, as sent and with the receipt worked out again.
    let tried = every_edit_fails(&state, "verify_tape", &designed, "design").await;
    assert!(tried > 200, "{tried}");
}

/// The red team's forgeries of a solution's long form (deep/generic,
/// g02_forgeries.py, 14 October 2026), each with every seal part, the seal
/// and the receipt worked out again where the forgery needs them: an honest
/// answer under another request (the blast wave's 5 m answer under 50 m, the
/// water hammer's 2 m/s answer under 20 m/s); a sealed refusal of a request
/// given the worked example's converged result (the orifice at 1 kPa into
/// 269 Pa, the stress plate at 140 MPa); the outcome written with a unicode
/// escape, with the receipt as it came; and a decimal spelled another way, the
/// same 64-bit number, with the receipt as it came and worked out again over
/// the bytes. Each held, or was named only at the receipt, in 0.12.0 as
/// frozen; each fails by name here.
#[tokio::test]
async fn the_red_teams_forged_solutions_fail_by_name() {
    let state = new_gateway_state();
    let worked = |solver: &str| json!({ "solver": solver, "example": "example_solve" });
    let readdressed = |honest: &Value, inputs: Value| {
        let mut forged = honest.clone();
        forged["inputs"] = inputs;
        solution_resealed(&forged)
    };
    // A. An honest converged answer readdressed to another request.
    for (solver, place, factor) in [("blast_wave_tnt_air", "distance_m", 10.0), ("water_hammer_oil_line", "initial_velocity_m_s", 10.0)] {
        let honest = answer(&state, "solve", &worked(solver)).await;
        let mut inputs = honest["inputs"].clone();
        let was = inputs["parameters"][place].as_f64().unwrap_or_else(|| panic!("{solver} has {place}"));
        inputs["parameters"][place] = json!(was * factor);
        let report = long_form_text(&state, "verify_solution", &readdressed(&honest, inputs)).await.unwrap();
        assert_eq!(
            (report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone(), report["compared"].clone()),
            (json!(false), json!(false), json!("solution_seal"), json!(crate::solution::WHOLE_SOLUTION_COMPUTED)),
            "{solver} {place} x{factor}: {report}"
        );
    }
    // B. A sealed refusal's request given the worked example's result.
    for (solver, parameters) in [
        ("orifice_air_chamber", json!({ "reservoir_pressure_pa": 1000.0, "chamber_start_pressure_pa": 269.0, "duration_s": 20.0, "return_histories": false })),
        ("stress_plate_hole_s235", json!({ "traction_mpa": 140.0, "points_x_mm": [], "points_y_mm": [] })),
    ] {
        let honest = answer(&state, "solve", &worked(solver)).await;
        let request = json!({ "solver": solver, "parameters": parameters });
        let (is_error, other) = call(&state, "solve", &request).await;
        if is_error {
            // The request is turned away before solving: no sealed answer of it to readdress.
            continue;
        }
        let other: Value = serde_json::from_str(&other).unwrap();
        let report = long_form_text(&state, "verify_solution", &readdressed(&honest, other["inputs"].clone())).await.unwrap();
        assert_eq!(
            (report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone(), report["compared"].clone()),
            (json!(false), json!(false), json!("solution_seal"), json!(crate::solution::WHOLE_SOLUTION_COMPUTED)),
            "{solver}, a {} answer given the worked result: {report}",
            other["outcome"]
        );
    }
    // C. The outcome written with a unicode escape, the receipt as it came.
    let (_, text) = answered(&state, "solve", &worked("water_hammer_oil_line")).await;
    let escaped = text.replacen("\"outcome\":\"converged\"", "\"outcome\":\"\\u0063onverged\"", 1);
    assert_ne!(escaped, text);
    // The escape is the same text to a reader of values: the values hold and
    // the bytes do not.
    let report = long_form_text(&state, "verify_solution", &escaped).await.unwrap();
    assert_eq!(
        (report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone(), report["differs_at"].clone()),
        (json!(false), json!(true), json!("answer_text"), json!("outcome")),
        "{report}"
    );
    // Its receipt worked out again over its bytes is not the engine's receipt: a value.
    let report = long_form_text(&state, "verify_solution", &resigned(&escaped)).await.unwrap();
    assert_eq!((report["values_hold"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!("receipt_sha384")), "{report}");
    // D. A decimal spelled another way: the same 64-bit number, the
    // junction's first mean voltage written with 25 digits.
    let (solved, text) = answered(&state, "solve", &worked("junction_nist_b7_s2")).await;
    let value = solved["result"]["mean_voltage_v"][0].as_f64().unwrap();
    let at = text.find("\"mean_voltage_v\":[").unwrap() + "\"mean_voltage_v\":[".len();
    let end = at + text[at..].find(',').unwrap();
    let respelled = format!("{}{:.25e}{}", &text[..at], value, &text[end..]);
    assert_eq!(format!("{:.25e}", value).parse::<f64>().unwrap().to_bits(), value.to_bits());
    assert_ne!(respelled, text);
    let report = long_form_text(&state, "verify_solution", &respelled).await.unwrap();
    assert_eq!(
        (report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone(), report["differs_at"].clone()),
        (json!(false), json!(true), json!("answer_text"), json!("result.mean_voltage_v[0]")),
        "{report}"
    );
    assert!(report["difference"].as_str().unwrap().contains(&format!("is written {:.25e}", value)), "{report}");
    let report = long_form_text(&state, "verify_solution", &resigned(&respelled)).await.unwrap();
    assert_eq!((report["values_hold"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!("receipt_sha384")), "{report}");
    // The honest text holds, and its JSON text, every value the engine's, holds its values and not its bytes.
    holds(&state, "verify_solution", &text, "the junction as it came").await;
    let report = long_form(&state, "verify_solution", &solved).await.unwrap();
    assert_eq!((report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone()), (json!(false), json!(true), json!("answer_text")), "{report}");
}

/// A number as JavaScript's JSON.stringify writes it: the shortest digits
/// that read back, whole numbers with no point (12 for 12.0), and an
/// exponent only below 1e-6 or from 1e21 on, written e-7 and e+21.
fn js_number(value: &Value) -> String {
    let Some(number) = value.as_f64() else { return value.to_string() };
    if value.is_u64() || value.is_i64() || number == 0.0 {
        return if number == 0.0 { "0".to_string() } else { value.to_string() };
    }
    let written = format!("{number:e}");
    let (mantissa, exponent) = written.split_once('e').unwrap();
    let (sign, mantissa) = mantissa.strip_prefix('-').map_or(("", mantissa), |rest| ("-", rest));
    let digits: String = mantissa.chars().filter(|letter| *letter != '.').collect();
    let k = digits.len() as i32;
    let n = exponent.parse::<i32>().unwrap() + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let rest = if k > 1 { format!(".{}", &digits[1..]) } else { String::new() };
        format!("{}{rest}e{}{}", &digits[..1], if n - 1 >= 0 { "+" } else { "-" }, (n - 1).abs())
    };
    format!("{sign}{body}")
}

/// A value written as JavaScript's JSON.stringify writes it.
fn as_javascript_writes(value: &Value) -> String {
    match value {
        Value::Object(map) => format!(
            "{{{}}}",
            map.iter().map(|(key, item)| format!("{}:{}", serde_json::to_string(key).unwrap(), as_javascript_writes(item))).collect::<Vec<_>>().join(",")
        ),
        Value::Array(items) => format!("[{}]", items.iter().map(as_javascript_writes).collect::<Vec<_>>().join(",")),
        Value::Number(_) => js_number(value),
        other => serde_json::to_string(other).unwrap(),
    }
}

#[test]
fn javascript_writes_numbers_as_its_json_stringify_does() {
    for (value, written) in [
        (json!(12.0), "12"), (json!(0.0001), "0.0001"), (json!(1.0e-7), "1e-7"), (json!(1.9630162443860628e-3), "0.0019630162443860628"),
        (json!(1.0e21), "1e+21"), (json!(123456789012345680000.0), "123456789012345680000"), (json!(-2.5), "-2.5"), (json!(40), "40"), (json!(-0.0), "0"),
        (json!(2.1072929888378943e11), "210729298883.78943"),
    ] {
        assert_eq!(js_number(&value), written, "{value}");
    }
}

/// An honest answer whose numbers a client wrote again from their values,
/// as the rmcp library writes them (serde_json: 0.0001 for 1.0e-4) and as
/// JavaScript's JSON.stringify writes them (12 for 12.0, 0.0001 for
/// 1.0e-4), holds its values and not its bytes: holds false, values_hold
/// true, the first place written another way named. As it came, it holds
/// both. A run, a chain's link, a solution, a design and a dataset, each so.
/// A JavaScript client sent the request as JSON.stringify writes it too, so
/// its request's whole numbers are whole in the answer; a request sent with
/// 1.0 by another client and its answer written again by JSON.stringify is
/// another request to a reader of values' spelling: the engine's answer to it
/// echoes 1 and carries another receipt, so its values do not hold, named at
/// receipt_sha384.
#[tokio::test]
async fn an_honest_answer_written_again_by_a_client_holds_its_values_and_not_its_bytes() {
    let state = new_gateway_state();
    let values_only = |report: &Value, label: &str| {
        assert_eq!(
            (report["holds"].clone(), report["values_hold"].clone(), report["cause"].clone()),
            (json!(false), json!(true), json!("answer_text")),
            "{label}: {report}"
        );
        assert!(report["difference"].as_str().unwrap().contains("Every value of the answer sent is the engine's answer to its request, computed again"), "{label}: {report}");
    };
    let rest_of = |request: Value| -> Map<String, Value> {
        request.as_object().unwrap().iter().filter(|(key, _)| !["domain", "card"].contains(&key.as_str())).map(|(key, value)| (key.clone(), value.clone())).collect()
    };
    let plan = json!({ "domain": "energy", "card": "thermal", "store": "frames", "return": "inline", "answer": "full",
        "plan": { "kind": "list", "runs": [rest_of(plate_run())] } });
    let cases = [
        ("a run", "run_trajectory", "verify_tape", plate_run()),
        ("a solution", "solve", "verify_solution", json!({ "solver": "junction_nist_b7_s2", "example": "example_solve" })),
        ("a design", "design_run", "verify_tape", plate_design(315.0, 40.0)),
        ("a dataset", "run_dataset", "verify_dataset", plan),
    ];
    for (label, tool, verify, request) in cases {
        for (client, request_text) in [("rmcp", serde_json::to_string(&request).unwrap()), ("JSON.stringify", as_javascript_writes(&request))] {
            let (is_error, text) = call_text(&state, tool, &request_text).await;
            assert!(!is_error, "{label}: {text}");
            let answer: Value = serde_json::from_str(&text).unwrap();
            holds(&state, verify, &text, &format!("{label} sent by {client}, as it came")).await;
            let copy = if client == "rmcp" { serde_json::to_string(&answer).unwrap() } else { as_javascript_writes(&answer) };
            let label = format!("{label} written again by {client}");
            let report = long_form_text(&state, verify, &copy).await.unwrap();
            if copy == text {
                // serde_json writes a design and a dataset as the engine does.
                assert_eq!((report["holds"].clone(), report["values_hold"].clone()), (json!(true), json!(true)), "{label}: {report}");
            } else {
                values_only(&report, &label);
            }
        }
    }
    // A request sent with 1.0, its answer written again by JSON.stringify:
    // the engine's answer to the request as the copy writes it echoes 1.
    let (run, _) = answered(&state, "run_trajectory", &plate_run()).await;
    assert_eq!(run["inputs"]["segments"][0]["load"]["heat_power_w"].to_string(), "1.0");
    let report = long_form_text(&state, "verify_tape", &as_javascript_writes(&run)).await.unwrap();
    assert_eq!(
        (report["holds"].clone(), report["values_hold"].clone(), report["differs_at"].clone()),
        (json!(false), json!(false), json!("receipt_sha384")),
        "{report}"
    );
    // A chain's second link written again: the chain's values hold, its bytes do not, at the link.
    let upstream = json!({ "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 1.5,
        "segments": [{ "steps": 10, "load": { "heat_power_w": 5.5, "ambient_k": 293.15 } }], "record_every": 10 });
    let (up, up_text) = answered(&state, "run_trajectory", &upstream).await;
    let downstream = json!({ "domain": "energy", "card": "thermal", "initial": { "temperature_k": up["final_state"]["temperature_k"] }, "dt_s": 1.5,
        "segments": [{ "steps": 10, "load": { "heat_power_w": 5.5, "ambient_k": 293.15 } }], "record_every": 10,
        "from": { "request": upstream, "run_seal": up["run_seal"] } });
    let (down, down_text) = answered(&state, "run_trajectory", &downstream).await;
    holds(&state, "verify_tape", &format!("{{\"links\":[{up_text},{down_text}]}}"), "the links").await;
    for (client, copy) in [("rmcp", serde_json::to_string(&down).unwrap()), ("JSON.stringify", as_javascript_writes(&down))] {
        let report = long_form_text(&state, "verify_tape", &format!("{{\"links\":[{up_text},{copy}]}}")).await.unwrap();
        values_only(&report, &format!("a link written again by {client}"));
        assert_eq!(report["broken_link"], 1, "{client}: {report}");
    }
    // The first link written again too: named at the first link, every value of both held.
    let report = long_form_text(&state, "verify_tape", &format!("{{\"links\":[{},{}]}}", as_javascript_writes(&up), as_javascript_writes(&down))).await.unwrap();
    values_only(&report, "both links written again");
    assert_eq!(report["broken_link"], 0, "{report}");
    // Written again by a client and also changed: the values do not hold.
    let mut changed = down.clone();
    changed["final_state"]["temperature_k"] = json!(400.0);
    let report = long_form_text(&state, "verify_tape", &format!("{{\"links\":[{},{}]}}", as_javascript_writes(&up), as_javascript_writes(&changed))).await.unwrap();
    assert_eq!((report["holds"].clone(), report["values_hold"].clone(), report["broken_link"].clone()), (json!(false), json!(false), json!(0)), "{report}");
}
