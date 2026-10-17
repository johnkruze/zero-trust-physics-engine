//! Checks on dataset files that need what an integration test cannot reach:
//! the permit pools, a slow row, a row that panics, and the count of finished
//! rows that wait for their turn.
//!
//! The row hook and the high-water mark are `#[cfg(test)]`. The binaries have
//! neither.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use reqwest::Client;
use serde_json::{Value, json};
use tokio::net::TcpListener;

use super::{AppState, dataset, router};
use dataset::RowHook;

#[path = "../tests/dataset_cases/mod.rs"]
mod cases;

struct TempDir(PathBuf);

static COUNTER: AtomicU64 = AtomicU64::new(0);

impl TempDir {
    fn new() -> TempDir {
        let path = std::env::temp_dir().join(format!(
            "gateway_files_probe_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&path).unwrap();
        TempDir(std::fs::canonicalize(path).unwrap())
    }

    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn spawn(state: AppState) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(client: &Client, base: &str, path: &str, body: &Value) -> (u16, String) {
    let url = format!("{base}{path}");
    let mut last_error = None;
    for _ in 0..20 {
        match client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_string(body).unwrap())
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

fn to_file(mut body: Value) -> Value {
    body["return"] = json!("file");
    body
}

/// Twelve runs of 1 to 3,000 steps, so they finish in another order than
/// they start.
fn unequal_list() -> Value {
    let steps = [3000, 5, 1200, 2, 800, 1, 2500, 40, 4, 1500, 7, 300];
    let runs: Vec<Value> = steps
        .iter()
        .enumerate()
        .map(|(index, steps)| {
            json!({
                "initial": { "temperature_k": 260.0 + 7.0 * index as f64 },
                "dt_s": 0.1,
                "segments": [{
                    "steps": steps,
                    "load": { "heat_power_w": index as f64 - 5.0, "ambient_k": 293.15 }
                }],
                "record_every": 100
            })
        })
        .collect();
    cases::request(json!({ "kind": "list", "runs": runs }), "frames", "file")
}

fn sample_of(rows: u64) -> Value {
    let mut body = to_file(cases::sample_request("frames"));
    body["plan"]["rows"] = json!(rows);
    body
}

/// The three files a stored dataset is: the head, the rows and the frames.
fn files_of(dir: &TempDir, answer: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let answer: Value = serde_json::from_str(answer).unwrap();
    let folder = dir.0.join(answer["files"]["folder"].as_str().unwrap());
    (
        std::fs::read(folder.join("head.json")).unwrap(),
        std::fs::read(folder.join("rows.jsonl")).unwrap(),
        std::fs::read(folder.join("frames.bin")).unwrap(),
    )
}

async fn files_with_pool(permits: usize, body: &Value) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let dir = TempDir::new();
    let state = AppState::new(permits).with_dataset_dir(Some(dir.0.clone()));
    let base = spawn(state.clone()).await;
    let client = Client::new();
    let (status, text) = post(&client, &base, "/run_dataset", body).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(state.physics_permits.available_permits(), permits);
    assert_eq!(state.dataset_permits.available_permits(), (permits / 2).max(1));
    assert_eq!(dir.entries().len(), 1, "{:?}", dir.entries());
    files_of(&dir, &text)
}

// F1
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_same_plan_gives_the_same_files_with_a_pool_of_one_two_or_eight() {
    for (name, body) in [
        ("list", unequal_list()),
        ("grid", to_file(cases::grid_request("frames"))),
        ("sample", sample_of(24)),
    ] {
        let reference = files_with_pool(1, &body).await;
        assert!(reference.2.len() > 64, "{name}");
        for permits in [2, 8] {
            let found = files_with_pool(permits, &body).await;
            assert_eq!(found.0, reference.0, "{name} head, pool {permits}");
            assert_eq!(found.1, reference.1, "{name} rows, pool {permits}");
            assert_eq!(found.2, reference.2, "{name} frames, pool {permits}");
        }
    }
}

// F1
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_same_plan_run_twenty_times_with_eight_permits_gives_the_same_files() {
    let body = unequal_list();
    let reference = files_with_pool(1, &body).await;
    for time in 0..20 {
        let found = files_with_pool(8, &body).await;
        assert_eq!(found, reference, "run {time}");
    }
}

// F6, and a panic in a row's run
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_job_that_fails_leaves_no_folder_and_gives_every_permit_back() {
    for permits in [1, 8] {
        for failing_row in [0, 5, 11] {
            let dir = TempDir::new();
            let mut state = AppState::new(permits).with_dataset_dir(Some(dir.0.clone()));
            state.row_hook = Some(RowHook::Panic(failing_row));
            let base = spawn(state.clone()).await;
            let client = Client::new();
            let (status, text) = post(&client, &base, "/run_dataset", &unequal_list()).await;
            assert_eq!(status, 500, "{text}");
            assert_eq!(
                text,
                "Physical Violation: a run stopped by a fault. The dataset was not written."
            );
            assert!(dir.entries().is_empty(), "{:?}", dir.entries());
            assert_eq!(state.physics_permits.available_permits(), permits);
            assert_eq!(state.dataset_permits.available_permits(), (permits / 2).max(1));
            // The gateway goes on: the same plan without the fault is written.
            state.row_hook = None;
            let base = spawn(state.clone()).await;
            let (status, text) = post(&client, &base, "/run_dataset", &unequal_list()).await;
            assert_eq!(status, 200, "{text}");
            assert_eq!(dir.entries().len(), 1);
        }
    }
}

// F6: a job that fails at the end, when the folder is put in place.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_job_whose_folder_cannot_be_renamed_into_place_leaves_no_temporary_folder() {
    // The final name is taken by a file, not a folder, so the rename fails
    // and the name is not a stored dataset.
    let dir = TempDir::new();
    let state = AppState::new(4).with_dataset_dir(Some(dir.0.clone()));
    let base = spawn(state.clone()).await;
    let client = Client::new();
    let (status, text) = post(&client, &base, "/run_dataset", &unequal_list()).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let seal = answer["files"]["folder"].as_str().unwrap().to_string();
    std::fs::remove_dir_all(dir.0.join(&seal)).unwrap();
    std::fs::write(dir.0.join(&seal), b"in the way").unwrap();
    let (status, text) = post(&client, &base, "/run_dataset", &unequal_list()).await;
    assert_eq!(status, 500, "{text}");
    assert_eq!(text, format!("Physical Violation: the folder {seal} could not be made."));
    assert_eq!(dir.entries(), vec![seal]);
}

// One check of a stored dataset runs at a time.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_second_check_of_a_stored_dataset_is_busy_while_one_runs() {
    let dir = TempDir::new();
    let state = AppState::new(4).with_dataset_dir(Some(dir.0.clone()));
    let base = spawn(state.clone()).await;
    let client = Client::new();
    let (status, text) = post(&client, &base, "/run_dataset", &unequal_list()).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let by_seal = json!({ "dataset_seal": answer["files"]["folder"] });
    let held = Arc::clone(&state.stored_verifications).try_acquire_owned().unwrap();
    let (status, text) = post(&client, &base, "/verify_dataset", &by_seal).await;
    assert_eq!(status, 429, "{text}");
    drop(held);
    let (status, text) = post(&client, &base, "/verify_dataset", &by_seal).await;
    assert_eq!(status, 200, "{text}");
    let report: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["holds"], true, "{report}");
}

