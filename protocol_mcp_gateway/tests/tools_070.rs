//! The tool changes of 0.7.0: a run's readout, the outcome "ended" at an end
//! a card declares, the sentence of a refused run, decimal model values, a
//! short dataset summary, the number rule of a solve request, a solution seal
//! that covers the engine version, the comparison of solves read by
//! genesis_seal, and a tool list that leaves the catalogue to list_cards and
//! list_solvers. Every connection gives the same bytes, and nothing is stored.

mod hosted_support;
use hosted_support::answer_text;

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use hosted_support::*;
use serde_json::{Value, json};
use tokio::net::TcpListener;

const KEY: &str = "tools-070-key";

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

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| panic!("not JSON: {text}"))
}

/// The burst of the reactor card's worked example, asked for longer than it
/// lasts: it stops at the first maximum of power.
fn reactor_burst(record_every: u64) -> Value {
    json!({
        "domain": "energy", "card": "reactor_spert3",
        "initial": { "coolant_temperature_k": 300.92777777777775 },
        "dt_s": 0.0007144471176212568,
        "segments": [{ "steps": 3000, "load": { "reactivity_inserted_dollar": 0.9982818460448638 } }],
        "record_every": record_every
    })
}

fn with_readout(mut request: Value, readout: Value) -> Value {
    request["readout"] = readout;
    request
}

/// The cold plate heated past its ceiling: refused part way.
fn hot_plate() -> Value {
    json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 390.0 }, "dt_s": 1.0,
        "segments": [{ "steps": 600, "load": { "heat_power_w": 3000.0, "ambient_k": 293.15 } }],
        "record_every": 600
    })
}

fn plate_grid(answer: Option<&str>) -> Value {
    let mut request = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "grid",
            "base": { "initial": { "temperature_k": 293.15 }, "dt_s": 1.0,
                      "segments": [{ "steps": 20, "load": { "ambient_k": 293.15 } }], "record_every": 20 },
            "axes": [{ "path": "segments[0].load.heat_power_w", "values": [0.0, 2.0, 5.0] }] },
        "store": "none", "return": "inline"
    });
    if let Some(answer) = answer {
        request["answer"] = json!(answer);
    }
    request
}

#[tokio::test(flavor = "multi_thread")]
async fn every_connection_gives_the_same_bytes_for_the_changed_tools() {
    let mut stdio = Stdio2::start(None);
    let base = web().await;
    let keyed = Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY),
            "cards": [["energy", "thermal"], ["energy", "reactor_spert3"], ["energy", "reactor_spert3_fuel"]],
            "solvers": ["atom_dft"], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let open = Open::start();
    let readout = json!({ "quantities": ["power_mw", "energy_mws"], "first_past": { "quantity": "power_mw", "above": 5.0 } });
    let calls = [
        ("run_trajectory", with_readout(reactor_burst(3000), readout)),
        ("run_trajectory", hot_plate()),
        ("validate_series", json!({ "run": thermal_run(10, 293.15), "quantity": "temperature_k", "measurements": [{ "step": 5, "value": 293.2 }] })),
        ("run_dataset", plate_grid(None)),
        ("run_dataset", plate_grid(Some("states"))),
        ("solve", json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } })),
        ("describe_card", json!({ "domain": "energy", "card": "reactor_spert3" })),
    ];
    for (tool, arguments) in &calls {
        let (error, local) = stdio.tool(tool, arguments);
        assert!(!error, "{tool}: {local}");
        let (status, over_web) = post(&base, tool, arguments).await;
        assert_eq!((status, over_web.as_str()), (200, local.as_str()), "{tool} web");
        let (status, error, hosted) = keyed.tool(KEY, tool, arguments);
        assert_eq!((status, error, hosted.as_str()), (200, false, local.as_str()), "{tool} hosted");
        let (error, opened) = open.tool(tool, arguments);
        assert_eq!((error, opened.as_str()), (false, local.as_str()), "{tool} open");
    }
}

