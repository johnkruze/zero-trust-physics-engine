//! `find_model`: from a plain description of a real thing to the served
//! models that fit it.
//!
//! No model is called and nothing is fetched: the description is matched
//! against a curated list of terms for each card and solver, written from
//! their own descriptions. The answer gives the best few, each with fit
//! first (the words that matched, and the one real thing the model is of)
//! and then accuracy (what it was compared with, and how it did against
//! that measurement, and no excerpt of its limits), kept apart so that a model that failed its pass marks
//! is not read as a model that does not fit. When nothing fits the answer is
//! complete in itself: every card and solver was compared, here are the
//! nearest by words and why they are not it, and here is how to ask for a
//! model. The same description always gives the same answer.
//!
//! How a description is read: its words are taken in their common forms, so
//! that forms of one word are one word; a term is found where its words stand
//! together; and each word of the description counts once for a model, at
//! the weight of the heaviest term found on it.

use serde_json::{Map, Value, json};

use crate::live::DoorError;
use crate::view::View;

/// The most models one answer returns.
pub const MOST_MATCHES: usize = 5;
/// How many it returns when `limit` is left out.
pub const DEFAULT_MATCHES: usize = 3;
/// The longest description read.
pub const MAX_DESCRIPTION_CHARS: usize = 400;
/// A model matches with at least this score and a term that names the thing
/// itself, or with two terms of its subject and one more point. Two terms of
/// its subject alone match when they are the whole description, as in "a
/// cell discharging": a description that is nothing but a model's subject is
/// a description of its kind.
const LEAST_SCORE: u32 = 4;
/// The most models an answer with no match names as the nearest by words.
pub const MOST_NEAREST: usize = 2;
/// The least score of a model named as nearest: one term that names a thing.
const NEAREST_SCORE: u32 = 3;

const STRONG: u32 = 3;
const PLAIN: u32 = 2;
const WEAK: u32 = 1;

/// Whether a model met the marks fixed before it was compared.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Marks {
    Met,
    Missed,
    InPart,
    /// A generic specimen: compared with no measurement.
    NotCompared,
    /// Compared with a reference calculation, not a measurement.
    Reference,
}

impl Marks {
    /// How a model did against its own measurements, in one word: the word
    /// the public site gives its thing. `list_cards`, `list_solvers` and
    /// `find_model` give it for each model as `against_measurement`.
    pub fn against_measurement(self) -> &'static str {
        match self {
            Marks::Met => "met",
            Marks::Missed => "missed",
            Marks::InPart => "met in part",
            Marks::NotCompared | Marks::Reference => "not held to a measurement",
        }
    }
}

/// What `against_measurement` holds, said in the description of each tool
/// that returns it.
pub(crate) const AGAINST_MEASUREMENT_SAYS: &str = "against_measurement is how the model did against its own measurements, in one of four words, \"met\", \"missed\", \"met in part\" or \"not held to a measurement\": it is accuracy and not fit, and a model that missed is still the served model of its thing.";

pub struct Entry {
    /// `domain/card` for a card, the solver's name for a solver.
    pub key: &'static str,
    pub marks: Marks,
    /// One short phrase after the verdict, from the model's own record.
    pub marks_words: &'static str,
    /// Terms that name the thing itself.
    pub strong: &'static [&'static str],
    /// Terms of its subject.
    pub plain: &'static [&'static str],
    /// Terms shared with many things.
    pub weak: &'static [&'static str],
}

const NOT_COMPARED: &str = "a generic specimen, not a named real object";

