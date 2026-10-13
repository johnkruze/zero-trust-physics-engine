//! A stored dataset: a plan sent again, a stored file that is not what it
//! should be, and the modes of what is written. On the hosted door and on the
//! stdio door, with the real binaries.

mod hosted_support;

use std::os::unix::fs::{MetadataExt, symlink};
use std::path::Path;
use std::time::{Duration, Instant};

use hosted_support::*;
use serde_json::{Value, json};

const KEY: &str = "key-of-the-only-client";
const OTHER: &str = "key-of-the-other-client";

fn hosted_in(dir: &Path) -> Hosted {
    Hosted::start(vec![client_entry("a", KEY, 4, 1), client_entry("b", OTHER, 4, 1)], Some(dir))
}

fn client_folder(dir: &Path, key: &str) -> std::path::PathBuf {
    dir.join(sha(&sha(key)))
}

// ------------------------------------------------------------ item 2

#[test]
fn a_plan_sent_again_with_its_keys_in_another_order_is_answered_from_the_stored_folder_on_the_hosted_door() {
    let dir = scratch("reorder_hosted");
    let door = hosted_in(&dir);
    let request = file_request(290.0);
    let (_, error, first) = door.tool(KEY, "run_dataset", &request);
    assert!(!error, "{first}");
    let (_, error, again) = door.tool(KEY, "run_dataset", &request);
    assert!(!error);
    assert_eq!(again, first, "the same request again is the same answer");
    let (_, error, backwards) = door.tool(KEY, "run_dataset", &reversed(&request));
    assert!(!error, "refused: {backwards}");
    // The answer to a repeat carries the request as it was first stored, and nothing else differs.
    let (first, backwards): (Value, Value) = (serde_json::from_str(&first).unwrap(), serde_json::from_str(&backwards).unwrap());
    assert_eq!(backwards, first);
    assert_eq!(backwards["head"]["request"], request);
    assert_eq!(backwards["head"]["dataset_seal"], first["head"]["dataset_seal"]);
    // One folder, and its head is as it was first stored.
    let folder = stored_folder(&client_folder(&dir, KEY), &serde_json::to_string(&first).unwrap());
    let stored = std::fs::read_to_string(folder.join("head.json")).unwrap();
    assert!(stored.starts_with("{\"request\":{\"domain\":\"energy\""), "{stored}");
    let (_, error, check) = door.tool(KEY, "verify_dataset", &json!({"dataset_seal": first["head"]["dataset_seal"]}));
    assert!(!error && check.contains("\"holds\":true"), "{check}");
}

#[test]
fn a_plan_sent_again_with_its_keys_in_another_order_is_answered_from_the_stored_folder_on_the_stdio_door() {
    let dir = scratch("reorder_stdio");
    let mut door = Stdio2::start(Some(&dir));
    let request = file_request(291.0);
    let (error, first) = door.tool("run_dataset", &request);
    assert!(!error, "{first}");
    let (error, backwards) = door.tool("run_dataset", &reversed(&request));
    assert!(!error, "refused: {backwards}");
    assert_eq!(backwards, first);
    let first: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first["head"]["request"], request);
}

#[test]
fn a_head_that_truly_differs_is_still_refused_with_the_old_sentence_and_nothing_is_overwritten() {
    let dir = scratch("differs");
    let door = hosted_in(&dir);
    let request = file_request(292.0);
    let (_, first) = (0, door.tool(KEY, "run_dataset", &request).2);
    let folder = stored_folder(&client_folder(&dir, KEY), &first);
    let head_path = folder.join("head.json");
    let original = std::fs::read_to_string(&head_path).unwrap();
    // A stored head whose count was edited (and so whose receipt no longer matches).
    std::fs::write(&head_path, original.replace("\"completed\":1", "\"completed\":0")).unwrap();
    let edited = std::fs::read_to_string(&head_path).unwrap();
    let (status, error, text) = door.tool(KEY, "run_dataset", &request);
    assert_eq!((status, error), (200, true));
    assert!(text.contains("already holds a head that is not this dataset's. Nothing was overwritten."), "{text}");
    assert_eq!(std::fs::read_to_string(&head_path).unwrap(), edited);
    // Also with the keys reversed.
    let (_, error, text) = door.tool(KEY, "run_dataset", &reversed(&request));
    assert!(error && text.contains("already holds a head that is not this dataset's"), "{text}");
    // A stored head of the same dataset with the receipt damaged is not trusted either.
    std::fs::write(&head_path, original.replace("\"receipt_sha256\":\"", "\"receipt_sha256\":\"0")[..original.len() - 1].to_string() + "\"}").unwrap();
    let (_, error, text) = door.tool(KEY, "run_dataset", &reversed(&request));
    assert!(error, "{text}");
}

// ------------------------------------------------------------ item 3

