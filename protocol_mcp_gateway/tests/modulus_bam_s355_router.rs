//! The bar solver `modulus_bam_s355` through `solve` and `verify_solution`:
//! against a direct call to `genesis_solvers`, at each of its limits, under
//! edits of a sealed solution, and on the three connections.
//!
//! The frequencies are the four published scans of the bars ERX1 and ERX2
//! (Zenodo, doi 10.5281/zenodo.6778336). The bars cut across the rolling
//! direction were the held-back comparison; their four scans, opened on
//! 2026-10-09, are used here only where the declared ranges are checked
//! against the eight scans compared with measurement.

mod answer_text;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use genesis_solvers::modulus_bam_s355::{self as modulus, Bar, Mode, Reading};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const SOLVER: &str = "modulus_bam_s355";

/// bar, flatwise, edgewise, torsion (Hz), as the laboratory published them.
const SCANS: [(&str, f64, f64, f64); 4] = [
    ("ERX1", 1588.627, 4700.015, 8950.895),
    ("ERX1", 1587.428, 4698.493, 8944.466),
    ("ERX2", 1598.579, 4704.533, 8899.446),
    ("ERX2", 1598.747, 4707.067, 8901.011),
];

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post_text(base: &str, path: &str, text: &str) -> (u16, String) {
    let client = Client::new();
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(text.to_string())
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

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    post_text(base, path, &answer_text::as_sent(body)).await
}

/// A question: the bar, and its series in the order flatwise, torsion, edgewise.
fn question(bar: &str, frequencies_hz: &[f64]) -> Value {
    json!({ "solver": SOLVER, "parameters": { "bar": bar, "frequencies_hz": frequencies_hz } })
}

fn scan(index: usize) -> Value {
    let (bar, flatwise, edgewise, torsion) = SCANS[index];
    question(bar, &[flatwise, torsion, edgewise])
}

fn direct(index: usize) -> modulus::BarModuli {
    let (bar, flatwise, edgewise, torsion) = SCANS[index];
    modulus::solve(
        Bar::from_name(bar).unwrap(),
        &[
            Reading { mode: Mode::Flatwise, frequency_hz: flatwise },
            Reading { mode: Mode::Edgewise, frequency_hz: edgewise },
            Reading { mode: Mode::Torsion, frequency_hz: torsion },
        ],
    )
    .expect("a published scan is solved")
}

async fn solved(base: &str, request: &Value) -> (String, Value) {
    let (status, text) = post(base, "/solve", request).await;
    assert_eq!(status, 200, "{text}");
    let body = serde_json::from_str(&text).unwrap();
    (text, body)
}