/// The index: one entry for each served card and solver, in the engine's order.
pub const INDEX: &[Entry] = &[
    Entry { key: "materials/ti64_lattice", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["ti-6al-4v", "ti64", "titanium", "lattice"],
        plain: &["column", "strut", "printed metal", "additive", "elastic", "torque", "alloy"],
        weak: &["force", "temperature", "metal", "structure", "load"] },
    Entry { key: "energy/grid_node", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["fuse", "fuse link", "copper link"],
        plain: &["overcurrent", "busbar", "conductor heating", "surge", "wire heating", "copper"],
        weak: &["current", "voltage", "electrical", "melt"] },
    Entry { key: "energy/swing", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["synchronous machine", "swing equation", "rotor angle", "synchronous generator"],
        plain: &["60 hz", "generator", "turbine", "pole slip", "mechanical power", "rotor", "synchronism"],
        weak: &["grid", "frequency", "machine", "power"] },
    Entry { key: "energy/battery_sag", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["battery pack", "12s", "pack", "voltage sag", "undervoltage"],
        plain: &["battery", "lithium", "lithium-ion", "li-ion", "cutoff", "drone battery", "discharge", "sag"],
        weak: &["current", "voltage", "cell", "amps", "load", "drain"] },
    Entry { key: "energy/cell_18650pf", marks: Marks::Met, marks_words: "met its marks on all eleven discharges",
        strong: &["18650", "ncr18650pf", "panasonic"],
        plain: &["lithium", "lithium-ion", "li-ion", "cell", "battery", "discharge", "1c", "capacity"],
        weak: &["voltage", "current", "cylindrical", "drain"] },
    Entry { key: "energy/thermal", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["cold plate", "heater", "avionics"],
        plain: &["heat sink", "thermal mass", "cooling", "warming", "bath", "heat"],
        weak: &["temperature", "power", "ambient", "electronics", "thermal"] },
    Entry { key: "signal/resonance", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["oscillator", "vibration mount", "spring mount", "mass on a spring", "isolator"],
        plain: &["spring", "resonance", "10 hz", "shaking", "vibration", "mount", "damper", "damped"],
        weak: &["acceleration", "displacement", "mass", "stroke"] },
    Entry { key: "signal/tesseract", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["duffing", "gyroscope", "mems", "drive mode", "resonator"],
        plain: &["nonlinear oscillator", "rate sensor", "gyro", "sensed velocity", "hardening spring"],
        weak: &["acceleration", "vibration", "sensor", "amplitude"] },
    Entry { key: "materials/hydraulic_fluid", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["hydraulic", "mineral oil", "oil column", "hydraulic line"],
        plain: &["actuator", "compressible fluid", "bulk modulus", "commanded flow", "oil"],
        weak: &["pressure", "flow", "fluid", "column", "pipe"] },
    Entry { key: "materials/cable_stretch", marks: Marks::NotCompared, marks_words: NOT_COMPARED,
        strong: &["dyneema", "tendon", "cable", "rope"],
        plain: &["stretch", "tension", "elongation", "winch", "line under load", "cord", "tether"],
        weak: &["force", "strain", "pull", "robot"] },
    Entry { key: "energy/generator_mmrtg_f1", marks: Marks::Missed, marks_words: "met its root mean square line (about 0.94 W against 1.154 W) and missed its final-year line (about 1.16 W high) on the held-back days 2,809 to 4,087, which it serves",
        strong: &["mmrtg", "rtg", "radioisotope", "curiosity", "plutonium", "thermoelectric generator"],
        plain: &["rover", "mars rover", "thermoelectric", "nuclear battery", "decay", "generator output", "mission day", "sol", "mars science laboratory"],
        weak: &["generator", "power", "mars", "electrical output", "years"] },
    Entry { key: "aerospace/capsule_stardust", marks: Marks::Met, marks_words: "met all three pass lines",
        strong: &["stardust", "sample return", "reentry", "entry capsule", "atmospheric entry"],
        plain: &["capsule", "atmosphere", "entry", "falling", "drogue", "parachute", "hypersonic", "deceleration", "heat shield", "returning from space", "meteor", "comet"],
        weak: &["earth", "mach", "altitude", "space", "g load", "descent"] },
    Entry { key: "aerospace/satellite_lageos1", marks: Marks::Met, marks_words: "met every pass line on 42 runs",
        strong: &["lageos", "satellite", "laser ranging"],
        plain: &["orbit", "spacecraft", "earth orbit", "gravity field", "orbital", "trajectory", "free flight", "ephemeris", "revolution"],
        weak: &["earth", "space", "sun", "moon", "tide", "position"] },
    Entry { key: "energy/reactor_spert3", marks: Marks::Missed, marks_words: "met five of eight pass lines",
        strong: &["spert", "reactivity", "power excursion", "prompt critical"],
        plain: &["reactor", "burst", "transient", "fission", "e-core", "kinetics", "power pulse", "criticality"],
        weak: &["nuclear", "power", "peak", "core", "step", "cold"] },
    Entry { key: "energy/cell_2170_m80", marks: Marks::Missed, marks_words: "only 27 of the 60 held-back runs met their marks",
        strong: &["2170", "tesla", "model 3", "m80"],
        plain: &["lithium", "lithium-ion", "li-ion", "cell", "battery", "discharge", "case temperature", "drive cycle"],
        weak: &["voltage", "current", "temperature", "electric car", "ev", "drain", "cold"] },
    Entry { key: "signal/cantilever_eth", marks: Marks::Missed, marks_words: "met three of four pass lines",
        strong: &["cantilever", "ring down", "ringdown", "ringing"],
        plain: &["beam", "strip", "aluminium", "aluminum", "bending mode", "damping", "vibrating", "clamped", "deflection", "free vibration", "eth", "flick", "pluck", "ruler", "tuning fork"],
        weak: &["vibration", "amplitude", "decay", "air", "oscillation"] },
    Entry { key: "energy/cell_2170_model3", marks: Marks::InPart, marks_words: "met all five conditions on one new cell and missed three of five on the other",
        strong: &["2170", "tesla", "model 3"],
        plain: &["lithium", "lithium-ion", "li-ion", "cell", "battery", "discharge", "case temperature", "drive cycle", "cold", "hysteresis", "freezing"],
        weak: &["voltage", "current", "temperature", "electric car", "ev", "winter", "drain"] },
    Entry { key: "materials/creep_cmsx6", marks: Marks::Met, marks_words: "the comparison passed on the six held-back tests",
        strong: &["creep", "cmsx-6", "cmsx6", "superalloy", "single crystal"],
        plain: &["nickel alloy", "turbine blade", "980", "high temperature", "tension", "rupture", "nickel"],
        weak: &["alloy", "strain", "hours", "load", "metal", "stress"] },
    Entry { key: "aerospace/capsule_insight", marks: Marks::InPart, marks_words: "met every pass line with the lift measured on the opened rows, and none of the first four with no lift",
        strong: &["insight", "mars entry", "entry capsule", "atmospheric entry", "mars lander"],
        plain: &["capsule", "atmosphere", "entry", "falling", "parachute", "hypersonic", "deceleration", "mars", "lander", "aeroshell", "landing", "meteor"],
        weak: &["mach", "altitude", "space", "g load", "descent"] },
    Entry { key: "energy/reactor_spert3_fuel", marks: Marks::Missed, marks_words: "met 18 of 24 pass lines across the three groups",
        strong: &["spert", "reactivity", "fuel temperature", "power excursion"],
        plain: &["reactor", "burst", "transient", "fission", "e-core", "inlet temperature", "coolant flow", "doppler", "power pulse"],
        weak: &["nuclear", "power", "peak", "core", "fuel", "hot"] },
    Entry { key: "aerospace/crater_ryugu_sci", marks: Marks::Missed, marks_words: "missed its pass line on Ryugu and met both laboratory lines",
        strong: &["crater", "ryugu", "hayabusa2", "hayabusa", "impactor", "asteroid"],
        plain: &["impact", "regolith", "ejecta", "projectile", "excavation", "microgravity"],
        weak: &["radius", "sand", "copper", "hit", "surface"] },
    Entry { key: "signal/link_gothenburg_10010", marks: Marks::Met, marks_words: "met all three pass lines",
        strong: &["microwave link", "rain attenuation", "rain fade", "openmrg", "gothenburg"],
        plain: &["rain", "32 ghz", "path loss", "radio link", "backhaul", "wet antenna", "attenuation", "millimetre wave", "microwave", "rainfall", "downpour"],
        weak: &["link", "signal", "weather", "antenna", "db", "storm"] },
    Entry { key: "energy/grid_gb", marks: Marks::Met, marks_words: "met all seven pass lines on the 12 held-back trips",
        strong: &["grid frequency", "power grid", "interconnector", "great britain", "transmission system", "national grid", "mains frequency", "system frequency"],
        plain: &["grid", "frequency", "50 hz", "inertia", "trip", "loss of infeed", "blackout", "losing a link", "electricity system", "outage", "generation loss", "frequency drop", "mains", "power cable", "undersea cable", "subsea cable", "infeed", "import", "nadir", "power station", "power cut"],
        weak: &["power", "loss", "demand", "link", "britain", "british", "uk", "england", "scotland", "wales", "france", "electricity", "mw", "fail", "sudden", "suddenly"] },
    Entry { key: "aerospace/solar_system", marks: Marks::InPart, marks_words: "two files met their lines, two missed and two comparisons were not made",
        strong: &["solar system", "planet", "n-body", "ephemeris"],
        plain: &["sun", "mercury", "venus", "jupiter", "saturn", "moon", "planetary", "gravitation", "barycentre", "mars orbit", "perihelion"],
        weak: &["earth", "orbit", "space", "gravity", "years", "mars"] },
    Entry { key: "materials/tire_eve_r19_ice", marks: Marks::Met, marks_words: "met all five pass lines",
        strong: &["tyre", "tire", "ice", "traction"],
        plain: &["slip", "friction", "wheel", "icy road", "all-season", "grip", "braking", "skid", "winter road"],
        weak: &["car", "road", "rolling", "load", "pressure", "cold"] },
    Entry { key: "aerospace/los_jwst_fine_guide", marks: Marks::Met, marks_words: "met all five marks on both axes",
        strong: &["jwst", "james webb", "webb", "fine guidance", "pointing"],
        plain: &["line of sight", "telescope", "jitter", "guide star", "kalman", "space telescope", "centroid", "pointing stability"],
        weak: &["sensor", "space", "star", "tracking", "filter"] },
    Entry { key: "energy/tokamak_jet_dte1", marks: Marks::Missed, marks_words: "met three of four marks on the primary pulse",
        strong: &["tokamak", "jet", "joint european torus", "deuterium-tritium", "h-mode", "plasma", "fusion"],
        plain: &["deuterium", "tritium", "confinement", "stored energy", "fusion power", "neutral beam", "dte1", "elmy", "culham", "d-t"],
        weak: &["energy", "pulse", "heating", "reactor", "magnetic"] },
    Entry { key: "materials/fingerpad_p02_glass", marks: Marks::Missed, marks_words: "met three of twelve pass lines",
        strong: &["finger", "fingertip", "finger pad", "fingerpad", "skin"],
        plain: &["glass", "twist", "touch", "slip torque", "friction", "touchscreen", "grip", "pressed"],
        weak: &["slip", "torque", "human", "contact", "hand"] },
    Entry { key: "energy/accumulator_sbo450", marks: Marks::Missed, marks_words: "met two of six pass lines",
        strong: &["accumulator", "hydac", "nitrogen charge", "diaphragm accumulator", "gas spring"],
        plain: &["hydraulic accumulator", "nitrogen", "piston", "gas pressure", "precharge", "compressed gas", "bladder", "diaphragm", "darmstadt"],
        weak: &["pressure", "oil", "gas", "hydraulic", "stiffness", "compression"] },
    Entry { key: "energy/reactor_triga_jsi", marks: Marks::Missed, marks_words: "met two of six pass lines on the held-back rows",
        strong: &["triga", "pulse mode", "reactivity", "research reactor"],
        plain: &["reactor", "pulse", "fission", "scram", "transient rod", "ljubljana", "fuel temperature", "power pulse"],
        weak: &["nuclear", "power", "peak", "core", "fuel", "rod"] },
    Entry { key: "energy/reactor_spert3_ccore", marks: Marks::Missed, marks_words: "missed all three pass lines",
        strong: &["spert", "c-core", "reactivity", "plate fuel"],
        plain: &["reactor", "transient", "fission", "plate temperature", "stainless", "step transient", "power excursion", "highly enriched"],
        weak: &["nuclear", "power", "peak", "core", "plate", "pressure"] },
    Entry { key: "atom_dft", marks: Marks::Reference, marks_words: "reproduces the NIST local-density tables within 1e-5 hartree; not the measured energy of the atom",
        strong: &["atom", "dft", "density functional", "electronic structure", "ground state"],
        plain: &["hydrogen", "helium", "neon", "argon atom", "orbital", "eigenvalue", "lda", "electron", "quantum", "total energy", "atomic number", "carbon", "oxygen"],
        weak: &["energy", "element", "chemistry", "nucleus"] },
    Entry { key: "ablator_slab", marks: Marks::Missed, marks_words: "its wear line passed and its hottest-face-temperature line missed on the average",
        strong: &["heat shield", "ablator", "ablation", "pica", "tacot", "charring"],
        plain: &["arc jet", "thermal protection", "recession", "pyrolysis", "entry heating", "slab", "heat flux", "ablative", "phenolic", "resin"],
        weak: &["heating", "temperature", "wear", "stardust", "material", "reentry"] },
    Entry { key: "journal_bearing_sheffield", marks: Marks::Met, marks_words: "met all three pass lines on the sixteen held-back runs",
        strong: &["journal bearing", "bearing", "oil film", "sommerfeld"],
        plain: &["lubrication", "eccentricity", "shaft", "hydrodynamic", "film thickness", "sheffield", "journal", "lubricated", "plain bearing", "oil-lubricated"],
        weak: &["oil", "load", "speed", "rpm", "friction", "rotating"] },
    Entry { key: "propeller_apc_10x7e", marks: Marks::Missed, marks_words: "missed both pass lines",
        strong: &["propeller", "apc", "blade element", "prop"],
        plain: &["thrust", "shaft power", "drone", "rotor", "rev/min", "airspeed", "uav", "quadcopter", "multirotor"],
        weak: &["rpm", "air", "blade", "motor", "aircraft", "power"] },
    Entry { key: "modulus_bam_s355", marks: Marks::Missed, marks_words: "missed one pass line (P1) and met every other",
        strong: &["young's modulus", "youngs modulus", "shear modulus", "s355", "structural steel", "elastic modulus"],
        plain: &["steel", "bar", "resonant frequency", "impulse excitation", "ringing bar", "stiffness", "bam", "modulus", "poisson"],
        weak: &["frequency", "metal", "elastic", "sheet", "rings"] },
    Entry { key: "seawater_column", marks: Marks::Met, marks_words: "met all three pass lines",
        strong: &["seawater", "ocean", "sound speed", "ctd", "teos-10", "sea"],
        plain: &["salinity", "depth", "density", "water column", "cast", "underwater", "sonar", "echo sounder", "oceanography", "speed of sound in water"],
        weak: &["water", "pressure", "temperature", "marine", "sound"] },
    Entry { key: "argon_pair_gas", marks: Marks::Met, marks_words: "met all five pass lines",
        strong: &["argon", "lennard-jones", "virial", "pair potential", "noble gas"],
        plain: &["speed of sound", "viscosity", "gas", "dilute gas", "second virial", "thermal conductivity", "low density"],
        weak: &["pressure", "temperature", "sound", "kinetic theory"] },
    Entry { key: "fusion_pb11_lhd", marks: Marks::Missed, marks_words: "missed three of four pass lines",
        strong: &["proton-boron", "p-b11", "pb11", "boron", "lhd", "large helical device", "aneutronic"],
        plain: &["fusion", "neutral beam", "alpha", "hydrogen beam", "stellarator", "helical", "plasma"],
        weak: &["beam", "reaction rate", "energy", "kev"] },
    Entry { key: "fusion_pb11_thermal", marks: Marks::Missed, marks_words: "missed one of eight gated pass lines; compared with published calculations",
        strong: &["proton-boron", "p-b11", "pb11", "boron", "aneutronic", "bremsstrahlung"],
        plain: &["fusion", "thermal plasma", "energy balance", "ignition", "synchrotron", "plasma", "electron temperature"],
        weak: &["reaction rate", "energy", "kev", "radiation"] },
    Entry { key: "step_flow_armaly", marks: Marks::Met, marks_words: "met all three pass lines on the held-back profiles",
        strong: &["backward-facing step", "step flow", "reattachment", "recirculation", "armaly"],
        plain: &["laminar", "separation", "channel flow", "separated flow", "duct", "navier-stokes", "cfd", "expansion"],
        weak: &["air", "flow", "reynolds", "velocity", "channel"] },
    Entry { key: "culture_trichoderma_ambr250", marks: Marks::Missed, marks_words: "missed three of six pass lines",
        strong: &["fed-batch", "trichoderma", "fermentation", "bioreactor", "culture", "mucic acid"],
        plain: &["fungus", "galacturonate", "biomass", "ambr", "bioprocess", "fermenter", "microbial", "feed", "broth", "cell culture"],
        weak: &["growth", "acid", "sugar", "vessel", "ph", "biology"] },
    Entry { key: "tokamak_dt_beam_target", marks: Marks::Missed, marks_words: "missed one of four marks",
        strong: &["tokamak", "deuterium-tritium", "fusion rate", "beam-target", "tftr", "jet", "joint european torus", "fusion"],
        plain: &["deuterium", "tritium", "neutron rate", "neutron", "neutral beam", "plasma", "fusion power", "d-t", "dt", "time slice", "culham", "princeton"],
        weak: &["beam", "reaction rate", "profile", "energy", "pulse"] },
    Entry { key: "pipe_flow_superpipe", marks: Marks::Missed, marks_words: "missed one of three pass lines (centreline, on one profile)",
        strong: &["pipe flow", "superpipe", "friction factor", "turbulent pipe", "pipe"],
        plain: &["turbulent", "turbulence", "smooth wall", "pressure drop", "velocity profile", "moody", "darcy", "duct", "mixing length", "princeton"],
        weak: &["air", "flow", "reynolds", "velocity", "wall", "tube"] },
];

