//! The journal bearing solver on the HTTP door: `POST /solve` against a
//! direct call to `genesis_solvers`, the bounds at the last number on each
//! side, a constant of the bearing sent as a field, the description, and
//! `POST /verify_solution` against edits of a sealed film.

use std::time::Duration;

use genesis_solvers::journal_bearing_sheffield as bearing;
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
    let text = serde_json::to_string(body).unwrap();
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
                return (status, response.text().await.unwrap());
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

async fn solved(base: &str, request: &Value) -> (String, Value) {
    let (status, text) = post(base, "/solve", request).await;
    assert_eq!(status, 200, "{text}");
    let body = serde_json::from_str(&text).unwrap();
    (text, body)
}

async fn verdict(base: &str, body: &Value) -> Value {
    let (status, text) = post(base, "/verify_solution", body).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn assert_fails(base: &str, body: &Value, part: &str, label: &str) {
    let report = verdict(base, body).await;
    assert_eq!(report["holds"], false, "{label}");
    assert_eq!(report["cause"], part, "{label}");
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

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The receipt covers the wire bytes before it.
fn assert_receipt(text: &str, body: &Value) {
    let receipt = body["receipt_sha256"].as_str().expect("receipt");
    let tail = format!(",\"receipt_sha256\":\"{receipt}\"}}");
    let document = text.strip_suffix(&tail).expect("the receipt is last");
    assert_eq!(sha256_hex(format!("{document}}}").as_bytes()), receipt);
}

fn film(load_n: f64, speed_rpm: f64) -> Value {
    json!({ "solver": "journal_bearing_sheffield", "parameters": { "load_n": load_n, "speed_rpm": speed_rpm } })
}

#[tokio::test]
async fn three_films_match_the_solver_bit_for_bit() {
    let base = spawn_app().await;
    for (load_n, speed_rpm) in [(2000.0, 800.0), (10000.0, 400.0), (20000.0, 100.0)] {
        let request = film(load_n, speed_rpm);
        let (text, body) = solved(&base, &request).await;
        let direct = bearing::solve(load_n, speed_rpm).expect("the solver converges");

        assert_eq!(body["inputs"], request);
        assert_eq!(body["engine_version"], "genesis_solvers_v0.8.0");
        assert_eq!(body["outcome"], "converged");
        assert!(body["reject"].is_null());
        assert!(body["reject_detail"].is_null());
        assert_eq!(body["iterations"], direct.shaft_positions);

        let result = &body["result"];
        assert_number(result, "sommerfeld_number", direct.sommerfeld_number);
        assert_number(result, "eccentricity_ratio", direct.eccentricity_ratio);
        assert_number(result, "minimum_film_thickness_m", direct.minimum_film_thickness_m);
        assert_eq!(result.as_object().unwrap().len(), 6, "a number and its hex for each");
        // Compared with no measurement, so not returned.
        assert!(result.get("attitude_angle_deg").is_none());
        assert!(result.get("peak_film_pressure_pa").is_none());

        let invariants = &body["invariants"];
        assert_number(invariants, "flow_balance_residual", direct.flow_balance_residual);
        assert_number(invariants, "load_residual_n", direct.load_residual_n);
        assert_eq!(invariants.as_object().unwrap().len(), 4);
        assert!(direct.flow_balance_residual <= bearing::FLOW_BALANCE_TOLERANCE);
        assert!(direct.load_residual_n.abs() <= 1.01 * bearing::LOAD_TOLERANCE * load_n);

        let mut field = Vec::with_capacity(8 + 8 * direct.film_pressure_pa.len());
        field.extend_from_slice(&(direct.film_pressure_pa.len() as u64).to_le_bytes());
        for sample in &direct.film_pressure_pa {
            field.extend_from_slice(&sample.to_le_bytes());
        }
        assert_eq!(
            direct.film_pressure_pa.len(),
            bearing::CELLS_AROUND * bearing::CELLS_ALONG_HALF
        );
        assert_eq!(body["field_sha256"], sha256_hex(&field));

        assert_eq!(body["solution_seal"].as_str().unwrap().len(), 64);
        assert_receipt(&text, &body);
        assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));
    }
}