async fn verdict(base: &str, body: &Value) -> Value {
    let (status, text) = post(base, "/verify_solution", body).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn assert_fails(base: &str, body: &Value, part: &str, label: &str) {
    let report = verdict(base, body).await;
    assert_eq!(report["holds"], false, "{label}");
    assert_eq!(report["cause"], part, "{label}");
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

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The receipt covers the wire bytes before it.
fn assert_receipt(text: &str, body: &Value) {
    let receipt = body["receipt_sha256"].as_str().expect("receipt");
    let tail = format!(",\"receipt_sha256\":\"{receipt}\"}}");
    let document = text.strip_suffix(&tail).expect("the receipt is last");
    assert_eq!(sha256_hex(format!("{document}}}").as_bytes()), receipt);
}

#[tokio::test]
async fn the_four_published_scans_match_the_solver_bit_for_bit() {
    let base = spawn_app().await;
    for index in 0..SCANS.len() {
        let request = scan(index);
        let (text, body) = solved(&base, &request).await;
        let direct = direct(index);

        assert_eq!(body["inputs"], request);
        assert_eq!(body["engine_version"], "genesis_solvers_v0.12.0");
        assert_eq!(body["outcome"], "converged");
        assert!(body["reject"].is_null());
        assert!(body["reject_detail"].is_null());
        assert_eq!(body["iterations"], direct.iterations);

        let result = &body["result"];
        assert_number(result, "youngs_modulus_pa", direct.youngs_modulus_pa);
        assert_number(result, "shear_modulus_pa", direct.shear_modulus_pa);
        assert_number(
            result,
            "youngs_modulus_method_uncertainty_pa",
            direct.youngs_modulus_method_uncertainty_pa,
        );
        assert_number(
            result,
            "shear_modulus_method_uncertainty_pa",
            direct.shear_modulus_method_uncertainty_pa,
        );
        assert_number(
            result,
            "shear_modulus_method_uncertainty_ceiling_pa",
            direct.shear_modulus_method_uncertainty_ceiling_pa,
        );
        // one percent of the value from the data set's own sheet, three from the later paper
        assert_eq!(direct.shear_modulus_method_uncertainty_pa, 0.01 * direct.shear_modulus_pa);
        assert_eq!(direct.shear_modulus_method_uncertainty_ceiling_pa, 0.03 * direct.shear_modulus_pa);
        assert_number(result, "edgewise_predicted_hz", direct.edgewise_predicted_hz);
        assert_number(
            result,
            "edgewise_residual_fraction",
            direct.edgewise_residual_fraction.expect("edgewise was supplied"),
        );
        assert_number(result, "flatwise_residual_fraction", direct.flatwise_residual_fraction);
        assert_number(result, "torsion_residual_fraction", direct.torsion_residual_fraction);
        assert_number(result, "last_step", direct.last_step);
        assert_eq!(result.as_object().unwrap().len(), 20);

        let invariants = &body["invariants"];
        assert_number(invariants, "flatwise_energy_balance", direct.flatwise_energy_balance);
        assert_number(invariants, "edgewise_energy_balance", direct.edgewise_energy_balance);
        assert_number(invariants, "flatwise_free_end", direct.flatwise_free_end);
        assert_number(invariants, "edgewise_free_end", direct.edgewise_free_end);
        assert_eq!(invariants.as_object().unwrap().len(), 8);
        for key in ["flatwise_energy_balance", "edgewise_energy_balance"] {
            assert!(invariants[key].as_f64().unwrap().abs() <= modulus::ENERGY_BALANCE_TOLERANCE);
        }

        // The field hash: the sample count, then the two modes the solver returned.
        let mut bytes = (direct.mode_shapes.len() as u64).to_le_bytes().to_vec();
        for sample in &direct.mode_shapes {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        assert_eq!(direct.mode_shapes.len(), 2 * 4 * (modulus::MODE_INTERVALS + 1));
        assert_eq!(body["field_sha256"], sha256_hex(&bytes));

        // The steel of the sheet: 209 to 213 GPa and 78 to 81 GPa, and the
        // resonance that was not used is predicted within 0.25 %.
        let e_gpa = result["youngs_modulus_pa"].as_f64().unwrap() / 1.0e9;
        let g_gpa = result["shear_modulus_pa"].as_f64().unwrap() / 1.0e9;
        assert!((209.0..213.0).contains(&e_gpa), "{e_gpa}");
        assert!((78.0..81.0).contains(&g_gpa), "{g_gpa}");
        assert!(result["edgewise_residual_fraction"].as_f64().unwrap().abs() < 0.0025);

        assert_receipt(&text, &body);
        assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    }
}

#[tokio::test]
async fn the_series_without_the_edgewise_frequency_is_answered_without_its_residual() {
    let base = spawn_app().await;
    let (bar, flatwise, _, torsion) = SCANS[0];
    let (_, body) = solved(&base, &question(bar, &[flatwise, torsion])).await;
    assert_eq!(body["outcome"], "converged");
    let result = body["result"].as_object().unwrap();
    assert!(!result.contains_key("edgewise_residual_fraction"));
    assert!(!result.contains_key("edgewise_residual_fraction_hex"));
    assert_eq!(result.len(), 18);
    // The moduli do not depend on whether the edgewise frequency came along.
    let (_, whole) = solved(&base, &scan(0)).await;
    for key in ["youngs_modulus_pa_hex", "shear_modulus_pa_hex", "edgewise_predicted_hz_hex"] {
        assert_eq!(body["result"][key], whole["result"][key], "{key}");
    }
    assert_ne!(body["solution_seal"], whole["solution_seal"]);
    assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));

    // A solution cannot gain the residual after it was sealed.
    let mut gained = body.clone();
    gained["result"]["edgewise_residual_fraction_hex"] = whole["result"]["edgewise_residual_fraction_hex"].clone();
    assert_fails(&base, &gained, "result", "a residual added").await;
}

#[tokio::test]
async fn a_request_outside_the_solver_is_422_with_no_seal() {
    let base = spawn_app().await;
    let (bar, flatwise, edgewise, torsion) = SCANS[0];
    let inside = "The bar's size and mass, the start and the tolerances stay inside the solver.";
    let cases: Vec<(Value, String)> = vec![
        (
            json!({ "solver": SOLVER }),
            "Missing required parameter: 'parameters' (or 'example', the name of a worked example describe_solver lists) (InputMissing).".into(),
        ),
        (
            json!({ "solver": SOLVER, "parameters": { "frequencies_hz": [flatwise, torsion] } }),
            "Missing required parameter: 'bar' (InputMissing).".into(),
        ),
        (
            json!({ "solver": SOLVER, "parameters": { "bar": bar } }),
            "Missing required parameter: 'frequencies_hz' (InputMissing).".into(),
        ),
        (
            question("ERX3", &[flatwise, torsion]),
            // Re-pinned for 0.10.1: a bar that is not one of the four carries the name UnknownBar.
            "Physical Violation: 'bar' must be one of \"ERX1\", \"ERX2\", \"ERY1\", \"ERY2\"; this request has \"ERX3\" (UnknownBar).".into(),
        ),
        (
            question("erx1", &[flatwise, torsion]),
            "Physical Violation: 'bar' must be one of \"ERX1\", \"ERX2\", \"ERY1\", \"ERY2\"; this request has \"erx1\" (UnknownBar).".into(),
        ),
        (
            json!({ "solver": SOLVER, "parameters": { "bar": 1, "frequencies_hz": [flatwise, torsion] } }),
            "Physical Violation: 'bar' must be a string; this request has 1.".into(),
        ),
        (
            json!({ "solver": SOLVER, "parameters": { "bar": bar, "frequencies_hz": flatwise } }),
            "Physical Violation: 'frequencies_hz' must be an array of numbers; this request has 1588.627.".into(),
        ),
        (
            json!({ "solver": SOLVER, "parameters": { "bar": bar, "frequencies_hz": ["1588.627", torsion] } }),
            "Physical Violation: 'frequencies_hz[0]' must be a number; this request has \"1588.627\".".into(),
        ),
        // Nothing is filled in: a property of the specimen, a property of the
        // steel or a knob of the search is not an input.
        (
            json!({ "solver": SOLVER, "parameters": { "bar": bar, "frequencies_hz": [flatwise, torsion], "thickness_m": 0.003 } }),
            format!("Physical Violation: 'thickness_m' is not an input of this solver. {inside}"),
        ),
        (
            json!({ "solver": SOLVER, "parameters": { "bar": bar, "frequencies_hz": [flatwise, torsion], "youngs_modulus_pa": 2.1e11 } }),
            format!("Physical Violation: 'youngs_modulus_pa' is not an input of this solver. {inside}"),
        ),
        (
            json!({ "solver": SOLVER, "parameters": { "bar": bar, "frequencies_hz": [flatwise, torsion] }, "max_iterations": 500 }),
            format!("Physical Violation: 'max_iterations' is not an input of this solver. {inside}"),
        ),
        // The limits the series itself shows, in the solver's own names.
        (
            question(bar, &[flatwise]),
            "Physical Violation: the shear modulus is not identifiable: the series holds no torsion frequency (ShearModulusNotIdentifiable).".into(),
        ),
        (
            question(bar, &[]),
            "Physical Violation: Young's modulus is not identifiable: the series holds no flatwise frequency (YoungsModulusNotIdentifiable).".into(),
        ),
        (
            question(bar, &[flatwise, -8950.895]),
            "Physical Violation: torsion frequency -8950.895 Hz is not a finite number above zero (FrequencyNotPositive).".into(),
        ),
        (
            question(bar, &[flatwise, torsion, 0.0]),
            "Physical Violation: edgewise frequency 0 Hz is not a finite number above zero (FrequencyNotPositive).".into(),
        ),
        (
            question(bar, &[flatwise, torsion, edgewise, edgewise]),
            // Re-pinned for 0.10.1: a fourth frequency carries the name TooManyFrequencies.
            "Physical Violation: 'frequencies_hz' holds 4 frequencies, and a series is at most 3: flatwise, torsion, edgewise (TooManyFrequencies).".into(),
        ),
    ];
    for (request, message) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert_eq!(text, message, "{request}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // The ends of the compared ranges, and past them, where engine 0.8.0
    // refused by name: each is answered with a seal that holds, the round
    // trip closed. The corners of the ranges served through engine 0.6.0,
    // and an edgewise frequency past every compared one, among them.
    for request in [
        question(bar, &[1608.28, 8936.544]),
        question(bar, &[1608.2800000000002, 8936.544]),
        question(bar, &[1608.5, torsion]),
        question("ERY1", &[1200.0, 6700.0]),
        question("ERY1", &[1608.28, 8936.544, 4801.5028966]),
        question("ERX1", &[1587.428, 8899.446]),
    ] {
        let (_, body) = solved(&base, &request).await;
        assert_eq!(body["outcome"], "converged", "{request}");
        for mode in ["flatwise", "torsion"] {
            assert!(body["result"][format!("{mode}_residual_fraction")].as_f64().unwrap().abs() <= modulus::ROUND_TRIP_TOLERANCE);
        }
        assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    }
    // A number too large to read and a repeated key never reach the solver.
    for body in [
        format!(r#"{{"solver":"{SOLVER}","parameters":{{"bar":"ERX1","frequencies_hz":[1e999,8950.895]}}}}"#),
        format!(r#"{{"solver":"{SOLVER}","parameters":{{"bar":"ERX1","bar":"ERY1","frequencies_hz":[1588.627,8950.895]}}}}"#),
    ] {
        let (status, text) = post_text(&base, "/solve", &body).await;
        assert!(status == 422 || status == 400, "{status} {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
}

#[tokio::test]
async fn a_limit_the_physics_finds_is_a_sealed_refusal_with_no_result() {
    let base = spawn_app().await;
    let (bar, flatwise, edgewise, torsion) = SCANS[0];
    let _ = edgewise;
    let cases = [
        // An edgewise pitch in the torsion place: no bar the same in every
        // direction rings at that pair.
        (question(bar, &[flatwise, 4700.015, torsion]), "PoissonRatioOutOfRange"),
        // A torsion pitch in the flatwise place: the search leaves the moduli
        // of every solid.
        (question(bar, &[torsion, flatwise]), "OutsideSearchRange"),
    ];
    let mut seals = Vec::new();
    for (request, name) in cases {
        let (text, body) = solved(&base, &request).await;
        assert_eq!(body["outcome"], "refused", "{name}");
        assert_eq!(body["reject"], name);
        let detail = body["reject_detail"].as_str().unwrap();
        assert!(
            detail.starts_with(&format!(
                "Solver Refusal: modulus_bam_s355 did not return the moduli ({name}). "
            )),
            "{detail}"
        );
        assert!(body["result"].is_null());
        assert!(body["invariants"].is_null());
        assert!(body["field_sha256"].is_null());
        assert_receipt(&text, &body);
        assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));

        let mut renamed = body.clone();
        renamed["reject"] = json!("NotConverged");
        assert_fails(&base, &renamed, "outcome", name).await;
        let mut promoted = body.clone();
        promoted["outcome"] = json!("converged");
        assert_fails(&base, &promoted, "outcome", name).await;
        seals.push(body["solution_seal"].as_str().unwrap().to_string());
    }
    assert_ne!(seals[0], seals[1]);
}

#[tokio::test]
async fn every_edit_of_a_sealed_solution_is_named() {
    let base = spawn_app().await;
    let (_, body) = solved(&base, &scan(2)).await;
    assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));

    for key in [
        "youngs_modulus_pa_hex",
        "shear_modulus_pa_hex",
        "edgewise_predicted_hz_hex",
        "edgewise_residual_fraction_hex",
        "last_step_hex",
    ] {
        let mut edited = body.clone();
        edited["result"][key] = json!(flip_hex(body["result"][key].as_str().unwrap()));
        assert_fails(&base, &edited, "result", key).await;
    }
    let mut edited = body.clone();
    edited["invariants"]["flatwise_energy_balance_hex"] =
        json!(flip_hex(body["invariants"]["flatwise_energy_balance_hex"].as_str().unwrap()));
    assert_fails(&base, &edited, "invariants", "energy balance").await;

    // The question: another bar, another frequency, a frequency dropped, two exchanged.
    let mut other_bar = body.clone();
    other_bar["inputs"]["parameters"]["bar"] = json!("ERY2");
    assert_fails(&base, &other_bar, "inputs", "bar").await;
    let mut other_pitch = body.clone();
    other_pitch["inputs"]["parameters"]["frequencies_hz"][0] = json!(1598.5790000000002);
    assert_fails(&base, &other_pitch, "inputs", "frequency").await;
    let mut dropped = body.clone();
    dropped["inputs"]["parameters"]["frequencies_hz"].as_array_mut().unwrap().pop();
    assert_fails(&base, &dropped, "inputs", "a frequency dropped").await;
    let mut exchanged = body.clone();
    exchanged["inputs"]["parameters"]["frequencies_hz"].as_array_mut().unwrap().swap(1, 2);
    assert_fails(&base, &exchanged, "inputs", "two frequencies exchanged").await;

    let mut other_field = body.clone();
    other_field["field_sha256"] = json!(flip_hex(body["field_sha256"].as_str().unwrap()));
    assert_fails(&base, &other_field, "field_sha256", "field").await;
    let mut other_count = body.clone();
    other_count["iterations"] = json!(body["iterations"].as_u64().unwrap() + 1);
    assert_fails(&base, &other_count, "outcome", "iterations").await;
    let mut demoted = body.clone();
    demoted["outcome"] = json!("refused");
    assert_fails(&base, &demoted, "outcome", "outcome").await;

    // A name this solver does not publish is refused before the seal is read:
    // an invented one, and one the atom solver publishes.
    for name in ["poisson_ratio_hex", "total_energy_ha_hex"] {
        let mut named = body.clone();
        named["result"][name] = json!("3fd3333333333333");
        let (status, text) = post(&base, "/verify_solution", &named).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains(&format!("'result.{name}'")), "{text}");
        assert!(!text.contains("holds"), "{text}");
    }
    // And a solution of the atom solver may not carry a modulus.
    let (_, helium) = solved(&base, &json!({ "solver": "atom_dft", "parameters": { "atomic_number": 2 } })).await;
    let mut crossed = helium.clone();
    crossed["result"]["youngs_modulus_pa_hex"] = body["result"]["youngs_modulus_pa_hex"].clone();
    let (status, text) = post(&base, "/verify_solution", &crossed).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'result.youngs_modulus_pa_hex'"), "{text}");
}

#[tokio::test]
async fn the_proof_is_over_the_values_of_the_series() {
    let base = spawn_app().await;
    let (text, body) = solved(&base, &scan(0)).await;
    let request = format!(
        r#"{{"solver":"{SOLVER}","parameters":{{"bar":"ERX1","frequencies_hz":[1588.627,8950.895,4700.015]}}}}"#
    );
    assert!(text.starts_with(&format!("{{\"inputs\":{request},")), "{text}");

    // The sealed form: keys sorted, each number as the bits it was read as,
    // the series in the order it was sent.
    let sealed_form = format!(
        r#"{{"parameters":{{"bar":"ERX1","frequencies_hz":[x{},x{},x{}]}},"solver":"{SOLVER}"}}"#,
        hex_of(1588.627),
        hex_of(8950.895),
        hex_of(4700.015)
    );
    assert_eq!(body["seal_parts"]["inputs"], sha256_hex(sealed_form.as_bytes()));

    // Another spelling of the same numbers, other spacing and another key
    // order: the same question, the same proof.
    let respelled = format!(
        r#"{{ "parameters": {{ "frequencies_hz": [ 1.588627e3, 0.8950895E4, 4700.0150 ], "bar": "ERX1" }}, "solver": "{SOLVER}" }}"#
    );
    let (status, again) = post_text(&base, "/solve", &respelled).await;
    assert_eq!(status, 200, "{again}");
    let again: Value = serde_json::from_str(&again).unwrap();
    assert_eq!(again["solution_seal"], body["solution_seal"]);
    assert_eq!(again["seal_parts"], body["seal_parts"]);
    let posted = text.replacen(&request, respelled.trim(), 1);
    assert_ne!(posted, text);
    let (status, report) = post_text(&base, "/verify_solution", &posted).await;
    assert_eq!(status, 200, "{report}");
    // Its seal holds; since 0.11.0 the long form also compares the copy with
    // the answer the engine writes, and a request spelled another way has
    // another receipt, which is the first place it names.
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!("receipt_sha256")), "{report}");

    // A number never seals the same as the string of its bits.
    for forged in [json!(format!("x{}", hex_of(1588.627))), json!("1588.627"), json!([1588.627])] {
        let mut edited = body.clone();
        edited["inputs"]["parameters"]["frequencies_hz"][0] = forged.clone();
        let (status, text) = post(&base, "/verify_solution", &edited).await;
        if status == 422 {
            continue;
        }
        assert_eq!(status, 200, "{forged}: {text}");
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{forged}");
        assert_eq!(report["cause"], "inputs", "{forged}");
    }

    // The same question twice gives the same bytes.
    let (second, _) = solved(&base, &scan(0)).await;
    assert_eq!(second, text);
}

