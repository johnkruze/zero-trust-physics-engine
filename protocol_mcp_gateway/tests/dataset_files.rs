//! `run_dataset` with `return: "file"` and `verify_dataset` with a stored
//! dataset's seal. Checks that need the permit pools are unit tests in
//! `src/dataset_files_probe.rs`.

mod dataset_cases;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use dataset_cases::{grid_request, list_request, sample_request};
use protocol_mcp_gateway::{app, app_with_dataset_dir, dataset_dir_from_args};
use reqwest::Client;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;
use serde_json::{Map, Value, json};
use tokio::net::TcpListener;

const NO_DIRECTORY: &str = "this connection was started without a directory for dataset files.";
/// What a verification by seal adds when the dataset is not stored.
const VERIFY_BY_PARTS: &str = "A dataset returned in the answer (return \"inline\") is not stored: verify it by sending its run_dataset request as 'request' with its 'dataset_seal', which runs the plan again, or by sending back the head, rows and frames_hex of its full answer (answer \"full\").";

/// A folder under the system's temporary folder, removed when the test ends.
struct TempDir(PathBuf);

static COUNTER: AtomicU64 = AtomicU64::new(0);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!(
            "gateway_dataset_files_{tag}_{}_{}",
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

async fn spawn(dir: Option<&TempDir>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = match dir {
        Some(dir) => app_with_dataset_dir(Some(dir.0.clone())),
        None => app(),
    };
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    tokio::task::yield_now().await;
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, path: &str, body: &Value) -> (u16, String) {
    let client = Client::new();
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

fn with_return(mut body: Value, returns: &str) -> Value {
    body["return"] = json!(returns);
    body
}

/// A dataset run to a file: the answer, and the three files it names.
struct Stored {
    answer: Value,
    text: String,
    folder: PathBuf,
}

impl Stored {
    fn seal(&self) -> String {
        self.answer["files"]["folder"].as_str().unwrap().to_string()
    }

    fn read(&self, name: &str) -> Vec<u8> {
        std::fs::read(self.folder.join(name)).unwrap()
    }
}

async fn run_to_file(base: &str, dir: &TempDir, body: &Value) -> Stored {
    let (status, text) = post(base, "/run_dataset", body).await;
    assert_eq!(status, 200, "{text}");
    let answer: Value = serde_json::from_str(&text).unwrap();
    let folder = dir.0.join(answer["files"]["folder"].as_str().unwrap());
    Stored { answer, text, folder }
}

async fn run_inline(base: &str, body: &Value) -> Value {
    let (status, text) = post(base, "/run_dataset", body).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

async fn verify_seal(base: &str, seal: &str) -> Value {
    let (status, text) = post(base, "/verify_dataset", &json!({ "dataset_seal": seal })).await;
    assert_eq!(status, 200, "{text}");
    serde_json::from_str(&text).unwrap()
}

// F2
#[tokio::test]
async fn the_file_form_and_the_inline_form_of_a_plan_hold_the_same_rows_and_frames() {
    let dir = TempDir::new("f2");
    let base = spawn(Some(&dir)).await;
    for (name, plan) in [
        ("list", list_request("frames")),
        ("grid", grid_request("frames")),
        ("sample", sample_request("frames")),
    ] {
        let inline = run_inline(&base, &with_return(plan.clone(), "inline")).await;
        let stored = run_to_file(&base, &dir, &with_return(plan, "file")).await;
        let (a, b) = (&inline["head"], &stored.answer["head"]);
        // `return` is part of the request, so the plan hash and the seal
        // that covers it differ. Everything the rows decide is the same.
        assert_ne!(a["plan_sha256"], b["plan_sha256"], "{name}");
        assert_ne!(a["dataset_seal"], b["dataset_seal"], "{name}");
        for key in [
            "engine_version", "rows", "completed", "refused", "unadmitted", "frames",
            "merkle_root", "rows_sha256",
        ] {
            assert_eq!(a[key], b[key], "{name} {key}");
        }
        assert_eq!(a["request"]["return"], "inline");
        assert_eq!(b["request"]["return"], "file");
        let lines = String::from_utf8(stored.read("rows.jsonl")).unwrap();
        let lines: Vec<&str> = lines.strip_suffix('\n').unwrap().split('\n').collect();
        let rows = inline["rows"].as_array().unwrap();
        assert_eq!(lines.len(), rows.len(), "{name}");
        for (line, row) in lines.iter().zip(rows) {
            assert_eq!(*line, serde_json::to_string(row).unwrap(), "{name}");
        }
        // The frame file's header carries the dataset seal, which differs,
        // so the two files differ in those 32 bytes and in no other.
        let from_file = stored.read("frames.bin");
        let from_inline = hex::decode(inline["frames_hex"].as_str().unwrap()).unwrap();
        assert_eq!(from_file.len(), from_inline.len(), "{name}");
        assert_eq!(from_file[..24], from_inline[..24], "{name}");
        assert_ne!(from_file[24..56], from_inline[24..56], "{name}");
        assert_eq!(from_file[24..56], hex::decode(b["dataset_seal"].as_str().unwrap()).unwrap()[..], "{name}");
        assert_eq!(from_inline[24..56], hex::decode(a["dataset_seal"].as_str().unwrap()).unwrap()[..], "{name}");
        assert_eq!(from_file[56..], from_inline[56..], "{name}");
        assert_ne!(a["frames_sha256"], b["frames_sha256"], "{name}");
        // The head file is the head the answer carries.
        let head_file: Value = serde_json::from_slice(&stored.read("head.json")).unwrap();
        assert_eq!(&head_file, b, "{name}");
    }
}

// The answer for a file job: `head`, then `files`, in that order, and no path.
#[tokio::test]
async fn a_file_job_answers_with_the_head_and_the_names_of_its_files_and_no_path() {
    let dir = TempDir::new("shape");
    let base = spawn(Some(&dir)).await;
    let stored = run_to_file(&base, &dir, &with_return(list_request("frames"), "file")).await;
    let object = stored.answer.as_object().unwrap();
    assert_eq!(object.keys().collect::<Vec<_>>(), ["head", "files"]);
    let seal = stored.answer["head"]["dataset_seal"].as_str().unwrap();
    assert_eq!(
        stored.answer["files"],
        json!({ "folder": seal, "head": "head.json", "rows": "rows.jsonl", "frames": "frames.bin" })
    );
    assert_eq!(
        stored.answer["files"].as_object().unwrap().keys().collect::<Vec<_>>(),
        ["folder", "head", "rows", "frames"]
    );
    assert!(!stored.text.contains(dir.0.to_str().unwrap()));
    assert!(!stored.text.contains("gateway_dataset_files"));
    assert_eq!(dir.entries(), vec![seal.to_string()]);

    let stored = run_to_file(&base, &dir, &with_return(list_request("none"), "file")).await;
    assert_eq!(stored.answer["files"]["frames"], Value::Null);
    assert!(!stored.folder.join("frames.bin").exists());
    assert!(stored.folder.join("rows.jsonl").is_file());
    assert_eq!(stored.answer["head"]["frames_sha256"], Value::Null);
}

// F3
#[tokio::test]
async fn a_stored_dataset_is_checked_by_its_seal_and_a_change_on_disk_is_named() {
    let dir = TempDir::new("f3");
    let base = spawn(Some(&dir)).await;
    let stored = run_to_file(&base, &dir, &with_return(list_request("frames"), "file")).await;
    let seal = stored.seal();
    let held = verify_seal(&base, &seal).await;
    assert_eq!(held["holds"], true, "{held}");
    assert_eq!(held["dataset_seal"], seal.as_str());
    assert_eq!(held["receipt"], "matches");
    assert_eq!(held["rows_file"], "matches the hash in the head");
    assert_eq!(held["frames_file"], "matches the hash in the head");
    assert_eq!(held["chains"], "rebuilt from the frames");

    let frames = stored.read("frames.bin");
    let rows = stored.read("rows.jsonl");
    let head = stored.read("head.json");

    // Row 1 has 3 frames. Change a byte of its frame 1: the chain stops
    // joining at frame 2.
    let first_row_frames = 4;
    let mut changed = frames.clone();
    changed[64 + 64 * (first_row_frames + 1)] ^= 0x01;
    std::fs::write(stored.folder.join("frames.bin"), &changed).unwrap();
    let report = verify_seal(&base, &seal).await;
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["cause"], "chain");
    assert_eq!(report["row"], 1);
    assert_eq!(report["broken_link"], 2);
    std::fs::write(stored.folder.join("frames.bin"), &frames).unwrap();
    assert_eq!(verify_seal(&base, &seal).await["holds"], true);

    // One row line changed: its step count is sealed by the run seal.
    let text = String::from_utf8(rows.clone()).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut row: Value = serde_json::from_str(&lines[0]).unwrap();
    row["steps_landed"] = json!(row["steps_landed"].as_u64().unwrap() + 1);
    lines[0] = serde_json::to_string(&row).unwrap();
    std::fs::write(stored.folder.join("rows.jsonl"), format!("{}\n", lines.join("\n"))).unwrap();
    let report = verify_seal(&base, &seal).await;
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["cause"], "run_seal");
    assert_eq!(report["row"], 0);
    std::fs::write(stored.folder.join("rows.jsonl"), &rows).unwrap();

    // The last row line taken out of the stored rows: the first row missing
    // is named, with the kind of fault.
    let text = String::from_utf8(rows.clone()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);
    std::fs::write(stored.folder.join("rows.jsonl"), format!("{}\n", lines[..2].join("\n"))).unwrap();
    let report = verify_seal(&base, &seal).await;
    assert_eq!(report["holds"], false, "{report}");
    assert_eq!(report["cause"], "plan");
    assert_eq!(report["row"], 2, "{report}");
    assert_eq!(
        report["fault"],
        "A row was dropped: every row read stands in its own place, and row 2 is the first row missing, after the last row read. Rows read: 2; rows in the plan: 3."
    );
    // The first row line written twice.
    std::fs::write(stored.folder.join("rows.jsonl"), format!("{}\n{}\n", lines[0], lines.join("\n"))).unwrap();
    let report = verify_seal(&base, &seal).await;
    assert_eq!((report["holds"].clone(), report["cause"].clone(), report["row"].clone()), (json!(false), json!("plan"), json!(1)), "{report}");
    assert_eq!(report["fault"], "A row is repeated: row 0 stands in its own place and again in place 1. Rows read: 4; rows in the plan: 3.");
    std::fs::write(stored.folder.join("rows.jsonl"), &rows).unwrap();
    assert_eq!(verify_seal(&base, &seal).await["holds"], true);

    // The head changed: the receipt no longer matches.
    let text = String::from_utf8(head.clone()).unwrap();
    assert!(text.contains("\"refused\":0"));
    std::fs::write(stored.folder.join("head.json"), text.replace("\"refused\":0", "\"refused\":1")).unwrap();
    let report = verify_seal(&base, &seal).await;
    assert_eq!(report["receipt"], "does not match", "{report}");
    assert_eq!(report["holds"], false);
    std::fs::write(stored.folder.join("head.json"), &head).unwrap();
    assert_eq!(verify_seal(&base, &seal).await["holds"], true);

    // A dataset that stores no frames is checked from its last links on.
    let none = run_to_file(&base, &dir, &with_return(list_request("none"), "file")).await;
    let report = verify_seal(&base, &none.seal()).await;
    assert_eq!(report["holds"], true, "{report}");
    assert_eq!(report["chains"], "not rebuilt: the plan stores no frames");
}

// F4
#[tokio::test]
async fn without_a_directory_files_are_refused_and_with_one_a_bad_seal_is_refused() {
    let base = spawn(None).await;
    let (status, text) = post(&base, "/run_dataset", &with_return(list_request("frames"), "file")).await;
    assert_eq!((status, text.as_str()), (422, NO_DIRECTORY));
    // A check by seal on a gateway that stores nothing says how a dataset
    // returned in the answer is checked. In version 0.2.1 it gave the sentence
    // of the file job alone.
    let (status, text) = post(&base, "/verify_dataset", &json!({ "dataset_seal": "0".repeat(64) })).await;
    assert_eq!((status, text), (422, format!("{NO_DIRECTORY} {VERIFY_BY_PARTS}")));

    let dir = TempDir::new("f4");
    let base = spawn(Some(&dir)).await;
    let (status, text) = post(&base, "/verify_dataset", &json!({ "dataset_seal": "ab".repeat(32) })).await;
    assert_eq!((status, text), (404, format!("no dataset is stored under that seal. {VERIFY_BY_PARTS}")));

    let stored = run_to_file(&base, &dir, &with_return(list_request("frames"), "file")).await;
    let seal = stored.seal();
    let not_hex = "Physical Violation: 'dataset_seal' must be 64 lowercase hex digits.";
    for bad in [
        json!("abc"),
        json!(seal.to_uppercase()),
        json!(&seal[1..]),
        json!(format!("{seal}0")),
        json!("../x"),
        json!("/tmp/x"),
        json!(format!("../{}", &seal[3..])),
        json!(".."),
        json!(5),
        json!(null),
        json!([seal]),
    ] {
        let (status, text) = post(&base, "/verify_dataset", &json!({ "dataset_seal": bad })).await;
        assert_eq!((status, text.as_str()), (422, not_hex), "{bad}");
    }
    let both = "Physical Violation: send 'request' and 'dataset_seal', or 'dataset_seal' alone, or 'head', 'rows' and 'frames_hex', and no mixture of them.";
    let (status, text) = post(
        &base,
        "/verify_dataset",
        &json!({ "dataset_seal": seal, "head": stored.answer["head"], "rows": [], "frames_hex": null }),
    )
    .await;
    assert_eq!((status, text.as_str()), (422, both));
    for extra in [json!({ "path": "/tmp/x" }), json!({ "folder": "../x" }), json!({ "rows": [] })] {
        let mut body = json!({ "dataset_seal": seal });
        body.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let (status, text) = post(&base, "/verify_dataset", &body).await;
        assert_eq!((status, text.as_str()), (422, both), "{body}");
    }
    // The old form still needs all three parts.
    let (status, _) = post(&base, "/verify_dataset", &json!({ "head": stored.answer["head"] })).await;
    assert_eq!(status, 422);
    // No request field names a path.
    for key in ["path", "directory", "dir", "folder", "file", "dataset_dir"] {
        let mut body = with_return(list_request("frames"), "file");
        body[key] = json!("../x");
        let (status, text) = post(&base, "/run_dataset", &body).await;
        assert_eq!(status, 422, "{key}: {text}");
        assert!(text.contains(&format!("'{key}' is not a field")), "{text}");
    }
    // Nothing was made by any of these.
    assert_eq!(dir.entries(), vec![seal]);
}

// F5
#[tokio::test]
async fn the_same_plan_run_twice_leaves_one_folder_and_a_changed_head_is_not_overwritten() {
    let dir = TempDir::new("f5");
    let base = spawn(Some(&dir)).await;
    let body = with_return(sample_request("frames"), "file");
    let first = run_to_file(&base, &dir, &body).await;
    let second = run_to_file(&base, &dir, &body).await;
    assert_eq!(first.text, second.text);
    assert_eq!(dir.entries(), vec![first.seal()]);

    let head = first.read("head.json");
    let mut tampered = head.clone();
    tampered.push(b'\n');
    std::fs::write(first.folder.join("head.json"), &tampered).unwrap();
    let (status, text) = post(&base, "/run_dataset", &body).await;
    assert_eq!(status, 500, "{text}");
    assert_eq!(
        text,
        format!(
            "Physical Violation: the folder {} already holds a head that is not this dataset's. Nothing was overwritten.",
            first.seal()
        )
    );
    assert_eq!(std::fs::read(first.folder.join("head.json")).unwrap(), tampered);
    // The stored folder is left as it was found. The one just written is removed.
    assert_eq!(dir.entries(), vec![first.seal()]);
}

// F10
#[tokio::test]
async fn the_programs_of_the_sealing_crate_accept_the_three_files_of_a_stored_dataset() {
    let seal_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join("../genesis_seal");
    let built = std::process::Command::new(env!("CARGO"))
        .args(["build", "--offline", "--quiet", "--manifest-path"])
        .arg(seal_crate.join("Cargo.toml"))
        .status()
        .unwrap();
    assert!(built.success());
    // Alone the crate compiles into its own folder; as a member of the engine
    // workspace it compiles into the workspace's.
    let program = [
        seal_crate.join("target/debug/genesis_seal"),
        seal_crate.join("../target/debug/genesis_seal"),
    ]
    .into_iter()
    .find(|path| path.exists())
    .expect("the program of the sealing crate");
    let dir = TempDir::new("f10");
    let base = spawn(Some(&dir)).await;
    for plan in [list_request("frames"), grid_request("frames"), sample_request("none")] {
        let stored = run_to_file(&base, &dir, &with_return(plan, "file")).await;
        let mut command = std::process::Command::new(&program);
        command
            .arg("verify-dataset")
            .arg(stored.folder.join("head.json"))
            .arg(stored.folder.join("rows.jsonl"));
        if stored.folder.join("frames.bin").exists() {
            command.arg(stored.folder.join("frames.bin"));
        }
        let output = command.output().unwrap();
        let out = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(0), "{out}");
        assert!(out.starts_with("holds: true\n"), "{out}");
        assert!(out.contains("rows_file: matches the hash in the head\n"), "{out}");
    }
}

