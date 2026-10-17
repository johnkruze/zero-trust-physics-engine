//! Signed loads at the door: an instrument's Ed25519 signature over a load,
//! a start, a solve's parameters and a measurement, verified before any
//! physics, and every attack of the design (SIGNED_LOADS.md, section 6) met
//! by name. The readings are signed here with the bench's TEST KEYS, whose
//! secrets are the SHA-256 of sentences that say they are test keys: anyone
//! can sign with them. The engine never signs.
//!
//! What is held: a signed run is sealed by the rule every run is sealed by,
//! and its answer's load_source says who signed each load; a request with no
//! signature keeps its bytes, its seal and its answer apart from the
//! label; an edited, moved, substituted, malleated or borrowed signature is
//! refused by name before computing; a stripped, spliced or replayed stream
//! runs and says so; the long form and the short form of verify_tape, the
//! program of genesis_seal and every connection agree.

mod hosted_support;
use hosted_support::answer_text;

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use ed25519_dalek::{Signer, SigningKey};
use genesis_seal::attest::{self, Place};
use hosted_support::*;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

const KEY: &str = "signed-key";
const CHALLENGE: &str = "9f2e4c1a0b7d3e58";
const SHUNT: &str = "shunt-01 (INA226, 2 mOhm, serial 0042), test key";
const ADC: &str = "adc-02 (ADS1115, cell voltage), test key";

/// The bench's TEST KEYS (evidence/verification/next_engine_version_2026-10-14/
/// signed_loads/bench_instrument.py).
fn test_key(name: &str) -> SigningKey {
    let sentence = match name {
        "A" => "zero-trust-physics bench TEST KEY A: signs currents; not an instrument's key",
        "B" => "zero-trust-physics bench TEST KEY B: signs voltages; not an instrument's key",
        "M" => "zero-trust-physics bench TEST KEY M: the attacker's own key; not an instrument's key",
        _ => unreachable!(),
    };
    SigningKey::from_bytes(&Sha256::digest(sentence.as_bytes()).into())
}

fn key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

fn fingerprint(key: &SigningKey) -> String {
    hex::encode(Sha256::digest(key.verifying_key().to_bytes()))[..16].to_string()
}

struct Reading<'a> {
    key: &'a SigningKey,
    name: &'a str,
    seq: u64,
    prev: Option<String>,
    challenge: Option<&'a str>,
    second: u64,
}

/// The signed object an instrument sends for `reading` at `place`, and the
/// SHA-256 of its signed text, which the next reading names in prev.
fn sign(reading: &Reading, place: Place, value: &Value) -> (Value, String) {
    let mut signed = Map::new();
    signed.insert("alg".into(), json!("ed25519"));
    signed.insert("key".into(), json!(key_hex(reading.key)));
    signed.insert("instrument".into(), json!(reading.name));
    signed.insert("time".into(), json!(format!("2026-10-21T14:{:02}:{:02}.000Z", 2 + reading.second / 60, reading.second % 60)));
    signed.insert("seq".into(), json!(reading.seq));
    signed.insert("prev".into(), reading.prev.clone().map(Value::String).unwrap_or(Value::Null));
    signed.insert("challenge".into(), reading.challenge.map(|text| json!(text)).unwrap_or(Value::Null));
    let text = attest::signed_text(place, &signed, value);
    signed.insert("sig".into(), json!(hex::encode(reading.key.sign(&text).to_bytes())));
    (Value::Object(signed), hex::encode(Sha256::digest(&text)))
}

/// The bench's currents: sixty one-second readings around 12.5 A.
fn current(index: usize) -> f64 {
    12.5 + 0.001 * ((index * 7) % 11) as f64
}

/// The battery pack run on `count` signed one-second current readings, chained by prev.
fn signed_battery(key: &SigningKey, count: usize, challenge: Option<&str>) -> Value {
    let mut segments = Vec::new();
    let mut prev = None;
    for index in 0..count {
        let load = json!({ "current_a": current(index) });
        let reading = Reading { key, name: SHUNT, seq: 1047 + index as u64, prev: prev.clone(), challenge, second: index as u64 };
        let (signed, hash) = sign(&reading, Place::Segment(index), &load);
        prev = Some(hash);
        segments.push(json!({ "steps": 1, "load": load, "signed": signed }));
    }
    json!({ "domain": "energy", "card": "battery_sag", "initial": { "soc": 1.0 }, "dt_s": 1.0, "segments": segments, "record_every": 1 })
}

