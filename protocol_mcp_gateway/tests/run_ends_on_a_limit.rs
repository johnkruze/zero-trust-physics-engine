//! Runs whose end in decimal is exactly a limit of a card: a clock, a length
//! of track, a last date. The clock or the distance is a running sum in
//! 64-bit arithmetic, so each such comparison carries the card's published
//! rounding allowance, and the run lands every step. One more step is
//! refused by the card's name for the limit, and the run keeps its proof up
//! to the last step that landed.
//!
//! No measurement is read here.

use std::time::Duration;

use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
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

async fn run(base: &str, body: &Value) -> Value {
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
    let url = format!("{base}/run_trajectory");
    let mut last_error = None;
    for _ in 0..20 {
        match client.post(&url).json(body).send().await {
            Ok(response) => {
                let text = response.text().await.unwrap();
                return serde_json::from_str(&text).unwrap_or_else(|_| json!({ "door": text }));
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("run_trajectory never accepted a connection: {last_error:?}");
}

/// The run lands every one of its `steps`; the same run with one step more
/// is refused at that step by `name` and is sealed up to the last landed.
async fn lands_whole_and_one_more_is_named(base: &str, body: &Value, steps: u64, name: &str) {
    let whole = run(base, body).await;
    assert_eq!(whole["outcome"], "completed", "{body}: {whole}");
    assert_eq!(whole["steps_landed"], steps, "{body}");
    assert!(whole["run_seal"].is_string());
    let mut longer = body.clone();
    longer["segments"][0]["steps"] = json!(steps + 1);
    longer["record_every"] = json!(steps + 1);
    let refused = run(base, &longer).await;
    // The track's end, the last day and the 60 s are ends their cards
    // declare: the run is ended there. Every other limit refuses.
    let declared_end = matches!(name, "TravelBeyondTrack" | "MissionTimeOutsideCard" | "TimeBeyondCard")
        && matches!(body["card"].as_str(), Some("tire_eve_r19_ice" | "generator_mmrtg_f1" | "grid_gb"));
    assert_eq!(refused["outcome"], if declared_end { "ended" } else { "refused" }, "{longer}: {refused}");
    assert_eq!(refused["reject"], name, "{longer}");
    assert_eq!(refused["steps_landed"], steps, "{longer}");
    assert_eq!(refused["final_state"], whole["final_state"], "{longer}");
}

/// A decimal with `digits` places from a whole number of its last place.
fn decimal(units: u64, digits: u32) -> f64 {
    let scale = 10_u64.pow(digits);
    format!("{}.{:0width$}", units / scale, units % scale, width = digits as usize).parse().unwrap()
}

/// The tyre: the track ends at 6 m and the hold at one slip at 21 s. A run
/// is at most 16 s, so each run here ends exactly on one limit. Sixteen step
/// sizes at three carriage speeds, each from the decimal start that ends on
/// 6 m before the hold runs out; the same sizes from 5 s to exactly 21 s;
/// and two runs to the end of the track that once came back short by a step.
#[tokio::test]
async fn a_tyre_run_that_ends_on_the_track_end_lands_every_step() {
    let base = spawn_app().await;
    let tyre = |start_m: f64, held_s: f64, dt_s: f64, steps: u64, speed_m_s: f64| {
        json!({
            "domain": "materials", "card": "tire_eve_r19_ice",
            "initial": { "distance_m": start_m, "slip_held_s": held_s }, "dt_s": dt_s,
            "segments": [{ "steps": steps, "load": {
                "normal_load_n": 5000.0, "inflation_pressure_pa": 250000.0,
                "slip_ratio": 0.1, "carriage_speed_m_s": speed_m_s } }],
            "record_every": steps
        })
    };
    // step sizes in units of 0.0001 s; each divides 8 s
    let sizes_e4: [u64; 16] = [
        80_000, 40_000, 20_000, 16_000, 10_000, 5_000, 4_000, 2_500, 2_000, 1_250, 1_000, 500,
        400, 200, 100, 50,
    ];
    for speed_e2 in [5_u64, 6, 7] {
        for dt_e4 in sizes_e4 {
            // 8 s of steps ending on 6 m, from 5 s held: the hold has 8 s to spare
            let steps = 80_000 / dt_e4;
            let start_e6 = 6_000_000 - steps * speed_e2 * dt_e4;
            let body = tyre(decimal(start_e6, 6), 5.0, decimal(dt_e4, 4), steps, decimal(speed_e2, 2));
            lands_whole_and_one_more_is_named(&base, &body, steps, "TravelBeyondTrack").await;
        }
    }
    for dt_e4 in sizes_e4 {
        // 16 s of steps from 5 s held, ending on 21 s
        let steps = 160_000 / dt_e4;
        let body = tyre(0.0, 5.0, decimal(dt_e4, 4), steps, 0.06);
        lands_whole_and_one_more_is_named(&base, &body, steps, "HoldBeyondMeasured").await;
    }
    lands_whole_and_one_more_is_named(&base, &tyre(0.0, 5.0, 16.0, 1, 0.06), 1, "HoldBeyondMeasured")
        .await;
    // the last of the runs of 0.1 s along the track, from where 900 steps left it
    let mut start_m = 0.0_f64;
    for _ in 0..900 {
        start_m += 0.06 * 0.1;
    }
    lands_whole_and_one_more_is_named(&base, &tyre(start_m, 5.0, 0.1, 100, 0.06), 100, "TravelBeyondTrack")
        .await;
    lands_whole_and_one_more_is_named(&base, &tyre(5.44, 5.0, 8.0, 1, 0.07), 1, "TravelBeyondTrack")
        .await;
    // a start one millimetre nearer the end: the last step is past the allowance
    // (the end of the track is the end the card declares, so the run is ended there)
    let short = run(&base, &tyre(start_m + 0.001, 5.0, 0.1, 100, 0.06)).await;
    assert_eq!(short["outcome"], "ended");
    assert_eq!(short["reject"], "TravelBeyondTrack");
    assert_eq!(short["steps_landed"], 99);
}

/// The Great Britain grid: the card ends at 60 s. Twenty step sizes, five
/// that once came back short by a step among them.
#[tokio::test]
async fn a_grid_run_of_sixty_seconds_lands_every_step() {
    let base = spawn_app().await;
    let grid = |dt_s: f64, steps: u64| {
        json!({
            "domain": "energy", "card": "grid_gb",
            "initial": { "frequency_hz": 50.0, "inertia_mw_s": 200000.0, "demand_mw": 30000.0 },
            "dt_s": dt_s,
            "segments": [{ "steps": steps, "load": { "loss_mw": 1000.0 } }],
            "record_every": steps
        })
    };
    // step sizes in units of 0.000001 s
    let sizes_e6: [u64; 20] = [
        20_000, 16_000, 15_625, 15_000, 12_500, 12_000, 10_000, 8_000, 7_500, 6_000, 5_000, 4_000,
        3_000, 2_500, 2_000, 1_600, 1_500, 1_200, 1_000, 800,
    ];
    for dt_e6 in sizes_e6 {
        let steps = 60_000_000 / dt_e6;
        assert_eq!(steps * dt_e6, 60_000_000);
        let body = grid(decimal(dt_e6, 6), steps);
        lands_whole_and_one_more_is_named(&base, &body, steps, "TimeBeyondCard").await;
    }
    // the deepest corner of the spans falls under the lowest compared
    // frequency, 49.55 Hz: refused by name and sealed to the step before
    let deepest = run(&base, &json!({
        "domain": "energy", "card": "grid_gb",
        "initial": { "frequency_hz": 49.85, "inertia_mw_s": 111000.0, "demand_mw": 18163.0 },
        "dt_s": 0.015625,
        "segments": [{ "steps": 3840, "load": { "loss_mw": 1402.0 } }],
        "record_every": 3840
    }))
    .await;
    assert_eq!(deepest["outcome"], "refused");
    assert_eq!(deepest["reject"], "FrequencyBelowCompared");
    assert!(deepest["final_state"]["lowest_frequency_hz"].as_f64().unwrap() >= 49.55);
    assert!(deepest["final_state"].get("below_statutory_limit").is_none());
}

/// The tokamak: the card ends 5.475 s after the start of the heated window,
/// the longest compared window. Twelve step sizes from the start. A start at
/// 5 s on the caller's clock carries a pulse on: the card takes it, and
/// without from the door refuses it by name; continued from the sealed run
/// that reached it, the run lands every step to the end of the window.
#[tokio::test]
async fn a_tokamak_run_to_the_end_of_the_window_lands_every_step() {
    let base = spawn_app().await;
    let body = |start_s: f64, dt_s: f64, steps: u64| json!({
        "domain": "energy", "card": "tokamak_jet_dte1",
        "initial": { "thermal_energy_j": 4000000.0, "fast_ion_energy_j": 500000.0, "elapsed_time_s": start_s },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": {
            "heating_power_w": 20000000.0, "ohmic_power_w": 500000.0,
            "line_density_per_m3": 6e19, "plasma_current_a": 3800000.0,
            "toroidal_field_t": 4.0, "tritium_fraction": 0.5 } }],
        "record_every": steps
    });
    // step sizes in units of 0.000001 s
    for dt_e6 in [1_000_u64, 730, 500, 365, 300, 250, 219, 200, 150, 100, 50, 25] {
        for start_e6 in [0_u64, 5_000_000] {
            let steps = (5_475_000 - start_e6) / dt_e6;
            if steps * dt_e6 != 5_475_000 - start_e6 {
                continue;
            }
            let request = body(decimal(start_e6, 6), decimal(dt_e6, 6), steps);
            if start_e6 == 0 {
                lands_whole_and_one_more_is_named(&base, &request, steps, "TimeBeyondCard").await;
            } else {
                let refused = run(&base, &request).await;
                let text = refused["door"].as_str().unwrap_or_default();
                assert!(text.ends_with("(ContinuationWithoutUpstream)."), "{refused}");
            }
        }
    }
    // Continued from 5000 steps of 1 ms, sealed: 475 more land, one more ends.
    let upstream = body(0.0, 0.001, 5_000);
    let up = run(&base, &upstream).await;
    assert_eq!(up["outcome"], "completed", "{up}");
    let state = &up["final_state"];
    let mut carried_on = body(0.0, 0.001, 475);
    carried_on["initial"] = json!({
        "thermal_energy_j": state["thermal_energy_j"],
        "fast_ion_energy_j": state["fast_ion_energy_j"],
        "elapsed_time_s": state["t_s"]
    });
    carried_on["from"] = json!({ "request": upstream, "run_seal": up["run_seal"] });
    lands_whole_and_one_more_is_named(&base, &carried_on, 475, "TimeBeyondCard").await;
}

