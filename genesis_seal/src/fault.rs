//! What a verification of a dataset shows, in plain words.
//!
//! `dataset::check_dataset` names the first break by the seal that fails.
//! This says what was done to the rows (one changed, two swapped, one
//! dropped, one repeated, a decimal changed) and the first row that shows it.
//! The tool `verify_dataset` and the program `genesis_seal verify-dataset`
//! give the same words.

use serde_json::Value;

use crate::dataset::{Cause, DatasetReport, FileCheck, Head};
use crate::plan::DatasetRequest;
use crate::rows::Row;
use crate::sealed_form::sealed_form;

/// The kind of fault, in plain words, and the first row that shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    /// The place, counted from 0 in the rows as they were read, of the first
    /// row at fault. `None` when no one row shows the fault.
    pub row: Option<u64>,
    pub says: String,
    /// True when every seal holds and a decimal of a final state is not the
    /// value of the hex beside it.
    pub decimal_changed: bool,
}

/// The bytes of plan rows one search for a row's place may write out. A row
/// of the plan is as large as its base, and a head may state a plan of any
/// number of rows, so the search is held to this much work whatever the head
/// says: the rows nearest the place are tried first, and a row further off
/// than the search reaches is said to be out of its place, not found.
pub const PLACE_SEARCH_BYTES: u64 = 8 << 20;

/// The most rows of the plan one search writes out, for a head of this many
/// bytes, held between 2 and `most_rows`.
pub fn search_most(head_bytes: usize, most_rows: u64) -> u64 {
    (PLACE_SEARCH_BYTES / (head_bytes as u64).max(1)).clamp(2, most_rows.max(2))
}

/// The rows as they were read, held against the plan in the head.
struct RowsRead<'a> {
    plan: DatasetRequest,
    lines: Vec<&'a [u8]>,
    /// The most rows of the plan one search for a row's place writes out.
    search_most: u64,
}

impl RowsRead<'_> {
    /// How many rows the plan has.
    fn planned(&self) -> u64 {
        self.plan.totals().rows
    }

    /// How many rows were read.
    fn count(&self) -> u64 {
        self.lines.len() as u64
    }

    /// The number the row in `place` carries, and its request in sealed form.
    /// `None` for a line that is not read as a row.
    fn read(&self, place: u64) -> Option<(Option<u64>, String)> {
        let value = crate::json_text::read(self.lines.get(usize::try_from(place).ok()?)?).ok()?;
        Some((value.get("row").and_then(Value::as_u64), sealed_form(value.get("request")?)))
    }

    /// The request the plan gives `row`, in sealed form.
    fn request_of(&self, row: u64) -> Option<String> {
        self.plan.row(row).ok().map(|request| sealed_form(&request))
    }

    /// True when the row in `place` is the plan's row of that place: it
    /// carries that number and that row's request.
    fn stands(&self, place: u64) -> bool {
        match self.read(place) {
            Some((Some(number), request)) => number == place && self.request_of(place).as_deref() == Some(request.as_str()),
            _ => false,
        }
    }

    /// The row of the plan that the row in `place` is: the row whose
    /// number it carries, when it carries that row's request; else the row of
    /// its place, when it carries that request; else the nearest row of the
    /// plan with its request, as far as the search reaches. Rows of one plan
    /// may share a request, so the number is read first.
    fn row_of(&self, place: u64) -> Option<u64> {
        let (number, request) = self.read(place)?;
        let is = |row: u64| self.request_of(row).as_deref() == Some(request.as_str());
        if let Some(number) = number.filter(|number| is(*number)) {
            return Some(number);
        }
        if is(place) {
            return Some(place);
        }
        // One row after the place, one before it, two after, two before, and so on.
        let planned = self.planned();
        (1..=self.search_most / 2 + 1)
            .flat_map(|step| [place.checked_add(step), place.checked_sub(step)])
            .flatten()
            .filter(|row| *row < planned)
            .take(usize::try_from(self.search_most).unwrap_or(usize::MAX))
            .find(|row| is(*row))
    }

    /// What the row in `place` shows, when it is not the plan's row of
    /// that place: which row it is, and so whether rows were swapped, one was
    /// dropped or one is repeated.
    fn fault_at(&self, place: u64) -> Fault {
        let (planned, count) = (self.planned(), self.count());
        let Some((number, _)) = self.read(place) else {
            return named(place, format!("The row in place {place} is not read as a row.{}", self.counts()));
        };
        let kind = match self.row_of(place) {
            Some(row) if row > place && self.row_of(row) == Some(place) => {
                format!("Two rows are swapped: row {row} stands in the place of row {place}, and row {place} in the place of row {row}.")
            }
            Some(row) if row > place && count < planned => {
                format!("A row was dropped: row {place} is the first row missing, and the row in its place is row {row}.")
            }
            Some(row) if row > place && self.stands(row) => {
                format!("A row is repeated: row {row} stands in its own place and also in the place of row {place}, so row {place} is missing.")
            }
            Some(row) if row > place => format!("Rows are out of order: the row in the place of row {place} is row {row}."),
            Some(row) if row < place => {
                let again = if self.stands(row) {
                    format!("row {row} stands in its own place and again")
                } else {
                    format!("row {row} is given again")
                };
                if place >= planned {
                    format!("A row is repeated: {again} in place {place}, after the plan's last row.")
                } else if count > planned {
                    format!("A row is repeated: {again} in place {place}.")
                } else {
                    format!("A row is repeated: {again} in the place of row {place}, so row {place} is missing.")
                }
            }
            Some(_) => match number {
                Some(number) => format!("A row was changed: the row in place {place} carries the request of row {place} and the number {number}."),
                None => format!("A row was changed: the row in place {place} carries the request of row {place} and no whole number as its row."),
            },
            None if place >= planned => {
                format!("A row was added: the plan has {planned} rows and so none for place {place}, and the row there is not found among the rows of the plan nearest that place.")
            }
            None if number == Some(place) => format!("A row was changed: the request of row {place} is not the request the plan gives it."),
            None => {
                format!("A row is out of its place or not of this plan: the row in place {place} does not carry the request the plan gives row {place}.")
            }
        };
        named(place, format!("{kind}{}", self.counts()))
    }

    /// The two counts, said when they differ.
    fn counts(&self) -> String {
        if self.count() == self.planned() {
            String::new()
        } else {
            format!(" Rows read: {}; rows in the plan: {}.", self.count(), self.planned())
        }
    }
}

