//! Verify a run, a dataset or one row without the engine, and expand a plan.
//!
//! Each answer is plain text, one `name: value` line each. The exit code is
//! 0 when everything given holds, 1 when a seal is broken, 2 when the input
//! was refused or could not be read, and 3 when every seal holds and the
//! bytes are not the engine's: a file is not, byte for byte, the one its
//! receipt or the manifest head names, or a decimal of a dataset row is not
//! the value of the hex beside it.
//!
//! `verify-validation` opens a commitment the comparison carries again from
//! the answer's own measurements, quantity, component and salt, and prints
//! the commitment's sha384, the one to hold beside the sha384 published
//! before the comparison; data that do not open it are `cause: commitment`,
//! and a `commitment_opened` said with no commitment in the inputs, or left
//! out beside one, is `cause: commitment_opened`.
//!
//! `verify-run`, `verify-validation` and `verify-dataset` take `--trust
//! <file>`, the verifier's own list of the keys it trusts. Each signed
//! reading is printed with its key, the name it gives itself, its time, its
//! seq, its challenge and what the list says of it. Exit code 4: every seal
//! holds and a signed reading is not one the list trusts.
//!
//! The engine's signature on an answer (ML-DSA-87 over its receipt,
//! `genesis_seal::answer_signature`) is checked by `verify-run`,
//! `verify-chain` (each link), `verify-validation`, `verify-dataset` (the
//! head) and `verify-signature`, against `--public-key <file>` (the base64 of
//! the engine's 2,592-byte key), at `--now <YYYY-MM-DDTHH:MM:SSZ>` or this
//! machine's clock, with `--nonce-ledger <file>`, the verifier's own list of
//! the nonces it has seen, one per line: a nonce found there is
//! `SignatureNonceReplayed`, and a signature that holds has its nonce added.
//! The lines `signature`, `signature_cause`, `key_id`, `issued_at`,
//! `expires_at` and `nonce` are printed for an answer that carries a
//! signature or when a key is given. Exit code 5: every seal holds and the
//! signature does not; it is said before 3 and 4. A record of seal rule v1
//! (SHA-256, engine 0.14.3 and earlier) is not read: `cause:
//! SealRuleNotServed`, exit code 2.

use std::path::Path;
use std::process::ExitCode;

use genesis_seal::answer_signature::{self, Check, PublicKey, SignatureReport};
use genesis_seal::attest::{Trust, Trusted, Verified, stream};
use genesis_seal::chain::check_chain_text;
use genesis_seal::dataset::{DatasetReport, check_dataset_stream, check_row_proof, row_proof};
use genesis_seal::fault::{Fault, bytes_differ, fault_of, report_lines};
use genesis_seal::plan::DatasetRequest;
use genesis_seal::receipt::{ReceiptCheck, check_receipt};
use genesis_seal::run::check_tape_text;
use genesis_seal::stored_file::open_regular;
use genesis_seal::validation::{ValidationCause, check_validation};

const USAGE: &str = "usage:
  genesis_seal verify-run <tape file> [--trust <trust file>] [signature options]
  genesis_seal verify-chain <chain file> [signature options]
  genesis_seal verify-validation <answer file> [--trust <trust file>] [signature options]
  genesis_seal verify-dataset <head file> <rows file> [frame file] [--trust <trust file>] [signature options]
  genesis_seal verify-signature <answer file> --public-key <key file> [--now <time>] [--nonce-ledger <file>]
  genesis_seal row-proof <head file> <rows file> <row number> [frame file]
  genesis_seal verify-proof <proof file>
  genesis_seal expand <request file> <row number>
signature options: --public-key <key file> --now <YYYY-MM-DDTHH:MM:SSZ> --nonce-ledger <file>";

/// Said whenever `verify-validation` finds no receipt at the end of the answer.
const RECEIPT_ABSENT_NOTE: &str = "note: the receipt is absent, so the bytes of this answer are not covered. The seal covers the measurements, the quantity, the component, the outcome, steps_landed, reject, the counts, the model values and the three statistics in hex. Only the receipt covers the decimal copies of the statistics, engine_version, reject_detail, does_not_show, and the run and any other field inside inputs. A change to one of those is not seen without it.";

