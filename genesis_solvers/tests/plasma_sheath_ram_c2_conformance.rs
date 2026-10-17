//! The solver contract, attacked on plasma_sheath_ram_c2 by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the solver.

mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_solvers::plasma_sheath_ram_c2::*;

#[test]
fn plasma_sheath_ram_c2_meets_the_solver_contract() {
    let found = findings(&Kit {
        model: "plasma_sheath_ram_c2",
        numbers: vec![("altitude_m".into(), 71_000.0), ("velocity_m_s".into(), 7650.0), ("signal_frequency_hz".into(), 9.21e9)],
        solve: |n| {
            let question = PlasmaSheathRamC2Question { altitude_m: n[0], velocity_m_s: n[1], signal_frequency_hz: n[2] };
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