// The stored check reads the frame file as a stream. Its answer is the
// answer of the check over the same three files held whole.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_stored_check_answers_as_the_check_of_the_files_held_whole() {
    let dir = TempDir::new();
    let state = AppState::new(4).with_dataset_dir(Some(dir.0.clone()));
    let base = spawn(state).await;
    let client = Client::new();
    let mut none = unequal_list();
    none["store"] = json!("none");
    let mut seals = Vec::new();
    for body in [unequal_list(), none] {
        let (status, text) = post(&client, &base, "/run_dataset", &to_file(body)).await;
        assert_eq!(status, 200, "{text}");
        let answer: Value = serde_json::from_str(&text).unwrap();
        seals.push(answer["files"]["folder"].as_str().unwrap().to_string());
    }
    let flat = |e: crate::live::DoorError| match e {
        crate::live::DoorError::Unprocessable(m) | crate::live::DoorError::Internal(m) | crate::live::DoorError::NotFound(m) | crate::live::DoorError::TooMany(m) => m,
    };
    let whole = |seal: &str| -> Result<String, String> {
        let folder = dir.0.join(seal);
        let head = std::fs::read(folder.join("head.json")).unwrap();
        let rows = std::fs::read(folder.join("rows.jsonl")).unwrap();
        let frames = std::fs::read(folder.join("frames.bin")).ok();
        dataset::report(&head, &rows, frames.as_deref()).map_err(|e| format!("{}", flat(e)))
    };
    let stored = |seal: &str| -> Result<String, String> {
        dataset::verify_stored(&dir.0, seal).map_err(|e| format!("{}", flat(e)))
    };
    // Intact, with frames and with none.
    for seal in &seals {
        let answer = stored(seal).unwrap();
        assert_eq!(Ok(answer.clone()), whole(seal));
        assert!(answer.contains("\"holds\":true"), "{answer}");
    }
    // Damaged frame files: a changed byte in a frame, a changed header, a
    // short file, a long file, an empty file.
    let frames = dir.0.join(&seals[0]).join("frames.bin");
    let original = std::fs::read(&frames).unwrap();
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for at in [original.len() - 1, 70, 10, 30] {
        let mut changed = original.clone();
        changed[at] ^= 0x01;
        cases.push(changed);
    }
    cases.push(original[..original.len() - 7].to_vec());
    cases.push([original.clone(), vec![0u8; 32]].concat());
    cases.push(original[..40].to_vec());
    cases.push(Vec::new());
    for (n, bytes) in cases.iter().enumerate() {
        std::fs::write(&frames, bytes).unwrap();
        let (a, b) = (stored(&seals[0]), whole(&seals[0]));
        assert_eq!(a, b, "case {n}");
    }
    // A frame file where the plan stores none, and a stored plan with none.
    std::fs::write(dir.0.join(&seals[1]).join("frames.bin"), &original).unwrap();
    assert_eq!(stored(&seals[1]), whole(&seals[1]));
    std::fs::remove_file(&frames).unwrap();
    assert_eq!(stored(&seals[0]), whole(&seals[0]));
    // A frame file that is not a file keeps its old 500.
    std::fs::create_dir(&frames).unwrap();
    assert!(stored(&seals[0]).unwrap_err().contains("the stored file frames.bin was not read"));
}

