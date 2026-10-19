//! One entry for each field solver, and what the solver tools say of it.
//!
//! An entry is everything that belongs to one solver: its name, its sentence,
//! its input fields with their bounds, its output fields, its refusals, the
//! check of a limit that spans more than one field, and the call. Request
//! parsing, the description and the schemas all read the same field lists.
//! Serving another solver is one more entry in [`ALL`]. Nothing else in this
//! file or in `solution.rs` names a solver.

use serde_json::{Map, Value, json};

use crate::DoorError;
use crate::view::View;

/// Every solver the engine serves, in the order they are listed. The stand-in
/// is compiled for tests only and is never served.
pub const ALL: &[&Solver] = &[
    &crate::solver_atom_dft::ATOM_DFT,
    &crate::solver_ablator_slab::ABLATOR_SLAB,
    &crate::solver_journal_bearing_sheffield::JOURNAL_BEARING_SHEFFIELD,
    &crate::solver_propeller_apc_10x7e::PROPELLER_APC_10X7E,
    &crate::solver_modulus_bam_s355::MODULUS_BAM_S355,
    &crate::solver_seawater_column::SEAWATER_COLUMN,
    &crate::solver_argon_pair_gas::ARGON_PAIR_GAS,
    &crate::solver_fusion_pb11_lhd::FUSION_PB11_LHD,
    &crate::solver_fusion_pb11_thermal::FUSION_PB11_THERMAL,
    &crate::solver_step_flow_armaly::STEP_FLOW_ARMALY,
    &crate::solver_culture_trichoderma_ambr250::CULTURE_TRICHODERMA_AMBR250,
    &crate::solver_tokamak_dt_beam_target::TOKAMAK_DT_BEAM_TARGET,
    &crate::solver_pipe_flow_superpipe::PIPE_FLOW_SUPERPIPE,
    &crate::solver_fuel_rod_triga_jsi::FUEL_ROD_TRIGA_JSI,
    &crate::solver_culture_trichoderma_atp::CULTURE_TRICHODERMA_ATP,
    &crate::solver_junction_nist_b7_s2::JUNCTION_NIST_B7_S2,
    &crate::solver_water_hammer_oil_line::WATER_HAMMER_OIL_LINE,
    &crate::solver_orifice_air_chamber::ORIFICE_AIR_CHAMBER,
    &crate::solver_pneumatic_hose_resonance::PNEUMATIC_HOSE_RESONANCE,
    &crate::solver_blast_wave_tnt_air::BLAST_WAVE_TNT_AIR,
    &crate::solver_stress_plate_hole_s235::STRESS_PLATE_HOLE_S235,
    &crate::solver_stress_plate_hole_s235_elastic::STRESS_PLATE_HOLE_S235_ELASTIC,
    &crate::solver_icf_hotspot_burn::ICF_HOTSPOT_BURN,
    &crate::solver_ln2_line_dlr_fttf::LN2_LINE_DLR_FTTF,
    &crate::solver_plasma_sheath_ram_c2::PLASMA_SHEATH_RAM_C2,
    #[cfg(test)]
    &crate::solver_probe::STAND_IN,
];

/// How a refusal reaches the caller.
const AT_THE_DOOR: &str = "422, no seal";
const SEALED_REFUSAL: &str = "200, outcome refused, sealed";

/// One solver, as the gateway serves it.
pub struct Solver {
    pub name: &'static str,
    /// One sentence, for the list and the description.
    pub summary: &'static str,
    /// Every key `parameters` takes, in the order they are described. A key
    /// outside this list is refused in a sentence that names the key, and so
    /// is a listed key that is missing. Neither sentence carries a refusal
    /// name: a name is for a value outside what the solver takes.
    pub inputs: &'static [Input],
    /// The sentence that follows "is not an input of this solver.": what the
    /// solver keeps to itself.
    pub kept_inside: &'static str,
    /// Every number `result` publishes, in the order it publishes them.
    pub result: &'static [Output],
    /// Every number `invariants` publishes.
    pub invariants: &'static [Output],
    /// What `iterations` counts, for the description.
    pub iterations: &'static str,
    /// What `field_sha256` covers, for the description.
    pub field: fn() -> String,
    /// What the solver returns when it does not refuse, as in "a ground
    /// state". It is written into the detail of a sealed refusal.
    pub answer: &'static str,
    /// Every refusal the solver names.
    pub refusals: &'static [Refusal],
    /// The refusals this entry names itself, for a value of an input that
    /// never reaches the solver: a word that is not one of an input's own, a
    /// series longer than the solver takes. Each is answered at the door,
    /// with no seal, and the description lists them after the solver's.
    pub door_refusals: &'static [Refusal],
    /// The parts of the description that are written by hand for this solver.
    pub sections: fn() -> Sections,
    /// The validation record of a solver that is compared with measurements
    /// of a real object: the object, the data, the pass line, the result and
    /// the limits, in the five parts a card's description gives. `None` for
    /// a solver that is compared with a reference calculation only: its
    /// description has no such record, and `reference` says what it is
    /// compared with.
    pub validation: Option<fn() -> [String; 5]>,
    /// The `parameters` of a request that runs unchanged.
    pub example: fn() -> Value,
    /// More requests that run unchanged, each under its own key of the
    /// description.
    pub further_examples: &'static [FurtherExample],
    /// The keys of the description that are given only when asked for, for a
    /// solver whose whole description is too long to read at one go. The
    /// description then says in a sentence what each holds, and
    /// `describe_solver` with `part` set to the key returns it. Empty for a
    /// solver described whole.
    pub parts: &'static [Part],
    /// A limit the request itself shows and no single field's bounds state,
    /// such as one field's limit that depends on another. It runs after every
    /// field has passed its own bounds and before a physics permit is taken.
    /// The error is the whole sentence the caller reads.
    pub check: fn(&Given) -> Result<(), String>,
    /// The call. It makes the solver's own request from the parameters, runs
    /// the solver, and names what came back. It runs holding a physics permit.
    pub solve: fn(&Given) -> Result<Solved, String>,
}

/// One more request that runs unchanged.
pub struct FurtherExample {
    /// The key of the description it is published under. It starts with
    /// `example_solve_`.
    pub key: &'static str,
    pub parameters: fn() -> Value,
}

/// One key of a solver's description that is given only when asked for.
pub struct Part {
    /// A key of the whole description. Not `solver`, `summary` or
    /// `engine_version`, which every description holds.
    pub key: &'static str,
    /// What the description says in its place.
    pub holds: &'static str,
}

/// One key of `parameters`.
pub struct Input {
    pub name: &'static str,
    pub kind: Kind,
    /// The refusal a value outside the bound takes: one of the solver's own,
    /// or one of `door_refusals`. It is written in round brackets at the end
    /// of the sentence. `None` for a flag alone, which has no value outside
    /// what the solver takes.
    pub refusal: Option<&'static str>,
    /// One sentence about the bound, for the description.
    pub bound: &'static str,
    /// `Some((switch, word))` when this key is an input only for one choice
    /// of a `Word` input listed before it: it is required when `switch` holds
    /// `word`, and refused, in a sentence that names the key and carries no
    /// refusal name, when `switch` holds anything else. The
    /// switch may itself go with a choice of an earlier word; when the
    /// request does not take the switch, it does not take this key either.
    /// `None` for a key every request gives.
    pub only_when: Option<(&'static str, &'static str)>,
    /// `true` where the solver asks this field's limit in an order, or
    /// against another field, that a list read field by field cannot keep.
    /// The value is then held to its kind as it is read, and the entry's
    /// `check` holds it to the bounds published here, together with the
    /// limits that span fields, in the solver's order and before a physics
    /// permit is taken.
    pub asked_by_check: bool,
}

