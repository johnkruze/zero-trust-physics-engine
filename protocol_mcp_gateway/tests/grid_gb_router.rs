//! The Great Britain card on the web connection.
//!
//! A step is bit-checked against the compiled kernel step with the same proof
//! bytes. A run of the 60 seconds after a loss is the kernel loop's, to the
//! bit; its tape verifies and its receipt matches. Two losses one after the
//! other are stepped by the same equations, flagged as outside the compared
//! trips. The run goes on past 60 s, flagged. Every published bound is
//! refused by name at the next representable number outside it. A loss
//! under 693 MW, the smallest compared, is answered with the flag, and a fall
//! under 49.55 Hz, the lowest compared frequency, lands with its flag down to
//! 48.8 Hz, where low-frequency demand disconnection stops it by name.
//!
//! The losses, inertias, demands and starting frequencies used here are those
//! of opened trips only (ranks 5 and 23). No held-back trip is run.

mod answer_text;

use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::grid_gb::{self, GridGb, GridLoad, GridState, StepReject};
use protocol_mcp_gateway::{VerifyPhysicsResponse, app, commit_request_proof};
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
    let client = Client::builder().timeout(Duration::from_secs(300)).build().unwrap();
    let url = format!("{base}{path}");
    let text = answer_text::as_sent(body);
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
                let text = response.text().await.unwrap();
                answer_text::remember(&text);
                return (status, text);
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
    panic!("{path} never accepted a connection: {last_error:?}");
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn below(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

fn above(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

fn card() -> GridGb {
    GridGb::interconnector_loss()
}

fn ceiling() -> f64 {
    card().gate_dt_s().unwrap()
}

/// Opened trips: start frequency Hz, inertia MW s, demand MW, loss MW.
const RANK_5: (f64, f64, f64, f64) = (49.942, 241_000.0, 27_132.0, 996.0);
const RANK_23: (f64, f64, f64, f64) = (50.077, 283_000.0, 28_510.0, 1402.0);

fn step_body(case: (f64, f64, f64, f64), dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": "grid_gb",
        "parameters": {
            "frequency_hz": case.0,
            "inertia_mw_s": case.1,
            "demand_mw": case.2,
            "loss_mw": case.3,
            "dt_s": dt_s
        }
    })
}

fn run_body(case: (f64, f64, f64, f64), dt_s: f64, steps: u64, record_every: u64) -> Value {
    json!({
        "domain": "energy",
        "card": "grid_gb",
        "initial": { "frequency_hz": case.0, "inertia_mw_s": case.1, "demand_mw": case.2 },
        "dt_s": dt_s,
        "segments": [{ "steps": steps, "load": { "loss_mw": case.3 } }],
        "record_every": record_every
    })
}

fn blank_frame(proof: [u8; 16]) -> LastStateFrame64 {
    LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof,
    }
}

/// The flags of the last frame of a tape.
fn last_flags(tape: &Value) -> u64 {
    let frames = tape["frames"].as_array().unwrap();
    let hex = frames.last().unwrap()["frame_hex"].as_str().unwrap();
    let mut bytes = [0u8; 64];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * index..2 * index + 2], 16).unwrap();
    }
    LastStateFrame64::from_bytes(bytes).flags
}

fn direct_loop(
    case: (f64, f64, f64, f64),
    dt_s: f64,
    limit: u64,
) -> (GridState, u64, Option<StepReject>) {
    let card = card();
    let mut state = GridState::try_start(&card, case.0, case.1, case.2).unwrap();
    let mut frame = blank_frame([0; 16]);
    let mut landed = 0;
    while landed < limit {
        match grid_gb::step(&card, &mut state, &mut frame, GridLoad { loss_mw: case.3 }, dt_s) {
            Ok(()) => landed += 1,
            Err(reject) => return (state, landed, Some(reject)),
        }
    }
    (state, landed, None)
}

fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn state_numbers(state: &GridState) -> Vec<(&'static str, f64)> {
    vec![
        ("t_s", state.t_s),
        ("frequency_hz", state.frequency_hz),
        ("response_mw", state.response_mw),
        ("relief_mw", state.relief_mw),
        ("net_power_mw", state.net_power_mw),
        ("frequency_rate_hz_per_s", state.frequency_rate_hz_per_s),
        ("lowest_frequency_hz", state.lowest_frequency_hz),
        ("lowest_time_s", state.lowest_time_s),
        ("start_frequency_hz", state.start_frequency_hz),
        ("inertia_mw_s", state.inertia_mw_s),
        ("demand_mw", state.demand_mw),
        ("kinetic_energy_mw_s", state.kinetic_energy_mw_s),
        ("kinetic_energy_anchor_mw_s", state.kinetic_energy_anchor_mw_s),
        ("energy_lost_mw_s", state.energy_lost_mw_s),
        ("energy_response_mw_s", state.energy_response_mw_s),
        ("frequency_deficit_integral_hz_s", state.frequency_deficit_integral_hz_s),
        ("energy_defect_mw_s", state.energy_defect_mw_s),
        ("residual_conservation_error", state.residual_conservation_error),
    ]
}

/// Every published number is the kernel's, to the bit, as the decimal and as
/// the hex beside it, and nothing else is published.
fn assert_state_is(published_state: &Value, state: &GridState, label: &str) {
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in state_numbers(state) {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_of_the_area_matches_the_compiled_step_with_the_same_proof_bytes() {
    let base = spawn_app().await;
    let cases = [
        step_body(RANK_5, ceiling()),
        step_body(RANK_23, ceiling()),
        step_body(RANK_5, card().floor_dt_s().unwrap()),
        step_body(
            (
                grid_gb::START_FREQUENCY_MIN_HZ,
                grid_gb::INERTIA_MIN_MW_S,
                grid_gb::DEMAND_MIN_MW,
                grid_gb::LOSS_MAX_MW,
            ),
            0.01,
        ),
    ];
    let mut seals = Vec::new();
    for request in cases {
        let (status, answer) = post(&base, "/verify_physics", &request).await;
        assert_eq!(status, 200, "{answer}");
        let response: VerifyPhysicsResponse = serde_json::from_str(&answer).unwrap();
        assert_eq!(response.inputs, request);
        let p = &request["parameters"];
        let number = |key: &str| p[key].as_f64().unwrap();
        let card = card();
        let mut state = GridState::try_start(
            &card,
            number("frequency_hz"),
            number("inertia_mw_s"),
            number("demand_mw"),
        )
        .unwrap();
        let mut frame = blank_frame(commit_request_proof(&request));
        grid_gb::step(
            &card,
            &mut state,
            &mut frame,
            GridLoad { loss_mw: number("loss_mw") },
            number("dt_s"),
        )
        .unwrap();
        assert_state_is(&response.state, &state, "step");
        assert_eq!(
            response.residual_conservation_error.to_bits(),
            state.residual_conservation_error.to_bits()
        );
        assert_eq!(response.sha256_seal, sha256_hex(&frame.to_bytes()));
        seals.push(response.sha256_seal);
    }
    seals.sort();
    seals.dedup();
    assert_eq!(seals.len(), 4, "four different steps, four different seals");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_of_sixty_seconds_is_the_kernel_loop_and_its_tape_and_receipt_hold() {
    let base = spawn_app().await;
    for case in [RANK_5, RANK_23] {
        // 1/64 s is exact in binary: 3,840 steps land on 60 s.
        let dt = 0.015625;
        let request = run_body(case, dt, 3_840, 64);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["outcome"], "completed", "{answer}");
        assert_eq!(tape["steps_landed"], 3_840);
        let (state, landed, reject) = direct_loop(case, dt, 3_840);
        assert_eq!((landed, reject), (3_840, None));
        assert_eq!(state.t_s, 60.0);
        assert_state_is(&tape["final_state"], &state, "run");
        // The fall is arrested: the lowest frequency is above 49.55 Hz and
        // below the start, and it came in the first ten seconds.
        assert!(
            state.lowest_frequency_hz > grid_gb::FREQUENCY_COMPARED_MIN_HZ
                && state.lowest_frequency_hz < case.0 - 0.2
        );
        assert!(state.lowest_time_s < 10.0, "{}", state.lowest_time_s);
        assert!(tape["final_state"].get("below_statutory_limit").is_none());

        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{report}");
        let (status, receipt) =
            post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{receipt}");
        let receipt: Value = serde_json::from_str(&receipt).unwrap();
        assert_eq!(receipt["matches"], true, "{receipt}");

        let (_, again) = post(&base, "/run_trajectory", &request).await;
        let again: Value = serde_json::from_str(&again).unwrap();
        assert_eq!(again["run_seal"], tape["run_seal"]);

        // One more step is past 60 s, the minute compared: the run goes on and
        // the step carries the flag.
        let request = run_body(case, dt, 3_841, 1);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["outcome"], "completed", "{answer}");
        assert_eq!(tape["steps_landed"], 3_841);
        assert_eq!(last_flags(&tape), grid_gb::FLAG_PAST_COMPARED_MINUTE);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_loss_is_stepped_and_flagged_and_a_changed_tape_breaks() {
    let base = spawn_app().await;
    let dt = 0.015625;
    let request = |first: f64, second: f64| {
        json!({
            "domain": "energy",
            "card": "grid_gb",
            "initial": { "frequency_hz": 50.0, "inertia_mw_s": 200_000.0, "demand_mw": 28_000.0 },
            "dt_s": dt,
            "segments": [
                { "steps": 640, "load": { "loss_mw": first } },
                { "steps": 640, "load": { "loss_mw": second } }
            ],
            "record_every": 64
        })
    };
    // One loss written as two segments is the run of that loss.
    let (status, answer) = post(&base, "/run_trajectory", &request(700.0, 700.0)).await;
    assert_eq!(status, 200, "{answer}");
    let mut tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed");
    let card = card();
    let mut state = GridState::try_start(&card, 50.0, 200_000.0, 28_000.0).unwrap();
    let mut frame = blank_frame([0; 16]);
    for _ in 0..1280 {
        grid_gb::step(&card, &mut state, &mut frame, GridLoad { loss_mw: 700.0 }, dt).unwrap();
    }
    assert_state_is(&tape["final_state"], &state, "two segments of one loss");
    tape["final_state"]["lowest_frequency_hz_hex"] = json!("4049000000000000");
    let (_, report) = post(&base, "/verify_tape", &tape).await;
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], false, "{report}");
    // A second loss, a smaller one after it, the link coming back and a quiet
    // period before the trip: each is stepped, the kernel loop to the bit,
    // and the last frame carries the flag.
    for (first, second) in [(700.0, 1400.0), (1000.0, 700.0), (1000.0, 0.0), (0.0, 1000.0)] {
        let (status, answer) = post(&base, "/run_trajectory", &request(first, second)).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["outcome"], "completed", "{answer}");
        let mut state = GridState::try_start(&card, 50.0, 200_000.0, 28_000.0).unwrap();
        for loss in [first, second] {
            for _ in 0..640 {
                grid_gb::step(&card, &mut state, &mut frame, GridLoad { loss_mw: loss }, dt).unwrap();
            }
        }
        assert_state_is(&tape["final_state"], &state, "two losses");
        assert_eq!(last_flags(&tape) & grid_gb::FLAG_OUTSIDE_COMPARED_TRIPS, grid_gb::FLAG_OUTSIDE_COMPARED_TRIPS);
        let (_, report) = post(&base, "/verify_tape", &tape).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{report}");
    }
}