// The stored check holds no more of the frame file than the reader's buffer
// and one frame. The reader here counts the largest read it is given.
#[test]
fn the_stored_check_asks_the_frame_file_for_one_buffer_at_a_time() {
    use std::io::Read;
    struct Counting<R>(R, Arc<AtomicU64>);
    impl<R: Read> Read for Counting<R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.1.fetch_max(buf.len() as u64, Ordering::SeqCst);
            self.0.read(buf)
        }
    }
    let dir = TempDir::new();
    let state = AppState::new(4).with_dataset_dir(Some(dir.0.clone()));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (base, answer) = runtime.block_on(async {
        let base = spawn(state).await;
        let (status, text) = post(&Client::new(), &base, "/run_dataset", &to_file(unequal_list())).await;
        assert_eq!(status, 200, "{text}");
        (base, serde_json::from_str::<Value>(&text).unwrap())
    });
    let _ = base;
    let folder = dir.0.join(answer["files"]["folder"].as_str().unwrap());
    let head = std::fs::read(folder.join("head.json")).unwrap();
    let rows = std::fs::read(folder.join("rows.jsonl")).unwrap();
    let file = std::fs::File::open(folder.join("frames.bin")).unwrap();
    let size = file.metadata().unwrap().len();
    let largest = Arc::new(AtomicU64::new(0));
    let reader = Counting(std::io::BufReader::with_capacity(1 << 20, file), Arc::clone(&largest));
    let checked = genesis_seal::dataset::check_dataset_stream(&head, &rows, Some(reader)).unwrap();
    assert!(checked.report.holds);
    assert!(size > 0 && largest.load(Ordering::SeqCst) <= 1 << 20);
}

