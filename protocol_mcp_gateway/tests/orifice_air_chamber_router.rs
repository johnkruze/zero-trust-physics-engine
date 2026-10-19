//! `orifice_air_chamber` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door, its description and
//! its worked examples.

mod answer_text;

use std::time::Duration;

use genesis_solvers::orifice_air_chamber::{self as chamber, ChamberQuestion};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha384};
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
        match client.post(&url).header(reqwest::header::CONTENT_TYPE, "application/json").body(text.clone()).send().await {
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

fn request(reservoir: f64, start: f64, duration: f64, histories: bool) -> Value {
    json!({ "solver": "orifice_air_chamber", "parameters": {
        "reservoir_pressure_pa": reservoir, "chamber_start_pressure_pa": start, "duration_s": duration, "return_histories": histories
    } })
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn assert_number(section: &Value, key: &str, expected: f64) {
    assert_eq!(section[format!("{key}_hex")], hex_of(expected), "{key}");
    assert_eq!(section[key].as_f64().unwrap().to_bits(), expected.to_bits(), "{key}");
}

fn flip_hex(text: &str) -> String {
    let last = if text.ends_with('0') { '1' } else { '0' };
    format!("{}{last}", &text[..text.len() - 1])
}

#[tokio::test]
async fn a_charge_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let q = ChamberQuestion { reservoir_pressure_pa: 7.0e5, chamber_start_pressure_pa: 101_325.0, duration_s: 1.0 };
    let req = request(q.reservoir_pressure_pa, q.chamber_start_pressure_pa, q.duration_s, true);
    let (status, text) = post(&base, "/solve", &req).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = chamber::solve(&q).unwrap();
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["iterations"], direct.steps);
    let result = &body["result"];
    assert_number(result, "start_mass_flow_kg_s", direct.start_mass_flow_kg_s);
    assert_eq!(result["start_choked"], 1.0);
    assert_number(result, "choked_until_s", direct.choked_until_s);
    assert_number(result, "final_pressure_pa", direct.final_pressure_pa);
    assert_number(result, "temperature_max_k", direct.temperature_max_k);
    assert_number(result, "heat_to_wall_j", direct.heat_to_wall_j);
    assert_number(result, "entropy_generated_j_k", direct.entropy_generated_j_k);
    assert_eq!(result["pressure_history_pa"].as_array().unwrap().len(), direct.pressure_history_pa.len());
    assert_eq!(result.as_object().unwrap().len(), 2 * 17);
    assert!(direct.mass_ledger_residual < 1e-12 && direct.energy_ledger_residual < 1e-12);
    assert_number(&body["invariants"], "energy_ledger_residual", direct.energy_ledger_residual);
    let mut hasher = Sha384::new();
    hasher.update((direct.field.len() as u64).to_le_bytes());
    for value in &direct.field {
        hasher.update(value.to_le_bytes());
    }
    assert_eq!(body["field_sha384"].as_str().unwrap(), hex::encode(hasher.finalize()));
    let (_, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
    let mut edited = body.clone();
    edited["result"]["final_pressure_pa_hex"] = json!(flip_hex(body["result"]["final_pressure_pa_hex"].as_str().unwrap()));
    let (_, text) = post(&base, "/verify_solution", &edited).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);
    let (_, again) = post(&base, "/solve", &req).await;
    assert_eq!(serde_json::to_string(&body).unwrap(), serde_json::to_string(&serde_json::from_str::<Value>(&again).unwrap()).unwrap());
}