fn make_dataset(door: &Hosted, key: &str, temperature_k: f64, dir: &Path) -> (String, std::path::PathBuf) {
    let (_, error, text) = door.tool(key, "run_dataset", &file_request(temperature_k));
    assert!(!error, "{text}");
    let seal = serde_json::from_str::<Value>(&text).unwrap()["head"]["dataset_seal"].as_str().unwrap().to_string();
    (seal, stored_folder(&client_folder(dir, key), &text))
}

#[test]
fn a_stored_file_that_is_not_a_regular_file_is_refused_by_name_and_the_door_goes_on() {
    let dir = scratch("not_regular");
    let mut door = hosted_in(&dir);
    let (seal_a, folder_a) = make_dataset(&door, KEY, 293.0, &dir);
    let (seal_b, _) = make_dataset(&door, OTHER, 294.0, &dir);
    let check = |door: &Hosted, key: &str, seal: &str| door.tool(key, "verify_dataset", &json!({"dataset_seal": seal}));
    assert!(check(&door, KEY, &seal_a).2.contains("\"holds\":true"));

    // frames.bin as a link to /dev/zero: refused at once, no gigabytes read.
    let frames = folder_a.join("frames.bin");
    let true_bytes = std::fs::read(&frames).unwrap();
    std::fs::remove_file(&frames).unwrap();
    symlink("/dev/zero", &frames).unwrap();
    let start = Instant::now();
    let (_, error, text) = check(&door, KEY, &seal_a);
    assert!(error && text.contains("the stored file frames.bin was not read. It is a symbolic link"), "{text}");
    assert!(start.elapsed() < Duration::from_secs(5));
    assert!(!text.contains(dir.to_str().unwrap()), "no path of the host in the answer: {text}");
    assert!(door.alive());

    // A link to the true bytes is refused too: the rule is the file itself.
    let real = folder_a.join("frames.real");
    std::fs::write(&real, &true_bytes).unwrap();
    std::fs::remove_file(&frames).unwrap();
    symlink(&real, &frames).unwrap();
    let (_, error, text) = check(&door, KEY, &seal_a);
    assert!(error && text.contains("It is a symbolic link"), "{text}");

    // A named pipe in its place: refused, nothing waits, and the other client's check goes on at once.
    std::fs::remove_file(&frames).unwrap();
    make_fifo(&frames);
    let (_, error, text) = check(&door, KEY, &seal_a);
    assert!(error && text.contains("It is a named pipe"), "{text}");
    let (_, error, text) = check(&door, OTHER, &seal_b);
    assert!(!error && text.contains("\"holds\":true"), "the one slot was kept: {text}");
    let (_, error, again) = check(&door, KEY, &seal_a);
    assert!(error && again.contains("named pipe"));

    // The rows file as a pipe, the head as a link, the folder as a link.
    std::fs::remove_file(&frames).unwrap();
    std::fs::write(&frames, &true_bytes).unwrap();
    assert!(check(&door, KEY, &seal_a).2.contains("\"holds\":true"));
    let rows = folder_a.join("rows.jsonl");
    let rows_bytes = std::fs::read(&rows).unwrap();
    std::fs::remove_file(&rows).unwrap();
    make_fifo(&rows);
    assert!(check(&door, KEY, &seal_a).2.contains("the stored file rows.jsonl was not read. It is a named pipe"));
    std::fs::remove_file(&rows).unwrap();
    std::fs::write(&rows, &rows_bytes).unwrap();
    let moved = client_folder(&dir, KEY).join("moved");
    std::fs::rename(&folder_a, &moved).unwrap();
    symlink(&moved, &folder_a).unwrap();
    let (_, error, text) = check(&door, KEY, &seal_a);
    assert!(error && text.contains(&format!("the stored dataset {seal_a} was not opened. It is a symbolic link")), "{text}");
    std::fs::remove_file(&folder_a).unwrap();
    std::fs::rename(&moved, &folder_a).unwrap();
    assert!(check(&door, KEY, &seal_a).2.contains("\"holds\":true"));
    assert!(door.alive());
}

#[test]
fn a_repeat_run_refuses_a_stored_file_that_is_not_a_regular_file_and_never_waits() {
    let dir = scratch("repeat_not_regular");
    let mut door = hosted_in(&dir);
    let (_, folder) = make_dataset(&door, KEY, 295.0, &dir);
    let frames = folder.join("frames.bin");
    std::fs::remove_file(&frames).unwrap();
    make_fifo(&frames);
    let start = Instant::now();
    let (status, error, text) = door.tool(KEY, "run_dataset", &file_request(295.0));
    assert_eq!((status, error), (200, true));
    assert!(text.contains("holds the head of this dataset, but its frames.bin is not the file the head names. Nothing was overwritten."), "{text}");
    assert!(start.elapsed() < Duration::from_secs(10));
    assert!(door.alive());
    // The stdio door, the same.
    let sdir = scratch("repeat_not_regular_stdio");
    let mut stdio = Stdio2::start(Some(&sdir));
    let (_, first) = stdio.tool("run_dataset", &file_request(296.0));
    let folder = stored_folder(&sdir, &first);
    std::fs::remove_file(folder.join("frames.bin")).unwrap();
    symlink("/dev/zero", folder.join("frames.bin")).unwrap();
    let (error, text) = stdio.tool("run_dataset", &file_request(296.0));
    assert!(error && text.contains("its frames.bin is not the file the head names"), "{text}");
}

