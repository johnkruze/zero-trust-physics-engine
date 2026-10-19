//! `argon_pair_gas` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door and its description.

mod answer_text;

use std::time::Duration;

use genesis_solvers::argon_pair_gas::{self as argon, Question};
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

fn gas(temperature_k: f64, pressure_kpa: f64) -> Value {
    json!({ "solver": "argon_pair_gas", "parameters": { "temperature_k": temperature_k, "pressure_kpa": pressure_kpa } })
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

fn flip_hex(text: &str) -> String {
    let last = if text.ends_with('0') { '1' } else { '0' };
    format!("{}{last}", &text[..text.len() - 1])
}

#[tokio::test]
async fn a_held_back_point_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let request = gas(325.046, 438.6);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = argon::solve(Question {
        temperature_k: 325.046,
        pressure_kpa: 438.6,
    })
    .expect("the solver settles");

    assert_eq!(body["inputs"], request);
    assert_eq!(body["engine_version"], format!("genesis_solvers_v{}", genesis_solvers::VERSION));
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert_eq!(body["iterations"], direct.refinements);
    let result = &body["result"];
    assert_number(result, "reduced_temperature", direct.reduced_temperature);
    assert_number(result, "second_virial_cm3_per_mol", direct.second_virial_cm3_per_mol);
    assert_number(
        result,
        "second_virial_first_derivative_cm3_per_mol",
        direct.second_virial_first_derivative_cm3_per_mol,
    );
    assert_number(
        result,
        "second_virial_second_derivative_cm3_per_mol",
        direct.second_virial_second_derivative_cm3_per_mol,
    );
    assert_number(result, "acoustic_virial_cm3_per_mol", direct.acoustic_virial_cm3_per_mol);
    assert_number(result, "sound_speed_m_per_s", direct.sound_speed_m_per_s);
    assert_number(result, "ideal_gas_sound_speed_m_per_s", direct.ideal_gas_sound_speed_m_per_s);
    assert_number(result, "collision_integral_22", direct.collision_integral_22);
    assert_number(result, "collision_integral_23", direct.collision_integral_23);
    assert_number(
        result,
        "viscosity_second_approximation_factor",
        direct.viscosity_second_approximation_factor,
    );
    assert_number(result, "viscosity_pa_s", direct.viscosity_pa_s);
    assert_eq!(result.as_object().unwrap().len(), 22, "eleven numbers and their hex");
    let invariants = &body["invariants"];
    assert_number(
        invariants,
        "virial_forms_residual_cm3_per_mol",
        direct.virial_forms_residual_cm3_per_mol,
    );
    assert_number(invariants, "least_turning_point_margin", direct.least_turning_point_margin);
    assert_number(invariants, "collision_integral_rule_gap", direct.collision_integral_rule_gap);

    // The field hash is the count of numbers, the energies and the cross-sections.
    let mut hasher = Sha256::new();
    hasher.update((direct.collision_energies.len() as u64 * 2).to_le_bytes());
    for value in direct.collision_energies.iter().chain(&direct.cross_sections) {
        hasher.update(value.to_le_bytes());
    }
    let own = hex::encode(hasher.finalize());
    let served = body["field_sha256"].as_str().expect("field hash");
    assert_eq!(served.len(), 64);
    assert_eq!(served, own);

    // The proof holds, and an edit of any number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    let mut edits = 0;
    for section in ["result", "invariants"] {
        let keys: Vec<String> = body[section].as_object().unwrap().keys().cloned().collect();
        for key in keys.iter().filter(|key| key.ends_with("_hex")) {
            let mut edited = body.clone();
            edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
            let (status, text) = post(&base, "/verify_solution", &edited).await;
            assert_eq!(status, 200, "{text}");
            let report: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(report["holds"], false, "{key}");
            assert_eq!(report["cause"], section, "{key}");
            edits += 1;
        }
    }
    assert_eq!(edits, 14, "eleven result numbers and three invariants");
    let mut field = body.clone();
    field["field_sha256"] = json!(flip_hex(served));
    let (_, text) = post(&base, "/verify_solution", &field).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);

    // The same question again is the same bytes.
    let (_, first) = post(&base, "/solve", &request).await;
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(first, again);
}

