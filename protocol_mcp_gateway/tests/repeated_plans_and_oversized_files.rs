//! Four faults of version 0.2.1, each held here on the real binaries.
//!
//! - A plan sent again with a whole number respelled (`1.0` as `1`) was told
//!   its own dataset was not its dataset. It is answered from the stored
//!   folder, and the stored rows are still held to the new run's rows, value
//!   for value.
//! - A stored file given a huge apparent size (a sparse file) was read for
//!   its whole apparent size. No stored file is read past what the head's
//!   counts allow.
//! - A request with both a `Content-Length` and a `Transfer-Encoding` header
//!   was served by the hosted door. It is refused with 400.
//! - Bodies over the limit left the hosted door holding about a gigabyte. A
//!   body is read into a buffer the door reuses.

mod hosted_support;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use hosted_support::*;
use serde_json::{Value, json};

const KEY: &str = "key-of-the-only-client";
const OTHER: &str = "key-of-the-other-client";
const GIB: u64 = 1 << 30;

fn hosted_in(dir: &Path) -> Hosted {
    Hosted::start(vec![client_entry("a", KEY, 4, 1), client_entry("b", OTHER, 4, 1)], Some(dir))
}

fn client_folder(dir: &Path, key: &str) -> std::path::PathBuf {
    dir.join(sha(&sha(key)))
}

/// The same request with its whole numbers written without a decimal point,
/// as a client that holds them as integers writes them.
fn respelled(request: &Value) -> Value {
    let mut other = request.clone();
    let run = &mut other["plan"]["runs"][0];
    let temperature_k = run["initial"]["temperature_k"].as_f64().unwrap();
    assert_eq!(temperature_k.fract(), 0.0);
    run["initial"]["temperature_k"] = json!(temperature_k as i64);
    run["segments"][0]["load"]["heat_power_w"] = json!(1);
    assert_ne!(other.to_string(), request.to_string());
    assert!(request.to_string().contains("\"heat_power_w\":1.0") && other.to_string().contains("\"heat_power_w\":1,"));
    other
}

fn inline(request: &Value) -> Value {
    let mut other = request.clone();
    other["return"] = json!("inline");
    other["answer"] = json!("full");
    other
}

// ------------------------------------------------------------ a whole number respelled

#[test]
fn a_plan_sent_again_with_a_whole_number_respelled_is_answered_from_the_stored_folder_on_the_hosted_door() {
    let dir = scratch("respelled_hosted");
    let door = hosted_in(&dir);
    let request = file_request(290.0);
    let other = respelled(&request);

    // The two spellings are one dataset: the same seal and the same row tree. The rows file
    // keeps each number as it was written, so its hash is not the same.
    let head = |request: &Value| -> Value {
        let (_, error, text) = door.tool(KEY, "run_dataset", &inline(request));
        assert!(!error, "{text}");
        serde_json::from_str::<Value>(&text).unwrap()["head"].clone()
    };
    let (typed, written) = (head(&request), head(&other));
    assert_eq!(typed["dataset_seal"], written["dataset_seal"]);
    assert_eq!(typed["merkle_root"], written["merkle_root"]);
    assert_eq!(typed["plan_sha384"], written["plan_sha384"]);
    assert_ne!(typed["rows_sha384"], written["rows_sha384"]);
    assert_eq!(typed["frames_sha384"], written["frames_sha384"]);

    let (_, error, first) = door.tool(KEY, "run_dataset", &request);
    assert!(!error, "{first}");
    let (_, error, again) = door.tool(KEY, "run_dataset", &other);
    assert!(!error, "refused: {again}");
    // The answer to a repeat carries the head as it was first stored.
    assert_eq!(again, first);
    let first: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first["head"]["request"], request);
    // Respelled and with its keys in another order, at once.
    let (_, error, backwards) = door.tool(KEY, "run_dataset", &reversed(&other));
    assert!(!error, "refused: {backwards}");
    assert_eq!(serde_json::from_str::<Value>(&backwards).unwrap(), first);
    // One folder, its files as first stored, and it still checks.
    let folders: Vec<_> = std::fs::read_dir(client_folder(&dir, KEY)).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(folders.len(), 1, "{folders:?}");
    let folder = stored_folder(&client_folder(&dir, KEY), &serde_json::to_string(&first).unwrap());
    let rows = std::fs::read_to_string(folder.join("rows.jsonl")).unwrap();
    assert!(rows.contains("\"heat_power_w\":1.0"), "{rows}");
    let (_, error, check) = door.tool(KEY, "verify_dataset", &json!({"dataset_seal": first["head"]["dataset_seal"]}));
    assert!(!error && check.contains("\"holds\":true") && check.contains("\"rows_file\":\"matches the hash in the head\""), "{check}");

    // The other way round, in the other client's folder: stored respelled, sent again as typed.
    let (_, error, first) = door.tool(OTHER, "run_dataset", &other);
    assert!(!error, "{first}");
    let (_, error, again) = door.tool(OTHER, "run_dataset", &request);
    assert!(!error, "refused: {again}");
    assert_eq!(again, first);
    assert_eq!(serde_json::from_str::<Value>(&first).unwrap()["head"]["request"], other);
}