#[test]
fn datasets_are_stored_in_folders_of_mode_0700_and_files_of_mode_0600() {
    let dir = scratch("modes");
    let door = hosted_in(&dir);
    // A client folder left by an earlier start with wider modes is narrowed.
    let (_, folder) = make_dataset(&door, KEY, 297.0, &dir);
    let mode = |path: &Path| std::fs::metadata(path).unwrap().mode() & 0o777;
    assert_eq!(mode(&client_folder(&dir, KEY)), 0o700);
    assert_eq!(mode(&folder), 0o700);
    for name in ["head.json", "rows.jsonl", "frames.bin"] {
        assert_eq!(mode(&folder.join(name)), 0o600, "{name}");
    }
    let sdir = scratch("modes_stdio");
    let mut stdio = Stdio2::start(Some(&sdir));
    let (_, text) = stdio.tool("run_dataset", &file_request(298.0));
    let folder = stored_folder(&sdir, &text);
    assert_eq!(mode(&folder), 0o700);
    assert_eq!(mode(&folder.join("rows.jsonl")), 0o600);
}

// ------------------------------------------------------------ item 8

#[test]
fn a_repeat_run_confirms_the_stored_rows_and_frames_by_hash_before_it_answers() {
    let dir = scratch("rerun_tamper");
    let door = hosted_in(&dir);
    let request = file_request(299.0);
    let (_, error, first) = door.tool(KEY, "run_dataset", &request);
    assert!(!error);
    let folder = stored_folder(&client_folder(&dir, KEY), &first);
    let (_, error, again) = door.tool(KEY, "run_dataset", &request);
    assert!(!error);
    assert_eq!(again, first, "a sound folder is answered as before");

    for (name, offset) in [("frames.bin", 100_usize), ("rows.jsonl", 10)] {
        let path = folder.join(name);
        let bytes = std::fs::read(&path).unwrap();
        let mut damaged = bytes.clone();
        damaged[offset] ^= 1;
        std::fs::write(&path, &damaged).unwrap();
        let (status, error, text) = door.tool(KEY, "run_dataset", &request);
        assert_eq!((status, error), (200, true));
        let sentence = format!("Physical Violation: the folder {} holds the head of this dataset, but its {name} is not the file the head names. Nothing was overwritten.", folder.file_name().unwrap().to_str().unwrap());
        assert_eq!(text, sentence);
        // Nothing was overwritten: the damaged bytes are still there. A reversed request is held to the same.
        assert_eq!(std::fs::read(&path).unwrap(), damaged);
        let (_, error, text) = door.tool(KEY, "run_dataset", &reversed(&request));
        assert!(error && text == sentence);
        std::fs::write(&path, &bytes).unwrap();
    }
    // A frame file that is gone, and one that is added to a dataset that stores none.
    let frames = folder.join("frames.bin");
    let bytes = std::fs::read(&frames).unwrap();
    std::fs::remove_file(&frames).unwrap();
    let (_, error, text) = door.tool(KEY, "run_dataset", &request);
    assert!(error && text.contains("its frames.bin is not the file the head names"), "{text}");
    std::fs::write(&frames, &bytes).unwrap();
    let (_, error, _) = door.tool(KEY, "run_dataset", &request);
    assert!(!error);
}

#[tokio::test]
async fn the_http_status_of_a_damaged_repeat_is_500_on_the_plain_http_door() {
    // The plain HTTP door answers the same sentence with HTTP 500.
    let dir = scratch("rerun_http");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = protocol_mcp_gateway::app_with_dataset_dir(Some(std::fs::canonicalize(&dir).unwrap()));
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let post = |body: Value| async move {
        let response = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/run_dataset"))
            .json(&body)
            .send()
            .await
            .unwrap();
        (response.status().as_u16(), response.text().await.unwrap())
    };
    let request = file_request(300.5);
    let (status, first) = post(request.clone()).await;
    assert_eq!(status, 200, "{first}");
    let frames = stored_folder(&dir, &first).join("frames.bin");
    let mut bytes = std::fs::read(&frames).unwrap();
    bytes[100] ^= 1;
    std::fs::write(&frames, bytes).unwrap();
    let (status, text) = post(request).await;
    assert_eq!(status, 500, "{text}");
    assert!(text.contains("its frames.bin is not the file the head names"));
}
