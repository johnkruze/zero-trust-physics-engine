//! `propeller_apc_10x7e` through `POST /solve` against a direct call to
//! `genesis_solvers`, its refusals at the door, and `POST /verify_solution`
//! against edits of a sealed solution.

use std::time::Duration;

use genesis_solvers::propeller_apc_10x7e::{self as propeller, PropellerQuestion};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app()).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::new();
    let url = format!("{base}{path}");
    let text = serde_json::to_string(body).unwrap();
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(text.clone())
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                return (status, response.text().await.unwrap());
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

fn ask(rev_per_min: f64, airspeed: f64, density: f64) -> Value {
    json!({ "solver": "propeller_apc_10x7e", "parameters": {
        "rotational_speed_rev_per_min": rev_per_min,
        "airspeed_m_s": airspeed,
        "air_density_kg_m3": density,
    }})
}

fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

#[tokio::test]
async fn a_moving_and_a_resting_propeller_match_the_solver_bit_for_bit() {
    let base = spawn_app().await;
    for (speed, airspeed, density) in [(5000.0, 8.0, 1.225), (6000.0, 0.0, 1.225), (4007.0, 12.68, 1.25), (6531.0, 3.1, 1.2), (5000.0, 8.0, 1.0)] {
        let request = ask(speed, airspeed, density);
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        let direct = propeller::solve(&PropellerQuestion {
            rotational_speed_rev_per_min: speed,
            airspeed_m_s: airspeed,
            air_density_kg_m3: density,
        })
        .expect("inside the range");

        assert_eq!(body["inputs"], request);
        assert_eq!(body["engine_version"], format!("genesis_solvers_v{}", genesis_solvers::VERSION));
        assert_eq!(body["outcome"], "converged");
        assert!(body["reject"].is_null() && body["reject_detail"].is_null());
        assert_eq!(body["iterations"], direct.iterations);
        let result = &body["result"];
        for (name, value) in [
            ("advance_ratio", direct.advance_ratio),
            ("thrust_coefficient", direct.thrust_coefficient),
            ("power_coefficient", direct.power_coefficient),
            ("thrust_n", direct.thrust_n),
            ("torque_n_m", direct.torque_n_m),
            ("shaft_power_w", direct.shaft_power_w),
            ("efficiency", direct.efficiency),
        ] {
            assert_eq!(result[name], value, "{name}");
            assert_eq!(result[format!("{name}_hex")], bits(value), "{name}");
        }
        for (name, values) in [
            ("ring_inflow_angle_rad", &direct.inflow_angle_rad),
            ("ring_thrust_n", &direct.ring_thrust_n),
            ("ring_torque_n_m", &direct.ring_torque_n_m),
        ] {
            let hexes: Vec<String> = values.iter().map(|value| bits(*value)).collect();
            assert_eq!(result[format!("{name}_hex")], json!(hexes), "{name}");
            assert_eq!(result[name].as_array().unwrap().len(), propeller::ANNULI);
        }
        assert_eq!(body["invariants"]["worst_ring_balance_gap_hex"], bits(direct.worst_ring_balance_gap));
        assert_eq!(body["invariants"]["thrust_balance_gap_hex"], bits(direct.thrust_balance_gap));

        // The field hash is the count, then the three ring arrays.
        let mut hasher = Sha256::new();
        hasher.update((3 * propeller::ANNULI as u64).to_le_bytes());
        for value in direct.inflow_angle_rad.iter().chain(&direct.ring_thrust_n).chain(&direct.ring_torque_n_m) {
            hasher.update(value.to_le_bytes());
        }
        assert_eq!(body["field_sha256"], hex::encode(hasher.finalize()));

        // The same request again is the same bytes.
        let (_, again) = post(&base, "/solve", &request).await;
        assert_eq!(again, text);

        // The solution holds, and an edited thrust does not.
        let (status, verdict) = post(&base, "/verify_solution", &body).await;
        assert_eq!(status, 200, "{verdict}");
        let verdict: Value = serde_json::from_str(&verdict).unwrap();
        assert_eq!(verdict["holds"], true, "{verdict}");
        let mut edited = body.clone();
        edited["result"]["thrust_n_hex"] = json!(bits(direct.thrust_n * 1.01));
        let (status, verdict) = post(&base, "/verify_solution", &edited).await;
        if status == 200 {
            let verdict: Value = serde_json::from_str(&verdict).unwrap();
            assert_eq!(verdict["holds"], false, "{verdict}");
            assert_eq!(verdict["cause"], "result");
        } else {
            assert_eq!(status, 422, "{verdict}");
        }
    }
}