/// What a key of `parameters` holds, and its bounds. Both ends of a range are
/// inside it.
pub enum Kind {
    /// A whole number written as digits. `spoken` names it in a refusal.
    Count {
        minimum: u64,
        maximum: u64,
        spoken: &'static str,
    },
    /// One number.
    Number {
        minimum: f64,
        maximum: f64,
        unit: &'static str,
    },
    /// One number above `above`, which is itself refused, and at most
    /// `maximum`: a range whose lower end is exclusive.
    NumberAbove {
        above: f64,
        maximum: f64,
        unit: &'static str,
    },
    /// One number from `minimum` to `maximum`, an end exclusive where its
    /// flag says so: that end itself is refused.
    NumberWithin {
        minimum: f64,
        minimum_exclusive: bool,
        maximum: f64,
        maximum_exclusive: bool,
        unit: &'static str,
    },
    /// One of the listed strings.
    Word { allowed: &'static [&'static str] },
    /// `true` or `false`.
    Flag,
    /// An array of at least `fewest` and at most `most` numbers, each inside
    /// the range.
    Numbers {
        fewest: usize,
        most: usize,
        minimum: f64,
        maximum: f64,
        unit: &'static str,
    },
    /// An array of at least `fewest` and at most `most` numbers, each inside
    /// the range, either end of which may be exclusive: an exclusive end is
    /// itself refused. `describe_solver` writes `minimum_exclusive: true` or
    /// `maximum_exclusive: true` beside the end, and the input schema
    /// `exclusiveMinimum` or `exclusiveMaximum`.
    NumbersWithin {
        fewest: usize,
        most: usize,
        minimum: f64,
        minimum_exclusive: bool,
        maximum: f64,
        maximum_exclusive: bool,
        unit: &'static str,
    },
}

/// The shape of one published value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    /// One number.
    Number,
    /// An array of numbers.
    Numbers,
    /// An array of arrays of numbers.
    Lists,
}

/// One name `result` or `invariants` publishes. Each has a `_hex` sibling.
pub struct Output {
    /// The published name. A family of names holds one placeholder in
    /// braces, as in `eigenvalue_{orbital}_ha`.
    pub name: &'static str,
    pub unit: &'static str,
    pub shape: Shape,
    /// The labels a family takes in place of its placeholder. Empty for a
    /// single name. A single name is in every solution. A family name is in
    /// a solution when the solver returns it.
    pub each: &'static [&'static str],
    /// One sentence for the description, or nothing.
    pub note: &'static str,
    /// The `Flag` input that asks for this value. It is in a solution when
    /// the request holds `true` there and in no solution that holds `false`.
    /// `None` for a value every solution holds.
    pub asked_by: Option<&'static str>,
}

/// One named refusal.
pub struct Refusal {
    /// The variant name of the solver's refusal.
    pub name: &'static str,
    /// The sentence above the variant in the solver's source.
    pub meaning: &'static str,
    /// `true` when the refusal is returned as a sealed solution with outcome
    /// `refused`. `false` when the request itself shows the limit: the answer
    /// is then 422 with no seal.
    pub sealed: bool,
}

/// The parts of a description that are written by hand for one solver.
pub struct Sections {
    /// The constants the caller cannot set.
    pub locked: Value,
    /// Each invariant: its rule, its tolerance, when it is compared, and the
    /// refusal that names a break.
    pub invariants: Value,
    /// When the solver accepts its own result.
    pub convergence: Value,
    /// The published source the solver is compared with.
    pub reference: Value,
}

/// One value of `parameters`, read and inside its bounds.
pub enum GivenValue {
    Count(u64),
    Number(f64),
    Word(String),
    Flag(bool),
    Numbers(Vec<f64>),
}

/// The values of `parameters`: one for every input of the solver that this
/// request takes, each read and inside its own bounds. Nothing is in it that
/// the request did not give.
pub struct Given {
    values: Vec<(&'static str, GivenValue)>,
}

impl Given {
    pub fn new(values: Vec<(&'static str, GivenValue)>) -> Self {
        Self { values }
    }

    /// One more value, read and held to its kind.
    pub fn hold(&mut self, name: &'static str, value: GivenValue) {
        self.values.push((name, value));
    }

    fn value(&self, name: &str) -> Option<&GivenValue> {
        self.values
            .iter()
            .find(|(given, _)| *given == name)
            .map(|(_, value)| value)
    }

    pub fn count(&self, name: &str) -> Result<u64, String> {
        match self.value(name) {
            Some(GivenValue::Count(count)) => Ok(*count),
            _ => Err(not_an_input(name, "whole number")),
        }
    }

    pub fn number(&self, name: &str) -> Result<f64, String> {
        match self.value(name) {
            Some(GivenValue::Number(number)) => Ok(*number),
            _ => Err(not_an_input(name, "number")),
        }
    }

    pub fn word(&self, name: &str) -> Result<&str, String> {
        match self.value(name) {
            Some(GivenValue::Word(word)) => Ok(word),
            _ => Err(not_an_input(name, "string")),
        }
    }

    /// The word a `Word` input holds, or nothing when this request does not
    /// take that input: an input that goes with a choice the request did not
    /// make.
    pub fn word_if_given(&self, name: &str) -> Option<&str> {
        match self.value(name) {
            Some(GivenValue::Word(word)) => Some(word),
            _ => None,
        }
    }

    pub fn flag(&self, name: &str) -> Result<bool, String> {
        match self.value(name) {
            Some(GivenValue::Flag(flag)) => Ok(*flag),
            _ => Err(not_an_input(name, "true or false")),
        }
    }

    pub fn numbers(&self, name: &str) -> Result<&[f64], String> {
        match self.value(name) {
            Some(GivenValue::Numbers(numbers)) => Ok(numbers),
            _ => Err(not_an_input(name, "array of numbers")),
        }
    }
}

/// An entry asked for a field its own list does not hold in that kind.
fn not_an_input(name: &str, kind: &str) -> String {
    format!(
        "Physical Violation: the solver's entry reads '{name}' as a {kind}, and its field list has no such input. Nothing is computed."
    )
}

/// One published value, as the solver returned it.
pub enum Published {
    Number(f64),
    Numbers(Vec<f64>),
    Lists(Vec<Vec<f64>>),
}

impl Published {
    pub fn shape(&self) -> Shape {
        match self {
            Published::Number(_) => Shape::Number,
            Published::Numbers(_) => Shape::Numbers,
            Published::Lists(_) => Shape::Lists,
        }
    }
}

/// What a solver returned, in the words every solver shares.
pub enum Solved {
    Answer(Answer),
    Refused(Refused),
}

pub struct Answer {
    pub iterations: u64,
    /// Each name of `result` with its value, in the order it is published.
    pub result: Vec<(String, Published)>,
    /// Each name of `invariants` with its value.
    pub invariants: Vec<(String, Published)>,
    /// The field `field_sha256` covers.
    pub field: Vec<f64>,
}

pub struct Refused {
    /// The variant name, as it is in the solver's list of refusals.
    pub name: &'static str,
    /// The solver's own sentence for this refusal, with no full stop.
    pub sentence: String,
    /// The count the refusal carries, when it carries one.
    pub iterations: Option<u64>,
}

impl Output {
    /// For a family, the text before its placeholder and the text after it.
    pub(crate) fn family_ends(&self) -> Option<(&'static str, &'static str)> {
        if self.each.is_empty() {
            return None;
        }
        let (before, rest) = self.name.split_once('{')?;
        let (_, after) = rest.split_once('}')?;
        Some((before, after))
    }

