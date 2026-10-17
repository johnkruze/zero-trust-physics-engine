//! Signed loads verified with no engine: a run, a comparison and the
//! program, under every attack of the design (SIGNED_LOADS.md, section 6)
//! that a record can carry. The runs are made here by the published rules
//! (`support::fabricated_run`), so each attack can rebuild every seal and
//! leave only the signature to speak.

#[allow(unused_macros, unused_imports)]
mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use ed25519_dalek::{Signer, SigningKey};
use genesis_seal::attest::{self, Place, Stream, Trust, Trusted};
use genesis_seal::run::{TapeCause, check_tape};
use genesis_seal::validation::check_validation;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use support::fabricated_run;

const PROGRAM: &str = env!("CARGO_BIN_EXE_genesis_seal");

/// The bench's TEST KEYS: each secret is the SHA-256 of a sentence that says
/// it is a test key (evidence/verification/next_engine_version_2026-10-14/
/// signed_loads/bench_instrument.py). Anyone can sign with them.
fn test_key(name: &str) -> SigningKey {
    let sentence = match name {
        "A" => "zero-trust-physics bench TEST KEY A: signs currents; not an instrument's key",
        "B" => "zero-trust-physics bench TEST KEY B: signs voltages; not an instrument's key",
        "M" => "zero-trust-physics bench TEST KEY M: the attacker's own key; not an instrument's key",
        _ => unreachable!(),
    };
    SigningKey::from_bytes(&Sha256::digest(sentence.as_bytes()).into())
}

const SHUNT: &str = "shunt-01 (INA226, 2 mOhm, serial 0042), test key";
const CHALLENGE: &str = "9f2e4c1a0b7d3e58";

fn key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

/// One signed reading at `place` over `reading`.
fn sign(key: &SigningKey, place: Place, reading: &Value, seq: u64, prev: Option<&str>, challenge: Option<&str>, second: u64) -> Value {
    let mut signed = Map::new();
    signed.insert("alg".into(), json!("ed25519"));
    signed.insert("key".into(), json!(key_hex(key)));
    signed.insert("instrument".into(), json!(SHUNT));
    signed.insert("time".into(), json!(format!("2026-10-21T14:{:02}:{:02}.000Z", 2 + second / 60, second % 60)));
    signed.insert("seq".into(), json!(seq));
    signed.insert("prev".into(), prev.map(|text| json!(text)).unwrap_or(Value::Null));
    signed.insert("challenge".into(), challenge.map(|text| json!(text)).unwrap_or(Value::Null));
    let text = attest::signed_text(place, &signed, reading);
    signed.insert("sig".into(), json!(hex::encode(key.sign(&text).to_bytes())));
    Value::Object(signed)
}

/// A battery run of `count` one-second segments, each load a current signed
/// by `key`, chained by prev from seq 1047.
fn signed_run(key: &SigningKey, count: usize, challenge: Option<&str>) -> Value {
    let mut segments = Vec::new();
    let mut prev: Option<String> = None;
    for index in 0..count {
        let load = json!({ "current_a": 12.5 + index as f64 / 1000.0 });
        let signed = sign(key, Place::Segment(index), &load, 1047 + index as u64, prev.as_deref(), challenge, index as u64);
        let text = attest::signed_text(Place::Segment(index), signed.as_object().unwrap(), &load);
        prev = Some(hex::encode(Sha256::digest(&text)));
        segments.push(json!({ "steps": 1, "load": load, "signed": signed }));
    }
    json!({ "domain": "energy", "card": "battery_sag", "initial": { "soc": 1.0 }, "dt_s": 1.0, "segments": segments, "record_every": 1 })
}

fn tape(request: &Value) -> Value {
    let steps = request["segments"].as_array().unwrap().len() as u64;
    fabricated_run(request.clone(), steps, 1, 7)
}

