//! The blast_wave_tnt_air solver through the local connection and `POST
//! /solve`. Stamped from the kit: the whole served contract, attacked from
//! the solver's own description by `conformance_support`, and each refusal's
//! sentence word for word from the solver's source; then the solver's own
//! worked answers: a held-back gauge of the comparison bit for bit with its
//! proof, the refusals at the door by name, and the far field answered.

mod conformance_support;

mod answer_text;

use std::time::Duration;

use conformance_support::{Door, assert_solver_conforms};
use genesis_solvers::blast_wave_tnt_air::{self as model, BlastQuestion, Burst};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const SOURCE: &str = include_str!("../../genesis_solvers/src/blast_wave_tnt_air.rs");

#[test]
fn blast_wave_tnt_air_meets_the_served_contract() {
    assert_solver_conforms("blast_wave_tnt_air");
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "blast_wave_tnt_air" })).expect("described");
    let start = SOURCE.find("pub enum BlastReject {").expect("the enum is in the solver");
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
    // The entry's own door refusal, UnknownBurst, is listed after the solver's.
    assert_eq!(seen, 17);
    assert_eq!(description["refusals"].as_array().unwrap().len(), 18);
}

#[test]
fn the_worked_question_converges() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "blast_wave_tnt_air" })).expect("described");
    for key in ["example_solve", "example_solve_surface_burst", "example_solve_acoustic_far_field"] {
        let answer = door.value("solve", &description[key]).expect("answered");
        assert_eq!(answer["outcome"], "converged", "{key}: {answer}");
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

fn blast(w: f64, r: f64, burst: &str) -> Value {
    json!({ "solver": "blast_wave_tnt_air", "parameters": { "charge_mass_kg": w, "distance_m": r, "burst": burst } })
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
}

fn flip_hex(text: &str) -> String {
    let last = if text.ends_with('0') { '1' } else { '0' };
    format!("{}{last}", &text[..text.len() - 1])
}

/// A held-back gauge of the comparison: the lean propane/oxygen charge
/// (0.7317 g of TNT equivalent by its heat of reaction) on the ground, the
/// gauge at 0.592 m. Answered bit for bit as the solver gives it, with its
/// proof; the scored miss: the peak 42 percent above the measured 0.180 atm,
/// the arrival 9.2 percent before the measured 1.235 ms.
#[tokio::test]
async fn a_held_back_gauge_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let w = 0.000_731_663_208_705_421_6;
    let request = blast(w, 0.592, "surface");
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = model::solve(&BlastQuestion { charge_mass_kg: w, distance_m: 0.592, burst: Burst::Surface }).expect("settles");
    assert_eq!(body["inputs"], request);
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert_eq!(body["iterations"], direct.steps);
    let result = &body["result"];
    assert_number(result, "peak_overpressure_pa", direct.peak_overpressure_pa);
    assert_number(result, "arrival_time_s", direct.arrival_time_s);
    assert_number(result, "positive_phase_duration_s", direct.positive_phase_duration_s);
    assert_number(result, "positive_impulse_pa_s", direct.positive_impulse_pa_s);
    assert_number(result, "friedlander_decay", direct.friedlander_decay);
    assert_number(result, "charge_energy_j", direct.charge_energy_j);
    assert_numbers(result, "history_overpressure_pa", &direct.history_overpressure_pa);
    assert_numbers(result, "friedlander_overpressure_pa", &direct.friedlander_overpressure_pa);
    assert_eq!(result.as_object().unwrap().len(), 38, "nineteen values and their hex");
    assert_number(result, "shock_radius_at_end_m", direct.shock_radius_at_end_m);
    assert_number(result, "shock_rise_time_s", direct.shock_rise_time_s);
    let invariants = &body["invariants"];
    assert_number(invariants, "energy_ledger_residual", direct.energy_ledger_residual);
    assert!(direct.energy_ledger_residual < 1.0e-12);
    let peak_error = direct.peak_overpressure_pa / model::AMBIENT_PRESSURE_PA / 0.18 - 1.0;
    let arrival_error = direct.arrival_time_s * 1.0e3 / 1.235 - 1.0;
    assert!((peak_error - 0.4199).abs() < 1.0e-3, "{peak_error}");
    assert!((arrival_error + 0.0919).abs() < 1.0e-3, "{arrival_error}");

    // The field hash is the count of numbers, the times and the overpressures.
    let mut hasher = Sha256::new();
    hasher.update((2 * model::HISTORY_POINTS as u64).to_le_bytes());
    for value in direct.history_time_after_arrival_s.iter().chain(&direct.history_overpressure_pa) {
        hasher.update(value.to_le_bytes());
    }
    assert_eq!(body["field_sha256"].as_str().unwrap(), hex::encode(hasher.finalize()));

    // The proof holds, and an edit of a number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
    for (section, key) in [("result", "peak_overpressure_pa_hex"), ("result", "positive_impulse_pa_s_hex"), ("invariants", "energy_ledger_residual_hex")] {
        let mut edited = body.clone();
        edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
        let (_, text) = post(&base, "/verify_solution", &edited).await;
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }
    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(serde_json::from_str::<Value>(&again).unwrap(), body);
}

