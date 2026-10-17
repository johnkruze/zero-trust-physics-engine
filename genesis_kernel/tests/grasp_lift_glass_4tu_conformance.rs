//! The card contract, attacked on grasp_lift_glass_4tu by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.
//! Two runs: a lift off the table at a grip that holds, and a hold in the air
//! at a grip under which the pads slide, so the attacks meet the object
//! resting, lifted and sliding.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::grasp_lift_glass_4tu::*;

fn kit(start: GraspLiftGlass4tuState, load: GraspLiftGlass4tuLoad, steps: usize) -> Vec<conformance_support::Finding> {
    let card = GraspLiftGlass4tu::specimen();
    findings(&Kit {
        model: "grasp_lift_glass_4tu",
        card,
        is_valid: |card| card.is_valid(),
        step,
        gate_dt_s: card.gate_dt_s(),
        floor_dt_s: card.floor_dt_s(),
        start,
        load,
        dt_s: card.gate_dt_s().unwrap(),
        steps,
        residual: |state| state.residual_conservation_error,
        limit: RESIDUAL_CONSERVATION_LIMIT,
        flags: FLAG_INCIPIENT_SLIP | FLAG_GROSS_SLIP | FLAG_ON_TABLE,
        ledger: vec![
            field!(GraspLiftGlass4tuState, energy_anchor_j),
            field!(GraspLiftGlass4tuState, hand_work_in_j),
            field!(GraspLiftGlass4tuState, hand_work_out_j),
            field!(GraspLiftGlass4tuState, friction_heat_j),
            field!(GraspLiftGlass4tuState, momentum_anchor_n_s),
            field!(GraspLiftGlass4tuState, contact_impulse_n_s),
            field!(GraspLiftGlass4tuState, gravity_impulse_n_s),
            field!(GraspLiftGlass4tuState, table_impulse_n_s),
        ],
        physical: vec![
            field!(GraspLiftGlass4tuState, object_height_m),
            field!(GraspLiftGlass4tuState, object_velocity_m_s),
            field!(GraspLiftGlass4tuState, element_force_n[0]),
            field!(GraspLiftGlass4tuState, element_force_n[31]),
        ],
        departures: vec![],
    })
}

fn report(found: Vec<conformance_support::Finding>) {
    for finding in &found {
        println!("{}", finding.line());
    }
    let broken: Vec<String> = found.iter().filter(|f| f.weight == Weight::Contract).map(|f| f.line()).collect();
    assert!(broken.is_empty(), "{broken:#?}");
}

#[test]
fn grasp_lift_glass_4tu_meets_the_card_contract_on_a_lift() {
    let card = GraspLiftGlass4tu::specimen();
    let start = GraspLiftGlass4tuState::try_start(&card, 0.0).expect("the start lands");
    report(kit(start, GraspLiftGlass4tuLoad { grip_force_n: 2.0, hand_acceleration_m_s2: 2.0 }, 400));
}

#[test]
fn grasp_lift_glass_4tu_meets_the_card_contract_lifted_and_sliding() {
    let card = GraspLiftGlass4tu::specimen();
    let dt = card.gate_dt_s().unwrap();
    // Lift at an ample grip until the object is well clear of the table.
    let mut state = GraspLiftGlass4tuState::try_start(&card, 0.0).unwrap();
    let mut frame = genesis_kernel::frame::LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [0; 16] };
    for _ in 0..600 {
        step(&card, &mut state, &mut frame, GraspLiftGlass4tuLoad { grip_force_n: 3.0, hand_acceleration_m_s2: 1.0 }, dt).unwrap();
    }
    assert!(state.object_height_m > 0.01 && !state.on_table);
    // Then hold the hand still at a grip whose friction is under the weight.
    report(kit(state, GraspLiftGlass4tuLoad { grip_force_n: 0.5, hand_acceleration_m_s2: -20.0 }, 40));
}
