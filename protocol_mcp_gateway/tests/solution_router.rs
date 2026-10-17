//! `POST /solve` against a direct call to `genesis_solvers`, and
//! `POST /verify_solution` against edits of a sealed solution.

mod answer_text;

use std::time::{Duration, Instant};

use genesis_solvers::atom_dft;
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

fn atom(atomic_number: u64) -> Value {
    json!({ "solver": "atom_dft", "parameters": { "atomic_number": atomic_number } })
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
async fn hydrogen_neon_and_argon_match_the_solver_bit_for_bit() {
    let base = spawn_app().await;
    for (atomic_number, orbitals) in [
        (1_u32, vec!["1s"]),
        (10, vec!["1s", "2s", "2p"]),
        (18, vec!["1s", "2s", "2p", "3s", "3p"]),
    ] {
        let request = atom(u64::from(atomic_number));
        let started = Instant::now();
        let (text, body) = solved(&base, &request).await;
        println!(
            "atomic number {atomic_number}: {:.1} ms through POST /solve",
            started.elapsed().as_secs_f64() * 1.0e3
        );
        let direct = atom_dft::solve(atomic_number).expect("the solver converges");

        assert_eq!(body["inputs"], request);
        assert_eq!(body["engine_version"], "genesis_solvers_v0.12.0");
        assert_eq!(body["outcome"], "converged");
        assert!(body["reject"].is_null());
        assert!(body["reject_detail"].is_null());
        assert_eq!(body["iterations"], direct.iterations);

        let result = &body["result"];
        assert_number(result, "total_energy_ha", direct.total_energy_ha);
        assert_number(result, "kinetic_energy_ha", direct.kinetic_energy_ha);
        assert_number(
            result,
            "electron_nucleus_energy_ha",
            direct.electron_nucleus_energy_ha,
        );
        assert_number(result, "hartree_energy_ha", direct.hartree_energy_ha);
        assert_number(
            result,
            "exchange_correlation_energy_ha",
            direct.exchange_correlation_energy_ha,
        );
        assert_eq!(direct.orbitals.len(), orbitals.len());
        for (orbital, label) in direct.orbitals.iter().zip(&orbitals) {
            assert_number(
                result,
                &format!("eigenvalue_{label}_ha"),
                orbital.eigenvalue_ha,
            );
            assert_number(result, &format!("occupation_{label}"), orbital.occupation);
        }
        assert_eq!(
            result.as_object().unwrap().len(),
            2 * (5 + 2 * orbitals.len()),
            "a number and its hex for each energy, eigenvalue and occupation"
        );

        let invariants = &body["invariants"];
        assert_number(invariants, "electron_count", direct.electron_count);
        assert_number(invariants, "virial_residual_ha", direct.virial_residual_ha);
        assert_eq!(invariants.as_object().unwrap().len(), 4);

        let mut field = Vec::with_capacity(8 + 8 * direct.electron_density.len());
        field.extend_from_slice(&(direct.electron_density.len() as u64).to_le_bytes());
        for sample in &direct.electron_density {
            field.extend_from_slice(&sample.to_le_bytes());
        }
        assert_eq!(direct.electron_density.len(), atom_dft::GRID_POINTS);
        assert_eq!(body["field_sha256"], sha256_hex(&field));

        assert_eq!(body["solution_seal"].as_str().unwrap().len(), 64);
        assert_receipt(&text, &body);
        let report = verdict(&base, &body).await;
        assert_eq!(report, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    }
}

/// The seal, rebuilt here from the response alone.
#[tokio::test]
async fn the_seal_is_the_request_the_hex_lines_the_field_hash_and_the_tail() {
    let base = spawn_app().await;
    let (_, body) = solved(&base, &atom(1)).await;
    let sealed = r#"{"parameters":{"atomic_number":x3ff0000000000000},"solver":"atom_dft"}"#;
    assert_eq!(
        sha256_hex(sealed.as_bytes()),
        "c9b4fa3ff898d0dead7f28184283950a3e9d8c8be6548c2154e952809a2e8f25"
    );
    assert_eq!(body["seal_parts"]["inputs"], sha256_hex(sealed.as_bytes()));
    let mut bytes = sealed.as_bytes().to_vec();
    // The engine version is a line of its own after the request.
    let version = format!("engine_version:{}\n", body["engine_version"].as_str().unwrap());
    assert_eq!(body["seal_parts"]["engine_version"], sha256_hex(version.as_bytes()));
    bytes.extend_from_slice(version.as_bytes());
    for section in ["invariants", "result"] {
        let object = body[section].as_object().unwrap();
        let mut keys: Vec<&String> = object.keys().filter(|key| key.ends_with("_hex")).collect();
        keys.sort();
        for key in keys {
            let line = format!("{section}.{key}:{}\n", object[key].as_str().unwrap());
            bytes.extend_from_slice(line.as_bytes());
        }
    }
    bytes.extend_from_slice(body["field_sha256"].as_str().unwrap().as_bytes());
    let tail = format!(
        r#"{{"outcome":"converged","iterations":{},"reject":null}}"#,
        body["iterations"]
    );
    bytes.extend_from_slice(tail.as_bytes());
    assert_eq!(body["solution_seal"], sha256_hex(&bytes));
    assert_eq!(body["seal_parts"]["outcome"], sha256_hex(tail.as_bytes()));
}

#[tokio::test]
async fn the_same_request_seals_the_same_and_another_atom_does_not() {
    let base = spawn_app().await;
    let (first_text, first) = solved(&base, &atom(2)).await;
    let (second_text, second) = solved(&base, &atom(2)).await;
    assert_eq!(first["solution_seal"], second["solution_seal"]);
    assert_eq!(first["field_sha256"], second["field_sha256"]);
    assert_eq!(first_text, second_text);

    let reordered: Value =
        serde_json::from_str(r#"{"parameters":{"atomic_number":2},"solver":"atom_dft"}"#).unwrap();
    let (_, third) = solved(&base, &reordered).await;
    assert_eq!(first["solution_seal"], third["solution_seal"]);
    assert_eq!(first["field_sha256"], third["field_sha256"]);

    let (_, other) = solved(&base, &atom(1)).await;
    assert_ne!(first["solution_seal"], other["solution_seal"]);
    assert_ne!(first["field_sha256"], other["field_sha256"]);
}

#[tokio::test]
async fn verify_solution_names_the_edited_part() {
    let base = spawn_app().await;
    let (_, body) = solved(&base, &atom(3)).await;
    assert_eq!(
        verdict(&base, &body).await,
        json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED })
    );

    let mut edits = 0_u32;
    for section in ["result", "invariants"] {
        let keys: Vec<String> = body[section].as_object().unwrap().keys().cloned().collect();
        for key in keys.iter().filter(|key| key.ends_with("_hex")) {
            let mut edited = body.clone();
            let hex = flip_hex(edited[section][key].as_str().unwrap());
            edited[section][key] = json!(hex);
            assert_fails(&base, &edited, section, key).await;
            edits += 1;
        }
    }
    assert_eq!(edits, 11, "nine result numbers and two invariants");

    let mut field = body.clone();
    field["field_sha256"] = json!(flip_hex(body["field_sha256"].as_str().unwrap()));
    assert_fails(&base, &field, "field_sha256", "field hash").await;

    let mut outcome = body.clone();
    outcome["outcome"] = json!("refused");
    assert_fails(&base, &outcome, "outcome", "outcome").await;

    let mut iterations = body.clone();
    iterations["iterations"] = json!(body["iterations"].as_u64().unwrap() + 1);
    assert_fails(&base, &iterations, "outcome", "iterations").await;

    let mut reject = body.clone();
    reject["reject"] = json!("NotConverged");
    assert_fails(&base, &reject, "outcome", "reject").await;

    let mut inputs = body.clone();
    inputs["inputs"]["parameters"]["atomic_number"] = json!(4);
    assert_fails(&base, &inputs, "inputs", "inputs").await;

    let mut seal = body.clone();
    seal["solution_seal"] = json!(flip_hex(body["solution_seal"].as_str().unwrap()));
    assert_fails(&base, &seal, "solution_seal", "seal").await;

    // An edit that also replaces its own part digest still fails the seal.
    let mut covered = body.clone();
    let forged = flip_hex(body["field_sha256"].as_str().unwrap());
    covered["seal_parts"]["field_sha256"] = json!(sha256_hex(forged.as_bytes()));
    covered["field_sha256"] = json!(forged);
    assert_fails(
        &base,
        &covered,
        "solution_seal",
        "edit with its part digest",
    )
    .await;

    let mut missing = body.clone();
    missing.as_object_mut().unwrap().remove("field_sha256");
    let (status, text) = post(&base, "/verify_solution", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, "Missing required parameter: 'field_sha256' (InputMissing).");
}

#[tokio::test]
async fn a_request_outside_the_solver_is_422_with_no_seal() {
    let base = spawn_app().await;
    let cases = [
        (
            atom(0),
            // Re-pinned for 0.10.1: an atomic number outside the range carries the name AtomicNumberOutOfRange, and the entry writes the range in words.
            "Physical Violation: atomic number 0 is outside 1 to 18 (AtomicNumberOutOfRange).",
        ),
        (
            atom(19),
            // Re-pinned for 0.10.1: an atomic number outside the range carries the name AtomicNumberOutOfRange, and the entry writes the range in words.
            "Physical Violation: atomic number 19 is outside 1 to 18 (AtomicNumberOutOfRange).",
        ),
        (
            atom(4_294_967_297),
            // Re-pinned for 0.10.1: an atomic number outside the range carries the name AtomicNumberOutOfRange, and the entry writes the range in words.
            "Physical Violation: atomic number 4294967297 is outside 1 to 18 (AtomicNumberOutOfRange).",
        ),
        (
            json!({ "solver": "atom_dft", "parameters": { "atomic_number": "10" } }),
            "Physical Violation: 'atomic_number' must be an integer written as digits, with no decimal point and no exponent; this request has \"10\".",
        ),
        (
            json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10.0 } }),
            "Physical Violation: 'atomic_number' must be an integer written as digits, with no decimal point and no exponent; this request has 10.0.",
        ),
        (
            json!({ "solver": "atom_dft", "parameters": { "atomic_number": -1 } }),
            "Physical Violation: atomic number -1 is outside 1 to 18 (AtomicNumberOutOfRange).",
        ),
        (
            json!({ "solver": "atom_dft", "parameters": {} }),
            "Missing required parameter: 'atomic_number' (InputMissing).",
        ),
        (
            json!({ "solver": "atom_dft" }),
            "Missing required parameter: 'parameters' (or 'example', the name of a worked example describe_solver lists) (InputMissing).",
        ),
        (
            json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10, "grid_points": 101 } }),
            "Physical Violation: 'grid_points' is not an input of this solver. The grid, the mixing and the tolerances stay inside the solver.",
        ),
        (
            json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10 }, "mixing_alpha": 0.9 }),
            "Physical Violation: 'mixing_alpha' is not an input of this solver. The grid, the mixing and the tolerances stay inside the solver.",
        ),
    ];
    for (request, message) in cases {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request} {text}");
        assert_eq!(text, message, "{request}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    let (status, text) = post(
        &base,
        "/solve",
        &json!({ "solver": "atom_hf", "parameters": { "atomic_number": 10 } }),
    )
    .await;
    assert_eq!(status, 404, "{text}");
    assert_eq!(text, "Solver not recognized (UnknownSolver).");
}

