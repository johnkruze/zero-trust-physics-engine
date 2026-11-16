//! The two proton-boron-11 solvers on the HTTP door: `POST /solve` against a
//! direct call to `genesis_solvers`, the bounds at the last number on each
//! side, a constant of a solver sent as a field, each description, a sealed
//! refusal, and `POST /verify_solution` against edits of a sealed answer.

mod answer_text;

use std::time::Duration;

use genesis_solvers::fusion_pb11_lhd::{self as lhd, LhdQuestion};
use genesis_solvers::fusion_pb11_thermal::{self as thermal, ThermalQuestion};
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

fn sha384_hex(bytes: &[u8]) -> String {
    hex::encode(Sha384::digest(bytes))
}

/// The receipt covers the wire bytes before it.
fn assert_receipt(text: &str, body: &Value) {
    let receipt = body["receipt_sha384"].as_str().expect("receipt");
    let tail = format!(",\"receipt_sha384\":\"{receipt}\"}}");
    let document = text.strip_suffix(&tail).expect("the receipt is last");
    assert_eq!(sha384_hex(format!("{document}}}").as_bytes()), receipt);
}

fn beam(question: &LhdQuestion) -> Value {
    json!({ "solver": "fusion_pb11_lhd", "parameters": {
        "beam_energy_kev": question.beam_energy_kev,
        "beam_power_w": question.beam_power_w,
        "electron_density_per_m3": question.electron_density_per_m3,
        "electron_temperature_kev": question.electron_temperature_kev,
        "boron_density_per_m3": question.boron_density_per_m3,
    } })
}

fn plasma(question: &ThermalQuestion) -> Value {
    json!({ "solver": "fusion_pb11_thermal", "parameters": {
        "ion_temperature_kev": question.ion_temperature_kev,
        "electron_density_per_m3": question.electron_density_per_m3,
        "boron_fraction": question.boron_fraction,
        "ash_fraction": question.ash_fraction,
        "magnetic_field_t": question.magnetic_field_t,
        "minor_radius_m": question.minor_radius_m,
        "wall_reflectivity": question.wall_reflectivity,
        "energy_confinement_time_s": question.energy_confinement_time_s,
        "kinetic_enhancement": question.kinetic_enhancement,
    } })
}

const BEAM: LhdQuestion = LhdQuestion {
    beam_energy_kev: 160.0,
    beam_power_w: 2.0e6,
    electron_density_per_m3: 5.0e18,
    electron_temperature_kev: 3.0,
    boron_density_per_m3: 1.0e17,
};

const PLASMA: ThermalQuestion = ThermalQuestion {
    ion_temperature_kev: 300.0,
    electron_density_per_m3: 1.6e20,
    boron_fraction: 0.15,
    ash_fraction: 0.0,
    magnetic_field_t: 10.0,
    minor_radius_m: 2.0,
    wall_reflectivity: 0.95,
    energy_confinement_time_s: 100.0,
    kinetic_enhancement: true,
};

fn field_hash(samples: &[f64]) -> String {
    let mut field = Vec::with_capacity(8 + 8 * samples.len());
    field.extend_from_slice(&(samples.len() as u64).to_le_bytes());
    for sample in samples {
        field.extend_from_slice(&sample.to_le_bytes());
    }
    sha384_hex(&field)
}

#[tokio::test]
async fn two_beams_match_the_solver_bit_for_bit() {
    let base = spawn_app().await;
    // A 45 keV beam of a megawatt in the same plasma: the positive-ion beam,
    // which 0.7.0 to this version answered in one plasma only.
    let positive_ion = LhdQuestion { beam_energy_kev: 45.0, beam_power_w: 1.0e6, ..BEAM };
    for question in [BEAM, positive_ion] {
        let request = beam(&question);
        let (text, body) = solved(&base, &request).await;
        let direct = lhd::solve(&question).expect("the solver settles");

        assert_eq!(body["inputs"], request);
        assert_eq!(body["engine_version"], "genesis_solvers_v0.13.0");
        assert_eq!(body["outcome"], "converged");
        assert!(body["reject"].is_null());
        assert_eq!(body["iterations"], direct.refinements);

        let result = &body["result"];
        assert_number(result, "alpha_emission_rate_per_s", direct.alpha_emission_rate_per_s);
        assert_number(result, "fusion_power_w", direct.fusion_power_w);
        assert_number(result, "fusion_gain", direct.fusion_gain);
        assert_eq!(
            direct.fusion_gain.to_bits(),
            (direct.fused_fraction * genesis_solvers::pb11_reaction::REACTION_ENERGY_KEV / question.beam_energy_kev).to_bits()
        );
        assert_number(result, "fused_fraction", direct.fused_fraction);
        assert_number(result, "peak_alpha_emissivity_per_m3_s", direct.peak_alpha_emissivity_per_m3_s);
        assert_number(result, "peak_emissivity_radius", direct.peak_emissivity_radius);
        assert_number(result, "slowing_time_s", direct.slowing_time_s);
        assert_number(result, "fast_proton_fraction_max", direct.fast_proton_fraction_max);
        assert_number(result, "energy_fraction_to_electrons", direct.energy_fraction_to_electrons);
        assert_number(result, "energy_fraction_to_ions", direct.energy_fraction_to_ions);
        assert_eq!(result.as_object().unwrap().len(), 20, "a number and its hex for each");

        let invariants = &body["invariants"];
        assert_number(invariants, "proton_ledger_residual", direct.proton_ledger_residual);
        assert_number(invariants, "energy_ledger_residual", direct.energy_ledger_residual);
        assert_eq!(invariants.as_object().unwrap().len(), 4);
        assert!(direct.proton_ledger_residual <= lhd::PROTON_LEDGER_TOLERANCE);
        assert!(direct.energy_ledger_residual <= lhd::ENERGY_LEDGER_TOLERANCE);

        assert_eq!(direct.alpha_emissivity_per_m3_s.len(), lhd::SHELLS);
        assert_eq!(body["field_sha384"], field_hash(&direct.alpha_emissivity_per_m3_s));
        assert_eq!(body["solution_seal"].as_str().unwrap().len(), 96);
        assert_receipt(&text, &body);
        assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    }
}

