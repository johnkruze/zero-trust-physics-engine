//! Engine invariant 4 on the three models new in engine 0.13.0 that a
//! request can drive under the normal range, through the local connection:
//! no answer publishes a subnormal number, and a state the engine sealed
//! steps on (the red team of 0.13.0, D1 and D2). Each value under 2^-1022 is
//! written as zero with its sign where it is computed, in the card's step and
//! the solver's result (`genesis_kernel::normal`, `genesis_solvers::normal`).

mod conformance_support;

use conformance_support::Door;
use serde_json::{Value, json};

/// Every subnormal number an answer holds, as a decimal or as `_hex`, by its
/// path.
fn subnormal_numbers(value: &Value, path: &str, out: &mut Vec<String>) {
    match value {
        Value::Number(number) => {
            if number.as_f64().is_some_and(f64::is_subnormal) {
                out.push(format!("{path} = {number}"));
            }
        }
        Value::String(text) if path.ends_with("_hex") || path.contains("_hex[") => {
            if let Ok(bits) = u64::from_str_radix(text, 16) {
                if text.len() == 16 && f64::from_bits(bits).is_subnormal() {
                    out.push(format!("{path} = {text}"));
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                subnormal_numbers(item, &format!("{path}[{index}]"), out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                subnormal_numbers(item, &if path.is_empty() { key.clone() } else { format!("{path}.{key}") }, out);
            }
        }
        _ => {}
    }
}

/// The answer to `arguments`, which must not be an error and must hold no
/// subnormal number.
fn answered(door: &mut Door, tool: &str, arguments: &Value) -> Value {
    let (error, text) = door.call(tool, arguments);
    assert!(!error, "{text}");
    let answer: Value = serde_json::from_str(&text).expect("an answer");
    let mut found = Vec::new();
    subnormal_numbers(&answer, "", &mut found);
    assert!(found.is_empty(), "{tool}: {found:?}");
    answer
}

/// The number a `_hex` field holds, exactly.
fn exact(hex: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(hex.as_str().expect("hex"), 16).expect("16 hex digits"))
}

fn grazing(card: &str, h_eci_nms: [f64; 3], distance_au: f64, steps: u64) -> Value {
    json!({
        "domain": "aerospace", "card": card, "initial": { "h_eci_nms": h_eci_nms }, "dt_s": 0.05,
        "segments": [{ "steps": steps, "load": {
            "q1": 0.0, "q2": 0.0, "q3": 0.0, "q4": 1.0, "sun_x": 1.0, "sun_y": 0.0, "sun_z": -1.0e-300, "sun_distance_au": distance_au
        } }],
        "record_every": 1
    })
}

/// The red team's grazing Sun on the dihedral card, (1, 0, -1e-300) at 1, 100
/// and 10,000 au: every answer normal or zero (D1). At 100 au, from a
/// momentum of zero, the sealed momentum, sent back with `from`, is continued
/// (D2: it had been refused as "not a finite normal number"), and the
/// continuation verifies by request.
#[test]
fn the_dihedral_card_under_a_grazing_sun_answers_normal_or_zero_and_its_sealed_state_is_continued() {
    let mut door = Door::start();
    for distance_au in [1.0, 100.0, 1.0e4] {
        for start in [[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]] {
            let run = answered(&mut door, "run_trajectory", &grazing("momentum_jwst_dihedral", start, distance_au, 3));
            assert_eq!(run["outcome"], "completed", "{run}");
        }
    }
    let first = grazing("momentum_jwst_dihedral", [0.0; 3], 100.0, 1);
    let upstream = answered(&mut door, "run_trajectory", &first);
    let h: Vec<f64> = upstream["final_state"]["h_eci_nms_hex"].as_array().unwrap().iter().map(exact).collect();
    assert!(h.iter().all(|value| *value == 0.0), "{h:?}");
    let mut second = grazing("momentum_jwst_dihedral", [h[0], h[1], h[2]], 100.0, 1);
    second["from"] = json!({ "request": first, "run_seal": upstream["run_seal"] });
    let continued = answered(&mut door, "run_trajectory", &second);
    assert_eq!(continued["outcome"], "completed", "{continued}");
    assert_eq!(continued["chain"]["kind"], "continuation");
    let verified = answered(
        &mut door,
        "verify_tape",
        &json!({ "request": second, "run_seal": continued["run_seal"], "chain_seal": continued["chain"]["chain_seal"] }),
    );
    assert_eq!(verified["holds"], json!(true), "{verified}");
}

/// The red team's cultures on culture_trichoderma_atp: fed like the fitted
/// cultures (glucose at 312 h 5.96e-309 g/L as computed) and unfed to 312 h
/// (1.2e-322): each answered +0, with no subnormal number anywhere.
#[test]
fn the_atp_culture_answers_an_exhausted_glucose_as_zero() {
    let mut door = Door::start();
    let fed = json!({ "solver": "culture_trichoderma_atp", "parameters": {
        "biomass_g_per_l": 0.8, "glucose_g_per_l": 20.5, "galacturonate_g_per_l": 4.5, "yeast_extract_g_per_l": 1.0, "ph": 4.0,
        "feed_glucose_g_per_l": 52.1, "feed_galacturonate_g_per_l": 95.0,
        "segment_end_h": [24.0, 48.0, 72.0, 96.0, 120.0, 144.0, 168.0, 192.0, 216.0, 240.0, 261.0, 288.0, 312.0],
        "feed_ml": [0.0, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 6.075949, 5.316456, 0.0, 0.0],
        "water_ml": vec![0.0; 13], "sample_ml": vec![4.3; 13]
    } });
    let solution = answered(&mut door, "solve", &fed);
    assert_eq!(solution["outcome"], "converged");
    assert_eq!(solution["result"]["glucose_g_per_l_hex"][12], "0000000000000000");
    let unfed = json!({ "solver": "culture_trichoderma_atp", "parameters": {
        "biomass_g_per_l": 0.9, "glucose_g_per_l": 20.8, "galacturonate_g_per_l": 4.6, "yeast_extract_g_per_l": 2.0, "ph": 3.5,
        "feed_glucose_g_per_l": 32.0, "feed_galacturonate_g_per_l": 94.0,
        "segment_end_h": [312.0], "feed_ml": [0.0], "water_ml": [0.0], "sample_ml": [0.0]
    } });
    let solution = answered(&mut door, "solve", &unfed);
    assert_eq!(solution["result"]["glucose_g_per_l_hex"][0], "0000000000000000");
    let verified = answered(&mut door, "verify_solution", &json!({ "request": unfed, "solution_seal": solution["solution_seal"] }));
    assert_eq!(verified["holds"], json!(true), "{verified}");
}

/// The red team's tractions on the elastic plate: at 1e-154 MPa the strain
/// energy and the boundary work (2.06e-313 J as computed) are answered as
/// zero; at 1e-300 MPa and at the smallest normal traction no number of the
/// field is subnormal.
#[test]
fn the_elastic_plate_answers_a_value_under_the_normal_range_as_zero() {
    let mut door = Door::start();
    for traction in [1.0e-154, 1.0e-300, f64::MIN_POSITIVE] {
        let solution = answered(
            &mut door,
            "solve",
            &json!({ "solver": "stress_plate_hole_s235_elastic", "parameters": { "traction_mpa": traction, "points_x_mm": [0.0, 4.5], "points_y_mm": [10.025, 0.0] } }),
        );
        assert_eq!(solution["outcome"], "converged", "{solution}");
        if traction == 1.0e-154 {
            assert_eq!(solution["result"]["strain_energy_j_hex"], "0000000000000000");
            assert_eq!(solution["result"]["boundary_work_j_hex"], "0000000000000000");
        }
    }
}
