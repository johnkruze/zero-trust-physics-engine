//! What a flag reports beside its bit, in the answer: the limit, the value,
//! and the extent past the limit in the limit's own unit and as a fraction
//! of it (NASA-STD-7009B [M&S 26]; `genesis_kernel::exceedance`).
//!
//! A step's answer carries `exceedances` beside `state` when its frame
//! carries a flag; a recorded frame of a run carries `exceedances` beside
//! `frame_hex` when that frame carries one; a run with any flagged step
//! carries `exceedance_extremes`, the largest extent of each limit over
//! every landed step, recorded or not. An answer with no flagged step has
//! none of them, so its bytes are those of 0.14.3. Each number has its
//! `_hex` sibling, the 16 hex digits of its 64 bits. The receipt covers
//! them, as it covers `state` and `readout`; the long form computes them
//! again; the frame seal and the run seal do not cover them, as they do not
//! cover `state`'s decimals: the flags word they stand beside is in the
//! frame, and `verify_tape` holds each recorded frame's `exceedances` to
//! the bits of its own flags word (cause `exceedances`).

use genesis_kernel::exceedance::{Exceedance, ExceedanceKind, Exceedances};
use serde_json::{Map, Value, json};

/// The limits a card's flags can mark, from the card's own list.
pub(crate) fn kinds(card: &str) -> &'static [ExceedanceKind] {
    use genesis_kernel::*;
    match card {
        "accumulator_sbo450" => accumulator_sbo450::EXCEEDANCE_KINDS,
        "battery_sag" => battery_sag::EXCEEDANCE_KINDS,
        "cable_stretch" => cable_stretch::EXCEEDANCE_KINDS,
        "cantilever_eth" => cantilever_eth::EXCEEDANCE_KINDS,
        "capsule_insight" => capsule_insight::EXCEEDANCE_KINDS,
        "capsule_stardust" => capsule_stardust::EXCEEDANCE_KINDS,
        "cell_18650pf" => cell_18650pf::EXCEEDANCE_KINDS,
        "cell_2170_model3" => cell_2170_twin::EXCEEDANCE_KINDS,
        "crater_ryugu_sci" => crater_ryugu_sci::EXCEEDANCE_KINDS,
        "creep_cmsx6" => creep_cmsx6::EXCEEDANCE_KINDS,
        "fingerpad_p02_glass" => fingerpad_p02_glass::EXCEEDANCE_KINDS,
        "generator_mmrtg_f1" => generator_mmrtg_f1::EXCEEDANCE_KINDS,
        "generator_mmrtg_seebeck" => generator_mmrtg_seebeck::EXCEEDANCE_KINDS,
        "grasp_lift_glass_4tu" => grasp_lift_glass_4tu::EXCEEDANCE_KINDS,
        "grid_gb" => grid_gb_twin::EXCEEDANCE_KINDS,
        "grid_node" => grid_node::EXCEEDANCE_KINDS,
        "hydraulic_fluid" => hydraulic_fluid::EXCEEDANCE_KINDS,
        "link_gothenburg_10010" => link_gothenburg_10010::EXCEEDANCE_KINDS,
        "los_jwst_fine_guide" => los_jwst_fine_guide::EXCEEDANCE_KINDS,
        "momentum_jwst_dihedral" => momentum_jwst_dihedral::EXCEEDANCE_KINDS,
        "momentum_jwst_srp" => momentum_jwst_srp::EXCEEDANCE_KINDS,
        "reactor_msre_pump" => reactor_msre_pump::EXCEEDANCE_KINDS,
        "reactor_spert3" => reactor_spert3_twin::EXCEEDANCE_KINDS,
        "reactor_spert3_ccore" => reactor_spert3_ccore_plate::EXCEEDANCE_KINDS,
        "reactor_triga_jsi" => reactor_triga_jsi::EXCEEDANCE_KINDS,
        "resonance" => resonance::EXCEEDANCE_KINDS,
        "runaway_18650_lco" => runaway_18650_lco_twin::EXCEEDANCE_KINDS,
        "satellite_lageos1" => satellite_lageos1::EXCEEDANCE_KINDS,
        "swing" => swing::EXCEEDANCE_KINDS,
        "thermal" => thermal::EXCEEDANCE_KINDS,
        "tire_eve_r19_ice" => tire_eve_r19_ice::EXCEEDANCE_KINDS,
        "tokamak_jet_dte1" => tokamak_jet_dte1_twin::EXCEEDANCE_KINDS,
        "tokamak_vde_mast" => tokamak_vde_mast::EXCEEDANCE_KINDS,
        _ => &[],
    }
}

