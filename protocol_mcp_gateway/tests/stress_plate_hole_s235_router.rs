//! The stress_plate_hole_s235 solver through the local connection and
//! through `POST /solve`. Stamped from the kit: the whole served contract,
//! attacked from the solver's own description by `conformance_support`, and
//! each refusal's sentence word for word from the solver's source. Then the
//! solver's own worked answers: the laboratory's load bit for bit with its
//! proof, every limit the request shows at the door with its name, and the
//! strain limit past the net section's collapse, sealed.

mod conformance_support;

mod answer_text;

use std::time::Duration;

use conformance_support::{Door, assert_solver_conforms};
use genesis_solvers::stress_plate_hole_s235::{self as plate, PlateQuestion};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const SOURCE: &str = include_str!("../../genesis_solvers/src/stress_plate_hole_s235.rs");

#[test]
fn stress_plate_hole_s235_meets_the_served_contract() {
    assert_solver_conforms("stress_plate_hole_s235");
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "stress_plate_hole_s235" })).expect("described");
    let start = SOURCE.find("pub enum PlateReject {").expect("the enum is in the solver");
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

#[test]
fn the_worked_questions_converge() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "stress_plate_hole_s235" })).expect("described");
    let answer = door.value("solve", &description["example_solve"]).expect("answered");
    assert_eq!(answer["outcome"], "converged", "{answer}");
    assert!(answer["result"]["plastic_zone_area_mm2"].as_f64().unwrap() > 5.0);
    let elastic = door.value("solve", &description["example_solve_elastic"]).expect("answered");
    assert_eq!(elastic["outcome"], "converged", "{elastic}");
    assert_eq!(elastic["iterations"], 0);
    assert_eq!(elastic["result"]["plastic_zone_area_mm2"], 0.0);
    assert_eq!(elastic["result"]["point_stress_xx_mpa"], json!([]));
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

fn plate_request(traction: f64, xs: &[f64], ys: &[f64]) -> Value {
    json!({ "solver": "stress_plate_hole_s235", "parameters": { "traction_mpa": traction, "points_x_mm": xs, "points_y_mm": ys } })
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

/// The laboratory's load, 106.26 MPa, answered bit for bit as the solver
/// gives it, with its proof; the hole's edge has yielded.
#[tokio::test]
async fn the_laboratory_s_load_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let (xs, ys) = ([0.0, 0.0, 4.5, -20.0], [4.0, 10.025, 0.0, 5.0]);
    let request = plate_request(plate::MEASURED_TRACTION_MPA, &xs, &ys);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = plate::solve(&PlateQuestion { traction_mpa: plate::MEASURED_TRACTION_MPA, points_x_mm: xs.to_vec(), points_y_mm: ys.to_vec() }).expect("settles");
    assert_eq!(body["inputs"], request);
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["iterations"], direct.newton_iterations);
    let result = &body["result"];
    assert_number(result, "hole_edge_stress_mpa", direct.hole_edge_stress_mpa);
    assert_number(result, "elastic_hole_edge_stress_mpa", direct.elastic_hole_edge_stress_mpa);
    assert_number(result, "stress_concentration_net", direct.stress_concentration_net);
    assert_number(result, "plastic_zone_area_mm2", direct.plastic_zone_area_mm2);
    assert_number(result, "parallel_section_elongation_mm", direct.parallel_section_elongation_mm);
    assert_numbers(result, "point_stress_xx_mpa", &direct.points.iter().map(|p| p.stress_xx_mpa).collect::<Vec<_>>());
    assert_numbers(result, "point_displacement_y_mm", &direct.points.iter().map(|p| p.displacement_y_mm).collect::<Vec<_>>());
    assert_eq!(result.as_object().unwrap().len(), 2 * 26, "twenty-six values and their hex");
    // Yielded at the edge: its stress is the yield strength, the elastic one 397.6 MPa.
    assert!((direct.hole_edge_stress_mpa / plate::YIELD_STRENGTH_MPA - 1.0).abs() < 5e-3);
    assert!((direct.elastic_hole_edge_stress_mpa - 397.6).abs() < 0.1);
    let invariants = &body["invariants"];
    assert_number(invariants, "equilibrium_residual", direct.equilibrium_residual);
    assert_number(invariants, "energy_residual", direct.energy_residual);
    assert_number(invariants, "section_force_residual", direct.section_force_residual);
    assert!(direct.equilibrium_residual <= plate::IMBALANCE_TOLERANCE);
    assert!(direct.energy_residual <= plate::ENERGY_TOLERANCE);
    assert!(direct.section_force_residual <= plate::SECTION_FORCE_TOLERANCE);

    // The field hash is the count of numbers, then seven numbers for each node.
    let mut hasher = Sha256::new();
    hasher.update((direct.nodes.len() as u64).to_le_bytes());
    for value in &direct.nodes {
        hasher.update(value.to_le_bytes());
    }
    let own = hex::encode(hasher.finalize());
    let served = body["field_sha256"].as_str().expect("field hash");
    assert_eq!(served, own);

    // The proof holds, and an edit of a number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
    for (section, key) in [("result", "hole_edge_stress_mpa_hex"), ("result", "plastic_zone_area_mm2_hex"), ("invariants", "energy_residual_hex")] {
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
    let many = vec![20.0; plate::MAX_POINTS + 1];
    let zeros = vec![0.0; plate::MAX_POINTS + 1];
    let cases = [
        (plate_request(0.0, &[], &[]), "TractionNotTensile"),
        (plate_request(-50.0, &[], &[]), "TractionNotTensile"),
        (plate_request(235.00000000000003, &[], &[]), "TractionAboveYield"),
        (plate_request(500.0, &[], &[]), "TractionAboveYield"),
        (plate_request(100.0, &[0.0, 1.0], &[5.0]), "PointCountsDiffer"),
        (plate_request(100.0, &many, &zeros), "TooManyPoints"),
        (plate_request(100.0, &[45.00000000000001], &[0.0]), "PointOutsidePlate"),
        (plate_request(100.0, &[0.0], &[-10.025000000000002]), "PointOutsidePlate"),
        (plate_request(100.0, &[0.0], &[3.9999999999999996]), "PointInsideHole"),
        (plate_request(100.0, &[2.0], &[-2.0]), "PointInsideHole"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name}).")), "{name}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // A key left out, and a constant of the specimen, are refused.
    let (status, _) = post(&base, "/solve", &json!({ "solver": "stress_plate_hole_s235", "parameters": { "traction_mpa": 100.0, "points_x_mm": [] } })).await;
    assert_eq!(status, 422);
    let (status, _) = post(&base, "/solve", &json!({ "solver": "stress_plate_hole_s235", "parameters": { "traction_mpa": 100.0, "points_x_mm": [], "points_y_mm": [], "yield_strength_mpa": 300.0 } })).await;
    assert_eq!(status, 422);
}