/// Said in its place when the answer carries a commitment, which the seal covers too.
const RECEIPT_ABSENT_NOTE_COMMITTED: &str = "note: the receipt is absent, so the bytes of this answer are not covered. The seal covers the measurements, the quantity, the component, the commitment, the outcome, steps_landed, reject, the counts, the model values and the three statistics in hex. Only the receipt covers the decimal copies of the statistics, engine_version, reject_detail, does_not_show, commitment_opened, and the run and any other field inside inputs. A change to one of those is not seen without it.";

/// The most rows of a plan one search for a row's place writes out: the
/// gateway's most rows in a dataset.
const SEARCH_MOST_ROWS: u64 = 100_000;

const HOLDS: u8 = 0;
const BROKEN: u8 = 1;
const REFUSED: u8 = 2;
const BYTES_DIFFER: u8 = 3;
const NOT_TRUSTED: u8 = 4;
const SIGNATURE_DOES_NOT_HOLD: u8 = 5;

/// What the verifier brings to check the engine's signature.
struct SignatureGiven {
    key: Option<PublicKey>,
    now_s: i64,
    ledger: Option<String>,
    seen: Vec<String>,
}

/// Take `--<name> <value>` out of the arguments, at most once, after the command.
fn take_option(arguments: &mut Vec<&str>, name: &str) -> Result<Option<String>, u8> {
    let flag = format!("--{name}");
    let count = arguments.iter().filter(|argument| **argument == flag).count();
    match arguments.iter().position(|argument| *argument == flag) {
        None => Ok(None),
        Some(at) if count == 1 && at >= 1 && at + 1 < arguments.len() => {
            let value = arguments[at + 1].to_string();
            arguments.drain(at..at + 2);
            Ok(Some(value))
        }
        Some(_) => {
            eprintln!("{USAGE}");
            Err(REFUSED)
        }
    }
}

fn read_signature_options(arguments: &mut Vec<&str>) -> Result<SignatureGiven, u8> {
    let key = match take_option(arguments, "public-key")? {
        None => None,
        Some(path) => {
            let bytes = read_file(&path)?;
            let key = std::str::from_utf8(&bytes).ok().and_then(PublicKey::from_base64);
            match key {
                Some(key) => Some(key),
                None => {
                    eprintln!("refused: the public key '{path}' is not the base64 of an ML-DSA-87 public key (2,592 bytes).");
                    return Err(REFUSED);
                }
            }
        }
    };
    let now_s = match take_option(arguments, "now")? {
        None => answer_signature::now_s(),
        Some(text) => match answer_signature::parse_time(&text) {
            Some(seconds) => seconds,
            None => {
                eprintln!("refused: '--now {text}' is not a time written YYYY-MM-DDTHH:MM:SSZ.");
                return Err(REFUSED);
            }
        },
    };
    let ledger = take_option(arguments, "nonce-ledger")?;
    let seen = match &ledger {
        None => Vec::new(),
        Some(path) => match std::fs::read_to_string(path) {
            Ok(text) => text.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_string).collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                eprintln!("refused: the nonce ledger '{path}' could not be read: {error}");
                return Err(REFUSED);
            }
        },
    };
    Ok(SignatureGiven { key, now_s, ledger, seen })
}

impl SignatureGiven {
    fn check(&self, text: &str) -> SignatureReport {
        answer_signature::check(text, Check { public_key: self.key.as_ref(), now_s: self.now_s, seen_nonces: &self.seen })
    }

