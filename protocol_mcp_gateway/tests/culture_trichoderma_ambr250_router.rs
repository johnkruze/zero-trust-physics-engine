//! `culture_trichoderma_ambr250` through `POST /solve`, against a direct call
//! to `genesis_solvers`, with its refusals at the door and its description.

use std::time::Duration;

use genesis_solvers::culture_trichoderma_ambr250::{self as culture, Question};
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

const ENDS_H: [f64; 4] = [21.5, 45.0, 69.0, 93.5];
const FEED_ML: [f64; 4] = [0.0, 2.25, 7.5, 7.125];
const WATER_ML: [f64; 4] = [-4.5, -0.75, 1.5, -0.25];
const SAMPLE_ML: [f64; 4] = [4.3, 4.3, 4.3, 4.3];

fn asked(change: impl FnOnce(&mut Value)) -> Value {
    let mut request = json!({
        "solver": "culture_trichoderma_ambr250",
        "parameters": {
            "biomass_g_per_l": 0.93,
            "glucose_g_per_l": 20.4,
            "galacturonate_g_per_l": 4.7,
            "feed_glucose_g_per_l": 32.0,
            "feed_galacturonate_g_per_l": 94.0,
            "segment_end_h": ENDS_H,
            "feed_ml": FEED_ML,
            "water_ml": WATER_ML,
            "sample_ml": SAMPLE_ML,
        }
    });
    change(&mut request["parameters"]);
    request
}

/// A record within the schedule to `last` h: a batch to 21.5 h, the feed
/// started to 45 h, then 0.25 mL/h.
fn to(p: &mut Value, last: f64) {
    p["segment_end_h"] = json!([21.5, 45.0, 120.0, 180.0, last]);
    p["feed_ml"] = json!([0.0, 2.25, 18.75, 15.0, 0.25 * (last - 180.0)]);
    p["water_ml"] = json!([-4.5, -0.75, -12.0, -12.0, -0.2 * (last - 180.0)]);
    p["sample_ml"] = json!([4.3, 4.3, 4.3, 4.3, 0.0]);
}