#[test]
fn a_readout_gives_decimals_extremes_and_a_level_passed_and_leaves_the_proof_alone() {
    let mut stdio = Stdio2::start(None);
    let (_, plain) = stdio.tool("run_trajectory", &reactor_burst(1));
    let readout = json!({ "quantities": ["power_mw", "energy_mws"], "first_past": { "quantity": "power_mw", "above": 5.0 } });
    let asked = with_readout(reactor_burst(1), readout.clone());
    let (error, text) = stdio.tool("run_trajectory", &asked);
    assert!(!error, "{text}");
    let (plain, read) = (parsed(&plain), parsed(&text));
    // The proof is that of the same run without the readout.
    for key in ["inputs", "outcome", "steps_landed", "reject", "final_state", "state_digest", "frames", "run_seal"] {
        assert_eq!(read[key], plain[key], "{key}");
    }
    assert!(read["inputs"].get("readout").is_none());
    assert_ne!(read["receipt_sha256"], plain["receipt_sha256"]);
    let (_, receipt) = stdio.tool("verify_receipt", &json!({ "response_text": text }));
    assert_eq!(parsed(&receipt)["matches"], json!(true), "{receipt}");
    // A check by request with the readout in the request holds.
    let (_, check) = stdio.tool("verify_tape", &json!({ "request": asked, "run_seal": plain["run_seal"] }));
    assert_eq!(parsed(&check)["holds"], json!(true), "{check}");

    let out = &read["readout"];
    let landed = read["steps_landed"].as_u64().unwrap();
    let steps = out["steps"].as_array().unwrap();
    assert_eq!(steps.len() as u64, landed);
    let power = &out["quantities"][0];
    assert_eq!(power["quantity"], "power_mw");
    let values: Vec<f64> = power["values"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    // The value at the last landed step is the final state's.
    assert_eq!(values.last().copied(), read["final_state"]["power_mw"].as_f64());
    let greatest = values.iter().cloned().fold(f64::MIN, f64::max);
    assert_eq!(power["maximum"].as_f64(), Some(greatest));
    let at = power["maximum_step"].as_u64().unwrap();
    assert_eq!(values[at as usize - 1], greatest);
    let dt = asked["dt_s"].as_f64().unwrap();
    assert_eq!(power["maximum_time_s"].as_f64(), Some(at as f64 * dt));
    // The first landed step past the level, and none before it.
    let first = out["first_past"]["step"].as_u64().unwrap();
    assert!(values[first as usize - 1] > 5.0);
    assert!(values[..first as usize - 1].iter().all(|value| *value <= 5.0));
    assert_eq!(out["first_past"]["value"].as_f64(), Some(values[first as usize - 1]));
    // With one recorded state, the extremes are still those of every step.
    let (_, sparse) = stdio.tool("run_trajectory", &with_readout(reactor_burst(3000), readout));
    let sparse = parsed(&sparse);
    assert_eq!(sparse["readout"]["steps"], json!([landed]));
    assert_eq!(sparse["readout"]["quantities"][0]["maximum"], power["maximum"]);
    assert_eq!(sparse["readout"]["first_past"], out["first_past"]);
    assert_eq!(sparse["run_seal"].as_str().map(str::len), Some(64));
}

#[test]
fn a_readout_that_cannot_be_read_is_refused_by_name_before_the_run() {
    let mut stdio = Stdio2::start(None);
    let cases = [
        (json!({ "quantities": ["no_such"] }), "'readout.quantities' names 'no_such', which the reactor_spert3 card does not publish as a number."),
        (json!({ "quantities": ["past_first_maximum"] }), "a flag"),
        (json!({ "quantities": [] }), "at least one quantity"),
        (json!({ "quantities": ["power_mw"], "extra": 1 }), "'readout.extra' is not a field of a readout."),
        (json!({ "quantities": ["power_mw"], "first_past": { "quantity": "energy_mws", "above": 1.0 } }), "which readout.quantities does not name"),
        (json!({ "quantities": ["power_mw"], "first_past": { "quantity": "power_mw", "above": 1.0, "below": 0.0 } }), "one of above or below"),
        (json!({ "quantities": ["power_mw", "power_mw"] }), "twice"),
    ];
    for (readout, words) in cases {
        let (error, text) = stdio.tool("run_trajectory", &with_readout(reactor_burst(3000), readout.clone()));
        assert!(error, "{readout}: {text}");
        assert!(text.contains(words), "{readout}: {text}");
    }
    // A 3-vector is read one component at a time.
    let satellite = json!({ "quantities": ["position_m"] });
    let (_, described) = stdio.tool("describe_card", &json!({ "domain": "aerospace", "card": "satellite_lageos1" }));
    let run = parsed(&described)["example_run_trajectory"].clone();
    let (error, text) = stdio.tool("run_trajectory", &with_readout(run.clone(), satellite));
    assert!(error && text.contains("name one component, position_m[0]"), "{text}");
    let (error, text) = stdio.tool("run_trajectory", &with_readout(run, json!({ "quantities": ["position_m[2]"] })));
    assert!(!error, "{text}");
    assert_eq!(parsed(&text)["readout"]["quantities"][0]["quantity"], "position_m[2]");
    // validate_series takes a run without a readout.
    let (error, text) = stdio.tool(
        "validate_series",
        &json!({ "run": with_readout(thermal_run(10, 293.15), json!({ "quantities": ["temperature_k"] })), "quantity": "temperature_k", "measurements": [{ "step": 5, "value": 293.2 }] }),
    );
    assert!(error && text.contains("'readout' is not a field of this request."), "{text}");
}

#[test]
fn a_declared_end_is_ended_and_every_other_refusal_names_its_steps() {
    let mut stdio = Stdio2::start(None);
    let (_, text) = stdio.tool("run_trajectory", &reactor_burst(3000));
    let burst = parsed(&text);
    let landed = burst["steps_landed"].as_u64().unwrap();
    assert_eq!(burst["outcome"], "ended");
    assert_eq!(burst["reject"], "PastFirstMaximum");
    assert_eq!(
        burst["reject_detail"],
        format!("Ended: the run reached the end the reactor_spert3 card declares (PastFirstMaximum): the first maximum of power, where the card's burst ends. The step past it is not taken, and the run is sealed to step {landed}.")
    );
    let (_, check) = stdio.tool("verify_tape", &burst);
    assert_eq!(parsed(&check)["holds"], json!(true), "{check}");
    let mut renamed = burst.clone();
    renamed["outcome"] = json!("refused");
    let (_, check) = stdio.tool("verify_tape", &renamed);
    assert_eq!(parsed(&check)["cause"], json!("run_seal"), "{check}");

    // The capsule stops at the drogue-deployment condition the radar recorded.
    let (_, described) = stdio.tool("describe_card", &json!({ "domain": "aerospace", "card": "capsule_stardust" }));
    let described = parsed(&described);
    assert_eq!(described["declared_ends"][0]["reject"], "PastFlightDeployment");
    assert_eq!(described["declared_ends"].as_array().unwrap().len(), 1);
    let mut entry = described["example_run_trajectory"].clone();
    entry["segments"][0]["steps"] = json!(20000);
    entry["record_every"] = json!(20000);
    let (_, capsule) = stdio.tool("run_trajectory", &entry);
    let capsule = parsed(&capsule);
    assert_eq!(capsule["outcome"], "ended", "{}", capsule["reject_detail"]);
    assert_eq!(capsule["reject"], "PastFlightDeployment");
    assert!(capsule["final_state"]["altitude_m"].as_f64().unwrap() >= 30_000.0);
    // A limit added because a region was never compared is a refusal, never an end.
    let (_, pad) = stdio.tool("describe_card", &json!({ "domain": "materials", "card": "fingerpad_p02_glass" }));
    assert_eq!(parsed(&pad)["declared_ends"], json!([]));
    let (_, insight) = stdio.tool("describe_card", &json!({ "domain": "aerospace", "card": "capsule_insight" }));
    let insight = parsed(&insight);
    assert_eq!(insight["declared_ends"].as_array().unwrap().len(), 1);
    assert_eq!(insight["declared_ends"][0]["reject"], "PastFlightDeployment");

    // The plate past its ceiling is refused, and the sentence names the steps.
    let (_, hot) = stdio.tool("run_trajectory", &hot_plate());
    let hot = parsed(&hot);
    let landed = hot["steps_landed"].as_u64().unwrap();
    assert_eq!(hot["outcome"], "refused");
    assert_eq!(
        hot["reject_detail"],
        format!("Physical Failure: the plate left 233.15 K to 398.15 K (TemperatureOutsideCard). The run is sealed to step {landed}. The state at step {} was refused and is not sealed.", landed + 1)
    );
    let (_, plate) = stdio.tool("describe_card", &json!({ "domain": "energy", "card": "thermal" }));
    assert_eq!(parsed(&plate)["declared_ends"], json!([]));

    // A dataset counts an ended row apart from a refused one.
    let request = json!({
        "domain": "energy", "card": "reactor_spert3",
        "plan": { "kind": "list", "runs": [{
            "initial": { "coolant_temperature_k": 300.92777777777775 }, "dt_s": 0.0007144471176212568,
            "segments": [{ "steps": 3000, "load": { "reactivity_inserted_dollar": 0.9982818460448638 } }], "record_every": 3000 }] },
        "store": "none", "return": "inline"
    });
    let (error, text) = stdio.tool("run_dataset", &request);
    assert!(!error, "{text}");
    let answer = parsed(&text);
    assert_eq!((answer["head"]["ended"].clone(), answer["head"]["refused"].clone()), (json!(1), json!(0)));
    assert_eq!(answer["summary"][0]["outcome"], "ended");
    assert_eq!(answer["summary"][0]["run_seal"], burst["run_seal"]);
    let (_, check) = stdio.tool("verify_dataset", &json!({ "request": request, "dataset_seal": answer["head"]["dataset_seal"] }));
    assert_eq!(parsed(&check)["holds"], json!(true), "{check}");
}

#[test]
fn validate_series_gives_the_model_values_in_decimals_beside_the_hex() {
    let mut stdio = Stdio2::start(None);
    let (_, text) = stdio.tool(
        "validate_series",
        &json!({ "run": thermal_run(10, 293.15), "quantity": "temperature_k", "measurements": [{ "step": 2, "value": 293.0 }, { "step": 5, "value": 293.2 }] }),
    );
    let answer = parsed(&text);
    let decimals = answer["model_values"].as_array().unwrap();
    let hexes = answer["model_values_hex"].as_array().unwrap();
    assert_eq!(decimals.len(), 2);
    for (decimal, hex) in decimals.iter().zip(hexes) {
        let bits = u64::from_str_radix(hex.as_str().unwrap(), 16).unwrap();
        assert_eq!(decimal.as_f64().unwrap().to_bits(), bits);
    }
}

#[test]
fn the_summary_is_short_by_default_and_gives_states_when_asked() {
    let mut stdio = Stdio2::start(None);
    let (_, short) = stdio.tool("run_dataset", &plate_grid(None));
    let (_, states) = stdio.tool("run_dataset", &plate_grid(Some("states")));
    let (_, full) = stdio.tool("run_dataset", &plate_grid(Some("full")));
    let (short, states, full) = (parsed(&short), parsed(&states), parsed(&full));
    assert_eq!(short["head"], full["head"]);
    assert_eq!(states["head"], full["head"]);
    let row = short["summary"][1].as_object().unwrap();
    assert_eq!(row.keys().collect::<Vec<_>>(), ["row", "set", "outcome", "steps_landed", "reject", "run_seal"]);
    assert_eq!(row["set"], json!({ "segments[0].load.heat_power_w": 2.0 }));
    assert_eq!(row["run_seal"], full["rows"][1]["run_seal"]);
    let state = states["summary"][1]["final_state"].as_object().unwrap();
    assert!(state.contains_key("temperature_k") && state.keys().all(|key| !key.ends_with("_hex")));
    let (error, text) = stdio.tool("run_dataset", &{
        let mut wrong = plate_grid(None);
        wrong["answer"] = json!("rows");
        wrong
    });
    assert!(error && text.contains("\"summary\", \"states\" or \"full\""), "{text}");
    let (error, text) = stdio.tool(
        "run_dataset",
        &json!({ "solver": "atom_dft", "plan": { "kind": "list", "solves": [{ "parameters": { "atomic_number": 1 } }] }, "answer": "states" }),
    );
    assert!(error && text.contains("'answer' \"states\" is for a plan of runs"), "{text}");
}

#[test]
fn a_subnormal_solve_input_is_refused_by_the_number_rule() {
    let mut stdio = Stdio2::start(None);
    let (_, described) = stdio.tool("describe_solver", &json!({ "solver": "fusion_pb11_lhd" }));
    let described = parsed(&described);
    let rule = described["number_rule"].as_str().unwrap();
    assert!(rule.contains("a subnormal such as 5e-324 is refused"), "{rule}");
    let mut request = described["example_solve"].clone();
    request["parameters"]["boron_density_per_m3"] = json!(5e-324);
    let (error, text) = stdio.tool("solve", &request);
    assert!(error, "{text}");
    assert_eq!(text, "Physical Violation: 'boron_density_per_m3' is not a finite normal number.");
    request["parameters"]["boron_density_per_m3"] = json!(0.0);
    let (error, text) = stdio.tool("solve", &request);
    assert!(!text.contains("finite normal"), "{error} {text}");
}

#[test]
fn a_solution_claimed_for_another_engine_version_does_not_hold() {
    let mut stdio = Stdio2::start(None);
    let (_, text) = stdio.tool("solve", &json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } }));
    let mut body = parsed(&text);
    let (_, report) = stdio.tool("verify_solution", &body);
    assert_eq!(parsed(&report)["holds"], json!(true), "{report}");
    // Since 0.11.0 the long form compares the whole answer: one sent without
    // its receipt is named there.
    body.as_object_mut().unwrap().remove("receipt_sha256");
    let (_, report) = stdio.tool("verify_solution", &body);
    assert_eq!((parsed(&report)["cause"].clone(), parsed(&report)["differs_at"].clone()), (json!("answer_text"), json!("receipt_sha256")), "{report}");
    body["engine_version"] = json!("genesis_solvers_v0.4.0");
    let (_, report) = stdio.tool("verify_solution", &body);
    assert_eq!(parsed(&report)["holds"], json!(false), "{report}");
    assert_eq!(parsed(&report)["cause"], json!("engine_version"), "{report}");
}