fn unsigned(request: &Value) -> Value {
    let mut plain = request.clone();
    for segment in plain["segments"].as_array_mut().unwrap() {
        segment.as_object_mut().unwrap().shift_remove("signed");
    }
    if let Some(initial) = plain["initial"].as_object_mut() {
        initial.shift_remove("signed");
    }
    plain
}

fn local() -> Stdio2 {
    Stdio2::start(None)
}

fn ok(door: &mut Stdio2, tool: &str, arguments: &Value) -> Value {
    let (error, text) = door.tool(tool, arguments);
    assert!(!error, "{tool}: {text}");
    serde_json::from_str(&text).unwrap_or_else(|_| panic!("not JSON: {text}"))
}

fn refused(door: &mut Stdio2, tool: &str, arguments: &Value) -> String {
    let (error, text) = door.tool(tool, arguments);
    assert!(error, "{tool} was answered: {text}");
    text
}

#[test]
fn a_signed_run_is_sealed_as_every_run_is_and_its_answer_says_who_signed_each_load() {
    let mut door = local();
    let request = signed_battery(&test_key("A"), 60, Some(CHALLENGE));
    let answer = ok(&mut door, "run_trajectory", &request);
    assert_eq!(answer["outcome"], "completed");
    let label = &answer["load_source"];
    assert_eq!(label["segments"], 60);
    assert_eq!(label["signed"], 60);
    assert_eq!(label["initial"], "the caller's number");
    assert_eq!(label["stream"], "unbroken");
    let keys = label["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0]["key"], key_hex(&test_key("A")));
    assert_eq!(keys[0]["fingerprint"], fingerprint(&test_key("A")));
    assert_eq!(keys[0]["names_itself"], SHUNT);
    assert_eq!(keys[0]["readings"], 60);
    assert_eq!(keys[0]["places"], "segments[0] to segments[59]");
    assert_eq!(keys[0]["seq"], "1047 to 1106");
    assert_eq!(keys[0]["time"], "2026-10-21T14:02:00.000Z to 2026-10-21T14:02:59.000Z");
    assert_eq!(keys[0]["challenges"], json!([CHALLENGE]));
    assert!(label["says"].as_str().unwrap().starts_with("Every load of this run carries a signature that verifies under the key shown"));
    // load_source is written before the frames, after the state digest.
    let order: Vec<&str> = answer.as_object().unwrap().keys().map(String::as_str).collect();
    let at = |key: &str| order.iter().position(|name| *name == key).unwrap();
    assert!(at("state_digest") < at("load_source") && at("load_source") < at("frames"));

    // The seal is the rule of every run: link_0 is the SHA-256 of the sealed
    // form of the whole request, signatures and all, and genesis_seal holds it.
    let link_0 = hex::encode(Sha256::digest(genesis_seal::sealed_form::sealed_form(&request).as_bytes()));
    assert_eq!(&answer["frames"][0]["frame_hex"].as_str().unwrap()[96..128], &link_0[..32]);
    let report = genesis_seal::run::check_tape(&answer).unwrap();
    assert!(report.holds);
    assert_eq!(report.signed_loads.len(), 60);
    assert_eq!(attest::load_source(60, &report.signed_loads), *label);

    // The same loads with no signature: the same physics, another seal, and
    // the label says they are the caller's numbers.
    let plain = ok(&mut door, "run_trajectory", &unsigned(&request));
    assert_eq!(plain["final_state"], answer["final_state"]);
    assert_ne!(plain["run_seal"], answer["run_seal"]);
    assert_eq!(plain["load_source"], json!({ "segments": 60, "signed": 0, "says": attest::CALLERS_NUMBER }));
}

#[test]
fn a_run_with_no_signature_keeps_its_seal_and_differs_only_by_its_label_and_receipt() {
    let mut door = local();
    let described = ok(&mut door, "describe_card", &json!({ "domain": "energy", "card": "thermal" }));
    let request = described["example_run_trajectory"].clone();
    let answer = ok(&mut door, "run_trajectory", &request);
    assert_eq!(answer["load_source"], json!({ "segments": 1, "signed": 0, "says": attest::CALLERS_NUMBER }));
    // Without the label the document is the one the receipt of 0.11.0 was over:
    // the receipt is the SHA-256 of every byte before it.
    let text = serde_json::to_string(&answer).unwrap();
    let mut without = answer.as_object().unwrap().clone();
    without.shift_remove("load_source");
    without.shift_remove("receipt_sha256");
    let document = serde_json::to_string(&without).unwrap();
    assert!(text.contains("\"load_source\":{\"segments\":1,\"signed\":0,"));
    assert_ne!(sha(&document), answer["receipt_sha256"].as_str().unwrap());
    let report = genesis_seal::run::check_tape(&answer).unwrap();
    assert!(report.holds && report.signed_loads.is_empty());
}

#[test]
fn an_edited_reading_is_refused_by_name_and_nothing_is_sealed() {
    let mut door = local();
    let request = signed_battery(&test_key("A"), 60, Some(CHALLENGE));
    let mut edited = request.clone();
    edited["segments"][22]["load"]["current_a"] = json!(current(22) + 0.001);
    let said = refused(&mut door, "run_trajectory", &edited);
    assert!(said.starts_with("Physical Violation: the signature at 'segments[22].signed' does not verify under the key "), "{said}");
    assert!(said.contains(&fingerprint(&test_key("A"))), "{said}");
    assert!(said.ends_with("Nothing is computed and nothing is sealed (LoadSignatureDoesNotVerify)."), "{said}");
    // The same value spelled another way is the same reading.
    let mut respelled = request.clone();
    respelled["segments"][22]["load"]["current_a"] = serde_json::from_str::<Value>(&format!("{:e}", current(22))).unwrap();
    assert!(!door.tool("run_trajectory", &respelled).0);
    // A field of the signature changed, and the signed seq given as 1.5 or a time with an offset.
    for (field, value, name) in [
        ("seq", json!(1048), "LoadSignatureDoesNotVerify"),
        ("time", json!("2026-10-21T14:02:22.001Z"), "LoadSignatureDoesNotVerify"),
        ("instrument", json!("shunt-02"), "LoadSignatureDoesNotVerify"),
        ("challenge", json!("00"), "LoadSignatureDoesNotVerify"),
        ("seq", json!(1.5), "LoadSignatureMalformed"),
        ("time", json!("2026-10-21T14:02:22+00:00"), "LoadSignatureMalformed"),
        ("sig", json!("AB".repeat(64)), "LoadSignatureMalformed"),
        ("key", json!("ab"), "LoadSignatureMalformed"),
        ("alg", json!("ecdsa-p256-sha256"), "LoadSignatureAlgorithmNotServed"),
    ] {
        let mut changed = request.clone();
        changed["segments"][22]["signed"][field] = value.clone();
        let said = refused(&mut door, "run_trajectory", &changed);
        assert!(said.ends_with(&format!("({name}).")), "{field} {value}: {said}");
        assert!(said.contains("segments[22].signed"), "{said}");
    }
    let mut missing = request.clone();
    missing["segments"][3]["signed"].as_object_mut().unwrap().shift_remove("prev");
    assert_eq!(refused(&mut door, "run_trajectory", &missing), "Missing required parameter: 'segments[3].signed.prev' (InputMissing).");
    let mut extra = request.clone();
    extra["segments"][3]["signed"]["extra"] = json!(1);
    assert!(refused(&mut door, "run_trajectory", &extra).ends_with("(LoadSignatureMalformed)."));
}

#[test]
fn a_stripped_signature_runs_as_the_callers_number_and_a_partly_signed_run_says_which() {
    let mut door = local();
    let request = signed_battery(&test_key("A"), 10, None);
    let mut one_stripped = request.clone();
    one_stripped["segments"][4].as_object_mut().unwrap().shift_remove("signed");
    one_stripped["segments"][4]["load"]["current_a"] = json!(30.0);
    let answer = ok(&mut door, "run_trajectory", &one_stripped);
    let label = &answer["load_source"];
    assert_eq!(label["signed"], 9);
    assert_eq!(label["keys"][0]["places"], "segments[0] to segments[3], segments[5] to segments[9]");
    assert!(label["says"].as_str().unwrap().contains("9 of the 10 loads of this run carry a signature"), "{label}");
    assert!(label["says"].as_str().unwrap().contains("the loads at segments[4] are the caller's number"), "{label}");
    assert!(label["stream"].as_str().unwrap().starts_with("breaks at segments[5]: its prev is not the SHA-256"), "{label}");
    assert!(label["says"].as_str().unwrap().contains("The signed readings are not one unbroken stream: the stream breaks at segments[5]"), "{label}");
}

#[test]
fn a_splice_runs_and_the_stream_names_the_first_break() {
    let mut door = local();
    let request = signed_battery(&test_key("A"), 30, Some(CHALLENGE));
    let mut dropped = request.clone();
    dropped["segments"].as_array_mut().unwrap().remove(22);
    let answer = ok(&mut door, "run_trajectory", &dropped);
    assert_eq!(answer["load_source"]["signed"], 29);
    assert_eq!(
        answer["load_source"]["stream"],
        format!("breaks at segments[22]: its prev is not the SHA-256 of the signed text at segments[21], the reading before it by the key {}", fingerprint(&test_key("A")))
    );
    // The sentence says it too, not only the stream: a reading dropped.
    assert_eq!(
        answer["load_source"]["says"],
        format!(
            "Every load of this run carries a signature that verifies under the key shown; its initial state is the caller's number. The signed readings are not one unbroken stream: the stream breaks at segments[22]: its prev is not the SHA-256 of the signed text at segments[21], the reading before it by the key {}. So a reading of that key is missing there, or the readings were reordered, or spliced from another stream. A signature shows that the holder of that key stated these readings; whether the key is a real instrument's is the verifier's list.",
            fingerprint(&test_key("A"))
        )
    );
    let mut swapped = request.clone();
    swapped["segments"].as_array_mut().unwrap().swap(10, 11);
    let answer = ok(&mut door, "run_trajectory", &swapped);
    assert!(answer["load_source"]["stream"].as_str().unwrap().starts_with("breaks at segments[10]"));
    assert!(answer["load_source"]["says"].as_str().unwrap().contains("The signed readings are not one unbroken stream: the stream breaks at segments[10]"), "{}", answer["load_source"]);
    // An unbroken stream's sentence says nothing of a stream.
    let answer = ok(&mut door, "run_trajectory", &request);
    assert_eq!(answer["load_source"]["stream"], "unbroken");
    assert!(!answer["load_source"]["says"].as_str().unwrap().contains("stream"), "{}", answer["load_source"]);
}

#[test]
fn a_genuine_reading_moved_to_another_field_is_refused() {
    let mut door = local();
    let request = signed_battery(&test_key("A"), 3, None);
    // A current reading carried into the start state with its signature.
    let mut moved = request.clone();
    moved["initial"] = json!({ "soc": 1.0, "signed": request["segments"][0]["signed"] });
    let said = refused(&mut door, "run_trajectory", &moved);
    assert!(said.contains("'initial.signed' does not verify"), "{said}");
    // A start state signed as a start state verifies there, and the label says so.
    let start = json!({ "soc": 1.0 });
    let (signed, _) = sign(&Reading { key: &test_key("A"), name: SHUNT, seq: 1, prev: None, challenge: None, second: 0 }, Place::Initial, &start);
    let mut signed_start = unsigned(&request);
    signed_start["initial"]["signed"] = signed;
    let answer = ok(&mut door, "run_trajectory", &signed_start);
    assert_eq!(answer["load_source"]["initial"], "signed");
    assert_eq!(answer["load_source"]["signed"], 0);
    // The start state's signature carried into a segment.
    let mut back = request.clone();
    back["segments"][1]["signed"] = signed_start["initial"]["signed"].clone();
    assert!(refused(&mut door, "run_trajectory", &back).contains("'segments[1].signed' does not verify"));
}

#[test]
fn a_replay_verifies_and_says_its_own_time_and_challenge() {
    let mut door = local();
    let yesterday = signed_battery(&test_key("A"), 3, None);
    let answer = ok(&mut door, "run_trajectory", &yesterday);
    assert_eq!(answer["load_source"]["keys"][0]["challenges"], json!([null]));
    assert_eq!(answer["load_source"]["keys"][0]["time"], "2026-10-21T14:02:00.000Z to 2026-10-21T14:02:02.000Z");
    let old = signed_battery(&test_key("A"), 3, Some("0000aaaa"));
    let answer = ok(&mut door, "run_trajectory", &old);
    assert_eq!(answer["load_source"]["keys"][0]["challenges"], json!(["0000aaaa"]));
    let mut today = old.clone();
    today["segments"][0]["signed"]["challenge"] = json!(CHALLENGE);
    assert!(refused(&mut door, "run_trajectory", &today).ends_with("(LoadSignatureDoesNotVerify)."));
}

#[test]
fn a_swapped_instrument_runs_under_its_own_key_whatever_name_it_gives() {
    let mut door = local();
    let honest = ok(&mut door, "run_trajectory", &signed_battery(&test_key("A"), 3, None));
    let swapped = ok(&mut door, "run_trajectory", &signed_battery(&test_key("M"), 3, None));
    assert_eq!(swapped["load_source"]["keys"][0]["names_itself"], honest["load_source"]["keys"][0]["names_itself"]);
    assert_eq!(swapped["load_source"]["keys"][0]["fingerprint"], fingerprint(&test_key("M")));
    assert_ne!(swapped["load_source"]["keys"][0]["fingerprint"], honest["load_source"]["keys"][0]["fingerprint"]);
    assert_eq!(swapped["final_state"], honest["final_state"]);
    assert_ne!(swapped["run_seal"], honest["run_seal"]);
}

#[test]
fn key_substitution_malleability_and_another_protocols_signature_are_refused() {
    let mut door = local();
    let request = signed_battery(&test_key("A"), 2, None);
    let sig = hex::decode(request["segments"][0]["signed"]["sig"].as_str().unwrap()).unwrap();
    let order: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0x10,
    ];
    let mut s = [0_u8; 32];
    let mut carry = 0_u16;
    for index in 0..32 {
        let total = sig[32 + index] as u16 + order[index] as u16 + carry;
        s[index] = total as u8;
        carry = total >> 8;
    }
    let load = request["segments"][0]["load"].clone();
    let fields = request["segments"][0]["signed"].as_object().unwrap().clone();
    let text = attest::signed_text(Place::Segment(0), &fields, &load);
    let bare = &text[attest::DOMAIN_LINE.len()..];
    let mut identity = [0_u8; 32];
    identity[0] = 1;
    for (name, field, value, words) in [
        ("key substitution", "key", json!(key_hex(&test_key("M"))), "not one this key made"),
        ("S + L", "sig", json!(format!("{}{}", hex::encode(&sig[..32]), hex::encode(s))), "malleated form of a signature"),
        ("another protocol", "sig", json!(hex::encode(test_key("A").sign(bare).to_bytes())), "not one this key made"),
        ("a weak key", "key", json!(hex::encode(identity)), "small order"),
        ("R the neutral point", "sig", json!(format!("{}{}", hex::encode(identity), hex::encode(&sig[32..]))), "R is a point of small order"),
    ] {
        let mut attacked = request.clone();
        attacked["segments"][0]["signed"][field] = value;
        let said = refused(&mut door, "run_trajectory", &attacked);
        assert!(said.contains(words) && said.ends_with("(LoadSignatureDoesNotVerify)."), "{name}: {said}");
    }
}