/// Words that say nothing of the thing. "model" is not one: it is a word of
/// a served thing's name, the Tesla Model 3.
const STOP: &[&str] = &[
    "a", "an", "the", "of", "in", "on", "at", "to", "into", "from", "for", "with", "and", "or", "by", "as", "is", "are",
    "its", "it", "that", "this", "some", "something", "thing", "after", "before", "under", "over", "how", "what", "when",
    "while", "my", "our", "be", "do", "does", "will", "would", "can", "i", "we", "you", "one", "two", "being", "has",
    "have", "very", "real", "predict", "simulate", "happen", "happens", "want", "need", "about", "through",
];

pub fn description() -> String {
    format!(
        "Find the served model for a real thing described in plain words, such as \"a lithium cell discharging in the cold\". Returns the best few cards and solvers, each with fit first (the words that matched, and model_of, the one real thing it is a model of) and then accuracy (compared_with, the data it was fitted to and the data held back and compared, and accuracy_against_measurement: whether it passed the pass marks fixed before it was compared); a model's range and stops are in describe_card or describe_solver. against_measurement gives that verdict in one of four words, \"met\", \"missed\", \"met in part\" or \"not held to a measurement\", the word list_cards and list_solvers give on every row: it is accuracy and not fit. The two are separate: a model that failed its pass marks is still the served model of its thing. The names it gives (domain and card, or solver) are the names describe_card, describe_solver, cite and every other tool take, so no list need be read after it. When nothing served fits it says so, having compared every card and solver, names the nearest by words and why they are not it, or says that there is no near one, and says who to write to for a model. Matched against a fixed list of terms with no model call: the same words give the same answer. Same bytes as POST /find_model."
    )
}

