//! The solver `seawater_column` on the web routes: `POST /solve` against a
//! direct call to `genesis_solvers`, the description, each limit of a cast
//! at the door, and `POST /verify_solution` against edits of a solution.

mod answer_text;

use std::time::Duration;

use genesis_solvers::seawater_column::{self as column, Cast};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
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

const PRESSURE: [f64; 8] = [2.0, 50.0, 150.0, 250.0, 350.0, 450.0, 550.0, 600.0];
const TEMPERATURE: [f64; 8] = [24.1, 23.0, 17.5, 13.2, 10.9, 9.1, 7.7, 7.2];
const SALINITY: [f64; 8] = [35.1, 35.2, 35.4, 35.1, 34.9, 34.7, 34.6, 34.6];

fn request() -> Value {
    json!({
        "solver": "seawater_column",
        "parameters": {
            "latitude_deg": -20.0,
            "sea_pressure_dbar": PRESSURE,
            "temperature_c": TEMPERATURE,
            "practical_salinity": SALINITY,
            "height_above_bottom_m": 8.0,
        }
    })
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

#[tokio::test]
async fn a_solved_cast_is_the_solver_bit_for_bit_and_its_seal_holds() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/solve", &request()).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = column::solve(&Cast {
        latitude_deg: -20.0,
        sea_pressure_dbar: &PRESSURE,
        temperature_c: &TEMPERATURE,
        practical_salinity: &SALINITY,
        height_above_bottom_m: 8.0,
    })
    .expect("settles");
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["iterations"], json!(direct.sweeps));
    assert_eq!(body["engine_version"], format!("genesis_solvers_v{}", genesis_solvers::VERSION));
    for (key, value) in [
        ("seabed_depth_m", direct.seabed_depth_m),
        ("echo_one_way_time_s", direct.echo_one_way_time_s),
        ("echo_sounder_depth_m", direct.echo_sounder_depth_m),
        ("mean_sound_speed_m_per_s", direct.mean_sound_speed_m_per_s),
    ] {
        assert_eq!(body["result"][format!("{key}_hex")], hex_of(value), "{key}");
        assert_eq!(body["result"][key].as_f64(), Some(value), "{key}");
    }
    for (key, values) in [
        ("depth_m", &direct.depth_m),
        ("density_kg_per_m3", &direct.density_kg_per_m3),
        ("sound_speed_m_per_s", &direct.sound_speed_m_per_s),
    ] {
        let hexes: Vec<String> = values.iter().map(|value| hex_of(*value)).collect();
        assert_eq!(body["result"][format!("{key}_hex")], json!(hexes), "{key}");
    }
    assert_eq!(
        body["invariants"]["hydrostatic_residual_hex"],
        hex_of(direct.hydrostatic_residual)
    );
    assert!(direct.hydrostatic_residual <= column::HYDROSTATIC_TOLERANCE);
    assert_eq!(body["result"].as_object().unwrap().len(), 14);
    assert_eq!(body["invariants"].as_object().unwrap().len(), 4);

    let (status, verdict) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{verdict}");
    assert_eq!(serde_json::from_str::<Value>(&verdict).unwrap(), json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));

    // One level's depth moved by one bit, one level's temperature in the
    // request, and the outcome are each named.
    let mut moved = body.clone();
    let one_up = hex_of(f64::from_bits(direct.depth_m[3].to_bits() + 1));
    moved["result"]["depth_m_hex"][3] = json!(one_up);
    let mut warmer = body.clone();
    warmer["inputs"]["parameters"]["temperature_c"][2] = json!(17.6);
    let mut refused = body.clone();
    refused["outcome"] = json!("refused");
    for (edited, part) in [(moved, "result"), (warmer, "inputs"), (refused, "outcome")] {
        let (status, verdict) = post(&base, "/verify_solution", &edited).await;
        assert_eq!(status, 200, "{verdict}");
        let verdict: Value = serde_json::from_str(&verdict).unwrap();
        assert_eq!(verdict["holds"], false, "{part}");
        assert_eq!(verdict["cause"], part);
    }

    // The same request again is the same bytes.
    let (_, again) = post(&base, "/solve", &request()).await;
    assert_eq!(again, text);
}

