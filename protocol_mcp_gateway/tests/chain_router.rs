//! Chained proofs: one run's sealed final state carried into the next request
//! by `from`, as a continuation of the same card or through a named coupling.
//!
//! What is held here: a chained answer and the same downstream request typed
//! by hand give identical result bits and differ only in the chain
//! commitment; a forged-but-self-consistent upstream, a swapped coupling name
//! and a broken ledger are each refused by name every time, with no proof;
//! the chain verifies with the program of genesis_seal, without the engine,
//! and fails after any single change; the chain's steps are capped by name.
//! Every connection gives the same bytes.

mod hosted_support;
use hosted_support::answer_text;

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use genesis_kernel::reactor_spert3_fuel as fuel;
use genesis_kernel::resonance::DynamicOscillator;
use hosted_support::*;
use serde_json::{Value, json};
use tokio::net::TcpListener;

const KEY: &str = "chain-key";

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
    let path = if tool == "verify_step" { "verify_physics" } else { tool };
    let response = reqwest::Client::new()
        .post(format!("{base}/{path}"))
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

fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")].as_str().unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

// The mount card: a run cut in two ends where the straight run does
// (tests/resonance_router.rs), so a continuation can be held to the bit.

fn gate() -> f64 {
    DynamicOscillator::mount_10hz().gate_dt_s().unwrap()
}

fn new_start(displacement_m: f64, velocity_m_s: f64) -> Value {
    json!({
        "displacement_m": displacement_m,
        "velocity_m_s": velocity_m_s,
        "t_s": 0.0,
        "energy_anchor_j": DynamicOscillator::mount_10hz().mechanical_energy_j(displacement_m, velocity_m_s),
        "boundary_work_j": 0.0,
    })
}

/// The start of a continuation: each start field read from the exact hex.
fn handed_back(state: &Value) -> Value {
    json!({
        "displacement_m": published(state, "displacement_m"),
        "velocity_m_s": published(state, "velocity_m_s"),
        "t_s": published(state, "t_s"),
        "energy_anchor_j": published(state, "energy_anchor_j"),
        "boundary_work_j": published(state, "boundary_work_j"),
    })
}

fn mount_run(initial: Value, steps: u64) -> Value {
    json!({
        "domain": "signal", "card": "resonance", "initial": initial, "dt_s": gate(),
        "segments": [{ "steps": steps, "load": { "accel_m_s2": 3.0 } }],
        "record_every": steps
    })
}

fn chained(mut request: Value, upstream: &Value, run_seal: &Value, coupling: Option<&str>) -> Value {
    let mut from = json!({ "request": upstream, "run_seal": run_seal });
    if let Some(name) = coupling {
        from["coupling"] = json!(name);
    }
    request["from"] = from;
    request
}

// The two SPERT III cards.

fn point_burst(reactivity: f64) -> Value {
    json!({
        "domain": "energy", "card": "reactor_spert3",
        "initial": { "coolant_temperature_k": 300.92777777777775 },
        "dt_s": 0.0007144471176212568,
        "segments": [{ "steps": 3000, "load": { "reactivity_inserted_dollar": reactivity } }],
        "record_every": 3000
    })
}

fn fuel_burst(inlet_k: f64, reactivity: f64, flow: f64) -> Value {
    json!({
        "domain": "energy", "card": "reactor_spert3_fuel",
        "initial": { "inlet_temperature_k": inlet_k },
        "dt_s": fuel_gate(),
        "segments": [{ "steps": 60000, "load": { "reactivity_inserted_dollar": reactivity, "coolant_flow_m_per_s": flow } }],
        "record_every": 3000
    })
}

fn fuel_gate() -> f64 {
    fuel::ReactorSpert3Fuel::e_core_low_initial_power().gate_dt_s().unwrap()
}

const REACTIVITY: f64 = 0.9982818460448638;

/// A door to call: the local connection.
fn local() -> Stdio2 {
    Stdio2::start(None)
}

fn ok(door: &mut Stdio2, tool: &str, arguments: &Value) -> Value {
    let (error, text) = door.tool(tool, arguments);
    assert!(!error, "{tool}: {text}");
    parsed(&text)
}

fn refused_by_name(door: &mut Stdio2, tool: &str, arguments: &Value, name: &str) -> String {
    let (error, text) = door.tool(tool, arguments);
    assert!(error, "{tool} answered where {name} was due: {text}");
    assert!(text.contains(&format!("({name})")), "{name} not named: {text}");
    assert!(!text.contains("run_seal\":") && !text.contains("chain_seal\":") && !text.contains("sha256_seal"), "{text}");
    text
}

const RESULT_KEYS: [&str; 10] = [
    "inputs",
    "engine_version",
    "outcome",
    "steps_landed",
    "reject",
    "final_state",
    "final_residual_conservation_error_hex",
    "state_digest",
    "frames",
    "run_seal",
];

/// Pass line (a): a chained continuation and the same request typed by hand
/// give the same result bits, the same frames and the same run seal; the
/// answers differ only in `chain` and in the receipt. Where the card's own
/// tests show a cut run ends on the straight run's state, so does the chain.
#[test]
fn a_chained_continuation_is_the_hand_typed_run_with_a_chain_beside_it() {
    let mut door = local();
    let straight = ok(&mut door, "run_trajectory", &mount_run(new_start(0.005, 0.1), 1000));
    let first_request = mount_run(new_start(0.005, 0.1), 400);
    let first = ok(&mut door, "run_trajectory", &first_request);
    let by_hand_request = mount_run(handed_back(&first["final_state"]), 600);
    // Typed by hand, without from, the continuation is refused by name: a
    // start that carries a run on arrives with from.
    refused_by_name(&mut door, "run_trajectory", &by_hand_request, "ContinuationWithoutUpstream");
    let request = chained(by_hand_request.clone(), &first_request, &first["run_seal"], None);
    let (error, text) = door.tool("run_trajectory", &request);
    assert!(!error, "{text}");
    let chain_answer = parsed(&text);
    // The run is the request without from: its inputs, and a tape that holds
    // by itself, with no physics, without its chain.
    assert_eq!(chain_answer["inputs"], by_hand_request);
    let mut tape = chain_answer.as_object().unwrap().clone();
    tape.shift_remove("chain");
    tape.shift_remove("receipt_sha256");
    let alone = genesis_seal::run::check_tape(&Value::Object(tape)).unwrap();
    assert!(alone.holds, "{alone:?}");
    // The straight run's state, bit for bit.
    for name in ["t_s", "displacement_m", "velocity_m_s", "energy_anchor_j", "boundary_work_j", "energy_defect_j", "residual_conservation_error"] {
        assert_eq!(
            published(&chain_answer["final_state"], name).to_bits(),
            published(&straight["final_state"], name).to_bits(),
            "{name}"
        );
    }
    let chain = &chain_answer["chain"];
    assert_eq!(chain["kind"], "continuation");
    assert_eq!(chain["coupling"], Value::Null);
    assert_eq!(chain["upstream_run_seal"], first["run_seal"]);
    assert_eq!(chain["upstream_chain_seal"], Value::Null);
    let handed: Vec<&str> = chain["handed"].as_array().unwrap().iter().map(|entry| entry["to"].as_str().unwrap()).collect();
    assert_eq!(handed, ["initial.displacement_m", "initial.velocity_m_s", "initial.t_s", "initial.energy_anchor_j", "initial.boundary_work_j"]);
    // The chain seal by its rule (REFERENCE.md).
    let handed: Vec<genesis_seal::chain::Handed> = chain["handed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| genesis_seal::chain::Handed {
            from: entry["from"].as_str().unwrap().into(),
            to: entry["to"].as_str().unwrap().into(),
            unit: entry["unit"].as_str().map(str::to_string),
            value_hex: entry["value_hex"].clone(),
        })
        .collect();
    let tail = genesis_seal::chain::chain_tail("continuation", None, &handed);
    assert_eq!(
        chain["chain_seal"].as_str().unwrap(),
        genesis_seal::chain::chain_seal(first["run_seal"].as_str().unwrap(), &tail, chain_answer["run_seal"].as_str().unwrap())
    );
    // The receipt covers the chain.
    let (_, receipt) = door.tool("verify_receipt", &json!({ "response_text": text }));
    assert_eq!(parsed(&receipt)["matches"], json!(true), "{receipt}");
    // A check by request computes the chain again and compares both seals.
    let check = ok(&mut door, "verify_tape", &json!({ "request": request, "run_seal": chain_answer["run_seal"], "chain_seal": chain["chain_seal"] }));
    assert_eq!(check["holds"], json!(true), "{check}");
    assert_eq!(check["recomputed_chain_seal"], chain["chain_seal"]);
    let wrong = ok(&mut door, "verify_tape", &json!({ "request": request, "run_seal": chain_answer["run_seal"], "chain_seal": "ab".repeat(32) }));
    assert_eq!((wrong["holds"].clone(), wrong["cause"].clone()), (json!(false), json!("chain_seal")), "{wrong}");
}

