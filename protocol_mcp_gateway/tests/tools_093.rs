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
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const KEY: &str = "tools-093-key";

/// SHA-256 over the full description of every card and then of every solver,
/// in the lists' order, each followed by a line feed. With every description
/// as 0.9.2 gave it the value is ea2229aa...4eb1; 0.9.3 rewords one sentence
/// in the result of `tokamak_dt_beam_target`, which gives ad4403a1...34c3.
/// On 0.10.0, kernel 0.9.0 and solvers 0.8.0, the value was ba46364c...095e:
/// the descriptions of `generator_mmrtg_f1`, `fusion_pb11_lhd` and
/// `ablator_slab` changed with their models, and every solver's carries the
/// new version. The value here is of 0.10.1, re-pinned because the
/// descriptions changed in words only: every card lists the gateway's own
/// refusal names after its kernel's (the two of a step outside the floor and
/// the ceiling, and the names of a continuation where the card gives them),
/// three solvers list the names their entries give at the door, and the
/// facts corrected in nine descriptions read as their models compute
/// (165f3d40...0e35 on the reactor build). Recomputed once at the merge of
/// the engine after 0.10.1 (kernel 0.10.0, solvers 0.9.0, gateway 0.11.0):
/// every compared-range wall lifted with its sentence, the three fusion
/// solvers reopened, the company's own specimens named so, describe_card's
/// example_compute_step, "compared" where the invariants said "checked", no
/// description saying "check", the accumulator rated at 330 bar, the tokamak
/// solver's beams to 1000 keV, and every solver carrying the new version.
/// Recomputed once at the merge of engine 0.12.0 (kernel 0.11.0, solvers
/// 0.10.0, gateway 0.12.0), and reproduced with Python's hashlib from the
/// stdio door by this rule, which gives 0.11.0's cdd6f753...ddff from its
/// binary: thirty-five cards and twenty-one solvers, each solver's
/// open_connection, the own-specimen solver record, the signed load's words.
/// Set once more at the end of 0.12.0's fix pass, and reproduced with
/// Python's hashlib from the stdio door by this rule (which gives the pin
/// above, 09fe2bae...4fec, from the frozen 0.12.0 binary): twenty-two
/// solvers with plasma_sheath_ram_c2, the hot spot's worked refusal and
/// re-evaluation, the kit's twenty-five findings worded or lifted, the red
/// team's defects of the new models, and the long form computing again.
/// Set once more on 0.12.0's final tree, and reproduced with Python's hashlib
/// from the stdio door by this rule (which gives the pin above,
/// 9ad1d4b5...57e8be, from the fix pass's binary): values_hold beside holds
/// in every long form's answer, and cantilever_eth served to 18.75 mm with
/// FLAG_ENVELOPE_ABOVE_COMPARED.
/// Set on the merged tree of next-0.13, and reproduced with Python's hashlib
/// from the stdio door by this rule (which gives 9eb2fb7b...81e6, 0.12.0's
/// pin, from the served 0.12.0 binary): thirty-six cards with
/// momentum_jwst_dihedral, twenty-three solvers with culture_trichoderma_atp,
/// and the two couplings of cell m80, with their conditions, listed by
/// cell_2170_m80 and cell_2170_model3.
/// Set again with generator_mmrtg_seebeck, the companion of
/// generator_mmrtg_f1 and the first card calibrated to the record, its
/// companion_to and not_blind in its record, thirty-seven cards; reproduced
/// with Python's hashlib from the stdio door of the cargo build --release
/// binary by this rule (which gives e9bd3218...794d, next-0.13's pin, from
/// the integration's binary).
/// Set on feat/companion-stress-plate-elastic after it, reproduced the same
/// way from the stdio door of that branch's cargo build --release binary
/// (from df8fed0b...e8a9, next-0.13's pin before it): twenty-four solvers
/// with stress_plate_hole_s235_elastic, the companion of
/// stress_plate_hole_s235, calibrated to the record.
/// Set for the 0.13.0 candidate, from c615481c...20ce, next-0.13's pin, by
/// the same rule from the stdio door of the final tree's cargo build
/// --release binary: genesis_solvers_v0.11.0, which every solver's
/// description names as its engine_version, and the cell m80 couplings'
/// fifth condition, the hour of rest at the 40 degC block's case
/// temperatures, and their meanings, listed by both cell cards. The same rule gives 9eb2fb7b...81e6 from the served 0.12.0
/// binary (evidence/verification/engine_0.12.0_served/mcp_stdio).
/// Set for 0.13.1, from dafcf9f0...bccd, 0.13.0's pin, by the same rule from
/// the stdio door of feat/words-0131's cargo build --release binary: the
/// sentences the records contradict mended, the eighteen comparisons that are
/// not blind saying so, accuracy_against_measurement in every short form
/// saying how the model did against the pass marks its record states, and
/// the words of capsule_insight, cell_2170_m80 and cell_2170_model3. The same
/// rule gives dafcf9f0...bccd from the frozen 0.13.0 rc2 binary
/// (evidence/verification/frozen_0.13.0_rc2/mcp_stdio).
/// Set for genesis_solvers 0.12.0, canonical math, from dafcf9f0...bccd,
/// next-0.13's pin, by the same rule from the stdio door of
/// feat/canonical-math-014's cargo build --release binary
/// (measure_014/scripts/full_descriptions.py, which gives dafcf9f0...bccd
/// from the frozen 0.13.0 candidate's): genesis_solvers_v0.12.0 in every
/// solver's description, and blast_wave_tnt_air's locked numbers as
/// genesis_kernel::math computes them, sedov_taylor_alpha
/// 0.8510719009049513, sedov_taylor_xi0 1.0327774565615964,
/// least_shock_reach_sachs 0.5150689976238669 and so the largest charge,
/// 106342834.30638722 kg, its bound and charge_mass_max_kg (4596f66b...d386);
/// then, by the same rule, the open connection's times measured again with
/// canonical math, in the descriptions of the seven solvers whose row moved.
/// Set for next-0.14, the merge of the two, from 1887be20...7371 (0.13.1)
/// and 9e9d042f...a156 (feat/canonical-math-014), by the same rule from the
/// stdio door of the merged tree's cargo build --release binary
/// (evidence/verification/next_engine_version_2026-10-15/pins.py, which gives
/// both from their own binaries): 0.13.1's words with canonical math's
/// version, numbers and open connection's times.
/// Set for the words of 0.14.0, from 1887be20...7371, 0.13.1's pin, by the
/// same rule from the stdio door of feat/words-014's cargo build --release
/// binary: the two JWST momentum cards' later blind months, satellite_lageos1's
/// frame and what a crust-fixed start costs through the engine, the SPERT III
/// couplings' CouplingSourceNotOneValue, and every solver's open_connection
/// (one solve counted above 15 s admitted alone; what one inline answer holds
/// of five solvers' worked solves). The same rule gives 1887be20...7371 from
/// the frozen 0.13.1 binary (evidence/verification/frozen_0.13.1/mcp_stdio;
/// both runs in evidence/verification/words_014_2026-10-16/).
/// Set for 0.14.0 with its words, the merge of next-0.14 and feat/words-014,
/// from 6e66505a...b2f1 (next-0.14) and 7922d9c7...92bb (feat/words-014),
/// by the same rule from the stdio door of the merged tree's cargo build
/// --release binary (evidence/verification/next_engine_version_2026-10-15/
/// pins.py, which gives both from their own binaries): the words of 0.14.0
/// with canonical math's version, numbers and open connection's times.
/// Then, by the same rule, water_hammer_oil_line's open_connection saying
/// one inline answer holds 31 of its worked solve, as canonical math's
/// answers measure it, where the words said 32.
/// Set for fix/breadth-014, the fix pass of 0.14.0, from 6e66505a...b2f1 by
/// the same rule from the fix copy's cargo build --release binary
/// (fix_014/measure_fix/pins.txt): the descriptions its fixes change, the
/// crater's continuation statement, rejects and load rule, capsule_stardust's
/// load rule, ablator_slab's open connection, the two culture solvers'
/// feed_ml and water_ml, and the two MMRTG cards' time label.
/// Set for 0.14.0 release candidate 1, the merge of next-0.14 with its words
/// and fix/breadth-014, from c9ef8bd2...525e (next-0.14) and 080ee048...6dd9
/// (fix/breadth-014), by the same rule from the stdio door of the merged
/// tree's cargo build --release binary (evidence/verification/
/// next_engine_version_2026-10-15/pins.py, which gives both from their own
/// binaries; integration_014/measure_014rc1/): the words of 0.14.0 and the
/// fix pass's descriptions together, ablator_slab's open_connection in the
/// words' sentence at the fix's 450 s and one a plan.
/// Set for 0.14.0 release candidate 2, from 40f4a2d6...94cf by the same rule
/// from the stdio door of rc2's cargo build --release binary
/// (rc2_014/measure_rc2/pins_item11.txt): fifteen solvers' open_connection,
/// the five that said an inline answer is "counted from the plan" saying it
/// is counted once the plan is solved (item 5), and ten more saying how many
/// of their worked solve one inline answer holds where the local connection
/// admits the engine's own 64 (item 6). No card's description moves.
/// Then, by the same rule (rc2_014/measure_rc2/pins_item15.txt), from
/// 1dc5ca06...f820: generator_mmrtg_seebeck's specimen, result and pass line
/// say its forecast was registered before any of its days' values was
/// published as a number, and the pass line names the protocol's four tiers
/// (item 15).
const FULL_DESCRIPTIONS: &str = "ebf5814ded225cc00aba9662f4b0aa6a8d4e16fc537290254983f6291be8021b";

