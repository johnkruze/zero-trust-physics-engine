//! The solver contract, attacked on {{name}} by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the solver.

mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_solvers::{{name}}::*;

#[test]
fn {{name}}_meets_the_solver_contract() {
    let found = findings(&Kit {
        model: "{{name}}",
        numbers: vec![("left_temperature_k".into(), 300.0), ("right_temperature_k".into(), 350.0), ("heat_generation_w_per_m3".into(), 1.0e5)],
        solve: |n| {
            let question = {{Name}}Question { left_temperature_k: n[0], right_temperature_k: n[1], heat_generation_w_per_m3: n[2] };
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
