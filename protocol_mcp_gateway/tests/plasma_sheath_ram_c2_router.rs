//! The plasma_sheath_ram_c2 solver through the local connection and through
//! `POST /solve`. Stamped from the kit: the whole served contract, attacked
//! from the solver's own description by `conformance_support`, and each
//! refusal's sentence word for word from the solver's source. Then the
//! solver's own worked answers: the X-band telemetry question at 71 km bit
//! for bit with its proof, the S-band and GPS examples, every limit the
//! request shows at the door with its name, and a shock past the fits,
//! sealed.

mod conformance_support;

mod answer_text;

use std::time::Duration;

use conformance_support::{Door, assert_solver_conforms};
use genesis_solvers::plasma_sheath_ram_c2::{self as sheath, PlasmaSheathRamC2Question};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const SOURCE: &str = include_str!("../../genesis_solvers/src/plasma_sheath_ram_c2.rs");

#[test]
fn plasma_sheath_ram_c2_meets_the_served_contract() {
    assert_solver_conforms("plasma_sheath_ram_c2");
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "plasma_sheath_ram_c2" })).expect("described");
    let start = SOURCE.find("pub enum PlasmaSheathRamC2Reject {").expect("the enum is in the solver");
    let mut doc = String::new();
    let mut seen = 0;
    for line in SOURCE[start..].lines().skip(1) {
        if line == "}" {
            break;
        }
        let text = line.trim_start();
        if let Some(sentence) = text.strip_prefix("/// ") {
            doc.push_str(sentence);
        } else if text.starts_with(|c: char| c.is_ascii_uppercase()) {
            let name: String = text.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            let row = description["refusals"].as_array().unwrap().iter().find(|row| row["name"] == name.as_str()).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(row["meaning"], doc.as_str(), "{name}");
            doc.clear();
            seen += 1;
        }
    }
    assert_eq!(seen, description["refusals"].as_array().unwrap().len());
}

/// The worked question and the two further examples converge, and each
/// answers its blackout question as the record says.
#[test]
fn the_examples_converge_and_answer_their_blackout_questions() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "plasma_sheath_ram_c2" })).expect("described");
    let worked = door.value("solve", &description["example_solve"]).expect("answered");
    assert_eq!(worked["outcome"], "converged", "{worked}");
    // X-band telemetry, 9.21 GHz, at 71 km: cut off at the stagnation point
    // and on the nose, through on the cone.
    assert_eq!(worked["result"]["signal_blacked_out_at_stagnation"], 1.0);
    assert_eq!(worked["result"]["station1_signal_blacked_out"], 1.0);
    assert_eq!(worked["result"]["station2_signal_blacked_out"], 0.0);
    for key in ["example_solve_s_band", "example_solve_glide_gps"] {
        let answer = door.value("solve", &description[key]).expect("answered");
        assert_eq!(answer["outcome"], "converged", "{key}: {answer}");
        // Apollo's S band at 80 km and GPS L1 at 40 km and 6 km/s: cut off everywhere.
        for flag in ["signal_blacked_out_at_stagnation", "station1_signal_blacked_out", "station2_signal_blacked_out"] {
            assert_eq!(answer["result"][flag], 1.0, "{key} {flag}");
        }
    }
}

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

