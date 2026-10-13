//! Permit and limit checks for `POST /run_trajectory`.
//!
//! A step count above 1_000_000, or a recording above 10_000 frames, is 422
//! and does not take a physics permit. A full pool is 429 and does not seal.
//! The hold is `#[cfg(test)]`. `app()` does not sleep.

use std::time::{Duration, Instant};

use genesis_kernel::{resonance, thermal};
use reqwest::Client;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use super::{app_with_permit_hold, new_gateway_state, physics_permit_count, trajectory};

const HOLD: Duration = Duration::from_millis(1500);
const ACQUIRE_WAIT: Duration = Duration::from_millis(300);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn limits_are_422_without_a_permit_and_a_full_pool_is_429() {
    let permits = physics_permit_count();
    let base = spawn_held_app().await;
    let client = Client::builder()
        .pool_max_idle_per_host(1024)
        .tcp_nodelay(true)
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    wait_until_healthy(&client, &base).await;

    let holders = fanout(&client, &base, "/verify_physics", &holder_body(), permits);
    tokio::time::sleep(ACQUIRE_WAIT).await;

    let too_many_steps = post(
        &client,
        &base,
        "/run_trajectory",
        &steps_body(1_000_001, 1_000_000),
    )
    .await;
    let too_many_frames = post(&client, &base, "/run_trajectory", &steps_body(10_001, 1)).await;
    let overflow = post(&client, &base, "/run_trajectory", &steps_body(1, 1)).await;
    // A capsule state off the card is refused before a permit too: above the
    // drag table, and at rest in the air at 40 km.
    let above_the_card = post(
        &client,
        &base,
        "/verify_physics",
        r#"{"domain":"aerospace","card":"capsule_stardust","parameters":{"radius_m":6509228.5,"radial_speed_m_s":-1842.1416729435698,"transverse_speed_m_s":12767.792058801891,"dt_s":0.05}}"#,
    )
    .await;
    let under_mach_one = post(
        &client,
        &base,
        "/run_trajectory",
        r#"{"domain":"aerospace","card":"capsule_stardust","initial":{"radius_m":6409228.5,"radial_speed_m_s":-100.0,"transverse_speed_m_s":347.0},"dt_s":0.05,"segments":[{"steps":1,"load":{}}],"record_every":1}"#,
    )
    .await;
    // A satellite state off the card is refused before a permit too: under
    // the radius band, and a compared start a second before the first date.
    let under_the_band = post(
        &client,
        &base,
        "/verify_physics",
        r#"{"domain":"aerospace","card":"satellite_lageos1","parameters":{"epoch_utc_s":840715200.0,"forecast_elapsed_s":0.0,"position_m":[10000000.0,0.0,0.0],"velocity_m_s":[0.0,-729.2,6313.5],"dt_s":10.0}}"#,
    )
    .await;
    let before_the_first_date = post(
        &client,
        &base,
        "/run_trajectory",
        r#"{"domain":"aerospace","card":"satellite_lageos1","initial":{"epoch_utc_s":840715199.0,"forecast_elapsed_s":0.0,"position_m":[3767126.6844992605,8956617.400906362,7564717.856833777],"velocity_m_s":[4119.825668508274,1676.7840658085788,-4053.2216929389388]},"dt_s":10.0,"segments":[{"steps":1,"load":{}}],"record_every":1}"#,
    )
    .await;
    // A generator mission time off the card is refused before a permit, so
    // the full pool does not turn it into a 429.
    let before_landing = post(
        &client,
        &base,
        "/verify_physics",
        r#"{"domain":"energy","card":"generator_mmrtg_f1","parameters":{"mission_time_s":-1.0,"dt_s":86400.0}}"#,
    )
    .await;
    let after_the_last_day = post(
        &client,
        &base,
        "/run_trajectory",
        r#"{"domain":"energy","card":"generator_mmrtg_f1","initial":{"mission_time_s":353116801.0},"dt_s":86400.0,"segments":[{"steps":1,"load":{}}],"record_every":1}"#,
    )
    .await;

    // A cantilever start off the card is refused before a permit too: a
    // deflection and a speed each inside its own bound whose envelope is
    // above the amplitude ceiling, and a speed above its own bound.
    let envelope_above_the_ceiling = post(
        &client,
        &base,
        "/verify_physics",
        r#"{"domain":"signal","card":"cantilever_eth","parameters":{"deflection_m":0.012,"velocity_m_s":0.6,"dt_s":0.00025}}"#,
    )
    .await;
    let speed_above_its_bound = post(
        &client,
        &base,
        "/run_trajectory",
        r#"{"domain":"signal","card":"cantilever_eth","initial":{"deflection_m":0.0,"velocity_m_s":1.0},"dt_s":0.00025,"segments":[{"steps":1,"load":{}}],"record_every":1}"#,
    )
    .await;
    // A start of the physical 2170 cell off the card is refused before a
    // permit too: a cell number between two cells that names neither, and a
    // capacity above the largest the cell had.
    let cell_that_is_not_held = post(
        &client,
        &base,
        "/verify_physics",
        r#"{"domain":"energy","card":"cell_2170_model3","parameters":{"cell_number":449.0,"state_of_charge":1.0,"capacity_ah":4.5,"case_temperature_least_c":25.0,"case_temperature_most_c":25.0,"current_a":4.5,"case_temperature_c":25.0,"dt_s":1.0}}"#,
    )
    .await;
    let capacity_above_the_card = post(
        &client,
        &base,
        "/run_trajectory",
        r#"{"domain":"energy","card":"cell_2170_model3","initial":{"cell_number":80.0,"state_of_charge":1.0,"capacity_ah":4.75,"case_temperature_least_c":25.0,"case_temperature_most_c":25.0},"dt_s":1.0,"segments":[{"steps":1,"load":{"current_a":4.5,"case_temperature_c":25.0}}],"record_every":1}"#,
    )
    .await;

    let held = join_all(holders).await;
    assert_eq!(held.len(), permits, "dropped holder tasks");
    for hit in &held {
        assert_eq!(hit.status, 200, "holder missed the pool: {}", hit.detail);
    }

    assert_eq!(too_many_steps.status, 422, "{}", too_many_steps.detail);
    assert!(
        too_many_steps
            .detail
            .contains("the run asks for more than 1000000 steps"),
        "{}",
        too_many_steps.detail
    );
    assert!(
        !too_many_steps.detail.contains("run_seal"),
        "{}",
        too_many_steps.detail
    );

    assert_eq!(too_many_frames.status, 422, "{}", too_many_frames.detail);
    assert!(
        too_many_frames
            .detail
            .contains("the run would record more than 10000 frames"),
        "{}",
        too_many_frames.detail
    );
    assert!(
        !too_many_frames.detail.contains("run_seal"),
        "{}",
        too_many_frames.detail
    );

    for (off_card, reject) in [
        (&above_the_card, "AltitudeOutsideCard"),
        (&under_mach_one, "MachOutsideCard"),
        (&under_the_band, "RadiusOutsideCard"),
        (&before_the_first_date, "EpochOutsideCard"),
        (&envelope_above_the_ceiling, "DeflectionAboveCard"),
        (&speed_above_its_bound, "DeflectionAboveCard"),
        (&cell_that_is_not_held, "CellNotRecognized"),
        (&capacity_above_the_card, "CapacityOutsideCard"),
    ] {
        assert_eq!(off_card.status, 422, "{}", off_card.detail);
        assert!(off_card.detail.contains(reject), "{}", off_card.detail);
        assert!(!off_card.detail.contains("_seal"), "{}", off_card.detail);
    }
    for off_card in [&before_landing, &after_the_last_day] {
        assert_eq!(off_card.status, 422, "{}", off_card.detail);
        assert!(
            off_card.detail.contains("MissionTimeOutsideCard"),
            "{}",
            off_card.detail
        );
        assert!(!off_card.detail.contains("_seal"), "{}", off_card.detail);
    }

    assert_eq!(overflow.status, 429, "{}", overflow.detail);
    assert!(
        overflow
            .detail
            .contains("Too many concurrent physics tasks."),
        "{}",
        overflow.detail
    );
    assert!(!overflow.detail.contains("run_seal"), "{}", overflow.detail);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_solve_takes_a_permit_and_a_range_error_does_not() {
    let permits = physics_permit_count();
    let base = spawn_held_app().await;
    let client = Client::builder()
        .pool_max_idle_per_host(1024)
        .tcp_nodelay(true)
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    wait_until_healthy(&client, &base).await;

    let holders = fanout(&client, &base, "/verify_physics", &holder_body(), permits);
    tokio::time::sleep(ACQUIRE_WAIT).await;

    let out_of_range = post(&client, &base, "/solve", &solve_body(19)).await;
    let overflow = post(&client, &base, "/solve", &solve_body(1)).await;

    let held = join_all(holders).await;
    assert_eq!(held.len(), permits, "dropped holder tasks");
    for hit in &held {
        assert_eq!(hit.status, 200, "holder missed the pool: {}", hit.detail);
    }

    assert_eq!(out_of_range.status, 422, "{}", out_of_range.detail);
    assert!(
        out_of_range
            .detail
            .contains("atomic number 19 is outside 1..=18"),
        "{}",
        out_of_range.detail
    );
    assert_eq!(overflow.status, 429, "{}", overflow.detail);
    assert!(
        overflow
            .detail
            .contains("Too many concurrent physics tasks."),
        "{}",
        overflow.detail
    );
    assert!(
        !overflow.detail.contains("solution_seal"),
        "{}",
        overflow.detail
    );
}

/// Release timing for one million thermal steps. Not part of the default suite.
#[tokio::test]
#[ignore]
async fn million_step_thermal_wall_time() {
    let node = thermal::ThermalNode::avionics_cold_plate();
    let dt = node.gate_dt_s().unwrap().min(0.1);
    let request = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": dt,
        "segments": [{
            "steps": 1_000_000,
            "load": {
                "heat_power_w": 1.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K
            }
        }],
        "record_every": 100_000
    });
    let state = new_gateway_state();
    let started = Instant::now();
    let response = match trajectory::run(&state, request).await {
        Ok(response) => response,
        Err(error) => panic!("million-step run: {}", error.message()),
    };
    let wall_s = started.elapsed().as_secs_f64();
    eprintln!(
        "million_step_thermal wall_s={wall_s:.6} outcome={} steps_landed={} frames={}",
        response.outcome,
        response.steps_landed,
        response.frames.len()
    );
    assert_eq!(
        response.outcome, "completed",
        "reject={:?} detail={:?} landed={}",
        response.reject, response.reject_detail, response.steps_landed
    );
    assert_eq!(response.steps_landed, 1_000_000);
    assert_eq!(response.frames.len(), 10);
}