/// The side of a bit set with no record: a fault of the card's list, which
/// the tests hold every card to never show.
pub(crate) const NOT_STATED: &str = "not_stated";

fn number(value: Option<f64>) -> (Value, Value) {
    match value.filter(|value| value.is_finite()) {
        Some(value) => (
            serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number),
            Value::String(format!("{:016x}", value.to_bits())),
        ),
        None => (Value::Null, Value::Null),
    }
}

fn put(object: &mut Map<String, Value>, key: &str, value: Option<f64>) {
    let (decimal, hex) = number(value);
    object.insert(key.to_string(), decimal);
    object.insert(format!("{key}_hex"), hex);
}

fn head(kind: &ExceedanceKind) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("flag".into(), json!(kind.flag));
    object.insert("bit".into(), json!(kind.bit()));
    object.insert("quantity".into(), json!(kind.quantity));
    object.insert("unit".into(), json!(kind.unit));
    object.insert("side".into(), json!(kind.side.word()));
    object
}

/// One record as the answer writes it.
pub(crate) fn record_json(record: &Exceedance) -> Value {
    let mut object = head(&record.kind);
    put(&mut object, "limit", record.limit());
    put(&mut object, "value", record.value());
    put(&mut object, "extent", record.extent());
    put(&mut object, "fraction", record.fraction());
    Value::Object(object)
}

/// The name of the flag at `bit` of a card, from its list.
fn flag_name(card: &str, mask: u64) -> String {
    kinds(card)
        .iter()
        .find(|kind| kind.mask == mask)
        .map(|kind| kind.flag.to_string())
        .unwrap_or_else(|| format!("bit {}", mask.trailing_zeros()))
}

/// In a build that checks (`genesis_kernel::exceedance::CHECKED`: debug
/// assertions, or the gateway's tests): the records of a landed step stand
/// for exactly the bits of its flags word, and none was dropped. Every test
/// that lands a step through the gateway holds the card's list to its flags.
pub(crate) fn debug_check(card: &str, step: u64, flags: u64, records: &Exceedances) {
    if !genesis_kernel::exceedance::CHECKED {
        return;
    }
    assert_eq!(
        records.mask(),
        flags,
        "{card}: the exceedances of step {step} stand for bits {:#b}, its frame carries {flags:#b}: {:?}",
        records.mask(),
        records.iter().map(|record| (record.kind.flag, record.kind.quantity)).collect::<Vec<_>>()
    );
    assert_eq!(records.dropped(), 0, "{card}: step {step} dropped records");
}

/// The `exceedances` of a frame whose flags word is `flags`: the records of
/// the flags it carries, in the card's order, and for a bit it carries with
/// no record, one entry that says so (`side` `not_stated`). Empty when the
/// frame carries no flag.
pub(crate) fn frame_json(card: &str, flags: u64, records: &Exceedances) -> Vec<Value> {
    if flags == 0 {
        return Vec::new();
    }
    let kept = records.only(flags);
    let mut out: Vec<Value> = kept.iter().map(record_json).collect();
    let mut missing = flags & !kept.mask();
    while missing != 0 {
        let mask = missing & missing.wrapping_neg();
        missing &= !mask;
        let mut object = Map::new();
        object.insert("flag".into(), json!(flag_name(card, mask)));
        object.insert("bit".into(), json!(mask.trailing_zeros()));
        object.insert("quantity".into(), Value::Null);
        object.insert("unit".into(), Value::Null);
        object.insert("side".into(), json!(NOT_STATED));
        for key in ["limit", "value", "extent", "fraction"] {
            put(&mut object, key, None);
        }
        out.push(Value::Object(object));
    }
    out
}

