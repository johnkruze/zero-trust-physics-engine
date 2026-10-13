//! `POST /validate_series` and the MCP tool of the same name.
//!
//! The answer holds the error of a run against a caller's measurements, in
//! numbers, sealed with a hash of the measurements, and no verdict.

use std::path::Path;
use std::time::Duration;

use protocol_mcp_gateway::app;
use reqwest::Client;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
use serde_json::{Map, Value, json};
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

async fn post_text(base: &str, path: &str, body: &str) -> (u16, String) {
    let client = Client::new();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
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

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    post_text(base, path, &serde_json::to_string(body).unwrap()).await
}

/// An answer, parsed, with its text.
async fn validate(base: &str, request: &Value) -> (Value, String) {
    let (status, text) = post(base, "/validate_series", request).await;
    assert_eq!(status, 200, "{text}");
    (serde_json::from_str(&text).unwrap(), text)
}

async fn refused(base: &str, request: &Value) -> String {
    let (status, text) = post(base, "/validate_series", request).await;
    assert_eq!(status, 422, "{text}");
    text
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn bits(text: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(text, 16).unwrap())
}

fn thermal_run(steps: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{ "steps": steps, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
        "record_every": 1
    })
}

fn request(run: &Value, quantity: &str, measurements: Value) -> Value {
    json!({ "run": run, "quantity": quantity, "measurements": measurements })
}

fn model_values(answer: &Value) -> Vec<f64> {
    answer["model_values_hex"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hex| bits(hex.as_str().unwrap()))
        .collect()
}

/// Numbers that are written in a state with a `_hex` sibling, with the width of each.
fn numeric_fields(state: &Value) -> Vec<(String, bool)> {
    let object = state.as_object().unwrap();
    object
        .iter()
        .filter(|(key, _)| !key.ends_with("_hex") && object.contains_key(&format!("{key}_hex")))
        .map(|(key, _)| (key.clone(), object[&format!("{key}_hex")].is_array()))
        .collect()
}

// V1
#[tokio::test]
async fn the_run_inside_carries_the_run_seal_of_the_same_run_alone_on_all_thirty_cards() {
    let base = spawn_app().await;
    let (status, listed) = {
        let client = Client::new();
        let response = client.get(format!("{base}/cards")).send().await.unwrap();
        (response.status().as_u16(), response.text().await.unwrap())
    };
    assert_eq!(status, 200);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 31);
    let mut quantities_checked = 0;
    for pair in cards {
        let (domain, card) = (pair["domain"].as_str().unwrap(), pair["card"].as_str().unwrap());
        let (status, described) =
            post(&base, "/describe_card", &json!({ "domain": domain, "card": card })).await;
        assert_eq!(status, 200, "{described}");
        let described: Value = serde_json::from_str(&described).unwrap();
        let run = described["example_run_trajectory"].clone();
        let (status, alone) = post(&base, "/run_trajectory", &run).await;
        assert_eq!(status, 200, "{card}: {alone}");
        let alone: Value = serde_json::from_str(&alone).unwrap();
        let landed = alone["steps_landed"].as_u64().unwrap();
        assert!(landed >= 1, "{card}");
        let fields = numeric_fields(&alone["final_state"]);
        assert!(!fields.is_empty(), "{card}");
        for (name, is_vector) in fields {
            let components: Vec<Option<u64>> =
                if is_vector { vec![Some(0), Some(1), Some(2)] } else { vec![None] };
            for component in components {
                let mut measurements = vec![json!({ "step": 1, "value": 0.5 })];
                if landed > 1 {
                    measurements.push(json!({ "step": landed, "value": 0.5 }));
                }
                let mut body = request(&run, &name, Value::Array(measurements));
                if let Some(component) = component {
                    body["component"] = json!(component);
                }
                let (answer, _) = validate(&base, &body).await;
                assert_eq!(answer["run_seal"], alone["run_seal"], "{card} {name}");
                assert_eq!(answer["outcome"], alone["outcome"], "{card} {name}");
                assert_eq!(answer["steps_landed"], alone["steps_landed"], "{card} {name}");
                assert_eq!(answer["engine_version"], alone["engine_version"], "{card} {name}");
                // The last model value is the quantity of the run's own final state, to the bit.
                let hex = &alone["final_state"][format!("{name}_hex")];
                let expected = match component {
                    Some(index) => hex[index as usize].as_str().unwrap(),
                    None => hex.as_str().unwrap(),
                };
                let model = answer["model_values_hex"].as_array().unwrap();
                assert_eq!(model.last().unwrap().as_str().unwrap(), expected, "{card} {name}");
                quantities_checked += 1;
            }
        }
    }
    assert!(quantities_checked >= 100, "{quantities_checked}");
}

