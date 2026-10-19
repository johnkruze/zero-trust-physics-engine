//! `find_model`: from a plain description of a real thing to the served
//! models that fit it.
//!
//! No model is called and nothing is fetched: the description is matched
//! against a curated list of terms for each card and solver, written from
//! their own descriptions. The answer gives the best few, each with fit
//! first (the words that matched, and the one real thing the model is of
//! with its range) and then accuracy (what it was fitted on, what it was
//! compared with outside the fit and within what, and the gap), kept apart
//! so that a model with a gap is not read as a model that does not fit. A
//! model a newer one supersedes (`crate::succession`) is given after its
//! successor, under its own ID. When nothing fits the answer is
//! complete in itself: every card and solver was compared, here are the
//! nearest by words and why they are not it, and where requests for a model
//! go, said as what is so and not as an instruction. The same description
//! always gives the same answer.
//!
//! How a description is read: its words are taken in their common forms, so
//! that forms of one word are one word; a term is found where its words stand
//! together; and each word of the description counts once for a model, at
//! the weight of the heaviest term found on it.
//!
//! A description that names another real thing of a model's kind (helium for
//! the argon gas, the Phoenix lander for the InSight capsule, an LG cell for
//! the Panasonic and Tesla cells) is of that other thing: the model is not a
//! match for it, whatever words the two share, and is named as near with the
//! name that set it aside. A description that names the model's own thing as
//! well is of the model's thing.

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

/// The kind of evidence behind a model. `Met`, `Missed`, `InPart` and
/// `Calibrated` are the record's own account of a comparison with
/// measurement and are served alike, as "compared with measurement"; the
/// numbers, in the model's spec sheet (`crate::spec_sheet`), say how good
/// it is. `Measured` is the same word for a new entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Marks {
    /// Compared with measurement of a real object: the word for a new entry.
    #[allow(dead_code)]
    Measured,
    Met,
    Missed,
    InPart,
    /// A generic specimen: compared with no measurement.
    NotCompared,
    /// Compared with a reference calculation, not a measurement.
    Reference,
    /// Compared with the record of a real object after its structure or
    /// constants were chosen with that record open: calibration, not a blind
    /// test. Served beside the scored original (its row of
    /// `CALIBRATIONS`), never in its place: `generator_mmrtg_seebeck`, beside
    /// `generator_mmrtg_f1`.
    Calibrated,
}

impl Marks {
    /// The kind of evidence behind a model, in one of three words.
    /// `list_cards`, `list_solvers`, `describe_card`, `describe_solver` and
    /// `find_model` give it for each model as `against_measurement`.
    pub fn against_measurement(self) -> &'static str {
        match self {
            Marks::Measured | Marks::Met | Marks::Missed | Marks::InPart | Marks::Calibrated => crate::spec_sheet::COMPARED,
            Marks::NotCompared => crate::spec_sheet::SPECIMEN,
            Marks::Reference => crate::spec_sheet::REFERENCE,
        }
    }

    /// Compared with measurement of a real object.
    pub fn compared(self) -> bool {
        self.against_measurement() == crate::spec_sheet::COMPARED
    }
}

/// What `against_measurement` holds, said in the description of each tool
/// that returns it.
pub(crate) const AGAINST_MEASUREMENT_SAYS: &str = crate::spec_sheet::AGAINST_MEASUREMENT_SAYS;


pub struct Entry {
    /// `domain/card` for a card, the solver's name for a solver.
    pub key: &'static str,
    pub marks: Marks,
    /// One short phrase of the model's evidence: served as its accuracy
    /// where it has no spec sheet (a company specimen, a reference
    /// calculation). A model compared with measurement is described by its
    /// spec sheet (`crate::spec_sheet`) instead.
    pub marks_words: &'static str,
    /// Terms that name the thing itself.
    pub strong: &'static [&'static str],
    /// Terms of its subject.
    pub plain: &'static [&'static str],
    /// Terms shared with many things.
    pub weak: &'static [&'static str],
    /// Names of other real things of this model's kind: another gas, another
    /// planet's probe, another maker's cell. A description that names one is
    /// of that thing, so the model is not a match for it, whatever words the
    /// two share; it is named as near, with the name that set it aside. The
    /// list holds names of specific other things and no common noun that a
    /// description of the model's own thing uses in passing: what the thing
    /// is made of, what it is found in, what it is set against (air for
    /// argon, a lake for the sea, dry asphalt for ice, a molecule for an atom).
    pub other: &'static [&'static str],
    /// The names of this model's own thing. A description that holds one is
    /// of this thing, though it names another beside it ("argon, compared
    /// with air"). A name of its own that stands inside another thing's name
    /// does not count: "LAGEOS-2" is not LAGEOS-1, and "liquid argon" is not
    /// the gas.
    pub own: &'static [&'static str],
}

/// What the index says of a card that is one of the company's own specimens.
const OWN_SPECIMEN: &str = "company specimen";

/// What the accuracy of a company specimen says after its name: what its
/// numbers are.
const OWN_SPECIMEN_ACCOUNT: &str = "an object the company specified, every constant stated in its description; each step holds its conservation ledger to the stated limit";