/// The run seal of each link, the chain seal of the last: three runs of the
/// mount in one request, and the same three as links checked with no physics.
#[test]
fn a_chain_of_three_runs_is_one_checkable_object() {
    let mut door = local();
    let r1 = mount_run(new_start(0.005, 0.1), 300);
    let a1 = ok(&mut door, "run_trajectory", &r1);
    let r2 = chained(mount_run(handed_back(&a1["final_state"]), 300), &r1, &a1["run_seal"], None);
    let a2 = ok(&mut door, "run_trajectory", &r2);
    let r3 = chained(mount_run(handed_back(&a2["final_state"]), 400), &r2, &a2["run_seal"], None);
    let a3 = ok(&mut door, "run_trajectory", &r3);
    assert_eq!(a3["chain"]["upstream_run_seal"], a2["run_seal"]);
    assert_eq!(a3["chain"]["upstream_chain_seal"], a2["chain"]["chain_seal"]);
    let straight = ok(&mut door, "run_trajectory", &mount_run(new_start(0.005, 0.1), 1000));
    assert_eq!(a3["final_state"], straight["final_state"]);

    let links = json!({ "links": [a1, a2, a3] });
    let report = ok(&mut door, "verify_tape", &links);
    assert_eq!(report["holds"], json!(true), "{report}");
    assert_eq!(report["chain_seal"], a3["chain"]["chain_seal"]);
    // The program of genesis_seal, without the engine.
    let program = seal_program();
    let dir = scratch("chain_three");
    let path = dir.join("chain.json");
    std::fs::write(&path, links.to_string()).unwrap();
    let out = Command::new(&program).arg("verify-chain").arg(&path).output().unwrap();
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(out.status.code(), Some(0), "{said}");
    assert!(said.contains("holds: true") && said.contains("links: 3"), "{said}");
    assert!(said.contains(&format!("chain_seal: {}", a3["chain"]["chain_seal"].as_str().unwrap())), "{said}");

    // Any single change to any link breaks it.
    let changes: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        ("a frame of the first run", Box::new(|v: &mut Value| flip_hex(&mut v["links"][0]["frames"][0]["frame_hex"]))),
        ("the final state of the first run", Box::new(|v: &mut Value| flip_hex(&mut v["links"][0]["final_state"]["t_s_hex"]))),
        ("the second run's seal", Box::new(|v: &mut Value| flip_hex(&mut v["links"][1]["run_seal"]))),
        ("a handed value", Box::new(|v: &mut Value| flip_hex(&mut v["links"][1]["chain"]["handed"][0]["value_hex"]))),
        ("the upstream seal named", Box::new(|v: &mut Value| flip_hex(&mut v["links"][2]["chain"]["upstream_run_seal"]))),
        ("the upstream chain seal named", Box::new(|v: &mut Value| flip_hex(&mut v["links"][2]["chain"]["upstream_chain_seal"]))),
        ("the last chain seal", Box::new(|v: &mut Value| flip_hex(&mut v["links"][2]["chain"]["chain_seal"]))),
        ("a start value of the last request", Box::new(|v: &mut Value| {
            let x = v["links"][2]["inputs"]["initial"]["t_s"].as_f64().unwrap();
            v["links"][2]["inputs"]["initial"]["t_s"] = json!(x.next_up());
        })),
    ];
    for (what, change) in changes {
        let mut broken = links.clone();
        change(&mut broken);
        let report = ok(&mut door, "verify_tape", &broken);
        assert_eq!(report["holds"], json!(false), "{what}: {report}");
        std::fs::write(&path, broken.to_string()).unwrap();
        let out = Command::new(&program).arg("verify-chain").arg(&path).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{what}: {}", String::from_utf8_lossy(&out.stdout));
    }
}

fn flip_hex(value: &mut Value) {
    let text = value.as_str().unwrap().to_string();
    let last = text.chars().last().unwrap();
    let flipped = if last == '0' { '1' } else { '0' };
    *value = json!(format!("{}{}", &text[..text.len() - 1], flipped));
}

/// Pass line (b): a forged-but-self-consistent upstream is refused by name
/// every time. The forger rewrites the upstream's final state and rebuilds
/// its state digest and run seal by the published rule, so the forged tape
/// holds alone; sent as the upstream, it is computed again and refused.
#[test]
fn a_forged_but_self_consistent_upstream_is_refused_by_name_every_time() {
    let mut door = local();
    let r1 = mount_run(new_start(0.005, 0.1), 400);
    let a1 = ok(&mut door, "run_trajectory", &r1);
    let mut forged = a1.clone();
    let lie = published(&a1["final_state"], "energy_anchor_j") * 0.5;
    forged["final_state"]["energy_anchor_j"] = json!(lie);
    forged["final_state"]["energy_anchor_j_hex"] = json!(format!("{:016x}", lie.to_bits()));
    let digest = genesis_seal::run::state_digest(
        forged["final_state"].as_object().unwrap(),
        forged["final_residual_conservation_error_hex"].as_str().unwrap(),
        forged["max_residual_conservation_error_hex"].as_str().unwrap(),
    )
    .unwrap();
    forged["state_digest"] = json!(digest);
    let last_link = genesis_seal::sealed_form::hash_from_hex(forged["frames"].as_array().unwrap().last().unwrap()["link"].as_str().unwrap()).unwrap();
    let seal = genesis_seal::run::run_seal(&last_link, "completed", 400, None, Some(&digest));
    forged["run_seal"] = json!(seal);
    // The forged tape holds alone.
    let alone = genesis_seal::run::check_tape(&forged).unwrap();
    assert!(alone.holds, "{alone:?}");
    // Its seals hold in the long form too, since nothing in a tape is keyed;
    // since 0.12.0 the long form computes its request again, and the run it
    // computes has another run seal.
    let check = ok(&mut door, "verify_tape", &forged);
    assert_eq!((check["cause"].clone(), check["differs_at"].clone()), (json!("run_seal"), json!("run_seal")), "the forged tape is self-consistent: {check}");
    assert_eq!(check["compared"], protocol_mcp_gateway::LONG_FORM_COMPUTED, "{check}");

    let downstream = mount_run(handed_back(&forged["final_state"]), 100);
    for _ in 0..3 {
        // The forged seal: the upstream computed again has another seal.
        refused_by_name(&mut door, "run_trajectory", &chained(downstream.clone(), &r1, &forged["run_seal"], None), "UpstreamProofBroken");
        // The true seal with the forged state: the start is not the upstream's.
        refused_by_name(&mut door, "run_trajectory", &chained(downstream.clone(), &r1, &a1["run_seal"], None), "ContinuationNotFromUpstream");
        // And as a step.
        let mut parameters = handed_back(&forged["final_state"]);
        parameters["accel_m_s2"] = json!(3.0);
        parameters["dt_s"] = json!(gate());
        let step = json!({ "domain": "signal", "card": "resonance", "parameters": parameters });
        refused_by_name(&mut door, "verify_step", &chained(step, &r1, &a1["run_seal"], None), "ContinuationNotFromUpstream");
    }
}

/// Pass line (b): a broken ledger is refused by name every time: any start
/// or continuation field one unit in the last place off the upstream's, and
/// a field left out.
#[test]
fn a_broken_ledger_is_refused_by_name_every_time() {
    let mut door = local();
    let r1 = mount_run(new_start(0.005, 0.1), 400);
    let a1 = ok(&mut door, "run_trajectory", &r1);
    for name in ["displacement_m", "velocity_m_s", "t_s", "energy_anchor_j", "boundary_work_j"] {
        for _ in 0..2 {
            let mut initial = handed_back(&a1["final_state"]);
            initial[name] = json!(initial[name].as_f64().unwrap().next_up());
            let text = refused_by_name(&mut door, "run_trajectory", &chained(mount_run(initial, 50), &r1, &a1["run_seal"], None), "ContinuationNotFromUpstream");
            assert!(text.contains(&format!("initial.{name}")), "{text}");
        }
        let mut initial = handed_back(&a1["final_state"]);
        initial.as_object_mut().unwrap().remove(name);
        refused_by_name(&mut door, "run_trajectory", &chained(mount_run(initial, 50), &r1, &a1["run_seal"], None), "ContinuationNotFromUpstream");
    }
    // A continuation onto another card is not a continuation.
    let burst = point_burst(REACTIVITY);
    let burst_answer = ok(&mut door, "run_trajectory", &burst);
    refused_by_name(&mut door, "run_trajectory", &chained(mount_run(handed_back(&a1["final_state"]), 50), &burst, &burst_answer["run_seal"], None), "ContinuationNotFromUpstream");
    // A run that did not complete is not continued.
    let hot = json!({
        "domain": "energy", "card": "thermal",
        "initial": { "temperature_k": 390.0 }, "dt_s": 1.0,
        "segments": [{ "steps": 600, "load": { "heat_power_w": 3000.0, "ambient_k": 293.15 } }],
        "record_every": 600
    });
    let hot_answer = ok(&mut door, "run_trajectory", &hot);
    assert_eq!(hot_answer["outcome"], "refused");
    let mut next = hot.clone();
    next["initial"] = json!({ "temperature_k": published(&hot_answer["final_state"], "temperature_k") });
    refused_by_name(&mut door, "run_trajectory", &chained(next, &hot, &hot_answer["run_seal"], None), "UpstreamOutcomeNotCarried");
}

