//! The JSI TRIGA Mark II pulse card on the web, the local and the hosted
//! connection.
//!
//! A run of one pulse with its scram is checked against the kernel loop on
//! each of the three rod-worth tables, every number of its last state to the
//! bit and every recorded frame byte for byte; its tape verifies and its
//! receipt matches. A scram inside the envelope compared with the records,
//! 1.50 to 7.45 s after the peak, lands at every time and every step tried.
//! Past every wall of 0.9.0 the run is answered and is the kernel loop's: a
//! start anywhere in liquid water on any table, a reactivity from a
//! thousandth of a dollar to the rod's whole worth, a scram at the fire
//! signal, while the rod moves, before the peak and long after it, the rods
//! held for a whole run of a million steps, and a run past 14 s. Each thing
//! the model does not define is refused by name: a reactivity that changes
//! after the start; rods raised after the scram; a scram value that is
//! neither 0 nor 1; a start that is not liquid water, a reactivity of zero or
//! less or above the rod's whole worth, and a rod table the card does not
//! hold. The three connections give the same bytes, and the hosted door
//! hides the card from a key that does not list it.
//!
//! No number a run produces is written here: every expected value comes from a
//! direct call into `genesis_kernel`.

mod answer_text;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use genesis_kernel::frame::LastStateFrame64;
use genesis_kernel::reactor_triga_jsi::{
    self, ReactorLoad, ReactorState, ReactorTrigaJsi, StepReject,
};
use protocol_mcp_gateway::app;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const CARD: &str = "reactor_triga_jsi";
/// The number of cards served, and this card's place among them.
const CARDS: usize = 37;
const PLACE: usize = 32;

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
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(answer_text::as_sent(body))
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

fn reactor() -> ReactorTrigaJsi {
    ReactorTrigaJsi::core_56_pulse_mode()
}

fn ceiling() -> f64 {
    reactor().gate_dt_s().unwrap()
}

fn load_json(rho: f64, dropped: f64) -> Value {
    json!({ "reactivity_inserted_dollar": rho, "rods_dropped": dropped })
}

fn run_body(start_k: f64, core: f64, dt_s: f64, segments: &[(u64, f64, f64)], record_every: u64) -> Value {
    let segments: Vec<Value> = segments
        .iter()
        .map(|(steps, rho, dropped)| json!({ "steps": steps, "load": load_json(*rho, *dropped) }))
        .collect();
    json!({
        "domain": "energy",
        "card": CARD,
        "initial": { "start_temperature_k": start_k, "rod_table_core": core },
        "dt_s": dt_s,
        "segments": segments,
        "record_every": record_every
    })
}

fn step_body(start_k: f64, core: f64, rho: f64, dropped: f64, dt_s: f64) -> Value {
    json!({
        "domain": "energy",
        "card": CARD,
        "parameters": {
            "start_temperature_k": start_k,
            "rod_table_core": core,
            "reactivity_inserted_dollar": rho,
            "rods_dropped": dropped,
            "dt_s": dt_s
        }
    })
}

struct Direct {
    state: ReactorState,
    frame: LastStateFrame64,
    landed: u64,
    reject: Option<StepReject>,
    kept: Vec<(u64, LastStateFrame64)>,
    flags_seen: u64,
    largest_residual: f64,
}

/// The kernel loop over the same segments.
fn direct_loop(start_k: f64, core: f64, dt_s: f64, segments: &[(u64, f64, f64)], record_every: u64) -> Direct {
    let reactor = reactor();
    let mut state = ReactorState::try_start(&reactor, start_k, core).unwrap();
    let mut frame = LastStateFrame64 {
        t: 0.0,
        pos: [0.0; 3],
        vel: [0.0; 3],
        force_torque: 0.0,
        residual: 0.0,
        flags: 0,
        proof: [0; 16],
    };
    let mut out = Direct {
        state,
        frame,
        landed: 0,
        reject: None,
        kept: Vec::new(),
        flags_seen: 0,
        largest_residual: 0.0,
    };
    'run: for (steps, rho, dropped) in segments {
        let load = ReactorLoad { reactivity_inserted_dollar: *rho, rods_dropped: *dropped };
        for _ in 0..*steps {
            match reactor_triga_jsi::step(&reactor, &mut state, &mut frame, load, dt_s) {
                Ok(()) => {
                    out.landed += 1;
                    out.flags_seen |= frame.flags;
                    out.largest_residual = out.largest_residual.max(state.residual_conservation_error);
                    if out.landed % record_every == 0 {
                        out.kept.push((out.landed, frame));
                    }
                }
                Err(reject) => {
                    out.reject = Some(reject);
                    break 'run;
                }
            }
        }
    }
    out.state = state;
    out.frame = frame;
    out
}

