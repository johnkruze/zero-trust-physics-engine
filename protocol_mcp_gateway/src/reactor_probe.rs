//! The reactor's bounds are refused before a physics permit is taken.
//!
//! The single permit of the pool is held for the whole test. A request that
//! is refused for its own fault answers 422 by name. A request that is valid
//! would need the permit and answers 429. So a 422 under a held permit shows
//! that the refusal came first.

use genesis_kernel::reactor_spert3;
use serde_json::{Value, json};

use super::{AppState, DoorError, acquire_physics_permit, compute_step, trajectory};

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

fn step(coolant_k: f64, reactivity: f64, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": "reactor_spert3",
        "parameters": {
            "coolant_temperature_k": coolant_k,
            "reactivity_inserted_dollar": reactivity,
            "dt_s": dt_s
        }
    })
}

fn run(coolant_k: f64, reactivities: &[f64], dt_s: f64) -> Value {
    let segments: Vec<Value> = reactivities
        .iter()
        .map(|rho| json!({ "steps": 3, "load": { "reactivity_inserted_dollar": rho } }))
        .collect();
    json!({
        "domain": "energy",
        "card": "reactor_spert3",
        "initial": { "coolant_temperature_k": coolant_k },
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
    let reactor = reactor_spert3::ReactorSpert3::e_core_cold_startup();
    let dt = reactor.gate_dt_s().unwrap();
    let floor = reactor.floor_dt_s().unwrap();
    let (water_min, water_max) = (reactor_spert3::WATER_FREEZING_K, reactor_spert3::WATER_CRITICAL_K);
    let (cold_min, cold_max) = (
        reactor_spert3::COOLANT_TEMPERATURE_MIN_K,
        reactor_spert3::COOLANT_TEMPERATURE_MAX_K,
    );
    let (rho_min, rho_max) = (
        reactor_spert3::REACTIVITY_MIN_DOLLAR,
        reactor_spert3::REACTIVITY_MAX_DOLLAR,
    );
    let state = AppState::new(1);
    let held = acquire_physics_permit(&state).expect("the one permit");

    // Inside the bounds the request needs the permit, so the held pool says 429: the
    // tested span, the old walls past it, and liquid water to its two ends.
    for inside in [
        step(cold_min, 1.0, dt),
        step(cold_max, 1.0, dt),
        step(cold_min, rho_min, dt),
        step(cold_max, rho_max, dt),
        step(cold_min, 1.0, floor),
        step(water_min, 1.0, dt),
        step(water_max, 1.0, dt),
        step(cold_min, below(rho_min), dt),
        step(cold_min, above(rho_max), dt),
        step(cold_min, f64::MIN_POSITIVE, dt),
        step(cold_min, 50.0, dt),
    ] {
        match compute_step(&state, inside.clone()).await {
            Err(DoorError::TooMany(_)) => {}
            Err(other) => panic!("{inside}: {}", other.message()),
            Ok(_) => panic!("{inside}: sealed with the pool held"),
        }
    }
    // A run of several segments, under one reactivity or each under its own, a later one at
    // zero or below included: it needs the permit.
    for reactivities in [
        [rho_min, rho_min, rho_min],
        [rho_max, rho_max, rho_max],
        [rho_min, rho_max, 2.0],
        [1.0, above(1.0), below(1.0)],
        [1.1, 0.0, -3.0],
    ] {
        match trajectory::run(&state, run(cold_min, &reactivities, dt)).await {
            Err(DoorError::TooMany(_)) => {}
            Err(other) => panic!("{reactivities:?}: {}", other.message()),
            Ok(_) => panic!("a run sealed with the pool held"),
        }
    }

    // The sentence names the field, the value, the bound and the refusal.
    let cool = |value: f64, bound: f64, upper: bool| {
        format!(
            "Physical Failure: 'coolant_temperature_k' {} K is {} {} K, the inclusive {} of the reactor_spert3 card (CoolantTemperatureNotLiquidWater). That state is not sealed.",
            crate::message_number(value),
            if upper { "above" } else { "below" },
            crate::message_number(bound),
            if upper { "maximum" } else { "minimum" },
        )
    };
    for (request, run_request, expected) in [
        (step(below(water_min), 1.0, dt), run(below(water_min), &[1.0], dt), cool(below(water_min), water_min, false)),
        (step(above(water_max), 1.0, dt), run(above(water_max), &[1.0], dt), cool(above(water_max), water_max, true)),
    ] {
        assert_eq!(message(compute_step(&state, request).await.unwrap_err()), expected);
        assert_eq!(
            message(trajectory::run(&state, run_request).await.err().expect("refused")),
            expected
        );
    }
    // The step in reactivity of the first segment must insert some: refused by name before a
    // permit, on a step and on a run, at zero, at its other sign and below.
    for rho in [0.0, -0.0, -f64::MIN_POSITIVE, -1.0] {
        let expected = "Physical Failure: the reactivity of the first step is zero or less: no step in reactivity is inserted, and there is no burst (ReactivityNotPositive). That state is not sealed.";
        assert_eq!(message(compute_step(&state, step(cold_min, rho, dt)).await.unwrap_err()), expected, "{rho}");
        assert_eq!(
            message(trajectory::run(&state, run(cold_min, &[rho, 1.0], dt)).await.err().expect("refused")),
            expected,
            "{rho}"
        );
    }

    // The clock: the number under the floor and the number over the ceiling.
    let under = message(compute_step(&state, step(cold_min, 1.0, below(floor))).await.unwrap_err());
    assert_eq!(
        under,
        format!(
            // Re-pinned for 0.10.1: a step below the floor carries the name DtBelowFloor.
            "Physical Violation: 'dt_s' {} is below {} s, the step floor of the reactor_spert3 card (DtBelowFloor).",
            crate::message_number(below(floor)),
            crate::message_number(floor)
        )
    );
    let over = message(compute_step(&state, step(cold_min, 1.0, above(dt))).await.unwrap_err());
    assert_eq!(
        over,
        format!(
            // Re-pinned for 0.10.1: a step above the ceiling carries the name DtAboveCeiling.
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the reactor_spert3 card (DtAboveCeiling).",
            crate::message_number(above(dt)),
            crate::message_number(dt)
        )
    );
    drop(held);
}

// ------------------------------------------- the fuel-temperature reactor

use genesis_kernel::reactor_spert3_fuel;

fn fuel_step(inlet_k: f64, reactivity: f64, flow: f64, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": "reactor_spert3_fuel",
        "parameters": {
            "inlet_temperature_k": inlet_k,
            "reactivity_inserted_dollar": reactivity,
            "coolant_flow_m_per_s": flow,
            "dt_s": dt_s
        }
    })
}