/// Past the collapse load of the net section the strain at the hole runs
/// away: the question is answered with a sealed refusal by name, and its
/// proof holds.
#[tokio::test]
async fn past_the_net_section_s_collapse_the_strain_limit_is_named_and_sealed() {
    let base = spawn_app().await;
    let request = plate_request(150.0, &[], &[]);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "StrainBeyondSmallStrain");
    assert!(body["reject_detail"].as_str().unwrap().contains("past 0.01, where the linear measure of strain no longer holds"), "{body}");
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
}

/// The ledger closes from the three numbers served, at the laboratory's load
/// and while elastic (the work then the strain energy itself); the smallest
/// tractions are answered, sealed, with their ledger closed; and a refusal
/// prints a number short.
#[tokio::test]
async fn the_served_ledger_closes_and_the_smallest_tractions_are_answered() {
    let base = spawn_app().await;
    for traction in [plate::MEASURED_TRACTION_MPA, 50.0] {
        let (status, text) = post(&base, "/solve", &plate_request(traction, &[], &[])).await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        let result = &body["result"];
        let (work, stored, heat) = (result["boundary_work_j"].as_f64().unwrap(), result["strain_energy_j"].as_f64().unwrap(), result["plastic_dissipation_j"].as_f64().unwrap());
        assert!(((work - stored - heat) / work).abs() < 1e-12, "{traction}: {work} {stored} {heat}");
        if traction < result["first_yield_traction_mpa"].as_f64().unwrap() {
            assert_eq!(heat, 0.0);
        }
        assert!((result["first_yield_traction_mpa"].as_f64().unwrap() - 63.494).abs() < 1e-3);
    }
    for traction in [1.0e-155, 1.0e-200, f64::MIN_POSITIVE] {
        let (status, text) = post(&base, "/solve", &plate_request(traction, &[20.0], &[5.0])).await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{traction:e}: {text}");
        assert!(body["invariants"]["energy_residual"].as_f64().unwrap() <= plate::ENERGY_TOLERANCE);
        assert!((body["result"]["stress_concentration_gross"].as_f64().unwrap() - 3.742).abs() < 1e-3);
        let (_, text) = post(&base, "/verify_solution", &body).await;
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
    }
    let (status, text) = post(&base, "/solve", &plate_request(-1.0e300, &[], &[])).await;
    assert_eq!(status, 422);
    assert!(text.contains("traction -1e300 MPa is not a pull") && text.len() < 400, "{text}");
}
