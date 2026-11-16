//! What the tools say of themselves, held to what the tools do.
//!
//! An agent working from the tool list alone needs five things said: how a
//! plan is written, how a dataset returned in the answer is checked, how
//! large an answer a request makes, what a check means when it holds and the
//! rows do not match, and how a refusal comes back. Each is said in a
//! description, at the input it is about, or in the answer: a description
//! says what a tool does inside the length a client shows, and the input
//! schema says the rest. This probe sends every worked plan as it is
//! written, measures every size that is stated, and makes every refusal that
//! is described.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Map, Value, json};

use crate::mcp_server::{REFUSAL_WORDS, dispatch, tools_in};
use crate::view::View;
use crate::{AppState, dataset, descriptor, new_gateway_state};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Folder(PathBuf);

impl Folder {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "gateway_description_probe_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Folder(std::fs::canonicalize(path).unwrap())
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One tool call, as both doors make it: whether the result is an error, and its text.
async fn call(state: &AppState, tool: &str, arguments: &Value) -> (bool, String) {
    let arguments: Map<String, Value> = arguments.as_object().expect("arguments are an object").clone();
    let result = serde_json::to_value(dispatch(state, tool, arguments).await).unwrap();
    let text = result["content"][0]["text"].as_str().unwrap_or("").to_string();
    (result["isError"] == true, text)
}

async fn answer(state: &AppState, tool: &str, arguments: &Value) -> Value {
    let (is_error, text) = call(state, tool, arguments).await;
    assert!(!is_error, "{tool}: {text}");
    serde_json::from_str(&text).unwrap_or_else(|_| panic!("{tool}: {text}"))
}

fn description_of(view: &View, tool: &str) -> String {
    tools_in(view)
        .into_iter()
        .find(|item| item.name == tool)
        .expect("tool")
        .description
        .expect("a description")
        .to_string()
}

/// What the tool list says at one input of a tool, or of its request as a
/// whole when no input is named.
fn said_at(view: &View, tool: &str, input: Option<&str>) -> String {
    let tools = tools_in(view);
    let schema = &tools.iter().find(|item| item.name == tool).expect("tool").input_schema;
    let words = match input {
        Some(name) => schema["properties"][name]["description"].as_str(),
        None => schema["description"].as_str(),
    };
    words.unwrap_or_else(|| panic!("{tool} says nothing at {input:?}")).to_string()
}

/// The JSON object that follows ` {kind}: ` in a description, exactly as it is
/// written there: from its opening brace to the brace that closes it.
fn written_plan(description: &str, kind: &str) -> String {
    let marker = format!(" {kind}: {{\"domain\"");
    let start = description.find(&marker).unwrap_or_else(|| panic!("no {kind} plan in: {description}")) + kind.len() + 3;
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, byte) in description.as_bytes()[start..].iter().enumerate() {
        if in_string {
            match (escaped, byte) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return description[start..=start + offset].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("the {kind} plan does not close");
}

fn one_card(domain: &str, card: &str) -> View {
    View::limited([(domain.to_string(), card.to_string())], [])
}

/// `true` when `text` holds the name `card` as a whole name: not as the
/// first part of a longer name, as `reactor_spert3` is of
/// `reactor_spert3_fuel`, and not inside another name, as `thermal` is in
/// `thermal_energy_j`, a field of the tokamak.
fn names_card(text: &str, card: &str) -> bool {
    let part_of_a_longer_name = |letter: char| letter.is_ascii_alphanumeric() || letter == '_';
    text.match_indices(card).any(|(at, name)| {
        !text[..at].chars().next_back().is_some_and(part_of_a_longer_name)
            && !text[at + name.len()..].chars().next().is_some_and(part_of_a_longer_name)
    })
}

/// The whole-name rule on both cases it was written for, and on the ways a
/// description does name a card.
#[test]
fn a_card_is_named_only_by_its_whole_name() {
    // A longer card's name does not name the shorter card it begins with.
    assert!(!names_card("energy/reactor_spert3_fuel", "reactor_spert3"));
    assert!(names_card("energy/reactor_spert3_fuel", "reactor_spert3_fuel"));
    // A field that holds a card's name inside it does not name the card.
    assert!(!names_card("initial.thermal_energy_j", "thermal"));
    assert!(!names_card("the plasma's thermal_energy_j and fast_ion_energy_j", "thermal"));
    assert!(!names_card("geothermal", "thermal"));
    // Named as a card is named: after a slash, in quotes, in a sentence, at
    // either end of the text, and beside a longer name that holds it.
    for text in [
        "energy/thermal",
        "\"card\":\"thermal\"",
        "the card thermal, in the energy domain",
        "thermal",
        "thermal.",
        "reactor_spert3_fuel and thermal",
        "thermal_energy_j of thermal",
    ] {
        assert!(names_card(text, "thermal"), "{text}");
    }
    assert!(names_card("energy/reactor_spert3 and energy/reactor_spert3_fuel", "reactor_spert3"));
    assert!(!names_card("", "thermal"));
}

/// The `plan` input of `run_dataset` carries one worked plan of each kind.
/// Each is sent exactly as it is written, in the full view and in the view
/// of every single card, and each gives a sealed dataset whose check holds.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn each_worked_plan_in_the_description_runs_as_written_and_is_a_sealed_dataset() {
    let state = new_gateway_state();
    let mut views: Vec<(String, View)> = vec![("full".to_string(), View::full())];
    for item in descriptor::all() {
        views.push((item.card.to_string(), one_card(item.domain, item.card)));
    }
    // Two cards and no cold plate: the plans are the first card's.
    views.push((
        "pack and cell".to_string(),
        View::limited(
            [("energy".to_string(), "battery_sag".to_string()), ("energy".to_string(), "cell_18650pf".to_string())],
            [],
        ),
    ));
    let mut sent = 0;
    for (label, view) in &views {
        // The description sends the reader to the input schema for them.
        assert!(description_of(view, "run_dataset").contains("The input schema says how each input is written, with a worked plan of each kind."), "{label}");
        let description = said_at(view, "run_dataset", Some("plan"));
        assert!(description.contains("One worked plan of each kind, each a whole request:"), "{label}");
        // How a path is written, what a varied value replaces, and what
        // cannot vary are said.
        assert!(description.contains("segments[0].load.<field>, with the segment's number in square brackets"), "{label}");
        assert!(description.contains("A value an axis or a draw sets replaces the base's value of that field"), "{label}");
        assert!(description.contains("steps and record_every are not paths and cannot vary: every row of a grid or a sample takes them from the base run."), "{label}");
        for (kind, rows_at_least) in [("list", 1), ("grid", 2), ("sample", 2)] {
            let text = written_plan(&description, kind);
            let request: Value = serde_json::from_str(&text).unwrap_or_else(|_| panic!("{label} {kind}: {text}"));
            assert_eq!(request["plan"]["kind"], kind, "{label}");
            // The request names a card of the view, and no other.
            let (domain, card) = (request["domain"].as_str().unwrap(), request["card"].as_str().unwrap());
            assert!(view.find(domain, card).is_some(), "{label} {kind} names {domain}/{card}");
            // As written, the answer is the summary: one row per run.
            let summary = answer(&state, "run_dataset", &request).await;
            assert_eq!(summary["summary"].as_array().map(Vec::len), summary["head"]["rows"].as_u64().map(|n| n as usize), "{label} {kind}");
            let mut full = request.clone();
            full["answer"] = json!("full");
            let body = answer(&state, "run_dataset", &full).await;
            let head = &body["head"];
            assert_eq!(head, &summary["head"], "{label} {kind}");
            let seal = head["dataset_seal"].as_str().unwrap_or_else(|| panic!("{label} {kind}: {body}"));
            assert!(seal.len() == 96 && seal.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')), "{label} {kind}");
            let rows = body["rows"].as_array().unwrap();
            assert!(rows.len() >= rows_at_least, "{label} {kind}: {} rows", rows.len());
            assert_eq!(head["rows"], rows.len(), "{label} {kind}");
            assert_eq!(head["unadmitted"], 0, "{label} {kind}: {body}");
            for row in rows {
                assert!(row["run_seal"].is_string(), "{label} {kind}: {row}");
            }
            let report = answer(
                &state,
                "verify_dataset",
                &json!({ "head": head, "rows": rows, "frames_hex": body["frames_hex"] }),
            )
            .await;
            assert_eq!(report["holds"], true, "{label} {kind}: {report}");
            assert_eq!(report["rows_file"], "matches the hash in the head", "{label} {kind}");
            assert!(report.get("what_to_report").is_none(), "{label} {kind}: {report}");
            sent += 1;
        }
    }
    // 0.15.0: the full view, one view per card (36) and one more view: 38, three plans each.
    assert_eq!(sent, 3 * 38);
    // The full view and the cold plate's own view show the cold plate, with both kinds of path.
    for view in [View::full(), one_card("energy", "thermal")] {
        let description = said_at(&view, "run_dataset", Some("plan"));
        let grid = written_plan(&description, "grid");
        assert!(grid.contains("\"path\":\"initial.temperature_k\"") && grid.contains("\"path\":\"segments[0].load.heat_power_w\""), "{grid}");
        let request: Value = serde_json::from_str(&grid).unwrap();
        assert!(request["plan"]["base"]["initial"].as_object().unwrap().is_empty());
        assert!(request["plan"]["base"]["segments"][0]["load"].get("heat_power_w").is_none());
        let body = answer(&state, "run_dataset", &request).await;
        assert_eq!(body["head"]["rows"], 6);
        assert_eq!(body["head"]["completed"], 6);
    }
    // A view of one card says nothing of another card.
    for item in descriptor::all() {
        let view = one_card(item.domain, item.card);
        let description = format!("{} {}", description_of(&view, "run_dataset"), said_at(&view, "run_dataset", Some("plan")));
        for other in descriptor::all() {
            if other.card != item.card {
                assert!(!names_card(&description, other.card), "{} names {}", item.card, other.card);
            }
        }
    }
    // A view with no card has no plan to show, and says the rest.
    let none = said_at(&View::limited([], []), "run_dataset", Some("plan"));
    assert!(!none.contains("One worked plan"), "{none}");
    assert!(none.contains("A path names one number"), "{none}");
}

/// The `record_every` input of `run_trajectory` and the `answer` input of
/// `run_dataset` say how large an answer a request makes. Measured on the
/// worked run of every card, recorded at every step and recorded once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_answer_is_as_large_as_the_descriptions_say() {
    let state = new_gateway_state();
    assert!(description_of(&View::full(), "run_trajectory").contains("record_every sets which steps are recorded, about 262 characters each"));
    let run_words = said_at(&View::full(), "run_trajectory", Some("record_every"));
    assert!(run_words.contains("about 262 characters for each recorded state, plus the request as it was sent, plus up to 8000 characters for the final state and the seals; a flagged state adds about 400 characters for each limit it is past (exceedances)"), "{run_words}");
    assert!(run_words.contains("What keeps it small: a larger record_every; the final state is returned in full whatever is recorded."), "{run_words}");
    let set_words = said_at(&View::full(), "run_dataset", Some("answer"));
    assert!(set_words.contains("a whole row counts its own request and 1900 characters, and for each number twice its name and 60 characters (150 for a 3-vector, 9 for a flag); store \"frames\" adds 128 characters for each recorded state to a full answer; the head repeats the request."), "{set_words}");
    assert!(set_words.contains("one above 42500 characters is refused with what to change"), "{set_words}");
    assert_eq!(dataset::REST_OF_AN_ANSWER_CHARS, 8000);

    for item in descriptor::all() {
        let description = descriptor::describe_value_in(&View::full(), item.domain, item.card).unwrap();
        let example = description["example_run_trajectory"].clone();
        let steps: u64 = example["segments"].as_array().unwrap().iter().map(|s| s["steps"].as_u64().unwrap()).sum();
        let mut every_step = example.clone();
        every_step["record_every"] = json!(1);
        let mut once = example.clone();
        once["record_every"] = json!(steps);
        for request in [&every_step, &once] {
            let (is_error, text) = call(&state, "run_trajectory", request).await;
            assert!(!is_error, "{text}");
            let body: Value = serde_json::from_str(&text).unwrap();
            let recorded = body["frames"].as_array().unwrap().len();
            let request_chars = serde_json::to_string(request).unwrap().len();
            // A flagged state's records, and the run's extremes, stand
            // beside the 262 (`src/exceedance.rs`).
            let records: usize = body["frames"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|frame| frame.get("exceedances"))
                .chain(body.get("exceedance_extremes"))
                .map(|list| serde_json::to_string(list).unwrap().len() + 22)
                .sum();
            let rest = text.len() as i64 - request_chars as i64 - 262 * recorded as i64 - records as i64;
            // About 262 for each recorded state (its 96-digit link): no more than 6 either way.
            assert!(
                (-6 * recorded as i64..=dataset::REST_OF_AN_ANSWER_CHARS as i64 + 6 * recorded as i64).contains(&rest),
                "{}: {} characters, {recorded} recorded, request {request_chars}",
                item.card,
                text.len()
            );
        }
        // Recorded once, the answer is the request and the rest, and one state.
        let (_, text) = call(&state, "run_trajectory", &once).await;
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["frames"].as_array().unwrap().len(), if body["steps_landed"] == 0 { 0 } else { 1 }, "{}", item.card);
        assert!(body["final_state"].is_object() || body["steps_landed"] == 0, "{}", item.card);

        // The dataset: two rows of the same run, with the frames and without.
        let mut run = once.clone();
        run.as_object_mut().unwrap().remove("domain");
        run.as_object_mut().unwrap().remove("card");
        for store in ["frames", "none"] {
            let request = json!({
                "domain": item.domain, "card": item.card,
                "plan": { "kind": "list", "runs": [run, run] },
                "store": store, "return": "inline", "answer": "full"
            });
            let (is_error, text) = call(&state, "run_dataset", &request).await;
            assert!(!is_error, "{text}");
            let body: Value = serde_json::from_str(&text).unwrap();
            let request_chars = serde_json::to_string(&request).unwrap().len();
            let mut recorded = 0;
            for row in body["rows"].as_array().unwrap() {
                let row_chars = serde_json::to_string(row).unwrap().len();
                let own_request = serde_json::to_string(&row["request"]).unwrap().len();
                assert!(row_chars <= own_request + dataset::REST_OF_AN_ANSWER_CHARS, "{} row: {row_chars} against request {own_request}", item.card);
                recorded += row["frames"].as_u64().unwrap() as usize;
            }
            match store {
                // 128 characters for each recorded state, after the 80-byte header of the frame file.
                "frames" => assert_eq!(body["frames_hex"].as_str().unwrap().len(), 128 * recorded + 160, "{}", item.card),
                _ => assert!(body["frames_hex"].is_null(), "{}", item.card),
            }
            let head_chars = serde_json::to_string(&body["head"]).unwrap().len();
            assert!(head_chars >= request_chars && head_chars <= request_chars + 1000, "{}: head {head_chars}, request {request_chars}", item.card);
        }
    }
}