#[test]
fn a_plan_sent_again_with_a_whole_number_respelled_is_answered_from_the_stored_folder_on_the_stdio_door() {
    let dir = scratch("respelled_stdio");
    let mut door = Stdio2::start(Some(&dir));
    let request = file_request(291.0);
    let (error, first) = door.tool("run_dataset", &request);
    assert!(!error, "{first}");
    let (error, again) = door.tool("run_dataset", &respelled(&request));
    assert!(!error, "refused: {again}");
    assert_eq!(again, first);
    let (error, backwards) = door.tool("run_dataset", &reversed(&respelled(&request)));
    assert!(!error, "refused: {backwards}");
    assert_eq!(backwards, first);
}

/// The receipt of a head: SHA-384 of the head without its receipt.
fn with_receipt(document: &str) -> String {
    assert!(document.ends_with('}'));
    format!("{},\"receipt_sha384\":\"{}\"}}", &document[..document.len() - 1], seal_sha(document))
}

fn without_receipt(head: &str) -> String {
    let at = head.rfind(",\"receipt_sha384\":\"").expect("a receipt");
    format!("{}}}", &head[..at])
}

/// A repeat is recognised by what it seals, and the hash of the rows file is
/// no longer part of that. So the stored rows are held to the new run's rows
/// value for value: a stored folder whose rows were changed in a value no
/// seal covers, with its head rewritten to match, is still not answered from.
#[test]
fn a_repeat_is_still_held_to_the_stored_rows_value_for_value() {
    let dir = scratch("respelled_values");
    let door = hosted_in(&dir);
    let request = file_request(292.0);
    let (_, error, first) = door.tool(KEY, "run_dataset", &request);
    assert!(!error, "{first}");
    let folder = stored_folder(&client_folder(&dir, KEY), &first);
    let (head_path, rows_path) = (folder.join("head.json"), folder.join("rows.jsonl"));
    let (head, rows) = (std::fs::read_to_string(&head_path).unwrap(), std::fs::read_to_string(&rows_path).unwrap());
    let refused = |door: &Hosted, request: &Value| {
        let (status, error, text) = door.tool(KEY, "run_dataset", request);
        assert_eq!((status, error), (200, true), "{text}");
        assert!(text.contains("already holds a head that is not this dataset's. Nothing was overwritten."), "{text}");
    };
    let rewrite = |changed_rows: &str| {
        std::fs::write(&rows_path, changed_rows).unwrap();
        let old_hash = serde_json::from_str::<Value>(&head).unwrap()["rows_sha384"].as_str().unwrap().to_string();
        let document = without_receipt(&head).replace(&old_hash, &seal_sha(changed_rows));
        std::fs::write(&head_path, with_receipt(&document)).unwrap();
    };

    // A decimal copy changed beside its untouched hex: no seal covers it, and
    // since 0.11.0 the check by seal names it as a text the engine did not write.
    let row: Value = serde_json::from_str(rows.trim_end()).unwrap();
    let decimal = row["final_state"]["temperature_k"].to_string();
    let changed = rows.replacen(&format!("\"temperature_k\":{decimal}"), "\"temperature_k\":123.5", 1);
    assert_ne!(changed, rows);
    rewrite(&changed);
    let seal = serde_json::from_str::<Value>(&first).unwrap()["head"]["dataset_seal"].clone();
    let (_, error, check) = door.tool(KEY, "verify_dataset", &json!({ "dataset_seal": seal }));
    assert!(!error && check.contains("\"holds\":false,\"values_hold\":false,\"cause\":\"answer_text\"") && check.contains("rows[0].final_state.temperature_k"), "{check}");
    // The repeat is not answered from it, as typed, respelled, or reversed.
    refused(&door, &request);
    refused(&door, &respelled(&request));
    refused(&door, &reversed(&respelled(&request)));
    assert_eq!(std::fs::read_to_string(&rows_path).unwrap(), changed, "nothing was overwritten");

    // The refusal's sentence changed, the same way.
    let changed = rows.replacen("\"reject_detail\":null", "\"reject_detail\":\"planted\"", 1);
    assert_ne!(changed, rows);
    rewrite(&changed);
    refused(&door, &request);
    refused(&door, &respelled(&request));

    // A row more, and a row fewer.
    rewrite(&format!("{rows}{rows}"));
    refused(&door, &respelled(&request));

    // The true files back: the repeat is answered again, in either spelling.
    std::fs::write(&rows_path, &rows).unwrap();
    std::fs::write(&head_path, &head).unwrap();
    for sent in [request.clone(), respelled(&request)] {
        let (_, error, again) = door.tool(KEY, "run_dataset", &sent);
        assert!(!error, "refused: {again}");
        assert_eq!(again, first);
    }
    // The rows as the respelled request writes them, with their own hash in the head: the same values.
    let (_, _, other_inline) = door.tool(KEY, "run_dataset", &inline(&respelled(&request)));
    let other_inline: Value = serde_json::from_str(&other_inline).unwrap();
    let other_rows: String = other_inline["rows"].as_array().unwrap().iter().map(|row| format!("{row}\n")).collect();
    assert_ne!(other_rows, rows);
    rewrite(&other_rows);
    let (_, error, again) = door.tool(KEY, "run_dataset", &request);
    assert!(!error, "refused: {again}");
    assert!(again.contains(&seal_sha(&other_rows)), "the answer carries the stored head: {again}");
}