/// A continuation into one step: the step is the hand-typed step, and the
/// chain seal commits its sha256_seal. As the last link of the links form it
/// is computed again by compute_step, from the link before it, since 0.12.0.
#[test]
fn a_step_continues_from_a_sealed_run() {
    let mut door = local();
    let r1 = mount_run(new_start(0.005, 0.1), 400);
    let a1 = ok(&mut door, "run_trajectory", &r1);
    let mut parameters = handed_back(&a1["final_state"]);
    parameters["accel_m_s2"] = json!(3.0);
    parameters["dt_s"] = json!(gate());
    let by_hand_request = json!({ "domain": "signal", "card": "resonance", "parameters": parameters });
    refused_by_name(&mut door, "verify_step", &by_hand_request, "ContinuationWithoutUpstream");
    let step = ok(&mut door, "verify_step", &chained(by_hand_request.clone(), &r1, &a1["run_seal"], None));
    assert_eq!(step["inputs"], by_hand_request);
    let straight = ok(&mut door, "run_trajectory", &mount_run(new_start(0.005, 0.1), 401));
    assert_eq!(
        published(&step["state"], "displacement_m").to_bits(),
        published(&straight["final_state"], "displacement_m").to_bits()
    );
    let handed: Vec<&str> = step["chain"]["handed"].as_array().unwrap().iter().map(|entry| entry["to"].as_str().unwrap()).collect();
    assert!(handed.iter().all(|to| to.starts_with("parameters.")), "{handed:?}");
    let report = ok(&mut door, "verify_tape", &json!({ "links": [a1, step] }));
    assert_eq!(report["holds"], json!(true), "{report}");
    assert!(report["compared"].as_str().unwrap().contains("computed again under a physics permit and the limits of run_trajectory, or of compute_step for a step"), "{report}");
}

/// Coupling: the SPERT III burst handed from one card to the other, both
/// directions; refused by name outside the downstream range, with a swapped
/// or unknown name, and with a value not the one handed.
#[test]
fn a_coupling_hands_the_burst_on_and_is_refused_by_name_where_it_must_be() {
    let mut door = local();
    let up = point_burst(REACTIVITY);
    let up_answer = ok(&mut door, "run_trajectory", &up);
    assert_eq!(up_answer["outcome"], "ended", "the burst ends at its first maximum");
    let inlet = published(&up_answer["final_state"], "coolant_temperature_k");
    let down = fuel_burst(inlet, REACTIVITY, 0.0);
    let by_hand = ok(&mut door, "run_trajectory", &down);
    let request = chained(down.clone(), &up, &up_answer["run_seal"], Some("spert3_burst_to_fuel_card"));
    let coupled = ok(&mut door, "run_trajectory", &request);
    for key in RESULT_KEYS {
        assert_eq!(coupled[key], by_hand[key], "{key}");
    }
    let chain = &coupled["chain"];
    assert_eq!(chain["kind"], "coupling");
    assert_eq!(chain["coupling"], "spert3_burst_to_fuel_card");
    assert_eq!(chain["handed"][0]["from"], "final_state.coolant_temperature_k");
    assert_eq!(chain["handed"][0]["to"], "initial.inlet_temperature_k");
    assert_eq!(chain["handed"][0]["unit"], "K");
    assert_eq!(chain["handed"][1]["from"], "inputs.segments[*].load.reactivity_inserted_dollar");
    assert_eq!(chain["handed"][1]["unit"], "dollars");
    // The ledger at the coupling: handed over equals received, bit for bit.
    assert_eq!(chain["handed"][0]["value_hex"], up_answer["final_state"]["coolant_temperature_k_hex"]);
    assert_eq!(chain["handed"][0]["value_hex"], json!(format!("{:016x}", coupled["inputs"]["initial"]["inlet_temperature_k"].as_f64().unwrap().to_bits())));
    let report = ok(&mut door, "verify_tape", &json!({ "links": [up_answer, coupled] }));
    assert_eq!(report["holds"], json!(true), "{report}");
    let check = ok(&mut door, "verify_tape", &json!({ "request": request, "run_seal": coupled["run_seal"], "chain_seal": chain["chain_seal"] }));
    assert_eq!(check["holds"], json!(true), "{check}");

    for _ in 0..3 {
        // A swapped coupling name.
        refused_by_name(&mut door, "run_trajectory", &chained(down.clone(), &up, &up_answer["run_seal"], Some("spert3_fuel_burst_to_point_card")), "CouplingNotOfThesePieces");
        refused_by_name(&mut door, "run_trajectory", &chained(down.clone(), &up, &up_answer["run_seal"], Some("capsule_stardust_to_ablator_slab")), "UnknownCoupling");
        // The ledger at the coupling broken: a value received is not the value handed.
        let mut off = down.clone();
        off["segments"][0]["load"]["reactivity_inserted_dollar"] = json!(REACTIVITY.next_up());
        refused_by_name(&mut door, "run_trajectory", &chained(off, &up, &up_answer["run_seal"], Some("spert3_burst_to_fuel_card")), "CouplingLedgerBreach");
        let mut off = down.clone();
        off["initial"]["inlet_temperature_k"] = json!(inlet.next_down());
        refused_by_name(&mut door, "run_trajectory", &chained(off, &up, &up_answer["run_seal"], Some("spert3_burst_to_fuel_card")), "CouplingLedgerBreach");
        // A forged upstream seal.
        let mut forged_seal = up_answer["run_seal"].clone();
        flip_hex(&mut forged_seal);
        refused_by_name(&mut door, "run_trajectory", &chained(down.clone(), &up, &forged_seal, Some("spert3_burst_to_fuel_card")), "UpstreamProofBroken");
    }

    // The other direction: a hot-startup burst of the fuel card is handed on to
    // the point card, which takes any liquid water and was compared cold only;
    // its burst there is the same as at a cold coolant.
    let band = 2;
    let hot_inlet = 0.5 * (fuel::BAND_INLET_MIN_K[band] + fuel::BAND_INLET_MAX_K[band]);
    let hot_reactivity = 0.5 * (fuel::BAND_REACTIVITY_MIN_DOLLAR[band] + fuel::BAND_REACTIVITY_MAX_DOLLAR[band]);
    let hot_flow = 0.5 * (fuel::BAND_FLOW_MIN_M_PER_S[band] + fuel::BAND_FLOW_MAX_M_PER_S[band]);
    let hot = fuel_burst(hot_inlet, hot_reactivity, hot_flow);
    let hot_answer = ok(&mut door, "run_trajectory", &hot);
    let to_point = point_burst(hot_reactivity);
    let mut to_point = to_point;
    to_point["initial"]["coolant_temperature_k"] = json!(hot_inlet);
    let handed_hot = ok(&mut door, "run_trajectory", &chained(to_point.clone(), &hot, &hot_answer["run_seal"], Some("spert3_fuel_burst_to_point_card")));
    assert_eq!(handed_hot["outcome"], "ended", "{handed_hot}");
    // A point burst at a coolant temperature between the fuel card's bands is
    // outside the fuel card's declared inlet temperature.
    let between_k = 350.0;
    let mut warm_up = point_burst(REACTIVITY);
    warm_up["initial"]["coolant_temperature_k"] = json!(between_k);
    let warm_answer = ok(&mut door, "run_trajectory", &warm_up);
    for _ in 0..2 {
        let text = refused_by_name(&mut door, "run_trajectory", &chained(fuel_burst(between_k, REACTIVITY, 0.0), &warm_up, &warm_answer["run_seal"], Some("spert3_burst_to_fuel_card")), "CouplingOutsideRange");
        assert!(text.contains("three inlet bands"), "{text}");
    }
    // A cold burst of the fuel card hands on to the point card.
    let cold = fuel_burst(inlet, REACTIVITY, 0.0);
    let cold_answer = ok(&mut door, "run_trajectory", &cold);
    let back = ok(&mut door, "run_trajectory", &chained(point_burst(REACTIVITY), &cold, &cold_answer["run_seal"], Some("spert3_fuel_burst_to_point_card")));
    assert_eq!(back["run_seal"], up_answer["run_seal"], "the point card's burst, whichever way it was reached");
    // describe_card names each card's couplings.
    let described = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "reactor_spert3" }));
    let names: Vec<&str> = described["couplings"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["spert3_burst_to_fuel_card", "spert3_fuel_burst_to_point_card"]);
    let thermal = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "thermal" }));
    assert!(thermal.get("couplings").is_none());
}