/// A check by seal of a dataset that is not stored says how a dataset
/// returned in the answer is checked, on a gateway with a directory and on
/// one without, and doing what it says gives a check that holds.
#[tokio::test]
async fn a_check_by_seal_of_a_dataset_that_is_not_stored_says_how_a_returned_one_is_checked() {
    let folder = Folder::new();
    let with_directory = new_gateway_state().with_dataset_dir(Some(folder.0.clone()));
    let without = new_gateway_state();
    let plan = serde_json::from_str::<Value>(&written_plan(&said_at(&View::full(), "run_dataset", Some("plan")), "list")).unwrap();
    assert_eq!(plan["return"], "inline");
    let summary = answer(&with_directory, "run_dataset", &plan).await;
    assert!(summary["summary"].is_array(), "{summary}");
    let mut plan = plan;
    plan["answer"] = json!("full");
    let made = answer(&with_directory, "run_dataset", &plan).await;
    assert_eq!(made["head"], summary["head"]);
    let seal = made["head"]["dataset_seal"].clone();

    let how = "A dataset returned in the answer (return \"inline\") is not stored: verify it by sending its run_dataset request as 'request' with its 'dataset_seal', which runs the plan again, or by sending back the head, rows and frames_hex of its full answer (answer \"full\").";
    let (is_error, text) = call(&with_directory, "verify_dataset", &json!({ "dataset_seal": seal })).await;
    assert!(is_error);
    assert_eq!(text, format!("no dataset is stored under that seal. {how}"));
    let (is_error, text) = call(&without, "verify_dataset", &json!({ "dataset_seal": seal })).await;
    assert!(is_error);
    assert_eq!(text, format!("this connection was started without a directory for dataset files. {how}"));
    // A file job on a gateway without a directory keeps its own sentence: it was not asking for a check.
    let mut to_file = plan.clone();
    to_file["return"] = json!("file");
    let (is_error, text) = call(&without, "run_dataset", &to_file).await;
    assert!(is_error);
    assert_eq!(text, "this connection was started without a directory for dataset files.");

    // What the sentence says to do.
    for state in [&with_directory, &without] {
        let report = answer(
            state,
            "verify_dataset",
            &json!({ "head": made["head"], "rows": made["rows"], "frames_hex": made["frames_hex"] }),
        )
        .await;
        assert_eq!(report["holds"], true, "{report}");
        assert_eq!(report["dataset_seal"], seal);
    }
    // The declaration of the tool says it too.
    let tools = tools_in(&View::full());
    let tool = tools.iter().find(|item| item.name == "verify_dataset").unwrap();
    let words = tool.input_schema["properties"]["dataset_seal"]["description"].as_str().unwrap();
    assert!(words.contains("A dataset returned in the answer is not stored: it is verified with its request, or by its head, rows and frames_hex."), "{words}");
    // A dataset that was stored is still found by its seal.
    let stored = answer(&with_directory, "run_dataset", &to_file).await;
    let report = answer(&with_directory, "verify_dataset", &json!({ "dataset_seal": stored["head"]["dataset_seal"] })).await;
    assert_eq!(report["holds"], true, "{report}");
}

