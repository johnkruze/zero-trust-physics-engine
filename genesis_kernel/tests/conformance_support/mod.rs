//! The card contract, attacked the same way on every card.
//!
//! A card plugs in with one `Kit`: its specimen, its `is_valid`, its `step`,
//! a start and a load that land, its ledger limit, and the fields of its
//! ledger and of its physical state by name. `findings` then attacks the
//! contract of `genesis_kernel/AGENTS.md` and returns what it found; nothing
//! here knows any card.
//!
//! What is attacked:
//!
//! - **Fixed constants** (rule 1). Every 64-bit word of the specimen that
//!   reads as a normal number is moved by one unit in the last place. The
//!   card must then fail `is_valid`, or `step` must refuse. A moved word that
//!   `is_valid` passes and that changes the landed state is a constant the
//!   card does not hold.
//! - **A refusal leaves the state whole** (rules 3 to 5). A step of NaN, of
//!   either infinity, of zero, of minus zero and of a negative size must be
//!   refused, with the state and the frame as they were, bit for bit.
//! - **Nothing filled in** (rule 4). Every word of the load and of the state
//!   that reads as a normal number is set to NaN and to each infinity in
//!   turn. The step must be refused with the state and the frame whole.
//! - **The ledger** (rule 6). A run at the working step, at the ceiling and
//!   at the floor reports a residual at or under the card's limit at every
//!   landed step. Each ledger field, then each physical field, is moved
//!   without booking it, as a wrong stepping method or a falsified ledger
//!   would leave the state; the next step must name it or close its ledger.
//! - **Flags and proof** (rules 7 and 8). A landed step sets no bit outside
//!   the card's declared flags, and never writes the frame's proof.
//! - **The step ceiling and floor** (rule 12). Both are declared, finite and
//!   positive, and the floor is under the ceiling.
//! - **Same input, same bits.** The same run twice gives the same state and
//!   the same frames.
//! - **Normal or zero** (engine invariant 4, `genesis_kernel::normal`). No
//!   landed state holds a subnormal number: not the kit's own run, and not a
//!   run with one word of the start or of the load set to the smallest
//!   normal number or to 2^-1000, each with its sign, which drives the
//!   values that scale with it under the normal range. A run of the card's
//!   own that stops on a state it wrote itself (a name with `Inconsistent`)
//!   is a finding too: the landed state must step on.
//!
//! The words are found by reading the value's bytes. This is a test-only
//! tool for cards, which are plain records of numbers: only a word that reads
//! as a normal 64-bit number is touched, so a count, a flag, an index or an
//! address is left alone.

#![allow(dead_code)]

use std::fmt::Debug;
use std::panic::{AssertUnwindSafe, catch_unwind};

use genesis_kernel::frame::LastStateFrame64;

/// How much a finding weighs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weight {
    /// A rule of the contract is broken.
    Contract,
    /// Worth a look; the contract may still hold.
    Note,
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub model: &'static str,
    pub rule: &'static str,
    pub weight: Weight,
    pub detail: String,
}

impl Finding {
    pub fn line(&self) -> String {
        let weight = match self.weight {
            Weight::Contract => "CONTRACT",
            Weight::Note => "note",
        };
        format!("{} | {} | {} | {}", self.model, weight, self.rule, self.detail)
    }

    /// One JSON line, for the report.
    pub fn json(&self) -> String {
        let escape = |text: &str| text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ");
        format!(
            "{{\"model\":\"{}\",\"layer\":\"kernel\",\"weight\":\"{}\",\"rule\":\"{}\",\"detail\":\"{}\"}}",
            self.model,
            if self.weight == Weight::Contract { "contract" } else { "note" },
            self.rule,
            escape(&self.detail)
        )
    }
}

