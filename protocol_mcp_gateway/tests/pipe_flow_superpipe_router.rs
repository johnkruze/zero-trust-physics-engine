//! `pipe_flow_superpipe` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door and its description.

mod answer_text;

use std::time::Duration;

use genesis_solvers::pipe_flow_superpipe::{self as pipe, PipeQuestion};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

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
    let client = Client::new();
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

fn flow(re: f64, p: f64, t: f64, k: f64) -> Value {
    json!({ "solver": "pipe_flow_superpipe", "parameters": { "bulk_reynolds_number": re, "static_pressure_pa": p, "temperature_k": t, "wall_roughness_rms_um": k } })
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// The hex is the solver's bits, and the decimal beside it reads back to them.
fn assert_number(section: &Value, key: &str, expected: f64) {
    assert_eq!(section[format!("{key}_hex")], hex_of(expected), "{key}");
    let decimal = section[key].as_f64().unwrap_or_else(|| panic!("{key}"));
    assert_eq!(decimal.to_bits(), expected.to_bits(), "{key}");
}

fn assert_numbers(section: &Value, key: &str, expected: &[f64]) {
    let hex: Vec<String> = expected.iter().map(|value| hex_of(*value)).collect();
    assert_eq!(section[format!("{key}_hex")], json!(hex), "{key}");
    let decimals = section[key].as_array().unwrap_or_else(|| panic!("{key}"));
    assert_eq!(decimals.len(), expected.len(), "{key}");
    for (decimal, value) in decimals.iter().zip(expected) {
        assert_eq!(decimal.as_f64().unwrap().to_bits(), value.to_bits(), "{key}");
    }
}

fn flip_hex(text: &str) -> String {
    let last = if text.ends_with('0') { '1' } else { '0' };
    format!("{}{last}", &text[..text.len() - 1])
}

fn text_of(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

/// A held-back condition, McKeon et al. 2004 survey 26 (Re 3.5724e7 at
/// 17.89 MPa), answered bit for bit as the solver gives it, with its proof.
#[tokio::test]
async fn a_compared_condition_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let request = flow(35_724_000.0, 17_890_000.0, 297.25, 0.15);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = pipe::solve(&PipeQuestion {
        bulk_reynolds_number: 35_724_000.0,
        static_pressure_pa: 17_890_000.0,
        temperature_k: 297.25,
        wall_roughness_rms_um: 0.15,
    })
    .expect("the solver settles");

    assert_eq!(body["inputs"], request);
    assert_eq!(body["engine_version"], format!("genesis_solvers_v{}", genesis_solvers::VERSION));
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert_eq!(body["iterations"], direct.newton_iterations);
    let result = &body["result"];
    assert_number(result, "darcy_friction_factor", direct.darcy_friction_factor);
    assert_number(result, "centreline_to_bulk_velocity_ratio", direct.centreline_to_bulk_velocity_ratio);
    assert_number(result, "friction_to_bulk_velocity_ratio", direct.friction_to_bulk_velocity_ratio);
    assert_number(result, "karman_number", direct.karman_number);
    assert_number(result, "pressure_drop_per_m_over_bulk_dynamic_pressure", direct.pressure_drop_per_m_over_bulk_dynamic_pressure);
    assert_numbers(result, "profile_y_over_radius", &direct.profile_y_over_radius);
    assert_numbers(result, "profile_y_plus", &direct.profile_y_plus);
    assert_numbers(result, "profile_u_plus", &direct.profile_u_plus);
    assert_numbers(result, "profile_u_over_bulk", &direct.profile_u_over_bulk);
    assert_number(result, "bulk_mach_number", direct.bulk_mach_number);
    assert_eq!(result.as_object().unwrap().len(), 20, "ten values and their hex");
    // At 35.7 million only the stations from 0.015 of the radius out are
    // served: nearer than 0.010591, the Pitot profiles' nearest point, nothing
    // was compared.
    assert_eq!(direct.profile_y_over_radius.len(), 14);
    assert_eq!(direct.profile_y_over_radius[0], 0.015);
    assert_eq!(result["profile_y_over_radius"][0], 0.015);
    // The scored miss of the friction factor at this condition: 2.69 percent low.
    let error = direct.darcy_friction_factor / 0.0070443 - 1.0;
    assert!((error + 0.0269).abs() < 0.0005, "{error}");
    let invariants = &body["invariants"];
    assert_number(invariants, "largest_imbalance", direct.largest_imbalance);
    assert_number(invariants, "flux_residual", direct.flux_residual);
    assert_number(invariants, "momentum_ledger_residual", direct.momentum_ledger_residual);
    assert!(direct.largest_imbalance <= pipe::IMBALANCE_TOLERANCE);
    assert!(direct.momentum_ledger_residual.abs() <= pipe::MOMENTUM_LEDGER_TOLERANCE);

    // The field hash is the count of numbers, the node positions and the velocities.
    let mut hasher = Sha256::new();
    hasher.update((direct.field_y_over_radius.len() as u64 * 2).to_le_bytes());
    for value in direct.field_y_over_radius.iter().chain(&direct.field_u_over_bulk) {
        hasher.update(value.to_le_bytes());
    }
    let own = hex::encode(hasher.finalize());
    let served = body["field_sha256"].as_str().expect("field hash");
    assert_eq!(served, own);

    // The proof holds, and an edit of a number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    for (section, key) in [("result", "darcy_friction_factor_hex"), ("result", "centreline_to_bulk_velocity_ratio_hex"), ("invariants", "largest_imbalance_hex"), ("invariants", "momentum_ledger_residual_hex")] {
        let mut edited = body.clone();
        edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
        let (status, text) = post(&base, "/verify_solution", &edited).await;
        assert_eq!(status, 200, "{text}");
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }
    let mut edited = body.clone();
    edited["result"]["profile_u_plus_hex"][10] = json!(flip_hex(body["result"]["profile_u_plus_hex"][10].as_str().unwrap()));
    let (_, text) = post(&base, "/verify_solution", &edited).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);
    let mut field = body.clone();
    field["field_sha256"] = json!(flip_hex(served));
    let (_, text) = post(&base, "/verify_solution", &field).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(text_of(&body), text_of(&serde_json::from_str::<Value>(&again).unwrap()));
}