/// When every seal holds and the rows sent back are not, byte for byte, the
/// rows the engine writes for them, the long form does not hold: the cause is
/// answer_text, and differs_at and difference name the first place, with its
/// row. In every other case the answer is as it was.
#[tokio::test]
async fn a_check_whose_seals_hold_over_rows_the_engine_did_not_write_does_not_hold_and_names_the_place() {
    let state = new_gateway_state();
    let mut plan = serde_json::from_str::<Value>(&written_plan(&said_at(&View::full(), "run_dataset", Some("plan")), "grid")).unwrap();
    plan["answer"] = json!("full");
    let made = answer(&state, "run_dataset", &plan).await;
    let check = |rows: Value, frames: Value| json!({ "head": made["head"], "rows": rows, "frames_hex": frames });
    const KEYS: [&str; 11] =
        ["holds", "values_hold", "cause", "row", "broken_link", "chains", "merkle_root", "dataset_seal", "receipt", "rows_file", "frames_file"];
    let keys_of = |report: &Value| -> Vec<String> { report.as_object().unwrap().keys().cloned().collect() };
    let with_difference = || {
        let mut keys = KEYS.map(String::from).to_vec();
        keys.extend(["differs_at".to_string(), "difference".to_string()]);
        keys
    };

    // As returned: holds, the rows match, and the answer has its eleven keys.
    let report = answer(&state, "verify_dataset", &check(made["rows"].clone(), made["frames_hex"].clone())).await;
    assert_eq!(report["holds"], true);
    assert_eq!(report["rows_file"], "matches the hash in the head");
    assert_eq!(keys_of(&report), KEYS);

    // A decimal that is not the value of its hex: the seals hold, the text is
    // not the engine's, and the row and the field are named.
    let mut decimal = made["rows"].clone();
    let exact = decimal[1]["final_state"]["temperature_k_hex"].clone();
    decimal[1]["final_state"]["temperature_k"] = json!(293.0);
    assert_eq!(decimal[1]["final_state"]["temperature_k_hex"], exact);
    let report = answer(&state, "verify_dataset", &check(decimal, made["frames_hex"].clone())).await;
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["row"].clone()), (json!(false), json!("answer_text"), json!(1)), "{report}");
    assert_eq!(report["differs_at"], "rows[1].final_state.temperature_k", "{report}");
    assert!(report["merkle_root"].is_null() && report["dataset_seal"].is_null(), "{report}");
    assert_eq!(keys_of(&report), with_difference());
    // A decimal taken out, and one added that the card does not publish.
    let mut taken = made["rows"].clone();
    taken[2]["final_state"].as_object_mut().unwrap().shift_remove("temperature_k");
    let report = answer(&state, "verify_dataset", &check(taken, made["frames_hex"].clone())).await;
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!("rows[2].final_state.temperature_k")), "{report}");
    // A refusal sentence is sealed by its leaf.
    let mut sentence = made["rows"].clone();
    sentence[0]["reject_detail"] = json!("copied by hand");
    let report = answer(&state, "verify_dataset", &check(sentence, made["frames_hex"].clone())).await;
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["row"].clone()), (json!(false), json!("leaf"), json!(0)), "{report}");
    assert!(report["fault"].as_str().unwrap().contains("its run_seal and its reject_detail give"), "{report}");
    // The keys of a row in another order: every value is the engine's, the
    // rows sent do not hash to the head's rows_sha384, and the plan run again
    // names the first key out of the engine's order.
    let mut reordered = made["rows"].clone();
    let first = reordered[2].as_object().unwrap().clone();
    let mut backwards = Map::new();
    for (key, value) in first.iter().rev() {
        backwards.insert(key.clone(), value.clone());
    }
    let last_key = first.keys().last().unwrap().clone();
    reordered[2] = Value::Object(backwards);
    let report = answer(&state, "verify_dataset", &check(reordered, made["frames_hex"].clone())).await;
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!(format!("rows[2].{last_key}"))), "{report}");
    assert_eq!(report["rows_file"], "does not match the hash in the head");
    assert_eq!(keys_of(&report), with_difference());
    // A head of another engine version is named as such.
    let mut other = made["head"].clone();
    other["engine_version"] = json!("genesis_kernel_v0.9.0");
    let report = answer(&state, "verify_dataset", &json!({ "head": other, "rows": made["rows"], "frames_hex": made["frames_hex"] })).await;
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("engine_version"), json!("head.engine_version")), "{report}");
    assert!(report["difference"].as_str().unwrap().starts_with("The dataset names engine_version genesis_kernel_v0.9.0, and this engine is genesis_kernel_v0.13.1."), "{report}");

    // A sealed value changed: the check does not hold, and no such sentence is given.
    let mut broken = made["rows"].clone();
    broken[1]["final_state"]["temperature_k_hex"] = json!("4072500000000000");
    let report = answer(&state, "verify_dataset", &check(broken, made["frames_hex"].clone())).await;
    assert_eq!(report["holds"], false);
    assert_eq!(report["cause"], "state");
    assert_eq!(report["rows_file"], "does not match the hash in the head");
    // An answer that does not hold says the kind of fault in words after its eleven keys.
    let mut keys = KEYS.map(String::from).to_vec();
    keys.push("fault".to_string());
    assert_eq!(keys_of(&report), keys);
    assert!(report["fault"].as_str().unwrap().starts_with("A row was changed: a sealed value of row 1 was altered."), "{report}");
}