#[tokio::test]
async fn a_question_past_a_limit_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    for (req, name) in [
        (request(1.2e6, 1e5, 1.0, false), "PressureAboveValveRating"),
        (request(7e5, 1.2e6, 1.0, false), "PressureAboveValveRating"),
        (request(100.0, 1e5, 1.0, false), "RarefiedFlow"),
        (request(7e5, 268.0, 1.0, false), "RarefiedFlow"),
        (request(7e5, 1e5, 0.0, false), "DurationOutOfRange"),
        (request(7e5, 1e5, 21.0, false), "DurationOutOfRange"),
    ] {
        let (status, text) = post(&base, "/solve", &req).await;
        assert_eq!(status, 422, "{req} {text}");
        assert!(text.contains(&format!("({name})")), "{req}: {text}");
        assert!(!text.contains("solution_seal"));
    }
    let (_, text) = post(&base, "/solve", &request(100.0, 1e5, 1.0, false)).await;
    assert_eq!(text, "Physical Violation: pressure 100 Pa is below 268.98631344978776 Pa, where the mean free path is a hundredth of the valve's throat and slip flow begins (RarefiedFlow).");
}

/// The published minimum is the computed limit to the 64-bit number: it is
/// answered, as are 268.987 Pa and 268.99999 Pa, which the red team found
/// refused inside it, and the number below it is refused by its name.
#[tokio::test]
async fn the_published_floor_is_answered_and_the_number_below_it_refused_by_name() {
    let base = spawn_app().await;
    let (_, text) = post(&base, "/describe_solver", &json!({ "solver": "orifice_air_chamber" })).await;
    let described: Value = serde_json::from_str(&text).unwrap();
    let floor = described["inputs"][0]["minimum"].as_f64().unwrap();
    assert_eq!(floor.to_bits(), chamber::rarefied_below_pa().to_bits());
    for reservoir in [floor, 268.987, 268.999_99] {
        let (status, text) = post(&base, "/solve", &request(reservoir, 101_325.0, 0.01, false)).await;
        assert_eq!(status, 200, "{reservoir}: {text}");
    }
    let (status, text) = post(&base, "/solve", &request(floor.next_down(), 101_325.0, 0.01, false)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(RarefiedFlow)."), "{text}");
}

/// A run of 20 s that the red team found refused on rounding alone, the
/// energy ledger past 1e-10 at 16.4 s, is answered, its ledgers near 1e-16,
/// at the reservoir's pressure, with its last sample at 20 s.
#[tokio::test]
async fn an_ordinary_long_run_is_answered_and_ends_at_the_asked_time() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/solve", &request(1000.0, 269.0, 20.0, true)).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "converged", "{text}");
    assert!(body["invariants"]["energy_ledger_residual"].as_f64().unwrap() < 1e-14);
    assert_eq!(body["result"]["final_pressure_pa"].as_f64().unwrap(), 1000.0);
    assert_eq!(body["result"]["sample_times_s"].as_array().unwrap().last().unwrap().as_f64(), Some(20.0));
    let (status, text) = post(&base, "/solve", &request(1.1e6, 269.0, 1.0e-5, true)).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["result"]["sample_times_s"], json!([0.0, 1.0e-5]));
}

#[tokio::test]
async fn the_description_carries_the_standing_and_its_examples_run_unchanged() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "orifice_air_chamber" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert!(described["summary"].as_str().unwrap().contains("ISO 6358-1:2013"));
    assert_eq!(described["validation"]["result"], "Company specimen: its constants are the company's, stated in its description, and every answer holds the conservation ledgers stated under invariants to their tolerances; what its method is held to, and within what, is under reference.");
    assert!(described["reference"]["covers"].as_str().unwrap().contains("C = 0.90e-8 m^3/(s Pa) and b = 0.56 at 200 kPa"));
    assert_eq!(described["locked"]["critical_pressure_ratio"], 0.3);
    for key in ["example_solve", "example_solve_vent"] {
        let (status, text) = post(&base, "/solve", &described[key]).await;
        assert_eq!(status, 200, "{key}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{key}");
    }
    let (_, text) = post(&base, "/solve", &described["example_solve_vent"]).await;
    let vent: Value = serde_json::from_str(&text).unwrap();
    assert!(vent["result"]["start_mass_flow_kg_s"].as_f64().unwrap() < 0.0);
    assert!(vent["result"]["temperature_min_k"].as_f64().unwrap() < 260.0);
}
