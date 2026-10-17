//! `ln2_line_dlr_fttf` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door and sealed, its proof
//! and its breaking, its description and its worked examples.

mod answer_text;

use std::time::Duration;

use genesis_solvers::ln2_line_dlr_fttf::{self as ln2, LineQuestion};
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

fn line_request(q: &LineQuestion, histories: bool) -> Value {
    json!({ "solver": "ln2_line_dlr_fttf", "parameters": {
        "temperature_k": q.temperature_k,
        "mass_flow_kg_s": q.mass_flow_kg_s,
        "s1_pressure_pa": q.s1_pressure_pa,
        "return_pressure_pa": q.return_pressure_pa,
        "stroke_time_s": q.stroke_time_s,
        "duration_s": q.duration_s,
        "return_histories": histories
    } })
}

/// The conditions of the traced test 20200803_11.
fn question(duration: f64) -> LineQuestion {
    LineQuestion {
        temperature_k: 83.7,
        mass_flow_kg_s: 1.9739,
        s1_pressure_pa: 1144325.0,
        return_pressure_pa: 1.7e5,
        stroke_time_s: ln2::STROKE_REFERENCE_S,
        duration_s: duration,
    }
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn assert_number(section: &Value, key: &str, expected: f64) {
    assert_eq!(section[format!("{key}_hex")], hex_of(expected), "{key}");
    assert_eq!(section[key].as_f64().unwrap_or_else(|| panic!("{key}")).to_bits(), expected.to_bits(), "{key}");
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

/// A cavitating line with its histories, bit for bit as the solver gives
/// it, with its field hash and its proof, and the proof broken by a changed
/// number.
#[tokio::test]
async fn a_transient_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let q = question(0.15);
    let request = line_request(&q, true);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = ln2::solve(&q).expect("the line runs");
    assert_eq!(body["inputs"], request);
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["iterations"], direct.steps);
    let result = &body["result"];
    assert_number(result, "wave_speed_m_s", direct.state.wave_speed_m_s);
    assert_number(result, "joukowsky_rise_pa", direct.state.joukowsky_rise_pa);
    assert_number(result, "s1_max_pa", direct.s1_max_pa);
    assert_number(result, "s3_max_time_s", direct.s3_max_time_s);
    assert_number(result, "line_pressure_min_pa", direct.line_pressure_min_pa);
    assert_number(result, "cavity_loss_j", direct.cavity_loss_j);
    assert_numbers(result, "first_vapour_time_s", &direct.first_vapour_time_s);
    assert_numbers(result, "s1_history_pa", &direct.s1_history_pa);
    assert_numbers(result, "liquid_energy_history_j", &direct.liquid_energy_history_j);
    assert_eq!(result.as_object().unwrap().len(), 2 * 48, "48 values and their hex");
    // The line cavitated, at the vapour pressure of the asked temperature.
    assert!(!direct.first_vapour_time_s.is_empty());
    let (lowest, vapour) = (direct.line_pressure_min_pa, direct.state.vapour_pressure_pa);
    assert!((lowest / vapour - 1.0).abs() < 1e-14, "{lowest} {vapour}");
    let invariants = &body["invariants"];
    assert_number(invariants, "largest_energy_ledger_residual", direct.largest_energy_ledger_residual);
    assert_number(invariants, "largest_volume_ledger_residual", direct.largest_volume_ledger_residual);
    assert!(direct.largest_energy_ledger_residual < 1e-12 && direct.largest_volume_ledger_residual < 1e-12);

    let mut hasher = Sha256::new();
    hasher.update((direct.field.len() as u64).to_le_bytes());
    for value in &direct.field {
        hasher.update(value.to_le_bytes());
    }
    assert_eq!(body["field_sha256"].as_str().expect("field hash"), hex::encode(hasher.finalize()));

    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
    for (section, key) in [("result", "s1_max_pa_hex"), ("result", "cavity_loss_j_hex"), ("invariants", "largest_volume_ledger_residual_hex")] {
        let mut edited = body.clone();
        edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
        let (_, text) = post(&base, "/verify_solution", &edited).await;
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }
    let mut edited = body.clone();
    edited["result"]["s2_history_pa_hex"][40] = json!(flip_hex(body["result"]["s2_history_pa_hex"][40].as_str().unwrap()));
    let (_, text) = post(&base, "/verify_solution", &edited).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);
    // A changed question does not carry the old seal.
    let mut edited = body.clone();
    edited["inputs"]["parameters"]["temperature_k"] = json!(83.8);
    let (_, text) = post(&base, "/verify_solution", &edited).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(text_of(&body), text_of(&serde_json::from_str::<Value>(&again).unwrap()));

    // Without the histories the rest is the same bits and the histories are absent.
    let (_, text) = post(&base, "/solve", &line_request(&q, false)).await;
    let short: Value = serde_json::from_str(&text).unwrap();
    assert!(short["result"].get("s1_history_pa").is_none());
    assert_eq!(short["result"]["s1_max_pa_hex"], body["result"]["s1_max_pa_hex"]);
    assert_eq!(short["field_sha256"], body["field_sha256"]);
}

