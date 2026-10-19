//! `water_hammer_oil_line` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door and sealed, its
//! description and its worked examples.

mod answer_text;

use std::time::Duration;

use genesis_solvers::water_hammer_oil_line::{self as hammer, HammerQuestion};
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

fn hammer_request(q: &HammerQuestion, histories: bool) -> Value {
    json!({ "solver": "water_hammer_oil_line", "parameters": {
        "supply_pressure_pa": q.supply_pressure_pa,
        "return_pressure_pa": q.return_pressure_pa,
        "initial_velocity_m_s": q.initial_velocity_m_s,
        "closure_time_s": q.closure_time_s,
        "free_air_fraction": q.free_air_fraction,
        "duration_s": q.duration_s,
        "return_histories": histories
    } })
}

fn question(supply: f64, velocity: f64, closure: f64, air: f64, duration: f64) -> HammerQuestion {
    HammerQuestion {
        supply_pressure_pa: supply,
        return_pressure_pa: 0.2e6,
        initial_velocity_m_s: velocity,
        closure_time_s: closure,
        free_air_fraction: air,
        duration_s: duration,
    }
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

/// A separating column with its histories, bit for bit as the solver gives
/// it, with its field hash and its proof.
#[tokio::test]
async fn a_transient_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let q = question(1.0e6, 1.5, 0.0, 0.0, 0.2);
    let request = hammer_request(&q, true);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = hammer::solve(&q).expect("the line runs");
    assert_eq!(body["inputs"], request);
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["iterations"], direct.steps);
    let result = &body["result"];
    assert_number(result, "liquid_wave_speed_m_s", direct.liquid_wave_speed_m_s);
    assert_number(result, "joukowsky_rise_liquid_pa", direct.joukowsky_rise_liquid_pa);
    assert_number(result, "valve_pressure_first_rise_pa", direct.valve_pressure_first_rise_pa);
    assert_number(result, "valve_pressure_max_pa", direct.valve_pressure_max_pa);
    assert_number(result, "valve_pressure_min_pa", direct.valve_pressure_min_pa);
    assert_number(result, "cavity_loss_j", direct.cavity_loss_j);
    assert_numbers(result, "first_vapour_time_s", &direct.first_vapour_time_s);
    assert_numbers(result, "pressure_max_along_line_pa", &direct.pressure_max_along_line_pa);
    assert_numbers(result, "valve_pressure_history_pa", &direct.valve_pressure_history_pa);
    assert_numbers(result, "friction_loss_history_j", &direct.friction_loss_history_j);
    assert_eq!(result.as_object().unwrap().len(), 2 * 39, "39 values and their hex");
    // The column separated at the valve and the vapour pressure was held there.
    assert_eq!(result["valve_pressure_min_pa"], 10.0);
    assert_eq!(result["first_vapour_position_m"][0], 4.0);
    let invariants = &body["invariants"];
    assert_number(invariants, "largest_energy_ledger_residual", direct.largest_energy_ledger_residual);
    assert_number(invariants, "largest_volume_ledger_residual", direct.largest_volume_ledger_residual);
    assert!(direct.largest_energy_ledger_residual < 1e-11);

    let mut hasher = Sha256::new();
    hasher.update((direct.field.len() as u64).to_le_bytes());
    for value in &direct.field {
        hasher.update(value.to_le_bytes());
    }
    let served = body["field_sha256"].as_str().expect("field hash");
    assert_eq!(served, hex::encode(hasher.finalize()));

    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
    for (section, key) in [("result", "valve_pressure_max_pa_hex"), ("result", "cavity_loss_j_hex"), ("invariants", "largest_energy_ledger_residual_hex")] {
        let mut edited = body.clone();
        edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
        let (_, text) = post(&base, "/verify_solution", &edited).await;
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }
    let mut edited = body.clone();
    edited["result"]["valve_pressure_history_pa_hex"][7] = json!(flip_hex(body["result"]["valve_pressure_history_pa_hex"][7].as_str().unwrap()));
    let (_, text) = post(&base, "/verify_solution", &edited).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(text_of(&body), text_of(&serde_json::from_str::<Value>(&again).unwrap()));

    // Without the histories, the rest is the same bits and the histories are absent.
    let (_, text) = post(&base, "/solve", &hammer_request(&q, false)).await;
    let short: Value = serde_json::from_str(&text).unwrap();
    assert!(short["result"].get("valve_pressure_history_pa").is_none());
    assert_eq!(short["result"]["valve_pressure_max_pa_hex"], body["result"]["valve_pressure_max_pa_hex"]);
    assert_eq!(short["field_sha256"], body["field_sha256"]);
}

