//! Writing `rows.parquet` and `frames.parquet` from a dataset that holds.
//!
//! The files are written under a temporary name and renamed at the end, so a
//! failure leaves nothing. The writer properties are fixed: no compression,
//! no timestamp, a fixed `created_by`, fixed row-group sizes. The same input
//! gives the same bytes.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::builder::{
    BooleanBuilder, FixedSizeBinaryBuilder, Float64Builder, StringBuilder, UInt64Builder,
};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field as ArrowField, Schema};
use genesis_seal::frames::HEADER_LEN;
use genesis_seal::rows::{Outcome, Row, lines};
use genesis_seal::run::{FRAME_LEN, PROOF_LEN, PROOF_START};
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::metadata::KeyValue;
use parquet::file::properties::{EnabledStatistics, WriterProperties};

use sha2::{Digest, Sha256};

use crate::verify::Verified;
use crate::state::{Cell, Decoded, Kind, StateSchema};

/// Rows in a row group of `rows.parquet`.
pub const ROW_GROUP_ROWS: usize = 8192;
/// Frames in a row group of `frames.parquet`: 32 Ki frames, about 3 MiB of bytes. A row group of 128 Ki frames was measured to hold the converter at 600 MB, so this stays small.
pub const ROW_GROUP_FRAMES: usize = 32_768;

/// The writer named in each file's metadata: this crate and its own version, read from its manifest.
pub const CREATED_BY: &str = concat!("genesis_seal_parquet ", env!("CARGO_PKG_VERSION"), " (Protocol Company)");

pub const NOTE: &str = "This Parquet file is a conversion of a dataset whose seals were verified, and held, before the conversion. The sealed record is the three source files: the manifest head, rows.jsonl and frames.bin. This file is not sealed itself. A float column holds the value decoded from the row's hex, never from its decimal.";

/// What was written.
#[derive(Debug)]
pub struct Written {
    pub rows_file: String,
    pub row_records: u64,
    pub frames_file: Option<String>,
    pub frame_records: Option<u64>,
}

/// Removes the temporary files unless it is told the work is done.
struct Cleanup {
    paths: Vec<PathBuf>,
    created_directory: Option<PathBuf>,
    armed: bool,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.armed {
            for path in &self.paths {
                let _ = fs::remove_file(path);
            }
            if let Some(directory) = &self.created_directory {
                let _ = fs::remove_dir(directory);
            }
        }
    }
}

fn metadata(verified: &Verified, table: &str) -> Vec<KeyValue> {
    let counts = &verified.head.counts;
    let pair = |key: &str, value: String| KeyValue::new(key.to_string(), value);
    vec![
        pair("table", table.into()),
        pair("note", NOTE.into()),
        pair("dataset_seal", verified.head.dataset_seal.clone()),
        pair("merkle_root", verified.head.merkle_root.clone()),
        pair("plan_sha256", verified.head.plan_sha256.clone()),
        pair("engine_version", counts.engine_version.clone()),
        pair("rows", counts.rows.to_string()),
        pair("completed", counts.completed.to_string()),
        pair("ended", counts.ended.to_string()),
        pair("refused", counts.refused.to_string()),
        pair("unadmitted", counts.unadmitted.to_string()),
        pair("frames", counts.frames.to_string()),
        pair("source_head_sha256", verified.head_sha256.clone()),
        pair("source_rows_sha256", verified.rows_sha256.clone()),
        pair(
            "source_frames_sha256",
            verified.frames_sha256.clone().unwrap_or_else(|| "not given".into()),
        ),
    ]
}

fn properties(verified: &Verified, table: &str, group: usize) -> WriterProperties {
    WriterProperties::builder()
        .set_created_by(CREATED_BY.to_string())
        .set_compression(Compression::UNCOMPRESSED)
        .set_statistics_enabled(EnabledStatistics::Chunk)
        .set_max_row_group_row_count(Some(group))
        .set_key_value_metadata(Some(metadata(verified, table)))
        .build()
}