// V2
#[tokio::test]
async fn measurements_equal_to_the_models_own_values_give_errors_of_exactly_zero() {
    let base = spawn_app().await;
    let run = thermal_run(6);
    let probe = request(
        &run,
        "temperature_k",
        json!([{ "step": 2, "value": 1.0 }, { "step": 4, "value": 1.0 }, { "step": 6, "value": 1.0 }]),
    );
    let (answer, _) = validate(&base, &probe).await;
    let model = model_values(&answer);
    assert_eq!(model.len(), 3);
    let exact = request(
        &run,
        "temperature_k",
        json!([
            { "step": 2, "value": model[0] },
            { "step": 4, "value": model[1] },
            { "step": 6, "value": model[2] }
        ]),
    );
    let (answer, _) = validate(&base, &exact).await;
    for name in ["rms_error", "mean_signed_error", "max_abs_error"] {
        assert_eq!(answer[name], 0.0, "{name}");
        assert_eq!(answer[format!("{name}_hex")], "0000000000000000", "{name}");
    }
    assert_eq!(answer["max_abs_error_step"], 2, "a tie goes to the first step");
    assert_eq!(answer["compared"], 3);
    assert_eq!(answer["not_reached"], 0);
}

// V3
#[tokio::test]
async fn a_known_offset_gives_exactly_that_mean_and_rms() {
    let base = spawn_app().await;
    let run = thermal_run(6);
    let probe = request(
        &run,
        "temperature_k",
        json!([{ "step": 1, "value": 1.0 }, { "step": 3, "value": 1.0 }, { "step": 6, "value": 1.0 }]),
    );
    let (answer, _) = validate(&base, &probe).await;
    let model = model_values(&answer);
    // A quarter of a kelvin is a multiple of the spacing of numbers near 300, so the
    // subtraction is exact and the error of every point is 0.25.
    let measurements: Vec<Value> = [1_u64, 3, 6]
        .iter()
        .zip(&model)
        .map(|(step, value)| json!({ "step": step, "value": value - 0.25 }))
        .collect();
    let (answer, _) = validate(&base, &request(&run, "temperature_k", Value::Array(measurements))).await;
    assert_eq!(answer["mean_signed_error"], 0.25);
    assert_eq!(answer["rms_error"], 0.25);
    assert_eq!(answer["max_abs_error"], 0.25);
    assert_eq!(answer["mean_signed_error_hex"], format!("{:016x}", 0.25_f64.to_bits()));
    assert_eq!(answer["max_abs_error_step"], 1);
}

// V4
#[tokio::test]
async fn the_statistics_rebuilt_by_the_tests_own_arithmetic_match_to_the_bit() {
    let base = spawn_app().await;
    let run = thermal_run(40);
    let values = [293.2_f64, 293.71, 294.3, 294.05, 295.9, 296.001, 296.5];
    let steps = [3_u64, 7, 12, 18, 25, 33, 40];
    let measurements: Vec<Value> = steps
        .iter()
        .zip(values)
        .map(|(step, value)| json!({ "step": step, "value": value }))
        .collect();
    let (answer, _) = validate(&base, &request(&run, "temperature_k", Value::Array(measurements))).await;
    let model = model_values(&answer);
    assert_eq!(model.len(), 7);
    // The rule, written out again here.
    let mut sum = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut greatest = -1.0_f64;
    let mut greatest_step = 0_u64;
    for index in 0..7 {
        let error = model[index] - values[index];
        sum = sum + error;
        let square = error * error;
        sum_squares = sum_squares + square;
        if error.abs() > greatest {
            greatest = error.abs();
            greatest_step = steps[index];
        }
    }
    let mean = sum / 7.0;
    let rms = (sum_squares / 7.0).sqrt();
    assert_eq!(answer["mean_signed_error_hex"], format!("{:016x}", mean.to_bits()));
    assert_eq!(answer["rms_error_hex"], format!("{:016x}", rms.to_bits()));
    assert_eq!(answer["max_abs_error_hex"], format!("{:016x}", greatest.to_bits()));
    assert_eq!(answer["max_abs_error_step"], greatest_step);
    assert_eq!(answer["mean_signed_error"].as_f64().unwrap().to_bits(), mean.to_bits());
    assert_eq!(answer["rms_error"].as_f64().unwrap().to_bits(), rms.to_bits());
    assert_eq!(answer["max_abs_error"].as_f64().unwrap().to_bits(), greatest.to_bits());

    // The seal, rebuilt from the answer by the recipe in the map.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(answer["run_seal"].as_str().unwrap().as_bytes());
    bytes.extend_from_slice(answer["measurements_sha256"].as_str().unwrap().as_bytes());
    bytes.extend_from_slice(
        format!(
            "{{\"quantity\":\"temperature_k\",\"component\":null,\"outcome\":{},\"steps_landed\":{},\"reject\":{},\"compared\":7,\"not_reached\":0,\"max_abs_error_step\":{greatest_step}}}",
            answer["outcome"],
            answer["steps_landed"],
            answer["reject"]
        )
        .as_bytes(),
    );
    for name in ["rms_error", "mean_signed_error", "max_abs_error"] {
        bytes.extend_from_slice(
            format!("{name}_hex:{}\n", answer[format!("{name}_hex")].as_str().unwrap()).as_bytes(),
        );
    }
    let joined: Vec<&str> =
        answer["model_values_hex"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    bytes.extend_from_slice(sha256_hex(joined.join(",").as_bytes()).as_bytes());
    assert_eq!(answer["validation_seal"], sha256_hex(&bytes));
}

fn discharge_run() -> Value {
    // A 1C discharge from full charge at the card's worked step: the cell
    // reaches its 2.5 V cutoff part way and the run refuses. (A start low in
    // charge carries a discharge on, and arrives with from.)
    json!({
        "domain": "energy",
        "card": "cell_18650pf",
        "initial": { "soc": 1.0, "held_current_a": 2.9 },
        "dt_s": 17.436,
        "segments": [{ "steps": 250, "load": { "current_a": 2.9 } }],
        "record_every": 1
    })
}

// V5
#[tokio::test]
async fn a_run_that_refuses_part_way_reports_not_reached_and_still_seals() {
    let base = spawn_app().await;
    let run = discharge_run();
    let (status, alone) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{alone}");
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["outcome"], "refused", "{alone}");
    let landed = alone["steps_landed"].as_u64().unwrap();
    assert!((1..250).contains(&landed), "{landed}");

    let measurements = json!([
        { "step": 1, "value": 3.6 },
        { "step": landed, "value": 3.5 },
        { "step": 250, "value": 3.0 }
    ]);
    let body = request(&run, "terminal_v", measurements);
    let (answer, _) = validate(&base, &body).await;
    assert_eq!(answer["outcome"], "refused");
    assert_eq!(answer["reject"], alone["reject"]);
    assert_eq!(answer["reject_detail"], alone["reject_detail"]);
    assert_eq!(answer["steps_landed"], alone["steps_landed"]);
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    let expected_compared = if landed == 1 { 1 } else { 2 };
    assert_eq!(answer["compared"], expected_compared);
    assert_eq!(answer["not_reached"], 3 - expected_compared);
    assert_eq!(answer["model_values_hex"].as_array().unwrap().len(), expected_compared as usize);
    assert_eq!(answer["validation_seal"].as_str().unwrap().len(), 64);
}