#[tokio::test]
async fn a_question_past_a_limit_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let z_free = model::near_field_scaled_distance(Burst::FreeAir);
    let z_ground = model::near_field_scaled_distance(Burst::Surface);
    let cases = [
        (blast(1.0, 0.999 * z_free, "free_air"), "ChargeProductsNearField"),
        (blast(1000.0, 0.999 * z_ground * 10.0, "surface"), "ChargeProductsNearField"),
        // Clarke et al. (2020), Series A: 78 g of PE4 as 93.6 g of TNT, 177.5 mm
        (blast(0.0936, 0.1775, "free_air"), "ChargeProductsNearField"),
        (blast(0.0, 5.0, "free_air"), "ChargeMassNotPositive"),
        (blast(-1.0, 5.0, "free_air"), "ChargeMassNotPositive"),
        (blast(1.0, 0.0, "free_air"), "DistanceNotPositive"),
        (blast(1.0, 843.46, "free_air"), "DistanceBeyondUniformAtmosphere"),
        (blast(1.0, model::DISTANCE_MAX_M, "free_air"), "DistanceBeyondUniformAtmosphere"),
        (blast(1.0e-18, 6.0e-6, "free_air"), "DistanceBelowContinuum"),
        (blast(3.0e-308, 1.0e-102, "surface"), "DistanceBelowContinuum"),
        (blast(1.1e8, 800.0, "free_air"), "ChargeBeyondUniformAtmosphere"),
        (blast(0.6 * model::CHARGE_MASS_MAX_KG, 800.0, "surface"), "ChargeBeyondUniformAtmosphere"),
        (blast(1.0, 5.0, "underwater"), "UnknownBurst"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name})")), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    let (_, text) = post(&base, "/solve", &blast(1.0, 0.5, "free_air")).await;
    assert_eq!(
        text,
        "Physical Violation: the point is inside the radius at which the air the shock has engulfed weighs as much as the charge, Z = 0.5798 m/kg^(1/3) in free air and 0.7305 on the ground: the charge's detonation products carry the blast there, and a point source of its energy cannot (ChargeProductsNearField)."
    );
}

