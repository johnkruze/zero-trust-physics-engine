//! The three tools added after 0.8.1, on every connection: cite, find_model
//! and find_flip. Every connection gives the same bytes. Nothing is kept by
//! any connection, and a stop before computing is the one sentence it was in
//! 0.8.1 with nothing beside it. A proof printed before these tools comes
//! back identical: a replay of the printed proofs holds that against the
//! binary; here the answers of the thirteen earlier tools are held to carry
//! no new key.

mod hosted_support;

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use hosted_support::*;
use serde_json::{Value, json};
use tokio::net::TcpListener;

const KEY: &str = "tools-090-key";

struct Open {
    child: Child,
    port: u16,
}

impl Open {
    fn start_with(extra: &[&str]) -> Open {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .args(["--open", "--port", "0"])
            .args(extra)
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

    fn start() -> Open {
        Open::start_with(&[])
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
    let path = if tool == "verify_step" { "verify_physics" } else { tool };
    let response = reqwest::Client::new()
        .post(format!("{base}/{path}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    (response.status().as_u16(), response.text().await.unwrap())
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| panic!("not JSON: {text}"))
}

fn ok(door: &mut Stdio2, tool: &str, arguments: &Value) -> Value {
    let (error, text) = door.tool(tool, arguments);
    assert!(!error, "{tool}: {text}");
    parsed(&text)
}

fn refused(door: &mut Stdio2, tool: &str, arguments: &Value) -> String {
    let (error, text) = door.tool(tool, arguments);
    assert!(error, "{tool} answered: {text}");
    text
}

fn keyed() -> Hosted {
    Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY),
            "cards": [["energy", "thermal"], ["energy", "battery_sag"], ["energy", "grid_gb"]],
            "solvers": ["atom_dft"], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    )
}

/// Each call on the local connection, the web connection, the hosted
/// connection with a key and the hosted connection without one.
async fn same_on_every_connection(calls: &[(&str, Value, bool)]) {
    let mut stdio = Stdio2::start(None);
    let base = web().await;
    let keyed = keyed();
    let open = Open::start();
    for (tool, arguments, error_due) in calls {
        let (error, local) = stdio.tool(tool, arguments);
        assert_eq!(error, *error_due, "{tool}: {local}");
        let (status, over_web) = post(&base, tool, arguments).await;
        assert_eq!(over_web, local, "{tool} web");
        assert_eq!(status == 200, !error_due, "{tool} web status {status}");
        let (status, hosted_error, hosted) = keyed.tool(KEY, tool, arguments);
        // find_model says how much it compared the description with: what
        // this key sees, three cards and one solver, where the others see all.
        let through_key = local
            .replace("\"searched\":{\"cards\":40,\"solvers\":25}", "\"searched\":{\"cards\":3,\"solvers\":1}")
            .replace("(40 cards and 25 solvers)", "(3 cards and 1 solver)");
        assert_eq!((status, hosted_error, hosted.as_str()), (200, *error_due, through_key.as_str()), "{tool} hosted");
        let (open_error, opened) = open.tool(tool, arguments);
        assert_eq!((open_error, opened.as_str()), (*error_due, local.as_str()), "{tool} open");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cite_and_find_model_give_the_same_bytes_on_every_connection() {
    same_on_every_connection(&[
        ("cite", json!({ "domain": "energy", "card": "grid_gb" }), false),
        ("cite", json!({ "domain": "energy", "card": "battery_sag" }), false),
        ("cite", json!({ "solver": "atom_dft" }), false),
        ("cite", json!({ "domain": "energy", "card": "no_such_card" }), true),
        ("cite", json!({}), false),
        ("find_model", json!({ "description": "power grid frequency after losing a link" }), false),
        ("find_model", json!({ "description": "the price of bitcoin tomorrow" }), false),
        ("find_model", json!({ "description": 7 }), true),
    ])
    .await;
}

#[test]
fn cite_gives_the_service_and_the_paper_first_then_the_people_whose_measurements_a_model_stands_on() {
    let mut door = Stdio2::start(None);
    let grid = ok(&mut door, "cite", &json!({ "domain": "energy", "card": "grid_gb" }));
    let data = grid["data"].as_array().unwrap();
    assert!(!data.is_empty());
    assert_eq!(data[0]["authors"], "National Energy System Operator");
    // No licence and no word on passing data on: a citation says who to cite and where their work is.
    assert!(data[0].get("licence").is_none() && data[0].get("may_redistribute_measurements").is_none());
    assert_eq!(data[0]["requested_acknowledgment"], "Supported by National Energy SO Open Data");
    assert_eq!(grid["paper"]["url"], "https://doi.org/10.5281/zenodo.23305697");
    let text = grid["text"].as_str().unwrap();
    let bibtex = grid["bibtex"].as_str().unwrap();
    assert!(bibtex.starts_with("@misc{zerotrustphysics_service,") && bibtex.contains("@misc{kruze2026zerotrustphysics,") && bibtex.contains("@misc{neso2026systemfrequency,"));
    // First the service itself, by name, with its maker and its address; then the paper; then the sources in their order.
    let lines: Vec<&str> = text.split("\n\n").collect();
    assert_eq!(lines[1], grid["paper"]["citation_text"].as_str().unwrap());
    assert!(lines[2].starts_with("National Energy System Operator (2026). System Frequency"));
    assert_eq!(lines.len(), data.len() + 2);
    let keys: Vec<&str> = grid.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["domain", "card", "service", "paper", "real_thing", "how_used", "data", "text", "bibtex", "about"]);
    assert_eq!(lines[0], "John Kruze (Protocol Company). Zero-Trust Physics: a physics verifier for Super Intelligence. Served at https://mcp.zerotrustphysics.com/mcp, engine genesis_kernel_v0.12.0. Accessed [the date you asked].");
    assert!(bibtex.contains("@misc{zerotrustphysics_service,"));
    // A generic specimen stands on no measured data: the service and the paper alone, and the answer says so.
    let pack = ok(&mut door, "cite", &json!({ "domain": "energy", "card": "battery_sag" }));
    assert_eq!(pack["data"], json!([]));
    assert!(pack["how_used"].as_str().unwrap().contains("No measured data"));
    assert_eq!(pack["text"], json!(format!("{}\n\n{}", pack["service"]["citation_text"].as_str().unwrap(), pack["paper"]["citation_text"].as_str().unwrap())));
    // The company's own records of a reference are not served.
    for answer in [&grid, &pack] {
        let whole = answer.to_string();
        assert!(!whole.contains("author_contact") && !whole.contains("Gmail") && !whole.contains("\"verified\""), "{whole}");
    }
    // Every served card and solver has its entry.
    let cards = ok(&mut door, "list_cards", &json!({}));
    for card in cards["cards"].as_array().unwrap() {
        let answer = ok(&mut door, "cite", &json!({ "domain": card["domain"], "card": card["card"] }));
        assert!(answer["bibtex"].as_str().unwrap().contains("kruze2026zerotrustphysics"), "{card}");
    }
    let solvers = ok(&mut door, "list_solvers", &json!({}));
    for solver in solvers["solvers"].as_array().unwrap() {
        let answer = ok(&mut door, "cite", &json!({ "solver": solver["solver"] }));
        assert!(answer["text"].as_str().unwrap().contains("zenodo.23305697"), "{solver}");
    }
    // A solver's answers carry the solvers' engine version, and its citation of the service says that one.
    let atom = ok(&mut door, "cite", &json!({ "solver": "atom_dft" }));
    assert_eq!(atom["service"]["engine_version"], "genesis_solvers_v0.12.0");
    // With no name: how to cite the service itself: its name, who made it,
    // where it is served, and the paper.
    let service = ok(&mut door, "cite", &json!({}));
    assert_eq!(service["service"]["name"], "Zero-Trust Physics");
    assert_eq!(service["service"]["maker"], "John Kruze");
    assert_eq!(service["service"]["publisher"], "Protocol Company");
    assert_eq!(service["service"]["address"], "https://mcp.zerotrustphysics.com/mcp");
    assert_eq!(service["service"]["site"], "https://zerotrustphysics.com");
    assert_eq!(service["service"]["paper"], "https://doi.org/10.5281/zenodo.23305697");
    assert_eq!(service["service"]["contact"], "kruze@zerotrustphysics.com");
    assert_eq!(service["service"]["engine_version"], "genesis_kernel_v0.12.0 and genesis_solvers_v0.12.0");
    let text = service["text"].as_str().unwrap();
    assert!(text.starts_with("John Kruze (Protocol Company). Zero-Trust Physics: a physics verifier for Super Intelligence. Served at https://mcp.zerotrustphysics.com/mcp"), "{text}");
    assert!(text.ends_with("https://doi.org/10.5281/zenodo.23305697"));
    assert!(service["about"].as_str().unwrap().starts_with("How to cite the service itself"));
    assert!(service.get("data").is_none());
    for (key, value) in service["service"].as_object().unwrap() {
        if key != "engine_version" && key != "citation_text" && key != "bibtex" {
            assert_eq!(&grid["service"][key], value, "{key}");
        }
    }
    assert_eq!(refused(&mut door, "cite", &json!({ "solver": "no_such" })), "Solver not recognized (UnknownSolver).");
    assert!(refused(&mut door, "cite", &json!({ "domain": "energy", "card": "thermal", "solver": "atom_dft" })).contains("not both"));
    assert!(refused(&mut door, "cite", &json!({ "domain": "energy" })).contains("'card'"));
    assert!(refused(&mut door, "cite", &json!({ "model": "thermal" })).contains("'model'"));
}

#[test]
fn a_key_cites_and_finds_only_what_it_sees() {
    let keyed = keyed();
    let (_, error, text) = keyed.tool(KEY, "cite", &json!({ "domain": "signal", "card": "resonance" }));
    assert_eq!((error, text.as_str()), (true, "Card not recognized (UnknownCard)."));
    let (_, error, text) = keyed.tool(KEY, "cite", &json!({ "solver": "ablator_slab" }));
    assert_eq!((error, text.as_str()), (true, "Solver not recognized (UnknownSolver)."));
    let (_, _, text) = keyed.tool(KEY, "find_model", &json!({ "description": "a satellite in orbit around the Earth" }));
    assert_eq!(parsed(&text)["count"], 0);
    let (_, _, text) = keyed.tool(KEY, "find_model", &json!({ "description": "power grid frequency after losing a link" }));
    assert_eq!(parsed(&text)["matches"][0]["card"], "grid_gb");
}

#[test]
fn find_model_gives_fit_first_and_then_how_the_model_did_against_measurement() {
    let mut door = Stdio2::start(None);
    let found = ok(&mut door, "find_model", &json!({ "description": "a lithium cell discharging in the cold" }));
    assert_eq!(found["count"], 3);
    assert_eq!(found["searched"], json!({ "cards": 40, "solvers": 25 }));
    let first = &found["matches"][0];
    assert_eq!((first["domain"].as_str(), first["card"].as_str()), (Some("energy"), Some("cell_2170_model3")));
    assert!(first["fit"].as_str().unwrap().starts_with("matches your description on the words ") && first["fit"].as_str().unwrap().contains("\"lithium\"") && first["fit"].as_str().unwrap().contains("\"cold\""));
    assert!(first["model_of"].as_str().unwrap().contains("2170"));
    assert!(first["compared_with"].as_str().unwrap().starts_with("Fitted on ") || first["compared_with"].as_str().unwrap().starts_with("Nothing fitted"), "{first}");
    assert_eq!(first["against_measurement"], "compared with measurement");
    let accuracy = first["accuracy_against_measurement"].as_str().unwrap();
    assert!(accuracy.starts_with("Compared with ") && accuracy.contains("within ") && accuracy.contains(" Gap: "), "{accuracy}");
    // The earlier names are gone, and so is any excerpt of the record's limits.
    for gone in ["met_its_marks", "marks", "tested_on", "cannot_answer", "why", "what", "limits"] {
        assert!(first.get(gone).is_none(), "{gone}");
    }
    // A model with a gap is found, with the gap stated as accuracy and not as fit.
    let strip = ok(&mut door, "find_model", &json!({ "description": "a thin aluminium strip clamped at one end, ringing after I flick it" }));
    assert_eq!(strip["matches"][0]["card"], "cantilever_eth");
    assert!(strip["matches"][0]["accuracy_against_measurement"].as_str().unwrap().contains(" Gap: "), "{strip}");
    assert!(strip["about"].as_str().unwrap().contains("a model with a gap") || strip["about"].as_str().unwrap().contains("is still the served model of its thing"));
    let burst = ok(&mut door, "find_model", &json!({ "description": "something falling into an atmosphere", "limit": 2 }));
    assert_eq!(burst["matches"][0]["card"], "capsule_stardust");
    assert!(burst["matches"][0]["accuracy_against_measurement"].as_str().unwrap().starts_with("Compared with "), "{burst}");
    assert_eq!(burst["matches"][1]["card"], "capsule_insight");
    let grid = ok(&mut door, "find_model", &json!({ "description": "power grid frequency after losing a link" }));
    assert_eq!(grid["matches"][0]["card"], "grid_gb");
    // A sentence in a user's own words finds the one model that fits.
    let cable = ok(&mut door, "find_model", &json!({ "description": "how far the mains frequency in Britain drops when the undersea power cable from France suddenly fails" }));
    assert_eq!((cable["count"].as_u64(), cable["matches"][0]["card"].as_str()), (Some(1), Some("grid_gb")));
    let pack = ok(&mut door, "find_model", &json!({ "description": "a 12S battery pack under load" }));
    assert_eq!(pack["matches"][0]["against_measurement"], "company specimen");
    assert!(pack["matches"][0]["accuracy_against_measurement"].as_str().unwrap().starts_with("Company specimen: an object the company specified, every constant stated"));
    // Nothing fits: the answer is complete in itself, and sends no one to the lists.
    let none = ok(&mut door, "find_model", &json!({ "description": "the temperature of the brake discs of a freight train after an emergency stop from 100 km/h" }));
    assert_eq!(none["matches"], json!([]));
    let said = none["none_fits"].as_str().unwrap();
    assert!(said.starts_with("Nothing served fits this description. It was compared with everything this connection serves (40 cards and 25 solvers), and no served model fits it; list_cards and list_solvers name everything served."), "{said}");
    // Where a request for a model goes: the maker, by name and address, said
    // as what is so. It read "write to John Kruze of Protocol Company at ...
    // and name an open dataset of it" until the instruction became a
    // statement. No tool keeps a list.
    assert!(said.contains("Requests for a model of a thing not served go to John Kruze of Protocol Company at kruze@zerotrustphysics.com."), "{said}");
    assert!(said.contains("a request names an open dataset of it") && !said.contains("write to"), "{said}");
    assert!(!said.contains("request_model"), "{said}");
    assert_eq!(none["nearest_by_words"], json!([]));
    assert!(none["nearest_by_words_about"].as_str().unwrap().contains("there is no near model to name"));
}

fn grid_run(loss_mw: f64) -> Value {
    json!({
        "domain": "energy", "card": "grid_gb",
        "initial": { "frequency_hz": 49.942, "inertia_mw_s": 241000.0, "demand_mw": 27132.0 },
        "dt_s": 0.02, "segments": [{ "steps": 100, "load": { "loss_mw": loss_mw } }], "record_every": 100
    })
}

fn pack_run(current_a: f64) -> Value {
    json!({
        "domain": "energy", "card": "battery_sag", "initial": { "soc": 1.0 }, "dt_s": 0.1,
        "segments": [{ "steps": 6000, "load": { "current_a": current_a } }], "record_every": 6000
    })
}

/// Every text block of a tool answer of the hosted connection.
fn blocks_of(reply: &Reply) -> Vec<String> {
    parsed(&reply.body)["result"]["content"]
        .as_array()
        .unwrap()
        .iter()
        .map(|block| block["text"].as_str().unwrap().to_string())
        .collect()
}

/// A stop before computing is the one sentence it was in 0.8.1, alone, on
/// every connection: one text block, and no header beside it on the web
/// connection. Nothing is suggested, moved to a limit or filled in.
#[tokio::test(flavor = "multi_thread")]
async fn a_model_that_stops_before_computing_answers_with_its_one_sentence_and_nothing_beside_it() {
    let mut stdio = Stdio2::start(None);
    let base = web().await;
    let keyed = keyed();
    let open = Open::start();
    let stops: Vec<(&str, Value, &str)> = vec![
        ("run_trajectory", { let mut run = grid_run(500.0); run["initial"]["demand_mw"] = json!(-1.0); run }, "Physical Failure: 'demand_mw' -1 MW is below 0 MW, the inclusive minimum of the grid_gb card (DemandNegative). That state is not sealed."),
        // Re-pinned for 0.10.1: a step above the ceiling carries the name DtAboveCeiling.
        ("verify_step", json!({ "domain": "energy", "card": "thermal", "parameters": { "temperature_k": 293.15, "heat_power_w": 1.0, "ambient_k": 293.15, "dt_s": 1.0e9 } }), "Physical Violation: 'dt_s' 1000000000 is above 23.625 s, the step ceiling of the thermal card (DtAboveCeiling)."),
        // Re-pinned for 0.10.1: an atomic number outside the range carries the name AtomicNumberOutOfRange, and the entry writes the range in words.
        ("solve", json!({ "solver": "atom_dft", "parameters": { "atomic_number": 40 } }), "Physical Violation: atomic number 40 is outside 1 to 18 (AtomicNumberOutOfRange)."),
    ];
    for (tool, request, sentence) in &stops {
        let local = stdio.blocks(tool, request);
        assert_eq!(local, vec![sentence.to_string()], "{tool}");
        let hosted = blocks_of(&keyed.post(KEY, &tool_body(1, tool, request)));
        assert_eq!(hosted, local, "{tool} hosted");
        let opened = blocks_of(&parse_reply(&raw(open.port, &request_bytes(None, &tool_body(1, tool, request)), 120)));
        assert_eq!(opened, local, "{tool} open");
        let path = if *tool == "verify_step" { "verify_physics" } else { tool };
        let response = reqwest::Client::new()
            .post(format!("{base}/{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(request.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 422);
        assert!(response.headers().get("nearest-request").is_none(), "{tool} web");
        assert_eq!(response.text().await.unwrap(), *sentence);
    }
}

#[test]
fn a_stop_part_way_and_an_answer_are_one_block_with_the_bytes_of_0_8_1() {
    let mut door = Stdio2::start(None);
    // A missing field is named; nothing is filled in.
    let mut missing = grid_run(996.0);
    missing["initial"].as_object_mut().unwrap().remove("demand_mw");
    assert_eq!(door.blocks("run_trajectory", &missing), vec!["Missing required parameter: 'demand_mw' (InputMissing).".to_string()]);
    let mut unknown = grid_run(996.0);
    unknown["card"] = json!("grid_fr");
    assert_eq!(door.blocks("run_trajectory", &unknown), vec!["Card not recognized (UnknownCard).".to_string()]);
    // A run that stops part way is a result, sealed to its last landed step, and its bytes are those of 0.8.1:
    // the seal printed for it by the saved 0.8.1 binary is the seal it has now.
    let blocks = door.blocks("run_trajectory", &pack_run(80.0));
    assert_eq!(blocks.len(), 1);
    let stopped = parsed(&blocks[0]);
    assert_eq!((stopped["outcome"].as_str(), stopped["steps_landed"].as_u64()), (Some("refused"), Some(4950)));
    assert_eq!(stopped["run_seal"], "bd8430a05a4fde3f3814808ae390a5c8d8f08e7144e9a16a5e9f0fe4b4873ac4");
    // An answer has one block, as before.
    assert_eq!(door.blocks("run_trajectory", &pack_run(50.0)).len(), 1);
}

/// The answers of the tools served in 0.8.1 carry the keys they carried
/// and no other: a receipt covers the whole answer, so a key added inside
/// one would change every printed receipt.
#[test]
fn the_earlier_answers_carry_no_new_key() {
    let mut door = Stdio2::start(None);
    let keys = |value: &Value| -> Vec<String> { value.as_object().unwrap().keys().cloned().collect() };
    let run = ok(&mut door, "run_trajectory", &pack_run(50.0));
    assert_eq!(
        keys(&run),
        ["inputs", "engine_version", "outcome", "steps_requested", "steps_landed", "reject", "reject_detail", "final_state",
         "final_residual_conservation_error", "final_residual_conservation_error_hex", "max_residual_conservation_error",
         "max_residual_conservation_error_hex", "state_digest", "load_source", "frames", "run_seal", "receipt_sha256"]
    );
    assert_eq!(run["run_seal"], "37e4cf1e9c2a207785891546f45a460661c50c895a9c8257646f4ef9713c0200");
    // One key was added, on John Kruze's word of 14 October 2026: every run
    // answer says where its loads came from. It moves every run receipt once
    // and no run seal; the answer with it taken out is, byte for byte, the
    // answer pinned below.
    assert_eq!(run["load_source"], json!({ "segments": 1, "signed": 0, "says": genesis_seal::attest::CALLERS_NUMBER }));
    let (_, labelled) = door.tool("run_trajectory", &pack_run(50.0));
    let label = format!(",\"load_source\":{}", serde_json::to_string(&run["load_source"]).unwrap());
    assert_eq!(labelled.matches(&label).count(), 1);
    let labelled_document = labelled.rsplit_once(",\"receipt_sha256\":").unwrap().0;
    assert_eq!(run["receipt_sha256"], sha(&format!("{labelled_document}}}")));
    let unlabelled = labelled.replacen(&label, "", 1);
    let unlabelled_document = unlabelled.rsplit_once(",\"receipt_sha256\":").unwrap().0;
    let run: Value = serde_json::from_str(&format!("{unlabelled_document},\"receipt_sha256\":\"{}\"}}", sha(&format!("{unlabelled_document}}}")))).unwrap();
    // The receipt covers every byte of the answer, the engine version among them, and moved with
    // kernel 0.9.0, 0.10.0, 0.11.0 and 0.12.0: the same text with genesis_kernel_v0.11.0 put back
    // gives the receipt pinned for 0.11.0, f5bd6143...636e, with genesis_kernel_v0.10.0 the
    // receipt pinned for 0.10.0, 323535ed...7d25, with genesis_kernel_v0.9.0 the receipt pinned
    // for 0.9.0, 743c0cfb...ee52, and with genesis_kernel_v0.8.0 the receipt printed before,
    // 2329593a...515e. The run seal does not cover the version and is the seal it was.
    assert_eq!(run["engine_version"], "genesis_kernel_v0.12.0");
    assert_eq!(run["receipt_sha256"], "dc02efc588c87bcef75162bcacfe37d8fe5d65b4da5c7e135add536d96c9fe55");
    let text = format!("{unlabelled_document},\"receipt_sha256\":\"{}\"}}", sha(&format!("{unlabelled_document}}}")));
    let document = text.strip_suffix(",\"receipt_sha256\":\"dc02efc588c87bcef75162bcacfe37d8fe5d65b4da5c7e135add536d96c9fe55\"}").expect("the receipt is last");
    assert_eq!(sha(&format!("{document}}}")), "dc02efc588c87bcef75162bcacfe37d8fe5d65b4da5c7e135add536d96c9fe55");
    let before = document.replacen("genesis_kernel_v0.12.0", "genesis_kernel_v0.11.0", 1);
    assert_eq!(sha(&format!("{before}}}")), "f5bd614398bfd91a0a039302c46a2e545013e8aec1112c44dffd7e988520636e");
    let before = document.replacen("genesis_kernel_v0.12.0", "genesis_kernel_v0.10.0", 1);
    assert_eq!(sha(&format!("{before}}}")), "323535ed160a543c495de118bea44d611529bdd86ba08ade320f6811a3757d25");
    let before = document.replacen("genesis_kernel_v0.12.0", "genesis_kernel_v0.9.0", 1);
    assert_eq!(sha(&format!("{before}}}")), "743c0cfbffc80865c572159464a6b43f7a50abac0a8fd7425579e44d9358ee52");
    let earlier = document.replacen("genesis_kernel_v0.12.0", "genesis_kernel_v0.8.0", 1);
    assert_eq!(sha(&format!("{earlier}}}")), "2329593a18f106db67fb9d04363f8f1eb37a8e7d2ca42784a41a21609914515e");
    let step = ok(&mut door, "verify_step", &thermal_step());
    assert_eq!(keys(&step), ["inputs", "engine_version", "state", "residual_conservation_error", "sha256_seal", "receipt_sha256"]);
    let solved = ok(&mut door, "solve", &json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10 } }));
    assert_eq!(
        keys(&solved),
        ["inputs", "engine_version", "outcome", "reject", "reject_detail", "iterations", "result", "invariants", "field_sha256",
         "seal_parts", "solution_seal", "receipt_sha256"]
    );
    assert_eq!(solved["seal_parts"]["inputs"], "1eac37e0cf92dfc49ae0d9699482e4d235b0815737d818439be116ccc464ab41");
}

fn pack_flip() -> Value {
    json!({ "request": pack_run(60.0), "vary": "segments[0].load.current_a", "low": 50.0, "high": 80.0 })
}

#[tokio::test(flavor = "multi_thread")]
async fn find_flip_gives_the_same_bytes_on_every_connection() {
    same_on_every_connection(&[
        ("find_flip", pack_flip(), false),
        ("find_flip", json!({ "request": pack_run(60.0), "vary": "segments[0].load.current_a", "low": 10.0, "high": 60.0, "level": { "quantity": "terminal_v", "value": 40.0 } }), false),
        ("find_flip", json!({ "request": { "solver": "atom_dft", "parameters": { "atomic_number": 10 } }, "vary": "parameters.atomic_number", "low": 10, "high": 30 }), false),
        ("find_flip", json!({ "request": pack_run(60.0), "vary": "segments[0].load.voltage_v", "low": 1, "high": 2 }), true),
        ("find_flip", json!({ "request": { "domain": "energy", "card": "grid_gb", "segments": [] }, "vary": "dt_s", "low": 1, "high": 2 }), true),
    ])
    .await;
}

#[test]
fn find_flip_returns_the_two_runs_either_side_each_with_the_proof_it_has_alone() {
    let mut door = Stdio2::start(None);
    let found = ok(&mut door, "find_flip", &pack_flip());
    assert_eq!(found["tool"], "run_trajectory");
    assert_eq!((found["low_is"].as_str(), found["high_is"].as_str()), (Some("completed"), Some("refused (UndervoltageCutoff)")));
    assert_eq!((found["flips"].as_bool(), found["neighbours"].as_bool()), (Some(true), Some(true)));
    let low = found["low"]["value"].as_f64().unwrap();
    let high = found["high"]["value"].as_f64().unwrap();
    // The two runs printed on the front page lie either side of the pair.
    assert!(71.44 < low && low < high && high < 71.45);
    assert_eq!(f64::from_bits(u64::from_str_radix(found["low"]["value_hex"].as_str().unwrap(), 16).unwrap()), low);
    assert_eq!(f64::from_bits(u64::from_str_radix(found["width_hex"].as_str().unwrap(), 16).unwrap()), high - low);
    assert_eq!(f64::from_bits(low.to_bits() + 1), high);
    // Each side replays: the request alone gives the same seal, and verify_tape holds it.
    for side in ["low", "high"] {
        let alone = ok(&mut door, "run_trajectory", &found[side]["request"]);
        assert_eq!(alone["run_seal"], found[side]["run_seal"], "{side}");
        assert_eq!(alone["outcome"], found[side]["outcome"]);
        assert_eq!(alone["reject_detail"], found[side]["reject_detail"]);
        let held = ok(&mut door, "verify_tape", &json!({ "request": found[side]["request"], "run_seal": found[side]["run_seal"] }));
        assert_eq!(held["holds"], true);
    }
    assert!(found["high"]["reject_detail"].as_str().unwrap().ends_with("The run is sealed to step 5999. The state at step 6000 was refused and is not sealed."));
    // The same request gives the same pair, byte for byte.
    assert_eq!(door.tool("find_flip", &pack_flip()), door.tool("find_flip", &pack_flip()));
    // One stop name to another, and a run that ends beside one that completes.
    let reactor = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "reactor_spert3" }));
    let reactor_run = reactor["example_run_trajectory"].clone();
    let asked = reactor_run["segments"][0]["steps"].as_u64().unwrap();
    let ended = ok(&mut door, "find_flip", &json!({
        "request": reactor_run, "vary": "segments[0].steps", "low": 1, "high": asked
    }));
    assert_eq!((ended["low_is"].as_str(), ended["high_is"].as_str()), (Some("completed"), Some("ended (PastFirstMaximum)")), "{ended}");
    assert_eq!(ended["neighbours"], true);
    // A side that stops before computing has no proof, and says so.
    let door_side = ok(&mut door, "find_flip", &json!({ "request": grid_run(0.0), "vary": "initial.demand_mw", "low": -1000.0, "high": 27132.0 }));
    assert_eq!(door_side["low_is"], "stopped before computing (DemandNegative)");
    assert_eq!(door_side["low"]["run_seal"], Value::Null);
    assert!(door_side["low"]["no_proof"].as_str().unwrap().contains("no proof is issued"));
    let edge = door_side["high"]["value"].as_f64().unwrap();
    assert!((0.0..1.0e-9).contains(&edge), "{edge}");
    assert_eq!(door_side["high"]["outcome"], "completed");
}

