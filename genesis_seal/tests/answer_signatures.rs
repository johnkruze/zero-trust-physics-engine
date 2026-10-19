//! The engine's signature on an answer, checked by the program with no
//! engine: a forged signature, an expired one, a nonce seen before, a value
//! changed after signing, the wrong key and an answer with none, each named;
//! and a record of seal rule v1, not read.

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use support::{dataset_of, eight_short_runs, needs_saved_runs};

use genesis_seal::answer_signature::{NONCE_LEN, SEED_LEN, SigningKey, split};
use sha2::{Digest, Sha384};

const PROGRAM: &str = env!("CARGO_BIN_EXE_genesis_seal");

/// 2026-10-18T21:04:05Z.
const ISSUED: i64 = 1_792_357_445;
const ISSUED_TEXT: &str = "2026-10-18T21:04:05Z";
const INSIDE: &str = "2026-10-18T21:30:00Z";
const AFTER: &str = "2026-10-18T22:04:06Z";

/// A test answer key made from a labelled seed: never an operator's.
fn test_key(label: &str) -> SigningKey {
    let digest = Sha384::digest(format!("a test answer key, never an operator's: {label}").as_bytes());
    let mut seed = [0_u8; SEED_LEN];
    seed.copy_from_slice(&digest[..SEED_LEN]);
    SigningKey::from_seed(&seed)
}

fn folder(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("answer_signatures").join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("a test folder is made");
    path
}

fn write(folder: &Path, name: &str, bytes: &[u8]) -> String {
    let path = folder.join(name);
    std::fs::write(&path, bytes).expect("a test file is written");
    path.to_string_lossy().into_owned()
}

fn run(arguments: &[&str]) -> (i32, String) {
    let output = Command::new(PROGRAM).args(arguments).output().expect("the program runs");
    (
        output.status.code().expect("the program exits"),
        String::from_utf8(output.stdout).expect("the program prints text"),
    )
}

/// The texts of the chain fixture's two answers, as the engine wrote them.
fn fixture_links() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/chain_thermal.json");
    let text = std::fs::read_to_string(path).expect("the chain fixture");
    let body = &text["{\"links\":[".len()..];
    let mut stream = serde_json::Deserializer::from_str(body).into_iter::<serde_json::Value>();
    stream.next().expect("a first link").expect("JSON");
    let first_end = stream.byte_offset();
    let second = &body[first_end + 1..];
    let mut stream = serde_json::Deserializer::from_str(second).into_iter::<serde_json::Value>();
    stream.next().expect("a second link").expect("JSON");
    vec![body[..first_end].to_string(), second[..stream.byte_offset()].to_string()]
}

fn signed_run(key: &SigningKey, nonce: u8) -> String {
    key.sign_text(&fixture_links()[0], ISSUED, 3600, &[nonce; NONCE_LEN]).expect("the run answer is signed")
}

fn line<'a>(out: &'a str, name: &str) -> Option<&'a str> {
    out.lines().find_map(|line| line.strip_prefix(&format!("{name}: ")))
}