#[test]
fn a_signed_run_holds_and_lists_every_reading_in_order() {
    let request = signed_run(&test_key("A"), 60, Some(CHALLENGE));
    let report = check_tape(&tape(&request)).unwrap();
    assert!(report.holds, "{report:?}");
    assert_eq!(report.signed_loads.len(), 60);
    assert_eq!(report.signed_loads[0].seq, 1047);
    assert_eq!(report.signed_loads[59].seq, 1106);
    assert_eq!(attest::stream(&report.signed_loads), Stream::Unbroken);
    let label = attest::load_source(60, &report.signed_loads);
    assert_eq!(label["signed"], 60);
    assert_eq!(label["keys"][0]["names_itself"], SHUNT);
    assert_eq!(label["keys"][0]["seq"], "1047 to 1106");
    assert_eq!(label["keys"][0]["time"], "2026-10-21T14:02:00.000Z to 2026-10-21T14:02:59.000Z");
    assert_eq!(label["keys"][0]["challenges"], json!([CHALLENGE]));
}

#[test]
fn an_edited_reading_breaks_the_chain_and_rebuilt_seals_leave_the_signature_to_name_it() {
    let request = signed_run(&test_key("A"), 60, Some(CHALLENGE));
    let honest = tape(&request);
    // One current changed by 1 mA, the frames as they were: the first link is not the request's.
    let mut edited = honest.clone();
    edited["inputs"]["segments"][22]["load"]["current_a"] = json!(12.523);
    let report = check_tape(&edited).unwrap();
    assert_eq!((report.holds, report.cause, report.broken_link), (false, Some(TapeCause::Chain), Some(0)));
    // The same edit with every frame, the state digest and the run seal made again.
    let forged = tape(&edited["inputs"]);
    let report = check_tape(&forged).unwrap();
    assert_eq!((report.holds, report.cause), (false, Some(TapeCause::LoadSignature)));
    assert_eq!(report.broken_place.as_deref(), Some("segments[22]"));
    let says = report.load_fault.unwrap();
    assert!(says.contains("does not verify") && says.ends_with("(LoadSignatureDoesNotVerify)."), "{says}");
}

#[test]
fn a_stripped_signature_runs_as_the_callers_number() {
    let mut request = signed_run(&test_key("A"), 5, None);
    for segment in request["segments"].as_array_mut().unwrap() {
        segment.as_object_mut().unwrap().shift_remove("signed");
    }
    let report = check_tape(&tape(&request)).unwrap();
    assert!(report.holds);
    assert!(report.signed_loads.is_empty());
    assert_eq!(attest::load_source(5, &report.signed_loads)["says"], attest::CALLERS_NUMBER);
    // One signature stripped from the middle: the others still verify, and
    // the stream shows the gap.
    let mut one = signed_run(&test_key("A"), 5, None);
    one["segments"][2].as_object_mut().unwrap().shift_remove("signed");
    let report = check_tape(&tape(&one)).unwrap();
    assert!(report.holds);
    assert!(matches!(attest::stream(&report.signed_loads), Stream::Breaks { at: Place::Segment(3), .. }));
    let label = attest::load_source(5, &report.signed_loads);
    assert_eq!(label["signed"], 4);
    assert!(label["says"].as_str().unwrap().contains("the loads at segments[2] are the caller's number"));
}

#[test]
fn a_splice_holds_every_seal_and_the_stream_names_where_it_breaks() {
    let request = signed_run(&test_key("A"), 30, None);
    // The awkward reading dropped.
    let mut dropped = request.clone();
    dropped["segments"].as_array_mut().unwrap().remove(22);
    let report = check_tape(&tape(&dropped)).unwrap();
    assert!(report.holds);
    match attest::stream(&report.signed_loads) {
        Stream::Breaks { at, says } => {
            assert_eq!(at, Place::Segment(22));
            assert!(says.contains("its prev is not the SHA-256"), "{says}");
        }
        Stream::Unbroken => panic!("a dropped reading went unseen"),
    }
    // Two genuine streams of one key joined.
    let other = signed_run(&test_key("A"), 30, None);
    let mut joined = request.clone();
    let first: Vec<Value> = request["segments"].as_array().unwrap()[..10].to_vec();
    let second: Vec<Value> = other["segments"].as_array().unwrap()[20..].to_vec();
    joined["segments"] = Value::Array(first.into_iter().chain(second).collect());
    let report = check_tape(&tape(&joined)).unwrap();
    assert!(matches!(attest::stream(&report.signed_loads), Stream::Breaks { at: Place::Segment(10), .. }));
    // Reordered.
    let mut swapped = request.clone();
    swapped["segments"].as_array_mut().unwrap().swap(5, 6);
    let report = check_tape(&tape(&swapped)).unwrap();
    assert!(matches!(attest::stream(&report.signed_loads), Stream::Breaks { at: Place::Segment(5), .. }));
}