/// Release timing for one million resonance steps. Not part of the default suite.
#[tokio::test]
#[ignore]
async fn million_step_resonance_wall_time() {
    let oscillator = resonance::DynamicOscillator::mount_10hz();
    let rest = resonance::OscillatorState::try_rest(&oscillator).expect("resonance rest");
    let dt = oscillator.gate_dt_s().expect("gate").min(0.1);
    let request = json!({
        "domain": "signal",
        "card": "resonance",
        "initial": {
            "displacement_m": rest.displacement_m,
            "velocity_m_s": rest.velocity_m_s,
            "t_s": rest.t_s,
            "energy_anchor_j": rest.energy_anchor_j,
            "boundary_work_j": rest.boundary_work_j
        },
        "dt_s": dt,
        "segments": [{
            "steps": 1_000_000,
            "load": { "accel_m_s2": rest.accel_m_s2 }
        }],
        "record_every": 100_000
    });
    let state = new_gateway_state();
    let started = Instant::now();
    let response = match trajectory::run(&state, request).await {
        Ok(response) => response,
        Err(error) => panic!("million-step resonance: {}", error.message()),
    };
    let wall_s = started.elapsed().as_secs_f64();
    eprintln!(
        "million_step_resonance wall_s={wall_s:.6} outcome={} steps_landed={} frames={}",
        response.outcome,
        response.steps_landed,
        response.frames.len()
    );
    assert_eq!(
        response.outcome, "completed",
        "reject={:?} detail={:?} landed={}",
        response.reject, response.reject_detail, response.steps_landed
    );
    assert_eq!(response.steps_landed, 1_000_000);
    assert_eq!(response.frames.len(), 10);
}