/// The description of `run_trajectory` says in short how a run comes back,
/// and its input schema says in full how each of the two kinds of refused
/// run comes back and what the two openings of a refusal sentence mean; the
/// `answer` input of `run_dataset` says what a refused row and an unadmitted
/// row carry. Each is made here and is as described.
#[tokio::test]
async fn the_two_kinds_of_refusal_come_back_as_the_descriptions_say() {
    let state = new_gateway_state();
    let run_words = description_of(&View::full(), "run_trajectory");
    for part in [
        "A run comes back in one of four ways.",
        "outcome \"ended\": the run reached an end its card declares, which describe_card lists under declared_ends; reject holds the end's name.",
        "outcome \"refused\": a step part way was refused; reject holds the name (null when the cause has none) and reject_detail the sentence.",
        "An ended or a refused run is sealed up to its last landed step, steps_landed.",
        "An error, not a result: the request was turned away before computing, in one sentence with the refusal's name in round brackets when it has one, and nothing is sealed.",
        "A sentence that begins \"Physical Failure\" says the state asked for or reached is outside the card; one that begins \"Physical Violation\" says the request, or a number in it or computed from it, is not one the engine takes.",
    ] {
        assert!(run_words.contains(part), "{part}");
    }
    assert!(said_at(&View::full(), "run_trajectory", None).ends_with(REFUSAL_WORDS));
    for part in [
        "A request turned away before computing is an error, not a result: its text is one sentence, nothing is sealed, and when the sentence gives the card's name for the refusal the name is in round brackets.",
        "A run that starts and is refused part way is a result: outcome is \"refused\", reject holds the name (null when the cause has none), reject_detail holds the sentence, which ends \"The run is sealed to step N. The state at step N+1 was refused and is not sealed.\", and the run is sealed up to that last landed step N: steps_landed is N, the count of steps that landed, so the step that was refused is steps_landed + 1.",
        "A run that reaches an end its card declares (the first maximum of power of a reactor burst, a capsule's flown parachute deployment, a crater that stops growing, a cell's runaway once every reactant is consumed) is not refused: outcome is \"ended\", reject holds the end's name, reject_detail a sentence beginning \"Ended\" that names the end, and the run is sealed up to its last landed step; describe_card lists a card's declared ends under declared_ends.",
        "A sentence that begins \"Physical Failure\" says the state asked for or reached is outside the card: past a published bound, a limit of the specimen, or the conservation ledger.",
        "A sentence that begins \"Physical Violation\" says the request, or a number in it or computed from it, is not one the engine takes: its shape, a missing or unknown key, the number rule, the step floor or ceiling, a load rule, a size limit, or a value that is not finite.",
    ] {
        assert!(REFUSAL_WORDS.contains(part), "{part}");
    }
    assert!(description_of(&View::full(), "run_dataset").contains("A row is completed, ended, refused or unadmitted: an ended or a refused row is a run sealed up to its last landed step, and an unadmitted row is a request turned away before computing, with no seal."));
    let set_words = said_at(&View::full(), "run_dataset", Some("answer"));
    assert!(set_words.contains("A refused row is a run the card refused part way: it has a run seal, the refusal's name in reject and its sentence in reject_detail."), "{set_words}");
    assert!(set_words.contains("An unadmitted row is a request turned away before computing: it has no run seal, and its sentence, with the refusal's name in round brackets when the sentence gives one, is its reject_detail; its summary row gives that name in reject."), "{set_words}");
    assert!(set_words.contains("What a sentence that begins \"Physical Failure\" or \"Physical Violation\" means is in the description of run_trajectory."), "{set_words}");

    let plate = |temperature_k: f64, power_w: f64, steps: u64| {
        json!({
            "initial": { "temperature_k": temperature_k }, "dt_s": 1.0,
            "segments": [{ "steps": steps, "load": { "heat_power_w": power_w, "ambient_k": 293.15 } }],
            "record_every": steps
        })
    };
    let named = |run: &Value| {
        let mut request = run.clone();
        request["domain"] = json!("energy");
        request["card"] = json!("thermal");
        request
    };
    // Turned away on its start state: an error, one sentence, the name in round brackets, nothing sealed.
    let outside = plate(400.0, 1.0, 10);
    let (is_error, door_text) = call(&state, "run_trajectory", &named(&outside)).await;
    assert!(is_error);
    assert_eq!(
        door_text,
        "Physical Failure: 'temperature_k' 400 K is at or above 398.15 K, the exclusive maximum of the thermal card (TemperatureOutsideCard). That state is not sealed."
    );
    assert!(serde_json::from_str::<Value>(&door_text).is_err());
    // Refused part way: a result, the name in a field, the sentence beside it, and a seal.
    let hot = plate(390.0, 3000.0, 600);
    let body = answer(&state, "run_trajectory", &named(&hot)).await;
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "TemperatureOutsideCard");
    let landed = body["steps_landed"].as_u64().unwrap();
    assert_eq!(
        body["reject_detail"],
        format!(
            "Physical Failure: the plate left 233.15 K to 398.15 K (TemperatureOutsideCard). The run is sealed to step {landed}. The state at step {} was refused and is not sealed.",
            landed + 1
        )
    );
    assert!(landed > 0 && body["run_seal"].is_string());
    // A request that is not one the engine takes: the other opening, at the door.
    let mut unknown = named(&plate(293.15, 1.0, 10));
    unknown["initial"]["pressure_pa"] = json!(1.0);
    let (is_error, text) = call(&state, "run_trajectory", &unknown).await;
    assert!(is_error);
    assert_eq!(text, "Physical Violation: 'pressure_pa' is not a field of this request.");
    let mut too_long = named(&plate(293.15, 1.0, 10));
    too_long["dt_s"] = json!(1.0e6);
    let (is_error, text) = call(&state, "run_trajectory", &too_long).await;
    assert!(is_error && text.starts_with("Physical Violation: 'dt_s' 1000000 is above"), "{text}");
    // And part way: a value that is not finite has no name, and is a Physical Violation.
    let over = json!({
        "domain": "signal", "card": "tesseract",
        "initial": { "displacement_m": 0.001, "velocity_m_s": 0.0, "energy_defect_j": 0.0 }, "dt_s": 3.0e-5,
        "segments": [{ "steps": 1, "load": { "accel_m_s2": 0.0, "omega_ext_rad_s": 3.0e38, "sense_velocity_m_s": 3.0e38 } }],
        "record_every": 1
    });
    let body = answer(&state, "run_trajectory", &over).await;
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], Value::Null);
    assert!(body["reject_detail"].as_str().unwrap().starts_with("Physical Violation: "));

    // The same three as rows of one dataset.
    let request = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "list", "runs": [plate(293.15, 1.0, 10), hot, outside] },
        "store": "none", "return": "inline", "answer": "full"
    });
    let made = answer(&state, "run_dataset", &request).await;
    let rows = made["rows"].as_array().unwrap();
    assert_eq!(rows[0]["outcome"], "completed");
    assert_eq!(rows[1]["outcome"], "refused");
    assert_eq!(rows[1]["reject"], "TemperatureOutsideCard");
    assert_eq!(rows[1]["reject_detail"], body_detail(&state, &named(&rows[1]["request"])).await);
    assert!(rows[1]["run_seal"].is_string());
    assert_eq!(rows[2]["outcome"], "unadmitted");
    assert_eq!(rows[2]["reject"], Value::Null);
    assert_eq!(rows[2]["run_seal"], Value::Null);
    assert_eq!(rows[2]["reject_detail"], door_text);
}

