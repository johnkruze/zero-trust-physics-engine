//! `junction_nist_b7_s2` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door, a sealed refusal, a
//! Shapiro step and its description.

mod answer_text;

use std::time::Duration;

use genesis_solvers::junction_nist_b7_s2::{self as junction, Drive, JunctionQuestion, Start};
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

fn held(bias: Value, start: &str) -> Value {
    json!({ "solver": "junction_nist_b7_s2", "parameters": { "bias_current_a": bias, "start": start, "drive": "none" } })
}

fn driven(bias: Value, start: &str, amplitude: Value, frequency: Value) -> Value {
    json!({ "solver": "junction_nist_b7_s2", "parameters": { "bias_current_a": bias, "start": start, "drive": "microwave", "drive_current_amplitude_a": amplitude, "drive_frequency_hz": frequency } })
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
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

/// Two scored points of the held-back comparison (the plateau at 0.47315 mA
/// rising, and the falling sweep's worst point, 0.11133 mA), answered bit for
/// bit as the solver gives them, with their proof.
#[tokio::test]
async fn compared_points_match_the_solver_bit_for_bit_and_their_proof_holds() {
    let base = spawn_app().await;
    let bias = [0.00047315330606175, 0.000111330189661589];
    let request = held(json!(bias), "running");
    let (status, first) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{first}");
    let body: Value = serde_json::from_str(&first).unwrap();
    let direct = junction::solve(&JunctionQuestion { bias_current_a: bias.to_vec(), start: Start::Running, drive: Drive::None }).expect("solves");
    assert_eq!(body["inputs"], request);
    assert_eq!(body["engine_version"], format!("genesis_solvers_v{}", genesis_solvers::VERSION));
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert_eq!(body["iterations"], direct.total_steps);
    let result = &body["result"];
    assert_numbers(result, "bias_current_a", &direct.bias_current_a);
    assert_numbers(result, "mean_voltage_v", &direct.mean_voltage_v);
    assert_numbers(result, "josephson_frequency_hz", &direct.josephson_frequency_hz);
    assert_numbers(result, "phase_turns", &direct.phase_turns);
    assert_numbers(result, "averaging_time_s", &direct.averaging_time_s);
    assert_numbers(result, "peak_voltage_v", &direct.peak_voltage_v);
    assert_numbers(result, "source_power_w", &direct.source_power_w);
    assert_numbers(result, "dissipated_power_w", &direct.dissipated_power_w);
    assert_numbers(result, "normal_line_reached", &direct.normal_line_reached);
    assert_numbers(result, "time_step_s", &direct.time_step_s);
    assert_numbers(result, "steps", &direct.steps);
    assert_eq!(result.as_object().unwrap().len(), 22, "eleven lists and their hex");
    // The falling sweep's worst point: 0.11133 mA, measured 2.58634 mV.
    assert_eq!(direct.mean_voltage_v[1], 0.0020653733411307145);
    let invariants = &body["invariants"];
    assert_eq!(invariants["largest_energy_residual_hex"], hex_of(direct.largest_energy_residual));
    assert_eq!(invariants["largest_flux_residual_quanta_hex"], hex_of(direct.largest_flux_residual_quanta));
    assert_eq!(invariants["largest_settle_difference_hex"], hex_of(direct.largest_settle_difference));
    assert!(direct.largest_energy_residual <= junction::RESIDUAL_CONSERVATION_LIMIT);

    // The field hash is the count, the final phases, then the final voltages.
    let mut hasher = Sha256::new();
    hasher.update((2 * direct.final_phase_rad.len() as u64).to_le_bytes());
    for value in direct.final_phase_rad.iter().chain(&direct.final_voltage_v) {
        hasher.update(value.to_le_bytes());
    }
    assert_eq!(body["field_sha256"].as_str().expect("field hash"), hex::encode(hasher.finalize()));

    // The proof holds, and an edited number breaks it.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
    for (section, key, index) in [("result", "mean_voltage_v_hex", 0), ("result", "mean_voltage_v_hex", 1), ("result", "phase_turns_hex", 0)] {
        let mut edited = body.clone();
        edited[section][key][index] = json!(flip_hex(body[section][key][index].as_str().unwrap()));
        let (_, text) = post(&base, "/verify_solution", &edited).await;
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }
    let mut ledger = body.clone();
    ledger["invariants"]["largest_energy_residual_hex"] = json!(flip_hex(body["invariants"]["largest_energy_residual_hex"].as_str().unwrap()));
    let (_, text) = post(&base, "/verify_solution", &ledger).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(again, first);
}

/// A Shapiro step through the engine: at 650 GHz and 0.2 mA, the mean voltage
/// at 0.3 mA is twice h f / 2e, and its proof holds.
#[tokio::test]
async fn a_shapiro_step_is_served_with_its_proof() {
    let base = spawn_app().await;
    let request = driven(json!([0.0003]), "running", json!(0.0002), json!(650_000_000_000.0));
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "converged");
    let frequency = body["result"]["josephson_frequency_hz"][0].as_f64().unwrap();
    assert!((frequency / 650.0e9 - 2.0).abs() < 1e-10, "{frequency}");
    let voltage = body["result"]["mean_voltage_v"][0].as_f64().unwrap();
    assert!((voltage / (2.0 * junction::FLUX_QUANTUM_WB * 650.0e9) - 1.0).abs() < 1e-10, "{voltage}");
    let (_, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
}

/// A driven point with no steady mean is refused by name, sealed: the answer
/// is 200 with the outcome refused, and its proof holds.
#[tokio::test]
async fn a_point_that_does_not_settle_is_a_sealed_refusal() {
    let base = spawn_app().await;
    let request = driven(json!([0.0001]), "running", json!(0.0001), json!(500_000_000_000.0));
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "NotSettled");
    let detail = body["reject_detail"].as_str().unwrap();
    assert!(detail.starts_with("Solver Refusal: junction_nist_b7_s2 did not return a current-voltage characteristic (NotSettled)."), "{detail}");
    assert!(body["result"].is_null());
    let (_, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
}

/// Each limit the question shows is 422 with its name and no seal.
#[tokio::test]
async fn a_question_past_a_limit_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let gap = junction::GAP_FREQUENCY_HZ;
    let cases = [
        (held(json!([]), "rest"), "NoBiasPoints"),
        (held(json!(vec![0.0001; 37]), "rest"), "StepsBeyondSolveBudget"),
        (held(json!([0.0001, 0.033]), "rest"), "StepsBeyondSolveBudget"),
        (held(json!([0.034]), "running"), "StepsBeyondSolveBudget"),
        (driven(json!([0.0001]), "rest", json!(0.0001), json!(gap)), "DriveAtOrAboveGapFrequency"),
        (driven(json!([0.0001]), "rest", json!(0.0001), json!(1.0e12)), "DriveAtOrAboveGapFrequency"),
        (driven(json!([0.0001]), "rest", json!(0.0001), json!(0.0)), "DriveFrequencyNotPositive"),
        (driven(json!([0.0001]), "rest", json!(0.0001), json!(-1.0e9)), "DriveFrequencyNotPositive"),
        (driven(json!([0.0001]), "rest", json!(0.0001), json!(1.0e8)), "StepsBeyondSolveBudget"),
        (driven(json!([0.0001]), "rest", json!(0.05), json!(1.0e11)), "StepsBeyondSolveBudget"),
        (held(json!([0.0001]), "sideways"), "UnknownStart"),
        (json!({ "solver": "junction_nist_b7_s2", "parameters": { "bias_current_a": [0.0001], "start": "rest", "drive": "laser" } }), "UnknownDrive"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name}).")), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    let (_, text) = post(&base, "/solve", &driven(json!([0.0001]), "rest", json!(0.0001), json!(gap))).await;
    assert!(text.contains("a photon of the drive breaks a Cooper pair"), "{text}");
    // A drive of zero amplitude has no photon: at the gap frequency and above it is answered.
    for frequency in [gap, 6.92e11] {
        let (status, text) = post(&base, "/solve", &driven(json!([0.0002]), "running", json!(0.0), json!(frequency))).await;
        assert_eq!(status, 200, "{frequency} {text}");
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["outcome"], "converged", "{text}");
    }
    // The published bias bound is the budget's own edge: one past it is refused, by name, with its count.
    let (status, text) = post(&base, "/solve", &held(json!([junction::CURRENT_REACH_MAX_A.next_up()]), "rest")).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("more than the budget of 120000000 one solve computes, a budget of computing and not a limit of the physics"), "{text}");
    let (_, text) = post(&base, "/solve", &held(json!([1.0e308]), "rest")).await;
    assert!(text.contains("the question needs more steps than a 64-bit number holds") && !text.contains("18446744073709551615"), "{text}");
    // Above the critical current no state at rest exists: a point creeping
    // toward its switch is refused, sealed, not settled at zero.
    let (status, text) = post(&base, "/solve", &held(json!([junction::CRITICAL_CURRENT_A * (1.0 + 1.0e-12)]), "rest")).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!((body["outcome"].clone(), body["reject"].clone()), (json!("refused"), json!("NotSettled")), "{text}");
    let (_, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true, "{text}");
    // The number just below the gap frequency is answered.
    let below = f64::from_bits(gap.to_bits() - 1);
    let (status, text) = post(&base, "/solve", &driven(json!([0.0001]), "rest", json!(0.00001), json!(below))).await;
    assert_eq!(status, 200, "{text}");
    // A key of the drive with no drive, and a drive with a key missing, are named.
    let (status, text) = post(&base, "/solve", &json!({ "solver": "junction_nist_b7_s2", "parameters": { "bias_current_a": [0.0001], "start": "rest", "drive": "microwave", "drive_current_amplitude_a": 0.0001 } })).await;
    assert_eq!(status, 422);
    assert!(text.contains("drive_frequency_hz"), "{text}");
    let (status, text) = post(&base, "/solve", &json!({ "solver": "junction_nist_b7_s2", "parameters": { "bias_current_a": [0.0001], "start": "rest", "drive": "none", "drive_frequency_hz": 1.0e11 } })).await;
    assert_eq!(status, 422);
    assert!(text.contains("drive_frequency_hz"), "{text}");
    let (status, text) = post(&base, "/solve", &json!({ "solver": "junction_nist_b7_s2", "parameters": { "bias_current_a": [0.0001], "start": "rest", "drive": "none", "capacitance_f": 1e-13 } })).await;
    assert_eq!(status, 422);
    assert!(text.starts_with("Physical Violation: 'capacitance_f' is not an input of this solver."), "{text}");
}

