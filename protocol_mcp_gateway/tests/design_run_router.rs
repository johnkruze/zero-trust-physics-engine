//! design_run, inverse design, on every connection: a target end state in,
//! the loads of each segment out. Every connection gives the same bytes; the
//! timeline found is an ordinary run that verify_tape recomputes; the
//! candidates are one run_dataset plan that verify_dataset recomputes; the
//! answer carries a receipt and a design seal. A forged design does not
//! hold, and the search moves no value to a limit and fills nothing in.

mod hosted_support;

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

use hosted_support::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha384};
use tokio::net::TcpListener;

const KEY: &str = "design-run-key";

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

fn plate(power: f64) -> Value {
    json!({
        "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 10.0,
        "segments": [{ "steps": 60, "load": { "heat_power_w": power, "ambient_k": 293.15 } }], "record_every": 60
    })
}

/// 315 K at 600 s with the heater from 0 to 40 W.
fn plate_design() -> Value {
    json!({
        "request": plate(0.0),
        "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 40.0 }],
        "target": { "quantity": "temperature_k", "value": 315.0, "tolerance": 0.01 }
    })
}

/// 390 K at 600 s with the heater up to 200 W: the high end overheats the
/// plate past its 398.15 K ceiling and is refused part way.
fn ceiling_design() -> Value {
    json!({
        "request": plate(0.0),
        "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 200.0 }],
        "target": { "quantity": "temperature_k", "value": 390.0, "tolerance": 0.01 }
    })
}

/// The fuse link brought to 300 K at 2 ms by a load current from 0 to 40 A.
fn fuse_design() -> Value {
    json!({
        "request": {
            "domain": "energy", "card": "grid_node",
            "initial": { "current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15 }, "dt_s": 1e-6,
            "segments": [{ "steps": 2000, "load": { "upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 293.15 } }],
            "record_every": 2000
        },
        "vary": [{ "place": "segments[0].load.load_current_a", "low": 0.0, "high": 40.0 }],
        "target": { "quantity": "temperature_k", "value": 300.0, "tolerance": 0.01 }
    })
}

fn keyed() -> Hosted {
    Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY),
            "cards": [["energy", "thermal"], ["energy", "grid_node"]],
            "solvers": [], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn design_run_gives_the_same_bytes_on_every_connection() {
    let mut stdio = Stdio2::start(None);
    let base = web().await;
    let keyed = keyed();
    let open = Open::start();
    let outside = json!({
        "request": plate(0.0),
        "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 40.0 }],
        "target": { "quantity": "temperature_k", "value": 450.0, "tolerance": 0.01 }
    });
    for (arguments, error_due) in [(plate_design(), false), (ceiling_design(), false), (fuse_design(), false), (outside, true)] {
        let (error, local) = stdio.tool("design_run", &arguments);
        assert_eq!(error, error_due, "{local}");
        let (status, over_web) = post(&base, "design_run", &arguments).await;
        assert_eq!(over_web, local, "web");
        assert_eq!(status == 200, !error_due, "web status {status}");
        let (status, hosted_error, hosted) = keyed.tool(KEY, "design_run", &arguments);
        assert_eq!((status, hosted_error, hosted.as_str()), (200, error_due, local.as_str()), "hosted");
        let (open_error, opened) = open.tool("design_run", &arguments);
        assert_eq!((open_error, opened.as_str()), (error_due, local.as_str()), "open");
        // The same request, the same bits.
        assert_eq!(stdio.tool("design_run", &arguments), (error, local));
    }
}

#[test]
fn the_timeline_found_is_an_ordinary_run_and_the_candidates_one_plan_each_recomputed() {
    let mut door = Stdio2::start(None);
    let (_, text) = door.tool("design_run", &plate_design());
    let designed = parsed(&text);
    assert_eq!(designed["outcome"], "reached", "{designed}");
    let miss = designed["run"]["reached"]["miss"].as_f64().unwrap();
    assert!(miss.abs() <= 0.01);
    // The run given is the run of its request sent alone, and verify_tape holds it.
    let alone = ok(&mut door, "run_trajectory", &designed["run"]["request"]);
    assert_eq!(alone["run_seal"], designed["run"]["run_seal"]);
    assert_eq!(alone["final_state"]["temperature_k_hex"], designed["run"]["reached"]["value_hex"]);
    let held = ok(&mut door, "verify_tape", &json!({ "request": designed["run"]["request"], "run_seal": designed["run"]["run_seal"] }));
    assert_eq!(held["holds"], true);
    // The candidates are one run_dataset plan: run_dataset gives the same seal, verify_dataset recomputes it.
    let plan = &designed["candidates"]["request"];
    let dataset = ok(&mut door, "run_dataset", plan);
    assert_eq!(dataset["head"]["dataset_seal"], designed["candidates"]["dataset_seal"]);
    assert_eq!(dataset["summary"].as_array().unwrap().len() as u64, designed["candidates"]["rows"].as_u64().unwrap());
    for (row, said) in dataset["summary"].as_array().unwrap().iter().zip(designed["candidates"]["summary"].as_array().unwrap()) {
        assert_eq!(row["run_seal"], said["run_seal"]);
    }
    let held = ok(&mut door, "verify_dataset", &json!({ "request": plan, "dataset_seal": designed["candidates"]["dataset_seal"] }));
    assert_eq!(held["holds"], true, "{held}");
    // The answer carries a receipt over every byte before it.
    let receipt = ok(&mut door, "verify_receipt", &json!({ "response_text": text }));
    assert_eq!(receipt["matches"], true);
    assert!(designed["design_seal"].as_str().is_some_and(|seal| seal.len() == 96));
}

/// The design seal by its rule: the text of the request, then the tail.
fn design_seal_of(answer: &Value) -> String {
    fn form(value: &Value, out: &mut String) {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                out.push('{');
                for (index, key) in keys.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&serde_json::to_string(key).unwrap());
                    out.push(':');
                    form(&map[*key], out);
                }
                out.push('}');
            }
            Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    form(item, out);
                }
                out.push(']');
            }
            Value::Number(number) => out.push_str(&format!("x{:016x}", number.as_f64().unwrap().to_bits())),
            other => out.push_str(&serde_json::to_string(other).unwrap()),
        }
    }
    let mut text = String::new();
    form(&answer["inputs"], &mut text);
    let run = &answer["run"];
    let tail = json!({
        "outcome": answer["outcome"], "reject": answer["reject"], "run_seal": run["run_seal"],
        "dataset_seal": answer["candidates"]["dataset_seal"], "reached_hex": run["reached"]["value_hex"], "miss_hex": run["reached"]["miss_hex"]
    });
    // serde_json keeps the order written above (preserve_order).
    text.push_str(&serde_json::to_string(&tail).unwrap());
    hex::encode(Sha384::digest(text.as_bytes()))
}