// V6
#[tokio::test]
async fn nothing_compared_gives_null_errors_and_still_seals() {
    let base = spawn_app().await;
    // A run whose first step refuses: landed 0, so every measurement is not reached.
    // The cold plate just under its ceiling, heated: its first step leaves the card.
    let run = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": 398.0 },
        "dt_s": 1.0,
        "segments": [{ "steps": 3, "load": { "heat_power_w": 3000.0, "ambient_k": 293.15 } }],
        "record_every": 1
    });
    let (status, alone) = post(&base, "/run_trajectory", &run).await;
    assert_eq!(status, 200, "{alone}");
    let alone: Value = serde_json::from_str(&alone).unwrap();
    assert_eq!(alone["steps_landed"], 0, "{alone}");
    let (answer, _) =
        validate(&base, &request(&run, "temperature_k", json!([{ "step": 1, "value": 4.0 }]))).await;
    assert_eq!(answer["compared"], 0);
    assert_eq!(answer["not_reached"], 1);
    for name in [
        "rms_error",
        "rms_error_hex",
        "mean_signed_error",
        "mean_signed_error_hex",
        "max_abs_error",
        "max_abs_error_hex",
        "max_abs_error_step",
    ] {
        assert!(answer[name].is_null(), "{name}");
    }
    assert_eq!(answer["model_values_hex"], json!([]));
    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["validation_seal"].as_str().unwrap().len(), 64);
}