// ------------------------------------------------------------ a stored file of a huge apparent size

fn make_dataset(door: &Hosted, key: &str, temperature_k: f64, dir: &Path) -> (String, std::path::PathBuf) {
    let (_, error, text) = door.tool(key, "run_dataset", &file_request(temperature_k));
    assert!(!error, "{text}");
    let seal = serde_json::from_str::<Value>(&text).unwrap()["head"]["dataset_seal"].as_str().unwrap().to_string();
    (seal, stored_folder(&client_folder(dir, key), &text))
}

fn set_apparent_size(path: &Path, bytes: u64) {
    std::fs::OpenOptions::new().write(true).open(path).unwrap().set_len(bytes).unwrap();
}

fn resident_mib(door: &Hosted) -> f64 {
    let out = Command::new("ps").args(["-o", "rss=", "-p", &door.child.id().to_string()]).output().expect("ps");
    String::from_utf8_lossy(&out.stdout).trim().parse::<f64>().expect("rss") / 1024.0
}

#[test]
fn a_stored_file_larger_than_the_heads_counts_allow_is_refused_before_it_is_read() {
    let dir = scratch("sparse");
    let mut door = hosted_in(&dir);
    let (seal, folder) = make_dataset(&door, KEY, 293.0, &dir);
    let check = |door: &Hosted| door.tool(KEY, "verify_dataset", &json!({"dataset_seal": seal}));
    assert!(check(&door).2.contains("\"holds\":true"));
    let idle = resident_mib(&door);

    for (name, apparent, of_what) in [
        ("rows.jsonl", 2 * GIB, "the head's count of rows allows"),
        ("frames.bin", 8 * GIB, "the head's count of frames allows"),
        ("head.json", 2 * GIB, "a head can hold"),
    ] {
        let path = folder.join(name);
        let true_size = std::fs::metadata(&path).unwrap().len();
        set_apparent_size(&path, apparent);
        let start = Instant::now();
        let (status, error, text) = check(&door);
        let took = start.elapsed();
        assert_eq!((status, error), (200, true), "{name}: {text}");
        assert!(
            text.starts_with(&format!("Physical Violation: the stored file {name} was not read. It is {apparent} bytes, more than the ")),
            "{name}: {text}"
        );
        assert!(text.ends_with(&format!(" bytes {of_what}.")), "{name}: {text}");
        assert!(!text.contains(dir.to_str().unwrap()), "no path of the host in the answer: {text}");
        // Not read: answered at once, and the door holds no more memory than before.
        assert!(took < Duration::from_secs(2), "{name}: {took:?}");
        assert!(resident_mib(&door) < idle + 64.0, "{name}: {} MiB against {idle} idle", resident_mib(&door));
        // The same plan sent again is not answered from that folder, and is not read either.
        let start = Instant::now();
        let (_, error, text) = door.tool(KEY, "run_dataset", &file_request(293.0));
        assert!(error, "{name}: {text}");
        assert!(text.contains("Nothing was overwritten.") || text.contains("its head was not read. is 2147483648 bytes"), "{name}: {text}");
        assert!(start.elapsed() < Duration::from_secs(2), "{name}");
        // The file at its true size again: the same bytes, and the check holds.
        set_apparent_size(&path, true_size);
        assert!(check(&door).2.contains("\"holds\":true"), "{name}");
    }
    // The other client's check was never held up: it is answered while nothing runs.
    let (seal_b, _) = make_dataset(&door, OTHER, 294.0, &dir);
    assert!(door.tool(OTHER, "verify_dataset", &json!({"dataset_seal": seal_b})).2.contains("\"holds\":true"));
    assert!(door.alive());

    // The frame file is read up to twice its true size, which is 80 bytes for the header and 64 for each frame, and no
    // further. A wrong length inside that is a broken dataset, named by the check as before.
    let frames = folder.join("frames.bin");
    let true_size = std::fs::metadata(&frames).unwrap().len();
    let head: Value = serde_json::from_str(&std::fs::read_to_string(folder.join("head.json")).unwrap()).unwrap();
    assert_eq!(true_size, 80 + 64 * head["frames"].as_u64().unwrap());
    set_apparent_size(&frames, 2 * true_size + 1);
    let (_, error, text) = check(&door);
    assert!(error && text.contains(&format!("It is {} bytes, more than the {} bytes", 2 * true_size + 1, 2 * true_size)), "{text}");
    for wrong in [2 * true_size, true_size + 32, true_size - 64] {
        set_apparent_size(&frames, wrong);
        let (_, error, text) = check(&door);
        assert!(!text.contains("was not read"), "{wrong}: {text}");
        assert!(error || text.contains("\"holds\":false"), "{wrong}: {text}");
    }
}