struct Hit {
    status: u16,
    detail: String,
}

fn fanout(client: &Client, base: &str, path: &str, body: &str, count: usize) -> JoinSet<Hit> {
    let mut join_set = JoinSet::new();
    for _ in 0..count {
        let client = client.clone();
        let base = base.to_string();
        let path = path.to_string();
        let body = body.to_string();
        join_set.spawn(async move { post(&client, &base, &path, &body).await });
    }
    join_set
}

async fn join_all(mut join_set: JoinSet<Hit>) -> Vec<Hit> {
    let mut hits = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        hits.push(joined.expect("physics task panicked"));
    }
    hits
}

async fn post(client: &Client, base: &str, path: &str, body: &str) -> Hit {
    let response = match client
        .post(format!("{base}{path}"))
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return Hit {
                status: 0,
                detail: format!("transport: {err}"),
            };
        }
    };
    let status = response.status().as_u16();
    let detail = response.text().await.unwrap_or_else(|err| err.to_string());
    Hit { status, detail }
}

fn holder_body() -> String {
    let node = genesis_kernel::grid_node::GridNode::copper_fuse_link();
    let temperature =
        serde_json::to_string(&genesis_kernel::grid_node::REFERENCE_TEMPERATURE_K).unwrap();
    let dt = serde_json::to_string(&node.gate_dt_s().unwrap()).unwrap();
    format!(
        r#"{{"domain":"energy","card":"grid_node","parameters":{{"current_a":0.0,"voltage_v":0.0,"temperature_k":{temperature},"upstream_voltage_v":25.0,"load_current_a":0.0,"ambient_k":{temperature},"dt_s":{dt}}}}}"#
    )
}

fn solve_body(atomic_number: u64) -> String {
    json!({ "solver": "atom_dft", "parameters": { "atomic_number": atomic_number } }).to_string()
}

fn steps_body(steps: u64, record_every: u64) -> String {
    let body = json!({
        "domain": "energy",
        "card": "thermal",
        "initial": { "temperature_k": thermal::REFERENCE_TEMPERATURE_K },
        "dt_s": 0.1,
        "segments": [{
            "steps": steps,
            "load": {
                "heat_power_w": 1.0,
                "ambient_k": thermal::REFERENCE_TEMPERATURE_K
            }
        }],
        "record_every": record_every
    });
    serde_json::to_string(&body).unwrap()
}

async fn spawn_held_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app_with_permit_hold(HOLD))
            .await
            .unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn wait_until_healthy(client: &Client, base: &str) {
    let mut last = String::new();
    for _ in 0..40 {
        match client.get(format!("{base}/health")).send().await {
            Ok(response) if response.status().as_u16() == 200 => return,
            Ok(response) => last = format!("status {}", response.status()),
            Err(err) => last = err.to_string(),
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("gateway did not become healthy: {last}");
}
