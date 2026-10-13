//! A client's share of the physics pool: how many physics permits one client
//! may hold at once, and that every permit goes back, by error and by panic.
//!
//! These run on the state of a client, in the process, without a socket. The
//! real-socket test with two clients is `tests/hosted_shares.rs`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::json;
use tokio::sync::Semaphore;

use super::{AppState, DoorError, acquire_physics_permit, dataset, on_pool};
use dataset::RowHook;

#[path = "../tests/dataset_cases/mod.rs"]
mod cases;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn folder() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "gateway_share_probe_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&path).unwrap();
    std::fs::canonicalize(path).unwrap()
}

fn sharing(pool: usize, share: usize) -> (AppState, Arc<Semaphore>) {
    let share_permits = Arc::new(Semaphore::new(share));
    (AppState::new(pool).with_share(Arc::clone(&share_permits)), share_permits)
}

#[test]
fn a_caller_at_its_share_is_busy_and_the_pool_keeps_its_other_permits() {
    let (state, share) = sharing(4, 2);
    let first = acquire_physics_permit(&state).unwrap();
    let second = acquire_physics_permit(&state).unwrap();
    assert_eq!((share.available_permits(), state.physics_permits.available_permits()), (0, 2));
    // The third is busy, with the same status and text as a full pool, and takes nothing.
    let busy = acquire_physics_permit(&state).err().unwrap();
    assert_eq!(busy.0, axum::http::StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(busy.1, "Too many concurrent physics tasks.");
    assert_eq!((share.available_permits(), state.physics_permits.available_permits()), (0, 2));
    // Another caller with its own share still gets the pool's other permits.
    let other = state.clone().with_share(Arc::new(Semaphore::new(2)));
    let third = acquire_physics_permit(&other).unwrap();
    let fourth = acquire_physics_permit(&other).unwrap();
    assert_eq!(state.physics_permits.available_permits(), 0);
    // The pool is empty: this caller, with share left, is busy in the same words.
    let (state_c, share_c) = (state.clone().with_share(Arc::new(Semaphore::new(1))), ());
    let _ = share_c;
    let busy = acquire_physics_permit(&state_c).err().unwrap();
    assert_eq!(busy.1, "Too many concurrent physics tasks.");
    drop((first, second, third, fourth));
    assert_eq!((share.available_permits(), state.physics_permits.available_permits()), (2, 4));
}

#[tokio::test]
async fn a_panic_in_a_run_gives_the_share_and_the_pool_permit_back() {
    let (state, share) = sharing(4, 2);
    let inner = state.clone();
    let outcome = tokio::spawn(async move {
        let _ = on_pool(&inner, || -> Result<(), String> { panic!("a physics step panicked") }).await;
    })
    .await;
    assert!(outcome.is_err(), "the task panicked");
    assert_eq!((share.available_permits(), state.physics_permits.available_permits()), (2, 4));
    // An error from the run gives them back as well.
    let refused = on_pool(&state, || -> Result<(), String> { Err("refused part way".into()) }).await;
    assert!(matches!(refused, Err(DoorError::Unprocessable(_))));
    assert_eq!((share.available_permits(), state.physics_permits.available_permits()), (2, 4));
    // And a finished run.
    assert!(on_pool(&state, || Ok::<u8, String>(1)).await.is_ok());
    assert_eq!((share.available_permits(), state.physics_permits.available_permits()), (2, 4));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_dataset_job_takes_no_more_workers_than_its_share_and_gives_all_back() {
    let dir = folder();
    let (state, share) = sharing(8, 2);
    let mut state = state.with_dataset_dir(Some(dir.clone()));
    state.row_hook = Some(RowHook::Slow(0, Duration::from_millis(1200)));
    let mut body = cases::sample_request("frames");
    body["plan"]["rows"] = json!(40);
    body["return"] = json!("file");
    let job = {
        let (state, body) = (state.clone(), body.clone());
        tokio::spawn(async move { dataset::run(&state, body).await })
    };
    tokio::time::sleep(Duration::from_millis(400)).await;
    // Eight permits and a dataset pool of four, but this client's share is two: two workers.
    assert_eq!(share.available_permits(), 0);
    assert_eq!(state.physics_permits.available_permits(), 6);
    assert_eq!(state.dataset_permits.available_permits(), 2);
    // A run of the same client is busy while its job holds the whole share.
    let busy = on_pool(&state, || Ok::<(), String>(())).await;
    assert!(matches!(busy, Err(DoorError::TooMany(_))));
    assert!(job.await.unwrap().is_ok());
    assert_eq!(share.available_permits(), 2);
    assert_eq!(state.physics_permits.available_permits(), 8);
    assert_eq!(state.dataset_permits.available_permits(), 4);

    // A job whose row panics ends with a sentence and gives everything back.
    let mut panicking = state.clone();
    panicking.row_hook = Some(RowHook::Panic(3));
    let failed = dataset::run(&panicking, cases::request(json!({"kind": "list", "runs": [
        cases::run(280.0, 1.0, 3), cases::run(281.0, 1.0, 3), cases::run(282.0, 1.0, 3), cases::run(283.0, 1.0, 3)
    ]}), "frames", "file")).await;
    assert!(failed.is_err());
    assert_eq!(share.available_permits(), 2);
    assert_eq!(state.physics_permits.available_permits(), 8);
    assert_eq!(state.dataset_permits.available_permits(), 4);
    let _ = std::fs::remove_dir_all(dir);
}