    /// Print the signature lines of one answer, when it carries a signature
    /// or a key was given; add its nonce to the ledger when it holds. True
    /// when a signature was checked and does not hold.
    fn print(&self, label: Option<&str>, report: &SignatureReport) -> Result<bool, u8> {
        if !report.present && self.key.is_none() {
            return Ok(false);
        }
        let prefix = label.map(|label| format!("{label}.")).unwrap_or_default();
        let said = match (report.present, report.cause) {
            (false, _) => "absent",
            (true, Some(answer_signature::SignatureCause::KeyNotGiven)) => "not checked",
            (true, None) => "holds",
            (true, Some(_)) => "does not hold",
        };
        println!("{prefix}signature: {said}");
        match report.cause {
            Some(answer_signature::SignatureCause::KeyNotGiven) => {
                println!("{prefix}signature_note: no --public-key was given, so the signature was not checked.");
            }
            Some(cause) => {
                println!("{prefix}signature_cause: {}", cause.name());
                println!("{prefix}signature_fault: {}", cause.sentence());
            }
            None => {}
        }
        if let Some(envelope) = &report.envelope {
            println!("{prefix}key_id: {}", envelope.key_id);
            println!("{prefix}issued_at: {}", envelope.issued_at);
            println!("{prefix}expires_at: {}", envelope.expires_at);
            println!("{prefix}nonce: {}", envelope.nonce);
        }
        if report.holds() {
            if let (Some(path), Some(envelope)) = (&self.ledger, &report.envelope) {
                use std::io::Write;
                let written = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .and_then(|mut file| file.write_all(format!("{}\n", envelope.nonce).as_bytes()));
                if let Err(error) = written {
                    eprintln!("refused: the nonce ledger '{path}' could not be written: {error}");
                    return Err(REFUSED);
                }
            }
        }
        Ok(self.key.is_some() && !report.holds())
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    // `--trust <file>`, at most once, after the command.
    let trust_at = arguments.iter().position(|argument| *argument == "--trust");
    let trust = match trust_at {
        Some(at) if at >= 1 && at + 1 < arguments.len() && arguments.iter().filter(|a| **a == "--trust").count() == 1 => {
            let path = arguments[at + 1];
            arguments.drain(at..at + 2);
            match read_trust(path) {
                Ok(trust) => Some(trust),
                Err(code) => return ExitCode::from(code),
            }
        }
        Some(_) => {
            eprintln!("{USAGE}");
            return ExitCode::from(REFUSED);
        }
        None => None,
    };
    let trust = trust.as_ref();
    let signing = match read_signature_options(&mut arguments) {
        Ok(signing) => signing,
        Err(code) => return ExitCode::from(code),
    };
    let signing = &signing;
    let code = match arguments.as_slice() {
        ["verify-run", tape] => verify_run(tape, trust, signing),
        ["verify-chain", chain] if trust.is_none() => verify_chain(chain, signing),
        ["verify-validation", answer] => verify_validation(answer, trust, signing),
        ["verify-dataset", head, rows] => verify_dataset(head, rows, None, trust, signing),
        ["verify-dataset", head, rows, frames] => verify_dataset(head, rows, Some(frames), trust, signing),
        ["verify-signature", answer] if trust.is_none() && signing.key.is_some() => verify_signature(answer, signing),
        ["row-proof", head, rows, row] => write_row_proof(head, rows, row, None),
        ["row-proof", head, rows, row, frames] => write_row_proof(head, rows, row, Some(frames)),
        // `check-proof` is the earlier name of `verify-proof`, kept for the
        // programs published with it.
        ["verify-proof" | "check-proof", proof] => verify_proof(proof),
        ["expand", request, row] => expand(request, row),
        _ => {
            eprintln!("{USAGE}");
            Err(REFUSED)
        }
    };
    ExitCode::from(match code {
        Ok(code) | Err(code) => code,
    })
}

/// A file given on the command line must be a regular file reached without a
/// link. It is read for no more bytes than its size when it was opened.
fn read_file(path: &str) -> Result<Vec<u8>, u8> {
    let file = open_regular(Path::new(path)).map_err(refused)?;
    file.read_all().map_err(|error| {
        eprintln!("refused: '{path}' could not be read: {error}");
        REFUSED
    })
}

fn read_row_number(text: &str) -> Result<u64, u8> {
    let is_whole = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    text.parse()
        .ok()
        .filter(|_| is_whole)
        .ok_or_else(|| {
            eprintln!("refused: '{text}' is not a row number.");
            REFUSED
        })
}

fn refused(refusal: impl std::fmt::Display) -> u8 {
    let text = refusal.to_string();
    if text.contains(genesis_seal::refusal::SEAL_RULE_NOT_SERVED) {
        println!("cause: {}", genesis_seal::refusal::SEAL_RULE_NOT_SERVED);
    }
    eprintln!("refused: {text}");
    REFUSED
}

fn text_of(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap_or("")
}

fn or_none<T: std::fmt::Display>(value: Option<T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "none".to_string(),
    }
}