#[tokio::test]
async fn two_plasmas_match_the_solver_bit_for_bit() {
    let base = spawn_app().await;
    let ignition = ThermalQuestion {
        electron_density_per_m3: 1.6e20,
        magnetic_field_t: 20.0,
        wall_reflectivity: 1.0,
        energy_confinement_time_s: 100.0,
        ..PLASMA
    };
    for question in [PLASMA, ignition] {
        let request = plasma(&question);
        let (text, body) = solved(&base, &request).await;
        let direct = thermal::solve(&question).expect("the solver settles");

        assert_eq!(body["inputs"], request);
        assert_eq!(body["outcome"], "converged");
        assert_eq!(body["iterations"], direct.iterations);
        let result = &body["result"];
        assert_number(result, "electron_temperature_kev", direct.electron_temperature_kev);
        assert_number(result, "reactivity_m3_per_s", direct.reactivity_m3_per_s);
        assert_number(result, "fusion_power_w_per_m3", direct.fusion_power_w_per_m3);
        assert_number(result, "bremsstrahlung_power_w_per_m3", direct.bremsstrahlung_power_w_per_m3);
        assert_number(result, "bremsstrahlung_power_xie_w_per_m3", direct.bremsstrahlung_power_xie_w_per_m3);
        assert_number(result, "synchrotron_power_w_per_m3", direct.synchrotron_power_w_per_m3);
        assert_number(result, "ion_electron_power_w_per_m3", direct.ion_electron_power_w_per_m3);
        assert_number(result, "alpha_energy_fraction_to_electrons", direct.alpha_energy_fraction_to_electrons);
        assert_number(result, "auxiliary_power_w_per_m3", direct.auxiliary_power_w_per_m3);
        assert_number(result, "ignition_margin", direct.ignition_margin);
        assert_number(result, "ignition_loss_rate_per_s", direct.ignition_loss_rate_per_s);
        assert_number(result, "pressure_pa", direct.pressure_pa);
        assert_number(result, "beta", direct.beta);
        assert_number(result, "internal_field_t", direct.internal_field_t);
        assert_number(result, "effective_charge", direct.effective_charge);
        assert_eq!(result.as_object().unwrap().len(), 30, "a number and its hex for each");
        let invariants = &body["invariants"];
        assert_number(invariants, "electron_energy_residual", direct.electron_energy_residual);
        assert_number(invariants, "total_energy_residual", direct.total_energy_residual);
        assert_number(invariants, "charge_residual", direct.charge_residual);
        assert_eq!(invariants.as_object().unwrap().len(), 6);
        assert_eq!(direct.balance_terms_w_per_m3.len(), 9);
        assert_eq!(body["field_sha384"], field_hash(&direct.balance_terms_w_per_m3));
        assert_receipt(&text, &body);
        assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    }
    // The switch has no default: left out it is named at the door, and the
    // switch off and the switch on are different sealed requests with
    // different answers.
    let mut off = PLASMA;
    off.kinetic_enhancement = false;
    let (_, written) = solved(&base, &plasma(&off)).await;
    let mut left_out = plasma(&off);
    left_out["parameters"].as_object_mut().unwrap().remove("kinetic_enhancement");
    let (status, text) = post(&base, "/solve", &left_out).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, "Missing required parameter: 'kinetic_enhancement' (InputMissing).");
    let (_, on) = solved(&base, &plasma(&PLASMA)).await;
    assert_ne!(written["solution_seal"], on["solution_seal"]);
    assert_ne!(on["result"]["reactivity_m3_per_s_hex"], written["result"]["reactivity_m3_per_s_hex"]);
}

/// The same question written three ways is one sealed request and one answer.
#[tokio::test]
async fn a_number_written_three_ways_is_one_question() {
    let base = spawn_app().await;
    let mut seals = Vec::new();
    for text in [
        r#"{"solver":"fusion_pb11_lhd","parameters":{"beam_energy_kev":160,"beam_power_w":2000000,"electron_density_per_m3":5e18,"electron_temperature_kev":3,"boron_density_per_m3":1e17}}"#,
        r#"{"solver":"fusion_pb11_lhd","parameters":{"boron_density_per_m3":1.0e17,"electron_temperature_kev":3.0,"electron_density_per_m3":5000000000000000000,"beam_power_w":2e6,"beam_energy_kev":1.6e2}}"#,
        r#"{"parameters":{"beam_energy_kev":160.0,"beam_power_w":2.0e6,"electron_density_per_m3":5.0e18,"electron_temperature_kev":3,"boron_density_per_m3":100000000000000000},"solver":"fusion_pb11_lhd"}"#,
    ] {
        let request: Value = serde_json::from_str(text).unwrap();
        let (_, body) = solved(&base, &request).await;
        seals.push(body["solution_seal"].as_str().unwrap().to_string());
    }
    seals.dedup();
    assert_eq!(seals.len(), 1);
}

