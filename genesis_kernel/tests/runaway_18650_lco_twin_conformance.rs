//! The card contract, attacked on runaway_18650_lco_twin by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::runaway_18650_lco_twin::*;

#[test]
fn runaway_18650_lco_twin_meets_the_card_contract() {
    let card = Runaway18650LcoTwin::fitted();
    let found = findings(&Kit {
        model: "runaway_18650_lco_twin",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start: RunawayState::try_fresh(&card, 301.15).expect("the start lands"),
        load: RunawayLoad { oven_temperature_k: 428.15, heater_power_w: 0.0, adiabatic: 0.0 },
        dt_s: 1.0,
        steps: 16,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_SELF_HEATING | FLAG_RUNAWAY | FLAG_SEPARATOR_MELTED | FLAG_VENTED | FLAG_OUTSIDE_COMPARED,
        ledger: vec![
            field!(RunawayState, heater_heat_j),
            field!(RunawayState, heat_lost_j),
            field!(RunawayState, start_temperature_k),
        ],
        physical: vec![field!(RunawayState, temperature_k)],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