/// The SPERT III couplings give, after the couplings that carry conditions
/// were added, the bytes engine 0.12.0 served: the chain seal of each
/// direction and of a coupled step, and every handed entry, read off the
/// served binary (`evidence/verification/engine_0.12.0_served/mcp_stdio`)
/// on the cards' worked runs.
#[test]
fn the_spert3_couplings_keep_the_seals_and_the_handed_bytes_of_0_12_0() {
    let mut door = local();
    let point = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "reactor_spert3" }));
    let fuel_card = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "reactor_spert3_fuel" }));
    assert!(point["couplings"].as_array().unwrap().iter().all(|entry| entry.get("conditions").is_none()), "{}", point["couplings"]);
    let up = point["example_run_trajectory"].clone();
    let up_answer = ok(&mut door, "run_trajectory", &up);
    let reactivity = up["segments"][0]["load"]["reactivity_inserted_dollar"].clone();
    let mut down = fuel_card["example_run_trajectory"].clone();
    down["initial"]["inlet_temperature_k"] = json!(published(&up_answer["final_state"], "coolant_temperature_k"));
    for segment in down["segments"].as_array_mut().unwrap() {
        segment["load"]["reactivity_inserted_dollar"] = reactivity.clone();
    }
    let to_fuel = ok(&mut door, "run_trajectory", &chained(down.clone(), &up, &up_answer["run_seal"], Some("spert3_burst_to_fuel_card")));
    let handed = |to: &str, inlet: &str| {
        json!([
            { "from": format!("final_state.{inlet}"), "to": format!("{to}.{}", if inlet == "coolant_temperature_k" { "inlet_temperature_k" } else { "coolant_temperature_k" }), "unit": "K", "value_hex": "4072ced82d82d82d" },
            { "from": "inputs.segments[*].load.reactivity_inserted_dollar", "to": if to == "parameters" { "parameters.reactivity_inserted_dollar" } else { "segments[*].load.reactivity_inserted_dollar" }, "unit": "dollars", "value_hex": "3feff1ecc51e8097" }
        ])
    };
    assert_eq!(to_fuel["chain"]["upstream_run_seal"], "4c8bdd5f99972231b3bb40f20787e89962b01270d577a2a3bff4196c1bc13c1d");
    assert_eq!(to_fuel["chain"]["handed"], handed("initial", "coolant_temperature_k"));
    assert_eq!(to_fuel["chain"]["chain_seal"], "31a4fc273e047c9019322fdea2ac1016983f7298ff57a4a1518a98993aad60af");
    let fuel_answer = ok(&mut door, "run_trajectory", &down);
    let mut back = up.clone();
    back["initial"]["coolant_temperature_k"] = json!(published(&fuel_answer["final_state"], "inlet_temperature_k"));
    let to_point = ok(&mut door, "run_trajectory", &chained(back, &down, &fuel_answer["run_seal"], Some("spert3_fuel_burst_to_point_card")));
    assert_eq!(to_point["chain"]["upstream_run_seal"], "8c369ac1d55d6fcd2cd5fa8f4cf52a3ce04a90a94773b5420fc0ffb7e81445c9");
    assert_eq!(to_point["chain"]["handed"], handed("initial", "inlet_temperature_k"));
    assert_eq!(to_point["chain"]["chain_seal"], "32e21fac572f4964169aefa542f42981f00b3b89190a098eb333b070b4987511");
    let mut step = fuel_card["example_compute_step"].clone();
    step["parameters"]["inlet_temperature_k"] = down["initial"]["inlet_temperature_k"].clone();
    step["parameters"]["reactivity_inserted_dollar"] = reactivity;
    let step = ok(&mut door, "compute_step", &chained(step, &up, &up_answer["run_seal"], Some("spert3_burst_to_fuel_card")));
    assert_eq!(step["chain"]["handed"], handed("parameters", "coolant_temperature_k"));
    assert_eq!(step["chain"]["chain_seal"], "96bd8e405e6e4f68621a12c1dc8e2201ca2c094879e3b8d6683a8803c67f846c");
}

// Cell m80 of the Tesla Model 3 2170 dataset, in its 40 degC block.

/// The capacity of cell m80's 40 degC block in both cards.
const M80_CAPACITY_40_AH: f64 = 4.67683635335689;

/// A run of the physical model of cell m80: a discharge at 4.5 A, then a rest.
fn model3_run(capacity_ah: f64, state_of_charge: f64, discharge_s: u64, rest_s: u64) -> Value {
    let mut segments = Vec::new();
    if discharge_s > 0 {
        segments.push(json!({ "steps": discharge_s, "load": { "current_a": 4.5, "case_temperature_c": 40.0 } }));
    }
    if rest_s > 0 {
        segments.push(json!({ "steps": rest_s, "load": { "current_a": 0.0, "case_temperature_c": 40.0 } }));
    }
    json!({
        "domain": "energy", "card": "cell_2170_model3",
        "initial": { "cell_number": 80.0, "state_of_charge": state_of_charge, "capacity_ah": capacity_ah, "case_temperature_least_c": 40.0, "case_temperature_most_c": 40.0 },
        "dt_s": 1.0, "segments": segments, "record_every": discharge_s + rest_s
    })
}

/// A run of the table card's cell m80: a discharge at 4.5 A, then a rest.
fn m80_run(block_chamber_c: f64, charge_removed_ah: f64, discharge_s: u64, rest_s: u64) -> Value {
    let mut segments = Vec::new();
    if discharge_s > 0 {
        segments.push(json!({ "steps": discharge_s, "load": { "current_a": -4.5, "case_temperature_c": 40.0 } }));
    }
    if rest_s > 0 {
        segments.push(json!({ "steps": rest_s, "load": { "current_a": 0.0, "case_temperature_c": 40.0 } }));
    }
    json!({
        "domain": "energy", "card": "cell_2170_m80",
        "initial": { "block_chamber_c": block_chamber_c, "charge_removed_ah": charge_removed_ah },
        "dt_s": 1.0, "segments": segments, "record_every": discharge_s + rest_s
    })
}

/// `request` with the case temperature of its last segment, its rest, set to `case_c`.
fn rest_at(request: &Value, case_c: f64) -> Value {
    let mut request = request.clone();
    request["segments"].as_array_mut().unwrap().last_mut().unwrap()["load"]["case_temperature_c"] = json!(case_c);
    request
}

/// A chain's links verify with the program of genesis_seal, without the engine.
fn verify_chain_holds(links: &Value, name: &str) {
    let program = seal_program();
    let path = scratch(name).join("chain.json");
    std::fs::write(&path, links.to_string()).unwrap();
    let out = Command::new(&program).arg("verify-chain").arg(&path).output().unwrap();
    let said = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(out.status.code(), Some(0), "{said}");
    assert!(said.contains("holds: true") && said.contains("links: 2"), "{said}");
}