/// A named `f64` of a state.
pub type Field<S> = (&'static str, fn(&mut S) -> &mut f64);

/// `field!(StateType, name)`: the state's field `name` as a [`Field`].
#[macro_export]
macro_rules! field {
    ($state:ty, $($path:tt)+) => {{
        fn reach(state: &mut $state) -> &mut f64 {
            &mut state.$($path)+
        }
        (stringify!($($path)+), reach as fn(&mut $state) -> &mut f64)
    }};
}

/// Everything the attacks need of one card.
pub struct Kit<C, S, L, R> {
    pub model: &'static str,
    pub card: C,
    pub is_valid: fn(&C) -> bool,
    pub step: fn(&C, &mut S, &mut LastStateFrame64, L, f64) -> Result<(), R>,
    pub gate_dt_s: Option<f64>,
    pub floor_dt_s: Option<f64>,
    /// A start that lands.
    pub start: S,
    /// A load under which `steps` steps of `dt_s` land from `start`.
    pub load: L,
    pub dt_s: f64,
    pub steps: usize,
    pub residual: fn(&S) -> f64,
    /// The card's `RESIDUAL_CONSERVATION_LIMIT`.
    pub limit: f64,
    /// The flags the card declares, ORed.
    pub flags: u64,
    /// The fields of its ledger: anchors, booked work, booked heat.
    pub ledger: Vec<Field<S>>,
    /// The fields of its physical state the ledger's stored quantity is
    /// computed from: moved unbooked, each must break the ledger. A field
    /// that drives a flow the ledger books as it goes (a governor's output,
    /// a held rate) is not one.
    pub physical: Vec<Field<S>>,
    /// Rules the card's crate map declares a departure from: a finding on
    /// one of them is reported as a note that says so.
    pub departures: Vec<&'static str>,
}

/// A value as its fields: the `Debug` of a record of numbers writes every
/// field, each 64-bit number in the digits that read back to its bits, and
/// leaves out the padding beside a flag, whose bytes are no part of it.
fn bytes_of<T: Debug>(value: &T) -> String {
    format!("{value:?}")
}

fn word_at<T>(value: &T, offset: usize) -> u64 {
    // SAFETY: offset + 8 is inside the value.
    unsafe { std::ptr::read_unaligned((value as *const T as *const u8).add(offset) as *const u64) }
}

fn with_word<T: Copy>(value: &T, offset: usize, bits: u64) -> T {
    let mut copy = *value;
    // SAFETY: offset + 8 is inside the value; only a word that read as a
    // normal 64-bit number is written, so no flag, count or address is.
    unsafe { std::ptr::write_unaligned((&mut copy as *mut T as *mut u8).add(offset) as *mut u64, bits) };
    copy
}

/// The offsets of the words of `value` that read as a normal 64-bit number
/// and that its `Debug` shows as a field of its own: a flag with padding
/// beside it, or two 32-bit numbers, is not one.
pub fn number_words<T: Copy + Debug>(value: &T) -> Vec<usize> {
    let size = std::mem::size_of::<T>();
    (0..size.saturating_sub(7))
        .step_by(8)
        .filter(|&offset| f64::from_bits(word_at(value, offset)).is_normal() && named(value, offset))
        .collect()
}

fn named<T: Copy + Debug>(value: &T, offset: usize) -> bool {
    format!("{:?}", with_word(value, offset, SENTINEL.to_bits())).contains(&format!("{SENTINEL:?}"))
}

const SENTINEL: f64 = 1.2345678912345678e-77;

/// The name of the field a word is in, read from the value's `Debug` with a
/// sentinel written into the word.
pub fn name_of_word<T: Copy + Debug>(value: &T, offset: usize) -> String {
    let marked = with_word(value, offset, SENTINEL.to_bits());
    let text = format!("{marked:?}");
    let needle = format!("{SENTINEL:?}");
    let Some(at) = text.find(&needle) else {
        return format!("word at byte {offset}");
    };
    let before = &text[..at];
    let mut name = String::new();
    if let Some(colon) = before.rfind(": ") {
        let head = &before[..colon];
        let start = head.rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).map_or(0, |i| i + 1);
        name = head[start..].to_string();
        let between = &before[colon + 2..];
        if let Some(open) = between.rfind('[') {
            let index = between[open..].matches(',').count();
            name = format!("{name}[{index}]");
        }
    }
    if name.is_empty() { format!("word at byte {offset}") } else { name }
}

