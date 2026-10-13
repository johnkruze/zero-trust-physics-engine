//! A client's share of the physics pool, with two clients and real concurrency
//! over real sockets against the real binary.

mod hosted_support;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use hosted_support::*;
use serde_json::json;

const HEAVY: &str = "key-of-the-heavy-client";
const LIGHT: &str = "key-of-the-light-client";

/// The pool the door makes on this machine: the cores, from 1 to 32.
fn pool() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(16).clamp(1, 32)
}

fn long_run() -> serde_json::Value {
    thermal_run(1_000_000, 293.15)
}

#[test]
fn a_heavy_client_at_its_share_never_stops_a_light_clients_steps() {
    let pool = pool();
    if pool < 2 {
        eprintln!("skipped: a pool of one permit cannot be shared by two clients");
        return;
    }
    // Shares that sum to the pool.
    let heavy_share = pool - 1;
    let door = Arc::new(Hosted::start(
        vec![client_entry("heavy", HEAVY, 200, heavy_share as u64), client_entry("light", LIGHT, 4, 1)],
        None,
    ));
    let stop = Arc::new(AtomicBool::new(false));
    let busy = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicUsize::new(0));
    let threads: Vec<_> = (0..heavy_share + 6)
        .map(|_| {
            let (door, stop, busy, done) = (door.clone(), stop.clone(), busy.clone(), done.clone());
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    let (status, error, text) = door.tool(HEAVY, "run_trajectory", &long_run());
                    assert_eq!(status, 200, "{text}");
                    if error {
                        // The share is used up: busy, as a full pool is, as a tool result.
                        assert_eq!(text, "Too many concurrent physics tasks.");
                        busy.fetch_add(1, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(5));
                    } else {
                        done.fetch_add(1, Ordering::SeqCst);
                    }
                }
            })
        })
        .collect();
    // Let the heavy client fill its share, then ask for light steps all the while.
    let start = Instant::now();
    while busy.load(Ordering::SeqCst) == 0 && start.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(busy.load(Ordering::SeqCst) > 0, "the heavy client never met its share");
    let mut answered = 0;
    for _ in 0..60 {
        let (status, error, text) = door.tool(LIGHT, "verify_step", &thermal_step());
        assert_eq!((status, error), (200, false), "the light client's step was stopped: {text}");
        answered += 1;
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(answered, 60);
    stop.store(true, Ordering::SeqCst);
    for thread in threads {
        thread.join().unwrap();
    }
    assert!(done.load(Ordering::SeqCst) > 0, "the heavy client ran nothing");
}

#[test]
fn no_permit_is_lost_after_busy_answers_refused_runs_and_a_full_share_of_runs() {
    let pool = pool();
    let share = pool.min(3);
    let door = Arc::new(Hosted::start(vec![client_entry("heavy", HEAVY, 100, share as u64)], None));
    // Many more than the share at once: some run, the rest are busy, none fails otherwise.
    // A start low in charge is a continuation and needs from, so the run that
    // refuses part way is the cold plate heated past its ceiling.
    let discharge = json!({
        "domain": "energy", "card": "thermal", "initial": { "temperature_k": 390.0 }, "dt_s": 1.0,
        "segments": [{ "steps": 600, "load": { "heat_power_w": 3000.0, "ambient_k": 293.15 } }], "record_every": 1
    });
    let wave = |run: serde_json::Value, count: usize| -> (usize, usize) {
        let threads: Vec<_> = (0..count)
            .map(|_| {
                let (door, run) = (door.clone(), run.clone());
                std::thread::spawn(move || door.tool(HEAVY, "run_trajectory", &run))
            })
            .collect();
        let mut ran = 0;
        let mut busy = 0;
        for thread in threads {
            let (status, error, text) = thread.join().unwrap();
            assert_eq!(status, 200);
            if error {
                assert_eq!(text, "Too many concurrent physics tasks.");
                busy += 1;
            } else {
                ran += 1;
            }
        }
        (ran, busy)
    };
    let (_, busy_long) = wave(long_run(), share + 8);
    assert!(busy_long > 0);
    // A run that refuses part way (the plate leaves its range) is a result, and gives its permit back.
    wave(discharge.clone(), share + 8);
    // A request refused before any permit (an unknown field) takes none.
    let mut unknown = thermal_run(3, 293.15);
    unknown["bogus"] = json!(1);
    for _ in 0..20 {
        let (status, error, _) = door.tool(HEAVY, "run_trajectory", &unknown);
        assert_eq!((status, error), (200, true));
    }
    // A dataset job holds a permit for each worker and gives them all back, errors included.
    let dir = scratch("share_dataset");
    drop(door);
    let door = Hosted::start(vec![client_entry("heavy", HEAVY, 100, share as u64)], Some(&dir));
    let mut plan = file_request(280.0);
    plan["plan"]["runs"] = json!((0..30).map(|i| {
        let mut run = thermal_run(2_000, 280.0 + i as f64);
        run.as_object_mut().unwrap().remove("domain");
        run.as_object_mut().unwrap().remove("card");
        run
    }).collect::<Vec<_>>());
    let (status, error, text) = door.tool(HEAVY, "run_dataset", &plan);
    assert_eq!((status, error), (200, false), "{text}");
    let mut bad = plan.clone();
    // A row the door will not admit is a row of the dataset, not a failure of the job.
    bad["plan"]["runs"][5]["initial"]["temperature_k"] = json!(-5.0);
    let (status, error, text) = door.tool(HEAVY, "run_dataset", &bad);
    assert_eq!((status, error), (200, false), "{text}");
    // A plan the door refuses whole takes no permit.
    bad["plan"]["runs"][6]["unknown_field"] = json!(1);
    let (_, error, _) = door.tool(HEAVY, "run_dataset", &bad);
    assert!(error);
    // Afterwards the whole share is free: exactly `share` long runs at once all run.
    let threads: Vec<_> = (0..share)
        .map(|_| {
            let port = door.port;
            std::thread::spawn(move || {
                let reply = parse_reply(&raw(port, &request_bytes(Some(HEAVY), &tool_body(1, "run_trajectory", &long_run())), 120));
                tool_result(&reply)
            })
        })
        .collect();
    for thread in threads {
        let (error, text) = thread.join().unwrap();
        assert!(!error, "a permit was lost: {text}");
    }
}

#[test]
fn a_share_larger_than_the_pool_stops_the_door_at_start_by_name() {
    let dir = scratch("share_too_big");
    let keys = dir.join("keys.json");
    std::fs::write(&keys, serde_json::to_string(&json!([client_entry("big", HEAVY, 4, 33)])).unwrap()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hosted_door"))
        .args(["--port", "0", "--keys"])
        .arg(&keys)
        .output()
        .unwrap();
    // 33 is above the pool on every machine: the pool never exceeds 32.
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("The hosted door did not start: client 'big': 'max_permits' is 33, more than the "), "{stderr}");
    assert!(output.stdout.is_empty());
}
