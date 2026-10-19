//! The card contract, attacked on reactor_spert3_ccore_twin by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::reactor_spert3_ccore_twin::*;

fn attack(name: &'static str, temperature_k: f64, pressure_psig: f64, load: ReactorLoad, steps: usize) {
    let card = ReactorSpert3CcoreTwin::c_core_twin();
    let found = findings(&Kit {
        model: name,
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start: ReactorState::try_start(&card, temperature_k, pressure_psig).expect("the start lands"),
        load,
        dt_s: card.gate_dt_s().unwrap(),
        steps,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_PROMPT_SUPERCRITICAL | FLAG_PAST_POWER_PEAK | FLAG_HOT_SPOT_BOILING | FLAG_OUTSIDE_COMPARED_RANGE,
        ledger: vec![
            field!(ReactorState, population_anchor_mw),
            field!(ReactorState, feedback_integral_dollar_mws),
            field!(ReactorState, swept_energy_mws),
            field!(ReactorState, hot_spot_swept_energy_mws),
        ],
        physical: vec![
            field!(ReactorState, power_mw),
            field!(ReactorState, plate_rise_k),
            field!(ReactorState, feedback_film_rise_k),
            field!(ReactorState, hot_spot_plate_rise_k),
        ],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}

#[test]
fn reactor_spert3_ccore_twin_meets_the_card_contract() {
    attack(
        "reactor_spert3_ccore_twin",
        298.15,
        0.0,
        ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: 0.0 },
        16,
    );
}

/// The same attacks with flow at 230 psig, where the sweep carries the ledger's swept terms.
#[test]
fn reactor_spert3_ccore_twin_meets_the_card_contract_with_flow() {
    attack(
        "reactor_spert3_ccore_twin",
        303.15,
        230.0,
        ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: 2.7432 },
        16,
    );
}