/// The answer with one load edited and every seal and the receipt made again
/// by their published rules: what a forger with no key can make.
fn forged(answer: &Value, segment: usize) -> Value {
    let mut forged = answer.clone();
    forged["inputs"]["segments"][segment]["load"]["current_a"] = json!(current(segment) + 0.001);
    let mut link = genesis_seal::run::link_0(&forged["inputs"]);
    for frame in forged["frames"].as_array_mut().unwrap() {
        let mut bytes: [u8; 64] = hex::decode(frame["frame_hex"].as_str().unwrap()).unwrap().try_into().unwrap();
        link = genesis_seal::run::seal_frame(&link, &mut bytes);
        frame["frame_hex"] = json!(hex::encode(bytes));
        frame["link"] = json!(hex::encode(link));
    }
    let seal = genesis_seal::run::run_seal(&link, "completed", forged["steps_landed"].as_u64().unwrap(), None, forged["state_digest"].as_str());
    forged["run_seal"] = json!(seal);
    let mut body = forged.as_object().unwrap().clone();
    body.shift_remove("receipt_sha256");
    let text = serde_json::to_string(&body).unwrap();
    forged["receipt_sha256"] = json!(sha(&text));
    forged
}

#[test]
fn the_long_form_names_a_reading_that_does_not_verify_and_holds_the_label_to_the_engines() {
    let mut door = local();
    let request = signed_battery(&test_key("A"), 30, Some(CHALLENGE));
    let answer = ok(&mut door, "run_trajectory", &request);
    let held = ok(&mut door, "verify_tape", &answer);
    assert_eq!(held["holds"], true, "{held}");
    assert_eq!(held["signed_loads"], 30);
    // An edited load with every seal and the receipt made again.
    let report = ok(&mut door, "verify_tape", &forged(&answer, 22));
    assert_eq!(report["holds"], false);
    assert_eq!(report["cause"], "load_signature");
    assert_eq!(report["broken_place"], "segments[22]");
    assert!(report["difference"].as_str().unwrap().ends_with("(LoadSignatureDoesNotVerify)."), "{report}");
    // The edit with the frames as they were: the chain names it first.
    let mut edited = answer.clone();
    edited["inputs"]["segments"][22]["load"]["current_a"] = json!(99.0);
    let report = ok(&mut door, "verify_tape", &edited);
    assert_eq!((report["cause"].as_str(), report["broken_link"].as_u64()), (Some("chain"), Some(0)));
    // The label rewritten: the seals hold, and the text is not the engine's.
    for (path, value) in [
        ("names_itself", json!("shunt-01, an instrument of the Army bench")),
        ("challenges", json!(["1234"])),
        ("fingerprint", json!("0000000000000000")),
    ] {
        let mut lied = answer.clone();
        lied["load_source"]["keys"][0][path] = value;
        let report = ok(&mut door, "verify_tape", &lied);
        assert_eq!(report["cause"], "answer_text", "{path}");
        let at = report["differs_at"].as_str().unwrap();
        assert!(at == format!("load_source.keys[0].{path}") || at == format!("load_source.keys[0].{path}[0]"), "{report}");
    }
    let mut lied = answer.clone();
    lied["load_source"]["says"] = json!(attest::CALLERS_NUMBER);
    assert_eq!(ok(&mut door, "verify_tape", &lied)["differs_at"], "load_source.says");
    let mut dropped = answer.clone();
    dropped.as_object_mut().unwrap().shift_remove("load_source");
    assert_eq!(ok(&mut door, "verify_tape", &dropped)["differs_at"], "load_source");
    // The short form computes again and says where the loads came from.
    let short = ok(&mut door, "verify_tape", &json!({ "request": request, "run_seal": answer["run_seal"] }));
    assert_eq!(short["holds"], true);
    assert_eq!(short["load_source"], answer["load_source"]);
    let mut edited_request = request.clone();
    edited_request["segments"][22]["load"]["current_a"] = json!(current(22) + 0.001);
    let said = refused(&mut door, "verify_tape", &json!({ "request": edited_request, "run_seal": answer["run_seal"] }));
    assert!(said.ends_with("(LoadSignatureDoesNotVerify)."), "{said}");
}