#[tokio::test]
async fn the_described_example_runs_unchanged_and_the_description_holds_its_limits() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 25);
    assert_eq!(solvers[4]["solver"], SOLVER);
    assert!(solvers[4]["summary"].as_str().unwrap().ends_with('.'));

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": SOLVER })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["summary"], solvers[4]["summary"]);
    assert_eq!(described["engine_version"], "genesis_solvers_v0.12.0");
    assert_eq!(described["inputs"][0]["name"], "bar");
    assert_eq!(described["inputs"][0]["allowed"], json!(["ERX1", "ERX2", "ERY1", "ERY2"]));
    assert_eq!(described["inputs"][1]["name"], "frequencies_hz");
    assert_eq!(described["inputs"][1]["most"], 3);
    assert_eq!(described["inputs"][1]["refusal"], "FrequencyNotPositive");
    assert_eq!(described["locked"]["series_order"], json!(["flatwise", "torsion", "edgewise"]));
    assert_eq!(
        described["locked"]["compared_frequency_range_hz"],
        json!({
            "flatwise": { "minimum": 1587.428, "maximum": 1608.28 },
            "torsion": { "minimum": 8899.446, "maximum": 8966.189 },
            "edgewise": { "minimum": 4695.035, "maximum": 4755.856 },
        })
    );
    // The constants of each bar are published, and they are the solver's.
    let bars = described["locked"]["bars"].as_array().unwrap();
    assert_eq!(bars.len(), 4);
    for (entry, bar) in bars.iter().zip(modulus::BARS) {
        let constants = bar.constants();
        assert_eq!(entry["bar"], bar.name());
        assert_eq!(entry["mass_kg"].as_f64().unwrap().to_bits(), constants.mass_kg.to_bits());
        assert_eq!(entry["length_m"].as_f64().unwrap().to_bits(), constants.length_m.to_bits());
        assert_eq!(entry["width_m"].as_f64().unwrap().to_bits(), constants.width_m.to_bits());
        assert_eq!(entry["thickness_m"].as_f64().unwrap().to_bits(), constants.thickness_m.to_bits());
    }
    assert_eq!(described["locked"]["max_iterations"], 50);
    assert_eq!(described["locked"]["edgewise_line_p2_fraction"], 0.0025);
    assert!(described["locked"]["not_returned"].as_str().unwrap().starts_with("Poisson's ratio."));
    assert_eq!(described["invariants"].as_array().unwrap().len(), 4);
    assert_eq!(described["invariants"][0]["tolerance"], 1.0e-8);
    assert_eq!(described["invariants"][0]["compared"], "every iteration");
    assert_eq!(described["convergence"]["step_tolerance"], 1.0e-12);
    assert_eq!(described["reference"]["url"], "https://doi.org/10.5281/zenodo.6778336");
    assert_eq!(described["reference"]["retrieved"], "2026-10-09");
    assert!(described["reference"]["kind"].as_str().unwrap().contains("the same frequencies, so this confirms the calculation"));

    // The validation record: five parts, and the comparison as it came out.
    let record = described["validation"].as_object().unwrap();
    let parts: Vec<&str> = record.keys().map(String::as_str).collect();
    assert_eq!(parts, ["object", "data", "pass_line", "result", "limits"]);
    assert!(record["data"].as_str().unwrap().contains("doi 10.5281/zenodo.6778336"));
    assert!(record["pass_line"].as_str().unwrap().contains("e606a12456dfb449387a8a3c627401ebbc087166264b885b336b466f46dcec2d"));
    assert!(record["pass_line"].as_str().unwrap().contains("ea32c1e8b46f5191316dc9b53dc30f2105e1e86a7600d422b4fa8da5744589da"));
    // The one place the tests hold the words of the result.
    assert!(record["result"].as_str().unwrap().starts_with(
        "On bars ERY1 and ERY2, not used to choose the model. P1: 215.68 GPa recovered against 210.25 GPa from the tensile slope, 2.58 percent apart (target 2.49)"
    ));
    assert!(record["limits"].as_str().unwrap().starts_with(
        "The one other measurement of this sheet's modulus is the slope of the tensile tests"
    ));
    // Sentences held in the one line of the list, the result and the limits.
    let summary = described["summary"].as_str().unwrap();
    assert!(summary.contains("215.68 GPa against 210.25 GPa from the tensile slope, 2.58 percent apart (target 2.49)"), "{summary}");
    assert!(summary.contains("flatwise 1587.428 to 1608.28 Hz") && summary.contains("within 0.114 percent"), "{summary}");
    assert!(summary.contains("(PoissonRatioOutOfRange)") && summary.contains("the shear modulus has no second measurement"), "{summary}");
    let result = record["result"].as_str().unwrap();
    for words in [
        "P1: 215.68 GPa recovered against 210.25 GPa from the tensile slope, 2.58 percent apart (target 2.49), 0.094 of a percentage point, 0.2 GPa, over the target.",
        "over 100 to 140, 140 to 180 or 100 to 200 MPa the recovered modulus is inside the target (2.10, 2.26 and 2.24 percent against 2.48, 2.38 and 2.43)",
        "over 60 to 100, 50 to 150, 180 to 220 or 50 to 250 MPa outside it",
        "P2: worst of four 0.114 percent (target 0.25).",
        "4755.130 Hz for both ERY1 and ERY2 in the laboratory's whole-spectrum column",
        "−0.129 percent (ERY1) and −0.027 percent (ERY2)",
        "P6: worst 0.053 percent (target 0.10). P6 is arithmetic",
        "0.032 percent in G and 0.051 to 0.053 percent in E",
        "the sizes were read before the frequencies",
        "211.6 GPa against 197.0 GPa, 7.4 percent apart (target 2.29).",
        "Later builds re-make every answer bit for bit",
        "P3 for G, 0.598 against 0.601 percent",
        "P3: 0.46 percent in E (target 1.41), 0.60 percent in G (target 4.24). P5: 9.2e-11 (target 1e-8).",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let limits = record["limits"].as_str().unwrap();
    for words in [
        "along the rolling direction the slope itself moves by up to 8 percent within one test",
        "Across the rolling direction the slope moves 1.4 to 2.0 percent within a test",
        "the scatter is between test pieces: 204.5 to 213.4 GPa",
        "a fixed 0.6 micrometres at each end of a 25 micrometre window, the same formula gives a target near 4.6 percent",
        "the record's own 5 percent \"too loose to decide\"",
        "A handbook 210 GPa is 0.12 percent from the mean of the tensile slopes across the rolling direction",
        "the solver reproduces the laboratory's own values on two bars not used to choose it, within 0.053 percent, and predicts a resonance it is not given, within 0.114 percent",
        "The first is arithmetic: the same formula on the same readings",
        "with the offset set by the bar's shape",
        "tests the shear and rotary-inertia correction of the bending model and the width and thickness readings",
        "The edgewise residual is unchanged when all three frequencies are scaled by one factor",
        "it moves 0.027 percent for 3 percent in G and 0.006 percent with the torsion end correction, and stays inside 0.25 percent with a shear coefficient of 5/6",
        "a handbook Poisson's ratio of 0.29 predicts the same four readings within 0.082 percent",
        "The solver cannot tell the bar name from the frequencies",
        "ERY1's frequencies sent under the name ERX1 converge, to 216.32 GPa",
        "The compared ranges are the envelope of the eight scans compared with measurement, two on each of the four bars: flatwise 1587.428 to 1608.28 Hz, torsion 8899.446 to 8966.189 Hz, and edgewise from 4695.035 Hz",
        "Inside them the answers run from 210.7 to 216.3 GPa in E and from 79.0 to 80.8 GPa in G.",
        "a supplied edgewise frequency is answered with its residual as it is, past the 0.25 percent of line P2 too",
        "The shear modulus has no second measurement of any kind.",
        "summary sheet states ±1 percent (95 percent confidence) for both E and G at room temperature",
        "that is the uncertainty returned for both moduli",
        "at most 1 percent for E and at most 3 percent for G at room temperature; the 3 percent is returned beside the shear modulus as shear_modulus_method_uncertainty_ceiling_pa",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }

    let refusals = described["refusals"].as_array().expect("refusals");
    let names: Vec<&str> = refusals.iter().map(|refusal| refusal["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "RepeatedMode",
            "FrequencyNotPositive",
            "YoungsModulusNotIdentifiable",
            "ShearModulusNotIdentifiable",
            "OutsideSearchRange",
            "FrequencyEquationNotBracketed",
            "NonFiniteValue",
            "EnergyBalanceBroken",
            "FreeEndBroken",
            "NotConverged",
            "RoundTripBroken",
            "PoissonRatioOutOfRange",
            // The two the entry names itself at the door follow the solver's twelve.
            "UnknownBar",
            "TooManyFrequencies",
        ]
    );
    let bound = described["inputs"][1]["bound"].as_str().unwrap();
    assert!(bound.contains("flatwise 1587.428 to 1608.28 Hz, torsion 8899.446 to 8966.189 Hz and edgewise 4695.035 to 4755.856 Hz"), "{bound}");
    assert!(bound.contains("line P2 of the validation"), "{bound}");
    assert!(!bound.contains("check"), "{bound}");
    assert!(!limits.contains("1200 Hz with torsion"), "{limits}");
    for refusal in refusals {
        assert!(refusal["meaning"].as_str().unwrap().ends_with('.'));
        assert!(!refusal["meaning"].as_str().unwrap().contains("..="), "{refusal}");
    }
    let (_, body) = solved(&base, &described["example_solve"]).await;
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["inputs"], described["example_solve"]);
    assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    // The example is the first published scan of ERX1.
    assert_eq!(described["example_solve"], scan(0));
}

