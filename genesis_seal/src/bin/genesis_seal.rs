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
//! the commitment's sha256, the one to hold beside the sha256 published
//! before the comparison; data that do not open it are `cause: commitment`,
//! and a `commitment_opened` said with no commitment in the inputs, or left
//! out beside one, is `cause: commitment_opened`.
//!
//! `verify-run`, `verify-validation` and `verify-dataset` take `--trust
//! <file>`, the verifier's own list of the keys it trusts. Each signed
//! reading is printed with its key, the name it gives itself, its time, its
//! seq, its challenge and what the list says of it. Exit code 4: every seal
//! holds and a signed reading is not one the list trusts.

use std::path::Path;
use std::process::ExitCode;

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
  genesis_seal verify-run <tape file> [--trust <trust file>]
  genesis_seal verify-chain <chain file>
  genesis_seal verify-validation <answer file> [--trust <trust file>]
  genesis_seal verify-dataset <head file> <rows file> [frame file] [--trust <trust file>]
  genesis_seal row-proof <head file> <rows file> <row number> [frame file]
  genesis_seal verify-proof <proof file>
  genesis_seal expand <request file> <row number>";

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
    let code = match arguments.as_slice() {
        ["verify-run", tape] => verify_run(tape, trust),
        ["verify-chain", chain] if trust.is_none() => verify_chain(chain),
        ["verify-validation", answer] => verify_validation(answer, trust),
        ["verify-dataset", head, rows] => verify_dataset(head, rows, None, trust),
        ["verify-dataset", head, rows, frames] => verify_dataset(head, rows, Some(frames), trust),
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
    eprintln!("refused: {refusal}");
    REFUSED
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

fn verify_run(tape: &str, trust: Option<&Trust>) -> Result<u8, u8> {
    let bytes = read_file(tape)?;
    let report = check_tape_text(&bytes).map_err(refused)?;
    let receipt = check_receipt(&bytes);
    println!("holds: {}", report.holds);
    println!("broken_link: {}", or_none(report.broken_link));
    println!("cause: {}", or_none(report.cause));
    println!("receipt: {}", receipt.as_str());
    print_signature_fault(&report.broken_place, &report.load_fault);
    // The readings are listed only for a tape that holds: a broken tape's
    // readings are not the ones it would have to hold.
    let untrusted = report.holds && print_signed("signed_loads", &report.signed_loads, trust);
    Ok(if !report.holds {
        BROKEN
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
fn verify_chain(chain: &str) -> Result<u8, u8> {
    let bytes = read_file(chain)?;
    let report = check_chain_text(&bytes).map_err(refused)?;
    println!("holds: {}", report.holds);
    println!("links: {}", report.links);
    println!("broken_link: {}", or_none(report.broken_link));
    println!("cause: {}", or_none(report.cause));
    println!("chain_seal: {}", or_none(report.chain_seal.as_deref()));
    if report.last_is_step {
        println!("note: the last link is a step; its answer holds no frame, so its sha256_seal is shown by sending its request again.");
    }
    Ok(if report.holds { HOLDS } else { BROKEN })
}

fn verify_validation(answer: &str, trust: Option<&Trust>) -> Result<u8, u8> {
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
    println!("measurements_sha256: {}", report.measurements_sha256);
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
    if let (Some(committed), Some(recomputed)) = (&report.commitment_sha256, &report.commitment_recomputed) {
        println!("commitment_sha256: {committed}");
        if committed == recomputed {
            println!("commitment: opens");
        } else {
            println!("commitment: does not open: the SHA-256 of the tag, the answer's committed text and the salt is {recomputed}");
        }
    }
    println!("receipt: {}", receipt.as_str());
    if receipt == ReceiptCheck::Absent {
        if report.commitment_sha256.is_some() {
            println!("{RECEIPT_ABSENT_NOTE_COMMITTED}");
        } else {
            println!("{RECEIPT_ABSENT_NOTE}");
        }
    }
    print_signature_fault(&report.broken_place, &report.load_fault);
    let mut untrusted = report.holds && print_signed("signed_loads", &report.signed_loads, trust);
    untrusted |= report.holds && print_signed("signed_measurements", &report.signed_measurements, trust);
    Ok(if !report.holds {
        BROKEN
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

fn verify_dataset(head: &str, rows: &str, frames: Option<&str>, trust: Option<&Trust>) -> Result<u8, u8> {
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
    } else if bytes_differ(&report, fault.as_ref()) {
        BYTES_DIFFER
    } else if untrusted {
        NOT_TRUSTED
    } else {
        HOLDS
    })
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