#[test]
fn a_plan_or_a_search_that_would_set_a_signed_value_is_refused_by_name() {
    let mut door = local();
    let request = signed_battery(&test_key("A"), 3, None);
    let mut base = request.clone();
    for key in ["domain", "card"] {
        base.as_object_mut().unwrap().shift_remove(key);
    }
    let grid = |path: &str| {
        json!({ "domain": "energy", "card": "battery_sag", "plan": { "kind": "grid", "base": base.clone(), "axes": [{ "path": path, "values": [10.0, 11.0] }] }, "store": "none", "return": "inline" })
    };
    for path in ["segments[1].load.current_a", "initial.soc"] {
        let mut plan = grid(path);
        if path == "initial.soc" {
            plan["plan"]["base"]["initial"]["signed"] = request["segments"][0]["signed"].clone();
        }
        let said = refused(&mut door, "run_dataset", &plan);
        assert!(said.ends_with("(SignedLoadReplaced)."), "{path}: {said}");
        assert!(said.starts_with(&format!("Physical Violation: '{path}' would set a value of the reading signed at")), "{said}");
    }
    // A value no signature covers may be varied: the step.
    let mut dt = grid("dt_s");
    dt["plan"]["axes"][0]["values"] = json!([1.0, 0.5]);
    let answer = ok(&mut door, "run_dataset", &dt);
    assert_eq!(answer["head"]["completed"], 2, "{answer}");
    // A list of signed runs is sealed row by row.
    let list = json!({ "domain": "energy", "card": "battery_sag", "plan": { "kind": "list", "runs": [base.clone(), base.clone()] }, "store": "none", "return": "inline" });
    let answer = ok(&mut door, "run_dataset", &list);
    assert_eq!(answer["head"]["completed"], 2, "{answer}");
    // A row whose reading was edited is not admitted, and says why.
    let mut bad = base.clone();
    bad["segments"][1]["load"]["current_a"] = json!(1.0);
    let list = json!({ "domain": "energy", "card": "battery_sag", "plan": { "kind": "list", "runs": [base.clone(), bad] }, "store": "none", "return": "inline", "answer": "full" });
    let answer = ok(&mut door, "run_dataset", &list);
    assert_eq!(answer["head"]["unadmitted"], 1, "{answer}");
    assert!(answer["rows"][1]["reject_detail"].as_str().unwrap().ends_with("(LoadSignatureDoesNotVerify)."), "{answer}");
    // A search over a signed value, one segment or every segment.
    for vary in ["segments[0].load.current_a", "segments[*].load.current_a", "segments[2].signed.seq"] {
        let said = refused(&mut door, "find_flip", &json!({ "request": request, "vary": vary, "low": 1.0, "high": 20.0 }));
        assert!(said.ends_with("(SignedLoadReplaced)."), "{vary}: {said}");
    }
}