#[test]
fn genesis_seal_reads_a_comparison_of_solves() {
    let mut stdio = Stdio2::start(None);
    let comparison = json!({
        "solves": { "solver": "atom_dft", "plan": { "kind": "grid", "base": { "parameters": {} },
                    "axes": [{ "path": "parameters.atomic_number", "values": [1, 2, 19] }] } },
        "quantity": "total_energy_ha",
        "measurements": [{ "row": 0, "value": -0.5 }, { "row": 1, "value": -2.9 }, { "row": 2, "value": -600.0 }]
    });
    let (error, text) = stdio.tool("validate_series", &comparison);
    assert!(!error, "{text}");
    let answer = parsed(&text);
    assert_eq!(answer["model_values"].as_array().map(Vec::len), Some(2));
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("tools_070");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("solves_comparison.json");
    std::fs::write(&file, &text).unwrap();
    let program = seal_program();
    let output = Command::new(&program).arg("verify-validation").arg(&file).output().unwrap();
    let printed = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{printed}");
    assert!(printed.contains("holds: true"), "{printed}");
    assert!(printed.contains("receipt: matches"), "{printed}");
    assert!(printed.contains(&format!("validation_seal: {}", answer["validation_seal"].as_str().unwrap())), "{printed}");
    // A changed dataset seal is named by the seal.
    let changed = text.replacen(answer["dataset_seal"].as_str().unwrap(), &"ab".repeat(32), 1);
    std::fs::write(&file, &changed).unwrap();
    let output = Command::new(&program).arg("verify-validation").arg(&file).output().unwrap();
    let printed = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{printed}");
    assert!(printed.contains("cause: validation_seal"), "{printed}");
}