#[test]
fn on_the_open_connection_a_search_is_long_work_and_is_marked_read_only() {
    let open = Open::start();
    let reply = parse_reply(&raw(open.port, &request_bytes(None, &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string()), 60));
    let listed = parsed(&reply.body);
    let tools = listed["result"]["tools"].as_array().unwrap();
    for name in ["cite", "find_model", "find_flip"] {
        let tool = tools.iter().find(|tool| tool["name"] == name).unwrap_or_else(|| panic!("{name} is not listed"));
        assert_eq!(tool["annotations"]["destructiveHint"], false, "{name}");
    }
    let flip = tools.iter().find(|tool| tool["name"] == "find_flip").unwrap();
    assert!(flip["description"].as_str().unwrap().contains("FLIP_MAX_RUNS") && flip["description"].as_str().unwrap().contains("long work"));
    // It answers there with the bytes of the local connection.
    let mut stdio = Stdio2::start(None);
    assert_eq!(open.tool("find_flip", &pack_flip()), stdio.tool("find_flip", &pack_flip()));
}

/// The new tools' own words say which name to give and that the two ends of
/// a span come in either order, and they speak of an agent, never a caller.
#[test]
fn the_new_tools_say_which_name_to_give_and_take_a_span_in_either_order() {
    let tools = protocol_mcp_gateway::mcp_server::tools_in(&protocol_mcp_gateway::view::View::full());
    let said = |name: &str| serde_json::to_string(tools.iter().find(|tool| tool.name == name).unwrap()).unwrap();
    // A name is the name find_model or the lists give: the two give the
    // same names, which every other tool takes.
    let whole = serde_json::to_string(&tools[13..16]).unwrap();
    assert!(!whole.contains("as list_cards names it") && !whole.contains("as list_solvers names it"));
    assert!(said("cite").contains("by the name find_model or list_cards gives") && said("cite").contains("by the name find_model or list_solvers gives"));
    assert!(said("find_flip").contains("by the name find_model or the lists give"));
    assert!(said("find_model").contains("are the names list_cards and list_solvers give and the names describe_card, describe_solver, cite and every other tool take"));
    // Since 0.10.1 no description says what need not be called.
    assert!(!whole.contains("no list need be read"), "{whole}");
    // low and high come in either order.
    assert!(said("find_flip").contains("in either order: the smaller is taken as low"));
    let mut door = Stdio2::start(None);
    let mut backward = pack_flip();
    let (low, high) = (backward["low"].clone(), backward["high"].clone());
    backward["low"] = high;
    backward["high"] = low;
    assert_eq!(ok(&mut door, "find_flip", &backward), ok(&mut door, "find_flip", &pack_flip()));
    // Tool texts are read by agents and people: an agent, never a caller.
    assert!(!whole.contains("caller"), "{whole}");
}

