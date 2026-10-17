//! The solver contract, attacked on blast_wave_tnt_air by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the solver.
//! The burst is a word, not a number: the worked question is attacked once
//! in free air and once on the ground.

mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_solvers::blast_wave_tnt_air::*;

fn debug(returned: Result<BlastWave, BlastReject>) -> Result<String, String> {
    returned.map(|wave| format!("{wave:?}")).map_err(|reject| format!("{reject:?}"))
}

fn assert_conforms(kit: Kit) {
    let found = findings(&kit);
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}

#[test]
fn blast_wave_tnt_air_meets_the_solver_contract_in_free_air() {
    assert_conforms(Kit {
        model: "blast_wave_tnt_air",
        numbers: vec![("charge_mass_kg".into(), 1.0), ("distance_m".into(), 5.0)],
        solve: |n| debug(solve(&BlastQuestion { charge_mass_kg: n[0], distance_m: n[1], burst: Burst::FreeAir })),
        every_number: true,
    });
}

#[test]
fn blast_wave_tnt_air_meets_the_solver_contract_on_the_ground() {
    assert_conforms(Kit {
        model: "blast_wave_tnt_air",
        numbers: vec![("charge_mass_kg".into(), 100.0), ("distance_m".into(), 30.0)],
        solve: |n| debug(solve(&BlastQuestion { charge_mass_kg: n[0], distance_m: n[1], burst: Burst::Surface })),
        every_number: true,
    });
}