/// Where the card drafted on 2026-10-06 would have stopped, past the top of
/// the table and on the negative side, the solver answers, flagged, with its
/// ledger closed.
#[tokio::test]
async fn past_the_compared_range_it_answers_and_its_ledger_closes() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/solve", &held(json!([0.0012, -0.0004]), "rest")).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["result"]["normal_line_reached"], json!([1.0, 0.0]));
    assert!(body["result"]["mean_voltage_v"][1].as_f64().unwrap() < -0.0028);
    assert!(body["invariants"]["largest_energy_residual"].as_f64().unwrap() < junction::RESIDUAL_CONSERVATION_LIMIT);
}

#[tokio::test]
async fn the_description_carries_the_standing_and_its_examples_run_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 25);
    assert_eq!(solvers[15]["solver"], "junction_nist_b7_s2");
    assert_eq!(solvers[15]["against_measurement"], "compared with measurement");
    let summary = solvers[15]["summary"].as_str().expect("a sentence");
    assert!(summary.contains("within 11.84 percent of I_c R_n RMS"), "{summary}");
    assert!(summary.contains("0 to 0.91847 mA") && summary.contains("up to 19.5 percent high"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "junction_nist_b7_s2" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["summary"], summary);
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.contains("27ad14413a30eed6e939afd1e1603ddaf4fb3d829d6823b8f5887a0f03e6238b"), "{pass_line}");
    let result = described["validation"]["result"].as_str().unwrap();
    for words in ["11.84 percent of I_c R_n (target 5)", "rising sweep 7.10, falling 15.13", "worst -36.3 at 0.1113 mA falling", "10.5 to 16.2 percent"] {
        assert!(result.contains(words), "{words}: {result}");
    }
    assert_eq!(described["inputs"].as_array().expect("inputs").len(), 5);
    assert_eq!(described["refusals"].as_array().expect("refusals").len(), 11);
    assert_eq!(described["locked"]["critical_current_a"], 2.98e-4);
    assert_eq!(described["locked"]["table_knots"], 199);

    for key in ["example_solve", "example_solve_shapiro_step"] {
        let example = described[key].clone();
        let (status, text) = post(&base, "/solve", &example).await;
        assert_eq!(status, 200, "{key} {example} {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{key}");
    }
}
