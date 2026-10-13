//! The answer of run_dataset: a small summary by default and the full rows on
//! request, under the same seal; a grid admitted row by row; an axis that
//! replaces a base value; a plan of solves under one dataset seal; and the
//! comparison of one result of a plan of solves with measurements. Every
//! connection gives the same bytes, and nothing is stored.

mod hosted_support;

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use hosted_support::*;
use serde_json::{Value, json};
use tokio::net::TcpListener;

const KEY: &str = "dataset-answers-key";

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

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| panic!("not JSON: {text}"))
}

/// A grid on the cold plate whose second start temperature is past the
/// card's bound, and whose base also sets the power an axis sets.
fn card_grid(answer: Option<&str>) -> Value {
    let mut request = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "grid",
            "base": { "initial": {}, "dt_s": 1.0,
                      "segments": [{ "steps": 30, "load": { "ambient_k": 293.15, "heat_power_w": 2.0 } }], "record_every": 10 },
            "axes": [{ "path": "initial.temperature_k", "values": [283.15, 400.0] },
                     { "path": "segments[0].load.heat_power_w", "values": [0.0, 5.0] }] },
        "store": "frames", "return": "inline"
    });
    if let Some(answer) = answer {
        request["answer"] = json!(answer);
    }
    request
}

/// Three atoms, the third past the solver's bound.
fn solve_grid(answer: Option<&str>) -> Value {
    let mut request = json!({
        "solver": "atom_dft",
        "plan": { "kind": "grid", "base": { "parameters": {} },
                  "axes": [{ "path": "parameters.atomic_number", "values": [1, 2, 19] }] }
    });
    if let Some(answer) = answer {
        request["answer"] = json!(answer);
    }
    request
}