#[tokio::test]
async fn the_list_and_the_description_say_what_the_solver_is() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers[5]["solver"], "seawater_column");
    assert!(solvers[5]["summary"].as_str().unwrap().ends_with("no constant is fitted."));

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "seawater_column" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    let names: Vec<&str> = described["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|input| input["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["latitude_deg", "sea_pressure_dbar", "temperature_c", "practical_salinity", "height_above_bottom_m"]
    );
    // The published ranges are the limits of the Gibbs function and of the
    // method; the envelope compared with measurement is stated in words.
    assert_eq!(described["inputs"][0]["minimum"], -90.0);
    assert_eq!(described["inputs"][0]["maximum"], 90.0);
    assert_eq!(described["inputs"][0]["refusal"], "LatitudeOutOfRange");
    assert!(described["inputs"][0]["bound"].as_str().unwrap().contains("-34.0023 to +26.4108"));
    assert_eq!(described["inputs"][1]["maximum"], 10000.0);
    assert_eq!(described["inputs"][1]["refusal"], "PressureOutOfRange");
    assert!(described["inputs"][1]["bound"].as_str().unwrap().contains("6108 dbar"));
    assert_eq!(described["inputs"][1]["most"], 1024);
    assert_eq!(described["inputs"][2]["maximum"], 40.0);
    assert_eq!(described["inputs"][2]["refusal"], "TemperatureOutOfRange");
    assert!(described["inputs"][2]["bound"].as_str().unwrap().contains("0.5088 to 31.169 degC"));
    assert_eq!(described["inputs"][3]["minimum"], 0.0);
    assert_eq!(described["inputs"][3]["maximum"], 41.8);
    assert!(described["inputs"][3]["bound"].as_str().unwrap().contains("30.9805 to 40.0731"));
    assert_eq!(described["inputs"][4]["maximum"], 100.0);
    assert_eq!(described["inputs"][4]["refusal"], "HeightAboveBottomOutOfRange");
    assert!(described["inputs"][4]["bound"].as_str().unwrap().contains("86 m"));
    assert_eq!(described["refusals"].as_array().unwrap().len(), 14);
    for refusal in described["refusals"].as_array().unwrap() {
        assert!(!refusal["meaning"].as_str().unwrap().contains("..="), "{refusal}");
    }
    assert_eq!(described["reference"]["retrieved"], "2026-10-09");
    assert_eq!(described["reference"]["kind"], "A reference calculation. It is not a measurement.");
    for part in ["object", "data", "pass_line", "result", "limits"] {
        assert!(described["validation"][part].as_str().is_some_and(|text| !text.is_empty()), "{part}");
    }
    assert!(described["validation"]["limits"].as_str().unwrap().starts_with("Nothing is fitted."));
    // Sentences held in the result, the limits and the sentence of the
    // temperature.
    let result = described["validation"]["result"].as_str().unwrap();
    for words in [
        "and all three lines were met",
        "the five legs' own medians were +4.30 to +4.92 m against 15 allowed",
        "The 0.80 line was met by two stations and is thin: the binomial standard error of the fraction is 0.016",
        "with leg 9, 10 or 13 left out it falls under 0.80 (0.790, 0.789, 0.798)",
        "a constant shift of the solver's depth between -14.55 m and +0.10 m, so the solver sits at the deep edge",
        "governed by the roughness of the sea floor, not by the model: on flat sea floor the scatter is 2.7 m",
        "a worse measurement (0.769 within 20 m on the scored stations), and give 0.775",
        "the fraction would be 632 of 792, 0.798",
        "EOS-80, also meets all three lines on the same stations (+3.72 m, 509 of 632)",
        "a quadratic in the maximum pressure with three constants fitted on the opened leg (511 of 632)",
        "48 m off, and does not separate equations of state",
        "spreading over 0.6 m for the solver against 12 m for the quadratic, and the three lines do not measure that",
        "evidence/verification/second_party/V-29/",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let limits = described["validation"]["limits"].as_str().unwrap();
    for words in [
        "the echo depth by EOS-80 differs from this solver's by 1.12 m in the median, 27 parts in a hundred thousand",
        "the solver computes the echo depth from the sea surface",
        "about 5 m for the depth of the ADCP transducer; no report gives the depth of the 12 kHz sounder's transducer",
        "one report lists that sounder in the bow chamber",
        "other places and months, not other instruments: the same ship, the same sounder, the same recorder and logging practice",
        "about +3 m on flat sea floor and +4.6 m pooled, rough floor adding about 1.5 m",
        "accounts for the sign and the size of that offset, and it is not proven",
        "With a transducer depth of 4.27 or 5.0 m applied, 515 or 516 of the 632 stations are within 20 m",
        "A cast whose levels lie close together at depth is answered: any pressures that rise from level to level are taken",
        "no least spacing of levels is declared",
        "the balance now allows each layer that rounding, half the spacing of 64-bit numbers at its depth",
        "The casts compared span latitudes -34.0023 to +26.4108, the last level no deeper than 6108 dbar",
        "Polar water, trenches, fresh and brackish water, and hot or hypersaline water are answered and were not compared",
        "The compared casts had 8 to 612 levels, 1 to 66 dbar apart",
        "Seawater below its freezing temperature is refused as TemperatureBelowFreezing",
        "The water between the last level and the sea floor is taken at the last level's sound speed.",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    assert_eq!(described["inputs"][2]["name"], "temperature_c");
    let bound = described["inputs"][2]["bound"].as_str().unwrap();
    assert!(bound.contains("-2.65 degC at absolute pressure zero, as SR7-09 states it for pure water"), "{bound}");
    assert!(bound.contains("TemperatureBelowFreezing: about -1.92 degC at Practical Salinity 35"), "{bound}");
    assert!(!bound.contains("as IAPWS states it"), "{bound}");
    assert_eq!(described["locked"]["compared_deepest_max_dbar"], 6108.0);

    // The example of the description runs unchanged.
    let (status, text) = post(&base, "/solve", &described["example_solve"]).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "converged");
    let floor = body["result"]["seabed_depth_m"].as_f64().unwrap();
    let sounder = body["result"]["echo_sounder_depth_m"].as_f64().unwrap();
    assert!(floor > 1002.0 && floor < 1006.0, "{floor}");
    // Sound in this short, cool column is a little slower than the 1500 m/s
    // the sounder assumes, so the sounder reads a little deep.
    let speed = body["result"]["mean_sound_speed_m_per_s"].as_f64().unwrap();
    assert!(speed > 1485.0 && speed < 1500.0, "{speed}");
    assert!(sounder > floor && sounder - floor < 10.0, "{sounder} {floor}");
}

