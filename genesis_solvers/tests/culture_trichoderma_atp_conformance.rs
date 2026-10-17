//! The solver contract, attacked on culture_trichoderma_atp by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the solver.

mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_solvers::culture_trichoderma_atp::*;

/// The worked question: a three-day record of the specimen at pH 3.5.
fn kit() -> Kit {
    let mut numbers: Vec<(String, f64)> = vec![
        ("biomass_g_per_l".into(), 0.9),
        ("glucose_g_per_l".into(), 20.8),
        ("galacturonate_g_per_l".into(), 4.6),
        ("yeast_extract_g_per_l".into(), 2.0),
        ("ph".into(), 3.5),
        ("feed_glucose_g_per_l".into(), 32.0),
        ("feed_galacturonate_g_per_l".into(), 94.0),
    ];
    for (name, values) in [
        ("segment_end_h", [22.0, 44.0, 68.0]),
        ("feed_ml", [0.0, 4.4, 7.2]),
        ("water_ml", [-2.0, -1.0, 0.5]),
        ("sample_ml", [4.3, 4.3, 4.3]),
    ] {
        numbers.extend(values.iter().enumerate().map(|(index, value)| (format!("{name}[{index}]"), *value)));
    }
    Kit {
        model: "culture_trichoderma_atp",
        numbers,
        solve: |n| {
            solve(&Question {
                biomass_g_per_l: n[0],
                glucose_g_per_l: n[1],
                galacturonate_g_per_l: n[2],
                yeast_extract_g_per_l: n[3],
                ph: n[4],
                feed_glucose_g_per_l: n[5],
                feed_galacturonate_g_per_l: n[6],
                segment_end_h: &n[7..10],
                feed_ml: &n[10..13],
                water_ml: &n[13..16],
                sample_ml: &n[16..19],
            })
            .map(|culture| format!("{culture:?}"))
            .map_err(|reject| format!("{reject:?}"))
        },
        every_number: true,
    }
}

#[test]
fn culture_trichoderma_atp_meets_the_solver_contract() {
    let found = findings(&kit());
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
