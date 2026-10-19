//! The card contract, attacked on grid_gb_twin by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::grid_gb_twin::*;

#[test]
fn grid_gb_twin_meets_the_card_contract() {
    let card = GridGbTwin::great_britain();
    let held = ResponseHeld { dc_low_mw: 1145.0, dm_low_mw: 500.0, dr_low_mw: 480.0, dc_high_mw: 1080.0, dm_high_mw: 510.0, dr_high_mw: 480.0, fast_delivery_s: 1.0, regulation_delivery_s: 10.0 };
    let found = findings(&Kit {
        model: "grid_gb_twin",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start: GridGbTwinState::try_start(&card, 49.948, 177_000.0, 26_774.0, held).expect("the start lands"),
        load: GridGbTwinLoad { loss_mw: 1007.0 },
        dt_s: card.gate_dt_s().unwrap(),
        steps: 16,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_OUTSIDE_COMPARED_TRIPS | FLAG_PAST_COMPARED_MINUTE | FLAG_BELOW_COMPARED_FREQUENCY,
        ledger: vec![field!(GridGbTwinState, kinetic_energy_anchor_mw_s), field!(GridGbTwinState, energy_lost_mw_s), field!(GridGbTwinState, energy_response_mw_s)],
        physical: vec![field!(GridGbTwinState, frequency_hz)],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