fn read_trust(path: &str) -> Result<Trust, u8> {
    let bytes = read_file(path)?;
    Trust::read(&bytes).map_err(|why| {
        eprintln!("refused: the trust list '{path}' is not read: {why}.");
        REFUSED
    })
}

/// Where a signed reading does not hold: its place and the sentence.
fn print_signature_fault(place: &Option<String>, fault: &Option<String>) {
    if let (Some(place), Some(fault)) = (place, fault) {
        println!("broken_place: {place}");
        println!("load_fault: {fault}");
    }
}

/// One block for each verified signed reading of one run or one series,
/// after the count and the stream. True when a list was given and a reading
/// is not one it trusts.
fn print_signed(label: &str, readings: &[Verified], trust: Option<&Trust>) -> bool {
    if readings.is_empty() {
        if trust.is_some() {
            println!("{label}: 0");
        }
        return false;
    }
    println!("{label}: {}", readings.len());
    println!("stream: {}", stream(readings).text());
    let rows: Vec<(Option<u64>, &Verified)> = readings.iter().map(|reading| (None, reading)).collect();
    print_readings(&rows, trust)
}

fn print_readings(readings: &[(Option<u64>, &Verified)], trust: Option<&Trust>) -> bool {
    let mut untrusted = false;
    for (row, reading) in readings {
        println!("load_signature: holds");
        if let Some(row) = row {
            println!("row: {row}");
        }
        println!("place: {}", reading.place);
        println!("key: {}", reading.key);
        println!("fingerprint: {}", reading.fingerprint);
        // The name is printed as JSON text: it is the instrument's own claim.
        println!("name: {}", serde_json::Value::String(reading.instrument.clone()));
        println!("time: {}", reading.time);
        println!("seq: {}", reading.seq);
        println!("challenge: {}", or_none(reading.challenge.as_deref()));
        let said = match trust {
            None => "no list given".to_string(),
            Some(trust) => {
                let judged = trust.judge(reading);
                untrusted |= matches!(judged, Trusted::No(_));
                judged.text()
            }
        };
        println!("trusted: {said}");
    }
    untrusted
}

fn verify_run(tape: &str, trust: Option<&Trust>, signing: &SignatureGiven) -> Result<u8, u8> {
    let bytes = read_file(tape)?;
    let report = check_tape_text(&bytes).map_err(refused)?;
    let receipt = check_receipt(&bytes);
    println!("holds: {}", report.holds);
    println!("broken_link: {}", or_none(report.broken_link));
    println!("cause: {}", or_none(report.cause));
    println!("receipt: {}", receipt.as_str());
    let signature_fails = signing.print(None, &signing.check(text_of(&bytes)))?;
    print_signature_fault(&report.broken_place, &report.load_fault);
    // The readings are listed only for a tape that holds: a broken tape's
    // readings are not the ones it would have to hold.
    let untrusted = report.holds && print_signed("signed_loads", &report.signed_loads, trust);
    Ok(if !report.holds {
        BROKEN
    } else if signature_fails {
        SIGNATURE_DOES_NOT_HOLD
    } else if receipt == ReceiptCheck::DoesNotMatch {
        BYTES_DIFFER
    } else if untrusted {
        NOT_TRUSTED
    } else {
        HOLDS
    })
}

/// A chain: `{"links": [<answer>, ...]}`, the answers in order. Every run
/// is checked alone, then each link against the one before it.
fn verify_chain(chain: &str, signing: &SignatureGiven) -> Result<u8, u8> {
    let bytes = read_file(chain)?;
    let report = check_chain_text(&bytes).map_err(refused)?;
    let links = link_texts(text_of(&bytes));
    println!("holds: {}", report.holds);
    println!("links: {}", report.links);
    println!("broken_link: {}", or_none(report.broken_link));
    println!("cause: {}", or_none(report.cause));
    println!("chain_seal: {}", or_none(report.chain_seal.as_deref()));
    if report.last_is_step {
        println!("note: the last link is a step; its answer holds no frame, so its sha384_seal is shown by sending its request again.");
    }
    let mut signature_fails = false;
    for (index, link) in links.iter().enumerate() {
        signature_fails |= signing.print(Some(&format!("links[{index}]")), &signing.check(link))?;
    }
    Ok(if !report.holds {
        BROKEN
    } else if signature_fails {
        SIGNATURE_DOES_NOT_HOLD
    } else {
        HOLDS
    })
}