#[test]
fn a_genuine_reading_moved_to_another_field_does_not_verify() {
    let request = signed_run(&test_key("A"), 3, None);
    let segment = request["segments"][0].clone();
    // The shunt's current moved into the start state, signature and all.
    let mut moved = request.clone();
    let mut initial = segment["load"].as_object().unwrap().clone();
    initial.insert("soc".into(), json!(1.0));
    initial.insert("signed".into(), segment["signed"].clone());
    moved["initial"] = Value::Object(initial);
    let report = check_tape(&tape(&moved)).unwrap();
    assert_eq!(report.cause, Some(TapeCause::LoadSignature));
    assert_eq!(report.broken_place.as_deref(), Some("initial"));
    // The same number under another quantity's name.
    let mut renamed = request.clone();
    renamed["segments"][1]["load"] = json!({ "ambient_k": request["segments"][1]["load"]["current_a"] });
    let report = check_tape(&tape(&renamed)).unwrap();
    assert_eq!(report.broken_place.as_deref(), Some("segments[1]"));
}

#[test]
fn a_replay_verifies_and_says_its_own_time_and_challenge() {
    // Yesterday's stream, with no challenge: it verifies, and says when it was made.
    let yesterday = signed_run(&test_key("A"), 3, None);
    let report = check_tape(&tape(&yesterday)).unwrap();
    assert!(report.holds);
    let label = attest::load_source(3, &report.signed_loads);
    assert_eq!(label["keys"][0]["challenges"], json!([null]));
    // Yesterday's stream made under yesterday's challenge: it does not carry today's.
    let old = signed_run(&test_key("A"), 3, Some("0000aaaa"));
    let report = check_tape(&tape(&old)).unwrap();
    let label = attest::load_source(3, &report.signed_loads);
    assert_eq!(label["keys"][0]["challenges"], json!(["0000aaaa"]));
    assert_ne!(label["keys"][0]["challenges"], json!([CHALLENGE]));
    // Today's challenge written into yesterday's readings breaks every signature.
    let mut rewritten = old.clone();
    rewritten["segments"][0]["signed"]["challenge"] = json!(CHALLENGE);
    let report = check_tape(&tape(&rewritten)).unwrap();
    assert_eq!(report.cause, Some(TapeCause::LoadSignature));
}

#[test]
fn a_swapped_instrument_verifies_under_its_own_key_and_the_list_does_not_trust_it() {
    let honest = check_tape(&tape(&signed_run(&test_key("A"), 2, None))).unwrap();
    let swapped = check_tape(&tape(&signed_run(&test_key("M"), 2, None))).unwrap();
    assert!(swapped.holds);
    assert_eq!(swapped.signed_loads[0].instrument, honest.signed_loads[0].instrument);
    assert_ne!(swapped.signed_loads[0].fingerprint, honest.signed_loads[0].fingerprint);
    let list = format!(
        r#"{{"keys":[{{"key":"{}","name":"bench shunt, TEST KEY A","quantities":["current_a"]}}]}}"#,
        key_hex(&test_key("A"))
    );
    let trust = Trust::read(list.as_bytes()).unwrap();
    assert_eq!(trust.judge(&honest.signed_loads[0]), Trusted::Yes("bench shunt, TEST KEY A".into()));
    assert!(matches!(trust.judge(&swapped.signed_loads[0]), Trusted::No(_)));
}

