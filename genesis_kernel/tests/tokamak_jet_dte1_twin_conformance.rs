//! The card contract, attacked on tokamak_jet_dte1_twin by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::tokamak_jet_dte1_twin::*;

/// The record pulse 42982 near its peak: 10.5 MW of deuterium and 9.5 MW of
/// tritium beams, 2.4 MW of ion-cyclotron power, 6.4e19 m^-3 at 3.8 MA and 3.9 T.
fn record_load() -> TwinLoad {
    TwinLoad {
        deuterium_beam_power_w: 10.5e6,
        tritium_beam_power_w: 9.5e6,
        deuterium_beam_energy_kev: 80.0,
        tritium_beam_energy_kev: 150.0,
        icrh_power_w: 2.4e6,
        ohmic_power_w: 0.5e6,
        line_density_per_m3: 6.4e19,
        plasma_current_a: 3.8e6,
        toroidal_field_t: 3.9,
        tritium_fraction: 0.451,
        electron_temperature_kev: 7.4,
        effective_charge: 2.4,
        hot_ion_regime: 0.0,
    }
}

#[test]
fn tokamak_jet_dte1_twin_meets_the_card_contract() {
    let card = TokamakJetDte1Twin::dte1();
    let found = findings(&Kit {
        model: "tokamak_jet_dte1_twin",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start: TwinState::try_start(&card, 9.0e6, 0.5e6, 0.7e6, 0.4e6).expect("the start lands"),
        load: record_load(),
        dt_s: card.gate_dt_s().unwrap(),
        steps: 16,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_NEAR_DENSITY_LIMIT | FLAG_OUTSIDE_COMPARED,
        ledger: vec![
            field!(TwinState, energy_anchor_j),
            field!(TwinState, energy_supplied_j),
            field!(TwinState, energy_lost_j),
        ],
        physical: vec![
            field!(TwinState, thermal_energy_j),
            field!(TwinState, deuterium_beam_ion_energy_j),
            field!(TwinState, tritium_beam_ion_energy_j),
            field!(TwinState, tail_ion_energy_j),
        ],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