/// No connection keeps anything. The two tools that kept a list are not
/// served, no tool declares or mentions them, the flags that named their
/// files are not arguments of the hosted connection, and an answer has no
/// address: one text block, no `_meta`, no header, and no page at `/p/`.
#[test]
fn no_connection_keeps_anything() {
    let tools = protocol_mcp_gateway::mcp_server::tools_in(&protocol_mcp_gateway::view::View::full());
    assert_eq!(tools.len(), 18);
    let declared = serde_json::to_string(&tools).unwrap();
    for gone in ["request_model", "list_model_requests", "short address", "proof_address", "is kept", "nearest_request"] {
        assert!(!declared.contains(gone), "the tool list says {gone}");
    }
    let mut door = Stdio2::start(None);
    for gone in ["request_model", "list_model_requests"] {
        assert_eq!(refused(&mut door, gone, &json!({})), format!("Tool not recognized: {gone}"));
    }
    let start = |arguments: &[&str]| -> String {
        let out = Command::new(env!("CARGO_BIN_EXE_hosted_door")).args(arguments).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{arguments:?}");
        String::from_utf8_lossy(&out.stderr).into_owned()
    };
    for flag in ["--model-requests", "--proof-store", "--proof-address"] {
        let said = start(&["--open", "--port", "0", flag, "x"]);
        assert!(said.contains(&format!("'{flag}' is not an argument of this program")), "{said}");
    }
    let open = Open::start();
    let reply = parse_reply(&raw(open.port, &request_bytes(None, &tool_body(1, "run_trajectory", &pack_run(71.44))), 120));
    let result = &parsed(&reply.body)["result"];
    assert!(result.get("_meta").is_none());
    assert_eq!(result["content"].as_array().unwrap().len(), 1);
    assert!(!reply.head.to_ascii_lowercase().contains("proof-address"), "{}", reply.head);
    // The same bytes with ?keep=no at the address's end: there is nothing to keep or not to keep.
    let by_address = String::from_utf8(request_bytes(None, &tool_body(1, "run_trajectory", &pack_run(71.44)))).unwrap().replacen("POST /mcp ", "POST /mcp?keep=no ", 1);
    let quiet = parse_reply(&raw(open.port, by_address.as_bytes(), 120));
    assert_eq!(parsed(&quiet.body)["result"], *result);
    let page = parse_reply(&raw(open.port, b"GET /p/6bd7e54b83c71da84eda HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n", 30));
    assert_eq!(page.status, 404);
    // What the connection says of itself when it opens names no list and no kept answer.
    let hello = parsed(&parse_reply(&raw(open.port, &request_bytes(None, &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string()), 30)).body);
    let said = hello["result"]["instructions"].as_str().unwrap();
    for gone in ["request_model", "list_model_requests", "is kept", "keep=no"] {
        assert!(!said.contains(gone), "{gone}: {said}");
    }
}