fn published(state: &Value, name: &str) -> f64 {
    let hex = state[format!("{name}_hex")]
        .as_str()
        .unwrap_or_else(|| panic!("{name}_hex in {state}"));
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

/// Every published number of `published_state` is the kernel's, to the bit,
/// the three booleans are the kernel's, and nothing else is published.
fn assert_state_is(published_state: &Value, state: &ReactorState, label: &str) {
    // Every field is named, so a field added to the card does not compile
    // here until it is published.
    let ReactorState {
        t_s,
        power_mw,
        precursor_mw,
        fuel_temperature_k,
        energy_mws,
        start_temperature_k,
        rod_table_core,
        reactivity_load_dollar,
        rods_dropped,
        scram_time_s,
        energy_at_scram_mws,
        power_rate_mw_per_s,
        reactivity_net_dollar,
        reactivity_inserted_now_dollar,
        population_anchor_mw,
        population_supplied_mw,
        population_defect_mw,
        residual_conservation_error,
        peak_power_mw,
        peak_energy_mws,
        peak_time_s,
        peak_fuel_temperature_k,
        prompt_supercritical,
        rod_moving,
    } = *state;
    let numbers = [
        ("t_s", t_s),
        ("power_mw", power_mw),
        ("energy_mws", energy_mws),
        ("power_rate_mw_per_s", power_rate_mw_per_s),
        ("fuel_temperature_k", fuel_temperature_k),
        ("reactivity_net_dollar", reactivity_net_dollar),
        ("reactivity_inserted_now_dollar", reactivity_inserted_now_dollar),
        ("start_temperature_k", start_temperature_k),
        ("rod_table_core", rod_table_core),
        ("reactivity_load_dollar", reactivity_load_dollar),
        ("scram_time_s", scram_time_s),
        ("energy_at_scram_mws", energy_at_scram_mws),
        ("precursor_1_mw", precursor_mw[0]),
        ("precursor_2_mw", precursor_mw[1]),
        ("precursor_3_mw", precursor_mw[2]),
        ("precursor_4_mw", precursor_mw[3]),
        ("precursor_5_mw", precursor_mw[4]),
        ("precursor_6_mw", precursor_mw[5]),
        ("population_anchor_mw", population_anchor_mw),
        ("population_supplied_mw", population_supplied_mw),
        ("population_defect_mw", population_defect_mw),
        ("residual_conservation_error", residual_conservation_error),
        ("peak_power_mw", peak_power_mw),
        ("peak_energy_mws", peak_energy_mws),
        ("peak_time_s", peak_time_s),
        ("peak_fuel_temperature_k", peak_fuel_temperature_k),
    ];
    let object = published_state.as_object().unwrap();
    let mut keys = std::collections::BTreeSet::new();
    for (name, value) in numbers {
        let decimal = object[name].as_f64().unwrap_or_else(|| panic!("{label} {name}"));
        assert_eq!(decimal.to_bits(), value.to_bits(), "{label} {name}");
        assert_eq!(published(published_state, name).to_bits(), value.to_bits(), "{label} {name}");
        keys.insert(name.to_string());
        keys.insert(format!("{name}_hex"));
    }
    for (name, value) in [
        ("prompt_supercritical", prompt_supercritical),
        ("rod_moving", rod_moving),
        ("rods_dropped", rods_dropped),
    ] {
        assert_eq!(object[name], value, "{label} {name}");
        keys.insert(name.to_string());
    }
    let found: std::collections::BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(found, keys, "{label}: the published names drifted from the card");
    // No thermocouple is published: it is not on the card.
    assert!(!found.iter().any(|name| name.contains("thermocouple")), "{label}");
}

fn recorded(frame: &Value) -> (u64, Vec<u8>, Vec<u8>) {
    let bytes = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap();
    assert_eq!(bytes.len(), 64);
    let link = hex::decode(frame["link"].as_str().unwrap()).unwrap();
    (frame["step"].as_u64().unwrap(), bytes, link)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pulse_with_its_scram_is_the_kernel_loop_on_each_rod_table_and_keeps_its_proof() {
    let base = spawn_app().await;
    let mut peaks = Vec::new();
    for (start_k, core, rho) in [
        (reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE[0].0, 215.0, reactor_triga_jsi::REACTIVITY_COMPARED_MIN_DOLLAR),
        (298.15, 217.0, 2.5),
        (298.15, 215.0, 2.5),
        (295.15, 231.0, 2.5),
        (reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE[2].1, 231.0, reactor_triga_jsi::REACTIVITY_COMPARED_MAX_DOLLAR),
    ] {
        let label = format!("{start_k} K, table {core}, {rho} dollar");
        // 2.3 s of pulse and tail, then 0.6 s after the scram.
        let segments = [(25_000, rho, 0.0), (6_000, rho, 1.0)];
        let record_every = 500;
        let request = run_body(start_k, core, ceiling(), &segments, record_every);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{label}: {answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        let direct = direct_loop(start_k, core, ceiling(), &segments, record_every);
        assert_eq!(direct.reject, None, "{label}");
        assert_eq!(tape["outcome"], "completed", "{label}: {}", tape["reject_detail"]);
        assert_eq!(tape["steps_landed"], direct.landed, "{label}");
        assert_state_is(&tape["final_state"], &direct.state, &label);
        assert!(direct.state.rods_dropped && !direct.state.rod_moving, "{label}");
        // the scram time and the energy at the scram are those of the last step of the first segment
        let before = direct_loop(start_k, core, ceiling(), &segments[..1], record_every);
        assert_eq!(direct.state.scram_time_s.to_bits(), before.state.t_s.to_bits(), "{label}");
        assert_eq!(direct.state.energy_at_scram_mws.to_bits(), before.state.energy_mws.to_bits(), "{label}");
        assert!(direct.state.energy_mws > direct.state.energy_at_scram_mws, "{label}");
        // The pulse happened, the fuel is hot, and all three flags were seen.
        assert!(direct.state.peak_power_mw > 1.0, "{label}");
        assert!(direct.state.peak_time_s < direct.state.scram_time_s, "{label}");
        assert!(direct.state.fuel_temperature_k > start_k + 40.0, "{label}");
        assert!(direct.state.peak_fuel_temperature_k >= direct.state.fuel_temperature_k, "{label}");
        assert_eq!(
            direct.flags_seen,
            reactor_triga_jsi::FLAG_PROMPT_SUPERCRITICAL
                | reactor_triga_jsi::FLAG_ROD_MOVING
                | reactor_triga_jsi::FLAG_RODS_DROPPED,
            "{label}"
        );
        assert_eq!(
            published(&tape, "max_residual_conservation_error").to_bits(),
            direct.largest_residual.to_bits(),
            "{label}"
        );
        assert!(direct.largest_residual <= reactor_triga_jsi::RESIDUAL_CONSERVATION_LIMIT / 10.0);
        if rho == 2.5 {
            peaks.push(direct.state.peak_power_mw);
        }

        // The recorded frames are the kernel's, each with the link before it as its proof.
        let frames = tape["frames"].as_array().unwrap();
        assert_eq!(frames.len(), direct.kept.len(), "{label}");
        let mut link_before: Option<Vec<u8>> = None;
        for (frame, (step, kernel_frame)) in frames.iter().zip(&direct.kept) {
            let (at, bytes, link) = recorded(frame);
            assert_eq!(at, *step, "{label}");
            assert_eq!(bytes[..48], kernel_frame.to_bytes()[..48], "{label} step {step}");
            if let Some(before) = &link_before {
                assert_eq!(bytes[48..], before[..16], "{label} step {step}: the proof");
            }
            assert_eq!(link, Sha256::digest(&bytes).to_vec(), "{label} step {step}: the link");
            link_before = Some(link);
        }
        assert!(tape["run_seal"].as_str().is_some_and(|seal| seal.len() == 64), "{label}");

        let (status, report) = post(&base, "/verify_tape", &tape).await;
        assert_eq!(status, 200, "{report}");
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{label}: {report}");
        let (status, receipt) = post(&base, "/verify_receipt", &json!({ "response_text": answer })).await;
        assert_eq!(status, 200, "{receipt}");
        let receipt: Value = serde_json::from_str(&receipt).unwrap();
        assert_eq!(receipt["matches"], true, "{label}: {receipt}");

        // A changed byte in a recorded frame breaks the tape.
        let mut forged = tape.clone();
        let (_, mut bytes, _) = recorded(&frames[3]);
        bytes[9] ^= 1;
        forged["frames"][3]["frame_hex"] = json!(hex::encode(bytes));
        let (_, report) = post(&base, "/verify_tape", &forged).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], false, "{label}: {report}");
    }
    // The same printed 2.5 dollars is three pulses on the three tables (217, 215, 231 in the order run).
    assert!(peaks[0] < peaks[1] && peaks[0] < peaks[2], "{peaks:?}");
    assert!(peaks[1] != peaks[2], "{peaks:?}");
}

/// A scram after the rod had stopped was once refused under a false name
/// near the peak; those scrams are now refused as earlier than any compared.
/// Through the door, every scram from 1.50 to 7.45 s after the card's own
/// peak lands and its tape holds, at every step tried, on the corners of the
/// span.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_scram_inside_the_compared_envelope_lands_at_every_step() {
    let base = spawn_app().await;
    let reactor = reactor();
    let mut probes = 0;
    let mut largest: f64 = 0.0;
    // each start at an end of its own table's span
    let [t215, _, t231] = reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE;
    let (first, last) = reactor_triga_jsi::SCRAM_AFTER_PEAK_COMPARED_S;
    for (start_k, core, rho) in [(t231.0, 231.0, 3.0), (298.15, 217.0, 2.0), (t215.1, 215.0, 2.5), (t231.1, 231.0, 2.0)] {
        for dt_s in [ceiling(), 5.0e-5, 2.0e-5] {
            // the card's own peak time at this step, from the kernel
            let free = direct_loop(start_k, core, dt_s, &[((0.5 / dt_s) as u64, rho, 0.0)], 1_000_000);
            let peak = free.state.peak_time_s;
            let peak_steps = (peak / dt_s).round() as u64;
            // held to 7.45 s after the peak, 3.0 dollars passes 600 K on core 231, where 0.9.0 refused
            for after in [first, 2.0, 3.5, 5.0, 7.0, last] {
                // the held steps whose clock reaches the scram instant, inside the envelope
                let steps_after = if after == first {
                    (after / dt_s).ceil()
                } else if after == last {
                    (after / dt_s).floor()
                } else {
                    (after / dt_s).round()
                };
                let before = peak_steps + steps_after as u64;
                let tail = (0.03 / dt_s) as u64;
                let segments = [(before, rho, 0.0), (tail, rho, 1.0)];
                let label = format!("{start_k} K, table {core}, {rho} dollar, scram {after} s after the peak, step {dt_s}");
                let (status, answer) = post(&base, "/run_trajectory", &run_body(start_k, core, dt_s, &segments, before + tail)).await;
                assert_eq!(status, 200, "{label}: {answer}");
                let tape: Value = serde_json::from_str(&answer).unwrap();
                assert_eq!(tape["outcome"], "completed", "{label}: {} {}", tape["reject"], tape["reject_detail"]);
                assert_eq!(tape["steps_landed"], before + tail, "{label}");
                assert_eq!(tape["final_state"]["rods_dropped"], true, "{label}");
                let direct = direct_loop(start_k, core, dt_s, &segments, before + tail);
                assert_eq!(direct.reject, None, "{label}");
                assert_state_is(&tape["final_state"], &direct.state, &label);
                let gap = direct.state.scram_time_s - direct.state.peak_time_s;
                assert!((gap - after).abs() < dt_s, "{label}: {gap}");
                assert!(gap >= first - 1.0e-9 && gap <= last + 1.0e-9, "{label}: {gap}");
                largest = largest.max(direct.largest_residual);
                probes += 1;
            }
        }
    }
    assert!(probes >= 72, "{probes}");
    assert!(largest < reactor_triga_jsi::RESIDUAL_CONSERVATION_LIMIT / 10.0, "{largest:e}");
}