#[tokio::test]
async fn the_described_example_runs_unchanged_and_converges() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    assert_eq!(listed.status().as_u16(), 200);
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), 24);
    assert_eq!(solvers[0]["solver"], "atom_dft");
    assert_eq!(solvers[1]["solver"], "ablator_slab");
    assert_eq!(solvers[3]["solver"], "propeller_apc_10x7e");
    assert_eq!(solvers[6]["solver"], "argon_pair_gas");
    assert!(solvers[0]["summary"].as_str().unwrap().ends_with('.'));

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "atom_dft" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(described["inputs"][0]["name"], "atomic_number");
    assert_eq!(described["inputs"][0]["minimum"], 1);
    assert_eq!(described["inputs"][0]["maximum"], 18);
    assert_eq!(described["invariants"][0]["tolerance"], 1.0e-8);
    assert_eq!(described["invariants"][1]["tolerance"], 1.0e-5);
    assert_eq!(described["reference"]["retrieved"], "2026-10-05");
    assert!(
        described["reference"]["url"]
            .as_str()
            .unwrap()
            .starts_with("https://www.nist.gov/")
    );
    let refusals = described["refusals"].as_array().expect("refusals");
    assert_eq!(refusals.len(), 9);
    for refusal in refusals {
        assert!(refusal["name"].is_string());
        assert!(refusal["meaning"].as_str().unwrap().ends_with('.'));
    }

    let (_, body) = solved(&base, &described["example_solve"]).await;
    assert_eq!(body["outcome"], "converged");
    assert_eq!(body["inputs"], described["example_solve"]);
    // The worked example in the map: neon, and the hash of its sealed form.
    assert_eq!(
        sha256_hex(br#"{"parameters":{"atomic_number":x4024000000000000},"solver":"atom_dft"}"#),
        "1eac37e0cf92dfc49ae0d9699482e4d235b0815737d818439be116ccc464ab41"
    );
    assert_eq!(
        body["seal_parts"]["inputs"],
        "1eac37e0cf92dfc49ae0d9699482e4d235b0815737d818439be116ccc464ab41"
    );
    assert_eq!(
        verdict(&base, &body).await,
        json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED })
    );

    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "atom_hf" })).await;
    assert_eq!(status, 404, "{text}");
    assert_eq!(text, "Solver not recognized (UnknownSolver).");
}