/// The columns of `rows.parquet`.
pub fn rows_schema(schema: &StateSchema) -> Schema {
    let mut fields = vec![
        ArrowField::new("row", DataType::UInt64, false),
        ArrowField::new("outcome", DataType::Utf8, false),
        ArrowField::new("steps_landed", DataType::UInt64, true),
        ArrowField::new("reject", DataType::Utf8, true),
        ArrowField::new("reject_detail", DataType::Utf8, true),
        ArrowField::new("frames", DataType::UInt64, false),
        ArrowField::new("state_digest", DataType::Utf8, true),
        ArrowField::new("last_link", DataType::Utf8, true),
        ArrowField::new("run_seal", DataType::Utf8, true),
        ArrowField::new("request", DataType::Utf8, false),
    ];
    for field in &schema.fields {
        for name in schema.column_names(field) {
            let kind = if field.kind == Kind::Flag { DataType::Boolean } else { DataType::Float64 };
            fields.push(ArrowField::new(name, kind, true));
        }
    }
    fields.push(ArrowField::new(crate::state::FINAL_RESIDUAL, DataType::Float64, true));
    fields.push(ArrowField::new(crate::state::MAX_RESIDUAL, DataType::Float64, true));
    Schema::new(fields)
}

/// The columns of `frames.parquet`.
pub fn frames_schema() -> Schema {
    Schema::new(vec![
        ArrowField::new("row", DataType::UInt64, false),
        ArrowField::new("frame", DataType::UInt64, false),
        ArrowField::new("frame_bytes", DataType::FixedSizeBinary(FRAME_LEN as i32), false),
        ArrowField::new("proof_slot", DataType::FixedSizeBinary(PROOF_LEN as i32), false),
    ])
}

/// Read every row once. Find the schema from the first row with a state, hold
/// every later row to it, and decode every value, so no later step can fail
/// on a row.
fn find_schema(rows_bytes: &[u8]) -> Result<StateSchema, String> {
    let lines = lines(rows_bytes).ok_or("the rows file is not one row on each line")?;
    let mut found: Option<(u64, StateSchema)> = None;
    for line in lines {
        let row = Row::read_line(line).map_err(|r| format!("a row was not read. {r}"))?;
        let Some(state) = &row.final_state else { continue };
        let at = |reason: String| format!("Row {}: {reason}", row.row);
        match &found {
            None => {
                let schema = StateSchema::from_state(state).map_err(at)?;
                schema
                    .decode(
                        state,
                        row.final_residual_conservation_error_hex.as_deref(),
                        row.max_residual_conservation_error_hex.as_deref(),
                    )
                    .map_err(at)?;
                found = Some((row.row, schema));
            }
            Some((first, schema)) => {
                schema.agrees_with(state).map_err(|reason| {
                    at(format!("final_state does not agree with row {first}: {reason}"))
                })?;
                schema
                    .decode(
                        state,
                        row.final_residual_conservation_error_hex.as_deref(),
                        row.max_residual_conservation_error_hex.as_deref(),
                    )
                    .map_err(at)?;
            }
        }
    }
    // A dataset with no row that has a state still converts: the state
    // columns are the two residuals only.
    Ok(found.map(|(_, schema)| schema).unwrap_or(StateSchema { fields: Vec::new() }))
}

struct RowsBatch<'a> {
    schema: &'a StateSchema,
    arrow: Arc<Schema>,
    row: UInt64Builder,
    outcome: StringBuilder,
    steps_landed: UInt64Builder,
    reject: StringBuilder,
    reject_detail: StringBuilder,
    frames: UInt64Builder,
    state_digest: StringBuilder,
    last_link: StringBuilder,
    run_seal: StringBuilder,
    request: StringBuilder,
    floats: Vec<Float64Builder>,
    flags: Vec<BooleanBuilder>,
    /// For each field: (the index of its first builder, in `floats` or `flags`).
    slots: Vec<usize>,
    final_residual: Float64Builder,
    max_residual: Float64Builder,
    count: usize,
}