/// The index: one entry for each served card and solver, in the engine's order.
pub const INDEX: &[Entry] = &[
    Entry { key: "materials/ti64_lattice", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["ti-6al-4v", "ti64", "titanium", "lattice"],
        plain: &["column", "strut", "printed metal", "additive", "elastic", "torque", "alloy"],
        weak: &["force", "temperature", "metal", "structure", "load"],
        other: &[],
        own: &[] },
    Entry { key: "energy/grid_node", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["fuse", "fuse link", "copper link"],
        plain: &["overcurrent", "busbar", "conductor heating", "surge", "wire heating", "copper"],
        weak: &["current", "voltage", "electrical", "melt"],
        other: &[],
        own: &[] },
    Entry { key: "energy/swing", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["synchronous machine", "swing equation", "rotor angle", "synchronous generator"],
        plain: &["60 hz", "generator", "turbine", "pole slip", "mechanical power", "rotor", "synchronism"],
        weak: &["grid", "frequency", "machine", "power"],
        other: &[],
        own: &[] },
    Entry { key: "energy/battery_sag", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["battery pack", "12s", "pack", "voltage sag", "undervoltage"],
        plain: &["battery", "lithium", "lithium-ion", "li-ion", "cutoff", "drone battery", "discharge", "sag"],
        weak: &["current", "voltage", "cell", "amps", "load", "drain"],
        other: &["lg", "samsung", "sony", "murata", "molicel", "a123", "catl", "byd", "panasonic", "tesla", "lead acid", "nimh", "nickel metal", "alkaline"],
        own: &[] },
    Entry { key: "energy/cell_18650pf", marks: Marks::Met, marks_words: "met its marks on all eleven discharges",
        strong: &["18650", "ncr18650pf", "panasonic"],
        plain: &["lithium", "lithium-ion", "li-ion", "cell", "battery", "discharge", "1c", "capacity"],
        weak: &["voltage", "current", "cylindrical", "drain"],
        other: &["lg", "samsung", "sony", "murata", "molicel", "a123", "catl", "byd", "tesla", "2170", "21700", "4680", "26650", "pouch", "prismatic", "lead acid", "nimh", "nickel metal", "alkaline", "lifepo4", "lfp"],
        own: &["ncr18650pf", "panasonic"] },
    Entry { key: "energy/thermal", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["cold plate", "heater", "avionics"],
        plain: &["heat sink", "thermal mass", "cooling", "warming", "bath", "heat", "flight computer"],
        weak: &["temperature", "power", "ambient", "electronics", "thermal", "plate", "coolant"],
        other: &[],
        own: &[] },
    Entry { key: "signal/resonance", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["oscillator", "vibration mount", "spring mount", "mass on a spring", "isolator"],
        plain: &["spring", "resonance", "10 hz", "shaking", "vibration", "mount", "damper", "damped"],
        weak: &["acceleration", "displacement", "mass", "stroke"],
        other: &[],
        own: &[] },
    Entry { key: "signal/tesseract", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["duffing", "gyroscope", "mems", "drive mode", "resonator"],
        plain: &["nonlinear oscillator", "rate sensor", "gyro", "sensed velocity", "hardening spring", "coriolis", "proof mass", "vibratory", "turn rate", "angular rate"],
        weak: &["acceleration", "vibration", "sensor", "amplitude"],
        other: &[],
        own: &[] },
    Entry { key: "materials/hydraulic_fluid", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["hydraulic", "mineral oil", "oil column", "hydraulic line"],
        plain: &["actuator", "compressible fluid", "bulk modulus", "commanded flow", "oil"],
        weak: &["pressure", "flow", "fluid", "column", "pipe"],
        other: &[],
        own: &[] },
    Entry { key: "materials/cable_stretch", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["dyneema", "tendon", "cable", "rope"],
        plain: &["stretch", "tension", "elongation", "winch", "line under load", "cord", "tether"],
        weak: &["force", "strain", "pull", "robot"],
        other: &[],
        own: &[] },
    Entry { key: "energy/generator_mmrtg_f1", marks: Marks::Missed, marks_words: "met its root mean square line (about 0.94 W against 1.154 W) and missed its final-year line (about 1.16 W high) on the held-back days 2,809 to 4,087, which it serves",
        strong: &["mmrtg", "rtg", "radioisotope", "curiosity", "plutonium", "thermoelectric generator"],
        plain: &["rover", "mars rover", "thermoelectric", "nuclear battery", "decay", "generator output", "mission day", "sol", "mars science laboratory"],
        weak: &["generator", "power", "mars", "electrical output", "years"],
        other: &["voyager", "cassini", "perseverance", "new horizons", "galileo", "ulysses", "viking", "pioneer 10", "pioneer 11", "apollo", "dragonfly"],
        own: &["curiosity"] },
    Entry { key: "aerospace/capsule_stardust", marks: Marks::Met, marks_words: "met all three pass lines; the comparison was not blind, as its validation says",
        strong: &["stardust", "sample return", "reentry", "entry capsule", "atmospheric entry"],
        plain: &["capsule", "atmosphere", "entry", "falling", "drogue", "parachute", "hypersonic", "deceleration", "heat shield", "returning from space", "meteor", "comet", "utah", "wild 2"],
        weak: &["earth", "mach", "altitude", "space", "g load", "descent"],
        other: &["apollo", "soyuz", "orion", "dragon", "shuttle", "genesis", "hayabusa", "osiris", "galileo", "huygens", "shenzhou", "starliner", "vostok", "phoenix", "perseverance", "viking", "pathfinder", "schiaparelli", "mars", "venus", "titan", "jupiter", "saturn", "moon", "lunar", "hayabusa2"],
        own: &["stardust"] },
    Entry { key: "aerospace/satellite_lageos1", marks: Marks::Met, marks_words: "met every pass line on 42 runs",
        strong: &["lageos", "satellite", "laser ranging"],
        plain: &["orbit", "spacecraft", "earth orbit", "gravity field", "orbital", "trajectory", "free flight", "ephemeris", "revolution"],
        weak: &["earth", "space", "sun", "moon", "tide", "position"],
        other: &["iss", "space station", "gps", "glonass", "galileo", "starlink", "hubble", "geostationary", "sputnik", "landsat"],
        own: &["lageos"] },
    Entry { key: "energy/reactor_spert3", marks: Marks::Missed, marks_words: "met five of eight pass lines; the comparison was not blind, as its validation says",
        strong: &["spert", "reactivity", "power excursion", "prompt critical"],
        plain: &["reactor", "burst", "transient", "fission", "e-core", "kinetics", "power pulse", "criticality", "step in reactivity"],
        weak: &["nuclear", "power", "peak", "core", "step", "cold"],
        other: &["spert i", "spert ii", "spert iv", "spert 1", "spert 2", "spert 4", "borax", "kewb", "godiva", "nsrr", "cabri", "chernobyl", "rbmk", "fukushima", "three mile island", "candu", "triga", "c-core", "sl-1", "ebr"],
        own: &["spert"] },
    Entry { key: "energy/cell_2170_m80", marks: Marks::Missed, marks_words: "only 27 of the 60 held-back runs met their marks",
        strong: &["2170", "tesla", "model 3", "m80"],
        plain: &["lithium", "lithium-ion", "li-ion", "cell", "battery", "discharge", "case temperature", "drive cycle"],
        weak: &["voltage", "current", "temperature", "electric car", "ev", "drain", "cold"],
        other: &["m448", "m1000", "lg", "samsung", "sony", "murata", "molicel", "a123", "catl", "byd", "18650", "ncr18650pf", "4680", "26650", "pouch", "prismatic", "lead acid", "nimh", "nickel metal", "alkaline", "lifepo4", "lfp"],
        own: &["m80"] },
    Entry { key: "signal/cantilever_eth", marks: Marks::Missed, marks_words: "met three of four pass lines; the comparison was not blind, as its validation says",
        strong: &["cantilever", "ring down", "ringdown", "ringing"],
        plain: &["beam", "strip", "aluminium", "aluminum", "bending mode", "damping", "vibrating", "clamped", "deflection", "free vibration", "eth", "flick", "pluck", "ruler", "tuning fork", "zurich", "zürich"],
        weak: &["vibration", "amplitude", "decay", "air", "oscillation"],
        other: &[],
        own: &[] },
    Entry { key: "energy/cell_2170_model3", marks: Marks::InPart, marks_words: "met all five conditions on one new cell and missed three of five on the other",
        strong: &["2170", "tesla", "model 3", "m448", "m1000"],
        plain: &["lithium", "lithium-ion", "li-ion", "cell", "battery", "discharge", "case temperature", "drive cycle", "cold", "hysteresis", "freezing"],
        weak: &["voltage", "current", "temperature", "electric car", "ev", "winter", "drain"],
        other: &["lg", "samsung", "sony", "murata", "molicel", "a123", "catl", "byd", "18650", "ncr18650pf", "4680", "26650", "pouch", "prismatic", "lead acid", "nimh", "nickel metal", "alkaline", "lifepo4", "lfp"],
        own: &["tesla", "model 3", "m448", "m1000", "m80"] },
    Entry { key: "materials/creep_cmsx6", marks: Marks::Met, marks_words: "the comparison passed on the six held-back tests",
        strong: &["creep", "cmsx-6", "cmsx6", "superalloy", "single crystal"],
        plain: &["nickel alloy", "turbine blade", "980", "high temperature", "tension", "rupture", "nickel"],
        weak: &["alloy", "strain", "hours", "load", "metal", "stress"],
        other: &["cmsx-4", "cmsx4", "cmsx-10", "cmsx-2", "inconel", "rene", "nimonic", "waspaloy", "hastelloy", "haynes", "steel", "copper", "solder", "concrete", "polymer", "plastic", "glacier"],
        own: &["cmsx-6", "cmsx6"] },
    Entry { key: "aerospace/capsule_insight", marks: Marks::InPart, marks_words: "met every pass line with the lift measured on the opened rows, and none of the first four with no lift; the comparison was not blind, as its validation says",
        strong: &["insight", "mars entry", "entry capsule", "atmospheric entry", "mars lander"],
        plain: &["capsule", "atmosphere", "entry", "falling", "parachute", "hypersonic", "deceleration", "mars", "lander", "aeroshell", "landing", "meteor"],
        weak: &["mach", "altitude", "space", "g load", "descent"],
        other: &["phoenix", "perseverance", "viking", "pathfinder", "schiaparelli", "beagle", "exomars", "tianwen", "zhurong", "msl", "mars science laboratory", "mars 2020", "curiosity rover", "apollo", "soyuz", "orion", "dragon", "shuttle", "genesis", "hayabusa", "osiris", "galileo", "huygens", "shenzhou", "starliner", "vostok", "stardust", "venus", "titan", "jupiter", "saturn", "moon", "lunar", "hayabusa2"],
        own: &["insight"] },
    Entry { key: "energy/reactor_spert3_fuel", marks: Marks::Missed, marks_words: "met 18 of 24 pass lines across the three groups; the comparison was not blind, as its validation says",
        strong: &["spert", "reactivity", "fuel temperature", "power excursion"],
        plain: &["reactor", "burst", "transient", "fission", "e-core", "inlet temperature", "coolant flow", "doppler", "power pulse", "transient rod", "ramp"],
        weak: &["nuclear", "power", "peak", "core", "fuel", "hot", "coolant", "flow", "inlet"],
        other: &["spert i", "spert ii", "spert iv", "spert 1", "spert 2", "spert 4", "borax", "kewb", "godiva", "nsrr", "cabri", "chernobyl", "rbmk", "fukushima", "three mile island", "candu", "triga", "c-core", "sl-1", "ebr"],
        own: &["spert"] },
    Entry { key: "aerospace/crater_ryugu_sci", marks: Marks::Missed, marks_words: "missed its pass line on Ryugu and met both laboratory lines; the comparison was not blind, as its validation says",
        strong: &["crater", "ryugu", "hayabusa2", "hayabusa", "impactor", "asteroid"],
        plain: &["impact", "regolith", "ejecta", "projectile", "excavation", "microgravity"],
        weak: &["radius", "sand", "copper", "hit", "surface"],
        other: &["dimorphos", "didymos", "dart", "deep impact", "tempel", "bennu", "itokawa", "eros", "vesta", "ceres", "meteor crater", "barringer", "arizona", "chicxulub", "moon", "lunar", "mars"],
        own: &["ryugu", "hayabusa2"] },
    Entry { key: "energy/reactor_spert3_twin", marks: Marks::Measured, marks_words: "its spec sheet gives the comparison and the gap",
        strong: &["spert iii", "spert 3", "digital twin", "hot standby", "operating power", "rod ejection", "reactor transient"],
        plain: &["spert", "reactor", "transient", "e-core", "burst", "fission", "power excursion", "transient rod", "rod worth", "clad temperature", "hot startup", "cold startup", "initial power", "uo2", "oxide fuel", "pressurized water", "test reactor", "twin"],
        weak: &["nuclear", "power", "peak", "core", "fuel", "hot", "coolant", "flow", "clad", "inlet", "pressure", "doppler"],
        other: &["spert i", "spert ii", "spert iv", "spert 1", "spert 2", "spert 4", "borax", "kewb", "godiva", "nsrr", "cabri", "chernobyl", "rbmk", "fukushima", "three mile island", "candu", "triga", "c-core", "sl-1", "ebr"],
        own: &["spert"] },
    Entry { key: "energy/reactor_spert3_ccore_twin", marks: Marks::Measured, marks_words: "its spec sheet gives the comparison and the gap",
        strong: &["subcooled boiling", "void feedback", "void reactivity", "c-19/52", "ccore twin"],
        plain: &["spert", "spert iii", "c-core", "reactor", "power excursion", "step insertion", "reactivity insertion", "flow", "void", "saturation temperature", "plate temperature", "highly enriched", "stainless steel clad", "plate core", "research reactor", "test reactor", "transient", "reactivity", "doppler", "moderator temperature", "peak power", "energy to peak", "pressurized", "psig", "gpm"],
        weak: &["pressure", "boiling", "nuclear", "fuel plate", "idaho", "ido-16918", "kinetics", "burst", "hot spot", "coolant"],
        other: &["triga", "e-core", "spert i", "borax", "tokamak", "fusion", "battery", "msre"],
        own: &["c-core", "subcooled", "twin"] },
    Entry { key: "energy/generator_mmrtg_seebeck", marks: Marks::Calibrated, marks_words: "within 0.00319 of 115.4 W by root mean square on days 0 to 4,087, the days its constants were fitted on; its blind forecast of the days after 4,087 is registered and waits for the record's next release",
        strong: &["mmrtg", "rtg", "radioisotope", "curiosity", "plutonium", "thermoelectric generator", "seebeck", "matched load"],
        plain: &["rover", "mars rover", "thermoelectric", "nuclear battery", "decay", "generator output", "mission day", "sol", "mars science laboratory", "thermocouple", "temperature drop", "calibrated"],
        weak: &["generator", "power", "mars", "electrical output", "years"],
        other: &["voyager", "cassini", "perseverance", "new horizons", "galileo", "ulysses", "viking", "pioneer 10", "pioneer 11", "apollo", "dragonfly"],
        own: &["curiosity"] },
    Entry { key: "aerospace/momentum_jwst_dihedral", marks: Marks::Missed, marks_words: "met four of five marks on the month of 15 July to 14 August 2023, forecast first, and missed the daily change of the momentum vector, 18 of 28 days against a line of 80 percent; on the next month, scored blind, three of five",
        strong: &["angular momentum", "reaction wheel", "momentum unload", "solar radiation pressure", "sunshield", "momentum management"],
        plain: &["dihedral", "attitude", "quaternion", "torque", "solar pressure", "roll", "boresight", "wheel desaturation", "desaturation", "momentum dump", "jwst", "james webb", "webb", "l2", "lagrange point"],
        weak: &["spacecraft", "observatory", "satellite", "sunlight", "wheel", "momentum", "space telescope", "slope"],
        other: &["kepler", "k2", "hubble", "tess", "iss", "control moment gyro", "magnetorquer", "cubesat"],
        own: &["sunshield", "reaction wheel"] },
    Entry { key: "aerospace/momentum_jwst_srp", marks: Marks::Missed, marks_words: "met four of five marks on the held-back month, 15 June to 14 July 2023, and missed the daily change of the momentum vector, 14 of 28 days against a line of 80 percent; scored blind on the two months after it, it met two of five and then three of five, 9 of 15 marks in its three blind months",
        strong: &["angular momentum", "reaction wheel", "momentum unload", "solar radiation pressure", "sunshield", "momentum management"],
        plain: &["attitude", "quaternion", "torque", "solar pressure", "wheel desaturation", "desaturation", "momentum dump", "jwst", "james webb", "webb", "l2", "lagrange point"],
        weak: &["spacecraft", "observatory", "satellite", "sunlight", "wheel", "momentum", "space telescope"],
        other: &["kepler", "k2", "hubble", "tess", "iss", "control moment gyro", "magnetorquer", "cubesat"],
        own: &["sunshield", "reaction wheel"] },
    Entry { key: "signal/link_gothenburg_10010", marks: Marks::Met, marks_words: "met all three pass lines",
        strong: &["microwave link", "rain attenuation", "rain fade", "openmrg", "gothenburg"],
        plain: &["rain", "32 ghz", "path loss", "radio link", "backhaul", "wet antenna", "attenuation", "millimetre wave", "microwave", "rainfall", "downpour"],
        weak: &["link", "signal", "weather", "antenna", "db", "storm"],
        other: &["satellite", "wifi", "wi-fi", "fibre", "fiber", "optical", "laser", "lidar"],
        own: &["gothenburg", "openmrg"] },
    Entry { key: "energy/grid_gb", marks: Marks::Met, marks_words: "met all seven pass lines on the 12 held-back trips; the comparison was not blind, as its validation says",
        strong: &["grid frequency", "power grid", "interconnector", "great britain", "transmission system", "national grid", "mains frequency", "system frequency"],
        plain: &["grid", "frequency", "50 hz", "inertia", "trip", "loss of infeed", "blackout", "losing a link", "electricity system", "outage", "generation loss", "frequency drop", "mains", "power cable", "undersea cable", "subsea cable", "infeed", "import", "nadir", "power station", "power cut"],
        weak: &["power", "loss", "demand", "link", "britain", "british", "uk", "england", "scotland", "wales", "france", "electricity", "mw", "fail", "sudden", "suddenly"],
        other: &["texas", "ercot", "california", "caiso", "pjm", "australia", "australian", "japan", "india", "china", "new zealand", "eastern interconnection", "western interconnection", "america", "american", "60 hz", "japanese", "indian", "chinese"],
        own: &["great britain", "britain", "british", "national grid", "gb"] },
    Entry { key: "aerospace/solar_system", marks: Marks::InPart, marks_words: "two files met their lines, two missed and two comparisons were not made; the comparison was not blind, as its validation says",
        strong: &["solar system", "planet", "n-body", "ephemeris", "earth to mars", "mars from the earth"],
        plain: &["sun", "mercury", "venus", "jupiter", "saturn", "moon", "planetary", "gravitation", "barycentre", "mars orbit", "perihelion", "uranus", "neptune"],
        weak: &["earth", "orbit", "space", "gravity", "years", "mars"],
        other: &["pluto", "exoplanet", "galaxy", "trappist", "proxima", "alpha centauri", "halley"],
        own: &[] },
    Entry { key: "materials/tire_eve_r19_ice", marks: Marks::Met, marks_words: "met all five pass lines",
        strong: &["tyre", "tire", "ice", "traction"],
        plain: &["slip", "friction", "wheel", "icy road", "all-season", "grip", "braking", "skid", "winter road"],
        weak: &["car", "road", "rolling", "load", "pressure", "cold"],
        other: &["formula 1", "f1", "racing", "slick", "truck", "lorry", "aircraft", "bicycle", "motorcycle", "tractor"],
        own: &[] },
    Entry { key: "aerospace/los_jwst_fine_guide", marks: Marks::Met, marks_words: "met all five marks on both axes",
        strong: &["jwst", "james webb", "webb", "fine guidance", "pointing"],
        plain: &["line of sight", "telescope", "jitter", "guide star", "kalman", "space telescope", "centroid", "pointing stability"],
        weak: &["sensor", "space", "star", "tracking", "filter"],
        other: &["hubble", "hst", "spitzer", "kepler", "chandra", "euclid", "gaia", "herschel", "tess", "keck", "vlt", "subaru", "backyard", "amateur", "binoculars"],
        own: &["jwst", "james webb", "webb"] },
    Entry { key: "energy/tokamak_jet_dte1", marks: Marks::Missed, marks_words: "met three of four marks on the primary pulse; the comparison was not blind, as its validation says",
        strong: &["tokamak", "jet", "joint european torus", "deuterium-tritium", "h-mode", "plasma", "fusion"],
        plain: &["deuterium", "tritium", "confinement", "stored energy", "fusion power", "neutral beam", "dte1", "elmy", "culham", "d-t", "thermal energy", "plasma current", "toroidal field", "heating power"],
        weak: &["energy", "pulse", "heating", "reactor", "magnetic"],
        other: &["diii-d", "iter", "asdex", "kstar", "jt-60", "alcator", "c-mod", "nstx", "mast", "tore supra", "sparc", "tftr", "stellarator", "lhd", "w7-x", "wendelstein", "nif", "national ignition facility", "laser", "inertial", "z-pinch", "z machine", "boron", "p-b11", "pb11", "aneutronic", "arc jet", "jet engine", "jet stream", "jet fuel", "jet aircraft", "water jet"],
        own: &["jet", "joint european torus", "dte1"] },
    Entry { key: "materials/fingerpad_p02_glass", marks: Marks::Missed, marks_words: "met three of twelve pass lines",
        strong: &["finger", "fingertip", "finger pad", "fingerpad", "skin"],
        plain: &["glass", "twist", "touch", "slip torque", "friction", "touchscreen", "grip", "pressed"],
        weak: &["slip", "torque", "human", "contact", "hand"],
        other: &[],
        own: &[] },
    Entry { key: "energy/accumulator_sbo450", marks: Marks::Missed, marks_words: "met two of six pass lines",
        strong: &["accumulator", "hydac", "nitrogen charge", "diaphragm accumulator", "gas spring"],
        plain: &["hydraulic accumulator", "nitrogen", "piston", "gas pressure", "precharge", "compressed gas", "bladder", "diaphragm", "darmstadt"],
        weak: &["pressure", "oil", "gas", "hydraulic", "stiffness", "compression"],
        other: &[],
        own: &[] },
    Entry { key: "energy/reactor_triga_jsi", marks: Marks::Missed, marks_words: "met two of six pass lines on the held-back rows; the comparison was not blind, as its validation says",
        strong: &["triga", "pulse mode", "reactivity", "research reactor"],
        plain: &["reactor", "pulse", "fission", "scram", "transient rod", "ljubljana", "fuel temperature", "power pulse"],
        weak: &["nuclear", "power", "peak", "core", "fuel", "rod"],
        other: &["spert", "borax", "kewb", "godiva", "nsrr", "cabri", "chernobyl", "rbmk", "fukushima", "three mile island", "candu", "acrr", "mainz", "vienna", "oregon", "pavia", "sl-1", "ebr"],
        own: &["ljubljana", "jsi", "jozef stefan", "jožef stefan"] },
    Entry { key: "energy/reactor_spert3_ccore", marks: Marks::Missed, marks_words: "missed all three pass lines; the comparison was not blind, as its validation says",
        strong: &["spert", "c-core", "reactivity", "plate fuel"],
        plain: &["reactor", "transient", "fission", "plate temperature", "stainless", "step transient", "power excursion", "highly enriched"],
        weak: &["nuclear", "power", "peak", "core", "plate", "pressure"],
        other: &["spert i", "spert ii", "spert iv", "spert 1", "spert 2", "spert 4", "borax", "kewb", "godiva", "nsrr", "cabri", "chernobyl", "rbmk", "fukushima", "three mile island", "candu", "triga", "e-core", "sl-1", "ebr"],
        own: &["spert"] },
    Entry { key: "energy/runaway_18650_lco", marks: Marks::Missed, marks_words: "met 8 of 23 pass lines; it self-heats and runs away early",
        strong: &["thermal runaway", "runaway", "licoo2", "lithium cobalt oxide", "oven test", "accelerating rate calorimetry", "accelerating rate calorimeter", "self-heating", "hatchard"],
        plain: &["18650", "oven", "abuse", "calorimeter", "calorimetry", "exothermic", "decomposition", "sei", "electrolyte", "cathode", "anode", "venting", "fire", "heat-wait-seek", "lco"],
        weak: &["cell", "battery", "lithium", "lithium-ion", "li-ion", "heat", "temperature", "safety", "heater"],
        other: &["ncr18650pf", "panasonic", "2170", "21700", "4680", "26650", "pouch", "prismatic", "lifepo4", "lfp", "nca", "nmc", "lmo", "sodium-ion", "lead acid", "nimh"],
        own: &["licoo2", "lco", "lithium cobalt oxide", "hatchard"] },
    Entry { key: "materials/grasp_lift_glass_4tu", marks: Marks::Missed, marks_words: "met one of four pass lines",
        strong: &["grasp", "two-finger grip", "precision grip", "pinch grip", "lifted object", "grip force", "pick up", "pick and place"],
        plain: &["lift", "grip", "slippage", "dexterity", "dexterous", "held object", "incipient slip", "gross slip", "stick ratio"],
        weak: &["slip", "object", "friction", "hand", "robot", "finger", "glass", "pad", "weight", "hold"],
        other: &[],
        own: &[] },
    Entry { key: "energy/tokamak_vde_mast", marks: Marks::InPart, marks_words: "met one of three pass lines on shot 15045; the comparison was not blind, as its validation says",
        strong: &["vertical displacement event", "vertical displacement", "vde", "vertical instability", "vertical stability", "mast", "mega amp spherical tokamak"],
        plain: &["tokamak", "spherical tokamak", "plasma", "vertical position", "growth rate", "eddy current", "passive structure", "vacuum vessel", "elongation", "elongated", "decay index", "stability margin", "vertical control", "radial field coil", "halo current", "disruption", "culham"],
        weak: &["magnetic", "fusion", "coil", "position", "control", "wall"],
        other: &["jet", "joint european torus", "diii-d", "iter", "asdex", "kstar", "jt-60", "alcator", "c-mod", "nstx", "tcv", "sparc", "tftr", "mast-u", "mast upgrade", "stellarator", "lhd", "w7-x", "wendelstein", "nif", "national ignition facility"],
        own: &["mast", "mega amp spherical tokamak", "15045"] },
    Entry { key: "energy/reactor_msre_pump", marks: Marks::Measured, marks_words: "its spec sheet gives the comparison and the gap",
        strong: &["msre", "molten salt reactor", "molten salt", "circulating fuel", "fuel salt", "fuel pump", "pump coast-down", "pump coastdown", "pump start-up", "pump startup"],
        plain: &["coast-down", "coastdown", "start-up", "startup", "delayed neutron", "precursor", "circulation", "reactivity", "rod compensation", "oak ridge", "ornl", "fluoride salt", "loss of flow", "pump trip", "zero power"],
        weak: &["reactor", "pump", "nuclear", "fuel", "flow", "transient", "rod", "core", "salt"],
        other: &["triga", "spert", "borax", "kewb", "godiva", "candu", "rbmk", "chernobyl", "fukushima", "three mile island", "sodium", "lead-cooled", "pebble", "hteg", "aircraft reactor", "msbr", "thorium"],
        own: &["msre", "molten salt"] },
    Entry { key: "atom_dft", marks: Marks::Reference, marks_words: "the NIST spin-unpolarized local-density tables, atomic number 1 to 18, within 1e-5 hartree; it is the local-density ground state, not the measured energy of the atom",
        strong: &["atom", "dft", "density functional", "electronic structure", "ground state"],
        plain: &["hydrogen", "helium", "neon", "argon atom", "orbital", "eigenvalue", "lda", "electron", "quantum", "total energy", "atomic number", "carbon", "oxygen"],
        weak: &["energy", "element", "chemistry", "nucleus"],
        other: &["protein", "potassium", "calcium", "titanium", "iron", "nickel", "copper", "zinc", "silver", "gold", "krypton", "xenon", "uranium", "plutonium"],
        own: &[] },
    Entry { key: "ablator_slab", marks: Marks::Missed, marks_words: "its wear line passed and its hottest-face-temperature line missed on the average; the comparison was not blind, as its validation says",
        strong: &["heat shield", "ablator", "ablation", "pica", "tacot", "charring"],
        plain: &["arc jet", "thermal protection", "recession", "pyrolysis", "entry heating", "slab", "heat flux", "ablative", "phenolic", "resin"],
        weak: &["heating", "temperature", "wear", "stardust", "material", "reentry"],
        other: &["galileo", "jupiter", "apollo", "avcoat", "shuttle", "orion", "dragon", "pica-x", "sla-561", "huygens", "titan", "venus", "msl", "mars science laboratory", "hayabusa", "genesis", "hayabusa2"],
        own: &["pica", "tacot"] },
    Entry { key: "journal_bearing_sheffield", marks: Marks::Met, marks_words: "met all three pass lines on the sixteen held-back runs",
        strong: &["journal bearing", "bearing", "oil film", "sommerfeld"],
        plain: &["lubrication", "eccentricity", "shaft", "hydrodynamic", "film thickness", "sheffield", "journal", "lubricated", "plain bearing", "oil-lubricated"],
        weak: &["oil", "load", "speed", "rpm", "friction", "rotating"],
        other: &["ball bearing", "roller bearing", "needle bearing", "air bearing", "magnetic bearing", "foil bearing", "thrust bearing", "tilting pad"],
        own: &["sheffield"] },
    Entry { key: "propeller_apc_10x7e", marks: Marks::Missed, marks_words: "missed both pass lines",
        strong: &["propeller", "apc", "blade element", "prop"],
        plain: &["thrust", "shaft power", "drone", "rotor", "rev/min", "airspeed", "uav", "quadcopter", "multirotor"],
        weak: &["rpm", "air", "blade", "motor", "aircraft", "power"],
        other: &["ship", "marine", "boat", "submarine", "hull", "knots", "bronze", "outboard", "helicopter", "wind turbine", "turboprop"],
        own: &["apc"] },
    Entry { key: "modulus_bam_s355", marks: Marks::Missed, marks_words: "missed one pass line (P1) and met every other",
        strong: &["young's modulus", "youngs modulus", "shear modulus", "s355", "structural steel", "elastic modulus"],
        plain: &["steel", "bar", "resonant frequency", "impulse excitation", "ringing bar", "stiffness", "bam", "modulus", "poisson"],
        weak: &["frequency", "metal", "elastic", "sheet", "rings"],
        other: &["inconel", "nickel", "aluminium", "aluminum", "titanium", "copper", "brass", "bronze", "tungsten", "magnesium", "glass", "ceramic", "concrete", "wood", "timber", "polymer", "plastic", "rubber", "stainless", "cast iron", "s235", "s275", "s460"],
        own: &["s355", "bam"] },
    Entry { key: "seawater_column", marks: Marks::Met, marks_words: "met all three pass lines",
        strong: &["seawater", "ocean", "sound speed", "ctd", "teos-10", "sea"],
        plain: &["salinity", "depth", "density", "water column", "cast", "underwater", "sonar", "echo sounder", "oceanography", "speed of sound in water"],
        weak: &["water", "pressure", "temperature", "marine", "sound"],
        other: &["freshwater", "fresh water", "tap water", "distilled", "dead sea"],
        own: &[] },
    Entry { key: "argon_pair_gas", marks: Marks::Met, marks_words: "met all five pass lines; the comparison was not blind, as its validation says",
        strong: &["argon", "lennard-jones", "virial", "pair potential", "noble gas"],
        plain: &["speed of sound", "viscosity", "gas", "dilute gas", "second virial", "thermal conductivity", "low density"],
        weak: &["pressure", "temperature", "sound", "kinetic theory"],
        other: &["helium", "neon", "krypton", "xenon", "radon", "nitrogen", "oxygen", "hydrogen", "methane", "carbon dioxide", "co2", "steam", "water vapour", "water vapor", "ammonia", "liquid argon", "sulfur hexafluoride", "sf6", "propane", "butane", "chlorine", "fluorine", "ozone", "natural gas", "refrigerant"],
        own: &["argon"] },
    Entry { key: "fusion_pb11_lhd", marks: Marks::InPart, marks_words: "met one of four pass lines against the measured alpha count rates of Ogawa et al. (2024) and the published calculation of Ogawa et al. (2022)",
        strong: &["proton-boron", "p-b11", "pb11", "boron", "lhd", "large helical device", "aneutronic"],
        plain: &["fusion", "neutral beam", "alpha", "hydrogen beam", "stellarator", "helical", "plasma"],
        weak: &["beam", "reaction rate", "energy", "kev"],
        other: &["tokamak", "jet", "tftr", "deuterium", "tritium", "diii-d", "iter", "asdex", "kstar", "jt-60", "alcator", "c-mod", "nstx", "mast", "tore supra", "sparc", "w7-x", "wendelstein", "nif", "national ignition facility", "laser", "inertial", "z-pinch", "z machine"],
        own: &["lhd", "large helical device"] },
    Entry { key: "fusion_pb11_thermal", marks: Marks::Reference, marks_words: "the published calculations of Ochs et al. (2022) and Putvinski et al. (2019) and the research's own plain model, ion temperatures 200 to 450 keV: 7 of 8 compared quantities within their targets, synchrotron over fusion power 7.9 percent off against a 5 percent target; at 300 keV the electron temperature 159.33 keV against a published 163.0",
        strong: &["proton-boron", "p-b11", "pb11", "boron", "aneutronic", "bremsstrahlung"],
        plain: &["fusion", "thermal plasma", "energy balance", "ignition", "synchrotron", "plasma", "electron temperature", "ion temperature", "fusion power"],
        weak: &["reaction rate", "energy", "kev", "radiation"],
        other: &["tokamak", "jet", "tftr", "deuterium", "tritium", "diii-d", "iter", "asdex", "kstar", "jt-60", "alcator", "c-mod", "nstx", "mast", "tore supra", "sparc", "w7-x", "wendelstein", "nif", "national ignition facility", "laser", "inertial", "z-pinch", "z machine"],
        own: &[] },
    Entry { key: "step_flow_armaly", marks: Marks::Met, marks_words: "met all three pass lines on the held-back profiles",
        strong: &["backward-facing step", "step flow", "reattachment", "recirculation", "armaly"],
        plain: &["laminar", "separation", "channel flow", "separated flow", "duct", "navier-stokes", "cfd", "expansion"],
        weak: &["air", "flow", "reynolds", "velocity", "channel"],
        other: &["forward-facing step", "cavity", "cylinder", "airfoil", "aerofoil"],
        own: &["armaly"] },
    Entry { key: "culture_trichoderma_ambr250", marks: Marks::Missed, marks_words: "missed three of six pass lines",
        strong: &["fed-batch", "trichoderma", "fermentation", "bioreactor", "culture", "mucic acid"],
        plain: &["fungus", "galacturonate", "biomass", "ambr", "bioprocess", "fermenter", "microbial", "feed", "broth", "cell culture"],
        weak: &["growth", "acid", "sugar", "vessel", "ph", "biology"],
        other: &["aspergillus", "penicillium", "yeast", "saccharomyces", "pichia", "coli", "escherichia", "bacillus", "streptomyces", "corynebacterium", "cho", "mammalian", "hybridoma", "citric acid", "penicillin", "ethanol", "insulin", "antibody"],
        own: &["trichoderma", "ambr250", "ambr", "yeast extract"] },
    Entry { key: "tokamak_dt_beam_target", marks: Marks::Reference, marks_words: "TRANSP's fusion power from the measured profiles of 34 TFTR slices outside any fit, the median ratio 1.022 and 30 of 34 within 0.75 to 1.25; against four measured values +7, +21, -11 and about +8 percent",
        strong: &["tokamak", "deuterium-tritium", "fusion rate", "beam-target", "tftr", "jet", "joint european torus", "fusion"],
        plain: &["deuterium", "tritium", "neutron rate", "neutron", "neutral beam", "plasma", "fusion power", "d-t", "dt", "time slice", "culham", "princeton", "shell", "electron temperature", "ion temperature", "electron density", "steady state"],
        weak: &["beam", "reaction rate", "profile", "energy", "pulse"],
        other: &["diii-d", "iter", "asdex", "kstar", "jt-60", "alcator", "c-mod", "nstx", "mast", "tore supra", "sparc", "stellarator", "lhd", "w7-x", "wendelstein", "nif", "national ignition facility", "laser", "inertial", "z-pinch", "z machine", "boron", "p-b11", "pb11", "aneutronic", "arc jet", "jet engine", "jet stream", "jet fuel", "jet aircraft", "water jet"],
        own: &["jet", "tftr", "joint european torus"] },
    Entry { key: "pipe_flow_superpipe", marks: Marks::Missed, marks_words: "missed one of three pass lines (centreline, on one profile)",
        strong: &["pipe flow", "superpipe", "friction factor", "turbulent pipe", "pipe"],
        plain: &["turbulent", "turbulence", "smooth wall", "pressure drop", "velocity profile", "moody", "darcy", "duct", "mixing length", "princeton"],
        weak: &["air", "flow", "reynolds", "velocity", "wall", "tube"],
        other: &["blood", "artery", "pipeline", "sewer"],
        own: &["superpipe"] },
    Entry { key: "fuel_rod_triga_jsi", marks: Marks::Measured, marks_words: "its spec sheet gives the comparison and the gap",
        strong: &["fuel element", "fuel rod", "element 6753", "instrumented element", "fuel thermocouple"],
        plain: &["triga", "fuel temperature", "u-zrh", "uranium zirconium hydride", "clad", "thermocouple"],
        weak: &["pulse", "jsi", "temperature", "heat", "conduction", "pool", "fuel"],
        other: &["spert", "msre", "molten salt", "pwr", "bwr", "candu", "magnox", "oxide fuel", "uo2"],
        own: &["element 6753", "fuel_rod_triga_jsi"] },
    Entry { key: "culture_trichoderma_atp", marks: Marks::Missed, marks_words: "missed three of six pass lines on the four held-back cultures at pH 3.5 and 3.0",
        strong: &["fed-batch", "trichoderma", "fermentation", "bioreactor", "culture", "mucic acid"],
        plain: &["fungus", "galacturonate", "biomass", "ambr", "bioprocess", "fermenter", "microbial", "feed", "broth", "cell culture", "ph"],
        weak: &["growth", "acid", "sugar", "vessel", "biology", "energy"],
        other: &["aspergillus", "penicillium", "yeast", "saccharomyces", "pichia", "coli", "escherichia", "bacillus", "streptomyces", "corynebacterium", "cho", "mammalian", "hybridoma", "citric acid", "penicillin", "ethanol", "insulin", "antibody"],
        own: &["trichoderma", "ambr250", "ambr", "yeast extract"] },
    Entry { key: "junction_nist_b7_s2", marks: Marks::Missed, marks_words: "missed one of three pass lines (voltage, 11.84 percent against 5)",
        strong: &["josephson junction", "josephson", "superconducting junction", "tunnel junction", "shapiro step", "nb/a-si/nb", "sis junction"],
        plain: &["superconductor", "superconducting", "niobium", "amorphous silicon", "critical current", "current-voltage", "i-v curve", "iv curve", "hysteresis", "flux quantum", "microwave drive", "rcsj", "plasma frequency", "gap voltage", "nist"],
        weak: &["junction", "voltage", "current", "microwave", "barrier", "cryogenic", "phase"],
        other: &["transmon", "al/alox/al", "cd3as2", "nbn", "mgb2", "ybco", "yba2cu3o7"],
        own: &["nist", "d220831b", "nb/a-si/nb", "niobium"] },
    Entry { key: "water_hammer_oil_line", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["water hammer", "fluid hammer", "hydraulic hammer", "joukowsky", "pressure surge", "hydraulic shock"],
        plain: &["valve closure", "valve closes", "column separation", "cavitation", "entrained air", "aerated oil", "wave speed", "hydraulic line", "steel line", "oil line", "pressure spike", "surge", "transient"],
        weak: &["hydraulic", "pressure", "valve", "oil", "pipe", "line", "tube", "flow"],
        other: &[],
        own: &[] },
    Entry { key: "orifice_air_chamber", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["iso 6358", "sonic conductance", "choked flow", "pneumatic valve", "orifice"],
        plain: &["critical pressure ratio", "air chamber", "pneumatic cylinder", "compressed air", "charging", "venting", "solenoid valve", "pneumatic", "chamber"],
        weak: &["air", "valve", "pressure", "tank", "flow", "gas", "fill"],
        other: &[],
        own: &[] },
    Entry { key: "pneumatic_hose_resonance", marks: Marks::NotCompared, marks_words: OWN_SPECIMEN,
        strong: &["pneumatic line", "pneumatic hose", "pneumatic tube", "line resonance", "transmission line"],
        plain: &["resonance", "hose", "tubing", "frequency response", "acoustic", "pneumatic", "dead volume", "damping ratio", "natural frequency"],
        weak: &["air", "tube", "frequency", "pressure", "line", "volume"],
        other: &[],
        own: &[] },
    Entry { key: "blast_wave_tnt_air", marks: Marks::Missed, marks_words: "missed all three lines on the propane/oxygen charges compared; the comparison was not blind, as its validation says",
        strong: &["blast wave", "blast overpressure", "overpressure", "explosion", "explosive", "tnt", "friedlander", "kingery-bulmash", "airblast", "air blast"],
        plain: &["shock wave", "charge", "detonation", "blast", "impulse", "stand-off", "standoff", "safe distance", "scaled distance", "hopkinson", "sachs", "sedov", "point blast", "surface burst", "free air", "c-4", "pe4", "semtex", "propane"],
        weak: &["pressure", "shock", "wave", "ground", "air"],
        other: &["nuclear", "underwater", "buried", "landmine", "dust explosion", "vapour cloud", "vapor cloud", "sonic boom", "volcano", "meteor", "tunnel"],
        own: &["tnt"] },
    Entry { key: "stress_plate_hole_s235", marks: Marks::Missed, marks_words: "missed one of three pass lines (the strain near the hole, by 0.6 points) and met the other two; the comparison was not blind, as its validation says",
        strong: &["plate with a hole", "hole in a plate", "hole in a steel plate", "steel plate with a hole", "open hole", "stress concentration", "notched plate", "perforated plate", "kirsch", "s235"],
        plain: &["stress field", "stress tensor", "displacement field", "digital image correlation", "dic", "von mises", "yield", "yielded zone", "plastic zone", "plasticity", "plane stress", "tensile bar", "tensile specimen", "notch", "finite element", "fem", "net section"],
        weak: &["steel", "stress", "strain", "tension", "hole", "plate", "elastic", "sheet", "bar"],
        other: &["s355", "s275", "s460", "aluminium", "aluminum", "titanium", "inconel", "composite", "cfrp", "carbon fibre", "carbon fiber", "concrete"],
        own: &["s235", "clausthal"] },
    Entry { key: "stress_plate_hole_s235_elastic", marks: Marks::Calibrated, marks_words: "after the fact, on stress_plate_hole_s235's three pass lines at 106.26 MPa, met all three, the strain near the hole within 1.4 percent where stress_plate_hole_s235 missed by 0.6 points",
        strong: &["plate with a hole", "hole in a plate", "hole in a steel plate", "steel plate with a hole", "open hole", "stress concentration", "notched plate", "perforated plate", "kirsch", "s235", "linear elastic", "elastic field"],
        plain: &["stress field", "stress tensor", "displacement field", "digital image correlation", "dic", "plane stress", "hooke", "tensile bar", "tensile specimen", "notch", "finite element", "fem", "net section"],
        weak: &["steel", "stress", "strain", "tension", "hole", "plate", "elastic", "sheet", "bar"],
        other: &["s355", "s275", "s460", "aluminium", "aluminum", "titanium", "inconel", "composite", "cfrp", "carbon fibre", "carbon fiber", "concrete"],
        own: &["s235", "clausthal"] },
    Entry { key: "icf_hotspot_burn", marks: Marks::Missed, marks_words: "met no shot and three marks of twenty-four on seven published NIF shots; the comparison was not blind, as its validation says",
        strong: &["inertial confinement", "inertial fusion", "icf", "nif", "national ignition facility", "hot spot", "hotspot", "laser fusion", "ignition"],
        plain: &["fusion", "deuterium-tritium", "d-t", "dt", "alpha heating", "burn", "yield", "lawson", "burn-up", "stagnation", "implosion", "capsule", "hohlraum", "laser", "inertial", "livermore", "target gain"],
        weak: &["plasma", "energy", "kev", "neutron", "reaction rate"],
        other: &["tokamak", "jet", "tftr", "diii-d", "iter", "asdex", "kstar", "jt-60", "stellarator", "lhd", "w7-x", "wendelstein", "sparc", "boron", "p-b11", "pb11", "aneutronic", "z-pinch", "z machine", "magnetized liner", "maglif", "jet engine", "jet stream", "car ignition", "ignition coil", "spark plug"],
        own: &["nif", "national ignition facility"] },
    Entry { key: "ln2_line_dlr_fttf", marks: LN2_LINE_MARKS, marks_words: LN2_LINE_MARKS_WORDS,
        strong: &["liquid nitrogen", "cryogenic", "ln2", "rocket feed line", "feed line", "lampoldshausen", "fluid transient test facility"],
        plain: &["fluid hammer", "pressure surge", "cavitation", "column separation", "valve closure", "cryogenic line", "rocket engine", "propellant line", "joukowsky", "surge"],
        weak: &["nitrogen", "pipe", "line", "valve", "pressure", "tube", "transient"],
        other: &[],
        own: &["fttf", "dlr"] },
    Entry { key: "plasma_sheath_ram_c2", marks: Marks::Missed, marks_words: "missed one of two pass lines (within a factor of 2 on one of three reflectometer onsets, against two: met at 70 km, 3.3 and 2.9 times at 77.7 and 80 km); the comparison was not blind, as its validation says",
        strong: &["plasma sheath", "radio blackout", "communications blackout", "blackout", "black out", "ram c-ii", "ram c", "reentry plasma", "plasma frequency"],
        plain: &["reentry", "hypersonic", "shock layer", "electron density", "ionization", "ionisation", "reflectometer", "glide vehicle", "hgv", "s-band", "x-band", "c-band", "gps", "telemetry", "stagnation point", "blunt cone", "sphere-cone", "dunn", "kang", "langley"],
        weak: &["plasma", "shock", "electron", "radio", "signal", "air", "nose", "cone", "atmosphere", "altitude", "velocity"],
        other: &["tokamak", "fusion", "laser", "inertial", "magnetic", "lightning", "arc jet", "plasma torch", "ionosphere", "mars", "venus", "titan"],
        own: &["ram c-ii", "ram c"] },
];