// V7
#[tokio::test]
async fn one_changed_measurement_changes_the_hash_and_the_seal_and_a_respelling_does_not() {
    let base = spawn_app().await;
    let run = thermal_run(5);
    let spelled = |text: &str| -> String {
        format!(
            "{{\"run\":{},\"quantity\":\"temperature_k\",\"measurements\":{text}}}",
            serde_json::to_string(&run).unwrap()
        )
    };
    let (status, first) = post_text(
        &base,
        "/validate_series",
        &spelled(r#"[{"step":2,"value":294.0},{"step":5,"value":1.0}]"#),
    )
    .await;
    assert_eq!(status, 200, "{first}");
    let first: Value = serde_json::from_str(&first).unwrap();

    // 294.0 written as 294, 1.0 as 1e0, the keys in another order, spaces added.
    let (status, respelled) = post_text(
        &base,
        "/validate_series",
        &spelled(r#"[ { "value" : 294 , "step" : 2 } , {"value":1e0,"step":5} ]"#),
    )
    .await;
    assert_eq!(status, 200, "{respelled}");
    let respelled: Value = serde_json::from_str(&respelled).unwrap();
    assert_eq!(first["measurements_sha256"], respelled["measurements_sha256"]);
    assert_eq!(first["validation_seal"], respelled["validation_seal"]);
    assert_eq!(first["rms_error_hex"], respelled["rms_error_hex"]);

    // The last bit of one value.
    let nudged = f64::from_bits(294.0_f64.to_bits() + 1);
    let (status, changed) = post_text(
        &base,
        "/validate_series",
        &spelled(&format!(r#"[{{"step":2,"value":{nudged:?}}},{{"step":5,"value":1.0}}]"#)),
    )
    .await;
    assert_eq!(status, 200, "{changed}");
    let changed: Value = serde_json::from_str(&changed).unwrap();
    assert_ne!(first["measurements_sha256"], changed["measurements_sha256"]);
    assert_ne!(first["validation_seal"], changed["validation_seal"]);
    assert_eq!(first["run_seal"], changed["run_seal"]);
    assert_eq!(first["model_values_hex"], changed["model_values_hex"]);
}

// V8
#[tokio::test]
async fn every_refusal_is_by_name() {
    let base = spawn_app().await;
    let run = thermal_run(10);
    let good = json!([{ "step": 2, "value": 294.0 }, { "step": 4, "value": 295.0 }]);
    let cell = discharge_run();
    let ti64 = {
        let mut run = json!({
            "domain": "materials",
            "card": "ti64_lattice",
            "initial": {
                "position_m": [0.0, 0.0, 0.0],
                "velocity_m_s": [0.0, 0.0, 0.0],
                "start_t_s": 0.0, "start_force_n": [0.0, 0.0, 0.0], "start_temperature_k": 293.15, "start_energy_anchor_j": 0.0, "start_boundary_work_j": 0.0
            },
            "dt_s": 1.0e-9,
            "segments": [{ "steps": 5, "load": {
                "force_n": [0.0, 0.0, 0.0],
                "torque_nm": [0.0, 0.0, 0.0],
                "temperature_k": 300.0
            } }],
            "record_every": 1
        });
        run["domain"] = json!("materials");
        run
    };

    let with = |key: &str, value: Value| {
        let mut body = request(&run, "temperature_k", good.clone());
        body[key] = value;
        body
    };
    let measured = |measurements: Value| request(&run, "temperature_k", measurements);

    let cases: Vec<(&str, Value, &str)> = vec![
        (
            "an unknown quantity",
            request(&run, "voltage_v", good.clone()),
            "Physical Violation: 'quantity' is 'voltage_v', which the thermal card does not publish.",
        ),
        (
            "the hex sibling as a quantity",
            request(&run, "temperature_k_hex", good.clone()),
            "Physical Violation: 'quantity' is 'temperature_k_hex', which the thermal card does not publish.",
        ),
        (
            "a flag as the quantity",
            request(&run, "derated", good.clone()),
            "Physical Violation: 'quantity' is 'derated', a flag. A flag is not a number and has no value to compare.",
        ),
        (
            "a component on a single number",
            with("component", json!(0)),
            "Physical Violation: 'component' is refused: 'temperature_k' is one number, not a 3-vector.",
        ),
        (
            "no component on a 3-vector",
            request(&ti64, "position_m", good.clone()),
            "Missing required parameter: 'component'",
        ),
        (
            "a component of 3",
            {
                let mut body = request(&ti64, "position_m", good.clone());
                body["component"] = json!(3);
                body
            },
            "Physical Violation: 'component' must be 0, 1 or 2.",
        ),
        (
            "a component written 1.0",
            {
                let mut body = request(&ti64, "position_m", good.clone());
                body["component"] = json!(1.0);
                body
            },
            "Physical Violation: 'component' must be 0, 1 or 2.",
        ),
        (
            "step 0",
            measured(json!([{ "step": 0, "value": 294.0 }])),
            "Physical Violation: 'measurements[0].step' must be an integer >= 1.",
        ),
        (
            "a step written 2.0",
            measured(json!([{ "step": 2.0, "value": 294.0 }])),
            "Physical Violation: 'measurements[0].step' must be an integer >= 1.",
        ),
        (
            "steps that do not rise",
            measured(json!([{ "step": 4, "value": 294.0 }, { "step": 4, "value": 295.0 }])),
            "Physical Violation: 'measurements[1].step' must be greater than the step before it.",
        ),
        (
            "steps that fall",
            measured(json!([{ "step": 4, "value": 294.0 }, { "step": 3, "value": 295.0 }])),
            "Physical Violation: 'measurements[1].step' must be greater than the step before it.",
        ),
        (
            "a step past the run",
            measured(json!([{ "step": 2, "value": 294.0 }, { "step": 11, "value": 295.0 }])),
            "Physical Violation: 'measurements[1].step' is 11, past the 10 steps the run asks for.",
        ),
        (
            "no measurements",
            measured(json!([])),
            "Physical Violation: 'measurements' must contain at least one measurement.",
        ),
        (
            "measurements that are not a list",
            measured(json!({ "step": 1, "value": 1.0 })),
            "Physical Violation: 'measurements' must be a JSON array.",
        ),
        (
            "an unknown key in a measurement",
            measured(json!([{ "step": 2, "value": 294.0, "sigma": 0.1 }])),
            "Physical Violation: 'sigma' is not a field of this request.",
        ),
        (
            "a measurement without a value",
            measured(json!([{ "step": 2 }])),
            "Missing required parameter: 'measurements[0].value'",
        ),
        (
            "a value that is a string",
            measured(json!([{ "step": 2, "value": "294" }])),
            "Physical Violation: 'measurements[0].value' must be a number.",
        ),
        (
            "an unknown top-level key",
            with("tolerance", json!(0.1)),
            "Physical Violation: 'tolerance' is not a field of this request.",
        ),
        (
            "a missing run",
            json!({ "quantity": "temperature_k", "measurements": good.clone() }),
            "Missing required parameter: 'run'",
        ),
        (
            "a missing quantity",
            json!({ "run": run.clone(), "measurements": good.clone() }),
            "Missing required parameter: 'quantity'",
        ),
        (
            "missing measurements",
            json!({ "run": run.clone(), "quantity": "temperature_k" }),
            "Missing required parameter: 'measurements'",
        ),
        (
            "a run refused as run_trajectory refuses it",
            {
                let mut bad = run.clone();
                bad["dt_s"] = json!(-1.0);
                request(&bad, "temperature_k", good.clone())
            },
            "",
        ),
        (
            "a subnormal value",
            measured(json!([{ "step": 2, "value": 5e-324 }])),
            "Physical Violation: 'measurements[0].value' is not a finite normal number.",
        ),
        (
            "a value too large for a frame slot",
            measured(json!([{ "step": 2, "value": 1e300 }])),
            "Physical Violation: 'measurements[0].value' does not fit in a finite frame slot.",
        ),
    ];
    for (label, body, sentence) in cases {
        let text = refused(&base, &body).await;
        if sentence.is_empty() {
            let (_, run_text) = post(&base, "/run_trajectory", &body["run"]).await;
            assert_eq!(text, run_text, "{label}");
        } else {
            assert_eq!(text, sentence, "{label}");
        }
    }

    // An unknown card is 404, as on run_trajectory.
    let mut unknown = request(&run, "temperature_k", good.clone());
    unknown["run"]["card"] = json!("no_such_card");
    let (status, text) = post(&base, "/validate_series", &unknown).await;
    assert_eq!(status, 404, "{text}");

    // A repeated key and a number that is not zero and reads as zero.
    let run_text = serde_json::to_string(&run).unwrap();
    let (status, text) = post_text(
        &base,
        "/validate_series",
        &format!(
            "{{\"run\":{run_text},\"quantity\":\"temperature_k\",\"quantity\":\"temperature_k\",\"measurements\":[{{\"step\":2,\"value\":294.0}}]}}"
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, "Physical Violation: the key 'quantity' is repeated.");
    let (status, text) = post_text(
        &base,
        "/validate_series",
        &format!(
            "{{\"run\":{run_text},\"quantity\":\"temperature_k\",\"measurements\":[{{\"step\":2,\"step\":3,\"value\":294.0}}]}}"
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, "Physical Violation: the key 'step' is repeated.");
    let (status, text) = post_text(
        &base,
        "/validate_series",
        &format!(
            "{{\"run\":{run_text},\"quantity\":\"temperature_k\",\"measurements\":[{{\"step\":2,\"value\":1e-400}}]}}"
        ),
    )
    .await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("1e-400") && text.contains("not zero and reads as zero"), "{text}");
    let (status, _) = post_text(&base, "/validate_series", "[1]").await;
    assert_eq!(status, 422);

    // The cell is a scalar card too: its quantity is the voltage, and a flag is refused.
    let flag = refused(&base, &request(&cell, "sag_warning", json!([{ "step": 1, "value": 1.0 }]))).await;
    assert!(flag.contains("a flag"), "{flag}");
}

// V9
#[tokio::test]
async fn too_many_measurements_are_refused_by_name_and_the_limit_itself_is_served() {
    let base = spawn_app().await;
    let run = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{ "steps": 10_001, "load": { "heat_power_w": 5.0, "ambient_k": 293.15 } }],
        "record_every": 2
    });
    let many = |count: u64| -> Value {
        Value::Array((1..=count).map(|step| json!({ "step": step, "value": 294.0 })).collect())
    };
    let text = refused(&base, &request(&run, "temperature_k", many(10_001))).await;
    assert_eq!(text, "Physical Violation: 'measurements' holds more than 10000 measurements.");
    let (answer, text) = validate(&base, &request(&run, "temperature_k", many(10_000))).await;
    assert_eq!(answer["compared"], 10_000);
    assert_eq!(answer["model_values_hex"].as_array().unwrap().len(), 10_000);
    assert!(text.len() < 1_000_000, "{}", text.len());
}

// V10
#[tokio::test]
async fn the_answer_has_its_keys_in_order_and_no_verdict() {
    let base = spawn_app().await;
    let run = thermal_run(5);
    let body = request(&run, "temperature_k", json!([{ "step": 3, "value": 294.0 }]));
    let (answer, text) = validate(&base, &body).await;
    let keys: Vec<&str> = answer.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "inputs",
            "engine_version",
            "outcome",
            "steps_landed",
            "reject",
            "reject_detail",
            "run_seal",
            "compared",
            "not_reached",
            "rms_error",
            "rms_error_hex",
            "mean_signed_error",
            "mean_signed_error_hex",
            "max_abs_error",
            "max_abs_error_hex",
            "max_abs_error_step",
            "model_values",
            "model_values_hex",
            "measurements_sha256",
            "validation_seal",
            "does_not_show",
            "receipt_sha256"
        ]
    );
    for forbidden in ["verdict", "pass", "fail", "passed", "failed", "threshold", "tolerance", "within", "ok", "valid"] {
        assert!(answer.get(forbidden).is_none(), "{forbidden}");
    }
    assert_eq!(answer["inputs"], body, "the request as received");
    assert_eq!(
        answer["does_not_show"],
        "The numbers say how far this run was from these measurements. They are not a verdict. The seal has no key, so it does not show who measured or who computed."
    );
    assert_eq!(answer["engine_version"], protocol_mcp_gateway::ENGINE_VERSION);
    // The same request gives the same bytes.
    let (status, again) = post(&base, "/validate_series", &body).await;
    assert_eq!(status, 200);
    assert_eq!(again, text);
}