#[tokio::test]
async fn the_bounds_hold_at_the_last_number_on_each_side() {
    let base = spawn_app().await;
    let below = |value: f64| f64::from_bits(value.to_bits() - 1);
    let above = |value: f64| f64::from_bits(value.to_bits() + 1);
    // Every limit of the beam is asked by the solver's own check and refused
    // in the solver's words, with the solver's name for the refusal. Each
    // end is a physical one, said in the sentence.
    let beams: [(fn(&mut LhdQuestion, f64), f64, &str, &str); 8] = [
        (|q, v| q.beam_energy_kev = v, lhd::BEAM_ENERGY_MIN_KEV, "beam energy", "BeamEnergyOutOfRange"),
        (|q, v| q.beam_energy_kev = v, above(lhd::BEAM_ENERGY_MAX_KEV), "beam energy", "BeamEnergyOutOfRange"),
        (|q, v| q.beam_power_w = v, 0.0, "beam power", "BeamPowerOutOfRange"),
        (|q, v| q.electron_density_per_m3 = v, below(lhd::ELECTRON_DENSITY_MIN_PER_M3), "electron density", "ElectronDensityOutOfRange"),
        (|q, v| q.electron_density_per_m3 = v, above(lhd::ELECTRON_DENSITY_MAX_PER_M3), "electron density", "ElectronDensityOutOfRange"),
        (|q, v| q.electron_temperature_kev = v, below(lhd::ELECTRON_TEMPERATURE_MIN_KEV), "electron temperature", "ElectronTemperatureOutOfRange"),
        (|q, v| q.electron_temperature_kev = v, above(lhd::ELECTRON_TEMPERATURE_MAX_KEV), "electron temperature", "ElectronTemperatureOutOfRange"),
        (|q, v| q.boron_density_per_m3 = v, -1.0, "boron density", "BoronDensityOutOfRange"),
    ];
    for (change, value, word, name) in beams {
        let mut question = BEAM;
        change(&mut question, value);
        let (status, text) = post(&base, "/solve", &beam(&question)).await;
        assert_eq!(status, 422, "{word} {value}: {text}");
        assert!(text.starts_with(&format!("Physical Violation: {word} ")), "{text}");
        assert!(!text.contains("compared"), "{text}");
        assert!(text.ends_with(&format!("({name}).")), "{text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // The ends of each range are inside it: answered with a proof, or a
    // sealed physical limit of the solve.
    let ends: [(fn(&mut LhdQuestion, f64), f64); 6] = [
        (|q, v| q.beam_energy_kev = v, above(lhd::BEAM_ENERGY_MIN_KEV)),
        (|q, v| q.beam_energy_kev = v, lhd::BEAM_ENERGY_MAX_KEV),
        (|q, v| q.electron_density_per_m3 = v, lhd::ELECTRON_DENSITY_MIN_PER_M3),
        (|q, v| q.electron_density_per_m3 = v, lhd::ELECTRON_DENSITY_MAX_PER_M3),
        (|q, v| q.electron_temperature_kev = v, lhd::ELECTRON_TEMPERATURE_MIN_KEV),
        (|q, v| q.electron_temperature_kev = v, lhd::ELECTRON_TEMPERATURE_MAX_KEV),
    ];
    for (change, value) in ends {
        let mut question = LhdQuestion { boron_density_per_m3: 5.0e16, beam_power_w: 1.0e5, ..BEAM };
        change(&mut question, value);
        let (_, body) = solved(&base, &beam(&question)).await;
        assert_eq!(body["outcome"], "converged", "{value}");
    }
    // The 45 keV beam of run A5, in the plasma it was compared in, leaves
    // more fast protons than electrons at the axis: a sealed refusal.
    let positive_ion = json!({ "solver": "fusion_pb11_lhd", "parameters": {"beam_energy_kev":45,"beam_power_w":4515000,"electron_density_per_m3":4.502e18,"electron_temperature_kev":2.1252,"boron_density_per_m3":8.1234e16} });
    let (_, body) = solved(&base, &positive_ion).await;
    assert_eq!(body["outcome"], "refused");
    assert_eq!(body["reject"], "FastProtonChargeExceedsElectrons");
    // The most boron for its electrons compared, at its own point.
    let most_boron = LhdQuestion {
        beam_energy_kev: 127.45,
        beam_power_w: 1.4056e6,
        electron_density_per_m3: 6.872e18,
        electron_temperature_kev: 3.3333,
        boron_density_per_m3: 2.84682e17,
    };
    let (_, body) = solved(&base, &beam(&most_boron)).await;
    assert_eq!(body["outcome"], "converged");
    // Four requests that 0.6.0 answered with a proof outside every
    // comparison, refused at the door from 0.7.0 to this version, are
    // admitted again; a subnormal power is refused by the number rule.
    for (parameters, answered) in [
        (json!({"beam_energy_kev":100,"beam_power_w":1000000,"electron_density_per_m3":3e19,"electron_temperature_kev":6,"boron_density_per_m3":7e17}), true),
        (json!({"beam_energy_kev":190,"beam_power_w":1000000,"electron_density_per_m3":1e18,"electron_temperature_kev":6,"boron_density_per_m3":1e17}), false),
        (json!({"beam_energy_kev":160,"beam_power_w":1000000,"electron_density_per_m3":1e18,"electron_temperature_kev":3,"boron_density_per_m3":1.036081e17}), false),
        (json!({"beam_energy_kev":180,"beam_power_w":4330000,"electron_density_per_m3":1.96e19,"electron_temperature_kev":5.4,"boron_density_per_m3":6.3e16}), true),
    ] {
        let request = json!({ "solver": "fusion_pb11_lhd", "parameters": parameters });
        let (_, body) = solved(&base, &request).await;
        if answered {
            assert_eq!(body["outcome"], "converged", "{request}");
        } else {
            assert_eq!(body["outcome"], "refused", "{request}");
            assert_eq!(body["reject"], "FastProtonChargeExceedsElectrons", "{request}");
        }
    }
    let subnormal = json!({ "solver": "fusion_pb11_lhd", "parameters": {"beam_energy_kev":160,"beam_power_w":5e-324,"electron_density_per_m3":3.137e18,"electron_temperature_kev":3.2438,"boron_density_per_m3":8.6706e16} });
    let (status, text) = post(&base, "/solve", &subnormal).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, "Physical Violation: 'beam_power_w' is not a finite normal number.");

    // Each closed range of the plasma is refused as the field list reads
    // it: by the field's name, with the solver's name for the refusal. The
    // field and the confinement time, open above zero, are asked of the
    // solver's own check and refused in its words.
    let plasmas: [(fn(&mut ThermalQuestion, f64), f64, &str, &str); 12] = [
        (|q, v| q.ion_temperature_kev = v, below(thermal::ION_TEMPERATURE_MIN_KEV), "ion_temperature_kev", "IonTemperatureOutOfRange"),
        (|q, v| q.ion_temperature_kev = v, above(thermal::ION_TEMPERATURE_MAX_KEV), "ion_temperature_kev", "IonTemperatureOutOfRange"),
        (|q, v| q.electron_density_per_m3 = v, below(thermal::ELECTRON_DENSITY_MIN_PER_M3), "electron_density_per_m3", "ElectronDensityOutOfRange"),
        (|q, v| q.electron_density_per_m3 = v, above(thermal::ELECTRON_DENSITY_MAX_PER_M3), "electron_density_per_m3", "ElectronDensityOutOfRange"),
        (|q, v| q.boron_fraction = v, below(thermal::BORON_FRACTION_MIN), "boron_fraction", "BoronFractionOutOfRange"),
        (|q, v| q.boron_fraction = v, above(thermal::BORON_FRACTION_MAX), "boron_fraction", "BoronFractionOutOfRange"),
        (|q, v| q.ash_fraction = v, -1.0e-9, "ash_fraction", "AshFractionOutOfRange"),
        (|q, v| q.ash_fraction = v, above(thermal::ASH_FRACTION_MAX), "ash_fraction", "AshFractionOutOfRange"),
        (|q, v| q.minor_radius_m = v, below(thermal::MINOR_RADIUS_MIN_M), "minor_radius_m", "MinorRadiusOutOfRange"),
        (|q, v| q.minor_radius_m = v, above(thermal::MINOR_RADIUS_MAX_M), "minor_radius_m", "MinorRadiusOutOfRange"),
        (|q, v| q.wall_reflectivity = v, -1.0e-9, "wall_reflectivity", "WallReflectivityOutOfRange"),
        (|q, v| q.wall_reflectivity = v, above(thermal::WALL_REFLECTIVITY_MAX), "wall_reflectivity", "WallReflectivityOutOfRange"),
    ];
    for (change, value, word, name) in plasmas {
        let mut question = PLASMA;
        change(&mut question, value);
        let (status, text) = post(&base, "/solve", &plasma(&question)).await;
        assert_eq!(status, 422, "{word} {value}: {text}");
        assert!(text.starts_with(&format!("Physical Violation: '{word}' ")), "{text}");
        assert!(text.contains("is outside"), "{text}");
        assert!(text.ends_with(&format!("({name}).")), "{text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    let open: [(fn(&mut ThermalQuestion, f64), f64, &str, &str); 3] = [
        (|q, v| q.magnetic_field_t = v, 0.0, "magnetic field", "MagneticFieldOutOfRange"),
        (|q, v| q.magnetic_field_t = v, above(thermal::MAGNETIC_FIELD_MAX_T), "magnetic field", "MagneticFieldOutOfRange"),
        (|q, v| q.energy_confinement_time_s = v, 0.0, "energy confinement time", "ConfinementTimeOutOfRange"),
    ];
    for (change, value, word, name) in open {
        let mut question = PLASMA;
        change(&mut question, value);
        let (status, text) = post(&base, "/solve", &plasma(&question)).await;
        assert_eq!(status, 422, "{word} {value}: {text}");
        assert!(text.starts_with(&format!("Physical Violation: {word} ")), "{text}");
        assert!(text.ends_with(&format!("({name}).")), "{text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // The ends of each range are inside it: answered with a proof, or a
    // sealed physical limit of the solve.
    let ends: [(fn(&mut ThermalQuestion, f64), f64); 10] = [
        (|q, v| q.ion_temperature_kev = v, thermal::ION_TEMPERATURE_MIN_KEV),
        (|q, v| q.ion_temperature_kev = v, thermal::ION_TEMPERATURE_MAX_KEV),
        (|q, v| q.electron_density_per_m3 = v, thermal::ELECTRON_DENSITY_MIN_PER_M3),
        (|q, v| q.electron_density_per_m3 = v, thermal::ELECTRON_DENSITY_MAX_PER_M3),
        (|q, v| q.boron_fraction = v, thermal::BORON_FRACTION_MIN),
        (|q, v| q.boron_fraction = v, thermal::BORON_FRACTION_MAX),
        (|q, v| q.ash_fraction = v, thermal::ASH_FRACTION_MAX),
        (|q, v| q.magnetic_field_t = v, thermal::MAGNETIC_FIELD_MAX_T),
        (|q, v| q.wall_reflectivity = v, thermal::WALL_REFLECTIVITY_MIN),
        (|q, v| q.wall_reflectivity = v, thermal::WALL_REFLECTIVITY_MAX),
    ];
    for (change, value) in ends {
        let mut question = PLASMA;
        change(&mut question, value);
        let (_, body) = solved(&base, &plasma(&question)).await;
        let named = ["PressureExceedsField", "ElectronTemperatureOutOfRange", "SynchrotronAboveUnabsorbedEmission"];
        assert!(body["outcome"] == "converged" || named.contains(&body["reject"].as_str().unwrap_or("")), "{value}: {body}");
    }
    // A corner of the 0.6.0 range, refused at the door from 0.7.0 to this version, is answered with a proof.
    let corner = json!({ "solver": "fusion_pb11_thermal", "parameters": {"ion_temperature_kev":300,"electron_density_per_m3":1e21,"boron_fraction":0.5,"ash_fraction":0.1,"magnetic_field_t":40,"minor_radius_m":10,"wall_reflectivity":0,"energy_confinement_time_s":100000,"kinetic_enhancement":true} });
    let (_, body) = solved(&base, &corner).await;
    assert_eq!(body["outcome"], "converged");
    // Below 10 keV, refused at the door until this version, down to the
    // electrons' 1 keV floor: answered with a proof that holds.
    for ion_temperature_kev in [1.25, 2.0, 5.0, 9.99] {
        let (_, body) = solved(&base, &plasma(&ThermalQuestion { ion_temperature_kev, ..PLASMA })).await;
        assert_eq!(body["outcome"], "converged", "{ion_temperature_kev}: {body}");
        assert_eq!(verdict(&base, &body).await["holds"], true, "{ion_temperature_kev}");
    }
}

/// The plasma the 2026-10-09 run sent at 4 T, a sealed
/// `PressureExceedsField` then and refused at the door from 0.7.0 to this
/// version, is a sealed `PressureExceedsField` again: the field cannot hold
/// it.
#[tokio::test]
async fn a_plasma_the_field_cannot_hold_is_a_sealed_refusal() {
    let base = spawn_app().await;
    let question = ThermalQuestion { magnetic_field_t: 4.0, ..PLASMA };
    let (text, body) = solved(&base, &plasma(&question)).await;
    assert_eq!(body["outcome"], "refused", "{text}");
    assert_eq!(body["reject"], "PressureExceedsField");
    assert_eq!(body["solution_seal"].as_str().unwrap().len(), 96);
    assert!(matches!(thermal::solve(&question), Err(thermal::ThermalReject::PressureExceedsField { .. })));
    assert_eq!(verdict(&base, &body).await["holds"], true);
}

#[tokio::test]
async fn what_is_not_a_question_of_these_solvers_is_refused_before_the_solver() {
    let base = spawn_app().await;
    let good_beam = beam(&BEAM);
    let good_plasma = plasma(&PLASMA);
    let with = |request: &Value, key: &str, value: Value| {
        let mut changed = request.clone();
        changed["parameters"][key] = value;
        changed
    };
    let without = |request: &Value, key: &str| {
        let mut changed = request.clone();
        changed["parameters"].as_object_mut().unwrap().remove(key);
        changed
    };
    for (request, message) in [
        (with(&good_beam, "beam_energy_kev", json!("160")), "'beam_energy_kev' must be a number"),
        (with(&good_beam, "boron_density_per_m3", json!(null)), "'boron_density_per_m3' must be a number"),
        (without(&good_beam, "beam_power_w"), "Missing required parameter: 'beam_power_w' (InputMissing)."),
        (without(&good_beam, "boron_density_per_m3"), "Missing required parameter: 'boron_density_per_m3' (InputMissing)."),
        (json!({ "solver": "fusion_pb11_lhd" }), "Missing required parameter: 'parameters' (or 'example', the name of a worked example describe_solver lists) (InputMissing)."),
        // A constant of the solver sent as a field.
        (with(&good_beam, "boron11_fraction", json!(1.0)), "'boron11_fraction' is not an input of this solver"),
        (with(&good_beam, "shells", json!(4)), "'shells' is not an input of this solver"),
        (with(&good_beam, "ion_temperature_kev", json!(5.0)), "'ion_temperature_kev' is not an input of this solver"),
        // The other solver's question, and the atom's.
        (with(&good_beam, "wall_reflectivity", json!(0.9)), "'wall_reflectivity' is not an input of this solver"),
        (with(&good_beam, "atomic_number", json!(5)), "'atomic_number' is not an input of this solver"),
        (without(&good_plasma, "wall_reflectivity"), "Missing required parameter: 'wall_reflectivity' (InputMissing)."),
        (without(&good_plasma, "energy_confinement_time_s"), "Missing required parameter: 'energy_confinement_time_s' (InputMissing)."),
        (without(&good_plasma, "kinetic_enhancement"), "Missing required parameter: 'kinetic_enhancement' (InputMissing)."),
        (with(&good_plasma, "kinetic_enhancement", json!("true")), "'kinetic_enhancement' must be true or false"),
        (with(&good_plasma, "kinetic_enhancement", json!(1)), "'kinetic_enhancement' must be true or false"),
        (with(&good_plasma, "electron_temperature_kev", json!(150)), "'electron_temperature_kev' is not an input of this solver"),
        (with(&good_plasma, "alpha_channeling", json!(0.5)), "'alpha_channeling' is not an input of this solver"),
        (with(&good_plasma, "spin_polarisation", json!(true)), "'spin_polarisation' is not an input of this solver"),
        (with(&good_plasma, "coulomb_logarithm", json!(15)), "'coulomb_logarithm' is not an input of this solver"),
        (with(&good_plasma, "beam_energy_kev", json!(160)), "'beam_energy_kev' is not an input of this solver"),
        (json!({ "solver": "atom_dft", "parameters": { "ion_temperature_kev": 300 } }), "'ion_temperature_kev' is not an input of this solver"),
    ] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request}: {text}");
        assert!(text.contains(message), "{request}: {text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    let (status, text) = post(&base, "/solve", &json!({ "solver": "fusion_pb11", "parameters": {} })).await;
    assert_eq!(status, 404, "{text}");
}

#[tokio::test]
async fn both_are_listed_and_described_and_each_example_runs_unchanged() {
    let base = spawn_app().await;
    let listed = reqwest::get(format!("{base}/solvers")).await.unwrap();
    let listed: Value = serde_json::from_str(&listed.text().await.unwrap()).unwrap();
    let solvers = listed["solvers"].as_array().expect("solvers");
    assert_eq!(solvers.len(), protocol_mcp_gateway::SOLVERS);
    assert_eq!(solvers[7]["solver"], "fusion_pb11_lhd");
    assert_eq!(solvers[8]["solver"], "fusion_pb11_thermal");
    for listed in &solvers[7..9] {
        let summary = listed["summary"].as_str().unwrap();
        assert!(summary.ends_with('.'));
    }
    // Since 0.14.4 the compared solver's row gives what it computes, the
    // comparison and the gap; what was fitted is in its description.
    assert!(!solvers[7]["summary"].as_str().unwrap().contains("Nothing fitted"));
    assert!(solvers[8]["summary"].as_str().unwrap().contains("Nothing fitted"));
    assert!(solvers[8]["summary"].as_str().unwrap().contains("Held to the published calculations of Ochs et al. 2022"));
    assert!(solvers[7]["summary"].as_str().unwrap().contains("the six beam-energy points in the measured order (Spearman 0.943), worst factor 2.94"));
    for place in [7, 8] {
        assert!(!solvers[place]["summary"].as_str().unwrap().contains("missed"), "{place}");
    }

    for (solver, inputs, results, invariants, refusals) in
        [("fusion_pb11_lhd", 5, 10, 2, 13), ("fusion_pb11_thermal", 9, 15, 3, 16)]
    {
        let (status, text) = post(&base, "/describe_solver", &json!({ "solver": solver })).await;
        assert_eq!(status, 200, "{text}");
        let described: Value = serde_json::from_str(&text).unwrap();
        assert!(described["summary"].as_str().unwrap().contains("Nothing fitted"), "{solver}");
        assert_eq!(described["inputs"].as_array().unwrap().len(), inputs);
        assert_eq!(described["refusals"].as_array().unwrap().len(), refusals);
        for refusal in described["refusals"].as_array().unwrap() {
            assert!(refusal["meaning"].as_str().unwrap().ends_with('.'));
        }
        assert_eq!(described["reference"]["retrieved"], "2026-10-09");
        assert_eq!(described["reference"]["kind"], "A reference calculation. It is not a measurement.");
        // The record is in the five parts every solver's record has; what
        // the shared cross-section was held to is with the locked constants.
        assert_eq!(described["validation"].as_object().unwrap().len(), 5);
        assert!(described["validation"]["limits"].as_str().unwrap().starts_with("Fitted constants: None"));
        assert_eq!(described["locked"]["cross_section"]["against_nacre_ii"]["within"], false);
        assert_eq!(described["locked"]["cross_section"]["against_nacre_ii"]["inside_its_stated_band"], 30);
        // Every result and invariant the description names is in a solution, and no other.
        let (_, body) = solved(&base, &described["example_solve"]).await;
        assert_eq!(body["outcome"], "converged");
        assert_eq!(body["inputs"], described["example_solve"]);
        for (section, count) in [("result", results), ("invariants", invariants)] {
            let named = described["returns"][section].as_array().expect("names");
            assert_eq!(named.len(), count);
            for entry in named {
                let name = entry["name"].as_str().unwrap();
                assert!(body[section][name].is_number(), "{section}.{name}");
                assert!(body[section][format!("{name}_hex")].is_string(), "{section}.{name}_hex");
            }
            assert_eq!(body[section].as_object().unwrap().len(), 2 * count);
        }
        assert_eq!(verdict(&base, &body).await, json!({ "holds": true, "values_hold": true, "cause": null, "compared": protocol_mcp_gateway::WHOLE_SOLUTION_COMPUTED }));
    }
    let (_, text) = post(&base, "/describe_solver", &json!({ "solver": "fusion_pb11_lhd" })).await;
    let described: Value = serde_json::from_str(&text).unwrap();
    // The beam energy is above 10 keV to 190 keV, and the 45 keV beam joins the band.
    assert_eq!(described["inputs"][0]["minimum"], 10.0);
    assert_eq!(described["inputs"][0]["minimum_exclusive"], true);
    assert_eq!(described["inputs"][0]["maximum"], 190.0);
    // The beam power is any power above zero, to the capacity of the input.
    assert_eq!(described["inputs"][1]["name"], "beam_power_w");
    assert_eq!(described["inputs"][1]["minimum"], 0.0);
    assert_eq!(described["inputs"][1]["minimum_exclusive"], true);
    assert_eq!(described["inputs"][1]["maximum"], 1.0e300);
    // No power is no beam; the 0.9389 MW floor of 0.7.0 to this version is gone.
    let mut tiny: Value = serde_json::from_str(r#"{"solver":"fusion_pb11_lhd","parameters":{"beam_energy_kev":160,"beam_power_w":0,"electron_density_per_m3":5e18,"electron_temperature_kev":3,"boron_density_per_m3":1e17}}"#).unwrap();
    let (status, text) = post(&base, "/solve", &tiny).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("BeamPowerOutOfRange"), "{text}");
    tiny["parameters"]["beam_power_w"] = json!(938899.9);
    let (_, body) = solved(&base, &tiny).await;
    assert_eq!(body["outcome"], "converged");
    // The smallest number above zero is a subnormal, refused first by the
    // number rule every request follows.
    tiny["parameters"]["beam_power_w"] = json!(5e-324);
    let (status, text) = post(&base, "/solve", &tiny).await;
    assert_eq!(status, 422, "{text}");
    assert_eq!(text, "Physical Violation: 'beam_power_w' is not a finite normal number.");
    // The published calculation's plasma is answered, and the reference says so.
    assert!(described["reference"]["covers"].as_str().unwrap().contains("The served solver answers this case"));
    let limits = described["validation"]["limits"].as_str().unwrap();
    assert!(limits.contains("The declared range is where the solver's equations hold"), "{limits}");
    assert!(limits.contains("Both beam families, the negative-ion beams and the 45 keV beam, share one range"), "{limits}");
    assert_eq!(described["locked"]["boron11_fraction"], 0.80);
    assert_eq!(described["locked"]["shells"], 20);
    // The comparison with the published calculation is stated in numbers beside its target.
    let result = described["validation"]["result"].as_str().unwrap();
    assert!(result.starts_with("Against the published calculation, with the boron of the paper's text"), "{result}");
    assert!(result.contains("with the boron of the paper's text, 6.3e16 per m^3: alpha rate 1.921e13 per second, solver over published 0.128 (target 0.5 to 2.0)."), "{result}");
    // So are the comparisons with the measured count rates.
    assert!(result.contains("Beam energy dependence: worst factor 2.94 (target 2), Spearman rank correlation 0.943, rise from 128 to 160 kV measured 2.10 and solver 11.1."), "{result}");
    // The record says which beam voltage the runs took, with the protocol's own beside it.
    assert!(result.contains("with the top panel's voltages the worst factor is 2.953 and the rise 10.7"), "{result}");
    assert!(result.contains("Time shape: worst factor 2.42 (target 2)"), "{result}");
    assert!(result.contains("Collapse under the 45 keV beam: solver ratio 1.26e-5, measured ratio 0.062 (target: at most the measured ratio)"), "{result}");
    assert_eq!(result.matches("(target").count(), 4);
    assert!(result.contains("Absolute count rate, with no target:") && result.contains("4.7 times above"), "{result}");
    let (_, text) = post(&base, "/describe_solver", &json!({ "solver": "fusion_pb11_thermal" })).await;
    let described: Value = serde_json::from_str(&text).unwrap();
    let result = described["validation"]["result"].as_str().unwrap();
    assert!(result.contains("7.9 percent off (target 5 percent)"), "{result}");
    assert!(described["validation"]["data"].as_str().unwrap().starts_with("Published calculations:"));
    assert!(described["validation"]["object"].as_str().unwrap().starts_with("A uniform thermal plasma"));
    // The switch is a field like the others: true or false, with no default.
    assert_eq!(described["inputs"][8]["name"], "kinetic_enhancement");
    // A request without the flag is a request with a key missing; its sentence names the key, as InputMissing.
    assert!(described["inputs"][8]["bound"].as_str().unwrap().starts_with("Required, true or false. It has no default, and a request that leaves it out is refused as a missing key, in a sentence that names the key, as InputMissing"));
    assert_eq!(described["inputs"][8]["type"], "true or false");
    assert!(described["inputs"][8]["bound"].as_str().unwrap().contains("It is theory"));
    // Five comparisons with published calculations, two with the plain-Python model and the closure, each beside its target.
    assert_eq!(result.matches("(target ").count(), 8);
    // The atom is described as before, and a name between is none.
    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "atom_dft" })).await;
    assert_eq!(status, 200, "{text}");
    assert!(!text.contains("beam_energy_kev"));
    let (status, _) = post(&base, "/describe_solver", &json!({ "solver": "fusion_pb11_lhd " })).await;
    assert_eq!(status, 404);
}

/// A 4.7 T, 0.1 m bare-wall plasma, which 0.6.0 refused as
/// `PressureExceedsField` although the field holds a balance near 105.6 keV,
/// and which 0.7.0 to this version refused at the door, is answered with a
/// proof. The door says how the first of several balances is chosen, what
/// the ignition loss rate is over, and which region was compared.
#[tokio::test]
async fn requests_at_the_edges_of_the_range_are_answered_truly_and_described() {
    let base = spawn_app().await;
    let question = ThermalQuestion {
        ion_temperature_kev: 300.0,
        electron_density_per_m3: 1.6e20,
        boron_fraction: 0.15,
        ash_fraction: 0.0,
        magnetic_field_t: 4.7,
        minor_radius_m: 0.1,
        wall_reflectivity: 0.0,
        energy_confinement_time_s: 100.0,
        kinetic_enhancement: false,
    };
    for field in [4.7, 4.701] {
        let asked = ThermalQuestion { magnetic_field_t: field, ..question };
        let (_, body) = solved(&base, &plasma(&asked)).await;
        assert_eq!(body["outcome"], "converged", "{field}");
        let direct = thermal::solve(&asked).expect("settles");
        assert_number(&body["result"], "electron_temperature_kev", direct.electron_temperature_kev);
        assert!((direct.electron_temperature_kev - 105.6).abs() < 0.2, "{}", direct.electron_temperature_kev);
    }
    // One over the ignition loss rate as the confinement time is answered: the auxiliary power is below zero there.
    let mut d2 = plasma(&ThermalQuestion { magnetic_field_t: 20.0, wall_reflectivity: 1.0, ..PLASMA });
    d2["parameters"]["energy_confinement_time_s"] = json!(532.3461727957897);
    let (_, body) = solved(&base, &d2).await;
    assert_eq!(body["outcome"], "converged");
    assert!(body["result"]["auxiliary_power_w_per_m3"].as_f64().unwrap() < 0.0);

    let (_, text) = post(&base, "/describe_solver", &json!({ "solver": "fusion_pb11_thermal" })).await;
    let described: Value = serde_json::from_str(&text).unwrap();
    let note = |name: &str| -> String {
        described["returns"]["result"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("{name}"))
            .to_string()
    };
    assert!(note("electron_temperature_kev").contains("turns from heating to loss"));
    let rate = note("ignition_loss_rate_per_s");
    assert!(rate.contains("stored energy of ions and electrons") && rate.contains("It differs from one over energy_confinement_time_s, which counts the ions alone"), "{rate}");
    assert!(!rate.contains("One over the confinement time"), "{rate}");
    assert_eq!(described["locked"]["electron_grid_step_kev"], thermal::ELECTRON_GRID_STEP_KEV);
    assert!(described["validation"]["result"].as_str().unwrap().contains("Total-energy ignition confinement time"));
    // The declared range is where the physics holds; the compared one is stated beside it.
    let limits = described["validation"]["limits"].as_str().unwrap();
    assert!(limits.contains("The declared range is where the balance's own forms hold"), "{limits}");
    assert!(limits.contains("The comparison covered ion temperature 200 to 450 keV"), "{limits}");
    assert!(described["summary"].as_str().unwrap().contains("ion temperature 1 to 1000 keV"));
    let inputs = described["inputs"].as_array().unwrap();
    for (index, name, minimum, maximum) in [
        (0, "ion_temperature_kev", 1.0, 1000.0),
        (1, "electron_density_per_m3", 1.0e18, 1.0e22),
        (2, "boron_fraction", 0.01, 0.9),
        (3, "ash_fraction", 0.0, 1.0),
        (4, "magnetic_field_t", 0.0, 1.0e6),
        (5, "minor_radius_m", 0.1, 10.0),
        (6, "wall_reflectivity", 0.0, 1.0),
        (7, "energy_confinement_time_s", 0.0, 1.0e300),
    ] {
        assert_eq!(inputs[index]["name"], name);
        assert_eq!(inputs[index]["minimum"], minimum, "{name}");
        assert_eq!(inputs[index]["maximum"], maximum, "{name}");
    }
}

#[tokio::test]
async fn an_edited_answer_is_named_by_its_part() {
    let base = spawn_app().await;
    let (_, body) = solved(&base, &beam(&BEAM)).await;

    // More alphas than the solver found.
    let mut more = body.clone();
    let rate = body["result"]["alpha_emission_rate_per_s"].as_f64().unwrap();
    more["result"]["alpha_emission_rate_per_s_hex"] = json!(hex_of(10.0 * rate));
    assert_fails(&base, &more, "result", "ten times the alphas").await;

    // The decimal alone is not in the seal; the hex is the value. Since
    // 0.11.0 the long form writes the answer again from the hex and names
    // the decimal that is not its value.
    let mut decimal = body.clone();
    decimal["result"]["alpha_emission_rate_per_s"] = json!(10.0 * rate);
    let report = verdict(&base, &decimal).await;
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["differs_at"].clone()), (json!(false), json!("answer_text"), json!("result.alpha_emission_rate_per_s")), "{report}");

    let mut closed = body.clone();
    closed["invariants"]["energy_ledger_residual_hex"] = json!(hex_of(0.0));
    assert_fails(&base, &closed, "invariants", "a better ledger").await;

    let mut more_boron = body.clone();
    more_boron["inputs"]["parameters"]["boron_density_per_m3"] = json!(2.0e17);
    assert_fails(&base, &more_boron, "inputs", "more boron").await;

    let mut other_field = body.clone();
    other_field["field_sha384"] = json!("0".repeat(96));
    assert_fails(&base, &other_field, "field_sha384", "another profile").await;

    let mut fewer = body.clone();
    fewer["iterations"] = json!(1);
    assert_fails(&base, &fewer, "outcome", "fewer refinements").await;

    // A beam answer cannot carry the thermal solver's names, nor be relabelled as it.
    let mut borrowed = body.clone();
    borrowed["result"]["ignition_margin_hex"] = json!(hex_of(0.5));
    let (status, text) = post(&base, "/verify_solution", &borrowed).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("'result.ignition_margin_hex' is not published by this solver"), "{text}");
    let mut relabelled = body.clone();
    relabelled["inputs"]["solver"] = json!("fusion_pb11_thermal");
    let (status, text) = post(&base, "/verify_solution", &relabelled).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("is not published by this solver"), "{text}");

    // The kinetic switch is in the sealed request: an answer got with it on cannot be shown as got with it off.
    let (_, on) = solved(&base, &plasma(&PLASMA)).await;
    let mut shown_off = on.clone();
    shown_off["inputs"]["parameters"]["kinetic_enhancement"] = json!(false);
    assert_fails(&base, &shown_off, "inputs", "the switch turned off afterwards").await;
}