#[tokio::test]
async fn each_limit_of_a_cast_is_422_by_name_with_no_seal() {
    let base = spawn_app().await;
    let edits: Vec<(&str, Value, &str)> = vec![
        ("latitude_deg", json!(-90.5), "LatitudeOutOfRange"),
        ("height_above_bottom_m", json!(101.0), "HeightAboveBottomOutOfRange"),
        ("sea_pressure_dbar", json!([21.0, 50.0, 150.0, 250.0, 350.0, 450.0, 550.0, 600.0]), "FirstLevelTooDeep"),
        ("sea_pressure_dbar", json!([2.0, 50.0, 150.0, 250.0, 350.0, 450.0, 550.0, 500.0]), "PressureNotRising"),
        ("sea_pressure_dbar", json!([2.0, 50.0, 150.0, 250.0, 350.0, 450.0, 550.0, 651.0]), "LevelGapTooWide"),
        ("sea_pressure_dbar", json!([2.0, 50.0, 150.0, 250.0, 350.0, 450.0, 550.0, 10001.0]), "PressureOutOfRange"),
        ("sea_pressure_dbar", json!([2.0, 50.0, 150.0, 250.0, 350.0, 450.0, 550.0]), "LevelsDoNotMatch"),
        ("temperature_c", json!([24.1, 23.0, 17.5, 13.2, 10.9, 9.1, 7.7, 40.5]), "TemperatureOutOfRange"),
        ("temperature_c", json!([-2.9, 23.0, 17.5, 13.2, 10.9, 9.1, 7.7, 7.2]), "TemperatureOutOfRange"),
        ("practical_salinity", json!([35.1, 35.2, 35.4, 35.1, 34.9, 34.7, 34.6, 41.9]), "SalinityOutOfRange"),
        ("temperature_c", json!([-2.5, 23.0, 17.5, 13.2, 10.9, 9.1, 7.7, 7.2]), "TemperatureBelowFreezing"),
    ];
    for (field, value, name) in edits {
        let mut asked = request();
        asked["parameters"][field] = value;
        let (status, text) = post(&base, "/solve", &asked).await;
        assert_eq!(status, 422, "{name}: {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.ends_with(&format!("({name}).")), "{text}");
        assert!(!text.contains("solution_seal"), "{text}");
        assert!(!text.contains("..="), "{text}");
    }
    // Past the casts compared with measurement, where engine 0.8.0 refused
    // at the door, each cast is answered with a seal that holds.
    let answered: Vec<(&str, Value)> = vec![
        ("latitude_deg", json!(61.0)),
        ("latitude_deg", json!(-89.0)),
        ("height_above_bottom_m", json!(100.0)),
        ("practical_salinity", json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])),
        ("practical_salinity", json!([41.8, 35.2, 35.4, 35.1, 34.9, 34.7, 34.6, 34.6])),
        ("temperature_c", json!([39.9, 23.0, 17.5, 13.2, 10.9, 9.1, 7.7, 7.2])),
        ("temperature_c", json!([-1.9, 23.0, 17.5, 13.2, 10.9, 9.1, 7.7, 7.2])),
    ];
    for (field, value) in answered {
        let mut asked = request();
        asked["parameters"][field] = value;
        let (status, text) = post(&base, "/solve", &asked).await;
        assert_eq!(status, 200, "{field}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{text}");
        let (status, checked) = post(&base, "/verify_solution", &body).await;
        assert_eq!(status, 200, "{checked}");
        let checked: Value = serde_json::from_str(&checked).unwrap();
        assert_eq!(checked["holds"], true, "{checked}");
    }
    // One level: every array is too short.
    let mut one = request();
    for field in ["sea_pressure_dbar", "temperature_c", "practical_salinity"] {
        one["parameters"][field] = json!([2.0]);
    }
    let (status, text) = post(&base, "/solve", &one).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(LevelCountOutOfRange)."), "{text}");
    // A key that is not an input, a missing input, and a value of the wrong kind.
    let mut extra = request();
    extra["parameters"]["density_kg_per_m3"] = json!(1025.0);
    let (status, text) = post(&base, "/solve", &extra).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'density_kg_per_m3' is not an input of this solver."), "{text}");
    let mut missing = request();
    missing["parameters"].as_object_mut().unwrap().remove("latitude_deg");
    let (status, text) = post(&base, "/solve", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'latitude_deg'"), "{text}");
    let mut worded = request();
    worded["parameters"]["temperature_c"][1] = json!("23.0");
    let (status, text) = post(&base, "/solve", &worded).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("temperature_c[1]"), "{text}");
}

