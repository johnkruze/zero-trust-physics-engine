//! A dataset: its seal, its manifest head, its check, and the proof of one row.
//!
//! The dataset seal is SHA-256 of three texts with nothing between them: the
//! plan hash as 64 hex characters, the root of the row tree as 64 hex
//! characters, and the compact JSON of the counts.
//!
//! A check names the first break in this order: `plan`, then for each row
//! `chain`, `state`, `run_seal`, `leaf`, then `root`, then `dataset_seal`.
//! No check here runs the engine. A dataset whose seals hold was not altered
//! after it was sealed. That the engine made it is shown only by sending a
//! row's request to the engine and comparing run seals.

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom, Write};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::frames::{FramesFile, FramesHeader, FramesRefusal, HEADER_LEN, write_frames};
use crate::plan::{DatasetRequest, PlanRefusal, Return, Store};
use crate::receipt::{ReceiptCheck, check_receipt, with_receipt};
use crate::refusal::{
    Refusal, optional_hash, require_exact_keys, require_hash, require_string, require_whole,
    required, wrong,
};
use crate::rows::{Outcome, Row, RunRefusal, leaf_text, lines, rows_text};
use crate::run::{FRAME_LEN, expected_state_digest, follow_frame, frames_recorded, run_seal};
use crate::sealed_form::{digest_hex, hash_from_hex, json_string, request_digest, sealed_form};
use crate::tree::{self, Hash};

const HEAD_KEYS: &[&str] = &[
    "request",
    "engine_version",
    "rows",
    "completed",
    "ended",
    "refused",
    "unadmitted",
    "frames",
    "plan_sha256",
    "merkle_root",
    "dataset_seal",
    "rows_sha256",
    "frames_sha256",
    "receipt_sha256",
];

const PROOF_KEYS: &[&str] = &[
    "row",
    "frames_hex",
    "audit_path",
    "plan_sha256",
    "merkle_root",
    "engine_version",
    "rows",
    "completed",
    "ended",
    "refused",
    "unadmitted",
    "frames",
    "dataset_seal",
];

/// The counts the dataset seal covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counts {
    pub engine_version: String,
    pub rows: u64,
    pub completed: u64,
    /// The runs that stopped at an end the card declares.
    pub ended: u64,
    pub refused: u64,
    pub unadmitted: u64,
    /// The frames the rows recorded, added together, whether or not they
    /// are stored.
    pub frames: u64,
}

impl Counts {
    /// The compact JSON the dataset seal ends with: `engine_version`, `rows`,
    /// `completed`, `ended`, `refused`, `unadmitted`, `frames`, in that order.
    pub fn text(&self) -> String {
        format!(
            "{{\"engine_version\":{},\"rows\":{},\"completed\":{},\"ended\":{},\"refused\":{},\"unadmitted\":{},\"frames\":{}}}",
            json_string(&self.engine_version),
            self.rows,
            self.completed,
            self.ended,
            self.refused,
            self.unadmitted,
            self.frames,
        )
    }

    fn read(object: &Map<String, Value>) -> Result<Counts, Refusal> {
        Ok(Counts {
            engine_version: require_string(object, "engine_version")?,
            rows: require_whole(object, "rows")?,
            completed: require_whole(object, "completed")?,
            ended: require_whole(object, "ended")?,
            refused: require_whole(object, "refused")?,
            unadmitted: require_whole(object, "unadmitted")?,
            frames: require_whole(object, "frames")?,
        })
    }

    fn write(&self, object: &mut Map<String, Value>) {
        object.insert("engine_version".into(), Value::from(self.engine_version.as_str()));
        object.insert("rows".into(), Value::from(self.rows));
        object.insert("completed".into(), Value::from(self.completed));
        object.insert("ended".into(), Value::from(self.ended));
        object.insert("refused".into(), Value::from(self.refused));
        object.insert("unadmitted".into(), Value::from(self.unadmitted));
        object.insert("frames".into(), Value::from(self.frames));
    }
}

/// SHA-256 of `plan_sha256`, `merkle_root` and the counts text, as hex.
pub fn dataset_seal(plan_sha256: &str, merkle_root: &str, counts: &Counts) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(plan_sha256.as_bytes());
    bytes.extend_from_slice(merkle_root.as_bytes());
    bytes.extend_from_slice(counts.text().as_bytes());
    digest_hex(&bytes)
}

/// The manifest head of a dataset.
///
/// Beside the request and the engine's name it holds five counts, three
/// hashes, the two file hashes and the receipt. It does not grow with what
/// the runs returned.
#[derive(Debug, Clone, PartialEq)]
pub struct Head {
    /// The whole `run_dataset` request, in the order received.
    pub request: Value,
    pub counts: Counts,
    pub plan_sha256: String,
    pub merkle_root: String,
    pub dataset_seal: String,
    /// SHA-256 of the rows file, or null when the dataset came back inline.
    pub rows_sha256: Option<String>,
    /// SHA-256 of the frame file, or null when there is none.
    pub frames_sha256: Option<String>,
}

impl Head {
    /// The JSON object of every field but the receipt.
    pub fn document(&self) -> String {
        let hash = |text: &Option<String>| match text {
            Some(text) => Value::from(text.as_str()),
            None => Value::Null,
        };
        let mut object = Map::new();
        object.insert("request".into(), self.request.clone());
        self.counts.write(&mut object);
        object.insert("plan_sha256".into(), Value::from(self.plan_sha256.as_str()));
        object.insert("merkle_root".into(), Value::from(self.merkle_root.as_str()));
        object.insert("dataset_seal".into(), Value::from(self.dataset_seal.as_str()));
        object.insert("rows_sha256".into(), hash(&self.rows_sha256));
        object.insert("frames_sha256".into(), hash(&self.frames_sha256));
        serde_json::to_string(&Value::Object(object)).expect("a head encodes as JSON")
    }

    /// The head as it is sent and stored: the document with its receipt.
    pub fn wire(&self) -> String {
        with_receipt(&self.document())
    }

    /// True when `other` is the head of this same dataset: every field but
    /// `request` is equal, and the two requests have the same sealed form, so
    /// they differ at most in the order of their keys and the spelling of
    /// their numbers. The receipt is not compared: it covers the request as
    /// it was written, so it follows the request's spelling.
    pub fn is_same_dataset_as(&self, other: &Head) -> bool {
        self.counts == other.counts
            && self.plan_sha256 == other.plan_sha256
            && self.merkle_root == other.merkle_root
            && self.dataset_seal == other.dataset_seal
            && self.rows_sha256 == other.rows_sha256
            && self.frames_sha256 == other.frames_sha256
            && sealed_form(&self.request) == sealed_form(&other.request)
    }