/// The `reject_detail` of the run a row's request makes alone.
async fn body_detail(state: &AppState, request: &Value) -> Value {
    answer(state, "run_trajectory", request).await["reject_detail"].clone()
}

/// The connections an agent reads a tool list on: the local connection and
/// a hosted connection with a key, in the full view, in the view of every
/// single card and of every single solver and in a view of nothing; and the
/// open connection as it starts by default, as a router is made in a test,
/// and with every limit that is read at start at the greatest value its
/// argument takes, where the numbers it states are longest.
fn every_connection() -> Vec<(String, View, Option<crate::open_door::OpenRules>)> {
    use std::time::Duration;

    let mut views: Vec<(String, View)> = vec![("full".to_string(), View::full()), ("nothing".to_string(), View::limited([], []))];
    for item in descriptor::all() {
        views.push((item.card.to_string(), one_card(item.domain, item.card)));
    }
    for solver in View::full().solver_names() {
        views.push((solver.to_string(), View::limited([], [solver.to_string()])));
    }
    let mut connections: Vec<_> = views.into_iter().map(|(label, view)| (format!("keyed or local, {label}"), view, None)).collect();
    let as_started = crate::open_door::OpenRules { worker: Some(std::path::PathBuf::from("hosted_door")), ..crate::open_door::OpenRules::for_this_machine() };
    let greatest = crate::open_door::OpenRules {
        requests_per_minute: 1_000_000,
        max_in_flight: 10_000,
        max_in_flight_per_address: 10_000,
        physics_permits: 32,
        permits_per_address: 32,
        request: Duration::from_secs(3600),
        ..as_started.clone()
    };
    let one_core = crate::open_door::OpenRules { physics_permits: 2, ..as_started.clone() };
    for (label, rules) in [
        ("open, as started", as_started),
        ("open, one core", one_core),
        ("open, every argument at its greatest", greatest),
        ("open, a router with no binary to start", crate::open_door::OpenRules::for_this_machine()),
    ] {
        connections.push((label.to_string(), View::full(), Some(rules)));
    }
    connections
}