/// A model calibrated to the record: the scored original it stands beside,
/// and what of the record was seen before its structure or
/// constants were chosen. Each row is its registration's `calibration`
/// (`kit/forecasts/<model>.json`, written by `kit/forecast.py calibrate`),
/// word for word; `describe_card` and `describe_solver` give both where they
/// give the model's word and its record.
pub(crate) struct Calibration {
    /// The model's name as it is served: a card's name, or a solver's.
    pub model: &'static str,
    /// The scored original it stands beside.
    pub companion_to: &'static str,
    /// One sentence: what of the record was seen before the structure or
    /// constants were chosen.
    pub not_blind: &'static str,
}

/// One row for each model with a calibration.
pub(crate) const CALIBRATIONS: &[Calibration] = &[
    Calibration { model: "generator_mmrtg_seebeck", companion_to: "generator_mmrtg_f1", not_blind: "Every published day of the record, mission days 0 to 4,087, was open: days 0 to 2,060 had fitted generator_mmrtg_f1, days 2,061 to 2,808 had chosen its structure, and days 2,809 to 4,087, held back from it, had been compared with it once on 2026-10-06, where it missed its final-365 line by 0.000094 of 115.4 W and this structure, with constants fixed on days 0 to 2,060, gave after the fact a root mean square error of 0.00507 and a final-365 mean of +0.00468, before it was preferred and its constants fitted on all of days 0 to 4,087." },
    Calibration { model: "stress_plate_hole_s235_elastic", companion_to: "stress_plate_hole_s235", not_blind: "Every number of stress_plate_hole_s235's scored comparison was read before this structure, the same field with the steel not yielding, was chosen: its held-back ring, and the three lines that field had been reported to meet there, not gated, beside the miss of the field yielding at 235 MPa." },
    // a model calibrated to the record: Calibration { model, companion_to, not_blind },
];