fn named(row: u64, says: String) -> Fault {
    Fault { row: Some(row), says, decimal_changed: false }
}

fn unnamed(says: &str) -> Fault {
    Fault { row: None, says: says.to_string(), decimal_changed: false }
}

/// The first place, in rows read against a plan, that does not hold its own
/// row: that place, the row of the plan found there, and what it shows. For
/// the tests of the search.
#[doc(hidden)]
pub fn first_out_of_place(plan: DatasetRequest, lines: &[&[u8]], search_most: u64) -> Option<(u64, Option<u64>, String)> {
    let rows = RowsRead { plan, lines: lines.to_vec(), search_most };
    let place = (0..rows.count()).find(|place| !rows.stands(*place))?;
    Some((place, rows.row_of(place), rows.fault_at(place).says))
}

/// A decimal copy held to the hex beside it: the number the decimal reads as
/// is the number the hex holds, and a 3-vector the same in each component.
/// Zero and minus zero are one number here, since a JSON writer may drop the
/// sign of a zero. A decimal that is not a number (a string, null) is not
/// its hex.
pub fn decimal_is_its_hex(decimal: &Value, hex: &Value) -> bool {
    match (decimal, hex) {
        (Value::Number(number), Value::String(digits)) => {
            match (number.as_f64(), u64::from_str_radix(digits, 16)) {
                (Some(read), Ok(bits)) => read == f64::from_bits(bits),
                _ => false,
            }
        }
        (Value::Array(decimals), Value::Array(hexes)) => {
            decimals.len() == hexes.len() && decimals.iter().zip(hexes).all(|(decimal, hex)| decimal_is_its_hex(decimal, hex))
        }
        _ => false,
    }
}

/// The decimal a hex value is written as, as the engine writes it.
pub fn decimal_of_hex(hex: &Value) -> Value {
    match hex {
        Value::String(digits) => u64::from_str_radix(digits, 16)
            .ok()
            .and_then(|bits| serde_json::Number::from_f64(f64::from_bits(bits)))
            .map_or(Value::Null, Value::Number),
        Value::Array(items) => Value::Array(items.iter().map(decimal_of_hex).collect()),
        _ => Value::Null,
    }
}

