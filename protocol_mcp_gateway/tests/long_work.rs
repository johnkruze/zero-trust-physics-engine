//! Long work on the open connection: it ends at the connection's time for an
//! answer and gives its permit and its place back, it answers the bytes of
//! the local connection, and a solve on an input far below anything physical
//! is refused by the solver itself, the same on the open connection as on
//! the local one, before anything is computed. Every test here drives the
//! `hosted_door` binary itself, since long work runs in a process it starts.

mod hosted_support;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use hosted_support::*;
use serde_json::{Value, json};

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

    fn call(&self, tool: &str, arguments: &Value, wait_s: u64) -> (bool, String) {
        let reply = parse_reply(&raw(self.port, &request_bytes(None, &tool_body(1, tool, arguments)), wait_s));
        assert_eq!(reply.status, 200, "{reply:?}");
        tool_result(&reply)
    }

    /// The processes this connection has started that are still running.
    fn children(&self) -> Vec<String> {
        let out = Command::new("pgrep").args(["-P", &self.child.id().to_string()]).output().unwrap();
        String::from_utf8_lossy(&out.stdout).split_whitespace().map(str::to_string).collect()
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A solve long enough to pass any limit a test sets, and inside every named
/// floor: the heat-shield slab under a history of 256 rows a tenth of a
/// second apart, every row a corner. About 157,000 time steps: 17 s in a
/// release build on one core of an Apple M4 Pro, and minutes in a test build.
fn a_long_solve() -> Value {
    let rows = 256;
    let mut times: Vec<f64> = (0..rows - 1).map(|row| row as f64 * 0.1).collect();
    times.push(60.0);
    let coefficient: Vec<f64> = (0..rows).map(|row| if row % 2 == 1 { 0.3 } else { 0.0 }).collect();
    json!({ "solver": "ablator_slab", "parameters": {
        "material": "TACOT 3.0", "thickness_m": 0.05, "start_temperature_k": 300.0, "thermocouple_depths_m": [],
        "face": "gas_stream", "stream_gas": "air", "history_time_s": times,
        "history_pressure_pa": vec![1000.0; rows], "history_heat_transfer_coefficient_kg_per_m2_s": coefficient,
        "history_flow_enthalpy_j_per_kg": vec![2.0e6; rows], "history_radiative_heating_w_per_m2": vec![0.0; rows],
        "surroundings_temperature_k": 300.0, "face_wears": true, "return_face_history": false,
        "return_thermocouple_histories": false, "return_final_profile": false
    } })
}

fn a_short_long_run() -> Value {
    json!({
        "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 1.0,
        "segments": [{ "steps": 10_001, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }], "record_every": 10_001
    })
}

fn lhd(boron: Value) -> Value {
    let mut door = Stdio2::start(None);
    let (_, text) = door.tool("describe_solver", &json!({ "solver": "fusion_pb11_lhd" }));
    let described: Value = serde_json::from_str(&text).unwrap();
    let mut request = described["example_solve"].clone();
    request["parameters"]["boron_density_per_m3"] = boron;
    request
}

#[test]
fn long_work_is_stopped_at_the_limit_and_its_permit_and_place_come_back() {
    let open = Open::start_with(&["--request-seconds", "2"]);
    let long = a_long_solve();
    let started = Instant::now();
    let (error, text) = open.call("solve", &long, 60);
    let took = started.elapsed();
    assert!(error, "{text}");
    assert_eq!(
        text,
        "Physical Violation: the answer did not come within 2 seconds, the longest the open connection waits for one call (OPEN_REQUEST_SECONDS). The work was stopped there: nothing is returned for this call and no proof was issued. The same request takes as long again and is stopped again, so do not send it as it is. Send a smaller one: fewer steps, fewer rows, a shorter history, or a value nearer the model's worked example."
    );
    assert!(took >= Duration::from_secs(2) && took < Duration::from_secs(6), "{took:?}");
    // The work ended with the answer: no process of the connection's is left computing.
    assert_eq!(open.children(), Vec::<String>::new());
    // And the one place for long work and the address's one permit are free at once:
    // long work from the same address is answered, not told the pool is busy.
    let (error, text) = open.call("run_trajectory", &a_short_long_run(), 30);
    assert!(!error, "{text}");
    assert!(text.contains("\"outcome\":\"completed\""), "{text}");
    // The words an agent reads when the connection opens say so.
    let hello = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string();
    let said: Value = serde_json::from_str(&parse_reply(&raw(open.port, &request_bytes(None, &hello), 30)).body).unwrap();
    let said = said["result"]["instructions"].as_str().unwrap();
    assert!(said.contains("An answer is waited for 2 seconds (OPEN_REQUEST_SECONDS)"), "{said}");
    assert!(said.contains("Long work is stopped at that time, and its permit and its place come back"), "{said}");
    assert!(!said.contains("send it again, or a smaller one"), "{said}");
}

#[test]
fn while_long_work_computes_other_long_work_is_told_so_and_a_step_is_answered() {
    let open = Open::start_with(&["--request-seconds", "4", "--physics-permits", "2", "--client-address-header", "x-test-address"]);
    let long = a_long_solve();
    let port = open.port;
    let from = |address: &str, tool: &str, arguments: &Value| -> (bool, String) {
        let body = tool_body(1, tool, arguments);
        let text = String::from_utf8(request_bytes(None, &body)).unwrap().replacen("\r\n", &format!("\r\nx-test-address: {address}\r\n"), 1);
        tool_result(&parse_reply(&raw(port, text.as_bytes(), 60)))
    };
    let running = std::thread::spawn({
        let long = long.clone();
        move || {
            let body = tool_body(1, "solve", &long);
            let text = String::from_utf8(request_bytes(None, &body)).unwrap().replacen("\r\n", "\r\nx-test-address: 10.0.0.1\r\n", 1);
            tool_result(&parse_reply(&raw(port, text.as_bytes(), 60)))
        }
    });
    std::thread::sleep(Duration::from_millis(700));
    // Another address's long work finds no place, by name, and nothing waits.
    let (error, text) = from("10.0.0.2", "run_trajectory", &a_short_long_run());
    assert_eq!((error, text.as_str()), (true, "Too many concurrent physics tasks."));
    // A single step from another address is answered: the pool's other permit is left for it.
    let (error, text) = from("10.0.0.2", "verify_step", &thermal_step());
    assert!(!error && text.contains("sha256_seal"), "{text}");
    let (error, text) = running.join().unwrap();
    assert!(error && text.contains("The work was stopped there"), "{text}");
    // After the stop the other address's long work is answered.
    let (error, text) = from("10.0.0.2", "run_trajectory", &a_short_long_run());
    assert!(!error, "{text}");
}

#[test]
fn long_work_in_its_own_process_answers_the_bytes_of_the_local_connection() {
    let open = Open::start_with(&[]);
    let mut local = Stdio2::start(None);
    let pack = json!({
        "domain": "energy", "card": "battery_sag", "initial": { "soc": 1.0 }, "dt_s": 0.01,
        "segments": [{ "steps": 20_000, "load": { "current_a": 50 } }, { "steps": 20_000, "load": { "current_a": 50.0 } }], "record_every": 40_000
    });
    let calls: Vec<(&str, Value)> = vec![
        ("run_trajectory", a_short_long_run()),
        // A whole number and the same number written with a point: the receipt follows the spelling.
        ("run_trajectory", pack),
        ("solve", json!({ "solver": "pipe_flow_superpipe", "example": "example_solve" })),
        ("solve", json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } })),
        // Stops before computing, from a process of its own, are the same one sentence.
        ("solve", json!({ "solver": "atom_dft", "parameters": { "atomic_number": 40 } })),
        ("solve", json!({ "solver": "no_such_solver", "parameters": {} })),
        ("run_dataset", json!({ "solver": "atom_dft", "plan": { "kind": "list", "solves": [{ "parameters": { "atomic_number": 1 } }, { "parameters": { "atomic_number": 2 } }] } })),
        ("find_flip", json!({ "request": { "solver": "atom_dft", "parameters": { "atomic_number": 2 } }, "vary": "parameters.atomic_number", "low": 1, "high": 30 })),
    ];
    for (tool, arguments) in &calls {
        let through = open.call(tool, arguments, 300);
        let here = local.tool(tool, arguments);
        assert_eq!(through, here, "{tool} {arguments}");
    }
    assert_eq!(open.children(), Vec::<String>::new());
}