#[tokio::test]
async fn a_question_outside_the_range_is_refused_by_name_with_no_seal() {
    let base = spawn_app().await;
    for (request, name) in [
        (ask(4006.0, 8.0, 1.225), "RotationalSpeedOutOfRange"),
        (ask(6532.0, 8.0, 1.225), "RotationalSpeedOutOfRange"),
        (ask(4100.0, 0.0, 1.225), "RotationalSpeedOutOfRange"),
        (ask(6600.0, 0.0, 1.225), "RotationalSpeedOutOfRange"),
        (ask(4007.0, 1.8829, 1.225), "ReynoldsNumberOutOfRange"),
        (ask(4500.0, 4.8, 1.0), "ReynoldsNumberOutOfRange"),
        (ask(6500.0, 8.0, 1.3), "ReynoldsNumberOutOfRange"),
        (ask(5000.0, 8.0, 0.9), "AirDensityOutOfRange"),
        (ask(5000.0, 8.0, 1.31), "AirDensityOutOfRange"),
        (ask(5000.0, -1.0, 1.225), "AirspeedNegative"),
        (ask(5000.0, 1.0, 1.225), "AdvanceRatioOutOfRange"),
        (ask(5000.0, 16.0, 1.225), "AdvanceRatioOutOfRange"),
        (ask(5000.0, 1.0e-12, 1.225), "AdvanceRatioOutOfRange"),
    ] {
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 422, "{request}: {text}");
        assert!(text.starts_with("Physical Violation: "), "{text}");
        assert!(text.ends_with(&format!("({name}).")), "{text}");
        assert!(!text.contains("solution_seal"), "{text}");
    }
    // A rotational speed and a density both outside: the solver names the first it asks.
    let (status, text) = post(&base, "/solve", &ask(100.0, 8.0, 5.0)).await;
    assert_eq!(status, 422);
    assert_eq!(
        text,
        "Physical Violation: rotational speed 100 rev/min is outside 4007..=6531 rev/min, the speeds compared with measurement with the air moving (RotationalSpeedOutOfRange)."
    );
    // Nothing is filled in and nothing else is taken.
    let mut missing = ask(5000.0, 8.0, 1.225);
    missing["parameters"].as_object_mut().unwrap().remove("airspeed_m_s");
    let (status, text) = post(&base, "/solve", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("airspeed_m_s"), "{text}");
    let mut extra = ask(5000.0, 8.0, 1.225);
    extra["parameters"]["alpha_zero_lift_deg"] = json!(-4.0);
    let (status, text) = post(&base, "/solve", &extra).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("alpha_zero_lift_deg"), "{text}");
}

