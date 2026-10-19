#![no_implicit_prelude]
//! What a flag reports beside its bit: the limit it marks, the value the
//! step reached, and the extent past the limit, in the limit's own unit and
//! as a fraction of it.
//!
//! A frame's flags word names a condition. NASA-STD-7009B [M&S 26] asks that
//! a result used outside its domain of validation also state the extent
//! that the limit was exceeded. Each card that sets flags lists, in
//! `EXCEEDANCE_KINDS`, every limit a flag of it can mark, and its function
//! `exceedances` returns, for the step from `before` to `after`, one
//! [`Exceedance`] for each limit the step is past. A flag that marks one of
//! several limits (a value outside a compared range on any of several
//! quantities) has one record for each limit the step is past. The bits of
//! the records are the bits of the frame's flags word: no record without
//! its bit, no bit without a record.
//!
//! Nothing here changes a step, a frame or a flag: the records are read
//! from the step as it landed.

extern crate core;

use core::iter::Iterator;
use core::option::Option::{self, None, Some};
use core::prelude::v1::*;


/// Which side of its limit a flagged value lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The flag is set when the value is above the limit.
    Above,
    /// The flag is set when the value is at or above the limit.
    AtOrAbove,
    /// The flag is set when the value is below the limit.
    Below,
    /// The flag is set when the value is at or below the limit.
    AtOrBelow,
    /// The flag is set when the value is other than the limit: the extent
    /// is the distance from it either way.
    OtherThan,
    /// The flag marks a state with no number to compare: no limit, no
    /// value, no extent.
    State,
}

impl Side {
    /// The word an answer writes for the side.
    pub const fn word(self) -> &'static str {
        match self {
            Side::Above => "above",
            Side::AtOrAbove => "at_or_above",
            Side::Below => "below",
            Side::AtOrBelow => "at_or_below",
            Side::OtherThan => "other_than",
            Side::State => "state",
        }
    }
}

/// One limit a flag of a card can mark.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExceedanceKind {
    /// The flag's name, `FLAG_…`.
    pub flag: &'static str,
    /// The flag's mask in the frame's flags word.
    pub mask: u64,
    /// The quantity compared with the limit, in snake case.
    pub quantity: &'static str,
    /// The unit of the quantity and of its limit: `"1"` for a pure number.
    pub unit: &'static str,
    pub side: Side,
    /// The limit when it is a constant of the card; `None` when it varies
    /// with the run (a share of a density that moves, the load a rod is
    /// still short of), in which case `limit_words` says what it is.
    pub limit: Option<f64>,
    /// The limit in words: what it is and where it comes from.
    pub limit_words: &'static str,
}

impl ExceedanceKind {
    /// A kind whose limit is the constant `limit`.
    pub const fn fixed(
        flag: &'static str,
        mask: u64,
        quantity: &'static str,
        unit: &'static str,
        side: Side,
        limit: f64,
        limit_words: &'static str,
    ) -> Self {
        Self { flag, mask, quantity, unit, side, limit: Some(limit), limit_words }
    }

    /// A kind whose limit varies with the run, said in `limit_words`.
    pub const fn varying(
        flag: &'static str,
        mask: u64,
        quantity: &'static str,
        unit: &'static str,
        side: Side,
        limit_words: &'static str,
    ) -> Self {
        Self { flag, mask, quantity, unit, side, limit: None, limit_words }
    }

    /// A kind that marks a state with no number.
    pub const fn state(flag: &'static str, mask: u64, quantity: &'static str, limit_words: &'static str) -> Self {
        Self { flag, mask, quantity, unit: "", side: Side::State, limit: None, limit_words }
    }

    /// The flag's bit: its place in the flags word.
    pub const fn bit(&self) -> u32 {
        self.mask.trailing_zeros()
    }
}

/// One limit a step is past: its kind, the limit, and the value the step
/// reached. A [`Side::State`] record has neither.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exceedance {
    pub kind: ExceedanceKind,
    pub reading: Option<Reading>,
}

/// The limit and the value of a record, in the kind's unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub limit: f64,
    pub value: f64,
}

impl Exceedance {
    /// The record of `kind` at `value` against `limit`.
    pub const fn at(kind: ExceedanceKind, limit: f64, value: f64) -> Self {
        Self {
            kind,
            reading: Some(Reading { limit, value }),
        }
    }