/// The process the connection starts for one call, driven directly: with a
/// permit it answers; without one it answers as a full pool does, and a
/// request that stops before computing still gets its own sentence.
#[test]
fn a_call_made_without_a_permit_is_answered_as_a_full_pool_answers() {
    let one_call = |name: &str, arguments: &Value, permit: bool| -> Value {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .arg("--one-call")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(json!({ "name": name, "arguments": arguments, "permit": permit }).to_string().as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        serde_json::from_slice(&out.stdout).unwrap()
    };
    let atom = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 1 } });
    let solved = one_call("solve", &atom, true);
    assert!(solved["isError"] != json!(true) && solved["content"][0]["text"].as_str().unwrap().contains("solution_seal"), "{solved}");
    let busy = one_call("solve", &atom, false);
    assert_eq!((busy["isError"].as_bool(), busy["content"][0]["text"].as_str()), (Some(true), Some("Too many concurrent physics tasks.")));
    let stopped = one_call("solve", &json!({ "solver": "atom_dft", "parameters": { "atomic_number": 40 } }), false);
    assert_eq!(stopped["content"][0]["text"], "Physical Violation: atomic number 40 is outside 1..=18.");
    let busy = one_call("run_trajectory", &a_short_long_run(), false);
    assert_eq!(busy["content"][0]["text"], "Too many concurrent physics tasks.");
}

