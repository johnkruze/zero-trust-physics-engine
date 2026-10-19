//! Converts a sealed dataset to Parquet, after its seal has been verified.
//!
//! The sealing rules are the sealing crate's. This crate implements none.
//! It calls `genesis_seal::dataset::check_dataset_stream`, which reads the
//! frame file as a stream so that a large frame file is never held whole. Only
//! a dataset that holds is
//! converted. A head that carries the engine's signature is read with it
//! split off; given `--public-key`, the signature is checked by
//! `genesis_seal::answer_signature::check`, and a head whose signature does
//! not hold is not converted.

pub mod verify;
pub mod convert;
pub mod state;

use std::path::Path;

pub use verify::{Failure, Signing, Verified, verify, verify_signed};

use genesis_seal::answer_signature::{self, PublicKey};

/// The exit codes, the same as the `genesis_seal` program gives.
pub const HOLDS: u8 = 0;
pub const BROKEN: u8 = 1;
pub const REFUSED: u8 = 2;
pub const BYTES_DIFFER: u8 = 3;
/// Every seal holds and the engine's signature on the head, checked against
/// the key given, does not.
pub const SIGNATURE_DOES_NOT_HOLD: u8 = 5;

pub const USAGE: &str = "usage: genesis_seal_parquet <head file> <rows file> [frame file] <output directory> [--public-key <key file>] [--now <YYYY-MM-DDTHH:MM:SSZ>]";

/// Take `--<name> <value>` out of the arguments, at most once.
fn take_option<'a>(arguments: &mut Vec<&'a str>, name: &str) -> Result<Option<&'a str>, u8> {
    let flag = format!("--{name}");
    let count = arguments.iter().filter(|argument| **argument == flag).count();
    match arguments.iter().position(|argument| *argument == flag) {
        None => Ok(None),
        Some(at) if count == 1 && at + 1 < arguments.len() => {
            let value = arguments[at + 1];
            arguments.drain(at..at + 2);
            Ok(Some(value))
        }
        Some(_) => {
            eprintln!("{USAGE}");
            Err(REFUSED)
        }
    }
}

/// The key and the clock the head's signature is checked with, read from
/// `--public-key <file>` (the base64 of the engine's 2,592-byte ML-DSA-87
/// key, as `genesis_seal` reads it) and `--now <time>` (this machine's clock
/// when not given).
fn read_signature_options(arguments: &mut Vec<&str>) -> Result<Signing, u8> {
    let key = match take_option(arguments, "public-key")? {
        None => None,
        Some(path) => {
            let bytes = genesis_seal::stored_file::open_regular(Path::new(path))
                .map_err(|refusal| refusal.to_string())
                .and_then(|file| file.read_all().map_err(|error| format!("'{path}' could not be read: {error}")));
            let bytes = match bytes {
                Ok(bytes) => bytes,
                Err(reason) => {
                    eprintln!("refused: {reason}");
                    return Err(REFUSED);
                }
            };
            match std::str::from_utf8(&bytes).ok().and_then(PublicKey::from_base64) {
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
        Some(text) => match answer_signature::parse_time(text) {
            Some(seconds) => seconds,
            None => {
                eprintln!("refused: '--now {text}' is not a time written YYYY-MM-DDTHH:MM:SSZ.");
                return Err(REFUSED);
            }
        },
    };
    Ok(Signing { key, now_s })
}

/// Verify the dataset, and convert it when it holds. The answer is the exit
/// code. Text for the reader goes to standard output and standard error.
pub fn run(arguments: &[&str]) -> u8 {
    let mut arguments = arguments.to_vec();
    let signing = match read_signature_options(&mut arguments) {
        Ok(signing) => signing,
        Err(code) => return code,
    };
    let (head, rows, frames, out) = match arguments.as_slice() {
        [head, rows, out] => (head, rows, None, out),
        [head, rows, frames, out] => (head, rows, Some(*frames), out),
        _ => {
            eprintln!("{USAGE}");
            return REFUSED;
        }
    };
    // An earlier run that was stopped may have left its temporary files here.
    convert::remove_stale_partials(Path::new(out));
    // Every file given must be a regular file reached without a link; a named
    // pipe or a device is refused by name, never waited for.
    let read = |path: &str| {
        genesis_seal::stored_file::open_regular(Path::new(path))
            .map_err(|refusal| refusal.to_string())?
            .read_all()
            .map_err(|error| format!("'{path}' could not be read: {error}"))
    };
    let (head_bytes, rows_bytes) = match (read(head), read(rows)) {
        (Ok(head), Ok(rows)) => (head, rows),
        (Err(reason), _) | (_, Err(reason)) => {
            eprintln!("refused: {reason}");
            return REFUSED;
        }
    };
    let verified = match verify_signed(&head_bytes, &rows_bytes, frames.map(Path::new), &signing) {
        Ok(verified) => verified,
        Err(failure) => {
            failure.print();
            return failure.code();
        }
    };
    verified.print();
    match convert::convert(&verified, &rows_bytes, frames.map(Path::new), Path::new(out)) {
        Ok(written) => {
            println!("converted_rows: {}", written.rows_file);
            println!("converted_row_count: {}", written.row_records);
            match (&written.frames_file, written.frame_records) {
                (Some(file), Some(count)) => {
                    println!("converted_frames: {file}");
                    println!("converted_frame_count: {count}");
                }
                _ => println!("converted_frames: none"),
            }
            HOLDS
        }
        Err(reason) => {
            eprintln!("refused: {reason}");
            eprintln!("nothing was written.");
            REFUSED
        }
    }
}