fn comparison() -> Value {
    json!({
        "solves": solve_grid(None),
        "quantity": "total_energy_ha",
        "measurements": [{ "row": 0, "value": -0.5 }, { "row": 1, "value": -2.9 }, { "row": 2, "value": -600.0 }]
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn every_connection_gives_the_same_bytes_for_the_new_answers() {
    let mut stdio = Stdio2::start(None);
    let base = web().await;
    let keyed = Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY), "cards": [["energy", "thermal"]],
            "solvers": ["atom_dft"], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let open = Open::start();

    let (error, summary) = stdio.tool("run_dataset", &card_grid(None));
    assert!(!error, "{summary}");
    let (error, solves) = stdio.tool("run_dataset", &solve_grid(None));
    assert!(!error, "{solves}");
    let card_seal = parsed(&summary)["head"]["dataset_seal"].clone();
    let solve_seal = parsed(&solves)["head"]["dataset_seal"].clone();
    let calls = [
        ("run_dataset", card_grid(None)),
        ("run_dataset", card_grid(Some("full"))),
        ("run_dataset", solve_grid(None)),
        ("run_dataset", solve_grid(Some("full"))),
        ("verify_dataset", json!({ "request": card_grid(Some("full")), "dataset_seal": card_seal })),
        ("verify_dataset", json!({ "request": solve_grid(None), "dataset_seal": solve_seal })),
        ("validate_series", comparison()),
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
        if tool.starts_with("verify") {
            assert_eq!(parsed(&local)["holds"], json!(true), "{local}");
        }
    }
}

#[test]
fn the_summary_is_small_and_the_full_rows_carry_the_same_head() {
    let mut stdio = Stdio2::start(None);
    let (_, summary) = stdio.tool("run_dataset", &card_grid(None));
    let (_, full) = stdio.tool("run_dataset", &card_grid(Some("full")));
    let (summary_answer, full_answer) = (parsed(&summary), parsed(&full));
    assert_eq!(summary_answer["head"], full_answer["head"]);
    assert!(summary.len() * 2 < full.len(), "{} against {}", summary.len(), full.len());
    // The answer form is not sealed: the head's request leaves it out.
    assert!(full_answer["head"]["request"].get("answer").is_none());
    // The base value the axis sets is replaced, and the answer says so.
    assert_eq!(summary_answer["base_replaced"], json!(["segments[0].load.heat_power_w"]));
    assert_eq!(full_answer["base_replaced"], json!(["segments[0].load.heat_power_w"]));
    let rows = summary_answer["summary"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows[1]["set"],
        json!({ "initial.temperature_k": 283.15, "segments[0].load.heat_power_w": 5.0 })
    );
    assert_eq!(rows[0]["outcome"], json!("completed"));
    assert_eq!(rows[0]["steps_landed"], json!(30));
    assert_eq!(rows[0]["run_seal"], full_answer["rows"][0]["run_seal"]);
    // The final state is in a summary row only when asked for.
    assert!(rows[0].get("final_state").is_none(), "{}", rows[0]);
    let (_, states) = stdio.tool("run_dataset", &card_grid(Some("states")));
    let states = parsed(&states);
    assert_eq!(states["head"], full_answer["head"]);
    let state = states["summary"][0]["final_state"].as_object().unwrap();
    assert!(state.contains_key("temperature_k"));
    assert!(state.keys().all(|key| !key.ends_with("_hex")), "{state:?}");
    // The grid is admitted row by row: the two rows past the bound are
    // unadmitted with the door's sentence, and the others run.
    for index in [2, 3] {
        assert_eq!(rows[index]["outcome"], json!("unadmitted"), "{}", rows[index]);
        assert_eq!(rows[index]["run_seal"], Value::Null);
        let sentence = full_answer["rows"][index]["reject_detail"].as_str().unwrap();
        assert!(sentence.contains("temperature_k"), "{sentence}");
    }
    assert_eq!(summary_answer["head"]["unadmitted"], json!(2));
    assert_eq!(summary_answer["head"]["completed"], json!(2));
    // The full form is checked by its parts as before.
    let (error, report) = stdio.tool(
        "verify_dataset",
        &json!({ "head": full_answer["head"], "rows": full_answer["rows"], "frames_hex": full_answer["frames_hex"] }),
    );
    assert!(!error, "{report}");
    assert_eq!(parsed(&report)["holds"], json!(true), "{report}");
}

#[test]
fn a_plan_sent_with_and_without_the_replaced_base_value_is_one_dataset() {
    let mut stdio = Stdio2::start(None);
    let mut without = card_grid(None);
    without["plan"]["base"]["segments"][0]["load"].as_object_mut().unwrap().remove("heat_power_w");
    let (_, with_value) = stdio.tool("run_dataset", &card_grid(None));
    let (_, plain) = stdio.tool("run_dataset", &without);
    assert_eq!(parsed(&with_value)["head"], parsed(&plain)["head"]);
    assert!(parsed(&plain).get("base_replaced").is_none(), "{plain}");
}

#[test]
fn an_inline_answer_above_its_limit_is_refused_before_anything_runs() {
    let mut stdio = Stdio2::start(None);
    let values: Vec<Value> = (0..30).map(|step| json!(250.0 + step as f64)).collect();
    let mut request = json!({
        "domain": "energy", "card": "thermal",
        "plan": { "kind": "grid",
            "base": { "initial": {}, "dt_s": 1.0,
                      "segments": [{ "steps": 30, "load": { "ambient_k": 293.15, "heat_power_w": 2.0 } }], "record_every": 10 },
            "axes": [{ "path": "initial.temperature_k", "values": values }] },
        "store": "none", "return": "inline", "answer": "full"
    });
    let (error, text) = stdio.tool("run_dataset", &request);
    assert!(error, "{text}");
    assert!(text.contains("more than the 40000 one inline answer may have"), "{text}");
    assert!(text.contains("leave out 'answer'"), "{text}");
    assert!(text.contains("Nothing was run."), "{text}");
    request.as_object_mut().unwrap().remove("answer");
    let (error, text) = stdio.tool("run_dataset", &request);
    assert!(!error, "{text}");
    assert!(text.len() < 40_000, "{}", text.len());
}

#[test]
fn a_plan_of_solves_is_sealed_row_by_row_and_compared_with_measurements() {
    let mut stdio = Stdio2::start(None);
    let (error, text) = stdio.tool("run_dataset", &solve_grid(Some("full")));
    assert!(!error, "{text}");
    let full = parsed(&text);
    let rows = full["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(full["head"]["completed"], json!(2));
    assert_eq!(full["head"]["unadmitted"], json!(1));
    assert_eq!(rows[2]["outcome"], json!("unadmitted"));
    // Each row carries the seal solve gives its request alone.
    let (_, alone) = stdio.tool("solve", &json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } }));
    assert_eq!(rows[1]["solution_seal"], parsed(&alone)["solution_seal"]);
    assert_eq!(rows[1]["leaf"]["solution_seal"], parsed(&alone)["solution_seal"]);
    let (_, summary) = stdio.tool("run_dataset", &solve_grid(None));
    let summary = parsed(&summary);
    assert_eq!(summary["head"], full["head"]);
    assert_eq!(summary["summary"][1]["set"], json!({ "parameters.atomic_number": 2 }));
    assert!(summary["summary"][1]["result"]["total_energy_ha"].is_number());
    // A dataset of solves is checked by request, not by parts.
    let (error, text) = stdio.tool(
        "verify_dataset",
        &json!({ "head": full["head"], "rows": full["rows"], "frames_hex": null }),
    );
    assert!(error, "{text}");
    assert!(text.contains("Check it by sending its run_dataset request"), "{text}");
    // A store or a return is not a field of a plan of solves.
    let mut stored = solve_grid(None);
    stored["return"] = json!("file");
    let (error, text) = stdio.tool("run_dataset", &stored);
    assert!(error && text.contains("is not stored"), "{text}");

    let (error, text) = stdio.tool("validate_series", &comparison());
    assert!(!error, "{text}");
    let compared = parsed(&text);
    assert_eq!(compared["dataset_seal"], full["head"]["dataset_seal"]);
    assert_eq!(compared["compared"], json!(2));
    assert_eq!(compared["not_reached"], json!(1));
    assert_eq!(compared["not_reached_rows"], json!([2]));
    assert_eq!(compared["model_values_hex"][1], rows[1]["result"]["total_energy_ha_hex"]);
    assert!(compared["validation_seal"].as_str().unwrap().len() == 64);
    assert!(compared["sealed_as"].as_str().unwrap().contains("in the place of a run seal"));
    let (error, receipt) = stdio.tool("verify_receipt", &json!({ "response_text": text }));
    assert!(!error, "{receipt}");
    assert_eq!(parsed(&receipt)["matches"], json!(true), "{receipt}");
    // An array or an unpublished name is refused by name.
    let mut wrong = comparison();
    wrong["quantity"] = json!("no_such_number");
    let (error, text) = stdio.tool("validate_series", &wrong);
    assert!(error && text.contains("does not publish as a number of its result"), "{text}");
}

#[test]
fn the_description_says_where_steps_come_from_and_what_the_answer_holds() {
    let tools = protocol_mcp_gateway::mcp_server::tools_in(&protocol_mcp_gateway::view::View::full());
    let description = |name: &str| tools.iter().find(|tool| tool.name == name).unwrap().description.clone().unwrap().to_string();
    let run = description("run_dataset");
    assert!(run.contains("steps and record_every are not paths and cannot vary: every row of a grid or a sample takes them from the base run."), "{run}");
    assert!(run.contains("replaces the base's value of that field"), "{run}");
    assert!(run.contains("one summary row per run"), "{run}");
    assert!(run.contains("A plan of solves names solver"), "{run}");
    assert!(run.contains("Each row of a grid is held to the card's bounds alone"), "{run}");
    assert!(description("validate_series").contains("In place of run, solves takes a plan of solves"));
}