/// The calibration of a model, by its served name; None for a model that has none.
pub(crate) fn calibration_of(model: &str) -> Option<&'static Calibration> {
    CALIBRATIONS.iter().find(|row| row.model == model)
}

/// The served name of an index entry: the card's name, or the solver's.
fn served_name(entry: &Entry) -> &'static str {
    entry.key.rsplit('/').next().unwrap_or(entry.key)
}


/// How `ln2_line_dlr_fttf` did on its six held-back tests.
const LN2_LINE_MARKS: Marks = Marks::Missed;
const LN2_LINE_MARKS_WORDS: &str = "missed seven of eleven marks on the six held-back tests and met four: the first rise at S2, the front's speed from S1 to S2, the second surge's time and the steady pressure drop";

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
        "Finds the served model for a real thing described in plain words, such as \"a lithium cell discharging in the cold\". It returns the best few cards and solvers, each with fit first (the words that matched, and model_of: the one real thing it is a model of, and its range) and then accuracy: against_measurement, the evidence behind it in one of three words, as list_cards and list_solvers give it (\"compared with measurement\", \"held to a reference calculation\" or \"company specimen\"); compared_with, what it was fitted on; and accuracy_against_measurement, what it was compared with outside the fit, within what, and the gap, in numbers. Fit and accuracy are separate: a model with a gap is still the served model of its thing. A model a newer one supersedes carries superseded_by and comes after it, still served under its own ID. The names it gives (domain and card, or solver) are the names list_cards and list_solvers give and the names describe_card, describe_solver, cite and every other tool take; describe_card or describe_solver gives the range and every refusal. When nothing served fits it says so, having compared every card and solver, names the nearest by words and why they are not it, or says that there is no near one, and says where requests for a model go. A model is set aside when the description names another thing of its kind (another gas, another maker's cell, another planet's probe). It is matched against a fixed list of terms with no model call: the same words give the same answer."
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
    /// The other real thing of the model's kind the description names, when
    /// it names one.
    another: Option<&'static str>,
}

