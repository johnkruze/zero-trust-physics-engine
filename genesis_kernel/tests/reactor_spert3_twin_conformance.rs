//! The card contract, attacked on reactor_spert3_twin by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::reactor_spert3_twin::*;

#[test]
fn reactor_spert3_twin_meets_the_card_contract() {
    let card = ReactorSpert3Twin::e_core();
    let start = ReactorState::try_start(&card, (76.0 + 459.67) / 1.8, 0.0853, 0.0, 1.17719e-5).expect("the start lands");
    let found = findings(&Kit {
        model: "reactor_spert3_twin",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start,
        load: ReactorLoad { reactivity_inserted_dollar: 1.0 },
        dt_s: card.gate_dt_s().unwrap(),
        steps: 16,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_ROD_MOVING | FLAG_PAST_PEAK | FLAG_HOT_SPOT_BOILING | FLAG_PROMPT_SUPERCRITICAL | FLAG_OUTSIDE_COMPARED,
        ledger: vec![
            field!(ReactorState, population_anchor_mw),
            field!(ReactorState, heat_anchor_mws),
            field!(ReactorState, hot_heat_anchor_mws),
            field!(ReactorState, reactivity_power_integral),
            field!(ReactorState, heat_carried_out_mws),
            field!(ReactorState, hot_heat_to_water_mws),
        ],
        physical: vec![
            field!(ReactorState, power_mw),
            field!(ReactorState, precursor_mw[3]),
            field!(ReactorState, fuel_k[0]),
            field!(ReactorState, fuel_k[5]),
            field!(ReactorState, clad_k),
            field!(ReactorState, hot_fuel_k[2]),
            field!(ReactorState, hot_clad_k),
            field!(ReactorState, water_k),
            field!(ReactorState, energy_released_mws),
        ],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