    /// `true` when `base` is this name, or a member of this family.
    fn names(&self, base: &str) -> bool {
        match self.family_ends() {
            None => self.each.is_empty() && self.name == base,
            Some((before, after)) => base
                .strip_prefix(before)
                .and_then(|rest| rest.strip_suffix(after))
                .is_some_and(|label| self.each.contains(&label)),
        }
    }
}

impl Solver {
    /// The outputs of `result` or of `invariants`.
    pub fn outputs(&self, section: &str) -> &'static [Output] {
        match section {
            "result" => self.result,
            "invariants" => self.invariants,
            _ => &[],
        }
    }

    /// The shape of `base` when this solver publishes it under `section`.
    pub fn output_shape(&self, section: &str, base: &str) -> Option<Shape> {
        self.outputs(section)
            .iter()
            .find(|output| output.names(base))
            .map(|output| output.shape)
    }

    /// The shape of `key` when it is a name this solver publishes under
    /// `section`, or the `_hex` sibling of one.
    pub fn published_shape(&self, section: &str, key: &str) -> Option<Shape> {
        let base = match key.strip_suffix("_hex") {
            Some(base) if !base.is_empty() => base,
            _ => key,
        };
        self.output_shape(section, base)
    }

    pub fn refusal(&self, name: &str) -> Option<&'static Refusal> {
        self.refusals.iter().find(|refusal| refusal.name == name)
    }

    /// Every refusal the description lists: the solver's, then the entry's own.
    pub fn listed_refusals(&self) -> impl Iterator<Item = &'static Refusal> {
        self.refusals.iter().chain(self.door_refusals)
    }
}

pub fn engine_version() -> String {
    format!("genesis_solvers_v{}", genesis_solvers::VERSION)
}

pub fn find(name: &str) -> Option<&'static Solver> {
    ALL.iter().copied().find(|solver| solver.name == name)
}

pub fn is_known(name: &str) -> bool {
    find(name).is_some()
}

pub fn list_value() -> Value {
    list_value_in(&View::full())
}

pub fn list_value_in(view: &View) -> Value {
    let listed: Vec<Value> = view
        .solvers()
        .into_iter()
        .map(|solver| {
            let mut row = Map::new();
            row.insert("solver".to_string(), json!(solver.name));
            // how the model did against its own measurements, in one word, before its sentence
            if let Some((word, _)) = crate::find_model::verdict_of_solver(solver.name) {
                row.insert("against_measurement".to_string(), json!(word));
            }
            crate::succession::into(&mut row, view, solver.name);
            row.insert("summary".to_string(), json!(crate::spec_sheet::summary_of(solver.name, solver.summary)));
            Value::Object(row)
        })
        .collect();
    json!({ "count": listed.len(), "solvers": listed })
}

/// A range as a refusal writes it, in words: `250 to 681.96 K`. A number
/// with no unit, or the unit `1` of a pure number, is written bare.
pub fn range_words(minimum: f64, maximum: f64, unit: &str) -> String {
    if unit.is_empty() || unit == "1" {
        format!("{minimum} to {maximum}")
    } else {
        format!("{minimum} to {maximum} {unit}")
    }
}

/// A range with either end exclusive, as a refusal writes it, in words:
/// `above 0 and below 50289.2338551663 Hz`; with both ends inclusive, as
/// [`range_words`] writes it.
pub fn range_within_words(minimum: f64, minimum_exclusive: bool, maximum: f64, maximum_exclusive: bool, unit: &str) -> String {
    if !minimum_exclusive && !maximum_exclusive {
        return range_words(minimum, maximum, unit);
    }
    let low = if minimum_exclusive { format!("above {minimum}") } else { format!("at least {minimum}") };
    let high = if maximum_exclusive { format!("below {maximum}") } else { format!("at most {maximum}") };
    if unit.is_empty() || unit == "1" { format!("{low} and {high}") } else { format!("{low} and {high} {unit}") }
}

/// Whether a number lies inside a range whose ends may be exclusive.
pub fn within(number: f64, minimum: f64, minimum_exclusive: bool, maximum: f64, maximum_exclusive: bool) -> bool {
    let above = if minimum_exclusive { number > minimum } else { number >= minimum };
    let below = if maximum_exclusive { number < maximum } else { number <= maximum };
    above && below
}

/// One input, as `describe_solver` shows it.
pub(crate) fn input_row(input: &Input) -> Value {
    let mut row = Map::new();
    row.insert("name".to_string(), json!(input.name));
    match &input.kind {
        Kind::Count {
            minimum, maximum, ..
        } => {
            row.insert("type".to_string(), json!("integer"));
            row.insert("minimum".to_string(), json!(minimum));
            row.insert("maximum".to_string(), json!(maximum));
        }
        Kind::Number {
            minimum,
            maximum,
            unit,
        } => {
            row.insert("type".to_string(), json!("number"));
            row.insert("minimum".to_string(), json!(minimum));
            row.insert("maximum".to_string(), json!(maximum));
            row.insert("unit".to_string(), json!(unit));
        }
        Kind::NumberAbove {
            above,
            maximum,
            unit,
        } => {
            row.insert("type".to_string(), json!("number"));
            row.insert("minimum".to_string(), json!(above));
            row.insert("minimum_exclusive".to_string(), json!(true));
            row.insert("maximum".to_string(), json!(maximum));
            row.insert("unit".to_string(), json!(unit));
        }
        Kind::NumberWithin {
            minimum,
            minimum_exclusive,
            maximum,
            maximum_exclusive,
            unit,
        } => {
            row.insert("type".to_string(), json!("number"));
            row.insert("minimum".to_string(), json!(minimum));
            if *minimum_exclusive {
                row.insert("minimum_exclusive".to_string(), json!(true));
            }
            row.insert("maximum".to_string(), json!(maximum));
            if *maximum_exclusive {
                row.insert("maximum_exclusive".to_string(), json!(true));
            }
            row.insert("unit".to_string(), json!(unit));
        }
        Kind::Word { allowed } => {
            row.insert("type".to_string(), json!("string"));
            row.insert("allowed".to_string(), json!(allowed));
        }
        Kind::Flag => {
            row.insert("type".to_string(), json!("true or false"));
        }
        Kind::Numbers {
            fewest,
            most,
            minimum,
            maximum,
            unit,
        } => {
            row.insert("type".to_string(), json!("array of numbers"));
            if *fewest > 0 {
                row.insert("fewest".to_string(), json!(fewest));
            }
            row.insert("most".to_string(), json!(most));
            row.insert("minimum".to_string(), json!(minimum));
            row.insert("maximum".to_string(), json!(maximum));
            row.insert("unit".to_string(), json!(unit));
        }
        Kind::NumbersWithin {
            fewest,
            most,
            minimum,
            minimum_exclusive,
            maximum,
            maximum_exclusive,
            unit,
        } => {
            row.insert("type".to_string(), json!("array of numbers"));
            if *fewest > 0 {
                row.insert("fewest".to_string(), json!(fewest));
            }
            row.insert("most".to_string(), json!(most));
            row.insert("minimum".to_string(), json!(minimum));
            if *minimum_exclusive {
                row.insert("minimum_exclusive".to_string(), json!(true));
            }
            row.insert("maximum".to_string(), json!(maximum));
            if *maximum_exclusive {
                row.insert("maximum_exclusive".to_string(), json!(true));
            }
            row.insert("unit".to_string(), json!(unit));
        }
    }
    if let Some(refusal) = input.refusal {
        row.insert("refusal".to_string(), json!(refusal));
    }
    if let Some((switch, word)) = input.only_when {
        row.insert(
            "given_only_when".to_string(),
            json!(format!("'{switch}' is \"{word}\"")),
        );
    }
    row.insert("bound".to_string(), json!(input.bound));
    Value::Object(row)
}

