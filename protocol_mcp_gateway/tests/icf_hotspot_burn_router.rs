//! `icf_hotspot_burn` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door, a sealed refusal, its
//! proof attacked and its description.

mod answer_text;

use std::time::Duration;

use genesis_solvers::icf_hotspot_burn::{self as burn, HotSpotQuestion};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha384};
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

fn hot_spot(t: f64, p: f64, r: f64, rho_r: f64, m: f64) -> Value {
    tamped_spot(t, p, r, rho_r, m, 0.0)
}

fn tamped_spot(t: f64, p: f64, r: f64, rho_r: f64, m: f64, ablator: f64) -> Value {
    json!({ "solver": "icf_hotspot_burn", "parameters": {
        "hot_spot_temperature_kev": t, "hot_spot_pressure_gbar": p, "hot_spot_radius_um": r,
        "shell_areal_density_g_per_cm2": rho_r, "fuel_mass_mg": m, "ablator_mass_mg": ablator } })
}

fn question(t: f64, p: f64, r: f64, rho_r: f64, m: f64) -> HotSpotQuestion {
    HotSpotQuestion {
        hot_spot_temperature_kev: t,
        hot_spot_pressure_gbar: p,
        hot_spot_radius_um: r,
        shell_areal_density_g_per_cm2: rho_r,
        fuel_mass_mg: m,
        ablator_mass_mg: 0.0,
    }
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

fn text_of(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

/// A sub-ignited hot spot, answered bit for bit as the solver gives it, its
/// ledgers inside their tolerances, its proof holding and broken by an edit.
#[tokio::test]
async fn a_burn_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let request = hot_spot(4.0, 150.0, 35.0, 0.8, 0.2);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = burn::solve(&question(4.0, 150.0, 35.0, 0.8, 0.2)).expect("the solver burns");

    assert_eq!(body["inputs"], request);
    assert_eq!(body["engine_version"], format!("genesis_solvers_v{}", genesis_solvers::VERSION));
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert_eq!(body["iterations"], direct.steps);
    let result = &body["result"];
    assert_number(result, "yield_mj", direct.yield_mj);
    assert_number(result, "neutron_yield", direct.neutron_yield);
    assert_number(result, "yield_amplification", direct.yield_amplification);
    assert_number(result, "burn_up_fraction", direct.burn_up_fraction);
    assert_number(result, "burn_width_ps", direct.burn_width_ps);
    assert_number(result, "burn_averaged_temperature_kev", direct.burn_averaged_temperature_kev);
    assert_number(result, "burn_averaged_pressure_gbar", direct.burn_averaged_pressure_gbar);
    assert_number(result, "self_heating_ratio_at_bang", direct.self_heating_ratio_at_bang);
    assert_number(result, "lawson_parameter_no_alpha", direct.lawson_parameter_no_alpha);
    assert_number(result, "lawson_criterion_lindl", direct.lawson_criterion_lindl);
    assert_number(result, "q_alpha_hot_spot", direct.q_alpha_hot_spot);
    assert_number(result, "alpha_energy_mj", direct.alpha_energy_mj);
    assert_number(result, "bremsstrahlung_mj", direct.bremsstrahlung_mj);
    let times = result["history_time_ps"].as_array().expect("history");
    assert_eq!(times.len(), direct.history.len());
    assert!(times.len() <= burn::HISTORY_ROWS_MAX);
    let invariants = &body["invariants"];
    assert_number(invariants, "energy_residual", direct.energy_residual);
    assert_number(invariants, "particle_residual", direct.particle_residual);
    assert!(direct.energy_residual <= burn::ENERGY_TOLERANCE);
    assert!(direct.particle_residual <= burn::PARTICLE_TOLERANCE);
    // The ledger in the published accounts.
    let change = direct.energy_at_end_mj - direct.energy_at_start_mj;
    let net = direct.alpha_energy_mj - direct.bremsstrahlung_mj - direct.alpha_energy_escaped_mj - direct.kinetic_energy_carried_by_products_mj;
    assert!((change - net).abs() <= burn::ENERGY_TOLERANCE * direct.energy_at_start_mj.max(direct.alpha_energy_mj));

    // The field hash is the count of numbers and the history rows.
    let mut hasher = Sha384::new();
    hasher.update((direct.history.len() as u64 * 5).to_le_bytes());
    for row in &direct.history {
        for value in row {
            hasher.update(value.to_le_bytes());
        }
    }
    let own = hex::encode(hasher.finalize());
    let served = body["field_sha384"].as_str().expect("field hash");
    assert_eq!(served, own);

    // The proof holds, and an edit of a number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
    for (section, key) in [("result", "yield_mj_hex"), ("result", "burn_up_fraction_hex"), ("invariants", "energy_residual_hex")] {
        let mut edited = body.clone();
        edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
        let (status, text) = post(&base, "/verify_solution", &edited).await;
        assert_eq!(status, 200, "{text}");
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }
    let mut field = body.clone();
    field["field_sha384"] = json!(flip_hex(served));
    let (_, text) = post(&base, "/verify_solution", &field).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(text_of(&body), text_of(&serde_json::from_str::<Value>(&again).unwrap()));
}

#[tokio::test]
async fn a_question_past_a_limit_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let cases = [
        (hot_spot(0.199, 150.0, 35.0, 0.8, 0.2), "TemperatureOutsideReactivityFit"),
        (hot_spot(100.0000001, 150.0, 35.0, 0.8, 0.2), "TemperatureOutsideReactivityFit"),
        (hot_spot(4.0, 0.0, 35.0, 0.8, 0.2), "PressureNotPositive"),
        (hot_spot(4.0, -5.0, 35.0, 0.8, 0.2), "PressureNotPositive"),
        (hot_spot(4.0, 150.0, 0.0, 0.8, 0.2), "RadiusNotPositive"),
        (hot_spot(4.0, 150.0, 35.0, 0.0, 0.2), "ShellArealDensityNotPositive"),
        (hot_spot(4.0, 150.0, 35.0, 0.8, 0.008), "FuelMassNotAboveHotSpotMass"),
        (hot_spot(4.0, 150.0, 35.0, 0.8, 0.12), "ShellMassBelowArealDensity"),
        (hot_spot(4.0, 150.0, 35.0, 0.05, 0.2), "ShellNotDenserThanHotSpot"),
        (tamped_spot(4.0, 150.0, 35.0, 0.8, 0.2, -0.1), "AblatorMassNegative"),
        // N221204 as PRL 132 065102 Table I prints it, made into a question by the protocol's rule.
        (hot_spot(12.0, 608.0, 67.530_562_465_934_66, 0.077_6, 0.22), "ShellNotDenserThanHotSpot"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(&format!("({name}).")), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    let others = [
        (
            json!({ "solver": "icf_hotspot_burn", "parameters": { "hot_spot_temperature_kev": 4.0, "hot_spot_pressure_gbar": 150.0, "hot_spot_radius_um": 35.0, "shell_areal_density_g_per_cm2": 0.8, "ablator_mass_mg": 0.0 } }),
            "Missing required parameter: 'fuel_mass_mg' (InputMissing).".to_string(),
        ),
        (
            json!({ "solver": "icf_hotspot_burn", "parameters": { "hot_spot_temperature_kev": 4.0, "hot_spot_pressure_gbar": 150.0, "hot_spot_radius_um": 35.0, "shell_areal_density_g_per_cm2": 0.8, "fuel_mass_mg": 0.2, "ablator_mass_mg": 0.0, "tritium_fraction": 0.5 } }),
            format!("Physical Violation: 'tritium_fraction' is not an input of this solver. {}", "The shell zones, the Courant number and its halvings, the start and end of the burn, the tolerances, the deuterium-tritium mix (50:50) and every physical constant stay inside the solver."),
        ),
    ];
    for (request, message) in others {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert_eq!(text, message, "{request}");
    }
}

/// A hot spot that collapses under its own radiation into degenerate
/// matter is refused by name inside the run, with a seal.
#[tokio::test]
async fn a_limit_reached_during_the_burn_is_a_sealed_refusal() {
    let base = spawn_app().await;
    let request = hot_spot(2.0, 400.0, 20.0, 1.5, 0.2);
    let direct = burn::solve(&question(2.0, 400.0, 20.0, 1.5, 0.2));
    assert!(matches!(direct, Err(burn::HotSpotReject::HotSpotDegenerate { .. })), "{direct:?}");
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "HotSpotDegenerate");
    assert!(body["reject_detail"].as_str().unwrap().contains("degenerate"), "{}", body["reject_detail"]);
    assert!(body["solution_seal"].is_string());
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], true);
}

