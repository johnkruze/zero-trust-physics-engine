//! The card contract, attacked on generator_mmrtg_seebeck by the shared
//! harness (`tests/conformance_support`), from inside the record and from
//! past its last day, where the registered forecast runs and both flags are
//! set. Shaped from the kit's template; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::generator_mmrtg_seebeck::*;

fn attack(start_s: f64) -> Vec<String> {
    let card = GeneratorMmrtgSeebeck::curiosity();
    let found = findings(&Kit {
        model: "generator_mmrtg_seebeck",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start: GeneratorMmrtgSeebeckState::try_mission_time(&card, start_s).expect("the start lands"),
        load: GeneratorMmrtgSeebeckLoad {},
        dt_s: 86_400.0,
        steps: 16,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_PAST_FITTING_SPAN | FLAG_PAST_RECORD,
        ledger: vec![
            field!(GeneratorMmrtgSeebeckState, energy_anchor_j),
            field!(GeneratorMmrtgSeebeckState, electrical_energy_j),
            field!(GeneratorMmrtgSeebeckState, heat_rejected_j),
            field!(GeneratorMmrtgSeebeckState, converter_account_anchor_j),
        ],
        physical: vec![],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect()
}

#[test]
fn generator_mmrtg_seebeck_meets_the_card_contract() {
    // Mission day 100, inside the record, and day 4,100, past it.
    let broken: Vec<String> = [8.64e6, 4_100.0 * 86_400.0].into_iter().flat_map(attack).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