#[test]
fn a_stored_head_that_states_more_than_one_dataset_may_have_is_refused_and_its_files_are_not_read() {
    let dir = scratch("sparse_head_counts");
    let door = hosted_in(&dir);
    let (seal, folder) = make_dataset(&door, KEY, 295.0, &dir);
    let head_path = folder.join("head.json");
    let head = std::fs::read_to_string(&head_path).unwrap();
    // A head rewritten to allow a rows file of any size, with a receipt that matches it.
    let document = without_receipt(&head).replacen("\"rows\":1,", "\"rows\":100001,", 1);
    assert_ne!(document, without_receipt(&head));
    std::fs::write(&head_path, with_receipt(&document)).unwrap();
    set_apparent_size(&folder.join("rows.jsonl"), 2 * GIB);
    let start = Instant::now();
    let (status, error, text) = door.tool(KEY, "verify_dataset", &json!({"dataset_seal": seal}));
    assert_eq!((status, error), (200, true), "{text}");
    assert!(text.starts_with("Physical Violation: the stored head states 100001 rows and "), "{text}");
    assert!(text.ends_with("Its files were not read."), "{text}");
    assert!(start.elapsed() < Duration::from_secs(2));
}

// ------------------------------------------------------------ a length and an encoding together

const PING: &str = "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}";
const BOTH: &str = "Bad Request: a request must not carry both Content-Length and Transfer-Encoding.";