    /// Scan the text of a head, then read it.
    pub fn read(bytes: &[u8]) -> Result<Head, Refusal> {
        let value = crate::json_text::read(bytes)?;
        let object = value.as_object().ok_or(Refusal::NotAnObject)?;
        require_exact_keys(object, HEAD_KEYS)?;
        let request = required(object, "request")?;
        if !request.is_object() {
            return Err(wrong("request", "a JSON object"));
        }
        require_hash(object, "receipt_sha256")?;
        Ok(Head {
            request: request.clone(),
            counts: Counts::read(object)?,
            plan_sha256: require_hash(object, "plan_sha256")?,
            merkle_root: require_hash(object, "merkle_root")?,
            dataset_seal: require_hash(object, "dataset_seal")?,
            rows_sha256: optional_hash(object, "rows_sha256")?,
            frames_sha256: optional_hash(object, "frames_sha256")?,
        })
    }
}

/// The part of a dataset that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// The plan hash is not the head's, the rows are not as many as the plan
    /// has, or a row's request is not the plan's row.
    Plan,
    /// A row's frames do not lead from its request to its last link.
    Chain,
    /// A signed reading of a row's request does not verify, or is not well
    /// formed (`crate::attest`).
    LoadSignature,
    /// A row's state digest is not the one its final state gives.
    State,
    /// A row's run seal is not the one its last link and outcome give.
    RunSeal,
    /// A row's stored leaf is not the leaf the row gives: its number, its
    /// request, its outcome, its run seal and its refusal sentence.
    Leaf,
    /// The root over the leaves is not the stored root.
    Root,
    /// The dataset seal is not the one the plan, the root and the counts give.
    DatasetSeal,
}

impl Cause {
    pub fn as_str(self) -> &'static str {
        match self {
            Cause::Plan => "plan",
            Cause::Chain => "chain",
            Cause::LoadSignature => "load_signature",
            Cause::State => "state",
            Cause::RunSeal => "run_seal",
            Cause::Leaf => "leaf",
            Cause::Root => "root",
            Cause::DatasetSeal => "dataset_seal",
        }
    }
}

impl fmt::Display for Cause {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(self.as_str())
    }
}

/// How far back each row was checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chains {
    /// Each row's chain was rebuilt from its frames, back to its request.
    FromFrames,
    /// The frames were not given. Each row was checked from its last link on.
    FramesNotGiven,
    /// The plan stores no frames. Each row was checked from its last link on.
    FramesNotStored,
}

impl Chains {
    pub fn as_str(self) -> &'static str {
        match self {
            Chains::FromFrames => "rebuilt from the frames",
            Chains::FramesNotGiven => "not rebuilt: the frames were not given",
            Chains::FramesNotStored => "not rebuilt: the plan stores no frames",
        }
    }
}

/// Whether a file is the one the head names. The dataset seal does not cover
/// this: it covers what the file seals, not its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCheck {
    /// The file hashes to the hash in the head.
    Matches,
    /// The file does not hash to the hash in the head.
    DoesNotMatch,
    /// The head holds no hash for this file.
    NotNamed,
    /// The file was not given.
    NotGiven,
}

impl FileCheck {
    pub fn as_str(self) -> &'static str {
        match self {
            FileCheck::Matches => "matches the hash in the head",
            FileCheck::DoesNotMatch => "does not match the hash in the head",
            FileCheck::NotNamed => "the head holds no hash for it",
            FileCheck::NotGiven => "not given",
        }
    }
}

/// The answer to the check of a dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetReport {
    pub holds: bool,
    pub cause: Option<Cause>,
    /// The row of the first break, when the break is in one row.
    pub row: Option<u64>,
    /// On a chain break, the index within its row of the first frame that
    /// does not carry the link before it. The row's frame count when every
    /// frame carries its link and the chain does not end at the row's last
    /// link. The frame file stores no link beside each frame, so a change to
    /// the body of frame k shows at frame k + 1.
    pub broken_link: Option<u64>,
    pub chains: Chains,
    /// The root rebuilt from the rows, when every row passed.
    pub merkle_root: Option<String>,
    /// The seal rebuilt from the plan, the root and the counts, when every
    /// row passed.
    pub dataset_seal: Option<String>,
    pub receipt: ReceiptCheck,
    pub rows_file: FileCheck,
    pub frames_file: FileCheck,
}

/// Why a dataset or a proof was not checked, or not made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetRefusal {
    /// The manifest head was not read.
    Head(Refusal),
    /// The request in the head is not a plan.
    Plan(PlanRefusal),
    /// A row was not read.
    Row { row: u64, refusal: Refusal },
    /// The rows file is empty, does not end with a line break, or holds an empty line.
    RowsNotLines,
    /// The frame file was not read.
    Frames(FramesRefusal),
    /// A frame file was given for a plan that stores no frames.
    FramesNotStored,
    /// The proof was not read.
    Proof(Refusal),
    /// The dataset does not hold, so no proof is made from it.
    DoesNotHold(DatasetReport),
    /// The dataset has no row of that number.
    NoSuchRow { row: u64, rows: u64 },
    /// The count of results is not the count of rows in the plan.
    ResultsNotRows { results: u64, rows: u64 },
    /// A result was not made into a row.
    Run { row: u64, refusal: RunRefusal },
    /// A result is the run of another request than the plan's row.
    NotThePlansRow(u64),
    /// A writer was given a row other than the next row of the plan.
    RowOutOfOrder { given: u64, expected: u64 },
    /// A writer has a frame sink for a plan that stores none, or none for a plan that stores frames.
    SinkNotAsStored,
    /// A file could not be written or read back.
    NotWritten(String),
}