/// verify_dataset's description, which on the open connection carries that
/// connection's dataset limits and its words for a call that finds no
/// permit, keeps room for a sentence on every connection: at least
/// `VERIFY_DATASET_ROOM` bytes under the limit, with every limit read at
/// start at its greatest.
const VERIFY_DATASET_ROOM: usize = 64;

#[test]
fn verify_datasets_description_keeps_room_for_a_sentence_on_every_connection() {
    for (label, view, open) in every_connection() {
        let tools = crate::mcp_server::tools_for(&view, open.as_ref());
        let tool = tools.iter().find(|tool| tool.name == "verify_dataset").expect("verify_dataset is served");
        let length = tool.description.as_deref().unwrap_or("").len();
        assert!(length + VERIFY_DATASET_ROOM <= crate::identity::MOST_CHARS_SAID, "{label}: {length} bytes");
    }
}

/// What a client shows of a description and of the instructions. Claude Code
/// cuts each at 2,048 characters, so nothing an agent needs may lie past it.
const CLIENT_CUT: usize = 2_048;

/// The word each description opens with: what the tool does.
const OPENS_WITH: [(&str, &str); 18] = [
    ("list_cards", "Lists"),
    ("describe_card", "Describes"),
    ("compute_step", "Computes"),
    ("run_trajectory", "Runs"),
    ("verify_tape", "Verifies"),
    ("list_solvers", "Lists"),
    ("describe_solver", "Describes"),
    ("solve", "Solves"),
    ("verify_solution", "Verifies"),
    ("run_dataset", "Runs"),
    ("verify_dataset", "Verifies"),
    ("verify_receipt", "Verifies"),
    ("validate_series", "Runs"),
    ("cite", "Gives"),
    ("find_model", "Finds"),
    ("find_flip", "Finds"),
    ("design_run", "Designs"),
    ("verify_step", "Computes"),
];