#[test]
fn a_signed_run_holds_with_its_key_and_says_when_it_was_issued() {
    let folder = folder("holds");
    let key = test_key("engine");
    let answer = write(&folder, "run.json", signed_run(&key, 1).as_bytes());
    let public = write(&folder, "engine.pub", key.public_key().to_base64().as_bytes());
    let (code, out) = run(&["verify-run", &answer, "--public-key", &public, "--now", INSIDE]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("holds: true\nbroken_link: none\ncause: none\nreceipt: matches\nsignature: holds\n"), "{out}");
    assert_eq!(line(&out, "key_id"), Some(key.public_key().key_id().as_str()));
    assert_eq!(line(&out, "issued_at"), Some(ISSUED_TEXT));
    assert_eq!(line(&out, "expires_at"), Some("2026-10-18T22:04:05Z"));
    assert_eq!(line(&out, "nonce"), Some("01".repeat(32).as_str()));
    // Without a key the seals are verified and the signature is said not checked.
    let (code, out) = run(&["verify-run", &answer]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(line(&out, "signature"), Some("not checked"));
    // verify-signature checks the signature and the receipt alone.
    let (code, out) = run(&["verify-signature", &answer, "--public-key", &public, "--now", INSIDE]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("receipt: matches\nsignature: holds\n"), "{out}");
}

#[test]
fn each_attack_on_a_signed_answer_fails_by_its_name() {
    let folder = folder("attacks");
    let key = test_key("engine");
    let other = test_key("another engine");
    let public = write(&folder, "engine.pub", key.public_key().to_base64().as_bytes());
    let other_public = write(&folder, "other.pub", other.public_key().to_base64().as_bytes());
    let signed = signed_run(&key, 2);
    let mine = split(&signed).unwrap().unwrap().envelope.unwrap();

    let named = |name: &str, text: &str, extra: &[&str]| {
        let path = write(&folder, &format!("{name}.json"), text.as_bytes());
        let mut arguments = vec!["verify-run", path.as_str(), "--public-key", public.as_str()];
        arguments.extend_from_slice(extra);
        run(&arguments)
    };

    // A forged signature: another key's signature value under this key's id.
    let by_other = other.sign_text(&fixture_links()[0], ISSUED, 3600, &[2; NONCE_LEN]).unwrap();
    let forged_value = split(&by_other).unwrap().unwrap().envelope.unwrap().value;
    let (code, out) = named("forged", &signed.replace(&mine.value, &forged_value), &["--now", INSIDE]);
    assert_eq!((code, line(&out, "signature_cause")), (5, Some("SignatureDoesNotVerify")), "{out}");
    assert_eq!(line(&out, "holds"), Some("true"), "the seals hold: {out}");

    // An expired signature.
    let (code, out) = named("expired", &signed, &["--now", AFTER]);
    assert_eq!((code, line(&out, "signature_cause")), (5, Some("SignatureExpired")), "{out}");

    // A replayed nonce: the first check adds it to the ledger, the second finds it there.
    let ledger = folder.join("nonces.txt").to_string_lossy().into_owned();
    let (code, out) = named("replayed", &signed, &["--now", INSIDE, "--nonce-ledger", &ledger]);
    assert_eq!((code, line(&out, "signature")), (0, Some("holds")), "{out}");
    let (code, out) = named("replayed", &signed, &["--now", INSIDE, "--nonce-ledger", &ledger]);
    assert_eq!((code, line(&out, "signature_cause")), (5, Some("SignatureNonceReplayed")), "{out}");
    assert_eq!(std::fs::read_to_string(&ledger).unwrap(), format!("{}\n", "02".repeat(32)));

    // A value changed after signing: a decimal copy, which no seal covers, so
    // only the receipt and the signature see it.
    let decimal = "\"final_residual_conservation_error\":";
    assert!(signed.contains(decimal));
    let at = signed.find(decimal).unwrap() + decimal.len();
    let mut tampered = signed.clone();
    tampered.insert(at, '-');
    let (code, out) = named("tampered", &tampered, &["--now", INSIDE]);
    assert_eq!((code, line(&out, "signature_cause")), (5, Some("ReceiptDoesNotMatch")), "{out}");
    assert_eq!(line(&out, "receipt"), Some("does not match"));
    // The same change with the receipt made again: the signature is over the old receipt.
    let unsigned = split(&tampered).unwrap().unwrap().unsigned;
    let rewritten_unsigned = genesis_seal::receipt::with_receipt(&format!(
        "{}}}",
        &unsigned[..unsigned.rfind(",\"receipt_sha384\":\"").unwrap()]
    ));
    let receipt_at = rewritten_unsigned.len() - 1;
    let rewritten = format!("{},\"signature\":{}}}", &rewritten_unsigned[..receipt_at], mine.json());
    let (code, out) = named("rewritten", &rewritten, &["--now", INSIDE]);
    assert_eq!((code, line(&out, "signature_cause")), (5, Some("SignatureDoesNotVerify")), "{out}");
    assert_eq!(line(&out, "receipt"), Some("matches"));

    // The wrong key.
    let path = write(&folder, "wrong_key.json", signed.as_bytes());
    let (code, out) = run(&["verify-run", &path, "--public-key", &other_public, "--now", INSIDE]);
    assert_eq!((code, line(&out, "signature_cause")), (5, Some("SignatureKeyMismatch")), "{out}");

    // An answer with no signature, checked with a key.
    let (code, out) = named("absent", &fixture_links()[0], &["--now", INSIDE]);
    assert_eq!((code, line(&out, "signature"), line(&out, "signature_cause")), (5, Some("absent"), Some("SignatureAbsent")), "{out}");

    // A broken seal is said first, exit 1, whatever the signature.
    let broken = signed.replacen("\"outcome\":\"completed\"", "\"outcome\":\"refused\"", 1);
    let (code, out) = named("broken", &broken, &["--now", INSIDE]);
    assert_eq!((code, line(&out, "cause")), (1, Some("run_seal")), "{out}");
}

#[test]
fn a_signed_chain_is_checked_link_by_link() {
    let folder = folder("chain");
    let key = test_key("engine");
    let public = write(&folder, "engine.pub", key.public_key().to_base64().as_bytes());
    let links = fixture_links();
    let first = key.sign_text(&links[0], ISSUED, 3600, &[3; NONCE_LEN]).unwrap();
    let second = key.sign_text(&links[1], ISSUED, 3600, &[4; NONCE_LEN]).unwrap();
    let chain = write(&folder, "chain.json", format!("{{\"links\":[{first},{second}]}}").as_bytes());
    let (code, out) = run(&["verify-chain", &chain, "--public-key", &public, "--now", INSIDE]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(line(&out, "links[0].signature"), Some("holds"));
    assert_eq!(line(&out, "links[1].signature"), Some("holds"));
    let (code, out) = run(&["verify-chain", &chain, "--public-key", &public, "--now", AFTER]);
    assert_eq!((code, line(&out, "links[1].signature_cause")), (5, Some("SignatureExpired")), "{out}");
}

#[test]
fn a_record_of_seal_rule_v1_is_named_and_not_read() {
    let v1 = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1_chain_thermal.json");
    let text = std::fs::read_to_string(&v1).unwrap();
    let (code, out) = run(&["verify-chain", &v1.to_string_lossy()]);
    assert_eq!((code, line(&out, "cause")), (2, Some("SealRuleNotServed")), "{out}");
    let folder = folder("v1");
    let body = &text["{\"links\":[".len()..];
    let mut stream = serde_json::Deserializer::from_str(body).into_iter::<serde_json::Value>();
    stream.next().unwrap().unwrap();
    let run_v1 = write(&folder, "run_v1.json", body[..stream.byte_offset()].as_bytes());
    let output = Command::new(PROGRAM).args(["verify-run", &run_v1]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "cause: SealRuleNotServed\n");
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(said.contains("seal rule v1 (SHA-256), engine 0.14.3 and earlier") && said.contains("genesis_seal 0.6.0"), "{said}");
}

#[test]
fn a_dataset_whose_head_is_signed_holds_and_a_head_changed_after_is_named() {
    needs_saved_runs!();
    let folder = folder("dataset");
    let key = test_key("engine");
    let public = write(&folder, "engine.pub", key.public_key().to_base64().as_bytes());
    let files = dataset_of(&eight_short_runs(), "frames");
    let head_text = String::from_utf8(files.head.clone()).unwrap();
    let signed_head = key.sign_text(&head_text, ISSUED, 3600, &[5; NONCE_LEN]).unwrap();
    let head = write(&folder, "head.json", signed_head.as_bytes());
    let rows = write(&folder, "rows.jsonl", &files.rows);
    let frames = write(&folder, "frames.bin", files.frames.as_deref().unwrap());
    let (code, out) = run(&["verify-dataset", &head, &rows, &frames, "--public-key", &public, "--now", INSIDE]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("receipt: matches\n"), "{out}");
    assert_eq!(line(&out, "signature"), Some("holds"));
    // The request's spelling changed in the head: no seal sees it, the receipt and the signature do.
    let respelled = signed_head.replacen("\"store\":\"frames\"", "\"store\": \"frames\"", 1);
    assert_ne!(respelled, signed_head);
    let head = write(&folder, "head_respelled.json", respelled.as_bytes());
    let (code, out) = run(&["verify-dataset", &head, &rows, &frames, "--public-key", &public, "--now", INSIDE]);
    assert_eq!((code, line(&out, "signature_cause")), (5, Some("ReceiptDoesNotMatch")), "{out}");
}
