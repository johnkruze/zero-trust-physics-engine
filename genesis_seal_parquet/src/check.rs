//! The check that comes before any conversion.
//!
//! The check is `genesis_seal::dataset::check_dataset_stream`: the head and
//! the rows file as bytes, the frame file as a reader, read once from its
//! start to its end. This module adds the one rule that is its own: nothing
//! is converted when the head names no hash for a file that was given.

use std::fs::File;
use std::io::{self, BufReader, Read, Take};
use std::path::{Path, PathBuf};

use genesis_seal::dataset::{DatasetRefusal, DatasetReport, FileCheck, Head, check_dataset_stream};
use genesis_seal::receipt::ReceiptCheck;
use genesis_seal::sealed_form::digest_hex;

use crate::{BROKEN, BYTES_DIFFER, REFUSED};

/// A dataset that holds, with every file the head names matching.
#[derive(Debug)]
pub struct Checked {
    pub head: Head,
    pub report: DatasetReport,
    pub head_sha256: String,
    pub rows_sha256: String,
    pub frames_sha256: Option<String>,
}

/// Why nothing is converted.
#[derive(Debug)]
pub enum Failure {
    /// The input was refused or could not be read. Exit code 2.
    Refused(String),
    /// A seal is broken (code 1), or every seal holds and a file is not the
    /// one the head names (code 3). The report is printed as the sealing
    /// program prints it.
    Report {
        report: Box<DatasetReport>,
        code: u8,
        also: Option<String>,
    },
}

impl Failure {
    pub fn code(&self) -> u8 {
        match self {
            Failure::Refused(_) => REFUSED,
            Failure::Report { code, .. } => *code,
        }
    }

    pub fn print(&self) {
        match self {
            Failure::Refused(reason) => eprintln!("refused: {reason}"),
            Failure::Report { report, also, .. } => {
                print_report(report);
                if let Some(also) = also {
                    eprintln!("{also}");
                }
                eprintln!("nothing was written.");
            }
        }
    }
}

impl Checked {
    pub fn print(&self) {
        print_report(&self.report);
    }
}

fn or_none<T: std::fmt::Display>(value: Option<T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "none".to_string(),
    }
}

/// The lines of `genesis_seal verify-dataset`, word for word.
pub fn print_report(report: &DatasetReport) {
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

fn refused(refusal: impl std::fmt::Display) -> Failure {
    Failure::Refused(refusal.to_string())
}

/// The frame file, opened when the check first reads it, so a head or a plan
/// that is refused is refused before the file is looked at.
struct FrameFile {
    path: PathBuf,
    open: Option<BufReader<Take<File>>>,
}

impl Read for FrameFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let path = &self.path;
        let named = |error: io::Error| io::Error::other(format!("{}: {error}", path.display()));
        if self.open.is_none() {
            // A regular file, no link, no wait, and no more bytes than its size at open.
            let file = genesis_seal::stored_file::open_regular(path).map_err(io::Error::other)?;
            self.open = Some(BufReader::with_capacity(1 << 20, file.reader()));
        }
        self.open.as_mut().expect("opened above").read(buffer).map_err(named)
    }
}

/// Check a dataset as `genesis_seal verify-dataset` does, and decide whether
/// it may be converted.
pub fn check(
    head_bytes: &[u8],
    rows_bytes: &[u8],
    frames: Option<&Path>,
) -> Result<Checked, Failure> {
    let input = frames.map(|path| FrameFile {
        path: path.to_path_buf(),
        open: None,
    });
    let streamed = check_dataset_stream(head_bytes, rows_bytes, input).map_err(refused)?;
    let report = streamed.report;
    let frames_sha256 = streamed.frames_sha256;
    let head = Head::read(head_bytes).map_err(|r| refused(DatasetRefusal::Head(r)))?;

    let rows_sha256 = streamed.rows_sha256;
    if !report.holds {
        return Err(Failure::Report {
            report: Box::new(report),
            code: BROKEN,
            also: None,
        });
    }
    // Every seal holds. The files must be the ones the head names, byte for
    // byte. Unlike `verify-dataset`, a file the head names no hash for is not
    // converted either: its identity is not shown.
    let unnamed = report.rows_file == FileCheck::NotNamed
        || (frames.is_some() && report.frames_file == FileCheck::NotNamed);
    let differ = report.receipt != ReceiptCheck::Matches
        || report.rows_file == FileCheck::DoesNotMatch
        || report.frames_file == FileCheck::DoesNotMatch;
    if differ || unnamed {
        let also = if differ {
            None
        } else {
            Some("The head names no hash for a file that was given, so it is not converted.".into())
        };
        return Err(Failure::Report {
            report: Box::new(report),
            code: BYTES_DIFFER,
            also,
        });
    }
    Ok(Checked {
        head_sha256: digest_hex(head_bytes),
        head,
        report,
        rows_sha256,
        frames_sha256,
    })
}
