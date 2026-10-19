//! `culture_trichoderma_ambr250` through `POST /solve`, against a direct call
//! to `genesis_solvers`, with its refusals at the door and its description.

mod answer_text;

use std::time::Duration;

use genesis_solvers::culture_trichoderma_ambr250::{self as culture, Question};
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
    let mut hasher = Sha384::new();
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
    let served = body["field_sha384"].as_str().expect("field hash");
    assert_eq!(served, own);

    // The proof holds, and an edited number breaks it by name.
    let (status, text) = post(&base, "/verify_solution", &body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
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
    let below_zero = -f64::MIN_POSITIVE;
    let cases: Vec<(Value, &str)> = vec![
        // A concentration below zero, by the least normal number below it.
        (asked(|p| p["biomass_g_per_l"] = json!(below_zero)), "BiomassNegative"),
        (asked(|p| p["glucose_g_per_l"] = json!(below_zero)), "GlucoseNegative"),
        (asked(|p| p["galacturonate_g_per_l"] = json!(below_zero)), "GalacturonateNegative"),
        (asked(|p| p["feed_glucose_g_per_l"] = json!(below_zero)), "FeedGlucoseNegative"),
        (asked(|p| p["feed_galacturonate_g_per_l"] = json!(-1.0)), "FeedGalacturonateNegative"),
        // The broth a ten-millionth of a mL above 250 mL, what the vessel holds.
        (
            asked(|p| {
                p["segment_end_h"] = json!([22.0, 44.0]);
                p["feed_ml"] = json!([0.0, 4.4]);
                p["water_ml"] = json!([100.0, 35.600_000_1]);
                p["sample_ml"] = json!([0.0, 0.0]);
            }),
            "VolumeOutOfRange",
        ),
        // A feed that overfills the vessel: 0.4 mL/h from inoculation to 400 h.
        (
            asked(|p| {
                p["segment_end_h"] = json!([400.0]);
                p["feed_ml"] = json!([160.0]);
                p["water_ml"] = json!([-15.0]);
                p["sample_ml"] = json!([0.0]);
            }),
            "VolumeOutOfRange",
        ),
        // A record past 1000 h, the capacity of one solve.
        (asked(|p| to(p, 1000.01)), "RunTooLong"),
        (asked(|p| p["feed_ml"] = json!([0.0, 2.25, 7.5])), "SegmentsDoNotMatch"),
        (asked(|p| p["segment_end_h"] = json!([21.5, 21.5, 69.0, 93.5])), "SegmentNotRising"),
        (asked(|p| p["segment_end_h"] = json!([21.5, 45.0, 40.0, 93.5])), "SegmentNotRising"),
        (asked(|p| p["feed_ml"] = json!([0.0, -0.01, 7.5, 7.125])), "FeedRateNegative"),
        (asked(|p| p["sample_ml"] = json!([4.3, 4.3, -0.01, 4.3])), "SampleNegative"),
        // Broth drawn to nothing, and evaporated to nothing.
        (asked(|p| p["sample_ml"] = json!([4.3, 4.3, 120.0, 4.3])), "VolumeOutOfRange"),
        (asked(|p| p["water_ml"] = json!([-110.0, -0.75, 1.5, -0.25])), "VolumeOutOfRange"),
    ];
    for (request, name) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.contains(name), "{name}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
        assert!(!text.contains("..="), "{text}");
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
        assert_eq!(text, format!("Missing required parameter: '{key}' (InputMissing)."));
    }
    // The exact decimal ends are answered: the feeds of the fitted cultures, a
    // record that ends at 215 h, a feed at exactly 0.331 mL/h over 7.3 h, and
    // a feed that begins at exactly 21.18 h at 0.2234 mL/h to 43.45 h. And
    // past every wall engine 0.12.0 held at the twelve compared cultures'
    // own starts, feeds, schedule and record end: no biomass and 0.29 to
    // 1.51 g/L, the scored run's feed ends, 9 and 54 g/L, the next numbers
    // outside the compared starts and feeds, a starved culture, a feed of
    // 0.4 mL/h from inoculation, a feed stopped, cut back or started early,
    // and records to 312.01 and 1000 h. Each is answered with its ledgers
    // closed and a proof that holds.
    for request in [
        asked(|p| p["biomass_g_per_l"] = json!(0.0)),
        asked(|p| p["biomass_g_per_l"] = json!(0.29)),
        asked(|p| p["biomass_g_per_l"] = json!(1.51)),
        asked(|p| p["glucose_g_per_l"] = json!(0.0)),
        asked(|p| p["glucose_g_per_l"] = json!(22.01)),
        asked(|p| p["galacturonate_g_per_l"] = json!(3.49)),
        asked(|p| p["galacturonate_g_per_l"] = json!(f64::from_bits(4.774_f64.to_bits() + 1))),
        asked(|p| p["feed_glucose_g_per_l"] = json!(9.0)),
        asked(|p| p["feed_glucose_g_per_l"] = json!(54.0)),
        asked(|p| p["feed_glucose_g_per_l"] = json!(f64::from_bits(10.077_f64.to_bits() - 1))),
        asked(|p| p["feed_galacturonate_g_per_l"] = json!(110.5)),
        asked(|p| p["feed_galacturonate_g_per_l"] = json!(0.0)),
        asked(|p| {
            p["segment_end_h"] = json!([24.0, 48.0, 72.0, 96.0, 120.0, 144.0, 168.0, 192.0, 215.0]);
            p["feed_ml"] = json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
            p["water_ml"] = json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
            p["sample_ml"] = json!([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 4.3]);
            p["glucose_g_per_l"] = json!(20.8);
        }),
        asked(|p| {
            p["segment_end_h"] = json!([100.0]);
            p["feed_ml"] = json!([40.0]);
            p["water_ml"] = json!([-15.0]);
            p["sample_ml"] = json!([0.0]);
        }),
        asked(|p| p["feed_ml"] = json!([0.0, 2.25, 7.5, 0.0])),
        asked(|p| p["feed_ml"] = json!([0.0, 2.25, 5.5, 7.125])),
        asked(|p| p["feed_ml"] = json!([0.5, 2.25, 7.5, 7.125])),
        asked(|p| p["segment_end_h"] = json!([21.5, 45.25, 69.0, 93.5])),
        asked(|p| p["feed_ml"] = json!([0.0, 9.6, 7.5, 7.125])),
        asked(|p| to(p, 312.01)),
        asked(|p| to(p, 1000.0)),
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
        // Past the fitted cultures' spans, where engine 0.8.0 refused by name:
        // the held-back feeds, a record to 312 h, water and samples past their
        // old spans, broth below 98 and above 122.15 mL, a segment of 5e-11 h
        // short of an hour.
        asked(|p| p["feed_glucose_g_per_l"] = json!(10.077)),
        asked(|p| p["feed_glucose_g_per_l"] = json!(11.06)),
        asked(|p| p["feed_glucose_g_per_l"] = json!(48.32)),
        asked(|p| p["feed_glucose_g_per_l"] = json!(52.101)),
        asked(|p| p["feed_glucose_g_per_l"] = json!(29.374)),
        asked(|p| p["biomass_g_per_l"] = json!(1.067)),
        asked(|p| p["glucose_g_per_l"] = json!(21.151)),
        asked(|p| p["galacturonate_g_per_l"] = json!(4.774)),
        asked(|p| to(p, 215.01)),
        asked(|p| to(p, 312.0)),
        asked(|p| p["water_ml"] = json!([-4.5, -0.75, 7.2, -0.25])),
        asked(|p| p["water_ml"] = json!([-8.7, -0.75, 1.5, -0.25])),
        asked(|p| p["water_ml"] = json!([-6.0, -6.0, 1.5, -0.25])),
        asked(|p| p["sample_ml"] = json!([4.3, 4.3, 5.0, 4.3])),
        asked(|p| {
            p["segment_end_h"] = json!([22.0, 44.0]);
            p["feed_ml"] = json!([0.0, 4.4]);
            p["water_ml"] = json!([4.18, 3.570_000_1]);
            p["sample_ml"] = json!([0.0, 0.0]);
        }),
        asked(|p| {
            p["segment_end_h"] = json!([20.0, 20.999_999_999_95]);
            p["feed_ml"] = json!([0.0, 0.0]);
            p["water_ml"] = json!([0.0, 0.0]);
            p["sample_ml"] = json!([0.0, 0.0]);
        }),
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
        assert!(body["invariants"]["carbon_residual"].as_f64().unwrap() <= culture::LEDGER_TOLERANCE);
        let (status, checked) = post(&base, "/verify_solution", &body).await;
        assert_eq!(status, 200, "{checked}");
        assert_eq!(serde_json::from_str::<Value>(&checked).unwrap()["holds"], true, "{checked}");
    }
}

