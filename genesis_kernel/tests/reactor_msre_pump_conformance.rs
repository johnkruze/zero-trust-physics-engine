//! The card contract, attacked on reactor_msre_pump by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::reactor_msre_pump::*;

#[test]
fn reactor_msre_pump_meets_the_card_contract() {
    let card = ReactorMsrePump::specimen();
    let found = findings(&Kit {
        model: "reactor_msre_pump",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start: ReactorMsrePumpState::try_start(&card, 0.0).expect("the start lands"),
        load: ReactorMsrePumpLoad { pump_speed_fraction: 0.05 },
        dt_s: card.gate_dt_s().unwrap(),
        steps: 16,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_FLOW_CHANGING | FLAG_FUEL_CIRCULATING | FLAG_OUTSIDE_COMPARED_RUNS,
        ledger: vec![
            field!(ReactorMsrePumpState, inventory_anchor_ws[1]),
            field!(ReactorMsrePumpState, born_ws[1]),
            field!(ReactorMsrePumpState, decayed_vessel_ws[3]),
            field!(ReactorMsrePumpState, decayed_outside_ws[3]),
        ],
        physical: vec![field!(ReactorMsrePumpState, precursor_ws[1][40]), field!(ReactorMsrePumpState, precursor_ws[3][200])],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