/// A run whose later segment asks for what the model does not define lands its
/// first segment, is refused by name at the first step of the later one, and
/// keeps its proof to the last landed step. A run that would change its
/// reactivity is refused whole, before its first step, with nothing sealed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn what_the_model_does_not_define_is_refused_by_name() {
    let base = spawn_app().await;
    let dt = ceiling();
    let rho = 2.0;
    let stop_steps = (reactor().rod_stop_s(rho) / dt).ceil() as u64;
    let moving = stop_steps / 2;
    let stopped = stop_steps + 10;
    // the card's own peak at this step, and the steps to two seconds after it
    let peak = direct_loop(298.15, 217.0, dt, &[(5_000, rho, 0.0)], 5_000).state.peak_time_s;
    let peak_steps = (peak / dt).round() as u64;
    let late = peak_steps + (2.0 / dt) as u64;
    for (segments, expected, words) in [
        // rods raised after the scram, after a compared scram and after one while the rod moved
        (vec![(late, rho, 0.0), (10, rho, 1.0), (10, rho, 0.0)], StepReject::RodsRaisedAfterScram, "raised after the scram"),
        (vec![(moving, rho, 0.0), (10, rho, 1.0), (10, rho, 0.0)], StepReject::RodsRaisedAfterScram, "raised after the scram"),
    ] {
        let label = format!("{segments:?}");
        let request = run_body(298.15, 217.0, dt, &segments, 100);
        let (status, answer) = post(&base, "/run_trajectory", &request).await;
        assert_eq!(status, 200, "{label}: {answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        let direct = direct_loop(298.15, 217.0, dt, &segments, 100);
        assert_eq!(direct.reject, Some(expected), "{label}");
        let landed: u64 = segments[..segments.len() - 1].iter().map(|s| s.0).sum();
        assert_eq!(direct.landed, landed, "{label}");
        assert_eq!(tape["outcome"], "refused", "{label}");
        assert_eq!(tape["reject"], format!("{expected:?}"), "{label}");
        assert!(tape["reject_detail"].as_str().unwrap().contains(words), "{label}: {}", tape["reject_detail"]);
        assert!(tape["reject_detail"].as_str().unwrap().ends_with(&format!("The run is sealed to step {landed}. The state at step {} was refused and is not sealed.", landed + 1)), "{label}");
        assert_eq!(tape["steps_landed"], landed, "{label}");
        assert_state_is(&tape["final_state"], &direct.state, &label);
        let (_, report) = post(&base, "/verify_tape", &tape).await;
        let report: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(report["holds"], true, "{label}: {report}");
    }
    // A reactivity that changes: a second insertion after the rod has stopped, a withdrawal,
    // a change while the rod is moving by one unit in the last place, a change hidden in the
    // scram segment. Each is refused before the first step; the kernel's own step refuses the
    // same change by the same name.
    for (segments, other) in [
        (vec![(stopped, rho, 0.0), (10, 2.5, 0.0)], 2.5),
        (vec![(stopped, rho, 0.0), (10, 1.5, 0.0)], 1.5),
        (vec![(moving, rho, 0.0), (10, above(rho), 0.0)], above(rho)),
        (vec![(stopped, rho, 0.0), (10, 3.0, 1.0)], 3.0),
    ] {
        let label = format!("{segments:?}");
        let (status, text) = post(&base, "/run_trajectory", &run_body(298.15, 217.0, dt, &segments, 1)).await;
        assert_eq!(status, 422, "{label}: {text}");
        assert_eq!(
            text,
            format!(
                "Physical Failure: 'reactivity_inserted_dollar' {other} dollar is not the {rho} dollar of the first segment: a run of the reactor_triga_jsi card is one pulse with one inserted reactivity, and the model defines no change of it but the scram (ReactivityChangedDuringRun). That state is not sealed."
            ),
            "{label}"
        );
        assert!(!text.contains("run_seal") && !text.contains("frame_hex"), "{label}");
        let direct = direct_loop(298.15, 217.0, dt, &segments, 1);
        assert_eq!(direct.reject, Some(StepReject::ReactivityChangedDuringRun), "{label}");
    }
    // On a single step: a scram value between 0 and 1.
    let (status, text) = post(&base, "/verify_physics", &step_body(298.15, 217.0, rho, 0.5, dt)).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("ScramValueOutsideCard"), "{text}");
}