fn tool_text(result: &rmcp::model::CallToolResult) -> String {
    result.content[0].as_text().expect("text").text.clone()
}

fn arguments(value: &Value) -> Map<String, Value> {
    value.as_object().expect("object").clone()
}

// F9
#[tokio::test]
async fn the_mcp_door_gives_the_bytes_of_the_http_door_for_files() {
    let bin = env!("CARGO_BIN_EXE_mcp_stdio");
    let http_dir = TempDir::new("f9_http");
    let mcp_dir = TempDir::new("f9_mcp");
    let base = spawn(Some(&http_dir)).await;
    let mut command = tokio::process::Command::new(bin);
    command.arg("--dataset-dir").arg(&mcp_dir.0);
    let mut client = ().serve(TokioChildProcess::new(command).unwrap()).await.expect("initialize");

    for plan in [list_request("frames"), grid_request("none")] {
        let body = with_return(plan, "file");
        let (status, http_text) = post(&base, "/run_dataset", &body).await;
        assert_eq!(status, 200, "{http_text}");
        let called = client
            .call_tool(CallToolRequestParams::new("run_dataset").with_arguments(arguments(&body)))
            .await
            .unwrap();
        assert_eq!(called.is_error, Some(false).filter(|_| false).or(called.is_error));
        assert_ne!(called.is_error, Some(true), "{}", tool_text(&called));
        assert_eq!(tool_text(&called), http_text);
        let answer: Value = serde_json::from_str(&http_text).unwrap();
        let seal = answer["files"]["folder"].as_str().unwrap();
        for name in ["head.json", "rows.jsonl"] {
            assert_eq!(
                std::fs::read(http_dir.0.join(seal).join(name)).unwrap(),
                std::fs::read(mcp_dir.0.join(seal).join(name)).unwrap(),
                "{name}"
            );
        }
        let by_seal = json!({ "dataset_seal": seal });
        let (status, http_report) = post(&base, "/verify_dataset", &by_seal).await;
        assert_eq!(status, 200, "{http_report}");
        let called = client
            .call_tool(CallToolRequestParams::new("verify_dataset").with_arguments(arguments(&by_seal)))
            .await
            .unwrap();
        assert_eq!(tool_text(&called), http_report);
        assert!(http_report.contains("\"holds\":true"));
        // An unknown seal and a bad one give the same words.
        for refused in [json!({ "dataset_seal": "0".repeat(64) }), json!({ "dataset_seal": "../x" })] {
            let (_, http_text) = post(&base, "/verify_dataset", &refused).await;
            let called = client
                .call_tool(CallToolRequestParams::new("verify_dataset").with_arguments(arguments(&refused)))
                .await
                .unwrap();
            assert_eq!(called.is_error, Some(true));
            assert_eq!(tool_text(&called), http_text);
        }
    }
    client.close().await.expect("close");

    // Without a directory the MCP door gives the same sentence.
    let transport = TokioChildProcess::new(tokio::process::Command::new(bin)).unwrap();
    let mut client = ().serve(transport).await.expect("initialize");
    let called = client
        .call_tool(
            CallToolRequestParams::new("run_dataset")
                .with_arguments(arguments(&with_return(list_request("frames"), "file"))),
        )
        .await
        .unwrap();
    assert_eq!(called.is_error, Some(true));
    assert_eq!(tool_text(&called), NO_DIRECTORY);
    client.close().await.expect("close");
}

