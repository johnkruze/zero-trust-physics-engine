//! The card contract, attacked on momentum_jwst_dihedral by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::momentum_jwst_dihedral::*;

#[test]
fn momentum_jwst_dihedral_meets_the_card_contract() {
    let card = MomentumJwstDihedral::observatory();
    let found = findings(&Kit {
        model: "momentum_jwst_dihedral",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        // The observatory on 2023-05-13 at 00:00:30 UTC: its telemetered
        // momentum and attitude, and the Sun from JPL Horizons, the worked
        // start of momentum_jwst_srp.
        start: MomentumJwstDihedralState::try_start(&card, [-1.4853731309945153, -0.6656560404607257, 1.1653501123098389]).expect("the start lands"),
        load: MomentumJwstDihedralLoad {
            q1: 0.7216001749,
            q2: 0.0366541296,
            q3: -0.5597470999,
            q4: 0.4057497233,
            sun_x: 0.6160922205743601,
            sun_y: 0.7226624561625701,
            sun_z: 0.31335179942172875,
            sun_distance_au: 1.0193963859269388,
        },
        dt_s: 60.0,
        steps: 16,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_OUTSIDE_FIELD_OF_REGARD,
        ledger: vec![
            field!(MomentumJwstDihedralState, h_anchor_eci_nms[0]),
            field!(MomentumJwstDihedralState, h_anchor_eci_nms[1]),
            field!(MomentumJwstDihedralState, h_anchor_eci_nms[2]),
            field!(MomentumJwstDihedralState, impulse_eci_nms[0]),
            field!(MomentumJwstDihedralState, impulse_eci_nms[1]),
            field!(MomentumJwstDihedralState, impulse_eci_nms[2]),
        ],
        physical: vec![
            field!(MomentumJwstDihedralState, h_eci_nms[0]),
            field!(MomentumJwstDihedralState, h_eci_nms[1]),
            field!(MomentumJwstDihedralState, h_eci_nms[2]),
        ],
        departures: vec![],
    });
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}