/// The generator: the card ends on mission day 4,087, 353,116,800 s. Twelve
/// step sizes at two lengths, each from the decimal start that leaves a
/// whole number of steps. The last day lies past 2^28 s, where the spacing
/// of 64-bit numbers doubles; the two runs of a thousand steps of a day
/// start before it and cross it.
#[tokio::test]
async fn a_generator_run_that_ends_on_the_last_day_lands_every_step() {
    let base = spawn_app().await;
    let mut crossed = 0;
    // step sizes in units of 0.1 s
    for dt_e1 in [
        42_244_999_u64, 10_000_001, 864_000, 864_005, 36_001, 10_003, 6_007, 1_001, 606, 101, 83, 41,
    ] {
        for steps in [10_u64, 1_000] {
            if steps * dt_e1 > 3_531_168_000 {
                continue;
            }
            crossed += u32::from(decimal(3_531_168_000 - steps * dt_e1, 1) < 268_435_456.0);
            let body = json!({
                "domain": "energy", "card": "generator_mmrtg_f1",
                "initial": { "mission_time_s": decimal(3_531_168_000 - steps * dt_e1, 1) },
                "dt_s": decimal(dt_e1, 1),
                "segments": [{ "steps": steps, "load": {} }],
                "record_every": steps
            });
            lands_whole_and_one_more_is_named(&base, &body, steps, "MissionTimeOutsideCard").await;
        }
    }
    assert_eq!(crossed, 2, "runs that start before 2^28 s");
}