fn sheath_request(altitude: f64, velocity: f64, frequency: f64) -> Value {
    json!({ "solver": "plasma_sheath_ram_c2", "parameters": { "altitude_m": altitude, "velocity_m_s": velocity, "signal_frequency_hz": frequency } })
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn assert_number(section: &Value, key: &str, expected: f64) {
    assert_eq!(section[format!("{key}_hex")], hex_of(expected), "{key}");
    assert_eq!(section[key].as_f64().unwrap_or_else(|| panic!("{key}")).to_bits(), expected.to_bits(), "{key}");
}

fn flip_hex(text: &str) -> String {
    let last = if text.ends_with('0') { '1' } else { '0' };
    format!("{}{last}", &text[..text.len() - 1])
}

/// The worked question, 71 km at 7.65 km/s and 9.21 GHz, answered bit for
/// bit as the solver gives it, with its proof.
#[tokio::test]
async fn the_worked_question_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let request = sheath_request(71_000.0, 7650.0, 9.21e9);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = sheath::solve(&PlasmaSheathRamC2Question { altitude_m: 71_000.0, velocity_m_s: 7650.0, signal_frequency_hz: 9.21e9 }).expect("settles");
    assert_eq!(body["inputs"], request);
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["iterations"], u64::from(direct.steps) + u64::from(direct.equilibrium_iterations));
    let result = &body["result"];
    assert_number(result, "shock_temperature_k", direct.shock_temperature_k);
    assert_number(result, "stagnation_electron_density_m3", direct.stagnation_electron_density_m3);
    assert_number(result, "station1_electron_density_m3", direct.stations[0].electron_density_m3);
    assert_number(result, "station2_electron_density_m3", direct.stations[1].electron_density_m3);
    assert_number(result, "station2_frozen_electron_density_m3", direct.stations[1].frozen_electron_density_m3);
    assert_number(result, "signal_critical_density_m3", direct.signal_critical_density_m3);
    assert_eq!(result.as_object().unwrap().len(), 2 * 44, "forty-four values and their hex");
    // The record's numbers: 8.916e18 and 6.631e17 per cubic metre at the stations.
    assert!((direct.stations[0].electron_density_m3 / 8.9162e18 - 1.0).abs() < 1.0e-4);
    assert!((direct.stations[1].electron_density_m3 / 6.6311e17 - 1.0).abs() < 1.0e-4);
    let invariants = &body["invariants"];
    assert_number(invariants, "shock_energy_residual", direct.shock_energy_residual);
    assert_number(invariants, "streamline_energy_residual", direct.streamline_energy_residual);
    assert_number(invariants, "streamline_entropy_fall", direct.streamline_entropy_fall);
    assert!(direct.streamline_energy_residual <= sheath::ENERGY_LEDGER_TOLERANCE);
    assert!(direct.streamline_charge_residual <= sheath::CHARGE_TOLERANCE);

    // The field hash is the count of numbers, then three for each point of the streamline.
    let mut hasher = Sha256::new();
    hasher.update((direct.streamline.len() as u64).to_le_bytes());
    for value in &direct.streamline {
        hasher.update(value.to_le_bytes());
    }
    let own = hex::encode(hasher.finalize());
    let served = body["field_sha256"].as_str().expect("field hash");
    assert_eq!(served, own);

    // The proof holds, and an edit of a number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
    for (section, key) in [("result", "station1_electron_density_m3_hex"), ("result", "station2_signal_blacked_out_hex"), ("invariants", "streamline_energy_residual_hex")] {
        let mut edited = body.clone();
        edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
        let (_, text) = post(&base, "/verify_solution", &edited).await;
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }
    let mut field = body.clone();
    field["field_sha256"] = json!(flip_hex(served));
    let (_, text) = post(&base, "/verify_solution", &field).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(serde_json::to_string(&body).unwrap(), serde_json::to_string(&serde_json::from_str::<Value>(&again).unwrap()).unwrap());
}

#[tokio::test]
async fn a_limit_the_request_shows_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let cases = [
        (sheath_request(-1.0, 7650.0, 1.0e9), "AltitudeBelowSeaLevel"),
        (sheath_request(-1.0e-9, 7650.0, 1.0e9), "AltitudeBelowSeaLevel"),
        (sheath_request(80_637.00000000001, 7650.0, 1.0e9), "RarefiedFlow"),
        (sheath_request(84_000.0, 7650.0, 3.348e9), "RarefiedFlow"),
        (sheath_request(71_000.0, 290.0, 1.0e9), "NoBowShock"),
        (sheath_request(0.0, 340.0, 1.0e9), "NoBowShock"),
        (sheath_request(71_000.0, 7650.0, 999_999.9999999999), "SignalFrequencyBelowResolution"),
        (sheath_request(71_000.0, 7650.0, 1.0e151), "SignalFrequencyBeyondArithmetic"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name}).")), "{name}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // A key left out, and a constant of the vehicle, are refused.
    let (status, _) = post(&base, "/solve", &json!({ "solver": "plasma_sheath_ram_c2", "parameters": { "altitude_m": 71_000.0, "velocity_m_s": 7650.0 } })).await;
    assert_eq!(status, 422);
    let (status, _) = post(&base, "/solve", &json!({ "solver": "plasma_sheath_ram_c2", "parameters": { "altitude_m": 71_000.0, "velocity_m_s": 7650.0, "signal_frequency_hz": 1.0e9, "nose_radius_m": 0.2 } })).await;
    assert_eq!(status, 422);
}

/// At sea level and 14 km/s the gas behind the shock is past 20,000 K, the
/// top of the NASA Glenn fits: answered with a sealed refusal by name, and
/// its proof holds.
#[tokio::test]
async fn a_shock_past_the_fits_is_named_and_sealed() {
    let base = spawn_app().await;
    let request = sheath_request(0.0, 14_000.0, 1.0e9);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "TemperatureAboveFits");
    assert!(body["reject_detail"].as_str().unwrap().contains("above 20 000 K, the top of the NASA Glenn thermodynamic fits"), "{body}");
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
}