/// One output, as `describe_solver` shows it.
pub(crate) fn output_row(output: &Output) -> Value {
    let mut row = Map::new();
    row.insert("name".to_string(), json!(output.name));
    row.insert("unit".to_string(), json!(output.unit));
    match output.shape {
        Shape::Number => {}
        Shape::Numbers => {
            row.insert("shape".to_string(), json!("array of numbers"));
        }
        Shape::Lists => {
            row.insert("shape".to_string(), json!("array of arrays of numbers"));
        }
    }
    if let Some(flag) = output.asked_by {
        row.insert(
            "returned_only_when".to_string(),
            json!(format!("'{flag}' is true")),
        );
    }
    if !output.note.is_empty() {
        row.insert("note".to_string(), json!(output.note));
    }
    Value::Object(row)
}

const HEX_OF_A_NUMBER: &str = "Each number has a `{field}_hex` sibling: 16 lowercase hex digits of `f64::to_bits`. The hex is the exact value.";
const HEX_OF_AN_ARRAY: &str = " An array has a `{field}_hex` sibling of the same shape, one hex for each number, and the proof covers them in the order given.";

/// How a description says what it has left out.
const MORE_HOW: &str = "describe_solver with 'solver' and 'part' set to a name below returns that part alone.";

/// The description a caller reads first: the whole description, less the
/// parts the solver's entry keeps for asking, with a sentence for each under
/// `more`. For a solver that keeps no part it is the whole description.
pub fn describe_value(name: &str) -> Option<Value> {
    let solver = find(name)?;
    let whole = describe_whole(name)?;
    if solver.parts.is_empty() {
        return Some(whole);
    }
    let Value::Object(whole) = whole else {
        unreachable!("a description is an object");
    };
    let mut described = Map::new();
    for (key, value) in whole {
        if !solver.parts.iter().any(|part| part.key == key) {
            described.insert(key, value);
        }
    }
    let mut more = Map::new();
    more.insert("how".to_string(), json!(MORE_HOW));
    for part in solver.parts {
        more.insert(part.key.to_string(), json!(part.holds));
    }
    described.insert("more".to_string(), Value::Object(more));
    Some(Value::Object(described))
}

/// One part of a description, alone: any key of the whole description but
/// the three every description holds.
pub fn describe_part(name: &str, part: &str) -> Option<Value> {
    let whole = describe_whole(name)?;
    let value = part_names(&whole)
        .contains(&part)
        .then(|| whole[part].clone())?;
    let mut described = Map::new();
    described.insert("solver".to_string(), json!(name));
    described.insert("part".to_string(), json!(part));
    described.insert(part.to_string(), value);
    Some(Value::Object(described))
}

/// The keys of a whole description that can be asked for as a part.
fn part_names(whole: &Value) -> Vec<&str> {
    whole
        .as_object()
        .map(|object| {
            object
                .keys()
                .map(String::as_str)
                .filter(|key| !["solver", "summary", "engine_version"].contains(key))
                .collect()
        })
        .unwrap_or_default()
}

/// Everything the gateway says of one solver.
pub fn describe_whole(name: &str) -> Option<Value> {
    let solver = find(name)?;
    let sections = (solver.sections)();
    let has_an_array = solver
        .result
        .iter()
        .chain(solver.invariants)
        .any(|output| output.shape != Shape::Number);
    let hex = if has_an_array {
        format!("{HEX_OF_A_NUMBER}{HEX_OF_AN_ARRAY}")
    } else {
        HEX_OF_A_NUMBER.to_string()
    };
    let mut described = Map::new();
    described.insert("solver".to_string(), json!(solver.name));
    described.insert("summary".to_string(), json!(crate::spec_sheet::summary_of(solver.name, solver.summary)));
    described.insert("engine_version".to_string(), json!(engine_version()));
    if let Some(validation) = solver.validation {
        let mut record = Map::new();
        for (part, text) in crate::descriptor::VALIDATION_PARTS.iter().zip(validation()) {
            record.insert((*part).to_string(), Value::String(text));
        }
        crate::descriptor::calibration_into(&mut record, solver.name);
        described.insert("validation".to_string(), Value::Object(record));
    }
    described.insert(
        "inputs".to_string(),
        json!(solver.inputs.iter().map(input_row).collect::<Vec<_>>()),
    );
    described.insert("number_rule".to_string(), json!(SOLVER_NUMBER_RULE));
    described.insert("locked".to_string(), sections.locked);
    described.insert(
        "returns".to_string(),
        json!({
            "outcome": "`converged` or `refused`. Both are HTTP 200.",
            "iterations": solver.iterations,
            "result": solver.result.iter().map(output_row).collect::<Vec<_>>(),
            "invariants": solver.invariants.iter().map(output_row).collect::<Vec<_>>(),
            "hex": hex,
            "field_sha256": (solver.field)(),
            "on_refusal": "`result`, `invariants` and `field_sha256` are null. `reject` names the refusal and `solution_seal` binds it.",
        }),
    );
    described.insert("invariants".to_string(), sections.invariants);
    described.insert("convergence".to_string(), sections.convergence);
    // The open connection's count of this solver in a plan of solves: the
    // tool descriptions state the rule once and point here.
    described.insert("open_connection".to_string(), json!(crate::open_plan::said_of_solver(solver.name)));
    described.insert(
        "refusals".to_string(),
        json!(solver.listed_refusals().map(|refusal| json!({
            "name": refusal.name,
            "meaning": refusal.meaning,
            "reported_as": if refusal.sealed { SEALED_REFUSAL } else { AT_THE_DOOR },
        })).collect::<Vec<_>>()),
    );
    described.insert("reference".to_string(), sections.reference);
    described.insert("example_solve".to_string(), example_solve(solver));
    for further in solver.further_examples {
        described.insert(
            further.key.to_string(),
            json!({ "solver": solver.name, "parameters": (further.parameters)() }),
        );
    }
    if let Some(refused) = example_refused(solver.name) {
        described.insert(EXAMPLE_REFUSED.to_string(), refused);
    }
    // The couplings into this solver: a card's sealed run handed in through
    // `from`, as describe_card lists a card's.
    let couplings = crate::couplings::table_into_solver(&crate::view::View::full(), solver.name);
    if couplings.as_array().is_some_and(|list| !list.is_empty()) {
        described.insert("couplings".to_string(), couplings);
    }
    Some(Value::Object(described))
}

/// The key of a worked request a solver turns away by name before computing,
/// as a card's description keys its own.
pub(crate) const EXAMPLE_REFUSED: &str = crate::descriptor::EXAMPLE_REFUSED;

/// A worked request a solver turns away by name before computing, with the
/// one sentence it is answered with: an error, not a result, and nothing
/// sealed. The hot spot's is NIF's N221204 as the registered comparison made
/// it into a question.
pub(crate) fn example_refused(solver: &str) -> Option<Value> {
    (solver == "icf_hotspot_burn").then(crate::solver_icf_hotspot_burn::example_refused)
}

/// What the full form of a solver's description has that the short form leaves out.
const FULL_FORM_HAS: &str = "each input's bound in words, what each returned name holds and its unit, the invariants and their tolerances, each refusal's meaning, the validation record or the reference, and every worked example";

