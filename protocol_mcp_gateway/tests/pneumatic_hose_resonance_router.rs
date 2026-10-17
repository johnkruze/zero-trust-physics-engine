//! `pneumatic_hose_resonance` through `POST /solve`, against a direct call
//! to `genesis_solvers`, with its refusals at the door, its description and
//! its worked examples.

mod answer_text;

use std::time::Duration;

use genesis_solvers::pneumatic_hose_resonance::{self as hose, HoseQuestion};
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

fn request(pressure: f64, frequencies: &[f64]) -> Value {
    json!({ "solver": "pneumatic_hose_resonance", "parameters": { "mean_pressure_pa": pressure, "frequencies_hz": frequencies } })
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn assert_numbers(section: &Value, key: &str, expected: &[f64]) {
    let hex: Vec<String> = expected.iter().map(|value| hex_of(*value)).collect();
    assert_eq!(section[format!("{key}_hex")], json!(hex), "{key}");
}

fn flip_hex(text: &str) -> String {
    let last = if text.ends_with('0') { '1' } else { '0' };
    format!("{}{last}", &text[..text.len() - 1])
}

#[tokio::test]
async fn a_response_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let frequencies = [1.0, 24.0, 92.0, 1000.0];
    let req = request(6.0e5, &frequencies);
    let (status, text) = post(&base, "/solve", &req).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = hose::solve(&HoseQuestion { mean_pressure_pa: 6.0e5, frequencies_hz: frequencies.to_vec() }).unwrap();
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["iterations"], direct.frequencies_computed);
    let result = &body["result"];
    assert_numbers(result, "natural_frequencies_hz", &direct.natural_frequencies_hz);
    assert_numbers(result, "damping_ratios", &direct.damping_ratios);
    assert_numbers(result, "peak_frequencies_hz", &direct.peak_frequencies_hz);
    assert_numbers(result, "gains", &direct.gains);
    assert_numbers(result, "phases_rad", &direct.phases_rad);
    assert_numbers(result, "line_dissipation_w", &direct.line_dissipation_w);
    assert_eq!(result["mode_numbers"], json!([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]));
    // The first mode near 24 Hz, its peak gain near 10.
    let first = direct.natural_frequencies_hz[0];
    assert!(first > 23.0 && first < 25.0, "{first}");
    assert!(direct.peak_gains[0] > 9.0 && direct.peak_gains[0] < 11.0);
    assert!(direct.largest_energy_ledger_residual < 1e-12 && direct.largest_volume_ledger_residual < 1e-11);
    let mut hasher = Sha256::new();
    hasher.update((direct.field.len() as u64).to_le_bytes());
    for value in &direct.field {
        hasher.update(value.to_le_bytes());
    }
    assert_eq!(body["field_sha256"].as_str().unwrap(), hex::encode(hasher.finalize()));
    let (_, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
    let mut edited = body.clone();
    edited["result"]["damping_ratios_hex"][0] = json!(flip_hex(body["result"]["damping_ratios_hex"][0].as_str().unwrap()));
    let (_, text) = post(&base, "/verify_solution", &edited).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);
    let (_, again) = post(&base, "/solve", &req).await;
    assert_eq!(serde_json::to_string(&body).unwrap(), serde_json::to_string(&serde_json::from_str::<Value>(&again).unwrap()).unwrap());
}

#[tokio::test]
async fn a_question_past_a_limit_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    for (req, name) in [
        (request(1.2e6, &[]), "PressureAboveTubeRating"),
        (request(150.0, &[]), "RarefiedFlow"),
        (request(1e5, &[0.0]), "FrequencyNotPositive"),
        (request(1e5, &[60_000.0]), "AboveFirstCrossMode"),
        (request(1e5, &[50_290.0]), "AboveFirstCrossMode"),
        (request(1e5, &[hose::FREQUENCY_MAX_HZ]), "AboveFirstCrossMode"),
        (request(1e5, &[1.0; 65]), "TooManyFrequencies"),
        (request(1.0e4, &[20_000.0]), "StokesLayerSlip"),
        (request(hose::PRESSURE_MIN_PA, &[180.0]), "StokesLayerSlip"),
    ] {
        let (status, text) = post(&base, "/solve", &req).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains(&format!("({name})")), "{name}: {text}");
        assert!(!text.contains("solution_seal"));
    }
    let (_, text) = post(&base, "/solve", &request(1.0e4, &[20_000.0])).await;
    assert_eq!(text, format!("Physical Violation: frequency 20000 Hz is above {} Hz, where at this mean pressure the oscillating viscous layer at the wall is a hundred mean free paths thick and slip flow begins in it (StokesLayerSlip).", hose::stokes_slip_hz(1.0e4)));
    // The top of the range is answered: the number below the cut-on, which
    // is exclusive, and the red team's 50,289.2 Hz and 170.5 Pa, refused
    // before inside the limits; and the smallest normal frequency.
    let rating = hose::ATMOSPHERE_PA + hose::TUBE_RATING_GAUGE_PA;
    for req in [request(rating, &[hose::FREQUENCY_MAX_HZ.next_down(), 50_289.2]), request(170.5, &[100.0]), request(6.0e5, &[f64::MIN_POSITIVE]), request(hose::PRESSURE_MIN_PA, &[hose::stokes_slip_hz(hose::PRESSURE_MIN_PA)])] {
        let (status, text) = post(&base, "/solve", &req).await;
        assert_eq!(status, 200, "{req}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{req}: {text}");
    }
}

/// The frequencies' bounds are exclusive at both ends, and the description
/// says so beside each, in the form the served contract reads.
#[tokio::test]
async fn the_frequencies_bounds_are_published_exclusive() {
    let base = spawn_app().await;
    let (_, text) = post(&base, "/describe_solver", &json!({ "solver": "pneumatic_hose_resonance" })).await;
    let described: Value = serde_json::from_str(&text).unwrap();
    let frequencies = &described["inputs"][1];
    assert_eq!(frequencies["name"], "frequencies_hz");
    assert_eq!((frequencies["minimum"].clone(), frequencies["minimum_exclusive"].clone()), (json!(0.0), json!(true)));
    assert_eq!(frequencies["maximum"].as_f64().unwrap().to_bits(), hose::first_cross_mode_hz().to_bits());
    assert_eq!(frequencies["maximum_exclusive"], json!(true));
    assert_eq!(described["inputs"][0]["minimum"].as_f64().unwrap().to_bits(), hose::rarefied_below_pa().to_bits());
    assert!(described["inputs"][0].get("minimum_exclusive").is_none());
}

#[tokio::test]
async fn the_description_carries_the_standing_and_its_examples_run_unchanged() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "pneumatic_hose_resonance" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert!(described["summary"].as_str().unwrap().contains("Nichols and Brown"));
    assert_eq!(described["validation"]["result"], "The company's own specimen: its constants are the company's, stated in its description, and every answer holds the conservation ledgers stated under invariants to their tolerances; what its method was held to, and how it did there, is under reference.");
    assert!(described["reference"]["url"].as_str().unwrap().contains("10.1002/andp.18682100602"));
    for key in ["example_solve", "example_solve_low_pressure"] {
        let (status, text) = post(&base, "/solve", &described[key]).await;
        assert_eq!(status, 200, "{key}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{key}");
    }
}
