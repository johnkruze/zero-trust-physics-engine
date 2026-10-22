//! The tool changes of 0.9.3, on every connection: one word of how a model
//! did against its own measurements on every row of the two lists and on
//! every match of find_model, and a short form of a card's and of a solver's
//! description. The full form of every description is byte for byte what
//! 0.9.2 gave, the short form holds no bound, name or number of its own, and
//! every connection gives the same bytes.

mod hosted_support;

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use hosted_support::*;
use hosted_support::answer_text;
use serde_json::{Value, json};
use sha2::Sha256;
use tokio::net::TcpListener;

const KEY: &str = "tools-093-key";

/// The characters (not bytes) of the text list_cards and list_solvers answer (pins.py).
/// Since 0.14.4 a compared model's row is what it computes, the comparison
/// and the gap, and the whole sheet is its description's: 57,331 and 40,406
/// at 0.14.3, 33,440 and 26,111 on words_0144, and 34,309 and 26,241 on
/// integration_0144 (grid_gb_twin's row, and the rows of capsule_stardust and
/// ablator_slab naming the coupling), and 36,079 and 27,559 on the final
/// integration_0144 (the rows of the three twins and the blast twin).
/// 37,041 and 27,658 with runaway_18650_lco_twin and ecore_triga's sheets.
/// 38,280 and 27,658 with reactor_spert3_ccore_plate's row.
/// 38,187 with the JET twin's shorter row. 29,361 and 26,446 in 0.15.0: one
/// model per real object (four card rows and a solver row fewer) and the
/// served words that no longer name an earlier model. 29,442 in 0.15.1: the
/// crater's comparison names the law fitted on all thirteen craters.
const LIST_CARDS_CHARS: usize = 29_442;
const LIST_SOLVERS_CHARS: usize = 26_446;

/// The one card whose short form is longer than a page: thirty vector fields
/// and a worked run that states all thirty.
const LONGEST_SHORT_CARD: &str = "solar_system";
/// The most characters a card's short form comes to, that card aside.
const SHORT_CARD_CHARS: usize = 4_000;

struct Open {
    child: Child,
    port: u16,
}