#[test]
fn a_forged_design_does_not_hold() {
    let mut door = Stdio2::start(None);
    let (_, text) = door.tool("design_run", &ceiling_design());
    let designed = parsed(&text);
    assert_eq!(designed["outcome"], "reached");
    assert_eq!(design_seal_of(&designed), designed["design_seal"].as_str().unwrap(), "the design seal follows its rule");

    // A timeline changed by a hair, sent with the seal the engine gave: the tape does not hold.
    let mut forged = designed["run"]["request"].clone();
    let power = forged["segments"][0]["load"]["heat_power_w"].as_f64().unwrap();
    forged["segments"][0]["load"]["heat_power_w"] = json!(f64::from_bits(power.to_bits() + 1));
    let held = ok(&mut door, "verify_tape", &json!({ "request": forged, "run_seal": designed["run"]["run_seal"] }));
    assert_eq!(held["holds"], false);
    // A run seal that is not the run's.
    let mut seal = designed["run"]["run_seal"].as_str().unwrap().to_string();
    seal.replace_range(0..1, if seal.starts_with('0') { "1" } else { "0" });
    let held = ok(&mut door, "verify_tape", &json!({ "request": designed["run"]["request"], "run_seal": seal }));
    assert_eq!(held["holds"], false);

    // A candidate taken out of the record, or one changed: the candidates' seal does not hold.
    let mut fewer = designed["candidates"]["request"].clone();
    fewer["plan"]["runs"].as_array_mut().unwrap().remove(1);
    let held = ok(&mut door, "verify_dataset", &json!({ "request": fewer, "dataset_seal": designed["candidates"]["dataset_seal"] }));
    assert_eq!(held["holds"], false);
    let mut changed = designed["candidates"]["request"].clone();
    changed["plan"]["runs"][1]["segments"][0]["load"]["heat_power_w"] = json!(150.0);
    let held = ok(&mut door, "verify_dataset", &json!({ "request": changed, "dataset_seal": designed["candidates"]["dataset_seal"] }));
    assert_eq!(held["holds"], false);

    // A miss written smaller: the receipt no longer covers the bytes.
    let smaller = text.replacen(&format!("\"miss\":{}", designed["run"]["reached"]["miss"]), "\"miss\":0.0", 1);
    assert_ne!(smaller, text);
    let receipt = ok(&mut door, "verify_receipt", &json!({ "response_text": smaller }));
    assert_eq!(receipt["matches"], false);
    // Its exact miss changed and the receipt made again over the forged bytes:
    // the design seal no longer follows its rule, and the engine's answer to the
    // same request is not the forged one.
    let mut forged_answer = designed.clone();
    forged_answer["run"]["reached"]["miss_hex"] = json!("0000000000000000");
    assert_ne!(design_seal_of(&forged_answer), designed["design_seal"].as_str().unwrap());
    // An outcome changed: a refusal of reach written as reached.
    let (_, short_text) = door.tool("design_run", &json!({
        "request": plate(0.0),
        "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 40.0 }],
        "target": { "quantity": "temperature_k", "value": 333.15, "tolerance": 0.01 }
    }));
    let short = parsed(&short_text);
    assert_eq!((short["outcome"].as_str(), short["reject"].as_str()), (Some("refused"), Some("TargetBeyondReach")));
    let mut claimed = short.clone();
    claimed["outcome"] = json!("reached");
    claimed["reject"] = Value::Null;
    assert_ne!(design_seal_of(&claimed), short["design_seal"].as_str().unwrap());
    assert_eq!(design_seal_of(&short), short["design_seal"].as_str().unwrap());
}