fn verify_validation(answer: &str, trust: Option<&Trust>, signing: &SignatureGiven) -> Result<u8, u8> {
    let bytes = read_file(answer)?;
    let report = check_validation(&bytes).map_err(refused)?;
    let receipt = check_receipt(&bytes);
    println!("holds: {}", report.holds);
    println!(
        "cause: {}",
        or_none(report.cause.map(|cause| cause.as_str()))
    );
    println!("compared: {}", report.compared);
    println!("not_reached: {}", report.not_reached);
    match &report.rebuilt {
        Some(statistics) => {
            println!("rms_error_hex: {}", statistics.rms_error_hex());
            println!("mean_signed_error_hex: {}", statistics.mean_signed_error_hex());
            println!("max_abs_error_hex: {}", statistics.max_abs_error_hex());
            println!("max_abs_error_step: {}", statistics.max_abs_error_step);
        }
        None => {
            println!("rms_error_hex: none");
            println!("mean_signed_error_hex: none");
            println!("max_abs_error_hex: none");
            println!("max_abs_error_step: none");
        }
    }
    println!("measurements_sha384: {}", report.measurements_sha384);
    println!("validation_seal: {}", report.validation_seal);
    // Said only of an answer whose commitment_opened does not follow from its
    // inputs, and of one that carries a commitment, so the lines of every
    // other answer are as they were.
    if report.cause == Some(ValidationCause::CommitmentOpened) {
        if report.commitment_opened {
            println!("commitment_opened: said, and the inputs carry no commitment");
        } else {
            println!("commitment_opened: not said, and the inputs carry a commitment");
        }
    }
    if let (Some(committed), Some(recomputed)) = (&report.commitment_sha384, &report.commitment_recomputed) {
        println!("commitment_sha384: {committed}");
        if committed == recomputed {
            println!("commitment: opens");
        } else {
            println!("commitment: does not open: the SHA-384 of the tag, the answer's committed text and the salt is {recomputed}");
        }
    }
    println!("receipt: {}", receipt.as_str());
    if receipt == ReceiptCheck::Absent {
        if report.commitment_sha384.is_some() {
            println!("{RECEIPT_ABSENT_NOTE_COMMITTED}");
        } else {
            println!("{RECEIPT_ABSENT_NOTE}");
        }
    }
    let signature_fails = signing.print(None, &signing.check(text_of(&bytes)))?;
    print_signature_fault(&report.broken_place, &report.load_fault);
    let mut untrusted = report.holds && print_signed("signed_loads", &report.signed_loads, trust);
    untrusted |= report.holds && print_signed("signed_measurements", &report.signed_measurements, trust);
    Ok(if !report.holds {
        BROKEN
    } else if signature_fails {
        SIGNATURE_DOES_NOT_HOLD
    } else if receipt == ReceiptCheck::DoesNotMatch {
        BYTES_DIFFER
    } else if untrusted {
        NOT_TRUSTED
    } else {
        HOLDS
    })
}

fn print_dataset(report: &DatasetReport, fault: Option<&Fault>) {
    print!("{}", report_lines(report, fault));
}

fn verify_dataset(head: &str, rows: &str, frames: Option<&str>, trust: Option<&Trust>, signing: &SignatureGiven) -> Result<u8, u8> {
    let head = read_file(head)?;
    let rows = read_file(rows)?;
    // The frame file is read as a stream, never held whole.
    let frames = frames
        .map(|path| {
            open_regular(Path::new(path))
                .map(|file| std::io::BufReader::with_capacity(1 << 20, file.reader()))
                .map_err(refused)
        })
        .transpose()?;
    let report = check_dataset_stream(&head, &rows, frames).map_err(refused)?.report;
    // The rows of a dropped or repeated row, and a decimal that is not its
    // hex, are named as the tool verify_dataset names them.
    let fault = fault_of(&head, &rows, &report, SEARCH_MOST_ROWS);
    print_dataset(&report, fault.as_ref());
    let signature_fails = signing.print(None, &signing.check(text_of(&head)))?;
    // The signed readings of every row, once the rows hold: a row's request
    // is an ordinary run request.
    // Each row is its own run, so no stream is read across rows.
    let mut untrusted = false;
    if report.holds {
        let mut readings = Vec::new();
        for (index, line) in genesis_seal::rows::lines(&rows).unwrap_or_default().into_iter().enumerate() {
            let request = serde_json::from_slice::<serde_json::Value>(line).ok().and_then(|row| row.get("request").cloned());
            if let Some(Ok(verified)) = request.as_ref().map(genesis_seal::attest::verify_run) {
                readings.extend(verified.into_iter().map(|reading| (index as u64, reading)));
            }
        }
        if !readings.is_empty() || trust.is_some() {
            println!("signed_loads: {}", readings.len());
        }
        let tagged: Vec<(Option<u64>, &Verified)> = readings.iter().map(|(row, reading)| (Some(*row), reading)).collect();
        untrusted = print_readings(&tagged, trust);
    }
    Ok(if !report.holds {
        BROKEN
    } else if signature_fails {
        SIGNATURE_DOES_NOT_HOLD
    } else if bytes_differ(&report, fault.as_ref()) {
        BYTES_DIFFER
    } else if untrusted {
        NOT_TRUSTED
    } else {
        HOLDS
    })
}