/// The satellite: the card ends on 2026-09-20 00:00, 843,134,400 s. Twelve step
/// sizes over a thousand steps, and four runs of half a million and a
/// million steps, the lengths at which the running sum of the clock is late
/// by more than the spacing of 64-bit numbers at the date.
#[tokio::test]
async fn a_satellite_run_that_ends_on_the_last_date_lands_every_step() {
    let base = spawn_app().await;
    // (the step in units of 0.01 s, steps)
    for (dt_e2, steps) in [
        (1_000_u64, 1_000_u64), (750, 1_000), (640, 1_000), (500, 1_000), (250, 1_000), (160, 1_000),
        (110, 1_000), (100, 1_000), (70, 1_000), (50, 1_000), (30, 1_000), (10, 1_000),
        (60, 500_000), (10, 1_000_000), (7, 1_000_000), (1, 1_000_000),
    ] {
        let body = json!({
            "domain": "aerospace", "card": "satellite_lageos1",
            "initial": {
                "epoch_utc_s": decimal(84_313_440_000 - steps * dt_e2, 2),
                "forecast_elapsed_s": 0.0,
                "position_m": [3767126.6844992605, 8956617.400906362, 7564717.856833777],
                "velocity_m_s": [4119.825668508274, 1676.7840658085788, -4053.2216929389388]
            },
            "dt_s": decimal(dt_e2, 2),
            "segments": [{ "steps": steps, "load": {} }],
            "record_every": steps
        });
        if steps == 1_000_000 {
            // one step more would pass the 1,000,000 steps of a run
            let whole = run(&base, &body).await;
            assert_eq!(whole["outcome"], "completed", "{body}: {whole}");
            assert_eq!(whole["steps_landed"], steps);
        } else {
            lands_whole_and_one_more_is_named(&base, &body, steps, "EpochOutsideCard").await;
        }
    }
}