/// The most characters the declaration of every tool comes to, compact.
const TOOL_LIST_CHARS: usize = 63_761;

#[test]
fn the_tool_list_declares_the_pair_and_the_solver_once_and_lists_no_catalogue() {
    let tools = protocol_mcp_gateway::mcp_server::tools_in(&protocol_mcp_gateway::view::View::full());
    assert_eq!(tools.len(), 18);
    let text = serde_json::to_string(&tools).unwrap();
    // 0.6.0 sent 42,886 characters (json.dumps), 12,474 of them the catalogue repeated.
    // Chained proofs declare from on run_trajectory and verify_step and the
    // chain check on verify_tape, and the list grew from under 33,000 to 34,021.
    // The changes held in tests/tools_071.rs add the readout's at_minimum,
    // at_maximum and at_step, the counts, solve's example and the short
    // check's not_held and text_not_compared: 35,595.
    // 0.8.1 says in verify_tape's and verify_solution's descriptions that the
    // check of a copy pairs verify_receipt with recomputed_receipt_sha256: 35,847.
    // 0.9.1 adds three tools (cite, find_model, find_flip) and changes no
    // earlier tool's text: 40,417.
    // 0.9.2 rewords two of the thirteen earlier declarations: run_trajectory
    // says that steps_landed counts the steps that landed, so the refused
    // step is steps_landed + 1, and verify_tape says the request is sent
    // with its readout and its from when it had them: 36,072 for the
    // thirteen, 40,642 for all sixteen.
    // 0.9.3 says in list_cards, list_solvers and find_model what
    // against_measurement is, and in describe_card and describe_solver that
    // a description has a short form, and declares form on both: 37,762 for
    // the thirteen, 42,548 for all sixteen.
    // 0.10.0 says in cite that the service and the paper come first and the
    // people whose measurements the model stands on after them, with no
    // licence among what each gives: 42,599 for all sixteen.
    // 0.10.1 holds every description to 2,000 characters, the length a client
    // shows: the descriptions fall from 21,517 characters to 15,149, and what
    // they no longer hold is said at the inputs it is about, so the input
    // schemas grow from 19,798 to 28,501 (the worked plans, how a plan is
    // written and an answer counted, how a run comes back in full); each tool
    // gains a title, and no description names a route of the web connection:
    // 39,840 for the thirteen and 44,965 for all sixteen. The input schemas
    // then stop naming a route of the web connection (verify_step, solve) and
    // say "stored as files" where they said "the gateway stored" (run_dataset,
    // verify_dataset): 39,721 for the thirteen, 44,846 for all sixteen. The
    // three parts of 0.10.1 together then say, in run_trajectory's inputs, what a
    // refusal sentence with no name in round brackets is; in verify_dataset,
    // that row and fault name the first row changed, swapped, dropped or
    // repeated and a decimal that differs from its hex; and in find_model, that
    // a model is set aside when the description names another thing of its
    // kind: 40,050 for the thirteen, 45,312 for all sixteen. The next
    // version serves compute_step, and verify_step beside it for one version
    // as its earlier name with the same input schema; says at the inputs of
    // the step and the solve the names of an input left out (InputMissing)
    // and of one given with a choice it does not go with
    // (InputNotWithChoice); words the verifying tools without "check"; and
    // names the fifth word of against_measurement, "held to a reference
    // calculation", in list_cards, list_solvers and find_model; and, since
    // the door's fixes of 0.11.0, says that the long form of each verifying
    // tool writes the answer again and compares it, and names UnknownCard and
    // UnknownSolver: 47,814 for all seventeen. A signed load (wave 3) then
    // declares signed at a segment, at a measurement of validate_series and
    // at a lone solver's parameters, says once at run_trajectory's segments
    // what it holds and how it is verified and what load_source says, says
    // at a solve's parameters that they may be signed, and adds a sentence
    // to run_trajectory and to verify_tape; engine 0.12.0 adds design_run,
    // inverse design, served before verify_step: 56,187 for all eighteen.
    // 0.12.0's fix pass says that every long form computes the request its
    // answer carries again and compares the whole answer byte for byte, and
    // names AnswerCarriesNoRequest: 57,766 for all eighteen. Each long form's
    // answer then says values_hold beside holds, in the three descriptions
    // and schemas: 59,530 for all eighteen, reproduced in Python from the
    // stdio door's tools/list as compact JSON. validate_series then declares
    // commitment, a salted SHA-256 of the measurements opened before
    // computing, at its input and in a sentence of its description: 61,345
    // for all eighteen. run_trajectory's examples of a declared end then
    // name ends the engine declares (a crater that stops growing, a runaway
    // once every reactant is consumed): 61,364 for all eighteen, reproduced
    // the same way. list_cards, list_solvers and find_model then name the
    // sixth word of against_measurement, "calibrated to the record", and say
    // what it means: 62,163 for all eighteen, reproduced the same way.
    // verify_receipt then says a response's engine_version is said beside
    // matches, with whether it is this engine's: 62,265 for all eighteen,
    // reproduced the same way. list_cards, list_solvers and find_model then
    // say what met, met in part and missed say, against the pass marks a
    // record states, the record saying whether the comparison was blind, and
    // what calibrated to the record is (0.13.1): 62,922 for all eighteen,
    // reproduced the same way. The upstream request in from is said to go
    // without its readout (run_trajectory's and compute_step's from, and
    // verify_tape), and run_dataset's schema says the 40000 characters of an
    // inline answer limit its rows first (the words of 0.14.0):
    // 63,291 for all eighteen, reproduced the same way
    // (evidence/verification/words_014_2026-10-16/pins_words_014.txt).
    // find_flip says its counted FLIP_MAX_SECONDS in its description and its
    // request's schema, and run_dataset's schema says about 140 summary rows
    // of a grid on one axis and fewer of a list (0.14.0 rc2, items 1 and 7):
    // 63,721 (rc2_014/measure_rc2/pins_item11.txt); and find_flip's request
    // schema says a search makes at least three solves (item 1, continued):
    // TOOL_LIST_CHARS for all eighteen, reproduced the same way
    // (rc2_014/measure_rc2/pins_item01b.txt).
    assert!(text.len() <= TOOL_LIST_CHARS, "{} characters", text.len());
    let earlier: Vec<_> = tools.iter().take(13).collect();
    // 40,050 as declared since 0.10.1; 40,433 once compute_step and the
    // verifying tools are worded without "check", the step and the solve
    // say InputMissing and InputNotWithChoice at their inputs, and the
    // fifth word of against_measurement is named; 41,038 once the long form
    // of each verifying tool says it writes the answer again and compares it,
    // and the step and the solve name UnknownCard and UnknownSolver; 41,059
    // once each link of a chain is held as the long form holds one answer;
    // 45,913 once run_trajectory declares and describes signed and
    // load_source, a lone solver's parameters declare signed, solve's
    // parameters say they may be signed, and verify_tape names load_signature;
    // 47,477 once the long form of each verifying tool computes the answer's
    // request again and compares it byte for byte, and verify_tape's schema
    // names AnswerCarriesNoRequest; 49,241 once each long form's answer says
    // values_hold beside holds, in the three descriptions and schemas; 51,056
    // once validate_series declares and describes commitment; 51,075 once
    // run_trajectory's examples of a declared end name ends the engine
    // declares; 51,613 once list_cards and list_solvers name the sixth word
    // of against_measurement, "calibrated to the record", and say what it
    // means; 51,715 once verify_receipt says it names a response's
    // engine_version beside matches; 52,169 once list_cards and list_solvers
    // say what the three words and the sixth say (0.13.1); 52,538 once the
    // upstream request in from goes without its readout and an inline answer's
    // 40000 characters are said to limit its rows (the words of 0.14.0);
    // 52,614 once run_dataset's schema says those rows of a grid on one axis
    // and of a list (0.14.0 rc2, item 7).
    assert_eq!(serde_json::to_string(&earlier).unwrap().len(), 52_614, "the thirteen earlier tools as declared in the next version");
    for name in ["ti64_lattice", "reactor_triga_jsi", "tokamak_dt_beam_target", "culture_trichoderma_ambr250"] {
        assert!(!text.contains(name), "the tool list names {name}");
    }
}

fn seal_program() -> std::path::PathBuf {
    let seal_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join("../genesis_seal");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--offline", "--quiet", "--manifest-path"])
        .arg(seal_crate.join("Cargo.toml"))
        .status()
        .unwrap();
    assert!(built.success());
    [
        seal_crate.join("target/debug/genesis_seal"),
        seal_crate.join("../target/debug/genesis_seal"),
    ]
    .into_iter()
    .find(|path| path.exists())
    .expect("the program of the sealing crate")
}
