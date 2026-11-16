//! The engine's signature on its answers, end to end: every computing tool's
//! answer is signed on the local connection and on the open hosted one, it
//! verifies, and each attack on it fails by name at verify_tape,
//! verify_solution and verify_dataset: a forged signature, an expired one, a
//! nonce seen before, a value changed after signing, the wrong key. A door
//! without a key refuses to start by name, a key file others can read is
//! refused, and GET /answer-key gives the public key. The seeds here are
//! test keys, labelled as such; no seed is ever printed.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use genesis_seal::answer_signature::{NONCE_LEN, SEED_LEN, SigningKey, split};
use serde_json::{Value, json};
use sha2::{Digest, Sha384};

const STDIO: &str = env!("CARGO_BIN_EXE_mcp_stdio");
const HOSTED: &str = env!("CARGO_BIN_EXE_hosted_door");
const ANSWER_KEY: &str = env!("CARGO_BIN_EXE_answer_key");

fn seed_hex(label: &str) -> String {
    let digest = Sha384::digest(format!("a test answer key, never an operator's: {label}").as_bytes());
    hex::encode(&digest[..SEED_LEN])
}

fn test_key(label: &str) -> SigningKey {
    SigningKey::from_seed_hex(&seed_hex(label)).unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("answer_signature_router").join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// A key file of the test key `label`, readable by its owner alone.
fn key_file(dir: &Path, label: &str) -> String {
    let path = dir.join(format!("{label}.key"));
    std::fs::write(&path, format!("{}\n", seed_hex(label))).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    path.to_string_lossy().into_owned()
}

/// The local connection, one JSON-RPC line at a time.
struct Door {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Door {
    fn start(arguments: &[&str]) -> Door {
        let mut child = Command::new(STDIO)
            .args(arguments)
            .env_remove("ZTP_SIGNING_KEY")
            .env_remove("ZTP_SIGNING_KEY_FILE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut door = Door { child, stdin, stdout, next: 1 };
        door.line(r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#);
        door.read();
        door.line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        door
    }

    fn line(&mut self, text: &str) {
        self.stdin.write_all(text.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    /// A tool call whose arguments are the exact text given; its answer text.
    fn call(&mut self, tool: &str, arguments: &str) -> (bool, String) {
        let id = self.next;
        self.next += 1;
        self.line(&format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#));
        let answer = self.read();
        let result = &answer["result"];
        let text = result["content"][0]["text"].as_str().unwrap_or_else(|| panic!("{tool}: {answer}")).to_string();
        (result["isError"].as_bool().unwrap_or(false), text)
    }

    fn stderr(mut self) -> String {
        drop(self.stdin);
        let _ = self.child.wait();
        let mut said = String::new();
        self.child.stderr.take().unwrap().read_to_string(&mut said).unwrap();
        said
    }
}

const RUN: &str = r#"{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":10,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}"#;
const STEP: &str = r#"{"domain":"energy","card":"thermal","parameters":{"temperature_k":293.15,"heat_power_w":1.0,"ambient_k":293.15,"dt_s":0.1}}"#;
const SOLVE: &str = r#"{"solver":"atom_dft","parameters":{"atomic_number":2}}"#;
const DATASET: &str = r#"{"domain":"energy","card":"thermal","plan":{"kind":"list","runs":[{"initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":5,"load":{"heat_power_w":1.0,"ambient_k":293.15}}],"record_every":1}]},"store":"frames","return":"inline","answer":"full"}"#;
const VALIDATE: &str = r#"{"run":{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":0.1,"segments":[{"steps":5,"load":{"heat_power_w":5.0,"ambient_k":293.15}}],"record_every":1},"quantity":"temperature_k","measurements":[{"step":1,"value":293.2},{"step":3,"value":293.3},{"step":5,"value":293.4}]}"#;
const DESIGN: &str = r#"{"request":{"domain":"energy","card":"thermal","initial":{"temperature_k":293.15},"dt_s":1.0,"segments":[{"steps":10,"load":{"heat_power_w":5.0,"ambient_k":293.15}}],"record_every":10},"vary":[{"place":"segments[0].load.heat_power_w","low":0.0,"high":50.0}],"target":{"quantity":"temperature_k","value":293.5,"tolerance":0.01}}"#;

fn with_fields(answer: &str, fields: &Value) -> String {
    let extra = serde_json::to_string(fields).unwrap();
    format!("{},{}", &answer[..answer.len() - 1], &extra[1..])
}

fn verdict(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|error| panic!("{error}: {text}"))
}

fn public(label: &str) -> String {
    test_key(label).public_key().to_base64()
}

/// The answer re-signed by the same key with another issue time or nonce.
fn re_signed(answer: &str, issued_s: i64, nonce: u8) -> String {
    let unsigned = split(answer).unwrap().unwrap().unsigned;
    test_key("engine").sign_text(&unsigned, issued_s, 3600, &[nonce; NONCE_LEN]).unwrap()
}

#[test]
fn every_computing_tool_signs_its_answer_and_each_attack_fails_by_name() {
    let dir = scratch("stdio");
    let key = key_file(&dir, "engine");
    let mut door = Door::start(&["--signing-key", &key]);
    let key_id = test_key("engine").public_key().key_id();
    let mut answers = Vec::new();
    for (tool, arguments) in [
        ("compute_step", STEP),
        ("verify_step", STEP),
        ("run_trajectory", RUN),
        ("solve", SOLVE),
        ("run_dataset", DATASET),
        ("validate_series", VALIDATE),
        ("design_run", DESIGN),
    ] {
        let (is_error, text) = door.call(tool, arguments);
        assert!(!is_error, "{tool}: {text}");
        let found = split(&text).unwrap().unwrap_or_else(|| panic!("{tool}: the answer carries no signature"));
        let envelope = found.envelope.unwrap();
        assert_eq!(envelope.alg, "ML-DSA-87", "{tool}");
        assert_eq!(envelope.key_id, key_id, "{tool}");
        let report = genesis_seal::answer_signature::check(
            &text,
            genesis_seal::answer_signature::Check {
                public_key: Some(test_key("engine").public_key()),
                now_s: genesis_seal::answer_signature::now_s(),
                seen_nonces: &[],
            },
        );
        assert_eq!(report.cause, None, "{tool}");
        assert!(text.contains("\"seal_rule\":\"zero-trust-physics/seal/v2\",\"receipt_sha384\":\""), "{tool}");
        answers.push((tool, text));
    }
    // Two deliveries of one request: the same bytes through the receipt, a new signature.
    let (_, again) = door.call("run_trajectory", RUN);
    let run = &answers[2].1;
    assert_eq!(split(&again).unwrap().unwrap().unsigned, split(run).unwrap().unwrap().unsigned);
    assert_ne!(again, *run);

    let solution = &answers[3].1;
    let dataset = &answers[4].1;
    let dataset_parts = {
        let body: Value = serde_json::from_str(dataset).unwrap();
        // The long form of a dataset is its three parts, as the answer carries them.
        let head_at = dataset.find("\"head\":").unwrap() + 7;
        let mut stream = serde_json::Deserializer::from_str(&dataset[head_at..]).into_iter::<Value>();
        stream.next().unwrap().unwrap();
        let head_end = head_at + stream.byte_offset();
        format!(
            r#"{{"head":{},"rows":{},"frames_hex":{}}}"#,
            &dataset[head_at..head_end],
            serde_json::to_string(&body["rows"]).unwrap(),
            serde_json::to_string(&body["frames_hex"]).unwrap()
        )
    };
    let now = genesis_seal::answer_signature::now_s();
    for (tool, answer) in [("verify_tape", run.clone()), ("verify_solution", solution.clone()), ("verify_dataset", dataset_parts.clone())] {
        // It holds, every byte, with the signature reported.
        let (is_error, text) = door.call(tool, &answer);
        assert!(!is_error, "{tool}: {text}");
        let said = verdict(&text);
        assert_eq!(said["holds"], true, "{tool}: {text}");
        assert_eq!(said["signature"]["holds"], true, "{tool}: {text}");
        let named = |door: &mut Door, arguments: &str| {
            let (is_error, text) = door.call(tool, arguments);
            assert!(!is_error, "{tool}: {text}");
            let said = verdict(&text);
            assert_eq!((said["holds"].clone(), said["cause"].clone()), (json!(false), json!("signature")), "{tool}: {text}");
            said["signature"]["cause"].as_str().unwrap().to_string()
        };
        // A forged signature: another key's value under this key's id.
        let mine = split(&answer).unwrap().unwrap().envelope.unwrap();
        let unsigned = split(&answer).unwrap().unwrap().unsigned;
        let theirs = test_key("another").sign_text(&unsigned, now, 3600, &[9; NONCE_LEN]).unwrap();
        let forged = answer.replace(&mine.value, &split(&theirs).unwrap().unwrap().envelope.unwrap().value);
        assert_eq!(named(&mut door, &forged), "SignatureDoesNotVerify", "{tool}");
        // An expired signature: the same key, issued two hours back.
        assert_eq!(named(&mut door, &re_signed(&answer, now - 7200, 7)), "SignatureExpired", "{tool}");
        // A nonce the caller has seen.
        let seen = with_fields(&answer, &json!({ "seen_nonces": [mine.nonce] }));
        assert_eq!(named(&mut door, &seen), "SignatureNonceReplayed", "{tool}");
        // A value changed after signing: the engine version, which every answer carries.
        let tampered = answer.replacen("genesis_", "genesiS_", 1);
        assert_ne!(tampered, answer);
        assert_eq!(named(&mut door, &tampered), "ReceiptDoesNotMatch", "{tool}");
        // The wrong key.
        let wrong = with_fields(&answer, &json!({ "public_key": public("another") }));
        assert_eq!(named(&mut door, &wrong), "SignatureKeyMismatch", "{tool}");
        // The right key given: it holds.
        let right = with_fields(&answer, &json!({ "public_key": public("engine") }));
        let (_, text) = door.call(tool, &right);
        assert_eq!(verdict(&text)["holds"], true, "{tool}: {text}");
        // An answer with no signature and a key given.
        let absent = with_fields(&unsigned, &json!({ "public_key": public("engine") }));
        assert_eq!(named(&mut door, &absent), "SignatureAbsent", "{tool}");
    }
    // verify_receipt reads the receipt of a signed answer and reports its signature.
    let (_, text) = door.call("verify_receipt", &serde_json::to_string(&json!({ "response_text": run })).unwrap());
    let said = verdict(&text);
    assert_eq!((said["matches"].clone(), said["signature"]["holds"].clone()), (json!(true), json!(true)), "{text}");
    let said = door.stderr();
    assert!(said.contains(&key_id), "{said}");
    assert!(!said.contains(&seed_hex("engine")), "the seed is never printed");
}

/// An answer written again the way a JSON client writes it: a space after
/// every colon and every comma, the values and their order kept.
fn written_again(answer: &str) -> String {
    struct Spaced;
    impl serde_json::ser::Formatter for Spaced {
        fn begin_object_key<W: ?Sized + Write>(&mut self, writer: &mut W, first: bool) -> std::io::Result<()> {
            if first { Ok(()) } else { writer.write_all(b", ") }
        }
        fn begin_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
            writer.write_all(b": ")
        }
        fn begin_array_value<W: ?Sized + Write>(&mut self, writer: &mut W, first: bool) -> std::io::Result<()> {
            if first { Ok(()) } else { writer.write_all(b", ") }
        }
    }
    let value: Value = serde_json::from_str(answer).unwrap();
    let mut out = Vec::new();
    serde::Serialize::serialize(&value, &mut serde_json::Serializer::with_formatter(&mut out, Spaced)).unwrap();
    String::from_utf8(out).unwrap()
}

/// 0.15.1: a signed answer sent back byte for byte holds; sent again with
/// its nonce seen it is refused SignatureNonceReplayed; written again by a
/// JSON client (spaces after colons and commas) its signature is verified
/// over the receipt it carries, never reported absent, the receipt is named
/// as not covering the bytes sent (ReceiptDoesNotMatch, written_again), and
/// the answer is computed again: values_hold true, holds false, differs_at
/// the first place its text differs. A replay, another key or a value
/// changed are named on the copy written again as on the answer as it came.
#[test]
fn a_signed_answer_written_again_by_a_client_names_the_true_cause() {
    let dir = scratch("written_again");
    let key = key_file(&dir, "engine");
    let mut door = Door::start(&["--signing-key", &key]);
    let (_, run) = door.call("run_trajectory", RUN);
    let (_, solution) = door.call("solve", SOLVE);
    let (_, dataset) = door.call("run_dataset", DATASET);
    let dataset_parts = {
        let body: Value = serde_json::from_str(&dataset).unwrap();
        let head_at = dataset.find("\"head\":").unwrap() + 7;
        let mut stream = serde_json::Deserializer::from_str(&dataset[head_at..]).into_iter::<Value>();
        stream.next().unwrap().unwrap();
        let head_end = head_at + stream.byte_offset();
        format!(
            r#"{{"head":{},"rows":{},"frames_hex":{}}}"#,
            &dataset[head_at..head_end],
            serde_json::to_string(&body["rows"]).unwrap(),
            serde_json::to_string(&body["frames_hex"]).unwrap()
        )
    };
    // 0.15.0 read a dataset written again with its head's signature unread,
    // compared the engine's text with its own writing of the values, and
    // said holds true: the bytes sent are now the bytes compared.
    for (tool, answer) in [("verify_tape", run), ("verify_solution", solution), ("verify_dataset", dataset_parts)] {
        let nonce = split(&answer).unwrap().unwrap().envelope.unwrap().nonce;
        // Byte for byte: it holds, and so with the key given.
        let (_, text) = door.call(tool, &answer);
        assert_eq!((verdict(&text)["holds"].clone(), verdict(&text)["signature"]["holds"].clone()), (json!(true), json!(true)), "{tool}: {text}");
        // Its nonce seen: refused by name, nothing computed.
        let (_, text) = door.call(tool, &with_fields(&answer, &json!({ "seen_nonces": [nonce] })));
        let said = verdict(&text);
        assert_eq!((said["cause"].clone(), said["signature"]["cause"].clone()), (json!("signature"), json!("SignatureNonceReplayed")), "{tool}: {text}");

        let again = written_again(&answer);
        assert_ne!(again, answer);
        assert_eq!(split(&again).unwrap(), None, "{tool}: the copy is not in the engine's form");
        for arguments in [again.clone(), with_fields(&again, &json!({ "public_key": public("engine") }))] {
            let (is_error, text) = door.call(tool, &arguments);
            assert!(!is_error, "{tool}: {text}");
            let said = verdict(&text);
            assert_eq!(said["holds"], false, "{tool}: {text}");
            assert_eq!(said["values_hold"], true, "{tool}: {text}");
            assert_eq!(said["cause"], "answer_text", "{tool}: {text}");
            assert!(said["differs_at"].is_string(), "{tool}: {text}");
            assert_eq!(said["signature"]["cause"], "ReceiptDoesNotMatch", "{tool}: {text}");
            assert_eq!(said["signature"]["holds"], false, "{tool}: {text}");
            assert_eq!(said["signature"]["written_again"], true, "{tool}: {text}");
            assert_eq!(said["signature"]["nonce"], json!(nonce), "{tool}: {text}");
            assert_ne!(said["signature"]["cause"], "SignatureAbsent", "{tool}: {text}");
        }
        let named = |door: &mut Door, arguments: &str| {
            let (_, text) = door.call(tool, arguments);
            let said = verdict(&text);
            assert_eq!(said["cause"], "signature", "{tool}: {text}");
            said["signature"]["cause"].as_str().unwrap().to_string()
        };
        assert_eq!(named(&mut door, &written_again(&with_fields(&answer, &json!({ "seen_nonces": [nonce] })))), "SignatureNonceReplayed", "{tool}");
        assert_eq!(named(&mut door, &written_again(&with_fields(&answer, &json!({ "public_key": public("another") })))), "SignatureKeyMismatch", "{tool}");
        let expired = written_again(&re_signed(&answer, genesis_seal::answer_signature::now_s() - 7200, 5));
        assert_eq!(named(&mut door, &expired), "SignatureExpired", "{tool}");
        // A value changed on the copy: the signature still verifies over the
        // receipt; the bytes are named, and the computation finds the value.
        let changed = again.replacen("genesis_", "genesiS_", 1);
        let (_, text) = door.call(tool, &changed);
        let said = verdict(&text);
        assert_eq!((said["holds"].clone(), said["values_hold"].clone()), (json!(false), json!(false)), "{tool}: {text}");
        assert_eq!(said["signature"]["cause"], "ReceiptDoesNotMatch", "{tool}: {text}");
    }
}

/// 0.15.1: a signed chained solve sent whole with its `from`, as
/// verify_solution's long form takes it, holds: the caller's `from` stands
/// after the signature and is not of the signed answer, so the signature is
/// verified without it (0.15.0 named ReceiptDoesNotMatch). Its nonce seen,
/// it is refused by name; written again by a client, the bytes are named
/// and the values hold.
#[test]
fn a_signed_chained_solve_sent_whole_with_its_from_holds() {
    let dir = scratch("chained");
    let key = key_file(&dir, "engine");
    let mut door = Door::start(&["--signing-key", &key]);
    let up = json!({
        "domain": "energy", "card": "reactor_triga_jsi",
        "initial": { "start_temperature_k": 296.15, "rod_table_core": 231.0 },
        "dt_s": 5.0e-5,
        "segments": [{ "steps": 4000, "load": { "reactivity_inserted_dollar": 2.5, "rods_dropped": 0.0 } }],
        "record_every": 200
    });
    let (is_error, card) = door.call("run_trajectory", &up.to_string());
    assert!(!is_error, "{card}");
    let card: Value = serde_json::from_str(&card).unwrap();
    let mut times = Vec::new();
    let mut energy = Vec::new();
    for frame in card["frames"].as_array().unwrap() {
        let bytes = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap();
        times.push(f64::from_le_bytes(bytes[0..8].try_into().unwrap()));
        energy.push(f64::from(f32::from_le_bytes(bytes[12..16].try_into().unwrap())));
    }
    let from = json!({ "request": up, "run_seal": card["run_seal"], "coupling": "triga_pulse_to_element_6753" });
    let request = json!({
        "solver": "fuel_rod_triga_jsi",
        "parameters": { "start_temperature_k": 296.15, "time_s": times, "core_energy_released_mws": energy },
        "from": from
    });
    let (is_error, chained) = door.call("solve", &request.to_string());
    assert!(!is_error, "{chained}");
    let nonce = split(&chained).unwrap().unwrap().envelope.unwrap().nonce;
    let whole = format!("{},\"from\":{}}}", &chained[..chained.len() - 1], from);
    for arguments in [whole.clone(), with_fields(&whole, &json!({ "public_key": public("engine") }))] {
        let (_, text) = door.call("verify_solution", &arguments);
        let said = verdict(&text);
        assert_eq!((said["holds"].clone(), said["values_hold"].clone(), said["cause"].clone()), (json!(true), json!(true), Value::Null), "{text}");
        assert_eq!(said["signature"]["holds"], true, "{text}");
    }
    let (_, text) = door.call("verify_solution", &with_fields(&whole, &json!({ "seen_nonces": [nonce] })));
    assert_eq!(verdict(&text)["signature"]["cause"], "SignatureNonceReplayed", "{text}");
    let (_, text) = door.call("verify_solution", &written_again(&whole));
    let said = verdict(&text);
    assert_eq!((said["holds"].clone(), said["values_hold"].clone()), (json!(false), json!(true)), "{text}");
    assert_eq!((said["signature"]["cause"].clone(), said["signature"]["written_again"].clone()), (json!("ReceiptDoesNotMatch"), json!(true)), "{text}");
}

#[test]
fn a_door_without_a_key_refuses_to_start_and_a_key_others_can_read_is_refused() {
    let output = Command::new(HOSTED)
        .args(["--open", "--port", "0"])
        .env_remove("ZTP_SIGNING_KEY")
        .env_remove("ZTP_SIGNING_KEY_FILE")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("SigningKeyNotGiven"), "{said}");

    let dir = scratch("modes");
    let key = key_file(&dir, "engine");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).unwrap();
        let output = Command::new(STDIO).args(["--signing-key", &key]).stdin(Stdio::null()).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        let said = String::from_utf8_lossy(&output.stderr);
        assert!(said.contains("readable by its owner alone"), "{said}");
        assert!(!said.contains(&seed_hex("engine")));
    }
    // answer_key makes a key file of mode 0600 and never overwrites one.
    let made = dir.join("made.key");
    let output = Command::new(ANSWER_KEY).arg("new").arg(&made).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    let description: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(description["alg"], "ML-DSA-87");
    let seed = std::fs::read_to_string(&made).unwrap();
    assert!(!String::from_utf8_lossy(&output.stdout).contains(seed.trim()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&made).unwrap().permissions().mode() & 0o777, 0o600);
    }
    let public = Command::new(ANSWER_KEY).arg("public").arg(&made).output().unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&public.stdout).unwrap(), description);
    assert_eq!(Command::new(ANSWER_KEY).arg("new").arg(&made).output().unwrap().status.code(), Some(2));
}