#[test]
fn key_substitution_malleability_and_another_protocols_signature_are_named() {
    let request = signed_run(&test_key("A"), 2, None);
    let sig = request["segments"][0]["signed"]["sig"].as_str().unwrap().to_string();
    let attacks: Vec<(&str, Box<dyn Fn(&mut Value)>, &str)> = vec![
        ("key substitution", Box::new(|run: &mut Value| run["segments"][0]["signed"]["key"] = json!(key_hex(&test_key("M")))), "not one this key made"),
        (
            "S + L",
            Box::new(move |run: &mut Value| {
                let bytes = hex::decode(&sig).unwrap();
                let order: [u8; 32] = [
                    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
                ];
                let mut s = [0_u8; 32];
                let mut carry = 0_u16;
                for index in 0..32 {
                    let total = bytes[32 + index] as u16 + order[index] as u16 + carry;
                    s[index] = total as u8;
                    carry = total >> 8;
                }
                run["segments"][0]["signed"]["sig"] = json!(format!("{}{}", hex::encode(&bytes[..32]), hex::encode(s)));
            }),
            "malleated",
        ),
        (
            "another protocol",
            Box::new(|run: &mut Value| {
                let load = run["segments"][0]["load"].clone();
                let signed = run["segments"][0]["signed"].as_object().unwrap().clone();
                let text = attest::signed_text(Place::Segment(0), &signed, &load);
                let bare = &text[attest::DOMAIN_LINE.len()..];
                run["segments"][0]["signed"]["sig"] = json!(hex::encode(test_key("A").sign(bare).to_bytes()));
            }),
            "not one this key made",
        ),
        ("a weak key", Box::new(|run: &mut Value| run["segments"][0]["signed"]["key"] = json!(format!("01{}", "00".repeat(31)))), "small order"),
    ];
    for (name, attack, words) in attacks {
        let mut run = request.clone();
        attack(&mut run);
        let report = check_tape(&tape(&run)).unwrap();
        assert_eq!(report.cause, Some(TapeCause::LoadSignature), "{name}");
        let says = report.load_fault.unwrap();
        assert!(says.contains(words), "{name}: {says}");
    }
}

#[test]
fn a_malformed_signed_object_is_named_and_does_not_hold() {
    let mut request = signed_run(&test_key("A"), 2, None);
    request["segments"][1]["signed"]["seq"] = json!(1.5);
    let report = check_tape(&tape(&request)).unwrap();
    assert_eq!(report.cause, Some(TapeCause::LoadSignature));
    assert!(report.load_fault.unwrap().ends_with("(LoadSignatureMalformed)."));
}

/// A comparison made by the rules of the crate alone, its measurements each
/// signed by TEST KEY B.
fn signed_comparison(values: &[f64]) -> Value {
    use genesis_seal::validation::{SealParts, bits_hex, measurements_sha256, statistics, validation_seal};
    let key = test_key("B");
    let mut measurements = Vec::new();
    let mut prev: Option<String> = None;
    for (index, value) in values.iter().enumerate() {
        let measurement = json!({ "step": index as u64 + 1, "value": value });
        let signed = sign(&key, Place::Measurement(index), &measurement, 500 + index as u64, prev.as_deref(), Some(CHALLENGE), index as u64);
        let text = attest::signed_text(Place::Measurement(index), signed.as_object().unwrap(), &measurement);
        prev = Some(hex::encode(Sha256::digest(&text)));
        let mut item = measurement.as_object().unwrap().clone();
        item.insert("signed".into(), signed);
        measurements.push(Value::Object(item));
    }
    let measurements = Value::Array(measurements);
    let model: Vec<f64> = values.iter().map(|value| value + 0.001).collect();
    let steps: Vec<u64> = (1..=values.len() as u64).collect();
    let stats = statistics(&steps, &model, values).unwrap().unwrap();
    let hexes: Vec<String> = model.iter().map(|value| bits_hex(*value)).collect();
    let run_seal = "ab".repeat(32);
    let hash = measurements_sha256(&measurements);
    let seal = validation_seal(&SealParts {
        run_seal: &run_seal,
        measurements_sha256: &hash,
        quantity: "terminal_v",
        component: None,
        outcome: "completed",
        steps_landed: values.len() as u64,
        reject: None,
        compared: values.len() as u64,
        not_reached: 0,
        statistics: Some(&stats),
        model_values_hex: &hexes,
        commitment: None,
    });
    json!({
        "inputs": { "run": signed_run(&test_key("A"), values.len(), Some(CHALLENGE)), "quantity": "terminal_v", "measurements": measurements },
        "outcome": "completed", "steps_landed": values.len(), "reject": null, "run_seal": run_seal,
        "compared": values.len(), "not_reached": 0,
        "rms_error_hex": stats.rms_error_hex(), "mean_signed_error_hex": stats.mean_signed_error_hex(),
        "max_abs_error_hex": stats.max_abs_error_hex(), "max_abs_error_step": stats.max_abs_error_step,
        "model_values_hex": hexes, "measurements_sha256": hash, "validation_seal": seal
    })
}