/// Cell m80, rested in its 40 degC block, handed between its two cards:
/// each coupled answer is the downstream run typed by hand, bit for bit, with
/// its chain beside it, and the chain verifies by request, as links computed
/// again and with the program of genesis_seal. A coupled step is the step
/// typed by hand.
#[test]
fn cell_m80_rested_in_its_40_degc_block_is_handed_between_its_two_cards() {
    let mut door = local();
    // The physical model to the table card: the charge removed since full,
    // the cell at the capacity of the block.
    let up = model3_run(M80_CAPACITY_40_AH, 1.0, 1800, 3600);
    let up_answer = ok(&mut door, "run_trajectory", &up);
    assert_eq!(up_answer["outcome"], "completed");
    let charge = published(&up_answer["final_state"], "charge_removed_ah");
    let down = m80_run(40.0, charge, 600, 0);
    let by_hand = ok(&mut door, "run_trajectory", &down);
    let request = chained(down.clone(), &up, &up_answer["run_seal"], Some("cell_2170_model3_to_m80"));
    let coupled = ok(&mut door, "run_trajectory", &request);
    for key in RESULT_KEYS {
        assert_eq!(coupled[key], by_hand[key], "{key}");
    }
    let chain = &coupled["chain"];
    assert_eq!((chain["kind"].as_str(), chain["coupling"].as_str()), (Some("coupling"), Some("cell_2170_model3_to_m80")));
    assert_eq!(chain["upstream_run_seal"], up_answer["run_seal"]);
    assert_eq!(
        chain["handed"],
        json!([{ "from": "final_state.charge_removed_ah", "to": "initial.charge_removed_ah", "unit": "Ah", "value_hex": up_answer["final_state"]["charge_removed_ah_hex"] }])
    );
    assert_eq!(chain["handed"][0]["value_hex"], json!(format!("{:016x}", coupled["inputs"]["initial"]["charge_removed_ah"].as_f64().unwrap().to_bits())));
    let links = json!({ "links": [up_answer, coupled] });
    let report = ok(&mut door, "verify_tape", &links);
    assert_eq!((report["holds"].clone(), report["chain_seal"].clone()), (json!(true), chain["chain_seal"].clone()), "{report}");
    let check = ok(&mut door, "verify_tape", &json!({ "request": request, "run_seal": coupled["run_seal"], "chain_seal": chain["chain_seal"] }));
    assert_eq!((check["holds"].clone(), check["recomputed_chain_seal"].clone()), (json!(true), chain["chain_seal"].clone()), "{check}");
    verify_chain_holds(&links, "cell_model3_to_m80");
    // Into one step of the table card.
    let described = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "cell_2170_m80" }));
    let mut step = described["example_compute_step"].clone();
    step["parameters"]["block_chamber_c"] = json!(40.0);
    step["parameters"]["charge_removed_ah"] = json!(charge);
    let step_by_hand = ok(&mut door, "compute_step", &step);
    let step_coupled = ok(&mut door, "compute_step", &chained(step.clone(), &up, &up_answer["run_seal"], Some("cell_2170_model3_to_m80")));
    assert_eq!(step_coupled["sha256_seal"], step_by_hand["sha256_seal"]);
    assert_eq!(step_coupled["state"], step_by_hand["state"]);
    assert_eq!(step_coupled["chain"]["handed"][0]["to"], "parameters.charge_removed_ah");
    let report = ok(&mut door, "verify_tape", &json!({ "links": [up_answer, step_coupled] }));
    assert_eq!(report["holds"], json!(true), "{report}");

    // The table card to the physical model: the charge level as the state of
    // charge, into cell m80 at the block's capacity.
    let up = m80_run(40.0, 0.0, 1800, 3600);
    let up_answer = ok(&mut door, "run_trajectory", &up);
    assert_eq!(up_answer["outcome"], "completed");
    let level = published(&up_answer["final_state"], "charge_level");
    let down = model3_run(M80_CAPACITY_40_AH, level, 600, 0);
    let by_hand = ok(&mut door, "run_trajectory", &down);
    let request = chained(down.clone(), &up, &up_answer["run_seal"], Some("cell_2170_m80_to_model3"));
    let coupled = ok(&mut door, "run_trajectory", &request);
    for key in RESULT_KEYS {
        assert_eq!(coupled[key], by_hand[key], "{key}");
    }
    let chain = &coupled["chain"];
    assert_eq!(
        chain["handed"],
        json!([{ "from": "final_state.charge_level", "to": "initial.state_of_charge", "unit": "1", "value_hex": up_answer["final_state"]["charge_level_hex"] }])
    );
    // One quantity: the physical model starts at the charge the table card removed.
    let removed = published(&up_answer["final_state"], "charge_removed_ah");
    let started = (1.0 - level) * M80_CAPACITY_40_AH;
    assert!((started - removed).abs() < 1e-12, "{started} {removed}");
    let links = json!({ "links": [up_answer, coupled] });
    let report = ok(&mut door, "verify_tape", &links);
    assert_eq!(report["holds"], json!(true), "{report}");
    let check = ok(&mut door, "verify_tape", &json!({ "request": request, "run_seal": coupled["run_seal"], "chain_seal": chain["chain_seal"] }));
    assert_eq!(check["holds"], json!(true), "{check}");
    verify_chain_holds(&links, "cell_m80_to_model3");

    // A chain of three: the physical model, the table card, the physical model.
    let first = model3_run(M80_CAPACITY_40_AH, 1.0, 1200, 3600);
    let first_answer = ok(&mut door, "run_trajectory", &first);
    let second = chained(
        m80_run(40.0, published(&first_answer["final_state"], "charge_removed_ah"), 1200, 3600),
        &first,
        &first_answer["run_seal"],
        Some("cell_2170_model3_to_m80"),
    );
    let second_answer = ok(&mut door, "run_trajectory", &second);
    let third = chained(
        model3_run(M80_CAPACITY_40_AH, published(&second_answer["final_state"], "charge_level"), 600, 0),
        &second,
        &second_answer["run_seal"],
        Some("cell_2170_m80_to_model3"),
    );
    let third_answer = ok(&mut door, "run_trajectory", &third);
    assert_eq!(third_answer["chain"]["upstream_chain_seal"], second_answer["chain"]["chain_seal"]);
    let report = ok(&mut door, "verify_tape", &json!({ "links": [first_answer, second_answer, third_answer] }));
    assert_eq!(report["holds"], json!(true), "{report}");
}

