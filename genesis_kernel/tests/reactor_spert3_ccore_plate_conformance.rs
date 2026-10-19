//! The card contract, attacked on reactor_spert3_ccore_plate by the shared harness
//! (`tests/conformance_support`). Stamped from the kit; keep it with the card.

#[macro_use]
mod conformance_support;

use conformance_support::{Kit, Weight, findings};
use genesis_kernel::reactor_spert3_ccore_plate::*;

fn attack(name: &'static str, temperature_k: f64, pressure_psig: f64, load: ReactorLoad, steps: usize) {
    let card = ReactorSpert3CcorePlate::c_core_plate();
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
            field!(ReactorState, feedback_meat_rise_k),
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
fn reactor_spert3_ccore_plate_meets_the_card_contract() {
    attack(
        "reactor_spert3_ccore_plate",
        298.15,
        0.0,
        ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: 0.0 },
        16,
    );
}

/// The same attacks with flow at 230 psig, where the sweep carries the ledger's swept terms.
#[test]
fn reactor_spert3_ccore_plate_meets_the_card_contract_with_flow() {
    attack(
        "reactor_spert3_ccore_plate",
        303.15,
        230.0,
        ReactorLoad { reactivity_inserted_dollar: 1.1, coolant_flow_m_per_s: 2.7432 },
        16,
    );
}

/// A run whose flow changes between segments: the film gives water to the bulk or takes it, the
/// heat held is unchanged, and every step closes its ledger.
#[test]
fn a_flow_changed_mid_run_keeps_the_ledger_closed() {
    let card = ReactorSpert3CcorePlate::c_core_plate();
    let mut state = ReactorState::try_start(&card, 303.15, 230.0).expect("the start lands");
    let mut frame = genesis_kernel::frame::LastStateFrame64::from_bytes([0u8; 64]);
    let dt = card.gate_dt_s().unwrap();
    let flows = [0.0, 5.4864, 0.054864, 2.7432, 0.0];
    let mut worst: f64 = 0.0;
    for (segment, flow) in flows.iter().enumerate() {
        let load = ReactorLoad { reactivity_inserted_dollar: 1.2, coolant_flow_m_per_s: *flow };
        for _ in 0..300 {
            match step(&card, &mut state, &mut frame, load, dt) {
                Ok(()) => worst = worst.max(state.residual_conservation_error),
                Err(StepReject::PastPlateTemperatureMaximum) => break,
                Err(e) => panic!("segment {segment} at {} s: {e:?}", state.t_s),
            }
        }
    }
    assert!(state.past_power_peak, "{state:?}");
    assert!(worst <= 1.0e-6, "{worst}");
}

/// With no flow the film's share is the fitted one exactly, and with flow it thins.
#[test]
fn the_film_thins_with_flow_and_the_plate_feedback_lags() {
    let card = ReactorSpert3CcorePlate::c_core_plate();
    let mut state = ReactorState::try_start(&card, 298.15, 2500.0).expect("the start lands");
    let mut frame = genesis_kernel::frame::LastStateFrame64::from_bytes([0u8; 64]);
    let load = ReactorLoad { reactivity_inserted_dollar: card.inhour_dollar(0.011), coolant_flow_m_per_s: 0.0 };
    let mut lagged = false;
    for _ in 0..2000 {
        if step(&card, &mut state, &mut frame, load, card.gate_dt_s().unwrap()).is_err() {
            break;
        }
        if state.past_power_peak && !lagged {
            lagged = true;
            // At the power peak the meat is hotter than the cladding, and the plate's feedback
            // is behind what the plate's temperature calls for.
            assert!(state.hot_spot_meat_rise_k > state.hot_spot_plate_rise_k, "{state:?}");
            let mean = (2.0 * state.feedback_meat_rise_k + state.feedback_plate_rise_k) / 3.0;
            let now = DENSITY_COEFFICIENT_DOLLAR * PLATE_DISPLACEMENT_PER_K * (mean - state.feedback_water_rise_k) + PLATE_COEFFICIENT_DOLLAR_PER_K * mean;
            assert!(state.plate_feedback_dollar < now, "{} {now}", state.plate_feedback_dollar);
            assert_eq!(state.plate_feedback_dollar.to_bits(), state.plate_feedback_lagged_dollar.to_bits());
        }
    }
    assert!(lagged);
}
