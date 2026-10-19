//! The stress_plate_hole_s235_elastic solver through the local connection
//! and through `POST /solve`. Stamped from the kit: the whole served
//! contract, attacked from the solver's own description by
//! `conformance_support`, and each refusal's sentence word for word from the
//! solver's source. Then the solver's own worked answers: the laboratory's
//! load bit for bit with its proof, the same field as stress_plate_hole_s235
//! wherever that solver's steel has not yielded, a traction past the one
//! compared answered with its flag, every limit the request shows at the
//! door with its name, and the strain limit, sealed.

mod conformance_support;

mod answer_text;

use std::time::Duration;

use conformance_support::{Door, assert_solver_conforms};
use genesis_solvers::stress_plate_hole_s235_elastic::{self as plate, ElasticPlateQuestion};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const SOURCE: &str = include_str!("../../genesis_solvers/src/stress_plate_hole_s235_elastic.rs");

#[test]
fn stress_plate_hole_s235_elastic_meets_the_served_contract() {
    assert_solver_conforms("stress_plate_hole_s235_elastic");
}

#[test]
fn every_refusal_carries_the_sentence_of_its_source() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "stress_plate_hole_s235_elastic" })).expect("described");
    let start = SOURCE.find("pub enum ElasticPlateReject {").expect("the enum is in the solver");
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

/// The worked question converges, and the list row and the description say
/// what it is: the second form of stress_plate_hole_s235, with its numbers.
#[test]
fn the_worked_question_converges_and_the_word_is_calibrated() {
    let mut door = Door::start();
    let description: Value = door.value("describe_solver", &json!({ "solver": "stress_plate_hole_s235_elastic" })).expect("described");
    let answer = door.value("solve", &description["example_solve"]).expect("answered");
    assert_eq!(answer["outcome"], "converged", "{answer}");
    assert_eq!(answer["iterations"], 0);
    assert_eq!(answer["result"]["past_compared_traction"], 0.0);
    let result = description["validation"]["result"].as_str().expect("a result");
    assert!(result.starts_with("On stress_plate_hole_s235's error measures at 106.26 MPa"), "{result}");
    assert!(result.contains("RMS 0.638 um in u_x (target 2.0) and 0.486 um in u_y (target 1.5)"), "{result}");
    let list: Value = door.value("list_solvers", &json!({})).expect("listed");
    let rows = list["solvers"].as_array().expect("rows");
    let row = rows.iter().find(|row| row["solver"] == "stress_plate_hole_s235_elastic").expect("listed");
    assert_eq!(row["against_measurement"], "compared with measurement", "{row}");
    let summary = row["summary"].as_str().expect("a summary");
    assert!(summary.contains("Nothing fitted here") && summary.contains("up to 590.04 MPa") && summary.contains("(StrainBeyondSmallStrain)"), "{summary}");
    let original = rows.iter().find(|row| row["solver"] == "stress_plate_hole_s235").expect("listed");
    assert_eq!(original["against_measurement"], "compared with measurement", "{original}");
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

fn plate_request(solver: &str, traction: f64, xs: &[f64], ys: &[f64]) -> Value {
    json!({ "solver": solver, "parameters": { "traction_mpa": traction, "points_x_mm": xs, "points_y_mm": ys } })
}

fn elastic_request(traction: f64, xs: &[f64], ys: &[f64]) -> Value {
    plate_request("stress_plate_hole_s235_elastic", traction, xs, ys)
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
/// gives it, with its proof; nothing yields.
#[tokio::test]
async fn the_laboratory_s_load_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let (xs, ys) = ([0.0, 0.0, 4.5, -20.0], [4.0, 10.025, 0.0, 5.0]);
    let request = elastic_request(plate::COMPARED_TRACTION_MPA, &xs, &ys);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = plate::solve(&ElasticPlateQuestion { traction_mpa: plate::COMPARED_TRACTION_MPA, points_x_mm: xs.to_vec(), points_y_mm: ys.to_vec() }).expect("settles");
    assert_eq!(body["inputs"], request);
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["iterations"], 0);
    let result = &body["result"];
    assert_number(result, "hole_edge_stress_mpa", direct.hole_edge_stress_mpa);
    assert_number(result, "stress_concentration_net", direct.stress_concentration_net);
    assert_number(result, "peak_von_mises_mpa", direct.peak_von_mises_mpa);
    assert_number(result, "parallel_section_elongation_mm", direct.parallel_section_elongation_mm);
    assert_number(result, "past_compared_traction", 0.0);
    assert_numbers(result, "point_stress_xx_mpa", &direct.points.iter().map(|p| p.stress_xx_mpa).collect::<Vec<_>>());
    assert_numbers(result, "point_displacement_x_mm", &direct.points.iter().map(|p| p.displacement_x_mm).collect::<Vec<_>>());
    assert_numbers(result, "point_displacement_y_mm", &direct.points.iter().map(|p| p.displacement_y_mm).collect::<Vec<_>>());
    assert_eq!(result.as_object().unwrap().len(), 2 * 21, "twenty-one values and their hex");
    assert!((direct.hole_edge_stress_mpa - 397.6).abs() < 0.1);
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
    for (section, key) in [("result", "hole_edge_stress_mpa_hex"), ("result", "past_compared_traction_hex"), ("invariants", "energy_residual_hex")] {
        let mut edited = body.clone();
        edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
        let (_, text) = post(&base, "/verify_solution", &edited).await;
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(serde_json::to_string(&body).unwrap(), serde_json::to_string(&serde_json::from_str::<Value>(&again).unwrap()).unwrap());
}

/// Wherever stress_plate_hole_s235's steel has not yielded, below 63.49
/// MPa, the two solvers serve the same field: the same points, bit for bit,
/// and the same field hash.
#[tokio::test]
async fn below_first_yield_the_two_solvers_serve_the_same_field() {
    let base = spawn_app().await;
    let (xs, ys) = ([0.0, 1.0, -20.0, 30.0], [5.5, -6.3, 5.0, 9.0]);
    let (_, text) = post(&base, "/solve", &elastic_request(50.0, &xs, &ys)).await;
    let elastic: Value = serde_json::from_str(&text).unwrap();
    let (_, text) = post(&base, "/solve", &plate_request("stress_plate_hole_s235", 50.0, &xs, &ys)).await;
    let original: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(original["outcome"], "converged");
    assert_eq!(original["result"]["plastic_zone_area_mm2"], 0.0);
    assert_eq!(elastic["field_sha256"], original["field_sha256"]);
    for key in ["point_displacement_x_mm_hex", "point_displacement_y_mm_hex", "point_stress_xx_mpa_hex", "point_strain_xx_hex", "hole_edge_stress_mpa_hex", "strain_energy_j_hex", "boundary_work_j_hex"] {
        assert_eq!(elastic["result"][key], original["result"][key], "{key}");
    }
}

/// Past the compared traction, and past 235 MPa where stress_plate_hole_s235
/// refuses, the elastic field is answered and sealed, with its flag; its
/// proof holds.
#[tokio::test]
async fn past_the_compared_traction_the_field_is_answered_with_its_flag() {
    let base = spawn_app().await;
    for traction in [106.26000000000002, 150.0, 300.0, 590.04] {
        let (status, text) = post(&base, "/solve", &elastic_request(traction, &[0.0], &[5.5])).await;
        assert_eq!(status, 200, "{traction}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{traction}");
        assert_eq!(body["result"]["past_compared_traction"], 1.0, "{traction}");
        let (_, text) = post(&base, "/verify_solution", &body).await;
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
    }
    let (status, text) = post(&base, "/solve", &plate_request("stress_plate_hole_s235", 300.0, &[], &[])).await;
    assert_eq!(status, 422);
    assert!(text.contains("(TractionAboveYield)."), "{text}");
}

#[tokio::test]
async fn a_limit_the_request_shows_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let many = vec![20.0; plate::MAX_POINTS + 1];
    let zeros = vec![0.0; plate::MAX_POINTS + 1];
    let cases = [
        (elastic_request(0.0, &[], &[]), "TractionNotTensile"),
        (elastic_request(-50.0, &[], &[]), "TractionNotTensile"),
        (elastic_request(100.0, &[0.0, 1.0], &[5.0]), "PointCountsDiffer"),
        (elastic_request(100.0, &many, &zeros), "TooManyPoints"),
        (elastic_request(100.0, &[45.00000000000001], &[0.0]), "PointOutsidePlate"),
        (elastic_request(100.0, &[0.0], &[-10.025000000000002]), "PointOutsidePlate"),
        (elastic_request(100.0, &[0.0], &[3.9999999999999996]), "PointInsideHole"),
        (elastic_request(100.0, &[2.0], &[-2.0]), "PointInsideHole"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name}).")), "{name}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // A key left out, and a constant of the specimen, are refused.
    let (status, _) = post(&base, "/solve", &json!({ "solver": "stress_plate_hole_s235_elastic", "parameters": { "traction_mpa": 100.0, "points_x_mm": [] } })).await;
    assert_eq!(status, 422);
    let (status, _) = post(&base, "/solve", &json!({ "solver": "stress_plate_hole_s235_elastic", "parameters": { "traction_mpa": 100.0, "points_x_mm": [], "points_y_mm": [], "yield_strength_mpa": 300.0 } })).await;
    assert_eq!(status, 422);
}