#[test]
fn a_comparison_with_signed_measurements_holds_and_an_edited_measurement_is_named() {
    let honest = signed_comparison(&[3.9, 3.89, 3.88]);
    let report = check_validation(honest.to_string().as_bytes()).unwrap();
    assert!(report.holds, "{report:?}");
    assert_eq!(report.signed_measurements.len(), 3);
    assert_eq!(report.signed_loads.len(), 3);
    assert_eq!(attest::stream(&report.signed_measurements), Stream::Unbroken);
    // A measured voltage changed, the hash of the measurements made again: the signature names it.
    let mut edited = honest.clone();
    edited["inputs"]["measurements"][1]["value"] = json!(3.95);
    edited["measurements_sha256"] = json!(genesis_seal::validation::measurements_sha256(&edited["inputs"]["measurements"]));
    let report = check_validation(edited.to_string().as_bytes()).unwrap();
    assert_eq!(report.cause.map(|cause| cause.as_str()), Some("load_signature"));
    assert_eq!(report.broken_place.as_deref(), Some("measurements[1]"));
    // The same edit with the hash as it was: the hash of the measurements names it first.
    let mut edited = honest.clone();
    edited["inputs"]["measurements"][1]["value"] = json!(3.95);
    let report = check_validation(edited.to_string().as_bytes()).unwrap();
    assert_eq!(report.cause.map(|cause| cause.as_str()), Some("measurements"));
}

fn folder(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
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
    (output.status.code().expect("the program exits"), String::from_utf8(output.stdout).expect("text"))
}