#[tokio::test]
async fn a_question_outside_the_range_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let cases = [
        (gas(249.9, 100.0), "TemperatureOutOfRange"),
        (gas(6000.000000000001, 100.0), "TemperatureOutOfRange"),
        (gas(1.0e9, 100.0), "TemperatureOutOfRange"),
        (gas(300.0, 500.001), "PressureOutOfRange"),
        (gas(300.0, -1.0), "PressureOutOfRange"),
        (gas(300.0, 6595.8), "PressureOutOfRange"),
        // The temperature is asked first, as the solver asks it.
        (gas(249.9, 0.0), "TemperatureOutOfRange"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.ends_with(&format!("({name}).")), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    let (_, text) = post(&base, "/solve", &gas(300.0, 500.001)).await;
    assert_eq!(text, "Physical Violation: pressure 500.001 kPa is outside 0 to 500 kPa, where the pressure series cut after its first term is written (PressureOutOfRange).");
    // The ends of both compared regions are answered: the speed of sound at
    // its pressures, the viscosity at zero pressure, as the scored run asked
    // all 49 of its points. And so is every question past them where engine
    // 0.8.0 refused by name, each with a seal that holds: pressures above
    // zero outside 30.5 to 467.2 kPa, the speed of sound above 349.868 K,
    // the gas at zero density below 291.09 K, and every property up to 6000 K.
    for request in [
        gas(250.0, 30.5),
        gas(349.868, 467.2),
        gas(291.09, 0.0),
        gas(681.96, 0.0),
        gas(500.0, -0.0),
        gas(300.0, 1.0),
        gas(300.0, 30.4),
        gas(300.0, 467.3),
        gas(300.0, 500.0),
        gas(681.96, 500.0),
        gas(349.87, 101.325),
        gas(681.96, 467.2),
        gas(250.0, 0.0),
        gas(291.08, -0.0),
        gas(681.97, 100.0),
        gas(2000.0, 0.0),
        gas(6000.0, 500.0),
    ] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{request} {text}");
        assert!(text.contains("\"outcome\":\"converged\""), "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        let (status, checked) = post(&base, "/verify_solution", &body).await;
        assert_eq!(status, 200, "{checked}");
        assert_eq!(serde_json::from_str::<Value>(&checked).unwrap()["holds"], true, "{checked}");
    }
    let others = [
        (
            json!({ "solver": "argon_pair_gas", "parameters": { "temperature_k": 300.0 } }),
            "Missing required parameter: 'pressure_kpa' (InputMissing).".to_string(),
        ),
        (
            json!({ "solver": "argon_pair_gas", "parameters": { "temperature_k": 300.0, "pressure_kpa": 100.0, "well_depth_k": 150.0 } }),
            "Physical Violation: 'well_depth_k' is not an input of this solver. The two constants of the pair potential, the molar mass, the quadrature rules and their tolerances stay inside the solver.".to_string(),
        ),
    ];
    for (request, message) in others {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert_eq!(text, message, "{request}");
    }
    let (status, text) = post(
        &base,
        "/solve",
        &json!({ "solver": "argon_pair_gas", "parameters": { "temperature_k": "300", "pressure_kpa": 100.0 } }),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(!text.contains("solution_seal"), "{text}");
}

#[tokio::test]
async fn the_description_says_what_is_fitted_and_its_example_runs_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let row = listed["solvers"]
        .as_array()
        .expect("solvers")
        .iter()
        .find(|row| row["solver"] == "argon_pair_gas")
        .expect("argon_pair_gas is listed");
    let summary = row["summary"].as_str().expect("a sentence");
    assert!(summary.contains("Fitted on 42 of the 66 measured speeds of sound at or below 500 kPa"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "argon_pair_gas" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["summary"], summary);
    for part in ["object", "data", "pass_line", "result", "limits"] {
        let words = described["validation"][part].as_str().unwrap_or_else(|| panic!("{part}"));
        assert!(!words.is_empty(), "{part}");
    }
    assert!(described["validation"]["limits"].as_str().unwrap().contains("extends the fit in temperature by up to 50 K"));
    assert!(described["validation"]["data"].as_str().unwrap().contains("calculated from an ab initio potential, not measured"));
    // Sentences held in the one line of the list and in each part of the
    // record.
    for words in [
        "Range: speed of sound compared at 250 to 349.868 K and 30.5 to 467.2 kPa, viscosity at 291.09 to 681.96 K; computes every property from 250 to 6000 K and 0 to 500 kPa with the same equations.",
        "Compared with the 24 speeds of sound not in the fit (310 to 349.868 K): within 0.0074 m/s RMS (worst 0.0146)",
        "within 1.47 percent mean (worst 2.97)",
        "Gap: the viscosity reads high at all 49 points, 2.95 percent at 291 K falling to 0.28 percent at 682 K",
        "(TemperatureOutOfRange)",
        "(PressureOutOfRange)",
    ] {
        assert!(summary.contains(words), "{words}: {summary}");
    }
    let data = described["validation"]["data"].as_str().unwrap();
    for words in [
        "the archive's compiler gives its own 95 percent uncertainty beside it, 0.50 to 0.59 m/s",
        "they are 17 distinct states, seven of them printed twice with different speeds",
        "the archive's compiler gives 0.30 to 0.31 percent at 95 percent",
        "The viscosity is at a density of 1.1 kg/m3, not at zero density",
        "what is measured is the ratio of the viscosity to its value at 298.15 K, and the absolute level of all 49 values rests on that one value",
        "the NIST ThermoML Archive, doi 10.18434/mds2-2422",
        "Fitted on the 42 points at or below 500 kPa of the eleven isotherms from 250 K to 300.117 K",
    ] {
        assert!(data.contains(words), "{words}: {data}");
    }
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    for words in [
        "RMS error at most 0.030 m/s, worst at most 0.060 m/s, RMS at most a quarter of the ideal gas's",
        "mean error at most 2.0 percent, worst at most 4.0 percent",
        "set knowing that textbook constants give about 2 percent",
    ] {
        assert!(pass_line.contains(words), "{words}: {pass_line}");
    }
    assert!(
        described["validation"]["limits"].as_str().unwrap().contains("The split and the targets were chosen with 29 of the 163 speed-of-sound rows read, from all 15 isotherms, and 11 of the 49 viscosity rows."),
        "the rows read before the split are stated"
    );
    let result = described["validation"]["result"].as_str().unwrap();
    for words in [
        "RMS 0.0074 m/s (target 0.030), worst 0.0146 m/s (target 0.060), RMS 0.023 of the ideal gas's 0.327 m/s (target 0.25)",
        "mean error 1.47 percent (target 2.0), worst 2.97 percent (target 4.0)",
        "above the measurement at all 49 points, by 2.95 percent at 291 K falling to 0.28 percent at 682 K: 1.4 to 20 times the measurement's uncertainty, 9 on average, and no point is within it",
        "143.84 K and 3.3379 angstrom fit all 49 points within 0.11 percent",
        "that pair is 0.22 m/s off on the speed of sound outside the fit; one pair of constants does not carry both properties",
        "a square well, van der Waals' second virial coefficient, a hard sphere with an r^-6 attraction, and Mie potentials from 9-6 to 30-6, each fitted on the same 42 points, give 0.0073 to 0.0107 m/s RMS",
        "the reference equation of state gives 0.0076, and repeated states differ by up to 0.016 m/s",
        "fitted at 250 to 300 K, it extrapolates 50 K",
        "SHA-256 90dfab953a94d199db146cfad89a4ce4bf750288c3b33b299fb2fed49e722066; a later build returns all 115 byte for byte, and engine 0.7.0 answers all 115, the 49 viscosity answers asked at zero pressure among them, the same in every field but the engine version and the receipt",
        "A second run on the same machine recomputes the same numbers.",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    assert!(!result.contains("ten to twenty times"), "{result}");
    assert!(described["validation"]["limits"].as_str().unwrap().contains(
        "B is higher by roughly 0.1 to 0.4 cm3/mol at 300 K, -15.05 cm3/mol here against -15.18 from the reference equation of state"
    ));
    assert_eq!(described["locked"]["well_depth_k"], argon::WELL_DEPTH_K);
    assert_eq!(described["locked"]["zero_distance_angstrom"], argon::ZERO_DISTANCE_ANGSTROM);
    assert_eq!(described["reference"]["kind"], "Reference calculations for the 12-6 potential itself: they hold the solver's integrals of the potential, and the comparison with argon is the validation record.");
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0]["minimum"], argon::TEMPERATURE_MIN_K);
    assert_eq!(inputs[0]["maximum"], argon::TEMPERATURE_MAX_K);
    assert_eq!(inputs[0]["maximum"], 6000.0);
    // Every property is answered at every pressure the series is written
    // for; the compared spans are stated in words.
    assert_eq!(inputs[1]["minimum"], argon::PRESSURE_MIN_KPA);
    assert_eq!(inputs[1]["maximum"], argon::PRESSURE_MAX_KPA);
    assert_eq!(inputs[1]["refusal"], "PressureOutOfRange");
    let bound = inputs[1]["bound"].as_str().unwrap();
    for words in [
        "The speed of sound was compared at 30.5 kPa to 467.2 kPa, and the viscosity at zero pressure",
        "Every property is returned at every pressure",
    ] {
        assert!(bound.contains(words), "{words}: {bound}");
    }
    assert!(!bound.contains("NotCompared"), "{bound}");
    let limits = described["validation"]["limits"].as_str().unwrap();
    assert!(limits.contains("Answered with the same equations and compared with no measurement: a pressure above zero outside 30.5 to 467.2 kPa; the speed of sound above 349.868 K; the viscosity below 291.09 K; every property above 681.96 K up to 6000 K"), "{limits}");
    assert!(!limits.contains("pressures above 500 kPa, where the solver refuses"), "{limits}");
    assert!(summary.contains("speed of sound compared at 250 to 349.868 K and 30.5 to 467.2 kPa, viscosity at 291.09 to 681.96 K"), "{summary}");
    let result = described["validation"]["result"].as_str().unwrap();
    assert!(result.contains("engine 0.7.0 answers all 115, the 49 viscosity answers asked at zero pressure among them, the same in every field but the engine version and the receipt."), "{result}");
    assert_eq!(described["refusals"].as_array().expect("refusals").len(), 9);
    for refusal in described["refusals"].as_array().unwrap() {
        assert!(!refusal["meaning"].as_str().unwrap().contains("..="), "{refusal}");
    }

    let example = described["example_solve"].clone();
    let (status, text) = post(&base, "/solve", &example).await;
    assert_eq!(status, 200, "{example} {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "converged");
    let speed = body["result"]["sound_speed_m_per_s"].as_f64().unwrap();
    let ideal = body["result"]["ideal_gas_sound_speed_m_per_s"].as_f64().unwrap();
    assert!(speed > ideal && speed - ideal < 0.2, "{speed} {ideal}");
    let viscosity = body["result"]["viscosity_pa_s"].as_f64().unwrap();
    assert!(viscosity > 2.0e-5 && viscosity < 2.6e-5, "{viscosity}");
}