/// From 590.05 MPa the strain at the hole passes 1 percent: the question is
/// answered with a sealed refusal by name, and its proof holds.
#[tokio::test]
async fn past_the_small_strain_limit_the_refusal_is_named_and_sealed() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/solve", &elastic_request(590.05, &[], &[])).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "StrainBeyondSmallStrain");
    assert!(body["reject_detail"].as_str().unwrap().contains("past 0.01, where the linear measure of strain no longer holds"), "{body}");
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
}

/// The ledger closes from the two numbers served; the smallest tractions
/// are answered, sealed, with their ledger closed; a refusal prints a number
/// short.
#[tokio::test]
async fn the_served_ledger_closes_and_the_smallest_tractions_are_answered() {
    let base = spawn_app().await;
    for traction in [plate::COMPARED_TRACTION_MPA, 400.0] {
        let (status, text) = post(&base, "/solve", &elastic_request(traction, &[], &[])).await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        let result = &body["result"];
        let (work, stored) = (result["boundary_work_j"].as_f64().unwrap(), result["strain_energy_j"].as_f64().unwrap());
        assert!(((work - stored) / work).abs() < 1e-12, "{traction}: {work} {stored}");
    }
    for traction in [1.0e-155, 1.0e-200, f64::MIN_POSITIVE] {
        let (status, text) = post(&base, "/solve", &elastic_request(traction, &[20.0], &[5.0])).await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{traction:e}: {text}");
        assert!(body["invariants"]["energy_residual"].as_f64().unwrap() <= plate::ENERGY_TOLERANCE);
        assert!((body["result"]["stress_concentration_gross"].as_f64().unwrap() - 3.742).abs() < 1e-3);
        let (_, text) = post(&base, "/verify_solution", &body).await;
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
    }
    let (status, text) = post(&base, "/solve", &elastic_request(-1.0e300, &[], &[])).await;
    assert_eq!(status, 422);
    assert!(text.contains("traction -1e300 MPa is not a pull") && text.len() < 400, "{text}");
}
