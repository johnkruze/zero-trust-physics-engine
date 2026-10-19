//! The tool changes made on the chained-proof build with no change of
//! version number: the other read quantities at each extreme and at the first
//! step past a level; a count in list_cards and list_solvers; a short check
//! that does not hold hands back no seal; every short check says it compared
//! no answer text; the part-way sentence leads with the sealed step; solve
//! takes a worked example by name; a start that carries a run on arrives with
//! from, or is refused as ContinuationWithoutUpstream. Every connection
//! gives the same bytes, and nothing is stored.

mod hosted_support;
use hosted_support::answer_text;

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use hosted_support::*;
use serde_json::{Value, json};
use tokio::net::TcpListener;

const KEY: &str = "tools-071-key";

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
    let response = if tool == "list_cards" || tool == "list_solvers" {
        let path = if tool == "list_cards" { "cards" } else { "solvers" };
        reqwest::Client::new().get(format!("{base}/{path}")).send().await.unwrap()
    } else {
        let path = if tool == "verify_step" { "verify_physics" } else { tool };
        reqwest::Client::new()
            .post(format!("{base}/{path}"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(answer_text::as_sent(body))
            .send()
            .await
            .unwrap()
    };
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    answer_text::remember(&text);
    (status, text)
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

fn reactor_burst(record_every: u64) -> Value {
    json!({
        "domain": "energy", "card": "reactor_spert3",
        "initial": { "coolant_temperature_k": 300.92777777777775 },
        "dt_s": 0.0007144471176212568,
        "segments": [{ "steps": 3000, "load": { "reactivity_inserted_dollar": 0.9982818460448638 } }],
        "record_every": record_every
    })
}

/// A run of the cold plate as a plan lists it, without its pair.
fn plan_run(steps: u64, temperature_k: f64) -> Value {
    let mut run = thermal_run(steps, temperature_k);
    let object = run.as_object_mut().unwrap();
    object.remove("domain");
    object.remove("card");
    run
}

fn with(mut request: Value, key: &str, value: Value) -> Value {
    request[key] = value;
    request
}

fn exact(hex: &Value) -> Value {
    match hex {
        Value::String(text) => json!(f64::from_bits(u64::from_str_radix(text, 16).unwrap())),
        Value::Array(items) => Value::Array(items.iter().map(exact).collect()),
        _ => panic!("{hex}"),
    }
}

/// Where a continuation reads a start field its card's returned state gives
/// under another name (src/chain.rs).
fn source(card: &str, field: &str) -> Option<&'static str> {
    Some(match (card, field) {
        ("ti64_lattice", "start_t_s") => "t_s",
        ("ti64_lattice", "start_force_n") => "applied_force_n",
        ("ti64_lattice", "start_temperature_k") => "temperature_k",
        ("ti64_lattice", "start_energy_anchor_j") => "energy_anchor_j",
        ("ti64_lattice", "start_boundary_work_j") => "boundary_work_j",
        ("generator_mmrtg_f1", "mission_time_s") => "t_s",
        ("generator_mmrtg_seebeck", "mission_time_s") => "t_s",
        ("tokamak_jet_dte1", "elapsed_time_s") => "t_s",
        _ => return None,
    })
}

/// The start a continuation of `upstream` states, every field read from the
/// exact hex of the upstream's final state.
fn handed_back(described: &Value, upstream: &Value, answer: &Value) -> Value {
    let card = described["card"].as_str().unwrap();
    let state = &answer["final_state"];
    let mut initial = serde_json::Map::new();
    for field in described["phase_fields"].as_array().unwrap() {
        let name = field["name"].as_str().unwrap();
        let value = if (card, name) == ("cell_18650pf", "held_current_a") {
            upstream["initial"]["held_current_a"].clone()
        } else {
            exact(&state[format!("{}_hex", source(card, name).unwrap_or(name))])
        };
        initial.insert(name.to_string(), value);
    }
    Value::Object(initial)
}

#[tokio::test(flavor = "multi_thread")]
async fn every_connection_gives_the_same_bytes_for_the_changed_tools() {
    let mut stdio = Stdio2::start(None);
    let base = web().await;
    let keyed = Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY),
            "cards": [["energy", "thermal"], ["energy", "reactor_spert3"], ["signal", "resonance"]],
            "solvers": ["atom_dft"], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let open = Open::start();
    let readout = json!({ "quantities": ["power_mw", "energy_mws"], "first_past": { "quantity": "power_mw", "above": 5.0 } });
    let (_, plate) = stdio.tool("run_trajectory", &thermal_run(10, 293.15));
    let plate = parsed(&plate);
    let wrong = "0".repeat(64);
    let resonance_step = json!({
        "domain": "signal", "card": "resonance",
        "parameters": { "displacement_m": 0.0, "velocity_m_s": 0.0, "t_s": 0.5, "energy_anchor_j": 0.0, "boundary_work_j": 0.0, "accel_m_s2": 1.0, "dt_s": 0.0003 }
    });
    let calls: Vec<(&str, Value, bool)> = vec![
        ("run_trajectory", with(reactor_burst(3000), "readout", readout), false),
        ("verify_tape", json!({ "request": thermal_run(10, 293.15), "run_seal": plate["run_seal"] }), false),
        ("verify_tape", json!({ "request": thermal_run(10, 293.15), "run_seal": wrong }), false),
        ("solve", json!({ "solver": "atom_dft", "example": "example_solve" }), false),
        ("verify_solution", json!({ "request": { "solver": "atom_dft", "example": "example_solve" }, "solution_seal": wrong }), false),
        ("verify_step", resonance_step, true),
        ("describe_card", json!({ "domain": "signal", "card": "resonance" }), false),
    ];
    for (tool, arguments, error_due) in &calls {
        let (error, local) = stdio.tool(tool, arguments);
        assert_eq!(error, *error_due, "{tool}: {local}");
        let (status, over_web) = post(&base, tool, arguments).await;
        assert_eq!(over_web, local, "{tool} web");
        assert_eq!(status == 200, !error_due, "{tool} web status {status}");
        let (status, hosted_error, hosted) = keyed.tool(KEY, tool, arguments);
        assert_eq!((status, hosted_error, hosted.as_str()), (200, *error_due, local.as_str()), "{tool} hosted");
        let (open_error, opened) = open.tool(tool, arguments);
        assert_eq!((open_error, opened.as_str()), (*error_due, local.as_str()), "{tool} open");
    }
    // Each list states its count, of what the caller's view holds.
    let (_, cards) = stdio.tool("list_cards", &json!({}));
    let (_, over_web) = post(&base, "list_cards", &json!({})).await;
    assert_eq!(over_web, cards);
    let (_, _, hosted) = keyed.tool(KEY, "list_cards", &json!({}));
    assert_eq!(parsed(&hosted)["count"], json!(3));
    assert_eq!(parsed(&hosted)["cards"].as_array().unwrap().len(), 3);
    let (_, opened) = open.tool("list_cards", &json!({}));
    assert_eq!(opened, cards);
    let (_, _, solvers) = keyed.tool(KEY, "list_solvers", &json!({}));
    assert_eq!(parsed(&solvers)["count"], json!(1));
}