fn hex_of(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn flip_hex(text: &str) -> String {
    let last = if text.ends_with('0') { '1' } else { '0' };
    format!("{}{last}", &text[..text.len() - 1])
}

#[tokio::test]
async fn a_culture_matches_the_solver_bit_for_bit_and_its_proof_holds() {
    let base = spawn_app().await;
    let request = asked(|_| {});
    let (status, text) = post(&base, "/solve", &request).await;
    assert_eq!(status, 200, "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let direct = culture::solve(&Question {
        biomass_g_per_l: 0.93,
        glucose_g_per_l: 20.4,
        galacturonate_g_per_l: 4.7,
        feed_glucose_g_per_l: 32.0,
        feed_galacturonate_g_per_l: 94.0,
        segment_end_h: &ENDS_H,
        feed_ml: &FEED_ML,
        water_ml: &WATER_ML,
        sample_ml: &SAMPLE_ML,
    })
    .expect("the solver settles");

    assert_eq!(body["inputs"], request);
    assert_eq!(body["engine_version"], format!("genesis_solvers_v{}", genesis_solvers::VERSION));
    assert_eq!(body["outcome"], "converged");
    assert!(body["reject"].is_null());
    assert_eq!(body["iterations"], direct.halvings);
    let result = &body["result"];
    for (key, values) in [
        ("biomass_g_per_l", &direct.biomass_g_per_l),
        ("glucose_g_per_l", &direct.glucose_g_per_l),
        ("galacturonate_g_per_l", &direct.galacturonate_g_per_l),
        ("mucic_acid_g_per_l", &direct.mucic_acid_g_per_l),
        ("volume_ml", &direct.volume_ml),
        ("carbon_dioxide_mol", &direct.carbon_dioxide_mol),
        ("oxygen_mol", &direct.oxygen_mol),
    ] {
        let hexes: Vec<String> = values.iter().map(|value| hex_of(*value)).collect();
        assert_eq!(result[format!("{key}_hex")], json!(hexes), "{key}");
        let decimals = result[key].as_array().unwrap_or_else(|| panic!("{key}"));
        assert_eq!(decimals.len(), 4, "{key}");
        for (decimal, value) in decimals.iter().zip(values.iter()) {
            assert_eq!(decimal.as_f64().unwrap().to_bits(), value.to_bits(), "{key}");
        }
    }
    assert_eq!(result["carbon_supplied_mol_hex"], hex_of(direct.carbon_supplied_mol));
    assert_eq!(result.as_object().unwrap().len(), 16, "eight values and their hex");
    let invariants = &body["invariants"];
    assert_eq!(invariants["carbon_residual_hex"], hex_of(direct.carbon_residual));
    assert_eq!(invariants["electron_residual_hex"], hex_of(direct.electron_residual));
    assert_eq!(invariants["settle_gap_hex"], hex_of(direct.settle_gap));
    assert!(direct.carbon_residual <= culture::LEDGER_TOLERANCE);
    assert!(direct.electron_residual <= culture::LEDGER_TOLERANCE);

    // The volume is the request's own arithmetic, before each sample.
    let volume = result["volume_ml"].as_array().unwrap();
    assert!((volume[0].as_f64().unwrap() - 105.5).abs() < 1.0e-9);
    assert!((volume[3].as_f64().unwrap() - (110.0 + 16.875 - 4.0 - 12.9)).abs() < 1.0e-9);

    // The field hash is the count and seven numbers for each segment.
    let mut hasher = Sha256::new();
    hasher.update(28_u64.to_le_bytes());
    for segment in 0..4 {
        for value in [
            direct.biomass_g_per_l[segment],
            direct.glucose_g_per_l[segment],
            direct.galacturonate_g_per_l[segment],
            direct.mucic_acid_g_per_l[segment],
            direct.volume_ml[segment],
            direct.carbon_dioxide_mol[segment],
            direct.oxygen_mol[segment],
        ] {
            hasher.update(value.to_le_bytes());
        }
    }
    let own = hex::encode(hasher.finalize());
    let served = body["field_sha256"].as_str().expect("field hash");
    assert_eq!(served, own);

    // The proof holds, and an edited number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), json!({ "holds": true, "cause": null, "checked": protocol_mcp_gateway::WHOLE_SOLUTION_CHECKED }));
    let mut moved = body.clone();
    let hex = body["result"]["carbon_dioxide_mol_hex"][2].as_str().unwrap();
    moved["result"]["carbon_dioxide_mol_hex"][2] = json!(flip_hex(hex));
    let (_, text) = post(&base, "/verify_solution", &moved).await;
    let report: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["holds"], false);
    assert_eq!(report["cause"], "result");
    let mut ledger = body.clone();
    let hex = body["invariants"]["carbon_residual_hex"].as_str().unwrap();
    ledger["invariants"]["carbon_residual_hex"] = json!(flip_hex(hex));
    let (_, text) = post(&base, "/verify_solution", &ledger).await;
    let report: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["holds"], false);
    assert_eq!(report["cause"], "invariants");

    // The same question again is the same bytes.
    let (_, again) = post(&base, "/solve", &request).await;
    let (_, first) = post(&base, "/solve", &request).await;
    assert_eq!(first, again);
}

