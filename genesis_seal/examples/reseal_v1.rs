//! Convert a run answer of seal rule v1 (SHA-256) to the answer seal rule v2
//! (SHA-384) gives for the same request.
//!
//! ```text
//! cargo run --release -p genesis_seal --example reseal_v1 -- <out dir> <answer.json>...
//! ```
//!
//! The hash rule does not touch the physics: a frame's first 48 bytes, the
//! final state, the residuals, the outcome and the counts are the same under
//! both rules. So the v2 answer is the v1 answer with the proof slot of each
//! frame (its last 16 bytes) rewritten from the v2 link before it, every link,
//! the state digest, the run seal and each chain seal recomputed by SHA-384,
//! `receipt_sha256` renamed `receipt_sha384`, `seal_rule` written just before
//! it, and the receipt taken again over the new bytes. Every other byte stays
//! as it was. Files are converted in the order given, so a chain's links are
//! given in order and each link's `upstream_run_seal` and
//! `upstream_chain_seal` take the converted seals of the link before.
//!
//! Only runs and chains of runs are converted. A step's answer and a solution
//! carry no frame or field bytes to rehash; they are named and skipped.
//!
//! This is a tool for test fixtures and saved records, made by the squad
//! `seal_v2` on 18 October 2026: its output is what the engine of seal rule v2
//! answers to the same request only because the physics bytes are unchanged,
//! and the gateway's tests hold that to the engine.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use genesis_seal::chain::{Handed, chain_seal, chain_tail};
use genesis_seal::receipt::with_receipt;
use genesis_seal::run::{FRAME_LEN, link_0, run_seal, seal_frame, state_digest};
use genesis_seal::{SEAL_RULE, SEAL_RULE_KEY};
use serde_json::Value;

/// Old hash text to new, for every hash a converted file wrote.
type Moved = HashMap<String, String>;

fn sealed_request(inputs: &Value) -> Value {
    let mut request = inputs.clone();
    if let Some(object) = request.as_object_mut() {
        object.shift_remove("readout");
        object.shift_remove("from");
    }
    request
}