// V11
#[tokio::test]
async fn verify_receipt_matches_the_answer_and_a_changed_decimal_does_not() {
    let base = spawn_app().await;
    let run = thermal_run(5);
    let (_, text) =
        validate(&base, &request(&run, "temperature_k", json!([{ "step": 3, "value": 294.0 }]))).await;
    let (status, report) = post(&base, "/verify_receipt", &json!({ "response_text": text })).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["matches"], true, "{report}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["receipt_sha256"], answer["receipt_sha256"]);
    // The receipt is the hash of every byte before it.
    let tail = format!(",\"receipt_sha256\":\"{}\"}}", answer["receipt_sha256"].as_str().unwrap());
    let document = format!("{}}}", text.strip_suffix(&tail).unwrap());
    assert_eq!(sha256_hex(document.as_bytes()), answer["receipt_sha256"]);

    let altered = change_rms_decimal(&text);
    assert_ne!(altered, text);
    let (_, report) = post(&base, "/verify_receipt", &json!({ "response_text": altered })).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["matches"], false);
}

/// The text with the decimal of `rms_error` changed and its hex left alone.
fn change_rms_decimal(text: &str) -> String {
    let start = text.find("\"rms_error\":").unwrap() + "\"rms_error\":".len();
    let end = start + text[start..].find(',').unwrap();
    format!("{}0.0{}", &text[..start], &text[end..])
}

