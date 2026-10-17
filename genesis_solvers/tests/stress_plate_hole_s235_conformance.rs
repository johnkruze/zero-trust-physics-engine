//! The solver contract, attacked on stress_plate_hole_s235 by the shared
//! harness (`tests/conformance_support`). Stamped from the kit; keep it with
//! the solver. The question is the measured traction and two points: the
//! top of the hole's edge and a point of the far field.

mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_solvers::stress_plate_hole_s235::*;

#[test]
fn stress_plate_hole_s235_meets_the_solver_contract() {
    let found = findings(&Kit {
        model: "stress_plate_hole_s235",
        numbers: vec![
            ("traction_mpa".into(), MEASURED_TRACTION_MPA),
            ("points_x_mm[0]".into(), 0.0),
            ("points_x_mm[1]".into(), -20.0),
            ("points_y_mm[0]".into(), 4.0),
            ("points_y_mm[1]".into(), 5.0),
        ],
        solve: |n| {
            let question = PlateQuestion { traction_mpa: n[0], points_x_mm: vec![n[1], n[2]], points_y_mm: vec![n[3], n[4]] };
            solve(&question).map(|field| format!("{field:?}")).map_err(|reject| format!("{reject:?}"))
        },
        every_number: true,
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
