//! The verification that comes before any conversion.
//!
//! The verification is `genesis_seal::dataset::check_dataset_stream`: the head and
//! the rows file as bytes, the frame file as a reader, read once from its
//! start to its end. This module adds the one rule that is its own: nothing
//! is converted when the head names no hash for a file that was given.
//!
//! A head may carry the engine's ML-DSA-87 signature right after its receipt.
//! It is read with the signature split off (`genesis_seal::dataset::Head::read`).
//! Given a public key, the signature is checked by
//! `genesis_seal::answer_signature::check` on the head's text, and a head
//! whose signature does not hold is not converted (exit 5, as `genesis_seal
//! verify-dataset` gives, said before 3); its cause is printed by name.

use std::fs::File;
use std::io::{self, BufReader, Read, Take};
use std::path::{Path, PathBuf};

use genesis_seal::answer_signature::{self, Check, PublicKey, SignatureCause, SignatureReport};
use genesis_seal::dataset::{DatasetRefusal, DatasetReport, FileCheck, Head, check_dataset_stream};
use genesis_seal::fault::{Fault, bytes_differ, fault_of, report_lines};
use genesis_seal::sealed_form::digest_hex;

use crate::{BROKEN, BYTES_DIFFER, REFUSED, SIGNATURE_DOES_NOT_HOLD};

/// A dataset that holds, with every file the head names matching.
#[derive(Debug)]
pub struct Verified {
    pub head: Head,
    pub report: DatasetReport,
    /// SHA-384 of the head file as given, its signature included.
    pub head_sha384: String,
    pub rows_sha384: String,
    pub frames_sha384: Option<String>,
    /// What the check of the head's signature found.
    pub signature: Signature,
}

/// The key and the clock a head's signature is checked with.
#[derive(Debug, Clone)]
pub struct Signing {
    /// `None`: no key was given; a signature the head carries is not checked.
    pub key: Option<PublicKey>,
    /// The verifier's clock, in seconds since 1970-01-01T00:00:00Z.
    pub now_s: i64,
}

impl Signing {
    /// No key: a signature is split off and not checked.
    pub fn none() -> Signing {
        Signing { key: None, now_s: answer_signature::now_s() }
    }
}

/// The check of a head's signature, and whether a key was given for it.
#[derive(Debug, Clone)]
pub struct Signature {
    pub report: SignatureReport,
    pub key_given: bool,
}

impl Signature {
    /// True when a key was given and the signature does not hold (or is absent).
    pub fn fails(&self) -> bool {
        self.key_given && !self.report.holds()
    }

    /// The word for the signature: `absent`, `not checked`, `holds` or `does not hold`.
    pub fn said(&self) -> &'static str {
        match (self.report.present, self.report.cause) {
            (false, _) => "absent",
            (true, Some(SignatureCause::KeyNotGiven)) => "not checked",
            (true, None) => "holds",
            (true, Some(_)) => "does not hold",
        }
    }

    /// The lines `genesis_seal verify-dataset` prints for the head's
    /// signature, word for word: printed when the head carries one or a key
    /// was given, and nothing otherwise.
    pub fn lines(&self) -> String {
        let report = &self.report;
        if !report.present && !self.key_given {
            return String::new();
        }
        let mut out = format!("signature: {}\n", self.said());
        match report.cause {
            Some(SignatureCause::KeyNotGiven) => {
                out.push_str("signature_note: no --public-key was given, so the signature was not checked.\n");
            }
            Some(cause) => {
                out.push_str(&format!("signature_cause: {}\nsignature_fault: {}\n", cause.name(), cause.sentence()));
            }
            None => {}
        }
        if let Some(envelope) = &report.envelope {
            out.push_str(&format!(
                "key_id: {}\nissued_at: {}\nexpires_at: {}\nnonce: {}\n",
                envelope.key_id, envelope.issued_at, envelope.expires_at, envelope.nonce
            ));
        }
        out
    }
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
        fault: Option<Fault>,
        signature: Box<Signature>,
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
            Failure::Report { report, fault, signature, also, .. } => {
                print_report(report, fault.as_ref());
                print!("{}", signature.lines());
                if let Some(also) = also {
                    eprintln!("{also}");
                }
                eprintln!("nothing was written.");
            }
        }
    }
}