/// The short form of a solver's description: what an agent needs to write a
/// first request and to know what it is dealing with, in a page. Every part
/// is taken from the whole description, so no bound, name or number is
/// written twice; the evidence is the one `find_model` and `list_solvers`
/// give. An input keeps everything but its prose and its refusal name; what
/// is returned, the refusals and the worked examples are given by name, and
/// the first worked example also as a request.
pub fn describe_short(name: &str) -> Option<Value> {
    let solver = find(name)?;
    let whole = describe_whole(name)?;
    let names = |list: &Value| Value::Array(list.as_array().into_iter().flatten().map(|row| row["name"].clone()).collect());
    let inputs: Vec<Value> = whole["inputs"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            let mut short = row.as_object().cloned().unwrap_or_default();
            short.shift_remove("bound");
            short.shift_remove("refusal");
            Value::Object(short)
        })
        .collect();
    let mut short = Map::new();
    short.insert("solver".to_string(), whole["solver"].clone());
    short.insert(crate::descriptor::FORM.to_string(), json!("short"));
    short.insert("summary".to_string(), json!(crate::spec_sheet::short_of(name, whole["summary"].as_str().unwrap_or(""))));
    if let Some((word, phrase)) = crate::find_model::verdict_of_solver(name) {
        short.insert("against_measurement".to_string(), json!(word));
        short.insert("accuracy_against_measurement".to_string(), json!(phrase));
    }
    crate::descriptor::calibration_into(&mut short, name);
    short.insert("inputs".to_string(), Value::Array(inputs));
    short.insert(
        "returns".to_string(),
        json!({ "result": names(&whole["returns"]["result"]), "invariants": names(&whole["returns"]["invariants"]) }),
    );
    short.insert("refusals".to_string(), names(&whole["refusals"]));
    short.insert("examples".to_string(), json!(example_names(solver)));
    short.insert("example_solve".to_string(), whole["example_solve"].clone());
    // A worked refusal, where the solver gives one, as a card's short form
    // gives its own: the request, and the one sentence it is answered with.
    if let Some(refused) = whole.get(EXAMPLE_REFUSED) {
        short.insert(EXAMPLE_REFUSED.to_string(), refused.clone());
    }
    short.insert(
        "full_form".to_string(),
        json!(format!(
            "This is the short form: inputs are the keys of parameters, and solve takes a name under examples as example in place of parameters. describe_solver with form \"full\", or with no form, has {FULL_FORM_HAS}."
        )),
    );
    Some(Value::Object(short))
}

/// `{"solver": "<name>"}` to the description of that solver.
pub fn describe_request(request: &Value) -> Result<Value, DoorError> {
    describe_request_in(&View::full(), request)
}

pub fn describe_request_in(view: &View, request: &Value) -> Result<Value, DoorError> {
    let object = request.as_object().ok_or_else(|| {
        DoorError::Unprocessable("Physical Violation: the request must be a JSON object.".into())
    })?;
    let solver = match object.get("solver") {
        None => {
            return Err(DoorError::Unprocessable(
                "Missing required parameter: 'solver' (InputMissing).".to_string(),
            ));
        }
        Some(Value::String(solver)) => Some(solver.as_str())
            .filter(|solver| view.sees_solver(solver) && is_known(solver))
            .ok_or_else(|| DoorError::NotFound(crate::SOLVER_NOT_RECOGNIZED.to_string()))?,
        Some(_) => {
            return Err(DoorError::Unprocessable(
                "Physical Violation: 'solver' must be a string.".to_string(),
            ));
        }
    };
    let not_found = || DoorError::NotFound(crate::SOLVER_NOT_RECOGNIZED.to_string());
    // The short form is of the whole description; a part is asked for alone.
    if crate::descriptor::form_of(object)? == crate::descriptor::Form::Short {
        if object.contains_key("part") {
            return Err(DoorError::Unprocessable(
                "Physical Violation: 'form' \"short\" and 'part' each ask for less than the whole description; send one of them.".to_string(),
            ));
        }
        return describe_short(solver).map(|short| with_successor(view, solver, short)).ok_or_else(not_found);
    }
    match object.get("part") {
        None => describe_value(solver).map(|whole| with_successor(view, solver, whole)).ok_or_else(not_found),
        Some(Value::String(part)) => match describe_part(solver, part) {
            Some(described) => Ok(described),
            None => {
                let whole = describe_whole(solver).ok_or_else(not_found)?;
                let names: Vec<String> = part_names(&whole)
                    .iter()
                    .map(|name| format!("\"{name}\""))
                    .collect();
                Err(DoorError::Unprocessable(format!(
                    "Physical Violation: 'part' must be one of {}; this request has {}.",
                    names.join(", "),
                    Value::String(part.clone())
                )))
            }
        },
        Some(other) => Err(DoorError::Unprocessable(format!(
            "Physical Violation: 'part' must be a string; this request has {other}."
        ))),
    }
}

/// A description of a solver a newer one supersedes, with `superseded_by`
/// after the solver's name.
fn with_successor(view: &View, solver: &str, described: Value) -> Value {
    let Some(by) = crate::succession::superseded_by(view, solver) else { return described };
    let Value::Object(map) = described else { return described };
    let mut out = Map::new();
    for (key, value) in map {
        let here = key == "solver";
        out.insert(key, value);
        if here {
            out.insert(crate::succession::KEY.to_string(), json!(by));
        }
    }
    Value::Object(out)
}

/// The names of a solver's worked examples, as its description keys them.
pub fn example_names(solver: &Solver) -> Vec<&'static str> {
    let mut names = vec!["example_solve"];
    names.extend(solver.further_examples.iter().map(|further| further.key));
    names
}

/// The `parameters` of a solver's worked example, by the name its
/// description keys it under.
pub fn example_parameters(solver: &Solver, name: &str) -> Option<Value> {
    if name == "example_solve" {
        return Some((solver.example)());
    }
    solver
        .further_examples
        .iter()
        .find(|further| further.key == name)
        .map(|further| (further.parameters)())
}

/// A request that runs unchanged.
pub fn example_solve(solver: &Solver) -> Value {
    json!({ "solver": solver.name, "parameters": (solver.example)() })
}

/// What every number of a solve request must be, whatever range an input
/// publishes: the number rule of a card's request, without its frame slot,
/// since a solution has no frame.
pub(crate) const SOLVER_NUMBER_RULE: &str = "Every number of a solve request must be a finite, normal 64-bit number or zero, and a number written as non-zero must not read as zero. This rule is the one every card's request follows, and it stands beside each input's range and refuses some values inside it: a subnormal such as 5e-324 is refused wherever zero and larger numbers are accepted. A solution has no frame, so the card's rule that a number fit a 32-bit frame slot does not apply.";

pub fn list_schema() -> Map<String, Value> {
    object_map(json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false,
        "description": "No arguments. Returns every solver."
    }))
}

/// The name of a solver in a schema: the one solver when the view has one,
/// else a string, declared once. The catalogue is left to `list_solvers`,
/// which names every solver of the caller's view, and to `describe_solver`,
/// which gives a solver's inputs.
fn solver_name_schema(view: &View) -> Value {
    match view.solver_names().as_slice() {
        [one] => json!({ "const": one }),
        _ => json!({ "type": "string", "description": "The solver, as list_solvers names it." }),
    }
}