impl Open {
    fn start() -> Open {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door")).arg("--unsigned")
            .args(["--open", "--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut banner = String::new();
        for _ in 0..3 {
            out.read_line(&mut banner).unwrap();
        }
        let address = banner.split("running on ").nth(1).unwrap().split(' ').next().unwrap();
        let port = address.rsplit(':').next().unwrap().parse().unwrap();
        Open { child, port }
    }

    fn tool(&self, tool: &str, arguments: &Value) -> (bool, String) {
        let reply = parse_reply(&raw(self.port, &request_bytes(None, &tool_body(1, tool, arguments)), 120));
        assert_eq!(reply.status, 200, "{reply:?}");
        tool_result(&reply)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn web() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, protocol_mcp_gateway::app()).await.unwrap();
    });
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, tool: &str, body: &Value) -> (u16, String) {
    let response = reqwest::Client::new()
        .post(format!("{base}/{tool}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(answer_text::as_sent(body))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    answer_text::remember(&text);
    (status, text)
}

async fn get(base: &str, path: &str) -> String {
    reqwest::Client::new().get(format!("{base}/{path}")).send().await.unwrap().text().await.unwrap()
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| panic!("not JSON: {text}"))
}

/// The text of an answer that is not an error.
fn text(door: &mut Stdio2, tool: &str, arguments: &Value) -> String {
    let (error, text) = door.tool(tool, arguments);
    assert!(!error, "{tool} {arguments}: {text}");
    text
}

fn ok(door: &mut Stdio2, tool: &str, arguments: &Value) -> Value {
    parsed(&text(door, tool, arguments))
}

fn refused(door: &mut Stdio2, tool: &str, arguments: &Value) -> String {
    let (error, text) = door.tool(tool, arguments);
    assert!(error, "{tool} answered: {text}");
    text
}

fn with_form(arguments: &Value, form: &str) -> Value {
    let mut arguments = arguments.clone();
    arguments["form"] = json!(form);
    arguments
}

/// The arguments that name each card, and each solver, the local connection lists.
fn served(door: &mut Stdio2) -> (Vec<Value>, Vec<Value>) {
    let cards = ok(door, "list_cards", &json!({}));
    let solvers = ok(door, "list_solvers", &json!({}));
    (
        cards["cards"].as_array().unwrap().iter().map(|row| json!({ "domain": row["domain"], "card": row["card"] })).collect(),
        solvers["solvers"].as_array().unwrap().iter().map(|row| json!({ "solver": row["solver"] })).collect(),
    )
}

fn names(list: &Value, key: &str) -> Vec<Value> {
    list.as_array().unwrap().iter().map(|row| row[key].clone()).collect()
}

fn keys(value: &Value) -> Vec<&str> {
    value.as_object().unwrap().keys().map(String::as_str).collect()
}

#[test]
fn with_no_form_or_full_every_description_is_the_whole_description_as_it_was() {
    let mut door = Stdio2::start(None);
    let (cards, solvers) = served(&mut door);
    assert_eq!((cards.len(), solvers.len()), (protocol_mcp_gateway::CARDS, protocol_mcp_gateway::SOLVERS));
    for (tool, named) in cards.iter().map(|card| ("describe_card", card)).chain(solvers.iter().map(|solver| ("describe_solver", solver))) {
        let plain = text(&mut door, tool, named);
        assert_eq!(text(&mut door, tool, &with_form(named, "full")), plain, "{named}");
        assert!(parsed(&plain).get("form").is_none(), "{named}: the full form names no form");
    }
}

#[test]
fn a_list_row_is_what_a_model_computes_its_comparison_and_its_gap() {
    let mut door = Stdio2::start(None);
    let cards = text(&mut door, "list_cards", &json!({}));
    let solvers = text(&mut door, "list_solvers", &json!({}));
    assert_eq!((cards.chars().count(), solvers.chars().count()), (LIST_CARDS_CHARS, LIST_SOLVERS_CHARS));
    for (tool, rows, line, describe) in [(&cards, "cards", "specimen", "describe_card"), (&solvers, "solvers", "summary", "describe_solver")] {
        for row in parsed(tool)[rows].as_array().unwrap() {
            let said = row[line].as_str().unwrap();
            let mut named = row.clone();
            for key in ["against_measurement", "superseded_by", line] {
                named.as_object_mut().unwrap().remove(key);
            }
            let full = ok(&mut door, describe, &named);
            let sheet = full[line].as_str().unwrap();
            match said.split_once(" Compared with ") {
                // A compared model: the sheet's first, fourth and fifth parts.
                Some((computes, comparison)) if row["against_measurement"] == "compared with measurement" => {
                    assert!(sheet.starts_with(computes), "{row}");
                    assert!(sheet.contains(&format!(" Compared with {comparison} Refuses")), "{row}");
                    assert!(!said.contains(" Range: ") && !said.contains(" Refuses "), "{row}");
                }
                // Any other model: its own sentence, as its description gives it.
                _ => assert_eq!(said, sheet, "{row}"),
            }
        }
    }
}

#[test]
fn a_cards_short_form_holds_the_full_forms_own_names_units_and_bounds() {
    let mut door = Stdio2::start(None);
    let (cards, _) = served(&mut door);
    let listed = ok(&mut door, "list_cards", &json!({}));
    let mut sizes = Vec::new();
    for (card, row) in cards.iter().zip(listed["cards"].as_array().unwrap()) {
        let full = ok(&mut door, "describe_card", card);
        let short_text = text(&mut door, "describe_card", &with_form(card, "short"));
        let short = parsed(&short_text);
        sizes.push((card["card"].as_str().unwrap().to_string(), short_text.len()));
        // A card that gives a worked refusal, the fuse link's since 0.11.0,
        // has it after its worked run.
        let mut expected = vec!["domain", "card", "form", "specimen", "against_measurement", "accuracy_against_measurement", "phase_fields", "load_fields",
             "dt_s", "stops", "declared_ends", "continues_with_from", "example_run_trajectory"];
        // A second form of a served card's object gives, after its evidence,
        // the card it stands beside, as its record does.
        if full["validation"].get("companion_to").is_some() {
            expected.insert(6, "companion_to");
            assert_eq!(short["companion_to"], full["validation"]["companion_to"], "{card}");
        }
        assert!(short.get("not_blind").is_none() && full["validation"].get("not_blind").is_none(), "{card}");
        if card["card"] == "grid_node" {
            expected.push("example_refused");
        }
        expected.push("full_form");
        assert_eq!(keys(&short), expected, "{card}");
        if let Some(refused) = short.get("example_refused") {
            assert_eq!(&full["example_refused"], refused, "{card}");
            let answered = self::refused(&mut door, "run_trajectory", &refused["run_trajectory"]);
            assert_eq!(answered, refused["answered"].as_str().unwrap(), "{card}");
            assert!(answered.ends_with("(TemperatureOutsideCard). That state is not sealed."), "{card}: {answered}");
        }
        assert_eq!(short["form"], "short");
        for same in ["domain", "card", "example_run_trajectory"] {
            assert_eq!(short[same], full[same], "{card} {same}");
        }
        // The spec sheet once: the full form's paragraph is the short form's
        // specimen and accuracy together, in its own order.
        within(&full["specimen"], &short["specimen"], &short["accuracy_against_measurement"], card);
        // The evidence is the word on the card's row of list_cards, and its phrase begins as a match of find_model begins.
        assert_eq!(short["against_measurement"], row["against_measurement"], "{card}");
        let phrase = short["accuracy_against_measurement"].as_str().unwrap();
        let opening = match short["against_measurement"].as_str().unwrap() {
            "compared with measurement" => "Compared with ",
            "company specimen" => "Company specimen: ",
            "held to a reference calculation" => "Held to a reference calculation: ",
            other => panic!("{card}: {other}"),
        };
        assert!(phrase.starts_with(opening), "{card}: {phrase}");
        // Every field: the full form's name, unit, minimum and maximum, and no prose.
        for list in ["phase_fields", "load_fields"] {
            let (short_fields, full_fields) = (short[list].as_array().unwrap(), full[list].as_array().unwrap());
            assert_eq!(short_fields.len(), full_fields.len(), "{card} {list}");
            for (brief, whole) in short_fields.iter().zip(full_fields) {
                for same in ["name", "unit", "minimum", "maximum"] {
                    assert!(brief.get(same).is_some() && brief[same] == whole[same], "{card} {list} {same}: {brief} against {whole}");
                }
                assert_eq!(brief.get("components").is_some(), whole["components"] != json!(1), "{card}: {brief}");
                assert!(brief.get("components").is_none_or(|count| *count == whole["components"]), "{card}: {brief}");
                for end in ["minimum_exclusive", "maximum_exclusive"] {
                    assert_eq!(brief.get(end), (whole[end] == json!(true)).then_some(&json!(true)), "{card} {end}: {brief}");
                }
                assert!(brief.get("bound").is_none() && brief.as_object().unwrap().len() <= 7, "{card}: {brief}");
            }
        }
        assert_eq!(short["dt_s"], json!({ "unit": full["dt_s"]["unit"], "floor_s": full["dt_s"]["floor_s"], "ceiling_s": full["dt_s"]["ceiling_s"] }), "{card}");
        assert_eq!(short["stops"], Value::Array(names(&full["rejects"], "name")), "{card}");
        assert_eq!(short["declared_ends"], Value::Array(names(&full["declared_ends"], "reject")), "{card}");
        assert_eq!(short["continues_with_from"], full["continuation"]["served"], "{card}");
        assert!(short["continues_with_from"].is_boolean(), "{card}");
        let closing = short["full_form"].as_str().unwrap();
        assert!(
            closing.ends_with("describe_card with form \"full\" or none has each bound's reason, the validation record, the frame layout and each stop's meaning."),
            "{closing}"
        );
        // The short form is words about a model: it carries no proof.
        for proof in ["seal", "receipt", "sha256"] {
            assert!(!keys(&short).iter().any(|key| key.contains(proof)), "{card}: {proof}");
        }
        // Its worked run is a request that runs as it is.
        if card["card"] == "thermal" {
            let run = ok(&mut door, "run_trajectory", &short["example_run_trajectory"]);
            assert_eq!(run["outcome"], "completed", "{run}");
        }
    }
    // A page: every card but one under 4,000 characters, and most under 2,500.
    for (card, size) in &sizes {
        assert!(*size < SHORT_CARD_CHARS || card == LONGEST_SHORT_CARD, "{card}: {size} characters");
    }
}

/// A solver's whole description: what describe_solver gives first, with each
/// part it keeps back for asking put in its place.
fn whole_solver(door: &mut Stdio2, solver: &Value) -> Value {
    let mut whole = ok(door, "describe_solver", solver);
    let kept_back: Vec<String> = match whole.get("more") {
        Some(more) => keys(more).into_iter().filter(|name| *name != "how").map(str::to_string).collect(),
        None => Vec::new(),
    };
    for part in kept_back {
        let mut asked = solver.clone();
        asked["part"] = json!(part);
        whole[&part] = ok(door, "describe_solver", &asked)[&part].clone();
    }
    whole
}

#[test]
fn a_solvers_short_form_holds_the_full_forms_own_inputs_names_and_example() {
    let mut door = Stdio2::start(None);
    let (_, solvers) = served(&mut door);
    let listed = ok(&mut door, "list_solvers", &json!({}));
    for (solver, row) in solvers.iter().zip(listed["solvers"].as_array().unwrap()) {
        let full = whole_solver(&mut door, solver);
        let short_text = text(&mut door, "describe_solver", &with_form(solver, "short"));
        let short = parsed(&short_text);
        // A solver that gives a worked refusal, the hot spot's since 0.12.0,
        // has it after its worked example, as a card's short form does.
        let mut expected = vec!["solver", "form", "summary", "against_measurement", "accuracy_against_measurement", "inputs", "returns", "refusals", "examples", "example_solve"];
        // A second form of a served solver's object gives, after its
        // evidence, the solver it stands beside, as its record does.
        if full["validation"].get("companion_to").is_some() {
            expected.insert(5, "companion_to");
            assert_eq!(short["companion_to"], full["validation"]["companion_to"], "{solver}");
        }
        assert!(short.get("not_blind").is_none() && full["validation"].get("not_blind").is_none(), "{solver}");
        if full.get("example_refused").is_some() {
            expected.push("example_refused");
            assert_eq!(short["example_refused"], full["example_refused"], "{solver}");
            let answered = refused(&mut door, "solve", &short["example_refused"]["solve"]);
            assert_eq!(answered, short["example_refused"]["answered"].as_str().unwrap(), "{solver}");
        }
        expected.push("full_form");
        assert_eq!(keys(&short), expected, "{solver}");
        assert_eq!(short["form"], "short");
        within(&full["summary"], &short["summary"], &short["accuracy_against_measurement"], solver);
        for same in ["solver", "example_solve"] {
            assert_eq!(short[same], full[same], "{solver} {same}");
        }
        assert_eq!(short["against_measurement"], row["against_measurement"], "{solver}");
        // Every input: the full form's row, less its prose and its refusal name.
        let (short_inputs, full_inputs) = (short["inputs"].as_array().unwrap(), full["inputs"].as_array().unwrap());
        assert_eq!(short_inputs.len(), full_inputs.len(), "{solver}");
        for (brief, whole) in short_inputs.iter().zip(full_inputs) {
            let mut expected = whole.as_object().unwrap().clone();
            assert!(expected.remove("bound").is_some(), "{whole}");
            expected.remove("refusal");
            assert_eq!(*brief, Value::Object(expected), "{solver}");
        }
        // What it returns, its refusals and its worked examples, by name.
        assert_eq!(short["returns"]["result"], Value::Array(names(&full["returns"]["result"], "name")), "{solver}");
        assert_eq!(short["returns"]["invariants"], Value::Array(names(&full["returns"]["invariants"], "name")), "{solver}");
        assert_eq!(keys(&short["returns"]), ["result", "invariants"]);
        assert_eq!(short["refusals"], Value::Array(names(&full["refusals"], "name")), "{solver}");
        let examples: Vec<&str> = keys(&full).into_iter().filter(|key| key.starts_with("example_solve")).collect();
        assert_eq!(short["examples"], json!(examples), "{solver}");
        assert!(short["full_form"].as_str().unwrap().contains("describe_solver with form \"full\", or with no form, has each input's bound in words"), "{solver}");
        for proof in ["seal", "receipt", "sha256"] {
            assert!(!keys(&short).iter().any(|key| key.contains(proof)), "{solver}: {proof}");
        }
        // Under 7,500 since 0.14.3: the spec sheet is in the short form, and
        // ablator_slab's, the longest, is 7,310.
        assert!(short_text.len() < 7_500, "{solver}: {} characters", short_text.len());
    }
    // The worked example runs as it is, and by its name.
    let short = ok(&mut door, "describe_solver", &json!({ "solver": "atom_dft", "form": "short" }));
    let solved = ok(&mut door, "solve", &short["example_solve"]);
    let by_name = ok(&mut door, "solve", &json!({ "solver": "atom_dft", "example": short["examples"][0] }));
    assert_eq!((solved["outcome"].as_str(), &solved["solution_seal"]), (Some("converged"), &by_name["solution_seal"]));
}

#[test]
fn a_form_that_is_neither_short_nor_full_is_refused_by_name() {
    let mut door = Stdio2::start(None);
    let plate = json!({ "domain": "energy", "card": "thermal" });
    let atom = json!({ "solver": "atom_dft" });
    assert_eq!(
        refused(&mut door, "describe_card", &with_form(&plate, "brief")),
        "Physical Violation: 'form' must be one of \"short\", \"full\"; this request has \"brief\"."
    );
    assert_eq!(
        refused(&mut door, "describe_solver", &with_form(&atom, "Short")),
        "Physical Violation: 'form' must be one of \"short\", \"full\"; this request has \"Short\"."
    );
    let mut numbered = plate.clone();
    numbered["form"] = json!(1);
    assert_eq!(refused(&mut door, "describe_card", &numbered), "Physical Violation: 'form' must be one of \"short\", \"full\"; this request has 1.");
    let mut empty = atom.clone();
    empty["form"] = json!(null);
    assert_eq!(refused(&mut door, "describe_solver", &empty), "Physical Violation: 'form' must be one of \"short\", \"full\"; this request has null.");
    // A card that is not served is not recognized in either form, and another key is still named.
    assert_eq!(refused(&mut door, "describe_card", &json!({ "domain": "energy", "card": "none", "form": "short" })), "Card not recognized (UnknownCard).");
    assert_eq!(refused(&mut door, "describe_solver", &json!({ "solver": "none", "form": "short" })), "Solver not recognized (UnknownSolver).");
    assert_eq!(
        refused(&mut door, "describe_card", &json!({ "domain": "energy", "card": "thermal", "forms": "short" })),
        "Physical Violation: 'forms' is not a field of this request."
    );
    // A part of a solver's description is asked for alone, or with the full form.
    let mut part = json!({ "solver": "ablator_slab", "part": "refusals" });
    assert_eq!(
        refused(&mut door, "describe_solver", &with_form(&part, "short")),
        "Physical Violation: 'form' \"short\" and 'part' each ask for less than the whole description; send one of them."
    );
    let alone = text(&mut door, "describe_solver", &part);
    part["form"] = json!("full");
    assert_eq!(text(&mut door, "describe_solver", &part), alone);
}

#[test]
fn the_tools_say_what_the_word_is_and_that_the_short_form_exists() {
    let tools = protocol_mcp_gateway::mcp_server::tools_in(&protocol_mcp_gateway::view::View::full());
    let tool = |name: &str| tools.iter().find(|tool| tool.name == name).unwrap();
    let said = |name: &str| tool(name).description.as_deref().unwrap_or("").to_string();
    for name in ["list_cards", "list_solvers"] {
        assert!(
            said(name).ends_with("against_measurement names the evidence behind the model, in one of three words: \"compared with measurement\" (a real object; its description says what it was fitted on, what it was compared with outside the fit and within what, the gap, and where it refuses), \"held to a reference calculation\" (compared with a published calculation), or \"company specimen\" (an object the company specified, every constant stated). The numbers say how good it is."),
            "{name}: {}",
            said(name)
        );
    }
    for word in ["\"compared with measurement\"", "\"held to a reference calculation\"", "\"company specimen\""] {
        assert!(said("find_model").contains(word), "{word}");
    }
    assert!(said("find_model").contains("Fit and accuracy are separate: a model with a gap is still the served model of its thing."));
    for name in ["describe_card", "describe_solver"] {
        let text = said(name);
        // Since 0.10.1 the second sentence says what is so of the full form, where it told the agent to read it.
        assert!(text.contains("With form \"short\" the answer is a page, enough to write a first request") && text.contains("The full form, the answer when form is left out, adds each bound's reason, the validation record and each stop's meaning."), "{name}: {text}");
        let form = &Value::Object(tool(name).input_schema.as_ref().clone())["properties"]["form"];
        assert_eq!((form["type"].as_str(), &form["enum"]), (Some("string"), &json!(["short", "full"])), "{name}");
        assert!(!Value::Object(tool(name).input_schema.as_ref().clone())["required"].as_array().unwrap().contains(&json!("form")));
    }
    // The instructions said it in one sentence from 0.10.1 to 0.14.1; since
    // 0.14.2 they say only that the short form comes before a first request,
    // and which three tools verify the proof of a run, a solution or a dataset.
    let instructions = protocol_mcp_gateway::identity::instructions();
    assert!(
        instructions.contains("read describe_card or describe_solver (form \"short\") before a first request.")
            && instructions.ends_with("verify_tape, verify_solution and verify_dataset verify the proof of a run, a solution or a dataset; cite gives sources."),
        "{instructions}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn every_connection_gives_the_same_bytes_for_the_lists_the_matches_and_the_short_forms() {
    let mut stdio = Stdio2::start(None);
    let base = web().await;
    let keyed = Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY),
            "cards": [["energy", "thermal"], ["energy", "battery_sag"], ["energy", "grid_gb"]],
            "solvers": ["atom_dft"], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let open = Open::start();
    // The two lists: every row with its word, the same bytes where every model is seen.
    for (tool, path, rows, count) in [("list_cards", "cards", "cards", protocol_mcp_gateway::CARDS), ("list_solvers", "solvers", "solvers", protocol_mcp_gateway::SOLVERS)] {
        let local = text(&mut stdio, tool, &json!({}));
        assert_eq!(get(&base, path).await, local, "{tool} web");
        assert_eq!(open.tool(tool, &json!({})), (false, local.clone()), "{tool} open");
        let listed = parsed(&local);
        assert_eq!(listed[rows].as_array().unwrap().len(), count);
        for row in listed[rows].as_array().unwrap() {
            assert!(["compared with measurement", "held to a reference calculation", "company specimen"].contains(&row["against_measurement"].as_str().unwrap()), "{row}");
        }
        // A key sees its own rows, each as the others see it.
        let (status, error, through_key) = keyed.tool(KEY, tool, &json!({}));
        assert_eq!((status, error), (200, false));
        for row in parsed(&through_key)[rows].as_array().unwrap() {
            assert!(listed[rows].as_array().unwrap().contains(row), "{row}");
        }
    }
    // The descriptions in both forms, a form refused by name, and a match with its word.
    let calls = [
        ("describe_card", json!({ "domain": "energy", "card": "thermal", "form": "short" }), false),
        ("describe_card", json!({ "domain": "energy", "card": "grid_gb", "form": "short" }), false),
        ("describe_card", json!({ "domain": "energy", "card": "grid_gb", "form": "full" }), false),
        ("describe_card", json!({ "domain": "energy", "card": "thermal", "form": "page" }), true),
        ("describe_solver", json!({ "solver": "atom_dft", "form": "short" }), false),
        ("describe_solver", json!({ "solver": "atom_dft", "form": "full" }), false),
        ("describe_solver", json!({ "solver": "atom_dft", "form": 2 }), true),
        ("describe_solver", json!({ "solver": "atom_dft", "form": "short", "part": "refusals" }), true),
    ];
    for (tool, arguments, error_due) in &calls {
        let (error, local) = stdio.tool(tool, arguments);
        assert_eq!(error, *error_due, "{tool}: {local}");
        let (status, over_web) = post(&base, tool, arguments).await;
        assert_eq!((status == 200, over_web.as_str()), (!error_due, local.as_str()), "{tool} web {status}");
        let (status, hosted_error, hosted) = keyed.tool(KEY, tool, arguments);
        assert_eq!((status, hosted_error, hosted.as_str()), (200, *error_due, local.as_str()), "{tool} hosted");
        let (open_error, opened) = open.tool(tool, arguments);
        assert_eq!((open_error, opened.as_str()), (*error_due, local.as_str()), "{tool} open");
    }
    // A key that does not see a card is answered as for a card that does not exist, in the short form too.
    let unseen = json!({ "domain": "aerospace", "card": "solar_system", "form": "short" });
    assert_eq!(keyed.tool(KEY, "describe_card", &unseen), (200, true, "Card not recognized (UnknownCard).".to_string()));
    assert_eq!(keyed.tool(KEY, "describe_solver", &json!({ "solver": "ablator_slab", "form": "short" })), (200, true, "Solver not recognized (UnknownSolver).".to_string()));
    // A match of find_model carries the word of its row of the list.
    let arguments = json!({ "description": "a capsule entering an atmosphere" });
    let local = text(&mut stdio, "find_model", &arguments);
    assert_eq!(post(&base, "find_model", &arguments).await, (200, local.clone()));
    assert_eq!(open.tool("find_model", &arguments), (false, local.clone()));
    let found = parsed(&local);
    let cards = ok(&mut stdio, "list_cards", &json!({}));
    assert_eq!(names(&found["matches"], "card"), [json!("capsule_stardust"), json!("capsule_insight")]);
    for row in found["matches"].as_array().unwrap() {
        let listed = cards["cards"].as_array().unwrap().iter().find(|card| card["card"] == row["card"]).unwrap();
        assert_eq!(row["against_measurement"], listed["against_measurement"], "{row}");
    }
    assert_eq!(names(&found["matches"], "against_measurement"), [json!("compared with measurement"), json!("compared with measurement")]);
}

/// The short form says a description's one sentence once: its specimen (or
/// summary) is the full form's where the model has no spec sheet; where it
/// has one, the full paragraph is made of the short form's specimen (what
/// the model computes), its accuracy, and, left out of the short form since
/// 0.14.4, its range (find_model's model_of), what it was fitted on
/// (find_model's compared_with) and the refusal sentence (whose names are its
/// stops).
fn within(full: &Value, short: &Value, accuracy: &Value, model: &Value) {
    let (full, short) = (full.as_str().unwrap(), short.as_str().unwrap());
    if full == short {
        return;
    }
    let accuracy = accuracy.as_str().unwrap();
    assert!(full.contains(" Gap: ") && accuracy.starts_with("Compared with ") && full.contains(accuracy), "{model}: {accuracy}");
    let rest = full.replacen(accuracy, "", 1);
    let rest: Vec<&str> = rest.split_whitespace().collect();
    let short: Vec<&str> = short.split_whitespace().collect();
    assert!(!short.contains(&"Range:") && !short.contains(&"Refuses"), "{model}: {short:?}");
    assert_eq!(&rest[..short.len()], short.as_slice(), "{model}");
    let left_out = rest[short.len()..].join(" ");
    assert!(left_out.starts_with("Range: ") && (left_out.contains(" Fitted on ") || left_out.contains(" Nothing fitted")) && left_out.contains(" Refuses "), "{model}: {left_out}");
}