fn same<T: Debug>(a: &T, b: &T) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

fn frame_with_proof() -> LastStateFrame64 {
    let mut frame = LastStateFrame64 { t: 0.0, pos: [0.0; 3], vel: [0.0; 3], force_torque: 0.0, residual: 0.0, flags: 0, proof: [0xA5; 16] };
    frame.proof[0] = 0x5A;
    frame
}

/// One step, its panic caught.
fn try_step<C: Copy, S: Copy, L: Copy, R: Copy + Debug>(
    kit: &Kit<C, S, L, R>,
    card: &C,
    state: &mut S,
    frame: &mut LastStateFrame64,
    load: L,
    dt_s: f64,
) -> Result<Result<(), R>, String> {
    let step = kit.step;
    catch_unwind(AssertUnwindSafe(|| step(card, state, frame, load, dt_s))).map_err(|panic| {
        panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
    })
}

struct Out {
    found: Vec<Finding>,
    model: &'static str,
}

impl Out {
    fn contract(&mut self, rule: &'static str, detail: String) {
        self.found.push(Finding { model: self.model, rule, weight: Weight::Contract, detail });
    }
    fn note(&mut self, rule: &'static str, detail: String) {
        self.found.push(Finding { model: self.model, rule, weight: Weight::Note, detail });
    }
}

/// Every attack on one card. An empty list is a card that met every one.
pub fn findings<C, S, L, R>(kit: &Kit<C, S, L, R>) -> Vec<Finding>
where
    C: Copy + Debug,
    S: Copy + Debug,
    L: Copy + Debug,
    R: Copy + Debug,
{
    let mut out = Out { found: Vec::new(), model: kit.model };
    let base = match baseline(kit, &mut out) {
        Some(base) => base,
        None => return out.found,
    };
    constants(kit, &base, &mut out);
    normal_or_zero(kit, &base, &mut out);
    step_sizes(kit, &mut out);
    nothing_filled_in(kit, &mut out);
    ledger_runs(kit, &mut out);
    ledger_attacks(kit, &mut out);
    for finding in &mut out.found {
        if finding.weight == Weight::Contract && kit.departures.contains(&finding.rule) {
            finding.weight = Weight::Note;
            finding.detail = format!("{} (a declared departure)", finding.detail);
        }
    }
    out.found
}

/// The run of `steps` steps from the start, with every landed state.
struct Run<S> {
    states: Vec<S>,
    frames: Vec<[u8; 64]>,
    stop: Option<String>,
}

fn run<C: Copy, S: Copy + Debug, L: Copy, R: Copy + Debug>(kit: &Kit<C, S, L, R>, card: &C, start: S, dt_s: f64, steps: usize) -> Run<S> {
    run_under(kit, card, start, kit.load, dt_s, steps)
}

/// The run of `steps` steps from `start` under `load`.
fn run_under<C: Copy, S: Copy + Debug, L: Copy, R: Copy + Debug>(kit: &Kit<C, S, L, R>, card: &C, start: S, load: L, dt_s: f64, steps: usize) -> Run<S> {
    let mut state = start;
    let mut frame = frame_with_proof();
    let mut out = Run { states: Vec::new(), frames: Vec::new(), stop: None };
    for index in 0..steps {
        match try_step(kit, card, &mut state, &mut frame, load, dt_s) {
            Ok(Ok(())) => {
                out.states.push(state);
                out.frames.push(frame.to_bytes());
            }
            Ok(Err(reject)) => {
                out.stop = Some(format!("step {} refused as {reject:?}", index + 1));
                break;
            }
            Err(panic) => {
                out.stop = Some(format!("step {} panicked: {panic}", index + 1));
                break;
            }
        }
    }
    out
}