pub fn describe_schema_in(view: &View) -> Map<String, Value> {
    object_map(json!({
        "type": "object",
        "properties": {
            "solver": solver_name_schema(view),
            "part": {
                "type": "string",
                "description": "Leave this key out to get the solver's whole description. Some long descriptions name, under 'more', parts kept back for asking: set part to one of the names listed there for that part alone. Only a name listed under 'more' is a part."
            },
            "form": crate::descriptor::form_schema()
        },
        "required": ["solver"],
        "additionalProperties": false,
        "description": "Inputs and bounds, outputs, invariants and tolerances, refusals and meanings, the published reference, and a worked example for one solver."
    }))
}

/// One input, as a schema declares it.
fn input_schema(input: &Input) -> Value {
    match &input.kind {
        Kind::Count {
            minimum, maximum, ..
        } => json!({ "type": "integer", "minimum": minimum, "maximum": maximum }),
        Kind::Number {
            minimum, maximum, ..
        } => json!({ "type": "number", "minimum": minimum, "maximum": maximum }),
        Kind::NumberAbove { above, maximum, .. } => {
            json!({ "type": "number", "exclusiveMinimum": above, "maximum": maximum })
        }
        Kind::NumberWithin {
            minimum,
            minimum_exclusive,
            maximum,
            maximum_exclusive,
            ..
        } => {
            let mut schema = json!({ "type": "number" });
            schema[if *minimum_exclusive { "exclusiveMinimum" } else { "minimum" }] = json!(minimum);
            schema[if *maximum_exclusive { "exclusiveMaximum" } else { "maximum" }] = json!(maximum);
            schema
        }
        Kind::Word { allowed } => json!({ "type": "string", "enum": allowed }),
        Kind::Flag => json!({ "type": "boolean" }),
        Kind::Numbers {
            fewest,
            most,
            minimum,
            maximum,
            ..
        } => {
            let mut schema = json!({
                "type": "array",
                "items": { "type": "number", "minimum": minimum, "maximum": maximum },
            });
            if *fewest > 0 {
                schema["minItems"] = json!(fewest);
            }
            schema["maxItems"] = json!(most);
            schema
        }
        Kind::NumbersWithin {
            fewest,
            most,
            minimum,
            minimum_exclusive,
            maximum,
            maximum_exclusive,
            ..
        } => {
            let mut items = json!({ "type": "number" });
            items[if *minimum_exclusive { "exclusiveMinimum" } else { "minimum" }] = json!(minimum);
            items[if *maximum_exclusive { "exclusiveMaximum" } else { "maximum" }] = json!(maximum);
            let mut schema = json!({ "type": "array", "items": items });
            if *fewest > 0 {
                schema["minItems"] = json!(fewest);
            }
            schema["maxItems"] = json!(most);
            schema
        }
    }
}

/// The inputs every request gives, and then, for each choice of a switch in
/// the order the list first names it, the inputs given only with that choice.
fn inputs_by_choice(solver: &Solver) -> (Vec<&'static str>, Vec<ChoiceInputs>) {
    let mut always = Vec::new();
    let mut choices: Vec<ChoiceInputs> = Vec::new();
    for input in solver.inputs {
        match input.only_when {
            None => always.push(input.name),
            Some((switch, word)) => {
                match choices
                    .iter_mut()
                    .find(|choice| choice.switch == switch && choice.word == word)
                {
                    Some(choice) => choice.inputs.push(input.name),
                    None => choices.push(ChoiceInputs {
                        switch,
                        word,
                        inputs: vec![input.name],
                    }),
                }
            }
        }
    }
    (always, choices)
}

/// The inputs given only when `switch` holds `word`.
struct ChoiceInputs {
    switch: &'static str,
    word: &'static str,
    inputs: Vec<&'static str>,
}

/// In words: which inputs go with which choice.
fn choice_words(choices: &[ChoiceInputs]) -> String {
    let each: Vec<String> = choices
        .iter()
        .map(|choice| {
            format!(
                "when {} is {} also {}",
                choice.switch,
                choice.word,
                choice.inputs.join(", ")
            )
        })
        .collect();
    each.join(", and ")
}

/// `parameters` of one solver: every input, each required, and no other key.
/// An input that goes with one choice of a switch is declared, is not in the
/// required list, and the rule is given in words: a schema with a choice
/// between shapes is one that some clients do not read.
pub(crate) fn parameters_schema(solver: &Solver) -> Value {
    let mut properties = Map::new();
    for input in solver.inputs {
        properties.insert(input.name.to_string(), input_schema(input));
    }
    let (required, choices) = inputs_by_choice(solver);
    let mut schema = json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    });
    if !choices.is_empty() {
        schema["description"] = json!(format!(
            "Every key in the required list, and {}. A key that goes with the other choice is refused in a sentence that names the key, as InputNotWithChoice.",
            choice_words(&choices)
        ));
    }
    schema
}

/// `parameters` when the view has more than one solver: a plain object. The
/// inputs of each solver are left to `describe_solver`. A schema with a
/// choice between shapes is one that some clients do not read.
fn parameters_of_many() -> Value {
    json!({
        "type": "object",
        "description": format!(
            "The inputs of the solver named in 'solver', each a number, a whole number, a word, a flag or an array as describe_solver gives it, and no other key but signed. {}",
            SOLVER_NUMBER_RULE
        ),
    })
}

fn request_schema(view: &View) -> Value {
    let seen = view.solvers();
    let parameters = match seen.as_slice() {
        [one] => {
            // An instrument's signature over the other inputs may stand beside them.
            let mut parameters = parameters_schema(one);
            parameters["properties"][crate::signed_load::SIGNED] = crate::signed_load::schema();
            parameters
        }
        _ => parameters_of_many(),
    };
    json!({
        "type": "object",
        "properties": {
            "solver": solver_name_schema(view),
            "parameters": parameters,
            "example": {
                "type": "string",
                "description": "In place of parameters: the name of one of the solver's worked examples as describe_solver keys it (example_solve, or a key that begins example_solve_). The request is filled with that example's parameters, and the filled request is the one solved, sealed and returned as inputs."
            },
        },
        "required": ["solver"],
        "additionalProperties": false,
    })
}

pub fn solve_schema_in(view: &View) -> Map<String, Value> {
    let mut schema = object_map(request_schema(view));
    // A sealed card run handed in through a coupling into a solver; the
    // declaration is the same in every view.
    schema["properties"]
        .as_object_mut()
        .expect("properties")
        .insert(crate::chain::FROM.to_string(), crate::chain::solve_from_schema(false));
    schema.insert(
        "description".to_string(),
        Value::String("One sealed solution.".to_string()),
    );
    schema
}

