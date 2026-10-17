//! The card contract, attacked on {{name}} by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::{{name}}::*;

#[test]
fn {{name}}_meets_the_card_contract() {
    let card = {{Name}}::specimen();
    let found = findings(&Kit {
        model: "{{name}}",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start: {{Name}}State::try_start(&card, 300.0).expect("the start lands"),
        load: {{Name}}Load { heat_power_w: 50.0, ambient_k: REFERENCE_TEMPERATURE_K },
        dt_s: card.gate_dt_s().unwrap(),
        steps: 16,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: 0,
        ledger: vec![field!({{Name}}State, energy_anchor_j), field!({{Name}}State, boundary_work_j)],
        physical: vec![field!({{Name}}State, temperature_k)],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
