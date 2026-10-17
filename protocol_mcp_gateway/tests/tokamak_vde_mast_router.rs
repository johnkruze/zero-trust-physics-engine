//! The MAST vertical displacement event card on the HTTP door.
//!
//! A step is held bit for bit to the compiled kernel step with the same proof
//! bytes, and a run of two voltages to the kernel loop, with its tape
//! verified. Each published bound is refused by name at the door before a
//! step; a run that reaches the wall is served as the end, ended, sealed to
//! its last landed step; a run past the ideal-wall limit carries its flag. A
//! run continued through from lands on the uncut run's state bit for bit. The
//! description carries the kernel's sentence for every refusal and the
//! comparison's result; find_model and cite name the card's thing and sources.
//! The scored run of the comparison is reproduced to its seal by
//! `the_scored_comparison_run_reproduces_its_seal`, run by name with
//! `--ignored` (about 20 s in release).

mod answer_text;

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::tokamak_vde_mast::{self, TokamakVdeMast, VdeLoad, VdeState};
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const KERNEL_SOURCE: &str = include_str!("../../genesis_kernel/src/tokamak_vde_mast.rs");

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::builder().timeout(Duration::from_secs(600)).build().unwrap();
    let url = format!("{base}{path}");
    let text = answer_text::as_sent(body);
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(text.clone())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                let text = response.text().await.unwrap();
                answer_text::remember(&text);
                return (status, text);
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn above(x: f64) -> f64 {
    if x >= 0.0 { f64::from_bits(x.to_bits() + 1) } else { f64::from_bits(x.to_bits() - 1) }
}

fn blank_frame(proof: [u8; 16]) -> LastStateFrame64 {
    LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof,
    }
}

/// The growing mode at 1 mm, as the worked example states it.
const MODE: [f64; 4] = [0.001, 0.006546503766672119, -21.946577663159488, -8.150423262860404];

fn initial(y: [f64; 4]) -> Value {
    json!({ "displacement_m": y[0], "velocity_m_s": y[1], "wall_current_a": y[2], "p6_current_a": y[3] })
}

fn step_body(y: [f64; 4], volts: f64, dt_s: f64) -> Value {
    let mut parameters = initial(y);
    parameters["p6_voltage_v"] = json!(volts);
    parameters["dt_s"] = json!(dt_s);
    json!({ "domain": "energy", "card": "tokamak_vde_mast", "parameters": parameters })
}

fn run_body(y: [f64; 4], segments: &[(u64, f64)], dt_s: f64, record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "tokamak_vde_mast",
        "initial": initial(y),
        "dt_s": dt_s,
        "segments": segments.iter().map(|(steps, volts)| json!({ "steps": steps, "load": { "p6_voltage_v": volts } })).collect::<Vec<_>>(),
        "record_every": record_every,
    })
}

fn state_numbers(state: &VdeState) -> Vec<(&'static str, f64)> {
    vec![
        ("t_s", state.t_s),
        ("displacement_m", state.displacement_m),
        ("velocity_m_s", state.velocity_m_s),
        ("wall_current_a", state.wall_current_a),
        ("p6_current_a", state.p6_current_a),
        ("field_force_n", state.field_force_n),
        ("wall_force_n", state.wall_force_n),
        ("p6_force_n", state.p6_force_n),
        ("wall_power_w", state.wall_power_w),
        ("p6_power_w", state.p6_power_w),
        ("supply_power_w", state.supply_power_w),
        ("kinetic_energy_j", state.kinetic_energy_j),
        ("potential_energy_j", state.potential_energy_j),
        ("magnetic_energy_j", state.magnetic_energy_j),
        ("energy_j", state.energy_j),
        ("energy_anchor_j", state.energy_anchor_j),
        ("stored_scale_anchor_j", state.stored_scale_anchor_j),
        ("energy_supplied_j", state.energy_supplied_j),
        ("energy_dissipated_j", state.energy_dissipated_j),
        ("energy_defect_j", state.energy_defect_j),
        ("residual_conservation_error", state.residual_conservation_error),
        ("gap_to_wall_m", state.gap_to_wall_m),
        ("substeps", f64::from(state.substeps)),
    ]
}

fn assert_state_is(published: &Value, state: &VdeState, label: &str) {
    for (name, value) in state_numbers(state) {
        assert_eq!(
            published[format!("{name}_hex")].as_str().unwrap_or_else(|| panic!("{label} {name}")),
            format!("{:016x}", value.to_bits()),
            "{label} {name}"
        );
    }
    assert_eq!(published["past_ideal_wall_limit"], state.past_ideal_wall_limit, "{label}");
}