#[test]
fn each_list_states_its_count() {
    let mut door = Stdio2::start(None);
    let cards = ok(&mut door, "list_cards", &json!({}));
    assert_eq!(cards["count"], json!(40));
    assert_eq!(cards["cards"].as_array().unwrap().len(), 40);
    let solvers = ok(&mut door, "list_solvers", &json!({}));
    assert_eq!(solvers["count"], json!(25));
    assert_eq!(solvers["solvers"].as_array().unwrap().len(), 25);
    // The count comes first, before the list it counts.
    let (_, text) = door.tool("list_cards", &json!({}));
    assert!(text.starts_with("{\"count\":40,\"cards\":["), "{}", &text[..40]);
}

#[test]
fn a_readout_gives_the_other_quantities_at_each_extreme_and_at_the_level() {
    let mut door = Stdio2::start(None);
    let readout = json!({ "quantities": ["power_mw", "energy_mws", "reactivity_net_dollar"], "first_past": { "quantity": "power_mw", "above": 5.0 } });
    let answer = ok(&mut door, "run_trajectory", &with(reactor_burst(1), "readout", readout.clone()));
    let out = &answer["readout"];
    let steps: Vec<u64> = out["steps"].as_array().unwrap().iter().map(|s| s.as_u64().unwrap()).collect();
    let names = ["power_mw", "energy_mws", "reactivity_net_dollar"];
    let at = |quantity: usize, step: u64| -> Value {
        let index = steps.iter().position(|s| *s == step).unwrap();
        out["quantities"][quantity]["values"][index].clone()
    };
    for (index, name) in names.iter().enumerate() {
        let entry = &out["quantities"][index];
        for (extreme, others) in [("minimum", "at_minimum"), ("maximum", "at_maximum")] {
            let step = entry[format!("{extreme}_step")].as_u64().unwrap();
            let listed = entry[others].as_object().unwrap();
            // Every other quantity, by name, in the readout's order, and not itself.
            let keys: Vec<&str> = listed.keys().map(String::as_str).collect();
            let expected: Vec<&str> = names.iter().copied().filter(|other| other != name).collect();
            assert_eq!(keys, expected, "{name} {others}");
            for (other, other_name) in names.iter().enumerate() {
                if other != index {
                    assert_eq!(listed[*other_name], at(other, step), "{name} {others} {other_name}");
                }
            }
        }
    }
    let first = &out["first_past"];
    let step = first["step"].as_u64().unwrap();
    assert_eq!(first["at_step"]["energy_mws"], at(1, step));
    assert_eq!(first["at_step"]["reactivity_net_dollar"], at(2, step));
    assert!(first["at_step"].get("power_mw").is_none());
    // With one recorded state the same values are given: they are read at
    // every landed step, not only at the recorded ones.
    let sparse = ok(&mut door, "run_trajectory", &with(reactor_burst(3000), "readout", readout));
    assert_eq!(sparse["readout"]["first_past"], out["first_past"]);
    for index in 0..3 {
        assert_eq!(sparse["readout"]["quantities"][index]["at_maximum"], out["quantities"][index]["at_maximum"]);
        assert_eq!(sparse["readout"]["quantities"][index]["at_minimum"], out["quantities"][index]["at_minimum"]);
    }
    // The proof is still that of the run without the readout.
    let plain = ok(&mut door, "run_trajectory", &reactor_burst(3000));
    assert_eq!(sparse["run_seal"], plain["run_seal"]);
}