pub fn schema() -> Map<String, Value> {
    match json!({
        "type": "object",
        "properties": {
            "description": { "type": "string", "description": "The real thing, in plain words." },
            "limit": { "type": "integer", "minimum": 1, "maximum": MOST_MATCHES, "description": format!("How many models to return; {DEFAULT_MATCHES} when left out.") }
        },
        "required": ["description"],
        "additionalProperties": false
    }) {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

/// Forms of a word that its endings do not join, each with the word it is a
/// form of: "entry" is the noun of "enter", so "a capsule entering an
/// atmosphere" and "atmospheric entry" share a word.
const FORMS: &[(&str, &str)] = &[
    ("entry", "enter"),
    ("reentry", "reenter"),
    ("sprung", "spring"),
    ("sprang", "spring"),
    ("rung", "ring"),
    ("rang", "ring"),
    ("fell", "fall"),
    ("fallen", "fall"),
    ("losing", "lose"),
    ("lost", "lose"),
    ("loss", "lose"),
];

/// A word with its common endings taken off, in this order: the plural, then
/// "-ing" or "-ed" (and the consonant they doubled: "slipping" is "slip"),
/// then "-ment", then a last "e". The plural goes first so that "bearings"
/// and "bearing", "speeds" and "speed" come to one word.
fn without_endings(word: &str) -> String {
    let mut word = word.to_string();
    let long = |word: &str, more_than: usize| word.chars().count() > more_than;
    if long(&word, 4) && word.ends_with("ies") {
        word.truncate(word.len() - 3);
        word.push('y');
    } else if long(&word, 3) && word.ends_with('s') && !word.ends_with("ss") && !word.ends_with("us") {
        word.pop();
    }
    let before = word.len();
    if long(&word, 5) && word.ends_with("ing") {
        word.truncate(word.len() - 3);
    } else if long(&word, 4) && word.ends_with("ed") {
        word.truncate(word.len() - 2);
    }
    if word.len() < before && long(&word, 3) {
        let mut last = word.chars().rev();
        if let (Some(a), Some(b)) = (last.next(), last.next()) {
            if a == b && "bdgmnprt".contains(a) {
                word.pop();
            }
        }
    }
    if long(&word, 7) && word.ends_with("ment") {
        word.truncate(word.len() - 4);
    }
    if long(&word, 4) && word.ends_with('e') {
        word.pop();
    }
    word
}

/// A word as it is compared: lowercase, with its common endings taken off, so
/// "discharging", "discharges" and "discharge" are one word, and a form the
/// endings do not join read as the word it is a form of.
fn stem(word: &str) -> String {
    let plain = without_endings(word);
    match FORMS.iter().find(|(form, _)| *form == word || *form == plain) {
        Some((_, of)) => without_endings(of),
        None => plain,
    }
}

/// The words of a text, in order: letters and digits. A hyphen parts two
/// words ("lithium-ion" is "lithium" and "ion"), except after "re", which
/// belongs to its word: "re-entry" is "reentry" and "re-attach" is "reattach".
fn words(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut out = Vec::new();
    let mut word = String::new();
    for ch in lower.chars() {
        if ch.is_alphanumeric() {
            word.push(ch);
        } else if ch == '-' && word == "re" {
            continue;
        } else if !word.is_empty() {
            out.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out.into_iter().map(|word| stem(&word)).collect()
}

/// The words of a text that say something of a thing: its words, less the
/// stop words. A description and a term are both read this way, so a term
/// that holds a stop word ("line of sight", "mass on a spring") is found.
fn telling_words(text: &str) -> Vec<String> {
    words(text).into_iter().filter(|word| !STOP.iter().any(|stop| stop == word || stem(stop) == *word)).collect()
}

/// The first place in the description where the term's words stand
/// together, in order.
fn place(described: &[String], term: &[String]) -> Option<usize> {
    if term.is_empty() || described.len() < term.len() {
        return None;
    }
    described.windows(term.len()).position(|window| window == term)
}

struct Scored {
    entry: &'static Entry,
    score: u32,
    matched: Vec<&'static str>,
    strong: bool,
    plain: usize,
    /// Whether every telling word of the description is held by a term.
    whole: bool,
}

/// A model's score for a description. Each term is found once, at its first
/// place, and each word of the description counts once for a model, at the
/// weight of the heaviest term found on it: "deuterium-tritium" does not
/// count again as "deuterium" and as "tritium", and two forms of one word
/// do not count as two words. A term counts as matched when it holds at
/// least one word no heavier or earlier term holds.
fn score(entry: &'static Entry, described: &[String]) -> Scored {
    // for each word of the description: the weight it counts at, and the term that holds it
    let mut held: Vec<(u32, usize)> = vec![(0, usize::MAX); described.len()];
    let mut found: Vec<(&'static str, u32)> = Vec::new();
    for (terms, weight) in [(entry.strong, STRONG), (entry.plain, PLAIN), (entry.weak, WEAK)] {
        for term in terms {
            let term_words = telling_words(term);
            if let Some(at) = place(described, &term_words) {
                for slot in &mut held[at..at + term_words.len()] {
                    if weight > slot.0 {
                        *slot = (weight, found.len());
                    }
                }
                found.push((*term, weight));
            }
        }
    }
    let holds_a_word = |index: usize| held.iter().any(|slot| slot.1 == index);
    let counted: Vec<(&'static str, u32)> =
        found.iter().enumerate().filter(|(index, _)| holds_a_word(*index)).map(|(_, term)| *term).collect();
    Scored {
        entry,
        score: held.iter().map(|slot| slot.0).sum(),
        matched: counted.iter().map(|(term, _)| *term).collect(),
        strong: counted.iter().any(|(_, weight)| *weight == STRONG),
        plain: counted.iter().filter(|(_, weight)| *weight == PLAIN).count(),
        whole: !held.is_empty() && held.iter().all(|slot| slot.0 > 0),
    }
}

/// The verdict of a model's comparison with measurement, in words that
/// cannot be read as a statement about fit: what it was held to, whether it
/// passed, and one clause from its own record.
fn accuracy(entry: &Entry) -> String {
    match entry.marks {
        Marks::Met => format!("passed the pass marks fixed before the comparison: {}", entry.marks_words),
        Marks::Missed => format!("failed the pass marks fixed before the comparison: {}", entry.marks_words),
        Marks::InPart => format!("passed in part the pass marks fixed before the comparison: {}", entry.marks_words),
        Marks::NotCompared => format!("not compared with any measurement: {}", entry.marks_words),
        Marks::Reference => format!("not compared with any measurement: {}", entry.marks_words),
    }
}

/// A model's verdict in one word and in one phrase: what `against_measurement`
/// and `accuracy_against_measurement` hold for it.
fn verdict(entry: &Entry) -> (&'static str, String) {
    (entry.marks.against_measurement(), accuracy(entry))
}

/// The verdict of a card, by its names. None for a card the index does not hold.
pub(crate) fn verdict_of_card(domain: &str, card: &str) -> Option<(&'static str, String)> {
    INDEX.iter().find(|entry| entry.key.split_once('/') == Some((domain, card))).map(verdict)
}

/// The verdict of a solver, by its name. None for a solver the index does not hold.
pub(crate) fn verdict_of_solver(solver: &str) -> Option<(&'static str, String)> {
    INDEX.iter().find(|entry| entry.key == solver).map(verdict)
}

/// What one served model says of itself, read from its own description: its
/// names and the one real thing it is a model of.
fn named(view: &View, entry: &Entry) -> Option<(Map<String, Value>, Option<Value>, Option<Value>)> {
    let mut row = Map::new();
    let (summary, validation, reference) = match entry.key.split_once('/') {
        Some((domain, card)) => {
            let described = crate::descriptor::describe_value_in(view, domain, card)?;
            row.insert("domain".to_string(), json!(domain));
            row.insert("card".to_string(), json!(card));
            (described["specimen"].clone(), described.get("validation").cloned(), None)
        }
        None => {
            if !view.sees_solver(entry.key) {
                return None;
            }
            let described = crate::solvers::describe_whole(entry.key)?;
            row.insert("solver".to_string(), json!(entry.key));
            (described["summary"].clone(), described.get("validation").cloned(), described.get("reference").cloned())
        }
    };
    row.insert("model_of".to_string(), summary);
    Some((row, validation, reference))
}

/// One matched model: fit first (the words that matched and the one real
/// thing it is a model of), then accuracy (what it was compared with and how
/// it did). No excerpt of the record's limits is given: the opening
/// sentences of that paragraph do not stand alone ("found 7.94 percent where
/// the first found 7.03", "an easier set than the opened ones"), and the
/// whole of it is one call away.
fn record(view: &View, entry: &Entry, matched: &[&str]) -> Option<Map<String, Value>> {
    let (named, validation, reference) = named(view, entry)?;
    let mut row = Map::new();
    let mut named = named.into_iter();
    // the names, then fit, then what it is a model of
    let mut model_of = Value::Null;
    for (key, value) in named.by_ref() {
        if key == "model_of" {
            model_of = value;
        } else {
            row.insert(key, value);
        }
    }
    let terms: Vec<String> = matched.iter().map(|term| format!("\"{term}\"")).collect();
    row.insert("fit".to_string(), json!(format!("matches your description on the words {}", terms.join(", "))));
    row.insert("model_of".to_string(), model_of);
    let text = |value: Option<&Value>| value.and_then(Value::as_str).unwrap_or("").to_string();
    let validation = validation.unwrap_or(Value::Null);
    // Where the company keeps its own record of a comparison is no part of
    // what the model was compared with.
    let data_text = text(validation.get("data"));
    let data_text = data_text.split(" Record: evidence/").next().unwrap_or("").to_string();
    let compared_with = match entry.marks {
        Marks::NotCompared => "No measurement: a generic specimen, not a named real object.".to_string(),
        Marks::Reference => {
            let reference = reference.unwrap_or(Value::Null);
            format!("No measurement; a reference calculation: {}.", text(reference.get("name")).trim_end_matches('.'))
        }
        // The record's whole account of its data: what the model was fitted
        // to and, further on, what was held back and compared. Its opening
        // alone names only the fitting data, which is not what the model was
        // compared with.
        _ => data_text,
    };
    row.insert("compared_with".to_string(), json!(compared_with));
    let (word, phrase) = verdict(entry);
    row.insert("against_measurement".to_string(), json!(word));
    row.insert("accuracy_against_measurement".to_string(), json!(phrase));
    Some(row)
}

/// What a connection serves, in words: "31 cards and 13 solvers".
fn served(cards: usize, solvers: usize) -> String {
    let counted = |count: usize, what: &str| if count == 1 { format!("1 {what}") } else { format!("{count} {what}s") };
    format!("{} and {}", counted(cards, "card"), counted(solvers, "solver"))
}

/// What an answer with matches says after them.
fn about(cards: usize, solvers: usize) -> String {
    let served = served(cards, solvers);
    format!(
        "Fit and accuracy are separate. Fit: each match is a match on words, and model_of is the one real thing the model is of. For a thing described by its kind (a cell discharging, a capsule entering an atmosphere), a match is the served model of that kind of thing, and fits. For one particular other object (a named cell, a dataset of your own), a model of a different object of the same kind is not a model of it, and nothing served fits. Accuracy: accuracy_against_measurement is how the model did against its own measurements, and a model that failed its pass marks is still the served model of its thing, served with the failure stated. compared_with is the record's whole account of its data: what the model was fitted to and what was held back and compared with it. describe_card or describe_solver, by the names given here, has the pass marks, the result, the range and every stop by name, and cite, by the same names, the people whose data it stands on. Everything this connection serves ({served}) was compared with the description; when nothing served fits, a model can be asked for: write to {} at {} and name an open dataset of the thing.",
        crate::identity::MAKER,
        crate::identity::EMAIL
    )
}

/// What an answer with no match says: complete in itself.
fn none_fits(cards: usize, solvers: usize) -> String {
    let served = served(cards, solvers);
    format!(
        "Nothing served fits this description. It was compared with everything this connection serves ({served}), so reading list_cards or list_solvers will not find one: each served model is of one named real thing, not of a class of things. {}",
        crate::identity::how_to_ask_for_a_model()
    )
}

const NO_NEAREST: &str = "No served model shares a word that names the thing described, so there is no near model to name; a word of a wide subject (steel, temperature, load) is shared with too many things to count.";
const NEAREST_ABOUT: &str = "The served models that share the most words with the description, named so that the lists need not be read. None of them matched: each is a model of the thing in its model_of, not of the thing described.";

pub fn find_in(view: &View, request: &Value) -> Result<Value, DoorError> {
    let violation = |text: String| DoorError::Unprocessable(text);
    let object = request
        .as_object()
        .ok_or_else(|| violation("Physical Violation: the request must be a JSON object.".into()))?;
    for key in object.keys() {
        if key != "description" && key != "limit" {
            return Err(violation(format!(
                "Physical Violation: '{key}' is not a key of find_model; it takes description and limit."
            )));
        }
    }
    let described = match object.get("description") {
        None => return Err(violation("Missing required parameter: 'description'".into())),
        Some(Value::String(text)) => text,
        Some(_) => return Err(violation("Physical Violation: 'description' must be a string.".into())),
    };
    if described.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(violation(format!(
            "Physical Violation: 'description' holds {} characters; at most {MAX_DESCRIPTION_CHARS} (MAX_DESCRIPTION_CHARS) are read.",
            described.chars().count()
        )));
    }
    let limit = match object.get("limit") {
        None => DEFAULT_MATCHES,
        Some(value) => match value.as_u64() {
            Some(count) if (1..=MOST_MATCHES as u64).contains(&count) && value.is_u64() => count as usize,
            _ => {
                return Err(violation(format!(
                    "Physical Violation: 'limit' must be a whole number from 1 to {MOST_MATCHES}."
                )));
            }
        },
    };
    let read = telling_words(described);
    let fits = |scored: &Scored| {
        scored.score >= LEAST_SCORE && (scored.strong || scored.plain >= 2 && (scored.score > LEAST_SCORE || scored.whole))
    };
    let mut scored: Vec<Scored> = INDEX.iter().map(|entry| score(entry, &read)).collect();
    // The best first; equal scores keep the engine's order.
    scored.sort_by(|a, b| b.score.cmp(&a.score));
    let mut matches = Vec::new();
    for found in scored.iter().filter(|scored| fits(scored)) {
        if matches.len() == limit {
            break;
        }
        let Some(mut row) = record(view, found.entry, &found.matched) else {
            continue;
        };
        row.insert("score".to_string(), json!(found.score));
        matches.push(Value::Object(row));
    }
    let (cards, solvers) = (view.cards().len(), view.solver_names().len());
    let mut answer = Map::new();
    answer.insert("description".to_string(), json!(described));
    answer.insert("searched".to_string(), json!({ "cards": cards, "solvers": solvers }));
    answer.insert("count".to_string(), json!(matches.len()));
    if matches.is_empty() {
        answer.insert("matches".to_string(), json!([]));
        answer.insert("none_fits".to_string(), json!(none_fits(cards, solvers)));
        // The nearest by words: a model that shares a term naming the thing
        // itself, or a term of its subject and one more point, short of a
        // match. One word of a subject ("capacity", "beam") is shared with
        // too many things to be worth naming.
        let mut nearest = Vec::new();
        for found in scored.iter().filter(|scored| (scored.strong || scored.plain >= 1) && scored.score >= NEAREST_SCORE) {
            if nearest.len() == MOST_NEAREST {
                break;
            }
            let Some((mut row, _, _)) = named(view, found.entry) else {
                continue;
            };
            let terms: Vec<String> = found.matched.iter().map(|term| format!("\"{term}\"")).collect();
            row.insert("not_a_match".to_string(), json!(format!("shares only {} with the description", terms.join(", "))));
            nearest.push(Value::Object(row));
        }
        let nearest_about = if nearest.is_empty() { NO_NEAREST } else { NEAREST_ABOUT };
        answer.insert("nearest_by_words".to_string(), Value::Array(nearest));
        answer.insert("nearest_by_words_about".to_string(), json!(nearest_about));
    } else {
        answer.insert("matches".to_string(), Value::Array(matches));
        answer.insert("about".to_string(), json!(about(cards, solvers)));
    }
    Ok(Value::Object(answer))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(description: &str) -> Vec<String> {
        let answer = find_in(&View::full(), &json!({ "description": description, "limit": 5 })).unwrap();
        answer["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| match row.get("solver") {
                Some(solver) => solver.as_str().unwrap().to_string(),
                None => format!("{}/{}", row["domain"].as_str().unwrap(), row["card"].as_str().unwrap()),
            })
            .collect()
    }

    #[test]
    fn the_index_holds_every_served_card_and_solver_in_order() {
        let served: Vec<String> = crate::descriptor::all()
            .iter()
            .map(|card| format!("{}/{}", card.domain, card.card))
            .chain(crate::solvers::ALL.iter().map(|solver| solver.name.to_string()))
            .filter(|name| name != crate::solver_probe::STAND_IN.name)
            .collect();
        let indexed: Vec<String> = INDEX.iter().map(|entry| entry.key.to_string()).collect();
        assert_eq!(indexed, served);
    }

    /// The verdict in the index is held to the model's own record: a generic
    /// specimen says it was compared with nothing, and no other model does.
    #[test]
    fn the_marks_follow_each_models_own_record() {
        let view = View::full();
        for entry in INDEX {
            let result = match entry.key.split_once('/') {
                Some((domain, card)) => crate::descriptor::describe_value_in(&view, domain, card).unwrap()["validation"]["result"].clone(),
                None => crate::solvers::describe_whole(entry.key).unwrap()["validation"]["result"].clone(),
            };
            let result = result.as_str().unwrap_or("");
            match entry.marks {
                Marks::NotCompared => assert_eq!(result, "not compared with a measurement", "{}", entry.key),
                Marks::Reference => assert_eq!(result, "", "{}", entry.key),
                Marks::Met => {
                    assert!(!result.contains("comparison failed") && !result.contains("did not pass") && !result.is_empty(), "{}: {result}", entry.key)
                }
                Marks::Missed => assert!(
                    result.contains("failed") || result.contains("missed") || result.contains("not pass"),
                    "{}: {result}",
                    entry.key
                ),
                Marks::InPart => assert!(result.contains("met") && (result.contains("missed") || result.contains("not met") || result.contains("none of")), "{}", entry.key),
            }
            assert!(record(&view, entry, &[]).is_some(), "{}", entry.key);
        }
        // The generator serves the days it failed on, and its phrase says
        // so in the record's own numbers: 0.008129 and +0.010094 of 115.4 W.
        let generator = INDEX.iter().find(|entry| entry.key == "energy/generator_mmrtg_f1").expect("the generator");
        assert_eq!(generator.marks, Marks::Missed);
        assert_eq!(
            accuracy(generator),
            "failed the pass marks fixed before the comparison: met its root mean square line (about 0.94 W against 1.154 W) and missed its final-year line (about 1.16 W high) on the held-back days 2,809 to 4,087, which it serves"
        );
        let record = crate::descriptor::describe_value_in(&view, "energy", "generator_mmrtg_f1").unwrap()["validation"].clone();
        let result = record["result"].as_str().unwrap();
        assert!(result.starts_with("One line met and one missed, so by the rule fixed beforehand the comparison failed."), "{result}");
        for words in ["2,809 to 4,087", "0.008129, about 0.94 W, inside the line of 0.010", "+0.010094, about 1.16 W high, outside the line"] {
            assert!(result.contains(words), "{words}: {result}");
        }
        assert!((0.008129_f64 * 115.4 - 0.94).abs() < 0.005 && (0.010094_f64 * 115.4 - 1.16).abs() < 0.005 && (0.010_f64 * 115.4 - 1.154).abs() < 1e-12);
    }

    /// Every term is found in a description that says it, stop words and
    /// all, and no model lists one word twice under two spellings.
    #[test]
    fn every_term_is_found_in_its_own_words_and_none_is_listed_twice() {
        let mut faults = Vec::new();
        for entry in INDEX {
            let mut seen: Vec<(Vec<String>, &str)> = Vec::new();
            for term in entry.strong.iter().chain(entry.plain).chain(entry.weak) {
                let read = telling_words(term);
                if read.is_empty() || !score(entry, &read).matched.contains(term) {
                    faults.push(format!("{}: \"{term}\" is not found in its own words", entry.key));
                }
                if let Some((_, other)) = seen.iter().find(|(words, _)| *words == read) {
                    faults.push(format!("{}: \"{term}\" and \"{other}\" are one word", entry.key));
                }
                seen.push((read, term));
            }
        }
        assert!(faults.is_empty(), "{}", faults.join("\n"));
    }

    #[test]
    fn plain_descriptions_find_their_models() {
        // (description, the model that must come first, models that must be among the matches)
        let table: &[(&str, &str, &[&str])] = &[
            ("a lithium cell discharging in the cold", "energy/cell_2170_model3", &["energy/cell_2170_m80", "energy/cell_18650pf"]),
            ("something falling into an atmosphere", "aerospace/capsule_stardust", &["aerospace/capsule_insight"]),
            ("power grid frequency after losing a link", "energy/grid_gb", &[]),
            ("a satellite in orbit around the Earth", "aerospace/satellite_lageos1", &[]),
            ("the heat shield of a capsule charring in an arc jet", "ablator_slab", &[]),
            ("a research reactor pulse after a step in reactivity", "energy/reactor_triga_jsi", &["energy/reactor_spert3"]),
            ("how fast does a turbine blade superalloy creep at high temperature", "materials/creep_cmsx6", &[]),
            ("a car tyre slipping on ice", "materials/tire_eve_r19_ice", &[]),
            ("pointing jitter of the James Webb telescope", "aerospace/los_jwst_fine_guide", &[]),
            ("rain fade on a microwave link", "signal/link_gothenburg_10010", &[]),
            ("the speed of sound in seawater at depth", "seawater_column", &[]),
            ("thrust of a drone propeller", "propeller_apc_10x7e", &[]),
            ("turbulent flow in a smooth pipe", "pipe_flow_superpipe", &[]),
            ("ground state energy of a neon atom", "atom_dft", &[]),
            ("the crater an impactor makes on an asteroid", "aerospace/crater_ryugu_sci", &[]),
            ("a fed-batch fermentation of a fungus", "culture_trichoderma_ambr250", &[]),
            ("the output of the Curiosity rover's radioisotope generator over the years", "energy/generator_mmrtg_f1", &[]),
            ("a 12S battery pack sagging under load until cutoff", "energy/battery_sag", &[]),
            ("an aluminium cantilever ringing down in air", "signal/cantilever_eth", &[]),
            ("a fingertip twisting on glass until it slips", "materials/fingerpad_p02_glass", &[]),
            ("oil film in a journal bearing", "journal_bearing_sheffield", &[]),
            ("Young's modulus of a steel bar from the frequency it rings at", "modulus_bam_s355", &[]),
            ("proton-boron fusion under a neutral beam", "fusion_pb11_lhd", &["fusion_pb11_thermal"]),
            ("deuterium-tritium fusion rate in a tokamak", "tokamak_dt_beam_target", &["energy/tokamak_jet_dte1"]),
            ("laminar air flow over a backward-facing step", "step_flow_armaly", &[]),
            ("viscosity of argon gas", "argon_pair_gas", &[]),
            ("the planets of the Solar System over twenty years", "aerospace/solar_system", &[]),
            ("a nitrogen-charged hydraulic accumulator", "energy/accumulator_sbo450", &[]),
            ("a Dyneema tendon stretching under force", "materials/cable_stretch", &[]),
            ("a copper fuse link under overcurrent", "energy/grid_node", &[]),
        ];
        for (description, first, among) in table {
            let found = keys(description);
            assert_eq!(found.first().map(String::as_str), Some(*first), "{description}: {found:?}");
            for key in *among {
                assert!(found.iter().any(|item| item == key), "{description}: {key} not in {found:?}");
            }
        }
    }

    #[test]
    fn descriptions_of_things_not_served_return_nothing() {
        for description in [
            "the price of bitcoin tomorrow",
            "protein folding of haemoglobin",
            "tomorrow's weather in Paris",
            "traffic on a motorway at rush hour",
            "the spread of a virus through a city",
            "a suspension bridge in a crosswind",
            "how many calories are in an apple",
            "the stock market after an election",
            "a wind turbine blade in a storm",
            "the temperature of the power of the load",
            "",
        ] {
            let answer = find_in(&View::full(), &json!({ "description": description })).unwrap();
            assert_eq!(answer["count"], 0, "{description}: {}", answer["matches"]);
            assert!(answer["none_fits"].as_str().unwrap().contains("kruze@zerotrustphysics.com"));
            assert!(answer.get("about").is_none());
        }
    }

    fn first_of(description: &str) -> Option<String> {
        keys(description).into_iter().next()
    }

    /// Sentences in a user's own words, and the model each finds first.
    #[test]
    fn a_users_own_sentence_finds_its_model() {
        let table: &[(&str, &str)] = &[
            ("how far the mains frequency in Britain drops when the undersea power cable from France suddenly fails", "energy/grid_gb"),
            ("how low the frequency of the Great Britain grid falls in the minute after it suddenly loses 1,800 MW of imports", "energy/grid_gb"),
            ("a thin aluminium strip clamped at one end, ringing after I flick it", "signal/cantilever_eth"),
            ("the electrical power of the Curiosity rover's generator on mission day 2,000", "energy/generator_mmrtg_f1"),
            ("the minimum oil film thickness in the University of Sheffield journal bearing rig at a load of 25 kN and 300 rpm", "journal_bearing_sheffield"),
            ("the largest constant current the battery pack can deliver for ten minutes starting from full charge", "energy/battery_sag"),
            ("cell m80 of the Tesla Model 3 2170 cell dataset, discharged at a constant current for thirty minutes", "energy/cell_2170_m80"),
            ("this service's model of the Stardust capsule's entry", "aerospace/capsule_stardust"),
            ("this service's creep model of the superalloy CMSX-6", "materials/creep_cmsx6"),
        ];
        for (description, first) in table {
            assert_eq!(first_of(description).as_deref(), Some(*first), "{description}: {:?}", keys(description));
        }
    }

    /// The same things in other words: the index is of the things, not of
    /// particular sentences.
    #[test]
    fn the_same_things_in_other_words_find_their_models() {
        let table: &[(&str, &str)] = &[
            ("how much the mains frequency sags when a big power station drops off the system", "energy/grid_gb"),
            ("what happens to the 50 Hz system frequency when an interconnector trips", "energy/grid_gb"),
            ("a clamped metal strip vibrating after being plucked", "signal/cantilever_eth"),
            ("power from the nuclear battery on the Mars rover after six years", "energy/generator_mmrtg_f1"),
            ("how thick the oil film is in a lubricated shaft bearing", "journal_bearing_sheffield"),
            ("a comet sample capsule coming back through the atmosphere", "aerospace/capsule_stardust"),
            ("a radio link losing signal in heavy rainfall", "signal/link_gothenburg_10010"),
            ("a steel cable stretching under a winch load", "materials/cable_stretch"),
        ];
        for (description, first) in table {
            assert_eq!(first_of(description).as_deref(), Some(*first), "{description}: {:?}", keys(description));
        }
    }

    /// The other direction: a description that shares a word with a served
    /// model, and is not of its thing, matches nothing. The wider terms of
    /// the grid ("mains", "power cable", "undersea cable") and of the strip
    /// ("ruler", "flick") must not pull these in.
    #[test]
    fn a_thing_that_only_shares_a_word_matches_nothing() {
        for description in [
            "how much load a steel I-beam bridge can carry",
            "steel I-beam bridge load capacity",
            "the temperature of the brake discs of a freight train after an emergency stop from 100 km/h",
            "an undersea telephone cable laid across the Atlantic",
            "a power cable overheating in a duct",
            "mains water pressure in a city's pipes",
            "the frequency of a guitar string",
            "the output of France's nuclear fleet next winter",
            "a ruler measuring the length of a table",
            "a rover wheel sinking in sand on Mars",
            "the mains voltage in a house in Britain",
        ] {
            let answer = find_in(&View::full(), &json!({ "description": description, "limit": 5 })).unwrap();
            assert_eq!(answer["count"], 0, "{description}: {}", answer["matches"]);
        }
    }

    /// An answer with no match is complete in itself: it says every card and
    /// solver was compared, that the lists will not find one, and how to ask;
    /// and it names the nearest by words as not matches.
    #[test]
    fn an_answer_with_no_match_is_complete_in_itself() {
        let view = View::full();
        let answer = find_in(&view, &json!({ "description": "how much load a steel I-beam bridge can carry" })).unwrap();
        let said = answer["none_fits"].as_str().unwrap();
        let (cards, solvers) = (view.cards().len(), view.solver_names().len());
        assert_eq!(answer["searched"], json!({ "cards": cards, "solvers": solvers }));
        assert!(said.starts_with("Nothing served fits this description."), "{said}");
        assert!(said.contains(&format!("everything this connection serves ({cards} cards and {solvers} solvers)")), "{said}");
        assert!(said.contains("reading list_cards or list_solvers will not find one"), "{said}");
        for field in ["write to John Kruze of Protocol Company at kruze@zerotrustphysics.com", "an open dataset", "its licence", "the quantity measured", "who measured it", "none from memory"] {
            assert!(said.contains(field), "{field}: {said}");
        }
        // One word of a subject is not worth naming: the bridge shares "steel"
        // with the steel bar and "beam" with the strip, and neither is named.
        assert_eq!(answer["nearest_by_words"], json!([]), "{answer}");
        assert!(answer["nearest_by_words_about"].as_str().unwrap().starts_with("No served model shares a word that names the thing described, so there is no near model to name"));
        // A word that names a served thing is: the nearest is given, as not a match.
        let cable = find_in(&view, &json!({ "description": "an undersea telephone cable laid across the Atlantic" })).unwrap();
        assert_eq!(cable["count"], 0);
        let nearest = cable["nearest_by_words"].as_array().unwrap();
        assert!(!nearest.is_empty() && nearest.len() <= MOST_NEAREST);
        assert_eq!(nearest[0]["card"], "cable_stretch");
        for row in nearest {
            assert!(row["not_a_match"].as_str().unwrap().starts_with("shares only \""), "{row}");
            assert!(row["model_of"].is_string() && row.get("fit").is_none() && row.get("accuracy_against_measurement").is_none(), "{row}");
        }
        assert!(cable["nearest_by_words_about"].as_str().unwrap().contains("None of them matched"));
        let alone = find_in(&view, &json!({ "description": "the price of bitcoin tomorrow" })).unwrap();
        assert!(alone["nearest_by_words"] == json!([]) && alone["none_fits"].is_string());
    }

    /// Fit and accuracy cannot be confused: fit comes first, in its own
    /// words; the verdict is a phrase that says what was failed or passed,
    /// with no true or false and no bare "missed:" before a count of what was
    /// met. A bare "met_its_marks": false can be read as "does not fit", and
    /// "missed: 27 of 60 runs met their marks" backwards.
    #[test]
    fn fit_comes_first_and_the_verdict_is_one_unambiguous_phrase() {
        let view = View::full();
        for entry in INDEX {
            let row = record(&view, entry, &["term"]).unwrap();
            let order: Vec<&str> = row.keys().map(String::as_str).filter(|key| !matches!(*key, "domain" | "card" | "solver")).collect();
            assert_eq!(order, ["fit", "model_of", "compared_with", "against_measurement", "accuracy_against_measurement"], "{}", entry.key);
            assert!(row.values().all(|value| value.is_string()), "{}: no true, false or null to misread", entry.key);
            let verdict = row["accuracy_against_measurement"].as_str().unwrap();
            let opening = match entry.marks {
                Marks::Met => "passed the pass marks fixed before the comparison: ",
                Marks::Missed => "failed the pass marks fixed before the comparison: ",
                Marks::InPart => "passed in part the pass marks fixed before the comparison: ",
                Marks::NotCompared | Marks::Reference => "not compared with any measurement: ",
            };
            assert!(verdict.starts_with(opening), "{}: {verdict}", entry.key);
            // the clause after it begins with what the model did, not with a count
            let clause = &verdict[opening.len()..];
            if matches!(entry.marks, Marks::Met | Marks::Missed | Marks::InPart) {
                assert!(
                    ["met ", "missed ", "only ", "its ", "the comparison passed", "two files met"].iter().any(|start| clause.starts_with(start)),
                    "{}: {clause}",
                    entry.key
                );
            }
            // nothing is cut part way through a sentence
            let text = row["compared_with"].as_str().unwrap();
            assert!(!text.contains('…') && text.ends_with(['.', ')', '"']), "{}: {text}", entry.key);
            // no path of the company's own disk
            assert!(!text.contains("evidence/"), "{}: {text}", entry.key);
        }
        let answer = find_in(&view, &json!({ "description": "a thin aluminium strip clamped at one end, ringing after I flick it" })).unwrap();
        let row = &answer["matches"][0];
        assert_eq!(row["card"], "cantilever_eth");
        assert_eq!(row["fit"], "matches your description on the words \"ringing\", \"strip\", \"aluminium\", \"clamped\", \"flick\"");
        assert_eq!(row["accuracy_against_measurement"], "failed the pass marks fixed before the comparison: met three of four pass lines");
        // what it was compared with is the data, not the specimen again
        assert!(row["compared_with"].as_str().unwrap().contains("recordings of the strip ringing down"), "{row}");
        let about = answer["about"].as_str().unwrap();
        assert!(about.starts_with("Fit and accuracy are separate.") && about.contains("is still the served model of its thing") && about.contains("kruze@zerotrustphysics.com") && !about.contains("request_model"), "{about}");
        // A thing described by its kind is fitted by the served model of that
        // kind; only a particular other object is not. The sentence says a
        // match fits, so that it is not read as none fits.
        assert!(about.contains("For a thing described by its kind (a cell discharging, a capsule entering an atmosphere), a match is the served model of that kind of thing, and fits."), "{about}");
        assert!(about.contains("For one particular other object (a named cell, a dataset of your own), a model of a different object of the same kind is not a model of it, and nothing served fits."), "{about}");
        // The two Tesla cards no longer tie for a sentence that names cell m80.
        assert_eq!(first_of("Tesla Model 3 2170 cell m80 constant current discharge at 25 C").as_deref(), Some("energy/cell_2170_m80"));
    }

    /// Every served card and solver carries `against_measurement` on its
    /// row of `list_cards` or `list_solvers` and in a match of `find_model`,
    /// and the word is the one its marks give.
    #[test]
    fn every_served_model_carries_its_word_on_its_list_row_and_in_a_match() {
        let view = View::full();
        let cards = crate::descriptor::list_value_in(&view);
        let solvers = crate::solvers::list_value_in(&view);
        let mut counted = std::collections::BTreeMap::new();
        for entry in INDEX {
            let word = match entry.marks {
                Marks::Met => "met",
                Marks::Missed => "missed",
                Marks::InPart => "met in part",
                Marks::NotCompared | Marks::Reference => "not held to a measurement",
            };
            assert_eq!(entry.marks.against_measurement(), word, "{}", entry.key);
            *counted.entry(word).or_insert(0) += 1;
            let row = match entry.key.split_once('/') {
                Some((domain, card)) => cards["cards"].as_array().unwrap().iter().find(|row| row["domain"] == domain && row["card"] == card),
                None => solvers["solvers"].as_array().unwrap().iter().find(|row| row["solver"] == entry.key),
            };
            assert_eq!(row.unwrap_or_else(|| panic!("{} is on no list", entry.key))["against_measurement"], word, "{}: its list row", entry.key);
            assert_eq!(record(&view, entry, &[]).unwrap()["against_measurement"], word, "{}: its match", entry.key);
        }
        assert_eq!(counted.into_iter().collect::<Vec<_>>(), [("met", 12), ("met in part", 3), ("missed", 19), ("not held to a measurement", 10)]);
        // No row of either list is without the word, but the solver compiled for tests only.
        assert_eq!(cards["cards"].as_array().unwrap().len(), INDEX.iter().filter(|entry| entry.key.contains('/')).count());
        for row in cards["cards"].as_array().unwrap() {
            assert!(row["against_measurement"].is_string(), "{row}");
            assert_eq!(row.as_object().unwrap().keys().collect::<Vec<_>>(), ["domain", "card", "against_measurement", "specimen"]);
        }
        for row in solvers["solvers"].as_array().unwrap() {
            if row["solver"] == crate::solver_probe::STAND_IN.name {
                assert!(row.get("against_measurement").is_none(), "{row}");
            } else {
                assert_eq!(row.as_object().unwrap().keys().collect::<Vec<_>>(), ["solver", "against_measurement", "summary"]);
            }
        }
        // And through the tool: a match gives the word before its phrase.
        let answer = find_in(&view, &json!({ "description": "rain fade on a microwave link" })).unwrap();
        assert_eq!(answer["matches"][0]["against_measurement"], "met");
        assert!(answer["matches"][0]["accuracy_against_measurement"].as_str().unwrap().starts_with("passed the pass marks"));
        // Each of the three tools says what the key is: accuracy, not fit.
        for said in [description().as_str(), AGAINST_MEASUREMENT_SAYS] {
            assert!(said.contains("against_measurement") && said.contains("\"met in part\"") && said.contains("accuracy and not fit"), "{said}");
        }
    }

    /// The kinds the answer itself gives as examples of a description that
    /// fits, and the example in the tool's description, each find a model.
    #[test]
    fn the_kinds_the_tool_gives_as_examples_find_their_models() {
        let about = about(31, 13);
        let examples = about.split("For a thing described by its kind (").nth(1).unwrap().split(')').next().unwrap();
        assert_eq!(examples, "a cell discharging, a capsule entering an atmosphere");
        assert_eq!(keys("a cell discharging"), ["energy/cell_18650pf", "energy/cell_2170_m80", "energy/cell_2170_model3"]);
        assert_eq!(keys("a capsule entering an atmosphere"), ["aerospace/capsule_stardust", "aerospace/capsule_insight"]);
        assert!(description().contains("such as \"a lithium cell discharging in the cold\""));
        assert_eq!(first_of("a lithium cell discharging in the cold").as_deref(), Some("energy/cell_2170_model3"));
    }

    /// Forms of one word are one word: a plural, "-ing", "-ed" with the
    /// consonant they doubled, "-ment", a form the endings do not join, and
    /// "re-" with or without its hyphen.
    #[test]
    fn forms_of_one_word_are_read_as_one_word() {
        for forms in [
            &["enter", "enters", "entering", "entered", "entry", "entries"][..],
            &["re-entry", "reentry", "re-entering", "reentered"],
            &["spring", "springs", "sprung"],
            &["bearing", "bearings"],
            &["speed", "speeds"],
            &["slip", "slips", "slipping", "slipped"],
            &["trip", "trips", "tripped", "tripping"],
            &["char", "chars", "charring", "charred"],
            &["reattach", "re-attach", "reattaches", "reattached", "reattachment"],
            &["ring", "rings", "ringing", "rang", "rung"],
            &["fall", "falls", "falling", "fell", "fallen"],
            &["lose", "loses", "losing", "lost", "loss", "losses"],
            &["discharge", "discharges", "discharging", "discharged"],
        ] {
            let read: Vec<Vec<String>> = forms.iter().map(|form| words(form)).collect();
            assert!(read.iter().all(|word| word.len() == 1 && *word == read[0]), "{forms:?}: {read:?}");
        }
        // A doubled letter that belongs to the word stays.
        assert_eq!(words("falling passing filled"), ["fall", "pass", "fill"]);
        // A hyphen parts two words, except after "re".
        assert_eq!(words("lithium-ion"), ["lithium", "ion"]);
        // A stop word in a term or in a description does not part a phrase.
        assert_eq!(telling_words("a mass on a spring"), telling_words("mass spring"));
    }

    /// A word of the description counts once for a model: a compound does
    /// not count again as its parts, and a phrase is not counted beside the
    /// word it holds.
    #[test]
    fn a_word_counts_once_for_a_model() {
        let entry = |key: &str| INDEX.iter().find(|entry| entry.key == key).unwrap();
        let solver = score(entry("tokamak_dt_beam_target"), &telling_words("deuterium-tritium"));
        assert_eq!((solver.score, solver.matched.as_slice()), (2 * STRONG, &["deuterium-tritium"][..]));
        let grid = score(entry("energy/grid_gb"), &telling_words("grid frequency"));
        assert_eq!((grid.score, grid.matched.as_slice()), (2 * STRONG, &["grid frequency"][..]));
        // A phrase and a heavier word inside it: the word at its weight, the rest of the phrase at the phrase's.
        let jet = score(entry("energy/tokamak_jet_dte1"), &telling_words("fusion power"));
        assert_eq!((jet.score, jet.matched.as_slice(), jet.strong, jet.plain), (STRONG + PLAIN, &["fusion", "fusion power"][..], true, 1));
    }

    /// A word that names a class of things is a term of the subject, not a
    /// name of the thing: a power reactor is not the research reactors, and
    /// a signal from orbit is not the laser-ranged satellite. Each is named
    /// as near, and not as a match.
    #[test]
    fn a_class_word_alone_does_not_match() {
        let view = View::full();
        for (description, near) in [
            ("a boiling-water power reactor at full power when the turbine trips: core power and steam pressure", "reactor_"),
            ("the nuclear reactor of a submarine at full power when a pump fails", "reactor_"),
            ("a television dish on a roof losing its signal from orbit in a downpour", "link_gothenburg_10010"),
            ("photographs of the Earth taken from orbit", ""),
        ] {
            let answer = find_in(&view, &json!({ "description": description, "limit": 5 })).unwrap();
            assert_eq!(answer["count"], 0, "{description}: {}", answer["matches"]);
            let nearest: Vec<&str> = answer["nearest_by_words"].as_array().unwrap().iter().filter_map(|row| row["card"].as_str()).collect();
            assert!(near.is_empty() || nearest.iter().any(|card| card.starts_with(near)), "{description}: {nearest:?}");
        }
        // The things themselves are still found by their kind.
        assert_eq!(first_of("a nuclear research reactor pulsed by a step in reactivity").as_deref(), Some("energy/reactor_triga_jsi"));
        assert_eq!(first_of("a satellite in orbit around the Earth").as_deref(), Some("aerospace/satellite_lageos1"));
    }

    /// Two models of one machine: the card is the pulse through time, the
    /// solver the fusion rate of one instant, and each comes first for its
    /// own description.
    #[test]
    fn the_tokamak_card_and_the_beam_target_solver_each_come_first_for_their_own_thing() {
        assert_eq!(
            keys("the JET tokamak at Culham in its deuterium-tritium campaign: plasma energy and fusion power through a pulse as the heating rises")[..2],
            ["energy/tokamak_jet_dte1", "tokamak_dt_beam_target"]
        );
        assert_eq!(
            keys("the neutron rate of one time slice of a deuterium-tritium pulse, from its profiles and its neutral beams")[..2],
            ["tokamak_dt_beam_target", "energy/tokamak_jet_dte1"]
        );
    }

    /// Things said without their usual name are found by a word a person
    /// uses for them: where the rig is, what the part is made of or called.
    #[test]
    fn a_thing_said_without_its_usual_name_is_found() {
        let table: &[(&str, &str)] = &[
            ("the gas side of a diaphragm pressure vessel on the Darmstadt hydraulic rig as oil is pushed in: nitrogen pressure and temperature", "energy/accumulator_sbo450"),
            ("an instrument on a sprung mount shaken at its base: does it run out of stroke", "signal/resonance"),
            ("a carbon and phenolic resin tile heated in an arc jet: how much chars and how much wears away", "ablator_slab"),
            ("where the air flow reattaches behind a step in a channel", "step_flow_armaly"),
            ("plain bearings with an oil film under load", "journal_bearing_sheffield"),
            ("a tyre slipping and skidding on an icy road", "materials/tire_eve_r19_ice"),
        ];
        for (description, first) in table {
            assert_eq!(first_of(description).as_deref(), Some(*first), "{description}: {:?}", keys(description));
        }
    }

    #[test]
    fn the_same_words_give_the_same_answer_and_a_view_hides_what_it_hides() {
        let request = json!({ "description": "a lithium cell discharging in the cold" });
        let once = serde_json::to_string(&find_in(&View::full(), &request).unwrap()).unwrap();
        let twice = serde_json::to_string(&find_in(&View::full(), &request).unwrap()).unwrap();
        assert_eq!(once, twice);
        let narrow = View::limited([("energy".to_string(), "thermal".to_string())], ["atom_dft".to_string()]);
        let hidden = find_in(&narrow, &request).unwrap();
        assert_eq!(hidden["count"], 0);
        assert!(hidden["none_fits"].as_str().unwrap().contains("everything this connection serves (1 card and 1 solver)"));
        let seen = find_in(&narrow, &json!({ "description": "an avionics cold plate under a heater" })).unwrap();
        assert_eq!(seen["matches"][0]["card"], "thermal");
    }

    #[test]
    fn a_request_out_of_shape_is_named() {
        let view = View::full();
        assert!(find_in(&view, &json!({})).unwrap_err().message().contains("'description'"));
        assert!(find_in(&view, &json!({ "description": 3 })).unwrap_err().message().contains("must be a string"));
        assert!(find_in(&view, &json!({ "description": "x", "limit": 9 })).unwrap_err().message().contains("'limit'"));
        assert!(find_in(&view, &json!({ "description": "x", "words": 1 })).unwrap_err().message().contains("'words'"));
        let long = "a".repeat(MAX_DESCRIPTION_CHARS + 1);
        assert!(find_in(&view, &json!({ "description": long })).unwrap_err().message().contains("MAX_DESCRIPTION_CHARS"));
    }
}
