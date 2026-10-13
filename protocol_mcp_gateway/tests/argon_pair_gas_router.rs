//! `argon_pair_gas` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door and its description.

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
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));
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
        (gas(681.97, 100.0), "TemperatureOutOfRange"),
        (gas(1.0e9, 100.0), "TemperatureOutOfRange"),
        (gas(300.0, 500.001), "PressureOutOfRange"),
        (gas(300.0, -1.0), "PressureOutOfRange"),
        (gas(300.0, 6595.8), "PressureOutOfRange"),
        // Above zero and outside the pressures compared with measurement:
        // no proof is given there.
        (gas(300.0, 1.0), "PressureNotCompared"),
        (gas(300.0, 30.4), "PressureNotCompared"),
        (gas(300.0, 467.3), "PressureNotCompared"),
        (gas(300.0, 500.0), "PressureNotCompared"),
        (gas(681.96, 500.0), "PressureNotCompared"),
        // The speed of sound above its compared temperatures.
        (gas(349.87, 101.325), "SoundSpeedNotCompared"),
        (gas(681.96, 467.2), "SoundSpeedNotCompared"),
        // The gas at zero density below the viscosity's compared temperatures.
        (gas(250.0, 0.0), "ViscosityNotCompared"),
        (gas(291.08, -0.0), "ViscosityNotCompared"),
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
    let (_, text) = post(&base, "/solve", &gas(300.0, 500.0)).await;
    assert_eq!(
        text,
        "Physical Violation: pressure 500 kPa is above zero and outside 30.5..=467.2 kPa, the pressures at which the speed of sound was compared with measurement; the viscosity was compared at zero pressure only (PressureNotCompared)."
    );
    let (_, text) = post(&base, "/solve", &gas(300.0, 500.001)).await;
    assert_eq!(text, "Physical Violation: pressure 500.001 kPa is outside 0..=500 kPa (PressureOutOfRange).");
    // The ends of both compared regions are answered: the speed of sound at
    // its pressures, the viscosity at zero pressure, as the scored run asked
    // all 49 of its points.
    for request in [gas(250.0, 30.5), gas(349.868, 467.2), gas(291.09, 0.0), gas(681.96, 0.0), gas(500.0, -0.0)] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{request} {text}");
        assert!(text.contains("\"outcome\":\"converged\""), "{text}");
    }
    let others = [
        (
            json!({ "solver": "argon_pair_gas", "parameters": { "temperature_k": 300.0 } }),
            "Missing required parameter: 'pressure_kpa'".to_string(),
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
    assert!(summary.contains("fitted to the measured speed of sound in argon"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "argon_pair_gas" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["summary"], summary);
    for part in ["object", "data", "pass_line", "result", "limits"] {
        let words = described["validation"][part].as_str().unwrap_or_else(|| panic!("{part}"));
        assert!(!words.is_empty(), "{part}");
    }
    assert!(described["validation"]["limits"].as_str().unwrap().contains("extrapolation in temperature"));
    assert!(described["validation"]["data"].as_str().unwrap().contains("calculated from an ab initio potential, not measured"));
    // Sentences held in the one line of the list and in each part of the
    // record.
    assert!(summary.ends_with(
        "Each property is answered where it was compared: the speed of sound from 250 K to 349.868 K at 30.5 kPa to 467.2 kPa, the viscosity from 291.09 K to 681.96 K at zero pressure; elsewhere the solver refuses by name."
    ), "{summary}");
    let data = described["validation"]["data"].as_str().unwrap();
    for words in [
        "the archive's compiler gives its own 95 percent uncertainty beside it, 0.50 to 0.59 m/s",
        "they are 17 distinct states, seven of them printed twice with different speeds",
        "the archive's compiler gives 0.30 to 0.31 percent at 95 percent",
        "The viscosity is at a density of 1.1 kg/m3, not at zero density",
        "what is measured is the ratio of the viscosity to its value at 298.15 K, and the absolute level of all 49 values rests on that one value",
        "the NIST ThermoML Archive, doi 10.18434/mds2-2422",
        "the National Institute of Standards and Technology be acknowledged as the source of the data",
    ] {
        assert!(data.contains(words), "{words}: {data}");
    }
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    for words in [
        "They were drawn knowing that textbook constants give about 2 percent, so meeting them is not evidence of agreement with the measurement",
        "Rows of both data sets were seen before the split",
    ] {
        assert!(pass_line.contains(words), "{words}: {pass_line}");
    }
    let result = described["validation"]["result"].as_str().unwrap();
    for words in [
        "All five lines were met.",
        "root mean square error 0.0074 m/s, largest 0.0146 m/s",
        "mean error 1.47 percent, largest 2.97 percent",
        "above the measurement at all 49 points, by 2.95 percent at 291 K falling to 0.28 percent at 682 K: 1.4 to 20 times the measurement's uncertainty, 9 on average, and no point is within it",
        "143.84 K and 3.3379 angstrom fit all 49 points within 0.11 percent",
        "What fails is one pair of constants for both properties: that pair misses the held-back speed of sound by 0.22 m/s",
        "does not distinguish the 12-6 form from a square well, van der Waals' second virial coefficient, a hard sphere with an r^-6 attraction, or Mie potentials from 9-6 to 30-6",
        "meets the same lines, at 0.0073 to 0.0107 m/s",
        "the reference equation of state scores 0.0076, and repeated states differ by up to 0.016 m/s",
        "the second acoustic virial coefficient fitted at 250 to 300 K extrapolates 50 K",
        "SHA-256 90dfab953a94d199db146cfad89a4ce4bf750288c3b33b299fb2fed49e722066) was overwritten by a later build, which returned all 115 saved answers byte for byte; engine 0.7.0 answers all 115, the 49 viscosity answers asked at zero pressure among them, the same in every field but the engine version and the receipt",
        "evidence/checks/second_party/V-34/",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    assert!(!result.contains("ten to twenty times"), "{result}");
    assert!(described["validation"]["limits"].as_str().unwrap().contains(
        "B is higher by roughly 0.1 to 0.4 cm3/mol at 300 K, -15.05 cm3/mol here against -15.18 from the reference equation of state"
    ));
    assert_eq!(described["locked"]["well_depth_k"], argon::WELL_DEPTH_K);
    assert_eq!(described["locked"]["zero_distance_angstrom"], argon::ZERO_DISTANCE_ANGSTROM);
    assert_eq!(described["reference"]["kind"], "Reference calculations for the 12-6 potential. They are not measurements, and they say nothing about argon.");
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0]["minimum"], argon::TEMPERATURE_MIN_K);
    assert_eq!(inputs[0]["maximum"], argon::TEMPERATURE_MAX_K);
    // The pressure says which property is asked, and each is answered where
    // it was compared.
    assert_eq!(inputs[1]["minimum"], argon::PRESSURE_MIN_KPA);
    assert_eq!(inputs[1]["maximum"], argon::PRESSURE_MAX_KPA);
    assert_eq!(inputs[1]["refusal"], "PressureOutOfRange");
    let bound = inputs[1]["bound"].as_str().unwrap();
    for words in [
        "From 30.5 kPa to 467.2 kPa, the span of the measured speeds of sound the solver was compared with, it asks the speed of sound, which is answered from 250 K to 349.868 K and refused above as SoundSpeedNotCompared.",
        "Zero asks the gas at zero density, where the viscosity was compared: it is answered from 291.09 K to 681.96 K and refused below as ViscosityNotCompared",
        "is refused as PressureNotCompared",
    ] {
        assert!(bound.contains(words), "{words}: {bound}");
    }
    assert!(!bound.contains("Zero gives the ideal gas"), "{bound}");
    let limits = described["validation"]["limits"].as_str().unwrap();
    assert!(limits.contains("Not tested, and refused by name: a pressure above zero outside 30.5 to 467.2 kPa; temperatures below 250 K or above 681.96 K; the speed of sound above 349.868 K; the gas at zero pressure below 291.09 K."), "{limits}");
    assert!(!limits.contains("pressures above 500 kPa, where the solver refuses"), "{limits}");
    assert!(summary.contains("the speed of sound from 250 K to 349.868 K at 30.5 kPa to 467.2 kPa, the viscosity from 291.09 K to 681.96 K at zero pressure"), "{summary}");
    let result = described["validation"]["result"].as_str().unwrap();
    assert!(result.contains("engine 0.7.0 answers all 115, the 49 viscosity answers asked at zero pressure among them, the same in every field but the engine version and the receipt."), "{result}");
    assert_eq!(described["refusals"].as_array().expect("refusals").len(), 12);

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