#[tokio::test]
async fn a_question_past_a_limit_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let base_q = question(0.05);
    let with = |change: fn(&mut LineQuestion)| {
        let mut q = base_q;
        change(&mut q);
        line_request(&q, false)
    };
    let cases = [
        (with(|q| q.temperature_k = 0.0), "TemperatureNotPositive"),
        (with(|q| q.temperature_k = 60.0), "LiquidFreezesBelowWallYield"),
        (with(|q| q.temperature_k = 68.5), "LiquidFreezesBelowWallYield"),
        (with(|q| q.temperature_k = 110.0), "VapourNotLightAgainstLiquid"),
        (with(|q| q.s1_pressure_pa = 2.0e5), "S1PressureNotAboveVapour"),
        (with(|q| q.s1_pressure_pa = 25.3e6), "SupplyAboveWallYield"),
        (with(|q| q.mass_flow_kg_s = -0.1), "MassFlowNegative"),
        (with(|q| q.return_pressure_pa = 0.0), "ReturnBelowTriplePoint"),
        (with(|q| q.return_pressure_pa = 12_522.0), "ReturnBelowTriplePoint"),
        (with(|q| q.return_pressure_pa = 11.4e5), "SupplyCannotDriveFlow"),
        (with(|q| { q.s1_pressure_pa = 2.3e5; q.mass_flow_kg_s = 6.0 }), "SteadyBelowVapour"),
        (with(|q| q.mass_flow_kg_s = 18.0), "InitialFlowMachAboveAcoustic"),
        (with(|q| q.duration_s = 10.5), "DurationOutOfRange"),
        (with(|q| q.stroke_time_s = 0.06), "StrokeTimeOutOfRange"),
        (with(|q| q.stroke_time_s = 0.0005), "StrokeFasterThanGrid"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name})")), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // The caller asks the question and never sets the specimen.
    for key in ["roughness_m", "valve_seat", "line_length_m", "sections"] {
        let mut extra = with(|_| {});
        extra["parameters"][key] = json!(1.0);
        let (status, text) = post(&base, "/solve", &extra).await;
        assert_eq!(status, 422);
        assert_eq!(
            text,
            format!("Physical Violation: '{key}' is not an input of this solver. The line, its wall, its stations, its friction, the valve's stroke and seat, the liquid's table, the grid of 93 sections and the tolerances stay inside the solver.")
        );
    }
    let mut missing = with(|_| {});
    missing["parameters"].as_object_mut().unwrap().remove("temperature_k");
    let (status, _) = post(&base, "/solve", &missing).await;
    assert_eq!(status, 422);
}

/// A surge that would yield the tube is answered as a sealed refusal at the
/// time and place it reached the yield.
#[tokio::test]
async fn a_rise_to_the_tubes_yield_is_a_sealed_refusal() {
    let base = spawn_app().await;
    let q = LineQuestion { mass_flow_kg_s: 2.6, s1_pressure_pa: 225.0e5, duration_s: 0.03, ..question(0.03) };
    let (status, text) = post(&base, "/solve", &line_request(&q, false)).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "WallYield");
    let direct = ln2::solve(&q).unwrap_err();
    assert_eq!(body["reject_detail"], format!("Solver Refusal: ln2_line_dlr_fttf did not return a transient (WallYield). {direct}."));
    assert!(body["result"].is_null() && body["invariants"].is_null() && body["field_sha256"].is_null());
    let (_, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
}

#[tokio::test]
async fn the_description_carries_the_comparison_and_its_examples_run_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let row = listed["solvers"].as_array().unwrap().iter().find(|row| row["solver"] == "ln2_line_dlr_fttf").expect("listed").clone();
    assert_ne!(row["against_measurement"], "not held to a measurement");
    let summary = row["summary"].as_str().unwrap();
    assert!(summary.contains("DLR's Fluid Transient Test Facility"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "ln2_line_dlr_fttf" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.contains(ln2::PROTOCOL_SHA256), "{pass_line}");
    assert_eq!(described["inputs"].as_array().unwrap().len(), 7);
    assert_eq!(described["inputs"][0]["minimum"], ln2::COLDEST_K);
    assert_eq!(described["locked"]["sections"], 93);
    assert_eq!(described["locked"]["roughness_m"], ln2::ROUGHNESS_M);
    for key in ["example_solve", "example_solve_dlr_20200805_10", "example_solve_low_pressure_cavitation"] {
        let example = described[key].clone();
        let (status, text) = post(&base, "/solve", &example).await;
        assert_eq!(status, 200, "{key}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{key}");
    }
}