#[tokio::test]
async fn a_repeated_key_in_a_solution_is_refused_and_not_sealed() {
    let base = spawn_app().await;
    let body = r#"{"inputs":{"solver":"atom_dft","parameters":{"atomic_number":1,"atomic_number":2}},"outcome":"converged","iterations":1,"reject":null,"result":null,"invariants":null,"field_sha256":null,"seal_parts":{"inputs":"aa","result":"aa","invariants":"aa","field_sha256":"aa","outcome":"aa"},"solution_seal":"bb"}"#;
    let client = Client::new();
    let response = client
        .post(format!("{base}/verify_solution"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    assert_eq!(status, 422, "{text}");
    assert_eq!(
        text,
        "Physical Violation: the key 'atomic_number' is repeated."
    );
    assert!(!text.contains("holds"), "{text}");
    assert!(!text.contains("solution_seal\":"), "{text}");
}

#[tokio::test]
async fn a_solution_survives_a_respelling_of_its_numbers() {
    let base = spawn_app().await;
    let (text, body) = solved(&base, &atom(2)).await;
    let request = r#"{"solver":"atom_dft","parameters":{"atomic_number":2}}"#;
    assert!(text.starts_with(&format!("{{\"inputs\":{request},")), "{text}");
    assert_eq!(
        verdict(&base, &body).await,
        json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED })
    );

    let client = Client::new();
    for spelling in ["2.0", "2e0", "0.2e1"] {
        let respelled = request.replacen(":2}", &format!(":{spelling}}}"), 1);
        assert_ne!(respelled, request);
        let posted = text.replacen(request, &respelled, 1);
        assert_ne!(posted, text);
        let response = client
            .post(format!("{base}/verify_solution"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(posted)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200, "{spelling}");
        // The seal is over the value and still holds. Since 0.11.0 the long
        // form also writes the answer again, and the engine answers no
        // request whose count is written with a decimal point or an
        // exponent: the inputs are the first place it names.
        let report: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();
        assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!("inputs")), "{spelling}: {report}");
        assert!(report["difference"].as_str().unwrap().contains("must be an integer written as digits"), "{spelling}: {report}");

        // The solver still takes a JSON integer and nothing else.
        let response = client
            .post(format!("{base}/solve"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(respelled)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 422, "{spelling}");
        assert_eq!(
            response.text().await.unwrap(),
            "Physical Violation: 'atomic_number' must be an integer written as digits, with no decimal point and no exponent; this request has 2.0.",
            "{spelling}"
        );
    }
}

