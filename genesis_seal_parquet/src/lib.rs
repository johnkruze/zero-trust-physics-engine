//! Converts a sealed dataset to Parquet, after its seal has been verified.
//!
//! The sealing rules are the sealing crate's. This crate implements none.
//! It calls `genesis_seal::dataset::check_dataset_stream`, which reads the
//! frame file as a stream so that a large frame file is never held whole. Only
//! a dataset that holds is
//! converted.

pub mod verify;
pub mod convert;
pub mod state;

use std::path::Path;

pub use verify::{Verified, Failure, verify};

/// The exit codes, the same as the `genesis_seal` program gives.
pub const HOLDS: u8 = 0;
pub const BROKEN: u8 = 1;
pub const REFUSED: u8 = 2;
pub const BYTES_DIFFER: u8 = 3;

pub const USAGE: &str =
    "usage: genesis_seal_parquet <head file> <rows file> [frame file] <output directory>";

/// Verify the dataset, and convert it when it holds. The answer is the exit
/// code. Text for the reader goes to standard output and standard error.
pub fn run(arguments: &[&str]) -> u8 {
    let (head, rows, frames, out) = match arguments {
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
    let verified = match verify(&head_bytes, &rows_bytes, frames.map(Path::new)) {
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