#[tokio::test]
async fn a_question_past_a_limit_of_the_physics_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let cases = [
        (flow(2000.0, 101_325.0, 296.0, 0.15), "LaminarFlow"),
        (flow(2300.0, 101_325.0, 296.0, 0.15), "TransitionalFlow"),
        (flow(3999.0, 101_325.0, 296.0, 0.15), "TransitionalFlow"),
        (flow(1.0e10_f64.next_up(), 3.0e9, 296.0, 0.15), "ReynoldsAboveGrid"),
        (flow(1.0e11, 3.0e10, 296.0, 0.15), "ReynoldsAboveGrid"),
        (flow(-1.0, 101_325.0, 296.0, 0.15), "LaminarFlow"),
        (flow(1.0e5, 0.0, 296.0, 0.15), "PressureNotPositive"),
        (flow(1.0e5, -101_325.0, 296.0, 0.15), "PressureNotPositive"),
        (flow(1.0e5, 101_325.0, 169.99999999999997, 0.15), "TemperatureOutsideViscosityLaw"),
        (flow(1.0e5, 101_325.0, 1900.0000000000002, 0.15), "TemperatureOutsideViscosityLaw"),
        (flow(3.5e7, 101_325.0, 296.0, 0.15), "MachAboveIncompressible"),
        (flow(1.0e6, 101_325.0, 296.0, 0.15), "MachAboveIncompressible"),
        (flow(1.0e5, 101_325.0, 296.0, 0.15000000000000002), "RoughWall"),
        (flow(1.0e5, 101_325.0, 296.0, 2.5), "RoughWall"),
        (flow(1.0e5, 101_325.0, 296.0, -0.01), "RoughnessNegative"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name}).")), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
        assert!(!text.contains("..="), "{text}");
    }
    let (status, text) = post(&base, "/solve", &flow(1.0e5, 101_325.0, 100.0, 0.15)).await;
    assert_eq!(status, 422);
    assert_eq!(
        text,
        "Physical Violation: temperature 100 K is outside 170 to 1900 K, where Sutherland's law gives the viscosity of air, so the bulk velocity cannot be known (TemperatureOutsideViscosityLaw)."
    );
    let (status, text) = post(&base, "/solve", &flow(2000.0, 101_325.0, 296.0, 0.15)).await;
    assert_eq!(status, 422);
    assert_eq!(
        text,
        "Physical Violation: bulk Reynolds number 2000 is below 2300: the flow is laminar, and this solver is of turbulent flow (LaminarFlow)."
    );
    let others = [
        (
            json!({ "solver": "pipe_flow_superpipe", "parameters": { "bulk_reynolds_number": 1.0e5, "static_pressure_pa": 101_325.0, "temperature_k": 296.0 } }),
            "Missing required parameter: 'wall_roughness_rms_um' (InputMissing).".to_string(),
        ),
        (
            json!({ "solver": "pipe_flow_superpipe", "parameters": { "bulk_reynolds_number": 1.0e5, "static_pressure_pa": 101_325.0, "temperature_k": 296.0, "wall_roughness_rms_um": 0.15, "diameter_m": 0.2 } }),
            "Physical Violation: 'diameter_m' is not an input of this solver. The bore, the closure and its constants (kappa 0.41, A+ 26, Nikuradse's outer shape), the grid and the tolerances stay inside the solver.".to_string(),
        ),
    ];
    for (request, message) in others {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert_eq!(text, message, "{request}");
    }
}