/// The one card whose short form is longer than a page: thirty vector fields
/// and a worked run that states all thirty.
const LONGEST_SHORT_CARD: &str = "solar_system";
/// The most characters a card's short form comes to, that card aside.
const SHORT_CARD_CHARS: usize = 4_000;
/// How many of the thirty-one cards have a short form under 2,500 characters.
// 25 since 0.11.0: the fuse link's short form carries a loaded worked run
// and a worked refusal, and is 2,875 characters.
const SHORT_CARDS_UNDER_2500: usize = 25;

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
    assert_eq!((cards.len(), solvers.len()), (37, 24));
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
        // A card that gives a worked refusal, the fuse link's since 0.11.0,
        // has it after its worked run.
        let mut expected = vec!["domain", "card", "form", "specimen", "against_measurement", "accuracy_against_measurement", "phase_fields", "load_fields",
             "dt_s", "stops", "declared_ends", "continues_with_from", "example_run_trajectory"];
        // A card calibrated to the record gives, after its word, the card it
        // stands beside and what of the record was seen first, as its record does.
        if full["validation"].get("companion_to").is_some() {
            expected.insert(6, "not_blind");
            expected.insert(6, "companion_to");
            for key in ["companion_to", "not_blind"] {
                assert_eq!(short[key], full["validation"][key], "{card} {key}");
            }
        }
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
        for same in ["domain", "card", "specimen", "example_run_trajectory"] {
            assert_eq!(short[same], full[same], "{card} {same}");
        }
        // The verdict is the word on the card's row of list_cards, and its phrase begins as a match of find_model begins.
        assert_eq!(short["against_measurement"], row["against_measurement"], "{card}");
        let phrase = short["accuracy_against_measurement"].as_str().unwrap();
        let opening = match short["against_measurement"].as_str().unwrap() {
            "met" => "passed the pass marks its record states: ",
            "missed" => "failed the pass marks its record states: ",
            "met in part" => "passed in part the pass marks its record states: ",
            "not held to a measurement" => "the company's own specimen: ",
            "held to a reference calculation" => "held to a reference calculation, not compared with a measurement: ",
            "calibrated to the record" => "calibrated to the record, not a blind test: ",
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
        // A solver that gives a worked refusal, the hot spot's since 0.12.0,
        // has it after its worked example, as a card's short form does.
        let mut expected = vec!["solver", "form", "summary", "against_measurement", "accuracy_against_measurement", "inputs", "returns", "refusals", "examples", "example_solve"];
        // A solver calibrated to the record gives, after its word, the solver
        // it stands beside and what of the record was seen first, as its record does.
        if full["validation"].get("companion_to").is_some() {
            expected.insert(5, "not_blind");
            expected.insert(5, "companion_to");
            for key in ["companion_to", "not_blind"] {
                assert_eq!(short[key], full["validation"][key], "{solver} {key}");
            }
        }
        if full.get("example_refused").is_some() {
            expected.push("example_refused");
            assert_eq!(short["example_refused"], full["example_refused"], "{solver}");
            let answered = refused(&mut door, "solve", &short["example_refused"]["solve"]);
            assert_eq!(answered, short["example_refused"]["answered"].as_str().unwrap(), "{solver}");
        }
        expected.push("full_form");
        assert_eq!(keys(&short), expected, "{solver}");
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
            said(name).ends_with("against_measurement says in one of six words how the model stands against measurement. \"met\", \"met in part\" and \"missed\" say how it did against the pass marks its record states, and its validation record, in describe_card or describe_solver, says whether that comparison was blind and, where it was not, what was seen and when. \"calibrated to the record\" is a companion whose structure or constants were chosen with the record of a scored original open, so its comparison is not a blind test; the original is served beside it under its own word. \"held to a reference calculation\" is compared with a published calculation, and \"not held to a measurement\" is the company's own specimen. It is accuracy and not fit, and a model that missed is still the served model of its thing."),
            "{name}: {}",
            said(name)
        );
    }
    assert!(said("find_model").contains("against_measurement gives that verdict in one of six words") && said("find_model").contains("It is accuracy and not fit"));
    assert!(said("find_model").contains("\"met\", \"met in part\" and \"missed\" say how the model did against the pass marks its record states, and its validation record says whether that comparison was blind and, where it was not, what was seen and when"));
    assert!(said("find_model").contains("\"calibrated to the record\" is a companion whose structure or constants were chosen with the record of a scored original open, so its comparison is not a blind test, and the original is served beside it under its own word"));
    // No tool says that the three words name blind tests.
    for name in ["list_cards", "list_solvers", "find_model"] {
        assert!(!said(name).contains("compared blind") && !said(name).contains("fixed before"), "{name}: {}", said(name));
    }
    for name in ["describe_card", "describe_solver"] {
        let text = said(name);
        // Since 0.10.1 the second sentence says what is so of the full form, where it told the agent to read it.
        assert!(text.contains("With form \"short\" the answer is a page, enough to write a first request") && text.contains("Only the full form, the answer when form is left out, has each bound's reason, the validation record and each stop's meaning: it is the form that explains a refusal and the form a result is relied on with."), "{name}: {text}");
        let form = &Value::Object(tool(name).input_schema.as_ref().clone())["properties"]["form"];
        assert_eq!((form["type"].as_str(), &form["enum"]), (Some("string"), &json!(["short", "full"])), "{name}");
        assert!(!Value::Object(tool(name).input_schema.as_ref().clone())["required"].as_array().unwrap().contains(&json!("form")));
    }
    // The instructions said it in one sentence from 0.10.1 to 0.14.1; since
    // 0.14.2 they say only that the short form comes before a first request,
    // and which three tools check the proof of a run, a solution or a dataset.
    let instructions = protocol_mcp_gateway::identity::instructions();
    assert!(
        instructions.contains("read describe_card or describe_solver (form \"short\") before a first request.")
            && instructions.ends_with("verify_tape, verify_solution and verify_dataset check the proof of a run, a solution or a dataset; cite gives sources."),
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
    for (tool, path, rows, count) in [("list_cards", "cards", "cards", 37), ("list_solvers", "solvers", "solvers", 24)] {
        let local = text(&mut stdio, tool, &json!({}));
        assert_eq!(get(&base, path).await, local, "{tool} web");
        assert_eq!(open.tool(tool, &json!({})), (false, local.clone()), "{tool} open");
        let listed = parsed(&local);
        assert_eq!(listed[rows].as_array().unwrap().len(), count);
        for row in listed[rows].as_array().unwrap() {
            assert!(["met", "missed", "met in part", "calibrated to the record", "held to a reference calculation", "not held to a measurement"].contains(&row["against_measurement"].as_str().unwrap()), "{row}");
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
    assert_eq!(names(&found["matches"], "against_measurement"), [json!("met"), json!("met in part")]);
}