#[test]
fn the_search_moves_no_value_to_a_limit_and_fills_nothing_in() {
    let mut door = Stdio2::start(None);
    // A span past the card's own bound is refused before any run: not moved to the bound.
    let past = refused(&mut door, "design_run", &json!({
        "request": plate(0.0),
        "vary": [{ "place": "segments[0].load.ambient_k", "low": 200.0, "high": 300.0 }],
        "target": { "quantity": "temperature_k", "value": 300.0, "tolerance": 0.01 }
    }));
    assert!(past.contains("no value is moved to it"), "{past}");
    // A place the request holds no number for is not filled in.
    let mut bare = plate(0.0);
    bare["segments"][0]["load"].as_object_mut().unwrap().remove("heat_power_w");
    let bare = refused(&mut door, "design_run", &json!({
        "request": bare,
        "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 40.0 }],
        "target": { "quantity": "temperature_k", "value": 300.0, "tolerance": 0.01 }
    }));
    assert!(bare.contains("nothing is filled in"), "{bare}");
    // A start value, the step or the steps are never set.
    for place in ["initial.temperature_k", "dt_s", "segments[0].steps"] {
        let said = refused(&mut door, "design_run", &json!({
            "request": plate(0.0), "vary": [{ "place": place, "low": 1.0, "high": 2.0 }],
            "target": { "quantity": "temperature_k", "value": 300.0, "tolerance": 0.01 }
        }));
        assert!(said.contains("sets loads only"), "{place}: {said}");
    }
    // A target outside the card's range is refused by name before any run.
    let outside = refused(&mut door, "design_run", &json!({
        "request": plate(0.0), "vary": [{ "place": "segments[0].load.heat_power_w", "low": 0.0, "high": 400.0 }],
        "target": { "quantity": "temperature_k", "value": 450.0, "tolerance": 0.01 }
    }));
    assert!(outside.contains("(TargetOutsideCard)"), "{outside}");

    // A candidate that crosses the ceiling is in the record at the value tried,
    // refused by its name, with its own run seal: sent alone it is the same refusal.
    let designed = ok(&mut door, "design_run", &ceiling_design());
    let rows = designed["candidates"]["summary"].as_array().unwrap();
    let runs = designed["candidates"]["request"]["plan"]["runs"].as_array().unwrap();
    let mut seen_refused = 0;
    for (row, run) in rows.iter().zip(runs) {
        let set = row["set"][0].as_f64().unwrap();
        assert!((0.0..=200.0).contains(&set), "{row}");
        assert_eq!(run["segments"][0]["load"]["heat_power_w"].as_f64().unwrap().to_bits(), set.to_bits());
        // Nothing but the load set differs from the request as sent.
        let mut as_sent = run.clone();
        as_sent["segments"][0]["load"]["heat_power_w"] = json!(0.0);
        let mut original = plate(0.0);
        original.as_object_mut().unwrap().remove("domain");
        original.as_object_mut().unwrap().remove("card");
        assert_eq!(as_sent, original);
        if row["outcome"] == "refused" {
            seen_refused += 1;
            assert_eq!(row["reject"], "TemperatureOutsideCard");
            let mut request = run.clone();
            request["domain"] = json!("energy");
            request["card"] = json!("thermal");
            let alone = ok(&mut door, "run_trajectory", &request);
            assert_eq!((alone["outcome"].as_str(), alone["run_seal"].clone()), (Some("refused"), row["run_seal"].clone()));
            assert!(alone["reject_detail"].as_str().unwrap().contains("is not sealed"));
        }
    }
    assert!(seen_refused >= 1, "{designed}");
    // The timeline found lies under the ceiling and was itself run.
    assert!(rows.iter().any(|row| row["run_seal"] == designed["run"]["run_seal"] && row["outcome"] == "completed"));
}