#[test]
fn the_program_prints_each_reading_and_what_the_verifiers_list_says() {
    let folder = folder("signed_loads_program");
    let trust = write(
        &folder,
        "bench.json",
        format!(
            r#"{{"keys":[{{"key":"{}","name":"bench shunt, TEST KEY A","quantities":["current_a"],"valid_from":"2026-10-01T00:00:00Z","valid_to":"2026-12-31T23:59:59Z"}},{{"key":"{}","name":"bench cell voltage, TEST KEY B","quantities":["step","value"]}}]}}"#,
            key_hex(&test_key("A")),
            key_hex(&test_key("B"))
        )
        .as_bytes(),
    );
    let honest = write(&folder, "honest.json", tape(&signed_run(&test_key("A"), 3, Some(CHALLENGE))).to_string().as_bytes());
    let (code, out) = run(&["verify-run", &honest, "--trust", &trust]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("holds: true\nbroken_link: none\ncause: none\nreceipt: absent\nsigned_loads: 3\nstream: unbroken\nload_signature: holds\nplace: segments[0]\n"), "{out}");
    assert_eq!(out.matches("trusted: yes, \"bench shunt, TEST KEY A\"").count(), 3, "{out}");
    assert!(out.contains(&format!("name: \"{SHUNT}\"\n")), "{out}");
    assert!(out.contains(&format!("challenge: {CHALLENGE}\n")), "{out}");
    // No list given.
    let (code, out) = run(&["verify-run", &honest]);
    assert_eq!(code, 0);
    assert_eq!(out.matches("trusted: no list given").count(), 3);
    // The attacker's own key under the shunt's name: every seal holds, the list says no.
    let swapped = write(&folder, "swapped.json", tape(&signed_run(&test_key("M"), 3, Some(CHALLENGE))).to_string().as_bytes());
    let (code, out) = run(&["verify-run", &swapped, "--trust", &trust]);
    assert_eq!(code, 4, "{out}");
    assert!(out.contains("holds: true\n") && out.contains("trusted: no: the key "), "{out}");
    // An edited reading with every seal made again.
    let mut request = signed_run(&test_key("A"), 3, Some(CHALLENGE));
    request["segments"][1]["load"]["current_a"] = json!(12.502);
    let forged = write(&folder, "forged.json", tape(&request).to_string().as_bytes());
    let (code, out) = run(&["verify-run", &forged, "--trust", &trust]);
    assert_eq!(code, 1, "{out}");
    assert!(out.starts_with("holds: false\nbroken_link: none\ncause: load_signature\nreceipt: absent\nbroken_place: segments[1]\nload_fault: Physical Violation: the signature at 'segments[1].signed' does not verify"), "{out}");
    // A broken tape lists no reading: its readings are not the ones it would have to hold.
    assert!(!out.contains("signed_loads:") && !out.contains("trusted:"), "{out}");
    // A comparison: the run's loads and the measurements, each with what the list says.
    let comparison = write(&folder, "comparison.json", signed_comparison(&[3.9, 3.89]).to_string().as_bytes());
    let (code, out) = run(&["verify-validation", &comparison, "--trust", &trust]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("signed_loads: 2\n") && out.contains("signed_measurements: 2\n"), "{out}");
    assert_eq!(out.matches("trusted: yes, \"bench cell voltage, TEST KEY B\"").count(), 2, "{out}");
    // A list that cannot be read is refused, and --trust takes one file.
    let bad = write(&folder, "bad.json", br#"{"keys":[{"key":"xyz","name":"x"}]}"#);
    assert_eq!(run(&["verify-run", &honest, "--trust", &bad]).0, 2);
    assert_eq!(run(&["verify-run", &honest, "--trust"]).0, 2);
    assert_eq!(run(&["verify-chain", &honest, "--trust", &trust]).0, 2);
}

/// The worked example of `protocol_mcp_gateway/REFERENCE.md`, "A signed
/// load": TEST KEY A's reading of 12.5 A, its signed text in full, its
/// signature, and the first link of the request that carries it.
#[test]
fn the_worked_signed_reading_is_the_one_the_reference_prints() {
    let load = json!({ "current_a": 12.5 });
    let mut signed = Map::new();
    signed.insert("alg".into(), json!("ed25519"));
    signed.insert("key".into(), json!(key_hex(&test_key("A"))));
    signed.insert("instrument".into(), json!(SHUNT));
    signed.insert("time".into(), json!("2026-10-21T14:02:00.000Z"));
    signed.insert("seq".into(), json!(1047));
    signed.insert("prev".into(), Value::Null);
    signed.insert("challenge".into(), json!(CHALLENGE));
    let text = attest::signed_text(Place::Segment(0), &signed, &load);
    assert_eq!(
        String::from_utf8(text.clone()).unwrap(),
        "zero-trust-physics/signed-reading/v1\n{\"alg\":\"ed25519\",\"challenge\":\"9f2e4c1a0b7d3e58\",\"instrument\":\"shunt-01 (INA226, 2 mOhm, serial 0042), test key\",\"key\":\"04d76a8f2751918c5cff10c6c90cd93a38a8f3d7db37b5351041adaece403b67\",\"load\":{\"current_a\":x4029000000000000},\"prev\":null,\"seq\":x40905c0000000000,\"time\":\"2026-10-21T14:02:00.000Z\"}"
    );
    assert_eq!(text.len(), 331);
    assert_eq!(hex::encode(Sha256::digest(&text)), "b2be406e997511e68b044c2e0ed08356ef271c7f134ad632b85aa5c5d856a54f");
    let sig = hex::encode(test_key("A").sign(&text).to_bytes());
    assert_eq!(sig, "515a1ff5c20559fe87e74c69ff6c2e587e5886a62330f6070a8ba1a95862bb9d69b91e5f8e1016d9c2eb01d042385438eb6b523a85249101edbbf0a39202c000");
    signed.insert("sig".into(), json!(sig));
    let request = json!({ "domain": "energy", "card": "battery_sag", "initial": { "soc": 1.0 }, "dt_s": 1.0,
        "segments": [{ "steps": 1, "load": load, "signed": Value::Object(signed) }], "record_every": 1 });
    let verified = attest::verify_run(&request).unwrap();
    assert_eq!(verified[0].fingerprint, "3078ec6e06aecfba");
    assert_eq!(verified[0].reading_sha256, "b2be406e997511e68b044c2e0ed08356ef271c7f134ad632b85aa5c5d856a54f");
    assert_eq!(hex::encode(genesis_seal::run::link_0(&request)), "6f8a33edbdc8ac5457a262c91ca94d54e71dd9c9c2ec28304d0296539ef01d24");
    let mut plain = request.clone();
    plain["segments"][0].as_object_mut().unwrap().shift_remove("signed");
    assert_eq!(hex::encode(genesis_seal::run::link_0(&plain)), "9e4b373a5c64f62bb111163a2a4d61dac68b18a5ea49343687aa6bbf310fc244");
}

/// A dataset of two signed runs, and the same dataset after a forger edited
/// one reading of row 1 and made its run, its leaf, the plan's hash, the root
/// and the seal again by their published rules, with no key: the signature is
/// what names it, at its row, after the row's chain and before its state.
#[test]
fn a_dataset_row_whose_signed_reading_was_edited_and_every_seal_made_again_is_named_at_its_row() {
    use genesis_seal::dataset::{Cause, Head, check_dataset, dataset_seal};
    use genesis_seal::rows::{Row, leaf_text, rows_text};
    let honest = vec![tape(&signed_run(&test_key("A"), 3, None)), tape(&signed_run(&test_key("A"), 4, Some(CHALLENGE)))];
    let files = support::dataset_of(&honest, "none");
    let report = check_dataset(&files.head, &files.rows, None).unwrap();
    assert!(report.holds, "{report:?}");

    // Row 1's request replaced, its run made again, and the plan, the leaf,
    // the root, the seal and the rows hash made again over it.
    let forge = |edited: &Value| {
        let forged = tape(edited);
        let plan = support::list_request(&[honest[0].clone(), forged.clone()], "none", "file");
        let mut rows: Vec<Row> = files.assembled.rows.clone();
        let mut row = rows[1].to_value();
        let link_0 = hex::encode(genesis_seal::run::link_0(edited));
        row["request"] = edited.clone();
        row["last_link"] = forged["frames"].as_array().unwrap().last().unwrap()["link"].clone();
        row["run_seal"] = forged["run_seal"].clone();
        row["leaf"] = serde_json::from_str(&leaf_text(1, &link_0, "completed", forged["run_seal"].as_str(), None)).unwrap();
        rows[1] = Row::read(&row).unwrap();
        let leaves: Vec<_> = rows
            .iter()
            .map(|row| genesis_seal::tree::leaf_hash(serde_json::to_string(&row.leaf).unwrap().as_bytes()))
            .collect();
        let root = hex::encode(genesis_seal::tree::root(&leaves).unwrap());
        let mut head: Head = files.assembled.head.clone();
        head.request = plan.value().clone();
        head.plan_sha256 = plan.plan_sha256();
        head.merkle_root = root.clone();
        head.dataset_seal = dataset_seal(&head.plan_sha256, &root, &head.counts);
        let rows_bytes = rows_text(&rows);
        head.rows_sha256 = Some(genesis_seal::sealed_form::digest_hex(&rows_bytes));
        check_dataset(head.wire().as_bytes(), &rows_bytes, None).unwrap()
    };
    let mut edited = honest[1]["inputs"].clone();
    edited["segments"][2]["load"]["current_a"] = json!(99.0);
    let report = forge(&edited);
    assert!(!report.holds);
    assert_eq!((report.cause, report.row), (Some(Cause::LoadSignature), Some(1)), "{report:?}");
    // The same forgery with that reading's signature taken out holds: every
    // seal is rebuilt by its published rules, and the reading is then the
    // caller's number, which is what a label of it would say.
    let mut stripped = edited.clone();
    stripped["segments"][2].as_object_mut().unwrap().shift_remove("signed");
    assert!(forge(&stripped).holds);
}