pub fn solution_schema_in(_view: &View) -> Map<String, Value> {
    let digest = json!({ "type": "string" });
    object_map(json!({
        "type": "object",
        "properties": {
            "request": {
                "type": "object",
                "description": "With solution_seal alone: the solve request as it was sent (the answer's inputs). It is solved again, under a physics permit and the limits of solve, and its seal is compared with solution_seal. Its fields are solve's."
            },
            "inputs": {
                "type": "object",
                "description": "The inputs of the solve answer, sent back as returned: the request that was solved. It is hashed, and solved again under a physics permit and the limits of solve."
            },
            "engine_version": { "type": "string" },
            "outcome": { "type": "string" },
            "iterations": { "type": ["integer", "null"], "minimum": 0 },
            "reject": { "type": ["string", "null"] },
            "result": { "type": ["object", "null"] },
            "invariants": { "type": ["object", "null"] },
            "field_sha256": { "type": ["string", "null"] },
            "seal_parts": {
                "type": "object",
                "properties": {
                    "inputs": digest,
                    "engine_version": digest,
                    "result": digest,
                    "invariants": digest,
                    "field_sha256": digest,
                    "outcome": digest,
                },
                "required": ["inputs", "engine_version", "result", "invariants", "field_sha256", "outcome"],
                "additionalProperties": true
            },
            "solution_seal": { "type": "string", "pattern": "^[0-9a-f]{64}$", "description": "The solution_seal of the solve answer: with request, or as part of the whole answer." },
            "from": {
                "type": "object",
                "description": "Beside the whole answer of a chained solve: the from solve took (request, run_seal, coupling). The upstream run is computed again and held to its seal, each handed value to it, and the request solved again with it; the answer alone is refused (AnswerCarriesNoRequest)."
            }
        },
        "additionalProperties": true,
        "description": "Either request and solution_seal alone, the short form: the request is solved again and its seal compared with solution_seal. Or the whole solve answer sent back as it came, without request, the long form: inputs, engine_version, outcome, iterations, reject, result, invariants, field_sha256, seal_parts and solution_seal are then all required. The solution seal is recomputed from its exact hex values and the part that fails is named; then its request is solved again under a physics permit and the limits of solve, and the engine's whole answer to it compared with the answer sent, value by value and then byte for byte, the receipt included, so an answer that holds is every byte the engine's. A seal has no key: an answer whose values were rewritten with its seal parts, seal and receipt worked out again holds every seal, and only the solve shows it is not the engine's (cause solution_seal). The long form's answer carries holds, true when the answer sent is every byte the engine's, and values_hold beside it, true when every value of it is the engine's answer to its request computed again, whatever its spelling: each number by its 64-bit value, strings, flags and null exactly, keys and their order, the receipt string among them. A copy whose numbers a client wrote again in its own spelling (0.0001 for 1.0e-4, 12 for 12.0) has holds false and values_hold true, and differs_at names the first place its text differs."
    }))
}