#[tokio::test]
async fn the_description_carries_the_standing_and_its_examples_run_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), protocol_mcp_gateway::SOLVERS);
    assert_eq!(solvers[22]["solver"], "icf_hotspot_burn");
    // Since 0.14.4 a row gives what the model computes, the comparison and
    // the gap; the whole spec sheet is the description's.
    let row_summary = solvers[22]["summary"].as_str().expect("a sentence");
    assert!(!row_summary.contains("Range: "), "{row_summary}");
    let (_, full) = post(&base, "/describe_solver", &json!({ "solver": "icf_hotspot_burn" })).await;
    let full: Value = serde_json::from_str(&full).unwrap();
    let summary = full["summary"].as_str().unwrap().to_string();
    let summary = summary.as_str();
    assert!(summary.starts_with(row_summary.split_once(" Compared with ").map_or(row_summary, |parts| parts.0)), "{summary}");
    assert!(summary.contains("Nothing fitted"), "{summary}");
    assert!(summary.contains("3 of 24 targets reached and no shot within all of its own"), "{summary}");
    assert!(summary.contains("with the stagnated mass as fuel, N221204 yields 2.19 to 2.97 MJ against 3.1"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "icf_hotspot_burn" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["summary"], summary);
    for part in ["object", "data", "pass_line", "result", "limits"] {
        let words = described["validation"][part].as_str().unwrap_or_else(|| panic!("{part}"));
        assert!(!words.is_empty(), "{part}");
        assert!(!words.to_lowercase().contains("check"), "{part}");
    }
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.contains("Record SHA-256 a3ef39766b8a527947f4b8c37d3cb2563849a9da3abdabc40fe2dd4b805d8553."), "{pass_line}");
    assert!(pass_line.contains("Y, the yield within a factor of 2 of the measured; W, the burn width within 30 percent of the measured"), "{pass_line}");
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 6);
    assert_eq!(inputs[0]["minimum"], burn::TEMPERATURE_MIN_KEV);
    assert_eq!(inputs[0]["maximum"], burn::TEMPERATURE_MAX_KEV);
    assert_eq!(described["refusals"].as_array().expect("refusals").len(), 20);
    assert_eq!(described["locked"]["shell_zones"], burn::SHELL_ZONES);

    for key in ["example_solve", "example_solve_igniting", "example_solve_n221204_stagnated_255", "example_solve_n221204_stagnated_510"] {
        let example = described[key].clone();
        let (status, text) = post(&base, "/solve", &example).await;
        assert_eq!(status, 200, "{example} {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{key}");
    }

    // The registered question of N221204 is the worked refusal: turned away by
    // name before computing, with the sentence the description gives.
    let refused = &described["example_refused"];
    let (status, text) = post(&base, "/solve", &refused["solve"]).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, refused["answered"].as_str().unwrap());
    assert!(text.ends_with("(ShellNotDenserThanHotSpot)."), "{text}");
    assert_eq!(refused["solve"]["parameters"]["fuel_mass_mg"], 0.22);
    // The consistent reading beside it, after the fact: the same question but
    // the fuel mass, the stagnated mass at 255 and 510 g/cm^3. The yields
    // are genesis_kernel::math's (since genesis_solvers 0.12.0), within
    // 5e-12 of the record's 2.1861092098299166 and 2.9724751621843315 MJ.
    for (key, yield_mj) in [("example_solve_n221204_stagnated_255", 2.1861092098257755), ("example_solve_n221204_stagnated_510", 2.9724751621704697)] {
        let mut same = described[key]["parameters"].clone();
        same["fuel_mass_mg"] = json!(0.22);
        assert_eq!(same, refused["solve"]["parameters"], "{key}");
        let (_, text) = post(&base, "/solve", &described[key]).await;
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["result"]["yield_mj"], yield_mj, "{key}");
        assert_eq!(body["result"]["burned_through"], 1.0, "{key}");
    }
    let covers = described["reference"]["covers"].as_str().unwrap();
    for words in ["Compared once as the record sets it: no shot within all its targets, 3 of 24 targets reached.", "the worked refusal, example_refused", "With no target, the consistent reading an independent reviewer proposed", "at 255 or 510 g/cm^3", "yield 2.19 and 2.97 MJ against 3.1"] {
        assert!(covers.contains(words), "{words}: {covers}");
    }
}