/// The far field is a regime, not a limit: near the farthest distance
/// served the shock is acoustic and the wave is answered and sealed,
/// weak-shock theory carrying it from Sachs distance 4 with r times the
/// impulse held, the shock within 843.45 m when the phase ends.
#[tokio::test]
async fn the_acoustic_far_field_and_the_near_field_limit_are_answered() {
    let base = spawn_app().await;
    let largest = model::CHARGE_MASS_MAX_KG * (1.0 - 1.0e-4);
    let length = (largest * model::TNT_ENERGY_J_PER_KG / model::AMBIENT_PRESSURE_PA).cbrt();
    for (request, euler_reach) in [
        (blast(1.0, 840.0, "free_air"), 4.0),
        (blast(10.0, 830.0, "surface"), 4.0),
        (blast(1.0, 1.000_001 * model::near_field_scaled_distance(Burst::FreeAir), "free_air"), 0.0),
        (blast(largest, model::LEAST_REACH_SACHS_DISTANCE * length, "free_air"), 0.0),
    ] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{request} {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{request}");
        assert!(body["solution_seal"].is_string());
        let reach = body["result"]["euler_reach_sachs_distance"].as_f64().unwrap();
        if euler_reach > 0.0 {
            assert_eq!(reach, euler_reach);
            assert!(body["invariants"]["weak_shock_area_residual"].as_f64().unwrap() < 1.0e-12);
            assert!(body["result"]["shock_mach_number"].as_f64().unwrap() < 1.001);
        } else {
            assert_eq!(reach, body["result"]["sachs_scaled_distance"].as_f64().unwrap());
        }
        assert!(body["result"]["shock_radius_at_end_m"].as_f64().unwrap() <= model::DISTANCE_MAX_M, "{request}");
    }
}

/// Two limits the solve reaches, each sealed with its proof holding: the
/// shock past the uniform air when the phase at the point ends (1 kg at
/// 843 m, whose shock is about 845.4 m out by then), and a shock not sharp
/// against its phase (a microgram at 840 m).
#[tokio::test]
async fn the_uniform_air_and_the_sharp_shock_are_sealed_limits() {
    let base = spawn_app().await;
    for (request, name) in [(blast(1.0, 843.0, "free_air"), "WaveBeyondUniformAtmosphere"), (blast(1.0e-9, 840.0, "free_air"), "ShockNotSharp"), (blast(1.0e-3, 840.0, "free_air"), "ShockNotSharp")] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{request} {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!((body["outcome"].clone(), body["reject"].clone()), (json!("refused"), json!(name)), "{request}");
        let (_, text) = post(&base, "/verify_solution", &body).await;
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
    }
}

#[tokio::test]
async fn the_description_carries_the_standing() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "blast_wave_tnt_air" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    for part in ["object", "data", "pass_line", "result", "limits"] {
        assert!(!described["validation"][part].as_str().unwrap_or_else(|| panic!("{part}")).is_empty(), "{part}");
    }
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.contains("Record SHA-256 e8f3ac5d85277fd482b81c8a8c2d9eb586e85a6406c3635117a34b82e03f9416."), "{pass_line}");
    assert!(pass_line.contains("the peak within 15 percent, the impulse within 20 percent and the arrival within 5 percent"), "{pass_line}");
    let result = described["validation"]["result"].as_str().unwrap();
    for words in ["0 of 24 within every target (target 20)", "33 to 72 percent above the measured", "43 to 78 percent above", "5.2 to 25.4 percent early"] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let object = described["validation"]["object"].as_str().unwrap();
    assert!(object.contains("not high explosive"), "{object}");
    assert!(object.contains("0.8841 g"), "{object}");
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0]["maximum"], model::CHARGE_MASS_MAX_KG);
    assert_eq!(inputs[1]["maximum"], model::DISTANCE_MAX_M);
    assert_eq!(inputs[1]["maximum_exclusive"], true);
    assert_eq!(inputs[1]["minimum"], model::DISTANCE_MIN_M);
    assert!(inputs[1].get("minimum_exclusive").is_none());
    assert_eq!(described["locked"]["tnt_energy_j_per_kg"], 4.184e6);
    let alpha = described["locked"]["sedov_taylor_alpha"].as_f64().unwrap();
    assert!((alpha - 0.851_072).abs() < 2.0e-6, "{alpha}");
}