fn head_with(key: Option<&str>, extra: &str, connection: &str) -> String {
    let key = key.map(|key| format!("Authorization: Bearer {key}\r\n")).unwrap_or_default();
    format!("POST /mcp HTTP/1.1\r\nHost: x\r\n{key}Content-Type: application/json\r\nAccept: application/json\r\n{extra}Connection: {connection}\r\n\r\n")
}

fn chunked(body: &str) -> String {
    format!("{:x}\r\n{body}\r\n0\r\n\r\n", body.len())
}

#[test]
fn a_request_with_both_a_length_and_an_encoding_is_refused_with_400_and_the_door_goes_on() {
    let dir = scratch("both_lengths");
    let mut door = hosted_in(&dir);
    let send = |text: String| parse_reply(&raw(door.port, text.as_bytes(), 10));
    let length = format!("Content-Length: {}\r\n", PING.len());

    // Both, in either order, with a key, with a wrong key and with none: 400, the door's sentence, and the connection closed.
    for key in [Some(KEY), Some("not-a-key"), None] {
        for extra in [format!("Transfer-Encoding: chunked\r\n{length}"), format!("{length}Transfer-Encoding: chunked\r\n"), format!("content-length: {}\r\ntransfer-encoding: chunked\r\n", PING.len())] {
            let reply = send(head_with(key, &extra, "close") + &chunked(PING));
            assert_eq!(reply.status, 400, "{reply:?}");
            assert_eq!(reply.body, BOTH, "{reply:?}");
            assert!(reply.head.to_ascii_lowercase().contains("connection: close"), "{reply:?}");
        }
    }
    // Either alone is served as before.
    let reply = send(head_with(Some(KEY), &length, "close") + PING);
    assert_eq!((reply.status, reply.body.as_str()), (200, "{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{}}"), "{reply:?}");
    let reply = send(head_with(Some(KEY), "Transfer-Encoding: chunked\r\n", "close") + &chunked(PING));
    assert_eq!((reply.status, reply.body.as_str()), (200, "{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{}}"), "{reply:?}");
    // What the library already refused is still 400: two lengths, two encodings, a length shorter than the body.
    let reply = send(head_with(Some(KEY), &format!("{length}Content-Length: {}\r\n", PING.len() + 9), "close") + PING);
    assert_eq!(reply.status, 400, "{reply:?}");
    let reply = send(head_with(Some(KEY), "Transfer-Encoding: chunked\r\nTransfer-Encoding: identity\r\n", "close") + &chunked(PING));
    assert_eq!(reply.status, 400, "{reply:?}");
    let reply = send(head_with(Some(KEY), &format!("Content-Length: {}\r\n", PING.len() - 5), "close") + PING);
    assert_eq!(reply.status, 400, "{reply:?}");
    // No key and no fault of framing is still the door's one 401.
    let reply = send(head_with(None, &length, "close") + PING);
    assert_eq!(reply.status, 401, "{reply:?}");

    // On one connection kept alive: two clean requests are served, each for itself, and the third,
    // which carries both, is refused and ends the connection, so the fourth is never read.
    let clean = head_with(Some(KEY), &length, "keep-alive") + PING;
    let chunks = head_with(Some(KEY), "Transfer-Encoding: chunked\r\n", "keep-alive") + &chunked(PING);
    let both = head_with(Some(KEY), &format!("{length}Transfer-Encoding: chunked\r\n"), "keep-alive") + &chunked(PING);
    let all = raw(door.port, format!("{clean}{chunks}{both}{clean}").as_bytes(), 10);
    let text = String::from_utf8_lossy(&all);
    let statuses: Vec<&str> = text.match_indices("HTTP/1.1 ").map(|(at, _)| &text[at + 9..at + 12]).collect();
    assert_eq!(statuses, ["200", "200", "400"], "{text}");
    assert!(text.ends_with(BOTH), "{text}");
    // A body that holds the two header names as text is a body.
    let tricky = format!("{{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\",\"params\":{{\"note\":\"Transfer-Encoding: chunked\"}}}}\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\n");
    let reply = send(head_with(Some(KEY), &format!("Content-Length: {}\r\n", tricky.len()), "close") + &tricky);
    assert_ne!(reply.body, BOTH, "{reply:?}");
    assert!(door.alive());
}