#[tokio::test]
async fn a_number_never_seals_the_same_as_a_string() {
    let base = spawn_app().await;
    let (_, body) = solved(&base, &atom(2)).await;
    assert_eq!(
        body["seal_parts"]["inputs"],
        sha256_hex(br#"{"parameters":{"atomic_number":x4000000000000000},"solver":"atom_dft"}"#)
    );
    for forged in [
        json!("x4000000000000000"),
        json!("4000000000000000"),
        json!("2"),
        json!([2]),
        json!({ "x4000000000000000": 2 }),
    ] {
        let mut edited = body.clone();
        edited["inputs"]["parameters"]["atomic_number"] = forged.clone();
        let (status, text) = post(&base, "/verify_solution", &edited).await;
        if status == 422 {
            continue;
        }
        assert_eq!(status, 200, "{forged}: {text}");
        let report: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(report["holds"], false, "{forged}");
        assert_eq!(report["cause"], "inputs", "{forged}");

        let (status, text) = post(
            &base,
            "/solve",
            &json!({ "solver": "atom_dft", "parameters": { "atomic_number": forged } }),
        )
        .await;
        assert_eq!(status, 422, "{forged}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
}

/// A plain integer the JSON reader holds as a float is refused by the range,
/// named as it was read. A token with a point or an exponent inside the range
/// is refused as not an integer.
#[tokio::test]
async fn an_integer_held_as_a_float_is_refused_by_the_range() {
    let base = spawn_app().await;
    let client = Client::new();
    // Re-pinned for 0.10.1: an atomic number outside the range carries the name AtomicNumberOutOfRange, and the entry writes the range in words.
    let range = |read: &str| format!("Physical Violation: atomic number read as {read} is outside 1 to 18 (AtomicNumberOutOfRange).");
    let not_integer = |read: &str| {
        format!(
            "Physical Violation: 'atomic_number' must be an integer written as digits, with no decimal point and no exponent; this request has {read}."
        )
    };
    for (written, expected) in [
        ("-0", range("-0.0")),
        ("-0.0", range("-0.0")),
        ("18446744073709551616", range("1.8446744073709552e+19")),
        ("18446744073709551626", range("1.8446744073709552e+19")),
        ("-9223372036854775809", range("-9.223372036854776e+18")),
        ("1e20", range("1e+20")),
        // Re-pinned for 0.10.1: an atomic number outside the range carries the name AtomicNumberOutOfRange, and the entry writes the range in words.
        ("18446744073709551615", "Physical Violation: atomic number 18446744073709551615 is outside 1 to 18 (AtomicNumberOutOfRange).".to_string()),
        ("-9223372036854775808", "Physical Violation: atomic number -9223372036854775808 is outside 1 to 18 (AtomicNumberOutOfRange).".to_string()),
        ("0", "Physical Violation: atomic number 0 is outside 1 to 18 (AtomicNumberOutOfRange).".to_string()),
        ("0.0", not_integer("0.0")),
        ("10.0", not_integer("10.0")),
        ("1e1", not_integer("10.0")),
        ("1.5", not_integer("1.5")),
        ("1e19", not_integer("1e+19")),
    ] {
        let body = format!(r#"{{"solver":"atom_dft","parameters":{{"atomic_number":{written}}}}}"#);
        let response = client
            .post(format!("{base}/solve"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 422, "{written}");
        let text = response.text().await.unwrap();
        assert_eq!(text, expected, "{written}");
        assert!(!text.contains("solution_seal"), "{written}");
    }
}

/// The summary names the approximation the result is in, and the reference
/// names the table it reproduces.
#[tokio::test]
async fn the_summary_says_spin_unpolarized_and_non_relativistic() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "atom_dft" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        described["summary"],
        "Spin-unpolarized, non-relativistic local-density ground state of one spherical neutral atom, atomic number 1 through 18: total energy, its four parts, and each orbital eigenvalue. It reproduces the NIST spin-unpolarized LDA tables and is not the measured energy of the atom."
    );
    let kind = described["reference"]["kind"].as_str().unwrap();
    assert!(kind.contains("spin-unpolarized, non-relativistic"), "{kind}");
    assert!(kind.contains("not a measurement"), "{kind}");
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    assert_eq!(listed.status().as_u16(), 200);
    let text = listed.text().await.unwrap();
    assert!(text.contains("Spin-unpolarized, non-relativistic local-density"), "{text}");
}