impl fmt::Display for DatasetRefusal {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DatasetRefusal::Head(refusal) => write!(out, "The head was not read. {refusal}"),
            DatasetRefusal::Plan(refusal) => refusal.fmt(out),
            DatasetRefusal::Row { row, refusal } => {
                write!(out, "Row {row} was not read. {refusal}")
            }
            DatasetRefusal::RowsNotLines => write!(
                out,
                "The rows file must hold one row on each line and end with a line break."
            ),
            DatasetRefusal::Frames(refusal) => refusal.fmt(out),
            DatasetRefusal::FramesNotStored => write!(
                out,
                "A frame file was given, and the plan stores no frames."
            ),
            DatasetRefusal::Proof(refusal) => write!(out, "The proof was not read. {refusal}"),
            DatasetRefusal::DoesNotHold(report) => match (report.cause, report.row) {
                (Some(cause), Some(row)) => {
                    write!(out, "The dataset does not hold: {cause} at row {row}.")
                }
                (Some(cause), None) => write!(out, "The dataset does not hold: {cause}."),
                _ => write!(out, "The dataset does not hold."),
            },
            DatasetRefusal::NoSuchRow { row, rows } => {
                write!(out, "The dataset has {rows} rows and no row {row}.")
            }
            DatasetRefusal::ResultsNotRows { results, rows } => write!(
                out,
                "The plan has {rows} rows and {results} results were given."
            ),
            DatasetRefusal::Run { row, refusal } => {
                write!(out, "The result of row {row} was not taken. {refusal}")
            }
            DatasetRefusal::NotThePlansRow(row) => write!(
                out,
                "The result of row {row} is the run of another request than the plan's row {row}."
            ),
            DatasetRefusal::RowOutOfOrder { given, expected } => write!(
                out,
                "Row {given} was given, and the next row of the plan is row {expected}."
            ),
            DatasetRefusal::SinkNotAsStored => write!(
                out,
                "A frame sink goes with a plan that stores frames, and only with that plan."
            ),
            DatasetRefusal::NotWritten(reason) => {
                write!(out, "A dataset file could not be written: {reason}")
            }
        }
    }
}

/// Check one row. The frames, when given, come one at a time from the
/// iterator, which holds the row's own frames next. The answer is the row's
/// leaf hash, or the first break and its frame index.
fn check_row(
    row: &Row,
    index: u64,
    frames: Option<&mut dyn Iterator<Item = [u8; FRAME_LEN]>>,
) -> Result<Hash, (Cause, Option<u64>)> {
    let link_0 = request_digest(&row.request);
    if row.outcome != Outcome::Unadmitted {
        let (Some(steps_landed), Some(last_link), Some(stored_seal)) = (
            row.steps_landed,
            row.last_link.as_deref().and_then(hash_from_hex),
            row.run_seal.as_deref(),
        ) else {
            return Err((Cause::Chain, None));
        };
        match frames {
            Some(frames) => {
                let mut link = link_0;
                for position in 0..row.frames {
                    let next = frames
                        .next()
                        .and_then(|frame| follow_frame(&link, &frame));
                    match next {
                        Some(next) => link = next,
                        None => return Err((Cause::Chain, Some(position))),
                    }
                }
                if link != last_link {
                    return Err((Cause::Chain, Some(row.frames)));
                }
            }
            None => {
                if row.frames == 0 && last_link != link_0 {
                    return Err((Cause::Chain, Some(0)));
                }
            }
        }
        // A row's request is an ordinary run request: its signed readings
        // are verified as a run's are, after its chain.
        if crate::attest::verify_run(&row.request).is_err() {
            return Err((Cause::LoadSignature, None));
        }
        let Some(expected) = expected_state_digest(
            steps_landed,
            row.final_state.as_ref(),
            row.final_residual_conservation_error_hex.as_deref(),
            row.max_residual_conservation_error_hex.as_deref(),
        ) else {
            return Err((Cause::State, None));
        };
        if expected != row.state_digest {
            return Err((Cause::State, None));
        }
        let rebuilt = run_seal(
            &last_link,
            row.outcome.as_str(),
            steps_landed,
            row.reject.as_deref(),
            expected.as_deref(),
        );
        if rebuilt != stored_seal {
            return Err((Cause::RunSeal, None));
        }
        // A run records a frame at each multiple of `record_every` and at
        // its last landed step. The run seal has shown `steps_landed` is the
        // sealed count, so a row that states another count of frames states
        // a chain that run does not have.
        let recorded = row
            .request
            .get("record_every")
            .and_then(Value::as_u64)
            .and_then(|every| frames_recorded(steps_landed, every));
        if recorded != Some(row.frames) {
            return Err((Cause::Chain, None));
        }
    }
    let leaf = leaf_text(
        index,
        &hex::encode(link_0),
        row.outcome.as_str(),
        row.run_seal.as_deref(),
        row.reject_detail.as_deref(),
    );
    let stored = serde_json::to_string(&row.leaf).unwrap_or_default();
    if row.row != index || stored != leaf {
        return Err((Cause::Leaf, None));
    }
    Ok(tree::leaf_hash(leaf.as_bytes()))
}

/// What a check of a dataset found, kept for making a proof.
struct Examined {
    report: DatasetReport,
    head: Head,
    leaves: Vec<Hash>,
    /// The index in the frame file of each row's first frame.
    first_frames: Vec<u64>,
}

fn named_check(sha256: &str, named: &Option<String>) -> FileCheck {
    match named {
        None => FileCheck::NotNamed,
        Some(named) if named == sha256 => FileCheck::Matches,
        Some(_) => FileCheck::DoesNotMatch,
    }
}

/// The frame file read as a stream, one frame at a time. It hashes every
/// byte it reads, counts the bytes after the header, and ends at the stated
/// count of frames, at the end of the file, or at a read error, whichever
/// comes first. A read error is kept, not lost.
struct FrameStream<R> {
    input: R,
    hasher: Sha256,
    /// Frames the header states that have not been given out.
    left: u64,
    /// Bytes read after the header, whole frames or not.
    after_header: u64,
    ended: bool,
    error: Option<io::Error>,
}