/// Every place in the description where the term's words stand together.
fn places(described: &[String], term: &[String]) -> Vec<usize> {
    if term.is_empty() || described.len() < term.len() {
        return Vec::new();
    }
    described.windows(term.len()).enumerate().filter(|(_, window)| *window == term).map(|(at, _)| at).collect()
}

/// The first of a model's other things that a description names, unless the
/// description names the model's own thing too. Both are read in all the
/// description's words, the stop words with them, so that "SPERT I" is its
/// own name and not "SPERT" beside a word that says nothing; and a name of
/// the model's own counts only where it stands outside another thing's name.
fn names_another(entry: &'static Entry, all: &[String]) -> Option<&'static str> {
    let mut another = None;
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for term in entry.other {
        let term_words = words(term);
        for at in places(all, &term_words) {
            spans.push((at, at + term_words.len()));
            another.get_or_insert(*term);
        }
    }
    let another = another?;
    let own_thing_named = entry.own.iter().any(|term| {
        let term_words = words(term);
        places(all, &term_words).into_iter().any(|at| !spans.iter().any(|(from, to)| *from <= at && at + term_words.len() <= *to))
    });
    if own_thing_named { None } else { Some(another) }
}

/// A model's score for a description. Each term is found once, at its first
/// place, and each word of the description counts once for a model, at the
/// weight of the heaviest term found on it: "deuterium-tritium" does not
/// count again as "deuterium" and as "tritium", and two forms of one word
/// do not count as two words. A term counts as matched when it holds at
/// least one word no heavier or earlier term holds. `all` is every word of
/// the description, the stop words with them.
fn score(entry: &'static Entry, described: &[String], all: &[String]) -> Scored {
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
        another: names_another(entry, all),
    }
}

/// The evidence behind a model, in numbers: a compared model's comparison
/// and gap from its spec sheet; a company specimen's account of itself; a
/// reference calculation's, from its record.
fn accuracy(entry: &Entry) -> String {
    if let Some(sheet) = crate::spec_sheet::of(served_name(entry)) {
        return sheet.accuracy();
    }
    match entry.marks {
        Marks::NotCompared => format!("Company specimen: {OWN_SPECIMEN_ACCOUNT}."),
        Marks::Reference => format!("Held to a reference calculation: {}.", entry.marks_words.trim_end_matches('.')),
        _ => entry.marks_words.to_string(),
    }
}

/// A model's evidence in one word and in numbers: what `against_measurement`
/// and `accuracy_against_measurement` hold for it.
fn verdict(entry: &Entry) -> (&'static str, String) {
    (entry.marks.against_measurement(), accuracy(entry))
}

/// The evidence of a card, by its names. None for a card the index does not hold.
pub(crate) fn verdict_of_card(domain: &str, card: &str) -> Option<(&'static str, String)> {
    INDEX.iter().find(|entry| entry.key.split_once('/') == Some((domain, card))).map(verdict)
}

/// The evidence of a solver, by its name. None for a solver the index does not hold.
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
    // A compared model is of what it computes, over its range; the rest of
    // its spec sheet is its accuracy.
    let summary = match crate::spec_sheet::of(served_name(entry)) {
        Some(sheet) => json!(sheet.model_of()),
        None => summary,
    };
    row.insert("model_of".to_string(), summary);
    crate::succession::into(&mut row, view, served_name(entry));
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
    let sheet = crate::spec_sheet::of(served_name(entry));
    let compared_with = match entry.marks {
        _ if sheet.is_some() => sheet.map(|sheet| sheet.fitted_on.to_string()).unwrap_or_default(),
        Marks::NotCompared => "Company specimen: every constant is stated in its description.".to_string(),
        Marks::Reference => {
            let reference = reference.unwrap_or(Value::Null);
            format!("A reference calculation: {}.", text(reference.get("name")).trim_end_matches('.'))
        }
        // The record's account of its data: what the model was fitted on
        // and what it was compared with outside the fit.
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
        "Fit and accuracy are separate. Fit: each match is a match on words; model_of is the one real thing the model is of, and its range. For a thing described by its kind (a cell discharging, a capsule entering an atmosphere), a match is the served model of that kind of thing, and fits. For one particular other object (a named cell, a dataset of your own), a model of a different object of the same kind is not a model of it, and nothing served fits. Accuracy: against_measurement names the evidence (compared with measurement, held to a reference calculation, or company specimen); compared_with says what the model was fitted on; accuracy_against_measurement gives what it was compared with outside the fit, within what, and the gap, in numbers. A model with a gap is still the served model of its thing. A model with superseded_by is still served under its own ID, every earlier proof with it verifies, and the newer model is listed before it. describe_card or describe_solver, by the names given here, has the whole record, the range and every refusal by name, and cite, by the same names, the people whose data it stands on. Everything this connection serves ({served}) was compared with the description; when nothing served fits, requests for a model go to {} at {}, each naming an open dataset of the thing.",
        crate::identity::MAKER,
        crate::identity::EMAIL
    )
}

/// Where a request for a model of a thing not served goes, and what such a
/// request holds, said as what is so.
fn where_requests_go() -> String {
    format!(
        "Requests for a model of a thing not served go to {} of {} at {}. A model is made from open measurements of one real thing, so a request names an open dataset of it (the real thing, where the dataset is, its licence, the quantity measured and who measured it, each as the dataset's own page states it and none from memory) or, with no dataset in hand, says what the thing is and what has been measured of it.",
        crate::identity::MAKER,
        crate::identity::COMPANY,
        crate::identity::EMAIL
    )
}

/// What an answer with no match says: complete in itself.
fn none_fits(cards: usize, solvers: usize) -> String {
    let served = served(cards, solvers);
    format!(
        "Nothing served fits this description. It was compared with everything this connection serves ({served}), and no served model fits it; list_cards and list_solvers name everything served. Each served model is of one named real thing, not of a class of things. {}",
        where_requests_go()
    )
}

const NO_NEAREST: &str = "No served model shares a word that names the thing described, so there is no near model to name; a word of a wide subject (steel, temperature, load) is shared with too many things to count.";
const NEAREST_ABOUT: &str = "The served models that share the most words with the description, named so that the lists need not be read. None of them matched: each is a model of the thing in its model_of, not of the thing described.";

/// Words of a description that speak of where a reading came from: a signed,
/// a spoofed or a tampered sensor.
const SIGNED_WORDS: &[&str] = &[
    "signed", "signature", "spoofed", "spoofing", "spoof", "tampered", "tampering", "attested", "attestation", "authenticated", "forged",
];

/// What an answer says when its description speaks of a signed, spoofed or
/// tampered reading: beside the models that take that load, where a
/// signature goes and what it shows.
pub(crate) const SIGNED_LOADS_SAYS: &str = "A reading an instrument measured can be sent signed: a segment's load, a run's initial, a solve's parameters and a measurement of validate_series may each carry signed, the instrument's Ed25519 signature over the reading, which is verified before computing, and every run answer's load_source names the key that signed each load or says the load is the caller's number. A signature shows who stated a reading, not that it is true: a spoofed input is found by running the model on the signed input and comparing it, in validate_series, with a second output another key signed.";