/// Every end of the served range is answered and sealed, and so is each
/// compared run at the corners of the compared envelope, and each question
/// past it where engine 0.8.0 refused by name: Reynolds numbers below
/// 31,577 and above 35,724,000, pressures, temperatures and bulk velocities
/// no compared run had.
#[tokio::test]
async fn the_ends_of_the_range_and_the_compared_corners_are_answered() {
    let base = spawn_app().await;
    for request in [
        flow(4000.0, 101_325.0, 296.0, 0.15),
        flow(31_576.999999999996, 101_325.0, 296.0, 0.15),
        flow(35_724_000.000000004, 18_000_000.0, 296.0, 0.15),
        flow(1.0e5, 100_089.99999999999, 296.0, 0.15),
        flow(3.5e7, 18_928_000.000000004, 296.0, 0.15),
        flow(1.0e5, 101_325.0, 291.78, 0.15),
        flow(1.0e5, 101_325.0, 350.0, 0.15),
        flow(1.0e5, 101_325.0, 170.0, 0.15),
        flow(4.0e4, 1.0e6, 1900.0, 0.15),
        flow(40_000.0, 1.0e6, 296.0, 0.15),
        flow(1.0e10, 3.0e9, 296.0, 0.15),
        flow(31_577.0, 100_290.0, 300.12, 0.15),
        flow(41_727.0, 100_090.0, 300.13, 0.15),
        flow(246_600.0, 344_505.0, 291.79, 0.15),
        flow(35_259_000.0, 18_928_000.0, 296.69, 0.15),
        flow(35_724_000.0, 17_890_000.0, 297.25, 0.0),
        flow(81_330.0, 101_325.0, 292.4, 0.15),
    ] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{request} {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{request}");
        assert!(body["solution_seal"].is_string());
        let y_plus = body["result"]["profile_y_plus"].as_array().unwrap();
        assert!(y_plus.iter().all(|y| y.as_f64().unwrap() >= 50.0));
        let mach = body["result"]["bulk_mach_number"].as_f64().unwrap();
        assert!(mach > 0.0 && mach <= pipe::MACH_MAX, "{request} {mach}");
        assert!(body["invariants"]["momentum_ledger_residual"].as_f64().unwrap().abs() <= pipe::MOMENTUM_LEDGER_TOLERANCE);
        let (status, checked) = post(&base, "/verify_solution", &body).await;
        assert_eq!(status, 200, "{checked}");
        assert_eq!(serde_json::from_str::<Value>(&checked).unwrap()["holds"], true, "{checked}");
    }
}

#[tokio::test]
async fn the_description_carries_the_standing_and_its_example_runs_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 25);
    assert_eq!(solvers[12]["solver"], "pipe_flow_superpipe");
    let summary = solvers[12]["summary"].as_str().expect("a sentence");
    assert!(summary.contains("Nothing fitted"), "{summary}");
    assert!(summary.contains("friction factor within 1.38 percent RMS, worst 2.69"), "{summary}");
    assert!(summary.contains("2.99 percent low on the NSTAP profile at 81,330"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "pipe_flow_superpipe" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["summary"], summary);
    for part in ["object", "data", "pass_line", "result", "limits"] {
        let words = described["validation"][part].as_str().unwrap_or_else(|| panic!("{part}"));
        assert!(!words.is_empty(), "{part}");
    }
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.contains("704700a99d27cdd566d47a92182d363f31ad8d1ee774c1e75c84a8de75b26893"), "{pass_line}");
    let result = described["validation"]["result"].as_str().unwrap();
    for words in [
        "RMS 1.38 percent (target 3), worst 2.69 percent",
        "pooled RMS of u+ 1.59 percent (target 2)",
        "the NSTAP profile at 81,330 is 2.99 percent low (target 2)",
        "20 profiles are 0.75 to 1.79 percent low",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 4);
    assert_eq!(inputs[0]["minimum"], pipe::TRANSITIONAL_BELOW_REYNOLDS);
    assert_eq!(inputs[0]["maximum"], pipe::REYNOLDS_MAX);
    assert_eq!(inputs[1]["minimum"], 0.0);
    assert_eq!(inputs[1]["minimum_exclusive"], true);
    assert_eq!(inputs[2]["minimum"], 170.0);
    assert_eq!(inputs[2]["maximum"], 1900.0);
    assert_eq!(inputs[3]["maximum"], pipe::WALL_ROUGHNESS_RMS_MAX_UM);
    assert!(inputs[0]["bound"].as_str().unwrap().contains("31,577"));
    assert!(inputs[1]["bound"].as_str().unwrap().contains("100,090 to 18,928,000 Pa"));
    assert!(inputs[2]["bound"].as_str().unwrap().contains("291.79 to 300.13 K"));
    assert!(!text.contains("291.78999999999996"), "the lowest compared temperature prints as 291.79");
    assert_eq!(described["refusals"].as_array().expect("refusals").len(), 16);
    for refusal in described["refusals"].as_array().unwrap() {
        assert!(!refusal["meaning"].as_str().unwrap().contains("..="), "{refusal}");
    }
    assert_eq!(described["locked"]["kappa"], 0.41);
    assert_eq!(described["locked"]["van_driest_a_plus"], 26.0);

    let example = described["example_solve"].clone();
    let (status, text) = post(&base, "/solve", &example).await;
    assert_eq!(status, 200, "{example} {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "converged");
    let f = body["result"]["darcy_friction_factor"].as_f64().unwrap();
    assert!(f > 0.0115 && f < 0.0119, "{f}");
}