/// The eight scans compared with measurement: bar, flatwise, edgewise, torsion (Hz).
const COMPARED_SCANS: [(&str, f64, f64, f64); 8] = [
    ("ERX1", 1588.627, 4700.015, 8950.895),
    ("ERX1", 1587.428, 4698.493, 8944.466),
    ("ERX2", 1598.579, 4704.533, 8899.446),
    ("ERX2", 1598.747, 4707.067, 8901.011),
    ("ERY1", 1606.61, 4755.13, 8948.668),
    ("ERY1", 1608.28, 4755.856, 8936.544),
    ("ERY2", 1607.396, 4755.13, 8966.189),
    ("ERY2", 1606.268, 4753.821, 8962.771),
];

/// Every compared scan is answered with two frequencies and again with the
/// edgewise frequency the two predict, with the same moduli; and the corner
/// whose prediction leaves the edgewise range is refused on both routes.
#[tokio::test]
async fn a_pair_and_the_pair_with_its_predicted_edgewise_frequency_agree() {
    let base = spawn_app().await;
    for (bar, flatwise, edgewise, torsion) in COMPARED_SCANS {
        let (_, pair) = solved(&base, &question(bar, &[flatwise, torsion])).await;
        assert_eq!(pair["outcome"], "converged", "{bar} {flatwise}");
        let predicted = pair["result"]["edgewise_predicted_hz"].as_f64().unwrap();
        let (_, again) = solved(&base, &question(bar, &[flatwise, torsion, predicted])).await;
        assert_eq!(again["outcome"], "converged", "{bar} {flatwise}");
        assert_eq!(again["result"]["youngs_modulus_pa_hex"], pair["result"]["youngs_modulus_pa_hex"]);
        assert_eq!(again["result"]["edgewise_residual_fraction"], 0.0);
        let (_, measured) = solved(&base, &question(bar, &[flatwise, torsion, edgewise])).await;
        assert_eq!(measured["outcome"], "converged", "{bar} {flatwise}");
        assert!(measured["result"]["edgewise_residual_fraction"].as_f64().unwrap().abs() <= 0.0025);
    }
    // The corner whose prediction leaves the compared edgewise range, which
    // engine 0.8.0 refused, is answered on both routes with the same moduli.
    let (_, pair) = solved(&base, &question("ERX1", &[1587.428, 8899.446])).await;
    assert_eq!(pair["outcome"], "converged");
    let predicted = pair["result"]["edgewise_predicted_hz"].as_f64().unwrap();
    assert!(predicted < modulus::EDGEWISE_RANGE_HZ.0, "{predicted}");
    let (_, again) = solved(&base, &question("ERX1", &[1587.428, 8899.446, predicted])).await;
    assert_eq!(again["outcome"], "converged");
    assert_eq!(again["result"]["youngs_modulus_pa_hex"], pair["result"]["youngs_modulus_pa_hex"]);
    // ERY1's frequencies under ERX1's name still converge, as the description says.
    let (_, crossed) = solved(&base, &question("ERX1", &[1608.28, 8936.544, 4755.856])).await;
    assert_eq!(crossed["outcome"], "converged");
    let e_gpa = crossed["result"]["youngs_modulus_pa"].as_f64().unwrap() / 1.0e9;
    assert!((e_gpa - 216.32).abs() < 0.005, "{e_gpa}");
}