fn text_of(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Convert one run answer; `Err` names why it is not a run of rule v1.
fn convert(text: &str, moved: &mut Moved) -> Result<String, String> {
    let body: Value = serde_json::from_str(text).map_err(|error| format!("not JSON: {error}"))?;
    let object = body.as_object().ok_or("not a JSON object")?;
    if object.contains_key("receipt_sha384") || object.contains_key(SEAL_RULE_KEY) {
        return Err("already seal rule v2".into());
    }
    if object.contains_key("sha256_seal") {
        return Err("a step's answer: its frame bytes are not in it, so its seal is not rehashed here".into());
    }
    if object.contains_key("solution_seal") {
        return Err("a solution: its field values are not in it, so field_sha256 is not rehashed here".into());
    }
    let frames = object.get("frames").and_then(Value::as_array).ok_or("no frames: not a run")?;
    let inputs = object.get("inputs").ok_or("no inputs")?;
    let mut link = link_0(&sealed_request(inputs));
    let mut local = Moved::new();
    for frame in frames {
        let old_hex = text_of(frame, "frame_hex").ok_or("a frame without frame_hex")?;
        let old_link = text_of(frame, "link").ok_or("a frame without link")?;
        let bytes = hex::decode(&old_hex).map_err(|_| "a frame_hex that is not hex")?;
        let mut frame_bytes: [u8; FRAME_LEN] = bytes.try_into().map_err(|_| "a frame that is not 64 bytes")?;
        link = seal_frame(&link, &mut frame_bytes);
        local.insert(old_hex, hex::encode(frame_bytes));
        local.insert(old_link, hex::encode(link));
    }
    let steps_landed = object.get("steps_landed").and_then(Value::as_u64).ok_or("no steps_landed")?;
    let outcome = text_of(&body, "outcome").ok_or("no outcome")?;
    let reject = text_of(&body, "reject");
    let digest = if steps_landed == 0 {
        None
    } else {
        let state = object.get("final_state").and_then(Value::as_object).ok_or("no final_state")?;
        let final_hex = text_of(&body, "final_residual_conservation_error_hex").ok_or("no final residual")?;
        let max_hex = text_of(&body, "max_residual_conservation_error_hex").ok_or("no max residual")?;
        Some(state_digest(state, &final_hex, &max_hex).ok_or("a state digest that cannot be taken")?)
    };
    if let (Some(old), Some(new)) = (text_of(&body, "state_digest"), digest.clone()) {
        local.insert(old, new);
    }
    let seal = run_seal(&link, &outcome, steps_landed, reject.as_deref(), digest.as_deref());
    let old_seal = text_of(&body, "run_seal").ok_or("no run_seal")?;
    local.insert(old_seal, seal.clone());
    if let Some(chain) = object.get("chain").filter(|chain| !chain.is_null()) {
        let up_run = text_of(chain, "upstream_run_seal").ok_or("a chain without upstream_run_seal")?;
        let new_up_run = moved.get(&up_run).cloned().ok_or("a chain whose upstream was not converted before it")?;
        let up_chain = text_of(chain, "upstream_chain_seal");
        let new_up_chain = match &up_chain {
            Some(old) => Some(moved.get(old).cloned().ok_or("a chain whose upstream chain seal was not converted before it")?),
            None => None,
        };
        let handed: Vec<Handed> = chain
            .get("handed")
            .and_then(Value::as_array)
            .ok_or("a chain without handed")?
            .iter()
            .map(|entry| Handed {
                from: text_of(entry, "from").unwrap_or_default(),
                to: text_of(entry, "to").unwrap_or_default(),
                unit: text_of(entry, "unit"),
                value_hex: entry.get("value_hex").cloned().unwrap_or(Value::Null),
            })
            .collect();
        let kind = text_of(chain, "kind").ok_or("a chain without kind")?;
        let coupling = text_of(chain, "coupling");
        let commitment = new_up_chain.clone().unwrap_or_else(|| new_up_run.clone());
        let new_chain_seal = chain_seal(&commitment, &chain_tail(&kind, coupling.as_deref(), &handed), &seal);
        local.insert(text_of(chain, "chain_seal").ok_or("a chain without chain_seal")?, new_chain_seal);
        local.insert(up_run, new_up_run);
        if let (Some(old), Some(new)) = (up_chain, new_up_chain) {
            local.insert(old, new);
        }
    }
    let mut out = rewrite_hashes(text, &local);
    moved.extend(local);
    out = out.replace("\"receipt_sha256\":\"", "\"receipt_sha384\":\"");
    let rule = format!(",\"{SEAL_RULE_KEY}\":\"{SEAL_RULE}\"");
    match out.rfind(",\"receipt_sha384\":\"") {
        Some(at) => {
            let mut document = out[..at].to_string();
            document.push_str(&rule);
            document.push('}');
            Ok(with_receipt(&document))
        }
        None => {
            let trimmed = out.trim_end();
            let body = trimmed.strip_suffix('}').ok_or("a text that does not end with }")?;
            Ok(format!("{body}{rule}}}"))
        }
    }
}

/// Every quoted string of 64 or 128 lowercase hex digits that `moved` names,
/// replaced by its new text; every other byte as it was.
fn rewrite_hashes(text: &str, moved: &Moved) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + text.len() / 2);
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'"' {
            let start = at + 1;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_digit() || (b'a'..=b'f').contains(&bytes[end])) {
                end += 1;
            }
            if end < bytes.len() && bytes[end] == b'"' && (end - start == 64 || end - start == 128) {
                if let Some(new) = moved.get(&text[start..end]) {
                    out.push('"');
                    out.push_str(new);
                    out.push('"');
                    at = end + 1;
                    continue;
                }
            }
        }
        // Copy one UTF-8 character.
        let width = utf8_width(bytes[at]);
        out.push_str(&text[at..at + width]);
        at += width;
    }
    out
}

fn utf8_width(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(out_dir) = args.next() else {
        eprintln!("usage: reseal_v1 <out dir> <answer.json>... (each written to <out dir>/<its file name>, or with --tree <root>, to its path under root)");
        std::process::exit(2);
    };
    let mut rest: Vec<String> = args.collect();
    let mut root: Option<PathBuf> = None;
    if rest.first().map(String::as_str) == Some("--tree") {
        rest.remove(0);
        root = Some(PathBuf::from(rest.remove(0)));
    }
    let mut moved = Moved::new();
    let mut converted = 0;
    let mut skipped = 0;
    for path in rest {
        let path = PathBuf::from(path);
        let target = match &root {
            Some(root) => Path::new(&out_dir).join(path.strip_prefix(root).unwrap_or(&path)),
            None => Path::new(&out_dir).join(path.file_name().expect("a file name")),
        };
        let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{} not read: {error}", path.display()));
        match convert(&text, &mut moved) {
            Ok(new) => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).expect("the out folder is made");
                }
                std::fs::write(&target, new).expect("the converted answer is written");
                converted += 1;
                println!("converted: {}", path.display());
            }
            Err(why) => {
                skipped += 1;
                println!("skipped: {}: {why}", path.display());
            }
        }
    }
    println!("converted {converted}, skipped {skipped}");
}