/// The first row whose final state carries a decimal that is not the value
/// of the hex beside it. The seals cover the hex values and no decimal, so a
/// dataset with a decimal changed still holds; this names the row.
pub fn changed_decimal(rows: &[u8]) -> Option<Fault> {
    let lines = crate::rows::lines(rows)?;
    for (place, line) in lines.iter().enumerate() {
        let Ok(row) = Row::read_line(line) else { continue };
        let Some(state) = &row.final_state else { continue };
        for (key, decimal) in state {
            if key.ends_with("_hex") || decimal.is_boolean() {
                continue;
            }
            let Some(hex) = state.get(&format!("{key}_hex")) else { continue };
            if !decimal_is_its_hex(decimal, hex) {
                return Some(Fault {
                    row: Some(place as u64),
                    says: format!(
                        "A decimal was changed in row {place}: final_state.{key} reads {decimal}, and the exact value beside it, {key}_hex {hex}, is {}. No seal covers a decimal, so holds is true: the sealed values, which are the hex values, are unaltered.",
                        decimal_of_hex(hex)
                    ),
                    decimal_changed: true,
                });
            }
        }
    }
    None
}

/// The kind of fault a report shows, in plain words, with the first row that
/// shows it. A dataset that holds has a fault only when a decimal of a final
/// state is not its hex. `most_rows` bounds the search for a row's place (see
/// `search_most`).
pub fn fault_of(head: &[u8], rows: &[u8], report: &DatasetReport, most_rows: u64) -> Option<Fault> {
    if report.holds {
        return changed_decimal(rows);
    }
    let rows_read = || -> Option<(RowsRead, String)> {
        let sealed_plan = Head::read(head).ok()?;
        let plan = DatasetRequest::from_value(sealed_plan.request.clone()).ok()?;
        let search_most = search_most(head.len(), most_rows);
        Some((RowsRead { plan, lines: crate::rows::lines(rows)?, search_most }, sealed_plan.plan_sha256))
    };
    match (report.cause?, report.row) {
        (Cause::Plan, None) => {
            let (read, sealed_plan) = rows_read()?;
            if read.plan.plan_sha256() != sealed_plan {
                return Some(unnamed(
                    "The plan was changed: the request in the head does not hash to the plan_sha256 in the head. No row is named: the rows were not compared with a plan that is not the sealed one.",
                ));
            }
            // The rows read are not as many as the plan has. The first place
            // that does not hold its own row shows what was done.
            match (0..read.count()).find(|place| !read.stands(*place)) {
                Some(place) => Some(read.fault_at(place)),
                None => Some(named(
                    read.count(),
                    format!(
                        "A row was dropped: every row read stands in its own place, and row {} is the first row missing, after the last row read.{}",
                        read.count(),
                        read.counts()
                    ),
                )),
            }
        }
        (Cause::Plan, Some(place)) => Some(rows_read()?.0.fault_at(place)),
        (Cause::Leaf, Some(place)) => {
            let (read, _) = rows_read()?;
            match read.read(place) {
                Some((Some(number), _)) if number == place => Some(named(
                    place,
                    format!("A row was changed: the leaf of row {place} is not the leaf its number, its request, its outcome, its run_seal and its reject_detail give."),
                )),
                _ => Some(read.fault_at(place)),
            }
        }
        (Cause::Chain, Some(row)) => Some(named(
            row,
            format!("A row was changed, or its frames were: the frames of row {row}, its count of frames and its last_link do not agree, so its chain does not lead from its request to its last_link."),
        )),
        (Cause::LoadSignature, Some(row)) => Some(named(
            row,
            format!("A row was changed: a signed reading in the request of row {row} does not verify, or is not well formed, so the reading it carries is not the one its instrument's key signed."),
        )),
        (Cause::State, Some(row)) => Some(named(
            row,
            format!("A row was changed: a sealed value of row {row} was altered. Its state_digest is not the digest of the hex values and flags of its final state and its two conservation errors."),
        )),
        (Cause::RunSeal, Some(row)) => Some(named(
            row,
            format!("A row was changed: the run_seal of row {row} is not the seal its last_link, outcome, steps_landed, reject and state_digest give, so one of those, or the run_seal, was altered."),
        )),
        (Cause::Root, _) => Some(unnamed(
            "No one row shows a fault: every row is consistent with itself and is the plan's row of its place, and together they do not give the merkle_root in the head. A row was replaced by another run of its request that is consistent with itself, or the merkle_root in the head was changed. The rows alone do not show which row: replaying a row's request through run_trajectory and comparing run seals shows whether that row is the engine's.",
        )),
        (Cause::DatasetSeal, _) => Some(unnamed(
            "No row shows a fault: the rows give the merkle_root in the head. The counts or the dataset_seal in the head were changed, or the header of the frame file states another count of rows or of frames, or another seal.",
        )),
        _ => None,
    }
}