impl Verified {
    pub fn print(&self) {
        print_report(&self.report, None);
        print!("{}", self.signature.lines());
    }
}

/// The lines of `genesis_seal verify-dataset`, word for word.
pub fn print_report(report: &DatasetReport, fault: Option<&Fault>) {
    print!("{}", report_lines(report, fault));
}

/// The most rows of a plan one search for a row's place writes out, as
/// `genesis_seal verify-dataset` searches.
const SEARCH_MOST_ROWS: u64 = 100_000;

fn refused(refusal: impl std::fmt::Display) -> Failure {
    Failure::Refused(refusal.to_string())
}

/// The frame file, opened when the verification first reads it, so a head or a plan
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

/// Verify a dataset as `genesis_seal verify-dataset` does with no key, and
/// decide whether it may be converted. A signature the head carries is split
/// off and not checked.
pub fn verify(
    head_bytes: &[u8],
    rows_bytes: &[u8],
    frames: Option<&Path>,
) -> Result<Verified, Failure> {
    verify_signed(head_bytes, rows_bytes, frames, &Signing::none())
}

/// Verify a dataset as `genesis_seal verify-dataset` does, the head's
/// signature checked with the key and the clock given, and decide whether it
/// may be converted.
pub fn verify_signed(
    head_bytes: &[u8],
    rows_bytes: &[u8],
    frames: Option<&Path>,
    signing: &Signing,
) -> Result<Verified, Failure> {
    let input = frames.map(|path| FrameFile {
        path: path.to_path_buf(),
        open: None,
    });
    let streamed = check_dataset_stream(head_bytes, rows_bytes, input).map_err(refused)?;
    let report = streamed.report;
    let frames_sha384 = streamed.frames_sha384;
    let head = Head::read(head_bytes).map_err(|r| refused(DatasetRefusal::Head(r)))?;
    let rows_sha384 = streamed.rows_sha384;
    // The head was read, so its bytes are text.
    let head_text = std::str::from_utf8(head_bytes).map_err(|_| refused("the head is not UTF-8 text."))?;
    let signature = Signature {
        report: answer_signature::check(
            head_text,
            Check { public_key: signing.key.as_ref(), now_s: signing.now_s, seen_nonces: &[] },
        ),
        key_given: signing.key.is_some(),
    };
    // A dropped or repeated row, and a decimal that is not its hex, are named
    // as verify-dataset names them.
    let fault = fault_of(head_bytes, rows_bytes, &report, SEARCH_MOST_ROWS);
    if !report.holds {
        return Err(Failure::Report {
            report: Box::new(report),
            fault,
            signature: Box::new(signature),
            code: BROKEN,
            also: None,
        });
    }
    // Every seal holds. A key was given and the head's signature does not
    // hold: nothing is converted, the cause named.
    if signature.fails() {
        return Err(Failure::Report {
            report: Box::new(report),
            fault,
            signature: Box::new(signature),
            code: SIGNATURE_DOES_NOT_HOLD,
            also: Some("The engine's signature on the head does not hold, so it is not converted.".into()),
        });
    }
    // The files must be the ones the head names, byte for
    // byte, and every decimal the value of its hex. Unlike `verify-dataset`,
    // a file the head names no hash for is not converted either: its identity
    // is not shown.
    let unnamed = report.rows_file == FileCheck::NotNamed
        || (frames.is_some() && report.frames_file == FileCheck::NotNamed);
    let differ = bytes_differ(&report, fault.as_ref());
    if differ || unnamed {
        let also = if differ {
            None
        } else {
            Some("The head names no hash for a file that was given, so it is not converted.".into())
        };
        return Err(Failure::Report {
            report: Box::new(report),
            fault,
            signature: Box::new(signature),
            code: BYTES_DIFFER,
            also,
        });
    }
    Ok(Verified {
        head_sha384: digest_hex(head_bytes),
        head,
        report,
        rows_sha384,
        frames_sha384,
        signature,
    })
}