/// The largest extent of one limit over the landed steps of a run.
struct Extreme {
    kind: ExceedanceKind,
    steps: u64,
    first_step: u64,
    /// The record with the largest extent and its step: the first such step
    /// on a tie.
    largest: Option<(u64, Exceedance)>,
}

/// The extremes of a run, kept as it goes: one entry for each limit of the
/// card's list that a landed step was past, in the list's order.
#[derive(Default)]
pub(crate) struct Extremes {
    entries: Vec<Extreme>,
}

fn same_kind(left: &ExceedanceKind, right: &ExceedanceKind) -> bool {
    left.mask == right.mask
        && left.quantity == right.quantity
        && left.side == right.side
        && left.limit.map(f64::to_bits) == right.limit.map(f64::to_bits)
        && left.limit_words == right.limit_words
}

impl Extremes {
    /// The records of landed step `step` whose frame carries `flags`.
    pub(crate) fn observe(&mut self, card: &str, step: u64, flags: u64, records: &Exceedances) {
        if flags == 0 {
            return;
        }
        for record in records.only(flags).iter() {
            let at = match self.entries.iter().position(|entry| same_kind(&entry.kind, &record.kind)) {
                Some(at) => at,
                None => {
                    let order = |kind: &ExceedanceKind| kinds(card).iter().position(|listed| same_kind(listed, kind));
                    let place = order(&record.kind);
                    let at = self
                        .entries
                        .iter()
                        .position(|entry| order(&entry.kind) > place)
                        .unwrap_or(self.entries.len());
                    self.entries.insert(at, Extreme { kind: record.kind, steps: 0, first_step: step, largest: None });
                    at
                }
            };
            let entry = &mut self.entries[at];
            entry.steps += 1;
            if let Some(extent) = record.extent() {
                let larger = match &entry.largest {
                    Some((_, held)) => held.extent().is_none_or(|held| extent > held),
                    None => true,
                };
                if larger {
                    entry.largest = Some((step, *record));
                }
            }
        }
    }

    /// `exceedance_extremes`: for each limit a landed step was past, how
    /// many steps were past it, the first, and the step of the largest
    /// extent with its limit, value, extent and fraction. Empty when no
    /// landed step carried a flag.
    pub(crate) fn finish(&self) -> Vec<Value> {
        self.entries
            .iter()
            .map(|entry| {
                let mut object = head(&entry.kind);
                object.insert("steps".into(), json!(entry.steps));
                object.insert("first_step".into(), json!(entry.first_step));
                let record = entry.largest.map(|(_, record)| record);
                object.insert(
                    "largest_at_step".into(),
                    entry.largest.map_or(Value::Null, |(step, _)| json!(step)),
                );
                put(&mut object, "limit", record.and_then(|record| record.limit()));
                put(&mut object, "value", record.and_then(|record| record.value()));
                put(&mut object, "extent", record.and_then(|record| record.extent()));
                put(&mut object, "fraction", record.and_then(|record| record.fraction()));
                Value::Object(object)
            })
            .collect()
    }
}

/// The kinds of a card as `describe_card` lists them under a flag.
pub(crate) fn describe_kinds(card: &str, mask: u64) -> Vec<Value> {
    kinds(card)
        .iter()
        .filter(|kind| kind.mask == mask)
        .map(|kind| {
            let mut object = Map::new();
            object.insert("quantity".into(), json!(kind.quantity));
            object.insert("unit".into(), json!(kind.unit));
            object.insert("side".into(), json!(kind.side.word()));
            let (decimal, hex) = number(kind.limit);
            object.insert("limit".into(), decimal);
            object.insert("limit_hex".into(), hex);
            object.insert("limit_words".into(), json!(kind.limit_words));
            Value::Object(object)
        })
        .collect()
}