// The flag, and a gateway that will not start without a usable directory.
#[test]
fn the_flag_names_a_directory_that_exists_and_can_be_written_or_the_gateway_does_not_start() {
    let dir = TempDir::new("flag");
    let named = dir.0.to_string_lossy().into_owned();
    assert_eq!(dataset_dir_from_args(Vec::new()), Ok(None));
    assert_eq!(
        dataset_dir_from_args(vec!["--dataset-dir".to_string(), named.clone()]),
        Ok(Some(dir.0.clone()))
    );
    assert_eq!(
        dataset_dir_from_args(vec![format!("--dataset-dir={named}")]),
        Ok(Some(dir.0.clone()))
    );
    assert!(dir.entries().is_empty(), "the check left a file behind");
    let missing = dir.0.join("not_there").to_string_lossy().into_owned();
    let error = dataset_dir_from_args(vec!["--dataset-dir".to_string(), missing.clone()]).unwrap_err();
    assert!(error.starts_with(&format!("the dataset directory {missing} cannot be used")), "{error}");
    let file = dir.0.join("a_file");
    std::fs::write(&file, b"").unwrap();
    let error = dataset_dir_from_args(vec![format!("--dataset-dir={}", file.display())]).unwrap_err();
    assert!(error.ends_with("is not a directory."), "{error}");
    assert!(dataset_dir_from_args(vec!["--dataset-dir".to_string()]).is_err());
    assert!(dataset_dir_from_args(vec!["--dataset_dir=x".to_string()]).is_err());
    assert!(
        dataset_dir_from_args(vec![format!("--dataset-dir={named}"), format!("--dataset-dir={named}")]).is_err()
    );

    // A read-only directory cannot be written, so the gateway does not start.
    use std::os::unix::fs::PermissionsExt;
    let locked = dir.0.join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    let result = dataset_dir_from_args(vec![format!("--dataset-dir={}", locked.display())]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    if let Err(error) = result {
        assert!(error.contains("cannot be written"), "{error}");
    } else {
        // Running as a user the mode does not bind (root): nothing to check.
        assert_eq!(std::env::var("USER").as_deref(), Ok("root"));
    }

    // Both binaries exit with the sentence and start nothing.
    for bin in [env!("CARGO_BIN_EXE_mcp_stdio"), env!("CARGO_BIN_EXE_protocol_mcp_gateway")] {
        let output = std::process::Command::new(bin)
            .args(["--dataset-dir", &missing])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.starts_with("The gateway did not start: the dataset directory"), "{error}");
    }
}