// ------------------------------------------------------------ bodies over the limit

#[test]
fn bodies_over_the_limit_do_not_leave_the_hosted_door_holding_more_and_more() {
    let dir = scratch("oversize_bodies");
    let mut door = hosted_in(&dir);
    let size = 17 * 1024 * 1024;
    let mut request = format!(
        "POST /mcp HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {KEY}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {size}\r\n\r\n{{\"jsonrpc\":\"2.0\",\"id\":41,\"method\":\"ping\",\"params\":{{\"pad\":\""
    )
    .into_bytes();
    let head_bytes = request.len() - "{\"jsonrpc\":\"2.0\",\"id\":41,\"method\":\"ping\",\"params\":{\"pad\":\"".len();
    request.resize(head_bytes + size, b'y');
    let over = |door: &Hosted| {
        let reply = parse_reply(&raw(door.port, &request, 30));
        // Answered as before: 413, the door's sentence, under the caller's id.
        assert_eq!(reply.status, 413, "{}", reply.head);
        assert!(reply.body.starts_with("{\"jsonrpc\":\"2.0\",\"id\":41,\"error\":{\"code\":-32600,\"message\":\"Request too large: a line is at most 16777216 bytes"), "{}", reply.body);
    };
    for _ in 0..3 {
        over(&door);
    }
    let after_three = resident_mib(&door);
    for _ in 0..37 {
        over(&door);
    }
    let after_forty = resident_mib(&door);
    // One buffer of at most the limit is reused. Version 0.2.1 grew by about 16 MiB a body: 600 MiB more over these 37.
    assert!(after_forty < after_three + 48.0, "{after_three:.0} MiB after three, {after_forty:.0} MiB after forty");
    assert!(after_forty < 160.0, "{after_forty:.0} MiB");
    // Bodies inside the limit are answered, large and small, and the door is no larger for them.
    let large = format!("{PING}{}", " ".repeat(15 * 1024 * 1024));
    for _ in 0..10 {
        let reply = door.post(KEY, &large);
        assert_eq!((reply.status, reply.body.as_str()), (200, "{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{}}"));
        let reply = door.post(KEY, PING);
        assert_eq!((reply.status, reply.body.as_str()), (200, "{\"jsonrpc\":\"2.0\",\"id\":7,\"result\":{}}"));
    }
    assert!(resident_mib(&door) < after_forty + 48.0, "{:.0} MiB", resident_mib(&door));
    assert!(door.alive());
}