// F7
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_job_takes_at_most_half_the_pool_and_gives_every_permit_back() {
    let dir = TempDir::new();
    let mut state = AppState::new(8).with_dataset_dir(Some(dir.0.clone()));
    state.row_hook = Some(RowHook::Slow(0, Duration::from_millis(1500)));
    let base = spawn(state.clone()).await;
    let client = Client::builder().timeout(Duration::from_secs(60)).build().unwrap();
    assert_eq!(state.dataset_permits.available_permits(), 4);

    // One job of many rows takes four pairs, and no more.
    let job = {
        let (client, base, body) = (client.clone(), base.clone(), sample_of(40));
        tokio::spawn(async move { post(&client, &base, "/run_dataset", &body).await })
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(state.dataset_permits.available_permits(), 0);
    assert_eq!(state.physics_permits.available_permits(), 4);
    // A single run still gets a permit while the job holds all four.
    let single = json!({
        "domain": "energy", "card": "thermal", "initial": { "temperature_k": 293.15 },
        "dt_s": 0.1,
        "segments": [{ "steps": 2, "load": { "heat_power_w": 1.0, "ambient_k": 293.15 } }],
        "record_every": 1
    });
    let (status, text) = post(&client, &base, "/run_trajectory", &single).await;
    assert_eq!(status, 200, "{text}");
    // A second job finds no dataset permit.
    let (status, text) = post(&client, &base, "/run_dataset", &sample_of(10)).await;
    assert_eq!((status, text.as_str()), (429, "Too many concurrent physics tasks."));
    assert_eq!(state.physics_permits.available_permits(), 4);
    let (status, text) = job.await.unwrap();
    assert_eq!(status, 200, "{text}");
    assert_eq!(state.physics_permits.available_permits(), 8);
    assert_eq!(state.dataset_permits.available_permits(), 4);

    // Two jobs together take at most four dataset permits. The job of three
    // rows starts first and holds three pairs while its slow first row runs;
    // started together, the larger job can take all four and the smaller is
    // then refused as busy, which is the rule and not a fault.
    let mut jobs = Vec::new();
    for rows in [3_u64, 40] {
        let (client, base, body) = (client.clone(), base.clone(), sample_of(rows));
        jobs.push(tokio::spawn(async move { post(&client, &base, "/run_dataset", &body).await }));
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert_eq!(state.dataset_permits.available_permits(), 0);
    assert_eq!(state.physics_permits.available_permits(), 4);
    for job in jobs {
        let (status, text) = job.await.unwrap();
        assert_eq!(status, 200, "{text}");
    }
    assert_eq!(state.physics_permits.available_permits(), 8);
    assert_eq!(state.dataset_permits.available_permits(), 4);

    // A job takes no more pairs than it has rows.
    let (client2, base2, body) = (client.clone(), base.clone(), sample_of(2));
    let job = tokio::spawn(async move { post(&client2, &base2, "/run_dataset", &body).await });
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(state.dataset_permits.available_permits(), 2);
    assert_eq!(state.physics_permits.available_permits(), 6);
    assert_eq!(job.await.unwrap().0, 200);

    // Where only one physics permit is free the job still runs, on one worker.
    let held = Arc::clone(&state.physics_permits).try_acquire_many_owned(7).unwrap();
    let job = {
        let (client, base, body) = (client.clone(), base.clone(), sample_of(6));
        tokio::spawn(async move { post(&client, &base, "/run_dataset", &body).await })
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(state.physics_permits.available_permits(), 0);
    assert_eq!(state.dataset_permits.available_permits(), 3);
    let (status, text) = job.await.unwrap();
    assert_eq!(status, 200, "{text}");
    assert_eq!(state.physics_permits.available_permits(), 1);
    drop(held);
    assert_eq!(state.physics_permits.available_permits(), 8);
    assert_eq!(state.dataset_permits.available_permits(), 4);
}

// F8
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn finished_rows_that_wait_for_a_slow_first_row_stay_within_the_window() {
    let dir = TempDir::new();
    let mut state = AppState::new(16).with_dataset_dir(Some(dir.0.clone()));
    state.row_hook = Some(RowHook::Slow(0, Duration::from_millis(1500)));
    assert_eq!(state.dataset_permits.available_permits(), 8);
    let base = spawn(state.clone()).await;
    let client = Client::builder().timeout(Duration::from_secs(60)).build().unwrap();
    let (status, slow) = post(&client, &base, "/run_dataset", &sample_of(60)).await;
    assert_eq!(status, 200, "{slow}");
    // Eight workers allow 16 rows running or waiting, and one is row 0.
    let high = state.window_high_water.load(Ordering::SeqCst);
    assert!(high <= 16, "{high} finished rows waited");
    assert!(high >= 8, "only {high} finished rows waited, so the window was not tried");

    // The slow row did not change a byte.
    let quick_dir = TempDir::new();
    let quick = AppState::new(1).with_dataset_dir(Some(quick_dir.0.clone()));
    let base = spawn(quick).await;
    let (status, fast) = post(&client, &base, "/run_dataset", &sample_of(60)).await;
    assert_eq!(status, 200, "{fast}");
    assert_eq!(slow, fast);
}

// F11
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_limits_of_a_file_job_refuse_before_any_permit() {
    let dir = TempDir::new();
    let state = AppState::new(4).with_dataset_dir(Some(dir.0.clone()));
    let base = spawn(state.clone()).await;
    let client = Client::new();
    // Every permit of both pools is held. A job that asked for one would be 429.
    let physics = Arc::clone(&state.physics_permits).try_acquire_many_owned(4).unwrap();
    let datasets = Arc::clone(&state.dataset_permits).try_acquire_many_owned(2).unwrap();
    let (status, text) = post(&client, &base, "/run_dataset", &sample_of(3)).await;
    assert_eq!((status, text.as_str()), (429, "Too many concurrent physics tasks."));

    let refusals = cases::limit_refusals(
        dataset::MAX_DATASET_ROWS,
        dataset::MAX_DATASET_STEPS,
        dataset::MAX_DATASET_FRAMES,
        dataset::MAX_INLINE_ROWS,
        dataset::MAX_INLINE_FRAMES,
    );
    let mut checked = 0;
    for case in refusals.iter().take(3) {
        let mut body: Value = serde_json::from_str(&case.body).unwrap();
        assert_eq!(body["return"], "inline", "{}", case.name);
        body["return"] = json!("file");
        let (status, text) = post(&client, &base, "/run_dataset", &body).await;
        assert_eq!(status, 422, "{}: {text}", case.name);
        assert!(text.contains(case.says), "{}: {text}", case.name);
        assert_eq!(state.physics_permits.available_permits(), 0);
        assert_eq!(state.dataset_permits.available_permits(), 0);
        checked += 1;
    }
    assert_eq!(checked, 3);
    assert!(dir.entries().is_empty());
    drop(physics);
    drop(datasets);

    // A file job may be as long as the limits allow and as large as a
    // single run allows: the same plans just inside the limits are not refused.
    let inside = |rows: u64, steps: u64, record_every: u64| {
        let mut body = to_file(cases::sample_request("none"));
        body["plan"]["rows"] = json!(rows);
        body["plan"]["base"]["segments"][0]["steps"] = json!(steps);
        body["plan"]["base"]["record_every"] = json!(record_every);
        body
    };
    // The inline limits do not bind a file job: 1,001 rows and 20,000 frames.
    let body = inside(1001, 20, 1);
    let (status, text) = post(&client, &base, "/run_dataset", &body).await;
    assert_eq!(status, 200, "{}", &text[..text.len().min(300)]);
}