fn baseline<C, S, L, R>(kit: &Kit<C, S, L, R>, out: &mut Out) -> Option<Run<S>>
where
    C: Copy + Debug,
    S: Copy + Debug,
    L: Copy + Debug,
    R: Copy + Debug,
{
    if !(kit.is_valid)(&kit.card) {
        out.contract("fixed constants", "the specimen as built fails its own is_valid".to_string());
        return None;
    }
    let first = run(kit, &kit.card, kit.start, kit.dt_s, kit.steps);
    if let Some(stop) = &first.stop {
        out.note("harness", format!("the kit's own run of {} steps of {} s stopped: {stop}", kit.steps, kit.dt_s));
    }
    if first.states.is_empty() {
        out.note("harness", "no step of the kit's run landed; the attacks that need a landed step were not run".to_string());
        return None;
    }
    // Same input, same bits.
    let second = run(kit, &kit.card, kit.start, kit.dt_s, kit.steps);
    if !(first.states.len() == second.states.len()
        && first.states.iter().zip(&second.states).all(|(a, b)| bytes_of(a) == bytes_of(b))
        && first.frames == second.frames)
    {
        out.contract("same input, same bits", "the same run twice gave different states or frames".to_string());
    }
    // Flags and proof.
    for (index, frame) in first.frames.iter().enumerate() {
        let frame = LastStateFrame64::from_bytes(*frame);
        if frame.flags & !kit.flags != 0 {
            out.contract("flags", format!("step {} set flag bits {:#x} outside the declared {:#x}", index + 1, frame.flags, kit.flags));
            break;
        }
        if frame.proof != frame_with_proof().proof {
            out.contract("proof", format!("step {} wrote the frame's proof", index + 1));
            break;
        }
    }
    Some(first)
}

fn constants<C, S, L, R>(kit: &Kit<C, S, L, R>, base: &Run<S>, out: &mut Out)
where
    C: Copy + Debug,
    S: Copy + Debug,
    L: Copy + Debug,
    R: Copy + Debug,
{
    let words = number_words(&kit.card);
    let mut unread = Vec::new();
    for &offset in &words {
        let bits = word_at(&kit.card, offset);
        for moved in [bits + 1, bits - 1] {
            let card = with_word(&kit.card, offset, moved);
            if !(kit.is_valid)(&card) {
                continue;
            }
            let attempt = run(kit, &card, kit.start, kit.dt_s, base.states.len());
            if attempt.stop.is_some() && attempt.states.len() < base.states.len() {
                // The step refused the specimen.
                continue;
            }
            let changed = attempt.states.iter().zip(&base.states).any(|(a, b)| bytes_of(a) != bytes_of(b));
            let name = name_of_word(&kit.card, offset);
            if changed {
                out.contract(
                    "fixed constants",
                    format!("specimen field {name} ({:e}) moved by one unit in the last place passes is_valid and changes the landed state", f64::from_bits(bits)),
                );
            } else {
                unread.push(name);
            }
            break;
        }
    }
    if !unread.is_empty() {
        unread.dedup();
        let shown: Vec<_> = unread.iter().take(8).cloned().collect();
        out.note(
            "fixed constants",
            format!(
                "{} of {} specimen words pass is_valid when moved and change nothing in this run: {}{}",
                unread.len(),
                words.len(),
                shown.join(", "),
                if unread.len() > shown.len() { ", ..." } else { "" }
            ),
        );
    }
}