/// Seawater of salinity 35 at -2.5 degC, below its freezing temperature, is
/// refused by name with no seal.
#[tokio::test]
async fn seawater_below_its_freezing_temperature_is_refused_by_name() {
    let base = spawn_app().await;
    let asked = json!({
        "solver": "seawater_column",
        "parameters": {
            "latitude_deg": 0.0,
            "sea_pressure_dbar": [0.0, 10.0],
            "temperature_c": [-2.5, -2.5],
            "practical_salinity": [35.0, 35.0],
            "height_above_bottom_m": 0.0,
        }
    });
    let (status, text) = post(&base, "/solve", &asked).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.starts_with("Physical Violation: temperature -2.5 degC at level 0 is below -1.91"), "{text}");
    assert!(text.ends_with("(TemperatureBelowFreezing)."), "{text}");
    assert!(!text.contains("solution_seal"), "{text}");
}

/// A cast to the deepest compared level is answered, and so are deeper
/// ones down to the limit of the Gibbs function, 10000 dbar; one beyond the
/// Gibbs function is refused by the physics' name.
#[tokio::test]
async fn a_cast_is_answered_to_the_limit_of_the_gibbs_function_and_refused_by_name_past_it() {
    let base = spawn_app().await;
    let mut compared: Vec<f64> = (0..=61).map(|level| 100.0 * level as f64).collect();
    compared.push(6108.0);
    let temperature: Vec<f64> = compared.iter().map(|p| 1.5 + 25.0 * (-p / 500.0_f64).exp() + 1.0e-4 * p).collect();
    let salinity = vec![34.7; compared.len()];
    let deepest = json!({
        "solver": "seawater_column",
        "parameters": {
            "latitude_deg": 11.3,
            "sea_pressure_dbar": compared,
            "temperature_c": temperature,
            "practical_salinity": salinity,
            "height_above_bottom_m": 86.0,
        }
    });
    let (status, text) = post(&base, "/solve", &deepest).await;
    assert_eq!(status, 200, "{text}");
    let mut deeper = deepest.clone();
    deeper["parameters"]["sea_pressure_dbar"][62] = json!(6108.5);
    let (status, text) = post(&base, "/solve", &deeper).await;
    assert_eq!(status, 200, "{text}");

    let pressure: Vec<f64> = (0..=100).map(|level| 100.0 * level as f64).collect();
    let temperature: Vec<f64> = pressure.iter().map(|p| 1.5 + 25.0 * (-p / 500.0_f64).exp() + 1.0e-4 * p).collect();
    let salinity = vec![34.7; pressure.len()];
    let mut asked = json!({
        "solver": "seawater_column",
        "parameters": {
            "latitude_deg": 11.3,
            "sea_pressure_dbar": pressure,
            "temperature_c": temperature,
            "practical_salinity": salinity,
            "height_above_bottom_m": 0.0,
        }
    });
    let (status, text) = post(&base, "/solve", &asked).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let floor = body["result"]["seabed_depth_m"].as_f64().unwrap();
    assert!(floor > 9700.0 && floor < 9760.0, "{floor}");
    for field in ["sea_pressure_dbar", "temperature_c", "practical_salinity"] {
        let last = if field == "sea_pressure_dbar" { 10_090.0 } else if field == "temperature_c" { 2.5 } else { 34.7 };
        asked["parameters"][field].as_array_mut().unwrap().push(json!(last));
    }
    let (status, text) = post(&base, "/solve", &asked).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.ends_with("(PressureOutOfRange)."), "{text}");
}