impl<R: Read> FrameStream<R> {
    /// Read up to `buffer.len()` bytes, until the buffer is full or the
    /// input ends. Gives the count read. A short read is not an end.
    fn fill(input: &mut R, buffer: &mut [u8]) -> io::Result<usize> {
        let mut have = 0;
        while have < buffer.len() {
            match input.read(&mut buffer[have..]) {
                Ok(0) => break,
                Ok(count) => have += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(have)
    }

    fn skip(&mut self, count: u64) {
        for _ in 0..count {
            if self.next().is_none() {
                break;
            }
        }
    }

    /// Read and hash everything that is left, so the count of bytes and the
    /// hash are those of the whole file.
    fn drain(&mut self) {
        let mut block = [0_u8; 1 << 14];
        while self.error.is_none() {
            match Self::fill(&mut self.input, &mut block) {
                Ok(0) => break,
                Ok(count) => {
                    self.hasher.update(&block[..count]);
                    self.after_header = self.after_header.saturating_add(count as u64);
                }
                Err(error) => self.error = Some(error),
            }
        }
        self.ended = true;
    }
}

impl<R: Read> Iterator for FrameStream<R> {
    type Item = [u8; FRAME_LEN];

    fn next(&mut self) -> Option<[u8; FRAME_LEN]> {
        if self.ended || self.left == 0 {
            return None;
        }
        let mut frame = [0_u8; FRAME_LEN];
        match Self::fill(&mut self.input, &mut frame) {
            Ok(count) => {
                self.hasher.update(&frame[..count]);
                self.after_header = self.after_header.saturating_add(count as u64);
                if count < FRAME_LEN {
                    self.ended = true;
                    return None;
                }
                self.left -= 1;
                Some(frame)
            }
            Err(error) => {
                self.error = Some(error);
                self.ended = true;
                None
            }
        }
    }
}

fn frames_not_read(error: io::Error) -> DatasetRefusal {
    DatasetRefusal::Frames(FramesRefusal::NotRead(error.to_string()))
}

/// What `examine` gives back besides the report: the hash of the frame file
/// when one was read.
struct Streamed {
    examined: Examined,
    frames_sha256: Option<String>,
}

/// The check, with the frame file as a stream. Refusals come in the order
/// they come when the frame file is held whole: the head, the plan, a frame
/// file for a plan that stores none, the frame file's header, the length of
/// the frame file against its header, the rows file, then each row. The
/// length is known only at the end of the stream, so after the header the
/// whole stream is read before a refusal of the rows file or of a row is
/// given.
fn examine<F: Read>(
    head_bytes: &[u8],
    rows_bytes: &[u8],
    frames: Option<F>,
) -> Result<Streamed, DatasetRefusal> {
    let head = Head::read(head_bytes).map_err(DatasetRefusal::Head)?;
    let request =
        DatasetRequest::from_value(head.request.clone()).map_err(DatasetRefusal::Plan)?;
    if frames.is_some() && request.store() == Store::None {
        return Err(DatasetRefusal::FramesNotStored);
    }
    let mut stream = match frames {
        None => None,
        Some(mut input) => {
            let mut first = [0_u8; HEADER_LEN];
            let have = FrameStream::fill(&mut input, &mut first).map_err(frames_not_read)?;
            let header =
                FramesHeader::from_bytes(&first[..have]).map_err(DatasetRefusal::Frames)?;
            let mut hasher = Sha256::new();
            hasher.update(first);
            Some((
                header,
                FrameStream {
                    input,
                    hasher,
                    left: header.frames,
                    after_header: 0,
                    ended: false,
                    error: None,
                },
            ))
        }
    };
    let header = stream.as_ref().map(|(header, _)| *header);
    let outcome = examine_rows(
        head_bytes,
        rows_bytes,
        &head,
        &request,
        header.as_ref(),
        stream.as_mut().map(|(_, stream)| stream),
    );
    let Some((header, mut stream)) = stream else {
        return outcome.map(|examined| Streamed {
            examined,
            frames_sha256: None,
        });
    };
    stream.drain();
    if let Some(error) = stream.error.take() {
        return Err(frames_not_read(error));
    }
    if header.frames.checked_mul(FRAME_LEN as u64) != Some(stream.after_header) {
        return Err(DatasetRefusal::Frames(FramesRefusal::WrongLength {
            stated_frames: header.frames,
            bytes_after_header: stream.after_header,
        }));
    }
    let mut examined = outcome?;
    let sha256 = hex::encode(stream.hasher.finalize());
    examined.report.frames_file = named_check(&sha256, &head.frames_sha256);
    Ok(Streamed {
        examined,
        frames_sha256: Some(sha256),
    })
}

fn examine_rows<R: Read>(
    head_bytes: &[u8],
    rows_bytes: &[u8],
    head: &Head,
    request: &DatasetRequest,
    header: Option<&FramesHeader>,
    mut frames: Option<&mut FrameStream<R>>,
) -> Result<Examined, DatasetRefusal> {
    let chains = match (header, request.store()) {
        (Some(_), _) => Chains::FromFrames,
        (None, Store::Frames) => Chains::FramesNotGiven,
        (None, Store::None) => Chains::FramesNotStored,
    };
    let lines = lines(rows_bytes).ok_or(DatasetRefusal::RowsNotLines)?;
    let mut report = DatasetReport {
        holds: false,
        cause: None,
        row: None,
        broken_link: None,
        chains,
        merkle_root: None,
        dataset_seal: None,
        receipt: check_receipt(head_bytes),
        rows_file: named_check(&digest_hex(rows_bytes), &head.rows_sha256),
        // Set from the stream's hash once the whole stream is read.
        frames_file: if header.is_some() {
            FileCheck::NotNamed
        } else {
            FileCheck::NotGiven
        },
    };
    let examined = |report: DatasetReport, leaves: Vec<Hash>, first_frames: Vec<u64>| Examined {
        report,
        head: head.clone(),
        leaves,
        first_frames,
    };
    let plan_sha256 = request.plan_sha256();
    let rows = lines.len() as u64;
    if plan_sha256 != head.plan_sha256 || rows != request.totals().rows {
        report.cause = Some(Cause::Plan);
        return Ok(examined(report, Vec::new(), Vec::new()));
    }

    let mut first_break: Option<(Cause, u64, Option<u64>)> = None;
    let mut leaves = Vec::with_capacity(lines.len());
    let mut first_frames = Vec::with_capacity(lines.len());
    let mut next_frame = 0_u64;
    let mut counts = Counts {
        engine_version: head.counts.engine_version.clone(),
        rows,
        completed: 0,
        ended: 0,
        refused: 0,
        unadmitted: 0,
        frames: 0,
    };
    for (index, line) in lines.iter().enumerate() {
        let index = index as u64;
        let row = Row::read_line(line).map_err(|refusal| DatasetRefusal::Row {
            row: index,
            refusal,
        })?;
        let planned = request.row(index).map_err(DatasetRefusal::Plan)?;
        if sealed_form(&planned) != sealed_form(&row.request) {
            report.cause = Some(Cause::Plan);
            report.row = Some(index);
            return Ok(examined(report, Vec::new(), Vec::new()));
        }
        if first_break.is_some() {
            // A later row can still fail the plan, which is named first.
            continue;
        }
        let own_frames = frames
            .as_deref_mut()
            .map(|stream| stream as &mut dyn Iterator<Item = [u8; FRAME_LEN]>);
        match check_row(&row, index, own_frames) {
            Ok(leaf) => {
                // A row that was not admitted is not followed, and its
                // frames are passed over.
                if let Some(stream) = frames.as_deref_mut()
                    && row.outcome == Outcome::Unadmitted
                {
                    stream.skip(row.frames);
                }
                leaves.push(leaf);
                first_frames.push(next_frame);
                next_frame = next_frame.saturating_add(row.frames);
                counts.frames = next_frame;
                match row.outcome {
                    Outcome::Completed => counts.completed += 1,
                    Outcome::Ended => counts.ended += 1,
                    Outcome::Refused => counts.refused += 1,
                    Outcome::Unadmitted => counts.unadmitted += 1,
                }
            }
            Err((cause, broken_link)) => first_break = Some((cause, index, broken_link)),
        }
    }
    if let Some((cause, row, broken_link)) = first_break {
        report.cause = Some(cause);
        report.row = Some(row);
        report.broken_link = broken_link;
        return Ok(examined(report, Vec::new(), Vec::new()));
    }

    let root = hex::encode(tree::root(&leaves).expect("a plan has at least one row"));
    report.merkle_root = Some(root.clone());
    if root != head.merkle_root {
        report.cause = Some(Cause::Root);
        return Ok(examined(report, leaves, first_frames));
    }
    let seal = dataset_seal(&plan_sha256, &root, &counts);
    report.dataset_seal = Some(seal.clone());
    let frame_file_agrees = header.is_none_or(|header| {
        header.rows == counts.rows
            && header.frames == counts.frames
            && hex::encode(header.dataset_seal) == seal
    });
    if counts != head.counts || seal != head.dataset_seal || !frame_file_agrees {
        report.cause = Some(Cause::DatasetSeal);
        return Ok(examined(report, leaves, first_frames));
    }
    report.holds = true;
    Ok(examined(report, leaves, first_frames))
}

/// Check a dataset without the engine: the manifest head, the rows file,
/// and the frame file when there is one.
///
/// The plan is expanded again and every row's request is compared with it.
/// Each row's chain is rebuilt from its frames, then its state digest, its
/// run seal and its leaf. The root and the dataset seal follow. The first
/// break is named.
pub fn check_dataset(
    head: &[u8],
    rows: &[u8],
    frames: Option<&[u8]>,
) -> Result<DatasetReport, DatasetRefusal> {
    check_dataset_stream(head, rows, frames).map(|checked| checked.report)
}

/// The answer of `check_dataset_stream`: the report, and the SHA-256 of the
/// two files as they were read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamChecked {
    pub report: DatasetReport,
    /// SHA-256 of the rows file, in hex.
    pub rows_sha256: String,
    /// SHA-256 of every byte read from the frame file, in hex, when one was given.
    pub frames_sha256: Option<String>,
}

/// `check_dataset` with the frame file as a reader. It gives the report
/// `check_dataset` gives for the same bytes, the same refusals in the same
/// words, and names the first break in the same order. It holds one row and
/// one frame of the frame file at a time, and reads the frame file once, from
/// its start, to its end. The rows file is held whole, as a slice: it is small
/// beside the frames, and the check needs its line count before it reads a row.
///
/// The reader is read in the size of a frame, so a file should be given
/// through a buffered reader. `check_dataset` is this function over the bytes
/// in memory.
pub fn check_dataset_stream<F: Read>(
    head: &[u8],
    rows: &[u8],
    frames: Option<F>,
) -> Result<StreamChecked, DatasetRefusal> {
    let streamed = examine(head, rows, frames)?;
    Ok(StreamChecked {
        report: streamed.examined.report,
        rows_sha256: digest_hex(rows),
        frames_sha256: streamed.frames_sha256,
    })
}

/// What shows that one row belongs to a dataset: the row, its frames when
/// they are stored, and the sibling hashes from its leaf to the root.
#[derive(Debug, Clone, PartialEq)]
pub struct RowProof {
    pub row: Row,
    pub frames: Option<Vec<[u8; FRAME_LEN]>>,
    /// The sibling hashes, lowest first, as hex.
    pub audit_path: Vec<String>,
    pub plan_sha256: String,
    pub merkle_root: String,
    pub counts: Counts,
    pub dataset_seal: String,
}

impl RowProof {
    /// The proof as one compact JSON object.
    pub fn to_text(&self) -> String {
        let mut object = Map::new();
        object.insert("row".into(), self.row.to_value());
        object.insert(
            "frames_hex".into(),
            match &self.frames {
                Some(frames) => Value::from(
                    frames
                        .iter()
                        .map(|frame| Value::from(hex::encode(frame)))
                        .collect::<Vec<Value>>(),
                ),
                None => Value::Null,
            },
        );
        object.insert(
            "audit_path".into(),
            Value::from(
                self.audit_path
                    .iter()
                    .map(|hash| Value::from(hash.as_str()))
                    .collect::<Vec<Value>>(),
            ),
        );
        object.insert("plan_sha256".into(), Value::from(self.plan_sha256.as_str()));
        object.insert("merkle_root".into(), Value::from(self.merkle_root.as_str()));
        self.counts.write(&mut object);
        object.insert("dataset_seal".into(), Value::from(self.dataset_seal.as_str()));
        serde_json::to_string(&Value::Object(object)).expect("a proof encodes as JSON")
    }