/// Past every wall of 0.9.0 the run is answered, and it is the kernel loop's
/// to the bit with its tape holding: a scram at the fire signal, while the rod
/// moves, after the rod has stopped and before the peak, and a second after
/// it (0.9.0: ScramBeforeRodStopped and ScramBeforeCompared); a 3.0 dollar
/// pulse held past 600 K and 7.45 s after its peak (FuelTemperatureBeyondCard
/// and RodsHeldBeyondCompared); the rod's whole worth from either end of
/// liquid water, held for a run of a million steps, past 1000 MW, 20 MW s and
/// 14 s (PowerBeyondCard, EnergyBeyondCard and TimeBeyondCard); a start on a
/// table outside the starts compared on it and a reactivity below 2.0
/// dollars (StartTemperatureOutsideCard and ReactivityOutsideCard).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn past_every_wall_of_the_compared_span_the_run_is_answered() {
    let base = spawn_app().await;
    let dt = ceiling();
    let rho = 2.0;
    let stop_steps = (reactor().rod_stop_s(rho) / dt).ceil() as u64;
    let peak = direct_loop(298.15, 217.0, dt, &[(5_000, rho, 0.0)], 5_000).state.peak_time_s;
    let peak_steps = (peak / dt).round() as u64;
    let answered = |start_k: f64, core: f64, segments: Vec<(u64, f64, f64)>, record_every: u64| {
        let base = base.clone();
        async move {
            let label = format!("{start_k} K, table {core}, {segments:?}");
            let (status, answer) = post(&base, "/run_trajectory", &run_body(start_k, core, dt, &segments, record_every)).await;
            assert_eq!(status, 200, "{label}: {answer}");
            let tape: Value = serde_json::from_str(&answer).unwrap();
            assert_eq!(tape["outcome"], "completed", "{label}: {} {}", tape["reject"], tape["reject_detail"]);
            let direct = direct_loop(start_k, core, dt, &segments, record_every);
            assert_eq!(direct.reject, None, "{label}");
            assert_eq!(tape["steps_landed"], direct.landed, "{label}");
            assert_state_is(&tape["final_state"], &direct.state, &label);
            assert!(direct.largest_residual < reactor_triga_jsi::RESIDUAL_CONSERVATION_LIMIT, "{label}");
            let (_, report) = post(&base, "/verify_tape", &tape).await;
            assert_eq!(serde_json::from_str::<Value>(&report).unwrap()["holds"], true, "{label}");
            direct
        }
    };
    // the scram at the fire signal, while the rod moves, after it has stopped and before the
    // peak, and a second after the peak
    for (held, label) in [(0, "at the fire signal"), (stop_steps / 2, "while the rod moves"), (stop_steps + 10, "before the peak"), (peak_steps + (1.0 / dt) as u64, "a second after the peak")] {
        let mut segments = vec![(20_000, rho, 1.0)];
        if held > 0 {
            segments.insert(0, (held, rho, 0.0));
        }
        let direct = answered(298.15, 217.0, segments, 1_000).await;
        assert!(direct.state.rods_dropped, "{label}");
        assert!(direct.flags_seen & reactor_triga_jsi::FLAG_RODS_DROPPED != 0, "{label}");
    }
    // a 3.0 dollar pulse held: past 600 K and 7.45 s after its peak, to 56 s
    let direct = answered(reactor_triga_jsi::START_TEMPERATURE_K_BY_TABLE[2].1, 231.0, vec![(600_000, 3.0, 0.0)], 100_000).await;
    assert!(direct.state.peak_fuel_temperature_k > 600.0);
    assert!(direct.state.t_s - direct.state.peak_time_s > 7.0 * reactor_triga_jsi::SCRAM_AFTER_PEAK_COMPARED_S.1);
    // the rod's whole worth from the warm end of liquid water on core 231, held for a whole
    // run: the hottest fuel any run reaches, far below the fuel's limit
    let direct = answered(reactor_triga_jsi::WATER_BOILING_K, 231.0, vec![(999_999, reactor_triga_jsi::ROD_WORTH_DOLLAR, 0.0)], 250_000).await;
    assert!((direct.state.peak_fuel_temperature_k - 769.74).abs() < 0.01, "{}", direct.state.peak_fuel_temperature_k);
    assert!(direct.state.peak_power_mw > 2.5 * reactor_triga_jsi::POWER_COMPARED_MAX_MW);
    assert!(direct.state.energy_mws > 2.0 * reactor_triga_jsi::ENERGY_COMPARED_MAX_MWS);
    assert!(direct.state.t_s > 6.0 * reactor_triga_jsi::TIME_COMPARED_MAX_S);
    // and from ice-cold water on core 215, the rods dropped while the rod moves
    let stop_whole = (reactor().rod_stop_s(reactor_triga_jsi::ROD_WORTH_DOLLAR) / dt) as u64;
    answered(reactor_triga_jsi::WATER_FREEZING_K, 215.0, vec![(stop_whole - 5, reactor_triga_jsi::ROD_WORTH_DOLLAR, 0.0), (150_000, reactor_triga_jsi::ROD_WORTH_DOLLAR, 1.0)], 50_000).await;
    // a start compared on another table, and reactivities below 2.0 dollars
    for (start_k, core, rho) in [(302.0, 215.0, 2.0), (294.2, 231.0, 3.0), (350.0, 217.0, 1.4), (298.15, 217.0, 0.5), (298.15, 217.0, 1.0e-3)] {
        answered(start_k, core, vec![(30_000, rho, 0.0), (10_000, rho, 1.0)], 10_000).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_end_of_the_card_lands_at_the_last_number_inside_and_is_refused_by_name_at_the_next_outside() {
    let base = spawn_app().await;
    let reactor = reactor();
    let dt = ceiling();
    let (t_min, t_max) = (reactor_triga_jsi::WATER_FREEZING_K, reactor_triga_jsi::WATER_BOILING_K);
    let r_max = reactor_triga_jsi::ROD_WORTH_DOLLAR;
    assert_eq!((t_min, t_max, r_max), (273.15, 373.124, 4.16));
    for core in reactor_triga_jsi::ROD_TABLE_CORE {
        for (start_k, rho) in [(t_min, r_max), (t_max, 1.0e-300), (t_min, 1.0e-3), (t_max, r_max), (296.0, below(2.0)), (302.0, above(3.0))] {
            let (status, text) = post(&base, "/verify_physics", &step_body(start_k, core, rho, 0.0, dt)).await;
            assert_eq!(status, 200, "{start_k} {core} {rho}: {text}");
            // and with the rods dropped at the fire signal
            let (status, text) = post(&base, "/verify_physics", &step_body(start_k, core, rho, 1.0, dt)).await;
            assert_eq!(status, 200, "{start_k} {core} {rho}: {text}");
        }
    }
    for (start_k, core, rho, dropped, name, field) in [
        // ice and boiling water
        (below(t_min), 217.0, 2.0, 0.0, "StartTemperatureNotLiquidWater", "start_temperature_k"),
        (above(t_max), 217.0, 2.0, 0.0, "StartTemperatureNotLiquidWater", "start_temperature_k"),
        (263.15, 231.0, 2.0, 0.0, "StartTemperatureNotLiquidWater", "start_temperature_k"),
        (373.15, 215.0, 3.0, 0.0, "StartTemperatureNotLiquidWater", "start_temperature_k"),
        // no reactivity inserted, and more than the rod holds
        (298.15, 217.0, 0.0, 0.0, "ReactivityNotPositive", "reactivity_inserted_dollar"),
        (298.15, 217.0, -1.0, 0.0, "ReactivityNotPositive", "reactivity_inserted_dollar"),
        (298.15, 217.0, -f64::MIN_POSITIVE, 1.0, "ReactivityNotPositive", "reactivity_inserted_dollar"),
        (298.15, 217.0, above(r_max), 0.0, "ReactivityAboveRodWorth", "reactivity_inserted_dollar"),
        (298.15, 231.0, 5.0, 1.0, "ReactivityAboveRodWorth", "reactivity_inserted_dollar"),
        (298.15, 217.0, 2.0, above(1.0), "ScramValueOutsideCard", "rods_dropped"),
        (298.15, 217.0, 2.0, -1.0, "ScramValueOutsideCard", "rods_dropped"),
        // outside the published span of the rod table
        (298.15, below(215.0), 2.0, 0.0, "RodTableOutsideCard", "rod_table_core"),
        (298.15, above(231.0), 2.0, 0.0, "RodTableOutsideCard", "rod_table_core"),
        (298.15, 253.0, 2.0, 0.0, "RodTableOutsideCard", "rod_table_core"),
        // inside the span and not a table: a core between two, and one unit in the last place off
        (298.15, 216.0, 2.0, 0.0, "RodTableOutsideCard", "rod_table_core"),
        (298.15, 224.0, 2.0, 0.0, "RodTableOutsideCard", "rod_table_core"),
        (298.15, above(215.0), 2.0, 0.0, "RodTableOutsideCard", "rod_table_core"),
        (298.15, below(231.0), 2.0, 0.0, "RodTableOutsideCard", "rod_table_core"),
        (298.15, 217.5, 2.0, 0.0, "RodTableOutsideCard", "rod_table_core"),
    ] {
        let (status, on_step) = post(&base, "/verify_physics", &step_body(start_k, core, rho, dropped, dt)).await;
        assert_eq!(status, 422, "{name}: {on_step}");
        assert!(on_step.contains(&format!("({name})")), "{on_step}");
        assert!(on_step.contains(&format!("'{field}'")), "{on_step}");
        assert!(!on_step.contains("frame_seal"), "{on_step}");
        // The same request as a run is refused before its first step, with the same sentence.
        let (status, on_run) = post(&base, "/run_trajectory", &run_body(start_k, core, dt, &[(3, rho, dropped)], 1)).await;
        assert_eq!(status, 422, "{name}: {on_run}");
        assert_eq!(on_run, on_step, "{name}");
    }
    // A request without the rod table is refused: nothing defaults.
    let mut without = step_body(298.15, 217.0, 2.0, 0.0, dt);
    without["parameters"].as_object_mut().unwrap().remove("rod_table_core");
    let (status, text) = post(&base, "/verify_physics", &without).await;
    assert_eq!(status, 422, "{text}");
    assert!(text.contains("rod_table_core"), "{text}");
    // The clock: the number over the ceiling and the number under the floor. The ceiling that
    // follows the state is the step's own and no request at or under this ceiling reaches it.
    let floor = reactor.floor_dt_s().unwrap();
    for (dt_s, word) in [(above(dt), "step ceiling"), (below(floor), "step floor")] {
        let (status, text) = post(&base, "/verify_physics", &step_body(298.15, 217.0, 2.0, 0.0, dt_s)).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains(word) && text.contains("reactor_triga_jsi"), "{text}");
        assert!(!text.contains("DtAboveStateCeiling"), "{text}");
    }
    for dt_s in [dt, floor] {
        let (status, text) = post(&base, "/verify_physics", &step_body(298.15, 217.0, 2.0, 0.0, dt_s)).await;
        assert_eq!(status, 200, "{text}");
    }
    // A constant of the card, or a state of the card, sent as a field is refused.
    for field in ["dollar_scale", "generation_time_s", "fuel_temperature_k", "energy_mws", "peak_power_mw", "thermocouple_temperature_k"] {
        let mut forged = step_body(298.15, 217.0, 2.0, 0.0, dt);
        forged["parameters"][field] = json!(1.0);
        let (status, text) = post(&base, "/verify_physics", &forged).await;
        assert_eq!(status, 422, "{text}");
        assert!(text.contains(field), "{text}");
    }
    // Past the 14 s compared: 200,000 steps of 7e-5 s make 14 s and all land, and the run goes
    // on, to three times that, as the kernel loop does.
    for steps in [160_000, 160_001, 560_000] {
        let segments = [(40_000, 2.0, 0.0), (steps, 2.0, 1.0)];
        let (status, answer) = post(&base, "/run_trajectory", &run_body(298.15, 217.0, 7.0e-5, &segments, 50_000)).await;
        assert_eq!(status, 200, "{answer}");
        let tape: Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(tape["outcome"], "completed", "{}", tape["reject_detail"]);
        assert_eq!(tape["steps_landed"], 40_000 + steps);
        let direct = direct_loop(298.15, 217.0, 7.0e-5, &segments, 50_000);
        assert_state_is(&tape["final_state"], &direct.state, &format!("{steps}"));
    }
}