#[test]
fn a_solve_with_signed_parameters_says_who_signed_them_and_an_edit_is_refused() {
    let mut door = local();
    let parameters = json!({ "atomic_number": 10 });
    let (signed, _) = sign(&Reading { key: &test_key("A"), name: SHUNT, seq: 7, prev: None, challenge: Some(CHALLENGE), second: 0 }, Place::Parameters, &parameters);
    let request = json!({ "solver": "atom_dft", "parameters": { "atomic_number": 10, "signed": signed } });
    let answer = ok(&mut door, "solve", &request);
    assert_eq!(answer["outcome"], "converged");
    assert_eq!(answer["load_source"]["parameters"], "signed");
    assert_eq!(answer["load_source"]["keys"][0]["fingerprint"], fingerprint(&test_key("A")));
    let plain = ok(&mut door, "solve", &json!({ "solver": "atom_dft", "parameters": parameters }));
    assert!(plain.get("load_source").is_none());
    assert_eq!(plain["result"], answer["result"]);
    assert_ne!(plain["solution_seal"], answer["solution_seal"]);
    let mut edited = request.clone();
    edited["parameters"]["atomic_number"] = json!(11);
    assert!(refused(&mut door, "solve", &edited).contains("'parameters.signed' does not verify"));
    // The long form holds the honest answer and names a forged signature.
    assert_eq!(ok(&mut door, "verify_solution", &answer)["holds"], true);
    let mut weak = answer.clone();
    weak["inputs"]["parameters"]["signed"]["seq"] = json!(8);
    let report = ok(&mut door, "verify_solution", &weak);
    assert_eq!(report["holds"], false);
    assert_eq!(report["cause"], "inputs");
}