#[test]
fn a_short_check_that_does_not_hold_hands_back_no_seal() {
    let mut door = Stdio2::start(None);
    let run = thermal_run(10, 293.15);
    let answer = ok(&mut door, "run_trajectory", &run);
    let typed = "2be9b0f1d5a5c1c7e8d1d5f3f0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2d";
    let (_, text) = door.tool("verify_tape", &json!({ "request": run, "run_seal": typed }));
    let report = parsed(&text);
    assert_eq!(report["holds"], json!(false));
    assert_eq!(report["cause"], json!("run_seal"));
    assert!(report.get("recomputed_run_seal").is_none() && report.get("recomputed_receipt_sha256").is_none(), "{text}");
    assert!(!text.contains(answer["run_seal"].as_str().unwrap()), "the true seal was handed back: {text}");
    assert!(report["not_held"].as_str().unwrap().contains("the true run_seal of this request is in the answer run_trajectory gives to it"), "{text}");
    // Held: the seal computed again is given.
    let held = ok(&mut door, "verify_tape", &json!({ "request": run, "run_seal": answer["run_seal"] }));
    assert_eq!(held["holds"], json!(true));
    assert_eq!(held["recomputed_run_seal"], answer["run_seal"]);
    assert!(held.get("not_held").is_none());

    // A solution.
    let solve = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } });
    let solved = ok(&mut door, "solve", &solve);
    let (_, text) = door.tool("verify_solution", &json!({ "request": solve, "solution_seal": typed }));
    let report = parsed(&text);
    assert_eq!((report["holds"].clone(), report["cause"].clone()), (json!(false), json!("solution_seal")));
    assert!(!text.contains(solved["solution_seal"].as_str().unwrap()), "{text}");
    assert!(report.get("recomputed_solution_seal").is_none());

    // A dataset.
    let plan = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "list", "runs": [plan_run(5, 293.15), plan_run(5, 300.0)] },
        "store": "none", "return": "inline"
    });
    let made = ok(&mut door, "run_dataset", &plan);
    let seal = made["head"]["dataset_seal"].clone();
    let (_, text) = door.tool("verify_dataset", &json!({ "request": plan, "dataset_seal": typed }));
    let report = parsed(&text);
    assert_eq!(report["holds"], json!(false));
    assert!(!text.contains(seal.as_str().unwrap()), "{text}");
    assert!(report.get("recomputed_dataset_seal").is_none());
    let held = ok(&mut door, "verify_dataset", &json!({ "request": plan, "dataset_seal": seal }));
    assert_eq!(held["recomputed_dataset_seal"], seal);
}