/// The engine's signature on any answer, with its receipt, and nothing else.
fn verify_signature(answer: &str, signing: &SignatureGiven) -> Result<u8, u8> {
    let bytes = read_file(answer)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| refused("the answer is not UTF-8 text."))?;
    let report = signing.check(text);
    let receipt = check_receipt(&bytes);
    println!("receipt: {}", receipt.as_str());
    let fails = signing.print(None, &report)?;
    Ok(if fails { SIGNATURE_DOES_NOT_HOLD } else { HOLDS })
}

/// The text of each answer of a chain file, `{"links": [<answer>, ...]}`, as
/// it stands in the file. Empty when the file is not of that form; the chain
/// check has refused it by then.
fn link_texts(text: &str) -> Vec<&str> {
    let Some(start) = text.find('[') else { return Vec::new() };
    let bytes = text.as_bytes();
    let mut links = Vec::new();
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut opened = None;
    for (index, &byte) in bytes.iter().enumerate().skip(start + 1) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                if depth == 0 {
                    opened = Some(index);
                }
                depth += 1;
            }
            b'}' | b']' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                if depth == 0 {
                    if let Some(open) = opened.take() {
                        links.push(&text[open..=index]);
                    }
                }
            }
            _ => {}
        }
    }
    links
}

fn write_row_proof(head: &str, rows: &str, row: &str, frames: Option<&str>) -> Result<u8, u8> {
    let head = read_file(head)?;
    let rows = read_file(rows)?;
    let row = read_row_number(row)?;
    let frames = frames.map(read_file).transpose()?;
    match row_proof(&head, &rows, frames.as_deref(), row) {
        Ok(proof) => {
            println!("{}", proof.to_text());
            Ok(HOLDS)
        }
        Err(genesis_seal::dataset::DatasetRefusal::DoesNotHold(report)) => {
            let fault = fault_of(&head, &rows, &report, SEARCH_MOST_ROWS);
            print_dataset(&report, fault.as_ref());
            Err(BROKEN)
        }
        Err(refusal) => Err(refused(refusal)),
    }
}

fn verify_proof(proof: &str) -> Result<u8, u8> {
    let bytes = read_file(proof)?;
    let report = check_row_proof(&bytes).map_err(refused)?;
    println!("holds: {}", report.holds);
    println!("cause: {}", or_none(report.cause));
    println!("row: {}", report.row);
    println!("broken_link: {}", or_none(report.broken_link));
    println!(
        "chain: {}",
        if report.chain_rebuilt {
            "rebuilt from the frames"
        } else {
            "not rebuilt: the proof carries no frames"
        }
    );
    println!("dataset_seal: {}", report.dataset_seal);
    Ok(if report.holds { HOLDS } else { BROKEN })
}

fn expand(request: &str, row: &str) -> Result<u8, u8> {
    let bytes = read_file(request)?;
    let row = read_row_number(row)?;
    let request = DatasetRequest::read(&bytes).map_err(refused)?;
    let expanded = request.row(row).map_err(refused)?;
    match serde_json::to_string(&expanded) {
        Ok(text) => {
            println!("{text}");
            Ok(HOLDS)
        }
        Err(error) => Err(refused(error)),
    }
}