#[tokio::test]
async fn a_question_outside_the_card_is_422_with_its_name_and_no_seal() {
    let base = spawn_app().await;
    let cases: Vec<(Value, &str)> = vec![
        (asked(|p| p["biomass_g_per_l"] = json!(0.29)), "BiomassOutOfRange"),
        (asked(|p| p["biomass_g_per_l"] = json!(1.51)), "BiomassOutOfRange"),
        (asked(|p| p["glucose_g_per_l"] = json!(22.01)), "GlucoseOutOfRange"),
        (asked(|p| p["galacturonate_g_per_l"] = json!(3.49)), "GalacturonateOutOfRange"),
        // The starts are the eight fitted cultures' own: the next number outside each end.
        (asked(|p| p["biomass_g_per_l"] = json!(f64::from_bits(0.6_f64.to_bits() - 1))), "BiomassOutOfRange"),
        (asked(|p| p["biomass_g_per_l"] = json!(f64::from_bits(1.034_f64.to_bits() + 1))), "BiomassOutOfRange"),
        (asked(|p| p["glucose_g_per_l"] = json!(f64::from_bits(20.097_f64.to_bits() - 1))), "GlucoseOutOfRange"),
        (asked(|p| p["glucose_g_per_l"] = json!(f64::from_bits(21.025_f64.to_bits() + 1))), "GlucoseOutOfRange"),
        (asked(|p| p["galacturonate_g_per_l"] = json!(f64::from_bits(4.146_f64.to_bits() - 1))), "GalacturonateOutOfRange"),
        (asked(|p| p["galacturonate_g_per_l"] = json!(f64::from_bits(4.745_f64.to_bits() + 1))), "GalacturonateOutOfRange"),
        // The broth a ten-millionth of a mL above 122.15 mL, the fitted cultures' most by 215 h.
        (
            asked(|p| {
                p["segment_end_h"] = json!([22.0, 44.0]);
                p["feed_ml"] = json!([0.0, 4.4]);
                p["water_ml"] = json!([4.18, 3.570_000_1]);
                p["sample_ml"] = json!([0.0, 0.0]);
            }),
            "VolumeOutOfRange",
        ),
        (asked(|p| p["feed_glucose_g_per_l"] = json!(29.374)), "FeedGlucoseOutOfRange"),
        (asked(|p| p["feed_glucose_g_per_l"] = json!(34.677)), "FeedGlucoseOutOfRange"),
        // The range of the scored run, and the four held-back feeds, are refused today.
        (asked(|p| p["feed_glucose_g_per_l"] = json!(9.0)), "FeedGlucoseOutOfRange"),
        (asked(|p| p["feed_glucose_g_per_l"] = json!(54.0)), "FeedGlucoseOutOfRange"),
        (asked(|p| p["feed_glucose_g_per_l"] = json!(10.077)), "FeedGlucoseOutOfRange"),
        (asked(|p| p["feed_glucose_g_per_l"] = json!(11.06)), "FeedGlucoseOutOfRange"),
        (asked(|p| p["feed_glucose_g_per_l"] = json!(48.32)), "FeedGlucoseOutOfRange"),
        (asked(|p| p["feed_glucose_g_per_l"] = json!(52.101)), "FeedGlucoseOutOfRange"),
        // A record past 215 h, as every measured culture's whole record is.
        (asked(|p| to(p, 215.01)), "RunTooLong"),
        (
            asked(|p| {
                p["segment_end_h"] = json!([21.5, 45.0, 140.0, 240.0, 320.0]);
                p["feed_ml"] = json!([0.0, 2.25, 23.75, 25.0, 0.0]);
                p["water_ml"] = json!([-4.5, -0.75, -19.0, -20.0, 0.0]);
                p["sample_ml"] = json!([4.3, 4.3, 4.3, 4.3, 4.3]);
            }),
            "RunTooLong",
        ),
        // A culture starved to 215 h and one fed 0.4 mL/h from inoculation,
        // each of which was once answered with a proof.
        (
            asked(|p| {
                p["segment_end_h"] = json!([24.0, 48.0, 72.0, 96.0, 120.0, 144.0, 168.0, 192.0, 215.0]);
                p["feed_ml"] = json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
                p["water_ml"] = json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
                p["sample_ml"] = json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4.3]);
                p["glucose_g_per_l"] = json!(20.8);
            }),
            "FeedScheduleOutOfRange",
        ),
        (
            asked(|p| {
                p["segment_end_h"] = json!([100.0]);
                p["feed_ml"] = json!([40.0]);
                p["water_ml"] = json!([-15.0]);
                p["sample_ml"] = json!([0.0]);
            }),
            "FeedRateOutOfRange",
        ),
        // The feed stopped once full, cut back, started before 21.18 h, and one
        // even rate from the start window into the full feed.
        (asked(|p| p["feed_ml"] = json!([0.0, 2.25, 7.5, 0.0])), "FeedScheduleOutOfRange"),
        (asked(|p| p["feed_ml"] = json!([0.0, 2.25, 5.5, 7.125])), "FeedScheduleOutOfRange"),
        (asked(|p| p["feed_ml"] = json!([0.5, 2.25, 7.5, 7.125])), "FeedScheduleOutOfRange"),
        (asked(|p| p["segment_end_h"] = json!([21.5, 45.25, 69.0, 93.5])), "FeedScheduleOutOfRange"),
        // The dilution inputs past the fitted spans, at the old ends of 0.7.0's predecessor.
        (asked(|p| p["water_ml"] = json!([-4.5, -0.75, 7.2, -0.25])), "WaterRateOutOfRange"),
        (asked(|p| p["sample_ml"] = json!([4.3, 4.3, 5.0, 4.3])), "SampleOutOfRange"),
        (asked(|p| p["feed_galacturonate_g_per_l"] = json!(110.5)), "FeedGalacturonateOutOfRange"),
        (asked(|p| p["feed_ml"] = json!([0.0, 2.25, 7.5])), "SegmentsDoNotMatch"),
        (asked(|p| p["segment_end_h"] = json!([21.5, 22.0, 69.0, 93.5])), "SegmentLengthOutOfRange"),
        (asked(|p| p["segment_end_h"] = json!([21.5, 45.0, 40.0, 93.5])), "SegmentLengthOutOfRange"),
        (asked(|p| p["feed_ml"] = json!([0.0, 9.6, 7.5, 7.125])), "FeedRateOutOfRange"),
        (asked(|p| p["water_ml"] = json!([-8.7, -0.75, 1.5, -0.25])), "WaterRateOutOfRange"),
        (asked(|p| p["sample_ml"] = json!([4.3, 4.3, 4.31, 4.3])), "SampleOutOfRange"),
        (asked(|p| p["water_ml"] = json!([-6.0, -6.0, 1.5, -0.25])), "VolumeOutOfRange"),
        // One segment 5e-11 h short of an hour: the 1 h end has its own allowance, 1e-12 h.
        (
            asked(|p| {
                p["segment_end_h"] = json!([20.0, 20.999_999_999_95]);
                p["feed_ml"] = json!([0.0, 0.0]);
                p["water_ml"] = json!([0.0, 0.0]);
                p["sample_ml"] = json!([0.0, 0.0]);
            }),
            "SegmentLengthOutOfRange",
        ),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(name), "{name}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // The feed is one for the whole record: there is no key to change it part way.
    let (status, text) =
        post(&base, "/solve", &asked(|p| p["feed_glucose_g_per_l"] = json!([32.0, 33.0, 33.0, 33.0]))).await;
    assert_eq!(status, 422, "{text}");
    assert!(!text.contains("solution_seal"), "{text}");
    // A constant of the organism is not an input, and every input is required.
    let (status, text) = post(&base, "/solve", &asked(|p| p["biomass_yield_g_per_g"] = json!(0.6))).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, "Physical Violation: 'biomass_yield_g_per_g' is not an input of this solver. The organism's constants, the reactions, the 110 mL charge, pH 4.0 and 35 °C, the step and the tolerances stay inside the solver.");
    let (status, text) = post(&base, "/solve", &asked(|p| p["ph"] = json!(3.5))).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'ph' is not an input of this solver"), "{text}");
    for key in ["biomass_g_per_l", "feed_glucose_g_per_l", "water_ml", "sample_ml"] {
        let request = asked(|p| {
            p.as_object_mut().unwrap().remove(key);
        });
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{text}");
        assert_eq!(text, format!("Missing required parameter: '{key}'"));
    }
    // The exact decimal ends are answered: the feeds of the fitted cultures, a
    // record that ends at 215 h, a feed at exactly 0.331 mL/h over 7.3 h, and
    // a feed that begins at exactly 21.18 h at 0.2234 mL/h to 43.45 h.
    for request in [
        asked(|p| p["feed_glucose_g_per_l"] = json!(29.375)),
        asked(|p| p["feed_glucose_g_per_l"] = json!(34.676)),
        asked(|p| p["biomass_g_per_l"] = json!(0.6)),
        asked(|p| p["biomass_g_per_l"] = json!(1.034)),
        asked(|p| p["glucose_g_per_l"] = json!(20.097)),
        asked(|p| p["glucose_g_per_l"] = json!(21.025)),
        asked(|p| p["galacturonate_g_per_l"] = json!(4.146)),
        asked(|p| p["galacturonate_g_per_l"] = json!(4.745)),
        // The broth exactly at 122.15 mL: 110 + 4.18 + 4.4 + 3.57.
        asked(|p| {
            p["segment_end_h"] = json!([22.0, 44.0]);
            p["feed_ml"] = json!([0.0, 4.4]);
            p["water_ml"] = json!([4.18, 3.57]);
            p["sample_ml"] = json!([0.0, 0.0]);
        }),
        asked(|p| to(p, 215.0)),
        asked(|p| {
            p["segment_end_h"] = json!([21.5, 45.0, 52.3, 93.5]);
            p["feed_ml"] = json!([0.0, 2.25, 0.331 * 7.3, 11.0]);
            p["water_ml"] = json!([-4.5, -0.75, 0.0, -0.25]);
            p["sample_ml"] = json!([4.3, 4.3, 0.0, 4.3]);
        }),
        asked(|p| {
            p["segment_end_h"] = json!([21.18, 43.45, 69.0, 93.5]);
            p["feed_ml"] = json!([0.0, 0.2234 * (43.45 - 21.18), 7.5, 7.125]);
        }),
    ] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{request} {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{text}");
        let segments = request["parameters"]["segment_end_h"].as_array().unwrap().len();
        assert_eq!(body["result"]["biomass_g_per_l"].as_array().unwrap().len(), segments);
    }
}