/// The offsets of the words of `value` that read as a subnormal 64-bit
/// number and that its `Debug` shows as a field of its own: a count, a flag
/// or padding whose bits read as one is not one.
pub fn subnormal_words<T: Copy + Debug>(value: &T) -> Vec<usize> {
    let size = std::mem::size_of::<T>();
    (0..size.saturating_sub(7))
        .step_by(8)
        .filter(|&offset| f64::from_bits(word_at(value, offset)).is_subnormal() && named(value, offset))
        .collect()
}

/// The smallest normal number and 2^-1000, the sizes a word is set to.
const TINY: [f64; 2] = [f64::MIN_POSITIVE, 9.332636185032189e-302];

/// No landed state holds a subnormal number, in the kit's run and in a run
/// with each word of the start and of the load made tiny; and no such run
/// stops on a state the card itself wrote.
fn normal_or_zero<C, S, L, R>(kit: &Kit<C, S, L, R>, base: &Run<S>, out: &mut Out)
where
    C: Copy + Debug,
    S: Copy + Debug,
    L: Copy + Debug,
    R: Copy + Debug,
{
    let steps = kit.steps.min(64);
    let mut runs: Vec<(String, Run<S>)> = Vec::new();
    for &offset in &number_words(&kit.load) {
        for tiny in TINY {
            let sign = word_at(&kit.load, offset) & 0x8000_0000_0000_0000;
            let load = with_word(&kit.load, offset, tiny.to_bits() | sign);
            let what = format!("load field {} at {:e}", name_of_word(&kit.load, offset), f64::from_bits(tiny.to_bits() | sign));
            runs.push((what, run_under(kit, &kit.card, kit.start, load, kit.dt_s, steps)));
        }
    }
    for &offset in &number_words(&kit.start) {
        for tiny in TINY {
            let sign = word_at(&kit.start, offset) & 0x8000_0000_0000_0000;
            let start = with_word(&kit.start, offset, tiny.to_bits() | sign);
            let what = format!("start field {} at {:e}", name_of_word(&kit.start, offset), f64::from_bits(tiny.to_bits() | sign));
            runs.push((what, run_under(kit, &kit.card, start, kit.load, kit.dt_s, steps)));
        }
    }
    let own = Run { states: base.states.clone(), frames: Vec::new(), stop: None };
    let mut subnormal = Vec::new();
    let mut stopped = Vec::new();
    for (what, ran) in std::iter::once(("the kit's own run".to_string(), own)).chain(runs) {
        if let Some((index, words)) = ran.states.iter().enumerate().find_map(|(index, state)| {
            let words = subnormal_words(state);
            (!words.is_empty()).then(|| (index, words.iter().map(|&offset| name_of_word(state, offset)).collect::<Vec<_>>()))
        }) {
            subnormal.push(format!("{what}: step {} landed {}", index + 1, words.join(", ")));
        }
        if let Some(stop) = &ran.stop {
            if !ran.states.is_empty() && stop.contains("Inconsistent") {
                stopped.push(format!("{what}: {stop}"));
            }
        }
    }
    if !subnormal.is_empty() {
        out.contract("normal or zero", format!("a landed state holds a subnormal number: {}", subnormal.join("; ")));
    }
    if !stopped.is_empty() {
        out.contract("normal or zero", format!("a run stopped on a state the card wrote: {}", stopped.join("; ")));
    }
}

fn refused_whole<C, S, L, R>(kit: &Kit<C, S, L, R>, state: S, load: L, dt_s: f64) -> Result<String, String>
where
    C: Copy,
    S: Copy + Debug,
    L: Copy,
    R: Copy + Debug,
{
    let mut after = state;
    let mut frame = frame_with_proof();
    let before = frame.to_bytes();
    match try_step(kit, &kit.card, &mut after, &mut frame, load, dt_s) {
        Ok(Ok(())) => Err("landed".to_string()),
        Ok(Err(reject)) => {
            if bytes_of(&after) != bytes_of(&state) || frame.to_bytes() != before {
                Err(format!("refused as {reject:?} but the state or the frame changed"))
            } else {
                Ok(format!("{reject:?}"))
            }
        }
        Err(panic) => Err(format!("panicked: {panic}")),
    }
}