    /// The record of `kind` at `value` against the kind's own constant
    /// limit. A kind with no constant limit is read against zero; such a
    /// kind is recorded with [`Exceedance::at`].
    pub fn fixed(kind: ExceedanceKind, value: f64) -> Self {
        Self::at(kind, kind.limit.unwrap_or(0.0), value)
    }

    /// The record of a state with no number.
    pub const fn state(kind: ExceedanceKind) -> Self {
        Self { kind, reading: None }
    }

    pub fn limit(&self) -> Option<f64> {
        self.reading.map(|reading| reading.limit)
    }

    pub fn value(&self) -> Option<f64> {
        self.reading.map(|reading| reading.value)
    }

    /// How far past the limit the value is, in the limit's unit: value less
    /// limit above it, limit less value below it, the distance either way
    /// for [`Side::OtherThan`]. Zero for a value at a limit the flag is set
    /// at.
    pub fn extent(&self) -> Option<f64> {
        let Reading { limit, value } = self.reading?;
        Some(match self.kind.side {
            Side::Above | Side::AtOrAbove => value - limit,
            Side::Below | Side::AtOrBelow => limit - value,
            Side::OtherThan => abs_f64(value - limit),
            Side::State => return None,
        })
    }

    /// The extent as a fraction of the limit's size: `extent / |limit|`.
    /// `None` when the limit is zero, where no fraction of it is defined.
    pub fn fraction(&self) -> Option<f64> {
        let extent = self.extent()?;
        let limit = self.limit()?;
        if limit == 0.0 {
            return None;
        }
        Some(extent / abs_f64(limit))
    }
}

/// The most records one step can carry. No card's flags mark more limits
/// than this at once; a record past it is counted and not kept.
pub const MOST_RECORDS: usize = 24;

/// The records of one step, in the order its card lists its kinds. Fixed in
/// size: a card's step allocates nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exceedances {
    records: [Option<Exceedance>; MOST_RECORDS],
    len: usize,
    /// Records pushed past [`MOST_RECORDS`]: zero on every card.
    dropped: usize,
}

impl Default for Exceedances {
    fn default() -> Self {
        Self::new()
    }
}

impl Exceedances {
    pub const fn new() -> Self {
        Self {
            records: [None; MOST_RECORDS],
            len: 0,
            dropped: 0,
        }
    }

    pub fn push(&mut self, record: Exceedance) {
        if self.len < MOST_RECORDS {
            self.records[self.len] = Some(record);
            self.len += 1;
        } else {
            self.dropped += 1;
        }
    }