impl<'a> RowsBatch<'a> {
    fn new(schema: &'a StateSchema, arrow: Arc<Schema>) -> Self {
        let (mut floats, mut flags, mut slots) = (0, 0, Vec::new());
        for field in &schema.fields {
            match field.kind {
                Kind::Flag => {
                    slots.push(flags);
                    flags += 1;
                }
                Kind::Scalar => {
                    slots.push(floats);
                    floats += 1;
                }
                Kind::Triple => {
                    slots.push(floats);
                    floats += 3;
                }
            }
        }
        RowsBatch {
            schema,
            arrow,
            row: UInt64Builder::new(),
            outcome: StringBuilder::new(),
            steps_landed: UInt64Builder::new(),
            reject: StringBuilder::new(),
            reject_detail: StringBuilder::new(),
            frames: UInt64Builder::new(),
            state_digest: StringBuilder::new(),
            last_link: StringBuilder::new(),
            run_seal: StringBuilder::new(),
            request: StringBuilder::new(),
            floats: (0..floats).map(|_| Float64Builder::new()).collect(),
            flags: (0..flags).map(|_| BooleanBuilder::new()).collect(),
            slots,
            final_residual: Float64Builder::new(),
            max_residual: Float64Builder::new(),
            count: 0,
        }
    }

    fn push(&mut self, row: &Row, decoded: Option<&Decoded>) -> Result<(), String> {
        self.row.append_value(row.row);
        self.outcome.append_value(row.outcome.as_str());
        self.steps_landed.append_option(row.steps_landed);
        self.reject.append_option(row.reject.as_deref());
        self.reject_detail.append_option(row.reject_detail.as_deref());
        self.frames.append_value(row.frames);
        self.state_digest.append_option(row.state_digest.as_deref());
        self.last_link.append_option(row.last_link.as_deref());
        self.run_seal.append_option(row.run_seal.as_deref());
        self.request.append_value(
            serde_json::to_string(&row.request).map_err(|error| error.to_string())?,
        );
        for (index, field) in self.schema.fields.iter().enumerate() {
            let slot = self.slots[index];
            let cell = decoded.map(|d| d.cells[index]);
            match (field.kind, cell) {
                (Kind::Flag, Some(Cell::Flag(v))) => self.flags[slot].append_value(v),
                (Kind::Flag, _) => self.flags[slot].append_null(),
                (Kind::Scalar, Some(Cell::Scalar(v))) => self.floats[slot].append_value(v),
                (Kind::Scalar, _) => self.floats[slot].append_null(),
                (Kind::Triple, Some(Cell::Triple(v))) => {
                    for (i, value) in v.iter().enumerate() {
                        self.floats[slot + i].append_value(*value);
                    }
                }
                (Kind::Triple, _) => {
                    for i in 0..3 {
                        self.floats[slot + i].append_null();
                    }
                }
            }
        }
        self.final_residual.append_option(decoded.map(|d| d.final_residual));
        self.max_residual.append_option(decoded.map(|d| d.max_residual));
        self.count += 1;
        Ok(())
    }

    fn finish(&mut self) -> Result<RecordBatch, String> {
        let mut columns: Vec<ArrayRef> = vec![
            Arc::new(self.row.finish()),
            Arc::new(self.outcome.finish()),
            Arc::new(self.steps_landed.finish()),
            Arc::new(self.reject.finish()),
            Arc::new(self.reject_detail.finish()),
            Arc::new(self.frames.finish()),
            Arc::new(self.state_digest.finish()),
            Arc::new(self.last_link.finish()),
            Arc::new(self.run_seal.finish()),
            Arc::new(self.request.finish()),
        ];
        for (index, field) in self.schema.fields.iter().enumerate() {
            let slot = self.slots[index];
            match field.kind {
                Kind::Flag => columns.push(Arc::new(self.flags[slot].finish())),
                Kind::Scalar => columns.push(Arc::new(self.floats[slot].finish())),
                Kind::Triple => {
                    for i in 0..3 {
                        columns.push(Arc::new(self.floats[slot + i].finish()));
                    }
                }
            }
        }
        columns.push(Arc::new(self.final_residual.finish()));
        columns.push(Arc::new(self.max_residual.finish()));
        self.count = 0;
        RecordBatch::try_new(self.arrow.clone(), columns).map_err(|e| e.to_string())
    }
}