/// The thermal solver's summary names two requests and the numbers of each:
/// the comparison's request of Ochs et al. (2022) at 300 keV, and the worked
/// example, another plasma. Each is solved here to the numbers it states.
#[tokio::test]
async fn the_summary_states_the_numbers_of_the_comparison_and_of_the_worked_example() {
    let base = spawn_app().await;
    let (status, described) = post(&base, "/describe_solver", &json!({ "solver": "fusion_pb11_thermal" })).await;
    assert_eq!(status, 200, "{described}");
    let described: Value = serde_json::from_str(&described).unwrap();
    let summary = described["summary"].as_str().unwrap();
    let ochs = json!({ "solver": "fusion_pb11_thermal", "parameters": {
        "ion_temperature_kev": 300, "electron_density_per_m3": 1.52e20, "boron_fraction": 0.13, "ash_fraction": 0,
        "magnetic_field_t": 20, "minor_radius_m": 2, "wall_reflectivity": 1, "energy_confinement_time_s": 100,
        "kinetic_enhancement": true } });
    let (_, compared) = solved(&base, &ochs).await;
    let electrons = compared["result"]["electron_temperature_kev"].as_f64().unwrap();
    assert_eq!(format!("{electrons:.2}"), "159.33");
    assert_eq!(format!("{:.3}", compared["result"]["ignition_margin"].as_f64().unwrap()), "0.037");
    assert!(summary.contains("gives an electron temperature of 159.33 keV against a published 163.0, and an ignition margin of 0.037"), "{summary}");
    let (_, worked) = solved(&base, &described["example_solve"]).await;
    let electrons = worked["result"]["electron_temperature_kev"].as_f64().unwrap();
    let margin = worked["result"]["ignition_margin"].as_f64().unwrap();
    assert_eq!((format!("{electrons:.3}"), format!("{margin:.3}")), ("113.152".to_string(), "-1.042".to_string()));
    assert!(margin < 0.0);
    assert!(summary.contains("an electron temperature of 113.152 keV and an ignition margin of -1.042, so it does not ignite"), "{summary}");
}