#[tokio::test]
async fn the_description_says_what_is_fitted_and_its_example_runs_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), protocol_mcp_gateway::SOLVERS);
    assert_eq!(solvers[10]["solver"], "culture_trichoderma_ambr250");
    // Since 0.14.4 a row gives what the model computes, the comparison and
    // the gap; the whole spec sheet is the description's.
    let row_summary = solvers[10]["summary"].as_str().expect("a sentence");
    assert!(!row_summary.contains("Range: "), "{row_summary}");
    let (_, full) = post(&base, "/describe_solver", &json!({ "solver": "culture_trichoderma_ambr250" })).await;
    let full: Value = serde_json::from_str(&full).unwrap();
    let summary = full["summary"].as_str().unwrap().to_string();
    let summary = summary.as_str();
    assert!(summary.starts_with(row_summary.split_once(" Compared with ").map_or(row_summary, |parts| parts.0)), "{summary}");
    assert!(summary.contains("Fitted on 8 of the 12 cultures, fed 29.375 to 34.676 g/L glucose with 0 to 2 g/L yeast extract: six constants."), "{summary}");
    assert!(summary.contains("oxygen taken up within 0.0108 mol (target 0.0153), dry biomass to 170 h within 1.05 g/L (target 2.15), glucose within 0.51 g/L (target 0.85); galacturonate 4.60 g/L RMS (target 2.74), mucic acid 6.45 g/L (target 4.51), carbon dioxide 0.01125 mol (target 0.00265)."), "{summary}");
    assert!(summary.contains("Range: compared on the twelve cultures' own starts, feeds (glucose 10.077 to 52.101 g/L) and schedule, to 312 h"), "{summary}");

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
    assert!(limits.contains("extend the fit in the glucose of the feed"), "{limits}");
    assert!(limits.contains("The carbon and electron ledgers close for any constants: they test the arithmetic."));
    let pass_line = described["validation"]["pass_line"].as_str().unwrap();
    assert!(pass_line.contains("Record SHA-256 68b833108c589ce7a6e063ec84b21483fa50c7cffa145108e69ed0de816c92db."));
    assert!(pass_line.contains("dry biomass to 170 h at most 2.15 g/L; glucose 0.85 g/L; galacturonate 2.74 g/L; mucic acid 4.51 g/L; carbon dioxide made 0.00265 mol; oxygen taken up 0.0153 mol"));
    assert!(pass_line.contains("Four baselines are computed on the same points"));
    let result = described["validation"]["result"].as_str().unwrap();
    for words in [
        "oxygen taken up 0.0108 mol (target 0.0153); dry biomass to 170 h 1.05 g/L (target 2.15); glucose 0.51 g/L (target 0.85)",
        "galacturonate 4.60 g/L (target 2.74)",
        "mucic acid 6.45 g/L (target 4.51)",
        "carbon dioxide made 0.01125 mol (target 0.00265)",
        "Oxygen is within its target where the baselines are not",
        "On its own fitted data, glucose and carbon dioxide are above their targets",
        "572ce3bedbb195b637045a60c5e2452f5b939315fc6fa699a9545949c5c63f71",
        "A second, independent run recomputes every number of the comparison",
        "The form, not the constants, sets the gap",
        "4.30 g/L, 5.21 g/L and 0.0108 mol",
        "one level cannot identify any dependence of galacturonate oxidation on the glucose supplied",
        "five to six times the target of 0.00265",
        "3.33 g/L in galacturonate and 0.0063 mol in carbon dioxide",
        "the error outside the fit is 10.6 g/L in place of 6.45",
        "The solver answers the four questions of the comparison, fed 10.077, 11.06, 48.32 and 52.101 g/L",
        "and on seven of them 0.0088 and 0.0111 mol of gas: T2 pH 4 (2) has a blank online volume",
        "slowed to as little as 0.009 mL/h from 211.28 h",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let data = described["validation"]["data"].as_str().unwrap();
    assert!(data.contains("Zenodo record 5846028"));
    assert!(data.contains("The measurements do not close on their own"));
    assert!(data.contains("0.78 to 1.15 between 40 and 215 h"), "{data}");
    assert_eq!(described["locked"]["glucose_uptake_max_g_per_g_h"], culture::GLUCOSE_UPTAKE_MAX_G_PER_G_H);
    assert_eq!(described["locked"]["mucic_acid_fraction"], culture::MUCIC_ACID_FRACTION);
    assert_eq!(described["locked"]["range_end_allowance"], culture::RANGE_END_ALLOWANCE);
    let inputs = described["inputs"].as_array().expect("inputs");
    assert_eq!(inputs.len(), 9);
    assert_eq!(inputs[3]["name"], "feed_glucose_g_per_l");
    assert_eq!((inputs[3]["minimum"].as_f64(), inputs[3]["maximum"].as_f64()), (Some(0.0), Some(f64::MAX)));
    assert_eq!(culture::COMPARED_FEED_GLUCOSE_G_PER_L, (10.077, 52.101));
    assert!(inputs[3]["bound"].as_str().unwrap().contains("The twelve compared cultures were fed 10.077 to 52.101 g/L"), "{}", inputs[3]);
    assert_eq!(inputs[5]["name"], "segment_end_h");
    assert!(inputs[5]["bound"].as_str().unwrap().contains("the record ends by 1000 h (RunTooLong past it), the capacity of one solve and no physics"), "{}", inputs[5]);
    assert_eq!(inputs[5]["maximum"], culture::RUN_END_MAX_H);
    assert!(!inputs[5]["bound"].as_str().unwrap().contains("untested"), "{}", inputs[5]);
    assert_eq!(inputs[6]["name"], "feed_ml");
    assert!(inputs[6]["bound"].as_str().unwrap().contains("from 211.28 h, when every feed was slowed and then stopped, 0.0093 to 0.2601 mL/h"), "{}", inputs[6]);
    assert_eq!(inputs[8]["name"], "sample_ml");
    assert_eq!(inputs[8]["maximum"], culture::VOLUME_MAX_ML);
    assert_eq!(inputs[8]["maximum"], 250.0);
    // Each output says in its own note how its comparison came out.
    let returns = serde_json::to_string(&described["returns"]).unwrap();
    for words in [
        "0.0088 mol RMS on seven of the fitted cultures",
        "and above it after, to the records' end at 312 h",
        "Compared with measurement to 170 h:",
        "it tests the bookkeeping of glucose and not the kinetics",
    ] {
        assert!(returns.contains(words), "{words}");
    }
    let refusals = serde_json::to_string(&described["refusals"]).unwrap();
    assert!(refusals.contains("through the gateway no input can be, since a number that is not finite is not JSON"));
    assert_eq!(described["refusals"].as_array().expect("refusals").len(), 17);
    assert!(!refusals.contains("FeedScheduleOutOfRange") && !refusals.contains("FeedRateOutOfRange"));
    assert!(refusals.contains("FeedRateNegative") && refusals.contains("BiomassNegative"));
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