fn or_none<T: std::fmt::Display>(value: Option<T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "none".to_string(),
    }
}

/// The lines `genesis_seal verify-dataset` prints, one `name: value` each,
/// each ending with a line break. `row` is the row of the first break as the
/// seals name it, or, where they name none (a row dropped or repeated, a
/// decimal changed), the first row the fault shows at. `fault` follows when
/// there is one.
pub fn report_lines(report: &DatasetReport, fault: Option<&Fault>) -> String {
    let row = report.row.or(fault.and_then(|fault| fault.row));
    let mut text = String::new();
    let mut line = |name: &str, value: &str| {
        text.push_str(name);
        text.push_str(": ");
        text.push_str(value);
        text.push('\n');
    };
    line("holds", &report.holds.to_string());
    line("cause", &or_none(report.cause));
    line("row", &or_none(row));
    line("broken_link", &or_none(report.broken_link));
    line("chains", report.chains.as_str());
    line("merkle_root", &or_none(report.merkle_root.as_deref()));
    line("dataset_seal", &or_none(report.dataset_seal.as_deref()));
    line("receipt", report.receipt.as_str());
    line("rows_file", report.rows_file.as_str());
    line("frames_file", report.frames_file.as_str());
    if let Some(fault) = fault {
        line("fault", &fault.says);
    }
    text
}

/// True when every seal holds and the bytes are not the engine's: the
/// receipt or a file does not match, or a decimal is not its hex.
pub fn bytes_differ(report: &DatasetReport, fault: Option<&Fault>) -> bool {
    report.holds
        && (report.receipt != crate::receipt::ReceiptCheck::Matches
            || report.rows_file == FileCheck::DoesNotMatch
            || report.frames_file == FileCheck::DoesNotMatch
            || fault.is_some_and(|fault| fault.decimal_changed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A decimal is held to the hex beside it by the number it reads as.
    #[test]
    fn a_decimal_is_its_hex_when_it_reads_as_the_number_the_hex_holds() {
        let hex = |value: f64| json!(format!("{:016x}", value.to_bits()));
        assert!(decimal_is_its_hex(&json!(293.15), &hex(293.15)));
        assert!(decimal_is_its_hex(&json!(10), &hex(10.0)));
        assert!(!decimal_is_its_hex(&json!(293.1), &hex(293.15)));
        // The neighbouring 64-bit number is another number.
        assert!(!decimal_is_its_hex(&json!(f64::from_bits(293.15_f64.to_bits() + 1)), &hex(293.15)));
        // A zero written without its sign is the same zero.
        assert!(decimal_is_its_hex(&json!(0), &hex(-0.0)) && decimal_is_its_hex(&json!(-0.0), &hex(0.0)));
        assert!(decimal_is_its_hex(&json!([1.5, 2.5, -3.5]), &json!([hex(1.5), hex(2.5), hex(-3.5)])));
        assert!(!decimal_is_its_hex(&json!([1.5, 2.5, 3.5]), &json!([hex(1.5), hex(2.5), hex(-3.5)])));
        assert!(!decimal_is_its_hex(&json!([1.5, 2.5]), &json!([hex(1.5), hex(2.5), hex(-3.5)])));
        assert!(!decimal_is_its_hex(&json!("293.15"), &hex(293.15)) && !decimal_is_its_hex(&Value::Null, &hex(293.15)));
        assert_eq!(decimal_of_hex(&hex(293.15)), json!(293.15));
        assert_eq!(decimal_of_hex(&json!([hex(1.5), hex(2.5)])), json!([1.5, 2.5]));
    }

    #[test]
    fn the_search_writes_out_no_more_rows_than_it_is_given_whatever_the_plans_size() {
        assert_eq!(search_most(4 << 20, 100_000), 2);
        assert_eq!(search_most(1, 100_000), 100_000);
        assert_eq!(search_most(1 << 20, 100_000), 8);
    }
}