/// Each condition of the two cell m80 couplings is refused by name, with no
/// proof and before any physics: sent with an upstream seal that is not the
/// engine's, a link that does not meet a condition is named for the
/// condition, where the same link meeting them all is computed again and
/// named UpstreamProofBroken. Outside the downstream range, with a value not
/// the one handed and with the other coupling's name, the link is refused
/// as every coupling's is.
#[test]
fn each_condition_of_the_cell_m80_couplings_is_refused_by_name_before_any_physics() {
    let mut door = local();
    let forged = json!("ab".repeat(32));
    let condition = |door: &mut Stdio2, request: &Value, says: &str| {
        for _ in 0..2 {
            let text = refused_by_name(door, "run_trajectory", request, "CouplingConditionNotMet");
            assert!(text.contains(says), "{says}: {text}");
            assert!(text.contains("so neither run is computed"), "{text}");
        }
    };
    // The physical model to the table card.
    let forward = |up: &Value, down: Value, seal: &Value| chained(down, up, seal, Some("cell_2170_model3_to_m80"));
    let up = model3_run(M80_CAPACITY_40_AH, 1.0, 1800, 3600);
    let down = m80_run(40.0, 2.25, 600, 0);
    refused_by_name(&mut door, "run_trajectory", &forward(&up, down.clone(), &forged), "UpstreamProofBroken");
    let mut other_cell = up.clone();
    other_cell["initial"]["cell_number"] = json!(1000.0);
    condition(&mut door, &forward(&other_cell, down.clone(), &forged), "the upstream request states cell_number 1000");
    // The physical model's own reading of the 40 degC block's capacity, to four places, is another capacity.
    condition(&mut door, &forward(&model3_run(4.6768, 1.0, 1800, 3600), down.clone(), &forged), "the upstream request states capacity_ah 4.6768");
    condition(&mut door, &forward(&model3_run(4.5006, 1.0, 1800, 3600), down.clone(), &forged), "the upstream request states capacity_ah 4.5006");
    condition(&mut door, &forward(&model3_run(M80_CAPACITY_40_AH, 1.0, 1800, 3599), down.clone(), &forged), "the upstream run ends with 3599 s at a current_a of zero");
    condition(&mut door, &forward(&model3_run(M80_CAPACITY_40_AH, 1.0, 1800, 0), down.clone(), &forged), "the upstream run ends with 0 s at a current_a of zero");
    condition(&mut door, &forward(&up, m80_run(25.0, 2.25, 600, 0), &forged), "the downstream request states block_chamber_c 25");
    // The hour of rest at a case temperature of the 40 degC block's compared
    // tests, 39.063 to 45.535 degC: a cold hour is refused, and at either
    // end of the band the link meets every condition and is computed, so
    // the forged seal is named.
    condition(&mut door, &forward(&rest_at(&up, -12.9), down.clone(), &forged), "the upstream request holds a case_temperature_c of -12.9 in its last 3600 s at a current_a of zero");
    condition(&mut door, &forward(&rest_at(&up, 25.0), down.clone(), &forged), "the upstream request holds a case_temperature_c of 25 in its last 3600 s");
    for case_c in [39.063, 45.535] {
        refused_by_name(&mut door, "run_trajectory", &forward(&rest_at(&up, case_c), down.clone(), &forged), "UpstreamProofBroken");
    }
    // A step of the table card in another block.
    let described = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "cell_2170_m80" }));
    let mut step = described["example_compute_step"].clone();
    step["parameters"]["block_chamber_c"] = json!(10.0);
    let text = refused_by_name(&mut door, "compute_step", &forward(&up, step, &forged), "CouplingConditionNotMet");
    assert!(text.contains("the downstream request states block_chamber_c 10"), "{text}");
    // The table card to the physical model.
    let reverse = |up: &Value, down: Value, seal: &Value| chained(down, up, seal, Some("cell_2170_m80_to_model3"));
    let up = m80_run(40.0, 0.0, 1800, 3600);
    let down = model3_run(M80_CAPACITY_40_AH, 0.5, 600, 0);
    refused_by_name(&mut door, "run_trajectory", &reverse(&up, down.clone(), &forged), "UpstreamProofBroken");
    condition(&mut door, &reverse(&m80_run(25.0, 0.0, 1800, 3600), down.clone(), &forged), "the upstream request states block_chamber_c 25");
    condition(&mut door, &reverse(&m80_run(40.0, 0.0, 1800, 60), down.clone(), &forged), "the upstream run ends with 60 s at a current_a of zero");
    condition(&mut door, &reverse(&rest_at(&up, -20.0), down.clone(), &forged), "the upstream request holds a case_temperature_c of -20 in its last 3600 s at a current_a of zero");
    for case_c in [39.063, 45.535] {
        refused_by_name(&mut door, "run_trajectory", &reverse(&rest_at(&up, case_c), down.clone(), &forged), "UpstreamProofBroken");
    }
    let mut other_cell = down.clone();
    other_cell["initial"]["cell_number"] = json!(448.0);
    condition(&mut door, &reverse(&up, other_cell, &forged), "the downstream request states cell_number 448");
    condition(&mut door, &reverse(&up, model3_run(4.6768, 0.5, 600, 0), &forged), "the downstream request states capacity_ah 4.6768");
    condition(&mut door, &reverse(&up, model3_run(M80_CAPACITY_40_AH.next_up(), 0.5, 600, 0), &forged), "the downstream request states capacity_ah 4.676836353356891");

    // With the upstream's own seal: outside the downstream range. A cell
    // started above full and rested has a charge removed below zero, past
    // the top of the table card's tables, -0.02 Ah.
    let above_full = model3_run(M80_CAPACITY_40_AH, 1.01, 0, 3600);
    let above_answer = ok(&mut door, "run_trajectory", &above_full);
    let removed = published(&above_answer["final_state"], "charge_removed_ah");
    assert!(removed < -0.02, "{removed}");
    for _ in 0..2 {
        let text = refused_by_name(&mut door, "run_trajectory", &forward(&above_full, m80_run(40.0, removed, 600, 0), &above_answer["run_seal"]), "CouplingOutsideRange");
        assert!(text.contains("the tables of cell_2170_m80's 40 degC block"), "{text}");
    }
    // A rest at the band's coldest end, with the upstream's own seal: the
    // coupled run is the run typed by hand.
    let edge = rest_at(&model3_run(M80_CAPACITY_40_AH, 1.0, 1800, 3600), 39.063);
    let edge_answer = ok(&mut door, "run_trajectory", &edge);
    let handed = m80_run(40.0, published(&edge_answer["final_state"], "charge_removed_ah"), 600, 0);
    let by_hand = ok(&mut door, "run_trajectory", &handed);
    let coupled = ok(&mut door, "run_trajectory", &forward(&edge, handed, &edge_answer["run_seal"]));
    assert_eq!(coupled["run_seal"], by_hand["run_seal"]);
    // A value received that is not the value handed; the other coupling's name.
    let up = model3_run(M80_CAPACITY_40_AH, 1.0, 1800, 3600);
    let up_answer = ok(&mut door, "run_trajectory", &up);
    let charge = published(&up_answer["final_state"], "charge_removed_ah");
    for _ in 0..2 {
        refused_by_name(&mut door, "run_trajectory", &forward(&up, m80_run(40.0, charge.next_up(), 600, 0), &up_answer["run_seal"]), "CouplingLedgerBreach");
        refused_by_name(&mut door, "run_trajectory", &chained(m80_run(40.0, charge, 600, 0), &up, &up_answer["run_seal"], Some("cell_2170_m80_to_model3")), "CouplingNotOfThesePieces");
        // Without a coupling's name the link is a continuation, which neither card takes.
        refused_by_name(&mut door, "run_trajectory", &chained(model3_run(M80_CAPACITY_40_AH, 0.5, 60, 0), &up, &up_answer["run_seal"], None), "ContinuationNotOnCard");
    }
}

/// describe_card lists each cell m80 coupling, with its conditions, on both
/// of its cards, and the engine serves four couplings.
#[test]
fn describe_card_lists_each_coupling_on_both_of_its_cards() {
    let mut door = local();
    let cards = ok(&mut door, "list_cards", &json!({}));
    let mut listed: Vec<(String, String)> = Vec::new();
    for entry in cards["cards"].as_array().unwrap() {
        let (domain, card) = (entry["domain"].as_str().unwrap(), entry["card"].as_str().unwrap());
        let described = ok(&mut door, "describe_card", &json!({ "domain": domain, "card": card }));
        for coupling in described.get("couplings").and_then(Value::as_array).into_iter().flatten() {
            let name = coupling["name"].as_str().unwrap().to_string();
            let here = format!("{domain}/{card}");
            assert!(coupling["upstream"] == json!(here) || coupling["downstream"] == json!(here), "{name} on {here}");
            listed.push((name, here));
        }
    }
    listed.sort();
    let names: Vec<&str> = listed.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "cell_2170_m80_to_model3", "cell_2170_m80_to_model3",
            "cell_2170_model3_to_m80", "cell_2170_model3_to_m80",
            "spert3_burst_to_fuel_card", "spert3_burst_to_fuel_card",
            "spert3_fuel_burst_to_point_card", "spert3_fuel_burst_to_point_card",
        ]
    );
    let m80 = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "cell_2170_m80" }));
    let model3 = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "cell_2170_model3" }));
    assert_eq!(m80["couplings"], model3["couplings"]);
    let entry = &m80["couplings"][0];
    let keys: Vec<&str> = entry.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["name", "upstream", "downstream", "handed", "conditions", "meaning"]);
    assert_eq!(entry["conditions"].as_array().unwrap().len(), 5);
    for coupling in m80["couplings"].as_array().unwrap() {
        let meaning = coupling["meaning"].as_str().unwrap();
        assert!(meaning.contains("40 degC block") && meaning.contains("bit for bit") && meaning.contains("4.67683635335689 Ah"), "{meaning}");
        assert!(!meaning.to_lowercase().contains("check"), "{meaning}");
    }
    // The short form lists no coupling.
    let short = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "cell_2170_m80", "form": "short" }));
    assert!(short.get("couplings").is_none());
}