fn write_rows(
    verified: &Verified,
    rows_bytes: &[u8],
    schema: &StateSchema,
    path: &Path,
) -> Result<u64, String> {
    let arrow = Arc::new(rows_schema(schema));
    let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut writer = ArrowWriter::try_new(
        BufWriter::new(file),
        arrow.clone(),
        Some(properties(verified, "rows", ROW_GROUP_ROWS)),
    )
    .map_err(|e| e.to_string())?;
    let mut batch = RowsBatch::new(schema, arrow);
    let mut records = 0_u64;
    // The outcomes written, counted as the head counts them.
    let mut outcomes = [0_u64; 4];
    for line in lines(rows_bytes).ok_or("the rows file is not one row on each line")? {
        let row = Row::read_line(line).map_err(|r| format!("a row was not read. {r}"))?;
        outcomes[match row.outcome {
            Outcome::Completed => 0,
            Outcome::Ended => 1,
            Outcome::Refused => 2,
            Outcome::Unadmitted => 3,
        }] += 1;
        let decoded = match &row.final_state {
            Some(state) => Some(
                schema
                    .decode(
                        state,
                        row.final_residual_conservation_error_hex.as_deref(),
                        row.max_residual_conservation_error_hex.as_deref(),
                    )
                    .map_err(|r| format!("Row {}: {r}", row.row))?,
            ),
            None => None,
        };
        batch.push(&row, decoded.as_ref())?;
        records += 1;
        if batch.count == ROW_GROUP_ROWS {
            writer.write(&batch.finish()?).map_err(|e| e.to_string())?;
        }
    }
    if batch.count > 0 {
        writer.write(&batch.finish()?).map_err(|e| e.to_string())?;
    }
    agree_with_head(verified, records, outcomes)?;
    close(writer)?;
    Ok(records)
}

/// The rows written, and each outcome among them (completed, ended, refused,
/// unadmitted), must be the counts the head carries, which the dataset seal
/// covers. A difference is refused and nothing is written.
fn agree_with_head(verified: &Verified, records: u64, outcomes: [u64; 4]) -> Result<(), String> {
    let counts = &verified.head.counts;
    let head = [
        ("rows", counts.rows, records),
        ("completed", counts.completed, outcomes[0]),
        ("ended", counts.ended, outcomes[1]),
        ("refused", counts.refused, outcomes[2]),
        ("unadmitted", counts.unadmitted, outcomes[3]),
    ];
    for (name, carried, written) in head {
        if carried != written {
            return Err(format!(
                "the head counts {carried} {name} rows and the rows file holds {written}."
            ));
        }
    }
    Ok(())
}

fn close(writer: ArrowWriter<BufWriter<File>>) -> Result<(), String> {
    let buffered = writer.into_inner().map_err(|e| e.to_string())?;
    let file = buffered.into_inner().map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

/// A reader that hashes every byte it gives.
struct Hashing<R> {
    inner: R,
    hasher: Sha256,
}

impl<R: Read> Read for Hashing<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.hasher.update(&buffer[..count]);
        Ok(count)
    }
}