fn start(y: [f64; 4]) -> (TokamakVdeMast, VdeState) {
    let card = TokamakVdeMast::mast_15045();
    let state = VdeState::try_start(&card, y[0], y[1], y[2], y[3]).unwrap();
    (card, state)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_matches_the_compiled_step_with_the_same_proof_bytes() {
    let base = spawn_app().await;
    let cases = [
        (MODE, 0.0, tokamak_vde_mast::GATE_DT_S),
        ([0.05, 0.0, 0.0, 0.0], 0.0, 1.0e-6),
        ([0.0, 0.0, 0.0, 0.0], 300.0, 1.0e-4),
        ([-0.2, -3.0, 4.0e3, 500.0], -1_000.0, tokamak_vde_mast::FLOOR_DT_S),
    ];
    let mut seals = Vec::new();
    for (y, volts, dt_s) in cases {
        let request = step_body(y, volts, dt_s);
        let (status, answer) = post(&base, "/verify_physics", &request).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);
        let (card, mut state) = start(y);
        let mut frame = blank_frame(commit_request_proof(&request));
        tokamak_vde_mast::step(&card, &mut state, &mut frame, VdeLoad { p6_voltage_v: volts }, dt_s).unwrap();
        assert_state_is(&response.state, &state, "step");
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        seals.push(response.sha256_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_of_two_voltages_carries_the_kernel_loop_and_its_tape_verifies() {
    let base = spawn_app().await;
    // 150 V would drive the plasma to the wall within 7 ms: 5 V lifts it gently
    let segments = [(40_u64, 0.0), (40, 5.0)];
    let request = run_body(MODE, &segments, 1.0e-4, 8);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
    assert_eq!(tape["steps_landed"], 80);
    assert_eq!(tape["frames"].as_array().unwrap().len(), 10);
    let (card, mut state) = start(MODE);
    let mut frame = blank_frame([0; 16]);
    let mut worst = 0.0_f64;
    for (steps, volts) in segments {
        for _ in 0..steps {
            tokamak_vde_mast::step(&card, &mut state, &mut frame, VdeLoad { p6_voltage_v: volts }, 1.0e-4).unwrap();
            worst = worst.max(state.residual_conservation_error);
        }
    }
    assert_state_is(&tape["final_state"], &state, "run");
    assert_eq!(tape["max_residual_conservation_error_hex"], format!("{:016x}", worst.to_bits()));
    assert!(worst < 1.0e-9);
    // the amplifier's 5 V lifts the plasma faster than its own growth would
    assert!(state.energy_supplied_j > 0.0 && state.displacement_m > 0.001 * (6.5465_f64 * 8.0e-3).exp());

    let (status, verdict) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{verdict}");
    let verdict: Value = serde_json::from_str(&verdict).unwrap();
    assert_eq!(verdict["holds"], true, "{verdict}");

    // A frame altered by one bit is not the sealed tape.
    let mut forged = tape.clone();
    let hex_frame = forged["frames"][3]["frame_hex"].as_str().unwrap().to_string();
    let mut bytes = hex::decode(&hex_frame).unwrap();
    bytes[9] ^= 1;
    forged["frames"][3]["frame_hex"] = json!(hex::encode(bytes));
    let (_, verdict) = post(&base, "/verify_tape", &forged).await;
    assert!(!verdict.contains("\"holds\":true"), "{verdict}");

    // The run's own final value, sent back as a measurement, compares to zero.
    let series = json!({
        "run": request,
        "quantity": "p6_current_a",
        "measurements": [{ "step": 80, "value": state.p6_current_a }],
    });
    let (status, answer) = post(&base, "/validate_series", &series).await;
    assert_eq!(status, 200, "{answer}");
    let answer: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(answer["max_abs_error_hex"], "0000000000000000");
    assert_eq!(answer["run_seal"], tape["run_seal"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_limit_is_refused_by_name_before_a_step() {
    let base = spawn_app().await;
    let up = tokamak_vde_mast::UPPER_CONTACT_M;
    let down = tokamak_vde_mast::LOWER_CONTACT_M;
    let alfven = tokamak_vde_mast::ALFVEN_SPEED_M_S;
    let cases: Vec<(Value, &str)> = vec![
        (step_body([up, 0.0, 0.0, 0.0], 0.0, 1.0e-4), "StartOutsideVessel"),
        (step_body([down, 0.0, 0.0, 0.0], 0.0, 1.0e-4), "StartOutsideVessel"),
        (step_body([above(up), 0.0, 0.0, 0.0], 0.0, 1.0e-4), "StartOutsideVessel"),
        (step_body([0.5, 0.0, 0.0, 0.0], 0.0, 1.0e-4), "StartOutsideVessel"),
        (step_body([-1.0, 0.0, 0.0, 0.0], 0.0, 1.0e-4), "StartOutsideVessel"),
        (step_body([0.0, alfven, 0.0, 0.0], 0.0, 1.0e-4), "SpeedAtAlfvenSpeed"),
        (step_body([0.0, -alfven, 0.0, 0.0], 0.0, 1.0e-4), "SpeedAtAlfvenSpeed"),
        (step_body([0.0, 1.0e7, 0.0, 0.0], 0.0, 1.0e-4), "SpeedAtAlfvenSpeed"),
        (step_body(MODE, 0.0, above(tokamak_vde_mast::GATE_DT_S)), "DtAboveCeiling"),
        (step_body(MODE, 0.0, 1.0e-10), "DtBelowFloor"),
        (step_body(MODE, 0.0, 0.0), "NonPositiveDt"),
    ];
    for (request, name) in cases {
        let (status, answer) = post(&base, "/verify_physics", &request).await;
        assert_eq!(status, 422, "{name}: {answer}");
        assert!(answer.contains(&format!("({name})")), "{name}: {answer}");
        // the run of the same start is refused before its first step, with no proof
        let parameters = request["parameters"].clone();
        let y = [
            parameters["displacement_m"].as_f64().unwrap(),
            parameters["velocity_m_s"].as_f64().unwrap(),
            0.0,
            0.0,
        ];
        let run = run_body(y, &[(3, 0.0)], parameters["dt_s"].as_f64().unwrap(), 1);
        let (status, answer) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 422, "{name}: {answer}");
        assert!(answer.contains(&format!("({name})")) && !answer.contains("run_seal"), "{name}: {answer}");
    }
    // A key the card does not take is refused by name.
    let mut extra = step_body(MODE, 0.0, 1.0e-4);
    extra["parameters"]["decay_index"] = json!(-1.0);
    let (status, answer) = post(&base, "/verify_physics", &extra).await;
    assert_eq!(status, 422, "{answer}");
    assert!(answer.contains("decay_index"), "{answer}");
}

/// From 0.30 m rising, past the ideal-wall limit, the run reaches the wall:
/// the outcome is ended, sealed to the last landed step, and every frame past
/// the limit carries its flag. The tape verifies.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_to_the_wall_is_the_end_of_the_event_and_is_sealed_to_its_last_step() {
    let base = spawn_app().await;
    let k = 300.0;
    let y = [0.30, MODE[1] * k, MODE[2] * k, MODE[3] * k];
    let request = run_body(y, &[(5_000, 0.0)], 1.0e-6, 1);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "ended", "{answer}");
    assert_eq!(tape["reject"], "PlasmaAtWall");
    let landed = tape["steps_landed"].as_u64().unwrap();
    assert!(landed > 0 && landed < 5_000, "{landed}");
    assert!(tape["reject_detail"].as_str().unwrap().contains("Ended: the run reached the end the tokamak_vde_mast card declares (PlasmaAtWall)"), "{}", tape["reject_detail"]);
    assert_eq!(tape["final_state"]["past_ideal_wall_limit"], true);
    let flagged = tape["frames"].as_array().unwrap().iter().all(|frame| {
        let bytes = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap();
        u64::from_le_bytes(bytes[40..48].try_into().unwrap()) == tokamak_vde_mast::FLAG_PAST_IDEAL_WALL_LIMIT
    });
    assert!(flagged);
    // the kernel loop ends at the same step with the same name
    let (card, mut state) = start(y);
    let mut frame = blank_frame([0; 16]);
    let mut steps = 0;
    let reject = loop {
        match tokamak_vde_mast::step(&card, &mut state, &mut frame, VdeLoad { p6_voltage_v: 0.0 }, 1.0e-6) {
            Ok(()) => steps += 1,
            Err(reject) => break reject,
        }
    };
    assert_eq!(reject, tokamak_vde_mast::StepReject::PlasmaAtWall);
    assert_eq!(steps, landed);
    assert_state_is(&tape["final_state"], &state, "ended");
    let (status, verdict) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{verdict}");
    assert!(verdict.contains("\"holds\":true"), "{verdict}");
}

/// The red team's start of 0.12.0 (plasma at rest at the equilibrium, 1 MA in
/// P6, 0 V): through the door every step from 1e-8 s to 1 ms is refused as
/// SpeedAtAlfvenSpeed, sealed to its last landed step, where 0.12.0 as frozen
/// ended the 1 ms and 1e-7 s runs at the wall, PlasmaAtWall, and refused only
/// the 1e-8 s run. The speed is tested at every stage, as the wall is; each
/// run stops on the kernel loop's step.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_alfven_speed_is_met_at_every_step_asked_for_and_never_passes_for_the_wall() {
    let base = spawn_app().await;
    let y = [0.0, 0.0, 0.0, 1.0e6];
    for (dt_s, steps) in [(1.0e-3, 1), (1.0e-5, 10), (1.0e-7, 100), (1.0e-8, 1_000)] {
        let (status, answer) = post(&base, "/run_trajectory", &run_body(y, &[(steps, 0.0)], dt_s, 1)).await;
        assert_eq!(status, 200, "{dt_s}: {answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["outcome"], "refused", "{dt_s}: {answer}");
        assert_eq!(tape["reject"], "SpeedAtAlfvenSpeed", "{dt_s}: {answer}");
        assert!(tape["reject_detail"].as_str().unwrap().contains("at the start or at a stage of a substep, where the wall is tested too"), "{}", tape["reject_detail"]);
        let (card, mut state) = start(y);
        let mut frame = blank_frame([0; 16]);
        let mut landed = 0_u64;
        let reject = loop {
            match tokamak_vde_mast::step(&card, &mut state, &mut frame, VdeLoad { p6_voltage_v: 0.0 }, dt_s) {
                Ok(()) => landed += 1,
                Err(reject) => break reject,
            }
        };
        assert_eq!(reject, tokamak_vde_mast::StepReject::SpeedAtAlfvenSpeed, "{dt_s}");
        assert_eq!(tape["steps_landed"].as_u64(), Some(landed), "{dt_s}");
        if landed > 0 {
            assert_state_is(&tape["final_state"], &state, "refused at the Alfven speed");
            let (status, verdict) = post(&base, "/verify_tape", &tape).await;
            assert_eq!(status, 200, "{verdict}");
            assert!(verdict.contains("\"holds\":true"), "{verdict}");
        }
        // the limit lies in the refused step: 3.4e-7 s after the start
        assert!(state.t_s <= 3.5e-7 && state.t_s + dt_s > 3.4e-7, "{dt_s}: {}", state.t_s);
    }
}

/// A run continued through from at its returned state lands where the uncut
/// run lands: the displacement, the velocity and both currents bit for bit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_continued_through_from_lands_on_the_uncut_run() {
    let base = spawn_app().await;
    let upstream = run_body(MODE, &[(30, 0.0)], 1.0e-4, 10);
    let (status, answer) = post(&base, "/run_trajectory", &upstream).await;
    assert_eq!(status, 200, "{answer}");
    let first: Value = serde_json::from_str(&answer).unwrap();
    let exact = |name: &str| f64::from_bits(u64::from_str_radix(first["final_state"][format!("{name}_hex")].as_str().unwrap(), 16).unwrap());
    let y = [exact("displacement_m"), exact("velocity_m_s"), exact("wall_current_a"), exact("p6_current_a")];
    let mut down = run_body(y, &[(30, 0.0)], 1.0e-4, 10);
    down["from"] = json!({ "request": upstream, "run_seal": first["run_seal"] });
    let (status, answer) = post(&base, "/run_trajectory", &down).await;
    assert_eq!(status, 200, "{answer}");
    let continued: Value = serde_json::from_str(&answer).unwrap();
    let (status, answer) = post(&base, "/run_trajectory", &run_body(MODE, &[(30, 0.0), (30, 0.0)], 1.0e-4, 10)).await;
    assert_eq!(status, 200, "{answer}");
    let straight: Value = serde_json::from_str(&answer).unwrap();
    for name in ["displacement_m", "velocity_m_s", "wall_current_a", "p6_current_a", "gap_to_wall_m", "wall_force_n"] {
        let key = format!("{name}_hex");
        assert_eq!(continued["final_state"][&key], straight["final_state"][&key], "{name}");
    }
    assert_ne!(continued["final_state"]["t_s_hex"], straight["final_state"]["t_s_hex"]);
}

/// `(variant, doc sentence)` for every variant of the kernel's `StepReject`.
fn documented_rejects(source: &str) -> Vec<(String, String)> {
    let start = source.find("pub enum StepReject {").expect("StepReject is in the card");
    let mut documented = Vec::new();
    let mut doc = String::new();
    for line in source[start..].lines().skip(1) {
        if line == "}" {
            break;
        }
        let text = line.trim_start();
        if let Some(sentence) = text.strip_prefix("/// ") {
            if !doc.is_empty() {
                doc.push(' ');
            }
            doc.push_str(sentence);
        } else if let Some(name) = text.strip_suffix(',') {
            documented.push((name.to_string(), doc.clone()));
            doc.clear();
        }
    }
    documented
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_description_carries_the_kernel_sentences_the_end_and_the_result() {
    let base = spawn_app().await;
    let (status, answer) = post(&base, "/describe_card", &json!({ "domain": "energy", "card": "tokamak_vde_mast" })).await;
    assert_eq!(status, 200, "{answer}");
    let body: Value = serde_json::from_str(&answer).unwrap();
    let served: Vec<(String, String)> = body["rejects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["name"].as_str().unwrap().to_string(), row["meaning"].as_str().unwrap_or("").to_string()))
        .collect();
    let documented = documented_rejects(KERNEL_SOURCE);
    assert_eq!(documented.len(), 10);
    for (name, sentence) in &documented {
        let row = served.iter().find(|(served_name, _)| served_name == name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(&row.1, sentence, "{name}");
    }
    assert_eq!(body["declared_ends"][0]["reject"], "PlasmaAtWall");
    let record = &body["validation"];
    assert!(record["pass_line"].as_str().unwrap().contains("3743912cffaa1274685092dd247e5aaf596f7af0f1e8275be7a1e3ccdc4b9f70"));
    let result = record["result"].as_str().unwrap();
    for words in ["met in part, one line of three", "6.55 per second against Windridge's 96.3", "7.10 per second", "121.3 per second", "-2,933 ampere-turns", "-3,126"] {
        assert!(result.contains(words), "{words}");
    }
    assert!(record["object"].as_str().unwrap().starts_with("The company's own derivation, from the MAST spherical tokamak"));
    assert!(record["data"].as_str().unwrap().contains("Creative Commons 4.0 BY-SA"));
    assert_eq!(body["continuation"]["served"], true);
    let list: Value = Client::new().get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let row = list["cards"].as_array().unwrap().iter().find(|row| row["card"] == "tokamak_vde_mast").unwrap();
    assert_eq!(row["against_measurement"], "met in part");
    // the frame's force slot carries the wall's force on the plasma, in newtons
    let slots = body["frame_slots"].as_array().unwrap();
    let force = slots.iter().find(|slot| slot["slot"] == "force_torque").unwrap().to_string();
    assert!(force.contains("wall_force_n") && force.contains("\"N\""), "{force}");
    // the card's own words hold none of an earlier version's
    let own = [
        body["specimen"].to_string(),
        body["validation"].to_string(),
        body["phase_fields"].to_string(),
        body["load_fields"].to_string(),
        body["dt_s"]["ceiling_from"].to_string(),
        body["dt_s"]["floor_from"].to_string(),
    ];
    for text in own {
        assert!(!text.contains("check") && !text.contains("verify_step"), "{text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_model_and_cite_name_the_cards_thing() {
    let base = spawn_app().await;
    let (status, answer) = post(&base, "/find_model", &json!({ "description": "a vertical displacement event of the plasma in the MAST spherical tokamak after the position feedback is cut" })).await;
    assert_eq!(status, 200, "{answer}");
    let found: Value = serde_json::from_str(&answer).unwrap();
    assert!(found["matches"][0].to_string().contains("tokamak_vde_mast"), "{answer}");
    let (status, answer) = post(&base, "/find_model", &json!({ "description": "a deuterium-tritium plasma in the JET tokamak under neutral beam heating" })).await;
    assert_eq!(status, 200, "{answer}");
    let jet: Value = serde_json::from_str(&answer).unwrap();
    assert!(jet["matches"][0].to_string().contains("tokamak_jet_dte1"), "{answer}");
    let (status, answer) = post(&base, "/cite", &json!({ "domain": "energy", "card": "tokamak_vde_mast" })).await;
    assert_eq!(status, 200, "{answer}");
    assert!(answer.contains("FAIR-MAST") && answer.contains("Windridge") && answer.contains("10.1109/TPS.2025.3583419"), "{answer}");
}

/// The run the comparison scored: 1,000 steps of 1 ms asked, from the growing
/// mode at 1 mm at zero volts. It ends at the wall after 729 and its seal is
/// the one the description states. About 20 s in release.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn the_scored_comparison_run_reproduces_its_seal() {
    let base = spawn_app().await;
    let request = run_body(MODE, &[(1_000, 0.0)], 1.0e-3, 1);
    let (status, answer) = post(&base, "/run_trajectory", &request).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "ended");
    assert_eq!(tape["steps_landed"], 729);
    assert_eq!(tape["run_seal"], "edc17e3633530d53756b14884dc9fcd0c9d7392c32a7ffffaf0d4ee156bd7a99");
}