/// Line P2 is the line the prediction was compared at: an edgewise frequency
/// within 0.25 percent of the prediction and one past it are both answered,
/// the residual returned as it is, with the same moduli.
#[tokio::test]
async fn the_edgewise_residual_is_returned_on_both_sides_of_line_p2() {
    let base = spawn_app().await;
    for ((bar, flatwise, _, torsion), sign) in [(COMPARED_SCANS[3], -1.0), (COMPARED_SCANS[5], 1.0)] {
        let (_, pair) = solved(&base, &question(bar, &[flatwise, torsion])).await;
        let predicted = pair["result"]["edgewise_predicted_hz"].as_f64().unwrap();
        for fraction in [0.0024, 0.0026] {
            let (_, answered) = solved(&base, &question(bar, &[flatwise, torsion, predicted / (1.0 + sign * fraction)])).await;
            assert_eq!(answered["outcome"], "converged", "{bar}");
            let residual = answered["result"]["edgewise_residual_fraction"].as_f64().unwrap();
            assert!((residual - sign * fraction).abs() < 1.0e-9, "{residual}");
            assert_eq!(answered["result"]["youngs_modulus_pa_hex"], pair["result"]["youngs_modulus_pa_hex"]);
            assert_eq!(verdict(&base, &answered).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
        }
    }
}

// ------------------------------------------------------ the three connections

const KEY_ALL: &str = "key-for-the-client-that-sees-both-solvers";
const KEY_BAR: &str = "key-for-the-bar-client";
const KEY_ATOM: &str = "key-for-the-atom-client";

fn client_entry(label: &str, key: &str, solvers: Value) -> Value {
    json!({ "client": label, "key_sha256": sha256_hex(key.as_bytes()), "cards": [], "solvers": solvers, "max_in_flight": 8, "max_permits": 1 })
}

/// The local connection: `mcp_stdio` as a child process.
struct Local {
    _child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Local {
    async fn start() -> Local {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut local = Local { _child: child, stdin, out };
        let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
        local.raw(&init.to_string()).await;
        local
            .stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .unwrap();
        local
    }

    async fn raw(&mut self, line: &str) -> String {
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.write_all(b"\n").await.unwrap();
        self.stdin.flush().await.unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).await.unwrap();
        answer.trim_end_matches('\n').to_string()
    }

    async fn call(&mut self, name: &str, arguments: &Value) -> String {
        let line = answer_text::tool_call(7, name, arguments);
        self.raw(&line.to_string()).await
    }
}

/// The hosted connection: `hosted_door` as a child process, with a keys file.
struct Hosted {
    _child: Child,
    base: String,
    _dir: PathBuf,
}

impl Hosted {
    async fn start(keys: &Value) -> Hosted {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("modulus_hosted_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("keys.json");
        std::fs::write(&file, serde_json::to_string_pretty(keys).unwrap()).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .arg("--port")
            .arg("0")
            .arg("--keys")
            .arg(&file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        out.read_line(&mut line).await.unwrap();
        let address = line
            .split("running on ")
            .nth(1)
            .unwrap_or_else(|| panic!("no address in {line:?}"))
            .split(' ')
            .next()
            .unwrap()
            .to_string();
        tokio::spawn(async move {
            let mut rest = String::new();
            while out.read_line(&mut rest).await.unwrap_or(0) > 0 {
                rest.clear();
            }
        });
        Hosted { _child: child, base: format!("http://{address}"), _dir: dir }
    }

    async fn call(&self, key: &str, tool: &str, arguments: &Value) -> String {
        let body = answer_text::tool_call(7, tool, arguments);
        let response = reqwest::Client::new()
            .post(format!("{}/mcp", self.base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Authorization", format!("Bearer {key}"))
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        response.text().await.unwrap()
    }

    async fn tools(&self, key: &str) -> Value {
        let body = json!({"jsonrpc":"2.0","id":3,"method":"tools/list"});
        let response = reqwest::Client::new()
            .post(format!("{}/mcp", self.base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Authorization", format!("Bearer {key}"))
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        serde_json::from_str(&response.text().await.unwrap()).unwrap()
    }
}

/// The text of a tool's result, and whether the connection marked it an error.
fn result_text(answer: &str) -> (String, bool) {
    let value: Value = serde_json::from_str(answer).unwrap();
    let text = value["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text in {answer}"))
        .to_string();
    answer_text::remember(&text);
    (text, value["result"]["isError"] == true)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_three_connections_give_the_same_bytes_and_a_key_sees_only_its_solver() {
    let web = spawn_app().await;
    let mut local = Local::start().await;
    let hosted = Hosted::start(&json!([
        client_entry("all", KEY_ALL, json!(["atom_dft", "ablator_slab", "journal_bearing_sheffield", "propeller_apc_10x7e", "modulus_bam_s355", "seawater_column", "argon_pair_gas", "fusion_pb11_lhd", "fusion_pb11_thermal", "step_flow_armaly", "culture_trichoderma_ambr250", "tokamak_dt_beam_target", "pipe_flow_superpipe", "fuel_rod_triga_jsi", "culture_trichoderma_atp", "junction_nist_b7_s2", "water_hammer_oil_line", "orifice_air_chamber", "pneumatic_hose_resonance", "blast_wave_tnt_air", "stress_plate_hole_s235", "stress_plate_hole_s235_elastic", "icf_hotspot_burn", "ln2_line_dlr_fttf", "plasma_sheath_ram_c2"])),
        client_entry("bar", KEY_BAR, json!([SOLVER])),
        client_entry("atom", KEY_ATOM, json!(["atom_dft"])),
    ]))
    .await;

    // The list.
    let on_local = local.call("list_solvers", &json!({})).await;
    let (listed, error) = result_text(&on_local);
    assert!(!error);
    let on_web = reqwest::get(format!("{web}/solvers")).await.unwrap().text().await.unwrap();
    assert_eq!(listed, on_web);
    assert_eq!(hosted.call(KEY_ALL, "list_solvers", &json!({})).await, on_local);
    let (bar_only, _) = result_text(&hosted.call(KEY_BAR, "list_solvers", &json!({})).await);
    let bar_only: Value = serde_json::from_str(&bar_only).unwrap();
    assert_eq!(bar_only["solvers"].as_array().unwrap().len(), 1);
    assert_eq!(bar_only["solvers"][0]["solver"], SOLVER);

    // The description, a solution, a sealed refusal and a refusal at the door.
    let which = json!({ "solver": SOLVER });
    let refused = question("ERX1", &[1587.428, 8899.446]);
    let at_the_door = question("ERX1", &[1588.627]);
    for (tool, path, arguments) in [
        ("describe_solver", "/describe_solver", which.clone()),
        ("solve", "/solve", scan(1)),
        ("solve", "/solve", refused.clone()),
        ("solve", "/solve", at_the_door.clone()),
    ] {
        let on_local = local.call(tool, &arguments).await;
        let (text, error) = result_text(&on_local);
        let (status, on_web) = post(&web, path, &arguments).await;
        assert_eq!(text, on_web, "{tool} {arguments}");
        assert_eq!(error, status != 200, "{tool} {arguments}");
        for key in [KEY_ALL, KEY_BAR] {
            assert_eq!(hosted.call(key, tool, &arguments).await, on_local, "{key} {tool}");
        }
    }
    let (_, sealed) = post(&web, "/solve", &scan(1)).await;
    let sealed: Value = serde_json::from_str(&sealed).unwrap();
    let on_local = local.call("verify_solution", &sealed).await;
    let (text, error) = result_text(&on_local);
    assert!(!error);
    assert_eq!(text, format!(r#"{{"holds":true,"values_hold":true,"cause":null,"compared":{}}}"#, serde_json::json!(protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED)));
    for key in [KEY_ALL, KEY_BAR] {
        assert_eq!(hosted.call(key, "verify_solution", &sealed).await, on_local, "{key}");
    }

    // A key without the bar solver: it is a solver the engine does not have.
    let unknown = json!({ "solver": "modulus_bam_s356" });
    assert_eq!(
        hosted.call(KEY_ATOM, "describe_solver", &which).await,
        hosted.call(KEY_ATOM, "describe_solver", &unknown).await.replace("modulus_bam_s356", SOLVER)
    );
    let (text, error) = result_text(&hosted.call(KEY_ATOM, "solve", &scan(1)).await);
    assert!(error);
    assert_eq!(text, "Solver not recognized (UnknownSolver).");
    let (hidden, error) = result_text(&hosted.call(KEY_ATOM, "verify_solution", &sealed).await);
    assert!(error, "{hidden}");
    let mut renamed = sealed.clone();
    renamed["inputs"]["solver"] = json!("modulus_bam_s356");
    let (lacking, _) = result_text(&hosted.call(KEY_ALL, "verify_solution", &renamed).await);
    assert_eq!(hidden, lacking);

    // What each key is shown of the solve tool: no word of a solver it lacks.
    let schema_of = |tools: &Value| -> String {
        tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "solve")
            .expect("solve")
            .to_string()
    };
    let for_bar = schema_of(&hosted.tools(KEY_BAR).await);
    assert!(for_bar.contains(SOLVER) && for_bar.contains("frequencies_hz"), "{for_bar}");
    assert!(!for_bar.contains("atom_dft") && !for_bar.contains("ablator_slab"), "{for_bar}");
    let for_atom = schema_of(&hosted.tools(KEY_ATOM).await);
    assert!(!for_atom.contains("modulus") && !for_atom.contains("frequencies_hz"), "{for_atom}");
    let for_all = schema_of(&hosted.tools(KEY_ALL).await);
    // With more than one solver in view the schema names none: list_solvers does.
    assert!(!for_all.contains(SOLVER) && !for_all.contains("atom_dft"), "{for_all}");
}