fn seal_program() -> std::path::PathBuf {
    let seal_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join("../genesis_seal");
    let built = std::process::Command::new(env!("CARGO"))
        .args(["build", "--offline", "--quiet", "--manifest-path"])
        .arg(seal_crate.join("Cargo.toml"))
        .status()
        .unwrap();
    assert!(built.success());
    [
        seal_crate.join("target/debug/genesis_seal"),
        seal_crate.join("../target/debug/genesis_seal"),
    ]
    .into_iter()
    .find(|path| path.exists())
    .expect("the program of the sealing crate")
}

fn run_program(program: &Path, file: &Path) -> (i32, String) {
    let output = std::process::Command::new(program)
        .arg("verify-validation")
        .arg(file)
        .output()
        .unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("validation_router");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

// V12
#[tokio::test]
async fn verify_validation_holds_a_genuine_answer_and_names_each_change() {
    let program = seal_program();
    let base = spawn_app().await;
    let run = thermal_run(30);
    let measurements = json!([
        { "step": 4, "value": 293.5 }, { "step": 9, "value": 294.1 },
        { "step": 20, "value": 296.0 }, { "step": 30, "value": 297.2 }
    ]);
    let (answer, text) = validate(&base, &request(&run, "temperature_k", measurements)).await;
    let file = scratch("genuine.json");
    std::fs::write(&file, &text).unwrap();
    let (code, out) = run_program(&program, &file);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("holds: true\ncause: none\ncompared: 4\nnot_reached: 0\n"), "{out}");
    assert!(out.contains(&format!("validation_seal: {}\n", answer["validation_seal"].as_str().unwrap())), "{out}");
    assert!(out.ends_with("receipt: matches\n"), "{out}");

    // Each change, written back with its receipt made again, so that the seal alone speaks.
    let rewrite = |edit: &dyn Fn(&mut Value)| -> String {
        let mut changed = answer.clone();
        edit(&mut changed);
        changed.as_object_mut().unwrap().remove("receipt_sha256");
        let document = serde_json::to_string(&changed).unwrap();
        let receipt = sha256_hex(document.as_bytes());
        format!("{},\"receipt_sha256\":\"{receipt}\"}}", document.strip_suffix('}').unwrap())
    };
    let statistic = rewrite(&|a| a["rms_error_hex"] = json!(format!("{:016x}", 0.5_f64.to_bits())));
    let model_value = rewrite(&|a| {
        a["model_values_hex"][2] = json!(format!("{:016x}", 300.0_f64.to_bits()));
    });
    let hash = rewrite(&|a| a["measurements_sha256"] = json!("0".repeat(64)));
    let measurement = rewrite(&|a| a["inputs"]["measurements"][1]["value"] = json!(294.2));
    // The three fields the seal took in with 0.2.1: each is named by the seal, with the receipt made again.
    let outcome = rewrite(&|a| a["outcome"] = json!("refused"));
    let reject = rewrite(&|a| a["reject"] = json!("FirstLawBreach"));
    let landed = rewrite(&|a| a["steps_landed"] = json!(31));
    for (name, text) in [("outcome", outcome), ("reject", reject), ("steps_landed", landed)] {
        let file = scratch(&format!("{name}.json"));
        std::fs::write(&file, text).unwrap();
        let (code, out) = run_program(&program, &file);
        assert_eq!(code, 1, "{name}: {out}");
        assert!(out.starts_with("holds: false\ncause: validation_seal\n"), "{name}: {out}");
    }
    for (name, text, cause) in [
        ("statistic", statistic, "statistics"),
        ("model_value", model_value, "model_values"),
        ("hash", hash, "measurements"),
        ("measurement", measurement, "measurements"),
    ] {
        let file = scratch(&format!("{name}.json"));
        std::fs::write(&file, text).unwrap();
        let (code, out) = run_program(&program, &file);
        assert_eq!(code, 1, "{name}: {out}");
        assert!(out.starts_with(&format!("holds: false\ncause: {cause}\n")), "{name}: {out}");
    }

    // A changed decimal beside its untouched hex: the seals hold and the receipt does not.
    let altered = change_rms_decimal(&text);
    assert_ne!(altered, text);
    let file = scratch("decimal.json");
    std::fs::write(&file, altered).unwrap();
    let (code, out) = run_program(&program, &file);
    assert_eq!(code, 3, "{out}");
    assert!(out.starts_with("holds: true\n") && out.ends_with("receipt: does not match\n"), "{out}");
}