#[test]
fn a_comparison_with_signed_measurements_says_whose_they_are() {
    let mut door = local();
    let run = signed_battery(&test_key("A"), 20, Some(CHALLENGE));
    let answer = ok(&mut door, "run_trajectory", &run);
    let model = |step: usize| {
        let hex = answer["frames"][step - 1]["frame_hex"].as_str().unwrap();
        let _ = hex;
        3.9
    };
    let mut measurements = Vec::new();
    let mut prev = None;
    for step in 1..=20_usize {
        let item = json!({ "step": step, "value": model(step) });
        let (signed, hash) = sign(&Reading { key: &test_key("B"), name: ADC, seq: 500 + step as u64, prev: prev.clone(), challenge: Some(CHALLENGE), second: step as u64 }, Place::Measurement(step - 1), &item);
        prev = Some(hash);
        let mut item = item.as_object().unwrap().clone();
        item.insert("signed".into(), signed);
        measurements.push(Value::Object(item));
    }
    let request = json!({ "run": run, "quantity": "soc", "measurements": measurements });
    let compared = ok(&mut door, "validate_series", &request);
    assert_eq!(compared["load_source"]["signed"], 20);
    assert_eq!(compared["measurement_source"]["signed"], 20);
    assert_eq!(compared["measurement_source"]["stream"], "unbroken");
    assert_eq!(compared["measurement_source"]["keys"][0]["names_itself"], ADC);
    let mut edited = request.clone();
    edited["measurements"][4]["value"] = json!(4.2);
    assert!(refused(&mut door, "validate_series", &edited).contains("'measurements[4].signed' does not verify"));
    // genesis_seal verify-validation rebuilds it with no engine.
    let report = genesis_seal::validation::check_validation(serde_json::to_string(&compared).unwrap().as_bytes()).unwrap();
    assert!(report.holds, "{report:?}");
    assert_eq!(report.signed_measurements.len(), 20);
    // With no signature the measurements are the caller's.
    let plain = ok(&mut door, "validate_series", &json!({ "run": unsigned(&run), "quantity": "soc", "measurements": [{ "step": 1, "value": 0.99 }] }));
    assert_eq!(plain["measurement_source"], json!({ "measurements": 1, "signed": 0, "says": attest::CALLERS_MEASUREMENTS }));
}

