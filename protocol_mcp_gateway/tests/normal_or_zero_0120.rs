//! Engine invariant 4 on the models engine 0.12.0 serves, through the local
//! connection: the red team of 0.13.0 found each of these publishing a
//! subnormal number in a completed answer, as served 0.12.0 does (D1), and
//! the SRP card's sealed momentum refused when continued (D2). Each value
//! under 2^-1022 is now written as zero with its sign where it is computed,
//! in the card's step and the solver's result. These answers move from
//! 0.12.0's, and only these: John Kruze decides whether the commit that
//! holds the 0.12.0 models to the rule is kept.

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

fn grazing(h_eci_nms: [f64; 3], distance_au: f64, steps: u64) -> Value {
    json!({
        "domain": "aerospace", "card": "momentum_jwst_srp", "initial": { "h_eci_nms": h_eci_nms }, "dt_s": 0.05,
        "segments": [{ "steps": steps, "load": {
            "q1": 0.0, "q2": 0.0, "q3": 0.0, "q4": 1.0, "sun_x": 1.0, "sun_y": 0.0, "sun_z": -1.0e-300, "sun_distance_au": distance_au
        } }],
        "record_every": 1
    })
}

/// The red team's grazing Sun on momentum_jwst_srp, as on the dihedral card:
/// every answer normal or zero, and the sealed momentum continued with `from`
/// and verified by request.
#[test]
fn the_srp_card_under_a_grazing_sun_answers_normal_or_zero_and_its_sealed_state_is_continued() {
    let mut door = Door::start();
    for distance_au in [1.0, 100.0, 1.0e4] {
        for start in [[0.0, 0.0, 0.0], [1.0, 2.0, 3.0]] {
            let run = answered(&mut door, "run_trajectory", &grazing(start, distance_au, 3));
            assert_eq!(run["outcome"], "completed", "{run}");
        }
    }
    let first = grazing([0.0; 3], 100.0, 1);
    let upstream = answered(&mut door, "run_trajectory", &first);
    let h: Vec<f64> = upstream["final_state"]["h_eci_nms_hex"].as_array().unwrap().iter().map(exact).collect();
    let mut second = grazing([h[0], h[1], h[2]], 100.0, 1);
    second["from"] = json!({ "request": first, "run_seal": upstream["run_seal"] });
    let continued = answered(&mut door, "run_trajectory", &second);
    assert_eq!(continued["outcome"], "completed", "{continued}");
    let verified = answered(
        &mut door,
        "verify_tape",
        &json!({ "request": second, "run_seal": continued["run_seal"], "chain_seal": continued["chain"]["chain_seal"] }),
    );
    assert_eq!(verified["holds"], json!(true), "{verified}");
}

/// The 2170 cell through the rest the m80 couplings required (removed in
/// 0.15.0, one model for each real object): 600 s at 4.5 A and 3600 s at
/// zero current at 40 degC. In 0.12.0 diffusion_lag_3 was 5e-324 here; the
/// served twin's answer is normal or zero.
#[test]
fn the_cells_rest_at_40_degc_answers_normal_or_zero() {
    let mut door = Door::start();
    let upstream = json!({
        "domain": "energy", "card": "cell_2170_model3",
        "initial": { "cell_number": 80.0, "state_of_charge": 1.0, "capacity_ah": 4.67683635335689, "case_temperature_least_c": 40.0, "case_temperature_most_c": 40.0 },
        "dt_s": 1.0,
        "segments": [{ "steps": 600, "load": { "current_a": 4.5, "case_temperature_c": 40.0 } }, { "steps": 3600, "load": { "current_a": 0.0, "case_temperature_c": 40.0 } }],
        "record_every": 4200
    });
    let rested = answered(&mut door, "run_trajectory", &upstream);
    assert_eq!(rested["outcome"], "completed", "{rested}");
}

/// The red team's traction on stress_plate_hole_s235 (2.06e-313 J of strain
/// energy at 1e-154 MPa as computed) and culture_trichoderma_ambr250's worked
/// start unfed to 312 h (glucose 5.4e-323 and galacturonate 2.4e-322 g/L as
/// computed): each answered as zero, with no subnormal number anywhere.
#[test]
fn the_plate_and_the_culture_answer_a_value_under_the_normal_range_as_zero() {
    let mut door = Door::start();
    for traction in [1.0e-154, 1.0e-300, f64::MIN_POSITIVE] {
        let solution = answered(
            &mut door,
            "solve",
            &json!({ "solver": "stress_plate_hole_s235", "parameters": { "traction_mpa": traction, "points_x_mm": [0.0, 4.5], "points_y_mm": [10.025, 0.0] } }),
        );
        assert_eq!(solution["outcome"], "converged", "{solution}");
        if traction == 1.0e-154 {
            assert_eq!(solution["result"]["strain_energy_j_hex"], "0000000000000000");
        }
    }
    let unfed = json!({ "solver": "culture_trichoderma_ambr250", "parameters": {
        "biomass_g_per_l": 0.9, "glucose_g_per_l": 20.8, "galacturonate_g_per_l": 4.6,
        "feed_glucose_g_per_l": 32.0, "feed_galacturonate_g_per_l": 94.0,
        "segment_end_h": [312.0], "feed_ml": [0.0], "water_ml": [0.0], "sample_ml": [0.0]
    } });
    let solution = answered(&mut door, "solve", &unfed);
    assert_eq!(solution["result"]["glucose_g_per_l_hex"][0], "0000000000000000");
    assert_eq!(solution["result"]["galacturonate_g_per_l_hex"][0], "0000000000000000");
}