    /// Scan the text of a proof, then read it.
    pub fn read(bytes: &[u8]) -> Result<RowProof, Refusal> {
        let value = crate::json_text::read(bytes)?;
        let object = value.as_object().ok_or(Refusal::NotAnObject)?;
        require_exact_keys(object, PROOF_KEYS)?;
        let row = Row::read(required(object, "row")?)?;
        let frames = match required(object, "frames_hex")? {
            Value::Null => None,
            Value::Array(items) => {
                let mut frames = Vec::with_capacity(items.len());
                for item in items {
                    let bytes = item
                        .as_str()
                        .filter(|text| crate::json_text::is_lower_hex(text, 2 * FRAME_LEN))
                        .and_then(|text| hex::decode(text).ok())
                        .ok_or(Refusal::NotExactHex {
                            field: "frames_hex".to_string(),
                            digits: 2 * FRAME_LEN,
                        })?;
                    let mut frame = [0_u8; FRAME_LEN];
                    frame.copy_from_slice(&bytes);
                    frames.push(frame);
                }
                Some(frames)
            }
            _ => return Err(wrong("frames_hex", "a JSON array of strings or null")),
        };
        let path = required(object, "audit_path")?
            .as_array()
            .ok_or_else(|| wrong("audit_path", "a JSON array of strings"))?;
        let mut audit_path = Vec::with_capacity(path.len());
        for item in path {
            let text = item
                .as_str()
                .filter(|text| crate::json_text::is_lower_hex(text, 64))
                .ok_or(Refusal::NotExactHex {
                    field: "audit_path".to_string(),
                    digits: 64,
                })?;
            audit_path.push(text.to_string());
        }
        Ok(RowProof {
            row,
            frames,
            audit_path,
            plan_sha256: require_hash(object, "plan_sha256")?,
            merkle_root: require_hash(object, "merkle_root")?,
            counts: Counts::read(object)?,
            dataset_seal: require_hash(object, "dataset_seal")?,
        })
    }
}

/// Make the proof of row `row` from a dataset that holds. A dataset that
/// does not hold gives no proof.
pub fn row_proof(
    head: &[u8],
    rows: &[u8],
    frames: Option<&[u8]>,
    row: u64,
) -> Result<RowProof, DatasetRefusal> {
    let examined = examine(head, rows, frames)?.examined;
    if !examined.report.holds {
        return Err(DatasetRefusal::DoesNotHold(examined.report));
    }
    let count = examined.leaves.len() as u64;
    let no_such_row = || DatasetRefusal::NoSuchRow { row, rows: count };
    let path = tree::audit_path(&examined.leaves, row).ok_or_else(no_such_row)?;
    let line = lines(rows)
        .and_then(|lines| lines.get(row as usize).copied())
        .ok_or_else(no_such_row)?;
    let the_row = Row::read_line(line).map_err(|refusal| DatasetRefusal::Row { row, refusal })?;
    let own_frames = match frames {
        None => None,
        Some(bytes) => {
            let file = FramesFile::read(bytes).map_err(DatasetRefusal::Frames)?;
            let first = examined.first_frames[row as usize];
            let mut own = Vec::with_capacity(the_row.frames as usize);
            for offset in 0..the_row.frames {
                let mut frame = [0_u8; FRAME_LEN];
                if let Some(bytes) = file.frame(first + offset) {
                    frame.copy_from_slice(bytes);
                }
                own.push(frame);
            }
            Some(own)
        }
    };
    Ok(RowProof {
        row: the_row,
        frames: own_frames,
        audit_path: path.iter().map(hex::encode).collect(),
        plan_sha256: examined.head.plan_sha256,
        merkle_root: examined.head.merkle_root,
        counts: examined.head.counts,
        dataset_seal: examined.head.dataset_seal,
    })
}

/// The answer to the check of one row's proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofReport {
    pub holds: bool,
    /// `chain`, `state`, `run_seal`, `leaf`, `root` or `dataset_seal`.
    pub cause: Option<Cause>,
    pub row: u64,
    pub broken_link: Option<u64>,
    /// `true` when the proof carries the row's frames and the chain was
    /// rebuilt from them.
    pub chain_rebuilt: bool,
    /// The dataset seal the proof leads to. The reader compares it with the
    /// seal of the dataset the row is said to belong to.
    pub dataset_seal: String,
}