#[test]
fn the_open_hosted_door_signs_and_serves_its_public_key() {
    let dir = scratch("hosted");
    let key = key_file(&dir, "engine");
    let mut child = Command::new(HOSTED)
        .args(["--open", "--port", "0", "--signing-key", &key])
        .env_remove("ZTP_SIGNING_KEY")
        .env_remove("ZTP_SIGNING_KEY_FILE")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut first).unwrap();
    let address = first.split("running on ").nth(1).unwrap().split_whitespace().next().unwrap().to_string();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (described, answer) = runtime.block_on(async {
        let client = reqwest::Client::new();
        let described: Value = client.get(format!("http://{address}/answer-key")).send().await.unwrap().json().await.unwrap();
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "run_trajectory", "arguments": serde_json::from_str::<Value>(RUN).unwrap() } });
        let body: Value = client
            .post(format!("http://{address}/mcp"))
            .header("accept", "application/json, text/event-stream")
            .header("content-type", "application/json")
            .header("mcp-protocol-version", "2025-06-18")
            .body(call.to_string())
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        (described, body["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string())
    });
    let _ = child.kill();
    assert_eq!(described["key_id"], test_key("engine").public_key().key_id());
    assert_eq!(described["public_key"], public("engine"));
    let envelope = split(&answer).unwrap().unwrap_or_else(|| panic!("unsigned: {answer}")).envelope.unwrap();
    assert_eq!(envelope.key_id, test_key("engine").public_key().key_id());
}