fn fuel_run(inlet_k: f64, loads: &[(f64, f64)], dt_s: f64) -> Value {
    let segments: Vec<Value> = loads
        .iter()
        .map(|(rho, flow)| {
            json!({ "steps": 3, "load": { "reactivity_inserted_dollar": rho, "coolant_flow_m_per_s": flow } })
        })
        .collect();
    json!({
        "domain": "energy",
        "card": "reactor_spert3_fuel",
        "initial": { "inlet_temperature_k": inlet_k },
        "dt_s": dt_s,
        "segments": segments,
        "record_every": 1
    })
}

/// The fuel-temperature reactor has three bands. Its inlet temperature is refused by name
/// before a permit at the next number outside each band, and needs the permit at the last
/// number inside. Its reactivity and flow are held only to the physical limits, a reactivity
/// above zero and a flow of zero or more, in every band; a later segment may carry another
/// flow, and not another reactivity.
#[tokio::test]
async fn each_band_of_the_fuel_reactor_is_refused_by_name_before_a_permit_at_the_last_number_on_each_side() {
    let reactor = reactor_spert3_fuel::ReactorSpert3Fuel::e_core_low_initial_power();
    let dt = reactor.gate_dt_s().unwrap();
    let floor = reactor.floor_dt_s().unwrap();
    let state = AppState::new(1);
    let held = acquire_physics_permit(&state).expect("the one permit");

    let needs_the_permit = |request: Value| {
        let state = &state;
        async move {
            match compute_step(state, request.clone()).await {
                Err(DoorError::TooMany(_)) => {}
                Err(other) => panic!("{request}: {}", other.message()),
                Ok(_) => panic!("{request}: sealed with the pool held"),
            }
        }
    };
    let run_needs_the_permit = |request: Value| {
        let state = &state;
        async move {
            match trajectory::run(state, request.clone()).await {
                Err(DoorError::TooMany(_)) => {}
                Err(other) => panic!("{request}: {}", other.message()),
                Ok(_) => panic!("a run sealed with the pool held"),
            }
        }
    };
    // The step and the run are both refused, with the same sentence.
    let refused = |step: Value, run: Value| {
        let state = &state;
        async move {
            let on_step = message(compute_step(state, step.clone()).await.unwrap_err());
            let on_run = message(trajectory::run(state, run).await.err().expect("refused"));
            assert_eq!(on_step, on_run, "{step}");
            assert!(on_step.starts_with("Physical Failure: '"), "{on_step}");
            assert!(on_step.ends_with(" That state is not sealed."), "{on_step}");
            on_step
        }
    };

    let mut ends = 0;
    for band in 0..3 {
        let (t_min, t_max) = (reactor.band_inlet_min_k[band], reactor.band_inlet_max_k[band]);
        let (rho_min, rho_max) = (
            reactor.band_reactivity_min_dollar[band],
            reactor.band_reactivity_max_dollar[band],
        );
        let (v_min, v_max) = (reactor.band_flow_min_m_per_s[band], reactor.band_flow_max_m_per_s[band]);
        let rho_mid = 0.5 * (rho_min + rho_max);

        // Inside: every end of the band, the old walls of reactivity and flow and past them,
        // and the floor of the clock.
        for inside in [
            fuel_step(t_min, rho_min, v_min, dt),
            fuel_step(t_max, rho_max, v_max, dt),
            fuel_step(t_min, rho_max, v_min, dt),
            fuel_step(t_max, rho_min, v_max, floor),
            fuel_step(t_min, below(rho_min), v_min, dt),
            fuel_step(t_min, above(rho_max), v_max, dt),
            fuel_step(t_min, 1.0e-300, 0.0, dt),
            fuel_step(t_max, 5.0, 100.0, dt),
            fuel_step(t_min, rho_mid, 0.0, dt),
            fuel_step(t_min, rho_mid, if v_max == 0.0 { 1.0e-30 } else { above(v_max) }, dt),
        ] {
            needs_the_permit(inside).await;
        }
        // A run of several segments under one load, at each end of the spans, and one whose
        // later segments carry other flows.
        for loads in [
            [(rho_min, v_min), (rho_min, v_min), (rho_min, v_min)],
            [(rho_max, v_max), (rho_max, v_max), (rho_max, v_max)],
            [(rho_mid, v_max), (rho_mid, v_min), (rho_mid, 50.0)],
        ] {
            run_needs_the_permit(fuel_run(t_min, &loads, dt)).await;
        }
        // A run is one burst: a later segment with another reactivity refuses the whole run by
        // name before a permit and before its first step, whatever its flow.
        for later_flow in [v_min, v_max] {
            let changed = message(
                trajectory::run(&state, fuel_run(t_min, &[(rho_min, v_min), (rho_min, v_min), (rho_max, later_flow)], dt))
                    .await
                    .err()
                    .expect("refused"),
            );
            assert_eq!(
                changed,
                format!(
                    "Physical Failure: 'reactivity_inserted_dollar' {} dollar is not the {} dollar of the first segment: a run of the reactor_spert3_fuel card is one burst with one reactivity, ramped in and then held (ReactivityChangedDuringBurst). That state is not sealed.",
                    crate::message_number(rho_max),
                    crate::message_number(rho_min),
                )
            );
        }

        // The inlet temperature, one number outside each end of the band.
        for outside in [below(t_min), above(t_max)] {
            let sentence = refused(
                fuel_step(outside, rho_mid, v_min, dt),
                fuel_run(outside, &[(rho_mid, v_min)], dt),
            )
            .await;
            assert!(sentence.contains("'inlet_temperature_k'"), "{sentence}");
            assert!(sentence.contains("(InletTemperatureOutsideCard)"), "{sentence}");
            ends += 1;
        }

        // A reactivity of zero or less, on the first segment and on a later one.
        for outside in [0.0, -f64::MIN_POSITIVE, -1.0] {
            let sentence = refused(
                fuel_step(t_min, outside, v_min, dt),
                fuel_run(t_min, &[(outside, v_min)], dt),
            )
            .await;
            assert_eq!(
                sentence,
                format!(
                    "Physical Failure: 'reactivity_inserted_dollar' {} dollar is at or below 0 dollar, the exclusive minimum of the reactor_spert3_fuel card (ReactivityNotPositive). That state is not sealed.",
                    crate::message_number(outside),
                )
            );
            let later = message(
                trajectory::run(&state, fuel_run(t_min, &[(rho_mid, v_min), (rho_mid, v_min), (outside, v_min)], dt))
                    .await
                    .err()
                    .expect("refused"),
            );
            assert_eq!(later, sentence);
            ends += 1;
        }

        // A flow below zero, on the first segment and on a later one. The numbers next to zero
        // are refused by the number rule before any bound is read, so a small flow that a
        // 32-bit slot still holds is tried.
        for outside in [-1.0e-30, -1.0] {
            let sentence = refused(
                fuel_step(t_min, rho_mid, outside, dt),
                fuel_run(t_min, &[(rho_mid, outside)], dt),
            )
            .await;
            assert_eq!(
                sentence,
                format!(
                    "Physical Failure: 'coolant_flow_m_per_s' {} m/s is below 0 m/s, the inclusive minimum of the reactor_spert3_fuel card (NegativeFlow). That state is not sealed.",
                    crate::message_number(outside),
                )
            );
            let later = message(
                trajectory::run(&state, fuel_run(t_min, &[(rho_mid, v_min), (rho_mid, outside)], dt))
                    .await
                    .err()
                    .expect("refused"),
            );
            assert_eq!(later, sentence);
            ends += 1;
        }
    }
    assert_eq!(ends, 21);

    // A flow of one band is taken in another: the hot bands' smallest flow in the cold band,
    // and no flow in a hot band.
    let cold = reactor.band_inlet_min_k[0];
    let hot = reactor.band_inlet_min_k[2];
    let hot_flow = reactor.band_flow_min_m_per_s[2];
    needs_the_permit(fuel_step(cold, 1.0, hot_flow, dt)).await;
    needs_the_permit(fuel_step(hot, 1.0, 0.0, dt)).await;

    // Between two bands: 300 degF.
    let between = (300.0 - 32.0) / 1.8 + 273.15;
    assert!(reactor.band_of(between).is_none());
    let sentence = message(compute_step(&state, fuel_step(between, 1.0, hot_flow, dt)).await.unwrap_err());
    assert!(
        sentence.starts_with(&format!(
            "Physical Failure: 'inlet_temperature_k' {} K is in none of the three tested bands of the reactor_spert3_fuel card: ",
            crate::message_number(between)
        )),
        "{sentence}"
    );
    assert!(sentence.ends_with("(InletTemperatureOutsideCard). That state is not sealed."), "{sentence}");

    // The clock: the number under the floor and the number over the ceiling.
    let mid = (reactor.band_inlet_min_k[0], 1.0, reactor.band_flow_min_m_per_s[0]);
    let under = message(compute_step(&state, fuel_step(mid.0, mid.1, mid.2, below(floor))).await.unwrap_err());
    assert_eq!(
        under,
        format!(
            // Re-pinned for 0.10.1: a step below the floor carries the name DtBelowFloor.
            "Physical Violation: 'dt_s' {} is below {} s, the step floor of the reactor_spert3_fuel card (DtBelowFloor).",
            crate::message_number(below(floor)),
            crate::message_number(floor)
        )
    );
    let over = message(compute_step(&state, fuel_step(mid.0, mid.1, mid.2, above(dt))).await.unwrap_err());
    assert_eq!(
        over,
        format!(
            // Re-pinned for 0.10.1: a step above the ceiling carries the name DtAboveCeiling.
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the reactor_spert3_fuel card (DtAboveCeiling).",
            crate::message_number(above(dt)),
            crate::message_number(dt)
        )
    );
    drop(held);
}
