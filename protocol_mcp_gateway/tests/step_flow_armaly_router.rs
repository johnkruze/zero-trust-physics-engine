//! `step_flow_armaly` through `POST /solve`, against a direct call to
//! `genesis_solvers`, with its refusals at the door and its description.

mod answer_text;

use std::time::Duration;

use genesis_solvers::step_flow_armaly::{self as step, StepQuestion};
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

fn flow(reynolds_number: f64, station_x_over_s: f64) -> Value {
    json!({ "solver": "step_flow_armaly", "parameters": { "reynolds_number": reynolds_number, "station_x_over_s": station_x_over_s } })
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

/// The opened condition, Reynolds number 100, at a measured station. The
/// upper end of the range takes three times the work and is run in the
/// solver's own release tests and in the scored run.
#[tokio::test]
async fn the_opened_condition_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let request = flow(100.0, 3.06);
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = step::solve(&StepQuestion {
        reynolds_number: 100.0,
        station_x_over_s: 3.06,
    })
    .expect("the solver settles");

    assert_eq!(body["inputs"], request);
    assert_eq!(body["engine_version"], format!("genesis_solvers_v{}", genesis_solvers::VERSION));
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert_eq!(body["iterations"], direct.newton_iterations);
    let result = &body["result"];
    assert_number(result, "reattachment_x_over_s", direct.reattachment_x_over_s);
    assert_number(result, "reattachment_mm", direct.reattachment_mm);
    assert_numbers(result, "floor_shear_sign_changes_x_over_s", &direct.floor_shear_sign_changes_x_over_s);
    assert_numbers(result, "upper_wall_shear_sign_changes_x_over_s", &direct.upper_wall_shear_sign_changes_x_over_s);
    assert!(direct.upper_wall_shear_sign_changes_x_over_s.is_empty());
    assert_numbers(result, "profile_y_over_h", &direct.profile_y_over_h);
    assert_numbers(result, "profile_u_over_umax", &direct.profile_u_over_umax);
    assert_eq!(direct.profile_u_over_umax.len(), 51);
    assert_eq!(result.as_object().unwrap().len(), 12, "six values and their hex");
    let invariants = &body["invariants"];
    assert_number(invariants, "largest_imbalance", direct.largest_imbalance);
    assert_number(invariants, "outlet_departure", direct.outlet_departure);
    assert!(direct.largest_imbalance <= step::IMBALANCE_TOLERANCE);

    // The field hash is the count of numbers, the stream function and the vorticity.
    let mut hasher = Sha256::new();
    hasher.update((direct.stream_function.len() as u64 * 2).to_le_bytes());
    for value in direct.stream_function.iter().chain(&direct.vorticity) {
        hasher.update(value.to_le_bytes());
    }
    let own = hex::encode(hasher.finalize());
    let served = body["field_sha256"].as_str().expect("field hash");
    assert_eq!(served, own);

    // The proof holds, and an edit of a number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    for (section, key) in [("result", "reattachment_x_over_s_hex"), ("result", "reattachment_mm_hex"), ("invariants", "largest_imbalance_hex"), ("invariants", "outlet_departure_hex")] {
        let mut edited = body.clone();
        edited[section][key] = json!(flip_hex(body[section][key].as_str().unwrap()));
        let (status, text) = post(&base, "/verify_solution", &edited).await;
        assert_eq!(status, 200, "{text}");
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{key}");
        assert_eq!(report["cause"], section, "{key}");
    }
    let mut edited = body.clone();
    edited["result"]["profile_u_over_umax_hex"][30] = json!(flip_hex(body["result"]["profile_u_over_umax_hex"][30].as_str().unwrap()));
    let (_, text) = post(&base, "/verify_solution", &edited).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);
    let mut field = body.clone();
    field["field_sha256"] = json!(flip_hex(served));
    let (_, text) = post(&base, "/verify_solution", &field).await;
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap()["holds"], false);

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    assert_eq!(text_of(&body), text_of(&serde_json::from_str::<Value>(&again).unwrap()));
}