/// Every sentence an input schema says: the words under each `description`,
/// one after another.
fn said_in(schema: &Value) -> String {
    match schema {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| match value {
                Value::String(words) if key == "description" => format!("{words} "),
                other => said_in(other),
            })
            .collect(),
        Value::Array(items) => items.iter().map(said_in).collect(),
        _ => String::new(),
    }
}

/// A sentence written as a command to the agent that reads it: it opens with
/// a bare verb, or tells the agent to send or read something after a colon
/// or a semicolon. A description states what is so.
fn commands_in(text: &str) -> Vec<String> {
    const VERBS: [&str; 18] = [
        "Send", "Read", "Use", "Call", "See", "List", "Take", "Run", "Check", "Solve", "Do", "Leave", "Set", "Add", "Map", "Write", "Note", "Wait",
    ];
    let mut found = Vec::new();
    for sentence in text.split(". ") {
        let first = sentence.trim_start().split(' ').next().unwrap_or("");
        if VERBS.contains(&first) || first == "To" {
            found.push(sentence.to_string());
        }
    }
    for told in [": send ", "; send ", ", send ", ": read ", "; read ", "is sent again", "send it again", "do not send", "need be read"] {
        if text.contains(told) {
            found.push(told.to_string());
        }
    }
    found
}

/// Every tool's description and the instructions, on every connection, fit
/// inside what a client shows; each description opens with what its tool
/// does and carries a title; none names a route of the web connection, which
/// no MCP connection serves; none uses a word of the engine's own code for
/// the agent or the verifier; and none is written as a command. On the open
/// connection the limits of each tool that computes, and what a call that
/// finds no permit is answered, are inside its description.
#[test]
fn what_every_connection_says_fits_what_a_client_shows_and_states_what_is_so() {
    let said_wrong = |label: &str, what: &str, text: &str| {
        // Bytes, the largest of the ways a client may count a length.
        assert!(text.len() <= crate::identity::MOST_CHARS_SAID, "{label}, {what}: {} characters", text.len());
        assert!(crate::identity::MOST_CHARS_SAID < CLIENT_CUT);
        assert!(!text.contains("POST /") && !text.contains("GET /"), "{label}, {what} names a route of the web connection: {text}");
        let lower = text.to_ascii_lowercase();
        for word in ["caller", "the door", "the gateway"] {
            assert!(!lower.contains(word), "{label}, {what} says \"{word}\": {text}");
        }
        // Super Intelligence, never the two letters.
        assert!(!text.split(|c: char| !c.is_ascii_alphanumeric()).any(|word| word == "AI"), "{label}, {what}: {text}");
        assert_eq!(commands_in(text), Vec::<String>::new(), "{label}, {what}: {text}");
    };
    // The check finds a command: each of these told the agent what to do.
    for command in [
        "With form \"short\" the answer is a page. Read the full form, the answer when form is left out, when a request is refused.",
        "It does not show the engine made it: send a row's request to run_trajectory and compare run seals.",
        "The receipt covers the request. To check a solution later, send this request and its solution_seal to verify_solution.",
        "No new run after 20 seconds of computing (FLIP_MAX_SECONDS); send the pair back as the span to narrow it.",
        "A call that finds no permit is answered \"Too many concurrent physics tasks.\" and is sent again; nothing waits.",
        "The names it gives are the names every other tool takes, so no list need be read after it.",
        "A model is described by describe_card. List the domain/card pairs, read one pair, take a step.",
    ] {
        assert!(!commands_in(command).is_empty(), "{command}");
    }
    let mut longest = 0;
    for (label, view, open) in every_connection() {
        let tools = crate::mcp_server::tools_for(&view, open.as_ref());
        assert_eq!(tools.iter().map(|tool| tool.name.as_ref()).collect::<Vec<_>>(), OPENS_WITH.map(|(name, _)| name), "{label}");
        for (tool, (name, opening)) in tools.iter().zip(OPENS_WITH) {
            let text = tool.description.as_deref().unwrap_or("");
            said_wrong(&label, name, text);
            longest = longest.max(text.len());
            assert_eq!(text.split(' ').next(), Some(opening), "{label}, {name}: {text}");
            // The input schema is read by the same agent: it names no route
            // of the web connection and uses none of the engine's own words.
            let declared = said_in(&Value::Object(tool.input_schema.as_ref().clone())).to_ascii_lowercase();
            for word in ["post /", "get /", "caller", "the door", "the gateway"] {
                assert!(!declared.contains(word), "{label}, the input schema of {name} says \"{word}\"");
            }
            // The title, at the top of the tool as revision 2025-06-18 has it.
            assert_eq!(tool.title.as_deref(), Some(crate::mcp_server::title_of(name).as_str()), "{label}, {name}");
            let computes = ["compute_step", "verify_step", "run_trajectory", "solve", "validate_series", "verify_tape", "verify_solution", "run_dataset", "verify_dataset", "find_flip", "design_run"].contains(&name);
            match &open {
                Some(rules) => {
                    let hints = tool.annotations.as_ref().unwrap_or_else(|| panic!("{label}, {name} has no hints"));
                    assert_eq!(hints.title.as_deref(), tool.title.as_deref(), "{label}, {name}");
                    assert_eq!((hints.read_only_hint, hints.destructive_hint), (Some(true), Some(false)), "{label}, {name}");
                    // Nothing is stored there, and nothing of a stored dataset is described.
                    assert!(!text.contains("\"file\"") && !text.contains("stored as files") && !text.contains("dataset_seal alone"), "{label}, {name}: {text}");
                    assert_eq!(text.contains(" On this open connection "), computes, "{label}, {name}: {text}");
                    if computes {
                        let permits = rules.permits_per_address;
                        let held = if permits == 1 { "1 physics permit".to_string() } else { format!("{permits} physics permits") };
                        assert!(text.contains(&format!("ne address holds at most {held} at a time")), "{label}, {name}: {text}");
                        assert!(text.contains("is answered \"Too many concurrent physics tasks.\" and nothing waits: the same call sent later is answered once one is free."), "{label}, {name}: {text}");
                        assert!(text.contains(&format!("An answer is waited for {} seconds", rules.request.as_secs())), "{label}, {name}: {text}");
                    }
                }
                None => assert!(tool.annotations.is_none() && !text.contains("open connection"), "{label}, {name}: {text}"),
            }
        }
        let mut instructions = crate::identity::instructions();
        if let Some(rules) = &open {
            let mut info = json!({ "instructions": instructions });
            rules.state_in_instructions(&mut info);
            instructions = info["instructions"].as_str().unwrap().to_string();
            // The limits no description states: requests and in flight (429), the permits, and both limits on a
            // body with the six tools that take the larger (since 0.14.0 rc2; RT3 W5). Since 0.14.2 the busy answer
            // and the wait are left to each tool that computes, which the last sentence names.
            let tasks = if rules.permits_per_address == 1 { "1 physics task".to_string() } else { format!("{} physics tasks", rules.permits_per_address) };
            assert!(instructions.contains(&format!("From one address: at most {} requests a minute and {} in flight ({} in all), then HTTP 429 with Retry-After; {tasks} at a time;", rules.requests_per_minute, rules.max_in_flight_per_address, rules.max_in_flight)), "{label}: {instructions}");
            assert!(instructions.contains(&format!("a body of at most {} bytes to {} or {} and {} to any other tool, then HTTP 413", rules.max_body_bytes, crate::guard::WIDE_TOOLS[..5].join(", "), crate::guard::WIDE_TOOLS[5], crate::guard::NARROW_LIMIT)), "{label}: {instructions}");
            assert!(instructions.ends_with(" Each tool that computes states its own limits in its description."), "{label}: {instructions}");
        }
        said_wrong(&label, "the instructions", &instructions);
        longest = longest.max(instructions.len());
    }
    // The check reads real text: the longest is past half of what a client shows.
    assert!(longest > CLIENT_CUT / 2, "{longest}");
}