/// Pass line (e): the chain's runs are capped by name before any physics:
/// at most 16 runs, and at most 1,000,000 steps together.
#[test]
fn the_chain_is_capped_by_name_before_any_physics() {
    let mut door = local();
    let plate = |steps: u64| json!({
        "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 1.0,
        "segments": [{ "steps": steps, "load": { "heat_power_w": 0.0, "ambient_k": 293.15 } }], "record_every": steps
    });
    let mut request = plate(1);
    for _ in 0..16 {
        request = chained(plate(1), &request, &json!("ab".repeat(32)), None);
    }
    refused_by_name(&mut door, "run_trajectory", &request, "ChainTooLong");
    let long = chained(plate(500_000), &plate(600_000), &json!("ab".repeat(32)), None);
    refused_by_name(&mut door, "run_trajectory", &long, "ChainStepsAboveLimit");
    // A from that is not one is named.
    let (error, text) = door.tool("run_trajectory", &json!({ "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 }, "dt_s": 1.0, "segments": [{ "steps": 1, "load": { "heat_power_w": 0.0, "ambient_k": 293.15 } }], "record_every": 1, "from": { "request": plate(1) } }));
    assert!(error && text.contains("'run_seal'"), "{text}");
}

/// Every connection gives the same bytes: the local, the web, the hosted with
/// a key and the open connection, for chained runs, a chained step, a
/// coupling, the checks, and the refusals.
#[tokio::test(flavor = "multi_thread")]
async fn every_connection_gives_the_same_bytes_for_a_chain() {
    let mut stdio = local();
    let r1 = mount_run(new_start(0.005, 0.1), 400);
    let a1 = ok(&mut stdio, "run_trajectory", &r1);
    let r2 = chained(mount_run(handed_back(&a1["final_state"]), 600), &r1, &a1["run_seal"], None);
    let a2 = ok(&mut stdio, "run_trajectory", &r2);
    let mut parameters = handed_back(&a1["final_state"]);
    parameters["accel_m_s2"] = json!(3.0);
    parameters["dt_s"] = json!(gate());
    let step = chained(json!({ "domain": "signal", "card": "resonance", "parameters": parameters }), &r1, &a1["run_seal"], None);
    let up = point_burst(REACTIVITY);
    let up_answer = ok(&mut stdio, "run_trajectory", &up);
    let inlet = published(&up_answer["final_state"], "coolant_temperature_k");
    let coupled = chained(fuel_burst(inlet, REACTIVITY, 0.0), &up, &up_answer["run_seal"], Some("spert3_burst_to_fuel_card"));
    let coupled_answer = ok(&mut stdio, "run_trajectory", &coupled);
    let mut broken = handed_back(&a1["final_state"]);
    broken["t_s"] = json!(broken["t_s"].as_f64().unwrap().next_up());
    let cell_up = model3_run(M80_CAPACITY_40_AH, 1.0, 600, 3600);
    let cell_up_answer = ok(&mut stdio, "run_trajectory", &cell_up);
    let cell_down = m80_run(40.0, published(&cell_up_answer["final_state"], "charge_removed_ah"), 300, 0);
    let cell_coupled = chained(cell_down.clone(), &cell_up, &cell_up_answer["run_seal"], Some("cell_2170_model3_to_m80"));
    let mut cell_elsewhere = cell_coupled.clone();
    cell_elsewhere["initial"]["block_chamber_c"] = json!(0.0);

    let base = web().await;
    let keyed = Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY),
            "cards": [["signal", "resonance"], ["energy", "reactor_spert3"], ["energy", "reactor_spert3_fuel"], ["energy", "cell_2170_m80"], ["energy", "cell_2170_model3"]],
            "solvers": [], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let open = Open::start();
    let calls = [
        ("run_trajectory", r2.clone()),
        ("verify_step", step),
        ("run_trajectory", coupled.clone()),
        ("verify_tape", json!({ "request": r2, "run_seal": a2["run_seal"], "chain_seal": a2["chain"]["chain_seal"] })),
        ("verify_tape", json!({ "links": [a1.clone(), a2.clone()] })),
        ("verify_tape", json!({ "links": [up_answer.clone(), coupled_answer.clone()] })),
        ("run_trajectory", chained(mount_run(broken, 50), &r1, &a1["run_seal"], None)),
        ("run_trajectory", chained(fuel_burst(inlet, REACTIVITY, 0.0), &up, &up_answer["run_seal"], Some("spert3_fuel_burst_to_point_card"))),
        ("describe_card", json!({ "domain": "energy", "card": "reactor_spert3_fuel" })),
        ("run_trajectory", cell_coupled.clone()),
        ("run_trajectory", cell_elsewhere),
        ("verify_tape", json!({ "request": cell_coupled, "run_seal": ok(&mut stdio, "run_trajectory", &cell_down)["run_seal"] })),
        ("describe_card", json!({ "domain": "energy", "card": "cell_2170_model3" })),
    ];
    for (tool, arguments) in &calls {
        let (error, local) = stdio.tool(tool, arguments);
        let (status, over_web) = post(&base, tool, arguments).await;
        assert_eq!(over_web, local, "{tool} web");
        assert_eq!(status == 200, !error, "{tool} web status {status}");
        let (status, hosted_error, hosted) = keyed.tool(KEY, tool, arguments);
        assert_eq!((status, hosted_error, hosted.as_str()), (200, error, local.as_str()), "{tool} hosted");
        let (open_error, opened) = open.tool(tool, arguments);
        assert_eq!((open_error, opened.as_str()), (error, local.as_str()), "{tool} open");
    }
    // A key that does not see the upstream card sees no such card.
    let narrow = Hosted::start(
        vec![json!({
            "client": "narrow", "key_sha256": sha(KEY),
            "cards": [["energy", "reactor_spert3_fuel"]],
            "solvers": [], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let (_, error, text) = narrow.tool(KEY, "run_trajectory", &coupled);
    assert!(error && text.contains("Card not recognized (UnknownCard)."), "{text}");
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

/// The cards whose returned state gives a start field under another name, as
/// each field's description says: ti64_lattice (start_t_s from t_s, and the
/// four other start values), cell_18650pf (the held current of the run
/// continued), generator_mmrtg_f1 and generator_mmrtg_seebeck (mission_time_s from t_s) and
/// tokamak_jet_dte1 (elapsed_time_s from t_s). Through `from` each stated
/// start value is held to the upstream's; the card alone could not tell a
/// returned value from a stated one.
#[test]
fn a_start_field_returned_under_another_name_is_carried_and_held() {
    let mut door = local();
    let sources: [(&str, &str, &[(&str, &str)]); 5] = [
        ("materials", "ti64_lattice", &[
            ("start_t_s", "final_state.t_s"),
            ("start_force_n", "final_state.applied_force_n"),
            ("start_temperature_k", "final_state.temperature_k"),
            ("start_energy_anchor_j", "final_state.energy_anchor_j"),
            ("start_boundary_work_j", "final_state.boundary_work_j"),
        ]),
        ("energy", "cell_18650pf", &[("held_current_a", "inputs.initial.held_current_a")]),
        ("energy", "generator_mmrtg_f1", &[("mission_time_s", "final_state.t_s")]),
        ("energy", "generator_mmrtg_seebeck", &[("mission_time_s", "final_state.t_s")]),
        ("energy", "tokamak_jet_dte1", &[("elapsed_time_s", "final_state.t_s")]),
    ];
    for (domain, card, renamed) in sources {
        let described = ok(&mut door, "describe_card", &json!({ "domain": domain, "card": card }));
        let upstream = described["example_run_trajectory"].clone();
        let answer = ok(&mut door, "run_trajectory", &upstream);
        assert_eq!(answer["outcome"], "completed", "{card}");
        let state = &answer["final_state"];
        let exact = |hex: &Value| -> Value {
            match hex {
                Value::String(text) => json!(f64::from_bits(u64::from_str_radix(text, 16).unwrap())),
                Value::Array(items) => Value::Array(items.iter().map(|item| json!(f64::from_bits(u64::from_str_radix(item.as_str().unwrap(), 16).unwrap()))).collect()),
                _ => panic!("{hex}"),
            }
        };
        let mut initial = serde_json::Map::new();
        for field in described["phase_fields"].as_array().unwrap() {
            let name = field["name"].as_str().unwrap();
            let source = renamed.iter().find(|(field, _)| *field == name).map(|(_, source)| *source);
            let value = match source {
                Some(path) if path.starts_with("inputs.initial.") => upstream["initial"][&path["inputs.initial.".len()..]].clone(),
                Some(path) => exact(&state[format!("{}_hex", &path["final_state.".len()..])]),
                None => exact(&state[format!("{name}_hex")]),
            };
            initial.insert(name.to_string(), value);
        }
        let mut by_hand_request = upstream.clone();
        by_hand_request["initial"] = Value::Object(initial.clone());
        let continued = ok(&mut door, "run_trajectory", &chained(by_hand_request.clone(), &upstream, &answer["run_seal"], None));
        // Typed by hand, a start that carries a run on is refused by name;
        // the generator's mission clock is its whole state, and a start at
        // any day is a fresh start, the same run as the chained one; the
        // cell has no memory, and a rested cell below full charge is a fresh
        // start too.
        let (error, text) = door.tool("run_trajectory", &by_hand_request);
        if card == "generator_mmrtg_f1" || card == "generator_mmrtg_seebeck" || card == "cell_18650pf" {
            assert!(!error, "{text}");
            let by_hand = parsed(&text);
            for key in RESULT_KEYS {
                assert_eq!(continued[key], by_hand[key], "{card} {key}");
            }
        } else {
            assert!(error && text.ends_with("(ContinuationWithoutUpstream)."), "{card}: {text}");
        }
        for (field, source) in renamed {
            let entry = continued["chain"]["handed"].as_array().unwrap().iter().find(|entry| entry["to"] == format!("initial.{field}")).unwrap_or_else(|| panic!("{card} {field}"));
            assert_eq!(entry["from"], *source, "{card}");
            // One unit in the last place off the upstream's is refused (from
            // zero, the least normal number: a subnormal meets the number rule).
            let nudge = |x: f64| if x == 0.0 { f64::MIN_POSITIVE } else { x.next_up() };
            let mut off = by_hand_request.clone();
            off["initial"][*field] = match &off["initial"][*field] {
                Value::Array(items) => Value::Array(items.iter().enumerate().map(|(i, v)| if i == 0 { json!(nudge(v.as_f64().unwrap())) } else { v.clone() }).collect()),
                value => json!(nudge(value.as_f64().unwrap())),
            };
            let text = refused_by_name(&mut door, "run_trajectory", &chained(off, &upstream, &answer["run_seal"], None), "ContinuationNotFromUpstream");
            assert!(text.contains(&format!("initial.{field}")), "{text}");
        }
    }
}

/// The start fields of a continuation of `answer`, read where the engine
/// reads them.
fn continued_initial(card: &str, described: &Value, upstream: &Value, answer: &Value) -> Value {
    let renamed: &[(&str, &str)] = match card {
        "ti64_lattice" => &[
            ("start_t_s", "t_s"),
            ("start_force_n", "applied_force_n"),
            ("start_temperature_k", "temperature_k"),
            ("start_energy_anchor_j", "energy_anchor_j"),
            ("start_boundary_work_j", "boundary_work_j"),
        ],
        "generator_mmrtg_f1" | "generator_mmrtg_seebeck" => &[("mission_time_s", "t_s")],
        "tokamak_jet_dte1" => &[("elapsed_time_s", "t_s")],
        _ => &[],
    };
    let exact = |hex: &Value| -> Value {
        match hex {
            Value::String(text) => json!(f64::from_bits(u64::from_str_radix(text, 16).unwrap())),
            Value::Array(items) => Value::Array(items.iter().map(|item| json!(f64::from_bits(u64::from_str_radix(item.as_str().unwrap(), 16).unwrap()))).collect()),
            _ => panic!("{card}: {hex}"),
        }
    };
    let mut initial = serde_json::Map::new();
    for field in described["phase_fields"].as_array().unwrap() {
        let name = field["name"].as_str().unwrap();
        let value = if card == "cell_18650pf" && name == "held_current_a" {
            upstream["initial"][name].clone()
        } else {
            let source = renamed.iter().find(|(field, _)| *field == name).map(|(_, source)| *source).unwrap_or(name);
            exact(&answer["final_state"][format!("{source}_hex")])
        };
        initial.insert(name.to_string(), value);
    }
    Value::Object(initial)
}

fn ulps_apart(a: &Value, b: &Value) -> u64 {
    match (a, b) {
        (Value::String(a), Value::String(b)) => {
            let a = u64::from_str_radix(a, 16).unwrap() as i64;
            let b = u64::from_str_radix(b, 16).unwrap() as i64;
            a.abs_diff(b)
        }
        (Value::Array(a), Value::Array(b)) => a.iter().zip(b).map(|(a, b)| ulps_apart(a, b)).max().unwrap_or(0),
        _ => u64::MAX,
    }
}

/// What describe_card says of a continuation, held to the engine: on every
/// card it serves, the card's worked run cut in two through from is compared
/// with the same run uncut (the segments twice), and the values that differ
/// are exactly the ones the statement names, those said to differ by one
/// unit in the last place by no more. On every card it refuses, a
/// continuation is refused by name with no proof.
#[test]
fn describe_card_says_per_card_what_a_continuation_carries_and_the_engine_agrees() {
    let mut door = local();
    let cards = ok(&mut door, "list_cards", &json!({}));
    let mut served = 0;
    let mut refused = 0;
    for entry in cards["cards"].as_array().unwrap() {
        let (domain, card) = (entry["domain"].as_str().unwrap(), entry["card"].as_str().unwrap());
        let described = ok(&mut door, "describe_card", &json!({ "domain": domain, "card": card }));
        let statement = &described["continuation"];
        let sentence = statement["sentence"].as_str().unwrap().to_string();
        let upstream = described["example_run_trajectory"].clone();
        if statement["served"] == json!(false) {
            refused += 1;
            assert!(sentence.contains("(ContinuationNotOnCard)"), "{card}: {sentence}");
            // A run of the card's own, completed, sent on through from.
            let mut short = upstream.clone();
            short["segments"] = json!([{ "steps": 1, "load": upstream["segments"][0]["load"] }]);
            short["record_every"] = json!(1);
            let (error, text) = door.tool("run_trajectory", &short);
            let answer = if error { None } else { Some(parsed(&text)) };
            let seal = answer.as_ref().map(|a| a["run_seal"].clone()).unwrap_or(json!("ab".repeat(32)));
            let mut next = upstream.clone();
            next["from"] = json!({ "request": short, "run_seal": seal });
            refused_by_name(&mut door, "run_trajectory", &next, "ContinuationNotOnCard");
            let mut step = described["example_compute_step"].clone();
            step["from"] = json!({ "request": short, "run_seal": seal });
            refused_by_name(&mut door, "verify_step", &step, "ContinuationNotOnCard");
            continue;
        }
        served += 1;
        let answer = ok(&mut door, "run_trajectory", &upstream);
        assert_eq!(answer["outcome"], "completed", "{card}");
        let mut down = upstream.clone();
        down["initial"] = continued_initial(card, &described, &upstream, &answer);
        let continued = ok(&mut door, "run_trajectory", &chained(down, &upstream, &answer["run_seal"], None));
        let mut uncut = upstream.clone();
        let mut segments = upstream["segments"].as_array().unwrap().clone();
        segments.extend(upstream["segments"].as_array().unwrap().clone());
        uncut["segments"] = Value::Array(segments);
        let straight = ok(&mut door, "run_trajectory", &uncut);
        assert_eq!(continued["outcome"], straight["outcome"], "{card}");
        let (cut, whole) = (continued["final_state"].as_object().unwrap(), straight["final_state"].as_object().unwrap());
        let mut differ: Vec<String> = Vec::new();
        let mut one_ulp: Vec<String> = Vec::new();
        for (key, value) in cut {
            if let Some(name) = key.strip_suffix("_hex") {
                if whole.get(key) != Some(value) {
                    differ.push(name.to_string());
                    if ulps_apart(value, &whole[key]) <= 1 {
                        one_ulp.push(name.to_string());
                    }
                }
            } else if value.is_boolean() {
                assert_eq!(Some(value), whole.get(key), "{card} {key}");
            }
        }
        differ.sort();
        if differ.is_empty() {
            assert!(sentence.contains("every published value bit for bit"), "{card}: {sentence}");
        } else {
            let named = sentence.split("bit for bit except in ").nth(1).unwrap_or_else(|| panic!("{card}: {sentence}"));
            let (list, ulps) = match named.split_once("; of these, ") {
                Some((list, rest)) => (list, rest.trim_end_matches('.').trim_end_matches(" differ by one unit in the last place")),
                None => (named.trim_end_matches('.'), ""),
            };
            let mut listed: Vec<String> = list.split(", ").map(str::to_string).collect();
            listed.sort();
            assert_eq!(listed, differ, "{card}");
            for name in ulps.split(", ").filter(|name| !name.is_empty()) {
                assert!(one_ulp.contains(&name.to_string()), "{card}: {name} differs by more than one unit");
            }
        }
    }
    assert_eq!((served, refused), (27, 10));
}

/// A key whose view holds one card of a coupling sees no coupling naming the
/// other: one SPERT III card, or one of the two cards of cell m80.
#[test]
fn a_keyed_view_lists_only_couplings_of_cards_it_holds() {
    let mut door = local();
    for (card, other) in [("reactor_spert3_fuel", "reactor_spert3"), ("cell_2170_m80", "cell_2170_model3"), ("cell_2170_model3", "cell_2170_m80")] {
        let narrow = Hosted::start(
            vec![json!({
                "client": "narrow", "key_sha256": sha(KEY),
                "cards": [["energy", card]],
                "solvers": [], "max_in_flight": 4, "max_permits": 2
            })],
            None,
        );
        let pair = json!({ "domain": "energy", "card": card });
        let (_, error, text) = narrow.tool(KEY, "describe_card", &pair);
        assert!(!error, "{text}");
        assert!(!text.contains("\"couplings\"") && !text.contains(&format!("energy/{other}\"")), "{text}");
        // The rest of the description is the full view's, the couplings, last in it, left out.
        let (_, full_text) = door.tool("describe_card", &pair);
        assert_eq!(text, format!("{}}}", &full_text[..full_text.find(",\"couplings\":").expect("the couplings")]), "{card}");
        let full = parsed(&full_text);
        assert_eq!(full["couplings"].as_array().unwrap().len(), 2, "{card}");
    }
    // A key that holds both cards of cell m80 sees both couplings.
    let both = Hosted::start(
        vec![json!({
            "client": "both", "key_sha256": sha(KEY),
            "cards": [["energy", "cell_2170_m80"], ["energy", "cell_2170_model3"]],
            "solvers": [], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let pair = json!({ "domain": "energy", "card": "cell_2170_m80" });
    let (_, error, text) = both.tool(KEY, "describe_card", &pair);
    assert!(!error, "{text}");
    assert_eq!(text, door.tool("describe_card", &pair).1);
}