/// The same question written three ways is one sealed request and one film.
#[tokio::test]
async fn a_number_written_three_ways_is_one_question() {
    let base = spawn_app().await;
    let mut seals = Vec::new();
    for text in [
        r#"{"solver":"journal_bearing_sheffield","parameters":{"load_n":6000,"speed_rpm":200}}"#,
        r#"{"solver":"journal_bearing_sheffield","parameters":{"speed_rpm":2e2,"load_n":6000.0}}"#,
        r#"{"parameters":{"load_n":6e3,"speed_rpm":200.0},"solver":"journal_bearing_sheffield"}"#,
    ] {
        let request: Value = serde_json::from_str(text).unwrap();
        let (_, body) = solved(&base, &request).await;
        assert_eq!(
            body["seal_parts"]["inputs"],
            sha256_hex(br#"{"parameters":{"load_n":x40b7700000000000,"speed_rpm":x4069000000000000},"solver":"journal_bearing_sheffield"}"#)
        );
        seals.push(body["solution_seal"].as_str().unwrap().to_string());
    }
    seals.dedup();
    assert_eq!(seals.len(), 1);
}

#[tokio::test]
async fn the_bounds_hold_at_the_last_number_on_each_side() {
    let base = spawn_app().await;
    let below = |value: f64| f64::from_bits(value.to_bits() - 1);
    let above = |value: f64| f64::from_bits(value.to_bits() + 1);
    for (load_n, speed_rpm, word) in [
        (below(bearing::LOAD_MIN_N), 400.0, "'load_n'"),
        (above(bearing::LOAD_MAX_N), 400.0, "'load_n'"),
        (10000.0, below(bearing::SPEED_MIN_RPM), "'speed_rpm'"),
        (10000.0, above(bearing::SPEED_MAX_RPM), "'speed_rpm'"),
        (0.0, 400.0, "'load_n'"),
        (-10000.0, 400.0, "'load_n'"),
        (10000.0, -400.0, "'speed_rpm'"),
        (1.0e300, 400.0, "'load_n'"),
    ] {
        let (status, text) = post(&base, "/solve", &film(load_n, speed_rpm)).await;
        assert_eq!(status, 422, "{load_n} {speed_rpm}: {text}");
        assert!(text.starts_with(&format!("Physical Violation: {word} ")), "{text}");
        assert!(text.contains("is outside"), "{text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // The four corners of the envelope are inside it.
    for (load_n, speed_rpm) in [
        (bearing::LOAD_MIN_N, bearing::SPEED_MIN_RPM),
        (bearing::LOAD_MAX_N, bearing::SPEED_MAX_RPM),
    ] {
        let (_, body) = solved(&base, &film(load_n, speed_rpm)).await;
        assert_eq!(body["outcome"], "converged");
    }
}

#[tokio::test]
async fn what_is_not_a_question_of_this_bearing_is_refused_before_the_solver() {
    let base = spawn_app().await;
    let solver = "journal_bearing_sheffield";
    for (request, fragment) in [
        (json!({ "solver": solver, "parameters": { "load_n": "10000", "speed_rpm": 400 } }), "'load_n' must be a number"),
        (json!({ "solver": solver, "parameters": { "load_n": 10000, "speed_rpm": null } }), "'speed_rpm' must be a number"),
        (json!({ "solver": solver, "parameters": { "load_n": [10000], "speed_rpm": 400 } }), "'load_n' must be a number"),
        (json!({ "solver": solver, "parameters": { "load_n": 10000 } }), "Missing required parameter: 'speed_rpm'"),
        (json!({ "solver": solver, "parameters": { "speed_rpm": 400 } }), "Missing required parameter: 'load_n'"),
        (json!({ "solver": solver, "parameters": {} }), "Missing required parameter: 'load_n'"),
        (json!({ "solver": solver }), "Missing required parameter: 'parameters' (or 'example', the name of a worked example describe_solver lists)"),
        // A constant of the bearing, of the oil or of the method sent as a field.
        (json!({ "solver": solver, "parameters": { "load_n": 10000, "speed_rpm": 400, "radial_clearance_m": 5.0e-5 } }), "'radial_clearance_m' is not an input of this solver"),
        (json!({ "solver": solver, "parameters": { "load_n": 10000, "speed_rpm": 400, "oil_viscosity_at_65c_pa_s": 0.03 } }), "'oil_viscosity_at_65c_pa_s' is not an input of this solver"),
        (json!({ "solver": solver, "parameters": { "load_n": 10000, "speed_rpm": 400, "film_viscosity_multiplier": 1.0 } }), "'film_viscosity_multiplier' is not an input of this solver"),
        (json!({ "solver": solver, "parameters": { "load_n": 10000, "speed_rpm": 400, "cells_around": 18 } }), "'cells_around' is not an input of this solver"),
        (json!({ "solver": solver, "parameters": { "load_n": 10000, "speed_rpm": 400 }, "max_sweeps": 5 }), "'max_sweeps' is not an input of this solver"),
        // The other solver's question.
        (json!({ "solver": solver, "parameters": { "atomic_number": 10 } }), "'atomic_number' is not an input of this solver"),
        (json!({ "solver": "atom_dft", "parameters": { "load_n": 10000, "speed_rpm": 400 } }), "'load_n' is not an input of this solver"),
    ] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request}: {text}");
        assert!(text.contains(fragment), "{request}: {text}");
    }
    // A number too large to be a 64-bit number, written in the request text.
    let client = Client::new();
    let response = client
        .post(format!("{base}/solve"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(r#"{"solver":"journal_bearing_sheffield","parameters":{"load_n":1e400,"speed_rpm":400}}"#)
        .send()
        .await
        .unwrap();
    assert!(response.status().is_client_error());
    assert!(!response.text().await.unwrap().contains("solution_seal"));

    let (status, text) = post(&base, "/solve", &json!({ "solver": "journal_bearing", "parameters": { "load_n": 10000, "speed_rpm": 400 } })).await;
    assert_eq!(status, 404, "{text}");
    assert_eq!(text, "Solver not recognized");
}

#[tokio::test]
async fn the_bearing_is_listed_and_described_and_its_example_runs_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 13);
    assert_eq!(solvers[2]["solver"], "journal_bearing_sheffield");
    assert!(solvers[2]["summary"].as_str().unwrap().ends_with('.'));

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "journal_bearing_sheffield" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["inputs"][0]["name"], "load_n");
    assert_eq!(described["inputs"][0]["minimum"], 2000.0);
    assert_eq!(described["inputs"][0]["maximum"], 20000.0);
    assert_eq!(described["inputs"][1]["name"], "speed_rpm");
    assert_eq!(described["inputs"][1]["minimum"], 100.0);
    assert_eq!(described["inputs"][1]["maximum"], 800.0);
    assert_eq!(described["locked"]["bearing_diameter_m"], 0.112);
    assert_eq!(described["locked"]["bearing_length_m"], 0.05055);
    assert_eq!(described["locked"]["radial_clearance_m"], 45.0e-6);
    assert_eq!(described["locked"]["oil_viscosity_at_65c_pa_s"], 0.02722);
    assert_eq!(described["locked"]["film_viscosity_multiplier"], 0.8201);
    assert!(described["locked"]["film_viscosity_multiplier_source"].as_str().unwrap().starts_with("Fitted"));
    assert_eq!(described["invariants"][0]["tolerance"], 1.0e-9);
    assert_eq!(described["invariants"][1]["tolerance"], 1.0e-10);
    assert_eq!(described["reference"]["retrieved"], "2026-10-09");
    assert_eq!(described["reference"]["kind"], "A reference calculation. It is not a measurement.");
    // The validation record is in the five parts a card's description gives.
    let validation = described["validation"].as_object().expect("a record");
    let parts: Vec<&str> = validation.keys().map(String::as_str).collect();
    assert_eq!(parts, ["object", "data", "pass_line", "result", "limits"]);
    let pass_line = validation["pass_line"].as_str().unwrap();
    assert!(pass_line.contains("mean of the absolute residual at most 0.15"), "{pass_line}");
    // The validation record states each comparison as it came out, the one that missed included.
    let result = validation["result"].as_str().unwrap();
    assert!(result.contains("on the thirty-four opened runs mean absolute 0.1293, worst absolute 0.4761, mean 0.1277, not met"), "{result}");
    assert!(result.contains("All three lines met on the sixteen held-back runs: mean absolute 0.0580, worst absolute 0.1563, mean 0.0199"), "{result}");
    assert!(result.contains("Not gated") && result.contains("0.9356"), "{result}");
    // Only what was compared is returned, and the limits say what is not.
    let summary = solvers[2]["summary"].as_str().unwrap();
    assert!(summary.contains("Sommerfeld number, eccentricity ratio and minimum film thickness"), "{summary}");
    assert!(!summary.contains("attitude") && !summary.contains("pressure"), "{summary}");
    let limits = validation["limits"].as_str().unwrap();
    assert!(limits.contains("The solver returns no attitude angle, no pressure"), "{limits}");
    let returned: Vec<&str> = described["returns"]["result"]
        .as_array()
        .expect("names")
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(returned, ["sommerfeld_number", "eccentricity_ratio", "minimum_film_thickness_m"]);
    let refusals = described["refusals"].as_array().expect("refusals");
    assert_eq!(refusals.len(), 7);
    for refusal in refusals {
        assert!(refusal["name"].is_string());
        assert!(refusal["meaning"].as_str().unwrap().ends_with('.'));
    }
    // Every result and invariant the description names is in a solution, and no other.
    let (_, body) = solved(&base, &described["example_solve"]).await;
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["inputs"], described["example_solve"]);
    for (section, count) in [("result", 3), ("invariants", 2)] {
        let named = described["returns"][section].as_array().expect("names");
        assert_eq!(named.len(), count);
        for entry in named {
            let name = entry["name"].as_str().unwrap();
            assert!(body[section][name].is_number(), "{section}.{name}");
            assert!(body[section][format!("{name}_hex")].is_string(), "{section}.{name}_hex");
        }
        assert_eq!(body[section].as_object().unwrap().len(), 2 * count);
    }
    assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));

    // The atom is described as before, and a name between the two is neither.
    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "atom_dft" })).await;
    assert_eq!(status, 200, "{text}");
    assert!(!text.contains("load_n"));
    let (status, _) = post(&base, "/describe_solver", &json!({ "solver": "journal_bearing_sheffield " })).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn an_edited_film_is_named_by_its_part() {
    let base = spawn_app().await;
    let (_, body) = solved(&base, &film(8000.0, 600.0)).await;

    // A thicker film than the solver found.
    let mut thicker = body.clone();
    let thin = body["result"]["minimum_film_thickness_m"].as_f64().unwrap();
    thicker["result"]["minimum_film_thickness_m_hex"] = json!(hex_of(2.0 * thin));
    assert_fails(&base, &thicker, "result", "a thicker film").await;

    // The decimal alone is not in the seal; the hex is the value.
    let mut decimal = body.clone();
    decimal["result"]["minimum_film_thickness_m"] = json!(2.0 * thin);
    assert_eq!(verdict(&base, &decimal).await["holds"], true);

    let mut balanced = body.clone();
    balanced["invariants"]["flow_balance_residual_hex"] = json!(hex_of(0.0));
    assert_fails(&base, &balanced, "invariants", "a better flow balance").await;

    let mut other_load = body.clone();
    other_load["inputs"]["parameters"]["load_n"] = json!(4000.0);
    assert_fails(&base, &other_load, "inputs", "another load").await;

    let mut other_field = body.clone();
    other_field["field_sha256"] = json!("0".repeat(64));
    assert_fails(&base, &other_field, "field_sha256", "another field").await;

    let mut fewer = body.clone();
    fewer["iterations"] = json!(1);
    assert_fails(&base, &fewer, "outcome", "fewer shaft positions").await;

    let mut refused = body.clone();
    refused["outcome"] = json!("refused");
    refused["reject"] = json!("FilmNotSettled");
    assert_fails(&base, &refused, "outcome", "a refusal put on a film").await;

    // A film cannot carry the atom's names, nor the atom the film's.
    let mut borrowed = body.clone();
    borrowed["result"]["total_energy_ha_hex"] = json!(hex_of(-1.0));
    let (status, text) = post(&base, "/verify_solution", &borrowed).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'result.total_energy_ha_hex' is not published by this solver"), "{text}");

    // The angle and the peak pressure, compared with no measurement, are not published.
    for name in ["attitude_angle_deg_hex", "peak_film_pressure_pa_hex"] {
        let mut added = body.clone();
        added["result"][name] = json!(hex_of(1.0));
        let (status, text) = post(&base, "/verify_solution", &added).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains(&format!("'result.{name}' is not published by this solver")), "{text}");
    }

    let mut relabelled = body.clone();
    relabelled["inputs"]["solver"] = json!("atom_dft");
    let (status, text) = post(&base, "/verify_solution", &relabelled).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("is not published by this solver"), "{text}");
}
