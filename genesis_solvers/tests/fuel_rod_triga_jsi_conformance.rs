//! The solver contract, attacked on fuel_rod_triga_jsi by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the solver.
//!
//! The kit's question is a pulse written from four numbers: the start
//! temperature, the core energy released, the time the release starts and
//! its width, on a history of 10 ms points to 3 s.

mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_solvers::fuel_rod_triga_jsi::*;

fn question(n: &[f64]) -> ElementQuestion {
    let mut time_s = Vec::with_capacity(300);
    let mut core_energy_released_mws = Vec::with_capacity(300);
    // A start or a width that is not finite reaches the energies as NaN, as
    // a history written from it would.
    let poison = n[2] * 0.0 + n[3] * 0.0;
    for k in 1..=300 {
        let t = k as f64 * 0.01;
        time_s.push(t);
        let share = (t - n[2]) / n[3];
        let share = if share < 0.0 { 0.0 } else if share > 1.0 { 1.0 } else { share };
        core_energy_released_mws.push(n[1] * if share.is_nan() { f64::NAN } else { share } + poison);
    }
    ElementQuestion { start_temperature_k: n[0], time_s, core_energy_released_mws }
}

#[test]
fn fuel_rod_triga_jsi_meets_the_solver_contract() {
    let found = findings(&Kit {
        model: "fuel_rod_triga_jsi",
        numbers: vec![
            ("start_temperature_k".into(), 297.0),
            ("core_energy_released_mws".into(), 12.0),
            ("release_start_s".into(), 0.3),
            ("release_width_s".into(), 0.02),
        ],
        solve: |n| solve(&question(n)).map(|answer| format!("{answer:?}")).map_err(|reject| format!("{reject:?}")),
        every_number: true,
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
