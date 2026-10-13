//! What the tools say of themselves, held to what the tools do.
//!
//! An agent working from the tool list alone needs five things said: how a
//! plan is written, how a dataset returned in the answer is checked, how
//! large an answer a request makes, what a check means when it holds and the
//! rows do not match, and how a refusal comes back. Each is said in a
//! description or in the answer. This probe sends every worked plan as it is
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

/// The description of `run_dataset` carries one worked plan of each kind.
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
        let description = description_of(view, "run_dataset");
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
            assert!(seal.len() == 64 && seal.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')), "{label} {kind}");
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
    assert_eq!(sent, 3 * 33);
    // The full view and the cold plate's own view show the cold plate, with both kinds of path.
    for view in [View::full(), one_card("energy", "thermal")] {
        let description = description_of(&view, "run_dataset");
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
        let description = description_of(&one_card(item.domain, item.card), "run_dataset");
        for other in descriptor::all() {
            if other.card != item.card {
                assert!(!names_card(&description, other.card), "{} names {}", item.card, other.card);
            }
        }
    }
    // A view with no card has no plan to show, and says the rest.
    let none = description_of(&View::limited([], []), "run_dataset");
    assert!(!none.contains("One worked plan"), "{none}");
    assert!(none.contains("A path names one number"), "{none}");
}

/// The descriptions of `run_trajectory` and `run_dataset` say how large an
/// answer a request makes. Measured on the worked run of each of the thirty
/// cards, recorded at every step and recorded once.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_answer_is_as_large_as_the_descriptions_say() {
    let state = new_gateway_state();
    let run_words = description_of(&View::full(), "run_trajectory");
    assert!(run_words.contains("about 230 characters for each recorded state, plus the request as it was sent, plus up to 8000 characters for the final state and the seals"), "{run_words}");
    assert!(run_words.contains("What keeps it small: a larger record_every; the final state is returned in full whatever is recorded."), "{run_words}");
    let set_words = description_of(&View::full(), "run_dataset");
    assert!(set_words.contains("a whole row counts its own request and 1900 characters, and for each number twice its name and 60 characters (150 for a 3-vector, 9 for a flag); store \"frames\" adds 128 characters for each recorded state to a full answer; the head repeats the request."), "{set_words}");
    assert!(set_words.contains("one above 40000 characters is refused with what to change"), "{set_words}");
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
            let rest = text.len() as i64 - request_chars as i64 - 230 * recorded as i64;
            // About 230 for each recorded state: no more than 6 either way.
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
                // 128 characters for each recorded state, after the 64-byte header of the frame file.
                "frames" => assert_eq!(body["frames_hex"].as_str().unwrap().len(), 128 * (recorded + 1), "{}", item.card),
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
    let plan = serde_json::from_str::<Value>(&written_plan(&description_of(&View::full(), "run_dataset"), "list")).unwrap();
    assert_eq!(plan["return"], "inline");
    let summary = answer(&with_directory, "run_dataset", &plan).await;
    assert!(summary["summary"].is_array(), "{summary}");
    let mut plan = plan;
    plan["answer"] = json!("full");
    let made = answer(&with_directory, "run_dataset", &plan).await;
    assert_eq!(made["head"], summary["head"]);
    let seal = made["head"]["dataset_seal"].clone();

    let how = "A dataset returned in the answer (return \"inline\") is not stored: check it by sending its run_dataset request as 'request' with its 'dataset_seal', which runs the plan again, or by sending back the head, rows and frames_hex of its full answer (answer \"full\").";
    let (is_error, text) = call(&with_directory, "verify_dataset", &json!({ "dataset_seal": seal })).await;
    assert!(is_error);
    assert_eq!(text, format!("no dataset is stored under that seal. {how}"));
    let (is_error, text) = call(&without, "verify_dataset", &json!({ "dataset_seal": seal })).await;
    assert!(is_error);
    assert_eq!(text, format!("the gateway was started without a directory for dataset files. {how}"));
    // A file job on a gateway without a directory keeps its own sentence: it was not asking for a check.
    let mut to_file = plan.clone();
    to_file["return"] = json!("file");
    let (is_error, text) = call(&without, "run_dataset", &to_file).await;
    assert!(is_error);
    assert_eq!(text, "the gateway was started without a directory for dataset files.");

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
    assert!(words.contains("A dataset returned in the answer is not stored: send it with its request, or send its head, rows and frames_hex."), "{words}");
    // A dataset that was stored is still found by its seal.
    let stored = answer(&with_directory, "run_dataset", &to_file).await;
    let report = answer(&with_directory, "verify_dataset", &json!({ "dataset_seal": stored["head"]["dataset_seal"] })).await;
    assert_eq!(report["holds"], true, "{report}");
}