fn step_sizes<C, S, L, R>(kit: &Kit<C, S, L, R>, out: &mut Out)
where
    C: Copy + Debug,
    S: Copy + Debug,
    L: Copy + Debug,
    R: Copy + Debug,
{
    match (kit.gate_dt_s, kit.floor_dt_s) {
        (Some(gate), Some(floor)) if gate.is_finite() && floor.is_finite() && floor > 0.0 && floor < gate => {}
        (Some(gate), Some(floor)) if gate == floor && gate > 0.0 => {
            out.note("step ceiling and floor", format!("the ceiling and the floor are one step, {gate} s"));
        }
        other => out.contract("step ceiling and floor", format!("the ceiling and the floor are not two finite positive steps, floor under ceiling: {other:?}")),
    }
    for dt_s in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, -1.0, -kit.dt_s] {
        match refused_whole(kit, kit.start, kit.load, dt_s) {
            Ok(name) => {
                let expected = if dt_s.is_finite() { "NonPositiveDt" } else { "NonFiniteInput" };
                if !name.starts_with(expected) {
                    out.note("refusal names", format!("a step of {dt_s} s is refused as {name}, not {expected}"));
                }
            }
            Err(what) => out.contract("refused with the state whole", format!("a step of {dt_s} s: {what}")),
        }
    }
}

fn nothing_filled_in<C, S, L, R>(kit: &Kit<C, S, L, R>, out: &mut Out)
where
    C: Copy + Debug,
    S: Copy + Debug,
    L: Copy + Debug,
    R: Copy + Debug,
{
    for &offset in &number_words(&kit.load) {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let load = with_word(&kit.load, offset, bad.to_bits());
            if let Err(what) = refused_whole(kit, kit.start, load, kit.dt_s) {
                out.contract("nothing filled in", format!("load field {} set to {bad}: {what}", name_of_word(&kit.load, offset)));
                break;
            }
        }
    }
    let mut ignored = Vec::new();
    for &offset in &number_words(&kit.start) {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let state = with_word(&kit.start, offset, bad.to_bits());
            match refused_whole(kit, state, kit.load, kit.dt_s) {
                Ok(_) => {}
                Err(what) if what == "landed" => {
                    ignored.push(format!("{} = {bad}", name_of_word(&kit.start, offset)));
                    break;
                }
                Err(what) => {
                    out.contract("nothing filled in", format!("state field {} set to {bad}: {what}", name_of_word(&kit.start, offset)));
                    break;
                }
            }
        }
    }
    if !ignored.is_empty() {
        out.note(
            "nothing filled in",
            format!("a state holding a non-finite number in a field the step writes over landed: {}", ignored.join(", ")),
        );
    }
}

fn ledger_runs<C, S, L, R>(kit: &Kit<C, S, L, R>, out: &mut Out)
where
    C: Copy + Debug,
    S: Copy + Debug,
    L: Copy + Debug,
    R: Copy + Debug,
{
    let mut sizes = vec![("working step", kit.dt_s)];
    if let Some(gate) = kit.gate_dt_s {
        sizes.push(("ceiling", gate));
    }
    if let Some(floor) = kit.floor_dt_s {
        sizes.push(("floor", floor));
    }
    for (what, dt_s) in sizes {
        let ran = run(kit, &kit.card, kit.start, dt_s, kit.steps);
        for (index, state) in ran.states.iter().enumerate() {
            let residual = (kit.residual)(state);
            if !(residual.is_finite() && residual >= 0.0 && residual <= kit.limit) {
                out.contract(
                    "ledger held at every step",
                    format!("at the {what} ({dt_s} s) step {} landed with residual {residual:e}, limit {:e}", index + 1, kit.limit),
                );
                break;
            }
        }
        if what != "working step" {
            if let Some(stop) = ran.stop {
                out.note("step ceiling and floor", format!("the kit's run at the {what} ({dt_s} s): {stop}"));
            }
        }
    }
}