#[test]
fn every_short_check_says_it_compared_no_answer_text() {
    let mut door = Stdio2::start(None);
    // A solve whose copy has one decimal changed and its hex left.
    let solve = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } });
    let (_, text) = door.tool("solve", &solve);
    let answer = parsed(&text);
    let total = answer["result"]["total_energy_ha"].as_f64().unwrap();
    let altered = text.replacen(&serde_json::to_string(&total).unwrap(), &serde_json::to_string(&(total + 0.1)).unwrap(), 1);
    assert_ne!(altered, text);
    let short = ok(&mut door, "verify_solution", &json!({ "request": parsed(&altered)["inputs"], "solution_seal": parsed(&altered)["solution_seal"] }));
    assert_eq!(short["holds"], json!(true), "the short check computes the request, not the copy");
    let words = short["text_not_compared"].as_str().unwrap();
    assert!(words.contains("No answer text you hold was compared, so a copy with a decimal changed still holds here"), "{words}");
    assert!(words.contains("compare that receipt_sha256 with recomputed_receipt_sha256"), "{words}");
    // verify_receipt on the copy is the check of the text.
    let receipt = ok(&mut door, "verify_receipt", &json!({ "response_text": altered }));
    assert_eq!(receipt["matches"], json!(false));
    // The same sentence on a run's and a dataset's short check, held or not.
    let run = thermal_run(10, 293.15);
    let made = ok(&mut door, "run_trajectory", &run);
    for seal in [made["run_seal"].clone(), json!("0".repeat(64))] {
        let report = ok(&mut door, "verify_tape", &json!({ "request": run, "run_seal": seal }));
        assert_eq!(report["text_not_compared"].as_str(), Some(words));
    }
    let plan = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "list", "runs": [plan_run(5, 293.15)] },
        "store": "none", "return": "inline"
    });
    // A dataset's says what is and is not compared for a dataset (0.8.1). The
    // words were "No tool compares the decimals in a dataset's rows" until the
    // long form held each decimal of a final state to its hex; since 0.12.0
    // the long form runs the plan again and compares the answer byte for byte.
    let report = ok(&mut door, "verify_dataset", &json!({ "request": plan, "dataset_seal": "0".repeat(64) }));
    let words = report["text_not_compared"].as_str().unwrap();
    assert!(words.contains("no rows you hold were compared") && words.contains("then runs its plan again and compares the engine's full answer with the one sent, value by value and byte for byte"), "{report}");
}

#[test]
fn the_part_way_sentence_leads_with_the_sealed_step() {
    let mut door = Stdio2::start(None);
    let hot = json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 390.0 }, "dt_s": 1.0,
        "segments": [{ "steps": 600, "load": { "heat_power_w": 3000.0, "ambient_k": 293.15 } }],
        "record_every": 600
    });
    let answer = ok(&mut door, "run_trajectory", &hot);
    let landed = answer["steps_landed"].as_u64().unwrap();
    let detail = answer["reject_detail"].as_str().unwrap();
    assert!(
        detail.ends_with(&format!(" The run is sealed to step {landed}. The state at step {} was refused and is not sealed.", landed + 1)),
        "{detail}"
    );
}

