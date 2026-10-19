//! The reactor's bounds are refused before a physics permit is taken.
//!
//! The single permit of the pool is held for the whole test. A request that
//! is refused for its own fault answers 422 by name. A request that is valid
//! would need the permit and answers 429. So a 422 under a held permit shows
//! that the refusal came first.
//!
//! 0.15.0: `reactor_spert3` is the SPERT III E-core twin, so the probe is
//! written on its own fields and its worked example; the point-kinetics card
//! and the fuel-temperature card (`reactor_spert3_fuel`, retired) are gone.

use genesis_kernel::reactor_spert3_twin;
use serde_json::{Value, json};

use super::{AppState, DoorError, acquire_physics_permit, compute_step, trajectory};

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

/// The worked example's start: cold startup at the Idaho site's atmosphere.
const INLET_K: f64 = 297.5944444444445;
const PRESSURE_MPA: f64 = 0.0853;
const POWER_MW: f64 = 7.9949547e-06;

#[derive(Clone, Copy)]
struct Start {
    inlet_k: f64,
    pressure_mpa: f64,
    flow_m_per_s: f64,
    power_mw: f64,
}

const COLD: Start = Start { inlet_k: INLET_K, pressure_mpa: PRESSURE_MPA, flow_m_per_s: 0.0, power_mw: POWER_MW };

fn step(start: Start, reactivity: f64, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": "reactor_spert3",
        "parameters": {
            "inlet_temperature_k": start.inlet_k,
            "system_pressure_mpa": start.pressure_mpa,
            "coolant_flow_m_per_s": start.flow_m_per_s,
            "initial_power_mw": start.power_mw,
            "reactivity_inserted_dollar": reactivity,
            "dt_s": dt_s
        }
    })
}

fn run(start: Start, reactivities: &[f64], dt_s: f64) -> Value {
    let segments: Vec<Value> = reactivities
        .iter()
        .map(|rho| json!({ "steps": 3, "load": { "reactivity_inserted_dollar": rho } }))
        .collect();
    json!({
        "domain": "energy",
        "card": "reactor_spert3",
        "initial": {
            "inlet_temperature_k": start.inlet_k,
            "system_pressure_mpa": start.pressure_mpa,
            "coolant_flow_m_per_s": start.flow_m_per_s,
            "initial_power_mw": start.power_mw
        },
        "dt_s": dt_s,
        "segments": segments,
        "record_every": 1
    })
}

fn message(error: DoorError) -> String {
    match error {
        DoorError::Unprocessable(message) => message,
        other => panic!("not a 422: {}", other.message()),
    }
}

#[tokio::test]
async fn each_bound_is_refused_by_name_before_a_permit_at_the_last_number_on_each_side() {
    let reactor = reactor_spert3_twin::ReactorSpert3Twin::e_core();
    let dt = reactor.gate_dt_s().unwrap();
    let floor = reactor.floor_dt_s().unwrap();
    let (p_min, p_max) = (reactor_spert3_twin::PRESSURE_MIN_MPA, reactor_spert3_twin::PRESSURE_MAX_MPA);
    let state = AppState::new(1);
    let held = acquire_physics_permit(&state).expect("the one permit");

    // Inside the bounds the request needs the permit, so the held pool says 429: the
    // worked example, the last number inside each open end, no flow, no reactivity,
    // and the floor and the ceiling of the clock.
    for inside in [
        step(COLD, 1.0, dt),
        step(Start { inlet_k: above(273.15), ..COLD }, 1.0, dt),
        step(Start { pressure_mpa: above(p_min), inlet_k: above(273.15), ..COLD }, 1.0, dt),
        step(Start { pressure_mpa: below(p_max), ..COLD }, 1.0, dt),
        step(Start { flow_m_per_s: 4.27, ..COLD }, 1.0, dt),
        step(Start { power_mw: f64::MIN_POSITIVE, ..COLD }, 1.0, dt),
        step(COLD, 0.0, dt),
        step(COLD, 1.0, floor),
    ] {
        match compute_step(&state, inside.clone()).await {
            Err(DoorError::TooMany(_)) => {}
            Err(other) => panic!("{inside}: {}", other.message()),
            Ok(_) => panic!("{inside}: sealed with the pool held"),
        }
    }
    // A run of several segments under one reactivity needs the permit.
    match trajectory::run(&state, run(COLD, &[1.0, 1.0, 1.0], dt)).await {
        Err(DoorError::TooMany(_)) => {}
        Err(other) => panic!("a run: {}", other.message()),
        Ok(_) => panic!("a run sealed with the pool held"),
    }

    // Past each end, the step and the run are refused by name, with the same sentence: the
    // field, the value, the bound and the refusal.
    let past = |field: &str, value: f64, unit: &str, side: &str, bound: f64, kind: &str, name: &str| {
        format!(
            "Physical Failure: '{field}' {} {unit} is {side} {} {unit}, the {kind} of the reactor_spert3 card ({name}). That state is not sealed.",
            crate::message_number(value),
            crate::message_number(bound),
        )
    };
    let tiny = -f64::MIN_POSITIVE;
    for (start, rho, expected) in [
        (Start { inlet_k: 273.15, ..COLD }, 1.0, past("inlet_temperature_k", 273.15, "K", "at or below", 273.15, "exclusive minimum", "StateOutsideCard")),
        (Start { pressure_mpa: p_min, ..COLD }, 1.0, past("system_pressure_mpa", p_min, "MPa", "at or below", p_min, "exclusive minimum", "StateOutsideCard")),
        (Start { pressure_mpa: p_max, ..COLD }, 1.0, past("system_pressure_mpa", p_max, "MPa", "at or above", p_max, "exclusive maximum", "StateOutsideCard")),
        (Start { flow_m_per_s: tiny, ..COLD }, 1.0, past("coolant_flow_m_per_s", tiny, "m/s", "below", 0.0, "inclusive minimum", "StateOutsideCard")),
        (Start { power_mw: 0.0, ..COLD }, 1.0, past("initial_power_mw", 0.0, "MW", "at or below", 0.0, "exclusive minimum", "StateOutsideCard")),
        (COLD, tiny, past("reactivity_inserted_dollar", tiny, "dollar", "below", 0.0, "inclusive minimum", "ReactivityNegative")),
    ] {
        assert_eq!(message(compute_step(&state, step(start, rho, dt)).await.unwrap_err()), expected);
        assert_eq!(
            message(trajectory::run(&state, run(start, &[rho], dt)).await.err().expect("refused")),
            expected
        );
    }

    // The clock: the number under the floor and the number over the ceiling.
    let under = message(compute_step(&state, step(COLD, 1.0, below(floor))).await.unwrap_err());
    assert_eq!(
        under,
        format!(
            "Physical Violation: 'dt_s' {} is below {} s, the step floor of the reactor_spert3 card (DtBelowFloor).",
            crate::message_number(below(floor)),
            crate::message_number(floor)
        )
    );
    let over = message(compute_step(&state, step(COLD, 1.0, above(dt))).await.unwrap_err());
    assert_eq!(
        over,
        format!(
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the reactor_spert3 card (DtAboveCeiling).",
            crate::message_number(above(dt)),
            crate::message_number(dt)
        )
    );
    drop(held);
}