/// The deepest corner of the compared spans was never compared. Its fall
/// passes under 49.55 Hz with the flag, the kernel loop to the bit, and its
/// tape holds. A loss of 4,000 MW on the same day falls to 48.8 Hz and is
/// refused by name there, sealed to the step before.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_fall_under_the_compared_frequencies_lands_flagged_until_disconnection() {
    let base = spawn_app().await;
    let case = (
        grid_gb::START_FREQUENCY_MIN_HZ,
        grid_gb::INERTIA_MIN_MW_S,
        grid_gb::DEMAND_MIN_MW,
        grid_gb::LOSS_MAX_MW,
    );
    let dt = 0.015625;
    let (state, landed, reject) = direct_loop(case, dt, 3_840);
    assert_eq!((landed, reject), (3_840, None));
    assert!(state.lowest_frequency_hz < grid_gb::FREQUENCY_COMPARED_MIN_HZ);
    let (status, answer) = post(&base, "/run_trajectory", &run_body(case, dt, 3_840, 3_840)).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{answer}");
    assert_state_is(&tape["final_state"], &state, "below the compared frequency");
    let (status, report) = post(&base, "/verify_tape", &tape).await;
    assert_eq!(status, 200, "{report}");
    let report: Value = serde_json::from_str(&report).unwrap();
    assert_eq!(report["holds"], true, "{report}");
    let deep = (case.0, case.1, case.2, 4_000.0);
    let (state, landed, reject) = direct_loop(deep, dt, 3_840);
    assert_eq!(reject, Some(StepReject::LowFrequencyDemandDisconnection));
    let (status, answer) = post(&base, "/run_trajectory", &run_body(deep, dt, 3_840, 3_840)).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "refused", "{answer}");
    assert_eq!(tape["reject"], "LowFrequencyDemandDisconnection");
    assert_eq!(tape["steps_landed"], landed);
    assert_state_is(&tape["final_state"], &state, "refused at disconnection");
}