#[tokio::test]
async fn the_description_says_what_is_fitted_and_its_example_runs_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 13);
    assert_eq!(solvers[10]["solver"], "culture_trichoderma_ambr250");
    let summary = solvers[10]["summary"].as_str().expect("a sentence");
    assert!(summary.contains("six constants are fitted to this organism's own measurements"), "{summary}");
    assert!(summary.contains("it met three of its six lines and missed three (galacturonate, mucic acid and carbon dioxide), a second scoring confirmed it, and it is not validated against measurement."), "{summary}");
    assert!(summary.ends_with("Since then it refuses by name a feed outside the fitted 29.375 to 34.676 g/L glucose and a record longer than 215 h, so the four held-back questions are refused today, and since 0.7.0 a starting biomass, glucose or galacturonate, a feed schedule, water, sample or broth volume outside the fitted cultures' own."), "{summary}");

    let (status, text) =
        post(&base, "/describe_solver", &json!({ "solver": "culture_trichoderma_ambr250" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["summary"], summary);
    for part in ["object", "data", "pass_line", "result", "limits"] {
        let words = described["validation"][part].as_str().unwrap_or_else(|| panic!("{part}"));
        assert!(!words.is_empty(), "{part}");
    }
    let limits = described["validation"]["limits"].as_str().unwrap();
    assert!(limits.contains("extrapolation in the glucose of the feed"), "{limits}");
    assert!(limits.contains("The carbon and electron ledgers close for any constants: they check the arithmetic."));
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.contains("68b833108c589ce7a6e063ec84b21483fa50c7cffa145108e69ed0de816c92db"));
    assert!(pass_line.contains("is not evidence for the model"));
    let result = described["validation"]["result"].as_str().unwrap();
    for words in [
        "it did not pass: three of the six lines were met and three were missed",
        "galacturonate, root-mean-square error 4.60 g/L against 2.74 allowed",
        "mucic acid, 6.45 g/L against 4.51",
        "carbon dioxide made, 0.01125 mol against 0.00265",
        "only the oxygen line is evidence for the model",
        "It therefore misses the glucose and the carbon dioxide lines on its own fitted data",
        "572ce3bedbb195b637045a60c5e2452f5b939315fc6fa699a9545949c5c63f71",
        "evidence/checks/second_party/V-41",
        "The form fails, not the constants",
        "4.30 g/L, 5.21 g/L and 0.0108 mol",
        "one level cannot identify any dependence of galacturonate oxidation on the glucose supplied",
        "five to six times the line of 0.00265",
        "3.33 g/L in galacturonate and 0.0063 mol in carbon dioxide",
        "the held-back error is 10.6 g/L in place of 6.45",
        "are therefore refused by this build",
        "and on seven of them 0.0088 and 0.0111 mol of gas: T2 pH 4 (2) has a blank online volume",
        "FeedScheduleOutOfRange",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let data = described["validation"]["data"].as_str().unwrap();
    assert!(data.contains("Zenodo record 5846028, CC BY 4.0"));
    assert!(data.contains("The measurements do not close on their own"));
    assert!(data.contains("0.78 to 1.15 between 40 and 215 h"), "{data}");
    assert_eq!(described["locked"]["glucose_uptake_max_g_per_g_h"], culture::GLUCOSE_UPTAKE_MAX_G_PER_G_H);
    assert_eq!(described["locked"]["mucic_acid_fraction"], culture::MUCIC_ACID_FRACTION);
    assert_eq!(described["locked"]["range_end_allowance"], culture::RANGE_END_ALLOWANCE);
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 9);
    assert_eq!(inputs[3]["name"], "feed_glucose_g_per_l");
    assert_eq!(inputs[3]["minimum"], culture::FEED_GLUCOSE_MIN_G_PER_L);
    assert_eq!(inputs[3]["maximum"], culture::FEED_GLUCOSE_MAX_G_PER_L);
    assert_eq!((inputs[3]["minimum"].as_f64(), inputs[3]["maximum"].as_f64()), (Some(29.375), Some(34.676)));
    assert_eq!(inputs[5]["name"], "segment_end_h");
    assert!(inputs[5]["bound"].as_str().unwrap().contains("the record ends by 215 h"), "{}", inputs[5]);
    assert!(!inputs[5]["bound"].as_str().unwrap().contains("untested"), "{}", inputs[5]);
    assert_eq!(inputs[6]["name"], "feed_ml");
    assert!(inputs[6]["bound"].as_str().unwrap().contains("none went unfed, below 0.2432 mL/h or above 0.3301 mL/h in those segments once its feed had begun"), "{}", inputs[6]);
    assert_eq!(inputs[8]["name"], "sample_ml");
    assert_eq!(inputs[8]["maximum"], culture::SAMPLE_MAX_ML);
    assert_eq!(inputs[8]["maximum"], 4.3);
    // Each output says in its own note how its comparison came out.
    let returns = serde_json::to_string(&described["returns"]).unwrap();
    for words in [
        "Not held anywhere: 0.0088 mol",
        "misses it after, which is why the record ends there",
        "Compared with measurement to 170 h only",
        "evidence for the bookkeeping of glucose and not for the kinetics",
    ] {
        assert!(returns.contains(words), "{words}");
    }
    let refusals = serde_json::to_string(&described["refusals"]).unwrap();
    assert!(refusals.contains("through the gateway no input can be, since a number that is not finite is not JSON"));
    assert_eq!(described["refusals"].as_array().expect("refusals").len(), 19);
    assert!(refusals.contains("FeedScheduleOutOfRange"));
    assert_eq!(described["reference"]["url"], "https://zenodo.org/records/5846028");

    let example = described["example_solve"].clone();
    let (status, text) = post(&base, "/solve", &example).await;
    assert_eq!(status, 200, "{example} {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["outcome"], "converged");
    let glucose = body["result"]["glucose_g_per_l"].as_array().unwrap();
    assert!(glucose[0].as_f64().unwrap() > 10.0 && glucose[2].as_f64().unwrap() < 1.0, "{glucose:?}");
    let gas = body["result"]["carbon_dioxide_mol"].as_array().unwrap();
    assert!(gas[2].as_f64().unwrap() > 0.03 && gas[2].as_f64().unwrap() < 0.08, "{gas:?}");
}