/// Check one row with its proof alone: the row's chain when the proof
/// carries its frames, its state digest, its run seal, its leaf, the path
/// from the leaf to the root, and the dataset seal over that root.
///
/// The proof does not carry the plan. That the row's request is the plan's
/// row of that number is checked by expanding the plan.
pub fn check_row_proof(bytes: &[u8]) -> Result<ProofReport, DatasetRefusal> {
    let proof = RowProof::read(bytes).map_err(DatasetRefusal::Proof)?;
    let mut report = ProofReport {
        holds: false,
        cause: None,
        row: proof.row.row,
        broken_link: None,
        chain_rebuilt: proof.frames.is_some(),
        dataset_seal: proof.dataset_seal.clone(),
    };
    let mut own_frames = proof.frames.as_ref().map(|frames| frames.iter().copied());
    let leaf = match check_row(
        &proof.row,
        proof.row.row,
        own_frames
            .as_mut()
            .map(|frames| frames as &mut dyn Iterator<Item = [u8; FRAME_LEN]>),
    ) {
        Ok(leaf) => leaf,
        Err((cause, broken_link)) => {
            report.cause = Some(cause);
            report.broken_link = broken_link;
            return Ok(report);
        }
    };
    let path: Vec<Hash> = proof
        .audit_path
        .iter()
        .filter_map(|text| hash_from_hex(text))
        .collect();
    let root = tree::root_from_path(proof.row.row, proof.counts.rows, &leaf, &path);
    if root.map(hex::encode).as_deref() != Some(proof.merkle_root.as_str()) {
        report.cause = Some(Cause::Root);
        return Ok(report);
    }
    if dataset_seal(&proof.plan_sha256, &proof.merkle_root, &proof.counts) != proof.dataset_seal {
        report.cause = Some(Cause::DatasetSeal);
        return Ok(report);
    }
    report.holds = true;
    Ok(report)
}

/// What one row of a plan came to.
#[derive(Debug, Clone, PartialEq)]
pub enum RowResult {
    /// The response of `run_trajectory` to the row's request.
    Run(Value),
    /// The door's sentence for a request it refused before computing.
    Unadmitted(String),
}

/// A sealed dataset, ready to be written.
#[derive(Debug, Clone, PartialEq)]
pub struct Assembled {
    pub head: Head,
    pub rows: Vec<Row>,
    /// The frames of every row in row order, when the plan stores them.
    pub frames: Option<Vec<[u8; FRAME_LEN]>>,
}

impl Assembled {
    /// The bytes of the rows file.
    pub fn rows_text(&self) -> Vec<u8> {
        rows_text(&self.rows)
    }

    /// The bytes of the frame file, when the plan stores frames.
    pub fn frames_bytes(&self) -> Option<Vec<u8>> {
        let frames = self.frames.as_ref()?;
        let mut bytes = Vec::with_capacity(crate::frames::HEADER_LEN + frames.len() * FRAME_LEN);
        let header = FramesHeader {
            rows: self.head.counts.rows,
            frames: frames.len() as u64,
            dataset_seal: hash_from_hex(&self.head.dataset_seal)?,
        };
        write_frames(&mut bytes, &header, frames).ok()?;
        Some(bytes)
    }
}

/// The row of a plan's result, and the frames the run recorded.
///
/// The result is the run of the plan's row of that number, or the door's
/// refusal of it. A result that is the run of another request, or whose own
/// seals do not hold, is refused. This is the one place a row is checked,
/// for `assemble` and for `DatasetWriter`.
fn take_row(
    request: &DatasetRequest,
    index: u64,
    result: &RowResult,
) -> Result<(Row, Vec<[u8; FRAME_LEN]>), DatasetRefusal> {
    let planned_row = request.row(index).map_err(DatasetRefusal::Plan)?;
    match result {
        RowResult::Run(response) => {
            let (row, own_frames) = Row::from_run(index, response)
                .map_err(|refusal| DatasetRefusal::Run { row: index, refusal })?;
            if sealed_form(&row.request) != sealed_form(&planned_row) {
                return Err(DatasetRefusal::NotThePlansRow(index));
            }
            Ok((row, own_frames))
        }
        RowResult::Unadmitted(sentence) => {
            Ok((Row::unadmitted(index, planned_row, sentence), Vec::new()))
        }
    }
}