/// The first thing an agent reads names the verifier, on the local connection
/// and on the hosted one with and without a key: the server's name, title and
/// site, and instructions that open with what this is.
#[test]
fn every_connection_says_who_it_is_first() {
    let hello = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string();
    // Since 0.10.1 the opening named the engine's own source; since 0.14.1 it
    // said, in John Kruze's words, that past what a model was compared with it
    // keeps going and flags every step. Since 0.14.2 (John Kruze, 18 October
    // 2026) the instructions are five sentences, about 560 characters: the
    // maker, the site, the paper and the sources are in cite's answer.
    let opening = "Zero-Trust Physics: deterministic physics, a proof on every answer. Start with find_model (describe the real thing) or list_cards / list_solvers, and read describe_card or describe_solver (form \"short\") before a first request. Past what a model was compared with, it keeps going with the same equations and flags every step. It stops only at a physical limit, or where its equations, constants or tables no longer hold, and it names the reason. verify_tape, verify_solution and verify_dataset check the proof of a run, a solution or a dataset; cite gives sources.";
    let stdio = Stdio2::start(None);
    let local = parsed(&stdio.hello);
    let keyed = keyed();
    let hosted = parsed(&keyed.post(KEY, &hello).body);
    let open = Open::start();
    let opened = parsed(&parse_reply(&raw(open.port, &request_bytes(None, &hello), 30)).body);
    for (at, answer) in [("local", &local), ("hosted", &hosted), ("open", &opened)] {
        let info = &answer["result"]["serverInfo"];
        assert_eq!(info["name"], "Zero-Trust Physics", "{at}");
        assert_eq!(info["title"], "Zero-Trust Physics: a physics verifier for Super Intelligence", "{at}");
        assert_eq!(info["websiteUrl"], "https://zerotrustphysics.com", "{at}");
        assert_eq!(info["version"], env!("CARGO_PKG_VERSION"), "{at}");
        assert_eq!(info["description"], "genesis_kernel_v0.12.0", "{at}");
        let said = answer["result"]["instructions"].as_str().unwrap();
        assert!(said.starts_with(opening), "{at}: {said}");
        for gone in ["John Kruze", "928-264-7883", "https://github.com/", "https://doi.org/"] {
            assert!(!said.contains(gone), "{at}, {gone}: {said}");
        }
    }
    // The local and the keyed connection say the same; the open one adds its limits after.
    assert_eq!(local["result"]["instructions"], hosted["result"]["instructions"]);
    let (local_said, open_said) = (local["result"]["instructions"].as_str().unwrap(), opened["result"]["instructions"].as_str().unwrap());
    assert_eq!(local_said, opening);
    assert!(open_said.starts_with(&format!("{local_said} This open connection asks no key and stores nothing. From one address:")) && open_said.ends_with(" Each tool that computes states its own limits in its description."), "{open_said}");
}