fn speaks_of_signed(all: &[String]) -> bool {
    SIGNED_WORDS.iter().any(|word| words(word).iter().all(|form| all.contains(form)))
}

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
        None => return Err(violation("Missing required parameter: 'description' (InputMissing).".into())),
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
    let all = words(described);
    // A model whose description names another thing of its kind is not a
    // match, however many words the two share.
    let fits = |scored: &Scored| {
        scored.another.is_none()
            && scored.score >= LEAST_SCORE
            && (scored.strong || scored.plain >= 2 && (scored.score > LEAST_SCORE || scored.whole))
    };
    let mut scored: Vec<Scored> = INDEX.iter().map(|entry| score(entry, &read, &all)).collect();
    // The best first; equal scores keep the engine's order.
    scored.sort_by(|a, b| b.score.cmp(&a.score));
    let mut matches = Vec::new();
    let mut listed: Vec<&'static str> = Vec::new();
    for found in scored.iter().filter(|scored| fits(scored)) {
        if matches.len() == limit {
            break;
        }
        let name = served_name(found.entry);
        if listed.contains(&name) {
            continue;
        }
        // A model a newer one supersedes: the newer first, then the
        // original under its own ID.
        if let Some(by) = crate::succession::superseded_by(view, name).filter(|by| !listed.contains(by)) {
            if let Some(newer) = INDEX.iter().find(|entry| served_name(entry) == by) {
                if let Some(mut row) = record(view, newer, &found.matched) {
                    let terms: Vec<String> = found.matched.iter().map(|term| format!("\"{term}\"")).collect();
                    row.insert("fit".to_string(), json!(format!("supersedes {name}, which matches your description on the words {}", terms.join(", "))));
                    row.insert("supersedes".to_string(), json!(crate::succession::supersedes(view, by)));
                    row.insert("score".to_string(), json!(found.score));
                    matches.push(Value::Object(row));
                    listed.push(by);
                    if matches.len() == limit {
                        break;
                    }
                }
            }
        }
        let Some(mut row) = record(view, found.entry, &found.matched) else {
            continue;
        };
        row.insert("score".to_string(), json!(found.score));
        matches.push(Value::Object(row));
        listed.push(name);
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
            let shares = format!("shares only {} with the description", terms.join(", "));
            let why = match found.another {
                Some(another) => format!("{shares}, which names \"{another}\": another thing than the one this is a model of"),
                None => shares,
            };
            row.insert("not_a_match".to_string(), json!(why));
            nearest.push(Value::Object(row));
        }
        let nearest_about = if nearest.is_empty() { NO_NEAREST } else { NEAREST_ABOUT };
        answer.insert("nearest_by_words".to_string(), Value::Array(nearest));
        answer.insert("nearest_by_words_about".to_string(), json!(nearest_about));
    } else {
        answer.insert("matches".to_string(), Value::Array(matches));
        answer.insert("about".to_string(), json!(about(cards, solvers)));
    }
    if speaks_of_signed(&all) {
        answer.insert("signed_loads".to_string(), json!(SIGNED_LOADS_SAYS));
    }
    Ok(Value::Object(answer))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every model compared with measurement has its spec sheet, and its
    /// accuracy, in find_model and in the short form, is the sheet's
    /// comparison and gap, with none of the old record's words. A model a
    /// newer squad adds to the index with no sheet fails here by name.
    #[test]
    fn every_model_compared_with_measurement_has_its_spec_sheet() {
        let view = View::full();
        for entry in INDEX.iter().filter(|entry| entry.marks.compared()) {
            let name = served_name(entry);
            let sheet = crate::spec_sheet::of(name).unwrap_or_else(|| panic!("{}: compared with measurement, and no row of spec_sheet::SHEETS", entry.key));
            assert_eq!(accuracy(entry), sheet.accuracy(), "{}", entry.key);
            let row = record(&view, entry, &["term"]).unwrap();
            assert_eq!(row["model_of"], sheet.model_of(), "{}", entry.key);
            assert_eq!(row["compared_with"], sheet.fitted_on, "{}", entry.key);
            // Each refusal the sheet names is one the model lists.
            let described = match entry.key.split_once('/') {
                Some((domain, card)) => crate::descriptor::describe_value_in(&view, domain, card),
                None => crate::solvers::describe_whole(entry.key),
            }
            .unwrap_or_else(|| panic!("{} is described", entry.key));
            let listed = described["rejects"].to_string() + &described["refusals"].to_string() + &described["declared_ends"].to_string();
            for named in crate::spec_sheet::names_in(sheet.refuses) {
                assert!(listed.contains(&format!("\"{named}\"")), "{}: the sheet names {named}, which it does not list", entry.key);
            }
        }
        for entry in INDEX.iter().filter(|entry| !entry.marks.compared()) {
            assert!(crate::spec_sheet::of(served_name(entry)).is_none(), "{}: a sheet, and not compared with measurement", entry.key);
        }
    }

    /// A model a newer one supersedes: its list row, its descriptions and its
    /// match say so, and find_model gives the newer model first and the
    /// original after it under its own ID. Stand-ins: two served cards of one
    /// object, before any successor is merged.
    #[test]
    fn a_superseded_model_is_said_so_and_its_successor_comes_first() {
        static STAND_IN: &[crate::succession::Succession] = &[crate::succession::Succession { model: "reactor_spert3", by: "reactor_spert3_ccore" }];
        let view = View::full();
        let before = find_in(&view, &json!({ "description": "SPERT III reactor power excursion", "limit": 5 })).unwrap();
        crate::succession::with_table(STAND_IN, || {
            let answer = find_in(&view, &json!({ "description": "SPERT III reactor power excursion", "limit": 5 })).unwrap();
            let cards: Vec<&str> = answer["matches"].as_array().unwrap().iter().map(|row| row["card"].as_str().unwrap()).collect();
            let at = |card: &str| cards.iter().position(|named| *named == card).unwrap_or_else(|| panic!("{card}: {answer}"));
            assert!(at("reactor_spert3_ccore") + 1 == at("reactor_spert3"), "{cards:?}");
            assert_eq!(cards.iter().filter(|card| **card == "reactor_spert3_ccore").count(), 1, "{cards:?}");
            let newer = &answer["matches"][at("reactor_spert3_ccore")];
            assert_eq!(newer["supersedes"], json!(["reactor_spert3"]));
            assert!(newer["fit"].as_str().unwrap().starts_with("supersedes reactor_spert3, which matches your description on the words"), "{newer}");
            let old = &answer["matches"][at("reactor_spert3")];
            assert_eq!(old["superseded_by"], "reactor_spert3_ccore");
            assert_eq!(old["model_of"], before["matches"].as_array().unwrap().iter().find(|row| row["card"] == "reactor_spert3").unwrap()["model_of"]);
            // The list row, the full and the short descriptions.
            let list = crate::descriptor::list_value_in(&view);
            let row = list["cards"].as_array().unwrap().iter().find(|row| row["card"] == "reactor_spert3").unwrap();
            assert_eq!(row["superseded_by"], "reactor_spert3_ccore");
            assert_eq!(row.as_object().unwrap().keys().collect::<Vec<_>>(), ["domain", "card", "against_measurement", "superseded_by", "specimen"]);
            let full = crate::descriptor::describe_value_in(&view, "energy", "reactor_spert3").unwrap();
            assert_eq!(full.as_object().unwrap().keys().take(3).collect::<Vec<_>>(), ["domain", "card", "superseded_by"]);
            let short = crate::descriptor::describe_short_in(&view, "energy", "reactor_spert3").unwrap();
            assert_eq!(short["superseded_by"], "reactor_spert3_ccore");
            // The successor and every other model say nothing of it.
            for card in ["reactor_spert3_ccore", "reactor_spert3_fuel"] {
                assert!(crate::descriptor::describe_value_in(&view, "energy", card).unwrap().get("superseded_by").is_none(), "{card}");
            }
            // A view that does not serve the successor says nothing either.
            let without = View::limited([("energy".to_string(), "reactor_spert3".to_string())], []);
            let list = crate::descriptor::list_value_in(&without);
            assert!(list["cards"][0].get("superseded_by").is_none(), "{list}");
        });
        // With the served table, which is empty until the merge, nothing changes.
        assert_eq!(find_in(&view, &json!({ "description": "SPERT III reactor power excursion", "limit": 5 })).unwrap(), before);
    }

    /// A solver a newer one supersedes, on a stand-in.
    #[test]
    fn a_superseded_solver_says_so_on_its_row_and_in_its_descriptions() {
        static STAND_IN: &[crate::succession::Succession] = &[crate::succession::Succession { model: "fusion_pb11_thermal", by: "fusion_pb11_lhd" }];
        let view = View::full();
        crate::succession::with_table(STAND_IN, || {
            let list = crate::solvers::list_value_in(&view);
            let row = list["solvers"].as_array().unwrap().iter().find(|row| row["solver"] == "fusion_pb11_thermal").unwrap();
            assert_eq!(row["superseded_by"], "fusion_pb11_lhd");
            for form in ["full", "short"] {
                let described = crate::solvers::describe_request_in(&view, &json!({ "solver": "fusion_pb11_thermal", "form": form })).unwrap();
                assert_eq!(described["superseded_by"], "fusion_pb11_lhd", "{form}");
                assert_eq!(described.as_object().unwrap().keys().nth(1).map(String::as_str), Some("superseded_by"), "{form}");
            }
        });
    }

    #[test]
    fn a_description_of_a_signed_or_spoofed_sensor_finds_the_cards_and_says_where_a_signature_goes() {
        let answer = find_in(&View::full(), &json!({ "description": "a battery pack whose current sensor may be spoofed, with signed readings", "limit": 5 })).unwrap();
        assert_eq!(answer["signed_loads"], SIGNED_LOADS_SAYS);
        let found: Vec<&str> = answer["matches"].as_array().unwrap().iter().filter_map(|row| row["card"].as_str()).collect();
        assert!(found.contains(&"battery_sag"), "{answer}");
        let plain = find_in(&View::full(), &json!({ "description": "a 12S battery pack sagging under load until cutoff" })).unwrap();
        assert!(plain.get("signed_loads").is_none());
    }

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
                // Every model held to no outside measurement is one of the
                // company's own specimens, and its record says so.
                // A card's record states the outside recomputation of its
                // runs; a solver's claims none, which no such solver has.
                Marks::NotCompared => {
                    assert_eq!(entry.marks_words, OWN_SPECIMEN, "{}", entry.key);
                    let own = if entry.key.contains('/') { crate::descriptor::OWN_SPECIMEN_RESULT } else { crate::descriptor::OWN_SPECIMEN_SOLVER_RESULT };
                    assert_eq!(result, own, "{}", entry.key);
                    if !entry.key.contains('/') {
                        let whole = crate::solvers::describe_whole(entry.key).unwrap().to_string();
                        assert!(!whole.contains("recomputation"), "{}: an own-specimen solver claims a recomputation", entry.key);
                    }
                }
                // A model held to a reference calculation: atom_dft has no
                // record of a comparison, and a fusion solver's record states
                // its comparison with the published calculation it was held to.
                Marks::Reference => assert!(
                    result.is_empty() || result.contains("TRANSP") || result.contains("calculation") || result.contains("published"),
                    "{}: {result}",
                    entry.key
                ),
                // A model compared with measurement states its result in numbers.
                _ => assert!(result.chars().any(|c| c.is_ascii_digit()), "{}: {result}", entry.key),
            }
            assert!(record(&view, entry, &[]).is_some(), "{}", entry.key);
        }
        // The generator serves the days outside its fit, and its accuracy
        // keeps the record's numbers: 0.008129 and +0.010094 of 115.4 W.
        let generator = INDEX.iter().find(|entry| entry.key == "energy/generator_mmrtg_f1").expect("the generator");
        assert!(generator.marks.compared());
        let said = accuracy(generator);
        for words in ["2,809 to 4,087", "0.008129", "0.010094"] {
            assert!(said.contains(words), "{words}: {said}");
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
                if read.is_empty() || !score(entry, &read, &read).matched.contains(term) {
                    faults.push(format!("{}: \"{term}\" is not found in its own words", entry.key));
                }
                if let Some((_, other)) = seen.iter().find(|(words, _)| *words == read) {
                    faults.push(format!("{}: \"{term}\" and \"{other}\" are one word", entry.key));
                }
                // A model's own term does not name another thing of its kind.
                if let Some(another) = names_another(entry, &words(term)) {
                    faults.push(format!("{}: its own term \"{term}\" names \"{another}\", which it lists as another thing", entry.key));
                }
                seen.push((read, term));
            }
            let mut others: Vec<Vec<String>> = Vec::new();
            for other in entry.other {
                let read = words(other);
                if read.is_empty() || others.contains(&read) {
                    faults.push(format!("{}: \"{other}\" is empty or listed twice among its other things", entry.key));
                }
                others.push(read);
            }
            // A name of its own is not also a name of another thing.
            for own in entry.own {
                let read = words(own);
                if read.is_empty() || others.contains(&read) {
                    faults.push(format!("{}: its own name \"{own}\" is empty or is listed as another thing", entry.key));
                }
            }
        }
        assert!(faults.is_empty(), "{}", faults.join("\n"));
    }

    /// The models found for a description, and for an answer with no match
    /// the models named as near with what each is said to share.
    fn near(description: &str) -> Vec<(String, String)> {
        let answer = find_in(&View::full(), &json!({ "description": description, "limit": 5 })).unwrap();
        assert_eq!(answer["count"], 0, "{description}: {}", answer["matches"]);
        answer["nearest_by_words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let name = row.get("solver").or(row.get("card")).and_then(Value::as_str).unwrap().to_string();
                (name, row["not_a_match"].as_str().unwrap().to_string())
            })
            .collect()
    }

    /// Yeast extract is part of the culture's own medium, and its description
    /// says so: a description that names it beside the culture is of the
    /// culture. A yeast culture is still another thing of its kind.
    #[test]
    fn yeast_extract_in_the_medium_does_not_set_the_culture_aside() {
        assert_eq!(
            first_of("A filamentous fungus fed-batch culture on glucose and yeast extract").as_deref(),
            Some("culture_trichoderma_ambr250")
        );
        assert_eq!(first_of("A filamentous fungus fed-batch culture on glucose").as_deref(), Some("culture_trichoderma_ambr250"));
        let nearest = near("a fed-batch culture of yeast on glucose");
        let row = nearest.iter().find(|(name, _)| name == "culture_trichoderma_ambr250").expect("named as near");
        assert!(row.1.ends_with(", which names \"yeast\": another thing than the one this is a model of"), "{}", row.1);
    }

    /// A description that names another real thing of a model's kind (another
    /// gas, another planet's probe, another maker's cell) is of that thing:
    /// nothing is matched, and the model is named as near with the name that
    /// set it aside. Each of these matched its model on the shared words
    /// before the models listed the other things of their kind.
    #[test]
    fn a_description_that_names_another_thing_of_a_models_kind_matches_nothing() {
        let table: &[(&str, &str, &str)] = &[
            ("the speed of sound in helium gas", "argon_pair_gas", "helium"),
            ("the capsule of the Phoenix lander entering the atmosphere of Mars", "capsule_insight", "phoenix"),
            ("an LG M50 lithium-ion cell discharging at 5 amps", "cell_18650pf", "lg"),
            ("pointing jitter of the Hubble Space Telescope on its guide star", "los_jwst_fine_guide", "hubble"),
            ("the crater the DART spacecraft made on the asteroid Dimorphos", "crater_ryugu_sci", "dimorphos"),
            ("the frequency of the Texas power grid after it loses a power station", "grid_gb", "texas"),
            ("a Formula 1 tyre slipping on dry asphalt", "tire_eve_r19_ice", "formula 1"),
            ("the SPERT I reactor after a step in reactivity", "reactor_spert3", "spert i"),
            ("a deuterium plasma in the DIII-D tokamak under neutral beam heating", "tokamak_jet_dte1", "diii-d"),
            ("Young's modulus of an Inconel bar from its resonant frequencies", "modulus_bam_s355", "inconel"),
            ("a fed-batch culture of the fungus Aspergillus niger", "culture_trichoderma_ambr250", "aspergillus"),
            ("the heat shield of the Galileo probe charring as it enters Jupiter", "ablator_slab", "galileo"),
            ("thrust and shaft power of a container ship's propeller", "propeller_apc_10x7e", "ship"),
            ("the electrical output of the radioisotope generator on Voyager 1", "generator_mmrtg_f1", "voyager"),
            ("a satellite of the GPS constellation in orbit around the Earth", "satellite_lageos1", "gps"),
            ("creep of a stainless steel tube at high temperature", "creep_cmsx6", "steel"),
            ("the ground state energy of a gold atom", "atom_dft", "gold"),
        ];
        for (description, model, another) in table {
            let nearest = near(description);
            let row = nearest.iter().find(|(name, _)| name == model).unwrap_or_else(|| panic!("{description}: {nearest:?}"));
            assert!(row.1.starts_with("shares only \""), "{description}: {}", row.1);
            assert!(
                row.1.ends_with(&format!(", which names \"{another}\": another thing than the one this is a model of")),
                "{description}: {}",
                row.1
            );
        }
        // The name is read whole: "SPERT III" is not "SPERT I", and a cell of
        // the dataset the card holds is not another maker's cell.
        // Each is superseded (succession::TABLE): its successor first, then the original under its own ID.
        assert_eq!(keys("the SPERT III E-core reactor, cold, after a step in reactivity")[..2], ["energy/reactor_spert3_twin", "energy/reactor_spert3"]);
        assert_eq!(keys("the SPERT III reactor with its C-core after a step in reactivity")[..2], ["energy/reactor_spert3_ccore_twin", "energy/reactor_spert3_ccore"]);
        // Another maker's cell, or another cathode, is not this LiCoO2 cell.
        assert_ne!(first_of("thermal runaway of a Panasonic NCR18650PF cell").as_deref(), Some("energy/runaway_18650_lco"));
        assert_ne!(first_of("thermal runaway of an NMC 18650 cell").as_deref(), Some("energy/runaway_18650_lco"));
        // The things themselves, and their kinds, are found as before.
        for (description, first) in [
            ("the speed of sound in argon gas", "argon_pair_gas"),
            ("the speed of sound in a gas", "argon_pair_gas"),
            ("the capsule of the InSight lander entering the atmosphere of Mars", "aerospace/capsule_insight"),
            ("a capsule entering the atmosphere of Mars", "aerospace/capsule_insight"),
            ("pointing jitter of the Webb telescope on its guide star", "aerospace/los_jwst_fine_guide"),
            ("the crater an impactor makes on an asteroid", "aerospace/crater_ryugu_sci"),
            ("a tyre slipping on ice", "materials/tire_eve_r19_ice"),
            ("an 18650 lithium-ion cell going into thermal runaway in an oven", "energy/runaway_18650_lco"),
            ("thermal runaway of a LiCoO2 18650 cell in an accelerating rate calorimeter", "energy/runaway_18650_lco"),
            ("when does a fully charged 18650 battery catch fire in a hot oven", "energy/runaway_18650_lco"),
            ("a deuterium-tritium plasma in the JET tokamak under neutral beam heating", "energy/tokamak_jet_dte1"),
            ("the neutron rate of a TFTR deuterium-tritium pulse from its profiles", "tokamak_dt_beam_target"),
        ] {
            assert_eq!(first_of(description).as_deref(), Some(first), "{description}: {:?}", keys(description));
        }
        // A probe of another planet is not the capsule that came back to the Earth.
        assert_eq!(keys("a capsule entering the atmosphere of Mars"), ["aerospace/capsule_insight"]);
        // A description that names the model's own thing is of that thing,
        // though it names another beside it.
        for (description, first) in [
            ("the viscosity of argon gas, compared with air", "argon_pair_gas"),
            ("pointing jitter of the Webb telescope, set against Hubble's", "aerospace/los_jwst_fine_guide"),
            ("the capsule of the InSight lander, built on the Phoenix design, entering the atmosphere of Mars", "aerospace/capsule_insight"),
            // reactor_spert3 is superseded: its successor comes first (succession::TABLE).
            ("the SPERT III E-core reactor after a step in reactivity, as SPERT I was tested before it", "energy/reactor_spert3_twin"),
            ("the electrical output of the radioisotope generator on the Curiosity rover, the kind Voyager carried before it", "energy/generator_mmrtg_f1"),
        ] {
            assert_eq!(first_of(description).as_deref(), Some(first), "{description}: {:?}", keys(description));
        }
        // A name of its own inside another thing's name is that other thing's:
        // the arc jet of a heat-shield test is not the JET tokamak.
        let found = keys("the heat shield of a capsule charring in an arc jet");
        assert!(found[0] == "ablator_slab" && !found.iter().any(|name| name.contains("tokamak")), "{found:?}");
        // LAGEOS-2 is the satellite card's own: it was compared on both orbits.
        for description in ["the orbit of the satellite LAGEOS-2 around the Earth", "orbit of the LAGEOS-2 satellite", "LAGEOS II laser ranging"] {
            assert_eq!(first_of(description).as_deref(), Some("aerospace/satellite_lageos1"), "{description}: {:?}", keys(description));
        }
        for (description, model, another) in [
            ("the speed of sound in liquid argon", "argon_pair_gas", "liquid argon"),
            ("a PICA-X heat shield charring in an arc jet", "ablator_slab", "pica-x"),
        ] {
            let nearest = near(description);
            let row = nearest.iter().find(|(name, _)| name == model).unwrap_or_else(|| panic!("{description}: {nearest:?}"));
            assert!(row.1.contains(&format!("which names \"{another}\"")), "{description}: {}", row.1);
        }
    }

    /// A model's own published quantities and inputs, and the names of the
    /// real things it takes, put it first: the distance the Solar System card
    /// publishes, the cells each Tesla card holds, the inputs that part the
    /// two cards of one reactor core and the two models of one tokamak.
    #[test]
    fn a_models_own_quantities_and_the_names_of_its_things_put_it_first() {
        // The card publishes earth_to_mars_m and earth_mars_distance_m. Neither sentence found it before.
        assert_eq!(keys("How far is Mars from the Earth one day after 1 January 1999?"), ["aerospace/solar_system"]);
        assert_eq!(first_of("the distance from the Earth to Mars over ten years").as_deref(), Some("aerospace/solar_system"));
        // Cell m448-N is held by the physical card alone; cell m80 by both, and the card of cell m80 alone comes first for it.
        assert_eq!(keys("cell m448-N of the Tesla Model 3 2170 lithium-ion cell dataset, discharged at a constant 4.5 amps"), ["energy/cell_2170_model3"]);
        assert_eq!(keys("cell m1000 of the Tesla Model 3 2170 lithium-ion cell dataset at 0 degrees Celsius"), ["energy/cell_2170_model3"]);
        assert_eq!(
            keys("cell m80 of the Tesla Model 3 2170 lithium-ion cell dataset, discharged at a constant 1.5 amps")[..2],
            ["energy/cell_2170_m80", "energy/cell_2170_model3"]
        );
        // The E-core's two cards: one takes a cold step, the other an inlet temperature, a coolant flow and a rod that ramps.
        // Both are superseded by reactor_spert3_twin, which comes first; then the original that fits.
        assert_eq!(keys("the SPERT III E-core reactor is cold when a step in reactivity is put in all at once")[..2], ["energy/reactor_spert3_twin", "energy/reactor_spert3"]);
        assert_eq!(
            keys("the SPERT III E-core reactor in a burst with hot coolant flowing through the core as its transient rod ramps in reactivity")[..2],
            ["energy/reactor_spert3_twin", "energy/reactor_spert3_fuel"]
        );
        // One tokamak, two models: the solver takes the plasma in shells, the card its energies under the heating power.
        assert_eq!(
            first_of("nested shells of a deuterium-tritium plasma in the JET tokamak in steady state: electron temperatures, ion temperatures and electron densities").as_deref(),
            Some("tokamak_dt_beam_target")
        );
        assert_eq!(
            first_of("the thermal energy of a deuterium-tritium plasma in the JET tokamak under 20 MW of heating power at a plasma current of 3.8 MA").as_deref(),
            Some("energy/tokamak_jet_dte1")
        );
        // The thermal proton-boron solver takes the ion temperature and publishes the fusion power.
        assert_eq!(first_of("fusion power of a hot hydrogen-boron plasma against its radiation losses at an ion temperature of 300 keV").as_deref(), Some("fusion_pb11_thermal"));
        // The molten-salt reactor, by its name and by what happens to it.
        assert_eq!(first_of("MSRE pump transient").as_deref(), Some("energy/reactor_msre_pump"));
        assert_eq!(first_of("molten salt reactor fuel pump coast-down").as_deref(), Some("energy/reactor_msre_pump"));
        assert_eq!(first_of("the reactivity lost when the fuel of a molten-salt reactor starts to circulate").as_deref(), Some("energy/reactor_msre_pump"));
    }

    /// The SPERT III twin is the first answer to the core described in plain
    /// words, its tests named as the report names them.
    #[test]
    fn the_spert_iii_core_in_plain_words_finds_its_twin_first() {
        for description in [
            "SPERT III reactor transient",
            "the SPERT III E-core reactor",
            "a digital twin of the SPERT III reactor",
            "SPERT III hot standby transient from 1 MW",
            "SPERT III operating power excursion at 19 MW",
            "SPERT 3 rod ejection with the clad temperature",
        ] {
            assert_eq!(first_of(description).as_deref(), Some("energy/reactor_spert3_twin"), "{description}");
        }
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
            ("the least grip force that lifts an object without slip in a two-finger grip", "materials/grasp_lift_glass_4tu", &[]),
            ("a robot hand picking up a glass object between two fingers", "materials/grasp_lift_glass_4tu", &[]),
            ("oil film in a journal bearing", "journal_bearing_sheffield", &[]),
            ("Young's modulus of a steel bar from the frequency it rings at", "modulus_bam_s355", &[]),
            ("proton-boron fusion under a neutral beam", "fusion_pb11_lhd", &["fusion_pb11_thermal"]),
            ("deuterium-tritium fusion rate in a tokamak", "tokamak_dt_beam_target", &["energy/tokamak_jet_dte1"]),
            ("the yield of an inertial confinement fusion hot spot at the National Ignition Facility", "icf_hotspot_burn", &[]),
            ("laminar air flow over a backward-facing step", "step_flow_armaly", &[]),
            ("viscosity of argon gas", "argon_pair_gas", &[]),
            ("the planets of the Solar System over twenty years", "aerospace/solar_system", &[]),
            ("a nitrogen-charged hydraulic accumulator", "energy/accumulator_sbo450", &[]),
            ("a Dyneema tendon stretching under force", "materials/cable_stretch", &[]),
            ("a copper fuse link under overcurrent", "energy/grid_node", &[]),
            ("water hammer when a valve closes fast on a hydraulic line", "water_hammer_oil_line", &[]),
            ("choked flow through a pneumatic valve filling a chamber", "orifice_air_chamber", &[]),
            ("the resonance of a pneumatic hose feeding a volume", "pneumatic_hose_resonance", &[]),
            ("the stress around a hole in a steel plate pulled in tension", "stress_plate_hole_s235", &[]),
            ("stress concentration at a notch in a structural steel bar", "stress_plate_hole_s235", &[]),
            ("digital image correlation of an S235 specimen with a hole", "stress_plate_hole_s235", &["stress_plate_hole_s235_elastic"]),
            ("the linear elastic stress field around a hole in a steel plate", "stress_plate_hole_s235_elastic", &["stress_plate_hole_s235"]),
            ("the elastic field of an S235 plate with a hole", "stress_plate_hole_s235_elastic", &["stress_plate_hole_s235"]),
            ("fluid hammer in a liquid nitrogen rocket feed line when a fast valve closes", "ln2_line_dlr_fttf", &[]),
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

    /// A description of a served thing that mentions another thing in
    /// passing, without the model's own name, still finds its model: what
    /// the thing is made of, what it is found in, what it is set against.
    #[test]
    fn a_served_thing_described_with_another_mentioned_in_passing_keeps_its_model() {
        let table: &[(&str, &str)] = &[
            ("What is the zero-density viscosity of the noble gas that makes up about one percent of air, at 400 K?", "argon_pair_gas"),
            ("Creep extension of a single-crystal nickel superalloy with aluminium and titanium in it, held under load at 980 degrees Celsius.", "materials/creep_cmsx6"),
            ("Ground-state energy of a single isolated neon atom, not a molecule or a crystal.", "atom_dft"),
            ("Sound speed profile down a deep ocean cast, as opposed to a lake.", "seawater_column"),
            ("Grip of an all-season SUV tyre on an ice track, well below what it has on dry asphalt.", "materials/tire_eve_r19_ice"),
            ("Friction factor of turbulent air in the Princeton Superpipe, whose wall is smooth rather than rough.", "pipe_flow_superpipe"),
            ("Rain fade on a short microwave backhaul link between two dish antennas in Gothenburg.", "signal/link_gothenburg_10010"),
            ("How wide did the artificial crater get that Hayabusa2's copper impactor blasted into a near-Earth asteroid?", "aerospace/crater_ryugu_sci"),
        ];
        for (description, model) in table {
            assert_eq!(first_of(description).as_deref(), Some(*model), "{description}");
        }
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
    /// solver was compared, that no served model fits, that the lists name
    /// everything served, and how to ask;
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
        assert!(said.contains("), and no served model fits it; list_cards and list_solvers name everything served."), "{said}");
        // Until 0.12.0 it ended "so reading list_cards or list_solvers will not
        // find one", which a reader could take as the lists being incomplete.
        assert!(!said.contains("will not find"), "{said}");
        // Where a request for a model goes is said as what is so, with the
        // address, and not as an instruction to the reader: until this was
        // reworded the sentence began "To ask for a model of this thing,
        // write to John Kruze of Protocol Company at ...".
        for field in ["Requests for a model of a thing not served go to John Kruze of Protocol Company at kruze@zerotrustphysics.com.", "an open dataset", "its licence", "the quantity measured", "who measured it", "none from memory"] {
            assert!(said.contains(field), "{field}: {said}");
        }
        for instruction in ["write to", "To ask", "name an open dataset", "say what"] {
            assert!(!said.contains(instruction), "{instruction}: {said}");
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
    /// words; the accuracy is one phrase in numbers, with no true or false
    /// to misread as "does not fit".
    #[test]
    fn fit_comes_first_and_the_accuracy_is_one_phrase_in_numbers() {
        let view = View::full();
        for entry in INDEX {
            let row = record(&view, entry, &["term"]).unwrap();
            let mut order: Vec<&str> = row.keys().map(String::as_str).filter(|key| !matches!(*key, "domain" | "card" | "solver")).collect();
            // A superseded model says its successor first (succession::TABLE), and only then.
            let superseded = crate::succession::superseded_by(&view, served_name(entry)).is_some();
            assert_eq!(order.first() == Some(&crate::succession::KEY), superseded, "{}", entry.key);
            if superseded {
                order.remove(0);
            }
            assert_eq!(order, ["fit", "model_of", "compared_with", "against_measurement", "accuracy_against_measurement"], "{}", entry.key);
            assert!(row.values().all(|value| value.is_string()), "{}: no true, false or null to misread", entry.key);
            let said = row["accuracy_against_measurement"].as_str().unwrap();
            let opening = match entry.marks {
                Marks::NotCompared => "Company specimen: ",
                Marks::Reference => "Held to a reference calculation: ",
                _ => "Compared with ",
            };
            assert!(said.starts_with(opening), "{}: {said}", entry.key);
            for word in crate::spec_sheet::UNSAID {
                assert!(!crate::spec_sheet::says(said, word), "{}: says \"{word}\": {said}", entry.key);
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
        assert_eq!(row["accuracy_against_measurement"], crate::spec_sheet::of("cantilever_eth").unwrap().accuracy());
        // what it was compared with is what it was fitted on, not the specimen again
        assert!(row["compared_with"].as_str().unwrap().starts_with("Fitted on ") || row["compared_with"].as_str().unwrap().starts_with("Nothing fitted"), "{row}");
        let about = answer["about"].as_str().unwrap();
        assert!(about.starts_with("Fit and accuracy are separate.") && about.contains("is still the served model of its thing") && about.contains("kruze@zerotrustphysics.com") && !about.contains("request_model"), "{about}");
        for word in crate::spec_sheet::UNSAID {
            assert!(!crate::spec_sheet::says(about, word), "{word}: {about}");
        }
        assert!(about.ends_with("when nothing served fits, requests for a model go to John Kruze at kruze@zerotrustphysics.com, each naming an open dataset of the thing.") && !about.contains("write to"), "{about}");
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
                Marks::NotCompared => "company specimen",
                Marks::Reference => "held to a reference calculation",
                Marks::Measured | Marks::Met | Marks::Missed | Marks::InPart | Marks::Calibrated => "compared with measurement",
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
        assert_eq!(counted.into_iter().collect::<Vec<_>>(), [("company specimen", 12), ("compared with measurement", 50), ("held to a reference calculation", 3)]);
        // No row of either list is without the word, but the solver compiled for tests only.
        assert_eq!(cards["cards"].as_array().unwrap().len(), INDEX.iter().filter(|entry| entry.key.contains('/')).count());
        for row in cards["cards"].as_array().unwrap() {
            assert!(row["against_measurement"].is_string(), "{row}");
            // A superseded card names its successor after its word (succession::TABLE).
            let superseded = crate::succession::superseded_by(&view, row["card"].as_str().unwrap()).is_some();
            let expected: &[&str] = if superseded { &["domain", "card", "against_measurement", "superseded_by", "specimen"] } else { &["domain", "card", "against_measurement", "specimen"] };
            assert_eq!(row.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(), expected, "{row}");
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
        assert_eq!(answer["matches"][0]["against_measurement"], "compared with measurement");
        assert!(answer["matches"][0]["accuracy_against_measurement"].as_str().unwrap().starts_with("Compared with "));
        // Each tool that gives the key says its three words, and that the
        // numbers say how good the model is.
        for said in [description().as_str(), AGAINST_MEASUREMENT_SAYS] {
            for word in crate::spec_sheet::WORDS {
                assert!(said.contains(&format!("\"{word}\"")), "{word}: {said}");
            }
            for word in crate::spec_sheet::UNSAID {
                assert!(!crate::spec_sheet::says(said, word), "{word}: {said}");
            }
        }
        assert!(AGAINST_MEASUREMENT_SAYS.ends_with("The numbers say how good it is."));
    }

    /// A second form of a model's object (served as calibrated to the record
    /// until 0.14.3) has its row of CALIBRATIONS, and the row is its
    /// registration's word for word: the model it stands beside is a served
    /// model other than itself. It is compared with measurement, by its spec
    /// sheet, like any other.
    #[test]
    fn every_second_form_has_its_row_and_its_registration() {
        let served: Vec<&str> = INDEX.iter().map(served_name).collect();
        for entry in INDEX.iter().filter(|entry| entry.marks == Marks::Calibrated) {
            assert!(calibration_of(served_name(entry)).is_some(), "{}: no row of CALIBRATIONS", entry.key);
            assert_eq!(entry.marks.against_measurement(), crate::spec_sheet::COMPARED, "{}", entry.key);
        }
        let forecasts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("kit/forecasts");
        let record_of = |model: &str| -> Value {
            let text = std::fs::read_to_string(forecasts.join(format!("{model}.json"))).unwrap_or_else(|_| panic!("{model}: no kit/forecasts/{model}.json"));
            serde_json::from_str(&text).unwrap_or_else(|_| panic!("{model}: its registration is not JSON"))
        };
        for row in CALIBRATIONS {
            assert_eq!(CALIBRATIONS.iter().filter(|other| other.model == row.model).count(), 1, "{}: two rows", row.model);
            assert!(served.contains(&row.model), "{}: not served", row.model);
            assert!(row.companion_to != row.model && served.contains(&row.companion_to), "{}: {} is not a served model beside it", row.model, row.companion_to);
            let calibration = record_of(row.model)["calibration"].clone();
            assert_eq!(calibration["companion_to"], row.companion_to, "{}: the row's companion is not its registration's", row.model);
            assert_eq!(calibration["not_blind"], row.not_blind, "{}: the row is not its registration's", row.model);
        }
        // The served description names the model beside it and nothing of the old record.
        let short = crate::descriptor::describe_short_in(&View::full(), "energy", "generator_mmrtg_seebeck").unwrap();
        assert_eq!(short["companion_to"], "generator_mmrtg_f1");
        assert!(short.get("not_blind").is_none());
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
        let scored = |key: &str, text: &str| score(entry(key), &telling_words(text), &words(text));
        let solver = scored("tokamak_dt_beam_target", "deuterium-tritium");
        assert_eq!((solver.score, solver.matched.as_slice()), (2 * STRONG, &["deuterium-tritium"][..]));
        let grid = scored("energy/grid_gb", "grid frequency");
        assert_eq!((grid.score, grid.matched.as_slice()), (2 * STRONG, &["grid frequency"][..]));
        // A phrase and a heavier word inside it: the word at its weight, the rest of the phrase at the phrase's.
        let jet = scored("energy/tokamak_jet_dte1", "fusion power");
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
        // The instrumented fuel element of the same reactor, by its own words.
        assert_eq!(first_of("TRIGA fuel element temperature").as_deref(), Some("fuel_rod_triga_jsi"));
        assert_eq!(first_of("the thermocouple of a TRIGA fuel rod through a pulse").as_deref(), Some("fuel_rod_triga_jsi"));
        assert_eq!(first_of("the JSI TRIGA reactor pulse").as_deref(), Some("energy/reactor_triga_jsi"));
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
            ("a liquid-cooled plate under a flight computer: how warm does it get at 150 watts", "energy/thermal"),
            ("a vibratory rate sensor: a proof mass driven on its spring, with the Coriolis force of a turn rate", "signal/tesseract"),
            ("the comet sample probe that came down in Utah under its parachute", "aerospace/capsule_stardust"),
            ("the aluminium strip of the Zürich laboratory left vibrating until it dies away", "signal/cantilever_eth"),
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