/// The open connection holds no floor of its own on the boron density: the
/// solver refuses a density under one nucleus in the plasma on every
/// connection, in its own words, before anything is computed.
#[test]
fn a_vanishing_boron_density_gets_the_solvers_own_refusal_alike_on_the_open_and_the_local_connection() {
    let open = Open::start_with(&[]);
    let mut local = Stdio2::start(None);
    let slow = lhd(json!(1.0e-297));
    let sentence = "Physical Violation: boron density 1e-297 m^-3 is not zero and is below 0.04973892441153873 m^-3, the peak density at which the plasma holds one boron nucleus: less than one nucleus is no boron, and zero is answered (BoronBelowOneNucleus).";
    let started = Instant::now();
    let through = open.call("solve", &slow, 30);
    assert_eq!(through, (true, sentence.to_string()));
    assert_eq!(through, local.tool("solve", &slow));
    // A check of it by request, and a plan with it as a row: the same bytes on both connections.
    let check = json!({ "request": slow, "solution_seal": "ab".repeat(32) });
    let checked = open.call("verify_solution", &check, 30);
    assert_eq!(checked, (true, sentence.to_string()));
    assert_eq!(checked, local.tool("verify_solution", &check));
    let plan = json!({ "solver": "fusion_pb11_lhd", "plan": { "kind": "list", "solves": [{ "parameters": slow["parameters"] }] } });
    let (error, planned) = open.call("run_dataset", &plan, 30);
    assert_eq!((error, planned.clone()), local.tool("run_dataset", &plan));
    assert!(!error, "{planned}");
    let planned: Value = serde_json::from_str(&planned).unwrap();
    // The row is unadmitted by the solver's name, with no proof, and nothing was solved.
    assert_eq!((planned["head"]["unadmitted"].as_u64(), planned["head"]["completed"].as_u64()), (Some(1), Some(0)), "{planned}");
    assert_eq!(
        (planned["summary"][0]["outcome"].as_str(), planned["summary"][0]["reject"].as_str()),
        (Some("unadmitted"), Some("BoronBelowOneNucleus")),
        "{planned}"
    );
    assert_eq!(planned["summary"][0]["solution_seal"], Value::Null, "{planned}");
    // All three stopped before computing: together they took far less than one solve at that density once did.
    assert!(started.elapsed() < Duration::from_secs(10), "{:?}", started.elapsed());
    assert_eq!(open.children(), Vec::<String>::new());

    // A search whose span reaches below the solver's floor is not turned
    // away: its lower end stopped before computing, by the solver's name,
    // with no proof, and no solve in it ran long.
    let floor = 0.04973892441153873_f64;
    let search = json!({ "request": lhd(json!(8.6706e16)), "vary": "parameters.boron_density_per_m3", "low": 0.04, "high": 0.06 });
    let started = Instant::now();
    let (error, searched) = open.call("find_flip", &search, 60);
    let took = started.elapsed();
    assert!(!error, "{searched}");
    let found: Value = serde_json::from_str(&searched).unwrap();
    assert_eq!((found["low_is"].as_str(), found["high_is"].as_str()), (Some("stopped before computing (BoronBelowOneNucleus)"), Some("converged")), "{searched}");
    assert_eq!(found["flips"], true);
    assert_eq!(found["low"]["solution_seal"], Value::Null);
    assert!(found["low"]["no_proof"].as_str().unwrap().contains("no proof is issued"), "{searched}");
    assert!(found["low"]["reject_detail"].as_str().unwrap().ends_with("and zero is answered (BoronBelowOneNucleus)."), "{searched}");
    assert!(found["high"]["solution_seal"].is_string(), "{searched}");
    let (low, high) = (found["low"]["value"].as_f64().unwrap(), found["high"]["value"].as_f64().unwrap());
    assert!(0.04 <= low && low < floor && floor <= high && high <= 0.06, "{low} {high}");
    if found["neighbours"] == json!(true) {
        // The search ended on the floor itself: the last density refused and the first answered.
        assert_eq!((low, high), (f64::from_bits(floor.to_bits() - 1), floor));
        assert_eq!(found["stopped_by"], Value::Null);
    } else {
        // On a slow machine it stops at its own time for computing, and says so.
        assert!(found["stopped_by"].as_str().unwrap().contains("FLIP_MAX_SECONDS"), "{searched}");
    }
    // The search computes for at most 20 s (FLIP_MAX_SECONDS) and was answered, not ended at the time for an answer.
    assert!(took < Duration::from_secs(45), "{took:?}");
    assert_eq!(open.children(), Vec::<String>::new());

    // None, the floor, a density the open connection once refused and the
    // worked example are answered, with the bytes of the local connection.
    for boron in [json!(0.0), json!(floor), json!(0.5), json!(1.0), json!(8.6706e16)] {
        let request = lhd(boron.clone());
        let through = open.call("solve", &request, 300);
        assert!(!through.0 && through.1.contains("\"outcome\":\"converged\""), "{boron}: {}", through.1);
        assert_eq!(through, local.tool("solve", &request), "{boron}");
    }
    // The limits an agent reads when the connection opens name the limit on a plan and no floor of the connection's own.
    let hello = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}).to_string();
    let said: Value = serde_json::from_str(&parse_reply(&raw(open.port, &request_bytes(None, &hello), 30)).body).unwrap();
    let said = said["result"]["instructions"].as_str().unwrap();
    assert!(said.contains("holds no more than 15 s of them (OPEN_PLAN_SECONDS)"), "{said}");
    for gone in ["OPEN_FUSION_BORON_DENSITY_FLOOR_PER_M3", "OPEN_ABLATOR_HISTORY_SPAN_FLOOR_S", "below anything physical"] {
        assert!(!said.contains(gone), "{gone}: {said}");
    }
}
