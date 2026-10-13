//! The reactor's bounds are refused before a physics permit is taken.
//!
//! The single permit of the pool is held for the whole test. A request that
//! is refused for its own fault answers 422 by name. A request that is valid
//! would need the permit and answers 429. So a 422 under a held permit shows
//! that the refusal came first.

use genesis_kernel::reactor_spert3;
use serde_json::{Value, json};

use super::{AppState, DoorError, acquire_physics_permit, trajectory, verify_step};

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

    // Inside the bounds the request needs the permit, so the held pool says 429.
    for inside in [
        step(cold_min, 1.0, dt),
        step(cold_max, 1.0, dt),
        step(cold_min, rho_min, dt),
        step(cold_max, rho_max, dt),
        step(cold_min, 1.0, floor),
    ] {
        match verify_step(&state, inside.clone()).await {
            Err(DoorError::TooMany(_)) => {}
            Err(other) => panic!("{inside}: {}", other.message()),
            Ok(_) => panic!("{inside}: sealed with the pool held"),
        }
    }
    // A run of several segments under one reactivity, at each end of the span.
    for rho in [rho_min, rho_max] {
        match trajectory::run(&state, run(cold_min, &[rho, rho, rho], dt)).await {
            Err(DoorError::TooMany(_)) => {}
            Err(other) => panic!("{}", other.message()),
            Ok(_) => panic!("a run sealed with the pool held"),
        }
    }
    // A run is one burst: a later segment with another reactivity, inside the
    // span, refuses the whole run by name before a permit and before its
    // first step. One bit apart is another reactivity.
    for (first, later) in [(rho_min, rho_max), (rho_max, rho_min), (1.0, above(1.0)), (1.0, below(1.0))] {
        let changed = message(
            trajectory::run(&state, run(cold_min, &[first, first, later], dt))
                .await
                .err()
                .expect("refused"),
        );
        assert_eq!(
            changed,
            format!(
                "Physical Failure: 'reactivity_inserted_dollar' {} dollar is not the {} dollar of the first segment: a run of the reactor_spert3 card is one burst with one reactivity, stepped in at time zero and then held (ReactivityChangedDuringBurst). That state is not sealed.",
                crate::message_number(later),
                crate::message_number(first),
            )
        );
    }

    // The sentence names the field, the value, the bound and the refusal.
    let say = |field: &str, unit: &str, value: f64, bound: f64, upper: bool, name: &str| {
        format!(
            "Physical Failure: '{field}' {}{unit} is {} {}{unit}, the inclusive {} of the reactor_spert3 card ({name}). That state is not sealed.",
            crate::message_number(value),
            if upper { "above" } else { "below" },
            crate::message_number(bound),
            if upper { "maximum" } else { "minimum" },
        )
    };
    let cool = |value: f64, bound: f64, upper: bool| {
        say("coolant_temperature_k", " K", value, bound, upper, "CoolantTemperatureOutsideCard")
    };
    let react = |value: f64, bound: f64, upper: bool| {
        say("reactivity_inserted_dollar", " dollar", value, bound, upper, "ReactivityOutsideCard")
    };
    for (request, run_request, expected) in [
        (step(below(cold_min), 1.0, dt), run(below(cold_min), &[1.0], dt), cool(below(cold_min), cold_min, false)),
        (step(above(cold_max), 1.0, dt), run(above(cold_max), &[1.0], dt), cool(above(cold_max), cold_max, true)),
        (step(cold_min, below(rho_min), dt), run(cold_min, &[below(rho_min)], dt), react(below(rho_min), rho_min, false)),
        (step(cold_min, above(rho_max), dt), run(cold_min, &[above(rho_max)], dt), react(above(rho_max), rho_max, true)),
        // A later segment out of the span refuses the whole run before its first step.
        (
            step(cold_min, above(rho_max), dt),
            run(cold_min, &[1.0, 1.0, above(rho_max)], dt),
            react(above(rho_max), rho_max, true),
        ),
        (
            step(cold_min, below(rho_min), dt),
            run(cold_min, &[1.0, below(rho_min)], dt),
            react(below(rho_min), rho_min, false),
        ),
    ] {
        assert_eq!(message(verify_step(&state, request).await.unwrap_err()), expected);
        assert_eq!(
            message(trajectory::run(&state, run_request).await.err().expect("refused")),
            expected
        );
    }

    // The clock: the number under the floor and the number over the ceiling.
    let under = message(verify_step(&state, step(cold_min, 1.0, below(floor))).await.unwrap_err());
    assert_eq!(
        under,
        format!(
            "Physical Violation: 'dt_s' {} is below {} s, the step floor of the reactor_spert3 card.",
            crate::message_number(below(floor)),
            crate::message_number(floor)
        )
    );
    let over = message(verify_step(&state, step(cold_min, 1.0, above(dt))).await.unwrap_err());
    assert_eq!(
        over,
        format!(
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the reactor_spert3 card.",
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

/// The fuel-temperature reactor has three bands, and the span of the
/// reactivity and of the flow is the band's own. Every end of every band, of
/// each of the three fields, is refused by name before a permit at the next
/// number outside, and needs the permit at the last number inside.
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
            match verify_step(state, request.clone()).await {
                Err(DoorError::TooMany(_)) => {}
                Err(other) => panic!("{request}: {}", other.message()),
                Ok(_) => panic!("{request}: sealed with the pool held"),
            }
        }
    };
    // The step and the run are both refused, with the same sentence.
    let refused = |step: Value, run: Value| {
        let state = &state;
        async move {
            let on_step = message(verify_step(state, step.clone()).await.unwrap_err());
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
        let name = crate::descriptor::REACTOR_FUEL_BAND_DEGF[band];

        // Inside: every end of the band, and the floor of the clock.
        for inside in [
            fuel_step(t_min, rho_min, v_min, dt),
            fuel_step(t_max, rho_max, v_max, dt),
            fuel_step(t_min, rho_max, v_min, dt),
            fuel_step(t_max, rho_min, v_max, floor),
        ] {
            needs_the_permit(inside).await;
        }
        // A run of several segments under one load, at each end of the spans.
        for (rho, v) in [(rho_min, v_min), (rho_max, v_max)] {
            match trajectory::run(&state, fuel_run(t_min, &[(rho, v), (rho, v), (rho, v)], dt)).await {
                Err(DoorError::TooMany(_)) => {}
                Err(other) => panic!("{}", other.message()),
                Ok(_) => panic!("a run sealed with the pool held"),
            }
        }
        // A run is one burst: a later segment with another reactivity, or with
        // another flow, each inside the band's span, refuses the whole run by
        // name before a permit and before its first step.
        let changed = message(
            trajectory::run(&state, fuel_run(t_min, &[(rho_min, v_min), (rho_min, v_min), (rho_max, v_min)], dt))
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
        if v_min != v_max {
            let changed = message(
                trajectory::run(&state, fuel_run(t_min, &[(rho_mid, v_max), (rho_mid, v_min)], dt))
                    .await
                    .err()
                    .expect("refused"),
            );
            assert_eq!(
                changed,
                format!(
                    "Physical Failure: 'coolant_flow_m_per_s' {} m/s is not the {} m/s of the first segment: a run of the reactor_spert3_fuel card is one burst with one flow rate, and no tested burst changed its flow (FlowChangedDuringBurst). That state is not sealed.",
                    crate::message_number(v_min),
                    crate::message_number(v_max),
                )
            );
            // Both changed: the reactivity is named.
            let both = message(
                trajectory::run(&state, fuel_run(t_min, &[(rho_min, v_max), (rho_max, v_min)], dt))
                    .await
                    .err()
                    .expect("refused"),
            );
            assert!(both.ends_with("(ReactivityChangedDuringBurst). That state is not sealed."), "{both}");
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

        // The reactivity, one number outside each end of the band's own span.
        for outside in [below(rho_min), above(rho_max)] {
            let sentence = refused(
                fuel_step(t_min, outside, v_min, dt),
                fuel_run(t_min, &[(outside, v_min)], dt),
            )
            .await;
            assert!(sentence.contains("'reactivity_inserted_dollar'"), "{sentence}");
            assert!(sentence.contains("(ReactivityOutsideCard)"), "{sentence}");
            // Inside the widest published span the sentence names the band.
            let widest_min = reactor.band_reactivity_min_dollar.iter().copied().fold(f64::INFINITY, f64::min);
            let widest_max = reactor.band_reactivity_max_dollar.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if outside >= widest_min && outside <= widest_max {
                assert_eq!(
                    sentence,
                    format!(
                        "Physical Failure: 'reactivity_inserted_dollar' {} dollar is outside [{}, {}] dollars, the span tested in the {name} degF band of the reactor_spert3_fuel card (ReactivityOutsideCard). That state is not sealed.",
                        crate::message_number(outside),
                        crate::message_number(rho_min),
                        crate::message_number(rho_max),
                    )
                );
            }
            // A later segment out of the span refuses the whole run before its first step.
            let later = message(
                trajectory::run(&state, fuel_run(t_min, &[(rho_mid, v_min), (rho_mid, v_min), (outside, v_min)], dt))
                    .await
                    .err()
                    .expect("refused"),
            );
            assert_eq!(later, sentence);
            ends += 1;
        }

        // The flow, one number outside each end of the band's own span. The
        // numbers next to zero are refused by the number rule before any
        // bound is read, so the cold band's one value is tried at a small
        // flow that a 32-bit slot still holds, on each side of its zero.
        let under = if v_min == 0.0 { -1.0e-30 } else { below(v_min) };
        let over = if v_max == 0.0 { 1.0e-30 } else { above(v_max) };
        for outside in [under, over] {
            let sentence = refused(
                fuel_step(t_min, rho_mid, outside, dt),
                fuel_run(t_min, &[(rho_mid, outside)], dt),
            )
            .await;
            assert!(sentence.contains("'coolant_flow_m_per_s'"), "{sentence}");
            assert!(sentence.contains("(FlowOutsideCard)"), "{sentence}");
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
    assert_eq!(ends, 18);

    // A flow of one band is refused in another: the hot bands' smallest flow
    // in the cold band, and no flow in a hot band.
    let cold = reactor.band_inlet_min_k[0];
    let hot = reactor.band_inlet_min_k[2];
    let hot_flow = reactor.band_flow_min_m_per_s[2];
    let sentence = message(verify_step(&state, fuel_step(cold, 1.0, hot_flow, dt)).await.unwrap_err());
    assert_eq!(
        sentence,
        format!(
            "Physical Failure: 'coolant_flow_m_per_s' {} m/s is outside [{}, {}] m/s, the span tested in the {} degF band of the reactor_spert3_fuel card (FlowOutsideCard). That state is not sealed.",
            crate::message_number(hot_flow),
            crate::message_number(reactor.band_flow_min_m_per_s[0]),
            crate::message_number(reactor.band_flow_max_m_per_s[0]),
            crate::descriptor::REACTOR_FUEL_BAND_DEGF[0],
        )
    );
    let sentence = message(verify_step(&state, fuel_step(hot, 1.0, 0.0, dt)).await.unwrap_err());
    assert!(sentence.contains("'coolant_flow_m_per_s' 0 m/s is outside ["), "{sentence}");
    assert!(sentence.contains("(FlowOutsideCard)"), "{sentence}");

    // Between two bands: 300 degF.
    let between = (300.0 - 32.0) / 1.8 + 273.15;
    assert!(reactor.band_of(between).is_none());
    let sentence = message(verify_step(&state, fuel_step(between, 1.0, hot_flow, dt)).await.unwrap_err());
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
    let under = message(verify_step(&state, fuel_step(mid.0, mid.1, mid.2, below(floor))).await.unwrap_err());
    assert_eq!(
        under,
        format!(
            "Physical Violation: 'dt_s' {} is below {} s, the step floor of the reactor_spert3_fuel card.",
            crate::message_number(below(floor)),
            crate::message_number(floor)
        )
    );
    let over = message(verify_step(&state, fuel_step(mid.0, mid.1, mid.2, above(dt))).await.unwrap_err());
    assert_eq!(
        over,
        format!(
            "Physical Violation: 'dt_s' {} is above {} s, the step ceiling of the reactor_spert3_fuel card.",
            crate::message_number(above(dt)),
            crate::message_number(dt)
        )
    );
    drop(held);
}