fn object_map(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("schema is an object"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// What every entry must hold for the parser, the description and the
    /// proof to agree. A new entry is held to it without a line added here.
    #[test]
    fn every_entry_is_whole() {
        let mut names = BTreeSet::new();
        for solver in ALL {
            let name = solver.name;
            assert!(crate::json_text::is_plain_field_name(name), "{name}");
            assert!(names.insert(name), "{name} is listed twice");
            assert!(solver.summary.ends_with('.'), "{name}");
            assert!(solver.kept_inside.ends_with('.'), "{name}");
            assert!(!solver.answer.is_empty(), "{name}");

            let mut inputs = BTreeSet::new();
            assert!(!solver.inputs.is_empty(), "{name}");
            for input in solver.inputs {
                let field = input.name;
                assert!(crate::json_text::is_plain_field_name(field), "{name} {field}");
                assert!(inputs.insert(field), "{name} {field} is listed twice");
                assert!(input.bound.ends_with('.'), "{name} {field}");
                match &input.kind {
                    Kind::Count {
                        minimum,
                        maximum,
                        spoken,
                    } => {
                        assert!(minimum <= maximum, "{name} {field}");
                        assert!(!spoken.is_empty(), "{name} {field}");
                    }
                    Kind::Number {
                        minimum, maximum, ..
                    } => assert!(minimum <= maximum, "{name} {field}"),
                    Kind::NumberAbove { above, maximum, .. } => assert!(above < maximum, "{name} {field}"),
                    Kind::NumberWithin { minimum, maximum, .. } => assert!(minimum < maximum, "{name} {field}"),
                    Kind::Word { allowed } => {
                        assert!(!allowed.is_empty(), "{name} {field}");
                        assert!(allowed.iter().all(|word| !word.is_empty()), "{name} {field}");
                    }
                    Kind::Flag => {
                        assert!(input.refusal.is_none(), "{name} {field}");
                        assert!(!input.asked_by_check, "{name} {field}");
                    }
                    Kind::Numbers {
                        fewest,
                        most,
                        minimum,
                        maximum,
                        ..
                    } => {
                        assert!(*most > 0, "{name} {field}");
                        assert!(fewest <= most, "{name} {field}");
                        assert!(minimum <= maximum, "{name} {field}");
                    }
                    Kind::NumbersWithin {
                        fewest,
                        most,
                        minimum,
                        minimum_exclusive,
                        maximum,
                        maximum_exclusive,
                        ..
                    } => {
                        assert!(*most > 0, "{name} {field}");
                        assert!(fewest <= most, "{name} {field}");
                        assert!(if *minimum_exclusive || *maximum_exclusive { minimum < maximum } else { minimum <= maximum }, "{name} {field}");
                    }
                }
                // A key that goes with one choice names a switch listed
                // before it, and a word that switch allows.
                if let Some((switch, word)) = input.only_when {
                    let place = solver
                        .inputs
                        .iter()
                        .position(|other| other.name == switch)
                        .unwrap_or_else(|| panic!("{name} {field}: no input {switch}"));
                    let here = solver
                        .inputs
                        .iter()
                        .position(|other| other.name == field)
                        .expect("listed");
                    assert!(place < here, "{name} {field}: {switch} is listed after it");
                    let switch_input = &solver.inputs[place];
                    match &switch_input.kind {
                        Kind::Word { allowed } => {
                            assert!(allowed.contains(&word), "{name} {field}: {word}")
                        }
                        _ => panic!("{name} {field}: {switch} is not a word"),
                    }
                }
                // Only a number or an array of numbers has bounds to ask.
                if input.asked_by_check {
                    assert!(
                        matches!(input.kind, Kind::Number { .. } | Kind::NumberAbove { .. } | Kind::NumberWithin { .. } | Kind::Numbers { .. } | Kind::NumbersWithin { .. }),
                        "{name} {field}"
                    );
                }
                // A bound's refusal is one the description lists, the
                // solver's or the entry's own, and the request shows it, so
                // it is answered with no seal.
                if let Some(refusal) = input.refusal {
                    let listed = solver
                        .listed_refusals()
                        .find(|listed| listed.name == refusal)
                        .unwrap_or_else(|| panic!("{name} {field}: {refusal} is not a refusal"));
                    assert!(!listed.sealed, "{name} {field}: {refusal}");
                }
                // Every input whose value can lie outside what the solver
                // takes has a name for it: a flag alone has no such value.
                if !matches!(input.kind, Kind::Flag) {
                    assert!(input.refusal.is_some(), "{name} {field}: a value outside it would be refused with no name");
                }
            }

            let mut refusals = BTreeSet::new();
            for refusal in solver.listed_refusals() {
                assert!(refusals.insert(refusal.name), "{name} {}", refusal.name);
                assert!(refusal.meaning.ends_with('.'), "{name} {}", refusal.name);
            }
            // What the entry names itself never reaches the solver, so it is
            // never sealed.
            for refusal in solver.door_refusals {
                assert!(!refusal.sealed, "{name} {}", refusal.name);
            }

            for section in ["result", "invariants"] {
                let mut outputs = BTreeSet::new();
                for output in solver.outputs(section) {
                    let field = output.name;
                    assert!(outputs.insert(field), "{name} {section}.{field} is listed twice");
                    assert!(!field.ends_with("_hex"), "{name} {section}.{field}");
                    assert!(!crate::json_text::is_flag(field), "{name} {section}.{field}");
                    let braces = field.matches('{').count() + field.matches('}').count();
                    // A value that is asked for is asked for by a flag every
                    // request gives, and is one name, not a family.
                    if let Some(flag) = output.asked_by {
                        assert!(output.each.is_empty(), "{name} {section}.{field}");
                        let asks = solver
                            .inputs
                            .iter()
                            .find(|input| input.name == flag)
                            .unwrap_or_else(|| panic!("{name} {section}.{field}: no input {flag}"));
                        assert!(matches!(asks.kind, Kind::Flag), "{name} {section}.{field}");
                        assert!(asks.only_when.is_none(), "{name} {section}.{field}");
                    }
                    if output.each.is_empty() {
                        assert_eq!(braces, 0, "{name} {section}.{field}");
                        assert!(crate::json_text::is_plain_field_name(field), "{name} {section}.{field}");
                        assert_eq!(solver.output_shape(section, field), Some(output.shape));
                    } else {
                        assert_eq!(braces, 2, "{name} {section}.{field}");
                        let (before, after) = output.family_ends().expect("a family");
                        for label in output.each {
                            let member = format!("{before}{label}{after}");
                            assert!(crate::json_text::is_plain_field_name(&member), "{member}");
                            assert_eq!(solver.output_shape(section, &member), Some(output.shape));
                            assert_eq!(
                                solver.published_shape(section, &format!("{member}_hex")),
                                Some(output.shape)
                            );
                        }
                    }
                }
            }

            // The example passes the parser, its own check included, and the
            // description is made.
            let example = example_solve(solver);
            crate::solution::prepare(&example)
                .unwrap_or_else(|error| panic!("{name}: {}", error.message()));
            let described = describe_whole(name).expect("described");
            assert_eq!(described["example_solve"], example, "{name}");
            // The description a caller reads first is the whole one, less
            // the parts kept for asking, each of which is said in a sentence
            // under `more` and is returned alone when asked for by its name.
            let first = describe_value(name).expect("described");
            if solver.parts.is_empty() {
                assert_eq!(first, described, "{name}");
            }
            let mut kept = BTreeSet::new();
            for part in solver.parts {
                let key = part.key;
                assert!(kept.insert(key), "{name} {key} is kept twice");
                assert!(!["solver", "summary", "engine_version", "more"].contains(&key), "{name} {key}");
                assert!(described.get(key).is_some(), "{name}: no key {key}");
                assert!(first.get(key).is_none(), "{name} {key}");
                assert!(part.holds.ends_with('.'), "{name} {key}");
                assert_eq!(first["more"][key], part.holds, "{name} {key}");
            }
            for (key, value) in described.as_object().expect("an object") {
                if kept.contains(key.as_str()) {
                    continue;
                }
                assert_eq!(&first[key], value, "{name} {key}");
            }
            for key in described.as_object().expect("an object").keys() {
                let alone = describe_part(name, key);
                if ["solver", "summary", "engine_version"].contains(&key.as_str()) {
                    assert!(alone.is_none(), "{name} {key}");
                } else {
                    assert_eq!(
                        alone,
                        Some(json!({ "solver": name, "part": key, key: described[key] })),
                        "{name} {key}"
                    );
                }
            }
            assert!(describe_part(name, "more").is_none(), "{name}");
            assert!(describe_part(name, "").is_none(), "{name}");
            let mut keys = BTreeSet::new();
            for further in solver.further_examples {
                assert!(further.key.starts_with("example_solve_"), "{name} {}", further.key);
                assert!(crate::json_text::is_plain_field_name(further.key), "{name} {}", further.key);
                assert!(keys.insert(further.key), "{name} {} is listed twice", further.key);
                let request = &described[further.key];
                assert_eq!(request["solver"], name);
                assert_eq!(request["parameters"], (further.parameters)(), "{name} {}", further.key);
                crate::solution::prepare(request)
                    .unwrap_or_else(|error| panic!("{name} {}: {}", further.key, error.message()));
            }
            // A bound the entry's check asks is asked: in every example, a
            // value just outside either end of the published range, and an
            // array one longer than its most, is refused before a permit.
            let mut examples = vec![example.clone()];
            for further in solver.further_examples {
                examples.push(described[further.key].clone());
            }
            for input in solver.inputs.iter().filter(|input| input.asked_by_check) {
                let field = input.name;
                let mut asked = 0;
                for example in &examples {
                    let Some(given) = example["parameters"].get(field) else {
                        continue;
                    };
                    let mut outside = Vec::new();
                    match &input.kind {
                        Kind::Number {
                            minimum, maximum, ..
                        } => {
                            outside.push(json!(minimum.next_down()));
                            outside.push(json!(maximum.next_up()));
                        }
                        // The lower end is exclusive: the end itself is refused.
                        Kind::NumberAbove { above, maximum, .. } => {
                            outside.push(json!(above));
                            outside.push(json!(maximum.next_up()));
                        }
                        // An exclusive end is itself refused.
                        Kind::NumberWithin {
                            minimum,
                            minimum_exclusive,
                            maximum,
                            maximum_exclusive,
                            ..
                        } => {
                            outside.push(json!(if *minimum_exclusive { *minimum } else { minimum.next_down() }));
                            outside.push(json!(if *maximum_exclusive { *maximum } else { maximum.next_up() }));
                        }
                        Kind::Numbers {
                            most,
                            minimum,
                            maximum,
                            ..
                        } => {
                            let items = given.as_array().expect("an array");
                            if let Some(last) = items.len().checked_sub(1) {
                                let mut low = items.clone();
                                low[last] = json!(minimum.next_down());
                                outside.push(Value::Array(low));
                                let mut high = items.clone();
                                high[last] = json!(maximum.next_up());
                                outside.push(Value::Array(high));
                            }
                            outside.push(json!(vec![*minimum; most + 1]));
                        }
                        // An exclusive end is itself refused.
                        Kind::NumbersWithin {
                            most,
                            minimum,
                            minimum_exclusive,
                            maximum,
                            maximum_exclusive,
                            ..
                        } => {
                            let items = given.as_array().expect("an array");
                            if let Some(last) = items.len().checked_sub(1) {
                                let mut low = items.clone();
                                low[last] = json!(if *minimum_exclusive { *minimum } else { minimum.next_down() });
                                outside.push(Value::Array(low));
                                let mut high = items.clone();
                                high[last] = json!(if *maximum_exclusive { *maximum } else { maximum.next_up() });
                                outside.push(Value::Array(high));
                            }
                            let inside = if *minimum_exclusive { minimum.next_up() } else { *minimum };
                            outside.push(json!(vec![inside; most + 1]));
                        }
                        _ => unreachable!("only a number or an array of numbers"),
                    }
                    for value in outside {
                        let mut request = example.clone();
                        request["parameters"][field] = value.clone();
                        assert!(
                            crate::solution::prepare(&request).is_err(),
                            "{name} {field}: {value} was not refused"
                        );
                        asked += 1;
                    }
                }
                assert!(asked > 0, "{name} {field}: no example gives it");
            }
            if let Some(validation) = solver.validation {
                for (part, text) in crate::descriptor::VALIDATION_PARTS.iter().zip(validation()) {
                    assert!(!text.is_empty(), "{name} validation.{part}");
                    assert_eq!(described["validation"][part], text.as_str(), "{name}");
                }
            } else {
                assert!(described.get("validation").is_none(), "{name}");
            }
            assert_eq!(
                described["inputs"].as_array().map(Vec::len),
                Some(solver.inputs.len()),
                "{name}"
            );
            assert!(described["returns"]["field_sha256"].as_str().unwrap().ends_with('.'));
        }
    }

    #[test]
    fn a_name_no_entry_has_is_not_known() {
        assert!(find("atom").is_none());
        assert!(!is_known(""));
        assert!(describe_value("atom_hf").is_none());
    }
}