fn tool_text(result: &rmcp::model::CallToolResult) -> String {
    result.content[0].as_text().expect("text").text.clone()
}

fn arguments(value: &Value) -> Map<String, Value> {
    value.as_object().expect("object").clone()
}

// V13
#[tokio::test]
async fn the_tool_gives_the_bytes_of_the_route_and_the_same_refusals() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let base = spawn_app().await;

    let listed = client.list_tools(None).await.expect("tools/list");
    let tool = listed.tools.iter().find(|tool| tool.name == "validate_series").expect("listed");
    // The thirteenth tool, the last of those served in 0.8.1; cite, find_model and find_flip follow it.
    assert_eq!(listed.tools[12].name, "validate_series");
    let schema = Value::Object(tool.input_schema.as_ref().clone());
    // run or solves: neither is always required.
    assert_eq!(schema["required"], json!(["quantity", "measurements"]));
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["measurements"]["maxItems"], 10_000);
    assert!(schema["properties"]["run"].get("oneOf").is_none());
    // The card is a string; list_cards names the cards.
    assert_eq!(schema["properties"]["run"]["properties"]["card"]["type"], "string");
    assert!(schema["properties"]["run"]["properties"]["card"].get("enum").is_none());
    assert!(schema["properties"]["run"]["properties"].get("readout").is_none());

    let run = thermal_run(8);
    let good = request(&run, "temperature_k", json!([{ "step": 2, "value": 294.0 }, { "step": 8, "value": 297.0 }]));
    let mut unknown = good.clone();
    unknown["quantity"] = json!("voltage_v");
    let mut late = good.clone();
    late["measurements"] = json!([{ "step": 9, "value": 294.0 }]);
    let mut extra = good.clone();
    extra["tolerance"] = json!(1);
    for (label, body, status) in [
        ("an answer", good, 200),
        ("an unknown quantity", unknown, 422),
        ("a step past the run", late, 422),
        ("an extra key", extra, 422),
    ] {
        let (http_status, http_text) = post(&base, "/validate_series", &body).await;
        assert_eq!(http_status, status, "{label}: {http_text}");
        let called = client
            .call_tool(CallToolRequestParams::new("validate_series").with_arguments(arguments(&body)))
            .await
            .expect("tool result");
        assert_eq!(called.is_error, Some(status != 200), "{label}");
        assert_eq!(tool_text(&called), http_text, "{label}");
    }
    client.close().await.expect("close");
}