#[test]
fn find_model_points_at_signed_loads_when_a_description_speaks_of_a_spoofed_sensor() {
    let mut door = local();
    let answer = ok(&mut door, "find_model", &json!({ "description": "a battery pack whose current shunt may be spoofed; the readings are signed" }));
    assert!(answer["signed_loads"].as_str().unwrap().starts_with("A reading an instrument measured can be sent signed"));
}

struct Open {
    child: Child,
    port: u16,
}

impl Open {
    fn start() -> Open {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hosted_door"))
            .args(["--open", "--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut banner = String::new();
        for _ in 0..3 {
            out.read_line(&mut banner).unwrap();
        }
        let address = banner.split("running on ").nth(1).unwrap().split(' ').next().unwrap();
        let port = address.rsplit(':').next().unwrap().parse().unwrap();
        Open { child, port }
    }

    fn tool(&self, tool: &str, arguments: &Value) -> (bool, String) {
        let reply = parse_reply(&raw(self.port, &request_bytes(None, &tool_body(1, tool, arguments)), 120));
        assert_eq!(reply.status, 200, "{reply:?}");
        tool_result(&reply)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn web() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, protocol_mcp_gateway::app()).await.unwrap();
    });
    format!("http://127.0.0.1:{port}")
}

async fn post(base: &str, tool: &str, body: &Value) -> (u16, String) {
    let response = reqwest::Client::new()
        .post(format!("{base}/{tool}"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(answer_text::as_sent(body))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    answer_text::remember(&text);
    (status, text)
}

#[tokio::test]
async fn every_connection_gives_the_same_bytes() {
    let mut stdio = local();
    let request = signed_battery(&test_key("A"), 10, Some(CHALLENGE));
    let answer = ok(&mut stdio, "run_trajectory", &request);
    let mut edited = request.clone();
    edited["segments"][3]["load"]["current_a"] = json!(1.0);
    let base = web().await;
    let keyed = Hosted::start(
        vec![json!({
            "client": "all", "key_sha256": sha(KEY),
            "cards": [["energy", "battery_sag"]],
            "solvers": [], "max_in_flight": 4, "max_permits": 2
        })],
        None,
    );
    let open = Open::start();
    let calls = [
        ("run_trajectory", request.clone()),
        ("run_trajectory", edited.clone()),
        ("run_trajectory", unsigned(&request)),
        ("verify_tape", answer.clone()),
        ("verify_tape", forged(&answer, 3)),
        ("verify_tape", json!({ "request": request, "run_seal": answer["run_seal"] })),
        ("verify_tape", json!({ "request": edited, "run_seal": answer["run_seal"] })),
    ];
    for (tool, arguments) in &calls {
        let (error, local) = stdio.tool(tool, arguments);
        let (status, over_web) = post(&base, tool, arguments).await;
        assert_eq!(over_web, local, "{tool} web");
        assert_eq!(status == 200, !error, "{tool} web status {status}");
        let (status, hosted_error, hosted) = keyed.tool(KEY, tool, arguments);
        assert_eq!((status, hosted_error, hosted.as_str()), (200, error, local.as_str()), "{tool} hosted");
        let (open_error, opened) = open.tool(tool, arguments);
        assert_eq!((open_error, opened.as_str()), (error, local.as_str()), "{tool} open");
    }
}

fn seal_program() -> std::path::PathBuf {
    let seal_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join("../genesis_seal");
    let built = Command::new(env!("CARGO"))
        .args(["build", "--offline", "--quiet", "--manifest-path"])
        .arg(seal_crate.join("Cargo.toml"))
        .status()
        .unwrap();
    assert!(built.success());
    [seal_crate.join("target/debug/genesis_seal"), seal_crate.join("../target/debug/genesis_seal")]
        .into_iter()
        .find(|path| path.exists())
        .expect("the program of the sealing crate")
}

#[test]
fn the_program_of_genesis_seal_verifies_the_signed_answer_with_no_engine() {
    let mut door = local();
    let answer_text = door.tool("run_trajectory", &signed_battery(&test_key("A"), 5, Some(CHALLENGE))).1;
    let swapped_text = door.tool("run_trajectory", &signed_battery(&test_key("M"), 5, Some(CHALLENGE))).1;
    let folder = scratch("signed_load_program");
    std::fs::create_dir_all(&folder).unwrap();
    let write = |name: &str, text: &str| {
        let path = folder.join(name);
        std::fs::write(&path, text).unwrap();
        path
    };
    let trust = write(
        "bench.json",
        &json!({ "keys": [{ "key": key_hex(&test_key("A")), "name": "bench shunt, TEST KEY A", "quantities": ["current_a"] }] }).to_string(),
    );
    let program = seal_program();
    let run = |file: &Path| {
        let output = Command::new(&program).arg("verify-run").arg(file).arg("--trust").arg(&trust).output().unwrap();
        (output.status.code().unwrap(), String::from_utf8(output.stdout).unwrap())
    };
    let (code, out) = run(&write("honest.json", &answer_text));
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("holds: true\nbroken_link: none\ncause: none\nreceipt: matches\nsigned_loads: 5\nstream: unbroken\n"), "{out}");
    assert_eq!(out.matches("trusted: yes, \"bench shunt, TEST KEY A\"").count(), 5, "{out}");
    let (code, out) = run(&write("swapped.json", &swapped_text));
    assert_eq!(code, 4, "{out}");
    assert_eq!(out.matches("trusted: no: the key ").count(), 5, "{out}");
}