fn write_frames(
    verified: &Verified,
    rows_bytes: &[u8],
    frames: &Path,
    path: &Path,
) -> Result<u64, String> {
    let arrow = Arc::new(frames_schema());
    let file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut writer = ArrowWriter::try_new(
        BufWriter::new(file),
        arrow.clone(),
        Some(properties(verified, "frames", ROW_GROUP_FRAMES)),
    )
    .map_err(|e| e.to_string())?;
    let mut input = Hashing {
        // The same rule as the verification: a regular file, no link, read for its size at open.
        inner: BufReader::with_capacity(
            1 << 20,
            genesis_seal::stored_file::open_regular(frames)
                .map_err(|e| e.to_string())?
                .reader(),
        ),
        hasher: Sha256::new(),
    };
    let mut header = [0_u8; HEADER_LEN];
    input.read_exact(&mut header).map_err(|e| e.to_string())?;

    let mut row_ids = UInt64Builder::new();
    let mut frame_ids = UInt64Builder::new();
    let mut bytes = FixedSizeBinaryBuilder::new(FRAME_LEN as i32);
    let mut proofs = FixedSizeBinaryBuilder::new(PROOF_LEN as i32);
    let mut in_batch = 0_usize;
    let mut records = 0_u64;
    let mut frame = [0_u8; FRAME_LEN];
    let flush = |writer: &mut ArrowWriter<BufWriter<File>>,
                     row_ids: &mut UInt64Builder,
                     frame_ids: &mut UInt64Builder,
                     bytes: &mut FixedSizeBinaryBuilder,
                     proofs: &mut FixedSizeBinaryBuilder|
     -> Result<(), String> {
        let columns: Vec<ArrayRef> = vec![
            Arc::new(row_ids.finish()),
            Arc::new(frame_ids.finish()),
            Arc::new(bytes.finish()),
            Arc::new(proofs.finish()),
        ];
        let batch = RecordBatch::try_new(arrow.clone(), columns).map_err(|e| e.to_string())?;
        writer.write(&batch).map_err(|e| e.to_string())
    };
    for line in lines(rows_bytes).ok_or("the rows file is not one row on each line")? {
        let row = Row::read_line(line).map_err(|r| format!("a row was not read. {r}"))?;
        for index in 0..row.frames {
            input.read_exact(&mut frame).map_err(|e| format!("the frame file ended early: {e}"))?;
            row_ids.append_value(row.row);
            frame_ids.append_value(index);
            bytes.append_value(frame).map_err(|e| e.to_string())?;
            proofs
                .append_value(&frame[PROOF_START..PROOF_START + PROOF_LEN])
                .map_err(|e| e.to_string())?;
            in_batch += 1;
            records += 1;
            if in_batch == ROW_GROUP_FRAMES {
                flush(&mut writer, &mut row_ids, &mut frame_ids, &mut bytes, &mut proofs)?;
                in_batch = 0;
            }
        }
    }
    if in_batch > 0 {
        flush(&mut writer, &mut row_ids, &mut frame_ids, &mut bytes, &mut proofs)?;
    }
    // The frame file is read a second time here. What was converted must be
    // the bytes the verification hashed: every byte is read to the end and hashed,
    // and nothing is kept unless the hash is the one the verification computed.
    io::copy(&mut input, &mut io::sink()).map_err(|e| e.to_string())?;
    let converted = hex::encode(input.hasher.finalize());
    if verified.frames_sha256.as_deref() != Some(converted.as_str()) {
        return Err(
            "the frame file is not the file that was verified: it changed between the verification and the conversion."
                .into(),
        );
    }
    close(writer)?;
    Ok(records)
}

const ROWS_PARTIAL: &str = ".rows.parquet.partial";
const FRAMES_PARTIAL: &str = ".frames.parquet.partial";

fn partial_paths(out: &Path) -> (PathBuf, PathBuf) {
    (out.join(ROWS_PARTIAL), out.join(FRAMES_PARTIAL))
}

/// Remove what an earlier run into this folder left under a temporary name
/// when it was stopped (killed, or out of time) before it could clean up.
pub fn remove_stale_partials(out: &Path) {
    let (rows, frames) = partial_paths(out);
    for path in [rows, frames] {
        let _ = fs::remove_file(path);
    }
}

/// Convert a dataset that `verify` passed. Nothing is left behind on failure.
pub fn convert(
    verified: &Verified,
    rows_bytes: &[u8],
    frames: Option<&Path>,
    out: &Path,
) -> Result<Written, String> {
    let schema = find_schema(rows_bytes)?;
    let created = if out.is_dir() {
        None
    } else {
        fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
        Some(out.to_path_buf())
    };
    let rows_final = out.join("rows.parquet");
    let frames_final = out.join("frames.parquet");
    let (rows_temp, frames_temp) = partial_paths(out);
    let mut cleanup = Cleanup {
        paths: vec![rows_temp.clone(), frames_temp.clone()],
        created_directory: created,
        armed: true,
    };
    let row_records = write_rows(verified, rows_bytes, &schema, &rows_temp)?;
    let frame_records = match frames {
        Some(frames) => Some(write_frames(verified, rows_bytes, frames, &frames_temp)?),
        None => None,
    };
    fs::rename(&rows_temp, &rows_final).map_err(|e| e.to_string())?;
    cleanup.paths = vec![rows_final.clone(), frames_temp.clone()];
    if frame_records.is_some() {
        fs::rename(&frames_temp, &frames_final).map_err(|e| e.to_string())?;
    }
    cleanup.armed = false;
    Ok(Written {
        rows_file: rows_final.display().to_string(),
        row_records,
        frames_file: frame_records.map(|_| frames_final.display().to_string()),
        frame_records,
    })
}
