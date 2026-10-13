//! Check a run, a dataset or one row without the engine, and expand a plan.
//!
//! Each answer is plain text, one `name: value` line each. The exit code is
//! 0 when everything given holds, 1 when a seal is broken, 2 when the input
//! was refused or could not be read, and 3 when every seal holds and a file
//! is not, byte for byte, the one its receipt or the manifest head names.

use std::path::Path;
use std::process::ExitCode;

use genesis_seal::chain::check_chain_text;
use genesis_seal::dataset::{
    DatasetReport, FileCheck, check_dataset_stream, check_row_proof, row_proof,
};
use genesis_seal::plan::DatasetRequest;
use genesis_seal::receipt::{ReceiptCheck, check_receipt};
use genesis_seal::run::check_tape_text;
use genesis_seal::stored_file::open_regular;
use genesis_seal::validation::check_validation;

const USAGE: &str = "usage:
  genesis_seal verify-run <tape file>
  genesis_seal verify-chain <chain file>
  genesis_seal verify-validation <answer file>
  genesis_seal verify-dataset <head file> <rows file> [frame file]
  genesis_seal row-proof <head file> <rows file> <row number> [frame file]
  genesis_seal check-proof <proof file>
  genesis_seal expand <request file> <row number>";

/// Said whenever `verify-validation` finds no receipt at the end of the answer.
const RECEIPT_ABSENT_NOTE: &str = "note: the receipt is absent, so the bytes of this answer are not covered. The seal covers the measurements, the quantity, the component, the outcome, steps_landed, reject, the counts, the model values and the three statistics in hex. Only the receipt covers the decimal copies of the statistics, engine_version, reject_detail, does_not_show, and the run and any other field inside inputs. A change to one of those is not seen without it.";

const HOLDS: u8 = 0;
const BROKEN: u8 = 1;
const REFUSED: u8 = 2;
const BYTES_DIFFER: u8 = 3;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let code = match arguments.as_slice() {
        ["verify-run", tape] => verify_run(tape),
        ["verify-chain", chain] => verify_chain(chain),
        ["verify-validation", answer] => verify_validation(answer),
        ["verify-dataset", head, rows] => verify_dataset(head, rows, None),
        ["verify-dataset", head, rows, frames] => verify_dataset(head, rows, Some(frames)),
        ["row-proof", head, rows, row] => write_row_proof(head, rows, row, None),
        ["row-proof", head, rows, row, frames] => write_row_proof(head, rows, row, Some(frames)),
        ["check-proof", proof] => check_proof(proof),
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

fn verify_run(tape: &str) -> Result<u8, u8> {
    let bytes = read_file(tape)?;
    let report = check_tape_text(&bytes).map_err(refused)?;
    let receipt = check_receipt(&bytes);
    println!("holds: {}", report.holds);
    println!("broken_link: {}", or_none(report.broken_link));
    println!("cause: {}", or_none(report.cause));
    println!("receipt: {}", receipt.as_str());
    Ok(if !report.holds {
        BROKEN
    } else if receipt == ReceiptCheck::DoesNotMatch {
        BYTES_DIFFER
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

fn verify_validation(answer: &str) -> Result<u8, u8> {
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
    println!("receipt: {}", receipt.as_str());
    if receipt == ReceiptCheck::Absent {
        println!("{RECEIPT_ABSENT_NOTE}");
    }
    Ok(if !report.holds {
        BROKEN
    } else if receipt == ReceiptCheck::DoesNotMatch {
        BYTES_DIFFER
    } else {
        HOLDS
    })
}

fn print_dataset(report: &DatasetReport) {
    println!("holds: {}", report.holds);
    println!("cause: {}", or_none(report.cause));
    println!("row: {}", or_none(report.row));
    println!("broken_link: {}", or_none(report.broken_link));
    println!("chains: {}", report.chains.as_str());
    println!("merkle_root: {}", or_none(report.merkle_root.as_deref()));
    println!("dataset_seal: {}", or_none(report.dataset_seal.as_deref()));
    println!("receipt: {}", report.receipt.as_str());
    println!("rows_file: {}", report.rows_file.as_str());
    println!("frames_file: {}", report.frames_file.as_str());
}

fn verify_dataset(head: &str, rows: &str, frames: Option<&str>) -> Result<u8, u8> {
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
    print_dataset(&report);
    let bytes_differ = report.receipt != ReceiptCheck::Matches
        || report.rows_file == FileCheck::DoesNotMatch
        || report.frames_file == FileCheck::DoesNotMatch;
    Ok(if !report.holds {
        BROKEN
    } else if bytes_differ {
        BYTES_DIFFER
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
            print_dataset(&report);
            Err(BROKEN)
        }
        Err(refusal) => Err(refused(refusal)),
    }
}

fn check_proof(proof: &str) -> Result<u8, u8> {
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