#[test]
fn solve_takes_a_worked_example_by_name_and_seals_the_filled_request() {
    let mut door = Stdio2::start(None);
    let described = ok(&mut door, "describe_solver", &json!({ "solver": "ablator_slab", "part": "example_solve_stardust_near_nose" }));
    let parameters = described["example_solve_stardust_near_nose"]["parameters"].clone();
    assert!(parameters.is_object(), "{described}");
    let typed = json!({ "solver": "ablator_slab", "parameters": parameters });
    let by_name = json!({ "solver": "ablator_slab", "example": "example_solve_stardust_near_nose" });
    let (_, typed_text) = door.tool("solve", &typed);
    let (error, named_text) = door.tool("solve", &by_name);
    assert!(!error, "{named_text}");
    // The same request is solved and sealed: the filled one.
    assert_eq!(named_text, typed_text);
    let named = parsed(&named_text);
    assert_eq!(named["inputs"], typed);
    // Checked by request in either form.
    for request in [&by_name, &typed] {
        let report = ok(&mut door, "verify_solution", &json!({ "request": request, "solution_seal": named["solution_seal"] }));
        assert_eq!(report["holds"], json!(true));
    }
    // The first worked example is example_solve.
    let atom = ok(&mut door, "solve", &json!({ "solver": "atom_dft", "example": "example_solve" }));
    let described = ok(&mut door, "describe_solver", &json!({ "solver": "atom_dft" }));
    assert_eq!(atom["inputs"], described["example_solve"]);
    // A name the solver does not list, both forms at once, and not a string.
    let text = refused(&mut door, "solve", &json!({ "solver": "atom_dft", "example": "example_solve_neon" }));
    assert!(text.contains("'example' must be the name of a worked example of atom_dft: \"example_solve\"; this request has \"example_solve_neon\"."), "{text}");
    let text = refused(&mut door, "solve", &json!({ "solver": "atom_dft", "example": "example_solve", "parameters": { "atomic_number": 2 } }));
    assert!(text.contains("takes 'parameters' or 'example', not both"), "{text}");
    let text = refused(&mut door, "solve", &json!({ "solver": "atom_dft", "example": 1 }));
    assert!(text.contains("'example' must be a string"), "{text}");
    let text = refused(&mut door, "solve", &json!({ "solver": "no_such", "example": "example_solve" }));
    assert_eq!(text, "Solver not recognized (UnknownSolver).");
}

/// For each card the rule holds: a value of a continuation field that a
/// fresh start does not state, which the card itself takes.
fn continued_start(card: &str, run: &mut Value, step: &mut Value) {
    let set = |request: &mut Value, section: &str, field: &str, value: Value| {
        request[section][field] = value;
    };
    let both = |run: &mut Value, step: &mut Value, field: &str, value: Value| {
        set(run, "initial", field, value.clone());
        set(step, "parameters", field, value);
    };
    match card {
        "ti64_lattice" => both(run, step, "start_t_s", json!(1.0e-3)),
        "resonance" => both(run, step, "t_s", json!(0.5)),
        "tesseract" => both(run, step, "energy_defect_j", json!(1.0e-13)),
        "cell_18650pf" => both(run, step, "soc", json!(0.9)),
        "satellite_lageos1" => both(run, step, "forecast_elapsed_s", json!(3600.0)),
        "cell_2170_model3" => both(run, step, "case_temperature_least_c", json!(24.0)),
        "tokamak_jet_dte1" => both(run, step, "elapsed_time_s", json!(1.0)),
        "accumulator_sbo450" => both(run, step, "work_moved_j", json!(100.0)),
        "solar_system" => both(run, step, "booked_work", json!(1.0e12)),
        other => panic!("{other}"),
    }
}

const RULE_CARDS: [(&str, &str, &str); 8] = [
    ("materials", "ti64_lattice", "start_t_s"),
    ("signal", "resonance", "t_s"),
    ("signal", "tesseract", "energy_defect_j"),
    ("aerospace", "satellite_lageos1", "forecast_elapsed_s"),
    ("energy", "cell_2170_model3", "case_temperature_least_c"),
    ("energy", "tokamak_jet_dte1", "elapsed_time_s"),
    ("energy", "accumulator_sbo450", "work_moved_j"),
    ("aerospace", "solar_system", "booked_work"),
];