/// A question at an end of the advance-ratio range is answered at its own
/// computed advance ratio, sealed with it, and never moved to the end; one
/// beyond the allowance is refused with the sentence that names it.
#[tokio::test]
async fn an_advance_ratio_at_an_end_is_answered_at_its_own_value() {
    let base = spawn_app().await;
    for (speed, airspeed, density) in [
        (6000.0, 18.9992, 1.225),
        (6000.0, 2.8194, 1.225),
        (4500.0, 14.2494, 1.225),
        (4500.0, 2.11455, 1.225),
        (4007.0, 12.6882, 1.225),
        (6531.0, 3.069, 1.225),
        (4007.0, 1.8829, 1.226),
        (6531.0, 20.6806, 1.22),
        (6531.0, propeller::AIRSPEED_MAX_M_S, 1.2),
        (4008.0, 0.111 * (4008.0 / 60.0) * 0.254, 1.25),
        (4009.0, 0.748 * (4009.0 / 60.0) * 0.254, 1.225),
    ] {
        let request = ask(speed, airspeed, density);
        let (status, text) = post(&base, "/solve", &request).await;
        assert_eq!(status, 200, "{request}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged");
        let question = PropellerQuestion {
            rotational_speed_rev_per_min: speed,
            airspeed_m_s: airspeed,
            air_density_kg_m3: density,
        };
        let own = propeller::advance_ratio(&question);
        assert_eq!(body["result"]["advance_ratio_hex"], format!("{:016x}", own.to_bits()), "{request}");
        let direct = propeller::solve(&question).expect("direct");
        assert_eq!(body["result"]["thrust_n_hex"], format!("{:016x}", direct.thrust_n.to_bits()));
    }
    // Some of these are not the end itself: they would be refused without the allowance.
    let off = |speed: f64, airspeed: f64| {
        propeller::advance_ratio(&PropellerQuestion {
            rotational_speed_rev_per_min: speed,
            airspeed_m_s: airspeed,
            air_density_kg_m3: 1.225,
        })
    };
    assert!(off(6531.0, propeller::AIRSPEED_MAX_M_S) > propeller::ADVANCE_RATIO_MAX);
    let beyond = [4007.0, 4008.0, 4009.0, 4010.0, 4011.0, 4012.0]
        .into_iter()
        .filter(|speed| off(*speed, 0.111 * (speed / 60.0) * 0.254) < propeller::ADVANCE_RATIO_MIN)
        .count();
    assert!(beyond >= 1, "{beyond} of six speeds compute an advance ratio below 0.111 from it");
    for (speed, airspeed, density) in [
        (4007.0, 1.8828, 1.25),
        (6000.0, 18.9993, 1.2),
        (6000.0, 2.8193, 1.2),
        (6531.0, 20.6807, 1.2),
        (6531.0, propeller::AIRSPEED_MAX_M_S.next_up(), 1.2),
    ] {
        let (status, text) = post(&base, "/solve", &ask(speed, airspeed, density)).await;
        assert_eq!(status, 422, "{text}");
        assert!(
            text.ends_with("by more than the allowance of 5 units in the last place below (ADVANCE_RATIO_MIN_ALLOWANCE_ULP) and 4 above (ADVANCE_RATIO_MAX_ALLOWANCE_ULP), and the airspeed is not zero (AdvanceRatioOutOfRange)."),
            "{text}"
        );
        assert!(!text.contains("solution_seal"), "{text}");
    }
}

#[tokio::test]
async fn the_description_runs_its_examples_and_states_what_is_fitted() {
    let base = spawn_app().await;
    let (status, text) = post(&base, "/describe_solver", &json!({ "solver": "propeller_apc_10x7e" })).await;
    assert_eq!(status, 200, "{text}");
    let described: Value = serde_json::from_str(&text).unwrap();
    assert!(described["summary"].as_str().unwrap().contains("two blade-section constants fitted to this propeller's own measured thrust and power"));
    assert!(described["validation"]["limits"].as_str().unwrap().contains("extrapolation in rotational speed"));
    assert!(described["validation"]["limits"].as_str().unwrap().contains("interpolation in advance ratio"));
    assert!(described["summary"].as_str().unwrap().contains("it missed both of its lines"));
    assert!(described["validation"]["result"].as_str().unwrap().contains("both lines were missed"));
    assert_eq!(described["refusals"].as_array().unwrap().len(), 11);
    for key in ["example_solve", "example_solve_at_rest"] {
        let (status, text) = post(&base, "/solve", &described[key]).await;
        assert_eq!(status, 200, "{key}: {text}");
        let body: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(body["outcome"], "converged", "{key}");
        assert!(body["result"]["thrust_n"].as_f64().unwrap() > 1.0, "{key}");
    }
}