fn text_of(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

#[tokio::test]
async fn a_question_past_a_limit_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let base_q = question(10.0e6, 2.0, 0.001, 0.0, 0.05);
    let with = |change: fn(&mut HammerQuestion)| {
        let mut q = base_q;
        change(&mut q);
        hammer_request(&q, false)
    };
    let cases = [
        (with(|q| q.supply_pressure_pa = 51.6e6), "SupplyAboveWallYield"),
        (with(|q| q.return_pressure_pa = 10.0), "ReturnNotAboveVapour"),
        (with(|q| q.return_pressure_pa = 10.0e6), "ReturnNotBelowSupply"),
        (with(|q| q.initial_velocity_m_s = -0.5), "VelocityNegative"),
        (with(|q| q.duration_s = 1.5), "DurationOutOfRange"),
        (with(|q| q.closure_time_s = 0.06), "ClosureTimeOutOfRange"),
        (with(|q| q.free_air_fraction = -0.1), "FreeAirNegative"),
        (with(|q| { q.free_air_fraction = 0.01; q.closure_time_s = 0.0005 }), "ClosureFasterThanGrid"),
        (with(|q| { q.supply_pressure_pa = 1.0e6; q.return_pressure_pa = 0.99e6 }), "SupplyCannotDriveFlow"),
        (with(|q| { q.supply_pressure_pa = 0.3e6; q.free_air_fraction = 1.0; q.initial_velocity_m_s = 0.1 }), "FreeAirAboveBubblyFlow"),
        (with(|q| { q.supply_pressure_pa = 0.25e6; q.return_pressure_pa = 0.11e6; q.free_air_fraction = 0.36; q.initial_velocity_m_s = 3.0 }), "InitialFlowMachAboveAcoustic"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name})")), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    let (status, text) = post(&base, "/solve", &with(|q| q.supply_pressure_pa = 52.0e6)).await;
    assert_eq!(status, 422);
    assert_eq!(
        text,
        "Physical Violation: supply pressure 52000000 Pa is above 51507575 Pa, where the tube's bore first yields (SupplyAboveWallYield)."
    );
    let mut extra = with(|_| {});
    extra["parameters"]["bore_m"] = json!(0.01);
    let (status, text) = post(&base, "/solve", &extra).await;
    assert_eq!(status, 422);
    assert_eq!(
        text,
        "Physical Violation: 'bore_m' is not an input of this solver. The tube, the oil, the grid of forty sections, the friction law and the tolerances stay inside the solver."
    );
}

/// A wave that would yield the tube is answered as a sealed refusal at the
/// time and place it reached the yield.
#[tokio::test]
async fn a_rise_to_the_tubes_yield_is_a_sealed_refusal() {
    let base = spawn_app().await;
    let q = question(45.0e6, 6.0, 0.0, 0.0, 0.05);
    let (status, text) = post(&base, "/solve", &hammer_request(&q, false)).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "WallYield");
    let direct = hammer::solve(&q).unwrap_err();
    assert_eq!(body["reject_detail"], format!("Solver Refusal: water_hammer_oil_line did not return a transient (WallYield). {direct}."));
    assert!(body["result"].is_null() && body["invariants"].is_null() && body["field_sha256"].is_null());
    let (_, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
}

#[tokio::test]
async fn the_description_carries_the_standing_and_its_examples_run_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let row = listed["solvers"].as_array().unwrap().iter().find(|row| row["solver"] == "water_hammer_oil_line").expect("listed").clone();
    assert_eq!(row["against_measurement"], "company specimen");
    let summary = row["summary"].as_str().unwrap();
    assert!(summary.starts_with("Company specimen: "), "{summary}");
    assert!(summary.contains("within 0.28 percent"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "water_hammer_oil_line" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["validation"]["result"], "Company specimen: its constants are the company's, stated in its description, and every answer holds the conservation ledgers stated under invariants to their tolerances; what its method is held to, and within what, is under reference.");
    let covers = described["reference"]["covers"].as_str().unwrap();
    assert!(covers.contains("0.13 percent long, 0.11 short, 0.26 long and 0.28 long"), "{covers}");
    let dlr = described["reference"]["second_covers"].as_str().unwrap();
    assert!(dlr.contains("6 of 9 within 5 percent, all low, by 0.4 to 6.9 percent"), "{dlr}");
    assert!(dlr.contains("within 5 percent in all 9 tests"), "{dlr}");
    assert_eq!(described["inputs"].as_array().unwrap().len(), 7);
    assert_eq!(described["inputs"][0]["maximum"], hammer::WALL_YIELD_PRESSURE_PA);
    assert_eq!(described["locked"]["sections"], 40);
    for key in ["example_solve", "example_solve_column_separation", "example_solve_aerated"] {
        let example = described[key].clone();
        let (status, text) = post(&base, "/solve", &example).await;
        assert_eq!(status, 200, "{key}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{key}");
    }
    let (_, text) = post(&base, "/solve", &described["example_solve_aerated"]).await;
    let aerated: Value = serde_json::from_str(&text).unwrap();
    // The first rise follows the chord of the gas law, above Wood's tangent.
    let rise = aerated["result"]["valve_pressure_first_rise_pa"].as_f64().unwrap();
    assert!(rise > aerated["result"]["joukowsky_rise_wood_pa"].as_f64().unwrap());
    assert!(rise < aerated["result"]["joukowsky_rise_liquid_pa"].as_f64().unwrap());
}