/// Losses under 693 MW were never compared, and the six trips of 500 to
/// 692 MW missed three of the protocol's lines: each is answered, as a step
/// and as a run, with the flag. A loss larger in size than the demand is
/// refused by name.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_loss_under_the_compared_losses_is_answered_with_the_flag() {
    let base = spawn_app().await;
    for loss in [692.0, 634.0, 572.0, 545.0, 529.0, 500.0, 300.0, 1.0, 0.0, -500.0] {
        let case = (50.1, 241_000.0, 27_132.0, loss);
        let (status, step_text) = post(&base, "/verify_physics", &step_body(case, 0.015625)).await;
        assert_eq!(status, 200, "{loss}: {step_text}");
        let (status, run_text) = post(&base, "/run_trajectory", &run_body(case, 0.015625, 640, 640)).await;
        assert_eq!(status, 200, "{loss}: {run_text}");
        let tape: Value = serde_json::from_str(&run_text).unwrap();
        assert_eq!(tape["outcome"], "completed", "{loss}: {run_text}");
        assert_eq!(last_flags(&tape) & grid_gb::FLAG_OUTSIDE_COMPARED_TRIPS, grid_gb::FLAG_OUTSIDE_COMPARED_TRIPS);
    }
    let case = (50.1, 241_000.0, 27_132.0, 30_000.0);
    let (status, run_text) = post(&base, "/run_trajectory", &run_body(case, 0.015625, 10, 10)).await;
    assert_eq!(status, 200, "{run_text}");
    let tape: Value = serde_json::from_str(&run_text).unwrap();
    assert_eq!(tape["outcome"], "refused", "{run_text}");
    assert_eq!(tape["reject"], "LossLargerThanDemand");
    let case = (50.1, 241_000.0, 27_132.0, grid_gb::LOSS_MIN_MW);
    let (status, answer) = post(&base, "/run_trajectory", &run_body(case, 0.015625, 3_840, 3_840)).await;
    assert_eq!(status, 200, "{answer}");
    let tape: Value = serde_json::from_str(&answer).unwrap();
    assert_eq!(tape["outcome"], "completed", "{answer}");
    assert_eq!(last_flags(&tape), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_bound_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let (_, described) =
        post(&base, "/describe_card", &json!({"domain": "energy", "card": "grid_gb"})).await;
    let described: Value = serde_json::from_str(&described).unwrap();
    assert_eq!(described["phase_fields"].as_array().unwrap().len(), 3);
    assert_eq!(described["load_fields"].as_array().unwrap().len(), 1);
    assert_eq!(described["phase_fields"][0]["unit"], "Hz");
    assert_eq!(described["load_fields"][0]["unit"], "MW");
    assert_eq!(
        described["dt_s"]["ceiling_s"].as_f64().unwrap().to_bits(),
        ceiling().to_bits()
    );
    assert_eq!(described["dt_s"]["floor_s"].as_f64().unwrap(), 1.0e-6);

    let names = |text: &str, name: &str| {
        text.contains(&format!("({name})")) && !text.contains("sha256_seal")
    };
    let mid = (50.0, 200_000.0, 28_000.0, 1000.0);
    let cases: [((f64, f64, f64, f64), Option<&str>); 16] = [
        ((above(grid_gb::FREQUENCY_MIN_HZ), mid.1, mid.2, 0.0), None),
        ((grid_gb::FREQUENCY_MIN_HZ, mid.1, mid.2, mid.3), Some("LowFrequencyDemandDisconnection")),
        ((below(grid_gb::START_FREQUENCY_MIN_HZ), mid.1, mid.2, mid.3), None),
        ((grid_gb::FREQUENCY_MAX_HZ, mid.1, mid.2, mid.3), None),
        ((above(grid_gb::FREQUENCY_MAX_HZ), mid.1, mid.2, mid.3), Some("FrequencyAboveCard")),
        ((mid.0, below(grid_gb::INERTIA_MIN_MW_S), mid.2, mid.3), None),
        ((mid.0, above(grid_gb::INERTIA_MAX_MW_S), mid.2, mid.3), None),
        ((mid.0, 0.0, mid.2, mid.3), Some("InertiaNotPositive")),
        ((mid.0, -1.0, mid.2, mid.3), Some("InertiaNotPositive")),
        ((mid.0, mid.1, below(grid_gb::DEMAND_MIN_MW), mid.3), None),
        ((mid.0, mid.1, above(grid_gb::DEMAND_MAX_MW), mid.3), None),
        ((mid.0, mid.1, -1.0, mid.3), Some("DemandNegative")),
        ((mid.0, mid.1, mid.2, below(grid_gb::LOSS_MIN_MW)), None),
        ((mid.0, mid.1, mid.2, 0.0), None),
        ((mid.0, mid.1, mid.2, above(grid_gb::LOSS_MAX_MW)), None),
        ((mid.0, mid.1, mid.2, -1000.0), None),
    ];
    for (case, refusal) in cases {
        let dt = card().gate_dt_s_at(case.1, case.2).unwrap_or(ceiling()).min(ceiling());
        let (step_status, step_text) = post(&base, "/verify_physics", &step_body(case, dt)).await;
        let (run_status, run_text) = post(&base, "/run_trajectory", &run_body(case, dt, 5, 1)).await;
        match refusal {
            None => {
                assert_eq!(step_status, 200, "{case:?}: {step_text}");
                assert_eq!(run_status, 200, "{case:?}: {run_text}");
            }
            Some(name) => {
                for (status, text) in [(step_status, &step_text), (run_status, &run_text)] {
                    assert_eq!(status, 422, "{case:?}: {text}");
                    assert!(names(text, name), "{case:?}: {text}");
                }
            }
        }
    }
    // The step ceiling and floor are held by the gateway, and a run's own
    // ceiling follows its inertia and demand.
    for dt in [above(ceiling()), below(1.0e-6)] {
        let (status, text) = post(&base, "/verify_physics", &step_body(mid, dt)).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains("grid_gb card"), "{text}");
    }
    let small = (mid.0, 5_000.0, 81_658.0, 300.0);
    let small_ceiling = card().gate_dt_s_at(small.1, small.2).unwrap();
    assert!(small_ceiling < ceiling());
    let (status, text) = post(&base, "/verify_physics", &step_body(small, ceiling())).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("DtAboveCeiling"), "{text}");
    let (status, text) = post(&base, "/verify_physics", &step_body(small, small_ceiling)).await;
    assert_eq!(status, 200, "{text}");
    // A missing field and an unknown field are named; nothing is filled in.
    let mut missing = step_body(mid, ceiling());
    missing["parameters"].as_object_mut().unwrap().remove("demand_mw");
    let (status, text) = post(&base, "/verify_physics", &missing).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("demand_mw"), "{text}");
    let mut extra = step_body(mid, ceiling());
    extra["parameters"]["response_held_mw"] = json!(1000.0);
    let (status, text) = post(&base, "/verify_physics", &extra).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("response_held_mw"), "{text}");
}