/// The words of `describe_card` on what a flag reports beside its bit.
pub(crate) const DESCRIBE_RULE: &str = "A flagged frame reports, for each flag it carries, the limit the flag marks, the value the step reached, and the extent past it, in the limit's unit and as a fraction of the limit's size (NASA-STD-7009B, M&S 26): a step's answer in exceedances beside state, a recorded frame of a run in exceedances beside frame_hex, and a run with any flagged step in exceedance_extremes, the largest extent of each limit over every landed step with its step, the count of steps past it and the first. Each record names flag, bit, quantity, unit and side (above, at_or_above, below, at_or_below, other_than, or state for a flag that marks a state with no number), with limit, value, extent and fraction, each with its _hex; extent is value less limit above a limit, limit less value below it, and the distance either way for other_than; fraction is extent over the limit's size, null where the limit is zero. A flag that marks one of several limits has one record for each limit the step is past. The limits each flag marks are listed under advisory_flags[].limits. The receipt covers them; the frame seal and the run seal do not, as they do not cover state's decimals, and verify_tape holds each recorded frame's records to the bits of its flags word. An answer with no flagged step carries none of them.";

/// The bits the `exceedances` of a recorded frame name, or `None` when an
/// entry has no bit that is a number from 0 to 63.
pub(crate) fn bits_named(entries: &[Value]) -> Option<u64> {
    let mut mask = 0u64;
    for entry in entries {
        let bit = entry.get("bit")?.as_u64()?;
        if bit > 63 {
            return None;
        }
        mask |= 1 << bit;
    }
    Some(mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_with_no_flag_has_no_record_and_a_bit_with_no_record_says_so() {
        let records = Exceedances::new();
        assert!(frame_json("thermal", 0, &records).is_empty());
        let out = frame_json("thermal", genesis_kernel::thermal::FLAG_THERMAL_DERATE | 1 << 5, &records);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["flag"], "FLAG_THERMAL_DERATE");
        assert_eq!(out[0]["side"], NOT_STATED);
        assert_eq!(out[1]["flag"], "bit 5");
        assert_eq!(bits_named(&out), Some(0b10_0001));
    }

    #[test]
    fn a_record_writes_each_number_with_its_hex() {
        let kind = genesis_kernel::thermal::EXCEEDANCE_KINDS[0];
        let mut records = Exceedances::new();
        records.push(Exceedance::fixed(kind, 360.15));
        let out = frame_json("thermal", kind.mask, &records);
        assert_eq!(out.len(), 1);
        let entry = &out[0];
        assert_eq!(entry["flag"], "FLAG_THERMAL_DERATE");
        assert_eq!(entry["bit"], 0);
        assert_eq!(entry["quantity"], "temperature_k");
        assert_eq!(entry["unit"], "K");
        assert_eq!(entry["side"], "at_or_above");
        assert_eq!(entry["limit"], 358.15);
        assert_eq!(entry["limit_hex"], format!("{:016x}", 358.15f64.to_bits()));
        assert_eq!(entry["value"], 360.15);
        assert_eq!(entry["extent_hex"], format!("{:016x}", (360.15f64 - 358.15).to_bits()));
        assert_eq!(entry["fraction_hex"], format!("{:016x}", ((360.15f64 - 358.15) / 358.15).to_bits()));
    }

    #[test]
    fn the_extremes_keep_the_largest_extent_and_its_step() {
        let kind = genesis_kernel::thermal::EXCEEDANCE_KINDS[0];
        let mut extremes = Extremes::default();
        for (step, value) in [(3u64, 358.5), (4, 361.0), (5, 361.0), (6, 359.0)] {
            let mut records = Exceedances::new();
            records.push(Exceedance::fixed(kind, value));
            extremes.observe("thermal", step, kind.mask, &records);
        }
        extremes.observe("thermal", 7, 0, &Exceedances::new());
        let out = extremes.finish();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["steps"], 4);
        assert_eq!(out[0]["first_step"], 3);
        assert_eq!(out[0]["largest_at_step"], 4);
        assert_eq!(out[0]["value"], 361.0);
    }
}