// --------------------------------------------------------- the three connections

const KEY_ALL: &str = "key-for-the-client-that-sees-everything";
const KEY_TRIGA: &str = "key-for-the-triga-reactor-client";
const KEY_THERMAL: &str = "key-for-the-thermal-client";

fn client_entry(label: &str, key: &str, cards: Value) -> Value {
    json!({ "client": label, "key_sha256": sha256_hex(key.as_bytes()), "cards": cards, "solvers": [], "max_in_flight": 8, "max_permits": 1 })
}

struct StdioDoor {
    _child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl StdioDoor {
    async fn start() -> StdioDoor {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mcp_stdio"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut door = StdioDoor { _child: child, stdin, out };
        let init = json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
        door.raw(&init.to_string()).await;
        door.stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .await
            .unwrap();
        door
    }

    async fn raw(&mut self, line: &str) -> String {
        self.stdin.write_all(line.as_bytes()).await.unwrap();
        self.stdin.write_all(b"\n").await.unwrap();
        self.stdin.flush().await.unwrap();
        let mut answer = String::new();
        self.out.read_line(&mut answer).await.unwrap();
        answer.trim_end_matches('\n').to_string()
    }

    async fn call(&mut self, tool: &str, arguments: &Value) -> String {
        self.raw(&answer_text::tool_call(7, tool, arguments).to_string())
            .await
    }
}

struct Hosted {
    _child: Child,
    base: String,
    _dir: PathBuf,
}

impl Hosted {
    async fn start(keys: &Value) -> Hosted {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("reactor_triga_hosted_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("keys.json");
        std::fs::write(&file, serde_json::to_string_pretty(keys).unwrap()).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .arg("--port")
            .arg("0")
            .arg("--keys")
            .arg(&file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        out.read_line(&mut line).await.unwrap();
        let address = line
            .split("running on ")
            .nth(1)
            .unwrap_or_else(|| panic!("no address in {line:?}"))
            .split(' ')
            .next()
            .unwrap()
            .to_string();
        tokio::spawn(async move {
            let mut rest = String::new();
            while out.read_line(&mut rest).await.unwrap_or(0) > 0 {
                rest.clear();
            }
        });
        Hosted { _child: child, base: format!("http://{address}"), _dir: dir }
    }

    async fn tool(&self, key: &str, tool: &str, arguments: &Value) -> String {
        let body = answer_text::tool_call(7, tool, arguments);
        let response = reqwest::Client::new()
            .post(format!("{}/mcp", self.base))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2025-06-18")
            .header("Authorization", format!("Bearer {key}"))
            .body(body.to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        response.text().await.unwrap()
    }
}

fn result_text(answer: &str) -> String {
    let value: Value = serde_json::from_str(answer).unwrap();
    let text = value["result"]["content"][0]["text"].as_str().unwrap_or_else(|| panic!("no text in {answer}")).to_string();
    answer_text::remember(&text);
    text
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_three_connections_give_the_same_bytes_and_the_hosted_door_hides_the_card_from_a_key_without_it() {
    let base = spawn_app().await;
    let mut stdio = StdioDoor::start().await;
    let listed = result_text(&stdio.call("list_cards", &json!({})).await);
    let listed: Value = serde_json::from_str(&listed).unwrap();
    let cards = listed["cards"].as_array().unwrap();
    assert_eq!(cards.len(), CARDS);
    assert_eq!(cards[PLACE]["domain"], "energy");
    assert_eq!(cards[PLACE]["card"], CARD);
    // The one line an agent sees in the list carries the standing.
    let line = cards[PLACE]["specimen"].as_str().unwrap();
    for words in [
        "Ten constants are fitted to 36 pulses of this reactor",
        "added after the twelve fixed beforehand had failed",
        "one thermocouple of one element",
        "the comparison with the 20 rows held back failed",
    ] {
        assert!(line.contains(words), "the list's line lacks: {words}");
    }
    let all_pairs: Vec<Value> = cards.iter().map(|c| json!([c["domain"], c["card"]])).collect();
    let keys = json!([
        client_entry("all", KEY_ALL, Value::Array(all_pairs)),
        client_entry("triga", KEY_TRIGA, json!([["energy", CARD]])),
        client_entry("thermal", KEY_THERMAL, json!([["energy", "thermal"]])),
    ]);
    let hosted = Hosted::start(&keys).await;

    let pair = json!({"domain": "energy", "card": CARD});
    let described = result_text(&stdio.call("describe_card", &pair).await);
    let description: Value = serde_json::from_str(&described).unwrap();
    // After the kernel's 18 names the list holds the gateway's own three for this card, DtBelowFloor and DtAboveCeiling and ContinuationNotOnCard.
    assert_eq!(description["rejects"].as_array().unwrap().len(), 21);
    let step = description["example_compute_step"].clone();
    let run = description["example_run_trajectory"].clone();
    // A pulse with its scram, on another table than the worked run's.
    let scram = run_body(294.15, 231.0, 5.0e-5, &[(40_000, 2.75, 0.0), (20_000, 2.75, 1.0)], 2_000);

    // The web routes give the bytes of the tools.
    let (status, on_http) = post(&base, "/describe_card", &pair).await;
    assert_eq!(status, 200);
    assert_eq!(on_http, described);
    let mut tapes = Vec::new();
    for (tool, path, arguments) in [
        ("verify_step", "/verify_physics", &step),
        ("run_trajectory", "/run_trajectory", &run),
        ("run_trajectory", "/run_trajectory", &scram),
    ] {
        let on_stdio = result_text(&stdio.call(tool, arguments).await);
        let (status, on_http) = post(&base, path, arguments).await;
        assert_eq!(status, 200, "{on_http}");
        assert_eq!(on_http, on_stdio, "{tool}");
        if tool == "run_trajectory" {
            tapes.push(serde_json::from_str::<Value>(&on_http).unwrap());
        }
    }
    // The worked run is the kernel loop's and passes its power peak.
    let steps = run["segments"][0]["steps"].as_u64().unwrap();
    let direct = direct_loop(
        run["initial"]["start_temperature_k"].as_f64().unwrap(),
        run["initial"]["rod_table_core"].as_f64().unwrap(),
        run["dt_s"].as_f64().unwrap(),
        &[(steps, run["segments"][0]["load"]["reactivity_inserted_dollar"].as_f64().unwrap(), 0.0)],
        1,
    );
    assert_eq!(tapes[0]["outcome"], "completed");
    assert_state_is(&tapes[0]["final_state"], &direct.state, "worked run");
    assert!(direct.state.peak_time_s < direct.state.t_s && direct.state.power_mw < 0.5 * direct.state.peak_power_mw);
    // The pulse with its scram is the kernel loop's.
    let direct = direct_loop(294.15, 231.0, 5.0e-5, &[(40_000, 2.75, 0.0), (20_000, 2.75, 1.0)], 2_000);
    assert_eq!(tapes[1]["outcome"], "completed");
    assert_state_is(&tapes[1]["final_state"], &direct.state, "pulse with its scram");
    // The tapes check on the tool as on the route.
    for tape in &tapes {
        let on_stdio = result_text(&stdio.call("verify_tape", tape).await);
        let (status, on_http) = post(&base, "/verify_tape", tape).await;
        assert_eq!(status, 200);
        assert_eq!(on_http, on_stdio);
        assert_eq!(serde_json::from_str::<Value>(&on_http).unwrap()["holds"], true);
    }

    // The full view and the key that lists the card get the bytes of the local connection.
    for key in [KEY_ALL, KEY_TRIGA] {
        assert_eq!(result_text(&hosted.tool(key, "describe_card", &pair).await), described);
        for (tool, arguments) in [
            ("verify_step", &step),
            ("run_trajectory", &run),
            ("run_trajectory", &scram),
            ("verify_tape", &tapes[1]),
        ] {
            let on_stdio = stdio.call(tool, arguments).await;
            assert_eq!(hosted.tool(key, tool, arguments).await, on_stdio, "{tool}");
        }
    }
    // Refusals are the same bytes on the three connections, with no proof: a reactivity
    // above the rod's whole worth, a rod table the card does not hold, a reactivity that changes.
    let outside = run_body(298.15, 217.0, ceiling(), &[(3, 4.5, 0.0)], 1);
    let no_table = run_body(298.15, 224.0, ceiling(), &[(3, 2.0, 0.0)], 1);
    let changed = run_body(298.15, 217.0, ceiling(), &[(900, 2.0, 0.0), (1, 2.5, 0.0)], 1);
    for (request, name) in [
        (&outside, "ReactivityAboveRodWorth"),
        (&no_table, "RodTableOutsideCard"),
        (&changed, "ReactivityChangedDuringRun"),
    ] {
        let (status, on_http) = post(&base, "/run_trajectory", request).await;
        assert_eq!(status, 422, "{on_http}");
        assert!(on_http.contains(&format!("({name})")), "{on_http}");
        let on_stdio = stdio.call("run_trajectory", request).await;
        assert_eq!(result_text(&on_stdio), on_http, "{name}");
        assert!(!on_stdio.contains("run_seal") && !on_stdio.contains("frame_hex"), "{on_stdio}");
        for key in [KEY_ALL, KEY_TRIGA] {
            assert_eq!(hosted.tool(key, "run_trajectory", request).await, on_stdio, "{name}");
        }
    }
    let (_, on_http) = post(&base, "/run_trajectory", &no_table).await;
    assert_eq!(
        on_http,
        "Physical Failure: 'rod_table_core' 224 is not one of the cores whose rod-worth table the reactor_triga_jsi card holds, 215, 217 and 231 (RodTableOutsideCard). That state is not sealed."
    );
    let seen = result_text(&hosted.tool(KEY_TRIGA, "list_cards", &json!({})).await);
    let seen: Value = serde_json::from_str(&seen).unwrap();
    assert_eq!(seen["cards"].as_array().unwrap().len(), 1);
    assert_eq!(seen["cards"][0]["card"], CARD);

    // A key that does not list it gets the answers of a card that does not exist.
    let invented = "qqqqqqqqqqqqqqqqq";
    assert_eq!(invented.len(), CARD.len());
    let rename = |value: &Value| {
        let mut value = value.clone();
        value["card"] = json!(invented);
        value
    };
    let mut renamed_tape = tapes[0].clone();
    renamed_tape["inputs"]["card"] = json!(invented);
    let mut session = Vec::new();
    for (tool, hidden, fake) in [
        ("describe_card", pair.clone(), rename(&pair)),
        ("verify_step", step.clone(), rename(&step)),
        ("run_trajectory", run.clone(), rename(&run)),
        ("verify_tape", tapes[0].clone(), renamed_tape),
    ] {
        let on_hidden = hosted.tool(KEY_THERMAL, tool, &hidden).await;
        let on_fake = hosted.tool(KEY_THERMAL, tool, &fake).await;
        assert_eq!(on_hidden, on_fake, "{tool}");
        session.push(on_hidden);
    }
    let plan = json!({"domain": "energy", "card": CARD,
        "plan": {"kind": "list", "runs": [{"initial": run["initial"], "dt_s": run["dt_s"], "segments": run["segments"], "record_every": run["record_every"]}]},
        "store": "none", "return": "inline", "answer": "full"});
    let validation = json!({"run": run, "quantity": "power_mw", "measurements": [{"step": 1, "value": 1.0}]});
    let validation_fake = json!({"run": rename(&run), "quantity": "power_mw", "measurements": [{"step": 1, "value": 1.0}]});
    let on_hidden = hosted.tool(KEY_THERMAL, "run_dataset", &plan).await;
    let on_fake = hosted.tool(KEY_THERMAL, "run_dataset", &rename(&plan)).await;
    assert_eq!(on_hidden, on_fake);
    session.push(on_hidden);
    let on_hidden = hosted.tool(KEY_THERMAL, "validate_series", &validation).await;
    let on_fake = hosted.tool(KEY_THERMAL, "validate_series", &validation_fake).await;
    assert_eq!(on_hidden, on_fake);
    session.push(on_hidden);
    assert_eq!(result_text(&hosted.tool(KEY_THERMAL, "describe_card", &pair).await), "Card not recognized (UnknownCard).");

    // Nothing the thermal key was sent names the reactor or its fields.
    let text = session.join("\n").to_lowercase();
    for word in ["reactor", "triga", "reactivity", "rod_table", "rods_dropped", "fuel_temperature", "rod_moving", "peak_power", "scram"] {
        assert!(!text.contains(word), "the word '{word}' of the hidden card was sent");
    }
    let tools = hosted.tool(KEY_THERMAL, "list_cards", &json!({})).await;
    assert!(!tools.to_lowercase().contains("reactor"));

    // With the key that lists it, the dataset and the validation answer, and the model values
    // of the validation are the kernel loop's fuel temperature.
    let on_triga = result_text(&hosted.tool(KEY_TRIGA, "run_dataset", &plan).await);
    assert!(on_triga.contains("dataset_seal") || on_triga.contains("run_seal"), "{on_triga}");
    let landed = steps;
    let validation = json!({"run": run, "quantity": "fuel_temperature_k", "measurements": [{"step": landed, "value": 300.0}]});
    let (status, text) = post(&base, "/validate_series", &validation).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let hex = answer["model_values_hex"][0].as_str().unwrap();
    let whole = direct_loop(
        run["initial"]["start_temperature_k"].as_f64().unwrap(),
        run["initial"]["rod_table_core"].as_f64().unwrap(),
        run["dt_s"].as_f64().unwrap(),
        &[(steps, 2.0, 0.0)],
        1,
    );
    assert_eq!(u64::from_str_radix(hex, 16).unwrap(), whole.state.fuel_temperature_k.to_bits());
    assert_eq!(result_text(&hosted.tool(KEY_TRIGA, "validate_series", &validation).await), text);
}