/// Casts with levels close together at depth, which were once refused: forty
/// levels 0.02 dbar apart at 3,000 and 6,000 dbar, 0.05 dbar apart at
/// 6,100 dbar (and 9,900 dbar in the solver's own test) and 0.001 dbar apart at 1,000 dbar, and forty levels one
/// millionth of a decibar apart at 6,000 dbar. Each is answered with a seal
/// that holds. A level that does not rise is still refused by name.
#[tokio::test]
async fn a_cast_with_levels_close_together_at_depth_is_answered() {
    let base = spawn_app().await;
    let mut above_the_tolerance = 0;
    for (deep_dbar, gap_dbar) in [(3000.0_f64, 0.02_f64), (6000.0, 0.02), (6100.0, 0.05), (1000.0, 0.001), (6000.0, 1.0e-6)] {
        let coarse = (deep_dbar / 50.0) as usize;
        let mut pressure: Vec<f64> = (0..=coarse).map(|level| 50.0 * level as f64).collect();
        for index in 1..=40 {
            pressure.push(deep_dbar + gap_dbar * index as f64);
        }
        let temperature: Vec<f64> = pressure.iter().map(|p| 1.5 + 25.0 * (-p / 500.0_f64).exp() + 1.0e-4 * p).collect();
        let salinity = vec![34.7; pressure.len()];
        let mut asked = json!({
            "solver": "seawater_column",
            "parameters": {
                "latitude_deg": -30.0,
                "sea_pressure_dbar": pressure,
                "temperature_c": temperature,
                "practical_salinity": salinity,
                "height_above_bottom_m": 10.0,
            }
        });
        let (status, text) = post(&base, "/solve", &asked).await;
        assert_eq!(status, 200, "{deep_dbar} dbar, {gap_dbar} apart: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{text}");
        if body["invariants"]["hydrostatic_residual"].as_f64().unwrap() > column::HYDROSTATIC_TOLERANCE {
            above_the_tolerance += 1;
        }
        let (status, checked) = post(&base, "/verify_solution", &body).await;
        assert_eq!(status, 200, "{checked}");
        let checked: Value = serde_json::from_str(&checked).unwrap();
        assert_eq!(checked["holds"], true, "{checked}");
        // the last level given the pressure of the one before it does not rise
        let levels = asked["parameters"]["sea_pressure_dbar"].as_array().unwrap().len();
        let before = asked["parameters"]["sea_pressure_dbar"][levels - 2].clone();
        asked["parameters"]["sea_pressure_dbar"][levels - 1] = before;
        let (status, text) = post(&base, "/solve", &asked).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains("PressureNotRising"), "{text}");
    }
    // these are the casts that the tolerance alone refused
    assert!(above_the_tolerance >= 3, "{above_the_tolerance}");
}