/// A start whose continuation fields are not a fresh start's arrives with
/// from or is refused by name; a fresh start needs no from. Per card: the
/// worked run and step (fresh) answer; a continued start without from is
/// refused as ContinuationWithoutUpstream, run and step, with no proof; the
/// worked run continued from its own sealed answer with from answers; and
/// the describe_card text of the field says so.
#[test]
fn a_start_that_carries_a_run_on_arrives_with_from_on_every_card_the_rule_holds() {
    let mut door = Stdio2::start(None);
    for (domain, card, field) in RULE_CARDS {
        let pair = json!({ "domain": domain, "card": card });
        let described = ok(&mut door, "describe_card", &pair);
        let words = described["phase_fields"].as_array().unwrap().iter().find(|row| row["name"] == field).unwrap()["bound"].as_str().unwrap().to_string();
        assert!(words.contains("At the door: a fresh start of this card states") && words.contains("refused as ContinuationWithoutUpstream"), "{card}: {words}");
        let mut run = described["example_run_trajectory"].clone();
        let mut step = described["example_compute_step"].clone();
        // Fresh: no from needed.
        let upstream = ok(&mut door, "run_trajectory", &run);
        ok(&mut door, "verify_step", &step);
        // Continued, without from: refused by name, run and step.
        continued_start(card, &mut run, &mut step);
        for (tool, request, section) in [("run_trajectory", &run, "initial"), ("verify_step", &step, "parameters")] {
            let text = refused(&mut door, tool, request);
            assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{card} {tool}: {text}");
            assert!(text.contains(&format!("'{section}.{field}' is ")), "{card} {tool}: {text}");
            assert!(!text.contains("run_seal") && !text.contains("sha256_seal"), "{card} {tool}: {text}");
        }
        // Continued from the worked run's sealed answer, with from: taken; a
        // card that serves no continuation through from takes only a fresh
        // start, and its field says so.
        assert_eq!(upstream["outcome"], "completed", "{card}");
        let mut downstream = described["example_run_trajectory"].clone();
        downstream["initial"] = handed_back(&described, &described["example_run_trajectory"], &upstream);
        downstream["from"] = json!({ "request": described["example_run_trajectory"], "run_seal": upstream["run_seal"] });
        if card == "cell_2170_model3" {
            assert!(words.contains("(ContinuationNotOnCard)"), "{card}: {words}");
            let text = refused(&mut door, "run_trajectory", &downstream);
            assert!(text.contains("(ContinuationNotOnCard)") && !text.contains("run_seal"), "{card}: {text}");
            continue;
        }
        let continued = ok(&mut door, "run_trajectory", &downstream);
        assert_eq!(continued["chain"]["kind"], "continuation", "{card}");
        assert_eq!(continued["inputs"].get("from"), None, "{card}");
    }
}

/// A fresh start of each kind passes, and a card's own start refusal is
/// named before the door's.
#[test]
fn a_fresh_start_needs_no_from_and_a_card_names_its_own_refusal_first() {
    let mut door = Stdio2::start(None);
    // The mount card at rest with no booked work: fresh, whatever its state.
    let mount = |t_s: f64, work: f64| json!({
        "domain": "signal", "card": "resonance",
        "initial": { "displacement_m": 0.0, "velocity_m_s": 0.0, "t_s": t_s, "energy_anchor_j": 0.0, "boundary_work_j": work },
        "dt_s": 0.0003, "segments": [{ "steps": 10, "load": { "accel_m_s2": 1.0 } }], "record_every": 10
    });
    ok(&mut door, "run_trajectory", &mount(0.0, 0.0));
    // t_s 0 with booked work is the card's own refusal, not the door's.
    let text = refused(&mut door, "run_trajectory", &mount(0.0, 1.0e-3));
    assert!(text.contains("LedgerNotOfThisState") && !text.contains("ContinuationWithoutUpstream"), "{text}");
    let text = refused(&mut door, "run_trajectory", &mount(0.5, 0.0));
    assert!(text.contains("ContinuationWithoutUpstream"), "{text}");
    // A 2170 cell whose carried pair equals its first case temperature is a
    // fresh start, at any temperature its block holds.
    let described = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "cell_2170_model3" }));
    let mut run = described["example_run_trajectory"].clone();
    let first = run["segments"][0]["load"]["case_temperature_c"].clone();
    run["initial"]["case_temperature_least_c"] = first.clone();
    run["initial"]["case_temperature_most_c"] = first;
    ok(&mut door, "run_trajectory", &run);
    // A grid of datasets holds each row to the rule: a continued row is
    // unadmitted by name, the others run.
    let plan = json!({
        "domain": "signal", "card": "resonance",
        "plan": { "kind": "grid", "base": mount(0.0, 0.0), "axes": [{ "path": "initial.t_s", "values": [0.0, 0.5] }] },
        "store": "none", "return": "inline"
    });
    let mut base = plan.clone();
    for key in ["domain", "card"] {
        base["plan"]["base"].as_object_mut().unwrap().remove(key);
    }
    let made = ok(&mut door, "run_dataset", &base);
    let rows = made["summary"].as_array().unwrap();
    assert_eq!(rows[0]["outcome"], "completed", "{made}");
    assert_eq!(rows[1]["outcome"], "unadmitted", "{made}");
    assert_eq!(rows[1]["reject"], "ContinuationWithoutUpstream", "{made}");
}