/// When every seal holds and the rows sent back are not, byte for byte, the
/// rows the head names, the answer carries one sentence that says what each
/// of the two means and which to report. In every other case the answer is
/// as it was.
#[tokio::test]
async fn a_check_that_holds_over_rows_that_do_not_match_says_what_each_means_and_which_to_report() {
    let state = new_gateway_state();
    let mut plan = serde_json::from_str::<Value>(&written_plan(&description_of(&View::full(), "run_dataset"), "grid")).unwrap();
    plan["answer"] = json!("full");
    let made = answer(&state, "run_dataset", &plan).await;
    let check = |rows: Value, frames: Value| json!({ "head": made["head"], "rows": rows, "frames_hex": frames });
    const KEYS: [&str; 10] =
        ["holds", "cause", "row", "broken_link", "chains", "merkle_root", "dataset_seal", "receipt", "rows_file", "frames_file"];
    let keys_of = |report: &Value| -> Vec<String> { report.as_object().unwrap().keys().cloned().collect() };

    // As returned: holds, the rows match, and the answer has its ten keys.
    let report = answer(&state, "verify_dataset", &check(made["rows"].clone(), made["frames_hex"].clone())).await;
    assert_eq!(report["holds"], true);
    assert_eq!(report["rows_file"], "matches the hash in the head");
    assert_eq!(keys_of(&report), KEYS);

    // Three ways an agent miscopies a row without touching a sealed value.
    let mut decimal = made["rows"].clone();
    let exact = decimal[1]["final_state"]["temperature_k_hex"].clone();
    decimal[1]["final_state"]["temperature_k"] = json!(293.0);
    assert_eq!(decimal[1]["final_state"]["temperature_k_hex"], exact);
    let mut sentence = made["rows"].clone();
    sentence[0]["reject_detail"] = json!("copied by hand");
    let mut reordered = made["rows"].clone();
    let first = reordered[2].as_object().unwrap().clone();
    let mut backwards = Map::new();
    for (key, value) in first.iter().rev() {
        backwards.insert(key.clone(), value.clone());
    }
    reordered[2] = Value::Object(backwards);
    for (label, rows) in [("a decimal", decimal), ("a sentence", sentence), ("the order of keys", reordered)] {
        let report = answer(&state, "verify_dataset", &check(rows, made["frames_hex"].clone())).await;
        assert_eq!(report["holds"], true, "{label}: {report}");
        assert_eq!(report["cause"], Value::Null, "{label}");
        assert_eq!(report["rows_file"], "does not match the hash in the head", "{label}");
        let words = report["what_to_report"].as_str().unwrap_or_else(|| panic!("{label}: {report}"));
        // One sentence: what holds means, what the mismatch means, and which to report.
        assert_eq!(words, dataset::HOLDS_AND_ROWS_DIFFER);
        assert_eq!(words.matches(". ").count(), 0, "{words}");
        assert!(words.starts_with("holds is true because every seal was rebuilt from the exact values of the rows as they were sent"), "{words}");
        assert!(words.contains("rows_file does not match because the text of the rows sent is not byte for byte the text the engine wrote"), "{words}");
        assert!(words.ends_with("report that the dataset holds, and say that the copy checked was not byte-identical."), "{words}");
        let mut keys = KEYS.map(String::from).to_vec();
        keys.push("what_to_report".to_string());
        assert_eq!(keys_of(&report), keys, "{label}");
    }

    // A sealed value changed: the check does not hold, and no such sentence is given.
    let mut broken = made["rows"].clone();
    broken[1]["final_state"]["temperature_k_hex"] = json!("4072500000000000");
    let report = answer(&state, "verify_dataset", &check(broken, made["frames_hex"].clone())).await;
    assert_eq!(report["holds"], false);
    assert_eq!(report["cause"], "state");
    assert_eq!(report["rows_file"], "does not match the hash in the head");
    assert_eq!(keys_of(&report), KEYS);
}

/// The description of `run_trajectory` says how each of the two kinds of
/// refused run comes back and what the two openings of a refusal sentence
/// mean; the description of `run_dataset` says what a refused row and an
/// unadmitted row carry. Each is made here and is as described.
#[tokio::test]
async fn the_two_kinds_of_refusal_come_back_as_the_descriptions_say() {
    let state = new_gateway_state();
    let run_words = description_of(&View::full(), "run_trajectory");
    assert!(run_words.ends_with(REFUSAL_WORDS), "{run_words}");
    for part in [
        "A request the door turns away before computing is an error, not a result: its text is one sentence, nothing is sealed, and when the sentence gives the card's name for the refusal the name is in round brackets.",
        "A run that starts and is refused part way is a result: outcome is \"refused\", reject holds the name (null when the cause has none), reject_detail holds the sentence, which ends \"The run is sealed to step N. The state at step N+1 was refused and is not sealed.\", and the run is sealed up to that last landed step N: steps_landed is N, the count of steps that landed, so the step that was refused is steps_landed + 1.",
        "A run that reaches an end its card declares (the first maximum of power of a reactor burst, a capsule's flown parachute deployment, a mission's last day, the end of a record's clock or track) is not refused: outcome is \"ended\", reject holds the end's name, reject_detail a sentence beginning \"Ended\" that names the end, and the run is sealed up to its last landed step; describe_card lists a card's declared ends under declared_ends.",
        "A sentence that begins \"Physical Failure\" says the state asked for or reached is outside the card: past a published bound, a limit of the specimen, or the conservation ledger.",
        "A sentence that begins \"Physical Violation\" says the request, or a number in it or computed from it, is not one the engine takes: its shape, a missing or unknown key, the number rule, the step floor or ceiling, a load rule, a size limit, or a value that is not finite.",
    ] {
        assert!(REFUSAL_WORDS.contains(part), "{part}");
    }
    let set_words = description_of(&View::full(), "run_dataset");
    assert!(set_words.contains("A refused row is a run the card refused part way: it has a run seal, the refusal's name in reject and its sentence in reject_detail."), "{set_words}");
    assert!(set_words.contains("An unadmitted row is a request the door turned away before computing: it has no run seal, and its sentence, with the refusal's name in round brackets when the sentence gives one, is its reject_detail; its summary row gives that name in reject."), "{set_words}");
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