/// A field moved, without booking it, from a state that landed: the next
/// step names the break or closes its ledger. A move that lands with the
/// residual over the limit, and a field no move of up to a thousand times
/// its size is caught on, are findings.
fn ledger_attacks<C, S, L, R>(kit: &Kit<C, S, L, R>, out: &mut Out)
where
    C: Copy + Debug,
    S: Copy + Debug,
    L: Copy + Debug,
    R: Copy + Debug,
{
    let mut landed = kit.start;
    let mut frame = frame_with_proof();
    if !matches!(try_step(kit, &kit.card, &mut landed, &mut frame, kit.load, kit.dt_s), Ok(Ok(()))) {
        return;
    }
    let scale = kit
        .ledger
        .iter()
        .chain(&kit.physical)
        .map(|(_, reach)| {
            let mut copy = landed;
            reach(&mut copy).abs()
        })
        .fold(1.0_f64, f64::max);
    let mut base_after = landed;
    let mut frame = frame_with_proof();
    let base_landed = matches!(try_step(kit, &kit.card, &mut base_after, &mut frame, kit.load, kit.dt_s), Ok(Ok(())));
    for (kind, fields) in [("ledger field", &kit.ledger), ("physical field", &kit.physical)] {
        for (name, reach) in fields.iter() {
            let mut caught: Option<(f64, String)> = None;
            let mut over = None;
            let mut unread = false;
            'kicks: for relative in [1e-12, 1e-9, 1e-6, 1e-3, 1e-1, 1.0, 1e3] {
                for sign in [1.0, -1.0] {
                    let mut moved = landed;
                    let value = reach(&mut moved);
                    let size = value.abs().max(scale * 1e-6);
                    *value += sign * relative * size;
                    let mut after = moved;
                    let mut frame = frame_with_proof();
                    match try_step(kit, &kit.card, &mut after, &mut frame, kit.load, kit.dt_s) {
                        Ok(Ok(())) => {
                            if relative == 1e3 && base_landed && bytes_of(&after) == bytes_of(&base_after) {
                                // The step wrote the field over and read nothing of it.
                                unread = true;
                                break 'kicks;
                            }
                            let residual = (kit.residual)(&after);
                            if !(residual <= kit.limit) && over.is_none() {
                                over = Some(format!("{name} moved by {:e} landed with residual {residual:e} over the limit {:e}", sign * relative * size, kit.limit));
                            }
                        }
                        Ok(Err(reject)) => {
                            if bytes_of(&after) != bytes_of(&moved) {
                                out.contract("refused with the state whole", format!("{kind} {name} moved: refused as {reject:?} with the state changed"));
                            }
                            caught = Some((relative, format!("{reject:?}")));
                            break 'kicks;
                        }
                        Err(panic) => {
                            out.contract("ledger attacked", format!("{kind} {name} moved: the step panicked: {panic}"));
                            break 'kicks;
                        }
                    }
                }
            }
            if let Some(detail) = over {
                out.contract("ledger attacked", detail);
            }
            if unread {
                out.note("ledger attacked", format!("{kind} {name} is written over by the step and read by nothing: a move of it changes no landed number, so it carries no ledger"));
                continue;
            }
            match caught {
                None => out.contract("ledger attacked", format!("{kind} {name} moved by up to a thousand times its size, unbooked, and every step landed with its ledger closed")),
                Some((relative, name_given)) => {
                    let range_name = ["Outside", "Beyond", "Limit", "Below", "Above"].iter().any(|word| name_given.contains(word));
                    if relative <= 1e-9 && range_name {
                        out.note("ledger attacked", format!("{kind} {name} moved by {relative:e} of its size is refused under a range name, {name_given}: a ledger-sized break is named as a range"));
                    }
                }
            }
        }
    }
}