/// The leaf hash of a row. The one place it is taken, for `assemble`, for
/// `prepare_row` and so for `DatasetWriter`.
fn leaf_hash_of(row: &Row) -> Hash {
    let leaf = serde_json::to_string(&row.leaf).expect("a leaf encodes as JSON");
    tree::leaf_hash(leaf.as_bytes())
}

/// A row checked and ready to append: its line of the rows file, its frame
/// bytes, its outcome and frame count, and its leaf hash.
///
/// `prepare_row` makes it from a plan, a row number and a result, on any
/// thread and in any order. `DatasetWriter::push_prepared` takes it, strictly
/// in plan order. The check of the row and the building of its bytes are
/// done once, in `prepare_row`.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedRow {
    index: u64,
    /// The row's line of the rows file, newline included.
    line: Vec<u8>,
    /// The row's frames, 64 bytes each, when the plan stores frames.
    frame_bytes: Vec<u8>,
    frame_count: u64,
    outcome: Outcome,
    row_frames: u64,
    leaf: Hash,
}

impl PreparedRow {
    /// The number of the plan's row this was made from.
    pub fn index(&self) -> u64 {
        self.index
    }
}

/// Check the result of the plan's row `index` as `assemble` does, and build
/// the bytes the writer appends. Reads the plan only, so workers may call it
/// at the same time for different rows.
pub fn prepare_row(
    request: &DatasetRequest,
    index: u64,
    result: &RowResult,
) -> Result<PreparedRow, DatasetRefusal> {
    let (row, own_frames) = take_row(request, index, result)?;
    let mut line = row.to_line().into_bytes();
    line.push(b'\n');
    let mut frame_bytes = Vec::new();
    if request.store() == Store::Frames {
        frame_bytes.reserve(own_frames.len() * FRAME_LEN);
        for frame in &own_frames {
            frame_bytes.extend_from_slice(frame);
        }
    }
    Ok(PreparedRow {
        index,
        line,
        frame_bytes,
        frame_count: own_frames.len() as u64,
        outcome: row.outcome,
        row_frames: row.frames,
        leaf: leaf_hash_of(&row),
    })
}

/// What the tree and the seal keep of the rows taken so far.
struct Tally {
    counts: Counts,
    leaves: Vec<Hash>,
}

impl Tally {
    fn new(engine_version: &str, rows: u64) -> Tally {
        Tally {
            counts: Counts {
                engine_version: engine_version.to_string(),
                rows,
                completed: 0,
                ended: 0,
                refused: 0,
                unadmitted: 0,
                frames: 0,
            },
            leaves: Vec::new(),
        }
    }

    /// Count one row and keep its leaf hash.
    fn add(&mut self, row: &Row) {
        self.keep(row.outcome, row.frames, leaf_hash_of(row));
    }

    /// Count one row by its outcome and frame count, and keep its leaf hash.
    fn keep(&mut self, outcome: Outcome, frames: u64, leaf: Hash) {
        self.counts.frames += frames;
        match outcome {
            Outcome::Completed => self.counts.completed += 1,
            Outcome::Ended => self.counts.ended += 1,
            Outcome::Refused => self.counts.refused += 1,
            Outcome::Unadmitted => self.counts.unadmitted += 1,
        }
        self.leaves.push(leaf);
    }

    /// The root, the dataset seal and the head, with no file hashes.
    fn finish(self, request: &DatasetRequest) -> Head {
        let plan_sha256 = request.plan_sha256();
        let merkle_root =
            hex::encode(tree::root(&self.leaves).expect("a plan has at least one row"));
        let seal = dataset_seal(&plan_sha256, &merkle_root, &self.counts);
        Head {
            request: request.value().clone(),
            counts: self.counts,
            plan_sha256,
            merkle_root,
            dataset_seal: seal,
            rows_sha256: None,
            frames_sha256: None,
        }
    }
}

/// Seal the results of a plan's rows, given in plan order, under one root.
///
/// Each result is the run of the plan's row of the same number, or the
/// door's refusal of it. A result that is the run of another request, or
/// whose own seals do not hold, is refused. The order of the leaves is the
/// order of the plan, whatever order the runs finished in.
pub fn assemble(
    request: &DatasetRequest,
    engine_version: &str,
    results: &[RowResult],
) -> Result<Assembled, DatasetRefusal> {
    let planned = request.totals().rows;
    if results.len() as u64 != planned {
        return Err(DatasetRefusal::ResultsNotRows {
            results: results.len() as u64,
            rows: planned,
        });
    }
    let mut rows = Vec::with_capacity(results.len());
    let mut frames = Vec::new();
    let mut tally = Tally::new(engine_version, planned);
    for (index, result) in results.iter().enumerate() {
        let (row, own_frames) = take_row(request, index as u64, result)?;
        frames.extend(own_frames);
        tally.add(&row);
        rows.push(row);
    }
    let mut assembled = Assembled {
        head: tally.finish(request),
        rows,
        frames: match request.store() {
            Store::Frames => Some(frames),
            Store::None => None,
        },
    };
    if request.returns() == Return::File {
        assembled.head.rows_sha256 = Some(digest_hex(&assembled.rows_text()));
        assembled.head.frames_sha256 = assembled.frames_bytes().map(|bytes| digest_hex(&bytes));
    }
    Ok(assembled)
}

/// What a finished `DatasetWriter` hands back: the head, with both file
/// hashes set, and the two sinks.
pub struct Written<R, F> {
    pub head: Head,
    pub rows: R,
    pub frames: Option<F>,
}

/// Seals a dataset as its rows arrive, and writes the rows file and the
/// frame file as it goes.
///
/// Rows are taken strictly in plan order, one result at a time. Each row is
/// checked as `assemble` checks it, its line and its frames are written at
/// once, and the row is dropped: what stays is the leaf hashes, the counts
/// and the running hash of the rows file. The 64-byte header of the frame
/// file carries the dataset seal, which is known only at the end, so 64
/// bytes are reserved first and written at offset 0 on `finish`. SHA-256 of
/// the frame file covers its header, so `finish` reads the file once more
/// from its start to hash the final bytes.
///
/// With `store` set to `"none"` no frame sink is given, and none is
/// written. For the same plan and the same results the head, the rows file
/// and the frame file are the bytes `assemble` gives.
pub struct DatasetWriter<R, F> {
    request: DatasetRequest,
    tally: Tally,
    next: u64,
    rows_out: R,
    rows_hash: Sha256,
    frames_out: Option<F>,
    stored_frames: u64,
}

