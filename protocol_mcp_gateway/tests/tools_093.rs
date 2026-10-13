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
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const KEY: &str = "tools-093-key";

/// SHA-256 over the full description of every card and then of every solver,
/// in the lists' order, each followed by a line feed. With every description
/// as 0.9.2 gave it the value is ea2229aa...4eb1; 0.9.3 rewords one sentence
/// in the result of `tokamak_dt_beam_target`, which gives ad4403a1...34c3.
/// The value here is of 0.10.0, on kernel 0.9.0 and solvers 0.8.0: the
/// descriptions of `generator_mmrtg_f1`, `fusion_pb11_lhd` and `ablator_slab`
/// changed with their models, and every solver's carries the new version.
const FULL_DESCRIPTIONS: &str = "ba46364cd235557496ec5dfa4f59867fe169f992da57684c4aa865f73c26095e";

/// The one card whose short form is longer than a page: thirty vector fields
/// and a worked run that states all thirty.
const LONGEST_SHORT_CARD: &str = "solar_system";
/// The most characters a card's short form comes to, that card aside.
const SHORT_CARD_CHARS: usize = 4_000;
/// How many of the thirty-one cards have a short form under 2,500 characters.
const SHORT_CARDS_UNDER_2500: usize = 26;

struct Open {
    child: Child,
    port: u16,
}

impl Open {
    fn start() -> Open {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
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
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    (response.status().as_u16(), response.text().await.unwrap())
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
    assert_eq!((cards.len(), solvers.len()), (31, 13));
    let mut all = Sha256::new();
    for (tool, named) in cards.iter().map(|card| ("describe_card", card)).chain(solvers.iter().map(|solver| ("describe_solver", solver))) {
        let plain = text(&mut door, tool, named);
        assert_eq!(text(&mut door, tool, &with_form(named, "full")), plain, "{named}");
        assert!(parsed(&plain).get("form").is_none(), "{named}: the full form names no form");
        all.update(plain.as_bytes());
        all.update(b"\n");
    }
    assert_eq!(hex::encode(all.finalize()), FULL_DESCRIPTIONS);
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
        assert_eq!(
            keys(&short),
            ["domain", "card", "form", "specimen", "against_measurement", "accuracy_against_measurement", "phase_fields", "load_fields",
             "dt_s", "stops", "declared_ends", "continues_with_from", "example_run_trajectory", "full_form"],
            "{card}"
        );
        assert_eq!(short["form"], "short");
        for same in ["domain", "card", "specimen", "example_run_trajectory"] {
            assert_eq!(short[same], full[same], "{card} {same}");
        }
        // The verdict is the word on the card's row of list_cards, and its phrase begins as a match of find_model begins.
        assert_eq!(short["against_measurement"], row["against_measurement"], "{card}");
        let phrase = short["accuracy_against_measurement"].as_str().unwrap();
        let opening = match short["against_measurement"].as_str().unwrap() {
            "met" => "passed the pass marks fixed before the comparison: ",
            "missed" => "failed the pass marks fixed before the comparison: ",
            "met in part" => "passed in part the pass marks fixed before the comparison: ",
            "not held to a measurement" => "not compared with any measurement: ",
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
            closing.ends_with("describe_card with form \"full\", or with no form, has each bound's reason, the validation record, the frame layout and each stop's meaning."),
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
    assert!(sizes.iter().filter(|(_, size)| *size < 2_500).count() >= SHORT_CARDS_UNDER_2500, "{sizes:?}");
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
        assert_eq!(
            keys(&short),
            ["solver", "form", "summary", "against_measurement", "accuracy_against_measurement", "inputs", "returns", "refusals", "examples",
             "example_solve", "full_form"],
            "{solver}"
        );
        assert_eq!(short["form"], "short");
        for same in ["solver", "summary", "example_solve"] {
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
        assert!(short_text.len() < 7_000, "{solver}: {} characters", short_text.len());
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
    assert_eq!(refused(&mut door, "describe_card", &json!({ "domain": "energy", "card": "none", "form": "short" })), "Card not recognized");
    assert_eq!(refused(&mut door, "describe_solver", &json!({ "solver": "none", "form": "short" })), "Solver not recognized");
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
            said(name).ends_with("against_measurement is how the model did against its own measurements, in one of four words, \"met\", \"missed\", \"met in part\" or \"not held to a measurement\": it is accuracy and not fit, and a model that missed is still the served model of its thing."),
            "{name}: {}",
            said(name)
        );
    }
    assert!(said("find_model").contains("against_measurement gives that verdict in one of four words") && said("find_model").contains("it is accuracy and not fit"));
    for name in ["describe_card", "describe_solver"] {
        let text = said(name);
        assert!(text.contains("With form \"short\" the answer is a page, enough to write a first request") && text.contains("Read the full form, the answer when form is left out, when a request is refused or before relying on a result"), "{name}: {text}");
        let form = &Value::Object(tool(name).input_schema.as_ref().clone())["properties"]["form"];
        assert_eq!((form["type"].as_str(), &form["enum"]), (Some("string"), &json!(["short", "full"])), "{name}");
        assert!(!Value::Object(tool(name).input_schema.as_ref().clone())["required"].as_array().unwrap().contains(&json!("form")));
    }
    // The working instructions say it in one sentence, after the three they have always held.
    let instructions = protocol_mcp_gateway::identity::instructions();
    assert!(
        instructions.ends_with("the whole answer need not be sent back. describe_card and describe_solver take form \"short\" for a page that is enough to write a first request; read the full form, the answer when form is left out, when a request is refused or before relying on a result."),
        "{instructions}"
    );
    // Tool texts are read by agents: an agent, never a caller.
    for name in ["list_cards", "list_solvers", "describe_card", "describe_solver", "find_model"] {
        assert!(!said(name).contains("caller"), "{name}");
    }
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
    for (tool, path, rows, count) in [("list_cards", "cards", "cards", 31), ("list_solvers", "solvers", "solvers", 13)] {
        let local = text(&mut stdio, tool, &json!({}));
        assert_eq!(get(&base, path).await, local, "{tool} web");
        assert_eq!(open.tool(tool, &json!({})), (false, local.clone()), "{tool} open");
        let listed = parsed(&local);
        assert_eq!(listed[rows].as_array().unwrap().len(), count);
        for row in listed[rows].as_array().unwrap() {
            assert!(["met", "missed", "met in part", "not held to a measurement"].contains(&row["against_measurement"].as_str().unwrap()), "{row}");
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
    assert_eq!(keyed.tool(KEY, "describe_card", &unseen), (200, true, "Card not recognized".to_string()));
    assert_eq!(keyed.tool(KEY, "describe_solver", &json!({ "solver": "ablator_slab", "form": "short" })), (200, true, "Solver not recognized".to_string()));
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
    assert_eq!(names(&found["matches"], "against_measurement"), [json!("met"), json!("met in part")]);
}