/// The one line of the list and the validation record carry the sentences
/// held here.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_description_states_what_the_second_scoring_found() {
    let base = spawn_app().await;
    let cards: Value = Client::new().get(format!("{base}/cards")).send().await.unwrap().json().await.unwrap();
    let mine = cards["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["card"] == "grid_gb")
        .expect("listed");
    let specimen = mine["specimen"].as_str().unwrap();
    assert!(specimen.starts_with("The Great Britain synchronous area, the 50 Hz transmission system of England, Scotland and Wales as one rotating mass, after an importing interconnector trips. Compared on trips of 693 to 1402 MW over their first 60 s"), "{specimen}");
    assert!(specimen.contains("on the six compared trips since November 2025 its fall is 39 to 80 percent deeper than the measured fall"), "{specimen}");
    let (status, described) =
        post(&base, "/describe_card", &json!({"domain": "energy", "card": "grid_gb"})).await;
    assert_eq!(status, 200, "{described}");
    let described: Value = serde_json::from_str(&described).unwrap();
    let record = &described["validation"];
    let data = record["data"].as_str().unwrap();
    assert!(
        data.contains("a response of 2074 MW/Hz below 50 Hz, a further 3136 MW/Hz below the knee, and a lag of 1.296 s"),
        "{data}"
    );
    let result = record["result"].as_str().unwrap();
    for words in [
        "all seven lines were met on 12 trips",
        "That was two opposite losses at once, an export link and an import link, and not one trip",
        "root mean square relative error 0.166 and mean -0.018",
        "The held-back twelve were the easiest third of three",
        "the depth error is 0.258 and 0.326 with three lines missed each",
        "18.7 percent of random twelves of the forty meet all seven lines",
        "is 0.015: depth proportional to loss 0.182, a fall to one fixed frequency 0.180, a straight line 0.171, the card 0.166",
        "The comparison is not blind: the loss and the lowest frequency of every trip, the held-back ones included, were on screen once",
        "before the protocol was fingerprinted and the trips were split, and the depth line scores that lowest frequency",
        "A further 39 trips of the same kind",
        "36 were inside the card as served through engine 0.6.0 and were scored after the fact, and six of them, of 500 to 692 MW, are under the losses the card takes from engine 0.7.0",
        "All 36: depth 0.269 and near 60 s 0.067 Hz, two lines missed",
        "May 2018 to February 2025, 23 trips: depth 0.136, all lines met",
        "2024 to 2026, 15 trips: depth 0.345, six lines missed",
        "The six trips since November 2025 run 39 to 80 percent too deep",
        "SHA-256 da8ee131279452aa4f1f56afb71e801f622a050882554fb6d632dd23f9b7384c) was overwritten by a later build, which returned all 41 saved answers byte for byte. Since engine 0.7.0 the final state no longer carries below_statutory_limit, so the saved answers no longer come back byte for byte.",
        "evidence/verification/second_party/V-32/",
    ] {
        assert!(result.contains(words), "{words}: {result}");
    }
    let limits = record["limits"].as_str().unwrap();
    for words in [
        "0.337 on the opened 28 with the card's constants (0.35 with each trip left out of the fit in turn)",
        "moves by +0.066 a year (standard error 0.013), from -0.30 in 2018 to +0.17 in 2025, nearly twice the 0.25 mark",
        "Dynamic Containment from October 2020, 500 MW rising to 1,576 MW by July 2023",
        "The card has one gain and one lag for all of it",
        "0.9 or 1.2 times the published inertia misses the line on the mean of the steepest second",
        "The size of the loss is steady to 0.35 percent by the steadiness rule",
        "System Operability Framework 2016, p. 70, and the NESO webinar of 12 March 2025; the documented range is 2.0 to 2.5",
        "The 49.8 Hz knee is Dynamic Containment's (NESO, Response Services Service Terms v6; Dynamic Containment testing guidelines, August 2020)",
        "before it there were static blocks at 49.7 and 49.6 Hz",
        "a relief from 0 to 5 percent per hertz changes no verdict, and the pass holds for a knee of 49.80 to 49.90 Hz and is lost at 49.75",
        "Every compared trip was one loss: through engine 0.9.0 a later segment's other loss was refused as LossChangedDuringRun",
        "No changing loss was compared with a measurement",
        "failed against its criterion as first written, in 39 of 80 runs",
        "the response by at most 4.2e-10 MW, and the frequency was never above 0.05 of the bound",
        "The yardstick was changed afterwards by the card's author",
        "A run of exactly 60 s in decimal lands every step: 3,000 steps of 0.02 s land 3,000 and read 60.00000000000378 s.",
        "Known defect of the scan that found the trips: its rule of 0.15 Hz or more lost four falls of exactly 0.150 Hz to rounding.",
        "It was scored twice, the second time by a second scorer, on one machine",
        "The range was narrowed on 2026-10-10, after the scoring and with every trip seen, so the narrowing is not a new test",
        "missed three of the seven: depth 0.2512 against 0.25, and against 0.2509 for the proportional rule, steepest second 0.273 against 0.25",
        "A loss under 693 MW was refused as LossOutsideCard through engine 0.9.0; now it lands with FLAG_OUTSIDE_COMPARED_TRIPS",
        "now it lands with FLAG_BELOW_COMPARED_FREQUENCY, down to 48.8 Hz",
    ] {
        assert!(limits.contains(words), "{words}: {limits}");
    }
    let whole = described.to_string();
    for words in [
        "Compared on 693 to 1402 MW",
        "A landing under 49.55 Hz sets FLAG_BELOW_COMPARED_FREQUENCY",
        "the lowest measured single-loss frequency among the 41 trips, 49.553 Hz, rounded outward",
        "a step that lands later than 60 s sets FLAG_PAST_COMPARED_MINUTE",
    ] {
        assert!(whole.contains(words), "{words}: {whole}");
    }
    assert!(!whole.contains("sets FLAG_BELOW_STATUTORY_LIMIT and the run goes on"), "{whole}");
    assert!(!whole.contains("inclusive [0, 1402]"), "{whole}");
    assert!(!limits.contains("neither fitted nor sourced"), "{limits}");
}