/// The saved full discharge of the cell, a run of engine 0.2. The saved runs
/// are not part of this crate: the environment variable
/// `ZERO_TRUST_PHYSICS_RUNS` names the folder `runs`, and this run stands in
/// the folder `runs_v02` beside it. `None` where the variable is unset or the
/// file is not there.
fn saved_discharge() -> Option<std::path::PathBuf> {
    let runs = std::path::PathBuf::from(std::env::var_os("ZERO_TRUST_PHYSICS_RUNS")?);
    let file = runs.with_file_name("runs_v02").join("cell_18650pf_validation/sealed_full_discharge_03-10.json");
    file.is_file().then_some(file)
}

// V14
#[tokio::test]
async fn one_saved_cell_run_is_scored_end_to_end_through_the_built_binary() {
    let Some(saved_discharge) = saved_discharge() else {
        // One line, written straight to standard error so that it is seen
        // while test output is captured.
        use std::io::Write;
        let line = "skipped: validation_router::one_saved_cell_run_is_scored_end_to_end_through_the_built_binary needs the saved runs; set ZERO_TRUST_PHYSICS_RUNS to their folder\n";
        let _ = std::io::stderr().write_all(line.as_bytes());
        return;
    };
    let saved: Value = serde_json::from_slice(&std::fs::read(&saved_discharge).expect("the saved run")).unwrap();
    let mut run = saved["inputs"].clone();
    // Engine 0.7.0 takes the held current of the discharge in 'initial': the
    // saved run's first logged current.
    let held = run["segments"][0]["load"]["current_a"].clone();
    run["initial"]["held_current_a"] = held;
    let landed = saved["steps_landed"].as_u64().unwrap();
    assert!(landed < saved["steps_requested"].as_u64().unwrap(), "the saved run refuses at the cutoff");

    // A handful of made-up terminal voltages, the last two after the cell's cutoff.
    let measurements = json!([
        { "step": 100, "value": 4.05 },
        { "step": 5000, "value": 3.84 },
        { "step": 20000, "value": 3.52 },
        { "step": landed, "value": 2.52 },
        { "step": landed + 500, "value": 2.5 },
        { "step": 40270, "value": 2.5 }
    ]);
    let body = request(&run, "terminal_v", measurements);

    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let transport =
        TokioChildProcess::new(tokio::process::Command::new(bin)).expect("spawn mcp_stdio");
    let mut client = ().serve(transport).await.expect("initialize");
    let called = client
        .call_tool(CallToolRequestParams::new("validate_series").with_arguments(arguments(&body)))
        .await
        .expect("validate_series");
    assert_eq!(called.is_error, Some(false));
    let text = tool_text(&called);
    let answer: Value = serde_json::from_str(&text).unwrap();
    let alone = client
        .call_tool(CallToolRequestParams::new("run_trajectory").with_arguments(arguments(&run)))
        .await
        .expect("run_trajectory");
    let alone: Value = serde_json::from_str(&tool_text(&alone)).unwrap();
    client.close().await.expect("close");

    assert_eq!(answer["run_seal"], alone["run_seal"]);
    assert_eq!(answer["outcome"], "refused");
    assert_eq!(answer["reject"], "UndervoltageCutoff");
    assert_eq!(answer["steps_landed"], landed);
    // The compared discharge is unchanged by the 0.7.0 range: the same last
    // state, bit for bit, as the saved run of engine 0.2.
    assert_eq!(alone["final_state"], saved["final_state"]);
    assert_eq!(answer["compared"], 4);
    assert_eq!(answer["not_reached"], 2);
    // The last compared model value is the terminal voltage of the run's own last state.
    let last = answer["model_values_hex"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last, alone["final_state"]["terminal_v_hex"]);
    assert!(answer["rms_error"].as_f64().unwrap() > 0.0);
    println!("{}", serde_json::to_string_pretty(&answer["rms_error"]).unwrap());

    let program = seal_program();
    let file = scratch("saved_cell.json");
    std::fs::write(&file, &text).unwrap();
    let (code, out) = run_program(&program, &file);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("holds: true\n"), "{out}");
}

// V15
#[tokio::test]
async fn the_route_takes_a_body_of_more_than_2_mib() {
    let base = spawn_app().await;
    let run = thermal_run(5);
    let mut body = request(&run, "temperature_k", json!([{ "step": 3, "value": 294.0 }]));
    body["note"] = json!("a".repeat(3 * 1024 * 1024));
    let text = refused(&base, &body).await;
    assert_eq!(text, "Physical Violation: 'note' is not a field of this request.");
}