    /// `record` pushed when `past` holds.
    pub fn push_if(&mut self, past: bool, record: Exceedance) {
        if past {
            self.push(record);
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn dropped(&self) -> usize {
        self.dropped
    }

    pub fn iter(&self) -> impl Iterator<Item = &Exceedance> {
        self.records[..self.len].iter().filter_map(Option::as_ref)
    }

    /// The flags word the records stand for: the union of their masks.
    pub fn mask(&self) -> u64 {
        self.iter().fold(0, |mask, record| mask | record.kind.mask)
    }

    /// The records whose flag is in `mask`, and no other.
    pub fn only(&self, mask: u64) -> Self {
        let mut kept = Self::new();
        for record in self.iter() {
            if record.kind.mask & mask != 0 {
                kept.push(*record);
            }
        }
        kept
    }
}

fn abs_f64(value: f64) -> f64 {
    if value < 0.0 { -value } else { value }
}

/// Whether this build checks, after every step that lands, that the step's
/// records stand for exactly the bits of its frame's flags word: in a build
/// with debug assertions, in the kernel's own tests, and with the feature
/// `check_exceedances`, which the gateway's tests turn on. No served build
/// checks: a served step's records are read only when its frame is flagged.
pub const CHECKED: bool = cfg!(any(test, debug_assertions, feature = "check_exceedances"));

/// The check [`CHECKED`] names: the records of a step that landed stand for
/// exactly the bits of its frame's flags word, and none was dropped. Every
/// card's `step` asks it in such a build, so every test that lands a step
/// holds the card's `exceedances` to its flags.
pub fn debug_check(card: &str, records: Exceedances, flags: u64) {
    core::assert_eq!(
        records.mask(),
        flags,
        "{}: the records of a landed step stand for bits {:#b}, its frame carries {:#b}",
        card,
        records.mask(),
        flags
    );
    core::assert_eq!(records.dropped(), 0, "{}: a landed step dropped records", card);
    #[cfg(test)]
    coverage::note(card, &records);
}

/// In the kernel's own tests, with `EXCEEDANCE_COVERAGE` naming a file:
/// each kind a landed step was past, once, as `card flag quantity side`, so
/// a run of the tests shows which kinds its steps reached.
#[cfg(test)]
mod coverage {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::io::Write;
    use std::string::{String, ToString};
    use std::sync::Mutex;
    use std::vec::Vec;

    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub fn note(card: &str, records: &super::Exceedances) {
        let Ok(path) = std::env::var("EXCEEDANCE_COVERAGE") else {
            return;
        };
        let mut seen = SEEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        for record in records.iter() {
            let line = std::format!(
                "{card}\t{}\t{}\t{}\t{:?}",
                record.kind.flag,
                record.kind.quantity,
                record.kind.side.word(),
                record.kind.limit
            );
            if !seen.contains(&line) {
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                    let _ = std::writeln!(file, "{line}");
                }
                seen.push(line.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::*;

    const ABOVE: ExceedanceKind = ExceedanceKind {
        flag: "FLAG_TEST_ABOVE",
        mask: 1 << 2,
        quantity: "temperature_k",
        unit: "K",
        side: Side::AtOrAbove,
        limit: Some(358.15),
        limit_words: "test",
    };
    const BELOW: ExceedanceKind = ExceedanceKind {
        flag: "FLAG_TEST_BELOW",
        mask: 1,
        quantity: "voltage_v",
        unit: "V",
        side: Side::Below,
        limit: Some(42.0),
        limit_words: "test",
    };
    const OTHER: ExceedanceKind = ExceedanceKind {
        flag: "FLAG_TEST_OTHER",
        mask: 1 << 1,
        quantity: "lift",
        unit: "1",
        side: Side::OtherThan,
        limit: Some(0.0),
        limit_words: "test",
    };

    #[test]
    fn the_extent_is_past_the_limit_on_its_side_in_its_unit_and_as_a_fraction() {
        let above = Exceedance::fixed(ABOVE, 360.15);
        assert_eq!(above.extent(), Some(360.15 - 358.15));
        assert_eq!(above.fraction(), Some((360.15 - 358.15) / 358.15));
        let at = Exceedance::fixed(ABOVE, 358.15);
        assert_eq!(at.extent(), Some(0.0));
        let below = Exceedance::fixed(BELOW, 40.0);
        assert_eq!(below.extent(), Some(2.0));
        assert_eq!(below.fraction(), Some(2.0 / 42.0));
        let other = Exceedance::fixed(OTHER, -0.25);
        assert_eq!(other.extent(), Some(0.25));
        assert_eq!(other.fraction(), None, "no fraction of a zero limit");
        let negative = Exceedance::at(BELOW, -2.0, -3.0);
        assert_eq!(negative.fraction(), Some(0.5), "the fraction is of the limit's size");
    }

    #[test]
    fn a_state_has_no_number_and_the_mask_is_the_union_of_the_records() {
        let state = Exceedance::state(ExceedanceKind { side: Side::State, limit: None, ..BELOW });
        assert_eq!(state.extent(), None);
        assert_eq!(state.fraction(), None);
        assert_eq!(state.limit(), None);
        assert_eq!(ABOVE.bit(), 2);
        let mut records = Exceedances::new();
        assert_eq!(records.mask(), 0);
        assert!(records.is_empty());
        records.push(Exceedance::fixed(ABOVE, 400.0));
        records.push_if(false, Exceedance::fixed(OTHER, 1.0));
        records.push(Exceedance::fixed(BELOW, 1.0));
        records.push(state);
        assert_eq!(records.mask(), 5);
        assert_eq!(records.len(), 3);
        assert_eq!(records.only(1).len(), 2);
        for _ in 0..MOST_RECORDS {
            records.push(state);
        }
        assert_eq!(records.len(), MOST_RECORDS);
        assert_eq!(records.dropped(), 3);
    }
}