fn text_of(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

#[tokio::test]
async fn a_question_outside_the_range_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let cases = [
        (flow(0.0, 1.0), "ReynoldsNotPositive"),
        (flow(-100.0, 1.0), "ReynoldsNotPositive"),
        (flow(400.00000000000006, 1.0), "ReynoldsOutOfRange"),
        (flow(401.0, 1.0), "ReynoldsOutOfRange"),
        (flow(1.0e9, 1.0), "ReynoldsOutOfRange"),
        (flow(100.0, -0.001), "StationOutOfRange"),
        (flow(100.0, 30.000000000000004), "StationOutOfRange"),
        (flow(389.0, 30.000000000000004), "StationOutOfRange"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.ends_with(&format!("({name}).")), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
        assert!(!text.contains("..="), "{text}");
    }
    // The exact lower end of the Reynolds range and both exact ends of the
    // station range are answered, the two stations on one flow.
    let (status, first) = post(&base, "/solve", &flow(100.0, 0.0)).await;
    assert_eq!(status, 200, "{first}");
    let (status, last) = post(&base, "/solve", &flow(100.0, 30.0)).await;
    assert_eq!(status, 200, "{last}");
    let first: Value = serde_json::from_str(&first).unwrap();
    let last: Value = serde_json::from_str(&last).unwrap();
    assert_eq!(first["outcome"], "converged");
    assert_eq!(last["outcome"], "converged");
    assert_eq!(first["field_sha256"], last["field_sha256"]);
    assert_eq!(first["result"]["reattachment_x_over_s_hex"], last["result"]["reattachment_x_over_s_hex"]);
    assert_ne!(first["result"]["profile_u_over_umax_hex"], last["result"]["profile_u_over_umax_hex"]);
    let others = [
        (
            json!({ "solver": "step_flow_armaly", "parameters": { "reynolds_number": 100.0 } }),
            "Missing required parameter: 'station_x_over_s' (InputMissing).".to_string(),
        ),
        (
            json!({ "solver": "step_flow_armaly", "parameters": { "station_x_over_s": 1.0 } }),
            "Missing required parameter: 'reynolds_number' (InputMissing).".to_string(),
        ),
        (
            json!({ "solver": "step_flow_armaly", "parameters": { "reynolds_number": 100.0, "station_x_over_s": 1.0, "step_height_mm": 10.0 } }),
            "Physical Violation: 'step_height_mm' is not an input of this solver. The geometry of the rig, the grid, the ladder of Reynolds numbers and the tolerances stay inside the solver.".to_string(),
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
        &json!({ "solver": "step_flow_armaly", "parameters": { "reynolds_number": "100", "station_x_over_s": 1.0 } }),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(!text.contains("solution_seal"), "{text}");
}

/// Above Reynolds number 100 the last station compared is 13.57, and below
/// 100 nothing was compared. Where engine 0.8.0 refused by name, the flow is
/// answered with a seal that holds: a station past 13.57 at 389, and
/// Reynolds numbers below 100 down to creeping flow. Where the version before
/// refused, from 389 to 400, the top of two-dimensional flow over this step,
/// it is answered the same way.
#[tokio::test]
async fn past_the_compared_stations_and_below_reynolds_100_the_flow_is_answered() {
    let base = spawn_app().await;
    let (status, edge) = post(&base, "/solve", &flow(389.0, 13.57)).await;
    assert_eq!(status, 200, "{edge}");
    let edge: Value = serde_json::from_str(&edge).unwrap();
    assert_eq!(edge["outcome"], "converged");
    assert!(edge["solution_seal"].is_string());
    for request in [flow(389.0, 13.570000000000002), flow(389.0, 30.0), flow(99.99999999999999, 30.0), flow(10.0, 1.0), flow(0.01, 20.0), flow(400.0, 13.57)] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{request} {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{request} {text}");
        let reynolds = request["parameters"]["reynolds_number"].as_f64().unwrap();
        assert!(body["invariants"]["largest_imbalance"].as_f64().unwrap() <= step::IMBALANCE_TOLERANCE * step::viscous_scale(reynolds));
        assert!(body["invariants"]["outlet_departure"].as_f64().unwrap() <= step::OUTLET_TOLERANCE);
        let (status, checked) = post(&base, "/verify_solution", &body).await;
        assert_eq!(status, 200, "{checked}");
        assert_eq!(serde_json::from_str::<Value>(&checked).unwrap()["holds"], true, "{checked}");
        if request["parameters"]["reynolds_number"] == 389.0 {
            assert_eq!(body["field_sha256"], edge["field_sha256"]);
        }
    }
    for station in [12.04, 16.33, 20.92, 30.0] {
        let (status, text) = post(&base, "/solve", &flow(100.0, station)).await;
        assert_eq!(status, 200, "{station} {text}");
    }
}

#[tokio::test]
async fn the_description_carries_the_standing_and_its_example_runs_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 25);
    assert_eq!(solvers[9]["solver"], "step_flow_armaly");
    let summary = solvers[9]["summary"].as_str().expect("a sentence");
    assert!(summary.contains("Nothing fitted"), "{summary}");
    assert!(summary.contains("Reynolds number above 0 to 400"), "{summary}");

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "step_flow_armaly" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["summary"], summary);
    for part in ["object", "data", "pass_line", "result", "limits"] {
        let words = described["validation"][part].as_str().unwrap_or_else(|| panic!("{part}"));
        assert!(!words.is_empty(), "{part}");
    }
    let data = described["validation"]["data"].as_str().unwrap();
    for words in [
        "Journal of Fluid Mechanics 127, 473-496, doi 10.1017/S0022112083002839",
        "Re = VD/ν, where V is two-thirds of the measured maximum inlet velocity",
        "the data's own floor is about 0.03",
        "7 are false and 20 were not found",
        "NASA Technical Memorandum 106248",
        "second-hand and digitised, two steps from the instrument",
        "Compared outside it: Figure 4, the whole condition at Reynolds number 389",
    ] {
        assert!(data.contains(words), "{words}: {data}");
    }
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.contains("dd3abc9bbc833852f0a53c3201994edaf23a2bd45edccf3ae3016136c3afce1a"), "{pass_line}");
    assert!(pass_line.contains("The two reattachment lengths were read before the solver was built; their target is 10 percent"), "{pass_line}");
    let limits = described["validation"]["limits"].as_str().unwrap();
    assert!(limits.contains("No measured velocity profile between 100 and 389 was compared"), "{limits}");
    assert!(limits.contains("the reattachment length is compared there with Armaly's Figure 14 at 149 and 298, read after the solver was built"), "{limits}");
    assert!(!limits.contains("No measurement between"), "{limits}");
    assert!(limits.contains("Between 7.76 and 12.04 at 100 no circle lies"), "{limits}");
    assert!(!limits.contains("relaxes toward"), "{limits}");
    assert!(summary.contains("at 100, on 80 circles of Armaly's Figure 15 at 12.04 to 30.31 step heights, within 0.019 RMS"), "{summary}");
    let station_bound = described["inputs"][1]["bound"].as_str().unwrap();
    for words in [
        "at Reynolds number 100 at stations from 0 to 7.76",
        "from 12.04 to 30.31 (Armaly's Figure 15",
        "at Reynolds number 389 at stations from 0 to 13.57",
        "Out to 30, where the grid ends, is answered at every Reynolds number",
    ] {
        assert!(station_bound.contains(words), "{words}: {station_bound}");
    }
    assert!(!station_bound.contains("The open measurements are at stations from 0 to 13.57"), "{station_bound}");
    assert!(limits.contains("closes for any viscosity"), "{limits}");
    assert!(summary.contains("within 0.0437 of the largest inlet speed RMS (target 0.05), worst station 0.052 (target 0.08)"), "{summary}");
    assert!(summary.contains("the reattachment length reads 6 to 7 percent short of Armaly's plot at 389 (8.07 against about 8.6 step heights) and 5.8 percent short at 97"), "{summary}");
    assert!(summary.ends_with("(StationOutOfRange)."), "{summary}");
    assert!(limits.contains("1.3 to 1.5 percent at Reynolds number 100 and under 0.6 percent from 200 up"), "{limits}");
    assert!(limits.contains("from about 350 to 445"), "{limits}");
    assert_eq!(described["reference"]["tolerance_fraction"], 0.02);
    assert_eq!(described["reference"]["test"], "erturk_2008_table_1_reattachment");
    let result = described["validation"]["result"].as_str().unwrap();
    for words in [
        "On the 174 circles of Figure 4 at Reynolds number 389",
        "RMS difference 0.0437 of the largest inlet speed (target 0.05)",
        "the solver's 0.043 is above the floor",
        "where Armaly's own plot has about 8.6",
        "3.01 step heights at Reynolds number 97, 4.00 at 149, 6.60 at 298 and 8.60 at 387",
        "own digitisation of 187 circles gives 0.0430, 0.0514 and 0.238",
        "the solver at 259.33 (the other reading of the memorandum's Reynolds number) 0.088",
        "2.83 step heights against a quoted approximately 3.3, 14 percent low (target 10)",
        "SHA-256 042bb92d94119f5d684998d7d7ff9b88134188dda06baa6e2aea8be7884bd616",
        "on 80 circles of Armaly's Figure 15 at 12.04, 16.33, 20.92 and 30.31 step heights",
        "the parabola itself is 0.019 from the circles and the solver at 259.33 is 0.023",
        "the third measure taken against the solver at Reynolds number 389",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    assert_eq!(described["locked"]["step_height_mm"], step::STEP_HEIGHT_MM);
    assert_eq!(described["locked"]["inlet_height_mm"], step::INLET_HEIGHT_MM);
    assert!(described["reference"]["kind"].as_str().unwrap().starts_with("A reference calculation of the same two-dimensional equations"));
    assert!(described["convergence"]["run_time"].as_str().unwrap().contains("Reynolds number 400, takes 2.2 s on one core"));
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0]["minimum"], step::REYNOLDS_ABOVE);
    assert_eq!(inputs[0]["minimum_exclusive"], true);
    assert_eq!(inputs[0]["maximum"], step::REYNOLDS_MAX);
    assert_eq!(inputs[1]["minimum"], step::STATION_MIN_X_OVER_S);
    assert_eq!(inputs[1]["maximum"], step::STATION_MAX_X_OVER_S);
    assert_eq!(described["refusals"].as_array().expect("refusals").len(), 9);
    for refusal in described["refusals"].as_array().unwrap() {
        assert!(!refusal["meaning"].as_str().unwrap().contains("..="), "{refusal}");
    }

    let example = described["example_solve"].clone();
    let (status, text) = post(&base, "/solve", &example).await;
    assert_eq!(status, 200, "{example} {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "converged");
    let length = body["result"]["reattachment_x_over_s"].as_f64().unwrap();
    assert!(length > 2.8 && length < 2.9, "{length}");
}