#[test]
fn a_ring_past_the_nodes_bound_is_recorded_and_the_node_voltage_is_reached() {
    let mut door = Stdio2::start(None);
    let described = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "grid_node", "form": "short" }));
    let bound = described["load_fields"][0]["maximum"].as_f64().unwrap();
    let designed = ok(&mut door, "design_run", &json!({
        "request": {
            "domain": "energy", "card": "grid_node",
            "initial": { "current_a": 0.0, "voltage_v": 0.0, "temperature_k": 293.15 }, "dt_s": 1e-6,
            "segments": [{ "steps": 500, "load": { "upstream_voltage_v": 0.0, "load_current_a": 0.0, "ambient_k": 293.15 } }],
            "record_every": 500
        },
        "vary": [{ "place": "segments[0].load.upstream_voltage_v", "low": 0.0, "high": bound }],
        "target": { "quantity": "voltage_v", "value": 48.0, "tolerance": 0.001 }
    }));
    assert_eq!(designed["outcome"], "reached", "{designed}");
    assert!(designed["candidates"]["summary"].as_array().unwrap().iter().any(|row| row["reject"] == "VoltageOutsideCard"));
    // The node rings: 48 V at half a millisecond takes less than 48 V upstream.
    let upstream = designed["timeline"][0]["load"]["upstream_voltage_v"].as_f64().unwrap();
    assert!(upstream < 48.0, "{upstream}");
    // No monotone map is declared for this card, so a miss would be not_reached, never a refusal of reach.
    assert!(designed["search"]["monotone"].is_null());
}

#[test]
fn design_run_is_listed_and_on_the_open_connection_is_long_work_and_read_only() {
    let open = Open::start();
    let reply = parse_reply(&raw(open.port, &request_bytes(None, &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string()), 60));
    let listed = parsed(&reply.body);
    let tools = listed["result"]["tools"].as_array().unwrap();
    let tool = tools.iter().find(|tool| tool["name"] == "design_run").expect("design_run is listed");
    assert_eq!(tool["annotations"]["readOnlyHint"], true);
    assert_eq!(tool["annotations"]["destructiveHint"], false);
    let said = tool["description"].as_str().unwrap();
    assert!(said.starts_with("Designs a run"), "{said}");
    assert!(said.contains("DESIGN_MAX_CANDIDATES") && said.contains("long work") && said.contains("TargetBeyondReach"), "{said}");
    // The engine's own words, before what the open connection adds.
    let own = said.split(" On this open connection").next().unwrap();
    assert!(!own.to_ascii_lowercase().contains("check"), "{own}");
    assert!(said.len() <= 2000, "{}", said.len());
}