fn write_failed(error: std::io::Error) -> DatasetRefusal {
    DatasetRefusal::NotWritten(error.to_string())
}

impl<R: Write, F: Read + Write + Seek> DatasetWriter<R, F> {
    /// Start a dataset. A frame sink is given when the plan stores frames,
    /// and only then. The sink is taken as it stands, at its start.
    pub fn new(
        request: &DatasetRequest,
        engine_version: &str,
        rows_out: R,
        mut frames_out: Option<F>,
    ) -> Result<DatasetWriter<R, F>, DatasetRefusal> {
        if (request.store() == Store::Frames) != frames_out.is_some() {
            return Err(DatasetRefusal::SinkNotAsStored);
        }
        if let Some(sink) = frames_out.as_mut() {
            sink.write_all(&[0_u8; HEADER_LEN]).map_err(write_failed)?;
        }
        Ok(DatasetWriter {
            request: request.clone(),
            tally: Tally::new(engine_version, request.totals().rows),
            next: 0,
            rows_out,
            rows_hash: Sha256::new(),
            frames_out,
            stored_frames: 0,
        })
    }

    /// The number of the row the writer takes next.
    pub fn next_row(&self) -> u64 {
        self.next
    }

    /// Take the result of row `index`, which must be the next row of the plan.
    pub fn push(&mut self, index: u64, result: &RowResult) -> Result<(), DatasetRefusal> {
        self.check_turn(index)?;
        let prepared = prepare_row(&self.request, index, result)?;
        self.append(prepared)
    }

    /// Take a row made by `prepare_row`, which must be the next row of the
    /// plan. The row was checked when it was prepared; here it is only put in
    /// its place: its line and frames are written, its hashes are fed and its
    /// leaf is kept. It must have been prepared from this plan.
    pub fn push_prepared(&mut self, prepared: PreparedRow) -> Result<(), DatasetRefusal> {
        self.check_turn(prepared.index)?;
        self.append(prepared)
    }

    fn check_turn(&self, index: u64) -> Result<(), DatasetRefusal> {
        let planned = self.request.totals().rows;
        if self.next >= planned {
            return Err(DatasetRefusal::ResultsNotRows {
                results: self.next + 1,
                rows: planned,
            });
        }
        if index != self.next {
            return Err(DatasetRefusal::RowOutOfOrder {
                given: index,
                expected: self.next,
            });
        }
        Ok(())
    }

    fn append(&mut self, prepared: PreparedRow) -> Result<(), DatasetRefusal> {
        self.rows_out.write_all(&prepared.line).map_err(write_failed)?;
        self.rows_hash.update(&prepared.line);
        if let Some(sink) = self.frames_out.as_mut() {
            sink.write_all(&prepared.frame_bytes).map_err(write_failed)?;
            self.stored_frames += prepared.frame_count;
        }
        self.tally.keep(prepared.outcome, prepared.row_frames, prepared.leaf);
        self.next += 1;
        Ok(())
    }

    /// Check that every row of the plan was taken, seal the dataset, write
    /// the header of the frame file, and hash both files.
    pub fn finish(mut self) -> Result<Written<R, F>, DatasetRefusal> {
        let planned = self.request.totals().rows;
        if self.next != planned {
            return Err(DatasetRefusal::ResultsNotRows {
                results: self.next,
                rows: planned,
            });
        }
        self.rows_out.flush().map_err(write_failed)?;
        let mut head = self.tally.finish(&self.request);
        head.rows_sha256 = Some(hex::encode(self.rows_hash.finalize()));
        if let Some(sink) = self.frames_out.as_mut() {
            let header = FramesHeader {
                rows: head.counts.rows,
                frames: self.stored_frames,
                dataset_seal: hash_from_hex(&head.dataset_seal).expect("a seal is 64 hex digits"),
            };
            sink.seek(SeekFrom::Start(0)).map_err(write_failed)?;
            sink.write_all(&header.to_bytes()).map_err(write_failed)?;
            sink.flush().map_err(write_failed)?;
            sink.seek(SeekFrom::Start(0)).map_err(write_failed)?;
            let mut hash = Sha256::new();
            let mut block = vec![0_u8; 1 << 20];
            loop {
                let read = sink.read(&mut block).map_err(write_failed)?;
                if read == 0 {
                    break;
                }
                hash.update(&block[..read]);
            }
            head.frames_sha256 = Some(hex::encode(hash.finalize()));
        }
        Ok(Written {
            head,
            rows: self.rows_out,
            frames: self.frames_out,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counts_text_is_compact_json_with_seven_keys_in_order() {
        let counts = Counts {
            engine_version: "genesis_kernel_v0.1.0".to_string(),
            rows: 8,
            completed: 6,
            ended: 2,
            refused: 1,
            unadmitted: 1,
            frames: 904,
        };
        assert_eq!(
            counts.text(),
            r#"{"engine_version":"genesis_kernel_v0.1.0","rows":8,"completed":6,"ended":2,"refused":1,"unadmitted":1,"frames":904}"#
        );
        let plan = "11".repeat(32);
        let root = "22".repeat(32);
        let mut bytes = format!("{plan}{root}").into_bytes();
        bytes.extend_from_slice(counts.text().as_bytes());
        assert_eq!(dataset_seal(&plan, &root, &counts), digest_hex(&bytes));
    }

    #[test]
    fn each_count_and_each_hash_changes_the_dataset_seal() {
        let counts = Counts {
            engine_version: "genesis_kernel_v0.1.0".to_string(),
            rows: 8,
            completed: 6,
            ended: 2,
            refused: 1,
            unadmitted: 1,
            frames: 904,
        };
        let plan = "11".repeat(32);
        let root = "22".repeat(32);
        let seal = dataset_seal(&plan, &root, &counts);
        assert_ne!(dataset_seal(&root, &plan, &counts), seal);
        assert_ne!(dataset_seal(&plan, &"23".repeat(32), &counts), seal);
        for edit in 0..7 {
            let mut other = counts.clone();
            match edit {
                0 => other.engine_version.push('1'),
                1 => other.rows += 1,
                2 => other.completed += 1,
                3 => other.refused += 1,
                4 => other.unadmitted += 1,
                5 => other.ended += 1,
                _ => other.frames += 1,
            }
            assert_ne!(dataset_seal(&plan, &root, &other), seal, "edit {edit}");
        }
    }
}
